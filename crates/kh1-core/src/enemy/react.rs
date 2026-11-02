//! KH1's enemy hit reactions (`0x1402B8E10` and its cases), one rule set for every enemy (moved
//! from the Shadow's port, M10; unchanged). What differs per enemy is which motions its model has
//! ([`Motions`]), its knock-back weight and its clash recoil requests (its attack rows `+0x40`).

use super::{Body, Engine};
use crate::math::{wrap_angle, Vec3};
use crate::rng::Kh1Rand;

/// KH1's motion requests the shared engine code plays (every enemy `.mset` uses the same numbers;
/// Checked (files): the Shadow's and the Soldier's request maps, `0x1402B7260`).
pub mod req {
    pub const IDLE: u16 = 0;
    pub const WALK: u16 = 1;
    pub const RUN: u16 = 2;
    pub const JUMP: u16 = 3;
    pub const FALL: u16 = 4;
    /// The air motion a reaction or an air attack queues (`0x40000006`).
    pub const AIR: u16 = 6;
    /// The landing motion of a plain fall (`param_1[0xf4] + 7`, `0x1402B7260`; base 0).
    pub const LAND: u16 = 7;
    /// Ground flinch: 0x48 / 0x49 hit from the front (part 0 / other), 0x4A / 0x4B from behind.
    pub const FLINCH: u16 = 0x48;
    /// Ground knock-back (kind 4): 0x4C front, 0x4D behind (root motion 400).
    pub const KNOCK: u16 = 0x4C;
    /// Air flinch: 0x4E front, 0x4F behind.
    pub const AIR_FLINCH: u16 = 0x4E;
    /// Air knock-back (kind 4 in the air), when the model has it.
    pub const AIR_KNOCK: u16 = 0x50;
    /// Launch on the ground: base, +1 air, +2 land, +3 lie, +4 get up; +5..+9 from behind.
    pub const LAUNCH: u16 = 0x52;
    /// Launch in the air (same layout).
    pub const AIR_LAUNCH: u16 = 0x5C;
}

/// A hit as the reaction code sees it: KH1's reaction kind against enemies (attack data `+0xC`),
/// the push direction (target − attacker, flat, unit; record `+0x10`), the hurt part (record
/// `+0x20`: 0 = the model's first hurt part, else another).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub kind: u8,
    pub dir: Vec3,
    pub part: u8,
}

/// What a reaction asks of the host.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactionStart {
    /// The motion request (an action: the AI waits) and the one queued after it.
    pub request: u16,
    pub then: u16,
    /// The facing it sets (radians), if any.
    pub yaw: Option<f32>,
    /// Launches: the base request (`+0x3EA`) the landing / lying / get-up follow from.
    pub base: Option<u16>,
    /// Lying-down time once landed (frames).
    pub down: f32,
}

/// The phase of a running reaction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Until the motion ends (flinch, knock-back, clash recoil, get-up, landing).
    Motion,
    /// In the air (launch, air flinch); `base`: the launch's base request (land = base + 2 / + 7).
    Air { base: Option<u16>, behind: bool },
    /// Lying (`(motion − base) % 5 == 3`), counting the down time.
    Down { base: u16, behind: bool, left: f32 },
}

/// Whether the model has a request (the `.mset` map names an entry): `0x14029F010`.
pub trait Motions {
    fn has(&self, request: u16) -> bool;
}

