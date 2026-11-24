//! Riku's Dark Form skin (fork owner, 2026-11-22: "ele vai desbloquear a skin do Dark Mode e vai
//! poder jogar com ela no modo sem estar com darkmode ativado"; "pode colocar a skin mas os
//! movimentos ser o normal mesmo"). Pure rules; the game side is `win::riku_skin`.
//!
//! - Files (asset step `kh1_recom_assets::riku_skin`): player groups 121 / 131 hold Dark Riku's
//!   model and textures (`PL0201` / `PL0211`) with Riku's own motions retargeted onto it and
//!   Riku's sound / effect events (`PL0101` / `PL0111`), bones remapped by name.
//! - Rows: each skin group's rows copy its Riku group's rows in Re:CoM's PLAYER table (same
//!   types and order); the files the step wrote are named by the skin group's paths, the rest
//!   (camera data `.BIN`, sound bank `.SND`, effect links) stay Riku's.
//! - Use: Re:CoM picks a player's group from `0x1405A6910[char]` (`.rdata`: 0, 1, 11, 101, 111,
//!   201, 211; Checked (exe)) in the player ctor (`0x1401E4CDA`, stored at `+0x39C`), the group
//!   load (`0x140211A5B`, `0x140211AE4`) and unload (`0x1402115EE`, `0x14021161F`). Chars 3 / 4 →
//!   the skin groups while the skin is on. Riku's weapon takes no group (`0x140224C58`: chars
//!   3..6 attach to a body bone). Riku's bones are looked up by name (`0x1401A4450`), and the
//!   skin's bone 1 carries Riku's name (`ex_1010_sk`).

use crate::keyblade_models::{kind, resource_path, PlannedRow, RowName, END};

/// Re:CoM's char → action table pointers (`0x1407C6300[char]`, 16-byte rows: name pointer, clip
/// index `+8`). Checked (exe: `0x1401FA7FE`).
pub const ACTION_TABLES_PTR: u64 = 0x1_407C_6300;

/// Re:CoM's char → group table and the entries of chars 3 and 4. Checked (exe).
pub const GROUP_TABLE: u64 = 0x1_405A_6910;
/// Re:CoM's char → motion pack name table (`0x1407C6338[char]`: "PL0001" … "PL0211"): the player's
/// action rows' clips are looked up in the `.ANN` of that resource name (`0x1401FA7F6`,
/// `0x1401F425C` → `0x1402A42B0`). Only these two readers and one pointer per name (Checked
/// (exe)). 0.154.0 changed the group but not this name: no clip was found (the owner's T-pose).
pub const ANN_NAME_TABLE: u64 = 0x1_407C_6338;
/// Riku's two names ("PL0101", "PL0111") the table points at. Checked (exe).
pub const RIKU_ANN_NAMES: [u64; 2] = [0x1_405A_145C, 0x1_405A_1464];

/// The menu's own model (`CMenuModel`, ctor `0x140111F10`, resource id = group + 10000) takes its
/// group from bytes in two instructions, not from the char table (Checked (exe)): the camp's
/// `mov dword [rsp+0x30], 0x6F650B01` at `0x1400E4701` (groups 1, 11, 101, 111 by story × 2 +
/// Halloween Town, read at `0x1400E47EF`) and the level-up screen's `mov dword [rbp-0x15],
/// 0x006F0065` at `0x14010EF68` (model / weapon pairs: 101, 0, 111, 0; read at `0x14010F084`,
/// `0x14010F0E9`). No branch lands inside them (Checked (exe)). Each entry: the byte's address
/// and Riku's group (101 or 111).
pub const MENU_BYTES: [(u64, u16); 4] = [(0x1_400E_4707, 101), (0x1_400E_4708, 111), (0x1_4010_EF6B, 101), (0x1_4010_EF6D, 111)];

/// The motion pack name of a group (`PL0121`).
pub fn ann_name(group: u16) -> String {
    format!("PL{group:04}")
}

/// Riku's two groups (chars 3, 4), as the table holds them.
pub const RIKU_GROUPS: [u16; 2] = [101, 111];

/// Row types the skin's own files fill (Texture anim `.TXA` 0x13 and `.SES` 0x30, Checked (exe:
/// the table's rows)).
const OWN_KINDS: [(u16, &str); 6] = [(kind::RTM, "RTM"), (0x13, "TXA"), (kind::MDL, "MDL"), (kind::ANN, "ANN"), (kind::EET, "EET"), (0x30, "SES")];

