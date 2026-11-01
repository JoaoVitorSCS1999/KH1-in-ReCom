//! Crash notes: a last vectored exception handler that writes a fatal exception's code, place
//! (Re:CoM's exe or our DLL, as a Steam address / DLL offset), registers and the return
//! addresses on the stack into `kh1-recom.log`, then lets the exception go on as before (the game
//! still crashes the way it would have). Diagnostics only: nothing of the game's is changed.
//! Added 2026-11-01: the owner's crash left an empty `CrashDump.dmp`.

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

#[link(name = "kernel32")]
extern "system" {
    fn AddVectoredExceptionHandler(first: u32, handler: unsafe extern "system" fn(*mut c_void) -> i32) -> *mut c_void;
}

const CONTINUE_SEARCH: i32 = 0;
/// Codes worth a note: access violation, illegal instruction, divide by zero, stack overflow,
/// stack buffer overrun / fail-fast, heap corruption.
const FATAL: [u32; 6] = [0xC000_0005, 0xC000_001D, 0xC000_0094, 0xC000_00FD, 0xC000_0409, 0xC000_0374];
static NOTES: AtomicUsize = AtomicUsize::new(0);
static OURS: AtomicUsize = AtomicUsize::new(0);
static OURS_END: AtomicUsize = AtomicUsize::new(0);

/// SizeOfImage of a loaded module (its PE header).
unsafe fn image_size(base: usize) -> usize {
    if base == 0 {
        return 0;
    }
    let pe = base + std::ptr::read_unaligned((base + 0x3C) as *const u32) as usize;
    std::ptr::read_unaligned((pe + 0x50) as *const u32) as usize
}

/// Installs the handler (once).
pub fn install(our_module: usize) {
    // SAFETY: our own module's mapped PE header.
    let size = unsafe { image_size(our_module) };
    OURS.store(our_module, Ordering::Relaxed);
    OURS_END.store(our_module + size, Ordering::Relaxed);
    // SAFETY: a plain kernel32 call with a handler that only reads.
    let h = unsafe { AddVectoredExceptionHandler(0, handler) };
    super::log(&format!("crash notes: {}", if h.is_null() { "handler not added" } else { "fatal exceptions are written here" }));
}

fn place(a: usize) -> String {
    let exe = super::base();
    // SAFETY: the exe's mapped PE header.
    let exe_end = exe + unsafe { image_size(exe) };
    let (o, oe) = (OURS.load(Ordering::Relaxed), OURS_END.load(Ordering::Relaxed));
    if exe != 0 && (exe..exe_end).contains(&a) {
        format!("exe {:#x}", a - exe + 0x1_4000_0000)
    } else if o != 0 && (o..oe).contains(&a) {
        format!("kh1-recom.dll+{:#x}", a - o)
    } else {
        format!("{a:#x}")
    }
}

unsafe extern "system" fn handler(info: *mut c_void) -> i32 {
    let rec = std::ptr::read(info as *const *const u8);
    let ctx = std::ptr::read((info as *const *const u8).add(1));
    if rec.is_null() || ctx.is_null() {
        return CONTINUE_SEARCH;
    }
    let code = std::ptr::read_unaligned(rec as *const u32);
    if !FATAL.contains(&code) || NOTES.fetch_add(1, Ordering::Relaxed) >= 3 {
        return CONTINUE_SEARCH;
    }
    let at = std::ptr::read_unaligned(rec.add(0x10) as *const usize);
    let n = std::ptr::read_unaligned(rec.add(0x18) as *const u32);
    let p = |i: usize| std::ptr::read_unaligned(rec.add(0x20 + 8 * i) as *const usize);
    let detail = if code == 0xC000_0005 && n >= 2 { format!(" ({} {:#x})", if p(0) == 1 { "writing" } else if p(0) == 8 { "executing" } else { "reading" }, p(1)) } else { String::new() };
    let r = |off: usize| std::ptr::read_unaligned(ctx.add(off) as *const usize);
    let (rax, rcx, rdx, rbx, rsp, rbp, rsi, rdi, r8, r9) = (r(0x78), r(0x80), r(0x88), r(0x90), r(0x98), r(0xA0), r(0xA8), r(0xB0), r(0xB8), r(0xC0));
    // Return addresses into the exe or our DLL among the first 256 stack slots.
    let mut calls = Vec::new();
    for i in 0..256 {
        let v = std::ptr::read_unaligned((rsp + 8 * i) as *const usize);
        let s = place(v);
        if !s.starts_with("0x") {
            calls.push(s);
            if calls.len() >= 16 {
                break;
            }
        }
    }
    super::log(&format!(
        "CRASH: exception {code:#x} at {}{detail}; rax {rax:#x} rcx {rcx:#x} rdx {rdx:#x} rbx {rbx:#x} rsi {rsi:#x} rdi {rdi:#x} r8 {r8:#x} r9 {r9:#x} rbp {rbp:#x} rsp {rsp:#x}; stack: {}",
        place(at),
        calls.join(", ")
    ));
    CONTINUE_SEARCH
}
