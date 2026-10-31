//! KH1 stats for the runtime (decisions 12, 18, 21): each Re:CoM enemy's KH1 twin record per
//! Re:CoM world, and Sora's KH1 Strength by level. Written as `kh1-stats.json` next to the motion
//! manifest. Everything is read from the player's own KH1 files; this module only knows where
//! to look (file names, script offsets, table offsets). Notes: `docs/recom-kh1-attack.md`.
//!
//! Where the numbers live (Checked (decomp + files), research notes `kh1-enemy-stats.md`):
//! - An ordinary Heartless's AI is part 5 of its own `.mdls` (`0x140290D60`, binder
//!   `0x140287E40`). At init it hands a 0x6C-byte record from its own script data to AI syscall
//!   45 (`0x1402BBDF0` → `0x1402A58E0`), chosen by the LV2 story bit (syscall 198) and, for the
//!   Shadow, by world (syscall 105). The script offsets below were found by evaluating each
//!   script for every world and both LV values.
//! - Record: r0 HP, r1 / r2 Strength / Defence index into btltbl `0xC8A8` (21 rows of
//!   {Strength, Defence}), r3 / r4 damage floor / cap (u16), r5 level, r7 EXP, r10..r25
//!   resistances (%).
//! - Sora: new-game template btltbl `0x39D8` (Strength at +3), Dive choice deltas
//!   `0x128 + 4 × choice` (keep Rod, give up Shield = choice 5), level bonuses `0x3AC0` (bonus 4 =
//!   Strength +2), Kingdom Key Strength from its weapon row (+0x30).

use crate::{Error, Result};

/// KH1 world codes (`0x1404DCCA0`) for Re:CoM world ids 1..13, in the order of Re:CoM's world name
/// table (`0x140506AA0`): Traverse Town, Agrabah, Olympus Coliseum, Wonderland, Monstro,
/// Halloween Town, Atlantica, Neverland, Hollow Bastion, 100 Acre Wood, Twilight Town, Destiny
/// Islands, Castle Oblivion. `None`: KH1 has no such world, so the LV2 record is used (owner,
/// decision 21). Re:CoM's world id is `0x1402339C0(0)`, this order + 1 (Checked (decomp + exe));
/// `0x14087B844` is the floor, which picks the enemy's `@ENP` row (see [`RECOM_ROWS`]).
pub const RECOM_WORLDS: [(&str, Option<&str>); 13] = [
    ("Traverse Town", Some("tw")),
    ("Agrabah", Some("al")),
    ("Olympus Coliseum", Some("he")),
    ("Wonderland", Some("aw")),
    ("Monstro", Some("pi")),
    ("Halloween Town", Some("nm")),
    ("Atlantica", Some("lm")),
    ("Neverland", Some("pp")),
    ("Hollow Bastion", Some("pc")),
    ("100 Acre Wood", Some("po")),
    ("Twilight Town", None),
    ("Destiny Islands", Some("di")),
    ("Castle Oblivion", None),
];

/// How a script picks its first-visit (LV1) record.
#[derive(Clone, Copy, Debug)]
pub enum Lv1 {
    /// One offset everywhere.
    Fixed(u32),
    /// By KH1 world code; worlds not listed use the fallback.
    ByWorld(&'static [(&'static str, u32)], u32),
}

/// A Re:CoM enemy class and its KH1 twin.
#[derive(Clone, Copy, Debug)]
pub struct Twin {
    /// Re:CoM's class (its RTTI name; the runtime knows each class's vtable).
    pub class: &'static str,
    /// KH1's model file (its part 5 is the AI script).
    pub model: &'static str,
    /// The LV1 record.
    pub lv1: Lv1,
    /// The LV2 record (the same as LV1 for single-record scripts).
    pub lv2: u32,
    /// The pairing comes from internal names only (Assumption).
    pub assumed: bool,
    /// Re:CoM's enemy id of the class (the factory `0x140188F40`'s case; its stats are
    /// `ENEMY/<id>/EN<id>.EPD`). Checked (decomp).
    pub recom_id: u16,
}

const SHADOW_WORLDS: &[(&str, u32)] = &[
    ("dh", 0x14),
    ("di", 0xEC),
    ("tw", 0x158),
    ("aw", 0x1C4),
    ("he", 0x230),
    ("tz", 0x29C),
    ("al", 0x3E0),
    ("pi", 0x44C),
    ("nm", 0x590),
    ("pp", 0x5FC),
    ("pc", 0x6D4),
    ("dc", 0x740),
    ("po", 0x740),
    ("lm", 0x740),
];

const fn lv(class: &'static str, model: &'static str, assumed: bool, recom_id: u16) -> Twin {
    Twin { class, model, lv1: Lv1::Fixed(0x14), lv2: 0x80, assumed, recom_id }
}
const fn one(class: &'static str, model: &'static str, assumed: bool, recom_id: u16) -> Twin {
    Twin { class, model, lv1: Lv1::Fixed(0x14), lv2: 0x14, assumed, recom_id }
}

/// Re:CoM's KH1-equivalent enemies. Re:CoM-only ones (and KH1 ones not identified: Tornado
/// Step) keep Re:CoM's numbers.
pub const TWINS: &[Twin] = &[
    Twin { class: "CEnemyShadow", model: "xa_ex_2020.mdls", lv1: Lv1::ByWorld(SHADOW_WORLDS, 0x740), lv2: 0x740, assumed: false, recom_id: 1 },
    lv("CEnemySoldier", "xa_ex_2010.mdls", false, 2),
    lv("CEnemyLargebody", "xa_ex_2050.mdls", false, 9),
    lv("CEnemyRednocturne", "xa_ex_2110.mdls", false, 3),
    lv("CEnemyBluerhapsody", "xa_ex_2120.mdls", false, 4),
    lv("CEnemyYellowopera", "xa_ex_2130.mdls", false, 5),
    lv("CEnemyGreenrequiem", "xa_ex_2140.mdls", false, 6),
    lv("CEnemyPowerwild", "xa_ex_2030.mdls", false, 7),
    lv("CEnemyBouncywild", "xa_ex_2040.mdls", false, 8),
    lv("CEnemyAirsoldier", "xa_ex_2160.mdls", false, 15),
    lv("CEnemyBandit", "xa_ex_2090.mdls", false, 11),
    lv("CEnemyFatbandit", "xa_ex_2060.mdls", false, 10),
    lv("CEnemyBarrelspider", "xa_ex_2170.mdls", true, 28),
    lv("CEnemySearchghost", "xa_ex_2230.mdls", false, 14),
    lv("CEnemyPirate", "xa_ex_2100.mdls", false, 21),
    lv("CEnemyAirpirate", "xa_ex_2210.mdls", false, 22),
    lv("CEnemyWightnight", "xa_ex_2200.mdls", false, 12),
    lv("CEnemyGargoyle", "xa_ex_2220.mdls", false, 13),
    lv("CEnemyDarkball", "xa_ex_2280.mdls", false, 23),
    lv("CEnemyDefender", "xa_ex_2340.mdls", false, 26),
    lv("CEnemyWizard", "xa_ex_2150.mdls", false, 25),
    lv("CEnemySeaneon", "xa_ex_2070.mdls", true, 18),
    lv("CEnemyAquatank", "xa_ex_2080.mdls", true, 20),
    lv("CEnemyScrewdiver", "xa_ex_2240.mdls", true, 19),
    lv("CEnemyWyvern", "xa_ex_2320.mdls", true, 24),
    one("CEnemyWhitemushroom", "xa_ex_2350.mdls", false, 16),
    one("CEnemyBlackfungus", "xa_ex_2380.mdls", false, 17),
    one("CEnemyCreapplant", "xa_ex_2290.mdls", true, 31),
    one("CEnemyNeoshadow", "xa_ex_2420.mdls", true, 27),
    one("CEnemySpadesoldier", "xa_aw_1030.mdls", false, 103),
    one("CEnemyHeartsoldier", "xa_aw_1060.mdls", false, 104),
];

