//! M13 stage 1: KH1's effect sets as Re:CoM's own effect resources (owner, 2026-11-03: "Let's work
//! on m13", "Go ahead"; `docs/recom-m13-plan.md`).
//!
//! KH1 and Re:CoM use one effect format (DPX `0x82` sets of PPP `0x96` programs; Re:CoM's
//! `ex_5010.dpx` is byte-for-byte KH1's `xw_ex_5010.wpn` section 0: Checked (files),
//! `docs/recom-m13-kh1-fx.md` § 8). So the step copies a KH1 set unchanged into the effect container
//! Sora's group already loads, under a `kh1_` name, and Re:CoM loads, steps, draws and ends it as
//! its own. Stage 1 adds **Sora's KH1 set** (`xa_ex_0010.mdls` section 1: 8 programs, ids 0–7, with
//! the two Ripple Drive effects Re:CoM's edited copy `ex_0010.dpx` lacks). The Keyblades' sets go
//! with their own Keyblade groups (stage 4), so only the equipped one is in memory.
//!
//! Re:CoM (Steam; Checked (decomp)):
//! - **Container** (`0x1402A2A60`): rows of 0x20 bytes `{u32 offset from the file start, name
//!   (row +4, up to 21 characters: `strncpy(…, 0x15)`), u8 +0x1A, u8 +0x1B, u32 size | 0x80000000}`,
//!   read until a row whose offset is 0; each row's type comes from its name's extension.
//! - **Where Sora's effects load:** PLAYER 1's LINK `31000` = EFFECT group 1000 = `EF1000.EFF`
//!   (`ex_0010.dpx`, `ex_5010.dpx`, `ex_0010.ese`). Checked (files).
//! - **Effect names are global** (`0x1402A43E0(5, name)` compares up to the `.` across every loaded
//!   resource of the type), hence the `kh1_` prefix.
//! - **PC textures** of a set: `0x140477BB0` → `0x1402BBBC0` names a texture
//!   `sprintf("%s%x", lower-case set name, texture − set start)` with its `.` made `_`, then finds the
//!   entry's PC asset of that name + `.dds` (`0x1402BBC42`: `'%s%x'`). KH1 tags the same texture by
//!   its file offset (`-4310.dds` = section 0x80 + 0x4290 ↔ Re:CoM's `ex_5010_dpx4290.dds`), so a KH1
//!   asset `-<hex file offset>.dds` inside the section becomes `<set>_dpx<hex offset in the set>.dds`.
//! - **Effect sounds** are not read from a program (`+0xFC`); they come from a `.ese` of the set's
//!   name (stage 2 writes it with the KH1 sound banks). Until then the KH1 set plays silent.
//!
//! Re:CoM's own rows keep their bytes, names and flags; only their offsets move by the new rows'
//! 0x20 bytes each (the container is read by offset). Re:CoM's PC assets are kept as they are.

use crate::{Error, Result};
use kh1_formats::egs::HdAsset;

/// Re:CoM's container that Sora's group loads (PLAYER 1's LINK `31000`).
pub const EFF_PATH: &str = "EFFECT/1000/EF1000.EFF";
/// The manifest the step writes (next to the other manifests).
pub const MANIFEST: &str = "kh1-effects.json";
/// The manifest's `format`.
pub const FORMAT: &str = "kh1-recom-assets effects 1";
/// Sora's KH1 model file and the section holding his effect set.
pub const SORA_MDLS: &str = "xa_ex_0010.mdls";
/// The `.mdls` section of Sora's effect set (Checked (files): magic `0x82`, 8 entries).
pub const SORA_SECTION: usize = 1;
/// Sora's KH1 set's name in Re:CoM.
pub const SORA_SET: &str = "kh1_ex_0010.dpx";

/// One container row.
pub const ROW: usize = 0x20;
/// Longest name Re:CoM reads from a row (`strncpy(…, 0x15)`).
pub const NAME_MAX: usize = 21;
/// Name field bytes (row +4 .. +0x1A).
const NAME_FIELD: usize = 0x16;
/// Sub-file alignment (Re:CoM's own `EF1000.EFF` packs at 16; its sizes are multiples of 16).
pub const ALIGN: usize = 16;
/// An effect set's magic (DPX).
pub const DPX_MAGIC: u32 = 0x82;

