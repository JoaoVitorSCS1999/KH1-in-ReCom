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
//!   sleight running, and the owner's MP ([`Cost`]). **Mid-air** too for Re:CoM's air-start
//!   sleights ([`AIR_START`], owner 2026-11-03, "Sleights menu follow-ups"), in a KH1 air state that
//!   lets a command start.
//! - **The start**: Re:CoM's own sleight start for the id (the summons' path, Sora's vtable `+0x130`
//!   = `0x1402247A0`); Re:CoM runs its motion, effects, hits, damage, protection and end.
//! - **Damage = KH1** ([`kh1_scaled`], owner, "Sleights menu follow-ups"): each hit Re:CoM's × the
//!   enemy's KH1 max HP / its Re:CoM max HP (twins only; `win::summons::scale_sleight_hit`).
//! - **KH1's specials** (M12 step 7, owner 2026-11-03: "Strike raid are arcanum and ragnarok in KH1FM
//!   are triangle prompts. Let them be triangle prompts and also show up in the sleights menu"):
//!   [`SPECIALS`] when equipped, under Re:CoM's sleight id of the same name (Strike Raid 8), named by
//!   KH1's own command word (338), usable when the special's own prompt would offer it (its
//!   `0x14028D0F0` score 2: a live target in range, its MP); confirm issues the special's command
//!   exactly as its prompt's Triangle does (`crate::specials`), not Re:CoM's sleight.

use crate::abilities::{self as ab, SleightMp};

/// The KH1 specials (by `crate::specials` index) the list also holds when equipped (M12 step 7):
/// Strike Raid, Ars Arcanum. Sonic Blade isn't (the owner named the other three).
pub const SPECIALS: [usize; 3] = [crate::specials::ARS_ARCANUM, crate::specials::STRIKE_RAID, crate::specials::RAGNAROK];

/// The list's most rows: the sleight abilities and [`SPECIALS`].
pub const MAX: usize = ab::SLEIGHTS.end - ab::SLEIGHTS.start + SPECIALS.len();

/// The Re:CoM sleight id a listed special stands under (Strike Raid 8): its ability's source.
pub fn special_sleight(i: usize) -> Option<u16> {
    if !SPECIALS.contains(&i) {
        return None;
    }
    match ab::ALL.get(ab::special_ability(i)?)?.source {
        ab::Source::Sleight(id) => Some(id),
        _ => None,
    }
}

/// The listed special of a Re:CoM sleight id.
pub fn special_index(id: u16) -> Option<usize> {
    SPECIALS.iter().copied().find(|&i| special_sleight(i) == Some(id))
}

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

/// The sleight of a command (the 27 sleight abilities and the listed specials).
pub fn of_command(c: u8) -> Option<u16> {
    let id = c.checked_sub(COMMAND_BASE)? as u16;
    (ability(id).is_some() || special_index(id).is_some()).then_some(id)
}

