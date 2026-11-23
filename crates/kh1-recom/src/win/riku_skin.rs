//! Riku's Dark Form skin in the game (rules and addresses: `crate::riku_skin`).
//!
//! - [`preload`] (before the Keyblade model table): reads `kh1-riku-skin.json` and keeps the skins
//!   whose files are all on disk next to it.
//! - The model table (`win::models::install`) adds their rows ([`skins`]) and says so
//!   ([`rows_in`]).
//! - [`apply`] (at start, and from the Costumes page): Re:CoM's char → group entries of chars 3 / 4
//!   (`0x1405A6910 + 12`) and their motion pack names (`0x1407C6338 + 24`) point at the skin
//!   groups (Dark Mode) or back at Riku's own (Default Attire), each write checked against what is
//!   there. Re:CoM reads them when it builds a player or loads its group: the next room or battle.

use super::{log, va};
use crate::riku_skin::{self as rs, Skin};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static SKINS: OnceLock<Vec<Skin>> = OnceLock::new();
/// The PLAYER table copy holds the skin groups' rows.
static ROWS: AtomicBool = AtomicBool::new(false);
/// The tables point at the skin groups now.
static ON: AtomicBool = AtomicBool::new(false);
/// Our motion pack names, per Riku group (kept for the process: Re:CoM reads the pointers).
static NAMES: OnceLock<Vec<(u16, usize)>> = OnceLock::new();

/// Reads the manifest and checks the skin groups' files.
pub fn preload(manifest: Option<&Path>) {
    let Some(path) = manifest else {
        log("riku skin: no kh1-riku-skin.json (asset step not run?); Riku keeps his model");
        return;
    };
    let skins = match std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display())).and_then(|t| rs::parse(&t)) {
        Ok(s) => s,
        Err(e) => {
            log(&format!("riku skin: {e}; Riku keeps his model"));
            return;
        }
    };
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut ok = Vec::new();
    for s in skins {
        let missing: Vec<String> = rs::files(s.skin).into_iter().filter(|f| !f.split('/').fold(dir.clone(), |p, x| p.join(x)).is_file()).collect();
        if missing.is_empty() {
            ok.push(s);
        } else {
            log(&format!("riku skin: group {} files not found {missing:?}; group {} keeps Riku's model", s.skin, s.riku));
        }
    }
    let _ = SKINS.set(ok);
}

/// The skins whose rows go in the PLAYER table.
pub fn skins() -> Vec<Skin> {
    SKINS.get().cloned().unwrap_or_default()
}

/// The PLAYER table copy with the skins' rows is in.
pub fn rows_in() {
    ROWS.store(true, Ordering::Release);
}

/// The motion pack name pointers for chars 3 / 4 (Riku's or the skins').
unsafe fn name_ptrs(b: usize, sk: &[Skin], on: bool) -> [u64; 2] {
    let names = NAMES.get_or_init(|| {
        sk.iter()
            .map(|s| {
                let c = std::ffi::CString::new(rs::ann_name(s.skin)).unwrap_or_default();
                (s.riku, Box::leak(c.into_boxed_c_str()).as_ptr() as usize)
            })
            .collect()
    });
    let mut out = [0u64; 2];
    for (k, g) in rs::RIKU_GROUPS.iter().enumerate() {
        let recom = va(b, rs::RIKU_ANN_NAMES[k]) as u64;
        out[k] = if on { names.iter().find(|n| n.0 == *g).map_or(recom, |n| n.1 as u64) } else { recom };
    }
    out
}

/// Points chars 3 / 4 at the skin groups (`on`, Dark Mode) or back at Riku's own.
///
/// # Safety
/// `b` is the running image's base; called on start-up or from the camp (no player being built).
pub unsafe fn apply(b: usize, on: bool) {
    let sk = skins();
    if sk.is_empty() || !ROWS.load(Ordering::Acquire) {
        if on {
            log("riku skin: Dark Mode asked, but the skin's files or rows are not in; Riku keeps his model");
        }
        return;
    }
    let now = ON.load(Ordering::Relaxed);
    if now == on {
        return;
    }
    let groups = |o: bool| -> Vec<u8> { rs::entries(&sk, o).iter().flat_map(|x| x.to_le_bytes()).collect() };
    let names = |o: bool| -> Vec<u8> { name_ptrs(b, &sk, o).iter().flat_map(|x| x.to_le_bytes()).collect() };
    let g_at = va(b, rs::GROUP_TABLE) + 12;
    let n_at = va(b, rs::ANN_NAME_TABLE) + 24;
    // Both or neither: the names first (checked), then the groups; a failed group write puts the
    // names back.
    if !super::hook::patch(n_at, &names(now), &names(on), "char -> motion pack name (Riku)") {
        log("riku skin: Re:CoM's motion pack names for Riku are not PL0101 / PL0111 there; Riku keeps his model");
        return;
    }
    if !super::hook::patch(g_at, &groups(now), &groups(on), "char -> group table (Riku)") {
        super::hook::patch(n_at, &names(on), &names(now), "char -> motion pack name (Riku, back)");
        log("riku skin: Re:CoM's char -> group table is not 101 / 111 there; Riku keeps his model");
        return;
    }
    ON.store(on, Ordering::Relaxed);
    log(&format!(
        "riku skin: {} (chars 3 / 4 -> groups {:?}, motion packs {:?}); shown from the next room or battle",
        if on { "Dark Mode (Dark Riku's model, Riku's own moves)" } else { "Default Attire" },
        rs::entries(&sk, on),
        rs::entries(&sk, on).map(|g| rs::ann_name(g as u16))
    ));
}