/// One skin: Riku's group and the group that stands for it, with its look: the Dark Mode attire
/// and the blade (0.160.0: 0 the Soul Eater, 1 the Keyblade of People's Hearts). Older manifests
/// (no `dark` / `blade`): Dark Mode with the Soul Eater.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Skin {
    pub riku: u16,
    pub skin: u16,
    pub dark: bool,
    pub blade: u8,
    /// 0.186.0: 0 Re:CoM's hair, 1 DDD Riku's (only the default attire has it).
    pub hair: u8,
}

/// Riku's look: the attire and the blade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Look {
    pub dark: bool,
    pub blade: u8,
    /// 0.178.0: Riku's Halloween Town look in every world (char 3 on his Halloween group 111 and
    /// its skins, as char 4 is there).
    pub halloween: bool,
    /// 0.186.0: 0 Re:CoM's hair, 1 DDD Riku's.
    pub hair: u8,
}

impl Look {
    /// As one byte (bit 0 Dark Mode, bits 1..2 the blade, bit 3 Halloween Town, bits 4..5 the hair).
    pub fn bits(self) -> u8 {
        self.dark as u8 | (self.blade & 3) << 1 | (self.halloween as u8) << 3 | (self.hair & 3) << 4
    }
    pub fn from_bits(b: u8) -> Look {
        Look { dark: b & 1 != 0, blade: (b >> 1) & 3, halloween: b & 8 != 0, hair: (b >> 4) & 3 }
    }
}

/// The Keyblades Riku can hold (the Costumes page's names).
pub const BLADES: [&str; 2] = ["Soul Eater", "Keyblade of Heart"];

/// Parses `kh1-riku-skin.json` (`{"format": 1, "skins": [{"riku": 101, "skin": 121}, …]}`).
pub fn parse(text: &str) -> Result<Vec<Skin>, String> {
    let v = crate::json::parse(text).map_err(|e| format!("kh1-riku-skin.json: {e}"))?;
    if v.get("format").and_then(|f| f.as_u32()) != Some(1) {
        return Err("kh1-riku-skin.json: not format 1".into());
    }
    let mut out = Vec::new();
    for s in v.get("skins").and_then(|s| s.as_array()).ok_or("kh1-riku-skin.json: no skins")? {
        let g = |k: &str| s.get(k).and_then(|x| x.as_u32()).filter(|&x| x > 0 && x < END as u32).map(|x| x as u16);
        let (Some(riku), Some(skin)) = (g("riku"), g("skin")) else { return Err("kh1-riku-skin.json: a skin without riku / skin".into()) };
        let dark = s.get("dark").map_or(true, |d| matches!(d, crate::json::Value::Bool(true)));
        let blade = s.get("blade").and_then(|b| b.as_u32()).unwrap_or(0);
        if blade as usize >= BLADES.len() {
            return Err(format!("kh1-riku-skin.json: skin {skin}: blade {blade}"));
        }
        let blade = blade as u8;
        let hair = s.get("hair").and_then(|b| b.as_u32()).unwrap_or(0).min(1) as u8;
        if !RIKU_GROUPS.contains(&riku) || (!dark && blade == 0 && hair == 0) || out.iter().any(|o: &Skin| (o.riku == riku && o.dark == dark && o.blade == blade && o.hair == hair) || o.skin == skin) {
            return Err(format!("kh1-riku-skin.json: skin {skin} for group {riku}"));
        }
        out.push(Skin { riku, skin, dark, blade, hair });
    }
    Ok(out)
}

/// The manifest says Riku's own packs carry Sora's KH1 Guard (`"riku_guard": true`, 0.157.0).
pub fn riku_guard(text: &str) -> bool {
    crate::json::parse(text).ok().and_then(|v| v.get("riku_guard").map(|g| matches!(g, crate::json::Value::Bool(true)))).unwrap_or(false)
}

/// 0.184.1: the manifest says Riku's and Dark Riku's own packs carry the cast clips (`"riku_cast": true`).
pub fn riku_cast(text: &str) -> bool {
    crate::json::parse(text).ok().and_then(|v| v.get("riku_cast").map(|g| matches!(g, crate::json::Value::Bool(true)))).unwrap_or(false)
}

/// The files of a skin group the DLL checks are on disk (relative to the manifest's folder).
pub fn files(skin: u16) -> Vec<String> {
    OWN_KINDS
        .iter()
        .map(|&(k, ext)| if k == kind::RTM || ext == "TXA" { format!("raw/{}", resource_path(skin, ext)) } else { resource_path(skin, ext) })
        .collect()
}

