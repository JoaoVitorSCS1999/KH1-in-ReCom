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

/// Stage 3, route 2 (the hit rewrite `0x14015D5F0`): the skills Re:CoM's rewrite tests, in card
/// order (Hades is KH1's Berserk, built in stage 1, and stays out).
pub const BUILT_3: &[(u16, &[u8])] = &[
    (0x47, b"Fire Boost: Fire magic does 1.5 times the damage.\0"),
    (0x48, b"Blizzard Boost: Blizzard magic does 1.5 times the damage.\0"),
    (0x49, b"Thunder Boost: Thunder magic does 1.5 times the damage.\0"),
    (0x4A, b"Cure Boost: Cure restores 1.5 times the HP.\0"),
    (0x51, b"Drain: Sora's swings drain HP from the foes they hit.\0"),
    (0x5A, b"Protect: physical attacks do a quarter less damage to Sora.\0"),
    (0x5C, b"Magic Boost: magic does 1.3 times the damage.\0"),
    (0x62, b"Summon Boost: summons do 1.3 times the damage.\0"),
    (0x6B, b"Shell: Fire, Blizzard and Thunder do a quarter less to Sora.\0"),
    (0x6D, b"Overdrive: Sora's swings do 1.5 times the damage.\0"),
    (0x73, b"Warp Break: a combo finisher may warp a foe away (not bosses).\0"),
    (0x75, b"Confu-Strike: Sora's swings may confuse the foes they hit.\0"),
    (0x77, b"Shot Charge: Fire, Blizzard and Gravity do 1.5 times the damage.\0"),
    (0x7A, b"Water Charge: Blizzard and water attacks do 1.75 times the damage.\0"),
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
    let mut k = 0;
    while k < BUILT_3.len() {
        if BUILT_3[k].0 == card {
            return Some(BUILT_3[k].1);
        }
        k += 1;
    }
    None
}

/// The equipped route-2 skills (bit k = `BUILT_3[k]`).
pub fn rewrite_mask(equipped: Bits) -> u32 {
    BUILT_3.iter().enumerate().filter(|(_, (c, _))| on(equipped, *c)).fold(0, |m, (k, _)| m | 1 << k)
}

/// The card ids of a route-2 mask, in card order (the order Re:CoM's rewrite runs them).
pub fn rewrite_ids(mask: u32) -> impl Iterator<Item = u8> {
    BUILT_3.iter().enumerate().filter(move |(k, _)| mask & (1 << k) != 0).map(|(_, (c, _))| *c as u8)
}

/// Stage 3: Re:CoM's hit rewrite `0x14015D5F0(CBTL, record)` and the owner's numbers in it.
pub mod rewrite {
    /// The 0x20-byte record (`0x140160F90` + its callers `0x1401E66A0` / `0x1401E67B0`).
    pub const SIDE: usize = 0;
    pub const HANDLE: usize = 4;
    pub const KIND: usize = 8;
    pub const ATTR: usize = 0xC;
    pub const MODE: usize = 0x10;
    pub const STATUS: usize = 0x12;
    pub const AP: usize = 0x14;
    pub const HP: usize = 0x16;
    pub const MAX_HP: usize = 0x18;
    pub const FINISHER: usize = 0x1A;
    pub const ATTACK_CARD: usize = 0x1E;
    /// Sora's slot `CBTL+0x1E8` (byte; < 0x44: none).
    pub const SLOT: usize = 0x1E8;
    /// Re:CoM's reaction kinds the rewrite writes: Drain, Warp Break, Confu-Strike.
    pub const DRAIN: u32 = 0x15;
    pub const WARP: u32 = 0x1F;
    pub const CONFUSE: u32 = 10;