/// KH1's choice for a hit (Checked (decomp + disasm): `0x1402B8E10`, `0x1402B9230`,
/// `0x1402B8B90`, `0x1402B77D0`, `0x1402B8CC0`, `0x1402B8530`, `0x1402B7E60`).
///
/// - kind 3 on the ground: 0x48 + part (front) / 0x4A + part (behind); the same request as the
///   one playing alternates (`(r − 0x48) ^ 1`); push slide `flinch_push × weight`;
/// - kind 3 in the air: 0x4E / 0x4F, a 20 hop, air velocity dir × `air_flinch_speed × weight`;
/// - kind 4 on the ground: 0x4C / 0x4D facing along / against the push (root motion), else
///   launch, else flinch; in the air 0x50 / 0x51 (hop 20, dir × `air_knock_speed`), else the air
///   launch, else the air flinch;
/// - kind 5: launch: speed `min + (max − min) × weight`, hop speed × `launch_hop`, base 0x52 / 0x5C
///   (+5 from behind), facing against / along the push, down time rand × `down_time`;
/// - lying down: only the push slide.
///
/// `facing`: the actor's facing vector `+0x30`; "behind" is `facing · dir ≥ 0`.
#[allow(clippy::too_many_arguments)]
pub fn react<C: Engine + ?Sized>(hit: Hit, yaw: f32, current: Option<u16>, airborne: bool, lying: bool, m: &dyn Motions, c: &C, rng: &mut Kh1Rand, body: &mut Body, y: f32) -> Option<ReactionStart> {
    let behind = Vec3::from_yaw(yaw).dot(hit.dir) >= 0.0;
    let along = hit.dir.x.atan2(hit.dir.z);
    let against = (-hit.dir.x).atan2(-hit.dir.z);
    let flinch_push = |body: &mut Body| body.push(hit.dir, c.flinch_push() * c.weight());
    if lying {
        flinch_push(body);
        return None;
    }
    let launch = |body: &mut Body, rng: &mut Kh1Rand, airborne: bool| -> Option<ReactionStart> {
        let s = (c.launch_max() - c.launch_min()) * c.weight() + c.launch_min();
        let roll = rng.next_f32();
        let base = if airborne { req::AIR_LAUNCH } else { req::LAUNCH };
        if !m.has(base) {
            return None;
        }
        let (r, then, yw) = if behind { (base + 5, base + 6, along) } else { (base, base + 1, against) };
        body.yaw = wrap_angle(yw);
        body.yaw_target = body.yaw;
        body.push = None;
        body.jump(s * c.launch_hop(), hit.dir.flat().normalized() * s, y, c);
        Some(ReactionStart { request: r, then, yaw: Some(body.yaw), base: Some(base), down: roll * c.down_time() })
    };
    let air_flinch = |body: &mut Body| -> ReactionStart {
        let r = if behind { req::AIR_FLINCH + 1 } else { req::AIR_FLINCH };
        body.jump(c.air_knock_hop(), hit.dir.flat().normalized() * (c.air_flinch_speed() * c.weight()), y, c);
        ReactionStart { request: r, then: req::AIR, yaw: None, base: None, down: 0.0 }
    };
    let ground_flinch = |body: &mut Body| -> Option<ReactionStart> {
        let mut r = if behind { req::FLINCH + 2 } else { req::FLINCH };
        if hit.part != 0 {
            r += 1;
        }
        if current == Some(r) {
            r = ((r - req::FLINCH) ^ 1) + req::FLINCH;
        }
        flinch_push(body);
        m.has(req::FLINCH).then_some(ReactionStart { request: r, then: req::IDLE, yaw: None, base: None, down: 0.0 })
    };
    match hit.kind {
        3 if airborne => Some(air_flinch(body)),
        3 => ground_flinch(body),
        4 if airborne => {
            if m.has(req::AIR_KNOCK) {
                let (r, yw) = if behind { (req::AIR_KNOCK + 1, along) } else { (req::AIR_KNOCK, against) };
                body.yaw = wrap_angle(yw);
                body.yaw_target = body.yaw;
                body.jump(c.air_knock_hop(), hit.dir.flat().normalized() * c.air_knock_speed(), y, c);
                Some(ReactionStart { request: r, then: req::AIR, yaw: Some(body.yaw), base: None, down: 0.0 })
            } else if m.has(req::AIR_LAUNCH) {
                launch(body, rng, true)
            } else {
                Some(air_flinch(body))
            }
        }
        4 => {
            if m.has(req::KNOCK) {
                let (r, yw) = if behind { (req::KNOCK + 1, along) } else { (req::KNOCK, against) };
                body.yaw = wrap_angle(yw);
                body.yaw_target = body.yaw;
                Some(ReactionStart { request: r, then: req::IDLE, yaw: Some(body.yaw), base: None, down: 0.0 })
            } else if m.has(req::LAUNCH) {
                launch(body, rng, false)
            } else {
                ground_flinch(body)
            }
        }
        5 => launch(body, rng, airborne).or_else(|| if airborne { Some(air_flinch(body)) } else { ground_flinch(body) }),
        _ => None,
    }
}

