//! KH1 FM's Powerwild (`xa_ex_2030.mdls` part 5, `ex_2030.bd`), engine-agnostic: its AI script on
//! the shared enemy framework (`crate::enemy`). M11 (`docs/recom-m11-enemies.md` § The Powerwild).
//! Re:CoM runs it natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s
//! `win::kh1_enemy`), Re:CoM plays the motions, collides the body and the attacks, applies the
//! damage.
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2030.bd` at
//! checked offsets); nothing of the game is in this file. Script offsets are the player's
//! `ex_2030.bd`'s (Checked (files): the script disassembled).
//!
//! # The script's shape
//!
//! - supervisor `0x2262`: init `0x2346` (stats records `0x14` / `0x80`, walk 3 / run 7 / hop 500,
//!   home, the state object, the handlers), a yield, the appearance thread `0x7BC`; then each frame:
//!   while idle (state 1) Sora within `idle_search` → engaged; a state request restarts the
//!   behaviour thread: 1 idle `0x78C`, 2 engaged `0x5A2`, 3 hit `0x4B0`.
//! - the hit handler `0x2436`: any hit ends the first look; a clash (or kinds 8 / 10) asks for the
//!   counter; a damaging hit of kind 4 / 5 above `own_launch_hp` of its HP replaces KH1's reaction
//!   with its own launch (`sys 38`: base 270 / 280) and, after it, the shake-off 224; any other
//!   damaging hit asks for the counter.
//! - the hops `0xBEC` → `0x1152`: a crouch, then movement state 3 (`sys 82`, a jump of fixed time;
//!   `crate::enemy::Body::timed_jump`) with its own motion trio (`sys 83`).
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0x888` (`0x1002(1500, 700)`: KH1
//! removes it far from Sora), the scripted event 1001 (`0x23D6`: `wp+0x38` = 3 → motion 260, a
//! room's scripted scare; Assumption: no Re:CoM battle sends it), the scripted-room target rules
//! (`world 11`) and the death routine (Re:CoM's defeat, prizes, EXP).

use crate::enemy::react::ScriptedLaunch;
use crate::enemy::script::{approach, dist_to_target, face_target, face_target_now, frame, jump_at, motion_done, motion_frames, poll, pos, push, rand, rand_range, set_state, set_target, take_state, target, tick_clock, wait_frames, wait_grounded, wander};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Powerwild's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The appearance (`0x802`: `0x1000045`, an action), 60 frames.
    pub const APPEAR: u16 = 69;
    /// Strafing to either side (`0xA8E`), 40 frames each.
    pub const STRAFE_A: u16 = 200;
    pub const STRAFE_B: u16 = 201;
    /// The hops (`0xBEC`): the crouch r, then r + 1 rising, r + 2 at the top, r + 3 landing.
    pub const HOP_LEFT: u16 = 202;
    pub const HOP_RIGHT: u16 = 207;
    pub const HOP_BACK: u16 = 215;
    /// The swipe (`0xE8C`), 62 frames, hit [38, 56).
    pub const SWIPE: u16 = 212;
    /// The flurry (`0xEC6`), 98 frames, three hit tracks.
    pub const FLURRY: u16 = 214;
    /// The pounce's strike in the air (`0xD34`: `0x10000DE`), 24 frames.
    pub const POUNCE: u16 = 222;
    /// The idle routine's stretch (`0x986`), 150 frames.
    pub const STRETCH: u16 = 223;
    /// The shake-off after its own launch (`0x94C`), 92 frames.
    pub const SHAKE: u16 = 224;
    /// The first look (`0x9A2`), 148 frames.
    pub const NOTICE: u16 = 227;
    /// The roar (`0x8D8`), 80 frames.
    pub const ROAR: u16 = 228;
    /// Its own launch's bases (`0x2436` → `sys 38`; ground / air), KH1's launch layout.
    pub const OWN_LAUNCH: u16 = 270;
    pub const OWN_AIR_LAUNCH: u16 = 280;
}

/// The Powerwild's attacks, in the order the host indexes them (the manifest's `attacks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    Swipe,
    Flurry,
    Pounce,
}

impl Attack {
    pub const ALL: [Attack; 3] = [Attack::Swipe, Attack::Flurry, Attack::Pounce];

    pub fn request(self) -> u16 {
        match self {
            Attack::Swipe => req::SWIPE,
            Attack::Flurry => req::FLURRY,
            Attack::Pounce => req::POUNCE,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Swipe => "swipe",
            Attack::Flurry => "flurry",
            Attack::Pounce => "pounce",
        }
    }

