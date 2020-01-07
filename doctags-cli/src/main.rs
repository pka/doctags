#[macro_use]
extern crate log;

use ::doctags::{config, doctags, index, search, walk};
use std::io::Write;
use structopt::StructOpt;

fn out_json(_id: u64, _parent_id: u64, path: &str, tags: &Vec<&String>) {
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
    /// Create search index
    Index {
        /// Docset name
        #[structopt(short = "n", long, name = "name", default_value = "default")]
        docset: String,
        /// Index directory
        #[structopt(short, long, name = "path")]
        index: Option<String>,
        /// Base directory for searching files to index
        basedir: String,
    },
    /// Recreate search index
    Reindex {
        /// Docset name
        #[structopt(short = "n", long, name = "name", default_value = "default")]
        docset: String,
    },
    /// Add tag to file
    Tag {
        /// Tag also subdirs
        #[structopt(short, long, parse(try_from_str), default_value = "true")]
        recursive: bool,
        /// File or directory
        path: String,
        /// Tag
        tag: String,
    },
    /// Search in index
    Search {
        /// Limit count of returned results. Use 0 for unlimited results.
        #[structopt(short, long, default_value = "10")]
        limit: usize,
        /// Docset name
        #[structopt(short = "n", long, name = "name", default_value = "default")]
        docset: String,
        /// Search text
        text: String,
    },
    /// Get statistics
    Stats {},
}

fn setup_logger() {
    match std::env::var("RUST_LOG") {
        Ok(_) => env_logger::builder()
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{} {} {}",
                    record.level(),
                    record.target(),
                    record.args()
                )
            })
            .init(),
        Err(_) => {
            std::env::set_var("RUST_LOG", "doctags=info");
            env_logger::builder()
                .format(|buf, record| writeln!(buf, "{} {}", record.level(), record.args()))
        }
        .init(),
    }
}

fn main() {
    setup_logger();
    match Cli::from_args() {
        Cli::Scan { git, basedir } => {
            if git {
                // walk::find_repos(&basedir, out_json);
            } else {
                walk::find(&basedir, out_json);
            }
        }
        Cli::Index {
            docset,
            index,
            basedir,
        } => {
            let mut config = config::load_config();
            let newcfg = config::docset_config(docset, index, basedir);
            info!("Writing configuration to {:?}", config::config_fn());
            let cfg = config.update_docset_config(newcfg).unwrap();
            index::create_and_write(&cfg.basedir, &cfg.index);
        }
        Cli::Reindex { docset } => {
            let config = config::load_config();
            let cfg = config
                .docset_config(&docset)
                .expect("Docset config missing");
            index::create_and_write(&cfg.basedir, &cfg.index);
        }
        Cli::Tag {
            path,
            tag,
            recursive,
        } => {
            doctags::add_tag(path, tag, recursive);
        }
        Cli::Search {
            docset,
            text,
            limit,
        } => {
            let config = config::load_config();
            let cfg = config
                .docset_config(&docset)
                .expect("Docset config missing");
            let index = index::open(&cfg.index).unwrap();
            search::search(&index, text, limit).unwrap();
        }
        Cli::Stats {} => {
            println!("Configuration {:?}", config::config_fn());
            let config = config::load_config();
            for cfg in config.docsets.iter().rev() {
                println!("Docset '{}':", cfg.name);
                let index = index::open(&cfg.index).unwrap();
                search::stats(&index).unwrap();
            }
        }
    }
}