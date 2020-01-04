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
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DocsetConfig {
    pub name: String,
    pub index: String,
    pub basedir: String,
}

pub fn config_fn() -> PathBuf {
    let config = match app_root(AppDataType::UserConfig, &APP_INFO) {
        Ok(mut dir) => {
            dir.push("config.toml");
            dir
        }
        Err(_) => panic!("Could not determine UserConfig directory"),
    };
    config
}

pub fn load_config() -> Config {
    let fname = config_fn();
    if !fname.exists() {
        fs::File::create(&fname).expect("Unable to create config file");
    }
    let toml = fs::read_to_string(&fname).expect("Couldn't read config file");
    toml::from_str(&toml).unwrap()
}

impl Config {
    pub fn docset_config(&self, name: &String) -> Option<&DocsetConfig> {
        self.docsets.iter().find(|cfg| cfg.name == *name)
    }
    pub fn update_docset_config(&mut self, config: DocsetConfig) -> Option<&DocsetConfig> {
        if let Some(idx) = self.docsets.iter().position(|cfg| cfg.name == *config.name) {
            self.docsets[idx] = config;
            self.save();
            self.docsets.get(idx)
        } else {
            self.docsets.push(config);
            self.save();
            self.docsets.last()
        }
    }
    pub fn save(&self) {
        let toml = toml::to_string(&self).unwrap();
        fs::write(config_fn(), toml).expect("Couldn't write config file");
    }
}

pub fn docset_config(name: String, index: Option<String>, basedir: String) -> DocsetConfig {
    let index_dir = index.unwrap_or({
        let dir = match app_root(AppDataType::UserData, &APP_INFO) {
            Ok(mut dir) => {
                dir.push(&name);
                dir.to_string_lossy().to_string()
            }
            Err(_) => panic!("Could not determine UserData directory"),
        };
        dir
    });
    let basedir = Path::new(&basedir)
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string();
    DocsetConfig {
        name,
        index: index_dir,
        basedir,
    }
}

#[test]
fn read_config() {
    let cfg = r#"
        [[docset]]
        name = "default"
        index = "/tmp/idxdefault"
        basedir = "/home/pi/Documents"

        [[docset]]
        name = "code"
        index = "/tmp/idxcode"
        basedir = "/home/pi/code"
    "#;
    let config: Config = toml::from_str(cfg).unwrap();
    assert_eq!(config.docsets[0].name, "default");

    let toml = toml::to_string(&config).unwrap();
    assert!(toml.contains(r#"name = "default""#));
}
