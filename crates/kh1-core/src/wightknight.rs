//! KH1 FM's Wight Knight (`xa_ex_2200.mdls` part 5, `ex_2200.bd`), engine-agnostic: its AI script on the shared
//! enemy framework (`crate::enemy`). M11 (`docs/recom-m11-enemies.md` § The Wight Knight). Re:CoM runs it
//! natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), Re:CoM plays the
//! motions, collides the body and the attacks, applies the damage.
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2200.bd` at checked offsets);
//! nothing of the game is in this file. Script offsets are the player's `ex_2200.bd`'s (Checked (files): the script
//! disassembled with `tools/kh1-ai-script.py`, code from `0x618`; the VM's ops and the syscalls named, Checked
//! (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x22F4`: init `0x23FE` (`wp+0x5C` the world, `wp+0x64`: `sys 105`'s second word is 8, the stats block's walk `+4` = 5, the
//!   state object `wp+0x20`, the hit handler `0x24D2`, the search object `wp+0x30`, `wp+0x50` / `+0x58` / `+0x60` = 0,
//!   the stat record `0x14` / `0x80`), a yield, the target party 0, the appearance thread `0xA90`; then each frame: an
//!   effect ends unless 212 / 218 / 207 / 213 / 200 / 69 plays (M13); a state request restarts the behaviour thread:
//!   1 engaged `0x7AA` → `0x7E2`, 2 hit `0x618` → `0x664`. The Wight Knight has no idle state: it engages at once.
//! - the hit handler `0x24D2`: a clash (kinds 6 / 7) asks for the hit state; the damage (`0x1E38` → `sys 44`): a
//!   killing hit of kind 3 becomes kind 4 (`0x222A`) and the death (`0x2644`, Re:CoM's defeat here); a living hit
//!   asks for the hit state. Hits of attribute 8 / 10 take other paths (`wp+0x50` / `+0x58` = 1 and the hit state):
//!   not carried (the host passes no attribute; every hit of Sora's is attribute 0, as every port).
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0x2764` (only in KH1's world 11: `0x89E`, `0x1120`),
//! the death routine `0x1F88` / `0x2644` (Re:CoM's defeat, prizes, EXP), the hanging appearance (spawn type 5:
//! `0xADE`, 215 until Sora comes near, then the drop 213; Re:CoM's battle spawns play the appearance 69, owner
//! 2026-11-02), KH1's effects (`sys 75`: 9, 10; M13), the appearance's actor flag 0x80 for 30 frames (`0x215A`),
//! `sys 163` ("drawn this frame", taken as true) and the attribute-8 paths (`wp+0x50` = 1: the lower leaps and the
//! hops skipped).

use crate::enemy::script::{face_target, frame, motion_frames, poll, pos, push, rand, rand_range, set_state, set_target, take_state, target, tick_clock, wait_frames};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Wight Knight's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The air motion at a timed jump's top (`sys 83(3)`: base 0, `0x1402AF630`).
    pub const APEX: u16 = 5;
    /// The appearance (`0xCA8`: a plain request), 112 frames.
    pub const APPEAR: u16 = 69;
    /// The combo's start (`0x1890`: `0x10000C8`, an action, 201 queued), 40 frames; its body row `0xEC` on [6, 40).
    pub const WINDUP: u16 = 200;
    /// The swipes (`0x17B0` / `0x1678`, actions, the other queued), 70 frames each, root motion; their hands
    /// [20, 60), their body row `0x15C` [0, 70).
    pub const SWIPE: u16 = 201;
    pub const BACK_SWIPE: u16 = 202;
    /// The combo's ends (`0x1544` / `0x1594`, actions, the idle queued): after 201, 203; after 202, 204.
    pub const SWIPE_END: u16 = 203;
    pub const BACK_SWIPE_END: u16 = 204;
    /// The leaps (`0x1284`): forward 205, back 206 (56 frames, root motion), 6 queued.
    pub const LEAP_FWD: u16 = 205;
    pub const LEAP_BACK: u16 = 206;
    /// The leap slash's crouch (`0x19FA`: `0x10000CF`, an action, 208 queued), 208 rising (bit 16), 209 at the
    /// top, 210 falling (`sys 83(207)`).
    pub const CROUCH: u16 = 207;
    /// The dance (`0x1090`), 240 frames.
    pub const DANCE: u16 = 212;
    /// The leap slash on landing (`0x1B34`), 96 frames, root motion; hands [2, 10) and [24, 34), body row `0x54C`
    /// [2, 34).
    pub const LEAP_SLASH: u16 = 216;
    /// The recoil of every row (`+0x40`), 52 frames, root motion.
    pub const RECOIL: u16 = 217;
    /// The sway (`0x10C0`), 240 frames.
    pub const SWAY: u16 = 218;
}

/// The Wight Knight's attacks, in the order the host indexes them (the manifest's `attacks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    Swipe,
    BackSwipe,
    LeapSlash,
}

impl Attack {
    pub const ALL: [Attack; 3] = [Attack::Swipe, Attack::BackSwipe, Attack::LeapSlash];

    pub fn request(self) -> u16 {
        match self {
            Attack::Swipe => req::SWIPE,
            Attack::BackSwipe => req::BACK_SWIPE,
            Attack::LeapSlash => req::LEAP_SLASH,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Swipe => "swipe",
            Attack::BackSwipe => "back swipe",
            Attack::LeapSlash => "leap slash",
        }
    }

    /// The damaging attack rows (script offsets, 0x70 bytes, `sys 23`) made with it. The body rows are [`Guard`]s.
    pub fn rows(self) -> &'static [usize] {
        match self {
            Attack::Swipe => &[0x1CC, 0x23C],
            Attack::BackSwipe => &[0x2AC, 0x31C],
            Attack::LeapSlash => &[0x38C, 0x3FC, 0x46C, 0x4DC],
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The Wight Knight's guard-type rows (`crate::enemy::guard`), in the order of the manifest's `guards`. All three
/// are on its body (bone code 8, 50) all round (cone cos −1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Guard {
    /// The combo's start `0xEC` (type 1, priority 100, recoil 110; HIT slot 0, on [6, 40) of 200): a side at 100 is
    /// never moved; Sora's swings get the strong recoil.
    Windup,
    /// The swipes' `0x15C` (type 2, priority 20, recoil 217; HIT slot 2, on through 201 / 202).
    Swipe,
    /// The leap slash's `0x54C` (type 2, priority 20, recoil 217; HIT slot 1, on [2, 34) of 216).
    Leap,
}

impl Guard {
    pub const ALL: [Guard; 3] = [Guard::Windup, Guard::Swipe, Guard::Leap];

    pub fn name(self) -> &'static str {
        match self {
            Guard::Windup => "windup",
            Guard::Swipe => "swipe",
            Guard::Leap => "leap",
        }
    }

