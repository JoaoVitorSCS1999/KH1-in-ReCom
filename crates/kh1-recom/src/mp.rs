//! KH1's MP at the KH1 setting (M7 stage 7.1, decision 34): the testable part.
//!
//! KH1 keeps MP as whole points plus a **charge** counter. The k-th point is worth 30·k charge
//! units: the refill makes point MP+1 when the charge reaches (MP+1)·30, one point per frame.
//! Hits charge their attack data's `+0x2C` × KH1's level factor. Spells pay whole points (cost
//! ≥ 100) or charge units (Fire / Blizzard: 30), borrowing a point when the charge runs out.
//! Everything below is a port of KH1's own functions (Steam addresses), with their integer and
//! `f32` steps. See `docs/recom-m7-mp.md`.
//!
//! | KH1 (Steam) | here |
//! |---|---|
//! | `0x1402A4A00` add points (clamped 0..=max) | [`Mp::add_points`] |
//! | `0x1402A4AE0` add charge (cap max·30, borrows a point when negative) | [`Mp::gain`] |
//! | `0x1402A4CB0` refill, every frame (pre-step `0x1402A43A0`) | [`Mp::refill_step`] |
//! | `0x1402A4C10` can pay | [`Mp::can_pay`] |
//! | `0x1402A5B60` pay (spell syscall 47 → `0x1402B37F0`) | [`Mp::pay`] |
//! | `0x1402A5130` battle stats from the save record (MP part) | [`Mp::build`] |
//! | `0x1402B5660` level-up bonus kind 2 (max MP +1, MP +1) | [`Mp::level_up_max`] |
//! | `0x1402A56F0` full restore (event syscall `0x1401B12D0`) | [`Mp::restore_full`] |
//! | `0x1402AD0B0` save point, per tick | [`Mp::save_point_tick`] |
//! | `0x1402BF330` charge per hit | [`hit_units`] (kh1-core's `mp_charge`) |
//!
//! Re:CoM keeps nothing of this: the state lives in a versioned block of Re:CoM's own save
//! ([`SavedMp`], next to the equipped-Keyblade byte), which Re:CoM saves, checksums, loads,
//! snapshots and zeroes itself. A block that was never written (all zero) means "full MP, no
//! charge" at Sora's current level.

use kh1_core::damage::{mp_charge, AttackerStats, MpChargeParams};

/// Charge units per point step (`0x1E` in `0x1402A4CB0`, `0x1402A4AE0`, `0x1402A5B60`). Checked
/// (decomp + exe).
pub const UNITS_PER_MP: i32 = 30;
/// Sora's max MP at level 1 (new-game record btltbl `0x39D8`: MP 3; the Dive keeps it on the
/// owner's path, keep the Rod). Checked (files), `m7-magic.md` § 4.1.2.
pub const START_MAX_MP: i32 = 3;
/// The levels whose level-up bonus is kind 2 (max MP +1) in Sora's table btltbl `0x3AC0`
/// (`0x140528CC0[0]`; byte `[L − 2]` for reaching level L): 12, 24, 36, 48, 60, no other in its
/// 99 entries. Checked (files + decomp `0x1402B51B0`, `0x1402B5660`).
pub const BONUS_MAX_MP_LEVELS: [u32; 5] = [12, 24, 36, 48, 60];
/// The levels whose entry is kind 2 in Sora's Dive list for the **Rod** (btltbl `0x3BF8`,
/// `*(0x140528D10 + 4 × save[0x48C])`; entry i = level i + 2): 30 and 66. The same level-up
/// `0x1402B51B0` passes each non-ability entry (1..7) of that list to the same bonus function
/// `0x1402B5660`, so kind 2 is max MP + 1 there too. Checked (files + decomp). The Rod: the mod's
/// Dive path keeps it (decision 18; `save[0x48C]` = 2 is the Rod: Assumption, as the ability list).
/// Owner, 2026-11-01: add them.
pub const ROD_MAX_MP_LEVELS: [u32; 2] = [30, 66];
/// Every max-MP level of Sora's KH1 growth on the owner's path, in order: the bonus list's and the
/// Rod list's ([`BONUS_MAX_MP_LEVELS`], [`ROD_MAX_MP_LEVELS`]; none shared).
pub const MAX_MP_UP_LEVELS: [u32; 7] = [12, 24, 30, 36, 48, 60, 66];

/// KH1's spell costs (spell data `+0x30` per level, btltbl via the level table `0x140528620`;
/// the same at all three levels). ≥ 100 = whole points (cost / 100), < 100 = charge units.
/// Checked (files + exe). For the spell stages (7.4–7.10).
pub mod cost {
    pub const FIRE: i32 = 30;
    pub const BLIZZARD: i32 = 30;
    pub const THUNDER: i32 = 100;
    pub const CURE: i32 = 100;
    pub const GRAVITY: i32 = 100;
    pub const STOP: i32 = 200;
    pub const AERO: i32 = 200;
}

