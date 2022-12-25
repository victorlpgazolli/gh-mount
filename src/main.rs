extern crate fuse;
extern crate libc;
extern crate time;
use fuse::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyOpen, ReplyStatfs, ReplyWrite, Request,
};
use libc::c_int;
use libc::{EEXIST, ENOENT, ENOSYS, EPERM};
use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use time::Timespec;
use std::option::Option;

extern crate serde_json;

enum GithubVirtualFileSystemPath {
    RepositoryPath,
    UserPath,
    FilePath,
    None
}
impl GithubVirtualFileSystemPath {
    fn as_str(&self) -> &'static str {
        match self {
            GithubVirtualFileSystemPath::RepositoryPath => "repo",
            GithubVirtualFileSystemPath::UserPath => "user",
            GithubVirtualFileSystemPath::FilePath => "file",
            GithubVirtualFileSystemPath::None => "none",
        }
    }
}
struct InodesTypes {
    usersInodes: HashMap<String, u64>,
    repositoriesInodes: HashMap<String, u64>,
    filesInodes: HashMap<String, u64>,
}
struct GithubVirtualFileSystem {
    repositoriesPerUser: HashMap<String, Vec<String>>,
    inodes: HashMap<String, u64>,
    attrs: HashMap<u64, FileAttr>,
}

