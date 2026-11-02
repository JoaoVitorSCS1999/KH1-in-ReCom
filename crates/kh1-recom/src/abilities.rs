//! KH1's abilities at the KH1 setting (roadmap M9, stage 9a: the combo modifiers and the ability
//! state; `docs/recom-m9-abilities.md`).
//!
//! - **What they do:** KH1's own Attack pickers (`kh1_core::combo`, `0x1402A6EE0` / `0x1402A6C70`)
//!   swap a modifier's row in for a swing when its bit in `0x142D5EC10` is set. Here the equipped
//!   modifiers become those bits in the `kh1_core::Abilities` the KH1 frame passes to the pickers;
//!   Re:CoM's attack action then plays, collides and ends the row (`crate::attack`).
//! - **Unlocks (owner, M7–M9 choices):** with Re:CoM's level-up sleights (Re:CoM's own learned
//!   state, `0x14023AB00(sleight)` on `save + 0x2840`, two bits per sleight), or for the five with no
//!   Re:CoM source at KH1's learn level on Re:CoM's level (the Rod list, btltbl `0x3BF8`: the mod's
//!   Dive path keeps the Rod, decision 18). Unlocks are never taken back.
//! - **Equipped (until the equip screen, stage 9b):** an unlocked ability is equipped as it unlocks.
//!   KH1 itself adds a learned ability unequipped (`0x140284E10` writes `id | 0x80`); 9b brings
//!   KH1's rule with the AP screen.
//! - **The state** is a block of Re:CoM's own save (Re:CoM saves, loads and zeroes it), after the
//!   items extension ([`BLOCK_FIRST`]).
//!
//! The movement abilities (High Jump, Glide, Guard, Combo Plus …) stay in `kh1-recom.ini`
//! (`settings::Kh1Abilities`) until the equip screen.

/// Where an ability unlocks from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Re:CoM's sleight of this id is learned (its level-up list `0x14010EF10`).
    Sleight(u16),
    /// Re:CoM's level reaches KH1's learn level (KH1's Rod list, btltbl `0x3BF8`).
    Level(u32),
    /// No source decided yet (owner question): never unlocks by itself.
    Undecided,
}

/// One ability of the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ability {
    pub name: &'static str,
    /// KH1's ability id (the ability list `save + 0x44`, btltbl `0x33D8` row id − 1). None: not
    /// read yet (the specials, stage 9c).
    pub kh1_id: Option<u8>,
    /// KH1's bit in `0x142D5EC10` (`0x1402A6380`), for the modifiers the pickers read.
    pub kh1_bit: u32,
    /// KH1's AP cost (btltbl `0x33D8 + 12 × (id − 1)`, u16 at +0), for stage 9b.
    pub ap: u8,
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

/// Every ability of the state, by [`ix`]. KH1 ids, bits and AP: Checked (decomp `0x1402A6380` +
/// files btltbl `0x33D8`); names of the ids: the Archipelago list (Assumption, strong). Learn
/// levels: KH1's Rod list btltbl `0x3BF8` (entry i = level i + 2, as `0x1402B51B0` reads it with the
/// level-up bonus list `0x3AC0`, whose MP +1 entries fall at 12 / 24 / 36 / 48 / 60). Checked
/// (decomp + files).
pub const ALL: [Ability; 13] = [
    Ability { name: "Sliding Dash", kh1_id: Some(54), kh1_bit: 0x100_0000, ap: 1, source: Source::Sleight(sleight::SLIDING_DASH) },
    Ability { name: "Blitz", kh1_id: Some(20), kh1_bit: 0x8, ap: 2, source: Source::Sleight(sleight::BLITZ) },
    Ability { name: "Stun Impact", kh1_id: Some(57), kh1_bit: 0x800_0000, ap: 2, source: Source::Sleight(sleight::STUN_IMPACT) },
    Ability { name: "Zantetsuken", kh1_id: Some(59), kh1_bit: 0x2000_0000, ap: 2, source: Source::Sleight(sleight::ZANTETSUKEN) },
    Ability { name: "Vortex", kh1_id: Some(17), kh1_bit: 0x1, ap: 1, source: Source::Level(84) },
    Ability { name: "Aerial Sweep", kh1_id: Some(18), kh1_bit: 0x2, ap: 2, source: Source::Level(36) },
    Ability { name: "Hurricane Blast", kh1_id: Some(55), kh1_bit: 0x200_0000, ap: 2, source: Source::Level(18) },
    Ability { name: "Ripple Drive", kh1_id: Some(56), kh1_bit: 0x400_0000, ap: 3, source: Source::Level(12) },
    // KH1 learns Gravity Break from no level list (none of 0x3B28 / 0x3B90 / 0x3BF8 holds 0xBA):
    // owner question.
    Ability { name: "Gravity Break", kh1_id: Some(58), kh1_bit: 0x1000_0000, ap: 2, source: Source::Undecided },
    // The specials (stage 9c): their state is kept now; nothing reads it yet.
    Ability { name: "Strike Raid", kh1_id: None, kh1_bit: 0, ap: 0, source: Source::Sleight(sleight::STRIKE_RAID) },
    Ability { name: "Sonic Blade", kh1_id: None, kh1_bit: 0, ap: 0, source: Source::Sleight(sleight::SONIC_BLADE) },
    Ability { name: "Ars Arcanum", kh1_id: None, kh1_bit: 0, ap: 0, source: Source::Sleight(sleight::ARS_ARCANUM) },
    Ability { name: "Ragnarok", kh1_id: None, kh1_bit: 0, ap: 0, source: Source::Sleight(sleight::RAGNAROK) },
];

