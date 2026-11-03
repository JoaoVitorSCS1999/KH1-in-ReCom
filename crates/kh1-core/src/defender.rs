//! KH1 FM's Defender (`xa_ex_2340.mdls` part 5, `ex_2340.bd`; its children `ex_2340_00.bd` (the flame) and
//! `ex_2340_01.bd` (the fireball), part 3), engine-agnostic: its AI script on the shared enemy framework
//! (`crate::enemy`). M11 (`docs/recom-m11-enemies.md` § The Defender). Re:CoM runs it natively: each part is the step of
//! a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), Re:CoM plays the motions, collides the body, the shield's
//! rows, its guard rows and the fireballs / flame (its own bullets), applies the damage.
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's scripts at checked offsets); nothing of
//! the game is in this file. Script offsets are the player's `ex_2340.bd`'s (Checked (files): the script disassembled
//! with `tools/kh1-ai-script.py`, code from `0x4C8`; the VM's ops and the syscalls named, Checked (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x2644`: init `0x27EC` (walk 2.5; **the shield's front row `0xEC`** in KH1's mode 3, kept until its death;
//!   handlers: hit `0x2BC0`, message `0x2A44`, the hit-effect filter `0x2A9C`; turn rate 12° a frame; records LV1 `0x14`
//!   / LV2 `0x80`); a yield; Sora the target; a partner object of kind 300 in the room (`sys 4(0xB0000 | 300)`, none in
//!   Re:CoM's battles: `wp+0x98` = 0); the appearance thread `0x9CA`; then each frame: the meshes shown and hittable
//!   once the appearance is over, the turn rate 2° while the walk (1) or the fireball's wind-up (210) plays, else 12°;
//!   a state request restarts the behaviour: 2 engaged `0x6EE`, 4 hit `0x514`.
//! - Its rounds keep its shield toward Sora: before most attacks the turn `0x17C0` (side steps 200 / 202 turning 0.45°
//!   a frame, the turn-around 201 turning 2.36°, the strike when he stays off its front).
//! - The hit handler `0x2BC0` ([`Ai`]'s `on_hit` / `armoured`): a repel record (its shield's front row turned a swing
//!   away) counts; the 5th in its fireballs, its flame or a quiet motion gives KH1's EXP bonus and the shield bash
//!   207 (else, in a quiet motion, the shield recoil 206). A hit of one of its own rows (its batted fireball) on its
//!   shield from the front: the EXP bonus, a knock-back and its hit state. A living damaging hit: no reaction (`sys 42`)
//!   during the strike 209, the spin 208, the turn-around 201, and the bash 207 from the front; else KH1's reaction
//!   and its hit state.
//!
//! Not ported (Re:CoM's battles only, decision M10): the partner object's rounds (`wp+0x98`: `0x7EC`, the hit state's
//! `0x5A2`, the messages 5403..5405 of `0x2A44`), the leash `0x3268` (`0x1700` / `0x1700`), Hollow Bastion's room rule
//! (`wp+0xB0`: hits from a non-Sora attacker of attribute 1 / 10 do nothing), the hit attributes (2: a Fire hit on its
//! shield from the front does nothing unless the flame plays; 3: a Blizzard hit there does nothing unless its fireball
//! plays; 1 / 8 / 9 / 10 / 11: every hit of Sora's is taken as attribute 0), the hurt part test (record `+0x20` = 2,
//! its shield: the front test stands for it, owner question), the effects (`sys 75` 2 / 10 / 14 / 15 / 18 / 19,
//! `sys 150` / `sys 153` the appearance's part fade: M13), the meshes hidden in the appearance (`sys 98` / `sys 99`),
//! the hit-effect filter `0x2A9C` (a look), the death routine `0x321C` (Re:CoM's defeat, prizes, EXP).

use crate::enemy::guard::ALWAYS;
use crate::enemy::script::{frame, motion_done, motion_frames, poll, push, rand, rand_range, set_state, set_target, set_turn_rate, steer, take_state, target, tick_clock, wait_frames};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Reward, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Defender's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// A side step turning it +0.45° a frame (`0x1A80`: bit 16, held at its end), 60 frames.
    pub const STEP_A: u16 = 200;
    /// The turn-around (`0x1B5E`: +2.36° a frame, bit 16), 76 frames.
    pub const TURN: u16 = 201;
    /// A side step turning it −0.45° a frame (`0x199A`, bit 16), 60 frames.
    pub const STEP_B: u16 = 202;
    /// The guard: up (action), 32 frames.
    pub const GUARD_UP: u16 = 203;
    /// The guard held (queued after 203, looping), 140 frames.
    pub const GUARD: u16 = 204;
    /// The guard down (action, blend 8), 26 frames.
    pub const GUARD_DOWN: u16 = 205;
    /// The shield recoil (action, blend 2), 60 frames.
    pub const RECOIL: u16 = 206;
    /// The shield bash (action with the row `0x3FC`, root motion), 208 frames, hit [120, 140).
    pub const BASH: u16 = 207;
    /// The spin (a plain request with the row `0x1CC` and the guard row `0x23C`), 120 frames, hit [44, 80).
    pub const SPIN: u16 = 208;
    /// The strike (action with the row `0x15C`, root motion), 128 frames, hit [10, 24).
    pub const STRIKE: u16 = 209;
    /// The fireball's wind-up (plain, 211 queued), 100 frames.
    pub const FIRE_UP: u16 = 210;
    /// The fireball (plain, root motion): the shot leaves at its start, 100 frames.
    pub const FIRE: u16 = 211;
    /// The flame (plain, the idle queued): the child's flame from frame 110, 280 frames.
    pub const FLAME: u16 = 212;
    /// The appearance (`0x9F8`, `sys 13`, blend 0), 66 frames.
    pub const APPEAR: u16 = 213;
}

