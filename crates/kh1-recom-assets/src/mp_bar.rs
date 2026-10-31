//! The MP bar's data (M7 stage 7.2, decision 34): KH1's MP gauge colours, read from the player's
//! KH1 `command2/uitex.bin`, as new sprites and animation groups in Re:CoM's own `Cockpit:00`
//! sequence set (`FORM/0002/FO0002.SPR` → `Cockpit.lad`, SEQD 0), which Re:CoM's 2D code loads,
//! animates and draws. The DLL (`crates/kh1-recom` `win/mp_bar.rs`) only adds the layers to a
//! `C2DSeqCtrl` of the player gauge and sets their frames. Notes: `docs/recom-m7-mp-bar.md`.
//!
//! Formats (Checked (files: the owner's FO0002.SPR / .RTM, OpenKH extract) + decomp):
//! - `.SPR` / `.RTM` container: `u32` header size, then 0x20-byte entries `{u32 offset,
//!   name[0x18], u32 size | 0x80000000}` and one all-zero entry; `.SPR` data 16-byte aligned
//!   (0xCD padding).
//! - `.lad` = KH2's LAYD: header `{magic, version, L1 count, L1 offset, L2 count, L2 offset, SEQD
//!   count, SEQD table offset}`; the SEQDs follow the table, back to back.
//! - SEQD (version 0x160): header 0x30 `{magic, version, sprites, parts, sprite groups, animations,
//!   animation groups}` as (count, offset) pairs, then those records back to back: sprite 0x2C
//!   `{u32, l, t, r, b, u/v scroll f32, 4 vertex colours ABGR}`, part 0x14 `{x0, y0, x1, y1,
//!   sprite}`, sprite group 4 `{i16 first part, i16 count}`, animation 0x90, animation group 0x24
//!   `{i16 first animation, i16 count, i16 play once, i16, i32 loop start, …}`. Re:CoM's loader
//!   (`0x140246DE0`) keeps the animation and group tables and their counts; an animation group
//!   index at or past the count adds nothing (`0x140246A40`). Checked (decomp).
//! - `.imz` = IMGZ of IMGD: 8-bit (PSMT8) PS2-swizzled pixels with a CSM1 256-colour CLUT.
//!
//! Colours: Re:CoM draws a sprite as texel × vertex colour / 128 (its 2D colour product is
//! `a * b >> 7`, `0x140250000`-area; Checked (decomp)). Our sprites use the 1-texel neutral patch
//! of the Cockpit atlas that Re:CoM's own tinted boxes use (`NEUTRAL_UV`, sprites 15–18 / 42–44),
//! read from the player's FO0002.RTM; the vertex colour is KH1's texel × 128 / that texel, so the
//! screen colour is KH1's texel. KH1 draws its MP gauge with vertex colour 0x80 (×1) over its own
//! texels (`0x14026B790` callers pass `0x80808080`; Checked (decomp)). That both PCs keep the
//! PS2 colour rule on screen: Assumption.

use crate::{bad, Result};

/// The Re:CoM file that holds `Cockpit.lad`.
pub const SPR_PATH: &str = "FORM/0002/FO0002.SPR";
/// The Re:CoM file that holds `Cockpit.imz` (the atlas `Cockpit:00` draws from).
pub const RTM_PATH: &str = "FORM/0002/FO0002.RTM";
/// KH1's HUD textures (`0x140266580` loads it; `0x14026DB00` draws the gauge from block 5).
pub const KH1_UITEX: &str = "command2/uitex.bin";
/// The manifest the DLL reads (next to `kh1-stats.json`).
pub const MANIFEST: &str = "kh1-mp-bar.json";
/// The layout and the sprite set (Re:CoM names it `Cockpit:00`: SEQD 0 of `Cockpit.lad`).
pub const LAYOUT: &str = "Cockpit.lad";
/// The atlas `Cockpit:00` draws from (in `FO0002.RTM`).
pub const ATLAS: &str = "Cockpit.imz";
/// The sprite set's name in Re:CoM's 2D registry.
pub const SET: &str = "Cockpit:00";
/// Re:CoM's own animation groups in `Cockpit:00` (Steam; Checked (files)). Ours follow.
pub const BASE_GROUPS: usize = 151;
/// The atlas patch Re:CoM's own tinted boxes sample (sprites 15–18 and 42–44 of `Cockpit:00`):
/// a block of texels (128, 128, 128, opaque). Checked (files).
pub const NEUTRAL_UV: [i32; 4] = [254, 47, 255, 48];

/// KH1 draws at most 20 MP (`0x14026C6E0` caps the straight segments at 0x14). Checked (decomp).
pub const SLOTS: i32 = 20;
/// Charge units per slot: point k needs 30·k (`0x1402A4CB0`), and the gauge draws charge c as c
/// units along the path where each MP slot is 30 (`0x14026A770` / `0x14026A970`). Checked (decomp).
pub const UNITS_PER_SLOT: i32 = 30;
/// KH1's straight segments are 26 units apart (`0x1A` in `0x14026C6E0` and `0x14026A970`).
/// Checked (decomp).
pub const PITCH: i32 = 26;
/// The scrub range: 20 slots × 30 units (KH1 also caps the charge at 600, `0x14026A970`).
pub const FRAME_MAX: i32 = SLOTS * UNITS_PER_SLOT;
/// Where the bar starts, relative to the HP gauge's origin (`gauge.lad` group 1 = (60, 60)): the
/// left and top edge of Riku's Dark gauge fill (`gauge:01` part `(1, -10, 42, 4)`). Checked
/// (files); how it looks on screen: Assumption until a screenshot.
pub const X0: i32 = 1;
/// The bar's top edge (see [`X0`]).
pub const Y0: i32 = -10;
/// A frame that is never reached (held pieces).
pub const LONG: i32 = 10_000_000;
/// KH1's change fade: 128 → 0 (or 0 → 128) by 12.8 per frame (`0x14027F760` / `0x14027F780` from
/// `0x14026C6E0` and `0x14026A770`), drawn after the first step: 115.2, 102.4 … 12.8, then done at
/// the 10th frame. Checked (decomp).
pub const FADE_FIRST: u8 = 115;
/// The fade's last drawn value (12.8).
pub const FADE_LAST: u8 = 13;
/// The frame the fade is over (10 frames = 500 Re:CoM units).
pub const FADE_FRAMES: i32 = 10;

/// Animation flags, each copied from a Re:CoM animation that does the same thing: a static piece
/// (`gauge:01` 0x77FF1, the HP bar's pieces), a piece whose X scale runs 0 → 1 (0x77DB1, the HP
/// bar's straight part), a colour fade (0x77B71, `Cockpit:00`'s tinted boxes). Checked (files).
const FLAGS_STATIC: u32 = 0x77FF1;
const FLAGS_GROW: u32 = 0x77DB1;
const FLAGS_COLOUR: u32 = 0x77B71;
/// Animation group word +6 (not read by Re:CoM's instance set-up `0x140248F40`; Checked
/// (decomp)), copied from Re:CoM groups of the same kind: `gauge:01` 0x12 (scrubbed) and
/// `Cockpit:00` 0x38 (a fade).
const GROUP_WORD_SCRUB: i16 = 1015;
const GROUP_WORD_FADE: i16 = 623;

/// Our animation groups, from [`BASE_GROUPS`] on.
pub mod group {
    /// The track: one slot per max MP (frame = 30 × max MP).
    pub const TRACK: usize = 0;
    /// The MP fill (frame = 30 × MP).
    pub const MP: usize = 1;
    /// The charge line (frame = charge).
    pub const CHARGE: usize = 2;
    /// A gained MP slot: black over it, fading out (KH1: the slot's colour rises from 0).
    pub const MP_IN: usize = 3;
    /// A lost MP slot: its colour falling to 0, then gone.
    pub const MP_OUT: usize = 4;
    /// Gained charge: its alpha rising, then held.
    pub const CHARGE_IN: usize = 5;
    /// Lost charge: its alpha falling, then gone.
    pub const CHARGE_OUT: usize = 6;
    /// Gained charge past KH1's ring part (no fade), held.
    pub const CHARGE_STATIC: usize = 7;
    /// How many we add.
    pub const COUNT: usize = 8;
}

// ---------------------------------------------------------------------------------------------
// Little-endian reading.