    /// The row's script offset.
    pub fn row(self) -> usize {
        match self {
            Guard::Windup => 0xEC,
            Guard::Swipe => 0x15C,
            Guard::Leap => 0x54C,
        }
    }

    /// The requests the script makes it with (`0x187A` before 200; `0x176A` / `0x1632` before 201 / 202; `0x1B2E`
    /// before 216).
    pub fn requests(self) -> &'static [u16] {
        match self {
            Guard::Windup => &[req::WINDUP],
            Guard::Swipe => &[req::SWIPE, req::BACK_SWIPE],
            Guard::Leap => &[req::LEAP_SLASH],
        }
    }

    pub fn index(self) -> u8 {
        Guard::ALL.iter().position(|&g| g == self).unwrap_or(0) as u8
    }
}

/// The names of the Wight Knight's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    "first_combo_chance",
    "first_leap_chance",
    "close_range",
    "far_hop_chance",
    "far_circle_rate",
    "close_back_chance",
    "close_far",
    "close_far_leap",
    "close_far_sway",
    "close_far_circle_rate",
    "close_near_leap",
    "close_near_sway",
    "hit_far",
    "hit_far_leap",
    "hit_leap_chance",
    "hit_fwd_chance",
    "hit_combo_range",
    "after_hop_range",
    "after_hop_leap",
    "appear_cut",
    "appear_fade",
    "appear_blend",
    "appear_queue_blend",
    "idle_blend",
    "idle_wait",
    "circle_min",
    "circle_max",
    "circle_side_chance",
    "circle_right",
    "circle_left",
    "circle_blend",
    "circle_speed",
    "circle_stuck",
    "circle_near",
    "circle_min_time",
    "dance_time",
    "sway_time",
    "dance_blend",
    "dance_near",
    "dance_min_time",
    "hop_blend",
    "hop_queue_blend",
    "hop_lift_at",
    "far_hop_lift_at",
    "combo_blend",
    "swipe_blend",
    "combo_end_blend",
    "combo_far",
    "combo_max",
    "leap_far",
    "leap_height",
    "leap_time",
    "leap_speed_k",
    "leap_drag",
    "leap_crouch_blend",
    "leap_rise_blend",
    "leap_cut_a",
    "leap_cut_b",
    "leap_slash_blend",
    "stuck_hop_blend",
    "stuck_hop_height",
    "stuck_hop_speed",
    "stuck_hop_drag",
    "stuck_hop_time",
];

/// The Wight Knight's numbers.
pub type Consts = Table;

/// The Wight Knight's state objects (`wp+0x20`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const ENGAGED: u8 = 1;
    pub const HIT: u8 = 2;
}

/// The Wight Knight's own work area: the swipe it played last (`wp+0x40`: 1 after 201, 0 after 202), Sora beyond the
/// combo's reach after a swipe (`wp+0x4C`), the swipes this combo (`wp+0x54`), a swipe's row landed (`wp+0x48`, the
/// rows' callback `0x1D0C`), the engaged state's first round done (`wp+0x60`); spawned by the room (`sys 132`), the
/// appearance done, the killing hit under way (the handler's death path).
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    after_swipe: bool,
    far: bool,
    swipes: i32,
    landed: bool,
    first_done: bool,
    spawned: bool,
    appeared: bool,
    dying: bool,
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

/// `while (+0x70 != 0) yield`: until the body is on the ground (KH1's movement state 0).
async fn on_ground(cx: &Cx) {
    while !grounded(cx) {
        frame().await;
    }
}

/// A plain motion request (`sys 12` / `sys 13` without bit 24).
fn play(cx: &Cx, r: u16, blend: f32) {
    {
        let mut io = cx.borrow_mut();
        if io.w.motion != Some(r) {
            io.clock = (Some(r), 0.0);
        }
    }
    push(cx, ECmd::Motion { request: r, blend });
}

/// `sys 23` of a guard-type row with the request it lives on.
fn guard(cx: &Cx, g: Guard, request: u16) {
    push(cx, ECmd::Guard(Some((g.index(), request))));
}

/// `sys 10(v, deg)` on a direction: turned about the vertical (the Air Soldier's, the Darkball's and the Air
/// Pirate's convention: the yaw + deg; Assumption, as theirs; the circling walk's two turns use it alike).
fn rotate(v: Vec3, deg: f32) -> Vec3 {
    let l = v.horizontal_length();
    Vec3::from_yaw(v.yaw() + deg.to_radians()) * l + Vec3::new(0.0, v.y, 0.0)
}

/// `0x1F48(actor, target)`: the 3D distance to the target (none: a large value).
fn dist(cx: &Cx) -> f32 {
    crate::enemy::script::dist_to_target(cx)
}

/// `0x2112(actor, target)`: face the target now (`sys 11`).
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

/// `0x2870`: the idle at blend 8, then 6 frames (`0x28BE`).
async fn idle_wait(cx: &Cx) {
    play(cx, req::IDLE, k(cx, "idle_blend"));
    wait_frames(cx, k(cx, "idle_wait")).await;
}

/// The rest of the behaviour thread after a state request (`misc8`): the supervisor replaces it at its next run.
async fn halt() {
    loop {
        frame().await;
    }
}

/// `0x1284(back, facing)`: a leap (`wp+0x50` = 0 only). On the ground; 205 forward (`back` = 0) or 206 back at blend 4
/// with 6 queued, turning to the target (`sys 36`); for the motion's length; from frame 20 movement state 7 with no
/// velocity (`sys 169`: the root motion carries it, it counts as airborne, nothing pulls it down); then movement
/// state 0 (`sys 88`), a yield, and until on the ground (turning to the target when `facing`).
async fn leap(cx: &Cx, back: bool, facing: bool) {
    on_ground(cx).await;
    let r = if back { req::LEAP_BACK } else { req::LEAP_FWD };
    hop_motion(cx, r, k(cx, "hop_lift_at"), facing).await;
}

/// The leap's body (`0x1284` / `0x1BAA`): the motion `r` with its lift at frame `lift_at`.
async fn hop_motion(cx: &Cx, r: u16, lift_at: f32, facing: bool) {
    let mut left = motion_frames(cx, r);
    let mut t = 0.0;
    let mut lifted = false;
    face_target(cx);
    play(cx, r, k(cx, "hop_blend"));
    push(cx, ECmd::Queue { request: req::AIR, blend: k(cx, "hop_queue_blend") });
    while left - 0.0 > 0.0 {
        let s = step(cx);
        left -= s;
        t += s;
        frame().await;
        if t - lift_at >= 0.0 && !lifted {
            lifted = true;
            cx.borrow_mut().body.air_fly(Vec3::ZERO);
        }
    }
    cx.borrow_mut().body.ground();
    frame().await;
    while !grounded(cx) {
        if facing {
            face_target(cx);
        }
        frame().await;
    }
}

