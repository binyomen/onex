use {
    scopeguard::ScopeGuard,
    sexe_loader::projfs::{Provider, ReadSeek},
    std::{
        env, fs,
        path::{Path, PathBuf},
    },
    util::{zip_app_dir, SeekableVec},
    uuid::Uuid,
    zip::ZipArchive,
};

fn setup() -> (ScopeGuard<PathBuf, impl FnOnce(PathBuf)>, Provider) {
    let mut uuid_buffer = Uuid::encode_buffer();
    let instance_id = Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut uuid_buffer);

    let dir_name = format!("sexe_test_{}", instance_id);
    let temp_dir: PathBuf = [env::temp_dir(), PathBuf::from(dir_name)].iter().collect();
    fs::create_dir(&temp_dir).unwrap();
    let temp_dir = scopeguard::guard(temp_dir, |d| {
        let _ = fs::remove_dir_all(d);
    });

    let zip_bytes = zip_app_dir(&PathBuf::from("../testapp/assets")).unwrap();
    let seeker = SeekableVec::new(zip_bytes);

    let seeker: Box<dyn ReadSeek> = Box::new(seeker);
    let archive = ZipArchive::new(seeker).unwrap();

    let provider = Provider::new(&temp_dir, archive).unwrap();
    (temp_dir, provider)
}

fn relative(root: &Path, sub: &str) -> PathBuf {
    [root, &PathBuf::from(sub)].iter().collect()
}

fn read_file(root: &Path, file: &str) -> String {
    fs::read_to_string(relative(root, file))
        .unwrap()
        .trim()
        .to_owned()
}

fn read_dir(root: &Path, dir: &str) -> Vec<String> {
    fs::read_dir(relative(root, dir))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_str().unwrap().to_owned())
        .collect::<Vec<String>>()
}

#[test]
fn can_read_file() {
    let (temp_dir, _provider) = setup();

    assert_eq!(read_file(&temp_dir, "file1.txt"), "file1 contents");
    assert_eq!(read_file(&temp_dir, "dir1/file2.txt"), "file2 contents");
}

#[test]
fn can_enumerate_directory() {
    let (temp_dir, _provider) = setup();

    assert_eq!(
        read_dir(&temp_dir, ""),
        vec!["dir1", "file1.txt", "sexe_run"]
    );
    assert_eq!(read_dir(&temp_dir, "dir1"), vec!["file2.txt", "file3.txt"]);
}
