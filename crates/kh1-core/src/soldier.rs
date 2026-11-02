//! KH1 FM's Soldier (`xa_ex_2010.mdls` part 5, `ex_2010.bd`), engine-agnostic: its AI script on
//! the shared enemy framework (`crate::enemy`: body physics, reactions, the script helpers). M11
//! (`docs/recom-m11-enemies.md` § The Soldier). Re:CoM runs it natively: each part is the step of
//! a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), Re:CoM plays the motions, collides the
//! body and the attacks, applies the damage.
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2010.bd` at
//! checked offsets); nothing of the game is in this file. Script offsets are the player's
//! `ex_2010.bd`'s (Checked (files): the script disassembled; the VM's ops Checked (decomp
//! `0x1402CC620`)).
//!
//! # The script's shape
//!
//! - supervisor `0x2152`: init `0x2298` (stats record, walk 3 / run 7 / hop 300 into the stats
//!   block, home, the state object, the handlers), a yield, the appearance thread `0x716`; then
//!   each frame: while idle (state 1) the target search (`0x13A0`: Sora within 1000 in sight →
//!   engaged); a state request restarts the behaviour thread: 1 idle `0x6D2`, 2 engaged `0x4D2`,
//!   3 hit `0x3E8`.
//! - the hit handler `0x234C`: a damaging hit counts (`wp+0x38` += 1) and asks for state 3; a
//!   blocked attack (the clash, kinds 6 / 7) asks for state 3 without counting; death → `0x1540`.
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0xE62` (KH1 removes a Soldier more
//! than 600 (out of sight) / 1500 away from Sora: in a Re:CoM battle room it would never be
//! defeated and the battle could not end), the scripted-room target rules (`world 11`, room 3 / 3 /
//! 2) and the death routine (Re:CoM's defeat, prizes, EXP).

use crate::enemy::script::{approach, dist_to_target, face_target, face_target_now, frame, jump_at, poll, pos, push, rand, rand_range, set_state, set_target, take_state, target, wait_airborne, wait_frames, wait_grounded, wander};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Soldier's script plays (its `.mset` map; Checked (files)). The
/// shared ones (idle 0 … landing 7, the reactions) are `crate::enemy::react::req`.
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The appearance (`0x16CA`: `0x1800045`, bits 24 and 23), 86 frames, root motion.
    pub const APPEAR: u16 = 69;
    /// The first engagement's look (`0x758`: plain request 208), 84 frames.
    pub const NOTICE: u16 = 208;
    /// The kick (`0xC08`), 110 frames, hit [60, 70).
    pub const KICK: u16 = 200;
    /// The double kick (`0xBAC`), 134 frames, hits [60, 70) and [92, 120).
    pub const DOUBLE_KICK: u16 = 201;
    /// The jumping kick (`0xA12`: `0x18000CA`), 140 frames, hit [76, 116).
    pub const JUMP_KICK: u16 = 202;
    /// The spin kick (`0x9B6`), 100 frames, hits [0, 46) and [46, 64).
    pub const SPIN_KICK: u16 = 203;
    /// The kick in the air at the end of the jump at a target above (`0x92C`: `0x18000CC`), 58
    /// frames, hit [20, 30).
    pub const AIR_KICK: u16 = 204;
    /// The rush (`0xAA0`: 207, then the leap 209 `0x18000D1`).
    pub const RUSH: u16 = 207;
    pub const RUSH_LEAP: u16 = 209;
    /// The crouch before the rush (`0x7C8`), 200 frames.
    pub const CROUCH: u16 = 216;
}

/// The Soldier's attacks, in the order the host indexes them (the manifest's `attacks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    Kick,
    DoubleKick,
    JumpKick,
    SpinKick,
    AirKick,
    Rush,
    RushLeap,
}

impl Attack {
    pub const ALL: [Attack; 7] = [Attack::Kick, Attack::DoubleKick, Attack::JumpKick, Attack::SpinKick, Attack::AirKick, Attack::Rush, Attack::RushLeap];

    pub fn request(self) -> u16 {
        match self {
            Attack::Kick => req::KICK,
            Attack::DoubleKick => req::DOUBLE_KICK,
            Attack::JumpKick => req::JUMP_KICK,
            Attack::SpinKick => req::SPIN_KICK,
            Attack::AirKick => req::AIR_KICK,
            Attack::Rush => req::RUSH,
            Attack::RushLeap => req::RUSH_LEAP,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Kick => "kick",
            Attack::DoubleKick => "double kick",
            Attack::JumpKick => "jumping kick",
            Attack::SpinKick => "spin kick",
            Attack::AirKick => "air kick",
            Attack::Rush => "rush",
            Attack::RushLeap => "rush leap",
        }
    }

