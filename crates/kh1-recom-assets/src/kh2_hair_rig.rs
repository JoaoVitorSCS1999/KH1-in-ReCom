//! KH2 Riku's hair on Re:CoM Riku's 18 hair bones, moved by KH2's own motion (0.195.0; fork owner, 2026-11-24: "A" — the 18
//! hair bones re-posed on KH2's and KH2's motion written into them — then "ss" to the plan).
//!
//! - KH2's hair (`P_EH000` group 3) hangs on 25 bones besides the head (98) and one neck bone (95), with unweighted bones between
//!   them and the head (Checked (files): the workspace's `knowledge/KH2/kh2-riku-hair-models.md`). Re:CoM Riku has 18 hair bones
//!   (`b_hair1-3`, `f_hair`, `l/r_hair1-7`; 14 on `head`, 4 on `f_hair`). [`SELECTED`] are the 18 KH2 bones kept: each of the 7
//!   others hands its vertices to the kept bone (or the head) whose motion moves them least away from where KH2 puts them, over
//!   KH2's idle, walk, run and jump ([`choose`], test `selection_is_the_greedy_one`).
//! - Frames: a KH2 bone's transform relative to KH2's head is carried into Re:CoM's head frame: rotation conjugated by the two
//!   heads' rest rotations, origin through the hair's rest fit (scale, width, shift of [`crate::kh2_hair`]). The rest pose of each
//!   Re:CoM hair bone is its KH2 bone's at KH2's bind pose, so the vertices placed by the rest fit are exact at rest; in motion a
//!   bone moves its vertices rigidly (the width stretch is not rotated with them: Assumption, small).
//! - Every clip of Riku's `PL0101.ANN` keeps every other bone; its 18 hair-bone channels are rewritten: KH2's idle / walk / run /
//!   jump / fall where [`CLIP_MAP`] says so (time stretched so a loop closes on the clip's own length), else KH2's idle at its
//!   first frame (the hair still, carried by the head).

use std::collections::HashMap;

use kh2_formats_lite::mdlx::Model;
use kh2_formats_lite::skeleton::{self, Mat};

use crate::ann::Ann;
use crate::kh2_motion::Kh2Motion;
use crate::math::{euler_xyz, m3, rot_xyz, tr, unscale, vm, Aff};
use crate::{bad, Error, Result};

/// KH2's head bone.
pub const HEAD: usize = 98;
/// KH2's neck bone that carries a few hair corners (they follow Re:CoM's `neck3`).
pub const NECK: usize = 95;
/// Re:CoM's bone for them.
pub const NECK_RECOM: &str = "neck3";
/// KH2's hair bones with vertices, besides the head and the neck. Checked (files).
pub const HAIR_BONES: [usize; 25] = [101, 104, 106, 108, 110, 112, 114, 116, 118, 119, 121, 122, 124, 125, 127, 129, 131, 133, 135, 137, 139, 142, 144, 146, 148];
/// KH2's `P_EH000.mset` slots (battle, with weapon: OpenKH `MotionSet.GetMotionSetIndex`): idle, walk, run, jump, fall.
/// Checked (files: the slots hold A000, A001, A002, A003, A004).
pub const KH2_IDLE: usize = 0;
/// See [`KH2_IDLE`].
pub const KH2_WALK: usize = 4;
/// See [`KH2_IDLE`].
pub const KH2_RUN: usize = 8;
/// See [`KH2_IDLE`].
pub const KH2_JUMP: usize = 14;
/// See [`KH2_IDLE`].
pub const KH2_FALL: usize = 18;
/// Riku's `PL0101.ANN` slot → KH2 slot. Assumption (files: Re:CoM's clips are named by number only; picked by the feet: idle =
/// looped, feet still on the floor; walk / run = looped, feet apart 0.47-0.61 / 0.81-0.82 m, the lower foot rising 0.06 / 0.18-0.31 m;
/// jump = the lower foot leaving the floor; fall = looped clips in the air).
pub const CLIP_MAP: [(usize, usize); 14] = [
    (1, KH2_IDLE),
    (31, KH2_IDLE),
    (79, KH2_IDLE),
    (2, KH2_WALK),
    (28, KH2_WALK),
    (32, KH2_WALK),
    (80, KH2_WALK),
    (3, KH2_RUN),
    (21, KH2_RUN),
    (33, KH2_RUN),
    (10, KH2_JUMP),
    (26, KH2_JUMP),
    (12, KH2_FALL),
    (13, KH2_FALL),
];
/// Extra forward turn (degrees; negative = the hanging strands drop) per KH2 motion, the owner's ask on 0.195.0 in game: "a animação
/// quando corre ficou quase perfeito da parte de trás do cabelo mas ele sobe Muito". Not KH2's (in KH2's own poses the back of the
/// hair sits higher still: KH2's head is bent 28-46 deg forward of its rest pose, Re:CoM's 1-31, measured on the emulator and the clips,
/// test `head_tilts`).
/// Assumption: the amount, chosen from the renders.
pub const EXTRA_TILT: [(usize, f64); 2] = [(KH2_RUN, -25.0), (KH2_WALK, -25.0)];
/// Hair bones rooted this far or more in front of the head bone (metres, the head's forward axis at rest) are left out of the extra
/// turn.
pub const TILT_BEHIND: f64 = -0.02;
/// KH2 bones the extra turn leaves alone (the skull cap: turning it would lift it off the head).
pub const NO_TILT: [usize; 2] = [101, 104];
/// The KH2 motions the selection is measured on.
pub const CHOOSE_ON: [usize; 4] = [KH2_IDLE, KH2_WALK, KH2_RUN, KH2_JUMP];
/// Re:CoM's hair bones.
pub const RECOM_HAIR: [&str; 18] = ["b_hair1", "b_hair2", "b_hair3", "f_hair", "l_hair1", "l_hair2", "r_hair1", "r_hair2", "l_hair3", "l_hair4", "l_hair5", "l_hair6", "l_hair7", "r_hair3", "r_hair4", "r_hair5", "r_hair6", "r_hair7"];
/// Bones always kept (0.195.1: 142, the strand by the ear, moves 9-13 KH2 units off the head where 139, which carried it, moves 3-5;
/// fork owner on 0.195.0: "uma mecha ficou cobrindo a orelha", "essa mecha desloque de um jeito não natural").
pub const FORCED: [usize; 1] = [142];
/// The 18 KH2 bones kept ([`choose`]'s result on `P_EH000` with [`FORCED`], pinned by test `selection_is_the_greedy_one`).
pub const SELECTED: [usize; 18] = [101, 104, 106, 108, 110, 112, 114, 116, 119, 125, 127, 129, 131, 135, 137, 139, 142, 146];

/// A column matrix (`kh2_formats_lite::skeleton::Mat`) as a row-vector affine, rotation unscaled.
pub fn aff(m: &Mat) -> Aff {
    let r = [[m[0][0], m[0][1], m[0][2]], [m[1][0], m[1][1], m[1][2]], [m[2][0], m[2][1], m[2][2]]];
    Aff { r: unscale(&r).0, t: [m[3][0], m[3][1], m[3][2]] }
}

/// Inverse of a rigid affine.
pub fn rigid_inv(a: &Aff) -> Aff {
    let r = tr(&a.r);
    let t = vm(a.t, &r);
    Aff { r, t: [-t[0], -t[1], -t[2]] }
}

/// Every bone's transform relative to KH2's head (rigid, KH2 units).
pub fn rel(world: &[Mat]) -> Vec<Aff> {
    let k = rigid_inv(&aff(&world[HEAD]));
    world.iter().map(|w| aff(w).mul(&k)).collect()
}

/// The hair's vertices per KH2 bone, in that bone's bind frame (KH2 units).
pub fn vertices_by_bone(model: &Model, group: usize) -> HashMap<usize, Vec<[f64; 3]>> {
    let pose = skeleton::bind_pose(&model.bones);
    let mut out: HashMap<usize, Vec<[f64; 3]>> = HashMap::new();
    if let Some(g) = model.groups.get(group) {
        for t in &g.triangles {
            for v in t {
                let b = v.bone as usize;
                let w = skeleton::point(&pose[b], v.position.map(|x| x as f64));
                let l = rigid_inv(&aff(&pose[b])).point(w);
                out.entry(b).or_default().push(l);
            }
        }
    }
    out
}