/// A Re:CoM boss with a KH1 equivalent (the proxy rule's boss pool). The KH1 record is read from
/// the boss model's own AI script (`.mdls` part 5, byte-identical to the room copy), as for the
/// ordinary twins. Offsets: research notes `kh1-enemy-stats.md` §5–6, re-read from the player's
/// extracted models (Checked (files)).
#[derive(Clone, Copy, Debug)]
pub struct BossTwin {
    /// Re:CoM's enemy id (factory `0x140188F40`). Checked (decomp).
    pub recom_id: u16,
    /// Re:CoM's class (RTTI name), for the log.
    pub class: &'static str,
    /// KH1's name for the fight (for the log).
    pub name: &'static str,
    /// The Re:CoM world (1..13) where Sora fights it; that world's other bosses scale from it.
    /// 0: none (the boss only stands for itself).
    pub home_world: usize,
    /// KH1's model whose part 5 holds the records.
    pub model: &'static str,
    /// The first-visit record (decision 21).
    pub lv1: u32,
    /// The LV2 record (Twilight Town, Castle Oblivion).
    pub lv2: u32,
    /// In the pooled boss ratio (worlds without a home boss).
    pub pool: bool,
    /// The pairing is an Assumption (which KH1 fight Re:CoM's boss stands for).
    pub assumed: bool,
}

#[allow(clippy::too_many_arguments)]
const fn boss(recom_id: u16, class: &'static str, name: &'static str, home_world: usize, model: &'static str, lv1: u32, lv2: u32, pool: bool, assumed: bool) -> BossTwin {
    BossTwin { recom_id, class, name, home_world, model, lv1, lv2, pool, assumed }
}

/// Re:CoM's bosses with a KH1 equivalent. Each takes its own KH1 record (a boss twin); the
/// world's other bosses (Re:CoM-only ones, Hades, Jafar's lamp and Iago…) scale from the
/// world's home boss, or from the pool where the world has none. Hades is not paired: KH1 fights
/// him only in the Hades Cup (1800 HP, level 64), no first-visit record.
pub const BOSS_TWINS: &[BossTwin] = &[
    boss(102, "CBossGuardarmor", "Guard Armor", 1, "xa_tw_3000.mdls", 0x14, 0x14, true, false),
    boss(101, "CTutorialLeon", "Leon", 0, "xa_ex_1030.mdls", 0x14, 0x14, false, false),
    boss(108, "CBossJafar", "Genie Jafar", 2, "xa_al_3020.mdls", 0x14, 0x14, true, true),
    boss(106, "CBossCloud", "Cloud", 3, "xa_ex_1150.mdls", 0x14, 0x80, true, false),
    boss(105, "CBossTrickMaster", "Trickmaster", 4, "xa_aw_3000.mdls", 0x14, 0x14, true, false),
    boss(110, "CBossParasiteCage", "Parasite Cage", 5, "xa_pi_3000.mdls", 0x14, 0xEC, true, false),
    boss(109, "CBossBoogie", "Oogie Boogie", 6, "xa_nm_3000.mdls", 0x14, 0x80, true, false),
    boss(111, "CBossUrsula", "Ursula (giant)", 7, "xa_lm_3000.mdls", 0x14, 0x80, true, true),
    boss(112, "CBossHook", "Captain Hook", 8, "xa_pp_3000.mdls", 0x14, 0x14, true, false),
    boss(113, "CBossMDragon", "Dragon Maleficent", 9, "xa_pc_3000.mdls", 0x14, 0x14, true, false),
    boss(115, "CBossDarkSide", "Darkside", 12, "xa_di_3000.mdls", 0x80, 0x80, true, false),
];

/// KH1's final boss (Ansem, `LASTBOSS`): Re:CoM-only bosses take its Gravity and Stop
/// resistances (decision 34: Gravity 2 %, Stop 0 = immune). Checked (files).
pub const FINAL_BOSS: (&str, u32) = ("xa_ew_3020.mdls", 0x14);
/// KH1 element slots of the resistance array (`+0x78 + 4 × element`): 8 Gravity, 9 Stop.
pub const GRAVITY: usize = 8;
/// See [`GRAVITY`].
pub const STOP: usize = 9;

/// Re:CoM's per-floor rows in an `@ENP` record that Sora's floors use: the stats init
/// `0x14017FF50` reads row `floor − 1` for floors 1..13 (0xE → 0, 0xF → 2, 100 → 0). Checked
/// (decomp).
pub const RECOM_ROWS: usize = 13;

/// A Re:CoM enemy's stats file (`ENEMY/0001/EN0001.EPD`): the exe's resource lists name one per
/// enemy id (loader type 7). Checked (exe).
pub fn epd_name(recom_id: u16) -> String {
    format!("ENEMY/{recom_id:04}/EN{recom_id:04}.EPD")
}

/// The HP of each per-floor row (rows `0..RECOM_ROWS`) of the first `@ENP` record in an `.EPD`,
/// before Re:CoM's difficulty % (`0x14052B4C8`: 75 / 100 / 100 / 100).
///
/// Layout, Checked (decomp `0x14017B4B0`, `0x14017FF50`): `"@ENP"`, `+4` 1 in the file (the
/// loader marks it relocated), `+8` the row table's offset from the record, 16-byte rows with
/// HP at `+0` (`+4` attack, `+8` the prize count); `+0x48`, `+0x50`, `+0x94` are the offsets of
/// the record's other tables (they bound the row table). Assumption (strong): the first `@ENP`
/// in the file is the record Re:CoM picks by default (index 0 of the type-0x15 resources with
/// the enemy's id; others are picked by map name or the enemy's `+0x294`).
pub fn enp_hp_rows(epd: &[u8]) -> Result<Vec<i32>> {
    let at = epd.windows(4).position(|w| w == b"@ENP").ok_or_else(|| Error::Format("no @ENP record".into()))?;
    let r = &epd[at..];
    let u32_at = |o: usize| -> Result<u32> { r.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes"))).ok_or_else(|| Error::Format(format!("@ENP: short at {o:#x}"))) };
    if u32_at(4)? != 1 {
        return Err(Error::Format(format!("@ENP: version {:#x}, not 1", u32_at(4)?)));
    }
    let rows = u32_at(8)? as usize;
    let end = rows + RECOM_ROWS * 16;
    if rows < 0x98 || end > r.len() {
        return Err(Error::Format(format!("@ENP: row table at {rows:#x} (record {:#x} bytes)", r.len())));
    }
    for o in [0x48, 0x50, 0x94] {
        let t = u32_at(o)? as usize;
        if t > rows && t < end {
            return Err(Error::Format(format!("@ENP: the table at {t:#x} starts inside {RECOM_ROWS} rows from {rows:#x}")));
        }
    }
    Ok((0..RECOM_ROWS).map(|i| i32::from_le_bytes(r[rows + 16 * i..rows + 16 * i + 4].try_into().expect("4 bytes"))).collect())
}

/// One proxy cell: what a Re:CoM enemy without a KH1 twin takes in one Re:CoM world on one
/// floor row. KH1 HP = its own Re:CoM row HP × `ratio`; Defence, level, floor / cap and
/// resistances are `record`'s.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// KH1 HP ÷ Re:CoM HP of the reference (`from`).
    pub ratio: f64,
    /// The reference: a twin class or a boss twin's class.
    pub from: String,
    /// The reference's Re:CoM HP on this row.
    pub recom_hp: i32,
    /// The reference's KH1 HP in this world.
    pub kh1_hp: i32,
    /// The reference's KH1 record (its HP is not the proxy's).
    pub record: Record,
}

/// One candidate reference: its name, KH1 record for the world and Re:CoM HP on the row.
pub type Ref<'a> = (&'a str, &'a Record, i32);

/// The lower median of the candidates by ratio (ties by name): the reference a proxy copies.
/// Candidates with no Re:CoM HP on the row are skipped.
pub fn median_ref<'a>(cands: &[Ref<'a>]) -> Option<Ref<'a>> {
    let mut v: Vec<(f64, Ref<'a>)> = cands.iter().filter(|c| c.2 > 0 && c.1.hp > 0).map(|c| (c.1.hp as f64 / c.2 as f64, *c)).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1 .0.cmp(b.1 .0)));
    Some(v[(v.len() - 1) / 2].1)
}