/// `0x266C`: the stuck hop. On the ground; the jump start 3 (an action, blend 0, the idle queued); 4 (bit 16) and
/// movement state 3 (`sys 82`: the facing × 20, height 400, drag 0.9, 50 frames; `sys 83(3)`: six frames before the
/// top 5 plays with 6 queued); until on the ground.
async fn stuck_hop(cx: &Cx) {
    on_ground(cx).await;
    let yaw = cx.borrow().body.yaw;
    let d = Vec3::from_yaw(yaw) * k(cx, "stuck_hop_speed");
    let v = Vec3::new(d.x, k(cx, "stuck_hop_height"), d.z);
    let b = k(cx, "stuck_hop_blend");
    push(cx, ECmd::Action { request: req::JUMP, attack: None, then: req::IDLE, blend: b });
    frame().await;
    play(cx, req::FALL, b);
    let (drag, time) = (k(cx, "stuck_hop_drag"), k(cx, "stuck_hop_time"));
    cx.borrow_mut().body.timed_jump(v, drag, time);
    timed_air(cx, req::JUMP, b, false).await;
}

/// A timed jump in the air (`sys 83(base + 3)`, `0x1402AF630`): six frames before its top, while `base + 1` plays,
/// `base + 2` plays with `base + 3` queued; facing the target now each frame when `face` (`0x2112`, the leap
/// slash); until on the ground.
async fn timed_air(cx: &Cx, base: u16, blend: f32, face: bool) {
    let mut apex = false;
    while cx.borrow().body.timed.is_some() {
        // `0x1402AF630` runs in the engine's step (after the script): here at the next frame's script (Assumption:
        // ±1 frame, as the Powerwild's and the Pirate's).
        let (at_top, cur) = {
            let io = cx.borrow();
            (io.body.timed_apex(), io.w.motion)
        };
        if !apex && at_top && cur == Some(base + 1) {
            apex = true;
            play(cx, base + 2, blend);
            push(cx, ECmd::Queue { request: base + 3, blend });
        }
        if face {
            face_now(cx);
        }
        frame().await;
    }
    while !grounded(cx) {
        if face {
            face_now(cx);
        }
        frame().await;
    }
}

/// `0xD96(rate)`: the circling walk. On the ground; [100, 200) frames; Sora's way (3D, normalised) turned −90° (60 %)
/// or +90° (the rate negated); each frame the walk (request 1, blend 16), the way turned by `rate` × step, the target
/// yaw along it (`sys 36`), the walk velocity along it at 5 (`+0x20` / `+0x2C`); after the yield: moved under 2.5 a
/// frame: the idle (`0x2870`) and the stuck hop; off the ground: the end; within 100 of Sora after 50 frames: the
/// idle and the back leap. Unless stopped, the idle at the end.
async fn circle(cx: &Cx, rate: f32) {
    on_ground(cx).await;
    let mut left = rand_range(cx, k(cx, "circle_min"), k(cx, "circle_max"), "circle time");
    let mut t = 0.0;
    let mut stopped = false;
    let mut rate = rate;
    let mut dir = target(cx).map_or(Vec3::from_yaw(cx.borrow().body.yaw), |s| (s - pos(cx)).normalized());
    if rand(cx, "circle side") - k(cx, "circle_side_chance") < 0.0 {
        dir = rotate(dir, k(cx, "circle_right"));
    } else {
        dir = rotate(dir, k(cx, "circle_left"));
        rate *= -1.0;
    }
    let speed = k(cx, "circle_speed");
    while left - 0.0 > 0.0 {
        play(cx, req::WALK, k(cx, "circle_blend"));
        let s = step(cx);
        dir = rotate(dir, rate * s);
        {
            let mut io = cx.borrow_mut();
            io.body.yaw_target = dir.yaw();
            io.body.walk = dir * speed;
        }
        left -= s;
        t += s;
        frame().await;
        let (moved, s) = {
            let io = cx.borrow();
            (io.w.moved, io.w.step)
        };
        if moved - k(cx, "circle_stuck") * s < 0.0 {
            stopped = true;
            idle_wait(cx).await;
            stuck_hop(cx).await;
            break;
        }
        if !grounded(cx) {
            break;
        }
        if dist(cx) - k(cx, "circle_near") < 0.0 && t - k(cx, "circle_min_time") > 0.0 {
            stopped = true;
            idle_wait(cx).await;
            leap(cx, true, false).await;
            break;
        }
    }
    if !stopped {
        idle_wait(cx).await;
    }
}

/// `0x103C(dance)`: on the ground; the dance 212 for 200 frames (`dance`) or the sway 218 for 150 frames turning to
/// the target (blend 16); off the ground: the end; another motion playing: the idle (`0x2870`), the end; within 200 of
/// Sora after 80 frames: the idle and the back leap facing him. Unless stopped, the idle; then the leap slash.
async fn dance(cx: &Cx, dance: bool) {
    on_ground(cx).await;
    let mut t = 0.0;
    let mut stopped = false;
    let (r, mut left) = if dance { (req::DANCE, k(cx, "dance_time")) } else { (req::SWAY, k(cx, "sway_time")) };
    play(cx, r, k(cx, "dance_blend"));
    while left - 0.0 > 0.0 {
        let s = step(cx);
        left -= s;
        t += s;
        frame().await;
        if !grounded(cx) {
            break;
        }
        if !dance {
            face_target(cx);
        }
        if cx.borrow().w.motion != Some(r) {
            stopped = true;
            idle_wait(cx).await;
            break;
        }
        if dist(cx) - k(cx, "dance_near") < 0.0 && t - k(cx, "dance_min_time") > 0.0 {
            stopped = true;
            idle_wait(cx).await;
            leap(cx, true, true).await;
            break;
        }
    }
    if !stopped {
        idle_wait(cx).await;
    }
    leap_slash(cx).await;
}

/// `0x1BAA`: the far leap before the leap slash: 205 as the leaps, its lift from frame 22, turning to the target until
/// on the ground.
async fn far_leap(cx: &Cx) {
    on_ground(cx).await;
    hop_motion(cx, req::LEAP_FWD, k(cx, "far_hop_lift_at"), true).await;
}

