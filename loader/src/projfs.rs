#![allow(clippy::mutex_atomic)]

use {
    lazy_static::lazy_static,
    log::{error, trace},
    std::{
        collections::HashMap,
        error,
        ffi::{OsStr, OsString},
        fmt, fs,
        io::{self, Read, Seek},
        iter, mem,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::{Path, PathBuf},
        ptr, slice,
        sync::{Mutex, PoisonError},
    },
    util::{Error, Result},
    winapi_local::{
        shared::{
            basetsd::{UINT32, UINT64},
            guiddef::GUID,
            ntdef::{LARGE_INTEGER, TRUE},
            winerror::{
                ERROR_FILE_NOT_FOUND, ERROR_INSUFFICIENT_BUFFER, E_FAIL, E_OUTOFMEMORY,
                E_UNEXPECTED, FAILED, HRESULT_FROM_WIN32, S_OK,
            },
        },
        um::{
            combaseapi::CoCreateGuid,
            projectedfslib::{
                PRJ_PLACEHOLDER_INFO_s1, PRJ_PLACEHOLDER_INFO_s2, PRJ_PLACEHOLDER_INFO_s3,
                PrjAllocateAlignedBuffer, PrjDoesNameContainWildCards, PrjFileNameCompare,
                PrjFileNameMatch, PrjFillDirEntryBuffer, PrjMarkDirectoryAsPlaceholder,
                PrjStartVirtualizing, PrjStopVirtualizing, PrjWriteFileData,
                PrjWritePlaceholderInfo, PRJ_CALLBACKS, PRJ_CALLBACK_DATA, PRJ_CALLBACK_DATA_FLAGS,
                PRJ_CB_DATA_FLAG_ENUM_RESTART_SCAN, PRJ_DIR_ENTRY_BUFFER_HANDLE,
                PRJ_FILE_BASIC_INFO, PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT,
                PRJ_PLACEHOLDER_ID_LENGTH, PRJ_PLACEHOLDER_INFO, PRJ_PLACEHOLDER_VERSION_INFO,
            },
            winnt::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, HRESULT, PCWSTR},
        },
    },
    zip::read::{ZipArchive, ZipFile},
};

macro_rules! handle_hresult {
    ($r:expr) => {
        let r = $r;
        if FAILED(r) {
            let err = io::Error::from_raw_os_error(r);
            error!("{}", err);
            return Err(err.into());
        }
    };
}

#[derive(Debug)]
struct HresultError(io::Error);
impl fmt::Display for HresultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl error::Error for HresultError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}
impl From<io::Error> for HresultError {
    fn from(err: io::Error) -> Self {
        Self(err)
    }
}
impl<T> From<PoisonError<T>> for HresultError {
    fn from(_err: PoisonError<T>) -> Self {
        Self(io::Error::from_raw_os_error(E_FAIL))
    }
}
impl From<HRESULT> for HresultError {
    fn from(r: HRESULT) -> Self {
        debug_assert!(r != S_OK);
        Self(io::Error::from_raw_os_error(r))
    }
}
impl From<Error> for HresultError {
    fn from(err: Error) -> Self {
        match err {
            Error::Io(err) => Self(err),
            _ => Self(io::Error::from_raw_os_error(E_FAIL)),
        }
    }
}

type HresultResult = std::result::Result<(), HresultError>;
fn report_hresult(r: HresultResult) -> HRESULT {
    match r {
        Ok(()) => S_OK,
        Err(err) => {
            error!("{}", err);
            err.0.raw_os_error().unwrap_or(E_FAIL)
        }
    }
}

struct InstanceHandle(PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT);
unsafe impl Send for InstanceHandle {}
unsafe impl Sync for InstanceHandle {}

pub trait ReadSeek: Read + Seek + Send + Sync {}
impl<T> ReadSeek for T where T: Read + Seek + Send + Sync {}

#[derive(Clone)]
struct EnumerationSession {
    dir_name: OsString,
    search_expression: Option<Option<OsString>>,
    index: usize,
}

struct ProviderState {
    handle: InstanceHandle,
    archive: Option<ZipArchive<Box<dyn ReadSeek>>>,
    enumeration_sessions: HashMap<String, EnumerationSession>,
}

