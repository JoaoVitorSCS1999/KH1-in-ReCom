//! KH1 FM's Bouncywild (`xa_ex_2040.mdls` part 5, `ex_2040.bd`; its children `ex_2040_00.bd` (the thrown banana),
//! `ex_2040_01.bd` (the tossed peel) and `ex_2040_02.bd` (the dropped peel), part 3), engine-agnostic: its AI script on
//! the shared enemy framework (`crate::enemy`). M11 (`docs/recom-m11-enemies.md` § The Bouncywild). Re:CoM runs it
//! natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), Re:CoM plays the motions,
//! collides the body, the banana and the peels (its own bullets), applies the damage and Sora's slip.
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's scripts at checked offsets); nothing of
//! the game is in this file. Script offsets are the player's `ex_2040.bd`'s (Checked (files): the script disassembled
//! with `tools/kh1-ai-script.py`, code from `0x300`; the VM's ops and the syscalls named, Checked (decomp)).
//!
//! # The script's shape (the Powerwild's older shape: state object `wp+0x20`, search `wp+0x30`)
//!
//! - entry `0x1DD8`: init `0x1EBC` (records LV1 `0x14` / LV2 `0x80`; walk 5, run 13, hop 500; home; handlers: hit
//!   `0x1FAC`, message `0x1F4C`, `0x1FA0`); a yield; the appearance thread `0x550`; then each frame: while idle (state
//!   1) Sora within `idle_search` → the target and engaged; a state request restarts the behaviour thread: 1 idle
//!   `0x4F2`, 2 engaged `0x39C`, 3 hit `0x300`.
//! - It has **no attack on its body** (no HIT track, no row on a bone): it bounces about, leaps over Sora, runs away
//!   dropping banana peels, tosses peels and throws a homing banana (its children: [`child`]).
//! - **It runs away for good** (`0x92C`: request 70, then a 30-frame fade and its removal, `sys 129`, no EXP, no
//!   prizes) after five rounds, after its taunt, after a hit from afar and after its own launch. Not ported (owner
//!   question, as KH1's leash: a Re:CoM battle could not end): where KH1's Bouncywild leaves, this one starts its five
//!   rounds again ([`escape`]).
//!
//! Not ported (Re:CoM's battles only, decision M10): the escape (above), the leash `0x8D4` (out of sight and 1500 away,
//! or 1800), the room's helpers its call summons (`sys 37(0xB0000 | 2)`: none in Re:CoM's battles, so its call always
//! ends in 207), the slip of another enemy on its peels (message 1001 → `0x7FC`: the drops `0x270` and 208; Re:CoM's
//! peels never touch enemies), the effects (`sys 106` / `sys 110` / `sys 75`: M13), world 11's search rule, the hit
//! attributes 8 / 9 / 10 / 11 (no attribute carried), the death routine `0x11F0` (Re:CoM's defeat, prizes, EXP).

use crate::enemy::react::ScriptedLaunch;
use crate::enemy::script::{dist_to_target, frame, motion_done, motion_frames, move_toward, poll, pos, push, rand, rand_range, set_state, set_target, take_state, target, tick_clock, wait_frames, wait_grounded, walk_to, wander};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers as _, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Bouncywild's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The appearance (`0x1A2A`: `sys 13`, blend 0), 80 frames.
    pub const APPEAR: u16 = 69;
    /// The escape (`0x19D0`: `0x40000046`), 60 frames: not ported (its escape is left out).
    pub const ESCAPE: u16 = 70;
    /// The leap's motions (`0x180A` with base 3): the crouch 3, rising 4 (bit 16), 5 at the top, 6 queued.
    pub const LEAP: u16 = 3;
    /// The peel toss (`0x9B8`, `sys 13`, blend 0), 140 frames: the peel leaves at frame 80.
    pub const TOSS: u16 = 200;
    /// The bounce-about's start (`0xA30`), 48 frames.
    pub const DANCE: u16 = 202;
    /// Its bounding legs (`0xB56`, `sys 12`; the run's clip), 52 frames, looping.
    pub const BOUNCE: u16 = 203;
    /// The pose after the bounce-about and the run-away (`0x13FA`: an action, blend 0), 46 frames.
    pub const POSE: u16 = 204;
    /// The turn after a near hit (`0x850`), 122 frames.
    pub const TURN: u16 = 205;
    /// The call (`0x600`: an action), 130 frames, and its end 207 (action, blend 12), 240 frames.
    pub const CALL: u16 = 206;
    pub const CALL_END: u16 = 207;
    /// The first look (`0x89A`, a placed one only), 80 frames.
    pub const NOTICE: u16 = 209;
    /// The banana throw (`0x952`), 140 frames: the banana leaves at frame 82.
    pub const THROW: u16 = 211;
    /// The recovery after its own launch (`0x838`: an action), 204 frames.
    pub const RECOVER: u16 = 212;
    /// The taunt (`0x736`), 98 frames, looping, and its end 217 (`0x7BE`), 134 frames.
    pub const TAUNT: u16 = 213;
    pub const TAUNT_END: u16 = 217;
    /// The look-about before it leaves (`0x59E`), 240 frames.
    pub const READY: u16 = 215;
    /// The idle's stretch (`0x820`: an action), 266 frames.
    pub const STRETCH: u16 = 216;
}

