//! KH1 FM's Bandit (`xa_ex_2090.mdls` part 5, `ex_2090.bd`), engine-agnostic: its AI script on
//! the shared enemy framework (`crate::enemy`). M11 (`docs/recom-m11-enemies.md` § The Bandit).
//! Re:CoM runs it natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s
//! `win::kh1_enemy`), Re:CoM plays the motions, collides the body and the attacks, applies the
//! damage.
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2090.bd` at
//! checked offsets); nothing of the game is in this file. Script offsets are the player's
//! `ex_2090.bd`'s (Checked (files): the script disassembled with `tools/kh1-ai-script.py`).
//!
//! # The script's shape (the Air Soldier's newer shape: the actor at `wp+0`, the state object at
//! `wp+0x30`)
//!
//! - entry `0x317C`: init `0x34B0` (stats records LV1 `0x14` / LV2 `0x80`, walk 2 / run 8, the
//!   handlers: hit `0x367A`), a yield, the party and group handles, the world (`wp+0x48` = KH1's
//!   world id, `wp+0x4C` its room), the appearance thread `0x1736` → `0x174A`; then each frame: a
//!   state request restarts the behaviour (1 idle `0x1166` → `0x1190`, 2 engaged `0xB0A` → `0xB38`,
//!   3 lost `0x8A6` → `0x8D0`, 4 hit `0x4C8` → `0x4FA`); while idle, the search: Sora within
//!   `idle_search` → engaged.
//! - the hit handler `0x367A`: a clash asks for the hit state (the block stance's flag kept); a
//!   damaging hit clears it, notes the 1st or 2nd hit of a run (`wp+0x54`, record `+0x24`),
//!   rewrites kind 4 to 5 (its launch) and asks for the hit state.
//! - its attacks are plain requests made with attack rows (`crate::enemy::Cmd::Attack`: the
//!   script keeps running while they play): the slash 200, the spin 210 → 211 (looped) → 212,
//!   the leap 216 → 217 (movement state 7, `crate::enemy::Body::air_flight`) → 218.
//!
//! - its guard-type rows (the shared enemy-guard step, `crate::enemy::guard`): the spin's body row
//!   `0xEC` (made with 210, 211, 212) and the block stance's blade row `0x15C` (230, 231, 233), each
//!   a [`Guard`] the host turns on while its request plays and its HIT slot is on
//!   (`crate::enemy::Cmd::Guard`); Sora's swing that meets one is stopped, both sides clash.
//!
//! Not ported (Re:CoM's battles only, decision M10; `docs/recom-m11-enemies.md` § The Bandit): the
//! thrown blade (`sys 94`'s child object `ex_2090_00.bd`, row `0x1CC`: owner question), the
//! leash `0x3B12` and the fall-out death `0x2C30` (KH1 removes it), the group leader's messages
//! (`sys 81`, `wp+0x18`), KH1's room rules (Agrabah rooms 2 / 3–6 / 9–11 / 17, the sitting Bandits
//! `wp+0x68`, the rooftop heights), the head look (`sys 121`), the effects (`sys 75`), the blade's
//! part mask (`sys 98` / `99`), the death routine (Re:CoM's defeat, prizes, EXP).

use crate::enemy::script::{approach, dist_to_target, face_target, frame, motion_done, motion_frames, move_toward, poll, pos, push, rand, rand_range, random_offset, set_state, set_target, steer, take_state, target, tick_clock, wait_frames, wait_grounded, walk_to};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Bandit's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The appearance (`0x2C8A`: plain, blend 0), 24 frames, looping until the queued idle.
    pub const APPEAR: u16 = 69;
    /// The slash (`0x23AA`), 60 frames, root motion; its row's hit slot 1 [26, 38).
    pub const SLASH: u16 = 200;
    /// The throw (`0x22DA`), 438 frames, root motion: the blade leaves at 90, back at 380.
    pub const THROW: u16 = 201;
    /// The spin (`0x1EC6`): the wind-up 210 (46 f), the loop 211 (40 f, looped), the end 212 (94 f).
    pub const SPIN_UP: u16 = 210;
    pub const SPIN: u16 = 211;
    pub const SPIN_END: u16 = 212;
    /// The leap (`0x1B36`): the crouch 216 (60 f), the flight 217 (20 f, looped), the landing
    /// slash 218 (40 f, request bit 16: it holds its last pose).
    pub const LEAP_CROUCH: u16 = 216;
    pub const LEAP: u16 = 217;
    pub const LEAP_SLASH: u16 = 218;
    /// The back hop (`0x41B8`): the crouch 221, 222 rising (bit 16), 223 at the top, 224 landing.
    pub const HOP: u16 = 221;
    /// The block stance (`0x400A`): 230 (12 f), the hold 231 (60 f, looped), 233 (16 f).
    pub const STANCE: u16 = 230;
    pub const STANCE_HOLD: u16 = 231;
    pub const STANCE_END: u16 = 233;
}

/// The Bandit's attacks, in the order the host indexes them (the manifest's `attacks`): each a
/// plain request made with its attack rows (`crate::enemy::Cmd::Attack`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    Slash,
    Spin,
    SpinEnd,
    Leap,
}

impl Attack {
    pub const ALL: [Attack; 4] = [Attack::Slash, Attack::Spin, Attack::SpinEnd, Attack::Leap];

    pub fn request(self) -> u16 {
        match self {
            Attack::Slash => req::SLASH,
            Attack::Spin => req::SPIN,
            Attack::SpinEnd => req::SPIN_END,
            Attack::Leap => req::LEAP,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Slash => "slash",
            Attack::Spin => "spin",
            Attack::SpinEnd => "spin end",
            Attack::Leap => "leap",
        }
    }

    /// The damaging attack rows (script offsets, 0x70 bytes, `sys 23`) made with it. The
    /// guard-type rows (`0xEC` with the spin, `0x15C` with the block stance) are [`Guard`]s.
    pub fn rows(self) -> &'static [usize] {
        match self {
            Attack::Slash => &[0x23C],
            Attack::Spin => &[0x2AC],
            Attack::SpinEnd => &[0x31C],
            Attack::Leap => &[0x3FC, 0x38C],
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The Bandit's guard-type rows (attack type 2: they meet only swings and type-3 attacks, never a
/// hurt body; `crate::enemy::guard`), in the order of the manifest's `guards`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Guard {
    /// The spin's body row `0xEC` (bone code 8, 60, priority 90, recoil 213; HIT slot 1).
    Spin,
    /// The block stance's blade row `0x15C` (bone code 48, 40, priority 30, recoil 232; HIT slot 0).
    Stance,
}

impl Guard {
    pub const ALL: [Guard; 2] = [Guard::Spin, Guard::Stance];

    pub fn name(self) -> &'static str {
        match self {
            Guard::Spin => "spin",
            Guard::Stance => "stance",
        }
    }

    /// The row's script offset.
    pub fn row(self) -> usize {
        match self {
            Guard::Spin => 0xEC,
            Guard::Stance => 0x15C,
        }
    }

    /// The requests the script makes it with (`0x1ED6` / `0x1F6E` / `0x2188`; `0x408E` / `0x40CC` /
    /// `0x4186`).
    pub fn requests(self) -> &'static [u16] {
        match self {
            Guard::Spin => &[req::SPIN_UP, req::SPIN, req::SPIN_END],
            Guard::Stance => &[req::STANCE, req::STANCE_HOLD, req::STANCE_END],
        }
    }

    pub fn index(self) -> u8 {
        Guard::ALL.iter().position(|&g| g == self).unwrap_or(0) as u8
    }
}

/// The names of the Bandit's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    "idle_search",
    "appear_blend",
    "appear_fade",
    "appear_wait",
    "wander_a_min",
    "wander_a_max",
    "wander_b_min",
    "wander_b_max",
    "wander_timeout",
    "wander_stop_chance",
    "wander_idle_chance",
    "wander_idle_min",
    "wander_idle_max",
    "home_max",
    "home_timeout",
    "idle_blend",
    "close_range",
    "mid_range",
    "close_hop_chance",
    "mid_strafe_turn",
    "mid_pause_min",
    "mid_pause_max",
    "mid_slash_chance",
    "mid_approach_time",
    "mid_approach_stop",
    "far_leash",
    "far_approach_time",
    "far_approach_stop",
    "far_pause_min",
    "far_pause_max",
    "agrabah_throw_chance",
    "agrabah_far_stop",
    "agrabah_leap_chance",
    "pause_min",
    "pause_max",
    "strafe_short_chance",
    "strafe_mid_chance",
    "strafe_turn",
    "strafe_short_min",
    "strafe_short_max",
    "strafe_mid_min",
    "strafe_mid_max",
    "strafe_long_min",
    "strafe_long_max",
    "strafe_side_chance",
    "strafe_angle_first",
    "strafe_angle",
    "strafe_speed",
    "strafe_extra",
    "strafe_stop",
    "run_blend",
    "strafe_turn_factor",
    "pause_extra",
    "pause_stop",
    "approach_extra",
    "approach_retarget",
    "hit_far",
    "hit_throw_chance",
    "hit_stance_chance",
    "hit_slash_chance",
    "hit_leap_chance",
    "hit_close",
    "hit_mid",
    "hop_angle",
    "hop_speed",
    "hop_jump_height",
    "hop_time",
    "hop_drag",
    "slash_blend",
    "slash_wait",
    "slash_effect_wait",
    "throw_blend",
    "throw_wait",
    "throw_release",
    "throw_hold",
    "spin_blend",
    "spin_windup",
    "spin_speed",
    "spin_loops",
    "spin_loop_blend",
    "spin_step",
    "spin_trim",
    "spin_end_wait_a",
    "spin_end_wait_b",
    "spin_end_wait_c",
    "leap_blend",
    "leap_lift",
    "leap_speed",
    "leap_arrive",
    "leap_timeout",
    "leap_air_blend",
    "leap_drop_height",
    "leap_stuck",
    "leap_stuck_half",
    "leap_end_wait",
    "stance_blend",
    "stance_hold_blend",
    "stance_min",
    "stance_max",
    "stance_angle",
];

/// The Bandit's numbers.
pub type Consts = Table;

/// The Bandit's state objects (`wp+0x30`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const LOST: u8 = 3;
    pub const HIT: u8 = 4;
}

/// KH1's world id of Agrabah (`al`, KH1's world code table `0x1404DCCA0`; Checked (exe)): the
/// script's `wp+0x48 == 8` rules.
pub const AGRABAH: u8 = 8;

/// The Bandit's own work area: spawned by the room (`sys 132` ≠ 0), the appearance done, KH1's
/// world (`wp+0x48`), the hit's place in its run (record `+0x24`), a 1st / 2nd hit seen
/// (`wp+0x54`, never cleared after: `0x756`), the block stance's flag (`wp+0x84`), the spin's
/// attack landed (`wp+0x80`, its row's hit callback `0x228A`), Sora's target is it, its last
/// standing height (KH1 y, for `sys 89`), the leap's last steering direction (`+0x20`).
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    spawned: bool,
    appeared: bool,
    world: Option<u8>,
    hit_count: u16,
    countered: bool,
    blocking: bool,
    landed: bool,
    targeted: bool,
    floor: Option<f32>,
    last_dir: Vec3,
}

type Cx = crate::enemy::script::Cx<Consts, Attack, Wp>;
type Task = crate::enemy::script::Task;
pub type Cmd = ECmd<Attack>;

fn k(cx: &Cx, n: &str) -> f32 {
    cx.borrow().c.num(n)
}

/// A plain motion request (`sys 12` / `sys 13` without bit 24); `sys 172`'s clock restarts for a
/// request that plays anew.
fn play(cx: &Cx, r: u16, blend: f32) {
    {
        let mut io = cx.borrow_mut();
        if io.w.motion != Some(r) {
            io.clock = (Some(r), 0.0);
        }
    }
    push(cx, ECmd::Motion { request: r, blend });
}

/// `sys 13` of an attack's request (its rows made just before): the motion plays from its start
/// (`0x14029F980` → `0x14029FC40`), the script keeps running.
fn attack_now(cx: &Cx, a: Attack, blend: f32) {
    cx.borrow_mut().clock = (Some(a.request()), 0.0);
    push(cx, ECmd::Attack { request: a.request(), attack: a, blend });
}

/// `sys 12` of an attack's request: nothing when it already plays (`0x14029F7C0` returns for the
/// entry playing), so a looped request carries on.
fn attack_keep(cx: &Cx, a: Attack, blend: f32) {
    if cx.borrow().w.motion != Some(a.request()) {
        attack_now(cx, a, blend);
    }
}

fn grounded(cx: &Cx) -> bool {
    cx.borrow().w.grounded
}

fn step(cx: &Cx) -> f32 {
    cx.borrow().w.step
}

/// `sys 51 == 6`: the air motion a reaction queues plays (`0x280E`).
fn in_air_motion(cx: &Cx) -> bool {
    cx.borrow().w.motion == Some(req::AIR)
}

/// Requests made with bit 16 (`0x100DA`, `(221 + 1) | 0x10000`): KH1 holds their last pose at the
/// end (`0x14029FC40` copies bit 16 to `+0x160` bit 3, `0x14029F0A0` then holds; Checked (decomp)).
fn holds(r: u16) -> bool {
    r == req::LEAP_SLASH || r == req::HOP + 1
}

/// `sys 172` (`0x14029EFC0`): the motion has played to its end this cycle. KH1 loops a plain
/// request at its end, back to its loop start (`+0x288`, 0 for an enemy's motions: only
/// `0x14029FA10` sets it; Checked (decomp)), so after one cycle "the end" is each cycle's end:
/// the clock ([`motion_done`]'s, which counts on) taken modulo the length; a holding request
/// stays at its end.
fn done_now(cx: &Cx) -> bool {
    if !motion_done(cx) {
        return false;
    }
    let io = cx.borrow();
    let Some(r) = io.clock.0 else { return true };
    let len = motion_frames(cx, r);
    if len <= 0.0 || holds(r) {
        return true;
    }
    len <= io.w.step + io.clock.1 % len
}

/// `0x2D5A`: a yield, then until the motion has played to its end (`sys 172`).
async fn wait_done(cx: &Cx) {
    loop {
        frame().await;
        if done_now(cx) {
            break;
        }
    }
}

/// `0x27DA`: until the motion is done or the air motion plays, turning to the target.
async fn wait_done_or_air(cx: &Cx) {
    while !done_now(cx) {
        if in_air_motion(cx) {
            break;
        }
        face_target(cx);
        frame().await;
    }
}

/// `0x2786(n)`: n frames turning to the target, unless the air motion plays.
async fn wait_facing(cx: &Cx, n: f32) {
    let mut t = 0.0;
    while t - n < 0.0 {
        if in_air_motion(cx) {
            break;
        }
        face_target(cx);
        t += step(cx);
        frame().await;
    }
}

/// While in movement state 2 (an arc; `*(actor+0x70) == 2`): not a timed jump (3) nor the leap's
/// state 7.
async fn wait_arc(cx: &Cx) {
    while cx.borrow().body.arc.is_some() {
        frame().await;
    }
}

/// The rest of the behaviour thread after a state request (`end_thread`): the supervisor replaces
/// it at its next run.
async fn halt() {
    loop {
        frame().await;
    }
}

/// `sys 11` toward Sora (`0x2DA2(actor, wp+8)`: party 0, not the target).
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

/// `sys 89`: the height above the floor (floor y − y, KH1 y down), the floor being where it last
/// stood (Assumption: Re:CoM's battle floors are flat, as for the fliers).
fn height(cx: &Cx) -> f32 {
    let io = cx.borrow();
    io.x.floor.map_or(0.0, |f| f - io.w.pos.y)
}

/// `sys 100`: the cosine of the angle between two vectors (`0x1402A8C70`: 3D, clamped to
/// [−1, 1], 0 when either is zero). Checked (decomp).
fn cos_between(a: Vec3, b: Vec3) -> f32 {
    let (la, lb) = (a.length(), b.length());
    if la == 0.0 || lb == 0.0 {
        return 0.0;
    }
    (a.dot(b) / (la * lb)).clamp(-1.0, 1.0)
}

/// `0x42DC(t, d)` → `0x2DEA(actor, 1, t + 8, 20, d)`: run at the target (re-read every 20 frames)
/// until within d or the time is out (the shared approach).
async fn approach_to(cx: &Cx, time: f32, stop: f32) {
    approach(cx, 1, time + k(cx, "approach_extra"), k(cx, "approach_retarget"), stop).await;
}

/// `0x4606(0.5, 0.8)`: the strafe's time, [30, 45) at 50 %, else [60, 90) below 80 %, else
/// [120, 150).
fn strafe_time(cx: &Cx) -> f32 {
    let r = rand(cx, "strafe time pick");
    if r - k(cx, "strafe_short_chance") < 0.0 {
        rand_range(cx, k(cx, "strafe_short_min"), k(cx, "strafe_short_max"), "strafe time")
    } else if r - k(cx, "strafe_mid_chance") < 0.0 {
        rand_range(cx, k(cx, "strafe_mid_min"), k(cx, "strafe_mid_max"), "strafe time")
    } else {
        rand_range(cx, k(cx, "strafe_long_min"), k(cx, "strafe_long_max"), "strafe time")
    }
}

/// `0x4334(target, time, turn)`: run sideways (60 % to −90° of the facing, else +90° with the turn
/// negated) at 8 for time + 10 frames, the direction turning by turn × 0.5° a frame; stops off the
/// ground or within 200 of the target.
async fn strafe(cx: &Cx, time: f32, turn: f32) {
    let yaw = cx.borrow().body.yaw;
    let mut turn = turn;
    let mut dir = if rand(cx, "strafe side") - k(cx, "strafe_side_chance") < 0.0 {
        yaw + k(cx, "strafe_angle_first").to_radians()
    } else {
        turn = -turn;
        yaw + k(cx, "strafe_angle").to_radians()
    };
    let (speed, stop, blend, factor) = (k(cx, "strafe_speed"), k(cx, "strafe_stop"), k(cx, "run_blend"), k(cx, "strafe_turn_factor"));
    let mut t = time + k(cx, "strafe_extra");
    while t - 0.0 > 0.0 {
        let s = step(cx);
        if !grounded(cx) || dist_to_target(cx) - stop < 0.0 {
            break;
        }
        play(cx, req::RUN, blend);
        dir += (turn * factor * s).to_radians();
        {
            let mut io = cx.borrow_mut();
            io.body.yaw_target = dir;
            io.body.walk = Vec3::from_yaw(dir) * speed;
            io.moving = Some(req::RUN);
        }
        t -= s;
        frame().await;
    }
}

/// `0x44AE(0, n)`: the idle (blend 8) for n + 8 frames, until within 400 (KH1's head look at the
/// target: not ported).
async fn pause(cx: &Cx, n: f32) {
    play(cx, req::IDLE, k(cx, "idle_blend"));
    let mut t = n + k(cx, "pause_extra");
    while t - 0.0 > 0.0 {
        if dist_to_target(cx) - k(cx, "pause_stop") < 0.0 {
            break;
        }
        t -= step(cx);
        frame().await;
    }
}

/// The closing pause of a round (`0x44AE(0, rand(60, 75))`).
async fn round_pause(cx: &Cx) {
    let n = rand_range(cx, k(cx, "pause_min"), k(cx, "pause_max"), "pause time");
    pause(cx, n).await;
}

/// `0x23AA`: the slash. On the ground; its row `0x23C`; turned to the target; 200 at blend 8; 8 and
/// 22 frames (its effect: not ported); to the motion's end; the idle at blend 8.
async fn slash(cx: &Cx) {
    wait_grounded(cx).await;
    face_target(cx);
    attack_now(cx, Attack::Slash, k(cx, "slash_blend"));
    wait_frames(cx, k(cx, "slash_wait")).await;
    wait_frames(cx, k(cx, "slash_effect_wait")).await;
    wait_done(cx).await;
    play(cx, req::IDLE, k(cx, "idle_blend"));
}

/// `0x22DA`: the throw. On the ground; 201 at blend 8; 8 frames; 82 frames turning to the target;
/// KH1 hides the blade (`sys 98`) and throws it (`sys 94`: the child object `ex_2090_00.bd` with
/// row `0x1CC`, **not built**: owner question); 290 frames turning to the target; the blade back
/// (`sys 99`); to the motion's end; the idle at blend 8.
async fn throw(cx: &Cx) {
    wait_grounded(cx).await;
    play(cx, req::THROW, k(cx, "throw_blend"));
    wait_frames(cx, k(cx, "throw_wait")).await;
    wait_facing(cx, k(cx, "throw_release")).await;
    wait_facing(cx, k(cx, "throw_hold")).await;
    wait_done(cx).await;
    play(cx, req::IDLE, k(cx, "idle_blend"));
}

/// `0x1EC6`: the spin. On the ground; 210 at blend 4 (with its guard-type body row `0xEC`, made again
/// for 211 and 212: [`Guard::Spin`]);
/// 34 frames turning to the target, then to its end; up to 4 rounds of 211 (its row `0x2AC`):
/// 12 frames then len − 2 − 12 more gliding at the target (`sys 40` mode 2 at the stats block's
/// `+0xC` = 4), each part ended off the ground, the rounds ended once the spin landed (`wp+0x80`);
/// to the loop's end; 212 (its row `0x31C`) and its effect's 12 / 22 / 16 frames, to its end; the
/// idle at blend 8.
async fn spin(cx: &Cx) {
    wait_grounded(cx).await;
    play(cx, req::SPIN_UP, k(cx, "spin_blend"));
    guard(cx, Guard::Spin, req::SPIN_UP);
    wait_facing(cx, k(cx, "spin_windup")).await;
    wait_done_or_air(cx).await;
    cx.borrow_mut().x.landed = false;
    // `0x1F6E`: the body row again, for the loop 211 (its first tick sees 211).
    guard(cx, Guard::Spin, req::SPIN);
    let speed = k(cx, "spin_speed");
    let first = k(cx, "spin_step");
    let rest = motion_frames(cx, req::SPIN) - k(cx, "spin_trim") - first;
    let mut n = k(cx, "spin_loops");
    while n - 0.0 > 0.0 {
        attack_keep(cx, Attack::Spin, k(cx, "spin_loop_blend"));
        for part in [first, rest] {
            let mut t = part;
            while t - 0.0 > 0.0 {
                if !grounded(cx) {
                    break;
                }
                if let Some(g) = target(cx) {
                    move_toward(cx, g, 2, speed);
                }
                t -= step(cx);
                frame().await;
            }
        }
        if cx.borrow().x.landed {
            break;
        }
        n -= 1.0;
    }
    wait_done(cx).await;
    attack_now(cx, Attack::SpinEnd, k(cx, "spin_loop_blend"));
    guard(cx, Guard::Spin, req::SPIN_END);
    wait_frames(cx, k(cx, "spin_end_wait_a")).await;
    wait_frames(cx, k(cx, "spin_end_wait_b")).await;
    wait_frames(cx, k(cx, "spin_end_wait_c")).await;
    wait_done(cx).await;
    play(cx, req::IDLE, k(cx, "idle_blend"));
}

/// `0x1B36`: the leap. On the ground; 216 at blend 8, to its end; a point 50 above the target;
/// movement state 7 with no velocity (`sys 169`); its rows `0x3FC` / `0x38C` and 217; the flight
/// `0x1D42(217, &p, 70, 100, 500)`; 218 (bit 16) at blend 0, 24 frames, to its end; movement state
/// 0 (`sys 88`) and, above the floor, a drop with its last direction (`sys 50(actor, 0, +0x20)`);
/// until on the ground.
async fn leap(cx: &Cx) {
    wait_grounded(cx).await;
    play(cx, req::LEAP_CROUCH, k(cx, "leap_blend"));
    wait_done_or_air(cx).await;
    let Some(t) = target(cx) else { return };
    let goal = Vec3::new(t.x, t.y - k(cx, "leap_lift"), t.z);
    cx.borrow_mut().body.air_fly(Vec3::ZERO);
    let air = k(cx, "leap_air_blend");
    attack_keep(cx, Attack::Leap, air);
    // `0x1D42(motion, goal, speed, arrive, timeout)`.
    let (speed, arrive, drop) = (k(cx, "leap_speed"), k(cx, "leap_arrive"), k(cx, "leap_drop_height"));
    let stuck = speed * k(cx, "leap_stuck");
    let mut left = k(cx, "leap_timeout");
    while left - 0.0 > 0.0 {
        attack_keep(cx, Attack::Leap, air);
        let p = pos(cx);
        let mut d = goal - p;
        cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
        let s = step(cx);
        if d.flat().length() - arrive < 0.0 {
            if d.y.abs() - speed * s < 0.0 {
                // `+0x2C = 0`: no steering this frame.
                break;
            }
        } else if d.y - 0.0 > 0.0 {
            if p.y - goal.y > 0.0 {
                d.y = 0.0;
            } else if height(cx) - drop > 0.0 {
                d.x = 0.0;
                d.z = 0.0;
            }
        }
        let (u, _) = d.normalize_len();
        cx.borrow_mut().x.last_dir = u;
        steer(cx, u, speed);
        left -= s;
        frame().await;
        // `0x24FE(70 × 0.1)`: the last frame's travel under half of 7 × step: stuck.
        let (moved, s) = (cx.borrow().w.moved, step(cx));
        if moved - stuck * s * k(cx, "leap_stuck_half") < 0.0 {
            break;
        }
    }
    play(cx, req::LEAP_SLASH, air);
    wait_frames(cx, k(cx, "leap_end_wait")).await;
    wait_done(cx).await;
    cx.borrow_mut().body.ground();
    if height(cx) - 0.0 > 0.0 {
        let (v, y) = (cx.borrow().x.last_dir, pos(cx).y);
        {
            let mut io = cx.borrow_mut();
            let c = io.c.clone();
            io.body.jump(0.0, Vec3::new(v.x, 0.0, v.z), y, &c);
        }
        push(cx, ECmd::Jump { height: 0.0, velocity: v });
    }
    wait_grounded(cx).await;
}

/// `0x41B8` → `0x41DC(221, 25, 60, 30, 180)`: the back hop. Face the target now; the crouch 221 at
/// blend 0, to its end; 222 (bit 16) and movement state 3 (`sys 82`: the facing turned 180° × 30,
/// height 60, 25 frames, drag 0.9) with its motion base (`sys 83`: six frames before the top 223,
/// with 224 queued, `0x1402AF630`); turning to the target until it lands.
async fn hop(cx: &Cx) {
    if let Some(t) = target(cx) {
        let d = t - pos(cx);
        let y = d.x.atan2(d.z);
        let mut io = cx.borrow_mut();
        io.body.yaw = y;
        io.body.yaw_target = y;
        io.out.push(ECmd::Yaw(y));
    }
    let yaw = cx.borrow().body.yaw;
    let d = Vec3::from_yaw(yaw + k(cx, "hop_angle").to_radians()) * k(cx, "hop_speed");
    let v = Vec3::new(d.x, k(cx, "hop_jump_height"), d.z);
    play(cx, req::HOP, 0.0);
    wait_done(cx).await;
    play(cx, req::HOP + 1, 0.0);
    let (drag, time) = (k(cx, "hop_drag"), k(cx, "hop_time"));
    cx.borrow_mut().body.timed_jump(v, drag, time);
    let mut apex = false;
    while cx.borrow().body.timed.is_some() {
        // `0x1402AF630` runs in the engine's step (after the script): here at the next frame's
        // script (Assumption: ±1 frame, as the Powerwild's).
        let (at_top, cur) = {
            let io = cx.borrow();
            (io.body.timed_apex(), io.w.motion)
        };
        if !apex && at_top && cur == Some(req::HOP + 1) {
            apex = true;
            play(cx, req::HOP + 2, 0.0);
            push(cx, ECmd::Queue { request: req::HOP + 3, blend: 0.0 });
        }
        face_target(cx);
        frame().await;
    }
}

/// `0x400A`: the block stance. Out of an arc; 230 at blend 4, to its end; turned to the target; 231
/// at blend 0 (`wp+0x84` set) for [60, 90) frames turning to the target, ended once the target is
/// more than 120° off its facing (`sys 100` < `sys 3(120)`); 233 at blend 0, to its end. Each part
/// makes KH1's blade row `0x15C` ([`Guard::Stance`]: attack type 2, priority 30): Sora's swing from
/// in front is stopped and both clash (`0x1402B4BC0`); the clash's hit state then hops back and
/// throws (`wp+0x84`).
async fn stance(cx: &Cx) {
    wait_arc(cx).await;
    play(cx, req::STANCE, k(cx, "stance_blend"));
    guard(cx, Guard::Stance, req::STANCE);
    wait_done(cx).await;
    face_target(cx);
    let hold = k(cx, "stance_hold_blend");
    play(cx, req::STANCE_HOLD, hold);
    guard(cx, Guard::Stance, req::STANCE_HOLD);
    cx.borrow_mut().x.blocking = true;
    let mut n = rand_range(cx, k(cx, "stance_min"), k(cx, "stance_max"), "stance time");
    let limit = k(cx, "stance_angle").to_radians().cos();
    while n - 0.0 > 0.0 {
        face_target(cx);
        if let Some(t) = target(cx) {
            let facing = Vec3::from_yaw(cx.borrow().body.yaw);
            if cos_between(facing, t - pos(cx)) - limit < 0.0 {
                break;
            }
        }
        n -= step(cx);
        frame().await;
    }
    play(cx, req::STANCE_END, hold);
    guard(cx, Guard::Stance, req::STANCE_END);
    wait_done(cx).await;
    cx.borrow_mut().x.blocking = false;
}

/// `sys 23` of a guard-type row (`0xEC` / `0x15C`) with the request it lives on.
fn guard(cx: &Cx, g: Guard, request: u16) {
    push(cx, ECmd::Guard(Some((g.index(), request))));
}

fn agrabah(cx: &Cx) -> bool {
    cx.borrow().x.world == Some(AGRABAH)
}

/// `0x107A`: close (< 200). 40 %: the back hop, the spin, a strafe; else a strafe, a pause, the
/// slash, the back hop, a strafe. Then the closing pause.
async fn close(cx: &Cx) {
    let turn = k(cx, "strafe_turn");
    if rand(cx, "close") - k(cx, "close_hop_chance") < 0.0 {
        hop(cx).await;
        spin(cx).await;
        let t = strafe_time(cx);
        strafe(cx, t, turn).await;
    } else {
        let t = strafe_time(cx);
        strafe(cx, t, turn).await;
        face_target(cx);
        round_pause(cx).await;
        slash(cx).await;
        hop(cx).await;
        let t = strafe_time(cx);
        strafe(cx, t, turn).await;
    }
    face_target(cx);
    round_pause(cx).await;
}

/// `0xFA8`: mid (< 400). A strafe (turn 3), a pause of [60, 90); 70 %: run in to 200 and slash,
/// a strafe; else the leap. Then the closing pause.
async fn mid(cx: &Cx) {
    let t = strafe_time(cx);
    strafe(cx, t, k(cx, "mid_strafe_turn")).await;
    face_target(cx);
    let n = rand_range(cx, k(cx, "mid_pause_min"), k(cx, "mid_pause_max"), "pause time");
    pause(cx, n).await;
    if rand(cx, "mid") - k(cx, "mid_slash_chance") < 0.0 {
        approach_to(cx, k(cx, "mid_approach_time"), k(cx, "mid_approach_stop")).await;
        slash(cx).await;
        let t = strafe_time(cx);
        strafe(cx, t, k(cx, "strafe_turn")).await;
    } else {
        leap(cx).await;
    }
    face_target(cx);
    round_pause(cx).await;
}

/// `0xD8E`: far. Agrabah: 30 % the throw then run in to 600, else 80 % the leap, else run in to
/// 600; the closing pause. Elsewhere: past 2000 from Sora the leash (not ported: nothing this
/// round), else run in to 400 and a pause of [30, 60).
async fn far(cx: &Cx) {
    let time = k(cx, "far_approach_time");
    if agrabah(cx) {
        let r = rand(cx, "far");
        if r - k(cx, "agrabah_throw_chance") < 0.0 {
            throw(cx).await;
            approach_to(cx, time, k(cx, "agrabah_far_stop")).await;
        } else if r - k(cx, "agrabah_leap_chance") < 0.0 {
            leap(cx).await;
        } else {
            approach_to(cx, time, k(cx, "agrabah_far_stop")).await;
        }
        face_target(cx);
        round_pause(cx).await;
    } else {
        let sora = {
            let io = cx.borrow();
            io.w.target.map_or(1.0e8, |t| (t - io.w.pos).length())
        };
        if sora - k(cx, "far_leash") > 0.0 {
            // `0x3B12(1)`: KH1's leash (it removes the Bandit when unseen): not ported.
            frame().await;
            return;
        }
        approach_to(cx, time, k(cx, "far_approach_stop")).await;
        face_target(cx);
        let n = rand_range(cx, k(cx, "far_pause_min"), k(cx, "far_pause_max"), "pause time");
        pause(cx, n).await;
    }
}

/// An idle wait of [238, 308) frames (`0x148C`), ended when Sora targets it (engaged).
async fn idle_wait(cx: &Cx) {
    play(cx, req::IDLE, k(cx, "idle_blend"));
    let mut n = rand_range(cx, k(cx, "wander_idle_min"), k(cx, "wander_idle_max"), "wander idle time");
    while n - 0.0 > 0.0 {
        if cx.borrow().x.targeted {
            set_target(cx, true);
            set_state(cx, st::ENGAGED);
            halt().await;
        }
        n -= step(cx);
        frame().await;
    }
}

/// `0x1346`: the wander. Two points about where it stands ([200, 400) and [400, 500)), walked to
/// (timeout 300 each); 70 %: 40 % of all an idle wait, then back to the idle routine; else in
/// Agrabah, farther than 500 from home, home (timeout 600) and an idle wait; again.
async fn wander(cx: &Cx) {
    loop {
        let c = |n: &str| k(cx, n);
        let p = pos(cx);
        let a = p + random_offset(cx, c("wander_a_min"), c("wander_a_max"));
        let p = pos(cx);
        let b = p + random_offset(cx, c("wander_b_min"), c("wander_b_max"));
        walk_to(cx, a, 0, c("wander_timeout")).await;
        walk_to(cx, b, 0, c("wander_timeout")).await;
        let r = rand(cx, "wander stop");
        if r - c("wander_stop_chance") < 0.0 {
            if r - c("wander_idle_chance") < 0.0 {
                idle_wait(cx).await;
            }
            return;
        }
        let home = cx.borrow().home;
        if agrabah(cx) && (home - pos(cx)).length() - c("home_max") > 0.0 {
            walk_to(cx, home, 0, c("home_timeout")).await;
            idle_wait(cx).await;
        }
        frame().await;
    }
}

/// The idle behaviour (`0x1190`): each round, Sora's target is it → engaged; else the wander.
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

/// The engaged behaviour (`0xB38`): by distance close / mid / far, then engaged again (the search
/// `0x2950(&wp40, 1, 50000, 1)`: Sora).
async fn engaged(cx: Cx) {
    let d = dist_to_target(&cx);
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

/// The lost behaviour (`0x8D0`): Sora's target is it → the throw; its remembered target (Sora,
/// alive) → engaged.
async fn lost(cx: Cx) {
    if cx.borrow().x.targeted {
        set_target(&cx, true);
        throw(&cx).await;
    }
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
}

/// The hit behaviour (`0x4FA`): out of an arc; a blocked swing during the stance → the back hop
/// and the throw; else past 400: 50 % the throw, else the leap; else after a 1st / 2nd hit seen:
/// 50 % the stance, 20 % the slash, else the back hop and the leap; else 50 % the leap, else the
/// spin. Outside Agrabah, then by distance: two slashes (< 200), the leap (< 400), the spin.
/// Then engaged.
async fn hit(cx: Cx) {
    wait_arc(&cx).await;
    let blocking = std::mem::take(&mut cx.borrow_mut().x.blocking);
    if blocking {
        hop(&cx).await;
        throw(&cx).await;
    } else if dist_to_target(&cx) - k(&cx, "hit_far") > 0.0 {
        if rand(&cx, "counter far") - k(&cx, "hit_throw_chance") < 0.0 {
            throw(&cx).await;
        } else {
            leap(&cx).await;
        }
    } else if cx.borrow().x.countered {
        let r = rand(&cx, "counter");
        if r - k(&cx, "hit_stance_chance") < 0.0 {
            stance(&cx).await;
        } else if r - k(&cx, "hit_slash_chance") < 0.0 {
            slash(&cx).await;
        } else {
            hop(&cx).await;
            leap(&cx).await;
        }
    } else if rand(&cx, "counter") - k(&cx, "hit_leap_chance") < 0.0 {
        leap(&cx).await;
    } else {
        spin(&cx).await;
    }
    if !agrabah(&cx) {
        let d = dist_to_target(&cx);
        if d - k(&cx, "hit_close") < 0.0 {
            face_target(&cx);
            slash(&cx).await;
            face_target(&cx);
            slash(&cx).await;
        } else if d - k(&cx, "hit_mid") < 0.0 {
            leap(&cx).await;
        } else {
            spin(&cx).await;
        }
    }
    set_state(&cx, st::ENGAGED);
}

/// The appearance (`0x174A` → `0x2C8A`, spawned by the room): home = here; Sora faced now; 69 at
/// blend 0 with the 30-frame fade-in (KH1's 30 frames of flag 0x80 and its effect: not ported);
/// 30 frames; the idle queued (`sys 14`, blend 0) with the action flag set: KH1 loops 69
/// (`0x14029F0A0`) and plays the idle at the next end of a cycle (`0x14029F6E0`), then the script
/// goes on; then the idle state. A Bandit Re:CoM placed itself (its alternate appearance): Sora
/// faced now (`0x190E`).
async fn appear(cx: Cx) {
    let spawned = cx.borrow().x.spawned;
    face_sora_now(&cx);
    if spawned {
        play(&cx, req::APPEAR, k(&cx, "appear_blend"));
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        wait_frames(&cx, k(&cx, "appear_wait")).await;
        let (len, played) = {
            let io = cx.borrow();
            (motion_frames(&cx, req::APPEAR), if io.clock.0 == Some(req::APPEAR) { io.clock.1 } else { 0.0 })
        };
        if len > 0.0 {
            let rest = (played / len).ceil() * len - played;
            wait_frames(&cx, rest).await;
        }
        play(&cx, req::IDLE, 0.0);
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The supervisor (`0x3272`): after the init frame, each frame: a state request restarts the
/// behaviour thread (the idle and lost states clear the target, the engaged and hit states set
/// it); while idle, Sora within `idle_search` → engaged.
async fn supervisor(cx: Cx) {
    frame().await;
    loop {
        if let Some(s) = take_state(&cx) {
            set_target(&cx, s == st::ENGAGED || s == st::HIT);
            push(&cx, ECmd::State(s));
        }
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
        frame().await;
    }
}

/// The Bandit's AI for one enemy.
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
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp { spawned, ..Default::default() });
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
            if w.grounded {
                io.x.floor = Some(w.pos.y);
            }
            tick_clock(&mut io);
        }
        if !self.started {
            self.started = true;
            {
                let mut io = self.io.borrow_mut();
                // `0x174A`: home (`wp+0x20`) = the position; the state object cleared (`0x2A18`).
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
    /// The hit handler (`0x367A`). A clash (kinds 6 / 7) asks for the hit state with the stance's
    /// flag kept (`0x3834`: the attacker known, state 4). A damaging hit clears the flag
    /// (`0x3894`), notes its 1st or 2nd hit of a run (`0x3936`: record `+0x24` is 1 or 2) and asks
    /// for the hit state; its kind 4 → 5 is [`Behaviour::reaction_kind`] (`0x3974`). The spin's
    /// landed flag goes (`0x376E`).
    fn on_hit(&self, news: HitNews) {
        {
            let mut io = self.io.borrow_mut();
            io.x.landed = false;
            if news == HitNews::Damage {
                io.x.blocking = false;
                if matches!(io.x.hit_count, 1 | 2) {
                    io.x.countered = true;
                }
            }
        }
        set_state(&self.io, st::HIT);
    }
    fn reaction_kind(&self, kind: u8) -> u8 {
        if kind == 4 {
            5
        } else {
            kind
        }
    }
    fn set_hit_count(&self, n: u16) {
        self.io.borrow_mut().x.hit_count = n;
    }
    fn set_world(&self, w: Option<u8>) {
        self.io.borrow_mut().x.world = w;
    }
    fn set_targeted(&self, by_sora: bool) {
        self.io.borrow_mut().x.targeted = by_sora;
    }
    /// The spin's row `0x2AC` landed (its hit callback `0x228A`: `wp+0x80` = 1; the others' callback
    /// `0x2438` only messages the group leader).
    fn on_landed(&self, attack: u8) {
        if attack == Attack::Spin.index() {
            self.io.borrow_mut().x.landed = true;
        }
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
    fn reset(&self) {
        set_state(&self.io, st::ENGAGED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consts() -> Consts {
        // The player's `ex_2090.bd` / `btltbl.bin` / KH1 exe numbers as the asset step reads them
        // (its manifest on the session's files), the exe's gravity and turn rate.
        let v: &[(&str, f32)] = &[("walk_speed", 2.0), ("run_speed", 8.0), ("idle_search", 1500.0), ("appear_blend", 0.0), ("appear_fade", 30.0), ("appear_wait", 30.0), ("wander_a_min", 200.0), ("wander_a_max", 400.0), ("wander_b_min", 400.0), ("wander_b_max", 500.0), ("wander_timeout", 300.0), ("wander_stop_chance", 0.7), ("wander_idle_chance", 0.4), ("wander_idle_min", 238.0), ("wander_idle_max", 308.0), ("home_max", 500.0), ("home_timeout", 600.0), ("idle_blend", 8.0), ("close_range", 200.0), ("mid_range", 400.0), ("close_hop_chance", 0.4), ("mid_strafe_turn", 3.0), ("mid_pause_min", 60.0), ("mid_pause_max", 90.0), ("mid_slash_chance", 0.7), ("mid_approach_time", 90.0), ("mid_approach_stop", 200.0), ("far_leash", 2000.0), ("far_approach_time", 120.0), ("far_approach_stop", 400.0), ("far_pause_min", 30.0), ("far_pause_max", 60.0), ("agrabah_throw_chance", 0.3), ("agrabah_far_stop", 600.0), ("agrabah_leap_chance", 0.8), ("pause_min", 60.0), ("pause_max", 75.0), ("strafe_short_chance", 0.5), ("strafe_mid_chance", 0.8), ("strafe_turn", 4.0), ("strafe_short_min", 30.0), ("strafe_short_max", 45.0), ("strafe_mid_min", 60.0), ("strafe_mid_max", 90.0), ("strafe_long_min", 120.0), ("strafe_long_max", 150.0), ("strafe_side_chance", 0.6), ("strafe_angle_first", -90.0), ("strafe_angle", 90.0), ("strafe_speed", 8.0), ("strafe_extra", 10.0), ("strafe_stop", 200.0), ("run_blend", 8.0), ("strafe_turn_factor", 0.5), ("pause_extra", 8.0), ("pause_stop", 400.0), ("approach_extra", 8.0), ("approach_retarget", 20.0), ("hit_far", 400.0), ("hit_throw_chance", 0.5), ("hit_stance_chance", 0.5), ("hit_slash_chance", 0.7), ("hit_leap_chance", 0.5), ("hit_close", 200.0), ("hit_mid", 400.0), ("hop_angle", 180.0), ("hop_speed", 30.0), ("hop_jump_height", 60.0), ("hop_time", 25.0), ("hop_drag", 0.9), ("attack_blend", 8.0), ("slash_blend", 8.0), ("slash_wait", 8.0), ("slash_effect_wait", 22.0), ("throw_blend", 8.0), ("throw_wait", 8.0), ("throw_release", 82.0), ("throw_hold", 290.0), ("spin_blend", 4.0), ("spin_windup", 34.0), ("spin_speed", 4.0), ("spin_loops", 4.0), ("spin_loop_blend", 0.0), ("spin_step", 12.0), ("spin_trim", 2.0), ("spin_end_wait_a", 12.0), ("spin_end_wait_b", 22.0), ("spin_end_wait_c", 16.0), ("leap_blend", 8.0), ("leap_lift", 50.0), ("leap_speed", 70.0), ("leap_arrive", 100.0), ("leap_timeout", 500.0), ("leap_air_blend", 0.0), ("leap_drop_height", 20.0), ("leap_stuck", 0.1), ("leap_stuck_half", 0.5), ("leap_end_wait", 24.0), ("stance_blend", 4.0), ("stance_hold_blend", 0.0), ("stance_min", 60.0), ("stance_max", 90.0), ("stance_angle", 120.0), ("air_accel", 0.1), ("air_drag", 0.95), ("fall_cap", 20.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.3499999), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("weight", 1.0)];
        let mut v: Vec<(String, f32)> = v.iter().map(|(k, x)| (k.to_string(), *x)).collect();
        v.push(("gravity".into(), crate::params::GRAVITY));
        v.push(("turn_rate".into(), std::f32::consts::PI / 16.0));
        Table::new(v, NAMES).unwrap()
    }

    /// The player's `xa_ex_2090.mset` lengths by request.
    fn frames() -> Vec<(u16, f32)> {
        vec![(0, 222.0), (1, 60.0), (2, 36.0), (3, 12.0), (4, 22.0), (5, 14.0), (6, 90.0), (7, 24.0), (66, 124.0), (67, 144.0), (68, 132.0), (69, 24.0), (70, 12.0), (72, 42.0), (73, 42.0), (74, 42.0), (75, 42.0), (78, 42.0), (79, 42.0), (80, 42.0), (81, 42.0), (82, 8.0), (83, 20.0), (84, 58.0), (85, 72.0), (86, 46.0), (87, 6.0), (88, 20.0), (89, 40.0), (90, 70.0), (91, 102.0), (92, 14.0), (93, 20.0), (94, 58.0), (95, 72.0), (96, 46.0), (97, 6.0), (98, 20.0), (99, 40.0), (100, 70.0), (101, 102.0), (102, 144.0), (103, 42.0), (104, 90.0), (200, 60.0), (201, 438.0), (202, 60.0), (205, 34.0), (210, 46.0), (211, 40.0), (212, 94.0), (213, 72.0), (216, 60.0), (217, 20.0), (218, 40.0), (221, 10.0), (222, 6.0), (223, 6.0), (224, 20.0), (225, 20.0), (227, 120.0), (228, 100.0), (230, 12.0), (231, 60.0), (232, 40.0), (233, 16.0), (235, 14.0), (236, 20.0), (237, 58.0), (238, 72.0), (239, 46.0), (240, 6.0), (241, 20.0), (242, 40.0), (243, 70.0), (244, 102.0)]
    }

    /// A flat floor at y = 0, Sora at `sora`; an action holds the AI for its motion's length; the
    /// current request is the last one played.
    struct Sim {
        ai: Ai,
        pos: Vec3,
        busy: f32,
        current: Option<u16>,
        moved: f32,
        log: Vec<Cmd>,
    }

    impl Sim {
        fn run(&mut self, sora: Vec3, n: usize) {
            for _ in 0..n {
                let (airborne, flying) = {
                    let b = self.ai.body();
                    (b.airborne(), b.flying)
                };
                let w = World { step: 1.0, pos: self.pos, target: Some(sora), grounded: !airborne && !flying, airborne, moved: self.moved, motion: self.current };
                if self.busy > 0.0 {
                    self.busy -= 1.0;
                } else {
                    for c in self.ai.frame(w) {
                        match c {
                            ECmd::Action { request, .. } => {
                                self.busy = frames().iter().find(|x| x.0 == request).map_or(30.0, |x| x.1);
                                self.current = Some(request);
                            }
                            ECmd::Motion { request, .. } | ECmd::Attack { request, .. } => self.current = Some(request),
                            ECmd::Queue { request, .. } => self.current = Some(request),
                            _ => {}
                        }
                        self.log.push(c);
                    }
                }
                let c = self.ai.consts();
                let steer = self.ai.take_steer();
                let d = self.ai.body().step(1.0, self.pos.y, steer, &c);
                self.pos += d;
                self.moved = d.length();
                if self.pos.y >= 0.0 && self.ai.body().falling() {
                    self.pos.y = 0.0;
                    self.ai.body().land();
                }
            }
        }
    }

    fn sim(seed: u32, spawned: bool, world: Option<u8>) -> Sim {
        let ai = Ai::new(consts(), frames(), seed, 0.0, spawned);
        Behaviour::set_world(&ai, world);
        Sim { ai, pos: Vec3::ZERO, busy: 0.0, current: None, moved: 0.0, log: Vec::new() }
    }

    /// A spawned Bandit past its appearance, idle, Sora far away.
    fn ready(seed: u32, world: Option<u8>) -> Sim {
        let mut s = sim(seed, true, world);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 60);
        assert!(s.ai.risen());
        s.log.clear();
        s
    }

    fn attacks(s: &Sim) -> Vec<Attack> {
        s.log.iter().filter_map(|c| if let ECmd::Attack { attack, .. } = c { Some(*attack) } else { None }).collect()
    }

    fn motions(s: &Sim) -> Vec<u16> {
        s.log.iter().filter_map(|c| match c {
            ECmd::Motion { request, .. } | ECmd::Attack { request, .. } => Some(*request),
            _ => None,
        }).collect()
    }

    #[test]
    fn the_names_cover_the_numbers() {
        assert!(Table::new(vec![], NAMES).is_err());
        assert_eq!((consts().num("leap_speed"), consts().num("stance_angle")), (70.0, 120.0));
    }

    /// `0x2C8A`: 69 at blend 0 with the fade-in; 30 frames; the queued idle at 69's next cycle end
    /// (frame 48 of its 24-frame loop); then the idle state and the wander.
    #[test]
    fn spawned_appearance_then_idle() {
        let mut s = sim(1, true, None);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 2);
        assert!(s.log.contains(&ECmd::FadeIn(30.0)) && s.log.contains(&ECmd::Motion { request: 69, blend: 0.0 }), "{:?}", s.log);
        let before = s.log.len();
        s.run(Vec3::new(0.0, 0.0, 5000.0), 46);
        assert!(!s.ai.risen(), "still in its appearance at frame 48: {:?}", &s.log[before..]);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 4);
        assert!(s.ai.risen() && s.log.contains(&ECmd::State(st::IDLE)));
        s.run(Vec3::new(0.0, 0.0, 5000.0), 10);
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Motion { request: req::WALK, .. })), "it wanders");
    }

    /// The search (1500) and Sora targeting it both engage (targeting: between the wander's walks
    /// and in its idle waits, `0x1190` / `0x148C`, so within a walk's 300-frame timeout).
    #[test]
    fn engages_within_its_search_or_when_targeted() {
        let mut s = ready(2, None);
        s.run(Vec3::new(0.0, 0.0, 1400.0), 3);
        assert!(s.log.contains(&ECmd::State(st::ENGAGED)), "{:?}", s.log);
        let mut s = ready(2, None);
        s.run(Vec3::new(0.0, 0.0, 1600.0), 3);
        assert!(!s.log.contains(&ECmd::State(st::ENGAGED)));
        Behaviour::set_targeted(&s.ai, true);
        s.run(Vec3::new(0.0, 0.0, 1600.0), 650);
        assert!(s.log.contains(&ECmd::State(st::ENGAGED)), "targeted: {:?}", s.log);
    }

    /// Close (< 200), mid (< 400), far: every attack shows up, the stance never (only a counter).
    #[test]
    fn close_mid_and_far_attack() {
        let mut seen = std::collections::HashSet::new();
        let mut thrown = false;
        for seed in 0..40 {
            let d = [150.0, 300.0, 900.0][seed as usize % 3];
            let mut s = ready(seed, Some(AGRABAH));
            s.run(Vec3::new(0.0, 0.0, d), 900);
            seen.extend(attacks(&s));
            thrown |= motions(&s).contains(&req::THROW);
            assert!(!motions(&s).contains(&req::STANCE), "the stance is only a counter");
        }
        for a in Attack::ALL {
            assert!(seen.contains(&a), "{a:?} never: {seen:?}");
        }
        assert!(thrown, "Agrabah's far round throws");
    }

    /// Outside Agrabah the far round only closes in (no throw, no leap), with a pause of [30, 60).
    #[test]
    fn far_outside_agrabah_closes_in() {
        for seed in 0..12 {
            let mut s = ready(seed, Some(11));
            s.ai.io.borrow_mut().x.targeted = true;
            s.run(Vec3::new(0.0, 0.0, 1200.0), 3);
            let n = s.log.len();
            s.run(Vec3::new(0.0, 0.0, 1200.0), 40);
            let m: Vec<u16> = s.log[n..].iter().filter_map(|c| if let ECmd::Motion { request, .. } = c { Some(*request) } else { None }).collect();
            assert!(m.contains(&req::RUN) && !m.contains(&req::THROW) && !m.contains(&req::LEAP_CROUCH), "seed {seed}: {m:?}");
        }
    }

    /// The handler: kind 4 → 5; a damaging 1st / 2nd hit of a run makes the near counter the
    /// stance, the slash or the back hop and leap; a later hit (3rd+) leaves the leap / spin.
    #[test]
    fn the_hit_count_picks_the_counter() {
        let s = ready(1, Some(AGRABAH));
        assert_eq!((s.ai.reaction_kind(4), s.ai.reaction_kind(3), s.ai.reaction_kind(5)), (5, 3, 5));
        let (mut stance, mut spin) = (false, false);
        for seed in 0..30 {
            let mut s = ready(seed, Some(AGRABAH));
            Behaviour::set_hit_count(&s.ai, 1);
            Behaviour::on_hit(&s.ai, HitNews::Damage);
            s.run(Vec3::new(0.0, 0.0, 300.0), 300);
            let m = motions(&s);
            stance |= m.contains(&req::STANCE);
            assert!(!m.contains(&req::SPIN_UP) || m.iter().position(|&r| r == req::SPIN_UP) > m.iter().position(|&r| r == req::SLASH || r == req::LEAP_CROUCH || r == req::STANCE), "seed {seed}: no spin as the counter after a 1st hit: {m:?}");
            let mut s = ready(seed, Some(AGRABAH));
            Behaviour::set_hit_count(&s.ai, 3);
            Behaviour::on_hit(&s.ai, HitNews::Damage);
            s.run(Vec3::new(0.0, 0.0, 300.0), 120);
            let m = motions(&s);
            assert!(!m.contains(&req::STANCE), "seed {seed}: {m:?}");
            spin |= m.contains(&req::SPIN_UP);
        }
        assert!(stance && spin);
    }

    /// A swing blocked during the stance (a clash with `wp+0x84` set): the back hop, then the throw.
    #[test]
    fn a_block_in_the_stance_hops_back_and_throws() {
        let mut s = ready(4, Some(AGRABAH));
        s.ai.io.borrow_mut().x.blocking = true;
        Behaviour::on_hit(&s.ai, HitNews::Clash);
        s.run(Vec3::new(0.0, 0.0, 300.0), 120);
        let m = motions(&s);
        let hop = m.iter().position(|&r| r == req::HOP).expect("the back hop");
        let throw = m.iter().position(|&r| r == req::THROW).expect("the throw");
        assert!(hop < throw, "{m:?}");
        // A damaging hit clears the flag: no throw for it.
        let s = ready(4, Some(AGRABAH));
        s.ai.io.borrow_mut().x.blocking = true;
        Behaviour::on_hit(&s.ai, HitNews::Damage);
        assert!(!s.ai.io.borrow().x.blocking);
    }

    /// Outside Agrabah a counter is followed by one more round by distance (two slashes close).
    #[test]
    fn outside_agrabah_a_counter_is_followed_up() {
        let mut slashes = 0;
        for seed in 0..20 {
            let mut s = ready(seed, None);
            Behaviour::set_hit_count(&s.ai, 3);
            Behaviour::on_hit(&s.ai, HitNews::Damage);
            s.run(Vec3::new(0.0, 0.0, 120.0), 700);
            slashes = slashes.max(attacks(&s).iter().filter(|&&a| a == Attack::Slash).count());
        }
        assert!(slashes >= 2, "{slashes}");
    }

    /// The leap: the crouch, then movement state 7 at 70 toward 50 above Sora (eased at 0.1 a
    /// frame); steering stops at the first frame within 100 of the point, and the flight velocity
    /// then eases off at 0.9 a frame (`0x1402CC000`), so it carries on through Sora during 218;
    /// then a drop to the floor.
    #[test]
    fn the_leap_flies_at_sora_and_lands() {
        let mut s = ready(5, Some(AGRABAH));
        s.ai.io.borrow_mut().x.targeted = true;
        s.ai.io.borrow_mut().has_target = true;
        s.ai.io.borrow_mut().state = (st::HIT, false);
        let cx = s.ai.io.clone();
        s.ai.behaviour = Some(Box::pin(async move {
            leap(&cx).await;
            halt().await;
        }));
        let sora = Vec3::new(0.0, 0.0, 700.0);
        let mut top = 0.0f32;
        let mut flew = false;
        let mut nearest = f32::INFINITY;
        for _ in 0..300 {
            s.run(sora, 1);
            top = top.min(s.pos.y);
            flew |= s.ai.body().air_flight;
            nearest = nearest.min((s.pos.z - sora.z).abs());
        }
        assert!(flew, "movement state 7");
        assert!(attacks(&s).contains(&Attack::Leap));
        assert!(motions(&s).contains(&req::LEAP_SLASH));
        assert!(nearest < 100.0, "it reaches Sora: {nearest}");
        assert!(s.pos.z > sora.z, "and carries on past him: z {}", s.pos.z);
        assert!(top < -30.0, "rises toward 50 above Sora: {top}");
        assert_eq!(s.pos.y, 0.0, "back on the floor");
        assert!(!s.ai.body().airborne());
    }

    /// The back hop: movement state 3, 30 a frame backward, 60 high, 25 frames, drag 0.9: about
    /// 30 × (1 − √0.9²⁶) / (1 − √0.9) ≈ 425 back.
    #[test]
    fn the_back_hop_travels_its_kh1_distance() {
        let mut s = ready(3, Some(AGRABAH));
        s.ai.io.borrow_mut().has_target = true;
        s.ai.io.borrow_mut().state = (st::HIT, false);
        let cx = s.ai.io.clone();
        s.ai.behaviour = Some(Box::pin(async move {
            hop(&cx).await;
            halt().await;
        }));
        s.run(Vec3::new(0.0, 0.0, 300.0), 60);
        let m = motions(&s);
        assert_eq!(&m[..2], &[req::HOP, req::HOP + 1], "{m:?}");
        assert!(m.contains(&(req::HOP + 2)) && s.log.iter().any(|c| matches!(c, ECmd::Queue { request: 224, .. })));
        let k = 0.9f32.sqrt();
        let want = -30.0 * (1.0 - k.powi(26)) / (1.0 - k);
        assert!((s.pos.z - want).abs() < 40.0, "z {} vs {want}", s.pos.z);
        assert!(!s.ai.body().airborne());
    }

    /// The spin glides at 4 for up to four rounds of 38 frames and ends early once it landed.
    #[test]
    fn the_spin_rounds_end_when_it_lands() {
        let run = |landed_at: Option<usize>| {
            let mut s = ready(6, Some(AGRABAH));
            s.ai.io.borrow_mut().has_target = true;
            s.ai.io.borrow_mut().state = (st::HIT, false);
            let cx = s.ai.io.clone();
            s.ai.behaviour = Some(Box::pin(async move {
                spin(&cx).await;
                halt().await;
            }));
            let mut f = 0;
            while !s.log.iter().any(|c| matches!(c, ECmd::Attack { attack: Attack::SpinEnd, .. })) && f < 1000 {
                if Some(f) == landed_at {
                    Behaviour::on_landed(&s.ai, Attack::Spin.index());
                }
                s.run(Vec3::new(0.0, 0.0, 2000.0), 1);
                f += 1;
            }
            f
        };
        let full = run(None);
        let early = run(Some(100));
        assert!(full > early + 60, "four rounds {full}, landed in the first {early}");
    }

    /// A long fight: the AI acts at least every 600 frames (the host's quiet watchdog), with Sora
    /// circling 2–7 m away and a hit every 150–400 frames (12 seeds, 20 000 frames each), in and
    /// out of Agrabah.
    #[test]
    fn a_long_fight_never_stalls() {
        for seed in 0..12u32 {
            let world = if seed % 2 == 0 { Some(AGRABAH) } else { None };
            let mut s = ready(seed, world);
            let (mut last, mut worst, mut next_hit) = (0usize, 0usize, 300usize);
            let mut r = Kh1Rand::new(seed + 100);
            let mut hits = 0u16;
            for f in 0..20_000usize {
                let a = f as f32 / 600.0;
                let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
                let sora = Vec3::new(a.sin() * rad, 0.0, a.cos() * rad);
                if f == next_hit {
                    hits = if r.next_f32() < 0.5 { 1 } else { hits + 1 };
                    Behaviour::set_hit_count(&s.ai, hits);
                    Behaviour::on_hit(&s.ai, if r.next_f32() < 0.15 { HitNews::Clash } else { HitNews::Damage });
                    s.busy = 34.0;
                    next_hit = f + 150 + (r.next_f32() * 250.0) as usize;
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

    /// The guard rows go with their requests (the shared enemy-guard step): the stance's blade row with
    /// 230, 231, 233; the spin's body row with 210, 211, 212.
    #[test]
    fn guard_rows_go_with_their_requests() {
        let guards = |s: &Sim| -> Vec<(u8, u16)> { s.log.iter().filter_map(|c| if let ECmd::Guard(Some(g)) = c { Some(*g) } else { None }).collect() };
        let mut s = ready(4, Some(AGRABAH));
        s.ai.io.borrow_mut().has_target = true;
        s.ai.io.borrow_mut().state = (st::HIT, false);
        let cx = s.ai.io.clone();
        s.ai.behaviour = Some(Box::pin(async move {
            stance(&cx).await;
            halt().await;
        }));
        s.run(Vec3::new(0.0, 0.0, 300.0), 200);
        let st = Guard::Stance.index();
        assert_eq!(guards(&s), vec![(st, 230), (st, 231), (st, 233)]);
        let mut s = ready(6, Some(AGRABAH));
        s.ai.io.borrow_mut().has_target = true;
        s.ai.io.borrow_mut().state = (st::HIT, false);
        let cx = s.ai.io.clone();
        s.ai.behaviour = Some(Box::pin(async move {
            spin(&cx).await;
            halt().await;
        }));
        s.run(Vec3::new(0.0, 0.0, 2000.0), 400);
        let sp = Guard::Spin.index();
        assert_eq!(guards(&s), vec![(sp, 210), (sp, 211), (sp, 212)]);
        assert_eq!(Guard::ALL.map(|g| g.row()), [0xEC, 0x15C]);
        assert!(Guard::ALL.iter().all(|g| g.requests().len() == 3));
    }

    #[test]
    fn attack_rows_and_indices() {
        assert_eq!(Attack::Leap.rows(), &[0x3FC, 0x38C]);
        assert_eq!(Attack::SpinEnd.index(), 2);
        assert_eq!(Attack::ALL.map(|a| a.request()), [200, 211, 212, 217]);
    }
}
