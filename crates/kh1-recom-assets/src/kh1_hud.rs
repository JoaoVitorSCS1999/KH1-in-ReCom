//! The KH1 HUD sprite set `kh1hud` (decision 39; stages H1 / H2, `docs/recom-kh1-hud.md`): KH1's
//! own HUD pictures and layouts turned into Re:CoM 2D data, added to Re:CoM's HUD files so
//! Re:CoM registers, animates and draws them.
//!
//! - `FORM/0002/FO0002.SPR` gets a 4th file `kh1hud.lad` (one SEQD, registered as `kh1hud:00`)
//!   and PC assets `<LANG>_kh1hud.lad` (the same records with the HD atlas' UVs);
//!   `FORM/0002/FO0002.RTM` gets `kh1hud.imz` and PC assets `<LANG>_kh1hud.imz` (the HD atlas,
//!   per language: KH1's language pictures). Re:CoM loads FO0002 with the player HUD and takes a
//!   PC asset whose name contains the file's name (`0x14029FCC0`), as for `Cockpit.lad` /
//!   `UK_Cockpit.imz`. Checked (decomp + files).
//! - Textures: one 32-bit IMGD per atlas (PSMCT32, linear, PS2 alpha 0x80 = opaque), the form of
//!   Re:CoM's own PC atlases (`UK_Cockpit.imz`: 1024 × 1024, header words checked against it).
//! - Sora's gauge cluster (H2): KH1's objects (`exchange/UK_uibin_g_s.bin`) at KH1's positions,
//!   and KH1's arcs (`0x14026B790` / `0x14026E450`: a triangle fan from the gauge centre over the
//!   square of half-size R, UVs fixed in the quarter-ring texture) baked by running KH1's own fan
//!   here into pieces (one per quadrant, per 5° / 30° / 1° step as KH1's values move), so that
//!   Re:CoM only shows pieces by frame. Frames are KH1's quantities (HP, MP, charge units).
//!
//! Nothing from either game is stored in the repository: every texel and position is read from
//! the player's files at this step.

use std::fmt::Write as _;

use kh1_formats::egs::HdAsset;

use crate::mp_bar::{self as mb, agroup, anim, part, sgroup, sprite, Container, Entry, Seqd, FLAGS_COLOUR, FLAGS_STATIC, GROUP_WORD_FADE, GROUP_WORD_SCRUB};
use crate::texture::{decode_dds, Image};
use crate::uibin::{Obj, Uibin};
use crate::{bad, Result};

/// The layout file added to `FO0002.SPR` (Re:CoM registers `kh1hud` and `kh1hud:00`).
pub const LAD: &str = "kh1hud.lad";
/// The atlas added to `FO0002.RTM` (registered as `kh1hud:00`, bound to the sequence set).
pub const IMZ: &str = "kh1hud.imz";
/// The sprite set's name in Re:CoM's 2D registry.
pub const SET: &str = "kh1hud:00";
/// The manifest the DLL reads.
pub const MANIFEST: &str = "kh1-hud.json";
/// The manifest's format line.
pub const FORMAT: &str = "kh1-recom-assets kh1-hud 2";
/// KH1's layout of Sora's gauge (`0x140268800` loads it; `UK` is English, `0x1400D7380`).
pub const G_S: &str = "exchange/UK_uibin_g_s.bin";
/// The danger flash object (`g_s_bar_b_hp_red`, slot `+0x4E0`).
pub const G_RED: &str = "exchange/UK_uibin_g_red.bin";
/// KH1's gauge motions (`vibe_s/ss/sss` the hit shakes, `slide_s_h/v` the size-class slides; slot
/// 0 copies objects 0..4 to `+0x2A0`, `+0x360`, `+0x420`, `+0x120`, `+0x1E0`, `0x14026E630` area).
pub const G_ACT: &str = "exchange/UK_uibin_g_act.bin";
/// The party's model files whose section 2 is the face strip: Sora (Checked (files)), Donald
/// and Goofy (Checked (files: their 48 × 48 faces in the owner's dump)).
pub const FACES: [&str; 3] = ["xa_ex_0010.mdls", "xa_ex_0040.mdls", "xa_ex_0050.mdls"];
/// `uitex.bin` pictures used: (index, TIM2 offset, pixel data offset = the PC asset tag).
pub const PICTURES: [(usize, usize, usize); 3] = [(5, 0xA280, 0xA2C0), (6, 0xE6C0, 0xE700), (7, 0x10740, 0x10780)];
/// Long hold for held animation steps (as `mp_bar::LONG`).
pub const LONG: i32 = 10_000_000;

/// KH1 constants (Checked (exe) unless noted).
pub mod kh1 {
    /// Sora's gauge centre `0x1405272B0`.
    pub const CENTRE: (i32, i32) = (443, 316);
    /// Size-class offsets `0x140527300` (class 0, 1, 2).
    pub const CLASS_OFFSET: [(i32, i32); 3] = [(24, 24), (0, 24), (0, 0)];
    /// Ring table `0x1405273B0` (1/16 units: u, v, radius): HP back, MP back (picture 7), HP, MP,
    /// charge (picture 5).
    pub const RING_HP_BACK: (i32, i32, i32) = (744, 1288, 688);
    /// MP back ring.
    pub const RING_MP_BACK: (i32, i32, i32) = (1720, 1512, 912);
    /// HP ring.
    pub const RING_HP: (i32, i32, i32) = (680, 1592, 656);
    /// MP ring.
    pub const RING_MP: (i32, i32, i32) = (904, 904, 880);
    /// Charge ring.
    pub const RING_CHARGE: (i32, i32, i32) = (1816, 904, 880);
    /// HP: 5° a point to 270° (54 HP), then 6 HP a straight segment, at most 120 HP.
    pub const HP_DEG: i32 = 5;
    /// HP filling the ring.
    pub const HP_RING: i32 = 54;
    /// HP a straight segment.
    pub const HP_PER_SEG: i32 = 6;
    /// HP drawn at most (`0x78`).
    pub const HP_CAP: i32 = 120;
    /// MP: 30° a point to 270° (9 MP), then a segment a point, at most 20.
    pub const MP_DEG: i32 = 30;
    /// MP filling the ring.
    pub const MP_RING: i32 = 9;
    /// MP drawn at most (`0x14`).
    pub const MP_CAP: i32 = 20;
    /// Charge: 1° a unit to 270, then 30 units a segment, at most 600.
    pub const CHARGE_RING: i32 = 270;
    /// Units a straight segment.
    pub const CHARGE_PER_SEG: i32 = 30;
    /// Units drawn at most (`0x258`).
    pub const CHARGE_CAP: i32 = 600;
    /// Straight segment pitch (Sora; party 17).
    pub const PITCH: i32 = 26;
    /// The HP / MP back caps' x (`0x1402698C0`: 0x19E; `0x140269F50`: 0x1B8).
    pub const HP_CAP_X: i32 = 0x19E;
    /// The MP cap's x.
    pub const MP_CAP_X: i32 = 0x1B8;
    /// Hurt face times (`0x1405270FC`, `0x140527134`, `0x14052726C`), 60 Hz frames.
    pub const HURT_FRAMES: [i32; 3] = [40, 50, 60];
}

/// Object indices in `g_s` (`0x1405270E0`, `0x140527138`, `0x140527168`, `0x140527180`; Checked
/// (exe) and their names Checked (files)).
pub mod obj {
    /// `g_s_fra_a_wa..wd`, drawn with picture 6 (`0x140269190`).
    pub const FACE_FRAME: [usize; 4] = [0, 1, 2, 3];
    /// `g_s_fra_a_a..d` (the face disc), `g_s_fra_a_hp`, `g_s_fra_a_mp` (picture 7).
    pub const DISC: [usize; 4] = [4, 5, 6, 7];
    /// "HP".
    pub const LABEL_HP: usize = 8;
    /// "mp".
    pub const LABEL_MP: usize = 9;
    /// Back segments: first HP (`+0xA8`), further HP (`+0xB0`), MP (`+0xB8`), caps (`+0xC0`, `+0xC8`).
    pub const BACK_HP_FIRST: usize = 16;
    /// Further HP back segments.
    pub const BACK_HP: usize = 18;
    /// MP back segments.
    pub const BACK_MP: usize = 17;
    /// HP back cap.
    pub const CAP_HP: usize = 20;
    /// MP back cap.
    pub const CAP_MP: usize = 21;
    /// Colour segments: HP (`+0x90`), MP (`+0x98`), charge (`+0xA0`).
    pub const SEG_HP: usize = 31;
    /// MP segment.
    pub const SEG_MP: usize = 32;
    /// Charge segment.
    pub const SEG_CHARGE: usize = 33;
    /// Max-MP ticks past 90° / 180° (`+0x78`, `+0x80`).
    pub const TICK_90: usize = 37;
    /// Past 180°.
    pub const TICK_180: usize = 38;
    /// The count `g_s` must have.
    pub const COUNT: usize = 39;
}

/// The animation groups of `kh1hud:00`. Held groups take KH1's value as their frame; the rest
/// play over time. The DLL (`crate::kh1_hud` there) uses the same numbers; the manifest carries
/// the count and the format, which it checks.
pub mod group {
    /// Face frame, disc, labels: frame 0 without "mp", 1 with.
    pub const FRAME: usize = 0;
    /// Back arcs and segments: frame = min(max HP, 120) / min(max MP, 20).
    pub const HP_BACK: usize = 1;
    /// MP back.
    pub const MP_BACK: usize = 2;
    /// MP: frame = min(MP, 20).
    pub const MP: usize = 3;
    /// HP: frame = min(HP, 120).
    pub const HP: usize = 4;
    /// Charge: frame = min(charge, 600).
    pub const CHARGE: usize = 5;
    /// Max-MP ticks: frame = min(max MP, 20).
    pub const TICKS: usize = 6;
    /// Face: frame = expression (0, 1 hurt, 2 danger).
    pub const FACE: usize = 7;
    /// Hurt face for 40 / 50 / 60 frames, then expression 0 or 2 (played): 8 + 2 i + (danger).
    pub const HURT: usize = 8;
    /// MP fades of cell j (played): gain 14 + j, loss 23 + j.
    pub const MP_GAIN: usize = 14;
    /// MP loss fades.
    pub const MP_LOSS: usize = 23;
    /// Charge fades of the piece [30 j, 30 j + t] (t = 1..30, played): gain 32 + 30 j + t − 1,
    /// loss 302 + 30 j + t − 1.
    pub const CHARGE_GAIN: usize = 32;
    /// Charge loss fades.
    pub const CHARGE_LOSS: usize = 302;
    /// Danger flash (looped) of the HP arc piece [90 q, 90 q + 5 k] (k = 1..18): 572 + 18 q + k − 1.
    pub const RED_ARC: usize = 572;
    /// Danger flash of a full straight HP segment, and of a partial one (remainder r = 1..5:
    /// 627 + r − 1).
    pub const RED_SEG: usize = 626;
    /// Partial segment flashes.
    pub const RED_SEG_PART: usize = 627;
    /// The fixed groups (the units' motion groups follow, `motion`).
    pub const COUNT: usize = 632;
}

// ---------------------------------------------------------------------------------------------
// Pictures.

