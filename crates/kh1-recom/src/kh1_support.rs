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
}
