use {
    log::error,
    std::{io, mem, path::PathBuf, ptr},
    util::{to_u16_vec, Result},
    winapi::{
        ctypes::c_void,
        shared::ntdef::{FALSE, TRUE},
        um::{
            handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
            ioapiset::{CreateIoCompletionPort, GetQueuedCompletionStatus},
            jobapi2::{AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject},
            processthreadsapi::{
                CreateProcessW, GetExitCodeProcess, ResumeThread, PROCESS_INFORMATION, STARTUPINFOW,
            },
            winbase::{CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, INFINITE},
            winnt::{
                JobObjectAssociateCompletionPortInformation, HANDLE,
                JOBOBJECT_ASSOCIATE_COMPLETION_PORT, JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO,
            },
        },
    },
};

macro_rules! handle_get_last_error {
    ($e:expr) => {{
        let handle = $e;
        if handle.is_null() {
            let err = io::Error::last_os_error();
            error!("{}", err);
            return Err(err.into());
        }
        handle
    }};
}

macro_rules! bool_get_last_error {
    ($e:expr) => {{
        let b = $e;
        if b == FALSE.into() {
            let err = io::Error::last_os_error();
            error!("{}", err);
            return Err(err.into());
        }
    }};
}

macro_rules! dword_get_last_error {
    ($e:expr) => {{
        let d = $e;
        if d == 1_u32.wrapping_neg() {
            let err = io::Error::last_os_error();
            error!("{}", err);
            return Err(err.into());
        }
    }};
}

pub struct WaitableJob {
    job: HANDLE,
    port: HANDLE,
    process: HANDLE,
}
impl WaitableJob {
    pub fn wait(self) -> Result<u32> {
        while !self.has_exited()? {}

        let mut exit_code = 0;
        bool_get_last_error! { unsafe {
            GetExitCodeProcess(self.process, &mut exit_code)
        }};

        Ok(exit_code)
    }

    fn has_exited(&self) -> Result<bool> {
        let mut completion_code = 0;
        let mut completion_key = 0;
        let mut overlapped = ptr::null_mut();
        bool_get_last_error! { unsafe { GetQueuedCompletionStatus(
            self.port,
            &mut completion_code,
            &mut completion_key,
            &mut overlapped,
            INFINITE // dwMilliseconds,
        )}}

        Ok(completion_key as HANDLE == self.job
            && completion_code == JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO)
    }
}
impl Drop for WaitableJob {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.job) };
        unsafe { CloseHandle(self.port) };
        unsafe { CloseHandle(self.process) };
    }
}

/// Based on https://devblogs.microsoft.com/oldnewthing/20130405-00/?p=4743
pub fn create_process_in_job_object(exe_file: PathBuf, args: Vec<String>) -> Result<WaitableJob> {
    let job = handle_get_last_error! { unsafe {
        CreateJobObjectW(
            ptr::null_mut(), // lpJobAttributes
            ptr::null(),     // lpName
        )
    }};

    let io_port = handle_get_last_error! { unsafe {
        CreateIoCompletionPort(
            INVALID_HANDLE_VALUE, // FileHandle
            ptr::null_mut(),      // ExistingCompletionPort
            0,                    // CompletionKey
            1,                    // NumberOfConcurrentThreads
        )
    }};

    let mut port = JOBOBJECT_ASSOCIATE_COMPLETION_PORT {
        CompletionKey: job,
        CompletionPort: io_port,
    };
    bool_get_last_error! { unsafe {
        SetInformationJobObject(
            job,
            JobObjectAssociateCompletionPortInformation,
            (&mut port as *mut JOBOBJECT_ASSOCIATE_COMPLETION_PORT).cast::<c_void>(),
            mem::size_of::<JOBOBJECT_ASSOCIATE_COMPLETION_PORT>() as u32,
        )
    }};

    let command_line = format!(
        "{} {}",
        (*exe_file.to_string_lossy()).to_owned(),
        &args.join(" ")
    );

    let mut startup_info = unsafe { mem::zeroed::<STARTUPINFOW>() };
    startup_info.cb = mem::size_of::<STARTUPINFOW>() as u32;

    let mut process_info = unsafe { mem::zeroed::<PROCESS_INFORMATION>() };

    bool_get_last_error! { unsafe {
        CreateProcessW(
            ptr::null(),     // lpApplicationName
            to_u16_vec(command_line).as_mut_ptr(),
            ptr::null_mut(), // lpProcessAttributes
            ptr::null_mut(), // lpThreadAttributes
            TRUE.into(),     // bInheritHandles
            CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
            ptr::null_mut(), // lpEnvironment
            ptr::null(),     // lpCurrentDirectory
            &mut startup_info,
            &mut process_info,
        )
    }};

    bool_get_last_error! { unsafe {
        AssignProcessToJobObject(job, process_info.hProcess)
    }};
    dword_get_last_error! { unsafe { ResumeThread(process_info.hThread)}};
    bool_get_last_error! { unsafe { CloseHandle(process_info.hThread)}};

    Ok(WaitableJob {
        job,
        port: io_port,
        process: process_info.hProcess,
    })
}
