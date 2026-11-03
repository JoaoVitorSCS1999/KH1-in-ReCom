//! M7 stage 7.11, spell unlocks (decision 34: "Spell unlocks: each spell's base level when Sora
//! first gets that Re:CoM magic card; the -ra / -ga levels at Re:CoM bosses"): the testable part.
//! The runtime is `win/unlocks.rs`; the table and its reasons are in `docs/recom-m7-unlocks.md`.
//!
//! **Everything is derived from Re:CoM's own card discovery state** (`coll + 0x9AC`, 2 bits per card
//! id; "obtained at least once" = state 2 or 3, Re:CoM's `0x14015B3F0`; never cleared but by a new
//! game). Every way a card enters Sora's collection (drops, rewards, event gifts, packs, shops, the
//! new-game deck) goes through Re:CoM's add `0x140158FC0` → `0x14015C7F0`, which turns the state
//! from 1 to 2 the first time (Checked (decomp + exe), `recom-cards-save.md` § 3):
//!
//! - **base level (1)**: the spell's magic card (kinds `0x1A`..`0x20`, Fire..Aero, normal or
//!   premium `| 0x100`) obtained at least once;
//! - **-ra / -ga**: each spell has **two bosses** ([`BOSS_LEARNS`]); every one of them beaten (its
//!   enemy card, type 7, obtained at least once: Re:CoM's reward for the boss) raises the spell one
//!   level, up to 3, once Sora has the spell. Re:CoM lets the player choose the order of floors
//!   2–6 and 7–10, so the rule is "+1 per boss" (always level 2, then 3, as KH1's order), not "this
//!   boss gives level 3".
//!
//! The levels are kept in our save block (`crate::mp::off::SPELLS`, KH1's save `+0x492 + spell`),
//! raised when they rise and never lowered.

/// KH1's spells (`crate::magic::spell`), and Re:CoM's magic cards for them (the card-info table
/// `0x1405256D0`: kinds `0x1A`..`0x20` = Fire, Blizzard, Thunder, Cure, Gravity, Stop, Aero, type 2,
/// premium versions `| 0x100`). Checked (exe).
pub const MAGIC_CARD_FIRST: u16 = 0x1A;
pub const PREMIUM: u16 = 0x100;
pub const SPELLS: usize = 7;

/// The magic card kind of a spell.
pub fn magic_card(spell: u8) -> u16 {
    MAGIC_CARD_FIRST + spell as u16
}

/// The spell of a magic card kind (premium or not).
pub fn spell_of_card(id: u16) -> Option<u8> {
    let kind = id & 0xFF;
    (MAGIC_CARD_FIRST..MAGIC_CARD_FIRST + SPELLS as u16).contains(&kind).then(|| (kind - MAGIC_CARD_FIRST) as u8)
}

/// KH1's names of the spell levels (KH1's own words come from the player's files in the menu; these
/// are for the log).
pub fn level_name(spell: u8, level: u8) -> &'static str {
    const N: [[&str; 3]; SPELLS] = [
        ["Fire", "Fira", "Firaga"],
        ["Blizzard", "Blizzara", "Blizzaga"],
        ["Thunder", "Thundara", "Thundaga"],
        ["Cure", "Cura", "Curaga"],
        ["Gravity", "Gravira", "Graviga"],
        ["Stop", "Stopra", "Stopga"],
        ["Aero", "Aerora", "Aeroga"],
    ];
    match (N.get(spell as usize), level) {
        (Some(n), 1..=3) => n[level as usize - 1],
        _ => "?",
    }
}

/// Why a boss gives a spell level (the table's rule, `docs/recom-m7-unlocks.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// KH1 gives this level for beating the same boss.
    SameBoss,
    /// KH1 gives it at the same world's ending (its keyhole), right after the boss Re:CoM keeps.
    SameWorld,
    /// No Re:CoM counterpart: KH1's next learn in its order on Re:CoM's next free boss in story order.
    Gap,
}

