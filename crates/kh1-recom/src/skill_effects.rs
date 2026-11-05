//! Enemy Skills' effects (plan `docs/recom-enemy-skills.md`): which skills are built, their help
//! lines, and the rules and numbers their effects use. Pure (tested); `win::enemy_skills` applies
//! them through Re:CoM's own code.
//!
//! **Stage 2, route 1: the play-time setters.** For each equipped skill, Re:CoM's own player-side
//! writes of `0x140163C00` (the switch Re:CoM runs when Sora's side plays an enemy card) or its
//! setters, without the bosses' affinity call `0x1401F8810` (owner: "skill only"), and the CBTL
//! flags `0x140163170` sets for Sora's side (Bio, Regen), at a KH1 battle's start, again after every
//! Sora-side clear `0x140161E10`, and when the ability set changes. Re:CoM's own readers then act:
//! its weapon setters in `procAttackCollision` `0x1401F5DC0` (Wide Attack, Back Attack, Double
//! Strike), its per-frame player exe `0x1401F9840` (Vanish, Aero Guard, the barrier timer), its
//! damage check `0x1401E6DE0` (Quick Barrier), its CBTL update `0x1401617F0` (Bio, Regen), its
//! prize pulls `0x140230C70` (Draw). Where the owner's number differs from Re:CoM's, Re:CoM's
//! constant is switched per battle (`sites::ENEMY_SKILL_PATCHES`, as M6 switches its patches:
//! Re:CoM's bytes at Combat = Re:CoM and in Riku's story) or the setter's parameter is ours.
//! Checked (decomp + exe): every address in this file.
//!
//! Combat = KH1 in Sora's story only; Combat = Re:CoM and Riku's story keep Re:CoM's enemy cards
//! untouched (nothing applied, every switched site Re:CoM's).

use crate::abilities::{ix, Bits};

/// The enemy cards whose skills this file builds.
pub mod card {
    pub const LARGE_BODY: u16 = 0x46;
    pub const BOUNCYWILD: u16 = 0x4C;
    pub const BANDIT: u16 = 0x4E;
    pub const FAT_BANDIT: u16 = 0x4F;
    pub const WIGHT_KNIGHT: u16 = 0x55;
    pub const GARGOYLE: u16 = 0x56;
    pub const NEOSHADOW: u16 = 0x5D;
    pub const GUARD_ARMOR: u16 = 0x63;
    pub const CARD_SOLDIER: u16 = 0x67;
    pub const OOGIE_BOOGIE: u16 = 0x6A;
    pub const XEMNAS: u16 = 0x76;
    pub const XALDIN: u16 = 0x78;
    pub const SAIX: u16 = 0x79;
    pub const ROXAS: u16 = 0x7C;
}

/// The skills built here and their help lines (NUL-terminated ASCII, within the help box): what
/// each does at KH1, with the owner's numbers. `crate::enemy_skills::SKILLS` takes these over its
/// own "(effect coming)" lines.
pub const BUILT: &[(u16, &[u8])] = &[
    (card::LARGE_BODY, b"Guard: hits to Sora's front do a tenth less damage.\0"),
    (card::BOUNCYWILD, b"Treasure Magnet: draws prizes in from 10 m (5 m without).\0"),
    (card::BANDIT, b"Combo Finish: every swing is a combo finisher.\0"),
    (card::FAT_BANDIT, b"Back Attack: swings from behind hit twice as hard.\0"),
    (card::WIGHT_KNIGHT, b"Float: Sora falls more slowly (his gravity x0.75).\0"),
    (card::GARGOYLE, b"Vanish: Sora fades in and out; faded, foes lose track.\0"),
    (card::NEOSHADOW, b"Bio: in battle every enemy loses 1 HP each second.\0"),
    (card::GUARD_ARMOR, b"Wide Attack: Sora's swings hit a 1.5 times wider area.\0"),
    (card::CARD_SOLDIER, b"Attack Haste: Sora swings 1.25 times as fast.\0"),
    (card::OOGIE_BOOGIE, b"Regen: every 6 seconds restores some of Sora's HP.\0"),
    (card::XEMNAS, b"Quick Barrier: a 1-second barrier after Sora takes a hit.\0"),
    (card::XALDIN, b"Aero Guard: a wind circles Sora, striking foes.\0"),
    (card::SAIX, b"Combo Boost: each combo swing hits harder than the last.\0"),
    (card::ROXAS, b"Double Strike: each swing strikes again at half power.\0"),
];