    /// The attack rows (script offsets, 0x70 bytes, `sys 23`) made with it.
    pub fn rows(self) -> &'static [usize] {
        match self {
            Attack::Kick => &[0xEC],
            Attack::DoubleKick | Attack::Rush => &[0xEC, 0x15C],
            Attack::JumpKick | Attack::RushLeap => &[0x1CC],
            Attack::SpinKick => &[0x23C, 0x2AC],
            Attack::AirKick => &[0x31C],
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The names of the Soldier's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    "hop_height",
    "idle_search",
    "close_range",
    "mid_range",
    "close_hits",
    "close_retreat_chance",
    "kick_range",
    "mid_jump_kick_chance",
    "mid_spin_chance",
    "mid_approach_timeout",
    "mid_approach_dist",
    "mid_again_chance",
    "mid_again_jump_kick_chance",
    "far_approach_timeout",
    "far_approach_dist",
    "approach_retarget",
    "pause_range",
    "pause_chance",
    "pause_min",
    "pause_max",
    "idle_blend",
    "strafe_short_chance",
    "strafe_short_min",
    "strafe_short_max",
    "strafe_long_min",
    "strafe_long_max",
    "strafe_near",
    "strafe_turn_near",
    "strafe_turn_far",
    "strafe_speed",
    "strafe_side_chance",
    "strafe_angle",
    "strafe_turn_factor",
    "back_off_min",
    "back_off_max",
    "back_off_timeout",
    "notice_wait_max",
    "notice_blend",
    "spin_wait",
    "double_kick_wait",
    "hit_range",
    "hit_count",
    "hit_hp_ratio",
    "hit_kick_chance",
    "hit_rush_chance",
    "jump_height",
    "jump_max_speed",
    "jump_offset",
    "jump_divisor",
    "jump_steer",
    "jump_kick_after",
    "jump_kick_window",
    "wander_inner",
    "wander_outer",
    "wander_band",
    "wander_timeout",
    "wander_idle_chance",
    "wander_idle_min",
    "wander_idle_max",
    "wander_home_max",
    "appear_fade",
];

/// The Soldier's numbers.
pub type Consts = Table;

/// The Soldier's state objects (`wp+0x20`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 3;
}

/// The Soldier's own work area: first engagement done (`wp+0x3C`), hits taken since the last
/// counter (`wp+0x38`), close-range rounds (`wp+0x44`), spawned by the room (`sys 132` ≠ 0, the
/// Re:CoM battle appearance), the appearance done, Re:CoM's HP ratio at the last hit (`0x1DBC`'s
/// HP / max HP: Re:CoM's HP, decision M10).
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    engaged: bool,
    hits: i32,
    rounds: i32,
    spawned: bool,
    appeared: bool,
    hp_ratio: f32,
}

type Cx = crate::enemy::script::Cx<Consts, Attack, Wp>;
type Task = crate::enemy::script::Task;
pub type Cmd = ECmd<Attack>;

fn k(cx: &Cx, n: &str) -> f32 {
    cx.borrow().c.num(n)
}

/// `0x17EC(request, blend, queue blend)`: an action (bit 24) with the idle queued (`0x40000000`).
/// The queue's blend is the host's (it gives a queued motion the attack blend, 8).
async fn action(cx: &Cx, r: u16, attack: Option<Attack>, blend: f32) {
    push(cx, ECmd::Action { request: r, attack, then: req::IDLE, blend });
    frame().await;
}

/// An action with KH1's request bit 23 (`0xA12`, `0x92C`, `0xB20`, `0x16CA`): the action, request
/// 6 queued, the jump of height 0 (`sys 50` with a zero vector: movement state 2), then the wait
/// while in the air (the action pauses it; the motion's root moves the body: [`Body::held`]).
async fn held_action(cx: &Cx, r: u16, attack: Option<Attack>, blend: f32) {
    push(cx, ECmd::Action { request: r, attack, then: req::AIR, blend });
    push(cx, ECmd::Held);
    let y = pos(cx).y;
    {
        let mut io = cx.borrow_mut();
        let c = io.c.clone();
        io.body.jump(0.0, Vec3::ZERO, y, &c);
    }
    push(cx, ECmd::Jump { height: 0.0, velocity: Vec3::ZERO });
    // The jump sets movement state 2 at once (`0x1402AF960`): the first wait is this frame's
    // yield (the action pauses the script until it ends).
    frame().await;
    wait_airborne(cx).await;
}

