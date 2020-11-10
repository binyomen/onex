use std::{
    env,
    fs::File,
    io::{Read, Write},
};

fn main() {
    let mut args = env::args();
    args.next().unwrap();
    let loader_path = args.next().unwrap();
    let data = args.next().unwrap();
    let output_path = args.next().unwrap();

    let mut loader = File::open(&loader_path).unwrap();
    let mut loader_content = Vec::new();
    loader.read_to_end(&mut loader_content).unwrap();

    let data_offset = loader_content.len() as u64;
    let mut new_content = loader_content;
    new_content.append(&mut data.into_bytes());
    let mut output_file = File::create(&output_path).unwrap();
    output_file.write_all(&new_content).unwrap();

    let data_offset_bytes = data_offset.to_le_bytes();
    output_file.write_all(&data_offset_bytes).unwrap();
}
