pub mod projfs;

use {
    std::{
        env,
        ffi::OsString,
        io, iter,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::PathBuf,
        slice,
    },
    util::Result,
    winapi_local::{shared::minwindef::MAX_PATH, um::fileapi::GetLongPathNameW},
};

pub fn get_temp_dir() -> Result<PathBuf> {
    let potentially_short_path = env::temp_dir();

    let mut long_path_name = [0; MAX_PATH];
    let result = unsafe {
        GetLongPathNameW(
            to_u16_vec(potentially_short_path).as_ptr(),
            long_path_name.as_mut_ptr(),
            MAX_PATH as u32,
        )
    };
    if result == 0 {
        return Err(io::Error::last_os_error().into());
    }

    Ok(raw_str_to_os_string(long_path_name.as_ptr()).into())
}

fn to_u16_vec<T: Into<OsString>>(s: T) -> Vec<u16> {
    s.into()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<u16>>()
}

fn raw_str_to_os_string(s: *const u16) -> OsString {
    let len = get_raw_str_length(s);
    let slice = unsafe { slice::from_raw_parts(s, len) };
    OsString::from_wide(slice)
}

fn get_raw_str_length(s: *const u16) -> usize {
    let mut i = 0;
    while unsafe { *s.offset(i) } != 0 {
        i += 1;
    }
    i as usize
}