/// The help line of a built skill (None: not built here).
pub const fn built_help(card: u16) -> Option<&'static [u8]> {
    let mut k = 0;
    while k < BUILT.len() {
        if BUILT[k].0 == card {
            return Some(BUILT[k].1);
        }
        k += 1;
    }
    None
}

/// KH1's level-72 Treasure Magnet copy (Sora's list): the same effect as Bouncywild's.
pub const TREASURE_MAGNET_HELP: &[u8] = b"Draws prizes in from 10 m (5 m without).\0";

/// An equipped skill's bit.
fn on(equipped: Bits, card: u16) -> bool {
    crate::enemy_skills::of_card(card).is_some_and(|s| equipped & (1 << crate::enemy_skills::ability_of(s)) != 0)
}

/// What the equipped skills of route 1 switch on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Play {
    /// Bouncywild, or KH1's level-72 Treasure Magnet (owner: either copy).
    pub draw: bool,
    pub large_body: bool,
    pub combo_finish: bool,
    pub back_attack: bool,
    pub float: bool,
    pub vanish: bool,
    pub bio: bool,
    pub wide: bool,
    pub haste: bool,
    pub regen: bool,
    pub quick_barrier: bool,
    pub aero_guard: bool,
    pub combo_boost: bool,
    pub double_strike: bool,
}

pub fn play(equipped: Bits) -> Play {
    let o = |c| on(equipped, c);
    Play {
        draw: o(card::BOUNCYWILD) || equipped & (1 << ix::TREASURE_MAGNET[1]) != 0,
        large_body: o(card::LARGE_BODY),
        combo_finish: o(card::BANDIT),
        back_attack: o(card::FAT_BANDIT),
        float: o(card::WIGHT_KNIGHT),
        vanish: o(card::GARGOYLE),
        bio: o(card::NEOSHADOW),
        wide: o(card::GUARD_ARMOR),
        haste: o(card::CARD_SOLDIER),
        regen: o(card::OOGIE_BOOGIE),
        quick_barrier: o(card::XEMNAS),
        aero_guard: o(card::XALDIN),
        combo_boost: o(card::SAIX),
        double_strike: o(card::ROXAS),
    }
}

/// Sora's `player+0x420` bits Re:CoM's apply sets (`0x140163C00`, Checked (exe): `or [rcx+0x420],
/// imm` at `0x140163CED` Fat Bandit 0x100000, `0x140163CFC` Guard Armor 0x80, `0x140163D0B` Card
/// Soldier 0x100, `0x140163D9C` Xemnas 0x1000, `0x140163DEF` Saïx 0x40000000, `0x140163E31` Roxas
/// 0x20000000) and the clear `0x140161E10` takes away. Vanish's 0x40 and Aero Guard's 0x80000000
/// are their setters' (`0x1401F7EC0`, `0x1401F6FB0`).
pub mod flag420 {
    pub const BACK_ATTACK: u32 = 0x0010_0000;
    pub const WIDE: u32 = 0x80;
    pub const HASTE: u32 = 0x100;
    pub const QUICK_BARRIER: u32 = 0x1000;
    pub const COMBO_BOOST: u32 = 0x4000_0000;
    pub const DOUBLE_STRIKE: u32 = 0x2000_0000;
    pub const VANISH: u32 = 0x40;
    pub const AERO_GUARD: u32 = 0x8000_0000;
    /// Every plain bit route 1 writes (the setters' own are theirs).
    pub const PLAIN: u32 = BACK_ATTACK | WIDE | HASTE | QUICK_BARRIER | COMBO_BOOST | DOUBLE_STRIKE;
}

