use {
    std::{
        ffi::OsString,
        fs,
        io::{BufRead, BufReader, BufWriter, Write},
        iter, mem,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::{Path, PathBuf},
        process::{Child, Command, Stdio},
        slice,
    },
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
};

fn setup() -> (PathBuf, Child) {
    let mut uuid_buffer = Uuid::encode_buffer();
    let instance_id = Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut uuid_buffer);

    let dir_name = format!("sexe_test_{}", instance_id);
    let temp_dir = ["../target/virt_roots", &dir_name]
        .iter()
        .collect::<PathBuf>();

    let mut c = Command::new("../target/debug/test_provider.exe")
        .arg(&temp_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = BufReader::new(c.stdout.as_mut().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert_eq!(line, "ready\n");

    (temp_dir, c)
}

fn shut_down(mut c: Child) {
    {
        let mut writer = BufWriter::new(c.stdin.as_mut().unwrap());
        writer.write_all(b"done\n").unwrap();
    }
    c.wait().unwrap();
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

#[test]
fn can_read_file() {
    let (temp_dir, provider) = setup();

    assert_eq!(read_file(&temp_dir, "file1.txt"), "file1 contents");
    assert_eq!(read_file(&temp_dir, "dir1/file2.txt"), "file2 contents");

    shut_down(provider);
}

#[test]
fn can_enumerate_directory() {
    let (temp_dir, provider) = setup();

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

    shut_down(provider);
}