/// One row of the table: a Re:CoM boss (its enemy card) and the spell level it stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BossLearn {
    /// Re:CoM's enemy card of the boss (type 7; the card-info table's names, Checked (exe)).
    pub card: u16,
    pub boss: &'static str,
    /// Where Sora meets it in Re:CoM (Assumption for the Organization's floors; `docs`).
    pub floor: &'static str,
    pub spell: u8,
    /// The level KH1's order has here (the rule itself is "+1 per boss", above).
    pub level: u8,
    /// KH1's source of that level (StrategyWiki, Assumption except where the docs say Checked).
    pub kh1: &'static str,
    pub why: Why,
}

use crate::magic::spell::*;

/// The fourteen -ra / -ga learns, two per spell. Same boss first, then same world, then the rest in
/// KH1's order on the remaining Re:CoM bosses in story order (`docs/recom-m7-unlocks.md`).
pub const BOSS_LEARNS: [BossLearn; 14] = [
    BossLearn { card: 0x69, boss: "Jafar", floor: "2F-6F (Agrabah)", spell: FIRE, level: 2, kh1: "Genie Jafar (Agrabah)", why: Why::SameBoss },
    BossLearn { card: 0x69, boss: "Jafar", floor: "2F-6F (Agrabah)", spell: BLIZZARD, level: 2, kh1: "Jafar (Agrabah)", why: Why::SameBoss },
    BossLearn { card: 0x6B, boss: "Ursula", floor: "7F-10F (Atlantica)", spell: THUNDER, level: 2, kh1: "Giant Ursula (Atlantica)", why: Why::SameBoss },
    BossLearn { card: 0x6C, boss: "Captain Hook", floor: "7F-10F (Neverland)", spell: CURE, level: 2, kh1: "Captain Hook, on the ship's deck (Neverland)", why: Why::SameBoss },
    BossLearn { card: 0x68, boss: "Hades", floor: "2F-6F (Olympus Coliseum)", spell: GRAVITY, level: 3, kh1: "Hades (Hades Cup)", why: Why::SameBoss },
    BossLearn { card: 0x6A, boss: "Oogie Boogie", floor: "2F-6F (Halloween Town)", spell: GRAVITY, level: 2, kh1: "Halloween Town keyhole (after Oogie Boogie)", why: Why::SameWorld },
    BossLearn { card: 0x6D, boss: "Dragon Maleficent", floor: "7F-10F (Hollow Bastion)", spell: FIRE, level: 3, kh1: "Hollow Bastion keyhole", why: Why::SameWorld },
    BossLearn { card: 0x6F, boss: "Axel", floor: "1F, after Traverse Town (Axel I)", spell: AERO, level: 2, kh1: "Neverland: the Trinity chest", why: Why::Gap },
    BossLearn { card: 0x65, boss: "Trickmaster", floor: "2F-6F (Wonderland)", spell: STOP, level: 2, kh1: "100 Acre Wood: Pooh's swing", why: Why::Gap },
    BossLearn { card: 0x64, boss: "Parasite Cage", floor: "2F-6F (Monstro)", spell: CURE, level: 3, kh1: "Hollow Bastion library: Aerith", why: Why::Gap },
    BossLearn { card: 0x70, boss: "Larxene", floor: "Larxene I", spell: STOP, level: 3, kh1: "Phantom (Neverland)", why: Why::Gap },
    BossLearn { card: 0x6E, boss: "Riku Replica", floor: "Riku Replica I", spell: BLIZZARD, level: 3, kh1: "Behemoth (Hades Cup)", why: Why::Gap },
    BossLearn { card: 0x71, boss: "Vexen", floor: "Vexen I", spell: THUNDER, level: 3, kh1: "Cerberus (Hades Cup)", why: Why::Gap },
    BossLearn { card: 0x66, boss: "Darkside", floor: "12F (Destiny Islands)", spell: AERO, level: 3, kh1: "99 Dalmatians", why: Why::Gap },
];

/// Whether a card id (normal or premium) matters to the unlocks: a magic card or a table boss.
pub fn tracked(id: u16) -> bool {
    spell_of_card(id).is_some() || (id < PREMIUM && BOSS_LEARNS.iter().any(|b| b.card == id))
}