/// No attack of its own makes rows on its body (KH1's Bouncywild has no HIT track; Checked (files)): its banana and its
/// peels are its children's ([`child`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {}

impl Attack {
    pub const ALL: [Attack; 0] = [];

    pub fn request(self) -> u16 {
        match self {}
    }

    pub fn name(self) -> &'static str {
        match self {}
    }

    pub fn index(self) -> u8 {
        match self {}
    }
}

/// Its children (`sys 94`'s sub-object index): the thrown objects the host makes with [`ECmd::Throw`].
pub mod child {
    /// `ex_2040_00.bd` with the row `0xEC` (batted: `0x15C`): the banana, a Re:CoM bullet turning 1.25° a frame toward
    /// Sora (the Fat Bandit's / Defender's fireball code).
    pub const BANANA: u8 = 0;
    /// `ex_2040_01.bd`: a peel tossed along its facing (80 frames into 200), then lying 120 frames: Sora slips on it.
    pub const PEEL: u8 = 1;
    /// `ex_2040_02.bd`: a peel dropped toward Sora as it runs away (at once).
    pub const DROP: u8 = 2;
}

/// The names of the Bouncywild's numbers (besides `crate::enemy::ENGINE_NAMES`); the host's own are [`HOST_NAMES`].
pub const NAMES: &[&str] = &[
    "hop_height",
    "idle_search",
    "appear_blend",
    "appear_fade",
    "appear_wait",
    "appear_then_blend",
    "wander_inner",
    "wander_outer",
    "rest_blend",
    "rest_min",
    "rest_max",
    "stretch_blend",
    "wander_band",
    "wander_timeout",
    "wander_idle_chance",
    "wander_idle_blend",
    "wander_idle_min",
    "wander_idle_max",
    "wander_home_max",
    "rounds",
    "close_range",
    "mid_range",
    "far_banana_chance",
    "mid_peel_chance",
    "ready_blend",
    "ready_close",
    "call_blend",
    "call_end_blend",
    "taunt_blend",
    "taunt_min",
    "taunt_range",
    "taunt_end_blend",
    "recover_blend",
    "turn_blend",
    "notice_blend",
    "throw_blend",
    "toss_blend",
    "dance_blend",
    "dance_close",
    "dance_legs",
    "dance_min",
    "dance_max",
    "dance_leg_time",
    "dance_fx_every",
    "bounce_blend",
    "bounce_speed",
    "stuck_speed",
    "arrive_dist",
    "pose_wait",
    "flee_dist",
    "flee_first",
    "flee_time",
    "leap_speed",
    "leap_height",
    "leap_drag",
    "leap_time",
    "stuck_factor",
    "hit_range",
    "own_launch_hop",
    "own_launch_speed",
    "own_launch_down",
    "fire_wait",
    "peel_wait",
];

/// Numbers only the host reads (its children's flights and Re:CoM's power).
pub const HOST_NAMES: &[&str] = &[
    "fire_child",
    "peel_child",
    "drop_child",
    "peel_count",
    "drop_count",
    "fire_t",
    "fire_turn",
    "fire_speed",
    "fire_life",
    "fire_fade",
    "fire_radius",
    "fire_recom_id",
    "fire_kind",
    "fire_at_bone",
    "fire_at_x",
    "fire_at_y",
    "fire_at_z",
    "peel_sweep",
    "peel_gravity",
    "peel_lie",
    "peel_reach",
    "peel_protect",
    "peel_t",
    "peel_spread",
    "peel_speed_min",
    "peel_speed_max",
    "peel_up_min",
    "peel_up_max",
    "peel_spread_min",
    "peel_spread_max",
    "drop_t",
    "drop_speed_min",
    "drop_speed_max",
    "drop_up_min",
    "drop_up_max",
];

/// Every name a manifest must hold ([`NAMES`] and [`HOST_NAMES`]).
pub fn all_names() -> Vec<&'static str> {
    NAMES.iter().chain(HOST_NAMES).copied().collect()
}

/// The Bouncywild's numbers.
pub type Consts = Table;

/// The Bouncywild's state objects (`wp+0x20`; KH1's numbers).
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 3;
}