/// Sora's KH1 base max MP at a level (no Keyblade bonus): 3, +1 at each of
/// [`MAX_MP_UP_LEVELS`] reached (10 at level 66 and above). Re:CoM's level is used by number (decision 34: "Max MP grows on
/// level up"; as Sora's KH1 Strength, decision 18).
pub fn base_max_mp(level: u32) -> i32 {
    START_MAX_MP + MAX_MP_UP_LEVELS.iter().filter(|&&l| l <= level).count() as i32
}

/// Max MP with the equipped Keyblade's KH1 MP bonus (weapon `+0x38`, added in `0x1402A5130`;
/// accessories don't exist at the KH1 setting). Never below 0 (KH1 has no clamp; Sora's lowest
/// is 3 − 2 = 1).
pub fn max_mp(level: u32, keyblade_bonus: i32) -> i32 {
    (base_max_mp(level) + keyblade_bonus).max(0)
}

/// KH1's MP state of one actor: stats `+0x44` MP, `+0x48` max MP, `+0xBE` charge. The `wrote_*`
/// flags record which of the save record's fields KH1 would have written (record `+3` MP is
/// written by every [`Mp::add_points`], `+0x38` charge by every charge write), so that only those
/// go back into the save, as KH1 does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mp {
    pub mp: i32,
    pub max: i32,
    pub charge: u16,
    pub wrote_mp: bool,
    pub wrote_charge: bool,
}

impl Mp {
    /// A state with nothing written yet.
    pub fn new(mp: i32, max: i32, charge: u16) -> Mp {
        Mp { mp, max, charge, wrote_mp: false, wrote_charge: false }
    }

    /// `0x1402A5130`'s MP part: MP = min(record MP, max MP); charge = min(record charge, max·30).
    pub fn build(saved_mp: u8, saved_charge: u16, max: i32) -> Mp {
        let mp = if max <= saved_mp as i32 { max } else { saved_mp as i32 };
        let cap = max * UNITS_PER_MP;
        let charge = if cap <= saved_charge as i32 { cap as u16 } else { saved_charge };
        Mp::new(mp, max, charge)
    }

    /// (MP, max MP, charge).
    pub fn view(&self) -> (i32, i32, u16) {
        (self.mp, self.max, self.charge)
    }

    /// Whether KH1's per-frame refill would make a point now.
    pub fn refill_ready(&self) -> bool {
        (self.mp + 1) * UNITS_PER_MP <= self.charge as i32
    }

    /// `0x1402A4A00(actor, n)`: MP + n, at most max, at least 0; returns the new MP. (KH1 scales
    /// the change by G`+0x88` = 0.5 while the actor's `+0x88 & 0x20` status is on without
    /// ability 9; Sora has no such status at the KH1 setting: not ported.)
    pub fn add_points(&mut self, n: i32) -> i32 {
        let v = self.mp + n;
        let r = if v > self.max {
            self.max
        } else if v < 0 {
            0
        } else {
            v
        };
        self.mp = r;
        self.wrote_mp = true;
        r
    }

    /// `0x1402A4AE0(actor, n)`: charge + n. Negative: borrows a point (MP − 1, charge + the old
    /// MP × 30), or 0 with no point. Positive: capped at max MP × 30 (no cap after a borrow).
    /// Stored as KH1's 16-bit field. Returns the new charge.
    pub fn gain(&mut self, n: i32) -> u16 {
        let v = self.charge as i32 + n;
        let out = if v < 0 {
            let before = self.mp;
            if before < 1 {
                0
            } else {
                self.add_points(-1);
                v + before * UNITS_PER_MP
            }
        } else if self.max * UNITS_PER_MP < v {
            self.max * UNITS_PER_MP
        } else {
            v
        };
        self.charge = out as u16;
        self.wrote_charge = true;
        self.charge
    }

    /// `0x1402A4CB0`, once per frame: when the charge reaches (MP + 1) × 30, MP + 1 (clamped) and
    /// the charge loses (MP + 1) × 30 (the old MP's next point). True when it did.
    pub fn refill_step(&mut self) -> bool {
        let before = self.mp;
        let need = (before + 1) * UNITS_PER_MP;
        let ready = need <= self.charge as i32;
        if ready {
            self.add_points(1);
            self.charge = self.charge.wrapping_sub(need as u16);
        }
        // KH1 writes the charge back every frame, changed or not.
        self.wrote_charge = true;
        ready
    }

    /// `0x1402A4C10(actor, cost)`: cost ≥ 100: cost / 100 ≤ MP. Else: cost ≤ the whole pool,
    /// (MP + 1)·MP / 2 · 30 + charge (every point's worth, 30·k for point k).
    pub fn can_pay(&self, cost: i32) -> bool {
        if cost > 99 {
            cost / 100 <= self.mp
        } else {
            cost <= ((self.mp + 1) * self.mp / 2) * UNITS_PER_MP + self.charge as i32
        }
    }

