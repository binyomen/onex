use {
    sexe_loader::projfs::Provider,
    std::{
        env,
        fs::{self, File},
        path::PathBuf,
        process::Command,
    },
    util::{OffsetSeeker, ReadSeek, Result, SexeFile},
    uuid::Uuid,
    winapi::um::wincon::FreeConsole,
    zip::ZipArchive,
};

fn main() -> Result<()> {
    enable_logging();

    let exe_path = env::current_exe()?;
    let mut file = SexeFile::new(File::open(exe_path)?)?;

    let seeker = file.data_accessor()?;
    run_app(seeker)?;

    Ok(())
}

fn run_app(seeker: OffsetSeeker) -> Result<()> {
    let mut uuid_buffer = Uuid::encode_buffer();
    let instance_id = Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut uuid_buffer);
    let dir_name = format!("sexe_{}", instance_id);
    let temp_dir = [env::temp_dir(), PathBuf::from(dir_name)]
        .iter()
        .collect::<PathBuf>();

    let seeker: Box<dyn ReadSeek> = Box::new(seeker);
    let archive = ZipArchive::new(seeker)?;
    let _provider = Provider::new(&temp_dir, archive)?;

    let exe_name_file = [&temp_dir, &PathBuf::from("sexe_run")]
        .iter()
        .collect::<PathBuf>();
    let exe_name = fs::read_to_string(exe_name_file)?.trim().to_owned();
    let exe_file = [&temp_dir, &PathBuf::from(exe_name)]
        .iter()
        .collect::<PathBuf>();

    let args = env::args().skip(1).collect::<Vec<String>>();
    let mut p = Command::new(exe_file).args(&args).spawn()?;

    unsafe { FreeConsole() };
    p.wait()?;

    Ok(())
}

#[cfg(debug_assertions)]
fn enable_logging() {
    flexi_logger::Logger::with_str("trace")
        .log_to_file()
        .directory(env::temp_dir())
        .discriminant("sexe")
        .print_message()
        .start()
        .unwrap();
}

#[cfg(not(debug_assertions))]
fn enable_logging() {}