/// Sampled head-relative transforms of the motions in `on` (every other frame).
pub fn samples(model: &Model, motions: &[&Kh2Motion]) -> Vec<Vec<Aff>> {
    let mut out = vec![];
    for m in motions {
        let mut f = 0.0f32;
        while f <= m.frame_end {
            out.push(rel(&m.world(&model.bones, f)));
            f += 2.0;
        }
    }
    out
}

/// The kept bones: starting from all of [`HAIR_BONES`], drop one at a time the bone whose vertices (and those it took over) move
/// least away from KH2's positions when they follow the best remaining bone or the head instead, until `keep` remain.
pub fn choose(model: &Model, group: usize, motions: &[&Kh2Motion], keep: usize, forced: &[usize]) -> Vec<usize> {
    let bind = rel(&skeleton::bind_pose(&model.bones));
    let verts = vertices_by_bone(model, group);
    let frames = samples(model, motions);
    // cost[d][h]: vertices of d following h (h = None: the head)
    let hosts: Vec<Option<usize>> = std::iter::once(None).chain(HAIR_BONES.iter().map(|&b| Some(b))).collect();
    let mut cost: HashMap<(usize, Option<usize>), f64> = HashMap::new();
    for &d in &HAIR_BONES {
        let vs = verts.get(&d).cloned().unwrap_or_default();
        for &h in &hosts {
            let mut c = 0.0;
            if h != Some(d) {
                // rest offset of d in h's frame
                let hb = h.map_or(Aff::IDENTITY, |h| bind[h]);
                let off = bind[d].mul(&rigid_inv(&hb));
                for fr in &frames {
                    let ht = h.map_or(Aff::IDENTITY, |h| fr[h]);
                    let approx = off.mul(&ht);
                    for v in &vs {
                        let (a, b) = (fr[d].point(*v), approx.point(*v));
                        c += ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
                    }
                }
            }
            cost.insert((d, h), c);
        }
    }
    let mut kept: Vec<usize> = HAIR_BONES.to_vec();
    let total = |kept: &[usize]| -> f64 {
        HAIR_BONES
            .iter()
            .map(|&d| if kept.contains(&d) { 0.0 } else { std::iter::once(None).chain(kept.iter().map(|&b| Some(b))).map(|h| cost[&(d, h)]).fold(f64::MAX, f64::min) })
            .sum()
    };
    while kept.len() > keep {
        let (i, _) = (0..kept.len())
            .filter(|&i| !forced.contains(&kept[i]))
            .map(|i| {
                let mut k = kept.clone();
                k.remove(i);
                (i, total(&k))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        kept.remove(i);
    }
    kept.sort();
    kept
}

/// For a KH2 bone, the bone its vertices follow: itself when kept, else the kept bone (or the head, `None`) of least cost.
pub fn hosts(model: &Model, group: usize, motions: &[&Kh2Motion], kept: &[usize]) -> HashMap<usize, Option<usize>> {
    let bind = rel(&skeleton::bind_pose(&model.bones));
    let verts = vertices_by_bone(model, group);
    let frames = samples(model, motions);
    let mut out = HashMap::new();
    for &d in &HAIR_BONES {
        if kept.contains(&d) {
            out.insert(d, Some(d));
            continue;
        }
        let vs = verts.get(&d).cloned().unwrap_or_default();
        let best = std::iter::once(None)
            .chain(kept.iter().map(|&b| Some(b)))
            .map(|h| {
                let hb = h.map_or(Aff::IDENTITY, |h| bind[h]);
                let off = bind[d].mul(&rigid_inv(&hb));
                let mut c = 0.0;
                for fr in &frames {
                    let approx = off.mul(&h.map_or(Aff::IDENTITY, |h| fr[h]));
                    for v in &vs {
                        let (a, b) = (fr[d].point(*v), approx.point(*v));
                        c += ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
                    }
                }
                (h, c)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|x| x.0)
            .unwrap_or(None);
        out.insert(d, best);
    }
    out
}

/// The carry from KH2's head frame to Re:CoM's: `A` (rotation, row vectors: `x_recom = x_kh2 · A`), the per-axis world scale
/// `c` (applied in world axes between the two rotations) and the world shift.
#[derive(Clone, Copy, Debug)]
pub struct Carry {
    /// KH2 head's rest rotation (row vectors).
    pub k: [[f64; 3]; 3],
    /// Re:CoM head's rest rotation (row vectors).
    pub h: [[f64; 3]; 3],
    /// World scale per axis (KH2 units → metres, with the width).
    pub c: [f64; 3],
    /// World shift (metres).
    pub shift: [f64; 3],
}

impl Carry {
    /// A point in KH2's head frame → Re:CoM's head frame.
    pub fn point(&self, q: [f64; 3]) -> [f64; 3] {
        let w = vm(q, &self.k);
        let w = [w[0] * self.c[0] + self.shift[0], w[1] * self.c[1] + self.shift[1], w[2] * self.c[2] + self.shift[2]];
        vm(w, &tr(&self.h))
    }
    /// A rotation in KH2's head frame → Re:CoM's head frame.
    pub fn rot(&self, r: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
        let a = m3(&self.k, &tr(&self.h));
        m3(&m3(&tr(&a), r), &a)
    }
    /// A KH2 head-relative transform → Re:CoM head-relative.
    pub fn aff(&self, x: &Aff) -> Aff {
        Aff { r: self.rot(&x.r), t: self.point(x.t) }
    }
}

/// The rig: which KH2 bone each Re:CoM hair bone carries.
#[derive(Clone, Debug)]
pub struct Rig {
    /// `(Re:CoM bone index, KH2 bone)`.
    pub pairs: Vec<(usize, usize)>,
    /// KH2 bone → the KH2 bone (kept) or head its vertices follow.
    pub host: HashMap<usize, Option<usize>>,
    /// The carry.
    pub carry: Carry,
}

impl Rig {
    /// Pairs the kept KH2 bones with Re:CoM's hair bones (each to the free Re:CoM bone nearest at rest, nearest pairs first).
    pub fn new(names: &[String], world: &[Aff], model: &Model, carry: Carry, host: HashMap<usize, Option<usize>>) -> Result<Rig> {
        let head = names.iter().position(|n| n == "head").ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let bind = rel(&skeleton::bind_pose(&model.bones));
        let hw = world[head];
        let recom: Vec<usize> = RECOM_HAIR.iter().map(|n| names.iter().position(|x| x == n).ok_or_else(|| Error::Format(format!("hair: no {n}")))).collect::<Result<_>>()?;
        // world positions at rest: KH2 bone origins carried, Re:CoM hair bones from the MDL
        let kpos: Vec<(usize, [f64; 3])> = SELECTED.iter().map(|&b| (b, Aff { r: crate::math::I3, t: carry.point(bind[b].t) }.mul(&hw).t)).collect();
        let mut cand: Vec<(f64, usize, usize)> = vec![];
        for &(b, p) in &kpos {
            for &r in &recom {
                let q = world[r].t;
                cand.push(((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2), r, b));
            }
        }
        cand.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mut used_r, mut used_k, mut pairs) = (vec![], vec![], vec![]);
        for (_, r, b) in cand {
            if used_r.contains(&r) || used_k.contains(&b) {
                continue;
            }
            used_r.push(r);
            used_k.push(b);
            pairs.push((r, b));
        }
        if pairs.len() != 18 {
            return bad(format!("hair: {} pairs", pairs.len()));
        }
        pairs.sort();
        Ok(Rig { pairs, host, carry })
    }

    /// The KH2 bone a Re:CoM bone carries.
    pub fn kh2_of(&self, r: usize) -> Option<usize> {
        self.pairs.iter().find(|p| p.0 == r).map(|p| p.1)
    }

    /// The Re:CoM bone a KH2 bone's vertices follow (`None`: the head).
    pub fn recom_of(&self, kh2: usize) -> Option<usize> {
        let h = self.host.get(&kh2).copied().flatten()?;
        self.pairs.iter().find(|p| p.1 == h).map(|p| p.0)
    }

    /// Head-relative (Re:CoM frame) transforms of the 18 Re:CoM hair bones for KH2 head-relative bone transforms `r`.
    pub fn head_rel(&self, r: &[Aff]) -> Vec<(usize, Aff)> {
        self.pairs.iter().map(|&(rb, kb)| (rb, self.carry.aff(&r[kb]))).collect()
    }

    /// Parent-relative locals of the 18 bones (their parent is `head` or another of them), given head-relative transforms.
    pub fn locals(&self, parents: &[i16], head: usize, hr: &[(usize, Aff)]) -> Vec<(usize, Aff)> {
        hr.iter()
            .map(|&(rb, a)| {
                let p = parents[rb] as usize;
                let pa = if p == head { Aff::IDENTITY } else { hr.iter().find(|x| x.0 == p).map_or(Aff::IDENTITY, |x| x.1) };
                (rb, a.mul(&rigid_inv(&pa)))
            })
            .collect()
    }
}

/// Writes the rest locals of the 18 bones into every model part's skeleton of a Re:CoM `.MDL` (Riku's: 4 parts).
pub fn patch_rest(mdl: &mut [u8], locals: &[(usize, Aff)]) -> Result<()> {
    let rd = |b: &[u8], o: usize| -> Result<u32> { b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| Error::Format("mdl: truncated".into())) };
    let first = rd(mdl, 0)? as usize;
    let parts: Vec<usize> = (0..first / 4).map(|i| rd(mdl, 4 * i).map(|x| x as usize)).collect::<Result<_>>()?;
    for p in parts.into_iter().filter(|&p| p != 0) {
        let n = u16::from_le_bytes([mdl[p], mdl[p + 1]]) as usize;
        let mt = p + rd(mdl, p + 8)? as usize;
        for &(b, a) in locals {
            if b >= n {
                return bad(format!("mdl: bone {b} of {n}"));
            }
            let m = [a.r[0][0], a.r[0][1], a.r[0][2], 0.0, a.r[1][0], a.r[1][1], a.r[1][2], 0.0, a.r[2][0], a.r[2][1], a.r[2][2], 0.0, a.t[0], a.t[1], a.t[2], 1.0];
            for (k, v) in m.iter().enumerate() {
                mdl[mt + 64 * b + 4 * k..mt + 64 * b + 4 * k + 4].copy_from_slice(&(*v as f32).to_le_bytes());
            }
        }
    }
    Ok(())
}

/// KH2 time (motion frames) for frame `f` of a Re:CoM clip of `frames` frames (30 fps both): a looped clip runs a whole number of
/// the KH2 motion's loops, stretched to close on the clip's length; otherwise KH2's own time, held at its end.
pub fn kh2_time(m: &Kh2Motion, looped: bool, frames: usize, f: usize) -> f32 {
    let (start, end, ret) = (m.frame_start, m.frame_end, m.frame_return.min(m.frame_end));
    if looped {
        let l = (end - ret).max(1.0);
        let n = ((frames as f32 / l).round()).max(1.0);
        let x = (f as f32 / frames as f32) * n * l;
        ret + x.rem_euclid(l)
    } else {
        (start + f as f32).min(end)
    }
}

/// The head's forward tilt (degrees) of a posed head rotation against the rest one (model y-z plane, + toward +z).
pub fn tilt(rest: &[[f64; 3]; 3], posed: &[[f64; 3]; 3]) -> f64 {
    let up = vm(vm([0.0, 1.0, 0.0], &tr(rest)), posed);
    up[2].atan2(up[1]).to_degrees()
}

/// Turns every hair bone but [`NO_TILT`]'s about its own origin by `deg` forward (about the model's x axis seen from Re:CoM's head
/// at rest, `head_rest`; a positive turn lifts strands that hang backwards): [`EXTRA_TILT`] (0.195.1).
pub fn tilt_hair(rig: &Rig, hr: &mut [(usize, Aff)], head_rest: &[[f64; 3]; 3], deg: f64, front: &[usize]) {
    if deg == 0.0 {
        return;
    }
    let rw = rot_xyz(deg.to_radians(), 0.0, 0.0);
    let rl = m3(&m3(head_rest, &rw), &tr(head_rest));
    for (rb, a) in hr.iter_mut() {
        if rig.kh2_of(*rb).is_some_and(|k| NO_TILT.contains(&k)) || front.contains(rb) {
            continue;
        }
        a.r = m3(&a.r, &rl);
    }
}

/// Rewrites the 18 hair bones' channels of every clip of Riku's pack. `sk`: Riku's original skeleton (for the head's tilt).
pub fn bake(ann: &mut Ann, rig: &Rig, sk: &crate::mdl::RecomSkeleton, head: usize, model: &Model, motions: &HashMap<usize, Kh2Motion>) -> Result<()> {
    let parents: Vec<i16> = sk.bones.iter().map(|b| b.parent).collect();
    let parents = &parents[..];
    let head_rest = unscale(&sk.rest_world()[head].r).0;
    let idle = motions.get(&KH2_IDLE).ok_or_else(|| Error::Format("KH2: no idle motion".into()))?;
    let still_hr = rig.head_rel(&rel(&idle.world(&model.bones, idle.frame_start)));
    // 0.195.1: every clip's Euler angles start on the branch of the bone's rest angles, so that a blend between two clips does not
    // cross a whole turn (0.195.0 wrote the same rotation up to 359 deg apart between clips: the back of the hair spun on a blend;
    // fork owner: "esse lance de uma mexida brusca é na parte de trás do cabelo inteiro")
    // 0.195.4: only bones rooted behind the head's middle take the extra turn (turning the front locks put them into the face;
    // fork owner on 0.195.3, walk: "traseira do cabelo ainda ta levantando muito quando anda")
    let rest_hr = rig.head_rel(&rel(&skeleton::bind_pose(&model.bones)));
    let front: Vec<usize> = rest_hr.iter().filter(|(_, a)| vm(a.t, &head_rest)[2] > TILT_BEHIND).map(|x| x.0).collect();
    let rest_e: HashMap<usize, [f64; 3]> = rig
        .locals(parents, head, &rig.head_rel(&rel(&skeleton::bind_pose(&model.bones))))
        .into_iter()
        .map(|(rb, a)| (rb, euler_xyz(&a.r, None)))
        .collect();
    for (slot, s) in ann.slots.iter_mut().enumerate() {
        let Some(c) = s.clip.as_mut() else { continue };
        let chans = c.chans();
        let frames = c.frames.max(1) as usize;
        let kh2 = CLIP_MAP.iter().find(|x| x.0 == slot).map(|x| x.1);
        let src = kh2.and_then(|k| motions.get(&k));
        let looped = c.looped() && c.loop_start == 1;
        let corr = EXTRA_TILT.iter().find(|x| Some(x.0) == kh2).map_or(0.0, |x| x.1);
        let rows: Vec<Vec<(usize, Aff)>> = (0..frames)
            .map(|f| {
                let mut hr = match src {
                    Some(m) => rig.head_rel(&rel(&m.world(&model.bones, kh2_time(m, looped, frames, f)))),
                    None => still_hr.clone(),
                };
                tilt_hair(rig, &mut hr, &head_rest, corr, &front);
                rig.locals(parents, head, &hr)
            })
            .collect();
        for (k, &(rb, _)) in rows[0].iter().enumerate() {
            if rb >= c.bones as usize {
                return bad(format!("hair: bone {rb} past clip {slot}'s {}", c.bones));
            }
            let mut prev: Option<[f64; 3]> = rest_e.get(&rb).copied();
            let mut vals = vec![vec![0.0f64; frames]; chans];
            for (f, row) in rows.iter().enumerate() {
                let a = row[k].1;
                let e = euler_xyz(&a.r, prev);
                prev = Some(e);
                for i in 0..3 {
                    vals[i][f] = a.t[i];
                    vals[3 + i][f] = e[i].to_degrees();
                }
                for v in vals.iter_mut().skip(6) {
                    v[f] = 1.0;
                }
            }
            for (ci, v) in vals.iter().enumerate() {
                let eps = [1e-7, 1e-5, 1e-6][ci / 3];
                c.channels[rb * chans + ci] = crate::convert::encode_channel(v, eps);
            }
        }
    }
    Ok(())
}

/// The head's world matrix of a clip at frame `t` (the clip's channels, the skeleton's rest for bones past the clip).
pub fn pose_head(sk: &crate::mdl::RecomSkeleton, c: &crate::ann::Clip, t: f64, head: usize) -> Aff {
    let mut w: Vec<Aff> = Vec::with_capacity(head + 1);
    for (i, b) in sk.bones.iter().enumerate().take(head + 1) {
        let local = if i < c.bones as usize {
            let s = c.sample(i, t);
            let mut r = rot_xyz(s.r[0], s.r[1], s.r[2]);
            for (row, sc) in r.iter_mut().zip(s.s) {
                for v in row.iter_mut() {
                    *v *= sc;
                }
            }
            Aff { r, t: s.t }
        } else {
            b.local
        };
        w.push(if b.parent < 0 { local } else { local.mul(&w[b.parent as usize]) });
    }
    w[head]
}

/// Checks that `rot_xyz` of a local's Euler angles gives back the local (used by the tests).
pub fn euler_roundtrip(a: &Aff) -> f64 {
    let e = euler_xyz(&a.r, None);
    let r = rot_xyz(e[0], e[1], e[2]);
    (0..3).flat_map(|i| (0..3).map(move |j| (i, j))).map(|(i, j)| (r[i][j] - a.r[i][j]).abs()).fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kh2_motion::read_slot;

    /// The 18 kept bones are the greedy choice on idle / walk / run / jump (local files; skipped without them).
    #[test]
    fn selection_is_the_greedy_one() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let ms: Vec<Kh2Motion> = CHOOSE_ON.iter().map(|&s| read_slot(&mset, s).unwrap().unwrap()).collect();
        let refs: Vec<&Kh2Motion> = ms.iter().collect();
        let kept = choose(&model, crate::kh2_hair::HAIR_GROUP, &refs, 18, &FORCED);
        let host = hosts(&model, crate::kh2_hair::HAIR_GROUP, &refs, &kept);
        let verts = vertices_by_bone(&model, crate::kh2_hair::HAIR_GROUP);
        eprintln!("kept {kept:?}");
        for d in HAIR_BONES { eprintln!("bone {d}: {} corners -> {:?}", verts.get(&d).map_or(0, |v| v.len()), host[&d]); }
        assert_eq!(kept, SELECTED.to_vec());
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::ddd_hair::{tests as dt, with_hair, HairSource, RIKU_0101_KH2};
    use crate::install;
    use crate::mdl::{split_parts, MdlPart, RecomSkeleton};

    /// 0.195.0 (owner's install, `RECOM_GAME`): the KH2 hair with KH2's motion, posed by the baked clips; renders in the
    /// workspace's `local/assets/recom/riku-hair/out/kh2/rig/`; the deviation of every hair vertex from KH2's exact motion (all 25
    /// bones carried) is printed per clip.
    #[test]
    fn kh2_hair_moves_as_kh2() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        assert!(src.mset.is_some());
        let mdl1 = src.prepare_mdl(&mdl0).unwrap();
        let swap = with_hair(&mdl1, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl0).unwrap().unwrap()).unwrap();
        let orig = crate::ann::Ann::parse(&ann0).unwrap();
        // only hair bones change
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let hairs: Vec<usize> = RECOM_HAIR.iter().map(|n| sk.find(n).unwrap()).collect();
        for (a, b) in ann.slots.iter().zip(&orig.slots) {
            let (Some(a), Some(b)) = (&a.clip, &b.clip) else { continue };
            assert_eq!((a.frames, a.bones, a.loop_start, a.flags), (b.frames, b.bones, b.loop_start, b.flags));
            let ch = a.chans();
            for (i, (x, y)) in a.channels.iter().zip(&b.channels).enumerate() {
                if !hairs.contains(&(i / ch)) {
                    assert!(x == y, "non-hair channel {i} changed");
                }
            }
        }
        let parts = split_parts(&swap.mdl).unwrap();
        let p3 = MdlPart::parse_bytes(&parts[3]).unwrap();
        let img = crate::texture::decode_dds(&swap.dds).unwrap();
        let out = dt::out_dir().join("kh2").join("rig");
        std::fs::create_dir_all(&out).unwrap();
        // exact reference: every hair vertex carried by its own KH2 bone
        let model = kh2_formats_lite::mdlx::parse(&src.mdlx).unwrap();
        let rig = src.rig(&mdl0).unwrap().unwrap();
        let motions = src.motions().unwrap();
        let verts = vertices_by_bone(&model, crate::kh2_hair::HAIR_GROUP);
        let head = sk.find("head").unwrap();
        {
            let d = out.join("rest");
            std::fs::create_dir_all(&d).unwrap();
            let w = sk.rest_world();
            let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 0, &d.join("body_side.png"));
            let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 3, &d.join("body_side2.png"));
            // rest dump: hair (texture 1 on head / hair bones) 'h', the rest of the head 'r', world metres
            let mut txt = String::new();
            for t in dt::collect(&p3) {
                if !t.v.iter().all(|v| { let n = &names[v.bone()]; n == "head" || n.contains("hair") }) { continue; }
                let hair = t.v.iter().any(|v| names[v.bone()].contains("hair")) || t.tex == 1 && t.v.iter().all(|v| v.uv().is_some_and(|u| u[0] >= 0.0));
                let _ = hair;
                txt += &format!("{} {}\n", if t.v.iter().any(|v| names[v.bone()].contains("hair")) { 'h' } else { 'r' }, t.v.iter().flat_map(|v| w[v.bone()].point(v.pos().map(|x| x as f64))).map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "));
            }
            std::fs::write(d.join("rest_tris.txt"), txt).unwrap();
        }
        for (clip, frames) in [(1usize, vec![0.0, 25.0, 50.0, 75.0]), (2, vec![0.0, 8.0, 16.0, 24.0]), (3, vec![0.0, 6.0, 11.0, 17.0]), (10, vec![5.0]), (12, vec![8.0]), (33, vec![5.0]), (37, vec![10.0]), (41, vec![16.0])] {
            let Some(c) = ann.slots[clip].clip.as_ref() else { continue };
            let src_m = CLIP_MAP.iter().find(|x| x.0 == clip).and_then(|x| motions.get(&x.1));
            let mut worst = 0.0f64;
            for f in frames {
                let w = crate::kh2_hair::tests::posed_world(&sk, c, f);
                let d = out.join(format!("c{clip}_f{f}"));
                std::fs::create_dir_all(&d).unwrap();
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 0, &d.join("body_side.png"));
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 1, &d.join("body_back.png"));
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 2, &d.join("body_front.png"));
                let _ = crate::kh2_hair::tests::render_body(&p3, &w, &names, &img, 3, &d.join("body_side2.png"));
                {
                    // posed dump in the rest head's frame (the face lands where it is at rest)
                    let hrest = sk.rest_world()[head];
                    let back = |p: [f64; 3]| hrest.point(rigid_inv(&Aff { r: unscale(&w[head].r).0, t: w[head].t }).point(p));
                    let mut txt = String::new();
                    for t in dt::collect(&p3) {
                        if !t.v.iter().all(|v| { let n = &names[v.bone()]; n == "head" || n.contains("hair") }) { continue; }
                        txt += &format!("{} {}\n", if t.v.iter().any(|v| names[v.bone()].contains("hair")) { 'h' } else { 'r' }, t.v.iter().flat_map(|v| back(w[v.bone()].point(v.pos().map(|x| x as f64)))).map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "));
                    }
                    std::fs::write(d.join("head_tris.txt"), txt).unwrap();
                }
                if let Some(m) = src_m {
                    let looped = c.looped() && c.loop_start == 1;
                    let r = rel(&m.world(&model.bones, kh2_time(m, looped, c.frames as usize, f as usize)));
                    let hr: HashMap<usize, Aff> = rig.head_rel(&r).into_iter().collect();
                    for &kb in &HAIR_BONES {
                        let host = rig.host[&kb];
                        let bind = rel(&kh2_formats_lite::skeleton::bind_pose(&model.bones));
                        for v in verts.get(&kb).into_iter().flatten() {
                            let exact = rig.carry.point(r[kb].point(*v));
                            let ours = match host {
                                Some(h) => rig.carry.point(bind[kb].mul(&rigid_inv(&bind[h])).mul(&r[h]).point(*v)),
                                None => rig.carry.point(bind[kb].point(*v)),
                            };
                            let _ = (&hr, head);
                            worst = worst.max(crate::math::dist(exact, ours));
                        }
                    }
                }
            }
            eprintln!("clip {clip}: worst vertex off KH2's exact motion {:.3} m", worst);
        }
    }
}

