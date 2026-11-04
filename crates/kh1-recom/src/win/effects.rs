//! M13 stage 1 (`crate::effects`): checks once, at the first battle, that Re:CoM has the KH1 effect
//! sets the asset step added to `EF1000.EFF`, through Re:CoM's own effect-by-name `0x1402A4920`
//! (`0x1402A43E0(5, name)`, then with `_`; it returns the set, resource `+0x68`). One log line per
//! set: found with its entry count (set `+0xC`), or not found. No hook, no patch, nothing per frame.

use super::log;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static SETS: Mutex<Vec<crate::effects::Set>> = Mutex::new(Vec::new());
static CHECKED: AtomicBool = AtomicBool::new(false);
static READY: AtomicBool = AtomicBool::new(false);

type ByNameFn = unsafe extern "win64" fn(*const u8) -> usize;

/// Reads the manifest and checks Re:CoM's effect-by-name.
pub fn install(base: usize, manifest: Option<&Path>) {
    let Some(path) = manifest else {
        log("effects (M13): no kh1-effects.json (asset step not run?): no KH1 effect sets");
        return;
    };
    let sets = match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| crate::effects::parse(&t)) {
        Ok(s) => s,
        Err(e) => {
            log(&format!("effects (M13): {e}"));
            return;
        }
    };
    let site = &crate::sites::EFFECT_CHECKED[0];
    // SAFETY: a read of the mapped exe.
    let have = unsafe { std::slice::from_raw_parts((base + site.rva as usize) as *const u8, site.expect.len()) };
    if have != site.expect {
        log(&format!("effects (M13): {} is not as expected; the KH1 sets are not checked", site.name));
        return;
    }
    log(&format!(
        "effects (M13): the asset step added {} to EF1000.EFF; checked at the first battle",
        sets.iter().map(|s| format!("{} ({} effects)", s.name, s.ids.len())).collect::<Vec<_>>().join(", ")
    ));
    *SETS.lock().unwrap() = sets;
    READY.store(true, Ordering::Release);
}

/// At a battle start (Re:CoM's battle manager ctor): once per session, looks each set up.
pub unsafe fn battle_check() {
    if !READY.load(Ordering::Acquire) || CHECKED.swap(true, Ordering::AcqRel) {
        return;
    }
    let by_name: ByNameFn = super::movement::func(crate::sites::EFFECT_CHECKED[0].rva);
    for s in SETS.lock().unwrap().iter() {
        let name = crate::effects::lookup_name(&s.name);
        let set = by_name(name.as_ptr());
        if set == 0 {
            log(&format!("effects (M13): {} is NOT loaded (Re:CoM serves its own EF1000.EFF? the mod's raw/EFFECT/1000/EF1000.EFF missing)", s.name));
        } else {
            let n = std::ptr::read_unaligned((set + 0xC) as *const u32);
            log(&format!("effects (M13): {} loaded by Re:CoM ({} entries; the manifest lists {})", s.name, n, s.ids.len()));
        }
    }
}