/// The Defender's attacks with rows of its own, in the order the host indexes them (the manifest's `attacks`): its
/// fireballs and its flame are its children's (Re:CoM bullets, [`child`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    Strike,
    Spin,
    Bash,
}

impl Attack {
    pub const ALL: [Attack; 3] = [Attack::Strike, Attack::Spin, Attack::Bash];

    pub fn request(self) -> u16 {
        match self {
            Attack::Strike => req::STRIKE,
            Attack::Spin => req::SPIN,
            Attack::Bash => req::BASH,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Strike => "strike",
            Attack::Spin => "spin",
            Attack::Bash => "bash",
        }
    }

    /// The attack rows (script offsets, 0x70 bytes, `sys 23`) made with it: each one sphere on the shield (bone code
    /// 48, radius 100, type 0, priority 100, recoil 110).
    pub fn rows(self) -> &'static [usize] {
        match self {
            Attack::Strike => &[0x15C],
            Attack::Spin => &[0x1CC],
            Attack::Bash => &[0x3FC],
        }
    }

    pub fn index(self) -> u8 {
        self as u8
    }
}

/// Its children (`sys 94`'s sub-object index): the thrown objects the host makes with [`ECmd::Throw`].
pub mod child {
    /// `ex_2340_00.bd` with the row `0x38C`: the flame, a chain of spheres from the shield's mouth while 212 plays.
    pub const FLAME: u8 = 0;
    /// `ex_2340_01.bd` with the row `0x2AC` (batted: `0x31C`): a Re:CoM bullet turning 1° a frame toward Sora.
    pub const FIREBALL: u8 = 1;
}

/// The Defender's guard-type rows (the manifest's `guards`): its shield's front row `0xEC` (type 1, priority 99, 70°,
/// KH1's mode 3) and its spin's row `0x23C` (type 1, priority 100, all round, with 208).
pub const FRONT: u8 = 0;
pub const SPIN_GUARD: u8 = 1;

/// The names of the Defender's numbers (besides `crate::enemy::ENGINE_NAMES`): the behaviour's, then the host's
/// ([`HOST_NAMES`]).
pub const NAMES: &[&str] = &[
    "turn_deg",
    "turn_slow_deg",
    "close_range",
    "far_range",
    "close_flame_chance",
    "close_strike_above",
    "close_follow_range",
    "close_follow_flame_above",
    "close_follow_far",
    "far_approach_chance",
    "far_fire_above",
    "mid_approach_chance",
    "mid_fire_above",
    "approach_near",
    "approach_near_min",
    "approach_near_max",
    "approach_far_min",
    "approach_far_max",
    "walk_blend",
    "walk_wait",
    "approach_speed",
    "approach_stop",
    "stuck_speed",
    "idle_blend",
    "idle_wait",
    "guard_break_min",
    "guard_break_max",
    "guard_time_min",
    "guard_time_max",
    "guard_break_chance",
    "guard_cone",
    "guard_hold_wait",
    "guard_aim_cone",
    "guard_strike_range",
    "guard_far",
    "guard_close",
    "guard_close_strike",
    "guard_drop_blend",
    "recoil_blend",
    "flame_len_less",
    "flame_turn_cone",
    "flame_turn_after",
    "flame_turn_near",
    "flame_turn_chance",
    "fire_len_less",
    "shot_len_less",
    "strike_cone",
    "face_cone",
    "back_deg",
    "back_cone",
    "side_deg",
    "side_cone",
    "step_max",
    "done_cone",
    "step_len_less",
    "step_deg",
    "turnaround_deg",
    "spin_range",
    "spin_deg",
    "fall_accel_a",
    "fall_accel_b",
    "fall_max",
    "hit_far",
    "hit_far_fire_above",
    "hit_near_strike",
    "hit_near_spin_above",
    "repel_count",
    "exp_div",
    "exp_add",
];

/// The host's numbers (the manifest must hold them; `kh1-recom`'s `win::kh1_enemy` / `win::kh1_thrown` /
/// `win::kh1_flame` read them): its own rows' serial range (`sys 188`), the children's indices, the fireball child's
/// (`fire_*`), the flame child's (`flame_*`), the bone points (`<point>_bone` / `_x` / `_y` / `_z`) and the Re:CoM power
/// (`fire_recom_id`, `fire_kind`, `fire_radius`).
pub const HOST_NAMES: &[&str] = &[
    "own_row_first",
    "own_row_last",
    "flame_child",
    "fire_child",
    "fire_t",
    "fire_turn",
    "fire_speed_min",
    "fire_speed_max",
    "fire_life",
    "fire_fade",
    "fire_radius",
    "fire_recom_id",
    "fire_kind",
    "flame_request",
    "flame_delay",
    "flame_rehit",
    "flame_t0",
    "flame_lift0",
    "flame_t1",
    "flame_lift1",
    "flame_ring",
    "flame_life",
    "flame_r0",
    "flame_r1",
    "flame_grow_until",
    "flame_grow_div",
    "flame_tail",
    "fire_at_bone",
    "fire_at_x",
    "fire_at_y",
    "fire_at_z",
    "flame0_bone",
    "flame0_x",
    "flame0_y",
    "flame0_z",
    "flame1_bone",
    "flame1_x",
    "flame1_y",
    "flame1_z",
];

/// Every name a manifest must hold ([`NAMES`] and [`HOST_NAMES`]).
pub fn all_names() -> Vec<&'static str> {
    NAMES.iter().chain(HOST_NAMES).copied().collect()
}

/// The Defender's numbers.
pub type Consts = Table;

/// The Defender's state objects (`wp+0x20`; KH1's numbers).
pub mod st {
    pub const NONE: u8 = 0;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 4;
}

