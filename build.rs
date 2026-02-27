use std::fs;
use std::path::Path;

fn main() {
    let src_dir = Path::new("jswasm");
    let dest_dir = Path::new("pkg/jswasm");

    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir).unwrap();
    }

    for entry in fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dest_path = dest_dir.join(entry.file_name());
        fs::copy(src_path, dest_path).unwrap();
    }

    println!("cargo:rerun-if-changed=jswasm/");
}
