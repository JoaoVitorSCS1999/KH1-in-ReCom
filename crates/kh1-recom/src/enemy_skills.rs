//! Enemy Skills (owner, 2026-11-03 / 2026-11-04; plan `docs/recom-enemy-skills.md`, stage 1): Re:CoM's
//! enemy cards as KH1 abilities at Combat = KH1 in Sora's story.
//!
//! - **The table** ([`SKILLS`]): each skill's enemy card (Re:CoM's card kind 0x44..=0x7E, type 7),
//!   the card's name (owner: "Have each ability have the name of the original card"), a one-line
//!   description of what it does at KH1 (ours, in English; the help line, within
//!   `crate::menu::HELP_MAX`), its AP (the owner's table) and whether its effect is built.
//! - **The five KH1 matches** ([`Kind::Kh1`]; owner: "all enemy cards that are the same as a KH1
//!   Ability should replace its respective KH1 ability"): Soldier → one Combo Plus (the Rod level-87
//!   copy), Hook → Second Chance (90), Hades → Berserk (60), Creeper Plant → Leaf Bracer (39),
//!   Bouncywild → Treasure Magnet (one copy: KH1's level-9 one). They are those abilities of
//!   `crate::abilities` (their bits, KH1's AP, the existing effects), learned from the card instead
//!   of the level.
//! - **The other skills** ([`Kind::Own`]) are abilities of `crate::abilities` too (from
//!   `crate::abilities::FIRST_SKILL`, in this table's order), with the owner's AP; their effects come
//!   in stages 2..5 of the plan.
//! - **Learning:** a skill unlocks at its card's first pickup: Re:CoM's discovery state
//!   (`0x14015B3F0`, "obtained at least once"; the first-obtained wrapper on `0x14015C7F0`), as the
//!   spells, summons and Keyblades. Checked (decomp: `docs/recom-m7-unlocks.md`).
//! - **Names:** Re:CoM's card-info table `0x1405256D0` (16-byte rows, the name pointer at `+8`; the
//!   page shows Re:CoM's localized card name, message `[0x140528560 + kind × 4]` = the kind, through
//!   `0x140164CA0`). Checked (exe). Ours are its English names in ASCII (Saïx → "Saix": Re:CoM's
//!   string has its own `ï` byte; Assumption that our notice's font has no `ï`, so ASCII).
//! - **Left out** (owner): the card-value cards (Shadow, Powerwild, Sea Neon, Screwdiver, Pirate),
//!   Value Break (Trickmaster), Dispel (Parasite Cage), Mimic (Darkside), Hyper Healing (White
//!   Mushroom), Darkball (Duel Trigger) and Wyvern (Reload Lock): [`LEFT_OUT`].

/// How a skill acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// One of KH1's abilities (`crate::abilities::ix`): its bit, its KH1 AP and its effect.
    Kh1(usize),
    /// A skill of its own (its effect: stages 2..5).
    Own,
}

/// One Enemy Skill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Skill {
    /// Re:CoM's enemy card kind (0x44..=0x7E).
    pub card: u16,
    /// The card's English name (ASCII).
    pub name: &'static str,
    /// Re:CoM's effect name (the exe's table `0x1407C2278`, index card − 0x44), for the log.
    pub effect: &'static str,
    /// AP to equip: the owner's table (2026-11-04), KH1's own for the five matches (btltbl `0x33D8`).
    pub ap: u8,
    /// The help line (NUL-terminated ASCII): what it does at KH1. A skill whose effect isn't built
    /// yet ends "(effect coming)"; Treasure Magnet "(not built yet)".
    pub help: &'static [u8],
    pub kind: Kind,
    /// Its effect is built (the page lets it be equipped). False: shown greyed, never equipped.
    pub built: bool,
}

const fn own(card: u16, name: &'static str, effect: &'static str, ap: u8, help: &'static [u8]) -> Skill {
    Skill { card, name, effect, ap, help, kind: Kind::Own, built: false }
}

const fn kh1(card: u16, name: &'static str, effect: &'static str, ap: u8, ix: usize, built: bool, help: &'static [u8]) -> Skill {
    Skill { card, name, effect, ap, help, kind: Kind::Kh1(ix), built }
}

