use {
    std::{
        env,
        fs::{self, remove_dir_all, File},
        io::{Read, Seek},
        path::PathBuf,
        process::Command,
    },
    util::{extract_zip, Result, SexeFile},
    uuid::Uuid,
    winapi::um::wincon::FreeConsole,
};

fn main() {
    real_main().unwrap();
}

fn real_main() -> Result<()> {
    let exe_path = env::current_exe()?;
    let mut file = SexeFile::new(File::open(exe_path)?)?;

    let seeker = file.data_accessor()?;
    run_app(seeker)?;

    Ok(())
}

fn run_app(seeker: impl Read + Seek) -> Result<()> {
    let mut uuid_buffer = Uuid::encode_buffer();
    let instance_id = Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut uuid_buffer);
    let dir_name = format!("sexe_{}", instance_id);
    let temp_dir: PathBuf = [env::temp_dir(), PathBuf::from(dir_name)].iter().collect();
    let temp_dir = scopeguard::guard(temp_dir, |d| {
        let _ = remove_dir_all(d);
    });

    extract_zip(seeker, &temp_dir)?;

    let exe_name_file: PathBuf = [&temp_dir, &PathBuf::from("sexe_run")].iter().collect();
    let exe_name = fs::read_to_string(exe_name_file)?.trim().to_owned();
    let exe_file: PathBuf = [&temp_dir, &PathBuf::from(exe_name)].iter().collect();

    let args: Vec<String> = env::args().skip(1).collect();
    let mut p = Command::new(exe_file).args(&args).spawn()?;

    unsafe { FreeConsole() };
    p.wait()?;

    Ok(())
}