    /// The attack rows (script offsets, 0x70 bytes, `sys 23`) made with it.
    pub fn rows(self) -> &'static [usize] {
        match self {
            Attack::Swipe => &[0x23C],
            Attack::Flurry => &[0xEC, 0x15C, 0x1CC],
            Attack::Pounce => &[0x2AC, 0x31C],
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The names of the Powerwild's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    "hop_height",
    "idle_search",
    "close_range",
    "mid_range",
    "hit_range",
    "hit_flurry_chance",
    "hit_hop_side_chance",
    "own_launch_hp",
    "own_launch_hop",
    "own_launch_speed",
    "own_launch_down",
    "close_side_chance",
    "mid_pause_chance",
    "mid_hop_strafe_chance",
    "far_idle_chance",
    "far_idle_min",
    "far_idle_max",
    "far_swipe_chance",
    "far_roar_chance",
    "approach_timeout",
    "approach_retarget",
    "approach_dist",
    "strafe_speed",
    "strafe_min",
    "strafe_max",
    "strafe_turn",
    "strafe_stop",
    "strafe_side_chance",
    "strafe_angle",
    "strafe_turn_factor",
    "pause_range",
    "pause_min",
    "pause_max",
    "pause_stop",
    "roar_stop",
    "hop_strafe_turn",
    "hop_strafe_side_chance",
    "hop_strafe_angle",
    "hop_strafe_min",
    "hop_strafe_max",
    "hop_strafe_speed",
    "hop_strafe_stop",
    "hop_back_angle",
    "hop_left_angle",
    "hop_right_angle",
    "hop_speed",
    "hop_jump_height",
    "hop_time",
    "hop_drag",
    "pounce_offset",
    "pounce_divisor",
    "pounce_max_speed",
    "pounce_height",
    "pounce_steer",
    "pounce_attack_after",
    "wander_inner",
    "wander_outer",
    "wander_band",
    "wander_timeout",
    "wander_idle_chance",
    "wander_idle_min",
    "wander_idle_max",
    "wander_home_max",
    "idle_blend",
    "motion_blend",
    "swipe_blend",
    "appear_fade",
    "appear_blend",
];

/// The Powerwild's numbers.
pub type Consts = Table;

/// The Powerwild's state objects (`wp+0x20`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 3;
}

/// What the hit behaviour does (`wp+0x38`, set by the hit handler `0x2436`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum After {
    /// 2: the counter (the flurry and the back hop, or a side hop and the pounce).
    #[default]
    Counter,
    /// 4: its own launch played: the shake-off 224.
    ShakeOff,
}

/// The Powerwild's own work area: first look done (`wp+0x3C`), the close-range alternation
/// (`wp+0x44`), what the hit behaviour does (`wp+0x38`), spawned by the room (`sys 132`), the
/// appearance done, Re:CoM's HP ratio and the last hit's kind (the handler's `0x1EB0` / `0x21F2`),
/// the launch it asked for.
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    noticed: bool,
    alternate: bool,
    after: After,
    spawned: bool,
    appeared: bool,
    hp_ratio: f32,
    hit_kind: u8,
    launch: Option<ScriptedLaunch>,
}

type Cx = crate::enemy::script::Cx<Consts, Attack, Wp>;
type Task = crate::enemy::script::Task;
pub type Cmd = ECmd<Attack>;

fn k(cx: &Cx, n: &str) -> f32 {
    cx.borrow().c.num(n)
}

/// A plain motion request; `sys 172`'s clock restarts for a request that plays anew.
fn play(cx: &Cx, r: u16, blend: f32) {
    {
        let mut io = cx.borrow_mut();
        if io.w.motion != Some(r) {
            io.clock = (Some(r), 0.0);
        }
    }
    push(cx, ECmd::Motion { request: r, blend });
}

/// `0x18E0(request, blend, queue blend)`: an action (bit 24) with the idle queued, then the
/// yield (the action pauses the script until it ends).
async fn action(cx: &Cx, r: u16, attack: Option<Attack>, blend: f32) {
    push(cx, ECmd::Action { request: r, attack, then: req::IDLE, blend });
    frame().await;
}

/// `0x1860`: until the motion is done (`sys 172`) or request 6 plays, facing the target.
async fn wait_done_or_air(cx: &Cx) {
    while !motion_done(cx) {
        if cx.borrow().w.motion == Some(req::AIR) {
            break;
        }
        face_target(cx);
        frame().await;
    }
}

