//! This module implements the FUSE-API to access the FAT filesystem provided by the `fatfs` crate.
use fatfs::{Dir, FileSystem as FatfsFileSystem, FsOptions};
use fuser::{
    FileAttr, FileType, Filesystem as FuseFilesystem, ReplyAttr, ReplyData, ReplyDirectory,
    ReplyEntry, Request,
};
use libc::{EIO, ENOENT};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Represent FAT-Filesystem
///
/// # Members
///
/// * `fs: Arc<Mutex<fatfs::Filesystem<File>>>`
/// * `inode_map: Mutex<HashMap<u64, PathBuf>>` - The map of all child nodes.
/// * `nnode: Mutex<u64>` - The ID of the next inode.
pub struct FatFilesystem {
    fs: Arc<Mutex<FatfsFileSystem<File>>>,
    inode_map: Mutex<HashMap<u64, PathBuf>>,
    nnode: Mutex<u64>,
}

impl FatFilesystem {
    /// Create a new instance of a FAT-Filesystem.
    ///
    /// # Parameters
    ///
    /// * `disk_image_path: &Path` - The path of the disk image to use.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of a `FatFilesystem`.
    pub fn new(disk_image_path: &Path) -> Self {
        let img_file = File::open(disk_image_path).expect("Failed to open disk image.");
        let fs = FatfsFileSystem::new(img_file, FsOptions::new())
            .expect("Failed to create new FileSystem.");

        let mut inode_map = HashMap::new();
        inode_map.insert(1, PathBuf::from("/"));

        FatFilesystem {
            fs: Arc::new(Mutex::new(fs)),
            inode_map: Mutex::new(inode_map),
            nnode: Mutex::new(2),
        }
    }

    /// Helper function to get or create an inode for a given path.
    /// Needed as `fatfs` does not support inode natively.
    ///
    /// # Parameters
    ///
    /// * `path: &Path` - The Path to return the inode of or create one for.
    ///
    /// # Returns
    ///
    /// * `u64` - The id of the node.
    fn get_or_create_inode(&self, path: &Path) -> u64 {
        let mut inode_map = self.inode_map.lock().unwrap();
        let mut nnode = self.nnode.lock().unwrap();

        // If given path has an Inode, return it.
        if let Some(&ino) = inode_map
            .iter()
            .find_map(|(ino, p)| if p == path { Some(ino) } else { None })
        {
            ino
        } else {
            // Create a new Inode and add them to the mapping.
            let ino = *nnode;
            inode_map.insert(ino, path.to_path_buf());
            *nnode += 1;
            ino
        }
    }

    /// Helper function to format the filesize correctly.
    ///
    /// # Parameters
    ///
    /// * `size: u64` - The size of the file.
    ///
    /// # Returns
    ///
    /// A format string including the size calculated into the correct unit.
    fn format_file_size(size: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;
        if size < KB {
            format!("{}B", size)
        } else if size < MB {
            format!("{}KB", size / KB)
        } else if size < GB {
            format!("{}MB", size / MB)
        } else {
            format!("{}GB", size / GB)
        }
    }
}

impl FuseFilesystem for FatFilesystem {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        println!(
            "lookup args: req:{:?}, parent: {:?}, name:{:?}",
            _req, parent, name
        );
        // Attribute time to live
        let ttl = Duration::from_secs(1);

        // Get path for given inode.
        let mut path = {
            let inode_map = self.inode_map.lock().unwrap();
            inode_map.get(&parent).cloned()
        };

        let mut path = match path {
            Some(path) => path,
            None => {
                reply.error(EIO);
                return;
            }
        };

        path.push(name);
        println!("Parent + path = {:?}", path);

        let fs = self.fs.lock().unwrap();

        let ino = self.get_or_create_inode(&path);

