use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    subcommand: Subcommand,
}

#[derive(StructOpt)]
enum Subcommand {
    /// create a single-file executable
    Pack {
        /// sexe_loader.exe file
        #[structopt(parse(from_os_str))]
        loader_path: PathBuf,

        /// directory of the app you want to package
        #[structopt(parse(from_os_str))]
        app_dir: PathBuf,

        /// the final packaged exe to be generated
        #[structopt(parse(from_os_str))]
        output_path: PathBuf,
    },
}

fn main() {
    let opt = Opt::from_args();
    match opt.subcommand {
        Subcommand::Pack {
            loader_path,
            app_dir,
            output_path,
        } => sexe::package_app(loader_path, app_dir, output_path).unwrap(),
    }
}
