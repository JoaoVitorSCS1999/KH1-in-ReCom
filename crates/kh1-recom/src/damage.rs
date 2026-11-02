//! The "Damage" setting (owner, 2026-11-01; `docs/recom-kh1-attack.md` § Re:CoM damage setting):
//! `damage = recom | kh1`, Re:CoM by default, only at Combat = KH1 in Sora's story.
//!
//! - **Re:CoM** (default): Re:CoM's own enemy HP (no KH1 twin / proxy HP), and every KH1 Keyblade
//!   swing carries the AP Re:CoM's row pick `0x1401F7520` computes for a card: the equipped
//!   Keyblade's card (`save + 0x2674`, `crate::keyblades`) at a power column of Re:CoM's
//!   `BattleCardList` ([`swing_column`]), plus Re:CoM's bonus `+0x664`, × Sora's attack stat
//!   `+0x43C` ÷ 100 ([`card_ap`]); the attribute and status bits are the card's, and Re:CoM's own
//!   enemy-card rewrite `0x1401E66A0` runs on them. Fire / Blizzard / Thunder / Gravity / Stop and
//!   Aero's wind carry their Re:CoM magic card's (level 1) or sleight's (levels 2, 3) AP
//!   ([`spell_source`]), Gravity and Stop with Re:CoM's kinds 0xD / 8 ([`recom_kind`]); Sonic Blade
//!   its sleight's ([`sonic_blade`]); Gravity Break and Zantetsuken KH1's % of Re:CoM's HP
//!   ([`percent_of_hp_row`]). Re:CoM's damage path then applies unchanged.
//!
//! The setting is ini-only (`kh1-recom.ini`, read at start): Re:CoM's Config has no free item slot.
//! - **KH1**: decisions 12 / 21 (KH1's formula on the twins' / proxies' KH1 records and HP).
//!
//! Everything here is Re:CoM's own arithmetic, Checked (decomp + disasm `0x1401F7520`,
//! `0x1401F0D90`, `0x1402203D0`, `0x140220DF0`, `0x140218950`; exe rows `0x1407C6C70`; the tables'
//! layout against the session's `SY0002.BIN`).

use crate::attack::{recom_id, COUNTERATTACK_ROW, RECOM_ROWS_1_7};
use kh1_core::data::{row, Moveset};

/// `*0x14087A808`: Re:CoM's `BattleCardList` (resource "BattleCardList", set by the `CCardBattle`
/// ctor `0x1401607A0` (`0x140160849`)). 16-byte rows by card kind: u16 power for columns 0..3 at
/// `+0 / +2 / +4 / +6`, `+8` the recovery index (`0x1407C7830[]`), `+9` status nibbles, `+0xA` CP,
/// `+0xB` attribute. Checked (decomp + disasm `0x1401F7AD5..0x1401F7B49`; `SY0002.BIN` entry 1 at
/// file offset 0x270, 0x7F0 bytes = 127 rows).
pub const BATTLE_CARD_LIST: u64 = 0x1_4087_A808;
/// `*0x14087A810`: Re:CoM's `StockSkillList` (set by the same ctor, `0x140160879`). 16-byte rows by
/// sleight id (`player+0x82C`): u16 power for columns 0..3, `+8` a u16 kind, `+0xA` attribute, `+0xB`
/// status nibbles. Checked (decomp `0x140220DF0`, `0x140218950`; `SY0002.BIN` entry 2 at 0xA60,
/// 0x960 bytes = 150 rows).
pub const STOCK_SKILL_LIST: u64 = 0x1_4087_A810;

/// Which table a row comes from: where its attribute and status bits sit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    /// `BattleCardList`: attribute `+0xB`, status `+9`.
    Card,
    /// `StockSkillList`: attribute `+0xA`, status `+0xB`.
    Sleight,
}

impl Table {
    pub fn attr_at(self) -> usize {
        match self {
            Table::Card => 0xB,
            Table::Sleight => 0xA,
        }
    }
    pub fn status_at(self) -> usize {
        match self {
            Table::Card => 9,
            Table::Sleight => 0xB,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Table::Card => "card",
            Table::Sleight => "sleight",
        }
    }
}

/// How Re:CoM turns a row's power into AP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// power × attack stat ÷ 100 (cards' swings and spells; `initStockSkill` mode 1).
    Stat,
    /// power × 10 ÷ 100 (Cure cards, Cura / Curaga: `initStockSkill` mode 2).
    Tenth,
    /// power as is (Gravity / Stop: `initStockSkill` mode 3, `initMagic` 0x1E / 0x1F).
    Raw,
}

/// The AP Re:CoM writes to `player+0x660`, with the attribute (`+0x66C`) and the status bits it
/// passes to its enemy-card rewrite `0x1401E66A0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecomAp {
    pub ap: i32,
    pub attr: u32,
    pub status: u16,
    /// The power used (after the bonus and the ×1.5), for the log.
    pub power: i32,
}