    /// `0x1402A5B60(actor, cost)`, KH1's payment (the `.mag` scripts' syscall 47 at the release
    /// frame). Cost ≥ 100: MP − cost / 100 (clamped at 0). Cost < 100: charge − cost; when that is
    /// negative and MP > 0, MP − 1 and the charge becomes (charge − cost) + old MP × 30 (the top
    /// point broken into its 30·MP units); with MP 0 the charge becomes 0. Not negative: capped at
    /// max MP × 30. It does not test [`Mp::can_pay`] itself (KH1's menu does, before the cast).
    pub fn pay(&mut self, cost: i32) {
        if cost < 100 {
            let v = self.charge as i32 - cost;
            let out = if v < 0 {
                let before = self.mp;
                if 0 < before {
                    self.add_points(-1);
                    v + before * UNITS_PER_MP
                } else {
                    0
                }
            } else if self.max * UNITS_PER_MP < v {
                self.max * UNITS_PER_MP
            } else {
                v
            };
            self.charge = out as u16;
            self.wrote_charge = true;
        } else {
            self.add_points(-(cost / 100));
        }
    }

    /// `0x1402B5660` kind 2 on a live actor: max MP + 1 (stats `+0x48`), then MP + 1 through
    /// `0x1402A4A00` (record `+8` max MP + 1 in the save). No refill: KH1's level-up gives only
    /// this point (`0x14029BA50` after it is the revive helper). Checked (decomp).
    pub fn level_up_max(&mut self) {
        self.max += 1;
        self.add_points(1);
    }

    /// `0x1402A56F0`'s MP part (event syscall `0x1401B12D0`, party swap `0x1403A1540`): when MP is
    /// below max, MP = max and the charge 0; at full MP nothing changes (the charge stays).
    pub fn restore_full(&mut self) {
        if self.mp < self.max {
            let m = self.max;
            self.add_points(m);
            self.charge = 0;
            self.wrote_charge = true;
        }
    }

    /// `0x1402AD0B0`'s MP part, one tick of a save point (per member, while Sora stands in it):
    /// when MP is below max, MP + 1 and the charge emptied (`0x1402A4AE0(−charge)`).
    pub fn save_point_tick(&mut self) {
        if self.mp < self.max {
            self.add_points(1);
            let c = self.charge as i32;
            self.gain(-c);
        }
    }
}

/// `0x1402BF330`'s charge for one KH1 hit: the attack data's `+0x2C` (swings 3, finishers 6,
/// Counterattack 24) × (1 + (enemy level − Sora's level) × 20 / 100) in `f32`, truncated, at
/// least 1, at most 250 % of `+0x2C` (btltbl G `+0x94` = 20, `+0x96` = 250). None when the enemy's
/// level is not above 0: KH1 then charges nothing (it never calls the gauge add). Bit 0 of the
/// ability block `+0x184` (ability 0x17, +150 %) is M9's: off here.
pub fn hit_units(record_charge: i32, enemy_level: i32, sora_level: u32) -> Option<i32> {
    hit_units_with(record_charge, enemy_level, sora_level, false)
}

/// [`hit_units`] with MP Haste (M12 step 3; KH1 ability 23 → ability block `+0x184` bit 0,
/// `0x1402A6380`): the factor gets `+ G+0x95 / 100` = + 1.5 in `f32` (`0x1402BF330`), before the
/// truncation, the floor 1 and the 250 % cap. So at equal levels a swing (3) charges 7 (3 × 2.5 =
/// 7.5, cap 7), a finisher (6) 15, Counterattack (24) 60. Checked (decomp + files).
pub fn hit_units_with(record_charge: i32, enemy_level: i32, sora_level: u32, mp_haste: bool) -> Option<i32> {
    if enemy_level <= 0 {
        return None;
    }
    let attacker = AttackerStats { level: sora_level as i32, mp_charge_bonus: mp_haste, ..Default::default() };
    Some(mp_charge(&MpChargeParams::default(), record_charge, enemy_level, &attacker))
}

/// The level a Re:CoM enemy without a KH1 twin (Re:CoM-only enemies, every boss) charges with.
/// KH1 has no such enemies. **Provisional, owner question**: Sora's own level (factor 1.0, KH1's
/// base 3 / 6 / 24 per hit).
pub fn no_twin_level(sora_level: u32) -> i32 {
    sora_level as i32
}

// ---- the save block ----

/// Our block in Re:CoM's save: `save + 0x2675 ..= save + 0x2684`, right after the equipped
/// Keyblade byte (`save + 0x2674`), inside the card collection's spare tail `save + 0x246B ..=
/// 0x2817` (`docs/recom-keyblade-equip.md`).
pub const BLOCK_FIRST: usize = 0x2675;
pub const BLOCK_LEN: usize = 0x10;
pub const BLOCK_LAST: usize = BLOCK_FIRST + BLOCK_LEN - 1;