/// RGBA, PS2 alpha (0x80 = opaque), rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pic {
    /// Width.
    pub w: usize,
    /// Height.
    pub h: usize,
    /// Pixels.
    pub px: Vec<[u8; 4]>,
}

impl Pic {
    fn blank(w: usize, h: usize) -> Pic {
        Pic { w, h, px: vec![[0; 4]; w * h] }
    }
    fn at(&self, x: i64, y: i64) -> [u8; 4] {
        let x = x.clamp(0, self.w as i64 - 1) as usize;
        let y = y.clamp(0, self.h as i64 - 1) as usize;
        self.px[y * self.w + x]
    }
    /// A copy of the rect (x, y, w, h).
    fn crop(&self, x: usize, y: usize, w: usize, h: usize) -> Pic {
        let mut p = Pic::blank(w, h);
        for j in 0..h {
            for i in 0..w {
                p.px[j * w + i] = self.at((x + i) as i64, (y + j) as i64);
            }
        }
        p
    }
    fn blit(&mut self, src: &Pic, x: usize, y: usize) {
        for j in 0..src.h {
            let o = (y + j) * self.w + x;
            self.px[o..o + src.w].copy_from_slice(&src.px[j * src.w..(j + 1) * src.w]);
        }
    }
}

/// A PC texture (alpha 0..255) as a PS2-alpha picture.
pub fn from_pc(img: &Image) -> Pic {
    Pic { w: img.w, h: img.h, px: img.px.iter().map(|p| [p[0], p[1], p[2], ((p[3] as u32 * 128 + 127) / 255) as u8]).collect() }
}

/// A KH1 picture: the PS2 one and its PC texture (and the PC texture's scale over the PS2 one).
#[derive(Clone, Debug)]
pub struct Source {
    /// The PS2 picture.
    pub ps2: Pic,
    /// The PC texture.
    pub hd: Pic,
    /// The PC texture's scale over the PS2 picture.
    pub scale: usize,
}

fn hd_scale(ps2: &Pic, hd: &Pic) -> Result<usize> {
    let s = hd.w / ps2.w.max(1);
    if s == 0 || hd.w != ps2.w * s || hd.h < ps2.h * s {
        return bad(format!("PC texture {}×{} is not a whole multiple of the PS2 picture {}×{}", hd.w, hd.h, ps2.w, ps2.h));
    }
    Ok(s)
}

/// Finds a `uitex.bin` PC asset tagged / named for pixel offset `off` (`<LANG>_uitex_bin<hex>` for
/// the language, else `-<hex>`), decoded.
fn uitex_asset(assets: &[HdAsset], off: usize, lang: &str) -> Result<Option<Pic>> {
    let names = [format!("{lang}_uitex_bin{off:x}.dds"), format!("-{off:x}.dds")];
    for n in &names {
        if let Some(a) = assets.iter().find(|a| a.name.eq_ignore_ascii_case(n)) {
            return Ok(Some(from_pc(&decode_dds(&a.data).map_err(|e| crate::Error::Format(format!("uitex.bin asset {}: {e}", a.name)))?)));
        }
    }
    Ok(None)
}

/// A face strip from section 2 of a `.mdls` (`{w, h, 0, 0}`, CSM1 CLUT at +0x10, 8-bit pixels at
/// +0x410; Checked (files: Sora 192 × 64, Donald / Goofy 48 × 48)) and its PC texture
/// `-<hex pixel offset>.dds`.
pub fn face(mdls: &[u8], assets: &[HdAsset]) -> Result<Source> {
    let rd = |at: usize| -> Result<usize> { mdls.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize).ok_or_else(|| crate::Error::Format("mdls: short".into())) };
    let n = rd(0)?;
    if !(3..=64).contains(&n) {
        return bad(format!("mdls: {n} sections"));
    }
    let s2 = rd(12)?;
    face_section(mdls.get(s2..).ok_or_else(|| crate::Error::Format("mdls: section 2 outside the file".into()))?, s2, assets)
}

/// A face strip from a `.mdls` section 2 that starts at `at` in its file.
pub fn face_section(sec: &[u8], at: usize, assets: &[HdAsset]) -> Result<Source> {
    let rd = |o: usize| -> Result<usize> { sec.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize).ok_or_else(|| crate::Error::Format("mdls: short section 2".into())) };
    let (w, h) = (rd(0)?, rd(4)?);
    if w == 0 || h == 0 || w > 1024 || h > 1024 || 0x410 + w * h > sec.len() {
        return bad(format!("mdls: section 2 at {at:#x} is not a face ({w}×{h})"));
    }
    let clut = &sec[0x10..0x410];
    let px = sec[0x410..0x410 + w * h]
        .iter()
        .map(|&i| {
            let j = crate::rtm::csm1(i as usize);
            clut[4 * j..4 * j + 4].try_into().unwrap()
        })
        .collect();
    let ps2 = Pic { w, h, px };
    let name = format!("-{:x}.dds", at + 0x410);
    let a = assets.iter().find(|a| a.name.eq_ignore_ascii_case(&name)).ok_or_else(|| crate::Error::Format(format!("mdls: no PC face texture {name}")))?;
    let hd = from_pc(&decode_dds(&a.data)?);
    let scale = hd_scale(&ps2, &hd)?;
    Ok(Source { ps2, hd, scale })
}

// ---------------------------------------------------------------------------------------------
// KH1's arc fan (`0x14026B790`, vertices `0x14026E450`), all in 1/16 units.

/// A ring: the gauge centre on screen and the quarter-ring's centre and radius in its picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ring {
    /// Screen centre x (1/16 units).
    pub x: i32,
    /// Screen centre y.
    pub y: i32,
    /// Texture centre u.
    pub uc: i32,
    /// Texture centre v.
    pub vc: i32,
    /// Radius.
    pub r: i32,
}

/// The fan vertex at `theta` degrees (0 = 9 o'clock, clockwise): (screen x, y; u, v), 1/16 units.
/// `0x14026E450`: d = trunc(tan(t) · R) with t the angle in the 45° octant (mirrored in odd
/// ones); the right / bottom halves are one unit further out (`+ 0x10` when d ≠ 0 at their start).
pub fn vertex(ring: &Ring, theta: f32) -> [i32; 4] {
    let k = (theta / 45.0) as u32;
    let mut t = theta - k as f32 * 45.0;
    if k & 1 != 0 {
        t = 45.0 - t;
    }
    let d = ((t * std::f32::consts::PI / 180.0).tan() * ring.r as f32) as i32;
    let s = if d != 0 { 0x10 } else { 0 };
    let (x, y, r, uc, vc) = (ring.x, ring.y, ring.r, ring.uc, ring.vc);
    match k {
        0 => [x - r, y - d, uc - r, vc - d],
        1 => [x - d, y - r, uc - d, vc - r],
        2 => [x + s + d, y - r, uc - d, vc - r],
        3 => [x + 0x10 + r, y - d, uc - r, vc - d],
        4 => [x + 0x10 + r, y + s + d, uc - r, vc - d],
        _ => [x + s + d, y + 0x10 + r, uc - d, vc - r],
    }
}

/// The fan `0x14026B790` draws for [a0, a1]: the centre (one unit down; UV the ring's centre), then
/// the vertex at a0, at every 45° past it up to a1, and at a1.
pub fn fan(ring: &Ring, a0: f32, a1: f32) -> Vec<[i32; 4]> {
    fan_to(ring, a0, a1)
}

fn fan_to(ring: &Ring, a0: f32, f: f32) -> Vec<[i32; 4]> {
    let mut v = vec![[ring.x, ring.y + 0x10, ring.uc, ring.vc], vertex(ring, a0)];
    let mut i = (a0 / 45.0) as i32;
    while {
        i += 1;
        i <= (f / 45.0) as i32
    } {
        v.push(vertex(ring, i as f32 * 45.0));
    }
    v.push(vertex(ring, f));
    v
}

/// KH1's soft end (`0x14026B790` mode 4, the HP, flash and charge arcs; Checked (decomp + exe: the
/// callers push 4, the constant 0.7 at `0x14042F7F4` is subtracted from the end)): when 1 < a1 <
/// 270 the fan stops at a1 − 0.7°, and one more triangle (centre, a1 − 0.7°, a1) is drawn with
/// the a1 vertex's alpha 0 (Gouraud: the edge fades out over the last 0.7°).
pub const SOFT_DEG: f32 = 0.7;

/// The fan and, with KH1's soft end, its fading wedge (centre, a1 − 0.7°, a1).
pub fn fan_soft(ring: &Ring, a0: f32, a1: f32, soft: bool) -> (Vec<[i32; 4]>, Option<[[i32; 4]; 3]>) {
    if soft && a1 < 270.0 && a1 > 1.0 {
        let f = a1 - SOFT_DEG;
        let v = fan_to(ring, a0, f);
        let w = [v[0], vertex(ring, f), vertex(ring, a1)];
        (v, Some(w))
    } else {
        (fan_to(ring, a0, a1), None)
    }
}

/// Renders a fan: the texel each sample point inside it shows. Samples at `(x0 + (i + ½) / s − ½,
/// y0 + (j + ½) / s − ½)` units (s = 1: the unit grid, where the GS samples). A point on the last
/// radial edge (the centre to the a1 vertex) is outside, so pieces meeting there don't overlap.
fn render(fan: &[[i32; 4]], wedge: Option<&[[i32; 4]; 3]>, src: &Pic, src_scale: usize, s: usize, rect: (i32, i32, i32, i32)) -> Pic {
    let (x0, y0, w, h) = rect;
    let mut p = Pic::blank(w as usize * s, h as usize * s);
    let c = fan[0];
    let n = fan.len();
    // Triangles (centre, a, b) and whether b's alpha is 0 (the soft wedge's end vertex).
    let mut tris: Vec<([i32; 4], [i32; 4], [i32; 4], bool)> = (1..n - 1).map(|k| (c, fan[k], fan[k + 1], false)).collect();
    if let Some(wd) = wedge {
        tris.push((wd[0], wd[1], wd[2], true));
    }
    let area = |t: &([i32; 4], [i32; 4], [i32; 4], bool)| (t.1[0] - t.0[0]) as i64 * (t.2[1] - t.0[1]) as i64 - (t.1[1] - t.0[1]) as i64 * (t.2[0] - t.0[0]) as i64;
    // The last triangle that has an area (KH1 repeats the end vertex when it is a 45° step).
    let last = (0..tris.len()).rev().find(|&k| area(&tris[k]) != 0).unwrap_or(0);
    for j in 0..h as usize * s {
        for i in 0..w as usize * s {
            let px = (x0 as f64 + (i as f64 + 0.5) / s as f64 - 0.5) * 16.0;
            let py = (y0 as f64 + (j as f64 + 0.5) / s as f64 - 0.5) * 16.0;
            for (k, &(c, a, b, soft)) in tris.iter().enumerate() {
                // Barycentric weights of a and b over the triangle (c, a, b).
                let det = ((a[0] - c[0]) as f64) * ((b[1] - c[1]) as f64) - ((a[1] - c[1]) as f64) * ((b[0] - c[0]) as f64);
                if det == 0.0 {
                    continue;
                }
                let la = ((px - c[0] as f64) * (b[1] - c[1]) as f64 - (py - c[1] as f64) * (b[0] - c[0]) as f64) / det;
                let lb = ((a[0] - c[0]) as f64 * (py - c[1] as f64) - (a[1] - c[1]) as f64 * (px - c[0] as f64)) / det;
                let lc = 1.0 - la - lb;
                const E: f64 = 1e-9;
                if la < -E || lb < -E || lc < -E {
                    continue;
                }
                // The last radial edge (c → b of the last triangle, where la = 0) is outside.
                if k == last && la <= E {
                    continue;
                }
                let u = lc * c[2] as f64 + la * a[2] as f64 + lb * b[2] as f64;
                let v = lc * c[3] as f64 + la * a[3] as f64 + lb * b[3] as f64;
                let tu = (u * src_scale as f64 / 16.0).floor() as i64;
                let tv = (v * src_scale as f64 / 16.0).floor() as i64;
                let mut t = src.at(tu, tv);
                if soft {
                    // Gouraud alpha: 1 at c and a, 0 at b (the GS's vertex alpha × texel alpha / 128).
                    t[3] = ((t[3] as f64) * (1.0 - lb).clamp(0.0, 1.0)).round() as u8;
                }
                p.px[j * w as usize * s + i] = t;
                break;
            }
        }
    }
    p
}

