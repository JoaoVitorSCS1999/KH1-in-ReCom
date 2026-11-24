//! BBS FM's kid Riku's hair on Re:CoM Riku's head (the third hair option, `riku_hair = bbs`; fork owner, 2026-11-24: "agora
//! quero implementar o cabelo do kid Riku do BBS"). It needs only the 1.5+2.5 collection (BBS FM's `bbs_fourth` archive); the swap
//! itself is [`crate::ddd_hair::with_hair`], as for [`crate::kh2_hair`].
//!
//! Source (notes: the workspace's `knowledge/BBS/bbs-kid-riku-hair-models.md`, Checked (files)): `chara/npc/n02di/00/n02di00.pmo`,
//! a PMO v1 (the format of `OpenKh.Bbs/Pmo.cs`; its skeleton is read by [`crate::pam::parse_skeleton`]), 35 bones, 1392 triangles,
//! positions in bind-pose model space (`s16 / 32768` times the header's scale 1.18934), weights `u8 / 128`. Its atlas is the PMO's
//! texture 0, `n02di00_00`, a 1024 x 1024 uncompressed DDS (the entry's HD asset `-0.dds`): face top left, the hair spray at the
//! top and about 20 hair shards on the right, the body below.
//!
//! Which triangles are hair: the note's colour rule (the triangle's centroid texel is grey-lavender: hue 0.55..0.85 with saturation
//! 0.05..0.5, or grey with saturation 0.05 or less, value 0.12..0.9) on the triangles of texture 0 in the head region (a corner at
//! or above `atama`'s y minus 0.03), then every UV island of the atlas that is mostly hair by that rule is taken whole (the island
//! rule of the DDD hair: it closes the holes the colour rule leaves, e.g. the 20 grey triangles of the crown).
//!
//! Fit (Assumption, the renders are the check): a vertex's bind position `P` (scaled) becomes `H + SCALE * (P - A) + SHIFT` (per axis) where
//! `A` is the kid's `atama` bone and `H` Re:CoM's `head` bone, both at rest. Bones (0.194.1): every vertex follows Re:CoM's `head`
//! rigidly (0.194.0 mapped the kid's `B_kami` / `L_kamiA` / ... to the nearest Re:CoM hair bone, whose clips split the short hair's
//! layers in game).

use crate::ddd_hair::{HairSource, SrcHair, SrcTri, SrcVert, HEAD_BONE};
use crate::math::Aff;
use crate::texture::{decode_dds, Image};
use crate::{bad, Error, Result};

/// The model (kid Riku, the field / NPC one). Checked (files).
pub const MODEL: &str = "chara/npc/n02di/00/n02di00.pmo";
/// The PMO's texture 0 and its HD asset in the archive entry. Checked (files).
pub const ATLAS_TEXTURE: &str = "n02di00_00";
/// See [`ATLAS_TEXTURE`].
pub const ATLAS_ASSET: &str = "-0.dds";
/// The kid's head bone.
pub const HEAD_BONE_BBS: &str = "atama";
/// The scale about the head bone per axis (x, y, z). 0.194.0 had the note's x1.1 on all three; in game the hair sat too high.
/// Height 1.0 keeps the crown at Re:CoM's own hair top; width stays wider (Re:CoM's face is about 1.2 times the kid's, at 1.0
/// its temples show beside the hair), fitted on the renders.
pub const SCALE: [f64; 3] = [1.15, 1.0, 1.1];
/// Offset added after the head alignment, in Re:CoM metres (x, y, z), fitted on the renders (the fringe to the brows).
pub const SHIFT: [f64; 3] = [0.0, -0.008, 0.0];
/// Passes of [`bleed`] on the atlas (pixels of shard colour grown into the black fill).
pub const BLEED: usize = 8;
/// A texel darker than this in every channel is fill (the hair's texels are 80 and up; the fill is 0 with a rim of 16..80
/// around the shards, Checked (files: a histogram of the atlas's hair area)).
pub const BLEED_BELOW: u8 = 72;
/// Corners this far under `atama` (model units) or higher are the head region.
pub const HEAD_MARGIN: f64 = 0.03;
/// The lining under the back of the hair (0.194.2): how far each vertex is pulled toward the head's centre (fraction).
pub const LINING: f64 = 0.08;
/// The head's centre for the lining, from `atama` (model units; x, y, z), the middle of the hair's bounds.
pub const LINING_CENTRE: [f64; 3] = [0.0, 0.08, -0.03];

