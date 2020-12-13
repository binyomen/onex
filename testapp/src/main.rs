use {
    std::{env, error, ffi::OsString, fmt, fs::File, io::Read, iter, os::windows::ffi::OsStrExt},
    walkdir::WalkDir,
    winapi::{
        shared::minwindef::DWORD,
        um::{
            fileapi::GetFileAttributesW,
            winnt::{
                FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_COMPRESSED, FILE_ATTRIBUTE_DEVICE,
                FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_ENCRYPTED, FILE_ATTRIBUTE_HIDDEN,
                FILE_ATTRIBUTE_INTEGRITY_STREAM, FILE_ATTRIBUTE_NORMAL,
                FILE_ATTRIBUTE_NOT_CONTENT_INDEXED, FILE_ATTRIBUTE_NO_SCRUB_DATA,
                FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_READONLY,
                FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
                FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_SPARSE_FILE, FILE_ATTRIBUTE_SYSTEM,
                FILE_ATTRIBUTE_TEMPORARY, FILE_ATTRIBUTE_VIRTUAL,
            },
        },
    },
};

fn main() -> Result<(), Box<dyn error::Error>> {
    let arg_string = env::args()
        .map(|a| format!("\"{}\"", a))
        .collect::<Vec<String>>()
        .join(" ");
    println!("Args: {}", arg_string);

    println!("Directory contents:");
    let exe_path = env::current_exe()?;
    let root_dir = exe_path.parent().unwrap();
    for entry in WalkDir::new(root_dir) {
        let entry = entry?;
        println!("{}", entry.path().display());

        if entry.path().is_file() {
            let mut file = File::open(entry.path())?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;
            print_tabbed_line("bytes", contents.len());
        }

        print_tabbed_line(
            "created xms ago",
            entry.metadata()?.created()?.elapsed()?.as_millis(),
        );

        let attrs = unsafe { GetFileAttributesW(to_u16_vec(entry.path()).as_ptr()) };
        print_tabbed_line("attributes", display_attributes(attrs));
    }

    println!();
    println!();

    Ok(())
}

fn print_tabbed_line<T: fmt::Display>(key: &str, value: T) {
    println!("\t{}: {}", key, value);
}

macro_rules! file_attribute {
    ($a:ident) => {
        ($a, stringify!($a))
    };
}

const ATTRIBUTES: [(DWORD, &str); 19] = [
    file_attribute!(FILE_ATTRIBUTE_ARCHIVE),
    file_attribute!(FILE_ATTRIBUTE_COMPRESSED),
    file_attribute!(FILE_ATTRIBUTE_DEVICE),
    file_attribute!(FILE_ATTRIBUTE_DIRECTORY),
    file_attribute!(FILE_ATTRIBUTE_ENCRYPTED),
    file_attribute!(FILE_ATTRIBUTE_HIDDEN),
    file_attribute!(FILE_ATTRIBUTE_INTEGRITY_STREAM),
    file_attribute!(FILE_ATTRIBUTE_NORMAL),
    file_attribute!(FILE_ATTRIBUTE_NOT_CONTENT_INDEXED),
    file_attribute!(FILE_ATTRIBUTE_NO_SCRUB_DATA),
    file_attribute!(FILE_ATTRIBUTE_OFFLINE),
    file_attribute!(FILE_ATTRIBUTE_READONLY),
    file_attribute!(FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS),
    file_attribute!(FILE_ATTRIBUTE_RECALL_ON_OPEN),
    file_attribute!(FILE_ATTRIBUTE_REPARSE_POINT),
    file_attribute!(FILE_ATTRIBUTE_SPARSE_FILE),
    file_attribute!(FILE_ATTRIBUTE_SYSTEM),
    file_attribute!(FILE_ATTRIBUTE_TEMPORARY),
    file_attribute!(FILE_ATTRIBUTE_VIRTUAL),
];

fn display_attributes(attrs: DWORD) -> impl fmt::Display {
    let mut attr_strings = Vec::new();
    for (attr, attr_string) in &ATTRIBUTES {
        if attrs & attr == *attr {
            attr_strings.push(*attr_string);
        }
    }

    attr_strings.join(", ")
}

fn to_u16_vec<T: Into<OsString>>(s: T) -> Vec<u16> {
    s.into()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<u16>>()
}
