//! KH2 Riku's hair on Re:CoM Riku's head (the hair option `riku_hair = kh2`; fork owner, 2026-11-24: "o cabelo começa sim pelo do
//! Riku do KH2"). It needs only the 1.5+2.5 collection (KH2 FM's `obj/P_EH000.mdlx` and its remastered hair atlas), the swap
//! itself is [`crate::ddd_hair::with_hair`].
//!
//! Source (notes: the workspace's `knowledge/KH2/kh2-riku-hair-models.md`): `P_EH000.mdlx`'s group 3 is the hair alone (613
//! triangles, one bone per vertex, 27 bones), drawn with texture 3, whose remastered DDS is the asset `-5.dds` (1024 x 1024).
//! Checked (files, that note).
//!
//! Fit (Assumption until seen in game; the renders in `local/assets/recom/riku-hair/out/kh2/` are the check):
//! - Frame: a KH2 vertex's bind world position `W` becomes `H + s * (W - K)` where `K` is KH2's head bone (98) and `H` Re:CoM's
//!   `head` bone, both at rest; `s` is [`SCALE`], the head-size ratio of the note (0.0086, two heights compared).
//! - Bones: KH2's cap bones (98, 101, 104) follow Re:CoM's `head` (rigid with the skull). Every other KH2 bone takes the Re:CoM
//!   bone (`head` or one with "hair" in its name) nearest to the KH2 bone's own rest position in that frame, so a chain keeps one
//!   Re:CoM bone and its strands move together. This is the 0.188.0 look, the one the owner liked ("ficou MUITO bom o cabelo do
//!   KH2"); 0.188.1 to 0.191.0 tried a rigid head binding, one sway bone, neck-bone bands and KH2's own motion baked into the
//!   hair bones, and the owner preferred this one ("a primeira versão é a melhor até agora"): Stage 363 went back to it.
//! - Width: [`WIDTH`] stretches x a little, Re:CoM's head is wider than KH2's at the temples.

use std::collections::HashMap;

use kh2_formats_lite::{mdlx, skeleton};

use crate::ddd_hair::{HairSource, SrcHair, SrcTri, SrcVert, HEAD_BONE};
use crate::math::Aff;
use crate::texture::decode_dds;
use crate::{bad, Error, Result};

/// KH2's model file of Riku (kh2_sixth). Checked (files).
pub const RIKU_MDLX: &str = "obj/P_EH000.mdlx";
/// The group that holds the hair. Checked (files).
pub const HAIR_GROUP: usize = 3;
/// The hair atlas: the remastered DDS of PS2 texture 3. Checked (files).
pub const ATLAS_ASSET: &str = "-5.dds";
/// KH2's head bone and the cap bones that follow Re:CoM's head. Checked (files: 752 + 183 + 98 of 1839 corners).
pub const HEAD_BONE_KH2: usize = 98;
/// Cap bones of the hair (the rigid skull part).
pub const CAP_BONES: [usize; 3] = [98, 101, 104];
/// KH2 units to Re:CoM metres (the note: 0.0086, hair width +-16.5 against +-0.142).
pub const SCALE: f64 = 0.0092;
/// Extra width factor (x only): Re:CoM's head is wider than KH2's at the temples.
pub const WIDTH: f64 = 1.1;
/// Offset added after the head alignment, in Re:CoM metres (x, y, z), fitted on the renders (see [`SHIFT`]).
pub const SHIFT: [f64; 3] = [0.0, 0.0, 0.012];

/// The face fit (0.195.4; fork owner on 0.195.3: "umas das mechas que marquei estão entrando na bochecha, e ainda aparece ta estranho
/// a marcação que fiz", with a picture of KH2's PS2 Riku: the ear shows, the front locks before it, the long hair behind). For each
/// direction from the head's centre, the outermost surface of Re:CoM's face and of KH2's (its head-bone skin, carried by the same
/// fit as the hair) are found by ray; a hair vertex moves along its direction by the difference, averaged over the directions
/// within [`FIT_SIGMA_DEG`] and faded where only one face exists (Re:CoM's back of the head is gone with his hair), held within
/// [`FIT_MIN`]..[`FIT_MAX`] (metres). Replaces 0.195.3's ear push.
pub const FIT_SIGMA_DEG: f64 = 10.0;
/// See [`FIT_SIGMA_DEG`].
pub const FIT_MIN: f64 = -0.004;
/// See [`FIT_SIGMA_DEG`].
pub const FIT_MAX: f64 = 0.025;
/// Angular step of the sampled directions (degrees).
pub const FIT_STEP_DEG: f64 = 3.0;