/// What the hit handler sorts the motion playing into (`0x1E58` → `wp+0xA0`): 0 a reaction, the fall, the recoil,
/// the bash, the strike, the turn-around or the spin; 1 its fireballs or its flame; 2 anything else (a quiet motion:
/// the idle, the walk, the side steps, the guard, the appearance).
pub fn busy_class(playing: Option<u16>) -> u8 {
    match playing {
        Some(72..=77 | req::AIR | req::RECOIL | req::BASH | req::STRIKE | req::TURN | req::SPIN) => 0,
        Some(req::FIRE | req::FIRE_UP | req::FLAME) => 1,
        _ => 2,
    }
}

/// The Defender's own work area: in its appearance (`wp+0x90`), the repel records counted (`wp+0x58`), the bash or the
/// recoil the next hit state plays (`wp+0xA8` / `wp+0xAC`), its fireball count (`wp+0x70`); spawned by the room
/// (`sys 132`), the appearance done, hittable (KH1's flag 0x40 clear); the last hit's kind, push direction and
/// source (one of its own rows: `sys 188` in [`own_row_first`, `own_row_last`]), the shield path taken, no reaction
/// for it, the request playing as the handler sees it, defeated, the commands its handler makes at once, the floor it
/// last stood on.
#[derive(Clone, Debug, Default)]
struct Wp {
    appearing: bool,
    repels: i32,
    bash: bool,
    recoil: bool,
    shots: i32,
    spawned: bool,
    appeared: bool,
    hittable: Option<bool>,
    hit_kind: u8,
    hit_dir: Vec3,
    own: bool,
    shield: bool,
    no_reaction: bool,
    cur: Option<u16>,
    dead: bool,
    now: Vec<ECmd<Attack>>,
    floor: Option<f32>,
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

use crate::enemy::Numbers as _;

/// `+0x70 != 0`: not in KH1's movement state 0.
fn off_ground(cx: &Cx) -> bool {
    !cx.borrow().w.grounded
}

/// `while (+0x70 != 0) yield` (the start of most routines).
async fn on_ground(cx: &Cx) {
    while off_ground(cx) {
        frame().await;
    }
}

/// The request playing as the script sees it (`sys 51`): the last one this frame asked for, else the host's.
fn playing(cx: &Cx) -> Option<u16> {
    let io = cx.borrow();
    io.out.iter().rev().find_map(|c| match c {
        ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Restart { request, .. } | ECmd::Attack { request, .. } | ECmd::Action { request, .. } => Some(*request),
        _ => None,
    }).or(io.w.motion)
}

/// `sys 12` (`0x14029F7C0`): a plain request, nothing when it already plays (bit 16, KH1's hold at the end, is the
/// clip's own: Re:CoM plays it as the asset step marked it; Assumption, a look past the scripts' waits); `sys 172`'s
/// clock restarts for a request that plays anew.
fn play(cx: &Cx, r: u16, blend: f32) {
    if playing(cx) != Some(r) {
        cx.borrow_mut().clock = (Some(r), 0.0);
        push(cx, ECmd::Motion { request: r, blend });
    }
}

/// `sys 13` (`0x14029F980`) of a plain request from its start: written whatever plays (the request playing starts
/// again: [`ECmd::Restart`]).
fn request(cx: &Cx, r: u16, blend: f32) {
    let again = playing(cx) == Some(r);
    cx.borrow_mut().clock = (Some(r), 0.0);
    push(cx, if again { ECmd::Restart { request: r, blend, start: 0.0 } } else { ECmd::Motion { request: r, blend } });
}

/// An action (`sys 13` with bit 24, the queued request with bit 30: blend 0): the script pauses until it ends.
async fn action(cx: &Cx, r: u16, attack: Option<Attack>, blend: f32) {
    cx.borrow_mut().clock = (Some(r), 0.0);
    push(cx, ECmd::Action { request: r, attack, then: req::IDLE, blend });
    frame().await;
}

/// `sys 36` toward the target (`0x23C2`): the yaw target only, the engine turns.
fn face(cx: &Cx) {
    crate::enemy::script::face_target(cx);
}

/// `sys 11` toward the target (`0x23EC`): the facing now.
fn face_now(cx: &Cx) {
    crate::enemy::script::face_target_now(cx);
}

/// `sys 10` on the facing then `sys 11` (`0x1A1A..0x1A46`): the facing turned by `deg` now (KH1's `sys 10` turns
/// (sin y, 0, cos y) to yaw + deg: Checked (decomp `0x1402B9AD0` → `0x1400E6C80`, the row-vector product
/// `0x1400E5AD0`; the axis matrix `0x1428672D0` taken as the identity, Assumption)).
fn turn_by(cx: &Cx, deg: f32) {
    let mut io = cx.borrow_mut();
    let y = crate::math::wrap_angle(io.body.yaw + deg.to_radians());
    io.body.yaw = y;
    io.body.yaw_target = y;
    io.out.push(ECmd::Yaw(y));
}

/// `0x2210(actor, target)`: the 3D distance to the target (none: a large value).
fn dist(cx: &Cx) -> f32 {
    crate::enemy::script::dist_to_target(cx)
}

/// `0x3362(deg, cone)` → `wp+0x5C`: true when the target lies **outside** `cone` degrees of the facing turned by `deg`
/// (both flat; `sys 100`'s cosine, 0 when either is zero, against `sys 3`'s cos(cone)).
pub fn outside(yaw: f32, me: Vec3, target: Option<Vec3>, deg: f32, cone: f32) -> bool {
    let Some(t) = target else { return true };
    let f = Vec3::from_yaw(yaw + deg.to_radians());
    let to = (t - me).flat();
    let l = to.length();
    let c = if l == 0.0 { 0.0 } else { (f.dot(to) / l).clamp(-1.0, 1.0) };
    c - cone.to_radians().cos() < 0.0
}

fn off(cx: &Cx, deg: f32, cone: f32) -> bool {
    let (yaw, me, t) = {
        let io = cx.borrow();
        (io.body.yaw, io.w.pos, target(cx))
    };
    outside(yaw, me, t, deg, cone)
}

/// `0x3344`: the idle (blend 8), 6 frames.
async fn idle_rest(cx: &Cx) {
    play(cx, req::IDLE, k(cx, "idle_blend"));
    wait_frames(cx, k(cx, "idle_wait")).await;
}

/// The rest of the behaviour thread after a state request (`misc8`): the supervisor replaces it at its next run.
async fn halt() {
    loop {
        frame().await;
    }
}

/// `0x1A80` / `0x199A` / `0x1B5E`: a turning step: on the ground; `r` (bit 16) for its length − 2 frames, each frame
/// `r` again (nothing while it plays) and the facing turned by `deg` × step now; cut off the ground or when the motion
/// is done (`sys 172`: a second step of the same request finds it done at once).
async fn turning_step(cx: &Cx, r: u16, deg: f32) {
    on_ground(cx).await;
    let mut t = motion_frames(cx, r) - k(cx, "step_len_less");
    play(cx, r, 0.0);
    while t - 0.0 > 0.0 {
        t -= step(cx);
        play(cx, r, 0.0);
        let d = deg * step(cx);
        turn_by(cx, d);
        frame().await;
        if off_ground(cx) || motion_done(cx) {
            break;
        }
    }
}

async fn turnaround(cx: &Cx) {
    let d = k(cx, "turnaround_deg");
    turning_step(cx, req::TURN, d).await;
}

/// `0x17C0`: keep the shield toward the target. On the ground. Sora within 30° of its front: nothing. Within 50° of
/// its back: the turn-around. Else side steps toward him (200 turning +0.45° unless he is within 80° of its −90° side:
/// 202 turning −0.45°), each followed by the check `0x192E` (within 35° of its front: done; within 50° of its back:
/// done, then the turn-around), at most 3, then the strike.
async fn turn_to_target(cx: &Cx) {
    on_ground(cx).await;
    let mut count = 0;
    if !off(cx, 0.0, k(cx, "face_cone")) {
        return;
    }
    if !off(cx, k(cx, "back_deg"), k(cx, "back_cone")) {
        turnaround(cx).await;
        return;
    }
    let plus = off(cx, k(cx, "side_deg"), k(cx, "side_cone"));
    let (mut done, mut back) = (false, false);
    while !done {
        let d = k(cx, "step_deg");
        if plus {
            turning_step(cx, req::STEP_A, d).await;
        } else {
            turning_step(cx, req::STEP_B, -d).await;
        }
        count += 1;
        if !off(cx, 0.0, k(cx, "done_cone")) {
            done = true;
        }
        if !off(cx, k(cx, "back_deg"), k(cx, "back_cone")) {
            back = true;
            done = true;
        }
        if count as f32 - k(cx, "step_max") >= 0.0 {
            done = true;
            strike_at(cx).await;
        }
        frame().await;
    }
    if back {
        turnaround(cx).await;
    }
}

/// `0x1D64`: the strike: on the ground; Sora off its front (30°): it turns to him (`sys 36`); the row `0x15C` and 209
/// (an action, blend 0) with the idle queued.
async fn strike_at(cx: &Cx) {
    on_ground(cx).await;
    if off(cx, 0.0, k(cx, "strike_cone")) {
        face(cx);
    }
    action(cx, req::STRIKE, Some(Attack::Strike), 0.0).await;
}

/// `0x16F8(mode)`: the strike after the turn (1) or as [`strike_at`] (2).
async fn strike(cx: &Cx, mode: u8) {
    on_ground(cx).await;
    if mode == 1 {
        turn_to_target(cx).await;
        action(cx, req::STRIKE, Some(Attack::Strike), 0.0).await;
    } else {
        strike_at(cx).await;
    }
}

/// `0x1C58`: the spin: on the ground; within 400: its rows `0x1CC` and `0x23C` (a guard row) and 208 (a plain request,
/// blend 0), each frame the facing turned by −3° × step, to its end; else the strike.
async fn spin(cx: &Cx) {
    on_ground(cx).await;
    if dist(cx) - k(cx, "spin_range") < 0.0 {
        cx.borrow_mut().clock = (Some(req::SPIN), 0.0);
        push(cx, ECmd::Attack { request: req::SPIN, attack: Attack::Spin, blend: 0.0 });
        push(cx, ECmd::Guard(Some((SPIN_GUARD, req::SPIN))));
        loop {
            frame().await;
            let d = k(cx, "spin_deg") * step(cx);
            turn_by(cx, d);
            if off_ground(cx) || motion_done(cx) {
                break;
            }
        }
    } else {
        strike_at(cx).await;
    }
}

/// `0x150C(n)`: the fireballs: on the ground; the turn; 210 (blend 0) with 211 queued for 211's length − 2 frames
/// (`sys 163` taken as true: drawn this frame); then the shot `0x15F6` while the count is above 0 (the shot sets it
/// to 0 first: one shot whatever `n`, Checked (files `0x1630`)).
async fn fireballs(cx: &Cx, n: i32) {
    on_ground(cx).await;
    turn_to_target(cx).await;
    let mut t = motion_frames(cx, req::FIRE) - k(cx, "fire_len_less");
    cx.borrow_mut().x.shots = n;
    request(cx, req::FIRE_UP, 0.0);
    push(cx, ECmd::Queue { request: req::FIRE, blend: 0.0 });
    while t - 0.0 > 0.0 {
        if motion_done(cx) {
            break;
        }
        t -= step(cx);
        frame().await;
    }
    while cx.borrow().x.shots > 0 {
        cx.borrow_mut().x.shots -= 1;
        shot(cx).await;
        frame().await;
    }
}

/// `0x15F6`: the shot: on the ground; 211 (blend 0) and the fireball child (`sys 94(actor, 1, target, 0x2AC, 0x31C)`)
/// at its start; for 211's length − 4 frames or until it is done; cut off the ground.
async fn shot(cx: &Cx) {
    on_ground(cx).await;
    let mut t = motion_frames(cx, req::FIRE) - k(cx, "shot_len_less");
    cx.borrow_mut().x.shots = 0;
    play(cx, req::FIRE, 0.0);
    push(cx, ECmd::Throw(child::FIREBALL));
    while t - 0.0 > 0.0 {
        t -= step(cx);
        frame().await;
        if off_ground(cx) {
            cx.borrow_mut().x.shots = 0;
            break;
        }
        if motion_done(cx) {
            break;
        }
    }
}

/// `0x1302`: the flame: on the ground; the turn; 212 (blend 0) with the idle queued and the flame child (`sys 94(actor,
/// 0, target, 0x38C, 0)`: it burns from frame 110); for 212's length − 2 frames: from frame 180 with Sora off its front
/// (100°): the idle, then within 500 50 % the turn-around else the spin, beyond a fireball; cut off the ground or
/// when 212 is done.
async fn flame(cx: &Cx) {
    on_ground(cx).await;
    turn_to_target(cx).await;
    let mut left = motion_frames(cx, req::FLAME) - k(cx, "flame_len_less");
    let mut t = 0.0;
    request(cx, req::FLAME, 0.0);
    push(cx, ECmd::Queue { request: req::IDLE, blend: 0.0 });
    push(cx, ECmd::Throw(child::FLAME));
    while left - 0.0 > 0.0 {
        t += step(cx);
        left -= step(cx);
        frame().await;
        if off_ground(cx) {
            break;
        }
        let behind = off(cx, 0.0, k(cx, "flame_turn_cone"));
        let d = dist(cx);
        if behind && t - k(cx, "flame_turn_after") > 0.0 {
            idle_rest(cx).await;
            if d - k(cx, "flame_turn_near") < 0.0 {
                if rand(cx, "flame turn") - k(cx, "flame_turn_chance") > 0.0 {
                    turnaround(cx).await;
                } else {
                    spin(cx).await;
                }
            } else {
                fireballs(cx, 1).await;
            }
            break;
        }
        if motion_done(cx) {
            break;
        }
    }
}

/// `0xD38`: the approach: on the ground; the turn; a walk of [100, 300) frames within 500 (else [300, 500)): the walk
/// (blend 8), 8 frames, then each frame the walk, turning to the target, along its facing at 2.5; within 250: the idle
/// and the strike; cut off the ground or stuck (moved under 1.25 × step); the idle unless it struck.
async fn approach(cx: &Cx) {
    on_ground(cx).await;
    turn_to_target(cx).await;
    let mut struck = false;
    let mut time = if dist(cx) - k(cx, "approach_near") < 0.0 {
        rand_range(cx, k(cx, "approach_near_min"), k(cx, "approach_near_max"), "approach time")
    } else {
        rand_range(cx, k(cx, "approach_far_min"), k(cx, "approach_far_max"), "approach time")
    };
    play(cx, req::WALK, k(cx, "walk_blend"));
    wait_frames(cx, k(cx, "walk_wait")).await;
    while time - 0.0 > 0.0 {
        play(cx, req::WALK, 0.0);
        time -= step(cx);
        face(cx);
        let dir = Vec3::from_yaw(cx.borrow().body.yaw);
        steer(cx, dir, k(cx, "approach_speed"));
        frame().await;
        if dist(cx) - k(cx, "approach_stop") < 0.0 {
            struck = true;
            idle_rest(cx).await;
            strike(cx, 2).await;
            break;
        }
        if off_ground(cx) {
            break;
        }
        if cx.borrow().w.moved - k(cx, "stuck_speed") * step(cx) < 0.0 {
            steer(cx, dir, 0.0);
            break;
        }
    }
    if !struck {
        idle_rest(cx).await;
    }
}

/// `0xEF6`: the guard: on the ground; a break-out threshold [30, 60), a time [60, 120), 50 % it may break out; only with
/// Sora within 40° of its front: 203 (an action) then 204 held; 10 frames; for the time, 204: Sora off its front (40°):
/// the idle and a fireball; may break out, fewer than the threshold left and Sora within 30°: the idle, then within 350
/// the strike, beyond 600 a fireball, else the flame; within 200: the idle, 50 % the strike else the flame; cut off the
/// ground. Unless it broke out, 205 (an action, blend 8) with the idle queued.
async fn guard(cx: &Cx) {
    on_ground(cx).await;
    let mut out = false;
    let threshold = rand_range(cx, k(cx, "guard_break_min"), k(cx, "guard_break_max"), "guard threshold");
    let mut time = rand_range(cx, k(cx, "guard_time_min"), k(cx, "guard_time_max"), "guard time");
    let may_break = rand(cx, "guard break") - k(cx, "guard_break_chance") > 0.0;
    if off(cx, 0.0, k(cx, "guard_cone")) {
        return;
    }
    cx.borrow_mut().clock = (Some(req::GUARD_UP), 0.0);
    push(cx, ECmd::Action { request: req::GUARD_UP, attack: None, then: req::GUARD, blend: 0.0 });
    frame().await;
    play(cx, req::GUARD, 0.0);
    wait_frames(cx, k(cx, "guard_hold_wait")).await;
    while time - 0.0 > 0.0 {
        play(cx, req::GUARD, 0.0);
        time -= step(cx);
        frame().await;
        if off(cx, 0.0, k(cx, "guard_cone")) {
            idle_rest(cx).await;
            fireballs(cx, 1).await;
            out = true;
            break;
        }
        if may_break && time - threshold < 0.0 && !off(cx, 0.0, k(cx, "guard_aim_cone")) {
            out = true;
            idle_rest(cx).await;
            let d = dist(cx);
            if d - k(cx, "guard_strike_range") < 0.0 {
                strike(cx, 2).await;
            } else if d - k(cx, "guard_far") > 0.0 {
                fireballs(cx, 1).await;
            } else {
                flame(cx).await;
            }
            break;
        }
        if dist(cx) - k(cx, "guard_close") < 0.0 {
            out = true;
            idle_rest(cx).await;
            if rand(cx, "guard close") - k(cx, "guard_close_strike") < 0.0 {
                strike(cx, 2).await;
            } else {
                flame(cx).await;
            }
            break;
        }
        if off_ground(cx) {
            out = true;
            break;
        }
    }
    if !out {
        let b = k(cx, "guard_drop_blend");
        action(cx, req::GUARD_DOWN, None, b).await;
    }
}

/// `0x1230`: the shield recoil: on the ground; 206 (an action, blend 2) with the idle queued.
async fn recoil(cx: &Cx) {
    on_ground(cx).await;
    let b = k(cx, "recoil_blend");
    action(cx, req::RECOIL, None, b).await;
}

/// `0x1280`: the shield bash: on the ground; facing the target now; the row `0x3FC` and 207 (an action, blend 0) with
/// the idle queued.
async fn bash(cx: &Cx) {
    on_ground(cx).await;
    face_now(cx);
    action(cx, req::BASH, Some(Attack::Bash), 0.0).await;
}

/// `0x8E0`: close (< 470): rand < 0.3 the flame; > 0.4 the strike after the turn, then within 400 the spin and (rand >
/// 0.3) the flame else a fireball, beyond 600 a fireball, else the approach; otherwise the spin.
async fn close(cx: &Cx) {
    let r = rand(cx, "close");
    if r - k(cx, "close_flame_chance") < 0.0 {
        flame(cx).await;
    } else if r - k(cx, "close_strike_above") > 0.0 {
        strike(cx, 1).await;
        let d = dist(cx);
        if d - k(cx, "close_follow_range") < 0.0 {
            spin(cx).await;
            if rand(cx, "close follow") - k(cx, "close_follow_flame_above") > 0.0 {
                flame(cx).await;
            } else {
                fireballs(cx, 2).await;
            }
        } else if d - k(cx, "close_follow_far") > 0.0 {
            fireballs(cx, 2).await;
        } else {
            approach(cx).await;
        }
    } else {
        spin(cx).await;
    }
}

/// `0x84C`: far (> 600): rand < 0.3 the approach, > 0.5 three fireballs (one shot), else the guard.
async fn far(cx: &Cx) {
    let r = rand(cx, "far");
    if r - k(cx, "far_approach_chance") < 0.0 {
        approach(cx).await;
    } else if r - k(cx, "far_fire_above") > 0.0 {
        fireballs(cx, 3).await;
    } else {
        guard(cx).await;
    }
}

/// `0x896`: mid: rand < 0.3 the approach, > 0.5 two fireballs (one shot), else the guard.
async fn mid(cx: &Cx) {
    let r = rand(cx, "mid");
    if r - k(cx, "mid_approach_chance") < 0.0 {
        approach(cx).await;
    } else if r - k(cx, "mid_fire_above") > 0.0 {
        fireballs(cx, 2).await;
    } else {
        guard(cx).await;
    }
}

/// The appearance (`0x9CA` → `0x9F8`, spawned by the room): in it (`wp+0x90`); its meshes hidden (not ported);
/// movement state 7 with no velocity (`sys 169`); unhittable (KH1's flag 0x40); off the ground: facing Sora now, falling
/// (its speed + 0.25 a frame to 20) to the floor (`sys 89`), then movement state 0; 213 (blend 0) to its end, turning
/// to the target (its part fade from frame 34 and the meshes at 54: looks, not ported); hittable; the idle (blend 0);
/// the engaged state.
async fn appear(cx: Cx) {
    if cx.borrow().x.spawned {
        cx.borrow_mut().x.appearing = true;
        cx.borrow_mut().body.air_fly(Vec3::ZERO);
        hittable(&cx, false);
        if off_ground(&cx) || cx.borrow().body.airborne() {
            face_now(&cx);
            let mut speed = 0.0f32;
            loop {
                speed += k(&cx, "fall_accel_a") * step(&cx) * k(&cx, "fall_accel_b");
                if speed - k(&cx, "fall_max") > 0.0 {
                    speed = k(&cx, "fall_max");
                }
                steer(&cx, Vec3::new(0.0, 1.0, 0.0), speed);
                play(&cx, req::IDLE, 0.0);
                face(&cx);
                frame().await;
                if height(&cx) - 0.0 <= 0.0 {
                    cx.borrow_mut().body.ground();
                    steer(&cx, Vec3::ZERO, 0.0);
                    break;
                }
            }
        }
        request(&cx, req::APPEAR, 0.0);
        loop {
            frame().await;
            face(&cx);
            if motion_done(&cx) || off_ground(&cx) {
                break;
            }
        }
        hittable(&cx, true);
        cx.borrow_mut().x.appearing = false;
        play(&cx, req::IDLE, 0.0);
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::ENGAGED);
}

/// `sys 89`: the height above the floor (KH1 y down), from the floor it last stood on (Assumption: Re:CoM's battle
/// floors are flat, as every port's; KH1 probes the floor under it, `0x1402999A0`).
fn height(cx: &Cx) -> f32 {
    let io = cx.borrow();
    io.x.floor.map_or(0.0, |f| f - io.w.pos.y)
}

/// KH1's flag 0x40 (`Cmd::Hittable`), on a change only.
fn hittable(cx: &Cx, on: bool) {
    let mut io = cx.borrow_mut();
    if io.x.hittable != Some(on) {
        io.x.hittable = Some(on);
        io.out.push(ECmd::Hittable(on));
    }
}

/// The engaged behaviour (`0x6EE`), a loop: the target dead → Sora (`0x341E`; always Sora); out of its appearance; the
/// partner's rounds (not ported); close (< 470), far (> 600) or mid; the leash (not ported); the search (Sora).
async fn engaged(cx: Cx) {
    loop {
        cx.borrow_mut().x.appearing = false;
        let d = dist(&cx);
        if d - k(&cx, "close_range") < 0.0 {
            close(&cx).await;
        } else if d - k(&cx, "far_range") > 0.0 {
            far(&cx).await;
        } else {
            mid(&cx).await;
        }
        set_target(&cx, true);
        frame().await;
    }
}

/// The hit behaviour (`0x514`): the bash (`wp+0xA8`) or the recoil (`wp+0xAC`) its handler asked for; else (no
/// partner) beyond 400: 70 % a fireball else the approach; within: < 0.3 the strike, > 0.7 the spin, else a fireball.
/// Then engaged.
async fn hit(cx: Cx) {
    let (b, r) = {
        let io = cx.borrow();
        (io.x.bash, io.x.recoil)
    };
    if b {
        bash(&cx).await;
    } else if r {
        recoil(&cx).await;
    } else if dist(&cx) - k(&cx, "hit_far") > 0.0 {
        if rand(&cx, "hit far") - k(&cx, "hit_far_fire_above") > 0.0 {
            fireballs(&cx, 1).await;
        } else {
            approach(&cx).await;
        }
    } else {
        let r = rand(&cx, "hit near");
        if r - k(&cx, "hit_near_strike") < 0.0 {
            strike(&cx, 2).await;
        } else if r - k(&cx, "hit_near_spin_above") > 0.0 {
            spin(&cx).await;
        } else {
            fireballs(&cx, 1).await;
        }
    }
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// The supervisor (`0x2644`): after the init frame and the set-up frame, each frame: hittable once out of the appearance (`0x26E4`: flag
/// 0x40 cleared, the meshes shown); the turn rate 2° while the walk (1) or the wind-up (210) plays, else 12°
/// (`0x2768`); a state request restarts the behaviour (with Sora the target).
async fn supervisor(cx: Cx) {
    // `0x2658`: the init frame's yield; `0x26E2`: the set-up frame's (Sora, the partner object, the appearance thread).
    frame().await;
    frame().await;
    loop {
        let appearing = cx.borrow().x.appearing;
        if !appearing {
            hittable(&cx, true);
        }
        let slow = matches!(playing(&cx), Some(req::WALK | req::FIRE_UP));
        let deg = if slow { k(&cx, "turn_slow_deg") } else { k(&cx, "turn_deg") };
        set_turn_rate(&cx, deg);
        if let Some(s) = take_state(&cx) {
            set_target(&cx, s != st::NONE);
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Defender's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as every port's: per-enemy seeds). `frames`: KH1 motion lengths
    /// by request. `spawned`: the room spawned it (its appearance plays; every Re:CoM battle appearance, owner
    /// 2026-11-02).
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, spawned: bool) -> Ai {
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp { spawned, ..Default::default() });
        Ai { io: Rc::new(RefCell::new(io)), main: None, behaviour: None, started: false }
    }

    pub fn consts(&self) -> Consts {
        self.io.borrow().c.clone()
    }

    /// One AI frame (the host calls it only while no action runs, or in a steered attack's state).
    pub fn frame(&mut self, w: World) -> Vec<Cmd> {
        {
            let mut io = self.io.borrow_mut();
            io.w = w;
            tick_clock(&mut io);
            if w.grounded {
                io.x.floor = Some(w.pos.y);
            }
        }
        if !self.started {
            self.started = true;
            {
                let mut io = self.io.borrow_mut();
                // `0x27EC`: home; the state object cleared; the shield's front row (mode 3).
                io.home = w.pos;
                io.state = (st::NONE, false);
                io.out.push(ECmd::Guard(Some((FRONT, ALWAYS))));
                io.has_target = true;
            }
            let deg = k(&self.io, "turn_deg");
            set_turn_rate(&self.io, deg);
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
                st::HIT => Box::pin(hit(cx)),
                _ => Box::pin(engaged(cx)),
            });
        }
        poll(&mut self.behaviour);
        std::mem::take(&mut self.io.borrow_mut().out)
    }

    /// KH1's push direction `dir` from its front: `sys 100` of its facing with the record's `+0x10` under 0 (Assumption,
    /// as every port: the host's direction for KH1's record).
    fn from_front(io: &crate::enemy::script::Core<Consts, Attack, Wp>, dir: Vec3) -> bool {
        let f = Vec3::from_yaw(io.body.yaw);
        let d = dir.flat();
        let l = d.length();
        l > 0.0 && f.dot(d) / l < 0.0
    }

    /// The handler's shield path (`0x2DA0..0x2E3C`): a hit from one of its own rows (`sys 188` in its range: its batted
    /// fireball) from the front (its shield; the hurt part test left out).
    fn shield(io: &crate::enemy::script::Core<Consts, Attack, Wp>) -> bool {
        io.x.own && Self::from_front(io, io.x.hit_dir)
    }

    /// Whether KH1's handler takes no reaction for this damaging hit (`0x3102..0x31B6`: `sys 42`): the bash 207 from the
    /// front; the strike 209, the spin 208, the turn-around 201 (attribute 1's flinch rule never comes: attribute 0).
    fn no_reaction(io: &crate::enemy::script::Core<Consts, Attack, Wp>, playing: Option<u16>) -> bool {
        match playing {
            Some(req::BASH) => Self::from_front(io, io.x.hit_dir),
            Some(req::STRIKE | req::SPIN | req::TURN) => true,
            _ => false,
        }
    }

    /// `sys 42` (`0x1402B7E60`): the flinch's push slide (`G+0x2C` × the stats block's `+0xC4`, the knock-back weight;
    /// `crate::enemy::react`'s).
    fn slide(io: &mut crate::enemy::script::Core<Consts, Attack, Wp>, dir: Vec3) {
        use crate::enemy::Engine;
        let p = io.c.flinch_push() * io.c.weight();
        io.body.push(dir, p);
    }

    /// The handler's start (`0x2BC0..0x2C10`): its flags cleared, out of its appearance.
    fn begin(io: &mut crate::enemy::script::Core<Consts, Attack, Wp>) {
        io.x.bash = false;
        io.x.recoil = false;
        io.x.shield = false;
        io.x.no_reaction = false;
        io.x.appearing = false;
    }

    /// KH1's EXP bonus (`sys 194(EXP / 10 + 1)`): carried out by the host at once (`take_now`).
    fn bonus(io: &mut crate::enemy::script::Core<Consts, Attack, Wp>) {
        let r = Reward { exp_div: io.c.num("exp_div") as i32, exp_add: io.c.num("exp_add") as i32, exp_min: 0, prizes: [0; 7] };
        io.x.now.push(ECmd::Reward(r));
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
    /// The hit handler (`0x2BC0`):
    /// - a repel record (its front row turned Sora's swing away): the count + 1; the 5th, in its fireballs or flame or a
    ///   quiet motion ([`busy_class`] 1 / 2): the count back to 0, KH1's EXP bonus and the bash next (`wp+0xA8`); short
    ///   of 5, in a quiet motion: the recoil next (`wp+0xAC`); either asks for the hit state. Off the ground: nothing.
    /// - a clash: nothing (`0x2100` answers −1; KH1's reaction plays its recoil);
    /// - a damaging hit: its batted fireball on its shield from the front: KH1's EXP bonus, a knock-back (kind 4) and the
    ///   hit state; a hit KH1 takes no reaction for ([`Behaviour::armoured`] couldn't take it first): no reaction;
    ///   else KH1's reaction and the hit state.
    fn on_hit(&self, news: HitNews) {
        let mut io = self.io.borrow_mut();
        Self::begin(&mut io);
        match news {
            HitNews::Repelled => {
                if !io.w.grounded {
                    return;
                }
                io.x.repels += 1;
                let class = busy_class(io.x.cur);
                if (io.x.repels as f32) - io.c.num("repel_count") >= 0.0 {
                    if class != 0 {
                        io.x.repels = 0;
                        Self::bonus(&mut io);
                        io.x.bash = true;
                        io.state = (st::HIT, true);
                    }
                } else if class == 2 {
                    io.x.recoil = true;
                    io.state = (st::HIT, true);
                }
            }
            HitNews::Clash => {}
            HitNews::Damage => {
                if matches!(io.x.hit_kind, 6 | 7) {
                    return;
                }
                if Self::shield(&io) {
                    io.x.shield = true;
                    Self::bonus(&mut io);
                    io.state = (st::HIT, true);
                    return;
                }
                let cur = io.x.cur;
                if Self::no_reaction(&io, cur) {
                    io.x.no_reaction = true;
                    let d = io.x.hit_dir;
                    Self::slide(&mut io, d);
                    return;
                }
                io.state = (st::HIT, true);
            }
        }
    }
    /// KH1's `sys 42` for a living damaging hit ([`Ai::no_reaction`]); never for a clash, nor for the shield path (its
    /// knock-back).
    fn armoured(&self, playing: Option<u16>, _launch_base: u16, kind: Option<u8>) -> bool {
        let io = self.io.borrow();
        if io.x.dead || matches!(kind, Some(6 | 7)) || Self::shield(&io) {
            return false;
        }
        Self::no_reaction(&io, playing)
    }
    fn on_armoured_hit(&self, dir: Vec3) {
        let mut io = self.io.borrow_mut();
        Self::begin(&mut io);
        Self::slide(&mut io, dir);
    }
    fn keeps_reaction(&self) -> bool {
        !self.io.borrow().x.no_reaction
    }
    /// The handler's kinds: the shield path's knock-back (kind 4, `0x2E28`); the killing hit's flinch becomes a knock-back
    /// (`0x3074`: kind 3 → 4).
    fn reaction_kind(&self, kind: u8) -> u8 {
        let io = self.io.borrow();
        if io.x.shield || (io.x.dead && kind == 3) {
            4
        } else {
            kind
        }
    }
    /// The killing hit: its shield path still gives the EXP bonus (`sys 194` before `sys 44`); its death removes the
    /// front row (`0x321C`: the host's defeat ends its guard rows).
    fn on_defeat(&self) {
        let mut io = self.io.borrow_mut();
        Self::begin(&mut io);
        if Self::shield(&io) {
            io.x.shield = true;
            Self::bonus(&mut io);
        }
        io.x.dead = true;
    }
    fn take_now(&self) -> Vec<ECmd<u8>> {
        std::mem::take(&mut self.io.borrow_mut().x.now).iter().map(|c| c.map(Attack::index)).collect()
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
    fn set_hit_dir(&self, dir: Vec3) {
        self.io.borrow_mut().x.hit_dir = dir;
    }
    fn set_hit_own_row(&self, own: bool) {
        self.io.borrow_mut().x.own = own;
    }
    fn set_playing(&self, request: Option<u16>) {
        self.io.borrow_mut().x.cur = request;
    }
    fn state_name(&self, s: u8) -> &'static str {
        match s {
            st::NONE => "none",
            st::ENGAGED => "engaged",
            st::HIT => "hit",
            _ => "?",
        }
    }
    fn reset(&self) {
        set_state(&self.io, st::ENGAGED);
    }
}

#[cfg(test)]
mod tests;
