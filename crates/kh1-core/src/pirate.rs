//! KH1 FM's Pirate (`xa_ex_2100.mdls` part 5, `ex_2100.bd`), engine-agnostic: its AI script on the
//! shared enemy framework (`crate::enemy`). M11 (`docs/recom-m11-enemies.md` § The Pirate). Re:CoM runs
//! it natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), Re:CoM
//! plays the motions, collides the body and the attacks, applies the damage.
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2100.bd` at checked
//! offsets); nothing of the game is in this file. Script offsets are the player's `ex_2100.bd`'s
//! (Checked (files): the script disassembled with `tools/kh1-ai-script.py`, code from `0x5A8`; the VM's
//! ops and the syscalls named, Checked (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x2824`: init `0x2960` (stats records `0x14` / `0x80`, walk 3 / run 6, home, the state
//!   object `wp+0x20`, the search object `wp+0x30`, the handlers: hit `0x2A3C`), a yield, the appearance
//!   thread `0x992`; then each frame: while idle (state 1) the search `0x182E` (Sora within 1000, in
//!   KH1's world 11 always) whose height difference is within ±100 → engaged, else no target; a state
//!   request restarts the behaviour thread: 1 idle `0x946`, 2 engaged `0x6B4`, 3 hit `0x5CC`.
//! - the hit handler `0x2A3C`: a clash (kinds 6 / 7) turns to the attacker, marks the hit
//!   (`wp+0x54` = 5) and asks for the hit state; a damaging hit while one of its attacks plays (216, 214,
//!   235, 236, 218, 210) or while it lies after a launch (`sys 70`) takes no reaction (`sys 42`: the push
//!   slide only); any other damaging hit of kind 4 / 5 replaces KH1's reaction with its own launch
//!   (`sys 38`: bases 240 / 250, hop 100, speed 30, no down time) and remembers the side it came from
//!   (`wp+0x48`, the get-up slash 235 / 236); any other damaging hit marks the hit and asks for the hit
//!   state.
//! - the hops `0x157C` / `0x20AE`: a crouch for its length, then movement state 3 (`sys 82`, a jump of
//!   fixed time; `crate::enemy::Body::timed_jump`) with its motion trio (`sys 83`).
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0x118C` (`0x13CE(20000, 20000)`: KH1
//! removes it 200 m from Sora, never in world 11), the death routine `0x19CE` (Re:CoM's defeat, prizes,
//! EXP), KH1's effects (`sys 75` / `sys 97`: the slash's 0x810 / 0x811, the spin's 14, the appearance's 2;
//! M13), the appearance's actor flag 0x80 for 30 frames (`0x24FA`), the hit types 8..11 told apart
//! (`0x1F40` / `0x196C`: every hit of Sora's is taken as type 0, as the Bandit's), the search's sight cone
//! (`sys 48` with `0x580`; its fallback finds Sora within the range anyway) and `sys 139`'s out-of-sight
//! test (taken as in sight).

use crate::enemy::react::ScriptedLaunch;
use crate::enemy::script::{approach, dist_to_target, face_target, frame, motion_done, motion_frames, move_toward, poll, pos, push, rand, random_offset, set_state, set_target, take_state, target, tick_clock, wait_frames, wait_grounded, wander};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, Engine, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Pirate's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The air motion at a timed jump's top (`sys 83(3)`: base 0, `0x1402AF630`).
    pub const APEX: u16 = 5;
    /// The appearance (`0x1B84`: `0x1800045`, bits 24 and 23), 32 frames, root motion.
    pub const APPEAR: u16 = 69;
    /// The leap (`0xB54` → `0x20AE(…, 200)`): the crouch 200, 201 rising (bit 16), 202 at the top, 203
    /// landing.
    pub const LEAP: u16 = 200;
    /// The blade stuck in the floor after the slam (`0xC60`: `0x10000CD`, an action), 206 queued, the
    /// pull 207 (an action).
    pub const STUCK: u16 = 205;
    pub const STUCK_HOLD: u16 = 206;
    pub const PULL: u16 = 207;
    /// The block stance (`0xA8C`), 48 frames (looped), its guard row on throughout; its recoil 209.
    pub const STANCE: u16 = 208;
    pub const STANCE_RECOIL: u16 = 209;
    /// The jump slash (`0xDEA`: `0x10000D2` from frame 8), 66 frames, hit [28, 34); its recoil 211.
    pub const JUMP_SLASH: u16 = 210;
    pub const JUMP_SLASH_RECOIL: u16 = 211;
    /// The laugh (`0x11CE`), 166 frames.
    pub const LAUGH: u16 = 212;
    /// The recovery after the slam (`0xC30`, an action), 46 frames.
    pub const RECOVER: u16 = 213;
    /// The slash (`0xE9E`), 178 frames, root motion, its row's hit [122, 134).
    pub const SLASH: u16 = 214;
    /// The recoil of the slash, the counter, the slam and the get-up slashes, 164 frames.
    pub const RECOIL: u16 = 215;
    /// The counter slash (`0x9DE`, an action), 110 frames, hit [32, 35).
    pub const COUNTER: u16 = 216;
    /// The spin (`0xF10`, looped), 48 frames, hits [12, 18) and [36, 42); its guard row on throughout.
    pub const SPIN: u16 = 217;
    /// The slam on landing from the leap (`0xBCE`), 52 frames, hit [0, 5).
    pub const SLAM: u16 = 218;
    /// After a spin that hit (`0x10E2`, an action), 152 frames.
    pub const DIZZY: u16 = 219;
    /// The taunt (`0x11A4`, an action), 150 frames.
    pub const TAUNT: u16 = 220;
    /// The spin's recoil, 96 frames.
    pub const SPIN_RECOIL: u16 = 221;
    /// The spin's wind-up (`0xEEC`: from frame 8), 130 frames.
    pub const WINDUP: u16 = 222;
    /// The get-up slashes after its own launch (`0xAD6`, actions): from the front 235 (88 frames, hit
    /// [44, 62)), from behind 236 (64 frames, hit [20, 30)).
    pub const GETUP_FRONT: u16 = 235;
    pub const GETUP_BACK: u16 = 236;
    /// Its own launch's bases (`0x25CA` → `sys 38`; ground / air), KH1's launch layout.
    pub const OWN_LAUNCH: u16 = 240;
    pub const OWN_AIR_LAUNCH: u16 = 250;
}

/// The motions a damaging hit doesn't interrupt (`0x267E`: `sys 51` against these; then `sys 42`).
pub const ARMOUR: [u16; 6] = [req::COUNTER, req::SLASH, req::GETUP_FRONT, req::GETUP_BACK, req::SLAM, req::JUMP_SLASH];

/// The Pirate's attacks, in the order the host indexes them (the manifest's `attacks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    Slash,
    Counter,
    JumpSlash,
    Slam,
    Spin,
    GetUpFront,
    GetUpBack,
}

impl Attack {
    pub const ALL: [Attack; 7] = [Attack::Slash, Attack::Counter, Attack::JumpSlash, Attack::Slam, Attack::Spin, Attack::GetUpFront, Attack::GetUpBack];

    pub fn request(self) -> u16 {
        match self {
            Attack::Slash => req::SLASH,
            Attack::Counter => req::COUNTER,
            Attack::JumpSlash => req::JUMP_SLASH,
            Attack::Slam => req::SLAM,
            Attack::Spin => req::SPIN,
            Attack::GetUpFront => req::GETUP_FRONT,
            Attack::GetUpBack => req::GETUP_BACK,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Slash => "slash",
            Attack::Counter => "counter",
            Attack::JumpSlash => "jump slash",
            Attack::Slam => "slam",
            Attack::Spin => "spin",
            Attack::GetUpFront => "getup front",
            Attack::GetUpBack => "getup back",
        }
    }

    /// The damaging attack rows (script offsets, 0x70 bytes, `sys 23`) made with it. The guard-type
    /// rows (`0x1CC` with the spin, `0x46C` with the stance) are [`Guard`]s.
    pub fn rows(self) -> &'static [usize] {
        match self {
            Attack::Slash => &[0x23C],
            Attack::Counter => &[0x4DC],
            Attack::JumpSlash => &[0x31C],
            Attack::Slam => &[0x38C],
            Attack::Spin => &[0xEC, 0x15C],
            Attack::GetUpFront | Attack::GetUpBack => &[0x3FC],
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The Pirate's guard-type rows (attack type 2: they meet only swings and type-3 attacks, never a hurt
/// body; `crate::enemy::guard`), in the order of the manifest's `guards`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Guard {
    /// The spin's body row `0x1CC` (bone code 8, 55, priority 90, recoil 221; HIT slot 2, on through 217).
    Spin,
    /// The stance's blade row `0x46C` (bone code 48, 25, priority 30, recoil 209; HIT slot 0, on through
    /// 208).
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
            Guard::Spin => 0x1CC,
            Guard::Stance => 0x46C,
        }
    }

    /// The requests the script makes it with (`0x1156` before `sys 12(217)`; `0xAB4` after
    /// `sys 12(208)`).
    pub fn requests(self) -> &'static [u16] {
        match self {
            Guard::Spin => &[req::SPIN],
            Guard::Stance => &[req::STANCE],
        }
    }

    pub fn index(self) -> u8 {
        Guard::ALL.iter().position(|&g| g == self).unwrap_or(0) as u8
    }
}

