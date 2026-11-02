//! KH1's abilities at the KH1 setting (roadmap M9; `docs/recom-m9-abilities.md`): the ability
//! state, KH1's AP and the equip screen's rules (stage 9b), and the combo modifiers (stage 9a).
//!
//! - **What they do:** the equipped abilities become the `kh1_core::Abilities` the KH1 frame
//!   passes to KH1's own handlers and pickers ([`to_core`]): High Jump, Glide, Superglide, Dodge
//!   Roll, Guard, Counterattack, Combo Plus / Air Combo Plus (copies), Combo Master, and the nine
//!   combo modifiers (`0x1402A6EE0` / `0x1402A6C70` swap a modifier's row in when its bit in
//!   `0x142D5EC10` is set; Re:CoM's attack action then plays, collides and ends the row).
//! - **Unlocks (owner, M7–M9 choices and the 9b follow-ups):** with Re:CoM's level-up sleights
//!   (Re:CoM's own learned state, `0x14023AB00(sleight)`), or at KH1's learn level on Re:CoM's level
//!   (KH1's **Rod** list, btltbl `0x3BF8`: the mod's Dive path keeps the Rod, decision 18); Gravity
//!   Break at Re:CoM level 30, Combo Master at 50 (owner); Guard and Dodge Roll from the start
//!   (owner); High Jump, Glide and Superglide when Re:CoM gives Sora its own (Re:CoM's ability
//!   variable, [`recom`]). Unlocks are never taken back.
//! - **Equipping (stage 9b):** KH1's rule: a learned ability arrives **unequipped**
//!   (`0x140284E10` writes `id | 0x80`, story grants too: event syscall `0x1401B1110` calls it);
//!   equipping one costs its AP (btltbl `0x33D8`), refused when the AP left is short
//!   (`0x140310650`); shared abilities (High Jump, Glide, Superglide: KH1's shared list `+0x599`)
//!   cost nothing and toggle freely. Dodge Roll alone arrives equipped (owner).
//! - **AP:** Sora's max AP is KH1's record `+9` on Re:CoM's level ([`max_ap`]): 3 at a new game,
//!   +2 at each level whose level-up bonus is kind 3.
//! - **The state** is a block of Re:CoM's own save (Re:CoM saves, loads and zeroes it), after the
//!   items extension ([`BLOCK_FIRST`]).

/// Where an ability unlocks from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Re:CoM's sleight of this id is learned (its level-up list `0x14010EF10`).
    Sleight(u16),
    /// Re:CoM's level reaches this level (KH1's Rod list btltbl `0x3BF8`, or the owner's level).
    Level(u32),
    /// From the start (owner): unlocked on a new or loaded save; `equip`: and equipped then.
    Start { equip: bool },
    /// Re:CoM gives Sora its own: any of these bits of Re:CoM's ability variable ([`recom`]).
    Recom(u32),
}

/// Re:CoM's own movement abilities: the bits of its save int variable (set 0, id 0x24), read with
/// Re:CoM's getter `0x1401B08C0(0, 0x24)`. Re:CoM's player setup `0x1401F0C50` copies them into the
/// player's ability word `+0x41C`, which its movement code reads. Checked (decomp + exe):
/// - bit 1 → `+0x41C & 0x200000`: its higher jump (jump init `0x1401F29F0`: row 0x14 and Sora's
///   jump speed 32.5 instead of 28.0, `0x1407C5B60 + 0x20` / `+0x1C`): **High Jump**;
/// - bit 2 → `0x400000` (Sora's story only): its glide (jump exe `0x1401EDA60` starts the glide
///   `CPlayerSora::initGlide` `0x1402202C0` on a jump press after 900 units in the air when
///   `+0x41C & 0xC00000`): **Glide**;
/// - bit 4 → `0x800000` (Sora's story only): the faster glide (glide exe `0x140217B50`: speed 8.0
///   instead of 5.0, `0x1407C5B48` / `0x1407C5B44`; row 0x1C instead of 0x1B), which also allows
///   the glide on its own: **Superglide** (and Glide).
///
/// Where Re:CoM sets them: bit 4 by its reward pick `0x1401BD9D0` in world 13 (Castle Oblivion), the
/// first time, unless byte variable (0, 0x61) is 3 (Checked (decomp)); every bit by the event
/// script machine `0x140145C50`: command 0x2C ORs bits into an int variable of set 0, command
/// 0x41 sets 0x24 to all bits (Checked (decomp)); the exe's only other writers are unreferenced
/// debug functions (`0x1400D1A00`, `0x1400D1FC0`, `0x1400D20B0`). Which events: Assumption (KH
/// wiki: High Jump after Monstro's Room of Beginnings, Glide on entering Neverland, Super Glide
/// in Castle Oblivion's Room of Rewards; Re:CoM's scripts are not in the session).
pub mod recom {
    /// Re:CoM's ability variable: int variable set 0, id 0x24.
    pub const VAR: (u32, u32) = (0, 0x24);
    pub const HIGH_JUMP: u32 = 1;
    pub const GLIDE: u32 = 2;
    pub const SUPERGLIDE: u32 = 4;
}

/// One ability of the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ability {
    pub name: &'static str,
    /// KH1's ability id (the ability list `save + 0x44`, btltbl `0x33D8` row id − 1). None: not
    /// read yet (the specials, stage 9c).
    pub kh1_id: Option<u8>,
    /// KH1's bit in `0x142D5EC10` (`0x1402A6380`), 0 for the abilities KH1 counts elsewhere.
    pub kh1_bit: u32,
    /// KH1's AP cost (btltbl `0x33D8 + 12 × (id − 1)`, byte +0, read by `0x1402E8120`).
    pub ap: u8,
    /// KH1's sort key (the same record's u16 `+2`, `0x1402E81B0`): the equip screen's order.
    pub key: u16,
    /// KH1's shared list (`save + 0x599`, counted with list 0xB by `0x1402A6380`): no AP.
    pub shared: bool,
    pub source: Source,
}

/// Our index of each ability: its bit in the block's `unlocked` / `equipped` words. Never reorder.
pub mod ix {
    pub const SLIDING_DASH: usize = 0;
    pub const BLITZ: usize = 1;
    pub const STUN_IMPACT: usize = 2;
    pub const ZANTETSUKEN: usize = 3;
    pub const VORTEX: usize = 4;
    pub const AERIAL_SWEEP: usize = 5;
    pub const HURRICANE_BLAST: usize = 6;
    pub const RIPPLE_DRIVE: usize = 7;
    pub const GRAVITY_BREAK: usize = 8;
    pub const STRIKE_RAID: usize = 9;
    pub const SONIC_BLADE: usize = 10;
    pub const ARS_ARCANUM: usize = 11;
    pub const RAGNAROK: usize = 12;
    // Stage 9b.
    pub const GUARD: usize = 13;
    pub const COUNTERATTACK: usize = 14;
    pub const DODGE_ROLL: usize = 15;
    pub const HIGH_JUMP: usize = 16;
    pub const GLIDE: usize = 17;
    pub const SUPERGLIDE: usize = 18;
    pub const COMBO_MASTER: usize = 19;
    /// Three copies (KH1's Rod list gives three).
    pub const COMBO_PLUS: [usize; 3] = [20, 21, 22];
    /// Two copies (KH1's Rod list gives two).
    pub const AIR_COMBO_PLUS: [usize; 2] = [23, 24];
}

