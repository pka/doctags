use ignore::WalkBuilder;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml::{self, Value};

// Returns `true` if this entry should be included in scans.
// fn filter_nongit_dirs(entry: &DirEntry) -> bool {
//     entry.file_type().is_dir() && (!is_hidden(entry) || entry.file_name() == ".git")
// }

/// Find directories containing git repos
// pub fn find_repos<F>(basedir: &str, mut out: F)
// where
//     F: FnMut(&str, bool),
// {
//     for entry in WalkDir::new(basedir)
//         .follow_links(true)
//         .same_file_system(true)
//         .into_iter()
//         .filter_entry(|e| filter_nongit_dirs(e))
//     {
//         if let Ok(entry) = entry {
//             if entry.file_name() == ".git" {
//                 let parent_path = entry.path().parent().expect("Could not determine parent.");
//                 if let Some(path) = parent_path.to_str() {
//                     // if git2::Repository::open(&path).is_ok() {
//                     out(&path, true);
//                 }
//             }
//         }
//     }
// }

#[cfg(any(unix, windows))]
const SAME_FS_SUPPORTED: bool = true;

#[cfg(not(any(unix, windows)))]
const SAME_FS_SUPPORTED: bool = false;

struct DocTags {
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

fn read_doctags_file(dir: &Path) -> DocTags {
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

fn all_tags<'a>(
    stack: &'a DocTagsStack,
    path: String,
    is_subdir: bool,
    no_tags: &'a Vec<String>,
) -> Vec<&'a String> {
    stack
        .iter()
        // collect dirtags
        .flat_map(|dt| &dt.dirtags)
        // append filtetags if path has matching entry
        .chain({
            let filetags_entry = if is_subdir {
                &stack[stack.len() - 2] // config from parent dir
            } else {
                &stack[stack.len() - 1]
            };
            if let Some(filetags) = filetags_entry.filetags.get(&path) {
                filetags.iter()
            } else {
                no_tags.iter()
            }
        })
        .collect()
}

/// Find files
pub fn find<F>(basedir: &str, mut out: F)
where
    F: FnMut(&str, &Vec<&String>),
{
    let path = Path::new(basedir).canonicalize().unwrap();
    let walker = WalkBuilder::new(path)
        .follow_links(true)
        .same_file_system(SAME_FS_SUPPORTED)
        .build();
    let mut depth = 0;
    // flattened tags for current depth
    let mut doctags_stack = vec![];
    doctags_stack.reserve(10);
    for entry in walker {
        if let Ok(entry) = entry {
            if entry.depth() > depth {
                depth = entry.depth();
            } else if entry.depth() < depth {
                depth = entry.depth();
                doctags_stack.truncate(depth);
            }
            if entry.file_type().unwrap().is_dir() {
                doctags_stack.push(read_doctags_file(entry.path()));
            }
            if let Some(path) = entry.path().to_str() {
                let no_tags: Vec<String> = vec![];
                let tags = all_tags(
                    &doctags_stack,
                    path.to_string(),
                    entry.file_type().unwrap().is_dir() && depth > 0,
                    &no_tags,
                );
                out(&path, &tags);
            }
        }
    }
}