/// `planned` (Re:CoM's rows, ours, then the end row) with each skin's rows before the end row:
/// its Riku group's rows, contiguous, retyped to the skin's group.
pub fn add_rows(mut planned: Vec<PlannedRow>, skins: &[Skin]) -> Result<Vec<PlannedRow>, String> {
    let end = planned.iter().position(|r| r.group == END).ok_or("no end row")?;
    if end + 1 != planned.len() {
        return Err("rows after the end row".into());
    }
    let mut add = Vec::new();
    for s in skins {
        if planned.iter().any(|r| r.group == s.skin) {
            return Err(format!("group {} is already in the table", s.skin));
        }
        let first = planned.iter().position(|r| r.group == s.riku).ok_or_else(|| format!("no rows of group {}", s.riku))?;
        let rows: Vec<&PlannedRow> = planned[first..].iter().take_while(|r| r.group == s.riku).collect();
        if planned.iter().filter(|r| r.group == s.riku).count() != rows.len() {
            return Err(format!("group {}'s rows are not contiguous", s.riku));
        }
        for &(k, _) in &OWN_KINDS {
            if !rows.iter().any(|r| r.kind == k) {
                return Err(format!("group {} has no row of type {k:#x}", s.riku));
            }
        }
        for r in rows {
            let name = match OWN_KINDS.iter().find(|o| o.0 == r.kind) {
                Some(&(_, ext)) => RowName::Own(resource_path(s.skin, ext)),
                None => r.name.clone(),
            };
            add.push(PlannedRow { group: s.skin, kind: r.kind, name });
        }
    }
    planned.splice(end..end, add);
    Ok(planned)
}

/// The table's entries for chars 3 and 4 for a look (Riku's own group where no skin has it).
pub fn entries(skins: &[Skin], look: Look) -> [u32; 2] {
    RIKU_GROUPS.map(|g| group_for_char(skins, g, look) as u32)
}

/// The group for Riku's char whose own group is `riku` (101 / 111): with the Halloween Town look,
/// char 3 takes Halloween Town's (111) and its skins.
pub fn group_for_char(skins: &[Skin], riku: u16, look: Look) -> u16 {
    group_for(skins, if look.halloween { RIKU_GROUPS[1] } else { riku }, look)
}

/// Riku's group `riku` in a look.
pub fn group_for(skins: &[Skin], riku: u16, look: Look) -> u16 {
    // 0.186.0: a look with DDD's hair that has no hair group (Dark Mode, Halloween Town…) shows Re:CoM's.
    let find = |hair: u8| skins.iter().find(|s| s.riku == riku && s.dark == look.dark && s.blade == look.blade && s.hair == hair);
    find(look.hair).or_else(|| find(0)).map_or(riku, |s| s.skin)
}

