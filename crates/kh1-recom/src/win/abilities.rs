//! KH1's ability state at the KH1 setting (M9 stage 9a; `crate::abilities`,
//! `docs/recom-m9-abilities.md`).
//!
//! - **Store:** our ability block in Re:CoM's save (`crate::abilities::BLOCK_FIRST`), read and
//!   written only at a sync. Re:CoM saves, loads, snapshots and zeroes it with its save block.
//! - **Sync** ([`sync`]): at each Attack press Re:CoM's dispatch hands to KH1's frame (the
//!   field-swing init wrapper), never per frame: the abilities not yet unlocked are checked against
//!   Re:CoM's own learned-sleight test (`0x14023AB00`) and Re:CoM's level; new ones are unlocked
//!   and equipped (interim rule until 9b), written back, logged.
//! - **Use:** [`equipped`] (the equipped modifiers whose rows the attack table holds) goes into the
//!   `kh1_core::Abilities` the KH1 frame passes to KH1's pickers (`win::movement::abilities`).

use super::movement::{func, rd, wr};
use super::{base, log, va};
use crate::abilities::{self as ab, State};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// The modifiers whose rows the attack table holds (`crate::attack::modifier_rows_ready`).
static READY: AtomicU32 = AtomicU32::new(0);
/// The equipped abilities after the last sync (`ab::ix` bits).
static EQUIPPED: AtomicU32 = AtomicU32::new(0);
/// `kh1_all_abilities = on` (debug): every modifier equipped, nothing written.
static ALL: AtomicBool = AtomicBool::new(false);
static LOGGED_FIRST: AtomicBool = AtomicBool::new(false);

/// Re:CoM's "sleight learned" (`0x14023AB00(u16 id)`, `sites::ABILITY_CALLEES`).
const SLEIGHT_LEARNED: u32 = 0x0023_AB00;
type LearnedFn = unsafe extern "win64" fn(u16) -> u64;

pub fn set_rows_ready(bits: u32) {
    READY.store(bits, Ordering::Relaxed);
}

pub fn set_all(on: bool) {
    ALL.store(on, Ordering::Relaxed);
    if on {
        log("abilities: kh1_all_abilities = on: every combo modifier equipped (debug; not saved)");
    }
}

/// The equipped modifiers the pickers get (`ab::ix` bits): those with rows.
pub fn equipped() -> u32 {
    let e = if ALL.load(Ordering::Relaxed) { ab::MODIFIER_BITS } else { EQUIPPED.load(Ordering::Relaxed) };
    e & READY.load(Ordering::Relaxed)
}

fn save() -> Option<usize> {
    // SAFETY: a static pointer of the image.
    let s: usize = unsafe { rd(va(base(), crate::keyblades::SAVE_PTR)) };
    (s != 0).then_some(s)
}

/// Brings the state up to Re:CoM's (sleights learned, level), writes it back when it changed.
/// Only while KH1's MP (the KH1 setting, Sora's story, a save) is live.
pub fn sync() {
    if !super::mp::active() {
        return;
    }
    let Some(save) = save() else { return };
    let at = save + ab::BLOCK_FIRST;
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
    let mut block: [u8; ab::BLOCK_LEN] = unsafe { rd(at) };
    let before = ab::read(&block);
    let mut s: State = before;
    let level = super::stats::sora_level();
    // SAFETY: Re:CoM's own test (`sites::ABILITY_CALLEES`, bytes checked at start), a pure read of
    // its save.
    let learned = unsafe { func::<LearnedFn>(SLEIGHT_LEARNED) };
    let new = ab::sync(&mut s, level, &mut |id| unsafe { learned(id) } & 0xFF != 0);
    if s != before {
        ab::write(&s, &mut block);
        // SAFETY: as above.
        unsafe { wr(at, block) };
    }
    if new != 0 {
        log(&format!("abilities: unlocked and equipped at Re:CoM level {level}: {:?}", ab::names(new)));
    }
    if !LOGGED_FIRST.swap(true, Ordering::Relaxed) {
        log(&format!(
            "abilities: level {level}; unlocked {:?}; equipped {:?}; modifiers the pickers get {:?}",
            ab::names(s.unlocked),
            ab::names(s.equipped),
            ab::names(s.equipped & READY.load(Ordering::Relaxed))
        ));
    }
    EQUIPPED.store(s.equipped, Ordering::Relaxed);
}