fn u16_at(d: &[u8], at: usize) -> Result<u16> {
    d.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| crate::Error::Format(format!("read past the end at {at:#x}")))
}

fn u32_at(d: &[u8], at: usize) -> Result<u32> {
    d.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| crate::Error::Format(format!("read past the end at {at:#x}")))
}

fn i32_at(d: &[u8], at: usize) -> Result<i32> {
    u32_at(d, at).map(|v| v as i32)
}

fn slice(d: &[u8], at: usize, n: usize) -> Result<&[u8]> {
    d.get(at..at + n).ok_or_else(|| crate::Error::Format(format!("{n} bytes at {at:#x}: past the end ({:#x})", d.len())))
}

fn align(n: usize, a: usize) -> usize {
    n.div_ceil(a) * a
}

// ---------------------------------------------------------------------------------------------
// The .SPR / .RTM container.

/// One file in a `.SPR` / `.RTM` container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The 0x18 name bytes as stored (`.RTM` keeps a type word in the last four).
    pub name_raw: [u8; 0x18],
    /// Bit 31 of the size word.
    pub flag: bool,
    /// The file.
    pub data: Vec<u8>,
}

impl Entry {
    /// The name up to its first NUL.
    pub fn name(&self) -> String {
        let n = self.name_raw.iter().position(|&b| b == 0).unwrap_or(self.name_raw.len());
        String::from_utf8_lossy(&self.name_raw[..n]).into_owned()
    }
}

/// A `.SPR` / `.RTM` container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Container {
    /// The files, in header order.
    pub entries: Vec<Entry>,
}

impl Container {
    /// Reads a container.
    pub fn parse(b: &[u8]) -> Result<Container> {
        let hs = u32_at(b, 0)? as usize;
        if hs < 0x20 || !hs.is_multiple_of(0x20) || hs > b.len() {
            return bad(format!("container: header size {hs:#x}"));
        }
        let mut entries = Vec::new();
        for at in (0..hs).step_by(0x20) {
            let off = u32_at(b, at)? as usize;
            let raw: [u8; 0x18] = slice(b, at + 4, 0x18)?.try_into().unwrap();
            let sw = u32_at(b, at + 0x1C)?;
            if raw[0] == 0 {
                break;
            }
            let size = (sw & 0x7FFF_FFFF) as usize;
            entries.push(Entry { name_raw: raw, flag: sw & 0x8000_0000 != 0, data: slice(b, off, size)?.to_vec() });
        }
        if entries.is_empty() {
            return bad("container: no entries");
        }
        Ok(Container { entries })
    }

    /// Writes it as Re:CoM's `.SPR`s are laid out: one slot per file and an all-zero one, data
    /// 16-byte aligned with 0xCD padding. Checked (files: FO0002.SPR writes back byte for byte).
    pub fn write(&self) -> Vec<u8> {
        let hs = (self.entries.len() + 1) * 0x20;
        let mut out = vec![0u8; hs];
        let mut pos = hs;
        for (i, e) in self.entries.iter().enumerate() {
            pos = align(pos, 16);
            let at = i * 0x20;
            out[at..at + 4].copy_from_slice(&(pos as u32).to_le_bytes());
            out[at + 4..at + 0x1C].copy_from_slice(&e.name_raw);
            let sw = e.data.len() as u32 | if e.flag { 0x8000_0000 } else { 0 };
            out[at + 0x1C..at + 0x20].copy_from_slice(&sw.to_le_bytes());
            out.resize(pos, 0xCD);
            out.extend_from_slice(&e.data);
            pos += e.data.len();
        }
        out.resize(align(out.len(), 16), 0xCD);
        out
    }

    /// The file named `name` (any letter case).
    pub fn find_mut(&mut self, name: &str) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.name().eq_ignore_ascii_case(name))
    }

    /// The file named `name` (any letter case).
    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name().eq_ignore_ascii_case(name))
    }
}

// ---------------------------------------------------------------------------------------------
// LAYD.

/// A `.lad`: everything before the first SEQD as stored, and the SEQDs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layd {
    head: Vec<u8>,
    table: usize,
    /// The sequence sets (`<name>:00`, `:01` … in this order).
    pub seqs: Vec<Vec<u8>>,
}

impl Layd {
    /// Reads a LAYD whose SEQDs follow their offset table back to back (all of Re:CoM's
    /// FO0002 layouts; Checked (files)).
    pub fn parse(d: &[u8]) -> Result<Layd> {
        if slice(d, 0, 4)? != b"LAYD" {
            return bad("lad: not LAYD");
        }
        let n = u32_at(d, 0x18)? as usize;
        let table = u32_at(d, 0x1C)? as usize;
        if n == 0 || n > 256 {
            return bad(format!("lad: {n} sequence sets"));
        }
        let offs: Vec<usize> = (0..n).map(|i| u32_at(d, table + 4 * i).map(|v| v as usize)).collect::<Result<_>>()?;
        if offs[0] < table + 4 * n || offs.windows(2).any(|w| w[1] <= w[0]) || *offs.last().unwrap() >= d.len() {
            return bad("lad: sequence sets out of order");
        }
        let mut seqs = Vec::with_capacity(n);
        for (i, &o) in offs.iter().enumerate() {
            let end = offs.get(i + 1).copied().unwrap_or(d.len());
            seqs.push(d[o..end].to_vec());
        }
        Ok(Layd { head: d[..offs[0]].to_vec(), table, seqs })
    }

    /// Writes it back with the SEQDs back to back (4-byte aligned) and the table updated.
    pub fn write(&self) -> Vec<u8> {
        let mut out = self.head.clone();
        let mut offs = Vec::with_capacity(self.seqs.len());
        for s in &self.seqs {
            out.resize(align(out.len(), 4), 0);
            offs.push(out.len() as u32);
            out.extend_from_slice(s);
        }
        for (i, o) in offs.iter().enumerate() {
            let at = self.table + 4 * i;
            out[at..at + 4].copy_from_slice(&o.to_le_bytes());
        }
        out
    }
}

// ---------------------------------------------------------------------------------------------
// SEQD.

const SPRITE: usize = 0x2C;
const PART: usize = 0x14;
const SGROUP: usize = 4;
const ANIM: usize = 0x90;
const AGROUP: usize = 0x24;

/// A SEQD as record tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seqd {
    /// The version word (0x160).
    pub version: u32,
    /// Sprites (0x2C bytes each).
    pub sprites: Vec<[u8; SPRITE]>,
    /// Parts (0x14).
    pub parts: Vec<[u8; PART]>,
    /// Sprite groups (4).
    pub sgroups: Vec<[u8; SGROUP]>,
    /// Animations (0x90).
    pub anims: Vec<[u8; ANIM]>,
    /// Animation groups (0x24).
    pub agroups: Vec<[u8; AGROUP]>,
}

fn records<const N: usize>(d: &[u8], count: usize, off: usize) -> Result<Vec<[u8; N]>> {
    (0..count).map(|i| slice(d, off + i * N, N).map(|s| s.try_into().unwrap())).collect()
}

impl Seqd {
    /// Reads a SEQD laid out as Re:CoM's are (header, then the five tables back to back).
    pub fn parse(d: &[u8]) -> Result<Seqd> {
        if slice(d, 0, 4)? != b"SEQD" {
            return bad("seqd: not SEQD");
        }
        let f = |i: usize| u32_at(d, 8 + 4 * i).map(|v| v as usize);
        let (sc, so, pc, po, gc, go, ac, ao, agc, ago) = (f(0)?, f(1)?, f(2)?, f(3)?, f(4)?, f(5)?, f(6)?, f(7)?, f(8)?, f(9)?);
        let expect = [(so, 0x30), (po, so + sc * SPRITE), (go, po + pc * PART), (ao, go + gc * SGROUP), (ago, ao + ac * ANIM)];
        if expect.iter().any(|(a, b)| a != b) || ago + agc * AGROUP > d.len() {
            return bad("seqd: tables not back to back (unknown layout)");
        }
        Ok(Seqd {
            version: u32_at(d, 4)?,
            sprites: records(d, sc, so)?,
            parts: records(d, pc, po)?,
            sgroups: records(d, gc, go)?,
            anims: records(d, ac, ao)?,
            agroups: records(d, agc, ago)?,
        })
    }

