use crate::doctags::{all_tags, read_doctags_file};
use ignore::WalkBuilder;
use std::path::Path;

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
                doctags_stack.push(read_doctags_file(entry.path(), true));
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