/// A row's u16 power at a column (0..=3).
pub fn power(row: &[u8; 16], col: u8) -> i32 {
    let c = (col.min(3) as usize) * 2;
    u16::from_le_bytes([row[c], row[c + 1]]) as i32
}

/// The status bits from a row's status byte: `1 << (low − 1)` | `1 << (high − 1)` for each
/// non-zero nibble (`0x1401F7B49..0x1401F7B70`).
pub fn status_bits(b: u8) -> u16 {
    let lo = b & 0xF;
    let hi = b >> 4;
    let l = if lo != 0 { 1u16 << (lo - 1) } else { 0 };
    let h = if hi != 0 { 1u16 << (hi - 1) } else { 0 };
    l | h
}

/// `+0x420` bit Re:CoM's card play sets from `+0x829` (`0x1401F5580`; Riku's Dark Mode,
/// Assumption): the row pick adds half the power (`0x1401F7AE8`).
pub const HALF_UP_FLAG: u32 = 0x0400_0000;
/// `+0x420` bit enemy card 0x79's effect sets (`0x140163C00` case 0x79): `initAttack` adds 20 to
/// the bonus `+0x664` for each attack (`0x1401F0E5F`), else the bonus is 0.
pub const BONUS_FLAG: u32 = 0x4000_0000;
/// The bonus `initAttack` (`0x1401F0D90`) leaves in `+0x664` before the row pick reads it.
pub fn attack_bonus(flags_420: u32, old: i32) -> i32 {
    if flags_420 & BONUS_FLAG != 0 {
        old.wrapping_add(20)
    } else {
        0
    }
}

/// `0x1401F7520`'s card branch: (power at `col` + bonus, + half when `half_up`) × stat ÷ 100, in
/// 32-bit ints (C's truncating division); the card's attribute `+0xB` and status bits `+9`.
pub fn card_ap(card_row: &[u8; 16], col: u8, stat: i32, bonus: i32, half_up: bool) -> RecomAp {
    let mut p = power(card_row, col).wrapping_add(bonus);
    if half_up {
        p = p.wrapping_add(p / 2);
    }
    RecomAp { ap: stat.wrapping_mul(p) / 100, attr: card_row[Table::Card.attr_at()] as u32, status: status_bits(card_row[Table::Card.status_at()]), power: p }
}

/// `initMagic` / `initStockSkill` / `exeStockSkill`: a row's power at `col` by the rule; the
/// attribute and status bits at the table's places.
pub fn row_ap(table: Table, row: &[u8; 16], col: u8, stat: i32, rule: Rule) -> RecomAp {
    let p = power(row, col);
    let ap = match rule {
        Rule::Stat => p.wrapping_mul(stat) / 100,
        Rule::Tenth => p * 10 / 100,
        Rule::Raw => p,
    };
    RecomAp { ap, attr: row[table.attr_at()] as u32, status: status_bits(row[table.status_at()]), power: p }
}

/// Re:CoM's power columns (owner, 2026-11-01): **normal** 0, **finisher** 2.
pub const COL_NORMAL: u8 = 0;
pub const COL_FINISHER: u8 = 2;

/// The Re:CoM row (1..=7) whose column a KH1 row takes when no owner rule names it: the row it is
/// built on (`attack::ROW_MAP`, `attack::MODIFIER_ROWS`; the Counterattack copies row 1).
pub fn base_recom_row(kh1_row: usize) -> Option<u32> {
    if kh1_row == COUNTERATTACK_ROW {
        return Some(1);
    }
    recom_id(kh1_row)
}

/// The column of a KH1 swing row (owner, 2026-11-01):
///
/// - KH1's combo finishers (records of reaction kind 5: the ground and air finishers, Blitz,
///   Ripple Drive, Hurricane Blast) → the finisher column 2;
/// - KH1's normal ground swings (swing 1, swing 2, the close second swing "lunge" row 14) → the
///   normal column 0;
/// - every other row (air swings, jump-up, hop, Counterattack, the other combo modifiers) → the
///   column of the Re:CoM row it is built on ([`base_recom_row`]; Re:CoM's rows 1..7 hold
///   0, 0, 1, 2, 1, 0, 2, Checked (exe `0x1407C6C80`)).
///
/// None: not a swing row (Sonic Blade's rows take its sleight's, [`sonic_blade`]).
pub fn swing_column(kh1_row: usize, m: &Moveset) -> Option<u8> {
    let rw = m.row(kh1_row)?;
    let data = m.attack(rw.attack)?;
    let base = base_recom_row(kh1_row)?;
    if data.kind == 5 {
        return Some(COL_FINISHER);
    }
    if matches!(kh1_row, row::SWING_1 | row::SWING_2 | row::LUNGE) {
        return Some(COL_NORMAL);
    }
    RECOM_ROWS_1_7.get(base as usize - 1).map(|r| r[0xA])
}

