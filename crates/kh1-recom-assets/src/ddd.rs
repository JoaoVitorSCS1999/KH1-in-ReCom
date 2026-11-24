//! KH Dream Drop Distance's (Steam HD 2.8) playable Riku, read from the player's own install for Riku's cast
//! motions (0.185.0; fork owner, 2026-11-23: "as animações faltante do RIku vc vai pegar do KH DDD", "a pega todas as
//! magias"; the table they chose: "a", every spell on DDD's own cast pair).
//!
//! Formats (the workspace's `knowledge/DDD/ddd-riku-animations.md`, Checked (files) on the owner's install against
//! OpenKh@41ac7b5 `OpenKh.Bbs/Pam.cs` and `OpenKh.Ddd/PmoV4.cs`):
//! - the archives `Image/dt/kh3d_*.hed / .pkg` are the 1.5+2.5 layout and cipher, with the same key table (the DDD
//!   exe's matches): `kh1_formats::egs` reads them with the keys the step already has;
//! - **PAM** (animation pack, version 2): `"PAM\0"`, u32 count, then 16-byte entries from 0x10 (`u32 offset`, `u32`
//!   name offset into a string table at the file's end, 8 zero bytes; offset 0 = an empty slot); each clip: u16 flag,
//!   u8 fps (30), u8 blend-in, u16 loop, u8 bone count, u8 pad, u16 frames, u16 return frame; u16 channel mask per bone
//!   (bits 0..8: tx ty tz rx ry rz sx sy sz); per present channel `f32 max, f32 min`, a key count (u8, u16 when frames
//!   > 255), then the keys (frame id u8 / u16, absent when the count equals the frames; u16 value) unless the count is 1
//!   (a constant: max); value = min + (max − min) × q / 65535, linear between keys;
//! - a bone's local transform: S · Rx · Ry · Rz + T, row vectors, radians (Re:CoM's clips' own convention); a missing
//!   channel is 0 (translation, rotation) or 1 (scale);
//! - **PMO** v4 skeleton: `"PMO\0"`, its skeleton offset at +0xC; at it `"BON\0"`, u32, u16 bone count, then 160-byte
//!   bones from +0x10: u16 index, u16 pad, u16 parent (0xFFFF none), u16 pad, u16 skinned index, u16 pad, u32 pad,
//!   char name[16], f32 local[16] (row-major, row vectors), f32 inverse world[16].
//!
//! Nothing of the game is in this file: the step reads the player's files.

use crate::ann::Clip;
use crate::math::{m3, rot_xyz, Aff, M3};
use crate::mdl::{RBone, RecomSkeleton};
use crate::{bad, Result};
use std::path::{Path, PathBuf};

/// The 2.8 collection's folder name next to 1.5+2.5's in a Steam library.
pub const FOLDER: &str = "KINGDOM HEARTS HD 2.8 Final Chapter Prologue";
/// DDD's archives.
pub const ARCHIVES: [&str; 4] = ["kh3d_first", "kh3d_second", "kh3d_third", "kh3d_fourth"];
/// DDD Riku's main animation pack (every cast clip is in it).
pub const RIKU_PAM: &str = "chara/pc/p_ex02/anm/rt/bin/p_ex02_000.pam";
/// DDD Riku's model (its skeleton).
pub const RIKU_PMO: &str = "chara/pc/p_ex02/mig/0/bin/p_ex020.pmo";

/// The 2.8 install next to the 1.5+2.5 game folder `game`, if there.
pub fn install_next_to(game: &Path) -> Option<PathBuf> {
    let d = game.parent()?.join(FOLDER);
    d.join("Image").join("dt").join("kh3d_first.hed").is_file().then_some(d)
}

