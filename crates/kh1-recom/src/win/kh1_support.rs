//! KH1's Rod-list support abilities at Combat = KH1, Sora's story (M12 step 3; rules in
//! `crate::kh1_support`, which ones are equipped in `crate::abilities`, notes in
//! `docs/recom-m9-abilities.md` § M12 step 3).
//!
//! Nothing of ours per frame: each part runs inside a wrapper that already exists, and only when
//! something happens there (a hit reaching Sora in Re:CoM's player damage check `0x1401E6DE0`).
//!
//! - **MP Rage** ([`after_damage`]): after Re:CoM's damage check took HP, KH1's charge for the HP
//!   lost (`0x14029BBD0`), through KH1's gauge add (`super::mp::gain_charge`, `0x1402A4AE0`).
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
