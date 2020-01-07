#[macro_use]
extern crate log;

mod fusefs;
mod vfs;

use ::doctags::{config, index};
use std::env;
use std::ffi::OsStr;
use vfs::DoctagsFS;

const DOCSET: &str = "test";

fn main() {
    env_logger::init();
    let config = config::load_config();
    let cfg = config
        .docset_config(&DOCSET.to_string())
        .expect("Docset config missing");
    let index = index::open(&cfg.index).unwrap();
    let fs = DoctagsFS {
        index,
        entries: vec![],
    };
    let mountpoint = env::args_os().nth(1).unwrap();
    let options = ["-o", "ro", "-o", "fsname=doctags"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();
    fuse::mount(fs, &mountpoint, &options).unwrap();
}