impl Play {
    /// The plain `+0x420` bits for these skills.
    pub fn flags_420(&self) -> u32 {
        use flag420::*;
        (if self.back_attack { BACK_ATTACK } else { 0 })
            | (if self.wide { WIDE } else { 0 })
            | (if self.haste { HASTE } else { 0 })
            | (if self.quick_barrier { QUICK_BARRIER } else { 0 })
            | (if self.combo_boost { COMBO_BOOST } else { 0 })
            | (if self.double_strike { DOUBLE_STRIKE } else { 0 })
    }

    /// Sora's side CBTL `+0x208` bits (`0x140163170`: Neoshadow `0x1000 << other side` = 0x2000 for
    /// side 1, Oogie Boogie `0x400 << side` = 0x400; Checked (exe `0x1401632B6`, `0x1401633C4`)).
    pub fn cbtl_bits(&self) -> u32 {
        (if self.bio { cbtl::BIO } else { 0 }) | (if self.regen { cbtl::REGEN } else { 0 })
    }

    pub fn any_battle(&self) -> bool {
        *self != Play { draw: self.draw, float: self.float, combo_finish: self.combo_finish, ..Play::default() }
    }
}

/// CBTL (`*0x14087A818`) fields of Sora's side.
pub mod cbtl {
    pub const FLAGS: usize = 0x208;
    /// Sora's Bio (his side 1's card works on side 2: `0x1000 << 1`).
    pub const BIO: u32 = 0x2000;
    /// Sora's Regen (`0x400 << 0`).
    pub const REGEN: u32 = 0x400;
    /// Their counters (`+0x1F8 + side × 4`: Regen side 0 at `+0x1F8`, Bio's "other side" 1 at `+0x1FC`).
    pub const REGEN_COUNT: usize = 0x1F8;
    pub const BIO_COUNT: usize = 0x1FC;
    /// Set by `0x140162250` (the battle's end: `+0x208 |= 0xC0000004`) before its clears.
    pub const ENDING: u32 = 0xC000_0004;
}

/// Re:CoM's Draw (`0x140163C00` case 0x4C, `0x140163D44`): `+0x54C` = 30.0, `+0x550` = 10.0; the
/// clear writes 5.0 / 5.0 (`0x140161E10`). `+0x550` is the prize pull radius (`0x140230C70`);
/// `+0x54C` has no reader (exe scan, stage 1).
pub mod draw {
    pub const FIELD_54C: usize = 0x54C;
    pub const RADIUS: usize = 0x550;
    pub const ON_54C: f32 = 30.0;
    pub const ON: f32 = 10.0;
    pub const OFF: f32 = 5.0;
}

/// Owner (2026-11-04, "Enemy Skills numbers"), Re:CoM's in brackets.
pub mod numbers {
    /// Card Soldier: swings × 1.25 (Re:CoM's row pick: 2.0, `0x1401F7A27`).
    pub const HASTE: f32 = 1.25;
    /// Wight Knight: Sora's gravity × 0.75 (Re:CoM: everyone's × 0.5, `0x1401615E0`).
    pub const FLOAT: f32 = 0.75;
    /// Guard Armor: the weapon's hit scale 0.75 (Re:CoM's `setWideAttack` 1.0; base 0.5): × 1.5.
    pub const WIDE_SCALE: f32 = 0.75;
    pub const BASE_SCALE: f32 = 0.5;
    /// Neoshadow: a Bio tick every 1 s = 3000 ticks (Re:CoM 1000, `0x140161991`).
    pub const BIO_TICKS: u32 = 3000;
    /// Oogie Boogie: a Regen tick every 6 s = 18000 ticks (Re:CoM 6000, `0x14016189D` / `0x1401600F2`).
    pub const REGEN_TICKS: u32 = 18000;
    /// Xemnas: the barrier for 1 s = 3000 ticks (Re:CoM 6000, `0x1401E73F3`).
    pub const BARRIER_TICKS: u32 = 3000;
    /// Large Body: damage to the front half × 90 % (Re:CoM: a block).
    pub const FRONT_CUT_PCT: i32 = 90;
    /// Saïx: + 20 power a combo step (`initAttack` `0x1401F0EEA`).
    pub const COMBO_BOOST_STEP: i32 = 20;
    /// Re:CoM's time: 3000 ticks a second (`docs/recom-timing.md`).
    pub const TICKS_PER_SECOND: u32 = 3000;
}