/// A baked arc piece: its screen rect (units) and its PS2 / PC pictures.
#[derive(Clone, Debug)]
pub struct Baked {
    /// x, y, w, h (units).
    pub rect: (i32, i32, i32, i32),
    /// The PS2 picture.
    pub ps2: Pic,
    /// The PC pictures (per language source).
    pub hd: Vec<Pic>,
}

/// Bakes the fan [a0, a1] of `ring` from `src` (per language for the PC pictures), trimmed to
/// what any of them shows. None when it shows nothing.
pub fn bake(ring: &Ring, a0: f32, a1: f32, src: &[&Source]) -> Option<Baked> {
    bake_soft(ring, a0, a1, false, src)
}

/// [`bake`] with KH1's soft end when `soft` (mode 4, see [`fan_soft`]).
pub fn bake_soft(ring: &Ring, a0: f32, a1: f32, soft: bool, src: &[&Source]) -> Option<Baked> {
    let (f, wedge) = fan_soft(ring, a0, a1, soft);
    let mut all = f.clone();
    if let Some(w) = &wedge {
        all.extend_from_slice(w);
    }
    let minx = all.iter().map(|v| v[0]).min().unwrap().div_euclid(16) - 1;
    let maxx = all.iter().map(|v| v[0]).max().unwrap().div_euclid(16) + 2;
    let miny = all.iter().map(|v| v[1]).min().unwrap().div_euclid(16) - 1;
    let maxy = all.iter().map(|v| v[1]).max().unwrap().div_euclid(16) + 2;
    let full = (minx, miny, maxx - minx, maxy - miny);
    let s = src[0].scale;
    let ps2 = render(&f, wedge.as_ref(), &src[0].ps2, 1, 1, full);
    let hds: Vec<Pic> = src.iter().map(|x| render(&f, wedge.as_ref(), &x.hd, x.scale, x.scale, full)).collect();
    // Trim to the units any picture shows.
    let (w, h) = (full.2 as usize, full.3 as usize);
    let mut used = vec![false; w * h];
    for j in 0..h {
        for i in 0..w {
            if ps2.px[j * w + i][3] != 0 {
                used[j * w + i] = true;
            }
        }
    }
    for hd in &hds {
        for j in 0..h * s {
            for i in 0..w * s {
                if hd.px[j * w * s + i][3] != 0 {
                    used[(j / s) * w + i / s] = true;
                }
            }
        }
    }
    let xs: Vec<usize> = (0..w).filter(|&i| (0..h).any(|j| used[j * w + i])).collect();
    let ys: Vec<usize> = (0..h).filter(|&j| (0..w).any(|i| used[j * w + i])).collect();
    let (&ix0, &ix1, &iy0, &iy1) = (xs.first()?, xs.last()?, ys.first()?, ys.last()?);
    let (tw, th) = (ix1 - ix0 + 1, iy1 - iy0 + 1);
    Some(Baked {
        rect: (full.0 + ix0 as i32, full.1 + iy0 as i32, tw as i32, th as i32),
        ps2: ps2.crop(ix0, iy0, tw, th),
        hd: hds.iter().map(|p| p.crop(ix0 * s, iy0 * s, tw * s, th * s)).collect(),
    })
}

// ---------------------------------------------------------------------------------------------
// The atlas.

/// A rect in an atlas: x, y, w, h.
type Slot = (usize, usize, usize, usize);

/// Shelf packing of (w, h) boxes into a power-of-two width; returns each box's place and the
/// atlas size (power-of-two sides).
fn pack(sizes: &[(usize, usize)], width: usize) -> (Vec<Slot>, usize, usize) {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(sizes[i].1), i));
    let mut out = vec![(0, 0, 0, 0); sizes.len()];
    let (mut x, mut y, mut shelf) = (0usize, 0usize, 0usize);
    for i in order {
        let (w, h) = sizes[i];
        if x + w > width {
            x = 0;
            y += shelf;
            shelf = 0;
        }
        out[i] = (x, y, w, h);
        x += w;
        shelf = shelf.max(h);
    }
    let height = (y + shelf).next_power_of_two().max(1);
    (out, width, height)
}

/// What a sprite shows: an atlas box, and the sub-rect of it in PS2 units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sub {
    item: usize,
    rect: [i32; 4],
}

/// The atlas items: each a PS2 picture and its PC pictures (one per language or one shared), at
/// a PC scale.
struct Atlas {
    items: Vec<(Pic, Vec<Pic>, usize)>,
}

impl Atlas {
    fn add(&mut self, ps2: Pic, hd: Vec<Pic>, scale: usize) -> usize {
        self.items.push((ps2, hd, scale));
        self.items.len() - 1
    }
}

/// The built atlases: the PS2 one and the PC one per language, and each item's places.
struct Packed {
    ps2: Pic,
    hd: Vec<Pic>,
    ps2_slots: Vec<Slot>,
    hd_slots: Vec<Slot>,
    scales: Vec<usize>,
}

fn build_atlas(a: &Atlas, langs: usize) -> Result<Packed> {
    let (ps2_slots, pw, ph) = pack(&a.items.iter().map(|i| (i.0.w + 2, i.0.h + 2)).collect::<Vec<_>>(), 1024);
    let (hd_slots, hw, hh) = pack(&a.items.iter().map(|i| (i.0.w * i.2 + 4, i.0.h * i.2 + 4)).collect::<Vec<_>>(), 2048);
    if ph > 2048 || hh > 4096 {
        return bad(format!("kh1hud atlas too big ({pw}×{ph}, PC {hw}×{hh})"));
    }
    let mut ps2 = Pic::blank(pw, ph);
    let mut hd = vec![Pic::blank(hw, hh); langs];
    for (k, (p, h, _)) in a.items.iter().enumerate() {
        // One texel of gap around each box (the PC atlas is filtered).
        ps2.blit(p, ps2_slots[k].0 + 1, ps2_slots[k].1 + 1);
        for (l, atlas) in hd.iter_mut().enumerate() {
            let pic = if h.len() == 1 { &h[0] } else { &h[l] };
            atlas.blit(pic, hd_slots[k].0 + 2, hd_slots[k].1 + 2);
        }
    }
    let ps2_slots = ps2_slots.iter().map(|s| (s.0 + 1, s.1 + 1, s.2 - 2, s.3 - 2)).collect();
    let hd_slots = hd_slots.iter().map(|s| (s.0 + 2, s.1 + 2, s.2 - 4, s.3 - 4)).collect();
    Ok(Packed { ps2, hd, ps2_slots, hd_slots, scales: a.items.iter().map(|i| i.2).collect() })
}

impl Packed {
    /// A sprite's UV rect (exclusive end) in the PS2 atlas and in the PC one.
    fn uv(&self, s: Sub) -> ([i32; 4], [i32; 4]) {
        let (px, py, ..) = self.ps2_slots[s.item];
        let (hx, hy, ..) = self.hd_slots[s.item];
        let k = self.scales[s.item] as i32;
        let [u0, v0, u1, v1] = s.rect;
        ([px as i32 + u0, py as i32 + v0, px as i32 + u1, py as i32 + v1], [hx as i32 + u0 * k, hy as i32 + v0 * k, hx as i32 + u1 * k, hy as i32 + v1 * k])
    }
}

// ---------------------------------------------------------------------------------------------
// Sprite set: pieces shown by frame.

/// A piece: what it shows, where (part rect, KH1 units; x1 < x0 / y1 < y0 mirror) and its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Piece {
    item: usize,
    uv: [i32; 4],
    rect: [i32; 4],
    colour: u32,
}

/// One animation of a group: its pieces over frames [f0, f1] with a colour start → end and a
/// translation (x0, y0) → (x1, y1) (KH1 units).
#[derive(Clone, Default)]
pub(crate) struct Step {
    pieces: Vec<Piece>,
    frames: (i32, i32),
    colour: (u32, u32),
    flags: u32,
    tr: [i32; 4],
}

pub(crate) struct Group {
    steps: Vec<Step>,
    once: bool,
    word: i16,
}

/// Compiles a held group from the pieces each frame shows: one animation per piece and run of
/// frames (pieces sharing their runs share a sprite group).
fn held(frames: &[Vec<Piece>]) -> Group {
    use std::collections::BTreeMap;
    let mut runs: BTreeMap<Piece, Vec<(i32, i32)>> = BTreeMap::new();
    for (f, ps) in frames.iter().enumerate() {
        for p in ps {
            let r = runs.entry(*p).or_default();
            match r.last_mut() {
                Some(last) if last.1 + 1 == f as i32 => last.1 = f as i32,
                _ => r.push((f as i32, f as i32)),
            }
        }
    }
    // The last frame's pieces stay for any larger frame.
    let lastf = frames.len() as i32 - 1;
    let mut by_runs: BTreeMap<Vec<(i32, i32)>, Vec<Piece>> = BTreeMap::new();
    for (p, mut r) in runs {
        if let Some(l) = r.last_mut() {
            if l.1 == lastf {
                l.1 = LONG;
            }
        }
        by_runs.entry(r).or_default().push(p);
    }
    let one = 0x8080_8080;
    let mut steps = Vec::new();
    for (r, ps) in by_runs {
        for fr in r {
            steps.push(Step { pieces: ps.clone(), frames: fr, colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] });
        }
    }
    Group { steps, once: true, word: GROUP_WORD_SCRUB }
}

