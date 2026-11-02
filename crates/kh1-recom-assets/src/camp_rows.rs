//! The camp list's green row with Re:CoM's **item** symbol (owner, 2026-11-01: "for the item
//! slots, the color is correct, but they have the green mickey symbol, but lets use the green item
//! symbol"; the plan "Yes, add it"). `docs/recom-m8-items.md` § Row looks.
//!
//! Re:CoM's camp list rows play one sequence of `camp:00` (SEQD 0 of `camp.lad` in
//! `FORM/0100/FO0100.SPR`), picked by the row set-up `0x140112F10` from the table it is given.
//! Re:CoM's green row is group 0x131: four animations (an empty piece, the green plate, a glow,
//! the icon = sprite group 85, sprite 278, the green Mickey symbol, tinted 0x801EDC1E). Re:CoM's
//! item symbol is sprite group 83 (sprite 166), drawn only on the Deck page's card-type tabs
//! (`0xED` lights the item cards' tab under it). No row of Re:CoM's has it. Checked (decomp + files).
//!
//! This step appends one group to `camp:00`: 0x131's four animations copied, the icon's sprite
//! group 85 swapped for 83, everything else (frames, place, tint, flags, the group's own words) as
//! 0x131 has it. Re:CoM's records stay as they are; the DLL gives the Stock row and Sora's item
//! slots the new group through the same row set-up (`kh1-camp-rows.json`).

use crate::mp_bar::{Container, Layd, Seqd, AGROUP, ANIM};
use crate::{bad, Result};

/// Re:CoM's camp menu sprite file.
pub const SPR_PATH: &str = "FORM/0100/FO0100.SPR";
/// The layout in it that holds the camp list's rows.
pub const LAYOUT: &str = "camp.lad";
/// The sprite set (SEQD 0 of `camp.lad`).
pub const SET: &str = "camp:00";
/// Its SEQD index in the layout.
pub const SEQ: usize = 0;
/// Re:CoM Steam's animation groups in `camp:00` (Checked (files): the owner's `FO0100.SPR`).
pub const BASE_GROUPS: usize = 353;
/// Re:CoM's green row (the camp list table `0x1407C11F0` entry 2).
pub const GREEN_ROW: usize = 0x131;
/// The green row's icon: sprite group 85 (sprite 278, the Mickey symbol).
pub const MICKEY_SG: usize = 85;
/// Re:CoM's item symbol: sprite group 83 (sprite 166).
pub const ITEM_SG: usize = 83;
/// The Deck page's item-card tab (lit), which draws the item symbol (`0x1407C10D0` tab 3).
pub const ITEM_TAB: usize = 0xED;
/// The manifest the DLL reads, next to the other asset-step manifests.
pub const MANIFEST: &str = "kh1-camp-rows.json";
/// The manifest's `format`.
pub const FORMAT: &str = "kh1-recom-assets camp-rows 1";

fn i32_at(r: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(r[at..at + 4].try_into().unwrap())
}

fn group_anims(seq: &Seqd, g: usize) -> Result<std::ops::Range<usize>> {
    let ag = seq.agroups.get(g).ok_or_else(|| crate::Error::Format(format!("{SET}: no group {g:#x}")))?;
    let a0 = u16::from_le_bytes([ag[0], ag[1]]) as usize;
    let n = u16::from_le_bytes([ag[2], ag[3]]) as usize;
    if a0 + n > seq.anims.len() {
        return bad(format!("{SET}: group {g:#x} past the animations"));
    }
    Ok(a0..a0 + n)
}

fn one_part(seq: &Seqd, sg: usize) -> bool {
    seq.sgroups.get(sg).is_some_and(|s| i16::from_le_bytes([s[2], s[3]]) == 1)
}

