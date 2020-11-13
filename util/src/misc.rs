use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

pub struct OffsetSeeker {
    file: File,
    offset: u64,
    cursor: u64,
    length: u64,
}

impl OffsetSeeker {
    pub fn new(mut file: File, offset: u64, length: u64) -> io::Result<Self> {
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
