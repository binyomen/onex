use std::{
    env,
    fs::File,
    io::{Read, Seek, SeekFrom},
};

fn main() {
    let exe_path = env::current_exe().unwrap();
    let mut file = File::open(exe_path).unwrap();

    file.seek(SeekFrom::End(-8)).unwrap();
    let mut data_offset = [0; 8];
    file.read_exact(&mut data_offset).unwrap();
    let data_offset = u64::from_le_bytes(data_offset);

    let file_length = file.metadata().unwrap().len();
    let data_length = file_length - data_offset - 8;
    file.seek(SeekFrom::Start(data_offset)).unwrap();
    let mut data = vec![0; data_length as usize];
    file.read_exact(&mut data).unwrap();

    let data_chars: Vec<char> = data.iter().map(|b| char::from(*b)).collect();
    println!("{:?}", data_chars);
}