        match fs.root_dir().open_file(path.to_str().unwrap()) {
            Ok(file) => {
                let size = file.bytes().count() as u64;
                let now = SystemTime::now();
                let file_attr = FileAttr {
                    ino,
                    size,
                    blocks: ((size + 511) / 512),
                    atime: now,
                    mtime: now,
                    ctime: now,
                    crtime: now,
                    kind: FileType::RegularFile,
                    perm: 0o644,
                    nlink: 1,
                    uid: 501,
                    gid: 20,
                    rdev: 0,
                    flags: 0,
                    blksize: 4096,
                };
                reply.entry(&ttl, &file_attr, 0);
                return;
            }
            Err(_) => {}
        };
        // HACK: Do a check whether it is a dir or not. May require stat to be implemented.
        match fs.root_dir().open_dir(path.to_str().unwrap()) {
            Ok(dir) => {
                // Beispiel für eine Datei
                let size = 1 as u64;
                let now = SystemTime::now();
                let file_attr = FileAttr {
                    ino,
                    size,
                    blocks: ((size + 511) / 512),
                    atime: now,
                    mtime: now,
                    ctime: now,
                    crtime: now,
                    kind: FileType::Directory,
                    perm: 0o755,
                    nlink: 1,
                    uid: 501,
                    gid: 20,
                    rdev: 0,
                    flags: 0,
                    blksize: 4096,
                };
                reply.entry(&ttl, &file_attr, 0);
            }
            Err(_) => {
                reply.error(libc::ENOENT);
            }
        };
    }

    /// Get the attributes of a file or directory.
    ///
    /// # Parameters
    ///
    /// * `_req: &Request` - The `fuser::Request` datastructure representing the request to the filesystem.
    /// * `ino: u64` - The inode-number of the given filesystem object.
    /// * `_fh: Option<u64>` - The File-Handle (optional).
    /// * `reply: ReplyAttr` - A `fuser::ReplyAttr` instance for returning attributes.
    ///
    /// # Returns
    ///
    /// This function does not return a value. It responds to the request with a reply or an error
    /// code if the requested inode does not exist.
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        println!(
            "getattr args: req:{:?}, ino:{:?}, _fh:{:?}, reply:{:?}",
            _req, ino, _fh, reply
        );
        // Attribute time to live
        let ttl = Duration::from_secs(1);

        // Get path for given inode.
        let path = {
            let inode_map = self.inode_map.lock().unwrap();
            inode_map.get(&ino).cloned()
        };

        let path = match path {
            Some(path) => path,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        if ino == 1 {
            let now = SystemTime::now();
            reply.attr(
                &ttl,
                &FileAttr {
                    ino: 1,
                    size: 0,
                    blocks: 0,
                    atime: now,
                    mtime: now,
                    ctime: now,
                    crtime: now,
                    kind: FileType::Directory,
                    perm: 0o755,
                    nlink: 2,
                    uid: 501,
                    gid: 20,
                    rdev: 0,
                    flags: 0,
                    blksize: 4096,
                },
            );
        } else {
            let fs = self.fs.lock().unwrap();
            match fs.root_dir().open_file(path.to_str().unwrap()) {
                Ok(file) => {
                    let size = file.bytes().count() as u64;
                    let now = SystemTime::now();
                    let file_attr = FileAttr {
                        ino,
                        size,
                        blocks: ((size + 511) / 512),
                        atime: now,
                        mtime: now,
                        ctime: now,
                        crtime: now,
                        kind: FileType::RegularFile,
                        perm: 0o644,
                        nlink: 1,
                        uid: 501,
                        gid: 20,
                        rdev: 0,
                        flags: 0,
                        blksize: 4096,
                    };
                    reply.attr(&ttl, &file_attr);
                    return;
                }
                Err(_) => {}
            };
            // HACK: Do a check whether it is a dir or not. May require stat to be implemented.
            match fs.root_dir().open_dir(path.to_str().unwrap()) {
                Ok(dir) => {
                    // Beispiel für eine Datei
                    let size = 1 as u64;
                    let now = SystemTime::now();
                    let file_attr = FileAttr {
                        ino,
                        size,
                        blocks: ((size + 511) / 512),
                        atime: now,
                        mtime: now,
                        ctime: now,
                        crtime: now,
                        kind: FileType::Directory,
                        perm: 0o755,
                        nlink: 1,
                        uid: 501,
                        gid: 20,
                        rdev: 0,
                        flags: 0,
                        blksize: 4096,
                    };
                    reply.attr(&ttl, &file_attr);
                }
                Err(_) => {
                    reply.error(libc::ENOENT);
                }
            };
        }
    }

    /// Read the contents of a directory.
    ///
    /// # Parameters
    ///
    /// * `_req: &Request` - The `fuser::Request` datastructure representing the request to the filesystem.
    /// * `ino: u64` - The inode number of the requested file or directory.
    /// * `_fh: u64` - The file handle, if given.
    /// * `offset: i64` - The offset of the entries in Bytes from Reply.
    /// * `reply: ReplyDirectory` - A `fuser::ReplyDirectory` instance for returning directory contents.
    ///
    /// # Returns
    ///
    /// This function does not return a value. It responds to the request with directory entries or an error code.
    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        println!("Reading dir...");
        println!(
            "Readdir args: _req:{:?}, ino:{:?}, _fh:{:?}, _offset:{:?}, reply:{:?}",
            _req, ino, _fh, offset, reply
        );
        let fs = self.fs.lock().unwrap();

        let path = {
            let inode_map = self.inode_map.lock().unwrap();
            match inode_map.get(&ino).cloned() {
                Some(path) => path,
                None => {
                    eprintln!(
                        "Unable to get inode when initializing path!\nMap: {:?}",
                        inode_map
                    );
                    reply.error(ENOENT);
                    return;
                }
            }
        };
        let dir: Dir<'_, File>;
        // Open dir and read entries.
        if path == PathBuf::from("/") {
            println!("Root directory detected:");
            dir = fs.root_dir();
        } else {
            dir = match fs.root_dir().open_dir(path.to_str().unwrap()) {
                Ok(dir) => dir,
                Err(_) => {
                    eprintln!(
                        "Unable to open given dir! Path: {:?}",
                        path.to_str().unwrap()
                    );
                    reply.error(ENOENT);
                    return;
                }
            };
        }

        // Iterate over all entries in the directory.
        for (index, entry) in dir.iter().skip(offset as usize).enumerate() {
            println!("Entry: {:?}", entry);
            let e = entry.unwrap();
            let file_name = e.file_name();
            let kind = if e.is_dir() {
                FileType::Directory
            } else {
                FileType::RegularFile
            };

            // Create an inode for every file
            let entry_path = path.join(file_name.as_str());
            let entry_inode = self.get_or_create_inode(&entry_path);

            let buffer_full: bool = reply.add(
                entry_inode,
                offset + index as i64 + 1,
                kind,
                file_name.as_str(),
            );

            if buffer_full {
                break;
            }
        }
        println!("reply: {:?}", reply);
        reply.ok();
    }

    /// Read data from a file.
    ///
    /// # Parameters
    ///
    /// * `_req: &Request` - The `fuse::Request` datastructure representing the request to the
    ///   filesystem.
    /// * `ino: u64` - The inode number of the file to read.
    /// * `_fh: u64` - File handle (not used in this implementation).
    /// * `offset: i64` - Offset in the file where reading starts.
    /// * `size: u32` - Number of bytes to read.
    /// * `_flags: i32` - Additional flags. (Not used in this implementation)
    /// * `_lock_owner: Option<u64>` - (Not used in this implementation)
    /// * `reply: ReplyData` - A `fuse::ReplyData` instance for returning file data.
    ///
    /// # Returns
    ///
    /// This function does not return a value. It responds to the request with a Reply or an error
    /// code if the requested inode does not exist.
    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        // Get path for given inode.
        let path = self.inode_map.lock().unwrap().get(&ino).cloned().unwrap();
        let fs = self.fs.lock().unwrap();
        match fs.root_dir().open_file(path.to_str().unwrap()) {
            Ok(file) => {
                let file_bytes = file
                    .bytes()
                    .skip(offset as usize)
                    .take(size as usize)
                    .map(|c| c.expect("Why no data?") as u8)
                    .collect::<Vec<u8>>();
                reply.data(&file_bytes);
            }
            Err(_) => {
                reply.error(libc::ENOENT);
            }
        };
    }

    // TODO: write, mkdir, etc.
}
