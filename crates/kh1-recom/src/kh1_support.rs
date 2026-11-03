//! KH1's support abilities from the Rod list at the KH1 setting (M12 step 3;
//! `docs/recom-m9-abilities.md` § M12 step 3): the rules, tested here; the glue is
//! `win::kh1_support`. Which are equipped: `crate::abilities` (MP Haste, MP Rage, Leaf Bracer,
//! Berserk, Second Chance; KH1's actor ability block `+0x184`, `0x1402A6380`).
//!
//! Steam addresses (KH1 and Re:CoM images at `0x140000000`). Tags: Checked (decomp / exe / files)
//! or Assumption.

/// MP Rage (KH1 ability 24, `+0x184` bits 1..3 = the equipped copies): KH1's damage function
/// `0x14029BBD0`, after its damage records, `if (hp_after < hp_before && count != 0)
/// 0x1402A4AE0(actor, (int)((float)(hp_before − hp_after) × (float)count))`: the HP lost × the
/// copies, as charge units (30·k make point k). Checked (decomp).
///
/// Sora's HP is Re:CoM's at Combat = KH1 (enemies hit him with Re:CoM's damage; Cure heals Re:CoM's
/// amount, decision 34), several times KH1's (KH1's Sora: 18 HP at level 1, 87 at 76+). So the HP
/// lost is first taken as **the same share of KH1's max HP at Sora's level** ([`kh1_hp_lost`]), the
/// HP KH1's Sora would have lost to a hit of that share; KH1's rule then runs on it. Owner question:
/// this share (built), or Re:CoM's HP lost as it is (several times KH1's MP).
pub mod mp_rage {
    /// KH1's new-game record (btltbl `0x39D8`) byte 0 → record `+5` / `+6` (HP / max HP;
    /// `0x1402852C0`). Checked (decomp + files).
    pub const KH1_START_HP: i32 = 18;
    /// Level-up bonus kind 1: record `+6` (max HP) and `+5` + 3 (`0x1402B5660` case 1). The Dive's
    /// choice changes no HP (`0x140284B70` writes `+7..+0xB`). Checked (decomp + files).
    pub const KH1_HP_UP: i32 = 3;
    /// The kind-1 levels of Sora's bonus list btltbl `0x3AC0` (byte `[L − 2]` for reaching L).
    /// Checked (files).
    pub const KH1_HP_UP_LEVELS: [u32; 23] = [6, 8, 10, 14, 16, 22, 26, 28, 34, 38, 40, 46, 50, 52, 54, 58, 62, 64, 66, 70, 72, 74, 76];

    /// KH1 Sora's max HP at a level (the Rod path; no accessory: Re:CoM has none of KH1's).
    pub fn kh1_max_hp(level: u32) -> i32 {
        KH1_START_HP + KH1_HP_UP * KH1_HP_UP_LEVELS.iter().filter(|&&l| l <= level).count() as i32
    }

    /// The HP KH1's Sora would lose to a hit taking the same share of his max HP: `lost` of
    /// Re:CoM's `recom_max`, as a share of [`kh1_max_hp`], rounded to the nearest point.
    pub fn kh1_hp_lost(lost: i32, recom_max: i32, level: u32) -> i32 {
        if lost <= 0 || recom_max <= 0 {
            return 0;
        }
        let k = kh1_max_hp(level) as i64;
        ((lost as i64 * k * 2 + recom_max as i64) / (recom_max as i64 * 2)) as i32
    }

    /// KH1's charge for a damage call: (float)(HP lost) × (float)copies, truncated
    /// (`0x14029BBD0`); 0 without a copy or without a loss.
    pub fn units(hp_before: i32, hp_after: i32, recom_max: i32, level: u32, copies: u8) -> i32 {
        let copies = copies & 7;
        if hp_after >= hp_before || copies == 0 {
            return 0;
        }
        let lost = kh1_hp_lost(hp_before - hp_after.max(0), recom_max, level);
        (lost as f32 * copies as f32) as i32
    }
}

