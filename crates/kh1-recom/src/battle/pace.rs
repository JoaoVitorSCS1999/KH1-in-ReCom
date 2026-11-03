//! Re:CoM-AI Heartless wait their own attack-end cooldown in a KH1 battle (owner, 2026-11-02,
//! "Enemy pace, Re:CoM-AI Heartless"; `docs/recom-m6-battles.md` § Attack-end cooldown). The
//! runtime is `win/battle.rs` (`attack_end`).
//!
//! Re:CoM's attack end `0x14017E540(enemy, next, cooldown)` (Checked (decomp + exe disasm)):
//! unless enchant 0x72 repeats the attack (an early return that keeps bit 0), it ends the card
//! (`0x14017E1D0`), clears bit 0, writes `cooldown` to `+0x2C8` and, with `next` set, makes
//! `next` the next state. With `next == 0` it **overwrites the cooldown with 250**
//! (`mov dword [rbx+0x2C8], 0xFA` at `0x14017E5F5`) and goes to the class's default state
//! (`+0x2F8`, think on: bit 0x10). Every one of its 198 call sites passes `next = 0`, so Re:CoM
//! always discards the value passed: 197 sites pass 6000, one (`0x1403FBA20`, Marluxia 2's
//! shared attack tail) passes 0. Checked (exe: every call site disassembled).
//!
//! The enemy update `0x140185320` counts `+0x2C8` down by the task's step `+0x14` (50 a frame at
//! 60 Hz, `docs/recom-timing.md`) and runs the think only while it is < 1. Checked (decomp).
//!
//! At Combat = KH1 in Sora's story, for the ordinary Heartless classes below while their
//! instances still run Re:CoM's AI (Re:CoM's own vtable; our KH1-AI enemies run a copy of it),
//! the wrapper writes the passed value back after Re:CoM's attack end. Bosses, the 100 Acre Wood
//! bee and our KH1-AI enemies keep Re:CoM's 250.

/// Re:CoM's cooldown at every attack end with no next state (`0x14017E5F5`).
pub const RECOM_COOLDOWN: i32 = 250;
/// Re:CoM time units a frame (`docs/recom-timing.md`, Checked).
pub const TICKS_PER_FRAME: i32 = 50;
/// What all 32 classes pass (`mov r8d, 0x1770` before each of their calls). Checked (exe).
pub const PASSED: i32 = 6000;

/// `+0x260` bit 0: in an attack (`crate::battle::bits::PLAYING`); the attack end clears it
/// except on its enchant-0x72 early return.
const PLAYING: u64 = crate::battle::bits::PLAYING;

/// One ordinary Heartless class whose attack states call the attack end.
#[derive(Debug)]
pub struct Heartless {
    pub rtti: &'static str,
    /// Re:CoM's vtable (RVA). Checked (exe: RTTI complete-object locator at `vt − 8`).
    pub vtable: u32,
}

