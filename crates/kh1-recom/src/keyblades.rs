//! Equipping Keyblades at the KH1 setting (decision 25): the testable part.
//!
//! Re:CoM's camp (pause) menu gets an 8th top-level entry, "KH1" (M7 stage 7.13; "Keyblades"
//! before it), below Config, drawn and run by Re:CoM's own camp code through four wrappers
//! (`win::keyblades`). It opens a submenu of Re:CoM's own entry buttons ([`sub`]): Items
//! (`crate::items_page`, whose Sora list starts with his Keyblade slot) and Customize
//! (`crate::customize`). The Keyblade slot opens the Keyblade list: the Keyblades Sora has earned,
//! the equipped one marked, the highlighted one's numbers, equip on confirm. The choice is one byte
//! of Re:CoM's own save block.
//! See `docs/recom-keyblade-equip.md`.
//!
//! Everything here is plain logic: the cursor rules (Re:CoM's, over 8 entries), the page's list
//! rules (Re:CoM's entry-4 list `0x1400EB7E0`), unlock filtering, the save byte and the stats
//! panel's strings.

use crate::attack::Keyblade;

/// Re:CoM's Keyblade card kinds (card-info table `0x1405256D0`, type 1). Checked (exe).
pub const KINGDOM_KEY: u16 = 1;
pub const SOUL_EATER: u16 = 0x12;
pub const LAST_KEYBLADE: u16 = 0x18;
/// A premium card's id is its kind | 0x100. Checked (decomp `0x140158FC0`, `0x14015C1F0`).
pub const PREMIUM: u16 = 0x100;

/// `*(u64*)0x14087B110`: Re:CoM's main save block (0x2AD4 bytes, saved, checksummed, loaded,
/// snapshotted and zeroed at new game by Re:CoM). Checked (decomp `0x14012BAE0`, `0x1401B1300`).
pub const SAVE_PTR: u64 = 0x1_4087_B110;
/// Our byte: `save + 0x2674` = card collection (`save + 0x1A74`, exe `0x140158F20`) + 0xC00, in
/// the collection's tail that nothing in the exe reads or writes. Checked (decomp + exe scan,
/// `recom-cards-save.md` § 4); that nothing reaches it through a pointer from elsewhere is an
/// Assumption, logged at the first camp open (`spare_range_report`).
pub const EQUIP_OFFSET: usize = 0x2674;
/// The spare range `save + 0x246B ..= save + 0x2817` (the collection tail after the discovery
/// bits). Checked (exe: the save's format string at `0x1405B8128`).
pub const SPARE_FIRST: usize = 0x246B;
pub const SPARE_LAST: usize = 0x2817;

/// The equipped Keyblade from the save byte: 0 (fresh or vanilla save) means the Kingdom Key;
/// anything outside 1..=0x18, Soul Eater, or a Keyblade the stats file has no numbers for also
/// gives the Kingdom Key.
pub fn equipped_card(byte: u8, has_numbers: impl Fn(u16) -> bool) -> u16 {
    let k = byte as u16;
    if (KINGDOM_KEY..=LAST_KEYBLADE).contains(&k) && k != SOUL_EATER && has_numbers(k) {
        k
    } else {
        KINGDOM_KEY
    }
}

/// The byte written when `card` is equipped: 0 for the Kingdom Key, so a save that never leaves
/// it keeps the spare range all zero, as in vanilla.
pub fn save_byte_for(card: u16) -> u8 {
    if card == KINGDOM_KEY || !(KINGDOM_KEY..=LAST_KEYBLADE).contains(&card) || card == SOUL_EATER {
        0
    } else {
        card as u8
    }
}