/// Berserk (KH1 ability 26, `+0x184` bit 5): KH1's attack object `0x1402B2DC0`, for a record whose
/// stat source is 0 (Strength: swings, finishers, the combo modifiers), adds G`+0xFE` (**4**) to its
/// stat term when the owner's HUD gauge is in its low-HP state (`0x140285030(party id)` → slot,
/// `0x140268950(slot)` → `0x14026E180`). The damage `0x1402BFA80` then indexes KH1's damage curve
/// with power + stat − Defence. Checked (decomp + files: G = btltbl + 0x58, `+0xFE` = 4).
///
/// At Damage = Re:CoM (owner, 2026-11-03: "a % damage bonus (the share KH1's +4 Strength gives at
/// that level)"): Re:CoM's damage for the hit × KH1's damage of that hit with the +4 ÷ KH1's damage
/// without it ([`share`]): KH1's formula at Sora's KH1 Strength (his level and Keyblade) against the
/// enemy's KH1 record (twin / proxy, else [`crate::attack::plain_record`]).
pub mod berserk {
    use crate::attack::{kh1_attack_on, EnemyStats, Sora};
    use kh1_core::data::{Moveset, WeaponData};

    /// G`+0xFE`: the Strength Berserk adds. Checked (files).
    pub const STRENGTH: i32 = 4;
    /// `0x14026E180`'s rule: above this max HP the share, else the count.
    pub const MAX_HP_SPLIT: i32 = 0x2C;
    pub const LOW_SHARE: f32 = 0.2;
    pub const LOW_COUNT: i32 = 9;

    /// KH1's low-HP state of a party member's gauge (`0x14026E180`, slot kinds 0..2: Sora is party
    /// slot 0, `save + 0x48E` = 0xFFFFFF00 at a new game, `0x1402852C0`): HP above 0, and with max
    /// HP above 44 HP ÷ max HP below 0.2 (`f32`), else HP below 9. Checked (decomp). On Re:CoM's HP
    /// (Sora's HP at Combat = KH1): Re:CoM's max HP is above 44, so the 20 % rule, which is KH1's
    /// own at Berserk's Rod level 60 (KH1 max HP 66).
    pub fn low_hp(hp: i32, max: i32) -> bool {
        if hp <= 0 {
            return false;
        }
        if MAX_HP_SPLIT < max {
            (hp as f32 / max as f32) < LOW_SHARE
        } else {
            hp < LOW_COUNT
        }
    }

    /// KH1's damage of this hit with and without Berserk's Strength (no critical: the share of the
    /// Strength alone), or None when Berserk changes nothing (a record with another stat source,
    /// or no row).
    pub fn share(m: &Moveset, kh1_row: usize, sora: Sora, weapon: &WeaponData, record: &EnemyStats, hp_now: i32) -> Option<(i32, i32)> {
        let rw = m.row(kh1_row)?;
        if m.attack(rw.attack)?.stat_source != 0 {
            return None;
        }
        let curve = kh1_core::DamageCurve::kh1_defaults();
        let mut rng = kh1_core::Kh1Rand::new(1);
        let mut d = |bonus: i32| -> Option<i32> {
            let (mut live, def) = kh1_attack_on(m, kh1_row, sora, bonus, weapon, record, hp_now)?;
            live.crit_chance = 0.0;
            Some(kh1_core::damage::compute_damage(&live, &curve, &def, &mut rng).damage)
        };
        let (with, without) = (d(STRENGTH)?, d(0)?);
        (without > 0).then_some((with, without))
    }

    /// Re:CoM's hit value × with ÷ without (truncated), within Re:CoM's i16 hit value.
    pub fn scale(value: i16, with: i32, without: i32) -> i16 {
        if without <= 0 {
            return value;
        }
        (value as i64 * with as i64 / without as i64).clamp(0, i16::MAX as i64) as i16
    }
}

