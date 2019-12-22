mod index;
#[allow(dead_code)]
mod path_util;
mod search;
mod walk;

use structopt::StructOpt;

fn out_json(entry: &str) {
    println!(r#"{{"path":"{}"}}"#, entry);
}

#[derive(Debug, StructOpt)]
enum Cli {
    Scan {
        /// Git repo search
        #[structopt(long)]
        git: bool,

        /// Base directory for searching files to index
        basedir: String,
    },
    Index {
        /// index directory
        index: String,
        /// Base directory for searching files to index
        basedir: String,
    },
    Search {
        /// index directory
        index: String,
        /// Search text
        text: String,
    },
}

fn main() {
    match Cli::from_args() {
        Cli::Scan { git, basedir } => {
            if git {
                walk::find_repos(&basedir, out_json);
            } else {
                walk::find(&basedir, out_json);
            }
        }
        Cli::Index { index, basedir } => {
            let mut index_writer = index::create(&index).unwrap();
            walk::find(&basedir, |entry| index_writer.add(entry).unwrap());
            let _ = index_writer.commit();
        }
        Cli::Search { index, text } => {
            let index = index::open(index).unwrap();
            search::search(&index, text).unwrap();
        }
    }
}