impl ProviderState {
    fn new() -> Self {
        ProviderState {
            handle: InstanceHandle(ptr::null_mut()),
            archive: None,
            enumeration_sessions: HashMap::new(),
        }
    }

    fn file_names(&self) -> impl Iterator<Item = &str> {
        self.archive.as_ref().unwrap().file_names()
    }

    fn get_file(&mut self, name: &str) -> Result<ZipFile> {
        let file = self.archive.as_mut().unwrap().by_name(name)?;
        Ok(file)
    }
}

lazy_static! {
    static ref PROVIDER_STATE: Mutex<ProviderState> = Mutex::new(ProviderState::new());
}

pub struct Provider {
    root: PathBuf,
}

impl Provider {
    pub fn new(virt_root: &Path, archive: ZipArchive<Box<dyn ReadSeek>>) -> Result<Self> {
        trace!("Provider::new");
        let provider = Provider {
            root: virt_root.to_path_buf(),
        };
        fs::create_dir_all(&provider.root)?;

        let instance_id = co_create_guid()?;
        mark_directory_as_placeholder(virt_root, instance_id)?;

        let callbacks = create_callback_table()?;

        let instance_handle = start_virtualizing(virt_root, callbacks)?;

        let mut state = PROVIDER_STATE.lock()?;
        *state = ProviderState::new();
        state.handle = InstanceHandle(instance_handle);
        state.archive = Some(archive);

        trace!("end Provider::new");
        Ok(provider)
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        trace!("Provider::drop");
        match PROVIDER_STATE.lock() {
            Ok(state) => {
                stop_virtualizing(state.handle.0);

                if let Err(err) = fs::remove_dir_all(&self.root) {
                    error!("drop: {}", err);
                }
            }
            Err(err) => error!("drop: {}", err),
        }
        trace!("end Provider::drop");
    }
}

extern "system" fn start_directory_enumeration_cb(
    callback_data: *const PRJ_CALLBACK_DATA,
    enumeration_id: *const GUID,
) -> HRESULT {
    let r = report_hresult(start_directory_enumeration_inner(
        callback_data,
        enumeration_id,
    ));
    trace!("end start_directory_enumeration_cb");
    r
}
extern "system" fn end_directory_enumeration_cb(
    callback_data: *const PRJ_CALLBACK_DATA,
    enumeration_id: *const GUID,
) -> HRESULT {
    let r = report_hresult(end_directory_enumeration_inner(
        callback_data,
        enumeration_id,
    ));
    trace!("end end_directory_enumeration_cb");
    r
}
extern "system" fn get_directory_enumeration_cb(
    callback_data: *const PRJ_CALLBACK_DATA,
    enumeration_id: *const GUID,
    search_expression: PCWSTR,
    dir_entry_buffer_handle: PRJ_DIR_ENTRY_BUFFER_HANDLE,
) -> HRESULT {
    let r = report_hresult(get_directory_enumeration_inner(
        callback_data,
        enumeration_id,
        search_expression,
        dir_entry_buffer_handle,
    ));
    trace!("end get_directory_enumeration_cb");
    r
}
extern "system" fn get_placeholder_info_cb(callback_data: *const PRJ_CALLBACK_DATA) -> HRESULT {
    let r = report_hresult(get_placeholder_info_inner(callback_data));
    trace!("end get_placeholder_info_cb");
    r
}
extern "system" fn get_file_data_cb(
    callback_data: *const PRJ_CALLBACK_DATA,
    byte_offset: UINT64,
    length: UINT32,
) -> HRESULT {
    let r = report_hresult(get_file_data_inner(callback_data, byte_offset, length));
    trace!("end get_file_data_cb");
    r
}

