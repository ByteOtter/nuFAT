use fuser::mount2;
use std::env;
use std::path::Path;
use std::process;

mod filesystem;
use filesystem::FatFilesystem;

fn main() {
    // Collect and parse CLI arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprint!("Usage: {} <disk.img> <mount_point>", args[0]);
        process::exit(1);
    }

    let disk_image_path = Path::new(&args[1]);
    let mount_point = Path::new(&args[2]);

    if let Err(e) = mount2(FatFilesystem::new(disk_image_path), mount_point, &[]) {
        eprintln!("Failed to mount filesystem: {}", e);
        process::exit(1);
    }
}