/// The PMO and its atlas.
pub struct BbsHairSource {
    /// `n02di00.pmo`.
    pub pmo: Vec<u8>,
    /// `-0.dds`.
    pub dds: Vec<u8>,
}

impl BbsHairSource {
    /// From the player's BBS archives (next to the game, as the Terra reads).
    pub fn read(game: &std::path::Path, keys: &kh1_formats::egs::KeyTable) -> Result<BbsHairSource> {
        for a in crate::terra::ARCHIVES {
            let hed = game.join("Image").join("dt").join(format!("{a}.hed"));
            if !hed.is_file() {
                continue;
            }
            let arc = kh1_formats::egs::Archive::open(&hed)?;
            if let Some(e) = arc.find(MODEL) {
                let raw = arc.read_raw_entry(e)?;
                let (pmo, assets) = kh1_formats::egs::decode_entry_full(keys, &raw)?;
                let dds = assets.into_iter().find(|x| x.name == ATLAS_ASSET).ok_or_else(|| Error::Format(format!("BBS: {MODEL} has no {ATLAS_ASSET}")))?.data;
                return Ok(BbsHairSource { pmo, dds });
            }
        }
        bad(format!("BBS: {MODEL} in none of the bbs archives"))
    }
}

/// One vertex: position (model space, scaled), UV, dominant bone index.
#[derive(Clone, Debug)]
pub struct PmoVert {
    /// Bind-pose position times the header scale.
    pub pos: [f64; 3],
    /// Texture coordinates (0..1 over the atlas).
    pub uv: [f64; 2],
    /// Index of the dominant-weight bone in the skeleton (none: the model has no skeleton).
    pub bone: usize,
}

/// One mesh chunk.
#[derive(Clone, Debug)]
pub struct PmoMesh {
    /// Its texture index.
    pub tex: usize,
    /// Its vertices.
    pub verts: Vec<PmoVert>,
    /// Its triangles (indices into `verts`).
    pub tris: Vec<[usize; 3]>,
}

/// A parsed PMO v1.
#[derive(Clone, Debug)]
pub struct Pmo {
    /// Texture names.
    pub textures: Vec<String>,
    /// Mesh chunks.
    pub meshes: Vec<PmoMesh>,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]])).ok_or_else(|| Error::Format("PMO: truncated".into()))
}
fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| Error::Format("PMO: truncated".into()))
}
fn f32_at(b: &[u8], o: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(b, o)?))
}
fn bits(v: u32, s: u32, n: u32) -> u32 {
    (v >> s) & ((1 << n) - 1)
}