/// The offsets (from the save block) in the spare range that are not zero, our bytes excepted
/// (the Keyblade byte, M7's MP block `crate::mp::BLOCK_FIRST ..= BLOCK_LAST`, and M8's items
/// extension `crate::items::EXT_FIRST ..= EXT_LAST`, M9's ability block
/// `crate::abilities::BLOCK_FIRST ..= BLOCK_LAST`, M8b's summon block
/// `crate::summons::BLOCK_FIRST ..= BLOCK_LAST`). `range` is `save[SPARE_FIRST..=SPARE_LAST]`.
pub fn spare_range_nonzero(range: &[u8]) -> Vec<usize> {
    let ours = |o: usize| o == EQUIP_OFFSET || (crate::mp::BLOCK_FIRST..=crate::mp::BLOCK_LAST).contains(&o) || (crate::items::EXT_FIRST..=crate::items::EXT_LAST).contains(&o)
        || (crate::abilities::BLOCK_FIRST..=crate::abilities::BLOCK_LAST).contains(&o)
        || (crate::summons::BLOCK_FIRST..=crate::summons::BLOCK_LAST).contains(&o);
    range
        .iter()
        .enumerate()
        .filter(|(i, b)| **b != 0 && !ours(SPARE_FIRST + i))
        .map(|(i, _)| SPARE_FIRST + i)
        .collect()
}

/// The Keyblades the page lists, in card order: those with numbers in the stats file
/// (`cards`), never Soul Eater, the Kingdom Key always, the others when Re:CoM's discovery state
/// says the card (plain or premium) was ever obtained (`earned`, `0x14015B3F0`).
pub fn unlocked(cards: impl IntoIterator<Item = u16>, earned: impl Fn(u16) -> bool) -> Vec<u16> {
    let mut v: Vec<u16> = cards
        .into_iter()
        .filter(|&k| (KINGDOM_KEY..=LAST_KEYBLADE).contains(&k) && k != SOUL_EATER)
        .filter(|&k| k == KINGDOM_KEY || earned(k) || earned(k | PREMIUM))
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The camp's top list: Re:CoM's 7 entries (0..6) and ours (7).
pub mod top {
    /// Re:CoM's entry count at the top level (camp ctor `0x1400D2730`). Checked (decomp).
    pub const RECOM_ENTRIES: i32 = 7;
    /// Our entry's index.
    pub const OURS: i32 = 7;
    pub const ENTRIES: i32 = 8;

    /// One frame of the pad as the top list's input (update `0x1400D9C80`, state 0xF) reads it.
    /// Checked (disasm `0x1400DAA8B..0x1400DAD84`): Start (mask 0x800, fresh) or cancel first,
    /// then confirm, then up (repeat, mask 0x101000), then down (repeat, mask 0x204000). At the
    /// ends of the list a held repeat stops; only a fresh press wraps.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Pad {
        pub start: bool,
        pub cancel: bool,
        pub confirm: bool,
        pub up: bool,
        pub up_fresh: bool,
        pub down: bool,
        pub down_fresh: bool,
    }

    /// What Re:CoM's own state 0xF does with `count` entries: the index it passes to the cursor
    /// set `0x1400DEA10`, or None when nothing moves. (Confirm / close are not moves.)
    pub fn recom_move(cursor: i32, count: i32, p: Pad) -> Option<i32> {
        if p.start || p.cancel || p.confirm {
            return None;
        }
        if p.up {
            if cursor - 1 < 0 && !p.up_fresh {
                return None;
            }
            return Some(cursor - 1);
        }
        if p.down {
            if count <= cursor + 1 && !p.down_fresh {
                return None;
            }
            return Some(cursor + 1);
        }
        None
    }

    /// Re:CoM's cursor set wraps an index below 0 to the last entry and one past the last to 0;
    /// with our entry the list has 8 entries. Checked (decomp `0x1400DEA10`).
    pub fn wrap(idx: i32) -> i32 {
        if idx < 0 {
            ENTRIES - 1
        } else if idx >= ENTRIES {
            0
        } else {
            idx
        }
    }

    /// What our update wrapper adds after Re:CoM's state 0xF has run (it always runs).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Fix {
        /// Re:CoM's own code (with the cursor-set wrapper) already did the right thing.
        None,
        /// A held "down" on entry 6: Re:CoM stops there (its last entry); with 8 entries it moves
        /// to ours, with Re:CoM's cursor set, sound 1 and glide.
        MoveToOurs,
        /// Confirm on our entry: Re:CoM skips index 7 (`cmp [camp+0xB8],6; jg` at `0x1400DAABD`);
        /// we run its confirm sequence for us.
        ConfirmOurs,
    }

    pub fn fix(cursor: i32, p: Pad) -> Fix {
        if p.start || p.cancel {
            return Fix::None;
        }
        if p.confirm {
            return if cursor == OURS { Fix::ConfirmOurs } else { Fix::None };
        }
        if p.up {
            return Fix::None;
        }
        if p.down && cursor == RECOM_ENTRIES - 1 && !p.down_fresh {
            return Fix::MoveToOurs;
        }
        Fix::None
    }

    /// The whole 8-entry rule as the game runs it: Re:CoM's state 0xF with its 7-entry count,
    /// the cursor set's wrap (ours) and our fix. Used by the tests.
    pub fn step(cursor: i32, p: Pad) -> Option<i32> {
        if let Some(i) = recom_move(cursor, RECOM_ENTRIES, p) {
            return Some(wrap(i));
        }
        match fix(cursor, p) {
            Fix::MoveToOurs => Some(OURS),
            _ => None,
        }
    }
}

