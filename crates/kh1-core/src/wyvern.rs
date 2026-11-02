//! KH1 FM's Wyvern (`xa_ex_2320.mdls` part 5, `ex_2320.bd`), engine-agnostic: its AI script on the shared
//! enemy framework (`crate::enemy`), flying (KH1's movement state 4, `crate::enemy::Body::flying`). M11
//! (`docs/recom-m11-enemies.md` § The Wyvern). Re:CoM runs it natively: each part is the step of a Re:CoM
//! enemy state (`kh1-recom`'s `win::kh1_enemy`).
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2320.bd` at checked
//! offsets). Script offsets are the player's `ex_2320.bd`'s (Checked (files): the script disassembled with
//! `tools/kh1-ai-script.py`, raw and folded; the syscalls named are Checked (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x21CC`: init `0x2362` (KH1's world: `wp+0x54` = world 11, Olympus; stats records LV1 `0x14` /
//!   LV2 `0x80`; the stats block's hop 800 (unused) and its flag 2 (not identified: not ported); the work
//!   area; home; the state object `wp+0x20`; the search table `0x350`; the handlers: hit `0x2484`), flight
//!   with no velocity (`sys 135` with (0, 0, 0, 1)), a yield; placed (`sys 132` = 0): the target and the
//!   engaged state; spawned: the appearance `0xC3C` → `0xC4C`. Then each frame: while idle (state 1) a
//!   target within 1500 (`0x1930`; world 11: 10000) → engaged; a state request restarts the behaviour (1 idle
//!   `0xB4E` and 3 lost `0x5D2` with the target cleared, 2 engaged `0x7BE` with the target or Sora, 4 hit
//!   `0x3C4` with the hitter or Sora). The script never sets a turn rate at init: the stats block's π/16
//!   (11.25°, the value every routine puts back).
//! - the hit handler `0x2484`: a hit of attribute 2 / 3 / 4 gets the engine's own reaction and nothing more
//!   (`sys 42` → `0x1402B7E60`: KH1's element reactions, left out (owner 2026-11-02): the host carries no
//!   attribute), 9 / 11 nothing more; the turn rate back to 11.25; while the dive 202 plays: every hit
//!   counts, a hit that is not a clash (kinds 6 / 7) nor of attribute 8 / 9 takes no reaction (its kind
//!   zeroed) and asks for the dive-hit motion 214; otherwise every hit but a clash counts; all ask for the
//!   hit state 4.
//!
//! The work area: `wp+0x48` hits, `+0x4C` noticed, `+0x50` hit in the dive, `+0x54` world 11, `+0x5C`
//! attack rounds.
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash (`0x26D6` → `0x163A`: removed 18 m / 12.6 m
//! out of sight from Sora when spawned; in a Re:CoM battle room it would never be defeated), the party
//! retargeting (`0x18D6`, `0x1930`: Sora is the only target), the hits' KH1 attribute (`hp[0xC] >> 6 & 15`),
//! the effects (`sys 75` 2), the fade-in and flag 0x80 (`0x208A`, `0x200E`: [`ECmd::FadeIn`] for the log
//! only), the death routine (`0x26B8`: Re:CoM's defeat), `sys1.0` (a debug print).

use crate::enemy::script::{fly, frame, height, motion_frames, poll, pos, push, rand, rand_range, set_air_accel, set_state, set_target, set_turn_rate, steer, take_state, target, tick_clock};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Wyvern's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The appearance (`0xC6E`), an action, 60 frames.
    pub const APPEAR: u16 = 69;
    /// The bite (`0x13D8`): a plain request with rows `0xEC` / `0x15C` (the two feet), 120 frames, hit
    /// [20, 84).
    pub const BITE: u16 = 200;
    /// Before the dive from above (`0x113E`), 140 frames.
    pub const DIVE_START: u16 = 201;
    /// The dive (`0x113E`): row `0x23C`, 120 frames looped, hit all along. Hits take no reaction.
    pub const DIVE: u16 = 202;
    /// The dive's landing (`0x113E`): row `0x2AC`, 100 frames, hit [0, 36).
    pub const LAND: u16 = 203;
    /// The charge's wind-up (`0xD70` / `0xF52`), 108 frames, root.
    pub const WINDUP: u16 = 204;
    /// The charge (`0xD70` / `0xF52`): row `0x1CC`, 120 frames looped, hit all along.
    pub const CHARGE: u16 = 205;
    /// The charge's end: row `0x1CC` again, 106 frames, hit [0, 24).
    pub const CHARGE_END: u16 = 206;
    /// Before a chase (`0x2ADA`), 90 frames.
    pub const CHASE_START: u16 = 207;
    /// Flying (`0x2ADA`, `0x28A4`), 260 frames looped.
    pub const FLY: u16 = 208;
    /// Banking (`0x2ADA`) and circling (`0x26EA`): 209 with the facing turned −90°, 210 with +90°.
    pub const BANK_L: u16 = 209;
    pub const BANK_R: u16 = 210;
    /// The halt (`0x26EA`, `0x28A4`), 130 frames.
    pub const HALT: u16 = 211;
    /// Hovering up or down (`0x30C2`, `0x2F5C`), 200 frames looped.
    pub const HOVER: u16 = 212;
    /// The roar before an attack (`0xAF2`), an action, 70 frames, root.
    pub const ROAR: u16 = 213;
    /// Hit out of the dive (`0x3C4`), 92 frames.
    pub const DIVE_HIT: u16 = 214;
    /// The notice (`0xCCC`), 204 frames.
    pub const NOTICE: u16 = 215;
}

/// The Wyvern's attacks, in the order the host indexes them (the manifest's `attacks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    /// `0x13D8`: the bite, 200 with rows `0xEC` / `0x15C` (`sys 23` mode 2: they live while 200 plays): the
    /// script keeps steering it.
    Bite,
    /// `0x113E`: the dive, 202 with row `0x23C`, steered straight down.
    Dive,
    /// `0x113E`: the landing, 203 with row `0x2AC`.
    Landing,
    /// `0xD70` / `0xF52`: the charge, 205 with row `0x1CC`, steered along the facing.
    Charge,
    /// `0xD70` / `0xF52`: the charge's end, 206 with row `0x1CC` (made again with the same hit bit).
    ChargeEnd,
}

impl Attack {
    pub const ALL: [Attack; 5] = [Attack::Bite, Attack::Dive, Attack::Landing, Attack::Charge, Attack::ChargeEnd];