fn grounded(cx: &Cx) -> bool {
    cx.borrow().w.grounded
}

/// `0xE8C`: the swipe (wait for the ground, face the target, action 212 at blend 2).
async fn swipe(cx: &Cx) {
    wait_grounded(cx).await;
    face_target(cx);
    action(cx, req::SWIPE, Some(Attack::Swipe), k(cx, "swipe_blend")).await;
}

/// `0xEC6`: the flurry (three rows, action 214).
async fn flurry(cx: &Cx) {
    wait_grounded(cx).await;
    face_target(cx);
    action(cx, req::FLURRY, Some(Attack::Flurry), k(cx, "attack_blend")).await;
}

/// `0xBEC(r)` → `0x1152(angle, speed, height, time, r)`: a hop. Face the target at once; the
/// crouch r for its length; r + 1 and movement state 3 (`sys 82`: the facing turned by the hop's
/// angle × speed, height, time, drag 0.9) with the motion base r − 3 (`sys 83`): six frames before
/// the top KH1 plays r + 2 and queues r + 3 (`0x1402AF630`, while r + 1 plays); facing the target
/// (`sys 11`) until it lands, then until on the ground.
async fn hop(cx: &Cx, r: u16) {
    let ang = match r {
        req::HOP_BACK => k(cx, "hop_back_angle"),
        req::HOP_LEFT => k(cx, "hop_left_angle"),
        _ => k(cx, "hop_right_angle"),
    };
    face_target_now(cx);
    let yaw = cx.borrow().body.yaw;
    let d = Vec3::from_yaw(yaw + ang.to_radians()) * k(cx, "hop_speed");
    let v = Vec3::new(d.x, k(cx, "hop_jump_height"), d.z);
    play(cx, r, 0.0);
    wait_frames(cx, motion_frames(cx, r)).await;
    // `(r + 1) | 0x10000`: bit 16 plays as a plain request here (Assumption, as the Soldier's fall).
    play(cx, r + 1, 0.0);
    let (drag, time) = (k(cx, "hop_drag"), k(cx, "hop_time"));
    cx.borrow_mut().body.timed_jump(v, drag, time);
    let mut apex = false;
    while cx.borrow().body.timed.is_some() {
        // `0x1402AF630` runs in the engine's step (after the script): here at the next frame's script
        // (Assumption: ±1 frame).
        let (at_top, cur) = {
            let io = cx.borrow();
            (io.body.timed_apex(), io.w.motion)
        };
        if !apex && at_top && cur == Some(r + 1) {
            apex = true;
            play(cx, r + 2, 0.0);
            push(cx, ECmd::Queue { request: r + 3, blend: 0.0 });
        }
        face_target_now(cx);
        frame().await;
    }
    while !grounded(cx) {
        frame().await;
    }
}

/// `0xD34`: the pounce. Turn to the target (`sys 36`), the jump start and a hop of `pounce_height`
/// at min((dist − offset) / divisor, max); in the air, steering at the target × `pounce_steer`;
/// from `pounce_attack_after` frames on, the strike 222 (an action) each time the script runs
/// while still in the air.
async fn pounce(cx: &Cx) {
    let Some(t) = target(cx) else { return };
    let (u, len) = (t - pos(cx)).normalize_len();
    cx.borrow_mut().body.yaw_target = u.x.atan2(u.z);
    let speed = ((len - k(cx, "pounce_offset")) / k(cx, "pounce_divisor")).min(k(cx, "pounce_max_speed"));
    jump_at(cx, k(cx, "pounce_height"), u * speed).await;
    let mut t = 0.0;
    while cx.borrow().w.airborne || t == 0.0 {
        if let Some(tg) = target(cx) {
            let (d, _) = (tg - pos(cx)).normalize_len();
            let mut io = cx.borrow_mut();
            io.body.yaw_target = d.x.atan2(d.z);
            io.steer = Some(d * k_io(&io.c, "pounce_steer"));
        }
        if t - k(cx, "pounce_attack_after") >= 0.0 {
            action(cx, req::POUNCE, Some(Attack::Pounce), 0.0).await;
        }
        t += cx.borrow().w.step;
        frame().await;
    }
}

fn k_io(c: &Consts, n: &str) -> f32 {
    c.num(n)
}

