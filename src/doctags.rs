use serde_derive::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml::{self, Value};

#[derive(Debug, Serialize)]
pub struct DocTags {
    #[serde(rename = "tags", default)]
    dirtags: Vec<String>,
    #[serde(rename = "files", default)]
    filetags: HashMap<String, Vec<String>>,
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
    fn from_toml(dir: &Path, toml: String, as_facets: bool) -> Result<DocTags, toml::de::Error> {
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
                .expect("tags must be table type")
                .iter()
                .for_each(|(fname, tags)| {
                    filetags.insert(
                        dir.join(fname)
                            .canonicalize()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        tag_value_to_vec(tags, as_facets),
                    );
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

type DocTagsStack = Vec<DocTags>;

pub fn all_tags<'a>(
    stack: &'a DocTagsStack,
    path: String,
    _is_subdir: bool,
    no_tags: &'a Vec<String>,
) -> Vec<&'a String> {
    stack
        .iter()
        // collect dirtags
        .flat_map(|dt| &dt.dirtags)
        // append filtetags if path has matching entry
        .chain({
            // filetags for directories in parent dir not supported for now
            // let filetags_entry = if is_subdir {
            //     &stack[stack.len() - 2] // config from parent dir
            // } else {
            //     &stack[stack.len() - 1]
            // };
            let filetags_entry = &stack[stack.len() - 1];
            if let Some(filetags) = filetags_entry.filetags.get(&path) {
                filetags.iter()
            } else {
                no_tags.iter()
            }
        })
        .collect()
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
