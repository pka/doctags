use crate::doctags::{read_doctags_file, DocTags};
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use indicatif::{FormattedDuration, ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::Instant;

pub struct DocTagsStackEntry {
    /// id of current directory
    id: u64,
    /// doctags of current directory
    doctags: DocTags,
}

type DocTagsStack = Vec<DocTagsStackEntry>;

/// Collect tags of traversed directories
pub fn all_tags<'a>(stack: &'a DocTagsStack, path: String) -> Vec<&'a String> {
    lazy_static! {
        static ref NO_TAGS: Vec<String> = vec![];
    }
    stack
        .iter()
        // collect dirtags
        .flat_map(|entry| &entry.doctags.dirtags)
        // append filtetags if path has matching entry
        .chain({
            let filetags_entry = &stack[stack.len() - 1].doctags;
            if let Some(filetags) = filetags_entry.filetags.get(&path) {
                filetags.iter()
            } else {
                NO_TAGS.iter()
            }
        })
        .collect()
}

#[cfg(any(unix, windows))]
const SAME_FS_SUPPORTED: bool = true;

#[cfg(not(any(unix, windows)))]
const SAME_FS_SUPPORTED: bool = false;

/// Find files
pub fn find<F>(basedirs: &Vec<String>, mut out: F) -> Result<()>
where
    F: FnMut(u64, u64, &str, &Vec<&String>),
{
    let mut id: u64 = 1; // we use doc ids > 1 (FUSE root inode)
    let pb = bar();
    let started = Instant::now();
    for basedir in basedirs {
        let path = Path::new(basedir).canonicalize()?;
        let walker = WalkBuilder::new(path)
            .follow_links(true)
            .same_file_system(SAME_FS_SUPPORTED)
            .build();
        let mut depth = 0;
        let mut doctags_stack: DocTagsStack = vec![];
        doctags_stack.reserve(10);
        for entry in walker {
            if let Ok(entry) = entry {
                if entry.depth() > depth {
                    depth = entry.depth();
                } else if entry.depth() < depth {
                    depth = entry.depth();
                    doctags_stack.truncate(depth);
                }
                id += 1;
                let parent_id = if depth > 0 {
                    doctags_stack[doctags_stack.len() - 1].id
                } else {
                    std::u64::MAX
                };
                if entry
                    .file_type()
                    .context("Couldn't detect file type")?
                    .is_dir()
                {
                    let stack_entry = DocTagsStackEntry {
                        id,
                        doctags: read_doctags_file(entry.path(), false),
                    };
                    doctags_stack.push(stack_entry);
                }
                if let Some(path) = entry.path().to_str() {
                    let tags = all_tags(&doctags_stack, path.to_string());
                    out(id, parent_id, &path, &tags);
                    pb.inc(1);
                    pb.set_message(path);
                }
            }
        }
    }
    pb.set_message(&format!(
        "files indexed [{}].",
        FormattedDuration(started.elapsed())
    ));
    pb.finish_at_current_pos();
    Ok(())
}

fn bar() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_draw_delta(101);
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠏", "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇"])
            .template("{spinner:.blue} {pos} {wide_msg}"),
    );
    return pb;
}

#[test]
fn collect_tags() -> Result<()> {
    use std::env;

    let toml = r#"
        tags = ["lang:rust", "author:pka"]

        [files]
        "." = ["gitrepo"]
        "Cargo.toml" = ["format:toml"]
    "#;
    let cwd = env::current_dir()?;
    let doctags = DocTags::from_toml(&cwd, toml.to_string())?;
    let doctags_stack = vec![DocTagsStackEntry { id: 3, doctags }];

    let path = cwd.to_string_lossy().to_string();
    assert_eq!(
        all_tags(&doctags_stack, path),
        vec!["/lang/rust", "/author/pka", "/gitrepo"]
    );

    let path = cwd.join("Cargo.toml").to_string_lossy().to_string();
    assert_eq!(
        all_tags(&doctags_stack, path),
        vec!["/lang/rust", "/author/pka", "/format/toml"]
    );

    let path = cwd.join("Cargo.lock").to_string_lossy().to_string();
    assert_eq!(
        all_tags(&doctags_stack, path),
        vec!["/lang/rust", "/author/pka"]
    );

    // without facet conversion
    let doctags = toml::from_str(&toml)?;
    let doctags_stack = vec![DocTagsStackEntry { id: 3, doctags }];

    let path = cwd.to_string_lossy().to_string();
    assert_eq!(
        all_tags(&doctags_stack, path),
        vec!["lang:rust", "author:pka"]
    );
    Ok(())
}
