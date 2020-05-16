#[macro_use]
extern crate log;

mod ui;

use ::doctags::{config, doctags, index, search};
use anyhow::Result;
use std::io::Write;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Cli {
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
    /// Start interactive search Ui
    Ui {
        /// Docset name
        #[structopt(short = "n", long, name = "name", default_value = "default")]
        docset: String,
        #[structopt(long)]
        /// Where to write the produced cmd (if any)
        outcmd: Option<String>,
        /// Print directory selected with Alt-c
        #[structopt(short, long, parse(try_from_str), default_value = "false")]
        printcd: bool,
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

fn command(cli_args: Cli) -> Result<()> {
    match cli_args {
        Cli::Index {
            docset,
            index,
            basedir,
        } => {
            let mut config = config::load_config()?;
            let newcfg = config::docset_config(docset, index, vec![basedir])?;
            info!("Writing configuration to {:?}", config::config_fn());
            let cfg = config.update_docset_config(newcfg)?;
            index::create_and_write(&cfg.basedirs, &cfg.index)?;
        }
        Cli::Reindex { docset } => {
            let config = config::load_config()?;
            let cfg = config.docset_config(&docset)?;
            index::create_and_write(&cfg.basedirs, &cfg.index)?;
        }
        Cli::Tag {
            path,
            tag,
            recursive,
        } => doctags::add_tag(path, tag, recursive)?,
        Cli::Search {
            docset,
            text,
            limit,
        } => {
            let config = config::load_config()?;
            let cfg = config.docset_config(&docset)?;
            let index = index::open(&cfg.index)?;
            search::search(&index, text, limit)?;
        }
        Cli::Ui { docset, outcmd, printcd } => {
            let config = config::load_config()?;
            let cfg = config.docset_config(&docset)?;
            let index = index::open(&cfg.index)?;
            ui::ui(&index, outcmd, printcd)?;
        }
        Cli::Stats {} => {
            println!("Configuration {:?}", config::config_fn());
            let config = config::load_config()?;
            for cfg in config.docsets.iter().rev() {
                println!("Docset '{}':", cfg.name);
                let index = index::open(&cfg.index)?;
                search::stats(&index)?;
            }
        }
    }
    Ok(())
}

fn main() {
    setup_logger();
    match command(Cli::from_args()) {
        Err(e) => {
            if let Some(source) = e.source() {
                error!("{} ({})", e, source);
            } else {
                error!("{}", e);
            }
        }
        Ok(_) => (),
    }
}