/// `0xC08`: the kick.
async fn kick(cx: &Cx) {
    wait_grounded(cx).await;
    face_target(cx);
    let b = k(cx, "attack_blend");
    action(cx, req::KICK, Some(Attack::Kick), b).await;
}

/// `0xBAC`: the double kick, then a wait.
async fn double_kick(cx: &Cx) {
    wait_grounded(cx).await;
    face_target(cx);
    let b = k(cx, "attack_blend");
    action(cx, req::DOUBLE_KICK, Some(Attack::DoubleKick), b).await;
    wait_frames(cx, k(cx, "double_kick_wait")).await;
}

/// `0xB88`: within `kick_range` the kick, else the double kick.
async fn kick_or_double(cx: &Cx) {
    if dist_to_target(cx) - k(cx, "kick_range") < 0.0 {
        kick(cx).await;
    } else {
        double_kick(cx).await;
    }
}

/// `0x9B6`: the spin kick (facing the target at once), then a wait.
async fn spin_kick(cx: &Cx) {
    wait_grounded(cx).await;
    face_target_now(cx);
    action(cx, req::SPIN_KICK, Some(Attack::SpinKick), 0.0).await;
    wait_frames(cx, k(cx, "spin_wait")).await;
}

/// `0xA12`: the jumping kick.
async fn jump_kick(cx: &Cx) {
    wait_grounded(cx).await;
    face_target(cx);
    held_action(cx, req::JUMP_KICK, Some(Attack::JumpKick), k(cx, "attack_blend")).await;
}

/// `0x7C8`, `0xAA0`: the crouch, then the rush (207) and its leap (209).
async fn crouch_and_rush(cx: &Cx) {
    let b = k(cx, "attack_blend");
    action(cx, req::CROUCH, None, b).await;
    wait_grounded(cx).await;
    face_target(cx);
    // 207 as an action with 209 queued (`0x400000D1`); the script yields until it ends.
    push(cx, ECmd::Action { request: req::RUSH, attack: Some(Attack::Rush), then: req::RUSH_LEAP, blend: b });
    frame().await;
    held_action(cx, req::RUSH_LEAP, Some(Attack::RushLeap), 0.0).await;
}

/// `0x92C`: the kick in the air (the jump at a target above).
async fn air_kick(cx: &Cx) {
    face_target(cx);
    held_action(cx, req::AIR_KICK, Some(Attack::AirKick), 0.0).await;
}

/// `0xC5E(timeout, dist)`: run at the target until within `dist`; near it, sometimes a pause in
/// the idle motion.
async fn approach_pause(cx: &Cx, timeout: f32, dist: f32) {
    approach(cx, 1, timeout, k(cx, "approach_retarget"), dist).await;
    if dist_to_target(cx) - k(cx, "pause_range") < 0.0 && rand(cx, "pause") - k(cx, "pause_chance") < 0.0 {
        push(cx, ECmd::Motion { request: req::IDLE, blend: k(cx, "idle_blend") });
        let n = rand_range(cx, k(cx, "pause_min"), k(cx, "pause_max"), "pause time");
        wait_frames(cx, n).await;
    }
}

/// `0xCDE` → `0x1214`: the strafe after a round. A run sideways (90° from the facing, left or
/// right at random), the direction turning back toward the facing by `turn × factor` degrees a
/// frame (`strafe_turn_near` within `strafe_near`, else `strafe_turn_far`), for a short or long
/// time; it stops early when the body leaves the ground.
async fn strafe(cx: &Cx) {
    let n = if rand(cx, "strafe length") - k(cx, "strafe_short_chance") < 0.0 {
        rand_range(cx, k(cx, "strafe_short_min"), k(cx, "strafe_short_max"), "strafe time")
    } else {
        rand_range(cx, k(cx, "strafe_long_min"), k(cx, "strafe_long_max"), "strafe time")
    };
    let mut turn = if dist_to_target(cx) - k(cx, "strafe_near") < 0.0 { k(cx, "strafe_turn_near") } else { k(cx, "strafe_turn_far") };
    // `0x1214`: the facing vector rotated −90° (then turning +) or +90° (then turning −).
    let yaw = cx.borrow().body.yaw;
    let ang = k(cx, "strafe_angle");
    let mut dir = if rand(cx, "strafe side") - k(cx, "strafe_side_chance") < 0.0 {
        yaw - ang.to_radians()
    } else {
        turn = -turn;
        yaw + ang.to_radians()
    };
    turn *= k(cx, "strafe_turn_factor");
    let speed = k(cx, "strafe_speed");
    let blend = cx.borrow().move_blend;
    let mut t = n;
    while t >= 0.0 {
        push(cx, ECmd::Motion { request: req::RUN, blend });
        let step = cx.borrow().w.step;
        dir += (turn * step).to_radians();
        {
            let mut io = cx.borrow_mut();
            io.body.yaw_target = dir;
            io.body.walk = Vec3::from_yaw(dir) * speed;
            io.moving = Some(req::RUN);
        }
        t -= step;
        if !cx.borrow().w.grounded {
            break;
        }
        frame().await;
    }
}