fn start_directory_enumeration_inner(
    callback_data: *const PRJ_CALLBACK_DATA,
    enumeration_id: *const GUID,
) -> HresultResult {
    let requested_path = unsafe { *callback_data }.FilePathName;
    let enumeration_id = format_guid(&unsafe { *enumeration_id });
    trace!(
        "start_directory_enumeration_cb: {:?} {}",
        raw_str_to_os_string(requested_path),
        enumeration_id
    );

    let mut state = PROVIDER_STATE.lock()?;
    let dir_name = {
        if unsafe { *requested_path } == 0 {
            OsString::from("")
        } else {
            let dir = get_file_from_provided_name(&mut state, requested_path)?;
            debug_assert!(dir.is_dir());
            OsString::from(dir.name().to_owned())
        }
    };

    if let Some(_old_value) = state.enumeration_sessions.insert(
        enumeration_id,
        EnumerationSession {
            dir_name,
            search_expression: None,
            index: 0,
        },
    ) {
        error!("We were requested to start an enumeration session with the same ID as one in progress.");
        return Err(E_UNEXPECTED.into());
    }

    Ok(())
}

fn end_directory_enumeration_inner(
    _callback_data: *const PRJ_CALLBACK_DATA,
    enumeration_id: *const GUID,
) -> HresultResult {
    let enumeration_id = format_guid(&unsafe { *enumeration_id });
    trace!("end_directory_enumeration_cb: {}", enumeration_id);

    let mut state = PROVIDER_STATE.lock()?;
    if state.enumeration_sessions.remove(&enumeration_id).is_none() {
        error!(
            "We were requested to end an enumeration session which was not in progress or failed."
        );
        return Err(E_UNEXPECTED.into());
    }

    Ok(())
}

fn get_directory_enumeration_inner(
    callback_data: *const PRJ_CALLBACK_DATA,
    enumeration_id: *const GUID,
    search_expression: PCWSTR,
    dir_entry_buffer_handle: PRJ_DIR_ENTRY_BUFFER_HANDLE,
) -> HresultResult {
    let flags = unsafe { *callback_data }.Flags;
    let enumeration_id = format_guid(&unsafe { *enumeration_id });
    trace!(
        "get_directory_enumeration_cb: {}, {}, {:?}, {:?}",
        flags,
        enumeration_id,
        raw_str_to_os_string(search_expression),
        dir_entry_buffer_handle
    );

    let mut state = PROVIDER_STATE.lock()?;
    match state.enumeration_sessions.remove(&enumeration_id) {
        Some(session) => {
            let (search_expression, mut session) =
                update_search_expression(search_expression, session, flags);

            let matches = get_search_expression_matches(
                search_expression,
                os_str_to_string(&session.dir_name),
                &mut state,
            );

            for (i, (name, normalized)) in matches.into_iter().enumerate().skip(session.index) {
                let file = state.get_file(&name)?;
                let mut basic_info = create_file_basic_info(&file);

                let normalized = to_u16_vec(normalized);
                trace!(
                    "Returning file {:?}.",
                    raw_str_to_os_string(normalized.as_ptr())
                );
                let hr = unsafe {
                    PrjFillDirEntryBuffer(
                        normalized.as_ptr(),
                        &mut basic_info,
                        dir_entry_buffer_handle,
                    )
                };
                session.index = i + 1;
                if hr == HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER) {
                    break;
                } else {
                    handle_hresult!(hr);
                }
            }

            if let Some(_old_value) = state.enumeration_sessions.insert(enumeration_id, session) {
                error!("An enumeration session was somehow added with the same ID.");
                return Err(E_UNEXPECTED.into());
            }

            Ok(())
        }
        None => {
            error!(
                "We were requested to work on an enumeration session which was not in progress."
            );
            Err(E_UNEXPECTED.into())
        }
    }
}

fn get_placeholder_info_inner(callback_data: *const PRJ_CALLBACK_DATA) -> HresultResult {
    let requested_name = unsafe { *callback_data }.FilePathName;
    trace!(
        "get_placeholder_info_cb: {:?}",
        raw_str_to_os_string(requested_name)
    );

    let mut state = PROVIDER_STATE.lock()?;

    let file = get_file_from_provided_name(&mut state, requested_name)?;

    let placeholder_info = create_placeholder_info(file);
    handle_hresult!(unsafe {
        PrjWritePlaceholderInfo(
            state.handle.0,
            requested_name,
            &placeholder_info,
            mem::size_of::<PRJ_PLACEHOLDER_INFO>() as u32,
        )
    });

    Ok(())
}

