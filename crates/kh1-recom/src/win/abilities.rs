//! KH1's ability state at the KH1 setting (M9 stages 9a / 9b; `crate::abilities`,
//! `docs/recom-m9-abilities.md`).
//!
//! - **Store:** our ability block in Re:CoM's save (`crate::abilities::BLOCK_FIRST`). Re:CoM saves,
//!   loads, snapshots and zeroes it with its save block.
//! - **Sync** ([`sync`]): at each Attack press Re:CoM's dispatch hands to KH1's frame, and when the
//!   camp's Abilities page opens, never per frame: the abilities not yet unlocked are checked
//!   against Re:CoM's own learned-sleight test (`0x14023AB00`) and Re:CoM's level (new ones arrive
//!   unequipped, KH1's rule), the story abilities against the ini; a block older than layout v2 is
//!   migrated once (`crate::abilities::migrate`); written back, logged.
//! - **Use:** [`equipped_live`] reads the equipped word from the block (a 15-byte read) for the
//!   abilities the KH1 frame passes on (`win::movement::abilities`), so what the camp's page
//!   changes applies at once, and a loaded save is followed with no event of ours.
//! - **The page** (`win::keyblades`, the KH1 entry's Abilities row): [`page_state`] and
//!   [`set_equipped`].

use super::movement::{func, rd, wr};
use super::{base, log, va};
use crate::abilities::{self as ab, State};
use crate::settings::Kh1Abilities;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

/// The modifiers whose rows the attack table holds (`crate::attack::modifier_rows_ready`).
static READY: AtomicU32 = AtomicU32::new(0);
/// `kh1_all_abilities = on` (debug): every ability equipped, nothing written.
static ALL: AtomicBool = AtomicBool::new(false);
/// The ini's story abilities (`crate::abilities::story_mask`) and its movement abilities for the
/// migration (`crate::abilities::ini_equipped`).
static STORY: AtomicU8 = AtomicU8::new(0);
static INI_EQUIPPED: AtomicU32 = AtomicU32::new(0);
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
        log("abilities: kh1_all_abilities = on: every ability equipped, AP ignored (debug; not saved)");
    }
}

/// The ini's abilities: the story ones' unlock, and the movement ones for the migration.
pub fn set_ini(a: &Kh1Abilities) {
    STORY.store(ab::story_mask(a), Ordering::Relaxed);
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

/// Brings the state up to Re:CoM's (sleights learned, level) and the ini's story keys, migrating
/// an older block once; writes it back when it changed. None unless KH1's MP is live (the KH1
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
    let mut s: State = before;
    let level = super::stats::sora_level();
    let story = STORY.load(Ordering::Relaxed);
    // SAFETY: Re:CoM's own test (`sites::ABILITY_CALLEES`, bytes checked at start), a pure read of
    // its save.
    let learned = unsafe { func::<LearnedFn>(SLEIGHT_LEARNED) };
    let mut ask = |id: u16| unsafe { learned(id) } & 0xFF != 0;
    let migrating = version < ab::BLOCK_V2;
    let r = if migrating { ab::migrate(&mut s, level, story, INI_EQUIPPED.load(Ordering::Relaxed), &mut ask) } else { ab::sync(&mut s, level, story, &mut ask) };
    if s != before || migrating {
        ab::write(&s, &mut block);
        // SAFETY: as above.
        unsafe { wr(at, block) };
    }
    if migrating {
        log(&format!(
            "abilities: block v{version} -> v2 at Re:CoM level {level} (max AP {}): equipped {:?} ({} AP){}",
            ab::max_ap(level),
            ab::names(s.equipped),
            ab::used_ap(s.equipped),
            if r.fitted != 0 { format!("; over KH1's AP, unequipped {:?}", ab::names(r.fitted)) } else { String::new() }
        ));
    }
    if r.new != 0 {
        log(&format!("abilities: learned at Re:CoM level {level} (unequipped, KH1's rule): {:?}", ab::names(r.new)));
    }
    if r.story_on | r.story_off != 0 {
        log(&format!("abilities: from the ini: on {:?}, off {:?}", ab::names(r.story_on), ab::names(r.story_off)));
    }
    if !LOGGED_FIRST.swap(true, Ordering::Relaxed) {
        log(&format!(
            "abilities: level {level}, AP {}/{} left; unlocked {:?}; equipped {:?}; modifiers the pickers get {:?}",
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
/// Each KH1 frame: one 15-byte read of the block; a block older than v2 is migrated by [`sync`]
/// once.
pub fn equipped_live() -> Option<u32> {
    if ALL.load(Ordering::Relaxed) {
        return Some(usable(ab::ALL_BITS & !specials()));
    }
    // The KH1 frame runs only for Sora at the KH1 setting (never in Riku's story), so no story
    // test here: one atomic load, one pointer read and the block.
    if !super::movement::kh1_on() {
        return None;
    }
    let save = save()?;
    // SAFETY: inside the 0x2AD4-byte save block.
    let block: [u8; ab::BLOCK_LEN] = unsafe { rd(save + ab::BLOCK_FIRST) };
    let e = if ab::version(&block) < ab::BLOCK_V2 { sync()?.equipped } else { ab::read(&block).equipped };
    Some(usable(e))
}

fn specials() -> u32 {
    ab::SPECIALS.fold(0, |b, i| b | 1 << i)
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
