//! The Moogle shop's pack screen with a fifth tab (M8d, owner 2026-11-04: "Items tab, then
//! packs"). `docs/recom-m8-items.md` § M8d.
//!
//! Re:CoM's pack screen (`CMoogleShopCore` mode 4) draws its tab row from two pieces of
//! `m_shop:01` (SEQD 1 of `m_shop.lad` in `FORM/0109/FO0109.SPR`), Checked (decomp + files: the
//! owner's FO0109.SPR and its `UK_m_shop.lad`):
//!
//! - **The window** (`CMShopCardPackWindow`, `0x14045AD20`): one group per tab, the window with
//!   that tab's notch raised (groups 66 / 68 / 70 / 72, `0x140690420`), and its close (67 / 69 /
//!   71 / 73, `0x140690428`). Each is two animations: the fill (sprite group 139 for tab 0) and the
//!   frame (143). The notch is the parts inside x 29..67 of tab 0's window that rise above the
//!   window's top (y < 0); the top border on each side of it ends at x 29 and starts at 67. The
//!   tabs are 64 apart.
//! - **The icons** (a `CMShopGSeq`, made by mode 4's init `0x1403A4A90` with groups 0x38 / 0x39 /
//!   0x3A = 56 appear, 57 loop, 58 close): the four packs as tinted copies of one sprite group at
//!   x 64, 128, 192 and the "all" stack about 256 (each animation's X translation).
//!
//! This step appends 13 groups to `m_shop:01` (Re:CoM's own groups unchanged): the window and its
//! close for 5 tab places 52 apart (the notch's parts moved, the top border's two ends moved with
//! it) and the icons for 5 tabs: Re:CoM's 4 icons moved one place right and, first, Re:CoM's own
//! item icon (the green bottle of the card trade's tab strip, group 181's 13th animation's sprite
//! group, drawn 2.5 times its size). Only sprites already in the atlas are used: `m_shop.imz` is
//! not touched. The DLL plays them at Combat = KH1 (`kh1-shop-tabs.json`).

use crate::mp_bar::{Container, Layd, Seqd, AGROUP, ANIM, PART, SGROUP};
use crate::{bad, Result};

/// Re:CoM's Moogle shop sprite file.
pub const SPR_PATH: &str = "FORM/0109/FO0109.SPR";
/// The layout in it.
pub const LAYOUT: &str = "m_shop.lad";
/// The sprite set (SEQD 1 of `m_shop.lad`).
pub const SET: &str = "m_shop:01";
/// Its SEQD index in the layout.
pub const SEQ: usize = 1;
/// Re:CoM Steam's animation groups in `m_shop:01` (base file and `UK_m_shop.lad`: Checked (files)).
pub const BASE_GROUPS: usize = 241;
/// The manifest the DLL reads.
pub const MANIFEST: &str = "kh1-shop-tabs.json";
/// The manifest's `format`.
pub const FORMAT: &str = "kh1-recom-assets shop-tabs 1";

/// Re:CoM's tab-0 window (`0x140690420[0]`).
pub const RECOM_WINDOW: usize = 66;
/// Its close (`0x140690428[0]`).
pub const RECOM_CLOSE: usize = 67;
/// The icons' appear / loop / close (mode 4's init: 0x38 / 0x39 / 0x3A).
pub const RECOM_ICONS: [usize; 3] = [56, 57, 58];
/// The card trade's tab strip (`0x1406904D0[4]`: the ALL tab lit).
pub const TRADE_STRIP: usize = 181;
/// Its animation drawing the item icon (the green bottle).
pub const TRADE_ITEM_ANIM: usize = 12;
/// Tab 0's notch: x 29..67 of the window. Checked (files).
pub const NOTCH: (i32, i32) = (29, 67);
/// Re:CoM's tab step (screen x of the icons and the cursor: 64, 128, 192, 256; `0x14065BA40`).
pub const RECOM_STEP: i32 = 64;
/// Ours: 64, 116, 168, 220, 272.
pub const STEP: i32 = 52;
/// Our tab places.
pub const TABS: usize = 5;
/// The item icon's size against Re:CoM's (12 × 15 units → 30 × 37.5).
pub const ICON_SCALE: f32 = 2.5;

