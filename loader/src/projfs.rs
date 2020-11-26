use {
    lazy_static::lazy_static,
    log::{error, trace},
    std::{
        error,
        ffi::OsString,
        fmt, io, iter, mem,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::Path,
        ptr, slice,
        sync::{Mutex, PoisonError},
    },
    util::{Error, OffsetSeeker, Result},
    winapi_local::{
        shared::{
            basetsd::{UINT32, UINT64},
            guiddef::GUID,
            ntdef::LARGE_INTEGER,
            winerror::{
                ERROR_FILE_NOT_FOUND, E_FAIL, E_OUTOFMEMORY, FAILED, HRESULT_FROM_WIN32, S_OK,
            },
        },
        um::{
            combaseapi::CoCreateGuid,
            projectedfslib::{
                PRJ_PLACEHOLDER_INFO_s1, PRJ_PLACEHOLDER_INFO_s2, PRJ_PLACEHOLDER_INFO_s3,
                PrjAllocateAlignedBuffer, PrjFileNameCompare, PrjMarkDirectoryAsPlaceholder,
                PrjStartVirtualizing, PrjStopVirtualizing, PrjWriteFileData,
                PrjWritePlaceholderInfo, PRJ_CALLBACKS, PRJ_CALLBACK_DATA,
                PRJ_DIR_ENTRY_BUFFER_HANDLE, PRJ_FILE_BASIC_INFO,
                PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT, PRJ_PLACEHOLDER_ID_LENGTH,
                PRJ_PLACEHOLDER_INFO, PRJ_PLACEHOLDER_VERSION_INFO,
            },
            winnt::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY, HRESULT, PCWSTR},
        },
    },
    zip::read::{ZipArchive, ZipFile},
};

macro_rules! handle_hresult {
    ($r:expr) => {
        let r = unsafe { $r };
        if FAILED(r) {
            return Err(io::Error::from_raw_os_error(r).into());
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

struct ProviderState {
    handle: InstanceHandle,
    archive: Option<ZipArchive<OffsetSeeker>>,
}

impl ProviderState {
    fn new() -> Self {
        ProviderState {
            handle: InstanceHandle(ptr::null_mut()),
            archive: None,
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

pub struct Provider;

impl Provider {
    pub fn new(virt_root: &Path, archive: ZipArchive<OffsetSeeker>) -> Result<Self> {
        let instance_id = co_create_guid()?;
        mark_directory_as_placeholder(virt_root, instance_id)?;

        let callbacks = create_callback_table()?;

        let instance_handle = start_virtualizing(virt_root, callbacks)?;

        let mut state = PROVIDER_STATE.lock()?;
        state.handle = InstanceHandle(instance_handle);
        state.archive = Some(archive);
        Ok(Provider {})
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        if let Ok(state) = PROVIDER_STATE.lock() {
            stop_virtualizing(state.handle.0)
        }
    }
}

extern "system" fn start_directory_enumeration_cb(
    callback_data: *const PRJ_CALLBACK_DATA,
    enumeration_id: *const GUID,
) -> HRESULT {
    report_hresult(start_directory_enumeration_inner(
        callback_data,
        enumeration_id,
    ))
}
extern "system" fn end_directory_enumeration_cb(
    callback_data: *const PRJ_CALLBACK_DATA,
    enumeration_id: *const GUID,
) -> HRESULT {
    report_hresult(end_directory_enumeration_inner(
        callback_data,
        enumeration_id,
    ))
}
extern "system" fn get_directory_enumeration_cb(
    callback_data: *const PRJ_CALLBACK_DATA,
    enumeration_id: *const GUID,
    search_expression: PCWSTR,
    dir_entry_buffer_handle: PRJ_DIR_ENTRY_BUFFER_HANDLE,
) -> HRESULT {
    report_hresult(get_directory_enumeration_inner(
        callback_data,
        enumeration_id,
        search_expression,
        dir_entry_buffer_handle,
    ))
}
extern "system" fn get_placeholder_info_cb(callback_data: *const PRJ_CALLBACK_DATA) -> HRESULT {
    report_hresult(get_placeholder_info_inner(callback_data))
}
extern "system" fn get_file_data_cb(
    callback_data: *const PRJ_CALLBACK_DATA,
    byte_offset: UINT64,
    length: UINT32,
) -> HRESULT {
    report_hresult(get_file_data_inner(callback_data, byte_offset, length))
}

fn start_directory_enumeration_inner(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _enumeration_id: *const GUID,
) -> HresultResult {
    trace!("start_directory_enumeration_cb");
    Ok(())
}

fn end_directory_enumeration_inner(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _enumeration_id: *const GUID,
) -> HresultResult {
    trace!("end_directory_enumeration_cb");
    Ok(())
}

fn get_directory_enumeration_inner(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _enumeration_id: *const GUID,
    _search_expression: PCWSTR,
    _dir_entry_buffer_handle: PRJ_DIR_ENTRY_BUFFER_HANDLE,
) -> HresultResult {
    trace!("get_directory_enumeration_cb");
    Ok(())
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
    handle_hresult!(PrjWritePlaceholderInfo(
        state.handle.0,
        requested_name,
        &placeholder_info,
        mem::size_of::<PRJ_PLACEHOLDER_INFO>() as u32
    ));

    Ok(())
}

fn get_file_data_inner(
    callback_data: *const PRJ_CALLBACK_DATA,
    _byte_offset: UINT64,
    _length: UINT32,
) -> HresultResult {
    let requested_name = unsafe { *callback_data }.FilePathName;
    let data_stream_id = unsafe { *callback_data }.DataStreamId;
    trace!(
        "get_file_data_cb: {:?}, {}",
        raw_str_to_os_string(requested_name),
        format_guid(&data_stream_id)
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

    handle_hresult!(PrjWriteFileData(
        handle,
        &data_stream_id,
        buffer,
        0, // byteOffset
        file.size() as u32,
    ));

    Ok(())
}

fn co_create_guid() -> Result<GUID> {
    let mut guid = GUID {
        Data1: 0,
        Data2: 0,
        Data3: 0,
        Data4: [0; 8],
    };
    handle_hresult!(CoCreateGuid(&mut guid));
    Ok(guid)
}

fn mark_directory_as_placeholder(
    root_path_name: &Path,
    virtualization_instance_id: GUID,
) -> Result<()> {
    handle_hresult!(PrjMarkDirectoryAsPlaceholder(
        to_u16_vec(root_path_name).as_ptr(),
        ptr::null(), // targetPathName
        ptr::null(), // versionInfo
        &virtualization_instance_id
    ));
    Ok(())
}

fn start_virtualizing(
    virtualization_root_path: &Path,
    callbacks: PRJ_CALLBACKS,
) -> Result<PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT> {
    let mut instance_handle = ptr::null_mut();
    handle_hresult!(PrjStartVirtualizing(
        to_u16_vec(virtualization_root_path).as_ptr(),
        &callbacks,
        ptr::null(), // instanceContext
        ptr::null(), // options
        &mut instance_handle
    ));
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
        .find(|name| {
            let native_name = to_u16_vec(name);
            let result = unsafe { PrjFileNameCompare(native_name.as_ptr(), requested_name) };
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
        FILE_ATTRIBUTE_READONLY
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
