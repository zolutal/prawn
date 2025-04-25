use libc::ENOENT;
use std::{
    ffi::OsStr,
    time::{Duration, UNIX_EPOCH},
};

const FUSE_DIR_ATTR: fuse::FileAttr = fuse::FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: fuse::FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 1000,
    gid: 1000,
    rdev: 0,
    flags: 0,
};

const FUSE_FILE_ATTR: fuse::FileAttr = fuse::FileAttr {
    ino: 2,
    size: 4096,
    blocks: 1,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: fuse::FileType::RegularFile,
    perm: 0o666,
    nlink: 1,
    uid: 1000,
    gid: 1000,
    rdev: 0,
    flags: 0,
};

pub trait FuseReadHandler {
    fn on_read(&mut self);
}

pub struct FuseFS<T: FuseReadHandler> {
    pub handler: T,
}

impl<T: FuseReadHandler + 'static> fuse::Filesystem for FuseFS<T> {
    fn lookup(&mut self, _req: &fuse::Request, parent: u64, name: &OsStr, reply: fuse::ReplyEntry) {
        //dbg!(name);
        if parent == 1 && name.to_str() == Some("pwn") {
            reply.entry(&Duration::from_secs(1), &FUSE_FILE_ATTR, 0);
            return;
        }
        reply.error(ENOENT);
    }

    fn readdir(
        &mut self,
        _req: &fuse::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuse::ReplyDirectory,
    ) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let entries = vec![
            (1, fuse::FileType::Directory, "."),
            (1, fuse::FileType::Directory, ".."),
            (2, fuse::FileType::RegularFile, "pwn"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2);
        }
        reply.ok();
    }

    fn getattr(&mut self, _req: &fuse::Request, ino: u64, reply: fuse::ReplyAttr) {
        match ino {
            1 => reply.attr(&Duration::from_secs(1), &FUSE_DIR_ATTR),
            2 => reply.attr(&Duration::from_secs(1), &FUSE_FILE_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &fuse::Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _size: u32,
        reply: fuse::ReplyData,
    ) {
        self.handler.on_read();
        let data: Vec<u8> = vec![0; 4096];
        reply.data(&data);
    }
}
