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
    pub fn from_toml(dir: &Path, toml: String) -> Result<DocTags, toml::de::Error> {
        let config: Value = toml::from_str(&toml)?;
        let dirtags = if let Some(tags) = config.get("tags") {
            tag_value_to_vec(tags)
        } else {
            vec![]
        };
        let mut filetags = HashMap::new();
        if let Some(filetable) = config.get("files") {
            filetable
                .as_table()
                .expect(&format!(
                    "tags must be table type in {}/.doctags.toml",
                    dir.display()
                ))
                .iter()
                .for_each(|(fname, tags)| {
                    if let Ok(fullpath) = dir.join(fname).canonicalize() {
                        filetags.insert(
                            fullpath.to_string_lossy().to_string(),
                            tag_value_to_vec(tags),
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

fn tag_value_to_vec(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("tags must be array type")
        .iter()
        .map(|tag| {
            let tagstr = tag.as_str().expect("tag must be string");
            facet(tagstr)
        })
        .collect()
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

pub fn add_tag(path: String, tag: String, recursive: bool) {
    let p = Path::new(&path);
    if !p.exists() {
        error!("File '{}' does not exist", path);
        return;
    }
    let is_dir_tag = p.is_dir() && recursive;
    let dirp = if p.is_file() {
        p.parent().expect("dirname not found")
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
            p.strip_prefix(dirp).unwrap().to_string_lossy().to_string()
        };
        let filetags = doctags.filetags.entry(relpath).or_insert(vec![]);
        (*filetags).push(tag);
    }
    debug!("Writing {:?}", toml_path);

    let toml = toml::to_string(&doctags).unwrap();
    fs::write(toml_path, toml).expect("Couldn't write config file");
}