/// Large Body: a hit's value × 90 % (C's truncating division, as Re:CoM's ÷ 100). Re:CoM has no
/// number for it (its Guard blocks); the cut is ours, on Re:CoM's barrier geometry.
pub fn front_cut(v: i16) -> i16 {
    ((v as i32 * numbers::FRONT_CUT_PCT) / 100) as i16
}

/// Saïx's bonus at a KH1 swing start (Damage = Re:CoM): `initAttack`'s rule (`+0x664 + 20` while the
/// flag is on, else 0), but from 0 at a combo's first swing (owner: "+20 power a step"; Re:CoM's
/// card counts it for its 20 attack cards, a skill is permanent).
pub fn combo_boost_before(counter: u8, flags_420: u32, old: i32) -> i32 {
    if flags_420 & flag420::COMBO_BOOST == 0 || counter <= 1 {
        0
    } else {
        old
    }
}

/// Where a collider keeps the scale `0x140172690` writes: shape type 2 with `+0x64` bit 0 clear at
/// `+0x74`; types 4, 5, 7, 8, 0xB at `+0x70`; others none. Checked (decomp).
pub fn collider_scale(shape_type: u32, flags_64: u32, f70: f32, f74: f32) -> Option<f32> {
    match shape_type {
        2 if flags_64 & 1 == 0 => Some(f74),
        4 | 5 | 7 | 8 | 0xB => Some(f70),
        _ => None,
    }
}

/// Guard Armor on a KH1 swing: KH1's sphere radii × Re:CoM's own wide scale over its base, while
/// Re:CoM's `setWideAttack` flag is on the weapon (`+0x2CC & 1`), else 1. Clamped to Re:CoM's own
/// range (× 1 .. × 2).
pub fn wide_factor(wide_flag: bool, scale: Option<f32>) -> f32 {
    match (wide_flag, scale) {
        (true, Some(s)) if s.is_finite() => (s / numbers::BASE_SCALE).clamp(1.0, 2.0),
        _ => 1.0,
    }
}

/// Fat Bandit at Damage = KH1: Re:CoM's apply doubled a back hit (`0x14016F1C0`: from behind, the
/// attack's `+0x4C` back bonus = its power `+0x48`, the value `+0x48 + +0x4C`); KH1's number takes
/// the part's value, so the doubling is carried over to it. `carried` the part's value Re:CoM's
/// apply wrote, `power` / `bonus` the weapon collision's `+0x48` / `+0x4C` (weapon `+0x288` /
/// `+0x28C`).
pub fn back_hit(carried: i32, power: i32, bonus: i32) -> bool {
    bonus != 0 && power > 0 && bonus == power && carried == power + bonus
}

/// Bandit's Combo Finish: KH1's combo length 1 (every swing the combo's last).
pub const COMBO_FINISH_LENGTH: u8 = 1;

/// Wight Knight: the factor on Sora's gravity (KH1's in the KH1 controller, Re:CoM's at
/// `jump_height = recom`).
pub fn gravity_factor(p: &Play) -> f32 {
    if p.float {
        numbers::FLOAT
    } else {
        1.0
    }
}