/// KH1's choice for a hit on a flying enemy (movement state 4: its state flags `& 3 == 2`, so
/// `0x1402B1AE0` is true and `0x1402B1B10` false; Checked (decomp)): `0x1402B8E10` then takes the
/// ground forms. Kind 3 is the ground flinch (`0x1402B9230`, the push slide `0x1402B7E60`); kind 5
/// skips the launch (`0x1402B8530` only when `0x1402B1AE0` is false) and falls into kind 4
/// (`0x1402B77D0` → `0x1402B8CC0`: the knock-back 0x4C / 0x4D, else the launch, else the flinch);
/// a clash is the ground recoil (`0x1402B9140`, [`clash`] with `airborne` false). M11 fliers.
#[allow(clippy::too_many_arguments)]
pub fn react_flying<C: Engine + ?Sized>(hit: Hit, yaw: f32, current: Option<u16>, m: &dyn Motions, c: &C, rng: &mut Kh1Rand, body: &mut Body, y: f32) -> Option<ReactionStart> {
    let kind = if hit.kind == 5 { 4 } else { hit.kind };
    react(Hit { kind, ..hit }, yaw, current, false, false, m, c, rng, body, y)
}

/// A launch a script's hit handler asks for in place of KH1's own reaction (`sys 38` →
/// `0x1402BBC80` → `0x1402B79F0(actor, hit, base, hop, speed, down)`; the Powerwild, M11): the base
/// request on the ground and in the air (the script picks by its movement state), the hop, the
/// speed and the lying time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScriptedLaunch {
    pub ground: u16,
    pub air: u16,
    pub hop: f32,
    pub speed: f32,
    pub down: f32,
}

/// `0x1402B79F0`: the scripted launch. The model must have the base (`0x14029F010`), else KH1
/// plays its `0x1402B77D0` instead (Assumption: the reaction KH1 already chose stands; none is
/// None here). From behind (`facing · dir ≥ 0`) base + 5 with base + 6 queued, facing along the
/// push; else base with base + 1 queued, facing against it. The hop `0x1402AF960(hop)`, the air
/// velocity the push direction (flat, unit) × speed; the lying time is `down`. Checked (decomp).
#[allow(clippy::too_many_arguments)]
pub fn scripted_launch<C: Engine + ?Sized>(hit: Hit, l: ScriptedLaunch, airborne: bool, yaw: f32, m: &dyn Motions, c: &C, body: &mut Body, y: f32) -> Option<ReactionStart> {
    let base = if airborne { l.air } else { l.ground };
    if !m.has(base) {
        return None;
    }
    let behind = Vec3::from_yaw(yaw).dot(hit.dir) >= 0.0;
    let (r, then, yw) = if behind { (base + 5, base + 6, hit.dir.x.atan2(hit.dir.z)) } else { (base, base + 1, (-hit.dir.x).atan2(-hit.dir.z)) };
    body.yaw = wrap_angle(yw);
    body.yaw_target = body.yaw;
    body.jump(l.hop, hit.dir.flat().normalized() * l.speed, y, c);
    Some(ReactionStart { request: r, then, yaw: Some(body.yaw), base: Some(base), down: l.down })
}

/// The clash recoil a blocked attack plays: the attack row's `+0x40` request (`0x1402B8E10` case
/// 6 / 7 → `0x1402B9140`): on the ground the request as an action with the idle queued; in the air
/// with a 20 hop and request 6 queued (`0x40000006`). The push of a negative-damage record is not
/// built (Assumption: a block's record carries no negative damage).
pub fn clash<C: Engine + ?Sized>(recoil: u16, airborne: bool, body: &mut Body, y: f32, c: &C) -> ReactionStart {
    if airborne {
        body.jump(c.air_knock_hop(), Vec3::ZERO, y, c);
        ReactionStart { request: recoil, then: req::AIR, yaw: None, base: None, down: 0.0 }
    } else {
        ReactionStart { request: recoil, then: req::IDLE, yaw: None, base: None, down: 0.0 }
    }
}

/// After a landing (`0x1402B7260`, top): the motion that follows a reaction in the air.
/// Launch base / base+1 → base+2 then base+3 (lying); behind +5 / +6 → +7 then +8; anything
/// else → request 7, then the idle.
pub fn landing(base: Option<u16>, behind: bool) -> (u16, u16, bool) {
    match base {
        Some(b) if behind => (b + 7, b + 8, true),
        Some(b) => (b + 2, b + 3, true),
        None => (req::LAND, req::IDLE, false),
    }
}

/// The get-up after lying (`(motion − base) % 5 == 3` → motion + 1, the idle queued).
pub fn get_up(base: u16, behind: bool) -> u16 {
    if behind {
        base + 9
    } else {
        base + 4
    }
}