/// The names of the Pirate's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    "idle_search",
    "engage_dy_max",
    "engage_dy_min",
    "world11_search",
    "close_range",
    "mid_range",
    "above_near",
    "above_far",
    "above_high",
    "close_slash_chance",
    "close_jump_chance",
    "mid_hop_chance",
    "mid_slash_chance",
    "far_leap_chance",
    "far_taunt_range",
    "far_taunt_chance",
    "far_laugh_chance",
    "wander_inner",
    "wander_outer",
    "wander_band",
    "wander_timeout",
    "wander_idle_chance",
    "wander_idle_min",
    "wander_idle_max",
    "wander_home_max",
    "idle_blend",
    "idle_taunt_chance",
    "hit_range",
    "hit_stance_max",
    "hit_stance_chance",
    "counter_blend",
    "stance_time",
    "stance_end_blend",
    "stance_blend",
    "getup_blend",
    "leap_speed",
    "leap_height",
    "leap_drag",
    "leap_time",
    "slam_blend",
    "recover_chance",
    "recover_blend",
    "stuck_blend",
    "pull_blend",
    "jslash_speed",
    "jslash_height",
    "jslash_drag",
    "jslash_time",
    "jslash_blend",
    "slash_blend",
    "slash_end_blend",
    "spin_after_hit",
    "windup_blend",
    "spin_blend",
    "spin_time",
    "spin_speed",
    "spin_near",
    "spin_away_min",
    "spin_away_max",
    "dizzy_blend",
    "taunt_blend",
    "laugh_blend",
    "laugh_end_blend",
    "hop_blend",
    "fwd_hop_angle",
    "fwd_hop_speed",
    "fwd_hop_height",
    "fwd_hop_time",
    "side_hop_angle",
    "side_hop_chance",
    "side_hop_speed",
    "side_hop_height",
    "side_hop_time",
    "back_hop_angle",
    "back_hop_speed",
    "back_hop_height",
    "back_hop_time",
    "approach_timeout",
    "approach_retarget",
    "approach_dist",
    "hop_drag",
    "appear_fade",
    "appear_blend",
    "own_launch_hop",
    "own_launch_speed",
    "own_launch_down",
];

/// The Pirate's numbers.
pub type Consts = Table;

/// The Pirate's state objects (`wp+0x20`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 3;
}

/// KH1's world 11 (`sys 105`'s id; Olympus Coliseum): the search reaches 100 m and always finds Sora
/// (`0x182E`), and the leash never removes it (not ported anyway).
pub const OLYMPUS: u8 = 11;

/// The spin's run (`wp+0x4C`): 8 spinning, 9 one of its rows landed (the rows' callback `0x117C`), 11
/// ended (10 is never written: the loop's own early end is unreachable).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SpinRun {
    Spinning,
    Landed,
    #[default]
    Over,
}

/// The Pirate's own work area: how many stances a counter has raised (`wp+0x40`), the side of its own
/// launch (`wp+0x48`: 0 from the front, 1 from behind, 2 none), whether a hit came since the stance
/// was raised (`wp+0x54`: 5, else 7), the spin's run (`wp+0x4C`), the slam's end (`wp+0x50`: 12 the
/// recovery, 13 the stuck blade); spawned by the room (`sys 132`), the appearance done, KH1's world 11,
/// Sora on the ground (`*(target + 0x70)`), the last hit's kind and push direction, the launch it asked for.
#[derive(Clone, Copy, Debug)]
struct Wp {
    stances: i32,
    side: u8,
    hit: bool,
    spin: SpinRun,
    stuck: bool,
    spawned: bool,
    appeared: bool,
    olympus: bool,
    sora_grounded: bool,
    hit_kind: u8,
    hit_dir: Vec3,
    launch: Option<ScriptedLaunch>,
}

