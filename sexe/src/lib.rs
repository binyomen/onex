mod zip_util;

use std::{
    error, fmt,
    fs::File,
    io::{self, Read, Write},
    path::StripPrefixError,
};
use walkdir;
use zip::result::ZipError;
use zip_util::get_app_dir_bytes;

#[derive(Debug)]
pub enum SexeError {
    Io(io::Error),
    Zip(ZipError),
    Walkdir(walkdir::Error),
    StripPrefix(StripPrefixError),
}

impl fmt::Display for SexeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SexeError::Io(err) => err.fmt(f),
            SexeError::Zip(err) => err.fmt(f),
            SexeError::Walkdir(err) => err.fmt(f),
            SexeError::StripPrefix(err) => err.fmt(f),
        }
    }
}

impl error::Error for SexeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            SexeError::Io(err) => Some(err),
            SexeError::Zip(err) => Some(err),
            SexeError::Walkdir(err) => Some(err),
            SexeError::StripPrefix(err) => Some(err),
        }
    }
}

impl From<io::Error> for SexeError {
    fn from(err: io::Error) -> Self {
        SexeError::Io(err)
    }
}

pub type SexeResult<T> = Result<T, SexeError>;

pub fn package_app(loader_path: String, app_dir: String, output_path: String) -> SexeResult<()> {
    let mut loader = File::open(&loader_path)?;
    let mut loader_bytes = Vec::new();
    loader.read_to_end(&mut loader_bytes)?;

    let app_dir_bytes = get_app_dir_bytes(&app_dir)?;
    let output = get_output_bytes(loader_bytes, app_dir_bytes);

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
