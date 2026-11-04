//! M13 stage 2: KH1's own sound banks in Re:CoM's (`crate::sounds`, `docs/recom-m13-plan.md`
//! § Stage 2 built).
//!
//! At start: `kh1-sounds.json` is checked against `kh1-sounds.bin`, and Re:CoM's id → bank map is
//! hooked (shared with the menu sounds: groups 9000..9999 their own banks). At a battle start with
//! Combat = KH1 (Re:CoM's battle manager ctor, already wrapped), and later at Sora's first KH1
//! motion after it (stage 3), [`ensure`] walks Re:CoM's bank list once and gives each KH1 SCD it
//! lacks to Re:CoM's own registration `0x1402F5D70` (a buffer from Re:CoM's `_aligned_malloc`,
//! owned by Re:CoM's entry from then), with KH1's system bank 9000 (the menu sounds' pack, the
//! effect programs' sounds). Re:CoM plays them by id through its own path. Nothing per frame.
//! M13 stage 4: a Keyblade's own SCDs (`Bank::cards`) are registered only for the Keyblade whose sounds
//! play; Sora's always. Banks registered earlier in the session stay (Re:CoM owns them; at most each
//! Keyblade's `.se`, about 0.9 MB, once).

use super::log;
use super::menu_sounds as msw;
use crate::sounds as sd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::OnceLock;

static FILES: OnceLock<(sd::Manifest, PathBuf)> = OnceLock::new();
/// The Keyblade card (+ 1) whose banks were all found in Re:CoM's list since the last battle start
/// (0: none; no walk needed while it stays the same).
static CONFIRMED: AtomicU32 = AtomicU32::new(0);
/// Re:CoM refused one: not tried again this session.
static REFUSED: AtomicBool = AtomicBool::new(false);
static LOGGED: AtomicBool = AtomicBool::new(false);

/// Reads and checks the asset step's files and hooks the id → bank map. Re:CoM's sounds without
/// them.
pub fn install(base: usize, manifest: Option<&Path>) {
    let Some(path) = manifest else {
        log("sounds (M13): no kh1-sounds.json (asset step not run?): no KH1 sound banks");
        return;
    };
    let pack = path.with_file_name(sd::PACK);
    let size = std::fs::metadata(&pack).map(|m| m.len()).unwrap_or(u64::MAX);
    let m = match std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display())).and_then(|t| sd::parse(&t, size)) {
        Ok(m) if !m.banks.is_empty() => m,
        Ok(_) => {
            log("sounds (M13): kh1-sounds.json lists no sound: no KH1 sound banks");
            return;
        }
        Err(e) => {
            log(&format!("sounds (M13): {e}; no KH1 sound banks"));
            return;
        }
    };
    if !msw::alloc_ok(base) {
        log("sounds (M13): Re:CoM's _aligned_malloc import is not ucrtbase's; no KH1 sound banks");
        return;
    }
    if !msw::install_map(base) {
        log("sounds (M13): Re:CoM's id → bank map is not hooked; no KH1 sound banks");
        return;
    }
    log(&format!(
        "sounds (M13): {} KH1 SCDs as Re:CoM banks {}..{} (KH1 {} .. {}; Kingdom Key base {}{}), registered at the first battle at combat = kh1",
        m.banks.len(),
        m.banks[0].bank,
        m.banks[m.banks.len() - 1].bank,
        m.banks[0].kh1_id,
        m.banks[m.banks.len() - 1].kh1_id,
        m.weapon_base,
        if m.missing.is_empty() { String::new() } else { format!("; {} wanted ids not in the player's files", m.missing.len()) }
    ));
    let _ = FILES.set((m, pack));
}

/// A battle starts (Re:CoM's battle manager ctor): at Combat = KH1, the banks are checked again.
pub fn battle_start() {
    CONFIRMED.store(0, Ordering::Release);
    if super::movement::kh1_on() {
        // The equipped Keyblade's own banks, or the Kingdom Key's (whose sounds the KH2 Keyblades play).
        let card = super::stats::equipped_card();
        let own = FILES.get().is_some_and(|(m, _)| m.banks.iter().any(|b| b.cards.contains(&card)));
        // SAFETY: on Re:CoM's update thread, as its own loaders register banks.
        unsafe { ensure(if own { card } else { crate::keyblades::KINGDOM_KEY }) };
    }
}

/// Every KH1 bank wanted with Keyblade card `card` (Sora's and that Keyblade's) is in Re:CoM's list
/// (registered now if not). True when all are.
///
/// # Safety
/// On Re:CoM's update thread (its bank list is not locked).
pub unsafe fn ensure(card: u16) -> bool {
    if CONFIRMED.load(Ordering::Acquire) == card as u32 + 1 {
        return true;
    }
    let Some((m, pack)) = FILES.get() else { return false };
    if REFUSED.load(Ordering::Relaxed) || !msw::sound_up() {
        return false;
    }
    let system = msw::ensure_system();
    let present = msw::our_banks(|b| m.banks.iter().find(|x| x.bank as i32 == b).map(|x| x.name.as_slice()));
    let mut added = 0;
    let mut failed = Vec::new();
    let mut wanted = 0;
    for b in m.banks.iter().filter(|b| b.wanted_with(card)) {
        wanted += 1;
        match present.iter().find(|p| p.0 == b.bank as i32) {
            Some((_, true)) => continue,
            Some((_, false)) => {
                failed.push(format!("bank {} is another's", b.bank));
                continue;
            }
            None => {}
        }
        let buf = msw::read_scd(pack, b.offset as u64, b.size);
        if buf == 0 {
            failed.push(format!("{} ({}) unreadable", b.name_str(), b.kh1_id));
            continue;
        }
        if msw::register(&b.name, b.bank as i32, buf, b.size) {
            added += 1;
        } else {
            failed.push(format!("{} ({}) refused", b.name_str(), b.kh1_id));
        }
    }
    let ok = failed.is_empty();
    if !ok {
        REFUSED.store(true, Ordering::Relaxed);
    }
    CONFIRMED.store(if ok { card as u32 + 1 } else { 0 }, Ordering::Release);
    if added > 0 || !ok || !LOGGED.swap(true, Ordering::Relaxed) {
        log(&format!(
            "sounds (M13): {} of {} KH1 banks for Keyblade card {card:#x} registered now ({} already there; {} in the pack); KH1's system bank 9000 {}{}",
            added,
            wanted,
            wanted - added - failed.len(),
            m.banks.len(),
            if system { "there" } else { "NOT there (no menu sounds' pack: the KH1 effects' sounds don't play)" },
            if ok { String::new() } else { format!("; FAILED: {} (not tried again this session)", failed.join(", ")) }
        ));
    }
    ok
}