/// Which hurt part a contact point is nearest to (the model's hurt parts' bone positions, in
/// order); KH1 tells its parts apart by the part record that was hit; Re:CoM's twins have their own
/// hurt shapes (decision 24), so the contact point's nearest part bone decides (decision M10:
/// "flinch by hurt part derived from the hit's height"). Ties go to the earlier part.
pub fn nearest_part(contact: Vec3, parts: &[Vec3]) -> u8 {
    let mut best = (0u8, f32::INFINITY);
    for (i, p) in parts.iter().enumerate() {
        let d = (contact - *p).length();
        if d < best.1 {
            best = (i as u8, d);
        }
    }
    best.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::Table;

    struct Has(&'static [u16]);
    impl Motions for Has {
        fn has(&self, r: u16) -> bool {
            self.0.contains(&r)
        }
    }

    fn consts() -> Table {
        let mut v: Vec<(String, f32)> = crate::enemy::EXE_NUMBERS.iter().map(|(k, x)| (k.to_string(), *x)).collect();
        for (k, x) in [("walk_speed", 8.0), ("run_speed", 8.0), ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("weight", 1.0), ("attack_blend", 8.0)] {
            v.push((k.to_string(), x));
        }
        Table::new(v, &[]).unwrap()
    }

    /// A flier (`F K a A` models, no launch): kind 3 is the ground flinch with its push; kind 4 and
    /// kind 5 the knock-back 0x4C / 0x4D (`0x1402B8E10` with `0x1402B1AE0` true); no hop.
    #[test]
    fn flying_hits_take_the_ground_forms() {
        let m = Has(&[0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50, 0x51]);
        let c = consts();
        let mut rng = Kh1Rand::new(1);
        let front = Vec3::new(0.0, 0.0, -1.0);
        for (kind, want) in [(3u8, 0x48u16), (4, 0x4C), (5, 0x4C)] {
            let mut b = Body::default();
            b.fly(Vec3::ZERO);
            let s = react_flying(Hit { kind, dir: front, part: 0 }, 0.0, None, &m, &c, &mut rng, &mut b, 0.0).unwrap();
            assert_eq!(s.request, want, "kind {kind}");
            assert!(b.flying && b.arc.is_none(), "kind {kind}: still flying, no hop");
            assert_eq!(b.push.is_some(), kind == 3, "kind {kind}: only the flinch pushes");
        }
        // The same kind 5 on a jumping (state 2) body is an air reaction.
        let mut b = Body::default();
        b.jump(20.0, Vec3::ZERO, 0.0, &c);
        let s = react(Hit { kind: 5, dir: front, part: 0 }, 0.0, None, true, false, &m, &c, &mut rng, &mut b, 0.0).unwrap();
        assert_eq!(s.request, 0x4E);
    }

    /// The Powerwild's `sys 38(hit, 270 / 280, 80, 25, 0)`: from the front base 270 with 271 queued,
    /// facing against the push; from behind 275 with 276 queued; hop 80, speed 25, no lying time; in
    /// the air base 280. A model without the base keeps KH1's own reaction.
    #[test]
    fn a_scripted_launch_replaces_the_reaction() {
        let c = consts();
        let l = ScriptedLaunch { ground: 270, air: 280, hop: 80.0, speed: 25.0, down: 0.0 };
        let m = Has(&[270, 280]);
        // Facing +z, pushed toward −z: from the front.
        let mut b = Body::default();
        let s = scripted_launch(Hit { kind: 5, dir: Vec3::new(0.0, 0.0, -1.0), part: 0 }, l, false, 0.0, &m, &c, &mut b, 0.0).unwrap();
        assert_eq!((s.request, s.then, s.base, s.down), (270, 271, Some(270), 0.0));
        assert!((b.yaw - 0.0).abs() < 1e-6 && b.airborne());
        assert!((b.air_vel.z + 25.0).abs() < 1e-4, "{:?}", b.air_vel);
        // Pushed along its facing: from behind.
        let mut b = Body::default();
        let s = scripted_launch(Hit { kind: 4, dir: Vec3::new(0.0, 0.0, 1.0), part: 0 }, l, true, 0.0, &m, &c, &mut b, 0.0).unwrap();
        assert_eq!((s.request, s.then), (285, 286));
        assert!(scripted_launch(Hit { kind: 4, dir: Vec3::new(0.0, 0.0, 1.0), part: 0 }, l, false, 0.0, &Has(&[]), &c, &mut Body::default(), 0.0).is_none());
    }
}
