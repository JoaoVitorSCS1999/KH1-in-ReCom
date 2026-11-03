//! KH1 FM's Gargoyle (`xa_ex_2220.mdls` part 5, `ex_2220.bd`), engine-agnostic: its AI script on the shared enemy
//! framework (`crate::enemy`), flying (KH1's movement state 4, `crate::enemy::Body::flying`), and its child, the ball
//! (`ex_2220_00.bd`, [`Ball`]). M11 (`docs/recom-m11-enemies.md` § The Gargoyle). Re:CoM runs it natively: each part
//! is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), the ball one of Re:CoM's own bullets moved by
//! [`Ball`] (`win::kh1_thrown`).
//!
//! Every number is data ([`NAMES`], [`BALL_NAMES`], read by the asset step from the player's `ex_2220.bd` and
//! `ex_2220_00.bd` at checked offsets). Script offsets are the player's own (Checked (files): the scripts disassembled
//! with `tools/kh1-ai-script.py`, the parent's code from `0x538`, the child's from `0x18`; the syscalls named are
//! Checked (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x2706`: init `0x279C` (stats records LV1 `0x14` / LV2 `0x80`, the hit handler `0x2896`, the search object
//!   `0x510`, `wp+0x44` / `+0x4C` / `+0x54` = 0; `wp+0x5C` = 1 only in KH1's world 10 room 9 area 1, which no Re:CoM
//!   battle is: always 0 here), flight with no velocity (`sys 135`), a yield, Sora as its target at once (`0xA14` →
//!   `0x2484`), the appearance as its behaviour thread (`0xA30`), then each frame a state request restarts the
//!   behaviour (1 engaged `0x5A8` → `0x5E0`, 2 hit `0x538` → `0x584`). It never sets a turn rate (`sys 125`): KH1's
//!   default π/16 a frame.
//! - Every motion it plays is a plain request (no action bit): the script never pauses for its own motions, only for
//!   KH1's reactions. Its three attacks are plain requests made with rows (`Cmd::Attack`), steered as they play.
//! - the hit handler `0x2896`: the effect removed, the model shown at once (`0x25E8(actor, 0)`); a clash or a repel
//!   record asks for the hit state; a living damaging hit too; the killing hit's kind 3 becomes 4 (`0x2910`), then its
//!   death `0x29B4` (Re:CoM's defeat here); attributes 8 / 10 ask for the hit state, other attribute hits nothing (the
//!   host carries no attribute: every hit is attribute 0, as for every ported enemy).
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0x1ED2` (KH1 vanishes it with request 70 when Sora is
//! 10 / 800 / 1000 away out of sight: a Re:CoM battle could never end), the perched appearance (spawn type 5, `0xA56`:
//! 210 until Sora is within 6 m, then 211 / 213; a room-placed KH1 Gargoyle), KH1's room rules (`wp+0x5C`, world 10
//! room 7's leash), the search object (`0x210A` → `sys 48`: Sora is the only target), the alpha fades as a look (the
//! model is hidden once a fade-out has run its frames and shown when a fade-in starts: [`Cmd::Visible`]), the effects
//! (`sys 75` / `sys 21`, M13), the appearance's flag 0x80 and its 30-frame fade-in, the death routine (Re:CoM's
//! defeat).

use crate::enemy::script::{fly, frame, motion_frames, poll, pos, push, rand, set_state, steer, take_state, target, tick_clock};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Gargoyle's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The approach (`0x1786`), 80 frames, looped.
    pub const FLY: u16 = 200;
    /// The hover away (`0x15BC`), 80 frames, looped.
    pub const HOVER: u16 = 201;
    /// The strafe, turned −90° (`0x13E2`), 70 frames.
    pub const STRAFE_A: u16 = 202;
    /// The dive (`0x1B98`), 100 frames, its wings' rows `0xEC` / `0x15C` hit [32, 38).
    pub const DIVE: u16 = 203;
    /// The swoop's vanish (near: 204, far: 206) and reappearance (near: 205, which is also the battle appearance; far:
    /// 207), all with root motion.
    pub const VANISH_NEAR: u16 = 204;
    pub const APPEAR: u16 = 205;
    pub const VANISH_FAR: u16 = 206;
    pub const APPEAR_FAR: u16 = 207;
    /// The back-off (`0x11BE`): 208 out (132 frames), 209 back (116), root motion.
    pub const BACK: u16 = 208;
    pub const BACK_IN: u16 = 209;
    /// The swipe (`0x1C86`): 212 (84 frames, hands and body), then 218 (114 frames, foot and body).
    pub const SWIPE: u16 = 212;
    pub const SWIPE_END: u16 = 218;
    /// The roar (`0x1E48`), 80 frames.
    pub const ROAR: u16 = 213;
    /// The strafe, turned +90°, 70 frames.
    pub const STRAFE_B: u16 = 217;
    /// The ball's throw (`0x1AF0`), 328 frames: its child is made with it and lives while it plays.
    pub const BALL: u16 = 219;
}

/// The Gargoyle's attacks, in the order the host indexes them (the manifest's `attacks`). All three are plain requests
/// made with rows (`sys 23`, KH1's mode 2: they live while their request plays): the script keeps steering them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    /// `0x1B98`: 203 with rows `0xEC` / `0x15C` (both wings).
    Dive,
    /// `0x1C86`: 212 with rows `0x1CC` / `0x23C` (the hands) and `0x2AC` (its body, type 2: it only meets swings).
    Swipe,
    /// `0x1D6E`: 218 with rows `0x31C` (a foot) and `0x38C` (its body, type 2).
    SwipeEnd,
}

impl Attack {
    pub const ALL: [Attack; 3] = [Attack::Dive, Attack::Swipe, Attack::SwipeEnd];

    pub fn request(self) -> u16 {
        match self {
            Attack::Dive => req::DIVE,
            Attack::Swipe => req::SWIPE,
            Attack::SwipeEnd => req::SWIPE_END,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Dive => "dive",
            Attack::Swipe => "swipe",
            Attack::SwipeEnd => "swipe end",
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The names of the Gargoyle's numbers (besides `crate::enemy::ENGINE_NAMES` and [`BALL_NAMES`]).
pub const NAMES: &[&str] = &[
    "close_range",
    "first_dive_chance",
    "first_approach_chance",
    "first_swoop_chance",
    "far_ball_chance",
    "far_swoop_chance",
    "close_dive_chance",
    "close_again_chance",
    "close_strafe2_chance",
    "hit_close_range",
    "hit_swipe_chance",
    "hit_dive_chance",
    "appear_fade",
    "appear_rise",
    "appear_rise_speed",
    "len_cut",
    "swipe2_cut",
    "motion_blend",
    "settle_wait",
    "swoop_near",
    "swoop_hover_chance",
    "swoop_far",
    "swoop_turn_after",
    "swoop_extra",
    "swoop_vanish",
    "swoop_fade",
    "away_deg",
    "swoop_rise",
    "swoop_rise_speed",
    "swoop_reveal",
    "swoop_dive_chance",
    "back_fade",
    "strafe_turn",
    "strafe_side_chance",
    "strafe_left",
    "strafe_right",
    "strafe_low",
    "strafe_rise",
    "strafe_high",
    "strafe_sink",
    "strafe_speed",
    "hover_time",
    "hover_range",
    "hover_blend",
    "hover_wait",
    "hover_low",
    "hover_rise",
    "hover_high",
    "hover_sink",
    "hover_speed",
    "hover_far",
    "hover_stuck",
    "approach_time",
    "a1_speed",
    "a1_low",
    "a1_high",
    "a1_sink",
    "a1_rise",
    "a1_stop",
    "a2_speed",
    "a2_low",
    "a2_high",
    "a2_sink",
    "a2_rise",
    "a2_stop",
    "a3_speed",
    "a3_low",
    "a3_high",
    "a3_sink",
    "a3_rise",
    "a3_stop",
    "approach_stuck",
    "hold_low0",
    "hold_high0",
    "hold_low1",
    "hold_high1",
    "hold_rise",
    "hold_sink",
    "hold_speed",
];

/// The ball's numbers (`ex_2220_00.bd`; [`Ball`]).
pub const BALL_NAMES: &[&str] = &[
    "ball_wait",
    "ball_charge",
    "ball_lift",
    "ball_ease",
    "ball_ease_k",
    "ball_toss",
    "ball_gravity",
    "ball_dist_max",
    "ball_rise_max",
    "ball_speed_base",
    "ball_speed_add",
    "ball_tilt_base",
    "ball_tilt_add",
    "ball_bounces",
    "ball_turn",
    "ball_speed",
    "ball_life",
    "ball_fade",
];

/// Every name the manifest must hold for the behaviour.
pub fn all_names() -> Vec<&'static str> {
    NAMES.iter().chain(BALL_NAMES).copied().collect()
}

/// The Gargoyle's numbers.
pub type Consts = Table;

/// The state objects (`wp+0x20`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const ENGAGED: u8 = 1;
    pub const HIT: u8 = 2;
}

/// The Gargoyle's own work area: the actor's steering direction `+0x20` (written by the script and left: only the speed
/// `+0x2C` is zeroed each frame), the roar done `wp+0x4C`, the first round done `wp+0x54`, the appearance, the alpha
/// (`+0x60` / `+0x64`: the model hidden at 0, [`Cmd::Visible`]), the killing hit.
#[derive(Clone, Copy, Debug)]
struct Wp {
    dir: Vec3,
    roared: bool,
    first_done: bool,
    spawned: bool,
    appeared: bool,
    alpha: f32,
    alpha_rate: f32,
    shown: bool,
    dying: bool,
    /// The hit handler showed the model (`0x28A8`): the host applies it with the call ([`Behaviour::take_now`]).
    show_now: bool,
}

impl Default for Wp {
    fn default() -> Wp {
        Wp { dir: Vec3::ZERO, roared: false, first_done: false, spawned: false, appeared: false, alpha: 1.0, alpha_rate: 0.0, shown: true, dying: false, show_now: false }
    }
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

/// The actor's target (`+0x74`): Sora (Re:CoM's battles have no other party member; `0x20C8` / the search keep him).
fn tgt(cx: &Cx) -> Vec3 {
    let io = cx.borrow();
    target(cx).or(io.w.target).unwrap_or(io.w.pos)
}

/// `0x2324`: the distance to the target (`sys 6`, 3D).
fn dist(cx: &Cx) -> f32 {
    (tgt(cx) - me(cx)).length()
}

/// The target's height below it (KH1 y down: `target.y − y`; the scripts' `*(*(wp0+116)+16)+4 − *(wp0+16)+4`).
fn below(cx: &Cx) -> f32 {
    tgt(cx).y - me(cx).y
}

/// `sys 9`: the vector normalised (its length 0 stays 0).
fn norm(v: Vec3) -> Vec3 {
    if v.length() > 0.0 {
        v.normalized()
    } else {
        v
    }
}

/// `sys 10(v, deg)` on a direction: turned about the vertical, its height kept (the Darkball's convention; the sign:
/// Assumption, the same for every port).
pub fn rotate(v: Vec3, deg: f32) -> Vec3 {
    let l = v.horizontal_length();
    Vec3::from_yaw(v.yaw() + deg.to_radians()) * l + Vec3::new(0.0, v.y, 0.0)
}

/// The facing vector `+0x30`.
fn facing(cx: &Cx) -> Vec3 {
    Vec3::from_yaw(cx.borrow().body.yaw)
}

fn set_dir(cx: &Cx, v: Vec3) {
    cx.borrow_mut().x.dir = v;
}

fn dir_y(cx: &Cx, y: f32) {
    cx.borrow_mut().x.dir.y = y;
}

/// `*(actor + 0x2C) = speed`: this frame's steering along `+0x20`.
fn go(cx: &Cx, speed: f32) {
    let d = cx.borrow().x.dir;
    steer(cx, d, speed);
}

/// `0x24C4` (`sys 36`): the target yaw toward the target (the engine turns).
fn yaw_tgt(cx: &Cx) {
    let d = tgt(cx) - me(cx);
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `sys 36` toward a vector.
fn yaw_along(cx: &Cx, d: Vec3) {
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `sys 12` (`0x14029F7C0`): a motion request, nothing when it already plays.
fn play(cx: &Cx, r: u16, blend: f32) {
    {
        let mut io = cx.borrow_mut();
        if io.w.motion == Some(r) {
            return;
        }
        io.clock = (Some(r), 0.0);
    }
    push(cx, ECmd::Motion { request: r, blend });
}

/// `sys 13` (`0x1402B9F80` → `0x14029F980`): a motion request written whatever plays: the request playing starts
/// again ([`ECmd::Restart`]).
fn request(cx: &Cx, r: u16, blend: f32) {
    let again = {
        let mut io = cx.borrow_mut();
        let again = io.w.motion == Some(r);
        io.clock = (Some(r), 0.0);
        again
    };
    push(cx, if again { ECmd::Restart { request: r, blend, start: 0.0 } } else { ECmd::Motion { request: r, blend } });
}

/// `sys 14`: the motion queued.
fn queue(cx: &Cx, r: u16, blend: f32) {
    push(cx, ECmd::Queue { request: r, blend });
}

/// `sys 13` of an attack's request with its rows (`Cmd::Attack`).
fn attack(cx: &Cx, a: Attack) {
    cx.borrow_mut().clock = (Some(a.request()), 0.0);
    let blend = k(cx, "motion_blend");
    push(cx, ECmd::Attack { request: a.request(), attack: a, blend });
}

/// `0x29D2`: wait `n` frames.
async fn wait(cx: &Cx, n: f32) {
    crate::enemy::script::wait_frames(cx, n).await;
}

/// `sys 43(r) − 2`.
fn len_cut(cx: &Cx, r: u16) -> f32 {
    motion_frames(cx, r) - k(cx, "len_cut")
}

/// `0x25BC(actor, n)`: the alpha from 1 toward 0 over `n` frames (`+0x60` = 1, `+0x64` = −1 / n).
fn fade_out(cx: &Cx, n: f32) {
    let mut io = cx.borrow_mut();
    io.x.alpha = 1.0;
    io.x.alpha_rate = -1.0 / n;
    io.out.push(ECmd::Note("fade out (KH1's alpha: the model hidden when it reaches 0)"));
}

/// `0x25E8(actor, n)`: the alpha from 0 toward 1 over `n` frames (`+0x60` = 0, `+0x64` = 1 / n); the model is shown
/// as it starts (Assumption: a look, KH1's ramp not built).
fn fade_in(cx: &Cx, n: f32) {
    let mut io = cx.borrow_mut();
    io.x.alpha = 0.0;
    io.x.alpha_rate = 1.0 / n;
    io.out.push(ECmd::FadeIn(n));
    if !io.x.shown {
        io.x.shown = true;
        io.out.push(ECmd::Visible(true));
    }
}

/// `0x1FAE(mode)`: the height kept during an attack: the target more than `low` below (y down: less) → up, more than
/// `high` → down (`dir` = (0, ∓0.5, 0)); in between the direction stays; speed 1 either way.
fn hold(cx: &Cx, mode: u8) {
    let (lo, hi) = if mode == 0 { (k(cx, "hold_low0"), k(cx, "hold_high0")) } else { (k(cx, "hold_low1"), k(cx, "hold_high1")) };
    let dy = below(cx);
    if dy - lo < 0.0 {
        set_dir(cx, Vec3::new(0.0, k(cx, "hold_rise"), 0.0));
    } else if dy - hi > 0.0 {
        set_dir(cx, Vec3::new(0.0, k(cx, "hold_sink"), 0.0));
    }
    go(cx, k(cx, "hold_speed"));
}

/// `0x1E48`: the roar: roared, 213 (the idle queued), len(213) − 2 frames facing the target.
async fn roar(cx: &Cx) {
    cx.borrow_mut().x.roared = true;
    let mut n = len_cut(cx, req::ROAR);
    request(cx, req::ROAR, k(cx, "motion_blend"));
    queue(cx, req::IDLE, 0.0);
    while n > 0.0 {
        n -= step(cx);
        yaw_tgt(cx);
        frame().await;
    }
}

/// `0x1AF0`: the ball: its child (`sys 94(actor, 0, target, 0x3FC, 0x46C)`: [`Ball`], the host's), 219 (the idle
/// queued), len(219) − 2 frames holding 180..210 above the target, facing it.
async fn ball(cx: &Cx) {
    let mut n = len_cut(cx, req::BALL);
    push(cx, ECmd::Throw(0));
    request(cx, req::BALL, k(cx, "motion_blend"));
    queue(cx, req::IDLE, 0.0);
    while n > 0.0 {
        hold(cx, 1);
        yaw_tgt(cx);
        n -= step(cx);
        frame().await;
    }
}

/// `0x1B98`: the dive: facing the target, 203 with its wings' rows (the idle queued), len(203) − 2 frames holding 50..80
/// above the target.
async fn dive(cx: &Cx) {
    let mut n = len_cut(cx, req::DIVE);
    yaw_tgt(cx);
    push(cx, ECmd::Note("effect 6 (M13)"));
    attack(cx, Attack::Dive);
    queue(cx, req::IDLE, 0.0);
    while n > 0.0 {
        hold(cx, 0);
        yaw_tgt(cx);
        n -= step(cx);
        frame().await;
    }
}

/// `0x1C86`: the swipe: 212 with its rows (218 queued) for len(212) − 2 frames, then 218 with its rows (the idle queued)
/// for len(218) − 8, holding 50..80 above the target.
async fn swipe(cx: &Cx) {
    let mut n = len_cut(cx, req::SWIPE);
    yaw_tgt(cx);
    attack(cx, Attack::Swipe);
    queue(cx, req::SWIPE_END, 0.0);
    while n > 0.0 {
        hold(cx, 0);
        n -= step(cx);
        frame().await;
    }
    let mut n = motion_frames(cx, req::SWIPE_END) - k(cx, "swipe2_cut");
    yaw_tgt(cx);
    attack(cx, Attack::SwipeEnd);
    queue(cx, req::IDLE, 0.0);
    while n > 0.0 {
        hold(cx, 0);
        n -= step(cx);
        frame().await;
    }
}

/// `0x13E2`: the strafe: the way to the target turned −90° (202) or +90° (217), then circling at `strafe_turn`° a frame
/// for len − 2 frames at `strafe_speed`, facing the target, its height kept 50..80 above it.
async fn strafe(cx: &Cx) {
    let mut d = norm(tgt(cx) - me(cx));
    let mut rate = k(cx, "strafe_turn");
    let r = if rand(cx, "strafe side") - k(cx, "strafe_side_chance") > 0.0 {
        d = rotate(d, k(cx, "strafe_left"));
        req::STRAFE_A
    } else {
        d = rotate(d, k(cx, "strafe_right"));
        rate = -rate;
        req::STRAFE_B
    };
    let mut n = len_cut(cx, r);
    play(cx, r, k(cx, "motion_blend"));
    wait(cx, k(cx, "settle_wait")).await;
    while n > 0.0 {
        play(cx, r, 0.0);
        let s = step(cx);
        n -= s;
        d = rotate(d, rate * s);
        set_dir(cx, d);
        yaw_tgt(cx);
        let dy = below(cx);
        if dy - k(cx, "strafe_low") < 0.0 {
            dir_y(cx, k(cx, "strafe_rise"));
        } else if dy - k(cx, "strafe_high") > 0.0 {
            dir_y(cx, k(cx, "strafe_sink"));
        } else {
            dir_y(cx, 0.0);
        }
        go(cx, k(cx, "strafe_speed"));
        frame().await;
    }
}

/// `0x15BC`: the hover: within `hover_range`, 201 and up to `hover_time` frames flying away from the target (facing it)
/// at `hover_speed`, 180..210 above it, until `hover_far` away or stuck.
async fn hover(cx: &Cx) {
    let mut t = k(cx, "hover_time");
    if dist(cx) - k(cx, "hover_range") >= 0.0 {
        return;
    }
    play(cx, req::HOVER, k(cx, "hover_blend"));
    wait(cx, k(cx, "hover_wait")).await;
    while t > 0.0 {
        play(cx, req::HOVER, 0.0);
        t -= step(cx);
        let d = rotate(norm(tgt(cx) - me(cx)), k(cx, "away_deg"));
        yaw_tgt(cx);
        set_dir(cx, d);
        let dy = below(cx);
        if dy - k(cx, "hover_low") < 0.0 {
            dir_y(cx, k(cx, "hover_rise"));
        } else if dy - k(cx, "hover_high") > 0.0 {
            dir_y(cx, k(cx, "hover_sink"));
        } else {
            dir_y(cx, 0.0);
        }
        go(cx, k(cx, "hover_speed"));
        frame().await;
        if dist(cx) - k(cx, "hover_far") > 0.0 {
            break;
        }
        if cx.borrow().w.moved - k(cx, "hover_stuck") * step(cx) < 0.0 {
            break;
        }
    }
}

/// `0x1786(mode)`: the approach: 200, up to `approach_time` frames along its facing (turning to the target) at the mode's
/// speed, its height kept by the mode's window; mode 1 swipes within its stop range, modes 2 / 3 end within theirs at the
/// height window; stuck: done.
async fn approach(cx: &Cx, mode: u8) {
    let p = |n: &str| k(cx, &format!("a{mode}_{n}"));
    let (speed, low, high, sink, rise, stop) = (p("speed"), p("low"), p("high"), p("sink"), p("rise"), p("stop"));
    let mut t = k(cx, "approach_time");
    play(cx, req::FLY, k(cx, "motion_blend"));
    wait(cx, k(cx, "settle_wait")).await;
    while t > 0.0 {
        play(cx, req::FLY, 0.0);
        yaw_tgt(cx);
        set_dir(cx, facing(cx));
        let dy = below(cx);
        if dy - low < 0.0 {
            dir_y(cx, rise);
        } else if dy - high > 0.0 {
            dir_y(cx, sink);
        } else {
            dir_y(cx, 0.0);
        }
        go(cx, speed);
        t -= step(cx);
        frame().await;
        let dy = below(cx);
        if mode == 1 {
            if dist(cx) - stop < 0.0 {
                Box::pin(swipe(cx)).await;
                return;
            }
        } else if dist(cx) - stop < 0.0 && dy - low > 0.0 && dy - high < 0.0 {
            return;
        }
        if cx.borrow().w.moved - speed * k(cx, "approach_stuck") * step(cx) < 0.0 {
            return;
        }
    }
}

/// `0x11BE`: the back-off: 208 (unhittable and fading out over 30 frames from about its middle), then 209 (fading in and
/// hittable again from about its middle), facing the target.
async fn back_off(cx: &Cx) {
    let f = k(cx, "back_fade");
    let mut n = len_cut(cx, req::BACK);
    let mut el = 0.0;
    let mut done = false;
    yaw_tgt(cx);
    request(cx, req::BACK, k(cx, "motion_blend"));
    while n > 0.0 {
        let s = step(cx);
        n -= s;
        el += s;
        frame().await;
        if el - (n - f) > 0.0 && !done {
            done = true;
            fade_out(cx, f);
            push(cx, ECmd::Hittable(false));
            push(cx, ECmd::Note("effect sys 21 (M13)"));
        }
    }
    let mut n = len_cut(cx, req::BACK_IN);
    let mut el = 0.0;
    let mut done = false;
    yaw_tgt(cx);
    request(cx, req::BACK_IN, k(cx, "motion_blend"));
    while n > 0.0 {
        let s = step(cx);
        n -= s;
        el += s;
        frame().await;
        if el - (n - f) > 0.0 && !done {
            fade_in(cx, f);
            done = true;
            push(cx, ECmd::Hittable(true));
        }
    }
}

/// `0xDAA`: the swoop: near (< 300) first the hover or the back-off; then the vanish (204, or 206 beyond 700:
/// unhittable and fading out 20 frames before its end), the unseen approach (mode 3, or 2 when far), the reappearance
/// (205 rising facing away, turning to the target after 40 frames; 207 facing it), hittable again halfway; then the dive
/// (90 %) or the swipe.
async fn swoop(cx: &Cx) {
    if dist(cx) - k(cx, "swoop_near") < 0.0 {
        if rand(cx, "swoop near") - k(cx, "swoop_hover_chance") > 0.0 {
            Box::pin(hover(cx)).await;
        } else {
            Box::pin(back_off(cx)).await;
        }
    }
    let far = dist(cx) - k(cx, "swoop_far") > 0.0;
    let (vanish, reappear) = if far { (req::VANISH_FAR, req::APPEAR_FAR) } else { (req::VANISH_NEAR, req::APPEAR) };
    let mut n = len_cut(cx, vanish) + k(cx, "swoop_extra");
    let mut gone = false;
    yaw_tgt(cx);
    request(cx, vanish, k(cx, "motion_blend"));
    while n > 0.0 {
        n -= step(cx);
        frame().await;
        if n - k(cx, "swoop_vanish") < 0.0 && !gone {
            gone = true;
            push(cx, ECmd::Hittable(false));
            fade_out(cx, k(cx, "swoop_fade"));
            push(cx, ECmd::Note("effect sys 21 (M13)"));
        }
    }
    Box::pin(approach(cx, if far { 2 } else { 3 })).await;
    let mut n = len_cut(cx, reappear);
    let mut el = 0.0;
    let mut back = false;
    if !far {
        let d = rotate(norm(tgt(cx) - me(cx)), k(cx, "away_deg"));
        set_dir(cx, d);
        yaw_along(cx, d);
        dir_y(cx, k(cx, "swoop_rise"));
        go(cx, k(cx, "swoop_rise_speed"));
    } else {
        yaw_tgt(cx);
    }
    fade_in(cx, k(cx, "swoop_fade"));
    request(cx, reappear, k(cx, "motion_blend"));
    while n > 0.0 {
        let s = step(cx);
        n -= s;
        el += s;
        frame().await;
        if !far {
            set_dir(cx, Vec3::new(0.0, k(cx, "swoop_rise"), 0.0));
            go(cx, k(cx, "swoop_rise_speed"));
            if el - k(cx, "swoop_turn_after") > 0.0 {
                yaw_tgt(cx);
            }
        }
        if el - n * k(cx, "swoop_reveal") > 0.0 && !back {
            back = true;
            push(cx, ECmd::Hittable(true));
        }
    }
    if rand(cx, "swoop end") - k(cx, "swoop_dive_chance") > 0.0 {
        dive(cx).await;
    } else {
        swipe(cx).await;
    }
}

/// `0x892`: close: the strafe, the roar, the dive (20 %) or the swipe; then (70 %, still close) the strafe, maybe another
/// (50 %), the roar and the swipe; the back-off.
async fn close(cx: &Cx) {
    strafe(cx).await;
    roar(cx).await;
    if rand(cx, "close attack") - k(cx, "close_dive_chance") > 0.0 {
        dive(cx).await;
    } else {
        swipe(cx).await;
    }
    let d = dist(cx);
    if rand(cx, "close again") - k(cx, "close_again_chance") > 0.0 && d - k(cx, "close_range") < 0.0 {
        strafe(cx).await;
        let r = rand(cx, "close strafe again");
        let d = dist(cx);
        if r - k(cx, "close_strafe2_chance") < 0.0 && d - k(cx, "close_range") < 0.0 {
            strafe(cx).await;
        }
        roar(cx).await;
        swipe(cx).await;
    }
    back_off(cx).await;
}

/// `0x7E6`: far: the ball (70 %); then the swoop (70 %) or the approach (mode 1).
async fn far(cx: &Cx) {
    if rand(cx, "far ball") - k(cx, "far_ball_chance") < 0.0 {
        ball(cx).await;
    }
    if rand(cx, "far move") - k(cx, "far_swoop_chance") > 0.0 {
        swoop(cx).await;
    } else {
        approach(cx, 1).await;
    }
}

/// The engaged behaviour (`0x5E0`): the roar once; the first round (close: the dive or the swipe; far: the approach, the
/// swoop or the ball), then each round close or far.
async fn engaged(cx: Cx) {
    if !cx.borrow().x.roared {
        roar(&cx).await;
    }
    loop {
        if !cx.borrow().x.first_done {
            cx.borrow_mut().x.first_done = true;
            let d = dist(&cx);
            let r = rand(&cx, "first round");
            if d - k(&cx, "close_range") < 0.0 {
                if r - k(&cx, "first_dive_chance") < 0.0 {
                    dive(&cx).await;
                } else {
                    swipe(&cx).await;
                }
            } else if r - k(&cx, "first_approach_chance") < 0.0 {
                approach(&cx, 1).await;
            } else if r - k(&cx, "first_swoop_chance") > 0.0 {
                swoop(&cx).await;
            } else {
                // `wp+0x5C` is 0 in every Re:CoM battle: the ball.
                ball(&cx).await;
            }
        }
        if dist(&cx) - k(&cx, "close_range") < 0.0 {
            close(&cx).await;
        } else {
            far(&cx).await;
        }
        frame().await;
    }
}

/// The hit behaviour (`0x584` → `0x9A2`): roared; within 400 the swipe (60 %) or the dive (20 %), then the back-off;
/// farther the swoop; engaged again.
async fn hit(cx: Cx) {
    cx.borrow_mut().x.roared = true;
    if dist(&cx) - k(&cx, "hit_close_range") < 0.0 {
        let r = rand(&cx, "hit attack");
        if r - k(&cx, "hit_swipe_chance") < 0.0 {
            swipe(&cx).await;
        } else if r - k(&cx, "hit_dive_chance") > 0.0 {
            dive(&cx).await;
        }
        back_off(&cx).await;
    } else {
        swoop(&cx).await;
    }
    set_state(&cx, st::ENGAGED);
}

/// The appearance (`0xA30`, a spawn type other than 0 and 5: `0xC5C`): the 30-frame fade-in, 205 (the idle queued)
/// rising `appear_rise_speed` a frame for its length, facing the target; then engaged.
async fn appear(cx: Cx) {
    let mut n = motion_frames(&cx, req::APPEAR);
    push(&cx, ECmd::Note("effect 11 (M13)"));
    push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
    yaw_tgt(&cx);
    request(&cx, req::APPEAR, k(&cx, "motion_blend"));
    queue(&cx, req::IDLE, k(&cx, "motion_blend"));
    set_dir(&cx, Vec3::ZERO);
    while n > 0.0 {
        n -= step(&cx);
        frame().await;
        set_dir(&cx, Vec3::new(0.0, k(&cx, "appear_rise"), 0.0));
        go(&cx, k(&cx, "appear_rise_speed"));
        yaw_tgt(&cx);
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::ENGAGED);
}

/// The supervisor (entry `0x2706`): after the init frame Sora is its target; a placed Gargoyle engages at once; then
/// each frame a state request restarts the behaviour.
async fn supervisor(cx: Cx) {
    frame().await;
    crate::enemy::script::set_target(&cx, true);
    if !cx.borrow().x.spawned {
        cx.borrow_mut().x.appeared = true;
        set_state(&cx, st::ENGAGED);
    }
    loop {
        if let Some(s) = take_state(&cx) {
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Gargoyle's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (as every port: per-enemy seeds). `frames`: KH1 motion lengths by request.
    /// `spawned`: the battle spawned it (its appearance plays; every Re:CoM battle appearance, owner 2026-11-02).
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, spawned: bool) -> Ai {
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp { spawned, ..Default::default() });
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
            alpha_step(&mut io);
        }
        if !self.started {
            self.started = true;
            self.io.borrow_mut().state = (st::NONE, false);
            // `0x273E`: flight with no velocity.
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
                _ => Box::pin(hit(cx)),
            });
        }
        poll(&mut self.behaviour);
        std::mem::take(&mut self.io.borrow_mut().out)
    }
}

/// KH1's alpha (`+0x60` += `+0x64` × step, clamped to 0..1 by the engine): the model hidden once a fade-out reaches 0.
fn alpha_step(io: &mut crate::enemy::script::Core<Consts, Attack, Wp>) {
    if io.x.alpha_rate == 0.0 {
        return;
    }
    io.x.alpha = (io.x.alpha + io.x.alpha_rate * io.w.step).clamp(0.0, 1.0);
    if io.x.alpha <= 0.0 && io.x.alpha_rate < 0.0 {
        io.x.alpha_rate = 0.0;
        if io.x.shown {
            io.x.shown = false;
            io.out.push(ECmd::Visible(false));
        }
    } else if io.x.alpha >= 1.0 {
        io.x.alpha_rate = 0.0;
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
    /// The hit handler (`0x2896`): its effect removed and its model shown (`0x28A8`: alpha 0 at a rate of 1 / 0, shown
    /// the next frame); a clash, a repel or a living damaging hit asks for the hit state.
    fn on_hit(&self, _news: HitNews) {
        {
            let mut io = self.io.borrow_mut();
            io.x.alpha = 1.0;
            io.x.alpha_rate = 0.0;
            if !io.x.shown {
                io.x.shown = true;
                io.x.show_now = true;
            }
        }
        set_state(&self.io, st::HIT);
    }
    fn take_now(&self) -> Vec<ECmd<u8>> {
        if std::mem::take(&mut self.io.borrow_mut().x.show_now) {
            vec![ECmd::Visible(true)]
        } else {
            Vec::new()
        }
    }
    /// The killing hit (`0x2214` answers 0): its death follows (`0x29B4`: Re:CoM's defeat here).
    fn on_defeat(&self) {
        self.io.borrow_mut().x.dying = true;
    }
    /// `0x2910`: the killing hit's kind 3 (a flinch) becomes 4 (a knock-back); living hits keep their kind.
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
    fn state_name(&self, s: u8) -> &'static str {
        ["none", "engaged", "hit"].get(s as usize).copied().unwrap_or("?")
    }
    fn set_floor(&self, y: f32) {
        self.io.borrow_mut().floor = Some(y);
    }
    /// The host's quiet watchdog: engaged again, flying with no velocity, shown and hittable.
    fn reset(&self) {
        {
            let mut io = self.io.borrow_mut();
            io.x.appeared = true;
            io.x.alpha = 1.0;
            io.x.alpha_rate = 0.0;
            io.x.shown = true;
            io.body.fly(Vec3::ZERO);
            io.out.push(ECmd::Visible(true));
            io.out.push(ECmd::Hittable(true));
        }
        set_state(&self.io, st::ENGAGED);
    }
}

/// The ball's numbers ([`BALL_NAMES`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BallNumbers {
    pub wait: f32,
    pub charge: f32,
    pub lift: f32,
    pub ease: f32,
    pub ease_k: f32,
    pub toss: f32,
    pub gravity: f32,
    pub dist_max: f32,
    pub rise_max: f32,
    pub speed_base: f32,
    pub speed_add: f32,
    pub tilt_base: f32,
    pub tilt_add: f32,
    pub bounces: f32,
    pub turn: f32,
    pub speed: f32,
    pub life: f32,
}

impl BallNumbers {
    pub fn from(c: &dyn Numbers) -> BallNumbers {
        let n = |s: &str| c.num(s);
        BallNumbers {
            wait: n("ball_wait"),
            charge: n("ball_charge"),
            lift: n("ball_lift"),
            ease: n("ball_ease"),
            ease_k: n("ball_ease_k"),
            toss: n("ball_toss"),
            gravity: n("ball_gravity"),
            dist_max: n("ball_dist_max"),
            rise_max: n("ball_rise_max"),
            speed_base: n("ball_speed_base"),
            speed_add: n("ball_speed_add"),
            tilt_base: n("ball_tilt_base"),
            tilt_add: n("ball_tilt_add"),
            bounces: n("ball_bounces"),
            turn: n("ball_turn"),
            speed: n("ball_speed"),
            life: n("ball_life"),
        }
    }
}

/// Where the ball is in its child script.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BallPhase {
    /// The child's first frame (`0xB22` → `0x18`: it yields at once).
    Start,
    /// `0x2E..0x7C`: nothing yet; the frames left.
    Wait(f32),
    /// `0x7C..0x184`: the model held at the hold point; the frames left.
    Charge(f32),
    /// `0x184..0x222`: easing toward the launch point; the frames left.
    Ease(f32),
    /// `0x2BA`: flying (its attack on).
    Flying,
    Ended,
}

/// What a ball frame did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BallEvent {
    Nothing,
    /// The model appears (`sys 15`, the charge's start): the host makes its bullet with its attack off.
    Appears,
    /// The attack is made and it flies (`0x222`).
    Launched,
    /// A floor contact re-aimed it (`0x3EE..0x4F8`).
    Bounced,
    /// It ended (its parent's 219 stopped before the launch, the third contact, its life, `sys1.3` / `sys 97`).
    Ended,
}

/// What the ball reads each frame (KH1 units, y down).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BallWorld {
    pub step: f32,
    /// The parent still plays 219 (`sys 51`; the checks after each yield before the launch).
    pub parent_plays: bool,
    /// The hold point (`sys 19(parent, 0x1027, 0)`), before the lift.
    pub hold: Vec3,
    /// The launch point (`sys 19(parent, 28, 1)`).
    pub at: Vec3,
    /// The parent's facing (`+0x30`).
    pub facing: Vec3,
    /// The target's lock point 0 while it lives (`sys 120` / `sys 119`).
    pub target: Option<Vec3>,
    /// The floor's height under it (the battle floor; Re:CoM's arenas are flat: Assumption, as every flier's).
    pub floor: Option<f32>,
}

/// The ball (`ex_2220_00.bd`): its child script's state, stepped once a frame. Before the launch it follows its
/// parent's bones; then it flies [`Ball::flight`]'s way: along its direction at its speed, falling 0.5 a frame faster
/// each frame (`sp+0x3C`), re-aimed at the target by each floor contact (KH1's background test from its last place to
/// its new one, `0x1402B3A00`: surface type 0 → `+4 & 0x100`, read by the script the next frame); the third contact ends
/// it. Checked (files + decomp).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub n: BallNumbers,
    pub phase: BallPhase,
    pub pos: Vec3,
    pub dir: Vec3,
    /// `hp[0x40]`.
    pub speed: f32,
    /// `sp+0x3C`: the fall added to the height each frame.
    pub fall: f32,
    /// `hp[0x44]`.
    pub time: f32,
    /// `sp+0x40`.
    pub bounces: f32,
    /// The last frame's move crossed the floor (`+4 & 0x100`).
    pub contact: bool,
    /// A swing sent it back (`0x748`): its second and last flight.
    pub batted: bool,
}

impl Ball {
    pub fn new(n: BallNumbers) -> Ball {
        Ball { n, phase: BallPhase::Start, pos: Vec3::ZERO, dir: Vec3::ZERO, speed: n.speed, fall: 0.0, time: 0.0, bounces: 0.0, contact: false, batted: false }
    }

    pub fn ended(&self) -> bool {
        self.phase == BallPhase::Ended
    }

    /// Its attack is on (it flies).
    pub fn flying(&self) -> bool {
        self.phase == BallPhase::Flying
    }

    /// The model exists (from the charge on).
    pub fn shown(&self) -> bool {
        matches!(self.phase, BallPhase::Charge(_) | BallPhase::Ease(_) | BallPhase::Flying)
    }

    fn hold_point(&self, w: &BallWorld) -> Vec3 {
        w.hold + Vec3::new(0.0, self.n.lift, 0.0)
    }

    /// `0x2BA`'s start: the direction tilted (`y` = −0.8) and normalised, the fall, the clock and the count back to 0.
    fn toss(&mut self) {
        self.dir.y = self.n.toss;
        self.dir = norm(self.dir);
        self.fall = 0.0;
        self.time = 0.0;
        self.bounces = 0.0;
        self.contact = false;
    }

    /// One frame.
    pub fn step(&mut self, w: &BallWorld) -> BallEvent {
        let s = w.step;
        match self.phase {
            BallPhase::Ended => BallEvent::Ended,
            BallPhase::Start => {
                self.phase = BallPhase::Wait(self.n.wait);
                BallEvent::Nothing
            }
            BallPhase::Wait(t) | BallPhase::Charge(t) | BallPhase::Ease(t) => {
                if !w.parent_plays {
                    self.phase = BallPhase::Ended;
                    return BallEvent::Ended;
                }
                let t = t - s;
                if t > 0.0 {
                    match self.phase {
                        BallPhase::Wait(_) => self.phase = BallPhase::Wait(t),
                        BallPhase::Charge(_) => {
                            self.phase = BallPhase::Charge(t);
                            self.pos = self.hold_point(w);
                        }
                        _ => {
                            self.phase = BallPhase::Ease(t);
                            self.pos = self.pos + (w.at - self.pos) * self.n.ease_k;
                        }
                    }
                    return BallEvent::Nothing;
                }
                match self.phase {
                    BallPhase::Wait(_) => {
                        self.phase = BallPhase::Charge(self.n.charge);
                        self.pos = self.hold_point(w);
                        BallEvent::Appears
                    }
                    BallPhase::Charge(_) => {
                        self.phase = BallPhase::Ease(self.n.ease);
                        self.pos = self.pos + (w.at - self.pos) * self.n.ease_k;
                        BallEvent::Nothing
                    }
                    _ => {
                        // `0x222`: the attack, along the parent's facing, the first flight's frame.
                        self.phase = BallPhase::Flying;
                        self.dir = w.facing;
                        self.toss();
                        self.fly(w);
                        BallEvent::Launched
                    }
                }
            }
            BallPhase::Flying => {
                let mut ev = BallEvent::Nothing;
                if self.contact {
                    self.contact = false;
                    if let Some(tp) = w.target {
                        // `0x41E..0x4E4`: re-aimed at the target's lock point.
                        let d = (tp - self.pos).length().clamp(0.0, self.n.dist_max);
                        let h = (self.pos.y - tp.y).abs().clamp(0.0, self.n.rise_max);
                        self.speed = self.n.speed_base + self.n.speed_add * (d / self.n.dist_max);
                        let tilt = self.n.tilt_base + self.n.tilt_add * (h / self.n.rise_max);
                        self.dir = norm(tp - self.pos);
                        self.dir.y = tilt;
                        self.dir = norm(self.dir);
                        self.fall = 0.0;
                        self.bounces += 1.0;
                        if self.n.bounces - self.bounces < 0.0 {
                            self.phase = BallPhase::Ended;
                            return BallEvent::Ended;
                        }
                        ev = BallEvent::Bounced;
                    }
                }
                if self.time - self.n.life >= 0.0 {
                    self.phase = BallPhase::Ended;
                    return BallEvent::Ended;
                }
                self.fly(w);
                ev
            }
        }
    }

    /// `0x304..0x3E4`: one move: `dir × speed × step` and the fall (+ 0.5 × step each frame) on its height; a crossing of
    /// the floor going down is a contact.
    fn fly(&mut self, w: &BallWorld) {
        let s = w.step;
        let mut d = self.dir * (self.speed * s);
        self.fall += self.n.gravity * s;
        d.y += self.fall;
        let prev = self.pos;
        self.pos += d;
        self.time += s;
        if let Some(f) = w.floor {
            if prev.y < f && self.pos.y >= f {
                self.contact = true;
            }
        }
    }

    /// `0x748`: a swing sent it back (its attack met the swing: `+4 & 0x30`). Toward its parent (`parent`: its lock point
    /// while it lives, else the swinger's facing), joined with the way to the swinger's target (`swinger_target`) when
    /// the turn number is not 0; then `0x2BA` again (tossed, its life anew, its count 0, its speed kept). Its second and
    /// last flight.
    pub fn bat(&mut self, parent: Option<Vec3>, swinger_target: Option<Vec3>, swinger_facing: Vec3) {
        let mut d = match parent {
            Some(p) => norm(p - self.pos),
            None => swinger_facing,
        };
        if let Some(t) = swinger_target {
            let to = norm(t - self.pos);
            d = if self.n.turn == 0.0 { to } else { norm(d + to) };
        }
        self.dir = d;
        self.batted = true;
        self.phase = BallPhase::Flying;
        self.toss();
    }
}

#[cfg(test)]
mod tests;