/// `0x11A8(dist, timeout)`: run to the point `dist` away from the target, then face it.
async fn back_off(cx: &Cx) {
    let d = rand_range(cx, k(cx, "back_off_min"), k(cx, "back_off_max"), "back off");
    let Some(t) = target(cx) else { return };
    let p = pos(cx);
    let (dir, _) = (t - p).normalize_len();
    let goal = p - dir * d;
    crate::enemy::script::walk_to(cx, goal, 1, k(cx, "back_off_timeout")).await;
    face_target(cx);
}

/// `0x758`: the first engagement: when the room placed it (not spawned), a look at the target
/// (208, facing it until the motion ends or request 6 plays), then the idle motion.
async fn notice(cx: &Cx) {
    cx.borrow_mut().x.engaged = true;
    if cx.borrow().x.spawned {
        return;
    }
    wait_grounded(cx).await;
    let n = rand_range(cx, 0.0, k(cx, "notice_wait_max"), "notice wait");
    wait_frames(cx, n).await;
    let blend = k(cx, "notice_blend");
    push(cx, ECmd::Motion { request: req::NOTICE, blend });
    // `0x176C`: until `sys 172` (Assumption: the motion has played to its length) or request 6.
    let len = crate::enemy::script::motion_frames(cx, req::NOTICE);
    let mut t = 0.0;
    loop {
        frame().await;
        t += cx.borrow().w.step;
        if t >= len || cx.borrow().w.motion == Some(req::AIR) {
            break;
        }
        face_target(cx);
    }
    push(cx, ECmd::Motion { request: req::IDLE, blend });
}

/// `0x7E0`: a target above (`target.y − y < 0`, y down): the jump at it (`0xFB2`: wait for the
/// ground, face it, hop `jump_height`, speed min((dist − offset) / divisor, max)), steering at it in
/// the air (`+0x20` × `jump_steer`); after `jump_kick_after` frames, within ±`jump_kick_window`
/// of its height, the air kick.
async fn jump_check(cx: &Cx) {
    let (Some(t), p) = (target(cx), pos(cx)) else { return };
    if t.y - p.y >= 0.0 {
        return;
    }
    wait_grounded(cx).await;
    let Some(t) = target(cx) else { return };
    let (dir, len) = (t - pos(cx)).normalize_len();
    cx.borrow_mut().body.yaw_target = dir.x.atan2(dir.z);
    let speed = ((len - k(cx, "jump_offset")) / k(cx, "jump_divisor")).min(k(cx, "jump_max_speed"));
    jump_at(cx, k(cx, "jump_height"), dir * speed).await;
    let mut t = 0.0;
    while cx.borrow().w.airborne || t == 0.0 {
        if let Some(tg) = target(cx) {
            let (d, _) = (tg - pos(cx)).normalize_len();
            let v = d * k(cx, "jump_steer");
            let mut io = cx.borrow_mut();
            io.body.yaw_target = d.x.atan2(d.z);
            io.steer = Some(v);
        }
        if t - k(cx, "jump_kick_after") >= 0.0 {
            if let Some(tg) = target(cx) {
                let dy = tg.y - pos(cx).y;
                let w = k(cx, "jump_kick_window");
                if dy - w <= 0.0 && dy + w >= 0.0 {
                    air_kick(cx).await;
                    return;
                }
            }
        }
        t += cx.borrow().w.step;
        frame().await;
    }
}

/// `0x684`: close range: a kick (or the double kick); after `close_hits` rounds, sometimes a back
/// off and the spin kick; the strafe.
async fn close(cx: &Cx) {
    kick_or_double(cx).await;
    let rounds = cx.borrow().x.rounds;
    if rounds - k(cx, "close_hits") as i32 >= 0 && rand(cx, "back off or not") - k(cx, "close_retreat_chance") < 0.0 {
        cx.borrow_mut().x.rounds = 0;
        back_off(cx).await;
        spin_kick(cx).await;
    }
    strafe(cx).await;
}