/// Reads a PMO v1 (the layout of `OpenKh.Bbs/Pmo.cs`, `ReadMeshData`).
pub fn parse_pmo(b: &[u8]) -> Result<Pmo> {
    if b.get(..4) != Some(b"PMO\0") {
        return bad("PMO: no magic");
    }
    let ntex = *b.get(8).ok_or_else(|| Error::Format("PMO: short".into()))? as usize;
    let skel = u32_at(b, 0xC)? as usize;
    let (m0, m1) = (u32_at(b, 0x10)? as usize, u32_at(b, 0x1C)? as usize);
    let scale = f32_at(b, 0x18)? as f64;
    let mut textures = vec![];
    for i in 0..ntex {
        let o = 0xA0 + i * 0x20 + 4;
        let n = b.get(o..o + 12).ok_or_else(|| Error::Format("PMO: texture name outside".into()))?;
        textures.push(String::from_utf8_lossy(&n[..n.iter().position(|&c| c == 0).unwrap_or(12)]).into_owned());
    }
    let mut meshes = vec![];
    for off in [m0, m1] {
        if off == 0 {
            continue;
        }
        let mut p = off;
        loop {
            let vcnt = u16_at(b, p)? as usize;
            let tex = *b.get(p + 2).ok_or_else(|| Error::Format("PMO: short".into()))? as usize;
            let vsz = *b.get(p + 3).ok_or_else(|| Error::Format("PMO: short".into()))? as usize;
            let fl = u32_at(b, p + 4)?;
            let nstrips = *b.get(p + 9).ok_or_else(|| Error::Format("PMO: short".into()))? as usize;
            p += 12;
            if vcnt == 0 {
                break;
            }
            let mut bt = [0u8; 8];
            if skel != 0 {
                bt.copy_from_slice(b.get(p..p + 8).ok_or_else(|| Error::Format("PMO: bone table".into()))?);
                p += 8;
            }
            if bits(fl, 24, 1) != 0 {
                p += 4;
            }
            let mut strips = vec![];
            for i in 0..nstrips {
                strips.push(u16_at(b, p + 2 * i)? as usize);
            }
            p += 2 * nstrips;
            let (tf, cf, nf, pf, wf, nw, prim) = (bits(fl, 0, 2), bits(fl, 2, 3), bits(fl, 5, 2), bits(fl, 7, 2), bits(fl, 9, 2), bits(fl, 14, 3) as usize, bits(fl, 28, 4));
            let mut verts = Vec::with_capacity(vcnt);
            for v in 0..vcnt {
                let s = p + v * vsz;
                let mut q = s;
                let mut w = vec![];
                if skel != 0 && wf != 0 {
                    for _ in 0..=nw {
                        match wf {
                            1 => {
                                w.push(*b.get(q).ok_or_else(|| Error::Format("PMO: vertex".into()))? as f64 / 128.0);
                                q += 1;
                            }
                            2 => {
                                w.push(u16_at(b, q)? as f64 / 32768.0);
                                q += 2;
                            }
                            _ => {
                                w.push(f32_at(b, q)? as f64);
                                q += 4;
                            }
                        }
                    }
                }
                let al = |q: usize, a: usize| q + ((a - ((q - s) & (a - 1))) & (a - 1));
                let mut uv = [0.0; 2];
                match tf {
                    1 => {
                        uv = [*b.get(q).ok_or_else(|| Error::Format("PMO: vertex".into()))? as f64 / 128.0, *b.get(q + 1).ok_or_else(|| Error::Format("PMO: vertex".into()))? as f64 / 128.0];
                        q += 2;
                    }
                    2 => {
                        q = al(q, 2);
                        uv = [u16_at(b, q)? as f64 / 32768.0, u16_at(b, q + 2)? as f64 / 32768.0];
                        q += 4;
                    }
                    3 => {
                        q = al(q, 4);
                        uv = [f32_at(b, q)? as f64, f32_at(b, q + 4)? as f64];
                        q += 8;
                    }
                    _ => {}
                }
                match cf {
                    7 => {
                        q = al(q, 4) + 4;
                    }
                    4..=6 => {
                        q = al(q, 2) + 2;
                    }
                    _ => {}
                }
                if nf != 0 {
                    match nf {
                        1 => q += 3,
                        2 => q = al(q, 2) + 6,
                        _ => q = al(q, 4) + 12,
                    }
                }
                let pos = match pf {
                    1 => {
                        let g = |k: usize| -> Result<f64> { Ok(*b.get(q + k).ok_or_else(|| Error::Format("PMO: vertex".into()))? as i8 as f64 / 128.0) };
                        [g(0)?, g(1)?, g(2)?]
                    }
                    2 => {
                        q = al(q, 2);
                        let g = |k: usize| -> Result<f64> { Ok(u16_at(b, q + 2 * k)? as i16 as f64 / 32768.0) };
                        [g(0)?, g(1)?, g(2)?]
                    }
                    _ => {
                        q = al(q, 4);
                        [f32_at(b, q)? as f64, f32_at(b, q + 4)? as f64, f32_at(b, q + 8)? as f64]
                    }
                };
                let dom = w.iter().enumerate().fold((0usize, f64::MIN), |a, (k, &x)| if x > a.1 { (k, x) } else { a }).0;
                verts.push(PmoVert { pos: pos.map(|x| x * scale), uv, bone: bt[dom.min(7)] as usize });
            }
            p += vcnt * vsz;
            p += p % 4;
            let n = verts.len();
            let mut tris = vec![];
            if !strips.is_empty() {
                let mut base = 0;
                for c in &strips {
                    for s in 0..c.saturating_sub(2) {
                        tris.push(if s % 2 == 0 { [base + s, base + s + 1, base + s + 2] } else { [base + s, base + s + 2, base + s + 1] });
                    }
                    base += c;
                }
            } else if prim == 4 {
                for s in 0..n.saturating_sub(2) {
                    tris.push(if s % 2 == 0 { [s, s + 1, s + 2] } else { [s + 1, s, s + 2] });
                }
            } else if prim == 3 {
                let mut i = 0;
                while i + 2 < n {
                    tris.push([i, i + 1, i + 2]);
                    i += 3;
                }
            }
            meshes.push(PmoMesh { tex, verts, tris });
        }
    }
    Ok(Pmo { textures, meshes })
}

