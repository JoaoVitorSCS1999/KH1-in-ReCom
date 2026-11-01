//! KH1's command menu sounds through Re:CoM's own sound banks (`crate::menu_sounds`;
//! `docs/recom-m7-command-menu.md` § Sounds).
//!
//! At start: read `kh1-menu-sounds.json` / `.bin` (the asset step's copy of KH1's own SCDs) and
//! hook Re:CoM's id → bank map `0x1402A8560` (groups 9000..9999 are their own bank; every other id
//! goes to Re:CoM's map unchanged). On a menu sound with KH1's look: if KH1's bank for it is not in
//! Re:CoM's bank list yet, hand it to Re:CoM's own registration `0x1402F5D70` in a buffer from
//! Re:CoM's own `_aligned_malloc` (its bank entry frees it with `_aligned_free`); then Re:CoM's
//! menu-sound call plays it. Nothing runs per frame.

use super::hook;
use super::log;
use super::movement::{func, rd};
use super::sys::{GetModuleHandleW, GetProcAddress};
use crate::command_menu::Sound;
use crate::menu_sounds as ms;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

/// The trampoline of Re:CoM's id → bank map.
pub static ORIG_SLOT: AtomicUsize = AtomicUsize::new(0);
static BANKS: OnceLock<Vec<ms::Bank>> = OnceLock::new();
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// One log line each: bit `id` registered, bit `id + 8` refused / foreign.
static NOTED: AtomicU32 = AtomicU32::new(0);
/// Bit `id`: Re:CoM refused that bank once; not tried again (Re:CoM's sound for it).
static REFUSED: AtomicU32 = AtomicU32::new(0);

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

/// Reads the asset step's files and hooks the map. False (and Re:CoM's sounds) without them.
pub fn install(base: usize, manifest: Option<&Path>) -> bool {
    let Some(path) = manifest else {
        log("menu sounds: no kh1-menu-sounds.json (asset step not run?): the command menu plays Re:CoM's sounds");
        return false;
    };
    let pack = path.with_file_name(ms::PACK);
    let banks = match (std::fs::read_to_string(path), std::fs::read(&pack)) {
        (Ok(t), Ok(b)) => ms::parse(&t, &b),
        (Err(e), _) => Err(format!("{}: {e}", path.display())),
        (_, Err(e)) => Err(format!("{}: {e}", pack.display())),
    };
    let banks = match banks {
        Ok(b) if !b.is_empty() => b,
        Ok(_) => {
            log("menu sounds: kh1-menu-sounds.json lists no sound: Re:CoM's sounds");
            return false;
        }
        Err(e) => {
            log(&format!("menu sounds: {e}; the command menu plays Re:CoM's sounds"));
            return false;
        }
    };
    // SAFETY: a read of the mapped image's import table; ucrtbase is loaded (Re:CoM imports it).
    let iat_ok = unsafe {
        let have: usize = rd(super::va(base, IAT_ALIGNED_MALLOC));
        let name: Vec<u16> = "ucrtbase.dll\0".encode_utf16().collect();
        let m = GetModuleHandleW(name.as_ptr());
        let want = if m.is_null() { 0 } else { GetProcAddress(m, b"_aligned_malloc\0".as_ptr()) as usize };
        have != 0 && have == want
    };
    if !iat_ok {
        log("menu sounds: Re:CoM's _aligned_malloc import is not ucrtbase's; Re:CoM's sounds");
        return false;
    }
    let site = |name: &str| crate::sites::MENU_SOUND_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [hook::Hook { site: site("sound id to bank (0x1402A8560)"), ours: sound_slot as *const () as usize, original: &ORIG_SLOT }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if !unsafe { hook::install_all(base, &hooks) } {
        log("menu sounds: the id → bank map is not hooked; Re:CoM's sounds");
        return false;
    }
    let list: Vec<String> = banks.iter().map(|b| format!("{} {} → id {}", b.kh1_id, b.name_str(), ms::recom_id(b.kh1_id))).collect();
    let _ = BANKS.set(banks);
    INSTALLED.store(true, Ordering::Release);
    log(&format!(
        "menu sounds: KH1's own [{}] as Re:CoM banks {}..{} (registered when first played), with the KH1 menu look (hud_kh1_menu); Re:CoM's 1 / 4 / 5 / 2 otherwise",
        list.join(", "),
        ms::GROUP_BASE,
        ms::GROUP_END - 1
    ));
    true
}

/// The Re:CoM id the command menu plays for `snd`: KH1's own sound when the KH1 look is on and
/// its bank is (or can now be) in Re:CoM's list, else Re:CoM's.
pub fn id_for(snd: Sound, kh1_look: bool) -> u32 {
    if kh1_look && INSTALLED.load(Ordering::Acquire) {
        if let Some(b) = BANKS.get().and_then(|v| v.iter().find(|b| b.kh1_id == snd.kh1_id())) {
            // SAFETY: Re:CoM's globals and list as its own play path reads them, on its update
            // thread; the register function was byte-checked at start.
            if unsafe { ensure(b) } {
                return ms::recom_id(b.kh1_id);
            }
        }
    }
    snd.recom_id()
}

fn note_once(bit: u32, msg: impl FnOnce() -> String) {
    if NOTED.fetch_or(1 << bit, Ordering::Relaxed) & (1 << bit) == 0 {
        log(&msg());
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

/// KH1's bank `b` is in Re:CoM's list (registered now if it wasn't).
unsafe fn ensure(b: &ms::Bank) -> bool {
    let base = super::base();
    let bank = b.group as i32;
    match find(base, bank, &b.name) {
        Some(true) => return true,
        Some(false) => {
            note_once(b.kh1_id + 8, || format!("menu sounds: Re:CoM bank {bank} is not KH1's {}; Re:CoM's sound for {}", b.name_str(), b.kh1_id));
            return false;
        }
        None => {}
    }
    if REFUSED.load(Ordering::Relaxed) & (1 << b.kh1_id) != 0 || rd::<usize>(super::va(base, SOUND_UP)) == 0 {
        return false;
    }
    let alloc: AlignedMallocFn = std::mem::transmute(rd::<usize>(super::va(base, IAT_ALIGNED_MALLOC)));
    let buf = alloc(b.data.len(), 0x40);
    if buf.is_null() {
        return false;
    }
    std::ptr::copy_nonoverlapping(b.data.as_ptr(), buf, b.data.len());
    // Re:CoM's entry owns the buffer from here (it frees it when the bank is released).
    let handle = func::<RegisterFn>(BANK_REGISTER)(b.name.as_ptr(), bank, buf, b.data.len() as i32);
    let ok = find(base, bank, &b.name) == Some(true);
    if ok {
        note_once(b.kh1_id, || format!("menu sounds: KH1 sound {} ({}, {} bytes) registered as Re:CoM bank {bank} (driver handle {handle:#x})", b.kh1_id, b.name_str(), b.data.len()));
    } else {
        REFUSED.fetch_or(1 << b.kh1_id, Ordering::Relaxed);
        note_once(b.kh1_id + 8, || format!("menu sounds: Re:CoM did not take KH1's {} as bank {bank}; Re:CoM's sound for {}", b.name_str(), b.kh1_id));
    }
    ok
}
