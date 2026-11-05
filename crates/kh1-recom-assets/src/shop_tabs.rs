//! The Moogle shop's pack screen with a fifth tab (M8d, owner 2026-11-04: "Items tab, then
//! packs") and the card trade's tab strip with a sixth cell (M8d step 3, [`strip`]).
//! `docs/recom-m8-items.md` § M8d.
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
pub const FORMAT: &str = "kh1-recom-assets shop-tabs 2";

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
    /// The card trade's six-cell tab strip with Re:CoM's tab t lit (t = 0..4 as `0x1406904D0`'s
    /// index; 4 = ALL): STRIP + t.
    pub const STRIP: usize = 13;
    /// The strip with the Items cell lit.
    pub const STRIP_ITEMS: usize = 18;
    /// Groups in all.
    pub const COUNT: usize = 19;
}

/// The card trade's tab strip (M8d step 3, owner 2026-11-04: the sell screen's Items tab "uses the
/// same green bottle icon, placed first in the trade's strip"). Re:CoM's strip
/// (`CMShopStockCardWnd`, played from `0x1406904D0[tab]`: groups 182 / 185 / 184 / 183 / 181 for
/// tabs 0..3 and ALL) is one group per lit tab, 14 animations each, Checked (files: the owner's
/// FO0109.SPR and `UK_m_shop.lad`):
///
/// | anim | sprite group | what |
/// |---|---|---|
/// | 0 | 37 | an empty point |
/// | 1 | 126 | the left cell's bottom corner fill (lit with ALL) |
/// | 2 | 125 | the bar under the tabs (the window's top, x −82..83) |
/// | 3..7 | 82 / 84 / 86 / 88 / 90 | the cell fills: ALL, tab 0, tab 3, tab 2, tab 1 (lit 0x80640000, else 0x80280000) |
/// | 8 | 83 / 85 / 91 / 89 / 87 | the outline |
/// | 9..13 | 95 / 98 / 101 / 100 / 99 | the "ALL" text and the 4 icons, placed by X translation |
///
/// The cells: ALL x −82..−26, then tabs 0..3 24 wide from −26. Ours keep Re:CoM's four tabs where
/// they are (their icons, fills, outlines and Re:CoM's finger places unchanged) and split ALL's
/// place in two: the Items cell at −82..−58 (Re:CoM's bottle, anim 12's sprite group, at −70; tab
/// 0's fill moved there) and ALL at −58..−26 (its fill's middle shortened, its text at −42 made
/// narrower to fit). The outline comes in two styles:
///
/// - **The PS2 layout** (`m_shop.lad` in FO0109.SPR: thin pieces, sprites 239 / 240 between
///   cells): Re:CoM's outline of the lit tab (the ALL-lit one for the Items cell) and a separator at
///   −58.
/// - **The PC layouts** (`<LANG>_m_shop.lad`: whole pieces; the lit tab's outline is one piece from
///   the left edge with the bar's top line and the raised tab): the pieces for the Items and ALL
///   cells are cut from Re:CoM's own pieces (new sprite records over the same atlas rectangles:
///   `m_shop.imz` is not touched) and the bar line after them is stretched as Re:CoM stretches it.
pub mod strip {
    /// Re:CoM's strips by tab (`0x1406904D0`, Checked (exe)).
    pub const SOURCES: [usize; 5] = [182, 185, 184, 183, 181];
    /// The ALL tab's index (the Items-lit strip is made from its strip).
    pub const ALL: usize = 4;
    /// Animations in each of Re:CoM's strips.
    pub const ANIMS: usize = 14;
    /// The strip animations by role (the table above).
    pub const CORNER: usize = 1;
    /// The cell fills.
    pub const FILLS: std::ops::Range<usize> = 3..8;
    /// ALL's fill.
    pub const ALL_FILL: usize = 3;
    /// Tab 0's fill: the Items cell's fill is a copy of it.
    pub const TAB0_FILL: usize = 4;
    /// The outline.
    pub const OUTLINE: usize = 8;
    /// The "ALL" text.
    pub const ALL_TEXT: usize = 9;
    /// The bottle (Re:CoM's item-card tab icon).
    pub const BOTTLE: usize = 12;
    /// Re:CoM's ALL cell (x).
    pub const ALL_CELL: (i32, i32) = (-82, -26);
    /// Ours.
    pub const ITEMS: (i32, i32) = (-82, -58);
    /// Our ALL cell.
    pub const NEW_ALL: (i32, i32) = (-58, -26);
    /// Tab 0's fill (x −27..−1 in the PS2 layout, −26..0 in the PC ones) moved to the Items cell.
    pub const ITEM_FILL_DX: i32 = -56;
    /// The bottle's X translation (the cell's middle).
    pub const ITEM_ICON_TX: i32 = -70;
    /// The ALL text's.
    pub const ALL_TEXT_TX: i32 = -42;
    /// The ALL text's width at most (the cell less 2 each side).
    pub const ALL_TEXT_W: i32 = 28;
    /// Its height scale at least.
    pub const ALL_TEXT_MIN_SY: f32 = 0.8;
    /// ALL's fill: its left corner (x ≤ −69) moves with the cell's left edge (+24), its right
    /// corner (x ≥ −38, [`FILL_RIGHT`]) stays, the middle shortens.
    pub const FILL_LEFT: i32 = -69;
    /// ALL's fill's right corner.
    pub const FILL_RIGHT: i32 = -38;
    /// The fills' colours (Checked (files)): lit.
    pub const LIT: u32 = 0x8064_0000;
    /// Unlit.
    pub const DIM: u32 = 0x8028_0000;
    /// The PS2 layout's separator sprites.
    pub const SEPARATORS: [i32; 2] = [239, 240];
    /// Re:CoM's finger x for ALL (`0x14045B950`: its layout point − 62.0), and ours: the same
    /// place in the new ALL cell (8 left of its middle, as Re:CoM's is 8 left of its cell's middle
    /// −54) = − 50.0, a constant already in Re:CoM's float pool. The Items finger at the bottle (as
    /// Re:CoM's tab fingers −14 / 10 / 34 / 58 sit at their icons' −14 / 11 / 35 / 57). Checked
    /// (disasm); **Assumption**: the finger's x is in the strip's coordinates.
    pub const RECOM_ALL_FINGER: i32 = -62;
    /// Ours for ALL.
    pub const ALL_FINGER: i32 = -50;
    /// Ours for the Items cell.
    pub const ITEMS_FINGER: i32 = ITEM_ICON_TX;