/// One sub-file of a container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The sub-file's name (its extension gives Re:CoM its type).
    pub name: String,
    /// The sub-file's bytes.
    pub data: Vec<u8>,
    /// Row bytes +0x1A and +0x1B, kept as Re:CoM wrote them.
    pub flags: [u8; 2],
}

fn bad<T>(m: impl Into<String>) -> Result<T> {
    Err(Error::Format(m.into()))
}

fn u32_at(d: &[u8], at: usize) -> Result<u32> {
    d.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| Error::Format(format!("short read at {at:#x}")))
}

/// Reads a container's rows (until the first row whose offset is 0).
pub fn parse_eff(d: &[u8]) -> Result<Vec<Entry>> {
    let mut out = Vec::new();
    let mut at = 0;
    loop {
        let off = u32_at(d, at)? as usize;
        if off == 0 {
            return Ok(out);
        }
        let name_b = d.get(at + 4..at + ROW).ok_or_else(|| Error::Format(format!("container row at {at:#x} cut short")))?;
        let name_b = &name_b[..NAME_FIELD];
        let len = name_b.iter().position(|&c| c == 0).unwrap_or(NAME_FIELD);
        let name = String::from_utf8_lossy(&name_b[..len]).into_owned();
        let flags = [d[at + 0x1A], d[at + 0x1B]];
        let size = (u32_at(d, at + 0x1C)? & 0x7FFF_FFFF) as usize;
        let data = d.get(off..off + size).ok_or_else(|| Error::Format(format!("container row {name}: {off:#x}+{size:#x} outside the file")))?.to_vec();
        out.push(Entry { name, data, flags });
        at += ROW;
    }
}

/// Writes a container: the rows, a zero row, then each sub-file at 16-byte alignment, each row's
/// size with bit 31 set (as Re:CoM's).
pub fn write_eff(entries: &[Entry]) -> Result<Vec<u8>> {
    let head = (entries.len() + 1) * ROW;
    let mut out = vec![0u8; head];
    for (i, e) in entries.iter().enumerate() {
        if e.name.is_empty() || e.name.len() > NAME_MAX || !e.name.is_ascii() {
            return bad(format!("container name {:?}: 1..={NAME_MAX} ASCII characters", e.name));
        }
        while out.len() % ALIGN != 0 {
            out.push(0);
        }
        let off = out.len();
        let r = i * ROW;
        out[r..r + 4].copy_from_slice(&(off as u32).to_le_bytes());
        out[r + 4..r + 4 + e.name.len()].copy_from_slice(e.name.as_bytes());
        out[r + 0x1A] = e.flags[0];
        out[r + 0x1B] = e.flags[1];
        out[r + 0x1C..r + 0x20].copy_from_slice(&(e.data.len() as u32 | 0x8000_0000).to_le_bytes());
        out.extend_from_slice(&e.data);
    }
    while out.len() % ALIGN != 0 {
        out.push(0);
    }
    Ok(out)
}

/// Section `i` of a KH1 `.mdls` (`u32 count`, then `count + 1` offsets): its start and bytes.
pub fn mdls_section(mdls: &[u8], i: usize) -> Result<(usize, &[u8])> {
    let n = u32_at(mdls, 0)? as usize;
    if i >= n || n > 64 {
        return bad(format!("mdls: section {i} of {n}"));
    }
    let start = u32_at(mdls, 4 + 4 * i)? as usize;
    let end = (u32_at(mdls, 8 + 4 * i)? as usize).min(mdls.len());
    if start >= end {
        return bad(format!("mdls: section {i} empty ({start:#x}..{end:#x})"));
    }
    Ok((start, &mdls[start..end]))
}

