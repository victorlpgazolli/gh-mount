extern crate fuser;
extern crate libc;
extern crate time;
use fuser::{
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
use std::time::{Duration,SystemTime};
use std::option::Option;

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
        let ts = SystemTime::now();
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
            blksize: 0,
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
        let ignoreUsers = [".git"];
        if ignoreUsers.contains(&username) {
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
        let ts = SystemTime::now();
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
            blksize: 0,
        };
        self.inodes.insert(username.to_string(), userAttr.ino);
        self.attrs.insert(userInode, userAttr);
        for repoName in repos.iter() {
            if repoName.len() == 0 {
                continue;
            };
            let newInode: u64 = self.inodes.len() as u64 + 1;
            let key = self.formatRepositoryName(username, repoName);
            let ts = SystemTime::now();
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
                blksize: 0,
            };
            if !self.inodes.contains_key(&key) { self.inodes.insert(key, attr.ino); } ;
            if !self.attrs.contains_key(&newInode) { self.attrs.insert(newInode, attr); } ;
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
            let ts = SystemTime::now();
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
                blksize: 0,
            };
            if !self.inodes.contains_key(&key) { self.inodes.insert(key, attr.ino); } ;
            if !self.attrs.contains_key(&newInode) { self.attrs.insert(newInode, attr); } ;
            
        }
    }
}

