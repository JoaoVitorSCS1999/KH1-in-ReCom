//! KH1 FM's casting Heartless family (M11, `docs/recom-m11-enemies.md` § The Red Nocturne): the Red Nocturne
//! (`xa_ex_2110` / `ex_2110.bd`), Blue Rhapsody (`2120`), Yellow Opera (`2130`) and Green Requiem (`2140`). One model
//! family, four AI scripts: they share KH1's library routines (`crate::enemy::script`) and, among themselves, the few
//! routines below, byte for byte up to their relative calls (Checked (files): the four scripts compared routine by
//! routine, 2026-11-03). Everything else (their rounds, their spells, their hit handlers) is each one's own.
//!
//! | Routine | Red Nocturne | Blue Rhapsody | Yellow Opera | Green Requiem | Here |
//! |---|---|---|---|---|---|
//! | one frame of circling steering, facing a target | `0x9FA` | `0x91A` | — | — | [`turn_step`] |
//! | the idle hover leg | `0x93E` | `0x85E` | — | — | [`wander_leg`] |
//! | fly to a point (the approach, the flee) | `0xABC` | `0x9DC` | `0xBE2` | — | [`fly_to`] |
//! | the stuck test (`sys 118`) | `0xFBA` | `0xEDA` | `0x10E0` | — | [`stuck`] |
//!
//! Their spells are child objects (`sys 94`, part 3 of the `.mdls`) that wait on their parent's motion before they act:
//! [`Cast`] is that wait (the Red Nocturne's `ex_2110_00.bd` / `ex_2110_01.bd` entry `0x6B6`); what the spell does once
//! it acts is the host's (a Re:CoM bullet moved by KH1's numbers, `kh1-recom`'s `win::kh1_thrown`).
//!
//! The constants are each enemy's own (its manifest); the routines take them as arguments.

use crate::enemy::script::{frame, height, pos, push, steer, Cx};
use crate::enemy::{Cmd, Numbers};
use crate::math::Vec3;

/// `sys 10(v, deg)` on a direction: turned about the vertical, its height kept (the convention every port uses; the
/// sign: Assumption, `crate::gargoyle::rotate`'s).
pub fn rotdeg(v: Vec3, deg: f32) -> Vec3 {
    let l = v.horizontal_length();
    Vec3::from_yaw(v.yaw() + deg.to_radians()) * l + Vec3::new(0.0, v.y, 0.0)
}

/// `sys 9`: normalised (a zero vector stays zero).
pub fn norm(v: Vec3) -> Vec3 {
    if v.length() > 0.0 {
        v.normalized()
    } else {
        v
    }
}

/// Whom [`turn_step`] faces at once (`0x1BAC(actor, who)`: `sys 11` with `who.pos − actor.pos`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Face {
    /// The target's position (`*(actor + 0x74) + 0x10`), when it has one.
    At(Option<Vec3>),
    /// The actor itself (the idle leg passes `wp+4`, the actor): `sys 11` of a zero vector, `0x1402B9EF0`:
    /// `atan2f(0, 0)` = 0, so the facing becomes yaw 0 every frame (Checked (decomp): `0x1402B9EF0`,
    /// `0x140297E10`). KH1's own quirk, kept.
    Itself,
}

/// `0x9FA(who, dir, turn, speed)` (Blue Rhapsody `0x91A`, the same bytes): `dir` turned by `turn` × step degrees
/// (`sys 10`), the actor's steering `+0x20` = `dir` with its height 0, `+0x2C` = `speed`; then it faces `who` at once.
/// Returns the steering direction (the caller may write its height, `+0x24`, after the call, as every caller does).
pub fn turn_step<C, A, X>(cx: &Cx<C, A, X>, dir: &mut Vec3, turn: f32, speed: f32, who: Face) -> Vec3 {
    let s = cx.borrow().w.step;
    *dir = rotdeg(*dir, turn * s);
    let d = Vec3::new(dir.x, 0.0, dir.z);
    steer(cx, d, speed);
    let p = pos(cx);
    let y = match who {
        Face::At(Some(t)) => {
            let v = t - p;
            Some(v.x.atan2(v.z))
        }
        // No target: KH1's `sys 66` (a null check); nothing turns.
        Face::At(None) => None,
        Face::Itself => Some(0.0f32.atan2(0.0)),
    };
    if let Some(y) = y {
        let mut io = cx.borrow_mut();
        io.body.yaw = y;
        io.body.yaw_target = y;
        io.out.push(Cmd::Yaw(y));
    }
    d
}

/// The same frame's steering with its height (`+0x24`) set: the walk the body follows (`+0x20` × `+0x2C`).
pub fn steer_y<C, A, X>(cx: &Cx<C, A, X>, d: Vec3, y: f32, speed: f32) {
    steer(cx, Vec3::new(d.x, y, d.z), speed);
}