/// The SEQD of a set of groups, with the UVs of the PS2 atlas or the PC one.
fn seqd(groups: &[Group], packed: &Packed, hd: bool) -> Result<Vec<u8>> {
    let mut s = Seqd { version: 0x160, sprites: Vec::new(), parts: Vec::new(), sgroups: Vec::new(), anims: Vec::new(), agroups: Vec::new() };
    use std::collections::HashMap;
    let mut spr: HashMap<([i32; 4], u32), usize> = HashMap::new();
    let mut sg_of: HashMap<Vec<Piece>, usize> = HashMap::new();
    for g in groups {
        let a0 = s.anims.len();
        for st in &g.steps {
            let key = st.pieces.clone();
            let sg = match sg_of.get(&key) {
                Some(&i) => i,
                None => {
                    let p0 = s.parts.len();
                    for p in &st.pieces {
                        let (b, h) = packed.uv(Sub { item: p.item, rect: p.uv });
                        let uv = if hd { h } else { b };
                        let si = *spr.entry((uv, p.colour)).or_insert_with(|| {
                            s.sprites.push(sprite(uv, [p.colour; 4]));
                            s.sprites.len() - 1
                        });
                        s.parts.push(part(p.rect, si));
                    }
                    s.sgroups.push(sgroup(p0, st.pieces.len()));
                    sg_of.insert(key, s.sgroups.len() - 1);
                    s.sgroups.len() - 1
                }
            };
            let mut a = anim(st.flags, sg, st.frames, 0, (1.0, 1.0), st.colour);
            // Translation start / end (`+0x10` x0, `+0x14` x1, `+0x18` y0, `+0x1C` y1, ints in 2D
            // units: `0x14024B4E0` reads them as (x0, y0) → `+0xD0`, (x1, y1) → `+0xD8`; Checked
            // (decomp)).
            let [x0, y0, x1, y1] = st.tr;
            for (at, v) in [(0x10, x0), (0x14, x1), (0x18, y0), (0x1C, y1)] {
                a[at..at + 4].copy_from_slice(&v.to_le_bytes());
            }
            s.anims.push(a);
        }
        s.agroups.push(agroup(a0, g.steps.len(), g.once, g.word));
    }
    // Re:CoM reads an animation group's first animation and count, and a sprite group's first
    // part, as unsigned 16-bit (`0x140248F40`: `*param_4`, `param_4[1]`; `0x140249150`'s part
    // pointer from `*(ushort *)(sgroup)`); Checked (decomp). Parts stay below 0x8000 (our sprite
    // group writer stores the first part as i16).
    if s.anims.len() > u16::MAX as usize || s.parts.len() > i16::MAX as usize || s.sgroups.len() > i16::MAX as usize {
        return bad(format!("kh1hud: too many records for 16-bit indices ({} animations, {} parts, {} sprite groups)", s.anims.len(), s.parts.len(), s.sgroups.len()));
    }
    Ok(s.write())
}

/// A LAYD holding one SEQD (layout tables: one empty entry each, as Re:CoM's registration
/// `0x140244AD0` only stores them).
pub fn layd(seq: &[u8]) -> Vec<u8> {
    let mut d = b"LAYD".to_vec();
    let l1 = 0x20u32;
    let l2 = l1 + 0x18;
    let st = l2 + 0x14;
    let so = (st + 4 + 15) & !15;
    for v in [0x100u32, 1, l1, 1, l2, 1, st] {
        d.extend(v.to_le_bytes());
    }
    d.extend([0u8; 0x18]);
    let mut l2e = vec![0u8; 0x14];
    l2e[2..4].copy_from_slice(&1i16.to_le_bytes());
    d.extend(l2e);
    d.extend(so.to_le_bytes());
    d.resize(so as usize, 0);
    d.extend_from_slice(seq);
    d
}

/// An IMGZ of one 32-bit IMGD (PSMCT32, linear, PS2 alpha): the header Re:CoM's own PC atlas
/// `UK_Cockpit.imz` has (Checked (files): every word but the sizes).
pub fn imgz(p: &Pic) -> Vec<u8> {
    let (w, h) = (p.w, p.h);
    let len = (w * h * 4) as u32;
    let mut g = b"IMGD".to_vec();
    for v in [0x100u32, 0x40, len, 0x40 + len, 0, 0xFFFF_FFFF] {
        g.extend(v.to_le_bytes());
    }
    for v in [w as u16, h as u16, w.trailing_zeros() as u16, h.trailing_zeros() as u16, (w / 64).max(1) as u16, 0] {
        g.extend(v.to_le_bytes());
    }
    g.extend(0xFFFF_FFFFu32.to_le_bytes());
    for v in [0x10u16, 0x10, 1, 0x13, 3, 0] {
        g.extend(v.to_le_bytes());
    }
    g.extend(0u32.to_le_bytes());
    g.extend(3u32.to_le_bytes());
    debug_assert_eq!(g.len(), 0x40);
    for px in &p.px {
        g.extend_from_slice(px);
    }
    let mut z = b"IMGZ".to_vec();
    for v in [0x100u32, 0x10, 1, 0x20, g.len() as u32, 0, 0] {
        z.extend(v.to_le_bytes());
    }
    z.extend(g);
    z
}

// ---------------------------------------------------------------------------------------------
// The build.

/// Bakes and caches arc pieces into the atlas.
struct Baker {
    centre: (i32, i32),
    src5: Vec<Source>,
    src7: Vec<Source>,
    p7: usize,
    /// A 1 × 1 transparent item, for a step that shows nothing.
    empty: usize,
    cache: std::collections::BTreeMap<(i32, i32, i32, i32, i32, bool), Option<(usize, (i32, i32, i32, i32))>>,
}

impl Baker {
    /// The piece of the fan [a0, a1] of ring `r` (picture 5, or picture 7 per language).
    fn piece(&mut self, atlas: &mut Atlas, r: (i32, i32, i32), p: usize, a0: i32, a1: i32) -> Piece {
        self.piece_soft(atlas, r, p, a0, a1, false)
    }

    /// [`Baker::piece`] with KH1's soft end (mode 4) when `soft` and 1 < a1 < 270.
    fn piece_soft(&mut self, atlas: &mut Atlas, r: (i32, i32, i32), p: usize, a0: i32, a1: i32, soft: bool) -> Piece {
        let soft = soft && a1 > 1 && a1 < 270;
        let ring = Ring { x: self.centre.0, y: self.centre.1, uc: r.0, vc: r.1, r: r.2 };
        let got = *self.cache.entry((r.0, r.1, r.2, a0, a1, soft)).or_insert_with(|| {
            let s: Vec<&Source> = if p == self.p7 { self.src7.iter().collect() } else { self.src5.iter().take(1).collect() };
            bake_soft(&ring, a0 as f32, a1 as f32, soft, &s).map(|b| {
                let hd = if p == self.p7 { b.hd } else { vec![b.hd[0].clone()] };
                (atlas.add(b.ps2, hd, s[0].scale), b.rect)
            })
        });
        match got {
            Some((it, (x, y, w, h))) => Piece { item: it, uv: [0, 0, w, h], rect: [x, y, x + w, y + h], colour: 0x8080_8080 },
            None => Piece { item: self.empty, uv: [0, 0, 1, 1], rect: [0, 0, 1, 1], colour: 0 },
        }
    }

    /// The arc [0, a] as pieces cut every `step` degrees (KH1 draws one fan; the union of the
    /// cut fans is the same polygon, its boundary points lying on the same square edges).
    fn arc(&mut self, atlas: &mut Atlas, r: (i32, i32, i32), p: usize, a: i32, step: i32) -> Vec<Piece> {
        self.arc_soft(atlas, r, p, a, step, false)
    }

    /// [`Baker::arc`] whose last piece has KH1's soft end (mode 4) when `soft`.
    fn arc_soft(&mut self, atlas: &mut Atlas, r: (i32, i32, i32), p: usize, a: i32, step: i32, soft: bool) -> Vec<Piece> {
        let mut v = Vec::new();
        let mut q0 = 0;
        while q0 < a {
            let q1 = (q0 + step - q0 % step).min(a);
            let pc = self.piece_soft(atlas, r, p, q0, q1, soft && q1 == a);
            if pc.item != self.empty {
                v.push(pc);
            }
            q0 = q1;
        }
        v
    }
}

/// What the step reads.
pub struct Sources<'a> {
    /// `command2/uitex.bin` and its PC assets.
    pub uitex: &'a [u8],
    /// Its PC assets.
    pub uitex_hd: &'a [HdAsset],
    /// `exchange/UK_uibin_g_s.bin`, `…_g_red.bin`.
    pub g_s: &'a [u8],
    /// `g_red`.
    pub g_red: &'a [u8],
    /// `g_act` (the gauge motions).
    pub g_act: &'a [u8],
    /// The three faces (Sora, Donald, Goofy).
    pub faces: [Source; 3],
}

/// The built files.
pub struct Built {
    /// `kh1hud.lad` (PS2 atlas UVs) and the PC copy (PC atlas UVs, for every language).
    pub lad: Vec<u8>,
    /// The PC copy.
    pub lad_hd: Vec<u8>,
    /// `kh1hud.imz` and `<LANG>_kh1hud.imz` (in `langs` order).
    pub imz: Vec<u8>,
    /// The PC atlases.
    pub imz_hd: Vec<Vec<u8>>,
    /// Sizes for the log.
    pub atlas: (usize, usize, usize, usize),
    /// Atlas items.
    pub pieces: usize,
    /// Sprites.
    pub sprites: usize,
    /// The motion programs and units (H2b), for the manifest.
    pub motion: motion::Motion,
    /// Groups in all.
    pub groups: usize,
}

fn colour_of(o: &Obj) -> u32 {
    let c = o.colours[0];
    let b = |v: i32| v.clamp(0, 255) as u32;
    b(c[0]) | b(c[1]) << 8 | b(c[2]) << 16 | b(c[3]) << 24
}

/// The piece an object draws at (dx, dy), with KH1's partial cut (`x += cut`, `w −= cut`, UV kept:
/// `0x1402698C0`, `0x14026BEE0`, `0x14026A970`) and an x override (the caps).
fn obj_piece(o: &Obj, item: usize, dx: i32, dy: i32, cut: i32, x: Option<i32>) -> Piece {
    let (uv, mu, mv) = o.texels();
    let [ox, oy, w, h] = o.rect;
    let x0 = x.unwrap_or(ox) + cut + dx;
    let (x1, y0, y1) = (x0 + w - cut, oy + dy, oy + dy + h);
    let rect = [if mu { x1 } else { x0 }, if mv { y1 } else { y0 }, if mu { x0 } else { x1 }, if mv { y0 } else { y1 }];
    Piece { item, uv, rect, colour: colour_of(o) }
}

/// KH1's cut of a partial straight segment: round((1 − rem / per) · width), at least 1, at most
/// width − 1 (`0x1402698C0`: f32 + 0.5, truncated).
pub fn cut(rem: i32, per: i32, width: i32) -> i32 {
    let c = ((1.0f32 - rem as f32 / per as f32) * width as f32 + 0.5) as i32;
    c.max(1).min(width - 1)
}

/// The straight pieces of HP-like rows (`0x1402698C0` back, `0x14026BEE0` colour): `n` full
/// segments of `obj` (the first of `first` for the back) at (−1 − 26 i, +1), then the partial one.
/// Returns (piece list, the x offset after them and the last cut) for the cap.
fn row(objs: &[Obj], item: usize, first: usize, rest: usize, units: i32, per: i32) -> (Vec<Piece>, i32, i32) {
    let n = units / per;
    let rem = units % per;
    let mut v = Vec::new();
    let mut dx = -1;
    for i in 0..n {
        let (o, x) = if i == 0 { (first, None) } else { (rest, Some(objs[rest].rect[0] + kh1::PITCH)) };
        let x = if first == rest { None } else { x };
        v.push(obj_piece(&objs[o], item, dx, 1, 0, x));
        dx -= kh1::PITCH;
    }
    let mut c = kh1::PITCH;
    if rem != 0 {
        c = cut(rem, per, kh1::PITCH);
        let (o, x) = if n == 0 || first == rest { (first, None) } else { (rest, Some(objs[rest].rect[0] + kh1::PITCH)) };
        v.push(obj_piece(&objs[o], item, dx, 1, c, x));
    }
    (v, dx, c)
}

