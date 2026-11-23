//! Riku's Dark Form skin: Dark Riku's model (`PLAYER/0201/PL0201.MDL` / `.RTM`, unchanged) playing
//! Riku's own motions (`PLAYER/0101/PL0101.ANN`) retargeted onto Dark Riku's skeleton (fork owner,
//! 2026-11-22: "pode colocar a skin mas os movimentos ser o normal mesmo").
//!
//! - Skeletons (Checked (files)): Riku `ex_1010_sk`, 102 bones; Dark Riku `ex_1580_sk`, 128 bones.
//!   Every body bone has the same name in both. The Dark model has 43 more bones: the coat tails
//!   `*_skt*`, `l_uparm2` / `r_uparm2`, and `wep1` / `wep2`, which stand for Riku's `k10` / `k11` on
//!   the Soul Eater's chain. Riku's belts / cuffs / pants have no Dark bone. The rest poses differ
//!   by up to 8 cm and the bones' rest axes differ, so channels can't be copied.
//! - Retarget, per frame: Riku's world rotation of a bone, taken relative to his rest world
//!   rotation, is applied to the Dark bone's rest world rotation (`U_b = R_b · R_aᵀ · U_a`). Bone
//!   translations are Dark's rest translations, except `Root`, which is Riku's (root motion), and
//!   bone 1, which is Dark's rest plus Riku's move off his rest (hip height, jumps). Dark-only bones
//!   keep their rest pose relative to their parent.
//! - Riku's `.SES` / `.EET` events name bones by index (`docs/recom-m13-recom-fx.md`): they are
//!   remapped by name to Dark Riku's indices; an event on a bone with no Dark match goes to its
//!   nearest mapped ancestor.
//!
//! The skin is written as new player groups ([`SKIN_GROUPS`]) so Riku's own files stay as they are.

use crate::ann::{Ann, Clip, Slot};
use crate::convert::{encode_channel, locals_for, Target};
use crate::math::{m3, rot_xyz, tr, unscale, M3};
use crate::mdl::RecomSkeleton;
use crate::{bad, Result};

/// (Riku's group, Dark Riku's group, the skin's group). Groups 101 / 111 are Riku's two player
/// groups and 201 / 211 Dark Riku's (`0x1405A6910`, Checked (exe)). 121 and 131 are not in
/// Recom.hed (Checked (files)).
pub const SKIN_GROUPS: [(u16, u16, u16); 2] = [(101, 201, 121), (111, 211, 131)];

/// Dark bone name → Riku bone name, where they differ.
fn alias(dark: &str) -> &str {
    match dark {
        "ex_1580_sk" => "ex_1010_sk",
        "wep1" => "k10",
        "wep2" => "k11",
        x => x,
    }
}

/// For each Dark bone, Riku's bone of the same role.
pub fn bone_map(riku: &RecomSkeleton, dark: &RecomSkeleton) -> Vec<Option<usize>> {
    dark.bones.iter().map(|b| riku.find(alias(&b.name))).collect()
}

/// For each Riku bone, the Dark bone for its events: the same role, else the nearest ancestor
/// that has one (`Root` at worst).
pub fn event_map(riku: &RecomSkeleton, dark: &RecomSkeleton) -> Vec<usize> {
    let inv: Vec<Option<usize>> = riku.bones.iter().map(|b| {
        let n = match b.name.as_str() {
            "ex_1010_sk" => "ex_1580_sk",
            "k10" => "wep1",
            "k11" => "wep2",
            x => x,
        };
        dark.find(n)
    }).collect();
    (0..riku.bones.len())
        .map(|mut i| loop {
            if let Some(j) = inv[i] {
                break j;
            }
            let p = riku.bones[i].parent;
            if p < 0 {
                break 0;
            }
            i = p as usize;
        })
        .collect()
}

fn world_rot(skel: &RecomSkeleton) -> Vec<M3> {
    skel.rest_world().iter().map(|w| unscale(&w.r).0).collect()
}