/// `0x13AA(stop, turn, n, speed, motion)` from `0xC80`: face the target; a walk sideways (±90° from
/// the facing, at random) for [`strafe_min`, `strafe_max`) frames, the direction turning back by
/// `strafe_turn × strafe_turn_factor` degrees a frame, at `strafe_speed`; it stops within
/// `strafe_stop` or off the ground; face the target.
async fn strafe(cx: &Cx) {
    face_target(cx);
    let mut n = rand_range(cx, k(cx, "strafe_min"), k(cx, "strafe_max"), "strafe time");
    let yaw = cx.borrow().body.yaw;
    let ang = k(cx, "strafe_angle").to_radians();
    let mut turn = k(cx, "strafe_turn");
    let mut dir = if rand(cx, "strafe side") - k(cx, "strafe_side_chance") < 0.0 {
        yaw - ang
    } else {
        turn = -turn;
        yaw + ang
    };
    turn *= k(cx, "strafe_turn_factor");
    let (speed, stop, blend) = (k(cx, "strafe_speed"), k(cx, "strafe_stop"), cx.borrow().move_blend);
    while n >= 0.0 {
        play(cx, req::WALK, blend);
        let step = cx.borrow().w.step;
        dir += (turn * step).to_radians();
        {
            let mut io = cx.borrow_mut();
            io.body.yaw_target = dir;
            io.body.walk = Vec3::from_yaw(dir) * speed;
            io.moving = Some(req::WALK);
        }
        n -= step;
        if dist_to_target(cx) - stop < 0.0 || !grounded(cx) {
            break;
        }
        frame().await;
    }
    face_target(cx);
}

/// `0xA8E`: the hop-strafe. Sideways (±90°, at random: the strafe motion 201 or 200), turning back
/// by `hop_strafe_turn` degrees a frame at `hop_strafe_speed`, facing the target at once each
/// frame, for [`hop_strafe_min`, `hop_strafe_max`) frames; within `hop_strafe_stop` it hops to that
/// side (200 → 202, 201 → 207) and swipes.
async fn hop_strafe(cx: &Cx) {
    let yaw = cx.borrow().body.yaw;
    let ang = k(cx, "hop_strafe_angle").to_radians();
    let mut turn = k(cx, "hop_strafe_turn");
    let (mut dir, m) = if rand(cx, "hop strafe side") - k(cx, "hop_strafe_side_chance") < 0.0 {
        (yaw - ang, req::STRAFE_B)
    } else {
        turn = -turn;
        (yaw + ang, req::STRAFE_A)
    };
    let mut n = rand_range(cx, k(cx, "hop_strafe_min"), k(cx, "hop_strafe_max"), "hop strafe time");
    let (speed, stop, blend) = (k(cx, "hop_strafe_speed"), k(cx, "hop_strafe_stop"), k(cx, "motion_blend"));
    while n >= 0.0 {
        play(cx, m, blend);
        let step = cx.borrow().w.step;
        dir += (turn * step).to_radians();
        face_target_now(cx);
        cx.borrow_mut().body.walk = Vec3::from_yaw(dir) * speed;
        n -= step;
        if dist_to_target(cx) - stop < 0.0 {
            let cur = cx.borrow().w.motion;
            match cur {
                Some(req::STRAFE_A) => hop(cx, req::HOP_LEFT).await,
                Some(req::STRAFE_B) => hop(cx, req::HOP_RIGHT).await,
                _ => {}
            }
            swipe(cx).await;
            return;
        }
        frame().await;
    }
}

/// `0x9E6`: farther than `pause_range`, stand (the idle motion) [`pause_min`, `pause_max`) frames
/// facing the target, until within `pause_stop` or off the ground.
async fn pause(cx: &Cx) {
    if dist_to_target(cx) - k(cx, "pause_range") <= 0.0 {
        return;
    }
    let mut n = rand_range(cx, k(cx, "pause_min"), k(cx, "pause_max"), "pause time");
    while n > 0.0 {
        play(cx, req::IDLE, k(cx, "idle_blend"));
        face_target(cx);
        if dist_to_target(cx) - k(cx, "pause_stop") < 0.0 || !grounded(cx) {
            break;
        }
        n -= cx.borrow().w.step;
        frame().await;
    }
}

/// `0x8D8`: the roar 228, facing the target, until it is done, the target is within `roar_stop` or
/// it leaves the ground.
async fn roar(cx: &Cx) {
    wait_grounded(cx).await;
    play(cx, req::ROAR, k(cx, "motion_blend"));
    loop {
        if motion_done(cx) {
            break;
        }
        face_target(cx);
        if dist_to_target(cx) - k(cx, "roar_stop") < 0.0 || !grounded(cx) {
            break;
        }
        frame().await;
    }
}

