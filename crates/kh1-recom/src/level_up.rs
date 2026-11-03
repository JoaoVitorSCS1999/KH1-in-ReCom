//! Re:CoM's level-up screen `CLevelUp` at Combat = KH1 (M12; `docs/recom-m12-sleights.md` § 8):
//! its CP choice becomes **AP Up** (step 1), and its "sleight learned" mode shows our "Sora learned
//! X" notices (step 2, `crate::notices`).
//!
//! `CLevelUp` (vtable `0x140512C10`, ctor `0x14010E9C0(this, parent, sleight)`, update
//! `0x14010EF10`, 0x2C0 bytes). Checked (decomp + exe) for every offset and state below:
//!
//! - state `+0x6C`; 0 / 1: resources, then the init block (`LAB_14010F013`): the level + 1 and the
//!   offers (`+0x238` HP: 0 while max HP < 560; **`+0x23C` CP: 0 while CP < 1625**; `+0x240`
//!   Sleight: the level list's first unlearned entry), the cursor `+0x220` on the first offered row;
//!   with `+0x258` ≠ 0 the "sleight learned" mode instead (cursor 2, `+0x250` = the sleight's name,
//!   state 0xE). 2: builds the rows (`0x140110480(this, 0)`), the status panel (`0x140110C60(this,
//!   0)`); 3 / 4: they open; 6: the pick (cursor moves re-print the panel, `0x140110C60(this, 7)`);
//!   confirm: row 0 max HP + 15, **row 1 (Sora) CP + 25**, row 2 learns the sleight; the result
//!   line (`0x140110140(this, 0)`) → 7; 8: a sleight pick opens `CTechHelp`; 0xB: the next level
//!   (state 1) or the end (0xD). Sleight mode: 0xE the line (its wording `0xA8070F`, or for ids
//!   ≥ 0x5A in Sora's story Re:CoM's "ability" wording `0xA80710`, with the name `+0x250`), 0xF a
//!   180-frame timer or a press, then `CTechHelp(this, id)` → 0x10 waits for it (`0x14087A4B0`),
//!   0x11 closes, 0xD. 0xD: the task ends.
//! - Text items (`D2::C2DFnt`, text pointer `+0x58`, set by `0x140258DE0`): row n's label
//!   `+0xC8 + 0x10 n` (message `0x1407C1838[(char × 3 + n) × 2]`: Sora's CP row `0xA80703`) and
//!   value `+0xD0 + 0x10 n` (`0xA80704`); the status panel's row n label `+0x170 + 0x10 n`
//!   (`0x1407C1810[char × 5 + n]`: `0xA80603`) and value `+0x178 + 0x10 n` (format
//!   `0x1407C1898[(n + char × 5) × 3]` `"%4d"`, the highlighted row `+1` `"%4d \xF9Y\x81\xA8\xF9R%4d"`;
//!   the exe's English names beside them: "Max HP", "Max CP", "Sleights", Riku's "AP"); the result
//!   line `+0x1E0` (`0x1407C1868[char × 3 + n]`: `0xA8070E`).
//! - Its child tasks: list at `+0x30` (`CSLCListT<CTask>`), tail node `+0x48`, node `+8` the task
//!   (`0x1402ABBA0` appends).
//!
//! Re:CoM's message texts (`0xA806xx` / `0xA807xx`) are not in the session; our words are English,
//! as every text of ours (Assumption: they read well beside Re:CoM's).

/// `CLevelUp` fields. Checked (decomp + exe).
pub mod off {
    pub const STATE: usize = 0x6C;
    /// Its child task list's tail node (`+0x30` list: `+0x48`); node `+8` = the task.
    pub const CHILD_TAIL: usize = 0x48;
    pub const NODE_TASK: usize = 0x8;
    pub const CURSOR: usize = 0x220;
    pub const CHARACTER: usize = 0x230;
    /// The three offers (i32: ≥ 0 offered, −1 greyed): HP, CP (Riku: AP), Sleight (Riku: DP).
    pub const OFFER: usize = 0x238;
    /// The sleight-learned line's name (`char *`, `%s` of its wording).
    pub const NAME: usize = 0x250;
    /// 0: a level-up; else the sleight-learned screen for this id.
    pub const SLEIGHT: usize = 0x258;
    pub const ROW_LABEL: usize = 0xC8;
    pub const ROW_VALUE: usize = 0xD0;
    pub const ROW_STEP: usize = 0x10;
    pub const PANEL_LABEL: usize = 0x170;
    pub const PANEL_VALUE: usize = 0x178;
    pub const PANEL_STEP: usize = 0x10;
    /// The pick's result line (made at the confirm).
    pub const RESULT: usize = 0x1E0;
    /// `D2::C2DFnt`'s text pointer.
    pub const FNT_TEXT: usize = 0x58;
}

