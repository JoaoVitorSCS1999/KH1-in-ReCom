//! M7 stage 7.13, Customize (decision 34: "Customize: one "KH1" camp entry … with a submenu:
//! Keyblades, Customize"): KH1's shortcut page, the testable part. The page itself is Re:CoM's
//! camp list (`win::keyblades`), opened from the "KH1" entry's submenu (`crate::keyblades::sub`).
//!
//! KH1 (Steam, Checked (decomp + exe); research `m7-menus-mp.md` § 1.4, kept with the decomp
//! tools):
//! - **Slot list** `0x140302950`: three rows, labels sysmsg `0xCE / 0xCF / 0xD0` (table
//!   `0x140548060`), each with its slot's spell name (`0x14028F290`: the command name
//!   `level + (spell + 2) × 3` at the spell's current level) or "none" (sysmsg `0xD4`) when the
//!   slot is empty. Confirm opens the picker for that slot; cancel leaves the page.
//! - **Picker** `0x140302740`, list filled by `0x1403041A0`: the learned spells in KH1's order
//!   (Fire … Aero, `0x140285230(0, spell)`), then "none" as the last row (at most 8 rows). It opens
//!   on the row its cursor was last on (`0x142E9A040`, one value for every slot). Confirm writes the
//!   slot (`0x1402857C0(slot, spell or −1)`, no duplicate check) and goes back to the slot list;
//!   cancel goes back without writing.
//!
//! Ours: the slot bytes of our save block (`crate::mp::off::SHORTCUTS`, 0 = none, else spell + 1,
//! stage 7.12), which the command menu reads every frame, so a change shows in the L1 list at once.
//! KH1's sysmsg file is not in the asset step: the labels and "None" are our English words
//! (Assumption: KH1's own words for 0xCE..0xD0 / 0xD4 not read); the spell names are KH1's words
//! from `kh1-command-menu.json`, as the command menu shows them.

use crate::command_menu::{cmd, shortcut as sc};

pub const SLOTS: usize = 3;
pub const SPELLS: usize = 7;

/// A picker row: a spell (0..6), or None for KH1's last row, "none".
pub type Pick = Option<u8>;

/// KH1's picker list (`0x1403041A0`): the learned spells (level 1..3) in KH1's order, then "none".
pub fn picks(levels: [u8; SPELLS]) -> Vec<Pick> {
    let mut v: Vec<Pick> = (0..SPELLS as u8).filter(|&s| (1..=3).contains(&levels[s as usize])).map(Some).collect();
    v.push(None);
    v
}

/// KH1's write (`0x1402857C0`): the slot gets the pick, whatever the other slots hold (KH1 has no
/// duplicate check). Our byte: 0 = none, else spell + 1.
pub fn set(slots: [u8; SLOTS], slot: usize, pick: Pick) -> [u8; SLOTS] {
    let mut s = slots;
    if slot < SLOTS {
        s[slot] = pick.map_or(0, sc::slot_byte);
    }
    s
}

/// The row the picker opens on: KH1 keeps one picker cursor for every slot (`0x142E9A040`) and
/// opens the picker on it. Clamped to the list (the list only grows in KH1: Assumption that a
/// shorter list would clamp).
pub fn picker_cursor(last: i32, count: i32) -> i32 {
    last.clamp(0, (count - 1).max(0))
}

/// The picker row that holds the slot's spell (the None row for an empty slot), for the row mark.
pub fn row_of(picks: &[Pick], slot_byte: u8) -> Option<usize> {
    let want = sc::slot_spell(slot_byte);
    picks.iter().position(|&p| p == want)
}

/// The message id of a spell's name at its level (KH1's `0x14028F290`: the command
/// `level + (spell + 2) × 3`, named by its own id). A level outside 1..=3 reads as level 1.
pub fn name_id(spell: u8, level: u8) -> u16 {
    cmd::name(cmd::spell(spell, level.clamp(1, 3)))
}