/// `0x9A2`: the first look (227 until done or request 6), then the idle.
async fn notice(cx: &Cx) {
    cx.borrow_mut().x.noticed = true;
    wait_grounded(cx).await;
    play(cx, req::NOTICE, k(cx, "motion_blend"));
    wait_done_or_air(cx).await;
    play(cx, req::IDLE, k(cx, "motion_blend"));
}

/// `0x94C`: after its own launch, the shake-off 224 (until done or request 6), then the idle.
async fn shake_off(cx: &Cx) {
    wait_grounded(cx).await;
    play(cx, req::SHAKE, k(cx, "motion_blend"));
    wait_done_or_air(cx).await;
    play(cx, req::IDLE, k(cx, "motion_blend"));
}

/// `0x6FA`: close range alternates: the back hop; next time a side hop (at random) and the swipe.
async fn close(cx: &Cx) {
    if !cx.borrow().x.alternate {
        cx.borrow_mut().x.alternate = true;
        hop(cx, req::HOP_BACK).await;
    } else {
        let r = if rand(cx, "close side") - k(cx, "close_side_chance") < 0.0 { req::HOP_RIGHT } else { req::HOP_LEFT };
        hop(cx, r).await;
        swipe(cx).await;
        cx.borrow_mut().x.alternate = false;
    }
}

/// `0x6A0`: mid range: the strafe; a pause or the roar; then the hop-strafe and the pounce, or the
/// swipe.
async fn mid(cx: &Cx) {
    strafe(cx).await;
    if rand(cx, "mid pause") - k(cx, "mid_pause_chance") < 0.0 {
        pause(cx).await;
    } else {
        roar(cx).await;
    }
    if rand(cx, "mid hop strafe") - k(cx, "mid_hop_strafe_chance") < 0.0 {
        hop_strafe(cx).await;
        pounce(cx).await;
    } else {
        swipe(cx).await;
    }
}

/// `0x658`: far: run in (`0xCCE`: until within `approach_dist` or the time is out; sometimes stand
/// a while); then the swipe, the roar or a pause.
async fn far(cx: &Cx) {
    approach(cx, 1, k(cx, "approach_timeout"), k(cx, "approach_retarget"), k(cx, "approach_dist")).await;
    if rand(cx, "far idle") - k(cx, "far_idle_chance") < 0.0 {
        play(cx, req::IDLE, k(cx, "idle_blend"));
        let n = rand_range(cx, k(cx, "far_idle_min"), k(cx, "far_idle_max"), "far idle time");
        wait_frames(cx, n).await;
    }
    let r = rand(cx, "far attack");
    if r - k(cx, "far_swipe_chance") < 0.0 {
        swipe(cx).await;
    } else if r - k(cx, "far_roar_chance") < 0.0 {
        roar(cx).await;
    } else {
        pause(cx).await;
    }
}

