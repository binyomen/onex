use {
    crate::{misc::OffsetSeeker, result::SexeResult},
    std::{
        fs::File,
        io::{Read, Seek, SeekFrom},
    },
};

const DATA_OFFSET_LENGTH: usize = 8;

pub struct SexeFile {
    f: File,
}

impl SexeFile {
    pub fn new(f: File) -> Self {
        SexeFile { f }
    }

    pub fn generate_bytes(loader_bytes: Vec<u8>, data_bytes: Vec<u8>) -> Vec<u8> {
        let data_offset = loader_bytes.len() as u64;

        let mut bytes =
            Vec::with_capacity(loader_bytes.len() + data_bytes.len() + DATA_OFFSET_LENGTH);
        bytes.extend(loader_bytes);
        bytes.extend(data_bytes);

        let data_offset_bytes = data_offset.to_le_bytes();
        bytes.extend(&data_offset_bytes);

        bytes
    }

    pub fn data_offset(&mut self) -> SexeResult<u64> {
        self.f.seek(SeekFrom::End(-(DATA_OFFSET_LENGTH as i64)))?;

        let mut data_offset = [0; DATA_OFFSET_LENGTH];
        self.f.read_exact(&mut data_offset)?;

        Ok(u64::from_le_bytes(data_offset))
    }

    pub fn data(&mut self) -> SexeResult<Vec<u8>> {
        let mut accessor = self.data_accessor()?;
        let mut data_bytes = Vec::new();
        accessor.read_to_end(&mut data_bytes)?;
        Ok(data_bytes)
    }

    pub fn data_accessor(&mut self) -> SexeResult<impl Read + Seek> {
        Ok(OffsetSeeker::new(
            self.f.try_clone()?,
            self.data_offset()?,
            self.data_length()?,
        )?)
    }

    fn file_length(&self) -> SexeResult<u64> {
        Ok(self.f.metadata()?.len())
    }

    fn data_length(&mut self) -> SexeResult<u64> {
        Ok(self.file_length()? - self.data_offset()? - DATA_OFFSET_LENGTH as u64)
    }
}