    /// ALL's fill part x.
    pub fn all_fill_x(x: i32) -> i32 {
        let shift = NEW_ALL.0 - ALL_CELL.0;
        if x <= FILL_LEFT {
            x + shift
        } else if x >= FILL_RIGHT {
            x
        } else {
            let (a, b) = ((FILL_LEFT + shift) as f32, FILL_RIGHT as f32);
            (a + (x - FILL_LEFT) as f32 * (b - a) / (FILL_RIGHT - FILL_LEFT) as f32).round() as i32
        }
    }

    /// The ALL text's scale for its width `w`.
    pub fn all_text_scale(w: i32) -> (f32, f32) {
        let sx = if w > ALL_TEXT_W { ALL_TEXT_W as f32 / w as f32 } else { 1.0 };
        (sx, sx.max(ALL_TEXT_MIN_SY))
    }

    /// The PC layouts' pieces, found by their rectangles (x0, y0, x1, y1). Checked (files:
    /// `UK_m_shop.lad`).
    pub mod pc {
        /// ALL unlit, with the strip's left side down to the bar (tab 0's strip).
        pub const ALL_UNLIT: [i32; 4] = [-82, -104, -26, -74];
        /// ALL lit: its raised cell from the left edge and the bar's line to −2 (the ALL strip).
        pub const ALL_LIT: [i32; 4] = [-82, -104, -2, -74];
        /// The bar's line after it, a one-pixel column stretched (the ALL strip).
        pub const LINE: [i32; 4] = [-2, -104, 70, -74];
        /// Tab 0 lit: the bar's left corner and line, the raised tab 0 (−26..−2), the line to 22.
        pub const TAB0_LIT: [i32; 4] = [-82, -104, 22, -74];
        /// An unlit tab (tab 0's in the ALL strip).
        pub const CELL_UNLIT: [i32; 4] = [-26, -104, -2, -86];
    }
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

/// Appends our 19 groups to `m_shop:01` (the pack screen's 13, then the card trade's 6 strips);
/// their first index. Refused unless the set is Re:CoM
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
    let _ = add_strips(seq)?;
    debug_assert_eq!(seq.agroups.len(), first + group::COUNT);
    Ok(first)
}