fn hsv(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let d = mx - mn;
    let s = if mx == 0.0 { 0.0 } else { d / mx };
    let h = if d == 0.0 {
        0.0
    } else if mx == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if mx == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h, s, mx)
}

/// The note's colour rule for one texel.
pub fn is_hair_texel(px: [u8; 4]) -> bool {
    let (h, s, v) = hsv(px[0] as f64 / 255.0, px[1] as f64 / 255.0, px[2] as f64 / 255.0);
    (((0.55..=0.85).contains(&h) && 0.05 < s && s < 0.5) || s <= 0.05) && 0.12 < v && v < 0.9
}

fn texel(img: &Image, uv: [f64; 2]) -> [u8; 4] {
    let (u, v) = (uv[0].rem_euclid(1.0), uv[1].rem_euclid(1.0));
    img.px[((v * img.h as f64) as usize % img.h) * img.w + ((u * img.w as f64) as usize % img.w)]
}

/// The hair triangles of the head region: `(mesh, triangle)` pairs, by the colour rule then whole UV islands, and how many the
/// colour rule alone gave.
pub fn classify(pmo: &Pmo, skel_atama_y: f64, atlas: &Image) -> (Vec<(usize, usize)>, usize) {
    let tex0 = pmo.textures.iter().position(|n| n == ATLAS_TEXTURE);
    let mut cand: Vec<(usize, usize)> = vec![];
    for (mi, m) in pmo.meshes.iter().enumerate() {
        if Some(m.tex) != tex0 {
            continue;
        }
        for (ti, t) in m.tris.iter().enumerate() {
            let p = t.map(|i| m.verts[i].pos);
            if p[0] == p[1] || p[1] == p[2] || p[0] == p[2] {
                continue;
            }
            if p.iter().map(|q| q[1]).fold(f64::MIN, f64::max) < skel_atama_y - HEAD_MARGIN {
                continue;
            }
            cand.push((mi, ti));
        }
    }
    let by_colour: Vec<bool> = cand
        .iter()
        .map(|&(mi, ti)| {
            let m = &pmo.meshes[mi];
            let uv = {
                let t = m.tris[ti];
                let u: [f64; 2] = [0, 1].map(|k| (m.verts[t[0]].uv[k] + m.verts[t[1]].uv[k] + m.verts[t[2]].uv[k]) / 3.0);
                u
            };
            is_hair_texel(texel(atlas, uv))
        })
        .collect();
    let q = |x: f64| (x * 4096.0).round() as i32;
    let corners: Vec<[(i32, i32); 3]> = cand.iter().map(|&(mi, ti)| pmo.meshes[mi].tris[ti].map(|i| (q(pmo.meshes[mi].verts[i].uv[0]), q(pmo.meshes[mi].verts[i].uv[1])))).collect();
    let (isl, n) = crate::ddd_hair::islands(&corners);
    let (mut hair, mut all) = (vec![0usize; n], vec![0usize; n]);
    for (k, &i) in isl.iter().enumerate() {
        all[i] += 1;
        hair[i] += by_colour[k] as usize;
    }
    let out = cand.iter().enumerate().filter(|(k, _)| hair[isl[*k]] * 2 > all[isl[*k]]).map(|(_, &c)| c).collect();
    (out, by_colour.iter().filter(|&&x| x).count())
}

