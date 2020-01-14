#[macro_use]
extern crate log;

mod fusefs;
mod vfs;

use ::doctags::{config, index};
use anyhow::{Context, Result};
use fork::{daemon, Fork};
use std::env;
use std::ffi::OsStr;
use vfs::DoctagsFS;

fn main() -> Result<()> {
    // mount helper options (https://linux.die.net/man/8/mount):
    // /sbin/mount.<suffix> spec dir [-sfnv] [-o options] [-t type.subtype]
    let docset = std::env::args().nth(1).context("docset expected")?;
    let mountpoint = env::args_os().nth(2).context("mount point expected")?;
    env_logger::init();
    let config = config::load_config()?;
    let cfg = config.docset_config(&docset)?;
    let index = index::open(&cfg.index)?;
    let mut fs = DoctagsFS::new(index);
    fs.create_vfs_tree()?;
    let options = ["-o", "ro", "-o", "fsname=doctags"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    if let Ok(Fork::Child) = daemon(false, true) {
        fuse::mount(fs, &mountpoint, &options)?;
    }
    Ok(())
}