/// Builds the set.
pub fn build(src: &Sources, langs: &[&str]) -> Result<Built> {
    if langs.is_empty() {
        return bad("kh1hud: no languages");
    }
    let g_s = Uibin::parse(src.g_s)?;
    if g_s.objs.len() != obj::COUNT || g_s.objs[obj::SEG_HP].name != "g_s_bar_e_hp" || g_s.objs[obj::FACE_FRAME[0]].name != "g_s_fra_a_wa" {
        return bad(format!("{G_S}: {} objects, not KH1 FM Steam's 39 (g_s_fra_a_wa … g_s_par_d_mp)", g_s.objs.len()));
    }
    let g_red = Uibin::parse(src.g_red)?;
    let red = g_red.find("g_s_bar_b_hp_red").ok_or_else(|| crate::Error::Format(format!("{G_RED}: no g_s_bar_b_hp_red")))?;
    if g_red.keys_a.len() < 3 || red.keys_a != (0, 3) {
        return bad(format!("{G_RED}: the flash keys are not KH1 FM Steam's (3 colour keys)"));
    }
    let objs = &g_s.objs;
    // Pictures 5, 6, 7 (PS2 and PC per language).
    let tims = mb::tim2_pictures(src.uitex)?;
    let mut pics: Vec<Source> = Vec::new();
    let mut pic_langs: Vec<Vec<Pic>> = Vec::new();
    for (i, off, data) in PICTURES {
        let t = tims.get(i).ok_or_else(|| crate::Error::Format(format!("uitex.bin: no picture {i}")))?;
        if t.offset != off || t.width != 128 || t.height != 128 {
            return bad(format!("uitex.bin: picture {i} at {:#x} {}×{}, KH1 draws it from {off:#x} 128×128", t.offset, t.width, t.height));
        }
        let ps2 = Pic { w: 128, h: 128, px: t.pixels.clone() };
        let mut per = Vec::new();
        for l in langs {
            let hd = uitex_asset(src.uitex_hd, data, l)?.ok_or_else(|| crate::Error::Format(format!("uitex.bin: no PC texture for picture {i} (-{data:x}.dds)")))?;
            per.push(hd);
        }
        let scale = hd_scale(&ps2, &per[0])?;
        if per.iter().any(|p| p.w != per[0].w || p.h != per[0].h) {
            return bad(format!("uitex.bin: picture {i}'s language textures differ in size"));
        }
        // Only the PS2 picture's area of the PC texture (picture 2's carries extra rows on PC).
        let per: Vec<Pic> = per.into_iter().map(|p| p.crop(0, 0, 128 * scale, 128 * scale)).collect();
        pics.push(Source { ps2: ps2.clone(), hd: per[0].clone(), scale });
        pic_langs.push(per);
    }
    let (p5, p6, p7) = (0usize, 1usize, 2usize);
    let mut atlas = Atlas { items: Vec::new() };
    let it5 = atlas.add(pics[p5].ps2.clone(), pic_langs[p5].clone(), pics[p5].scale);
    let it6 = atlas.add(pics[p6].ps2.clone(), pic_langs[p6].clone(), pics[p6].scale);
    let it7 = atlas.add(pics[p7].ps2.clone(), pic_langs[p7].clone(), pics[p7].scale);
    let faces: Vec<usize> = src.faces.iter().map(|f| atlas.add(f.ps2.clone(), vec![f.hd.clone()], f.scale)).collect();
    let (cx, cy) = kh1::CENTRE;
    let per_lang_src = |p: usize| -> Vec<Source> { pic_langs[p].iter().map(|hd| Source { ps2: pics[p].ps2.clone(), hd: hd.clone(), scale: pics[p].scale }).collect() };
    let empty = atlas.add(Pic::blank(1, 1), vec![Pic::blank(2, 2)], 2);
    let mut bk = Baker { centre: (cx * 16, cy * 16), src5: per_lang_src(p5), src7: per_lang_src(p7), p7, empty, cache: Default::default() };
    let mut groups: Vec<Group> = Vec::new();
    // FRAME: picture 6's face frame, picture 7's disc and labels.
    let frame_pieces = |mp: bool| -> Vec<Piece> {
        let mut v: Vec<Piece> = obj::FACE_FRAME.iter().map(|&i| obj_piece(&objs[i], it6, 0, 0, 0, None)).collect();
        v.extend(obj::DISC.iter().map(|&i| obj_piece(&objs[i], it7, 0, 0, 0, None)));
        v.push(obj_piece(&objs[obj::LABEL_HP], it7, 0, 0, 0, None));
        if mp {
            v.push(obj_piece(&objs[obj::LABEL_MP], it7, 0, 0, 0, None));
        }
        v
    };
    groups.push(held(&[frame_pieces(false), frame_pieces(true)]));
    // HP_BACK: the back arc to min(max, 120) · 5 (≤ 270), then from 54 the straight back and its cap.
    let mut f = Vec::new();
    for m in 0..=kh1::HP_CAP {
        let mut v = if m > 0 { bk.arc(&mut atlas, kh1::RING_HP_BACK, p7, (m * kh1::HP_DEG).min(270), 90) } else { Vec::new() };
        if m >= kh1::HP_RING {
            let (segs, dx, c) = row(objs, it7, obj::BACK_HP_FIRST, obj::BACK_HP, m - kh1::HP_RING, kh1::HP_PER_SEG);
            v.extend(segs);
            v.push(obj_piece(&objs[obj::CAP_HP], it7, dx + c, 1, 0, Some(kh1::HP_CAP_X)));
        }
        f.push(v);
    }
    groups.push(held(&f));
    // MP_BACK: to min(max MP, 20) · 30 (≤ 270), then from 9 a back segment a point and the cap.
    let mut f = Vec::new();
    for m in 0..=kh1::MP_CAP {
        let mut v = if m > 0 { bk.arc(&mut atlas, kh1::RING_MP_BACK, p7, (m * kh1::MP_DEG).min(270), 90) } else { Vec::new() };
        if m >= kh1::MP_RING {
            let mut dx = -1;
            for _ in 0..m - kh1::MP_RING {
                v.push(obj_piece(&objs[obj::BACK_MP], it7, dx, 1, 0, None));
                dx -= kh1::PITCH;
            }
            v.push(obj_piece(&objs[obj::CAP_MP], it7, dx, 1, 0, Some(kh1::MP_CAP_X)));
        }
        f.push(v);
    }
    groups.push(held(&f));
    // MP: cells [30 j, 30 j + 30] for j < min(MP, 9), then a segment a point past 9.
    let cells: Vec<Piece> = (0..kh1::MP_RING).map(|j| bk.piece(&mut atlas, kh1::RING_MP, p5, 30 * j, 30 * j + 30)).collect();
    let mut f = Vec::new();
    for m in 0..=kh1::MP_CAP {
        let mut v: Vec<Piece> = cells.iter().take(m.min(kh1::MP_RING) as usize).filter(|p| p.item != bk.empty).copied().collect();
        let mut dx = -1;
        for _ in kh1::MP_RING..m {
            v.push(obj_piece(&objs[obj::SEG_MP], it5, dx, 1, 0, None));
            dx -= kh1::PITCH;
        }
        f.push(v);
    }
    groups.push(held(&f));
    // HP: the arc to min(HP, 54) · 5, then 6 HP a segment (partial last), at most 120.
    let mut f = Vec::new();
    for h in 0..=kh1::HP_CAP {
        let mut v = if h > 0 { bk.arc_soft(&mut atlas, kh1::RING_HP, p5, (h * kh1::HP_DEG).min(270), 90, true) } else { Vec::new() };
        if h > kh1::HP_RING {
            v.extend(row(objs, it5, obj::SEG_HP, obj::SEG_HP, h - kh1::HP_RING, kh1::HP_PER_SEG).0);
        }
        f.push(v);
    }
    groups.push(held(&f));
    // CHARGE: cells and the partial cell to min(c, 270), then 30 units a segment at (−26 i, +1)
    // with the partial one cut by round((1 − rem / 30) · 24).
    let seg_w = objs[obj::SEG_CHARGE].rect[2];
    let mut f = Vec::new();
    for c in 0..=kh1::CHARGE_CAP {
        let mut v = if c > 0 { bk.arc_soft(&mut atlas, kh1::RING_CHARGE, p5, c.min(270), 30, true) } else { Vec::new() };
        if c > kh1::CHARGE_RING {
            let (n, rem) = ((c - kh1::CHARGE_RING) / kh1::CHARGE_PER_SEG, (c - kh1::CHARGE_RING) % kh1::CHARGE_PER_SEG);
            let mut dx = 0;
            for _ in 0..n {
                v.push(obj_piece(&objs[obj::SEG_CHARGE], it5, dx, 1, 0, None));
                dx -= kh1::PITCH;
            }
            if rem != 0 {
                v.push(obj_piece(&objs[obj::SEG_CHARGE], it5, dx, 1, cut(rem, kh1::CHARGE_PER_SEG, seg_w), None));
            }
        }
        f.push(v);
    }
    groups.push(held(&f));
    // TICKS: picture 7's ticks past 90° / 180° of the max MP arc.
    let f: Vec<Vec<Piece>> = (0..=kh1::MP_CAP)
        .map(|m| {
            let mut v = Vec::new();
            if m * 30 > 90 {
                v.push(obj_piece(&objs[obj::TICK_90], it7, 0, 0, 0, None));
            }
            if m * 30 > 180 {
                v.push(obj_piece(&objs[obj::TICK_180], it7, 0, 0, 0, None));
            }
            v
        })
        .collect();
    groups.push(held(&f));
    // FACE: Sora's strip, cell = expression, h × h centred on the gauge centre (`0x14027F470`).
    let fh = src.faces[0].ps2.h as i32;
    let face_piece = |e: i32| Piece { item: faces[0], uv: [fh * e, 0, fh * (e + 1), fh], rect: [cx - fh / 2, cy - fh / 2, cx + fh / 2, cy + fh / 2], colour: 0x8080_8080 };
    groups.push(held(&[vec![face_piece(0)], vec![face_piece(1)], vec![face_piece(2)]]));
    let one = 0x8080_8080u32;
    // H2b: KH1's gauge motions (`g_act`) and the held groups' units.
    let g_act = Uibin::parse(src.g_act)?;
    let tracks = motion::tracks(&g_act)?;
    // HURT: expression 1 for 40 / 50 / 60 frames, then 0 or 2 (danger), with the hit shake of the
    // same strength baked in (`0x140268D60` starts both together; played from frame 1).
    for (si, n) in kh1::HURT_FRAMES.into_iter().enumerate() {
        for after in [0, 2] {
            let t = &tracks[si];
            let mut steps = motion::steps(t, &[face_piece(1)]);
            steps.pop(); // the hold: expression 1 until the timer ends, then `after`
            steps.push(Step { pieces: vec![face_piece(1)], frames: (t.len, n), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] });
            steps.push(Step { pieces: vec![face_piece(after)], frames: (n + 1, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] });
            groups.push(Group { steps, once: true, word: GROUP_WORD_FADE });
        }
    }
    // Fades (KH1: 12.8 a frame for 10 frames, `0x14027F760`; drawn 115.2 … 12.8).
    let last = mb::FADE_FRAMES - 2;
    let alpha = |a: u8| (a as u32) << 24 | 0x80_8080;
    let grey = |c: u8| 0x8000_0000 | (c as u32) * 0x01_0101;
    let fade = |p: Piece, from: u32, to: u32, then: u32, held: bool| Group {
        steps: vec![
            Step { pieces: vec![p], frames: (0, last), colour: (from, to), flags: FLAGS_COLOUR, tr: [0; 4] },
            Step { pieces: vec![p], frames: (last + 1, if held { LONG } else { mb::FADE_FRAMES }), colour: (then, then), flags: FLAGS_COLOUR, tr: [0; 4] },
        ],
        once: true,
        word: GROUP_WORD_FADE,
    };
    // MP gain: black over the cell, alpha 115 → 13 (KH1 draws the gained cell RGB × 12.8 … 115.2).
    for c in &cells {
        groups.push(fade(Piece { colour: 0x8000_0000, ..*c }, alpha(mb::FADE_FIRST), alpha(mb::FADE_LAST), 0x0080_8080, false));
    }
    // MP loss: the cell's RGB 115 → 13, then gone.
    for c in &cells {
        groups.push(fade(*c, grey(mb::FADE_FIRST), grey(mb::FADE_LAST), 0, false));
    }
    // Charge: [30 j, 30 j + t] fading in (held) or out.
    let mut charge_pieces = Vec::new();
    for j in 0..9 {
        for t in 1..=30 {
            charge_pieces.push(bk.piece_soft(&mut atlas, kh1::RING_CHARGE, p5, 30 * j, 30 * j + t, t < 30));
        }
    }
    for p in &charge_pieces {
        groups.push(fade(*p, alpha(mb::FADE_LAST), alpha(mb::FADE_FIRST), one, true));
    }
    for p in &charge_pieces {
        groups.push(fade(*p, alpha(mb::FADE_FIRST), alpha(mb::FADE_LAST), 0x0080_8080, false));
    }
    // Danger flash (`g_red`: colour (128, 0, 0, a), a 0 → 128 over 8 key units, back to 0 over 8;
    // players step 0.5 a frame and draw after the step: a = 8, 16 … 128, 120 … 0, looped).
    let rk = &g_red.keys_a;
    let (rr, rg, rb) = (rk[0].vals[0] as u32, rk[0].vals[1] as u32, rk[0].vals[2] as u32);
    let up = rk[0].dur * 2;
    let down = rk[1].dur * 2;
    let peak = rk[1].vals[3] as u32;
    let rc = |a: u32| a << 24 | rb << 16 | rg << 8 | rr;
    let step_a = peak / up as u32;
    let flash = |p: Piece| Group {
        steps: vec![
            Step { pieces: vec![p], frames: (0, up - 1), colour: (rc(step_a), rc(peak)), flags: FLAGS_COLOUR, tr: [0; 4] },
            Step { pieces: vec![p], frames: (up, up + down - 1), colour: (rc(peak - step_a), rc(0)), flags: FLAGS_COLOUR, tr: [0; 4] },
        ],
        once: false,
        word: GROUP_WORD_FADE,
    };
    for q in 0..3 {
        for k in 1..=18 {
            let p = bk.piece_soft(&mut atlas, kh1::RING_HP, p5, 90 * q, 90 * q + 5 * k, k < 18);
            groups.push(flash(p));
        }
    }
    groups.push(flash(obj_piece(&objs[obj::SEG_HP], it5, -1, 1, 0, None)));
    for rem in 1..kh1::HP_PER_SEG {
        groups.push(flash(obj_piece(&objs[obj::SEG_HP], it5, -1, 1, cut(rem, kh1::HP_PER_SEG, kh1::PITCH), None)));
    }
    if groups.len() != group::COUNT {
        return bad(format!("kh1hud: {} groups, expected {}", groups.len(), group::COUNT));
    }
    // H2b: every unit of the held cluster groups gets its motion programs (played groups).
    let motion = motion::units(&groups[..=group::FACE], &tracks, group::COUNT);
    for (u, pieces) in motion.unit_pieces.iter().enumerate() {
        for p in motion.programs_of(u) {
            groups.push(motion::program_group(&motion.programs[p], &tracks, pieces));
        }
    }
    let groups_total = groups.len();
    let packed = build_atlas(&atlas, langs.len())?;
    let base = seqd(&groups, &packed, false)?;
    let hd = seqd(&groups, &packed, true)?;
    let sprites = Seqd::parse(&base)?.sprites.len();
    Ok(Built {
        lad: layd(&base),
        lad_hd: layd(&hd),
        imz: imgz(&packed.ps2),
        imz_hd: packed.hd.iter().map(imgz).collect(),
        atlas: (packed.ps2.w, packed.ps2.h, packed.hd[0].w, packed.hd[0].h),
        pieces: atlas.items.len(),
        sprites,
        motion,
        groups: groups_total,
    })
}