/// What the hit behaviour does (`wp+0x38`, set by the hit handler `0x1FAC` / the message handler `0x1F4C`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum After {
    /// 2: within `hit_range` the turn and the run-away, else the look-about.
    #[default]
    Counter,
    /// 4: its own launch played: the recovery 212, then the look-about.
    Launched,
}

/// The Bouncywild's own work area: the first round done (`wp+0x3C`), the rounds (`wp+0x44`), what the hit behaviour does
/// (`wp+0x38`), spawned by the room (`sys 132`), the appearance done, paused by it (flag 4 while 69 plays), the last
/// hit's kind, the launch it asked for, the escapes left out (for the log and the tests).
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    engaged_once: bool,
    rounds: f32,
    after: After,
    spawned: bool,
    appeared: bool,
    paused: bool,
    hit_kind: u8,
    launch: Option<ScriptedLaunch>,
    escapes: u32,
}

type Cx = crate::enemy::script::Cx<Consts, Attack, Wp>;
type Task = crate::enemy::script::Task;
pub type Cmd = ECmd<Attack>;

fn k(cx: &Cx, n: &str) -> f32 {
    cx.borrow().c.num(n)
}

fn step(cx: &Cx) -> f32 {
    cx.borrow().w.step
}

/// The request playing as the script sees it (`sys 51`): the last one this frame asked for, else the host's.
fn playing(cx: &Cx) -> Option<u16> {
    let io = cx.borrow();
    io.out
        .iter()
        .rev()
        .find_map(|c| match c {
            ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Restart { request, .. } | ECmd::Action { request, .. } => Some(*request),
            _ => None,
        })
        .or(io.w.motion)
}

/// `sys 12` (`0x14029F7C0`): a plain request, nothing when it already plays; `sys 172`'s clock restarts for a request
/// that plays anew.
fn play(cx: &Cx, r: u16, blend: f32) {
    if playing(cx) != Some(r) {
        cx.borrow_mut().clock = (Some(r), 0.0);
        push(cx, ECmd::Motion { request: r, blend });
    }
}

/// `sys 13` (`0x1402B9F80` → `0x14029F980`): a plain request written whatever plays: the request playing starts again.
fn request(cx: &Cx, r: u16, blend: f32) {
    let again = playing(cx) == Some(r);
    cx.borrow_mut().clock = (Some(r), 0.0);
    push(cx, if again { ECmd::Restart { request: r, blend, start: 0.0 } } else { ECmd::Motion { request: r, blend } });
}

/// `0x13FA(r, blend, queue blend)`: an action (bit 24) with the idle queued, then the yield (the action pauses the
/// script until it ends).
async fn action(cx: &Cx, r: u16, blend: f32) {
    push(cx, ECmd::Action { request: r, attack: None, then: req::IDLE, blend });
    frame().await;
}

/// `0x1B24`: facing Sora at once (`sys 11`).
fn face_now(cx: &Cx) {
    if let Some(t) = target(cx) {
        let d = t - pos(cx);
        let y = d.x.atan2(d.z);
        let mut io = cx.borrow_mut();
        io.body.yaw = y;
        io.body.yaw_target = y;
        io.out.push(ECmd::Yaw(y));
    }
}

/// `0x1B24(actor, Sora)`: facing Sora at once (the appearance's `func_153a`: party member 0, whatever the target).
fn face_sora_now(cx: &Cx) {
    let t = cx.borrow().w.target;
    if let Some(t) = t {
        let d = t - pos(cx);
        let y = d.x.atan2(d.z);
        let mut io = cx.borrow_mut();
        io.body.yaw = y;
        io.body.yaw_target = y;
        io.out.push(ECmd::Yaw(y));
    }
}

/// `0x1AFA`: turning toward the target at the turn rate (`sys 36`).
fn face(cx: &Cx) {
    if let Some(t) = target(cx) {
        let d = t - pos(cx);
        cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
    }
}

/// `0x137A`: until the motion is done (`sys 172`) or the fall (6) plays, turning toward the target.
async fn until_done(cx: &Cx) {
    loop {
        if motion_done(cx) || playing(cx) == Some(req::AIR) {
            break;
        }
        face(cx);
        frame().await;
    }
}

/// `0xE4C(speed)`: stuck: it moved under speed × step × 0.5 last frame (`sys 118`: `+0x10 − +0x490`).
fn stuck(cx: &Cx, speed: f32) -> bool {
    let (moved, s) = {
        let io = cx.borrow();
        (io.w.moved, io.w.step)
    };
    moved - speed * s * k(cx, "stuck_factor") < 0.0
}

