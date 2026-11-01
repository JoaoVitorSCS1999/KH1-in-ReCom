//! M7 stage 7.11, spell unlocks (decision 34; `crate::unlocks`, `docs/recom-m7-unlocks.md`).
//!
//! - **The wrapper** on Re:CoM's "first obtained" `0x14015C7F0(id)` (the collection add
//!   `0x140158FC0` → `0x14015CDB0` calls it after the discovery step; it turns a card id's state
//!   from 1 to 2 the first time): after Re:CoM's own function, a magic card or a table boss's enemy
//!   card syncs Sora's spell levels from Re:CoM's discovery state ([`sync`]). In Sora's story, at
//!   either Combat setting (it writes only our save block, which Re:CoM never reads).
//! - **Older saves** (and saves played without the mod or at the Re:CoM setting before): the first
//!   read of the spell levels at the KH1 setting (`super::mp::spell_levels`, the command menu's)
//!   runs the same sync once, then marks the block (`crate::unlocks::SYNCED`).
//! - Each learn is logged. Nothing of ours runs per frame (one byte test in a read the menu already
//!   makes).

use super::movement::{orig, rd, wr};
use super::{base, log, va};
use crate::mp::{off, BLOCK_FIRST, BLOCK_LEN};
use crate::unlocks as u;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub static ORIG_FIRST_OBTAINED: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(60);

/// Sora's card collection (`save + 0x1A74`, set by `0x140158F20`), read by `0x14015B3F0`. Checked
/// (exe).
const COLLECTION: u64 = 0x1_4087_A7E8;
const COLLECTION_IN_SAVE: usize = 0x1A74;

type IdFn = unsafe extern "win64" fn(u64);

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("unlocks: {}", line()));
    }
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

pub fn start_line() -> String {
    format!(
        "unlocks: KH1 spell unlocks in Sora's story (M7 stage 7.11): a spell's base level with its Re:CoM magic card's first pickup, -ra / -ga with {} Re:CoM bosses (their enemy cards' first pickup, two per spell), wrapper on Re:CoM's first-obtained 0x14015C7F0; levels at save+{:#x} (7 bytes), sync flag save+{:#x} bit 0",
        u::BOSS_LEARNS.iter().map(|b| b.card).collect::<std::collections::BTreeSet<_>>().len(),
        BLOCK_FIRST + off::SPELLS,
        BLOCK_FIRST + off::SPARE
    )
}

/// Re:CoM's `0x14015C7F0(id)` (the id in `cx`; passed through whole), then the unlocks for a card
/// that matters to them.
pub unsafe extern "win64" fn first_obtained(id: u64) {
    let o: IdFn = orig(&ORIG_FIRST_OBTAINED);
    o(id);
    let id = (id & 0xFFFF) as u16;
    if u::tracked(id) {
        sync(&format!("card {id:#x} ({}) obtained for the first time", card_name(id)));
    }
}

fn card_name(id: u16) -> String {
    if let Some(s) = u::spell_of_card(id) {
        return format!("{}{} magic card", u::level_name(s, 1), if id >= u::PREMIUM { " premium" } else { "" });
    }
    u::BOSS_LEARNS.iter().find(|b| b.card == id).map_or_else(|| "?".into(), |b| format!("{} enemy card: {} beaten", b.boss, b.boss))
}

/// The first read of the spell levels at the KH1 setting for a save that was never synced: the
/// sync, once. True when it ran (the block is marked).
pub fn sync_first() -> bool {
    // SAFETY: reads and writes inside Re:CoM's save block, behind the checks in `sync`.
    unsafe { sync("the first read of this save's spells at the KH1 setting (Re:CoM's discovery state)") }
}

/// Sora's spell levels from Re:CoM's discovery state (`crate::unlocks::levels`, Re:CoM's
/// `0x14015B3F0` through `super::keyblades::ever_obtained`), merged into our save block (never
/// lowered) and the block marked synced; each rise logged. Nothing in Riku's story, without a save
/// or a collection, or when the wrapper isn't in.
unsafe fn sync(why: &str) -> bool {
    if !INSTALLED.load(Ordering::Acquire) || super::stats::riku_story() {
        return false;
    }
    let save: usize = rd(va(base(), crate::keyblades::SAVE_PTR));
    let coll: usize = rd(va(base(), COLLECTION));
    if save == 0 || coll != save + COLLECTION_IN_SAVE {
        return false;
    }
    let at = save + BLOCK_FIRST;
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
    let old: [u8; BLOCK_LEN] = rd(at);
    let mut stored = [0u8; u::SPELLS];
    stored.copy_from_slice(&old[off::SPELLS..off::SPELLS + off::SPELL_COUNT]);
    let derived = u::levels(|id| super::keyblades::ever_obtained(id));
    let merged = u::merged(stored, derived);
    let before = stored.map(|l| if l <= 3 { l } else { 0 });
    for (s, from, to) in u::learns(before, merged) {
        let what = if from == 0 { format!("learned {}", u::level_name(s, to)) } else { format!("{} -> {}", u::level_name(s, from), u::level_name(s, to)) };
        note(|| format!("{what} (spell {s} level {from} -> {to}): {why}"));
    }
    let mut block = old;
    block[off::SPELLS..off::SPELLS + off::SPELL_COUNT].copy_from_slice(&merged);
    block[off::SPARE] |= u::SYNCED;
    if block != old {
        wr(at, block);
    }
    true
}
