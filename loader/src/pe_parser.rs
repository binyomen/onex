use std::{
    env::current_exe,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

#[allow(dead_code)]
fn read_image_size() -> io::Result<u32> {
    let exe = current_exe()?;
    println!("Exe: {:?}", exe);
    let mut file = File::open(exe)?;

    file.seek(SeekFrom::Start(0x3c))?;
    let mut signature_offset = [0; 1];
    file.read_exact(&mut signature_offset)?;
    println!("Signature offset: {:?}", signature_offset[0]);

    file.seek(SeekFrom::Start(signature_offset[0].into()))?;
    let mut signature = [0; 4];
    file.read_exact(&mut signature)?;
    let signature_chars: Vec<char> = signature.iter().map(|b| char::from(*b)).collect();
    println!("Signature: {:?}", signature_chars);

    file.seek(SeekFrom::Current(20))?;
    let mut optional_header_magic = [0; 2];
    file.read_exact(&mut optional_header_magic)?;
    let optional_header_magic = u16::from_le_bytes(optional_header_magic);
    println!("Optional header magic bytes: 0x{:x}", optional_header_magic);
    file.seek(SeekFrom::Current(-2))?;

    file.seek(SeekFrom::Current(32))?;
    let mut section_alignment = [0; 4];
    file.read_exact(&mut section_alignment)?;
    let section_alignment = u32::from_le_bytes(section_alignment);
    println!("Section alignment: {}", section_alignment);
    file.seek(SeekFrom::Current(-4))?;
    file.seek(SeekFrom::Current(-32))?;

    file.seek(SeekFrom::Current(36))?;
    let mut file_alignment = [0; 4];
    file.read_exact(&mut file_alignment)?;
    let file_alignment = u32::from_le_bytes(file_alignment);
    println!("File alignment: {}", file_alignment);
    file.seek(SeekFrom::Current(-4))?;
    file.seek(SeekFrom::Current(-36))?;

    file.seek(SeekFrom::Current(56))?;
    let mut image_size = [0; 4];
    file.read_exact(&mut image_size)?;
    let image_size = u32::from_le_bytes(image_size);
    println!("Image size: {}", image_size);

    return Ok(image_size);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_read_image_size() {
        let exe = current_exe().unwrap();
        let file = File::open(exe).unwrap();

        let expected_image_size = file.metadata().unwrap().len();
        let actual_image_size = read_image_size().unwrap();
        assert_eq!(expected_image_size, actual_image_size.into());
    }
}
