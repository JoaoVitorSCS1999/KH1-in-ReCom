//! KH1 FM's Search Ghost (`xa_ex_2230.mdls` part 5, `ex_2230.bd`, and its child object `ex_2230_00.bd`,
//! part 3), engine-agnostic: its AI script on the shared enemy framework (`crate::enemy`), flying (KH1's
//! movement state 4, `crate::enemy::Body::flying`). M11 (`docs/recom-m11-enemies.md` § The Search Ghost).
//! Re:CoM runs it natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`).
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2230.bd` / `ex_2230_00.bd`
//! at checked offsets). Script offsets are the player's `ex_2230.bd`'s unless marked "child" (Checked
//! (files): both scripts disassembled with `tools/kh1-ai-script.py`, raw and folded; the syscalls named are
//! Checked (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x253A`: init `0x26BC` (stats records LV1 `0x14` / LV2 `0x80`; the stats block's hop 800, unused;
//!   KH1's world: `wp+0x78` = world 11, `wp+0x74` = world 9 room 15 (never read); the work area; home `wp+0x10`;
//!   the state object `wp+0x20`; the search table; handlers: hit `0x285A`), flight with no velocity (`sys 135`
//!   with (0, 0, 0, 1)), a yield, the drain flags cleared, the start routine `0x10E6`; then each frame: nothing
//!   while it warps (`wp+0x54`); a state request restarts the behaviour (1 idle `0xCDE` with the target cleared,
//!   2 engaged `0x786` with the target or Sora, 3 hit `0x628` with the hitter or Sora); while engaged, a drain
//!   that reached it (`wp+0x68`) is held (`wp+0x6C`) for 568 frames.
//! - the start `0x10E6`: placed (`sys 132` = 0) outside world 11: the idle (drains always, `wp+0x7C` = 2);
//!   spawn type 6: face Sora, the appearance, the idle (drains always); any other spawn (Re:CoM's battles:
//!   Assumption, as the Darkball's): Sora found (5000) or Sora, face him, the appearance (outside world 11),
//!   engaged (drains only below half its HP, `wp+0x7C` = 1).
//! - the child object (`sys 94` at `0xD12`, `ex_2230_00.bd`): the cone. While the Ghost plays its idle (0) or
//!   its search (202), each frame: a party member whose lock point 0 (`sys 32`) is within 300 of the eye (bone
//!   45's parent 44, `sys 19`) and within 35° of the eye's axis (`sys 115`, its row 0) becomes its target
//!   (`sys 49`), and the child ends. The idle routines engage once a target is set. Its effect (`sys 109` 7
//!   on bone 45): the effects milestone (M13).
//! - the hit handler `0x285A`: the flight easing back to 0.2; attribute 9: damage only (not carried); a hit
//!   while it holds a drain (the killing hit too, [`Behaviour::on_defeat`]): the hold ends and KH1 gives an EXP
//!   bonus and 3 prizes (`sys 194(EXP / 10 + 1)`, the drop table `0x580`: [`ECmd::Reward`], owner 2026-11-02:
//!   on Re:CoM's EXP and prizes); the drain's orb is dropped; attribute 8: the same reward on every Gravity
//!   hit (not carried); every hit, clashes and repelled hits too, asks for the hit state 3.
//! - the hover after a drain (`0x2A50` mode 1): once the hold is off unhit (568 frames, or at once when the
//!   orb never counted: it started within 1 m) its HP goes back to full (`0x2ADE`: [`ECmd::HealFull`], on
//!   Re:CoM's HP).
//!
//! The work area: `wp+0x54` warping, `+0x58` drain landed (0, 1, 2, 3), `+0x5C` noticed, `+0x60` roam rounds,
//! `+0x64` the notice's effect live, `+0x68` drained, `+0x6C` holding it, `+0x70` noticed (the leash),
//! `+0x7C` 2 = drains always, `+0x84` the drained target, `+0x88` drain taken.
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash (`0x34EE` → `0x18F0`: removed 15 m / 12 m out
//! of sight from Sora when spawned; in a Re:CoM battle room it would never be defeated), the party retargeting
//! (Sora is the only target), the hits' KH1 attribute (`hp[0xC] >> 6 & 15`), the effects (`sys 75` / `sys 107`
//! / `sys 109`, the drain's orb `sys 15` / `sys 16`: its flight is counted, not drawn), the alpha fades
//! (`0x23B8` / `0x23E4`: [`ECmd::FadeIn`] for the log only), flag 0x80 at the appearance, the lock point mask
//! while it warps (`sys 116` / `117`: Re:CoM's lockable flag stays, the Darkball's rule), the death routine
//! (`0x2A14`: Re:CoM's defeat), `sys1.0` (a debug print).

use crate::enemy::script::{fly, frame, height, motion_frames, poll, pos, push, rand, rand_range, random_offset, set_air_accel, set_state, set_target, set_turn_rate, steer, take_state, target, tick_clock};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Sight, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Search Ghost's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The appearance (`0x2130`), 120 frames.
    pub const APPEAR: u16 = 69;
    /// The lunge it flies in on (`0x30C4` with 200: the drain's approach and the invisible one out of a hit),
    /// 40 frames looped.
    pub const LUNGE: u16 = 200;
    /// The vanish (`0x2FF2`), 120 frames looped.
    pub const VANISH: u16 = 201;
    /// The search (`0x1044`, `0x7EC`), 120 frames looped: the cone looks while it (or the idle) plays.
    pub const SEARCH: u16 = 202;
    /// The drain (`0x13BE`): a plain request with rows `0xEC` / `0x15C` (the two hands), 120 frames (looped by
    /// the asset step's pose rule; the script plays it its length), hit [34, 54).
    pub const DRAIN: u16 = 203;
    /// The burst (`0x128E`): a plain request with rows `0x1CC` / `0x23C` (hit [36, 48)) and `0x2AC` / `0x31C`
    /// (hit [108, 118)), 164 frames.
    pub const BURST: u16 = 204;
    /// The clash recoil both attacks' rows name (`+0x40` = 205), 82 frames, root.
    pub const RECOIL: u16 = 205;
    /// The notice (`0x1210`), an action (`0x10000CE`), 108 frames.
    pub const NOTICE: u16 = 206;
}

/// The Search Ghost's attacks, in the order the host indexes them (the manifest's `attacks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    /// `0x128E`: the burst, 204 with its damaging rows (`sys 23` mode 2: they live while 204 plays); the
    /// script keeps facing Sora while it plays.
    Burst,
    /// `0x13BE`: the drain, 203 with rows `0xEC` / `0x15C`, steered (it creeps in, or backs off when on him).
    Drain,
}

impl Attack {
    pub const ALL: [Attack; 2] = [Attack::Burst, Attack::Drain];

