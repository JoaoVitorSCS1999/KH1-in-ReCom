//! The asset build step's KH1 motions in Re:CoM's `PL0001.ANN`: its manifest
//! (`kh1-motions.json`) and the pack's header. Nothing uses this at run time yet.
//!
//! The "KH1 motions test" switch that re-pointed Sora's action-table rows to these clips was
//! removed by owner decision (2026-10-30: Re:CoM's own animations stay; KH1 clips only for what
//! Re:CoM lacks). The findings it relied on stay true and are kept for that later work
//! (`docs/recom-motion-switch.md`): Sora's action table (`0x1407C5E80`, through
//! `0x1407C6300[charIndex]`) holds 16-byte rows `{const char* name; int index; int pad}`; the
//! player's `stateInitial` (`0x1401FA6E0`) resolves every row's name to a pack slot with
//! `0x1402A42B0` (exact-name scan, no slot limit), and the motion request `0x1401F7180` plays
//! `row.index`. Checked (decomp + exe).

use crate::json::{self, Value};

/// Address of Sora's action table (Steam). `0x1407C6300[1]` and `[2]` point here. Checked (exe).
pub const SORA_TABLE: u64 = 0x1_407C_5E80;
/// The per-character table pointer array. Checked (exe + decomp).
pub const CHAR_TABLES: u64 = 0x1_407C_6300;
/// Rows in Sora's table before the terminating null name (0x47). Checked (exe).
pub const SORA_ROWS: usize = 0x47;
/// Re:CoM's own slots in `PL0001.ANN`. Checked (files).
pub const RECOM_SLOTS: u32 = 161;

/// One clip of the manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clip {
    pub slot: u32,
    pub name: String,
    pub entry: Option<u32>,
    pub request: Option<u32>,
}

/// Reads `kh1-motions.json`. Accepted shapes: a top-level array of clip objects, or an object
/// whose `slots` / `motions` / `clips` member is that array. Each clip needs `slot` or `name`
/// (`p001_161`), and may give `kh1_entry` / `entry` and `kh1_request` / `request`.
pub fn parse_manifest(text: &str) -> Result<Vec<Clip>, String> {
    let v = json::parse(text)?;
    let arr = match &v {
        Value::Arr(a) => a,
        Value::Obj(_) => match ["slots", "motions", "clips"].iter().find_map(|k| v.get(k)) {
            Some(Value::Arr(a)) => a,
            _ => return Err("no `slots` array".into()),
        },
        _ => return Err("not an object or array".into()),
    };
    let mut out = Vec::new();
    for (i, c) in arr.iter().enumerate() {
        let name = c.get("name").and_then(Value::as_str).map(str::to_string);
        let slot = c.get("slot").and_then(Value::as_u32).or_else(|| name.as_deref().and_then(slot_of_name));
        let Some(slot) = slot else {
            return Err(format!("clip {i}: no slot or name"));
        };
        let name = name.unwrap_or_else(|| format!("p001_{slot:03}"));
        if name.is_empty() || name.len() > 15 || !name.is_ascii() {
            return Err(format!("clip {i}: name `{name}` must be 1-15 ASCII characters"));
        }
        let pick = |a: &str, b: &str| c.get(a).or_else(|| c.get(b)).and_then(Value::as_u32);
        out.push(Clip { slot, name, entry: pick("kh1_entry", "entry"), request: pick("kh1_request", "request") });
    }
    Ok(out)
}

/// Stage 9c.1: the manifest's `xl_limit` section (KH1's specials' motions of `xl_limit.dat`, keyed
/// by xl entry): each `{"xl_entry", "slot", "name"}` as a [`Clip`] whose `entry` is the xl entry.
/// An older manifest (none): empty.
pub fn parse_xl(text: &str) -> Result<Vec<Clip>, String> {
    let v = json::parse(text)?;
    let Some(Value::Arr(arr)) = v.get("xl_limit") else { return Ok(Vec::new()) };
    let mut out = Vec::new();
    for (i, c) in arr.iter().enumerate() {
        let name = c.get("name").and_then(Value::as_str).map(str::to_string);
        let (Some(slot), Some(entry)) = (c.get("slot").and_then(Value::as_u32), c.get("xl_entry").and_then(Value::as_u32)) else {
            return Err(format!("xl clip {i}: no slot or xl_entry"));
        };
        let name = name.unwrap_or_else(|| format!("p001_{slot:03}"));
        if name.is_empty() || name.len() > 15 || !name.is_ascii() {
            return Err(format!("xl clip {i}: name `{name}` must be 1-15 ASCII characters"));
        }
        out.push(Clip { slot, name, entry: Some(entry), request: c.get("kh1_request").and_then(Value::as_u32) });
    }
    Ok(out)
}

