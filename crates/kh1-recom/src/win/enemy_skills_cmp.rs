//! Enemy Skills stage 4: Re:CoM's direct compares for Axel, Larxene, Vexen, Platinum Card and Ansem
//! (`crate::enemy_skills_cmp`, `docs/recom-enemy-skills.md` § "Stage 4 (built)").
//!
//! - **Install** ([`install`], once, after Second Chance's stub: Platinum's first site sits in that
//!   stub's checked context): every site's bytes and context checked (`crate::enemy_skills_cmp::PATCH`
//!   / `CALLS` / `CHECKED`), each stub placed near its site, the site replaced by a jump to it; the
//!   Auto-Life count call retargeted to [`revive_count`]. A skill whose sites aren't all in stays inert
//!   (logged).
//! - **Arm** ([`battle_start`], from the battle manager's ctor wrapper, once per battle): the stubs'
//!   bytes ([`ARMED`]) from the equipped skills at Combat = KH1 in Sora's story (Black Fungus's skill
//!   for the battle too, `super::enemy_skills_deck::bonus_bits`); Platinum's 5 hits and
//!   Auto-Life's one revive start again. Nothing of ours per frame: the stubs run inside Re:CoM's
//!   code only when it reaches the compares (a hit, Sora's stick class, a KO).
//! - **Dash in KH1's run** ([`dash_on`]): `super::movement::stick_class` multiplies KH1's run by the
//!   same × 1.25 in battle.

use super::movement::{func, rd};
use super::log;
use crate::enemy_skills_cmp::{self as ec, card, Armed};
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

/// What the stubs read (`crate::enemy_skills_cmp::Armed`, its address in every stub).
pub static ARMED: Armed = Armed::new();
/// Bits of the cards whose stubs are all in (bit `card − 0x40`).
static PLACED: AtomicU32 = AtomicU32::new(0);
/// The Auto-Life revives this battle (for the log), Platinum's hits armed this battle.
static REVIVED: AtomicU8 = AtomicU8::new(0);
static PLATINUM_ARMED: AtomicU8 = AtomicU8::new(0);
static LINES: AtomicU32 = AtomicU32::new(60);

fn note(line: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("enemy skills: {}", line()));
    }
}

fn bit(c: u8) -> u32 {
    1 << (c - 0x40)
}

/// Re:CoM's enemy card count (`0x14015E010(side, kind, effect)`, `CHECKED`).
const COUNT: u32 = 0x0015_E010;
type CountFn = unsafe extern "win64" fn(i32, i32, i32);

/// Places every stub and the revive call (bytes checked first; a skill with a site that fails stays
/// inert).
pub unsafe fn install(bs: usize) {
    for c in ec::PATCH.iter().chain(ec::CALLS).chain(ec::CHECKED) {
        if std::slice::from_raw_parts((bs + c.rva as usize) as *const u8, c.expect.len()) != c.expect {
            log(&format!("enemy skills: {} is not as expected: Quick Recovery, Dash, Auto-Life, Invincible and Sleightblind do nothing", c.name));
            return;
        }
    }
    let armed = &ARMED as *const Armed as u64;
    let mut failed = 0u32;
    for (i, s) in ec::PATCH.iter().enumerate() {
        let c = ec::STUBS[i].1;
        let at = bs + s.rva as usize;
        let Some(len) = ec::stub(i, ec::Place { at: at as u64 - 0x1000_0000, armed, base: bs as u64 }).map(|v| v.len()) else {
            failed |= bit(c);
            continue;
        };
        let filler = vec![0xCCu8; len];
        let Some(block) = super::hook::place_near(at, &filler) else {
            log(&format!("enemy skills: no memory near {}", s.name));
            failed |= bit(c);
            continue;
        };
        let (Some(code), Some(entry)) = (ec::stub(i, ec::Place { at: block as u64, armed, base: bs as u64 }), crate::hookplan::entry_patch(at as u64, s.expect.len(), block as u64)) else {
            log(&format!("enemy skills: {} is out of the stub's reach", s.name));
            failed |= bit(c);
            continue;
        };
        if !super::hook::patch(block, &filler, &code, "enemy skill stub") || !super::hook::patch(at, s.expect, &entry, s.name) {
            failed |= bit(c);
        }
    }
    // Auto-Life's count call at the revive → our callback (a thunk near it).
    let s = &ec::CALLS[0];
    let at = bs + s.rva as usize;
    let mut jmp = vec![0xFF, 0x25, 0, 0, 0, 0];
    jmp.extend_from_slice(&(revive_count as *const () as u64).to_le_bytes());
    let call = super::hook::place_near(at, &jmp).and_then(|t| crate::hookplan::call_rel32(at as u64, t as u64));
    if !call.is_some_and(|c| super::hook::patch(at, s.expect, &c, s.name)) {
        failed |= bit(card::VEXEN);
    }
    let all = bit(card::AXEL) | bit(card::LARXENE) | bit(card::VEXEN) | bit(card::PLATINUM) | bit(card::ANSEM);
    PLACED.store(all & !failed, Ordering::Release);
    log(&format!(
        "enemy skills: Re:CoM's compares for Quick Recovery (0x1401e73a8), Dash (0x1401e9672), Auto-Life (0x1401f1a85, 0x140211032, 0x1401a1a13), Invincible (0x1401e71f3, 0x1401f9129) and Sleightblind's resistance (0x1401e7150) answer for the equipped skills{}",
        if failed == 0 { String::new() } else { format!("; NOT placed (inert): {:#x}", failed) }
    ));
}