/// Animation flags of a moving piece: Re:CoM's static flags (`FLAGS_STATIC`) with the translation
/// interpolated (bits 0x10000 and 0x1000 clear: `0x14024B4E0` folds 0x1000 into 0x10000, and
/// `0x14024A9xx` interpolates `+0xB0` from `+0xD0` to `+0xD8` only when that bit is clear) and an
/// exclusive end (0x8000: no extra frame after `f1`, so consecutive segments never draw together).
/// Checked (decomp); Re:CoM's own `Cockpit:00` uses 0x8000 (e.g. 0x4FB71) and moving pieces with
/// both translation bits clear (e.g. 0x6B73).
pub(crate) const FLAGS_MOVE: u32 = 0x6EFF1;

/// H2b: KH1's gauge motions baked into played groups (decision 39: "hit shake / size-class slide
/// baked into every piece's animation data"; Re:CoM plays them, nothing of ours per frame).
///
/// KH1 moves the whole cluster: `0x14026D990` adds the running motion's keyed offset (slot `+0x110`:
/// a hit's shake `0x140268D60`, or a size-class change's slide `0x14026D570`) to the centre every
/// piece is drawn from. Re:CoM animates per layer, and a scrubbed (held) layer never advances in time
/// (`0x140248BB0`: a set frame `+0x38` ≥ 0 freezes it). So each distinct piece set of the held
/// groups (a *unit*) gets one played group per motion program (the motion, then its last offset
/// held to frame 10,000,000: the group's longest animation, which Re:CoM keeps drawn after the end,
/// `0x140248F40`'s bit 4), and the DLL swaps the held layers for the units of what they show while
/// a motion runs.
pub mod motion {
    use super::*;
    use crate::uibin::Key;

    /// `g_act` objects the programs come from (Sora's slot; the party's are `vibe_c…`).
    pub const OBJECTS: [&str; 5] = ["vibe_s", "vibe_ss", "vibe_sss", "slide_s_h", "slide_s_v"];

    /// A KH1 position track at Re:CoM's frames: segments (frames [f0, f1), offset from → to), its
    /// length and its last offset. KH1's players step 0.5 key units a 60 Hz frame (`0x14233FBE0`),
    /// so a key unit is 2 frames; frame f of the group is key time f / 2.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Track {
        /// Segments: (f0, f1), (x0, y0), (x1, y1).
        pub segs: Vec<((i32, i32), (i32, i32), (i32, i32))>,
        /// Frames.
        pub len: i32,
        /// The last key's offset.
        pub end: (i32, i32),
    }

    /// A track from KH1's keys (`0x14027E430`: linear between keys; a key with a duration ≤ 0 ends
    /// the track with its value).
    pub fn track(keys: &[Key]) -> Result<Track> {
        let v = |k: &Key| (k.vals[0].round() as i32, k.vals[1].round() as i32);
        let mut segs = Vec::new();
        let mut t = 0;
        for w in keys.windows(2) {
            if w[0].dur <= 0 {
                break;
            }
            let f1 = t + 2 * w[0].dur;
            segs.push(((t, f1), v(&w[0]), v(&w[1])));
            t = f1;
        }
        let last = keys.get(segs.len()).ok_or_else(|| crate::Error::Format("g_act: a track without keys".into()))?;
        if segs.is_empty() || last.dur > 0 {
            return bad("g_act: a track without an end key");
        }
        Ok(Track { segs, len: t, end: v(last) })
    }

    /// The five tracks of [`OBJECTS`].
    pub fn tracks(u: &Uibin) -> Result<Vec<Track>> {
        OBJECTS
            .iter()
            .map(|n| {
                let o = u.find(n).ok_or_else(|| crate::Error::Format(format!("{G_ACT}: no {n}")))?;
                let k = u.c_keys(o).ok_or_else(|| crate::Error::Format(format!("{G_ACT}: {n} has no position keys")))?;
                track(k)
            })
            .collect()
    }

    /// What a program does.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Kind {
        /// A track of [`OBJECTS`].
        Move(usize),
        /// KH1's gauge fade (`0x140268E30`: alpha ±0.1 a frame, drawn after the step): out or in.
        Fade(bool),
    }

    /// A motion program: its kind, the frame the DLL plays it from (`0x140248550`) and its length.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Program {
        /// Name for the manifest.
        pub name: &'static str,
        /// Kind.
        pub kind: Kind,
        /// Frame to play from: 1 for the tracks (KH1 draws key time 0.5 first), 0 for the fades
        /// (KH1 draws 0.9 / 0.1 first, the group's frame 0).
        pub start: i32,
        /// Frames until the motion is over.
        pub len: i32,
    }

    /// The units: distinct piece sets of the held groups, their runs and programs.
    #[derive(Clone, Debug, Default)]
    pub struct Motion {
        /// Programs, in the order of a unit's groups (the slides last, only for slide units).
        pub programs: Vec<Program>,
        /// Each unit's pieces.
        pub(crate) unit_pieces: Vec<Vec<Piece>>,
        /// Whether a unit has the slide programs.
        pub unit_slides: Vec<bool>,
        /// Each unit's first group.
        pub first: Vec<usize>,
        /// (held group, f0, f1, unit): the unit is shown while the held group's frame is in [f0, f1].
        pub runs: Vec<(usize, i32, i32, usize)>,
    }

    /// Programs every unit has.
    pub const BASE_PROGRAMS: usize = 5;

    impl Motion {
        /// The programs unit `u` has (indices into [`Motion::programs`]).
        pub fn programs_of(&self, u: usize) -> std::ops::Range<usize> {
            0..if self.unit_slides[u] { self.programs.len() } else { BASE_PROGRAMS }
        }
    }

    /// The programs: the three hit shakes, the fades out / in, the two slides.
    pub fn programs(tracks: &[Track]) -> Vec<Program> {
        let mv = |name, t: usize| Program { name, kind: Kind::Move(t), start: 1, len: tracks[t].len };
        vec![
            mv("vibe_s", 0),
            mv("vibe_ss", 1),
            mv("vibe_sss", 2),
            Program { name: "fade_out", kind: Kind::Fade(true), start: 0, len: mb::FADE_FRAMES },
            Program { name: "fade_in", kind: Kind::Fade(false), start: 0, len: mb::FADE_FRAMES },
            mv("slide_s_h", 3),
            mv("slide_s_v", 4),
        ]
    }

    /// The highest frame of a held group a slide can show: a slide plays only while the size class
    /// changes, i.e. while max HP < 37 (class 0 / 1, `0x1402690D0`) on one side of the change, so HP
    /// stays near the ring (≤ 72: one Re:CoM level-up of +15 past 36 and a margin) and the charge
    /// near 6 MP (≤ 210); other groups are small. Assumption (a bound, logged by the DLL when a
    /// slide finds a unit without it: it then moves at once).
    fn slide_limit(held: usize) -> i32 {
        match held {
            group::HP_BACK | group::HP => 72,
            group::CHARGE => 210,
            _ => i32::MAX,
        }
    }

    /// The units of the held groups `held` (indices = group numbers), numbered from group `base`.
    pub(crate) fn units(held: &[Group], tracks: &[Track], base: usize) -> Motion {
        let mut m = Motion { programs: programs(tracks), ..Default::default() };
        let mut of: std::collections::BTreeMap<Vec<Piece>, usize> = Default::default();
        for (gi, g) in held.iter().enumerate() {
            for st in &g.steps {
                let u = *of.entry(st.pieces.clone()).or_insert_with(|| {
                    m.unit_pieces.push(st.pieces.clone());
                    m.unit_slides.push(false);
                    m.unit_pieces.len() - 1
                });
                if st.frames.0 <= slide_limit(gi) {
                    m.unit_slides[u] = true;
                }
                m.runs.push((gi, st.frames.0, st.frames.1, u));
            }
        }
        let mut next = base;
        for u in 0..m.unit_pieces.len() {
            m.first.push(next);
            next += m.programs_of(u).len();
        }
        m
    }

    /// A track's steps on `pieces`: the segments (moving, exclusive ends), then the last offset held.
    pub(crate) fn steps(t: &Track, pieces: &[Piece]) -> Vec<Step> {
        let one = 0x8080_8080;
        let mut v: Vec<Step> = t
            .segs
            .iter()
            .map(|&(fr, a, b)| Step { pieces: pieces.to_vec(), frames: fr, colour: (one, one), flags: FLAGS_MOVE, tr: [a.0, a.1, b.0, b.1] })
            .collect();
        v.push(Step { pieces: pieces.to_vec(), frames: (t.len, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [t.end.0, t.end.1, t.end.0, t.end.1] });
        v
    }

    /// The played group of program `p` on a unit's pieces.
    pub(crate) fn program_group(p: &Program, tracks: &[Track], pieces: &[Piece]) -> Group {
        let a = |x: u32| x << 24 | 0x80_8080;
        let steps = match p.kind {
            Kind::Move(t) => steps(&tracks[t], pieces),
            Kind::Fade(out) => {
                // 0.9 … 0.0 (out) or 0.1 … 1.0 (in) over frames 0..9, then held.
                let (from, to) = if out { (115, 0) } else { (13, 128) };
                vec![
                    Step { pieces: pieces.to_vec(), frames: (0, p.len - 1), colour: (a(from), a(to)), flags: FLAGS_COLOUR, tr: [0; 4] },
                    Step { pieces: pieces.to_vec(), frames: (p.len, LONG), colour: (a(to), a(to)), flags: FLAGS_COLOUR, tr: [0; 4] },
                ]
            }
        };
        Group { steps, once: true, word: GROUP_WORD_FADE }
    }
}