/// The 32 `CEnemy*` classes whose attack-state steps call `0x14017E540` (vtable slots `+0x120`,
/// `+0x138`, and `+0x150` for the Loudness and the Wizard), each passing (0, 6000). Checked
/// (exe: the call sites' functions found in these vtables). Not listed: `CEnemyWhitemushroom`
/// (never calls it), the bosses, `CEnemy100AcreBB_Bee` (100 Acre Wood, not a Heartless).
pub static HEARTLESS: [Heartless; 32] = [
    Heartless { rtti: ".?AVCEnemyAirpirate@@", vtable: 0x0053_09E8 },
    Heartless { rtti: ".?AVCEnemyAirsoldier@@", vtable: 0x0053_1CC8 },
    Heartless { rtti: ".?AVCEnemyAquatank@@", vtable: 0x0053_2FE8 },
    Heartless { rtti: ".?AVCEnemyBandit@@", vtable: 0x0053_4368 },
    Heartless { rtti: ".?AVCEnemyBarrelspider@@", vtable: 0x0053_5688 },
    Heartless { rtti: ".?AVCEnemyBlackfungus@@", vtable: 0x0053_69C8 },
    Heartless { rtti: ".?AVCEnemyBluerhapsody@@", vtable: 0x0053_7CA8 },
    Heartless { rtti: ".?AVCEnemyBouncywild@@", vtable: 0x0053_8FB8 },
    Heartless { rtti: ".?AVCEnemyCreapplant@@", vtable: 0x0053_A348 },
    Heartless { rtti: ".?AVCEnemyDarkball@@", vtable: 0x0053_B7A8 },
    Heartless { rtti: ".?AVCEnemyDefender@@", vtable: 0x0053_CA98 },
    Heartless { rtti: ".?AVCEnemyFatbandit@@", vtable: 0x0053_DDF8 },
    Heartless { rtti: ".?AVCEnemyGargoyle@@", vtable: 0x0053_F198 },
    Heartless { rtti: ".?AVCEnemyGreenrequiem@@", vtable: 0x0054_04C8 },
    Heartless { rtti: ".?AVCEnemyHeartsoldier@@", vtable: 0x0054_17D0 },
    Heartless { rtti: ".?AVCEnemyLargebody@@", vtable: 0x0054_2AD0 },
    Heartless { rtti: ".?AVCEnemyLoudness@@", vtable: 0x0054_3DE8 },
    Heartless { rtti: ".?AVCEnemyNeoshadow@@", vtable: 0x0054_5128 },
    Heartless { rtti: ".?AVCEnemyPirate@@", vtable: 0x0054_6458 },
    Heartless { rtti: ".?AVCEnemyPowerwild@@", vtable: 0x0054_7798 },
    Heartless { rtti: ".?AVCEnemyRednocturne@@", vtable: 0x0054_8A58 },
    Heartless { rtti: ".?AVCEnemyScrewdiver@@", vtable: 0x0054_9E18 },
    Heartless { rtti: ".?AVCEnemySeaneon@@", vtable: 0x0054_B0E8 },
    Heartless { rtti: ".?AVCEnemySearchghost@@", vtable: 0x0054_C408 },
    Heartless { rtti: ".?AVCEnemyShadow@@", vtable: 0x0054_D728 },
    Heartless { rtti: ".?AVCEnemySoldier@@", vtable: 0x0054_EA68 },
    Heartless { rtti: ".?AVCEnemySpadesoldier@@", vtable: 0x0054_FD90 },
    Heartless { rtti: ".?AVCEnemyTornadostep@@", vtable: 0x0055_1098 },
    Heartless { rtti: ".?AVCEnemyWightnight@@", vtable: 0x0055_3748 },
    Heartless { rtti: ".?AVCEnemyWizard@@", vtable: 0x0055_4A58 },
    Heartless { rtti: ".?AVCEnemyWyvern@@", vtable: 0x0055_5DC8 },
    Heartless { rtti: ".?AVCEnemyYellowopera@@", vtable: 0x0055_70C8 },
];

/// The listed class whose Re:CoM vtable is at `vtable_rva` (an instance's vtable minus the image
/// base). Our KH1-AI copies live elsewhere, so they are never found.
pub fn heartless(vtable_rva: u64) -> Option<usize> {
    HEARTLESS.iter().position(|h| h.vtable as u64 == vtable_rva)
}

/// One call of Re:CoM's attack end, as the wrapper saw it.
#[derive(Clone, Copy, Debug)]
pub struct End {
    /// This battle is a KH1 one (Combat = KH1, Sora's story).
    pub kh1_battle: bool,
    /// The enemy's class is a listed Heartless on Re:CoM's own vtable.
    pub heartless: bool,
    /// The arguments.
    pub next: usize,
    pub passed: i32,
    /// `+0x260` and `+0x2C8` after Re:CoM's attack end returned.
    pub flags_after: u64,
    pub cooldown_after: i32,
}