/// `0x92C`: KH1's escape: facing the target, `sys 129`, 70 and a 30-frame fade, then it is removed (no EXP, no prizes)
/// and its script waits forever. **Left out** (owner question, as the leash: a Re:CoM battle could not end): its rounds
/// start again (`wp+0x44` = 0) and the behaviour goes on where KH1's would have stopped.
async fn escape(cx: &Cx) {
    face(cx);
    {
        let mut io = cx.borrow_mut();
        io.x.rounds = 0.0;
        io.x.escapes += 1;
    }
    push(cx, ECmd::Note("escape (KH1 0x92C: 70, a 30-frame fade, removed) left out: its rounds start again"));
}

/// `0xD4A`: the leap: facing Sora at once; `0x180A(actor, facing × leap_speed with leap_height, leap_drag, leap_time,
/// 3)`: request 3 for its length, then 4 (bit 16: a plain request here, Assumption as the Powerwild's hops) and movement
/// state 3 (`sys 82`, [`Body::timed_jump`]) with the motion base 3 (`sys 83`): six frames before the top 5 with 6
/// queued (`0x1402AF630`); it waits while the jump lasts. It doesn't turn in the air.
async fn leap(cx: &Cx) {
    face_now(cx);
    let yaw = cx.borrow().body.yaw;
    let d = Vec3::from_yaw(yaw) * k(cx, "leap_speed");
    let v = Vec3::new(d.x, k(cx, "leap_height"), d.z);
    request(cx, req::LEAP, 0.0);
    wait_frames(cx, motion_frames(cx, req::LEAP)).await;
    request(cx, req::LEAP + 1, 0.0);
    let (drag, time) = (k(cx, "leap_drag"), k(cx, "leap_time"));
    cx.borrow_mut().body.timed_jump(v, drag, time);
    let mut apex = false;
    while cx.borrow().body.timed.is_some() {
        // `0x1402AF630` runs in the engine's step (after the script): here at the next frame's script (Assumption: ±1
        // frame, as the Powerwild's).
        let (at_top, cur) = {
            let io = cx.borrow();
            (io.body.timed_apex(), io.w.motion)
        };
        if !apex && at_top && cur == Some(req::LEAP + 1) {
            apex = true;
            play(cx, req::LEAP + 2, 0.0);
            push(cx, ECmd::Queue { request: req::LEAP + 3, blend: 0.0 });
        }
        frame().await;
    }
}

/// `0xC24`: the run-away: a point `flee_dist` (−800) along the way to Sora; run there (`0x1B6C`, mode 1) for
/// `flee_first` frames; drop a peel (`0xD18`); run on for at most `flee_time` frames until stuck or within
/// `arrive_dist`; drop another; the pose 204 and `pose_wait` frames; on the ground.
async fn run_away(cx: &Cx) {
    let Some(t) = target(cx) else { return };
    let me = pos(cx);
    let goal = me + (t - me).normalized() * k(cx, "flee_dist");
    walk_to(cx, goal, 1, k(cx, "flee_first")).await;
    push(cx, ECmd::Throw(child::DROP));
    let mut time = 0.0;
    while time - k(cx, "flee_time") < 0.0 {
        let r = move_toward(cx, goal, 1, 0.0);
        time += step(cx);
        frame().await;
        if stuck(cx, k(cx, "stuck_speed")) || r - k(cx, "arrive_dist") <= 0.0 {
            break;
        }
    }
    push(cx, ECmd::Throw(child::DROP));
    action(cx, req::POSE, 0.0).await;
    wait_frames(cx, k(cx, "pose_wait")).await;
    frame().await;
    wait_grounded(cx).await;
}

/// `0xA30`: the bounce-about: on the ground; 202 (blend 8) to its end or Sora within `dance_close`; then `dance_legs`
/// legs, each to a point [`dance_min`, `dance_max`) away at a random angle about where it stands, bounding (203, `sys
/// 12`) and gliding at `bounce_speed` (`sys 40` mode 2) for at most `dance_leg_time` frames, until stuck or within
/// `arrive_dist`; its effect every `dance_fx_every` frames (M13); then the pose 204 and `pose_wait` frames.
async fn dance(cx: &Cx) {
    let mut fx = 0.0;
    wait_grounded(cx).await;
    request(cx, req::DANCE, k(cx, "dance_blend"));
    loop {
        if motion_done(cx) {
            break;
        }
        face(cx);
        if dist_to_target(cx) - k(cx, "dance_close") < 0.0 {
            break;
        }
        frame().await;
    }
    push(cx, ECmd::Note("bounce-about effect (sys 106 8 / 12): M13"));
    let mut legs = k(cx, "dance_legs");
    while legs > 0.0 {
        let len = rand_range(cx, k(cx, "dance_min"), k(cx, "dance_max"), "leg length");
        let ang = rand_range(cx, 0.0, 360.0, "leg angle");
        // `sys 10` turns (len, 0, 0) by `ang` degrees: (sin, 0, cos) of yaw 90° + ang (the ports' sign).
        let goal = pos(cx) + Vec3::from_yaw((90.0 + ang).to_radians()) * len;
        let mut t = 0.0;
        while t - k(cx, "dance_leg_time") < 0.0 {
            if fx - k(cx, "dance_fx_every") >= 0.0 {
                fx = 0.0;
            }
            play(cx, req::BOUNCE, k(cx, "bounce_blend"));
            let r = move_toward(cx, goal, 2, k(cx, "bounce_speed"));
            let s = step(cx);
            t += s;
            fx += s;
            frame().await;
            if stuck(cx, k(cx, "stuck_speed")) || r - k(cx, "arrive_dist") <= 0.0 {
                break;
            }
        }
        legs -= 1.0;
        frame().await;
    }
    action(cx, req::POSE, 0.0).await;
    wait_frames(cx, k(cx, "pose_wait")).await;
    frame().await;
}