/// Retargets one of Riku's clips onto Dark Riku's skeleton. Frame count, fps, loop and blend are
/// the clip's own.
pub fn retarget(src: &Clip, riku: &RecomSkeleton, dark: &RecomSkeleton, map: &[Option<usize>]) -> Result<Clip> {
    let na = riku.bones.len();
    if src.bones as usize != na {
        return bad(format!("{}: {} bones, Riku's skeleton has {na}", src.name_str(), src.bones));
    }
    let ra = world_rot(riku);
    let rb = world_rot(dark);
    let nb = dark.bones.len();
    let mut rows: Vec<Vec<[f64; 9]>> = Vec::with_capacity(src.frames as usize);
    for f in 0..src.frames.max(1) as usize {
        // Riku's world rotations and positions at this frame.
        let mut ua: Vec<M3> = Vec::with_capacity(na);
        let mut pa: Vec<[f64; 3]> = Vec::with_capacity(na);
        let mut loc_t: Vec<[f64; 3]> = Vec::with_capacity(na);
        for (i, b) in riku.bones.iter().enumerate() {
            let s = src.sample(i, f as f64);
            let r = rot_xyz(s.r[0], s.r[1], s.r[2]);
            loc_t.push(s.t);
            if b.parent < 0 {
                ua.push(r);
                pa.push(s.t);
            } else {
                let p = b.parent as usize;
                let u = m3(&r, &ua[p]);
                let pos = crate::math::vm(s.t, &ua[p]);
                pa.push([pos[0] + pa[p][0], pos[1] + pa[p][1], pos[2] + pa[p][2]]);
                ua.push(u);
            }
        }
        // Dark's targets.
        let mut tg: Vec<Target> = Vec::with_capacity(nb);
        for (j, b) in dark.bones.iter().enumerate() {
            let u = match map[j] {
                Some(i) => m3(&m3(&rb[j], &tr(&ra[i])), &ua[i]),
                None if b.parent < 0 => rb[j],
                None => m3(&unscale(&b.local.r).0, &tg[b.parent as usize].u),
            };
            let p = if b.parent < 0 {
                map[j].map_or([0.0; 3], |i| pa[i])
            } else {
                let par = &tg[b.parent as usize];
                let lt = match (j, map[j]) {
                    (1, Some(i)) => {
                        let rt = riku.bones[i].local.t;
                        let d = [loc_t[i][0] - rt[0], loc_t[i][1] - rt[1], loc_t[i][2] - rt[2]];
                        [b.local.t[0] + d[0], b.local.t[1] + d[1], b.local.t[2] + d[2]]
                    }
                    _ => b.local.t,
                };
                let w = crate::math::vm(lt, &par.u);
                [w[0] + par.p[0], w[1] + par.p[1], w[2] + par.p[2]]
            };
            tg.push(Target { u, s: [1.0; 3], p });
        }
        let l = locals_for(dark, &tg, None, rows.last().map(|v| v.as_slice()));
        rows.push(l);
    }
    Ok(encode(src, &rows))
}

/// The clip from per-frame locals, keeping the source clip's header fields (no scale channels).
fn encode(src: &Clip, rows: &[Vec<[f64; 9]>]) -> Clip {
    const EPS: [f64; 2] = [1e-7, 1e-5];
    let nb = rows[0].len();
    let mut channels = Vec::with_capacity(nb * 6);
    for b in 0..nb {
        for c in 0..6 {
            let rot = c >= 3;
            let vals: Vec<f64> = rows.iter().map(|r| if rot { r[b][c].to_degrees() } else { r[b][c] }).collect();
            channels.push(encode_channel(&vals, EPS[rot as usize]));
        }
    }
    Clip {
        name: src.name,
        bones: nb as i16,
        frames: rows.len() as i16,
        loop_start: src.loop_start,
        flags: src.flags & !2,
        has_blend: src.has_blend,
        blend: src.blend,
        blend_pad: src.blend_pad,
        mask_pad: vec![0; (((nb * 6 + 7) >> 3) + 3) & !3],
        channels,
        trailer: vec![0; 4],
    }
}