#[cfg(test)]
mod kh2_self_render {
    use super::*;
    use crate::kh2_motion::read_slot;

    /// Debug: KH2 Riku's whole model posed by his own motions with this reader (side views, hair red), to see how KH2's hair
    /// hangs in KH2's own world. `local/assets/recom/riku-hair/out/kh2/rig/kh2_self.png`.
    #[test]
    fn kh2_self_side_views() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let pose0 = kh2_formats_lite::skeleton::bind_pose(&model.bones);
        let views: Vec<(usize, f32)> = vec![(KH2_IDLE, 0.0), (KH2_IDLE, 60.0), (KH2_WALK, 10.0), (KH2_RUN, 6.0), (KH2_JUMP, 8.0), (KH2_FALL, 25.0)];
        let (w, h) = (400usize, 500usize);
        let mut sheet = vec![30u8; w * views.len() * h * 3];
        for (vi, &(slot, f)) in views.iter().enumerate() {
            let m = read_slot(&mset, slot).unwrap().unwrap();
            // the emulator's matrices (the whole body as KH2 poses it, IK and constraints included)
            let raw = std::fs::read(format!("{}/slot{slot}.f32", crate::kh2_motion::tests::emu())).unwrap();
            let fl: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            let per = (fl.len() / 16) / (m.frame_end.round() as usize + 1);
            let fi = (f.round() as usize).min(m.frame_end.round() as usize);
            let world: Vec<kh2_formats_lite::skeleton::Mat> = (0..model.bones.len()).map(|b| { let o = (fi * per + b) * 16; core::array::from_fn(|i| core::array::from_fn(|j| fl[o + i * 4 + j] as f64)) }).collect();
            let mut zb = vec![f64::MIN; w * h];
            for (gi, g) in model.groups.iter().enumerate() {
                for t in &g.triangles {
                    let p: Vec<[f64; 3]> = t.iter().map(|v| {
                        let wb = kh2_formats_lite::skeleton::point(&pose0[v.bone as usize], v.position.map(|x| x as f64));
                        let l = rigid_inv(&aff(&pose0[v.bone as usize])).point(wb);
                        aff(&world[v.bone as usize]).point(l)
                    }).collect();
                    // side view: screen x = z, y = y; depth = -x
                    let s: Vec<[f64; 3]> = p.iter().map(|q| [200.0 + q[2] * 2.2, 480.0 - q[1] * 2.2, -q[0]]).collect();
                    let det = (s[1][1] - s[2][1]) * (s[0][0] - s[2][0]) + (s[2][0] - s[1][0]) * (s[0][1] - s[2][1]);
                    if det.abs() < 1e-9 { continue; }
                    let (x0, x1) = (s.iter().map(|q| q[0]).fold(f64::MAX, f64::min).max(0.0) as usize, (s.iter().map(|q| q[0]).fold(f64::MIN, f64::max) as usize + 1).min(w - 1));
                    let (y0, y1) = (s.iter().map(|q| q[1]).fold(f64::MAX, f64::min).max(0.0) as usize, (s.iter().map(|q| q[1]).fold(f64::MIN, f64::max) as usize + 1).min(h - 1));
                    for y in y0..=y1 { for x in x0..=x1 {
                        let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                        let l0 = ((s[1][1] - s[2][1]) * (px - s[2][0]) + (s[2][0] - s[1][0]) * (py - s[2][1])) / det;
                        let l1 = ((s[2][1] - s[0][1]) * (px - s[2][0]) + (s[0][0] - s[2][0]) * (py - s[2][1])) / det;
                        let l2 = 1.0 - l0 - l1;
                        if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                        let z = l0 * s[0][2] + l1 * s[1][2] + l2 * s[2][2];
                        if z <= zb[y * w + x] { continue; }
                        zb[y * w + x] = z;
                        let c = if gi == crate::kh2_hair::HAIR_GROUP { [200u8, 80, 80] } else { [110, 140, 200] };
                        let o = 3 * (y * w * views.len() + vi * w + x);
                        sheet[o..o + 3].copy_from_slice(&c);
                    } }
                }
            }
        }
        let out = crate::ddd_hair::tests::out_dir().join("kh2").join("rig");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("kh2_self.png"), crate::ddd_hair::tests::png(w * views.len(), h, &sheet)).unwrap();
    }
}

