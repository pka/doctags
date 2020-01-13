use anyhow::{Context, Result};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml::{self, Value};

#[derive(Debug, Deserialize, Serialize)]
pub struct DocTags {
    #[serde(rename = "tags", default)]
    pub dirtags: Vec<String>,
    #[serde(rename = "files", default)]
    pub filetags: HashMap<String, Vec<String>>,
}

fn facet(tag: &str) -> String {
    format!("/{}", tag.replace(":", "/"))
}

// fn tag(facet: &str) -> String {
//     let tag = facet.replace("/", ":");
//     if tag.starts_with(":") {
//         tag[1..].to_string()
//     } else {
//         tag
//     }
// }

impl DocTags {
    /// Read toml with conversion to facets and absolute paths
    pub fn from_toml(dir: &Path, toml: String) -> Result<DocTags> {
        let config: Value = toml::from_str(&toml)?;
        let dirtags = if let Some(tags) = config.get("tags") {
            tag_value_to_vec(tags)?
        } else {
            vec![]
        };
        let mut filetags = HashMap::new();
        if let Some(filetable) = config.get("files") {
            filetable
                .as_table()
                .with_context(|| {
                    format!("tags must be table type in {}/.doctags.toml", dir.display())
                })?
                .iter()
                .for_each(|(fname, tags)| {
                    if let Ok(fullpath) = dir.join(fname).canonicalize() {
                        filetags.insert(
                            fullpath.to_string_lossy().to_string(),
                            tag_value_to_vec(tags).expect("tag_value_to_vec error"), // TODO
                        );
                    } else {
                        warn!(
                            "Ignoring invalid files entry '{}' in {}/.doctags.toml",
                            &fname,
                            dir.display()
                        );
                    }
                });
        }
        let doctags = DocTags { dirtags, filetags };
        Ok(doctags)
    }
}

fn tag_value_to_vec(value: &Value) -> Result<Vec<String>> {
    let facets = value
        .as_array()
        .context("tags must be array type")?
        .iter()
        .map(|tag| {
            let tagstr = tag.as_str().expect("tag must be string"); // TODO
            facet(tagstr)
        })
        .collect();
    Ok(facets)
}

pub fn read_doctags_file(dir: &Path, raw: bool) -> DocTags {
    let path = dir.join(".doctags.toml");
    if path.exists() {
        if let Ok(toml) = fs::read_to_string(path) {
            if raw {
                if let Ok(doctags) = toml::from_str(&toml) {
                    return doctags;
                }
            } else if let Ok(doctags) = DocTags::from_toml(dir, toml) {
                return doctags;
            }
        }
    }
    DocTags {
        dirtags: vec![],
        filetags: HashMap::new(),
    }
}

pub fn add_tag(path: String, tag: String, recursive: bool) -> Result<()> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(anyhow!("File '{}' does not exist", path));
    }
    let is_dir_tag = p.is_dir() && recursive;
    let dirp = if p.is_file() {
        p.parent().context("dirname not found")?
    } else {
        p
    };
    let toml_path = dirp.join(".doctags.toml");
    let mut doctags = read_doctags_file(dirp, true);
    if is_dir_tag {
        doctags.dirtags.push(tag);
    } else {
        // make relative to parent dir
        let relpath = if p.is_dir() {
            ".".to_string()
        } else {
            p.strip_prefix(dirp)?.to_string_lossy().to_string()
        };
        let filetags = doctags.filetags.entry(relpath).or_insert(vec![]);
        (*filetags).push(tag);
    }
    debug!("Writing {:?}", toml_path);

    let toml = toml::to_string(&doctags)?;
    fs::write(&toml_path, toml)
        .with_context(|| format!("Couldn't write config file {:?}", toml_path))?;
    Ok(())
}

#[test]
fn parse_toml() -> Result<()> {
    use std::env;

    let toml = r#"
        tags = ["lang:rust", "author:pka"]

        [files]
        "." = ["gitrepo"]
        "Cargo.toml" = ["format:toml"]
    "#;
    let cwd = env::current_dir()?;
    let doctags = DocTags::from_toml(&cwd, toml.to_string());
    assert!(doctags.is_ok());

    let toml = "tags =";
    let doctags = DocTags::from_toml(&cwd, toml.to_string());
    assert_eq!(
        format!("#{:?}", doctags),
        "#Err(unexpected eof encountered at line 1 column 7)"
    );
    Ok(())
}