    pub fn request(self) -> u16 {
        match self {
            Attack::Bite => req::BITE,
            Attack::Dive => req::DIVE,
            Attack::Landing => req::LAND,
            Attack::Charge => req::CHARGE,
            Attack::ChargeEnd => req::CHARGE_END,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Bite => "bite",
            Attack::Dive => "dive",
            Attack::Landing => "landing",
            Attack::Charge => "charge",
            Attack::ChargeEnd => "charge end",
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The names of the Wyvern's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    // Supervisor `0x21CC`, search `0x1930`.
    "search_range",
    "world11_search",
    // Blends and the default rates (the values the routines put back).
    "motion_blend",
    "turn_default",
    "ease_default",
    // Appearance `0xC6E`.
    "appear_fade",
    "appear_blend",
    "appear_then_blend",
    // Hit `0x3C4`.
    "dive_hit_blend",
    "hit_close",
    "hit_mid",
    "hits_lo",
    "hits_hi",
    "idle_pause",
    // The low test (`0x3C4`, `0x5D2`, `0x904`, `0xB4E`) and the rise `0x30C2`.
    "low_height",
    "low_above",
    "rise_up",
    "rise_speed",
    "rise_time",
    "rise_check",
    "rise_turn_rate",
    "rise_dir",
    "hover_blend",
    // Lost `0x5D2`, idle `0xB4E`, circle `0x26EA`.
    "lost_search",
    "lost_rest_blend",
    "idle_rest_blend",
    "idle_turn_rate",
    "rest_lo",
    "rest_hi",
    "circle_time_lo",
    "circle_time_hi",
    "circle_turn_lo",
    "circle_turn_hi",
    "circle_turn_rate",
    "circle_chance",
    "circle_left",
    "circle_right",
    "circle_flip",
    "circle_blend",
    "circle_wait",
    "circle_half",
    "circle_speed",
    "halt_ease",
    "halt_tail",
    // Engaged `0x7BE`, notice `0xCCC`, roar `0xAF2`.
    "round_limit",
    "engage_far",
    "far_chance",
    "close_rounds",
    "notice_turn_rate",
    "notice_then_blend",
    "notice_extra",
    "roar_blend",
    "roar_then_blend",
    // The far attack `0x904`, the dive attack `0xA8C`.
    "approach_face_rate",
    "chase_speed",
    "chase_arrive_far",
    "chase_arrive_dive",
    "chase_time",
    "flyby_ahead",
    "flyby_up",
    "flyby_speed",
    "flyby_arrive",
    "flyby_time",
    "flyby_chance",
    "dive_above",
    // The charges `0xD70` / `0xF52`.
    "hit_charge_face_rate",
    "charge_face_rate",
    "hit_windup_blend",
    "windup_blend",
    "windup_wait",
    "windup_short",
    "windup_turn_rate",
    "charge_blend",
    "charge_time",
    "charge_sink",
    "charge_speed",
    "charge_then_blend",
    "charge_extra",
    // The dive `0x113E`.
    "bomb_above",
    "bomb_queue_blend",
    "bomb_flat",
    "bomb_creep",
    "bomb_floor",
    "bomb_down",
    "bomb_speed",
    "bomb_stuck",
    "land_floor",
    "land_down",
    "land_speed",
    // The bite `0x13D8`.
    "bite_turn_rate",
    "bite_blend_flying",
    "bite_wait",
    "bite_tail",
    "bite_tail_speed",
    "bite_speed",
    "bite_above",
    "bite_sink",
    "bite_end_blend",
    // Fly to `0x28A4`, the chase `0x2ADA`.
    "stuck_after",
    "stuck_factor",
    "fly_halt_ease",
    "fly_idle_blend",
    "fly_level",
    "chase_queue_blend",
    "chase_near",
    "chase_stop_ease",
    "bank_turn",
    "chase_turn_rate",
    "bank_cos",
    "bank_blend",
    "bank_hold",
    "bank_face",
    "chase_floor",
    // Descend `0x2F5C`.
    "descend_face_rate",
    "descend_idle_blend",
    "descend_idle_start",
    "descend_wait",
    "descend_down",
    "descend_speed",
    "descend_stuck",
    // Turn `0x322A`.
    "turn_time",
    "turn_cos",
];

/// The Wyvern's numbers.
pub type Consts = Table;

/// The state objects (`wp+0x20`): the supervisor's requests.
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const LOST: u8 = 3;
    pub const HIT: u8 = 4;
}

/// The Wyvern's own work area: the actor's steering direction `+0x20` (a script writes it, all or only its
/// `y`, and leaves it; only the speed `+0x2C` is zeroed each frame), the counters and flags of the module
/// notes, the appearance, Sora on the ground (`*(target + 0x70)`, his movement state), the last hit's kind
/// and whether it keeps its reaction.
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    dir: Vec3,
    hits: i32,
    rounds: i32,
    noticed: bool,
    dive_hit: bool,
    world11: bool,
    spawned: bool,
    appeared: bool,
    target_ground: bool,
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

/// `func_1B80` (`sys 4(0)`): party 0, Sora.
fn sora(cx: &Cx) -> Vec3 {
    let io = cx.borrow();
    io.w.target.unwrap_or(io.w.pos)
}

/// The actor's target (`+0x74`): Sora once set (Re:CoM's battles have no other party member).
fn tgt(cx: &Cx) -> Vec3 {
    target(cx).unwrap_or_else(|| sora(cx))
}

/// `func_1D1E` (`sys 6`, 3D): the distance from the actor to `p`.
fn dist_to(cx: &Cx, p: Vec3) -> f32 {
    (p - me(cx)).length()
}

/// `p` at the actor's height minus the actor: the flat line to it (the scripts' `y = actor.y` copies).
fn flat_to(cx: &Cx, p: Vec3) -> Vec3 {
    let m = me(cx);
    Vec3::new(p.x - m.x, 0.0, p.z - m.z)
}

/// Sora on the ground: his movement state `*(target + 0x70)` is 0 (`0x2B60`; Checked (decomp):
/// `0x1402B1960` keeps it at `+0x70`). The host gives it ([`Behaviour::set_target_grounded`]).
fn target_ground(cx: &Cx) -> bool {
    cx.borrow().x.target_ground
}

/// `sys 9`: the vector normalized (its length 0 stays 0).
fn norm(v: Vec3) -> Vec3 {
    if v.length() > 0.0 {
        v.normalized()
    } else {
        v
    }
}

/// `sys 100` (`0x1402B9D10` → `0x1402A8C70`): the cosine between two vectors, 0 when either is zero,
/// clamped to [−1, 1]. Checked (decomp).
fn cosine(a: Vec3, b: Vec3) -> f32 {
    let (la, lb) = (a.length(), b.length());
    if la == 0.0 || lb == 0.0 {
        return 0.0;
    }
    (a.dot(b) / (lb * la)).clamp(-1.0, 1.0)
}

/// `sys 10(v, deg)` on a direction: turned about the vertical (the Air Soldier's, the Darkball's and the
/// Air Pirate's convention: the yaw + deg; Assumption, as theirs).
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

/// `func_1F1E` / `sys 36`: the target yaw toward `p` (the engine turns).
fn yaw_to(cx: &Cx, p: Vec3) {
    let d = p - me(cx);
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `sys 36` along a vector.
fn yaw_along(cx: &Cx, d: Vec3) {
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `func_1F48` / `sys 11`: face `p` now (`+0x150` and the facing).
fn face_now(cx: &Cx, p: Vec3) {
    let d = p - me(cx);
    let y = d.x.atan2(d.z);
    let mut io = cx.borrow_mut();
    io.body.yaw = y;
    io.body.yaw_target = y;
    io.out.push(ECmd::Yaw(y));
}

/// `sys 12` / `sys 13`: a motion request; `sys 172`'s clock restarts for a request that plays anew. A
/// request of the motion already playing, with no other motion asked for this frame, changes nothing
/// ([`ECmd::Motion`]'s rule, `0x14029F7C0`'s early return): it is not sent (the script asks for its
/// flight motions every frame).
fn play(cx: &Cx, r: u16, blend: f32) {
    {
        let mut io = cx.borrow_mut();
        let asked = io.out.iter().any(|c| matches!(c, ECmd::Motion { .. } | ECmd::MotionAt { .. } | ECmd::Attack { .. } | ECmd::Action { .. }));
        if io.w.motion == Some(r) && !asked {
            return;
        }
        if io.w.motion != Some(r) && io.clock.0 != Some(r) {
            io.clock = (Some(r), 0.0);
        }
    }
    push(cx, ECmd::Motion { request: r, blend });
}

/// `sys 14`: the motion queued after the current one.
fn queue(cx: &Cx, r: u16, blend: f32) {
    push(cx, ECmd::Queue { request: r, blend });
}

/// A plain request made with its attack rows (`sys 23`, mode 2, then or before `sys 12` / `sys 13`).
fn attack(cx: &Cx, a: Attack, blend: f32) {
    cx.borrow_mut().clock = (Some(a.request()), 0.0);
    push(cx, ECmd::Attack { request: a.request(), attack: a, blend });
}

/// `func_32F2`: wait `n` frames.
async fn wait(cx: &Cx, n: f32) {
    crate::enemy::script::wait_frames(cx, n).await;
}

/// Whether last frame's displacement (`sys 118`: `+0x10 − +0x490`) is under `speed` a frame (the stuck
/// tests).
fn stuck(cx: &Cx, speed: f32) -> bool {
    let io = cx.borrow();
    io.w.moved - speed * io.w.step < 0.0
}

/// A target within `range` (`func_1930` → `sys 48`): Sora (Assumption: KH1's search finds Sora when he is
/// within the range; `func_1930`'s fallbacks also give him within it; `sys 139`'s out-of-sight test is taken
/// as in sight). In KH1's world 11 the range is `world11_search` and Sora is always found (`0x1962`,
/// `0x19E6`).
fn search(cx: &Cx, range: f32) -> bool {
    if cx.borrow().x.world11 {
        return cx.borrow().w.target.is_some();
    }
    cx.borrow().w.target.is_some() && dist_to(cx, sora(cx)) - range < 0.0
}

/// The low test before a climb (`0x52A`, `0x90A`: under `low_height` above the floor, or `0x552`, `0x622`,
/// `0x702`, `0x932`, `0xB6C`: less than `low_above` above `p`).
fn low(cx: &Cx, p: Vec3) -> bool {
    me(cx).y - (p.y - k(cx, "low_above")) > 0.0
}

/// How the routines end: `func_1C3E` + `misc8` ends the behaviour thread from any depth (a state request
/// is pending; the supervisor restarts the behaviour next frame).
type Flow = bool;
const END: Flow = true;
const ON: Flow = false;

/// `func_322A(p, rate)`: turn to face `p` (flat) at `rate` degrees a frame playing the idle, until the
/// facing's cosine to it is `turn_cos` or `turn_time` frames have passed; the turn rate back to the default.
async fn turn_to(cx: &Cx, p: impl Fn(&Cx) -> Vec3, rate: f32) {
    let mut t = k(cx, "turn_time");
    set_turn_rate(cx, rate);
    play(cx, req::IDLE, k(cx, "motion_blend"));
    while t > 0.0 {
        let g = flat_to(cx, p(cx));
        if cosine(facing(cx), g) - k(cx, "turn_cos") >= 0.0 {
            break;
        }
        yaw_along(cx, g);
        t -= step(cx);
        frame().await;
    }
    set_turn_rate(cx, k(cx, "turn_default"));
}

/// `func_30C2(target, from, speed, dy)`: rise `dy` (negative: up) from `from`'s height playing 212, facing
/// the target, `rise_time` frames at most; stuck after `rise_check` frames: stop.
async fn rise(cx: &Cx, from: Vec3, speed: f32, dy: f32) {
    let mut t = k(cx, "rise_time");
    let check = t - k(cx, "rise_check");
    yaw_to(cx, tgt(cx));
    play(cx, req::HOVER, k(cx, "hover_blend"));
    while t > 0.0 {
        set_turn_rate(cx, k(cx, "rise_turn_rate"));
        if (me(cx).y - from.y) - dy > 0.0 {
            set_dir(cx, Vec3::new(0.0, k(cx, "rise_dir"), 0.0));
            go(cx, speed);
            yaw_to(cx, tgt(cx));
            t -= step(cx);
            frame().await;
            if t - check < 0.0 && stuck(cx, speed * k(cx, "stuck_factor")) {
                halt(cx);
                break;
            }
        } else {
            halt(cx);
            frame().await;
            break;
        }
    }
    set_turn_rate(cx, k(cx, "turn_default"));
}

/// The rise the routines take when low (`0x3C4`, `0x5D2`, `0x904`, `0xB4E`): `rise_up` from here at
/// `rise_speed`.
async fn rise_here(cx: &Cx) {
    let from = me(cx);
    rise(cx, from, k(cx, "rise_speed"), k(cx, "rise_up")).await;
}

/// `func_26EA(time, turn)`: the circle: the facing turned 90° to a side (at random: 209 with −90°, 210 with
/// +90° and the turn negated), turned once more by the turn, `circle_wait` frames into the bank motion, then
/// while the time lasts flying along the direction at `circle_speed` while it turns by half the turn a
/// frame, facing it; the halt (211) easing off at `halt_ease` until `halt_tail` frames are left of it.
async fn circle(cx: &Cx, time: f32, turn: f32) {
    let mut time = time;
    let mut turn = turn;
    set_turn_rate(cx, k(cx, "circle_turn_rate"));
    let mut f = facing(cx);
    let side = if rand(cx, "circle side") - k(cx, "circle_chance") < 0.0 {
        f = rotate(f, k(cx, "circle_left"));
        f = rotate(f, turn);
        req::BANK_L
    } else {
        f = rotate(f, k(cx, "circle_right"));
        turn *= k(cx, "circle_flip");
        f = rotate(f, turn);
        req::BANK_R
    };
    yaw_along(cx, f);
    play(cx, side, k(cx, "circle_blend"));
    wait(cx, k(cx, "circle_wait")).await;
    while time >= 0.0 {
        f = rotate(f, turn * k(cx, "circle_half") * step(cx));
        yaw_along(cx, f);
        set_dir(cx, f);
        go(cx, k(cx, "circle_speed"));
        time -= step(cx);
        frame().await;
    }
    play(cx, req::HALT, k(cx, "motion_blend"));
    let mut t = motion_frames(cx, req::HALT);
    set_air_accel(cx, k(cx, "halt_ease"));
    while t - k(cx, "halt_tail") > 0.0 {
        t -= step(cx);
        frame().await;
    }
    halt(cx);
    set_air_accel(cx, k(cx, "ease_default"));
    set_turn_rate(cx, k(cx, "turn_default"));
}

/// The two random draws of a circle (`func_1CC4(250, 500)` then `func_1CC4(3, 5)`, the time first).
fn circle_draws(cx: &Cx) -> (f32, f32) {
    let time = rand_range(cx, k(cx, "circle_time_lo"), k(cx, "circle_time_hi"), "circle time");
    let turn = rand_range(cx, k(cx, "circle_turn_lo"), k(cx, "circle_turn_hi"), "circle turn");
    (time, turn)
}

/// The halt the flights end with (`0x28A4`'s two copies): 211, easing off at `fly_halt_ease` until
/// `halt_tail` frames are left, the idle.
async fn fly_halt(cx: &Cx) {
    play(cx, req::HALT, k(cx, "motion_blend"));
    let mut t = motion_frames(cx, req::HALT);
    set_air_accel(cx, k(cx, "fly_halt_ease"));
    while t - k(cx, "halt_tail") > 0.0 {
        t -= step(cx);
        frame().await;
    }
    halt(cx);
    play(cx, req::IDLE, k(cx, "fly_idle_blend"));
    set_air_accel(cx, k(cx, "ease_default"));
}

/// `func_28A4(p, speed, arrive, time)`: fly to `p` (208), its vertical part × `fly_level`, until within
/// `arrive` or the time is out; stuck after `stuck_after` frames: the halt.
async fn fly_to(cx: &Cx, p: Vec3, speed: f32, arrive: f32, time: f32) {
    let mut t = time;
    let check = t - k(cx, "stuck_after");
    while t > 0.0 {
        play(cx, req::FLY, k(cx, "motion_blend"));
        let v = p - me(cx);
        if v.length() - arrive < 0.0 {
            fly_halt(cx).await;
            return;
        }
        let mut d = norm(v);
        d.y *= k(cx, "fly_level");
        set_dir(cx, d);
        go(cx, speed);
        frame().await;
        if t - check < 0.0 && stuck(cx, speed * k(cx, "stuck_factor")) {
            fly_halt(cx).await;
            return;
        }
        t -= step(cx);
    }
}

/// `func_2ADA(target, p, speed, arrive, time)`: the chase: 207, then 208 queued; toward `p` (its x / z the
/// target's while the target stands), facing the target, banking (209 / 210) while it is off to a side
/// after the first `stuck_after` frames, `time` frames at most; within `chase_near` of the target: done;
/// within `arrive` of the point: done once level, else stopped at once (easing `chase_stop_ease`) until the
/// stuck test ends it; stuck after `stuck_after` frames: done. The default rates at the end.
async fn chase(cx: &Cx, p: Vec3, speed: f32, arrive: f32, time: f32) {
    let mut t = time;
    let check = t - k(cx, "stuck_after");
    let mut bank = 0.0;
    play(cx, req::CHASE_START, k(cx, "motion_blend"));
    queue(cx, req::FLY, k(cx, "chase_queue_blend"));
    while t > 0.0 {
        let tg = tgt(cx);
        let mut v = p;
        if target_ground(cx) {
            v.x = tg.x;
            v.z = tg.z;
        }
        v = v - me(cx);
        if dist_to(cx, tg) - k(cx, "chase_near") < 0.0 {
            halt(cx);
            break;
        }
        if v.length() - arrive < 0.0 {
            if v.y.abs() - speed * step(cx) < 0.0 {
                halt(cx);
                break;
            }
            set_air_accel(cx, k(cx, "chase_stop_ease"));
            v = Vec3::ZERO;
        } else {
            let f = facing(cx);
            let g = flat_to(cx, tg);
            let c1 = cosine(f, g);
            let c2 = cosine(rotate(f, k(cx, "bank_turn")), g);
            set_turn_rate(cx, k(cx, "chase_turn_rate"));
            if t - check < 0.0 {
                let blend = k(cx, "bank_blend");
                if c2 > 0.0 && c1 - k(cx, "bank_cos") <= 0.0 {
                    bank = 0.0;
                    yaw_to(cx, tg);
                    play(cx, req::BANK_R, blend);
                } else if c2 < 0.0 && c1 - k(cx, "bank_cos") <= 0.0 {
                    bank = 0.0;
                    yaw_to(cx, tg);
                    play(cx, req::BANK_L, blend);
                } else if bank - k(cx, "bank_hold") > 0.0 {
                    play(cx, req::FLY, blend);
                    if bank - k(cx, "bank_face") < 0.0 {
                        yaw_to(cx, tg);
                    }
                } else {
                    yaw_to(cx, tg);
                }
            } else {
                yaw_to(cx, tg);
            }
            if v.y > 0.0 {
                if me(cx).y - p.y > 0.0 {
                    v.y = 0.0;
                } else if height(cx) - k(cx, "chase_floor") > 0.0 {
                    // `v × speed` before the normalization (no effect; as the script writes it).
                    v = v * speed;
                }
            }
        }
        set_dir(cx, norm(v));
        go(cx, speed);
        frame().await;
        if t - check < 0.0 && stuck(cx, speed * k(cx, "stuck_factor")) {
            break;
        }
        let s = step(cx);
        t -= s;
        bank += s;
    }
    set_turn_rate(cx, k(cx, "turn_default"));
    set_air_accel(cx, k(cx, "ease_default"));
}

/// `func_2F5C(target)`: down to the target's height (as it was) playing 212 at `descend_speed`, facing the
/// target, after turning to it; at its height or the floor: the idle from frame `descend_idle_start` and
/// `descend_wait` frames; stuck: stop.
async fn descend(cx: &Cx) {
    let q = tgt(cx);
    turn_to(cx, tgt, k(cx, "descend_face_rate")).await;
    play(cx, req::HOVER, k(cx, "hover_blend"));
    loop {
        if me(cx).y - q.y >= 0.0 || height(cx) <= 0.0 {
            halt(cx);
            push(cx, ECmd::MotionAt { request: req::IDLE, blend: k(cx, "descend_idle_blend"), start: k(cx, "descend_idle_start") });
            wait(cx, k(cx, "descend_wait")).await;
            return;
        }
        set_dir(cx, Vec3::new(0.0, k(cx, "descend_down"), 0.0));
        go(cx, k(cx, "descend_speed"));
        yaw_to(cx, tgt(cx));
        frame().await;
        if stuck(cx, k(cx, "descend_stuck")) {
            halt(cx);
            return;
        }
    }
}

/// `func_13D8`: the bite: facing the target, turn rate `bite_turn_rate`; 200 with its rows (blend 0 out of
/// the fly motion 208, else `motion_blend` and `bite_wait` frames); for its length along the facing at
/// `bite_speed` (sinking by `bite_sink` while more than `bite_above` over the target), the last `bite_tail`
/// frames at `bite_tail_speed`; then 208, the default turn rate.
async fn bite(cx: &Cx) {
    yaw_to(cx, tgt(cx));
    set_turn_rate(cx, k(cx, "bite_turn_rate"));
    if cx.borrow().w.motion == Some(req::FLY) {
        attack(cx, Attack::Bite, k(cx, "bite_blend_flying"));
    } else {
        attack(cx, Attack::Bite, k(cx, "motion_blend"));
        wait(cx, k(cx, "bite_wait")).await;
    }
    yaw_to(cx, tgt(cx));
    let mut t = motion_frames(cx, req::BITE);
    while t > 0.0 {
        set_dir(cx, facing(cx));
        if t - k(cx, "bite_tail") <= 0.0 {
            go(cx, k(cx, "bite_tail_speed"));
        } else {
            if me(cx).y - (tgt(cx).y - k(cx, "bite_above")) < 0.0 {
                dir_y(cx, k(cx, "bite_sink"));
            }
            go(cx, k(cx, "bite_speed"));
        }
        t -= step(cx);
        frame().await;
    }
    play(cx, req::FLY, k(cx, "bite_end_blend"));
    set_turn_rate(cx, k(cx, "turn_default"));
}

/// `func_113E(target)`: the dive: 201 (202 queued) for its length facing the target, creeping toward it at
/// `bomb_creep` while farther than `bomb_flat` (flat); 202 with its row straight down at `bomb_speed`
/// facing the target until `bomb_above` over it (as it was), `bomb_floor` over the floor or stuck; then
/// 203 with its row, sinking at `land_speed` until `land_floor` over the floor, for its length; the idle.
async fn dive(cx: &Cx) {
    let mut p = tgt(cx);
    p.y -= k(cx, "bomb_above");
    play(cx, req::DIVE_START, k(cx, "motion_blend"));
    queue(cx, req::DIVE, k(cx, "bomb_queue_blend"));
    let mut t = motion_frames(cx, req::DIVE_START);
    while t > 0.0 {
        yaw_to(cx, tgt(cx));
        let g = flat_to(cx, tgt(cx));
        if g.length() - k(cx, "bomb_flat") > 0.0 {
            set_dir(cx, norm(g));
            go(cx, k(cx, "bomb_creep"));
        }
        t -= step(cx);
        frame().await;
    }
    attack(cx, Attack::Dive, k(cx, "motion_blend"));
    loop {
        if me(cx).y - p.y > 0.0 || height(cx) - k(cx, "bomb_floor") <= 0.0 {
            break;
        }
        yaw_to(cx, tgt(cx));
        set_dir(cx, Vec3::new(0.0, k(cx, "bomb_down"), 0.0));
        go(cx, k(cx, "bomb_speed"));
        frame().await;
        if stuck(cx, k(cx, "bomb_stuck")) {
            break;
        }
    }
    attack(cx, Attack::Landing, k(cx, "motion_blend"));
    let mut t = motion_frames(cx, req::LAND);
    while t > 0.0 {
        if height(cx) - k(cx, "land_floor") <= 0.0 {
            halt(cx);
            break;
        }
        dir_y(cx, k(cx, "land_down"));
        go(cx, k(cx, "land_speed"));
        t -= step(cx);
        frame().await;
    }
    wait(cx, t).await;
    play(cx, req::IDLE, k(cx, "motion_blend"));
}

/// `func_D70` (out of a hit) and `func_F52` (engaged): the charge: turn to the target (`face_rate`), the
/// wind-up 204 (its blend; `func_F52` waits `windup_wait` frames first) for its length − `windup_short`
/// turning to the target at `windup_turn_rate`; 205 with its row for `charge_time` frames along the facing at
/// `charge_speed` (sinking by `charge_sink` while above the target); 206 with the row again; the idle when
/// 206 has played (KH1's queue: the host gives a steered attack no queue), `charge_extra` frames more; a yield.
async fn charge(cx: &Cx, face_rate: f32, windup_blend: f32, windup_wait: f32) {
    turn_to(cx, tgt, face_rate).await;
    play(cx, req::WINDUP, windup_blend);
    if windup_wait > 0.0 {
        wait(cx, windup_wait).await;
    }
    let mut t = motion_frames(cx, req::WINDUP) - k(cx, "windup_short");
    while t > 0.0 {
        set_turn_rate(cx, k(cx, "windup_turn_rate"));
        yaw_to(cx, tgt(cx));
        t -= step(cx);
        frame().await;
    }
    set_turn_rate(cx, k(cx, "turn_default"));
    attack(cx, Attack::Charge, k(cx, "charge_blend"));
    let mut t = k(cx, "charge_time");
    let mut first = true;
    while t > 0.0 {
        if !first {
            // `sys 12(205, 0)` each frame: it plays (nothing new; the first one is the request itself).
            play(cx, req::CHARGE, k(cx, "charge_blend"));
        }
        first = false;
        set_dir(cx, norm(facing(cx)));
        if me(cx).y - tgt(cx).y < 0.0 {
            dir_y(cx, k(cx, "charge_sink"));
        }
        go(cx, k(cx, "charge_speed"));
        t -= step(cx);
        frame().await;
    }
    halt(cx);
    attack(cx, Attack::ChargeEnd, k(cx, "motion_blend"));
    let n = motion_frames(cx, req::CHARGE_END);
    wait(cx, n).await;
    play(cx, req::IDLE, k(cx, "charge_then_blend"));
    wait(cx, k(cx, "charge_extra")).await;
    frame().await;
}

/// `func_AF2`: face the target; the roar 213 as an action (`0x10000D5`), the idle queued (bit 30); a yield.
async fn roar(cx: &Cx) {
    yaw_to(cx, tgt(cx));
    push(cx, ECmd::Action { request: req::ROAR, attack: None, then: req::IDLE, blend: k(cx, "roar_blend") });
    queue(cx, req::IDLE, k(cx, "roar_then_blend"));
    frame().await;
}

/// `func_A8C`: the dive attack: the chase to `dive_above` over the target (arriving within
/// `chase_arrive_dive`), the dive, a round.
async fn dive_attack(cx: &Cx) {
    let mut p = tgt(cx);
    p.y -= k(cx, "dive_above");
    chase(cx, p, k(cx, "chase_speed"), k(cx, "chase_arrive_dive"), k(cx, "chase_time")).await;
    dive(cx).await;
    cx.borrow_mut().x.rounds += 1;
}

/// `func_904`: the far attack: a rise when low (under `low_height` over the floor or under `low_above` over
/// the target); turn to the target; the chase at it; the bite; a round; a yield; a fly-by to a point
/// `flyby_ahead` ahead and `flyby_up` up; a yield; down to the target's height; then (`flyby_chance`) the
/// charge and a round, or the dive attack.
async fn far_attack(cx: &Cx) {
    if height(cx) - k(cx, "low_height") < 0.0 || low(cx, tgt(cx)) {
        rise_here(cx).await;
    }
    turn_to(cx, tgt, k(cx, "approach_face_rate")).await;
    let p = tgt(cx);
    chase(cx, p, k(cx, "chase_speed"), k(cx, "chase_arrive_far"), k(cx, "chase_time")).await;
    bite(cx).await;
    cx.borrow_mut().x.rounds += 1;
    frame().await;
    let mut q = norm(facing(cx)) * k(cx, "flyby_ahead") + me(cx);
    q.y -= k(cx, "flyby_up");
    play(cx, req::FLY, k(cx, "motion_blend"));
    fly_to(cx, q, k(cx, "flyby_speed"), k(cx, "flyby_arrive"), k(cx, "flyby_time")).await;
    frame().await;
    descend(cx).await;
    if rand(cx, "far follow-up") - k(cx, "flyby_chance") > 0.0 {
        charge(cx, k(cx, "charge_face_rate"), k(cx, "windup_blend"), k(cx, "windup_wait")).await;
        cx.borrow_mut().x.rounds += 1;
    } else {
        dive_attack(cx).await;
    }
}

/// `func_CCC`: the notice: noticed; turn rate `notice_turn_rate`; 215 (the idle queued) for its length +
/// `notice_extra` facing the target; the default turn rate; a yield.
async fn notice(cx: &Cx) {
    cx.borrow_mut().x.noticed = true;
    set_turn_rate(cx, k(cx, "notice_turn_rate"));
    play(cx, req::NOTICE, k(cx, "motion_blend"));
    queue(cx, req::IDLE, k(cx, "notice_then_blend"));
    let mut t = motion_frames(cx, req::NOTICE) + k(cx, "notice_extra");
    while t > 0.0 {
        yaw_to(cx, tgt(cx));
        t -= step(cx);
        frame().await;
    }
    set_turn_rate(cx, k(cx, "turn_default"));
    frame().await;
}

/// The appearance (`0xC4C` → `0xC6E`, a spawned Wyvern): face Sora now, the fade-in (and flag 0x80: not
/// mapped), a yield, 69 as an action (`0x1000045`) with the idle queued (bit 30); the idle state, asked in
/// the same frame and taken by the supervisor once the action has played (KH1's whole script pauses
/// while it plays): the appearance is done then.
async fn appear(cx: Cx) {
    let s = sora(&cx);
    face_now(&cx, s);
    push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
    frame().await;
    push(&cx, ECmd::Action { request: req::APPEAR, attack: None, then: req::IDLE, blend: k(&cx, "appear_blend") });
    queue(&cx, req::IDLE, k(&cx, "appear_then_blend"));
    set_state(&cx, st::IDLE);
}

/// The idle behaviour (`0xB4E`): not noticed; forever: a rise when less than `low_above` over Sora, the
/// circle, the idle (blend `idle_rest_blend`) at turn rate `idle_turn_rate` facing Sora for `rest_lo..hi`
/// frames. The supervisor's search ends it.
async fn idle(cx: Cx) {
    cx.borrow_mut().x.noticed = false;
    loop {
        if low(&cx, sora(&cx)) {
            rise_here(&cx).await;
            frame().await;
        }
        let (time, turn) = circle_draws(&cx);
        circle(&cx, time, turn).await;
        play(&cx, req::IDLE, k(&cx, "idle_rest_blend"));
        set_turn_rate(&cx, k(&cx, "idle_turn_rate"));
        let mut t = rand_range(&cx, k(&cx, "rest_lo"), k(&cx, "rest_hi"), "rest");
        while t > 0.0 {
            yaw_to(&cx, sora(&cx));
            t -= step(&cx);
            frame().await;
        }
    }
}

/// The lost behaviour (`0x5D2`): the hits forgotten; Sora found (`lost_search`): the target, a yield, a rise
/// when low, the circle, the idle `idle_pause` frames, the engaged state. Not found: a rise when low, the
/// circle, the idle facing Sora for `rest_lo..hi` frames, again.
async fn lost(cx: Cx) {
    cx.borrow_mut().x.hits = 0;
    loop {
        let found = cx.borrow().w.target.is_some() && dist_to(&cx, sora(&cx)) - k(&cx, "lost_search") < 0.0;
        if found {
            set_target(&cx, true);
            frame().await;
            if low(&cx, tgt(&cx)) {
                rise_here(&cx).await;
                frame().await;
            }
            let (time, turn) = circle_draws(&cx);
            circle(&cx, time, turn).await;
            play(&cx, req::IDLE, k(&cx, "motion_blend"));
            wait(&cx, k(&cx, "idle_pause")).await;
            set_state(&cx, st::ENGAGED);
            return;
        }
        if low(&cx, sora(&cx)) {
            rise_here(&cx).await;
            frame().await;
        }
        let (time, turn) = circle_draws(&cx);
        circle(&cx, time, turn).await;
        play(&cx, req::IDLE, k(&cx, "lost_rest_blend"));
        let mut t = rand_range(&cx, k(&cx, "rest_lo"), k(&cx, "rest_hi"), "rest");
        while t > 0.0 {
            yaw_to(&cx, sora(&cx));
            t -= step(&cx);
            frame().await;
        }
    }
}

/// The engaged behaviour (`0x7BE`): the notice once; then rounds: after `round_limit` counted attack rounds
/// the lost state; farther than `engage_far`: the far attack (`far_chance`) or the charge; closer: the roar,
/// then the dive attack, every third time the far attack; a yield; again (Sora re-picked).
async fn engaged(cx: Cx) {
    let mut close_rounds = 0;
    cx.borrow_mut().x.rounds = 0;
    if !cx.borrow().x.noticed {
        notice(&cx).await;
    }
    loop {
        set_turn_rate(&cx, k(&cx, "turn_default"));
        if cx.borrow().x.rounds as f32 - k(&cx, "round_limit") >= 0.0 {
            cx.borrow_mut().x.rounds = 0;
            set_state(&cx, st::LOST);
            return;
        }
        let d = dist_to(&cx, tgt(&cx));
        if d - k(&cx, "engage_far") > 0.0 {
            if rand(&cx, "far") - k(&cx, "far_chance") > 0.0 {
                far_attack(&cx).await;
            } else {
                charge(&cx, k(&cx, "charge_face_rate"), k(&cx, "windup_blend"), k(&cx, "windup_wait")).await;
            }
        } else {
            roar(&cx).await;
            if close_rounds as f32 - k(&cx, "close_rounds") >= 0.0 {
                close_rounds = 0;
                far_attack(&cx).await;
            } else {
                dive_attack(&cx).await;
                close_rounds += 1;
            }
        }
        frame().await;
        set_target(&cx, true);
    }
}

/// The hit behaviour (`0x3C4`): hit in the dive: 214 (blend 0) for its length; then within `hit_close`:
/// from `hits_hi` hits the idle `idle_pause` frames, the roar and the dive attack; within `hit_mid`: from
/// `hits_hi` hits the idle, the roar, the charge, a rise when low, the lost state; then the engaged state.
async fn hit(cx: Cx) -> Flow {
    if cx.borrow().x.dive_hit {
        cx.borrow_mut().x.dive_hit = false;
        play(&cx, req::DIVE_HIT, k(&cx, "dive_hit_blend"));
        let n = motion_frames(&cx, req::DIVE_HIT);
        wait(&cx, n).await;
    }
    let d = dist_to(&cx, tgt(&cx));
    let hits = cx.borrow().x.hits as f32;
    let (lo, hi) = (k(&cx, "hits_lo"), k(&cx, "hits_hi"));
    // `1 ≤ hits < 5`: straight to the engaged state; `hits ≥ 5`: the follow-up (0 hits: neither).
    let some = hits - lo >= 0.0 && hits - hi < 0.0;
    if d - k(&cx, "hit_close") < 0.0 {
        if !some && hits - hi >= 0.0 {
            cx.borrow_mut().x.hits = 0;
            play(&cx, req::IDLE, k(&cx, "motion_blend"));
            wait(&cx, k(&cx, "idle_pause")).await;
            roar(&cx).await;
            dive_attack(&cx).await;
        }
    } else if d - k(&cx, "hit_mid") < 0.0 && !some && hits - hi >= 0.0 {
        cx.borrow_mut().x.hits = 0;
        play(&cx, req::IDLE, k(&cx, "motion_blend"));
        wait(&cx, k(&cx, "idle_pause")).await;
        roar(&cx).await;
        charge(&cx, k(&cx, "hit_charge_face_rate"), k(&cx, "hit_windup_blend"), 0.0).await;
        if height(&cx) - k(&cx, "low_height") < 0.0 || low(&cx, tgt(&cx)) {
            rise_here(&cx).await;
        }
        set_state(&cx, st::LOST);
        return END;
    }
    set_state(&cx, st::ENGAGED);
    ON
}

/// The supervisor (entry `0x21CC`): after the init's yield, each frame: while idle, Sora found (within
/// `search_range`) → the target and the engaged state; a state request restarts the behaviour (the target
/// set for engaged and hit, cleared for idle and lost).
async fn supervisor(cx: Cx) {
    frame().await;
    if !cx.borrow().x.spawned {
        // Re:CoM's own appearance played (its first-strike stun): the appearance thread's end.
        let s = sora(&cx);
        face_now(&cx, s);
        cx.borrow_mut().x.appeared = true;
        set_state(&cx, st::IDLE);
    }
    loop {
        if cx.borrow().state.0 == st::IDLE {
            let r = k(&cx, "search_range");
            if search(&cx, r) {
                set_target(&cx, true);
                set_state(&cx, st::ENGAGED);
            }
        }
        if let Some(s) = take_state(&cx) {
            cx.borrow_mut().x.appeared = true;
            set_target(&cx, s == st::ENGAGED || s == st::HIT);
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Wyvern's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as the Shadow's: per-enemy seeds). `frames`: KH1
    /// motion lengths by request. `spawned`: its appearance plays (every Re:CoM battle appearance, owner
    /// 2026-11-02).
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, spawned: bool) -> Ai {
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp { spawned, keep: true, target_ground: true, ..Default::default() });
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
            // `0x2204`: flight with no velocity.
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
                st::ENGAGED => Box::pin(engaged(cx)),
                st::IDLE => Box::pin(idle(cx)),
                st::LOST => Box::pin(lost(cx)),
                _ => Box::pin(async move {
                    let _ = hit(cx).await;
                }),
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
    /// The hit handler (`0x2484`): the turn rate back to the default (`0x254E`); while the dive 202 plays
    /// (`0x2560`) every hit counts and one that is not a clash (kinds 6 / 7) takes no reaction and asks for
    /// 214 (`0x2570..0x25F4`); otherwise every hit but a clash counts (`0x261A..0x26A6`); the hit state in
    /// every case. KH1's attribute exceptions (2 / 3 / 4: the engine's element reaction; 8 / 9 keep the dive's
    /// reaction; 9 / 11 nothing): not carried (Assumption; owner 2026-11-02: element reactions left out).
    fn on_hit(&self, news: HitNews) {
        let turn = self.io.borrow().c.num("turn_default");
        set_turn_rate(&self.io, turn);
        {
            let mut io = self.io.borrow_mut();
            io.x.keep = true;
            let clash = news == HitNews::Clash || matches!(io.x.kind, 6 | 7);
            if io.w.motion == Some(req::DIVE) {
                if !clash {
                    io.x.keep = false;
                    io.x.dive_hit = true;
                }
                io.x.hits += 1;
            } else if !clash {
                io.x.hits += 1;
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
    fn set_target_grounded(&self, on_ground: bool) {
        self.io.borrow_mut().x.target_ground = on_ground;
    }
    /// KH1's world 11 (Olympus): the supervisor's search reaches 100 m and always finds Sora (`0x2362`'s
    /// `wp+0x54`, `0x1930`).
    fn set_world(&self, kh1_world: Option<u8>) {
        self.io.borrow_mut().x.world11 = kh1_world == Some(11);
    }
    /// The host's quiet watchdog: engaged again (as the supervisor's state request does), flying with no
    /// velocity, the dive-hit flag cleared, and the turn rate and the flight easing every routine puts back
    /// at its end (11.25°, 0.2: a routine the restart cuts leaves its own).
    fn reset(&self) {
        let (turn, ease) = {
            let io = self.io.borrow();
            (io.c.num("turn_default"), io.c.num("ease_default"))
        };
        {
            let mut io = self.io.borrow_mut();
            io.x.appeared = true;
            io.x.dive_hit = false;
            io.body.fly(Vec3::ZERO);
        }
        set_turn_rate(&self.io, turn);
        set_air_accel(&self.io, ease);
        set_state(&self.io, st::ENGAGED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test numbers: the player's script's own (the asset step reads them; the disassembly's values), the
    /// shared engine's.
    pub(crate) fn consts() -> Consts {
        let v: &[(&str, f32)] = &[
            ("search_range", 1500.0), ("world11_search", 10000.0), ("motion_blend", 8.0), ("turn_default", 11.25), ("ease_default", 0.2),
            ("appear_fade", 30.0), ("appear_blend", 8.0), ("appear_then_blend", 12.0), ("dive_hit_blend", 0.0), ("hit_close", 300.0), ("hit_mid", 1000.0),
            ("hits_lo", 1.0), ("hits_hi", 5.0), ("idle_pause", 6.0), ("low_height", 250.0), ("low_above", 170.0), ("rise_up", -300.0), ("rise_speed", 4.0),
            ("rise_time", 300.0), ("rise_check", 30.0), ("rise_turn_rate", 5.0), ("rise_dir", -1.0), ("hover_blend", 16.0), ("lost_search", 5000.0),
            ("lost_rest_blend", 16.0), ("idle_rest_blend", 30.0), ("idle_turn_rate", 3.0), ("rest_lo", 90.0), ("rest_hi", 120.0), ("circle_time_lo", 250.0),
            ("circle_time_hi", 500.0), ("circle_turn_lo", 3.0), ("circle_turn_hi", 5.0), ("circle_turn_rate", 4.0), ("circle_chance", 0.6), ("circle_left", -90.0),
            ("circle_right", 90.0), ("circle_flip", -1.0), ("circle_blend", 30.0), ("circle_wait", 10.0), ("circle_half", 0.5), ("circle_speed", 16.0),
            ("halt_ease", 0.1), ("halt_tail", 60.0), ("round_limit", 3.0), ("engage_far", 600.0), ("far_chance", 0.3), ("close_rounds", 2.0),
            ("notice_turn_rate", 8.0), ("notice_then_blend", 0.0), ("notice_extra", 6.0), ("roar_blend", 0.0), ("roar_then_blend", 0.0),
            ("approach_face_rate", 6.0), ("chase_speed", 30.0), ("chase_arrive_far", 250.0), ("chase_arrive_dive", 100.0), ("chase_time", 600.0),
            ("flyby_ahead", 600.0), ("flyby_up", 450.0), ("flyby_speed", 30.0), ("flyby_arrive", 400.0), ("flyby_time", 500.0), ("flyby_chance", 0.3),
            ("dive_above", 400.0), ("hit_charge_face_rate", 1.4), ("charge_face_rate", 3.0), ("hit_windup_blend", 0.0), ("windup_blend", 16.0),
            ("windup_wait", 16.0), ("windup_short", 2.0), ("windup_turn_rate", 2.0), ("charge_blend", 0.0), ("charge_time", 40.0), ("charge_sink", 0.5),
            ("charge_speed", 40.0), ("charge_then_blend", 0.0), ("charge_extra", 6.0), ("bomb_above", 250.0), ("bomb_queue_blend", 0.0), ("bomb_flat", 250.0),
            ("bomb_creep", 4.0), ("bomb_floor", 50.0), ("bomb_down", 1.0), ("bomb_speed", 24.0), ("bomb_stuck", 1.0), ("land_floor", 10.0), ("land_down", 1.0),
            ("land_speed", 14.0), ("bite_turn_rate", 6.0), ("bite_blend_flying", 0.0), ("bite_wait", 8.0), ("bite_tail", 20.0), ("bite_tail_speed", 24.0),
            ("bite_speed", 4.0), ("bite_above", 30.0), ("bite_sink", 0.2), ("bite_end_blend", 0.0), ("stuck_after", 10.0), ("stuck_factor", 0.25),
            ("fly_halt_ease", 0.08), ("fly_idle_blend", 10.0), ("fly_level", 0.7), ("chase_queue_blend", 0.0), ("chase_near", 250.0), ("chase_stop_ease", 1.0),
            ("bank_turn", -90.0), ("chase_turn_rate", 6.0), ("bank_cos", 0.98), ("bank_blend", 16.0), ("bank_hold", 16.0), ("bank_face", 60.0),
            ("chase_floor", 50.0), ("descend_face_rate", 6.0), ("descend_idle_blend", 24.0), ("descend_idle_start", 16.0), ("descend_wait", 24.0),
            ("descend_down", 1.0), ("descend_speed", 4.0), ("descend_stuck", 0.4), ("turn_time", 200.0), ("turn_cos", 0.99), ("attack_blend", 8.0),
            ("walk_speed", 2.0), ("run_speed", 8.0), ("air_accel", 0.1), ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0), ("launch_min", 20.0),
            ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("down_time", 30.0),
            ("air_knock_hop", 20.0), ("weight", 1.0), ("gravity", crate::params::GRAVITY), ("fall_cap", 20.0),
        ];
        Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), NAMES).unwrap()
    }

    fn frames() -> Vec<(u16, f32)> {
        use req::*;
        vec![
            (IDLE, 320.0), (APPEAR, 60.0), (BITE, 120.0), (DIVE_START, 140.0), (DIVE, 120.0), (LAND, 100.0), (WINDUP, 108.0), (CHARGE, 120.0), (CHARGE_END, 106.0),
            (CHASE_START, 90.0), (FLY, 260.0), (BANK_L, 260.0), (BANK_R, 260.0), (HALT, 130.0), (HOVER, 200.0), (ROAR, 70.0), (DIVE_HIT, 92.0), (NOTICE, 204.0),
            (FLINCH, 92.0), (KNOCK, 100.0),
        ]
    }

    /// Clips the asset step marks looping (`kh1-wyvern.json`: the idle, the bite, the dive, the charge, flying,
    /// the banks, hovering, the notice): the sim plays them on when nothing is queued.
    fn looped(r: u16) -> bool {
        use req::*;
        matches!(r, IDLE | BITE | DIVE | CHARGE | FLY | BANK_L | BANK_R | HOVER | NOTICE)
    }

    /// A flat floor at y = 0 (KH1 y down), Sora on it (or in the air); the host plays motions to their length
    /// and queues, pauses the AI for actions (the appearance, the roar), keeps polling it during the steered
    /// attacks; the body flies by KH1's flight step, never through the floor.
    struct Sim {
        ai: Ai,
        pos: Vec3,
        last: Vec3,
        current: Option<u16>,
        played: f32,
        queued: Option<u16>,
        action: Option<u16>,
        log: Vec<(usize, Cmd)>,
        frame: usize,
    }

    impl Sim {
        fn new(seed: u32, spawned: bool, at: Vec3) -> Sim {
            Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: at, last: at, current: Some(req::IDLE), played: 0.0, queued: None, action: None, log: Vec::new(), frame: 0 }
        }

        fn len(r: u16) -> f32 {
            frames().iter().find(|x| x.0 == r).map_or(30.0, |x| x.1)
        }

        fn run(&mut self, sora: impl Fn(usize) -> (Vec3, bool), n: usize) {
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
                        } else if looped(c) {
                            self.played = 0.0;
                        }
                    }
                }
                let (s, ground) = sora(self.frame);
                self.ai.set_floor(0.0);
                self.ai.set_target_grounded(ground);
                if self.action.is_none() {
                    let moved = (self.pos - self.last).length();
                    let w = World { step: 1.0, pos: self.pos, target: Some(s), grounded: false, airborne: false, moved, motion: self.current };
                    for c in self.ai.frame(w) {
                        match c {
                            ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } if self.current != Some(request) => {
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
                            _ => {}
                        }
                        self.log.push((self.frame, c));
                    }
                }
                self.last = self.pos;
                let c = self.ai.consts();
                let steer = self.ai.take_steer();
                let d = self.ai.body().step(1.0, self.pos.y, steer, &c);
                self.pos += d;
                if self.pos.y > 0.0 {
                    self.pos.y = 0.0;
                }
            }
        }

        fn attacks(&self) -> Vec<(usize, Attack)> {
            self.log.iter().filter_map(|(f, c)| if let ECmd::Attack { attack, .. } = c { Some((*f, *attack)) } else { None }).collect()
        }

        fn plays(&self, r: u16) -> bool {
            self.log.iter().any(|(_, c)| matches!(c, ECmd::Motion { request, .. } if *request == r))
        }

        fn requests(&self) -> Vec<u16> {
            self.log
                .iter()
                .filter_map(|(_, c)| match c {
                    ECmd::Motion { request, .. } | ECmd::Attack { request, .. } | ECmd::Action { request, .. } | ECmd::MotionAt { request, .. } => Some(*request),
                    _ => None,
                })
                .collect()
        }
    }

    fn order_ok(seq: &[u16], want: &[u16]) -> bool {
        let mut i = 0;
        for &r in seq {
            if i < want.len() && r == want[i] {
                i += 1;
            }
        }
        i == want.len()
    }

    #[test]
    fn the_names_cover_the_numbers() {
        assert!(Table::new(vec![], NAMES).is_err());
        let c = consts();
        assert_eq!((c.num("chase_speed"), c.num("dive_above"), c.num("round_limit")), (30.0, 400.0, 3.0));
        assert_eq!(Attack::ALL.map(|a| a.request()), [200, 202, 203, 205, 206]);
    }

    /// The battle spawn: face Sora, the fade-in, KH1's appearance (69, an action), then idle: Sora within 15 m
    /// engages it at once, it notices him (215 for its length + 6) and starts its rounds.
    #[test]
    fn appears_notices_and_engages() {
        let mut s = Sim::new(7, true, Vec3::new(0.0, -300.0, 0.0));
        let sora = |_| (Vec3::new(0.0, 0.0, 800.0), true);
        s.run(sora, 3);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::Action { request: req::APPEAR, .. })));
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::FadeIn(_))));
        assert!(s.ai.body().flying);
        s.run(sora, 70);
        assert!(s.ai.risen());
        // The idle state asked at the appearance is engaged at once by the supervisor's search (8 m).
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))), "{:?}", s.log);
        assert!(s.plays(req::NOTICE));
        let noticed = s.log.iter().find(|(_, c)| matches!(c, ECmd::Motion { request: req::NOTICE, .. })).unwrap().0;
        s.run(sora, 260);
        // Its first round starts after the notice's 204 + 6 frames.
        let next = s.log.iter().find(|(f, c)| *f > noticed && !matches!(c, ECmd::Yaw(_)) && matches!(c, ECmd::Motion { .. } | ECmd::Action { .. } | ECmd::Attack { .. }) && !matches!(c, ECmd::Motion { request: req::NOTICE, .. })).unwrap();
        assert!(next.0 - noticed >= 210, "{} after {}", next.0, noticed);
    }

    /// Far (> 6 m), 70 %: the far attack: turn, the chase (207, 208 queued) to 2.5 m, the bite (200), a fly-by
    /// (208) up and past him, down to his height (212, the idle from frame 16), then the charge or the dive.
    #[test]
    fn the_far_attack_chases_bites_flies_by_and_comes_down() {
        let mut s = Sim::new(5, false, Vec3::new(0.0, -300.0, 0.0));
        let sora = |_| (Vec3::new(0.0, 0.0, 1400.0), true);
        s.run(sora, 2);
        let cx = s.ai.io.clone();
        cx.borrow_mut().x.noticed = true;
        s.ai.behaviour = Some(Box::pin(async move { far_attack(&cx).await }));
        s.log.clear();
        s.run(sora, 2000);
        let seq = s.requests();
        assert!(order_ok(&seq, &[req::CHASE_START, req::BITE, req::FLY, req::HOVER, req::IDLE]), "{seq:?}");
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::MotionAt { request: req::IDLE, start, .. } if *start == 16.0)), "the idle from frame 16");
        assert!(s.attacks().iter().any(|a| a.1 == Attack::Bite));
        assert!(s.attacks().iter().any(|a| matches!(a.1, Attack::Charge | Attack::Dive)), "{:?}", s.attacks());
    }

    /// The dive attack: the chase to 4 m above Sora, 201 (202 queued), the dive 202 straight down to 2.5 m above
    /// him, the landing 203 sinking to the floor.
    #[test]
    fn the_dive_attack_comes_down_on_him() {
        let mut s = Sim::new(3, false, Vec3::new(0.0, -200.0, 0.0));
        let sora = |_| (Vec3::new(100.0, 0.0, 600.0), true);
        s.run(sora, 2);
        let cx = s.ai.io.clone();
        s.ai.behaviour = Some(Box::pin(async move { dive_attack(&cx).await }));
        s.log.clear();
        let mut top = 0.0f32;
        for _ in 0..1200 {
            s.run(sora, 1);
            top = top.min(s.pos.y);
        }
        let seq = s.requests();
        assert!(order_ok(&seq, &[req::CHASE_START, req::DIVE_START, req::DIVE, req::LAND, req::IDLE]), "{seq:?}");
        let at: Vec<Attack> = s.attacks().iter().map(|a| a.1).collect();
        assert_eq!(at, vec![Attack::Dive, Attack::Landing]);
        assert!(top < -350.0, "it rose to about 4 m above him: {top}");
        assert!(s.pos.y > -20.0, "it came down: {}", s.pos.y);
        assert_eq!(s.ai.io.borrow().x.rounds, 1);
    }

    /// The charge: the wind-up 204 turning to him, the charge 205 for 40 frames at 40 a frame, its end 206, the
    /// idle when 206 has played.
    #[test]
    fn the_charge_rushes_along_the_facing() {
        let mut s = Sim::new(9, false, Vec3::new(0.0, 0.0, 0.0));
        let sora = |_| (Vec3::new(0.0, 0.0, 1200.0), true);
        s.run(sora, 2);
        let cx = s.ai.io.clone();
        s.ai.behaviour = Some(Box::pin(async move { charge(&cx, 3.0, 16.0, 16.0).await }));
        s.log.clear();
        s.run(sora, 140);
        let start = s.pos;
        let f0 = s.frame;
        s.run(sora, 400);
        let at = s.attacks();
        assert_eq!(at.iter().map(|a| a.1).collect::<Vec<_>>(), vec![Attack::Charge, Attack::ChargeEnd]);
        assert_eq!(at[1].0 - at[0].0, 40, "40 frames of charge");
        assert!(at[0].0 >= f0 - 20 && (s.pos - start).length() > 1000.0, "it rushed: {:?}", s.pos - start);
        let end = s.log.iter().find(|(f, c)| *f > at[1].0 && matches!(c, ECmd::Motion { request: req::IDLE, .. })).unwrap().0;
        assert_eq!(end - at[1].0, 106, "the idle when 206 has played");
    }

    /// The hit handler: a hit in the dive takes no reaction and asks for 214; a clash outside it is not counted
    /// (in it, it is); five hits within 3 m: the roar and the dive attack.
    #[test]
    fn the_hit_handler() {
        let mut s = Sim::new(1, false, Vec3::new(0.0, -150.0, -100.0));
        s.run(|_| (Vec3::new(0.0, 0.0, 50.0), true), 5);
        s.ai.io.borrow_mut().w.motion = Some(req::DIVE);
        s.ai.set_hit_kind(3);
        s.ai.on_hit(HitNews::Damage);
        assert!(!s.ai.keeps_reaction() && s.ai.io.borrow().x.dive_hit);
        s.ai.set_hit_kind(6);
        s.ai.on_hit(HitNews::Clash);
        assert!(s.ai.keeps_reaction());
        assert_eq!(s.ai.io.borrow().x.hits, 2, "the clash in the dive counts");
        s.ai.io.borrow_mut().w.motion = Some(req::IDLE);
        s.ai.on_hit(HitNews::Clash);
        assert_eq!(s.ai.io.borrow().x.hits, 2, "a clash outside the dive doesn't");
        assert_eq!(s.ai.io.borrow().body.turn, Some(11.25f32.to_radians()));
        s.log.clear();
        s.run(|_| (Vec3::new(0.0, 0.0, 50.0), true), 3);
        assert!(s.plays(req::DIVE_HIT), "{:?}", s.log);
        // Five hits close: after 214 the idle, the roar, the dive attack.
        s.ai.io.borrow_mut().x.hits = 4;
        s.ai.set_hit_kind(3);
        s.ai.on_hit(HitNews::Damage);
        s.log.clear();
        s.run(|_| (Vec3::new(0.0, 0.0, 50.0), true), 200);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::Action { request: req::ROAR, .. })), "{:?}", s.log);
        assert!(s.plays(req::CHASE_START));
        // Two hits close: straight back to engaged.
        let mut s = Sim::new(2, false, Vec3::new(0.0, -200.0, 0.0));
        s.run(|_| (Vec3::new(0.0, 0.0, 250.0), true), 5);
        s.ai.set_hit_kind(3);
        s.ai.on_hit(HitNews::Damage);
        s.log.clear();
        s.run(|_| (Vec3::new(0.0, 0.0, 250.0), true), 3);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))));
    }

    /// Five hits at 3–10 m: the roar, the charge (wind-up at 1.4° a frame), the lost state (the circle), engaged.
    #[test]
    fn mid_range_hits_charge_then_circle() {
        let mut s = Sim::new(4, false, Vec3::new(0.0, -200.0, 0.0));
        let sora = |_| (Vec3::new(0.0, 0.0, 700.0), true);
        s.run(sora, 5);
        s.ai.io.borrow_mut().x.hits = 4;
        s.ai.set_hit_kind(3);
        s.ai.on_hit(HitNews::Damage);
        s.log.clear();
        s.run(sora, 1500);
        let seq = s.requests();
        assert!(order_ok(&seq, &[req::ROAR, req::WINDUP, req::CHARGE, req::CHARGE_END]), "{seq:?}");
        let lost = s.log.iter().position(|(_, c)| matches!(c, ECmd::State(st::LOST))).expect("lost");
        assert!(s.log[lost..].iter().any(|(_, c)| matches!(c, ECmd::Motion { request: req::BANK_L | req::BANK_R, .. })), "the circle");
        assert!(s.log[lost..].iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))));
    }

    /// Three counted rounds: the lost state (the circle), then engaged again.
    #[test]
    fn three_rounds_then_it_circles() {
        let mut s = Sim::new(6, false, Vec3::new(0.0, -200.0, 0.0));
        let sora = |_| (Vec3::new(0.0, 0.0, 400.0), true);
        s.run(sora, 6000);
        let lost = s.log.iter().filter(|(_, c)| matches!(c, ECmd::State(st::LOST))).count();
        assert!(lost >= 1, "{lost}");
    }

    /// KH1's world 11: the idle's search finds Sora at any distance.
    #[test]
    fn world_11_finds_him_anywhere() {
        let mut s = Sim::new(8, false, Vec3::new(0.0, -200.0, 0.0));
        let sora = |_| (Vec3::new(0.0, 0.0, 4000.0), true);
        s.run(sora, 30);
        assert!(!s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))), "40 m: it stays idle");
        assert!(s.plays(req::BANK_L) || s.plays(req::BANK_R) || s.plays(req::HOVER), "it circles");
        let mut s = Sim::new(8, false, Vec3::new(0.0, -200.0, 0.0));
        s.ai.set_world(Some(11));
        s.run(sora, 30);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))));
    }

    /// The quiet watchdog's reset: engaged, flying, the default rates.
    #[test]
    fn reset_engages_again() {
        let mut s = Sim::new(2, false, Vec3::new(0.0, -200.0, 0.0));
        s.run(|_| (Vec3::new(0.0, 0.0, 300.0), true), 200);
        s.ai.io.borrow_mut().x.dive_hit = true;
        s.ai.io.borrow_mut().body.accel = Some(1.0);
        s.ai.reset();
        s.log.clear();
        s.run(|_| (Vec3::new(0.0, 0.0, 300.0), true), 3);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))));
        let io = s.ai.io.borrow();
        assert!(!io.x.dive_hit && io.body.flying && io.body.accel == Some(0.2) && io.body.turn == Some(11.25f32.to_radians()));
    }

    /// No stall: Sora circling 2–9 m away for 20 000 frames (12 seeds), jumping now and then, with or without
    /// a hit every 150–400 frames (sometimes in the dive): it keeps acting (an attack at least every 2000
    /// frames), steers or plays something at least every 600, never goes under the floor; every seed uses
    /// the dive and its landing and one more attack, and the seeds together use all five. (With the hits, each
    /// restarts the engaged rounds, so the close rounds' third-time far attack is rare: KH1's rule.)
    fn long_fight(hits: bool) -> std::collections::HashSet<Attack> {
        let mut all = std::collections::HashSet::new();
        for i in 1u32..=12 {
            let seed = i.wrapping_mul(0x9E37_79B9) ^ 0x1234_5678;
            let mut s = Sim::new(seed, true, Vec3::new(0.0, -150.0, 0.0));
            let sora = |f: usize| {
                let a = f as f32 * 0.002;
                let r = 550.0 + 350.0 * (f as f32 * 0.0007).sin();
                let air = (f / 300) % 7 == 3 && f % 300 < 40;
                (Vec3::new(r * a.cos(), if air { -120.0 } else { 0.0 }, r * a.sin()), !air)
            };
            let mut next_hit = 300;
            let mut quiet = 0;
            let mut worst_quiet = 0;
            for f in 0..20_000usize {
                let before = s.log.len();
                s.run(sora, 1);
                let steering = s.ai.body().air_vel.length() > 0.01;
                if s.log.len() > before || steering || s.action.is_some() {
                    quiet = 0;
                } else {
                    quiet += 1;
                    worst_quiet = worst_quiet.max(quiet);
                }
                assert!(s.pos.y <= 0.0 && s.pos.length().is_finite(), "seed {seed}: {:?}", s.pos);
                if hits && f == next_hit {
                    next_hit += 150 + (f * 7919 + seed as usize * 104_729) % 250;
                    if s.action.is_none() {
                        s.ai.io.borrow_mut().w.motion = s.current;
                        s.ai.set_hit_kind(if f % 5 == 0 { 4 } else { 3 });
                        s.ai.on_hit(HitNews::Damage);
                    }
                }
            }
            assert!(worst_quiet < 600, "seed {seed}: quiet {worst_quiet} frames");
            let at = s.attacks();
            assert!(at.len() > 10, "seed {seed}: {} attacks", at.len());
            let mut last = 0;
            for &(f, _) in &at {
                assert!(f - last < 2000, "seed {seed}: no attack from {last} to {f}");
                last = f;
            }
            assert!(20_000 - last < 2000, "seed {seed}: nothing after {last}");
            assert!(s.ai.body().flying, "seed {seed}");
            let kinds: std::collections::HashSet<_> = at.iter().map(|a| a.1).collect();
            assert!(kinds.contains(&Attack::Dive) && kinds.contains(&Attack::Landing) && kinds.len() >= 3, "seed {seed}: {kinds:?}");
            all.extend(kinds);
        }
        all
    }

    #[test]
    fn a_long_fight_never_stalls() {
        assert_eq!(long_fight(true).len(), 5);
        assert_eq!(long_fight(false).len(), 5);
    }
}