/// Texts: NUL-terminated ASCII for Re:CoM's text items (no `%`: the help line's formatter treats
/// it as a code).
pub mod text {
    /// The slot labels (KH1: sysmsg 0xCE..0xD0, not read; our words).
    pub const SLOT_LABELS: [&[u8]; 3] = [b"Shortcut 1\0", b"Shortcut 2\0", b"Shortcut 3\0"];
    /// KH1's "none" (sysmsg 0xD4, not read; our word).
    pub const NONE: &[u8] = b"None\0";
    /// The slot list's help: which button casts the slot with L1 held (stage 7.12: Triangle /
    /// Square / confirm, KH1's `0x1402937F0`).
    pub const SLOT_HELP: [&[u8]; 3] = [b"Hold L1 and press Triangle to cast this spell.\0", b"Hold L1 and press Square to cast this spell.\0", b"Hold L1 and press Cross to cast this spell.\0"];
    /// Slot 3's help when Re:CoM's confirm button is Circle.
    pub const SLOT3_HELP_CIRCLE: &[u8] = b"Hold L1 and press Circle to cast this spell.\0";
    /// The picker's help.
    pub const PICK_HELP: [&[u8]; 3] = [b"Choose the spell for Shortcut 1.\0", b"Choose the spell for Shortcut 2.\0", b"Choose the spell for Shortcut 3.\0"];

    /// The slot list's help for slot `k` (0..2).
    pub fn slot_help(k: usize, confirm_is_circle: bool) -> &'static [u8] {
        match k {
            2 if confirm_is_circle => SLOT3_HELP_CIRCLE,
            k => SLOT_HELP[k.min(2)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picker_lists_learned_spells_in_kh1_order_then_none() {
        assert_eq!(picks([0; 7]), vec![None]);
        assert_eq!(picks([1, 0, 3, 0, 0, 2, 0]), vec![Some(0), Some(2), Some(5), None]);
        assert_eq!(picks([3; 7]).len(), 8, "KH1's 8 rows at most");
        // A bad level byte is not a learned spell.
        assert_eq!(picks([4, 0, 0, 0, 0, 0, 1]), vec![Some(6), None]);
    }

    #[test]
    fn the_write_is_kh1s_without_a_duplicate_check() {
        assert_eq!(set([1, 2, 3], 0, Some(1)), [2, 2, 3], "Blizzard twice: allowed, as KH1");
        assert_eq!(set([1, 2, 3], 2, None), [1, 2, 0]);
        assert_eq!(set([0, 0, 0], 1, Some(6)), [0, 7, 0]);
        assert_eq!(set([1, 2, 3], 3, Some(0)), [1, 2, 3], "no fourth slot");
    }

    #[test]
    fn the_picker_opens_on_its_last_row_and_marks_the_slots_spell() {
        assert_eq!(picker_cursor(0, 1), 0);
        assert_eq!(picker_cursor(3, 5), 3);
        assert_eq!(picker_cursor(7, 4), 3);
        assert_eq!(picker_cursor(-1, 4), 0);
        let p = picks([2, 0, 1, 0, 0, 0, 0]);
        assert_eq!(row_of(&p, 3), Some(1), "Thunder");
        assert_eq!(row_of(&p, 0), Some(2), "empty: the None row");
        assert_eq!(row_of(&p, 2), None, "Blizzard not learned");
        assert_eq!(row_of(&p, 9), Some(2), "a bad byte reads as none");
    }

    #[test]
    fn spell_names_are_the_command_menus_words_at_the_level() {
        assert_eq!(name_id(0, 1), 7, "Fire");
        assert_eq!(name_id(0, 2), 8, "Fira");
        assert_eq!(name_id(6, 3), 27, "Aeroga");
        assert_eq!(name_id(3, 0), 16, "Cure (level 0 reads as 1)");
    }

    #[test]
    fn texts_fit_the_help_box() {
        let all = [text::SLOT_LABELS[0], text::SLOT_LABELS[1], text::SLOT_LABELS[2], text::NONE, text::SLOT3_HELP_CIRCLE]
            .into_iter()
            .chain(text::SLOT_HELP)
            .chain(text::PICK_HELP);
        for s in all {
            assert_eq!(s.last(), Some(&0));
            assert!(s[..s.len() - 1].iter().all(|&b| (0x20..0x7F).contains(&b) && b != b'%'));
            assert!(s.len() - 1 <= crate::menu::HELP_MAX);
        }
        assert_eq!(text::slot_help(2, true), text::SLOT3_HELP_CIRCLE);
        assert_eq!(text::slot_help(2, false), text::SLOT_HELP[2]);
        assert_eq!(text::slot_help(0, true), text::SLOT_HELP[0]);
    }
}
