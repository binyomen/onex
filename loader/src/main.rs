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
    winapi::um::{
        wincon::GetConsoleWindow,
        winuser::{ShowWindow, SW_HIDE},
    },
};

fn main() {
    real_main().unwrap();
}

fn real_main() -> Result<()> {
    // When a packaged app is run from the start menu, a console window is
    // created for the loader. When it's a GUI app, the console window stays
    // open while the GUI stays open. And if you close the console window
    // before closing the app itself, the loader exits and won't clean up after
    // itself. Hide the console window so this isn't an issue.
    hide_console_window();
    ctrlc::set_handler(|| {})?;

    let exe_path = env::current_exe()?;
    let mut file = SexeFile::new(File::open(exe_path)?)?;

    let seeker = file.data_accessor()?;
    run_app(seeker)?;

    Ok(())
}

fn hide_console_window() {
    let window = unsafe { GetConsoleWindow() };
    if !window.is_null() {
        unsafe {
            ShowWindow(window, SW_HIDE);
        }
    }
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
    Command::new(exe_file).args(&args).spawn()?.wait()?;

    Ok(())
}