fn set_colour(r: &mut [u8; ANIM], c: u32) {
    put(r, 0x88, c as i32);
    put(r, 0x8C, c as i32);
}

/// A new sprite group of `parts`; its index.
fn push_parts(seq: &mut Seqd, parts: Vec<[u8; PART]>) -> Result<usize> {
    let first = seq.parts.len();
    let n = parts.len();
    seq.parts.extend(parts);
    new_sg(seq, first, n)
}

fn with_rect(mut p: [u8; PART], r: [i32; 4]) -> [u8; PART] {
    for (k, o) in [0, 4, 8, 12].into_iter().enumerate() {
        put(&mut p, o, r[k]);
    }
    p
}

/// The outline style of a layout's strips ([`strip`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// `m_shop.lad` in FO0109.SPR.
    Ps2,
    /// The PKG entry's `<LANG>_m_shop.lad`.
    Pc,
}

/// A part's sprite.
fn sprite_of(p: &[u8; PART]) -> i32 {
    i32_at(&p[..], 16)
}

/// A new sprite record: `part`'s sprite with its U range cut to the part's x `xa..xb` (`xa == xb`:
/// a one-pixel column, as Re:CoM's stretched bar line); its index.
fn cut_sprite(seq: &mut Seqd, part: &[u8; PART], xa: i32, xb: i32) -> Result<i32> {
    let r = rect(part);
    let s = sprite_of(part);
    let Some(&rec) = usize::try_from(s).ok().and_then(|i| seq.sprites.get(i)) else {
        return bad(format!("{SET}: part's sprite {s} past the sprites"));
    };
    if r[2] <= r[0] {
        return bad(format!("{SET}: strip piece {r:?} is mirrored"));
    }
    let (u0, u1) = (i32_at(&rec, 4), i32_at(&rec, 12));
    let u = |x: i32| u0 + ((x - r[0]) as f32 * (u1 - u0) as f32 / (r[2] - r[0]) as f32).round() as i32;
    let mut new = rec;
    put(&mut new, 4, u(xa));
    put(&mut new, 12, u(xb));
    if seq.sprites.len() >= i16::MAX as usize {
        return bad("seqd: too many sprites");
    }
    seq.sprites.push(new);
    Ok(seq.sprites.len() as i32 - 1)
}

/// `piece` cut to x `xa..xb` and drawn at `xa2..xb2` (its own y).
fn cut_piece(seq: &mut Seqd, piece: &[u8; PART], (xa, xb): (i32, i32), (xa2, xb2): (i32, i32)) -> Result<[u8; PART]> {
    let s = cut_sprite(seq, piece, xa, xb)?;
    let r = rect(piece);
    let mut p = with_rect(*piece, [xa2, r[1], xb2, r[3]]);
    put(&mut p, 16, s);
    Ok(p)
}

/// A piece cut in three (its left to `l`, a one-pixel column at `l`, its right from `r`) and
/// drawn over `to`: the corners keep their size, the middle stretches.
fn three_slice(seq: &mut Seqd, piece: &[u8; PART], l: i32, r: i32, to: (i32, i32)) -> Result<Vec<[u8; PART]>> {
    let pr = rect(piece);
    let (lw, rw) = (l - pr[0], pr[2] - r);
    Ok(vec![
        cut_piece(seq, piece, (pr[0], l), (to.0, to.0 + lw))?,
        cut_piece(seq, piece, (l, l), (to.0 + lw, to.1 - rw))?,
        cut_piece(seq, piece, (r, pr[2]), (to.1 - rw, to.1))?,
    ])
}

fn find_piece(parts: &[[u8; PART]], want: [i32; 4], what: &str) -> Result<[u8; PART]> {
    parts.iter().copied().find(|p| rect(p) == want).ok_or_else(|| crate::Error::Format(format!("{SET}: no PC strip piece {what} at {want:?}")))
}