/// Where a KH1 spell level's Re:CoM AP comes from: its Re:CoM magic card (level 1) or the sleight
/// of that level (2, 3), the Re:CoM row its KH1 row is built on (column `+0xA`), and the rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellSource {
    pub table: Table,
    pub id: u16,
    pub recom_row: u8,
    pub col: u8,
    pub rule: Rule,
}

/// `initMagic` (cards 0x1A..0x20 → rows 0x22..0x28) and `initStockSkill` (sleights 0xD..0x1A →
/// rows 0x29..0x3D). Checked (decomp `0x1402203D0`, `0x140220DF0`). Every one of these rows has
/// column 0 (Checked (exe), `crate::magic::RECOM_*_ROWS`).
pub fn spell_source(spell: u8, level: u8) -> Option<SpellSource> {
    use crate::magic::spell as s;
    // (card, row), (sleight, row) for -ra, (sleight, row) for -ga, the rule.
    let (card, ra, ga, rule): ((u16, u8), (u16, u8), (u16, u8), Rule) = match spell {
        s::FIRE => ((0x1A, 0x22), (0x0D, 0x29), (0x14, 0x2A), Rule::Stat),
        s::BLIZZARD => ((0x1B, 0x23), (0x0E, 0x2B), (0x15, 0x2C), Rule::Stat),
        s::THUNDER => ((0x1C, 0x24), (0x0F, 0x2E), (0x16, 0x2F), Rule::Stat),
        s::CURE => ((0x1D, 0x25), (0x10, 0x36), (0x17, 0x37), Rule::Tenth),
        s::GRAVITY => ((0x1E, 0x26), (0x11, 0x38), (0x18, 0x39), Rule::Raw),
        s::STOP => ((0x1F, 0x27), (0x12, 0x3A), (0x19, 0x3B), Rule::Raw),
        s::AERO => ((0x20, 0x28), (0x13, 0x3C), (0x1A, 0x3D), Rule::Stat),
        _ => return None,
    };
    let col = recom_rows(spell)?.get(level.clamp(1, 3) as usize - 1)?[0xA];
    let ((id, r), table) = match level.clamp(1, 3) {
        1 => (card, Table::Card),
        2 => (ra, Table::Sleight),
        _ => (ga, Table::Sleight),
    };
    Some(SpellSource { table, id, recom_row: r, col, rule })
}

/// Re:CoM's rows of a spell's three levels, as read from the Steam exe (`crate::magic`).
fn recom_rows(spell: u8) -> Option<&'static [[u8; 16]; 3]> {
    use crate::magic::{self as mg, spell as s};
    Some(match spell {
        s::FIRE => &mg::RECOM_FIRE_ROWS,
        s::BLIZZARD => &mg::RECOM_BLIZZARD_ROWS,
        s::THUNDER => &mg::RECOM_THUNDER_ROWS,
        s::CURE => &mg::RECOM_CURE_ROWS,
        s::GRAVITY => &mg::RECOM_GRAVITY_ROWS,
        s::STOP => &mg::RECOM_STOP_ROWS,
        s::AERO => &mg::RECOM_AERO_ROWS,
        _ => return None,
    })
}

/// The spells whose hits carry Re:CoM's card / sleight AP at Damage = Re:CoM (owner, 2026-11-01:
/// Fire / Blizzard / Thunder; then Gravity, Stop and Aero's wind "use Re:CoM's own numbers"). Cure
/// already heals Re:CoM's amounts at both settings (decision 34). Aero's barrier stays KH1's (it is
/// no hit). `docs/recom-m7-magic.md` § Spell damage at Damage = Re:CoM.
pub fn recom_spell(spell: u8) -> bool {
    use crate::magic::spell as s;
    matches!(spell, s::FIRE | s::BLIZZARD | s::THUNDER | s::GRAVITY | s::STOP | s::AERO)
}

/// The spells whose Re:CoM numbers only exist with Re:CoM's kind (Gravity's % of HP 0xD, Stop's
/// ticks 8): outside a card battle (no `BattleCardList` AP) their hits keep KH1's rules, where the
/// others take Re:CoM's cardless AP (the attack stat), as Re:CoM's field swing.
pub fn needs_recom_kind(spell: u8) -> bool {
    use crate::magic::spell as s;
    matches!(spell, s::GRAVITY | s::STOP)
}

/// Whether a spell's cast records "took Re:CoM's card / sleight numbers" for its hits or its
/// shape: Gravity and Stop (their hits are Re:CoM's kinds only then), Aero (level 1's wind, below).
pub fn tracks_recom_cast(spell: u8) -> bool {
    needs_recom_kind(spell) || spell == crate::magic::spell::AERO
}

