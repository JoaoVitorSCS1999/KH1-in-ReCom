//! KH1 FM's Air Soldier (`xa_ex_2160.mdls` part 5, `ex_2160.bd`), engine-agnostic: its AI script on
//! the shared enemy framework (`crate::enemy`), flying (KH1's movement state 4,
//! `crate::enemy::Body::flying`). M11 (`docs/recom-m11-enemies.md` § The Air Soldier). Re:CoM runs
//! it natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`).
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2160.bd` at checked
//! offsets). Script offsets are the player's `ex_2160.bd`'s (Checked (files): the script
//! disassembled with `tools/kh1-ai-script.py`).
//!
//! # The script's shape
//!
//! - entry `0x3670`: init `0x38C0` (stats record LV1 `0x14` / LV2 `0x80`, turn rate 7.5°, walk 8,
//!   the head-look limits, the handlers), flight with no velocity (`sys 135`), a yield, the party
//!   and group handles, the appearance thread `0x1952` → `0x1966`; then each frame: a state request
//!   restarts the behaviour thread (1 idle `0x1410` → `0x1442`, 2 engaged `0xFA0` → `0x1022`, 3 lost
//!   `0xD3A` → `0xD84`, 4 hit `0x5C0` → `0x5F2`); while idle, the target search (Sora within 1500
//!   → engaged).
//! - the hit handler `0x3A90`: the clash (kinds 6 / 7) stops the flight; a damaging hit adds to the
//!   anger meter `wp+0x54` (kind 5 +25 and taken as kind 4, kind 4 +10, kind 3 +5); both ask for the
//!   hit state 4.
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash (`0x3E52`, `0x2A7C`: KH1 removes it
//! far from Sora / out of sight), the group leader's messages (`sys 81` to `party(0xB0000 | 160)`:
//! none in Re:CoM), the world-8 room rule (`0x15C4`), the spawn type 23 entry, the head look
//! (`sys 121`), the death routine (Re:CoM's defeat).

use crate::enemy::script::{face_target, fly, frame, height, motion_done, poll, pos, push, rand, rand_range, random_offset, set_state, set_target, set_turn_rate, steer, take_state, target, tick_clock, wait_frames};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Air Soldier's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The flight (request 1, the shared walk number; looped).
    pub const FLY: u16 = 1;
    /// The appearance (`0x3282`: 69, then an action until it ends), 66 frames.
    pub const APPEAR: u16 = 69;
    /// The swoop kick (`0x2802`), 64 frames, root motion, hit [28, 50).
    pub const KICK: u16 = 200;
    /// The slash (`0x2286`: from frame 20), 78 frames, root motion, hit [22, 52).
    pub const SLASH: u16 = 201;
    /// The swoop (`0x246E`), 88 frames, hit [32, 42).
    pub const SWOOP: u16 = 202;
    /// The spin (`0x1AD4`), 80 frames, hit [24, 32).
    pub const SPIN: u16 = 203;
    /// The charge (`0x1ECE`): the wind-up 210, the loop 211 (hit all along), the end 212 (it hit)
    /// or 213 (it missed).
    pub const CHARGE_UP: u16 = 210;
    pub const CHARGE: u16 = 211;
    pub const CHARGE_HIT: u16 = 212;
    pub const CHARGE_MISS: u16 = 213;
    /// The back-off flight (`0x41F8`).
    pub const BACK: u16 = 220;
    /// The strafes (`0x4346`): left, right.
    pub const STRAFE_L: u16 = 221;
    pub const STRAFE_R: u16 = 222;
    /// The power-up (`0x9C2`): 225, the approach 226, the end 227.
    pub const POWER: u16 = 225;
    pub const POWER_FLY: u16 = 226;
    pub const POWER_END: u16 = 227;
    /// The climb at a target far above (`0x4CF2`): 230, 231 (loop), 232.
    pub const CLIMB: u16 = 230;
    pub const CLIMB_LOOP: u16 = 231;
    pub const CLIMB_END: u16 = 232;
    /// The tuck (`0x1AD4`'s start, `0x4960`'s dive): 235, 236 (loop), 237.
    pub const TUCK: u16 = 235;
    pub const TUCK_LOOP: u16 = 236;
    pub const TUCK_END: u16 = 237;
}

/// The Air Soldier's attacks, in the order the host indexes them (the manifest's `attacks`): each
/// a plain request made with attack rows (`crate::enemy::Cmd::Attack`: the script keeps steering).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    Kick,
    Slash,
    Swoop,
    Spin,
    Charge,
}

impl Attack {
    pub const ALL: [Attack; 5] = [Attack::Kick, Attack::Slash, Attack::Swoop, Attack::Spin, Attack::Charge];

    pub fn request(self) -> u16 {
        match self {
            Attack::Kick => req::KICK,
            Attack::Slash => req::SLASH,
            Attack::Swoop => req::SWOOP,
            Attack::Spin => req::SPIN,
            Attack::Charge => req::CHARGE,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Kick => "swoop kick",
            Attack::Slash => "slash",
            Attack::Swoop => "swoop",
            Attack::Spin => "spin",
            Attack::Charge => "charge",
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The names of the Air Soldier's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    "turn_deg",
    "idle_search",
    // Wander `0x15FA`.
    "wander_min",
    "wander_max",
    "wander_probe",
    "pit_depth",
    "pit_lift",
    "wander_high",
    "wander_near_lo",
    "wander_near_hi",
    "wander_far_lo",
    "wander_far_hi",
    "fly_up",
    "fly_speed",
    "fly_arrive",
    "fly_time_min",
    "fly_time_max",
    "fly_low",
    "rest_min",
    "rest_max",
    "idle_blend",
    // Engaged `0x1022`.
    "climb_dy",
    "dive_dy",
    "close_range",
    "mid_range",
    "mid_kick_chance",
    "mid_swoop_chance",
    "close_approach_chance",
    "hover_min",
    "hover_max",
    "hover_short_min",
    "hover_short_max",
    "hover_near",
    "back_kick_min",
    "back_kick_max",
    "back_up",
    "back_speed",
    "back_time",
    "back_hover_min",
    "back_hover_max",
    // Approach `0x4664`.
    "approach_speed",
    "approach_arrive",
    "approach_time",
    "approach_accel",
    "approach_up",
    // Strafe `0x4346`.
    "strafe_speed",
    "strafe_turn",
    "strafe_turn_factor",
    "strafe_up",
    "strafe_above_min",
    "strafe_above_max",
    "strafe_down",
    "strafe_rise",
    "strafe_left",
    "strafe_right",
    "strafe_side_chance",
    "strafe_close_min",
    "strafe_close_max",
    "strafe_mid_min",
    "strafe_mid_max",
    "strafe_hit_min",
    "strafe_hit_max",
    // Attacks.
    "kick_near",
    "kick_up",
    "kick_speed",
    "slash_near",
    "slash_up",
    "slash_speed",
    "swoop_up",
    "swoop_aim_up",
    "swoop_rise_speed",
    "swoop_rise_time",
    "swoop_wait_a",
    "swoop_wait_b",
    "swoop_low",
    "swoop_down_speed",
    "swoop_down_time",
    "spin_off_min",
    "spin_off_max",
    "spin_off_up",
    "spin_tuck_speed",
    "spin_time",
    "spin_up",
    "spin_speed",
    "spin_near",
    "spin_close_speed",
    "charge_wait",
    "charge_speed",
    "charge_arrive",
    "charge_time_min",
    "charge_time_max",
    "charge_miss_add",
    // Hit `0x5F2`.
    "retarget_chance",
    "hit_close",
    "hit_level",
    "hit_swoop_dy",
    "hit_swoop_height",
    "hit_back_min",
    "hit_back_max",
    "anger_full",
    "anger_power_chance",
    "anger_hp_div",
    "power_speed",
    "power_arrive",
    "power_time",
    "power_reach",
    "power_tail",
    "power_close",
    "power_level",
    "anger_launch",
    "anger_knock",
    "anger_flinch",
    // Climb / dive `0x4CF2` / `0x4960`.
    "climb_over",
    "dive_over",
    "vert_time",
    "vert_fast_time",
    "vert_fast_speed",
    "vert_slow_time",
    "vert_slow_speed",
    "vert_speed",
    "dive_stop_fast",
    "dive_stop_slow",
    "vert_tail",
    "vert_wait",
    // Shared.
    "attack_blend",
    "motion_tail",
    "steer_cap",
    "appear_fade",
    "appear_wait",
];

/// The Air Soldier's numbers.
pub type Consts = Table;

/// The state objects (`wp+0x30`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const LOST: u8 = 3;
    pub const HIT: u8 = 4;
}

/// The Air Soldier's own work area: home (`wp+0x20`), the last target (`wp+0x4`, here: Sora or
/// none), the anger meter (`wp+0x54`), the misses in a row (`wp+0x5C`), the attack-landed flags of
/// its attacks (`wp+0x60` kick, `+0x64` slash, `+0x68` swoop, `+0x6C` spin, `+0x70` charge), the
/// appearance done, Sora's target is it, the last hit's kind, Re:CoM's HP ratio (`0x998`'s HP <
/// max / 4 test: Re:CoM's HP, decision M10).
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    home: Vec3,
    last: bool,
    anger: i32,
    misses: i32,
    landed: [bool; 5],
    spawned: bool,
    appeared: bool,
    targeted: bool,
    kind: u8,
    hp_ratio: f32,
}

type Cx = crate::enemy::script::Cx<Consts, Attack, Wp>;
type Task = crate::enemy::script::Task;
pub type Cmd = ECmd<Attack>;

fn k(cx: &Cx, n: &str) -> f32 {
    cx.borrow().c.num(n)
}

fn me(cx: &Cx) -> Vec3 {
    pos(cx)
}

/// `sys 9`: the vector normalized (its length 0 stays 0).
fn norm(v: Vec3) -> Vec3 {
    if v.length() > 0.0 {
        v.normalized()
    } else {
        v
    }
}

/// `*(actor + 0x20) = v.4w`: this frame's steering (a unit direction and its speed, capped at
/// `steer_cap` by the routines that accelerate).
fn go(cx: &Cx, dir: Vec3, speed: f32) {
    steer(cx, dir, speed);
}

/// `*(actor + 0x2C) = 0`.
fn stop(cx: &Cx) {
    steer(cx, Vec3::ZERO, 0.0);
}

/// `sys 12`: a motion now unless it plays.
fn play(cx: &Cx, r: u16, blend: f32) {
    start(cx, ECmd::Motion { request: r, blend });
}

/// A motion request: the command, and `sys 172`'s clock restarts for a request that plays anew (KH1
/// holds it in the queue `+0x178` until the motion update: not done until it has played).
fn start(cx: &Cx, c: Cmd) {
    let (r, anew) = match c {
        ECmd::Motion { request, .. } => (request, true),
        ECmd::Attack { request, .. } => (request, false),
        _ => {
            push(cx, c);
            return;
        }
    };
    {
        let mut io = cx.borrow_mut();
        if !anew || io.w.motion != Some(r) {
            io.clock = (Some(r), 0.0);
        }
    }
    push(cx, c);
}

/// `0x3382`: face the target (`sys 36` toward it).
fn face(cx: &Cx) {
    face_target(cx);
}

/// `0x3612`: the flat distance to the target (its position at the actor's height).
fn flat_dist(cx: &Cx) -> f32 {
    match target(cx) {
        Some(t) => (Vec3::new(t.x, me(cx).y, t.z) - me(cx)).length(),
        None => 1.0e8,
    }
}

/// `0x30D4`: the distance to the target.
fn dist(cx: &Cx) -> f32 {
    target(cx).map_or(1.0e8, |t| (t - me(cx)).length())
}

/// `0x3364`: wait until the motion is done (`sys 172`), a yield first.
async fn wait_done(cx: &Cx) {
    loop {
        frame().await;
        if motion_done(cx) {
            break;
        }
    }
}

/// `0x2DF0`: until the motion is done or the air motion 6 plays, facing the target.
async fn wait_done_or_air(cx: &Cx) {
    while !motion_done(cx) {
        if cx.borrow().w.motion == Some(req::AIR) {
            break;
        }
        face(cx);
        frame().await;
    }
}

/// `0x50C0`: wait `n` frames.
async fn wait(cx: &Cx, n: f32) {
    wait_frames(cx, n).await;
}

/// `0x5098`: the idle motion, then wait `n` + 8.
async fn rest(cx: &Cx, n: f32) {
    play(cx, req::IDLE, k(cx, "idle_blend"));
    wait(cx, n + k(cx, "motion_tail")).await;
}

/// `0x4FFA`: hover in the idle motion `n` + 8 frames, or until the target is within `hover_near`.
async fn hover(cx: &Cx, n: f32) {
    play(cx, req::IDLE, k(cx, "idle_blend"));
    let mut t = n + k(cx, "motion_tail");
    while t > 0.0 {
        if dist(cx) - k(cx, "hover_near") < 0.0 {
            stop(cx);
            break;
        }
        t -= cx.borrow().w.step;
        frame().await;
    }
}

/// The vertical part of the flights toward a point (`0x15FA`, `0x1ECE`, `0x4664`): `d` the vector to
/// the point; below the arrival distance only vertical (`None`: arrived); a low body rises
/// (`fly_low`); a point below goes level when the body is under it or low.
fn vertical(cx: &Cx, mut d: Vec3, arrive: f32, speed: f32, goal_y: f32) -> Option<Vec3> {
    let step = cx.borrow().w.step;
    if d.length() - arrive < 0.0 {
        if d.y.abs() - speed * step < 0.0 {
            return None;
        }
        d.x = 0.0;
        d.z = 0.0;
    } else if height(cx) - k(cx, "fly_low") <= 0.0 {
        d.y = -1.0;
    } else if d.y > 0.0 {
        if me(cx).y - goal_y > 0.0 {
            d.y = 0.0;
        } else if height(cx) - k(cx, "fly_low") < 0.0 {
            d.y = 0.0;
            d.x *= speed;
            d.z *= speed;
        }
    }
    Some(d)
}

/// `0x15FA(min, max)`: the wander flight: a point [min, max) from home in a random direction, its
/// height from the floor there (or about the current height when the floor is below and the body
/// isn't too high), then the flight to it (KH1's flight request 1) until arrived or the time is out;
/// then a rest.
async fn wander(cx: &Cx) {
    let off = random_offset(cx, k(cx, "wander_min"), k(cx, "wander_max"));
    let probe = k(cx, "wander_probe");
    let floor = cx.borrow().floor;
    // `0x2ECE`: the floor under a point (KH1's line test down 1e6; none: the point + 1e6).
    let floor_at = |p: Vec3| floor.unwrap_or(p.y + 1.0e6);
    let mut home = cx.borrow().x.home;
    if floor_at(home - Vec3::new(0.0, probe, 0.0)) - k(cx, "pit_depth") > 0.0 {
        home.y += k(cx, "pit_lift");
        cx.borrow_mut().x.home = home;
    }
    let mut goal = off + home;
    goal.y = floor_at(goal - Vec3::new(0.0, probe, 0.0));
    if goal.y - me(cx).y > 0.0 && height(cx) - k(cx, "wander_high") < 0.0 {
        goal.y = me(cx).y - rand_range(cx, k(cx, "wander_near_lo"), k(cx, "wander_near_hi"), "wander height");
    } else {
        goal.y -= rand_range(cx, k(cx, "wander_far_lo"), k(cx, "wander_far_hi"), "wander height");
    }
    let speed = k(cx, "fly_speed");
    let arrive = k(cx, "fly_arrive");
    let mut t = rand_range(cx, k(cx, "fly_time_min"), k(cx, "fly_time_max"), "wander time") + k(cx, "motion_tail");
    while t > 0.0 {
        let to = goal - Vec3::new(0.0, k(cx, "fly_up"), 0.0) - me(cx);
        play(cx, req::FLY, k(cx, "idle_blend"));
        cx.borrow_mut().body.yaw_target = to.yaw();
        let step = cx.borrow().w.step;
        let Some(d) = vertical(cx, to, arrive, speed, goal.y) else {
            stop(cx);
            break;
        };
        go(cx, norm(d), speed.min(k(cx, "steer_cap")));
        t -= step;
        frame().await;
    }
    let n = rand_range(cx, k(cx, "rest_min"), k(cx, "rest_max"), "wander rest");
    rest(cx, n).await;
}

/// `0x4346(n)`: the strafe: sideways (90° left or right of the facing, at random; 221 / 222), the
/// direction turning by `strafe_turn × factor` degrees a frame toward the other side, facing the
/// target, keeping above it by [above_min, above_max) or rising to it; `n` + 8 frames.
async fn strafe(cx: &Cx, n: f32) {
    let yaw = cx.borrow().body.yaw;
    let mut turn = k(cx, "strafe_turn");
    // `sys 10` turns the facing by −90° / +90° (`0x4396` / `0x43B0`).
    let (mut dir, anim) = if rand(cx, "strafe side") - k(cx, "strafe_side_chance") < 0.0 {
        (yaw + k(cx, "strafe_left").to_radians(), req::STRAFE_L)
    } else {
        turn = -turn;
        (yaw + k(cx, "strafe_right").to_radians(), req::STRAFE_R)
    };
    let up = k(cx, "strafe_up");
    // `sp+0x50`: the target's height at the start, `strafe_up` above it.
    let level = target(cx).map(|t| t.y - up);
    let above = level.is_some_and(|l| me(cx).y - l <= 0.0);
    face(cx);
    let speed = k(cx, "strafe_speed").min(k(cx, "steer_cap"));
    let mut t = n + k(cx, "motion_tail");
    while t > 0.0 {
        let step = cx.borrow().w.step;
        play(cx, anim, k(cx, "idle_blend"));
        dir += (turn * k(cx, "strafe_turn_factor") * step).to_radians();
        face(cx);
        let mut d = Vec3::from_yaw(dir);
        if let Some(tg) = target(cx) {
            if above {
                let lim = tg.y - rand_range(cx, k(cx, "strafe_above_min"), k(cx, "strafe_above_max"), "strafe height");
                if me(cx).y - lim <= 0.0 {
                    d.y = k(cx, "strafe_down");
                    d = norm(d);
                }
            } else if me(cx).y - level.unwrap_or(tg.y - up) >= 0.0 {
                d.y = k(cx, "strafe_rise");
                d = norm(d);
            } else {
                d.y = 0.0;
                d = norm(d);
            }
        }
        go(cx, d, speed);
        t -= step;
        frame().await;
    }
}

/// `0x41F8(min, max)`: back off to a point [min, max) from the target in a random direction,
/// `back_up` above it, at `back_speed` for `back_time` frames (request 220), facing the target;
/// then a hover.
async fn back_off(cx: &Cx, lo: f32, hi: f32) {
    let off = random_offset(cx, lo, hi);
    let Some(tg) = target(cx) else { return };
    let goal = off + tg - Vec3::new(0.0, k(cx, "back_up"), 0.0);
    play(cx, req::BACK, k(cx, "idle_blend"));
    let speed = k(cx, "back_speed").min(k(cx, "steer_cap"));
    let mut t = k(cx, "back_time");
    while t > 0.0 {
        go(cx, norm(goal - me(cx)), speed);
        face(cx);
        t -= cx.borrow().w.step;
        frame().await;
    }
    face(cx);
    if target(cx).is_some() {
        let n = rand_range(cx, k(cx, "back_hover_min"), k(cx, "back_hover_max"), "back hover");
        hover(cx, n).await;
    }
}

/// `0x4664(anim, goal, speed, arrive, n)`: fly at the target (`approach_up` above it) in `anim`,
/// accelerating by `approach_accel` a frame up to `steer_cap`, until within `arrive` (flat) or the
/// time is out.
async fn approach(cx: &Cx, anim: u16, speed0: f32, arrive: f32, n: f32) {
    let mut speed = speed0;
    let mut t = n + k(cx, "motion_tail");
    while t > 0.0 {
        play(cx, anim, k(cx, "idle_blend"));
        let Some(tg) = target(cx) else { break };
        let to = tg - Vec3::new(0.0, k(cx, "approach_up"), 0.0) - me(cx);
        cx.borrow_mut().body.yaw_target = to.yaw();
        let step = cx.borrow().w.step;
        if to.flat().length() - arrive < 0.0 {
            // Arrived: only vertical, with this frame's speed (0: the actor task zeroed it).
            stop(cx);
            break;
        }
        let mut d = to;
        // A low body (`fly_low`) goes on as aimed (`0x482E`: its own write is overwritten below).
        if height(cx) - k(cx, "fly_low") > 0.0 && d.y > 0.0 {
            if me(cx).y - tg.y > 0.0 {
                d.y = 0.0;
            } else if height(cx) - k(cx, "fly_low") < 0.0 {
                d.y = 0.0;
            }
        }
        speed += k(cx, "approach_accel");
        speed = speed.min(k(cx, "steer_cap"));
        go(cx, norm(d), speed);
        t -= step;
        frame().await;
    }
}

/// `0x2802`: the swoop kick (200, root motion): flying at the target (`kick_up` above it) at
/// `kick_speed` until its motion is done or within `kick_near`; then the idle. A miss counts.
async fn kick(cx: &Cx) {
    cx.borrow_mut().x.landed[Attack::Kick as usize] = false;
    start(cx, ECmd::Attack { request: req::KICK, attack: Attack::Kick, blend: k(cx, "attack_blend") });
    while !motion_done(cx) {
        if dist(cx) - k(cx, "kick_near") < 0.0 {
            break;
        }
        if let Some(tg) = target(cx) {
            go(cx, norm(tg - Vec3::new(0.0, k(cx, "kick_up"), 0.0) - me(cx)), k(cx, "kick_speed"));
        }
        face(cx);
        frame().await;
    }
    wait_done_or_air(cx).await;
    after_attack(cx, Attack::Kick);
}

/// `0x2286`: the slash (201 from frame 20, root motion): only vertical tracking (`slash_up` above
/// the target at `slash_speed`) until its motion is done; it stops within `slash_near` or once it
/// hit. A miss counts.
async fn slash(cx: &Cx) {
    cx.borrow_mut().x.landed[Attack::Slash as usize] = false;
    start(cx, ECmd::Attack { request: req::SLASH, attack: Attack::Slash, blend: k(cx, "attack_blend") });
    while !motion_done(cx) {
        if dist(cx) - k(cx, "slash_near") < 0.0 || cx.borrow().x.landed[Attack::Slash as usize] {
            stop(cx);
        } else if let Some(tg) = target(cx) {
            let mut d = norm(tg - Vec3::new(0.0, k(cx, "slash_up"), 0.0) - me(cx));
            d.x = 0.0;
            d.z = 0.0;
            go(cx, d, k(cx, "slash_speed"));
        }
        frame().await;
    }
    after_attack(cx, Attack::Slash);
}

/// `0x246E`: the swoop: rise to `swoop_up` above the target (flight 1, `swoop_rise_time` frames),
/// then 202 with its rows, a coast (two waits), the descent to `swoop_low` above the target's
/// first height; then the idle once its motion is done. A miss counts.
async fn swoop(cx: &Cx) {
    cx.borrow_mut().x.landed[Attack::Swoop as usize] = false;
    let Some(t0) = target(cx) else { return };
    face(cx);
    play(cx, req::FLY, k(cx, "idle_blend"));
    let top = t0.y - k(cx, "swoop_up");
    let mut t = k(cx, "swoop_rise_time");
    while t > 0.0 {
        face(cx);
        if me(cx).y - top > 0.0 {
            face(cx);
            if let Some(tg) = target(cx) {
                go(cx, norm(tg - Vec3::new(0.0, k(cx, "swoop_aim_up"), 0.0) - me(cx)), k(cx, "swoop_rise_speed").min(k(cx, "steer_cap")));
            }
        } else {
            break;
        }
        t -= cx.borrow().w.step;
        frame().await;
    }
    start(cx, ECmd::Attack { request: req::SWOOP, attack: Attack::Swoop, blend: k(cx, "attack_blend") });
    wait(cx, k(cx, "swoop_wait_a")).await;
    wait(cx, k(cx, "swoop_wait_b")).await;
    let low = t0.y - k(cx, "swoop_low");
    let mut t = k(cx, "swoop_down_time");
    while t > 0.0 {
        face(cx);
        if me(cx).y - low < 0.0 {
            go(cx, Vec3::new(0.0, 1.0, 0.0), k(cx, "swoop_down_speed").min(k(cx, "steer_cap")));
        } else {
            break;
        }
        t -= cx.borrow().w.step;
        frame().await;
    }
    wait_done_or_air(cx).await;
    after_attack(cx, Attack::Swoop);
}

/// `0x1AD4`: the spin: the tuck (235) flying to a point [spin_off_min, max) from the target while it
/// plays (`0x1D8E`), then 203 with its rows, dashing at the target (`spin_up` above it at
/// `spin_speed`, then at it at `spin_close_speed`) for `spin_time` frames or until within
/// `spin_near`; the anger meter empties; a miss counts; the idle once its motion is done.
async fn spin(cx: &Cx) {
    cx.borrow_mut().x.landed[Attack::Spin as usize] = false;
    start(cx, ECmd::Motion { request: req::TUCK, blend: k(cx, "attack_blend") });
    // `0x1D8E`.
    if let Some(tg) = target(cx) {
        let goal = random_offset(cx, k(cx, "spin_off_min"), k(cx, "spin_off_max")) + tg - Vec3::new(0.0, k(cx, "spin_off_up"), 0.0);
        while !motion_done(cx) {
            go(cx, norm(goal - me(cx)), k(cx, "spin_tuck_speed").min(k(cx, "steer_cap")));
            face(cx);
            frame().await;
        }
    }
    start(cx, ECmd::Motion { request: req::TUCK_LOOP, blend: 0.0 });
    let mut made = false;
    let mut t = k(cx, "spin_time");
    while t > 0.0 {
        let Some(tg) = target(cx) else { break };
        let to = tg - Vec3::new(0.0, k(cx, "spin_up"), 0.0) - me(cx);
        cx.borrow_mut().body.yaw_target = to.yaw();
        go(cx, norm(to), k(cx, "spin_speed"));
        if !made {
            made = true;
            start(cx, ECmd::Attack { request: req::SPIN, attack: Attack::Spin, blend: 0.0 });
        }
        face(cx);
        if dist(cx) - k(cx, "spin_near") < 0.0 {
            break;
        }
        go(cx, norm(tg - me(cx)), k(cx, "spin_close_speed"));
        t -= cx.borrow().w.step;
        frame().await;
    }
    cx.borrow_mut().x.anger = 0;
    count_miss(cx, Attack::Spin);
    wait_done(cx).await;
    play(cx, req::IDLE, k(cx, "idle_blend"));
}

/// `0x1ECE`: the charge: the wind-up 210 (`charge_wait` frames facing the target), then the loop
/// 211 with its row, flying at the target's first position at `charge_speed` for [time_min, max)
/// frames, until within `charge_arrive` or it hit; then 212 (it hit: the misses reset) or 213 (a
/// miss counts `charge_miss_add`).
async fn charge(cx: &Cx) {
    start(cx, ECmd::Motion { request: req::CHARGE_UP, blend: k(cx, "attack_blend") });
    // `0x2D9C(90)`: facing the target, until request 6 plays.
    let mut w = 0.0;
    while w - k(cx, "charge_wait") < 0.0 {
        if cx.borrow().w.motion == Some(req::AIR) {
            break;
        }
        face(cx);
        w += cx.borrow().w.step;
        frame().await;
    }
    cx.borrow_mut().x.landed[Attack::Charge as usize] = false;
    let Some(goal) = target(cx) else { return };
    let speed = k(cx, "charge_speed");
    let arrive = k(cx, "charge_arrive");
    let mut t = rand_range(cx, k(cx, "charge_time_min"), k(cx, "charge_time_max"), "charge time");
    fly(cx, Vec3::ZERO);
    let mut made = false;
    while t > 0.0 {
        if !made {
            made = true;
            start(cx, ECmd::Attack { request: req::CHARGE, attack: Attack::Charge, blend: k(cx, "attack_blend") });
        } else {
            play(cx, req::CHARGE, k(cx, "attack_blend"));
        }
        let to = goal - me(cx);
        cx.borrow_mut().body.yaw_target = to.yaw();
        let step = cx.borrow().w.step;
        if cx.borrow().x.landed[Attack::Charge as usize] {
            stop(cx);
            break;
        }
        let Some(d) = vertical(cx, to, arrive, speed, goal.y) else {
            stop(cx);
            break;
        };
        go(cx, norm(d), speed);
        t -= step;
        frame().await;
    }
    if cx.borrow().x.landed[Attack::Charge as usize] {
        {
            let mut io = cx.borrow_mut();
            io.x.misses = 0;
            io.x.landed[Attack::Charge as usize] = false;
        }
        stop(cx);
        start(cx, ECmd::Motion { request: req::CHARGE_HIT, blend: 0.0 });
    } else {
        let add = k(cx, "charge_miss_add") as i32;
        cx.borrow_mut().x.misses += add;
        start(cx, ECmd::Motion { request: req::CHARGE_MISS, blend: 0.0 });
    }
    wait_done(cx).await;
    play(cx, req::IDLE, k(cx, "idle_blend"));
}

/// The attack's end (`0x23E4` and the like): the idle; a miss counts, a hit resets the count.
fn after_attack(cx: &Cx, a: Attack) {
    play(cx, req::IDLE, k(cx, "idle_blend"));
    count_miss(cx, a);
}

fn count_miss(cx: &Cx, a: Attack) {
    let mut io = cx.borrow_mut();
    if io.x.landed[a as usize] {
        io.x.misses = 0;
    } else {
        io.x.misses += 1;
    }
}

/// `0x4CF2` (`up` true) / `0x4960`: the climb at a target far above / the dive at one far below:
/// the start motion (230 / 235) until done, then straight up / down at `vert_speed` (231 / 236)
/// until past `climb_over` above / `dive_over` above the target's height; then the end motion
/// (232 / 237) easing off: `vert_fast_time` frames at `vert_fast_speed`, then `vert_slow_time` at
/// `vert_slow_speed` (the dive stops at `dive_stop_fast` / `dive_stop_slow` above the floor and
/// carries the frames left over), then the end motion's tail; `vert_time` frames at most.
async fn vertical_move(cx: &Cx, up: bool) {
    let Some(tg) = target(cx) else { return };
    face(cx);
    let (first, lp, end) = if up { (req::CLIMB, req::CLIMB_LOOP, req::CLIMB_END) } else { (req::TUCK, req::TUCK_LOOP, req::TUCK_END) };
    start(cx, ECmd::Motion { request: first, blend: k(cx, "attack_blend") });
    wait_done(cx).await;
    let s = if up { -1.0 } else { 1.0 };
    let mark = tg.y - if up { k(cx, "climb_over") } else { k(cx, "dive_over") };
    let mut t = k(cx, "vert_time");
    while t > 0.0 {
        let past = if up { me(cx).y - mark < 0.0 } else { me(cx).y - mark > 0.0 };
        if past {
            start(cx, ECmd::Motion { request: end, blend: if up { k(cx, "attack_blend") } else { 0.0 } });
            if up {
                wait(cx, k(cx, "vert_wait")).await;
            }
            let mut u = k(cx, "vert_fast_time");
            while u > 0.0 {
                if !up && height(cx) - k(cx, "dive_stop_fast") <= 0.0 {
                    stop(cx);
                    break;
                }
                face(cx);
                let mut d = cx.borrow().body.walk;
                d.y = s;
                go(cx, norm(d), k(cx, "vert_fast_speed"));
                u -= cx.borrow().w.step;
                frame().await;
            }
            // The dive carries the frames left over; the climb counts afresh.
            u = if up { k(cx, "vert_slow_time") } else { u + k(cx, "vert_slow_time") };
            while u > 0.0 {
                if !up && height(cx) - k(cx, "dive_stop_slow") <= 0.0 {
                    stop(cx);
                    break;
                }
                face(cx);
                let mut d = cx.borrow().body.walk;
                d.y = s;
                go(cx, norm(d), k(cx, "vert_slow_speed"));
                u -= cx.borrow().w.step;
                frame().await;
            }
            if !up {
                wait(cx, u).await;
            }
            let n = crate::enemy::script::motion_frames(cx, end) - 2.0 - k(cx, "vert_tail");
            wait(cx, n).await;
            stop(cx);
            break;
        }
        if !up && height(cx) - k(cx, "dive_stop_slow") <= 0.0 {
            stop(cx);
            break;
        }
        face(cx);
        go(cx, Vec3::new(0.0, s, 0.0), k(cx, "vert_speed").min(k(cx, "steer_cap")));
        play(cx, lp, 0.0);
        t -= cx.borrow().w.step;
        frame().await;
    }
    push(cx, ECmd::Queue { request: req::IDLE, blend: 0.0 });
    frame().await;
}

/// `0x1264`: mid range (300..600): the swoop kick and a back off, a hover / swoop / hover, or a
/// strafe / swoop / hover.
async fn mid(cx: &Cx) {
    let r = rand(cx, "mid attack");
    if r - k(cx, "mid_kick_chance") < 0.0 {
        kick(cx).await;
        back_off(cx, k(cx, "back_kick_min"), k(cx, "back_kick_max")).await;
        face(cx);
        let n = rand_range(cx, k(cx, "hover_min"), k(cx, "hover_max"), "hover");
        hover(cx, n).await;
    } else if r - k(cx, "mid_swoop_chance") < 0.0 {
        face(cx);
        let n = rand_range(cx, k(cx, "hover_min"), k(cx, "hover_max"), "hover");
        hover(cx, n).await;
        swoop(cx).await;
        face(cx);
        let n = rand_range(cx, k(cx, "hover_min"), k(cx, "hover_max"), "hover");
        hover(cx, n).await;
    } else {
        let n = rand_range(cx, k(cx, "strafe_mid_min"), k(cx, "strafe_mid_max"), "strafe");
        strafe(cx, n).await;
        swoop(cx).await;
        face(cx);
        let n = rand_range(cx, k(cx, "hover_short_min"), k(cx, "hover_short_max"), "hover");
        hover(cx, n).await;
    }
}

/// `0x1364`: close (< 300): a strafe, then a hover, the approach and the slash, or the spin.
async fn close(cx: &Cx) {
    let n = rand_range(cx, k(cx, "strafe_close_min"), k(cx, "strafe_close_max"), "strafe");
    strafe(cx, n).await;
    if rand(cx, "close attack") - k(cx, "close_approach_chance") < 0.0 {
        face(cx);
        let n = rand_range(cx, k(cx, "hover_min"), k(cx, "hover_max"), "hover");
        hover(cx, n).await;
        approach(cx, req::FLY, k(cx, "approach_speed"), k(cx, "approach_arrive"), k(cx, "approach_time")).await;
        slash(cx).await;
    } else {
        spin(cx).await;
        face(cx);
        let n = rand_range(cx, k(cx, "hover_min"), k(cx, "hover_max"), "hover");
        hover(cx, n).await;
    }
}

/// `0x123A`: far (≥ 600): face, hover, the charge.
async fn far(cx: &Cx) {
    face(cx);
    let n = rand_range(cx, k(cx, "hover_min"), k(cx, "hover_max"), "hover");
    hover(cx, n).await;
    charge(cx).await;
}

/// The appearance (`0x1966`, a room spawn → `0x3282`): request 69 with the fade-in, the script
/// waits `appear_wait` frames, then (actor flag 4, the idle queued) until it ends; facing Sora
/// at once; then the idle state.
async fn appear(cx: Cx) {
    cx.borrow_mut().x.home = me(&cx);
    fly(&cx, Vec3::ZERO);
    if cx.borrow().x.spawned {
        start(&cx, ECmd::Motion { request: req::APPEAR, blend: 0.0 });
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        wait(&cx, k(&cx, "appear_wait")).await;
        push(&cx, ECmd::Queue { request: req::IDLE, blend: 0.0 });
        while cx.borrow().w.motion == Some(req::APPEAR) {
            frame().await;
        }
    }
    // `0x33AC`: face Sora now.
    let to = {
        let io = cx.borrow();
        io.w.target.map(|t| t - io.w.pos)
    };
    if let Some(d) = to {
        let y = d.yaw();
        let mut io = cx.borrow_mut();
        io.body.yaw = y;
        io.body.yaw_target = y;
        io.out.push(ECmd::Yaw(y));
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The idle behaviour (`0x1442`): Sora's target is it → engaged; else wander about home.
async fn idle(cx: Cx) {
    loop {
        if cx.borrow().x.targeted {
            set_target(&cx, true);
            set_state(&cx, st::ENGAGED);
            return;
        }
        wander(&cx).await;
        frame().await;
    }
}

/// The engaged behaviour (`0x1022`): the climb / dive at a target far above / below, then by flat
/// distance close / mid / far; then engaged again.
async fn engaged(cx: Cx) {
    if let Some(tg) = target(&cx) {
        let dy = tg.y - me(&cx).y;
        if dy - k(&cx, "climb_dy") < 0.0 {
            vertical_move(&cx, true).await;
        } else if dy - k(&cx, "dive_dy") > 0.0 {
            vertical_move(&cx, false).await;
        }
    }
    let d = flat_dist(&cx);
    if d - k(&cx, "close_range") < 0.0 {
        close(&cx).await;
    } else if d - k(&cx, "mid_range") < 0.0 {
        mid(&cx).await;
    } else {
        far(&cx).await;
    }
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
}

/// The lost behaviour (`0xD84`): Sora (Re:CoM's battles have only him) becomes the target again.
async fn lost(cx: Cx) {
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
}

/// The hit behaviour (`0x5F2`): the flight stops; sometimes the last target is Sora again; close:
/// the spin (level with it) or the swoop (a little below it, high enough), then a back off; far
/// and angry: the power-up (225, the approach 226, 227) and, near and level, the slash; then a
/// strafe and engaged.
async fn hit(cx: Cx) {
    fly(&cx, Vec3::ZERO);
    if !cx.borrow().x.last || rand(&cx, "retarget") - k(&cx, "retarget_chance") < 0.0 {
        cx.borrow_mut().x.last = true;
    }
    set_target(&cx, true);
    let Some(tg) = target(&cx) else {
        set_state(&cx, st::LOST);
        return;
    };
    let d = flat_dist(&cx);
    let dy = tg.y - me(&cx).y;
    if d - k(&cx, "hit_close") < 0.0 {
        if dy.abs() - k(&cx, "hit_level") < 0.0 {
            spin(&cx).await;
        } else if dy - k(&cx, "hit_swoop_dy") < 0.0 && height(&cx) - k(&cx, "hit_swoop_height") > 0.0 {
            face(&cx);
            swoop(&cx).await;
        }
        face(&cx);
        back_off(&cx, k(&cx, "hit_back_min"), k(&cx, "hit_back_max")).await;
    } else if cx.borrow().x.anger as f32 - k(&cx, "anger_full") >= 0.0 {
        let r = rand(&cx, "power up");
        let low = cx.borrow().x.hp_ratio - 1.0 / k(&cx, "anger_hp_div") < 0.0;
        if r - k(&cx, "anger_power_chance") < 0.0 || low {
            face(&cx);
            start(&cx, ECmd::Motion { request: req::POWER, blend: 0.0 });
            wait_done(&cx).await;
            approach(&cx, req::POWER_FLY, k(&cx, "power_speed"), k(&cx, "power_arrive"), k(&cx, "power_time")).await;
            if flat_dist(&cx) - k(&cx, "power_reach") < 0.0 {
                play(&cx, req::POWER_END, k(&cx, "idle_blend"));
                let n = crate::enemy::script::motion_frames(&cx, req::POWER_END) - 2.0 - k(&cx, "power_tail");
                wait(&cx, n).await;
                let near = flat_dist(&cx) - k(&cx, "power_close") < 0.0;
                let level = target(&cx).is_some_and(|t| (t.y - me(&cx).y).abs() - k(&cx, "power_level") < 0.0);
                if near && level {
                    slash(&cx).await;
                    cx.borrow_mut().x.anger = 0;
                } else {
                    wait(&cx, k(&cx, "power_tail")).await;
                }
            } else {
                play(&cx, req::POWER_END, k(&cx, "idle_blend"));
                wait_done(&cx).await;
            }
        }
        // (Its other branch only acts when a line test finds a wall between: none in Re:CoM's
        // battle rooms.)
    }
    // (The charge on a target far away behind a wall (`0x92A`, `0xCE8`): no walls in Re:CoM's
    // battle rooms, KH1's line test `sys 77` finds nothing: Assumption.)
    let n = rand_range(&cx, k(&cx, "strafe_hit_min"), k(&cx, "strafe_hit_max"), "strafe");
    strafe(&cx, n).await;
    set_state(&cx, st::ENGAGED);
}

/// The supervisor (entry `0x3670`): after the init frame, each frame: while idle, Sora within
/// `idle_search` → engaged (the target set); a state request restarts the behaviour thread.
async fn supervisor(cx: Cx) {
    frame().await;
    loop {
        let (state, sora, me) = {
            let io = cx.borrow();
            (io.state.0, io.w.target, io.w.pos)
        };
        if state == st::IDLE {
            if let Some(t) = sora {
                if (t - me).length() - k(&cx, "idle_search") < 0.0 {
                    set_target(&cx, true);
                    set_state(&cx, st::ENGAGED);
                }
            }
        }
        if let Some(s) = take_state(&cx) {
            match s {
                st::IDLE => set_target(&cx, false),
                _ => set_target(&cx, true),
            }
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Air Soldier's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as the Shadow's: per-enemy seeds).
    /// `frames`: KH1 motion lengths by request. `spawned`: the room spawned it (its appearance
    /// plays; every Re:CoM battle appearance, owner 2026-11-02).
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, spawned: bool) -> Ai {
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp { spawned, hp_ratio: 1.0, ..Default::default() });
        Ai { io: Rc::new(RefCell::new(io)), main: None, behaviour: None, started: false }
    }

    pub fn consts(&self) -> Consts {
        self.io.borrow().c.clone()
    }

    /// One AI frame. Returns the commands.
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
                io.state = (st::NONE, false);
            }
            // `0x38C0`: the turn rate; `0x36A8`: flight with no velocity.
            let deg = k(&self.io, "turn_deg");
            set_turn_rate(&self.io, deg);
            fly(&self.io, Vec3::ZERO);
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
                st::LOST => Box::pin(lost(cx)),
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
    /// The hit handler (`0x3A90`): the clash stops the flight (`0x3B88`: `sys 135` with no velocity,
    /// this frame's steering turned back); a damaging hit adds to the anger meter by its kind
    /// (`0x3CC4`..`0x3D4A`); both ask for the hit state (an attacker is always there: Sora).
    fn on_hit(&self, news: HitNews) {
        {
            let mut io = self.io.borrow_mut();
            match news {
                HitNews::Clash => io.body.fly(Vec3::ZERO),
                HitNews::Damage => {
                    let add = match io.x.kind {
                        5 => io.c.num("anger_launch"),
                        4 => io.c.num("anger_knock"),
                        3 => io.c.num("anger_flinch"),
                        // Other kinds: no state change (`0x3D74`).
                        _ => return,
                    };
                    io.x.anger += add as i32;
                }
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
    fn set_hp_ratio(&self, r: f32) {
        self.io.borrow_mut().x.hp_ratio = r;
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["none", "idle", "engaged", "lost", "hit"].get(s as usize).copied().unwrap_or("?")
    }
    fn set_floor(&self, y: f32) {
        self.io.borrow_mut().floor = Some(y);
    }
    /// The attack rows' hit callbacks (`0x297E`, `0x241E`, `0x27B2`, `0x1E7E`, `0x222C`): the
    /// attack's landed flag.
    fn on_landed(&self, attack: u8) {
        if let Some(f) = self.io.borrow_mut().x.landed.get_mut(attack as usize) {
            *f = true;
        }
    }
    fn set_hit_kind(&self, kind: u8) {
        self.io.borrow_mut().x.kind = kind;
    }
    fn set_targeted(&self, by_sora: bool) {
        self.io.borrow_mut().x.targeted = by_sora;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test numbers: the player's script's own (the asset step reads them; the disassembly's
    /// values), the shared engine's.
    pub(crate) fn consts() -> Consts {
        let v: &[(&str, f32)] = &[
            ("turn_deg", 7.5), ("idle_search", 1500.0), ("wander_min", 200.0), ("wander_max", 800.0), ("wander_probe", 100.0), ("pit_depth", 1200.0),
            ("pit_lift", 1500.0), ("wander_high", 1200.0), ("wander_near_lo", -150.0), ("wander_near_hi", 150.0), ("wander_far_lo", -200.0), ("wander_far_hi", 200.0),
            ("fly_up", 30.0), ("fly_speed", 8.0), ("fly_arrive", 10.0), ("fly_time_min", 120.0), ("fly_time_max", 180.0), ("fly_low", 100.0),
            ("rest_min", 55.0), ("rest_max", 110.0), ("idle_blend", 8.0), ("climb_dy", -700.0), ("dive_dy", 700.0), ("close_range", 300.0),
            ("mid_range", 600.0), ("mid_kick_chance", 0.4), ("mid_swoop_chance", 0.7), ("close_approach_chance", 0.7), ("hover_min", 90.0),
            ("hover_max", 150.0), ("hover_short_min", 60.0), ("hover_short_max", 120.0), ("hover_near", 480.0), ("back_kick_min", 780.0),
            ("back_kick_max", 1200.0), ("back_up", 150.0), ("back_speed", 14.4), ("back_time", 45.0), ("back_hover_min", 60.0), ("back_hover_max", 90.0),
            ("approach_speed", 8.0), ("approach_arrive", 300.0), ("approach_time", 500.0), ("approach_accel", 0.5), ("approach_up", 50.0),
            ("strafe_speed", 6.0), ("strafe_turn", 3.0), ("strafe_turn_factor", 0.5), ("strafe_up", 50.0), ("strafe_above_min", 50.0),
            ("strafe_above_max", 100.0), ("strafe_down", 0.4), ("strafe_rise", -0.2), ("strafe_left", -90.0), ("strafe_right", 90.0),
            ("strafe_side_chance", 0.5), ("strafe_close_min", 60.0), ("strafe_close_max", 120.0), ("strafe_mid_min", 90.0), ("strafe_mid_max", 150.0),
            ("strafe_hit_min", 18.0), ("strafe_hit_max", 54.0), ("kick_near", 100.0), ("kick_up", 40.0), ("kick_speed", 20.0), ("slash_near", 50.0),
            ("slash_up", 20.0), ("slash_speed", 5.0), ("swoop_up", 120.0), ("swoop_aim_up", 130.0), ("swoop_rise_speed", 12.0), ("swoop_rise_time", 60.0),
            ("swoop_wait_a", 8.0), ("swoop_wait_b", 24.0), ("swoop_low", 20.0), ("swoop_down_speed", 4.0), ("swoop_down_time", 30.0),
            ("spin_off_min", 400.0), ("spin_off_max", 500.0), ("spin_off_up", 100.0), ("spin_tuck_speed", 6.0), ("spin_time", 150.0), ("spin_up", 100.0),
            ("spin_speed", 60.0), ("spin_near", 100.0), ("spin_close_speed", 30.0), ("charge_wait", 90.0), ("charge_speed", 50.0), ("charge_arrive", 100.0),
            ("charge_time_min", 120.0), ("charge_time_max", 180.0), ("charge_miss_add", 2.0), ("retarget_chance", 0.25), ("hit_close", 300.0),
            ("hit_level", 100.0), ("hit_swoop_dy", 217.5), ("hit_swoop_height", 100.0), ("hit_back_min", 600.0), ("hit_back_max", 900.0),
            ("anger_full", 100.0), ("anger_power_chance", 0.5), ("anger_hp_div", 4.0), ("power_speed", 12.0), ("power_arrive", 750.0), ("power_time", 600.0),
            ("power_reach", 900.0), ("power_tail", 46.0), ("power_close", 300.0), ("power_level", 100.0), ("anger_launch", 25.0), ("anger_knock", 10.0),
            ("anger_flinch", 5.0), ("climb_over", 150.0), ("dive_over", 350.0), ("vert_time", 300.0), ("vert_fast_time", 30.0), ("vert_fast_speed", 10.0),
            ("vert_slow_time", 20.0), ("vert_slow_speed", 1.0), ("vert_speed", 12.0), ("dive_stop_fast", 50.0), ("dive_stop_slow", 30.0), ("vert_tail", 50.0),
            ("vert_wait", 8.0), ("attack_blend", 8.0), ("motion_tail", 8.0), ("steer_cap", 30.0), ("appear_fade", 30.0), ("appear_wait", 30.0),
            ("walk_speed", 8.0), ("run_speed", 8.0), ("air_accel", 0.1), ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0),
            ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0),
            ("air_knock_speed", 16.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("weight", 1.0), ("gravity", crate::params::GRAVITY),
            ("fall_cap", 20.0), ("attack_blend", 8.0),
        ];
        Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), NAMES).unwrap()
    }

    fn frames() -> Vec<(u16, f32)> {
        use req::*;
        vec![(IDLE, 110.0), (FLY, 28.0), (APPEAR, 66.0), (KICK, 64.0), (SLASH, 58.0), (SWOOP, 88.0), (SPIN, 80.0), (CHARGE_UP, 108.0), (CHARGE, 20.0), (CHARGE_HIT, 42.0), (CHARGE_MISS, 80.0), (BACK, 36.0), (STRAFE_L, 36.0), (STRAFE_R, 36.0), (POWER, 50.0), (POWER_FLY, 64.0), (POWER_END, 60.0), (CLIMB, 34.0), (CLIMB_LOOP, 64.0), (CLIMB_END, 74.0), (TUCK, 50.0), (TUCK_LOOP, 60.0), (TUCK_END, 60.0)]
    }

    /// A flat floor at y = 0 (KH1 y down), Sora on it; the host plays motions to their length and
    /// queues, pauses the AI for actions, keeps polling it during steered attacks.
    struct Sim {
        ai: Ai,
        pos: Vec3,
        current: Option<u16>,
        played: f32,
        queued: Option<u16>,
        log: Vec<(usize, Cmd)>,
        frame: usize,
    }

    impl Sim {
        fn new(seed: u32) -> Sim {
            Sim { ai: Ai::new(consts(), frames(), seed, 0.0, true), pos: Vec3::new(0.0, -60.0, 0.0), current: None, played: 0.0, queued: None, log: Vec::new(), frame: 0 }
        }

        fn len(r: u16) -> f32 {
            frames().iter().find(|x| x.0 == r).map_or(30.0, |x| x.1)
        }

        fn run(&mut self, sora: impl Fn(usize) -> Vec3, n: usize) {
            for _ in 0..n {
                self.frame += 1;
                self.played += 1.0;
                if let Some(c) = self.current {
                    if self.played >= Self::len(c) {
                        if let Some(q) = self.queued.take() {
                            self.current = Some(q);
                            self.played = 0.0;
                        }
                    }
                }
                self.ai.set_floor(0.0);
                let w = World { step: 1.0, pos: self.pos, target: Some(sora(self.frame)), grounded: false, airborne: false, moved: 5.0, motion: self.current };
                for c in self.ai.frame(w) {
                    match c {
                        ECmd::Motion { request, .. } if self.current != Some(request) => {
                            self.current = Some(request);
                            self.played = 0.0;
                            self.queued = None;
                        }
                        ECmd::Attack { request, .. } => {
                            self.current = Some(request);
                            self.played = 0.0;
                            self.queued = None;
                        }
                        ECmd::Queue { request, .. } => self.queued = Some(request),
                        _ => {}
                    }
                    self.log.push((self.frame, c));
                }
                let c = self.ai.consts();
                let d = self.ai.body().step(1.0, self.pos.y, None, &c);
                self.pos += d;
                // Re:CoM's floor stops the body.
                if self.pos.y > 0.0 {
                    self.pos.y = 0.0;
                }
            }
        }

        fn attacks(&self) -> Vec<(usize, Attack)> {
            self.log.iter().filter_map(|(f, c)| match c {
                ECmd::Attack { attack, .. } => Some((*f, *attack)),
                _ => None,
            }).collect()
        }
    }

    /// The battle spawn: KH1's appearance (69), then it flies (movement state 4) and engages Sora
    /// within 15 m; it stays in the air.
    #[test]
    fn appears_flies_and_engages() {
        let mut s = Sim::new(7);
        s.run(|_| Vec3::new(0.0, 0.0, 1200.0), 3);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::Motion { request: req::APPEAR, .. })));
        assert!(s.ai.body().flying, "flight from the first frame (sys 135)");
        s.run(|_| Vec3::new(0.0, 0.0, 1200.0), 120);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))), "{:?}", s.log.iter().filter(|(_, c)| matches!(c, ECmd::State(_))).collect::<Vec<_>>());
        s.run(|_| Vec3::new(0.0, 0.0, 1200.0), 2000);
        assert!(!s.attacks().is_empty(), "it attacks");
        assert!(s.ai.body().flying);
    }

    /// A far Sora (≥ 6 m flat): the hover, then the charge's wind-up 210 and the charge 211, flown
    /// at 50 (eased by the flight).
    #[test]
    fn far_target_gets_the_charge() {
        let mut s = Sim::new(3);
        s.run(|_| Vec3::new(0.0, 0.0, 1400.0), 600);
        let first = s.attacks().first().copied();
        assert_eq!(first.map(|a| a.1), Some(Attack::Charge), "{:?}", s.attacks());
        let up = s.log.iter().position(|(_, c)| matches!(c, ECmd::Motion { request: req::CHARGE_UP, .. })).unwrap();
        let at = s.log.iter().position(|(_, c)| matches!(c, ECmd::Attack { attack: Attack::Charge, .. })).unwrap();
        assert!(s.log[at].0 - s.log[up].0 >= 90, "the 90-frame wind-up");
    }

    /// The hit handler: kind 5 adds 25 to the anger meter, kind 4 10, kind 3 5; other kinds change
    /// nothing; a clash stops the flight.
    #[test]
    fn hits_fill_the_anger_meter() {
        let mut s = Sim::new(1);
        s.run(|_| Vec3::new(0.0, 0.0, 500.0), 200);
        for (kind, add) in [(5u8, 25), (4, 10), (3, 5), (1, 0)] {
            let before = s.ai.io.borrow().x.anger;
            s.ai.set_hit_kind(kind);
            s.ai.on_hit(HitNews::Damage);
            assert_eq!(s.ai.io.borrow().x.anger - before, add, "kind {kind}");
        }
        s.ai.body().air_vel = Vec3::new(5.0, 0.0, 0.0);
        s.ai.on_hit(HitNews::Clash);
        assert_eq!(s.ai.body().air_vel, Vec3::ZERO);
    }

    /// No stall: Sora circling 2–9 m away for 20 000 frames, it keeps attacking (some attack at
    /// least every 1500 frames) and never lands.
    #[test]
    fn a_long_fight_never_stalls() {
        for seed in [1u32, 2, 3, 4, 5, 6] {
            let mut s = Sim::new(seed);
            let sora = |f: usize| {
                let a = f as f32 * 0.002;
                let r = 550.0 + 350.0 * (f as f32 * 0.0007).sin();
                Vec3::new(r * a.cos(), 0.0, r * a.sin())
            };
            let mut low = f32::INFINITY;
            for _ in 0..200 {
                s.run(sora, 100);
                low = low.min(-s.pos.y);
            }
            assert!(low >= 0.0, "seed {seed}: never under the floor");
            let at = s.attacks();
            assert!(at.len() > 10, "seed {seed}: {} attacks", at.len());
            let mut last = 0;
            for &(f, _) in &at {
                assert!(f - last < 1500, "seed {seed}: no attack from {last} to {f}");
                last = f;
            }
            assert!(s.ai.body().flying, "seed {seed}");
            let kinds: std::collections::HashSet<_> = at.iter().map(|a| a.1).collect();
            assert!(kinds.len() >= 3, "seed {seed}: {kinds:?}");
        }
    }
}
