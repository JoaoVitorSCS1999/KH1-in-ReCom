//! M12 step 5: the command menu's Sleights row and list (owner, 2026-11-03: "If you press left on
//! the d-pad, instead of summons at the bottom of the command menu, it should say "sleights" In the
//! sleights menu I only want ACTIVELY EQUIPPED sleights to show in the list"; answers: "D-pad Left
//! toggles Summon / Sleights anywhere on the command menu (drops KH1's Left → Attack)", "Cost: MP by
//! tier"). The testable part; the menu's rules are in `crate::command_menu`, the start in
//! `win/summons.rs`, notes `docs/recom-m12-sleights.md` § 8.5.
//!
//! - **The bottom row**: KH1 FM's 4th root row shows Summon (`crate::command_menu`); D-pad Left at
//!   the root switches it to "Sleights" and back ([`Bottom`]). The choice is kept per save in the
//!   summon block's spare byte ([`crate::summons::field::BOTTOM_ROW`]).
//! - **The list**: the equipped sleight abilities (`crate::abilities::equipped_sleights`), in
//!   Re:CoM's id order, as commands [`command`] (ours: KH1's command table has none), named by our
//!   words [`word`] (the abilities' names, English in every language, as Cloud's).
//! - **Usable** ([`usable`]): the summons' rule (`crate::summons::usable`, KH1's `0x140288F30`
//!   without its once-per-battle test): Re:CoM's battle room, KH1's ground state (flag 0x80), no
//!   sleight running, and the owner's MP ([`Cost`]).
//! - **The start**: Re:CoM's own sleight start for the id (the summons' path, Sora's vtable `+0x130`
//!   = `0x1402247A0`); Re:CoM runs its motion, effects, hits, damage, protection and end.

use crate::abilities::{self as ab, SleightMp};

/// How many sleight abilities there are (and so the list's most rows).
pub const MAX: usize = ab::SLEIGHTS.end - ab::SLEIGHTS.start;

/// Our first sleight command: `0x80 + id` (ids 0x1B..=0x43 → 0x9B..=0xC3). KH1's command table
/// ends below 0x80 and our other commands sit elsewhere (Stagger 0xF0). Checked (code).
pub const COMMAND_BASE: u8 = 0x80;
/// Our words for the sleights' names: `0x8100 + id` (0x8000 = ours, as Cloud's 0x8029). The asset
/// step's `command_menu::SLEIGHT_WORDS` holds the same ids and names.
pub const WORD_BASE: u16 = 0x8100;
/// Our word for the row: "Sleights" (the owner's word; KH1 has none, as "Stagger" 901).
pub const ROW_WORD: u16 = 902;
pub const ROW_TEXT: &str = "Sleights";

/// What the root's 4th row shows (owner: D-pad Left switches it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Bottom {
    /// KH1's Summon (the default, and every save before M12 step 5).
    #[default]
    Summon,
    Sleights,
}

impl Bottom {
    /// The save byte (0 Summon, 1 Sleights; anything else reads as Summon).
    pub fn of_byte(b: u8) -> Bottom {
        if b == 1 {
            Bottom::Sleights
        } else {
            Bottom::Summon
        }
    }
    pub fn byte(self) -> u8 {
        match self {
            Bottom::Summon => 0,
            Bottom::Sleights => 1,
        }
    }
    pub fn other(self) -> Bottom {
        match self {
            Bottom::Summon => Bottom::Sleights,
            Bottom::Sleights => Bottom::Summon,
        }
    }
}

/// The sleight ability of a Re:CoM id (one of the 27).
fn ability(id: u16) -> Option<usize> {
    ab::of_sleight(id)
}

/// The command of sleight `id`.
pub fn command(id: u16) -> u8 {
    COMMAND_BASE.wrapping_add(id as u8)
}

/// The sleight of a command (only the 27 sleight abilities).
pub fn of_command(c: u8) -> Option<u16> {
    let id = c.checked_sub(COMMAND_BASE)? as u16;
    ability(id).map(|_| id)
}

/// The word of sleight `id`.
pub fn word(id: u16) -> u16 {
    WORD_BASE + id
}

/// Every sleight's word and name (the abilities' names), for the menu's word list.
pub fn words() -> impl Iterator<Item = (u16, &'static str)> {
    ab::SLEIGHTS.filter_map(|i| ab::sleight_of(i).map(|id| (word(id), ab::ALL[i].name)))
}

/// The sleight's name (the Abilities page's).
pub fn name(id: u16) -> &'static str {
    ability(id).map_or("?", |i| ab::ALL[i].name)
}

/// What sleight `id` costs (the owner's MP by tier).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cost {
    /// Whole MP points.
    Points(i32),
    /// All of Sora's whole MP; usable with at least 1 (owner: "cost all MP like KH1 summons", and the
    /// summons are "usable as long as you have MP").
    All,
}

pub fn cost(id: u16) -> Option<Cost> {
    Some(match ab::sleight_mp(ability(id)?)? {
        SleightMp::Points(n) => Cost::Points(n as i32),
        SleightMp::All => Cost::All,
    })
}