/// Re:CoM's sleight ids (its `initStockSkill` `0x140220DF0` switch; level-up list `0x14010EF10`;
/// names in `SY0002.CTD`'s order, Fira = 0xD). Checked (decomp + files).
pub mod sleight {
    pub const SLIDING_DASH: u16 = 4;
    pub const BLITZ: u16 = 5;
    pub const STUN_IMPACT: u16 = 6;
    pub const ZANTETSUKEN: u16 = 7;
    pub const STRIKE_RAID: u16 = 8;
    pub const SONIC_BLADE: u16 = 9;
    pub const ARS_ARCANUM: u16 = 10;
    pub const RAGNAROK: u16 = 11;
}

const fn a(name: &'static str, id: u8, kh1_bit: u32, ap: u8, key: u16, source: Source) -> Ability {
    Ability { name, kh1_id: Some(id), kh1_bit, ap, key, shared: false, source }
}

const fn shared(name: &'static str, id: u8, kh1_bit: u32, key: u16, bits: u32) -> Ability {
    Ability { name, kh1_id: Some(id), kh1_bit, ap: 0, key, shared: true, source: Source::Recom(bits) }
}

const fn special(name: &'static str, s: u16) -> Ability {
    Ability { name, kh1_id: None, kh1_bit: 0, ap: 0, key: 0, shared: false, source: Source::Sleight(s) }
}

/// Every ability of the state, by [`ix`]. Checked (decomp + files): KH1 ids and bits
/// (`0x1402A6380`, `0x1402A5130`: Combo Plus 6, Air Combo Plus 7; `0x1402A7510`: Combo Master
/// 0x41), AP costs and sort keys (btltbl `0x33D8`, row id − 1 as `0x14028F8F0` indexes it), learn
/// levels (Rod list `0x3BF8`, entry i = level i + 2, `0x1402B51B0`). Names: English (KH1's own
/// ability names are its sysmsg, not in the asset step; the Archipelago list names the ids:
/// Assumption, strong).
/// Combo Master (0x41) has no row in btltbl's table (row 65 is zero: cost 0, key 0; Checked
/// (files)); KH1 FM gives it by no level list.
pub const ALL: [Ability; 25] = [
    a("Sliding Dash", 54, 0x100_0000, 1, 205, Source::Sleight(sleight::SLIDING_DASH)),
    a("Blitz", 20, 0x8, 2, 208, Source::Sleight(sleight::BLITZ)),
    a("Stun Impact", 57, 0x800_0000, 2, 211, Source::Sleight(sleight::STUN_IMPACT)),
    a("Zantetsuken", 59, 0x2000_0000, 2, 213, Source::Sleight(sleight::ZANTETSUKEN)),
    a("Vortex", 17, 0x1, 1, 206, Source::Level(84)),
    a("Aerial Sweep", 18, 0x2, 2, 207, Source::Level(36)),
    a("Hurricane Blast", 55, 0x200_0000, 2, 209, Source::Level(18)),
    a("Ripple Drive", 56, 0x400_0000, 3, 210, Source::Level(12)),
    // KH1 learns Gravity Break from no level list; the owner chose Re:CoM level 30 (2026-11-01).
    a("Gravity Break", 58, 0x1000_0000, 2, 212, Source::Level(30)),
    // The specials (stage 9c): their state is kept; not on the equip screen yet.
    special("Strike Raid", sleight::STRIKE_RAID),
    special("Sonic Blade", sleight::SONIC_BLADE),
    special("Ars Arcanum", sleight::ARS_ARCANUM),
    special("Ragnarok", sleight::RAGNAROK),
    // Guard from the start (owner, 2026-11-01; KH1's Rod list has it at 33); Counterattack at the
    // Rod list's 57.
    a("Guard", 21, 0x10, 2, 101, Source::Start { equip: false }),
    a("Counterattack", 19, 0x4, 2, 102, Source::Level(57)),
    // KH1 gives these by story (no level list holds them). Dodge Roll: equipped from the start
    // (owner); High Jump, Glide, Superglide: where Re:CoM gives Sora its own (owner); Combo
    // Master: Re:CoM level 50 (owner).
    a("Dodge Roll", 22, 0x20, 1, 100, Source::Start { equip: true }),
    shared("High Jump", 1, 0x200, 1, recom::HIGH_JUMP),
    shared("Glide", 3, 0x80, 3, recom::GLIDE | recom::SUPERGLIDE),
    shared("Superglide", 4, 0x100, 4, recom::SUPERGLIDE),
    a("Combo Master", 0x41, 0, 0, 0, Source::Level(50)),
    // The Rod list's Combo Plus (87, 93, 99) and Air Combo Plus (24, 45).
    a("Combo Plus", 6, 0, 1, 201, Source::Level(87)),
    a("Combo Plus", 6, 0, 1, 201, Source::Level(93)),
    a("Combo Plus", 6, 0, 1, 201, Source::Level(99)),
    a("Air Combo Plus", 7, 0, 1, 202, Source::Level(24)),
    a("Air Combo Plus", 7, 0, 1, 202, Source::Level(45)),
];

/// The combo modifiers (stage 9a), by [`ix`].
pub const MODIFIERS: std::ops::Range<usize> = ix::SLIDING_DASH..ix::GRAVITY_BREAK + 1;
/// The specials (stage 9c): kept, not on the screen, nothing reads them.
pub const SPECIALS: std::ops::Range<usize> = ix::STRIKE_RAID..ix::RAGNAROK + 1;
/// Every bit of [`ALL`].
pub const ALL_BITS: u32 = (1 << ALL.len()) - 1;
/// The modifiers' bits.
pub const MODIFIER_BITS: u32 = (1 << MODIFIERS.end) - 1;
const SPECIAL_BITS: u32 = ((1 << SPECIALS.end) - 1) & !MODIFIER_BITS;

// ---- AP ----

/// Sora's AP at a new game: the new-game record btltbl `0x39D8` byte 2 (→ record `+9`,
/// `0x1402852C0`), plus the Dive's AP delta for keep Rod / give up Shield (choice 5's fourth byte,
/// btltbl `0x128 + 4 × 5 + 3` = 0; `0x140284B70`). Checked (decomp + files).
pub const START_AP: i32 = 3;
/// The levels whose level-up bonus is kind 3 (record `+9` += 2, at most 100: `0x1402B5660`) in
/// Sora's bonus list btltbl `0x3AC0` (byte `[L − 2]` for reaching level L). Checked (files +
/// decomp). That `+9` is AP: the AP Up item (0x9A) raises it, capped at 100 (`0x1402CD780`; its
/// use test `0x1402CD3A0`), the battle stats copy it to `+0xE4` (`0x1402A5130`), the Dive moves it
/// by the choice's fourth byte (`0x140284B70`), and the new game fills it from the template's byte
/// 2 (`0x1402852C0`). Checked (decomp).
pub const AP_UP_LEVELS: [u32; 24] = [11, 17, 20, 23, 29, 32, 35, 41, 44, 47, 53, 56, 59, 65, 68, 71, 77, 80, 83, 86, 89, 92, 95, 98];
/// KH1's cap of the record's AP (`0x1402B5660` case 3).
pub const AP_CAP: i32 = 100;