    /// Writes it in the same layout.
    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"SEQD");
        out.extend_from_slice(&self.version.to_le_bytes());
        let mut off = 0x30usize;
        for (n, size) in [(self.sprites.len(), SPRITE), (self.parts.len(), PART), (self.sgroups.len(), SGROUP), (self.anims.len(), ANIM), (self.agroups.len(), AGROUP)] {
            out.extend_from_slice(&(n as u32).to_le_bytes());
            out.extend_from_slice(&(off as u32).to_le_bytes());
            off += n * size;
        }
        debug_assert_eq!(out.len(), 0x30);
        self.sprites.iter().for_each(|r| out.extend_from_slice(r));
        self.parts.iter().for_each(|r| out.extend_from_slice(r));
        self.sgroups.iter().for_each(|r| out.extend_from_slice(r));
        self.anims.iter().for_each(|r| out.extend_from_slice(r));
        self.agroups.iter().for_each(|r| out.extend_from_slice(r));
        out
    }
}

fn sprite(uv: [i32; 4], colours: [u32; 4]) -> [u8; SPRITE] {
    let mut r = [0u8; SPRITE];
    for (i, v) in uv.iter().enumerate() {
        r[4 + 4 * i..8 + 4 * i].copy_from_slice(&v.to_le_bytes());
    }
    for (i, c) in colours.iter().enumerate() {
        r[0x1C + 4 * i..0x20 + 4 * i].copy_from_slice(&c.to_le_bytes());
    }
    r
}

fn part(rect: [i32; 4], sprite: usize) -> [u8; PART] {
    let mut r = [0u8; PART];
    for (i, v) in rect.iter().chain(std::iter::once(&(sprite as i32))).enumerate() {
        r[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
    }
    r
}

fn sgroup(first: usize, count: usize) -> [u8; SGROUP] {
    let mut r = [0u8; SGROUP];
    r[..2].copy_from_slice(&(first as i16).to_le_bytes());
    r[2..].copy_from_slice(&(count as i16).to_le_bytes());
    r
}

/// One animation: `frames` inclusive, X translate `tx`, X scale `sx` start → end, colour start →
/// end (ABGR, 0x80 = ×1). Everything else as Re:CoM's static pieces have it.
fn anim(flags: u32, sg: usize, frames: (i32, i32), tx: i32, sx: (f32, f32), colour: (u32, u32)) -> [u8; ANIM] {
    let mut r = [0u8; ANIM];
    let mut put = |at: usize, b: [u8; 4]| r[at..at + 4].copy_from_slice(&b);
    put(0x00, flags.to_le_bytes());
    put(0x04, (sg as i32).to_le_bytes());
    put(0x08, frames.0.to_le_bytes());
    put(0x0C, frames.1.to_le_bytes());
    put(0x10, tx.to_le_bytes());
    put(0x14, tx.to_le_bytes());
    // 0x18..0x30: Y translate and pivot 0; 0x30..0x48: rotations 0.
    for at in [0x48, 0x4C, 0x58, 0x5C, 0x64, 0x68, 0x6C] {
        put(at, 1.0f32.to_le_bytes()); // scale, scale Y, curve (0, 1, 1, 1) as Re:CoM's
    }
    put(0x50, sx.0.to_le_bytes());
    put(0x54, sx.1.to_le_bytes());
    // 0x70..0x88: bounce, bounce counts, colour blend 0.
    put(0x88, colour.0.to_le_bytes());
    put(0x8C, colour.1.to_le_bytes());
    r
}

fn agroup(first: usize, count: usize, once: bool, word: i16) -> [u8; AGROUP] {
    let mut r = [0u8; AGROUP];
    r[0..2].copy_from_slice(&(first as i16).to_le_bytes());
    r[2..4].copy_from_slice(&(count as i16).to_le_bytes());
    r[4..6].copy_from_slice(&(once as i16).to_le_bytes());
    r[6..8].copy_from_slice(&word.to_le_bytes());
    r
}

/// ABGR with alpha `a` (0x80 = opaque, ×1).
pub fn abgr(rgb: [u8; 3], a: u8) -> u32 {
    rgb[0] as u32 | (rgb[1] as u32) << 8 | (rgb[2] as u32) << 16 | (a as u32) << 24
}

/// The vertex colour that shows KH1's texel `kh1` on Re:CoM's neutral texel `neutral`:
/// `round(kh1 × 128 / neutral)` per channel, at most 255; alpha 0x80 (opaque).
pub fn vertex_colour(kh1: [u8; 4], neutral: [u8; 4]) -> u32 {
    let ch = |c: u8, t: u8| -> u8 { ((c as u32 * 128 + t as u32 / 2) / (t.max(1) as u32)).min(255) as u8 };
    abgr([ch(kh1[0], neutral[0]), ch(kh1[1], neutral[1]), ch(kh1[2], neutral[2])], 0x80)
}

/// KH1's MP gauge look, read from `uitex.bin`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kh1Look {
    /// The straight MP segment: top and bottom row colours (RGBA, PS2 alpha) and its size.
    pub mp_top: [u8; 4],
    /// The MP segment's bottom row.
    pub mp_bottom: [u8; 4],
    /// The charge line (drawn over the top of the MP segment): its top row.
    pub charge_top: [u8; 4],
    /// The charge line's bottom row.
    pub charge_bottom: [u8; 4],
    /// The gauge background piece's fill.
    pub back: [u8; 4],
    /// The background piece's border (top and bottom rows).
    pub border: [u8; 4],
    /// The MP segment's width (24).
    pub slot_w: i32,
    /// The MP segment's height (12).
    pub mp_h: i32,
    /// The charge line's height (4).
    pub charge_h: i32,
    /// The background border's height (2).
    pub border_h: i32,
}

/// The sprites, parts and groups added to `Cockpit:00`; returns the first new group's index.
pub fn add_bar(seq: &mut Seqd, look: &Kh1Look, neutral: [u8; 4]) -> Result<usize> {
    // Language copies have their own group counts (`FR_Cockpit.lad`: 157); ours go after them.
    if seq.agroups.is_empty() || seq.agroups.len() > BASE_GROUPS + 64 {
        return bad(format!("{SET} has {} animation groups (Re:CoM Steam's base file: {BASE_GROUPS})", seq.agroups.len()));
    }
    // A set that already ends with our sprites (this step's output fed back) is refused.
    let mut probe = seq.clone();
    let p0 = probe.sprites.len();
    add_bar_unchecked(&mut probe, look, neutral)?;
    let ours = &probe.sprites[p0..];
    if !ours.is_empty() && seq.sprites.len() >= ours.len() && seq.sprites[seq.sprites.len() - ours.len()..] == *ours {
        return bad(format!("{SET} already ends with the MP bar's sprites (already patched)"));
    }
    add_bar_unchecked(seq, look, neutral)
}