fn get_file_data_inner(
    callback_data: *const PRJ_CALLBACK_DATA,
    byte_offset: UINT64,
    length: UINT32,
) -> HresultResult {
    let requested_name = unsafe { *callback_data }.FilePathName;
    let data_stream_id = unsafe { *callback_data }.DataStreamId;
    trace!(
        "get_file_data_cb: {:?}, {}, {}, {}",
        raw_str_to_os_string(requested_name),
        format_guid(&data_stream_id),
        byte_offset,
        length
    );

    let mut state = PROVIDER_STATE.lock()?;
    let handle = state.handle.0;

    let mut file = get_file_from_provided_name(&mut state, requested_name)?;

    let buffer = unsafe { PrjAllocateAlignedBuffer(handle, file.size() as usize) };
    if buffer.is_null() {
        return Err(E_OUTOFMEMORY.into());
    }

    // Always return the whole file, since we don't have the ability to seek
    // within a zip file.
    let mut slice_buffer =
        unsafe { slice::from_raw_parts_mut(buffer as *mut u8, file.size() as usize) };
    io::copy(&mut file, &mut slice_buffer)?;

    handle_hresult!(unsafe {
        PrjWriteFileData(
            handle,
            &data_stream_id,
            buffer,
            0, // byteOffset
            file.size() as u32,
        )
    });

    Ok(())
}

fn co_create_guid() -> Result<GUID> {
    let mut guid = GUID {
        Data1: 0,
        Data2: 0,
        Data3: 0,
        Data4: [0; 8],
    };
    handle_hresult!(unsafe { CoCreateGuid(&mut guid) });
    Ok(guid)
}

fn mark_directory_as_placeholder(
    root_path_name: &Path,
    virtualization_instance_id: GUID,
) -> Result<()> {
    handle_hresult!(unsafe {
        PrjMarkDirectoryAsPlaceholder(
            to_u16_vec(root_path_name).as_ptr(),
            ptr::null(), // targetPathName
            ptr::null(), // versionInfo
            &virtualization_instance_id,
        )
    });
    Ok(())
}

fn start_virtualizing(
    virtualization_root_path: &Path,
    callbacks: PRJ_CALLBACKS,
) -> Result<PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT> {
    let mut instance_handle = ptr::null_mut();
    handle_hresult!(unsafe {
        PrjStartVirtualizing(
            to_u16_vec(virtualization_root_path).as_ptr(),
            &callbacks,
            ptr::null(), // instanceContext
            ptr::null(), // options
            &mut instance_handle,
        )
    });
    Ok(instance_handle)
}

fn stop_virtualizing(namespace_virtualization_context: PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT) {
    unsafe { PrjStopVirtualizing(namespace_virtualization_context) };
}

fn get_file_from_provided_name(
    state: &mut ProviderState,
    requested_name: PCWSTR,
) -> std::result::Result<ZipFile, HresultError> {
    let name = state
        .file_names()
        .find(|n| {
            let n = n.replace("/", "\\").trim_end_matches('\\').to_owned();
            let n = to_u16_vec(n);
            let result = unsafe { PrjFileNameCompare(n.as_ptr(), requested_name) };
            result == 0
        })
        .map(|n| n.to_owned());

    match name {
        Some(name) => {
            let file = state.get_file(&name)?;
            Ok(file)
        }
        None => Err(HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND).into()),
    }
}

fn update_search_expression(
    provided_search_expression: PCWSTR,
    mut session: EnumerationSession,
    flags: PRJ_CALLBACK_DATA_FLAGS,
) -> (Option<OsString>, EnumerationSession) {
    let search_expression = match session.search_expression {
        Some(opt_expr) => {
            if flags & PRJ_CB_DATA_FLAG_ENUM_RESTART_SCAN == PRJ_CB_DATA_FLAG_ENUM_RESTART_SCAN {
                session.index = 0;
                ptr_str_to_option(provided_search_expression)
            } else {
                opt_expr
            }
        }
        None => ptr_str_to_option(provided_search_expression),
    };
    session.search_expression = Some(search_expression.clone());

    (search_expression, session)
}

