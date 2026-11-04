//! KH1's command menu sounds through Re:CoM's own sound banks (`crate::menu_sounds`;
//! `docs/recom-m7-command-menu.md` § Sounds).
//!
//! At start: check `kh1-menu-sounds.json` against `kh1-menu-sounds.bin` (KH1's system bank
//! `se000.win32.scd`, unchanged), read the SCD into a buffer from Re:CoM's own `_aligned_malloc`,
//! and hook Re:CoM's id → bank map `0x1402A8560` (groups 9000..9999 are their own bank; every
//! other id goes to Re:CoM's map unchanged). On a menu sound with KH1's look: if bank 9000 is not
//! in Re:CoM's bank list yet, hand the buffer to Re:CoM's own registration `0x1402F5D70` (its
//! bank entry owns it from then, and frees it with `_aligned_free`); then Re:CoM's menu-sound
//! call plays entry `n` as id `9000000 + n`. Nothing runs per frame.

use super::hook;
use super::log;
use super::movement::{func, rd};
use super::sys::{GetModuleHandleW, GetProcAddress};
use crate::command_menu::Sound;
use crate::menu_sounds as ms;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// The trampoline of Re:CoM's id → bank map.
pub static ORIG_SLOT: AtomicUsize = AtomicUsize::new(0);
static BANK: OnceLock<(ms::Bank, PathBuf)> = OnceLock::new();
/// The SCD read at start, in Re:CoM's `_aligned_malloc` memory, until Re:CoM's bank takes it.
static READY: Mutex<usize> = Mutex::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// One log line each (bits: 0 registered, 1 refused, 2 foreign bank, 3 read failed).
static NOTED: AtomicU32 = AtomicU32::new(0);
/// Re:CoM refused the bank once: not tried again (Re:CoM's sounds).
static REFUSED: AtomicBool = AtomicBool::new(false);

// Re:CoM (RVAs). The register function's bytes (`sites::MENU_SOUND_CALLEES`) carry both globals.
const BANK_REGISTER: u32 = 0x002F_5D70;
/// The bank list's head (`Axa::BankEntry`: `+8` next, `+0x10` bank, `+0x30` name).
const BANK_LIST: u64 = 0x1_4082_2370;
/// Non-zero once Re:CoM's sound system is up (the register function does nothing before).
const SOUND_UP: u64 = 0x1_40C1_58B0;
/// Re:CoM's import of `_aligned_malloc` (`api-ms-win-crt-heap-l1-1-0`; Checked (exe: import
/// table)); checked against ucrtbase's export at start.
const IAT_ALIGNED_MALLOC: u64 = 0x1_4050_47A0;
const E_NEXT: usize = 0x08;
const E_BANK: usize = 0x10;
const E_NAME: usize = 0x30;

type SlotFn = unsafe extern "win64" fn(u32) -> i32;
type RegisterFn = unsafe extern "win64" fn(*const u8, i32, *mut u8, i32) -> u32;
type AlignedMallocFn = unsafe extern "C" fn(usize, usize) -> *mut u8;

/// Re:CoM's id → bank map (`0x1402A8560`), with KH1's groups as their own banks.
pub unsafe extern "win64" fn sound_slot(id: u32) -> i32 {
    if let Some(b) = ms::slot_of(id) {
        return b;
    }
    let f: SlotFn = std::mem::transmute(ORIG_SLOT.load(Ordering::Acquire));
    f(id)
}

fn note_once(bit: u32, msg: impl FnOnce() -> String) {
    if NOTED.fetch_or(1 << bit, Ordering::Relaxed) & (1 << bit) == 0 {
        log(&msg());
    }
}

/// The pack read whole into a buffer from Re:CoM's `_aligned_malloc` (0 on failure).
unsafe fn read_pack(path: &Path, size: usize) -> usize {
    read_scd(path, 0, size)
}