/// Whether an Aero cast has the wind (owner, 2026-11-01, follow-ups 2, "1 (b)"): KH1's Aerora and
/// Aeroga always (`magic::aero::has_wind`); **Aero (level 1) too at Damage = Re:CoM when the cast
/// took Re:CoM's Aero card's AP** (`recom_card`: a battle room with the live table), carrying that
/// value. KH1's level 1 has no wind (no wind record; not KH1, the owner's choice): its wind is
/// Aerora's (`magic::aero_wind_attack`: radius 80 cm × the size, re-hit every 30 frames, following
/// Sora for the whole barrier, KH1's flinch). At Damage = KH1, or with no card AP (the field), level
/// 1 stays KH1's: barrier only.
pub fn aero_wind(level: u8, recom_card: bool) -> bool {
    crate::magic::aero::has_wind(level) || (level.clamp(1, 3) == 1 && recom_card)
}

/// The hit kind Re:CoM's own card / sleight of this level gives its attack (`initMagic` /
/// `initStockSkill`: `+0x668` = the row's `+0xB`), for the spells whose Re:CoM effect is the kind:
/// Gravity / Gravira / Graviga **0xD** (rows 0x26 / 0x38 / 0x39: current HP × AP ÷ 100 behind the
/// param gate `+0x40`, then the flatten state `+0xAC0`), Stop / Stopra / Stopga **8** (rows 0x27 /
/// 0x3A / 0x3B: AP ticks into `+0x2D8` behind the gate `+0x2C`, then the stop state `+0x940`).
/// Checked (exe rows, decomp `0x14017DC50`, `0x14017D260`). None: the KH1 row's kind stands (Fire,
/// Blizzard, Thunder and Aero keep KH1's reactions, as the swings do).
pub fn recom_kind(spell: u8, level: u8) -> Option<u32> {
    if !needs_recom_kind(spell) {
        return None;
    }
    recom_rows(spell)?.get(level.clamp(1, 3) as usize - 1).map(|r| r[0xB] as u32)
}

/// Owner (2026-11-01, follow-ups 2): at Damage = Re:CoM, Gravity Break and Zantetsuken hits skip
/// Re:CoM's boss falloff (`+0x288` / `+0x28C` / `+0x290`: from a boss's 6th hit within 1500 ticks,
/// −0.25 a hit to 0.05, `0x14017F7E0` / `0x140185320`, applied by `0x14017DAB0`), so KH1's
/// `resist[8]` and floor alone reduce bosses. Its frozen bonus (`0x14015D940`), resistance and weak
/// parts stay. Checked (decomp).
pub fn skips_falloff(damage_kh1: bool, kh1_row: usize, m: &Moveset) -> bool {
    !damage_kh1 && percent_of_hp_row(kh1_row, m)
}

/// Re:CoM's Gravity kind (`0x14017DC50` case 0xD) and Stop kind (case 8). Checked (decomp).
pub const RECOM_GRAVITY_KIND: u32 = 0xD;
pub const RECOM_STOP_KIND: u32 = crate::magic::RECOM_STOP_KIND;

/// Re:CoM's enemy param block (`enemy+0x308`) gates its kinds read (`0x14017DC50`; 0 = immune):
/// Stop `+0x2C`, Gravity `+0x40`. Checked (disasm `0x14017DCB5`, `0x14017DD4C`).
pub fn recom_gate_offset(spell: u8) -> Option<usize> {
    use crate::magic::spell as s;
    match spell {
        s::STOP => Some(0x2C),
        s::GRAVITY => Some(0x40),
        _ => None,
    }
}

/// Decision 14 for a spell hit (KH1's hits land on attacking actors): the card-interrupt bit
/// `0x2000000` for an enemy in an attack (`+0x260 & 1`) that hasn't got it, **unless Re:CoM's own
/// gate for the hit's kind is shut** (`gate` Some(false): Re:CoM refuses the hit; an immune enemy's
/// attack goes on, as KH1's Stop on a `resist[9]` 0 enemy). `gate` None: no gate (KH1's rules, or
/// a kind without one).
pub fn interrupts(flags: u64, gate: Option<bool>) -> bool {
    flags & 1 != 0 && flags & 0x200_0000 == 0 && gate != Some(false)
}

/// What Re:CoM's kind 0xD takes from an enemy (`0x14017DC50` → `0x14017DAB0`), for tests and the
/// log: `(float)HP × AP / 100` (+ the frozen bonus `0x14015D940` when the enemy is frozen, `+0x2B0`
/// its "ice" effect: not modelled),
/// then `(int)(value × resistance % × +0x290 / 100)` in doubles, at least 1. Immune (gate 0): 0.
pub fn recom_gravity_damage(hp: i32, ap: i32, gate_open: bool, resist_pct: f32, scale_290: f32) -> i32 {
    if !gate_open {
        return 0;
    }
    let v = (hp as f32 * ap as f32) / 100.0;
    let d = ((v as f64) * (resist_pct as f64) * (scale_290 as f64) / 100.0) as i32;
    d.max(1)
}