/// Reads `name` from whichever DDD archive holds it.
pub fn read(ddd: &Path, keys: &kh1_formats::egs::KeyTable, name: &str) -> Result<Vec<u8>> {
    for a in ARCHIVES {
        let hed = ddd.join("Image").join("dt").join(format!("{a}.hed"));
        if !hed.is_file() {
            continue;
        }
        let arc = kh1_formats::egs::Archive::open(&hed)?;
        if arc.find(name).is_some() {
            return Ok(arc.read(keys, name)?);
        }
    }
    bad(format!("DDD: {name} in none of the kh3d archives"))
}

/// One animation channel.
#[derive(Clone, Debug, PartialEq)]
pub struct Channel {
    max: f32,
    min: f32,
    /// (frame, quantized value); empty: a constant (`max`).
    keys: Vec<(u16, u16)>,
}

impl Channel {
    fn dq(&self, q: u16) -> f64 {
        self.min as f64 + (self.max as f64 - self.min as f64) * q as f64 / 65535.0
    }

    /// The value at frame `t` (fractional: linear between keys).
    pub fn value(&self, t: f64) -> f64 {
        let Some(first) = self.keys.first() else { return self.max as f64 };
        if t <= first.0 as f64 {
            return self.dq(first.1);
        }
        for w in self.keys.windows(2) {
            let (f0, f1) = (w[0].0 as f64, w[1].0 as f64);
            if t <= f1 {
                let u = if f1 > f0 { (t - f0) / (f1 - f0) } else { 0.0 };
                return self.dq(w[0].1) + (self.dq(w[1].1) - self.dq(w[0].1)) * u;
            }
        }
        self.dq(self.keys.last().unwrap().1)
    }
}

/// One animation of a PAM pack.
#[derive(Clone, Debug, PartialEq)]
pub struct PamClip {
    /// Its name (DDD's are 3 digits, `"315"`).
    pub name: String,
    /// Its frame rate (30 in every DDD pack read).
    pub fps: u8,
    /// Its frame count.
    pub frames: u16,
    /// Per bone, the nine channels (tx ty tz rx ry rz sx sy sz), None when absent.
    pub bones: Vec<[Option<Channel>; 9]>,
}

impl PamClip {
    /// A bone's local (translation, rotation radians, scale) at frame `t`.
    pub fn local(&self, bone: usize, t: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let ch = |c: usize, d: f64| self.bones.get(bone).and_then(|b| b[c].as_ref()).map_or(d, |x| x.value(t));
        ([ch(0, 0.0), ch(1, 0.0), ch(2, 0.0)], [ch(3, 0.0), ch(4, 0.0), ch(5, 0.0)], [ch(6, 1.0), ch(7, 1.0), ch(8, 1.0)])
    }
}

fn u8_at(b: &[u8], o: usize) -> Result<u8> {
    b.get(o).copied().ok_or_else(|| crate::Error::Format("PAM: outside the file".into()))
}
fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(b.get(o..o + 2).ok_or_else(|| crate::Error::Format("PAM: outside the file".into()))?.try_into().unwrap()))
}
fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(o..o + 4).ok_or_else(|| crate::Error::Format("PAM/PMO: outside the file".into()))?.try_into().unwrap()))
}
fn f32_at(b: &[u8], o: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(b, o)?))
}