/// M11 (the Neoshadow's grab): the manifest's `held` section, KH1's held clip (request `kh1_request` of
/// the Neoshadow's `.mset` player set) as a new slot of Sora's pack, with KH1's request blend.
#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    pub clip: Clip,
    /// KH1's frames (126 with the owner's files).
    pub frames: u32,
    /// KH1's blend of Sora's request (`ex_2420.bd` `0x1CDA`: 4).
    pub blend: f32,
}

/// Reads the manifest's `held` section (`{"enemy", "kh1_request", "slot", "name", "frames", "blend"}`). An
/// older manifest (none): `None`.
pub fn parse_held(text: &str) -> Result<Option<Held>, String> {
    let v = json::parse(text)?;
    let Some(h) = v.get("held") else { return Ok(None) };
    let n = |k: &str| h.get(k).and_then(Value::as_u32).ok_or_else(|| format!("held clip: no \"{k}\""));
    let slot = n("slot")?;
    let frames = n("frames")?;
    let blend = h.get("blend").and_then(Value::as_f64).filter(|b| b.is_finite() && *b >= 0.0).ok_or("held clip: no \"blend\"")? as f32;
    let name = h.get("name").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| format!("p001_{slot:03}"));
    if name.is_empty() || name.len() > 15 || !name.is_ascii() || slot < RECOM_SLOTS || frames == 0 {
        return Err(format!("held clip: slot {slot} `{name}`, {frames} frames"));
    }
    Ok(Some(Held { clip: Clip { slot, name, entry: None, request: h.get("kh1_request").and_then(Value::as_u32) }, frames, blend }))
}

/// The held clip when the pack holds it at its slot (the name and a non-empty clip, as
/// [`clip_for_entry`] checks).
pub fn held_in_pack<'a>(h: &'a Held, ann: &[(u32, String)]) -> Result<&'a Held, String> {
    match ann.get(h.clip.slot as usize) {
        Some((size, name)) if *size > 0 && *name == h.clip.name => Ok(h),
        Some((size, name)) => Err(format!("PL0001.ANN slot {} is `{name}` ({size} bytes), manifest says `{}`", h.clip.slot, h.clip.name)),
        None => Err(format!("PL0001.ANN has {} slots, no slot {} for the held clip", ann.len(), h.clip.slot)),
    }
}

/// `p001_161` → 161 (the three trailing digits, as Re:CoM's `0x1401A47E0` reads them).
pub fn slot_of_name(name: &str) -> Option<u32> {
    let d = name.get(name.len().checked_sub(3)?..)?;
    if d.bytes().all(|b| b.is_ascii_digit()) {
        d.parse().ok()
    } else {
        None
    }
}