/// Whether a twin record deals a fixed damage (floor ≥ cap, e.g. White Mushroom 10 / 10, Black
/// Fungus 2 / 2): kept out of the ordinary pool, its floor / cap would make every proxy take a
/// fixed number.
pub fn fixed_damage(r: &Record) -> bool {
    r.cap != 0 && r.floor >= r.cap
}

/// The ordinary proxy cell: the lower median, by KH1 HP ÷ Re:CoM HP, of the ordinary twins
/// (all of them: Re:CoM's per-world rosters are not read; fixed-damage records left out).
/// Resistances 100 % (KH1's ordinary Heartless).
pub fn ordinary_cell(twins: &[Ref]) -> Option<Cell> {
    let pool: Vec<Ref> = twins.iter().copied().filter(|t| !fixed_damage(t.1)).collect();
    let (name, rec, rhp) = median_ref(&pool)?;
    let mut record = rec.clone();
    record.resist = [100; 16];
    Some(Cell { ratio: rec.hp as f64 / rhp as f64, from: name.to_string(), recom_hp: rhp, kh1_hp: rec.hp, record })
}

/// The boss proxy cell for a world: the world's home boss twin when it has one (with Re:CoM HP
/// on the row), else the lower median of the pooled boss twins. Its record, with KH1's final
/// boss's Gravity and Stop (decision 34; the cell is only used for bosses without a twin).
pub fn boss_cell(home: Option<Ref>, pool: &[Ref], final_boss: &Record) -> Option<Cell> {
    let (name, rec, rhp) = home.filter(|h| h.2 > 0 && h.1.hp > 0).or_else(|| median_ref(pool))?;
    let mut record = rec.clone();
    record.resist[GRAVITY] = final_boss.resist[GRAVITY];
    record.resist[STOP] = final_boss.resist[STOP];
    Some(Cell { ratio: rec.hp as f64 / rhp as f64, from: name.to_string(), recom_hp: rhp, kh1_hp: rec.hp, record })
}

/// The KH1 record offset of a boss twin for a Re:CoM world (decision 21: LV2 where KH1 lacks
/// the world).
pub fn boss_offset(b: &BossTwin, recom_world: usize) -> u32 {
    match RECOM_WORLDS.get(recom_world.wrapping_sub(1)).and_then(|w| w.1) {
        Some(_) => b.lv1,
        None => b.lv2,
    }
}

/// One record, resolved (Strength / Defence through btltbl's index table).
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// r0.
    pub hp: i32,
    /// btltbl `0xC8A8[r1].0`.
    pub strength: i32,
    /// btltbl `0xC8A8[r2].1`.
    pub defence: i32,
    /// r3, r4 (u16).
    pub floor: u16,
    /// Damage cap (0 = none).
    pub cap: u16,
    /// r5.
    pub level: i32,
    /// r7.
    pub exp: i32,
    /// r10..r25, %.
    pub resist: [i32; 16],
}

/// Part `k` of a `.mdls` (an i32 count, then count + 1 offsets).
pub fn mdls_part(d: &[u8], k: usize) -> Option<&[u8]> {
    let n = i32::from_le_bytes(d.get(0..4)?.try_into().ok()?) as usize;
    if !(k + 1..=64).contains(&n) {
        return None;
    }
    let off = |i: usize| -> Option<usize> { Some(i32::from_le_bytes(d.get(4 + 4 * i..8 + 4 * i)?.try_into().ok()?) as usize) };
    let (a, b) = (off(k)?, off(k + 1)?);
    d.get(a..b)
}

/// The radii (KH1 units) of the model's kind-0 lock points, in file order: what KH1's ground
/// picker subtracts from the distance to each point (`0x1402A7320` → `0x1402B0110` iterates the
/// kind-0 points, `0x1402B04C0` returns point `+0xC` × actor `+0x40`). They are the collision
/// block's (`.mdls` part 4, `CLS\x03`, counts at `+4..+7`, 0x14-byte entries from `+0x20`) last
/// group; an entry's kind is byte 0 & 0xF. Checked (decomp + files).
pub fn lock_radii(mdls: &[u8]) -> Result<Vec<f32>> {
    let bad = |w: &str| Error::Format(format!("collision block: {w}"));
    let c = mdls_part(mdls, 4).ok_or_else(|| bad("no part 4"))?;
    if c.get(0..4) != Some(b"CLS\x03".as_slice()) {
        return Err(bad("not CLS\\x03"));
    }
    let n = |i: usize| c[4 + i] as usize;
    let first = 0x20 + (n(0) + n(1) + n(2)) * 0x14;
    let mut out = Vec::new();
    for i in 0..n(3) {
        let e = c.get(first + i * 0x14..first + (i + 1) * 0x14).ok_or_else(|| bad("lock points past the end"))?;
        if e[0] & 0xF == 0 {
            out.push(f32::from_le_bytes(e[0xC..0x10].try_into().expect("4 bytes")));
        }
    }
    if out.is_empty() {
        return Err(bad("no kind-0 lock point"));
    }
    Ok(out)
}