/// `0xFBA(k)` (`sys 118`: the last frame's displacement `+0x10 − +0x490`): stuck when it is shorter than
/// `k × step × half` (`half` the routine's 0.5).
pub fn stuck<C, A, X>(cx: &Cx<C, A, X>, k: f32, half: f32) -> bool {
    let io = cx.borrow();
    io.w.moved - k * io.w.step * half < 0.0
}

/// The numbers of [`wander_leg`] (the Red Nocturne's `0x93E` pushes; Blue Rhapsody's `0x85E` the same bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wander {
    /// The idle hover's request (219) and its blend (8).
    pub request: u16,
    pub blend: f32,
    /// `5`: degrees a frame the direction turns.
    pub turn: f32,
    /// `0.3`: the height of the steering (down, y down); up (× −1) when lower than `low` above the floor.
    pub sink: f32,
    pub flip: f32,
    pub low: f32,
    /// The leg's length [min, max) frames.
    pub min: f32,
    pub max: f32,
    pub speed: f32,
}

/// `0x93E` (Blue Rhapsody `0x85E`): one idle leg: along its facing turning `turn`° a frame at `speed`, facing yaw 0
/// ([`Face::Itself`]), sinking (`sink` × speed a frame) or, lower than `low` above the floor (`sys 89`), rising, for
/// [min, max) frames, its idle hover playing.
pub async fn wander_leg<C: Numbers, A, X>(cx: &Cx<C, A, X>, n: Wander) {
    let mut d = Vec3::from_yaw(cx.borrow().body.yaw);
    let mut vy = n.sink;
    if height(cx) - n.low < 0.0 {
        vy *= n.flip;
    }
    let mut t = crate::enemy::script::rand_range(cx, n.min, n.max, "wander leg");
    while t > 0.0 {
        play(cx, n.request, n.blend);
        let s = turn_step(cx, &mut d, n.turn, n.speed, Face::Itself);
        steer_y(cx, s, vy, n.speed);
        t -= cx.borrow().w.step;
        frame().await;
    }
}

/// `sys 12` (`0x14029F7C0`): a motion request, nothing when it already plays.
pub fn play<C, A, X>(cx: &Cx<C, A, X>, r: u16, blend: f32) {
    {
        let mut io = cx.borrow_mut();
        if io.w.motion == Some(r) {
            return;
        }
        io.clock = (Some(r), 0.0);
    }
    push(cx, Cmd::Motion { request: r, blend });
}

/// The numbers of [`fly_to`] (`0xABC`'s seven arguments; the routine's own: the blend 8 at `0xB5E`, the stuck
/// factor 0.3 at `0xC3C`, "no motion" 999 at `0xB48`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlyTo {
    pub request: u16,
    pub speed: f32,
    /// Done within this distance of the point (3D).
    pub stop: f32,
    /// The point is aimed at this much above (y down: `point.y − up`).
    pub up: f32,
    /// Its height-above-floor test (`sys 89 > high`).
    pub high: f32,
    pub time: f32,
    pub blend: f32,
    pub stuck: f32,
    pub stuck_half: f32,
    pub no_motion: u16,
}

/// `0xABC(motion, point, speed, stop, up, high, time)` (Blue Rhapsody `0x9DC`, Yellow Opera `0xBE2`: the same bytes):
/// each frame toward `point()` (re-read each frame: the approach passes the target's live position) aimed `up`
/// above it, until within `stop`, the time is out or it is stuck (`speed × 0.3`); the motion played each frame; it
/// turns toward its way (`sys 36`). While the aim is below it: no descent when it is below the point itself, else
/// (higher than `high` above the floor) the way is scaled by the speed before it is normalised (KH1's: no effect on
/// the normalised way; kept as written).
pub async fn fly_to<C: Numbers, A, X>(cx: &Cx<C, A, X>, n: FlyTo, point: impl Fn(&Cx<C, A, X>) -> Vec3) {
    let mut t = n.time;
    while t > 0.0 {
        let pt = point(cx);
        let me = pos(cx);
        let mut v = pt;
        v.y -= n.up;
        v = v - me;
        if v.length() - n.stop < 0.0 {
            break;
        }
        if n.request != n.no_motion {
            play(cx, n.request, n.blend);
        }
        if v.y > 0.0 {
            if me.y - pt.y > 0.0 {
                v.y = 0.0;
            } else if height(cx) - n.high > 0.0 {
                v.x *= n.speed;
                v.z *= n.speed;
                v.y *= n.speed;
            }
        }
        let v = norm(v);
        cx.borrow_mut().body.yaw_target = v.x.atan2(v.z);
        steer(cx, v, n.speed);
        t -= cx.borrow().w.step;
        frame().await;
        if stuck(cx, n.speed * n.stuck, n.stuck_half) {
            break;
        }
    }
}

/// What a spell child's wait did this frame ([`Cast::step`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastStep {
    Waits,
    /// Its parent's motion left the cast (a reaction, another request): it ends unseen (`end_thread`).
    Ends,
    /// It acts now (the host makes the spell).
    Launches,
}

