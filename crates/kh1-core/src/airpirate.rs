//! KH1 FM's Air Pirate (`xa_ex_2210.mdls` part 5, `ex_2210.bd`), engine-agnostic: its AI script on the
//! shared enemy framework (`crate::enemy`), flying (KH1's movement state 4,
//! `crate::enemy::Body::flying`). M11 (`docs/recom-m11-enemies.md` § The Air Pirate). Re:CoM runs it
//! natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`).
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2210.bd` at checked
//! offsets). Script offsets are the player's `ex_2210.bd`'s (Checked (files): the script
//! disassembled with `tools/kh1-ai-script.py`, raw and folded; the syscalls named are Checked
//! (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x1CF6`: init `0x1E04` (stats records LV1 `0x14` / LV2 `0x80`, the stats block's hop 800
//!   (unused: it never hops), the work area, home, the search table `0x200`, the handlers: hit
//!   `0x1F0C`), flight with no velocity (`sys 135` with (0, 0, 0, 1)), a yield; outside KH1's world 11
//!   (Olympus, `sys 105`) the appearance thread `0xA6E`; then each frame a state request restarts the
//!   behaviour (1 engaged `0x4A2` with the target, 2 idle `0x8D2` with the target cleared, 3 hit
//!   `0x274` with the hitter or Sora as the target). The script never sets a turn rate or a flight
//!   easing: the stats block's defaults (π/16 a frame, 0.1).
//! - the hit handler `0x1F0C`: a hit while the dive 206 plays takes no reaction (the hit's reaction
//!   kind zeroed) and asks for the dive-hit motion 211; a launch (kind 5) asks for an 8-frame pause;
//!   every hit but a clash outside the dive counts (`wp+0x38`); all ask for the hit state 3.
//!
//! The work area: `wp+0x38` hits, `+0x3C` rounds, `+0x40` the fly motion asked for, `+0x44` near (no
//! wind-up), `+0x48` done (ends a round's loop), `+0x4C` hit in the dive, `+0x50` launched.
//!
//! Not ported (Re:CoM's battles only, decision M10): KH1's world-11 rules (`0x1E04`, `0x13BE`: Olympus
//! cup targets), the spawn type 5 rise from below (`0xAA4`; Re:CoM's battle spawns take the plain
//! appearance: Assumption), the party retargeting (`0x1364`, `0x13BE`: Sora is the only target), the
//! hits' KH1 attribute (`hp[0xC] >> 6 & 15`: 9 / 11 end the handler, 8 / 9 keep the dive's reaction;
//! the host carries no attribute), the effects (`sys 75` 2 / 7, `sys 74`), the fade-in and flag 0x80
//! (`0x1BB4`, `0x1B38`: [`ECmd::FadeIn`] for the log only), the death routine (`0x20E2`: Re:CoM's
//! defeat).

use crate::enemy::script::{fly, frame, height, motion_done, motion_frames, poll, pos, push, rand, rand_range, random_offset, set_state, set_target, steer, take_state, target, tick_clock};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Air Pirate's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The appearance (`0x195A`), 78 frames.
    pub const APPEAR: u16 = 69;
    /// The swoop (`0xC4E` / `0xEBC`): a plain request with row `0xEC`, 122 frames, root, hit [41, 48).
    pub const SWOOP: u16 = 200;
    /// The take-off before a flight (`0x3368`, `0x2100`, the circle `0x358A`), 30 frames.
    pub const TAKE_OFF: u16 = 201;
    /// Flying (`0x3368`, `0x2A22`), 64 frames, looped.
    pub const FLY: u16 = 202;
    /// The halt (`0x3368`; the circle's end, an action), 50 frames.
    pub const HALT: u16 = 203;
    /// Rising (`0x3706`, `0x2642`), 64 frames, looped.
    pub const RISE: u16 = 204;
    /// Before the dive from above (`0x2100`), 44 frames.
    pub const DIVE_START: u16 = 205;
    /// The dive (`0x27CA`), 64 frames, looped: hits take no reaction.
    pub const DIVE: u16 = 206;
    /// The dive's last stretch (`0x27CA`), 46 frames.
    pub const DIVE_END: u16 = 207;
    /// Banking (`0x2A22`) and circling (`0x358A`): 208 with the facing turned −90°, 209 with +90°.
    pub const BANK_L: u16 = 208;
    pub const BANK_R: u16 = 209;
    /// The wind-up before a long flight (`0x2100`), 110 frames.
    pub const WINDUP: u16 = 210;
    /// Hit out of the dive (`0x274`), 86 frames.
    pub const DIVE_HIT: u16 = 211;
    /// The slash (`0xBC6`): a plain request with row `0x15C`, 100 frames, root, hit [39.3, 51).
    pub const SLASH: u16 = 212;
    /// The swoop's clash recoil (row `0xEC` `+0x40`), 80 frames; the engine plays it.
    pub const RECOIL: u16 = 213;
}

/// The Air Pirate's attacks, in the order the host indexes them (the manifest's `attacks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    /// `0xC4E` / `0xEBC`: the swoop, 200 with row `0xEC` (`sys 23` mode 2: it lives while 200 plays):
    /// the script keeps steering it at the target.
    Swoop,
    /// `0xBC6`: the slash, 212 with row `0x15C`: the script keeps turning to the target.
    Slash,
}

impl Attack {
    pub const ALL: [Attack; 2] = [Attack::Swoop, Attack::Slash];