/// Our groups, from the first one (`first` in the manifest).
pub mod group {
    /// The window with tab place p raised: WINDOW + p.
    pub const WINDOW: usize = 0;
    /// Its close: CLOSE + p.
    pub const CLOSE: usize = 5;
    /// The icons' appear (Re:CoM's 56 with the item icon first).
    pub const ICONS_IN: usize = 10;
    /// The icons' loop (57).
    pub const ICONS: usize = 11;
    /// The icons' close (58).
    pub const ICONS_OUT: usize = 12;
    /// Groups in all.
    pub const COUNT: usize = 13;
}

fn i32_at(r: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(r[at..at + 4].try_into().unwrap())
}

fn put(r: &mut [u8], at: usize, v: i32) {
    r[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn anims_of(seq: &Seqd, g: usize) -> Result<std::ops::Range<usize>> {
    let ag = seq.agroups.get(g).ok_or_else(|| crate::Error::Format(format!("{SET}: no group {g}")))?;
    let a0 = u16::from_le_bytes([ag[0], ag[1]]) as usize;
    let n = u16::from_le_bytes([ag[2], ag[3]]) as usize;
    if a0 + n > seq.anims.len() {
        return bad(format!("{SET}: group {g} past the animations"));
    }
    Ok(a0..a0 + n)
}

fn parts_of(seq: &Seqd, sg: usize) -> Result<std::ops::Range<usize>> {
    let s = seq.sgroups.get(sg).ok_or_else(|| crate::Error::Format(format!("{SET}: no sprite group {sg}")))?;
    let p0 = i16::from_le_bytes([s[0], s[1]]);
    let n = i16::from_le_bytes([s[2], s[3]]);
    if p0 < 0 || n < 0 || p0 as usize + n as usize > seq.parts.len() {
        return bad(format!("{SET}: sprite group {sg} past the parts"));
    }
    Ok(p0 as usize..p0 as usize + n as usize)
}

/// A part's rect (x0, y0, x1, y1; a mirrored part has x1 < x0).
fn rect(p: &[u8; PART]) -> [i32; 4] {
    [0, 4, 8, 12].map(|o| i32_at(p, o))
}

/// Tab 0's window part moved to tab place `dx` further: the notch's parts (inside the notch's x
/// range and above the window's top) move by `dx`; the top border's ends at the notch move with it.
/// True when the part changed.
pub fn move_notch(r: &mut [i32; 4], dx: i32) -> bool {
    let (l, rr) = NOTCH;
    let (xa, xb) = (r[0].min(r[2]), r[0].max(r[2]));
    let top = r[1].min(r[3]);
    if xa >= l && xb <= rr && top < 0 {
        r[0] += dx;
        r[2] += dx;
        return true;
    }
    if top > 0 {
        return false;
    }
    let mut changed = false;
    for k in [0, 2] {
        let other = r[2 - k];
        if r[k] == l && other < l {
            r[k] = l + dx;
            changed = true;
        } else if r[k] == rr && other > rr {
            r[k] = rr + dx;
            changed = true;
        }
    }
    changed
}

/// A copy of sprite group `sg` with every part through `f`; its index.
fn copy_sg(seq: &mut Seqd, sg: usize, f: impl Fn(&mut [i32; 4])) -> Result<usize> {
    let parts: Vec<[u8; PART]> = parts_of(seq, sg)?.map(|i| seq.parts[i]).collect();
    let first = seq.parts.len();
    for mut p in parts.iter().copied() {
        let mut r = rect(&p);
        f(&mut r);
        for (k, o) in [0, 4, 8, 12].into_iter().enumerate() {
            put(&mut p, o, r[k]);
        }
        seq.parts.push(p);
    }
    new_sg(seq, first, parts.len())
}

fn new_sg(seq: &mut Seqd, first: usize, n: usize) -> Result<usize> {
    if first + n > i16::MAX as usize || seq.sgroups.len() >= i16::MAX as usize {
        return bad("seqd: too many parts for 16-bit indices");
    }
    let mut r = [0u8; SGROUP];
    r[..2].copy_from_slice(&(first as i16).to_le_bytes());
    r[2..].copy_from_slice(&(n as i16).to_le_bytes());
    seq.sgroups.push(r);
    Ok(seq.sgroups.len() - 1)
}

/// Appends a group of `anims` with the words of Re:CoM's group `like`; its index.
fn push_group(seq: &mut Seqd, like: usize, anims: Vec<[u8; ANIM]>) -> Result<usize> {
    let first = seq.anims.len();
    if first + anims.len() > i16::MAX as usize {
        return bad("seqd: too many animations for 16-bit indices");
    }
    let n = anims.len();
    seq.anims.extend(anims);
    let mut r: [u8; AGROUP] = seq.agroups[like];
    r[0..2].copy_from_slice(&(first as u16).to_le_bytes());
    r[2..4].copy_from_slice(&(n as u16).to_le_bytes());
    seq.agroups.push(r);
    Ok(seq.agroups.len() - 1)
}

/// Which of Re:CoM's 4 tab places an icon animation's X translation is at (64, 128, 192, about 256).
pub fn icon_place(tx: i32) -> usize {
    (((tx - RECOM_STEP) as f32 / RECOM_STEP as f32).round().clamp(0.0, 3.0)) as usize
}

/// An icon's X translation at our place `p` (0 = Items) for Re:CoM's at place `k`.
pub fn moved_tx(tx: i32, p: usize) -> i32 {
    let k = icon_place(tx);
    tx - (RECOM_STEP + RECOM_STEP * k as i32) + (RECOM_STEP + STEP * p as i32)
}

/// Appends our 13 groups to `m_shop:01`; their first index. Refused unless the set is Re:CoM
/// Steam's (exactly 241 groups: a fed-back output has more), tab 0's window draws the notch where
/// this step expects it, the icons' groups have 6 animations each, and the trade strip's item
/// animation draws a one-part sprite group.
pub fn add_tabs(seq: &mut Seqd) -> Result<usize> {
    if seq.agroups.len() != BASE_GROUPS {
        return bad(format!("{SET} has {} animation groups, Re:CoM Steam's has {BASE_GROUPS} (already patched?)", seq.agroups.len()));
    }
    let first = seq.agroups.len();
    // The window: its two animations and their sprite groups.
    let win: Vec<usize> = anims_of(seq, RECOM_WINDOW)?.collect();
    let close: Vec<usize> = anims_of(seq, RECOM_CLOSE)?.collect();
    let sgs: Vec<usize> = win.iter().map(|&a| i32_at(&seq.anims[a], 4) as usize).collect();
    if win.len() != 2 || close.len() != 2 || close.iter().map(|&a| i32_at(&seq.anims[a], 4) as usize).collect::<Vec<_>>() != sgs {
        return bad(format!("{SET}: groups {RECOM_WINDOW} / {RECOM_CLOSE} are not two animations on the same sprite groups"));
    }
    let notch = sgs.iter().map(|&sg| parts_of(seq, sg)).collect::<Result<Vec<_>>>()?.into_iter().flatten().any(|i| {
        let r = rect(&seq.parts[i]);
        r[0].min(r[2]) == NOTCH.0 && r[0].max(r[2]) == NOTCH.1 && r[1].min(r[3]) < 0
    });
    if !notch {
        return bad(format!("{SET}: group {RECOM_WINDOW} has no notch part at x {}..{}", NOTCH.0, NOTCH.1));
    }
    let mut placed = Vec::new();
    for p in 0..TABS {
        let dx = STEP * p as i32;
        let mut new = Vec::new();
        for &sg in &sgs {
            new.push(copy_sg(seq, sg, |r| {
                move_notch(r, dx);
            })?);
        }
        placed.push(new);
    }
    let with_sgs = |seq: &Seqd, anims: &[usize], new: &[usize]| -> Vec<[u8; ANIM]> {
        anims
            .iter()
            .zip(new)
            .map(|(&a, &sg)| {
                let mut r = seq.anims[a];
                put(&mut r, 4, sg as i32);
                r
            })
            .collect()
    };
    for new in &placed {
        let a = with_sgs(seq, &win, new);
        push_group(seq, RECOM_WINDOW, a)?;
    }
    for new in &placed {
        let a = with_sgs(seq, &close, new);
        push_group(seq, RECOM_CLOSE, a)?;
    }
    // The item icon: the trade strip's item animation's sprite group, scaled, in one new part.
    let strip: Vec<usize> = anims_of(seq, TRADE_STRIP)?.collect();
    let item_anim = *strip.get(TRADE_ITEM_ANIM).ok_or_else(|| crate::Error::Format(format!("{SET}: group {TRADE_STRIP} has no animation {TRADE_ITEM_ANIM}")))?;
    let item_sg = i32_at(&seq.anims[item_anim], 4) as usize;
    let ip: Vec<usize> = parts_of(seq, item_sg)?.collect();
    if ip.len() != 1 {
        return bad(format!("{SET}: the item icon's sprite group {item_sg} has {} parts", ip.len()));
    }
    let icon_sg = copy_sg(seq, item_sg, |r| {
        for v in r.iter_mut() {
            *v = (*v as f32 * ICON_SCALE).round() as i32;
        }
    })?;
    for &g in &RECOM_ICONS {
        let anims: Vec<usize> = anims_of(seq, g)?.collect();
        if anims.len() != 6 {
            return bad(format!("{SET}: icon group {g} has {} animations (Re:CoM's has 6)", anims.len()));
        }
        let mut out = Vec::with_capacity(7);
        // Ours first: the first pack icon's animation (its fades), on the item icon, untinted.
        let mut item = seq.anims[anims[0]];
        put(&mut item, 4, icon_sg as i32);
        put(&mut item, 0x10, RECOM_STEP);
        put(&mut item, 0x14, RECOM_STEP);
        put(&mut item, 0x18, 0);
        put(&mut item, 0x1C, 0);
        let neutral = |c: u32| (c & 0xFF00_0000) | 0x0080_8080;
        let (c0, c1) = (i32_at(&item, 0x88) as u32, i32_at(&item, 0x8C) as u32);
        put(&mut item, 0x88, neutral(c0) as i32);
        put(&mut item, 0x8C, neutral(c1) as i32);
        out.push(item);
        for &a in &anims {
            let mut r = seq.anims[a];
            let k = icon_place(i32_at(&r, 0x10));
            for o in [0x10, 0x14] {
                let tx = i32_at(&r, o);
                put(&mut r, o, moved_tx(tx, k + 1));
            }
            out.push(r);
        }
        push_group(seq, g, out)?;
    }
    debug_assert_eq!(seq.agroups.len(), first + group::COUNT);
    Ok(first)
}

/// Adds the tabs to an `m_shop.lad`; the layout and our first group.
pub fn patch_layout(lad: &[u8]) -> Result<(Vec<u8>, usize)> {
    let mut l = Layd::parse(lad)?;
    let raw = l.seqs.get(SEQ).ok_or_else(|| crate::Error::Format(format!("{LAYOUT}: no {SET}")))?;
    let mut s = Seqd::parse(raw)?;
    let g = add_tabs(&mut s)?;
    l.seqs[SEQ] = s.write();
    Ok((l.write(), g))
}

/// An `m_shop.lad` name: `m_shop.lad` or a language's `<LANG>_m_shop.lad`.
pub fn is_shop_layout(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == LAYOUT || (n.len() == LAYOUT.len() + 3 && n.ends_with(LAYOUT) && n.as_bytes()[2] == b'_')
}

/// Adds the tabs to the `m_shop.lad` inside an `FO0109.SPR` container.
pub fn patch_container(spr: &[u8]) -> Result<(Vec<u8>, usize)> {
    let mut c = Container::parse(spr)?;
    let e = c.find_mut(LAYOUT).ok_or_else(|| crate::Error::Format(format!("{SPR_PATH}: no {LAYOUT}")))?;
    let (lad, g) = patch_layout(&e.data)?;
    e.data = lad;
    Ok((c.write(), g))
}

/// Patches the `FO0109.SPR` container and every PC asset of its entry that is an `m_shop.lad`
/// (any language prefix) or a whole container with one; our first group (the same in every copy:
/// each must have Re:CoM's 241) and the files patched.
pub fn patch_spr(spr: &mut Vec<u8>, assets: &mut [kh1_formats::egs::HdAsset]) -> Result<(usize, Vec<String>)> {
    let (out, g) = patch_container(spr)?;
    *spr = out;
    let mut files = vec![SPR_PATH.to_string()];
    for a in assets.iter_mut() {
        let patched = if a.data.starts_with(b"LAYD") {
            if !is_shop_layout(&a.name) {
                continue;
            }
            patch_layout(&a.data)
        } else {
            match Container::parse(&a.data) {
                Ok(c) if c.find(LAYOUT).is_some() => patch_container(&a.data),
                _ => continue,
            }
        };
        let (d, f) = patched.map_err(|e| crate::Error::Format(format!("{SPR_PATH} asset {}: {e}", a.name)))?;
        if f != g {
            return bad(format!("{SPR_PATH} asset {}: first group {f}, the base file's {g}", a.name));
        }
        a.data = d;
        files.push(a.name.clone());
    }
    Ok((g, files))
}

/// `kh1-shop-tabs.json`.
pub fn manifest_json(first: usize, files: &[String], sources: &[(&str, String)]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"format\": \"{FORMAT}\",");
    let _ = writeln!(s, "  \"set\": \"{SET}\",");
    let _ = writeln!(s, "  \"base_groups\": {BASE_GROUPS},");
    let _ = writeln!(s, "  \"first\": {first},");
    let _ = writeln!(s, "  \"count\": {},", group::COUNT);
    let _ = writeln!(s, "  \"tabs\": {TABS},");
    let _ = writeln!(s, "  \"step\": {STEP},");
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

    #[test]
    fn the_notch_moves_and_the_border_follows() {
        // Base file's tab-0 parts (Checked (files)).
        let mut r = [47, -24, 63, -8];
        assert!(move_notch(&mut r, 52));
        assert_eq!(r, [99, -24, 115, -8]);
        let mut r = [47, -24, 31, -8];
        move_notch(&mut r, 52);
        assert_eq!(r, [99, -24, 83, -8]);
        let mut r = [29, -25, 67, 4];
        move_notch(&mut r, 104);
        assert_eq!(r, [133, -25, 171, 4]);
        let mut r = [13, 0, 29, 13];
        move_notch(&mut r, 52);
        assert_eq!(r, [13, 0, 81, 13]);
        let mut r = [67, 0, 292, 13];
        move_notch(&mut r, 52);
        assert_eq!(r, [119, 0, 292, 13]);
        // The UK copy's top border rises above 0 too.
        let mut r = [67, -25, 292, 13];
        move_notch(&mut r, 52);
        assert_eq!(r, [119, -25, 292, 13]);
        // Untouched: the fill's top row, the sides, the bottom.
        for mut r in [[13, 0, 292, 13], [0, 13, 13, 175], [13, 188, 292, 175], [178, 116, 227, 187]] {
            let before = r;
            assert!(!move_notch(&mut r, 52));
            assert_eq!(r, before);
        }
        // p = 0 changes nothing; the last notch ends inside the window (292).
        let mut r = [29, -25, 67, 4];
        move_notch(&mut r, 0);
        assert_eq!(r, [29, -25, 67, 4]);
        assert!(NOTCH.1 + STEP * (TABS as i32 - 1) < 292);
    }

    #[test]
    fn icons_move_one_place_right() {
        assert_eq!([64, 128, 192, 249, 256, 263].map(icon_place), [0, 1, 2, 3, 3, 3]);
        assert_eq!(moved_tx(64, 1), 116);
        assert_eq!(moved_tx(128, 2), 168);
        assert_eq!(moved_tx(192, 3), 220);
        assert_eq!([249, 256, 263].map(|t| moved_tx(t, 4)), [265, 272, 279]);
    }

    /// The owner's FO0109.SPR and UK_m_shop.lad (`KH1_FO0109` = a folder with the dump's
    /// `FORM_0109_FO0109.SPR` and `FORM_0109_FO0109.SPR.hd04UK_m_shop.lad`).
    #[test]
    fn owner_shop_files() {
        let Ok(dir) = std::env::var("KH1_FO0109") else { return };
        let dir = std::path::Path::new(&dir);
        let spr = std::fs::read(dir.join("FORM_0109_FO0109.SPR")).unwrap();
        // The container reads back byte for byte.
        assert_eq!(Container::parse(&spr).unwrap().write(), spr);
        let (out, g) = patch_container(&spr).unwrap();
        assert_eq!(g, BASE_GROUPS);
        let c = Container::parse(&out).unwrap();
        let l = Layd::parse(&c.find(LAYOUT).unwrap().data).unwrap();
        let s = Seqd::parse(&l.seqs[SEQ]).unwrap();
        assert_eq!(s.agroups.len(), BASE_GROUPS + group::COUNT);
        // Fed back: refused.
        assert!(patch_container(&out).is_err());
        // Re:CoM's own groups are unchanged.
        let before = Seqd::parse(&Layd::parse(&Container::parse(&spr).unwrap().find(LAYOUT).unwrap().data).unwrap().seqs[SEQ]).unwrap();
        assert_eq!(s.agroups[..BASE_GROUPS], before.agroups[..]);
        assert_eq!(s.anims[..before.anims.len()], before.anims[..]);
        assert_eq!(s.parts[..before.parts.len()], before.parts[..]);
        // The window at place 0 draws Re:CoM's tab-0 window (same parts, copied).
        let uk = dir.join("FORM_0109_FO0109.SPR.hd04UK_m_shop.lad");
        if let Ok(lad) = std::fs::read(uk) {
            let (_, g) = patch_layout(&lad).unwrap();
            assert_eq!(g, BASE_GROUPS);
        }
    }
}