/// Parses a PAM pack's clips (empty slots skipped).
pub fn parse_pam(b: &[u8]) -> Result<Vec<PamClip>> {
    if b.get(..4) != Some(b"PAM\0") {
        return bad("PAM: no magic");
    }
    let count = u32_at(b, 4)? as usize;
    let mut out = Vec::new();
    for i in 0..count {
        let e = 0x10 + i * 16;
        let off = u32_at(b, e)? as usize;
        if off == 0 {
            continue;
        }
        let no = u32_at(b, e + 4)? as usize;
        let end = b.get(no..).and_then(|s| s.iter().position(|&c| c == 0)).ok_or_else(|| crate::Error::Format("PAM: name outside".into()))?;
        let name = String::from_utf8_lossy(&b[no..no + end]).into_owned();
        let fps = u8_at(b, off + 2)?;
        let nb = u8_at(b, off + 6)? as usize;
        let frames = u16_at(b, off + 8)?;
        let mut p = off + 12;
        let mut masks = Vec::with_capacity(nb);
        for _ in 0..nb {
            masks.push(u16_at(b, p)?);
            p += 2;
        }
        let mut bones = Vec::with_capacity(nb);
        for mask in masks {
            let mut chans: [Option<Channel>; 9] = Default::default();
            for (c, slot) in chans.iter_mut().enumerate() {
                if mask >> c & 1 == 0 {
                    continue;
                }
                let (max, min) = (f32_at(b, p)?, f32_at(b, p + 4)?);
                p += 8;
                let kc = if frames > 255 {
                    p += 2;
                    u16_at(b, p - 2)? as usize
                } else {
                    p += 1;
                    u8_at(b, p - 1)? as usize
                };
                let mut keys = Vec::new();
                if kc != 1 {
                    for z in 0..kc {
                        let fid = if kc == frames as usize {
                            z as u16
                        } else if frames > 255 {
                            p += 2;
                            u16_at(b, p - 2)?
                        } else {
                            p += 1;
                            u8_at(b, p - 1)? as u16
                        };
                        keys.push((fid, u16_at(b, p)?));
                        p += 2;
                    }
                }
                *slot = Some(Channel { max, min, keys });
            }
            bones.push(chans);
        }
        out.push(PamClip { name, fps, frames, bones });
    }
    Ok(out)
}

/// A PMO v4 model's skeleton, as a Re:CoM-style skeleton (its rest local matrices).
pub fn parse_skeleton(b: &[u8]) -> Result<RecomSkeleton> {
    if b.get(..4) != Some(b"PMO\0") {
        return bad("PMO: no magic");
    }
    let sk = u32_at(b, 0xC)? as usize;
    if b.get(sk..sk + 4) != Some(b"BON\0") {
        return bad("PMO: no skeleton");
    }
    let n = u16_at(b, sk + 8)? as usize;
    let mut bones = Vec::with_capacity(n);
    for i in 0..n {
        let o = sk + 16 + i * 160;
        let parent = u16_at(b, o + 4)?;
        let name = b.get(o + 16..o + 32).ok_or_else(|| crate::Error::Format("PMO: bone outside".into()))?;
        let name = String::from_utf8_lossy(&name[..name.iter().position(|&c| c == 0).unwrap_or(16)]).into_owned();
        let m = |k: usize| -> Result<f64> { Ok(f32_at(b, o + 32 + 4 * k)? as f64) };
        let r: M3 = [[m(0)?, m(1)?, m(2)?], [m(4)?, m(5)?, m(6)?], [m(8)?, m(9)?, m(10)?]];
        let t = [m(12)?, m(13)?, m(14)?];
        bones.push(RBone { name, parent: if parent == 0xFFFF { -1 } else { parent as i16 }, local: Aff { r, t } });
    }
    Ok(RecomSkeleton { bones })
}

/// A PAM clip as a Re:CoM clip on its own (DDD) skeleton, at its own rate: one row per frame (translation, rotation,
/// scale 1: DDD's 0.95 keyed on the head and hair is not carried over).
pub fn to_clip(c: &PamClip, name: &str) -> Clip {
    let frames = c.frames.max(1) as usize;
    let rows: Vec<Vec<[f64; 9]>> = (0..frames)
        .map(|f| {
            (0..c.bones.len())
                .map(|b| {
                    let (t, r, _) = c.local(b, f as f64);
                    [t[0], t[1], t[2], r[0], r[1], r[2], 1.0, 1.0, 1.0]
                })
                .collect()
        })
        .collect();
    // The fps is 60 / (d + 1) with d in `flags` bits 2..4 (`crate::ann`).
    let d = (60 / c.fps.max(1) as u32).saturating_sub(1).min(7) as u8;
    let template = Clip {
        name: crate::ann::name16(name),
        bones: c.bones.len() as i16,
        frames: frames as i16,
        loop_start: 0,
        flags: d << 2,
        has_blend: 2,
        blend: 8,
        blend_pad: [0; 2],
        mask_pad: Vec::new(),
        channels: Vec::new(),
        trailer: vec![0; 4],
    };
    crate::riku_skin::encode_rows(&template, &rows)
}