#[cfg(test)]
mod amp_tests {
    use super::*;
    use crate::kh2_motion::read_slot;
    /// Debug: per KH2 hair bone, its corners, and how far (KH2 units) its vertices move off the rigid head on each motion.
    #[test]
    fn bone_amplitudes() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let bind = rel(&kh2_formats_lite::skeleton::bind_pose(&model.bones));
        let verts = vertices_by_bone(&model, crate::kh2_hair::HAIR_GROUP);
        for &b in &HAIR_BONES {
            let mut line = format!("bone {b} at {:?} corners {}:", bind[b].t.map(|x| x.round()), verts.get(&b).map_or(0, |v| v.len()));
            for s in [KH2_IDLE, KH2_WALK, KH2_RUN, KH2_JUMP] {
                let m = read_slot(&mset, s).unwrap().unwrap();
                let mut worst = 0.0f64;
                let mut f = 0.0;
                while f <= m.frame_end {
                    let r = rel(&m.world(&model.bones, f));
                    for v in verts.get(&b).into_iter().flatten() { worst = worst.max(crate::math::dist(r[b].point(*v), bind[b].point(*v))); }
                    f += 2.0;
                }
                line += &format!(" {:.1}", worst);
            }
            eprintln!("{line}");
        }
    }
}

#[cfg(test)]
mod tilt_tests {
    use super::*;
    use crate::kh2_motion::read_slot;
    /// Debug: the head's forward tilt (degrees, in the model's y-z plane, + = face down / forward) against its rest, KH2 (emulator
    /// matrices) and Re:CoM (the mapped clips).
    #[test]
    fn head_tilts() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let kb = aff(&kh2_formats_lite::skeleton::bind_pose(&model.bones)[HEAD]);
        // fwd tilt of a head: carry the rest head's world up (0,1,0) by (posed · rest^-1); angle in y-z, + toward +z
        let tilt = |rest: &[[f64; 3]; 3], posed: &[[f64; 3]; 3]| { let up = vm(vm([0.0, 1.0, 0.0], &tr(rest)), posed); up[2].atan2(up[1]).to_degrees() };
        for s in [KH2_IDLE, KH2_WALK, KH2_RUN, KH2_JUMP, KH2_FALL] {
            let m = read_slot(&mset, s).unwrap().unwrap();
            let Ok(raw) = std::fs::read(format!("{}/slot{s}.f32", crate::kh2_motion::tests::emu())) else { continue };
            let fl: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            let n = m.frame_end.round() as usize + 1;
            let per = (fl.len() / 16) / n;
            let v: Vec<f64> = (0..n).map(|f| { let o = (f * per + HEAD) * 16; let mm: kh2_formats_lite::skeleton::Mat = core::array::from_fn(|i| core::array::from_fn(|j| fl[o + i * 4 + j] as f64)); tilt(&kb.r, &aff(&mm).r) }).collect();
            eprintln!("KH2 slot {s}: mean {:.1} range {:.1}..{:.1}", v.iter().sum::<f64>() / n as f64, v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
        }
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = crate::install::keys(&g).unwrap();
        let arc = crate::install::recom_archive(&g).unwrap();
        let sk = crate::mdl::RecomSkeleton::parse(&crate::install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap()).unwrap();
        let ann = crate::ann::Ann::parse(&crate::install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let head = sk.find("head").unwrap();
        let rest = unscale(&sk.rest_world()[head].r).0;
        for &(slot, s) in CLIP_MAP.iter() {
            let c = ann.slots[slot].clip.as_ref().unwrap();
            let v: Vec<f64> = (0..c.frames as usize).map(|f| tilt(&rest, &unscale(&crate::kh2_hair::tests::posed_world(&sk, c, f as f64)[head].r).0)).collect();
            eprintln!("Re:CoM {slot} (KH2 {s}): mean {:.1} range {:.1}..{:.1}", v.iter().sum::<f64>() / v.len() as f64, v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
        }
    }
}

#[cfg(test)]
mod euler_tests {
    use super::*;
    use crate::ddd_hair::HairSource;
    use crate::install;
    /// Hair-bone Euler channels of the baked pack: the largest step between frames inside a clip, and the largest difference between
    /// clips' first frames (what a blend from one clip into another crosses), in degrees.
    #[test]
    fn baked_euler_is_continuous() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let _ = src.prepare_mdl(&mdl0).unwrap();
        let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl0).unwrap().unwrap()).unwrap();
        let sk = crate::mdl::RecomSkeleton::parse(&mdl0).unwrap();
        let hairs: Vec<usize> = RECOM_HAIR.iter().map(|n| sk.find(n).unwrap()).collect();
        let (mut step, mut across) = (0.0f64, 0.0f64);
        let mut firsts: Vec<(usize, Vec<f64>)> = vec![];
        for (i, s) in ann.slots.iter().enumerate() {
            let Some(c) = &s.clip else { continue };
            let mut first = vec![];
            for &b in &hairs {
                for ch in 3..6 {
                    let v: Vec<f64> = (0..c.frames as usize).map(|f| c.value(b, ch, f as f64)).collect();
                    for w in v.windows(2) { let d = (w[1] - w[0]).abs(); if d > step { step = d; eprintln!("step {d:.1} deg: clip {i} bone {b} ch {ch}"); } }
                    first.push(v[0]);
                }
            }
            for (j, f) in &firsts { for (k, (a, b)) in f.iter().zip(&first).enumerate() { let d = (a - b).abs(); if d > 60.0 { eprintln!("big {d:.1} deg: clip {j} vs {i}, hair {} ch {}", k / 3, 3 + k % 3); } across = across.max(d); } }
            firsts.push((i, first));
        }
        eprintln!("worst step {step:.1}, worst across clips {across:.1}");
        // 0.195.1: no clip's first frame is a turn away from another's (0.195.0: up to 359 deg, the hair spun on a blend)
        assert!(across < 180.0, "hair-bone Euler angles {across:.1} deg apart between clips' first frames");
        // Seen, not asserted (0.196.2): in the run (clips 3, 21) `l_hair1` / `l_hair2` step up to 45 deg in one frame (KH2's run
        // at full speed, or an Euler near its gimbal: not settled).
    }
}

