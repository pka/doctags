#[allow(dead_code)]
mod path_util;

use crate::path_util::is_hidden;
use walkdir::{DirEntry, WalkDir};

/// Returns `true` if this entry should be included in scans.
fn filter_dirs(entry: &DirEntry) -> bool {
    entry.file_type().is_dir() && (!is_hidden(entry) || entry.file_name() == ".git")
}

/// Walks the configured base directory, looking for git repos.
fn find_repos(basedir: &str) -> Vec<String> {
    let mut repos = Vec::new();
    for entry in WalkDir::new(basedir)
        .follow_links(true)
        .same_file_system(true)
        .into_iter()
        .filter_entry(|e| filter_dirs(e))
    {
        if let Ok(entry) = entry {
            if entry.file_name() == ".git" {
                let parent_path = entry.path().parent().expect("Could not determine parent.");
                if let Some(path) = parent_path.to_str() {
                    // if git2::Repository::open(&path).is_ok() {
                    repos.push(path.to_string());
                }
            }
        }
    }
    repos
}

fn main() {
    println!("{:?}", find_repos(".."));
}