/// The KH1 swing rows whose record is the **% of HP** element (8): Gravity Break (`0x4F68`, 40 +
/// Strength) and Zantetsuken (`0x4FD8`, 250, no stat term). At Damage = Re:CoM (owner, 2026-11-01)
/// these take **KH1's percentage of Re:CoM's HP** instead of the Keyblade card's column 2:
/// KH1's `0x1402BFA80` element 8 (`(int)(current HP × (power + stat [+ crit bonus]) / 100 ×
/// resist[8])`, at least the record's floor, 1 when it is under 2; no Defence, no cap) on the
/// enemy's Re:CoM current HP `+0x274`. Checked (KH1 decomp `0x1402BFA80`; btltbl records).
pub fn percent_of_hp_row(kh1_row: usize, m: &Moveset) -> bool {
    m.row(kh1_row).and_then(|rw| m.attack(rw.attack)).is_some_and(|a| a.element & 0xF == 8)
}

/// Re:CoM's Sonic Blade: sleight 9 (`initStockSkill` case 9), its dash row 0xF (column 0) and
/// finisher row 0x10 (column 2), power × attack stat ÷ 100 (`exeStockSkill` `0x140218950`
/// sub-states 0x8C → 0x8D and each chain, `0x140219A61..`). Checked (decomp + exe rows,
/// `sites::SPECIAL_CHECKED`).
pub mod sonic_blade {
    pub const SLEIGHT: u16 = crate::specials::recom::SLEIGHT;
    /// Re:CoM's rows 0xE (start), 0xF (dash), 0x10 (finisher) `+0xA`: 0, 0, 2. Checked (exe).
    pub fn column(recom_id: u32) -> Option<u8> {
        match recom_id {
            0xE | 0xF => Some(0),
            0x10 => Some(2),
            _ => None,
        }
    }
}

