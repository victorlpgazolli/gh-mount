
extern crate libc;
extern crate time;
extern crate fuse;
use std::env;
use std::ffi::OsStr;
use libc::c_int;
use std::path::Path;
use libc::{EEXIST,ENOENT,ENOSYS};
use time::Timespec;
use fuse::{FileAttr, FileType, Filesystem, Request, ReplyAttr, ReplyWrite,ReplyData, ReplyEntry, ReplyDirectory,  ReplyEmpty,ReplyStatfs, ReplyOpen};
use std::process::Command;
use std::collections::HashMap;

extern crate serde_json;
struct GithubVirtualFileSystem {
   repositoriesPerUser: HashMap<String, Vec<String>>,
   inodes: HashMap<String, u64>,
   attrs: HashMap<u64, FileAttr>
}


impl GithubVirtualFileSystem{
    fn new() -> GithubVirtualFileSystem {
        let mut inodes = HashMap::new();
        let mut attrs = HashMap::new();
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
        attrs.insert(1, attr);
        inodes.insert("/".to_string(), 1);
        GithubVirtualFileSystem {
            repositoriesPerUser: HashMap::new(),
            inodes: inodes,
            attrs: attrs
        }
    }
    fn add(&mut self, username: &str) -> () {
        let args = ["repo", "list", username, "--json", "name", "--source", "--jq", ".[].name"];
        println!("args={:?}", args);
       let listOutput = Command::new("gh")
            .args(args)
            .output()
            .expect(format!("Error when running: gh {:?}", args.join(" ").as_str()).as_str());
        let stdout = String::from_utf8(listOutput.stdout).unwrap();
        let result: Vec<String> = stdout.split("\n").map(|s| s.to_string()).collect();
        let mut repositoriesPerUser = HashMap::new();
        repositoriesPerUser.insert(username.to_string(), result);
        self.repositoriesPerUser=repositoriesPerUser;
         let repos = self.repositoriesPerUser.get(username).unwrap();
            
        let mut index = self.inodes.len()as u64;
        for repoName in repos.iter() {
            if repoName.len() == 0 {continue};
            let newInode: u64 = index + 1;
            let key = username.to_string() + "/" + &repoName.to_string();
            let ts = Timespec::new(0, 0);
            let attr = FileAttr {
                ino: newInode,
                size: repoName.len() as u64,
                blocks: 0,
                atime: ts,
                mtime: ts,
                ctime: ts,
                crtime: ts,
                kind: FileType::Directory,
                perm: 0o644,
                nlink: 0,
                uid: 0,
                gid: 0,
                rdev: 0,
                flags: 0,
            };
            self.inodes.insert(key, attr.ino);
            self.attrs.insert(newInode, attr);
        }
    }
}

impl Filesystem for GithubVirtualFileSystem {
    fn init(&mut self, _req: &Request) -> Result<(), c_int>{
        println!("init(_req={:?})", _req);
        return Ok(())
    }
    fn destroy(&mut self, _req: &Request) {
        println!("destroy");
    }
    fn forget(&mut self, _req: &Request, _ino: u64, _nlookup: u64) {
        println!("forget(ino={}, _req={:?}, _nlookup={})", _ino, _req, _nlookup);
    }
    fn getattr(&mut self, _req: &Request, _ino: u64, reply: ReplyAttr) {
        println!("getattr(ino={}, _req={:?})", _ino, _req);
        let ts = Timespec::new(0, 0);
        let attr = FileAttr {
            ino: _ino,
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
        reply.attr(&ttl, &attr);
    }
    fn setattr(&mut self,
        _req: &Request,
        _ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<Timespec>,
        _mtime: Option<Timespec>,
        _fh: Option<u64>,
        _crtime: Option<Timespec>,
        _chgtime: Option<Timespec>,
        _bkuptime: Option<Timespec>,
        _flags: Option<u32>,
         reply: ReplyAttr
    ) {
        println!("setattr(_ino={},_mode={:?},_uid={:?},_gid={:?},_size={:?},_atime={:?},_mtime={:?})", 
            _ino,
            _mode,
            _uid,
            _gid,
            _size,
            _atime,
            _mtime
        );
        let ts = Timespec::new(0, 0);
        let attr = FileAttr {
            ino: _ino,
            size: 0,
            blocks: 0,
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        let ttl = Timespec::new(1, 0);
        reply.attr(&ttl, &attr);
    }
    fn readlink(&mut self, _req: &Request, _ino: u64, reply: ReplyData) {
        println!("readlink");
        reply.error(ENOSYS)
    }
    fn open(&mut self, _req: &Request, _ino: u64, _flags: u32, reply: ReplyOpen) {
        println!("open(_ino={}, _flags={})", _ino, _flags);
        reply.opened(_ino, _flags);
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
        println!("flush(_ino={}, _fh={}, _lock_owner={})", _ino, _fh,_lock_owner);
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
        let ts = time::now().to_timespec();
        
        println!("lookup(parent={}, name={})", parent, name.to_str().unwrap());
        let (repoName, inode) =  match self.inodes
            .iter()
            .find(|(repo, index)| repo.starts_with(&name.to_str().unwrap().to_string())) {
            Some(inode) => inode,
            None => {
                if parent == 1 {
                    println!("not found, fetching {}",name.to_str().unwrap());
                    self.add(name.to_str().unwrap());
                    reply.error(ENOENT);
                    return;
                };
                if parent == 2 {
                    println!("repo!");
                    reply.error(EEXIST);
                    return;
                };
                return;
            },
        };
        
        match self.attrs.get(inode) {
            Some(attr) => {
                let ttl = Timespec::new(1, 0);
                println!("attr found! inode: {}",inode);
                reply.entry(&ttl, attr, 0);
            },
            None => reply.error(ENOENT),
        };
 
    }
    fn opendir(&mut self,_req: &Request,_ino: u64,_flags: u32, reply: ReplyOpen) {
        println!("opendir(ino={}, _flags={})", _ino, _flags);
      
        reply.opened(_ino, _flags);
    }
    fn readdir(&mut self,_req: &Request,_ino: u64,_fh: u64,_offset: i64, mut reply: ReplyDirectory) {
        println!("readdir(ino={}, _fh={}, _offset={})", _ino, _fh, _offset);
        if _offset == 0 {
            reply.add(1, 0, FileType::Directory, &Path::new("."));
            reply.add(1, 1, FileType::Directory, &Path::new(".."));
            let mut index = 2;
            for (repositoryName, inode) in self.inodes.iter() {
                let userAndRepo: Vec<&str> = repositoryName.split("/").collect();
                let user = userAndRepo[0];
                let repo = userAndRepo[1];
                reply.add(1, index, FileType::Directory, &Path::new(repo));
                index += 1;
            }
        }
        reply.ok();
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
        println!("access(ino={}, _mask={})", _ino, _mask);
        reply.ok()
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
    let fs = GithubVirtualFileSystem::new();

    fuse::mount(fs, &mountpoint, &[]);
}