/// The Keyblade page's list: Re:CoM's entry-4 list rules (`0x1400EB7E0` sub-state 2 and draw
/// mode 9 of `0x1400E6DF0`). Checked (decomp).
pub mod list {
    /// Rows on screen (`camp+0xC58[8]`).
    pub const VISIBLE: i32 = 8;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct List {
        pub cursor: i32,
        pub top: i32,
        pub count: i32,
    }

    /// One frame of input. `up` / `down` are `0x140111A70`'s repeat result; `*_fresh` is the
    /// fresh-press flag it leaves in `camp+0x94` for that direction. `page_up` / `page_down` are
    /// the raw repeat reads of pad bits 4 / 8 (`0x14029B2E0`), `*_fresh` the fresh reads
    /// (`0x14029B2F0`).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Pad {
        pub up: bool,
        pub up_fresh: bool,
        pub down: bool,
        pub down_fresh: bool,
        pub page_up: bool,
        pub page_up_fresh: bool,
        pub page_down: bool,
        pub page_down_fresh: bool,
    }

    /// The list after one frame, and whether the cursor moved (Re:CoM then plays sound 1 and
    /// redraws the rows). Re:CoM's up past the first row goes to its tab bar; this page has no
    /// tabs, so it wraps to the last row as "down" wraps to the first.
    pub fn step(l: List, p: Pad) -> (List, bool) {
        let mut n = l;
        let old = l.cursor;
        let c = l.cursor;
        if l.count <= 0 {
            return (l, false);
        }
        if p.up {
            if c - 1 < 0 {
                if !p.up_fresh {
                    return (l, false);
                }
                n.cursor = l.count - 1;
            } else {
                n.cursor = c - 1;
            }
        } else if p.down {
            if c + 1 < l.count {
                n.cursor = c + 1;
            } else if p.down_fresh {
                n.cursor = 0;
            } else {
                return (l, false);
            }
        } else if p.page_up {
            let t = c - 8;
            if t >= 0 {
                n.cursor = t;
                n.top = (l.top - 8).max(0);
            } else if t < -7 {
                if !p.page_up_fresh {
                    return (l, false);
                }
                n.cursor = l.count - 1;
            } else {
                n.cursor = 0;
            }
        } else if p.page_down {
            let t = c + 8;
            if t < l.count {
                n.cursor = t;
                n.top = l.top + 8;
                if l.count - 8 <= n.top {
                    n.top = l.count - 8;
                }
            } else if t < l.count + 7 {
                n.cursor = l.count - 1;
            } else {
                if !p.page_down_fresh {
                    return (l, false);
                }
                n.cursor = 0;
            }
        } else {
            return (l, false);
        }
        (n, n.cursor != old)
    }

    /// The top row a redraw uses (draw mode 9): the cursor is kept inside the 8 visible rows.
    pub fn top_for(l: List) -> i32 {
        if l.cursor < l.top {
            l.cursor
        } else if l.top + (VISIBLE - 1) < l.cursor {
            l.cursor - (VISIBLE - 1)
        } else {
            l.top
        }
    }

    /// The scroll bar's (position, size) fractions (`0x140112CB0` as draw mode 9 calls it:
    /// `min(1, top / n)`, `min(1, 8 / n)`, n = count or 1). Checked (disasm `0x1400E7B6F`).
    pub fn bar(l: List) -> (f32, f32) {
        let n = if l.count == 0 { 1.0 } else { l.count as f32 };
        ((l.top as f32 / n).min(1.0), (8.0 / n).min(1.0))
    }

    /// The list opened on the equipped Keyblade.
    pub fn open_on(count: i32, cursor: i32) -> List {
        let l = List { cursor: cursor.clamp(0, (count - 1).max(0)), top: 0, count };
        List { top: top_for(l), ..l }
    }
}

