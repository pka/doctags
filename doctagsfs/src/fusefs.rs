use ::doctags::{search, Index};
use fuse::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::linux::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::SystemTime;
use time::Timespec;

const TTL: Timespec = Timespec { sec: 1, nsec: 0 }; // 1 second

pub struct DoctagsFS {
    pub index: Index,
}

const BASEDIR: &str = "/home/pi/code/rust/doctags";

impl Filesystem for DoctagsFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        debug!("lookup parent: {} name: {}", parent, name.to_str().unwrap());
        if let Ok(Some((id, path))) = search::file_from_dir_entry(&self.index, parent, name) {
            if let Ok(attr) = file_attr(id, &path) {
                reply.entry(&TTL, &attr, 0);
                return;
            }
        } else {
            if parent == 1 {
                if let Ok(attr) = file_attr(2, &BASEDIR.to_string()) {
                    reply.entry(&TTL, &attr, 0);
                    return;
                }
            }
        }
        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        debug!("getattr ino: {}", ino);
        if let Ok(Some((id, path))) = search::file_from_id(&self.index, ino) {
            if let Ok(attr) = file_attr(id, &path) {
                reply.attr(&TTL, &attr);
                return;
            }
        } else {
            if ino == 1 {
                if let Ok(attr) = file_attr(2, &BASEDIR.to_string()) {
                    reply.attr(&TTL, &attr);
                    return;
                }
            }
        }
        reply.error(ENOENT);
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: ReplyData,
    ) {
        debug!("read ino: {}", ino);
        if let Ok(Some((_id, path))) = search::file_from_id(&self.index, ino) {
            if let Ok(mut f) = File::open(path) {
                f.seek(SeekFrom::Start(offset as u64)).unwrap();
                let mut data = Vec::with_capacity(size as usize);
                data.resize(size as usize, 0);
                f.read(&mut data).unwrap();
                reply.data(&data);
                return;
            }
        }
        reply.error(ENOENT);
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        debug!("readdir ino: {}", ino);
        let dot_entries = vec![
            (ino, FileType::Directory, "."),
            (ino, FileType::Directory, ".."),
        ];
        for (i, entry) in dot_entries.into_iter().enumerate().skip(offset as usize) {
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2);
        }
        if let Ok(docs) = search::files_from_parent_id(&self.index, ino) {
            for (i, (id, path)) in docs
                .iter()
                .enumerate()
                .skip(offset.saturating_sub(2) as usize)
            {
                if let Ok((ft, basename)) = dir_entry(path) {
                    debug!("[{}] {:?}", id, basename);
                    reply.add(*id, (i + 3) as i64, ft, basename);
                }
            }
        }
        reply.ok();
    }
}

fn dir_entry<'a>(path: &'a String) -> std::io::Result<(FileType, &'a OsStr)> {
    let attr = fs::metadata(path)?;
    let ft = if attr.is_dir() {
        FileType::Directory
    } else {
        FileType::RegularFile
    };
    let basename = Path::new(path).file_name().unwrap();
    Ok((ft, basename))
}

fn timespec(st: &SystemTime) -> Timespec {
    if let Ok(dur_since_epoch) = st.duration_since(std::time::UNIX_EPOCH) {
        Timespec::new(
            dur_since_epoch.as_secs() as i64,
            dur_since_epoch.subsec_nanos() as i32,
        )
    } else {
        Timespec::new(0, 0)
    }
}

fn file_attr(id: u64, path: &String) -> std::io::Result<FileAttr> {
    let meta = fs::metadata(path)?;
    let ft = if meta.is_dir() {
        FileType::Directory
    } else {
        FileType::RegularFile
    };
    let fattr = FileAttr {
        ino: id,
        size: meta.len(),
        blocks: meta.st_blocks(),
        atime: timespec(&meta.accessed().unwrap()),
        mtime: timespec(&meta.modified().unwrap()),
        ctime: timespec(&meta.created().unwrap()),
        crtime: timespec(&meta.created().unwrap()),
        kind: ft,
        perm: meta.permissions().mode() as u16,
        nlink: meta.st_nlink() as u32,
        uid: meta.st_uid(),
        gid: meta.st_gid(),
        rdev: meta.st_rdev() as u32,
        flags: 0,
    };
    Ok(fattr)
}
