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
        "ex_1580_sk" | "b_ex110_sk" => "ex_1010_sk",
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
        dark.find(n).or_else(|| dark.find(&b.name))
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
    retarget_with(src, riku, dark, map, None)
}

/// [`retarget`] with the target's bones that have no source bone posed from `fill` at its frame 0
/// (their local rotation and translation), else from their rest pose. Used for Sora's KH1 Guard on
/// Riku (0.157.0): his blade chain, belts and hair from his own battle stance.
pub fn retarget_with(src: &Clip, riku: &RecomSkeleton, dark: &RecomSkeleton, map: &[Option<usize>], fill: Option<&Clip>) -> Result<Clip> {
    let na = riku.bones.len();
    if src.bones as usize != na {
        return bad(format!("{}: {} bones, Riku's skeleton has {na}", src.name_str(), src.bones));
    }
    let ra = world_rot(riku);
    let rb = world_rot(dark);
    let nb = dark.bones.len();
    let grip = Grip::new(riku, dark);
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
        let mut gripped: Vec<Option<Target>> = vec![None; nb];
        for (j, b) in dark.bones.iter().enumerate() {
            if let Some(g) = &grip {
                if j == g.first {
                    g.place(&tg, &ua, &pa, &mut gripped);
                }
                if let Some(t) = gripped[j] {
                    tg.push(t);
                    continue;
                }
            }
            let u = match map[j] {
                // The blade past `wep0`: relative to `wep0`, as Riku's is to his (the rest frames
                // relative to `wep0` corrected), so it never twists against the grip.
                Some(i) if grip.as_ref().is_some_and(|g| g.below[j]) => {
                    let g = grip.as_ref().unwrap();
                    let qa = m3(&ua[i], &tr(&ua[g.wep0_a]));
                    let qa0 = m3(&ra[i], &tr(&ra[g.wep0_a]));
                    let qb0 = m3(&rb[j], &tr(&rb[g.wep0_b]));
                    // Under the un-turned `wep0`: the chain (trails, hits) stays where Riku's is;
                    // only the blade mesh on `wep0` is turned over.
                    m3(&m3(&m3(&qb0, &tr(&qa0)), &qa), &m3(&BLADE_FLIP, &tg[g.wep0_b].u))
                }
                Some(i) => m3(&m3(&rb[j], &tr(&ra[i])), &ua[i]),
                None if b.parent < 0 => rb[j],
                None => {
                    let r = match fill {
                        Some(c) => {
                            let s = c.sample(j, 0.0);
                            rot_xyz(s.r[0], s.r[1], s.r[2])
                        }
                        None => unscale(&b.local.r).0,
                    };
                    m3(&r, &tg[b.parent as usize].u)
                }
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
                    (_, None) if fill.is_some() => fill.unwrap().sample(j, 0.0).t,
                    _ => b.local.t,
                };
                let w = crate::math::vm(lt, &par.u);
                [w[0] + par.p[0], w[1] + par.p[1], w[2] + par.p[2]]
            };
            tg.push(Target { u, s: [1.0; 3], p });
        }
        let mut l = locals_for(dark, &tg, None, rows.last().map(|v| v.as_slice()));
        // Riku's own scale channels, bone for bone (0.154.4): his menu / draw clips hide the
        // Soul Eater by scaling `wep0` and `k1`..`k9` to 0 (Checked (files): 4 clips, 117 frames).
        // The translations above stay unscaled; the local scale is Riku's as his clip has it.
        if src.chans() == 9 {
            for (j, m) in map.iter().enumerate() {
                if let Some(i) = m {
                    let sc = src.sample(*i, f as f64).s;
                    l[j][6..9].copy_from_slice(&sc);
                }
            }
        }
        rows.push(l);
    }
    Ok(encode(src, &rows))
}

/// The Soul Eater's hold. Riku's `k11` / `k10` and Dark Riku's `wep2` / `wep1` both join the hand
/// to the blade root `wep0`, but they are not the same chain: Riku's goes 17 cm down the hand's
/// −z and back, Dark's 22 cm up its +z and back (Checked (files): rest positions in the hand's
/// frame). Mapped bone to bone they turn the blade the other way round (the owner's screenshot,
/// 0.154.1). At rest `wep0` sits the same in both hands (rotation identical, position 1.6 cm
/// apart). So: Dark's `wep0` takes Riku's `wep0` relative to the hand (in the hand's own axes, plus
/// the rest difference), and `wep2` / `wep1` follow `wep0` rigidly as in Dark's rest pose.
/// Half a turn about `wep0`'s x (the blade's axis), in its own frame (row vectors: on the left).
const BLADE_FLIP: M3 = [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];

struct Grip {
    /// Dark's first chain bone (`wep2`): from here the three are placed together.
    first: usize,
    hand_b: usize,
    hand_a: usize,
    wep0_a: usize,
    wep0_b: usize,
    /// Dark's bones below `wep0` (the blade segments).
    below: Vec<bool>,
    /// Dark's chain bones with their rest rotation and offset relative to Dark's `wep0` (wep0 itself:
    /// identity, 0).
    chain: Vec<(usize, M3, [f64; 3])>,
    /// Dark's rest `wep0` position in Dark's hand minus Riku's in Riku's (hand axes).
    d: [f64; 3],
}

impl Grip {
    fn new(riku: &RecomSkeleton, dark: &RecomSkeleton) -> Option<Grip> {
        let (hand_a, wep0_a) = (riku.find("r_hand")?, riku.find("wep0")?);
        let (hand_b, w0, w1, w2) = (dark.find("r_hand")?, dark.find("wep0")?, dark.find("wep1")?, dark.find("wep2")?);
        let (wa, wb) = (riku.rest_world(), dark.rest_world());
        let rot = |w: &crate::math::Aff| unscale(&w.r).0;
        let in_frame = |p: [f64; 3], o: &crate::math::Aff| crate::math::vm([p[0] - o.t[0], p[1] - o.t[1], p[2] - o.t[2]], &tr(&rot(o)));
        let pa = in_frame(wa[wep0_a].t, &wa[hand_a]);
        let pb = in_frame(wb[w0].t, &wb[hand_b]);
        let u0 = rot(&wb[w0]);
        let chain = [w0, w1, w2].iter().map(|&k| (k, m3(&rot(&wb[k]), &tr(&u0)), in_frame(wb[k].t, &wb[w0]))).collect();
        let mut below = vec![false; dark.bones.len()];
        for (j, b) in dark.bones.iter().enumerate() {
            below[j] = b.parent >= 0 && (b.parent as usize == w0 || below[b.parent as usize]);
        }
        Some(Grip { first: w2.min(w1).min(w0), hand_b, hand_a, wep0_a, wep0_b: w0, below, chain, d: [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]] })
    }

    /// The three chain bones' targets, from Dark's hand target (already placed) and Riku's pose.
    fn place(&self, tg: &[Target], ua: &[M3], pa: &[[f64; 3]], out: &mut [Option<Target>]) {
        let hb = &tg[self.hand_b];
        // Riku's blade root in his hand's axes, then in Dark's hand.
        let c = crate::math::vm(
            [pa[self.wep0_a][0] - pa[self.hand_a][0], pa[self.wep0_a][1] - pa[self.hand_a][1], pa[self.wep0_a][2] - pa[self.hand_a][2]],
            &tr(&ua[self.hand_a]),
        );
        let c = [c[0] + self.d[0], c[1] + self.d[1], c[2] + self.d[2]];
        let w = crate::math::vm(c, &hb.u);
        let p0 = [hb.p[0] + w[0], hb.p[1] + w[1], hb.p[2] + w[2]];
        // Riku's blade's rotation in his hand, in Dark's hand (the rest rotations match), turned
        // half a turn about the blade's own axis: Dark Riku's blade mesh is Riku's turned over
        // (its bounds on `wep0`: y −0.12..0.13 against Riku's −0.14..0.10), and Dark Riku's own
        // clips hold it so (his grip against Riku's: 137..178° about `wep0`'s x in 12 clips,
        // Checked (files)).
        let u0 = m3(&BLADE_FLIP, &m3(&m3(&ua[self.wep0_a], &tr(&ua[self.hand_a])), &hb.u));
        for &(k, rel, off) in &self.chain {
            let o = crate::math::vm(off, &u0);
            out[k] = Some(Target { u: m3(&rel, &u0), s: [1.0; 3], p: [p0[0] + o[0], p0[1] + o[1], p0[2] + o[2]] });
        }
    }
}

