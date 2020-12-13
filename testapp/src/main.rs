use {
    std::{env, error, fmt, fs::File, io::Read},
    walkdir::WalkDir,
};

fn main() -> Result<(), Box<dyn error::Error>> {
    let arg_string = env::args()
        .map(|a| format!("\"{}\"", a))
        .collect::<Vec<String>>()
        .join(" ");
    println!("Args: {}", arg_string);

    println!("Directory contents:");
    let exe_path = env::current_exe()?;
    let root_dir = exe_path.parent().unwrap();
    for entry in WalkDir::new(root_dir) {
        let entry = entry?;
        if entry.path().is_file() {
            let mut file = File::open(entry.path())?;
            println!("{}", entry.path().display());

            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;

            print_tabbed_line("bytes", contents.len());
            print_tabbed_line(
                "created xms ago",
                file.metadata()?.created()?.elapsed()?.as_millis(),
            );
        }
    }

    println!();
    println!();

    Ok(())
}

fn print_tabbed_line<T: fmt::Display>(key: &str, value: T) {
    println!("\t{}: {}", key, value);
}