/// Outermost hit of the ray from `c` along unit `d` with `tris` (distance), if any.
pub fn outermost(c: [f64; 3], d: [f64; 3], tris: &[[[f64; 3]; 3]]) -> Option<f64> {
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [f64; 3], b: [f64; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let mut best: Option<f64> = None;
    for t in tris {
        let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
        let p = cross(d, e2);
        let det = dot(e1, p);
        if det.abs() < 1e-12 {
            continue;
        }
        let s = sub(c, t[0]);
        let u = dot(s, p) / det;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = cross(s, e1);
        let v = dot(d, q) / det;
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        let dist = dot(e2, q) / det;
        if dist > 0.0 && best.is_none_or(|b| dist > b) {
            best = Some(dist);
        }
    }
    best
}

/// The face fit: sampled directions with Re:CoM's minus KH2's face radius.
#[derive(Clone, Debug, Default)]
pub struct FaceFit {
    /// The head's centre (world, rest).
    pub c: [f64; 3],
    /// `(direction, difference)`.
    pub samples: Vec<([f64; 3], f64)>,
}

impl FaceFit {
    /// From the two faces (world rest triangles) about centre `c`.
    pub fn new(c: [f64; 3], recom: &[[[f64; 3]; 3]], kh2: &[[[f64; 3]; 3]]) -> FaceFit {
        let mut samples = vec![];
        let step = FIT_STEP_DEG.to_radians();
        let mut el = -70f64.to_radians();
        while el <= 80f64.to_radians() {
            let n = ((2.0 * std::f64::consts::PI * el.cos()) / step).ceil().max(1.0) as usize;
            for i in 0..n {
                let az = i as f64 * 2.0 * std::f64::consts::PI / n as f64;
                let d = [el.cos() * az.sin(), el.sin(), el.cos() * az.cos()];
                if let (Some(a), Some(b)) = (outermost(c, d, recom), outermost(c, d, kh2)) {
                    samples.push((d, a - b));
                }
            }
            el += step;
        }
        FaceFit { c, samples }
    }

    /// How far a point moves along its direction from the centre.
    pub fn offset(&self, p: [f64; 3]) -> f64 {
        let r = [p[0] - self.c[0], p[1] - self.c[1], p[2] - self.c[2]];
        let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        if l < 1e-9 || self.samples.is_empty() {
            return 0.0;
        }
        let d = [r[0] / l, r[1] / l, r[2] / l];
        let s2 = 2.0 * FIT_SIGMA_DEG.to_radians().powi(2);
        let (mut wsum, mut vsum, mut full) = (0.0, 0.0, 0.0);
        // the weight a fully covered neighbourhood gets (samples spaced FIT_STEP_DEG): its sum, to fade where coverage is partial
        let cell = FIT_STEP_DEG.to_radians().powi(2);
        for (q, v) in &self.samples {
            let a = (d[0] * q[0] + d[1] * q[1] + d[2] * q[2]).clamp(-1.0, 1.0).acos();
            let w = (-a * a / s2).exp();
            wsum += w;
            vsum += w * v;
        }
        full += std::f64::consts::PI * s2 / cell;
        if wsum < 1e-9 {
            return 0.0;
        }
        let cover = (wsum / full).min(1.0);
        (vsum / wsum * cover).clamp(FIT_MIN, FIT_MAX)
    }

    /// The point moved.
    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        let o = self.offset(p);
        if o == 0.0 {
            return p;
        }
        let r = [p[0] - self.c[0], p[1] - self.c[1], p[2] - self.c[2]];
        let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        [p[0] + r[0] / l * o, p[1] + r[1] / l * o, p[2] + r[2] / l * o]
    }
}

/// The ears (0.196.1; fork owner on 0.196.0: "não resolveu o problemna da foto que te mandei da orelha e da mexa", with KH2's
/// PS2 model as reference). Checked (files: KH2's head-bone skin carried by the hair's fit, Re:CoM's kept face; both head-relative,
/// metres): KH2's model has an ear on the +x side only (26 skin vertices past |x| 0.085 there, centre y 0.074 z 0.015; on -x its
/// hair covers the side and its skin has none), Re:CoM has both (centre y 0.063 z -0.001). So on -x Re:CoM's ear is flattened
/// under the hair ([`EAR_FLAT_X`]), and on +x the hair around KH2's ear moves by Re:CoM's ear minus KH2's ([`EAR_SHIFT`]) so that
/// the gap KH2's ear sits in lands on Re:CoM's.
pub const EAR_FLAT_X: f64 = 0.085;
/// The ear region flattened on -x (head-relative y, z ranges, metres).
pub const EAR_FLAT_Y: [f64; 2] = [0.0, 0.125];
/// See [`EAR_FLAT_Y`].
pub const EAR_FLAT_Z: [f64; 2] = [-0.09, 0.04];
/// KH2's ear centre on +x (head-relative y, z) and the shift of the hair around it (y, z).
pub const KH2_EAR: [f64; 2] = [0.074, 0.015];
/// See [`KH2_EAR`].
pub const EAR_SHIFT: [f64; 2] = [-0.011, -0.016];
/// The shift's reach (Gaussian sigma over y, z, metres).
pub const EAR_SHIFT_REACH: f64 = 0.035;

