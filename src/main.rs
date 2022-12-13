
extern crate libc;
extern crate time;
extern crate fuse;
use std::env;

use std::ffi::OsStr;
use std::path::Path;
use libc::{ENOENT, ENOSYS, EPERM};
use time::Timespec;
use fuse::{FileAttr, FileType, Filesystem, Request, ReplyAttr, ReplyWrite,ReplyData, ReplyEntry, ReplyDirectory, ReplyCreate, ReplyEmpty,ReplyStatfs, ReplyOpen};
// use std::collections::BTreeMap;
extern crate serde_json;

struct GithubFilesystem;


impl Filesystem for GithubFilesystem {
    fn destroy(&mut self, _req: &Request) {
        println!("destroy");
    }
    fn forget(&mut self, _req: &Request, _ino: u64, _nlookup: u64) {
        println!("forget");
    }
    fn getattr(&mut self, _req: &Request, _ino: u64, reply: ReplyAttr) {
        println!("getattr(ino={}, _req={:?})", _ino, _req);
        let ts = Timespec::new(0, 0);
    let attr = FileAttr {
        ino: 1,
        size: 0,
        blocks: 0,
        atime: ts,
        mtime: ts,
        ctime: ts,
        crtime: ts,
        kind: FileType::Directory,
        perm: 0o755,
        nlink: 0,
        uid: 0,
        gid: 0,
        rdev: 0,
        flags: 0,
    };
    let ttl = Timespec::new(1, 0);
    if _ino == 1 {
        reply.attr(&ttl, &attr);
    } else {
        reply.error(ENOSYS);
    }
    }
    fn setattr(&mut self,_req: &Request,_ino: u64,_mode: Option<u32>,_uid: Option<u32>,_gid: Option<u32>,_size: Option<u64>,_atime: Option<Timespec>,_mtime: Option<Timespec>,_fh: Option<u64>,_crtime: Option<Timespec>,_chgtime: Option<Timespec>,_bkuptime: Option<Timespec>,_flags: Option<u32>, reply: ReplyAttr) {
        println!("setattr");
        reply.error(ENOSYS)
    }
    fn readlink(&mut self, _req: &Request, _ino: u64, reply: ReplyData) {
        println!("readlink");
        reply.error(ENOSYS)
    }
    fn open(&mut self, _req: &Request, _ino: u64, _flags: u32, reply: ReplyOpen) {
        println!("open");
        reply.error(ENOSYS)
    }
    fn read(&mut self,_req: &Request,_ino: u64,_fh: u64,_offset: i64,_size: u32, reply: ReplyData) {
        println!("read");
        reply.error(ENOSYS)
    }
    fn write(&mut self,_req: &Request,_ino: u64,_fh: u64,_offset: i64,_data: &[u8],_flags: u32, reply: ReplyWrite) {
        println!("write");
        reply.error(ENOSYS)
    }
    fn flush(&mut self,_req: &Request,_ino: u64,_fh: u64,_lock_owner: u64, reply: ReplyEmpty) {
        println!("flush");
        reply.error(ENOSYS)
    }
    fn release(&mut self,_req: &Request,_ino: u64,_fh: u64,_flags: u32,_lock_owner: u64,_flush: bool, reply: ReplyEmpty) {
        println!("release");
        reply.error(ENOSYS)
    }
    fn fsync(&mut self,_req: &Request,_ino: u64,_fh: u64,_datasync: bool, reply: ReplyEmpty) {
        println!("fsync");
        reply.error(ENOSYS)
    }
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        println!("lookup(parent={}, name={:?})", parent, name.to_str());
      
        let ttl = Timespec::new(1, 0);
        let ts = time::now().to_timespec();
        let attr = FileAttr {
            ino: 1,
            size: 0,
            blocks: 0,
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        reply.entry(&ttl, &attr, 0);
    }
    fn opendir(&mut self,_req: &Request,_ino: u64,_flags: u32, reply: ReplyOpen) {
        println!("opendir(ino={}, _flags={})", _ino, _flags);
      
        reply.opened(_ino, _flags);
    }
    fn readdir(&mut self,_req: &Request,_ino: u64,_fh: u64,_offset: i64, mut reply: ReplyDirectory) {
        println!("readdir(ino={}, _fh={}, _offset={})", _ino, _fh, _offset);
        if _ino == 1 {
            if _offset == 0 {
                reply.add(1, 0, FileType::Directory, &Path::new("."));
                reply.add(1, 1, FileType::Directory, &Path::new(".."));
            }
            reply.ok();
        } else {
            reply.error(ENOSYS);
        }
    }
    fn releasedir(&mut self,_req: &Request,_ino: u64,_fh: u64,_flags: u32, reply: ReplyEmpty) {
        println!("releasedir(ino={}, _fh={}, _flags={})", _ino, _fh, _flags);
        reply.error(ENOSYS)
    }
    fn fsyncdir(&mut self,_req: &Request,_ino: u64,_fh: u64,_datasync: bool, reply: ReplyEmpty) {
        println!("fsyncdir");
        reply.error(ENOSYS)
    }
    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        println!("statfs");
        reply.error(ENOSYS)
    }
    fn access(&mut self,_req: &Request,_ino: u64,_mask: u32, reply: ReplyEmpty) {
        println!("readdir(ino={}, _mask={})", _ino, _mask);
        reply.error(ENOSYS)
    }
   
}

fn main() {
    let mountpoint = match env::args().nth(1) {
        Some(path) => path,
        None => {
            println!("Usage: {} <MOUNTPOINT>", env::args().nth(0).unwrap());
            return;
        }
    };
    fuse::mount(GithubFilesystem, &mountpoint, &[]);
}