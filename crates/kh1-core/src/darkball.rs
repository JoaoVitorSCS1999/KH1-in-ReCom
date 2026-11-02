//! KH1 FM's Darkball (`xa_ex_2280.mdls` part 5, `ex_2280.bd`), engine-agnostic: its AI script on the
//! shared enemy framework (`crate::enemy`), flying (KH1's movement state 4,
//! `crate::enemy::Body::flying`). M11 (`docs/recom-m11-enemies.md` § The Darkball). Re:CoM runs it
//! natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`).
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2280.bd` at checked
//! offsets). Script offsets are the player's `ex_2280.bd`'s (Checked (files): the script
//! disassembled with `tools/kh1-ai-script.py`; the syscalls named are Checked (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x1A28`: init `0x1BD0` (stats records LV1 `0x14` / LV2 `0x80` / `0xEC`, the work area,
//!   the handlers: hit `0x1E20`), flight with no velocity (`sys 135`), a yield, the turn rate 5°
//!   (`sys 125`); a placed Darkball (`sys 132` = 0) engages a target within 1000 or else searches
//!   (`0x520`), a spawned one plays its appearance (`0x96E` → `0x97E`); then each frame: nothing while
//!   it warps (`wp+0x4C`), a state request restarts the behaviour (1 idle `0x8C2` → `0x8DA`, 2
//!   engaged `0x65E` → `0x682`, 3 lost `0x520` → `0x538`, 4 hit `0x338` → `0x35C`), while idle a
//!   target within 1000 → engaged.
//! - the hit handler `0x1E20`: noticed (`wp+0x50`); a launch or knock-back (kinds 5 / 4) counts a hit
//!   (`wp+0x48`) and resets the light-hit run (`wp+0x54`); another non-clash hit counts both, the
//!   fifth light hit in a row marks the bite (`wp+0x48` = 100), the sixth and later take no reaction
//!   and no state change (the hit's reaction kind zeroed); both ask for the hit state 4.
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0x2112` (KH1 sinks it away (request
//! 70) and removes it when Sora is farther than 1500 / 1050 out of sight: in a Re:CoM battle room it
//! would never be defeated), the retargeting to another party member (`0x74C`, `0x10D8`: Sora is the
//! only target), the world rules (`0x1BD0`'s world 11 / 15 / 16 flags), the hits' KH1 attribute
//! (`hp[0xC] >> 6 & 15`: 8 (Gravity) asks for the stagger `wp+0x48` = 200, 9 / 11 no state change;
//! the host carries no attribute: every hit is attribute 0), the effects (`sys 74` / `75` / `21` /
//! `97`, `sys1.3`), the alpha fades (`0x18BA` / `0x18E6`: [`Cmd::FadeIn`] for the log only), the death
//! routine (Re:CoM's defeat).

use crate::enemy::script::{fly, frame, height, motion_frames, poll, pos, push, rand, rand_range, random_offset, set_air_accel, set_state, set_target, set_turn_rate, steer, take_state, target, tick_clock, wait_frames, warp};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Darkball's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The appearance (`0x9A0`), 172 frames.
    pub const APPEAR: u16 = 69;
    /// The warp: the vanish (`0x2C7A`), the reappearance (`0x2A78`) and the drop (`0x27F8`, with its
    /// row), 100 frames, hit [12, 28).
    pub const WARP: u16 = 200;
    /// The dash (`0xCC4`): the wind-up 201 (68 f), the dash 202 (60 f, looped, hit all along), the end
    /// 203 (72 f).
    pub const DASH_UP: u16 = 201;
    pub const DASH: u16 = 202;
    pub const DASH_END: u16 = 203;
    /// The bite (`0xC64`), an action, 152 frames, root motion, hit [22, 74).
    pub const BITE: u16 = 204;
    /// The notice (`0xBCE`) and the stagger (`0x3A0`), 162 frames.
    pub const NOTICE: u16 = 205;
}

/// The Darkball's attacks, in the order the host indexes them (the manifest's `attacks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    /// `0xC64`: the bite, an action (204, `0x10000CC`) with row `0x238`.
    Bite,
    /// `0xCC4`: the dash, a plain request (202) with row `0x158` (`sys 213` mode 2: it lives while 202
    /// plays): the script keeps steering it.
    Dash,
    /// `0x27F8`: the drop, the reappearance's request 200 with row `0x1C8`: the script keeps scaling
    /// it.
    Drop,
}

impl Attack {
    pub const ALL: [Attack; 3] = [Attack::Bite, Attack::Dash, Attack::Drop];

