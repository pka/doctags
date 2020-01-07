use serde_derive::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml::{self, Value};

#[derive(Debug, Serialize)]
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
    pub fn from_toml(
        dir: &Path,
        toml: String,
        as_facets: bool,
    ) -> Result<DocTags, toml::de::Error> {
        let config: Value = toml::from_str(&toml)?;
        let dirtags = if let Some(tags) = config.get("tags") {
            tag_value_to_vec(tags, as_facets)
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
                            tag_value_to_vec(tags, as_facets),
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

fn tag_value_to_vec(value: &Value, as_facets: bool) -> Vec<String> {
    value
        .as_array()
        .expect("tags must be array type")
        .iter()
        .map(|tag| {
            let tagstr = tag.as_str().expect("tag must be string");
            if as_facets {
                facet(tagstr)
            } else {
                tagstr.to_string()
            }
        })
        .collect()
}

pub fn read_doctags_file(dir: &Path, as_facets: bool) -> DocTags {
    let path = dir.join(".doctags.toml");
    if path.exists() {
        if let Ok(toml) = fs::read_to_string(path) {
            if let Ok(doctags) = DocTags::from_toml(dir, toml, as_facets) {
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
    let mut p = Path::new(&path);
    if !p.exists() {
        error!("File '{}' does not exist", path);
        return;
    }
    let path = p.canonicalize().unwrap().to_string_lossy().to_string();
    let is_dir_tag = p.is_dir() && recursive;
    if p.is_file() {
        p = p.parent().expect("dirname not found");
    }
    let toml_path = p.join(".doctags.toml");
    let mut doctags = read_doctags_file(p, false);
    if is_dir_tag {
        doctags.dirtags.push(tag);
    } else {
        let filetags = doctags.filetags.entry(path).or_insert(vec![]);
        (*filetags).push(tag);
    }
    debug!("Writing {:?}", toml_path);

    let toml = toml::to_string(&doctags).unwrap();
    fs::write(toml_path, toml).expect("Couldn't write config file");
}