/// Second Chance (KH1 ability 25, `+0x184` bit 4): KH1's damage `0x1402BFA80`, last step of its
/// ordinary branch: `if (target +0x184 & 0x10 && target HP <= damage) damage = HP − 1;` then `damage
/// == 0 ? 1 : damage`. So a hit that would kill from above 1 HP leaves exactly 1 HP; at 1 HP the
/// damage is 0 → 1 and kills; each hit (each damage record) is its own test, so the next hit of a
/// combo kills. Its % of HP (element 8) and power-as-damage (9 / 11 / 12) branches return before it.
/// Checked (decomp).
///
/// Re:CoM's player damage check `0x1401E6DE0` already has this rule for one of its enemy cards
/// (`0x1401E720C`: with the side's card 0x6C, new HP < 1 and old HP > 1 → HP 1, Checked (disasm)).
/// Ours adds Second Chance to the same place: the check's HP store (`0x1401E7238`: `test ebx,ebx;
/// mov r12d,r15d; cmovns r12d,ebx; mov [rdi+0x42C],r12d`) goes through [`stub`], which sets the new
/// HP `ebx` to 1 when Sora (`rdi`) is armed, `ebx` < 1, the HP before (`r8d`) > 1 and the reaction
/// kind (`r13d`) is not 0x22 (Re:CoM's "take all HP", which skips the damage scale, as KH1's raw
/// branches skip Second Chance: Assumption), then runs the four instructions and jumps back.
pub mod second_chance {
    /// Re:CoM's reaction kind whose damage is the whole HP (`cmp r13d,0x22; cmovne` at
    /// `0x1401E71C9`). Checked (disasm).
    pub const TAKE_ALL_HP: i32 = 0x22;

    /// The rule as the stub runs it: the new HP for `old` HP and Re:CoM's unclamped `new` HP.
    pub fn new_hp(armed: bool, old: i32, new: i32, kind: i32) -> i32 {
        if armed && new < 1 && old > 1 && kind != TAKE_ALL_HP {
            1
        } else {
            new
        }
    }

    /// The four instructions the patch moves (Re:CoM's HP store at `0x1401E7238`).
    pub const STORE: [u8; 16] = [0x85, 0xDB, 0x45, 0x8B, 0xE7, 0x44, 0x0F, 0x49, 0xE3, 0x44, 0x89, 0xA7, 0x2C, 0x04, 0x00, 0x00];