/// `CLevelUp` states. Checked (decomp).
pub mod state {
    pub const RESOURCES: i32 = 0;
    pub const INIT: i32 = 1;
    pub const BUILD: i32 = 2;
    pub const OPENING: i32 = 3;
    pub const PICK: i32 = 6;
    pub const PICKED: i32 = 7;
    pub const NEXT: i32 = 0xB;
    pub const END: i32 = 0xD;
    pub const LEARNED: i32 = 0xE;
    pub const LEARNED_SHOW: i32 = 0xF;
    pub const LEARNED_HELP: i32 = 0x10;
    pub const LEARNED_CLOSE: i32 = 0x11;
}

/// Sora's CP row (row 1; Riku's is his AP row).
pub const CP_ROW: usize = 1;
/// Re:CoM's CP offer cap: offered while CP < 1625 (`cmp [stats+0xC],0x659` at `0x14010F210`).
pub const RECOM_CP_OFFER_CAP: i32 = 0x659;
/// The sleight id our ability notices open the sleight-learned screen with: Re:CoM's High Jump
/// (0x5A), the first id its line words as an ability (`cmp [rbx+0x258],0x5A` at `0x1401102DD`).
/// Only the wording follows the id; the name is ours (`+0x250`).
pub const ABILITY_WORDING_ID: u16 = 0x5A;

/// `CTechHelp` (vtable `0x14051B4A8`, ctor `0x14012BF80`, update `0x14012D330`, dtor `0x14012C230`):
/// state `+0x88` (0 loads its page `+0x90` = 300 + n, …, 5 and up: the task ends itself), page
/// `+0x90` (its dtor unloads it when ≠ 0), `0x14087A4B0` = 1 when it is gone. Checked (decomp + exe).
pub mod tech_help {
    pub const VTABLE: u64 = 0x1_4051_B4A8;
    pub const STATE: usize = 0x88;
    pub const PAGE: usize = 0x90;
    /// Its update's default branch: the task ends (no page loaded, none unloaded with page 0).
    pub const DONE: u32 = 5;
}

/// The AP Up row's words and values (owner, M12: "On level up, instead of having the choice to
/// level up CP, when the combat is set to KH1, it should be AP up, and it should give you 2 AP").
/// NUL-terminated for Re:CoM's text items; no `%` (Re:CoM prints some lines through its sprintf).
pub mod text {
    pub const ROW_LABEL: &[u8] = b"AP Up\0";
    pub const ROW_VALUE: &[u8] = b"Max AP +2\0";
    /// The status panel's label (Re:CoM's "Max CP" there).
    pub const PANEL_LABEL: &[u8] = b"Max AP\0";
    pub const RESULT: &[u8] = b"Max AP increased by 2.\0";
}

/// The status panel's AP value as Re:CoM prints its CP: `"%4d"`, and on the highlighted row its
/// preview `"%4d \xF9Y\x81\xA8\xF9R%4d"` (the arrow glyph between colour codes; the exe's format
/// `0x140512C70`). `max`: the max AP now.
pub fn panel_value(max: i32, highlighted: bool) -> Vec<u8> {
    let mut v = format!("{max:4}").into_bytes();
    if highlighted {
        v.extend_from_slice(b" \xF9Y\x81\xA8\xF9R");
        v.extend_from_slice(format!("{:4}", max.saturating_add(crate::abilities::AP_PER_UP)).as_bytes());
    }
    v.push(0);
    v
}

/// Whether a C string equals `want` (with its NUL).
pub fn same_text(have: &[u8], want: &[u8]) -> bool {
    have == want.strip_suffix(&[0]).unwrap_or(want)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_value_matches_recoms_formats() {
        assert_eq!(panel_value(3, false), b"   3\0");
        assert_eq!(panel_value(13, true), b"  13 \xF9Y\x81\xA8\xF9R  15\0");
        assert_eq!(panel_value(199, true), b" 199 \xF9Y\x81\xA8\xF9R 201\0");
    }

    /// Re:CoM's preview format, as the exe holds it at `0x140512C70` (Checked (exe)).
    #[test]
    fn the_preview_is_recoms_glyphs() {
        let fmt: &[u8] = b"%4d \xF9Y\x81\xA8\xF9R%4d";
        let ours = panel_value(1, true);
        assert_eq!(&ours[4..11], &fmt[3..10]);
    }

    #[test]
    fn texts_are_plain() {
        for t in [text::ROW_LABEL, text::ROW_VALUE, text::PANEL_LABEL, text::RESULT] {
            assert_eq!(t.last(), Some(&0));
            assert!(t[..t.len() - 1].iter().all(|&b| (0x20..0x7F).contains(&b) && b != b'%'));
            assert!(t.len() - 1 <= crate::menu::HELP_MAX);
        }
        assert!(same_text(b"AP Up", text::ROW_LABEL));
        assert!(!same_text(b"CP Up", text::ROW_LABEL));
    }

    #[test]
    fn ids() {
        assert_eq!(ABILITY_WORDING_ID, 0x5A);
        assert_eq!(RECOM_CP_OFFER_CAP, 1625);
        assert_eq!(off::ROW_LABEL + off::ROW_STEP * CP_ROW, 0xD8);
        assert_eq!(off::PANEL_VALUE + off::PANEL_STEP * CP_ROW, 0x188);
    }
}