/// Re:CoM Riku's bone → DDD Riku's (the workspace's DDD note § 7, Checked (files): the two rest skeletons coincide for
/// the legs, hips, arms and head; the spine, neck, weapon and hair by role). Riku's bones with no DDD counterpart
/// (fingers, belts, cuffs, twist bones, the Soul Eater chain under `wep0`, `neck1` / `neck2`, `chest2`) take his
/// battle stance under their parent.
pub const RIKU_FROM_DDD: &[(&str, &str)] = &[
    ("Root", "Root"),
    ("ex_1010_sk", "center"),
    ("hip", "koshi"),
    ("r_femur", "R_momo"),
    ("r_tibia", "R_sune"),
    ("r_foot", "R_ashi1"),
    ("r_toes", "R_ashi2"),
    ("l_femur", "L_momo"),
    ("l_tibia", "L_sune"),
    ("l_foot", "L_ashi1"),
    ("l_toes", "L_ashi2"),
    ("waist", "hara"),
    ("chest1", "mune"),
    ("neck3", "kubi"),
    ("head", "atama"),
    ("r_collar", "R_kata"),
    ("r_uparm", "R_ude1"),
    ("r_foarm", "R_ude2"),
    ("r_hand", "R_te"),
    ("l_collar", "L_kata"),
    ("l_uparm", "L_ude1"),
    ("l_foarm", "L_ude2"),
    ("l_hand", "L_te"),
    ("wep0", "R_buki"),
    ("f_hair", "F_kami"),
    ("r_hair4", "R_kamiB"),
    ("l_hair4", "L_kamiB"),
];

/// The bones whose local translation moves with DDD's (target, source; a target's sources add up): Riku's body
/// `ex_1010_sk` with DDD's `center` and `koshi` (DDD moves the body by `koshi` and `hara` together, Checked (files);
/// Re:CoM by `ex_1010_sk`, the parent of `hip` and `waist`).
pub const CARRIED_TRANSLATION: &[(&str, &str)] = &[("ex_1010_sk", "center"), ("ex_1010_sk", "koshi")];

/// DDD Riku's clips on the ground that the mod uses (the thrust `315`, the raise `317`, the guard `220`): their feet are
/// kept on the floor where they start (`riku_skin::plant_feet`). Checked (files): posed on DDD's skeleton.
pub const GROUND_CLIPS: &[&str] = &["315", "317", "220"];

/// The retarget map (Riku's bones → DDD's indices).
pub fn riku_map(riku: &RecomSkeleton, ddd: &RecomSkeleton) -> Vec<Option<usize>> {
    riku.bones.iter().map(|b| RIKU_FROM_DDD.iter().find(|(r, _)| *r == b.name).and_then(|(_, d)| ddd.find(d))).collect()
}

/// The world rotation of every bone of a DDD clip at frame `t` (tests).
pub fn world_rotations(c: &PamClip, skel: &RecomSkeleton, t: f64) -> Vec<M3> {
    let mut out: Vec<M3> = Vec::with_capacity(skel.bones.len());
    for (i, b) in skel.bones.iter().enumerate() {
        let (_, r, _) = c.local(i, t);
        let l = rot_xyz(r[0], r[1], r[2]);
        out.push(if b.parent < 0 { l } else { m3(&l, &out[b.parent as usize]) });
    }
    out
}

