//! The few Windows API functions we need.

use std::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    pub fn GetModuleFileNameW(module: *mut c_void, buf: *mut u16, size: u32) -> u32;
    pub fn VirtualAlloc(addr: *mut c_void, size: usize, kind: u32, prot: u32) -> *mut c_void;
    pub fn VirtualFree(addr: *mut c_void, size: usize, kind: u32) -> i32;
    pub fn VirtualProtect(addr: *mut c_void, size: usize, prot: u32, old: *mut u32) -> i32;
    pub fn FlushInstructionCache(process: *mut c_void, addr: *const c_void, size: usize) -> i32;
    pub fn GetCurrentProcess() -> *mut c_void;
    pub fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

pub const MEM_COMMIT_RESERVE: u32 = 0x3000;
pub const MEM_RELEASE: u32 = 0x8000;
pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;

#[link(name = "user32")]
extern "system" {
    pub fn GetAsyncKeyState(vk: i32) -> i16;
    pub fn GetForegroundWindow() -> *mut c_void;
    pub fn GetWindowThreadProcessId(window: *mut c_void, pid: *mut u32) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetCurrentProcessId() -> u32;
}

/// `STARTUPINFOW` (x64 layout).
#[repr(C)]
pub struct StartupInfoW {
    pub cb: u32,
    pub reserved: *mut u16,
    pub desktop: *mut u16,
    pub title: *mut u16,
    pub x: u32,
    pub y: u32,
    pub x_size: u32,
    pub y_size: u32,
    pub x_chars: u32,
    pub y_chars: u32,
    pub fill: u32,
    pub flags: u32,
    pub show_window: u16,
    pub reserved2_len: u16,
    pub reserved2: *mut u8,
    pub std_input: *mut c_void,
    pub std_output: *mut c_void,
    pub std_error: *mut c_void,
}

impl Default for StartupInfoW {
    fn default() -> Self {
        // SAFETY: all-zero is a valid STARTUPINFOW (null pointers, zero numbers).
        unsafe { std::mem::zeroed() }
    }
}

/// `PROCESS_INFORMATION`.
#[repr(C)]
pub struct ProcessInformation {
    pub process: *mut c_void,
    pub thread: *mut c_void,
    pub process_id: u32,
    pub thread_id: u32,
}

impl Default for ProcessInformation {
    fn default() -> Self {
        ProcessInformation { process: std::ptr::null_mut(), thread: std::ptr::null_mut(), process_id: 0, thread_id: 0 }
    }
}

// The first-launch build (`win::assets`).
#[link(name = "kernel32")]
extern "system" {
    #[allow(clippy::too_many_arguments)]
    pub fn CreateProcessW(app: *const u16, cmd: *mut u16, pattr: *mut c_void, tattr: *mut c_void, inherit: i32, flags: u32, env: *mut c_void, dir: *const u16, si: *mut StartupInfoW, pi: *mut ProcessInformation) -> i32;
    pub fn WaitForSingleObject(handle: *mut c_void, ms: u32) -> u32;
    pub fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
    pub fn TerminateProcess(process: *mut c_void, code: u32) -> i32;
    pub fn CloseHandle(handle: *mut c_void) -> i32;
    pub fn GetLastError() -> u32;
    pub fn GetSystemDirectoryW(buf: *mut u16, size: u32) -> u32;
    pub fn AllocConsole() -> i32;
    pub fn FreeConsole() -> i32;
    pub fn SetConsoleOutputCP(cp: u32) -> i32;
    pub fn SetConsoleTitleW(title: *const u16) -> i32;
    pub fn ExitProcess(code: u32) -> !;
}

#[link(name = "user32")]
extern "system" {
    pub fn MessageBoxW(window: *mut c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
}

pub const MB_OK: u32 = 0;
pub const MB_ICONWARNING: u32 = 0x30;
pub const MB_SETFOREGROUND: u32 = 0x1_0000;
pub const MB_TOPMOST: u32 = 0x4_0000;
