use {
    onex_loader::projfs::Provider,
    std::{
        env,
        fs::{self, File},
        path::PathBuf,
        process::{self, Command},
    },
    util::{OffsetSeeker, OnexFile, ReadSeek, Result},
    uuid::Uuid,
    winapi::um::wincon::FreeConsole,
    zip::ZipArchive,
};

fn main() -> Result<()> {
    enable_logging();

    let exe_path = env::current_exe()?;
    let mut file = OnexFile::new(File::open(exe_path)?)?;

    let seeker = file.data_accessor()?;
    let exit_code = run_app(seeker)?;
    process::exit(exit_code);
}

fn run_app(seeker: OffsetSeeker) -> Result<i32> {
    let mut uuid_buffer = Uuid::encode_buffer();
    let instance_id = Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut uuid_buffer);
    let dir_name = format!("onex_{}", instance_id);
    let temp_dir = [onex_loader::get_temp_dir()?, PathBuf::from(dir_name)]
        .iter()
        .collect::<PathBuf>();

    let seeker: Box<dyn ReadSeek> = Box::new(seeker);
    let archive = ZipArchive::new(seeker)?;
    let _provider = Provider::new(&temp_dir, archive)?;

    let exe_name_file = [&temp_dir, &PathBuf::from("onex_run")]
        .iter()
        .collect::<PathBuf>();
    let exe_name = fs::read_to_string(exe_name_file)?.trim().to_owned();
    let exe_file = [&temp_dir, &PathBuf::from(exe_name)]
        .iter()
        .collect::<PathBuf>();

    let args = env::args().skip(1).collect::<Vec<String>>();
    let mut p = Command::new(exe_file).args(&args).spawn()?;

    unsafe { FreeConsole() };
    let exit_status = p.wait()?;
    Ok(exit_status.code().unwrap()) // This should never be None on Windows.
}

#[cfg(debug_assertions)]
fn enable_logging() {
    flexi_logger::Logger::with_str("trace")
        .log_to_file()
        .directory(onex_loader::get_temp_dir().unwrap())
        .discriminant("onex")
        .print_message()
        .start()
        .unwrap();
}

#[cfg(not(debug_assertions))]
fn enable_logging() {}