/// Roxas' stub (`sites::ENEMY_SKILL_STUB_SITES`), placed at `stub`, back to `back`: Re:CoM's
/// `movsx eax, word [rbx+0x288]`, then Re:CoM's own ÷ 2 (`cdq; sub eax,edx; sar eax,1`, as its
/// Defender case `0x14015D66E`), Re:CoM's `mov [rbp-0x60], eax`, `jmp back`.
pub fn double_strike_stub(stub: u64, back: u64) -> [u8; 20] {
    let mut c = [0x0F, 0xBF, 0x83, 0x88, 0x02, 0x00, 0x00, 0x99, 0x2B, 0xC2, 0xD1, 0xF8, 0x89, 0x45, 0xA0, 0xE9, 0, 0, 0, 0];
    let rel = (back as i64 - (stub as i64 + 20)) as i32;
    c[16..20].copy_from_slice(&rel.to_le_bytes());
    c
}

/// The Roxas site's bytes in a KH1 battle: `jmp stub` and 5 nops (10 bytes, Re:CoM's two
/// instructions).
pub fn double_strike_jump(at: u64, stub: u64) -> [u8; 10] {
    let r = ((stub as i64 - (at as i64 + 5)) as i32).to_le_bytes();
    [0xE9, r[0], r[1], r[2], r[3], 0x90, 0x90, 0x90, 0x90, 0x90]
}