/// The +x temple / ear window whose kept head triangles are written twice, with both facing flags (0.196.3; fork owner on
/// 0.196.2: "um pequeno ponto que fica transparente mostrando o fundo do cenário" at the top of the ear's gap). Checked (render
/// `local/work/holefind/out/cull_one_sign_ear_f0.png`, the pinhole finder with one facing sign kept): behind that point there
/// is one single-copy head triangle (centroid x +0.09 from the head bone, 'head, head, head', facing flag 0) that a facing
/// cull removes from some views; the hair's own triangles are all written twice. Assumption: that is the point the owner sees
/// (not checked in game). Head-relative metres: x from [`EAR_TWO_SIDED_X`], y and z the ear's [`EAR_FLAT_Y`] / [`EAR_FLAT_Z`].
pub const EAR_TWO_SIDED_X: f64 = 0.06;

/// Whether a head triangle centred at `p` (world rest; `head` = the head bone's at rest) lies in the +x ear window.
pub fn in_ear_two_sided_window(p: [f64; 3], head: [f64; 3]) -> bool {
    let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
    r[0] > EAR_TWO_SIDED_X && (EAR_FLAT_Y[0]..=EAR_FLAT_Y[1]).contains(&r[1]) && (EAR_FLAT_Z[0]..=EAR_FLAT_Z[1]).contains(&r[2])
}

/// A hair rest position with the +x ear gap moved to Re:CoM's ear.
pub fn ear_shift(p: [f64; 3], head: [f64; 3]) -> [f64; 3] {
    let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
    if r[0] < 0.03 {
        return p;
    }
    let d2 = (r[1] - KH2_EAR[0]).powi(2) + (r[2] - KH2_EAR[1]).powi(2);
    let w = (-d2 / (2.0 * EAR_SHIFT_REACH * EAR_SHIFT_REACH)).exp();
    [p[0], p[1] + EAR_SHIFT[0] * w, p[2] + EAR_SHIFT[1] * w]
}

/// KH2 Riku's motion set (0.195.0: the hair's own motion). Checked (files: kh2_sixth).
pub const RIKU_MSET: &str = "obj/P_EH000.mset";

/// KH2's `P_EH000.mdlx` and its hair atlas.
pub struct Kh2HairSource {
    /// The `.mdlx`.
    pub mdlx: Vec<u8>,
    /// `-5.dds`.
    pub dds: Vec<u8>,
    /// `P_EH000.mset` (none: the hair follows Re:CoM's own hair bones as in 0.188.0).
    pub mset: Option<Vec<u8>>,
    /// The rig, made from Re:CoM's original skeleton by [`HairSource::prepare_mdl`].
    rig: std::cell::OnceCell<crate::kh2_hair_rig::Rig>,
    /// Re:CoM's face that stays, from [`HairSource::set_face`].
    face: std::cell::RefCell<Vec<[[f64; 3]; 3]>>,
}

impl Kh2HairSource {
    /// From the player's KH2 archives.
    pub fn read(inst: &kh1_formats::egs::Install) -> Result<Kh2HairSource> {
        let (mdlx, assets, _) = inst.read_full(RIKU_MDLX)?;
        let dds = assets.into_iter().find(|a| a.name == ATLAS_ASSET).ok_or_else(|| Error::Format(format!("KH2: {RIKU_MDLX} has no {ATLAS_ASSET}")))?.data;
        let mset = inst.read_full(RIKU_MSET).ok().map(|x| x.0);
        Ok(Kh2HairSource { mdlx, dds, mset, rig: Default::default(), face: Default::default() })
    }

    /// From the files themselves.
    pub fn new(mdlx: Vec<u8>, dds: Vec<u8>, mset: Option<Vec<u8>>) -> Kh2HairSource {
        Kh2HairSource { mdlx, dds, mset, rig: Default::default(), face: Default::default() }
    }

    /// The face [`HairSource::set_face`] was given (tests).
    pub fn face_for_tests(&self) -> Vec<[[f64; 3]; 3]> {
        self.face.borrow().clone()
    }

    /// The KH2 motions the rig uses, by `.mset` slot.
    pub fn motions(&self) -> Result<HashMap<usize, crate::kh2_motion::Kh2Motion>> {
        use crate::kh2_hair_rig as r;
        let mut out = HashMap::new();
        if let Some(m) = &self.mset {
            for s in [r::KH2_IDLE, r::KH2_WALK, r::KH2_RUN, r::KH2_JUMP, r::KH2_FALL] {
                if let Some(x) = crate::kh2_motion::read_slot(m, s)? {
                    out.insert(s, x);
                }
            }
        }
        Ok(out)
    }