/// The Keyblades KH1 has, by Re:CoM's Keyblade card kind (1 Kingdom Key … 0x18 Bond of Flame, in
/// the order of the names at `0x1405060F0`; kind 0 is "no card"; per-kind table `0x1405256D0`,
/// Checked (exe + decomp)), with their KH1 item ids. Matched by name; the item order and each row's Strength /
/// Magic agree with KH1 FM's Keyblade list (e.g. Ultima Weapon +14 / +2, Oathkeeper +9 / +1,
/// One-Winged Angel +8 / −2). Checked (files); the names themselves are an Assumption (KH1's
/// item-name file not read). Decision 25: these 17 use KH1's rows and models; the six Re:CoM
/// has beyond them (0x13..=0x18) come from KH2 (`kh2_keyblades`, [`Kh2Row`]).
pub const KH1_KEYBLADES: &[(u8, &str, u16)] = &[
    (1, "Kingdom Key", 0x51),
    (2, "Three Wishes", 0x57),
    (3, "Crabclaw", 0x5A),
    (4, "Pumpkinhead", 0x59),
    (5, "Fairy Harp", 0x58),
    (6, "Wishing Star", 0x63),
    (7, "Spellbinder", 0x5C),
    (8, "Metal Chocobo", 0x5F),
    (9, "Olympia", 0x5D),
    (10, "Lionheart", 0x5E),
    (11, "Lady Luck", 0x62),
    (12, "Divine Rose", 0x5B),
    (13, "Oathkeeper", 0x60),
    (14, "Oblivion", 0x61),
    (15, "Ultima Weapon", 0x64),
    (16, "Diamond Dust", 0x65),
    (17, "One-Winged Angel", 0x66),
];

/// A KH2 Keyblade's row (decision 25): its KH2 numbers and its blade's length against KH2's
/// Kingdom Key's (`kh2_keyblades`).
#[derive(Clone, Debug, PartialEq)]
pub struct Kh2Row {
    /// Item, stat row, Strength, Magic and model, from the player's KH2 files.
    pub keyblade: crate::kh2_keyblades::Kh2Keyblade,
    /// Its blade length and KH2's Kingdom Key's (metres, `kh2_keyblades::blade_length`).
    pub blade_length: f32,
    /// See `blade_length`.
    pub kk_blade_length: f32,
}

fn model_fields(models: &[crate::keyblade::KeybladeModel], card: u8) -> String {
    match models.iter().find(|m| m.card == card) {
        Some(m) => format!(", \"group\": {}, \"textures\": [{}]", m.group, m.textures.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(", ")),
        None => String::new(),
    }
}

/// The `keyblades` array: each KH1 Keyblade's weapon row (`btltbl` weapon table, 0x58-byte rows;
/// `kh1-formats` `btltbl::Weapon`), keyed by Re:CoM's card kind, then the KH2 Keyblades of `kh2`
/// (`source` `kh2`): KH2's Strength and Magic, `blade_far` scaled from the Kingdom Key's KH1
/// row by the blade lengths (`kh2_keyblades::blade_far`), and the Kingdom Key's KH1 radius,
/// `blade_near`, fraction scale, critical rate / bonus and priority. A Keyblade with a model in
/// `models` also gets `group` (its Re:CoM PLAYER resource group) and `textures` (its texture
/// names; `docs/recom-keyblade-models.md`).
pub fn keyblades_json(t: &kh1_formats::btltbl::Btltbl, models: &[crate::keyblade::KeybladeModel], kh2: &[Kh2Row]) -> Result<String> {
    let mut rows = Vec::new();
    for (idx, name, item) in KH1_KEYBLADES {
        let w = t.weapon_for_item(*item).map_err(|e| Error::Format(format!("{name}: {e}")))?;
        let model = model_fields(models, *idx);
        rows.push(format!(
            "    {{\"card\": {idx}, \"name\": \"{name}\", \"source\": \"kh1\", \"kh1_item\": {item}, \"model\": \"{}\", \"radius\": {}, \"blade_far\": {}, \"blade_near\": {}, \"frac_scale\": {}, \"strength\": {}, \"crit\": {}, \"crit_bonus\": {}, \"priority\": {}, \"magic\": {}{model}}}",
            w.model, w.radius, w.blade_far, w.blade_near, w.frac_scale, w.strength, w.crit, w.crit_bonus, w.priority, w.magic
        ));
    }
    if !kh2.is_empty() {
        let kk = t.weapon_for_item(KH1_KEYBLADES[0].2).map_err(|e| Error::Format(format!("Kingdom Key: {e}")))?;
        for r in kh2 {
            let k = &r.keyblade;
            if !(0x13..=0x18).contains(&k.card) {
                return Err(Error::Format(format!("{}: card {:#x} is not one of the KH2 Keyblades", k.name, k.card)));
            }
            let far = crate::kh2_keyblades::blade_far(kk.blade_far, r.blade_length, r.kk_blade_length);
            let model = model_fields(models, k.card);
            rows.push(format!(
                "    {{\"card\": {}, \"name\": \"{}\", \"source\": \"kh2\", \"kh2_item\": {}, \"kh2_stat\": {}, \"model\": \"{}\", \"blade_length\": {}, \"kh2_kingdom_key_blade_length\": {}, \"radius\": {}, \"blade_far\": {far}, \"blade_near\": {}, \"frac_scale\": {}, \"strength\": {}, \"crit\": {}, \"crit_bonus\": {}, \"priority\": {}, \"magic\": {}{model}}}",
                k.card, k.name, k.item, k.stat, k.model, r.blade_length, r.kk_blade_length, kk.radius, kk.blade_near, kk.frac_scale, k.strength, kk.crit, kk.crit_bonus, kk.priority, k.magic
            ));
        }
    }
    Ok(format!("  \"keyblades\": [\n{}\n  ],\n", rows.join(",\n")))
}

/// Reads one record from a script.
pub fn record(script: &[u8], at: u32, btltbl: &[u8]) -> Result<Record> {
    let at = at as usize;
    let r = |i: usize| -> Option<i32> { Some(i32::from_le_bytes(script.get(at + 4 * i..at + 4 * i + 4)?.try_into().ok()?)) };
    let bad = || Error::Format(format!("no stats record at script offset {at:#x}"));
    let idx = |i: usize| -> Result<(u8, u8)> {
        let v = r(i).ok_or_else(bad)?;
        if !(0..=20).contains(&v) {
            return Err(Error::Format(format!("record at {at:#x}: level index {v} outside 0..20")));
        }
        let o = 0xC8A8 + 2 * v as usize;
        Ok((*btltbl.get(o).ok_or_else(bad)?, *btltbl.get(o + 1).ok_or_else(bad)?))
    };
    let hp = r(0).ok_or_else(bad)?;
    if hp <= 0 {
        return Err(Error::Format(format!("record at {at:#x}: HP {hp}")));
    }
    let mut resist = [0; 16];
    for (k, x) in resist.iter_mut().enumerate() {
        *x = r(10 + k).ok_or_else(bad)?;
    }
    Ok(Record {
        hp,
        strength: idx(1)?.0 as i32,
        defence: idx(2)?.1 as i32,
        floor: (r(3).ok_or_else(bad)? & 0xFFFF) as u16,
        cap: (r(4).ok_or_else(bad)? & 0xFFFF) as u16,
        level: r(5).ok_or_else(bad)?,
        exp: r(7).ok_or_else(bad)?,
        resist,
    })
}

/// The record offset for a Re:CoM world (1-based index into [`RECOM_WORLDS`]).
pub fn offset_for(t: &Twin, recom_world: usize) -> u32 {
    match RECOM_WORLDS.get(recom_world.wrapping_sub(1)).and_then(|w| w.1) {
        None => t.lv2,
        Some(code) => match t.lv1 {
            Lv1::Fixed(o) => o,
            Lv1::ByWorld(list, fallback) => list.iter().find(|(c, _)| *c == code).map_or(fallback, |x| x.1),
        },
    }
}