    /// The owner's numbers in the cells our stubs read: Zexion's chance (Re:CoM 25 %, `cmp r8d,
    /// 0x19` at `0x14015D70A`), Lexaeus' (67 %, `0x14015D74C`), Demyx's factor (× 2, `add ax,ax` at
    /// `0x14015D831`), Defender's and Ursula's (÷ 2, `0x14015D66A` / `0x14015D64D`), in %.
    pub const RECOM_CELLS: [i32; 5] = [25, 67, 200, 50, 50];
    pub const OWNER_CELLS: [i32; 5] = [10, 15, 175, 75, 75];
    /// Where each stub goes back (or jumps) in Re:CoM's rewrite.
    pub const RET_ZEXION: u64 = 0x1_4015_D714;
    pub const RET_LEXAEUS: u64 = 0x1_4015_D756;
    pub const SKIP: u64 = 0x1_4015_D83C;
    /// Re:CoM's ÷ 100 tail (`ecx` = the product; stores `[rbx+0x14]`, returns).
    pub const DIV100_TAIL: u64 = 0x1_4015_D7D6;
    pub const RET_DEFENDER: u64 = 0x1_4015_D673;
    pub const RET_URSULA: u64 = 0x1_4015_D656;
}

/// A mode-1 record as `0x1401E66A0` builds it for Sora (side 1): `+0x1E` "attack card" for
/// situations 1..7 (`player+0x630`), `+0x1A` too for 4 / 7 (the finishers). Checked (decomp).
#[allow(clippy::too_many_arguments)]
pub fn hit_record(handle: u32, kind: u32, attr: u32, status: u16, ap: i32, hp: i32, max_hp: i32, situation: u32) -> [u8; 0x20] {
    use rewrite::*;
    let mut r = [0u8; 0x20];
    r[SIDE..SIDE + 4].copy_from_slice(&1u32.to_le_bytes());
    r[HANDLE..HANDLE + 4].copy_from_slice(&handle.to_le_bytes());
    r[KIND..KIND + 4].copy_from_slice(&kind.to_le_bytes());
    r[ATTR..ATTR + 4].copy_from_slice(&attr.to_le_bytes());
    r[MODE..MODE + 2].copy_from_slice(&1u16.to_le_bytes());
    r[STATUS..STATUS + 2].copy_from_slice(&status.to_le_bytes());
    r[AP..AP + 2].copy_from_slice(&(ap.clamp(i16::MIN as i32, i16::MAX as i32) as i16).to_le_bytes());
    r[HP..HP + 2].copy_from_slice(&(hp as i16).to_le_bytes());
    r[MAX_HP..MAX_HP + 2].copy_from_slice(&(max_hp as i16).to_le_bytes());
    if matches!(situation, 4 | 7) {
        r[FINISHER] = 1;
    }
    if matches!(situation, 1..=7) {
        r[ATTACK_CARD] = 1;
    }
    r
}

/// Re:CoM's ÷ 100 (`mov eax,0x51EB851F; imul ecx; sar edx,5; shr; add`): C's truncating division.
pub fn recom_div100(x: i32) -> i32 {
    let q = ((x as i64 * 0x51EB_851F) >> 32) as i32 >> 5;
    q + ((q as u32) >> 31) as i32
}