/// The look of a camp list row: the variant Re:CoM's `CButton` set-up `0x140112F10` takes (its
/// last argument) to pick the row's sequence from the list's table `0x1407C11F0`
/// (`seqs[variant]`, Checked (decomp)). Re:CoM's Sleights page passes each row's category
/// (`0x1400E6DF0` mode 9 reads it from the list entry's second byte, filled in mode 10 from the
/// sleight's kind: Sora's 0..3, Riku's 4..7; Checked (decomp)). What each sequence draws, from
/// `camp.lad` in `FO0100.SPR` (Checked (files)): a plate (sprites 70 / 71) tinted by its
/// animation's colour, and an icon sprite.
pub mod row_look {
    /// Sequence 0x12E: red plate (colour 0x8000003C), icon sprite 167 untinted (Re:CoM's
    /// physical sleights; the rows we drew before took this one).
    pub const RED: i32 = 0;
    /// Sequence 0x133: blue plate (0x803C1E00), icon sprite 168 (Re:CoM's magic sleights).
    pub const BLUE: i32 = 1;
    /// Sequence 0x131: green plate (0x80143200), icon sprite 278 tinted green (0x801EDC1E): the
    /// green **Mickey** symbol (owner, in game, 2026-11-01), not the item symbol. Re:CoM's item
    /// symbol is sprite 166 (sprite group 83; the Deck page's item-card tab, Checked (decomp +
    /// files)); Re:CoM has no row with it, the asset step adds one ([`ITEM`]):
    /// `docs/recom-m8-items.md` § Row looks.
    pub const GREEN: i32 = 2;
    /// The asset step's group (`crate::camp_rows`): Re:CoM's green row 0x131 with the item symbol
    /// (sprite 166). Only in the DLL's own 8-entry copy of the list table, never in Re:CoM's.
    pub const ITEM: i32 = crate::camp_rows::ITEM_VARIANT;
}

/// Texts: NUL-terminated ASCII handed to Re:CoM's text items (as the Config row's are; seen in
/// game there). No `%`: the help line's formatter `0x1402402B0` treats it as a code.
pub mod text {
    /// The camp entry (M7 stage 7.13, decision 34: "one "KH1" camp entry (our entry renamed) with a
    /// submenu"), its header title and help line.
    pub const ENTRY_LABEL: &[u8] = b"KH1\0";
    pub const TITLE: &[u8] = b"KH1\0";
    pub const ENTRY_HELP: &[u8] = b"Sora's Keyblade and items, spell shortcuts and abilities.\0";
    /// The Keyblade list's help (opened from the Items page's Keyblade slot).
    pub const PAGE_HELP: &[u8] = b"Choose the Keyblade Sora fights with.\0";
    pub const STAT_LABELS: [&[u8]; 3] = [b"Strength\0", b"Magic\0", b"Reach\0"];
    /// The Customize row (KH1's pause-menu word) and its help.
    pub const CUSTOMIZE_LABEL: &[u8] = b"Customize\0";
    pub const CUSTOMIZE_HELP: &[u8] = b"Set the spells of the three shortcuts.\0";
}

/// The "KH1" entry's submenu (M7 stage 7.13): Re:CoM's own entry buttons relabelled with these
/// rows, as Re:CoM's Deck submenu relabels them (`0x1400E3DE0` mode 0 at level 1), run with
/// Re:CoM's Deck-submenu input rule (update state 0x22, `0x1400D9C80`). Checked (decomp).
///
/// Adding a row is one more [`Row`] in [`ROWS`] with its label, help and page (M8 stage 8a.2 added
/// Items this way, M9 stage 9b Abilities). Since the owner's ask of 2026-11-01 the Keyblades row is gone:
/// the Keyblade is the first slot of the Items page's Sora list (`crate::items_page::SORA_KINDS`).
pub mod sub {
    use super::text;
    use super::top::Pad;

