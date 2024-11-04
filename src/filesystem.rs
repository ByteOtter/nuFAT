//! This module implements the FUSE-API to access the FAT filesystem provided by the `fatfs` crate.
use fatfs::{FileSystem as FatfsFileSystem, FsOptions};
use fuser::{FileAttr, FileType, Filesystem as FuseFilesystem, ReplyAttr, ReplyData, ReplyDirectory, Request};
use libc::ENOENT;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Represent FAT-Filesystem
///
/// # Members
///
/// * `fs: Arc<Mutex<fatfs::Filesystem<File>>>`
pub struct FatFilesystem {
    fs: Arc<Mutex<FatfsFileSystem<File>>>,
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
        FatFilesystem {
            fs: Arc::new(Mutex::new(fs)),
        }
    }
}

impl FuseFilesystem for FatFilesystem {
    /// Get the attributes of a file or directory.
    ///
    /// # Parameters
    ///
    /// * `_req: &Request` - The `fuser::Request` datastructure representing the request to the filesystem.
    /// * `ino: u64` - The inode number of the requested file or directory.
    /// * `reply: ReplyAttr` - A `fuser::ReplyAttr` instance for returning attributes.
    ///
    /// # Returns
    ///
    /// This function does not return a value. It responds to the request with a reply or an error
    /// code if the requested inode does not exist.
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        // Attribute time to live
        let ttl = Duration::from_secs(1);

        if ino == 1 {
            // If the current directory is root, return these attributes
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
            // If the file does not exist, return ENOENT
            reply.error(ENOENT);
        }
    }

    /// Read the contents of a directory.
    ///
    /// # Parameters
    ///
    /// * `_req: &Request` - The `fuser::Request` datastructure representing the request to the filesystem.
    /// * `ino: u64` - The inode number of the requested directory.
    /// * `_fh: u64` - File handle (not used in this example).
    /// * `_offset: i64` - Offset for reading (not used in this example).
    /// * `reply: ReplyDirectory` - A `fuser::ReplyDirectory` instance for returning directory contents.
    ///
    /// # Returns
    ///
    /// This function does not return a value. It responds to the request with directory entries or an error code.
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, _offset: i64, mut reply: ReplyDirectory) {
        if ino != 1 {
            // If not root-Directory, return ENOENT
            reply.error(ENOENT);
            return;
        }

        // Lock access to filesystem
        let fs = self.fs.lock().unwrap();
        let root_dir = fs.root_dir();

        let mut entry_index: u64 = 2; // Start inode numbering from 2 (1 is reserved for the root)

        for entry in root_dir.iter().filter_map(Result::ok) {
            let name = entry.file_name(); // Get the file name
                                          // Use a simple incrementing number as inode
            reply.add(entry_index, 0, fuser::FileType::RegularFile, &name); // Add the entry to the reply
            entry_index += 1; // Increment the index for the next entry
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