/// The combo modifiers (stage 9a), by [`ix`].
pub const MODIFIERS: std::ops::Range<usize> = ix::SLIDING_DASH..ix::GRAVITY_BREAK + 1;
/// Every bit of [`ALL`].
pub const ALL_BITS: u32 = (1 << ALL.len()) - 1;
/// The modifiers' bits.
pub const MODIFIER_BITS: u32 = (1 << MODIFIERS.end) - 1;

// ---- the save block ----

/// Our ability block: right after the items extension, inside Re:CoM's spare tail
/// `save + 0x246B ..= 0x2817` that no Re:CoM code reads or writes (`docs/recom-keyblade-equip.md`).
pub const BLOCK_FIRST: usize = crate::items::EXT_LAST + 1;
pub const BLOCK_LEN: usize = 15;
pub const BLOCK_LAST: usize = BLOCK_FIRST + BLOCK_LEN - 1;

/// Offsets inside the block. Fields never move.
pub mod field {
    /// 0 = never written (an older save, or a new game): nothing unlocked yet; 1 = written.
    pub const VERSION: usize = 0;
    /// u32 LE: unlocked abilities, bit [`super::ix`].
    pub const UNLOCKED: usize = 1;
    /// u32 LE: equipped abilities, bit [`super::ix`].
    pub const EQUIPPED: usize = 5;
    /// 6 bytes kept zero for stage 9b (KH1's AP: the total and what is spent).
    pub const RESERVED: usize = 9;
}

pub const BLOCK_V1: u8 = 1;

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

/// Writes the state and the block's version (never lowered); the reserved bytes are kept.
pub fn write(s: &State, block: &mut [u8; BLOCK_LEN]) {
    block[field::VERSION] = block[field::VERSION].max(BLOCK_V1);
    block[field::UNLOCKED..field::UNLOCKED + 4].copy_from_slice(&s.unlocked.to_le_bytes());
    block[field::EQUIPPED..field::EQUIPPED + 4].copy_from_slice(&s.equipped.to_le_bytes());
}

/// Whether an ability's source is met: Re:CoM's sleight learned (`learned`, Re:CoM's own
/// `0x14023AB00`), or Re:CoM's level at KH1's learn level.
pub fn source_met(a: &Ability, level: u32, learned: &mut dyn FnMut(u16) -> bool) -> bool {
    match a.source {
        Source::Sleight(id) => learned(id),
        Source::Level(l) => l <= level,
        Source::Undecided => false,
    }
}

/// Unlocks what Re:CoM's state and level now give (never taking one back). A new unlock is also
/// equipped (the interim rule until stage 9b's equip screen). Returns the newly unlocked bits.
/// Only abilities not yet unlocked are asked about.
pub fn sync(s: &mut State, level: u32, learned: &mut dyn FnMut(u16) -> bool) -> u32 {
    let mut new = 0;
    for (i, a) in ALL.iter().enumerate() {
        let bit = 1 << i;
        if s.unlocked & bit == 0 && source_met(a, level, learned) {
            new |= bit;
        }
    }
    s.unlocked |= new;
    s.equipped |= new;
    new
}

/// The equipped modifiers as KH1's ability bits (`0x142D5EC10` layout), as the trace writes them.
pub fn kh1_bits(equipped: u32) -> u32 {
    ALL.iter().enumerate().filter(|(i, _)| equipped & (1 << i) != 0).fold(0, |b, (_, a)| b | a.kh1_bit)
}

/// The equipped modifiers into the abilities the KH1 pickers read.
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