/// Reads `name` with its HD assets (`kh1_formats::egs::decode_entry_full`) from whichever DDD archive holds it.
pub fn read_full(ddd: &Path, keys: &kh1_formats::egs::KeyTable, name: &str) -> Result<(Vec<u8>, Vec<kh1_formats::egs::HdAsset>)> {
    for a in ARCHIVES {
        let hed = ddd.join("Image").join("dt").join(format!("{a}.hed"));
        if !hed.is_file() {
            continue;
        }
        let arc = kh1_formats::egs::Archive::open(&hed)?;
        if let Some(e) = arc.find(name) {
            let raw = arc.read_raw_entry(e)?;
            return Ok(kh1_formats::egs::decode_entry_full(keys, &raw)?);
        }
    }
    bad(format!("DDD: {name} in none of the kh3d archives"))
}

// ---------------------------------------------------------------------------------------------
// PMO v4 meshes.
// ---------------------------------------------------------------------------------------------

/// One PMO v4 vertex (22 bytes). The layout was found on `p_ex020.pmo` itself; OpenKH's `PmoV4.cs` reads the section
/// headers the same way but its vertex code does not describe this (it divides UV by 32768 and skips the weights).
///
/// `+0 s16 u, s16 v`, `+4 u8 rgba[4]`, `+8 s16 x, y, z` (÷ 32768 × the model scale: bind-pose model space, not
/// bone-local), `+14 u8 weight[4]` (sum 128), `+18 u8 slot[4]` (3 × an index into the section's 8-entry bone table).
/// Checked (files: `p_ex020.pmo`; `knowledge/DDD/ddd-riku-hair.md` § 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PmoVertex {
    /// Raw `s16` u and v as stored.
    pub raw_uv: [i16; 2],
    /// Colour.
    pub rgba: [u8; 4],
    /// Bind-pose position, model units (scale applied).
    pub pos: [f32; 3],
    /// The four weights (of 128).
    pub weight: [u8; 4],
    /// The four slots (3 × table index).
    pub slot: [u8; 4],
}

impl PmoVertex {
    /// `(u / 2048, 1 − v / 2048)`: texture coordinates with the origin at the top left, not wrapped (they leave 0..1:
    /// the atlas repeats). Checked (files: the UVs overlay `p_ex020_00.dds`'s face and hair islands exactly).
    pub fn uv(&self) -> [f64; 2] {
        [self.raw_uv[0] as f64 / 2048.0, 1.0 - self.raw_uv[1] as f64 / 2048.0]
    }
}

/// One PMO v4 mesh section.
#[derive(Clone, Debug, PartialEq)]
pub struct PmoSection {
    /// 0 for the first section list, 1 for the second.
    pub list: u8,
    /// Index into [`Pmo::textures`].
    pub texture: u8,
    /// The flags word; `flags >> 28` = primitive (3 triangle list, 4 one strip over the section's vertices).
    pub flags: u32,
    /// The section's attribute word.
    pub attribute: u16,
    /// The section's bone table (indices of [`parse_skeleton`]'s bones, `0xFF` unused).
    pub bone_table: [u8; 8],
    /// The vertices.
    pub verts: Vec<PmoVertex>,
}

impl PmoSection {
    /// The triangles as index triples in drawing (strip) order: a list's `3k..3k+2`, or a strip's `i, i+1, i+2`.
    /// Degenerate ones (strip joins) are included; filter them with [`PmoSection::is_degenerate`].
    pub fn triangles(&self) -> Vec<[usize; 3]> {
        let n = self.verts.len();
        match self.flags >> 28 {
            3 => (0..n / 3).map(|k| [3 * k, 3 * k + 1, 3 * k + 2]).collect(),
            4 => (0..n.saturating_sub(2)).map(|i| [i, i + 1, i + 2]).collect(),
            _ => Vec::new(),
        }
    }

