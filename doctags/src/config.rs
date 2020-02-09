use anyhow::{Context, Result};
use app_dirs::{app_root, AppDataType, AppInfo};
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use toml;

const APP_INFO: AppInfo = AppInfo {
    name: "doctags",
    author: "pka",
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(rename = "docset", default)]
    pub docsets: Vec<DocsetConfig>,
    #[serde(rename = "shortcut", default)]
    pub shortcuts: Vec<ShortcutConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DocsetConfig {
    pub name: String,
    pub index: String,
    pub basedirs: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ShortcutConfig {
    pub name: String,
    pub search: String,
    pub command: String,
    pub command_type: CommandType,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommandType {
    Foreach,
    Eachdir,
}

pub fn config_fn() -> Result<PathBuf> {
    app_root(AppDataType::UserConfig, &APP_INFO)
        .map(|mut dir| {
            dir.push("config.toml");
            dir
        })
        .context("Could not determine UserConfig directory")
}

pub fn command_history_fn() -> Result<PathBuf> {
    app_root(AppDataType::UserCache, &APP_INFO)
        .map(|mut dir| {
            dir.push("history.txt");
            dir
        })
        .context("Could not determine UserCache directory")
}

pub fn load_config() -> Result<Config> {
    let fname = config_fn()?;
    if !fname.exists() {
        fs::File::create(&fname)
            .with_context(|| format!("Unable to create config file {:?}", fname))?;
    }
    let toml = fs::read_to_string(&fname).context("Couldn't read config file")?;
    toml::from_str(&toml).context("Toml syntax error")
}

impl Config {
    pub fn docset_config(&self, name: &String) -> Result<&DocsetConfig> {
        self.docsets
            .iter()
            .find(|cfg| cfg.name == *name)
            .ok_or(anyhow!("Docset config missing"))
    }
    pub fn update_docset_config(&mut self, config: DocsetConfig) -> Result<&DocsetConfig> {
        if let Some(idx) = self.docsets.iter().position(|cfg| cfg.name == *config.name) {
            self.docsets[idx] = config;
            self.save()?;
            self.docsets.get(idx).ok_or(anyhow!("Docset not found"))
        } else {
            self.docsets.push(config);
            self.save()?;
            self.docsets.last().ok_or(anyhow!("Docset not found"))
        }
    }
    pub fn save(&self) -> Result<()> {
        let toml = toml::to_string(&self)?;
        fs::write(config_fn()?, toml).context("Couldn't write config file")?;
        Ok(())
    }
}

pub fn docset_config(
    name: String,
    index: Option<String>,
    basedirs: Vec<String>,
) -> Result<DocsetConfig> {
    let index_dir = index.unwrap_or({
        app_root(AppDataType::UserData, &APP_INFO).map(|mut dir| {
            dir.push(&name);
            dir.to_string_lossy().to_string()
        })?
    });
    let basedirs: Result<Vec<String>> = basedirs
        .iter()
        .map(|dir| {
            Path::new(&dir)
                .canonicalize()
                .context("canonicalize failed")
                .map(|d| d.to_string_lossy().to_string())
        })
        .collect();
    Ok(DocsetConfig {
        name,
        index: index_dir,
        basedirs: basedirs?,
    })
}

#[test]
fn read_config() -> Result<()> {
    let cfg = r#"
        [[docset]]
        name = "default"
        index = "/tmp/idxdefault"
        basedirs = ["/home/pi/Documents"]

        [[docset]]
        name = "code"
        index = "/tmp/idxcode"
        basedirs = ["/home/pi/code"]
    "#;
    let config: Config = toml::from_str(cfg)?;
    assert_eq!(config.docsets[0].name, "default");

    let toml = toml::to_string(&config)?;
    assert!(toml.contains(r#"name = "default""#));
    Ok(())
}
