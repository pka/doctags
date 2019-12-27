use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml::{self, Value};

pub struct DocTags {
    dirtags: Vec<String>,
    filetags: HashMap<String, Vec<String>>,
}

fn facet(tag: &str) -> String {
    format!("/{}", tag.replace(":", "/"))
}

impl DocTags {
    fn from_toml(dir: &Path, toml: String) -> Result<DocTags, toml::de::Error> {
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
                .expect("tags must be table type")
                .iter()
                .for_each(|(fname, tags)| {
                    filetags.insert(
                        dir.join(fname)
                            .canonicalize()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        tag_value_to_vec(tags),
                    );
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
        .map(|tag| facet(tag.as_str().expect("tag must be string")))
        .collect()
}

pub fn read_doctags_file(dir: &Path) -> DocTags {
    let path = dir.join(".doctags.toml");
    if path.exists() {
        if let Ok(toml) = fs::read_to_string(path) {
            if let Ok(doctags) = DocTags::from_toml(dir, toml) {
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