/// The names of the abilities in `bits`, for the log.
pub fn names(bits: u32) -> Vec<&'static str> {
    ALL.iter().enumerate().filter(|(i, _)| bits & (1 << i) != 0).map(|(_, a)| a.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_round_trip_and_new_game() {
        let mut b = [0u8; BLOCK_LEN];
        assert_eq!(read(&b), State::default(), "a block never written: nothing unlocked");
        let s = State { unlocked: 0b1_0101, equipped: 0b101 };
        write(&s, &mut b);
        assert_eq!(b[field::VERSION], BLOCK_V1);
        assert_eq!(read(&b), s);
        // Equipped is only ever a subset of unlocked; unknown bits are dropped.
        b[field::EQUIPPED..field::EQUIPPED + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        b[field::UNLOCKED..field::UNLOCKED + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(read(&b), State { unlocked: ALL_BITS, equipped: ALL_BITS });
        assert!(b[field::RESERVED..].iter().all(|&x| x == 0));
    }

    #[test]
    fn layout_follows_the_items_extension() {
        assert_eq!(BLOCK_FIRST, 0x269F);
        assert_eq!(BLOCK_LAST, 0x26AD);
    }

    #[test]
    fn unlocks_from_recoms_sleights_and_kh1s_levels() {
        let mut s = State::default();
        // Level 1, no sleight: nothing.
        assert_eq!(sync(&mut s, 1, &mut |_| false), 0);
        // Re:CoM's Lv2 sleight (Sliding Dash, 4) and Lv 12: Ripple Drive (KH1's Rod list).
        let new = sync(&mut s, 12, &mut |id| id == sleight::SLIDING_DASH);
        assert_eq!(names(new), vec!["Sliding Dash", "Ripple Drive"]);
        assert_eq!(s.equipped, s.unlocked, "the interim rule: unlocked = equipped");
        // Only abilities not yet unlocked are asked about.
        let mut asked = Vec::new();
        sync(&mut s, 12, &mut |id| {
            asked.push(id);
            false
        });
        assert!(!asked.contains(&sleight::SLIDING_DASH));
        // Never taken back (a lower level or a lost sleight).
        sync(&mut s, 1, &mut |_| false);
        assert_eq!(names(s.unlocked), vec!["Sliding Dash", "Ripple Drive"]);
        // An unequipped one (9b) is not equipped again by a later sync.
        s.equipped &= !(1 << ix::RIPPLE_DRIVE);
        sync(&mut s, 18, &mut |_| false);
        assert_eq!(names(s.equipped), vec!["Sliding Dash", "Hurricane Blast"]);
        // The rest of the Rod list: Aerial Sweep 36, Vortex 84; Gravity Break never by itself.
        sync(&mut s, 100, &mut |_| true);
        assert_eq!(s.unlocked, ALL_BITS & !(1 << ix::GRAVITY_BREAK));
    }

    #[test]
    fn learn_levels_are_kh1s_rod_list() {
        let lv = |i: usize| match ALL[i].source {
            Source::Level(l) => l,
            _ => 0,
        };
        assert_eq!([lv(ix::RIPPLE_DRIVE), lv(ix::HURRICANE_BLAST), lv(ix::AERIAL_SWEEP), lv(ix::VORTEX)], [12, 18, 36, 84]);
        assert_eq!(ALL[ix::GRAVITY_BREAK].source, Source::Undecided);
    }

    #[test]
    fn equipped_bits_to_the_pickers() {
        let mut a = kh1_core::Abilities::default();
        apply(1 << ix::STUN_IMPACT | 1 << ix::VORTEX | 1 << ix::STRIKE_RAID, &mut a);
        assert!(a.stun_impact && a.vortex && !a.zantetsuken && !a.blitz);
        assert_eq!(kh1_bits(1 << ix::STUN_IMPACT | 1 << ix::VORTEX), 0x800_0000 | 0x1);
        assert_eq!(kh1_bits(MODIFIER_BITS), 0x3F00_0000 | 0x8 | 0x1 | 0x2);
        // The specials have no picker bit yet.
        assert_eq!(kh1_bits(1 << ix::RAGNAROK), 0);
    }

    /// KH1's AP costs (btltbl `0x33D8`, row id − 1, u16 +0). Checked (files).
    #[test]
    fn ap_costs() {
        let ap: Vec<(u8, u8)> = ALL[MODIFIERS].iter().map(|a| (a.kh1_id.unwrap(), a.ap)).collect();
        assert_eq!(ap, vec![(54, 1), (20, 2), (57, 2), (59, 2), (17, 1), (18, 2), (55, 2), (56, 3), (58, 2)]);
    }
}