/// Sora's KH1 Strength for levels 1..=100 (index 0 = level 1), keep Rod / give up Shield
/// (decision 18): template + choice delta + Kingdom Key + 2 per bonus-4 level.
pub fn sora_strength(btltbl: &[u8], keyblade_strength: i32) -> Result<Vec<i32>> {
    let b = |o: usize| btltbl.get(o).copied().ok_or_else(|| Error::Format(format!("btltbl too short for {o:#x}")));
    const TEMPLATE: usize = 0x39D8; // Sora's row: HP, MP, AP, Strength, Defence, …
    const CHOICE: usize = 0x128 + 4 * 5; // keep Rod, give up Shield: (Str, Def, MP, AP) deltas
    const BONUS: usize = 0x3AC0; // bonus for reaching level L at [L − 2]
    let mut s = b(TEMPLATE + 3)? as i32 + b(CHOICE)? as i8 as i32 + keyblade_strength;
    let mut out = vec![s];
    for l in 2..=100usize {
        if b(BONUS + l - 2)? == 4 {
            s += 2;
        }
        out.push(s);
    }
    Ok(out)
}

fn rec_json(r: &Record) -> String {
    let res: Vec<String> = r.resist.iter().map(|x| x.to_string()).collect();
    format!(
        "{{\"hp\": {}, \"strength\": {}, \"defence\": {}, \"floor\": {}, \"cap\": {}, \"level\": {}, \"exp\": {}, \"resist\": [{}]}}",
        r.hp,
        r.strength,
        r.defence,
        r.floor,
        r.cap,
        r.level,
        r.exp,
        res.join(", ")
    )
}

/// What [`build_json`] made: the file, the number of twins, and notes for the console (the
/// Re:CoM HP it read, the proxy ratios, what was missing).
#[derive(Clone, Debug)]
pub struct StatsJson {
    /// `kh1-stats.json`.
    pub json: String,
    /// Ordinary twins written.
    pub twins: usize,
    /// Boss twins written.
    pub bosses: usize,
    /// Lines for the console.
    pub notes: Vec<String>,
}

fn cell_json(c: &Option<Cell>) -> String {
    match c {
        None => "null".into(),
        Some(c) => format!("{{\"ratio\": {:.6}, \"from\": \"{}\", \"recom_hp\": {}, \"kh1_hp\": {}, \"record\": {}}}", c.ratio, c.from, c.recom_hp, c.kh1_hp, rec_json(&c.record)),
    }
}

fn ints_json(v: &[i32]) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")
}

/// Re:CoM HP rows of an enemy id, or empty (with a note) when its `.EPD` is missing or unread.
fn recom_rows(recom: &mut dyn FnMut(&str) -> Option<Vec<u8>>, id: u16, who: &str, notes: &mut Vec<String>) -> Vec<i32> {
    let name = epd_name(id);
    match recom(&name) {
        None => {
            notes.push(format!("note: {name} ({who}) not read: no proxy ratio from it"));
            Vec::new()
        }
        Some(b) => match enp_hp_rows(&b) {
            Ok(r) => r,
            Err(e) => {
                notes.push(format!("note: {name} ({who}): {e}: no proxy ratio from it"));
                Vec::new()
            }
        },
    }
}

/// The proxy tables (13 worlds × 13 floor rows) for ordinary enemies and bosses without a twin.
/// `twins`: (class, KH1 record per world, Re:CoM HP per row); `bosses`: the boss twins with the
/// same; `final_boss`: KH1's final boss record (decision 34).
pub fn proxy_tables(twins: &[(&str, Vec<Record>, Vec<i32>)], bosses: &[(BossTwin, Vec<Record>, Vec<i32>)], final_boss: &Record) -> (Vec<Vec<Option<Cell>>>, Vec<Vec<Option<Cell>>>) {
    let at = |v: &Vec<i32>, f: usize| v.get(f).copied().unwrap_or(0);
    let mut ordinary = Vec::new();
    let mut boss = Vec::new();
    for w in 1..=RECOM_WORLDS.len() {
        let mut orow = Vec::new();
        let mut brow = Vec::new();
        for f in 0..RECOM_ROWS {
            let refs: Vec<Ref> = twins.iter().map(|(c, recs, hp)| (*c, &recs[w - 1], at(hp, f))).collect();
            orow.push(ordinary_cell(&refs));
            let home = bosses.iter().find(|(b, _, _)| b.home_world == w).map(|(b, recs, hp)| (b.class, &recs[w - 1], at(hp, f)));
            let pool: Vec<Ref> = bosses.iter().filter(|(b, _, _)| b.pool).map(|(b, recs, hp)| (b.class, &recs[w - 1], at(hp, f))).collect();
            brow.push(boss_cell(home, &pool, final_boss));
        }
        ordinary.push(orow);
        boss.push(brow);
    }
    (ordinary, boss)
}