/// Offsets inside the block. Fields never move: a later layout only adds fields and raises the
/// version.
pub mod off {
    /// Layout version: 0 = never written (defaults: full MP, no charge), 1 = this layout.
    pub const VERSION: usize = 0;
    /// MP, whole points (KH1 record `+3`, u8).
    pub const MP: usize = 1;
    /// Charge, u16 little-endian (KH1 record `+0x38`).
    pub const CHARGE: usize = 2;
    /// Sora's Re:CoM level when the block was last written (max-MP level-ups after it are applied
    /// when read).
    pub const LEVEL: usize = 4;
    /// Stage 7.11: 7 spell levels, Fire, Blizzard, Thunder, Cure, Gravity, Stop, Aero (KH1 save
    /// `+0x492 + spell`); 0 = not learned. Written by the unlocks (`crate::unlocks`), raised only.
    pub const SPELLS: usize = 5;
    pub const SPELL_COUNT: usize = 7;
    /// Stage 7.12: the 3 shortcut slots (KH1 save `+0x844..0x846`); 0 = none, else spell + 1.
    /// Filled by KH1's learn rule (`crate::unlocks::shortcuts`), once from the spells already
    /// learned for an older save; Customize (7.13, `win::mp::set_shortcuts`) sets them.
    pub const SHORTCUTS: usize = 0xC;
    pub const SHORTCUT_COUNT: usize = 3;
    /// Stage 7.11: bit 0 = the spell levels were synced from Re:CoM's card collection for this
    /// save (`crate::unlocks::SYNCED`); stage 7.12: bit 1 = the shortcut slots were set up
    /// (`crate::unlocks::SHORTCUTS_SET`); the other bits reserved (zero).
    pub const SPARE: usize = 0xF;
}

pub const LAYOUT_V1: u8 = 1;

// The block stays inside the spare range, and its last field inside the block.
const _: () = assert!(BLOCK_LAST <= crate::keyblades::SPARE_LAST && off::SPARE < BLOCK_LEN);

/// The MP fields of our save block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SavedMp {
    pub version: u8,
    pub mp: u8,
    pub charge: u16,
    pub level: u8,
}

impl SavedMp {
    pub fn read(block: &[u8; BLOCK_LEN]) -> SavedMp {
        SavedMp {
            version: block[off::VERSION],
            mp: block[off::MP],
            charge: u16::from_le_bytes([block[off::CHARGE], block[off::CHARGE + 1]]),
            level: block[off::LEVEL],
        }
    }

    /// Writes these fields only; the spell, shortcut and spare bytes stay as they are.
    pub fn write(&self, block: &mut [u8; BLOCK_LEN]) {
        block[off::VERSION] = self.version;
        block[off::MP] = self.mp;
        block[off::CHARGE..off::CHARGE + 2].copy_from_slice(&self.charge.to_le_bytes());
        block[off::LEVEL] = self.level;
    }
}

/// The live MP for a saved block at Sora's Re:CoM level, with the Keyblade's MP bonus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Live {
    pub mp: Mp,
    /// The level the state is synced to (Sora's level now).
    pub level: u32,
    /// The block was never written: the state is the default (full, no charge).
    pub fresh: bool,
    /// Max-MP level-ups applied since the block was written.
    pub level_ups: u32,
}

/// Sora's level as the block stores it (1..=255).
fn level_byte(level: u32) -> u8 {
    level.clamp(1, 255) as u8
}

/// The live state: KH1's stats build (`Mp::build`) at the level the block was written, then
/// KH1's level-up bonus for each max-MP level reached since (`Mp::level_up_max`, in order). A
/// never-written block is full MP and no charge at the current level. A level below the stored
/// one (not seen in Re:CoM's own flow) syncs to it with no MP change.
pub fn live(saved: &SavedMp, level: u32, keyblade_bonus: i32) -> Live {
    let level = level.clamp(1, 255);
    if saved.version == 0 {
        let max = max_mp(level, keyblade_bonus);
        return Live { mp: Mp::new(max, max, 0), level, fresh: true, level_ups: 0 };
    }
    let seen = if saved.level == 0 { level } else { saved.level as u32 };
    if level <= seen {
        return Live { mp: Mp::build(saved.mp, saved.charge, max_mp(level, keyblade_bonus)), level, fresh: false, level_ups: 0 };
    }
    let mut mp = Mp::build(saved.mp, saved.charge, max_mp(seen, keyblade_bonus));
    let mut ups = 0;
    for _ in MAX_MP_UP_LEVELS.iter().filter(|&&l| seen < l && l <= level) {
        mp.level_up_max();
        ups += 1;
    }
    Live { mp, level, fresh: false, level_ups: ups }
}

/// The block's fields after a change: the version (at least 1), the level, and the MP fields KH1
/// would have written (all of them when the block was fresh, and the MP when level-ups were
/// applied). None when nothing was written (the block stays as it is, all zero if fresh).
pub fn stored(prior: &SavedMp, l: &Live) -> Option<SavedMp> {
    let m = &l.mp;
    if !m.wrote_mp && !m.wrote_charge {
        return None;
    }
    let mut s = *prior;
    s.version = prior.version.max(LAYOUT_V1);
    s.level = level_byte(l.level);
    if m.wrote_mp || l.fresh || l.level_ups > 0 {
        s.mp = m.mp.clamp(0, 255) as u8;
    }
    if m.wrote_charge || l.fresh {
        s.charge = m.charge;
    }
    Some(s)
}

