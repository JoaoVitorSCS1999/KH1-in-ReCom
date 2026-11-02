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
