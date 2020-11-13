use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};
use util::{zip_app_dir, SexeResult};

pub fn package_app(loader_path: PathBuf, app_dir: PathBuf, output_path: PathBuf) -> SexeResult<()> {
    let mut loader_file = File::open(&loader_path)?;
    let mut loader_bytes = Vec::new();
    loader_file.read_to_end(&mut loader_bytes)?;

    let app_dir_bytes = zip_app_dir(&app_dir)?;
    let output = get_output_bytes(loader_bytes, app_dir_bytes);

    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&output)?;

    Ok(())
}

pub fn swap_app_loader(
    app_path: PathBuf,
    loader_path: PathBuf,
    output_path: Option<PathBuf>,
) -> SexeResult<()> {
    let mut loader_file = File::open(&loader_path)?;
    let mut loader_bytes = Vec::new();
    loader_file.read_to_end(&mut loader_bytes)?;

    let data_bytes = get_data_bytes(&app_path)?;
    let output = get_output_bytes(loader_bytes, data_bytes);

    let output_path = output_path.unwrap_or(app_path);
    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&output)?;

    Ok(())
}

fn get_output_bytes(loader_bytes: Vec<u8>, data_bytes: Vec<u8>) -> Vec<u8> {
    let data_offset = loader_bytes.len() as u64;

    let mut output = Vec::with_capacity(loader_bytes.len() + data_bytes.len() + 8);
    output.extend(loader_bytes);
    output.extend(data_bytes);

    let data_offset_bytes = data_offset.to_le_bytes();
    output.extend(&data_offset_bytes);

    return output;
}

fn get_data_bytes(app_path: &PathBuf) -> io::Result<Vec<u8>> {
    let mut app_file = File::open(&app_path)?;

    app_file.seek(SeekFrom::End(-8))?;
    let mut data_offset = [0; 8];
    app_file.read_exact(&mut data_offset)?;
    let data_offset = u64::from_le_bytes(data_offset);

    let file_length = app_file.metadata()?.len();
    let data_length = file_length - data_offset - 8;

    let mut data = vec![0; data_length as usize];
    app_file.seek(SeekFrom::Start(data_offset))?;
    app_file.read_exact(&mut data)?;

    Ok(data)
}
