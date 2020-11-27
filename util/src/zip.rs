use {
    crate::{result::Result, SeekableVec},
    ::zip::{result::ZipError, write::FileOptions, CompressionMethod, ZipArchive, ZipWriter},
    std::{
        fs::{create_dir_all, File},
        io::{self, Read, Seek, Write},
        path::{Path, PathBuf},
    },
    walkdir::{self, WalkDir},
};

pub fn zip_app_dir(app_dir: &Path) -> Result<Vec<u8>> {
    if !app_dir.is_dir() {
        return Err(ZipError::FileNotFound.into());
    }

    let mut output_bytes = SeekableVec::new();
    let mut zip = ZipWriter::new(&mut output_bytes);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for entry_result in WalkDir::new(app_dir) {
        let entry = entry_result?;
        let path = entry.path();
        let stripped_path = path.strip_prefix(app_dir)?;
        let name = stripped_path.to_string_lossy();

        if path.is_file() {
            zip.start_file(name, options)?;
            let mut f = File::open(path)?;

            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if name.len() != 0 {
            zip.add_directory(name, options)?;
        }
    }
    zip.finish()?;

    drop(zip);
    Ok(output_bytes.into_vec())
}

pub fn extract_zip<S: Read + Seek>(seeker: S, output_path: &Path) -> Result<()> {
    let mut archive = ZipArchive::new(seeker)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_output_path: PathBuf =
            [output_path, &PathBuf::from(entry.name())].iter().collect();

        if entry.is_file() {
            let parent = entry_output_path.parent().unwrap();
            if !parent.exists() {
                create_dir_all(&parent)?;
            }

            let mut output_file = File::create(&entry_output_path)?;
            io::copy(&mut entry, &mut output_file)?;
        } else {
            create_dir_all(&entry_output_path)?;
        }
    }

    Ok(())
}

pub fn list_zip_contents<S: Read + Seek>(seeker: S) -> Result<()> {
    let mut archive = ZipArchive::new(seeker)?;

    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().replace("/", "\\");
        println!(
            "{} ({}/{} bytes compressed/uncompressed)",
            name,
            entry.compressed_size(),
            entry.size()
        );
    }

    Ok(())
}