/// The spell levels Re:CoM's discovery state gives (`obtained(id)`: Re:CoM's `0x14015B3F0`).
pub fn levels(mut obtained: impl FnMut(u16) -> bool) -> [u8; SPELLS] {
    let mut out = [0u8; SPELLS];
    for (s, l) in out.iter_mut().enumerate() {
        let card = magic_card(s as u8);
        if !(obtained(card) || obtained(card | PREMIUM)) {
            continue;
        }
        let bosses = BOSS_LEARNS.iter().filter(|b| b.spell as usize == s && obtained(b.card)).count();
        *l = (1 + bosses).min(3) as u8;
    }
    out
}

/// What the save keeps: never lower than what it had (a later source of levels, e.g. KH1-style
/// events, may have raised one), at most 3; a stored value above 3 counts as 0 (as `win::mp::spell_state`).
pub fn merged(stored: [u8; SPELLS], derived: [u8; SPELLS]) -> [u8; SPELLS] {
    let mut out = [0u8; SPELLS];
    for i in 0..SPELLS {
        let s = if stored[i] <= 3 { stored[i] } else { 0 };
        out[i] = s.max(derived[i]).min(3);
    }
    out
}

/// The learns between two level sets: (spell, from, to) for each spell that rose.
pub fn learns(before: [u8; SPELLS], after: [u8; SPELLS]) -> Vec<(u8, u8, u8)> {
    (0..SPELLS).filter(|&i| after[i] > before[i]).map(|i| (i as u8, before[i], after[i])).collect()
}

/// The spare byte of our save block (`crate::mp::off::SPARE`), bit 0: the one-time sync from
/// Re:CoM's discovery state has run for this save (an older save, or one played at the Re:CoM
/// setting before, gets its levels at its first read at the KH1 setting). Zero in every save
/// before this stage.
pub const SYNCED: u8 = 1;

/// The spare byte, bit 1 (stage 7.12): this save's shortcut slots were set up (zero in every save
/// before this stage: the first sync at or after it fills them once from the spells already
/// learned).
pub const SHORTCUTS_SET: u8 = 2;

/// KH1's learn (`0x140285260`, the script's learn-a-spell command): the spell goes into the first
/// empty shortcut slot, if one is empty (no duplicate check; a learn happens once per spell).
/// Checked (decomp). Slots: 0 = none, else spell + 1 (`crate::command_menu::shortcut`); a slot
/// holding a sleight (M12 step 6) is set, not empty: KH1's learn never overwrites it.
pub fn fill_slot(slots: &mut [u8; 3], spell: u8) -> Option<usize> {
    use crate::command_menu::shortcut as sc;
    let k = slots.iter().position(|&b| !sc::is_set(b))?;
    slots[k] = sc::slot_byte(spell);
    Some(k)
}