impl Default for Wp {
    /// `0x2960`: `wp+0x40` = 0, `+0x54` = 7, `+0x4C` = 11, `+0x50` = 12; `+0x48` starts 0 (only the hit
    /// handler writes it, and always 2 before anything reads it).
    fn default() -> Wp {
        Wp { stances: 0, side: 2, hit: false, spin: SpinRun::Over, stuck: false, spawned: false, appeared: false, olympus: false, sora_grounded: true, hit_kind: 0, hit_dir: Vec3::ZERO, launch: None }
    }
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

fn grounded(cx: &Cx) -> bool {
    cx.borrow().w.grounded
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

/// `sys 13` of an attack's request (its rows made just before): the motion plays from its start, the
/// script keeps running.
fn attack_now(cx: &Cx, a: Attack, blend: f32) {
    cx.borrow_mut().clock = (Some(a.request()), 0.0);
    push(cx, ECmd::Attack { request: a.request(), attack: a, blend });
}

/// `sys 12` of an attack's request: nothing when it already plays (`0x14029F7C0` returns for the entry
/// playing; the spin's loop asks for 217 every frame). The entry playing is the last one this frame asked
/// for, else the host's.
fn attack_keep(cx: &Cx, a: Attack, blend: f32) {
    let playing = {
        let io = cx.borrow();
        io.out.iter().rev().find_map(|c| match c {
            ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Attack { request, .. } | ECmd::Action { request, .. } => Some(*request),
            _ => None,
        }).or(io.w.motion)
    };
    if playing != Some(a.request()) {
        attack_now(cx, a, blend);
    }
}

/// `0x1C9E(r, blend, queue blend)`: an action (bit 24) with the idle queued, then the yield (the action
/// pauses the script until it ends). The queue's own blend is the host's (Assumption, as every port).
async fn action(cx: &Cx, r: u16, attack: Option<Attack>, blend: f32) {
    push(cx, ECmd::Action { request: r, attack, then: req::IDLE, blend });
    frame().await;
}

/// `sys 23` of a guard-type row with the request it lives on.
fn guard(cx: &Cx, g: Guard, request: u16) {
    push(cx, ECmd::Guard(Some((g.index(), request))));
}

/// `sys 51 == 6`: the air motion a reaction or a jump queues plays.
fn in_air_motion(cx: &Cx) -> bool {
    cx.borrow().w.motion == Some(req::AIR)
}

/// Requests made with bit 16 (`(r + 1) | 0x10000` of the hops, `0x15F0` / `0x20F6`, and the jump slash's
/// `0x10004`): KH1 holds their last pose (`0x14029FC40` copies bit 16 to `+0x160` bit 3, `0x14029F0A0`
/// then holds; Checked (decomp)).
fn holds(r: u16) -> bool {
    r == req::FALL || r == req::LEAP + 1
}

/// `sys 172` (`0x14029EFC0`): the motion has played to its end this cycle (KH1 loops a plain request at
/// its end unless it holds: each cycle's end; the Bandit's rule).
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

/// `0x2320`: a yield, then until the motion has played to its end (`sys 172`).
async fn wait_done(cx: &Cx) {
    loop {
        frame().await;
        if done_now(cx) {
            break;
        }
    }
}

/// `0x1BFA`: until the motion is done or the air motion plays, turning to the target (`sys 36`).
async fn wait_done_or_air(cx: &Cx) {
    while !done_now(cx) {
        if in_air_motion(cx) {
            break;
        }
        face_target(cx);
        frame().await;
    }
}

/// `0x1C2E`: `0x1BFA` without the turn.
async fn wait_done_or_air_still(cx: &Cx) {
    while !done_now(cx) {
        if in_air_motion(cx) {
            break;
        }
        frame().await;
    }
}

/// The rest of the behaviour thread after a state request (`misc8`): the supervisor replaces it at its
/// next run.
async fn halt() {
    loop {
        frame().await;
    }
}

/// `sys 11` toward a point (`0x2368`): the facing at once.
fn face_now_to(cx: &Cx, t: Vec3) {
    let d = t - pos(cx);
    let y = d.x.atan2(d.z);
    let mut io = cx.borrow_mut();
    io.body.yaw = y;
    io.body.yaw_target = y;
    io.out.push(ECmd::Yaw(y));
}

/// `0x2368(actor, target)`: face the target now (none: nothing; KH1's `sys 66` debug print).
fn face_target_now(cx: &Cx) {
    if let Some(t) = target(cx) {
        face_now_to(cx, t);
    }
}

/// `0x2368(actor, party 0)`: face Sora now.
fn face_sora_now(cx: &Cx) {
    let t = cx.borrow().w.target;
    if let Some(t) = t {
        face_now_to(cx, t);
    }
}

/// `0x233E(actor, attacker)`: turn toward Sora (`sys 36`: the target yaw only).
fn face_sora(cx: &Cx) {
    let t = cx.borrow().w.target;
    if let Some(t) = t {
        let d = t - pos(cx);
        cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
    }
}


/// `sys 100` (`0x1402A8C70`): the cosine of the angle between two vectors (3D, clamped, 0 when either is
/// zero). Checked (decomp).
fn cos_between(a: Vec3, b: Vec3) -> f32 {
    let (la, lb) = (a.length(), b.length());
    if la == 0.0 || lb == 0.0 {
        return 0.0;
    }
    (a.dot(b) / (la * lb)).clamp(-1.0, 1.0)
}

/// `0x182E(&wp30, 0, range, 1)`: the target search. In KH1's world 11 the range is `world11_search` and
/// Sora is always found (`0x185C`, `0x18E4`); else Sora within the range (Assumption: KH1's search
/// `sys 48` finds him within its range; `0x182E`'s fallback `0x190A` gives him within it anyway;
/// `sys 139`'s out-of-sight test is taken as in sight, as the Wyvern's).
fn search(cx: &Cx, range: f32) -> Option<Vec3> {
    let io = cx.borrow();
    let t = io.w.target?;
    (io.x.olympus || (t - io.w.pos).length() - range < 0.0).then_some(t)
}

/// `0x157C(angle, speed, height, time, r)` / `0x20AE(actor, v, drag, time, r)`: a hop. The facing turned
/// by `angle` × `speed` with `height` up (`sys 10`, `sys 93`); the crouch r (blend 0) for its length;
/// r + 1 (bit 16) and movement state 3 (`sys 82`: drag, time) with its motion base (`sys 83(r)`: six
/// frames before the top KH1 plays r + 2 and queues r + 3 while r + 1 plays, `0x1402AF630`); while in the
/// air, facing the target now each frame (`0x157C` only: `face`); then (`0x157C`) until on the ground.
#[allow(clippy::too_many_arguments)]
async fn hop(cx: &Cx, angle: f32, speed: f32, height: f32, time: f32, drag: f32, r: u16, face: bool) {
    let yaw = cx.borrow().body.yaw;
    let d = Vec3::from_yaw(yaw + angle.to_radians()) * speed;
    let v = Vec3::new(d.x, height, d.z);
    let b = k(cx, "hop_blend");
    play(cx, r, b);
    wait_frames(cx, motion_frames(cx, r)).await;
    // `(r + 1) | 0x10000`: bit 16 holds its last pose ([`holds`]).
    play(cx, r + 1, b);
    cx.borrow_mut().body.timed_jump(v, drag, time);
    let mut apex = false;
    while cx.borrow().body.timed.is_some() {
        // `0x1402AF630` runs in the engine's step (after the script): here at the next frame's script
        // (Assumption: ±1 frame, as the Powerwild's).
        let (at_top, cur) = {
            let io = cx.borrow();
            (io.body.timed_apex(), io.w.motion)
        };
        if !apex && at_top && cur == Some(r + 1) {
            apex = true;
            play(cx, r + 2, b);
            push(cx, ECmd::Queue { request: r + 3, blend: b });
        }
        if face {
            face_target_now(cx);
        }
        frame().await;
    }
    if face {
        while !grounded(cx) {
            frame().await;
        }
    }
}

/// `0x1200`: on the ground, facing the target now, the forward hop (`0x157C(0, 30, 150, 40, 3)`).
async fn fwd_hop(cx: &Cx) {
    wait_grounded(cx).await;
    face_target_now(cx);
    let (a, s, h, t, d) = (k(cx, "fwd_hop_angle"), k(cx, "fwd_hop_speed"), k(cx, "fwd_hop_height"), k(cx, "fwd_hop_time"), k(cx, "hop_drag"));
    hop(cx, a, s, h, t, d, req::JUMP, true).await;
}

/// `0x1238`: ±15° at random (`-15` below one half), on the ground, a low hop on its current facing
/// (`0x157C(a, 25, 30, 30, 3)`).
async fn side_hop(cx: &Cx) {
    let mut a = k(cx, "side_hop_angle");
    if rand(cx, "side hop") - k(cx, "side_hop_chance") < 0.0 {
        a = -a;
    }
    wait_grounded(cx).await;
    let (s, h, t, d) = (k(cx, "side_hop_speed"), k(cx, "side_hop_height"), k(cx, "side_hop_time"), k(cx, "hop_drag"));
    hop(cx, a, s, h, t, d, req::JUMP, true).await;
}

/// `0x1296`: on the ground, turning to the target (`sys 36`), the back hop on its current facing
/// (`0x157C(180, 20, 100, 30, 3)`); on the ground, the run in (`0x23B0(1, 300, 20, 350)`, the shared
/// approach).
async fn back_hop_in(cx: &Cx) {
    wait_grounded(cx).await;
    face_target(cx);
    let (a, s, h, t, d) = (k(cx, "back_hop_angle"), k(cx, "back_hop_speed"), k(cx, "back_hop_height"), k(cx, "back_hop_time"), k(cx, "hop_drag"));
    hop(cx, a, s, h, t, d, req::JUMP, true).await;
    wait_grounded(cx).await;
    approach(cx, 1, k(cx, "approach_timeout"), k(cx, "approach_retarget"), k(cx, "approach_dist")).await;
}

/// `0xE6C`: the slash. On the ground (its two effects: M13); its row `0x23C` and 214 (blend 0, a plain
/// request: the script keeps turning to the target); until it is done or the air motion plays; the idle
/// (blend 8); until on the ground.
async fn slash(cx: &Cx) {
    wait_grounded(cx).await;
    attack_now(cx, Attack::Slash, k(cx, "slash_blend"));
    wait_done_or_air(cx).await;
    play(cx, req::IDLE, k(cx, "slash_end_blend"));
    wait_grounded(cx).await;
}

/// `0x9C0`: the counter slash. Turning to the target, its row `0x4DC` and action 216 (blend 0).
async fn counter(cx: &Cx) {
    face_target(cx);
    let b = k(cx, "counter_blend");
    action(cx, req::COUNTER, Some(Attack::Counter), b).await;
}

/// `0xAD6`: the get-up slash after its own launch: 235 when it came from the front (`wp+0x48` = 0), 236
/// from behind (1); `wp+0x48` back to 2; turning to the target, its row `0x3FC`, the action (blend 0).
async fn getup_slash(cx: &Cx) {
    let side = cx.borrow().x.side;
    cx.borrow_mut().x.side = 2;
    face_target(cx);
    let b = k(cx, "getup_blend");
    match side {
        0 => action(cx, req::GETUP_FRONT, Some(Attack::GetUpFront), b).await,
        1 => action(cx, req::GETUP_BACK, Some(Attack::GetUpBack), b).await,
        _ => {}
    }
}

/// `0xA8C`: raise the stance: turning to the target, 208 (blend 6), its blade row `0x46C` made mode 2
/// (`sys 213(row, 2)`: it lives while 208 plays; [`Guard::Stance`]), `wp+0x54` = 7.
fn raise_stance(cx: &Cx) {
    face_target(cx);
    play(cx, req::STANCE, k(cx, "stance_blend"));
    guard(cx, Guard::Stance, req::STANCE);
    cx.borrow_mut().x.hit = false;
}

/// `0x9F6`: the block stance for `stance_time` frames turning to the target; a hit seen meanwhile
/// (`wp+0x54` = 5) counts (`wp+0x40`) and raises it again from the start; then the idle at blend 12 and a
/// yield. (KH1's hit handler asks for the hit state at every hit, which restarts the behaviour before
/// this loop sees the mark: the count only grows if a hit came without one; ported as written.)
async fn stance(cx: &Cx) {
    raise_stance(cx);
    let mut t = 0.0;
    while t - k(cx, "stance_time") < 0.0 {
        face_target(cx);
        if cx.borrow().x.hit {
            cx.borrow_mut().x.stances += 1;
            raise_stance(cx);
            t = 0.0;
        }
        t += step(cx);
        frame().await;
    }
    play(cx, req::IDLE, k(cx, "stance_end_blend"));
    frame().await;
}

/// `0xB54`: the leap. On the ground, facing the target now; `0x20AE(facing × 12 with height 200, 0.9, 45,
/// 200)` (no turn in the air); on landing its row `0x38C` (its callback `0xCAE` sets `wp+0x50` = 12, which
/// the draw below overwrites) and the slam 218 (blend 0, a plain request); to its end; 90 % the recovery
/// 213, else the blade stuck (205 an action, 206 queued, until 206 is done) and the pull 207; a yield.
async fn leap(cx: &Cx) {
    wait_grounded(cx).await;
    face_target_now(cx);
    let (s, h, t, d) = (k(cx, "leap_speed"), k(cx, "leap_height"), k(cx, "leap_time"), k(cx, "leap_drag"));
    hop(cx, 0.0, s, h, t, d, req::LEAP, false).await;
    attack_now(cx, Attack::Slam, k(cx, "slam_blend"));
    wait_done(cx).await;
    let stuck = rand(cx, "slam end") - k(cx, "recover_chance") >= 0.0;
    cx.borrow_mut().x.stuck = stuck;
    if !stuck {
        let b = k(cx, "recover_blend");
        action(cx, req::RECOVER, None, b).await;
    } else {
        let b = k(cx, "stuck_blend");
        push(cx, ECmd::Action { request: req::STUCK, attack: None, then: req::STUCK_HOLD, blend: b });
        frame().await;
        wait_done_or_air_still(cx).await;
        let b = k(cx, "pull_blend");
        action(cx, req::PULL, None, b).await;
    }
    frame().await;
}

/// `0xCBE`: the jump slash. On the ground, facing the target now; the crouch 3 (blend 0) for its length;
/// 4 (bit 16) and movement state 3 (`sys 82`: the facing × 18 with height 12, 0.9, 60 frames; `sys 83(3)`)
/// and at once its row `0x31C` and action 210 from frame 8 (blend 8; the asset step's clip starts there,
/// `enemy::STARTS`) with 6 queued; after it, unless 6 plays, a jump of height 0 (`sys 50` with a zero
/// vector: movement state 2); then the forward hop.
async fn jump_slash(cx: &Cx) {
    wait_grounded(cx).await;
    face_target_now(cx);
    let yaw = cx.borrow().body.yaw;
    let d = Vec3::from_yaw(yaw) * k(cx, "jslash_speed");
    let v = Vec3::new(d.x, k(cx, "jslash_height"), d.z);
    let b = k(cx, "hop_blend");
    play(cx, req::JUMP, b);
    wait_frames(cx, motion_frames(cx, req::JUMP)).await;
    play(cx, req::FALL, b);
    let (drag, time) = (k(cx, "jslash_drag"), k(cx, "jslash_time"));
    cx.borrow_mut().body.timed_jump(v, drag, time);
    if cx.borrow().body.timed.is_some() {
        let b = k(cx, "jslash_blend");
        push(cx, ECmd::Action { request: req::JUMP_SLASH, attack: Some(Attack::JumpSlash), then: req::AIR, blend: b });
        frame().await;
        if !in_air_motion(cx) {
            let y = pos(cx).y;
            {
                let mut io = cx.borrow_mut();
                let c = io.c.clone();
                io.body.jump(0.0, Vec3::ZERO, y, &c);
            }
            push(cx, ECmd::Jump { height: 0.0, velocity: Vec3::ZERO });
        }
    }
    fwd_hop(cx).await;
}

/// `0xED0`: the spin. On the ground; the wind-up 222 (blend 8, from frame 8: `enemy::STARTS`) until it is
/// done or the air motion plays, turning to the target; on the ground; its rows `0x15C` / `0xEC` (callback
/// `0x117C`: `wp+0x4C` = 9) and the guard row `0x1CC` (mode 2, [`Guard::Spin`]), 217 (blend 8); for at most
/// 180 frames: off the ground it ends; once a row landed, 15 frames more; gliding (`sys 40` mode 2 at the
/// stats block's `+0xC` = 3.6) at the target at its own height, or, within 30 of it, at a random point
/// 400–600 away; then, if a row landed, 219 (an action, blend 10).
async fn spin(cx: &Cx) {
    let mut after = k(cx, "spin_after_hit");
    wait_grounded(cx).await;
    play(cx, req::WINDUP, k(cx, "windup_blend"));
    wait_done_or_air(cx).await;
    wait_grounded(cx).await;
    let b = k(cx, "spin_blend");
    attack_keep(cx, Attack::Spin, b);
    guard(cx, Guard::Spin, req::SPIN);
    cx.borrow_mut().x.spin = SpinRun::Spinning;
    let mut t = k(cx, "spin_time");
    let speed = k(cx, "spin_speed");
    while t - 0.0 > 0.0 {
        if !grounded(cx) {
            cx.borrow_mut().x.spin = SpinRun::Over;
            break;
        }
        if cx.borrow().x.spin == SpinRun::Landed {
            if after - 0.0 > 0.0 {
                after -= step(cx);
            } else {
                break;
            }
        }
        let me = pos(cx);
        let mut goal = target(cx).unwrap_or(me);
        goal.y = me.y;
        if (goal - me).length() - k(cx, "spin_near") < 0.0 {
            goal = me + random_offset(cx, k(cx, "spin_away_min"), k(cx, "spin_away_max"));
        }
        attack_keep(cx, Attack::Spin, b);
        move_toward(cx, goal, 2, speed);
        t -= step(cx);
        frame().await;
    }
    let landed = cx.borrow().x.spin == SpinRun::Landed;
    if landed {
        let b = k(cx, "dizzy_blend");
        action(cx, req::DIZZY, None, b).await;
    }
    cx.borrow_mut().x.spin = SpinRun::Over;
}

/// `0x11A0`: on the ground, the taunt 220 (an action, blend 8).
async fn taunt(cx: &Cx) {
    wait_grounded(cx).await;
    let b = k(cx, "taunt_blend");
    action(cx, req::TAUNT, None, b).await;
}

/// `0x11BC`: on the ground, the laugh 212 (blend 8) until it is done or the air motion plays, turning to
/// the target; the idle (blend 8).
async fn laugh(cx: &Cx) {
    wait_grounded(cx).await;
    play(cx, req::LAUGH, k(cx, "laugh_blend"));
    wait_done_or_air(cx).await;
    play(cx, req::IDLE, k(cx, "laugh_end_blend"));
}

/// `0x870`: Sora on the ground above it (`*(target + 0x70)` = 0, KH1 y down): 50–300 above, the leap; 400
/// or more above, the back hop, the run in and the leap. Heights relative to its own (no world height).
async fn above(cx: &Cx) {
    let (t, me, on_floor) = {
        let io = cx.borrow();
        (io.w.target.filter(|_| io.has_target), io.w.pos, io.x.sora_grounded)
    };
    let Some(t) = t else { return };
    if !(t.y - me.y < 0.0 && on_floor) {
        return;
    }
    let dy = t.y - me.y;
    if dy - k(cx, "above_near") <= 0.0 && dy - k(cx, "above_far") >= 0.0 {
        leap(cx).await;
    } else if dy - k(cx, "above_high") <= 0.0 {
        back_hop_in(cx).await;
        leap(cx).await;
    }
}

/// `0x828`: close: 30 % a side hop and the slash; else (same draw) under one half the jump slash; else the
/// spin.
async fn close(cx: &Cx) {
    let r = rand(cx, "close");
    if r - k(cx, "close_slash_chance") < 0.0 {
        side_hop(cx).await;
        slash(cx).await;
    } else if r - k(cx, "close_jump_chance") < 0.0 {
        jump_slash(cx).await;
    } else {
        spin(cx).await;
    }
}

/// `0x7DA`: mid: 30 % the forward hop; then 40 % a side hop and the slash, else the spin.
async fn mid(cx: &Cx) {
    if rand(cx, "mid hop") - k(cx, "mid_hop_chance") < 0.0 {
        fwd_hop(cx).await;
    }
    if rand(cx, "mid slash") - k(cx, "mid_slash_chance") < 0.0 {
        side_hop(cx).await;
        slash(cx).await;
    } else {
        spin(cx).await;
    }
}

/// `0x756`: far: 60 % the back hop, the run in and the leap, then farther than 450 30 % the taunt; else 30 %
/// the laugh, then the spin.
async fn far(cx: &Cx) {
    if rand(cx, "far") - k(cx, "far_leap_chance") < 0.0 {
        back_hop_in(cx).await;
        leap(cx).await;
        if dist_to_target(cx) - k(cx, "far_taunt_range") > 0.0 && rand(cx, "far taunt") - k(cx, "far_taunt_chance") < 0.0 {
            taunt(cx).await;
        }
    } else {
        if rand(cx, "far laugh") - k(cx, "far_laugh_chance") < 0.0 {
            laugh(cx).await;
        }
        spin(cx).await;
    }
}

/// The appearance (`0x992` → `0x1B58`, spawned by the room): the fade-in (30 frames); KH1's actor flag
/// 0x80 for 30 frames (`0x24FA`: not ported); facing Sora now; action 69 with bit 23 (blend 8), 6 queued,
/// a jump of height 0 (`sys 50` with a zero vector; its effect 2: M13); while in the air; then the idle
/// state.
async fn appear(cx: Cx) {
    if cx.borrow().x.spawned {
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        face_sora_now(&cx);
        let b = k(&cx, "appear_blend");
        push(&cx, ECmd::Action { request: req::APPEAR, attack: None, then: req::AIR, blend: b });
        push(&cx, ECmd::Held);
        let y = pos(&cx).y;
        {
            let mut io = cx.borrow_mut();
            let c = io.c.clone();
            io.body.jump(0.0, Vec3::ZERO, y, &c);
        }
        push(&cx, ECmd::Jump { height: 0.0, velocity: Vec3::ZERO });
        frame().await;
        while cx.borrow().w.airborne {
            frame().await;
        }
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The idle behaviour (`0x946`): forever the wander about the current place (`0x1A48(100, 300, &home)`,
/// the shared routine), then 60 % the taunt, a yield.
async fn idle(cx: Cx) {
    loop {
        let c = |n: &str| k(&cx, n);
        let (inner, outer, band, timeout) = (c("wander_inner"), c("wander_outer"), c("wander_band"), c("wander_timeout"));
        let (chance, lo, hi, home, blend) = (c("wander_idle_chance"), c("wander_idle_min"), c("wander_idle_max"), c("wander_home_max"), c("idle_blend"));
        wander(&cx, inner, outer, band, timeout, chance, lo, hi, home, blend).await;
        if rand(&cx, "idle taunt") - c("idle_taunt_chance") < 0.0 {
            taunt(&cx).await;
        }
        frame().await;
    }
}

/// The engaged behaviour (`0x6B4`; the leash `0x118C` before and after: not ported): Sora above it
/// (`0x870`), by distance close (< 250) / mid (< 450) / far, Sora above again; the engaged search (Sora)
/// and engaged again.
async fn engaged(cx: Cx) {
    above(&cx).await;
    let d = dist_to_target(&cx);
    if d - k(&cx, "close_range") < 0.0 {
        close(&cx).await;
    } else if d - k(&cx, "mid_range") < 0.0 {
        mid(&cx).await;
    } else {
        far(&cx).await;
    }
    above(&cx).await;
    // `0x712`: the engaged search (`0x1DF0(&wp30, 1, 5000, 1)` → `0x17D4`): Sora (Re:CoM's battles have
    // only him).
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// The hit behaviour (`0x5CC`): after its own launch the get-up slash and the spin; else after a hit (or a
/// clash) within 300: while it has raised the stance at most once, 30 % the stance, else the counter slash
/// (past that the count starts again with the counter slash); farther, the jump slash; with no hit marked,
/// the leap. Then engaged.
async fn hit(cx: Cx) {
    let (side, marked, stances) = {
        let io = cx.borrow();
        (io.x.side, io.x.hit, io.x.stances)
    };
    if side != 2 {
        getup_slash(&cx).await;
        spin(&cx).await;
    } else if marked {
        if dist_to_target(&cx) - k(&cx, "hit_range") < 0.0 {
            if (stances as f32) - k(&cx, "hit_stance_max") <= 0.0 {
                if rand(&cx, "stance") - k(&cx, "hit_stance_chance") < 0.0 {
                    stance(&cx).await;
                } else {
                    counter(&cx).await;
                }
            } else {
                cx.borrow_mut().x.stances = 0;
                counter(&cx).await;
            }
        } else {
            jump_slash(&cx).await;
        }
    } else {
        leap(&cx).await;
    }
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// The supervisor (`0x2824`): after the init frame, each frame: while idle, the search (Sora within 1000;
/// world 11: always) with Sora within ±100 of its height → engaged with the target, else no target; then a
/// state request restarts the behaviour thread (the idle clears the target, `0x92E`; the engaged and hit
/// states set it, `0x690` / `0x5A8`: Sora, the only party member and attacker).
async fn supervisor(cx: Cx) {
    frame().await;
    loop {
        if cx.borrow().state.0 == st::IDLE {
            if let Some(t) = search(&cx, k(&cx, "idle_search")) {
                let dy = t.y - pos(&cx).y;
                if dy - k(&cx, "engage_dy_max") <= 0.0 && dy - k(&cx, "engage_dy_min") >= 0.0 {
                    set_target(&cx, true);
                    set_state(&cx, st::ENGAGED);
                } else {
                    set_target(&cx, false);
                }
            }
        }
        if let Some(s) = take_state(&cx) {
            set_target(&cx, s != st::IDLE);
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Pirate's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as the Shadow's: per-enemy seeds). `frames`: KH1
    /// motion lengths by request. `spawned`: the room spawned it (its appearance plays; every Re:CoM battle
    /// appearance, owner 2026-11-02).
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
        }
        if !self.started {
            self.started = true;
            {
                let mut io = self.io.borrow_mut();
                // `0x2960`: home (`wp+0x10`) = the position; the state object cleared (`0x1EB8`).
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
    /// The hit handler (`0x2A3C`): `wp+0x48` = 2. A clash (kinds 6 / 7): turn to the attacker, the hit
    /// marked, the hit state. A damaging hit while armoured never comes here (the host's armour keeps
    /// Re:CoM's state and calls [`Behaviour::on_armoured_hit`]). Another damaging hit of kind 4 / 5
    /// (`0x25CA`): its own launch (`sys 38`, ground or air base by the host's movement state), the side
    /// it came from (`0x151E`: `sys 100` of its facing against the push < 0 is the front), the hit state
    /// (the handler ends there: the mark is not set). Any other: the hit marked, the hit state.
    fn on_hit(&self, news: HitNews) {
        let (kind, dir) = {
            let io = self.io.borrow();
            (io.x.hit_kind, io.x.hit_dir)
        };
        {
            let mut io = self.io.borrow_mut();
            io.x.side = 2;
            io.x.launch = None;
        }
        if news == HitNews::Clash || matches!(kind, 6 | 7) {
            face_sora(&self.io);
            self.io.borrow_mut().x.hit = true;
        } else if kind == 4 || kind == 5 {
            let mut io = self.io.borrow_mut();
            let front = cos_between(Vec3::from_yaw(io.body.yaw), dir) < 0.0;
            io.x.side = if front { 0 } else { 1 };
            io.x.launch = Some(ScriptedLaunch { ground: req::OWN_LAUNCH, air: req::OWN_AIR_LAUNCH, hop: io.c.num("own_launch_hop"), speed: io.c.num("own_launch_speed"), down: io.c.num("own_launch_down") });
        } else {
            self.io.borrow_mut().x.hit = true;
        }
        set_state(&self.io, st::HIT);
    }
    fn armoured(&self, playing: Option<u16>, launch_base: u16) -> bool {
        let Some(r) = playing else { return false };
        // `0x267E`: its attacks; `sys 70` (`0x1402B7EE0`, unsigned): lying after a launch.
        let d = u32::from(r).wrapping_sub(u32::from(launch_base));
        ARMOUR.contains(&r) || (d < 10 && d % 5 == 3)
    }
    /// `sys 42` (`0x1402B7E60`): the flinch's push slide (`G+0x2C` × the stats block's `+0xC4`, the
    /// knock-back weight; `crate::enemy::react`'s); `wp+0x48` = 2 (the handler's first writes).
    fn on_armoured_hit(&self, dir: Vec3) {
        let mut io = self.io.borrow_mut();
        io.x.side = 2;
        let p = io.c.flinch_push() * io.c.weight();
        io.body.push(dir, p);
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
    fn scripted_launch(&self) -> Option<ScriptedLaunch> {
        self.io.borrow_mut().x.launch.take()
    }
    fn set_world(&self, w: Option<u8>) {
        self.io.borrow_mut().x.olympus = w == Some(OLYMPUS);
    }
    fn set_target_grounded(&self, on_ground: bool) {
        self.io.borrow_mut().x.sora_grounded = on_ground;
    }
    /// A row landed: the spin's (`0x117C`: `wp+0x4C` = 9); the slam's (`0xCAE`: `wp+0x50` = 12, which the
    /// slam's own draw overwrites before it is read: nothing to keep).
    fn on_landed(&self, attack: u8) {
        if attack == Attack::Spin.index() {
            let mut io = self.io.borrow_mut();
            if io.x.spin == SpinRun::Spinning {
                io.x.spin = SpinRun::Landed;
            }
        }
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
    use crate::enemy::host::model::Model;

    fn consts() -> Consts {
        // The player's `ex_2100.bd` / `btltbl.bin` / KH1 exe numbers as the asset step reads them (its
        // manifest on the session's files), the exe's gravity and turn rate.
        let v: &[(&str, f32)] = &[
            ("walk_speed", 3.0), ("run_speed", 6.0), ("idle_search", 1000.0), ("engage_dy_max", 100.0), ("engage_dy_min", -100.0), ("world11_search", 10000.0),
            ("close_range", 250.0), ("mid_range", 450.0), ("above_near", -50.0), ("above_far", -300.0), ("above_high", -400.0), ("close_slash_chance", 0.3),
            ("close_jump_chance", 0.5), ("mid_hop_chance", 0.3), ("mid_slash_chance", 0.4), ("far_leap_chance", 0.6), ("far_taunt_range", 450.0),
            ("far_taunt_chance", 0.3), ("far_laugh_chance", 0.3), ("wander_inner", 100.0), ("wander_outer", 300.0), ("wander_band", 100.0),
            ("wander_timeout", 300.0), ("wander_idle_chance", 0.4), ("wander_idle_min", 90.0), ("wander_idle_max", 180.0), ("wander_home_max", 700.0),
            ("idle_blend", 8.0), ("idle_taunt_chance", 0.6), ("hit_range", 300.0), ("hit_stance_max", 1.0), ("hit_stance_chance", 0.3), ("counter_blend", 0.0),
            ("stance_time", 120.0), ("stance_end_blend", 12.0), ("stance_blend", 6.0), ("getup_blend", 0.0), ("leap_speed", 12.0), ("leap_height", 200.0),
            ("leap_drag", 0.9), ("leap_time", 45.0), ("slam_blend", 0.0), ("recover_chance", 0.9), ("recover_blend", 0.0), ("stuck_blend", 0.0),
            ("pull_blend", 0.0), ("jslash_speed", 18.0), ("jslash_height", 12.0), ("jslash_drag", 0.9), ("jslash_time", 60.0), ("jslash_blend", 8.0),
            ("slash_blend", 0.0), ("slash_end_blend", 8.0), ("spin_after_hit", 15.0), ("windup_blend", 8.0), ("spin_blend", 8.0), ("spin_time", 180.0),
            ("spin_speed", 3.6), ("spin_near", 30.0), ("spin_away_min", 400.0), ("spin_away_max", 600.0), ("dizzy_blend", 10.0), ("taunt_blend", 8.0),
            ("laugh_blend", 8.0), ("laugh_end_blend", 8.0), ("hop_blend", 0.0), ("fwd_hop_angle", 0.0), ("fwd_hop_speed", 30.0), ("fwd_hop_height", 150.0),
            ("fwd_hop_time", 40.0), ("side_hop_angle", 15.0), ("side_hop_chance", 0.5), ("side_hop_speed", 25.0), ("side_hop_height", 30.0),
            ("side_hop_time", 30.0), ("back_hop_angle", 180.0), ("back_hop_speed", 20.0), ("back_hop_height", 100.0), ("back_hop_time", 30.0),
            ("approach_timeout", 300.0), ("approach_retarget", 20.0), ("approach_dist", 350.0), ("hop_drag", 0.9), ("appear_fade", 30.0),
            ("appear_blend", 8.0), ("own_launch_hop", 100.0), ("own_launch_speed", 30.0), ("own_launch_down", 0.0), ("attack_blend", 8.0),
            ("air_accel", 0.1), ("air_drag", 0.95), ("fall_cap", 20.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("launch_min", 20.0),
            ("launch_max", 35.0), ("launch_hop", 1.3499999), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("weight", 1.0),
        ];
        let mut v: Vec<(String, f32)> = v.iter().map(|(k, x)| (k.to_string(), *x)).collect();
        v.push(("gravity".into(), crate::params::GRAVITY));
        v.push(("turn_rate".into(), std::f32::consts::PI / 16.0));
        Table::new(v, NAMES).unwrap()
    }

    /// The player's `xa_ex_2100.mset` lengths by request (the manifest's: 210 and 222 from frame 8).
    fn frames() -> Vec<(u16, f32)> {
        vec![
            (0, 290.0), (1, 60.0), (2, 40.0), (3, 18.0), (4, 12.0), (5, 22.0), (6, 90.0), (7, 24.0), (69, 32.0), (72, 42.0), (73, 50.0), (74, 42.0), (75, 40.0),
            (78, 36.0), (79, 36.0), (82, 8.0), (83, 20.0), (84, 28.0), (85, 88.0), (86, 64.0), (92, 12.0), (200, 16.0), (201, 10.0), (202, 34.0), (203, 90.0),
            (205, 142.0), (206, 44.0), (207, 144.0), (208, 48.0), (209, 24.0), (210, 58.0), (211, 60.0), (212, 166.0), (213, 46.0), (214, 178.0), (215, 164.0),
            (216, 110.0), (217, 48.0), (218, 52.0), (219, 152.0), (220, 150.0), (221, 96.0), (222, 122.0), (235, 88.0), (236, 64.0), (240, 8.0), (241, 20.0),
            (242, 28.0), (243, 88.0), (244, 2.0), (245, 6.0), (249, 2.0), (250, 12.0),
        ]
    }

    /// The asset step's looping clips among the requests the host plays steered (`enemy::LOOPS` and the
    /// pose rule).
    fn loops() -> Vec<u16> {
        vec![0, 1, 6, 203, 206, 208, 209, 210, 212, 214, 217, 220, 222]
    }

    /// A flat floor at y = 0, Sora at `sora`; an action holds the AI for its motion's length; the current
    /// request is the last one played.
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
                let airborne = self.ai.body().airborne();
                let w = World { step: 1.0, pos: self.pos, target: Some(sora), grounded: !airborne, airborne, moved: self.moved, motion: self.current };
                if self.busy > 0.0 {
                    self.busy -= 1.0;
                } else {
                    for c in self.ai.frame(w) {
                        match c {
                            ECmd::Action { request, .. } => {
                                self.busy = frames().iter().find(|x| x.0 == request).map_or(30.0, |x| x.1);
                                self.current = Some(request);
                            }
                            ECmd::Motion { request, .. } | ECmd::Attack { request, .. } | ECmd::Queue { request, .. } => self.current = Some(request),
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

    /// A spawned Pirate past its appearance, idle, Sora far away.
    fn ready(seed: u32) -> Sim {
        let mut s = sim(seed, true, None);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 40);
        assert!(s.ai.risen());
        s.log.clear();
        s
    }

    /// A Pirate past its appearance running `f` alone (Sora its target, the hit state's thread).
    fn with<F: std::future::Future<Output = ()> + 'static>(seed: u32, f: impl FnOnce(Cx) -> F) -> Sim {
        let mut s = ready(seed);
        s.ai.io.borrow_mut().has_target = true;
        s.ai.io.borrow_mut().state = (st::HIT, false);
        let fut = f(s.ai.io.clone());
        s.ai.behaviour = Some(Box::pin(async move {
            fut.await;
            halt().await;
        }));
        s
    }

    fn attacks(log: &[Cmd]) -> Vec<Attack> {
        log.iter()
            .filter_map(|c| match c {
                ECmd::Attack { attack, .. } => Some(*attack),
                ECmd::Action { attack: Some(a), .. } => Some(*a),
                _ => None,
            })
            .collect()
    }

    fn motions(log: &[Cmd]) -> Vec<u16> {
        log.iter()
            .filter_map(|c| match c {
                ECmd::Motion { request, .. } | ECmd::Attack { request, .. } | ECmd::Action { request, .. } => Some(*request),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_names_cover_the_numbers() {
        assert!(Table::new(vec![], NAMES).is_err());
        assert_eq!((consts().num("spin_speed"), consts().num("leap_height")), (3.6, 200.0));
    }

    /// `0x1B58`: the fade-in, facing Sora, action 69 with bit 23 and 6 queued, a jump of 0; once on the
    /// ground, the idle state and the wander.
    #[test]
    fn spawned_appearance_then_idle() {
        let mut s = sim(1, true, None);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 2);
        assert!(s.log.contains(&ECmd::FadeIn(30.0)), "{:?}", s.log);
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Action { request: req::APPEAR, then: req::AIR, attack: None, blend } if *blend == 8.0)));
        assert!(s.log.contains(&ECmd::Held) && s.log.iter().any(|c| matches!(c, ECmd::Jump { .. })));
        assert!(!s.ai.risen());
        s.run(Vec3::new(0.0, 0.0, 5000.0), 40);
        assert!(s.ai.risen() && s.log.contains(&ECmd::State(st::IDLE)));
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Motion { request: req::WALK, .. })), "it wanders");
    }

    /// The search: Sora within 1000 at its height (±100) engages; farther, or 1.5 m above, not; in KH1's
    /// world 11 (Re:CoM's Olympus) at any distance.
    #[test]
    fn the_search_and_its_height_window() {
        let mut s = ready(2);
        s.run(Vec3::new(0.0, 0.0, 900.0), 2);
        assert!(s.log.contains(&ECmd::State(st::ENGAGED)), "{:?}", s.log);
        let mut s = ready(2);
        s.run(Vec3::new(0.0, 0.0, 1100.0), 3);
        assert!(!s.log.contains(&ECmd::State(st::ENGAGED)));
        let mut s = ready(2);
        s.run(Vec3::new(0.0, -150.0, 900.0), 3);
        assert!(!s.log.contains(&ECmd::State(st::ENGAGED)), "Sora 1.5 m above");
        let mut s = sim(2, true, Some(OLYMPUS));
        s.run(Vec3::new(0.0, 0.0, 5000.0), 40);
        assert!(s.log.contains(&ECmd::State(st::ENGAGED)), "Olympus: 100 m");
    }

    /// Close (< 250), mid (< 450) and far: the slash, the jump slash, the leap's slam and the spin all show
    /// up; the counter and the get-up slashes only after hits.
    #[test]
    fn close_mid_and_far_attack() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..40 {
            let d = [150.0, 350.0, 900.0][seed as usize % 3];
            let mut s = ready(seed);
            s.run(Vec3::new(0.0, 0.0, d), 900);
            let a = attacks(&s.log);
            assert!(!a.iter().any(|x| matches!(x, Attack::Counter | Attack::GetUpFront | Attack::GetUpBack)), "seed {seed}: {a:?}");
            seen.extend(a);
        }
        for a in [Attack::Slash, Attack::JumpSlash, Attack::Slam, Attack::Spin] {
            assert!(seen.contains(&a), "{a:?} never: {seen:?}");
        }
    }

    /// `0x267E`: its attacks take no reaction (`sys 42`), and lying after a launch (`sys 70`:
    /// `(request − base) < 10`, `% 5 == 3`). Before any launch the base is 0, so KH1 also armours its jump
    /// crouch 3 (and 8).
    #[test]
    fn the_armour() {
        let s = ready(1);
        for r in ARMOUR {
            assert!(s.ai.armoured(Some(r), 0x52), "{r}");
        }
        for r in [0, 1, 2, 208, 217, 222, 219, 72, 0x52, 0x53, 0x54, 0x56, 240, 244] {
            assert!(!s.ai.armoured(Some(r), 240) || r == 243, "{r}");
        }
        assert!(s.ai.armoured(Some(243), 240) && s.ai.armoured(Some(248), 240) && s.ai.armoured(Some(0x55), 0x52) && s.ai.armoured(Some(0x5A), 0x52));
        assert!(!s.ai.armoured(Some(0x55), 240), "below the base");
        assert!(s.ai.armoured(Some(req::JUMP), 0) && !s.ai.armoured(Some(req::JUMP), 0x52));
        assert!(!s.ai.armoured(None, 0));
        // `sys 42`: the push slide only; no state asked for.
        s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
        let io = s.ai.io.borrow();
        assert!(!io.state.1 && io.body.push.is_some_and(|(d, v)| d.z < 0.0 && (v - 80.0 * 0.223_143_55).abs() < 1e-3));
    }

    /// `0x25CA`: a knock or launch (kinds 4 / 5) is its own launch (bases 240 / 250, hop 100, speed 30, no
    /// down time) and the hit state then plays the get-up slash of its side (235 from the front, 236 from
    /// behind) and the spin. A flinch (kind 3) marks the hit instead.
    #[test]
    fn its_own_launch_then_the_get_up_slash() {
        for (dir, want) in [(-1.0f32, req::GETUP_FRONT), (1.0, req::GETUP_BACK)] {
            let mut s = ready(3);
            s.ai.io.borrow_mut().has_target = true;
            s.ai.body().yaw = 0.0;
            s.ai.set_hit_kind(5);
            s.ai.set_hit_dir(Vec3::new(0.0, 0.0, dir));
            Behaviour::on_hit(&s.ai, HitNews::Damage);
            let l = s.ai.scripted_launch().expect("its own launch");
            assert_eq!((l.ground, l.air, l.hop, l.speed, l.down), (240, 250, 100.0, 30.0, 0.0));
            assert!(!s.ai.io.borrow().x.hit, "the launch path doesn't mark the hit");
            s.run(Vec3::new(0.0, 0.0, 200.0), 2 + want as usize - 200);
            let m = motions(&s.log);
            let g = m.iter().position(|&r| r == want).unwrap_or_else(|| panic!("{want}: {m:?}"));
            s.run(Vec3::new(0.0, 0.0, 200.0), 120);
            let m = motions(&s.log);
            assert!(m[g..].contains(&req::WINDUP), "then the spin: {m:?}");
        }
        let s = ready(3);
        s.ai.set_hit_kind(3);
        Behaviour::on_hit(&s.ai, HitNews::Damage);
        assert!(s.ai.scripted_launch().is_none() && s.ai.io.borrow().x.hit);
    }

    /// A clash or a hit within 300: the stance (30 %, while raised at most once) or the counter slash;
    /// farther, the jump slash; no hit marked: the leap.
    #[test]
    fn the_hit_state_counters() {
        let (mut stance, mut counter) = (false, false);
        for seed in 0..30 {
            let mut s = ready(seed);
            s.ai.set_hit_kind(3);
            Behaviour::on_hit(&s.ai, if seed % 2 == 0 { HitNews::Clash } else { HitNews::Damage });
            s.run(Vec3::new(0.0, 0.0, 200.0), 3);
            let m = motions(&s.log);
            stance |= m.contains(&req::STANCE);
            counter |= m.contains(&req::COUNTER);
            assert!(m.contains(&req::STANCE) || m.contains(&req::COUNTER), "seed {seed}: {m:?}");
        }
        assert!(stance && counter);
        let mut s = ready(1);
        Behaviour::on_hit(&s.ai, HitNews::Clash);
        s.run(Vec3::new(0.0, 0.0, 600.0), 40);
        assert!(attacks(&s.log).contains(&Attack::JumpSlash), "{:?}", s.log);
        let mut s = ready(1);
        s.ai.io.borrow_mut().has_target = true;
        set_state(&s.ai.io, st::HIT);
        s.run(Vec3::new(0.0, 0.0, 600.0), 3);
        assert!(motions(&s.log).contains(&req::LEAP), "no hit marked: the leap {:?}", s.log);
        // Past one stance the count starts again with the counter slash.
        let mut s = ready(4);
        s.ai.io.borrow_mut().x.stances = 2;
        Behaviour::on_hit(&s.ai, HitNews::Clash);
        s.run(Vec3::new(0.0, 0.0, 200.0), 3);
        assert!(attacks(&s.log).contains(&Attack::Counter) && s.ai.io.borrow().x.stances == 0);
    }

    /// The stance: 208 at blend 6 with its blade row (guard 1 with 208), held 120 frames turning to Sora,
    /// then the idle at blend 12.
    #[test]
    fn the_stance_holds_its_guard_row() {
        let mut s = with(5, |cx| async move { stance(&cx).await });
        s.run(Vec3::new(0.0, 0.0, 200.0), 2);
        assert!(s.log.contains(&ECmd::Motion { request: req::STANCE, blend: 6.0 }) && s.log.contains(&ECmd::Guard(Some((Guard::Stance.index(), req::STANCE)))));
        s.run(Vec3::new(0.0, 0.0, 200.0), 117);
        assert!(!s.log.contains(&ECmd::Motion { request: req::IDLE, blend: 12.0 }));
        s.run(Vec3::new(0.0, 0.0, 200.0), 4);
        assert!(s.log.contains(&ECmd::Motion { request: req::IDLE, blend: 12.0 }));
    }

    /// The spin: 222, then 217 with its two blade rows and its body guard row, gliding at Sora at 3.6 a
    /// frame for 180 frames; a row that lands ends it 15 frames later with 219.
    #[test]
    fn the_spin_glides_and_ends() {
        let mut s = with(6, |cx| async move { spin(&cx).await });
        let sora = Vec3::new(0.0, 0.0, 3000.0);
        s.run(sora, 125);
        assert!(motions(&s.log).contains(&req::WINDUP));
        assert_eq!(s.log.iter().filter(|c| matches!(c, ECmd::Attack { request: req::SPIN, .. })).count(), 1, "217 from the wind-up's end (122 frames)");
        assert!(s.log.contains(&ECmd::Guard(Some((Guard::Spin.index(), req::SPIN)))));
        let z = s.pos.z;
        s.run(sora, 100);
        assert_eq!(s.log.iter().filter(|c| matches!(c, ECmd::Attack { request: req::SPIN, .. })).count(), 1, "asked for once");
        assert!(s.pos.z - z > 3.5 * 95.0, "glides at 3.6: {}", s.pos.z - z);
        s.ai.on_landed(Attack::Spin.index());
        let n = s.log.len();
        s.run(sora, 15);
        assert!(!s.log[n..].iter().any(|c| matches!(c, ECmd::Action { request: req::DIZZY, .. })));
        s.run(sora, 3);
        assert!(s.log[n..].iter().any(|c| matches!(c, ECmd::Action { request: req::DIZZY, .. })), "219 after a hit");
        // Without a hit: 180 frames and no 219.
        let mut s = with(6, |cx| async move { spin(&cx).await });
        s.run(sora, 125 + 185);
        assert!(!motions(&s.log).contains(&req::DIZZY));
        assert_eq!(s.ai.io.borrow().x.spin, SpinRun::Over);
    }

    /// The leap: the crouch 200 for its 16 frames, 201 and a timed jump of 45 frames at 12 a frame (drag
    /// 0.9, √0.9 a frame) 200 high, 202 at the top with 203 queued; on landing the slam 218 (a plain
    /// request with its row), then 213, or the stuck blade 205 → 206 → 207.
    #[test]
    fn the_leap_lands_and_slams() {
        let (mut recover, mut stuck) = (false, false);
        for seed in 0..40 {
            let mut s = with(seed, |cx| async move { leap(&cx).await });
            let mut top = 0.0f32;
            for _ in 0..70 {
                s.run(Vec3::new(0.0, 0.0, 3000.0), 1);
                top = top.min(s.pos.y);
            }
            let m = motions(&s.log);
            assert!(m.starts_with(&[req::LEAP, req::LEAP + 1]), "{m:?}");
            assert!(m.contains(&(req::LEAP + 2)) && s.log.contains(&ECmd::Queue { request: req::LEAP + 3, blend: 0.0 }));
            assert!(s.log.iter().any(|c| matches!(c, ECmd::Attack { request: req::SLAM, .. })), "the slam on landing: {m:?}");
            assert!((top + 200.0).abs() < 12.0, "200 high: {top}");
            let k = 0.9f32.sqrt();
            let travel = 12.0 * (1.0 - k.powi(45)) / (1.0 - k);
            assert!((s.pos.z - travel).abs() < 15.0, "{} vs {travel}", s.pos.z);
            s.run(Vec3::new(0.0, 0.0, 3000.0), 400);
            let m = motions(&s.log);
            recover |= m.contains(&req::RECOVER);
            stuck |= m.contains(&req::STUCK) && m.contains(&req::PULL);
        }
        assert!(recover && stuck);
    }

    /// The jump slash: the crouch 3, 4, a timed jump of 60 frames at 18 a frame and, at once, action 210
    /// (its row) with 6 queued; then the forward hop.
    #[test]
    fn the_jump_slash() {
        let mut s = with(7, |cx| async move { jump_slash(&cx).await });
        s.run(Vec3::new(0.0, 0.0, 3000.0), 20);
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Action { request: req::JUMP_SLASH, attack: Some(Attack::JumpSlash), then: req::AIR, .. })), "{:?}", s.log);
        assert!(s.ai.body().timed.is_some());
        s.run(Vec3::new(0.0, 0.0, 3000.0), 120);
        let m = motions(&s.log);
        assert_eq!(m.iter().filter(|&&r| r == req::JUMP).count(), 2, "then the forward hop: {m:?}");
    }

    /// A long fight: the AI acts at least every 600 frames (the host's quiet watchdog), with Sora circling
    /// 2–7 m away and a hit every 150–400 frames (12 seeds, 20 000 frames each).
    #[test]
    fn a_long_fight_never_stalls() {
        for seed in 0..12u32 {
            let mut s = ready(seed);
            let (mut last, mut worst, mut next_hit) = (0usize, 0usize, 300usize);
            let mut r = Kh1Rand::new(seed + 100);
            for f in 0..20_000usize {
                let a = f as f32 / 600.0;
                let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
                let sora = Vec3::new(a.sin() * rad, 0.0, a.cos() * rad);
                if f == next_hit {
                    let kind = [3u8, 3, 4, 5][(r.next_f32() * 4.0) as usize % 4];
                    s.ai.set_hit_kind(kind);
                    s.ai.set_hit_dir(Vec3::new(0.0, 0.0, if r.next_f32() < 0.5 { 1.0 } else { -1.0 }));
                    if s.current.is_some_and(|c| s.ai.armoured(Some(c), 0x52)) {
                        s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
                    } else {
                        Behaviour::on_hit(&s.ai, if r.next_f32() < 0.15 { HitNews::Clash } else { HitNews::Damage });
                        let _ = s.ai.scripted_launch();
                        s.busy = 34.0;
                    }
                    next_hit = f + 150 + (r.next_f32() * 250.0) as usize;
                }
                let n = s.log.len();
                s.run(sora, 1);
                if s.log[n..].iter().any(|c| !matches!(c, ECmd::Yaw(_) | ECmd::Motion { request: 1 | 2, .. })) || s.busy > 0.0 || s.ai.body().walk != Vec3::ZERO {
                    worst = worst.max(f - last);
                    last = f;
                }
            }
            assert!(worst < 600, "seed {seed}: {worst} frames without acting");
        }
    }

    /// The Pirate through the host's rules (`crate::enemy::host::model`, what `kh1-recom`'s
    /// `win::kh1_enemy` decides) with its class map (`ground::pirate::CLASS.attack_states`) and the asset
    /// step's looping clips.
    fn hosted(s: Sim) -> Model<Consts> {
        let Sim { ai, pos, .. } = s;
        let mut m = Model::new(Box::new(ai), consts(), frames(), loops(), vec![3, 3, 2, 3, 2, 2, 2]);
        m.pos = pos;
        m
    }

    /// The spin in the host: 217 asked for once and played in state 2 without a break for its 180 frames
    /// (past three cycles of its 48), never released.
    #[test]
    fn hosted_the_spin_plays_217_for_180_frames() {
        // The routine after it plays its next motion (here the idle).
        let s = with(8, |cx| async move {
            spin(&cx).await;
            play(&cx, req::IDLE, 8.0);
        });
        let mut m = hosted(s);
        m.run(Vec3::new(0.0, 0.0, 3000.0), 400);
        assert_eq!(m.released, 0);
        assert_eq!(m.attacks_of(req::SPIN).len(), 1, "{:?}", m.attacks_of(req::SPIN));
        let run = m.longest_run(req::SPIN);
        assert!((178..=182).contains(&run), "180 frames of 217: {run}");
        assert!(m.trace.iter().all(|t| t.1 != Some(req::SPIN) || t.0 == 2), "217 plays in state 2");
    }

    /// The slash in the host: 214 (a plain request with its row) plays in state 3 to its end (178 frames,
    /// looped in Re:CoM: the script ends it), never released, then the idle.
    #[test]
    fn hosted_the_slash_plays_214_to_its_end() {
        let s = with(9, |cx| async move { slash(&cx).await });
        let mut m = hosted(s);
        m.run(Vec3::new(0.0, 0.0, 200.0), 260);
        assert_eq!(m.released, 0);
        assert_eq!(m.attacks_of(req::SLASH).len(), 1);
        let run = m.longest_run(req::SLASH);
        // KH1 plays it from the requesting frame; Re:CoM's attack state begins one update later.
        assert!((176..=178).contains(&run), "214's 178 frames: {run}");
        assert!(m.trace.iter().all(|t| t.1 != Some(req::SLASH) || t.0 == 3));
        assert_eq!(m.current, Some(req::IDLE));
    }

    /// A long fight in the host (12 seeds, 20 000 frames, Sora circling 2–7 m away): no attack is
    /// released, none is asked for again within two frames, the looping spin plays only in state 2.
    #[test]
    fn hosted_a_long_fight_never_loops_an_attack() {
        for seed in 0..12u32 {
            let mut m = hosted(ready(seed));
            for f in 0..20_000usize {
                let a = f as f32 / 600.0;
                let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
                m.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
            }
            assert_eq!(m.released, 0, "seed {seed}");
            let issued: Vec<(usize, u16)> = m.log.iter().filter_map(|(f, c)| if let ECmd::Attack { request, .. } = c { Some((*f, *request)) } else { None }).collect();
            assert!(issued.len() > 10, "seed {seed}: it attacks ({})", issued.len());
            for w in issued.windows(2) {
                assert!(w[1].1 != w[0].1 || w[1].0 > w[0].0 + 2, "seed {seed}: {} asked again at {} after {}", w[0].1, w[1].0, w[0].0);
            }
            assert!(m.trace.iter().all(|t| t.1 != Some(req::SPIN) || t.0 == 2), "seed {seed}: 217 outside state 2");
        }
    }

    /// The guard rows go with their requests (the shared enemy-guard step): the spin's body row with 217,
    /// the stance's blade row with 208.
    #[test]
    fn guard_rows_and_attack_rows() {
        assert_eq!((Guard::Spin.row(), Guard::Spin.requests(), Guard::Stance.row(), Guard::Stance.requests()), (0x1CC, &[217u16][..], 0x46C, &[208u16][..]));
        assert_eq!(Attack::Spin.rows(), &[0xEC, 0x15C]);
        assert_eq!(Attack::GetUpBack.rows(), Attack::GetUpFront.rows());
        assert_eq!((Attack::Slam.index(), Attack::ALL.len()), (3, 7));
        assert!(ARMOUR.iter().all(|r| Attack::ALL.iter().any(|a| a.request() == *r)));
    }
}