    /// The rig for Re:CoM's original model `mdl` (made once).
    pub fn rig(&self, mdl: &[u8]) -> Result<Option<&crate::kh2_hair_rig::Rig>> {
        use crate::kh2_hair_rig as r;
        if let Some(x) = self.rig.get() {
            return Ok(Some(x));
        }
        let motions = self.motions()?;
        if !motions.contains_key(&r::KH2_IDLE) {
            return Ok(None);
        }
        let model = mdlx::parse(&self.mdlx).map_err(|e| Error::Format(format!("KH2 {RIKU_MDLX}: {e}")))?;
        let sk = crate::mdl::RecomSkeleton::parse(mdl)?;
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        let world = sk.rest_world();
        let head = sk.find(HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let pose = skeleton::bind_pose(&model.bones);
        let carry = r::Carry { k: r::aff(&pose[HEAD_BONE_KH2]).r, h: crate::math::unscale(&world[head].r).0, c: [WIDTH * SCALE, SCALE, SCALE], shift: SHIFT };
        let on: Vec<&crate::kh2_motion::Kh2Motion> = r::CHOOSE_ON.iter().filter_map(|s| motions.get(s)).collect();
        let host = r::hosts(&model, HAIR_GROUP, &on, &r::SELECTED);
        let rig = r::Rig::new(&names, &world, &model, carry, host)?;
        Ok(Some(self.rig.get_or_init(|| rig)))
    }
}

fn dist2(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum()
}

impl HairSource for Kh2HairSource {
    fn set_face(&self, face: &[[[f64; 3]; 3]]) {
        *self.face.borrow_mut() = face.to_vec();
    }

    fn adjust_face(&self, p: [f64; 3], head: [f64; 3]) -> [f64; 3] {
        let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
        if r[0] < -EAR_FLAT_X && (EAR_FLAT_Y[0]..=EAR_FLAT_Y[1]).contains(&r[1]) && (EAR_FLAT_Z[0]..=EAR_FLAT_Z[1]).contains(&r[2]) {
            return [head[0] - EAR_FLAT_X, p[1], p[2]];
        }
        p
    }

    fn two_sided_face(&self, p: [f64; 3], head: [f64; 3]) -> bool {
        in_ear_two_sided_window(p, head)
    }