/// Appends the green row with the item symbol (0x131 with its icon's sprite group 85 → 83) and
/// returns the new group's index. Refused unless the set looks like Re:CoM Steam's: its group
/// count, 0x131 with exactly one animation on sprite group 85, sprite groups 83 and 85 one part
/// each, and the Deck's item tab 0xED drawing sprite group 83.
pub fn add_item_row(seq: &mut Seqd) -> Result<usize> {
    if seq.agroups.len() != BASE_GROUPS {
        let note = if seq.agroups.len() == BASE_GROUPS + 1 { " (already patched?)" } else { "" };
        return bad(format!("{SET} has {} animation groups, Re:CoM Steam's has {BASE_GROUPS}{note}", seq.agroups.len()));
    }
    if !one_part(seq, MICKEY_SG) || !one_part(seq, ITEM_SG) {
        return bad(format!("{SET}: sprite groups {MICKEY_SG} / {ITEM_SG} are not Re:CoM's one-part icons"));
    }
    if !group_anims(seq, ITEM_TAB)?.any(|a| i32_at(&seq.anims[a], 4) as usize == ITEM_SG) {
        return bad(format!("{SET}: the Deck's item tab {ITEM_TAB:#x} does not draw sprite group {ITEM_SG}"));
    }
    let range = group_anims(seq, GREEN_ROW)?;
    let icons = range.clone().filter(|&a| i32_at(&seq.anims[a], 4) as usize == MICKEY_SG).count();
    if icons != 1 {
        return bad(format!("{SET}: group {GREEN_ROW:#x} has {icons} animations on sprite group {MICKEY_SG} (Re:CoM's has one)"));
    }
    let mut anims: Vec<[u8; ANIM]> = range.map(|a| seq.anims[a]).collect();
    for a in anims.iter_mut().filter(|a| i32_at(&a[..], 4) as usize == MICKEY_SG) {
        a[4..8].copy_from_slice(&(ITEM_SG as i32).to_le_bytes());
    }
    let first = seq.anims.len();
    if first + anims.len() > i16::MAX as usize {
        return bad("seqd: too many animations for 16-bit indices");
    }
    seq.anims.extend(anims);
    let mut r: [u8; AGROUP] = seq.agroups[GREEN_ROW];
    r[0..2].copy_from_slice(&(first as u16).to_le_bytes());
    seq.agroups.push(r);
    Ok(seq.agroups.len() - 1)
}

/// Adds the row to a `camp.lad` (its `camp:00`); returns the layout and the new group.
pub fn patch_layout(lad: &[u8]) -> Result<(Vec<u8>, usize)> {
    let mut l = Layd::parse(lad)?;
    let raw = l.seqs.get(SEQ).ok_or_else(|| crate::Error::Format(format!("{LAYOUT}: no {SET}")))?;
    let mut s = Seqd::parse(raw)?;
    let g = add_item_row(&mut s)?;
    l.seqs[SEQ] = s.write();
    Ok((l.write(), g))
}

/// A `camp.lad` file name: `camp.lad` or a language's `<LANG>_camp.lad`.
pub fn is_camp_layout(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == LAYOUT || (n.len() == LAYOUT.len() + 3 && n.ends_with(LAYOUT) && n.as_bytes()[2] == b'_')
}

/// Adds the row to the `camp.lad` inside an `FO0100.SPR` container.
pub fn patch_container(spr: &[u8]) -> Result<(Vec<u8>, usize)> {
    let mut c = Container::parse(spr)?;
    let e = c.find_mut(LAYOUT).ok_or_else(|| crate::Error::Format(format!("{SPR_PATH}: no {LAYOUT}")))?;
    let (lad, g) = patch_layout(&e.data)?;
    e.data = lad;
    Ok((c.write(), g))
}

/// Patches the `FO0100.SPR` container and every PC asset of its entry that is a `camp.lad`
/// (any language prefix) or a whole container with one; returns the new group and the files
/// patched. Every copy must give the same group (the DLL names one for all languages).
pub fn patch_spr(spr: &mut Vec<u8>, assets: &mut [kh1_formats::egs::HdAsset]) -> Result<(usize, Vec<String>)> {
    let (out, g) = patch_container(spr)?;
    *spr = out;
    let mut files = vec![SPR_PATH.to_string()];
    for a in assets.iter_mut() {
        let patched = if a.data.starts_with(b"LAYD") {
            if !is_camp_layout(&a.name) {
                continue;
            }
            patch_layout(&a.data)
        } else {
            match Container::parse(&a.data) {
                Ok(c) if c.find(LAYOUT).is_some() => patch_container(&a.data),
                _ => continue,
            }
        };
        let (d, f) = patched.map_err(|e| crate::Error::Format(format!("{} asset {}: {e}", SPR_PATH, a.name)))?;
        if f != g {
            return bad(format!("{SPR_PATH} asset {}: new group {f}, the base file's {g}", a.name));
        }
        a.data = d;
        files.push(a.name.clone());
    }
    Ok((g, files))
}

