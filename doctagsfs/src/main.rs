#[macro_use]
extern crate log;

mod fusefs;
mod vfs;

use ::doctags::{config, index};
use std::env;
use std::ffi::OsStr;
use vfs::DoctagsFS;

fn main() {
    let mountpoint = env::args_os().nth(1).expect("mount point expected");
    let docset = std::env::args().nth(2).unwrap_or("default".to_string());
    env_logger::init();
    let config = config::load_config();
    let cfg = config
        .docset_config(&docset)
        .expect("Docset config missing");
    let index = index::open(&cfg.index).unwrap();
    let mut fs = DoctagsFS::new(index);
    fs.create_vfs_tree();
    let options = ["-o", "ro", "-o", "fsname=doctags"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();
    fuse::mount(fs, &mountpoint, &options).unwrap();
}
