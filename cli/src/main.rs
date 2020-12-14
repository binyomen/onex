use {
    std::{env, path::PathBuf, process},
    structopt::StructOpt,
    util::Result,
};

const SUPPORTED_ARCHES: [&str; 2] = ["x86_64", "aarch64"];

#[derive(StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    subcommand: Subcommand,
}

#[derive(StructOpt)]
enum Subcommand {
    /// create a single-file executable
    Pack {
        /// directory of the app you want to package
        #[structopt(parse(from_os_str))]
        app_dir: PathBuf,

        /// the final packaged exe to be generated
        #[structopt(parse(from_os_str))]
        output_path: PathBuf,

        /// onex_loader.exe file (default use loader bundled with onex.exe)
        #[structopt(long = "loader", conflicts_with("architecture"), parse(from_os_str))]
        loader_path: Option<PathBuf>,

        /// The architecture of the loader you want to use (default the host architecture)
        #[structopt(long = "arch", possible_values(&SUPPORTED_ARCHES), default_value = env::consts::ARCH)]
        architecture: String,
    },
    /// swap out a loader in one packed app for another
    Swap {
        /// the packaged app you want to modify
        #[structopt(parse(from_os_str))]
        app_path: PathBuf,

        /// the new onex_loader.exe file (default use loader bundled with onex.exe)
        #[structopt(long = "loader", parse(from_os_str))]
        loader_path: Option<PathBuf>,

        /// the final packaged exe to be generated (default modify in place)
        #[structopt(long = "output", conflicts_with("architecture"), parse(from_os_str))]
        output_path: Option<PathBuf>,

        /// The architecture of the loader you want to use (default the host architecture)
        #[structopt(long = "arch", possible_values(&SUPPORTED_ARCHES), default_value = env::consts::ARCH)]
        architecture: String,
    },

    /// List the contents of an onex app
    List {
        /// the packaged app you want to list the contents of
        #[structopt(parse(from_os_str))]
        app_path: PathBuf,
    },

    /// Extract the contents of an onex app
    Extract {
        /// the packaged app you want to extract the contents of
        #[structopt(parse(from_os_str))]
        app_path: PathBuf,

        /// the directory to extract to
        #[structopt(parse(from_os_str))]
        output_path: PathBuf,
    },

    /// Succeeds if the given file is an onex app, fails otherwise
    Check {
        /// the packaged app you want to check
        #[structopt(parse(from_os_str))]
        app_path: PathBuf,
    },

    /// Mounts the onex app to the given directory
    Mount {
        /// the packaged app you want to mount
        #[structopt(parse(from_os_str))]
        app_path: PathBuf,

        /// the directory to mount to
        #[structopt(parse(from_os_str))]
        mount_path: PathBuf,
    },
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let exit_code = match opt.subcommand {
        Subcommand::Pack {
            app_dir,
            output_path,
            loader_path,
            architecture,
        } => onex::package_app(app_dir, output_path, loader_path, architecture).map(|_| 0),
        Subcommand::Swap {
            app_path,
            loader_path,
            output_path,
            architecture,
        } => onex::swap_app_loader(app_path, loader_path, output_path, architecture).map(|_| 0),
        Subcommand::List { app_path } => onex::list_app_contents(app_path).map(|_| 0),
        Subcommand::Extract {
            app_path,
            output_path,
        } => onex::extract_app_contents(app_path, output_path).map(|_| 0),
        Subcommand::Check { app_path } => {
            if onex::check_app(app_path)? {
                Ok(0)
            } else {
                eprintln!("This is not an onex app.");
                Ok(1)
            }
        }
        Subcommand::Mount {
            app_path,
            mount_path,
        } => onex::mount_app(app_path, mount_path).map(|_| 0),
    }?;

    process::exit(exit_code);
}