// ---------------------------------------------------------------------------------------------
// Adding the set to Re:CoM's files.

/// The languages Re:CoM's `FO0002.SPR` has PC layouts for (`<LANG>_Cockpit.lad`).
pub fn languages(spr_assets: &[HdAsset]) -> Vec<String> {
    let mut v: Vec<String> = spr_assets.iter().filter_map(|a| a.name.strip_suffix("_Cockpit.lad").map(str::to_string)).filter(|l| l.len() == 2).collect();
    v.sort();
    v.dedup();
    v
}

fn entry(name: &str, kind: [u8; 4], data: Vec<u8>) -> Result<Entry> {
    if name.len() >= 0x14 {
        return bad(format!("{name}: name too long for the container"));
    }
    let mut raw = [0u8; 0x18];
    raw[..name.len()].copy_from_slice(name.as_bytes());
    raw[0x14..].copy_from_slice(&kind);
    Ok(Entry { name_raw: raw, flag: true, data })
}

/// Adds `kh1hud.lad` to an `FO0002.SPR` container and `<LANG>_kh1hud.lad` to its PC assets
/// (refusing a file that already has it).
pub fn add_to_spr(spr: &[u8], assets: &mut Vec<HdAsset>, b: &Built, langs: &[String]) -> Result<Vec<u8>> {
    let mut c = Container::parse(spr)?;
    if c.find(LAD).is_some() || assets.iter().any(|a| a.name.ends_with(LAD)) {
        return bad(format!("FO0002.SPR already has {LAD}"));
    }
    let kind: [u8; 4] = c.entries[0].name_raw[0x14..].try_into().unwrap();
    c.entries.push(entry(LAD, kind, b.lad.clone())?);
    for l in langs {
        assets.push(HdAsset { name: format!("{l}_{LAD}"), orig_offset: 0xFFFF, data: b.lad_hd.clone() });
    }
    Ok(c.write())
}

/// Adds `kh1hud.imz` to an `FO0002.RTM` container and `<LANG>_kh1hud.imz` to its PC assets.
pub fn add_to_rtm(rtm: &[u8], assets: &mut Vec<HdAsset>, b: &Built, langs: &[String]) -> Result<Vec<u8>> {
    let mut c = Container::parse(rtm)?;
    if c.find(IMZ).is_some() || assets.iter().any(|a| a.name.ends_with(IMZ)) {
        return bad(format!("FO0002.RTM already has {IMZ}"));
    }
    let atlas = c.find(mb::ATLAS).ok_or_else(|| crate::Error::Format(format!("FO0002.RTM: no {}", mb::ATLAS)))?;
    let kind: [u8; 4] = atlas.name_raw[0x14..].try_into().unwrap();
    c.entries.push(entry(IMZ, kind, b.imz.clone())?);
    for (l, d) in langs.iter().zip(&b.imz_hd) {
        assets.push(HdAsset { name: format!("{l}_{IMZ}"), orig_offset: 0xFFFF, data: d.clone() });
    }
    Ok(c.write())
}

