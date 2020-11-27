use {
    sexe_loader::projfs::{Provider, ReadSeek},
    std::{
        env,
        ffi::OsString,
        fs, iter, mem,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::{Path, PathBuf},
        slice,
        sync::Once,
    },
    util::{zip_app_dir, SeekableVec},
    uuid::Uuid,
    winapi_local::{
        shared::ntdef::TRUE,
        um::{
            fileapi::{FindFirstFileW, FindNextFileW},
            handleapi::INVALID_HANDLE_VALUE,
            minwinbase::WIN32_FIND_DATAW,
            winnt::HANDLE,
        },
    },
    zip::ZipArchive,
};

fn setup() -> (PathBuf, Provider) {
    let mut uuid_buffer = Uuid::encode_buffer();
    let instance_id = Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut uuid_buffer);

    let dir_name = format!("sexe_test_{}", instance_id);
    let temp_dir = [env::temp_dir(), PathBuf::from(dir_name)]
        .iter()
        .collect::<PathBuf>();

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

fn to_u16_vec<T: Into<OsString>>(s: T) -> Vec<u16> {
    s.into()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<u16>>()
}

fn raw_str_to_string(s: *const u16) -> String {
    let len = get_raw_str_length(s);
    let slice = unsafe { slice::from_raw_parts(s, len) };
    (*OsString::from_wide(slice).to_string_lossy()).to_owned()
}

fn get_raw_str_length(s: *const u16) -> usize {
    let mut i = 0;
    while unsafe { *s.offset(i) } != 0 {
        i += 1;
    }
    i as usize
}

fn find_first_file(search: &str) -> Option<(HANDLE, String)> {
    let mut data = unsafe { mem::zeroed::<WIN32_FIND_DATAW>() };
    let handle = unsafe { FindFirstFileW(to_u16_vec(search).as_ptr(), &mut data) };
    if handle != INVALID_HANDLE_VALUE {
        Some((handle, raw_str_to_string(data.cFileName.as_ptr())))
    } else {
        None
    }
}

fn find_next_file(handle: HANDLE) -> Option<String> {
    let mut data = unsafe { mem::zeroed::<WIN32_FIND_DATAW>() };
    let succeeded = unsafe { FindNextFileW(handle, &mut data) };
    if succeeded == TRUE.into() {
        Some(raw_str_to_string(data.cFileName.as_ptr()))
    } else {
        None
    }
}

fn read_dir_wildcards(root: &Path, search: &str) -> Vec<String> {
    match find_first_file(&*relative(root, search).to_string_lossy()) {
        Some((handle, file)) => {
            let mut files = vec![file];
            while let Some(file) = find_next_file(handle) {
                files.push(file);
            }
            files
        }
        None => Vec::new(),
    }
}

static LOGGING: Once = Once::new();
fn enable_logging() {
    LOGGING.call_once(|| {
        flexi_logger::Logger::with_str("trace").start().unwrap();
    });
}

#[test]
fn can_read_file() {
    enable_logging();
    let (temp_dir, _provider) = setup();

    assert_eq!(read_file(&temp_dir, "file1.txt"), "file1 contents");
    assert_eq!(read_file(&temp_dir, "dir1/file2.txt"), "file2 contents");
}

#[test]
fn can_enumerate_directory() {
    enable_logging();
    let (temp_dir, _provider) = setup();

    assert_eq!(
        read_dir(&temp_dir, ""),
        vec!["dir1", "file1.txt", "sexe_run"]
    );
    assert_eq!(read_dir(&temp_dir, "dir1"), vec!["file2.txt", "file3.txt"]);

    assert_eq!(
        read_dir_wildcards(&temp_dir, "*"),
        vec![".", "..", "dir1", "file1.txt", "sexe_run"]
    );
    assert_eq!(
        read_dir_wildcards(&temp_dir, "dir1/*"),
        vec![".", "..", "file2.txt", "file3.txt"]
    );

    assert_eq!(read_dir_wildcards(&temp_dir, "f*"), vec!["file1.txt"]);
    assert_eq!(read_dir_wildcards(&temp_dir, "*f*"), vec!["file1.txt"]);
    assert_eq!(read_dir_wildcards(&temp_dir, "*f*t"), vec!["file1.txt"]);
    assert_eq!(read_dir_wildcards(&temp_dir, "*f*txt*"), vec!["file1.txt"]);

    assert_eq!(
        read_dir_wildcards(&temp_dir, "dir1/*2"),
        Vec::new() as Vec<String>
    );
    assert_eq!(read_dir_wildcards(&temp_dir, "dir1/*2*"), vec!["file2.txt"]);
    assert_eq!(read_dir_wildcards(&temp_dir, "sexe_run"), vec!["sexe_run"]);
    assert_eq!(
        read_dir_wildcards(&temp_dir, "dir1/file3.txt"),
        vec!["file3.txt"]
    );
}