#[cfg(test)]
mod loop_tests {
    use super::*;
    use crate::kh2_motion::read_slot;
    /// How far each hair vertex is between a looped KH2 motion's loop start and its end (must be ~0: no jump at the loop), and the
    /// largest move between two consecutive frames inside the motion (KH2 units, printed).
    #[test]
    fn kh2_loops_close() {
        let Ok(mset) = std::fs::read(format!("{}/obj__P_EH000.mset", crate::kh2_motion::tests::raw())) else { return };
        let model = kh2_formats_lite::mdlx::parse(&std::fs::read(format!("{}/obj__P_EH000.mdlx", crate::kh2_motion::tests::raw())).unwrap()).unwrap();
        let verts = vertices_by_bone(&model, crate::kh2_hair::HAIR_GROUP);
        let at = |m: &Kh2Motion, t: f32| -> Vec<[f64; 3]> { let r = rel(&m.world(&model.bones, t)); HAIR_BONES.iter().flat_map(|&b| verts.get(&b).into_iter().flatten().map(move |v| (b, *v))).map(|(b, v)| r[b].point(v)).collect() };
        for s in [KH2_IDLE, KH2_WALK, KH2_RUN, KH2_FALL] {
            let m = read_slot(&mset, s).unwrap().unwrap();
            let (a, b) = (at(&m, m.frame_return), at(&m, m.frame_end));
            let gap = a.iter().zip(&b).map(|(x, y)| crate::math::dist(*x, *y)).fold(0.0, f64::max);
            let mut step = 0.0f64;
            let mut f = m.frame_return;
            while f < m.frame_end { let (x, y) = (at(&m, f), at(&m, f + 1.0)); step = step.max(x.iter().zip(&y).map(|(p, q)| crate::math::dist(*p, *q)).fold(0.0, f64::max)); f += 1.0; }
            eprintln!("KH2 slot {s}: loop {}..{}: seam {gap:.2}, largest frame step {step:.2}", m.frame_return, m.frame_end);
            // 0.195.2: a looped motion closes on itself (0.195.0 / 0.195.1 jumped 6.4 KH2 units here)
            assert!(gap < 0.05, "KH2 slot {s}: the hair jumps {gap:.2} KH2 units at the loop");
            let a0 = at(&m, m.frame_start);
            let mut best = (0.0f32, f64::MAX);
            let mut t = m.frame_start + 1.0;
            while t <= m.frame_end + 0.01 { let d = at(&m, t).iter().zip(&a0).map(|(x, y)| crate::math::dist(*x, *y)).fold(0.0, f64::max); if d < best.1 { best = (t, d); } t += 0.5; }
            let hair_curves: Vec<&crate::kh2_motion::Curve> = m.curves.iter().filter(|c| c.joint >= 99 && c.joint <= 148).collect();
            let kt0 = hair_curves.iter().filter_map(|c| c.keys.first().map(|k| k.time)).fold(f32::MAX, f32::min);
            let kt1 = hair_curves.iter().filter_map(|c| c.keys.last().map(|k| k.time)).fold(f32::MIN, f32::max);
            let posts: std::collections::BTreeSet<(u8, u8)> = hair_curves.iter().map(|c| (c.pre, c.post)).collect();
            eprintln!("   start {} end {} ret {} fps {}; closest to frame start at t {} ({:.2}); hair keys {kt0}..{kt1}, (pre, post) {posts:?}", m.frame_start, m.frame_end, m.frame_return, m.fps, best.0, best.1);
        }
    }
}

