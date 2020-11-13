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
    /// swap out a loader in one packed app for another
    Swap {
        /// the packaged app you want to modify
        #[structopt(parse(from_os_str))]
        app_path: PathBuf,

        /// the new sexe_loader.exe file
        #[structopt(parse(from_os_str))]
        loader_path: PathBuf,

        /// the final packaged exe to be generated (default modify in place)
        #[structopt(parse(from_os_str))]
        output_path: Option<PathBuf>,
    },
}

fn main() {
    let opt = Opt::from_args();
    match opt.subcommand {
        Subcommand::Pack {
            loader_path,
            app_dir,
            output_path,
        } => sexe::package_app(loader_path, app_dir, output_path),
        Subcommand::Swap {
            app_path,
            loader_path,
            output_path,
        } => sexe::swap_app_loader(app_path, loader_path, output_path),
    }
    .unwrap();
}