    pub fn request(self) -> u16 {
        match self {
            Attack::Swoop => req::SWOOP,
            Attack::Slash => req::SLASH,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Swoop => "swoop",
            Attack::Slash => "slash",
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The names of the Air Pirate's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    "search_range",
    // Blends.
    "motion_blend",
    "appear_blend",
    "rest_blend",
    "dive_hit_blend",
    "close_swoop_blend",
    "bank_blend",
    // Appearance `0x195A`.
    "appear_fade",
    "appear_wait",
    // Hit `0x274`.
    "hit_wait",
    "hit_close",
    "hit_mid",
    "hit_high",
    "hits_lo",
    "hits_mid",
    "hits_hi",
    // Engaged `0x4A2`.
    "engage_far",
    "engage_mid",
    "engage_close",
    "high_above",
    // Mid `0x5DA`, close `0x69C`.
    "approach_speed",
    "approach_near",
    "back_speed",
    "back_far",
    "back_near",
    "back_add_near",
    "back_add_far",
    "close_chance",
    "rise_floor",
    "rise_above",
    "close_rise",
    "rise_speed",
    "roam_min",
    "roam_max",
    "roam_speed",
    "roam_arrive",
    "roam_time",
    "round_limit",
    // Idle `0x8D2`.
    "circle_time_lo",
    "circle_time_hi",
    "circle_turn_lo",
    "circle_turn_hi",
    "rest_lo",
    "rest_hi",
    "idle_above",
    "idle_rise",
    // Slash `0xBC6`, swoop `0xC4E`.
    "slash_tail",
    "swoop_low",
    "swoop_settle",
    "swoop_settle_speed",
    "swoop_high",
    "swoop_sink",
    "swoop_sink_speed",
    "swoop_tail",
    "swoop_near",
    "swoop_speed",
    // Far `0x2100`.
    "below_dir",
    "above_dir",
    "below_cos",
    "level_cos",
    "above_cos",
    "above_level_cos",
    "far_windup",
    "windup_above",
    "charge_drop",
    "charge_speed",
    "charge_arrive",
    "charge_time",
    "level_speed",
    // Climb `0x2642`.
    "climb_under",
    "climb_near",
    "climb_up",
    "climb_speed",
    "climb_stuck",
    // Dive `0x27CA`.
    "dive_above",
    "dive_near",
    "dive_floor",
    "dive_sink",
    "dive_sink_speed",
    "dive_down",
    "dive_speed",
    "dive_stuck",
    // Charge `0x2A22`.
    "charge_near",
    "charge_low",
    "bank_turn",
    "bank_cos",
    "bank_hold",
    "bank_face",
    "charge_floor",
    "charge_check",
    "charge_stuck",
    "charge_rise",
    // Approach `0x2E7E`.
    "approach_top",
    "approach_over",
    "approach_floor",
    "approach_check",
    "approach_stuck",
    "approach_rise",
    "approach_rise_speed",
    // Fly to a point `0x30B6`.
    "fly_check",
    "fly_near",
    "fly_floor",
    "fly_stuck",
    "fly_stuck_wait",
    // Nudge `0x32B4`.
    "nudge_range",
    "nudge_y",
    "nudge_z",
    "nudge_speed",
    // Back off `0x3368`.
    "back_chance",
    "back_turn",
    "back_turn_other",
    "back_wait",
    "back_check",
    "back_stuck",
    "back_close",
    "back_close_speed",
    "back_rise",
    // Circle `0x358A`.
    "circle_half",
    "circle_chance",
    "circle_left",
    "circle_right",
    "circle_flip",
    "circle_speed",
    // Rise `0x3706`.
    "rise_up",
    "rise_stuck",
];

/// The Air Pirate's numbers.
pub type Consts = Table;

/// The state objects (`wp+4`): the supervisor's requests.
pub mod st {
    pub const NONE: u8 = 0;
    pub const ENGAGED: u8 = 1;
    pub const IDLE: u8 = 2;
    pub const HIT: u8 = 3;
}

/// The Air Pirate's own work area: the actor's steering direction `+0x20` (a script writes it, all or
/// only its `y`, and leaves it; only the speed `+0x2C` is zeroed each frame), the counters and flags of
/// the module notes, the appearance, Sora on the ground (`*(target + 0x70)`, his movement state), the
/// last hit's kind and whether it keeps its reaction.
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    dir: Vec3,
    hits: i32,
    rounds: i32,
    fly: bool,
    near: bool,
    done: bool,
    dive_hit: bool,
    launched: bool,
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

/// `func_160E` (`sys 4(0)`): party 0, Sora.
fn sora(cx: &Cx) -> Vec3 {
    let io = cx.borrow();
    io.w.target.unwrap_or(io.w.pos)
}

/// The actor's target (`+0x74`): Sora once set (Re:CoM's battles have no other party member).
fn tgt(cx: &Cx) -> Vec3 {
    target(cx).unwrap_or_else(|| sora(cx))
}

/// `func_17AC`: the distance from the actor to `p` (`sys 6`, 3D).
fn dist_to(cx: &Cx, p: Vec3) -> f32 {
    (p - me(cx)).length()
}

/// `p` at the actor's height minus the actor: the flat line to it (the scripts' `y = actor.y` copies).
fn flat_to(cx: &Cx, p: Vec3) -> Vec3 {
    let m = me(cx);
    Vec3::new(p.x - m.x, 0.0, p.z - m.z)
}

/// A height the script compares as a world `y` (`0x69C`'s −170, `0x2E7E`'s −100), taken from the
/// battle floor (Assumption: KH1's Air Pirate rooms have their floor at y = 0; Re:CoM's battle floors
/// sit at any height). Without a floor: the world `y`.
fn rel_y(cx: &Cx, y: f32) -> f32 {
    y - cx.borrow().floor.unwrap_or(0.0)
}

/// Sora on the ground: his movement state `*(target + 0x70)` is 0 (Checked (decomp): `0x1402B1960`
/// keeps it at `+0x70`; 2 is the air). The host gives it ([`Behaviour::set_target_grounded`]).
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

/// `sys 10(v, deg)` on a direction: turned about the vertical (the Air Soldier's and the Darkball's
/// convention: the yaw + deg; Assumption, as theirs).
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

/// `func_1A48` / `sys 36`: the target yaw toward `p` (the engine turns).
fn yaw_to(cx: &Cx, p: Vec3) {
    let d = p - me(cx);
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `sys 36` along a vector.
fn yaw_along(cx: &Cx, d: Vec3) {
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `sys 12` / `sys 13`: a motion request; `sys 172`'s clock restarts for a request that plays anew.
/// A request of the motion already playing, with no other motion asked for this frame, changes nothing
/// ([`ECmd::Motion`]'s rule): it is not sent (the script asks for its flight motions every frame).
fn play(cx: &Cx, r: u16, blend: f32) {
    {
        let mut io = cx.borrow_mut();
        let asked = io.out.iter().any(|c| matches!(c, ECmd::Motion { .. } | ECmd::Attack { .. } | ECmd::Action { .. }));
        if io.w.motion == Some(r) && !asked {
            return;
        }
        if io.w.motion != Some(r) && io.clock.0 != Some(r) {
            io.clock = (Some(r), 0.0);
        }
    }
    push(cx, ECmd::Motion { request: r, blend });
}

/// A plain request made with its attack row (`sys 23` then `sys 13`).
fn attack(cx: &Cx, a: Attack, blend: f32) {
    cx.borrow_mut().clock = (Some(a.request()), 0.0);
    push(cx, ECmd::Attack { request: a.request(), attack: a, blend });
}

/// `func_380E`: wait `n` frames.
async fn wait(cx: &Cx, n: f32) {
    crate::enemy::script::wait_frames(cx, n).await;
}

/// `func_1A2A`: a yield, then until the motion has played to its end (`sys 172`).
async fn wait_motion(cx: &Cx) {
    loop {
        frame().await;
        if motion_done(cx) {
            break;
        }
    }
}

/// The frames this frame's displacement must reach for a stuck test (`sys 118`'s `+0x10 − +0x490`).
fn stuck(cx: &Cx, speed: f32) -> bool {
    let io = cx.borrow();
    io.w.moved - speed * io.w.step < 0.0
}

/// A target within `range` (`func_13BE` → `sys 48`): Sora (Assumption: KH1's search finds Sora when he
/// is within the range; `func_13BE`'s fallbacks also give him within it; `sys 139`'s out-of-sight test
/// is taken as in sight).
fn search(cx: &Cx, range: f32) -> bool {
    cx.borrow().w.target.is_some() && dist_to(cx, sora(cx)) - range < 0.0
}

/// How the routines end: `func_16CC` + `misc8` ends the behaviour thread from any depth (a state
/// request is pending; the supervisor restarts the behaviour next frame).
type Flow = bool;
const END: Flow = true;
const ON: Flow = false;

/// `func_3706(dy, motion, speed, from)`: rise until `dy` (negative: up) from the height `from`, playing
/// `motion`; stuck (moved under `speed × rise_stuck` a frame): the idle and the end (no yield).
async fn rise(cx: &Cx, dy: f32, motion: u16, speed: f32, from: f32) {
    loop {
        if (me(cx).y - from) - dy > 0.0 {
            set_dir(cx, Vec3::new(0.0, k(cx, "rise_up"), 0.0));
            go(cx, speed);
            play(cx, motion, k(cx, "motion_blend"));
            frame().await;
            if stuck(cx, speed * k(cx, "rise_stuck")) {
                halt(cx);
                play(cx, req::IDLE, k(cx, "motion_blend"));
                return;
            }
        } else {
            halt(cx);
            play(cx, req::IDLE, k(cx, "motion_blend"));
            frame().await;
            return;
        }
    }
}

/// `func_32B4`: the idle, then while within `nudge_range` (flat) of the target: (0, `nudge_y`,
/// `nudge_z`) at `nudge_speed` (world axes, as the script writes them; Checked (files)).
async fn nudge(cx: &Cx) {
    play(cx, req::IDLE, k(cx, "motion_blend"));
    loop {
        let v = flat_to(cx, tgt(cx));
        if v.length() - k(cx, "nudge_range") >= 0.0 {
            return;
        }
        set_dir(cx, Vec3::new(0.0, k(cx, "nudge_y"), k(cx, "nudge_z")));
        go(cx, k(cx, "nudge_speed"));
        frame().await;
    }
}

/// `func_C4E` / `func_EBC` (the same routine, the request's blend 8 or 0): the swoop. Face the target;
/// low (under `swoop_low` above it): stop (`func_3706(100, 0, 4)` ends at once) and two yields; high
/// (over `swoop_high` above it) over a target on the ground: sink at `swoop_sink_speed` to that height;
/// then 200 with its row, steering at the target (flat, at `swoop_speed`, beyond `swoop_near`) until
/// `swoop_tail` frames are left, those frames still, the idle.
async fn swoop(cx: &Cx, blend: f32) {
    yaw_to(cx, tgt(cx));
    if me(cx).y - (tgt(cx).y - k(cx, "swoop_low")) > 0.0 {
        let from = me(cx).y;
        rise(cx, k(cx, "swoop_settle"), req::IDLE, k(cx, "swoop_settle_speed"), from).await;
        frame().await;
    } else if me(cx).y - (tgt(cx).y - k(cx, "swoop_high")) < 0.0 && target_ground(cx) {
        loop {
            if me(cx).y - (tgt(cx).y - k(cx, "swoop_high")) < 0.0 {
                set_dir(cx, Vec3::new(0.0, k(cx, "swoop_sink"), 0.0));
                go(cx, k(cx, "swoop_sink_speed"));
                frame().await;
            } else {
                halt(cx);
                frame().await;
                break;
            }
        }
    }
    attack(cx, Attack::Swoop, blend);
    let mut t = motion_frames(cx, req::SWOOP);
    while t - k(cx, "swoop_tail") > 0.0 {
        yaw_to(cx, tgt(cx));
        if dist_to(cx, tgt(cx)) - k(cx, "swoop_near") > 0.0 {
            set_dir(cx, norm(flat_to(cx, tgt(cx))));
            go(cx, k(cx, "swoop_speed"));
        } else {
            halt(cx);
        }
        t -= step(cx);
        frame().await;
    }
    halt(cx);
    wait(cx, t).await;
    play(cx, req::IDLE, k(cx, "motion_blend"));
}

/// `func_BC6`: the slash: 212 with its row, facing the target until `slash_tail` frames are left,
/// those frames, the idle.
async fn slash(cx: &Cx) {
    attack(cx, Attack::Slash, k(cx, "motion_blend"));
    let mut t = motion_frames(cx, req::SLASH);
    while t - k(cx, "slash_tail") > 0.0 {
        yaw_to(cx, tgt(cx));
        t -= step(cx);
        frame().await;
    }
    wait(cx, t).await;
    play(cx, req::IDLE, k(cx, "motion_blend"));
}

/// `func_3368(target, speed, far, motion)`: back off: the flat line from the target turned 180° (or
/// −180°, at random), turn to it, 10 frames, the take-off and flying along it until `far` (flat) from
/// the target, then the halt (203 for its length); stuck after `back_check` frames: rise 300 (near the
/// target: idle at `back_close_speed`). The idle at the end.
async fn back_off(cx: &Cx, speed: f32, far: f32, motion: u16) {
    let mut t = 0.0;
    let mut v = flat_to(cx, tgt(cx));
    let turn = if rand(cx, "back side") - k(cx, "back_chance") > 0.0 { k(cx, "back_turn") } else { k(cx, "back_turn_other") };
    v = rotate(v, turn);
    yaw_along(cx, v);
    wait(cx, k(cx, "back_wait")).await;
    frame().await;
    play(cx, req::TAKE_OFF, k(cx, "motion_blend"));
    loop {
        play(cx, req::FLY, k(cx, "motion_blend"));
        let g = flat_to(cx, tgt(cx));
        let d3 = dist_to(cx, tgt(cx));
        if g.length() - far <= 0.0 {
            v = norm(v);
            set_dir(cx, v);
            go(cx, speed);
            frame().await;
            if t - k(cx, "back_check") > 0.0 && stuck(cx, speed * k(cx, "back_stuck")) {
                let (m, s) = if d3 - k(cx, "back_close") < 0.0 { (req::IDLE, k(cx, "back_close_speed")) } else { (motion, speed) };
                let from = me(cx).y;
                rise(cx, k(cx, "back_rise"), m, s, from).await;
                frame().await;
                break;
            }
        } else {
            halt(cx);
            play(cx, req::HALT, k(cx, "motion_blend"));
            let n = motion_frames(cx, req::HALT);
            wait(cx, n).await;
            frame().await;
            break;
        }
        t += step(cx);
    }
    play(cx, req::IDLE, k(cx, "motion_blend"));
}

/// The back-off distance `0x5DA` / `0x69C` take: the distance + `back_add_near` within `back_near`,
/// else + `back_add_far`.
fn back_far(cx: &Cx) -> f32 {
    let d = dist_to(cx, tgt(cx));
    if d - k(cx, "back_near") < 0.0 {
        d + k(cx, "back_add_near")
    } else {
        d + k(cx, "back_add_far")
    }
}

/// `func_30B6(p, speed, arrive, time)`: fly to `p` (facing it) until within `arrive` or the time is
/// out; within `fly_near` of the target: the back-off and done; stuck after `fly_check` frames: 20
/// frames and the end.
async fn fly_to(cx: &Cx, p: Vec3, speed: f32, arrive: f32, time: f32) {
    let mut t = time;
    let check = t - k(cx, "fly_check");
    while t > 0.0 {
        if dist_to(cx, tgt(cx)) - k(cx, "fly_near") < 0.0 {
            back_off(cx, k(cx, "back_speed"), k(cx, "back_far"), req::RISE).await;
            cx.borrow_mut().x.done = true;
            return;
        }
        let mut v = p - me(cx);
        yaw_along(cx, v);
        if v.length() - arrive < 0.0 {
            frame().await;
            return;
        }
        if v.y > 0.0 {
            if me(cx).y - p.y > 0.0 {
                v.y = 0.0;
            } else if height(cx) - k(cx, "fly_floor") > 0.0 {
                v = v * speed;
            }
        }
        set_dir(cx, norm(v));
        go(cx, speed);
        frame().await;
        if t - check < 0.0 && stuck(cx, speed * k(cx, "fly_stuck")) {
            wait(cx, k(cx, "fly_stuck_wait")).await;
            return;
        }
        t -= step(cx);
    }
}

/// `func_2E7E(target, speed, near)`: fly at a point `approach_over` above the target (the target
/// itself while it is in the air over `approach_top` above the floor), facing it, until within `near`
/// of the point or of the target; stuck after `approach_check` frames: rise 100 at
/// `approach_rise_speed`, the end.
async fn approach(cx: &Cx, speed: f32, near: f32) {
    let mut t = 0.0;
    loop {
        let tg = tgt(cx);
        let mut q = tg;
        if !(!target_ground(cx) && rel_y(cx, tg.y) - k(cx, "approach_top") < 0.0) {
            q.y -= k(cx, "approach_over");
        }
        let mut v = q - me(cx);
        yaw_to(cx, tg);
        let d = dist_to(cx, tg);
        if v.length() - near < 0.0 || d - near < 0.0 {
            halt(cx);
            return;
        }
        if v.y > 0.0 {
            if me(cx).y - q.y > 0.0 {
                v.y = 0.0;
            } else if height(cx) - k(cx, "approach_floor") > 0.0 {
                yaw_to(cx, tgt(cx));
                v = v * speed;
            }
        }
        set_dir(cx, norm(v));
        go(cx, speed);
        frame().await;
        if t - k(cx, "approach_check") > 0.0 && stuck(cx, speed * k(cx, "approach_stuck")) {
            let from = me(cx).y;
            rise(cx, k(cx, "approach_rise"), req::IDLE, k(cx, "approach_rise_speed"), from).await;
            return;
        }
        t += step(cx);
    }
}

/// `func_2A22(target, p, speed, arrive, time)`: the charge: near (no wind-up next time); toward `p`
/// (its x / z the target's while the target stands), facing it with the fly motion, banking (208 /
/// 209) while the target is off to a side, `time` frames at most; within `charge_near` of the target:
/// the swoop (over `charge_low` above it) or the slash, done; within `arrive` of the point: done;
/// stuck after `time − charge_check` frames: rise 300, done.
async fn charge(cx: &Cx, p: Vec3, speed: f32, arrive: f32, time: f32) {
    cx.borrow_mut().x.near = true;
    let mut t = time;
    // `sp+0x70`, the frames since the last bank: the script never clears it before its first test
    // (Assumption: the VM's fresh frame holds 0).
    let mut bank = 0.0;
    while t > 0.0 {
        let tg = tgt(cx);
        let mut v = p;
        if target_ground(cx) {
            v.x = tg.x;
            v.z = tg.z;
        }
        v = v - me(cx);
        let d = dist_to(cx, tg);
        if d - k(cx, "charge_near") < 0.0 {
            halt(cx);
            if me(cx).y - (tg.y - k(cx, "charge_low")) < 0.0 {
                swoop(cx, k(cx, "motion_blend")).await;
            } else {
                slash(cx).await;
            }
            let mut io = cx.borrow_mut();
            io.x.rounds += 1;
            io.x.near = false;
            io.x.fly = false;
            io.x.done = true;
            return;
        }
        if v.length() - arrive < 0.0 {
            halt(cx);
            cx.borrow_mut().x.done = true;
            return;
        }
        if !cx.borrow().x.fly {
            yaw_to(cx, tg);
            play(cx, req::FLY, k(cx, "motion_blend"));
            cx.borrow_mut().x.fly = true;
        } else {
            let f = facing(cx);
            let g = flat_to(cx, tg);
            let c1 = cosine(f, g);
            let c2 = cosine(rotate(f, k(cx, "bank_turn")), g);
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
        }
        if v.y > 0.0 {
            if me(cx).y - p.y > 0.0 {
                v.y = 0.0;
            } else if height(cx) - k(cx, "charge_floor") > 0.0 {
                v = v * speed;
            }
        }
        set_dir(cx, norm(v));
        go(cx, speed);
        frame().await;
        if t - k(cx, "charge_check") < 0.0 && stuck(cx, speed * k(cx, "charge_stuck")) {
            let from = me(cx).y;
            rise(cx, k(cx, "charge_rise"), req::RISE, k(cx, "rise_speed"), from).await;
            frame().await;
            let mut io = cx.borrow_mut();
            io.x.done = true;
            io.x.fly = false;
            return;
        }
        let s = step(cx);
        t -= s;
        bank += s;
    }
}

/// `func_2642(target)`: the climb to a target above: within `climb_near` the slash (a round); at or
/// over `climb_under` below it: the idle, near; else rising (204) at `climb_speed`; stuck: near.
async fn climb(cx: &Cx) {
    loop {
        let tg = tgt(cx);
        let mut p = tg;
        p.y += k(cx, "climb_under");
        if dist_to(cx, tg) - k(cx, "climb_near") < 0.0 {
            slash(cx).await;
            let mut io = cx.borrow_mut();
            io.x.rounds += 1;
            io.x.fly = false;
            return;
        }
        if me(cx).y - p.y <= 0.0 {
            play(cx, req::IDLE, k(cx, "motion_blend"));
            yaw_to(cx, tg);
            halt(cx);
            cx.borrow_mut().x.near = true;
            return;
        }
        yaw_to(cx, tg);
        set_dir(cx, Vec3::new(0.0, k(cx, "climb_up"), 0.0));
        go(cx, k(cx, "climb_speed"));
        play(cx, req::RISE, k(cx, "motion_blend"));
        frame().await;
        if stuck(cx, k(cx, "climb_stuck")) {
            halt(cx);
            {
                let mut io = cx.borrow_mut();
                io.x.near = true;
                io.x.fly = false;
            }
            frame().await;
            return;
        }
    }
}

/// `func_27CA(target)`: the dive at a target below: to `dive_above` above where it was, diving (206)
/// at `dive_speed`; from there 207, sinking at `dive_sink_speed` (only `+0x24` written: the direction's
/// x / z stay as they were) for its length; within `dive_near` of the target: the nudge, the swoop, a
/// round; near the floor (`dive_floor`) or stuck: stop. Near, done at the end.
async fn dive(cx: &Cx) {
    let mut p = tgt(cx);
    p.y -= k(cx, "dive_above");
    loop {
        if me(cx).y - p.y > 0.0 {
            play(cx, req::DIVE_END, k(cx, "motion_blend"));
            let mut t = motion_frames(cx, req::DIVE_END);
            while t > 0.0 {
                if dist_to(cx, tgt(cx)) - k(cx, "dive_near") < 0.0 {
                    nudge(cx).await;
                    swoop(cx, k(cx, "motion_blend")).await;
                    frame().await;
                    cx.borrow_mut().x.rounds += 1;
                    cx.borrow_mut().x.done = true;
                    break;
                }
                if height(cx) - k(cx, "dive_floor") <= 0.0 {
                    halt(cx);
                    break;
                }
                yaw_to(cx, tgt(cx));
                dir_y(cx, k(cx, "dive_sink"));
                go(cx, k(cx, "dive_sink_speed"));
                t -= step(cx);
                frame().await;
            }
            {
                let mut io = cx.borrow_mut();
                io.x.near = true;
                io.x.fly = false;
                io.x.done = true;
            }
            frame().await;
            return;
        }
        if height(cx) - k(cx, "dive_floor") <= 0.0 {
            halt(cx);
            {
                let mut io = cx.borrow_mut();
                io.x.near = true;
                io.x.fly = false;
            }
            frame().await;
            return;
        }
        yaw_to(cx, tgt(cx));
        set_dir(cx, Vec3::new(0.0, k(cx, "dive_down"), 0.0));
        go(cx, k(cx, "dive_speed"));
        play(cx, req::DIVE, k(cx, "motion_blend"));
        frame().await;
        if stuck(cx, k(cx, "dive_stuck")) {
            halt(cx);
            {
                let mut io = cx.borrow_mut();
                io.x.near = true;
                io.x.fly = false;
            }
            frame().await;
            return;
        }
    }
}

/// The wind-up (`0x2100`): before a long flight (farther than `far_windup`, or with the target less
/// than `windup_above` across in height) 210 for its length facing the target; then the take-off (201,
/// its length), the fly motion asked anew.
async fn take_off(cx: &Cx, windup: bool) {
    if windup {
        play(cx, req::WINDUP, k(cx, "motion_blend"));
        let mut t = motion_frames(cx, req::WINDUP);
        while t > 0.0 {
            yaw_to(cx, tgt(cx));
            t -= step(cx);
            frame().await;
        }
    }
    cx.borrow_mut().x.fly = false;
    play(cx, req::TAKE_OFF, k(cx, "motion_blend"));
    let n = motion_frames(cx, req::TAKE_OFF);
    wait(cx, n).await;
}

/// `func_2100(target)`: a far round. A target below: nearly straight below (`below_cos`) the dive
/// (after 205); fairly level (`level_cos`) the charge at a point `charge_drop` under it until done
/// (a yield first while it is in the air); else the charge at its x / z at the actor's height (`level_speed`).
/// A target above or level: nearly straight above (`above_cos`) the climb; fairly level
/// (`above_level_cos`) the charge at it until done; else the climb. The level charges take the wind-up
/// unless near.
async fn far(cx: &Cx) {
    let tg = tgt(cx);
    let m = me(cx);
    let v = tg - m;
    let f = Vec3::new(tg.x - m.x, 0.0, tg.z - m.z);
    yaw_to(cx, tg);
    if tg.y - m.y > 0.0 {
        let cd = cosine(Vec3::new(0.0, k(cx, "below_dir"), 0.0), v);
        let cf = cosine(f, v);
        if cd - k(cx, "below_cos") >= 0.0 {
            cx.borrow_mut().x.near = false;
            play(cx, req::DIVE_START, k(cx, "motion_blend"));
            let n = motion_frames(cx, req::DIVE_START);
            wait(cx, n).await;
            dive(cx).await;
            frame().await;
        } else if cf - k(cx, "level_cos") >= 0.0 {
            if !cx.borrow().x.near {
                let w = dist_to(cx, tg) - k(cx, "far_windup") > 0.0 || me(cx).y - (tg.y - k(cx, "windup_above")) > 0.0;
                take_off(cx, w).await;
            }
            loop {
                if !target_ground(cx) {
                    frame().await;
                }
                let mut p = tgt(cx);
                p.y += k(cx, "charge_drop");
                charge(cx, p, k(cx, "charge_speed"), k(cx, "charge_arrive"), k(cx, "charge_time")).await;
                if cx.borrow().x.done {
                    break;
                }
            }
        } else {
            if !cx.borrow().x.near {
                take_off(cx, false).await;
            }
            let mut p = tgt(cx);
            p.y = me(cx).y;
            charge(cx, p, k(cx, "level_speed"), k(cx, "charge_arrive"), k(cx, "charge_time")).await;
            frame().await;
        }
    } else {
        let cu = cosine(Vec3::new(0.0, k(cx, "above_dir"), 0.0), v);
        let cf = cosine(f, v);
        if cu - k(cx, "above_cos") >= 0.0 {
            cx.borrow_mut().x.near = false;
            climb(cx).await;
            frame().await;
        } else if cf - k(cx, "above_level_cos") >= 0.0 {
            if !cx.borrow().x.near {
                let w = dist_to(cx, tg) - k(cx, "far_windup") > 0.0 || tg.y - (me(cx).y - k(cx, "windup_above")) > 0.0;
                take_off(cx, w).await;
            }
            loop {
                let p = tgt(cx);
                charge(cx, p, k(cx, "charge_speed"), k(cx, "charge_arrive"), k(cx, "charge_time")).await;
                if cx.borrow().x.done {
                    break;
                }
            }
        } else {
            cx.borrow_mut().x.near = false;
            climb(cx).await;
            frame().await;
        }
    }
}

/// `func_5A4`: far rounds until one is done.
async fn far_round(cx: &Cx) {
    loop {
        far(cx).await;
        let done = cx.borrow().x.done;
        if done {
            cx.borrow_mut().x.done = false;
            return;
        }
    }
}

/// `func_5DA`: mid range: the idle, the approach to `approach_near` above the target, the swoop (a
/// round), the back-off.
async fn mid(cx: &Cx) {
    cx.borrow_mut().x.near = false;
    play(cx, req::IDLE, k(cx, "motion_blend"));
    approach(cx, k(cx, "approach_speed"), k(cx, "approach_near")).await;
    swoop(cx, k(cx, "motion_blend")).await;
    {
        let mut io = cx.borrow_mut();
        io.x.rounds += 1;
        io.x.fly = false;
    }
    frame().await;
    let far = back_far(cx);
    back_off(cx, k(cx, "back_speed"), far, req::RISE).await;
}

/// `func_69C`: close: at random the back-off (then a rise of `close_rise` when low over the floor or
/// over the target), or the swoop (blend 0, a round), a flight to a point `roam_min..max` away and the
/// back-off; after `round_limit` rounds the idle state.
async fn close(cx: &Cx) -> Flow {
    cx.borrow_mut().x.near = false;
    if rand(cx, "close") - k(cx, "close_chance") > 0.0 {
        let far = back_far(cx);
        back_off(cx, k(cx, "back_speed"), far, req::RISE).await;
        let m = me(cx);
        if rel_y(cx, m.y) - k(cx, "rise_floor") > 0.0 || m.y - (tgt(cx).y - k(cx, "rise_above")) > 0.0 {
            rise(cx, k(cx, "close_rise"), req::RISE, k(cx, "rise_speed"), m.y).await;
            frame().await;
        }
    } else {
        swoop(cx, k(cx, "close_swoop_blend")).await;
        {
            let mut io = cx.borrow_mut();
            io.x.rounds += 1;
            io.x.fly = false;
        }
        let p = random_offset(cx, k(cx, "roam_min"), k(cx, "roam_max")) + me(cx);
        fly_to(cx, p, k(cx, "roam_speed"), k(cx, "roam_arrive"), k(cx, "roam_time")).await;
        let done = cx.borrow().x.done;
        if done {
            cx.borrow_mut().x.done = false;
        } else {
            frame().await;
            let far = back_far(cx);
            back_off(cx, k(cx, "back_speed"), far, req::RISE).await;
        }
    }
    if cx.borrow().x.rounds as f32 - k(cx, "round_limit") >= 0.0 {
        set_state(cx, st::IDLE);
        return END;
    }
    ON
}

/// `func_358A(time, turn)`: the circle: the facing turned 90° to a side (at random; 208 with −90°, 209
/// with +90°), the turn halved (and signed for the side), the take-off to its end, then `time` frames
/// flying along the direction at `circle_speed` while it turns by the turn a frame, facing it; the
/// halt (203, an action, the idle queued).
async fn circle(cx: &Cx, time: f32, turn: f32) {
    let mut f = facing(cx);
    let mut turn = turn * k(cx, "circle_half");
    let side = if rand(cx, "circle side") - k(cx, "circle_chance") > 0.0 {
        f = rotate(f, k(cx, "circle_left"));
        f = rotate(f, turn * step(cx));
        req::BANK_L
    } else {
        f = rotate(f, k(cx, "circle_right"));
        turn *= k(cx, "circle_flip");
        f = rotate(f, turn * step(cx));
        req::BANK_R
    };
    yaw_along(cx, f);
    play(cx, req::TAKE_OFF, k(cx, "motion_blend"));
    wait_motion(cx).await;
    let mut t = time;
    while t > 0.0 {
        f = rotate(f, turn * step(cx));
        yaw_along(cx, f);
        set_dir(cx, f);
        go(cx, k(cx, "circle_speed"));
        play(cx, side, k(cx, "motion_blend"));
        t -= step(cx);
        frame().await;
    }
    halt(cx);
    push(cx, ECmd::Action { request: req::HALT, attack: None, then: req::IDLE, blend: k(cx, "motion_blend") });
    frame().await;
}

/// The appearance (`0xA6E` → `0x195A`, every spawn but type 5): face Sora, 69 (blend 0) with a fade-in,
/// `appear_wait` frames, then (actor flag 4, the idle queued) until 69 ends; the idle state.
async fn appear(cx: Cx) {
    yaw_to(&cx, sora(&cx));
    if cx.borrow().x.spawned {
        play(&cx, req::APPEAR, k(&cx, "appear_blend"));
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        wait(&cx, k(&cx, "appear_wait")).await;
        push(&cx, ECmd::Queue { request: req::IDLE, blend: 0.0 });
        frame().await;
        while cx.borrow().w.motion == Some(req::APPEAR) {
            frame().await;
        }
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The idle behaviour (`0x8D2`): the hits forgotten; Sora found (always, in a Re:CoM battle): the
/// target, a yield, a rise of `idle_rise` when under `idle_above` above him, the circle, the engaged
/// state. Not found: the circle, the idle (blend 0) facing Sora for `rest_lo..hi` frames, again.
async fn idle(cx: Cx) {
    cx.borrow_mut().x.hits = 0;
    loop {
        if search(&cx, k(&cx, "search_range")) {
            set_target(&cx, true);
            frame().await;
            let m = me(&cx);
            if m.y - (tgt(&cx).y - k(&cx, "idle_above")) > 0.0 {
                rise(&cx, k(&cx, "idle_rise"), req::RISE, k(&cx, "rise_speed"), m.y).await;
                frame().await;
            }
            let time = rand_range(&cx, k(&cx, "circle_time_lo"), k(&cx, "circle_time_hi"), "circle time");
            let turn = rand_range(&cx, k(&cx, "circle_turn_lo"), k(&cx, "circle_turn_hi"), "circle turn");
            circle(&cx, time, turn).await;
            {
                let mut io = cx.borrow_mut();
                io.x.rounds = 0;
                io.x.fly = false;
                io.x.near = false;
            }
            frame().await;
            set_state(&cx, st::ENGAGED);
            return;
        }
        let time = rand_range(&cx, k(&cx, "circle_time_lo"), k(&cx, "circle_time_hi"), "circle time");
        let turn = rand_range(&cx, k(&cx, "circle_turn_lo"), k(&cx, "circle_turn_hi"), "circle turn");
        circle(&cx, time, turn).await;
        play(&cx, req::IDLE, k(&cx, "rest_blend"));
        let mut t = rand_range(&cx, k(&cx, "rest_lo"), k(&cx, "rest_hi"), "rest");
        while t > 0.0 {
            yaw_to(&cx, sora(&cx));
            t -= step(&cx);
            frame().await;
        }
        frame().await;
    }
}

/// The engaged behaviour (`0x4A2`): by the distance to the target, the far rounds (beyond
/// `engage_mid`, or beyond `engage_close` and more than `high_above` over it), mid, or close; a yield;
/// again (the search re-picks Sora).
async fn engaged(cx: Cx) {
    loop {
        let d = dist_to(&cx, tgt(&cx));
        if d - k(&cx, "engage_far") > 0.0 || d - k(&cx, "engage_mid") > 0.0 {
            far_round(&cx).await;
        } else if d - k(&cx, "engage_close") > 0.0 {
            if me(&cx).y - (tgt(&cx).y - k(&cx, "high_above")) < 0.0 {
                far_round(&cx).await;
            } else {
                mid(&cx).await;
            }
        } else if close(&cx).await == END {
            return;
        }
        frame().await;
    }
}

/// The hit behaviour (`0x274`): hit in the dive: 211 (blend 0) for its length; launched: 8 frames;
/// then close (`hit_close`): with 1..5 hits the swoop (over `hit_high` above the floor) or the slash,
/// with 5..10 the back-off, else the idle state; within `hit_mid`: with 5..10 hits the back-off, from
/// 10 the idle state; then a yield and the engaged state (the round count cleared).
async fn hit(cx: Cx) {
    if cx.borrow().x.dive_hit {
        cx.borrow_mut().x.dive_hit = false;
        play(&cx, req::DIVE_HIT, k(&cx, "dive_hit_blend"));
        let n = motion_frames(&cx, req::DIVE_HIT);
        wait(&cx, n).await;
    }
    if cx.borrow().x.launched {
        cx.borrow_mut().x.launched = false;
        wait(&cx, k(&cx, "hit_wait")).await;
    }
    let d = dist_to(&cx, tgt(&cx));
    let hits = cx.borrow().x.hits as f32;
    let (lo, mid, hi) = (k(&cx, "hits_lo"), k(&cx, "hits_mid"), k(&cx, "hits_hi"));
    if d - k(&cx, "hit_close") < 0.0 {
        if hits - lo >= 0.0 && hits - mid < 0.0 {
            if height(&cx) - k(&cx, "hit_high") > 0.0 {
                swoop(&cx, k(&cx, "motion_blend")).await;
            } else {
                slash(&cx).await;
            }
            frame().await;
        } else if hits - mid >= 0.0 && hits - hi < 0.0 {
            back_off(&cx, k(&cx, "back_speed"), k(&cx, "back_far"), req::RISE).await;
            frame().await;
        } else {
            set_state(&cx, st::IDLE);
            return;
        }
    } else if d - k(&cx, "hit_mid") < 0.0 {
        if hits - mid >= 0.0 && hits - hi < 0.0 {
            back_off(&cx, k(&cx, "back_speed"), k(&cx, "back_far"), req::RISE).await;
            frame().await;
        } else if hits - hi >= 0.0 {
            set_state(&cx, st::IDLE);
            return;
        }
    }
    frame().await;
    {
        let mut io = cx.borrow_mut();
        io.x.fly = false;
        io.x.near = false;
        io.x.rounds = 0;
    }
    set_state(&cx, st::ENGAGED);
}

/// The supervisor (entry `0x1CF6`): after the init's yield, each frame a state request restarts the
/// behaviour (the target set for engaged and hit, cleared for idle).
async fn supervisor(cx: Cx) {
    frame().await;
    if !cx.borrow().x.spawned {
        // Re:CoM's own appearance played (its first-strike stun): the appearance thread's end.
        yaw_to(&cx, sora(&cx));
        cx.borrow_mut().x.appeared = true;
        set_state(&cx, st::IDLE);
    }
    loop {
        if let Some(s) = take_state(&cx) {
            set_target(&cx, s != st::IDLE);
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Air Pirate's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as the Shadow's: per-enemy seeds).
    /// `frames`: KH1 motion lengths by request. `spawned`: its appearance plays (every Re:CoM battle
    /// appearance, owner 2026-11-02).
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
            // `0x1D2A`: flight with no velocity.
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
    /// The hit handler (`0x1F0C`): while the dive 206 plays a hit (not a clash) takes no reaction and
    /// asks for 211 (`0x1F86..0x2002`), and every hit counts; otherwise a launch (kind 5) counts and
    /// asks for the pause, any other hit but a clash (kinds 6 / 7) counts (`0x201C..0x20CC`); the hit
    /// state in every case (KH1's attribute-8 / 9 / 11 exceptions: not carried, Assumption).
    fn on_hit(&self, news: HitNews) {
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
            } else if !clash && io.x.kind == 5 {
                io.x.launched = true;
                io.x.hits += 1;
            } else if !clash {
                io.x.hits += 1;
            }
        }
        set_state(&self.io, st::HIT);
    }
    /// The fallback when the host couldn't take the hit on the armour first ([`Behaviour::armoured`]).
    fn keeps_reaction(&self) -> bool {
        self.io.borrow().x.keep
    }
    /// The handler's no-reaction case, asked before Re:CoM's handler (M11, owner 2026-11-02: "move onto the
    /// no-reaction armour"): `0x1F72`: while 206 plays (`sys 51`, the request playing), a hit that is not a
    /// clash (6 / 7; attributes 8 / 9 aren't carried) has its reaction kind zeroed. Unknown kind: not here.
    fn armoured(&self, playing: Option<u16>, _launch_base: u16, kind: Option<u8>) -> bool {
        playing == Some(req::DIVE) && kind.is_some_and(|k| !crate::enemy::host::is_clash(k))
    }
    /// That hit's handler (`0x1FF4..0x2016`, `0x20DA`): the kind zeroed (no push: no `sys 42`, a kind-0
    /// record plays nothing), the dive marked (`wp+0x4C`), the hit counted, the hit state asked for (its
    /// behaviour plays 211). Re:CoM's state stays (the dive is a plain motion in state 1); the hit state's
    /// 211 replaces 206 at the next AI frame, as KH1's state request does.
    fn on_armoured_hit(&self, _dir: Vec3) {
        {
            let mut io = self.io.borrow_mut();
            io.x.keep = true;
            io.x.dive_hit = true;
            io.x.hits += 1;
        }
        set_state(&self.io, st::HIT);
    }
    fn risen(&self) -> bool {
        self.io.borrow().x.appeared
    }
    fn skip_rise(&self) {
        self.io.borrow_mut().x.spawned = false;
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["none", "engaged", "idle", "hit"].get(s as usize).copied().unwrap_or("?")
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
    /// The host's quiet watchdog: engaged again (as the supervisor's state request does), flying with
    /// no velocity, the round's flags cleared.
    fn reset(&self) {
        {
            let mut io = self.io.borrow_mut();
            io.x.appeared = true;
            io.x.done = false;
            io.x.near = false;
            io.x.fly = false;
            io.x.dive_hit = false;
            io.x.launched = false;
            io.body.fly(Vec3::ZERO);
        }
        set_state(&self.io, st::ENGAGED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::host::model::Model;

    /// Test numbers: the player's script's own (the asset step reads them; the disassembly's values),
    /// the shared engine's.
    pub(crate) fn consts() -> Consts {
        let v: &[(&str, f32)] = &[
            ("search_range", 5000.0), ("motion_blend", 8.0), ("appear_blend", 0.0), ("rest_blend", 0.0), ("dive_hit_blend", 0.0), ("close_swoop_blend", 0.0),
            ("bank_blend", 15.0), ("appear_fade", 30.0), ("appear_wait", 30.0), ("hit_wait", 8.0), ("hit_close", 150.0), ("hit_mid", 500.0), ("hit_high", 50.0),
            ("hits_lo", 1.0), ("hits_mid", 5.0), ("hits_hi", 10.0), ("engage_far", 1500.0), ("engage_mid", 500.0), ("engage_close", 200.0), ("high_above", 300.0),
            ("approach_speed", 10.0), ("approach_near", 180.0), ("back_speed", 18.0), ("back_far", 600.0), ("back_near", 600.0), ("back_add_near", 600.0),
            ("back_add_far", 300.0), ("close_chance", 0.5), ("rise_floor", -170.0), ("rise_above", 100.0), ("close_rise", -200.0), ("rise_speed", 16.0),
            ("roam_min", 500.0), ("roam_max", 700.0), ("roam_speed", 6.0), ("roam_arrive", 100.0), ("roam_time", 500.0), ("round_limit", 10.0),
            ("circle_time_lo", 250.0), ("circle_time_hi", 500.0), ("circle_turn_lo", 3.0), ("circle_turn_hi", 5.0), ("rest_lo", 90.0), ("rest_hi", 120.0),
            ("idle_above", 170.0), ("idle_rise", -170.0), ("slash_tail", 60.0), ("swoop_low", 50.0), ("swoop_settle", 100.0), ("swoop_settle_speed", 4.0),
            ("swoop_high", 80.0), ("swoop_sink", 1.0), ("swoop_sink_speed", 4.0), ("swoop_tail", 80.0), ("swoop_near", 200.0), ("swoop_speed", 8.0),
            ("below_dir", 1.0), ("above_dir", -1.0), ("below_cos", 0.9), ("level_cos", 0.8), ("above_cos", 0.76), ("above_level_cos", 0.86), ("far_windup", 1000.0),
            ("windup_above", 170.0), ("charge_drop", 30.0), ("charge_speed", 24.0), ("charge_arrive", 200.0), ("charge_time", 60.0), ("level_speed", 16.0),
            ("climb_under", 100.0), ("climb_near", 150.0), ("climb_up", -1.0), ("climb_speed", 16.0), ("climb_stuck", 4.0), ("dive_above", 320.0),
            ("dive_near", 150.0), ("dive_floor", 100.0), ("dive_sink", 1.0), ("dive_sink_speed", 8.0), ("dive_down", 1.0), ("dive_speed", 16.0), ("dive_stuck", 4.0),
            ("charge_near", 200.0), ("charge_low", 50.0), ("bank_turn", -90.0), ("bank_cos", 0.98), ("bank_hold", 30.0), ("bank_face", 60.0), ("charge_floor", 100.0),
            ("charge_check", 50.0), ("charge_stuck", 0.25), ("charge_rise", -300.0), ("approach_top", -100.0), ("approach_over", 100.0), ("approach_floor", 100.0),
            ("approach_check", 10.0), ("approach_stuck", 0.1), ("approach_rise", -100.0), ("approach_rise_speed", 6.0), ("fly_check", 30.0), ("fly_near", 100.0),
            ("fly_floor", 100.0), ("fly_stuck", 0.25), ("fly_stuck_wait", 20.0), ("nudge_range", 100.0), ("nudge_y", 0.2), ("nudge_z", -1.0), ("nudge_speed", 8.0),
            ("back_chance", 0.5), ("back_turn", 180.0), ("back_turn_other", -180.0), ("back_wait", 10.0), ("back_check", 10.0), ("back_stuck", 0.25),
            ("back_close", 150.0), ("back_close_speed", 6.0), ("back_rise", -300.0), ("circle_half", 0.5), ("circle_chance", 0.5), ("circle_left", -90.0),
            ("circle_right", 90.0), ("circle_flip", -1.0), ("circle_speed", 16.0), ("rise_up", -1.0), ("rise_stuck", 0.1), ("attack_blend", 8.0),
            ("walk_speed", 2.0), ("run_speed", 8.0), ("air_accel", 0.1), ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0), ("launch_min", 20.0),
            ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("down_time", 30.0),
            ("air_knock_hop", 20.0), ("weight", 1.0), ("gravity", crate::params::GRAVITY), ("fall_cap", 20.0),
        ];
        Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), NAMES).unwrap()
    }

    fn frames() -> Vec<(u16, f32)> {
        use req::*;
        vec![
            (IDLE, 128.0), (APPEAR, 78.0), (SWOOP, 122.0), (TAKE_OFF, 30.0), (FLY, 64.0), (HALT, 50.0), (RISE, 64.0), (DIVE_START, 44.0), (DIVE, 64.0), (DIVE_END, 46.0),
            (BANK_L, 64.0), (BANK_R, 64.0), (WINDUP, 110.0), (DIVE_HIT, 86.0), (SLASH, 100.0), (RECOIL, 80.0), (FLINCH, 46.0), (KNOCK, 86.0),
        ]
    }

    /// A flat floor at y = 0 (KH1 y down), Sora on it (or in the air); the host plays motions to their
    /// length and queues, pauses the AI for actions (the circle's halt), keeps polling it during the
    /// steered attacks; the body flies by KH1's flight step, never through the floor.
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
    }

    #[test]
    fn the_names_cover_the_numbers() {
        assert!(Table::new(vec![], NAMES).is_err());
        let c = consts();
        assert_eq!((c.num("charge_speed"), c.num("dive_above"), c.num("round_limit")), (24.0, 320.0, 10.0));
    }

    /// The battle spawn: KH1's appearance (69) facing Sora; then idle: Sora found, a rise to 170 above
    /// him, the circle (201 to its end, a bank motion, the halt 203 as an action), engaged.
    #[test]
    fn appears_rises_circles_and_engages() {
        let mut s = Sim::new(7, true, Vec3::new(0.0, -120.0, 0.0));
        let sora = |_| (Vec3::new(0.0, 0.0, 800.0), true);
        s.run(sora, 3);
        assert!(s.plays(req::APPEAR));
        assert!(s.ai.body().flying);
        s.run(sora, 90);
        assert!(s.ai.risen(), "risen after 69");
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::IDLE))));
        s.run(sora, 60);
        assert!(s.pos.y < -160.0, "rose to about 170 above Sora: {}", s.pos.y);
        s.run(sora, 700);
        assert!(s.plays(req::TAKE_OFF) && (s.plays(req::BANK_L) || s.plays(req::BANK_R)));
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::Action { request: req::HALT, .. })), "the circle's halt");
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))));
    }

    /// The circle flies a curve: the turn of `circle_turn` × 0.5 degrees a frame at 16.
    #[test]
    fn the_circle_curves() {
        let mut s = Sim::new(11, false, Vec3::new(0.0, -300.0, 0.0));
        let sora = |_| (Vec3::new(0.0, 0.0, 400.0), true);
        // Through the idle's search and the take-off into the circle.
        s.run(sora, 60);
        let start = s.pos;
        let mut far = 0.0f32;
        for _ in 0..200 {
            s.run(sora, 1);
            far = far.max((s.pos - start).horizontal_length());
        }
        let end = (s.pos - start).horizontal_length();
        assert!(far > 400.0, "it flies: {far}");
        assert!(end < far, "it curves back: {end} < {far}");
    }

    /// Mid range (2–5 m, not high above): the approach, the swoop (200 steered), the back-off.
    #[test]
    fn mid_range_swoops_then_backs_off() {
        let mut s = Sim::new(5, false, Vec3::new(0.0, -200.0, 0.0));
        let sora = |_| (Vec3::new(0.0, 0.0, 380.0), true);
        s.run(sora, 2500);
        let at = s.attacks();
        assert!(at.iter().any(|a| a.1 == Attack::Swoop), "{at:?}");
        assert!(s.plays(req::FLY) && s.plays(req::HALT), "the back-off flies, then halts");
    }

    /// A target straight below (a far round): 205, the dive (206) to 320 above him, its last stretch
    /// (207) sinking to him, the nudge, the swoop.
    #[test]
    fn straight_below_dives_then_swoops() {
        let mut s = Sim::new(3, false, Vec3::new(0.0, -900.0, 0.0));
        let sora = |_| (Vec3::new(30.0, 0.0, 60.0), true);
        s.run(sora, 2);
        let cx = s.ai.io.clone();
        s.ai.behaviour = Some(Box::pin(async move { far_round(&cx).await }));
        s.log.clear();
        s.run(sora, 400);
        let order: Vec<u16> = s.log.iter().filter_map(|(_, c)| match c {
            ECmd::Motion { request, .. } | ECmd::Attack { request, .. } => Some(*request),
            _ => None,
        }).collect();
        let at = |r: u16| order.iter().position(|&x| x == r).unwrap_or_else(|| panic!("{r} not in {order:?}"));
        assert!(at(req::DIVE_START) < at(req::DIVE) && at(req::DIVE) < at(req::DIVE_END) && at(req::DIVE_END) < at(req::SWOOP), "{order:?}");
        assert!(s.attacks().iter().any(|a| a.1 == Attack::Swoop));
        assert!(s.pos.y > -150.0, "it came down to him: {}", s.pos.y);
    }

    /// The no-reaction armour (owner 2026-11-02): while 206 plays a hit that isn't a clash is taken without a
    /// reaction (asked before Re:CoM's handler); the handler's counts and its hit state follow, no push.
    #[test]
    fn the_dive_on_the_armour() {
        let s = Sim::new(1, false, Vec3::new(0.0, -200.0, 0.0));
        assert!(s.ai.armoured(Some(req::DIVE), 0, Some(3)) && s.ai.armoured(Some(req::DIVE), 0, Some(5)));
        for (p, k) in [(Some(req::DIVE), Some(6)), (Some(req::DIVE), Some(7)), (Some(req::DIVE), None), (Some(req::DIVE_START), Some(3)), (Some(req::DIVE_END), Some(3)), (None, Some(3))] {
            assert!(!s.ai.armoured(p, 0, k), "{p:?} {k:?}");
        }
        s.ai.io.borrow_mut().state.1 = false;
        s.ai.set_hit_kind(3);
        s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
        let io = s.ai.io.borrow();
        assert!(io.x.dive_hit && io.x.hits == 1 && io.state.1 && io.body.push.is_none(), "counted, the hit state, no push");
    }

    /// The dive through the host's rules (`crate::enemy::host::model`, its class map: swoop 3, slash 2): 206
    /// plays in Re:CoM's state 1 (a plain motion); a hit on the armour leaves that state alone and 211 plays
    /// in state 1 from the next frame, for its length; nothing is released.
    #[test]
    fn hosted_a_hit_in_the_dive_plays_211_in_state_1() {
        let mut s = Sim::new(3, false, Vec3::new(0.0, -900.0, 0.0));
        let sora = |_| (Vec3::new(30.0, 0.0, 60.0), true);
        s.run(sora, 2);
        let cx = s.ai.io.clone();
        s.ai.behaviour = Some(Box::pin(async move { far_round(&cx).await }));
        let Sim { ai, pos, .. } = s;
        let mut m = Model::new(Box::new(ai), consts(), frames(), vec![], vec![3, 2]);
        m.pos = pos;
        let at = Vec3::new(30.0, 0.0, 60.0);
        for _ in 0..400 {
            if m.current == Some(req::DIVE) && m.age > 2.0 {
                break;
            }
            m.run(at, 1);
        }
        assert_eq!((m.state, m.current), (1, Some(req::DIVE)), "diving");
        assert!(!m.armoured_hit(6, Vec3::new(0.0, 0.0, -1.0)), "a clash is not armoured");
        assert!(m.armoured_hit(3, Vec3::new(0.0, 0.0, -1.0)));
        m.run(at, 1);
        assert_eq!((m.state, m.current), (1, Some(req::DIVE_HIT)), "211 at once, in state 1");
        let from = m.trace.len();
        m.run(at, 120);
        let run = m.longest_run(req::DIVE_HIT);
        assert!(run >= 85, "211 for its 86 frames: {run}");
        assert!(m.trace[from..].iter().all(|t| t.0 == 1 || t.1 != Some(req::DIVE_HIT)), "211 only in state 1");
        assert_eq!(m.released, 0);
    }

    /// A target high above: the climb (204) and the slash.
    #[test]
    fn a_target_above_gets_the_climb_and_slash() {
        let mut s = Sim::new(9, false, Vec3::new(0.0, -100.0, 0.0));
        // Sora on a high ledge 6 m up, 1 m away flat.
        let sora = |_| (Vec3::new(0.0, -600.0, 100.0), true);
        s.run(sora, 3000);
        assert!(s.plays(req::RISE));
        assert!(s.attacks().iter().any(|a| a.1 == Attack::Slash), "{:?}", s.attacks());
    }

    /// The hit handler: a hit in the dive takes no reaction and asks for 211; a launch for the pause;
    /// a clash outside the dive is not counted; ten hits within 5 m: the idle state.
    #[test]
    fn the_hit_handler() {
        let mut s = Sim::new(1, false, Vec3::new(0.0, -200.0, 0.0));
        s.run(|_| (Vec3::new(0.0, 0.0, 300.0), true), 5);
        s.ai.io.borrow_mut().w.motion = Some(req::DIVE);
        s.ai.set_hit_kind(3);
        s.ai.on_hit(HitNews::Damage);
        assert!(!s.ai.keeps_reaction() && s.ai.io.borrow().x.dive_hit);
        assert_eq!(s.ai.io.borrow().x.hits, 1);
        s.ai.io.borrow_mut().w.motion = Some(req::IDLE);
        s.ai.set_hit_kind(5);
        s.ai.on_hit(HitNews::Damage);
        assert!(s.ai.keeps_reaction() && s.ai.io.borrow().x.launched);
        s.ai.set_hit_kind(6);
        s.ai.on_hit(HitNews::Clash);
        assert_eq!(s.ai.io.borrow().x.hits, 2, "a clash outside the dive is not counted");
        // The hit behaviour: 211 first.
        s.log.clear();
        s.run(|_| (Vec3::new(0.0, 0.0, 300.0), true), 3);
        assert!(s.plays(req::DIVE_HIT), "{:?}", s.log);
        // Ten hits close: the idle state.
        s.ai.io.borrow_mut().x.hits = 9;
        s.ai.set_hit_kind(3);
        s.ai.on_hit(HitNews::Damage);
        s.log.clear();
        // The launch's 8-frame pause (still pending), then 5 m: the idle state.
        s.run(|_| (Vec3::new(0.0, 0.0, 300.0), true), 8);
        assert!(!s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::IDLE))), "the pause first");
        s.run(|_| (Vec3::new(0.0, 0.0, 300.0), true), 4);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::IDLE))), "{:?}", s.log);
    }

    /// The quiet watchdog's reset: engaged, flying, the round flags cleared.
    #[test]
    fn reset_engages_again() {
        let mut s = Sim::new(2, false, Vec3::new(0.0, -200.0, 0.0));
        s.run(|_| (Vec3::new(0.0, 0.0, 300.0), true), 200);
        s.ai.io.borrow_mut().x.done = true;
        s.ai.reset();
        s.log.clear();
        s.run(|_| (Vec3::new(0.0, 0.0, 300.0), true), 3);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))));
        assert!(!s.ai.io.borrow().x.done && s.ai.body().flying);
    }

    /// No stall: Sora circling 2–9 m away for 20 000 frames (12 seeds), jumping now and then, hit now and
    /// then: it keeps acting (an attack at least every 2000 frames), steers or plays something at least
    /// every 600, never goes under the floor, and uses both attacks.
    #[test]
    fn a_long_fight_never_stalls() {
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
                if f == next_hit {
                    next_hit += 150 + (f * 7919 + seed as usize * 104_729) % 250;
                    if s.action.is_none() {
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
            assert_eq!(kinds.len(), 2, "seed {seed}: {kinds:?}");
        }
    }
}
