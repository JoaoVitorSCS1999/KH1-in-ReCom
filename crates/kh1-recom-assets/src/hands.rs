//! Riku's right hand relaxed when his weapon is put away (0.198.10; fork owner, 2026-11-26: "ficou PERFEITO empunhando a espada,
//! mas sem espada na mão pode deixar o movimento das mãos mais natural, no andar corrida").
//!
//! - Where (Checked (renders `hand_grip_checks` with `NO_WEAPON`)): in the field walk and run (`p002_002`, `p002_003`) the right hand
//!   keeps the handle's hollow "C" with nothing in it; the left hand hangs relaxed.
//! - When those clips play (Checked (exe)): `0x1401F9380` picks rows 2 / 3 when player `+0x418` bit 1 is set, else 0x12 / 0x13 (and
//!   0x2B / 0x2C in the battle stance); the player's draw-weapon `0x1401F5D30` clears that bit (`and dword [+0x418], ~2` at
//!   `0x1401F5D4A`), Riku's put-away sets it (`or … 2` at `0x140216090`, just before his draw `0x1402160A0`). So rows 2 / 3 play only
//!   with the weapon put away. The field idle (row 1) also plays with the weapon out (`0x1401F9380`, cases 2 / 0x1F) and is left as it is.
//! - [`relax`]: in those clips, each of the 15 right-finger bones takes its left twin's rotation at the same frame, mirrored: with
//!   `P_b = Rw_r · S · Rw_lᵀ` from the rest pose (`S` the model's left-right mirror), the right local is `P_c · L_l · P_parent⁻¹`. The
//!   right hand itself, the arm and the rest of the body are untouched; the fingers move with the gait as the left hand's do.
//! - Clearance (model-reviewer of 0.198.10, no-go: on the Dark Form skin, run frame 11, the opened fingers went ~2 cm through the coat
//!   tail): per frame, the right-finger vertices may come no nearer the coat's triangles (bones `*_skt*`) than [`CLEAR`] or the
//!   clip's own distance, whichever is less; the mirror's weight is lowered in tenths where they would, then eased (the lowest weight
//!   within [`EASE`] frames, averaged over the same window), so the hand closes back toward the clip's own shape smoothly there. The
//!   re-review found a tip through the tail and out the far side, which the distance counts as clear: no finger vertex may change
//!   side of the coat either ([`Clearance::sides`]: the parity of the coat crossings from the wrist, against the clip's own pose).

use crate::ann::{Ann, Clip};
use crate::math::{euler_xyz, m3, rot_xyz, tr, unscale, M3};
use crate::mdl::{split_parts, MdlPart, RecomSkeleton};
use crate::Result;

/// The clips played only with the weapon put away (Riku's action rows 2 / 3, the field walk and run): slot, clip name suffix.
pub const SHEATHED: [(usize, &str); 2] = [(2, "_002"), (3, "_003")];

/// The least distance (metres) the relaxed fingers keep from the coat tails, unless the clip's own hand is nearer.
pub const CLEAR: f64 = 0.015;
/// Where the clip's own hand is nearer the coat than [`CLEAR`], the relaxed fingers still keep this much (metres; second re-review:
/// "no nearer than its own" at 3 mm let a tip through the tail at 1.2 mm).
pub const MIN_GAP: f64 = 0.006;
/// Frames on either side over which a lowered weight is eased.
pub const EASE: usize = 3;

