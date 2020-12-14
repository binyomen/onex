use {
    onex_loader::job_object::create_process_in_job_object,
    std::{
        env,
        fs::{self, File},
        path::PathBuf,
        process,
    },
    util::{get_temp_dir, OffsetSeeker, OnexFile, ProjfsProvider, ReadSeek, Result},
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
    process::exit(exit_code as i32);
}

fn run_app(seeker: OffsetSeeker) -> Result<u32> {
    let mut uuid_buffer = Uuid::encode_buffer();
    let instance_id = Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut uuid_buffer);
    let dir_name = format!("onex_{}", instance_id);
    let temp_dir = [get_temp_dir()?, PathBuf::from(dir_name)]
        .iter()
        .collect::<PathBuf>();

    let seeker: Box<dyn ReadSeek> = Box::new(seeker);
    let archive = ZipArchive::new(seeker)?;
    let _provider = ProjfsProvider::new(&temp_dir, archive)?;

    let exe_name_file = [&temp_dir, &PathBuf::from("onex_run")]
        .iter()
        .collect::<PathBuf>();
    let exe_name = fs::read_to_string(exe_name_file)?.trim().to_owned();
    let exe_file = [&temp_dir, &PathBuf::from(exe_name)]
        .iter()
        .collect::<PathBuf>();

    let args = env::args().skip(1).collect::<Vec<String>>();
    let job = create_process_in_job_object(exe_file, args)?;

    unsafe { FreeConsole() };
    let exit_code = job.wait()?;
    Ok(exit_code)
}

#[cfg(debug_assertions)]
fn enable_logging() {
    flexi_logger::Logger::with_str("trace")
        .log_to_file()
        .directory(get_temp_dir().unwrap())
        .discriminant("onex")
        .print_message()
        .start()
        .unwrap();
}

#[cfg(not(debug_assertions))]
fn enable_logging() {}