/// The `.ANN` header: `u32 count`, then `count × {u32 size; char name[16]}`. Checked (decomp
/// `0x14029F3A0` + files). Returns (size, name) per slot.
pub fn ann_header(bytes: &[u8]) -> Result<Vec<(u32, String)>, String> {
    let count = bytes.get(0..4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or("file too short")? as usize;
    if count > 4096 {
        return Err(format!("slot count {count} is not plausible"));
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = 4 + i * 20;
        let e = bytes.get(o..o + 20).ok_or_else(|| format!("header cut at slot {i}"))?;
        let size = u32::from_le_bytes([e[0], e[1], e[2], e[3]]);
        let n = &e[4..20];
        let n = &n[..n.iter().position(|&b| b == 0).unwrap_or(16)];
        out.push((size, String::from_utf8_lossy(n).into_owned()));
    }
    Ok(out)
}

/// Bytes of the header for `count` slots (to read only that much of the file).
pub fn ann_header_len(count: u32) -> usize {
    4 + count as usize * 20
}

/// The manifest clip holding KH1 mset `entry`, if it lies past Re:CoM's own slots and the
/// pack's header has it under the same name (what later stages need before using a clip).
pub fn clip_for_entry<'a>(manifest: &'a [Clip], ann: &[(u32, String)], entry: u32) -> Result<&'a Clip, String> {
    let c = manifest.iter().find(|c| c.entry == Some(entry)).ok_or_else(|| format!("manifest has no KH1 entry {entry}"))?;
    if c.slot < RECOM_SLOTS {
        return Err(format!("entry {entry}: slot {} would be one of Re:CoM's own", c.slot));
    }
    match ann.get(c.slot as usize) {
        Some((size, name)) if *size > 0 && *name == c.name => Ok(c),
        Some((size, name)) => Err(format!("PL0001.ANN slot {} is `{name}` ({size} bytes), manifest says `{}`", c.slot, c.name)),
        None => Err(format!("PL0001.ANN has {} slots, no slot {} for entry {entry}", ann.len(), c.slot)),
    }
}

/// The manifest clip of KH1 mset `entry` when it is one of Re:CoM's own (slot below 161: Re:CoM
/// already has the motion, decision 11), for finding the Re:CoM action row that plays it.
pub fn recom_clip_for_entry(manifest: &[Clip], entry: u32) -> Option<&Clip> {
    manifest.iter().find(|c| c.entry == Some(entry)).filter(|c| c.slot < RECOM_SLOTS)
}