/// `0x18CA`: the leap slash. On the ground; beyond 600 the far leap; Sora's way (3D) and distance; turning to him, the
/// crouch 207 (an action, blend 0, 208 queued); 208 (bit 16) and movement state 3 (`sys 82`: the way × distance ×
/// 0.035 with height 500, drag 0.9, 30 frames; `sys 83(207)`: 209 at the top, 210 queued), facing him now each frame
/// in the air; on the ground, facing him now, its rows (two hands at two strikes) and its body row `0x54C`, 216 (blend
/// 0, a plain request: the script runs on) with the idle queued, for its length − 2 − 8 frames or until off the
/// ground.
async fn leap_slash(cx: &Cx) {
    on_ground(cx).await;
    if dist(cx) - k(cx, "leap_far") > 0.0 {
        far_leap(cx).await;
    }
    // `sys 163` (drawn this frame): taken as true (Assumption).
    let Some(t) = target(cx) else { return };
    let to = t - pos(cx);
    let (dir, d) = (to.normalized(), to.length());
    let f = d * k(cx, "leap_speed_k");
    let v = Vec3::new(dir.x * f, k(cx, "leap_height"), dir.z * f);
    face_target(cx);
    push(cx, ECmd::Action { request: req::CROUCH, attack: None, then: req::CROUCH + 1, blend: k(cx, "leap_crouch_blend") });
    frame().await;
    let b = k(cx, "leap_rise_blend");
    play(cx, req::CROUCH + 1, b);
    let (drag, time) = (k(cx, "leap_drag"), k(cx, "leap_time"));
    cx.borrow_mut().body.timed_jump(v, drag, time);
    timed_air(cx, req::CROUCH, b, true).await;
    let mut left = motion_frames(cx, req::LEAP_SLASH) - k(cx, "leap_cut_a") - k(cx, "leap_cut_b");
    face_now(cx);
    cx.borrow_mut().clock = (Some(req::LEAP_SLASH), 0.0);
    push(cx, ECmd::Attack { request: req::LEAP_SLASH, attack: Attack::LeapSlash, blend: k(cx, "leap_slash_blend") });
    guard(cx, Guard::Leap, req::LEAP_SLASH);
    push(cx, ECmd::Queue { request: req::IDLE, blend: k(cx, "leap_slash_blend") });
    while left - 0.0 > 0.0 {
        left -= step(cx);
        frame().await;
        if !grounded(cx) {
            break;
        }
    }
}

/// `0x1834`: the combo's start. On the ground; `wp+0x40` / `+0x54` / `+0x48` = 0; its body row `0xEC` and the action
/// 200 (blend 4, 201 queued).
async fn windup(cx: &Cx) {
    on_ground(cx).await;
    {
        let mut io = cx.borrow_mut();
        io.x.after_swipe = false;
        io.x.swipes = 0;
        io.x.landed = false;
    }
    push(cx, ECmd::Action { request: req::WINDUP, attack: None, then: req::SWIPE, blend: k(cx, "combo_blend") });
    guard(cx, Guard::Windup, req::WINDUP);
    frame().await;
}

/// `0x16FC` (201, `back` false) / `0x15C4` (202): a swipe. On the ground; `wp+0x40` = 1 / 0, `wp+0x4C` = 0, one more
/// swipe, `wp+0x48` = 0; turning to the target; its body row `0x15C` and its hands; the action (blend 0, the other
/// swipe queued); then a landed row ends the combo (`wp+0x54` = 4) and Sora beyond 350 marks `wp+0x4C`.
async fn swipe(cx: &Cx, back: bool) {
    on_ground(cx).await;
    {
        let mut io = cx.borrow_mut();
        io.x.after_swipe = !back;
        io.x.far = false;
        io.x.swipes += 1;
        io.x.landed = false;
    }
    face_target(cx);
    let (a, then) = if back { (Attack::BackSwipe, req::SWIPE) } else { (Attack::Swipe, req::BACK_SWIPE) };
    push(cx, ECmd::Action { request: a.request(), attack: Some(a), then, blend: k(cx, "swipe_blend") });
    guard(cx, Guard::Swipe, a.request());
    frame().await;
    if cx.borrow().x.landed {
        let n = k(cx, "combo_max") as i32;
        cx.borrow_mut().x.swipes = n;
    }
    if dist(cx) - k(cx, "combo_far") > 0.0 {
        cx.borrow_mut().x.far = true;
    }
}

/// `0x146E`: the combo. On the ground; the start (`0x1834`); swipes by turns (201 first) until Sora is beyond 350 or
/// four swipes (a landed one counts as the last), a yield between; the end that follows the last swipe (203 after
/// 201, 204 after 202: actions, the idle queued); Sora beyond 350: the leap slash.
async fn combo(cx: &Cx) {
    on_ground(cx).await;
    windup(cx).await;
    loop {
        let back = cx.borrow().x.after_swipe;
        swipe(cx, back).await;
        let (far, n) = {
            let io = cx.borrow();
            (io.x.far, io.x.swipes)
        };
        if far || n as f32 - k(cx, "combo_max") >= 0.0 {
            break;
        }
        frame().await;
    }
    on_ground(cx).await;
    let end = if cx.borrow().x.after_swipe { req::SWIPE_END } else { req::BACK_SWIPE_END };
    push(cx, ECmd::Action { request: end, attack: None, then: req::IDLE, blend: k(cx, "combo_end_blend") });
    frame().await;
    if cx.borrow().x.far {
        leap_slash(cx).await;
    }
}

/// `0xD34` (after the far forward leap): beyond 250, above one half the leap slash, else the dance; within, the combo.
async fn after_hop(cx: &Cx) {
    if dist(cx) - k(cx, "after_hop_range") > 0.0 {
        if rand(cx, "after hop") - k(cx, "after_hop_leap") > 0.0 {
            leap_slash(cx).await;
        } else {
            dance(cx, true).await;
        }
    } else {
        combo(cx).await;
    }
}

/// `0x910`: far: above 80 % the forward leap and `0xD34`; else the circling walk (0.6° a frame) and the dance.
async fn far(cx: &Cx) {
    if rand(cx, "far") - k(cx, "far_hop_chance") > 0.0 {
        leap(cx, false, false).await;
        after_hop(cx).await;
    } else {
        circle(cx, k(cx, "far_circle_rate")).await;
        dance(cx, true).await;
    }
}

/// `0x95A`: close: above one half the back leap, then beyond 400 (under 30 % the leap slash, above one half the sway,
/// else the circling walk (0.5° a frame) and the forward leap), within (under 40 % the leap slash, above 80 % the
/// sway, else the combo); else the combo and the back leap.
async fn close(cx: &Cx) {
    if rand(cx, "close") - k(cx, "close_back_chance") > 0.0 {
        leap(cx, true, false).await;
        let d = dist(cx);
        let r = rand(cx, "close after leap");
        if d - k(cx, "close_far") > 0.0 {
            if r - k(cx, "close_far_leap") < 0.0 {
                leap_slash(cx).await;
            } else if r - k(cx, "close_far_sway") > 0.0 {
                dance(cx, false).await;
            } else {
                circle(cx, k(cx, "close_far_circle_rate")).await;
                leap(cx, false, false).await;
            }
        } else if r - k(cx, "close_near_leap") < 0.0 {
            leap_slash(cx).await;
        } else if r - k(cx, "close_near_sway") > 0.0 {
            dance(cx, false).await;
        } else {
            combo(cx).await;
        }
    } else {
        combo(cx).await;
        leap(cx, true, false).await;
    }
}