/// `0x952`: the banana: on the ground; its child `ex_2040_00.bd` (rows `0xEC` / `0x15C`); 211 (blend 8) to its end,
/// turning toward the target; the idle queued. The child leaves `fire_wait` frames after it was made, only while 211
/// still plays (a reaction's motion ends it): the throw is asked for then.
async fn banana(cx: &Cx) {
    wait_grounded(cx).await;
    request(cx, req::THROW, k(cx, "throw_blend"));
    let mut t = 0.0;
    let mut thrown = false;
    loop {
        if !thrown && t - k(cx, "fire_wait") >= 0.0 {
            thrown = true;
            if playing(cx) == Some(req::THROW) {
                push(cx, ECmd::Throw(child::BANANA));
            }
        }
        if motion_done(cx) || playing(cx) == Some(req::AIR) {
            break;
        }
        face(cx);
        t += step(cx);
        frame().await;
    }
    push(cx, ECmd::Queue { request: req::IDLE, blend: k(cx, "attack_blend") });
}

/// `0x9B8`: the peel toss: on the ground; its effect (M13); its child `ex_2040_01.bd` (one peel); 200 (`sys 13`, blend
/// 0) to its end, turning toward the target. The peel leaves `peel_wait` frames after the child was made, only while 200
/// still plays.
async fn toss(cx: &Cx) {
    wait_grounded(cx).await;
    push(cx, ECmd::Note("peel toss effect (sys 110 9 / 0x1039): M13"));
    request(cx, req::TOSS, k(cx, "toss_blend"));
    let mut t = 0.0;
    let mut tossed = false;
    loop {
        if !tossed && t - k(cx, "peel_wait") >= 0.0 {
            tossed = true;
            if playing(cx) == Some(req::TOSS) {
                push(cx, ECmd::Throw(child::PEEL));
            }
        }
        if motion_done(cx) || playing(cx) == Some(req::AIR) {
            break;
        }
        face(cx);
        t += step(cx);
        frame().await;
    }
}

/// `0x736` → `0x7BE`: the taunt 213 (blend 8) to its end, cut once `taunt_min` frames have passed with Sora within
/// `taunt_range`; then on the ground, 217 (blend 8) to its end, the idle (blend 8), and its escape.
async fn taunt(cx: &Cx) {
    request(cx, req::TAUNT, k(cx, "taunt_blend"));
    let mut t = k(cx, "taunt_min");
    loop {
        if motion_done(cx) {
            break;
        }
        face(cx);
        if dist_to_target(cx) - k(cx, "taunt_range") < 0.0 && t < 0.0 {
            break;
        }
        t -= step(cx);
        frame().await;
    }
    wait_grounded(cx).await;
    request(cx, req::TAUNT_END, k(cx, "taunt_end_blend"));
    until_done(cx).await;
    request(cx, req::IDLE, k(cx, "taunt_end_blend"));
    escape(cx).await;
}

/// `0x600`: the call: turning toward the target, 206 (an action, blend 8); the room's helpers (`sys 37(0xB0000 | 2)`)
/// would come to it; none in Re:CoM's battles (Assumption, as the Defender's partner): 207 (an action, blend 12).
async fn call(cx: &Cx) {
    face(cx);
    action(cx, req::CALL, k(cx, "call_blend")).await;
    push(cx, ECmd::Note("call: no helper (sys 37 0xB0000 | 2) in Re:CoM's battles"));
    action(cx, req::CALL_END, k(cx, "call_end_blend")).await;
}