fn get_search_expression_matches(
    search_expression: Option<OsString>,
    dir_name: String,
    state: &mut ProviderState,
) -> Vec<(String, String)> {
    let file_names_in_directory = state
        .file_names()
        .filter(|n| is_in_directory(n, &dir_name))
        .map(|n| (n.to_owned(), normalize_dir_entry(n)));
    let mut matches: Vec<(String, String)> = match search_expression {
        Some(expr) => {
            let expr_vec = to_u16_vec(&expr);
            if unsafe { PrjDoesNameContainWildCards(expr_vec.as_ptr()) } == TRUE {
                trace!("Search expression {:?} contains wildcards.", &expr);
                file_names_in_directory
                    .filter(|(_, normalized)| {
                        let n_vec = to_u16_vec(&normalized);
                        let result = unsafe { PrjFileNameMatch(n_vec.as_ptr(), expr_vec.as_ptr()) };
                        result == TRUE
                    })
                    .collect()
            } else {
                file_names_in_directory
                    .filter(|(_, normalized)| {
                        let n_vec = to_u16_vec(&normalized);
                        let expr_vec = to_u16_vec(&expr);
                        let result =
                            unsafe { PrjFileNameCompare(n_vec.as_ptr(), expr_vec.as_ptr()) };
                        result == 0
                    })
                    .collect()
            }
        }
        None => file_names_in_directory.collect(),
    };
    matches.sort_by(|(_, n1), (_, n2)| {
        let n1_vec = to_u16_vec(n1);
        let n2_vec = to_u16_vec(n2);
        let result = unsafe { PrjFileNameCompare(n1_vec.as_ptr(), n2_vec.as_ptr()) };
        result.cmp(&0)
    });

    matches
}

fn normalize_dir_entry(n: &str) -> String {
    PathBuf::from(n)
        .file_name()
        .unwrap_or(&OsString::new())
        .to_string_lossy()
        .into_owned()
}

fn is_in_directory(n: &str, dir: &str) -> bool {
    let path = PathBuf::from(n);
    let dir = PathBuf::from(dir);
    match path.strip_prefix(dir) {
        Ok(pref) => path.file_name().unwrap_or(&OsString::new()) == pref,
        Err(_) => false,
    }
}

fn create_callback_table() -> Result<PRJ_CALLBACKS> {
    let callbacks = PRJ_CALLBACKS {
        StartDirectoryEnumerationCallback: Some(start_directory_enumeration_cb),
        EndDirectoryEnumerationCallback: Some(end_directory_enumeration_cb),
        GetDirectoryEnumerationCallback: Some(get_directory_enumeration_cb),
        GetPlaceholderInfoCallback: Some(get_placeholder_info_cb),
        GetFileDataCallback: Some(get_file_data_cb),

        // these are optional
        QueryFileNameCallback: None,
        NotificationCallback: None,
        CancelCommandCallback: None,
    };

    Ok(callbacks)
}

fn create_placeholder_info(file: ZipFile) -> PRJ_PLACEHOLDER_INFO {
    let basic_info = create_file_basic_info(&file);

    PRJ_PLACEHOLDER_INFO {
        FileBasicInfo: basic_info,
        EaInformation: PRJ_PLACEHOLDER_INFO_s1 {
            EaBufferSize: 0,
            OffsetToFirstEa: 0,
        },
        SecurityInformation: PRJ_PLACEHOLDER_INFO_s2 {
            SecurityBufferSize: 0,
            OffsetToSecurityDescriptor: 0,
        },
        StreamsInformation: PRJ_PLACEHOLDER_INFO_s3 {
            StreamsInfoBufferSize: 0,
            OffsetToFirstStreamInfo: 0,
        },
        VersionInfo: PRJ_PLACEHOLDER_VERSION_INFO {
            ProviderID: [0; PRJ_PLACEHOLDER_ID_LENGTH as usize],
            ContentID: [0; PRJ_PLACEHOLDER_ID_LENGTH as usize],
        },
        VariableData: [0; 1],
    }
}

