#[allow(dead_code)]
mod path_util;

use crate::path_util::is_hidden;
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

// Returns `true` if this entry should be included in scans.
fn filter_nongit_dirs(entry: &DirEntry) -> bool {
    entry.file_type().is_dir() && (!is_hidden(entry) || entry.file_name() == ".git")
}

/// Find directories containing git repos
fn find_repos(basedir: &str) -> Vec<String> {
    let mut repos = Vec::new();
    for entry in WalkDir::new(basedir)
        .follow_links(true)
        .same_file_system(true)
        .into_iter()
        .filter_entry(|e| filter_nongit_dirs(e))
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

/// Find files
fn find(basedir: &str) -> Vec<String> {
    let mut dirs = Vec::new();
    for entry in WalkDir::new(basedir)
        .follow_links(true)
        .same_file_system(true)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        if let Ok(entry) = entry {
            dirs.push(entry.path().display().to_string());
        }
    }
    dirs
}

#[derive(Debug, StructOpt)]
struct Cli {
    /// Git repo search
    #[structopt(short, long)]
    git: bool,

    /// Input file to read
    basedir: String,
}

fn main() {
    let args = Cli::from_args();
    let dirs = if args.git {
        find_repos(&args.basedir)
    } else {
        find(&args.basedir)
    };
    println!("{:?}", dirs);
}
