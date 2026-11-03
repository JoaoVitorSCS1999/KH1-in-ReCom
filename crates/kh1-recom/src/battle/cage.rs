//! The Parasite Cage at Combat = KH1 (owner, 2026-11-03: "the parasite cage shouldnt flinch when
//! you attack, let it go through with its attacks, give it about 3 seconds between each attack";
//! `docs/recom-m6-battles.md` § Parasite Cage). The runtime is `win/battle.rs` (`cage_armour_on` /
//! `cage_armour_off`, `attack_end`) and the enemy-hit wrapper `win::attack::enemy_hit`.
//!
//! Re:CoM (Checked (decomp + exe)):
//! - `CBossParasiteCage` (vtable `0x140669220`) is one enemy body. Its four attack states 2..5
//!   (steps `0x1403C48F0`, `0x1403C4930`, `0x1403C4A10` (the grab and spit), `0x1403C5220`) all end
//!   through Re:CoM's attack end `0x14017E540(cage, 0, 6000)`, which writes 250 over the 6000
//!   (`0x14017E5F5`); its think `0x1403C26F0` runs only while the cooldown `+0x2C8` is below 1.
//! - A hit reaches Re:CoM's hit handler `0x14017F7E0`: the damage, then the cage's own hit callback
//!   (vtable `+0x3E0` = `0x1403C4060`: its faint, state 17, on a Fire hit or a kind-2 hit by the
//!   chances `FireFaint` / `AtkFaint` of its parameter file), then Re:CoM's reaction picker
//!   `0x14017D260`: the flinch (state 11) or the blow-away (state 12), which ends its attack.
//! - No state of its own sets bits 20 / 21 for its attacks: at KH1 (where its hurt bodies stay on
//!   during attacks, decision 26, and our swings carry the card-interrupt bit, decision 14) every
//!   hit flinched it out of its attack.
//!
//! At KH1: Re:CoM's own super armour (`kh1_core::enemy::host::RECOM_ARMOUR`, bits 20 / 21) for each
//! call of the hit handler on the cage, as KH1's no-reaction enemies take it: the damage lands, the
//! picker plays only the damage voice and asks for no state, its attack goes on. Its status
//! reactions (Stop, stun, freeze, Gravity's flatten), its faint, its defeat and the card break
//! (message 3 → state 13) are Re:CoM's. And its attack end keeps [`WAIT`] ticks instead of 250.

/// `CBossParasiteCage`'s vtable (RVA) and RTTI name. Checked (exe: the RTTI locator at `vt − 8`).
pub const VTABLE: u32 = 0x0066_9220;
pub const RTTI: &str = ".?AVCBossParasiteCage@@";

/// About 3 s between its attacks (owner): 180 frames at Re:CoM's 50 time units a frame (the task
/// step `+0x14` the enemy update `0x140185320` counts `+0x2C8` down by: 50 per 60 Hz frame,
/// `docs/recom-timing.md`; Checked (decomp)).
pub const WAIT_FRAMES: i32 = 180;
pub const WAIT: i32 = WAIT_FRAMES * crate::battle::pace::TICKS_PER_FRAME;

/// What its four attack states pass to the attack end (`mov r8d, 0x1770` before each call). Checked (exe).
pub const PASSED: i32 = 6000;

/// The cage's faint: its state 17 (begin `0x1403C2DD0`, step `0x1403C5290`, end `0x1403C3560`),
/// asked by its hit callback `0x1403C4060`. Re:CoM's, kept.
pub const FAINT_STATE: usize = 17;

/// One call of the hit handler on an enemy, as the enemy-hit wrapper sees it before the call.
#[derive(Clone, Copy, Debug)]
pub struct Ask {
    /// This battle is a KH1 one (Combat = KH1, Sora's story).
    pub kh1_battle: bool,
    /// The enemy is the Parasite Cage (Re:CoM's own vtable).
    pub cage: bool,
    /// Sora's Guard blocked it this frame: the block's reaction runs inside this call and stays the
    /// Guard reactions' (as for KH1's no-reaction enemies).
    pub guard_block: bool,
}

/// Whether the call takes Re:CoM's super armour.
pub fn armoured(a: Ask) -> bool {
    a.kh1_battle && a.cage && !a.guard_block
}

/// Bits 20 / 21 on for the call.
pub fn flags_for_call(flags: u64) -> u64 {
    kh1_core::enemy::host::armour_flags(flags)
}

/// After the call: the two bits as they were. The faint's own bit 21 (`0x1403C4060` sets it with
/// state 17) is read only by the picker, which in state 17 takes its bit-19 path (set by the faint's
/// begin) and never reads it; the cage's state 1 begin `0x1403C2E00` clears it. So clearing it here
/// changes nothing. Checked (decomp).
pub fn flags_after(now: u64, before: u64) -> u64 {
    kh1_core::enemy::host::restore_armour(now, before)
}

/// The log line, once per battle.
pub fn armour_line(battle: u32) -> String {
    format!(
        "m6: Parasite Cage (battle {battle}): Sora's hits land on Re:CoM's super armour (+0x260 bits 20 / 21 for each call of the hit handler 0x14017F7E0): damage, the damage voice, no flinch or blow-away, its attack goes on; Stop / stun / freeze / Gravity, its faint (state 17), the card break and the defeat stay Re:CoM's"
    )
}

pub fn wait_line(battle: u32, passed: i32) -> String {
    format!(
        "m6: Parasite Cage (battle {battle}): waits {WAIT} ticks ({WAIT_FRAMES} frames, about 3 s) after its attack before its think may start the next (passed {passed}; Re:CoM writes {})",
        crate::battle::pace::RECOM_COOLDOWN
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask() -> Ask {
        Ask { kh1_battle: true, cage: true, guard_block: false }
    }

    #[test]
    fn only_the_cage_in_a_kh1_battle() {
        assert!(armoured(ask()));
        assert!(!armoured(Ask { kh1_battle: false, ..ask() }));
        assert!(!armoured(Ask { cage: false, ..ask() }));
        // A Guard block keeps the Guard reaction (Re:CoM's picker would only play the voice).
        assert!(!armoured(Ask { guard_block: true, ..ask() }));
    }

    #[test]
    fn bits_on_for_the_call_and_back_after() {
        let before = 0x0000_0000_0200_0011;
        let on = flags_for_call(before);
        assert_eq!(on, before | 0x30_0000);
        // The handler cleared bit 0 (the card-interrupt path) and asked nothing else.
        assert_eq!(flags_after(on & !1, before), before & !1);
        // Bits Re:CoM already had stay.
        let had = before | 0x20_0000;
        assert_eq!(flags_after(flags_for_call(had), had), had);
    }

    #[test]
    fn three_seconds() {
        assert_eq!(WAIT, 9000);
        assert_eq!(crate::battle::pace::frames(WAIT), 180.0);
        assert_eq!(WAIT_FRAMES as f32 / 60.0, 3.0);
        assert!(WAIT > PASSED);
    }
}
