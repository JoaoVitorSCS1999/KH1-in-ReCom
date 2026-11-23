//! Riku's Dark Form skin in the game (rules and addresses: `crate::riku_skin`).
//!
//! - [`preload`] (before the Keyblade model table): reads `kh1-riku-skin.json` and keeps the skins
//!   whose files are all on disk next to it.
//! - The model table (`win::models::install`) adds their rows ([`skins`]).
//! - [`apply`] (after the table is in): with `riku_dark_skin = on`, Re:CoM's char → group entries
//!   of chars 3 and 4 (`0x1405A6910 + 12`, two i32) become the skin groups, checked against
//!   Re:CoM's 101 / 111 first. Nothing per frame: Re:CoM reads them when it builds a player or
//!   loads its group.

use super::{log, va};
use crate::riku_skin::{self as rs, Skin};
use std::path::Path;
use std::sync::OnceLock;

static SKINS: OnceLock<Vec<Skin>> = OnceLock::new();

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

/// Points chars 3 / 4 at the skin groups (or back at Riku's own).
///
/// # Safety
/// `b` is the running image's base; called on start-up, before any player is built.
pub unsafe fn apply(b: usize, on: bool) {
    let sk = skins();
    if sk.is_empty() {
        return;
    }
    let at = va(b, rs::GROUP_TABLE) + 12;
    let bytes = |e: [u32; 2]| -> Vec<u8> { e.iter().flat_map(|x| x.to_le_bytes()).collect() };
    let from = bytes(rs::entries(&sk, !on));
    let to = bytes(rs::entries(&sk, on));
    if from == to {
        return;
    }
    if super::hook::patch(at, &from, &to, "char -> group table (Riku)") {
        log(&format!("riku skin: {} (Dark Form: Dark Riku's model, Riku's own moves): chars 3 / 4 -> groups {:?}", if on { "on" } else { "off" }, rs::entries(&sk, on)));
    } else {
        log("riku skin: Re:CoM's char -> group table is not 101 / 111 there; Riku keeps his model");
    }
}