/// The whole MP points sleight `id` takes from `mp` (None: it can't pay).
pub fn pay_points(id: u16, mp: i32) -> Option<i32> {
    match cost(id)? {
        Cost::Points(n) if mp >= n => Some(n),
        Cost::All if mp >= 1 => Some(mp),
        _ => None,
    }
}

/// KH1's usable test for sleight `id` (the summons' gate; `crate::summons::Gate`'s `out` = any
/// Re:CoM sleight running).
pub fn usable(g: &crate::summons::Gate, id: u16) -> bool {
    g.battle && g.ground && !g.out && pay_points(id, g.mp).is_some()
}

/// The list's ids from the equipped ones (Re:CoM's order; anything not a sleight ability dropped,
/// at most [`MAX`]).
pub fn list(equipped: &[u16]) -> Vec<u16> {
    let mut v: Vec<u16> = equipped.iter().copied().filter(|&id| ability(id).is_some()).collect();
    v.sort_unstable();
    v.dedup();
    v.truncate(MAX);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::sleight as s;
    use crate::summons::Gate;

    #[test]
    fn commands_and_words_round_trip_for_the_27() {
        assert_eq!(MAX, 27);
        let ids: Vec<u16> = ab::SLEIGHTS.filter_map(ab::sleight_of).collect();
        for &id in &ids {
            assert_eq!(of_command(command(id)), Some(id));
            assert!(command(id) >= 0x9B && command(id) <= 0xC3);
            assert_ne!(command(id), crate::gimmick::STAGGER);
            assert!(crate::summons::of_command(command(id)).is_none());
            assert!(crate::command_menu::cmd::spell_of(command(id)).is_none());
        }
        // Not ours: the summons' and the KH1-twin sleights.
        assert_eq!(of_command(command(s::SONIC_BLADE)), None);
        assert_eq!(of_command(0x7F), None);
        assert_eq!(of_command(0x35), None);
        assert_eq!(words().count(), 27);
        assert!(words().any(|w| w == (word(s::LETHAL_FRAME), "Lethal Frame")));
        assert!(words().any(|w| w == (word(s::CROSS_SLASH_PLUS), "Cross-slash+")));
        assert_eq!(name(s::FREEZE), "Freeze");
    }

    #[test]
    fn mp_by_tier_and_all_mp() {
        // Owner: AP 2 → 1 MP, 3 → 2, 4 → 2, 5 → 3; Idyll Romp / Cross-slash+ all MP.
        assert_eq!(cost(0x1F), Some(Cost::Points(1)), "Judgment, 2 AP");
        assert_eq!(cost(s::FIRE_RAID), Some(Cost::Points(2)), "3 AP");
        assert_eq!(cost(0x1D), Some(Cost::Points(2)), "Thunder Raid, 4 AP");
        assert_eq!(cost(s::HOLY), Some(Cost::Points(3)), "5 AP");
        assert_eq!(cost(s::IDYLL_ROMP), Some(Cost::All));
        assert_eq!(cost(s::CROSS_SLASH_PLUS), Some(Cost::All));
        assert_eq!(cost(s::SONIC_BLADE), None);
        assert_eq!(pay_points(s::HOLY, 2), None);
        assert_eq!(pay_points(s::HOLY, 3), Some(3));
        assert_eq!(pay_points(s::IDYLL_ROMP, 0), None);
        assert_eq!(pay_points(s::IDYLL_ROMP, 1), Some(1));
        assert_eq!(pay_points(s::CROSS_SLASH_PLUS, 7), Some(7));
    }

    #[test]
    fn usable_is_the_summons_gate_with_the_sleights_mp() {
        let g = Gate { battle: true, ground: true, mp: 2, out: false };
        assert!(usable(&g, s::FIRE_RAID));
        assert!(!usable(&g, s::HOLY));
        assert!(usable(&g, s::IDYLL_ROMP));
        assert!(!usable(&Gate { battle: false, ..g }, s::FIRE_RAID), "battle rooms only");
        assert!(!usable(&Gate { ground: false, ..g }, s::FIRE_RAID), "on the ground");
        assert!(!usable(&Gate { out: true, ..g }, s::FIRE_RAID), "one sleight at a time");
        assert!(!usable(&Gate { mp: 0, ..g }, s::IDYLL_ROMP));
    }

    #[test]
    fn list_and_bottom_byte() {
        assert_eq!(list(&[s::HOLY, s::FIRE_RAID, s::SONIC_BLADE, s::FIRE_RAID]), vec![s::FIRE_RAID, s::HOLY]);
        assert_eq!(Bottom::of_byte(0), Bottom::Summon);
        assert_eq!(Bottom::of_byte(1), Bottom::Sleights);
        assert_eq!(Bottom::of_byte(7), Bottom::Summon);
        assert_eq!(Bottom::Sleights.byte(), 1);
        assert_eq!(Bottom::Summon.other(), Bottom::Sleights);
    }
}