#[cfg(test)]
mod side_by_side {
    //! Debug (0.195.3): KH2 Riku as KH2 poses him (OpenKH emulator matrices, `local/assets/kh2/riku/emu_full`, whole motions in
    //! 60 Hz time) next to Re:CoM Riku with the baked KH2 hair, frame by frame, same head-centred camera turned to the face's
    //! heading, same scale (KH2 units × [`crate::kh2_hair::SCALE`]). Frames in the workspace's
    //! `local/assets/recom/riku-hair/out/kh2/sbs/<clip>/`, panels: KH2 side | Re:CoM side | KH2 back | Re:CoM back.
    use super::*;
    use crate::ddd_hair::{tests as dt, with_hair, HairSource, RIKU_0101_KH2};
    use crate::install;
    use crate::mdl::{split_parts, MdlPart, RecomSkeleton};
    use crate::texture::Image;

    const S: usize = 320;

    struct Tri { p: [[f64; 3]; 3], uv: Option<[[f64; 2]; 3]>, hair: bool }

    /// Rasterises `tris` (already in a head-centred, face-to-+z frame, metres) from the side (+x) or the back, z-buffered.
    fn raster(tris: &[Tri], atlas: &Image, back: bool, px_per_m: f64) -> Vec<u8> {
        let mut rgb = vec![28u8; S * S * 3];
        let mut zb = vec![f64::MIN; S * S];
        for t in tris {
            let q: Vec<[f64; 3]> = t.p.iter().map(|p| if back { [-p[0], p[1], -p[2]] } else { [p[2], p[1], -p[0]] }).collect();
            let s: Vec<[f64; 2]> = q.iter().map(|p| [S as f64 * 0.5 + p[0] * px_per_m, S as f64 * 0.42 - p[1] * px_per_m]).collect();
            let det = (s[1][1] - s[2][1]) * (s[0][0] - s[2][0]) + (s[2][0] - s[1][0]) * (s[0][1] - s[2][1]);
            if det.abs() < 1e-9 { continue; }
            let a = [q[1][0] - q[0][0], q[1][1] - q[0][1], q[1][2] - q[0][2]];
            let b = [q[2][0] - q[0][0], q[2][1] - q[0][1], q[2][2] - q[0][2]];
            let n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
            let shade = 0.55 + 0.45 * (n[2].abs() / (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12));
            let (x0, x1) = (s.iter().map(|p| p[0]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (s.iter().map(|p| p[0]).fold(f64::MIN, f64::max).ceil().max(0.0) as usize).min(S - 1));
            let (y0, y1) = (s.iter().map(|p| p[1]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (s.iter().map(|p| p[1]).fold(f64::MIN, f64::max).ceil().max(0.0) as usize).min(S - 1));
            if x0 > x1 || y0 > y1 { continue; }
            for y in y0..=y1 { for x in x0..=x1 {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let l0 = ((s[1][1] - s[2][1]) * (px - s[2][0]) + (s[2][0] - s[1][0]) * (py - s[2][1])) / det;
                let l1 = ((s[2][1] - s[0][1]) * (px - s[2][0]) + (s[0][0] - s[2][0]) * (py - s[2][1])) / det;
                let l2 = 1.0 - l0 - l1;
                if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                let z = l0 * q[0][2] + l1 * q[1][2] + l2 * q[2][2];
                if z <= zb[y * S + x] { continue; }
                zb[y * S + x] = z;
                let c = match (t.hair, t.uv) {
                    (true, Some(uv)) => {
                        let u = l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0];
                        let v = l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1];
                        let (u, v) = (u.rem_euclid(1.0), v.rem_euclid(1.0));
                        let c = atlas.px[((v * atlas.h as f64) as usize).min(atlas.h - 1) * atlas.w + ((u * atlas.w as f64) as usize).min(atlas.w - 1)];
                        [(c[0] as f64 * shade) as u8, (c[1] as f64 * shade) as u8, (c[2] as f64 * shade) as u8]
                    }
                    _ => { let k = 60.0 + 150.0 * shade - 60.0; [(k * 0.55) as u8, (k * 0.7) as u8, k as u8] }
                };
                rgb[3 * (y * S + x)..3 * (y * S + x) + 3].copy_from_slice(&c);
            } }
        }
        rgb
    }

