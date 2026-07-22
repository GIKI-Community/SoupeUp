//! Windows Job Object helpers — kill all descendants when the job handle closes.

#![cfg(windows)]

use std::ptr;

type HANDLE = *mut std::ffi::c_void;
type BOOL = i32;
type DWORD = u32;

const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x0000_2000;
const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: u32 = 9;
const PROCESS_SET_QUOTA: DWORD = 0x0100;
const PROCESS_TERMINATE: DWORD = 0x0001;
const PROCESS_ASSIGN_ACCESS: DWORD = PROCESS_SET_QUOTA | PROCESS_TERMINATE;

#[repr(C)]
struct JoSecurityAttributes {
    n_length: DWORD,
    lp_security_descriptor: *mut std::ffi::c_void,
    b_inherit_handle: BOOL,
}

#[repr(C)]
struct JoBasicLimitInformation {
    per_process_user_time_limit: i64,
    per_job_user_time_limit: i64,
    limit_flags: DWORD,
    minimum_working_set_size: usize,
    maximum_working_set_size: usize,
    active_process_limit: DWORD,
    affinity: usize,
    priority_class: DWORD,
    scheduling_class: DWORD,
}

#[repr(C)]
struct IoCounters {
    read_operation_count: u64,
    write_operation_count: u64,
    other_operation_count: u64,
    read_transfer_count: u64,
    write_transfer_count: u64,
    other_transfer_count: u64,
}

#[repr(C)]
struct JoExtendedLimitInformation {
    basic_limit_information: JoBasicLimitInformation,
    io_info: IoCounters,
    process_memory_limit: usize,
    job_memory_limit: usize,
    peak_process_memory_used: usize,
    peak_job_memory_used: usize,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateJobObjectW(
        lp_job_attributes: *const JoSecurityAttributes,
        lp_name: *const u16,
    ) -> HANDLE;

    fn SetInformationJobObject(
        h_job: HANDLE,
        job_object_information_class: u32,
        lp_job_object_information: *const std::ffi::c_void,
        cb_job_object_information_length: DWORD,
    ) -> BOOL;

    fn AssignProcessToJobObject(h_job: HANDLE, h_process: HANDLE) -> BOOL;

    fn OpenProcess(dw_desired_access: DWORD, b_inherit_handle: BOOL, dw_process_id: DWORD)
        -> HANDLE;

    fn CloseHandle(h_object: HANDLE) -> BOOL;

    fn GetLastError() -> DWORD;
}

/// A Job Object that kills all assigned processes when dropped.
pub struct JobObject {
    handle: HANDLE,
}

unsafe impl Send for JobObject {}
unsafe impl Sync for JobObject {}

impl JobObject {
    /// Create a job with `KILL_ON_JOB_CLOSE` and assign the process with `pid` to it.
    pub fn for_pid(pid: u32) -> Option<Self> {
        unsafe {
            let process = OpenProcess(PROCESS_ASSIGN_ACCESS, 0, pid);
            if process.is_null() {
                log::warn!(
                    "OpenProcess({}) failed for job assignment (error={})",
                    pid,
                    GetLastError()
                );
                return None;
            }

            let handle = CreateJobObjectW(ptr::null(), ptr::null());
            if handle.is_null() {
                log::warn!("CreateJobObjectW failed (error={})", GetLastError());
                CloseHandle(process);
                return None;
            }

            let mut info: JoExtendedLimitInformation = std::mem::zeroed();
            info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let ok = SetInformationJobObject(
                handle,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                &info as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JoExtendedLimitInformation>() as DWORD,
            );
            if ok == 0 {
                log::warn!(
                    "SetInformationJobObject failed (error={})",
                    GetLastError()
                );
                CloseHandle(handle);
                CloseHandle(process);
                return None;
            }

            let ok = AssignProcessToJobObject(handle, process);
            CloseHandle(process);
            if ok == 0 {
                log::warn!(
                    "AssignProcessToJobObject failed (error={}) — process may break away",
                    GetLastError()
                );
                CloseHandle(handle);
                return None;
            }

            Some(Self { handle })
        }
    }

    /// Explicitly close the job (terminates all processes in it).
    pub fn terminate(self) {
        drop(self);
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                CloseHandle(self.handle);
            }
            self.handle = ptr::null_mut();
        }
    }
}

/// Force-kill leftover Ray daemon processes by image name (last-resort sweep).
pub fn kill_orphaned_ray_processes() {
    use std::os::windows::process::CommandExt;
    use std::process::Stdio;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const IMAGES: &[&str] = &[
        "gcs_server.exe",
        "raylet.exe",
        "plasma_store_server.exe",
        "ray_dashboard.exe",
    ];

    for image in IMAGES {
        let _ = std::process::Command::new("taskkill")
            .args(["/IM", image, "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}
