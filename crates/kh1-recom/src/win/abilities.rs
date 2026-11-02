//! KH1's ability state at the KH1 setting (M9 stages 9a / 9b; `crate::abilities`,
//! `docs/recom-m9-abilities.md`).
//!
//! - **Store:** our ability block in Re:CoM's save (`crate::abilities::BLOCK_FIRST`). Re:CoM saves,
//!   loads, snapshots and zeroes it with its save block.
//! - **Sync** ([`sync`]): at each Attack press Re:CoM's dispatch hands to KH1's frame, and when the
//!   camp's Abilities page opens, never per frame: the abilities not yet unlocked are checked
//!   against Re:CoM's own learned-sleight test (`0x14023AB00`), Re:CoM's level and Re:CoM's
//!   ability variable (0, 0x24) read with its getter `0x1401B08C0` (new ones arrive unequipped,
//!   KH1's rule; Re:CoM's High Jump / Glide / Superglide equipped, owner); a block older than layout
//!   v4 is migrated once (`crate::abilities::migrate`: Guard and Dodge Roll leave it, they are
//!   Sora's kit); written back, logged.
//! - **Use:** [`equipped_live`] reads the equipped word from the block (a 15-byte read) for the
//!   abilities the KH1 frame passes on (`win::movement::abilities`), so what the camp's page
//!   changes applies at once, and a loaded save is followed with no event of ours.
//! - **The page** (`win::keyblades`, the KH1 entry's Abilities row): [`page_state`] and
//!   [`set_equipped`].

use super::movement::{func, rd, wr};
use super::{base, log, va};
use crate::abilities::{self as ab, State};
use crate::settings::Kh1Abilities;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// The modifiers whose rows the attack table holds (`crate::attack::modifier_rows_ready`).
static READY: AtomicU32 = AtomicU32::new(0);
/// `kh1_all_abilities = on` (debug): every ability equipped, nothing written.
static ALL: AtomicBool = AtomicBool::new(false);
/// The ini's movement abilities for the migration of a v0 / v1 block
/// (`crate::abilities::ini_equipped`).
static INI_EQUIPPED: AtomicU32 = AtomicU32::new(0);
static LOGGED_FIRST: AtomicBool = AtomicBool::new(false);

/// Re:CoM's "sleight learned" (`0x14023AB00(u16 id)`, `sites::ABILITY_CALLEES`).
const SLEIGHT_LEARNED: u32 = 0x0023_AB00;
type LearnedFn = unsafe extern "win64" fn(u16) -> u64;
/// Re:CoM's int variable getter (`0x1401B08C0(set, id)`, `sites::M6_DECK_CALLEES`, bytes checked at
/// start), for its ability variable (`crate::abilities::recom::VAR`).
const VAR_GET: u32 = 0x001B_08C0;
type VarGetFn = unsafe extern "win64" fn(u32, u32) -> u64;

pub fn set_rows_ready(bits: u32) {
    READY.store(bits, Ordering::Relaxed);
}

pub fn set_all(on: bool) {
    ALL.store(on, Ordering::Relaxed);
    if on {
        log("abilities: kh1_all_abilities = on: every ability equipped, AP ignored (debug; not saved)");
    }
}

/// The ini's movement abilities, read once per save at its migration from v0 / v1. (The ini's
/// `high_jump`, `glide`, `superglide`, `dodge_roll`, `combo_master` no longer unlock anything:
/// only the KH1 frame's fallback with no save block reads them, `win::movement::abilities`.)
pub fn set_ini(a: &Kh1Abilities) {
    INI_EQUIPPED.store(ab::ini_equipped(a), Ordering::Relaxed);
}

fn save() -> Option<usize> {
    // SAFETY: a static pointer of the image.
    let s: usize = unsafe { rd(va(base(), crate::keyblades::SAVE_PTR)) };
    (s != 0).then_some(s)
}

/// The modifiers without rows never reach the pickers.
fn usable(e: u32) -> u32 {
    e & (READY.load(Ordering::Relaxed) | !ab::MODIFIER_BITS)
}