/// The shortcut slots after a sync from `before` to `after` (stage 7.12; owner, 2026-11-01:
/// "Slots filled by KH1's fill-on-learn rule; existing saves: fill once from the spells already
/// learned, in KH1's spell order, first three"):
/// - a save whose slots were never set up (`set_up` false) first gets the spells it already had
///   (`before`), in KH1's spell order, into the first three slots (bytes other than a spell + 1
///   or, M12 step 6, a sleight ability's id count as empty and are cleared);
/// - then every spell learned now (level 0 → more), in spell order, fills the first empty slot.
///
/// Returns the slots and, per slot filled, the spell (for the log).
pub fn shortcuts(slots: [u8; 3], set_up: bool, before: [u8; SPELLS], after: [u8; SPELLS]) -> ([u8; 3], Vec<(usize, u8)>) {
    use crate::command_menu::shortcut as sc;
    let mut s = slots.map(|b| if sc::is_set(b) { b } else { 0 });
    let mut filled = Vec::new();
    if !set_up {
        for sp in (0..SPELLS as u8).filter(|&i| (1..=3).contains(&before[i as usize])) {
            if let Some(k) = fill_slot(&mut s, sp) {
                filled.push((k, sp));
            }
        }
    }
    for (sp, from, _) in learns(before, after) {
        if from == 0 {
            if let Some(k) = fill_slot(&mut s, sp) {
                filled.push((k, sp));
            }
        }
    }
    (s, filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_spell_has_two_bosses_and_every_boss_card_is_a_sora_boss() {
        for s in 0..SPELLS as u8 {
            let rows: Vec<&BossLearn> = BOSS_LEARNS.iter().filter(|b| b.spell == s).collect();
            assert_eq!(rows.len(), 2, "{}", level_name(s, 1));
            let mut lv: Vec<u8> = rows.iter().map(|b| b.level).collect();
            lv.sort();
            assert_eq!(lv, vec![2, 3], "{}", level_name(s, 1));
        }
        // Re:CoM's boss enemy cards (type 7, the card-info table, Checked (exe)): 0x63 Guard Armor,
        // 0x64 Parasite Cage, 0x65 Trickmaster, 0x66 Darkside, 0x68 Hades, 0x69 Jafar, 0x6A Oogie
        // Boogie, 0x6B Ursula, 0x6C Hook, 0x6D Dragon Maleficent, 0x6E Riku, 0x6F Axel, 0x70
        // Larxene, 0x71 Vexen, 0x72 Marluxia. Every table card is one of them; only Jafar carries
        // two (KH1's Agrabah gives two); Guard Armor (KH1's Fire: the base level comes from the card)
        // and Marluxia (the last fight) carry none.
        let sora_bosses = [0x64u16, 0x65, 0x66, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F, 0x70, 0x71];
        for b in BOSS_LEARNS {
            assert!(sora_bosses.contains(&b.card), "{}", b.boss);
            let n = BOSS_LEARNS.iter().filter(|x| x.card == b.card).count();
            assert_eq!(n, if b.card == 0x69 { 2 } else { 1 }, "{}", b.boss);
        }
        assert!(!BOSS_LEARNS.iter().any(|b| b.card == 0x63 || b.card == 0x72));
        // Same boss: Jafar ×2, Ursula, Hook, Hades; same world: Oogie, Dragon Maleficent; the rest gaps.
        let n = |w| BOSS_LEARNS.iter().filter(|b| b.why == w).count();
        assert_eq!((n(Why::SameBoss), n(Why::SameWorld), n(Why::Gap)), (5, 2, 7));
    }

    #[test]
    fn levels_come_from_the_card_then_one_per_boss() {
        let have = |ids: &'static [u16]| move |id: u16| ids.contains(&id);
        // Nothing: nothing.
        assert_eq!(levels(have(&[])), [0; 7]);
        // The Fire card: Fire 1. A premium Blizzard card: Blizzard 1.
        assert_eq!(levels(have(&[0x1A, 0x11B])), [1, 1, 0, 0, 0, 0, 0]);
        // Jafar beaten with Fire and Blizzard: Fira and Blizzara.
        assert_eq!(levels(have(&[0x1A, 0x1B, 0x69])), [2, 2, 0, 0, 0, 0, 0]);
        // A boss before its spell's card gives nothing until the card comes: then the level at once.
        assert_eq!(levels(have(&[0x69])), [0; 7]);
        assert_eq!(levels(have(&[0x69, 0x6D, 0x1A])), [3, 0, 0, 0, 0, 0, 0]);
        // Hades before Oogie (Re:CoM's free floor order): Gravity 2 at Hades, 3 at Oogie.
        assert_eq!(levels(have(&[0x1E, 0x68])), [0, 0, 0, 0, 2, 0, 0]);
        assert_eq!(levels(have(&[0x1E, 0x68, 0x6A])), [0, 0, 0, 0, 3, 0, 0]);
        // Every card and boss: all at 3.
        let all: Vec<u16> = (0x1A..=0x20).chain(BOSS_LEARNS.iter().map(|b| b.card)).collect();
        assert_eq!(levels(|id| all.contains(&id)), [3; 7]);
        // Guard Armor and Marluxia raise nothing.
        assert_eq!(levels(have(&[0x1A, 0x63, 0x72])), [1, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn the_save_keeps_the_highest_and_logs_each_rise() {
        assert_eq!(merged([0, 2, 3, 9, 0, 0, 1], [1, 1, 3, 1, 0, 2, 0]), [1, 2, 3, 1, 0, 2, 1]);
        assert_eq!(learns([0, 2, 3, 0, 0, 0, 1], [1, 2, 3, 1, 0, 2, 1]), vec![(0, 0, 1), (3, 0, 1), (5, 0, 2)]);
        assert_eq!((level_name(0, 2), level_name(4, 3), level_name(6, 1), level_name(7, 1), level_name(0, 0)), ("Fira", "Graviga", "Aero", "?", "?"));
        assert!(tracked(0x1A) && tracked(0x120) && tracked(0x6F) && !tracked(0x63) && !tracked(0x72) && !tracked(0x44) && !tracked(0x16F));
        assert_eq!((spell_of_card(0x11C), spell_of_card(0x21), magic_card(6)), (Some(2), None, 0x20));
        // The flag lives in the block's spare byte.
        assert!(crate::mp::off::SPARE < crate::mp::BLOCK_LEN && SYNCED == 1);
    }

    #[test]
    fn shortcuts_fill_as_kh1_learns() {
        let none = [0u8; 7];
        // A new game: nothing learned, slots empty; Fire learned → slot 1, then Cure → slot 2.
        let (s, f) = shortcuts([0; 3], false, none, none);
        assert_eq!((s, f), ([0; 3], vec![]));
        let (s, f) = shortcuts(s, true, none, [1, 0, 0, 0, 0, 0, 0]);
        assert_eq!((s, f), ([1, 0, 0], vec![(0, 0)]));
        let (s, _) = shortcuts(s, true, [1, 0, 0, 0, 0, 0, 0], [1, 0, 0, 1, 0, 0, 0]);
        assert_eq!(s, [1, 4, 0]);
        // A level rise fills nothing.
        let (s2, f) = shortcuts(s, true, [1, 0, 0, 1, 0, 0, 0], [2, 0, 0, 1, 0, 0, 0]);
        assert_eq!((s2, f), (s, vec![]));
        // Full slots: a new spell goes nowhere (KH1 keeps the three).
        let (s, f) = shortcuts([1, 2, 3], true, [1, 1, 1, 0, 0, 0, 0], [1, 1, 1, 1, 0, 0, 0]);
        assert_eq!((s, f), ([1, 2, 3], vec![]));
        // An existing save (spells learned, never set up): its first three in KH1's order, once.
        let had = [0, 2, 0, 1, 3, 1, 0];
        let (s, f) = shortcuts([0; 3], false, had, had);
        assert_eq!((s, f), ([2, 4, 5], vec![(0, 1), (1, 3), (2, 4)]));
        // Set up already: not filled again (a cleared slot stays clear until the next learn).
        assert_eq!(shortcuts([2, 0, 5], true, had, had).0, [2, 0, 5]);
        // Never set up with a learn in the same sync: what it had first, then the new one.
        let (s, _) = shortcuts([0; 3], false, [0, 0, 0, 1, 0, 0, 0], [1, 0, 0, 1, 0, 0, 0]);
        assert_eq!(s, [4, 1, 0]);
        // A bad byte counts as empty.
        assert_eq!(shortcuts([9, 0, 0], true, none, [0, 0, 1, 0, 0, 0, 0]).0, [3, 0, 0]);
        // M12 step 6: a sleight slot (Fire Raid 0x1B) is set: kept, and a learn skips it.
        assert_eq!(shortcuts([0x1B, 0, 0], true, none, [0, 0, 1, 0, 0, 0, 0]).0, [0x1B, 3, 0]);
        assert_eq!(shortcuts([0x1B, 0x43, 0x24], true, none, [1, 0, 0, 0, 0, 0, 0]).0, [0x1B, 0x43, 0x24]);
        assert_eq!(SHORTCUTS_SET & SYNCED, 0);
    }
}