    /// A row of the submenu: the page it opens.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Row {
        /// M8 stage 8a.2: KH1's pause-menu Items screen (`crate::items_page`); Sora's list holds
        /// his Keyblade slot, then his item slots.
        Items,
        Customize,
        /// M9 stage 9b: KH1's ability screen with AP (`crate::abilities::page`).
        Abilities,
    }

    /// The rows, top to bottom.
    pub const ROWS: &[Row] = &[Row::Items, Row::Customize, Row::Abilities];

    /// Re:CoM's camp has 7 entry buttons (`camp+0x1B8[7]`); the submenu reuses them. Checked
    /// (decomp).
    pub const MAX_ROWS: usize = 7;
    const _: () = assert!(ROWS.len() <= MAX_ROWS && !ROWS.is_empty());

    impl Row {
        /// The button's label, also the page's header title (header item 1, the breadcrumb).
        pub fn label(self) -> &'static [u8] {
            match self {
                Row::Items => crate::items_page::text::LABEL,
                Row::Customize => text::CUSTOMIZE_LABEL,
                Row::Abilities => crate::abilities::page::text::LABEL,
            }
        }

        /// The help line while the row is highlighted.
        pub fn help(self) -> &'static [u8] {
            match self {
                Row::Items => crate::items_page::text::HELP,
                Row::Customize => text::CUSTOMIZE_HELP,
                Row::Abilities => crate::abilities::page::text::HELP,
            }
        }
    }

    /// What one frame of input does in the submenu.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Act {
        None,
        /// Back to the top list (sound 4).
        Close,
        /// Open the row's page (sound 2).
        Open(usize),
        /// The cursor to this row (sound 1).
        Move(i32),
    }

    /// Re:CoM's Deck submenu (state 0x22, `0x1400DB3F5..`): cancel first (`0x14029B410`; Start
    /// does nothing there), then confirm (`0x14029B490`), then up / down (repeat masks 0x101000 /
    /// 0x204000): at the ends a held repeat stops and only a fresh press goes on, and the cursor
    /// set `0x1400DEA10` wraps over the count (`camp+0x21C4`). Checked (decomp).
    pub fn step(cursor: i32, p: Pad) -> Act {
        let n = ROWS.len() as i32;
        if p.cancel {
            return Act::Close;
        }
        if p.confirm {
            return if (0..n).contains(&cursor) { Act::Open(cursor as usize) } else { Act::None };
        }
        let mv = super::top::recom_move(cursor, n, Pad { start: false, cancel: false, confirm: false, ..p });
        match mv {
            Some(i) if i < 0 => Act::Move(n - 1),
            Some(i) if i >= n => Act::Move(0),
            Some(i) => Act::Move(i),
            None => Act::None,
        }
    }
}

/// The blade span a Keyblade's hit spheres cover, in the weapon's bone parameter (KH1 weapon row
/// `+0x24 / +0x28 / +0x2C`, `kh1_core::WeaponData::blade_t`): `|far − near| × fraction scale`.
fn span(w: &kh1_core::WeaponData) -> f32 {
    (w.blade_far - w.blade_near).abs() * w.frac_scale
}

/// The stats panel's values for a Keyblade: KH1's Strength and Magic bonuses ("+3"), and its
/// reach as its blade span over the Kingdom Key's ("x1.00"). The reach is a display figure
/// only (nothing in either game shows it).
pub fn stat_values(k: &Keyblade) -> [String; 3] {
    let kk = span(&kh1_core::WeaponData::kingdom_key());
    let r = if kk > 0.0 { span(&k.weapon) / kk } else { 1.0 };
    [format!("{:+}", k.weapon.strength), format!("{:+}", k.magic), format!("x{r:.2}")]
}