use crate::abilities::ix;

/// Every Enemy Skill, by card kind. The [`Kind::Own`] ones are `crate::abilities::ALL[FIRST_SKILL..]`
/// in this order: **never reorder or remove** (their save bits follow it).
pub const SKILLS: [Skill; 48] = [
    kh1(0x45, "Soldier", "Combo Plus", 1, ix::COMBO_PLUS[0], true, b"Combo Plus: one more swing in a ground combo.\0"),
    own(0x46, "Large Body", "Guard", 4, b"Guard: blocks physical attacks from the front. (effect coming)\0"),
    own(0x47, "Red Nocturne", "Fire Boost", 2, b"Fire Boost: raises the damage of Fire. (effect coming)\0"),
    own(0x48, "Blue Rhapsody", "Blizzard Boost", 2, b"Blizzard Boost: raises the damage of Blizzard. (effect coming)\0"),
    own(0x49, "Yellow Opera", "Thunder Boost", 2, b"Thunder Boost: raises the damage of Thunder. (effect coming)\0"),
    own(0x4A, "Green Requiem", "Cure Boost", 2, b"Cure Boost: Cure restores more HP. (effect coming)\0"),
    kh1(0x4C, "Bouncywild", "Draw", 2, ix::TREASURE_MAGNET[0], false, b"Treasure Magnet: draws prizes in from afar. (not built yet)\0"),
    own(0x4D, "Air Soldier", "Reload Kinesis", 2, b"Reload Kinesis: MP recharges while Sora moves. (effect coming)\0"),
    own(0x4E, "Bandit", "Combo Finish", 7, b"Combo Finish: every swing is a combo finisher. (effect coming)\0"),
    own(0x4F, "Fat Bandit", "Back Attack", 2, b"Back Attack: stronger hits from behind. (effect coming)\0"),
    own(0x50, "Barrel Spider", "Quickload", 4, b"Quickload: MP recharges in one go. (effect coming)\0"),
    own(0x51, "Search Ghost", "Drain", 3, b"Drain: Sora's swings restore his HP. (effect coming)\0"),
    own(0x54, "Aquatank", "Auto-Reload", 3, b"Auto-Reload: MP refills once when it runs out. (effect coming)\0"),
    own(0x55, "Wight Knight", "Float", 2, b"Float: gravity in battle is halved. (effect coming)\0"),
    own(0x56, "Gargoyle", "Vanish", 3, b"Vanish: Sora fades in and out of sight. (effect coming)\0"),
    own(0x58, "Air Pirate", "Item Bracer", 2, b"Item Bracer: using an item can't be interrupted. (effect coming)\0"),
    own(0x5A, "Defender", "Protect", 3, b"Protect: halves damage from physical attacks. (effect coming)\0"),
    own(0x5C, "Wizard", "Magic Boost", 3, b"Magic Boost: raises the damage of magic. (effect coming)\0"),
    own(0x5D, "Neoshadow", "Bio", 3, b"Bio: enemies slowly lose HP. (effect coming)\0"),
    own(0x5F, "Black Fungus", "Random Flush", 2, b"Random Flush: a random skill's effect each battle. (effect coming)\0"),
    kh1(0x60, "Creeper Plant", "Leaf Bracer", 5, ix::LEAF_BRACER, true, b"Leaf Bracer: nothing can interrupt Sora while he casts Cure.\0"),
    own(0x61, "Tornado Step", "Reload Haste", 3, b"Reload Haste: MP recharges faster. (effect coming)\0"),
    own(0x62, "Crescendo", "Summon Boost", 3, b"Summon Boost: raises the damage of summons. (effect coming)\0"),
    own(0x63, "Guard Armor", "Wide Attack", 2, b"Wide Attack: Sora's swings hit a wider area. (effect coming)\0"),
    own(0x67, "Card Soldier", "Attack Haste", 4, b"Attack Haste: Sora swings twice as fast. (effect coming)\0"),
    kh1(0x68, "Hades", "Berserk", 1, ix::BERSERK, true, b"Berserk: raises Strength while Sora's HP is low.\0"),
    own(0x69, "Jafar", "Attack Bracer", 4, b"Attack Bracer: Sora's swings can't be interrupted. (effect coming)\0"),
    own(0x6A, "Oogie Boogie", "Regen", 4, b"Regen: slowly restores Sora's HP. (effect coming)\0"),
    own(0x6B, "Ursula", "Shell", 3, b"Shell: halves damage from Fire, Blizzard, Thunder. (effect coming)\0"),
    kh1(0x6C, "Hook", "Second Chance", 5, ix::SECOND_CHANCE, true, b"Second Chance: survive a knockout blow with 1 HP.\0"),
    own(0x6D, "Dragon Maleficent", "Overdrive", 3, b"Overdrive: raises the damage of Sora's swings. (effect coming)\0"),
    own(0x6E, "Riku", "Sleight Lock", 3, b"Sleight Lock: sleights cost less MP. (effect coming)\0"),
    own(0x6F, "Axel", "Quick Recovery", 3, b"Quick Recovery: Sora recovers from hits sooner. (effect coming)\0"),
    own(0x70, "Larxene", "Dash", 2, b"Dash: Sora runs faster. (effect coming)\0"),
    own(0x71, "Vexen", "Auto-Life", 4, b"Auto-Life: revive once a battle after a knockout. (effect coming)\0"),
    own(0x72, "Marluxia", "Double Sleight", 5, b"Double Sleight: a sleight plays twice. (effect coming)\0"),
    own(0x73, "Lexaeus", "Warp Break", 5, b"Warp Break: sleights may warp enemies away. (effect coming)\0"),
    own(0x74, "Ansem", "Sleightblind", 3, b"Sleightblind: hides sleights from the enemy. (effect coming)\0"),
    own(0x75, "Zexion", "Confu-Strike", 3, b"Confu-Strike: Sora's swings may confuse enemies. (effect coming)\0"),
    own(0x76, "Xemnas", "Quick Barrier", 4, b"Quick Barrier: a short barrier after taking a hit. (effect coming)\0"),
    own(0x77, "Xigbar", "Shot Charge", 3, b"Shot Charge: raises the damage of shots. (effect coming)\0"),
    own(0x78, "Xaldin", "Aero Guard", 3, b"Aero Guard: a wind circles Sora, striking foes. (effect coming)\0"),
    own(0x79, "Saix", "Combo Boost", 3, b"Combo Boost: each combo swing hits harder. (effect coming)\0"),
    own(0x7A, "Demyx", "Water Charge", 3, b"Water Charge: raises the damage of water attacks. (effect coming)\0"),
    own(0x7B, "Luxord", "Omni Break", 4, b"Omni Break: Sora's swings always win clashes. (effect coming)\0"),
    own(0x7C, "Roxas", "Double Strike", 7, b"Double Strike: each swing strikes twice. (effect coming)\0"),
    own(0x7D, "Gold Card", "Premium Guard", 5, b"Premium Guard: sleights keep their premium cards. (effect coming)\0"),
    own(0x7E, "Platinum Card", "Invincible", 5, b"Invincible: Sora takes no damage. (effect coming)\0"),
];