fn add_bar_unchecked(seq: &mut Seqd, look: &Kh1Look, neutral: [u8; 4]) -> Result<usize> {
    let s0 = seq.sprites.len();
    let v = |c: [u8; 4]| vertex_colour(c, neutral);
    let flat = |c: u32| [c; 4];
    // Vertex order TL, TR, BL, BR: Re:CoM's digit sprites shade top → bottom as (c, c, d, d).
    // Checked (files: gauge:01 sprites 23–32); the corner order itself: Assumption.
    let vertical = |top: u32, bottom: u32| [top, top, bottom, bottom];
    seq.sprites.push(sprite(NEUTRAL_UV, flat(v(look.border)))); // +0 border
    seq.sprites.push(sprite(NEUTRAL_UV, flat(v(look.back)))); // +1 track fill
    seq.sprites.push(sprite(NEUTRAL_UV, vertical(v(look.mp_top), v(look.mp_bottom)))); // +2 MP
    seq.sprites.push(sprite(NEUTRAL_UV, vertical(v(look.charge_top), v(look.charge_bottom)))); // +3 charge
    let (w, h, hc, bh) = (look.slot_w, look.mp_h, look.charge_h, look.border_h);
    let p0 = seq.parts.len();
    seq.parts.push(part([-1, Y0 - bh, w + 1, Y0 + h + bh], s0)); // +0 border
    seq.parts.push(part([0, Y0, w, Y0 + h], s0 + 1)); // +1 fill
    seq.parts.push(part([0, Y0, w, Y0 + h], s0 + 2)); // +2 MP
    seq.parts.push(part([0, Y0, w, Y0 + hc], s0 + 3)); // +3 charge
    seq.parts.push(part([0, Y0, w, Y0 + h], s0)); // +4 black over an MP slot
    let g0 = seq.sgroups.len();
    seq.sgroups.push(sgroup(p0, 2)); // track
    seq.sgroups.push(sgroup(p0 + 2, 1)); // MP
    seq.sgroups.push(sgroup(p0 + 3, 1)); // charge
    seq.sgroups.push(sgroup(p0 + 4, 1)); // black
    let (sg_track, sg_mp, sg_chg, sg_black) = (g0, g0 + 1, g0 + 2, g0 + 3);
    let one = 0x8080_8080u32;
    let alpha = |a: u8| (a as u32) << 24 | 0x80_8080;
    let grey = |c: u8| 0x8000_0000 | (c as u32) * 0x01_0101;
    let mut groups: Vec<(Vec<[u8; ANIM]>, bool, i16)> = Vec::new();
    let slot_x = |i: i32| X0 + PITCH * i;
    // Scrubbed: slot i shows from frame 30·(i+1) (a whole point) on.
    let full_from = |sg: usize| (0..SLOTS).map(|i| anim(FLAGS_STATIC, sg, (UNITS_PER_SLOT * (i + 1), FRAME_MAX), slot_x(i), (1.0, 1.0), (one, one))).collect::<Vec<_>>();
    groups.push((full_from(sg_track), true, GROUP_WORD_SCRUB));
    groups.push((full_from(sg_mp), true, GROUP_WORD_SCRUB));
    // The charge: slot i grows over frames 30i..30i+30 (KH1's partial segment), then stays.
    let mut charge = Vec::new();
    for i in 0..SLOTS {
        let a = UNITS_PER_SLOT * i;
        charge.push(anim(FLAGS_GROW, sg_chg, (a, a + UNITS_PER_SLOT), slot_x(i), (0.0, 1.0), (one, one)));
        if a + UNITS_PER_SLOT < FRAME_MAX {
            charge.push(anim(FLAGS_STATIC, sg_chg, (a + UNITS_PER_SLOT + 1, FRAME_MAX), slot_x(i), (1.0, 1.0), (one, one)));
        }
    }
    groups.push((charge, true, GROUP_WORD_SCRUB));
    // Fades (time-played, one slot-wide piece; the DLL places and X-scales each piece's control).
    let last = FADE_FRAMES - 2; // 8: the 9 drawn steps 115.2 … 12.8
    let fade = |sg: usize, from: u32, to: u32, then: u32, held: bool| {
        vec![anim(FLAGS_COLOUR, sg, (0, last), 0, (1.0, 1.0), (from, to)), anim(FLAGS_COLOUR, sg, (last + 1, if held { LONG } else { FADE_FRAMES }), 0, (1.0, 1.0), (then, then))]
    };
    groups.push((fade(sg_black, alpha(FADE_FIRST), alpha(FADE_LAST), 0x0080_8080, false), true, GROUP_WORD_FADE)); // MP in
    groups.push((fade(sg_mp, grey(FADE_FIRST), grey(FADE_LAST), 0, false), true, GROUP_WORD_FADE)); // MP out
    groups.push((fade(sg_chg, alpha(FADE_LAST), alpha(FADE_FIRST), one, true), true, GROUP_WORD_FADE)); // charge in
    groups.push((fade(sg_chg, alpha(FADE_FIRST), alpha(FADE_LAST), 0x0080_8080, false), true, GROUP_WORD_FADE)); // charge out
    groups.push((vec![anim(FLAGS_COLOUR, sg_chg, (0, LONG), 0, (1.0, 1.0), (one, one))], true, GROUP_WORD_FADE)); // charge static
    debug_assert_eq!(groups.len(), group::COUNT);
    let first = seq.agroups.len();
    for (anims, once, word) in groups {
        let a0 = seq.anims.len();
        let n = anims.len();
        seq.anims.extend(anims);
        seq.agroups.push(agroup(a0, n, once, word));
    }
    if seq.anims.len() > i16::MAX as usize || seq.parts.len() > i16::MAX as usize {
        return bad("seqd: too many records for 16-bit indices");
    }
    Ok(first)
}

/// Adds the bar to a `Cockpit.lad` (its SEQD 0, `Cockpit:00`).
pub fn patch_layout(lad: &[u8], look: &Kh1Look, neutral: [u8; 4]) -> Result<(Vec<u8>, usize)> {
    let mut l = Layd::parse(lad)?;
    let mut s = Seqd::parse(&l.seqs[0])?;
    let first = add_bar(&mut s, look, neutral)?;
    l.seqs[0] = s.write();
    Ok((l.write(), first))
}

/// Adds the bar to the `Cockpit.lad` inside an `FO0002.SPR` container.
pub fn patch_spr(spr: &[u8], look: &Kh1Look, neutral: [u8; 4]) -> Result<(Vec<u8>, usize)> {
    let mut c = Container::parse(spr)?;
    let e = c.find_mut(LAYOUT).ok_or_else(|| crate::Error::Format(format!("no {LAYOUT}")))?;
    let (lad, first) = patch_layout(&e.data, look, neutral)?;
    e.data = lad;
    Ok((c.write(), first))
}

/// How one file of the `FO0002.SPR` entry (the file itself or one of its PC assets, e.g. a
/// language's `Cockpit.lad`) was handled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Patched {
    /// The bar was added; its first group.
    Bar(usize),
    /// Not a layout this step patches: kept as it is.
    Kept,
}

/// Patches an asset of the `FO0002.SPR` entry when it is a whole container with `Cockpit.lad`, or
/// a `Cockpit.lad` itself (a name ending in it, e.g. a language's `UK_Cockpit.lad`).
pub fn patch_asset(name: &str, data: &[u8], look: &Kh1Look, neutral: [u8; 4]) -> Result<(Vec<u8>, Patched)> {
    if data.starts_with(b"LAYD") {
        if name.to_ascii_lowercase().ends_with(&LAYOUT.to_ascii_lowercase()) {
            let (d, first) = patch_layout(data, look, neutral)?;
            return Ok((d, Patched::Bar(first)));
        }
        return Ok((data.to_vec(), Patched::Kept));
    }
    match Container::parse(data) {
        Ok(c) if c.find(LAYOUT).is_some() => {
            let (d, first) = patch_spr(data, look, neutral)?;
            Ok((d, Patched::Bar(first)))
        }
        _ => Ok((data.to_vec(), Patched::Kept)),
    }
}

// ---------------------------------------------------------------------------------------------
// KH1's TIM2 pictures.

/// One TIM2 picture of a file that is several TIM2 files back to back (KH1's `uitex.bin`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    /// Where the TIM2 starts in the file, and its size.
    pub offset: usize,
    /// Its size in bytes (header included).
    pub size: usize,
    /// Width in texels.
    pub width: usize,
    /// Height in texels.
    pub height: usize,
    /// RGBA, PS2 alpha (0x80 = opaque), row by row.
    pub pixels: Vec<[u8; 4]>,
}

impl Picture {
    /// The pixel at (x, y).
    pub fn at(&self, x: usize, y: usize) -> [u8; 4] {
        self.pixels[y * self.width + x]
    }
}

/// Splits a file of back-to-back one-picture TIM2s (4- or 8-bit, 32-bit CLUT) and decodes each.
pub fn tim2_pictures(d: &[u8]) -> Result<Vec<Picture>> {
    let mut out = Vec::new();
    let mut off = 0;
    while off + 16 <= d.len() && &d[off..off + 4] == b"TIM2" {
        if u16_at(d, off + 6)? != 1 {
            return bad(format!("tim2 at {off:#x}: not one picture"));
        }
        let p = off + 16;
        let total = u32_at(d, p)? as usize;
        let clut = u32_at(d, p + 4)? as usize;
        let img = u32_at(d, p + 8)? as usize;
        let hdr = u16_at(d, p + 0xC)? as usize;
        let colours = u16_at(d, p + 0xE)? as usize;
        let (ctype, itype) = (d[p + 0x12], d[p + 0x13]);
        let (w, h) = (u16_at(d, p + 0x14)? as usize, u16_at(d, p + 0x16)? as usize);
        let px = slice(d, p + hdr, img)?;
        let cl = slice(d, p + hdr + img, clut)?;
        if ctype & 3 != 3 || clut < colours * 4 {
            return bad(format!("tim2 at {off:#x}: not a 32-bit CLUT"));
        }
        let entry = |i: usize| -> [u8; 4] {
            // 256-colour CLUTs are stored in CSM1 order (`rtm::csm1`) unless type bit 7 says linear.
            let j = if colours == 256 && ctype & 0x80 == 0 { crate::rtm::csm1(i) } else { i };
            cl[4 * j..4 * j + 4].try_into().unwrap()
        };
        let pixels: Vec<[u8; 4]> = match itype {
            5 if img == w * h && colours == 256 => px.iter().map(|&i| entry(i as usize)).collect(),
            4 if img * 2 == w * h && colours >= 16 => px.iter().flat_map(|&b| [entry((b & 15) as usize), entry((b >> 4) as usize)]).collect(),
            _ => return bad(format!("tim2 at {off:#x}: image type {itype} with {colours} colours not read")),
        };
        out.push(Picture { offset: off, size: 16 + total, width: w, height: h, pixels });
        off += 16 + total;
    }
    if out.is_empty() {
        return bad("no TIM2 pictures");
    }
    Ok(out)
}