thread_local! {
    /// Tests: builds without [`relax`] on this thread.
    pub static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// What [`relax`] did.
#[derive(Debug, Default, Clone)]
pub struct RelaxReport {
    /// Clip names changed.
    pub clips: Vec<String>,
    /// Per clip, the lowest eased weight of the mirror (1 = the left hand's shape in every frame).
    pub min_weight: Vec<f64>,
    /// The worst mismatch of the rest pose's mirror over the finger pairs (metres, world positions): the model's symmetry.
    pub asymmetry: f64,
}

fn finger_pairs(sk: &RecomSkeleton) -> Vec<(usize, usize)> {
    (1..=5).flat_map(|n| ["c", "b", "a"].map(|s| sk.find(&format!("l_fing{n}{s}")).zip(sk.find(&format!("r_fing{n}{s}"))))).flatten().collect()
}

/// Relaxes the right hand in [`SHEATHED`] clips of `ann` (a pack on `mdl`'s skeleton) by mirroring the left hand's fingers.
pub fn relax(ann: &mut Ann, mdl: &[u8]) -> Result<RelaxReport> {
    let mut rep = RelaxReport::default();
    if OFF.with(|x| x.get()) {
        return Ok(rep);
    }
    let sk = RecomSkeleton::parse(mdl)?;
    let pairs = finger_pairs(&sk);
    let (Some(lh), Some(rh)) = (sk.find("l_hand"), sk.find("r_hand")) else { return Ok(rep) };
    if pairs.len() != 15 {
        return Ok(rep);
    }
    let w = sk.rest_world();
    // the mirror axis: the coordinate in which the two hands' rest positions differ in sign
    let ax = (0..3).max_by(|&a, &b| (w[lh].t[a] - w[rh].t[a]).abs().total_cmp(&(w[lh].t[b] - w[rh].t[b]).abs())).unwrap_or(0);
    let mut s = crate::math::I3;
    s[ax][ax] = -1.0;
    let mirror = |p: [f64; 3]| { let mut q = p; q[ax] = -q[ax]; q };
    let rot = |b: usize| unscale(&w[b].r).0;
    let p_of = |l: usize, r: usize| -> M3 { m3(&m3(&rot(r), &s), &tr(&rot(l))) };
    let mut all = pairs.clone();
    all.push((lh, rh));
    for &(l, r) in &all {
        rep.asymmetry = rep.asymmetry.max(crate::math::dist(mirror(w[l].t), w[r].t));
    }
    let p: Vec<(M3, M3, usize, usize)> = pairs
        .iter()
        .map(|&(l, r)| {
            let par_r = sk.bones[r].parent as usize;
            let par_l = sk.bones[l].parent as usize;
            (p_of(l, r), tr(&p_of(par_l, par_r)), l, r)
        })
        .collect();
    let clear = Clearance::new(&sk, mdl)?;
    let near = |row: &[[f64; 9]]| clear.near(row);
    for (slot, suffix) in SHEATHED {
        let Some(c) = ann.slots.get(slot).and_then(|s| s.clip.as_ref()) else { continue };
        if c.bones as usize != sk.bones.len() || !c.name_str().ends_with(suffix) {
            continue;
        }
        let n = c.frames.max(1) as usize;
        let orig: Vec<Vec<[f64; 9]>> = (0..n).map(|f| row(c, f)).collect();
        let posed = |f: usize, k: f64| -> Vec<[f64; 9]> {
            let mut r = orig[f].clone();
            for (pc, pp_inv, l, b) in &p {
                let cur = rot_xyz(orig[f][*b][3], orig[f][*b][4], orig[f][*b][5]);
                let ll = rot_xyz(orig[f][*l][3], orig[f][*l][4], orig[f][*l][5]);
                let target = m3(&m3(pc, &ll), pp_inv);
                let m = if k >= 1.0 { target } else { crate::kh2_hair_rig::rot_toward(&cur, &target, k) };
                let e = euler_xyz(&m, Some([orig[f][*b][3], orig[f][*b][4], orig[f][*b][5]]));
                r[*b][3..6].copy_from_slice(&e);
            }
            r
        };
        // per frame, the largest weight (in tenths) that keeps the clearance
        let raw: Vec<f64> = (0..n)
            .map(|f| {
                let Some(d0) = near(&orig[f]) else { return 1.0 };
                // second re-review: near the clip's own contact (d0 a few mm) "no nearer than its own" still let a tip show through
                // the tail from the outer side (1.2 mm away); the relaxed shape keeps CLEAR or gives way to the clip's own hand
                let need = d0.max(MIN_GAP).min(CLEAR);
                let side0 = clear.sides(&orig[f]);
                let bad = |k: f64| { let r = posed(f, k); near(&r).is_some_and(|d| d < need) || clear.sides(&r) != side0 };
                let mut k: f64 = 1.0;
                while k > 0.0 && bad(k) {
                    k = ((k - 0.1) * 10.0).round() / 10.0;
                }
                k.max(0.0)
            })
            .collect();
        // eased: the lowest within EASE frames (the clip loops), then averaged over the same window (never above a frame's own)
        let at = |v: &[f64], i: isize| v[i.rem_euclid(n as isize) as usize];
        let win = EASE as isize;
        let low: Vec<f64> = (0..n as isize).map(|i| (-win..=win).map(|d| at(&raw, i + d)).fold(1.0, f64::min)).collect();
        let eased: Vec<f64> = (0..n as isize).map(|i| (-win..=win).map(|d| at(&low, i + d)).sum::<f64>() / (2 * win + 1) as f64).collect();
        // an in-between weight is not always within the limits where the full one is: each eased weight is checked too, lowered in
        // tenths where it fails (the clip's own hand, weight 0, always passes)
        let eased: Vec<f64> = (0..n)
            .map(|f| {
                let Some(d0) = near(&orig[f]) else { return eased[f] };
                // in between, the fingers pass from the clip's own hand to the relaxed one: no nearer the coat than either end allows
                let need = d0.max(MIN_GAP).min(CLEAR).min(d0);
                let side0 = clear.sides(&orig[f]);
                let ok = |k: f64| { if k <= 0.0 { return true; } let r = posed(f, k); near(&r).is_none_or(|d| d >= need) && clear.sides(&r) == side0 };
                let mut k = eased[f];
                while !ok(k) {
                    k = ((k - 0.1) * 10.0).floor() / 10.0;
                }
                k.max(0.0)
            })
            .collect();
        let rows: Vec<Vec<[f64; 9]>> = (0..n).map(|f| posed(f, eased[f])).collect();
        let enc = crate::riku_skin::encode_rows(c, &rows);
        rep.clips.push(c.name_str());
        rep.min_weight.push(eased.iter().copied().fold(1.0, f64::min));
        if std::env::var("HANDS_DEBUG").is_ok() {
            eprintln!("HANDS {}: weights {:?}", c.name_str(), eased.iter().map(|w| (w * 100.0).round() / 100.0).collect::<Vec<_>>());
        }
        if let Some(s) = ann.slots.get_mut(slot) {
            s.clip = Some(enc);
        }
    }
    Ok(rep)
}

/// The right fingers' distance to the coat tails (bones `*_skt*`) in a pose.
pub(crate) struct Clearance<'a> {
    sk: &'a RecomSkeleton,
    rh: usize,
    fingers: Vec<(usize, [f64; 3])>,
    coat: Vec<[(usize, [f64; 3]); 3]>,
}