    /// The frame that puts `head` at the origin and turns `fwd` (a head-relative point ahead of the face) to +z about y.
    fn facing(head: [f64; 3], fwd: [f64; 3]) -> impl Fn([f64; 3]) -> [f64; 3] {
        let d = [fwd[0] - head[0], fwd[2] - head[2]];
        let yaw = d[0].atan2(d[1]); // angle from +z toward +x
        let (s, c) = yaw.sin_cos();
        move |p: [f64; 3]| { let x = p[0] - head[0]; let z = p[2] - head[2]; [x * c - z * s, p[1] - head[1], x * s + z * c] }
    }

    /// Debug (0.196.1): KH2's whole head (skin flat, hair textured) carried by the hair's fit (scale, width, shift; no face fit)
    /// next to Re:CoM's final head and hair at rest, both sides, the same camera. `out/kh2/sbs/head_rest.png`: KH2 right side |
    /// ours | KH2 left side | ours.
    #[test]
    fn kh2_head_vs_ours_at_rest() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let mdl1 = src.prepare_mdl(&mdl0).unwrap();
        let swap = with_hair(&mdl1, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let p3 = MdlPart::parse_bytes(&split_parts(&swap.mdl).unwrap()[3]).unwrap();
        let ratlas = crate::texture::decode_dds(&swap.dds).unwrap();
        let katlas = crate::texture::decode_dds(&src.dds).unwrap();
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let w = sk.rest_world();
        let h = w[sk.find("head").unwrap()].t;
        let model = kh2_formats_lite::mdlx::parse(&src.mdlx).unwrap();
        let pose = kh2_formats_lite::skeleton::bind_pose(&model.bones);
        let k = kh2_formats_lite::skeleton::point(&pose[HEAD], [0.0; 3]);
        let (sc, wd, sh) = (crate::kh2_hair::SCALE, crate::kh2_hair::WIDTH, crate::kh2_hair::SHIFT);
        let frame = |p: [f64; 3]| [wd * sc * (p[0] - k[0]) + sh[0], sc * (p[1] - k[1]) + sh[1] - 0.04, sc * (p[2] - k[2]) + sh[2] + 0.03];
        let mut kt = vec![];
        for (gi, gr) in model.groups.iter().enumerate() {
            for t in &gr.triangles {
                let p = t.clone().map(|v| frame(kh2_formats_lite::skeleton::point(&pose[v.bone as usize], v.position.map(|x| x as f64))));
                if p.iter().any(|q| q[1] < -0.2) { continue; }
                kt.push(Tri { p, uv: Some(t.clone().map(|v| [v.uv[0] as f64, v.uv[1] as f64])), hair: gi == crate::kh2_hair::HAIR_GROUP });
            }
        }
        let keep = |n: &str| n == "head" || n.contains("hair") || n.starts_with("neck") || n.starts_with("chest");
        let mut rt = vec![];
        for t in dt::collect(&p3) {
            if !t.v.iter().all(|v| keep(&names[v.bone()])) { continue; }
            let p = t.v.clone().map(|v| { let q = w[v.bone()].point(v.pos().map(|x| x as f64)); [q[0] - h[0], q[1] - h[1] - 0.04, q[2] - h[2] + 0.03] });
            rt.push(Tri { p, uv: Some(t.v.clone().map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]))), hair: t.tex == 1 && t.v.iter().any(|v| names[v.bone()].contains("hair") || names[v.bone()] == "head") });
        }
        // dumps for the ear landmarks: KH2's head skin (non-hair groups, head bone) and Re:CoM's face, head-relative, metres
        {
            let mut txt = String::new();
            for (gi, gr) in model.groups.iter().enumerate() {
                if gi == crate::kh2_hair::HAIR_GROUP { continue; }
                for t in &gr.triangles {
                    if !t.iter().all(|v| v.bone as usize == HEAD) { continue; }
                    let p = t.clone().map(|v| { let q = kh2_formats_lite::skeleton::point(&pose[v.bone as usize], v.position.map(|x| x as f64)); [wd * sc * (q[0] - k[0]) + sh[0], sc * (q[1] - k[1]) + sh[1], sc * (q[2] - k[2]) + sh[2]] });
                    txt += &format!("k {}\n", p.iter().flatten().map(|x| format!("{x:.5}")).collect::<Vec<_>>().join(" "));
                }
            }
            std::fs::write(dt::out_dir().join("kh2").join("sbs").join("kh2_skin.txt"), txt).unwrap();
        }
        // the Re:CoM face is on the same atlas as the hair: texture all of ours; KH2's face flat
        for t in rt.iter_mut() { t.hair = true; }
        let mirror = |ts: &[Tri]| -> Vec<Tri> { ts.iter().map(|t| Tri { p: t.p.map(|q| [-q[0], q[1], q[2]]), uv: t.uv, hair: t.hair }).collect() };
        let ppm = 1300.0;
        let panels = [raster(&kt, &katlas, false, ppm), raster(&rt, &ratlas, false, ppm), raster(&mirror(&kt), &katlas, false, ppm), raster(&mirror(&rt), &ratlas, false, ppm)];
        let mut img = vec![0u8; 4 * S * S * 3];
        for (kk, pnl) in panels.iter().enumerate() {
            for y in 0..S { img[3 * (y * 4 * S + kk * S)..3 * (y * 4 * S + kk * S + S)].copy_from_slice(&pnl[3 * y * S..3 * (y + 1) * S]); }
        }
        let out = dt::out_dir().join("kh2").join("sbs");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("head_rest.png"), dt::png(4 * S, S, &img)).unwrap();
    }

    #[test]
    fn kh2_and_recom_side_by_side() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        if !std::path::Path::new(&format!("{}/slot0.f32", crate::kh2_motion::tests::emu_full())).exists() { return; }
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let mdl1 = src.prepare_mdl(&mdl0).unwrap();
        let swap = with_hair(&mdl1, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let ann = crate::ann::Ann::parse(&src.bake_ann(&ann0, &mdl0).unwrap().unwrap()).unwrap();
        let p3 = MdlPart::parse_bytes(&split_parts(&swap.mdl).unwrap()[3]).unwrap();
        let ratlas = crate::texture::decode_dds(&swap.dds).unwrap();
        let katlas = crate::texture::decode_dds(&src.dds).unwrap();
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let rhead = sk.find("head").unwrap();
        let model = kh2_formats_lite::mdlx::parse(&src.mdlx).unwrap();
        let pose0 = kh2_formats_lite::skeleton::bind_pose(&model.bones);
        let motions = src.motions().unwrap();
        let s = crate::kh2_hair::SCALE;
        let keep = |n: &str| n == "head" || n.contains("hair") || n.starts_with("neck") || n.starts_with("chest") || n == "waist" || n.ends_with("collar") || n.ends_with("uparm") || n.ends_with("foarm");
        let base = dt::out_dir().join("kh2").join("sbs");
        for (clip, kslot, n) in [(1usize, KH2_IDLE, 16usize), (2, KH2_WALK, 12), (3, KH2_RUN, 12), (10, KH2_JUMP, 10), (12, KH2_FALL, 8)] {
            let Some(c) = ann.slots[clip].clip.as_ref() else { continue };
            let m = &motions[&kslot];
            let raw = std::fs::read(format!("{}/slot{kslot}.f32", crate::kh2_motion::tests::emu_full())).unwrap();
            let fl: Vec<f32> = raw.chunks_exact(4).map(|x| f32::from_le_bytes(x.try_into().unwrap())).collect();
            let samples = (m.frame_end * m.keys_per_frame()).round() as usize + 1;
            let per = (fl.len() / 16) / samples;
            let looped = c.looped() && c.loop_start == 1;
            let d = base.join(format!("c{clip}"));
            std::fs::create_dir_all(&d).unwrap();
            let frames = c.frames as usize;
            for i in 0..n {
                let f = i * frames / n;
                // KH2 side: the emulator at the same KH2 time the bake used
                let kt = kh2_time(m, looped, frames, f) * m.keys_per_frame();
                let ki = (kt.round() as usize).min(samples - 1);
                let km: Vec<kh2_formats_lite::skeleton::Mat> = (0..model.bones.len()).map(|b| { let o = (ki * per + b) * 16; core::array::from_fn(|r| core::array::from_fn(|q| fl[o + r * 4 + q] as f64)) }).collect();
                let kw = |b: usize, l: [f64; 3]| aff(&km[b]).point(l);
                let khead = aff(&km[HEAD]).t;
                let kfwd = kw(HEAD, rigid_inv(&aff(&pose0[HEAD])).point(kh2_formats_lite::skeleton::point(&pose0[118], [0.0; 3])));
                let kf = facing(khead, kfwd);
                let mut kt_tris = vec![];
                for (gi, gr) in model.groups.iter().enumerate() {
                    for t in &gr.triangles {
                        let p = t.clone().map(|v| { let wb = kh2_formats_lite::skeleton::point(&pose0[v.bone as usize], v.position.map(|x| x as f64)); let l = rigid_inv(&aff(&pose0[v.bone as usize])).point(wb); let q = kf(kw(v.bone as usize, l)); [q[0] * s, q[1] * s, q[2] * s] });
                        if p.iter().any(|q| q[1] < -0.45) { continue; }
                        kt_tris.push(Tri { p, uv: Some(t.clone().map(|v| [v.uv[0] as f64, v.uv[1] as f64])), hair: gi == crate::kh2_hair::HAIR_GROUP });
                    }
                }
                // Re:CoM side
                let w = crate::kh2_hair::tests::posed_world(&sk, c, f as f64);
                let fh = sk.find("f_hair").unwrap();
                let rfwd = w[rhead].point(rigid_inv(&sk.rest_world()[rhead]).point([sk.rest_world()[rhead].t[0], sk.rest_world()[rhead].t[1], sk.rest_world()[rhead].t[2] + 0.1]));
                let _ = fh;
                let rf = facing(w[rhead].t, rfwd);
                let mut rt_tris = vec![];
                for t in dt::collect(&p3) {
                    if !t.v.iter().all(|v| keep(&names[v.bone()])) { continue; }
                    let p = t.v.clone().map(|v| rf(w[v.bone()].point(v.pos().map(|x| x as f64))));
                    rt_tris.push(Tri { p, uv: Some(t.v.clone().map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]))), hair: t.tex == 1 });
                }
                let ppm = 700.0;
                let panels = [raster(&kt_tris, &katlas, false, ppm), raster(&rt_tris, &ratlas, false, ppm), raster(&kt_tris, &katlas, true, ppm), raster(&rt_tris, &ratlas, true, ppm)];
                let mut img = vec![0u8; 4 * S * S * 3];
                for (k, pnl) in panels.iter().enumerate() {
                    for y in 0..S { img[3 * (y * 4 * S + k * S)..3 * (y * 4 * S + k * S + S)].copy_from_slice(&pnl[3 * y * S..3 * (y + 1) * S]); }
                }
                std::fs::write(d.join(format!("f{i:02}.png")), dt::png(4 * S, S, &img)).unwrap();
            }
        }
    }
}


