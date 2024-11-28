use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEntry, ReplyWrite, Request
};
use libc;
use std::{
    ffi::OsStr,
    hash::{Hash, Hasher},
    process::Command,
    time::{Duration, SystemTime},
};

pub struct Fat32Fs {
    disk_img: String, // Path to the FAT32 disk image file
}

impl Fat32Fs {
    pub fn new(disk_img: String) -> Self {
        Self { disk_img }
    }

    /// Run an `mtools` command with the `-i` flag for `disk.img`
    fn run_mtools_command(&self, args: &[&str]) -> Result<String, String> {
        let mut full_args = vec!["-i", &self.disk_img];
        full_args.extend_from_slice(args);

        let output = Command::new("mtools").args(full_args).output();
        match output {
            Ok(output) => {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).to_string())
                }
            }
            Err(err) => Err(err.to_string()),
        }
    }

    fn calculate_ino(&self, name: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish()
    }
}

impl Filesystem for Fat32Fs {
    fn readdir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if offset != 0 {
            reply.ok();
            return;
        }

        // Use `mdir` to list files in the root directory
        let output = self.run_mtools_command(&["mdir", "::"]);
        match output {
            Ok(contents) => {
                let mut entries = vec![
                    (1, ".", FileType::Directory),  // Current directory
                    (2, "..", FileType::Directory), // Parent directory
                ];

                // Parse the `mdir` output to extract file/directory names
                for line in contents.lines() {
                    if let Some(name) = line.split_whitespace().last() {
                        let file_type = if line.contains("<DIR>") {
                            FileType::Directory
                        } else {
                            FileType::RegularFile
                        };
                        // Add the file/directory entry to the list
                        entries.push((entries.len() as u64 + 1, name, file_type));
                    }
                }

                // Iterate over entries and add them to the reply
                for (i, (ino, name, file_type)) in entries.iter().enumerate().skip(offset as usize)
                {
                    reply.add(
                        *ino,           // File inode number
                        (i as i64) + 1, // Next offset
                        *file_type,     // File type
                        name,           // File name
                    );
                }
                reply.ok();
            }
            Err(err) => {
                eprintln!("Error in readdir: {}", err);
                reply.error(libc::EIO);
            }
        }
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let temp_file = "/tmp/read_temp";

        // Copy the file from the FAT32 image to a temporary location using `mcopy`
        let output = self.run_mtools_command(&["mcopy", "::somefile", temp_file]);
        if let Err(err) = output {
            eprintln!("Error in mcopy for read: {}", err);
            reply.error(libc::EIO);
            return;
        }

        // Read the temporary file to extract the requested range
        match std::fs::read(temp_file) {
            Ok(contents) => {
                let start = offset as usize;
                let end = std::cmp::min(start + size as usize, contents.len());
                reply.data(&contents[start..end]);
            }
            Err(err) => {
                eprintln!("Error reading temporary file: {}", err);
                reply.error(libc::EIO);
            }
        }
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        let temp_file = "/tmp/write_temp";

        // Write data to a temporary file
        match std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(temp_file)
        {
            Ok(mut file) => {
                use std::io::Write;
                if let Err(err) = file.write_all(data) {
                    eprintln!("Error writing temporary file: {}", err);
                    reply.error(libc::EIO);
                    return;
                }
            }
            Err(err) => {
                eprintln!("Error creating temporary file: {}", err);
                reply.error(libc::EIO);
                return;
            }
        }

        // Copy the temporary file to the FAT32 image using `mcopy`
        let output = self.run_mtools_command(&["mcopy", temp_file, "::somefile"]);
        if let Err(err) = output {
            eprintln!("Error in mcopy for write: {}", err);
            reply.error(libc::EIO);
            return;
        }

        reply.written(data.len() as u32);
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let ttl = Duration::new(1, 0); // 1-second attribute cache timeout

        // Handle root directory
        if ino == 1 {
            let attr = FileAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                blksize: 4096,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                flags: 0,
            };
            reply.attr(&ttl, &attr);
            return;
        }

        // Retrieve file attributes using `mdir` for the given inode
        let output = self.run_mtools_command(&["mdir", "::"]);
        if let Ok(contents) = output {
            for line in contents.lines() {
                if let Some(name) = line.split_whitespace().last() {
                    if self.calculate_ino(name) == ino {
                        let file_type = if line.contains("<DIR>") {
                            FileType::Directory
                        } else {
                            FileType::RegularFile
                        };

                        let attr = FileAttr {
                            ino,
                            size: 0, // Placeholder; could be parsed from `mdir`
                            blocks: 0,
                            blksize: 4096,
                            atime: SystemTime::now(),
                            mtime: SystemTime::now(),
                            ctime: SystemTime::now(),
                            crtime: SystemTime::now(),
                            kind: file_type,
                            perm: 0o644,
                            nlink: 1,
                            uid: 1000,
                            gid: 1000,
                            rdev: 0,
                            flags: 0,
                        };
                        reply.attr(&ttl, &attr);
                        return;
                    }
                }
            }
        }
        reply.error(libc::ENOENT); // File not found
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        if parent != 1 {
            reply.error(libc::ENOENT); // Only allow creating directories in the root
            return;
        }

        let dir_name = name.to_str().unwrap_or("");
        let output = self.run_mtools_command(&["mmd", &format!("::{}", dir_name)]);
        if output.is_ok() {
            let ino = self.calculate_ino(dir_name);
            let attr = FileAttr {
                ino,
                size: 0,
                blocks: 0,
				blksize: 4096,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                flags: 0,
            };
            reply.entry(&Duration::new(1, 0), &attr, 0);
        } else {
            reply.error(libc::EIO); // Failed to create directory
        }
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
		_umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        if parent != 1 {
            reply.error(libc::ENOENT); // Only allow creating files in the root
            return;
        }

        let file_name = name.to_str().unwrap_or("");
        let temp_file = "/tmp/fuse_temp_create";

        // Create a temporary file
        if let Err(err) = std::fs::File::create(temp_file) {
            eprintln!("Error creating temporary file: {}", err);
            reply.error(libc::EIO);
            return;
        }

        // Copy the empty file to the FAT32 filesystem
        let output = self.run_mtools_command(&["mcopy", temp_file, &format!("::{}", file_name)]);
        if output.is_ok() {
            let ino = self.calculate_ino(file_name);
            let attr = FileAttr {
                ino,
                size: 0,
                blocks: 0,
				blksize: 4096,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
                kind: FileType::RegularFile,
                perm: 0o644,
                nlink: 1,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                flags: 0,
            };
            reply.created(&Duration::new(1, 0), &attr, 0, 0, 0);
        } else {
            reply.error(libc::EIO); // Failed to create file
        }
    }
}