    /// True when two of the triangle's corners sit at the same position (a strip join).
    pub fn is_degenerate(&self, t: [usize; 3]) -> bool {
        let p = |i: usize| self.verts[t[i]].pos.map(f32::to_bits);
        p(0) == p(1) || p(1) == p(2) || p(0) == p(2)
    }

    /// The bone (an index of the PMO bone list) with the largest weight at vertex `v` (a tie: the lower slot).
    /// Checked (files): the head sections' slots all name table entries that exist.
    pub fn dominant_bone(&self, v: usize) -> Option<u8> {
        let vx = &self.verts[v];
        let mut best: Option<(u32, u8)> = None;
        for k in 0..4 {
            let b = *self.bone_table.get(vx.slot[k] as usize / 3)?;
            if b == 0xFF || vx.weight[k] == 0 {
                continue;
            }
            let w: u32 = (0..4).filter(|&j| self.bone_table.get(vx.slot[j] as usize / 3) == Some(&b)).map(|j| vx.weight[j] as u32).sum();
            if best.is_none_or(|(bw, _)| w > bw) {
                best = Some((w, b));
            }
        }
        best.map(|(_, b)| b)
    }
}

/// A PMO v4 model's mesh (not its skeleton: [`parse_skeleton`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Pmo {
    /// The model scale (header `+0x18`).
    pub scale: f32,
    /// Texture names, in the header's order.
    pub textures: Vec<String>,
    /// The sections: first list, then second list, in file order (the vertex stream's order).
    pub sections: Vec<PmoSection>,
}