/// The look a group shows (Riku's own: the Default Attire with the Soul Eater).
pub fn look_of(skins: &[Skin], group: u16) -> Look {
    match skins.iter().find(|s| s.skin == group) {
        Some(s) => Look { dark: s.dark, blade: s.blade, halloween: s.riku == RIKU_GROUPS[1], hair: s.hair },
        None => Look { halloween: group == RIKU_GROUPS[1], ..Look::default() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(g: u16, k: u16, n: &str) -> PlannedRow {
        PlannedRow { group: g, kind: k, name: RowName::Own(n.into()) }
    }

    #[test]
    fn skin_rows_copy_rikus_and_name_its_files() {
        let mut t = Vec::new();
        for (k, n) in [(0xC, "RTM"), (0x13, "TXA"), (0xB, "MDL"), (0x1D, "ANN"), (0x17, "BIN"), (0x1A, "EET"), (0x27, "31500"), (0x30, "SES"), (0x2C, "SND")] {
            t.push(row(101, k, &format!("PLAYER/0101/PL0101.{n}")));
        }
        t.push(row(201, 0xC, "x"));
        t.push(row(END, 0, "end"));
        let skins = parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 121}]}"#).unwrap();
        let out = add_rows(t.clone(), &skins).unwrap();
        assert_eq!(out.len(), t.len() + 9);
        assert_eq!(out.last().unwrap().group, END);
        let ours: Vec<_> = out.iter().filter(|r| r.group == 121).collect();
        assert_eq!(ours.len(), 9);
        let name = |k: u16| match &ours.iter().find(|r| r.kind == k).unwrap().name { RowName::Own(s) => s.clone(), RowName::Recom(_) => String::new() };
        assert_eq!(name(0xB), "PLAYER/0121/PL0121.MDL");
        assert_eq!(name(0x13), "PLAYER/0121/PL0121.TXA");
        assert_eq!(name(0x30), "PLAYER/0121/PL0121.SES");
        assert_eq!(name(0x17), "PLAYER/0101/PL0101.BIN", "Riku's camera data");
        assert_eq!(name(0x2C), "PLAYER/0101/PL0101.SND", "Riku's sound bank");
        assert!(add_rows(out, &skins).is_err(), "twice");
        assert_eq!(entries(&skins, Look { dark: true, blade: 0, halloween: false, hair: 0 }), [121, 111]);
        assert_eq!(entries(&skins, Look::default()), [101, 111]);
        assert_eq!(skins[0], Skin { riku: 101, skin: 121, dark: true, blade: 0, hair: 0 }, "an older manifest: Dark Mode, Soul Eater");
        let all = parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 121, "dark": true, "blade": 0}, {"riku": 101, "skin": 141, "dark": false, "blade": 1}, {"riku": 101, "skin": 161, "dark": true, "blade": 1}, {"riku": 111, "skin": 151, "dark": false, "blade": 1}]}"#).unwrap();
        assert_eq!(entries(&all, Look { dark: false, blade: 1, halloween: false, hair: 0 }), [141, 151]);
        assert_eq!(entries(&all, Look { dark: true, blade: 1, halloween: false, hair: 0 }), [161, 111], "no Halloween group for that look: Riku's own");
        assert_eq!(look_of(&all, 161), Look { dark: true, blade: 1, halloween: false, hair: 0 });
        // 0.178.0: the Halloween Town look puts char 3 on 111 and its skins.
        let hw = |dark, blade| Look { dark, blade, halloween: true, hair: 0 };
        assert_eq!(entries(&all, hw(false, 0)), [111, 111]);
        assert_eq!(entries(&all, hw(false, 1)), [151, 151]);
        assert_eq!(entries(&all, hw(true, 1)), [111, 111], "no Halloween Dark group in this manifest: Riku's own Halloween look");
        assert_eq!(look_of(&all, 111), hw(false, 0));
        assert_eq!(look_of(&all, 101), Look::default());
        assert_eq!(Look::from_bits(hw(true, 1).bits()), hw(true, 1));
        assert_eq!(Look::from_bits(Look { dark: true, blade: 1, halloween: false, hair: 0 }.bits()), Look { dark: true, blade: 1, halloween: false, hair: 0 });
        assert!(parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 141, "dark": false, "blade": 0}]}"#).is_err(), "Riku's own look");
        // 0.186.0: DDD's hair groups (default attire, Soul Eater 181 / Keyblade of Heart 191); a look without one shows Re:CoM's.
        let hair = parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 121, "dark": true, "blade": 0}, {"riku": 101, "skin": 141, "dark": false, "blade": 1}, {"riku": 101, "skin": 181, "dark": false, "blade": 0, "hair": 1}, {"riku": 101, "skin": 191, "dark": false, "blade": 1, "hair": 1}]}"#).unwrap();
        let lk = |dark, blade, hair| Look { dark, blade, halloween: false, hair };
        assert_eq!(entries(&hair, lk(false, 0, 1)), [181, 111]);
        assert_eq!(entries(&hair, lk(false, 1, 1)), [191, 111]);
        assert_eq!(entries(&hair, lk(false, 0, 0)), [101, 111], "Re:CoM's hair: Riku's own group");
        assert_eq!(entries(&hair, lk(false, 1, 0)), [141, 111]);
        assert_eq!(entries(&hair, lk(true, 0, 1)), [121, 111], "Dark Mode has no DDD hair: Re:CoM's");
        assert_eq!(look_of(&hair, 191), lk(false, 1, 1));
        assert_eq!(Look::from_bits(lk(false, 1, 1).bits()), lk(false, 1, 1));
        assert!(parse(r#"{"format": 1, "skins": [{"riku": 101, "skin": 181, "dark": false, "blade": 0, "hair": 1}, {"riku": 101, "skin": 182, "dark": false, "blade": 0, "hair": 1}]}"#).is_err(), "one group per look");
        assert_eq!(ann_name(121), "PL0121");
        assert!(files(121).contains(&"raw/PLAYER/0121/PL0121.RTM".to_string()));
        assert!(parse(r#"{"format": 1, "skins": [{"riku": 3, "skin": 121}]}"#).is_err());
        assert!(riku_guard(r#"{"format": 1, "skins": [], "riku_guard": true}"#));
        assert!(riku_cast(r#"{"format": 1, "skins": [], "riku_guard": false, "riku_cast": true}"#));
        assert!(!riku_cast(r#"{"format": 1, "skins": [], "riku_guard": true}"#), "older manifests: no cast clips");
        assert!(!riku_guard(r#"{"format": 1, "skins": [{"riku": 101, "skin": 121}]}"#), "older manifests: no guard");
    }
}