/// The card trade's six-cell strips ([`strip`]): Re:CoM's tab t lit (t = 0..4), then the Items cell
/// lit. Refused unless each of Re:CoM's strips has its 14 animations and its outline is one of the
/// two styles this step knows.
fn add_strips(seq: &mut Seqd) -> Result<Style> {
    use strip::*;
    let mut srcs = Vec::new();
    for &g in &SOURCES {
        let a: Vec<usize> = anims_of(seq, g)?.collect();
        if a.len() != ANIMS {
            return bad(format!("{SET}: strip group {g} has {} animations (Re:CoM's has {ANIMS})", a.len()));
        }
        srcs.push(a);
    }
    let sg_of = |seq: &Seqd, a: usize| i32_at(&seq.anims[a], 4) as usize;
    let parts_in = |seq: &Seqd, sg: usize| -> Result<Vec<[u8; PART]>> { Ok(parts_of(seq, sg)?.map(|i| seq.parts[i]).collect()) };
    let outlines: Vec<Vec<[u8; PART]>> = srcs.iter().map(|s| parts_in(seq, sg_of(seq, s[OUTLINE]))).collect::<Result<_>>()?;
    let style = if SEPARATORS.iter().all(|&sp| outlines[ALL].iter().any(|p| sprite_of(p) == sp)) { Style::Ps2 } else { Style::Pc };
    // The outlines, by lit cell (Some(t): Re:CoM's tab t; None: the Items cell).
    let mut outline_parts: Vec<Vec<[u8; PART]>> = Vec::new();
    match style {
        Style::Ps2 => {
            let mut separator = Vec::new();
            for sp in SEPARATORS {
                let p = *outlines[ALL].iter().find(|p| sprite_of(p) == sp).expect("found above");
                let r = rect(&p);
                let c = (r[0] + r[2]) / 2;
                separator.push(with_rect(p, [r[0] - c + ITEMS.1, r[1], r[2] - c + ITEMS.1, r[3]]));
            }
            for lit in (0..SOURCES.len()).map(Some).chain([None]) {
                let mut v = outlines[lit.unwrap_or(ALL)].clone();
                v.extend(separator.iter().copied());
                outline_parts.push(v);
            }
        }
        Style::Pc => {
            let all_unlit = find_piece(&outlines[0], pc::ALL_UNLIT, "ALL unlit")?;
            let tab0_lit = find_piece(&outlines[0], pc::TAB0_LIT, "tab 0 lit")?;
            let all_lit = find_piece(&outlines[ALL], pc::ALL_LIT, "ALL lit")?;
            let line = find_piece(&outlines[ALL], pc::LINE, "bar line")?;
            let cell = find_piece(&outlines[ALL], pc::CELL_UNLIT, "unlit tab")?;
            // The Items cell unlit: Re:CoM's unlit ALL (the strip's left side down to the bar) cut
            // to 24; ALL unlit: an unlit tab stretched to 32 (both by their middles).
            let items_unlit = three_slice(seq, &all_unlit, ITEMS.0 + 10, ALL_CELL.1 - 10, ITEMS)?;
            let all_unlit_new = three_slice(seq, &cell, pc::CELL_UNLIT[0] + 10, pc::CELL_UNLIT[2] - 14, NEW_ALL)?;
            // ALL lit: tab 0's lit piece's left corner, its line, and its raised tab (−26..+2)
            // moved 32 left and stretched by its middle to ALL's 32; the line from there.
            let mut all_lit_new = vec![
                cut_piece(seq, &tab0_lit, (-82, -70), (-82, -70))?,
                cut_piece(seq, &tab0_lit, (-60, -60), (-70, NEW_ALL.0))?,
            ];
            all_lit_new.push(cut_piece(seq, &tab0_lit, (-26, -14), (NEW_ALL.0, NEW_ALL.0 + 12))?);
            all_lit_new.push(cut_piece(seq, &tab0_lit, (-14, -14), (NEW_ALL.0 + 12, NEW_ALL.1 - 12))?);
            all_lit_new.push(cut_piece(seq, &tab0_lit, (-14, 2), (NEW_ALL.1 - 12, NEW_ALL.1 + 4))?);
            // The Items cell lit: ALL lit's left (the strip's side, the raised top) and its raised
            // cell's right end (−34..−22) moved 32 left, meeting at −66; the line from there.
            let items_lit = vec![
                cut_piece(seq, &all_lit, (-82, -66), (-82, -66))?,
                cut_piece(seq, &all_lit, (-34, -22), (-66, -54))?,
            ];
            let line_from = |x: i32| {
                let r = rect(&line);
                with_rect(line, [x, r[1], r[2], r[3]])
            };
            for lit in (0..SOURCES.len()).map(Some).chain([None]) {
                let base = &outlines[lit.unwrap_or(ALL)];
                let v: Vec<[u8; PART]> = match lit {
                    Some(t) if t < ALL => {
                        let mut v: Vec<_> = base.iter().copied().filter(|p| rect(p) != pc::ALL_UNLIT).collect();
                        if v.len() + 1 != base.len() {
                            return bad(format!("{SET}: strip group {}'s outline has no single ALL-unlit piece", SOURCES[t]));
                        }
                        v.extend(items_unlit.iter().copied());
                        v.extend(all_unlit_new.iter().copied());
                        v
                    }
                    _ => {
                        let mut v: Vec<_> = base.iter().copied().filter(|p| rect(p) != pc::ALL_LIT && rect(p) != pc::LINE).collect();
                        if v.len() + 2 != base.len() {
                            return bad(format!("{SET}: strip group {}'s outline has no single ALL-lit piece and line", SOURCES[ALL]));
                        }
                        if lit.is_some() {
                            v.extend(items_unlit.iter().copied());
                            v.extend(all_lit_new.iter().copied());
                            v.push(line_from(NEW_ALL.1 + 4));
                        } else {
                            v.extend(items_lit.iter().copied());
                            v.extend(all_unlit_new.iter().copied());
                            v.push(line_from(-54));
                        }
                        v
                    }
                };
                outline_parts.push(v);
            }
        }
    }
    // The Items cell's fill: tab 0's, moved to the cell.
    let tab0 = parts_in(seq, sg_of(seq, srcs[ALL][TAB0_FILL]))?;
    let item_fill: Vec<[u8; PART]> = tab0
        .iter()
        .map(|&p| {
            let r = rect(&p);
            with_rect(p, [r[0] + ITEM_FILL_DX, r[1], r[2] + ITEM_FILL_DX, r[3]])
        })
        .collect();
    let item_fill_sg = push_parts(seq, item_fill)?;
    // ALL's fill and text (the same in every strip).
    let all_fill_sg = copy_sg(seq, sg_of(seq, srcs[ALL][ALL_FILL]), |r| {
        r[0] = all_fill_x(r[0]);
        r[2] = all_fill_x(r[2]);
    })?;
    let text = parts_in(seq, sg_of(seq, srcs[ALL][ALL_TEXT]))?;
    let w = text.iter().map(|p| rect(p)).map(|r| r[0].max(r[2])).max().unwrap_or(0) - text.iter().map(|p| rect(p)).map(|r| r[0].min(r[2])).min().unwrap_or(0);
    let (sx, sy) = all_text_scale(w);
    let text_sg = copy_sg(seq, sg_of(seq, srcs[ALL][ALL_TEXT]), |r| {
        r[0] = (r[0] as f32 * sx).round() as i32;
        r[2] = (r[2] as f32 * sx).round() as i32;
        r[1] = (r[1] as f32 * sy).round() as i32;
        r[3] = (r[3] as f32 * sy).round() as i32;
    })?;
    for (k, lit) in (0..SOURCES.len()).map(Some).chain([None]).enumerate() {
        let src = &srcs[lit.unwrap_or(ALL)];
        let items = lit.is_none();
        let mut out = Vec::with_capacity(ANIMS + 2);
        for (i, &a) in src.iter().enumerate() {
            let mut r = seq.anims[a];
            if i == CORNER {
                set_colour(&mut r, if items { LIT } else { DIM });
            } else if i == ALL_FILL {
                put(&mut r, 4, all_fill_sg as i32);
                if items {
                    set_colour(&mut r, DIM);
                }
            } else if i == OUTLINE {
                // The Items cell's fill goes under the outline.
                let mut fill = seq.anims[src[TAB0_FILL]];
                put(&mut fill, 4, item_fill_sg as i32);
                set_colour(&mut fill, if items { LIT } else { DIM });
                out.push(fill);
                let sg = push_parts(seq, outline_parts[k].clone())?;
                put(&mut r, 4, sg as i32);
            } else if i == ALL_TEXT {
                put(&mut r, 4, text_sg as i32);
                put(&mut r, 0x10, ALL_TEXT_TX);
                put(&mut r, 0x14, ALL_TEXT_TX);
            }
            out.push(r);
        }
        let mut bottle = seq.anims[src[BOTTLE]];
        put(&mut bottle, 0x10, ITEM_ICON_TX);
        put(&mut bottle, 0x14, ITEM_ICON_TX);
        out.push(bottle);
        push_group(seq, SOURCES[lit.unwrap_or(ALL)], out)?;
    }
    Ok(style)
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
    let _ = writeln!(s, "  \"strip\": {},", group::STRIP);
    let _ = writeln!(s, "  \"strip_all_finger\": {},", strip::ALL_FINGER);
    let _ = writeln!(s, "  \"strip_items_finger\": {},", strip::ITEMS_FINGER);
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
    fn the_strip_splits_alls_place() {
        use strip::*;
        // ALL's fill (PS2 and PC parts): the left corner moves 24 right, the right one stays.
        assert_eq!([-82, -69, -38, -37, -26, -24].map(all_fill_x), [-58, -45, -38, -37, -26, -24]);
        assert_eq!(all_fill_x(-53), -41);
        // The cells meet: Items −82..−58, ALL −58..−26, Re:CoM's tab 0 from −26.
        assert_eq!((ITEMS.1, NEW_ALL.0, NEW_ALL.1), (-58, -58, ALL_CELL.1));
        assert_eq!(ITEM_ICON_TX, (ITEMS.0 + ITEMS.1) / 2);
        assert_eq!(ALL_TEXT_TX, (NEW_ALL.0 + NEW_ALL.1) / 2);
        // The fingers: ALL keeps its place in its cell (8 left of the middle).
        assert_eq!(RECOM_ALL_FINGER - (ALL_CELL.0 + ALL_CELL.1) / 2, ALL_FINGER - ALL_TEXT_TX);
        // The text: the PS2 layout's 35 wide, the PC's 48.
        assert_eq!(all_text_scale(35), (0.8, 0.8));
        let (sx, sy) = all_text_scale(48);
        assert!((sx * 48.0 - 28.0).abs() < 1e-4 && sy == 0.8);
        assert_eq!(all_text_scale(20), (1.0, 1.0));
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
        // The strips: Re:CoM's 14 animations, the Items fill and the bottle.
        for g in group::STRIP..=group::STRIP_ITEMS {
            assert_eq!(anims_of(&s, BASE_GROUPS + g).unwrap().len(), strip::ANIMS + 2);
        }
        // Fed back: refused.
        assert!(patch_container(&out).is_err());
        // Re:CoM's own groups are unchanged.
        let before = Seqd::parse(&Layd::parse(&Container::parse(&spr).unwrap().find(LAYOUT).unwrap().data).unwrap().seqs[SEQ]).unwrap();
        assert_eq!(s.agroups[..BASE_GROUPS], before.agroups[..]);
        assert_eq!(s.anims[..before.anims.len()], before.anims[..]);
        assert_eq!(s.parts[..before.parts.len()], before.parts[..]);
        // The window at place 0 draws Re:CoM's tab-0 window (same parts, copied).
        let uk = dir.join("FORM_0109_FO0109.SPR.hd04UK_m_shop.lad");
        let uk_out = std::fs::read(uk).ok().map(|lad| {
            let (out, g) = patch_layout(&lad).unwrap();
            assert_eq!(g, BASE_GROUPS);
            let s = Seqd::parse(&Layd::parse(&out).unwrap().seqs[SEQ]).unwrap();
            assert_eq!(s.agroups.len(), BASE_GROUPS + group::COUNT);
            out
        });
        // The patched layouts, for a look (`KH1_FO0109_OUT` = a folder outside the repo).
        if let Ok(o) = std::env::var("KH1_FO0109_OUT") {
            let o = std::path::Path::new(&o);
            std::fs::write(o.join("m_shop.lad"), &c.find(LAYOUT).unwrap().data).unwrap();
            if let Some(u) = uk_out {
                std::fs::write(o.join("UK_m_shop.lad"), u).unwrap();
            }
        }
    }
}