/// The enemy cards with no skill (owner, 2026-11-03 "1. C" and 2026-11-04): Shadow, Powerwild, Sea
/// Neon, Screwdiver, Pirate (card values), Darkball (Duel Trigger, Riku's story only), Wyvern (Reload
/// Lock), White Mushroom (Hyper Healing, needs the party), Trickmaster (Value Break), Parasite Cage
/// (Dispel), Darkside (Mimic).
pub const LEFT_OUT: [u16; 11] = [0x44, 0x4B, 0x52, 0x53, 0x57, 0x59, 0x5B, 0x5E, 0x65, 0x64, 0x66];

/// How many skills are [`Kind::Own`] (their abilities follow `crate::abilities::FIRST_SKILL`).
pub const OWN_COUNT: usize = {
    let mut n = 0;
    let mut k = 0;
    while k < SKILLS.len() {
        if matches!(SKILLS[k].kind, Kind::Own) {
            n += 1;
        }
        k += 1;
    }
    n
};

/// The skill of an enemy card kind.
pub fn of_card(card: u16) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.card == card)
}

/// The `crate::abilities` index of a skill: a match's KH1 ability, or its own (in table order from
/// `crate::abilities::FIRST_SKILL`).
pub fn ability_of(s: &Skill) -> usize {
    match s.kind {
        Kind::Kh1(i) => i,
        Kind::Own => {
            let k = SKILLS.iter().take_while(|t| t.card != s.card).filter(|t| t.kind == Kind::Own).count();
            crate::abilities::FIRST_SKILL + k
        }
    }
}

