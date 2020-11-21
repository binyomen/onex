use {
    std::{
        fs::File,
        io::{Read, Write},
        path::PathBuf,
    },
    util::{zip_app_dir, Result, SexeFile},
};

pub fn package_app(loader_path: PathBuf, app_dir: PathBuf, output_path: PathBuf) -> Result<()> {
    let mut loader_file = File::open(&loader_path)?;
    let mut loader_bytes = Vec::new();
    loader_file.read_to_end(&mut loader_bytes)?;

    let app_dir_bytes = zip_app_dir(&app_dir)?;
    let output = SexeFile::generate_bytes(loader_bytes, app_dir_bytes);

    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&output)?;

    Ok(())
}

pub fn swap_app_loader(
    app_path: PathBuf,
    loader_path: PathBuf,
    output_path: Option<PathBuf>,
) -> Result<()> {
    let mut loader_file = File::open(&loader_path)?;
    let mut loader_bytes = Vec::new();
    loader_file.read_to_end(&mut loader_bytes)?;

    let mut sexe_file = SexeFile::new(File::open(&app_path)?)?;
    let output = SexeFile::generate_bytes(loader_bytes, sexe_file.data()?);

    let output_path = output_path.unwrap_or(app_path);
    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&output)?;

    Ok(())
}