    /// The stub at `at`: `armed` is the address of a `{ u64 player, u32 count }` pair (the armed
    /// player; the count of hits it saved), `back` the instruction after the store (`0x1401E7248`).
    /// 62 bytes; rax is free there (the next instruction is a call). None if `back` is out of reach.
    pub fn stub(armed: u64, at: u64, back: u64) -> Option<Vec<u8>> {
        let mut v = Vec::with_capacity(64);
        v.extend_from_slice(&[0x48, 0xB8]); // mov rax, armed
        v.extend_from_slice(&armed.to_le_bytes());
        v.extend_from_slice(&[0x48, 0x3B, 0x38]); // cmp rdi, [rax]
        v.extend_from_slice(&[0x75, 26]); // jne store
        v.extend_from_slice(&[0x83, 0xFB, 0x01]); // cmp ebx, 1
        v.extend_from_slice(&[0x7D, 21]); // jge store
        v.extend_from_slice(&[0x41, 0x83, 0xF8, 0x01]); // cmp r8d, 1
        v.extend_from_slice(&[0x7E, 15]); // jle store
        v.extend_from_slice(&[0x41, 0x83, 0xFD, TAKE_ALL_HP as u8]); // cmp r13d, 0x22
        v.extend_from_slice(&[0x74, 9]); // je store
        v.extend_from_slice(&[0xBB, 0x01, 0x00, 0x00, 0x00]); // mov ebx, 1
        v.extend_from_slice(&[0xF0, 0xFF, 0x40, 0x08]); // lock inc dword [rax+8]
        debug_assert_eq!(v.len(), 41);
        v.extend_from_slice(&STORE);
        let from = at + v.len() as u64;
        v.extend_from_slice(&crate::hookplan::rel_jmp(from, back)?);
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kh1_max_hp_by_level() {
        use mp_rage::*;
        assert_eq!(kh1_max_hp(1), 18);
        assert_eq!(kh1_max_hp(5), 18);
        assert_eq!(kh1_max_hp(6), 21);
        assert_eq!(kh1_max_hp(42), 18 + 3 * 11);
        assert_eq!(kh1_max_hp(75), 18 + 3 * 22);
        assert_eq!(kh1_max_hp(99), 87);
        // The player's btltbl: the new-game record's HP, the kind-1 levels, the Rod Dive's deltas.
        let Ok(dir) = std::env::var("KH1_TEST_DATA") else { return };
        let Ok(b) = std::fs::read(std::path::Path::new(&dir).join("btltbl.bin")) else { return };
        assert_eq!(b[0x39D8] as i32, KH1_START_HP);
        let kind1: Vec<u32> = (2..=100u32).filter(|&l| b[0x3AC0 + l as usize - 2] == 1).collect();
        assert_eq!(kind1, KH1_HP_UP_LEVELS);
    }

    /// KH1: HP lost × copies; ours: the lost share of Re:CoM's max HP as KH1's HP at Sora's level.
    #[test]
    fn mp_rage_charge() {
        use mp_rage::*;
        // Level 42 (KH1 max HP 51), Re:CoM max HP 400: a 40-HP hit is 10 % → 5.1 → 5 KH1 HP.
        assert_eq!(kh1_hp_lost(40, 400, 42), 5);
        assert_eq!(units(400, 360, 400, 42, 1), 5);
        assert_eq!(units(400, 360, 400, 42, 2), 10, "two copies: × 2");
        // Rounded to the nearest: 4 of 400 at 51 is 0.51 → 1; 3 of 400 → 0.38 → 0.
        assert_eq!(units(400, 396, 400, 42, 1), 1);
        assert_eq!(units(400, 397, 400, 42, 1), 0);
        // No copy, no loss, a heal: nothing.
        assert_eq!(units(400, 300, 400, 42, 0), 0);
        assert_eq!(units(300, 300, 400, 42, 2), 0);
        assert_eq!(units(300, 350, 400, 42, 2), 0);
        // A killing hit counts the HP Sora had (KH1 charges a dying actor too).
        assert_eq!(units(100, -50, 400, 75, 1), kh1_hp_lost(100, 400, 75));
        assert_eq!(kh1_hp_lost(100, 400, 75), 21, "84 × 0.25 = 21");
        // A Re:CoM max HP equal to KH1's: KH1's own numbers.
        assert_eq!(units(51, 40, 51, 42, 2), 22);
    }

    /// KH1's `0x14026E180` for Sora's gauge.
    #[test]
    fn berserk_low_hp() {
        use berserk::low_hp;
        // Max HP above 44: below 20 %.
        assert!(low_hp(19, 100) && !low_hp(20, 100));
        assert!(low_hp(79, 400) && !low_hp(80, 400));
        assert!(low_hp(1, 560));
        assert!(!low_hp(0, 100), "HP 0: not");
        // Max HP 44 or less: below 9 (KH1's low levels).
        assert!(low_hp(8, 44) && !low_hp(9, 44));
        assert!(low_hp(8, 18), "8 of 18 is low in KH1");
        // KH1 at Berserk's Rod level 60: max HP 66, the 20 % rule (13 is low, 14 is not).
        assert_eq!(mp_rage::kh1_max_hp(60), 66);
        assert!(low_hp(13, 66) && !low_hp(14, 66));
    }

    /// The share of Berserk's +4 Strength: KH1's curve at power + Strength − Defence, + 4.
    #[test]
    fn berserk_share_is_kh1s_curve() {
        use crate::attack::{plain_record, Sora};
        use kh1_core::data::{row, Moveset, WeaponData};
        let m = Moveset::kh1_defaults();
        let w = WeaponData::kingdom_key();
        let curve = kh1_core::DamageCurve::kh1_defaults().0;
        let data = m.attack(m.row(row::SWING_1).unwrap().attack).unwrap().clone();
        assert_eq!(data.stat_source, 0, "a swing reads Strength");
        let mut rec = plain_record(500);
        rec.defence = 20;
        let sora = Sora { strength: 26, max_mp: 3 };
        let (with, without) = berserk::share(&m, row::SWING_1, sora, &w, &rec, 500).unwrap();
        let at = |s: i32| curve[(data.power + s - 20).clamp(0, curve.len() as i32 - 1) as usize] as i32;
        if data.min == 0 && data.max == 0 {
            assert_eq!((with, without), (at(26 + 4), at(26)));
        }
        assert!(with > without, "+4 Strength raises KH1's damage below the curve's cap");
        // Re:CoM's value scaled by the share.
        assert_eq!(berserk::scale(100, with, without) as i32, 100 * with / without);
        assert_eq!(berserk::scale(30000, 3, 1), i16::MAX, "within Re:CoM's i16 hit value");
        assert_eq!(berserk::scale(50, 1, 0), 50);
        // At the curve's cap (300 from index 29) +4 adds nothing.
        let rec0 = plain_record(500);
        let (w2, wo2) = berserk::share(&m, row::SWING_1, Sora { strength: 60, max_mp: 3 }, &w, &rec0, 500).unwrap();
        assert_eq!(w2, wo2);
        // Zantetsuken's record has no stat term (source 2): Berserk changes nothing (KH1's case 2).
        assert_eq!(berserk::share(&m, row::ZANTETSUKEN, sora, &w, &rec, 500), None);
        // KH1's G+0xFE in the player's btltbl.
        if let Ok(dir) = std::env::var("KH1_TEST_DATA") {
            if let Ok(b) = std::fs::read(std::path::Path::new(&dir).join("btltbl.bin")) {
                assert_eq!(b[0x58 + 0xFE] as i32, berserk::STRENGTH);
            }
        }
    }

    /// KH1's `0x1402BFA80` step on Re:CoM's HP store.
    #[test]
    fn second_chance_rule() {
        use second_chance::new_hp;
        assert_eq!(new_hp(true, 50, -10, 1), 1, "a killing hit from above 1 HP leaves 1");
        assert_eq!(new_hp(true, 50, 0, 1), 1, "damage = HP: KH1's `HP <= damage`");
        assert_eq!(new_hp(true, 2, -100, 1), 1);
        assert_eq!(new_hp(true, 1, 0, 1), 0, "at 1 HP the hit kills (KH1: 0 → 1 damage)");
        assert_eq!(new_hp(true, 50, 20, 1), 20, "a hit that doesn't kill is untouched");
        assert_eq!(new_hp(false, 50, -10, 1), -10, "not equipped");
        assert_eq!(new_hp(true, 50, 0, 0x22), 0, "Re:CoM's take-all-HP reaction");
        // A combo: the first hit leaves 1, the next kills.
        let hp = new_hp(true, 30, 30 - 40, 1);
        assert_eq!(new_hp(true, hp, hp - 5, 1), -4);
    }

    #[test]
    fn second_chance_stub_layout() {
        let at = 0x7FF0_0000_0000u64;
        let back = at + 0x1000;
        let v = second_chance::stub(0x1122_3344_5566_7788, at, back).unwrap();
        assert_eq!(v.len(), 62);
        assert!(v.len() <= crate::hookplan::BLOCK);
        assert_eq!(&v[2..10], &0x1122_3344_5566_7788u64.to_le_bytes());
        // Every short jump lands on the moved store (offset 41).
        for (op_at, op) in [(13usize, 0x75u8), (18, 0x7D), (24, 0x7E), (30, 0x74)] {
            assert_eq!(v[op_at], op);
            assert_eq!(op_at + 2 + v[op_at + 1] as usize, 41, "jump at {op_at}");
        }
        assert_eq!(&v[41..57], &second_chance::STORE);
        assert_eq!(v[57], 0xE9);
        let rel = i32::from_le_bytes(v[58..62].try_into().unwrap()) as i64;
        assert_eq!((at as i64 + 62 + rel) as u64, back);
        // The Steam exe's store bytes (`sites::SECOND_CHANCE_PATCH`).
        assert_eq!(crate::sites::SECOND_CHANCE_PATCH[0].expect, &second_chance::STORE);
        assert_eq!(second_chance::stub(0, 0, 1 << 40), None, "out of rel32 reach");
    }
}
