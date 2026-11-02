//! The camp list's green row with Re:CoM's item symbol (owner, 2026-11-01: "for the item slots,
//! the color is correct, but they have the green mickey symbol, but lets use the green item
//! symbol"; plan approved "Yes, add it"). Notes: `docs/recom-m8-items.md` § Row looks.
//!
//! The asset step (`kh1-recom-assets` `camp_rows`) appends one group to Re:CoM's `camp:00`
//! (`FORM/0100/FO0100.SPR`, every language copy): Re:CoM's green row 0x131 with its icon's sprite
//! group 85 (sprite 278, the Mickey symbol) swapped for 83 (sprite 166, the item symbol of the Deck
//! page's item-card tab), and writes `kh1-camp-rows.json` naming it.
//!
//! How a row gets it (Checked (decomp)): Re:CoM's row set-up `0x140112F10` keeps the table it is
//! given (`CButton+0x10`) and plays sequence `table[variant]` (`*(u16*)(table − 2 + 2 × (variant +
//! 1))`); its state change `0x140112E00` reads the same kept table again for a new variant. The
//! camp list's table `0x1407C11F0` has 7 entries (0x12E 0x133 0x131 0x132 0x12F 0x160 0x130) and a
//! 0 after them, so the new group needs its own entry: the DLL keeps an 8-entry table in its own
//! memory, Re:CoM's 7 read from the exe plus ours at [`ITEM_VARIANT`], and gives that table to the
//! set-up (every variant still means what it means in Re:CoM's). A language's copy of `camp.lad`
//! may have more groups than the base file (the owner's `FR_` .. `UK_camp.lad`: 367, Checked
//! (files)), so its new group is after its own: the manifest lists every copy's new group
//! (`item_rows`) and the DLL takes the one the loaded set ends with (its group count − 1).
//! Without the manifest, or when the loaded `camp:00` doesn't end with one of them (Panacea serves
//! Re:CoM's own file), the rows keep Re:CoM's table and its green row 0x131.

use crate::json;

/// The manifest's file name, next to the other asset-step manifests.
pub const MANIFEST: &str = "kh1-camp-rows.json";
/// The manifest's `format` (`kh1-recom-assets` writes it).
pub const FORMAT: &str = "kh1-recom-assets camp-rows 2";
/// The sprite set the group is in.
pub const SET: &str = "camp:00";
/// Re:CoM Steam's animation groups in `camp:00` (Checked (files)).
pub const BASE_GROUPS: u32 = 353;
/// Re:CoM's green row the new group copies.
pub const COPY_OF: u32 = 0x131;
/// The icon's sprite group in 0x131 (Mickey symbol) and in the copy (item symbol).
pub const MICKEY_SG: u32 = 85;
pub const ITEM_SG: u32 = 83;
/// Entries of Re:CoM's camp list table `0x1407C11F0` before its 0.
pub const RECOM_ENTRIES: usize = 7;
/// Our table's entry for the new group (after Re:CoM's 7).
pub const ITEM_VARIANT: i32 = RECOM_ENTRIES as i32;

/// Reads `kh1-camp-rows.json`: every copy's new group (sorted), after checking it is what this DLL
/// expects (format, set, Re:CoM's group count, a copy of 0x131 with sprite group 85 → 83, the base
/// file's group right after Re:CoM's, each copy's at or after it).
pub fn parse(text: &str) -> Result<Vec<u16>, String> {
    let v = json::parse(text)?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(format!("not a \"{FORMAT}\" manifest (an older asset step? run it again)"));
    }
    let n = |k: &str| v.get(k).and_then(|x| x.as_u32()).ok_or_else(|| format!("no \"{k}\""));
    if v.get("set").and_then(|s| s.as_str()) != Some(SET) {
        return Err(format!("not the set {SET}"));
    }
    let (base, row, copy) = (n("base_groups")?, n("item_row")?, n("copy_of")?);
    let icon = v.get("icon_sprite_group").ok_or("no \"icon_sprite_group\"")?;
    let (from, to) = (icon.get("from").and_then(|x| x.as_u32()), icon.get("to").and_then(|x| x.as_u32()));
    if base != BASE_GROUPS || row != BASE_GROUPS || copy != COPY_OF || from != Some(MICKEY_SG) || to != Some(ITEM_SG) {
        return Err(format!("group {row} (copy of {copy:#x}, icon {from:?} → {to:?}, base {base}), this DLL expects {BASE_GROUPS} (copy of {COPY_OF:#x}, icon {MICKEY_SG} → {ITEM_SG}, base {BASE_GROUPS})"));
    }
    let list = v.get("item_rows").and_then(|a| a.as_array()).ok_or("no \"item_rows\"")?;
    let mut rows = Vec::with_capacity(list.len());
    for x in list {
        match x.as_u32() {
            Some(g) if (BASE_GROUPS..u16::MAX as u32).contains(&g) => rows.push(g as u16),
            _ => return Err(format!("\"item_rows\": {x:?} is not a group after Re:CoM's {BASE_GROUPS}")),
        }
    }
    rows.sort_unstable();
    rows.dedup();
    if !rows.contains(&(row as u16)) {
        return Err(format!("\"item_rows\" {rows:?} without the base file's {row}"));
    }
    Ok(rows)
}