/// Sora's max AP at a Re:CoM level (decision: KH1's AP +2 at KH1's levels, on Re:CoM's level).
pub fn max_ap(level: u32) -> i32 {
    AP_UP_LEVELS.iter().filter(|&&l| l <= level).fold(START_AP, |ap, _| (ap + 2).min(AP_CAP))
}

/// The AP the equipped abilities use: KH1's `0x1402E8660` sums the costs of Sora's equipped list
/// (the shared list is not counted).
pub fn used_ap(equipped: u32) -> i32 {
    ALL.iter().enumerate().filter(|(i, a)| equipped & (1 << i) != 0 && !a.shared).map(|(_, a)| a.ap as i32).sum()
}

/// The AP left: KH1's `0x1402E8660` (max − used, never below 0).
pub fn ap_left(equipped: u32, level: u32) -> i32 {
    (max_ap(level) - used_ap(equipped)).max(0)
}

/// KH1's rule for equipping `i` (`0x140311C20` greys it, `0x140310650` refuses it): a shared
/// ability always; else its cost must not exceed the AP left.
pub fn can_equip(equipped: u32, i: usize, level: u32) -> bool {
    ALL.get(i).is_some_and(|a| a.shared || a.ap as i32 <= ap_left(equipped, level))
}

/// Unequips from the most expensive down (ties: the last in [`ALL`]) until the used AP fits.
/// Returns the bits taken off.
pub fn fit_ap(equipped: &mut u32, level: u32) -> u32 {
    let mut off = 0;
    while used_ap(*equipped) > max_ap(level) {
        let worst = ALL.iter().enumerate().filter(|(i, a)| *equipped & (1 << i) != 0 && !a.shared && a.ap > 0).max_by_key(|(i, a)| (a.ap, *i));
        let Some((i, _)) = worst else { break };
        *equipped &= !(1 << i);
        off |= 1 << i;
    }
    off
}

// ---- the save block ----

/// Our ability block: right after the items extension, inside Re:CoM's spare tail
/// `save + 0x246B ..= 0x2817` that no Re:CoM code reads or writes (`docs/recom-keyblade-equip.md`).
pub const BLOCK_FIRST: usize = crate::items::EXT_LAST + 1;
pub const BLOCK_LEN: usize = 15;
pub const BLOCK_LAST: usize = BLOCK_FIRST + BLOCK_LEN - 1;

/// Offsets inside the block. Fields never move.
pub mod field {
    /// 0 = never written (an older save, or a new game); 1 = stage 9a (unlocked = equipped);
    /// 2 = stage 9b (KH1's equip rules; the story abilities from the ini); 3 = the 9b follow-ups
    /// (their sources from Re:CoM, the start and levels). Same fields in all.
    pub const VERSION: usize = 0;
    /// u32 LE: unlocked abilities, bit [`super::ix`].
    pub const UNLOCKED: usize = 1;
    /// u32 LE: equipped abilities, bit [`super::ix`].
    pub const EQUIPPED: usize = 5;
    /// 6 bytes kept zero (KH1's AP Up items, if they ever come: the max AP is derived from the
    /// level, nothing to store).
    pub const RESERVED: usize = 9;
}

pub const BLOCK_V1: u8 = 1;
pub const BLOCK_V2: u8 = 2;
pub const BLOCK_V3: u8 = 3;
/// The layout this build writes.
pub const BLOCK_NOW: u8 = BLOCK_V3;

const _: () = assert!(BLOCK_LAST <= crate::keyblades::SPARE_LAST && field::RESERVED + 6 == BLOCK_LEN);

/// What a block holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub unlocked: u32,
    pub equipped: u32,
}

fn u32_at(b: &[u8; BLOCK_LEN], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// The state a block holds (a block never written: nothing unlocked).
pub fn read(block: &[u8; BLOCK_LEN]) -> State {
    if block[field::VERSION] == 0 {
        return State::default();
    }
    let unlocked = u32_at(block, field::UNLOCKED) & ALL_BITS;
    State { unlocked, equipped: u32_at(block, field::EQUIPPED) & unlocked }
}

/// The block's version.
pub fn version(block: &[u8; BLOCK_LEN]) -> u8 {
    block[field::VERSION]
}

/// Writes the state as layout [`BLOCK_NOW`]; the reserved bytes are kept.
pub fn write(s: &State, block: &mut [u8; BLOCK_LEN]) {
    block[field::VERSION] = block[field::VERSION].max(BLOCK_NOW);
    block[field::UNLOCKED..field::UNLOCKED + 4].copy_from_slice(&s.unlocked.to_le_bytes());
    block[field::EQUIPPED..field::EQUIPPED + 4].copy_from_slice(&s.equipped.to_le_bytes());
}

// ---- unlocks ----

/// Whether an ability's source is met: Re:CoM's sleight learned (`learned`, Re:CoM's own
/// `0x14023AB00`), Re:CoM's level, the start, or Re:CoM's ability variable (`recom_var`, [`recom`]).
pub fn source_met(a: &Ability, level: u32, recom_var: u32, learned: &mut dyn FnMut(u16) -> bool) -> bool {
    match a.source {
        Source::Sleight(id) => learned(id),
        Source::Level(l) => l <= level,
        Source::Start { .. } => true,
        Source::Recom(bits) => recom_var & bits != 0,
    }
}

/// The abilities that arrive equipped (Dodge Roll, owner: "equipped from the start").
pub fn start_equipped_bits() -> u32 {
    ALL.iter().enumerate().filter(|(_, a)| a.source == Source::Start { equip: true }).fold(0, |b, (i, _)| b | 1 << i)
}

/// The abilities a layout-v2 block had from the ini's story keys (High Jump, Glide, Superglide,
/// Combo Master; Dodge Roll's unlock stays): v2 → v3 takes them back and asks their new source.
pub const V2_STORY_BITS: u32 = 1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE | 1 << ix::COMBO_MASTER;

/// What a sync changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Synced {
    /// Newly unlocked and left unequipped (KH1's rule).
    pub new: u32,
    /// Newly unlocked and equipped (Dodge Roll, owner).
    pub new_equipped: u32,
    /// Taken back (only at the v2 → v3 migration: the ini's story abilities whose new source isn't
    /// met yet).
    pub taken: u32,
    /// Taken off to fit the AP (only at a migration).
    pub fitted: u32,
}

/// Brings the state up to Re:CoM's sleights, its level and its ability variable: every ability
/// not yet unlocked whose source is met is unlocked **unequipped** (KH1's `0x140284E10`), but
/// Dodge Roll arrives equipped when the AP allows (owner). Never taken back; only abilities not
/// yet unlocked are asked about.
pub fn sync(s: &mut State, level: u32, recom_var: u32, learned: &mut dyn FnMut(u16) -> bool) -> Synced {
    let mut out = Synced::default();
    let eq = start_equipped_bits();
    for (i, a) in ALL.iter().enumerate() {
        let bit = 1 << i;
        if s.unlocked & bit != 0 || !source_met(a, level, recom_var, learned) {
            continue;
        }
        s.unlocked |= bit;
        if eq & bit != 0 && can_equip(s.equipped, i, level) {
            s.equipped |= bit;
            out.new_equipped |= bit;
        } else {
            out.new |= bit;
        }
    }
    out
}

