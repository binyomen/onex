use {
    sexe_loader::projfs::{Provider, ReadSeek},
    std::{env, error, io, path::PathBuf},
    util::{zip_app_dir, SeekableVec},
    zip::ZipArchive,
};

fn main() -> Result<(), Box<dyn error::Error>> {
    let mut args = env::args();
    let root = args.nth(1).unwrap();

    let zip_bytes = zip_app_dir(&PathBuf::from("../testapp/assets")).unwrap();
    let seeker = SeekableVec::new(zip_bytes);

    let seeker: Box<dyn ReadSeek> = Box::new(seeker);
    let archive = ZipArchive::new(seeker).unwrap();

    let _provider = Provider::new(&PathBuf::from(root), archive).unwrap();

    println!("ready");

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;

    Ok(())
}