impl<'a> Clearance<'a> {
    pub(crate) fn new(sk: &'a RecomSkeleton, mdl: &[u8]) -> Result<Clearance<'a>> {
        let fset: Vec<usize> = finger_pairs(sk).iter().map(|p| p.1).collect();
        let body: Vec<MdlPart> = split_parts(mdl)?.iter().filter_map(|pb| MdlPart::parse_bytes(pb).ok()).filter(|p| p.textures.len() > 1).collect();
        let fingers = body.iter().flat_map(|p| p.list_a.iter().flat_map(|k| k.verts.iter())).filter(|v| fset.contains(&v.bone())).map(|v| (v.bone(), v.pos().map(|x| x as f64))).collect();
        let coat = body
            .iter()
            .flat_map(|p| {
                let mut t = crate::ddd_hair::extract(&p.list_a).unwrap_or_default();
                if let Some(lb) = p.list_b.as_ref() {
                    t.extend(crate::ddd_hair::extract(lb).unwrap_or_default());
                }
                t
            })
            .filter(|t| t.v.iter().any(|v| sk.bones[v.bone()].name.contains("_skt")))
            .map(|t| t.v.map(|v| (v.bone(), v.pos().map(|x| x as f64))))
            .collect();
        Ok(Clearance { sk, rh: sk.find("r_hand").unwrap_or(0), fingers, coat })
    }

    /// The least distance (metres) of a right-finger vertex to a coat triangle within 30 cm of the hand; None without one.
    pub(crate) fn near(&self, row: &[[f64; 9]]) -> Option<f64> {
        if self.coat.is_empty() {
            return None;
        }
        let w = world(self.sk, row);
        let fp: Vec<[f64; 3]> = self.fingers.iter().map(|(b, p)| w[*b].point(*p)).collect();
        let hp = w[self.rh].t;
        let mut best: Option<f64> = None;
        for t in &self.coat {
            let q = t.map(|(b, p)| w[b].point(p));
            if q.iter().all(|x| crate::math::dist(*x, hp) > 0.3) {
                continue;
            }
            for p in &fp {
                let d = point_tri(*p, q);
                best = Some(best.map_or(d, |b: f64| b.min(d)));
            }
        }
        best
    }

    /// Per right-finger vertex, whether the segment from `r_hand`'s origin to it crosses the coat an odd number of times (re-review of
    /// 0.198.10: an unsigned distance let a tip that had gone through the coat tail and out past [`CLEAR`] count as clear; a vertex
    /// whose parity differs from the clip's own pose is on the other side of the coat).
    pub(crate) fn sides(&self, row: &[[f64; 9]]) -> Vec<bool> {
        let w = world(self.sk, row);
        let hp = w[self.rh].t;
        let tris: Vec<[[f64; 3]; 3]> = self.coat.iter().map(|t| t.map(|(b, p)| w[b].point(p))).filter(|q| q.iter().any(|x| crate::math::dist(*x, hp) <= 0.3)).collect();
        self.fingers.iter().map(|(b, p)| { let v = w[*b].point(*p); tris.iter().filter(|q| seg_tri(hp, v, **q)).count() % 2 == 1 }).collect()
    }
}

/// Whether the segment `a`-`b` crosses the triangle `t` (Möller-Trumbore).
fn seg_tri(a: [f64; 3], b: [f64; 3], t: [[f64; 3]; 3]) -> bool {
    let sub = |x: [f64; 3], y: [f64; 3]| [x[0] - y[0], x[1] - y[1], x[2] - y[2]];
    let dot = |x: [f64; 3], y: [f64; 3]| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
    let cross = |x: [f64; 3], y: [f64; 3]| [x[1] * y[2] - x[2] * y[1], x[2] * y[0] - x[0] * y[2], x[0] * y[1] - x[1] * y[0]];
    let d = sub(b, a);
    let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
    let h = cross(d, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-12 {
        return false;
    }
    let inv = 1.0 / det;
    let s = sub(a, t[0]);
    let u = dot(s, h) * inv;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let k = dot(e2, q) * inv;
    (0.0..=1.0).contains(&k)
}

fn world(sk: &RecomSkeleton, row: &[[f64; 9]]) -> Vec<crate::math::Aff> {
    let mut w: Vec<crate::math::Aff> = Vec::with_capacity(sk.bones.len());
    for (i, b) in sk.bones.iter().enumerate() {
        let r = &row[i];
        let mut m = rot_xyz(r[3], r[4], r[5]);
        for (line, sc) in m.iter_mut().zip([r[6], r[7], r[8]]) {
            for v in line.iter_mut() {
                *v *= sc;
            }
        }
        let l = crate::math::Aff { r: m, t: [r[0], r[1], r[2]] };
        w.push(if b.parent < 0 { l } else { l.mul(&w[b.parent as usize]) });
    }
    w
}

/// The distance from `p` to the triangle `t` (closest point on the triangle; Ericson, Real-Time Collision Detection, 5.1.5).
fn point_tri(p: [f64; 3], t: [[f64; 3]; 3]) -> f64 {
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let add = |a: [f64; 3], b: [f64; 3], k: f64| [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k];
    let [a, b, c] = t;
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    let q = if d1 <= 0.0 && d2 <= 0.0 {
        a
    } else {
        let bp = sub(p, b);
        let (d3, d4) = (dot(ab, bp), dot(ac, bp));
        let cp = sub(p, c);
        let (d5, d6) = (dot(ab, cp), dot(ac, cp));
        let (vc, vb, va) = (d1 * d4 - d3 * d2, d5 * d2 - d1 * d6, d3 * d6 - d5 * d4);
        if d3 >= 0.0 && d4 <= d3 {
            b
        } else if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
            add(a, ab, d1 / (d1 - d3))
        } else if d6 >= 0.0 && d5 <= d6 {
            c
        } else if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
            add(a, ac, d2 / (d2 - d6))
        } else if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
            add(b, sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)))
        } else {
            let den = 1.0 / (va + vb + vc);
            add(add(a, ab, vb * den), ac, vc * den)
        }
    };
    crate::math::dist(p, q)
}

