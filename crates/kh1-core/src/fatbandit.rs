//! KH1 FM's Fat Bandit (`xa_ex_2060.mdls` part 5, `ex_2060.bd`; its children `ex_2060_00.bd` (the fireball) and
//! `ex_2060_01.bd` (the flame), part 3), engine-agnostic: its AI script on the shared enemy framework
//! (`crate::enemy`). M11 (`docs/recom-m11-enemies.md` § The Fat Bandit). Re:CoM runs it natively: each part is the
//! step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), Re:CoM plays the motions, collides the body, the
//! belly's rows, the front guard and the fireballs / flame (its own bullets), applies the damage.
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's scripts at checked offsets); nothing of
//! the game is in this file. Script offsets are the player's `ex_2060.bd`'s (Checked (files): the script disassembled
//! with `tools/kh1-ai-script.py`, code from `0x4C0`; the VM's ops and the syscalls named, Checked (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x24E8`: init `0x27A0` / `0x280C` (records LV1 `0x14` / LV2 `0x80`; walk 2, run 6; turn rate 10° a frame;
//!   the head look's cone and limits `0x3810`; the hit handler `0x29BC`); a yield; the party handles; the appearance
//!   thread `0x1132`; **the front row `0xEC`** (`sys 23`, `|= 512`, `sys 213(row, 3)`: mode 3, tested every frame);
//!   then each frame: while idle the search (a room-spawned Fat Bandit finds Sora within 1500) → engaged with the
//!   target; a state request restarts the behaviour thread: 1 idle `0xCDE`, 2 engaged `0xA56`, 3 lost `0x83E`, 4 hit
//!   `0x4F2`.
//! - Every motion it plays is a plain request (no action bit 24): its script never pauses for one (only the
//!   appearance's flag 4 holds it), so a state request takes effect the next frame whatever plays.
//! - the hit handler `0x29BC` ([`Ai`]'s `on_armoured_hit` / `on_hit`): the turn rate back to 10; a clash (kinds 6 / 7)
//!   or a repel record: the hit state. A damaging hit (`sys 44`): kind 5 → 4 (`0x2B4C`); the killing hit: its death
//!   `0x30F8` (Re:CoM's defeat here). Alive: a non-flinch hit (kind ≠ 3) on its front (`sys 100` of its facing with
//!   the way to the attacker above cos 80°) while its front row is down (the breath's wind-up) — or on its hurt part
//!   id 2 (not carried: owner question) — **staggers** it: the front row back, `wp+0x7C` = 1, `sys 42`, the hit state;
//!   else, the front row down → back up; the count `wp+0x70` (past 3 → 0; a run's first hit +1 for a flinch, +2 for
//!   another kind; its second +1); then **`sys 42` and nothing else**: KH1's Fat Bandit takes no reaction and asks for
//!   no state from an ordinary hit (`0x308E`: the kind zeroed; every damaging kind reaches it, 5 having become 4).
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0x37CC` (`0x18E4(1500, 900)`: KH1 removes it 15 m
//! from Sora, 9 m out of sight) and the vanishing (`0x209E`, request 70), the party handles and messages (`sys 81`,
//! `wp+0x18`'s object), Agrabah's room rules (the rooftop vanish, the narrow rooms' walk instead of the run: Re:CoM's
//! battle arenas take the run), the home return (only above its home in Agrabah: flat battle floors never are), the
//! death routine `0x30F8` / `0x1FD8` (Re:CoM's defeat, prizes, EXP), the effects (`sys 75` 1 / 2 / 3 / 4 / 18: M13),
//! the appearance's flag 0x80, the fade-in (no Re:CoM alpha), the head look (`sys 121`: the model's own; the
//! fireball's aim takes its direction, [`fire_dir`]), the hit attributes (1, 2, 3, 8..11: every hit of Sora's is
//! taken as attribute 0), the hurt part id 2 test (record `+0x20`).

use crate::enemy::guard::ALWAYS;
use crate::enemy::script::{face_target, frame, motion_done, poll, pos, push, rand, random_offset, set_state, set_target, set_turn_rate, steer, take_state, target, tick_clock, wait_frames, wait_grounded};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Reward, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Fat Bandit's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The appearance (`0x20F8`: 69, a plain request, blend 0), 102 frames.
    pub const APPEAR: u16 = 69;
    /// The belly (`0x1750`: a plain request with its rows, blend 14), 158 frames, both hands, hits [54, 68) / [84, 106)
    /// and [50, 72) / [86, 100).
    pub const BELLY: u16 = 200;
    /// The fireball's motion (`0x16FC`, blend 8; the volley's again from 12, blend 4), 108 frames, root motion; the
    /// fireball leaves 64 (48 from 12) frames in.
    pub const FIRE: u16 = 201;
    /// The belly rows' recoil (`+0x40` 0xCA), 52 frames.
    pub const BELLY_RECOIL: u16 = 202;
    /// The breath's wind-up (`0x1308`, blend 8), 180 frames.
    pub const WINDUP: u16 = 205;
    /// The breath (blend 8; from frame 40 at blend 18 in the counters), 264 frames: the flame lives while it plays.
    pub const BREATH: u16 = 207;
    /// The stagger (the hit state `0x6B8`: `0x400000D0`, blend 0), 268 frames.
    pub const STAGGER: u16 = 208;
}

/// The Fat Bandit's attacks with rows of its own, in the order the host indexes them (the manifest's `attacks`): its
/// fireballs and its flame are its children's (Re:CoM bullets, [`child`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    Belly,
}

impl Attack {
    pub const ALL: [Attack; 1] = [Attack::Belly];

    pub fn request(self) -> u16 {
        req::BELLY
    }

    pub fn name(self) -> &'static str {
        "belly"
    }

    /// The attack rows (script offsets, 0x70 bytes, `sys 23`) made with it: both hands (`0x31C` bone code 25, `0x38C`
    /// code 24; t 1.3 → 0, radius 60, priority 30, recoil 202).
    pub fn rows(self) -> &'static [usize] {
        &[0x31C, 0x38C]
    }

    pub fn index(self) -> u8 {
        0
    }
}

/// Its children (`sys 94`'s sub-object index): the thrown objects the host makes with [`ECmd::Throw`].
pub mod child {
    /// `ex_2060_00.bd` with the row `0x1CC` (the first shot) or `0x23C` (the volley's: the same numbers): a Re:CoM
    /// bullet flying straight at 45 a frame for 240 frames along the head's direction ([`super::fire_dir`]).
    pub const FIREBALL: u8 = 0;
    /// `ex_2060_01.bd` with the row `0x15C`: the flame, a chain of spheres from the mouth while 207 plays
    /// ([`super::flame_spheres`]).
    pub const FLAME: u8 = 1;
}

/// The Fat Bandit's guard-type rows (the manifest's `guards`): its belly's front row `0xEC` (bone code 12, t 0 → 0.7,
/// 110, type 1, priority 100, recoil 110, cone turn 0 / cos 0.342 = 70°; mode 3).
pub const FRONT: u8 = 0;

/// The names of the Fat Bandit's numbers (besides `crate::enemy::ENGINE_NAMES`): the behaviour's, then the host's
/// (the children's and the bone points, [`HOST_NAMES`]).
pub const NAMES: &[&str] = &[
    "turn_deg",
    "idle_search",
    "lost_search",
    "wander_inner_min",
    "wander_inner_max",
    "wander_outer_min",
    "wander_outer_max",
    "wander_speed",
    "wander_time",
    "wander_reaim",
    "wander_stop",
    "wander_done_chance",
    "wander_idle_chance",
    "idle_blend",
    "wander_idle_min",
    "wander_idle_max",
    "close_range",
    "mid_range",
    "close_belly_chance",
    "close_fire_chance",
    "mid_volley_chance",
    "mid_approach_time",
    "mid_approach_dist",
    "far_breath_chance",
    "far_approach_time",
    "far_approach_dist",
    "approach_speed",
    "approach_reaim",
    "route_turn_deg",
    "belly_blend",
    "belly_wait",
    "windup_blend",
    "windup_open",
    "windup_hold",
    "breath_blend",
    "breath_wait",
    "breath_aim_wait",
    "flame_time",
    "combo_gap",
    "combo_blend",
    "combo_from",
    "combo_wait",
    "quick_blend",
    "quick_from",
    "quick_wait",
    "fire_blend",
    "fire_from",
    "fire_wait",
    "volley_count",
    "volley_gap",
    "volley_blend",
    "volley_from",
    "volley_wait",
    "volley_rest",
    "appear_blend",
    "appear_fade",
    "appear_wait",
    "appear_then_blend",
    "stagger_front_deg",
    "count_reset",
    "count_flinch",
    "count_other",
    "count_second",
    "hit_sora_chance",
    "stagger_exp_div",
    "stagger_exp_min",
    "hit_count_max",
    "hit_flame_chance",
];

/// The host's numbers (the manifest must hold them; `kh1-recom`'s `win::kh1_enemy` / `win::kh1_thrown` /
/// `win::kh1_flame` read them): the fireball child's (`fire_*`), the head look's (`look_*`), the flame child's
/// (`flame_*`), the bone points (`<point>_bone` / `_x` / `_y` / `_z`: a KH1 bone code's point at t in the frame of
/// the Re:CoM bone standing for the code's parent, metres, as the asset step's records) and the Re:CoM power
/// (`fire_recom_id`, `fire_kind_index`, `fire_radius`).
pub const HOST_NAMES: &[&str] = &[
    "fire_t",
    "fire_turn",
    "fire_speed",
    "fire_life",
    "fire_fade",
    "fire_radius",
    "fire_recom_id",
    "fire_kind",
    "look_cone_in",
    "look_cone_out",
    "look_up",
    "look_down",
    "look_left",
    "look_right",
    "flame_rehit",
    "flame_t0",
    "flame_life",
    "flame_lift0",
    "flame_t1",
    "flame_lift1",
    "flame_r0",
    "flame_r1",
    "flame_grow",
    "flame_grow_div",
    "fire_at_bone",
    "fire_at_x",
    "fire_at_y",
    "fire_at_z",
    "eye_bone",
    "eye_x",
    "eye_y",
    "eye_z",
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

/// The Fat Bandit's numbers.
pub type Consts = Table;

/// The Fat Bandit's state objects (`wp+0x50`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const LOST: u8 = 3;
    pub const HIT: u8 = 4;
}

/// The Fat Bandit's own work area: the counted hits (`wp+0x70`), staggered (`wp+0x7C`), its front row down for the
/// breath (`wp+0x84` = 1), the row made (`wp+0x20`), a target picked before (`wp+0x4`), Sora's target is it
/// (`*(party 0 + 0x74)`); spawned by the room (`sys 132`), the appearance done, paused by it (flag 4 while 69 plays);
/// the last hit's kind and push direction, its place in its run (record `+0x24`), defeated, the commands its handler
/// makes at once, the request playing as the handler sees it.
#[derive(Clone, Debug, Default)]
struct Wp {
    hits: i32,
    stagger: bool,
    open: bool,
    front: bool,
    picked: bool,
    targeted: bool,
    spawned: bool,
    appeared: bool,
    paused: bool,
    hit_kind: u8,
    hit_dir: Vec3,
    hit_count: u16,
    dead: bool,
    now: Vec<ECmd<Attack>>,
    cur: Option<u16>,
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

/// The request playing as the script sees it (`sys 51`): the last one this frame asked for, else the host's.
fn playing(cx: &Cx) -> Option<u16> {
    let io = cx.borrow();
    io.out.iter().rev().find_map(|c| match c {
        ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Restart { request, .. } | ECmd::Attack { request, .. } | ECmd::Action { request, .. } => Some(*request),
        _ => None,
    }).or(io.w.motion)
}

/// `sys 12` (`0x14029F7C0`): a plain request, nothing when it already plays; `sys 172`'s clock restarts for a request
/// that plays anew.
fn play(cx: &Cx, r: u16, blend: f32) {
    if playing(cx) != Some(r) {
        cx.borrow_mut().clock = (Some(r), 0.0);
        push(cx, ECmd::Motion { request: r, blend });
    }
}

/// `sys 13` (`0x1402B9F80` → `0x14029F980`): a plain request written whatever plays, from `start` KH1 frames: the
/// request playing starts again ([`ECmd::Restart`]).
fn request(cx: &Cx, r: u16, blend: f32, start: f32) {
    let again = playing(cx) == Some(r);
    cx.borrow_mut().clock = (Some(r), start);
    push(
        cx,
        if again {
            ECmd::Restart { request: r, blend, start }
        } else if start > 0.0 {
            ECmd::MotionAt { request: r, blend, start }
        } else {
            ECmd::Motion { request: r, blend }
        },
    );
}

/// `0x1C7C`: the fall (6) plays: the waits below stop.
fn falling(cx: &Cx) -> bool {
    playing(cx) == Some(req::AIR)
}

/// `0x1C04(n)`: n frames turning to the target each frame (`0x21E6`: `sys 36`), cut short when 6 plays.
async fn wait_turning(cx: &Cx, n: f32) {
    let mut t = 0.0;
    while t - n < 0.0 {
        if falling(cx) {
            break;
        }
        face_target(cx);
        t += step(cx);
        frame().await;
    }
}

/// `0x1C58`: until the motion is done (`sys 172`) or 6 plays (tested before each yield).
async fn wait_done(cx: &Cx) {
    while !motion_done(cx) && !falling(cx) {
        frame().await;
    }
}

/// `0x21C8`: a yield, then until the motion is done (`sys 172`).
async fn wait_done_after(cx: &Cx) {
    loop {
        frame().await;
        if motion_done(cx) {
            break;
        }
    }
}

/// The rest of the behaviour thread after a state request (`misc8`): the supervisor replaces it at its next run.
async fn halt() {
    loop {
        frame().await;
    }
}

/// `0x1444` (and `0x254E` / `0x2ED6`): the front row made (`sys 23` with `0xEC`, `sys 213(row, 3)`, the actor mask
/// `512`) when it is down (`wp+0x84` = 1) and the Fat Bandit lives; `now`: from its hit handler (the host applies it
/// at once).
fn front_back(io: &mut crate::enemy::script::Core<Consts, Attack, Wp>, now: bool) {
    if io.x.open && !io.x.dead {
        io.x.open = false;
        front_on(io, now);
    }
}

fn front_on(io: &mut crate::enemy::script::Core<Consts, Attack, Wp>, now: bool) {
    if !io.x.front {
        io.x.front = true;
        let c = ECmd::Guard(Some((FRONT, ALWAYS)));
        if now {
            io.x.now.push(c);
        } else {
            io.out.push(c);
        }
    }
}

/// `0x13DC`: the front row removed for the breath (`wp+0x84` = 1, `sys 213(row, 0)`, the mask cleared), once.
fn front_off(cx: &Cx) {
    let mut io = cx.borrow_mut();
    if !io.x.open {
        io.x.open = true;
        if io.x.front {
            io.x.front = false;
            io.out.push(ECmd::Guard(None));
        }
    }
}

/// `sys 11` toward Sora (`0x2210(actor, party 0)`): the facing at once.
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

/// Where a route goes: a fixed point, or the target's live position (`0x35E0` passes `actor+0x74`'s `+0x10`: read
/// each turn of the route).
#[derive(Clone, Copy, Debug)]
enum Goal {
    At(Vec3),
    Target,
}

/// `0x36BE(goal, motion, speed, time, reaim, stop)`: the turn rate 5° a frame; while time is left and the goal is
/// farther than `stop` (3D): the yaw toward the goal (`sys 36`, flat), then for `reaim` frames the facing as it is
/// now (`actor+0x30`) with the speed (`+0x20` / `+0x2C`) and the motion (`sys 12`, blend 8), then a frame with no
/// speed (the actor task zeroes `+0x2C`, `0x140297FE0`); the time goes down by the `reaim` frames only. The turn rate
/// back to 10.
async fn route(cx: &Cx, goal: Goal, motion: u16, speed: f32, time: f32, reaim: f32, stop: f32) {
    set_turn_rate(cx, k(cx, "route_turn_deg"));
    let mut time = time;
    while time - 0.0 > 0.0 {
        let p = match goal {
            Goal::At(p) => p,
            Goal::Target => match target(cx) {
                Some(t) => t,
                None => break,
            },
        };
        let me = pos(cx);
        if (p - me).length() - stop < 0.0 {
            break;
        }
        let d = (p - me).flat();
        cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
        let dir = Vec3::from_yaw(cx.borrow().body.yaw);
        let blend = crate::enemy::MOVE_BLEND;
        let mut t = 0.0;
        while t - reaim < 0.0 {
            steer(cx, dir, speed);
            play(cx, motion, blend);
            t += step(cx);
            frame().await;
        }
        time -= t;
        frame().await;
    }
    let deg = k(cx, "turn_deg");
    set_turn_rate(cx, deg);
}

/// `0x35E0(time, dist)`: the leash (not ported); the head looks at the target (not ported); at the target, running
/// (2 at 6 a frame, the way taken every 6 frames) until within `dist` or the time is out.
async fn approach(cx: &Cx, time: f32, dist: f32) {
    let (speed, reaim) = (k(cx, "approach_speed"), k(cx, "approach_reaim"));
    route(cx, Goal::Target, req::RUN, speed, time, reaim, dist).await;
}

/// `0x1750`: the belly: the leash (not ported), the look off, turning to the target, its two rows (`0x31C` / `0x38C`,
/// their hit callback `0x17F0` messages `wp+0x18`'s object: not ported) and 200 (blend 14, a plain request: Re:CoM's
/// attack state, the AI runs on); to its end (or 6); the idle (blend 8); 75 frames; on the ground.
async fn belly(cx: &Cx) {
    face_target(cx);
    {
        let mut io = cx.borrow_mut();
        io.clock = (Some(req::BELLY), 0.0);
    }
    let b = k(cx, "belly_blend");
    push(cx, ECmd::Attack { request: req::BELLY, attack: Attack::Belly, blend: b });
    wait_done(cx).await;
    request(cx, req::IDLE, k(cx, "idle_blend"), 0.0);
    wait_frames(cx, k(cx, "belly_wait")).await;
    wait_grounded(cx).await;
}

/// `0x14B8`: the flame: turning to the target; the child (`sys 94(actor, 1, target, 0x15C, 0)`: the host's flame,
/// [`child::FLAME`]); then at most 140 frames while 207 plays (the head look toward a target 150 above: not ported);
/// to 207's end; the idle (blend 8).
async fn flame(cx: &Cx) {
    let mut t = k(cx, "flame_time");
    face_target(cx);
    push(cx, ECmd::Throw(child::FLAME));
    while t - 0.0 > 0.0 {
        if playing(cx) != Some(req::BREATH) {
            break;
        }
        t -= step(cx);
        frame().await;
    }
    if playing(cx) == Some(req::BREATH) {
        wait_done(cx).await;
    }
    request(cx, req::IDLE, k(cx, "idle_blend"), 0.0);
}

/// `0x1308`: the breath: the look off, turning to the target; 205 (blend 8); 68 frames; the front row removed
/// (`0x13DC`); 90 frames; 207 (blend 8); 8 frames; the head look (not ported); 60 frames; the flame; the front row
/// back (`0x1444`).
async fn breath(cx: &Cx) {
    face_target(cx);
    request(cx, req::WINDUP, k(cx, "windup_blend"), 0.0);
    wait_frames(cx, k(cx, "windup_open")).await;
    front_off(cx);
    wait_frames(cx, k(cx, "windup_hold")).await;
    request(cx, req::BREATH, k(cx, "breath_blend"), 0.0);
    wait_frames(cx, k(cx, "breath_wait")).await;
    wait_frames(cx, k(cx, "breath_aim_wait")).await;
    flame(cx).await;
    let mut io = cx.borrow_mut();
    front_back(&mut io, false);
}

/// `0x16FC`: one fireball: the head looks at the target (the aim, [`fire_dir`]); 201 (blend 8, from 0: again when it
/// plays); 64 frames turning to the target; the child (`sys 94(actor, 0, target, 0x1CC, 0x2AC)`: the host's fireball,
/// [`child::FIREBALL`]); on the ground.
async fn fireball(cx: &Cx) {
    request(cx, req::FIRE, k(cx, "fire_blend"), k(cx, "fire_from"));
    wait_turning(cx, k(cx, "fire_wait")).await;
    push(cx, ECmd::Throw(child::FIREBALL));
    wait_grounded(cx).await;
}

/// `0x1610`: the volley: the leash (not ported); turning to the target; one fireball; 6 frames; twice while 201
/// plays: 201 again from 12 (blend 4), 48 frames turning, the fireball (the row `0x23C`), 6 frames, a yield; to 201's
/// end (or 6); the look off; the idle (blend 8); 60 frames.
async fn volley(cx: &Cx) {
    let mut n = k(cx, "volley_count") as i32;
    face_target(cx);
    fireball(cx).await;
    wait_frames(cx, k(cx, "volley_gap")).await;
    while n > 0 && playing(cx) == Some(req::FIRE) {
        request(cx, req::FIRE, k(cx, "volley_blend"), k(cx, "volley_from"));
        wait_turning(cx, k(cx, "volley_wait")).await;
        push(cx, ECmd::Throw(child::FIREBALL));
        wait_frames(cx, k(cx, "volley_gap")).await;
        n -= 1;
        frame().await;
    }
    wait_done(cx).await;
    request(cx, req::IDLE, k(cx, "idle_blend"), 0.0);
    wait_frames(cx, k(cx, "volley_rest")).await;
}

/// `0x129A`: two fireballs and a flame: the leash (not ported); two fireballs; 18 frames; 207 from its frame 40
/// (blend 18); 38 frames turning; the flame; on the ground.
async fn fire_and_flame(cx: &Cx) {
    fireball(cx).await;
    fireball(cx).await;
    wait_frames(cx, k(cx, "combo_gap")).await;
    request(cx, req::BREATH, k(cx, "combo_blend"), k(cx, "combo_from"));
    wait_turning(cx, k(cx, "combo_wait")).await;
    flame(cx).await;
    wait_grounded(cx).await;
}

/// `0x12DC`: the quick flame: the leash (not ported); 207 from its frame 40 (blend 18); 18 frames turning; the flame.
async fn quick_flame(cx: &Cx) {
    request(cx, req::BREATH, k(cx, "quick_blend"), k(cx, "quick_from"));
    wait_turning(cx, k(cx, "quick_wait")).await;
    flame(cx).await;
}

/// `0xC5C`: close: 70 % the belly, else the breath; then 40 % two fireballs and a flame.
async fn close(cx: &Cx) {
    if rand(cx, "close") - k(cx, "close_belly_chance") < 0.0 {
        belly(cx).await;
    } else {
        breath(cx).await;
    }
    if rand(cx, "close follow-up") - k(cx, "close_fire_chance") < 0.0 {
        fire_and_flame(cx).await;
    }
}

/// `0xC08`: mid: 30 % the volley then the approach (180 frames, to 300); else the approach, the breath and the belly.
async fn mid(cx: &Cx) {
    let (t, d) = (k(cx, "mid_approach_time"), k(cx, "mid_approach_dist"));
    if rand(cx, "mid") - k(cx, "mid_volley_chance") < 0.0 {
        volley(cx).await;
        approach(cx, t, d).await;
    } else {
        approach(cx, t, d).await;
        breath(cx).await;
        belly(cx).await;
    }
}

/// `0xBC8`: far: 80 % the approach (180 frames, to 600) and the breath; else the volley.
async fn far(cx: &Cx) {
    if rand(cx, "far") - k(cx, "far_breath_chance") < 0.0 {
        let (t, d) = (k(cx, "far_approach_time"), k(cx, "far_approach_dist"));
        approach(cx, t, d).await;
        breath(cx).await;
    } else {
        volley(cx).await;
    }
}

/// The appearance (`0x1132` → `0x20F8`, spawned by the room): home and facing kept; facing Sora now; 69 (blend 0, a
/// plain request); the fade-in (30 frames); KH1's flag 0x80 for 30 frames (not ported) and its effect 2 (M13); 30
/// frames; then actor flag 4 with the idle queued (bit 30): the whole script waits until 69 ends; the idle state.
async fn appear(cx: Cx) {
    if cx.borrow().x.spawned {
        face_sora_now(&cx);
        request(&cx, req::APPEAR, k(&cx, "appear_blend"), 0.0);
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

/// `0xEDA`: the wander: two points about where it stands ([200, 400) and [400, 500)), each walked to (walk 1 at 2 a
/// frame, the way taken every 4 frames, 300 frames, within 60); 70 %: done (with 40 % of all first an idle (blend 8)
/// of [120, 180) frames); else (Agrabah's home return when above its home: never on a battle floor) a yield and
/// again.
async fn wander(cx: &Cx) {
    loop {
        let me = pos(cx);
        let a = me + random_offset(cx, k(cx, "wander_inner_min"), k(cx, "wander_inner_max"));
        let b = me + random_offset(cx, k(cx, "wander_outer_min"), k(cx, "wander_outer_max"));
        let (speed, time, reaim, stop) = (k(cx, "wander_speed"), k(cx, "wander_time"), k(cx, "wander_reaim"), k(cx, "wander_stop"));
        route(cx, Goal::At(a), req::WALK, speed, time, reaim, stop).await;
        route(cx, Goal::At(b), req::WALK, speed, time, reaim, stop).await;
        let r = rand(cx, "wander");
        if r - k(cx, "wander_done_chance") < 0.0 {
            if r - k(cx, "wander_idle_chance") < 0.0 {
                play(cx, req::IDLE, k(cx, "idle_blend"));
                let lo = k(cx, "wander_idle_min");
                let n = crate::enemy::script::rand_range(cx, lo, k(cx, "wander_idle_max"), "wander idle time");
                wait_frames(cx, n).await;
            }
            return;
        }
        frame().await;
    }
}

/// The idle behaviour (`0xCDE`): Sora's target is it → engaged with him; the wander; again.
async fn idle(cx: Cx) {
    loop {
        if cx.borrow().x.targeted {
            set_target(&cx, true);
            set_state(&cx, st::ENGAGED);
            halt().await;
        }
        wander(&cx).await;
        frame().await;
    }
}

/// The engaged behaviour (`0xA56`): the target dead → lost; the leash (not ported); the idle (blend 8); close (< 300),
/// mid (< 600) or far; the leash; the engaged search (Sora) and engaged again.
async fn engaged(cx: Cx) {
    if target(&cx).is_none() {
        set_state(&cx, st::LOST);
        halt().await;
    }
    play(&cx, req::IDLE, k(&cx, "idle_blend"));
    let d = crate::enemy::script::dist_to_target(&cx);
    if d - k(&cx, "close_range") < 0.0 {
        close(&cx).await;
    } else if d - k(&cx, "mid_range") < 0.0 {
        mid(&cx).await;
    } else {
        far(&cx).await;
    }
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// The lost behaviour (`0x83E`): Sora alive (the last target `wp+0x4`, or the search within 10000) → engaged; else
/// the idle motion (spawn type 0's; a spawned one vanishes by distance: the leash, not ported), again.
async fn lost(cx: Cx) {
    loop {
        // `0x8F8`: the last target (`wp+0x4`: Sora) alive → engaged with it; else the search (10000).
        let found = {
            let io = cx.borrow();
            io.w.target.is_some()
        };
        if found {
            set_target(&cx, true);
            set_state(&cx, st::ENGAGED);
            halt().await;
        }
        play(&cx, req::IDLE, k(&cx, "idle_blend"));
        frame().await;
    }
}

/// The hit behaviour (`0x4F2`): the event's flame (`wp+0x78`, never here); the target: Sora the first time, then
/// 25 % back to Sora (one draw; always Sora here); the target dead → lost. Staggered (`wp+0x7C`): 208 (blend 0), the
/// EXP bonus max(EXP / 5, 1) (`sys 195`, shown 140 above it), the drop table `0x464` only after an attribute-8 hit
/// (never here), to 208's end. Else at most 2 counted hits: 80 % the quick flame, else two fireballs and a flame;
/// more: the count back to 0 and the volley. Then engaged.
async fn hit(cx: Cx) {
    let picked = std::mem::replace(&mut cx.borrow_mut().x.picked, true);
    if picked {
        let _ = rand(&cx, "hit target") - k(&cx, "hit_sora_chance") < 0.0;
    }
    if target(&cx).is_none() {
        set_state(&cx, st::LOST);
        halt().await;
    }
    let stagger = std::mem::take(&mut cx.borrow_mut().x.stagger);
    if stagger {
        request(&cx, req::STAGGER, 0.0, 0.0);
        let r = Reward { exp_div: k(&cx, "stagger_exp_div") as i32, exp_add: 0, exp_min: k(&cx, "stagger_exp_min") as i32, prizes: [0; 7] };
        push(&cx, ECmd::Reward(r));
        wait_done_after(&cx).await;
    } else if (cx.borrow().x.hits as f32) - k(&cx, "hit_count_max") <= 0.0 {
        if rand(&cx, "counter") - k(&cx, "hit_flame_chance") < 0.0 {
            quick_flame(&cx).await;
        } else {
            fire_and_flame(&cx).await;
        }
    } else {
        cx.borrow_mut().x.hits = 0;
        volley(&cx).await;
    }
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// The supervisor (`0x24E8`): after the init frame, each frame (nothing while the appearance's flag 4 holds the
/// script): while idle, the search (a room-spawned one: Sora within 1500) → engaged with the target (`wp+0x4`
/// picked); then a state request restarts the behaviour thread.
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
                cx.borrow_mut().x.picked = true;
                set_target(&cx, true);
                set_state(&cx, st::ENGAGED);
            }
        }
        if let Some(s) = take_state(&cx) {
            set_target(&cx, s != st::IDLE);
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Fat Bandit's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as the Shadow's: per-enemy seeds). `frames`: KH1 motion
    /// lengths by request. `spawned`: the room spawned it (its appearance plays; every Re:CoM battle appearance, owner
    /// 2026-11-02).
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, spawned: bool) -> Ai {
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp { spawned, ..Default::default() });
        Ai { io: Rc::new(RefCell::new(io)), main: None, behaviour: None, started: false }
    }

    pub fn consts(&self) -> Consts {
        self.io.borrow().c.clone()
    }

    /// One AI frame (the host calls it every frame: no action of its pauses it).
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
                // `0x27A0`: home = the position; the state object cleared; `0x254E`: the front row (mode 3).
                io.home = w.pos;
                io.state = (st::NONE, false);
                front_on(&mut io, false);
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
                st::IDLE => Box::pin(idle(cx)),
                st::ENGAGED => Box::pin(engaged(cx)),
                st::LOST => Box::pin(lost(cx)),
                _ => Box::pin(hit(cx)),
            });
        }
        poll(&mut self.behaviour);
        std::mem::take(&mut self.io.borrow_mut().out)
    }

    /// `sys 42` (`0x1402B7E60`): the flinch's push slide (`G+0x2C` × the stats block's `+0xC4`, the knock-back weight;
    /// `crate::enemy::react`'s).
    fn slide(io: &mut crate::enemy::script::Core<Consts, Attack, Wp>, dir: Vec3) {
        use crate::enemy::Engine;
        let p = io.c.flinch_push() * io.c.weight();
        io.body.push(dir, p);
    }

    /// The handler's damaging path for a living Fat Bandit (`0x2B12..0x30C0`, attribute 0): see the module's notes.
    fn damage(&self, dir: Vec3) {
        let mut io = self.io.borrow_mut();
        let deg = io.c.num("turn_deg");
        io.body.turn = Some(deg / 180.0 * std::f32::consts::PI);
        let mut kind = if io.x.hit_kind == 5 { 4 } else { io.x.hit_kind };
        // `0x2D3E`: the stagger on its front while its front row is down (`sys 100` of the facing with the way to the
        // attacker, `−dir` flat: Assumption, KH1's is 3D), for a hit that isn't a flinch.
        let face = Vec3::from_yaw(io.body.yaw);
        let front = io.x.open && face.dot(-dir.flat().normalized()) - io.c.num("stagger_front_deg").to_radians().cos() > 0.0;
        if front && kind != 3 {
            front_back(&mut io, true);
            io.x.stagger = true;
            Self::slide(&mut io, dir);
            io.state = (st::HIT, true);
            return;
        }
        // `0x2EC6`: its front row down: made again, the kind 4.
        if io.x.open {
            front_back(&mut io, true);
            kind = 4;
        }
        // `0x2F1A`: the count.
        match io.x.hit_count {
            1 => {
                if (io.x.hits as f32) - io.c.num("count_reset") > 0.0 {
                    io.x.hits = 0;
                }
                let add = if kind == 3 { io.c.num("count_flinch") } else { io.c.num("count_other") };
                io.x.hits += add as i32;
            }
            2 => {
                let add = io.c.num("count_second");
                io.x.hits += add as i32;
            }
            _ => {}
        }
        // `0x308E`: the kind zeroed and `sys 42`: no reaction, no state.
        Self::slide(&mut io, dir);
    }
}

/// KH1's fireball direction (`ex_2060_00.bd` `0x644`: `sys 143` → `0x1402BAE10`: with the head look on, `+0x450`, the
/// head's direction; `0x16FC` turns the look to the target before 201): the way from the head's point (`0x1402A2980`:
/// bone code 0 at t 0.5, `eye`) to the target's lock point 0, in the facing's frame: inside the cone (`cone_deg`) its
/// pitch clamped to `up_deg` above / `down_deg` below (`+0x2C` / `+0x30`) and its turn to `left_deg` / `right_deg`
/// (`+0x34` / `+0x38`); outside it (or no target) straight ahead. KH1 units, y down. Checked (decomp `0x1402A2510`,
/// `0x1402A2980`). Assumption: the look has reached it (KH1 turns it 2° a frame, stats `+0x24`; 64 frames of 201
/// turning to the target come first) and `+0x474`'s wider cone (once looking) is the one in force.
pub fn fire_dir(yaw: f32, eye: Vec3, target: Option<Vec3>, cone_deg: f32, up_deg: f32, down_deg: f32, left_deg: f32, right_deg: f32) -> Vec3 {
    let fwd = Vec3::from_yaw(yaw);
    let Some(t) = target else { return fwd };
    let (d, len) = (t - eye).normalize_len();
    if len <= 0.0 {
        return fwd;
    }
    // The facing's frame: forward, side (yaw + 90°), down.
    let side = Vec3::from_yaw(yaw + std::f32::consts::FRAC_PI_2);
    let (f, s, v) = (d.dot(fwd), d.dot(side), d.y);
    if f - cone_deg.to_radians().cos() < 0.0 {
        return fwd;
    }
    let pitch = v.clamp(-1.0, 1.0).asin().clamp(-up_deg.to_radians(), down_deg.to_radians());
    let turn = s.atan2(f).clamp(-left_deg.to_radians(), right_deg.to_radians());
    let (cp, sp) = (pitch.cos(), pitch.sin());
    fwd * (cp * turn.cos()) + side * (cp * turn.sin()) + Vec3::new(0.0, sp, 0.0)
}

/// KH1's flame (`ex_2060_01.bd`; its attack object of shape 2, `0x1402B3800` → `0x1402B3D50`): the spheres along the
/// line from `a` (radius `r0`) to `b` (radius `r1`), its first `grow` frames scaled by `t / grow_div` about `a` (the
/// line's end and both radii: `0x39C..0x400`); spheres every (r0 + r1) / 2 along it from `a`, the radius stepping by
/// (r1 − r0) × spacing / length, one at least, while length is left. KH1 units. Checked (files + decomp).
pub fn flame_spheres(a: Vec3, b: Vec3, r0: f32, r1: f32, t: f32, grow: f32, grow_div: f32) -> Vec<(Vec3, f32)> {
    let (mut b, mut r0, mut r1) = (b, r0, r1);
    if t - grow < 0.0 {
        let f = t / grow_div;
        b = a + (b - a) * f;
        r0 *= f;
        r1 *= f;
    }
    let spacing = (r0 + r1) * 0.5;
    if spacing == 0.0 {
        return Vec::new();
    }
    let (dir, mut left) = (b - a).normalize_len();
    let dr = if left == 0.0 { 0.0 } else { (r1 - r0) * spacing / left };
    let mut out = Vec::new();
    let (mut p, mut r) = (a, r0);
    loop {
        out.push((p, r));
        p += dir * spacing;
        left -= spacing;
        r += dr;
        if left - 0.0 <= 0.0 || out.len() >= 64 {
            break;
        }
    }
    out
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
    /// The hit handler (`0x29BC`) for a clash, a repel, or a damaging hit that took Re:CoM's reaction path (the
    /// armour below takes the rest): a clash or repel (an attacker: Sora) → the hit state; a damaging hit → the
    /// damaging path (KH1 takes no reaction for it: [`Behaviour::keeps_reaction`] false).
    fn on_hit(&self, news: HitNews) {
        let kind = self.io.borrow().x.hit_kind;
        if news != HitNews::Damage || matches!(kind, 6 | 7) {
            let mut io = self.io.borrow_mut();
            let deg = io.c.num("turn_deg");
            io.body.turn = Some(deg / 180.0 * std::f32::consts::PI);
            io.state = (st::HIT, true);
            return;
        }
        let dir = self.io.borrow().x.hit_dir;
        self.damage(dir);
    }
    /// Every damaging hit that leaves it alive keeps Re:CoM's state (its super armour): KH1's handler ends each in
    /// `sys 42` (the stagger with its hit state too).
    fn armoured(&self, _playing: Option<u16>, _launch_base: u16, _kind: Option<u8>) -> bool {
        !self.io.borrow().x.dead
    }
    fn on_armoured_hit(&self, dir: Vec3) {
        self.damage(dir);
    }
    fn keeps_reaction(&self) -> bool {
        false
    }
    /// The handler's kind 5 → 4 (`0x2B4C`): the killing hit's reaction (KH1 plays it after its death routine's 208
    /// request, Assumption: the reaction's request replaces it).
    fn reaction_kind(&self, kind: u8) -> u8 {
        if kind == 5 {
            4
        } else {
            kind
        }
    }
    fn on_defeat(&self) {
        let mut io = self.io.borrow_mut();
        io.x.dead = true;
        io.x.stagger = false;
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
    fn set_hit_count(&self, n: u16) {
        self.io.borrow_mut().x.hit_count = n;
    }
    fn set_playing(&self, request: Option<u16>) {
        self.io.borrow_mut().x.cur = request;
    }
    fn set_targeted(&self, by_sora: bool) {
        self.io.borrow_mut().x.targeted = by_sora;
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["none", "idle", "engaged", "lost", "hit"].get(s as usize).copied().unwrap_or("?")
    }
    fn reset(&self) {
        set_state(&self.io, st::ENGAGED);
    }
}

#[cfg(test)]
mod tests;
