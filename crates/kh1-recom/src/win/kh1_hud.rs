//! The KH1 HUD option (decision 39; `crate::kh1_hud`, `docs/recom-kh1-hud.md`).
//!
//! Stage H1: the `kh1hud` sprite set. The asset step adds `kh1hud.lad` / `kh1hud.imz` (and their
//! PC copies) to Re:CoM's `FO0002.SPR` / `.RTM`, which Re:CoM loads with the player HUD; Re:CoM
//! registers `kh1hud:00` itself. Here: the manifest, and at each player gauge's init (the MP bar's
//! wrapper) a check that Re:CoM's 2D registry holds `kh1hud:00` with the manifest's group count,
//! logged once. Nothing is drawn yet; `hud = kh1` in `kh1-recom.ini` (interim switch) changes
//! nothing visible in H1.

use super::log;
use crate::kh1_hud::{self as kh, Manifest};
use crate::settings::Mode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static MANIFEST: OnceLock<Manifest> = OnceLock::new();
static CHECKED: AtomicBool = AtomicBool::new(false);

/// The manifest, once installed.
pub fn manifest() -> Option<&'static Manifest> {
    MANIFEST.get()
}

/// The setting: `hud = kh1` (and a manifest).
pub fn wanted() -> bool {
    manifest().is_some() && super::settings().hud == Mode::Kh1
}

/// Reads `kh1-hud.json` (needs the MP bar's gauge wrappers, installed before).
pub fn install(manifest: Option<&std::path::Path>) -> bool {
    let Some(path) = manifest else {
        log("kh1 hud: no kh1-hud.json (asset step not run?): Re:CoM's HUD only");
        return false;
    };
    let m = match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| kh::parse_manifest(&t)) {
        Ok(m) => m,
        Err(e) => {
            log(&format!("kh1 hud: {}: {e}; Re:CoM's HUD only", path.display()));
            return false;
        }
    };
    log(&format!(
        "kh1 hud: kh1hud:00 ({} groups, PC copies {:?}) from {}; hud = {} in kh1-recom.ini",
        m.groups,
        m.languages,
        path.display(),
        super::settings().hud.as_str()
    ));
    let _ = MANIFEST.set(m);
    true
}

/// A player gauge was made (its init has run; Re:CoM has loaded FO0002): checks once that
/// Re:CoM registered our set with the manifest's group count.
pub unsafe fn gauge_made(_gauge: usize, _list: i32) {
    let Some(m) = manifest() else { return };
    if CHECKED.swap(true, Ordering::Relaxed) {
        return;
    }
    match super::mp_bar::set_groups(kh::SET) {
        Some(n) if n == m.groups => log(&format!(
            "kh1 hud: Re:CoM registered kh1hud:00 with {n} groups (FO0002's kh1hud.lad / .imz are served); hud = {}",
            if wanted() { "kh1" } else { "recom" }
        )),
        Some(n) => log(&format!("kh1 hud: kh1hud:00 has {n} groups, not {} (an older FO0002.SPR is served?)", m.groups)),
        None => log("kh1 hud: kh1hud:00 is not in Re:CoM's 2D registry (Panacea serves Re:CoM's own FORM/0002/FO0002.SPR: add raw/FORM/0002/FO0002.SPR and .RTM to mod.yml)"),
    }
}
