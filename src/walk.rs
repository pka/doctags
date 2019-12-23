use ignore::WalkBuilder;

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
    F: FnMut(&str, bool),
{
    let walker = WalkBuilder::new(basedir)
        .follow_links(true)
        .same_file_system(SAME_FS_SUPPORTED)
        .build();
    for entry in walker {
        if let Ok(entry) = entry {
            if let Some(path) = entry.path().to_str() {
                out(&path, entry.file_type().unwrap().is_dir());
            }
        }
    }
}