#[cfg(test)]
mod face_tests {
    use super::*;
    use crate::ddd_hair::{with_hair, HairSource, RIKU_0101_KH2};
    use crate::install;

    /// 0.196.1 (owner's install, `RECOM_GAME`): the KH2 hair sits on Re:CoM's face at rest. At most 5 hair vertices of the head's
    /// front half lie more than 3 mm inside the face (0.195.4: 1-3, within 2.6 mm); on -x, where KH2's model has no ear, no face
    /// vertex of the ear region stands out past the flattening.
    #[test]
    fn kh2_hair_sits_on_the_face() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl0 = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = crate::kh2_hair::Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let mdl1 = src.prepare_mdl(&mdl0).unwrap();
        let swap = with_hair(&mdl1, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let sk = crate::mdl::RecomSkeleton::parse(&mdl1).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let world = sk.rest_world();
        let head = world[sk.find("head").unwrap()].t;
        let face = src.face_for_tests();
        // 0.196.3: the kept head triangles in the +x ear window are written twice (both facing flags); the count is the face's
        // triangles in that window, and small (the window is the ear's, not the head's).
        let in_window = face
            .iter()
            .filter(|t| crate::kh2_hair::in_ear_two_sided_window([0, 1, 2].map(|k| (t[0][k] + t[1][k] + t[2][k]) / 3.0), head))
            .count();
        eprintln!("kept head triangles written twice: {} (face triangles in the +x ear window: {in_window})", swap.report.face_two_sided);
        assert_eq!(swap.report.face_two_sided, in_window);
        assert!((1..=60).contains(&in_window), "{in_window} triangles in the ear window");
        let hair = src.hair(&names, &world).unwrap();
        let c = [head[0] + crate::ddd_hair::HEAD_CENTRE[0], head[1] + crate::ddd_hair::HEAD_CENTRE[1], head[2] + crate::ddd_hair::HEAD_CENTRE[2]];
        let mut inside = vec![];
        for t in &hair.tris {
            for v in &t.v {
                let r = [v.pos[0] - c[0], v.pos[1] - c[1], v.pos[2] - c[2]];
                let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
                let d = [r[0] / l, r[1] / l, r[2] / l];
                if d[2] < -0.2 { continue; }
                if let Some(f) = crate::kh2_hair::outermost(c, d, &face) {
                    if l < f - 0.003 { inside.push(f - l); }
                }
            }
        }
        inside.sort_by(|a, b| b.total_cmp(a));
        inside.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        eprintln!("hair vertices more than 3 mm inside the face: {} (deepest {:?})", inside.len(), inside.first());
        assert!(inside.len() <= 5, "{} hair vertices inside the face", inside.len());
        let out: Vec<[f64; 3]> = face.iter().flatten().copied().filter(|p| {
            let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
            r[0] < -crate::kh2_hair::EAR_FLAT_X - 1e-4 && (crate::kh2_hair::EAR_FLAT_Y[0]..=crate::kh2_hair::EAR_FLAT_Y[1]).contains(&r[1]) && (crate::kh2_hair::EAR_FLAT_Z[0]..=crate::kh2_hair::EAR_FLAT_Z[1]).contains(&r[2])
        }).collect();
        assert!(out.is_empty(), "the -x ear stands out: {out:?}");
    }
}