    pub fn request(self) -> u16 {
        match self {
            Attack::Burst => req::BURST,
            Attack::Drain => req::DRAIN,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Burst => "burst",
            Attack::Drain => "drain",
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The names of the Search Ghost's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    // Supervisor `0x253A`, start `0x10E6`, the search `0x1B8C`.
    "hold_time",
    "start_search",
    "world11_search",
    // Blends and the default rates.
    "motion_blend",
    "rest_blend",
    "turn_default",
    "ease_default",
    // Appearance `0x2130`.
    "appear_blend",
    "appear_fade",
    "appear_wait",
    "appear_then_blend",
    // Hit `0x628`, engaged `0x786`.
    "drain_hp",
    "hit_warp_chance",
    "lunge_speed",
    "lunge_near",
    "warp_lo",
    "warp_hi",
    "warp_speed",
    "hit_hover_speed",
    "hover_near",
    "hit_hover_time",
    // Roam `0x7EC`.
    "roam_lo",
    "roam_hi",
    "roam_search",
    "roam_close",
    "roam_wait_lo",
    "roam_wait_hi",
    "roam_near",
    "roam_lost_lo",
    "roam_lost_hi",
    "roam_lost_near",
    "roam_rounds",
    "roam_rest_lo",
    "roam_rest_hi",
    // The hold `0xA0E`, the drain's approach `0xA34`.
    "hold_hover_speed",
    "hold_hover_time",
    "drain_speed",
    "drain_near",
    // The attack round `0xB62`.
    "approach_speed",
    "approach_near",
    "away_lo",
    "away_hi",
    "away_speed",
    "away_arrive",
    "away_time",
    "watch_ease",
    "watch_turn_rate",
    "watch_time",
    "watch_near",
    // Idle `0xCDE`, its flight `0xDCE`, its search `0x1044`.
    "idle_turn_rate",
    "idle_far",
    "idle_near_lo",
    "idle_near_hi",
    "idle_lo",
    "idle_hi",
    "wander_speed",
    "wander_arrive",
    "search_lo",
    "search_hi",
    "wander_time",
    "low_height",
    "home_below",
    "sora_above",
    "rise_y",
    "sora_below",
    "sink_y",
    "wander_check",
    "stuck_factor",
    "wander_new_lo",
    "wander_new_hi",
    "search_blend",
    // Notice `0x1210`, burst `0x128E`.
    "notice_then_blend",
    "burst_turn_until",
    "burst_end_blend",
    // The drain `0x13BE` and its orb.
    "drain_steer_until",
    "drain_back_after",
    "drain_close",
    "drain_back_dir",
    "drain_back_speed",
    "drain_creep_speed",
    "orb_up",
    "orb_keep",
    "orb_near",
    "orb_done",
    "orb_last",
    "orb_drop",
    "orb_fade",
    // The hover `0x2A50`.
    "hover_turn_rate",
    "hover_rest",
    "hover_level",
    "hover_attack_turn_rate",
    "hover_bursts",
    "hover_back_chance",
    "hover_back",
    "hover_back_neg",
    "hover_far",
    "hover_turn_lo",
    "hover_turn_hi",
    "hover_side_lo",
    "hover_side_hi",
    "hover_side2_lo",
    "hover_side2_hi",
    "hover_check",
    "hover_stuck",
    "hover_stuck_timer",
    // The warp `0x2EBA`: vanish `0x2FF2`, reappear `0x2EEC`.
    "vanish_blend",
    "warp_arrive",
    "warp_time",
    "vanish_fade",
    "reappear_wait",
    "reappear_fade",
    // The approach `0x30C4`, the flight to a point `0x335A`.
    "approach_above",
    "approach_check",
    "approach_retry",
    "approach_detour_lo",
    "approach_detour_hi",
    "approach_detour_time",
    "fly_check",
    "fly_detour_lo",
    "fly_detour_hi",
    // The child's cone (`ex_2230_00.bd`) and its bone (the asset step's).
    "cone_range",
    "cone_angle",
    "sight_bone",
    // The hit handler's reward while it holds a drain (`0x28A6`): `sys 194(EXP / 10 + 1)` and the drop
    // table `0x580`'s seven prize counts (the asset step's).
    "reward_exp_div",
    "reward_exp_add",
    "reward_prize_0",
    "reward_prize_1",
    "reward_prize_2",
    "reward_prize_3",
    "reward_prize_4",
    "reward_prize_5",
    "reward_prize_6",
];

/// The Search Ghost's numbers.
pub type Consts = Table;

/// The state objects (`wp+0x20`): the supervisor's requests.
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 3;
}

/// The Search Ghost's own work area (the module notes), the start flags, the host's inputs (its HP ratio, the
/// cone's sight, the last hit's kind) and the child to make.
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    /// `wp+0x54`: a warp runs (the supervisor takes no request).
    warping: bool,
    /// `wp+0x58`: the drain landed (1), its orb set off (2), flying in (3).
    drain: u8,
    /// `wp+0x5C`: noticed (the notice plays once).
    noticed: bool,
    /// `wp+0x60`: the roam's rounds.
    roams: i32,
    /// `wp+0x68`: the orb reached it; `wp+0x6C`: the hold runs (the supervisor's 568 frames).
    drained: bool,
    holding: bool,
    /// `wp+0x7C` = 2: it drains whatever its HP (placed, or spawn type 6).
    drains_always: bool,
    /// `wp+0x88`: the drain's callback took its hit (once per drain).
    drain_taken: bool,
    /// Spawned by the room (`sys 132` ≠ 0): Re:CoM's battles (owner 2026-11-02: KH1's appearance plays).
    spawned: bool,
    /// KH1's appearance motion plays (false after Re:CoM's own appearance, its first-strike stun).
    appear_motion: bool,
    appeared: bool,
    /// The appearance's pause (actor flag 4 while 69 ends, `0x2190`): the whole script waits.
    paused: bool,
    /// The supervisor restarted the behaviour to the start routine (`0x2594`).
    start: bool,
    world11: bool,
    hp: f32,
    kind: u8,
    /// The idle made its child (`sys 94`, `0xD12`): [`Ai::frame`] starts it.
    child_new: bool,
    sight: Option<Sight>,
    /// The hit handler gave its reward (`0x28A6`): [`Behaviour::take_now`] hands it to the host.
    reward: bool,
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

/// `func_1DE4` (`sys 4(0)`): party 0, Sora.
fn sora(cx: &Cx) -> Vec3 {
    let io = cx.borrow();
    io.w.target.unwrap_or(io.w.pos)
}

/// The actor's target (`+0x74`): Sora once set (Re:CoM's battles have no other party member).
fn tgt(cx: &Cx) -> Vec3 {
    target(cx).unwrap_or_else(|| sora(cx))
}

fn has_target(cx: &Cx) -> bool {
    cx.borrow().has_target
}

/// `func_1F82` (`sys 6`, 3D): the distance from the actor to `p`.
fn dist_to(cx: &Cx, p: Vec3) -> f32 {
    (p - me(cx)).length()
}

/// `sys 9` (`0x1402A93D0`): the vector normalized; a zero vector becomes (0, 0, 1) (Checked (decomp)).
fn norm(v: Vec3) -> Vec3 {
    let l = v.length();
    if l != 0.0 {
        v * (1.0 / l)
    } else {
        Vec3::new(0.0, 0.0, 1.0)
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

/// `sys 10(v, deg)` on a direction: turned about the vertical (the Air Soldier's, the Darkball's, the Air
/// Pirate's and the Wyvern's convention: the yaw + deg; Assumption, as theirs).
fn rotate(v: Vec3, deg: f32) -> Vec3 {
    let l = v.horizontal_length();
    Vec3::from_yaw(v.yaw() + deg.to_radians()) * l + Vec3::new(0.0, v.y, 0.0)
}

/// The facing vector `+0x30`.
fn facing(cx: &Cx) -> Vec3 {
    Vec3::from_yaw(cx.borrow().body.yaw)
}

/// `*(actor + 0x2C) = 0`.
fn halt(cx: &Cx) {
    steer(cx, Vec3::ZERO, 0.0);
}

/// `func_222E` / `sys 36`: the target yaw toward `p` (the engine turns).
fn yaw_to(cx: &Cx, p: Vec3) {
    let d = p - me(cx);
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `sys 36` along a vector.
fn yaw_along(cx: &Cx, d: Vec3) {
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `func_2258` / `sys 11`: face `p` now (`+0x150` and the facing).
fn face_now(cx: &Cx, p: Vec3) {
    let d = p - me(cx);
    face_along(cx, d);
}

/// `sys 11` along a vector.
fn face_along(cx: &Cx, d: Vec3) {
    let y = d.x.atan2(d.z);
    let mut io = cx.borrow_mut();
    io.body.yaw = y;
    io.body.yaw_target = y;
    io.out.push(ECmd::Yaw(y));
}

/// `sys 12` / `sys 13`: a motion request; a request of the motion already playing, with no other motion asked
/// for this frame, changes nothing (`0x14029F7C0`'s early return): it is not sent (the script asks for its
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

/// A plain request made with its attack rows (`sys 23`, mode 2, then `sys 13`).
fn attack(cx: &Cx, a: Attack, blend: f32) {
    cx.borrow_mut().clock = (Some(a.request()), 0.0);
    push(cx, ECmd::Attack { request: a.request(), attack: a, blend });
}

/// `func_3524`: wait `n` frames.
async fn wait(cx: &Cx, n: f32) {
    crate::enemy::script::wait_frames(cx, n).await;
}

/// `func_3502(r, n)`: request `r` (`sys 12`, blend `rest_blend`), then `n` frames.
async fn rest(cx: &Cx, r: u16, n: f32) {
    play(cx, r, k(cx, "rest_blend"));
    wait(cx, n).await;
}

/// Whether last frame's displacement (`sys 118`: `+0x10 − +0x490`) is under `speed` a frame (the stuck tests).
fn stuck(cx: &Cx, speed: f32) -> bool {
    let io = cx.borrow();
    io.w.moved - speed * io.w.step < 0.0
}

/// A target within `range` (`func_1B8C` → `sys 48`): Sora (Assumption: KH1's search finds Sora when he is
/// within the range; `func_1B8C`'s fallbacks also give him within it; `sys 139`'s out-of-sight test is taken as
/// in sight). In KH1's world 11 the range is `world11_search` and Sora is always found (`0x1BBA`, `0x1C42`).
fn search(cx: &Cx, range: f32) -> bool {
    if cx.borrow().x.world11 {
        return cx.borrow().w.target.is_some();
    }
    cx.borrow().w.target.is_some() && dist_to(cx, sora(cx)) - range < 0.0
}

/// `func_2212`'s test (`0x646`, `0x7B4`): under `drain_hp` of its HP (Re:CoM's HP, decision M10: the host's
/// ratio at each hit), or it drains always (`wp+0x7C` = 2).
fn drains(cx: &Cx) -> bool {
    let io = cx.borrow();
    io.x.hp - io.c.num("drain_hp") < 0.0 || io.x.drains_always
}

/// How the routines end: `func_1EA2` + `misc8` ends the behaviour thread from any depth (a state request is
/// pending; the supervisor restarts the behaviour next frame).
type Flow = bool;
const END: Flow = true;
const ON: Flow = false;

/// `func_335A(p, speed, arrive, time)`: fly to `p` playing the idle, until within `arrive` flat and one
/// frame's travel vertically, or the time is out; stuck (`fly_check` frames on, under a quarter of the speed):
/// face ahead, stop, and a new point `fly_detour_lo..hi` from the target.
async fn fly_to(cx: &Cx, p: &mut Vec3, speed: f32, arrive: f32, time: f32) {
    let mut t = time;
    let mut check = t - k(cx, "fly_check");
    play(cx, req::IDLE, k(cx, "motion_blend"));
    while t > 0.0 {
        let v = *p - me(cx);
        yaw_along(cx, v);
        let flat = Vec3::new(v.x, 0.0, v.z);
        if flat.length() - arrive < 0.0 && v.y.abs() - speed * step(cx) < 0.0 {
            halt(cx);
            return;
        }
        steer(cx, norm(v), speed);
        t -= step(cx);
        frame().await;
        if t - check < 0.0 && stuck(cx, speed * k(cx, "stuck_factor")) {
            let f = facing(cx);
            face_along(cx, f);
            halt(cx);
            *p = tgt(cx) + random_offset(cx, k(cx, "fly_detour_lo"), k(cx, "fly_detour_hi"));
            check = t - k(cx, "fly_check");
        }
    }
}

/// `func_30C4(target, speed, dist, motion)`: fly at the target (`approach_above` over it) playing `motion`,
/// facing it, until within `dist` of it (or of that point); stuck after `approach_check` frames: done within
/// `approach_retry` × `dist`, else a detour to a point `approach_detour_lo..hi` from the target
/// ([`fly_to`], `approach_detour_time` frames) and again.
async fn approach(cx: &Cx, speed: f32, dist: f32, motion: u16) {
    let mut timer = 0.0;
    loop {
        let tg = tgt(cx);
        let mut g = tg;
        g.y -= k(cx, "approach_above");
        let mut v = g - me(cx);
        yaw_to(cx, tg);
        let s30 = v;
        let d = dist_to(cx, tg);
        if d - dist < 0.0 || s30.length() - dist < 0.0 {
            halt(cx);
            return;
        }
        if v.y > 0.0 {
            if me(cx).y - g.y > 0.0 {
                v.y = 0.0;
            } else if height(cx) - k(cx, "low_height") > 0.0 {
                // `v × speed` before the normalization (no effect; as the script writes it).
                v = v * speed;
            }
        }
        play(cx, motion, k(cx, "motion_blend"));
        steer(cx, norm(v), speed);
        frame().await;
        timer += step(cx);
        if timer - k(cx, "approach_check") > 0.0 && stuck(cx, speed * k(cx, "stuck_factor")) {
            let r = k(cx, "approach_retry");
            let d = dist_to(cx, tgt(cx));
            if d - dist * r < 0.0 || s30.length() - dist * r < 0.0 {
                halt(cx);
                return;
            }
            let mut p = tgt(cx) + random_offset(cx, k(cx, "approach_detour_lo"), k(cx, "approach_detour_hi"));
            fly_to(cx, &mut p, speed, dist, k(cx, "approach_detour_time")).await;
            frame().await;
            timer = 0.0;
        }
    }
}

/// `func_2FF2(blend)`: the vanish: warping; 201 (the idle queued); unhittable (flags 0x20 / 0x40 / 0x10; the
/// lock point masked: Re:CoM's lockable flag stays); its effect (M13); a 100-frame fade-out (no Re:CoM model
/// alpha); `vanish_fade` frames; hidden (flags 0x10 / 0x2 / 0x1 cleared).
async fn vanish(cx: &Cx, blend: f32) {
    cx.borrow_mut().x.warping = true;
    play(cx, req::VANISH, blend);
    queue(cx, req::IDLE, 0.0);
    push(cx, ECmd::Hittable(false));
    wait(cx, k(cx, "vanish_fade")).await;
    push(cx, ECmd::Visible(false));
}

/// `func_2EEC`: the reappearance: stopped; the idle `reappear_wait` frames; facing the target; shown (flags
/// 1 / 2 / 16), its effect (M13), a fade-in; the idle `reappear_wait` frames more; hittable again (flags 16 /
/// 32 / 64 cleared, the lock point unmasked); the warp done. Its request 201 (`0x2F32`, `sys 13`, the idle
/// queued) is cancelled in its own frame by the `sys 12` of `0x2F9E` (`0x14029F7C0` zeroes the pending count
/// `+0x178` first; Checked (decomp)): only the idle plays.
async fn reappear(cx: &Cx) {
    halt(cx);
    rest(cx, req::IDLE, k(cx, "reappear_wait")).await;
    yaw_to(cx, tgt(cx));
    push(cx, ECmd::Visible(true));
    push(cx, ECmd::FadeIn(k(cx, "reappear_fade")));
    rest(cx, req::IDLE, k(cx, "reappear_wait")).await;
    push(cx, ECmd::Hittable(true));
    cx.borrow_mut().x.warping = false;
}

/// `func_2EBA(p, speed)`: the warp: vanish (blend `vanish_blend`), fly invisibly to `p`, reappear, a yield.
async fn warp_to(cx: &Cx, p: Vec3, speed: f32) {
    let mut p = p;
    vanish(cx, k(cx, "vanish_blend")).await;
    fly_to(cx, &mut p, speed, k(cx, "warp_arrive"), k(cx, "warp_time")).await;
    reappear(cx).await;
    frame().await;
}

/// `func_128E`: the burst: its effect after 8 frames (`sys 107`, M13); its rows; facing the target, 204 for its
/// length (facing it until `burst_turn_until` frames are left); the idle (blend `burst_end_blend`); a yield.
async fn burst(cx: &Cx) {
    yaw_to(cx, tgt(cx));
    attack(cx, Attack::Burst, k(cx, "motion_blend"));
    let mut t = motion_frames(cx, req::BURST);
    while t - k(cx, "burst_turn_until") > 0.0 {
        yaw_to(cx, tgt(cx));
        t -= step(cx);
        frame().await;
    }
    wait(cx, t).await;
    play(cx, req::IDLE, k(cx, "burst_end_blend"));
    frame().await;
}

/// `func_13BE`: the drain: its effect after 8 frames (M13); its rows (their hit callback `0x1794`:
/// [`Behaviour::on_landed`]); facing the target, 203 for its length. While more than `drain_steer_until`
/// frames are left it faces the target; while more than `drain_back_after` are left it backs off
/// (`drain_back_dir` × the facing at `drain_back_speed`) within `drain_close` of him, else creeps at him at
/// `drain_creep_speed`. Once the drain landed, the orb (the drained HP, `sys 15` / `sys 16`: not drawn) sits
/// `orb_up` above the victim; in the last `drain_steer_until` frames it flies to `orb_drop` above the Ghost
/// (`p ← (p − me) × k + me − orb_drop`, `k` from `orb_keep` down `orb_fade` a frame): within `orb_near` it has
/// drained (`wp+0x68`, effect 11: M13); within `orb_done` or with `orb_last` frames left it is gone and the
/// drain ends early. The rest of 203's length, then the idle; a yield.
async fn drain(cx: &Cx) {
    {
        let mut io = cx.borrow_mut();
        io.x.drain_taken = false;
    }
    let (mut arrived, mut near) = (false, false);
    yaw_to(cx, tgt(cx));
    attack(cx, Attack::Drain, k(cx, "motion_blend"));
    let mut t = motion_frames(cx, req::DRAIN);
    let mut keep = 0.0f32;
    let mut orb = Vec3::ZERO;
    while t > 0.0 {
        if t - k(cx, "drain_steer_until") >= 0.0 {
            yaw_to(cx, tgt(cx));
            let d = dist_to(cx, tgt(cx));
            if t - k(cx, "drain_back_after") > 0.0 {
                if d - k(cx, "drain_close") < 0.0 {
                    steer(cx, norm(facing(cx)) * k(cx, "drain_back_dir"), k(cx, "drain_back_speed"));
                } else {
                    steer(cx, norm(tgt(cx) - me(cx)), k(cx, "drain_creep_speed"));
                }
            }
            let s = sora(cx);
            let mut io = cx.borrow_mut();
            if io.x.drain > 0 {
                if io.x.drain == 1 {
                    io.x.drain = 2;
                }
                // `*(wp+0x84) + 0x10`: the victim's live position (Sora's).
                orb = s;
                orb.y -= io.c.num("orb_up");
            }
        } else if cx.borrow().x.drain > 0 {
            if cx.borrow().x.drain == 2 {
                keep = k(cx, "orb_keep");
                cx.borrow_mut().x.drain = 3;
            }
            if !arrived {
                orb = sora(cx);
                orb.y -= k(cx, "orb_up");
                arrived = true;
            }
            orb = orb - me(cx);
            let l = orb.length();
            if l - k(cx, "orb_near") < 0.0 {
                if l - k(cx, "orb_done") < 0.0 || t - k(cx, "orb_last") <= 0.0 {
                    // `sys1.3`: the orb removed.
                    break;
                }
                if !near {
                    cx.borrow_mut().x.drained = true;
                    near = true;
                }
            }
            let mut base = me(cx);
            base.y -= k(cx, "orb_drop");
            orb = orb * keep + base;
            keep = (keep - k(cx, "orb_fade")).max(0.0);
        }
        t -= step(cx);
        frame().await;
    }
    wait(cx, t).await;
    play(cx, req::IDLE, k(cx, "motion_blend"));
    frame().await;
}

/// `func_2A50(target, speed, dist, time, mode)`: the hover: turn rate `hover_turn_rate`; it flies away
/// (`hover_far` behind it, turned ±180°: phase 1), after `hover_turn_lo..hi` frames turns off to a side (±60..120°:
/// phase 2), and in phase 2 within `dist` of the target and level with it (± `hover_level`) bursts and flies off
/// again, `hover_bursts` times at most; rising when under `low_height` over the floor; stuck after `hover_check`
/// frames: phase 1 at once. Mode 1 (after a drain): once the hold has run out unhit (`wp+0x6C` back to 0) KH1
/// restores its HP to full (`0x2ADE`: HP = max HP; not built, owner question) and faces the target
/// `hover_rest` frames: done.
async fn hover(cx: &Cx, speed: f32, dist: f32, time: f32, heal: bool) {
    let mut t = time;
    let mut phase = 0;
    let mut bursts = 0;
    let mut timer = 0.0f32;
    let mut p = me(cx);
    set_turn_rate(cx, k(cx, "hover_turn_rate"));
    while t > 0.0 {
        if heal && !cx.borrow().x.holding {
            // `0x2ADE`: stats `+0x3C` = `+0x40` (owner 2026-11-02: on Re:CoM's HP).
            push(cx, ECmd::HealFull);
            let mut n = k(cx, "hover_rest");
            while n > 0.0 {
                yaw_to(cx, tgt(cx));
                n -= step(cx);
                frame().await;
            }
            break;
        }
        if phase == 2 {
            if search(cx, dist) {
                set_target(cx, true);
            }
            let tg = tgt(cx);
            if dist_to(cx, tg) - dist < 0.0 {
                halt(cx);
                let lv = k(cx, "hover_level");
                let y = me(cx).y;
                if y - (tg.y - lv) > 0.0 && y - (tg.y + lv) < 0.0 {
                    phase = 0;
                    timer = 0.0;
                    set_turn_rate(cx, k(cx, "hover_attack_turn_rate"));
                    burst(cx).await;
                    set_turn_rate(cx, k(cx, "hover_turn_rate"));
                    bursts += 1;
                    if bursts as f32 - k(cx, "hover_bursts") >= 0.0 {
                        break;
                    }
                }
            }
        }
        if phase == 0 {
            halt(cx);
            phase = 1;
            let turn = if rand(cx, "hover back") - k(cx, "hover_back_chance") > 0.0 { k(cx, "hover_back") } else { k(cx, "hover_back_neg") };
            p = norm(rotate(facing(cx), turn)) * k(cx, "hover_far") + me(cx);
        } else if phase >= 1 && timer - rand_range(cx, k(cx, "hover_turn_lo"), k(cx, "hover_turn_hi"), "hover turn time") >= 0.0 {
            halt(cx);
            phase = 2;
            timer = 0.0;
            let turn = if rand(cx, "hover side") - k(cx, "hover_back_chance") > 0.0 {
                rand_range(cx, k(cx, "hover_side_lo"), k(cx, "hover_side_hi"), "hover side angle")
            } else {
                rand_range(cx, k(cx, "hover_side2_lo"), k(cx, "hover_side2_hi"), "hover side angle")
            };
            p = norm(rotate(facing(cx), turn)) * k(cx, "hover_far") + me(cx);
        }
        let mut d = norm(p - me(cx));
        yaw_along(cx, d);
        if height(cx) - k(cx, "low_height") < 0.0 {
            d.y = k(cx, "rise_y");
        }
        steer(cx, d, speed);
        t -= step(cx);
        timer += step(cx);
        frame().await;
        if timer - k(cx, "hover_check") > 0.0 && stuck(cx, speed * k(cx, "hover_stuck")) {
            phase = 1;
            timer = k(cx, "hover_stuck_timer");
        }
    }
    set_turn_rate(cx, k(cx, "turn_default"));
}

/// `func_DCE(p, speed, arrive)`: the idle's flight to `p` playing the idle, `wander_time` frames at most:
/// a target set (the cone) → engaged; within `arrive` flat and a frame's travel vertically: done; rising
/// (`rise_y`) when under `low_height` over the floor, more than `home_below` under its home or more than
/// `sora_above` under Sora, sinking (`sink_y`) when more than `sora_below` above him; stuck (after
/// `wander_check` frames, under a quarter of the speed): a new point `wander_new_lo..hi` from itself.
async fn wander(cx: &Cx, p: Vec3, speed: f32, arrive: f32) -> Flow {
    let mut p = p;
    let mut t = k(cx, "wander_time");
    let mut check = 0.0;
    play(cx, req::IDLE, k(cx, "motion_blend"));
    while t > 0.0 {
        if has_target(cx) {
            set_turn_rate(cx, k(cx, "turn_default"));
            set_state(cx, st::ENGAGED);
            return END;
        }
        let v = p - me(cx);
        yaw_along(cx, v);
        let flat = Vec3::new(v.x, 0.0, v.z);
        if flat.length() - arrive < 0.0 && v.y.abs() - speed * step(cx) < 0.0 {
            frame().await;
            break;
        }
        let mut d = norm(v);
        let m = me(cx);
        let (home, s) = (cx.borrow().home, sora(cx));
        if height(cx) - k(cx, "low_height") < 0.0 || m.y - (home.y + k(cx, "home_below")) > 0.0 || s.y - (m.y - k(cx, "sora_above")) < 0.0 {
            d.y = k(cx, "rise_y");
        } else if s.y - (m.y + k(cx, "sora_below")) > 0.0 {
            d.y = k(cx, "sink_y");
        }
        steer(cx, d, speed);
        check += step(cx);
        t -= step(cx);
        frame().await;
        if check - k(cx, "wander_check") > 0.0 && stuck(cx, speed * k(cx, "stuck_factor")) {
            p = me(cx) + random_offset(cx, k(cx, "wander_new_lo"), k(cx, "wander_new_hi"));
            check = 0.0;
        }
    }
    ON
}

/// `func_1044(time)`: the search 202 (blend `search_blend`) for `time` frames: a target set (the cone) →
/// engaged.
async fn look(cx: &Cx, time: f32) -> Flow {
    let mut t = time;
    play(cx, req::SEARCH, k(cx, "search_blend"));
    while t > 0.0 {
        if has_target(cx) {
            set_turn_rate(cx, k(cx, "turn_default"));
            set_state(cx, st::ENGAGED);
            return END;
        }
        t -= step(cx);
        frame().await;
    }
    ON
}

/// The child object (`ex_2230_00.bd` `0x202`, made by `sys 94` at `0xD12`): its effect on bone 45 (`sys 109`
/// 7: M13); while the Ghost plays its idle (0) or its search (202): Sora's lock point within `cone_range` of
/// the eye and within `cone_angle` of its axis (`sys 100` against `sys 3`, the cosine of the angle) → the
/// Ghost's target (`sys 49`), the child ends; any other motion: the child ends. The eye, its axis and Sora's
/// lock point are the host's ([`Behaviour::set_sight`]); party members 1 and 2 (none in Re:CoM's battles) and
/// KH1's flag 0x8000 test on them (Assumption: Sora passes) aren't read.
async fn cone(cx: Cx) {
    loop {
        let m = cx.borrow().w.motion;
        if m != Some(req::IDLE) && m != Some(req::SEARCH) {
            return;
        }
        let s = cx.borrow().x.sight;
        if let Some(Sight { eye, axis, target: Some(t) }) = s {
            let v = t - eye;
            if v.length() - k(&cx, "cone_range") < 0.0 && cosine(axis, norm(v)) - k(&cx, "cone_angle").to_radians().cos() > 0.0 {
                set_target(&cx, true);
                return;
            }
        }
        frame().await;
    }
}

/// The appearance (`0x2130`, a spawned Ghost): 69 (blend `appear_blend`), a fade-in (`appear_fade`), flag 0x80
/// for 30 frames (not mapped), its effect (M13), `appear_wait` frames; then (actor flag 4 with the idle queued,
/// bit 30) the whole script waits until 69 ends.
async fn appear(cx: &Cx) {
    play(cx, req::APPEAR, k(cx, "appear_blend"));
    push(cx, ECmd::FadeIn(k(cx, "appear_fade")));
    wait(cx, k(cx, "appear_wait")).await;
    queue(cx, req::IDLE, k(cx, "appear_then_blend"));
    cx.borrow_mut().x.paused = true;
    frame().await;
    while cx.borrow().w.motion == Some(req::APPEAR) {
        frame().await;
    }
    cx.borrow_mut().x.paused = false;
}

/// The start (`0x10E6`): placed (not spawned) outside world 11: the idle state, draining always. Spawned (every
/// Re:CoM battle spawn; spawn type 6 not taken: Assumption): Sora found (`start_search`) or Sora as the target,
/// face him, the appearance (Re:CoM's own appearance instead after its first-strike stun; KH1 skips it in world
/// 11: kept there too, owner 2026-11-02 "battle spawns play KH1's appearance"), the engaged state.
async fn start(cx: Cx) {
    let (spawned, w11) = {
        let io = cx.borrow();
        (io.x.spawned, io.x.world11)
    };
    if !spawned && !w11 {
        cx.borrow_mut().x.drains_always = true;
        cx.borrow_mut().x.appeared = true;
        set_state(&cx, st::IDLE);
        return;
    }
    let _ = search(&cx, k(&cx, "start_search"));
    set_target(&cx, true);
    let s = sora(&cx);
    face_now(&cx, s);
    if spawned && cx.borrow().x.appear_motion {
        appear(&cx).await;
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::ENGAGED);
}

/// The idle behaviour (`0xCDE`): not noticed; turn rate `idle_turn_rate`; the cone (its child); forever: a
/// point `idle_near_lo..hi` from Sora when he is farther than `idle_far`, else `idle_lo..hi` from itself; the
/// flight to it ([`wander`]); the search for `search_lo..hi` frames ([`look`]). A target set ends it (engaged).
async fn idle(cx: Cx) {
    cx.borrow_mut().x.noticed = false;
    set_turn_rate(&cx, k(&cx, "idle_turn_rate"));
    cx.borrow_mut().x.child_new = true;
    loop {
        let s = sora(&cx);
        let p = if dist_to(&cx, s) - k(&cx, "idle_far") > 0.0 {
            s + random_offset(&cx, k(&cx, "idle_near_lo"), k(&cx, "idle_near_hi"))
        } else {
            me(&cx) + random_offset(&cx, k(&cx, "idle_lo"), k(&cx, "idle_hi"))
        };
        if wander(&cx, p, k(&cx, "wander_speed"), k(&cx, "wander_arrive")).await == END {
            return;
        }
        let t = rand_range(&cx, k(&cx, "search_lo"), k(&cx, "search_hi"), "search time");
        if look(&cx, t).await == END {
            return;
        }
        frame().await;
    }
}

/// `func_1210`: the notice: noticed; facing the target; its effect (M13); a yield; 206 as an action (the idle
/// queued, bit 30, blend `notice_then_blend`); a yield.
async fn notice(cx: &Cx) {
    cx.borrow_mut().x.noticed = true;
    yaw_to(cx, tgt(cx));
    frame().await;
    push(cx, ECmd::Action { request: req::NOTICE, attack: None, then: req::IDLE, blend: k(cx, "motion_blend") });
    queue(cx, req::IDLE, k(cx, "notice_then_blend"));
    frame().await;
}

/// `func_A34`: the drain's approach: at the target at `drain_speed` to `drain_near` playing 200, facing it,
/// the drain; a yield; until a drain lands (`wp+0x58`), 200 again. The target's own checks (alive, `sys 120`,
/// flag 0x8000; else a new search) are taken as passing for Sora (Assumption).
async fn drain_attempt(cx: &Cx) {
    loop {
        approach(cx, k(cx, "drain_speed"), k(cx, "drain_near"), req::LUNGE).await;
        yaw_to(cx, tgt(cx));
        drain(cx).await;
        frame().await;
        if cx.borrow().x.drain >= 1 {
            cx.borrow_mut().x.drain = 0;
            return;
        }
        play(cx, req::LUNGE, k(cx, "motion_blend"));
    }
}

/// `func_7EC`: the roam after a drain: warp to a point `roam_lo..hi` away; Sora within `roam_search`: the
/// target; within `roam_close` the burst, else the search for `roam_wait_lo..hi` frames facing him, bursting
/// once he is within `roam_near`; not found: the search for `roam_lost_lo..hi` frames facing Sora, bursting
/// within `roam_lost_near`; after `roam_rounds` rounds the idle `roam_rest_lo..hi` frames and the idle state.
async fn roam(cx: &Cx) -> Flow {
    loop {
        let p = me(cx) + random_offset(cx, k(cx, "roam_lo"), k(cx, "roam_hi"));
        warp_to(cx, p, k(cx, "warp_speed")).await;
        frame().await;
        if search(cx, k(cx, "roam_search")) {
            set_target(cx, true);
            if dist_to(cx, tgt(cx)) - k(cx, "roam_close") < 0.0 {
                burst(cx).await;
                frame().await;
            } else {
                play(cx, req::SEARCH, k(cx, "motion_blend"));
                let mut t = rand_range(cx, k(cx, "roam_wait_lo"), k(cx, "roam_wait_hi"), "roam wait");
                while t > 0.0 {
                    if dist_to(cx, tgt(cx)) - k(cx, "roam_near") < 0.0 {
                        burst(cx).await;
                        break;
                    }
                    yaw_to(cx, tgt(cx));
                    t -= step(cx);
                    frame().await;
                }
                frame().await;
            }
        } else {
            play(cx, req::SEARCH, k(cx, "motion_blend"));
            let mut t = rand_range(cx, k(cx, "roam_lost_lo"), k(cx, "roam_lost_hi"), "roam wait");
            while t > 0.0 {
                if dist_to(cx, sora(cx)) - k(cx, "roam_lost_near") < 0.0 {
                    burst(cx).await;
                    break;
                }
                yaw_to(cx, sora(cx));
                t -= step(cx);
                frame().await;
            }
            frame().await;
        }
        let rounds = {
            let mut io = cx.borrow_mut();
            io.x.roams += 1;
            io.x.roams
        };
        if rounds as f32 - k(cx, "roam_rounds") >= 0.0 {
            cx.borrow_mut().x.roams = 0;
            let n = rand_range(cx, k(cx, "roam_rest_lo"), k(cx, "roam_rest_hi"), "roam rest");
            rest(cx, req::IDLE, n).await;
            set_state(cx, st::IDLE);
            return END;
        }
        frame().await;
    }
}

/// `func_B62`: the attack round: the approach to `approach_near` at `approach_speed` playing the idle; facing
/// the target, the burst; a flight to a point `away_lo..hi` away at the target's height; then (easing
/// `watch_ease`, turn rate `watch_turn_rate`) `watch_time` frames facing the target, warping away
/// (`warp_lo..hi`) once he is within `watch_near`; the default rates.
async fn attack_round(cx: &Cx) {
    approach(cx, k(cx, "approach_speed"), k(cx, "approach_near"), req::IDLE).await;
    yaw_to(cx, tgt(cx));
    burst(cx).await;
    let mut p = me(cx) + random_offset(cx, k(cx, "away_lo"), k(cx, "away_hi"));
    p.y = tgt(cx).y;
    fly_to(cx, &mut p, k(cx, "away_speed"), k(cx, "away_arrive"), k(cx, "away_time")).await;
    set_air_accel(cx, k(cx, "watch_ease"));
    set_turn_rate(cx, k(cx, "watch_turn_rate"));
    let mut t = k(cx, "watch_time");
    while t > 0.0 {
        if dist_to(cx, tgt(cx)) - k(cx, "watch_near") < 0.0 {
            let p = me(cx) + random_offset(cx, k(cx, "warp_lo"), k(cx, "warp_hi"));
            warp_to(cx, p, k(cx, "warp_speed")).await;
            break;
        }
        yaw_to(cx, tgt(cx));
        t -= step(cx);
        frame().await;
    }
    set_turn_rate(cx, k(cx, "turn_default"));
    set_air_accel(cx, k(cx, "ease_default"));
}

/// The engaged behaviour (`0x786`): the notice once; then forever: under half its HP (or always draining): the
/// drain's approach, the hover with the hold (`hold_hover_speed`, `hover_near`, `hold_hover_time`, mode 1),
/// the roam (it ends in the idle state); else the attack round; a yield.
async fn engaged(cx: Cx) {
    if !cx.borrow().x.noticed {
        notice(&cx).await;
    }
    loop {
        if drains(&cx) {
            drain_attempt(&cx).await;
            hover(&cx, k(&cx, "hold_hover_speed"), k(&cx, "hover_near"), k(&cx, "hold_hover_time"), true).await;
            if roam(&cx).await == END {
                return;
            }
        } else {
            attack_round(&cx).await;
        }
        frame().await;
    }
}

/// The hit behaviour (`0x628`): noticed. Under half its HP (or always draining): at `hit_warp_chance` the
/// vanish (blend 0), invisibly at the target at `lunge_speed` to `lunge_near` playing 200, facing him, the
/// reappearance; then engaged. Else at `hit_warp_chance` a warp to a point `warp_lo..hi` away, otherwise the
/// burst and the hover (`hit_hover_speed`, `hover_near`, `hit_hover_time`, mode 0); then engaged.
async fn hit(cx: Cx) {
    cx.borrow_mut().x.noticed = true;
    if drains(&cx) {
        if rand(&cx, "hit warp") - k(&cx, "hit_warp_chance") > 0.0 {
            vanish(&cx, 0.0).await;
            set_turn_rate(&cx, k(&cx, "turn_default"));
            approach(&cx, k(&cx, "lunge_speed"), k(&cx, "lunge_near"), req::LUNGE).await;
            let t = tgt(&cx);
            face_now(&cx, t);
            reappear(&cx).await;
        }
        set_state(&cx, st::ENGAGED);
        return;
    }
    if rand(&cx, "hit warp") - k(&cx, "hit_warp_chance") > 0.0 {
        let p = me(&cx) + random_offset(&cx, k(&cx, "warp_lo"), k(&cx, "warp_hi"));
        warp_to(&cx, p, k(&cx, "warp_speed")).await;
    } else {
        burst(&cx).await;
        hover(&cx, k(&cx, "hit_hover_speed"), k(&cx, "hover_near"), k(&cx, "hit_hover_time"), false).await;
    }
    set_state(&cx, st::ENGAGED);
}

/// The supervisor (entry `0x253A`): after the init's yield, the start routine; each frame: nothing while it
/// warps (or while the appearance's pause holds the whole script); a state request restarts the behaviour (the
/// target set for engaged and hit, cleared for idle); while engaged, a drain that reached it is held for
/// `hold_time` frames.
async fn supervisor(cx: Cx) {
    frame().await;
    cx.borrow_mut().x.start = true;
    let mut held = 0.0;
    loop {
        while {
            let io = cx.borrow();
            io.x.warping || io.x.paused
        } {
            frame().await;
        }
        if let Some(s) = take_state(&cx) {
            cx.borrow_mut().x.appeared = true;
            set_target(&cx, s != st::IDLE);
            push(&cx, ECmd::State(s));
        }
        if cx.borrow().state.0 == st::ENGAGED {
            let (drained, holding) = {
                let io = cx.borrow();
                (io.x.drained, io.x.holding)
            };
            if drained && !holding {
                cx.borrow_mut().x.holding = true;
                held = 0.0;
            }
            if cx.borrow().x.holding {
                held += step(&cx);
                if held - k(&cx, "hold_time") >= 0.0 {
                    let mut io = cx.borrow_mut();
                    io.x.drained = false;
                    io.x.holding = false;
                }
            }
        }
        frame().await;
    }
}

/// The Search Ghost's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    /// The child object (the cone), while it runs.
    child: Option<Task>,
    started: bool,
}

impl Ai {
    /// `0x28A6`: while it holds a drain (`wp+0x6C` = 1), the hold ends (`wp+0x68` = `wp+0x6C` = 0) and the
    /// reward is given.
    fn hold_reward(&self) {
        let mut io = self.io.borrow_mut();
        if io.x.holding {
            io.x.holding = false;
            io.x.drained = false;
            io.x.reward = true;
        }
    }

    /// A new AI. `seed`: its random stream (Assumption, as the Shadow's: per-enemy seeds). `frames`: KH1 motion
    /// lengths by request. `spawned`: a room spawn (every Re:CoM battle appearance, owner 2026-11-02; false: a
    /// placed Ghost, which drains always).
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, spawned: bool) -> Ai {
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp { spawned, appear_motion: true, hp: 1.0, ..Default::default() });
        Ai { io: Rc::new(RefCell::new(io)), main: None, behaviour: None, child: None, started: false }
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
                io.home = w.pos;
            }
            // `0x2572`: flight with no velocity.
            fly(&self.io, Vec3::ZERO);
            self.main = Some(Box::pin(supervisor(self.io.clone())));
        }
        poll(&mut self.main);
        if std::mem::take(&mut self.io.borrow_mut().x.start) {
            self.behaviour = Some(Box::pin(start(self.io.clone())));
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
        self.children();
        std::mem::take(&mut self.io.borrow_mut().out)
    }

    /// The child objects' frame (after the parent's script: Assumption ±1 frame, KH1's object update order not
    /// read): the cone the idle made starts here.
    fn children(&mut self) {
        if std::mem::take(&mut self.io.borrow_mut().x.child_new) {
            self.child = Some(Box::pin(cone(self.io.clone())));
        }
        poll(&mut self.child);
    }

    /// Whether the cone runs (tests and the log).
    pub fn looking(&self) -> bool {
        self.child.is_some()
    }
}

impl Behaviour for Ai {
    fn frame(&mut self, w: World) -> Vec<ECmd<u8>> {
        Ai::frame(self, w).iter().map(|c| c.map(Attack::index)).collect()
    }
    fn side_frame(&mut self, w: World) -> Vec<ECmd<u8>> {
        if self.child.is_none() {
            return Vec::new();
        }
        self.io.borrow_mut().w = w;
        self.children();
        std::mem::take(&mut self.io.borrow_mut().out).iter().map(|c| c.map(Attack::index)).collect()
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
    /// The hit handler (`0x285A`): the flight easing back to `ease_default` (`0x286E`); a hit while it holds a
    /// drain ends the hold and gives KH1's reward (`0x28A6`: `sys 194(EXP / 10 + 1)` and the drop table
    /// `0x580`, [`Behaviour::take_now`]; owner 2026-11-02: on Re:CoM's EXP and prizes); the drain's orb dropped
    /// (`0x298E`); the hit state 3, clashes and repelled hits too (`0x2A0C`). KH1's attribute exceptions (9:
    /// damage only; 11: no state; 8: the reward on every Gravity hit, `0x29BC`): not carried.
    fn on_hit(&self, _news: HitNews) {
        let ease = self.io.borrow().c.num("ease_default");
        set_air_accel(&self.io, ease);
        self.hold_reward();
        {
            let mut io = self.io.borrow_mut();
            if io.x.drain > 0 {
                io.x.drain = 0;
            }
        }
        set_state(&self.io, st::HIT);
    }
    /// The killing hit: KH1's handler gives the hold's reward before it applies the damage (`0x28A6`
    /// before `sys 44` and the death `0x2A14`).
    fn on_defeat(&self) {
        self.hold_reward();
    }
    /// The reward its hit handler gave (`0x28A6`): KH1's `sys 194` and drop table `0x580` as one
    /// [`ECmd::Reward`].
    fn take_now(&self) -> Vec<ECmd<u8>> {
        let mut io = self.io.borrow_mut();
        if !std::mem::take(&mut io.x.reward) {
            return Vec::new();
        }
        let n = |s: &str| io.c.num(s);
        let mut prizes = [0u8; 7];
        for (i, p) in prizes.iter_mut().enumerate() {
            *p = n(&format!("reward_prize_{i}")).clamp(0.0, 255.0) as u8;
        }
        vec![ECmd::Reward(crate::enemy::Reward { exp_div: n("reward_exp_div") as i32, exp_add: n("reward_exp_add") as i32, prizes })]
    }
    /// The drain's rows' callback (`0x1794`): once per drain, the drained target and its orb (`wp+0x58` = 1).
    /// KH1 refuses a target that is reacting (`sys 71`), has flag 0x80 or isn't `sys 170`'s 1: taken as
    /// passing whenever Re:CoM applied the hit to Sora (Assumption).
    fn on_landed(&self, attack: u8) {
        if attack != Attack::Drain.index() {
            return;
        }
        let mut io = self.io.borrow_mut();
        if !io.x.drain_taken {
            io.x.drain = 1;
            io.x.drain_taken = true;
        }
    }
    fn risen(&self) -> bool {
        self.io.borrow().x.appeared
    }
    fn skip_rise(&self) {
        self.io.borrow_mut().x.appear_motion = false;
    }
    fn set_hp_ratio(&self, r: f32) {
        self.io.borrow_mut().x.hp = r;
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["none", "idle", "engaged", "hit"].get(s as usize).copied().unwrap_or("?")
    }
    fn set_floor(&self, y: f32) {
        self.io.borrow_mut().floor = Some(y);
    }
    fn set_hit_kind(&self, kind: u8) {
        self.io.borrow_mut().x.kind = kind;
    }
    fn set_sight(&self, s: Sight) {
        self.io.borrow_mut().x.sight = Some(s);
    }
    /// KH1's world 11 (Olympus): its searches reach 100 m and always find Sora (`0x1BBA`, `0x1C42`).
    fn set_world(&self, kh1_world: Option<u8>) {
        self.io.borrow_mut().x.world11 = kh1_world == Some(11);
    }
    /// The host's quiet watchdog: engaged again (as the supervisor's state request does), flying with no
    /// velocity, shown and hittable (a warp the restart cuts leaves it hidden), no warp, drain or pause, the
    /// default turn rate and easing.
    fn reset(&self) {
        let (turn, ease) = {
            let io = self.io.borrow();
            (io.c.num("turn_default"), io.c.num("ease_default"))
        };
        {
            let mut io = self.io.borrow_mut();
            io.x.appeared = true;
            io.x.warping = false;
            io.x.paused = false;
            io.x.drain = 0;
            io.body.fly(Vec3::ZERO);
            io.out.push(ECmd::Visible(true));
            io.out.push(ECmd::Hittable(true));
        }
        set_turn_rate(&self.io, turn);
        set_air_accel(&self.io, ease);
        set_state(&self.io, st::ENGAGED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test numbers: the player's scripts' own (the asset step reads them; the disassembly's values), the
    /// shared engine's.
    pub(crate) fn consts() -> Consts {
        let v: &[(&str, f32)] = &[
            ("hold_time", 568.0), ("start_search", 5000.0), ("world11_search", 10000.0), ("motion_blend", 8.0), ("rest_blend", 20.0), ("turn_default", 11.25),
            ("ease_default", 0.2), ("appear_blend", 0.0), ("appear_fade", 30.0), ("appear_wait", 30.0), ("appear_then_blend", 0.0), ("drain_hp", 0.5),
            ("hit_warp_chance", 0.5), ("lunge_speed", 24.0), ("lunge_near", 80.0), ("warp_lo", 500.0), ("warp_hi", 600.0), ("warp_speed", 24.0),
            ("hit_hover_speed", 6.0), ("hover_near", 400.0), ("hit_hover_time", 300.0), ("roam_lo", 700.0), ("roam_hi", 1000.0), ("roam_search", 1000.0),
            ("roam_close", 500.0), ("roam_wait_lo", 400.0), ("roam_wait_hi", 600.0), ("roam_near", 350.0), ("roam_lost_lo", 200.0), ("roam_lost_hi", 300.0),
            ("roam_lost_near", 525.0), ("roam_rounds", 3.0), ("roam_rest_lo", 30.0), ("roam_rest_hi", 60.0), ("hold_hover_speed", 8.0), ("hold_hover_time", 800.0),
            ("drain_speed", 20.0), ("drain_near", 250.0), ("approach_speed", 8.0), ("approach_near", 450.0), ("away_lo", 500.0), ("away_hi", 800.0),
            ("away_speed", 8.0), ("away_arrive", 100.0), ("away_time", 300.0), ("watch_ease", 0.6), ("watch_turn_rate", 6.0), ("watch_time", 120.0),
            ("watch_near", 200.0), ("idle_turn_rate", 6.0), ("idle_far", 1000.0), ("idle_near_lo", 200.0), ("idle_near_hi", 500.0), ("idle_lo", 200.0),
            ("idle_hi", 1000.0), ("wander_speed", 3.0), ("wander_arrive", 10.0), ("search_lo", 60.0), ("search_hi", 600.0), ("wander_time", 500.0),
            ("low_height", 50.0), ("home_below", 200.0), ("sora_above", 200.0), ("rise_y", -0.2), ("sora_below", 150.0), ("sink_y", 0.2),
            ("wander_check", 30.0), ("stuck_factor", 0.25), ("wander_new_lo", 200.0), ("wander_new_hi", 500.0), ("search_blend", 20.0), ("notice_then_blend", 2.0),
            ("burst_turn_until", 85.0), ("burst_end_blend", 6.0), ("drain_steer_until", 60.0), ("drain_back_after", 96.0), ("drain_close", 30.0), ("drain_back_dir", -3.0),
            ("drain_back_speed", 4.0), ("drain_creep_speed", 2.0), ("orb_up", 80.0), ("orb_keep", 0.95), ("orb_near", 320.0), ("orb_done", 100.0),
            ("orb_last", 20.0), ("orb_drop", 30.0), ("orb_fade", 0.02), ("hover_turn_rate", 2.0), ("hover_rest", 30.0), ("hover_level", 150.0),
            ("hover_attack_turn_rate", 6.0), ("hover_bursts", 3.0), ("hover_back_chance", 0.5), ("hover_back", 180.0), ("hover_back_neg", -180.0), ("hover_far", 2000.0),
            ("hover_turn_lo", 100.0), ("hover_turn_hi", 180.0), ("hover_side_lo", -120.0), ("hover_side_hi", -60.0), ("hover_side2_lo", 60.0), ("hover_side2_hi", 120.0),
            ("hover_check", 60.0), ("hover_stuck", 0.15), ("hover_stuck_timer", 180.0), ("vanish_blend", 10.0), ("warp_arrive", 10.0), ("warp_time", 240.0),
            ("vanish_fade", 100.0), ("reappear_wait", 80.0), ("reappear_fade", 100.0), ("approach_above", 50.0), ("approach_check", 30.0), ("approach_retry", 1.5),
            ("approach_detour_lo", 400.0), ("approach_detour_hi", 600.0), ("approach_detour_time", 200.0), ("fly_check", 30.0), ("fly_detour_lo", 600.0), ("fly_detour_hi", 800.0),
            ("cone_range", 300.0), ("cone_angle", 35.0), ("sight_bone", 32.0), ("attack_blend", 8.0),
            ("reward_exp_div", 10.0), ("reward_exp_add", 1.0), ("reward_prize_0", 0.0), ("reward_prize_1", 0.0), ("reward_prize_2", 0.0), ("reward_prize_3", 0.0),
            ("reward_prize_4", 3.0), ("reward_prize_5", 0.0), ("reward_prize_6", 0.0),
            ("walk_speed", 2.0), ("run_speed", 8.0), ("air_accel", 0.1), ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0), ("launch_min", 20.0),
            ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("down_time", 30.0),
            ("air_knock_hop", 20.0), ("weight", 1.0), ("gravity", crate::params::GRAVITY), ("fall_cap", 20.0),
        ];
        Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), NAMES).unwrap()
    }

    fn frames() -> Vec<(u16, f32)> {
        use req::*;
        vec![(IDLE, 300.0), (APPEAR, 120.0), (LUNGE, 40.0), (VANISH, 120.0), (SEARCH, 120.0), (DRAIN, 120.0), (BURST, 164.0), (RECOIL, 82.0), (NOTICE, 108.0), (FLINCH, 80.0), (KNOCK, 120.0)]
    }

    /// Clips the asset step marks looping (the player's `kh1-searchghost.json`: the idle, 200, 201, 202, 203,
    /// 206): the sim plays them on when nothing is queued.
    fn looped(r: u16) -> bool {
        use req::*;
        matches!(r, IDLE | LUNGE | VANISH | SEARCH | DRAIN | NOTICE)
    }

    /// A flat floor at y = 0 (KH1 y down), Sora on it; the host plays motions to their length and queues,
    /// pauses the AI for actions (the notice) and polls only the child then (`side_frame`), keeps polling it
    /// during the steered attacks; the body flies by KH1's flight step, never through the floor. The sight: the
    /// eye 1 m above the body along its facing, Sora's lock point 90 above his feet. The drain lands when Sora
    /// is within 1.5 m in its hit window.
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
        visible: bool,
        hittable: bool,
        /// The host gives Sora's lock point in the sight.
        sees: bool,
    }