pub(crate) fn row(c: &Clip, f: usize) -> Vec<[f64; 9]> {
    (0..c.bones as usize).map(|b| { let x = c.sample(b, f as f64); [x.t[0], x.t[1], x.t[2], x.r[0], x.r[1], x.r[2], x.s[0], x.s[1], x.s[2]] }).collect()
}

#[cfg(test)]
mod tests {
    /// The relaxed right fingers, in `r_hand`'s frame, are the left fingers in `l_hand`'s frame mirrored through the rest pose (worst
    /// over every frame of the field walk and run under 1 mm); only the right fingers change.
    #[test]
    fn relaxed_right_hand_mirrors_the_left() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = crate::install::keys(&g).unwrap();
        let arc = crate::install::recom_archive(&g).unwrap();
        let mdl = crate::install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let orig = crate::ann::Ann::parse(&crate::install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let mut ann = orig.clone();
        let rep = super::relax(&mut ann, &mdl).unwrap();
        assert_eq!(rep.clips, ["p002_002", "p002_003"]);
        assert!(rep.asymmetry < 1e-4, "the rest pose is not mirror-symmetric: {}", rep.asymmetry);
        let sk = crate::mdl::RecomSkeleton::parse(&mdl).unwrap();
        let rw = sk.rest_world();
        let (lh, rh) = (sk.find("l_hand").unwrap(), sk.find("r_hand").unwrap());
        let ax = (0..3).max_by(|&a, &b| (rw[lh].t[a] - rw[rh].t[a]).abs().total_cmp(&(rw[lh].t[b] - rw[rh].t[b]).abs())).unwrap();
        let rigid = |a: &crate::math::Aff| crate::math::Aff { r: crate::math::unscale(&a.r).0, t: a.t };
        let pairs = super::finger_pairs(&sk);
        for (slot, _) in super::SHEATHED {
            let c = ann.slots[slot].clip.as_ref().unwrap();
            let o = orig.slots[slot].clip.as_ref().unwrap();
            let mut worst: f64 = 0.0;
            let mut body: f64 = 0.0;
            for f in 0..c.frames as usize {
                let w = crate::kh2_hair::tests::posed_world(&sk, c, f as f64);
                let wo = crate::kh2_hair::tests::posed_world(&sk, o, f as f64);
                let inv = |h: usize| crate::kh2_hair_rig::rigid_inv(&rigid(&w[h]));
                for &(l, r) in &pairs {
                    // the left finger carried to the rest pose's left hand, mirrored, read in the rest pose's right hand
                    let mut m = rigid(&rw[lh]).point(inv(lh).point(w[l].t));
                    m[ax] = -m[ax];
                    let pm = crate::kh2_hair_rig::rigid_inv(&rigid(&rw[rh])).point(m);
                    worst = worst.max(crate::math::dist(pm, inv(rh).point(w[r].t)));
                }
                for b in (0..sk.bones.len()).filter(|b| !pairs.iter().any(|p| p.1 == *b)) {
                    body = body.max(crate::math::dist(w[b].t, wo[b].t));
                }
            }
            assert!(worst < 1e-3, "slot {slot}: right finger {:.2} mm off the mirrored left", worst * 1000.0);
            assert!(body < 1e-3, "slot {slot}: a bone other than the right fingers moved {:.2} mm", body * 1000.0);
        }
    }

