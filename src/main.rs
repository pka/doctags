#[macro_use]
extern crate log;

mod index;
#[allow(dead_code)]
mod search;
mod walk;

use structopt::StructOpt;

fn out_json(path: &str, tags: &Vec<&String>) {
    println!(r#"{{"path":"{}","tags":{:?}}}"#, path, tags);
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
    Count {
        /// index directory
        index: String,
        /// Search text
        text: String,
    },
}

fn main() {
    env_logger::init();
    match Cli::from_args() {
        Cli::Scan { git, basedir } => {
            if git {
                // walk::find_repos(&basedir, out_json);
            } else {
                walk::find(&basedir, out_json);
            }
        }
        Cli::Index { index, basedir } => {
            let mut index_writer = index::create(&index).unwrap();
            walk::find(&basedir, |path, tags| index_writer.add(path, tags).unwrap());
            let _ = index_writer.commit();
        }
        Cli::Search { index, text } => {
            let index = index::open(index).unwrap();
            search::search(&index, text).unwrap();
        }
        Cli::Count { index, text } => {
            let index = index::open(index).unwrap();
            search::count(&index, text).unwrap();
        }
    }
}