impl GithubVirtualFileSystem {
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
            attrs: attrs,
        }
    }
    fn getTypeFromPath(fullRepositoryName: &str) -> GithubVirtualFileSystemPath {
        let fullpathSplitted = GithubVirtualFileSystem::parseRepositoryName(&fullRepositoryName);
        let isRoot = fullRepositoryName == "/";
        if isRoot {
            return GithubVirtualFileSystemPath::None
        }
        let isUser = fullpathSplitted.len() == 1;
        if isUser {
            return GithubVirtualFileSystemPath::UserPath
        }
        let isRepo = fullpathSplitted.len() == 2;
        if isRepo {
            return GithubVirtualFileSystemPath::RepositoryPath
        }
        return GithubVirtualFileSystemPath::FilePath
    }
    fn getFilesFromRepo(&self, fullRepositoryName: &str) -> HashMap<String, u64> {
        let inodes = self.getInodesPerType();
        let mut filesFromRepository = HashMap::new();
        let repository = GithubVirtualFileSystem::parseRepositoryName(fullRepositoryName)[1];
        for (pathname, pathInode) in inodes.repositoriesInodes.iter() {
            let fullpathSplitted = GithubVirtualFileSystem::parseRepositoryName(&pathname);
            let fileName = match self.getCurrentPathType(*pathInode).0 {
                GithubVirtualFileSystemPath::UserPath =>  Option::None,
                GithubVirtualFileSystemPath::RepositoryPath => Option::None,
                GithubVirtualFileSystemPath::FilePath => Option::Some(fullpathSplitted),
                GithubVirtualFileSystemPath::None => Option::None,
            };
            match fileName {
                Some(fileName) => {
                    let currentRepository = fileName[1];
                    if currentRepository == repository  {
                        filesFromRepository.insert(pathname.as_str().to_string(), (*pathInode) as u64);
                    }
                },
                None => ()
            }
        }
        return filesFromRepository;
    }
    fn getRepositoriesFromUser(&self, usernameRaw: &str) -> HashMap<String, u64> {
        let hasToParseUserName = usernameRaw.contains("/");
        let username = match hasToParseUserName {
            false => usernameRaw,
            true => GithubVirtualFileSystem::parseRepositoryName(usernameRaw)[0]
        };
        let inodes = self.getInodesPerType();
        let mut repositoriesFromUser = HashMap::new();
        for (pathname, pathInode) in inodes.repositoriesInodes.iter() {
            let fullpathSplitted = GithubVirtualFileSystem::parseRepositoryName(&pathname);
            let repositoryName = match self.getCurrentPathType(*pathInode).0 {
                GithubVirtualFileSystemPath::UserPath =>  Option::None,
                GithubVirtualFileSystemPath::RepositoryPath => Option::Some(fullpathSplitted),
                GithubVirtualFileSystemPath::FilePath =>  Option::None,
                GithubVirtualFileSystemPath::None => Option::None,
            };
            match repositoryName {
                Some(repositoryName) => {
                    if pathname.starts_with(username)  {
                        repositoriesFromUser.insert(pathname.as_str().to_string(), (*pathInode) as u64);
                    }
                },
                None => ()
            }
        }
        return repositoriesFromUser;
    }
    fn getCurrentPathType(&self, inode: u64) -> (GithubVirtualFileSystemPath, &str) {
        let fullRepositoryName = GithubVirtualFileSystem::findRepositoryNamePerInode(&self.inodes, &inode);
        let pathtype = GithubVirtualFileSystem::getTypeFromPath(fullRepositoryName);
        return (pathtype, fullRepositoryName);
    }
    fn getInodesPerType(&self) -> InodesTypes {
        let mut usersInodes = HashMap::new();
        let mut repositoriesInodes = HashMap::new();
        let mut filesInodes = HashMap::new();
        for (pathname, pathInode) in self.inodes.iter() {
            let fullpathSplitted = GithubVirtualFileSystem::parseRepositoryName(&pathname);
            match self.getCurrentPathType(*pathInode).0 {
                GithubVirtualFileSystemPath::UserPath => usersInodes.insert(pathname.to_string(), pathInode.to_owned()),
                GithubVirtualFileSystemPath::RepositoryPath =>  repositoriesInodes.insert(pathname.to_string(), pathInode.to_owned()),
                GithubVirtualFileSystemPath::FilePath => filesInodes.insert(pathname.to_string(), pathInode.to_owned()),
                GithubVirtualFileSystemPath::None => Option::None,
            };
        }

        InodesTypes {
            usersInodes,
            repositoriesInodes,
            filesInodes
        }
    }

    fn formatRepositoryName(&self, username: &str, repositoryName: &str) -> String {
        let key = username.to_string() + "/" + &repositoryName.to_string();
        return key;
    }
    fn parseRepositoryName(fullRepositoryName: &str) -> Vec<&str> {
        return fullRepositoryName.split("/").collect();
    }
    fn findRepositoryNamePerInode<'a>(inodes: &'a HashMap<String, u64>, inode: &u64) -> &'a str {
        for (repoName, repoInode) in inodes.iter() {
            if repoInode == inode {
                return repoName;
            }
        }
        return "";
    }
    fn addUser(&mut self, username: &str) -> () {
        let args = [
            "repo", "list", username, "--json", "name", "--source", "--jq", ".[].name",
        ];
        if username == ".git" {
            return;
        }
        println!("args={:?}", args);
        let listOutput = Command::new("gh")
            .args(args)
            .output()
            .expect(format!("Error when running: gh {:?}", args.join(" ").as_str()).as_str());
        let stdout = String::from_utf8(listOutput.stdout).unwrap();
        let result: Vec<String> = stdout.split("\n").map(|s| s.to_string()).collect();
        let mut repositoriesPerUser = HashMap::new();
        repositoriesPerUser.insert(username.to_string(), result);
        self.repositoriesPerUser = repositoriesPerUser;
        let repos = self.repositoriesPerUser.get(username).unwrap();

        let mut index = self.inodes.len() as u64;
        let userInode: u64 = index + 1;
        let ts = Timespec::new(0, 0);
        let userAttr = FileAttr {
            ino: userInode,
            size: username.to_string().len() as u64,
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
        self.inodes.insert(username.to_string(), userAttr.ino);
        self.attrs.insert(userInode, userAttr);
        for repoName in repos.iter() {
            if repoName.len() == 0 {
                continue;
            };
            let newInode: u64 = self.inodes.len() as u64 + 1;
            let key = self.formatRepositoryName(username, repoName);
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
    fn addRepoFiles(&mut self, fullRepositoryName: &str) -> () {
        let userAndRepo: Vec<&str> = GithubVirtualFileSystem::parseRepositoryName(fullRepositoryName);
        let username = userAndRepo[0];
        let repoName =  userAndRepo[1];
        let args = [
            "api", &format!("repos/{}/{}/git/trees/HEAD", username, repoName), "--jq", ".tree[].path"
        ];
        println!("args={:?}", args);
        let listOutput = Command::new("gh")
            .args(args)
            .output()
            .expect(format!("Error when running: gh {:?}", args.join(" ").as_str()).as_str());
        let stdout = String::from_utf8(listOutput.stdout).unwrap();
        let result: Vec<String> = stdout.split("\n").map(|s| s.to_string()).collect();
        for filename in result {
            if repoName.len() == 0 {
                continue;
            };
            let newInode: u64 = self.inodes.len() as u64 + 1;
            let key = self.formatRepositoryName(username, repoName) + "/" +&filename;
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
    fn init(&mut self, _req: &Request) -> Result<(), c_int> {
        println!("init(_req={:?})", _req);
        return Ok(());
    }
    fn destroy(&mut self, _req: &Request) {
        println!("destroy");
    }
    fn forget(&mut self, _req: &Request, _ino: u64, _nlookup: u64) {
        println!(
            "forget(ino={}, _req={:?}, _nlookup={})",
            _ino, _req, _nlookup
        );
    }
    fn getattr(&mut self, _req: &Request, _ino: u64, reply: ReplyAttr) {
        println!("getattr(ino={})", _ino);
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
    fn setattr(
        &mut self,
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
        reply: ReplyAttr,
    ) {
        println!(
            "setattr(_ino={},_mode={:?},_uid={:?},_gid={:?},_size={:?},_atime={:?},_mtime={:?})",
            _ino, _mode, _uid, _gid, _size, _atime, _mtime
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
        println!("readlink(_ino={})", _ino);
        let path = "../..";
        reply.data(path.as_bytes());
    }
    fn open(&mut self, _req: &Request, _ino: u64, _flags: u32, reply: ReplyOpen) {
        println!("open(_ino={}, _flags={})", _ino, _flags);
        reply.opened(_ino, _flags);
    }
    fn read(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _size: u32,
        reply: ReplyData,
    ) {
        println!("read");
        reply.error(ENOSYS)
    }
    fn write(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        println!("write");
        reply.error(ENOSYS)
    }
    fn flush(&mut self, _req: &Request, _ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        println!(
            "flush(_ino={}, _fh={}, _lock_owner={})",
            _ino, _fh, _lock_owner
        );
        reply.error(ENOSYS)
    }
    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        println!("release");
        reply.error(ENOSYS)
    }
    fn fsync(&mut self, _req: &Request, _ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        println!("fsync");
        reply.error(ENOSYS)
    }
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        println!("lookup(parent={}, name={})", parent, name.to_str().unwrap());

        let ts = time::now().to_timespec();
        let inodesPerTypes = self.getInodesPerType();
        let (currentPathType, fullRepositoryName) = self.getCurrentPathType(parent);
        let inode = match currentPathType {
            GithubVirtualFileSystemPath::UserPath => {
                let repositories = self.getRepositoriesFromUser(fullRepositoryName);
                for (repositoryName, inode) in repositories.iter() {
                    let fullpathSplitted = GithubVirtualFileSystem::parseRepositoryName(&repositoryName);
                    let isSameRepo = fullpathSplitted[1].ends_with(&name.to_str().unwrap().to_string());
                    if !isSameRepo {  continue;   };
                    return inode;
                }
            },
            GithubVirtualFileSystemPath::RepositoryPath => {
                
            },
            GithubVirtualFileSystemPath::FilePath => {},
            GithubVirtualFileSystemPath::None => {
                for (user, inode) in inodesPerTypes.usersInodes.iter() {
                    let isSameUser = name.to_str().unwrap().to_string().eq(&user.to_owned());
                    if !isSameUser {  continue;   };
                    return inode;
                }
            },
        };
       

        match self.attrs.get(inode) {
            Some(attr) => {
                let ttl = Timespec::new(1, 0);
                // println!("attr found! inode: {}",inode);
                // if *inode > 1 {
                //     self.addRepoFiles(name.to_str().unwrap());
                //  }
                reply.entry(&ttl, attr, 0);
            }
            None => reply.error(ENOENT),
        };
    }
    fn opendir(&mut self, _req: &Request, _ino: u64, _flags: u32, reply: ReplyOpen) {
        println!("opendir(ino={}, _flags={})", _ino, _flags);

        reply.opened(_ino, _flags);
    }
    fn readdir(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        mut reply: ReplyDirectory,
    ) {
        println!("readdir(ino={}, _fh={}, _offset={})", _ino, _fh, _offset);
        let inodesPerTypes = self.getInodesPerType();
        let (currentPathType, fullRepositoryName) = self.getCurrentPathType(_ino);
        println!("{}",currentPathType.as_str());
        if _offset == 0 {
            reply.add(_ino, 0, FileType::Directory, &Path::new("."));
            reply.add(_ino, 1, FileType::Directory, &Path::new(".."));
        }
        match currentPathType {
            GithubVirtualFileSystemPath::UserPath => {
                let repositories = self.getRepositoriesFromUser(fullRepositoryName);
                for (repositoryName, inode) in repositories.iter() {
                    let fullpathSplitted = GithubVirtualFileSystem::parseRepositoryName(&repositoryName);
                    println!("inode ={} reply.add {}",inode, fullpathSplitted[1]);
                    if _offset == 0 {
                        reply.add(*inode, (*inode) as i64, FileType::Directory, &Path::new(fullpathSplitted[1]));
                    }
                }
            },
            GithubVirtualFileSystemPath::RepositoryPath => {
                let files = self.getFilesFromRepo(fullRepositoryName);
                for (filename, inode) in files.iter() {
                    // let fullpathSplitted = GithubVirtualFileSystem::parseRepositoryName(&filename);
                    println!("inode ={} reply.add {}",inode, filename);
                    if _offset == 0 {
                        reply.add(*inode, (*inode) as i64, FileType::Directory, &Path::new(filename));
                    }
                }
            },
            GithubVirtualFileSystemPath::FilePath => {},
            GithubVirtualFileSystemPath::None => {
                reply.error(EPERM);
                return;
            },
        };
        reply.ok();
    }
    fn releasedir(&mut self, _req: &Request, _ino: u64, _fh: u64, _flags: u32, reply: ReplyEmpty) {
        println!("releasedir(ino={}, _fh={}, _flags={})", _ino, _fh, _flags);
        reply.error(ENOSYS)
    }
    fn fsyncdir(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        println!("fsyncdir");
        reply.error(ENOSYS)
    }
    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        println!("statfs");
        reply.error(ENOSYS)
    }
    fn access(&mut self, _req: &Request, _ino: u64, _mask: u32, reply: ReplyEmpty) {
        println!("access(ino={}, _mask={})", _ino, _mask);
        let (currentPathType, fullRepositoryName) = self.getCurrentPathType(_ino);
        match currentPathType {
            GithubVirtualFileSystemPath::UserPath => {
      
            },
            GithubVirtualFileSystemPath::RepositoryPath => {
        
            },
            GithubVirtualFileSystemPath::FilePath => {},
            GithubVirtualFileSystemPath::None => {
       
            },
        };
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
