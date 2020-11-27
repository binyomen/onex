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

fn setup() -> (Provider, ScopeGuard<PathBuf, impl FnOnce(PathBuf)>) {
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
    (provider, temp_dir)
}

fn relative(root: &Path, sub: &str) -> PathBuf {
    [root, &PathBuf::from(sub)].iter().collect()
}

#[test]
fn can_read_file() {
    let (_provider, temp_dir) = setup();

    let content1 = fs::read_to_string(relative(&temp_dir, "file1.txt"))
        .unwrap()
        .trim()
        .to_owned();
    assert_eq!(content1, "file1 contents");

    let content2 = fs::read_to_string(relative(&temp_dir, "dir1/file2.txt"))
        .unwrap()
        .trim()
        .to_owned();
    assert_eq!(content2, "file2 contents");
}
