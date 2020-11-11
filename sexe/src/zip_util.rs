use crate::{SexeError, SexeResult};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, StripPrefixError},
};
use walkdir::{self, WalkDir};
use zip::{result::ZipError, write::FileOptions, CompressionMethod, ZipWriter};

struct SeekableWriter {
    cursor: usize,
    vec: Vec<u8>,
}

impl SeekableWriter {
    fn new() -> Self {
        SeekableWriter {
            cursor: 0,
            vec: Vec::new(),
        }
    }

    fn to_vec(self) -> Vec<u8> {
        self.vec
    }

    fn validate_and_set_cursor(&mut self, new_cursor: i64) -> io::Result<u64> {
        if new_cursor < 0 {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Cannot seek before byte 0.",
            ))
        } else {
            self.cursor = if new_cursor <= self.vec.len() as i64 {
                new_cursor as usize
            } else {
                self.vec.len()
            };
            Ok(self.cursor as u64)
        }
    }

    fn write_byte(&mut self, byte: u8) {
        if self.cursor == self.vec.len() {
            self.vec.push(byte);
        } else {
            self.vec[self.cursor] = byte;
        }
        self.cursor += 1;
    }
}

impl Seek for SeekableWriter {
    fn seek(&mut self, seek_from: SeekFrom) -> io::Result<u64> {
        match seek_from {
            SeekFrom::Start(i) => self.validate_and_set_cursor(i as i64),
            SeekFrom::End(i) => self.validate_and_set_cursor(self.vec.len() as i64 + i),
            SeekFrom::Current(i) => self.validate_and_set_cursor(self.cursor as i64 + i),
        }
    }
}

impl Write for SeekableWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for byte in buf {
            self.write_byte(*byte);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl From<ZipError> for SexeError {
    fn from(err: ZipError) -> Self {
        SexeError::Zip(err)
    }
}

impl From<walkdir::Error> for SexeError {
    fn from(err: walkdir::Error) -> Self {
        SexeError::Walkdir(err)
    }
}

impl From<StripPrefixError> for SexeError {
    fn from(err: StripPrefixError) -> Self {
        SexeError::StripPrefix(err)
    }
}

pub fn get_app_dir_bytes(app_dir: &str) -> SexeResult<Vec<u8>> {
    if !Path::new(app_dir).is_dir() {
        return Err(ZipError::FileNotFound.into());
    }

    let mut output_bytes = SeekableWriter::new();
    let mut zip = ZipWriter::new(&mut output_bytes);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for entry_result in WalkDir::new(app_dir) {
        let entry = entry_result?;
        let path = entry.path();
        let stripped_path = path.strip_prefix(Path::new(app_dir))?;
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
    Ok(output_bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seekable_writer_no_negative_cursor() {
        {
            let mut s = SeekableWriter::new();
            let result = s.seek(SeekFrom::End(-1));
            assert_eq!(result.is_err(), true);
            assert_eq!(
                format!("{}", result.err().unwrap()),
                "Cannot seek before byte 0."
            );
        }

        {
            let mut s = SeekableWriter::new();
            s.write_all(b"abc").unwrap();
            let result = s.seek(SeekFrom::End(-4));
            assert_eq!(result.is_err(), true);
            assert_eq!(
                format!("{}", result.err().unwrap()),
                "Cannot seek before byte 0."
            );
        }
    }

    #[test]
    fn seekable_writer_past_end_ok() {
        {
            let mut s = SeekableWriter::new();
            let result = s.seek(SeekFrom::End(1));
            assert_eq!(result.is_ok(), true);
            assert_eq!(result.unwrap(), 0);
        }

        {
            let mut s = SeekableWriter::new();
            s.write_all(b"abc").unwrap();
            let result = s.seek(SeekFrom::End(3));
            assert_eq!(result.is_ok(), true);
            assert_eq!(result.unwrap(), 3);
        }
    }

    #[test]
    fn seekable_writer_random_writes() {
        let mut s = SeekableWriter::new();
        s.write_all(b"abcdef").unwrap();

        s.seek(SeekFrom::Start(1)).unwrap();
        s.write_all(b"B").unwrap();

        s.seek(SeekFrom::End(-3)).unwrap();
        s.write_all(b"D").unwrap();

        s.seek(SeekFrom::Current(1)).unwrap();
        s.write_all(b"F").unwrap();

        s.seek(SeekFrom::End(0)).unwrap();
        s.write_all(b"g").unwrap();

        assert_eq!(
            s.to_vec()
                .into_iter()
                .map(|b| b.into())
                .collect::<Vec<char>>(),
            vec!['a', 'B', 'c', 'D', 'e', 'F', 'g']
        )
    }
}