/// `kh1-camp-rows.json`: the set, Re:CoM's group count, the new group, what it copies, the files
/// patched and the input's sha256.
pub fn manifest_json(group: usize, files: &[String], sources: &[(&str, String)]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"format\": \"{FORMAT}\",");
    let _ = writeln!(s, "  \"set\": \"{SET}\",");
    let _ = writeln!(s, "  \"base_groups\": {BASE_GROUPS},");
    let _ = writeln!(s, "  \"item_row\": {group},");
    let _ = writeln!(s, "  \"copy_of\": {GREEN_ROW},");
    let _ = writeln!(s, "  \"icon_sprite_group\": {{\"from\": {MICKEY_SG}, \"to\": {ITEM_SG}}},");
    let f: Vec<String> = files.iter().map(|n| format!("{n:?}")).collect();
    let _ = writeln!(s, "  \"files\": [{}],", f.join(", "));
    let src: Vec<String> = sources.iter().map(|(k, v)| format!("{k:?}: {v:?}")).collect();
    let _ = writeln!(s, "  \"sources\": {{{}}}", src.join(", "));
    let _ = writeln!(s, "}}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp_bar::{agroup, anim, part, sgroup, sprite};

    /// A tiny `camp:00`: 353 groups; sprite groups 0..=85 one part each; group 0x131 = four
    /// animations (sprite groups 46, 70, 46, 85 as Re:CoM's), 0xED one on 83, the rest one on 0.
    fn seq() -> Seqd {
        let mut s = Seqd { version: 0x160, sprites: Vec::new(), parts: Vec::new(), sgroups: Vec::new(), anims: Vec::new(), agroups: Vec::new() };
        s.sprites.push(sprite([1, 1, 2, 2], [0x8080_8080; 4]));
        for g in 0..=MICKEY_SG {
            s.parts.push(part([-6, -8, 6, 7], 0));
            s.sgroups.push(sgroup(g, 1));
        }
        s.anims.push(anim(0x47FF3, 0, (0, 7), 0, (1.0, 1.0), (0x8080_8080, 0x8080_8080)));
        for (sg, c) in [(46, 0x8080_8080), (70, 0x8014_3200), (46, 0x80F0_F0F0), (MICKEY_SG, 0x801E_DC1E)] {
            s.anims.push(anim(0x47BF3, sg, (0, 7), 13, (1.0, 1.0), (c, c)));
        }
        s.anims.push(anim(0x47FF3, ITEM_SG, (0, 16), 35, (1.0, 1.0), (0x8080_8080, 0x8080_8080)));
        for g in 0..BASE_GROUPS {
            match g {
                GREEN_ROW => s.agroups.push(agroup(1, 4, false, 0x36F)),
                ITEM_TAB => s.agroups.push(agroup(5, 1, false, 0x36F)),
                _ => s.agroups.push(agroup(0, 1, false, 0x36F)),
            }
        }
        s
    }

    fn sg_of(a: &[u8; ANIM]) -> usize {
        i32_at(a, 4) as usize
    }

    #[test]
    fn the_new_group_is_0x131_with_the_item_icon() {
        let mut s = seq();
        let before = s.clone();
        let g = add_item_row(&mut s).unwrap();
        assert_eq!(g, BASE_GROUPS);
        // Re:CoM's records untouched.
        assert_eq!(&s.anims[..before.anims.len()], &before.anims[..]);
        assert_eq!(&s.agroups[..BASE_GROUPS], &before.agroups[..]);
        assert_eq!((s.sprites.len(), s.parts.len(), s.sgroups.len()), (before.sprites.len(), before.parts.len(), before.sgroups.len()));
        // The group: 0x131's words, four new animations; only the icon's sprite group differs.
        let ag = s.agroups[g];
        assert_eq!((u16::from_le_bytes([ag[0], ag[1]]) as usize, u16::from_le_bytes([ag[2], ag[3]])), (before.anims.len(), 4));
        assert_eq!(ag[2..], before.agroups[GREEN_ROW][2..]);
        for k in 0..4 {
            let (new, old) = (s.anims[before.anims.len() + k], before.anims[1 + k]);
            if sg_of(&old) == MICKEY_SG {
                assert_eq!(sg_of(&new), ITEM_SG);
                assert_eq!(new[8..], old[8..], "place, frames, tint as 0x131's");
                assert_eq!(new[..4], old[..4]);
            } else {
                assert_eq!(new, old);
            }
        }
    }

    #[test]
    fn refuses_what_is_not_recoms_set() {
        let mut s = seq();
        add_item_row(&mut s).unwrap();
        assert!(add_item_row(&mut s).unwrap_err().to_string().contains("already patched"));
        let mut s = seq();
        s.anims[5][4..8].copy_from_slice(&0i32.to_le_bytes()); // the tab no longer on 83
        assert!(add_item_row(&mut s).is_err());
        let mut s = seq();
        s.anims[4][4..8].copy_from_slice(&0i32.to_le_bytes()); // 0x131 without the Mickey icon
        assert!(add_item_row(&mut s).is_err());
        let mut s = seq();
        s.sgroups[ITEM_SG] = sgroup(0, 2);
        assert!(add_item_row(&mut s).is_err());
    }

    fn layd(seqs: &[Vec<u8>]) -> Vec<u8> {
        let mut d = b"LAYD".to_vec();
        for v in [0x100u32, 0, 0x20, 0, 0x20, seqs.len() as u32, 0x20] {
            d.extend(v.to_le_bytes());
        }
        let mut at = 0x20 + 4 * seqs.len() as u32;
        for q in seqs {
            d.extend(at.to_le_bytes());
            at += q.len() as u32;
        }
        for q in seqs {
            d.extend(q);
        }
        d
    }

    #[test]
    fn container_and_language_copies() {
        let lad = layd(&[seq().write()]);
        let other = layd(&[Seqd { agroups: seq().agroups[..3].to_vec(), ..seq() }.write()]);
        let entry = |name: &str, data: Vec<u8>| {
            let mut raw = [0u8; 0x18];
            raw[..name.len()].copy_from_slice(name.as_bytes());
            crate::mp_bar::Entry { name_raw: raw, flag: true, data }
        };
        let spr = Container { entries: vec![entry("camp.lad", lad.clone()), entry("worldmap.lad", other.clone())] }.write();
        let mut base = spr.clone();
        let mut assets = vec![
            kh1_formats::egs::HdAsset { name: "UK_camp.lad".into(), orig_offset: 0xFFFF, data: lad.clone() },
            kh1_formats::egs::HdAsset { name: "UK_worldmap.lad".into(), orig_offset: 0xFFFF, data: other.clone() },
            kh1_formats::egs::HdAsset { name: "whole".into(), orig_offset: 0xFFFF, data: spr.clone() },
        ];
        let (g, files) = patch_spr(&mut base, &mut assets).unwrap();
        assert_eq!(g, BASE_GROUPS);
        assert_eq!(files, vec![SPR_PATH.to_string(), "UK_camp.lad".into(), "whole".into()]);
        assert_eq!(assets[1].data, other, "other layouts kept");
        let c = Container::parse(&base).unwrap();
        assert_eq!(c.find("worldmap.lad").unwrap().data, other);
        let s = Seqd::parse(&Layd::parse(&c.find("camp.lad").unwrap().data).unwrap().seqs[0]).unwrap();
        assert_eq!(s.agroups.len(), BASE_GROUPS + 1);
        let s = Seqd::parse(&Layd::parse(&assets[0].data).unwrap().seqs[0]).unwrap();
        assert_eq!(s.agroups.len(), BASE_GROUPS + 1);
        assert!(is_camp_layout("FR_camp.lad") && is_camp_layout("CAMP.LAD") && !is_camp_layout("UK_worldmap.lad") && !is_camp_layout("UK_bootcamp.lad"));
        let m = manifest_json(g, &files, &[("FO0100.SPR sha256", "ab".into())]);
        assert!(m.contains("\"item_row\": 353") && m.contains("\"format\": \"kh1-recom-assets camp-rows 1\""));
    }

    /// The owner's `FO0100.SPR` container (`KH1_FO0100_SPR` = its path; the base file without
    /// its PC assets): read back byte for byte, Re:CoM's records kept, the new group at 353.
    #[test]
    fn owner_camp_file() {
        let Ok(path) = std::env::var("KH1_FO0100_SPR") else { return };
        let spr = std::fs::read(path).unwrap();
        let c = Container::parse(&spr).unwrap();
        let lad = c.find(LAYOUT).unwrap().data.clone();
        let l = Layd::parse(&lad).unwrap();
        assert_eq!(l.write(), lad, "camp.lad reads back byte for byte");
        let s0 = Seqd::parse(&l.seqs[SEQ]).unwrap();
        assert_eq!(s0.write(), l.seqs[SEQ], "camp:00 reads back byte for byte");
        let (out, g) = patch_container(&spr).unwrap();
        assert_eq!(g, BASE_GROUPS);
        let c2 = Container::parse(&out).unwrap();
        for e in &c.entries {
            if e.name() != LAYOUT {
                assert_eq!(c2.find(&e.name()).unwrap().data, e.data);
            }
        }
        let s = Seqd::parse(&Layd::parse(&c2.find(LAYOUT).unwrap().data).unwrap().seqs[SEQ]).unwrap();
        assert_eq!(&s.anims[..s0.anims.len()], &s0.anims[..]);
        assert_eq!(&s.agroups[..BASE_GROUPS], &s0.agroups[..]);
        let r = group_anims(&s, g).unwrap();
        let sgs: Vec<usize> = r.map(|a| sg_of(&s.anims[a])).collect();
        let old: Vec<usize> = group_anims(&s0, GREEN_ROW).unwrap().map(|a| sg_of(&s0.anims[a])).collect();
        eprintln!("camp:00 group {g:#x}: sprite groups {sgs:?} (0x131: {old:?})");
        assert_eq!(sgs.iter().filter(|&&x| x == ITEM_SG).count(), 1);
        assert_eq!(old.iter().filter(|&&x| x == MICKEY_SG).count(), 1);
    }
}