impl Filesystem for GithubVirtualFileSystem {
    fn getattr(&mut self, _req: &Request, _ino: u64, reply: ReplyAttr) {
        println!("getattr(ino={})", _ino);
        let ts = SystemTime::now();
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
            blksize: 0,
        };
        let ttl = Duration::new(0,0);
        reply.attr(&ttl, &attr);
    }
    fn readlink(&mut self, _req: &Request, _ino: u64, reply: ReplyData) {
        println!("readlink(_ino={})", _ino);
        let (currentPathType, fullRepositoryName) = self.getCurrentPathType(_ino);
        let homeUser = match env::home_dir() {
            Some(path) => path.display().to_string(),
            None => ".".to_owned(),
        };
        let pathToPersist = homeUser + &"/.config/gh_mount/".to_owned() + &fullRepositoryName.to_owned();
        reply.data(pathToPersist.as_bytes());
    }
    fn open(&mut self, _req: &Request, _ino: u64, _flags: i32, reply: ReplyOpen) {
        println!("open(_ino={}, _flags={})", _ino, _flags);
        reply.opened(_ino, _flags as u32);
    }
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        println!("lookup(parent={}, name={})", parent, name.to_str().unwrap());

        let ts = time::now().to_timespec();
        let (currentPathType, fullRepositoryName) = self.getCurrentPathType(parent);
        let inode = match currentPathType {
            GithubVirtualFileSystemPath::UserPath => {
                let mut desiredInode = 0;
                let repositories = self.getRepositoriesFromUser(fullRepositoryName);
                for (repositoryName, inode) in repositories.iter() {
                    let fullpathSplitted = GithubVirtualFileSystem::parseRepositoryName(&repositoryName);
                    let isSameRepo = fullpathSplitted[1].eq(&name.to_str().unwrap().to_string());
                    if !isSameRepo {  continue;   };
                    desiredInode = *inode;
                    break;
                }
                desiredInode
            },
            GithubVirtualFileSystemPath::RepositoryPath => {
                let exampleInode = 0;
                exampleInode
            },
            GithubVirtualFileSystemPath::FilePath => {
                let exampleInode = 0;
                exampleInode
            },
            GithubVirtualFileSystemPath::None => {
                let repositories = self.getRepositoriesFromUser(fullRepositoryName);
                self.addUser(name.to_str().unwrap());
                let inodesPerTypes = self.getInodesPerType();
                let mut desiredInode = 0;
                for (user, inode) in inodesPerTypes.usersInodes.iter() {
                    let isSameUser = name.to_str().unwrap().to_string().eq(&user.to_owned());
                    if !isSameUser {  continue;   };
                    desiredInode = *inode;
                    break;
                }
                desiredInode
            },
        };
       
        match self.attrs.get(&inode) {
            Some(attr) => {
                let (currentPathType, fullRepositoryName) = self.getCurrentPathType(inode);
                let ttl = Duration::new(0,0);
                let homeUser = match env::home_dir() {
                    Some(path) => path.display().to_string(),
                    None => ".".to_owned(),
                };
                let pathToPersist = homeUser + &"/.config/gh_mount/".to_owned() + &fullRepositoryName.to_owned();
                let hasToBeASymlink = match currentPathType {
                    GithubVirtualFileSystemPath::UserPath => false,
                    GithubVirtualFileSystemPath::RepositoryPath => {
                        let pathAlreadyExists = Path::new(&pathToPersist).exists();
                        pathAlreadyExists
                    },
                    GithubVirtualFileSystemPath::FilePath => false,
                    GithubVirtualFileSystemPath::None => false,
                };
                if !hasToBeASymlink {
                    reply.entry(&ttl, attr, 0);
                    return;
                }
                let mut newAttr = attr.clone();
                newAttr.kind = FileType::Symlink;
           
                reply.entry(&ttl, &newAttr, 0);
            }
            None => reply.error(ENOENT),
        };
    }
    fn opendir(&mut self, _req: &Request, _ino: u64, _flags: i32, reply: ReplyOpen) {
        println!("opendir(ino={}, _flags={})", _ino, _flags);

        reply.opened(_ino, _flags as u32);
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
        println!("{:?}", inodesPerTypes.repositoriesInodes);
        let (currentPathType, fullRepositoryName) = self.getCurrentPathType(_ino);

        if _offset == 0 {
            reply.add(_ino, 0, FileType::Directory, &Path::new("."));
            reply.add(_ino, 1, FileType::Directory, &Path::new(".."));
        }
        match currentPathType {
            GithubVirtualFileSystemPath::UserPath => {
                let repositories = self.getRepositoriesFromUser(fullRepositoryName);
                for (repositoryName, inode) in repositories.iter() {
                    let fullpathSplitted = GithubVirtualFileSystem::parseRepositoryName(&repositoryName);
                    if _offset == 0 {
                        reply.add(*inode, (*inode) as i64, FileType::Directory, &Path::new(fullpathSplitted[1]));
                    }
                }
            },
            GithubVirtualFileSystemPath::RepositoryPath => {
                let files = self.getFilesFromRepo(fullRepositoryName);
                for (filename, inode) in files.iter() {
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
    fn access(&mut self, _req: &Request, _ino: u64, _mask: i32, reply: ReplyEmpty) {
        println!("access(ino={}, _mask={})", _ino, _mask);
        let (currentPathType, fullRepositoryName) = self.getCurrentPathType(_ino);
        match currentPathType {
            GithubVirtualFileSystemPath::UserPath => {
      
            },
            GithubVirtualFileSystemPath::RepositoryPath => {
                let homeUser = match env::home_dir() {
                    Some(path) => path.display().to_string(),
                    None => ".".to_owned(),
                };
                let pathToPersist = homeUser + &"/.config/gh_mount/".to_owned() + &fullRepositoryName.to_owned();
                let createPersistPathArgs = [
                    "-p", &pathToPersist
                ];
                Command::new("mkdir")
                    .args(createPersistPathArgs)
                    .output()
                    .expect(format!("Error when running: mkdir {:?}", createPersistPathArgs.join(" ").as_str()).as_str());

                let args = [
                    "repo", "clone", fullRepositoryName, "--", &pathToPersist
                ];
                println!("args={:?}", args);
                let listOutput = Command::new("gh")
                    .args(args)
                    .output()
                    .expect(format!("Error when running: gh {:?}", args.join(" ").as_str()).as_str());
                let stdout = String::from_utf8(listOutput.stdout).unwrap();
                let mut pathAttr: FileAttr = *self.attrs.get(&_ino).unwrap();
                pathAttr.kind = FileType::Symlink;
                self.attrs.insert(_ino, pathAttr);
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

    fuser::mount2(fs, &mountpoint, &[]);
}