    impl Sim {
        fn new(seed: u32, spawned: bool, at: Vec3) -> Sim {
            Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: at, last: at, current: Some(req::IDLE), played: 0.0, queued: None, action: None, log: Vec::new(), frame: 0, visible: true, hittable: true, sees: true }
        }

        fn len(r: u16) -> f32 {
            frames().iter().find(|x| x.0 == r).map_or(30.0, |x| x.1)
        }

        fn apply(&mut self, c: Cmd) {
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
                ECmd::Visible(v) => self.visible = v,
                ECmd::Hittable(h) => self.hittable = h,
                _ => {}
            }
            self.log.push((self.frame, c));
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
                        } else if looped(c) {
                            self.played = 0.0;
                        }
                    }
                }
                let s = sora(self.frame);
                self.ai.set_floor(0.0);
                let yaw = self.ai.body().yaw;
                self.ai.set_sight(Sight { eye: self.pos + Vec3::new(0.0, -100.0, 0.0), axis: Vec3::from_yaw(yaw), target: self.sees.then_some(s + Vec3::new(0.0, -90.0, 0.0)) });
                if self.current == Some(req::DRAIN) && (34.0..54.0).contains(&self.played) && (s - self.pos).length() < 150.0 {
                    self.ai.on_landed(Attack::Drain.index());
                }
                let moved = (self.pos - self.last).length();
                let w = World { step: 1.0, pos: self.pos, target: Some(s), grounded: false, airborne: false, moved, motion: self.current };
                let cmds = if self.action.is_none() { self.ai.frame(w) } else { Behaviour::side_frame(&mut self.ai, w).iter().map(|c| c.map(|i| Attack::ALL[i as usize])).collect() };
                for c in cmds {
                    self.apply(c);
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

        fn states(&self) -> Vec<u8> {
            self.log.iter().filter_map(|(_, c)| if let ECmd::State(s) = c { Some(*s) } else { None }).collect()
        }

        fn notes(&self) -> Vec<&'static str> {
            self.log.iter().filter_map(|(_, c)| if let ECmd::Note(n) = c { Some(*n) } else { None }).collect()
        }

        /// The behaviour in a routine of the test's choice (the state object set as the supervisor would).
        fn start(&mut self, s: u8, f: impl FnOnce(Cx) -> Task) {
            let cx = self.ai.io.clone();
            cx.borrow_mut().state = (s, false);
            cx.borrow_mut().x.appeared = true;
            cx.borrow_mut().has_target = s != st::IDLE;
            self.ai.behaviour = Some(f(cx));
            self.log.clear();
        }
    }

    #[test]
    fn the_names_cover_the_numbers() {
        assert!(Table::new(vec![], NAMES).is_err());
        let c = consts();
        assert_eq!((c.num("cone_range"), c.num("cone_angle"), c.num("hold_time")), (300.0, 35.0, 568.0));
        assert_eq!(Attack::ALL.map(|a| a.request()), [204, 203]);
        assert_eq!(norm(Vec3::ZERO), Vec3::new(0.0, 0.0, 1.0), "KH1's sys 9 of a zero vector");
    }

    /// A battle spawn: Sora as its target, facing him, KH1's appearance (69 with the fade-in, 30 frames, then
    /// the whole script waits for 69's end), then engaged: the notice (206, an action) once.
    #[test]
    fn appears_then_notices() {
        let mut s = Sim::new(7, true, Vec3::new(0.0, -150.0, 0.0));
        let sora = |_| Vec3::new(0.0, 0.0, 800.0);
        s.run(sora, 3);
        assert!(s.plays(req::APPEAR) && s.log.iter().any(|(_, c)| matches!(c, ECmd::FadeIn(_))));
        assert!(!s.ai.risen());
        s.run(sora, 125);
        assert!(s.ai.risen());
        assert_eq!(s.states(), vec![st::ENGAGED]);
        assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::Action { request: req::NOTICE, .. })));
        assert!(s.ai.body().flying);
    }

    /// Its HP above half (spawned): the attack round: in to 4.5 m, the burst (204), a flight away, the watch.
    #[test]
    fn the_attack_round_closes_in_and_bursts() {
        let mut s = Sim::new(3, true, Vec3::new(0.0, -150.0, 0.0));
        let sora = |_| Vec3::new(0.0, 0.0, 1200.0);
        s.run(sora, 2);
        s.start(st::ENGAGED, |cx| Box::pin(async move { attack_round(&cx).await }));
        s.run(sora, 900);
        let at = s.attacks();
        assert_eq!(at.first().map(|a| a.1), Some(Attack::Burst), "{:?}", s.log);
        let f = at[0].0;
        assert!(s.log.iter().any(|(g, c)| *g > f + 160 && matches!(c, ECmd::Motion { request: req::IDLE, .. })), "the idle after 204");
        assert!(!s.attacks().iter().any(|a| a.1 == Attack::Drain));
    }

    /// The cone (the child `ex_2230_00.bd`): a placed Ghost idles and searches; Sora within 3 m and 35° of the
    /// eye's axis becomes its target and it engages; behind it or 4 m away he doesn't.
    #[test]
    fn the_cone_finds_him_in_front_only() {
        let run = |sora: Vec3, n: usize| {
            let mut s = Sim::new(5, false, Vec3::new(0.0, -150.0, 0.0));
            s.run(|_| sora, n);
            s
        };
        // In front (the facing starts at yaw 0: +z), 2.5 m, about level with the eye.
        let s = run(Vec3::new(0.0, -10.0, 250.0), 60);
        assert_eq!(s.states().first(), Some(&st::IDLE));
        assert!(s.states().contains(&st::ENGAGED), "{:?}", s.states());
        assert!(s.log.iter().any(|(_, c)| *c == ECmd::Target(true)));
        // Behind it, 2 m: never (it turns along its flight; checked only for the first frames).
        let s = run(Vec3::new(0.0, -10.0, -200.0), 8);
        assert!(!s.states().contains(&st::ENGAGED) && s.ai.looking());
        // In front, 4 m: out of range.
        let s = run(Vec3::new(0.0, -10.0, 400.0), 8);
        assert!(!s.states().contains(&st::ENGAGED));
        // The child ends when the Ghost plays something else (a reaction).
        let mut s = run(Vec3::new(0.0, -10.0, 4000.0), 5);
        assert!(s.ai.looking());
        s.current = Some(req::FLINCH);
        s.action = Some(req::FLINCH);
        s.run(|_| Vec3::new(0.0, -10.0, 4000.0), 2);
        assert!(!s.ai.looking(), "the child sees the reaction (side frames) and ends");
    }

    /// Sora at 9 m, stepping back 2 m once the drain has started (frame 100), so its orb starts between
    /// `orb_done` and `orb_near` from it.
    fn back_off(f: usize) -> Vec3 {
        Vec3::new(0.0, 0.0, if f < 100 { 900.0 } else { 1100.0 })
    }

    /// A Ghost under half its HP, engaged, the notice done.
    fn draining(seed: u32, sora: impl Fn(usize) -> Vec3) -> Sim {
        let mut s = Sim::new(seed, true, Vec3::new(0.0, -150.0, 0.0));
        s.run(&sora, 2);
        s.ai.set_hp_ratio(0.4);
        s.ai.io.borrow_mut().x.noticed = true;
        s.start(st::ENGAGED, |cx| Box::pin(engaged(cx)));
        s
    }

    /// The frame of its first full heal (`0x2ADE`).
    fn healed_at(s: &Sim) -> Option<usize> {
        s.log.iter().find(|(_, c)| *c == ECmd::HealFull).map(|x| x.0)
    }

    /// Its HP under half: the drain's approach (200), the drain (203). When it lands with the Ghost right on
    /// Sora, its orb is home at once (within `orb_done`): no hold, and the hover after it at once restores its
    /// HP to full (`0x2ADE`).
    #[test]
    fn a_drain_from_close_heals_at_once() {
        let sora = |_| Vec3::new(0.0, 0.0, 900.0);
        let mut s = draining(9, sora);
        s.run(sora, 400);
        assert!(s.plays(req::LUNGE), "the approach plays 200");
        assert_eq!(s.attacks().first().map(|a| a.1), Some(Attack::Drain), "{:?}", s.attacks());
        assert!(!s.ai.io.borrow().x.drained && !s.ai.io.borrow().x.holding);
        let end = s.attacks()[0].0 + 120;
        let heal = healed_at(&s).expect("the heal");
        assert!(heal >= end && heal - end < 10, "{heal} after {end}");
        assert_eq!(s.log.iter().filter(|(_, c)| *c == ECmd::HealFull).count(), 1, "once");
        assert!(s.notes().is_empty() && s.ai.take_now().is_empty(), "no reward");
    }

    /// The orb starting 1–3.2 m away: drained, the hold runs while it hovers off; 568 frames unhit and its HP
    /// goes back to full; then the roam (warps).
    #[test]
    fn the_drain_holds_then_heals() {
        let mut s = draining(9, back_off);
        s.run(back_off, 400);
        assert_eq!(s.attacks().first().map(|a| a.1), Some(Attack::Drain), "{:?}", s.attacks());
        assert!(s.ai.io.borrow().x.holding, "the orb reached it: holding");
        assert!(healed_at(&s).is_none() && s.notes().is_empty());
        let start = s.attacks()[0].0;
        s.run(back_off, 700);
        assert!(!s.ai.io.borrow().x.holding);
        let heal = healed_at(&s).expect("the heal");
        assert!(heal - start > 568 && heal - start < 568 + 130, "{heal} after {start}");
        assert!(s.ai.take_now().is_empty(), "unhit: no reward");
        s.run(back_off, 800);
        let hid = s.log.iter().position(|(_, c)| *c == ECmd::Visible(false)).expect("a warp");
        assert!(s.log[hid..].iter().any(|(_, c)| *c == ECmd::Hittable(true)));
    }

    /// The reward the player's script gives (`0x28A6`: `sys 194(EXP / 10 + 1)`, the drop table `0x580`: 3
    /// prizes of KH1 kind 4).
    fn the_reward() -> ECmd<u8> {
        ECmd::Reward(crate::enemy::Reward { exp_div: 10, exp_add: 1, prizes: [0, 0, 0, 0, 3, 0, 0] })
    }

    /// A hit while it holds the drain: the hold ends, KH1's reward comes in the handler itself (`take_now`,
    /// once), the hit state; the hover then never heals it.
    #[test]
    fn a_hit_during_the_hold_ends_it_with_the_reward() {
        let mut s = draining(9, back_off);
        s.run(back_off, 400);
        assert!(s.ai.io.borrow().x.holding);
        let hit = s.frame;
        s.ai.on_hit(HitNews::Damage);
        assert_eq!(s.ai.take_now(), vec![the_reward()]);
        assert!(s.ai.take_now().is_empty(), "once");
        s.run(back_off, 3);
        assert!(!s.ai.io.borrow().x.holding);
        assert!(s.states().contains(&st::HIT));
        // The hover is cut by the hit: no heal until its next drain (which heals by the rule again).
        s.run(back_off, 900);
        let next = s.attacks().iter().find(|a| a.0 > hit && a.1 == Attack::Drain).map_or(usize::MAX, |a| a.0);
        assert!(healed_at(&s).is_none_or(|h| h > next), "{:?} {next}", healed_at(&s));
        // A second hit: no hold, no reward.
        s.ai.on_hit(HitNews::Damage);
        assert!(s.ai.take_now().is_empty());
        // KH1's numbers: the LV1 record's EXP 10 → 2, LV2's 120 → 13.
        let r = crate::enemy::Reward { exp_div: 10, exp_add: 1, prizes: [0; 7] };
        assert_eq!((r.exp_bonus(10), r.exp_bonus(120), r.exp_bonus(9), r.exp_bonus(0)), (2, 13, 1, 1));
    }

    /// The killing hit while it holds: the reward (before Re:CoM's defeat prizes); not holding: nothing.
    #[test]
    fn a_killing_hit_during_the_hold_rewards_too() {
        let mut s = draining(9, back_off);
        s.run(back_off, 400);
        assert!(s.ai.io.borrow().x.holding);
        s.ai.on_defeat();
        assert_eq!(s.ai.take_now(), vec![the_reward()]);
        let s = Sim::new(9, true, Vec3::ZERO);
        s.ai.on_defeat();
        assert!(s.ai.take_now().is_empty());
    }

    /// The warp: unhittable at once, hidden after the 100-frame fade, shown 80 frames after it arrives, hittable
    /// 80 frames later; no state request is taken while it warps.
    #[test]
    fn the_warp_hides_it_then_shows_it() {
        let mut s = Sim::new(4, true, Vec3::new(0.0, -150.0, 0.0));
        let sora = |_| Vec3::new(0.0, 0.0, 300.0);
        s.run(sora, 2);
        s.start(st::ENGAGED, |cx| Box::pin(async move { warp_to(&cx, Vec3::new(600.0, -150.0, 0.0), 24.0).await }));
        s.run(sora, 1);
        assert!(!s.hittable && s.visible && s.plays(req::VANISH));
        s.ai.on_hit(HitNews::Damage);
        s.run(sora, 100);
        assert!(!s.visible, "hidden after the fade");
        assert!(!s.states().contains(&st::HIT), "the request waits for the warp");
        let hidden = s.log.iter().find(|(_, c)| *c == ECmd::Visible(false)).unwrap().0;
        let mut at_shown = None;
        for _ in 0..600 {
            s.run(sora, 1);
            if at_shown.is_none() && s.visible {
                at_shown = Some(s.pos);
            }
        }
        let shown = s.log.iter().find(|(_, c)| *c == ECmd::Visible(true)).unwrap().0;
        let back = s.log.iter().find(|(_, c)| *c == ECmd::Hittable(true)).unwrap().0;
        assert!(shown - hidden >= 80 && back - shown == 80, "{hidden} {shown} {back}");
        // It arrives within 10 and coasts on at KH1's easing (0.1 a frame): about 100 past.
        let p = at_shown.unwrap();
        assert!((p - Vec3::new(600.0, -150.0, 0.0)).horizontal_length() < 120.0, "{p:?}");
        assert!(s.states().contains(&st::HIT), "taken once the warp is done");
    }

    /// Its HP above half, hit: a warp away or the burst and the hover; then engaged.
    #[test]
    fn hit_reactions_by_hp() {
        let mut warped = 0;
        let mut burst = 0;
        for i in 1u32..12 {
            let seed = i.wrapping_mul(0x9E37_79B9) ^ 0x1234_5678;
            let mut s = Sim::new(seed, true, Vec3::new(0.0, -150.0, 0.0));
            let sora = |_| Vec3::new(0.0, 0.0, 300.0);
            s.run(sora, 2);
            s.start(st::HIT, |cx| Box::pin(hit(cx)));
            s.run(sora, 700);
            if s.log.iter().any(|(_, c)| *c == ECmd::Visible(false)) {
                warped += 1;
            } else if s.attacks().first().map(|a| a.1) == Some(Attack::Burst) {
                burst += 1;
            }
            assert!(s.states().contains(&st::ENGAGED), "seed {seed}");
        }
        assert!(warped > 0 && burst > 0, "{warped} {burst}");
    }

    /// KH1's world 11: the start's search reaches 100 m (Sora is the target there anyway).
    #[test]
    fn world_11_search() {
        let mut s = Sim::new(8, true, Vec3::new(0.0, -150.0, 0.0));
        s.ai.set_world(Some(11));
        s.run(|_| Vec3::new(0.0, 0.0, 6000.0), 2);
        assert!(search(&s.ai.io, 1000.0));
        s.ai.set_world(None);
        assert!(!search(&s.ai.io, 1000.0));
    }

    /// The quiet watchdog's reset: engaged, flying, shown and hittable, the default rates.
    #[test]
    fn reset_engages_again() {
        let mut s = Sim::new(2, true, Vec3::new(0.0, -150.0, 0.0));
        s.run(|_| Vec3::new(0.0, 0.0, 300.0), 200);
        s.ai.io.borrow_mut().x.warping = true;
        s.ai.io.borrow_mut().body.accel = Some(0.6);
        s.ai.reset();
        s.log.clear();
        // The notice (an action) may still play: the sim polls the AI again once it ends.
        s.run(|_| Vec3::new(0.0, 0.0, 300.0), 120);
        assert!(s.states().contains(&st::ENGAGED));
        assert!(s.log.iter().any(|(_, c)| *c == ECmd::Visible(true)) && s.log.iter().any(|(_, c)| *c == ECmd::Hittable(true)));
        let io = s.ai.io.borrow();
        assert!(!io.x.warping && io.body.flying && io.body.accel == Some(0.2) && io.body.turn == Some(11.25f32.to_radians()));
    }

    /// No stall: Sora wandering 1–9 m around for 20 000 frames (12 seeds), with or without a hit every 150–400
    /// frames (its HP falling below half after the 10th, so the drain's path runs): it keeps acting (a command
    /// or steering at least every 900 frames, the host's quiet limit), never goes under the floor, stays
    /// finite, and is never left hidden or unhittable for long (a warp is at most 100 + 240 + 160 frames). With
    /// the hits every seed bursts and the seeds together drain.
    fn long_fight(hits: bool) -> (usize, usize) {
        let (mut bursts, mut drains) = (0, 0);
        for i in 1u32..=12 {
            let seed = i.wrapping_mul(0x9E37_79B9) ^ 0x1234_5678;
            let mut s = Sim::new(seed, true, Vec3::new(0.0, -150.0, 0.0));
            let sora = |f: usize| {
                let a = f as f32 * 0.003;
                let r = 500.0 + 400.0 * (f as f32 * 0.0011).sin();
                Vec3::new(r * a.cos(), 0.0, r * a.sin())
            };
            let mut next_hit = 300;
            let (mut quiet, mut worst_quiet) = (0, 0);
            let (mut hidden, mut worst_hidden) = (0, 0);
            let mut hit_n = 0;
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
                if !s.visible || !s.hittable {
                    hidden += 1;
                    worst_hidden = worst_hidden.max(hidden);
                } else {
                    hidden = 0;
                }
                assert!(s.pos.y <= 0.0 && s.pos.length().is_finite(), "seed {seed}: {:?}", s.pos);
                if hits && f == next_hit {
                    next_hit += 150 + (f * 7919 + seed as usize * 104_729) % 250;
                    if s.action.is_none() && s.hittable {
                        hit_n += 1;
                        s.ai.set_hp_ratio(if hit_n > 10 { 0.4 } else { 0.9 });
                        s.ai.on_hit(HitNews::Damage);
                    }
                }
            }
            assert!(worst_quiet < 900, "seed {seed}: quiet {worst_quiet} frames");
            assert!(worst_hidden <= 520, "seed {seed}: hidden {worst_hidden} frames");
            let at = s.attacks();
            if hits {
                assert!(at.iter().any(|a| a.1 == Attack::Burst), "seed {seed}: {at:?}");
            }
            assert!(s.ai.body().flying, "seed {seed}");
            bursts += at.iter().filter(|a| a.1 == Attack::Burst).count();
            drains += at.iter().filter(|a| a.1 == Attack::Drain).count();
        }
        (bursts, drains)
    }

    /// KH1's longest silent wait: the idle's search (`0x1044`: 202, then up to 600 frames with no request,
    /// steering or turn). A placed Ghost whose cone never sees Sora (no lock point given) idles for 20 000
    /// frames: the
    /// silences come close to 600 and stay under the host's quiet limit (900, `kh1-recom`'s
    /// `kh1_enemy::watchdog::QUIET_LIMIT`, raised from 600 for it).
    #[test]
    fn the_idle_search_is_the_longest_silence() {
        let mut worst = 0;
        for i in 1u32..=6 {
            let seed = i.wrapping_mul(0x9E37_79B9) ^ 0x1234_5678;
            let mut s = Sim::new(seed, false, Vec3::new(0.0, -150.0, 0.0));
            s.sees = false;
            let sora = |_| Vec3::new(0.0, 0.0, 5000.0);
            let mut quiet = 0;
            for _ in 0..20_000 {
                let before = s.log.len();
                s.run(sora, 1);
                if s.log.len() > before || s.ai.body().air_vel.length() > 0.01 {
                    quiet = 0;
                } else {
                    quiet += 1;
                    worst = worst.max(quiet);
                }
            }
            assert!(!s.states().contains(&st::ENGAGED), "seed {seed}: blind, it never engages");
        }
        assert!(worst > 450 && worst < 900, "{worst}");
    }

    #[test]
    fn a_long_fight_never_stalls() {
        let (b, d) = long_fight(true);
        assert!(b > 12 && d > 0, "bursts {b}, drains {d}");
        let (b, _) = long_fight(false);
        assert!(b > 0, "without hits it still finds him and bursts: {b}");
    }
}