/// NUL-terminated copy for Re:CoM's text items (non-ASCII and NUL bytes become '?').
pub fn c_text(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.bytes().map(|b| if (0x20..0x7F).contains(&b) { b } else { b'?' }).collect();
    v.push(0);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh1_core::WeaponData;

    #[test]
    fn save_byte_falls_back_to_the_kingdom_key() {
        let all = |_: u16| true;
        assert_eq!(equipped_card(0, all), KINGDOM_KEY);
        assert_eq!(equipped_card(1, all), 1);
        assert_eq!(equipped_card(0x11, all), 0x11);
        assert_eq!(equipped_card(0x18, all), 0x18);
        assert_eq!(equipped_card(0x12, all), KINGDOM_KEY, "Soul Eater");
        assert_eq!(equipped_card(0x19, all), KINGDOM_KEY);
        assert_eq!(equipped_card(0xFF, all), KINGDOM_KEY);
        assert_eq!(equipped_card(0x13, |k| k <= 0x11), KINGDOM_KEY, "no numbers yet");
        assert_eq!(save_byte_for(1), 0);
        assert_eq!(save_byte_for(0x0D), 0x0D);
        assert_eq!(save_byte_for(0x12), 0);
        assert_eq!(save_byte_for(0x30), 0);
        assert_eq!(EQUIP_OFFSET, 0x1A74 + 0xC00);
        assert!((SPARE_FIRST..=SPARE_LAST).contains(&EQUIP_OFFSET));
    }

    #[test]
    fn spare_range_ignores_our_byte() {
        let mut r = vec![0u8; SPARE_LAST - SPARE_FIRST + 1];
        assert!(spare_range_nonzero(&r).is_empty());
        r[EQUIP_OFFSET - SPARE_FIRST] = 5;
        assert!(spare_range_nonzero(&r).is_empty());
        // M7's MP block (`crate::mp`) is ours too.
        r[crate::mp::BLOCK_FIRST - SPARE_FIRST] = 1;
        r[crate::mp::BLOCK_LAST - SPARE_FIRST] = 1;
        assert!(spare_range_nonzero(&r).is_empty());
        // M8's items extension right after it is ours too, and M9's ability block after that; the
        // byte after that is not.
        r[crate::items::EXT_FIRST - SPARE_FIRST] = 1;
        r[crate::items::EXT_LAST - SPARE_FIRST] = 9;
        r[crate::abilities::BLOCK_FIRST - SPARE_FIRST] = 1;
        r[crate::abilities::BLOCK_LAST - SPARE_FIRST] = 4;
        assert!(spare_range_nonzero(&r).is_empty());
        // M8b's summon block after the ability block is ours; the byte after it is not.
        r[crate::summons::BLOCK_FIRST - SPARE_FIRST] = 1;
        r[crate::summons::BLOCK_LAST - SPARE_FIRST] = 7;
        assert!(spare_range_nonzero(&r).is_empty());
        r[crate::summons::BLOCK_LAST + 1 - SPARE_FIRST] = 3;
        assert_eq!(spare_range_nonzero(&r), vec![crate::summons::BLOCK_LAST + 1]);
        r[crate::summons::BLOCK_LAST + 1 - SPARE_FIRST] = 0;
        r[0] = 1;
        r[SPARE_LAST - SPARE_FIRST] = 2;
        assert_eq!(spare_range_nonzero(&r), vec![SPARE_FIRST, SPARE_LAST]);
    }

    #[test]
    fn unlocked_lists_earned_keyblades_with_numbers() {
        let cards: Vec<u16> = (1..=0x11).chain([0x12]).collect();
        // Nothing earned: the Kingdom Key only.
        assert_eq!(unlocked(cards.clone(), |_| false), vec![1]);
        // Plain Oathkeeper, premium Lionheart, Soul Eater "earned" (never listed).
        let e = |id: u16| id == 0x0D || id == (0x0A | PREMIUM) || id == 0x12;
        assert_eq!(unlocked(cards.clone(), e), vec![1, 0x0A, 0x0D]);
        // A Keyblade without numbers is not listed even when earned.
        assert_eq!(unlocked([1, 2], |_| true), vec![1, 2]);
        assert_eq!(unlocked([2, 1, 2, 0x19, 0], |_| true), vec![1, 2]);
        // No Kingdom Key numbers: not listed.
        assert_eq!(unlocked([3], |_| false), Vec::<u16>::new());
    }

    use top::{Fix, Pad};
    const NONE: Pad = Pad { start: false, cancel: false, confirm: false, up: false, up_fresh: false, down: false, down_fresh: false };
    const DOWN_HELD: Pad = Pad { down: true, ..NONE };
    const DOWN_FRESH: Pad = Pad { down: true, down_fresh: true, ..NONE };
    const UP_HELD: Pad = Pad { up: true, ..NONE };
    const UP_FRESH: Pad = Pad { up: true, up_fresh: true, ..NONE };

    #[test]
    fn recom_rule_unchanged_inside_the_list() {
        for c in 0..6 {
            assert_eq!(top::step(c, DOWN_HELD), Some(c + 1));
            assert_eq!(top::step(c, DOWN_FRESH), Some(c + 1));
        }
        for c in 1..8 {
            assert_eq!(top::step(c, UP_HELD), Some(c - 1));
            assert_eq!(top::step(c, UP_FRESH), Some(c - 1));
        }
    }

    #[test]
    fn eight_entries_with_recoms_end_rule() {
        // Down reaches our entry held or fresh.
        assert_eq!(top::step(6, DOWN_HELD), Some(7));
        assert_eq!(top::step(6, DOWN_FRESH), Some(7));
        // At the end: held stops, fresh wraps.
        assert_eq!(top::step(7, DOWN_HELD), None);
        assert_eq!(top::step(7, DOWN_FRESH), Some(0));
        assert_eq!(top::step(0, UP_HELD), None);
        assert_eq!(top::step(0, UP_FRESH), Some(7));
        // Confirm, cancel and Start are not moves; confirm on ours needs the fix.
        assert_eq!(top::step(7, Pad { confirm: true, ..NONE }), None);
        assert_eq!(top::fix(7, Pad { confirm: true, ..NONE }), Fix::ConfirmOurs);
        assert_eq!(top::fix(6, Pad { confirm: true, ..NONE }), Fix::None);
        assert_eq!(top::fix(7, Pad { confirm: true, cancel: true, ..NONE }), Fix::None, "cancel wins");
        assert_eq!(top::fix(6, Pad { down: true, up: true, ..NONE }), Fix::None, "up wins");
        assert_eq!(top::fix(6, DOWN_HELD), Fix::MoveToOurs);
        assert_eq!(top::fix(6, DOWN_FRESH), Fix::None, "Re:CoM's own code moves to 7 (wrapper)");
    }

    #[test]
    fn cursor_set_wraps_over_eight() {
        assert_eq!(top::wrap(-1), 7);
        assert_eq!(top::wrap(7), 7);
        assert_eq!(top::wrap(8), 0);
        assert_eq!(top::wrap(3), 3);
    }

    use list::{List, Pad as LPad};

    #[test]
    fn list_moves_like_recoms_list() {
        let l = List { cursor: 0, top: 0, count: 5 };
        let down = LPad { down: true, ..Default::default() };
        let (n, moved) = list::step(l, down);
        assert!(moved);
        assert_eq!(n.cursor, 1);
        // Held at the end stops; fresh wraps.
        let end = List { cursor: 4, ..l };
        assert_eq!(list::step(end, down), (end, false));
        assert_eq!(list::step(end, LPad { down: true, down_fresh: true, ..Default::default() }).0.cursor, 0);
        let up = LPad { up: true, ..Default::default() };
        assert_eq!(list::step(l, up), (l, false));
        assert_eq!(list::step(l, LPad { up: true, up_fresh: true, ..Default::default() }).0.cursor, 4);
        // Nothing pressed.
        assert_eq!(list::step(l, LPad::default()), (l, false));
    }

    #[test]
    fn list_pages_like_recoms_list() {
        let l = List { cursor: 2, top: 0, count: 17 };
        let pd = LPad { page_down: true, ..Default::default() };
        let (n, _) = list::step(l, pd);
        assert_eq!((n.cursor, n.top), (10, 8));
        let (n, _) = list::step(n, pd);
        assert_eq!((n.cursor, n.top), (16, 8), "past the end: the last row");
        assert_eq!(list::step(n, pd), (n, false), "held at the end stops");
        assert_eq!(list::step(n, LPad { page_down: true, page_down_fresh: true, ..Default::default() }).0.cursor, 0);
        let pu = LPad { page_up: true, ..Default::default() };
        let (m, _) = list::step(List { cursor: 12, top: 8, count: 17 }, pu);
        assert_eq!((m.cursor, m.top), (4, 0));
        assert_eq!(list::step(List { cursor: 5, top: 0, count: 17 }, pu).0.cursor, 0);
        let first = List { cursor: 0, top: 0, count: 17 };
        assert_eq!(list::step(first, pu), (first, false));
        assert_eq!(list::step(first, LPad { page_up: true, page_up_fresh: true, ..Default::default() }).0.cursor, 16);
    }

    #[test]
    fn top_row_follows_the_cursor() {
        assert_eq!(list::top_for(List { cursor: 9, top: 0, count: 17 }), 2);
        assert_eq!(list::top_for(List { cursor: 3, top: 5, count: 17 }), 3);
        assert_eq!(list::top_for(List { cursor: 6, top: 2, count: 17 }), 2);
        assert_eq!(list::open_on(17, 16), List { cursor: 16, top: 9, count: 17 });
        assert_eq!(list::open_on(3, 9), List { cursor: 2, top: 0, count: 3 });
        assert_eq!(list::bar(List { cursor: 0, top: 0, count: 4 }), (0.0, 1.0));
        assert_eq!(list::bar(List { cursor: 16, top: 8, count: 16 }), (0.5, 0.5));
        assert_eq!(list::bar(List { cursor: 0, top: 0, count: 0 }), (0.0, 1.0));
    }

    #[test]
    fn stats_panel_strings() {
        let k = Keyblade { card: 1, name: "Kingdom Key".into(), source: "kh1".into(), model: String::new(), weapon: WeaponData::kingdom_key(), magic: 0, group: None, textures: vec![] };
        assert_eq!(stat_values(&k), ["+3".to_string(), "+0".into(), "x1.00".into()]);
        let mut long = k.clone();
        long.weapon.blade_far = -7.4;
        long.weapon.strength = -1;
        long.magic = 4;
        assert_eq!(stat_values(&long), ["-1".to_string(), "+4".into(), "x2.00".into()]);
        for s in stat_values(&long) {
            assert!(!s.contains('%'));
        }
    }

    #[test]
    fn texts_are_terminated_ascii_without_codes() {
        let all = [
            text::ENTRY_LABEL,
            text::TITLE,
            text::ENTRY_HELP,
            text::PAGE_HELP,
            text::STAT_LABELS[0],
            text::STAT_LABELS[1],
            text::STAT_LABELS[2],
            text::CUSTOMIZE_LABEL,
            text::CUSTOMIZE_HELP,
        ];
        for s in all {
            assert_eq!(s.last(), Some(&0));
            assert!(s[..s.len() - 1].iter().all(|&b| (0x20..0x7F).contains(&b) && b != b'%'));
            assert!(s.len() - 1 <= crate::menu::HELP_MAX);
        }
        assert_eq!(c_text("One-Winged Angel"), b"One-Winged Angel\0".to_vec());
        assert_eq!(c_text("a\u{e9}"), b"a??\0".to_vec());
    }

    #[test]
    fn submenu_rows_and_moves_follow_recoms_deck_submenu() {
        use sub::{Act, Row, ROWS};
        // The owner's ask (2026-11-01): Items (the Keyblade slot and the item slots), Customize;
        // M9 9b: Abilities.
        assert_eq!(ROWS, &[Row::Items, Row::Customize, Row::Abilities]);
        assert_eq!((Row::Items.label(), Row::Customize.label(), Row::Abilities.label()), (&b"Items\0"[..], &b"Customize\0"[..], &b"Abilities\0"[..]));
        // Inside: held or fresh moves.
        assert_eq!(sub::step(0, DOWN_HELD), Act::Move(1));
        assert_eq!(sub::step(1, DOWN_HELD), Act::Move(2));
        assert_eq!(sub::step(1, UP_HELD), Act::Move(0));
        // Ends: held stops, fresh wraps (Re:CoM's cursor set wraps over the count).
        assert_eq!(sub::step(2, DOWN_HELD), Act::None);
        assert_eq!(sub::step(2, DOWN_FRESH), Act::Move(0));
        assert_eq!(sub::step(0, UP_HELD), Act::None);
        assert_eq!(sub::step(0, UP_FRESH), Act::Move(2));
        // Cancel first, then confirm; Start is nothing here.
        assert_eq!(sub::step(1, Pad { confirm: true, ..NONE }), Act::Open(1));
        assert_eq!(sub::step(1, Pad { confirm: true, cancel: true, ..NONE }), Act::Close);
        assert_eq!(sub::step(0, Pad { start: true, ..NONE }), Act::None);
        assert_eq!(sub::step(5, Pad { confirm: true, ..NONE }), Act::None);
        assert_eq!(sub::step(0, Pad { confirm: true, down: true, ..NONE }), Act::Open(0), "confirm before moves");
    }
}