    fn prepare_mdl(&self, mdl: &[u8]) -> Result<Vec<u8>> {
        use crate::kh2_hair_rig as r;
        let Some(rig) = self.rig(mdl)? else { return Ok(mdl.to_vec()) };
        let model = mdlx::parse(&self.mdlx).map_err(|e| Error::Format(format!("KH2 {RIKU_MDLX}: {e}")))?;
        let sk = crate::mdl::RecomSkeleton::parse(mdl)?;
        let parents: Vec<i16> = sk.bones.iter().map(|b| b.parent).collect();
        let head = sk.find(HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let rest = rig.locals(&parents, head, &rig.head_rel(&r::rel(&skeleton::bind_pose(&model.bones))));
        let mut out = mdl.to_vec();
        r::patch_rest(&mut out, &rest)?;
        Ok(out)
    }

    fn bake_ann(&self, ann: &[u8], mdl: &[u8]) -> Result<Option<Vec<u8>>> {
        let Some(rig) = self.rig(mdl)? else { return Ok(None) };
        let model = mdlx::parse(&self.mdlx).map_err(|e| Error::Format(format!("KH2 {RIKU_MDLX}: {e}")))?;
        let sk = crate::mdl::RecomSkeleton::parse(mdl)?;
        let head = sk.find(HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let mut a = crate::ann::Ann::parse(ann)?;
        crate::kh2_hair_rig::bake(&mut a, rig, &sk, head, &model, &self.motions()?)?;
        Ok(Some(a.to_bytes()))
    }

    fn hair(&self, names: &[String], world: &[Aff]) -> Result<SrcHair> {
        let model = mdlx::parse(&self.mdlx).map_err(|e| Error::Format(format!("KH2 {RIKU_MDLX}: {e}")))?;
        let pose = skeleton::bind_pose(&model.bones);
        let group = model.groups.get(HAIR_GROUP).ok_or_else(|| Error::Format(format!("KH2 {RIKU_MDLX}: no group {HAIR_GROUP}")))?;
        if group.texture != 3 || pose.len() <= HEAD_BONE_KH2 {
            return bad(format!("KH2 {RIKU_MDLX}: group {HAIR_GROUP} is texture {} with {} bones, not the hair", group.texture, pose.len()));
        }
        let img = decode_dds(&self.dds)?;
        let head = names.iter().position(|n| n == HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
        let k = skeleton::point(&pose[HEAD_BONE_KH2], [0.0; 3]);
        let h = world[head].t;
        let frame = |w: [f64; 3]| [h[0] + WIDTH * SCALE * (w[0] - k[0]) + SHIFT[0], h[1] + SCALE * (w[1] - k[1]) + SHIFT[1], h[2] + SCALE * (w[2] - k[2]) + SHIFT[2]];
        // Re:CoM's candidate bones: the head and every hair bone, at rest.
        let cands: Vec<usize> = names.iter().enumerate().filter(|(i, n)| *i == head || n.contains("hair")).map(|(i, _)| i).collect();
        let mut bone_name: HashMap<u16, String> = HashMap::new();
        let rig = self.rig.get();
        // the face fit: KH2's head skin (non-hair groups, vertices on the head bone) carried by the same fit
        let fit = {
            let rf = self.face.borrow();
            let mut kf = vec![];
            for (gi, g) in model.groups.iter().enumerate() {
                if gi == HAIR_GROUP {
                    continue;
                }
                for t in &g.triangles {
                    if t.iter().all(|v| v.bone as usize == HEAD_BONE_KH2) {
                        kf.push(t.clone().map(|v| frame(skeleton::point(&pose[v.bone as usize], v.position.map(|x| x as f64)))));
                    }
                }
            }
            let c = [h[0] + crate::ddd_hair::HEAD_CENTRE[0], h[1] + crate::ddd_hair::HEAD_CENTRE[1], h[2] + crate::ddd_hair::HEAD_CENTRE[2]];
            if rf.is_empty() || kf.is_empty() { FaceFit::default() } else { FaceFit::new(c, &rf, &kf) }
        };
        let mut name_of = |b: u16| -> String {
            bone_name
                .entry(b)
                .or_insert_with(|| {
                    if let Some(rig) = rig {
                        // 0.195.0: the rig's bones (KH2's own motion)
                        let b = b as usize;
                        if b == crate::kh2_hair_rig::NECK {
                            return crate::kh2_hair_rig::NECK_RECOM.to_string();
                        }
                        return rig.recom_of(b).map_or(HEAD_BONE.to_string(), |r| names[r].clone());
                    }
                    if CAP_BONES.contains(&(b as usize)) {
                        return HEAD_BONE.to_string();
                    }
                    let p = frame(skeleton::point(&pose[b as usize], [0.0; 3]));
                    let best = cands.iter().min_by(|&&x, &&y| dist2(world[x].t, p).total_cmp(&dist2(world[y].t, p))).copied().unwrap_or(head);
                    names[best].clone()
                })
                .clone()
        };
        let q = |x: f32| (x as f64 * 4096.0).round() as i32;
        let corners: Vec<[(i32, i32); 3]> = group.triangles.iter().map(|t| t.map(|v| (q(v.uv[0]), q(v.uv[1])))).collect();
        let (isl, n) = crate::ddd_hair::islands(&corners);
        let mut tris = Vec::with_capacity(group.triangles.len());
        for (t, &island) in group.triangles.iter().zip(&isl) {
            let mk = |v: &mdlx::Vertex, bn: &mut dyn FnMut(u16) -> String| SrcVert {
                pos: fit.apply(ear_shift(frame(skeleton::point(&pose[v.bone as usize], v.position.map(|x| x as f64))), h)),
                uv: [v.uv[0] as f64, v.uv[1] as f64],
                bone: bn(v.bone),
            };
            tris.push(SrcTri { v: [mk(&t[0], &mut name_of), mk(&t[1], &mut name_of), mk(&t[2], &mut name_of)], island, cont: false });
        }
        Ok(SrcHair { tris, islands: n, skin: 0, atlas: img, one_sided: false })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ddd_hair::{tests as dt, with_hair, RIKU_0101_KH2};
    use crate::install;
    use crate::mdl::{split_parts, MdlPart, RecomSkeleton};

    /// 0.196.3: the two-sided window is the +x ear's (head-relative), not the -x side, the front hairline or the neck.
    #[test]
    fn ear_two_sided_window_is_the_plus_x_ear() {
        let head = [0.0, 1.40, 0.0];
        let at = |x: f64, y: f64, z: f64| [head[0] + x, head[1] + y, head[2] + z];
        assert!(in_ear_two_sided_window(at(0.09, 0.063, -0.001), head), "Re:CoM's ear centre on +x");
        assert!(in_ear_two_sided_window(at(0.095, 0.06, -0.045), head), "the temple triangle behind the point (centroid x +0.09)");
        assert!(!in_ear_two_sided_window(at(-0.09, 0.063, -0.001), head), "-x side");
        assert!(!in_ear_two_sided_window(at(0.03, 0.063, -0.001), head), "too close to the middle");
        assert!(!in_ear_two_sided_window(at(0.09, 0.2, -0.001), head), "above the ear window");
        assert!(!in_ear_two_sided_window(at(0.09, 0.063, 0.1), head), "in front of the ear window");
    }

    /// 0.188.0 (owner's install, `RECOM_GAME`; only the 1.5+2.5 collection is needed): the swap runs, and the debug renders
    /// of the head with KH2's hair go to the workspace's `local/assets/recom/riku-hair/out/kh2/`.
    #[test]
    fn swaps_rikus_hair_for_kh2s() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let src = Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        {
            let sk = RecomSkeleton::parse(&mdl).unwrap();
            let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
            let h = src.hair(&names, &sk.rest_world()).unwrap();
            let mut bb: std::collections::BTreeMap<usize, ([f64; 4], usize)> = Default::default();
            for t in &h.tris {
                for v in &t.v {
                    let b = bb.entry(t.island).or_insert(([9.0, 9.0, -9.0, -9.0], 0));
                    b.0 = [b.0[0].min(v.uv[0]), b.0[1].min(v.uv[1]), b.0[2].max(v.uv[0]), b.0[3].max(v.uv[1])];
                }
                bb.get_mut(&t.island).unwrap().1 += 1;
            }
            eprintln!("islands {} of {}", bb.len(), h.islands);
            for (i, (bx, n)) in &bb {
                eprintln!("island {i}: {n} tris uv [{:.3} {:.3} {:.3} {:.3}] -> {:.0}x{:.0} px", bx[0], bx[1], bx[2], bx[3], (bx[2] - bx[0]) * 1024.0, (bx[3] - bx[1]) * 1024.0);
            }
            let mut by: std::collections::BTreeMap<String, usize> = Default::default();
            for t in &h.tris { for v in &t.v { *by.entry(v.bone.clone()).or_default() += 1; } }
            eprintln!("bones {by:?}");
            let (mut lo, mut hi) = ([9.0f64; 3], [-9.0f64; 3]);
            for t in &h.tris { for v in &t.v { for k in 0..3 { lo[k] = lo[k].min(v.pos[k]); hi[k] = hi[k].max(v.pos[k]); } } }
            eprintln!("hair bounds {lo:?} {hi:?}");
            let (rlo, rhi) = dt::head_front_bounds(&mdl);
            eprintln!("Re:CoM head-bone front triangles bounds {rlo:?} {rhi:?}");
            let model = kh2_formats_lite::mdlx::parse(&src.mdlx).unwrap();
            let pose = kh2_formats_lite::skeleton::bind_pose(&model.bones);
            let k = kh2_formats_lite::skeleton::point(&pose[98], [0.0; 3]);
            eprintln!("kh2 head bone {k:?}");
            for (gi, g) in model.groups.iter().enumerate() {
                let (mut a, mut b) = ([1e9f64; 3], [-1e9f64; 3]);
                for t in &g.triangles { for v in t { let q = kh2_formats_lite::skeleton::point(&pose[v.bone as usize], v.position.map(|x| x as f64)); for c in 0..3 { a[c] = a[c].min(q[c]); b[c] = b[c].max(q[c]); } } }
                eprintln!("kh2 group {gi} ({} tris) bounds {:?} {:?}", g.triangles.len(), a.map(|x| ((x - k[0 ]) * 1.0).round()), b.map(|x| x.round()));
            }
        }
        let swap = with_hair(&mdl, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        eprintln!("{:?}", swap.report);
        assert_eq!(swap.report.ddd_hair, 613);
        let again = with_hair(&mdl, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        assert!(swap.mdl == again.mdl && swap.dds == again.dds, "deterministic");
        crate::mdl::Mdl::parse(&swap.mdl).unwrap();
        let parts = split_parts(&swap.mdl).unwrap();
        let p3 = MdlPart::parse_bytes(&parts[3]).unwrap();
        assert!(p3.texture_names().contains(&RIKU_0101_KH2.new_texture.to_string()));
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let world = sk.rest_world();
        let img = crate::texture::decode_dds(&swap.dds).unwrap();
        let out = dt::out_dir().join("kh2");
        std::fs::create_dir_all(&out).unwrap();
        dt::render(&p3, &world, &img, &out);
        std::fs::write(out.join("rk_hair_kh2_atlas.png"), dt::png(img.w, img.h, &img.px.iter().flat_map(|p| [p[0], p[1], p[2]]).collect::<Vec<_>>())).unwrap();
        std::fs::write(out.join("PL0101_hair_kh2.MDL"), &swap.mdl).unwrap();
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        // body vs hair behind the back: per 5 cm band of y, the most backward body vertex and hair vertex (|x| < 0.15)
        {
            let (b3, hr) = (MdlPart::parse_bytes(&split_parts(&mdl).unwrap()[3]).unwrap(), src.hair(&names, &world).unwrap());
            let mut body: std::collections::BTreeMap<i32, f64> = Default::default();
            let mut hairz: std::collections::BTreeMap<i32, f64> = Default::default();
            for t in dt::collect(&b3) {
                for v in &t.v {
                    let w = world[v.bone()].point(v.pos().map(|x| x as f64));
                    let nm = &names[v.bone()];
                    if w[0].abs() < 0.15 && !nm.contains("hair") && nm != "head" {
                        let e = body.entry((w[1] * 20.0).floor() as i32).or_insert(9.0);
                        *e = e.min(w[2]);
                    }
                }
            }
            for t in &hr.tris {
                for v in &t.v {
                    let e = hairz.entry((v.pos[1] * 20.0).floor() as i32).or_insert(9.0);
                    *e = e.min(v.pos[2]);
                }
            }
            for (k, z) in &hairz {
                eprintln!("band y {:.2}: hair back z {:.3}, body back z {:?}", *k as f64 / 20.0, z, body.get(k));
            }
        }
        let h = dt::top_holes(&p3, &world);
        eprintln!("top-down holes: {}", h.len());
        for (i, n) in names.iter().enumerate().filter(|(_, n)| n.contains("hair") || *n == "head") {
            eprintln!("bone {i} {n}: {:?}", world[i].t);
        }
    }


    /// Debug (0.188.1): Re:CoM's `PL0101.ANN` clips and where the hair bones go in them, relative to the head.
    #[test]
    fn hair_bones_in_riku_clips() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let ann = crate::ann::Ann::parse(&install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        let sk = RecomSkeleton::parse(&mdl).unwrap();
        let head = sk.find("head").unwrap();
        let hb: Vec<usize> = sk.bones.iter().enumerate().filter(|(_, b)| b.name.contains("hair")).map(|(i, _)| i).collect();
        eprintln!("slots {}", ann.slots.len());
        for &b in &hb { eprintln!("hair bone {b} {} parent {}", sk.bones[b].name, sk.bones[b].parent); }
        let rest = sk.rest_world();
        let rel = |w: &[Aff], b: usize| -> crate::math::M3 {
            let (rb, _) = crate::math::unscale(&w[b].r);
            let (rh, _) = crate::math::unscale(&w[head].r);
            crate::math::m3(&rb, &crate::math::tr(&rh))
        };
        for (i, s) in ann.slots.iter().enumerate() {
            let Some(c) = &s.clip else { continue };
            let mut worst = vec![0.0f64; hb.len()];
            for f in 0..c.frames.max(1) as usize {
                let w = posed_world(&sk, c, f as f64);
                for (k, &b) in hb.iter().enumerate() {
                    let d = crate::math::angle_deg(&crate::math::m3(&rel(&w, b), &crate::math::tr(&rel(&rest, b))));
                    worst[k] = worst[k].max(d);
                }
            }
            eprintln!("clip {i}: max hair-bone turn vs rest, relative to head, deg {:?}", worst.iter().map(|x| x.round()).collect::<Vec<_>>());
        }
    }

    /// The skeleton posed by `clip` at frame `t`: world matrices (rotation / translation / scale of the clip's channels).
    pub(crate) fn posed_world(sk: &RecomSkeleton, c: &crate::ann::Clip, t: f64) -> Vec<Aff> {
        let mut w: Vec<Aff> = Vec::with_capacity(sk.bones.len());
        for (i, b) in sk.bones.iter().enumerate() {
            let local = if i < c.bones as usize {
                let s = c.sample(i, t);
                let mut r = crate::math::rot_xyz(s.r[0], s.r[1], s.r[2]);
                for (row, sc) in r.iter_mut().zip(s.s) { for v in row.iter_mut() { *v *= sc; } }
                Aff { r, t: s.t }
            } else { b.local };
            w.push(if b.parent < 0 { local } else { local.mul(&w[b.parent as usize]) });
        }
        w
    }

    /// Debug (0.188.1): the swapped head and hair posed by Re:CoM's own clips, renders in `out/kh2/pose_<tag>/<clip>_<frame>/`
    /// (`tag` = env `HAIR_TAG`, default `now`).
    #[test]
    fn posed_hair_renders() {
        let Ok(g) = std::env::var("RECOM_GAME") else { return };
        let g = std::path::PathBuf::from(g);
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        let e = arc.find("PLAYER/0101/PL0101.RTM").unwrap();
        let (rtm, hd) = kh1_formats::egs::decode_entry_full(&keys, &arc.read_raw_entry(e).unwrap()).unwrap();
        let ann = crate::ann::Ann::parse(&install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.ANN").unwrap()).unwrap();
        
        let src = Kh2HairSource::read(&install::kh2(&g, keys.clone()).unwrap()).unwrap();
        let swap = with_hair(&mdl, &rtm, &hd, &RIKU_0101_KH2, &src).unwrap();
        let parts = split_parts(&swap.mdl).unwrap();
        let p3 = MdlPart::parse_bytes(&parts[3]).unwrap();
        let sk = RecomSkeleton::parse(&swap.mdl).unwrap();
        let img = crate::texture::decode_dds(&swap.dds).unwrap();
        let tag = std::env::var("HAIR_TAG").unwrap_or_else(|_| "now".into());
        let base = dt::out_dir().join("kh2").join(format!("pose_{tag}"));
        let names: Vec<String> = sk.bones.iter().map(|b| b.name.clone()).collect();
        {
            let d = base.join("rest");
            std::fs::create_dir_all(&d).unwrap();
            let w = sk.rest_world();
            let _ = render_body(&p3, &w, &names, &img, 0, &d.join("body_side.png"));
            let _ = render_body(&p3, &w, &names, &img, 1, &d.join("body_back.png"));
            let mut by: std::collections::BTreeMap<String, (usize, f64, f64)> = Default::default();
            for t in dt::collect(&p3) { if t.tex != 1 { continue; } for v in &t.v { let n = names[v.bone()].clone(); let y = w[v.bone()].point(v.pos().map(|x| x as f64))[1]; let e = by.entry(n).or_insert((0, 9.0, -9.0)); e.0 += 1; e.1 = e.1.min(y); e.2 = e.2.max(y); } }
            for (n, (c, lo, hi)) in &by { if n.starts_with("neck") || n.starts_with("chest") || n == "head" { eprintln!("REST tex1 corners on {n}: {c}, y {lo:.3}..{hi:.3}"); } }
        }
        for (clip, frames) in [(1usize, vec![0.0, 25.0, 50.0, 75.0]), (3, vec![0.0, 6.0, 11.0, 17.0]), (4, vec![8.0]), (14, vec![5.0]), (17, vec![7.0]), (26, vec![20.0]), (37, vec![10.0]), (41, vec![16.0])] {
            let Some(c) = ann.slots[clip].clip.as_ref() else { continue };
            for f in frames {
                let w = posed_world(&sk, c, f);
                let d = base.join(format!("c{clip}_f{f}"));
                std::fs::create_dir_all(&d).unwrap();
                let _ = render_body(&p3, &w, &names, &img, 0, &d.join("body_side.png"));
                let _ = render_body(&p3, &w, &names, &img, 1, &d.join("body_back.png"));
            }
        }
    }

    /// Debug (0.188.3): z-buffered render of the upper body (head, hair, neck, chest, collars, upper arms) of part 3 posed by
    /// `world`; hair texels from `atlas`, the rest flat grey-blue shaded by its normal. `view`: 0 = side (looking along -x), 1 = back, 2 = front, 3 = the other side.
    pub(crate) fn render_body(p3: &MdlPart, world: &[Aff], names: &[String], atlas: &crate::texture::Image, view: usize, path: &std::path::Path) -> usize {
        let ok = |n: &str| n == "head" || n.contains("hair") || n.starts_with("neck") || n.starts_with("chest") || n == "waist" || n.ends_with("collar") || n.ends_with("uparm");
        let size = 700usize;
        let tx = |q: [f64; 3]| -> [f64; 3] { match view { 0 => [q[2], q[1], -q[0]], 2 => [q[0], q[1], q[2]], 3 => [-q[2], q[1], q[0]], _ => [-q[0], q[1], -q[2]] } };
        let mut tris: Vec<([[f64; 3]; 3], [[f64; 2]; 3], bool)> = vec![];
        for t in dt::collect(p3) {
            if !t.v.iter().all(|v| ok(&names[v.bone()])) { continue; }
            let p = t.v.clone().map(|v| tx(world[v.bone()].point(v.pos().map(|x| x as f64))));
            let hair = t.tex == 1;
            let uv = t.v.clone().map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]));
            tris.push((p, uv, hair && t.tex == 1));
        }
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for (p, _, _) in &tris { for q in p { for k in 0..2 { lo[k] = lo[k].min(q[k]); hi[k] = hi[k].max(q[k]); } } }
        let sc = 640.0 / (hi[0] - lo[0]).max(hi[1] - lo[1]);
        let mut rgb = vec![30u8; size * size * 3];
        let mut zb = vec![f64::MIN; size * size];
        // body first, then hair: a hair fragment farther than the body surface is hidden by it (inside the torso or arm)
        let mut hidden = 0usize;
        let mut sd_body = vec![false; size * size];
        tris.sort_by_key(|t| t.2);
        for (p, uv, hair) in &tris {
            let s: Vec<[f64; 2]> = p.iter().map(|q| [30.0 + (q[0] - lo[0]) * sc, 670.0 - (q[1] - lo[1]) * sc]).collect();
            let n = { let a = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]]; let b = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]; let c = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]; let l = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt().max(1e-12); c[2].abs() / l };
            let det = (s[1][1] - s[2][1]) * (s[0][0] - s[2][0]) + (s[2][0] - s[1][0]) * (s[0][1] - s[2][1]);
            if det.abs() < 1e-9 { continue; }
            let (x0, x1) = (s.iter().map(|q| q[0]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (s.iter().map(|q| q[0]).fold(f64::MIN, f64::max).ceil() as usize).min(size - 1));
            let (y0, y1) = (s.iter().map(|q| q[1]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (s.iter().map(|q| q[1]).fold(f64::MIN, f64::max).ceil() as usize).min(size - 1));
            for y in y0..=y1 { for x in x0..=x1 {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let l0 = ((s[1][1] - s[2][1]) * (px - s[2][0]) + (s[2][0] - s[1][0]) * (py - s[2][1])) / det;
                let l1 = ((s[2][1] - s[0][1]) * (px - s[2][0]) + (s[0][0] - s[2][0]) * (py - s[2][1])) / det;
                let l2 = 1.0 - l0 - l1;
                if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 { continue; }
                let z = l0 * p[0][2] + l1 * p[1][2] + l2 * p[2][2];
                if z <= zb[y * size + x] { if *hair && z < zb[y * size + x] - 0.004 && sd_body[y * size + x] { hidden += 1; } continue; }
                zb[y * size + x] = z;
                sd_body[y * size + x] = !*hair;
                let c = if *hair {
                    let u = l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0]; let v = l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1];
                    let c = atlas.px[((v * atlas.h as f64) as usize).min(atlas.h - 1) * atlas.w + ((u * atlas.w as f64) as usize).min(atlas.w - 1)];
                    [c[0], c[1], c[2]]
                } else { let k = 60.0 + 140.0 * n; [(k * 0.5) as u8, (k * 0.7) as u8, k as u8] };
                rgb[3 * (y * size + x)..3 * (y * size + x) + 3].copy_from_slice(&c);
            } }
        }
        std::fs::write(path, dt::png(size, size, &rgb)).unwrap();
        hidden
    }




}