/// Builds `kh1-stats.json`. `read` gives a KH1 file by name; `recom` a Re:CoM file by archive
/// path (None when missing or unreadable: the enemy then gives no proxy ratio); `models` the
/// converted Keyblade models and `kh2` the KH2 Keyblades ([`keyblades_json`]).
///
/// Format 5 adds, for the proxies of Re:CoM enemies without a KH1 twin (`docs/recom-kh1-attack.md`
/// § KH1 numbers for enemies without a twin): each twin's `recom_id` / `recom_hp` (its `@ENP`
/// HP per floor row), `bosses` (Re:CoM's bosses with a KH1 equivalent: their KH1 records and
/// Re:CoM HP), `final_boss`, and `proxies` (`ordinary` / `boss`: 13 worlds × 13 rows of cells).
pub fn build_json(read: &mut dyn FnMut(&str) -> Result<Vec<u8>>, recom: &mut dyn FnMut(&str) -> Option<Vec<u8>>, btltbl: &[u8], keyblade_strength: i32, models: &[crate::keyblade::KeybladeModel], kh2: &[Kh2Row]) -> Result<StatsJson> {
    let strength = sora_strength(btltbl, keyblade_strength)?;
    let base = sora_strength(btltbl, 0)?;
    let parsed = kh1_formats::btltbl::Btltbl::parse(btltbl.to_vec()).map_err(|e| Error::Format(format!("btltbl.bin: {e}")))?;
    let mut notes = Vec::new();
    let mut s = String::from("{\n  \"format\": \"kh1-recom-assets stats 5\",\n");
    s.push_str(&format!("  \"sora_strength\": [{}],\n", ints_json(&strength)));
    // Without the Keyblade: the runtime adds the equipped one's Strength (decision 25).
    s.push_str(&format!("  \"sora_base_strength\": [{}],\n", ints_json(&base)));
    s.push_str(&keyblades_json(&parsed, models, kh2)?);
    s.push_str(&format!(
        "  \"worlds\": [{}],\n  \"recom_rows\": {RECOM_ROWS},\n  \"enemies\": [\n",
        RECOM_WORLDS.iter().map(|w| format!("\"{}\"", w.0)).collect::<Vec<_>>().join(", ")
    ));
    let mut n = 0;
    let mut twin_data: Vec<(&str, Vec<Record>, Vec<i32>)> = Vec::new();
    for (i, t) in TWINS.iter().enumerate() {
        let mdls = read(t.model)?;
        let script = mdls_part(&mdls, 5).ok_or_else(|| Error::Format(format!("{}: no AI script (part 5)", t.model)))?;
        let mut worlds = Vec::new();
        let mut recs = Vec::new();
        for w in 1..=RECOM_WORLDS.len() {
            let r = record(script, offset_for(t, w), btltbl).map_err(|e| Error::Format(format!("{}: {e}", t.model)))?;
            worlds.push(format!("      {}", rec_json(&r)));
            recs.push(r);
        }
        let radii = lock_radii(&mdls).map_err(|e| Error::Format(format!("{}: {e}", t.model)))?;
        let hp = recom_rows(recom, t.recom_id, t.class, &mut notes);
        s.push_str(&format!(
            "    {{\"class\": \"{}\", \"kh1_model\": \"{}\", \"assumed\": {}, \"lock_radius\": {}, \"lock_radii\": [{}], \"recom_id\": {}, \"recom_hp\": [{}], \"worlds\": [\n{}\n    ]}}{}\n",
            t.class,
            t.model,
            t.assumed,
            radii[0],
            radii.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", "),
            t.recom_id,
            ints_json(&hp),
            worlds.join(",\n"),
            if i + 1 < TWINS.len() { "," } else { "" }
        ));
        if !hp.is_empty() {
            notes.push(format!("Re:CoM HP {} (EN{:04}) floors 1..13: {:?}", t.class, t.recom_id, hp));
        }
        twin_data.push((t.class, recs, hp));
        n += 1;
    }
    s.push_str("  ],\n");
    // KH1's final boss: Gravity / Stop for the bosses without a twin (decision 34).
    let fb = {
        let mdls = read(FINAL_BOSS.0)?;
        let script = mdls_part(&mdls, 5).ok_or_else(|| Error::Format(format!("{}: no AI script (part 5)", FINAL_BOSS.0)))?;
        record(script, FINAL_BOSS.1, btltbl).map_err(|e| Error::Format(format!("{}: {e}", FINAL_BOSS.0)))?
    };
    s.push_str(&format!("  \"final_boss\": {{\"kh1_model\": \"{}\", \"gravity\": {}, \"stop\": {}}},\n", FINAL_BOSS.0, fb.resist[GRAVITY], fb.resist[STOP]));
    // The boss twins: Re:CoM's bosses with a KH1 equivalent (a missing model is a note, not a stop).
    let mut boss_data: Vec<(BossTwin, Vec<Record>, Vec<i32>)> = Vec::new();
    let mut boss_rows = Vec::new();
    for b in BOSS_TWINS {
        let mdls = match read(b.model) {
            Ok(m) => m,
            Err(e) => {
                notes.push(format!("note: {} ({}): {e}; it keeps the boss proxy rule", b.model, b.name));
                continue;
            }
        };
        let Some(script) = mdls_part(&mdls, 5) else {
            notes.push(format!("note: {}: no AI script (part 5); {} keeps the boss proxy rule", b.model, b.name));
            continue;
        };
        let mut recs = Vec::new();
        for w in 1..=RECOM_WORLDS.len() {
            recs.push(record(script, boss_offset(b, w), btltbl).map_err(|e| Error::Format(format!("{} ({}): {e}", b.model, b.name)))?);
        }
        let hp = recom_rows(recom, b.recom_id, b.class, &mut notes);
        if !hp.is_empty() {
            notes.push(format!("Re:CoM HP {} (EN{:04}) floors 1..13: {:?}; KH1 {} HP {}", b.class, b.recom_id, hp, b.name, recs[0].hp));
        }
        boss_rows.push(format!(
            "    {{\"recom_id\": {}, \"class\": \"{}\", \"name\": \"{}\", \"kh1_model\": \"{}\", \"home_world\": {}, \"pool\": {}, \"assumed\": {}, \"recom_hp\": [{}], \"worlds\": [\n{}\n    ]}}",
            b.recom_id,
            b.class,
            b.name,
            b.model,
            b.home_world,
            b.pool,
            b.assumed,
            ints_json(&hp),
            recs.iter().map(|r| format!("      {}", rec_json(r))).collect::<Vec<_>>().join(",\n")
        ));
        boss_data.push((*b, recs, hp));
    }
    s.push_str(&format!("  \"bosses\": [\n{}\n  ],\n", boss_rows.join(",\n")));
    // The proxy tables, and their ratios for the console.
    let (ordinary, boss) = proxy_tables(&twin_data, &boss_data, &fb);
    let table = |t: &Vec<Vec<Option<Cell>>>| t.iter().map(|row| format!("      [{}]", row.iter().map(cell_json).collect::<Vec<_>>().join(", "))).collect::<Vec<_>>().join(",\n");
    s.push_str(&format!("  \"proxies\": {{\n    \"ordinary\": [\n{}\n    ],\n    \"boss\": [\n{}\n    ]\n  }}\n}}\n", table(&ordinary), table(&boss)));
    for (kind, t) in [("ordinary", &ordinary), ("boss", &boss)] {
        for (w, row) in t.iter().enumerate() {
            let cells: Vec<String> = row.iter().map(|c| c.as_ref().map_or("-".into(), |c| format!("{:.3} ({})", c.ratio, c.from.trim_start_matches("CEnemy").trim_start_matches("CBoss")))).collect();
            notes.push(format!("proxy {kind} {}: floors 1..13: {}", RECOM_WORLDS[w].0, cells.join(", ")));
        }
    }
    Ok(StatsJson { json: s, twins: n, bosses: boss_data.len(), notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn btl() -> Vec<u8> {
        let mut b = vec![0u8; 0xD000];
        // Index rows: row i = (i + 2, i + 1).
        for i in 0..21 {
            b[0xC8A8 + 2 * i] = (i + 2) as u8;
            b[0xC8A8 + 2 * i + 1] = (i + 1) as u8;
        }
        b[0x39D8 + 3] = 4;
        b[0x128 + 20] = 0; // keep Rod, give up Shield: Strength +0
        b[0x3AC0] = 4; // level 2: Strength +2
        b[0x3AC0 + 1] = 5;
        b[0x3AC0 + 2] = 4; // level 4
        b
    }

    #[test]
    fn lock_radii_are_the_kind_0_points_of_part_4() {
        // Part table: 6 parts, part 4 at 0x40.
        let mut d = vec![0u8; 0x40];
        d[0..4].copy_from_slice(&6i32.to_le_bytes());
        for i in 0..=6 {
            let at = if i <= 4 { 0x40 } else { 0x40 + 0x20 + 5 * 0x14 };
            d[4 + 4 * i..8 + 4 * i].copy_from_slice(&(at as i32).to_le_bytes());
        }
        let mut c = vec![0u8; 0x20 + 5 * 0x14];
        c[0..4].copy_from_slice(b"CLS\x03");
        c[4..8].copy_from_slice(&[1, 1, 0, 3]);
        let lock = 0x20 + 2 * 0x14;
        for (i, (kind, r)) in [(0u8, 30f32), (2, 10.0), (0, 40.0)].iter().enumerate() {
            c[lock + i * 0x14] = *kind;
            c[lock + i * 0x14 + 0xC..lock + i * 0x14 + 0x10].copy_from_slice(&r.to_le_bytes());
        }
        d.extend_from_slice(&c);
        assert_eq!(lock_radii(&d).unwrap(), vec![30.0, 40.0]);
        d[0x40] = b'X';
        assert!(lock_radii(&d).is_err());
    }

    #[test]
    fn strength_by_level() {
        let s = sora_strength(&btl(), 3).unwrap();
        assert_eq!(&s[..4], &[7, 9, 9, 11]);
        assert_eq!(s.len(), 100);
    }

    #[test]
    fn shadow_record_per_world_and_lv2_for_unknown_worlds() {
        let shadow = &TWINS[0];
        assert_eq!(offset_for(shadow, 1), 0x158); // Traverse Town
        assert_eq!(offset_for(shadow, 2), 0x3E0); // Agrabah
        assert_eq!(offset_for(shadow, 10), 0x740); // 100 Acre Wood (po)
        assert_eq!(offset_for(shadow, 11), 0x740); // Twilight Town: LV2
        let soldier = &TWINS[1];
        assert_eq!(offset_for(soldier, 1), 0x14);
        assert_eq!(offset_for(soldier, 13), 0x80); // Castle Oblivion: LV2
    }

    #[test]
    fn reads_a_record_from_part_5() {
        let mut script = vec![0u8; 0x100];
        let rec: [i32; 27] = [24, 3, 3, 3, 300, 8, 0x3F80_0000, 5, 5021, 0x3F80_0000, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 0];
        for (i, v) in rec.iter().enumerate() {
            script[0x14 + 4 * i..0x18 + 4 * i].copy_from_slice(&v.to_le_bytes());
        }
        let mut mdls = Vec::new();
        mdls.extend_from_slice(&6i32.to_le_bytes());
        let base = 4 + 7 * 4;
        for i in 0..7 {
            let o: i32 = if i <= 5 { base } else { base + script.len() as i32 };
            mdls.extend_from_slice(&o.to_le_bytes());
        }
        mdls.extend_from_slice(&script);
        let p = mdls_part(&mdls, 5).unwrap();
        let r = record(p, 0x14, &btl()).unwrap();
        assert_eq!((r.hp, r.strength, r.defence, r.floor, r.cap, r.level, r.exp), (24, 5, 4, 3, 300, 8, 5));
        assert!(record(p, 0x1C, &btl()).is_err(), "a misaligned record fails the index check");
    }

    fn rec(hp: i32, defence: i32, floor: u16, cap: u16) -> Record {
        Record { hp, strength: defence, defence, floor, cap, level: 10, exp: 1, resist: [100; 16] }
    }

    /// An `.EPD` with some bytes before the record, the row table at 0xA0 and the other tables
    /// after it.
    fn epd(hp: &[i32]) -> Vec<u8> {
        let mut r = vec![0u8; 0xA0 + 16 * hp.len() + 0x40];
        r[0..4].copy_from_slice(b"@ENP");
        r[4..8].copy_from_slice(&1u32.to_le_bytes());
        r[8..12].copy_from_slice(&0xA0u32.to_le_bytes());
        let after = (0xA0 + 16 * hp.len()) as u32;
        for o in [0x48, 0x50, 0x94] {
            r[o..o + 4].copy_from_slice(&after.to_le_bytes());
        }
        for (i, h) in hp.iter().enumerate() {
            r[0xA0 + 16 * i..0xA4 + 16 * i].copy_from_slice(&h.to_le_bytes());
            r[0xA4 + 16 * i..0xA8 + 16 * i].copy_from_slice(&7i32.to_le_bytes());
        }
        let mut f = b"PADDING!".to_vec();
        f.extend_from_slice(&r);
        f
    }

    #[test]
    fn enp_rows_are_the_hp_of_the_first_record() {
        let hp: Vec<i32> = (0..13).map(|i| 33 + 3 * i).collect();
        assert_eq!(enp_hp_rows(&epd(&hp)).unwrap(), hp);
        assert_eq!(epd_name(1), "ENEMY/0001/EN0001.EPD");
        assert_eq!(epd_name(115), "ENEMY/0115/EN0115.EPD");
        // Not relocated in the file: +4 is 1.
        let mut bad = epd(&hp);
        bad[8 + 4] = 2;
        assert!(enp_hp_rows(&bad).is_err());
        // Fewer than 13 rows before the next table.
        let mut short = epd(&hp);
        short[8 + 0x50..8 + 0x54].copy_from_slice(&(0xA0u32 + 16 * 5).to_le_bytes());
        assert!(enp_hp_rows(&short).is_err());
        // A row table inside the header, or past the end.
        let mut inside = epd(&hp);
        inside[8 + 8..8 + 12].copy_from_slice(&0x40u32.to_le_bytes());
        assert!(enp_hp_rows(&inside).is_err());
        assert!(enp_hp_rows(&epd(&hp[..12])).is_err());
        assert!(enp_hp_rows(b"no record here").is_err());
    }

    #[test]
    fn the_median_reference_is_the_lower_middle_by_ratio() {
        let (a, b, c, d) = (rec(12, 8, 2, 300), rec(24, 8, 3, 300), rec(160, 10, 6, 144), rec(240, 24, 8, 80));
        // Ratios: 12/33 = 0.36, 24/55 = 0.44, 160/100 = 1.6, 240/120 = 2.0.
        let refs: Vec<Ref> = vec![("D", &d, 120), ("A", &a, 33), ("C", &c, 100), ("B", &b, 55)];
        assert_eq!(median_ref(&refs).unwrap().0, "B", "four: the lower middle");
        assert_eq!(median_ref(&refs[..3]).unwrap().0, "C", "three: the middle");
        // No Re:CoM HP on the row: skipped.
        let no_row: Vec<Ref> = vec![("A", &a, 0), ("D", &d, 120)];
        assert_eq!(median_ref(&no_row).unwrap().0, "D");
        assert!(median_ref(&[("A", &a, 0)]).is_none());
        // Ties by name, so the pick never depends on file order.
        let t: Vec<Ref> = vec![("Z", &a, 33), ("Y", &a, 33)];
        assert_eq!(median_ref(&t).unwrap().0, "Y");
    }

    #[test]
    fn the_ordinary_cell_copies_the_median_twin_with_full_resistances() {
        let shadow = rec(12, 8, 2, 300);
        let mut nocturne = rec(24, 9, 3, 300);
        nocturne.resist[2] = 50;
        let mushroom = rec(30, 8, 10, 10);
        let refs: Vec<Ref> = vec![("CEnemyShadow", &shadow, 33), ("CEnemyRednocturne", &nocturne, 40), ("CEnemyWhitemushroom", &mushroom, 1)];
        let c = ordinary_cell(&refs).unwrap();
        // The fixed-damage White Mushroom (ratio 30) is left out; of 0.36 and 0.6, the lower.
        assert_eq!(c.from, "CEnemyShadow");
        assert!((c.ratio - 12.0 / 33.0).abs() < 1e-9);
        assert_eq!((c.record.defence, c.record.floor, c.record.cap, c.recom_hp, c.kh1_hp), (8, 2, 300, 33, 12));
        assert!(fixed_damage(&mushroom) && !fixed_damage(&shadow));
        let c2 = ordinary_cell(&refs[1..]).unwrap();
        assert_eq!(c2.record.resist, [100; 16], "ordinary proxies resist nothing");
        // A twin used on its own numbers gets its own KH1 HP back.
        assert_eq!((33.0 * c.ratio).round() as i32, 12);
    }

    #[test]
    fn the_boss_cell_is_the_home_boss_else_the_pool_with_the_final_boss_gravity_and_stop() {
        let mut fb = rec(1500, 30, 4, 20);
        fb.resist[GRAVITY] = 2;
        fb.resist[STOP] = 0;
        let mut ga = rec(700, 8, 2, 30);
        ga.resist[GRAVITY] = 4;
        let tm = rec(600, 9, 2, 20);
        let hook = rec(900, 21, 2, 20);
        let pool: Vec<Ref> = vec![("CBossGuardarmor", &ga, 350), ("CBossTrickMaster", &tm, 600), ("CBossHook", &hook, 300)];
        let home = boss_cell(Some(("CBossGuardarmor", &ga, 350)), &pool, &fb).unwrap();
        assert_eq!((home.from.as_str(), home.ratio, home.record.cap), ("CBossGuardarmor", 2.0, 30));
        assert_eq!((home.record.resist[GRAVITY], home.record.resist[STOP]), (2, 0), "decision 34");
        // No home boss, or the home boss has no Re:CoM HP on the row: the pool's lower middle
        // (ratios 1.0, 2.0, 3.0 → 2.0).
        let pooled = boss_cell(None, &pool, &fb).unwrap();
        assert_eq!(pooled.from, "CBossGuardarmor");
        let pooled = boss_cell(Some(("CBossHook", &hook, 0)), &pool, &fb).unwrap();
        assert_eq!(pooled.from, "CBossGuardarmor");
        assert!(boss_cell(None, &[], &fb).is_none());
    }

    #[test]
    fn boss_twins_use_lv2_where_kh1_lacks_the_world() {
        let cloud = BOSS_TWINS.iter().find(|b| b.recom_id == 106).unwrap();
        assert_eq!(boss_offset(cloud, 3), 0x14, "Olympus Coliseum: KH1's first visit");
        assert_eq!(boss_offset(cloud, 11), 0x80, "Twilight Town: LV2");
        assert_eq!(boss_offset(cloud, 13), 0x80, "Castle Oblivion: LV2");
        // One home boss per world at most, and the twins' Re:CoM ids are the factory's.
        for w in 1..=13 {
            assert!(BOSS_TWINS.iter().filter(|b| b.home_world == w).count() <= 1);
        }
        assert_eq!(TWINS.iter().find(|t| t.class == "CEnemyShadow").unwrap().recom_id, 1);
        assert_eq!(TWINS.iter().find(|t| t.class == "CEnemyCreapplant").unwrap().recom_id, 31);
        assert_eq!(TWINS.iter().find(|t| t.class == "CEnemyHeartsoldier").unwrap().recom_id, 104);
        let mut ids: Vec<u16> = TWINS.iter().map(|t| t.recom_id).chain(BOSS_TWINS.iter().map(|b| b.recom_id)).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), TWINS.len() + BOSS_TWINS.len(), "every id once");
    }

    /// A `.mdls` with a collision block (one kind-0 lock point) and an AI script holding a
    /// record at each offset given.
    fn mdls(records: &[(u32, [i32; 27])]) -> Vec<u8> {
        let mut script = vec![0u8; 0x900];
        for (at, r) in records {
            for (i, v) in r.iter().enumerate() {
                let o = *at as usize + 4 * i;
                script[o..o + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        let mut cls = vec![0u8; 0x20 + 0x14];
        cls[0..4].copy_from_slice(b"CLS\x03");
        cls[7] = 1;
        cls[0x20 + 0xC..0x20 + 0x10].copy_from_slice(&30f32.to_le_bytes());
        let head = 4 + 7 * 4;
        let mut d = Vec::new();
        d.extend_from_slice(&6i32.to_le_bytes());
        for i in 0..7 {
            let o = match i {
                0..=4 => head,
                5 => head + cls.len(),
                _ => head + cls.len() + script.len(),
            };
            d.extend_from_slice(&(o as i32).to_le_bytes());
        }
        d.extend_from_slice(&cls);
        d.extend_from_slice(&script);
        d
    }

    #[test]
    fn proxy_tables_from_the_twins_and_boss_twins_records() {
        // Every KH1 record: HP 20 × (1 + idx), Strength / Defence index 5, floor 2, cap 300.
        let offsets = [0x14u32, 0x80, 0xEC, 0x158, 0x1C4, 0x230, 0x29C, 0x3E0, 0x44C, 0x590, 0x5FC, 0x6D4, 0x740];
        let records: Vec<(u32, [i32; 27])> = offsets
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let mut r = [100i32; 27];
                r[..10].copy_from_slice(&[20 * (1 + i as i32), 5, 5, 2, 300, 10, 0, 1, 0, 0]);
                (*o, r)
            })
            .collect();
        let model = mdls(&records);
        // Re:CoM: every enemy 33 HP on 1F, +3 a floor; Darkside's file is missing.
        let recom = |name: &str| -> Option<Vec<u8>> { (name != epd_name(115)).then(|| epd(&(0..13).map(|i| 33 + 3 * i).collect::<Vec<_>>())) };
        let mut twins = Vec::new();
        for t in TWINS {
            let script = mdls_part(&model, 5).unwrap();
            let recs: Vec<Record> = (1..=13).map(|w| record(script, offset_for(t, w), &btl()).unwrap()).collect();
            twins.push((t.class, recs, enp_hp_rows(&recom(&epd_name(t.recom_id)).unwrap()).unwrap()));
        }
        let script = mdls_part(&model, 5).unwrap();
        let bosses: Vec<(BossTwin, Vec<Record>, Vec<i32>)> = BOSS_TWINS
            .iter()
            .map(|bt| {
                let recs = (1..=13).map(|w| record(script, boss_offset(bt, w), &btl()).unwrap()).collect();
                let hp = recom(&epd_name(bt.recom_id)).map(|e| enp_hp_rows(&e).unwrap()).unwrap_or_default();
                (*bt, recs, hp)
            })
            .collect();
        let fb = record(script, FINAL_BOSS.1, &btl()).unwrap();
        let (ordinary, boss) = proxy_tables(&twins, &bosses, &fb);
        assert_eq!((ordinary.len(), ordinary[0].len(), boss.len(), boss[0].len()), (13, 13, 13, 13));
        // Traverse Town 1F: Guard Armor is home (record 0x14: HP 20, Re:CoM 33).
        let tt = boss[0][0].as_ref().unwrap();
        assert_eq!(tt.from, "CBossGuardarmor");
        assert!((tt.ratio - 20.0 / 33.0).abs() < 1e-9);
        // Destiny Islands: Darkside's file is missing, so the pool decides.
        assert_ne!(boss[11][0].as_ref().unwrap().from, "CBossDarkSide");
        // Castle Oblivion has no home boss: pooled, with the LV2 records (Cloud's 0x80: HP 40).
        assert!(boss[12][5].is_some());
        // Ordinary cells exist everywhere; Twilight Town uses the twins' LV2 records (0x80).
        assert!(ordinary.iter().all(|w| w.iter().all(|c| c.is_some())));
        assert!(ordinary[10][0].as_ref().unwrap().kh1_hp >= ordinary[0][0].as_ref().unwrap().kh1_hp);
        // The written cell is what the runtime parses (`kh1-recom` `parse_stats`).
        let c = cell_json(&ordinary[0][0]);
        assert!(c.starts_with("{\"ratio\": ") && c.contains("\"from\": \"") && c.contains("\"record\": {\"hp\": "));
        assert_eq!(cell_json(&None), "null");
    }
}