/// Parses a PMO v4's meshes. Header: `+0x8 u8` texture count, `+0x10 u32` model header offset, `+0x18 f32` scale;
/// texture records of 0x20 bytes from `+0xA0` (`u32`, `char[12]`, 2 × `f32`, pad); the model header = 7 `u32` (section
/// list 0 offset, list 1 offset, vertex total, unknown, count 0, count 1, vertex data offset); a section = 12 bytes
/// (`u16` vertex count, `u8` texture, `u8` vertex size = 0x16, `u32` flags, `u8` group, `u8` strip count, `u16`
/// attribute) + 8 bone-table bytes + `u32` colour + `strip count` × `u16`; the vertices follow in section order.
/// Checked (files: `p_ex020.pmo`, 28 sections, 4603 vertices; also OpenKh@41ac7b5 `PmoV4.cs` for the headers).
pub fn parse_pmo(b: &[u8]) -> Result<Pmo> {
    if b.get(..4) != Some(b"PMO\0") {
        return bad("PMO: no magic");
    }
    let texc = *b.get(8).ok_or_else(|| crate::Error::Format("PMO: short".into()))? as usize;
    let m0 = u32_at(b, 0x10)? as usize;
    let scale = f32_at(b, 0x18)?;
    let mut textures = Vec::new();
    for i in 0..texc {
        let o = 0xA0 + 0x20 * i + 4;
        let n = b.get(o..o + 12).ok_or_else(|| crate::Error::Format("PMO: texture record outside".into()))?;
        textures.push(String::from_utf8_lossy(&n[..n.iter().position(|&c| c == 0).unwrap_or(12)]).into_owned());
    }
    let (l0, l1, c0, c1, vo) = (u32_at(b, m0)? as usize, u32_at(b, m0 + 4)? as usize, u32_at(b, m0 + 16)? as usize, u32_at(b, m0 + 20)? as usize, u32_at(b, m0 + 24)? as usize);
    let mut sections = Vec::new();
    let mut headers = Vec::new();
    for (list, (mut o, c)) in [(l0, c0), (l1, c1)].into_iter().enumerate() {
        for _ in 0..c {
            let vc = u16_at(b, o)? as usize;
            let (tex, vs) = (u8_at(b, o + 2)?, u8_at(b, o + 3)?);
            let flags = u32_at(b, o + 4)?;
            let ts = u8_at(b, o + 9)? as usize;
            let attribute = u16_at(b, o + 10)?;
            if vs != 0x16 {
                return bad(format!("PMO: vertex size {vs:#x}, not 0x16"));
            }
            let bone_table: [u8; 8] = b.get(o + 12..o + 20).ok_or_else(|| crate::Error::Format("PMO: section outside".into()))?.try_into().unwrap();
            o += 12 + 8 + 4 + 2 * ts;
            headers.push((list as u8, tex, flags, attribute, bone_table, vc));
        }
    }
    let mut o = vo;
    for (list, texture, flags, attribute, bone_table, vc) in headers {
        let mut verts = Vec::with_capacity(vc);
        for _ in 0..vc {
            let s = |k: usize| -> Result<i16> { Ok(u16_at(b, o + k)? as i16) };
            let p = |k: usize| -> Result<f32> { Ok(s(k)? as f32 / 32768.0 * scale) };
            let at = |k: usize| -> Result<[u8; 4]> { Ok(b.get(o + k..o + k + 4).ok_or_else(|| crate::Error::Format("PMO: vertex outside".into()))?.try_into().unwrap()) };
            verts.push(PmoVertex { raw_uv: [s(0)?, s(2)?], rgba: at(4)?, pos: [p(8)?, p(10)?, p(12)?], weight: at(14)?, slot: at(18)? });
            o += 22;
        }
        sections.push(PmoSection { list, texture, flags, attribute, bone_table, verts });
    }
    Ok(Pmo { scale, textures, sections })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install;

    /// The owner's install (`RECOM_GAME`, the 2.8 folder next to it): DDD Riku's pack and skeleton read, the cast clips
    /// there with their frames, and clip 000's first frame rebuilds the PMO's rest pose (the format notes).
    #[test]
    fn reads_ddd_rikus_pack_and_skeleton() {
        let Some(g) = std::env::var_os("RECOM_GAME").map(std::path::PathBuf::from) else { return };
        let Some(ddd) = install_next_to(&g) else { return };
        let keys = install::keys(&g).unwrap();
        let pam = parse_pam(&read(&ddd, &keys, RIKU_PAM).unwrap()).unwrap();
        let sk = parse_skeleton(&read(&ddd, &keys, RIKU_PMO).unwrap()).unwrap();
        assert_eq!(sk.bones.len(), 44);
        assert_eq!((sk.bones[1].name.as_str(), sk.bones[28].name.as_str()), ("center", "R_te"));
        let get = |n: &str| pam.iter().find(|c| c.name == n).unwrap();
        assert_eq!([get("315").frames, get("316").frames, get("317").frames, get("318").frames], [36, 35, 35, 35]);
        assert!(pam.iter().all(|c| c.fps == 30));
        let c0 = get("000");
        for (i, b) in sk.bones.iter().enumerate() {
            let (t, r, _) = c0.local(i, 0.0);
            let m = rot_xyz(r[0], r[1], r[2]);
            for (a, e) in m.iter().flatten().zip(b.local.r.iter().flatten()) {
                assert!((a - e).abs() < 2e-3, "bone {} {}: rest rotation", i, b.name);
            }
            for k in 0..3 {
                assert!((t[k] - b.local.t[k]).abs() < 2e-3, "bone {} {}: rest translation", i, b.name);
            }
        }
        eprintln!("DDD read: {} clips, {} bones", pam.len(), sk.bones.len());
        let clip = to_clip(get("317"), "p002_x");
        assert_eq!((clip.frames, clip.fps(), clip.bones), (35, 30, 44));
        let rs = RecomSkeleton::parse(&install::read_recom(&install::recom_archive(&g).unwrap(), &keys, "PLAYER/0101/PL0101.MDL").unwrap()).unwrap();
        let map = riku_map(&rs, &sk);
        assert_eq!(map.iter().filter(|m| m.is_some()).count(), RIKU_FROM_DDD.len(), "every pair found by name");
    }
}