/// Riku's whole motion pack on Dark Riku's skeleton: the same slots, names and order.
pub fn retarget_ann(ann: &Ann, riku: &RecomSkeleton, dark: &RecomSkeleton) -> Result<Ann> {
    let map = bone_map(riku, dark);
    let slots = ann
        .slots
        .iter()
        .map(|s| {
            Ok(Slot {
                name: s.name,
                clip: match &s.clip {
                    Some(c) => Some(retarget(c, riku, dark, &map)?),
                    None => None,
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut out = Ann { slots, tail: Vec::new() };
    out.pad_tail_like_recom();
    Ok(out)
}

fn remap_u32(ev: &mut [u8], at: usize, map: &[usize]) {
    let v = u32::from_le_bytes(ev[at..at + 4].try_into().unwrap()) as usize;
    if let Some(&n) = map.get(v) {
        ev[at..at + 4].copy_from_slice(&(n as u32).to_le_bytes());
    }
}

/// Riku's `.SES` with each event's bone (`+0x08`) moved to Dark Riku's index.
pub fn remap_ses(d: &[u8], map: &[usize]) -> Result<Vec<u8>> {
    let mut es = crate::ses::parse(d)?;
    for e in &mut es {
        for ev in &mut e.events {
            remap_u32(ev, 0x08, map);
        }
    }
    Ok(crate::ses::write(&es))
}

/// Riku's `.EET` with list A's bone (`+0x24`) and list B's three bones (`+0x24`, `+0x34`,
/// `+0x44`) moved to Dark Riku's indices.
pub fn remap_eet(d: &[u8], map: &[usize]) -> Result<Vec<u8>> {
    let mut rs = crate::eet::parse(d)?;
    for r in &mut rs {
        for ev in &mut r.a {
            remap_u32(ev, 0x24, map);
        }
        for ev in &mut r.b {
            for at in [0x24, 0x34, 0x44] {
                remap_u32(ev, at, map);
            }
        }
    }
    crate::eet::write(&rs)
}

/// `PLAYER/<g>/PL<g>.<ext>`.
pub fn path(g: u16, ext: &str) -> String {
    format!("PLAYER/{g:04}/PL{g:04}.{ext}")
}


/// The skin manifest's file name (next to the other manifests).
pub const MANIFEST: &str = "kh1-riku-skin.json";

/// Dark Riku's bone 1 as Riku's name, so Riku's own bone lookups (`0x1401A4450` by name:
/// `ex_1010_sk`, `wep0`, `chest1`, `head`, `l_toes`, `r_toes`, from the strings at
/// `0x1407C5E10..0x1407C5E50`; Checked (exe)) find it. Same length; every part's name table.
pub fn rename_root(mdl: &[u8]) -> Result<Vec<u8>> {
    let from = b"ex_1580_sk\0";
    let to = b"ex_1010_sk\0";
    let mut out = mdl.to_vec();
    let mut n = 0;
    let mut i = 0;
    while i + from.len() <= out.len() {
        if &out[i..i + from.len()] == from {
            out[i..i + to.len()].copy_from_slice(to);
            n += 1;
            i += from.len();
        } else {
            i += 1;
        }
    }
    if n == 0 {
        return bad("Dark Riku's model has no ex_1580_sk bone");
    }
    Ok(out)
}

/// The files of one skin group, relative to the mod's `recom` root: the MDL / ANN / EET / SES
/// loose, the RTM / TXA as raw entries with their PC textures.
pub struct Built {
    /// (relative path, bytes).
    pub files: Vec<(String, Vec<u8>)>,
    /// Clips retargeted.
    pub clips: usize,
}

/// Reads Riku's group `riku` and Dark Riku's group `dark` from Recom.pkg and builds skin group
/// `skin`.
pub fn build(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, riku: u16, dark: u16, skin: u16) -> Result<Built> {
    let rd = |g: u16, ext: &str| crate::install::read_recom(arc, keys, &path(g, ext));
    let raw = |g: u16, ext: &str| -> Result<(Vec<u8>, Vec<kh1_formats::egs::HdAsset>, i32)> {
        let n = path(g, ext);
        let e = arc.find(&n).ok_or_else(|| crate::Error::Format(format!("{n} not in Recom.hed")))?;
        let raw = arc.read_raw_entry(e)?;
        let date = raw.get(12..16).map_or(0, |b| i32::from_le_bytes(b.try_into().unwrap()));
        let (d, a) = kh1_formats::egs::decode_entry_full(keys, &raw)?;
        Ok((d, a, date))
    };
    let rmdl = rd(riku, "MDL")?;
    let dmdl = rd(dark, "MDL")?;
    let rs = RecomSkeleton::parse(&rmdl)?;
    let ds = RecomSkeleton::parse(&dmdl)?;
    let ann = Ann::parse(&rd(riku, "ANN")?)?;
    let out_ann = retarget_ann(&ann, &rs, &ds)?;
    let clips = out_ann.slots.iter().filter(|s| s.clip.is_some()).count();
    let em = event_map(&rs, &ds);
    let mut files = vec![
        (path(skin, "MDL"), rename_root(&dmdl)?),
        (path(skin, "ANN"), out_ann.to_bytes()),
        (path(skin, "EET"), remap_eet(&rd(riku, "EET")?, &em)?),
        (path(skin, "SES"), remap_ses(&rd(riku, "SES")?, &em)?),
    ];
    for ext in ["RTM", "TXA"] {
        let (d, a, date) = raw(dark, ext)?;
        files.push((format!("raw/{}", path(skin, ext)), kh1_formats::egs::build_plain_entry(&d, &a, date)?));
    }
    Ok(Built { files, clips })
}

/// The manifest: which Riku group each skin group stands for.
pub fn manifest_json(skins: &[(u16, u16)]) -> String {
    let rows: Vec<String> = skins.iter().map(|(r, s)| format!("{{\"riku\": {r}, \"skin\": {s}}}")).collect();
    format!("{{\"format\": 1, \"skins\": [{}]}}\n", rows.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install;

    fn game() -> Option<std::path::PathBuf> {
        std::env::var_os("RECOM_GAME").map(Into::into)
    }

    /// With the owner's install (`RECOM_GAME`): Riku's idle on the Dark skeleton keeps its feet
    /// where Riku's are (within Dark's longer legs' difference), and an idle pose's bones
    /// stay near Dark's own rest.
    #[test]
    fn riku_idle_on_the_dark_skeleton() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let rd = |n: &str| install::read_recom(&arc, &keys, n).unwrap();
        let riku = RecomSkeleton::parse(&rd("PLAYER/0101/PL0101.MDL")).unwrap();
        let dark = RecomSkeleton::parse(&rd("PLAYER/0201/PL0201.MDL")).unwrap();
        let ann = Ann::parse(&rd("PLAYER/0101/PL0101.ANN")).unwrap();
        let map = bone_map(&riku, &dark);
        let idle = ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == "p002_001")).unwrap();
        let out = retarget(idle, &riku, &dark, &map).unwrap();
        assert_eq!(out.bones as usize, dark.bones.len());
        let world_y = |c: &Clip, sk: &RecomSkeleton, name: &str| {
            let k = sk.find(name).unwrap();
            let mut chain = vec![k];
            while sk.bones[*chain.last().unwrap()].parent >= 0 {
                chain.push(sk.bones[*chain.last().unwrap()].parent as usize);
            }
            let mut u = crate::math::I3;
            let mut p = [0.0; 3];
            for &b in chain.iter().rev() {
                let s = c.sample(b, 0.0);
                let w = crate::math::vm(s.t, &u);
                p = [p[0] + w[0], p[1] + w[1], p[2] + w[2]];
                u = m3(&rot_xyz(s.r[0], s.r[1], s.r[2]), &u);
            }
            p[1]
        };
        for foot in ["l_foot", "r_foot", "head"] {
            let a = world_y(idle, &riku, foot);
            let b = world_y(&out, &dark, foot);
            eprintln!("{foot}: riku {a:.3} dark {b:.3}");
            let ra = riku.rest_world()[riku.find(foot).unwrap()].t[1];
            let rb = dark.rest_world()[dark.find(foot).unwrap()].t[1];
            assert!(((a - ra) - (b - rb)).abs() < 0.03, "{foot}: Riku {a} (rest {ra}), Dark {b} (rest {rb})");
        }
        for &(r, d, k) in &SKIN_GROUPS {
            let b = build(&arc, &keys, r, d, k).unwrap();
            let get = |ext: &str| &b.files.iter().find(|f| f.0.ends_with(ext)).unwrap().1;
            let sk = RecomSkeleton::parse(get(".MDL")).unwrap();
            assert_eq!(sk.bones[1].name, "ex_1010_sk");
            let a = Ann::parse(get(".ANN")).unwrap();
            assert!(a.slots.iter().flatten_clips().all(|c| c.bones as usize == sk.bones.len()));
            crate::eet::parse(get(".EET")).unwrap();
            crate::ses::parse(get(".SES")).unwrap();
            eprintln!("group {k}: {} clips, files {:?}", b.clips, b.files.iter().map(|f| (f.0.clone(), f.1.len())).collect::<Vec<_>>());
        }
    }

    trait Clips<'a> {
        fn flatten_clips(self) -> Box<dyn Iterator<Item = &'a Clip> + 'a>;
    }
    impl<'a, I: Iterator<Item = &'a Slot> + 'a> Clips<'a> for I {
        fn flatten_clips(self) -> Box<dyn Iterator<Item = &'a Clip> + 'a> {
            Box::new(self.filter_map(|s| s.clip.as_ref()))
        }
    }
}