impl HairSource for BbsHairSource {
    fn hair(&self, names: &[String], world: &[Aff]) -> Result<SrcHair> {
        let pmo = parse_pmo(&self.pmo)?;
        let skel = crate::pam::parse_skeleton(&self.pmo)?;
        let kw = skel.rest_world();
        let atama = skel.find(HEAD_BONE_BBS).ok_or_else(|| Error::Format("BBS: no atama bone".into()))?;
        let img = decode_dds(&self.dds)?;
        let head = names.iter().position(|n| n == HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let (a, h) = (kw[atama].t, world[head].t);
        let frame = |p: [f64; 3]| [0, 1, 2].map(|k| h[k] + SCALE[k] * (p[k] - a[k]) + SHIFT[k]);
        let (hair, _) = classify(&pmo, a[1], &img);
        // 0.194.1: every corner on `head` (rigid). Re:CoM's hair bones are keyed for its long hair (idle: `l_hair4` / `r_hair4` 18 to
        // 19 deg off rest against the head; run: `b_hair2` 69 deg); on the kid's short closed shell they split the layers (side tufts
        // off the face, the nape's under-layer spread sideways, seen in game on 0.194.0).
        let to_recom: Vec<usize> = vec![head; skel.bones.len()];
        let q = |x: f64| (x * 4096.0).round() as i32;
        let corners: Vec<[(i32, i32); 3]> = hair.iter().map(|&(mi, ti)| pmo.meshes[mi].tris[ti].map(|i| (q(pmo.meshes[mi].verts[i].uv[0]), q(pmo.meshes[mi].verts[i].uv[1])))).collect();
        let (isl, n) = crate::ddd_hair::islands(&corners);
        let mut tris = Vec::with_capacity(hair.len());
        for (k, &(mi, ti)) in hair.iter().enumerate() {
            let m = &pmo.meshes[mi];
            let mk = |i: usize| -> Result<SrcVert> {
                let v = &m.verts[i];
                let bone = to_recom.get(v.bone).copied().ok_or_else(|| Error::Format(format!("BBS: bone {} out of range", v.bone)))?;
                Ok(SrcVert { pos: frame(v.pos), uv: v.uv, bone: names[bone].clone() })
            };
            let t = m.tris[ti];
            tris.push(SrcTri { v: [mk(t[0])?, mk(t[1])?, mk(t[2])?], island: isl[k], cont: false });
        }
        // 0.194.2: a lining under the back of the hair. Re:CoM's face is a one-sided mask (its back and top were his hair, now
        // removed), so from behind nothing is drawn under the kid's thin shards and a crack between them showed the background
        // (fork owner's screenshot, 0.194.1). The hair triangles whose centroid is behind `atama` are written again, pulled
        // [`LINING`] of the way toward the head's centre, same UVs and island (no new atlas space).
        let c = [a[0], a[1] + LINING_CENTRE[1], a[2] + LINING_CENTRE[2]];
        for (k, &(mi, ti)) in hair.iter().enumerate() {
            let m = &pmo.meshes[mi];
            let t = m.tris[ti];
            if (0..3).map(|j| m.verts[t[j]].pos[2]).sum::<f64>() / 3.0 >= a[2] {
                continue;
            }
            let mk = |i: usize| {
                let p = m.verts[i].pos;
                SrcVert { pos: frame([0, 1, 2].map(|j| c[j] + (1.0 - LINING) * (p[j] - c[j]))), uv: m.verts[i].uv, bone: names[head].clone() }
            };
            tris.push(SrcTri { v: [mk(t[0]), mk(t[1]), mk(t[2])], island: isl[k], cont: false });
        }
        // 0.194.3: one-sided, as BBS draws it (two-sided, the shell's inner layer, painted dark and facing in, showed from outside as
        // flat dark triangles, fork owner's screenshots); the black fill between the atlas's shards is filled with the shards'
        // colour (strand tips at an island's edge took it in when the islands were scaled into Re:CoM's atlas).
        Ok(SrcHair { tris, islands: n, skin: 0, atlas: bleed(&img, BLEED), one_sided: true })
    }
}

/// The atlas with its black fill and its dark anti-aliased rim (every channel under [`BLEED_BELOW`]) replaced, `passes` times, by the mean of its non-fill 8 neighbours.
pub fn bleed(img: &Image, passes: usize) -> Image {
    let mut out = img.clone();
    let fill = |p: [u8; 4]| p[0].max(p[1]).max(p[2]) < BLEED_BELOW;
    let mut is_fill: Vec<bool> = out.px.iter().map(|&p| fill(p)).collect();
    for _ in 0..passes {
        let mut next = out.px.clone();
        let mut now = is_fill.clone();
        for y in 0..out.h {
            for x in 0..out.w {
                let i = y * out.w + x;
                if !is_fill[i] {
                    continue;
                }
                let (mut sum, mut n) = ([0u32; 3], 0u32);
                for dy in -1i64..=1 {
                    for dx in -1i64..=1 {
                        let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                        if (dx, dy) == (0, 0) || nx < 0 || ny < 0 || nx >= out.w as i64 || ny >= out.h as i64 {
                            continue;
                        }
                        let j = ny as usize * out.w + nx as usize;
                        if !is_fill[j] {
                            for k in 0..3 {
                                sum[k] += out.px[j][k] as u32;
                            }
                            n += 1;
                        }
                    }
                }
                if n > 0 {
                    next[i] = [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, out.px[i][3]];
                    now[i] = false;
                }
            }
        }
        out.px = next;
        is_fill = now;
    }
    out
}

/// The lining's triangles: the hair triangles whose centroid is behind `atama` (`a` = its rest position).
pub fn lining_count(pmo: &Pmo, hair: &[(usize, usize)], a: [f64; 3]) -> usize {
    hair.iter().filter(|&&(mi, ti)| { let m = &pmo.meshes[mi]; let t = m.tris[ti]; (0..3).map(|j| m.verts[t[j]].pos[2]).sum::<f64>() / 3.0 < a[2] }).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ddd_hair::{tests as dt, with_hair, HairTarget};
    use crate::install;
    use crate::mdl::{split_parts, MdlPart, RecomSkeleton};

    /// The new atlas's name.
    pub const TARGET: HairTarget = crate::ddd_hair::RIKU_0101_BBS;

    /// 0.194.0 (owner's install, `RECOM_GAME`; the 1.5+2.5 collection only): the kid's hair classified (the note: 392 triangles by
    /// colour), swapped in, the debug renders written to the workspace's `local/assets/recom/riku-hair/out/bbs/`.
    #[test]
    fn swaps_rikus_hair_for_the_kid_s() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let src = BbsHairSource::read(&g, &keys).unwrap();
        let pmo = parse_pmo(&src.pmo).unwrap();
        assert_eq!(pmo.textures[0], ATLAS_TEXTURE);
        let sk = crate::pam::parse_skeleton(&src.pmo).unwrap();
        let atama = sk.find(HEAD_BONE_BBS).unwrap();
        let img = decode_dds(&src.dds).unwrap();
        assert_eq!((img.w, img.h), (1024, 1024));
        let (hair, by_colour) = classify(&pmo, sk.rest_world()[atama].t[1], &img);
        eprintln!("kid hair: {} triangles by islands, {by_colour} by colour; atama {:?}", hair.len(), sk.rest_world()[atama].t);
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let swap = with_hair(&mdl, &rtm, &hd, &TARGET, &src).unwrap();
        eprintln!("{:?}", swap.report);
        let sk_n = lining_count(&pmo, &hair, sk.rest_world()[atama].t);
        eprintln!("lining: {sk_n} triangles");
        assert_eq!(swap.report.ddd_hair, hair.len() + sk_n);
        let again = with_hair(&mdl, &rtm, &hd, &TARGET, &src).unwrap();
        assert!(swap.mdl == again.mdl && swap.dds == again.dds, "deterministic");
        crate::mdl::Mdl::parse(&swap.mdl).unwrap();
        let parts = split_parts(&swap.mdl).unwrap();
        let p3 = MdlPart::parse_bytes(&parts[3]).unwrap();
        assert!(p3.texture_names().contains(&TARGET.new_texture.to_string()));
        let rsk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let img = decode_dds(&swap.dds).unwrap();
        let out = dt::out_dir().join("bbs");
        std::fs::create_dir_all(&out).unwrap();
        dt::render(&p3, &rsk.rest_world(), &img, &out);
        std::fs::write(out.join("rk_hair_bbs_atlas.png"), dt::png(img.w, img.h, &img.px.iter().flat_map(|p| [p[0], p[1], p[2]]).collect::<Vec<_>>())).unwrap();
        // the hair's bones
        let names: Vec<String> = rsk.bones.iter().map(|b| b.name.clone()).collect();
        let h = src.hair(&names, &rsk.rest_world()).unwrap();
        let mut by: std::collections::BTreeMap<String, usize> = Default::default();
        for t in &h.tris {
            for v in &t.v {
                *by.entry(v.bone.clone()).or_default() += 1;
            }
        }
        eprintln!("corner bones {by:?}");
        // debug: hair (first) and lining (last `sk_n`) triangles plus Re:CoM's kept head as a text dump for outside renders
        {
            let mut txt = String::new();
            for (k, t) in h.tris.iter().enumerate() {
                let g = if k >= h.tris.len() - sk_n { 's' } else { 'h' };
                txt += &format!("{g} {}\n", t.v.iter().flat_map(|v| v.pos).map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "));
            }
            let w = rsk.rest_world();
            for t in dt::collect(&p3) {
                if !t.v.iter().all(|v| names[v.bone()] == "head" || names[v.bone()].starts_with("neck")) { continue; }
                txt += &format!("r {} {} {}\n", t.v.iter().flat_map(|v| w[v.bone()].point(v.pos().map(|x| x as f64))).map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "), t.tex, t.v.iter().flat_map(|v| v.uv().map_or([0.0f32; 2], |u| [u[0], u[1]])).map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "));
            }
            std::fs::write(out.join("tris.txt"), txt).unwrap();
        }
        let (mut lo, mut hi) = ([9.0f64; 3], [-9.0f64; 3]);
        for t in &h.tris {
            for v in &t.v {
                for k in 0..3 {
                    lo[k] = lo[k].min(v.pos[k]);
                    hi[k] = hi[k].max(v.pos[k]);
                }
            }
        }
        eprintln!("hair bounds {lo:?} {hi:?}");
        // posed renders with the torso (the same clips as the KH2 hair's): idle, run, attack, jump
        let ann = crate::ann::Ann::parse(&install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let base = out.join("pose");
        for (clip, frames) in [(1usize, vec![0.0, 50.0]), (3, vec![0.0, 6.0, 11.0, 17.0]), (14, vec![5.0]), (37, vec![10.0]), (41, vec![16.0])] {
            let Some(c) = ann.slots[clip].clip.as_ref() else { continue };
            for f in frames {
                let w = crate::kh2_hair::tests::posed_world(&rsk, c, f);
                let d = base.join(format!("c{clip}_f{f}"));
                std::fs::create_dir_all(&d).unwrap();
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 0, &d.join("body_side.png"));
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 1, &d.join("body_back.png"));
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 2, &d.join("body_front.png"));
            }
        }
    }

    /// Debug (0.194.0): the classified triangles drawn straight from the source atlas (no repacking), two-sided, front / back / top,
    /// to tell a classification or UV problem from an atlas-packing one.
    #[test]
    fn source_renders() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let keys = install::keys(&std::path::PathBuf::from(&g)).unwrap();
        let src = BbsHairSource::read(std::path::Path::new(&g), &keys).unwrap();
        let (arc, mdl) = { let arc = install::recom_archive(std::path::Path::new(&g)).unwrap(); let m = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap(); (arc, m) };
        let _ = arc;
        let rsk = RecomSkeleton::parse(&mdl).unwrap();
        let names: Vec<String> = rsk.bones.iter().map(|b| b.name.clone()).collect();
        let h = src.hair(&names, &rsk.rest_world()).unwrap();
        let out = dt::out_dir().join("bbs");
        std::fs::create_dir_all(&out).unwrap();
        for (name, hx, hs, vy, dx, ds) in [("front", 0usize, 1.0f64, 1usize, 2usize, 1.0f64), ("back", 0, -1.0, 1, 2, -1.0), ("side", 2, 1.0, 1, 0, -1.0)] {
            let size = 800usize;
            let mut tris: Vec<([[f64; 3]; 3], [[f64; 2]; 3])> = h.tris.iter().map(|t| (t.v.clone().map(|v| v.pos), t.v.clone().map(|v| v.uv))).collect();
            tris.sort_by(|a, b| { let d = |t: &([[f64; 3]; 3], _)| t.0.iter().map(|p| p[dx] * ds).sum::<f64>(); d(a).total_cmp(&d(b)) });
            let (mut lo, mut hi) = ([9.0f64; 2], [-9.0f64; 2]);
            for (p, _) in &tris { for q in p { let t = [q[hx] * hs, q[vy]]; for k in 0..2 { lo[k] = lo[k].min(t[k]); hi[k] = hi[k].max(t[k]); } } }
            let sc = 700.0 / (hi[0] - lo[0]).max(hi[1] - lo[1]);
            let mut rgb = vec![30u8; size * size * 3];
            for (p, uv) in &tris {
                let s: Vec<[f64; 2]> = p.iter().map(|q| [50.0 + (q[hx] * hs - lo[0]) * sc, 750.0 - (q[vy] - lo[1]) * sc]).collect();
                let det = (s[1][1] - s[2][1]) * (s[0][0] - s[2][0]) + (s[2][0] - s[1][0]) * (s[0][1] - s[2][1]);
                if det.abs() < 1e-9 { continue; }
                let (x0, x1) = (s.iter().map(|q| q[0]).fold(f64::MAX, f64::min).max(0.0) as usize, (s.iter().map(|q| q[0]).fold(f64::MIN, f64::max) as usize + 1).min(size - 1));
                let (y0, y1) = (s.iter().map(|q| q[1]).fold(f64::MAX, f64::min).max(0.0) as usize, (s.iter().map(|q| q[1]).fold(f64::MIN, f64::max) as usize + 1).min(size - 1));
                for y in y0..=y1 { for x in x0..=x1 {
                    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                    let l0 = ((s[1][1] - s[2][1]) * (px - s[2][0]) + (s[2][0] - s[1][0]) * (py - s[2][1])) / det;
                    let l1 = ((s[2][1] - s[0][1]) * (px - s[2][0]) + (s[0][0] - s[2][0]) * (py - s[2][1])) / det;
                    let l2 = 1.0 - l0 - l1;
                    if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                    let u = l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0]; let v = l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1];
                    let c = texel(&h.atlas, [u, v]);
                    rgb[3 * (y * size + x)..3 * (y * size + x) + 3].copy_from_slice(&c[..3]);
                } }
            }
            std::fs::write(out.join(format!("source_{name}.png")), dt::png(size, size, &rgb)).unwrap();
        }
    }
}
