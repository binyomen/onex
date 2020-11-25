use {
    lazy_static::lazy_static,
    log::{error, trace},
    std::{
        ffi::OsString,
        io,
        iter::once,
        os::windows::ffi::OsStrExt,
        path::Path,
        ptr::{null, null_mut},
        sync::Mutex,
    },
    util::{OffsetSeeker, Result},
    winapi_local::{
        shared::{
            basetsd::{UINT32, UINT64},
            guiddef::GUID,
            winerror::{E_FAIL, FAILED, S_OK},
        },
        um::{
            combaseapi::CoCreateGuid,
            projectedfslib::{
                PrjMarkDirectoryAsPlaceholder, PrjStartVirtualizing, PrjStopVirtualizing,
                PRJ_CALLBACKS, PRJ_CALLBACK_DATA, PRJ_DIR_ENTRY_BUFFER_HANDLE,
                PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT,
            },
            winnt::{HRESULT, PCWSTR},
        },
    },
    zip::ZipArchive,
};

macro_rules! handle_hresult {
    ($r:expr) => {
        let r = unsafe { $r };
        if FAILED(r) {
            return Err(io::Error::from_raw_os_error(r).into());
        }
    };
}

macro_rules! io_result_to_hresult {
    ($r:expr) => {{
        let r = $r;
        match r {
            Ok(()) => S_OK,
            Err(err) => {
                error!("{}", err);
                err.raw_os_error().unwrap_or(E_FAIL)
            }
        }
    }};
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
            handle: InstanceHandle(null_mut()),
            archive: None,
        }
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
    trace!("start_directory_enumeration_cb");
    io_result_to_hresult!(start_directory_enumeration_inner(
        callback_data,
        enumeration_id
    ))
}
extern "system" fn end_directory_enumeration_cb(
    callback_data: *const PRJ_CALLBACK_DATA,
    enumeration_id: *const GUID,
) -> HRESULT {
    trace!("end_directory_enumeration_cb");
    io_result_to_hresult!(end_directory_enumeration_inner(
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
    trace!("get_directory_enumeration_cb");
    io_result_to_hresult!(get_directory_enumeration_inner(
        callback_data,
        enumeration_id,
        search_expression,
        dir_entry_buffer_handle
    ))
}
extern "system" fn get_placeholder_info_cb(callback_data: *const PRJ_CALLBACK_DATA) -> HRESULT {
    trace!("get_placeholder_info_cb");
    io_result_to_hresult!(get_placeholder_info_inner(callback_data))
}
extern "system" fn get_file_data_cb(
    callback_data: *const PRJ_CALLBACK_DATA,
    byte_offset: UINT64,
    length: UINT32,
) -> HRESULT {
    trace!("get_file_data_cb");
    io_result_to_hresult!(get_file_data_inner(callback_data, byte_offset, length))
}

fn start_directory_enumeration_inner(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _enumeration_id: *const GUID,
) -> io::Result<()> {
    Ok(())
}

fn end_directory_enumeration_inner(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _enumeration_id: *const GUID,
) -> io::Result<()> {
    Ok(())
}

fn get_directory_enumeration_inner(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _enumeration_id: *const GUID,
    _search_expression: PCWSTR,
    _dir_entry_buffer_handle: PRJ_DIR_ENTRY_BUFFER_HANDLE,
) -> io::Result<()> {
    Ok(())
}

fn get_placeholder_info_inner(_callback_data: *const PRJ_CALLBACK_DATA) -> io::Result<()> {
    Ok(())
}

fn get_file_data_inner(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _byte_offset: UINT64,
    _length: UINT32,
) -> io::Result<()> {
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
        path_to_u16_vec(root_path_name).as_ptr(),
        null(),
        null(),
        &virtualization_instance_id
    ));
    Ok(())
}

fn start_virtualizing(
    virtualization_root_path: &Path,
    callbacks: PRJ_CALLBACKS,
) -> Result<PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT> {
    let mut instance_handle = null_mut();
    handle_hresult!(PrjStartVirtualizing(
        path_to_u16_vec(virtualization_root_path).as_ptr(),
        &callbacks,
        null(),
        null(),
        &mut instance_handle
    ));
    Ok(instance_handle)
}

fn stop_virtualizing(namespace_virtualization_context: PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT) {
    unsafe { PrjStopVirtualizing(namespace_virtualization_context) };
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

fn path_to_u16_vec(p: &Path) -> Vec<u16> {
    OsString::from(p)
        .encode_wide()
        .chain(once(0))
        .collect::<Vec<u16>>()
}