/// `kh1-hud.json`: the set, the group count, the languages and the inputs' sha256.
pub fn manifest_json(b: &Built, langs: &[String], sources: &[(&str, String)]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"format\": \"{FORMAT}\",");
    let _ = writeln!(s, "  \"set\": \"{SET}\",");
    let _ = writeln!(s, "  \"groups\": {},", b.groups);
    let _ = writeln!(s, "  \"base_groups\": {},", group::COUNT);
    let m = &b.motion;
    let _ = writeln!(
        s,
        "  \"programs\": [{}],",
        m.programs.iter().map(|p| format!("{{\"name\": \"{}\", \"start\": {}, \"frames\": {}}}", p.name, p.start, p.len)).collect::<Vec<_>>().join(", ")
    );
    let _ = writeln!(s, "  \"units\": [{}],", (0..m.first.len()).map(|u| format!("[{}, {}]", m.first[u], m.programs_of(u).len())).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s, "  \"runs\": [{}],", m.runs.iter().map(|r| format!("[{}, {}, {}, {}]", r.0, r.1, r.2, r.3)).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s, "  \"languages\": [{}],", langs.iter().map(|l| format!("\"{l}\"")).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s, "  \"atlas\": [{}, {}, {}, {}],", b.atlas.0, b.atlas.1, b.atlas.2, b.atlas.3);
    let _ = writeln!(s, "  \"sources\": {{");
    for (i, (k, v)) in sources.iter().enumerate() {
        let _ = writeln!(s, "    \"{k}\": \"{v}\"{}", if i + 1 < sources.len() { "," } else { "" });
    }
    let _ = writeln!(s, "  }}");
    let _ = write!(s, "}}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp_bar::Layd;

    fn ring() -> Ring {
        Ring { x: 443 * 16, y: 316 * 16, uc: kh1::RING_HP.0, vc: kh1::RING_HP.1, r: kh1::RING_HP.2 }
    }

    #[test]
    fn fan_vertices_follow_kh1() {
        let r = ring();
        // 0°: west edge at the centre's height; 90°: top at the centre column; 180°: east edge one
        // unit out; 270°: bottom one unit down.
        assert_eq!(vertex(&r, 0.0), [r.x - r.r, r.y, r.uc - r.r, r.vc]);
        assert_eq!(vertex(&r, 90.0), [r.x, r.y - r.r, r.uc, r.vc - r.r]);
        assert_eq!(vertex(&r, 180.0), [r.x + 16 + r.r, r.y, r.uc - r.r, r.vc]);
        assert_eq!(vertex(&r, 270.0), [r.x, r.y + 16 + r.r, r.uc, r.vc - r.r]);
        assert_eq!(vertex(&r, 45.0), [r.x - r.r, r.y - r.r, r.uc - r.r, r.vc - r.r]);
        // 45°-steps between the ends, then the end.
        let f = fan(&r, 0.0, 100.0);
        assert_eq!(f.len(), 1 + 1 + 2 + 1);
        assert_eq!(f[0], [r.x, r.y + 16, r.uc, r.vc]);
    }

    fn solid(w: usize, h: usize) -> Pic {
        Pic { w, h, px: (0..w * h).map(|i| [(i % w) as u8, (i / w) as u8, 0, 0x80]).collect() }
    }

    #[test]
    fn baked_quadrants_tile_without_overlap() {
        let src = Source { ps2: solid(128, 128), hd: solid(256, 256), scale: 2 };
        let r = ring();
        let full = bake(&r, 0.0, 90.0, &[&src]).unwrap();
        let a = bake(&r, 0.0, 45.0, &[&src]).unwrap();
        let b = bake(&r, 45.0, 90.0, &[&src]).unwrap();
        // Same texels where the halves draw; no unit drawn by both.
        let at = |p: &Baked, x: i32, y: i32| -> Option<[u8; 4]> {
            let (px, py) = (x - p.rect.0, y - p.rect.1);
            (px >= 0 && py >= 0 && px < p.rect.2 && py < p.rect.3).then(|| p.ps2.px[(py * p.rect.2 + px) as usize]).filter(|c| c[3] != 0)
        };
        let mut n = 0;
        for y in full.rect.1..full.rect.1 + full.rect.3 {
            for x in full.rect.0..full.rect.0 + full.rect.2 {
                let (f, ha, hb) = (at(&full, x, y), at(&a, x, y), at(&b, x, y));
                assert!(!(ha.is_some() && hb.is_some()), "overlap at {x},{y}");
                if let Some(c) = ha.or(hb) {
                    assert_eq!(Some(c), f);
                    n += 1;
                }
            }
        }
        assert!(n > 1000);
        // The quadrant spans the KH1 square: x from 443 − 41 to 443, y from 316 − 41.
        assert_eq!((full.rect.0, full.rect.1), (402, 275));
        // Texel mapping: unit (403, 300) shows texel (403 − 400.5 → 2, 300 − 216.5 → 83).
        assert_eq!(at(&full, 403, 300).unwrap()[..2], [2, 83]);
    }

    #[test]
    fn tracks_follow_kh1_keys() {
        use crate::uibin::Key;
        let k = |dur: i32, x: f32, y: f32| Key { dur, vals: vec![x, y, 128.0, 128.0] };
        // vibe_s: 0, 3, -3, 2, -2, end 0 (1 unit each).
        let t = motion::track(&[k(1, 0.0, 0.0), k(1, 0.0, 3.0), k(1, 0.0, -3.0), k(1, 0.0, 2.0), k(1, 0.0, -2.0), k(-5, 0.0, 0.0)]).unwrap();
        assert_eq!(t.len, 10);
        assert_eq!(t.segs[0], ((0, 2), (0, 0), (0, 3)));
        assert_eq!(t.segs[4], ((8, 10), (0, -2), (0, 0)));
        assert_eq!(t.end, (0, 0));
        // A moving step's frames are exclusive and interpolate (x0, y0) → (x1, y1); the hold keeps
        // the end to LONG.
        let p = Piece { item: 0, uv: [0, 0, 1, 1], rect: [0, 0, 1, 1], colour: 0 };
        let st = motion::steps(&t, &[p]);
        assert_eq!(st.len(), 6);
        assert_eq!((st[1].flags, st[1].frames, st[1].tr), (FLAGS_MOVE, (2, 4), [0, 3, 0, -3]));
        assert_eq!((st[5].flags, st[5].frames), (FLAGS_STATIC, (10, LONG)));
        assert!(motion::track(&[k(1, 0.0, 0.0)]).is_err());
    }

    #[test]
    fn soft_end_fades_the_last_degrees() {
        let src = Source { ps2: solid(128, 128), hd: solid(256, 256), scale: 2 };
        let r = ring();
        let (f, w) = fan_soft(&r, 0.0, 100.0, true);
        assert_eq!(f.last().copied(), Some(vertex(&r, 100.0 - SOFT_DEG)));
        assert_eq!(w.unwrap()[2], vertex(&r, 100.0));
        assert!(fan_soft(&r, 0.0, 270.0, true).1.is_none());
        let soft = bake_soft(&r, 0.0, 100.0, true, &[&src]).unwrap();
        let hard = bake(&r, 0.0, 100.0, &[&src]).unwrap();
        // Same footprint; some texels near the end edge are part-transparent.
        assert_eq!(soft.rect, hard.rect);
        assert!(soft.ps2.px.iter().zip(&hard.ps2.px).any(|(a, b)| a[3] < b[3] && a[3] > 0) || soft.hd[0].px.iter().zip(&hard.hd[0].px).any(|(a, b)| a[3] < b[3] && a[3] > 0));
    }

    #[test]
    fn cut_matches_kh1() {
        assert_eq!(cut(1, 6, 26), 22);
        assert_eq!(cut(5, 6, 26), 4);
        assert_eq!(cut(29, 30, 24), 1);
        assert_eq!(cut(1, 30, 24), 23);
    }

    #[test]
    fn layd_and_imgz_read_back() {
        let s = Seqd { version: 0x160, sprites: vec![sprite([0, 0, 1, 1], [0x8080_8080; 4])], parts: vec![part([0, 0, 1, 1], 0)], sgroups: vec![sgroup(0, 1)], anims: vec![anim(FLAGS_STATIC, 0, (0, 1), 0, (1.0, 1.0), (0, 0))], agroups: vec![agroup(0, 1, true, 1)] };
        let l = layd(&s.write());
        let p = Layd::parse(&l).unwrap();
        assert_eq!(p.seqs.len(), 1);
        assert_eq!(Seqd::parse(&p.seqs[0]).unwrap().agroups.len(), 1);
        let z = imgz(&Pic::blank(1024, 512));
        assert_eq!(&z[0x20..0x24], b"IMGD");
        let w = |at: usize| u32::from_le_bytes(z[0x20 + at..0x24 + at].try_into().unwrap());
        // As UK_Cockpit.imz's words: 0x100, 0x40, size, size + 0x40, 0, −1, 1024 / 512, …
        assert_eq!((w(4), w(8), w(0xC), w(0x10), w(0x14), w(0x18)), (0x100, 0x40, 1024 * 512 * 4, 1024 * 512 * 4 + 0x40, 0, 0xFFFF_FFFF));
        assert_eq!(w(0x1C), 512 << 16 | 1024);
        assert_eq!(w(0x20), 9 << 16 | 10);
        assert_eq!((w(0x24), w(0x28), w(0x2C), w(0x30), w(0x34), w(0x38), w(0x3C)), (0x10, 0xFFFF_FFFF, 0x0010_0010, 0x0013_0001, 3, 0, 3));
        assert_eq!(z.len(), 0x20 + 0x40 + 1024 * 512 * 4);
    }

    #[test]
    fn held_groups_extend_the_last_frame() {
        let p = |x: i32| Piece { item: 0, uv: [0, 0, 1, 1], rect: [x, 0, x + 1, 1], colour: 0 };
        let g = held(&[vec![], vec![p(0)], vec![p(0), p(1)]]);
        let mut runs: Vec<(Vec<Piece>, (i32, i32))> = g.steps.iter().map(|s| (s.pieces.clone(), s.frames)).collect();
        runs.sort_by_key(|r| r.1);
        assert_eq!(runs, vec![(vec![p(0)], (1, LONG)), (vec![p(1)], (2, LONG))]);
    }

    #[test]
    fn languages_from_cockpit_copies() {
        let a = |n: &str| HdAsset { name: n.into(), orig_offset: -1, data: vec![] };
        assert_eq!(languages(&[a("UK_Cockpit.lad"), a("FR_gauge.lad"), a("FR_Cockpit.lad"), a("Cockpit.lad")]), vec!["FR", "UK"]);
    }

    /// With `KH1_HUD_DUMP` set to a `hud-h0` dump folder (the player's own files, outside the
    /// repository): builds the set from it and writes the atlases and layouts to `KH1_HUD_OUT`.
    #[test]
    fn builds_from_a_dump() {
        let Ok(dir) = std::env::var("KH1_HUD_DUMP") else { return };
        let d = std::path::Path::new(&dir).join("kh1");
        let rd = |p: &str| std::fs::read(d.join(p)).unwrap();
        let assets = |p: &str| -> Vec<HdAsset> {
            std::fs::read_dir(d.join(p)).map(|r| r.flatten().filter(|e| !e.file_name().to_string_lossy().ends_with(".png")).map(|e| HdAsset { name: e.file_name().to_string_lossy().into_owned(), orig_offset: -1, data: std::fs::read(e.path()).unwrap() }).collect()).unwrap_or_default()
        };
        let face = |n: &str, at: usize| face_section(&rd(&format!("faces/{n}.sec2.bin")), at, &assets(&format!("faces/{n}.hd"))).unwrap();
        let uitex_hd = assets("command2/uitex.bin.hd");
        let src = Sources {
            uitex: &rd("command2/uitex.bin"),
            uitex_hd: &uitex_hd,
            g_s: &rd("exchange/UK_uibin_g_s.bin"),
            g_red: &rd("exchange/UK_uibin_g_red.bin"),
            g_act: &rd("exchange/UK_uibin_g_act.bin"),
            faces: [face("xa_ex_0010.mdls", 0xA2E80), face("xa_ex_0040.mdls", 0x75600), face("xa_ex_0050.mdls", 0x8D400)],
        };
        let langs = ["FR", "GR", "IT", "SP", "UK"];
        let b = build(&src, &langs).unwrap();
        // The owner's FO0002.SPR / .RTM with their PC assets: the set goes in, Re:CoM's stays.
        let r = std::path::Path::new(&dir).join("recom/FORM/0002");
        let ra = |p: &str| -> Vec<HdAsset> {
            let mut v: Vec<HdAsset> = std::fs::read_dir(r.join(p)).unwrap().flatten().filter(|e| !e.file_name().to_string_lossy().ends_with(".png")).map(|e| HdAsset { name: e.file_name().to_string_lossy().into_owned(), orig_offset: 0xFFFF, data: std::fs::read(e.path()).unwrap() }).collect();
            v.sort_by(|a, b| a.name.cmp(&b.name));
            v
        };
        let spr = std::fs::read(r.join("FO0002.SPR")).unwrap();
        let mut sa = ra("FO0002.SPR.hd");
        let ls = languages(&sa);
        assert_eq!(ls, langs);
        let spr2 = add_to_spr(&spr, &mut sa, &b, &ls).unwrap();
        let c = Container::parse(&spr2).unwrap();
        let c0 = Container::parse(&spr).unwrap();
        assert_eq!(c.entries.len(), c0.entries.len() + 1);
        assert_eq!(c.entries[..c0.entries.len()], c0.entries[..]);
        assert!(sa.iter().any(|a| a.name == "UK_kh1hud.lad"));
        assert!(add_to_spr(&spr2, &mut sa.clone(), &b, &ls).is_err());
        let rtm = std::fs::read(r.join("FO0002.RTM")).unwrap();
        let mut rta = ra("FO0002.RTM.hd");
        let rtm2 = add_to_rtm(&rtm, &mut rta, &b, &ls).unwrap();
        let c = Container::parse(&rtm2).unwrap();
        assert_eq!(c.entries.last().unwrap().name(), IMZ);
        assert_eq!(c.entries.last().unwrap().name_raw[0x14..], Container::parse(&rtm).unwrap().entries[0].name_raw[0x14..]);
        assert_eq!(rta.len(), 10);
        let sq = Seqd::parse(&Layd::parse(&b.lad).unwrap().seqs[0]).unwrap();
        eprintln!(
            "atlas {:?}, {} items, {} sprites, lad {} / {} bytes; {} groups, {} units ({} with slides), {} runs, {} animations, {} parts",
            b.atlas,
            b.pieces,
            b.sprites,
            b.lad.len(),
            b.lad_hd.len(),
            b.groups,
            b.motion.first.len(),
            b.motion.unit_slides.iter().filter(|&&x| x).count(),
            b.motion.runs.len(),
            sq.anims.len(),
            sq.parts.len()
        );
        assert_eq!(sq.agroups.len(), b.groups);
        // KH1's g_act tracks at 2 frames a key unit: vibe_s 5 units, vibe_ss 11, vibe_sss 19,
        // the slides 10.
        assert_eq!(b.motion.programs.iter().map(|p| p.len).collect::<Vec<_>>(), vec![10, 22, 38, 10, 10, 20, 20]);
        if let Ok(out) = std::env::var("KH1_HUD_OUT") {
            let o = std::path::Path::new(&out);
            std::fs::write(o.join("kh1hud.lad"), &b.lad).unwrap();
            std::fs::write(o.join("UK_kh1hud.lad"), &b.lad_hd).unwrap();
            std::fs::write(o.join("kh1hud.imz"), &b.imz).unwrap();
            std::fs::write(o.join("UK_kh1hud.imz"), &b.imz_hd[4]).unwrap();
        }
    }
}