/// The appearance (`0xA90` → `0xC46`, spawned by the room): 69 for its length − 4 frames (a plain request, blend 0,
/// the idle queued at 0) turning to Sora, after the 30-frame fade-in and facing him now; off the ground it ends; then
/// the idle (`0x2870`); then the engaged state. A room-placed one (spawn type 0) engages at once.
async fn appear(cx: Cx) {
    if cx.borrow().x.spawned {
        let mut left = motion_frames(&cx, req::APPEAR) - k(&cx, "appear_cut");
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        push(&cx, ECmd::Note("the appearance's effect 10 / 9 (M13) and actor flag 0x80 for 30 frames"));
        face_now(&cx);
        play(&cx, req::APPEAR, k(&cx, "appear_blend"));
        push(&cx, ECmd::Queue { request: req::IDLE, blend: k(&cx, "appear_queue_blend") });
        while left - 0.0 > 0.0 {
            if !grounded(&cx) {
                break;
            }
            left -= step(&cx);
            face_target(&cx);
            frame().await;
        }
        idle_wait(&cx).await;
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// The engaged behaviour (`0x7AA` → `0x7E2`): the first time 20 % the combo, 50 % the leap slash, else the forward
/// leap and the leap slash; then forever: close (< 300) or far, a yield (the leash in KH1's world 11 and the engaged
/// search `0x1D2E(&wp30, 1, 5000, 3)`: Sora, Re:CoM's battles have only him).
async fn engaged(cx: Cx) {
    set_target(&cx, true);
    if !cx.borrow().x.first_done {
        cx.borrow_mut().x.first_done = true;
        let r = rand(&cx, "first");
        if r - k(&cx, "first_combo_chance") > 0.0 {
            combo(&cx).await;
        } else if r - k(&cx, "first_leap_chance") < 0.0 {
            leap_slash(&cx).await;
        } else {
            leap(&cx, false, false).await;
            leap_slash(&cx).await;
        }
    }
    loop {
        if dist(&cx) - k(&cx, "close_range") < 0.0 {
            close(&cx).await;
        } else {
            far(&cx).await;
        }
        frame().await;
    }
}

/// The hit behaviour (`0x618` → `0x664`; the hitter as target: Sora): beyond 400, under 80 % the leap slash, else the
/// forward leap; within, above 70 % the leap slash, else a leap (above one half forward, else back) and, beyond 300,
/// the leap slash, else the combo. Then engaged.
async fn hit(cx: Cx) {
    set_target(&cx, true);
    if dist(&cx) - k(&cx, "hit_far") > 0.0 {
        if rand(&cx, "hit far") - k(&cx, "hit_far_leap") < 0.0 {
            leap_slash(&cx).await;
        } else {
            leap(&cx, false, false).await;
        }
    } else if rand(&cx, "hit") - k(&cx, "hit_leap_chance") > 0.0 {
        leap_slash(&cx).await;
    } else {
        let back = rand(&cx, "hit leap") - k(&cx, "hit_fwd_chance") <= 0.0;
        leap(&cx, back, false).await;
        if dist(&cx) - k(&cx, "hit_combo_range") > 0.0 {
            leap_slash(&cx).await;
        } else {
            combo(&cx).await;
        }
    }
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// The supervisor (`0x22F4`): after the init frame, each frame a state request restarts the behaviour thread (the
/// engaged and hit states set the target: Sora, the only party member and attacker).
async fn supervisor(cx: Cx) {
    frame().await;
    loop {
        if let Some(s) = take_state(&cx) {
            set_target(&cx, s != st::NONE);
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Wight Knight's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as every port's: per-enemy seeds). `frames`: KH1 motion
    /// lengths by request. `spawned`: the room spawned it (its appearance plays; every Re:CoM battle appearance,
    /// owner 2026-11-02).
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
                // `0x23FE`: home (`wp+0x10`, unused); the state object cleared (`0x1DF6`).
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
                st::HIT => Box::pin(hit(cx)),
                _ => Box::pin(engaged(cx)),
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
    /// The hit handler (`0x24D2`): a clash or a living damaging hit asks for the hit state (`0x24F6`, `0x263C`). A
    /// repel record (`hp[0] & 0x20`) asks for nothing (`0x1E38` answers −1, attribute 0: `0x262C`); none comes (its
    /// rows never repel: type 2, and its type-1 row is at 100).
    fn on_hit(&self, news: HitNews) {
        if news != HitNews::Repelled {
            set_state(&self.io, st::HIT);
        }
    }
    /// The killing hit (`0x1E38` answers 0): the handler's death path follows (`0x2644`: Re:CoM's defeat here); a kind 3
    /// becomes kind 4 first (`0x256E` → `0x222A`).
    fn on_defeat(&self) {
        self.io.borrow_mut().x.dying = true;
    }
    /// `0x256E`: the killing hit's kind 3 (a flinch) becomes 4 (a knock-back); living hits keep their kind.
    fn reaction_kind(&self, kind: u8) -> u8 {
        if self.io.borrow().x.dying && kind == 3 {
            4
        } else {
            kind
        }
    }
    fn risen(&self) -> bool {
        self.io.borrow().x.appeared
    }
    fn skip_rise(&self) {
        self.io.borrow_mut().x.spawned = false;
    }
    /// A swipe's row landed (`0x1D0C`, the rows `0x1CC` / `0x23C` / `0x2AC` / `0x31C`'s callback: `wp+0x48` = 1).
    fn on_landed(&self, attack: u8) {
        if attack == Attack::Swipe.index() || attack == Attack::BackSwipe.index() {
            self.io.borrow_mut().x.landed = true;
        }
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["none", "engaged", "hit"].get(s as usize).copied().unwrap_or("?")
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
        // The player's `ex_2200.bd` / `btltbl.bin` / KH1 exe numbers as the asset step reads them (its manifest on the
        // session's files), the exe's gravity and turn rate.
        let v: &[(&str, f32)] = &[
            ("walk_speed", 5.0), ("run_speed", 8.0), ("first_combo_chance", 0.8), ("first_leap_chance", 0.5), ("close_range", 300.0),
            ("far_hop_chance", 0.8), ("far_circle_rate", 0.6), ("close_back_chance", 0.5), ("close_far", 400.0), ("close_far_leap", 0.3),
            ("close_far_sway", 0.5), ("close_far_circle_rate", 0.5), ("close_near_leap", 0.4), ("close_near_sway", 0.8), ("hit_far", 400.0),
            ("hit_far_leap", 0.8), ("hit_leap_chance", 0.7), ("hit_fwd_chance", 0.5), ("hit_combo_range", 300.0), ("after_hop_range", 250.0),
            ("after_hop_leap", 0.5), ("appear_cut", 4.0), ("appear_fade", 30.0), ("appear_blend", 0.0), ("appear_queue_blend", 0.0),
            ("idle_blend", 8.0), ("idle_wait", 6.0), ("circle_min", 100.0), ("circle_max", 200.0), ("circle_side_chance", 0.6),
            ("circle_right", -90.0), ("circle_left", 90.0), ("circle_blend", 16.0), ("circle_speed", 5.0), ("circle_stuck", 2.5),
            ("circle_near", 100.0), ("circle_min_time", 50.0), ("dance_time", 200.0), ("sway_time", 150.0), ("dance_blend", 16.0),
            ("dance_near", 200.0), ("dance_min_time", 80.0), ("hop_blend", 4.0), ("hop_queue_blend", 0.0), ("hop_lift_at", 20.0),
            ("far_hop_lift_at", 22.0), ("combo_blend", 4.0), ("swipe_blend", 0.0), ("combo_end_blend", 0.0), ("combo_far", 350.0),
            ("combo_max", 4.0), ("leap_far", 600.0), ("leap_height", 500.0), ("leap_time", 30.0), ("leap_speed_k", 0.035), ("leap_drag", 0.9),
            ("leap_crouch_blend", 0.0), ("leap_rise_blend", 0.0), ("leap_cut_a", 2.0), ("leap_cut_b", 8.0), ("leap_slash_blend", 0.0),
            ("stuck_hop_blend", 0.0), ("stuck_hop_height", 400.0), ("stuck_hop_speed", 20.0), ("stuck_hop_drag", 0.9), ("stuck_hop_time", 50.0),
            ("attack_blend", 0.0), ("air_accel", 0.1), ("air_drag", 0.95), ("fall_cap", 20.0), ("down_time", 30.0), ("air_knock_hop", 20.0),
            ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.3499999), ("flinch_push", 80.0), ("air_flinch_speed", 4.0),
            ("air_knock_speed", 16.0), ("weight", 1.0),
        ];
        let mut v: Vec<(String, f32)> = v.iter().map(|(k, x)| (k.to_string(), *x)).collect();
        v.push(("gravity".into(), crate::params::GRAVITY));
        v.push(("turn_rate".into(), std::f32::consts::PI / 16.0));
        Table::new(v, NAMES).unwrap()
    }

    /// The player's `xa_ex_2200.mset` lengths by request (the manifest's).
    fn frames() -> Vec<(u16, f32)> {
        vec![
            (0, 430.0), (1, 80.0), (3, 16.0), (4, 26.0), (5, 24.0), (6, 40.0), (7, 8.0), (69, 112.0), (72, 36.0), (73, 36.0), (74, 36.0), (75, 36.0),
            (78, 36.0), (79, 36.0), (82, 4.0), (83, 60.0), (84, 22.0), (85, 80.0), (86, 62.0), (110, 52.0), (200, 40.0), (201, 70.0), (202, 70.0),
            (203, 80.0), (204, 80.0), (205, 56.0), (206, 56.0), (207, 16.0), (208, 26.0), (209, 24.0), (210, 40.0), (212, 240.0), (213, 170.0),
            (215, 180.0), (216, 96.0), (217, 52.0), (218, 240.0),
        ]
    }

    /// The asset step's looping clips (the pose rule).
    fn loops() -> Vec<u16> {
        vec![0, 1, 6, 83, 85, 210, 212, 215, 218]
    }

    /// The class map (`ground::wightknight::CLASS.attack_states`).
    const STATES: [i32; 3] = [3, 3, 2];

    /// A flat floor at y = 0, Sora at `sora`; an action holds the AI for its motion's length; the current request is
    /// the last one played.
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
                            ECmd::Motion { request, .. } | ECmd::Attack { request, .. } => self.current = Some(request),
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

    fn sim(seed: u32, spawned: bool) -> Sim {
        Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: Vec3::ZERO, busy: 0.0, current: None, moved: 0.0, log: Vec::new() }
    }

    /// A Wight Knight at the end of its appearance (the engaged state asked for, its first round not begun).
    fn ready(seed: u32) -> Sim {
        let mut s = sim(seed, true);
        for _ in 0..200 {
            if s.ai.risen() {
                break;
            }
            s.run(Vec3::new(0.0, 0.0, 5000.0), 1);
        }
        assert!(s.ai.risen());
        s.log.clear();
        s
    }

    /// A room-placed Wight Knight running `f` alone from its second frame (Sora its target, its first round done).
    fn with<F: std::future::Future<Output = ()> + 'static>(seed: u32, f: impl FnOnce(Cx) -> F) -> Sim {
        let mut s = sim(seed, false);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 1);
        s.log.clear();
        {
            let mut io = s.ai.io.borrow_mut();
            io.has_target = true;
            io.x.first_done = true;
            io.x.appeared = true;
            io.state = (st::HIT, false);
        }
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
        assert_eq!((consts().num("leap_height"), consts().num("circle_speed")), (500.0, 5.0));
    }

    /// `0xC46`: the fade-in, facing Sora, 69 (a plain request, blend 0) with the idle queued, for 108 frames, the idle
    /// for 6, then engaged (no idle state: it engages at once).
    #[test]
    fn spawned_appearance_then_engaged() {
        let mut s = sim(1, true);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 2);
        assert!(s.log.contains(&ECmd::FadeIn(30.0)), "{:?}", s.log);
        assert!(s.log.contains(&ECmd::Motion { request: req::APPEAR, blend: 0.0 }) && s.log.contains(&ECmd::Queue { request: req::IDLE, blend: 0.0 }));
        s.run(Vec3::new(0.0, 0.0, 5000.0), 100);
        assert!(!s.ai.risen());
        s.run(Vec3::new(0.0, 0.0, 5000.0), 20);
        assert!(s.ai.risen() && s.log.contains(&ECmd::State(st::ENGAGED)));
        assert!(s.log.contains(&ECmd::Motion { request: req::IDLE, blend: 8.0 }));
        // A room-placed one engages at once.
        let mut s = sim(1, false);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 3);
        assert!(s.ai.risen() && s.log.contains(&ECmd::State(st::ENGAGED)) && !s.log.iter().any(|c| matches!(c, ECmd::FadeIn(_))));
    }

    /// The combo: 200 with its body row (priority 100), then swipes 201 / 202 by turns with the swipe body row, four at
    /// most, then the end that follows the last (203 after 201, 204 after 202).
    #[test]
    fn the_combo_alternates_and_ends() {
        let mut s = with(2, |cx| async move { combo(&cx).await });
        s.run(Vec3::new(0.0, 0.0, 150.0), 600);
        let m = motions(&s.log);
        assert_eq!(&m[..6], &[req::WINDUP, req::SWIPE, req::BACK_SWIPE, req::SWIPE, req::BACK_SWIPE, req::BACK_SWIPE_END], "{m:?}");
        assert!(s.log.contains(&ECmd::Guard(Some((Guard::Windup.index(), req::WINDUP)))));
        assert!(s.log.contains(&ECmd::Guard(Some((Guard::Swipe.index(), req::SWIPE)))) && s.log.contains(&ECmd::Guard(Some((Guard::Swipe.index(), req::BACK_SWIPE)))));
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Action { request: req::WINDUP, then: req::SWIPE, attack: None, blend } if *blend == 4.0)));
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Action { request: req::SWIPE, then: req::BACK_SWIPE, attack: Some(Attack::Swipe), .. })));
        // A landed swipe ends the combo after it: 200, 201 (landed), 203.
        let mut s = with(2, |cx| async move { combo(&cx).await });
        s.run(Vec3::new(0.0, 0.0, 150.0), 42);
        s.ai.on_landed(Attack::Swipe.index());
        s.run(Vec3::new(0.0, 0.0, 150.0), 300);
        let m = motions(&s.log);
        assert_eq!(&m[..3], &[req::WINDUP, req::SWIPE, req::SWIPE_END], "{m:?}");
        // Sora beyond 350 after a swipe: the end, then the leap slash.
        let mut s = with(2, |cx| async move { combo(&cx).await });
        s.run(Vec3::new(0.0, 0.0, 150.0), 42);
        s.run(Vec3::new(0.0, 0.0, 500.0), 400);
        let m = motions(&s.log);
        assert_eq!(&m[..4], &[req::WINDUP, req::SWIPE, req::SWIPE_END, req::CROUCH], "{m:?}");
    }

    /// The leap slash: the crouch 207, 208 and a 30-frame timed jump 500 high toward Sora at distance × 0.035 (drag 0.9),
    /// 209 at the top with 210 queued; on landing 216 with its rows and its body row, then the idle queued; the script
    /// waits 86 frames.
    #[test]
    fn the_leap_slash() {
        let mut s = with(3, |cx| async move { leap_slash(&cx).await });
        let sora = Vec3::new(0.0, 0.0, 500.0);
        let mut top = 0.0f32;
        for _ in 0..66 {
            s.run(sora, 1);
            top = top.min(s.pos.y);
        }
        let m = motions(&s.log);
        assert_eq!(&m[..3], &[req::CROUCH, req::CROUCH + 1, req::CROUCH + 2], "{m:?}");
        assert!(s.log.contains(&ECmd::Queue { request: req::CROUCH + 3, blend: 0.0 }));
        assert!((top + 500.0).abs() < 50.0, "500 high: {top}");
        // 500 × 0.035 a frame, × √0.9 each frame, for the 43 frames in the air (the rise's 15 and the fall at KH1's
        // cap of 20 a frame, `0x1402AFAE0`).
        let k = 0.9f32.sqrt();
        let travel = 500.0 * 0.035 * (1.0 - k.powi(43)) / (1.0 - k);
        assert!((s.pos.z - travel).abs() < 5.0, "{} vs {travel}", s.pos.z);
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Attack { request: req::LEAP_SLASH, attack: Attack::LeapSlash, .. })), "{m:?}");
        assert!(s.log.contains(&ECmd::Guard(Some((Guard::Leap.index(), req::LEAP_SLASH)))));
        let n = s.log.len();
        s.run(sora, 80);
        assert!(!s.log[n..].iter().any(|c| matches!(c, ECmd::Motion { .. } | ECmd::Action { .. })), "it waits 86 frames: {:?}", &s.log[n..]);
        // Beyond 600: the far leap 205 first.
        let mut s = with(3, |cx| async move { leap_slash(&cx).await });
        s.run(Vec3::new(0.0, 0.0, 900.0), 4);
        assert_eq!(motions(&s.log).first(), Some(&req::LEAP_FWD));
    }

    /// The leaps: 205 / 206 at blend 4 with 6 queued, movement state 7 (airborne, no fall) from frame 20 to the motion's
    /// end, then the ground.
    #[test]
    fn the_leaps_lift_at_frame_20() {
        for back in [false, true] {
            let mut s = with(4, move |cx| async move { leap(&cx, back, false).await });
            s.run(Vec3::new(0.0, 0.0, 200.0), 2);
            let r = if back { req::LEAP_BACK } else { req::LEAP_FWD };
            assert!(s.log.contains(&ECmd::Motion { request: r, blend: 4.0 }) && s.log.contains(&ECmd::Queue { request: req::AIR, blend: 0.0 }), "{:?}", s.log);
            s.run(Vec3::new(0.0, 0.0, 200.0), 18);
            assert!(!s.ai.body().air_flight);
            s.run(Vec3::new(0.0, 0.0, 200.0), 2);
            assert!(s.ai.body().air_flight && s.ai.body().airborne());
            s.run(Vec3::new(0.0, 0.0, 200.0), 40);
            assert!(!s.ai.body().airborne());
        }
    }

    /// The circling walk: request 1 at blend 16, walking at 5 a frame across Sora's way, the way turning 0.6° a frame;
    /// within 100 of him after 50 frames: the idle and the back leap.
    #[test]
    fn the_circling_walk() {
        let mut s = with(5, |cx| async move { circle(&cx, 0.6).await });
        let sora = Vec3::new(0.0, 0.0, 600.0);
        s.run(sora, 30);
        assert!(s.log.contains(&ECmd::Motion { request: req::WALK, blend: 16.0 }));
        assert!((s.moved - 5.0).abs() < 1e-3, "{}", s.moved);
        let d = (sora - s.pos).length();
        assert!((d - 600.0).abs() < 60.0, "it circles: {d}");
        // Sora steps in after 60 frames: the back leap.
        let mut s = with(5, |cx| async move { circle(&cx, 0.6).await });
        s.run(Vec3::new(0.0, 0.0, 600.0), 60);
        let p = s.pos;
        s.run(p + Vec3::new(0.0, 0.0, 50.0), 10);
        assert!(motions(&s.log).contains(&req::LEAP_BACK), "{:?}", motions(&s.log));
    }

    /// The dance 212 for 200 frames (blend 16), then the idle and the leap slash; the sway 218 for 150; Sora close after
    /// 80 frames: the back leap facing him, then the leap slash.
    #[test]
    fn the_dance_and_the_sway() {
        let mut s = with(6, |cx| async move { dance(&cx, true).await });
        s.run(Vec3::new(0.0, 0.0, 900.0), 199);
        assert!(s.log.contains(&ECmd::Motion { request: req::DANCE, blend: 16.0 }) && !motions(&s.log).contains(&req::CROUCH));
        s.run(Vec3::new(0.0, 0.0, 900.0), 120);
        let m = motions(&s.log);
        assert!(m.contains(&req::IDLE) && m.contains(&req::LEAP_FWD), "then the leap slash (far leap first): {m:?}");
        let mut s = with(6, |cx| async move { dance(&cx, false).await });
        s.run(Vec3::new(0.0, 0.0, 500.0), 90);
        s.run(Vec3::new(0.0, 0.0, 150.0), 120);
        let m = motions(&s.log);
        assert_eq!(&m[..3], &[req::SWAY, req::IDLE, req::LEAP_BACK], "{m:?}");
        assert!(m.contains(&req::CROUCH), "{m:?}");
    }

    /// Close, far and the hit state: every attack shows up.
    #[test]
    fn every_attack_shows_up() {
        let mut seen = std::collections::HashSet::new();
        let mut moves = std::collections::HashSet::new();
        for seed in 0..40 {
            let d = [150.0, 350.0, 900.0][seed as usize % 3];
            let mut s = ready(seed);
            s.run(Vec3::new(0.0, 0.0, d), 1500);
            seen.extend(attacks(&s.log));
            moves.extend(motions(&s.log));
        }
        for a in Attack::ALL {
            assert!(seen.contains(&a), "{a:?} never: {seen:?}");
        }
        for r in [req::WINDUP, req::LEAP_FWD, req::LEAP_BACK, req::DANCE, req::SWAY, req::WALK, req::CROUCH, req::BACK_SWIPE_END] {
            assert!(moves.contains(&r), "{r} never: {moves:?}");
        }
    }

    /// The hit handler: a damaging hit or a clash asks for the hit state; the killing hit's kind 3 becomes 4.
    #[test]
    fn the_hit_handler() {
        let s = ready(7);
        Behaviour::on_hit(&s.ai, HitNews::Damage);
        assert_eq!(s.ai.io.borrow().state, (st::HIT, true));
        s.ai.io.borrow_mut().state = (st::ENGAGED, false);
        Behaviour::on_hit(&s.ai, HitNews::Clash);
        assert_eq!(s.ai.io.borrow().state, (st::HIT, true));
        s.ai.io.borrow_mut().state = (st::ENGAGED, false);
        Behaviour::on_hit(&s.ai, HitNews::Repelled);
        assert_eq!(s.ai.io.borrow().state, (st::ENGAGED, false));
        assert_eq!((s.ai.reaction_kind(3), s.ai.reaction_kind(5)), (3, 5));
        s.ai.on_defeat();
        assert_eq!((s.ai.reaction_kind(3), s.ai.reaction_kind(5)), (4, 5));
        assert!(!s.ai.armoured(Some(req::SWIPE), 0, Some(3)), "no `sys 42`: every hit reacts");
    }

    /// The hit state picks a leap slash, a leap or the combo by distance.
    #[test]
    fn the_hit_state() {
        let (mut slash, mut combo_seen) = (false, false);
        // Seeds spread out (the stream's first draw follows a small seed closely).
        for seed in (0..30u32).map(|i| i.wrapping_mul(2_654_435_761)) {
            let mut s = ready(seed);
            s.ai.io.borrow_mut().x.first_done = true;
            Behaviour::on_hit(&s.ai, HitNews::Damage);
            s.run(Vec3::new(0.0, 0.0, 200.0), 200);
            let m = motions(&s.log);
            slash |= m.first() == Some(&req::CROUCH);
            combo_seen |= m.contains(&req::WINDUP);
            assert!(matches!(m.first(), Some(&(req::CROUCH | req::LEAP_FWD | req::LEAP_BACK))), "seed {seed}: {m:?}");
        }
        assert!(slash && combo_seen, "{slash} {combo_seen}");
    }

    /// A long fight: the AI acts at least every 600 frames (the host's quiet watchdog), with Sora circling 2–7 m away and
    /// a hit every 150–400 frames (12 seeds, 20 000 frames each).
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
                    Behaviour::on_hit(&s.ai, if r.next_f32() < 0.15 { HitNews::Clash } else { HitNews::Damage });
                    s.busy = 34.0;
                    next_hit = f + 150 + (r.next_f32() * 250.0) as usize;
                }
                let n = s.log.len();
                s.run(sora, 1);
                if s.log[n..].iter().any(|c| !matches!(c, ECmd::Yaw(_))) || s.busy > 0.0 || s.ai.body().walk != Vec3::ZERO || s.ai.body().airborne() {
                    worst = worst.max(f - last);
                    last = f;
                }
            }
            assert!(worst < 600, "seed {seed}: {worst} frames without acting");
        }
    }

    /// The Wight Knight through the host's rules (`crate::enemy::host::model`, what `kh1-recom`'s `win::kh1_enemy`
    /// decides) with its class map and the asset step's looping clips.
    fn hosted(s: Sim) -> Model<Consts> {
        let Sim { ai, pos, .. } = s;
        let mut m = Model::new(Box::new(ai), consts(), frames(), loops(), STATES.to_vec());
        m.pos = pos;
        m
    }

    /// The leap slash in the host: 216 (a plain request with its rows) asked for once and played in state 2 for the
    /// script's 86 frames, never released; the combo's swipes play in state 3.
    #[test]
    fn hosted_the_leap_slash_and_the_swipes() {
        let s = with(8, |cx| async move {
            leap_slash(&cx).await;
            play(&cx, req::IDLE, 8.0);
        });
        let mut m = hosted(s);
        m.run(Vec3::new(0.0, 0.0, 500.0), 300);
        assert_eq!(m.released, 0);
        assert_eq!(m.attacks_of(req::LEAP_SLASH).len(), 1);
        let run = m.longest_run(req::LEAP_SLASH);
        assert!((84..=88).contains(&run), "216 for 86 frames: {run}");
        assert!(m.trace.iter().all(|t| t.1 != Some(req::LEAP_SLASH) || t.0 == 2), "216 plays in state 2");
        let s = with(8, |cx| async move { combo(&cx).await });
        let mut m = hosted(s);
        m.run(Vec3::new(0.0, 0.0, 150.0), 500);
        assert_eq!(m.released, 0);
        assert!(m.trace.iter().all(|t| !matches!(t.1, Some(req::SWIPE | req::BACK_SWIPE)) || t.0 == 3 || t.0 == 1), "swipes in state 3 (the queued one a frame in state 1)");
        assert!(m.trace.iter().filter(|t| t.1 == Some(req::SWIPE) && t.0 == 3).count() >= 60);
    }

    /// A long fight in the host (12 seeds, 20 000 frames, Sora circling 2–7 m away): no attack is released, none is
    /// asked for again within two frames, it attacks.
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
            let issued: Vec<(usize, u16)> = m.log.iter().filter_map(|(f, c)| match c {
                ECmd::Attack { request, .. } => Some((*f, *request)),
                ECmd::Action { request, attack: Some(_), .. } => Some((*f, *request)),
                _ => None,
            }).collect();
            assert!(issued.len() > 10, "seed {seed}: it attacks ({})", issued.len());
            for w in issued.windows(2) {
                assert!(w[1].1 != w[0].1 || w[1].0 > w[0].0 + 2, "seed {seed}: {} asked again at {} after {}", w[0].1, w[1].0, w[0].0);
            }
        }
    }

    /// The rows and the guard rows (the shared enemy-guard step).
    #[test]
    fn guard_rows_and_attack_rows() {
        assert_eq!((Guard::Windup.row(), Guard::Swipe.row(), Guard::Leap.row()), (0xEC, 0x15C, 0x54C));
        assert_eq!(Guard::Swipe.requests(), &[201u16, 202][..]);
        assert_eq!(Attack::LeapSlash.rows(), &[0x38C, 0x3FC, 0x46C, 0x4DC]);
        assert_eq!((Attack::LeapSlash.index(), Attack::ALL.len()), (2, 3));
    }
}