/// The row of Sora's action table (`names[i]` = row i's clip name) that plays `clip`.
pub fn row_of_clip(names: &[Option<String>], clip: &str) -> Option<u32> {
    names.iter().position(|n| n.as_deref() == Some(clip)).map(|i| i as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ann(n: u32) -> Vec<(u32, String)> {
        (0..n).map(|i| (100, format!("p001_{i:03}"))).collect()
    }

    const MANIFEST: &str = r#"{"slots": [
        {"slot": 161, "name": "p001_161", "kh1_entry": 0, "kh1_request": 0},
        {"slot": 162, "kh1_entry": 1},
        {"slot": 163, "kh1_entry": 2, "kh1_request": 1},
        {"slot": 164, "kh1_entry": 3},
        {"slot": 165, "kh1_entry": 5, "kh1_request": 2},
        {"slot": 166, "kh1_entry": 6},
        {"name": "p001_167", "entry": 98, "request": 200},
        {"slot": 168, "kh1_entry": 99, "kh1_request": "0xC9"},
        {"slot": 169, "kh1_entry": 100},
        {"slot": 170, "kh1_entry": 101}
    ]}"#;

    #[test]
    fn manifest_shapes() {
        let m = parse_manifest(MANIFEST).unwrap();
        assert_eq!(m.len(), 10);
        assert_eq!(m[1], Clip { slot: 162, name: "p001_162".into(), entry: Some(1), request: None });
        assert_eq!(m[6], Clip { slot: 167, name: "p001_167".into(), entry: Some(98), request: Some(200) });
        assert_eq!(m[7].request, Some(201));
        let arr = parse_manifest(r#"[{"slot": 170}]"#).unwrap();
        assert_eq!(arr[0].name, "p001_170");
        assert!(parse_manifest(r#"{"x": 1}"#).is_err());
        assert!(parse_manifest(r#"[{"kh1_entry": 1}]"#).is_err());
        assert!(parse_manifest(r#"[{"name": "a_name_that_is_too_long"}]"#).is_err());
    }

    #[test]
    fn slot_names() {
        assert_eq!(slot_of_name("p001_161"), Some(161));
        assert_eq!(slot_of_name("p001_05x"), None);
        assert_eq!(slot_of_name("ab"), None);
    }

    #[test]
    fn ann_header_reads_names() {
        let mut b = 3u32.to_le_bytes().to_vec();
        for (s, n) in [(10u32, "p001_000"), (0, ""), (5, "p001_161_long16c")] {
            b.extend_from_slice(&s.to_le_bytes());
            let mut name = [0u8; 16];
            name[..n.len()].copy_from_slice(n.as_bytes());
            b.extend_from_slice(&name);
        }
        let h = ann_header(&b).unwrap();
        assert_eq!(h, vec![(10, "p001_000".into()), (0, String::new()), (5, "p001_161_long16c".into())]);
        assert_eq!(ann_header_len(3), b.len());
        assert!(ann_header(&b[..30]).is_err());
    }

    #[test]
    fn recoms_own_clips_and_their_rows() {
        let m = parse_manifest(r#"{"slots": [{"slot": 103, "name": "p001_103", "kh1_entry": 106}, {"slot": 222, "kh1_entry": 107}]}"#).unwrap();
        assert_eq!(recom_clip_for_entry(&m, 106).unwrap().name, "p001_103");
        assert!(recom_clip_for_entry(&m, 107).is_none(), "a KH1-only clip is not Re:CoM's");
        let names = vec![None, Some("p001_001".to_string()), Some("p001_103".to_string())];
        assert_eq!(row_of_clip(&names, "p001_103"), Some(2));
        assert_eq!(row_of_clip(&names, "p001_104"), None);
    }

    #[test]
    fn held_section() {
        let t = r#"{"slots": [], "held": {"enemy": "neoshadow", "kh1_request": 200, "slot": 233, "name": "p001_233", "frames": 126, "blend": 4.0}}"#;
        let h = parse_held(t).unwrap().unwrap();
        assert_eq!((h.clip.slot, h.clip.name.as_str(), h.clip.request, h.frames, h.blend), (233, "p001_233", Some(200), 126, 4.0));
        assert!(held_in_pack(&h, &ann(234)).is_ok());
        assert!(held_in_pack(&h, &ann(233)).is_err(), "the pack lacks the slot");
        assert_eq!(parse_held(MANIFEST).unwrap(), None, "an older manifest has none");
        assert!(parse_held(r#"{"held": {"slot": 100, "frames": 126, "blend": 4}}"#).is_err(), "one of Re:CoM's own slots");
        assert!(parse_held(r#"{"held": {"slot": 233, "frames": 126}}"#).is_err());
    }

    #[test]
    fn xl_section() {
        let t = r#"{"slots": [{"slot": 161, "kh1_entry": 0}], "xl_limit": [{"xl_entry": 0, "kh1_request": 200, "slot": 230, "name": "p001_230"}, {"xl_entry": 1, "slot": 231}]}"#;
        let x = parse_xl(t).unwrap();
        assert_eq!(x[0], Clip { slot: 230, name: "p001_230".into(), entry: Some(0), request: Some(200) });
        assert_eq!(x[1].name, "p001_231");
        assert_eq!(clip_for_entry(&x, &ann(232), 1).unwrap().slot, 231);
        assert!(parse_xl(MANIFEST).unwrap().is_empty(), "an older manifest has none");
        assert!(parse_xl(r#"{"xl_limit": [{"slot": 1}]}"#).is_err());
    }

    #[test]
    fn finds_clips_by_kh1_entry() {
        let m = parse_manifest(MANIFEST).unwrap();
        assert_eq!(clip_for_entry(&m, &ann(171), 98).unwrap().name, "p001_167");
        assert_eq!(clip_for_entry(&m, &ann(171), 101).unwrap().slot, 170);
        // Not in the manifest; pack too short; slot renamed or empty; inside Re:CoM's range.
        assert!(clip_for_entry(&m, &ann(171), 4).is_err());
        assert!(clip_for_entry(&m, &ann(161), 98).is_err());
        let mut a = ann(171);
        a[167].1 = "other".into();
        assert!(clip_for_entry(&m, &a, 98).is_err());
        let mut a = ann(171);
        a[170].0 = 0;
        assert!(clip_for_entry(&m, &a, 101).is_err());
        let mut m3 = m.clone();
        m3[9].slot = 58;
        m3[9].name = "p001_058".into();
        assert!(clip_for_entry(&m3, &ann(171), 101).is_err());
    }
}
