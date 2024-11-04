use fuser::mount;
use std::env;
use std::path::Path;

mod filesystem;
use filesystem::FatFilesystem;

fn main() {
    // Collect and parse CLI arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprint!("Usage: {} <disk.img> <mount_point>", args[0]);
        return;
    }

    let disk_image_path = Path::new(&args[1]);
    let mount_point = Path::new(&args[2]);

    if let Err(e) = mount(FatFilesystem::new(disk_image_path), &mount_point, &[]) {
        eprintln!("Failed to mount filesystem: {}", e);
    }
}