/// The ini's movement abilities at the migration (stage 9a's block or none → now): which of the
/// abilities that now come from the start or KH1's levels the ini had equipped (Guard,
/// Counterattack, the first `combo_plus` / `air_combo_plus` copies).
pub fn ini_equipped(a: &crate::settings::Kh1Abilities) -> u32 {
    let mut b = 0;
    if a.guard {
        b |= 1 << ix::GUARD;
    }
    if a.counterattack {
        b |= 1 << ix::COUNTERATTACK;
    }
    for (k, &i) in ix::COMBO_PLUS.iter().enumerate() {
        if (k as u8) < a.combo_plus {
            b |= 1 << i;
        }
    }
    for (k, &i) in ix::AIR_COMBO_PLUS.iter().enumerate() {
        if (k as u8) < a.air_combo_plus {
            b |= 1 << i;
        }
    }
    b
}

/// The migration of a block older than [`BLOCK_NOW`] (`from` = its version 0, 1 or 2) at its first
/// sync:
/// - v2 (9b): the ini's story abilities ([`V2_STORY_BITS`]) are taken back, to be asked again from
///   their new source; Dodge Roll keeps its state;
/// - the sync's unlocks (a newly unlocked Dodge Roll wanted equipped);
/// - everything equipped before stays equipped where it is still unlocked: the block's own set
///   (v1: 9a equipped every unlock) and, from v0 / v1, the ini's movement abilities
///   ([`ini_equipped`]; the ini is read once per save);
/// - then KH1's AP rule: over the max AP, unequipped from the most expensive down ([`fit_ap`]).
pub fn migrate(s: &mut State, from: u8, level: u32, recom_var: u32, ini: u32, learned: &mut dyn FnMut(u16) -> bool) -> Synced {
    let before = s.equipped;
    let mut taken = 0;
    if from == BLOCK_V2 {
        taken = s.unlocked & V2_STORY_BITS;
        s.unlocked &= !V2_STORY_BITS;
        s.equipped &= s.unlocked;
    }
    let mut out = sync(s, level, recom_var, learned);
    out.taken = taken & !s.unlocked;
    let ini = if from < BLOCK_V2 { ini } else { 0 };
    let wanted = (out.new | out.new_equipped) & start_equipped_bits();
    s.equipped |= (before | ini | wanted) & s.unlocked;
    out.fitted = fit_ap(&mut s.equipped, level);
    // Report what really arrived equipped after the fit.
    let fresh = out.new | out.new_equipped;
    out.new_equipped = fresh & wanted & s.equipped;
    out.new = fresh & !out.new_equipped;
    out
}

// ---- use ----

/// The equipped abilities as KH1's bits (`0x142D5EC10` layout), as the trace writes them.
pub fn kh1_bits(equipped: u32) -> u32 {
    ALL.iter().enumerate().filter(|(i, _)| equipped & (1 << i) != 0).fold(0, |b, (_, a)| b | a.kh1_bit)
}

/// The equipped combo modifiers into the abilities the KH1 pickers read (the rest untouched).
pub fn apply(equipped: u32, a: &mut kh1_core::Abilities) {
    let on = |i: usize| equipped & (1 << i) != 0;
    a.sliding_dash = on(ix::SLIDING_DASH);
    a.blitz = on(ix::BLITZ);
    a.stun_impact = on(ix::STUN_IMPACT);
    a.zantetsuken = on(ix::ZANTETSUKEN);
    a.vortex = on(ix::VORTEX);
    a.aerial_sweep = on(ix::AERIAL_SWEEP);
    a.hurricane_blast = on(ix::HURRICANE_BLAST);
    a.ripple_drive = on(ix::RIPPLE_DRIVE);
    a.gravity_break = on(ix::GRAVITY_BREAK);
}

/// Every equipped ability as the abilities the KH1 frame passes on (handlers and pickers).
pub fn to_core(equipped: u32) -> kh1_core::Abilities {
    let on = |i: usize| equipped & (1 << i) != 0;
    let count = |v: &[usize]| v.iter().filter(|&&i| on(i)).count() as u8;
    let mut a = kh1_core::Abilities {
        high_jump: on(ix::HIGH_JUMP),
        glide: on(ix::GLIDE),
        superglide: on(ix::SUPERGLIDE),
        dodge_roll: on(ix::DODGE_ROLL),
        guard: on(ix::GUARD),
        counterattack: on(ix::COUNTERATTACK),
        combo_master: on(ix::COMBO_MASTER),
        combo_plus: count(&ix::COMBO_PLUS),
        air_combo_plus: count(&ix::AIR_COMBO_PLUS),
        ..Default::default()
    };
    apply(equipped, &mut a);
    a
}

/// The names of the abilities in `bits`, for the log.
pub fn names(bits: u32) -> Vec<&'static str> {
    ALL.iter().enumerate().filter(|(i, _)| bits & (1 << i) != 0).map(|(_, a)| a.name).collect()
}

/// Whether an ability is on the equip screen (the specials wait for stage 9c).
pub fn on_screen(i: usize) -> bool {
    i < ALL.len() && SPECIAL_BITS & (1 << i) == 0
}

// ---- the equip screen (the KH1 camp entry's "Abilities" row) ----

/// KH1's ability screen (Steam, Checked (decomp)): `0x140310B60` the owner list (each party member,
/// then "Shared Abilities"), `0x140311C20` Sora's list, `0x1403120F0` the shared list (rows: icon,
/// name, cost; greyed when unequipped and the cost is over the AP left), `0x140310650` the command
/// window on a confirmed ability (Equip / Remove, Move, Sort): Equip refused with error sound 4 and
/// message 0xCA when the AP is short, else the bit 7 cleared; Remove sets it; a shared ability
/// toggles with no AP test; `0x1403108E0` the panel: AP left / max AP.
pub mod page {
    use super::{ap_left, can_equip, max_ap, on_screen, used_ap, State, ALL};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Owner {
        Sora,
        Shared,
    }

    pub const OWNERS: [Owner; 2] = [Owner::Sora, Owner::Shared];