/// `0x59E`: the look-about 215 (blend 8) to its end, turning toward the target; Sora within `ready_close`: the leap and
/// the run-away; then its escape.
async fn ready(cx: &Cx) {
    request(cx, req::READY, k(cx, "ready_blend"));
    loop {
        if motion_done(cx) {
            break;
        }
        face(cx);
        if dist_to_target(cx) - k(cx, "ready_close") < 0.0 {
            leap(cx).await;
            run_away(cx).await;
            break;
        }
        frame().await;
    }
    escape(cx).await;
}

/// `0x89A`: the first look (a placed Bouncywild only): on the ground, 209 (blend 8) to its end, the idle (blend 8), on
/// the ground.
async fn notice(cx: &Cx) {
    wait_grounded(cx).await;
    request(cx, req::NOTICE, k(cx, "notice_blend"));
    until_done(cx).await;
    request(cx, req::IDLE, k(cx, "notice_blend"));
    wait_grounded(cx).await;
}

/// `0x850`: after a near hit: facing Sora at once, 205 (blend 8) to its end, the idle (blend 8), on the ground.
async fn turn(cx: &Cx) {
    face_now(cx);
    request(cx, req::TURN, k(cx, "turn_blend"));
    until_done(cx).await;
    request(cx, req::IDLE, k(cx, "turn_blend"));
    wait_grounded(cx).await;
}