/// A set's effect ids (entries `+0x10 + 0x20 × k`, id at `+8`), checking its magic and that every
/// entry's blob lies inside the set.
pub fn set_ids(set: &[u8]) -> Result<Vec<u32>> {
    if u32_at(set, 0)? != DPX_MAGIC {
        return bad(format!("not an effect set (magic {:#x})", u32_at(set, 0)?));
    }
    let n = u32_at(set, 0xC)? as usize;
    if n == 0 || n > 256 {
        return bad(format!("effect set: {n} entries"));
    }
    let mut ids = Vec::with_capacity(n);
    for k in 0..n {
        let e = 0x10 + 0x20 * k;
        let blob = u32_at(set, e)? as usize;
        if blob == 0 || blob >= set.len() {
            return bad(format!("effect set entry {k}: blob {blob:#x} outside the set"));
        }
        ids.push(u32_at(set, e + 8)?);
    }
    Ok(ids)
}

/// The file offset a KH1 PC texture is tagged by (`-4310.dds` → 0x4310).
pub fn kh1_texture_offset(name: &str) -> Option<usize> {
    let hex = name.strip_prefix('-')?.strip_suffix(".dds")?;
    usize::from_str_radix(hex, 16).ok()
}

/// The PC asset name Re:CoM looks up for the texture at `off` in set `set` (`0x1402BBC42`).
pub fn hd_name(set: &str, off: usize) -> String {
    format!("{}{off:x}.dds", set.to_ascii_lowercase().replacen('.', "_", 1))
}

/// The tag written with a new asset: Re:CoM's own `EF1000.EFF` assets carry `0x48` followed by the
/// texture's offset in hex digits (`ex_5010_dpx4290.dds` → `0x484290`; Checked (files)). The name,
/// not the tag, finds the texture on this path (`0x1402BBBC0` with a name); the tag is copied as
/// Square writes it. Assumption: unused for set textures.
pub fn hd_tag(off: usize) -> i32 {
    i64::from_str_radix(&format!("48{off:x}"), 16).map(|v| v as u32 as i32).unwrap_or(0x48)
}

/// A KH1 set taken for Re:CoM.
#[derive(Clone, Debug)]
pub struct Set {
    /// Its name in the container.
    pub name: String,
    /// Where it came from (`xa_ex_0010.mdls` section 1).
    pub source: String,
    /// The set's bytes, unchanged.
    pub data: Vec<u8>,
    /// Its effect ids, in entry order.
    pub ids: Vec<u32>,
    /// Its PC textures, renamed for Re:CoM.
    pub hd: Vec<HdAsset>,
}

/// Takes section `section` of a KH1 `.mdls` as set `name`, with the file's PC textures that lie
/// inside it.
pub fn kh1_set(name: &str, source: &str, mdls: &[u8], section: usize, assets: &[HdAsset]) -> Result<Set> {
    let (start, data) = mdls_section(mdls, section)?;
    let ids = set_ids(data)?;
    let mut hd = Vec::new();
    for a in assets {
        let Some(off) = kh1_texture_offset(&a.name) else { continue };
        if off >= start && off < start + data.len() {
            hd.push(HdAsset { name: hd_name(name, off - start), orig_offset: hd_tag(off - start), data: a.data.clone() });
        }
    }
    Ok(Set { name: name.to_string(), source: format!("{source} section {section}"), data: data.to_vec(), ids, hd })
}

/// Re:CoM's container with `sets` added after its own rows, and its PC assets with the sets'
/// textures added. Refuses a container that already holds a name being added.
pub fn add_sets(eff: &[u8], assets: &[HdAsset], sets: &[Set]) -> Result<(Vec<u8>, Vec<HdAsset>)> {
    let mut entries = parse_eff(eff)?;
    for s in sets {
        let stem = s.name.split('.').next().unwrap_or("");
        if entries.iter().any(|e| e.name.split('.').next() == Some(stem)) {
            return bad(format!("{} already holds {}", EFF_PATH, s.name));
        }
        entries.push(Entry { name: s.name.clone(), data: s.data.clone(), flags: [0, 0] });
    }
    let out = write_eff(&entries)?;
    let mut all = assets.to_vec();
    for s in sets {
        for a in &s.hd {
            if all.iter().any(|b| b.name == a.name) {
                return bad(format!("{EFF_PATH}: PC asset {} twice", a.name));
            }
            all.push(a.clone());
        }
    }
    Ok((out, all))
}

