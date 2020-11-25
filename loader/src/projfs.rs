use {
    std::{
        ffi::OsString,
        io::{self, Read, Seek},
        iter::once,
        os::windows::ffi::OsStrExt,
        path::Path,
        ptr::{null, null_mut},
    },
    util::Result,
    winapi_local::{
        shared::{
            basetsd::{UINT32, UINT64},
            guiddef::GUID,
            winerror::{FAILED, S_OK},
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

extern "system" fn start_directory_enumeration_cb(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _enumeration_id: *const GUID,
) -> HRESULT {
    S_OK
}

extern "system" fn end_directory_enumeration_cb(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _enumeration_id: *const GUID,
) -> HRESULT {
    S_OK
}

extern "system" fn get_directory_enumeration_cb(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _enumeration_id: *const GUID,
    _search_expression: PCWSTR,
    _dir_entry_buffer_handle: PRJ_DIR_ENTRY_BUFFER_HANDLE,
) -> HRESULT {
    S_OK
}

extern "system" fn get_placeholder_info_cb(_callback_data: *const PRJ_CALLBACK_DATA) -> HRESULT {
    S_OK
}

extern "system" fn get_file_data_cb(
    _callback_data: *const PRJ_CALLBACK_DATA,
    _byte_offset: UINT64,
    _length: UINT32,
) -> HRESULT {
    S_OK
}

pub fn initialize<R: Read + Seek>(
    virt_root: &Path,
    _archive: ZipArchive<R>,
) -> Result<PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT> {
    let instance_id = co_create_guid()?;
    mark_directory_as_placeholder(virt_root, instance_id)?;

    let callbacks = create_callback_table()?;

    let instance_handle = start_virtualizing(virt_root, callbacks)?;

    Ok(instance_handle)
}

pub fn shut_down(instance_handle: PRJ_NAMESPACE_VIRTUALIZATION_CONTEXT) {
    stop_virtualizing(instance_handle)
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