/// The trace token: `mp <MP>/<max> c<charge>`, then the frame's events.
pub fn trace_token(view: Option<(i32, i32, u16)>, events: &[String]) -> String {
    let mut t = match view {
        Some((mp, max, c)) => format!("mp {mp}/{max} c{c}"),
        None => "mp -".to_string(),
    };
    for e in events {
        t.push(' ');
        t.push_str(e);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mp(mp: i32, max: i32, charge: u16) -> Mp {
        Mp::new(mp, max, charge)
    }

    #[test]
    fn max_mp_by_level() {
        assert_eq!(base_max_mp(1), 3);
        assert_eq!(base_max_mp(11), 3);
        assert_eq!(base_max_mp(12), 4);
        assert_eq!(base_max_mp(23), 4);
        assert_eq!(base_max_mp(24), 5);
        // The Rod list's +1 at 30 (owner, 2026-11-01).
        assert_eq!(base_max_mp(29), 5);
        assert_eq!(base_max_mp(30), 6);
        assert_eq!(base_max_mp(36), 7);
        assert_eq!(base_max_mp(48), 8);
        assert_eq!(base_max_mp(59), 8);
        assert_eq!(base_max_mp(60), 9);
        // And at 66.
        assert_eq!(base_max_mp(65), 9);
        assert_eq!(base_max_mp(66), 10);
        assert_eq!(base_max_mp(99), 10);
        // Keyblade bonuses (KH1 weapon +0x38): Diamond Dust +3, One-Winged Angel −2.
        assert_eq!(max_mp(1, 3), 6);
        assert_eq!(max_mp(1, -2), 1);
        assert_eq!(max_mp(60, -2), 7);
        assert_eq!(max_mp(1, -5), 0);
    }

    #[test]
    fn add_points_clamps() {
        let mut m = mp(2, 3, 0);
        assert_eq!(m.add_points(5), 3);
        assert_eq!(m.add_points(-9), 0);
        assert!(m.wrote_mp && !m.wrote_charge);
    }

    #[test]
    fn gain_caps_at_max_times_30() {
        let mut m = mp(0, 3, 0);
        assert_eq!(m.gain(3), 3);
        assert_eq!(m.gain(100), 90, "cap = max MP × 30");
        assert_eq!(m.mp, 0, "gain alone never makes a point (the refill does)");
        assert!(m.wrote_charge && !m.wrote_mp);
    }

    #[test]
    fn gain_negative_borrows_a_point() {
        // 0x1402A4AE0: charge 10 − 25 = −15; MP 2 → 1, charge −15 + 2·30 = 45 (no cap after a borrow).
        let mut m = mp(2, 3, 10);
        assert_eq!(m.gain(-25), 45);
        assert_eq!(m.mp, 1);
        // No point to borrow: 0.
        let mut m = mp(0, 3, 10);
        assert_eq!(m.gain(-25), 0);
        assert_eq!(m.mp, 0);
    }

    #[test]
    fn refill_point_k_needs_30k() {
        let mut m = mp(0, 8, 0);
        m.gain(29);
        assert!(!m.refill_step());
        m.gain(1);
        assert!(m.refill_step(), "30 units make the first point");
        assert_eq!(m.view(), (1, 8, 0));
        m.gain(59);
        assert!(!m.refill_step(), "the second point needs 60");
        m.gain(1);
        assert!(m.refill_step());
        assert_eq!(m.view(), (2, 8, 0));
        m.gain(90);
        assert!(m.refill_step(), "the third needs 90");
        assert_eq!(m.view(), (3, 8, 0));
    }

    #[test]
    fn refill_is_one_point_per_frame() {
        // MP 0, charge 150 (cap 240): frame 1 → MP 1, 120; frame 2 → MP 2, 60; frame 3: needs 90.
        let mut m = mp(0, 8, 0);
        m.gain(150);
        assert!(m.refill_step());
        assert_eq!(m.view(), (1, 8, 120));
        assert!(m.refill_step());
        assert_eq!(m.view(), (2, 8, 60));
        assert!(!m.refill_step());
        assert_eq!(m.view(), (2, 8, 60));
    }

    #[test]
    fn hits_in_one_frame_cap_before_the_refill() {
        // KH1 caps each gain at max·30 and refills once per frame afterwards. Two +60 hits in one
        // frame at MP 0 / max 3, charge 29: 89, then 149 → 90; frames: MP 1 (60), MP 2 (0).
        let mut m = mp(0, 3, 29);
        m.gain(60);
        m.gain(60);
        assert_eq!(m.charge, 90);
        assert!(m.refill_step());
        assert!(m.refill_step());
        assert!(!m.refill_step());
        assert_eq!(m.view(), (2, 3, 0));
    }

    #[test]
    fn full_mp_banks_charge() {
        // At full MP the charge keeps filling up to max·30 and makes no point; after a point is
        // spent, the banked charge refills it on the next frame.
        let mut m = mp(3, 3, 0);
        m.gain(200);
        assert_eq!(m.charge, 90);
        assert!(!m.refill_step());
        m.pay(cost::THUNDER);
        assert_eq!(m.view(), (2, 3, 90));
        assert!(m.refill_step(), "90 ≥ (2 + 1)·30");
        assert_eq!(m.view(), (3, 3, 0));
    }

    #[test]
    fn can_pay_whole_points_and_units() {
        assert!(mp(1, 3, 0).can_pay(cost::THUNDER));
        assert!(!mp(1, 3, 0).can_pay(cost::STOP));
        assert!(mp(2, 3, 0).can_pay(cost::AERO));
        // Units against the pool 30·MP(MP+1)/2 + charge.
        assert!(!mp(0, 3, 29).can_pay(cost::FIRE));
        assert!(mp(0, 3, 30).can_pay(cost::FIRE));
        assert!(mp(1, 3, 0).can_pay(cost::FIRE));
        assert!(mp(1, 3, 0).can_pay(30) && !mp(1, 3, 0).can_pay(31));
        assert!(mp(2, 3, 0).can_pay(90) && !mp(2, 3, 0).can_pay(91));
    }

    #[test]
    fn fire_pays_charge_first_then_breaks_the_top_point() {
        // Charge 40: Fire takes 30 of it.
        let mut m = mp(2, 3, 40);
        m.pay(cost::FIRE);
        assert_eq!(m.view(), (2, 3, 10));
        // Charge 10 < 30: point 2 (worth 60) breaks: MP 1, charge 10 − 30 + 60 = 40.
        m.pay(cost::FIRE);
        assert_eq!(m.view(), (1, 3, 40));
        // Charge 40: MP 1, charge 10.
        m.pay(cost::FIRE);
        assert_eq!(m.view(), (1, 3, 10));
        // Point 1 (worth 30) breaks: MP 0, charge 10.
        m.pay(cost::FIRE);
        assert_eq!(m.view(), (0, 3, 10));
        // Nothing to break: charge 0 (KH1 pays even when short; its menu tests first).
        m.pay(cost::FIRE);
        assert_eq!(m.view(), (0, 3, 0));
    }

    #[test]
    fn fire_at_three_points_costs_a_third_of_the_top() {
        // MP 3, charge 0: point 3 (90 units) breaks into 90 − 30 = 60 left: MP 2, charge 60.
        let mut m = mp(3, 3, 0);
        m.pay(cost::FIRE);
        assert_eq!(m.view(), (2, 3, 60));
        // The refill needs 90 for point 3; 60 stays.
        assert!(!m.refill_step());
    }

    #[test]
    fn pay_whole_points() {
        let mut m = mp(3, 3, 12);
        m.pay(cost::STOP);
        assert_eq!(m.view(), (1, 3, 12), "the charge is untouched");
        assert!(!m.wrote_charge);
        m.pay(cost::THUNDER);
        assert_eq!(m.view(), (0, 3, 12));
        m.pay(cost::THUNDER);
        assert_eq!(m.view(), (0, 3, 12), "clamped at 0");
        // cost / 100 truncates: 150 → 1 point.
        let mut m = mp(3, 3, 0);
        m.pay(150);
        assert_eq!(m.mp, 2);
    }

    #[test]
    fn pay_units_cap_and_16_bit_store() {
        // Not negative: capped at max·30 (KH1's own cap; only matters after the max fell).
        let mut m = mp(1, 1, 200);
        m.pay(10);
        assert_eq!(m.charge, 30);
        // A unit cost KH1's data never has (31..99) can leave the borrow negative: stored as KH1's
        // 16-bit field.
        let mut m = mp(1, 3, 0);
        m.pay(90);
        assert_eq!(m.view(), (0, 3, (-60i32) as u16));
    }

    #[test]
    fn build_clamps_to_max() {
        assert_eq!(Mp::build(5, 200, 3).view(), (3, 3, 90));
        assert_eq!(Mp::build(2, 40, 3).view(), (2, 3, 40));
        let b = Mp::build(2, 40, 3);
        assert!(!b.wrote_mp && !b.wrote_charge);
    }

    #[test]
    fn level_up_gives_one_point() {
        let mut m = mp(1, 3, 25);
        m.level_up_max();
        assert_eq!(m.view(), (2, 4, 25));
        let mut m = mp(3, 3, 0);
        m.level_up_max();
        assert_eq!(m.view(), (4, 4, 0));
    }

    #[test]
    fn restore_and_save_point() {
        let mut m = mp(1, 4, 25);
        m.restore_full();
        assert_eq!(m.view(), (4, 4, 0));
        // Full: nothing changes, the charge stays.
        let mut m = mp(4, 4, 25);
        m.restore_full();
        assert_eq!(m.view(), (4, 4, 25));
        assert!(!m.wrote_mp && !m.wrote_charge);
        let mut m = mp(1, 4, 25);
        m.save_point_tick();
        assert_eq!(m.view(), (2, 4, 0));
        m.save_point_tick();
        m.save_point_tick();
        m.save_point_tick();
        assert_eq!(m.view(), (4, 4, 0));
    }

    #[test]
    fn hit_charge_by_level() {
        // Same level: 3 / 6 / 24.
        assert_eq!(hit_units(3, 10, 10), Some(3));
        assert_eq!(hit_units(6, 10, 10), Some(6));
        assert_eq!(hit_units(24, 10, 10), Some(24));
        // One level above Sora: 1.2f × 3 = 3.6 → 3; × 6 = 7.2 → 7; × 24 = 28.8 → 28.
        assert_eq!(hit_units(3, 11, 10), Some(3));
        assert_eq!(hit_units(6, 11, 10), Some(7));
        assert_eq!(hit_units(24, 11, 10), Some(28));
        // One below: 0.8f × 3 = 2.4 → 2.
        assert_eq!(hit_units(3, 9, 10), Some(2));
        // Far below: at least 1. Far above: at most 250 % (24 → 60, 3 → 7).
        assert_eq!(hit_units(24, 1, 40), Some(1));
        assert_eq!(hit_units(24, 50, 10), Some(60));
        assert_eq!(hit_units(3, 50, 10), Some(7));
        // Level 0 (KH1's Destiny Islands Shadow): nothing.
        assert_eq!(hit_units(3, 0, 1), None);
        // Guard's record (+0x2C = 0): 0.
        assert_eq!(hit_units(0, 10, 10), Some(0));
    }

    /// MP Haste (M12 step 3): `0x1402BF330`'s factor + 150 / 100 (G `+0x95`), then the same
    /// truncation, floor 1 and 250 % cap (G `+0x96`).
    #[test]
    fn hit_charge_with_mp_haste() {
        assert_eq!(hit_units_with(3, 10, 10, false), hit_units(3, 10, 10));
        // Same level: 2.5f × 3 = 7.5 → 7 (the cap is 7 too); × 6 = 15; × 24 = 60.
        assert_eq!(hit_units_with(3, 10, 10, true), Some(7));
        assert_eq!(hit_units_with(6, 10, 10, true), Some(15));
        assert_eq!(hit_units_with(24, 10, 10, true), Some(60));
        // Two levels below: (1 − 0.4 + 1.5) = 2.1f × 6 = 12.6 → 12.
        assert_eq!(hit_units_with(6, 8, 10, true), Some(12));
        // Far below: 1 + (−39 × 0.2) + 1.5 < 0 → at least 1. Far above: the cap.
        assert_eq!(hit_units_with(24, 1, 40, true), Some(1));
        assert_eq!(hit_units_with(6, 50, 10, true), Some(15));
        // Level 0 still charges nothing (KH1 never calls the gauge add).
        assert_eq!(hit_units_with(3, 0, 1, true), None);
        // KH1's G numbers in the player's btltbl (G = btltbl + 0x58).
        if let Ok(dir) = std::env::var("KH1_TEST_DATA") {
            if let Ok(b) = std::fs::read(std::path::Path::new(&dir).join("btltbl.bin")) {
                assert_eq!((b[0x58 + 0x94], b[0x58 + 0x95], u16::from_le_bytes([b[0x58 + 0x96], b[0x58 + 0x97]])), (20, 150, 250));
            }
        }
    }

    #[test]
    fn fresh_block_is_full_at_the_current_level() {
        let z = SavedMp::read(&[0; BLOCK_LEN]);
        assert_eq!(z, SavedMp::default());
        let l = live(&z, 1, 0);
        assert!(l.fresh);
        assert_eq!(l.mp.view(), (3, 3, 0));
        let l = live(&z, 30, 2);
        assert_eq!(l.mp.view(), (8, 8, 0), "12, 24 and the Rod's 30, + 2");
        // Reading writes nothing.
        assert_eq!(stored(&z, &l), None);
    }

    #[test]
    fn first_write_fills_every_field() {
        let z = SavedMp::default();
        let mut l = live(&z, 12, 0);
        l.mp.gain(3);
        let s = stored(&z, &l).expect("written");
        assert_eq!(s, SavedMp { version: 1, mp: 4, charge: 3, level: 12 });
        let mut block = [0u8; BLOCK_LEN];
        block[off::SPELLS] = 2;
        s.write(&mut block);
        assert_eq!(block[..5], [1, 4, 3, 0, 12]);
        assert_eq!(block[off::SPELLS], 2, "other stages' bytes are kept");
        assert_eq!(SavedMp::read(&block), s);
    }

    #[test]
    fn only_written_fields_go_back() {
        // KH1's record keeps an MP above max while a lower-max Keyblade is on, until MP changes.
        let saved = SavedMp { version: 1, mp: 5, charge: 10, level: 30 };
        let mut l = live(&saved, 30, -2);
        assert_eq!(l.mp.view(), (4, 4, 10));
        l.mp.gain(3);
        let s = stored(&saved, &l).unwrap();
        assert_eq!(s.mp, 5, "a charge write leaves the record's MP");
        assert_eq!(s.charge, 13);
        assert_eq!(live(&s, 30, 0).mp.view(), (5, 6, 13), "back to the Kingdom Key: 5 again");
        // An MP write stores the clamped MP.
        let mut l = live(&saved, 30, -2);
        l.mp.pay(cost::THUNDER);
        assert_eq!(stored(&saved, &l).unwrap().mp, 3);
    }

    #[test]
    fn level_ups_since_the_last_write() {
        // Written at level 11 with MP 1 of 3; now level 25: two max-MP level-ups (12, 24).
        let saved = SavedMp { version: 1, mp: 1, charge: 7, level: 11 };
        let l = live(&saved, 25, 0);
        assert_eq!(l.level_ups, 2);
        assert_eq!(l.mp.view(), (3, 5, 7), "each gives max + 1 and MP + 1");
        // Until something is written, the block keeps level 11 (the sync is redone).
        let mut l2 = l;
        l2.mp.gain(1);
        let s = stored(&saved, &l2).unwrap();
        assert_eq!(s, SavedMp { version: 1, mp: 3, charge: 8, level: 25 });
        assert_eq!(live(&s, 25, 0).level_ups, 0);
        // No max-MP level between: nothing.
        let l = live(&SavedMp { version: 1, mp: 1, charge: 0, level: 13 }, 23, 0);
        assert_eq!((l.level_ups, l.mp.view()), (0, (1, 4, 0)));
        // A full bar stays full through a level-up.
        let l = live(&SavedMp { version: 1, mp: 3, charge: 0, level: 1 }, 12, 0);
        assert_eq!(l.mp.view(), (4, 4, 0));
        // The Rod list's 30 and 66 are level-ups too (owner, 2026-11-01).
        let l = live(&SavedMp { version: 1, mp: 2, charge: 0, level: 29 }, 31, 0);
        assert_eq!((l.level_ups, l.mp.view()), (1, (3, 6, 0)));
        let l = live(&SavedMp { version: 1, mp: 9, charge: 0, level: 60 }, 99, 0);
        assert_eq!((l.level_ups, l.mp.view()), (1, (10, 10, 0)));
    }

    #[test]
    fn max_mp_levels_are_the_bonus_and_rod_lists() {
        let mut v: Vec<u32> = BONUS_MAX_MP_LEVELS.iter().chain(ROD_MAX_MP_LEVELS.iter()).copied().collect();
        v.sort();
        assert_eq!(v, MAX_MP_UP_LEVELS);
        assert!(MAX_MP_UP_LEVELS.windows(2).all(|w| w[0] < w[1]), "none shared");
        // From the player's btltbl when the test data is there: kind 2 in the bonus list 0x3AC0
        // and in the Rod list 0x3BF8 (entry i = level i + 2).
        let Ok(dir) = std::env::var("KH1_TEST_DATA") else { return };
        let Ok(b) = std::fs::read(std::path::Path::new(&dir).join("btltbl.bin")) else { return };
        let kind2 = |list: usize| -> Vec<u32> { (0..99).filter(|&i| b[list + i] == 2).map(|i| i as u32 + 2).collect() };
        assert_eq!(kind2(0x3AC0), BONUS_MAX_MP_LEVELS);
        assert_eq!(kind2(0x3BF8), ROD_MAX_MP_LEVELS);
    }

    #[test]
    fn level_below_the_stored_one() {
        let saved = SavedMp { version: 1, mp: 4, charge: 0, level: 30 };
        let l = live(&saved, 10, 0);
        assert_eq!(l.mp.view(), (3, 3, 0));
        assert_eq!(l.level, 10);
        // A level byte of 0 in a written block: taken as the current level.
        let l = live(&SavedMp { version: 1, mp: 2, charge: 0, level: 0 }, 40, 0);
        assert_eq!((l.level_ups, l.mp.view()), (0, (2, 7, 0)));
    }

    #[test]
    fn later_layouts_keep_v1_fields() {
        let saved = SavedMp { version: 3, mp: 2, charge: 5, level: 20 };
        let mut l = live(&saved, 20, 0);
        l.mp.gain(1);
        assert_eq!(stored(&saved, &l).unwrap().version, 3, "never lowered");
    }

    #[test]
    fn block_sits_after_the_keyblade_byte_in_the_spare_range() {
        // The range and the field order are also checked at compile time (below `LAYOUT_V1`).
        assert_eq!(BLOCK_FIRST, crate::keyblades::EQUIP_OFFSET + 1);
        assert_eq!(off::SPELLS + off::SPELL_COUNT, off::SHORTCUTS);
        assert_eq!(off::SHORTCUTS + off::SHORTCUT_COUNT, off::SPARE);
    }

    #[test]
    fn trace_tokens() {
        assert_eq!(trace_token(Some((2, 3, 15)), &[]), "mp 2/3 c15");
        assert_eq!(trace_token(Some((2, 3, 15)), &["+3".into(), "pay30".into()]), "mp 2/3 c15 +3 pay30");
        assert_eq!(trace_token(None, &[]), "mp -");
    }
}
