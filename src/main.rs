use fuser::mount2;
use std::env;
use std::path::Path;

mod filesystem;
use filesystem::Fat32Fs;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <disk.img> <mount_point>", args[0]);
        return;
    }

    let disk_img = args[1].clone(); // Path to `disk.img`
    let mount_point = args[2].clone();

    let fs = Fat32Fs::new(disk_img);

    // Use MountOption for mount options
    let options = vec![
        fuser::MountOption::FSName("fuser_mtools_test".to_string()), // Filesystem name
        fuser::MountOption::RW,                                      // Read-write mount
    ];

    // Mount the filesystem
    fuser::mount2(fs, mount_point, &options).expect("Failed to mount FUSE filesystem");
}