/// The word of sleight `id` (a listed special: KH1's own word of its command).
pub fn word(id: u16) -> u16 {
    match special_index(id).and_then(|i| crate::specials::cmd::name(crate::specials::TABLE[i].1)) {
        Some(w) => w,
        None => WORD_BASE + id,
    }
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

/// Re:CoM's sleights of the 27 that start in the air (owner, 2026-11-03, "Sleights menu
/// follow-ups": "Re:CoM's air-start sleights (Homing Blizzara, Warpinator, Warp, Bind, Confuse,
/// Terror, Teleport) usable mid-air"): the ids whose case in `initStockSkill` `0x140220DF0` asks
/// Re:CoM's airborne test `0x1401F3C00` and, in the air, takes its air version (`player+0x418 |=
/// 0x800000`, `+0x478` = 4 (the air clips; Teleport −36), which `initStockSkill`'s end turns into the
/// low-gravity bit `0x2000000`). Of Sora's other ids only the -ra / -ga tiers (0xD..=0x1A) do so; no
/// other of the 27 has an air case. Checked (decomp).
pub const AIR_START: [u16; 7] = [0x24, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x32];

/// Sleight `id` may start mid-air ([`AIR_START`]).
pub fn air_start(id: u16) -> bool {
    AIR_START.contains(&id)
}

/// KH1's usable test for sleight `id` (the summons' gate; `crate::summons::Gate`'s `out` = any
/// Re:CoM sleight running): on the ground, or (an [`AIR_START`] sleight) in a KH1 air state that
/// lets a command start (`Gate::air`); Re:CoM's start then picks its air version itself.
pub fn usable(g: &crate::summons::Gate, id: u16) -> bool {
    g.battle && (g.ground || (g.air && air_start(id))) && !g.out && pay_points(id, g.mp).is_some()
}

/// A listed special's usable test (M12 step 7): battle, on the ground, nothing running, and its own
/// prompt's offer (`offered`: the bit of its `crate::specials` index, `0x14028D0F0`'s score 2).
pub fn special_usable(g: &crate::summons::Gate, id: u16, offered: u8) -> bool {
    special_index(id).is_some_and(|i| g.battle && g.ground && !g.out && offered & (1 << i) != 0)
}

/// Re:CoM's hit kinds (the hurt body's `+0x80`) whose value isn't damage: Stop's ticks (8), Gravity's
/// percent (0xD), the timed states 0x10 / 0x17 (Shock Impact's, Lethal Frame's) and Magnet's 0x14
/// (their value goes to `+0x2D8` or is a share of HP, `0x14017DC50`). Re:CoM's own mask for them,
/// the hit handler's `0x912100` (`0x14017F7E0`: kinds above 0x17 or with their bit clear take its
/// frozen bonus). Checked (decomp).
pub const NOT_DAMAGE_KINDS: u32 = 0x0091_2100;

/// Whether a hit of Re:CoM kind `kind` carries damage (Re:CoM's rule, [`NOT_DAMAGE_KINDS`]).
pub fn is_damage_kind(kind: u32) -> bool {
    kind > 0x17 || (NOT_DAMAGE_KINDS >> kind) & 1 == 0
}

/// Damage = KH1 (owner, 2026-11-03, "Sleights menu follow-ups": "at Damage = KH1 sleight damage
/// scaled by the enemy's KH1 max HP / its Re:CoM max HP"): a sleight hit's value `value` (Re:CoM's,
/// on the hurt body `+0x92`, before Re:CoM's resistance, weak part and boss falloff, which then apply
/// to it as to any hit) on an enemy whose KH1 max HP is `kh1_max` and Re:CoM max HP `recom_max`:
/// value × kh1_max / recom_max, rounded, at least 1. None (Re:CoM's value stands): not a damage kind,
/// nothing to scale, or no HP to scale by.
pub fn kh1_scaled(value: i16, kind: u32, kh1_max: i32, recom_max: i32) -> Option<i16> {
    if !is_damage_kind(kind) || value <= 0 || kh1_max <= 0 || recom_max <= 0 {
        return None;
    }
    let v = (value as i64 * kh1_max as i64 * 2 + recom_max as i64) / (recom_max as i64 * 2);
    Some(v.clamp(1, i16::MAX as i64) as i16)
}

/// The list's ids from the equipped ones (Re:CoM's order; anything not a sleight ability dropped,
/// at most [`MAX`]).
pub fn list(equipped: &[u16]) -> Vec<u16> {
    let mut v: Vec<u16> = equipped.iter().copied().filter(|&id| ability(id).is_some() || special_index(id).is_some()).collect();
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
        assert_eq!(MAX, 27 + SPECIALS.len());
        let ids: Vec<u16> = ab::SLEIGHTS.filter_map(ab::sleight_of).collect();
        for &id in &ids {
            assert_eq!(of_command(command(id)), Some(id));
            assert!(command(id) >= 0x9B && command(id) <= 0xC3);
            assert_ne!(command(id), crate::gimmick::STAGGER);
            assert!(crate::summons::of_command(command(id)).is_none());
            assert!(crate::command_menu::cmd::spell_of(command(id)).is_none());
        }
        // Not ours: the summons' and Sonic Blade (the owner listed the other three).
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
        let g = Gate { battle: true, ground: true, air: false, mp: 2, out: false };
        assert!(usable(&g, s::FIRE_RAID));
        assert!(!usable(&g, s::HOLY));
        assert!(usable(&g, s::IDYLL_ROMP));
        assert!(!usable(&Gate { battle: false, ..g }, s::FIRE_RAID), "battle rooms only");
        assert!(!usable(&Gate { ground: false, ..g }, s::FIRE_RAID), "on the ground");
        assert!(!usable(&Gate { out: true, ..g }, s::FIRE_RAID), "one sleight at a time");
        assert!(!usable(&Gate { mp: 0, ..g }, s::IDYLL_ROMP));
    }

    /// Owner (2026-11-03, "Sleights menu follow-ups"): Re:CoM's air-start sleights usable mid-air,
    /// the rest ground-only; exactly the ids `initStockSkill` gives an air version.
    #[test]
    fn air_start_sleights_are_recoms_seven() {
        let names: Vec<&str> = AIR_START.iter().map(|&id| name(id)).collect();
        assert_eq!(names, ["Homing Blizzara", "Warpinator", "Warp", "Bind", "Confuse", "Terror", "Teleport"]);
        let air = Gate { battle: true, ground: false, air: true, mp: 3, out: false };
        for id in ab::SLEIGHTS.filter_map(ab::sleight_of) {
            assert_eq!(usable(&air, id), air_start(id), "{}", name(id));
            // On the ground every one is usable as before.
            assert!(usable(&Gate { ground: true, air: false, ..air }, id), "{}", name(id));
        }
        // Mid-air still needs a battle room, nothing running and the MP; neither ground nor a KH1 air
        // state that takes a command (e.g. a reaction): none.
        assert!(!usable(&Gate { battle: false, ..air }, 0x24));
        assert!(!usable(&Gate { out: true, ..air }, 0x2C));
        assert!(!usable(&Gate { mp: 0, ..air }, 0x32));
        assert!(!usable(&Gate { air: false, ..air }, 0x24));
        // Not ours: the tiers (also air in Re:CoM) aren't sleight abilities.
        assert!(!air_start(0xD) && !air_start(s::FIRE_RAID) && !air_start(s::IDYLL_ROMP));
    }

    /// Owner (2026-11-03, "Sleights menu follow-ups"): at Damage = KH1 a sleight's damage is Re:CoM's
    /// × (the enemy's KH1 max HP / its Re:CoM max HP); values that aren't damage stay Re:CoM's.
    #[test]
    fn kh1_damage_scales_by_the_enemys_max_hp_ratio() {
        // A Shadow-like twin: KH1 max HP 24 against Re:CoM's 160: × 0.15.
        assert_eq!(kh1_scaled(500, 1, 24, 160), Some(75));
        // Rounded to the nearest (1000 × 37 / 300 = 123.3; 50 × 3 / 4 = 37.5 → 38).
        assert_eq!(kh1_scaled(1000, 1, 37, 300), Some(123));
        assert_eq!(kh1_scaled(50, 3, 3, 4), Some(38));
        // At least 1 (Re:CoM's own floor for a damaging hit); capped at the carrier.
        assert_eq!(kh1_scaled(3, 2, 1, 1000), Some(1));
        assert_eq!(kh1_scaled(i16::MAX, 1, 10, 1), Some(i16::MAX));
        // A KH1 HP above Re:CoM's scales up (the same formula).
        assert_eq!(kh1_scaled(100, 1, 300, 200), Some(150));
        // Not damage (Re:CoM's mask): Stop ticks 8 (Teleport), Gravity % 0xD, timed states 0x10 / 0x17,
        // Magnet 0x14.
        for k in [8u32, 0xD, 0x10, 0x14, 0x17] {
            assert!(!is_damage_kind(k));
            assert_eq!(kh1_scaled(9000, k, 24, 160), None, "kind {k:#x}");
        }
        for k in [0u32, 1, 2, 3, 7, 9, 0xA, 0xB, 0xC, 0xE, 0xF, 0x11, 0x12, 0x13, 0x15, 0x16, 0x1D, 0x1E] {
            assert!(is_damage_kind(k), "kind {k:#x}");
        }
        // Nothing to scale.
        assert_eq!(kh1_scaled(0, 1, 24, 160), None);
        assert_eq!(kh1_scaled(500, 1, 0, 160), None);
        assert_eq!(kh1_scaled(500, 1, 24, 0), None);
    }

    /// M12 step 7 (owner: "also show up in the sleights menu"): Strike Raid in the list under Re:CoM's
    /// sleight 8 (before the magic sleights, Re:CoM's order), KH1's word 338, usable as its prompt.
    #[test]
    fn strike_raid_is_listed_as_its_prompt() {
        assert_eq!(special_sleight(crate::specials::STRIKE_RAID), Some(s::STRIKE_RAID));
        assert_eq!(special_sleight(crate::specials::SONIC_BLADE), None);
        assert_eq!(special_index(s::STRIKE_RAID), Some(crate::specials::STRIKE_RAID));
        assert_eq!(of_command(command(s::STRIKE_RAID)), Some(s::STRIKE_RAID));
        assert_eq!(command(s::STRIKE_RAID), 0x88);
        assert_eq!(word(s::STRIKE_RAID), 338);
        assert_eq!(cost(s::STRIKE_RAID), None, "its MP is KH1's (the special's own)");
        assert_eq!(list(&[s::HOLY, s::STRIKE_RAID, s::FIRE_RAID]), vec![s::STRIKE_RAID, s::FIRE_RAID, s::HOLY]);
        let g = Gate { battle: true, ground: true, air: false, mp: 0, out: false };
        let bit = 1 << crate::specials::STRIKE_RAID;
        assert!(special_usable(&g, s::STRIKE_RAID, bit));
        assert!(!special_usable(&g, s::STRIKE_RAID, 0), "its prompt wouldn't offer it");
        assert!(!special_usable(&Gate { ground: false, air: true, ..g }, s::STRIKE_RAID, bit), "ground only (KH1's state 0)");
        assert!(!special_usable(&Gate { out: true, ..g }, s::STRIKE_RAID, bit));
        assert!(!special_usable(&Gate { battle: false, ..g }, s::STRIKE_RAID, bit));
        assert!(!usable(&g, s::STRIKE_RAID), "not by the sleights' MP rule");
        // Ars Arcanum the same way: Re:CoM's sleight 10, KH1's word 331 ("Arcanum").
        assert_eq!(special_sleight(crate::specials::ARS_ARCANUM), Some(s::ARS_ARCANUM));
        assert_eq!((command(s::ARS_ARCANUM), word(s::ARS_ARCANUM)), (0x8A, 331));
        assert_eq!(special_sleight(crate::specials::RAGNAROK), Some(s::RAGNAROK));
        assert_eq!(special_index(s::RAGNAROK), Some(crate::specials::RAGNAROK));
        assert_eq!((command(s::RAGNAROK), word(s::RAGNAROK)), (0x8B, 334));
        assert_eq!(list(&[s::FIRE_RAID, s::ARS_ARCANUM, s::STRIKE_RAID]), vec![s::STRIKE_RAID, s::ARS_ARCANUM, s::FIRE_RAID]);
        assert!(special_usable(&g, s::ARS_ARCANUM, 1 << crate::specials::ARS_ARCANUM) && !special_usable(&g, s::ARS_ARCANUM, bit));
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