/// KH1's gauge texture: `uitex.bin` picture 5 (8-bit, 128 × 128). `0x140266580`-area loads it from
/// offset 5 × 0x2080 = 0xA280 (blocks 0–4 are 0x2080 bytes each, block 5 is 0x4440). Checked
/// (decomp + files).
pub const GAUGE_PICTURE: (usize, usize) = (5, 0xA280);
/// KH1's gauge background texture: picture 7 at 0x10740 (`_DAT_1427F14C0`). Checked (decomp + files).
pub const BACK_PICTURE: (usize, usize) = (7, 0x10740);
/// Where the straight MP segment, the charge line and the background piece sit in those
/// pictures (search windows, x0..x1 × y0..y1, end exclusive). Found by reading the owner's
/// `uitex.bin`: the blue 24 × 12 strip at (30, 102), the orange 24 × 4 at (30, 116), the
/// background piece at (8, 85). The records that name them (`exchange/<lang>_uibin_g_s.bin`) are
/// not in this session: Assumption that these strips are the ones `0x14026C6E0` (gauge `+0x98`)
/// and `0x14026A970` (`+0xA0`) draw (they are the only straight blue / orange strips, and their
/// size matches the 26-unit pitch).
pub const MP_WINDOW: [usize; 4] = [28, 100, 56, 115];
/// The charge line's search window (see [`MP_WINDOW`]).
pub const CHARGE_WINDOW: [usize; 4] = [28, 115, 56, 122];
/// The background piece's search window, in picture 7 (see [`MP_WINDOW`]).
pub const BACK_WINDOW: [usize; 4] = [6, 84, 36, 101];

/// The opaque (alpha ≥ 0x40) box inside a window: (x, y, w, h).
fn opaque_box(p: &Picture, win: [usize; 4]) -> Result<(usize, usize, usize, usize)> {
    let [x0, y0, x1, y1] = win;
    if x1 > p.width || y1 > p.height {
        return bad("window outside the picture");
    }
    let mut b: Option<(usize, usize, usize, usize)> = None;
    for y in y0..y1 {
        for x in x0..x1 {
            if p.at(x, y)[3] >= 0x40 {
                b = Some(match b {
                    None => (x, y, x, y),
                    Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x), e.max(y)),
                });
            }
        }
    }
    let (a, c, d, e) = b.ok_or_else(|| crate::Error::Format(format!("nothing opaque in window {win:?}")))?;
    Ok((a, c, d - a + 1, e - c + 1))
}

/// A strip whose rows are each one colour: (top colour, bottom colour, width, height).
fn strip(p: &Picture, win: [usize; 4]) -> Result<([u8; 4], [u8; 4], usize, usize)> {
    let (x, y, w, h) = opaque_box(p, win)?;
    for yy in y..y + h {
        if (x..x + w).any(|xx| p.at(xx, yy) != p.at(x, yy)) {
            return bad(format!("strip in {win:?}: row {yy} is not one colour"));
        }
    }
    Ok((p.at(x, y), p.at(x, y + h - 1), w, h))
}

/// Reads KH1's MP gauge look from `uitex.bin`.
pub fn kh1_look(uitex: &[u8]) -> Result<Kh1Look> {
    let pics = tim2_pictures(uitex)?;
    let pick = |(i, off): (usize, usize), what: &str| -> Result<&Picture> {
        let p = pics.get(i).ok_or_else(|| crate::Error::Format(format!("uitex.bin: no picture {i} ({what})")))?;
        if p.offset != off || p.width != 128 || p.height != 128 {
            return bad(format!("uitex.bin: picture {i} ({what}) at {:#x}, {}×{}; KH1 draws it from {off:#x}, 128×128", p.offset, p.width, p.height));
        }
        Ok(p)
    };
    let g = pick(GAUGE_PICTURE, "gauge")?;
    let b = pick(BACK_PICTURE, "gauge background")?;
    let (mp_top, mp_bottom, w, h) = strip(g, MP_WINDOW)?;
    let (charge_top, charge_bottom, wc, hc) = strip(g, CHARGE_WINDOW)?;
    if wc != w {
        return bad(format!("uitex.bin: charge line {wc} wide, MP segment {w}"));
    }
    let blue = |c: [u8; 4]| c[2] > c[0] && c[2] > c[1];
    let orange = |c: [u8; 4]| c[0] > c[1] && c[1] > c[2];
    if !blue(mp_top) || !blue(mp_bottom) || !orange(charge_top) || !orange(charge_bottom) {
        return bad(format!("uitex.bin: MP {mp_top:?}/{mp_bottom:?} and charge {charge_top:?}/{charge_bottom:?} are not KH1's blue and orange"));
    }
    if w as i32 + 2 != PITCH {
        return bad(format!("uitex.bin: MP segment {w} wide; KH1 places them {PITCH} apart"));
    }
    let (bx, by, bw, bhh) = opaque_box(b, BACK_WINDOW)?;
    let mid = bx + bw / 2;
    let border = b.at(mid, by);
    let back = b.at(mid, by + bhh / 2);
    let border_h = (by..by + bhh).take_while(|&y| b.at(mid, y) == border).count();
    if border == back || border_h == 0 || border_h * 2 >= bhh {
        return bad("uitex.bin: background piece has no border");
    }
    Ok(Kh1Look { mp_top, mp_bottom, charge_top, charge_bottom, back, border, slot_w: w as i32, mp_h: h as i32, charge_h: hc as i32, border_h: border_h as i32 })
}

// ---------------------------------------------------------------------------------------------
// Re:CoM's atlas (IMGZ / IMGD, PS2-swizzled PSMT8).

const BC: [usize; 32] = [0, 1, 4, 5, 16, 17, 20, 21, 2, 3, 6, 7, 18, 19, 22, 23, 8, 9, 12, 13, 24, 25, 28, 29, 10, 11, 14, 15, 26, 27, 30, 31];
const C0: [usize; 64] = [
    0x00, 0x04, 0x10, 0x14, 0x20, 0x24, 0x30, 0x34, 0x02, 0x06, 0x12, 0x16, 0x22, 0x26, 0x32, 0x36, 0x08, 0x0c, 0x18, 0x1c, 0x28, 0x2c, 0x38, 0x3c, 0x0a, 0x0e, 0x1a, 0x1e, 0x2a, 0x2e, 0x3a, 0x3e, 0x21, 0x25, 0x31, 0x35, 0x01, 0x05, 0x11, 0x15, 0x23, 0x27, 0x33, 0x37, 0x03,
    0x07, 0x13, 0x17, 0x29, 0x2d, 0x39, 0x3d, 0x09, 0x0d, 0x19, 0x1d, 0x2b, 0x2f, 0x3b, 0x3f, 0x0b, 0x0f, 0x1b, 0x1f,
];
const C1: [usize; 64] = [
    0x20, 0x24, 0x30, 0x34, 0x00, 0x04, 0x10, 0x14, 0x22, 0x26, 0x32, 0x36, 0x02, 0x06, 0x12, 0x16, 0x28, 0x2c, 0x38, 0x3c, 0x08, 0x0c, 0x18, 0x1c, 0x2a, 0x2e, 0x3a, 0x3e, 0x0a, 0x0e, 0x1a, 0x1e, 0x01, 0x05, 0x11, 0x15, 0x21, 0x25, 0x31, 0x35, 0x03, 0x07, 0x13, 0x17, 0x23,
    0x27, 0x33, 0x37, 0x09, 0x0d, 0x19, 0x1d, 0x29, 0x2d, 0x39, 0x3d, 0x0b, 0x0f, 0x1b, 0x1f, 0x2b, 0x2f, 0x3b, 0x3f,
];
const PAO: [usize; 16] = [0, 1, 4, 5, 8, 9, 12, 13, 2, 3, 6, 7, 10, 11, 14, 15];