/// The stub block (`win::enemy_skills`, placed near the rewrite): the five cells, then the five
/// stubs, each Re:CoM's instructions with its constant read from its cell. Returns the bytes and
/// each stub's offset (Zexion, Lexaeus, Demyx, Defender, Ursula).
pub fn rewrite_block(at: u64) -> (Vec<u8>, [usize; 5]) {
    use rewrite::*;
    let mut b = vec![0u8; 0x20];
    for (k, v) in RECOM_CELLS.iter().enumerate() {
        b[k * 4..k * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    let rel = |from_end: usize, to: u64| -> [u8; 4] { ((to as i64 - (at as i64 + from_end as i64)) as i32).to_le_bytes() };
    let cell = |k: usize| at + k as u64 * 4;
    let mut offs = [0usize; 5];
    // Zexion / Lexaeus: cmp r8d,[cell]; jge skip; jmp back.
    for (k, back) in [(0usize, RET_ZEXION), (1, RET_LEXAEUS)] {
        offs[k] = b.len();
        b.extend([0x44, 0x3B, 0x05]);
        let e = b.len() + 4;
        b.extend(rel(e, cell(k)));
        b.extend([0x0F, 0x8D]);
        let e = b.len() + 4;
        b.extend(rel(e, SKIP));
        b.push(0xE9);
        let e = b.len() + 4;
        b.extend(rel(e, back));
    }
    // Demyx: movsx eax,[rbx+0x14]; imul eax,[cell]; mov ecx,eax; jmp Re:CoM's ÷ 100 tail.
    offs[2] = b.len();
    b.extend([0x0F, 0xBF, 0x43, 0x14, 0x0F, 0xAF, 0x05]);
    let e = b.len() + 4;
    b.extend(rel(e, cell(2)));
    b.extend([0x8B, 0xC8, 0xE9]);
    let e = b.len() + 4;
    b.extend(rel(e, DIV100_TAIL));
    // Defender / Ursula: movsx eax,[rbx+0x14]; imul eax,[cell]; Re:CoM's ÷ 100 into eax; jmp back.
    for (k, back) in [(3usize, RET_DEFENDER), (4, RET_URSULA)] {
        offs[k] = b.len();
        b.extend([0x0F, 0xBF, 0x43, 0x14, 0x0F, 0xAF, 0x05]);
        let e = b.len() + 4;
        b.extend(rel(e, cell(k)));
        b.extend([0x8B, 0xC8, 0xB8, 0x1F, 0x85, 0xEB, 0x51, 0xF7, 0xE9, 0xC1, 0xFA, 0x05, 0x8B, 0xC2, 0xC1, 0xE8, 0x1F, 0x03, 0xC2, 0xE9]);
        let e = b.len() + 4;
        b.extend(rel(e, back));
    }
    (b, offs)
}

/// A rewrite site's bytes in a KH1 battle: `jmp stub`, then nops to its length.
pub fn jump_to(at: u64, stub: u64, len: usize) -> Vec<u8> {
    let mut v = vec![0xE9];
    v.extend(((stub as i64 - (at as i64 + 5)) as i32).to_le_bytes());
    v.resize(len, 0x90);
    v
}

/// Lexaeus "never bosses": a hit whose kind Warp Break made 0x1F takes its kind from before on a
/// boss (Re:CoM's class name `CBoss…`).
pub fn warp_kind(kind: u32, from: Option<u32>, rtti: &str) -> u32 {
    match from {
        Some(k) if kind == rewrite::WARP && rtti.starts_with(".?AVCBoss") => k,
        _ => kind,
    }
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
        // Stage 2's fourteen; stage 3's fourteen (Dragon Maleficent's ×1.5 the hit rewrite's).
        assert_eq!(BUILT.len(), 14);
        assert_eq!(BUILT_3.len(), 14);
        for (c, h) in BUILT_3 {
            let s = of_card(*c).unwrap_or_else(|| panic!("{c:#x}"));
            assert!(s.built && s.help == *h, "{}", s.name);
        }
        assert!(BUILT_3.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(built_help(0x68).is_none() && of_card(0x68).unwrap().built, "Hades: stage 1's Berserk");
        assert!(!BUILT_3.iter().any(|(c, _)| *c == 0x68));
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
    fn route_2_masks_and_records() {
        let m = rewrite_mask(bit(0x47) | bit(0x7A) | bit(card::ROXAS));
        assert_eq!(rewrite_ids(m).collect::<Vec<_>>(), vec![0x47, 0x7A]);
        assert_eq!(rewrite_mask(0), 0);
        // 0x1401E66A0's rule: situations 1..7 attack cards, 4 / 7 also finishers.
        let r = hit_record(0x1234, 1, 0, 0x4, 300, 50, 100, 4);
        assert_eq!(u32::from_le_bytes(r[0..4].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(r[0x10..0x12].try_into().unwrap()), 1);
        assert_eq!(i16::from_le_bytes(r[0x14..0x16].try_into().unwrap()), 300);
        assert_eq!((r[0x1A], r[0x1E]), (1, 1));
        let r = hit_record(0, 1, 0, 0, 40000, 0, 0, 0);
        assert_eq!((r[0x1A], r[0x1E]), (0, 0), "a spell: situation 0");
        assert_eq!(i16::from_le_bytes(r[0x14..0x16].try_into().unwrap()), i16::MAX);
        assert_eq!(hit_record(0, 1, 0, 0, 1, 0, 0, 2)[0x1A], 0);
        assert_eq!(warp_kind(0x1F, Some(1), ".?AVCBossLexaeus@@"), 1);
        assert_eq!(warp_kind(0x1F, Some(1), ".?AVCEnemyShadow@@"), 0x1F);
        assert_eq!(warp_kind(0x1F, None, ".?AVCBossLexaeus@@"), 0x1F);
    }

    /// The rewrite's stubs: at Re:CoM's cells every result is Re:CoM's own; at the owner's, the
    /// owner's numbers.
    #[test]
    fn the_rewrite_stubs() {
        use rewrite::*;
        // Re:CoM's ÷ 100 is C's truncating division.
        for x in [-32768 * 200, -1999, -150, -1, 0, 1, 99, 150, 1999, 32767 * 200] {
            assert_eq!(recom_div100(x), x / 100, "{x}");
        }
        for x in i16::MIN as i32..=i16::MAX as i32 {
            // Demyx: Re:CoM's `add ax,ax` (16 bits) = the stub's x × 200 ÷ 100 (16 bits stored).
            assert_eq!((x as i16).wrapping_add(x as i16), recom_div100(x * RECOM_CELLS[2]) as i16, "{x}");
            // Defender / Ursula: Re:CoM's `cdq; sub; sar 1` = x × 50 ÷ 100.
            assert_eq!((x - (x >> 31)) >> 1, recom_div100(x * RECOM_CELLS[3]), "{x}");
        }
        assert_eq!(recom_div100(100 * OWNER_CELLS[2]) as i16, 175);
        assert_eq!(recom_div100(100 * OWNER_CELLS[3]), 75);
        // `cmp r8d, cell; jge skip`: hit when the roll (0..99) is below the cell.
        assert_eq!(RECOM_CELLS[0], 0x19);
        assert_eq!(RECOM_CELLS[1], 0x43);
        let at = 0x1_4000_0000u64 - 0x2_0000;
        let (b, offs) = rewrite_block(at);
        let rel = |o: usize| at as i64 + o as i64 + 4 + i32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as i64;
        for k in 0..5 {
            assert_eq!(i32::from_le_bytes(b[k * 4..k * 4 + 4].try_into().unwrap()), RECOM_CELLS[k]);
        }
        for (k, back) in [(0usize, RET_ZEXION), (1, RET_LEXAEUS)] {
            let o = offs[k];
            assert_eq!(&b[o..o + 3], &[0x44, 0x3B, 0x05]);
            assert_eq!(rel(o + 3), (at + k as u64 * 4) as i64);
            assert_eq!(&b[o + 7..o + 9], &[0x0F, 0x8D]);
            assert_eq!(rel(o + 9), SKIP as i64);
            assert_eq!(b[o + 13], 0xE9);
            assert_eq!(rel(o + 14), back as i64);
        }
        let o = offs[2];
        assert_eq!(rel(o + 7), (at + 8) as i64);
        assert_eq!(rel(o + 14), DIV100_TAIL as i64);
        for (k, back) in [(3usize, RET_DEFENDER), (4, RET_URSULA)] {
            let o = offs[k];
            assert_eq!(rel(o + 7), (at + k as u64 * 4) as i64);
            assert_eq!(b[o + 30], 0xE9);
            assert_eq!(rel(o + 31), back as i64);
            assert_eq!(o + 35, if k == 3 { offs[4] } else { b.len() });
        }
        let j = jump_to(0x1_4015_D70A, at + offs[0] as u64, 10);
        assert_eq!(j.len(), 10);
        assert_eq!(0x1_4015_D70Fi64 + i32::from_le_bytes(j[1..5].try_into().unwrap()) as i64, (at + offs[0] as u64) as i64);
        assert!(j[5..].iter().all(|&x| x == 0x90));
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