/// The manifest: the container, the sets added (name, source, effect ids, size, PC textures) and
/// the sources' hashes.
pub fn manifest_json(sets: &[Set], sources: &[(&str, String)]) -> String {
    let mut s = format!("{{\n  \"format\": \"{FORMAT}\",\n  \"container\": \"{EFF_PATH}\",\n  \"sets\": [");
    for (i, set) in sets.iter().enumerate() {
        let ids: Vec<String> = set.ids.iter().map(|v| v.to_string()).collect();
        let hd: Vec<String> = set.hd.iter().map(|a| format!("\"{}\"", a.name)).collect();
        s += &format!(
            "{}\n    {{\"name\": \"{}\", \"source\": \"{}\", \"bytes\": {}, \"ids\": [{}], \"hd\": [{}]}}",
            if i > 0 { "," } else { "" },
            set.name,
            set.source,
            set.data.len(),
            ids.join(", "),
            hd.join(", ")
        );
    }
    s += "\n  ],\n  \"sources\": {";
    for (i, (k, v)) in sources.iter().enumerate() {
        s += &format!("{}\n    \"{k}\": \"{v}\"", if i > 0 { "," } else { "" });
    }
    s += "\n  }\n}\n";
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ids: &[u32], len: usize) -> Vec<u8> {
        let mut d = vec![0u8; len];
        d[0..4].copy_from_slice(&DPX_MAGIC.to_le_bytes());
        d[0xC..0x10].copy_from_slice(&(ids.len() as u32).to_le_bytes());
        for (k, id) in ids.iter().enumerate() {
            let e = 0x10 + 0x20 * k;
            d[e..e + 4].copy_from_slice(&0x80u32.to_le_bytes());
            d[e + 8..e + 12].copy_from_slice(&id.to_le_bytes());
        }
        d
    }

    #[test]
    fn container_round_trip_keeps_rows() {
        let a = Entry { name: "ex_0010.dpx".into(), data: set(&[0, 1], 0x100), flags: [0, 0] };
        let b = Entry { name: "ex_0010.ese".into(), data: vec![7; 0x40], flags: [1, 2] };
        let eff = write_eff(&[a.clone(), b.clone()]).unwrap();
        // Two rows and the zero row; the first sub-file right after them.
        assert_eq!(u32_at(&eff, 0).unwrap(), 0x60);
        assert_eq!(u32_at(&eff, 0x1C).unwrap(), 0x8000_0100);
        assert_eq!(u32_at(&eff, 0x40).unwrap(), 0, "the zero row ends the list");
        assert_eq!(parse_eff(&eff).unwrap(), vec![a, b]);
    }

    #[test]
    fn rows_are_aligned_and_names_checked() {
        let odd = Entry { name: "a.dpx".into(), data: vec![1; 0x13], flags: [0, 0] };
        let next = Entry { name: "b.ese".into(), data: vec![2; 4], flags: [0, 0] };
        let eff = write_eff(&[odd, next]).unwrap();
        assert_eq!(u32_at(&eff, 0x20).unwrap() as usize % ALIGN, 0);
        assert!(write_eff(&[Entry { name: "x".repeat(22), data: vec![], flags: [0, 0] }]).is_err(), "22 characters: Re:CoM reads 21");
        assert!(write_eff(&[Entry { name: "kh1_ex_0010.dpx".into(), data: vec![], flags: [0, 0] }]).is_ok());
    }

    #[test]
    fn texture_names_follow_recoms_sprintf() {
        // Re:CoM's own: ex_5010.dpx's texture at 0x4290 is ex_5010_dpx4290.dds (Checked (files)).
        assert_eq!(hd_name("ex_5010.dpx", 0x4290), "ex_5010_dpx4290.dds");
        assert_eq!(hd_name("kh1_ex_0010.dpx", 0x7C00), "kh1_ex_0010_dpx7c00.dds", "lower-case hex, as %x");
        assert_eq!(kh1_texture_offset("-4310.dds"), Some(0x4310));
        assert_eq!(kh1_texture_offset("-9a40.cvbl"), None);
        assert_eq!(kh1_texture_offset("w_ex010_01_tm2.dds"), None);
        // Square's tags in EF1000: 0x480, 0x487C00, 0x484290.
        assert_eq!(hd_tag(0), 0x480);
        assert_eq!(hd_tag(0x7C00), 0x48_7C00);
        assert_eq!(hd_tag(0x4290), 0x48_4290);
    }

    #[test]
    fn a_kh1_section_becomes_a_set_with_its_textures() {
        // An mdls of 3 sections: 0x20.., 0x200.. (the set), 0x400..
        let mut m = vec![0u8; 0x500];
        m[0..4].copy_from_slice(&3u32.to_le_bytes());
        for (i, o) in [0x20u32, 0x200, 0x400, 0x500].iter().enumerate() {
            m[4 + 4 * i..8 + 4 * i].copy_from_slice(&o.to_le_bytes());
        }
        m[0x200..0x300].copy_from_slice(&set(&[0, 1, 2], 0x100));
        let tex = |n: &str| HdAsset { name: n.into(), orig_offset: 0, data: vec![9] };
        let assets = [tex("-210.dds"), tex("-30.dds"), tex("-410.dds"), tex("-220.cvbl")];
        let s = kh1_set(SORA_SET, SORA_MDLS, &m, 1, &assets).unwrap();
        assert_eq!(s.ids, vec![0, 1, 2]);
        assert_eq!(s.data.len(), 0x200);
        let names: Vec<&str> = s.hd.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["kh1_ex_0010_dpx10.dds"], "only the textures inside the section, offset from its start");
        assert!(kh1_set(SORA_SET, SORA_MDLS, &m, 0, &assets).is_err(), "section 0 is not an effect set");
    }

    #[test]
    fn recoms_rows_stay_and_the_set_is_added() {
        let own = [
            Entry { name: "ex_0010.dpx".into(), data: set(&[0], 0x100), flags: [0, 0] },
            Entry { name: "ex_5010.dpx".into(), data: set(&[5], 0x80), flags: [0, 0] },
            Entry { name: "ex_0010.ese".into(), data: vec![3; 0x40], flags: [0, 0] },
        ];
        let eff = write_eff(&own).unwrap();
        let recom_asset = HdAsset { name: "ex_5010_dpx4290.dds".into(), orig_offset: 0x48_4290, data: vec![1] };
        let kh1 = Set { name: SORA_SET.into(), source: "t".into(), data: set(&[0, 1, 6, 7], 0x180), ids: vec![0, 1, 6, 7], hd: vec![HdAsset { name: "kh1_ex_0010_dpx80.dds".into(), orig_offset: hd_tag(0x80), data: vec![2] }] };
        let (out, assets) = add_sets(&eff, std::slice::from_ref(&recom_asset), std::slice::from_ref(&kh1)).unwrap();
        let rows = parse_eff(&out).unwrap();
        assert_eq!(&rows[..3], &own[..], "Re:CoM's rows: same names, bytes and flags");
        assert_eq!(rows[3].name, SORA_SET);
        assert_eq!(rows[3].data, kh1.data);
        assert_eq!(assets[0], recom_asset, "Re:CoM's PC assets kept as they are");
        assert_eq!(assets[1].name, "kh1_ex_0010_dpx80.dds");
        assert!(add_sets(&out, &assets, &[kh1]).is_err(), "never added twice");
    }

    #[test]
    fn the_manifest_lists_the_sets() {
        let s = Set { name: SORA_SET.into(), source: "xa_ex_0010.mdls section 1".into(), data: vec![0; 16], ids: vec![0, 7], hd: vec![] };
        let j = manifest_json(&[s], &[("EF1000.EFF sha256", "ab".into())]);
        assert!(j.contains("\"format\": \"kh1-recom-assets effects 1\""));
        assert!(j.contains("\"name\": \"kh1_ex_0010.dpx\""));
        assert!(j.contains("\"ids\": [0, 7]"));
    }
}
