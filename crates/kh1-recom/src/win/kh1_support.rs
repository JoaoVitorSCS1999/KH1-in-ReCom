//! KH1's Rod-list support abilities at Combat = KH1, Sora's story (M12 step 3; rules in
//! `crate::kh1_support`, which ones are equipped in `crate::abilities`, notes in
//! `docs/recom-m9-abilities.md` § M12 step 3).
//!
//! Nothing of ours per frame: each part runs inside a wrapper that already exists, and only when
//! something happens there (a hit reaching Sora in Re:CoM's player damage check `0x1401E6DE0`).
//!
//! - **MP Rage** ([`after_damage`]): after Re:CoM's damage check took HP, KH1's charge for the HP
//!   lost (`0x14029BBD0`), through KH1's gauge add (`super::mp::gain_charge`, `0x1402A4AE0`).
//! - **Berserk** ([`swing_started`], [`berserk_bonus`], [`berserk_recom`]): KH1's +4 Strength on
//!   Strength-source records while Sora's HP is low, tested at the swing's start (KH1 builds the
//!   attack object then); at Damage = KH1 in KH1's damage, at Damage = Re:CoM as the share it gives
//!   on the hit's Re:CoM value (owner).
//! - **Leaf Bracer** ([`leaf_bracer`]): while Sora's KH1 Cure casts, the damage check runs with
//!   Re:CoM's own protection bit `player+0x418 & 1` (its exit zeroes the hit: no damage, no
//!   reaction), as the summons' invincibility does; KH1 sets its flag 0x80 for the cast
//!   (`0x14028F140` → `0x140297F30(Sora, 0)`).

use super::movement::rd;
use super::{log, mp};
use crate::abilities as ab;
use crate::kh1_support as ks;
use std::sync::atomic::{AtomicU32, Ordering};

/// Sora's HP and max HP (`player+0x42C` / `+0x430`; the damage check's own fields). Checked
/// (decomp `0x1401E6DE0`, disasm `0x1401E71AB` / `0x1401E7241`).
const HP: usize = 0x42C;
const MAX_HP: usize = 0x430;

static LINES: AtomicU32 = AtomicU32::new(40);

fn note(line: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("support: {}", line()));
    }
}

/// Sora's HP before Re:CoM's damage check, for [`after_damage`] (the wrapper reads it only when a
/// hit arrived).
pub unsafe fn hp(pl: usize) -> i32 {
    rd(pl + HP)
}

/// After Re:CoM's damage check (`win::attack::damage_check`, a hit having arrived), at Combat =
/// KH1: MP Rage's charge for the HP the check took.
pub unsafe fn after_damage(pl: usize, hp_before: i32) {
    let after: i32 = rd(pl + HP);
    if after >= hp_before || !mp::active() {
        return;
    }
    let Some(e) = super::abilities::equipped_live() else { return };
    let copies = ab::mp_rage(e);
    if copies == 0 {
        return;
    }
    let max: i32 = rd(pl + MAX_HP);
    let level = super::stats::sora_level();
    let units = ks::mp_rage::units(hp_before, after, max, level, copies);
    let kh1_lost = ks::mp_rage::kh1_hp_lost(hp_before - after.max(0), max, level);
    note(|| format!("MP Rage x{copies}: HP {hp_before} -> {after} of {max} = {kh1_lost} of KH1's {} at level {level}: charge +{units}", ks::mp_rage::kh1_max_hp(level)));
    if units > 0 {
        mp::gain_charge(units, &format!("MP Rage x{copies}, {kh1_lost} KH1 HP lost"));
    }
}

static BRACER_LINES: AtomicU32 = AtomicU32::new(20);