/// Brings the state up to Re:CoM's (sleights learned, level, its ability variable), migrating an
/// older block once; writes it back when it changed. None unless KH1's MP is live (the KH1
/// setting, Sora's story, a save).
pub fn sync() -> Option<State> {
    if !super::mp::active() {
        return None;
    }
    let save = save()?;
    let at = save + ab::BLOCK_FIRST;
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
    let mut block: [u8; ab::BLOCK_LEN] = unsafe { rd(at) };
    let version = ab::version(&block);
    let before = ab::read(&block);
    // The words as stored (an older block may still hold the kit's bits, which `read` hides).
    let raw = State { unlocked: u32::from_le_bytes(block[ab::field::UNLOCKED..ab::field::UNLOCKED + 4].try_into().unwrap_or([0; 4])), equipped: u32::from_le_bytes(block[ab::field::EQUIPPED..ab::field::EQUIPPED + 4].try_into().unwrap_or([0; 4])) };
    let mut s: State = before;
    let level = super::stats::sora_level();
    // SAFETY: Re:CoM's own getter (bytes checked at start), a pure read of its save variables.
    let recom_var = unsafe { func::<VarGetFn>(VAR_GET)(ab::recom::VAR.0, ab::recom::VAR.1) } as u32;
    // SAFETY: Re:CoM's own test (`sites::ABILITY_CALLEES`, bytes checked at start), a pure read of
    // its save.
    let learned = unsafe { func::<LearnedFn>(SLEIGHT_LEARNED) };
    let mut ask = |id: u16| unsafe { learned(id) } & 0xFF != 0;
    let migrating = version < ab::BLOCK_NOW;
    let r = if migrating { ab::migrate(&mut s, raw, version, level, recom_var, INI_EQUIPPED.load(Ordering::Relaxed), &mut ask) } else { ab::sync(&mut s, level, recom_var, &mut ask) };
    if s != before || migrating {
        ab::write(&s, &mut block);
        // SAFETY: as above.
        unsafe { wr(at, block) };
    }
    if migrating {
        log(&format!(
            "abilities: block v{version} -> v{} at Re:CoM level {level} (max AP {}, Re:CoM's ability variable {recom_var:#x}): equipped {:?} ({} AP){}{}{}",
            ab::BLOCK_NOW,
            ab::max_ap(level),
            ab::names(s.equipped),
            ab::used_ap(s.equipped),
            if r.kit_dropped != 0 { format!("; Sora's kit now, out of the block (their AP back) {:?}", ab::names(r.kit_dropped)) } else { String::new() },
            if r.taken != 0 { format!("; the ini's, not yet from their source, taken back {:?}", ab::names(r.taken)) } else { String::new() },
            if r.fitted != 0 { format!("; over KH1's AP, unequipped {:?}", ab::names(r.fitted)) } else { String::new() }
        ));
    }
    if r.new != 0 {
        log(&format!("abilities: learned at Re:CoM level {level} (unequipped, KH1's rule; Re:CoM's ability variable {recom_var:#x}): {:?}", ab::names(r.new)));
    }
    if r.new_equipped != 0 {
        log(&format!("abilities: learned at Re:CoM level {level} and equipped (owner: Re:CoM gave it; removable): {:?}", ab::names(r.new_equipped)));
    }
    if !LOGGED_FIRST.swap(true, Ordering::Relaxed) {
        log(&format!(
            "abilities: level {level}, Re:CoM's ability variable {recom_var:#x}, AP {}/{} left; unlocked {:?}; equipped {:?}; modifiers the pickers get {:?}",
            ab::ap_left(s.equipped, level),
            ab::max_ap(level),
            ab::names(s.unlocked),
            ab::names(s.equipped),
            ab::names(usable(s.equipped) & ab::MODIFIER_BITS)
        ));
    }
    Some(s)
}

/// The equipped abilities the KH1 frame passes on (`ab::ix` bits, the modifiers only with rows),
/// or None at the Re:CoM setting or with no save block (the ini then stands, as before stage 9b).
/// Each KH1 frame: one 15-byte read of the block; a block older than v3 is migrated by [`sync`]
/// once.
pub fn equipped_live() -> Option<u32> {
    if ALL.load(Ordering::Relaxed) {
        return Some(usable(ab::ALL_BITS & !ab::SPECIALS_WAITING & !ab::KIT_BITS));
    }
    // The KH1 frame runs only for Sora at the KH1 setting (never in Riku's story), so no story
    // test here: one atomic load, one pointer read and the block.
    if !super::movement::kh1_on() {
        return None;
    }
    let save = save()?;
    // SAFETY: inside the 0x2AD4-byte save block.
    let block: [u8; ab::BLOCK_LEN] = unsafe { rd(save + ab::BLOCK_FIRST) };
    let e = if ab::version(&block) < ab::BLOCK_NOW { sync()?.equipped } else { ab::read(&block).equipped };
    Some(usable(e))
}

/// The camp's Abilities page: the state after a sync, and Sora's level. None when not live.
pub fn page_state() -> Option<(State, u32)> {
    let s = sync()?;
    Some((s, super::stats::sora_level()))
}

/// The camp's Abilities page writes the equipped set (a subset of the unlocked set). False when
/// the state isn't live.
pub fn set_equipped(equipped: u32) -> bool {
    let Some(mut s) = sync() else { return false };
    let Some(save) = save() else { return false };
    s.equipped = equipped & s.unlocked;
    let at = save + ab::BLOCK_FIRST;
    // SAFETY: inside the 0x2AD4-byte save block.
    let mut block: [u8; ab::BLOCK_LEN] = unsafe { rd(at) };
    ab::write(&s, &mut block);
    // SAFETY: as above.
    unsafe { wr(at, block) };
    true
}
