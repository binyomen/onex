use std::{
    env::current_exe,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

#[allow(dead_code)]
fn read_image_size() -> io::Result<u64> {
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

    return Ok(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_read_image_size() {
        read_image_size().unwrap();
    }
}