/// The skill whose ability is `i` (`crate::abilities::ix`).
pub fn of_ability(i: usize) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| ability_of(s) == i)
}

/// The `k`th [`Kind::Own`] skill (const: `crate::abilities::ALL` is built from it).
pub const fn own_at(k: usize) -> &'static Skill {
    let mut n = 0;
    let mut j = 0;
    while j < SKILLS.len() {
        if matches!(SKILLS[j].kind, Kind::Own) {
            if n == k {
                return &SKILLS[j];
            }
            n += 1;
        }
        j += 1;
    }
    panic!("no such skill")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{self as ab, Source, ALL};

    /// The owner's AP table (2026-11-04) and KH1's for the matches (btltbl `0x33D8`).
    #[test]
    fn ap_is_the_owners_table() {
        let want: &[(&str, u8)] = &[
            ("Red Nocturne", 2),
            ("Blue Rhapsody", 2),
            ("Yellow Opera", 2),
            ("Green Requiem", 2),
            ("Demyx", 3),
            ("Wizard", 3),
            ("Crescendo", 3),
            ("Xigbar", 3),
            ("Defender", 3),
            ("Ursula", 3),
            ("Ansem", 3),
            ("Large Body", 4),
            ("Fat Bandit", 2),
            ("Saix", 3),
            ("Guard Armor", 2),
            ("Search Ghost", 3),
            ("Neoshadow", 3),
            ("Oogie Boogie", 4),
            ("Vexen", 4),
            ("Larxene", 2),
            ("Wight Knight", 2),
            ("Gargoyle", 3),
            ("Zexion", 3),
            ("Axel", 3),
            ("Xemnas", 4),
            ("Xaldin", 3),
            ("Dragon Maleficent", 3),
            ("Lexaeus", 5),
            ("Bandit", 7),
            ("Roxas", 7),
            ("Card Soldier", 4),
            ("Gold Card", 5),
            ("Platinum Card", 5),
            // The deck-to-KH1 group.
            ("Tornado Step", 3),
            ("Aquatank", 3),
            ("Barrel Spider", 4),
            ("Air Soldier", 2),
            ("Jafar", 4),
            ("Air Pirate", 2),
            ("Riku", 3),
            ("Marluxia", 5),
            ("Luxord", 4),
            ("Black Fungus", 2),
            // The five KH1 matches: KH1's AP (Combo Plus 1, Second Chance 5, Berserk 1, Leaf Bracer
            // 5, Treasure Magnet 2).
            ("Soldier", 1),
            ("Hook", 5),
            ("Hades", 1),
            ("Creeper Plant", 5),
            ("Bouncywild", 2),
        ];
        assert_eq!(want.len(), SKILLS.len());
        for (name, ap) in want {
            let s = SKILLS.iter().find(|s| s.name == *name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(s.ap, *ap, "{name}");
            assert_eq!(ALL[ability_of(s)].ap, *ap, "{name}: the ability's AP");
        }
    }

    /// One skill per card, in card order; ids are Re:CoM's enemy cards; every enemy card is a skill or
    /// left out (owner).
    #[test]
    fn cards_and_left_out() {
        assert!(SKILLS.windows(2).all(|w| w[0].card < w[1].card));
        let mut all: Vec<u16> = SKILLS.iter().map(|s| s.card).chain(LEFT_OUT).collect();
        all.sort_unstable();
        assert_eq!(all, (0x44..=0x7E).collect::<Vec<u16>>());
        assert_eq!(OWN_COUNT, 43);
        assert_eq!(SKILLS.iter().filter(|s| matches!(s.kind, Kind::Kh1(_))).count(), 5);
        for c in LEFT_OUT {
            assert!(of_card(c).is_none());
        }
    }

    /// The names as Re:CoM's card-info table has them (`0x1405256D0`, read from the exe: Checked
    /// (exe), `docs/recom-enemy-skills.md` § Stage 1), ASCII; Saïx as "Saix".
    #[test]
    fn names_are_the_cards() {
        let names: Vec<(u16, &str)> = SKILLS.iter().map(|s| (s.card, s.name)).collect();
        for (c, n) in [(0x45, "Soldier"), (0x4C, "Bouncywild"), (0x60, "Creeper Plant"), (0x68, "Hades"), (0x6C, "Hook"), (0x6D, "Dragon Maleficent"), (0x79, "Saix"), (0x7D, "Gold Card"), (0x7E, "Platinum Card")] {
            assert!(names.contains(&(c, n)), "{c:#x} {n}");
        }
        for s in &SKILLS {
            assert!(s.name.bytes().all(|b| (0x20..0x7F).contains(&b) && b != b'%'));
            assert!(s.name.len() <= crate::notices::NAME_MAX);
        }
    }

    /// The five KH1 matches are KH1's abilities, learned from the card (owner); the rest their own.
    #[test]
    fn the_five_matches() {
        let m = |c: u16| of_card(c).unwrap();
        assert_eq!(m(0x45).kind, Kind::Kh1(ix::COMBO_PLUS[0]), "Soldier: the Rod level-87 Combo Plus");
        assert_eq!(m(0x6C).kind, Kind::Kh1(ix::SECOND_CHANCE));
        assert_eq!(m(0x68).kind, Kind::Kh1(ix::BERSERK));
        assert_eq!(m(0x60).kind, Kind::Kh1(ix::LEAF_BRACER));
        assert_eq!(m(0x4C).kind, Kind::Kh1(ix::TREASURE_MAGNET[0]), "Bouncywild: KH1's level-9 Treasure Magnet");
        for s in SKILLS.iter() {
            let i = ability_of(s);
            assert_eq!(ALL[i].source, Source::Card(s.card), "{}", s.name);
            assert_eq!(ALL[i].name, s.name);
            assert_eq!(of_ability(i), Some(s));
            assert_eq!(ab::skill_of(i), Some(s));
        }
        // The other copies stay by level: Combo Plus 93 / 99, Treasure Magnet 72.
        assert_eq!([ALL[ix::COMBO_PLUS[1]].source, ALL[ix::COMBO_PLUS[2]].source], [Source::Level(93), Source::Level(99)]);
        assert_eq!(ALL[ix::TREASURE_MAGNET[1]].source, Source::Level(72));
        // The four built matches act; Treasure Magnet and every own skill wait.
        let built: Vec<&str> = SKILLS.iter().filter(|s| s.built).map(|s| s.name).collect();
        assert_eq!(built, vec!["Soldier", "Creeper Plant", "Hades", "Hook"]);
    }

    /// Help lines: ASCII, within the help box, unique; an unbuilt skill says "(effect coming)",
    /// Treasure Magnet "(not built yet)"; each names its effect first.
    #[test]
    fn help_lines() {
        let mut seen = std::collections::HashSet::new();
        for s in &SKILLS {
            let h = s.help;
            assert_eq!(h.last(), Some(&0), "{}", s.name);
            let t = std::str::from_utf8(&h[..h.len() - 1]).unwrap();
            assert!(t.bytes().all(|b| (0x20..0x7F).contains(&b) && b != b'%'), "{t}");
            assert!(t.len() <= crate::menu::HELP_MAX, "{} ({}): {t}", s.name, t.len());
            assert!(seen.insert(t), "{t}");
            if s.built {
                assert!(!t.contains("(effect coming)") && !t.contains("(not built yet)"), "{t}");
            } else if s.card == 0x4C {
                assert!(t.ends_with(" (not built yet)"), "{t}");
            } else {
                assert!(t.ends_with(" (effect coming)"), "{t}");
            }
            let effect = match s.card {
                0x45 => "Combo Plus",
                0x4C => "Treasure Magnet",
                _ => s.effect,
            };
            assert!(t.starts_with(&format!("{effect}: ")), "{t}");
        }
    }
}
