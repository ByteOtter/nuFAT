//! This module implements the FUSE-API to access the FAT filesystem provided by the `fatfs` crate.
use fatfs::{FileSystem as FatfsFileSystem, FsOptions};
use fuser::{
    FileAttr, FileType, Filesystem as FuseFilesystem, ReplyAttr, ReplyData, ReplyDirectory, Request,
};
use libc::ENOENT;
use std::collections::HashMap;
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
                    // Beispiel für eine Datei
                    let size = 0;
                    let now = SystemTime::now();
                    let file_attr = FileAttr {
                        ino,
                        size,
                        blocks: (size / 512) + 1,
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
    /// * `_offset: i64` - The offset of the entries.
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
        _offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let fs = self.fs.lock().unwrap();
        let inode_map = self.inode_map.lock().unwrap();

        // Get the path for the given inode, if it exists.
        let path = match inode_map.get(&ino) {
            Some(path) => path.clone(),
            None => {
                // Inode not found, return ENOENT.
                reply.error(ENOENT);
                return;
            }
        };

        // Open dir and read entries.
        let dir = match fs.root_dir().open_dir(path.to_str().unwrap()) {
            Ok(dir) => dir,
            Err(_) => {
                reply.error(ENOENT);
                return;
            }
        };

        // Iterate over all entries in the directory.
        for entry in dir.iter().flatten() {
            let file_name = entry.file_name();
            let kind = if entry.is_dir() {
                FileType::Directory
            } else {
                FileType::RegularFile
            };

            // Create an inode for every file
            let entry_path = path.join(file_name.as_str());
            let entry_inode = self.get_or_create_inode(&entry_path);

            let _ = reply.add(entry_inode, 0, kind, file_name.as_str());
        }

        reply.ok();
    }

    /// Read data from a file (not yet implemented).
    ///
    /// # Parameters
    ///
    /// * `_req: &Request` - The `fuse::Request` datastructure representing the request to the
    ///   filesystem.
    /// * `ino: u64` - The inode number of the file to read.
    /// * `_fh: u64` - File handle (not used in this implementation).
    /// * `offset: i64` - Offset in the file where reading starts.
    /// * `size: u32` - Number of bytes to read.
    /// * `flags: i32`
    /// * `lock_owner: Option<u64>`
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
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        todo!("File read not yet implemented!")
    }

    // TODO: write, mkdir, etc.
}