/// `0x5D8`: mid range: the jumping kick, the spin kick, or a run in and a kick; the strafe; then
/// often another jumping kick or run in and kick.
async fn mid(cx: &Cx) {
    let r = rand(cx, "mid attack");
    if r - k(cx, "mid_jump_kick_chance") < 0.0 {
        jump_kick(cx).await;
    } else if r - k(cx, "mid_spin_chance") < 0.0 {
        spin_kick(cx).await;
    } else {
        approach_pause(cx, k(cx, "mid_approach_timeout"), k(cx, "mid_approach_dist")).await;
        kick_or_double(cx).await;
    }
    strafe(cx).await;
    if rand(cx, "mid again") - k(cx, "mid_again_chance") < 0.0 {
        if rand(cx, "mid again attack") - k(cx, "mid_again_jump_kick_chance") < 0.0 {
            jump_kick(cx).await;
        } else {
            approach_pause(cx, k(cx, "mid_approach_timeout"), k(cx, "mid_approach_dist")).await;
            kick_or_double(cx).await;
        }
    }
}

/// `0x5B6`: far: run in, then face the target.
async fn far(cx: &Cx) {
    approach_pause(cx, k(cx, "far_approach_timeout"), k(cx, "far_approach_dist")).await;
    face_target(cx);
}