/// Leaf Bracer covers this hit: Sora's KH1 Cure is casting (`super::magic::kh1_cure_casting`:
/// Re:CoM's magic action with a KH1 Cure row) and Leaf Bracer is equipped. KH1: the list start
/// `0x14028F140` for list 3 (Cure: spell index 3, its cast motion `0x36 + 3` / `0x83 + 3`) cast by
/// the actor on itself, with `+0x184` bit 6, calls `0x140297F30(actor, 0)`: flag 0x80 with a zero
/// timer, which the actor tick (`0x140297FE0`, its `0x1402984BA` branch) clears the frame after the cast
/// motion's lock (flag 4, set by `0x14029F980` with 0x1000000) is released. While it is set, the
/// damage function `0x14029BBD0` / `0x1402BEEE0` takes no HP and starts no reaction (but for the
/// no-damage record kinds 6 / 7). Checked (decomp). Ours ends with Re:CoM's magic action (the
/// cast motion's end): ±1 frame (Assumption).
pub unsafe fn leaf_bracer(pl: usize) -> bool {
    super::magic::kh1_cure_casting(pl) && super::abilities::equipped_live().is_some_and(ab::leaf_bracer)
}

/// A hit Leaf Bracer took (logged, 20 lines).
pub fn bracer_hit(value: i16) {
    if BRACER_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("support: Leaf Bracer: a hit of {value} on Sora during his Cure: no damage, no reaction (KH1's flag 0x80)"));
    }
}

/// The Berserk test of the swing that started last: `(KH1 row + 1) << 1 | on`. KH1 makes the swing's
/// attack object (and adds Berserk's Strength, `0x1402B2DC0`) at its start, so a swing keeps the
/// state it began with.
static SWING: AtomicU32 = AtomicU32::new(0);
static BERSERK_LINES: AtomicU32 = AtomicU32::new(30);

/// Berserk is on now: Sora's HP in KH1's low-HP state (`crate::kh1_support::berserk::low_hp` on
/// Re:CoM's HP and max HP) and Berserk equipped.
unsafe fn berserk_now(pl: usize) -> bool {
    ks::berserk::low_hp(rd(pl + HP), rd(pl + MAX_HP)) && mp::active() && super::abilities::equipped_live().is_some_and(ab::berserk)
}

/// A KH1 swing row starts (`win::attack::apply_row`): KH1's moment for Berserk's test.
pub unsafe fn swing_started(pl: usize, kh1_row: usize) {
    let on = berserk_now(pl);
    SWING.store(((kh1_row as u32 + 1) << 1) | on as u32, Ordering::Relaxed);
}

/// The Strength Berserk adds to a hit of `kh1_row` (KH1's G`+0xFE` = 4, or 0): the state its swing
/// started with, else (a row that didn't start through [`swing_started`], e.g. a special's dash)
/// the state now.
pub unsafe fn berserk_bonus(pl: usize, kh1_row: usize) -> i32 {
    let s = SWING.load(Ordering::Relaxed);
    let on = if s >> 1 == kh1_row as u32 + 1 { s & 1 != 0 } else { berserk_now(pl) };
    if on {
        ks::berserk::STRENGTH
    } else {
        0
    }
}

/// Damage = Re:CoM (owner, 2026-11-03: "a % damage bonus (the share KH1's +4 Strength gives at that
/// level)"): the hit part's Re:CoM value (`+0x92`, before Re:CoM's resistance, weak part and boss
/// falloff) × KH1's damage with Berserk ÷ without, for this row on this enemy.
pub unsafe fn berserk_recom(enemy: usize, part: usize, kh1_row: usize, hp_now: i32, max_mp: i32) {
    let Some((with, without)) = super::stats::berserk_share(enemy, kh1_row, super::stats::sora_level(), hp_now, max_mp) else { return };
    let v: i16 = rd(part + 0x92);
    let n = ks::berserk::scale(v, with, without);
    if n != v {
        super::movement::wr(part + 0x92, n);
    }
    if BERSERK_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("support: Berserk (Damage = Re:CoM): row {kh1_row:#x} on {:#x}: KH1 {with} / {without} with / without +4 Strength: Re:CoM's {v} -> {n}", enemy & 0x00FF_FFFF));
    }
}

/// The log line for a Berserk hit at Damage = KH1 (30 lines shared with [`berserk_recom`]).
pub fn berserk_kh1_line(enemy: usize, kh1_row: usize, d: Option<i32>) {
    if BERSERK_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("support: Berserk (Damage = KH1): row {kh1_row:#x} on {:#x}: KH1's damage with Strength + 4 = {d:?}", enemy & 0x00FF_FFFF));
    }
}