/// The CBTL tick sites' immediates (`cmp dword [..], imm32`): ours for the owner's times.
pub fn imm32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::ALL;
    use crate::enemy_skills::{ability_of, of_card, SKILLS};

    fn bit(c: u16) -> Bits {
        1 << ability_of(of_card(c).unwrap())
    }

    #[test]
    fn built_skills_and_their_lines() {
        for (c, h) in BUILT {
            let s = of_card(*c).unwrap_or_else(|| panic!("{c:#x}"));
            assert!(s.built, "{}", s.name);
            assert_eq!(s.help, *h, "{}", s.name);
            assert!(crate::abilities::built(ability_of(s)), "{}", s.name);
            let t = std::str::from_utf8(&h[..h.len() - 1]).unwrap();
            assert!(!t.contains("coming") && !t.contains("not built"), "{t}");
        }
        // Treasure Magnet's level-72 copy is built with Bouncywild's.
        assert!(crate::abilities::built(ix::TREASURE_MAGNET[1]));
        // Stage 2's fifteen (Dragon Maleficent's ×1.5 is the hit rewrite's, stage 3).
        assert_eq!(BUILT.len(), 14);
        assert!(built_help(0x6D).is_none());
        assert!(SKILLS.iter().filter(|s| s.built).count() >= 4 + 14);
    }

    #[test]
    fn play_from_the_equipped_bits() {
        assert_eq!(play(0), Play::default());
        assert!(play(bit(card::BOUNCYWILD)).draw);
        assert!(play(1 << ix::TREASURE_MAGNET[1]).draw, "KH1's level-72 copy too (owner: either copy)");
        let all: Bits = BUILT.iter().map(|(c, _)| bit(*c)).fold(0, |a, b| a | b);
        let p = play(all);
        assert!(p.large_body && p.combo_finish && p.back_attack && p.float && p.vanish && p.bio && p.wide && p.haste && p.regen && p.quick_barrier && p.aero_guard && p.combo_boost && p.double_strike);
        assert_eq!(p.flags_420(), flag420::PLAIN);
        assert_eq!(p.cbtl_bits(), 0x2400);
        assert!(p.any_battle());
        assert!(!play(bit(card::BOUNCYWILD) | bit(card::WIGHT_KNIGHT) | bit(card::BANDIT)).any_battle());
        // Every route-1 skill is a card ability of the table.
        for (c, _) in BUILT {
            assert!(matches!(ALL[ability_of(of_card(*c).unwrap())].source, crate::abilities::Source::Card(x) if x == *c));
        }
    }

    #[test]
    fn the_owners_numbers() {
        assert_eq!(front_cut(100), 90);
        assert_eq!(front_cut(15), 13);
        assert_eq!(front_cut(1), 0);
        assert_eq!(front_cut(-10), -9);
        assert_eq!(numbers::BIO_TICKS, numbers::TICKS_PER_SECOND);
        assert_eq!(numbers::REGEN_TICKS, 6 * numbers::TICKS_PER_SECOND);
        assert_eq!(numbers::BARRIER_TICKS, numbers::TICKS_PER_SECOND);
        assert_eq!(wide_factor(true, Some(numbers::WIDE_SCALE)), 1.5);
        assert_eq!(wide_factor(true, Some(1.0)), 2.0, "Re:CoM's own scale");
        assert_eq!(wide_factor(false, Some(1.0)), 1.0);
        assert_eq!(wide_factor(true, None), 1.0);
        assert_eq!(wide_factor(true, Some(0.5)), 1.0);
        assert_eq!(gravity_factor(&Play { float: true, ..Play::default() }), 0.75);
        assert_eq!(gravity_factor(&Play::default()), 1.0);
    }

    #[test]
    fn combo_boost_is_per_combo() {
        let f = flag420::COMBO_BOOST;
        // A combo's first swing starts from 0; later swings keep the last bonus (initAttack adds 20).
        assert_eq!(combo_boost_before(1, f, 80), 0);
        assert_eq!(combo_boost_before(2, f, 20), 20);
        assert_eq!(crate::damage::attack_bonus(f, combo_boost_before(1, f, 80)), 20);
        assert_eq!(crate::damage::attack_bonus(f, combo_boost_before(3, f, 40)), 60);
        // Without the flag: 0 (Re:CoM's own rule).
        assert_eq!(combo_boost_before(3, 0, 40), 0);
    }

    #[test]
    fn the_double_strike_stub() {
        let site = &crate::sites::ENEMY_SKILL_STUB_SITES[0];
        let at = 0x1_4000_0000u64 + site.rva as u64;
        let back = at + site.expect.len() as u64;
        assert_eq!(back, 0x1_4022_5D86);
        let stub = 0x1_4000_0000u64 - 0x10_0000;
        let c = double_strike_stub(stub, back);
        // Re:CoM's two instructions around the ÷ 2.
        assert_eq!(&c[..7], &site.expect[..7]);
        assert_eq!(&c[12..15], &site.expect[7..10]);
        assert_eq!(&c[7..12], &[0x99, 0x2B, 0xC2, 0xD1, 0xF8]);
        assert_eq!(c[15], 0xE9);
        assert_eq!(stub as i64 + 20 + i32::from_le_bytes(c[16..20].try_into().unwrap()) as i64, back as i64);
        let j = double_strike_jump(at, stub);
        assert_eq!(at as i64 + 5 + i32::from_le_bytes(j[1..5].try_into().unwrap()) as i64, stub as i64);
        assert_eq!(&j[5..], &[0x90; 5]);
        // C's ÷ 2 (toward 0), as the stub computes it.
        let half = |v: i32| (v - (v >> 31)) >> 1;
        assert_eq!([half(15), half(14), half(-3), half(1)], [7, 7, -1, 0]);
    }

    #[test]
    fn colliders_and_back_hits() {
        assert_eq!(collider_scale(2, 0, 1.0, 0.75), Some(0.75));
        assert_eq!(collider_scale(2, 1, 1.0, 0.75), None);
        for t in [4, 5, 7, 8, 0xB] {
            assert_eq!(collider_scale(t, 0, 0.75, 0.0), Some(0.75));
        }
        assert_eq!(collider_scale(1, 0, 0.75, 0.75), None);
        assert!(back_hit(30, 15, 15));
        assert!(!back_hit(15, 15, 15), "a front hit after a back one: +0x4A, not doubled");
        assert!(!back_hit(16, 15, 1), "setBackAttack's 1: no back hit yet");
        assert!(!back_hit(0, 0, 0));
    }
}
