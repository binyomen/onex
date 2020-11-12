use std::env;
use walkdir::WalkDir;

fn main() {
    print!("Args: ");
    for arg in env::args() {
        print!("\"{}\" ", arg);
    }
    println!("");

    println!("Directory contents:");
    let exe_path = env::current_exe().unwrap();
    let root_dir = exe_path.parent().unwrap();
    for entry in WalkDir::new(root_dir) {
        let entry = entry.unwrap();
        println!(
            "{} ({} bytes)",
            entry.path().display(),
            entry.metadata().unwrap().len()
        );
    }
}
