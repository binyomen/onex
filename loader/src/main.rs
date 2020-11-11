use std::{
    env,
    fs::{create_dir_all, File},
    io::{self, Read, Seek, SeekFrom},
    path::PathBuf,
};
use uuid::Uuid;
use zip::ZipArchive;

struct OffsetSeeker {
    file: File,
    offset: u64,
    cursor: u64,
    length: u64,
}

impl OffsetSeeker {
    fn new(mut file: File, offset: u64, length: u64) -> io::Result<Self> {
        file.seek(SeekFrom::Start(offset))?;
        Ok(OffsetSeeker {
            file,
            offset,
            cursor: 0,
            length,
        })
    }
}

impl Seek for OffsetSeeker {
    fn seek(&mut self, seek_from: SeekFrom) -> io::Result<u64> {
        let new_cursor = match seek_from {
            SeekFrom::Start(i) => i as i64,
            SeekFrom::End(i) => self.length as i64 + i,
            SeekFrom::Current(i) => self.cursor as i64 + i,
        };

        if new_cursor < 0 {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Cannot seek before byte 0.",
            ))
        } else {
            self.cursor = if new_cursor <= self.length as i64 {
                new_cursor as u64
            } else {
                self.length
            };

            self.file.seek(SeekFrom::Start(self.cursor + self.offset))?;
            Ok(self.cursor)
        }
    }
}

impl Read for OffsetSeeker {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let resized_buf = if self.cursor + buf.len() as u64 > self.length {
            &mut buf[..(self.length - self.cursor) as usize]
        } else {
            buf
        };
        let result = self.file.read(resized_buf)?;
        self.cursor += result as u64;
        Ok(result)
    }
}

fn main() {
    let exe_path = env::current_exe().unwrap();
    let mut file = File::open(exe_path).unwrap();

    file.seek(SeekFrom::End(-8)).unwrap();
    let mut data_offset = [0; 8];
    file.read_exact(&mut data_offset).unwrap();
    let data_offset = u64::from_le_bytes(data_offset);

    let file_length = file.metadata().unwrap().len();
    let data_length = file_length - data_offset - 8;

    let seeker = OffsetSeeker::new(file, data_offset, data_length).unwrap();
    print_app_dir(seeker).unwrap();
}

fn print_app_dir(seeker: OffsetSeeker) -> io::Result<()> {
    let mut archive = ZipArchive::new(seeker)?;
    let mut uuid_buffer = Uuid::encode_buffer();
    let instance_id = Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut uuid_buffer);
    let temp_dir = [env::temp_dir(), PathBuf::from(instance_id.to_owned())]
        .iter()
        .collect();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let output_path: PathBuf = [&temp_dir, &PathBuf::from(entry.name())].iter().collect();

        if entry.is_file() {
            let parent = output_path.parent().unwrap();
            if !parent.exists() {
                create_dir_all(&parent)?;
            }

            let mut output_file = File::create(&output_path)?;
            io::copy(&mut entry, &mut output_file)?;
        } else {
            create_dir_all(&output_path)?;
        }
    }

    Ok(())
}