/// The appearance (`0x560` → `0x1A2A`, spawned by the room): facing Sora at once; 69 (`sys 13`, blend 0); the 30-frame
/// fade-in; flag 0x80 for 30 frames (not ported); effect 2 (M13); 30 frames; flag 4 with the idle queued: the script
/// waits until 69 ends. Then the idle state.
async fn appear(cx: Cx) {
    if cx.borrow().x.spawned {
        face_sora_now(&cx);
        request(&cx, req::APPEAR, k(&cx, "appear_blend"));
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        wait_frames(&cx, k(&cx, "appear_wait")).await;
        push(&cx, ECmd::Queue { request: req::IDLE, blend: k(&cx, "appear_then_blend") });
        cx.borrow_mut().x.paused = true;
        frame().await;
        while cx.borrow().w.motion == Some(req::APPEAR) {
            frame().await;
        }
        cx.borrow_mut().x.paused = false;
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The idle behaviour (`0x4F2` → `0x50A`): the target cleared; forever the wander about the current place, the idle
/// (`sys 12`, blend 12) for [`rest_min`, `rest_max`) frames, the stretch 216.
async fn idle(cx: Cx) {
    loop {
        let c = |n: &str| k(&cx, n);
        let (inner, outer, band, timeout) = (c("wander_inner"), c("wander_outer"), c("wander_band"), c("wander_timeout"));
        let (chance, lo, hi, home, blend) = (c("wander_idle_chance"), c("wander_idle_min"), c("wander_idle_max"), c("wander_home_max"), c("wander_idle_blend"));
        wander(&cx, inner, outer, band, timeout, chance, lo, hi, home, blend).await;
        play(&cx, req::IDLE, c("rest_blend"));
        let n = rand_range(&cx, c("rest_min"), c("rest_max"), "rest time");
        wait_frames(&cx, n).await;
        action(&cx, req::STRETCH, c("stretch_blend")).await;
        frame().await;
    }
}

/// The engaged behaviour (`0x39C` → `0x3C0`): the first time (`wp+0x3C`), the first look for a placed one, then the
/// bounce-about, the leap and the run-away; then rounds while fewer than `rounds` (`wp+0x44`) were played: close the
/// leap and the run-away, mid 70 % the peel toss else the taunt, far 80 % the banana else the call and the run-away;
/// then the look-about (and its escape); engaged again.
async fn engaged(cx: Cx) {
    if !cx.borrow().x.engaged_once {
        cx.borrow_mut().x.engaged_once = true;
        if !cx.borrow().x.spawned {
            notice(&cx).await;
        }
        dance(&cx).await;
        leap(&cx).await;
        run_away(&cx).await;
    }
    while cx.borrow().x.rounds - k(&cx, "rounds") < 0.0 {
        cx.borrow_mut().x.rounds += 1.0;
        let d = dist_to_target(&cx);
        if d - k(&cx, "close_range") < 0.0 {
            leap(&cx).await;
            run_away(&cx).await;
        } else if d - k(&cx, "mid_range") < 0.0 {
            if rand(&cx, "mid") - k(&cx, "mid_peel_chance") < 0.0 {
                toss(&cx).await;
            } else {
                taunt(&cx).await;
            }
        } else if rand(&cx, "far") - k(&cx, "far_banana_chance") < 0.0 {
            banana(&cx).await;
        } else {
            call(&cx).await;
            run_away(&cx).await;
        }
        frame().await;
    }
    ready(&cx).await;
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
}

/// The hit behaviour (`0x300` → `0x324`): after its own launch the recovery 212 and the look-about; else within
/// `hit_range` the turn and the run-away, farther the look-about. The leash `0x8D4` is not ported. Then engaged.
async fn hit(cx: Cx) {
    let after = cx.borrow().x.after;
    if after == After::Launched {
        action(&cx, req::RECOVER, k(&cx, "recover_blend")).await;
        ready(&cx).await;
    } else if dist_to_target(&cx) - k(&cx, "hit_range") < 0.0 {
        turn(&cx).await;
        run_away(&cx).await;
    } else {
        ready(&cx).await;
    }
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
}

/// The supervisor (`0x1DD8`): after the init frame, each frame (nothing while the appearance's flag 4 holds the
/// script): while idle, Sora within `idle_search` → the target and engaged; a state request restarts the behaviour
/// thread.
async fn supervisor(cx: Cx) {
    frame().await;
    loop {
        while cx.borrow().x.paused {
            frame().await;
        }
        if cx.borrow().state.0 == st::IDLE {
            let found = {
                let io = cx.borrow();
                io.w.target.is_some_and(|t| (t - io.w.pos).length() - io.c.num("idle_search") < 0.0)
            };
            if found {
                set_target(&cx, true);
                set_state(&cx, st::ENGAGED);
            }
        }
        if let Some(s) = take_state(&cx) {
            // `0x4F2`: the idle clears the target.
            set_target(&cx, s != st::IDLE);
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Bouncywild's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as the Shadow's: per-enemy seeds). `frames`: KH1 motion lengths
    /// by request. `spawned`: the room spawned it (its appearance plays; every Re:CoM battle appearance, owner
    /// 2026-11-02).
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, spawned: bool) -> Ai {
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp { spawned, ..Default::default() });
        Ai { io: Rc::new(RefCell::new(io)), main: None, behaviour: None, started: false }
    }

    pub fn consts(&self) -> Consts {
        self.io.borrow().c.clone()
    }

    /// How many times KH1's escape was left out (the log, the tests).
    pub fn escapes(&self) -> u32 {
        self.io.borrow().x.escapes
    }

    /// One AI frame (the host calls it only while no action runs). Returns the commands.
    pub fn frame(&mut self, w: World) -> Vec<Cmd> {
        {
            let mut io = self.io.borrow_mut();
            io.w = w;
            tick_clock(&mut io);
        }
        if !self.started {
            self.started = true;
            {
                let mut io = self.io.borrow_mut();
                // `0x1EBC`: home = the position, the state object cleared.
                io.home = w.pos;
                io.state = (st::NONE, false);
            }
            self.main = Some(Box::pin(supervisor(self.io.clone())));
            self.behaviour = Some(Box::pin(async {}));
        }
        poll(&mut self.main);
        if self.behaviour.is_none() && !self.io.borrow().x.appeared && self.io.borrow().state.0 == st::NONE {
            self.behaviour = Some(Box::pin(appear(self.io.clone())));
        }
        let restart = self.io.borrow().out.iter().rev().find_map(|c| match c {
            ECmd::State(s) => Some(*s),
            _ => None,
        });
        if let Some(s) = restart {
            let cx = self.io.clone();
            self.behaviour = Some(match s {
                st::IDLE => Box::pin(idle(cx)),
                st::ENGAGED => Box::pin(engaged(cx)),
                _ => Box::pin(hit(cx)),
            });
        }
        poll(&mut self.behaviour);
        std::mem::take(&mut self.io.borrow_mut().out)
    }
}

impl Behaviour for Ai {
    fn frame(&mut self, w: World) -> Vec<ECmd<u8>> {
        Ai::frame(self, w).iter().map(|c| c.map(Attack::index)).collect()
    }
    fn body(&self) -> std::cell::RefMut<'_, Body> {
        std::cell::RefMut::map(self.io.borrow_mut(), |io| &mut io.body)
    }
    fn take_steer(&self) -> Option<Vec3> {
        self.io.borrow_mut().steer.take()
    }
    fn take_draws(&self) -> Vec<Draw> {
        std::mem::take(&mut self.io.borrow_mut().draws)
    }
    fn rng(&self) -> Kh1Rand {
        self.io.borrow().rng.clone()
    }
    fn set_rng(&self, r: Kh1Rand) {
        self.io.borrow_mut().rng = r;
    }
    /// The hit handler (`0x1FAC`): the first round counts as played (`wp+0x3C` = 1); a clash asks for the hit state
    /// (`wp+0x38` = 2); a living damaging hit of kind 4 / 5 plays its own launch (`sys 38(actor, hit, 0, 350, 35, 0)`:
    /// base 0 = KH1's launch 0x52 on the ground / 0x5C in the air) and asks for the recovery (4); any other living
    /// damaging hit asks for the hit state (2). A repel record without a clash does nothing (`0x118E`).
    fn on_hit(&self, news: HitNews) {
        {
            let mut io = self.io.borrow_mut();
            io.x.engaged_once = true;
            io.x.launch = None;
            io.x.after = After::Counter;
            if news == HitNews::Repelled {
                return;
            }
            let kind = io.x.hit_kind;
            if news == HitNews::Damage && (kind == 4 || kind == 5) {
                io.x.launch = Some(ScriptedLaunch { ground: req::LAUNCH, air: req::AIR_LAUNCH, hop: io.c.num("own_launch_hop"), speed: io.c.num("own_launch_speed"), down: io.c.num("own_launch_down") });
                io.x.after = After::Launched;
            }
        }
        set_state(&self.io, st::HIT);
    }
    fn risen(&self) -> bool {
        self.io.borrow().x.appeared
    }
    fn skip_rise(&self) {
        self.io.borrow_mut().x.spawned = false;
    }
    fn set_hit_kind(&self, kind: u8) {
        self.io.borrow_mut().x.hit_kind = kind;
    }
    fn scripted_launch(&self) -> Option<ScriptedLaunch> {
        self.io.borrow_mut().x.launch.take()
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["none", "idle", "engaged", "hit"].get(s as usize).copied().unwrap_or("?")
    }
    fn reset(&self) {
        self.io.borrow_mut().x.paused = false;
        set_state(&self.io, st::ENGAGED);
    }
}

