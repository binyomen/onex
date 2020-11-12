use std::env;

fn main() {
    let mut args = env::args();
    args.next().unwrap();
    let loader_path = args.next().unwrap();
    let app_dir = args.next().unwrap();
    let output_path = args.next().unwrap();

    sexe::package_app(loader_path, app_dir, output_path).unwrap();
}
