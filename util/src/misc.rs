use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
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

pub struct SeekableVec {
    cursor: usize,
    vec: Vec<u8>,
}

impl SeekableVec {
    pub fn new(vec: Vec<u8>) -> Self {
        SeekableVec { cursor: 0, vec }
    }

    pub fn into_vec(self) -> Vec<u8> {
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

    fn read_byte(&mut self) -> Option<u8> {
        if self.cursor == self.vec.len() {
            None
        } else {
            let r = Some(self.vec[self.cursor]);
            self.cursor += 1;
            r
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

impl Seek for SeekableVec {
    fn seek(&mut self, seek_from: SeekFrom) -> io::Result<u64> {
        match seek_from {
            SeekFrom::Start(i) => self.validate_and_set_cursor(i as i64),
            SeekFrom::End(i) => self.validate_and_set_cursor(self.vec.len() as i64 + i),
            SeekFrom::Current(i) => self.validate_and_set_cursor(self.cursor as i64 + i),
        }
    }
}

impl Read for SeekableVec {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for (i, v) in buf.iter_mut().enumerate() {
            match self.read_byte() {
                Some(byte) => *v = byte,
                None => return Ok(i),
            }
        }
        Ok(buf.len())
    }
}

impl Write for SeekableVec {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seekable_vec_no_negative_cursor() {
        {
            let mut s = SeekableVec::new(Vec::new());
            let result = s.seek(SeekFrom::End(-1));
            assert_eq!(result.is_err(), true);
            assert_eq!(
                format!("{}", result.err().unwrap()),
                "Cannot seek before byte 0."
            );
        }

        {
            let mut s = SeekableVec::new(Vec::new());
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
    fn seekable_vec_past_end_ok() {
        {
            let mut s = SeekableVec::new(Vec::new());
            let result = s.seek(SeekFrom::End(1));
            assert_eq!(result.is_ok(), true);
            assert_eq!(result.unwrap(), 0);
        }

        {
            let mut s = SeekableVec::new(Vec::new());
            s.write_all(b"abc").unwrap();
            let result = s.seek(SeekFrom::End(3));
            assert_eq!(result.is_ok(), true);
            assert_eq!(result.unwrap(), 3);
        }
    }

    #[test]
    fn seekable_vec_random_reads_writes() {
        let mut s = SeekableVec::new(Vec::new());
        s.write_all(b"abcdef").unwrap();

        s.seek(SeekFrom::Start(1)).unwrap();
        s.write_all(b"B").unwrap();

        s.seek(SeekFrom::End(-3)).unwrap();
        s.write_all(b"D").unwrap();

        s.seek(SeekFrom::Current(1)).unwrap();
        s.write_all(b"F").unwrap();

        s.seek(SeekFrom::End(0)).unwrap();
        s.write_all(b"g").unwrap();

        let mut result_string = String::new();
        s.seek(SeekFrom::Start(0)).unwrap();
        s.read_to_string(&mut result_string).unwrap();

        assert_eq!(result_string, "aBcDeFg");
        assert_eq!(
            s.into_vec()
                .into_iter()
                .map(|b| b.into())
                .collect::<Vec<char>>(),
            vec!['a', 'B', 'c', 'D', 'e', 'F', 'g']
        )
    }
}