/// One peel (`ex_2040_01.bd` / `_02.bd`'s `0x18` → `0x96`), KH1 units (y down): it falls (its speed's y +
/// `peel_gravity` × step a frame) moving by speed × step through a radius-`peel_sweep` sweep (`sys 140`) until that
/// sphere touches the floor; then it lies for `peel_lie` frames. The host makes it one of Re:CoM's bullets, its attack
/// on only while it lies, and ends it when Sora slips on it (Re:CoM's own hit ends it). Checked (files); the floor is
/// the battle floor (flat: Assumption, as every M11 port), the sweep's contact the sphere's bottom on it (Assumption:
/// `sys 140` keeps the sphere's centre where it touches).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Peel {
    pub pos: Vec3,
    pub vel: Vec3,
    /// Frames lying; None while it falls.
    pub lying: Option<f32>,
    pub ended: bool,
}

/// The numbers a peel reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PeelNumbers {
    pub sweep: f32,
    pub gravity: f32,
    pub lie: f32,
}

impl Peel {
    pub fn new(at: Vec3, vel: Vec3) -> Peel {
        Peel { pos: at, vel, lying: None, ended: false }
    }

    /// The tossed peel's speed (`ex_2040_01.bd` `0x502`): along the thrower's facing `yaw` turned by `angle` degrees,
    /// × `speed`, with `up` upward (KH1's y down: −up).
    pub fn tossed(yaw: f32, angle: f32, speed: f32, up: f32) -> Vec3 {
        let d = Vec3::from_yaw(yaw + angle.to_radians()) * speed;
        Vec3::new(d.x, -up, d.z)
    }

    /// The dropped peel's speed (`ex_2040_02.bd` `0x502`): along the way from `from` to `to` (normalised, its height
    /// included) × `speed`, then its y set to −`up`.
    pub fn dropped(from: Vec3, to: Vec3, speed: f32, up: f32) -> Vec3 {
        let d = (to - from).normalized() * speed;
        Vec3::new(d.x, -up, d.z)
    }

    /// One frame of `step` KH1 frames over a floor at `floor` (KH1 y): the place after it. Lying, it counts its frames
    /// and ends past `lie`.
    pub fn step(&mut self, n: &PeelNumbers, floor: f32, step: f32) -> Vec3 {
        if self.ended {
            return self.pos;
        }
        match self.lying {
            Some(t) => {
                let t = t + step;
                self.lying = Some(t);
                if t - n.lie >= 0.0 {
                    self.ended = true;
                }
            }
            None => {
                let rest = floor - n.sweep;
                let next = self.pos + self.vel * step;
                if next.y - rest >= 0.0 {
                    // The sweep stops where its sphere touches the floor (`0xBE` `sys 140` → contact, `0x108`).
                    let k = if next.y != self.pos.y { ((rest - self.pos.y) / (next.y - self.pos.y)).clamp(0.0, 1.0) } else { 1.0 };
                    self.pos = self.pos + (next - self.pos) * k;
                    self.pos.y = rest;
                    self.lying = Some(0.0);
                } else {
                    self.pos = next;
                    self.vel.y += n.gravity * step;
                }
            }
        }
        self.pos
    }
}

#[cfg(test)]
mod tests;