/// The clip from per-frame locals, keeping the source clip's header fields (no scale channels).
fn encode(src: &Clip, rows: &[Vec<[f64; 9]>]) -> Clip {
    const EPS: [f64; 3] = [1e-7, 1e-5, 1e-6];
    let nb = rows[0].len();
    let scale = rows.iter().any(|r| r.iter().any(|b| (6..9).any(|c| (b[c] - 1.0).abs() > EPS[2])));
    let chans = if scale { 9 } else { 6 };
    let mut channels = Vec::with_capacity(nb * chans);
    for b in 0..nb {
        for c in 0..chans {
            let kind = c / 3;
            let vals: Vec<f64> = rows.iter().map(|r| if kind == 1 { r[b][c].to_degrees() } else { r[b][c] }).collect();
            channels.push(encode_channel(&vals, EPS[kind]));
        }
    }
    Clip {
        name: src.name,
        bones: nb as i16,
        frames: rows.len() as i16,
        loop_start: src.loop_start,
        flags: if scale { src.flags | 2 } else { src.flags & !2 },
        has_blend: src.has_blend,
        blend: src.blend,
        blend_pad: src.blend_pad,
        mask_pad: vec![0; (((nb * chans + 7) >> 3) + 3) & !3],
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


/// Dark Riku's own entry flip (his action row 0x37, played by his Dark Mode switch-in
/// `0x14020485B`; Checked (exe)), added to the skin's pack on his own skeleton (the skin's), with
/// its EET / SES events (`ex_1580.dpx` 18 at bone 0, `transform.dpx` 1 at bone 51; Checked (files)).
/// The costume change plays it after Riku's own transformation crouch (`win::riku_skin`).
pub const DARK_FLIP: &str = "p003_067";

/// Riku's Guard clip (0.157.0 Sora's KH1 Guard retargeted; 0.158.0 his own KH1 guard).
///
/// 0.158.0 (fork owner, 2026-11-23: "no KH1 tem uma animação do Riku defendendo, ela acontece na
/// Destiny Island … quando ele duela com o Sora usando uma espada de madeira … inclusive tem a
/// variação de quando em defesa ele toma impacto"): KH1's Riku (`xa_ex_1010`, the Destiny Islands
/// duel set) guards in his AI script's `func_1e62` (`ex_1010.bd`, Checked (files: the script listed
/// with `tools/kh1-ai-script.py`)): request **240** (entry 28, 30 frames: into the guard), then
/// **241** (entry 29, 60 frames, played looping: the guard held), with his guard row `0x550` (type
/// 2, priority 10) on; the row's recoil is request **110** (entry 13), the same clip Re:CoM baked as
/// his `p002_043` (row 0x24), which his Guard already plays on a block. The clip here is 240's 30
/// frames then 241's first 24 (54 frames, the length of Sora's KH1 Guard, whose numbers the KH1
/// frame uses), converted from KH1's motions onto Riku's Re:CoM skeleton (`ex_1010_sk`, KH1's own:
/// `crate::convert::Converter`).
pub const RIKU_GUARD: &str = "p002_225";
/// KH1 Riku's guard requests: into the guard, the guard held.
pub const RIKU_GUARD_REQUESTS: (u16, u16) = (240, 241);
/// The clip's length: Sora's KH1 Guard (54 frames), whose timing the KH1 frame uses.
pub const RIKU_GUARD_FRAMES: usize = 54;
/// Frames of the held part after the way in (0.158.1: 7, then the way out).
pub const RIKU_GUARD_HOLD: u32 = 27;
/// Frames 240 (the way into the guard) is played in: KH1's guard object's first live frame.
pub const RIKU_GUARD_UP: usize = 10;

/// KH1 Riku's own guard on a Riku skeleton of Re:CoM's (named [`RIKU_GUARD`]).
pub fn riku_kh1_guard(mset: &[u8], mdls: &[u8], riku: &RecomSkeleton) -> Result<Clip> {
    let ms = kh1_formats::mset::MotionSet::parse(mset)?;
    let (into, held) = RIKU_GUARD_REQUESTS;
    let (_, m_in) = ms.motion_for(into).ok_or_else(|| crate::Error::Format(format!("xa_ex_1010.mset: no request {into}")))?;
    let (_, m_hold) = ms.motion_for(held).ok_or_else(|| crate::Error::Format(format!("xa_ex_1010.mset: no request {held}")))?;
    let conv = crate::convert::Converter::new(mdls, riku.clone())?;
    let mut rig = conv.rig();
    let mut rows: Vec<Vec<[f64; 9]>> = Vec::new();
    rig.start_motion(m_in);
    for f in 0..m_in.frames {
        let tg = conv.targets(&mut rig, m_in, f as f32, false);
        let l = conv.locals(&tg, rows.last().map(|v| v.as_slice()));
        rows.push(l);
    }
    let into: Vec<Vec<[f64; 9]>> = rows.clone();
    // 0.159.0 (fork owner: "Tempo/duração estranhos"): the guard is up when KH1's guard object goes
    // live (frame 10 of the Guard's 54, live 10..36): 240 is played in its first 10 frames, 241
    // held over the live window, then the way out.
    rows.clear();
    let last_in = into.len().saturating_sub(1);
    for k in 0..RIKU_GUARD_UP {
        rows.push(into[((k + 1) * last_in + RIKU_GUARD_UP / 2) / RIKU_GUARD_UP].clone());
    }
    rig.start_motion(m_hold);
    for f in 0..RIKU_GUARD_HOLD.min(m_hold.frames) {
        let tg = conv.targets(&mut rig, m_hold, f as f32, false);
        let l = conv.locals(&tg, rows.last().map(|v| v.as_slice()));
        rows.push(l);
    }
    // 0.158.1 (fork owner: "quando passa um tempo e ele desativa a guarda ficou estranha, sera que
    // nn tem como colocar a mesma animação mas ao contrário?"): the way out is the way in played
    // backwards, faster, to fill the clip: it ends on 240's first frame (his stance before the
    // guard), so the next motion starts from there instead of the held guard.
    let out = RIKU_GUARD_FRAMES.saturating_sub(rows.len()).max(1);
    let last = into.len().saturating_sub(1);
    for k in 0..out {
        let src = last - ((k + 1) * last + out / 2) / out;
        rows.push(into[src].clone());
    }
    let spec = crate::convert::ClipSpec { name: RIKU_GUARD.into(), looped: false, root_motion: false, blend: 8 };
    Ok(crate::convert::encode_clip(&spec, &rows))
}

/// KH1 Riku's guard recoil (request 110, entry 13, 56 frames; 0.158.2), named `p002_226`.
pub const RIKU_RECOIL: &str = "p002_226";

/// KH1 Riku's own guard recoil on a Riku skeleton of Re:CoM's (named [`RIKU_RECOIL`]).
pub fn riku_kh1_recoil(mset: &[u8], mdls: &[u8], riku: &RecomSkeleton) -> Result<Clip> {
    let ms = kh1_formats::mset::MotionSet::parse(mset)?;
    let (_, m) = ms.motion_for(110).ok_or_else(|| crate::Error::Format("xa_ex_1010.mset: no request 110".into()))?;
    let conv = crate::convert::Converter::new(mdls, riku.clone())?;
    conv.convert(m, &crate::convert::ClipSpec { name: RIKU_RECOIL.into(), looped: false, root_motion: false, blend: 8 })
}

/// The guard's sources for every Riku skeleton: KH1 Riku's motion set and model.
pub struct GuardSource<'a> {
    /// `xa_ex_1010.mset`.
    pub mset: &'a [u8],
    /// `xa_ex_1010.mdls`.
    pub mdls: &'a [u8],
}

/// Sora's KH1 Guard on Riku (0.157.0; replaced by [`riku_kh1_guard`] in 0.158.0, kept for tests).
/// Riku's battle stance (his action row 0x2A). Checked (exe: `0x1407C6370`).
pub const RIKU_STANCE: &str = "p002_031";

/// Sora's bone of the same role as Riku's.
fn sora_alias(riku: &str) -> &str {
    match riku {
        "ex_1010_sk" => "ex_0010_sk",
        x => x,
    }
}

/// Sora's guard clip on Riku's skeleton, named [`RIKU_GUARD`].
pub fn riku_guard(sora_guard: &Clip, sora: &RecomSkeleton, riku: &RecomSkeleton, riku_ann: &Ann) -> Result<Clip> {
    let map: Vec<Option<usize>> = riku.bones.iter().map(|b| sora.find(sora_alias(&b.name))).collect();
    let stance = riku_ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == RIKU_STANCE));
    let mut c = retarget_with(sora_guard, sora, riku, &map, stance)?;
    c.name = crate::ann::name16(RIKU_GUARD);
    Ok(c)
}