/// The cooldown to write after Re:CoM's attack end, or None to leave Re:CoM's. Only when
/// Re:CoM took its "no next state" path (bit 0 cleared, 250 written) and the class passed a
/// longer wait than Re:CoM's.
pub fn kept(e: &End) -> Option<i32> {
    let recom_path = e.next == 0 && e.flags_after & PLAYING == 0 && e.cooldown_after == RECOM_COOLDOWN;
    (e.kh1_battle && e.heartless && recom_path && e.passed > RECOM_COOLDOWN).then_some(e.passed)
}

/// Frames of a cooldown in Re:CoM time units.
pub fn frames(ticks: i32) -> f32 {
    ticks as f32 / TICKS_PER_FRAME as f32
}

/// The once-per-class log line.
pub fn class_line(passed: i32) -> String {
    format!(
        "waits its own attack-end cooldown at KH1: {passed} ticks ({:.0} frames) instead of Re:CoM's {RECOM_COOLDOWN} ({:.0} frames)",
        frames(passed),
        frames(RECOM_COOLDOWN)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end() -> End {
        End { kh1_battle: true, heartless: true, next: 0, passed: PASSED, flags_after: 0x10, cooldown_after: RECOM_COOLDOWN }
    }

    #[test]
    fn fat_bandit_waits_120_frames() {
        assert_eq!(kept(&end()), Some(6000));
        assert_eq!(frames(PASSED), 120.0);
        assert_eq!(frames(RECOM_COOLDOWN), 5.0);
        let i = heartless(0x0053_DDF8).unwrap();
        assert_eq!(HEARTLESS[i].rtti, ".?AVCEnemyFatbandit@@");
    }

    #[test]
    fn recom_is_left_alone() {
        // Combat = Re:CoM or Riku's story.
        assert_eq!(kept(&End { kh1_battle: false, ..end() }), None);
        // A boss, our KH1-AI copy, anything unlisted.
        assert_eq!(kept(&End { heartless: false, ..end() }), None);
        // A next state given: Re:CoM already keeps the value.
        assert_eq!(kept(&End { next: 0x1234, ..end() }), None);
        // Enchant 0x72's early return: bit 0 still set, nothing written.
        assert_eq!(kept(&End { flags_after: 0x11, cooldown_after: -40, ..end() }), None);
        // Something else wrote the cooldown.
        assert_eq!(kept(&End { cooldown_after: 0, ..end() }), None);
        // Never shorter than Re:CoM's (our KH1 enemies pass 0; Marluxia 2 passes 0).
        assert_eq!(kept(&End { passed: 0, ..end() }), None);
        assert_eq!(kept(&End { passed: RECOM_COOLDOWN, ..end() }), None);
    }

    #[test]
    fn table_is_the_heartless_with_an_attack_end() {
        let mut v: Vec<u32> = HEARTLESS.iter().map(|h| h.vtable).collect();
        v.sort();
        v.dedup();
        assert_eq!(v.len(), HEARTLESS.len());
        assert!(HEARTLESS.iter().all(|h| h.rtti.starts_with(".?AVCEnemy") && h.rtti.ends_with("@@")));
        assert!(heartless(0x0054_17D0).is_some());
        // Not the White Mushroom (no attack end), not the bee, not a boss (Guard Armor).
        assert!(heartless(0x0055_23F8).is_none());
        assert!(heartless(0x0068_5CA8).is_none());
        assert!(heartless(0x0065_F4B8).is_none());
        // Every class we port to KH1's AI is one of them (on Re:CoM's AI until installed).
        for c in crate::kh1_enemy::CLASSES.iter() {
            let i = heartless(c.vtable as u64).unwrap_or_else(|| panic!("{}", c.name));
            assert_eq!(HEARTLESS[i].rtti, c.rtti, "{}", c.name);
        }
    }

    #[test]
    fn line() {
        assert_eq!(class_line(6000), "waits its own attack-end cooldown at KH1: 6000 ticks (120 frames) instead of Re:CoM's 250 (5 frames)");
    }
}