    /// Every row of the page (owners and abilities) is Re:CoM's blue magic row: list entry 0x133
    /// through its row set-up `0x140112F10` (owner, 2026-11-01: "the Abilities page rows use
    /// Re:CoM's blue box"), as the Items page's item rows take 0x131.
    pub const ROW_LOOK: i32 = crate::keyblades::row_look::BLUE;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Label {
        Owner(Owner),
        Ability(usize),
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Row {
        pub label: Label,
        /// KH1's greyed row: unequipped, and the cost is over the AP left.
        pub grey: bool,
        /// Equipped (Re:CoM's row mark).
        pub marked: bool,
    }

    /// The owner's unlocked abilities in KH1's sort order (its Sort option 0, `0x140310260`: by
    /// the record's key; ours keeps that order fixed, ties by our index).
    pub fn list(owner: Owner, s: &State) -> Vec<usize> {
        let mut v: Vec<usize> = (0..ALL.len()).filter(|&i| on_screen(i) && s.unlocked & (1 << i) != 0 && ALL[i].shared == (owner == Owner::Shared)).collect();
        v.sort_by_key(|&i| (ALL[i].key, i));
        v
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Outcome {
        Nothing,
        /// The owner's list opened (sound 2).
        Opened,
        /// An owner with no abilities: nothing to open (sound 5).
        Empty,
        Equipped(usize),
        Removed(usize),
        /// Not enough AP (KH1's message 0xCA; error sound).
        Refused(usize),
        /// Back to the owner list (sound 4).
        Back,
        /// Close the page.
        Close,
    }

    /// The screen: the owner list, or (`owner`) that owner's abilities.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Screen {
        pub owner: Option<usize>,
        pub owner_cursor: i32,
        pub cursor: i32,
    }

    impl Screen {
        pub fn rows(&self, s: &State, level: u32) -> Vec<Row> {
            match self.owner {
                None => OWNERS.iter().map(|&o| Row { label: Label::Owner(o), grey: list(o, s).is_empty(), marked: false }).collect(),
                Some(o) => list(OWNERS[o], s)
                    .into_iter()
                    .map(|i| {
                        let on = s.equipped & (1 << i) != 0;
                        Row { label: Label::Ability(i), grey: !on && !can_equip(s.equipped, i, level), marked: on }
                    })
                    .collect(),
            }
        }

        /// The list's cursor (the owner list's or the ability list's).
        pub fn cursor(&self) -> i32 {
            if self.owner.is_some() { self.cursor } else { self.owner_cursor }
        }

        pub fn set_cursor(&mut self, c: i32) {
            if self.owner.is_some() {
                self.cursor = c;
            } else {
                self.owner_cursor = c;
            }
        }

        /// The highlighted ability.
        pub fn ability(&self, s: &State) -> Option<usize> {
            let o = self.owner?;
            list(OWNERS[o], s).get(self.cursor.max(0) as usize).copied()
        }

        /// Confirm: open an owner's list (on its first row), or toggle the highlighted ability
        /// with KH1's rule. `s` is changed in place; the caller writes it.
        pub fn confirm(&mut self, s: &mut State, level: u32) -> Outcome {
            match self.owner {
                None => {
                    let o = self.owner_cursor.clamp(0, OWNERS.len() as i32 - 1) as usize;
                    if list(OWNERS[o], s).is_empty() {
                        return Outcome::Empty;
                    }
                    self.owner = Some(o);
                    self.cursor = 0;
                    Outcome::Opened
                }
                Some(_) => {
                    let Some(i) = self.ability(s) else { return Outcome::Nothing };
                    let bit = 1 << i;
                    if s.equipped & bit != 0 {
                        s.equipped &= !bit;
                        Outcome::Removed(i)
                    } else if can_equip(s.equipped, i, level) {
                        s.equipped |= bit;
                        Outcome::Equipped(i)
                    } else {
                        Outcome::Refused(i)
                    }
                }
            }
        }

        /// Cancel: the ability list goes back to the owner list; the owner list closes the page.
        pub fn cancel(&mut self) -> Outcome {
            if self.owner.take().is_some() {
                Outcome::Back
            } else {
                Outcome::Close
            }
        }

        /// The status panel's values: AP (KH1's panel: AP left / max AP), the highlighted
        /// ability's cost, the AP used / max AP.
        pub fn status(&self, s: &State, level: u32) -> [String; 3] {
            let max = max_ap(level);
            let cost = self.ability(s).map_or_else(|| "-".to_string(), |i| ALL[i].ap.to_string());
            [format!("{}/{max}", ap_left(s.equipped, level)), cost, format!("{}/{max}", used_ap(s.equipped))]
        }
    }

    /// Texts: NUL-terminated ASCII for Re:CoM's text items (no `%`). KH1's own words (sysmsg) are
    /// not in the asset step: ours, in English (Assumption).
    pub mod text {
        pub const LABEL: &[u8] = b"Abilities\0";
        pub const HELP: &[u8] = b"Equip Sora's abilities with AP.\0";
        pub const SORA: &[u8] = b"Sora\0";
        pub const SHARED: &[u8] = b"Shared\0";
        pub const STATUS: [&[u8]; 3] = [b"AP\0", b"Cost\0", b"AP used\0"];
        pub const OWNER_HELP: [&[u8]; 2] = [b"Sora's abilities. Equipping one uses AP.\0", b"Abilities for movement. They use no AP.\0"];
        pub const EMPTY_HELP: &[u8] = b"No abilities learned yet.\0";
        /// KH1's message 0xCA (not read).
        pub const NO_AP: &[u8] = b"Not enough AP to equip this ability.\0";

        /// The ability's help line (KH1 shows its own description; ours).
        pub fn help(i: usize) -> &'static [u8] {
            use super::super::ix::*;
            match i {
                SLIDING_DASH => b"Dash at a distant locked-on enemy.\0",
                BLITZ => b"A rising strike to end a combo.\0",
                STUN_IMPACT => b"A combo finisher that stuns enemies nearby.\0",
                ZANTETSUKEN => b"A combo finisher that can fell an enemy at once.\0",
                VORTEX => b"Close in on a nearby enemy with a spinning slash.\0",
                AERIAL_SWEEP => b"Leap up and strike an enemy in the air.\0",
                HURRICANE_BLAST => b"A spinning finisher for an air combo.\0",
                RIPPLE_DRIVE => b"A combo finisher that strikes all around. Uses Magic.\0",
                GRAVITY_BREAK => b"A combo finisher that cuts an enemy's HP.\0",
                GUARD => b"Press Square to block an attack.\0",
                COUNTERATTACK => b"After a block, press Triangle to strike back.\0",
                DODGE_ROLL => b"Press Square while moving to roll.\0",
                HIGH_JUMP => b"Jump higher.\0",
                GLIDE => b"Hold the jump button in the air to glide.\0",
                SUPERGLIDE => b"Press Square in the air to glide at speed.\0",
                COMBO_MASTER => b"Keep a combo going even when a swing misses.\0",
                20..=22 => b"One more swing in a ground combo.\0",
                23..=24 => b"One more swing in an air combo.\0",
                _ => b"\0",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::page::{Label, Outcome, Owner, Screen};
    use super::*;

    #[test]
    fn block_round_trip_and_new_game() {
        let mut b = [0u8; BLOCK_LEN];
        assert_eq!(read(&b), State::default(), "a block never written: nothing unlocked");
        let s = State { unlocked: 0b1_0101, equipped: 0b101 };
        write(&s, &mut b);
        assert_eq!(b[field::VERSION], BLOCK_V3);
        assert_eq!(read(&b), s);
        // Equipped is only ever a subset of unlocked; unknown bits are dropped.
        b[field::EQUIPPED..field::EQUIPPED + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        b[field::UNLOCKED..field::UNLOCKED + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(read(&b), State { unlocked: ALL_BITS, equipped: ALL_BITS });
        assert!(b[field::RESERVED..].iter().all(|&x| x == 0));
        // A v1 block reads as before (its bits are a subset of v2's).
        let mut v1 = [0u8; BLOCK_LEN];
        v1[field::VERSION] = BLOCK_V1;
        v1[field::UNLOCKED] = 0x81;
        v1[field::EQUIPPED] = 0x81;
        assert_eq!(read(&v1), State { unlocked: 0x81, equipped: 0x81 });
    }

    #[test]
    fn layout_follows_the_items_extension() {
        assert_eq!(BLOCK_FIRST, 0x269F);
        assert_eq!(BLOCK_LAST, 0x26AD);
        assert_eq!(ALL.len(), 25);
        assert_eq!(ALL_BITS, 0x01FF_FFFF);
    }

    /// KH1's AP: 3 at a new game (keep Rod / give up Shield adds 0), +2 at each kind-3 level of the
    /// bonus list. Checked (files): the 24 levels.
    #[test]
    fn max_ap_is_kh1s_record_on_recoms_level() {
        assert_eq!(max_ap(1), 3);
        assert_eq!(max_ap(10), 3);
        assert_eq!(max_ap(11), 5);
        assert_eq!(max_ap(17), 7);
        assert_eq!(max_ap(30), 13);
        assert_eq!(max_ap(99), 51);
        assert_eq!(max_ap(100), 51);
        assert_eq!(AP_UP_LEVELS.len(), 24);
        assert!(AP_UP_LEVELS.windows(2).all(|w| w[0] < w[1]));
    }

    /// The btltbl values, read from the player's file when the test data is there.
    #[test]
    fn ap_tables_match_btltbl() {
        let Ok(dir) = std::env::var("KH1_TEST_DATA") else { return };
        let Ok(b) = std::fs::read(std::path::Path::new(&dir).join("btltbl.bin")) else { return };
        assert_eq!(b[0x39D8 + 2] as i32 + b[0x128 + 4 * 5 + 3] as i8 as i32, START_AP);
        let kind3: Vec<u32> = (2..=100u32).filter(|&l| b[0x3AC0 + l as usize - 2] == 3).collect();
        assert_eq!(kind3, AP_UP_LEVELS);
        for a in ALL.iter().filter(|a| a.kh1_id.is_some_and(|id| id < 64)) {
            let row = 0x33D8 + 12 * (a.kh1_id.unwrap() as usize - 1);
            assert_eq!((b[row], u16::from_le_bytes([b[row + 2], b[row + 3]])), (a.ap, a.key), "{}", a.name);
        }
        // Combo Master (0x41) has no row.
        assert!(b[0x33D8 + 12 * 64..0x33D8 + 12 * 65].iter().all(|&x| x == 0));
        // The Rod list's levels for every Level source but the owner's (Gravity Break 30, Combo
        // Master 50).
        let rod = |id: u8| -> Vec<u32> { (0..99).filter(|&i| b[0x3BF8 + i] == 0x80 | id).map(|i| i as u32 + 2).collect() };
        for (i, a) in ALL.iter().enumerate() {
            if let (Source::Level(l), Some(id)) = (a.source, a.kh1_id) {
                if i != ix::GRAVITY_BREAK && i != ix::COMBO_MASTER {
                    assert!(rod(id).contains(&l), "{} at {l}: {:?}", a.name, rod(id));
                }
            }
        }
        assert_eq!(rod(6), vec![87, 93, 99]);
        assert_eq!(rod(21), vec![33], "KH1's Guard level (the owner moved it to the start)");
        assert_eq!(rod(7), vec![24, 45]);
        for id in [1, 3, 4, 22, 58, 59] {
            assert!(rod(id).is_empty(), "id {id}");
        }
    }

    #[test]
    fn costs_keys_and_bits() {
        let ap: Vec<(u8, u8)> = ALL[MODIFIERS].iter().map(|a| (a.kh1_id.unwrap(), a.ap)).collect();
        assert_eq!(ap, vec![(54, 1), (20, 2), (57, 2), (59, 2), (17, 1), (18, 2), (55, 2), (56, 3), (58, 2)]);
        let x = |i: usize| (ALL[i].kh1_id.unwrap(), ALL[i].ap, ALL[i].key, ALL[i].kh1_bit, ALL[i].shared);
        assert_eq!(x(ix::GUARD), (21, 2, 101, 0x10, false));
        assert_eq!(x(ix::COUNTERATTACK), (19, 2, 102, 0x4, false));
        assert_eq!(x(ix::DODGE_ROLL), (22, 1, 100, 0x20, false));
        assert_eq!(x(ix::HIGH_JUMP), (1, 0, 1, 0x200, true));
        assert_eq!(x(ix::GLIDE), (3, 0, 3, 0x80, true));
        assert_eq!(x(ix::SUPERGLIDE), (4, 0, 4, 0x100, true));
        assert_eq!(x(ix::COMBO_MASTER), (0x41, 0, 0, 0, false));
        // The movement bits as `settings::Kh1Abilities::bits` has them.
        let all_move = 1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE | 1 << ix::GUARD | 1 << ix::DODGE_ROLL | 1 << ix::COUNTERATTACK;
        let ini = crate::settings::Kh1Abilities { high_jump: true, glide: true, superglide: true, dodge_roll: true, guard: true, counterattack: true, combo_plus: 0, air_combo_plus: 0, combo_master: false };
        assert_eq!(kh1_bits(all_move), ini.bits());
        assert_eq!(kh1_bits(MODIFIER_BITS), 0x3F00_0000 | 0x8 | 0x1 | 0x2);
        assert_eq!(kh1_bits(1 << ix::RAGNAROK), 0);
    }

    #[test]
    fn learn_levels_are_kh1s_rod_list_and_the_owners() {
        let lv = |i: usize| match ALL[i].source {
            Source::Level(l) => l,
            _ => 0,
        };
        assert_eq!([lv(ix::RIPPLE_DRIVE), lv(ix::HURRICANE_BLAST), lv(ix::AERIAL_SWEEP), lv(ix::VORTEX)], [12, 18, 36, 84]);
        assert_eq!(lv(ix::GRAVITY_BREAK), 30, "owner, 2026-11-01");
        assert_eq!(lv(ix::COMBO_MASTER), 50, "owner, 2026-11-01");
        assert_eq!(lv(ix::COUNTERATTACK), 57, "KH1's Rod list");
        assert_eq!(ALL[ix::GUARD].source, Source::Start { equip: false }, "owner: Guard from the start");
        assert_eq!(ALL[ix::DODGE_ROLL].source, Source::Start { equip: true }, "owner: Dodge Roll equipped from the start");
        assert_eq!(start_equipped_bits(), 1 << ix::DODGE_ROLL);
        assert_eq!(ix::COMBO_PLUS.map(lv), [87, 93, 99]);
        assert_eq!(ix::AIR_COMBO_PLUS.map(lv), [24, 45]);
        // Re:CoM's own movement abilities (its variable (0, 0x24)).
        assert_eq!(ALL[ix::HIGH_JUMP].source, Source::Recom(1));
        assert_eq!(ALL[ix::GLIDE].source, Source::Recom(2 | 4), "Re:CoM glides with either bit");
        assert_eq!(ALL[ix::SUPERGLIDE].source, Source::Recom(4));
    }

    const NO_RECOM: u32 = 0;
    const START: u32 = 1 << ix::GUARD | 1 << ix::DODGE_ROLL;

    #[test]
    fn new_unlocks_arrive_unequipped() {
        let mut s = State::default();
        // A new game: Guard (unequipped) and Dodge Roll (equipped) from the start.
        let r = sync(&mut s, 1, NO_RECOM, &mut |_| false);
        assert_eq!((names(r.new), names(r.new_equipped)), (vec!["Guard"], vec!["Dodge Roll"]));
        assert_eq!(s, State { unlocked: START, equipped: 1 << ix::DODGE_ROLL });
        assert_eq!(sync(&mut s, 1, NO_RECOM, &mut |_| false), Synced::default());
        // Re:CoM's Lv2 sleight and level 12: Sliding Dash, Ripple Drive, unequipped (KH1).
        let r = sync(&mut s, 12, NO_RECOM, &mut |id| id == sleight::SLIDING_DASH);
        assert_eq!(names(r.new), vec!["Sliding Dash", "Ripple Drive"]);
        assert_eq!(s.equipped, 1 << ix::DODGE_ROLL);
        // Only abilities not yet unlocked are asked about; never taken back.
        let mut asked = Vec::new();
        sync(&mut s, 12, NO_RECOM, &mut |id| {
            asked.push(id);
            false
        });
        assert!(!asked.contains(&sleight::SLIDING_DASH));
        sync(&mut s, 1, NO_RECOM, &mut |_| false);
        assert_eq!(names(s.unlocked & !START), vec!["Sliding Dash", "Ripple Drive"]);
        // Level 30: Gravity Break (owner), Hurricane Blast 18, Air Combo Plus 24.
        let r = sync(&mut s, 30, NO_RECOM, &mut |_| false);
        assert_eq!(names(r.new), vec!["Hurricane Blast", "Gravity Break", "Air Combo Plus"]);
        // Level 50: Combo Master (owner), unequipped.
        let r = sync(&mut s, 50, NO_RECOM, &mut |_| false);
        assert_eq!(names(r.new), vec!["Aerial Sweep", "Combo Master", "Air Combo Plus"]);
        // Level 99 and every sleight: everything but Re:CoM's movement abilities.
        sync(&mut s, 99, NO_RECOM, &mut |_| true);
        assert_eq!(s.unlocked, ALL_BITS & !(1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE));
        assert_eq!(s.equipped, 1 << ix::DODGE_ROLL);
    }

    #[test]
    fn movement_abilities_follow_recoms_variable() {
        let mut s = State::default();
        sync(&mut s, 1, NO_RECOM, &mut |_| false);
        // Re:CoM's High Jump (bit 1): unlocked, unequipped (KH1's rule; shared: free to equip).
        let r = sync(&mut s, 1, recom::HIGH_JUMP, &mut |_| false);
        assert_eq!(names(r.new), vec!["High Jump"]);
        assert_eq!(s.equipped & 1 << ix::HIGH_JUMP, 0);
        // Its Glide (bit 2), then the faster glide (bit 4): Superglide.
        let r = sync(&mut s, 1, recom::HIGH_JUMP | recom::GLIDE, &mut |_| false);
        assert_eq!(names(r.new), vec!["Glide"]);
        let r = sync(&mut s, 1, 7, &mut |_| false);
        assert_eq!(names(r.new), vec!["Superglide"]);
        // Bit 4 alone gives Re:CoM's glide too: Glide and Superglide.
        let mut t = State::default();
        let r = sync(&mut t, 1, recom::SUPERGLIDE, &mut |_| false);
        assert_eq!(names(r.new), vec!["Guard", "Glide", "Superglide"]);
        // Never taken back, whatever the variable says later; other bits mean nothing.
        sync(&mut s, 1, 0xFFFF_FFF8, &mut |_| false);
        assert_eq!(s.unlocked & (1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE), 1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE);
        let mut u = State::default();
        sync(&mut u, 1, 0xFFFF_FFF8, &mut |_| false);
        assert_eq!(u.unlocked, START);
    }

    #[test]
    fn migration_keeps_the_old_set_within_kh1s_ap() {
        // A 9a save at level 34: 9a equipped every unlock (Sliding Dash, Blitz, Stun Impact,
        // Zantetsuken, Hurricane Blast, Ripple Drive, Aerial Sweep = 1+2+2+2+2+3+2 = 14 AP).
        let mut s = State { unlocked: 0, equipped: 0 };
        let nine_a = [ix::SLIDING_DASH, ix::BLITZ, ix::STUN_IMPACT, ix::ZANTETSUKEN, ix::HURRICANE_BLAST, ix::RIPPLE_DRIVE, ix::AERIAL_SWEEP].iter().fold(0, |b, &i| b | 1 << i);
        s.unlocked = nine_a;
        s.equipped = nine_a;
        let ini = crate::settings::Kh1Abilities::default(); // guard, counterattack on
        let r = migrate(&mut s, BLOCK_V1, 34, NO_RECOM, ini_equipped(&ini), &mut |id| id <= sleight::ZANTETSUKEN);
        // Level 34: 15 AP. Wanted: 14 (9a) + Guard 2 + Dodge Roll 1 = 17: the most expensive go
        // first: Ripple Drive (3).
        assert_eq!(max_ap(34), 15);
        assert_eq!(names(r.fitted), vec!["Ripple Drive"]);
        assert!(used_ap(s.equipped) <= 15);
        assert!(s.equipped & 1 << ix::GUARD != 0, "the ini's Guard stays (from the start now)");
        assert!(s.equipped & 1 << ix::DODGE_ROLL != 0, "Dodge Roll arrives equipped");
        assert_eq!(names(r.new_equipped), vec!["Dodge Roll"]);
        assert!(s.equipped & 1 << ix::COUNTERATTACK == 0, "Counterattack not unlocked before 57");
        assert!(s.equipped & 1 << ix::GRAVITY_BREAK == 0, "a new unlock: unequipped");
        // A new game (block never written) with the default ini: Dodge Roll and the ini's Guard,
        // 3 AP of 3.
        let mut n = State::default();
        migrate(&mut n, 0, 1, NO_RECOM, ini_equipped(&ini), &mut |_| false);
        assert_eq!((names(n.equipped), used_ap(n.equipped)), (vec!["Guard", "Dodge Roll"], 3));
        // With the ini's guard off: Guard unlocked, unequipped (KH1's rule).
        let off = crate::settings::Kh1Abilities { guard: false, ..ini };
        let mut o = State::default();
        migrate(&mut o, 0, 1, NO_RECOM, ini_equipped(&off), &mut |_| false);
        assert_eq!((o.unlocked, names(o.equipped)), (START, vec!["Dodge Roll"]));
        // The ini's combo_plus copies at a level that has them.
        let cp = crate::settings::Kh1Abilities { combo_plus: 2, air_combo_plus: 1, ..ini };
        let mut m = State::default();
        migrate(&mut m, 0, 99, NO_RECOM, ini_equipped(&cp), &mut |_| false);
        let c = to_core(m.equipped);
        assert_eq!((c.combo_plus, c.air_combo_plus, c.guard, c.counterattack, c.dodge_roll), (2, 1, true, true, true));
    }

    #[test]
    fn v2_story_abilities_move_to_their_new_sources() {
        // A 9b save at level 40 whose ini had High Jump, Glide, Superglide, Combo Master on (all
        // equipped) and Dodge Roll removed in the camp; Re:CoM has given only its High Jump.
        let had = 1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE | 1 << ix::COMBO_MASTER | 1 << ix::GUARD;
        let mut s = State { unlocked: had | 1 << ix::DODGE_ROLL, equipped: had };
        let r = migrate(&mut s, BLOCK_V2, 40, recom::HIGH_JUMP, u32::MAX, &mut |_| false);
        assert_eq!(names(r.taken), vec!["Glide", "Superglide", "Combo Master"]);
        assert!(s.equipped & 1 << ix::HIGH_JUMP != 0, "Re:CoM has it: stays equipped");
        assert_eq!(s.unlocked & V2_STORY_BITS, 1 << ix::HIGH_JUMP);
        assert_eq!(s.equipped & 1 << ix::DODGE_ROLL, 0, "the player's removal stands");
        assert_eq!(s.equipped & 1 << ix::COUNTERATTACK, 0, "the ini is read only from v0 / v1");
        // Level 50 and Re:CoM's Glide later: back, unequipped (a later sync, not a migration).
        let r = sync(&mut s, 50, recom::HIGH_JUMP | recom::GLIDE, &mut |_| false);
        assert!(r.new & (1 << ix::GLIDE | 1 << ix::COMBO_MASTER) == 1 << ix::GLIDE | 1 << ix::COMBO_MASTER);
        // At the migration itself, a source already met keeps the old equipped state.
        let mut t = State { unlocked: had, equipped: had };
        let r = migrate(&mut t, BLOCK_V2, 50, 7, 0, &mut |_| false);
        assert_eq!(r.taken, 0);
        assert_eq!(t.equipped & V2_STORY_BITS, V2_STORY_BITS);
        assert_eq!(names(r.new_equipped), vec!["Dodge Roll"], "never unlocked before: equipped now");
    }

    #[test]
    fn fit_takes_the_most_expensive_first() {
        let mut e = 1 << ix::RIPPLE_DRIVE | 1 << ix::GUARD | 1 << ix::SLIDING_DASH | 1 << ix::HIGH_JUMP;
        assert_eq!(used_ap(e), 6);
        assert_eq!(fit_ap(&mut e, 1), 1 << ix::RIPPLE_DRIVE);
        assert_eq!(used_ap(e), 3);
        assert!(e & 1 << ix::HIGH_JUMP != 0, "shared: free");
        // Equal costs: the later index goes first.
        let mut e = 1 << ix::BLITZ | 1 << ix::GUARD;
        assert_eq!(fit_ap(&mut e, 1), 1 << ix::GUARD);
    }

    #[test]
    fn to_core_counts_copies() {
        let e = 1 << ix::COMBO_PLUS[0] | 1 << ix::COMBO_PLUS[2] | 1 << ix::AIR_COMBO_PLUS[1] | 1 << ix::GLIDE | 1 << ix::VORTEX;
        let c = to_core(e);
        assert_eq!((c.combo_plus, c.air_combo_plus, c.glide, c.vortex, c.guard), (2, 1, true, true, false));
    }

    #[test]
    fn the_screen_follows_kh1s_rules() {
        let level = 11; // 5 AP
        let mut s = State { unlocked: 1 << ix::RIPPLE_DRIVE | 1 << ix::GUARD | 1 << ix::DODGE_ROLL | 1 << ix::GLIDE | 1 << ix::STRIKE_RAID, equipped: 1 << ix::GLIDE };
        let mut sc = Screen::default();
        let rows = sc.rows(&s, level);
        assert_eq!(rows.iter().map(|r| r.label).collect::<Vec<_>>(), vec![Label::Owner(Owner::Sora), Label::Owner(Owner::Shared)]);
        assert_eq!(sc.confirm(&mut s, level), Outcome::Opened);
        // Sora's list in KH1's key order; the special (9c) isn't listed.
        let l: Vec<Label> = sc.rows(&s, level).iter().map(|r| r.label).collect();
        assert_eq!(l, vec![Label::Ability(ix::DODGE_ROLL), Label::Ability(ix::GUARD), Label::Ability(ix::RIPPLE_DRIVE)]);
        // Equip Ripple Drive (3), then Guard (2) = 5; Dodge Roll (1) is greyed and refused.
        sc.set_cursor(2);
        assert_eq!(sc.confirm(&mut s, level), Outcome::Equipped(ix::RIPPLE_DRIVE));
        sc.set_cursor(1);
        assert_eq!(sc.confirm(&mut s, level), Outcome::Equipped(ix::GUARD));
        assert_eq!(sc.status(&s, level), ["0/5".to_string(), "2".into(), "5/5".into()]);
        let rows = sc.rows(&s, level);
        assert!(rows[0].grey && !rows[0].marked && rows[1].marked && rows[2].marked);
        sc.set_cursor(0);
        assert_eq!(sc.confirm(&mut s, level), Outcome::Refused(ix::DODGE_ROLL));
        // Remove Guard: Dodge Roll fits.
        sc.set_cursor(1);
        assert_eq!(sc.confirm(&mut s, level), Outcome::Removed(ix::GUARD));
        sc.set_cursor(0);
        assert_eq!(sc.confirm(&mut s, level), Outcome::Equipped(ix::DODGE_ROLL));
        // Shared: free toggles.
        assert_eq!(sc.cancel(), Outcome::Back);
        sc.set_cursor(1);
        assert_eq!(sc.confirm(&mut s, level), Outcome::Opened);
        assert_eq!(sc.confirm(&mut s, level), Outcome::Removed(ix::GLIDE));
        assert_eq!(sc.confirm(&mut s, level), Outcome::Equipped(ix::GLIDE));
        assert_eq!(sc.cancel(), Outcome::Back);
        assert_eq!(sc.cursor(), 1, "back on the owner row");
        assert_eq!(sc.cancel(), Outcome::Close);
        // An owner with nothing.
        let mut e = State::default();
        let mut sc = Screen::default();
        assert_eq!(sc.confirm(&mut e, 1), Outcome::Empty);
        assert!(sc.rows(&e, 1)[0].grey);
    }

    /// The page's rows take Re:CoM's list entry 0x133 (the blue magic row): `seqs[ROW_LOOK]` of
    /// the checked sequence table `0x1407C11F0`.
    #[test]
    fn rows_are_recoms_blue_magic_row() {
        let seqs = crate::sites::KEYBLADE_DATA.iter().find(|s| s.rva == 0x007C_11F0).unwrap().expect;
        let at = 2 * page::ROW_LOOK as usize;
        assert_eq!(u16::from_le_bytes([seqs[at], seqs[at + 1]]), 0x133);
    }

    #[test]
    fn texts_fit() {
        let mut all: Vec<&[u8]> = vec![page::text::LABEL, page::text::HELP, page::text::SORA, page::text::SHARED, page::text::EMPTY_HELP, page::text::NO_AP];
        all.extend(page::text::STATUS);
        all.extend(page::text::OWNER_HELP);
        all.extend((0..ALL.len()).filter(|&i| on_screen(i)).map(page::text::help));
        for s in all {
            assert_eq!(s.last(), Some(&0));
            assert!(s.len() > 1);
            assert!(s[..s.len() - 1].iter().all(|&b| (0x20..0x7F).contains(&b) && b != b'%'));
            assert!(s.len() - 1 <= crate::menu::HELP_MAX, "{}", std::str::from_utf8(s).unwrap());
        }
    }
}