    pub fn request(self) -> u16 {
        match self {
            Attack::Bite => req::BITE,
            Attack::Dash => req::DASH,
            Attack::Drop => req::WARP,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Bite => "bite",
            Attack::Dash => "dash",
            Attack::Drop => "drop",
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The names of the Darkball's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    "turn_deg",
    "search_range",
    // Appearance `0x9A0`.
    "appear_up",
    "appear_cut",
    "appear_drift_delay",
    "appear_drift",
    "appear_drift_speed",
    "appear_wait",
    // Blends.
    "motion_blend",
    "rest_blend",
    "reappear_queue_blend",
    // Engaged `0x682`, notice `0xBCE`, close `0x7B6`, mid `0x806`.
    "notice_tail",
    "miss_limit",
    "far_range",
    "far_chance",
    "misses_close",
    "misses_mid",
    "side_near",
    "side_time",
    "side_speed",
    "dash_count",
    // Lost `0x538`.
    "lost_speed",
    "lost_time",
    "lost_near",
    "lost_back_range",
    "lost_back",
    "lost_up_lo",
    "lost_up_hi",
    "lost_fly_speed",
    "lost_fly_arrive",
    "lost_fly_time",
    // Idle `0x8DA`.
    "wander_min",
    "wander_max",
    "wander_low",
    "wander_up_lo",
    "wander_up_hi",
    "wander_speed",
    "wander_arrive",
    "wander_time",
    "wander_rest",
    // Fly to a point `0x331C`.
    "fly_check",
    "fly_low",
    "fly_rise",
    "fly_high",
    "fly_sink",
    "fly_stuck",
    "fly_new_min",
    "fly_new_max",
    // Turn to `0x34A0`.
    "turn_time",
    "turn_cos",
    // Approach at the side `0x3060`.
    "side_check",
    "side_bob_start",
    "side_range",
    "side_chance",
    "side_left",
    "side_right",
    "side_far_factor",
    "side_low",
    "bob_min",
    "bob_max",
    "side_above_lo",
    "side_rise",
    "side_above_hi",
    "side_sink",
    "bob_chance",
    "bob_up",
    "bob_down",
    "side_low_rise",
    "side_stuck",
    "side_stuck_wait",
    // Dash `0xCC4`.
    "dash_wait",
    "dash_time",
    "dash_speed",
    "dash_low",
    "dash_above",
    "dash_sink",
    "dash_rise",
    // Hit `0x35C`, handler `0x1E20`.
    "bite_mark",
    "gravity_mark",
    "hit_teleport",
    "hit_close",
    "hit_mid",
    "hit_low",
    "hit_rise",
    "hit_rise_speed",
    "light_limit",
    "armor_limit",
    // Warp `0x2304`.
    "warp_pause",
    "warp_near_lo",
    "warp_near_hi",
    "warp_above",
    "warp_far",
    "warp_hop_lo",
    "warp_hop_hi",
    "warp_low",
    "warp_low_up",
    "warp_accel",
    "glide_speed",
    "glide_arrive",
    "glide_time",
    "accel_after",
    "warp_reach",
    "warp_retry_wait",
    "warp_tries",
    "warp_give_up_accel",
    // Glide `0x25C0`.
    "glide_check_a",
    "glide_check_b",
    "glide_near",
    "glide_stop_accel",
    "glide_arrive_accel",
    "glide_low",
    "glide_rise",
    "glide_stuck",
    // Vanish `0x2C7A`.
    "vanish_cut",
    "vanish_low",
    "vanish_rise",
    "vanish_rise_speed",
    // Reappearance `0x2A78`.
    "reappear_rest",
    "reappear_cut",
    "reappear_drift_delay",
    "reappear_drift",
    "reappear_drift_speed",
    // Drop `0x27F8`.
    "strike_rest",
    "strike_wait",
    "strike_fade",
    "strike_grow_time",
    "strike_grow_div",
    "strike_hold",
    "strike_cut",
    "strike_peak",
    "strike_tail",
    // Bone code 0's place in the idle (the reappearance's warp, `sys 19(actor, 0, 0)`): the asset step
    // evaluates KH1's pose.
    "core_x",
    "core_y",
    "core_z",
];

/// The Darkball's numbers.
pub type Consts = Table;

/// The state objects (`wp+0x20`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const LOST: u8 = 3;
    pub const HIT: u8 = 4;
}

/// The Darkball's own work area: the actor's steering direction `+0x20` (a script writes it and
/// leaves it; only the speed `+0x2C` is zeroed each frame), the hit count `wp+0x48`, noticed
/// `wp+0x50`, the light-hit run `wp+0x54`, the rounds `wp+0x58`, warping `wp+0x4C`, the appearance,
/// the last hit's kind and whether it keeps its reaction.
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    dir: Vec3,
    hits: i32,
    noticed: bool,
    light: i32,
    misses: i32,
    warping: bool,
    spawned: bool,
    appeared: bool,
    kind: u8,
    keep: bool,
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

fn step(cx: &Cx) -> f32 {
    cx.borrow().w.step
}

/// `func_1382` (`sys 4(0)`): party 0, Sora.
fn sora(cx: &Cx) -> Vec3 {
    let io = cx.borrow();
    io.w.target.unwrap_or(io.w.pos)
}

/// The actor's target (`+0x74`): Sora once set (Re:CoM's battles have no other party member).
fn tgt(cx: &Cx) -> Vec3 {
    target(cx).unwrap_or_else(|| sora(cx))
}

/// `0x1520`: the distance from the actor to `p` (`sys 6`, 3D).
fn dist_to(cx: &Cx, p: Vec3) -> f32 {
    (p - me(cx)).length()
}

/// `sys 9`: the vector normalized (its length 0 stays 0).
fn norm(v: Vec3) -> Vec3 {
    if v.length() > 0.0 {
        v.normalized()
    } else {
        v
    }
}

/// `sys 10(v, deg)` on a direction: turned about the vertical (the Air Soldier's convention).
fn rotate(v: Vec3, deg: f32) -> Vec3 {
    let l = v.horizontal_length();
    Vec3::from_yaw(v.yaw() + deg.to_radians()) * l + Vec3::new(0.0, v.y, 0.0)
}

/// The facing vector `+0x30`.
fn facing(cx: &Cx) -> Vec3 {
    Vec3::from_yaw(cx.borrow().body.yaw)
}

/// `*(actor + 0x20) = v` (the speed is written apart).
fn set_dir(cx: &Cx, v: Vec3) {
    cx.borrow_mut().x.dir = v;
}

/// `*(actor + 0x24) = y`.
fn dir_y(cx: &Cx, y: f32) {
    cx.borrow_mut().x.dir.y = y;
}

/// `*(actor + 0x2C) = speed`: this frame's steering along `+0x20`.
fn go(cx: &Cx, speed: f32) {
    let d = cx.borrow().x.dir;
    steer(cx, d, speed);
}

/// `*(actor + 0x2C) = 0`.
fn halt(cx: &Cx) {
    steer(cx, Vec3::ZERO, 0.0);
}

/// `0x1744` / `sys 36`: the target yaw toward `p` (the engine turns).
fn yaw_to(cx: &Cx, p: Vec3) {
    let d = p - me(cx);
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `sys 36` toward a vector.
fn yaw_along(cx: &Cx, d: Vec3) {
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `sys 13` / `sys 12`: a motion request; `sys 172`'s clock restarts for a request that plays anew.
fn play(cx: &Cx, r: u16, blend: f32) {
    {
        let mut io = cx.borrow_mut();
        if io.w.motion != Some(r) {
            io.clock = (Some(r), 0.0);
        }
    }
    push(cx, ECmd::Motion { request: r, blend });
}

fn queue(cx: &Cx, r: u16, blend: f32) {
    push(cx, ECmd::Queue { request: r, blend });
}

/// `0x355E`: wait `n` frames.
async fn wait(cx: &Cx, n: f32) {
    wait_frames(cx, n).await;
}

/// A target within `range` (`0x1132` → `sys 48`): Sora (Assumption: KH1's search finds Sora when he
/// is within the range; `0x1132`'s fallbacks also give him within it).
fn search(cx: &Cx, range: f32) -> bool {
    cx.borrow().w.target.is_some() && dist_to(cx, sora(cx)) - range < 0.0
}

/// `0x34A0(p)`: turn toward `p` (flat) until the facing's cosine to it reaches `turn_cos` (`sys 100`),
/// `turn_time` frames at most; no yield when it already faces it.
async fn turn_to(cx: &Cx, p: Vec3) {
    let mut t = k(cx, "turn_time");
    while t > 0.0 {
        let mut d = p;
        d.y = me(cx).y;
        let d = d - me(cx);
        let f = facing(cx);
        let (lf, ld) = (f.length(), d.length());
        let cos = if lf != 0.0 && ld != 0.0 { (f.dot(d) / (lf * ld)).clamp(-1.0, 1.0) } else { 0.0 };
        if cos - k(cx, "turn_cos") >= 0.0 {
            break;
        }
        yaw_along(cx, d);
        t -= step(cx);
        frame().await;
    }
}

/// `0x331C(p, speed, arrive, time)`: fly to `p` (turning to it first) until within `arrive` or the
/// time is out; low over the floor it rises, high above Sora it sinks; stuck (moved less than
/// `fly_stuck` a frame after `fly_check` frames) it stops with a new point.
async fn fly_to(cx: &Cx, mut p: Vec3, speed: f32, arrive: f32, time: f32) {
    let mut t = time;
    let check = t - k(cx, "fly_check");
    turn_to(cx, p).await;
    while t > 0.0 {
        let d = p - me(cx);
        if d.length() - arrive < 0.0 {
            halt(cx);
            break;
        }
        set_dir(cx, norm(d));
        if height(cx) - k(cx, "fly_low") < 0.0 {
            dir_y(cx, k(cx, "fly_rise"));
        } else if height(cx) - k(cx, "fly_high") >= 0.0 && me(cx).y - sora(cx).y < 0.0 {
            dir_y(cx, k(cx, "fly_sink"));
        }
        go(cx, speed);
        frame().await;
        if t - check < 0.0 && cx.borrow().w.moved - k(cx, "fly_stuck") * step(cx) < 0.0 {
            // A new point (the caller's, unused by them) and the end.
            p = random_offset(cx, k(cx, "fly_new_min"), k(cx, "fly_new_max")) + me(cx);
            let _ = p;
            t = 0.0;
        }
        t -= step(cx);
    }
}

/// `0x3060(target, near, time, speed)`: approach `who` (the target or Sora), at its side (the facing
/// turned ±80°, chosen once) within `side_range`, straight at twice the speed beyond; bobbing up and
/// down between `side_above_lo` and `side_above_hi` above it; until within `near`, the time is out or
/// it is stuck.
async fn approach_side(cx: &Cx, who: fn(&Cx) -> Vec3, near: f32, time: f32, speed: f32) {
    let side = rand(cx, "side");
    let mut t = time;
    let check = t - k(cx, "side_check");
    let mut bob = k(cx, "side_bob_start");
    let mut bob_r = 0.0;
    turn_to(cx, who(cx)).await;
    while t > 0.0 {
        if dist_to(cx, who(cx)) - near < 0.0 {
            halt(cx);
            frame().await;
            return;
        }
        yaw_to(cx, who(cx));
        let mut d = facing(cx);
        let v = if dist_to(cx, who(cx)) - k(cx, "side_range") <= 0.0 {
            d = rotate(d, if side - k(cx, "side_chance") > 0.0 { k(cx, "side_left") } else { k(cx, "side_right") });
            speed
        } else {
            speed * k(cx, "side_far_factor")
        };
        turn_to(cx, who(cx)).await;
        set_dir(cx, d);
        if height(cx) - k(cx, "side_low") > 0.0 {
            if bob - rand_range(cx, k(cx, "bob_min"), k(cx, "bob_max"), "bob time") >= 0.0 {
                bob_r = rand(cx, "bob");
                bob = 0.0;
            }
            let ty = who(cx).y;
            if me(cx).y - (ty - k(cx, "side_above_lo")) > 0.0 {
                dir_y(cx, k(cx, "side_rise"));
            } else if me(cx).y - (ty - k(cx, "side_above_hi")) < 0.0 {
                dir_y(cx, k(cx, "side_sink"));
            } else if bob_r - k(cx, "bob_chance") > 0.0 {
                dir_y(cx, k(cx, "bob_up"));
            } else {
                dir_y(cx, k(cx, "bob_down"));
            }
        } else {
            dir_y(cx, k(cx, "side_low_rise"));
        }
        go(cx, v);
        frame().await;
        if t - check < 0.0 && cx.borrow().w.moved - speed * k(cx, "side_stuck") * step(cx) < 0.0 {
            turn_to(cx, who(cx)).await;
            wait(cx, k(cx, "side_stuck_wait")).await;
            return;
        }
        let s = step(cx);
        t -= s;
        bob += s;
    }
}

/// `0x25C0(p, speed, arrive, time)`: the invisible glide to `p`: near the target (after
/// `glide_check_a` frames) or within `arrive` of the point only vertical, done once level (the
/// stats block's easing set to `glide_stop_accel` / `glide_arrive_accel`); stuck after
/// `glide_check_b` frames: done.
async fn glide(cx: &Cx, p: Vec3, speed: f32, arrive: f32, time: f32) {
    let mut t = time;
    let near_after = t - k(cx, "glide_check_a");
    let check = t - k(cx, "glide_check_b");
    while t > 0.0 {
        let mut d = p - me(cx);
        yaw_along(cx, d);
        let s = step(cx);
        if dist_to(cx, tgt(cx)) - k(cx, "glide_near") <= 0.0 && near_after - t >= 0.0 {
            if d.y.abs() - speed * s < 0.0 {
                set_air_accel(cx, k(cx, "glide_stop_accel"));
                halt(cx);
                return;
            }
            d.x = 0.0;
            d.z = 0.0;
        }
        if d.length() - arrive < 0.0 {
            if d.y.abs() - speed * s < 0.0 {
                set_air_accel(cx, k(cx, "glide_arrive_accel"));
                halt(cx);
                return;
            }
            d.x = 0.0;
            d.z = 0.0;
        }
        set_dir(cx, norm(d));
        if height(cx) - k(cx, "glide_low") < 0.0 {
            dir_y(cx, k(cx, "glide_rise"));
        }
        go(cx, speed);
        t -= s;
        frame().await;
        if t - check < 0.0 && cx.borrow().w.moved - speed * k(cx, "glide_stuck") * step(cx) < 0.0 {
            set_air_accel(cx, k(cx, "glide_arrive_accel"));
            halt(cx);
            return;
        }
    }
}

/// `0x2C7A`: the vanish: warping (the supervisor waits), request 200 (the idle queued), unhittable
/// (flags 0x20 | 0x40 | 0x10), shrinking from 1 to 0 over len(200) − `vanish_cut` frames facing the
/// target (rising when low), back to where it started (`sys 86` with that position's fourth word,
/// 1.0, as the facing), its scale 1 and its model hidden (flags 0x10 and 0x1 cleared).
async fn vanish(cx: &Cx) {
    cx.borrow_mut().x.warping = true;
    play(cx, req::WARP, k(cx, "motion_blend"));
    queue(cx, req::IDLE, k(cx, "motion_blend"));
    let start = me(cx);
    push(cx, ECmd::Hittable(false));
    let n = motion_frames(cx, req::WARP) - k(cx, "vanish_cut");
    let mut t = 0.0;
    while t - n < 0.0 {
        yaw_to(cx, tgt(cx));
        push(cx, ECmd::Scale(1.0 - t / n));
        if height(cx) - k(cx, "vanish_low") < 0.0 {
            set_dir(cx, Vec3::new(0.0, k(cx, "vanish_rise"), 0.0));
            go(cx, k(cx, "vanish_rise_speed"));
        }
        t += step(cx);
        frame().await;
    }
    warp(cx, start, 1.0);
    push(cx, ECmd::Scale(1.0));
    push(cx, ECmd::Visible(false));
}

/// Bone code 0's place (`sys 19(actor, 0, 0)`: bone 3, the core) in the world, from the asset
/// step's pose (KH1's local +z is the facing: Assumption, as `Vec3::from_yaw`).
fn core(cx: &Cx) -> Vec3 {
    let o = Vec3::new(k(cx, "core_x"), k(cx, "core_y"), k(cx, "core_z"));
    let yaw = cx.borrow().body.yaw;
    let (s, c) = yaw.sin_cos();
    me(cx) + Vec3::new(o.x * c + o.z * s, o.y, -o.x * s + o.z * c)
}

/// `0x2A78`: the reappearance without the drop: the idle, the turn to the target, request 200, the
/// model shown, the body moved to its core (`sys 19` → `sys 86`), growing from 0 to 1 over len(200)
/// − `reappear_cut` frames, sinking from `reappear_drift_delay`; hittable again, warping done.
async fn reappear(cx: &Cx) {
    halt(cx);
    play(cx, req::IDLE, k(cx, "rest_blend"));
    wait(cx, k(cx, "reappear_rest")).await;
    frame().await;
    turn_to(cx, tgt(cx)).await;
    play(cx, req::WARP, k(cx, "motion_blend"));
    queue(cx, req::IDLE, k(cx, "reappear_queue_blend"));
    push(cx, ECmd::Visible(true));
    let n = motion_frames(cx, req::WARP) - k(cx, "reappear_cut");
    let to = core(cx);
    let yaw = cx.borrow().body.yaw;
    warp(cx, to, yaw);
    push(cx, ECmd::FadeIn(n));
    let mut t = 0.0;
    while t - n < 0.0 {
        t += step(cx);
        yaw_to(cx, tgt(cx));
        push(cx, ECmd::Scale(t / n));
        if t - k(cx, "reappear_drift_delay") >= 0.0 {
            set_dir(cx, Vec3::new(0.0, k(cx, "reappear_drift"), 0.0));
            go(cx, k(cx, "reappear_drift_speed"));
        }
        frame().await;
    }
    push(cx, ECmd::Scale(1.0));
    push(cx, ECmd::Hittable(true));
    cx.borrow_mut().x.warping = false;
}

/// `0x27F8`: the drop: the idle, request 200 with its row (KH1 makes the row `strike_wait` frames
/// after the request; its hit window starts at 12, so the hits are the same), the model shown,
/// growing to `strike_peak` over `strike_grow_time` frames, held, then back to 1 over len(200) −
/// `strike_cut` frames; hittable again, warping done; the idle when 200 has played (KH1's queue: the
/// host drops a steered attack's queue).
async fn strike(cx: &Cx) {
    halt(cx);
    play(cx, req::IDLE, k(cx, "rest_blend"));
    wait(cx, k(cx, "strike_rest")).await;
    yaw_to(cx, tgt(cx));
    {
        let mut io = cx.borrow_mut();
        io.clock = (Some(req::WARP), 0.0);
    }
    push(cx, ECmd::Attack { request: req::WARP, attack: Attack::Drop, blend: k(cx, "motion_blend") });
    wait(cx, k(cx, "strike_wait")).await;
    push(cx, ECmd::Visible(true));
    push(cx, ECmd::FadeIn(k(cx, "strike_fade")));
    let grow = k(cx, "strike_grow_time");
    let mut t = 0.0;
    while t - grow < 0.0 {
        t += step(cx);
        push(cx, ECmd::Scale(t / k(cx, "strike_grow_div")));
        frame().await;
    }
    wait(cx, k(cx, "strike_hold")).await;
    let n = motion_frames(cx, req::WARP) - k(cx, "strike_cut");
    let mut t = 0.0;
    while t - n < 0.0 {
        t += step(cx);
        push(cx, ECmd::Scale(k(cx, "strike_peak") - t / n));
        frame().await;
    }
    push(cx, ECmd::Scale(1.0));
    push(cx, ECmd::Hittable(true));
    cx.borrow_mut().x.warping = false;
    wait(cx, k(cx, "strike_tail")).await;
    play(cx, req::IDLE, k(cx, "motion_blend"));
}

/// `0x2304`: the warp: the vanish, a pause, then invisible glides alternating between a point
/// `warp_hop_lo..hi` from itself (a target farther than `warp_far`: one near it) and one
/// `warp_near_lo..hi` from the target `warp_above` above it, until within `warp_reach` of the target
/// (the drop) or `warp_tries` tries (the plain reappearance).
async fn teleport(cx: &Cx) {
    let mut tries = 0.0;
    let mut phase = 0;
    vanish(cx).await;
    frame().await;
    wait(cx, k(cx, "warp_pause")).await;
    let near_target = |cx: &Cx| {
        let mut p = random_offset(cx, k(cx, "warp_near_lo"), k(cx, "warp_near_hi")) + tgt(cx);
        p.y -= k(cx, "warp_above");
        p
    };
    loop {
        let p = if phase == 1 {
            phase = 0;
            near_target(cx)
        } else {
            phase = 1;
            if dist_to(cx, tgt(cx)) - k(cx, "warp_far") > 0.0 {
                near_target(cx)
            } else {
                let mut p = random_offset(cx, k(cx, "warp_hop_lo"), k(cx, "warp_hop_hi")) + me(cx);
                if height(cx) - k(cx, "warp_low") < 0.0 {
                    p.y -= k(cx, "warp_low_up");
                }
                p
            }
        };
        set_air_accel(cx, k(cx, "warp_accel"));
        glide(cx, p, k(cx, "glide_speed"), k(cx, "glide_arrive"), k(cx, "glide_time")).await;
        set_air_accel(cx, k(cx, "accel_after"));
        frame().await;
        if dist_to(cx, tgt(cx)) - k(cx, "warp_reach") <= 0.0 {
            break;
        }
        wait(cx, k(cx, "warp_retry_wait")).await;
        tries += 1.0;
        if tries - k(cx, "warp_tries") >= 0.0 {
            break;
        }
        frame().await;
    }
    if tries - k(cx, "warp_tries") >= 0.0 {
        set_air_accel(cx, k(cx, "warp_give_up_accel"));
        halt(cx);
        reappear(cx).await;
        set_air_accel(cx, k(cx, "accel_after"));
    } else {
        strike(cx).await;
    }
    frame().await;
}

/// `0xC64`: the bite: turn to the target, then the action 204 with its row (the idle queued, bit 30).
async fn bite(cx: &Cx) {
    turn_to(cx, tgt(cx)).await;
    frame().await;
    push(cx, ECmd::Action { request: req::BITE, attack: Some(Attack::Bite), then: req::IDLE, blend: k(cx, "attack_blend") });
    frame().await;
}

/// `0xCC4`: the dash: turn to the target, the wind-up 201 (facing it), then 202 with its row, dashing
/// along the facing at `dash_speed` for `dash_time` frames (sinking when high above the target, rising
/// when low), the end 203 (the idle queued).
async fn dash(cx: &Cx) {
    turn_to(cx, tgt(cx)).await;
    play(cx, req::DASH_UP, k(cx, "motion_blend"));
    wait(cx, k(cx, "dash_wait")).await;
    let mut t = motion_frames(cx, req::DASH_UP);
    while t > 0.0 {
        yaw_to(cx, tgt(cx));
        t -= step(cx);
        frame().await;
    }
    {
        let mut io = cx.borrow_mut();
        io.clock = (Some(req::DASH), 0.0);
    }
    push(cx, ECmd::Attack { request: req::DASH, attack: Attack::Dash, blend: k(cx, "motion_blend") });
    let mut t = k(cx, "dash_time");
    let mut first = true;
    while t > 0.0 {
        if !first {
            // `sys 12(202, 0)`: it plays (nothing new).
            play(cx, req::DASH, 0.0);
        }
        first = false;
        set_dir(cx, norm(facing(cx)));
        if height(cx) - k(cx, "dash_low") > 0.0 {
            if me(cx).y - (tgt(cx).y - k(cx, "dash_above")) < 0.0 {
                dir_y(cx, k(cx, "dash_sink"));
            }
        } else {
            dir_y(cx, k(cx, "dash_rise"));
        }
        go(cx, k(cx, "dash_speed"));
        t -= step(cx);
        frame().await;
    }
    halt(cx);
    let n = motion_frames(cx, req::DASH_END);
    play(cx, req::DASH_END, k(cx, "motion_blend"));
    queue(cx, req::IDLE, k(cx, "motion_blend"));
    wait(cx, n).await;
    frame().await;
}

/// `0xBCE`: the notice: noticed, request 205 (the idle queued), len(205) + `notice_tail` frames facing
/// Sora.
async fn notice(cx: &Cx) {
    cx.borrow_mut().x.noticed = true;
    yaw_to(cx, tgt(cx));
    play(cx, req::NOTICE, k(cx, "motion_blend"));
    queue(cx, req::IDLE, k(cx, "motion_blend"));
    let mut t = motion_frames(cx, req::NOTICE) + k(cx, "notice_tail");
    while t > 0.0 {
        yaw_to(cx, sora(cx));
        t -= step(cx);
        frame().await;
    }
}

/// `0x806`: the side approach to `side_near` of the target, then `dash_count` dashes; a round.
async fn approach_dash(cx: &Cx) {
    approach_side(cx, tgt, k(cx, "side_near"), k(cx, "side_time"), k(cx, "side_speed")).await;
    frame().await;
    let n = k(cx, "dash_count");
    let mut i = 0.0;
    while i - n < 0.0 {
        dash(cx).await;
        i += 1.0;
        frame().await;
    }
    let m = k(cx, "misses_mid") as i32;
    cx.borrow_mut().x.misses += m;
}

/// `0x7B6`: close (within `far_range`): the bite, the warp, the side approach and the dashes; two
/// rounds.
async fn close(cx: &Cx) {
    bite(cx).await;
    teleport(cx).await;
    frame().await;
    approach_dash(cx).await;
    let m = k(cx, "misses_close") as i32;
    cx.borrow_mut().x.misses += m;
}

/// The appearance (`0x9A0`, a room spawn): unhittable, request 69 (the idle queued), 80 up
/// (`sys 86`), growing from 0 to 1 over len(69) − `appear_cut` frames facing Sora, sinking from
/// `appear_drift_delay`; hittable, a wait; then the idle state.
async fn appear(cx: Cx) {
    push(&cx, ECmd::Hittable(false));
    play(&cx, req::APPEAR, k(&cx, "motion_blend"));
    queue(&cx, req::IDLE, 0.0);
    let mut p = me(&cx);
    p.y -= k(&cx, "appear_up");
    let yaw = cx.borrow().body.yaw;
    warp(&cx, p, yaw);
    let n = motion_frames(&cx, req::APPEAR) - k(&cx, "appear_cut");
    push(&cx, ECmd::FadeIn(n));
    let mut t = 0.0;
    while t - n < 0.0 {
        yaw_to(&cx, sora(&cx));
        t += step(&cx);
        push(&cx, ECmd::Scale(t / n));
        if t - k(&cx, "appear_drift_delay") >= 0.0 {
            set_dir(&cx, Vec3::new(0.0, k(&cx, "appear_drift"), 0.0));
            go(&cx, k(&cx, "appear_drift_speed"));
        }
        frame().await;
    }
    push(&cx, ECmd::Scale(1.0));
    push(&cx, ECmd::Hittable(true));
    wait(&cx, k(&cx, "appear_wait")).await;
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The idle behaviour (`0x8DA`): not noticed; wander: a point `wander_min..max` away (raised when
/// low), turn to it, fly to it, rest.
async fn idle(cx: Cx) {
    cx.borrow_mut().x.noticed = false;
    loop {
        let mut p = random_offset(&cx, k(&cx, "wander_min"), k(&cx, "wander_max"));
        if height(&cx) - k(&cx, "wander_low") < 0.0 {
            p.y -= rand_range(&cx, k(&cx, "wander_up_lo"), k(&cx, "wander_up_hi"), "wander height");
        }
        let p = p + me(&cx);
        turn_to(&cx, p).await;
        fly_to(&cx, p, k(&cx, "wander_speed"), k(&cx, "wander_arrive"), k(&cx, "wander_time")).await;
        wait(&cx, k(&cx, "wander_rest")).await;
        frame().await;
    }
}

/// The engaged behaviour (`0x682`): the notice once; then rounds: after `miss_limit` rounds lost;
/// beyond `far_range` the warp (`far_chance`) or the side approach and dashes, else the close round.
async fn engaged(cx: Cx) {
    if !cx.borrow().x.noticed {
        notice(&cx).await;
    }
    loop {
        if cx.borrow().x.misses as f32 - k(&cx, "miss_limit") >= 0.0 {
            cx.borrow_mut().x.misses = 0;
            set_state(&cx, st::LOST);
            return;
        }
        if dist_to(&cx, tgt(&cx)) - k(&cx, "far_range") > 0.0 {
            if rand(&cx, "far attack") - k(&cx, "far_chance") > 0.0 {
                teleport(&cx).await;
                let m = k(&cx, "misses_mid") as i32;
                cx.borrow_mut().x.misses += m;
                frame().await;
            } else {
                approach_dash(&cx).await;
            }
        } else {
            close(&cx).await;
        }
        frame().await;
    }
}

/// The lost behaviour (`0x538`): the side approach to Sora; near him, back off `lost_back` along the
/// line from him (a little higher); Sora within `search_range` → engaged.
async fn lost(cx: Cx) {
    loop {
        approach_side(&cx, sora, k(&cx, "lost_near"), k(&cx, "lost_time"), k(&cx, "lost_speed")).await;
        if dist_to(&cx, sora(&cx)) - k(&cx, "lost_back_range") <= 0.0 {
            let s = sora(&cx);
            let p = Vec3::new(s.x, me(&cx).y, s.z) - me(&cx);
            frame().await;
            let mut p = norm(p) * k(&cx, "lost_back") + me(&cx);
            p.y -= rand_range(&cx, k(&cx, "lost_up_lo"), k(&cx, "lost_up_hi"), "lost height");
            fly_to(&cx, p, k(&cx, "lost_fly_speed"), k(&cx, "lost_fly_arrive"), k(&cx, "lost_fly_time")).await;
            frame().await;
        }
        if search(&cx, k(&cx, "search_range")) {
            set_target(&cx, true);
            set_state(&cx, st::ENGAGED);
            return;
        }
        frame().await;
    }
}

/// The hit behaviour (`0x35C`): the fifth light hit's bite; the stagger (205, a Gravity hit: not
/// reached, see the module notes); then under `hit_teleport` hits the bite (within `hit_close`) or a
/// dash (within `hit_mid`) and engaged, else the warp and lost.
async fn hit(cx: Cx) {
    set_target(&cx, true);
    let hits = cx.borrow().x.hits as f32;
    if hits - k(&cx, "bite_mark") == 0.0 {
        cx.borrow_mut().x.hits = 0;
        bite(&cx).await;
        cx.borrow_mut().x.light = 0;
        frame().await;
        set_state(&cx, st::ENGAGED);
        return;
    }
    if hits - k(&cx, "gravity_mark") == 0.0 {
        cx.borrow_mut().x.hits = 0;
        play(&cx, req::NOTICE, k(&cx, "motion_blend"));
        queue(&cx, req::IDLE, k(&cx, "motion_blend"));
        let mut t = motion_frames(&cx, req::NOTICE) + k(&cx, "notice_tail");
        while t > 0.0 {
            if height(&cx) - k(&cx, "hit_low") < 0.0 {
                dir_y(&cx, k(&cx, "hit_rise"));
                go(&cx, k(&cx, "hit_rise_speed"));
            }
            yaw_to(&cx, tgt(&cx));
            t -= step(&cx);
            frame().await;
        }
    }
    let d = dist_to(&cx, tgt(&cx));
    if (cx.borrow().x.hits as f32) - k(&cx, "hit_teleport") < 0.0 {
        if d - k(&cx, "hit_close") < 0.0 {
            bite(&cx).await;
        } else if d - k(&cx, "hit_mid") < 0.0 {
            dash(&cx).await;
        }
        cx.borrow_mut().x.light = 0;
        set_state(&cx, st::ENGAGED);
    } else {
        cx.borrow_mut().x.hits = 0;
        teleport(&cx).await;
        cx.borrow_mut().x.light = 0;
        set_state(&cx, st::LOST);
    }
}

/// The supervisor (entry `0x1A28`): after the init frame the turn rate; a placed Darkball engages or
/// searches; then each frame: nothing while it warps, a state request restarts the behaviour, while
/// idle Sora within `search_range` → engaged.
async fn supervisor(cx: Cx) {
    frame().await;
    let deg = k(&cx, "turn_deg");
    set_turn_rate(&cx, deg);
    if !cx.borrow().x.spawned {
        if search(&cx, k(&cx, "search_range")) {
            set_target(&cx, true);
            set_state(&cx, st::ENGAGED);
        } else {
            set_state(&cx, st::LOST);
        }
    }
    loop {
        while cx.borrow().x.warping {
            frame().await;
        }
        if let Some(s) = take_state(&cx) {
            match s {
                st::IDLE | st::LOST => set_target(&cx, false),
                _ => set_target(&cx, true),
            }
            push(&cx, ECmd::State(s));
        }
        if cx.borrow().state.0 == st::IDLE && search(&cx, k(&cx, "search_range")) {
            set_target(&cx, true);
            set_state(&cx, st::ENGAGED);
        }
        frame().await;
    }
}

/// The Darkball's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as the Shadow's: per-enemy seeds).
    /// `frames`: KH1 motion lengths by request. `spawned`: the room spawned it (its appearance
    /// plays; every Re:CoM battle appearance, owner 2026-11-02; a spawn type other than 1:
    /// Assumption, `0x1132`'s type-1 rule not taken).
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, spawned: bool) -> Ai {
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp { spawned, keep: true, ..Default::default() });
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
            self.io.borrow_mut().state = (st::NONE, false);
            // `0x1A60`: flight with no velocity.
            fly(&self.io, Vec3::ZERO);
            self.main = Some(Box::pin(supervisor(self.io.clone())));
            self.behaviour = Some(Box::pin(async {}));
        }
        poll(&mut self.main);
        let start_appear = {
            let io = self.io.borrow();
            self.behaviour.is_none() && io.x.spawned && !io.x.appeared && io.state.0 == st::NONE
        };
        if start_appear {
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
        // The steering this frame: `+0x20` × `+0x2C` (the body steps it).
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
    /// The hit handler (`0x1E20`): noticed; a clash asks for the hit state; a launch or knock-back
    /// (kinds 5 / 4, `0x1F6C`) counts a hit and ends the light run; another hit (`0x1FBC`) counts
    /// both, the `light_limit`th light hit in a row marks the bite, from the `armor_limit`th on the
    /// hit has no reaction and no state change (`0x2034..0x207C`; KH1's attribute-10 and status
    /// (`actor+0x88 & 0x102`) exceptions: not carried, Assumption).
    fn on_hit(&self, news: HitNews) {
        {
            let mut io = self.io.borrow_mut();
            io.x.noticed = true;
            io.x.keep = true;
            if news == HitNews::Damage {
                match io.x.kind {
                    4 | 5 => {
                        io.x.hits += 1;
                        io.x.light = 0;
                    }
                    6 | 7 => {}
                    _ => {
                        io.x.hits += 1;
                        io.x.light += 1;
                        if io.x.light as f32 - io.c.num("light_limit") >= 0.0 {
                            io.x.hits = io.c.num("bite_mark") as i32;
                        }
                        if io.x.light as f32 - io.c.num("armor_limit") >= 0.0 {
                            io.x.keep = false;
                            return;
                        }
                    }
                }
            }
        }
        set_state(&self.io, st::HIT);
    }
    fn keeps_reaction(&self) -> bool {
        self.io.borrow().x.keep
    }
    fn risen(&self) -> bool {
        self.io.borrow().x.appeared
    }
    fn skip_rise(&self) {
        self.io.borrow_mut().x.spawned = false;
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["none", "idle", "engaged", "lost", "hit"].get(s as usize).copied().unwrap_or("?")
    }
    fn set_floor(&self, y: f32) {
        self.io.borrow_mut().floor = Some(y);
    }
    fn set_hit_kind(&self, kind: u8) {
        self.io.borrow_mut().x.kind = kind;
    }
    /// The host's quiet watchdog: engaged again (as the supervisor's state request does), flying
    /// with no velocity, not warping, the model shown at its scale and hittable (a warp it cut).
    fn reset(&self) {
        {
            let mut io = self.io.borrow_mut();
            io.x.appeared = true;
            io.x.warping = false;
            io.body.fly(Vec3::ZERO);
            io.out.push(ECmd::Visible(true));
            io.out.push(ECmd::Scale(1.0));
            io.out.push(ECmd::Hittable(true));
        }
        set_state(&self.io, st::ENGAGED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test numbers: the player's script's own (the asset step reads them; the disassembly's
    /// values), the shared engine's; the core offset is the asset step's idle pose (rounded).
    pub(crate) fn consts() -> Consts {
        let v: &[(&str, f32)] = &[
            ("turn_deg", 5.0), ("search_range", 1000.0), ("appear_up", 80.0), ("appear_cut", 50.0), ("appear_drift_delay", 30.0), ("appear_drift", 0.3),
            ("appear_drift_speed", 1.0), ("appear_wait", 50.0), ("motion_blend", 8.0), ("rest_blend", 20.0), ("reappear_queue_blend", 10.0), ("notice_tail", 8.0),
            ("miss_limit", 2.0), ("far_range", 600.0), ("far_chance", 0.5), ("misses_close", 2.0), ("misses_mid", 1.0), ("side_near", 450.0), ("side_time", 300.0),
            ("side_speed", 4.0), ("dash_count", 3.0), ("lost_speed", 4.0), ("lost_time", 300.0), ("lost_near", 300.0), ("lost_back_range", 300.0), ("lost_back", -800.0),
            ("lost_up_lo", 0.0), ("lost_up_hi", 100.0), ("lost_fly_speed", 6.0), ("lost_fly_arrive", 100.0), ("lost_fly_time", 500.0), ("wander_min", 500.0),
            ("wander_max", 1000.0), ("wander_low", 70.0), ("wander_up_lo", 50.0), ("wander_up_hi", 100.0), ("wander_speed", 3.0), ("wander_arrive", 10.0),
            ("wander_time", 500.0), ("wander_rest", 30.0), ("fly_check", 30.0), ("fly_low", 70.0), ("fly_rise", -0.4), ("fly_high", 150.0), ("fly_sink", 0.1),
            ("fly_stuck", 0.5), ("fly_new_min", 500.0), ("fly_new_max", 1000.0), ("turn_time", 200.0), ("turn_cos", 0.9), ("side_check", 10.0), ("side_bob_start", 60.0),
            ("side_range", 1000.0), ("side_chance", 0.5), ("side_left", 80.0), ("side_right", -80.0), ("side_far_factor", 2.0), ("side_low", 70.0), ("bob_min", 20.0),
            ("bob_max", 40.0), ("side_above_lo", 50.0), ("side_rise", -0.1), ("side_above_hi", 150.0), ("side_sink", 0.1), ("bob_chance", 0.5), ("bob_up", -0.2),
            ("bob_down", 0.2), ("side_low_rise", -0.1), ("side_stuck", 0.25), ("side_stuck_wait", 20.0), ("dash_wait", 8.0), ("dash_time", 60.0), ("dash_speed", 24.0),
            ("dash_low", 70.0), ("dash_above", 70.0), ("dash_sink", 0.1), ("dash_rise", -0.4), ("bite_mark", 100.0), ("gravity_mark", 200.0), ("hit_teleport", 5.0),
            ("hit_close", 350.0), ("hit_mid", 1000.0), ("hit_low", 70.0), ("hit_rise", -0.4), ("hit_rise_speed", 1.0), ("light_limit", 5.0), ("armor_limit", 6.0),
            ("warp_pause", 30.0), ("warp_near_lo", 10.0), ("warp_near_hi", 50.0), ("warp_above", 70.0), ("warp_far", 1000.0), ("warp_hop_lo", 500.0), ("warp_hop_hi", 600.0),
            ("warp_low", 50.0), ("warp_low_up", 50.0), ("warp_accel", 0.1), ("glide_speed", 24.0), ("glide_arrive", 300.0), ("glide_time", 600.0), ("accel_after", 0.2),
            ("warp_reach", 300.0), ("warp_retry_wait", 60.0), ("warp_tries", 4.0), ("warp_give_up_accel", 1.0), ("glide_check_a", 60.0), ("glide_check_b", 30.0),
            ("glide_near", 300.0), ("glide_stop_accel", 0.05), ("glide_arrive_accel", 0.1), ("glide_low", 70.0), ("glide_rise", -0.2), ("glide_stuck", 0.25),
            ("vanish_cut", 30.0), ("vanish_low", 30.0), ("vanish_rise", -0.4), ("vanish_rise_speed", 2.0), ("reappear_rest", 20.0), ("reappear_cut", 30.0),
            ("reappear_drift_delay", 10.0), ("reappear_drift", 0.5), ("reappear_drift_speed", 1.0), ("strike_rest", 10.0), ("strike_wait", 8.0), ("strike_fade", 6.0),
            ("strike_grow_time", 10.0), ("strike_grow_div", 5.0), ("strike_hold", 10.0), ("strike_cut", 36.0), ("strike_peak", 2.0), ("strike_tail", 8.0),
            ("core_x", -0.6), ("core_y", -96.0), ("core_z", -13.8), ("attack_blend", 8.0),
            ("walk_speed", 2.0), ("run_speed", 8.0), ("air_accel", 0.1), ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0), ("launch_min", 20.0),
            ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("down_time", 30.0),
            ("air_knock_hop", 20.0), ("weight", 1.0), ("gravity", crate::params::GRAVITY), ("fall_cap", 20.0),
        ];
        Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), NAMES).unwrap()
    }

    fn frames() -> Vec<(u16, f32)> {
        use req::*;
        vec![(IDLE, 240.0), (APPEAR, 172.0), (WARP, 100.0), (DASH_UP, 68.0), (DASH, 60.0), (DASH_END, 72.0), (BITE, 152.0), (NOTICE, 162.0), (FLINCH, 68.0), (KNOCK, 120.0)]
    }

    /// A flat floor at y = 0 (KH1 y down), Sora on it; the host plays motions to their length and
    /// queues, pauses the AI for actions (the bite), keeps polling it during steered attacks; it
    /// keeps the scale / visible / hittable state the AI asked for.
    struct Sim {
        ai: Ai,
        pos: Vec3,
        current: Option<u16>,
        played: f32,
        queued: Option<u16>,
        action: Option<u16>,
        log: Vec<(usize, Cmd)>,
        frame: usize,
        scale: f32,
        visible: bool,
        hittable: bool,
    }

    impl Sim {
        fn new(seed: u32, spawned: bool) -> Sim {
            Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: Vec3::new(0.0, -80.0, 0.0), current: None, played: 0.0, queued: None, action: None, log: Vec::new(), frame: 0, scale: 1.0, visible: true, hittable: true }
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
                        if self.action == Some(c) {
                            self.action = None;
                        }
                        if let Some(q) = self.queued.take() {
                            self.current = Some(q);
                            self.played = 0.0;
                        }
                    }
                }
                self.ai.set_floor(0.0);
                if self.action.is_none() {
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
                            ECmd::Action { request, then, .. } => {
                                self.current = Some(request);
                                self.played = 0.0;
                                self.queued = Some(then);
                                self.action = Some(request);
                            }
                            ECmd::Queue { request, .. } => self.queued = Some(request),
                            ECmd::Scale(k) => self.scale = k,
                            ECmd::Visible(v) => self.visible = v,
                            ECmd::Hittable(h) => self.hittable = h,
                            _ => {}
                        }
                        self.log.push((self.frame, c));
                    }
                }
                let c = self.ai.consts();
                let d = self.ai.body().step(1.0, self.pos.y, None, &c);
                self.pos += d;
                if self.pos.y > 0.0 {
                    self.pos.y = 0.0;
                }
            }
        }

        fn attacks(&self) -> Vec<(usize, Attack)> {
            self.log
                .iter()
                .filter_map(|(f, c)| match c {
                    ECmd::Attack { attack, .. } => Some((*f, *attack)),
                    ECmd::Action { attack: Some(a), .. } => Some((*f, *a)),
                    _ => None,
                })
                .collect()
        }
    }

    #[test]
    fn the_names_cover_the_numbers() {
        assert!(Table::new(vec![], NAMES).is_err());
        let c = consts();
        assert_eq!((c.num("far_range"), c.num("glide_speed"), c.num("armor_limit")), (600.0, 24.0, 6.0));
    }

    /// The battle spawn: KH1's appearance (69), 80 up (`sys 86`), growing from 0 to 1, unhittable
    /// until it has grown; then idle, and Sora within 10 m: engaged with the notice (205).
    #[test]
    fn appears_grows_and_engages() {
        let mut s = Sim::new(7, true);
        s.run(|_| Vec3::new(0.0, 0.0, 800.0), 3);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::Motion { request: req::APPEAR, .. })));
        assert!(!s.hittable && s.scale < 0.05, "scale {}", s.scale);
        assert!((s.pos.y + 160.0).abs() < 1.0, "80 up: {}", s.pos.y);
        assert!(s.ai.body().flying);
        s.run(|_| Vec3::new(0.0, 0.0, 800.0), 200);
        assert!(s.hittable && s.scale == 1.0);
        assert!(s.ai.risen());
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))));
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::Motion { request: req::NOTICE, .. })), "the notice");
    }

    /// Close (≤ 6 m): the bite (an action), then the warp: unhittable, shrinking, hidden, gliding,
    /// shown again for the drop (scale up to 2), hittable again at its end.
    #[test]
    fn close_bites_then_warps_and_drops() {
        let mut s = Sim::new(3, false);
        let sora = |_| Vec3::new(0.0, 0.0, 300.0);
        s.run(sora, 400);
        let at = s.attacks();
        assert_eq!(at.first().map(|a| a.1), Some(Attack::Bite), "{at:?}");
        s.run(sora, 1500);
        let at = s.attacks();
        assert!(at.iter().any(|a| a.1 == Attack::Drop), "{at:?}");
        let hidden = s.log.iter().position(|(_, c)| *c == ECmd::Visible(false)).expect("hidden");
        let shown = s.log.iter().position(|(_, c)| *c == ECmd::Visible(true)).expect("shown");
        let drop = s.log.iter().position(|(_, c)| matches!(c, ECmd::Attack { attack: Attack::Drop, .. })).unwrap();
        assert!(hidden < drop && drop < shown);
        let off = s.log.iter().position(|(_, c)| *c == ECmd::Hittable(false)).unwrap();
        let on = s.log.iter().skip(off).position(|(_, c)| *c == ECmd::Hittable(true)).unwrap() + off;
        assert!(off < hidden && on > shown, "unhittable through the warp");
        let peak = s.log[shown..on].iter().filter_map(|(_, c)| if let ECmd::Scale(k) = c { Some(*k) } else { None }).fold(0.0f32, f32::max);
        assert!((peak - 2.0).abs() < 0.01, "the drop grows to 2: {peak}");
        // The stats block's easing: 0.2 after a warp.
        assert_eq!(s.ai.body().accel, Some(0.2));
    }

    /// Far (> 6 m), at random: the warp, or the side approach and three dashes (201, 202 with its row,
    /// 203).
    #[test]
    fn far_warps_or_dashes() {
        let mut dashed = false;
        let mut warped = false;
        for i in 1..12u32 {
            let seed = i.wrapping_mul(0x9E37_79B9);
            let mut s = Sim::new(seed, false);
            s.run(|_| Vec3::new(0.0, 0.0, 900.0), 900);
            let at = s.attacks();
            if at.iter().any(|a| a.1 == Attack::Dash) {
                dashed = true;
                assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::Motion { request: req::DASH_UP, .. })));
            }
            if s.log.iter().any(|(_, c)| *c == ECmd::Visible(false)) {
                warped = true;
            }
        }
        assert!(dashed && warped);
    }

    /// The hit handler: the fifth light hit in a row marks the bite; the sixth takes no reaction and
    /// no state change; a knock-back resets the run; a clash asks for the hit state.
    #[test]
    fn light_hits_armour_up() {
        let mut s = Sim::new(1, false);
        s.run(|_| Vec3::new(0.0, 0.0, 300.0), 50);
        for i in 1..=5 {
            s.ai.set_hit_kind(3);
            s.ai.on_hit(HitNews::Damage);
            assert!(s.ai.keeps_reaction(), "hit {i}");
        }
        assert_eq!(s.ai.io.borrow().x.hits, 100);
        s.ai.set_hit_kind(3);
        s.ai.io.borrow_mut().state.1 = false;
        s.ai.on_hit(HitNews::Damage);
        assert!(!s.ai.keeps_reaction() && !s.ai.io.borrow().state.1, "the sixth: no reaction, no state");
        s.ai.set_hit_kind(4);
        s.ai.on_hit(HitNews::Damage);
        assert!(s.ai.keeps_reaction() && s.ai.io.borrow().x.light == 0);
        s.ai.on_hit(HitNews::Clash);
        assert!(s.ai.io.borrow().state.1);
        // The hit state with the bite mark: the bite at once.
        s.ai.io.borrow_mut().x.hits = 100;
        s.ai.on_hit(HitNews::Clash);
        s.log.clear();
        s.run(|_| Vec3::new(0.0, 0.0, 300.0), 250);
        assert_eq!(s.attacks().first().map(|a| a.1), Some(Attack::Bite), "{:?}", s.log.iter().take(10).collect::<Vec<_>>());
    }

    /// The quiet watchdog's reset mid-warp: shown, its scale, hittable, engaged.
    #[test]
    fn reset_restores_a_cut_warp() {
        let mut s = Sim::new(3, false);
        s.run(|_| Vec3::new(0.0, 0.0, 300.0), 400);
        // Into the warp.
        for _ in 0..600 {
            s.run(|_| Vec3::new(0.0, 0.0, 300.0), 1);
            if !s.visible {
                break;
            }
        }
        assert!(!s.visible && !s.hittable);
        s.ai.reset();
        s.run(|_| Vec3::new(0.0, 0.0, 300.0), 2);
        assert!(s.visible && s.hittable && s.scale == 1.0);
        assert!(!s.ai.io.borrow().x.warping);
        assert!(s.log.iter().rev().take(30).any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))));
    }

    /// No stall: Sora circling 2–9 m away for 20 000 frames (12 seeds), hit now and then: it keeps
    /// acting (an attack or a warp at least every 1500 frames; a warp whose glides miss Sora ends in
    /// the plain reappearance, `0x2A78`), never stays hidden past four glides, never goes under the
    /// floor, and uses all three attacks.
    #[test]
    fn a_long_fight_never_stalls() {
        for i in 1u32..=12 {
            let seed = i.wrapping_mul(0x9E37_79B9) ^ 0x1234_5678;
            let mut s = Sim::new(seed, true);
            let sora = |f: usize| {
                let a = f as f32 * 0.002;
                let r = 550.0 + 350.0 * (f as f32 * 0.0007).sin();
                Vec3::new(r * a.cos(), 0.0, r * a.sin())
            };
            let mut low = f32::INFINITY;
            let mut hidden_for = 0;
            let mut worst_hidden = 0;
            let mut next_hit = 300;
            for f in 0..20_000usize {
                s.run(sora, 1);
                low = low.min(-s.pos.y);
                if !s.visible {
                    hidden_for += 1;
                    worst_hidden = worst_hidden.max(hidden_for);
                } else {
                    hidden_for = 0;
                }
                if f == next_hit {
                    next_hit += 150 + (f * 7919 + seed as usize * 104_729) % 250;
                    if s.hittable && s.action.is_none() {
                        s.ai.set_hit_kind(if f % 5 == 0 { 4 } else { 3 });
                        s.ai.on_hit(HitNews::Damage);
                    }
                }
            }
            assert!(low >= 0.0, "seed {seed}: never under the floor");
            let at = s.attacks();
            assert!(at.len() > 10, "seed {seed}: {} attacks", at.len());
            let mut acts: Vec<usize> = at.iter().map(|a| a.0).chain(s.log.iter().filter(|(_, c)| *c == ECmd::Visible(false)).map(|(f, _)| *f)).collect();
            acts.sort();
            let mut last = 0;
            for &f in &acts {
                assert!(f - last < 1500, "seed {seed}: nothing from {last} to {f}");
                last = f;
            }
            assert!(20_000 - last < 1500, "seed {seed}: nothing after {last}");
            assert!(worst_hidden < 4 * 700, "seed {seed}: hidden {worst_hidden} frames in a row");
            assert!(s.ai.body().flying, "seed {seed}");
            let kinds: std::collections::HashSet<_> = at.iter().map(|a| a.1).collect();
            assert_eq!(kinds.len(), 3, "seed {seed}: {kinds:?}");
        }
    }
}