/// Our list table: Re:CoM's 7 entries, then the item row, so `table[variant]` is Re:CoM's for its
/// variants and ours for [`ITEM_VARIANT`].
pub fn list_table(recom: [u16; RECOM_ENTRIES], item_row: u16) -> [u16; RECOM_ENTRIES + 1] {
    let mut t = [0u16; RECOM_ENTRIES + 1];
    t[..RECOM_ENTRIES].copy_from_slice(&recom);
    t[RECOM_ENTRIES] = item_row;
    t
}

/// The item row of the loaded set: its last group when that is one of the manifest's (the served
/// file is the asset step's, in this language's copy too); `None` otherwise.
pub fn loaded_row(loaded_groups: Option<u32>, rows: &[u16]) -> Option<u16> {
    let last = loaded_groups?.checked_sub(1)?;
    rows.iter().copied().find(|&r| r as u32 == last)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"{
  "format": "kh1-recom-assets camp-rows 2",
  "set": "camp:00",
  "base_groups": 353,
  "item_row": 353,
  "item_rows": [353, 367],
  "copy_of": 305,
  "icon_sprite_group": {"from": 85, "to": 83},
  "files": ["FORM/0100/FO0100.SPR", "UK_camp.lad"],
  "sources": {"FO0100.SPR sha256": "ab"}
}"#;

    #[test]
    fn manifest_names_the_group_after_recoms() {
        assert_eq!(parse(OK), Ok(vec![353, 367]));
        assert!(parse(&OK.replace("\"item_rows\": [353, 367],", "")).is_err());
        assert!(parse(&OK.replace("[353, 367]", "[367]")).is_err(), "without the base file's");
        assert!(parse(&OK.replace("[353, 367]", "[353, 12]")).is_err());
        assert!(parse(&OK.replace("camp-rows 2", "camp-rows 1")).is_err());
        assert!(parse(&OK.replace("\"item_row\": 353", "\"item_row\": 354")).is_err());
        assert!(parse(&OK.replace("\"copy_of\": 305", "\"copy_of\": 307")).is_err());
        assert!(parse(&OK.replace("\"to\": 83", "\"to\": 80")).is_err());
        assert!(parse(&OK.replace("camp:00", "Cockpit:00")).is_err());
        assert!(parse("{}").is_err());
    }

    /// The table keeps Re:CoM's 7 entries (the checked bytes of `0x1407C11F0`) where they are.
    #[test]
    fn our_table_is_recoms_plus_the_item_row() {
        let seqs = crate::sites::KEYBLADE_DATA.iter().find(|s| s.rva == 0x007C_11F0).unwrap().expect;
        let recom: [u16; RECOM_ENTRIES] = std::array::from_fn(|i| u16::from_le_bytes([seqs[2 * i], seqs[2 * i + 1]]));
        let t = list_table(recom, 353);
        assert_eq!(t, [0x12E, 0x133, 0x131, 0x132, 0x12F, 0x160, 0x130, 0x161]);
        use crate::keyblades::row_look::{BLUE, GREEN, RED};
        assert_eq!((t[RED as usize], t[BLUE as usize], t[GREEN as usize], t[ITEM_VARIANT as usize]), (0x12E, 0x133, 0x131, 0x161));
    }

    #[test]
    fn only_a_set_with_the_group_uses_it() {
        let rows = [353, 367];
        assert_eq!(loaded_row(Some(354), &rows), Some(353));
        assert_eq!(loaded_row(Some(368), &rows), Some(367), "a language's copy");
        assert_eq!(loaded_row(Some(353), &rows), None, "Re:CoM's own file");
        assert_eq!(loaded_row(Some(367), &rows), None, "a language's own file");
        assert_eq!(loaded_row(Some(0), &rows), None);
        assert_eq!(loaded_row(None, &rows), None);
    }
}