/// A spell child's wait (the Red Nocturne's `ex_2110_00.bd` / `ex_2110_01.bd`, entry `0x6B6`; Checked (files)): while
/// its parent plays the charge request it waits; then the parent must play the cast request (else it ends); then for
/// `wait` frames (counted down by the step each frame, checked first) the parent must still play the cast request; then
/// it acts. Neither child tests its parent's HP: a dying parent's children go on by this rule (the host steps them after
/// the defeat too, `Behaviour::dead_frame`), so the killing hit's reaction motion ends them. `_01` (the big Fire,
/// `0x734..0x754`) tests the motion only while `sys 120` (`0x1402BAB10` → `0x140291FF0`: actor `+0x374` bit 0 set, bit 1
/// clear; bit 1 is set only by the actor's removal `0x140292390`; Checked (decomp)) says the parent is still there: once
/// it is removed (KH1: 30 frames after its death, its fade `0x1C9A(actor, 30)` with flag 8) the big Fire counts on and
/// flies (from the removed actor's last place). Not modelled: the host has no enemy to step after Re:CoM removes it (30
/// frames after its defeat), and it plays a reaction motion on every killing hit, so a cast never outlasts its parent's
/// defeat here. Charge effects (`sys 75` 2052 / 2054, 2053 / 2058) wait for M13.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cast {
    /// Its sub-object index (`sys 94`'s second word): the host's [`Cmd::Throw`].
    pub child: u8,
    pub charge: u16,
    pub cast: u16,
    pub wait: f32,
    left: Option<f32>,
}

impl Cast {
    pub fn new(child: u8, charge: u16, cast: u16, wait: f32) -> Cast {
        Cast { child, charge, cast, wait, left: None }
    }

    /// One frame of the child's script with its parent playing `motion`.
    pub fn step(&mut self, motion: Option<u16>, step: f32) -> CastStep {
        let left = match self.left {
            Some(l) => l,
            None => {
                if motion == Some(self.charge) {
                    return CastStep::Waits;
                }
                if motion != Some(self.cast) {
                    return CastStep::Ends;
                }
                self.wait
            }
        };
        if left > 0.0 {
            if motion != Some(self.cast) {
                return CastStep::Ends;
            }
            self.left = Some(left - step);
            return CastStep::Waits;
        }
        CastStep::Launches
    }
}

/// Steps a parent's waiting spell children once (the frame's parent motion): the [`Cmd::Throw`]s of those that act,
/// in their order; the finished ones are dropped.
pub fn step_casts<A>(casts: &mut Vec<Cast>, motion: Option<u16>, step: f32) -> Vec<Cmd<A>> {
    let mut out = Vec::new();
    casts.retain_mut(|c| match c.step(motion, step) {
        CastStep::Waits => true,
        CastStep::Ends => false,
        CastStep::Launches => {
            out.push(Cmd::Throw(c.child));
            false
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Fire child: waits through 200, needs 201, 40 frames, then acts (on the 41st frame of 201).
    #[test]
    fn a_cast_waits_its_frames_in_the_cast_motion() {
        let mut c = Cast::new(0, 200, 201, 40.0);
        for _ in 0..60 {
            assert_eq!(c.step(Some(200), 1.0), CastStep::Waits);
        }
        let mut f = 0;
        loop {
            match c.step(Some(201), 1.0) {
                CastStep::Waits => f += 1,
                CastStep::Launches => break,
                CastStep::Ends => panic!(),
            }
        }
        assert_eq!(f, 40, "the frame 201 is first seen counts");
        let mut v = vec![Cast::new(1, 202, 203, 90.0)];
        assert!(step_casts::<u8>(&mut v, Some(202), 1.0).is_empty() && v.len() == 1);
        // A reaction's motion ends it.
        assert!(step_casts::<u8>(&mut v, Some(0x48), 1.0).is_empty() && v.is_empty());
        let mut v = vec![Cast::new(0, 200, 201, 2.0)];
        let mut thrown = Vec::new();
        for _ in 0..4 {
            thrown.extend(step_casts::<u8>(&mut v, Some(201), 1.0));
        }
        assert_eq!(thrown, vec![Cmd::Throw(0)]);
        assert!(v.is_empty());
        // Neither the charge nor the cast: it ends at once (its parent never cast).
        let mut c = Cast::new(0, 200, 201, 40.0);
        assert_eq!(c.step(Some(220), 1.0), CastStep::Ends);
        // Leaving the cast midway ends it.
        let mut c = Cast::new(0, 200, 201, 40.0);
        assert_eq!(c.step(Some(201), 1.0), CastStep::Waits);
        assert_eq!(c.step(Some(220), 1.0), CastStep::Ends);
    }

    #[test]
    fn rotdeg_keeps_height_and_length() {
        let v = rotdeg(Vec3::new(0.0, 0.3, 1.0), 90.0);
        assert!((v.x - 1.0).abs() < 1e-5 && v.z.abs() < 1e-5 && v.y == 0.3);
        assert_eq!(0.0f32.atan2(0.0), 0.0);
    }
}