/// The appearance (`0x7BC` → `0x802`, spawned by the room): face Sora at once, the fade-in, a
/// yield, action 69, then while in the air; then the idle state.
async fn appear(cx: Cx) {
    if cx.borrow().x.spawned {
        face_target_now(&cx);
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        frame().await;
        let b = k(&cx, "appear_blend");
        action(&cx, req::APPEAR, None, b).await;
        while cx.borrow().w.airborne {
            frame().await;
        }
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The idle behaviour (`0x78C`): forever the wander about the current place, then the stretch 223.
async fn idle(cx: Cx) {
    loop {
        let c = |n: &str| k(&cx, n);
        let (inner, outer, band, timeout) = (c("wander_inner"), c("wander_outer"), c("wander_band"), c("wander_timeout"));
        let (chance, lo, hi, home, blend) = (c("wander_idle_chance"), c("wander_idle_min"), c("wander_idle_max"), c("wander_home_max"), c("idle_blend"));
        wander(&cx, inner, outer, band, timeout, chance, lo, hi, home, blend).await;
        wait_grounded(&cx).await;
        action(&cx, req::STRETCH, None, c("motion_blend")).await;
        frame().await;
    }
}

/// The engaged behaviour (`0x5A2`): the first look, by distance close / mid / far, then engaged
/// again (the supervisor restarts it).
async fn engaged(cx: Cx) {
    if !cx.borrow().x.noticed {
        notice(&cx).await;
    }
    let d = dist_to_target(&cx);
    if d - k(&cx, "close_range") < 0.0 {
        close(&cx).await;
    } else if d - k(&cx, "mid_range") < 0.0 {
        mid(&cx).await;
    } else {
        far(&cx).await;
    }
    // `0x610`: the engaged search (`sys 48`, 5000) or Sora: Re:CoM's battles have only Sora.
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
}

/// The hit behaviour (`0x4B0`): after its own launch the shake-off; else within `hit_range` the
/// flurry and the back hop, or a side hop and the pounce. Then engaged.
async fn hit(cx: Cx) {
    let after = cx.borrow().x.after;
    if after == After::ShakeOff {
        shake_off(&cx).await;
    } else if dist_to_target(&cx) - k(&cx, "hit_range") < 0.0 {
        if rand(&cx, "counter") - k(&cx, "hit_flurry_chance") < 0.0 {
            flurry(&cx).await;
            hop(&cx, req::HOP_BACK).await;
        } else {
            let r = if rand(&cx, "counter side") - k(&cx, "hit_hop_side_chance") < 0.0 { req::HOP_RIGHT } else { req::HOP_LEFT };
            hop(&cx, r).await;
            pounce(&cx).await;
        }
    }
    set_state(&cx, st::ENGAGED);
}

/// The supervisor (`0x2262`): after the init frame, each frame: while idle, Sora within
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
            // `0x774`: the idle clears the target (`0x1E5E`).
            set_target(&cx, s != st::IDLE);
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Powerwild's AI for one enemy.
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
                // `0x2346`: home = the position, the state object cleared.
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
    /// The hit handler (`0x2436`): the first look ends; a clash asks for the counter; a damaging
    /// hit of kind 4 / 5 above `own_launch_hp` of its HP plays its own launch (`sys 38`) and asks
    /// for the shake-off; any other damaging hit asks for the counter.
    fn on_hit(&self, news: HitNews) {
        {
            let mut io = self.io.borrow_mut();
            io.x.noticed = true;
            io.x.after = After::Counter;
            io.x.launch = None;
            let kind = io.x.hit_kind;
            if news == HitNews::Damage && (kind == 4 || kind == 5) && io.x.hp_ratio - io.c.num("own_launch_hp") > 0.0 {
                io.x.launch = Some(ScriptedLaunch { ground: req::OWN_LAUNCH, air: req::OWN_AIR_LAUNCH, hop: io.c.num("own_launch_hop"), speed: io.c.num("own_launch_speed"), down: io.c.num("own_launch_down") });
                io.x.after = After::ShakeOff;
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
        set_state(&self.io, st::ENGAGED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consts() -> Consts {
        // Test numbers (the script's own come from the player's file at the asset step; these are
        // the disassembly's).
        let v: &[(&str, f32)] = &[
            ("hop_height", 500.0), ("idle_search", 1500.0), ("close_range", 300.0), ("mid_range", 700.0), ("hit_range", 700.0),
            ("hit_flurry_chance", 0.5), ("hit_hop_side_chance", 0.5), ("own_launch_hp", 0.4), ("own_launch_hop", 80.0),
            ("own_launch_speed", 25.0), ("own_launch_down", 0.0), ("close_side_chance", 0.5), ("mid_pause_chance", 0.6),
            ("mid_hop_strafe_chance", 0.7), ("far_idle_chance", 0.3), ("far_idle_min", 60.0), ("far_idle_max", 120.0),
            ("far_swipe_chance", 0.6), ("far_roar_chance", 0.8), ("approach_timeout", 150.0), ("approach_retarget", 20.0),
            ("approach_dist", 560.0), ("strafe_speed", 7.0), ("strafe_min", 60.0), ("strafe_max", 120.0), ("strafe_turn", 2.0),
            ("strafe_stop", 300.0), ("strafe_side_chance", 0.5), ("strafe_angle", 90.0), ("strafe_turn_factor", 0.5),
            ("pause_range", 300.0), ("pause_min", 30.0), ("pause_max", 60.0), ("pause_stop", 300.0), ("roar_stop", 300.0),
            ("hop_strafe_turn", 0.45), ("hop_strafe_side_chance", 0.5), ("hop_strafe_angle", 90.0), ("hop_strafe_min", 60.0),
            ("hop_strafe_max", 120.0), ("hop_strafe_speed", 3.0), ("hop_strafe_stop", 300.0), ("hop_back_angle", 180.0),
            ("hop_left_angle", 70.0), ("hop_right_angle", -70.0), ("hop_speed", 20.0), ("hop_jump_height", 50.0), ("hop_time", 30.0),
            ("hop_drag", 0.9), ("pounce_offset", 200.0), ("pounce_divisor", 30.0), ("pounce_max_speed", 10.0), ("pounce_height", 200.0),
            ("pounce_steer", 7.0), ("pounce_attack_after", 19.0), ("wander_inner", 350.0), ("wander_outer", 450.0), ("wander_band", 100.0),
            ("wander_timeout", 300.0), ("wander_idle_chance", 0.4), ("wander_idle_min", 90.0), ("wander_idle_max", 180.0),
            ("wander_home_max", 700.0), ("idle_blend", 8.0), ("motion_blend", 8.0), ("swipe_blend", 2.0), ("appear_fade", 30.0),
            ("appear_blend", 8.0),
            // The shared engine's.
            ("walk_speed", 3.0), ("run_speed", 7.0), ("air_accel", 0.1), ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0),
            ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0),
            ("air_knock_speed", 16.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("weight", 1.0), ("gravity", crate::params::GRAVITY),
            ("fall_cap", 20.0), ("attack_blend", 8.0),
        ];
        Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), NAMES).unwrap()
    }

    fn frames() -> Vec<(u16, f32)> {
        vec![(req::APPEAR, 60.0), (req::NOTICE, 148.0), (req::ROAR, 80.0), (req::SHAKE, 92.0), (req::SWIPE, 62.0), (req::FLURRY, 98.0), (req::POUNCE, 24.0), (req::STRETCH, 150.0), (202, 14.0), (207, 14.0), (215, 14.0), (203, 8.0), (208, 8.0), (216, 8.0)]
    }

    /// A flat floor at y = 0, Sora at `sora`; an action holds the AI for its motion's length; the
    /// current request is the last one played.
    struct Sim {
        ai: Ai,
        pos: Vec3,
        busy: f32,
        current: Option<u16>,
        log: Vec<Cmd>,
    }

    impl Sim {
        fn run(&mut self, sora: Vec3, n: usize) {
            for _ in 0..n {
                let airborne = self.ai.body().airborne();
                let w = World { step: 1.0, pos: self.pos, target: Some(sora), grounded: !airborne, airborne, moved: 5.0, motion: self.current };
                if self.busy > 0.0 {
                    self.busy -= 1.0;
                } else {
                    for c in self.ai.frame(w) {
                        match c {
                            ECmd::Action { request, .. } => {
                                self.busy = frames().iter().find(|x| x.0 == request).map_or(30.0, |x| x.1);
                                self.current = Some(request);
                            }
                            ECmd::Motion { request, .. } => self.current = Some(request),
                            _ => {}
                        }
                        self.log.push(c);
                    }
                }
                let c = self.ai.consts();
                let steer = self.ai.take_steer();
                let d = self.ai.body().step(1.0, self.pos.y, steer, &c);
                self.pos += d;
                if self.pos.y >= 0.0 && self.ai.body().falling() {
                    self.pos.y = 0.0;
                    self.ai.body().land();
                }
            }
        }
    }

    fn sim(seed: u32, spawned: bool) -> Sim {
        Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: Vec3::ZERO, busy: 0.0, current: None, log: Vec::new() }
    }

    /// A spawned Powerwild past its appearance, idle, Sora far away.
    fn ready(seed: u32) -> Sim {
        let mut s = sim(seed, true);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 70);
        assert!(s.ai.risen());
        s.log.clear();
        s
    }

    fn attacks(s: &Sim) -> Vec<Attack> {
        s.log.iter().filter_map(|c| if let ECmd::Action { attack: Some(a), .. } = c { Some(*a) } else { None }).collect()
    }

    fn motions(s: &Sim) -> Vec<u16> {
        s.log.iter().filter_map(|c| if let ECmd::Motion { request, .. } = c { Some(*request) } else { None }).collect()
    }

    #[test]
    fn the_names_cover_the_numbers() {
        assert!(Table::new(vec![], NAMES).is_err());
        assert_eq!(consts().num("hop_jump_height"), 50.0);
    }

    #[test]
    fn spawned_appearance_then_idle() {
        let mut s = sim(1, true);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 3);
        assert!(s.log.contains(&ECmd::FadeIn(30.0)), "{:?}", s.log);
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Action { request: 69, attack: None, .. })));
        s.run(Vec3::new(0.0, 0.0, 5000.0), 70);
        assert!(s.ai.risen() && s.log.contains(&ECmd::State(st::IDLE)));
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Motion { request: req::WALK, .. })), "it wanders");
    }

    #[test]
    fn close_range_hops_back_then_side_hops_and_swipes() {
        let mut s = ready(3);
        s.run(Vec3::new(0.0, 0.0, 200.0), 400);
        let m = motions(&s);
        assert!(m.contains(&req::NOTICE), "the first look: {m:?}");
        let back = m.iter().position(|&r| r == req::HOP_BACK).expect("the back hop first");
        assert_eq!(&m[back..back + 2], &[215, 216], "crouch, then rising");
        assert!(m.contains(&217), "the top motion: {m:?}");
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Queue { request: 218, .. })));
        // The hop went backward (−z) and landed.
        assert!(!s.ai.body().airborne());
    }

    #[test]
    fn the_hop_travels_its_kh1_distance() {
        let mut s = ready(3);
        s.ai.io.borrow_mut().x.noticed = true;
        let z0 = s.pos.z;
        s.run(Vec3::new(0.0, 0.0, 200.0), 60);
        // Facing +z (Sora), the back hop: about 20 × (1 − √0.9³¹) / (1 − √0.9) ≈ 314 backward.
        assert!((s.pos.z - z0 + 314.0).abs() < 25.0, "z {} → {}", z0, s.pos.z);
    }

    #[test]
    fn mid_and_far_attack() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..30 {
            let mut s = ready(seed);
            s.ai.io.borrow_mut().x.noticed = true;
            let d = if seed % 2 == 0 { 500.0 } else { 900.0 };
            s.run(Vec3::new(0.0, 0.0, d), 600);
            for a in attacks(&s) {
                seen.insert(a);
            }
        }
        assert!(seen.contains(&Attack::Swipe) && seen.contains(&Attack::Pounce), "{seen:?}");
    }

    #[test]
    fn a_knock_above_40_percent_asks_for_its_own_launch_then_shakes_off() {
        let mut s = ready(2);
        s.ai.set_hp_ratio(0.9);
        s.ai.set_hit_kind(5);
        Behaviour::on_hit(&s.ai, HitNews::Damage);
        let l = s.ai.scripted_launch().expect("its own launch");
        assert_eq!((l.ground, l.air, l.hop, l.speed, l.down), (270, 280, 80.0, 25.0, 0.0));
        assert!(s.ai.scripted_launch().is_none(), "taken once");
        s.run(Vec3::new(0.0, 0.0, 400.0), 5);
        assert!(motions(&s).contains(&req::SHAKE), "{:?}", s.log);
        // Low HP or a flinch kind: KH1's own reaction, the counter.
        s.ai.set_hp_ratio(0.3);
        Behaviour::on_hit(&s.ai, HitNews::Damage);
        assert!(s.ai.scripted_launch().is_none());
        s.ai.set_hp_ratio(0.9);
        s.ai.set_hit_kind(3);
        Behaviour::on_hit(&s.ai, HitNews::Damage);
        assert!(s.ai.scripted_launch().is_none());
    }

    #[test]
    fn the_counter_flurries_or_pounces() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..20 {
            let mut s = ready(seed);
            s.ai.set_hit_kind(3);
            Behaviour::on_hit(&s.ai, HitNews::Clash);
            s.run(Vec3::new(0.0, 0.0, 400.0), 300);
            for a in attacks(&s) {
                seen.insert(a);
            }
        }
        assert!(seen.contains(&Attack::Flurry) && seen.contains(&Attack::Pounce), "{seen:?}");
    }

    /// A long fight: the AI acts at least every 600 frames (the host's quiet watchdog).
    #[test]
    fn a_long_fight_never_stalls() {
        for seed in 0..8u32 {
            let mut s = ready(seed);
            let (mut last, mut worst, mut next_hit) = (0usize, 0usize, 300usize);
            for f in 0..12_000usize {
                let a = f as f32 / 600.0;
                let r = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
                let sora = Vec3::new(a.sin() * r, 0.0, a.cos() * r);
                if f == next_hit {
                    Behaviour::on_hit(&s.ai, HitNews::Damage);
                    s.busy = 34.0;
                    next_hit = f + 250;
                }
                let n = s.log.len();
                s.run(sora, 1);
                if s.log[n..].iter().any(|c| !matches!(c, ECmd::Yaw(_) | ECmd::Motion { request: 1 | 2, .. })) || s.busy > 0.0 {
                    worst = worst.max(f - last);
                    last = f;
                }
            }
            assert!(worst < 600, "seed {seed}: {worst} frames without acting");
        }
    }

    #[test]
    fn attack_rows_and_indices() {
        assert_eq!(Attack::Flurry.rows(), &[0xEC, 0x15C, 0x1CC]);
        assert_eq!(Attack::Pounce.index(), 2);
    }
}