/// Checks the asset step's files, reads the SCD and hooks the map. False (and Re:CoM's sounds)
/// without them.
pub fn install(base: usize, manifest: Option<&Path>) -> bool {
    let Some(path) = manifest else {
        log("menu sounds: no kh1-menu-sounds.json (asset step not run?): the command menu plays Re:CoM's sounds");
        return false;
    };
    let pack = path.with_file_name(ms::PACK);
    let head = || -> std::io::Result<(u64, Vec<u8>)> {
        let mut f = std::fs::File::open(&pack)?;
        let n = f.metadata()?.len();
        let mut h = vec![0u8; 0x14];
        f.read_exact(&mut h)?;
        Ok((n, h))
    };
    let bank = match (std::fs::read_to_string(path), head()) {
        (Ok(t), Ok((n, h))) => ms::parse(&t, n, &h),
        (Err(e), _) => Err(format!("{}: {e}", path.display())),
        (_, Err(e)) => Err(format!("{}: {e}", pack.display())),
    };
    let bank = match bank {
        Ok(b) if !b.ids.is_empty() => b,
        Ok(_) => {
            log("menu sounds: kh1-menu-sounds.json lists no menu sound: Re:CoM's sounds");
            return false;
        }
        Err(e) => {
            log(&format!("menu sounds: {e}; the command menu plays Re:CoM's sounds"));
            return false;
        }
    };
    if !alloc_ok(base) {
        log("menu sounds: Re:CoM's _aligned_malloc import is not ucrtbase's; Re:CoM's sounds");
        return false;
    }
    // SAFETY: the import was just checked; the buffer is ours until Re:CoM's bank takes it.
    let buf = unsafe { read_pack(&pack, bank.size) };
    if buf == 0 {
        log(&format!("menu sounds: {} could not be read into Re:CoM's memory; Re:CoM's sounds", pack.display()));
        return false;
    }
    if !install_map(base) {
        log("menu sounds: the id → bank map is not hooked; Re:CoM's sounds");
        return false;
    }
    let list: Vec<String> = bank.ids.iter().map(|&i| format!("{i} → id {}", ms::recom_id(i))).collect();
    let line = format!(
        "menu sounds: KH1's system bank {} ({} bytes) as Re:CoM bank {} (registered when first played), entries [{}], with the KH1 menu look (hud_kh1_menu); Re:CoM's 1 / 4 / 5 / 2 otherwise",
        bank.name_str(),
        bank.size,
        bank.group,
        list.join(", ")
    );
    if let Ok(mut r) = READY.lock() {
        *r = buf;
    }
    let _ = BANK.set((bank, pack));
    INSTALLED.store(true, Ordering::Release);
    log(&line);
    true
}

/// The menu sounds' files were read (their bank can be registered).
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

static MAP: OnceLock<bool> = OnceLock::new();

/// Hooks Re:CoM's id → bank map once (groups 9000..9999 their own banks): shared by the menu
/// sounds and M13's KH1 sound banks (`win::sounds`). True when the map is ours.
pub fn install_map(base: usize) -> bool {
    *MAP.get_or_init(|| {
        let site = |name: &str| crate::sites::MENU_SOUND_HOOKS.iter().find(|h| h.name == name).expect("site");
        let hooks = [hook::Hook { site: site("sound id to bank (0x1402A8560)"), ours: sound_slot as *const () as usize, original: &ORIG_SLOT }];
        // SAFETY: the target's bytes are checked again inside, before anything is written.
        unsafe { hook::install_all(base, &hooks) }
    })
}

/// Re:CoM's `_aligned_malloc` import is ucrtbase's (checked once): the buffers handed to
/// Re:CoM's bank registration come from it, as Re:CoM frees them with `_aligned_free`.
pub fn alloc_ok(base: usize) -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    // SAFETY: a read of the mapped image's import table; ucrtbase is loaded (Re:CoM imports it).
    *OK.get_or_init(|| unsafe {
        let have: usize = rd(super::va(base, IAT_ALIGNED_MALLOC));
        let name: Vec<u16> = "ucrtbase.dll\0".encode_utf16().collect();
        let m = GetModuleHandleW(name.as_ptr());
        let want = if m.is_null() { 0 } else { GetProcAddress(m, b"_aligned_malloc\0".as_ptr()) as usize };
        have != 0 && have == want
    })
}

/// `size` bytes of `path` from `offset`, read into a buffer from Re:CoM's `_aligned_malloc` and
/// checked as an SCD (0 on failure; the buffer is then given back).
pub unsafe fn read_scd(path: &Path, offset: u64, size: usize) -> usize {
    use std::io::{Seek, SeekFrom};
    let alloc: AlignedMallocFn = std::mem::transmute(rd::<usize>(super::va(super::base(), IAT_ALIGNED_MALLOC)));
    let buf = alloc(size, 0x40);
    if buf.is_null() {
        return 0;
    }
    let dst = std::slice::from_raw_parts_mut(buf, size);
    let ok = std::fs::File::open(path).and_then(|mut f| f.seek(SeekFrom::Start(offset)).and_then(|_| f.read_exact(dst))).is_ok() && ms::scd_head_ok(dst, size);
    if !ok {
        free_buf(buf);
        return 0;
    }
    buf as usize
}

/// Gives a buffer that Re:CoM did not take back the way Re:CoM would (`_aligned_free`, by name).
unsafe fn free_buf(buf: *mut u8) {
    let name: Vec<u16> = "ucrtbase.dll\0".encode_utf16().collect();
    let m = GetModuleHandleW(name.as_ptr());
    let free = if m.is_null() { std::ptr::null_mut() } else { GetProcAddress(m, b"_aligned_free\0".as_ptr()) };
    if !free.is_null() {
        let f: unsafe extern "C" fn(*mut u8) = std::mem::transmute(free);
        f(buf);
    }
}

/// Re:CoM's sound system is up (its bank registration does nothing before).
pub unsafe fn sound_up() -> bool {
    rd::<usize>(super::va(super::base(), SOUND_UP)) != 0
}