/// The PS2 PSMCT32 write of linear 32-bit data, `bw` × `bh` pages of 64 × 32 (OpenKH's
/// `Ps2.Encode32`).
fn encode32(b: &[u8], bw: usize, bh: usize) -> Vec<u8> {
    let mut out = vec![0u8; b.len()];
    for i in (0..0x20 * bh).step_by(0x20) {
        for j in (0..0x40 * bw).step_by(0x40) {
            let n3 = 0x2000 * (j / 0x40 + bw * (i / 0x20));
            for k in (0..0x20).step_by(8) {
                for m in (0..0x40).step_by(8) {
                    let n6 = 0x100 * BC[m / 8 + (k / 8) * 8];
                    for n in 0..4 {
                        for (q, &pao) in PAO.iter().enumerate() {
                            let (x, y) = (j + m + q % 8, i + k + 2 * n + q / 8);
                            let idx = 4 * (x + 0x40 * bw * y);
                            let to = 4 * pao + 0x40 * n + n6 + n3;
                            out[to..to + 4].copy_from_slice(&b[idx..idx + 4]);
                        }
                    }
                }
            }
        }
    }
    out
}

/// The PS2 PSMT8 read (OpenKH's `Ps2.Decode8`), `bw` × `bh` pages of 128 × 64.
fn decode8(b: &[u8], bw: usize, bh: usize) -> Vec<u8> {
    let mut out = vec![0u8; b.len()];
    for i in (0..0x40 * bh).step_by(0x40) {
        for j in (0..0x80 * bw).step_by(0x80) {
            let n3 = 0x2000 * (j / 0x80 + bw * (i / 0x40));
            for k in (0..0x40).step_by(0x10) {
                for m in (0..0x80).step_by(0x10) {
                    let n6 = 0x100 * BC[m / 0x10 + 8 * (k / 0x10)];
                    for n in 0..4 {
                        let t = if n % 2 == 0 { &C0 } else { &C1 };
                        for (q, &tq) in t.iter().enumerate() {
                            let (x, y) = (j + m + q % 16, i + k + 4 * n + q / 16);
                            out[x + 0x80 * bw * y] = b[n3 + n6 + 0x40 * n + tq];
                        }
                    }
                }
            }
        }
    }
    out
}

/// The texel at `(x, y)` of the first picture of an IMGZ (RGBA, PS2 alpha).
pub fn imgz_texel(imz: &[u8], x: usize, y: usize) -> Result<[u8; 4]> {
    if slice(imz, 0, 4)? != b"IMGZ" || u32_at(imz, 0xC)? == 0 {
        return bad("imz: not IMGZ");
    }
    let eo = u32_at(imz, 8)? as usize;
    let (o, l) = (u32_at(imz, eo)? as usize, u32_at(imz, eo + 4)? as usize);
    let g = slice(imz, o, l)?;
    if slice(g, 0, 4)? != b"IMGD" {
        return bad("imz: not IMGD");
    }
    let (po, pl, co, cl) = (u32_at(g, 8)? as usize, u32_at(g, 0xC)? as usize, u32_at(g, 0x10)? as usize, u32_at(g, 0x14)? as usize);
    let (w, h) = (u16_at(g, 0x1C)? as usize, u16_at(g, 0x1E)? as usize);
    let psm = u16_at(g, 0x26)?;
    let flags = i32_at(g, 0x3C)?;
    if psm != 0x13 || pl != w * h || cl != 1024 || x >= w || y >= h {
        return bad(format!("imz: psm {psm:#x} {w}×{h}: not a 256-colour 8-bit picture"));
    }
    let mut px = slice(g, po, pl)?.to_vec();
    if flags & 4 != 0 {
        if !w.is_multiple_of(128) || !h.is_multiple_of(64) {
            return bad("imz: swizzled picture size");
        }
        px = decode8(&encode32(&px, w / 128, h / 64), w / 128, h / 64);
    }
    let i = crate::rtm::csm1(px[y * w + x] as usize);
    Ok(slice(g, co + 4 * i, 4)?.try_into().unwrap())
}

/// Re:CoM's neutral texel (at [`NEUTRAL_UV`]) from `FO0002.RTM`; it must be grey and opaque.
pub fn neutral_texel(rtm: &[u8]) -> Result<[u8; 4]> {
    let c = Container::parse(rtm)?;
    let e = c.find(ATLAS).ok_or_else(|| crate::Error::Format(format!("FO0002.RTM: no {ATLAS}")))?;
    let t = imgz_texel(&e.data, NEUTRAL_UV[0] as usize, NEUTRAL_UV[1] as usize)?;
    if t[0] == 0 || t[0] != t[1] || t[1] != t[2] || t[3] < 0x7F {
        return bad(format!("{ATLAS} texel at ({}, {}) is {t:?}, not an opaque grey", NEUTRAL_UV[0], NEUTRAL_UV[1]));
    }
    Ok(t)
}

// ---------------------------------------------------------------------------------------------
// The manifest.