/// The hits trace's tag for a Re:CoM AP (`:recom <ap> col <n> card <kind>`).
pub fn trace_tag(ap: i32, col: u8, table: Table, id: u16) -> String {
    format!(":recom {ap} col {col} {} {id:#x}", table.name())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::magic::spell as s;
    use kh1_core::data::Moveset;

    /// A card row laid out as `BattleCardList`'s (powers 100 / 100 / 150, status 0x03, CP 10,
    /// attribute 0): the shape of Re:CoM's Kingdom Key row (layout Checked (files)).
    const KEY: [u8; 16] = [100, 0, 100, 0, 150, 0, 0, 0, 5, 0x03, 10, 0, 0, 0, 0, 0];
    /// A sleight row laid out as `StockSkillList`'s (powers 200 / 0 / 400, attribute 1, status 0).
    const SONIC: [u8; 16] = [200, 0, 0, 0, 0x90, 1, 0, 0, 0x01, 0x01, 1, 0, 0, 0, 0, 0];

    #[test]
    fn recom_rows_columns_as_read_from_the_exe() {
        // Owner's reading: rows 1 and 2 column 0, row 3 column 1, row 4 column 2; rows 5..7 too.
        let cols: Vec<u8> = RECOM_ROWS_1_7.iter().map(|r| r[0xA]).collect();
        assert_eq!(cols, vec![0, 0, 1, 2, 1, 0, 2]);
    }

    #[test]
    fn column_mapping() {
        let m = Moveset::kh1_defaults();
        let col = |r| swing_column(r, &m);
        // Normal ground swings: column 0 (owner), including the close second swing.
        assert_eq!(col(row::SWING_1), Some(0));
        assert_eq!(col(row::SWING_2), Some(0));
        assert_eq!(col(row::LUNGE), Some(0));
        // Finishers (KH1 kind 5): column 2.
        assert_eq!(col(row::GROUND_FINISHER), Some(2));
        assert_eq!(col(row::AIR_FINISHER), Some(2));
        assert_eq!(col(row::BLITZ), Some(2));
        assert_eq!(col(row::RIPPLE_DRIVE), Some(2));
        assert_eq!(col(row::HURRICANE_BLAST), Some(2));
        // Air swings / jump-up / hop: the Re:CoM row they are built on (6 → 0, 5 → 1).
        assert_eq!(col(row::AIR_SWING_1), Some(0));
        assert_eq!(col(row::AIR_SWING_2), Some(1));
        assert_eq!(col(row::JUMP_UP), Some(1));
        assert_eq!(col(row::HOP), Some(0));
        // The other modifiers: their Re:CoM row (5 → 1, 4 → 2, 1 → 0, 2 → 0).
        assert_eq!(col(row::AERIAL_SWEEP), Some(1));
        assert_eq!(col(row::STUN_IMPACT), Some(2));
        assert_eq!(col(row::GRAVITY_BREAK), Some(2));
        assert_eq!(col(row::ZANTETSUKEN), Some(2));
        assert_eq!(col(row::SLIDING_DASH), Some(0));
        assert_eq!(col(row::VORTEX), Some(0));
        // Counterattack: row 1's column.
        assert_eq!(col(row::COUNTERATTACK), Some(0));
        // Sonic Blade's rows are not swings.
        assert_eq!(col(row::SONIC_BLADE_DASH), None);
        assert_eq!(col(row::SONIC_BLADE_FINISHER), None);
    }

    #[test]
    fn card_ap_is_the_row_picks_formula() {
        // Attack stat 10: 10 × 100 / 100 = 10 (normal), 10 × 150 / 100 = 15 (finisher).
        assert_eq!(card_ap(&KEY, 0, 10, 0, false).ap, 10);
        assert_eq!(card_ap(&KEY, 2, 10, 0, false).ap, 15);
        // C's truncating division: 7 × 150 / 100 = 10 (10.5).
        assert_eq!(card_ap(&KEY, 2, 7, 0, false).ap, 10);
        // The bonus is added to the power; the ×1.5 adds half the sum (integer).
        assert_eq!(card_ap(&KEY, 0, 10, 20, false), RecomAp { ap: 12, attr: 0, status: 0b100, power: 120 });
        assert_eq!(card_ap(&KEY, 2, 10, 1, true).power, 151 + 75);
        // Status 0x03: low nibble 3 → bit 2; high nibble 0 → none.
        assert_eq!(status_bits(0x03), 0b100);
        assert_eq!(status_bits(0x93), (1 << 2) | (1 << 8));
        assert_eq!(status_bits(0), 0);
    }

    #[test]
    fn bonus_is_initattacks() {
        assert_eq!(attack_bonus(0, 40), 0);
        assert_eq!(attack_bonus(BONUS_FLAG, 40), 60);
        assert_eq!(attack_bonus(HALF_UP_FLAG, 40), 0);
    }

    #[test]
    fn sleight_and_spell_rules() {
        // Sonic Blade: dash column 0 (200), finisher column 2 (400), × stat ÷ 100.
        assert_eq!(sonic_blade::column(0xF), Some(0));
        assert_eq!(sonic_blade::column(0x10), Some(2));
        assert_eq!(row_ap(Table::Sleight, &SONIC, 0, 13, Rule::Stat).ap, 26);
        assert_eq!(row_ap(Table::Sleight, &SONIC, 2, 13, Rule::Stat).ap, 52);
        assert_eq!(row_ap(Table::Sleight, &SONIC, 0, 13, Rule::Stat).attr, 1);
        assert_eq!(row_ap(Table::Sleight, &SONIC, 2, 0, Rule::Tenth).ap, 40);
        assert_eq!(row_ap(Table::Sleight, &SONIC, 2, 0, Rule::Raw).ap, 400);
        // The card / sleight of each level and the Re:CoM row the KH1 row is built on.
        assert_eq!(spell_source(s::FIRE, 1), Some(SpellSource { table: Table::Card, id: 0x1A, recom_row: 0x22, col: 0, rule: Rule::Stat }));
        assert_eq!(spell_source(s::FIRE, 2).map(|x| (x.table, x.id, x.recom_row)), Some((Table::Sleight, 0xD, 0x29)));
        assert_eq!(spell_source(s::FIRE, 3).map(|x| (x.id, x.recom_row)), Some((0x14, 0x2A)));
        assert_eq!(spell_source(s::BLIZZARD, 3).map(|x| (x.id, x.recom_row)), Some((0x15, 0x2C)));
        assert_eq!(spell_source(s::THUNDER, 2).map(|x| (x.id, x.recom_row)), Some((0xF, 0x2E)));
        assert_eq!(spell_source(s::CURE, 2).map(|x| x.rule), Some(Rule::Tenth));
        assert_eq!(spell_source(s::GRAVITY, 1).map(|x| x.rule), Some(Rule::Raw));
        assert_eq!(spell_source(s::AERO, 3).map(|x| (x.id, x.recom_row)), Some((0x1A, 0x3D)));
        // Every magic row Re:CoM uses has column 0 (exe).
        for sp in 0..7 {
            for lv in 1..=3 {
                assert_eq!(spell_source(sp, lv).map(|x| x.col), Some(0));
            }
        }
        assert!(recom_spell(s::FIRE) && recom_spell(s::BLIZZARD) && recom_spell(s::THUNDER));
        // Owner (2026-11-01, follow-ups): Gravity, Stop and Aero's wind too; Cure is a heal.
        assert!(recom_spell(s::GRAVITY) && recom_spell(s::STOP) && recom_spell(s::AERO));
        assert!(!recom_spell(s::CURE));
        assert_eq!(trace_tag(15, 2, Table::Card, 1), ":recom 15 col 2 card 0x1");
    }

    /// The session's `SY0002.BIN` rows (Checked (files): `BattleCardList` at 0x270, `StockSkillList`
    /// at 0xA60): Gravity card 0x1E power 20, status 0x93, attribute 5; Gravira 0x11 40, Graviga
    /// 0x18 70 (attribute `+0xA` 5, status `+0xB` 0x93); Stop card 0x1F 9000, Stopra 0x12 15000,
    /// Stopga 0x19 21000; Aero card 0x20 300 (attribute 1), Aerora 0x13 600, Aeroga 0x1A 900.
    fn card(power: u16, status: u8, attr: u8) -> [u8; 16] {
        let p = power.to_le_bytes();
        [p[0], p[1], 0, 0, 0, 0, 0, 0, 0, status, 0, attr, 0, 0, 0, 0]
    }
    fn sleight(power: u16, attr: u8, status: u8) -> [u8; 16] {
        let p = power.to_le_bytes();
        [p[0], p[1], 0, 0, 0, 0, 0, 0, 0, 2, attr, status, 0, 0, 0, 0]
    }

    #[test]
    fn gravity_maps_to_recoms_card_and_sleights() {
        // KH1 level → Re:CoM's Gravity card, Gravira, Graviga; the power as is (initMagic 0x1E,
        // initStockSkill mode 3).
        let src: Vec<_> = (1..=3).map(|lv| spell_source(s::GRAVITY, lv).unwrap()).collect();
        assert_eq!(src.iter().map(|x| (x.table, x.id, x.recom_row, x.rule)).collect::<Vec<_>>(), vec![(Table::Card, 0x1E, 0x26, Rule::Raw), (Table::Sleight, 0x11, 0x38, Rule::Raw), (Table::Sleight, 0x18, 0x39, Rule::Raw)]);
        let rows = [card(20, 0x93, 5), sleight(40, 5, 0x93), sleight(70, 5, 0x93)];
        let aps: Vec<i32> = src.iter().zip(rows.iter()).map(|(x, r)| row_ap(x.table, r, x.col, 37, x.rule).ap).collect();
        assert_eq!(aps, vec![20, 40, 70], "20 / 40 / 70 % whatever the attack stat");
        let a = row_ap(Table::Card, &rows[0], 0, 37, Rule::Raw);
        assert_eq!((a.attr, a.status), (5, (1 << 2) | (1 << 8)));
        // Re:CoM's kind 0xD for every level (rows 0x26 / 0x38 / 0x39 +0xB).
        assert_eq!((1..=3).map(|lv| recom_kind(s::GRAVITY, lv)).collect::<Vec<_>>(), vec![Some(RECOM_GRAVITY_KIND); 3]);
        assert_eq!(recom_gate_offset(s::GRAVITY), Some(0x40));
        // Kind 0xD: HP × AP / 100, × resistance % × +0x290 / 100, at least 1; immune: nothing.
        assert_eq!(recom_gravity_damage(1000, 20, true, 100.0, 1.0), 200);
        assert_eq!(recom_gravity_damage(1000, 70, true, 50.0, 1.0), 350);
        assert_eq!(recom_gravity_damage(333, 40, true, 100.0, 1.0), 133, "133.2 truncated");
        assert_eq!(recom_gravity_damage(2, 20, true, 100.0, 1.0), 1, "at least 1");
        assert_eq!(recom_gravity_damage(1000, 70, false, 100.0, 1.0), 0, "gate +0x40 = 0: immune");
    }

    #[test]
    fn stop_maps_to_recoms_card_and_sleights() {
        let src: Vec<_> = (1..=3).map(|lv| spell_source(s::STOP, lv).unwrap()).collect();
        assert_eq!(src.iter().map(|x| (x.table, x.id, x.recom_row, x.rule)).collect::<Vec<_>>(), vec![(Table::Card, 0x1F, 0x27, Rule::Raw), (Table::Sleight, 0x12, 0x3A, Rule::Raw), (Table::Sleight, 0x19, 0x3B, Rule::Raw)]);
        let rows = [card(9000, 0x03, 5), sleight(15000, 5, 0x03), sleight(21000, 5, 0x03)];
        let ticks: Vec<i32> = src.iter().zip(rows.iter()).map(|(x, r)| row_ap(x.table, r, x.col, 37, x.rule).ap).collect();
        assert_eq!(ticks, vec![9000, 15000, 21000], "Re:CoM ticks (50 a frame): 3 / 5 / 7 s");
        // Every one fits the i16 the hit part carries (`+0x92`).
        assert!(ticks.iter().all(|&t| t <= i16::MAX as i32));
        assert_eq!((1..=3).map(|lv| recom_kind(s::STOP, lv)).collect::<Vec<_>>(), vec![Some(RECOM_STOP_KIND); 3]);
        assert_eq!(recom_gate_offset(s::STOP), Some(0x2C));
        assert!(needs_recom_kind(s::STOP) && needs_recom_kind(s::GRAVITY));
    }

    #[test]
    fn aero_wind_maps_to_recoms_card_and_sleights() {
        let src: Vec<_> = (1..=3).map(|lv| spell_source(s::AERO, lv).unwrap()).collect();
        assert_eq!(src.iter().map(|x| (x.table, x.id, x.recom_row, x.rule)).collect::<Vec<_>>(), vec![(Table::Card, 0x20, 0x28, Rule::Stat), (Table::Sleight, 0x13, 0x3C, Rule::Stat), (Table::Sleight, 0x1A, 0x3D, Rule::Stat)]);
        let rows = [card(300, 0x03, 1), sleight(600, 1, 0x03), sleight(900, 1, 0x03)];
        // Power × attack stat ÷ 100 (initMagic 0x20, initStockSkill mode 1), attack stat 13.
        let aps: Vec<(i32, u32)> = src.iter().zip(rows.iter()).map(|(x, r)| row_ap(x.table, r, x.col, 13, x.rule)).map(|a| (a.ap, a.attr)).collect();
        assert_eq!(aps, vec![(39, 1), (78, 1), (117, 1)]);
        // The wind keeps KH1's reaction (no Re:CoM kind), as Fire / Blizzard / Thunder.
        assert_eq!(recom_kind(s::AERO, 2), None);
        assert_eq!(recom_kind(s::FIRE, 1), None);
        assert!(!needs_recom_kind(s::AERO) && !needs_recom_kind(s::FIRE));
        assert_eq!(recom_gate_offset(s::AERO), None);
    }

    #[test]
    fn aero_level_1_wind_only_with_recoms_card() {
        // KH1: Aerora / Aeroga have the wind, Aero doesn't.
        assert_eq!((1..=3).map(|lv| aero_wind(lv, false)).collect::<Vec<_>>(), vec![false, true, true]);
        // Damage = Re:CoM with the Aero card's AP: level 1 gets the wind too (owner, "1 (b)").
        assert_eq!((1..=3).map(|lv| aero_wind(lv, true)).collect::<Vec<_>>(), vec![true, true, true]);
        // Its value: the Aero card 0x20, 300 × attack ÷ 100 (attack 13: 39), attribute 1.
        let src = spell_source(s::AERO, 1).unwrap();
        assert_eq!((src.table, src.id, src.recom_row, src.rule), (Table::Card, 0x20, 0x28, Rule::Stat));
        let a = row_ap(src.table, &card(300, 0x03, 1), src.col, 13, src.rule);
        assert_eq!((a.ap, a.attr), (39, 1));
        // Aero's casts record whether they took the card's numbers; Fire's don't need to.
        assert!(tracks_recom_cast(s::AERO) && tracks_recom_cast(s::GRAVITY) && tracks_recom_cast(s::STOP));
        assert!(!tracks_recom_cast(s::FIRE) && !tracks_recom_cast(s::CURE));
        // Level 1's wind is Aerora's KH1 data (radius 80, re-hit 30): the only KH1 wind record.
        let w1 = crate::magic::aero_wind_attack(1);
        let w2 = crate::magic::aero_wind_attack(2);
        assert_eq!((w1.radius, w1.kind), (w2.radius, w2.kind));
        assert_eq!((crate::magic::aero::WIND_RADIUS, crate::magic::aero::REHIT), (80.0, 30.0));
    }

    #[test]
    fn spell_interrupts_follow_recoms_gate() {
        // An attacking enemy: interrupted unless Re:CoM's gate refuses the hit.
        assert!(interrupts(1, None));
        assert!(interrupts(1, Some(true)));
        assert!(!interrupts(1, Some(false)));
        // Not attacking, or already interruptible: nothing to do.
        assert!(!interrupts(0, Some(true)));
        assert!(!interrupts(1 | 0x200_0000, Some(true)));
    }

    #[test]
    fn percent_rows_are_gravity_break_and_zantetsuken() {
        let m = Moveset::kh1_defaults();
        let pct: Vec<usize> = (0..0x40).filter(|&r| percent_of_hp_row(r, &m)).collect();
        assert_eq!(pct, vec![row::GRAVITY_BREAK, row::ZANTETSUKEN]);
    }

    #[test]
    fn only_percent_rows_skip_the_falloff_at_recom() {
        let m = Moveset::kh1_defaults();
        // Damage = Re:CoM: Gravity Break and Zantetsuken only (owner, follow-ups 2).
        let skip: Vec<usize> = (0..0x40).filter(|&r| skips_falloff(false, r, &m)).collect();
        assert_eq!(skip, vec![row::GRAVITY_BREAK, row::ZANTETSUKEN]);
        // Damage = KH1: nothing changes.
        assert!((0..0x40).all(|r| !skips_falloff(true, r, &m)));
    }
}