    /// The Dark Form skin (Riku's clips on Dark Riku's model, coat tails animated): the relaxed fingers keep [`super::CLEAR`] from the
    /// coat, or the clip's own distance where that is less (model-reviewer of 0.198.10: run frame 11 went through the coat tail).
    #[test]
    fn relaxed_skin_hand_keeps_clear_of_the_coat() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = crate::install::keys(&g).unwrap();
        let arc = crate::install::recom_archive(&g).unwrap();
        let mdl = crate::install::read_recom(&arc, &keys, "PLAYER/0201/PL0201.MDL").unwrap();
        let path = crate::riku_skin::path(121, "ANN");
        // before: the build's pack without the relax (the 0.198.9 grip on); after: the build's pack
        let pack = |off: bool| {
            super::OFF.with(|x| x.set(off));
            let b = crate::riku_skin::build_with(&arc, &keys, 101, 201, 121, None).unwrap();
            super::OFF.with(|x| x.set(false));
            crate::ann::Ann::parse(&b.files.iter().find(|f| f.0 == path).unwrap().1).unwrap()
        };
        let (before, after) = (pack(true), pack(false));
        let sk = crate::mdl::RecomSkeleton::parse(&mdl).unwrap();
        let d = super::Clearance::new(&sk, &mdl).unwrap();
        // the relaxed right fingers turn no faster than the left ones they mirror (+ 2 deg/frame)
        let ang = |a: &crate::math::M3, b: &crate::math::M3| { let d = crate::math::m3(&crate::math::tr(a), b); ((d[0][0] + d[1][1] + d[2][2] - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees() };
        let rot = |c: &crate::ann::Clip, b: usize, f: usize| { let r = c.sample(b, f as f64).r; crate::math::rot_xyz(r[0], r[1], r[2]) };
        let pairs = super::finger_pairs(&sk);
        for (slot, _) in super::SHEATHED {
            let c = after.slots[slot].clip.as_ref().unwrap();
            let n = c.frames.max(1) as usize;
            let speed = |side: usize| (1..n).map(|f| pairs.iter().map(|p| { let b = if side == 0 { p.0 } else { p.1 }; ang(&rot(c, b, f - 1), &rot(c, b, f)) }).fold(0.0, f64::max)).fold(0.0, f64::max);
            let (l, r) = (speed(0), speed(1));
            assert!(r <= l + 2.0, "slot {slot}: right fingers {r:.1} deg/frame against the left's {l:.1}");
        }
        for (slot, _) in super::SHEATHED {
            let (o, c) = (before.slots[slot].clip.as_ref().unwrap(), after.slots[slot].clip.as_ref().unwrap());
            for f in 0..c.frames as usize {
                let (Some(d0), Some(d1)) = (d.near(&super::row(o, f)), d.near(&super::row(c, f))) else { continue };
                let (s0, s1) = (d.sides(&super::row(o, f)), d.sides(&super::row(c, f)));
                let flipped = s0.iter().zip(&s1).filter(|(a, b)| a != b).count();
                assert!(flipped == 0, "slot {slot} frame {f}: {flipped} finger vertices on the other side of the coat");
                // the relaxed shape keeps CLEAR; where it gives way the hand is the clip's own (its own distance)
                let changed = (0..c.bones as usize).any(|b| (0..3).any(|k| (super::row(o, f)[b][3 + k] - super::row(c, f)[b][3 + k]).abs() > 1e-3));
                assert!(!changed || d1 >= d0.min(super::CLEAR) - 1e-3, "slot {slot} frame {f}: fingers {:.1} mm from the coat (own {:.1})", d1 * 1000.0, d0 * 1000.0);
            }
        }
    }
}