/// The battle manager ctor (once per battle): the stubs' bytes for this battle. `kh1`: Combat = KH1
/// in Sora's story.
pub fn battle_start(kh1: bool) {
    let left = ARMED.platinum_left.load(Ordering::Relaxed);
    if PLATINUM_ARMED.swap(0, Ordering::Relaxed) != 0 {
        note(|| format!("last battle: Invincible kept the HP of {} hit(s)", ec::PLATINUM_HITS - left.max(0)));
    }
    if REVIVED.swap(0, Ordering::Relaxed) != 0 {
        note(|| "last battle: Auto-Life revived Sora".into());
    }
    let equipped = if kh1 { super::abilities::equipped_live().unwrap_or(0) | super::enemy_skills_deck::bonus_bits() } else { 0 };
    let has = |c: u8| crate::enemy_skills::of_card(c as u16).is_some_and(|s| equipped & (1 << crate::enemy_skills::ability_of(s)) != 0);
    let placed = PLACED.load(Ordering::Acquire);
    let a = ec::arm(kh1, &has, &|c| placed & bit(c) != 0);
    for (i, f) in a.flags.iter().enumerate() {
        ARMED.flags[i].store(*f, Ordering::Relaxed);
    }
    ARMED.platinum_left.store(a.platinum_left, Ordering::Relaxed);
    PLATINUM_ARMED.store((a.platinum_left > 0) as u8, Ordering::Relaxed);
    if a != ec::Arming::default() {
        let names = [(a.flags[ec::ix::AXEL], "Quick Recovery"), (a.flags[ec::ix::LARXENE], "Dash x1.25"), (a.flags[ec::ix::VEXEN], "Auto-Life (once)"), ((a.platinum_left > 0) as u8, "Invincible (5 hits)"), (a.flags[ec::ix::ANSEM], "Sleightblind (Fire / Blizzard / Thunder 50 %)")];
        note(|| format!("battle start: armed {}", names.iter().filter(|n| n.0 != 0).map(|n| n.1).collect::<Vec<_>>().join(", ")));
    }
}

/// Dash is armed for this battle (KH1's run in `super::movement::stick_class`, Re:CoM's card battle
/// object set: battle only, as Re:CoM's own test).
pub unsafe fn dash_on(pl: usize) -> bool {
    ARMED.flags[ec::ix::LARXENE].load(Ordering::Relaxed) != 0 && rd::<usize>(pl + CARD_BATTLE) != 0
}

/// Re:CoM's card battle object (`player+0x3E8`). Checked (decomp `0x1401E9630`).
const CARD_BATTLE: usize = 0x3E8;

/// The retargeted count call at Re:CoM's revive (`0x1401EC369`, `0x14015E010(1, 0x28, 0x2E)`):
/// Auto-Life is used for this battle (its slot reads answer the real slot again), then Re:CoM's
/// count (which touches only the real slot).
unsafe extern "win64" fn revive_count(side: i32, kind: i32, effect: i32) {
    if ARMED.flags[ec::ix::VEXEN].swap(0, Ordering::Relaxed) != 0 {
        REVIVED.store(1, Ordering::Relaxed);
        note(|| "Auto-Life: Re:CoM's revive (max HP / 4, 1 s untouchable); used for this battle".into());
    }
    func::<CountFn>(COUNT)(side, kind, effect);
}
