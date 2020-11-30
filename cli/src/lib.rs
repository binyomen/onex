use {
    std::{
        env,
        fs::File,
        io::{Read, Write},
        path::PathBuf,
    },
    util::{extract_zip, list_zip_contents, zip_app_dir, OnexFile, Result},
};

pub fn package_app(
    app_dir: PathBuf,
    output_path: PathBuf,
    loader_path: Option<PathBuf>,
) -> Result<()> {
    let loader_path = get_loader_bytes(loader_path)?;
    let mut loader_file = File::open(&loader_path)?;
    let mut loader_bytes = Vec::new();
    loader_file.read_to_end(&mut loader_bytes)?;

    let app_dir_bytes = zip_app_dir(&app_dir)?;
    let output = OnexFile::generate_bytes(loader_bytes, app_dir_bytes);

    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&output)?;

    Ok(())
}

pub fn swap_app_loader(
    app_path: PathBuf,
    loader_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
) -> Result<()> {
    let loader_path = get_loader_bytes(loader_path)?;
    let mut loader_file = File::open(&loader_path)?;
    let mut loader_bytes = Vec::new();
    loader_file.read_to_end(&mut loader_bytes)?;

    let mut onex_file = OnexFile::new(File::open(&app_path)?)?;
    let output = OnexFile::generate_bytes(loader_bytes, onex_file.data()?);

    let output_path = output_path.unwrap_or(app_path);
    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&output)?;

    Ok(())
}

pub fn list_app_contents(app_path: PathBuf) -> Result<()> {
    let mut onex_file = OnexFile::new(File::open(&app_path)?)?;
    list_zip_contents(onex_file.data_accessor()?)?;
    Ok(())
}

pub fn extract_app_contents(app_path: PathBuf, output_path: PathBuf) -> Result<()> {
    let mut onex_file = OnexFile::new(File::open(&app_path)?)?;
    extract_zip(onex_file.data_accessor()?, &output_path)?;
    Ok(())
}

pub fn check_app(app_path: PathBuf) -> Result<bool> {
    let mut file = File::open(&app_path)?;
    Ok(OnexFile::validate(&mut file).is_ok())
}

fn get_loader_bytes(loader_path: Option<PathBuf>) -> Result<PathBuf> {
    let exe_path = env::current_exe()?;
    Ok(loader_path.unwrap_or_else(|| {
        match exe_path.parent() {
            Some(path) => path.join("onex_loader.exe"),
            // This should never be reached, since the parent of a file should
            // always be its containing directory.
            None => unreachable!(),
        }
    }))
}
