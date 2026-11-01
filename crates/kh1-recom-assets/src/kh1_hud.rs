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
/// KH1's command menu layouts (H4): the root list (`com`, 12 objects), its battle title colours
/// (`com_battle`), the Magic list (`mag`, 18 objects). Loader `0x140277FE0`; Checked (files).
pub const COM: &str = "exchange/UK_uibin_com.bin";
/// The root title's field / battle colour keys.
pub const COM_BATTLE: &str = "exchange/UK_com_battle.bin";
/// The Magic list.
pub const MAG: &str = "exchange/UK_uibin_mag.bin";
/// `uitex.bin` pictures the menu draws from (`0x140278180` passes picture 2 for the root's panels,
/// 3 for the submenus', 1 for the titles and icons): (index, TIM2 offset, pixel data offset).
pub const MENU_PICTURES: [(usize, usize, usize); 3] = [(1, 0x2080, 0x20C0), (2, 0x4100, 0x4140), (3, 0x6180, 0x61C0)];
/// Picture 10 (the lock-on cursors, H7; PC `-18080.dds` 2×).
pub const CURSOR_PICTURE: (usize, usize, usize) = (10, 0x18040, 0x18080);
/// Picture 0 (the LOCK ON! plate's art at texels 100..125 × 100..123, H5; the PC copy 4×).
pub const PLATE_PICTURE: (usize, usize, usize) = (0, 0, 0x40);
/// The corner of picture 0 taken into the atlas (texels 96..128 square).
pub const PLATE_CORNER: usize = 96;
/// The LOCK ON! plate (`0x1402740F0` loads it; objects 5..10) and the lock-on cursors (0..4, H7).
pub const LOCK: &str = "exchange/UK_uibin_lock.bin";
/// The target's HP gauge (`0x140274E00`).
pub const G_ENM: &str = "exchange/UK_uibin_g_enm.bin";
/// KH1's LEVEL UP!! box (`0x1402724D0` loads it; H8).
pub const LEV_S: &str = "exchange/UK_uibin_lev_s.bin";
/// The party's gauges (slots 1, 2; loader `0x140268800`).
pub const G_C: &str = "exchange/UK_uibin_g_c.bin";
/// KH1 PC's menu font (font 1 of the table `0x1404D9150`, the command words `0x14027A920` →
/// `0x1402D1CD0`): `Image/dt/Theater` `<lang>/font/menufont.tm2` (2048 × 2048 32-bit), `.cod`
/// (8-byte glyphs: code, x, y, 0, width), `.inf` (count, size, cell). Checked (files).
pub const FONT_FILES: [&str; 3] = ["uk/font/menufont.tm2", "uk/font/menufont.cod", "uk/font/menufont.inf"];
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
    // Picture 1's language copies are PNGs (`<LANG>_uitex_bin20c0.png`, Checked (files)).
    let names = [format!("{lang}_uitex_bin{off:x}.dds"), format!("{lang}_uitex_bin{off:x}.png"), format!("-{off:x}.dds")];
    for n in &names {
        if let Some(a) = assets.iter().find(|a| a.name.eq_ignore_ascii_case(n)) {
            let img = if n.ends_with(".png") { crate::texture::decode_png(&a.data) } else { decode_dds(&a.data) };
            return Ok(Some(from_pc(&img.map_err(|e| crate::Error::Format(format!("uitex.bin asset {}: {e}", a.name)))?)));
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
pub(crate) struct Atlas {
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
    /// The bottom vertices' colour when not 0 (KH1's 4-vertex-colour objects, flag 0x20, drawn as
    /// a vertical gradient: their top and bottom pairs are equal in the files used).
    grad: u32,
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
    let mut spr: HashMap<([i32; 4], u32, u32), usize> = HashMap::new();
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
                        let g = if p.grad != 0 { p.grad } else { p.colour };
                        let si = *spr.entry((uv, p.colour, g)).or_insert_with(|| {
                            // Re:CoM's sprite colours: TL, TR, BL, BR (`mp_bar` assumption 3).
                            s.sprites.push(sprite(uv, [p.colour, p.colour, g, g]));
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
    /// Picture 5 itself as an atlas item (the party's full quarter rings are its texels).
    fn src5_item(&mut self, atlas: &mut Atlas, ps2: &Source, per: &[Pic]) -> usize {
        atlas.add(ps2.ps2.clone(), vec![per[0].clone()], ps2.scale)
    }

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
            Some((it, (x, y, w, h))) => Piece { item: it, uv: [0, 0, w, h], rect: [x, y, x + w, y + h], colour: 0x8080_8080, grad: 0 },
            None => Piece { item: self.empty, uv: [0, 0, 1, 1], rect: [0, 0, 1, 1], colour: 0, grad: 0 },
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
    /// H4: `com`, `com_battle`, `mag` and the menu font; the command words per language (as the
    /// command menu decodes them). Without them no menu look is built.
    pub menu: Option<MenuSources<'a>>,
    /// H5: `lock` and `g_enm`.
    pub lock: Option<&'a [u8]>,
    /// `g_enm`.
    pub g_enm: Option<&'a [u8]>,
    /// H6: `g_c`.
    pub g_c: Option<&'a [u8]>,
    /// H8: `lev_s`.
    pub lev_s: Option<&'a [u8]>,
    /// The three faces (Sora, Donald, Goofy).
    pub faces: [Source; 3],
}

/// The command menu's inputs (H4).
pub struct MenuSources<'a> {
    /// `com`.
    pub com: &'a [u8],
    /// `com_battle`.
    pub com_battle: &'a [u8],
    /// `mag`.
    pub mag: &'a [u8],
    /// The menu font (`menufont.tm2`, `.cod`, `.inf`).
    pub font: [&'a [u8]; 3],
    /// (language, [(message id, word)]) as `command_menu::words` decodes them.
    pub words: Vec<(String, Vec<(u16, String)>)>,
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
    /// H4: the menu groups (None without menu sources).
    pub menu: Option<menu::Built>,
    /// H5: the target groups' first group.
    pub target: Option<usize>,
    /// H6: Donald's and Goofy's gauge groups.
    pub party: Option<[usize; 2]>,
    /// H7: the lock-on cursor's group.
    pub cursor: Option<usize>,
    /// H8: the LEVEL UP!! box's group.
    pub level_up: Option<usize>,
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
    Piece { item, uv, rect, colour: colour_of(o), grad: 0 }
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
    let face_piece = |e: i32| Piece { item: faces[0], uv: [fh * e, 0, fh * (e + 1), fh], rect: [cx - fh / 2, cy - fh / 2, cx + fh / 2, cy + fh / 2], colour: 0x8080_8080, grad: 0 };
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
    // H4 / H5: pictures 1, 2, 3 (titles, icons, words; the root's panels; the submenus') and
    // picture 0's LOCK ON! plate corner (texels 96..128 square: the plate's art, at the PC's 4×).
    let mut mp = Vec::new();
    for (i, off, data) in MENU_PICTURES {
        let t = tims.get(i).ok_or_else(|| crate::Error::Format(format!("uitex.bin: no picture {i}")))?;
        if t.offset != off || t.width != 128 || t.height != 128 {
            return bad(format!("uitex.bin: picture {i} at {:#x} {}×{}, KH1 draws it from {off:#x} 128×128", t.offset, t.width, t.height));
        }
        let ps2 = Pic { w: 128, h: 128, px: t.pixels.clone() };
        let mut per = Vec::new();
        for l in langs {
            per.push(uitex_asset(src.uitex_hd, data, l)?.ok_or_else(|| crate::Error::Format(format!("uitex.bin: no PC texture for picture {i}")))?);
        }
        let scale = hd_scale(&ps2, &per[0])?;
        let per: Vec<Pic> = per.into_iter().map(|p| p.crop(0, 0, 128 * scale, 128 * scale)).collect();
        // One shared PC picture when every language has the same one (pictures 2, 3).
        let per = if per.iter().all(|p| *p == per[0]) { vec![per[0].clone()] } else { per };
        mp.push(atlas.add(ps2, per, scale));
    }
    let p0 = {
        let (i, off, data) = PLATE_PICTURE;
        let t = tims.get(i).ok_or_else(|| crate::Error::Format(format!("uitex.bin: no picture {i}")))?;
        if t.offset != off || t.width != 128 || t.height != 128 {
            return bad(format!("uitex.bin: picture {i} at {:#x} {}×{}", t.offset, t.width, t.height));
        }
        let ps2 = Pic { w: 128, h: 128, px: t.pixels.clone() };
        let mut per = Vec::new();
        for l in langs {
            per.push(uitex_asset(src.uitex_hd, data, l)?.ok_or_else(|| crate::Error::Format(format!("uitex.bin: no PC texture for picture {i}")))?);
        }
        let scale = hd_scale(&ps2, &per[0])?;
        let c = PLATE_CORNER;
        let per: Vec<Pic> = per.into_iter().map(|p| p.crop(c * scale, c * scale, (128 - c) * scale, (128 - c) * scale)).collect();
        let per = if per.iter().all(|p| *p == per[0]) { vec![per[0].clone()] } else { per };
        atlas.add(ps2.crop(c, c, 128 - c, 128 - c), per, scale)
    };
    // H4: KH1's command menu pieces and words.
    let menu_built = match &src.menu {
        Some(ms) => Some(menu::build(ms, &mut atlas, [mp[0], mp[1], mp[2]], &mut groups)?),
        None => None,
    };
    // H5: the LOCK ON! plate and the target's HP gauge.
    let target_built = match (src.lock, src.g_enm) {
        (Some(l), Some(e)) => Some(target::build(l, e, [p0, mp[0], it6], &mut groups)?),
        _ => None,
    };
    // H7: KH1's lock-on cursor (`lock` object 2, `lockon_c`, picture 10).
    let cursor_built = match src.lock {
        Some(l) => {
            let (i, off, data) = CURSOR_PICTURE;
            let t = tims.get(i).ok_or_else(|| crate::Error::Format(format!("uitex.bin: no picture {i}")))?;
            if t.offset != off || t.width != 128 || t.height != 128 {
                return bad(format!("uitex.bin: picture {i} at {:#x} {}×{}", t.offset, t.width, t.height));
            }
            let ps2 = Pic { w: 128, h: 128, px: t.pixels.clone() };
            let hd = uitex_asset(src.uitex_hd, data, langs[0])?.ok_or_else(|| crate::Error::Format(format!("uitex.bin: no PC texture for picture {i}")))?;
            let scale = hd_scale(&ps2, &hd)?;
            let item = atlas.add(ps2, vec![hd.crop(0, 0, 128 * scale, 128 * scale)], scale);
            Some(cursor::build(l, item, &mut groups)?)
        }
        None => None,
    };
    // H6: Donald's and Goofy's gauges, full (Re:CoM's friends have no HP or MP).
    let party_built = match src.g_c {
        Some(gc) => Some(party::build(gc, [it6, bk.src5_item(&mut atlas, &pics[p5], &pic_langs[p5])], [faces[1], faces[2]], [src.faces[1].ps2.h as i32, src.faces[2].ps2.h as i32], &mut groups)?),
        None => None,
    };
    // H8: KH1's LEVEL UP!! box (`lev_s`, pictures 3 and 1).
    let level_built = match src.lev_s {
        Some(l) => Some(popup::level_up(l, [mp[2], mp[0]], &mut groups)?),
        None => None,
    };
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
        menu: menu_built,
        target: target_built,
        party: party_built,
        cursor: cursor_built,
        level_up: level_built,
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

/// H4: KH1's command menu look (decision 39: KH1's panels, cursor and COMMAND title, the field look
/// in the field and the battle look in battle rooms, the words in KH1's own menu font).
///
/// KH1 (Steam; Checked (decomp) unless tagged): `0x14027B870` draws the root list at (16, 260)
/// (+ the left margin), a submenu at its parent's x + 11; `0x140278B40` draws a list: its header
/// (`*_fra_top` + the title) at the origin, rows at y + 20 + 22 i (the frame `*_fra_c`, the last
/// row `*_fra_d` + the connector `*_fra_cd`), the cursor row not as a frame but as the selection
/// (`*_sel_sha / act / gra / bla` + the command's icon) at (x + 11, y + 18 + 22 c); words at the
/// row's (x + 16, y + 22 + 22 i), the cursor row's at (x + 29, y + 25 + 22 c) (`0x14027A920` adds
/// (16, 2)). The root's pieces are 24 units wider (its right vertices and their texels, the
/// `param_10` of `0x14027A600` / `0x14027A9F0`: the panels' art in picture 2 runs to texel 125,
/// Checked (files)). In battle (`0x142867364 & 1`, the root record's `+0x1F8` = 0) the root's
/// panels are drawn with RGB (109, 30, 32) and the selection's plate (216, 109, 100)
/// (`0x14027A600`; the colours are `0x140278B40`'s locals), and the title is `com_battle`'s
/// `com_command_start` (blue → white → red over 20 frames, restarted when the mode changes),
/// `com_command_end` in the field.
pub mod menu {
    use super::*;

    /// Groups from the menu's first group: the root's field look, its battle look (each [`LOOK`]
    /// groups), the battle title's animation, the Magic list, then the words.
    pub mod g {
        /// Header frame.
        pub const HEADER: usize = 0;
        /// Title ("COMMAND"), static.
        pub const TITLE: usize = 1;
        /// A row's frame.
        pub const ROW: usize = 2;
        /// The last row's frame and connector.
        pub const ROW_LAST: usize = 3;
        /// The selection (cursor row).
        pub const SEL: usize = 4;
        /// The icons a / b / c (Attack, Magic, Items).
        pub const ICON: usize = 5;
        /// Groups a look.
        pub const LOOK: usize = 8;
        /// The battle look's first group.
        pub const BATTLE: usize = LOOK;
        /// The title turning red (played from frame 1, 20 frames, then held).
        pub const TITLE_TO_BATTLE: usize = 2 * LOOK;
        /// The Magic list: header, title, row, last row, selection, icon.
        pub const MAGIC: usize = 2 * LOOK + 1;
        /// The words.
        pub const WORDS: usize = MAGIC + 6;
    }

    /// The root's extra width (`param_10` = 24 for list type 0, `0x140278B40`).
    pub const ROOT_EXT: i32 = 24;
    /// Battle colours (`0x140278B40`: `local_168`, `local_118`).
    pub const BATTLE_PANEL: [i32; 3] = [109, 30, 32];
    /// The battle selection plate's colour.
    pub const BATTLE_SEL: [i32; 3] = [216, 109, 100];
    /// The menu font's size over its cell: KH1 draws font 1 at size 14 (`0x14054534C`) in units;
    /// the PC font's cells are 32 texels: 14 / 32 units a texel. Assumption (strong: "Attack"
    /// measures 43 units on the owner's KH1 screenshot, 96 texels of advances × 14 / 32 = 42).
    pub const FONT_UNITS: f32 = 14.0 / 32.0;
    /// A space's advance (texels): the font has no glyph for it. Assumption.
    pub const SPACE: i32 = 9;

    /// The built menu: its first group and the words' groups.
    #[derive(Clone, Debug)]
    pub struct Built {
        /// First group.
        pub first: usize,
        /// (language, [(message id, group)]).
        pub words: Vec<(String, Vec<(u16, usize)>)>,
        /// Glyphs found / characters missing from the font.
        pub glyphs: usize,
        /// Characters without a glyph.
        pub missing: Vec<char>,
    }

    /// The menu font: its 32-bit picture and glyphs (code → x, y, width).
    pub struct Font {
        w: usize,
        px: Vec<[u8; 4]>,
        cell: usize,
        glyphs: std::collections::BTreeMap<u32, (usize, usize, usize)>,
    }

    /// Reads `menufont.tm2` / `.cod` / `.inf`.
    pub fn font(tm2: &[u8], cod: &[u8], inf: &[u8]) -> Result<Font> {
        let u16at = |d: &[u8], at: usize| d.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
        let (w, h) = (u16at(inf, 2).unwrap_or(0), u16at(inf, 4).unwrap_or(0));
        let cell = *inf.get(6).unwrap_or(&0) as usize;
        if &tm2.get(..4).unwrap_or(&[]) != b"TIM2" || tm2.len() < 0x40 || cell == 0 {
            return bad("menufont: not a TIM2 / no cell size");
        }
        let hs = u16at(tm2, 0x1C).unwrap_or(0);
        let (tw, th, kind) = (u16at(tm2, 0x24).unwrap_or(0), u16at(tm2, 0x26).unwrap_or(0), tm2[0x23]);
        if kind != 3 || tw != w || th != h || 0x10 + hs + w * h * 4 > tm2.len() {
            return bad(format!("menufont.tm2: {tw}×{th} type {kind}, .inf says {w}×{h} 32-bit"));
        }
        let px = tm2[0x10 + hs..0x10 + hs + w * h * 4].chunks(4).map(|p| [p[0], p[1], p[2], p[3]]).collect();
        let mut glyphs = std::collections::BTreeMap::new();
        for e in cod.chunks_exact(8) {
            let code = u16::from_le_bytes([e[0], e[1]]) as u32;
            let (x, y) = (u16::from_le_bytes([e[2], e[3]]) as usize, u16::from_le_bytes([e[4], e[5]]) as usize);
            if x + cell <= w && y + cell <= h {
                glyphs.insert(code, (x, y, e[7] as usize));
            }
        }
        Ok(Font { w, px, cell, glyphs })
    }

    impl Font {
        /// A glyph's cell: the PS2-size picture (half) and the PC one.
        fn cell_pics(&self, x: usize, y: usize) -> (Pic, Pic) {
            let c = self.cell;
            let mut hd = Pic::blank(c, c);
            for j in 0..c {
                for i in 0..c {
                    hd.px[j * c + i] = self.px[(y + j) * self.w + x + i];
                }
            }
            let h = c / 2;
            let mut ps2 = Pic::blank(h, h);
            for j in 0..h {
                for i in 0..h {
                    let mut a = [0u32; 4];
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let p = hd.px[(2 * j + dy) * c + 2 * i + dx];
                        for k in 0..4 {
                            a[k] += p[k] as u32;
                        }
                    }
                    ps2.px[j * h + i] = a.map(|v| ((v + 2) / 4) as u8);
                }
            }
            // The PC atlas holds full-range RGB (KH1's and Re:CoM's PC pictures); the font's texels
            // are PS2-range (0x80 = white, Checked (files)): doubled for the PC copy.
            for p in hd.px.iter_mut() {
                for k in 0..3 {
                    p[k] = (p[k] as u32 * 255 / 128).min(255) as u8;
                }
            }
            (ps2, hd)
        }
    }

    /// An object's piece: `ext` units (and texels) wider, its RGB replaced by `over`.
    fn mpiece(o: &Obj, item: usize, dx: i32, dy: i32, ext: i32, over: Option<[i32; 3]>) -> Piece {
        let (mut uv, ..) = o.texels();
        uv[2] += ext;
        let [x, y, w, h] = o.rect;
        let col = |c: [i32; 4]| {
            let c = match over {
                Some(o) => [o[0], o[1], o[2], c[3]],
                None => c,
            };
            let b = |v: i32| v.clamp(0, 255) as u32;
            b(c[0]) | b(c[1]) << 8 | b(c[2]) << 16 | b(c[3]) << 24
        };
        let grad = if o.flags & 0x20 != 0 { col(o.colours[1]) } else { 0 };
        Piece { item, uv, rect: [x + dx, y + dy, x + dx + w + ext, y + dy + h], colour: col(o.colours[0]), grad }
    }

    fn held1(pieces: Vec<Piece>) -> Group {
        let one = 0x8080_8080;
        Group { steps: vec![Step { pieces, frames: (0, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] }], once: true, word: GROUP_WORD_SCRUB }
    }

    /// Builds the menu groups (appended to `groups`) and the glyphs (into `atlas`).
    pub(crate) fn build(ms: &MenuSources, atlas: &mut Atlas, [p1, p2, p3]: [usize; 3], groups: &mut Vec<Group>) -> Result<Built> {
        let com = Uibin::parse(ms.com)?;
        let cb = Uibin::parse(ms.com_battle)?;
        let mag = Uibin::parse(ms.mag)?;
        let need = |u: &Uibin, n: &str, names: &[&str]| -> Result<()> {
            for (i, want) in names.iter().enumerate() {
                if u.objs.get(i).map(|o| o.name.as_str()) != Some(want) {
                    return bad(format!("{n}: object {i} is not {want} (not KH1 FM Steam's layout)"));
                }
            }
            Ok(())
        };
        need(&com, COM, &["com_fra_d", "com_fra_cd", "com_fra_c", "com_fra_top", "com_sel_sha_d", "com_sel_act_d", "com_sel_gra_d", "com_sel_bla_d", "com_command", "com_sel_ico_a", "com_sel_ico_b", "com_sel_ico_c"])?;
        need(&cb, COM_BATTLE, &["com_command_start", "com_command_end"])?;
        need(&mag, MAG, &["mag_fra_d", "mag_fra_2_d", "mag_fra_cd", "mag_fra_c", "mag_fra_2_c", "mag_fra_top", "mag_fra_2_top", "mag_sel_sha_d", "mag_sel_sha_2_d", "mag_sel_act_d", "mag_sel_act_2_d", "mag_sel_gra_d", "mag_sel_gra_2_d", "mag_sel_bla_d", "mag_sel_bla_2_d", "mag_magic", "mag_sel_ico_d"])?;
        let first = groups.len();
        let o = &com.objs;
        let ext = ROOT_EXT;
        // The title's colours: com_command_end's last key (field), com_command_start's (battle).
        let keys_of = |obj: &Obj| -> Vec<crate::uibin::Key> {
            let (f, n) = obj.keys_a;
            if f < 0 || n <= 0 {
                return Vec::new();
            }
            cb.keys_a.get(f as usize..(f + n) as usize).map(|k| k.to_vec()).unwrap_or_default()
        };
        let start = keys_of(&cb.objs[0]);
        let end = keys_of(&cb.objs[1]);
        if start.len() < 2 || end.len() < 2 {
            return bad(format!("{COM_BATTLE}: the title keys are not KH1's"));
        }
        let rgba = |k: &crate::uibin::Key| -> [i32; 4] { [k.vals[0] as i32, k.vals[1] as i32, k.vals[2] as i32, k.vals[3] as i32] };
        let title_col = [rgba(end.last().unwrap()), rgba(start.last().unwrap())];
        for (look, over, sel_over) in [(0, None, None), (1, Some(BATTLE_PANEL), Some(BATTLE_SEL))] {
            groups.push(held1(vec![mpiece(&o[3], p2, 0, 0, ext, over)]));
            let mut t = mpiece(&o[8], p1, 0, 0, 0, None);
            let c = title_col[look];
            t.colour = (c[0].clamp(0, 255) | c[1].clamp(0, 255) << 8 | c[2].clamp(0, 255) << 16 | c[3].clamp(0, 255) << 24) as u32;
            groups.push(held1(vec![t]));
            groups.push(held1(vec![mpiece(&o[2], p2, 0, 0, ext, over)]));
            groups.push(held1(vec![mpiece(&o[0], p2, 0, 0, ext, over), mpiece(&o[1], p2, 0, 0, 0, over)]));
            groups.push(held1(vec![mpiece(&o[4], p2, 0, 0, ext, None), mpiece(&o[5], p2, 0, 0, ext, sel_over), mpiece(&o[6], p2, 0, 0, ext, None), mpiece(&o[7], p2, 0, 0, ext, None)]));
            for k in 9..12 {
                groups.push(held1(vec![mpiece(&o[k], p1, ext, 0, 0, None)]));
            }
        }
        // The title turning red: com_command_start's colour keys (2 frames a key unit), from frame 1.
        {
            let mut t = mpiece(&o[8], p1, 0, 0, 0, None);
            t.colour = 0x8080_8080;
            let col = |c: [i32; 4]| (c[0].clamp(0, 255) | c[1].clamp(0, 255) << 8 | c[2].clamp(0, 255) << 16 | c[3].clamp(0, 255) << 24) as u32;
            let mut steps = Vec::new();
            let mut f = 0;
            for w in start.windows(2) {
                if w[0].dur <= 0 {
                    break;
                }
                let f1 = f + 2 * w[0].dur;
                steps.push(Step { pieces: vec![t], frames: (f, f1), colour: (col(rgba(&w[0])), col(rgba(&w[1]))), flags: FLAGS_COLOUR | 0x8000, tr: [0; 4] });
                f = f1;
            }
            let last = col(rgba(start.last().unwrap()));
            steps.push(Step { pieces: vec![t], frames: (f, LONG), colour: (last, last), flags: FLAGS_COLOUR, tr: [0; 4] });
            groups.push(Group { steps, once: true, word: GROUP_WORD_FADE });
        }
        // The Magic list (`0x14027A1C0`: the objects' own colours, no extra width).
        let m = &mag.objs;
        groups.push(held1(vec![mpiece(&m[5], p3, 0, 0, 0, None), mpiece(&m[6], p3, 0, 0, 0, None)]));
        groups.push(held1(vec![mpiece(&m[15], p1, 0, 0, 0, None)]));
        groups.push(held1(vec![mpiece(&m[3], p3, 0, 0, 0, None), mpiece(&m[4], p3, 0, 0, 0, None)]));
        groups.push(held1(vec![mpiece(&m[0], p3, 0, 0, 0, None), mpiece(&m[1], p3, 0, 0, 0, None), mpiece(&m[2], p3, 0, 0, 0, None)]));
        groups.push(held1((7..15).map(|k| mpiece(&m[k], p3, 0, 0, 0, None)).collect()));
        groups.push(held1(vec![mpiece(&m[16], p1, 0, 0, 0, None)]));
        debug_assert_eq!(groups.len() - first, g::WORDS);
        // Words: KH1's menu font, a glyph a part, at the font's size.
        let f = font(ms.font[0], ms.font[1], ms.font[2])?;
        let mut glyph_item: std::collections::BTreeMap<u32, usize> = Default::default();
        let mut missing = Vec::new();
        let mut words = Vec::new();
        for (lang, ws) in &ms.words {
            let mut list = Vec::new();
            for (id, text) in ws {
                let mut pieces = Vec::new();
                let mut x = 0.0f32;
                for ch in text.chars() {
                    let Some(&(gx, gy, w)) = f.glyphs.get(&(ch as u32)) else {
                        if ch != ' ' && !missing.contains(&ch) {
                            missing.push(ch);
                        }
                        x += SPACE as f32 * FONT_UNITS;
                        continue;
                    };
                    let item = *glyph_item.entry(ch as u32).or_insert_with(|| {
                        let (ps2, hd) = f.cell_pics(gx, gy);
                        atlas.add(ps2, vec![hd], 2)
                    });
                    let x0 = x.round() as i32;
                    let size = (f.cell as f32 * FONT_UNITS).round() as i32;
                    let half = (f.cell / 2) as i32;
                    pieces.push(Piece { item, uv: [0, 0, half, half], rect: [x0, 0, x0 + size, size], colour: 0x8080_8080, grad: 0 });
                    x += w as f32 * FONT_UNITS;
                }
                list.push((*id, groups.len()));
                groups.push(held1(pieces));
            }
            words.push((lang.clone(), list));
        }
        Ok(Built { first, words, glyphs: glyph_item.len(), missing })
    }
}

/// H5: KH1's LOCK ON! plate (`0x1402747F0`) and the target's HP gauge (`0x1402758B0` →
/// `0x140275000`). KH1 (Checked (decomp) unless tagged):
/// - the plate: `lock` objects 5..7 (picture 0) and 8..10 (picture 1, "LOCK ON!") at (left margin
///   + 16, 24); on a lock their colour keys restart (`0x14027E710`) and play (alpha 0 → peak over
///   2 frames, → rest over 10), then hold; at the lock's end alpha − 1/6 a frame (6 frames);
/// - the gauge at (left margin + 16, 24 + 28 while the plate shows, `0x1402743E0`): `g_enm` 0
///   (picture 6) and 1..3 (picture 1, the green "HP") at the origin; 4 (picture 6) and 5 (1)
///   `L` = min(max HP / 2, 150) wide from x + 28; the cap 12 (6) and 13 (1) at x + 28 + L; five bars
///   (one per 300 HP, each min(HP − 300 k + 1, 300) / 2 wide, the texture stretched); with more
///   than one bar a marker (11) at the top bar's end; alpha 0 → 1 by 0.2 a frame and back.
/// The widths are Re:CoM X scales over frames 0..150 (as the MP bar's partial segment), the
/// moving pieces controls of their own, placed on events.
pub mod target {
    use super::*;

    /// Groups from the first: the plate (played), the gauge's fixed part, its length, its cap,
    /// the five bars, the marker.
    pub mod g {
        /// The plate (played from frame 1: its colour keys, then held).
        pub const PLATE: usize = 0;
        /// The gauge's head ("HP").
        pub const HEAD: usize = 1;
        /// Its body, X-scaled: frame = L (0..150).
        pub const BODY: usize = 2;
        /// Its cap.
        pub const CAP: usize = 3;
        /// Bar k (0..4), X-scaled: frame = its length.
        pub const BAR: usize = 4;
        /// The top bar's end marker.
        pub const MARKER: usize = 9;
        /// Groups.
        pub const COUNT: usize = 10;
    }

    /// The bars' colours by stack index: green first, then yellow, orange, magenta, violet
    /// (`g_enm` objects 10, 9, 8, 7, 6). Assumption (KH1's slot order not read).
    pub const BAR_OBJECTS: [usize; 5] = [10, 9, 8, 7, 6];

    fn grow(pieces: Vec<Piece>) -> Group {
        let one = 0x8080_8080;
        Group {
            steps: vec![
                Step { pieces: pieces.clone(), frames: (0, 150), colour: (one, one), flags: crate::mp_bar::FLAGS_GROW_PUB, tr: [0; 4] },
                Step { pieces, frames: (151, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] },
            ],
            once: true,
            word: GROUP_WORD_SCRUB,
        }
    }

    /// A piece at `dx` from its object's place (`x0`: replace the object's x).
    fn tp(o: &Obj, item: usize, x0: Option<i32>, dx: i32, uv_off: i32) -> Piece {
        let (mut uv, mu, mv) = o.texels();
        for k in 0..4 {
            uv[k] -= uv_off;
        }
        let [ox, oy, w, h] = o.rect;
        let x = x0.unwrap_or(ox) + dx;
        let rect = [if mu { x + w } else { x }, if mv { oy + h } else { oy }, if mu { x } else { x + w }, if mv { oy } else { oy + h }];
        let c = |c: [i32; 4]| c[0].clamp(0, 255) as u32 | (c[1].clamp(0, 255) as u32) << 8 | (c[2].clamp(0, 255) as u32) << 16 | (c[3].clamp(0, 255) as u32) << 24;
        let grad = if o.flags & 0x20 != 0 { c(o.colours[1]) } else { 0 };
        Piece { item, uv, rect, colour: c(o.colours[0]), grad }
    }

    /// Builds the groups (appended to `groups`); returns the first.
    pub(crate) fn build(lock: &[u8], g_enm: &[u8], [p0, p1, p6]: [usize; 3], groups: &mut Vec<Group>) -> Result<usize> {
        let l = Uibin::parse(lock)?;
        let e = Uibin::parse(g_enm)?;
        let ln = ["lockon_plate_a", "lockon_plate_b", "lockon_plate_c", "lockon_text_a", "lockon_text_b", "lockon_text_c"];
        for (k, n) in ln.iter().enumerate() {
            if l.objs.get(5 + k).map(|o| o.name.as_str()) != Some(n) {
                return bad(format!("{LOCK}: object {} is not {n}", 5 + k));
            }
        }
        let en = ["g_enm_out_a", "g_enm_fra_a", "g_enm_fra_hp", "g_enm_fra_aa", "g_enm_out_b", "g_enm_fra_b", "g_enm_bar_v", "g_enm_bar_m", "g_enm_bar_o", "g_enm_bar_y", "g_enm_bar_g", "g_enm_fra_p", "g_enm_out_c", "g_enm_fra_c"];
        for (k, n) in en.iter().enumerate() {
            if e.objs.get(k).map(|o| o.name.as_str()) != Some(n) {
                return bad(format!("{G_ENM}: object {k} is not {n}"));
            }
        }
        let first = groups.len();
        // The plate: each piece's colour keys (2 frames a key unit), played from frame 1, held.
        let mut steps = Vec::new();
        for (k, o) in l.objs[5..11].iter().enumerate() {
            let piece = if k < 3 { tp(o, p0, None, 0, PLATE_CORNER as i32) } else { tp(o, p1, None, 0, 0) };
            let (f0, n) = o.keys_a;
            let keys = l.keys_a.get(f0.max(0) as usize..(f0 + n).max(0) as usize).unwrap_or(&[]);
            let col = |k: &crate::uibin::Key| k.vals[0].clamp(0.0, 255.0) as u32 | (k.vals[1].clamp(0.0, 255.0) as u32) << 8 | (k.vals[2].clamp(0.0, 255.0) as u32) << 16 | (k.vals[3].clamp(0.0, 255.0) as u32) << 24;
            let mut f = 0;
            for w in keys.windows(2) {
                if w[0].dur <= 0 {
                    break;
                }
                let f1 = f + 2 * w[0].dur;
                // The piece's own colour is the multiplier's base: draw with colour 0x80 and the keys.
                steps.push(Step { pieces: vec![Piece { colour: 0x8080_8080, ..piece }], frames: (f, f1), colour: (col(&w[0]), col(&w[1])), flags: FLAGS_COLOUR | 0x8000, tr: [0; 4] });
                f = f1;
            }
            let last = keys.last().map(col).unwrap_or(piece.colour);
            steps.push(Step { pieces: vec![Piece { colour: 0x8080_8080, ..piece }], frames: (f, LONG), colour: (last, last), flags: FLAGS_COLOUR, tr: [0; 4] });
        }
        groups.push(Group { steps, once: true, word: GROUP_WORD_FADE });
        let one = 0x8080_8080;
        let held1 = |pieces: Vec<Piece>| Group { steps: vec![Step { pieces, frames: (0, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] }], once: true, word: GROUP_WORD_SCRUB };
        let o = &e.objs;
        groups.push(held1(vec![tp(&o[0], p6, None, 0, 0), tp(&o[1], p1, None, 0, 0), tp(&o[2], p1, None, 0, 0), tp(&o[3], p1, None, 0, 0)]));
        // Body and bars from x = 0 (their control at the origin + 28): X scale = width / 150.
        groups.push(grow(vec![tp(&o[4], p6, Some(0), 0, 0), tp(&o[5], p1, Some(0), 0, 0)]));
        groups.push(held1(vec![tp(&o[12], p6, Some(0), 0, 0), tp(&o[13], p1, Some(0), 0, 0)]));
        for k in BAR_OBJECTS {
            groups.push(grow(vec![tp(&o[k], p1, Some(0), 0, 0)]));
        }
        groups.push(held1(vec![tp(&o[11], p1, Some(0), 0, 0)]));
        debug_assert_eq!(groups.len() - first, g::COUNT);
        Ok(first)
    }
}

/// H6: Donald's and Goofy's gauges (slots 1 and 2; KH1 Checked (decomp + exe + files) unless
/// tagged): `g_c` objects at their places, the slot's offset (Donald (0, 0), Goofy (68, 20):
/// `0x140527330`, `0x140527370`) + the right margin; the face (48 × 48, `xa_ex_0040` / `0050`)
/// centred on (399, 44) (`0x1405272C0`, `0x14027F470`). Drawn full: the HP ring's three quarters
/// (`g_c_bar_b/c/d_hp`, picture 5, the texels KH1's fan samples for r26) and the MP ring's
/// (`_mp`, r32), over their dark backs (`g_c_fra_b/c/d_*`, picture 6), the face frame
/// (`g_c_fra_a_w`) and disc (`g_c_fra_a_a`). Order as Sora's (`0x14026CF90`): frame, disc, HP
/// back, MP back, MP, HP, face. Assumption: the party's draw order is Sora's.
pub mod party {
    use super::*;

    /// The face's centre (`0x1405272C0`).
    pub const CENTRE: (i32, i32) = (399, 44);

    /// Builds Donald's and Goofy's groups; returns them.
    pub(crate) fn build(g_c: &[u8], [p6, p5]: [usize; 2], faces: [usize; 2], face_h: [i32; 2], groups: &mut Vec<Group>) -> Result<[usize; 2]> {
        let u = Uibin::parse(g_c)?;
        let names = ["g_c_fra_a_w", "g_c_fra_a_a", "g_c_fra_b_hp", "g_c_fra_b_mp", "g_c_fra_c_hp", "g_c_fra_c_mp", "g_c_fra_d_hp", "g_c_fra_d_mp"];
        for (k, n) in names.iter().enumerate() {
            if u.objs.get(k).map(|o| o.name.as_str()) != Some(n) {
                return bad(format!("{G_C}: object {k} is not {n}"));
            }
        }
        for (k, n) in [(14, "g_c_bar_b_hp"), (15, "g_c_bar_b_mp"), (16, "g_c_bar_c_hp"), (17, "g_c_bar_c_mp"), (18, "g_c_bar_d_hp"), (19, "g_c_bar_d_mp")] {
            if u.objs.get(k).map(|o| o.name.as_str()) != Some(n) {
                return bad(format!("{G_C}: object {k} is not {n}"));
            }
        }
        let o = &u.objs;
        let one = 0x8080_8080;
        let mut out = [0usize; 2];
        for m in 0..2 {
            let mut v = vec![obj_piece(&o[0], p6, 0, 0, 0, None), obj_piece(&o[1], p6, 0, 0, 0, None)];
            v.extend([2, 4, 6].map(|k| obj_piece(&o[k], p6, 0, 0, 0, None)));
            v.extend([3, 5, 7].map(|k| obj_piece(&o[k], p6, 0, 0, 0, None)));
            v.extend([15, 17, 19].map(|k| obj_piece(&o[k], p5, 0, 0, 0, None)));
            v.extend([14, 16, 18].map(|k| obj_piece(&o[k], p5, 0, 0, 0, None)));
            let h = face_h[m];
            let (cx, cy) = CENTRE;
            v.push(Piece { item: faces[m], uv: [0, 0, h, h], rect: [cx - h / 2, cy - h / 2, cx + h / 2, cy + h / 2], colour: one, grad: 0 });
            out[m] = groups.len();
            groups.push(Group { steps: vec![Step { pieces: v, frames: (0, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] }], once: true, word: GROUP_WORD_SCRUB });
        }
        Ok(out)
    }
}

/// H7: KH1's lock-on cursor (`0x140274B20`; Checked (decomp + files)): a locked target (state 3,
/// sub-state 0) draws `lock` object 2 (`lockon_c`, picture 10, 38 × 38) centred on the target's
/// projected lock point: the quad at (−w / 2 + kx, −h / 2 + ky) of size (w, h) from its position
/// keys (−4, −4, 46, 46 → 2, 2, 34, 34 → 0, 0, 38, 38 over 4 + 4 frames) and its colour keys
/// (alpha 0 → 255 → 192), restarted when the cursor kind changes; drawn after the step (key time
/// 0.5 first). Baked per frame (frame k = key time (k + 1) / 2), then held.
pub mod cursor {
    use super::*;

    /// KH1's cursor of a lock (`DAT_14284F9E0` = object 2).
    pub const OBJECT: &str = "lockon_c";

    fn lerp(keys: &[crate::uibin::Key], t: f32) -> Vec<f32> {
        let mut at = 0.0;
        for w in keys.windows(2) {
            if w[0].dur <= 0 {
                break;
            }
            let d = w[0].dur as f32;
            if t <= at + d {
                let f = (t - at) / d;
                return w[0].vals.iter().zip(&w[1].vals).map(|(a, b)| a + (b - a) * f).collect();
            }
            at += d;
        }
        keys.last().map(|k| k.vals.clone()).unwrap_or_default()
    }

    /// Builds the cursor's group (appended); returns it.
    pub(crate) fn build(lock: &[u8], item: usize, groups: &mut Vec<Group>) -> Result<usize> {
        let u = Uibin::parse(lock)?;
        let o = u.objs.get(2).filter(|o| o.name == OBJECT).ok_or_else(|| crate::Error::Format(format!("{LOCK}: object 2 is not {OBJECT}")))?;
        let ck = u.c_keys(o).ok_or_else(|| crate::Error::Format(format!("{LOCK}: {OBJECT} has no position keys")))?;
        let (f0, n) = o.keys_a;
        let ak = u.keys_a.get(f0.max(0) as usize..(f0 + n).max(0) as usize).unwrap_or(&[]);
        let len = |k: &[crate::uibin::Key]| k.iter().take_while(|k| k.dur > 0).map(|k| k.dur).sum::<i32>();
        let frames = 2 * len(ck).max(len(ak));
        let (uv, ..) = o.texels();
        let col = |v: &[f32]| v[0].clamp(0.0, 255.0) as u32 | (v[1].clamp(0.0, 255.0) as u32) << 8 | (v[2].clamp(0.0, 255.0) as u32) << 16 | (v[3].clamp(0.0, 255.0) as u32) << 24;
        let piece = |t: f32| -> Piece {
            let p = lerp(ck, t);
            let (kx, ky, w, h) = (p[0] as i32, p[1] as i32, p[2] as i32, p[3] as i32);
            let x0 = -w / 2 + kx;
            let y0 = -h / 2 + ky;
            Piece { item, uv, rect: [x0, y0, x0 + w, y0 + h], colour: if ak.is_empty() { colour_of(o) } else { col(&lerp(ak, t)) }, grad: 0 }
        };
        let one = 0x8080_8080;
        let mut steps = Vec::new();
        for k in 0..frames {
            steps.push(Step { pieces: vec![piece((k + 1) as f32 / 2.0)], frames: (k, k), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] });
        }
        steps.push(Step { pieces: vec![piece(frames as f32)], frames: (frames, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] });
        groups.push(Group { steps, once: true, word: GROUP_WORD_FADE });
        Ok(groups.len() - 1)
    }
}

/// H8: KH1's popups where Re:CoM has the event. Built: the LEVEL UP!! box (`0x140272F10`, Checked
/// (decomp + files)): Sora's box at (left margin + 16, 16); state 1 restarts its objects' keys,
/// 2 steps, 3 holds 91 frames (`> 0x5A`), 4, then 5 fades alpha − 0.125 a frame, 6 ends. Its header
/// is `lev_s` objects 6..11 (frame, picture 3) and 12 (`level_up_s`, "LEVEL UP!!", picture 1),
/// each with colour keys (alpha 0 → 128 → 96 over 10 + 10 frames) and position keys (the drop-in:
/// y 36 → −12 → 6 → −3 → 0 over 10 + 6 + 2 + 2 frames). Baked per frame (frame k = key time
/// (k + 1) / 2), held, faded and ended in the data: Re:CoM plays it once.
pub mod popup {
    use super::*;

    /// The header's objects.
    pub const HEADER: [&str; 7] = ["lev_s_fra_top", "lev_s_fra_2_top", "lev_s_fra_3_top", "lev_s_fra_4_top", "lev_s_fra_5_top", "lev_s_fra_6_top", "level_up_s"];
    /// Frames KH1 holds the box (state 3 counts to 0x5A, then one frame of state 4), after the
    /// first two states' frames.
    pub const HOLD: i32 = 2 + 0x5B + 1;
    /// The fade: − 0.125 a frame.
    pub const FADE: i32 = 8;

    fn lerp(keys: &[crate::uibin::Key], t: f32) -> Option<Vec<f32>> {
        let mut at = 0.0;
        for w in keys.windows(2) {
            if w[0].dur <= 0 {
                break;
            }
            let d = w[0].dur as f32;
            if t <= at + d {
                let f = (t - at) / d;
                return Some(w[0].vals.iter().zip(&w[1].vals).map(|(a, b)| a + (b - a) * f).collect());
            }
            at += d;
        }
        keys.last().map(|k| k.vals.clone())
    }

    /// The LEVEL UP!! box's group (appended); returns it.
    pub(crate) fn level_up(lev_s: &[u8], [p3, p1]: [usize; 2], groups: &mut Vec<Group>) -> Result<usize> {
        let u = Uibin::parse(lev_s)?;
        let mut objs = Vec::new();
        for (k, n) in HEADER.iter().enumerate() {
            let o = u.objs.get(6 + k).filter(|o| o.name == *n).ok_or_else(|| crate::Error::Format(format!("{LEV_S}: object {} is not {n}", 6 + k)))?;
            objs.push(o);
        }
        let keys_of = |o: &Obj, a: bool| -> Vec<crate::uibin::Key> {
            let (f, n) = if a { o.keys_a } else { o.keys_c };
            let src = if a { &u.keys_a } else { &u.keys_c };
            src.get(f.max(0) as usize..(f + n).max(0) as usize).map(|k| k.to_vec()).unwrap_or_default()
        };
        let len = |k: &[crate::uibin::Key]| k.iter().take_while(|k| k.dur > 0).map(|k| k.dur).sum::<i32>();
        let keyed = objs.iter().map(|o| 2 * len(&keys_of(o, true)).max(len(&keys_of(o, false)))).max().unwrap_or(0);
        let col = |v: &[f32], f: f32| v[0].clamp(0.0, 255.0) as u32 | (v[1].clamp(0.0, 255.0) as u32) << 8 | (v[2].clamp(0.0, 255.0) as u32) << 16 | ((v[3] * f).clamp(0.0, 255.0) as u32) << 24;
        let pieces = |t: f32, fade: f32| -> Vec<Piece> {
            objs.iter()
                .enumerate()
                .map(|(k, o)| {
                    let (uv, mu, mv) = o.texels();
                    let r = lerp(&keys_of(o, false), t).map(|v| [v[0] as i32, v[1] as i32, v[2] as i32, v[3] as i32]).unwrap_or(o.rect);
                    let [x, y, w, h] = r;
                    let rect = [if mu { x + w } else { x }, if mv { y + h } else { y }, if mu { x } else { x + w }, if mv { y } else { y + h }];
                    let c = lerp(&keys_of(o, true), t).unwrap_or_else(|| o.colours[0].iter().map(|&v| v as f32).collect());
                    Piece { item: if k == 6 { p1 } else { p3 }, uv, rect, colour: col(&c, fade), grad: 0 }
                })
                .collect()
        };
        let one = 0x8080_8080;
        let mut steps = Vec::new();
        let hold = HOLD.max(keyed);
        for k in 0..hold {
            steps.push(Step { pieces: pieces((k + 1) as f32 / 2.0, 1.0), frames: (k, k), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] });
        }
        for j in 0..FADE {
            let f = 1.0 - 0.125 * (j + 1) as f32;
            steps.push(Step { pieces: pieces(hold as f32, f), frames: (hold + j, hold + j), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4] });
        }
        // Over: the group's longest animation (kept drawn after the end) shows nothing.
        steps.push(Step { pieces: pieces(hold as f32, 0.0), frames: (hold + FADE, LONG), colour: (0x0080_8080, 0x0080_8080), flags: FLAGS_STATIC, tr: [0; 4] });
        groups.push(Group { steps, once: true, word: GROUP_WORD_FADE });
        Ok(groups.len() - 1)
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
    if let Some(l) = b.level_up {
        let _ = writeln!(s, "  \"level_up\": {l},");
    }
    if let Some(c) = b.cursor {
        let _ = writeln!(s, "  \"cursor\": {c},");
    }
    if let Some([d, g]) = b.party {
        let _ = writeln!(s, "  \"party\": [{d}, {g}],");
    }
    if let Some(t) = b.target {
        let _ = writeln!(s, "  \"target\": {{\"first\": {t}}},");
    }
    if let Some(mn) = &b.menu {
        let words = mn.words.iter().map(|(l, w)| format!("\"{l}\": [{}]", w.iter().map(|(id, g)| format!("[{id}, {g}]")).collect::<Vec<_>>().join(", "))).collect::<Vec<_>>().join(", ");
        let _ = writeln!(s, "  \"menu\": {{\"first\": {}, \"words\": {{{words}}}}},", mn.first);
    }
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
        let p = Piece { item: 0, uv: [0, 0, 1, 1], rect: [0, 0, 1, 1], colour: 0, grad: 0 };
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
        let p = |x: i32| Piece { item: 0, uv: [0, 0, 1, 1], rect: [x, 0, x + 1, 1], colour: 0, grad: 0 };
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
        let (com, com_battle, mag) = (rd("exchange/UK_uibin_com.bin"), rd("exchange/UK_com_battle.bin"), rd("exchange/UK_uibin_mag.bin"));
        let (lock, g_enm) = (rd("exchange/UK_uibin_lock.bin"), rd("exchange/UK_uibin_g_enm.bin"));
        let fonts: Vec<Vec<u8>> = FONT_FILES.iter().map(|f| rd(&format!("fonts/{f}"))).collect();
        let src = Sources {
            uitex: &rd("command2/uitex.bin"),
            uitex_hd: &uitex_hd,
            g_s: &rd("exchange/UK_uibin_g_s.bin"),
            g_red: &rd("exchange/UK_uibin_g_red.bin"),
            g_act: &rd("exchange/UK_uibin_g_act.bin"),
            lock: Some(&lock),
            g_c: Some(&rd("exchange/UK_uibin_g_c.bin")),
            lev_s: Some(&rd("exchange/UK_uibin_lev_s.bin")),
            g_enm: Some(&g_enm),
            menu: Some(MenuSources {
                com: &com,
                com_battle: &com_battle,
                mag: &mag,
                font: [&fonts[0], &fonts[1], &fonts[2]],
                words: vec![("us".into(), vec![(1, "Attack".into()), (2, "Magic".into()), (3, "Items".into()), (6, "Summon".into()), (305, "-".into()), (294, "Open".into()), (313, "Jump On".into())])],
            }),
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
        let mn = b.menu.as_ref().unwrap();
        eprintln!("menu: first group {}, {} glyphs, missing {:?}, words {:?}", mn.first, mn.glyphs, mn.missing, mn.words);
        assert!(mn.missing.is_empty());
        assert!(b.target.is_some());
        assert!(b.party.is_some());
        assert!(b.cursor.is_some());
        assert!(b.level_up.is_some());
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