/// `ann` with `clip` added as its last slot (named as the clip), unless a slot already has it.
pub fn with_clip(mut ann: Ann, clip: Clip) -> Ann {
    if ann.find(&clip.name_str()).is_none() {
        ann.slots.push(Slot { name: clip.name, clip: Some(clip) });
        ann.pad_tail_like_recom();
    }
    ann
}

/// The skin manifest's file name (next to the other manifests).
pub const MANIFEST: &str = "kh1-riku-skin.json";

/// Dark Riku's bone 1 as Riku's name, so Riku's own bone lookups (`0x1401A4450` by name:
/// `ex_1010_sk`, `wep0`, `chest1`, `head`, `l_toes`, `r_toes`, from the strings at
/// `0x1407C5E10..0x1407C5E50`; Checked (exe)) find it. Same length; every part's name table.
pub fn rename_root(mdl: &[u8]) -> Result<Vec<u8>> {
    rename_bone(mdl, b"ex_1580_sk\0")
}

/// `mdl` with its bone named `from` (as long as Riku's `ex_1010_sk`) renamed `ex_1010_sk`.
fn rename_bone(mdl: &[u8], from: &[u8]) -> Result<Vec<u8>> {
    let to = b"ex_1010_sk\0";
    if from.len() != to.len() {
        return bad("rename_bone: names of different lengths");
    }
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
        return bad(format!("the model has no {} bone", String::from_utf8_lossy(&from[..from.len() - 1])));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// The Ansem costume (fork 0.176.0; fork owner, 2026-11-23: "implementa a skin do Ansem para o
// Riku").
//
// - Re:CoM (Checked (files + the owner's screenshot, 0.176.1)): Ansem, Seeker of Darkness is
//   `ENEMY/0122/EN0122` (textures `ex_1630_0x`, KH1's `xa_ex_1630`, voices `an00`; skeleton `e122_sk`,
//   157 bones). 0.176.0 took `EN0118` (`b_ex110`), which is Axel. His model reads whole (parts 0
//   textured, 1 untextured). His legs are `lf_*` / `rf_*`, his chest `chest2` (no `chest1`), his
//   neck `neck`: renamed to Riku's (`ANSEM_RENAMES`), as Riku's code looks bones up by name
//   (`chest1`, `l_toes`, `r_toes`, `head`) and the retarget maps by name. He holds no weapon: no
//   `wep0`.
// - The skin: his model with a `wep0` added under his `r_hand` at Riku's rest `wep0` relative to
//   Riku's `r_hand`, and Riku's own weapon parts (the Soul Eater on `wep0`, PL0101's parts 0 / 1)
//   moved onto it; bone 1 renamed `ex_1010_sk` (Riku's code looks his bones up by name); Riku's
//   clips retargeted by name ([`retarget_ann`]: the body; the robe and hair at rest; `wep0` by its
//   world rotation, its offset in the hand Riku's); his events remapped; his RTM with Riku's weapon
//   texture added. Assumption: the hands' rest frames are close enough for the blade's angle.

/// Ansem's model (`ENEMY/0122`).
pub const ANSEM_ENEMY: u16 = 122;
/// Ansem's bones renamed to Riku's (the same roles).
pub const ANSEM_RENAMES: [(&str, &str); 12] = [
    ("e122_sk", "ex_1010_sk"),
    ("lf_femur", "l_femur"),
    ("lf_tibia", "l_tibia"),
    ("lf_foot", "l_foot"),
    ("lf_toes", "l_toes"),
    ("rf_femur", "r_femur"),
    ("rf_tibia", "r_tibia"),
    ("rf_foot", "r_foot"),
    ("rf_toes", "r_toes"),
    ("chest2", "chest1"),
    ("neck", "neck1"),
    ("waist", "waist"),
];
/// (Riku's group, the Ansem group with the Soul Eater, the Ansem group with the Keyblade of Heart).
pub const ANSEM_GROUPS: [(u16, u16, u16); 2] = [(101, 181, 221), (111, 191, 231)];

fn enemy_path(n: u16, ext: &str) -> String {
    format!("ENEMY/{n:04}/EN{n:04}.{ext}")
}

/// Ansem's model with Riku's weapon on a `wep0` of its own, parts ordered as Riku's (weapon
/// untextured, weapon textured, body untextured, body textured).
pub fn ansem_mdl(riku_mdl: &[u8], ansem: &[u8]) -> Result<Vec<u8>> {
    use crate::mdl::{join_parts, split_parts, MdlBone, MdlPart, FMT_PLAIN};
    let rs = RecomSkeleton::parse(riku_mdl)?;
    let (rh, rw) = (rs.find("r_hand").ok_or_else(|| crate::Error::Format("Riku: no r_hand".into()))?, rs.find("wep0").ok_or_else(|| crate::Error::Format("Riku: no wep0".into()))?);
    let wr = rs.rest_world();
    let (rot_h, _) = unscale(&wr[rh].r);
    let rel_r = m3(&wr[rw].r, &tr(&rot_h));
    let rel_t = crate::math::vm([wr[rw].t[0] - wr[rh].t[0], wr[rw].t[1] - wr[rh].t[1], wr[rw].t[2] - wr[rh].t[2]], &tr(&rot_h));
    let mut rest = [0u32; 16];
    for r in 0..3 {
        for c in 0..3 {
            rest[r * 4 + c] = (rel_r[r][c] as f32).to_bits();
        }
    }
    for c in 0..3 {
        rest[12 + c] = (rel_t[c] as f32).to_bits();
    }
    rest[15] = 1f32.to_bits();
    let rparts = split_parts(riku_mdl)?;
    let aparts = split_parts(ansem)?;
    if aparts.len() != 2 {
        return Err(crate::Error::Format(format!("Ansem's model: {} parts, not 2", aparts.len())));
    }
    let mut a: Vec<MdlPart> = aparts.iter().map(|b| MdlPart::parse_bytes(b)).collect::<Result<_>>()?;
    for p in a.iter_mut() {
        for b in p.bones.iter_mut() {
            let n = crate::ann::cstr(&b.name);
            if let Some(&(_, to)) = ANSEM_RENAMES.iter().find(|r| r.0 == n) {
                b.name = [0; 16];
                b.name[..to.len()].copy_from_slice(to.as_bytes());
            }
        }
    }
    let hand = a[0].bones.iter().position(|b| crate::ann::cstr(&b.name) == "r_hand").ok_or_else(|| crate::Error::Format("Ansem: no r_hand".into()))?;
    let nb = a[0].bones.len();
    let mut name = [0u8; 16];
    name[..4].copy_from_slice(b"wep0");
    for p in a.iter_mut() {
        if p.bones.len() != nb {
            return Err(crate::Error::Format("Ansem's parts disagree on the bones".into()));
        }
        p.bones.push(MdlBone { name, parent: hand as i16, index: nb as i16 });
        p.rest.push(rest);
        p.bounds.push(crate::keyblade::part_bounds(nb + 1, &[])[nb]);
    }
    let mut w = Vec::new();
    for k in 0..2 {
        let mut p = MdlPart::parse_bytes(&rparts[k])?;
        for pk in p.list_a.iter_mut().chain(p.list_b.iter_mut().flatten()) {
            for v in pk.verts.iter_mut() {
                if v.bone() != rw {
                    return Err(crate::Error::Format("Riku's weapon parts: a vertex off wep0".into()));
                }
                v.q1[3] = ((nb as u32) << 16) | (nb as u32 * 4);
            }
        }
        p.bones = a[0].bones.clone();
        p.rest = a[0].rest.clone();
        p.bounds = crate::keyblade::part_bounds(nb + 1, &[&p.list_a, p.list_b.as_deref().unwrap_or(&[])]);
        w.push(p.write_bytes()?);
    }
    let plain_first = a[0].list_a.first().is_some_and(|k| k.fmt == FMT_PLAIN);
    let (ap, at) = if plain_first { (0, 1) } else { (1, 0) };
    // The weapon parts carry Ansem's bone list as renamed.
    let parts = vec![w[0].clone(), w[1].clone(), a[ap].write_bytes()?, a[at].write_bytes()?];
    Ok(join_parts(&parts))
}

/// One Ansem group: Ansem's model with Riku's weapon (or, with `blade`, the Keyblade of Heart),
/// Riku's clips of `riku` (`have`: the step's own, else Re:CoM's) retargeted, his events remapped,
/// Ansem's raw RTM (+ the weapon texture) and TXA, under `target`'s names.
pub fn build_ansem(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, have: &std::collections::HashMap<String, Vec<u8>>, riku: u16, target: u16, blade: Option<&BladeSource>) -> Result<Built> {
    let get = |n: String| -> Result<Vec<u8>> {
        match have.get(&n) {
            Some(b) => Ok(b.clone()),
            None => crate::install::read_recom(arc, keys, &n),
        }
    };
    let raw = |n: String| -> Result<(Vec<u8>, Vec<kh1_formats::egs::HdAsset>, i32)> {
        let e = arc.find(&n).ok_or_else(|| crate::Error::Format(format!("{n} not in Recom.hed")))?;
        let raw = arc.read_raw_entry(e)?;
        let date = raw.get(12..16).map_or(0, |b| i32::from_le_bytes(b.try_into().unwrap()));
        let (d, a) = kh1_formats::egs::decode_entry_full(keys, &raw)?;
        Ok((d, a, date))
    };
    let rmdl = crate::install::read_recom(arc, keys, &path(riku, "MDL"))?;
    let rs = RecomSkeleton::parse(&rmdl)?;
    let mut mdl = ansem_mdl(&rmdl, &crate::install::read_recom(arc, keys, &enemy_path(ANSEM_ENEMY, "MDL"))?)?;
    if let Some(src) = blade {
        mdl = with_blade(&mdl, src, false)?;
    }
    let ask = RecomSkeleton::parse(&mdl)?;
    let ann = Ann::parse(&get(path(riku, "ANN"))?)?;
    let out_ann = retarget_ann(&ann, &rs, &ask)?;
    let em = event_map(&rs, &ask);
    let eet = remap_eet(&get(path(riku, "EET"))?, &em)?;
    let ses = remap_ses(&get(path(riku, "SES"))?, &em)?;
    let clips = out_ann.slots.iter().filter(|s| s.clip.is_some()).count();
    let mut files = vec![(path(target, "MDL"), mdl), (path(target, "ANN"), out_ann.to_bytes()), (path(target, "EET"), eet), (path(target, "SES"), ses)];
    // Ansem's textures + Riku's weapon texture (or the Keyblade of Heart's).
    let (d, mut a, date) = raw(enemy_path(ANSEM_ENEMY, "RTM"))?;
    let entry = match blade {
        Some(src) => rtm_with_blade(&d, &a, date, src)?,
        None => {
            let (rd_, ra, _) = raw(path(riku, "RTM"))?;
            let rr = crate::rtm::Rtm::parse(&rd_)?;
            let mut rtm = crate::rtm::Rtm::parse(&d)?;
            for e in rr.entries.iter().filter(|e| e.name_str().starts_with("weapon_")) {
                let hd = format!("{}_tm2.dds", e.name_str().trim_end_matches(".tm2"));
                rtm.entries.push(e.clone());
                if let Some(x) = ra.iter().find(|x| x.name == hd) {
                    a.push(x.clone());
                }
            }
            kh1_formats::egs::build_plain_entry(&rtm.write(), &a, date)?
        }
    };
    files.push((format!("raw/{}", path(target, "RTM")), entry));
    let (d, a, date) = raw(enemy_path(ANSEM_ENEMY, "TXA"))?;
    files.push((format!("raw/{}", path(target, "TXA")), kh1_formats::egs::build_plain_entry(&d, &a, date)?));
    Ok(Built { files, clips })
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
    build_with(arc, keys, riku, dark, skin, None)
}

/// [`build`], Riku's pack first given Sora's KH1 Guard (`sora`: his guard clip and skeleton;
/// 0.157.0). Riku's own pack is then one of the files (`PLAYER/01x1/PL01x1.ANN`), and the skin's
/// pack gets the guard too.
pub fn build_with(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, riku: u16, dark: u16, skin: u16, guard: Option<&GuardSource>) -> Result<Built> {
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
    let mut ann = Ann::parse(&rd(riku, "ANN")?)?;
    let mut riku_pack = None;
    if let Some(src) = guard {
        let guard = riku_kh1_guard(src.mset, src.mdls, &rs)?;
        ann = with_clip(ann, guard);
        ann = with_clip(ann, riku_kh1_recoil(src.mset, src.mdls, &rs)?);
        riku_pack = Some((path(riku, "ANN"), ann.to_bytes()));
    }
    let mut out_ann = retarget_ann(&ann, &rs, &ds)?;
    let em = event_map(&rs, &ds);
    let mut eet = crate::eet::parse(&remap_eet(&rd(riku, "EET")?, &em)?)?;
    let mut ses = crate::ses::parse(&remap_ses(&rd(riku, "SES")?, &em)?)?;
    // Dark Riku's entry flip with its own events (his skeleton is the skin's).
    let dann = Ann::parse(&rd(dark, "ANN")?)?;
    let flip = dann.slots.iter().find(|s| s.clip.as_ref().is_some_and(|c| c.name_str() == DARK_FLIP)).ok_or_else(|| crate::Error::Format(format!("{}: no {DARK_FLIP}", path(dark, "ANN"))))?;
    if out_ann.find(DARK_FLIP).is_none() {
        let tail = std::mem::take(&mut out_ann.tail);
        out_ann.slots.push(flip.clone());
        out_ann.tail = tail;
        out_ann.pad_tail_like_recom();
    }
    eet.extend(crate::eet::parse(&rd(dark, "EET")?)?.into_iter().filter(|r| r.name == DARK_FLIP));
    ses.extend(crate::ses::parse(&rd(dark, "SES")?)?.into_iter().filter(|e| e.name() == DARK_FLIP));
    let clips = out_ann.slots.iter().filter(|s| s.clip.is_some()).count();
    let mut files = vec![
        (path(skin, "MDL"), rename_root(&dmdl)?),
        (path(skin, "ANN"), out_ann.to_bytes()),
        (path(skin, "EET"), crate::eet::write(&eet)?),
        (path(skin, "SES"), crate::ses::write(&ses)),
    ];
    files.extend(riku_pack);
    for ext in ["RTM", "TXA"] {
        let (d, a, date) = raw(dark, ext)?;
        files.push((format!("raw/{}", path(skin, ext)), kh1_formats::egs::build_plain_entry(&d, &a, date)?));
    }
    Ok(Built { files, clips })
}

/// The manifest: which Riku group each skin group stands for, with its look (Dark Mode attire,
/// blade 0 Soul Eater / 1 the Keyblade of People's Hearts).
pub fn manifest_json(skins: &[Made], riku_guard: bool) -> String {
    let rows: Vec<String> = skins.iter().map(|m| format!("{{\"riku\": {}, \"skin\": {}, \"dark\": {}, \"costume\": {}, \"blade\": {}}}", m.riku, m.skin, m.dark, m.costume(), m.blade)).collect();
    format!("{{\"format\": 1, \"skins\": [{}], \"riku_guard\": {riku_guard}}}\n", rows.join(", "))
}

/// One group made: the Riku group it stands for, and its look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Made {
    /// Riku's own group (101 / 111).
    pub riku: u16,
    /// The group made.
    pub skin: u16,
    /// The Dark Mode attire.
    pub dark: bool,
    /// 0 the Soul Eater, 1 the Keyblade of People's Hearts.
    pub blade: u8,
    /// 0.176.0: Ansem's model (costume 2).
    pub ansem: bool,
}