/// The `kh1-mp-bar.json` the DLL reads.
pub fn manifest_json(first: usize, firsts: &[usize], look: &Kh1Look, neutral: [u8; 4], files: &[(String, Patched)], sources: &[(&str, String)]) -> String {
    use std::fmt::Write;
    let hex = |c: u32| format!("\"{c:#010x}\"");
    let v = |c: [u8; 4]| hex(vertex_colour(c, neutral));
    let mut s = String::new();
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"format\": \"kh1-recom-assets mp-bar 1\",");
    let _ = writeln!(s, "  \"set\": \"{SET}\",");
    let _ = writeln!(s, "  \"base_groups\": {BASE_GROUPS},");
    let _ = writeln!(s, "  \"first_group\": {first},");
    let _ = writeln!(s, "  \"first_groups\": [{}],", firsts.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s, "  \"group_count\": {},", group::COUNT);
    let _ = writeln!(s, "  \"slots\": {SLOTS},");
    let _ = writeln!(s, "  \"units_per_slot\": {UNITS_PER_SLOT},");
    let _ = writeln!(s, "  \"pitch\": {PITCH},");
    let _ = writeln!(s, "  \"slot_width\": {},", look.slot_w);
    let _ = writeln!(s, "  \"mp_height\": {},", look.mp_h);
    let _ = writeln!(s, "  \"charge_height\": {},", look.charge_h);
    let _ = writeln!(s, "  \"x0\": {X0},");
    let _ = writeln!(s, "  \"y0\": {Y0},");
    let _ = writeln!(s, "  \"frame_max\": {FRAME_MAX},");
    let _ = writeln!(s, "  \"fade_frames\": {FADE_FRAMES},");
    let _ = writeln!(s, "  \"neutral_texel\": [{}, {}, {}, {}],", neutral[0], neutral[1], neutral[2], neutral[3]);
    let _ = writeln!(
        s,
        "  \"kh1_colours\": {{\"mp_top\": {:?}, \"mp_bottom\": {:?}, \"charge_top\": {:?}, \"charge_bottom\": {:?}, \"back\": {:?}, \"border\": {:?}}},",
        look.mp_top, look.mp_bottom, look.charge_top, look.charge_bottom, look.back, look.border
    );
    let _ = writeln!(
        s,
        "  \"vertex_colours\": {{\"mp_top\": {}, \"mp_bottom\": {}, \"charge_top\": {}, \"charge_bottom\": {}, \"back\": {}, \"border\": {}}},",
        v(look.mp_top),
        v(look.mp_bottom),
        v(look.charge_top),
        v(look.charge_bottom),
        v(look.back),
        v(look.border)
    );
    let f: Vec<String> = files.iter().map(|(n, p)| format!("{{\"name\": {n:?}, \"patched\": {}}}", matches!(p, Patched::Bar(_)))).collect();
    let _ = writeln!(s, "  \"files\": [{}],", f.join(", "));
    let src: Vec<String> = sources.iter().map(|(k, v)| format!("{k:?}: {v:?}")).collect();
    let _ = writeln!(s, "  \"sources\": {{{}}}", src.join(", "));
    let _ = writeln!(s, "}}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> Kh1Look {
        Kh1Look {
            mp_top: [45, 137, 224, 127],
            mp_bottom: [71, 65, 187, 128],
            charge_top: [254, 136, 1, 128],
            charge_bottom: [255, 173, 2, 126],
            back: [24, 28, 42, 128],
            border: [0, 0, 0, 128],
            slot_w: 24,
            mp_h: 12,
            charge_h: 4,
            border_h: 2,
        }
    }

    /// A SEQD with `groups` empty animation groups and one sprite / part / group / animation.
    fn seqd(groups: usize) -> Seqd {
        Seqd {
            version: 0x160,
            sprites: vec![sprite([1, 2, 3, 4], [0x8080_8080; 4])],
            parts: vec![part([0, 0, 8, 8], 0)],
            sgroups: vec![sgroup(0, 1)],
            anims: vec![anim(FLAGS_STATIC, 0, (0, 10), 0, (1.0, 1.0), (0x8080_8080, 0x8080_8080))],
            agroups: (0..groups).map(|_| agroup(0, 1, false, 0)).collect(),
        }
    }

    fn layd(seqs: &[Vec<u8>]) -> Vec<u8> {
        // header 0x20, 2 L1 elements (0x18), 1 L2 group (0x14), the SEQD table, the SEQDs.
        let l1 = 0x20;
        let l2 = l1 + 2 * 0x18;
        let table = l2 + 0x14;
        let mut d = Vec::new();
        d.extend_from_slice(b"LAYD");
        for v in [0x100u32, 2, l1 as u32, 1, l2 as u32, seqs.len() as u32, table as u32] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.resize(table, 0x11);
        let mut pos = table + 4 * seqs.len();
        for s in seqs {
            d.extend_from_slice(&(pos as u32).to_le_bytes());
            pos += s.len();
        }
        for s in seqs {
            d.extend_from_slice(s);
        }
        d
    }

    #[test]
    fn container_round_trip_and_alignment() {
        let mk = |n: &str| {
            let mut raw = [0u8; 0x18];
            raw[..n.len()].copy_from_slice(n.as_bytes());
            raw
        };
        let c = Container {
            entries: vec![
                Entry { name_raw: mk("gauge.lad"), flag: true, data: vec![1; 13] },
                Entry { name_raw: mk("Cockpit.lad"), flag: true, data: vec![2; 32] },
                Entry { name_raw: mk("layout_card.lad"), flag: false, data: vec![3; 5] },
            ],
        };
        let b = c.write();
        assert_eq!(u32_at(&b, 0).unwrap(), 0x80, "3 files + the zero slot");
        assert_eq!(u32_at(&b, 0x20).unwrap(), 0x90, "13 bytes, next file 16-aligned");
        assert_eq!(u32_at(&b, 0x3C).unwrap(), 0x8000_0020);
        assert_eq!(b.len() % 16, 0);
        let back = Container::parse(&b).unwrap();
        assert_eq!(back, c);
        assert_eq!(back.find("COCKPIT.LAD").unwrap().data, vec![2; 32]);
    }

    #[test]
    fn seqd_and_layd_round_trip() {
        let s = seqd(3);
        let b = s.write();
        assert_eq!(Seqd::parse(&b).unwrap(), s);
        let l = layd(&[b.clone(), seqd(1).write()]);
        let p = Layd::parse(&l).unwrap();
        assert_eq!(p.write(), l, "unchanged layouts write back byte for byte");
        assert!(Seqd::parse(&b[..b.len() - 1]).is_err());
    }

    #[test]
    fn bar_groups_follow_re_coms_and_scrub_by_kh1_units() {
        let mut s = seqd(BASE_GROUPS);
        let (sp, pa, sg, an) = (s.sprites.len(), s.parts.len(), s.sgroups.len(), s.anims.len());
        let first = add_bar(&mut s, &look(), [128, 128, 128, 255]).unwrap();
        assert_eq!(first, BASE_GROUPS);
        assert_eq!(s.agroups.len(), BASE_GROUPS + group::COUNT);
        assert_eq!((s.sprites.len(), s.parts.len(), s.sgroups.len()), (sp + 4, pa + 5, sg + 4));
        // track 20 + MP 20 + charge 20 grow + 19 full + 4 fades × 2 + static 1.
        assert_eq!(s.anims.len(), an + 20 + 20 + 39 + 8 + 1);
        let g = |k: usize| {
            let r = s.agroups[first + k];
            (i16::from_le_bytes([r[0], r[1]]) as usize, i16::from_le_bytes([r[2], r[3]]) as usize, i16::from_le_bytes([r[4], r[5]]))
        };
        let a = |i: usize, at: usize| i32_at(&s.anims[i], at).unwrap();
        let f = |i: usize, at: usize| f32::from_le_bytes(s.anims[i][at..at + 4].try_into().unwrap());
        // MP slot 2 (third point): from frame 90 = 3 × 30 on, at x = 1 + 52.
        let (m0, mn, once) = g(group::MP);
        assert_eq!((mn, once), (20, 1));
        assert_eq!((a(m0 + 2, 8), a(m0 + 2, 0xC), a(m0 + 2, 0x10)), (90, 600, X0 + 52));
        // Charge slot 1 grows over frames 30..60 (X scale 0 → 1), then a full piece from 61.
        let (c0, cn, _) = g(group::CHARGE);
        assert_eq!(cn, 39);
        assert_eq!((a(c0 + 2, 0), a(c0 + 2, 8), a(c0 + 2, 0xC), f(c0 + 2, 0x50), f(c0 + 2, 0x54)), (FLAGS_GROW as i32, 30, 60, 0.0, 1.0));
        assert_eq!((a(c0 + 3, 8), a(c0 + 3, 0xC)), (61, 600));
        assert_eq!((a(c0 + 38, 8), a(c0 + 38, 0xC)), (570, 600), "the last slot only grows");
        // The fades: 9 drawn steps (frames 0..8), done at 10 (500 units).
        let (i0, _, _) = g(group::MP_IN);
        assert_eq!((a(i0, 8), a(i0, 0xC), u32_at(&s.anims[i0], 0x88).unwrap() >> 24, u32_at(&s.anims[i0], 0x8C).unwrap() >> 24), (0, 8, 115, 13));
        assert_eq!((a(i0 + 1, 8), a(i0 + 1, 0xC), u32_at(&s.anims[i0 + 1], 0x88).unwrap() >> 24), (9, 10, 0));
        let (o0, _, _) = g(group::MP_OUT);
        assert_eq!(u32_at(&s.anims[o0], 0x88).unwrap(), 0x8073_7373);
        let (h0, _, _) = g(group::CHARGE_IN);
        assert_eq!((a(h0 + 1, 0xC), u32_at(&s.anims[h0 + 1], 0x88).unwrap()), (LONG, 0x8080_8080), "gained charge is held");
        // Sprites: the neutral patch, KH1's colours as vertex colours (TL, TR, BL, BR).
        let blue = &s.sprites[sp + 2];
        assert_eq!(&blue[4..20], &[254i32, 47, 255, 48].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>()[..]);
        assert_eq!(u32_at(blue, 0x1C).unwrap(), 0x80E0_892D);
        assert_eq!(u32_at(blue, 0x24).unwrap(), 0x80BB_4147);
        // A second patch is refused (the count is no longer Re:CoM's).
        assert!(add_bar(&mut s, &look(), [128, 128, 128, 255]).is_err());
    }

    #[test]
    fn layout_patch_keeps_the_other_sets() {
        let other = seqd(2).write();
        let l = layd(&[seqd(BASE_GROUPS).write(), other.clone()]);
        let (out, first) = patch_layout(&l, &look(), [128, 128, 128, 255]).unwrap();
        assert_eq!(first, BASE_GROUPS);
        let p = Layd::parse(&out).unwrap();
        assert_eq!(p.seqs[1], other);
        assert_eq!(Seqd::parse(&p.seqs[0]).unwrap().agroups.len(), BASE_GROUPS + group::COUNT);
        // As an asset: a language's layout is patched by name, anything else kept.
        assert!(matches!(patch_asset("UK_Cockpit.lad", &l, &look(), [128; 4]).unwrap().1, Patched::Bar(_)));
        assert_eq!(patch_asset("UK_gauge.lad", &l, &look(), [128; 4]).unwrap(), (l.clone(), Patched::Kept));
        assert_eq!(patch_asset("x.dds", b"DDS ", &look(), [128; 4]).unwrap().1, Patched::Kept);
    }

    #[test]
    fn vertex_colour_shows_kh1_on_the_neutral_texel() {
        // Texel 128 (Re:CoM's): vertex = KH1's colour.
        assert_eq!(vertex_colour([45, 137, 224, 128], [128, 128, 128, 255]), 0x80E0_892D);
        // Texel 255: halved; texel 64: doubled, at most 255.
        assert_eq!(vertex_colour([200, 100, 0, 128], [255; 4]), abgr([100, 50, 0], 0x80));
        assert_eq!(vertex_colour([200, 100, 0, 128], [64; 4]), abgr([255, 200, 0], 0x80));
    }

    fn tim2(w: u16, h: u16, itype: u8, pixels: &[u8], colours: &[[u8; 4]]) -> Vec<u8> {
        let clut = colours.len() * 4;
        let total = 0x30 + pixels.len() + clut;
        let mut out = Vec::new();
        out.extend_from_slice(b"TIM2");
        out.extend_from_slice(&[4, 0, 1, 0]);
        out.extend_from_slice(&[0; 8]);
        for v in [total as u32, clut as u32, pixels.len() as u32] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&0x30u16.to_le_bytes());
        out.extend_from_slice(&(colours.len() as u16).to_le_bytes());
        out.extend_from_slice(&[0, 1, 3, itype]);
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&h.to_le_bytes());
        out.extend_from_slice(&[0; 24]);
        out.extend_from_slice(pixels);
        let mut cl = colours.to_vec();
        if colours.len() == 256 {
            for (i, c) in colours.iter().enumerate() {
                cl[crate::rtm::csm1(i)] = *c;
            }
        }
        cl.iter().for_each(|c| out.extend_from_slice(c));
        out
    }

    #[test]
    fn kh1_look_reads_the_strips() {
        // Pictures 0..4: 4-bit, 0x2080 bytes; 5: 8-bit gauge; 6: 4-bit; 7: 4-bit background.
        let blank4 = tim2(128, 128, 4, &[0u8; 8192], &[[0u8; 4]; 16]);
        assert_eq!(blank4.len(), 0x2080);
        let mut pal = [[0u8; 4]; 256];
        let mut px = vec![0u8; 128 * 128];
        for (k, y) in (102..114).enumerate() {
            pal[1 + k] = [45 + 2 * k as u8, 137 - 6 * k as u8, 224 - 3 * k as u8, 0x80];
            (30..54).for_each(|x| px[y * 128 + x] = 1 + k as u8);
        }
        for (k, y) in (116..120).enumerate() {
            pal[20 + k] = [254, 136 + 10 * k as u8, 1, 0x80];
            (30..54).for_each(|x| px[y * 128 + x] = 20 + k as u8);
        }
        let gauge = tim2(128, 128, 5, &px, &pal);
        assert_eq!(gauge.len(), 0x4440);
        let mut bpx = vec![0u8; 64 * 128];
        let bpal = [[0, 0, 0, 0], [0, 0, 0, 0x80], [24, 28, 42, 0x80], [0; 4], [0; 4], [0; 4], [0; 4], [0; 4], [0; 4], [0; 4], [0; 4], [0; 4], [0; 4], [0; 4], [0; 4], [0; 4]];
        for y in 85..100 {
            for x in 8..34 {
                let v = if y < 87 || y > 97 { 1 } else { 2 };
                let i = y * 128 + x;
                bpx[i / 2] |= if i % 2 == 0 { v } else { v << 4 };
            }
        }
        let back = tim2(128, 128, 4, &bpx, &bpal);
        let mut file = Vec::new();
        for _ in 0..5 {
            file.extend_from_slice(&blank4);
        }
        file.extend_from_slice(&gauge);
        file.extend_from_slice(&blank4);
        file.extend_from_slice(&back);
        let l = kh1_look(&file).unwrap();
        assert_eq!((l.slot_w, l.mp_h, l.charge_h, l.border_h), (24, 12, 4, 2));
        assert_eq!((l.mp_top, l.mp_bottom), ([45, 137, 224, 0x80], [67, 71, 191, 0x80]));
        assert_eq!((l.charge_top, l.charge_bottom), ([254, 136, 1, 0x80], [254, 166, 1, 0x80]));
        assert_eq!((l.back, l.border), ([24, 28, 42, 0x80], [0, 0, 0, 0x80]));
        // A file where picture 5 is somewhere else is refused (KH1 draws it from 0xA280).
        assert!(kh1_look(&file[0x2080..]).is_err());
    }

    #[test]
    fn swizzle_is_a_permutation() {
        let n = 256 * 256;
        let src: Vec<u8> = (0..n).map(|i| (i * 7 + i / 256) as u8).collect();
        let a = decode8(&encode32(&src, 2, 4), 2, 4);
        let mut x = a.clone();
        let mut y = src.clone();
        x.sort_unstable();
        y.sort_unstable();
        assert_eq!(x, y, "same bytes, moved");
        // Distinct positions: decode the index ramp and check every slot is hit once.
        let n = n / 4;
        let ramp: Vec<u32> = (0..n as u32).collect();
        let bytes: Vec<u8> = ramp.iter().flat_map(|v| v.to_le_bytes()).collect();
        let e = encode32(&bytes, 2, 4);
        let mut seen = vec![false; n];
        for c in e.chunks(4) {
            let v = u32::from_le_bytes(c.try_into().unwrap()) as usize;
            assert!(!seen[v]);
            seen[v] = true;
        }
    }

    /// With the owner's files (`KH1_RECOM_FORM` = a folder with `FO0002.SPR`, `FO0002.RTM` and
    /// `kh1_command2_uitex.bin`): the real numbers. Skipped when unset.
    #[test]
    fn owner_files() {
        let Ok(dir) = std::env::var("KH1_RECOM_FORM") else { return };
        let dir = std::path::Path::new(&dir);
        let uitex = std::fs::read(dir.join("kh1_command2_uitex.bin")).unwrap();
        let l = kh1_look(&uitex).unwrap();
        assert_eq!((l.slot_w, l.mp_h, l.charge_h, l.border_h), (24, 12, 4, 2));
        assert_eq!((l.mp_top, l.mp_bottom), ([45, 137, 224, 127], [71, 65, 187, 128]));
        assert_eq!((l.charge_top, l.charge_bottom), ([254, 136, 1, 128], [255, 173, 2, 126]));
        assert_eq!((l.back, l.border), ([24, 28, 42, 128], [0, 0, 0, 128]));
        let rtm = std::fs::read(dir.join("FO0002.RTM")).unwrap();
        assert_eq!(neutral_texel(&rtm).unwrap(), [128, 128, 128, 128]);
        let spr = std::fs::read(dir.join("FO0002.SPR")).unwrap();
        let c = Container::parse(&spr).unwrap();
        let w = c.write();
        assert_eq!((w.len(), w.iter().zip(&spr).position(|(a, b)| a != b)), (spr.len(), None), "Re:CoM's container writes back byte for byte");
        for e in &c.entries {
            let l = Layd::parse(&e.data).unwrap();
            assert!(l.write() == e.data, "{}", e.name());
            for s in &l.seqs {
                assert!(Seqd::parse(s).unwrap().write() == *s, "{}", e.name());
            }
        }
        let (out, first) = patch_spr(&spr, &look(), [128, 128, 128, 128]).unwrap();
        assert_eq!(first, BASE_GROUPS);
        let p = Container::parse(&out).unwrap();
        assert_eq!(p.find("gauge.lad").unwrap().data, c.find("gauge.lad").unwrap().data);
        let s = Seqd::parse(&Layd::parse(&p.find(LAYOUT).unwrap().data).unwrap().seqs[0]).unwrap();
        assert_eq!(s.agroups.len(), BASE_GROUPS + group::COUNT);
        if let Ok(o) = std::env::var("KH1_MP_BAR_OUT") {
            std::fs::write(o, &out).unwrap();
        }
    }
}