/// The banks in 9000..9999 Re:CoM's list holds, with whether each one's name is `name_of`'s.
/// One walk of the list.
pub unsafe fn our_banks(name_of: impl Fn(i32) -> Option<&'static [u8]>) -> Vec<(i32, bool)> {
    let mut out = Vec::new();
    let mut e: usize = rd(super::va(super::base(), BANK_LIST));
    let mut n = 0;
    while e != 0 && n < 100_000 {
        let b: i32 = rd(e + E_BANK);
        if (ms::GROUP_FIRST as i32..ms::GROUP_END as i32).contains(&b) && !out.iter().any(|x: &(i32, bool)| x.0 == b) {
            let same = name_of(b).is_some_and(|name| std::slice::from_raw_parts((e + E_NAME) as *const u8, name.len()) == name);
            out.push((b, same));
        }
        e = rd(e + E_NEXT);
        n += 1;
    }
    out
}

/// Hands `buf` (`size` bytes, from [`read_scd`]) to Re:CoM's own bank registration as bank `bank`
/// named `name` (NUL-terminated); Re:CoM's entry owns it from then. True when the bank is then in
/// Re:CoM's list under that name; otherwise the buffer is given back.
pub unsafe fn register(name: &[u8], bank: i32, buf: usize, size: usize) -> bool {
    func::<RegisterFn>(BANK_REGISTER)(name.as_ptr(), bank, buf as *mut u8, size as i32);
    let ok = find(super::base(), bank, name) == Some(true);
    if !ok && find(super::base(), bank, name).is_none() {
        // Not taken (no entry made): still ours to give back.
        free_buf(buf as *mut u8);
    }
    ok
}

/// The Re:CoM id the command menu plays for `snd`: KH1's own entry when the KH1 look is on and
/// KH1's bank is (or can now be) in Re:CoM's list, else Re:CoM's.
pub fn id_for(snd: Sound, kh1_look: bool) -> u32 {
    if kh1_look && INSTALLED.load(Ordering::Acquire) {
        if let Some((b, pack)) = BANK.get() {
            // SAFETY: Re:CoM's globals and list as its own play path reads them, on its update
            // thread; the register function was byte-checked at start.
            if b.ids.contains(&snd.kh1_id()) && unsafe { ensure(b, pack) } {
                return ms::recom_id(snd.kh1_id());
            }
        }
    }
    snd.recom_id()
}

/// KH1's system bank (bank 9000: the menu sounds' pack, also the KH1 effect programs' sounds,
/// M13) is in Re:CoM's list, registered now if it wasn't. False without the menu sounds' files.
pub fn ensure_system() -> bool {
    if !INSTALLED.load(Ordering::Acquire) {
        return false;
    }
    match BANK.get() {
        // SAFETY: as `id_for`.
        Some((b, pack)) => unsafe { ensure(b, pack) },
        None => false,
    }
}

/// Re:CoM's entry for bank `bank`: Some(true) ours, Some(false) another, None none.
unsafe fn find(base: usize, bank: i32, name: &[u8]) -> Option<bool> {
    let mut e: usize = rd(super::va(base, BANK_LIST));
    let mut n = 0;
    while e != 0 && n < 100_000 {
        if rd::<i32>(e + E_BANK) == bank {
            let have = std::slice::from_raw_parts((e + E_NAME) as *const u8, name.len());
            return Some(have == name);
        }
        e = rd(e + E_NEXT);
        n += 1;
    }
    None
}

/// KH1's bank is in Re:CoM's list (registered now if it wasn't).
unsafe fn ensure(b: &ms::Bank, pack: &Path) -> bool {
    let base = super::base();
    let bank = b.group as i32;
    match find(base, bank, &b.name) {
        Some(true) => return true,
        Some(false) => {
            note_once(2, || format!("menu sounds: Re:CoM bank {bank} is not KH1's {}; Re:CoM's sounds", b.name_str()));
            return false;
        }
        None => {}
    }
    if REFUSED.load(Ordering::Relaxed) || rd::<usize>(super::va(base, SOUND_UP)) == 0 {
        return false;
    }
    // The buffer read at start; after a release of the bank (a sound-system reset), read again.
    let mut buf = READY.lock().map(|mut r| std::mem::take(&mut *r)).unwrap_or(0);
    if buf == 0 {
        buf = read_pack(pack, b.size);
        if buf == 0 {
            note_once(3, || format!("menu sounds: {} could not be read again; Re:CoM's sounds", pack.display()));
            REFUSED.store(true, Ordering::Relaxed);
            return false;
        }
    }
    // Re:CoM's entry owns the buffer from here (it frees it when the bank is released).
    let handle = func::<RegisterFn>(BANK_REGISTER)(b.name.as_ptr(), bank, buf as *mut u8, b.size as i32);
    let ok = find(base, bank, &b.name) == Some(true);
    if ok {
        note_once(0, || format!("menu sounds: KH1's {} ({} bytes) registered as Re:CoM bank {bank} (driver handle {handle:#x})", b.name_str(), b.size));
    } else {
        REFUSED.store(true, Ordering::Relaxed);
        note_once(1, || format!("menu sounds: Re:CoM did not take KH1's {} as bank {bank}; Re:CoM's sounds", b.name_str()));
    }
    ok
}