fn create_file_basic_info(file: &ZipFile) -> PRJ_FILE_BASIC_INFO {
    let attrs = if file.is_dir() {
        FILE_ATTRIBUTE_DIRECTORY
    } else {
        // TODO: Find a way for this to be FILE_ATTRIBUTE_READONLY and still be
        // able to delete the directory contents.
        FILE_ATTRIBUTE_NORMAL
    };

    let large_int_zero = unsafe { mem::zeroed::<LARGE_INTEGER>() };
    PRJ_FILE_BASIC_INFO {
        IsDirectory: file.is_dir().into(),
        FileSize: file.size() as i64,
        CreationTime: large_int_zero,
        LastAccessTime: large_int_zero,
        LastWriteTime: large_int_zero,
        ChangeTime: large_int_zero,
        FileAttributes: attrs,
    }
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

fn ptr_str_to_option(p: *const u16) -> Option<OsString> {
    if p.is_null() {
        None
    } else {
        Some(raw_str_to_os_string(p))
    }
}

fn os_str_to_string(s: &OsStr) -> String {
    s.to_string_lossy().into()
}

fn format_guid(g: &GUID) -> String {
    format!(
        "{:x}-{:x}-{:x}-{:x}{:x}-{:x}{:x}{:x}{:x}{:x}{:x}",
        g.Data1,
        g.Data2,
        g.Data3,
        g.Data4[0],
        g.Data4[1],
        g.Data4[2],
        g.Data4[3],
        g.Data4[4],
        g.Data4[5],
        g.Data4[6],
        g.Data4[7]
    )
}

#[cfg(test)]
mod testing {
    use super::*;

    #[test]
    fn is_in_directory_test() {
        assert!(is_in_directory(r#"a"#, r#""#));
        assert!(is_in_directory(r#"a\"#, r#""#));
        assert!(!is_in_directory(r#"a\b"#, r#""#));
        assert!(!is_in_directory(r#"a\b\"#, r#""#));

        assert!(!is_in_directory(r#"a"#, r#"a"#));
        assert!(!is_in_directory(r#"a\"#, r#"a"#));
        assert!(!is_in_directory(r#"a"#, r#"a\"#));
        assert!(!is_in_directory(r#"a\"#, r#"a\"#));

        assert!(is_in_directory(r#"a\b"#, r#"a"#));
        assert!(is_in_directory(r#"a\b\"#, r#"a"#));
        assert!(is_in_directory(r#"a\b"#, r#"a\"#));
        assert!(is_in_directory(r#"a\b\"#, r#"a\"#));
        assert!(!is_in_directory(r#"a\b\c"#, r#"a"#));
        assert!(!is_in_directory(r#"a\b\c\"#, r#"a"#));
        assert!(!is_in_directory(r#"a\b\c"#, r#"a\"#));
        assert!(!is_in_directory(r#"a\b\c\"#, r#"a\"#));

        assert!(!is_in_directory(r#"a\b"#, r#"a\b"#));
        assert!(!is_in_directory(r#"a\b\"#, r#"a\b"#));
        assert!(!is_in_directory(r#"a\b"#, r#"a\b\"#));
        assert!(!is_in_directory(r#"a\b\"#, r#"a\b\"#));
        assert!(!is_in_directory(r#"a"#, r#"a\b"#));
        assert!(!is_in_directory(r#"a\"#, r#"a\b"#));
        assert!(!is_in_directory(r#"a"#, r#"a\b\"#));
        assert!(!is_in_directory(r#"a\"#, r#"a\b\"#));

        assert!(is_in_directory(r#"a\b\c"#, r#"a\b"#));
        assert!(is_in_directory(r#"a\b\c\"#, r#"a\b"#));
        assert!(is_in_directory(r#"a\b\c"#, r#"a\b\"#));
        assert!(is_in_directory(r#"a\b\c\"#, r#"a\b\"#));
        assert!(!is_in_directory(r#"a\b\c\d"#, r#"a\b"#));
        assert!(!is_in_directory(r#"a\b\c\d\"#, r#"a\b"#));
        assert!(!is_in_directory(r#"a\b\c\d"#, r#"a\b\"#));
        assert!(!is_in_directory(r#"a\b\c\d\"#, r#"a\b\"#));
    }
}