impl Made {
    /// The costume: 0 Default Attire, 1 Dark Mode, 2 Ansem.
    pub fn costume(&self) -> u8 {
        if self.ansem {
            2
        } else {
            self.dark as u8
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Riku's Keyblade (fork 0.160.0; fork owner, 2026-11-23: "quero implementar para o Riku a troca de
// Keyblade, vai ser simplesmente cosmética … entre a Soul Eater e a Keyblade of Heart"; "no Kingdom
// Hearts 1 essa é a única keyblade que ele utiliza, numa bossfight inclusive com o mesmo traje do
// DarkMode").
//
// - KH1 (Checked (files)): Riku-Ansem's battle model `xa_ex_1580` (voices `rk04`, the Dark Mode
//   attire) holds it as its HD mesh `-c0.cvbl` mesh 0, on joint 211 alone, texture slot 0
//   (`-1c0.dds`, 512 × 512: the dark blade, the red heart, the crown teeth).
// - Re:CoM (Checked (files)): Riku's weapon is two parts of his own model (`PL0101` / `PL0111`
//   parts 0 untextured and 1 textured, Dark Riku's `PL0201` / `PL0211` the same), every vertex on
//   `wep0` (77; Dark's 105). Re:CoM's Soul Eater on `wep0` = KH1 `xa_ex_1560`'s on its joint 191
//   × 0.01, same axes (the same bounds to the centimetre).
// - The Keyblade's joint 211 (Riku-Ansem's 247-joint skeleton) sits [`BLADE_SHIFT`] from joint
//   191's: KH1 `xa_ex_1160` (Riku in Hollow Bastion, the same skeleton) holds the Soul Eater on
//   joint 211; matched to `xa_ex_1560`'s, a shift of (4.54, −0.52, −0.02) cm (mean miss 1.2 cm:
//   the two meshes differ a little). Assumption: Riku-Ansem's grip is that Riku's.
// - Dark Riku's `wep0` is Riku's turned half a turn about its x (`BLADE_FLIP`, 0.154.3): the blade
//   is turned the same way there, so it shows as on Riku.
// - The new groups ([`BLADE_GROUPS`]) are the four looks' own copies; only their `.MDL` (the two
//   weapon parts replaced, the rest kept byte for byte) and `.RTM` (+ the Keyblade's texture)
//   differ.

/// KH1's model holding the Keyblade.
pub const BLADE_MODEL: &str = "xa_ex_1580.mdls";
/// Its HD mesh.
pub const BLADE_CVBL: &str = "-c0.cvbl";
/// The Keyblade's mesh in it.
pub const BLADE_MESH: usize = 0;
/// The joint it is on.
pub const BLADE_JOINT: u32 = 211;
/// Its texture (cvbl slot 0).
pub const BLADE_DDS: &str = "-1c0.dds";
/// Its texture's name in Re:CoM (looked up globally: one name for the four groups).
pub const BLADE_TEXTURE: &str = "rk_kophearts";
/// Joint 211 → joint 191 (KH1 units).
pub const BLADE_SHIFT: [f32; 3] = [4.54, -0.52, -0.02];

/// (base group, the group whose raw `.RTM` / `.TXA` it uses, new group, Dark Mode attire).
pub const BLADE_GROUPS: [(u16, u16, u16, bool); 4] = [(101, 101, 141, false), (111, 111, 151, false), (121, 201, 161, true), (131, 211, 171, true)];

/// The Keyblade's sources: KH1's HD mesh and texture.
pub struct BladeSource {
    /// The HD mesh file.
    pub cvbl: Vec<u8>,
    /// The texture (DXT).
    pub dds: Vec<u8>,
}

impl BladeSource {
    /// From `xa_ex_1580.mdls`'s HD assets.
    pub fn from_assets(assets: &[kh1_formats::egs::HdAsset]) -> Result<BladeSource> {
        let get = |n: &str| assets.iter().find(|a| a.name == n).map(|a| a.data.clone()).ok_or_else(|| crate::Error::Format(format!("{BLADE_MODEL}: no {n}")));
        Ok(BladeSource { cvbl: get(BLADE_CVBL)?, dds: get(BLADE_DDS)? })
    }
}

/// `mdl` (Riku's or Dark Riku's, parts 0 / 1 his weapon on `wep0`) with the Keyblade as its weapon.
pub fn with_blade(mdl: &[u8], src: &BladeSource, flip: bool) -> Result<Vec<u8>> {
    use crate::mdl::{join_parts, split_parts, MdlPart, FMT_PLAIN, FMT_TEXTURED};
    let mut parts = split_parts(mdl)?;
    if parts.len() < 3 {
        return Err(crate::Error::Format("Riku's model: fewer than 3 parts".into()));
    }
    let p1 = MdlPart::parse_bytes(&parts[1])?;
    let p2 = MdlPart::parse_bytes(&parts[2])?;
    let p2 = if p2.list_a.first().is_some_and(|k| k.fmt == FMT_PLAIN) { p2 } else { MdlPart::parse_bytes(&parts[3.min(parts.len() - 1)])? };
    let wep0 = p1.bones.iter().position(|b| crate::ann::cstr(&b.name) == "wep0").ok_or_else(|| crate::Error::Format("Riku's model: no wep0".into()))?;
    let only_wep0 = |p: &MdlPart| p.list_a.iter().chain(p.list_b.iter().flatten()).all(|k| k.verts.iter().all(|v| v.bone() == wep0));
    let tx = p1.list_a.first().filter(|k| k.fmt == FMT_TEXTURED).ok_or_else(|| crate::Error::Format("Riku's model: part 1 is not textured".into()))?;
    let pl = p2.list_a.first().filter(|k| k.fmt == FMT_PLAIN).ok_or_else(|| crate::Error::Format("Riku's model: part 2 is not untextured".into()))?;
    if !only_wep0(&p1) || parts[0].len() < 0x30 || u16::from_le_bytes([parts[0][0], parts[0][1]]) as usize != p1.bones.len() {
        return Err(crate::Error::Format("Riku's model: parts 0 / 1 are not his weapon alone".into()));
    }
    let cv = kh1_formats::cvbl::Cvbl::parse(&src.cvbl)?;
    let (_, _, img) = crate::keyblade::texture_entry(BLADE_TEXTURE, &src.dds)?;
    let place = |p: [f32; 3]| {
        let q = [(p[0] + BLADE_SHIFT[0]) * 0.01, (p[1] + BLADE_SHIFT[1]) * 0.01, (p[2] + BLADE_SHIFT[2]) * 0.01];
        if flip {
            [q[0], -q[1], -q[2]]
        } else {
            q
        }
    };
    let bp = crate::keyblade::blade_parts(&cv, BLADE_MESH, BLADE_JOINT, wep0 as u32, &place, &img, (tx.gif, tx.regs), (pl.gif, pl.regs))?;
    let nb = p1.bones.len();
    let mut name = [0u8; 32];
    name[..BLADE_TEXTURE.len()].copy_from_slice(BLADE_TEXTURE.as_bytes());
    let textured = MdlPart {
        bounds: crate::keyblade::part_bounds(nb, &[&bp.textured_a, bp.textured_b.as_deref().unwrap_or(&[])]),
        textures: vec![name],
        list_a: bp.textured_a,
        list_b: bp.textured_b,
        ..p1.clone()
    };
    let plain = MdlPart {
        flags: [parts[0][2], parts[0][3]],
        filler: parts[0][0x20..0x30].try_into().unwrap(),
        bones: p1.bones.clone(),
        rest: p1.rest.clone(),
        textures: vec![],
        bounds: crate::keyblade::part_bounds(nb, &[&bp.plain]),
        list_a: bp.plain,
        list_b: None,
    };
    parts[0] = plain.write_bytes()?;
    parts[1] = textured.write_bytes()?;
    Ok(join_parts(&parts))
}

/// A raw `.RTM` entry (`(data, HD assets, date)` as the archive holds it) with the Keyblade's
/// texture added (TIM2 + HD), as a plain entry for Panacea's raw route.
pub fn rtm_with_blade(d: &[u8], assets: &[kh1_formats::egs::HdAsset], date: i32, src: &BladeSource) -> Result<Vec<u8>> {
    let mut rtm = crate::rtm::Rtm::parse(d)?;
    let (e, hd, _) = crate::keyblade::texture_entry(BLADE_TEXTURE, &src.dds)?;
    rtm.entries.retain(|x| x.name_str() != BLADE_TEXTURE);
    rtm.entries.push(e);
    let mut a = assets.to_vec();
    a.retain(|x| x.name != hd.name);
    a.push(hd);
    Ok(kh1_formats::egs::build_plain_entry(&rtm.write(), &a, date)?)
}

/// One Keyblade group: `base`'s files (`have`: what the step wrote this run, else Re:CoM's) under
/// `target`'s names, its model with the Keyblade, its raw `.RTM` / `.TXA` from `raw_src`.
pub fn build_blade(arc: &kh1_formats::egs::Archive, keys: &kh1_formats::egs::KeyTable, have: &std::collections::HashMap<String, Vec<u8>>, base: u16, raw_src: u16, target: u16, dark: bool, src: &BladeSource) -> Result<Built> {
    let get = |ext: &str| -> Result<Vec<u8>> {
        let n = path(base, ext);
        match have.get(&n) {
            Some(b) => Ok(b.clone()),
            None => crate::install::read_recom(arc, keys, &n),
        }
    };
    let raw = |ext: &str| -> Result<(Vec<u8>, Vec<kh1_formats::egs::HdAsset>, i32)> {
        let n = path(raw_src, ext);
        let e = arc.find(&n).ok_or_else(|| crate::Error::Format(format!("{n} not in Recom.hed")))?;
        let raw = arc.read_raw_entry(e)?;
        let date = raw.get(12..16).map_or(0, |b| i32::from_le_bytes(b.try_into().unwrap()));
        let (d, a) = kh1_formats::egs::decode_entry_full(keys, &raw)?;
        Ok((d, a, date))
    };
    let mut files = vec![(path(target, "MDL"), with_blade(&get("MDL")?, src, dark)?)];
    for ext in ["ANN", "EET", "SES"] {
        files.push((path(target, ext), get(ext)?));
    }
    let (d, a, date) = raw("RTM")?;
    files.push((format!("raw/{}", path(target, "RTM")), rtm_with_blade(&d, &a, date, src)?));
    let (d, a, date) = raw("TXA")?;
    files.push((format!("raw/{}", path(target, "TXA")), kh1_formats::egs::build_plain_entry(&d, &a, date)?));
    let clips = Ann::parse(&files[1].1)?.slots.iter().filter(|s| s.clip.is_some()).count();
    Ok(Built { files, clips })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install;

    /// With the owner's installs (`RECOM_GAME`): the Keyblade replaces Riku's weapon parts alone
    /// (the other parts byte for byte), on `wep0`, its grip where the Soul Eater's is (the
    /// Keyblade spans the Soul Eater's grip region), turned over on Dark Riku's model.
    #[test]
    fn the_keyblade_replaces_rikus_weapon_parts() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let Ok((_, assets, _)) = install::kh1(&g, keys.clone()).unwrap().read_full(BLADE_MODEL) else { return };
        let src = BladeSource::from_assets(&assets).unwrap();
        for (name, flip, wep0) in [("PLAYER/0101/PL0101.MDL", false, 77usize), ("PLAYER/0201/PL0201.MDL", true, 105)] {
            let old = install::read_recom(&arc, &keys, name).unwrap();
            let new = with_blade(&old, &src, flip).unwrap();
            let (a, b) = (crate::mdl::split_parts(&old).unwrap(), crate::mdl::split_parts(&new).unwrap());
            assert_eq!(a.len(), b.len());
            assert_eq!(a[2..], b[2..], "{name}: the body kept byte for byte");
            let p = crate::mdl::MdlPart::parse_bytes(&b[1]).unwrap();
            assert_eq!(p.texture_names(), [BLADE_TEXTURE]);
            let vs: Vec<_> = p.list_a.iter().chain(p.list_b.iter().flatten()).flat_map(|k| k.verts.iter()).collect();
            assert!(vs.iter().all(|v| v.bone() == wep0));
            let xs: Vec<f32> = vs.iter().map(|v| v.pos()[0]).collect();
            let (lo, hi) = (xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max));
            assert!(lo < -1.0 && hi > 0.2 && hi < 0.3, "{name}: {lo}..{hi} m along the blade");
            let ys: Vec<f32> = vs.iter().map(|v| v.pos()[1]).collect();
            let ymax = ys.iter().cloned().fold(f32::MIN, f32::max);
            assert!(if flip { ymax < 0.18 } else { ymax > 0.18 }, "{name}: turned over on Dark's model ({ymax})");
            assert!(crate::mdl::MdlPart::parse_bytes(&b[0]).is_ok());
        }
    }

    fn game() -> Option<std::path::PathBuf> {
        std::env::var_os("RECOM_GAME").map(Into::into)
    }

    /// With the owner's install (`RECOM_GAME`): Riku's idle on the Dark skeleton keeps its feet
    /// where Riku's are (within Dark's longer legs' difference), and an idle pose's bones
    /// stay near Dark's own rest.
    /// With the owner's install: the Ansem costume's model holds Riku's weapon on a `wep0` of its
    /// own under its `r_hand`; Riku's idle retargeted keeps the feet and head where Riku's are
    /// (relative to rest), and the blade root stays in the hand.
    #[test]
    fn riku_idle_on_ansem() {
        let Some(g) = game() else { return };
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let rd = |n: &str| install::read_recom(&arc, &keys, n).unwrap();
        let rmdl = rd("PLAYER/0101/PL0101.MDL");
        let riku = RecomSkeleton::parse(&rmdl).unwrap();
        let mdl = ansem_mdl(&rmdl, &rd(&enemy_path(ANSEM_ENEMY, "MDL"))).unwrap();
        let ansem = RecomSkeleton::parse(&mdl).unwrap();
        assert_eq!(ansem.bones[1].name, "ex_1010_sk");
        let w0 = ansem.find("wep0").unwrap();
        assert_eq!(ansem.bones[w0].parent as usize, ansem.find("r_hand").unwrap());
        let parts = crate::mdl::split_parts(&mdl).unwrap();
        assert_eq!(parts.len(), 4);
        for pb in &parts {
            crate::mdl::MdlPart::parse_bytes(pb).unwrap();
        }
        let ann = Ann::parse(&rd("PLAYER/0101/PL0101.ANN")).unwrap();
        let map = bone_map(&riku, &ansem);
        let idle = ann.slots.iter().find_map(|s| s.clip.as_ref().filter(|c| c.name_str() == "p002_001")).unwrap();
        let out = retarget(idle, &riku, &ansem, &map).unwrap();
        let world = |c: &Clip, sk: &RecomSkeleton| {
            let mut u = vec![crate::math::I3; sk.bones.len()];
            let mut p = vec![[0.0; 3]; sk.bones.len()];
            for (i, b) in sk.bones.iter().enumerate() {
                let s = c.sample(i, 0.0);
                let r = rot_xyz(s.r[0], s.r[1], s.r[2]);
                if b.parent < 0 {
                    u[i] = r;
                    p[i] = s.t;
                } else {
                    let q = b.parent as usize;
                    let w = crate::math::vm(s.t, &u[q]);
                    p[i] = [p[q][0] + w[0], p[q][1] + w[1], p[q][2] + w[2]];
                    u[i] = m3(&r, &u[q]);
                }
            }
            p
        };
        let (pa, pb) = (world(idle, &riku), world(&out, &ansem));
        for bone in ["l_foot", "r_foot", "head"] {
            let (i, j) = (riku.find(bone).unwrap(), ansem.find(bone).unwrap());
            let ra = riku.rest_world()[i].t[1];
            let rb = ansem.rest_world()[j].t[1];
            eprintln!("{bone}: riku {:.3} (rest {ra:.3}) ansem {:.3} (rest {rb:.3})", pa[i][1], pb[j][1]);
            assert!(((pa[i][1] - ra) - (pb[j][1] - rb)).abs() < 0.05, "{bone}");
        }
        let h = ansem.find("r_hand").unwrap();
        let d = crate::math::dist(pb[w0], pb[h]);
        let dr = crate::math::dist(pa[riku.find("wep0").unwrap()], pa[riku.find("r_hand").unwrap()]);
        eprintln!("wep0 to hand: ansem {d:.3} riku {dr:.3}");
        assert!((d - dr).abs() < 0.05);
    }

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
        // The Soul Eater: on every frame of every clip, the blade root and tip in the hand's axes
        // match Riku's (within the rest difference, 2 cm).
        let in_hand = |c: &Clip, sk: &RecomSkeleton, f: f64, bone: &str| -> [f64; 3] {
            let mut u = vec![crate::math::I3; sk.bones.len()];
            let mut p = vec![[0.0; 3]; sk.bones.len()];
            for (i, b) in sk.bones.iter().enumerate() {
                let s = c.sample(i, f);
                let r = rot_xyz(s.r[0], s.r[1], s.r[2]);
                if b.parent < 0 {
                    u[i] = r;
                    p[i] = s.t;
                } else {
                    let q = b.parent as usize;
                    let w = crate::math::vm(s.t, &u[q]);
                    p[i] = [p[q][0] + w[0], p[q][1] + w[1], p[q][2] + w[2]];
                    u[i] = m3(&r, &u[q]);
                }
            }
            let h = sk.find("r_hand").unwrap();
            let k = sk.find(bone).unwrap();
            crate::math::vm([p[k][0] - p[h][0], p[k][1] - p[h][1], p[k][2] - p[h][2]], &tr(&u[h]))
        };
        let mut worst: f64 = 0.0;
        let mut worst_rot: f64 = 0.0;
        let rot_in_hand = |c: &Clip, sk: &RecomSkeleton, f: f64| -> M3 {
            let mut u = vec![crate::math::I3; sk.bones.len()];
            for (i, b) in sk.bones.iter().enumerate() {
                let s = c.sample(i, f);
                let r = rot_xyz(s.r[0], s.r[1], s.r[2]);
                u[i] = if b.parent < 0 { r } else { m3(&r, &u[b.parent as usize]) };
            }
            m3(&u[sk.find("wep0").unwrap()], &tr(&u[sk.find("r_hand").unwrap()]))
        };
        for c in ann.slots.iter().filter_map(|s| s.clip.as_ref()) {
            let o = retarget(c, &riku, &dark, &map).unwrap();
            for f in 0..c.frames as usize {
                let (ra_, rb_) = (rot_in_hand(c, &riku, f as f64), rot_in_hand(&o, &dark, f as f64));
                let want = m3(&BLADE_FLIP, &ra_);
                worst_rot = worst_rot.max(crate::math::angle_deg(&m3(&rb_, &tr(&want))));
                // The root where Riku's is in the hand; the tip where Riku's is from the root (in
                // Dark's turned-over `wep0` axes).
                worst = worst.max(crate::math::dist(in_hand(c, &riku, f as f64, "wep0"), in_hand(&o, &dark, f as f64, "wep0")));
                let tip = |cl: &Clip, sk: &RecomSkeleton, r: &M3| {
                    let (k, w) = (in_hand(cl, sk, f as f64, "k1"), in_hand(cl, sk, f as f64, "wep0"));
                    crate::math::vm([k[0] - w[0], k[1] - w[1], k[2] - w[2]], &tr(r))
                };
                let ta = crate::math::vm(tip(c, &riku, &ra_), &BLADE_FLIP);
                let tb = tip(&o, &dark, &rb_);
                worst = worst.max(crate::math::dist(ta, tb));
            }
        }
        eprintln!("Soul Eater in the hand: worst {worst:.4} m");
        // Riku's hidden blade stays hidden: every frame where his `wep0` is scaled, Dark's is too.
        let (wa, wb) = (riku.find("wep0").unwrap(), dark.find("wep0").unwrap());
        let mut scaled = 0;
        for c in ann.slots.iter().filter_map(|s| s.clip.as_ref()).filter(|c| c.chans() == 9) {
            let o = retarget(c, &riku, &dark, &map).unwrap();
            assert_eq!(o.chans(), 9, "{}", c.name_str());
            for f in 0..c.frames as usize {
                let (a, b) = (c.sample(wa, f as f64).s, o.sample(wb, f as f64).s);
                assert!((0..3).all(|k| (a[k] - b[k]).abs() < 1e-3), "{} frame {f}: {a:?} {b:?}", c.name_str());
                scaled += (a[0] < 0.5) as usize;
            }
        }
        eprintln!("frames with Riku's blade hidden, kept hidden: {scaled}");
        assert!(scaled > 0);
        assert!(worst < 0.03, "the blade moved in the hand by {worst} m");
        eprintln!("Soul Eater's turn in the hand against Riku's turned over: worst {worst_rot:.2} deg");
        assert!(worst_rot < 0.5);
        for &(r, d, k) in &SKIN_GROUPS {
            let b = build(&arc, &keys, r, d, k).unwrap();
            let get = |ext: &str| &b.files.iter().find(|f| f.0.ends_with(ext)).unwrap().1;
            let sk = RecomSkeleton::parse(get(".MDL")).unwrap();
            assert_eq!(sk.bones[1].name, "ex_1010_sk");
            let a = Ann::parse(get(".ANN")).unwrap();
            assert!(a.slots.iter().flatten_clips().all(|c| c.bones as usize == sk.bones.len()));
            assert!(a.find(DARK_FLIP).is_some(), "the flip in the pack");
            assert!(crate::eet::parse(get(".EET")).unwrap().iter().any(|r| r.name == DARK_FLIP));
            assert!(crate::ses::parse(get(".SES")).unwrap().iter().any(|e| e.name() == DARK_FLIP));
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
