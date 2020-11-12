use std::env;

fn main() {
    print!("Args: ");
    for arg in env::args() {
        print!("\"{}\" ", arg);
    }
    println!("");
}