/// The appearance (`0x716` → `0x16CA`, a Soldier the room spawns): the fade-in, facing Sora at
/// once, request 69 with bit 23 (request 6 queued), the zero jump, the wait while in the air; then
/// the idle state.
async fn appear(cx: Cx) {
    if cx.borrow().x.spawned {
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        let to = {
            let io = cx.borrow();
            io.w.target.map(|t| t - io.w.pos)
        };
        if let Some(d) = to {
            let y = d.x.atan2(d.z);
            let mut io = cx.borrow_mut();
            io.body.yaw = y;
            io.body.yaw_target = y;
            io.out.push(ECmd::Yaw(y));
        }
        held_action(&cx, req::APPEAR, None, k(&cx, "attack_blend")).await;
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The idle behaviour (`0x6D2` → `0x6EA`): forever wander about the current place.
async fn idle(cx: Cx) {
    loop {
        let c = |n: &str| k(&cx, n);
        let (inner, outer, band, timeout) = (c("wander_inner"), c("wander_outer"), c("wander_band"), c("wander_timeout"));
        let (chance, lo, hi, home, blend) = (c("wander_idle_chance"), c("wander_idle_min"), c("wander_idle_max"), c("wander_home_max"), c("idle_blend"));
        wander(&cx, inner, outer, band, timeout, chance, lo, hi, home, blend).await;
        frame().await;
    }
}

/// The engaged behaviour (`0x4D2` → `0x4F6`): the first look, by distance close / mid / far, the
/// jump at a target above, then engaged again (the supervisor restarts it).
async fn engaged(cx: Cx) {
    if !cx.borrow().x.engaged {
        notice(&cx).await;
    }
    cx.borrow_mut().x.rounds += 1;
    let d = dist_to_target(&cx);
    if d - k(&cx, "close_range") < 0.0 {
        close(&cx).await;
    } else if d - k(&cx, "mid_range") < 0.0 {
        mid(&cx).await;
    } else {
        far(&cx).await;
    }
    jump_check(&cx).await;
    // `0x58C`: the engaged search (`sys 48`, 5000) or Sora: Re:CoM's battles have only Sora.
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
}

/// The hit behaviour (`0x3E8` → `0x40C`): the counter. Close: after `hit_count` counted hits, the
/// spin kick below `hit_hp_ratio` of its HP, else a kick; otherwise a kick or the spin kick. Far:
/// sometimes the crouch and the rush. Then engaged.
async fn hit(cx: Cx) {
    if dist_to_target(&cx) - k(&cx, "hit_range") < 0.0 {
        if cx.borrow().x.hits - k(&cx, "hit_count") as i32 >= 0 {
            cx.borrow_mut().x.hits = 0;
            if cx.borrow().x.hp_ratio - k(&cx, "hit_hp_ratio") < 0.0 {
                spin_kick(&cx).await;
            } else {
                kick_or_double(&cx).await;
            }
        } else if rand(&cx, "counter") - k(&cx, "hit_kick_chance") < 0.0 {
            kick_or_double(&cx).await;
        } else {
            spin_kick(&cx).await;
        }
    } else if rand(&cx, "rush") - k(&cx, "hit_rush_chance") < 0.0 {
        crouch_and_rush(&cx).await;
    }
    set_state(&cx, st::ENGAGED);
}

/// The supervisor (`0x2152`): after the init frame, each frame: while idle, Sora within
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
                let near = (t - me).length() - k(&cx, "idle_search") < 0.0;
                if near {
                    set_state(&cx, st::ENGAGED);
                    set_target(&cx, true);
                }
            }
        }
        if let Some(s) = take_state(&cx) {
            match s {
                st::IDLE => set_target(&cx, false),
                _ => set_target(&cx, true),
            }
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Soldier's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as the Shadow's: per-enemy seeds).
    /// `frames`: KH1 motion lengths by request. `spawned`: the room spawned it (its appearance
    /// plays; every Re:CoM battle appearance, as the Shadow's).
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, spawned: bool) -> Ai {
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp { spawned, hp_ratio: 1.0, ..Default::default() });
        Ai { io: Rc::new(RefCell::new(io)), main: None, behaviour: None, started: false }
    }

    pub fn consts(&self) -> Consts {
        self.io.borrow().c.clone()
    }

    /// Re:CoM's HP / max HP after a hit (the counter's HP rule `0x1DBC`).
    pub fn set_hp_ratio(&self, r: f32) {
        self.io.borrow_mut().x.hp_ratio = r;
    }

    /// One AI frame (the host calls it only while no action runs). Returns the commands.
    pub fn frame(&mut self, w: World) -> Vec<Cmd> {
        self.io.borrow_mut().w = w;
        if !self.started {
            self.started = true;
            {
                let mut io = self.io.borrow_mut();
                // `0x2298`: home = the position, the state object cleared.
                io.home = w.pos;
                io.state = (st::NONE, false);
            }
            self.main = Some(Box::pin(supervisor(self.io.clone())));
            // `0x2166`: after the init's yield, the appearance thread (`0x706` → `0x716`); the
            // supervisor's first frame is the init's.
            self.behaviour = Some(Box::pin(async {}));
        }
        poll(&mut self.main);
        if self.behaviour.is_none() && !self.io.borrow().x.appeared && self.io.borrow().state.0 == st::NONE {
            // Second frame: the appearance starts (it runs the same frame, `0x1402BD640`).
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
    /// The hit handler (`0x234C`): every hit ends the first look (`wp+0x3C` = 1); a damaging hit
    /// counts (`wp+0x38` += 1); both ask for the counter (state 3).
    fn on_hit(&self, news: HitNews) {
        {
            let mut io = self.io.borrow_mut();
            io.x.engaged = true;
            if news == HitNews::Damage {
                io.x.hits += 1;
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
        Ai::set_hp_ratio(self, r)
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["none", "idle", "engaged", "hit"].get(s as usize).copied().unwrap_or("?")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consts() -> Consts {
        // Test numbers (the script's own come from the player's file at the asset step).
        let v: &[(&str, f32)] = &[
            ("hop_height", 300.0), ("idle_search", 1000.0), ("close_range", 350.0), ("mid_range", 450.0), ("close_hits", 3.0),
            ("close_retreat_chance", 0.7), ("kick_range", 280.0), ("mid_jump_kick_chance", 0.4), ("mid_spin_chance", 0.8),
            ("mid_approach_timeout", 120.0), ("mid_approach_dist", 350.0), ("mid_again_chance", 0.8), ("mid_again_jump_kick_chance", 0.8),
            ("far_approach_timeout", 300.0), ("far_approach_dist", 450.0), ("approach_retarget", 20.0), ("pause_range", 450.0),
            ("pause_chance", 0.3), ("pause_min", 30.0), ("pause_max", 60.0), ("idle_blend", 8.0), ("strafe_short_chance", 0.3),
            ("strafe_short_min", 30.0), ("strafe_short_max", 45.0), ("strafe_long_min", 60.0), ("strafe_long_max", 75.0),
            ("strafe_near", 350.0), ("strafe_turn_near", 2.0), ("strafe_turn_far", 1.0), ("strafe_speed", 7.0), ("strafe_side_chance", 0.5),
            ("strafe_angle", 90.0), ("strafe_turn_factor", 0.5), ("back_off_min", 150.0), ("back_off_max", 200.0), ("back_off_timeout", 180.0),
            ("notice_wait_max", 12.0), ("notice_blend", 8.0), ("spin_wait", 8.0), ("double_kick_wait", 8.0), ("hit_range", 450.0),
            ("hit_count", 3.0), ("hit_hp_ratio", 0.5), ("hit_kick_chance", 0.6), ("hit_rush_chance", 0.6), ("jump_height", 200.0),
            ("jump_max_speed", 20.0), ("jump_offset", 50.0), ("jump_divisor", 60.0), ("jump_steer", 20.0), ("jump_kick_after", 37.0),
            ("jump_kick_window", 30.0), ("wander_inner", 300.0), ("wander_outer", 400.0), ("wander_band", 100.0), ("wander_timeout", 300.0),
            ("wander_idle_chance", 0.4), ("wander_idle_min", 90.0), ("wander_idle_max", 180.0), ("wander_home_max", 700.0), ("appear_fade", 30.0),
            // The shared engine's.
            ("walk_speed", 3.0), ("run_speed", 7.0), ("air_accel", 0.1), ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0),
            ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0),
            ("air_knock_speed", 16.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("weight", 1.0), ("gravity", crate::params::GRAVITY),
            ("fall_cap", 20.0), ("attack_blend", 8.0),
        ];
        Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), NAMES).unwrap()
    }

    fn frames() -> Vec<(u16, f32)> {
        vec![(req::APPEAR, 86.0), (req::NOTICE, 84.0), (req::KICK, 110.0), (req::SPIN_KICK, 100.0), (req::CROUCH, 200.0)]
    }

    /// A flat floor at y = 0, Sora at `sora`; an action keeps the AI paused for 30 frames.
    struct Sim {
        ai: Ai,
        pos: Vec3,
        busy: f32,
        log: Vec<Cmd>,
    }

    impl Sim {
        fn run(&mut self, sora: Vec3, n: usize) {
            for _ in 0..n {
                let airborne = self.ai.body().airborne();
                let w = World { step: 1.0, pos: self.pos, target: Some(sora), grounded: !airborne, airborne, moved: 5.0, motion: None };
                if self.busy > 0.0 {
                    self.busy -= 1.0;
                    if self.busy <= 0.0 {
                        self.ai.body().held = false;
                    }
                } else {
                    for c in self.ai.frame(w) {
                        match c {
                            ECmd::Action { .. } => self.busy = 30.0,
                            ECmd::Held => self.ai.body().held = true,
                            _ => {}
                        }
                        self.log.push(c);
                    }
                }
                let c = self.ai.consts();
                let steer = self.ai.io.borrow_mut().steer.take();
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
        Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: Vec3::ZERO, busy: 0.0, log: Vec::new() }
    }

    /// A spawned Soldier past its appearance, idle, Sora far away.
    fn ready(seed: u32) -> Sim {
        let mut s = sim(seed, true);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 50);
        assert!(s.ai.risen());
        s.log.clear();
        s
    }

    fn attacks(s: &Sim) -> Vec<Attack> {
        s.log.iter().filter_map(|c| if let ECmd::Action { attack: Some(a), .. } = c { Some(*a) } else { None }).collect()
    }

    #[test]
    fn the_names_cover_the_numbers() {
        assert!(Table::new(vec![], NAMES).is_err());
        let c = consts();
        assert_eq!((c.num("close_range"), c.num("kick_range")), (350.0, 280.0));
    }

    #[test]
    fn spawned_appearance_then_idle() {
        let mut s = sim(1, true);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 3);
        assert!(s.log.contains(&ECmd::FadeIn(30.0)), "{:?}", s.log);
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Action { request: 69, then: 6, attack: None, .. })));
        assert!(s.log.contains(&ECmd::Held));
        assert!(!s.log.iter().any(|c| matches!(c, ECmd::State(_))), "nothing until the appearance ends");
        s.run(Vec3::new(0.0, 0.0, 5000.0), 40);
        assert!(s.ai.risen());
        assert!(s.log.contains(&ECmd::State(st::IDLE)));
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Motion { request: req::WALK, .. })), "it wanders");
    }

    #[test]
    fn close_target_kicks_and_counters() {
        // Sora 2 m away (< 280): the kick, from the engaged behaviour.
        let mut s = sim(3, false);
        s.run(Vec3::new(0.0, 0.0, 200.0), 200);
        assert!(s.log.contains(&ECmd::State(st::ENGAGED)));
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Motion { request: req::NOTICE, .. })), "a placed Soldier looks first");
        assert_eq!(attacks(&s).first(), Some(&Attack::Kick), "{:?}", s.log);
        // A hit asks for the counter (state 3).
        s.ai.on_hit(HitNews::Damage);
        s.run(Vec3::new(0.0, 0.0, 200.0), 60);
        assert!(s.log.contains(&ECmd::State(st::HIT)));
    }

    #[test]
    fn mid_range_picks_kicks_and_spins() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..40 {
            let mut s = ready(seed);
            s.run(Vec3::new(0.0, 0.0, 400.0), 120);
            for a in attacks(&s) {
                seen.insert(a);
            }
        }
        assert!(seen.contains(&Attack::JumpKick) && seen.contains(&Attack::SpinKick), "{seen:?}");
    }

    #[test]
    fn far_target_runs_in() {
        let mut s = ready(2);
        s.run(Vec3::new(0.0, 0.0, 900.0), 30);
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Motion { request: req::RUN, .. })), "{:?}", s.log);
        assert!(s.pos.z > 100.0, "runs at 7 a frame: {:?}", s.pos);
    }

    #[test]
    fn a_target_above_gets_the_jump_and_the_air_kick() {
        let mut s = ready(5);
        // Sora 3 m up and 6 m away (far: runs in, then the jump check).
        s.run(Vec3::new(0.0, -150.0, 600.0), 400);
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Jump { height, .. } if *height == 200.0)), "{:?}", s.log);
    }

    #[test]
    fn far_counter_crouches_and_rushes() {
        for seed in 0..30 {
            let mut s = ready(seed);
            s.ai.on_hit(HitNews::Clash);
            s.run(Vec3::new(0.0, 0.0, 800.0), 200);
            let a = attacks(&s);
            if a.contains(&Attack::Rush) {
                assert!(s.log.iter().any(|c| matches!(c, ECmd::Action { request: req::CROUCH, .. })));
                assert!(s.log.iter().any(|c| matches!(c, ECmd::Action { attack: Some(Attack::Rush), then: req::RUSH_LEAP, .. })));
                assert!(a.contains(&Attack::RushLeap), "{a:?}");
                return;
            }
        }
        panic!("no seed rushed");
    }

    #[test]
    fn held_actions_hang_the_body() {
        let c = consts();
        let mut b = Body::default();
        b.jump(0.0, Vec3::ZERO, -50.0, &c);
        b.held = true;
        for _ in 0..10 {
            let d = b.step(1.0, -50.0, None, &c);
            assert_eq!(d.y, 0.0);
        }
        assert!(!b.falling());
        b.held = false;
        assert!(b.falling(), "falls once the motion ends");
        assert!(b.step(1.0, -50.0, None, &c).y > 0.0);
    }

    #[test]
    fn attack_rows_and_indices() {
        assert_eq!(Attack::Rush.rows(), &[0xEC, 0x15C]);
        assert_eq!(Attack::RushLeap.index(), 6);
        assert_eq!(req::LAND, 7);
    }

    /// A long fight (owner 2026-11-02: enemies froze until hit): Sora circling 2–7 m away, a hit
    /// every 150–400 frames, each action and reaction holding the AI for its motion's length.
    /// The AI must keep acting (an action, a motion of its own or a state change) at least every
    /// 600 frames: a frozen AI emits nothing.
    #[test]
    fn a_long_fight_never_stalls() {
        for seed in 0..12u32 {
            let mut ai = Ai::new(consts(), frames(), seed, 0.0, true);
            let mut rng = crate::rng::Kh1Rand::new(seed ^ 0x5EED);
            let (mut pos, mut busy, mut last_action, mut next_hit) = (Vec3::ZERO, 0.0f32, 0usize, 200usize);
            let mut worst = 0usize;
            for f in 0..20_000usize {
                let a = f as f32 / 600.0;
                let r = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
                let sora = Vec3::new(a.sin() * r, 0.0, a.cos() * r);
                let airborne = ai.body().airborne();
                let w = World { step: 1.0, pos, target: Some(sora), grounded: !airborne, airborne, moved: 5.0, motion: None };
                if f == next_hit {
                    ai.on_hit(crate::enemy::HitNews::Damage);
                    busy = 34.0;
                    ai.body().held = false;
                    next_hit = f + 150 + (rng.next_f32() * 250.0) as usize;
                }
                if busy > 0.0 {
                    busy -= 1.0;
                    if busy <= 0.0 {
                        ai.body().held = false;
                    }
                } else {
                    for c in ai.frame(w) {
                        // Acting: an action, a motion of its own (a taunt, a sink, a rise), a state
                        // change; not the per-frame facing, walk or height.
                        if !matches!(c, Cmd::Yaw(_) | Cmd::Height(_) | Cmd::Motion { request: 1 | 2, .. }) {
                            worst = worst.max(f - last_action);
                            last_action = f;
                        }
                        match c {
                            Cmd::Action { request, .. } => {
                                busy = frames().iter().find(|x| x.0 == request).map_or(40.0, |x| x.1);
                            }
                            Cmd::Held => ai.body().held = true,
                            _ => {}
                        }
                    }
                }
                let c = ai.consts();
                let steer = ai.take_steer();
                let d = ai.body().step(1.0, pos.y, steer, &c);
                pos += d;
                if pos.y >= 0.0 && ai.body().falling() {
                    pos.y = 0.0;
                    ai.body().land();
                }
            }
            worst = worst.max(20_000 - last_action);
            assert!(worst < 600, "seed {seed}: {worst} frames without acting");
        }
    }
}
