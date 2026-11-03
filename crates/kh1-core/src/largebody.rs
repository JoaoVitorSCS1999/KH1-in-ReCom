//! KH1 FM's Large Body (`xa_ex_2050.mdls` part 5, `ex_2050.bd`; its child `ex_2050_00.bd`, part 3), engine-agnostic:
//! its AI script on the shared enemy framework (`crate::enemy`). M11 (`docs/recom-m11-enemies.md` § The Large Body).
//! Re:CoM runs it natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), Re:CoM
//! plays the motions, collides the body, the attacks and the front guard, applies the damage.
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's scripts at checked offsets); nothing of
//! the game is in this file. Script offsets are the player's `ex_2050.bd`'s (Checked (files): the script disassembled
//! with `tools/kh1-ai-script.py`, code from `0x458`; the VM's ops and the syscalls named, Checked (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x1FF8`: init `0x20DC` (records LV1 `0x14` / LV2 `0x80`; walk 3.5, run 5; turn rate 8° a frame; home;
//!   `wp+0x48` = 0 (not enraged), `+0x54` = 4 (the first look pending), `+0x58` = 7 (no charge); **the front guard
//!   row `0x38C`** (`0x9CA`: `sys 23` then `sys 213(row, 3)`, mode 3: on every frame until removed); the state
//!   object; the search object; the handlers: hit `0x21BE`), a yield, the appearance thread `0x682`; then each frame:
//!   while idle the search (Sora within 1500; KH1's world 11: 10000 and always) → engaged with the target; a state
//!   request restarts the behaviour thread: 1 idle `0x636`, 2 engaged `0x53A`, 3 hit `0x47C`.
//! - the hit handler `0x21BE` ([`Ai`]'s `on_hit`): every hit cancels the first look (`wp+0x54` = 5). A clash (kinds
//!   6 / 7) or a repel record (`hp[0] & 0x20`): during the charge (207) the charge ends (`wp+0x58` = 9) and the front
//!   row comes back (`0x908`); else the count (`wp+0x50`) + 1 and the hit state (`wp+0x4C` = 2). A damaging hit that
//!   leaves it alive: not yet enraged and under 0.4 of its HP → `sys 42` (no reaction, the push slide), actor flag 4
//!   cleared (its action ends now), enraged, the hit state (`wp+0x4C` = 3: the rage); during the charge → the charge
//!   ends (8), the front row back, **`sys 79`** (`0x1402BBF40` → `0x1402B77D0`: KH1's knock-back whatever the hit's
//!   kind) and the hit state; a knock-back or launch (kinds 4 / 5): during the stomp 230, the slam 231 or the rage 217
//!   `sys 42`, else `sys 79` and the hit state; the count + 1 either way; **any other hit (a flinch): `sys 42`, no
//!   reaction and no state**. The killing hit: its death `0x24EA` (Re:CoM's defeat here); KH1 then plays the hit's
//!   own reaction.
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0xBEA` (`0xEC2(1800, 1080)`: KH1 removes it 18 m
//! from Sora, 10.8 m out of sight; never in world 11), the sleeping spawn (`sys 132` = 30: 211 / 212 / 213), the
//! death routine `0x24EA` / `0x13A8` (Re:CoM's defeat, prizes, EXP), KH1's effects (`sys 75` 2 / 9 / 10 / 13 / 14,
//! the ring's 7: M13), the appearance's flag 0x80 (`0x1E12`), the fade-in (no Re:CoM alpha), the hit attributes 8..11
//! (`0x1F5A`: every hit of Sora's is taken as type 0 / attribute 0, as the Pirate's), the search's sight cone
//! (`sys 48`; its fallback finds Sora within the range anyway) and `sys 139`'s out-of-sight test (taken as in
//! sight).

use crate::enemy::guard::ALWAYS;
use crate::enemy::script::{approach, dist_to_target, face_target, frame, motion_done, poll, pos, push, rand_range, random_offset, set_state, set_target, set_turn_rate, take_state, target, tick_clock, wait_frames, wait_grounded, walk_to, wander};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, Engine, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Large Body's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The appearance (`0x1B30`: 69, a plain request, blend 0), 100 frames.
    pub const APPEAR: u16 = 69;
    /// The punch (`0xAB4`: action 200), 158 frames, both hands, hits [50, 72) / [54, 68) and [84, 106) / [86, 100).
    pub const PUNCH: u16 = 200;
    /// The charge's wind-up (`0x6D8`), 92 frames.
    pub const WINDUP: u16 = 206;
    /// The charge (`sys 12` each frame), 24 frames looped, its belly row on throughout.
    pub const CHARGE: u16 = 207;
    /// After the charge: it hit Sora (208, action, 116 frames) or didn't (209, action, 296 frames).
    pub const CHARGE_HIT: u16 = 208;
    pub const CHARGE_MISS: u16 = 209;
    /// The first look on engaging (`0xC30`), 92 frames.
    pub const LOOK: u16 = 214;
    /// The recoils its rows name: the punch's 215, the slam's 216, the charge's 218 (all in the model; the
    /// stomp ring's 110 is not).
    pub const PUNCH_RECOIL: u16 = 215;
    pub const SLAM_RECOIL: u16 = 216;
    pub const CHARGE_RECOIL: u16 = 218;
    /// The rage (`0xCC2`), 128 frames (looped).
    pub const RAGE: u16 = 217;
    /// The stomp (`0xA08`, a plain request), 182 frames; its ring (the child) from frame 88.
    pub const STOMP: u16 = 230;
    /// The slam (`0xA74`: action 231), 272 frames, root motion, hit [102, 120).
    pub const SLAM: u16 = 231;
}

/// The motions a knock-back or launch doesn't interrupt (`0x2442`: `sys 51` against these; then `sys 42`).
pub const STEADY: [u16; 3] = [req::STOMP, req::SLAM, req::RAGE];

/// The Large Body's attacks, in the order the host indexes them (the manifest's `attacks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    Punch,
    Slam,
    Charge,
    /// The stomp: its ring is the child's row `0x31C`, on Re:CoM's own shockwave record (the asset step's
    /// `enemy::RINGS`; the host grows its radius by the ring's numbers).
    Stomp,
}

impl Attack {
    pub const ALL: [Attack; 4] = [Attack::Punch, Attack::Slam, Attack::Charge, Attack::Stomp];

    pub fn request(self) -> u16 {
        match self {
            Attack::Punch => req::PUNCH,
            Attack::Slam => req::SLAM,
            Attack::Charge => req::CHARGE,
            Attack::Stomp => req::STOMP,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Punch => "punch",
            Attack::Slam => "slam",
            Attack::Charge => "charge",
            Attack::Stomp => "stomp",
        }
    }

    /// The attack rows (script offsets, 0x70 bytes, `sys 23`) made with it; the stomp's is its child's.
    pub fn rows(self) -> &'static [usize] {
        match self {
            Attack::Punch => &[0xEC, 0x15C],
            Attack::Slam => &[0x1CC],
            Attack::Charge => &[0x23C],
            Attack::Stomp => &[0x31C],
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The Large Body's guard-type rows (the manifest's `guards`): its belly's front row `0x38C` (bone code 12, 100,
/// type 1, priority 100, recoil 110, cone turn 0 / cos 0.342 = 70°; mode 3).
pub const FRONT: u8 = 0;

/// The names of the Large Body's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    "turn_deg",
    "idle_search",
    "world11_search",
    "close_range",
    "mid_range",
    "wander_inner",
    "wander_outer",
    "wander_band",
    "wander_timeout",
    "wander_idle_chance",
    "idle_blend",
    "wander_idle_min",
    "wander_idle_max",
    "wander_home_max",
    "windup_blend",
    "charge_time",
    "charge_near",
    "charge_past_min",
    "charge_past_max",
    "charge_arrive",
    "charge_stuck",
    "stuck_factor",
    "charge_blend",
    "charge_speed",
    "landed_blend",
    "landed_then",
    "landed_min",
    "landed_max",
    "missed_blend",
    "missed_then",
    "missed_min",
    "missed_max",
    "stomp_request",
    "stomp_blend",
    "ring_at",
    "ring_life",
    "ring_growth",
    "ring_band",
    "ring_height",
    "slam_blend",
    "slam_then",
    "slam_wait",
    "punch_blend",
    "punch_then",
    "punch_wait",
    "near_range",
    "away_min",
    "away_max",
    "away_time_min",
    "away_time_max",
    "far_range",
    "approach_min",
    "approach_max",
    "approach_retarget",
    "approach_dist",
    "look_blend",
    "look_end_blend",
    "enrage_blend",
    "enrage_end_blend",
    "appear_blend",
    "appear_fade",
    "appear_wait",
    "appear_then_blend",
    "enrage_hp",
    "hit_stomp_count",
];

/// The Large Body's numbers.
pub type Consts = Table;

/// The Large Body's state objects (`wp+0x20`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 3;
}

/// KH1's world 11 (`sys 105`'s id; Olympus Coliseum): the search reaches 100 m and always finds Sora (`0x11B8`).
pub const OLYMPUS: u8 = 11;

/// The charge's run (`wp+0x58`): 6 charging, 7 over, 8 a damaging hit ended it, 9 a clash or repel ended it, 10 its
/// row landed on Sora (the row's callback `0x968`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChargeRun {
    Charging,
    #[default]
    Over,
    Hit,
    Clashed,
    Landed,
}

/// What the hit state does (`wp+0x4C`): 2 a counter (the punch, the stomp or the charge), 3 the rage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Next {
    #[default]
    None,
    Counter,
    Rage,
}

/// The Large Body's own work area: enraged (`wp+0x48`), the first look pending (`wp+0x54` = 4), the charge's run
/// (`wp+0x58`), the counted hits (`wp+0x50`), the hit state's choice (`wp+0x4C`), the front row made (`wp+0x60`);
/// spawned by the room (`sys 132`), the appearance done, paused by it (flag 4 while 69 plays), KH1's world 11, the
/// last hit's kind, push direction and the HP left (Re:CoM's ratio), what its handler did to the hit (no reaction,
/// KH1's knock-back), flag 4 cleared, defeated, the commands its handler makes at once.
#[derive(Clone, Debug)]
struct Wp {
    enraged: bool,
    look: bool,
    charge: ChargeRun,
    counted: i32,
    next: Next,
    front: bool,
    spawned: bool,
    appeared: bool,
    paused: bool,
    olympus: bool,
    hit_kind: u8,
    hit_dir: Vec3,
    hp: f32,
    no_reaction: bool,
    knock: bool,
    cut: bool,
    dead: bool,
    now: Vec<ECmd<Attack>>,
    /// The request playing as the hit handler sees it (`sys 51`; [`Behaviour::set_playing`]).
    cur: Option<u16>,
}

impl Default for Wp {
    /// `0x20DC`: `wp+0x48` = 0, `+0x54` = 4, `+0x58` = 7; `+0x50`, `+0x4C`, `+0x60` start 0.
    fn default() -> Wp {
        Wp { enraged: false, look: true, charge: ChargeRun::Over, counted: 0, next: Next::None, front: false, spawned: false, appeared: false, paused: false, olympus: false, hit_kind: 0, hit_dir: Vec3::ZERO, hp: 1.0, no_reaction: false, knock: false, cut: false, dead: false, now: Vec::new(), cur: None }
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

/// A plain motion request (`sys 12` / `sys 13` without bit 24); `sys 172`'s clock restarts for a request that
/// plays anew.
fn play(cx: &Cx, r: u16, blend: f32) {
    {
        let mut io = cx.borrow_mut();
        if io.w.motion != Some(r) {
            io.clock = (Some(r), 0.0);
        }
    }
    push(cx, ECmd::Motion { request: r, blend });
}

/// The request playing as the script sees it (`sys 51`): the last one this frame asked for, else the host's.
fn playing(cx: &Cx) -> Option<u16> {
    let io = cx.borrow();
    io.out.iter().rev().find_map(|c| match c {
        ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Attack { request, .. } | ECmd::Action { request, .. } => Some(*request),
        _ => None,
    }).or(io.w.motion)
}

/// `sys 12` / `sys 13` of an attack's request (its rows made just before): a plain request, the script keeps
/// running; `sys 12` does nothing when it already plays (`0x14029F7C0`).
fn attack_keep(cx: &Cx, a: Attack, blend: f32) {
    if playing(cx) != Some(a.request()) {
        cx.borrow_mut().clock = (Some(a.request()), 0.0);
        push(cx, ECmd::Attack { request: a.request(), attack: a, blend });
    }
}

/// `0x15B2(blend after, blend, r)`: an action (bit 24) with the idle queued (bit 30), then the yield (the action
/// pauses the script until it ends). The queue's own blend is the host's (`attack_blend`, the scripts' 8).
async fn action(cx: &Cx, r: u16, attack: Option<Attack>, blend: f32) {
    push(cx, ECmd::Action { request: r, attack, then: req::IDLE, blend });
    frame().await;
}

/// `0x1532`: until the motion is done (`sys 172`) or the air motion plays (6), turning to the target each frame.
async fn wait_done_turning(cx: &Cx) {
    loop {
        if motion_done(cx) || cx.borrow().w.motion == Some(req::AIR) {
            break;
        }
        face_target(cx);
        frame().await;
    }
}

/// `0x1C00`: a yield, then until the motion is done (`sys 172`).
async fn wait_done(cx: &Cx) {
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

/// `0x9CA`: the front row made again when it isn't (`wp+0x60` = 0 → 1): `sys 23` with `0x38C`, `sys 213(row, 3)`.
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

/// `0x978`: with the charge's row made, the front row removed (`wp+0x60` = 0, `sys 213(row, 0)`).
fn front_off(cx: &Cx) {
    let mut io = cx.borrow_mut();
    if io.x.front {
        io.x.front = false;
        io.out.push(ECmd::Guard(None));
    }
}

/// `sys 11` toward Sora (`0x1C64(actor, party 0)`): the facing at once.
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

/// `0x11B8(&wp30, 0, range, 1)`: the target search. In KH1's world 11 the range is `world11_search` and Sora is
/// always found; else Sora within the range (Assumption: KH1's search `sys 48` finds him within its range; its
/// fallback `func_1982` gives him within it anyway; `sys 139`'s out-of-sight test is taken as in sight).
fn search(cx: &Cx, range: f32) -> Option<Vec3> {
    let io = cx.borrow();
    let t = io.w.target?;
    let r = if io.x.olympus { io.c.num("world11_search") } else { range };
    (io.x.olympus || (t - io.w.pos).length() - r < 0.0).then_some(t)
}

/// `0xB0C`: where it stands before an attack. Walking (or running once enraged): within 400 of the target, to a
/// random point 400–600 from where it stands for 120–150 frames; farther than 450, the shared approach (240–300
/// frames, the target re-read every 5, until within 350); the leash `0xBEA` (not ported).
async fn position(cx: &Cx) {
    let mode = if cx.borrow().x.enraged { 1 } else { 0 };
    if dist_to_target(cx) - k(cx, "near_range") < 0.0 {
        let p = pos(cx) + random_offset(cx, k(cx, "away_min"), k(cx, "away_max"));
        let t = rand_range(cx, k(cx, "away_time_min"), k(cx, "away_time_max"), "away time");
        walk_to(cx, p, mode, t).await;
    }
    if dist_to_target(cx) - k(cx, "far_range") > 0.0 {
        let t = rand_range(cx, k(cx, "approach_min"), k(cx, "approach_max"), "approach time");
        approach(cx, mode, t, k(cx, "approach_retarget"), k(cx, "approach_dist")).await;
    }
}

/// `0xAB4`: the punch: turning to the target (`sys 36`), both hands' rows (`0xEC` / `0x15C`, one hit bit) and action
/// 200 (blend 8, the idle queued); then 75 frames.
async fn punch(cx: &Cx) {
    face_target(cx);
    let b = k(cx, "punch_blend");
    action(cx, req::PUNCH, Some(Attack::Punch), b).await;
    wait_frames(cx, k(cx, "punch_wait")).await;
}

/// `0xA74`: the slam: its belly row `0x1CC`, turning to the target, action 231 (blend 8, the idle queued); then 60
/// frames.
async fn slam(cx: &Cx) {
    face_target(cx);
    let b = k(cx, "slam_blend");
    action(cx, req::SLAM, Some(Attack::Slam), b).await;
    wait_frames(cx, k(cx, "slam_wait")).await;
}

/// `0xA08`: the stomp: on the ground, turning to the target, 230 (blend 0, a plain request); 88 frames; its effect
/// 13 (M13) and the ring (`sys 94`: the child `ex_2050_00.bd` with the row `0x31C`: Re:CoM's shockwave record on the
/// clip from frame `ring_at`, grown by the host); until 230 is done.
async fn stomp(cx: &Cx) {
    wait_grounded(cx).await;
    face_target(cx);
    attack_keep(cx, Attack::Stomp, k(cx, "stomp_blend"));
    wait_frames(cx, k(cx, "ring_at")).await;
    push(cx, ECmd::Note("the stomp's ring (KH1's child, effect 13): Re:CoM's shockwave record grown by KH1's numbers"));
    wait_done(cx).await;
}

/// `0xC30`: the first look on engaging: `wp+0x54` = 5; 214 (blend 0, a plain request; the sleeping spawn's 211 →
/// 213 is not taken) until it is done, turning to the target; the idle (blend 8).
async fn first_look(cx: &Cx) {
    cx.borrow_mut().x.look = false;
    play(cx, req::LOOK, k(cx, "look_blend"));
    wait_done_turning(cx).await;
    play(cx, req::IDLE, k(cx, "look_end_blend"));
}

/// `0xCC2`: the rage: its effect 9 (M13); 217 (blend 8) until it is done, turning to the target; the idle (blend 8).
async fn rage(cx: &Cx) {
    play(cx, req::RAGE, k(cx, "enrage_blend"));
    wait_done_turning(cx).await;
    play(cx, req::IDLE, k(cx, "enrage_end_blend"));
}

/// `0x6D8`: the charge. On the ground; the wind-up 206 (blend 8) until it is done, turning to the target; the
/// charge's belly row `0x23C` (mode 2, its callback `0x968`: `wp+0x58` = 10) and the front row removed (`0x978`);
/// `wp+0x58` = 6; for at most 180 frames while it is 6: at the target until within 100 of the floor under him
/// (`0x12F6`), then on to a point 300–500 from there until within 100 of it or stuck (moved under 3.6 × step × 0.5
/// last frame, `sys 118`); each frame 207 (`sys 12`, blend 8) and the glide at the stats block's `+0xC` = 12
/// (`sys 40` mode 2); ended when 207 no longer plays. Then the front row back (`0x908`), on the ground, 208 (blend
/// 0) and 45–120 frames when its row hit Sora, else 209 (blend 8) and 30–150 frames; `wp+0x58` = 7.
async fn charge(cx: &Cx) {
    let mut past = false;
    wait_grounded(cx).await;
    play(cx, req::WINDUP, k(cx, "windup_blend"));
    wait_done_turning(cx).await;
    front_off(cx);
    cx.borrow_mut().x.charge = ChargeRun::Charging;
    let mut t = k(cx, "charge_time");
    let mut goal = pos(cx);
    let (speed, blend) = (k(cx, "charge_speed"), k(cx, "charge_blend"));
    while t - 0.0 > 0.0 {
        if cx.borrow().x.charge != ChargeRun::Charging {
            break;
        }
        let me = pos(cx);
        if !past {
            goal = target(cx).unwrap_or(me);
            // `0x12F6(target, 0)`: the floor under him (a line test down from 20 above his feet); Re:CoM's battle
            // floors are flat (Assumption): the floor is its own height.
            let under = Vec3::new(goal.x, me.y, goal.z);
            if (under - me).length() - k(cx, "charge_near") < 0.0 {
                past = true;
                goal = me + random_offset(cx, k(cx, "charge_past_min"), k(cx, "charge_past_max"));
            }
        } else {
            let stuck = cx.borrow().w.moved - k(cx, "charge_stuck") * step(cx) * k(cx, "stuck_factor") < 0.0;
            if (goal - me).length() - k(cx, "charge_arrive") < 0.0 || stuck {
                break;
            }
        }
        attack_keep(cx, Attack::Charge, blend);
        crate::enemy::script::move_toward(cx, goal, 2, speed);
        t -= step(cx);
        frame().await;
        if cx.borrow().w.motion != Some(req::CHARGE) {
            break;
        }
    }
    {
        let mut io = cx.borrow_mut();
        front_on(&mut io, false);
    }
    wait_grounded(cx).await;
    if cx.borrow().x.charge == ChargeRun::Landed {
        let b = k(cx, "landed_blend");
        action(cx, req::CHARGE_HIT, None, b).await;
        let n = rand_range(cx, k(cx, "landed_min"), k(cx, "landed_max"), "after charge");
        wait_frames(cx, n).await;
    } else {
        let b = k(cx, "missed_blend");
        action(cx, req::CHARGE_MISS, None, b).await;
        let n = rand_range(cx, k(cx, "missed_min"), k(cx, "missed_max"), "after charge");
        wait_frames(cx, n).await;
    }
    cx.borrow_mut().x.charge = ChargeRun::Over;
}

/// `0x5C8`: not enraged: close (< 250) the positioning and the punch; mid (< 750) the positioning and the slam; far
/// the positioning, the stomp and the slam.
async fn calm(cx: &Cx) {
    let d = dist_to_target(cx);
    if d - k(cx, "close_range") < 0.0 {
        position(cx).await;
        punch(cx).await;
    } else if d - k(cx, "mid_range") < 0.0 {
        position(cx).await;
        slam(cx).await;
    } else {
        position(cx).await;
        stomp(cx).await;
        slam(cx).await;
    }
}

/// The appearance (`0x682` → `0x1B30`, spawned by the room): facing Sora now; 69 (blend 0, a plain request); the
/// fade-in (30 frames); KH1's flag 0x80 for 30 frames (not ported) and its effect 2 (M13); 30 frames; then actor
/// flag 4 with the idle queued (bit 30): the whole script waits until 69 ends; the idle state.
async fn appear(cx: Cx) {
    if cx.borrow().x.spawned {
        face_sora_now(&cx);
        play(&cx, req::APPEAR, k(&cx, "appear_blend"));
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

/// The idle behaviour (`0x636`): forever the shared wander about the current place (`0x1422(300, 400, &home)`).
async fn idle(cx: Cx) {
    loop {
        let c = |n: &str| k(&cx, n);
        let (inner, outer, band, timeout) = (c("wander_inner"), c("wander_outer"), c("wander_band"), c("wander_timeout"));
        let (chance, lo, hi, home, blend) = (c("wander_idle_chance"), c("wander_idle_min"), c("wander_idle_max"), c("wander_home_max"), c("idle_blend"));
        wander(&cx, inner, outer, band, timeout, chance, lo, hi, home, blend).await;
        frame().await;
    }
}

/// The engaged behaviour (`0x53A`): the first look once; the leash (not ported); calm (`0x5C8`) or, enraged, the
/// charge; the leash; the engaged search (Sora) and engaged again.
async fn engaged(cx: Cx) {
    if cx.borrow().x.look {
        first_look(&cx).await;
    }
    if !cx.borrow().x.enraged {
        calm(&cx).await;
    } else {
        charge(&cx).await;
    }
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// The hit behaviour (`0x47C`, the target the hitter or Sora): a counter (`wp+0x4C` = 2): calm, fewer than 2 counted
/// hits the punch, else the stomp and the count back to 0; enraged the charge. The rage (3): enraged, 217. Then
/// engaged.
async fn hit(cx: Cx) {
    let next = std::mem::take(&mut cx.borrow_mut().x.next);
    match next {
        Next::Counter => {
            let (enraged, counted) = {
                let io = cx.borrow();
                (io.x.enraged, io.x.counted)
            };
            if !enraged {
                if (counted as f32) - k(&cx, "hit_stomp_count") < 0.0 {
                    punch(&cx).await;
                } else {
                    stomp(&cx).await;
                    cx.borrow_mut().x.counted = 0;
                }
            } else {
                charge(&cx).await;
            }
        }
        Next::Rage => {
            cx.borrow_mut().x.enraged = true;
            rage(&cx).await;
        }
        Next::None => {}
    }
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// The supervisor (`0x1FF8`): after the init frame, each frame (nothing while the appearance's flag 4 holds the
/// script): while idle, the search (Sora within 1500; world 11: always) → engaged with the target; then a state
/// request restarts the behaviour thread (the idle clears the target, `0x61E`; the engaged and hit states set it).
async fn supervisor(cx: Cx) {
    frame().await;
    loop {
        while cx.borrow().x.paused {
            frame().await;
        }
        if cx.borrow().state.0 == st::IDLE && search(&cx, k(&cx, "idle_search")).is_some() {
            set_target(&cx, true);
            set_state(&cx, st::ENGAGED);
        }
        if let Some(s) = take_state(&cx) {
            set_target(&cx, s != st::IDLE);
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Large Body's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as the Shadow's: per-enemy seeds). `frames`: KH1 motion
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
                // `0x20DC`: home = the position; the turn rate; the front row (`0x9CA`); the state object cleared.
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
                _ => Box::pin(hit(cx)),
            });
        }
        poll(&mut self.behaviour);
        std::mem::take(&mut self.io.borrow_mut().out)
    }

    /// `sys 42` (`0x1402B7E60`): the flinch's push slide (`G+0x2C` × the stats block's `+0xC4`, the knock-back
    /// weight; `crate::enemy::react`'s).
    fn slide(io: &mut crate::enemy::script::Core<Consts, Attack, Wp>, dir: Vec3) {
        let p = io.c.flinch_push() * io.c.weight();
        io.body.push(dir, p);
    }

    /// The handler's first steps for a damaging hit that leaves it alive (`0x2346`): not enraged and under
    /// `enrage_hp` of its HP: the rage (true: `sys 42`, flag 4 cleared, enraged, the hit state with 3).
    fn rage_hit(&self, dir: Vec3) -> bool {
        let mut io = self.io.borrow_mut();
        let limit = io.c.num("enrage_hp");
        if io.x.enraged || io.x.hp - limit >= 0.0 {
            return false;
        }
        Self::slide(&mut io, dir);
        io.x.enraged = true;
        io.x.next = Next::Rage;
        io.x.no_reaction = true;
        io.x.cut = true;
        io.state = (st::HIT, true);
        true
    }

    /// `0x908` in the handler: the charge's row gone, the front row made again now.
    fn charge_ends(&self, how: ChargeRun) {
        let mut io = self.io.borrow_mut();
        io.x.charge = how;
        front_on(&mut io, true);
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
    fn set_hp_ratio(&self, r: f32) {
        self.io.borrow_mut().x.hp = r;
    }
    /// The hit handler (`0x21BE`) for a clash, a repel or a damaging hit that took Re:CoM's reaction path (the
    /// armour below keeps the rest): see the module's notes.
    fn on_hit(&self, news: HitNews) {
        let (kind, dir, charging, cur) = {
            let mut io = self.io.borrow_mut();
            io.x.look = false;
            io.x.no_reaction = false;
            io.x.knock = false;
            let cur = io.x.cur.or(io.w.motion);
            (io.x.hit_kind, io.x.hit_dir, cur == Some(req::CHARGE), cur)
        };
        if news != HitNews::Damage || matches!(kind, 6 | 7) {
            if charging {
                self.charge_ends(ChargeRun::Clashed);
            } else {
                let mut io = self.io.borrow_mut();
                io.x.counted += 1;
                io.x.next = Next::Counter;
                io.state = (st::HIT, true);
            }
            return;
        }
        if self.rage_hit(dir) {
            return;
        }
        if charging {
            self.charge_ends(ChargeRun::Hit);
            let mut io = self.io.borrow_mut();
            io.x.knock = true;
            io.x.next = Next::Counter;
            io.state = (st::HIT, true);
            return;
        }
        let mut io = self.io.borrow_mut();
        if kind == 4 || kind == 5 {
            io.x.counted += 1;
            if cur.is_some_and(|r| STEADY.contains(&r)) {
                Self::slide(&mut io, dir);
                io.x.no_reaction = true;
            } else {
                io.x.knock = true;
                io.x.next = Next::Counter;
                io.state = (st::HIT, true);
            }
        } else {
            Self::slide(&mut io, dir);
            io.x.no_reaction = true;
        }
    }
    /// `sys 42`'s hits keep Re:CoM's state (its super armour): every hit but a knock-back or launch outside the
    /// stomp, the slam and the rage, and none during the charge (those take KH1's knock-back, `sys 79`). The rage
    /// check comes after the damage ([`Behaviour::on_armoured_hit`]).
    fn armoured(&self, playing: Option<u16>, _launch_base: u16, kind: Option<u8>) -> bool {
        let io = self.io.borrow();
        if io.x.dead || playing == Some(req::CHARGE) {
            return false;
        }
        !matches!(kind.unwrap_or(io.x.hit_kind), 4 | 5) || playing.is_some_and(|r| STEADY.contains(&r))
    }
    fn on_armoured_hit(&self, dir: Vec3) {
        self.io.borrow_mut().x.look = false;
        if self.rage_hit(dir) {
            return;
        }
        let mut io = self.io.borrow_mut();
        if matches!(io.x.hit_kind, 4 | 5) {
            io.x.counted += 1;
        }
        Self::slide(&mut io, dir);
    }
    fn ends_action(&self) -> bool {
        std::mem::take(&mut self.io.borrow_mut().x.cut)
    }
    fn keeps_reaction(&self) -> bool {
        !self.io.borrow().x.no_reaction
    }
    /// `sys 79`: KH1's knock-back (kind 4) whatever the hit's kind; the killing hit keeps its own.
    fn reaction_kind(&self, kind: u8) -> u8 {
        let io = self.io.borrow();
        if io.x.knock && !io.x.dead {
            4
        } else {
            kind
        }
    }
    fn on_defeat(&self) {
        let mut io = self.io.borrow_mut();
        io.x.dead = true;
        io.x.knock = false;
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
    fn set_playing(&self, request: Option<u16>) {
        self.io.borrow_mut().x.cur = request;
    }
    fn set_world(&self, w: Option<u8>) {
        self.io.borrow_mut().x.olympus = w == Some(OLYMPUS);
    }
    /// The charge's row landed on Sora (`0x968`: `wp+0x58` = 10).
    fn on_landed(&self, attack: u8) {
        if attack == Attack::Charge.index() {
            let mut io = self.io.borrow_mut();
            if io.x.charge == ChargeRun::Charging {
                io.x.charge = ChargeRun::Landed;
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
        // The player's `ex_2050.bd` / `ex_2050_00.bd` / `btltbl.bin` / KH1 exe numbers as the asset step reads them
        // (its manifest on the session's files).
        let v: &[(&str, f32)] = &[
            ("walk_speed", 3.5), ("run_speed", 5.0), ("turn_deg", 8.0), ("idle_search", 1500.0), ("world11_search", 10000.0), ("close_range", 250.0),
            ("mid_range", 750.0), ("wander_inner", 300.0), ("wander_outer", 400.0), ("wander_band", 100.0), ("wander_timeout", 300.0),
            ("wander_idle_chance", 0.4), ("idle_blend", 8.0), ("wander_idle_min", 90.0), ("wander_idle_max", 180.0), ("wander_home_max", 700.0),
            ("windup_blend", 8.0), ("charge_time", 180.0), ("charge_near", 100.0), ("charge_past_min", 300.0), ("charge_past_max", 500.0),
            ("charge_arrive", 100.0), ("charge_stuck", 3.6), ("stuck_factor", 0.5), ("charge_blend", 8.0), ("charge_speed", 12.0), ("landed_blend", 0.0),
            ("landed_then", 8.0), ("landed_min", 45.0), ("landed_max", 120.0), ("missed_blend", 8.0), ("missed_then", 8.0), ("missed_min", 30.0),
            ("missed_max", 150.0), ("stomp_request", 230.0), ("stomp_blend", 0.0), ("ring_at", 88.0), ("ring_life", 28.0), ("ring_growth", 25.0),
            ("ring_band", 100.0), ("ring_height", 50.0), ("slam_blend", 8.0), ("slam_then", 8.0), ("slam_wait", 60.0), ("punch_blend", 8.0),
            ("punch_then", 8.0), ("punch_wait", 75.0), ("near_range", 400.0), ("away_min", 400.0), ("away_max", 600.0), ("away_time_min", 120.0),
            ("away_time_max", 150.0), ("far_range", 450.0), ("approach_min", 240.0), ("approach_max", 300.0), ("approach_retarget", 5.0),
            ("approach_dist", 350.0), ("look_blend", 0.0), ("look_end_blend", 8.0), ("enrage_blend", 8.0), ("enrage_end_blend", 8.0), ("appear_blend", 0.0),
            ("appear_fade", 30.0), ("appear_wait", 30.0), ("appear_then_blend", 0.0), ("enrage_hp", 0.4), ("hit_stomp_count", 2.0), ("attack_blend", 8.0),
            ("air_accel", 0.1), ("air_drag", 0.95), ("fall_cap", 20.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("launch_min", 20.0),
            ("launch_max", 35.0), ("launch_hop", 1.3499999), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("weight", 1.0),
        ];
        let mut v: Vec<(String, f32)> = v.iter().map(|(k, x)| (k.to_string(), *x)).collect();
        v.push(("gravity".into(), crate::params::GRAVITY));
        v.push(("turn_rate".into(), std::f32::consts::PI / 16.0));
        Table::new(v, NAMES).unwrap()
    }

    /// The player's `xa_ex_2050.mset` lengths by request (the asset step's manifest).
    fn frames() -> Vec<(u16, f32)> {
        vec![
            (0, 356.0), (1, 72.0), (2, 56.0), (6, 60.0), (7, 92.0), (66, 110.0), (67, 210.0), (68, 182.0), (69, 100.0), (70, 80.0), (72, 58.0), (73, 58.0), (74, 58.0),
            (75, 58.0), (76, 68.0), (77, 68.0), (82, 16.0), (83, 30.0), (84, 72.0), (85, 94.0), (86, 82.0), (87, 16.0), (88, 30.0), (89, 72.0), (90, 94.0), (91, 82.0),
            (200, 158.0), (206, 92.0), (207, 24.0), (208, 116.0), (209, 296.0), (210, 158.0), (211, 290.0), (212, 460.0), (213, 136.0), (214, 92.0), (215, 52.0),
            (216, 180.0), (217, 128.0), (218, 52.0), (219, 56.0), (230, 182.0), (231, 272.0),
        ]
    }

    /// The asset step's looping clips (the pose rule).
    fn loops() -> Vec<u16> {
        vec![0, 1, 2, 6, 67, 83, 85, 88, 90, 102, 104, 207, 211, 217, 219]
    }

    /// A flat floor at y = 0, Sora at `sora`; an action holds the AI for its motion's length; the current request is
    /// the last one played.
    struct Sim {
        ai: Ai,
        pos: Vec3,
        busy: f32,
        current: Option<u16>,
        moved: f32,
        log: Vec<Cmd>,
        /// The motion queued after the current one, and the frames the current one has played.
        queued: Option<u16>,
        age: f32,
    }

    impl Sim {
        fn run(&mut self, sora: Vec3, n: usize) {
            for _ in 0..n {
                let airborne = self.ai.body().airborne();
                let w = World { step: 1.0, pos: self.pos, target: Some(sora), grounded: !airborne, airborne, moved: self.moved, motion: self.current };
                if self.busy > 0.0 {
                    self.busy -= 1.0;
                    if self.busy <= 0.0 {
                        self.current = Some(req::IDLE);
                    }
                } else {
                    for c in self.ai.frame(w) {
                        match c {
                            ECmd::Action { request, .. } => {
                                self.busy = frames().iter().find(|x| x.0 == request).map_or(30.0, |x| x.1);
                                self.current = Some(request);
                                self.age = 0.0;
                            }
                            ECmd::Motion { request, .. } | ECmd::Attack { request, .. } => {
                                if self.current != Some(request) {
                                    self.current = Some(request);
                                    self.queued = None;
                                    self.age = 0.0;
                                }
                            }
                            ECmd::Queue { request, .. } => self.queued = Some(request),
                            _ => {}
                        }
                        self.log.push(c);
                    }
                }
                // A one-shot motion that played to its end: the queued one.
                self.age += 1.0;
                if let Some(cur) = self.current {
                    let len = frames().iter().find(|x| x.0 == cur).map_or(0.0, |x| x.1);
                    if self.busy <= 0.0 && self.age >= len && !loops().contains(&cur) {
                        if let Some(q) = self.queued.take() {
                            self.current = Some(q);
                            self.age = 0.0;
                        }
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
        Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: Vec3::ZERO, busy: 0.0, current: None, moved: 0.0, log: Vec::new(), queued: None, age: 0.0 }
    }

    /// A spawned Large Body past its appearance, idle, Sora far away.
    fn ready(seed: u32) -> Sim {
        let mut s = sim(seed, true);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 110);
        assert!(s.ai.risen());
        s.log.clear();
        s
    }

    /// A Large Body past its appearance running `f` alone (Sora its target, the hit state's thread).
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
        assert_eq!((consts().num("charge_speed"), consts().num("ring_growth"), consts().num("enrage_hp")), (12.0, 25.0, 0.4));
        assert_eq!(Attack::ALL.map(|a| a.request()), [200, 231, 207, 230]);
        assert_eq!((Attack::Stomp.index(), Attack::Stomp.rows()), (3, &[0x31C][..]));
    }

    /// `0x20DC`: the front row on from the first frame (mode 3), the turn rate 8° a frame; `0x1B30`: facing Sora, 69
    /// (blend 0) with the fade-in, 30 frames, then paused until 69 ends (100 frames); the idle state, the wander.
    #[test]
    fn appearance_with_the_front_row_on() {
        let mut s = sim(1, true);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 2);
        assert!(s.log.contains(&ECmd::Guard(Some((FRONT, ALWAYS)))), "{:?}", s.log);
        assert!(s.log.contains(&ECmd::Motion { request: req::APPEAR, blend: 0.0 }) && s.log.contains(&ECmd::FadeIn(30.0)));
        assert!((s.ai.body().turn.unwrap() - 8f32.to_radians()).abs() < 1e-6);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 60);
        assert!(!s.ai.risen(), "paused while 69 plays");
        s.current = Some(req::IDLE);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 3);
        assert!(s.ai.risen() && s.log.contains(&ECmd::State(st::IDLE)));
        s.run(Vec3::new(0.0, 0.0, 5000.0), 10);
        assert!(s.log.iter().any(|c| matches!(c, ECmd::Motion { request: req::WALK, .. })), "it wanders");
        assert_eq!(s.log.iter().filter(|c| matches!(c, ECmd::Guard(_))).count(), 1, "the row is made once");
    }

    /// The search: Sora within 1500 engages (the first look 214, once); farther, not; KH1's world 11: always.
    #[test]
    fn the_search_and_the_first_look() {
        let mut s = ready(2);
        s.run(Vec3::new(0.0, 0.0, 1400.0), 3);
        assert!(s.log.contains(&ECmd::State(st::ENGAGED)) && motions(&s.log).contains(&req::LOOK), "{:?}", s.log);
        s.run(Vec3::new(0.0, 0.0, 1400.0), 2000);
        assert_eq!(motions(&s.log).iter().filter(|&&r| r == req::LOOK).count(), 1);
        let mut s = ready(2);
        s.run(Vec3::new(0.0, 0.0, 1600.0), 3);
        assert!(!s.log.contains(&ECmd::State(st::ENGAGED)));
        let mut s = sim(2, true);
        Behaviour::set_world(&s.ai, Some(OLYMPUS));
        s.run(Vec3::new(0.0, 0.0, 9000.0), 110);
        s.current = Some(req::IDLE);
        s.run(Vec3::new(0.0, 0.0, 9000.0), 5);
        assert!(s.log.contains(&ECmd::State(st::ENGAGED)), "Olympus");
        // A hit before it engages cancels the look.
        let mut s = ready(3);
        s.ai.set_hit_kind(3);
        Behaviour::on_hit(&s.ai, HitNews::Damage);
        s.run(Vec3::new(0.0, 0.0, 1000.0), 400);
        assert!(!motions(&s.log).contains(&req::LOOK));
    }

    /// Close (< 250) the punch, mid (< 750) the slam, far the stomp then the slam; the charge only once enraged.
    #[test]
    fn calm_rounds_by_distance() {
        for (d, want) in [(200.0, vec![Attack::Punch]), (600.0, vec![Attack::Slam]), (1200.0, vec![Attack::Stomp, Attack::Slam])] {
            let mut s = with(4, move |cx| async move { calm(&cx).await });
            // Sora keeps his distance (he steps back as it repositions).
            for _ in 0..1200 {
                let p = s.pos + Vec3::new(0.0, 0.0, d);
                s.run(p, 1);
            }
            let a = attacks(&s.log);
            assert_eq!(&a[..want.len().min(a.len())], &want[..], "{d}: {a:?}");
            assert!(!a.contains(&Attack::Charge));
        }
    }

    /// The punch: action 200 (blend 8) with both hands' rows, then 75 frames.
    #[test]
    fn the_punch() {
        let mut s = with(5, |cx| async move { punch(&cx).await });
        s.run(Vec3::new(0.0, 0.0, 200.0), 2);
        assert!(s.log.contains(&ECmd::Action { request: req::PUNCH, attack: Some(Attack::Punch), then: req::IDLE, blend: 8.0 }));
    }

    /// The stomp: 230 (a plain request with blend 0, Re:CoM's attack state 3 in the class map), held to its end (182
    /// frames); the ring is the host's (frames 88..116).
    #[test]
    fn the_stomp() {
        let mut s = with(6, |cx| async move {
            stomp(&cx).await;
            play(&cx, req::IDLE, 8.0);
        });
        s.run(Vec3::new(0.0, 0.0, 600.0), 2);
        assert!(s.log.contains(&ECmd::Attack { request: req::STOMP, attack: Attack::Stomp, blend: 0.0 }));
        s.run(Vec3::new(0.0, 0.0, 600.0), 178);
        assert!(!s.log.contains(&ECmd::Motion { request: req::IDLE, blend: 8.0 }));
        s.run(Vec3::new(0.0, 0.0, 600.0), 6);
        assert!(s.log.contains(&ECmd::Motion { request: req::IDLE, blend: 8.0 }), "after 230's 182 frames");
    }

    /// The charge: the wind-up 206 for its 92 frames, then the front row off and 207 at 12 a frame at Sora; within
    /// 100 of him on to a random point 300–500 from there; then the row back and 209 (it missed) or 208 (its row hit him).
    #[test]
    fn the_charge() {
        let sora = Vec3::new(0.0, 0.0, 1500.0);
        let mut s = with(7, |cx| async move { charge(&cx).await });
        s.run(sora, 90);
        assert!(motions(&s.log).contains(&req::WINDUP) && !motions(&s.log).contains(&req::CHARGE));
        s.run(sora, 5);
        assert!(s.log.contains(&ECmd::Guard(None)) && s.log.iter().filter(|c| matches!(c, ECmd::Attack { request: req::CHARGE, .. })).count() == 1, "{:?}", s.log);
        let z = s.pos.z;
        s.run(sora, 50);
        assert!(s.pos.z - z > 11.0 * 50.0, "12 a frame: {}", s.pos.z - z);
        let mut near = f32::MAX;
        for _ in 0..200 {
            s.run(sora, 1);
            near = near.min((s.pos - sora).length());
        }
        let m = motions(&s.log);
        assert!(m.contains(&req::CHARGE_MISS) && !m.contains(&req::CHARGE_HIT), "{m:?}");
        assert!(s.log.iter().rposition(|c| *c == ECmd::Guard(Some((FRONT, ALWAYS)))) > s.log.iter().position(|c| *c == ECmd::Guard(None)), "the row back");
        assert!(near < 100.0 + 12.0, "it reached him: {near}");
        let d = (s.pos - sora).length();
        assert!((150.0..=520.0).contains(&d), "then on to a point 300–500 from there: {d}");
        // Its row hit Sora: 208.
        let mut s = with(7, |cx| async move { charge(&cx).await });
        s.run(sora, 100);
        s.ai.on_landed(Attack::Charge.index());
        s.run(sora, 10);
        assert!(motions(&s.log).contains(&req::CHARGE_HIT));
    }

    /// The hit handler: a flinch takes no reaction and asks nothing (`sys 42`); a knock-back or launch is KH1's
    /// knock-back (`sys 79`, kind 4) and the counter (the punch, after two counted hits the stomp); during the stomp,
    /// the slam or the rage no reaction; a clash or repel counts and counters.
    #[test]
    fn the_hit_handler() {
        let s = ready(8);
        s.ai.set_hit_kind(3);
        assert!(s.ai.armoured(Some(req::PUNCH), 0, None), "a flinch keeps Re:CoM's state");
        s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
        assert!(!s.ai.io.borrow().state.1 && s.ai.io.borrow().body.push.is_some());
        s.ai.set_hit_kind(5);
        assert!(!s.ai.armoured(Some(req::PUNCH), 0, None) && s.ai.armoured(Some(req::STOMP), 0, None) && s.ai.armoured(Some(req::RAGE), 0, None));
        Behaviour::on_hit(&s.ai, HitNews::Damage);
        assert_eq!((s.ai.reaction_kind(5), s.ai.keeps_reaction(), s.ai.io.borrow().state), (4, true, (st::HIT, true)));
        // Counted: 1; a clash: 2 → the stomp.
        Behaviour::on_hit(&s.ai, HitNews::Clash);
        assert_eq!(s.ai.io.borrow().x.counted, 2);
        let mut s2 = ready(8);
        s2.ai.io.borrow_mut().has_target = true;
        s2.ai.set_hit_kind(4);
        Behaviour::on_hit(&s2.ai, HitNews::Damage);
        s2.run(Vec3::new(0.0, 0.0, 200.0), 3);
        assert!(attacks(&s2.log).contains(&Attack::Punch), "one counted hit: the punch {:?}", s2.log);
        let mut s3 = ready(8);
        s3.ai.io.borrow_mut().has_target = true;
        s3.ai.io.borrow_mut().x.counted = 1;
        Behaviour::on_hit(&s3.ai, HitNews::Repelled);
        s3.run(Vec3::new(0.0, 0.0, 200.0), 300);
        assert!(attacks(&s3.log).contains(&Attack::Stomp) && s3.ai.io.borrow().x.counted == 0, "two: the stomp {:?}", attacks(&s3.log));
        // The killing hit keeps its own reaction.
        s3.ai.set_hit_kind(5);
        Behaviour::on_hit(&s3.ai, HitNews::Damage);
        s3.ai.on_defeat();
        assert_eq!(s3.ai.reaction_kind(5), 5);
    }

    /// The rage (`0x2346`): the first damaging hit that leaves it under 0.4 of its HP takes no reaction, ends its
    /// action at once (flag 4) and roars (217); from then on it charges; it happens once.
    #[test]
    fn the_rage_then_charges() {
        let mut s = ready(9);
        s.ai.io.borrow_mut().has_target = true;
        s.ai.set_hit_kind(3);
        s.ai.set_hp_ratio(0.39);
        assert!(s.ai.armoured(Some(req::PUNCH), 0, None));
        s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
        assert!(s.ai.ends_action() && !s.ai.ends_action(), "taken once");
        s.current = Some(req::IDLE);
        s.run(Vec3::new(0.0, 0.0, 300.0), 3);
        assert!(motions(&s.log).contains(&req::RAGE) && s.ai.io.borrow().x.enraged);
        s.run(Vec3::new(0.0, 0.0, 300.0), 400);
        assert!(motions(&s.log).contains(&req::WINDUP), "then the charge: {:?}", motions(&s.log));
        // Once enraged, no second rage; a knock-back on the reaction path enrages too.
        s.ai.set_hp_ratio(0.1);
        s.ai.set_hit_kind(3);
        s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
        assert!(!s.ai.ends_action());
        let s = ready(10);
        s.ai.set_hit_kind(5);
        s.ai.set_hp_ratio(0.2);
        Behaviour::on_hit(&s.ai, HitNews::Damage);
        assert!(!s.ai.keeps_reaction() && s.ai.io.borrow().x.enraged);
    }

    /// During the charge: a damaging hit of any kind is KH1's knock-back and a new counter (the charge again once
    /// enraged), the front row back at once; a clash ends it into 209.
    #[test]
    fn hits_during_the_charge() {
        let sora = Vec3::new(0.0, 0.0, 2500.0);
        let mut s = with(11, |cx| async move { charge(&cx).await });
        s.ai.io.borrow_mut().x.enraged = true;
        s.run(sora, 100);
        assert_eq!(s.current, Some(req::CHARGE));
        s.ai.set_hit_kind(3);
        assert!(!s.ai.armoured(s.current, 0, None));
        s.ai.set_playing(s.current);
        Behaviour::on_hit(&s.ai, HitNews::Damage);
        assert_eq!((s.ai.reaction_kind(3), s.ai.keeps_reaction()), (4, true));
        assert_eq!(s.ai.take_now(), vec![ECmd::Guard(Some((FRONT, ALWAYS)))]);
        let mut s = with(11, |cx| async move { charge(&cx).await });
        s.run(sora, 100);
        s.ai.set_playing(s.current);
        Behaviour::on_hit(&s.ai, HitNews::Clash);
        assert_eq!(s.ai.io.borrow().x.charge, ChargeRun::Clashed);
        s.current = Some(req::IDLE);
        s.run(sora, 5);
        assert!(motions(&s.log).contains(&req::CHARGE_MISS));
    }

    /// A long fight: the AI acts at least every 600 frames (the host's quiet watchdog), with Sora circling 2–7 m away
    /// and a hit every 150–400 frames (12 seeds, 20 000 frames each), through its rage.
    #[test]
    fn a_long_fight_never_stalls() {
        for seed in 0..12u32 {
            let mut s = ready(seed);
            let (mut last, mut worst, mut next_hit, mut hp) = (0usize, 0usize, 300usize, 1.0f32);
            let mut r = Kh1Rand::new(seed + 100);
            for f in 0..20_000usize {
                let a = f as f32 / 600.0;
                let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
                let sora = Vec3::new(a.sin() * rad, 0.0, a.cos() * rad);
                if f == next_hit {
                    let kind = [3u8, 3, 4, 5][(r.next_f32() * 4.0) as usize % 4];
                    hp = (hp - 0.03).max(0.05);
                    s.ai.set_hit_kind(kind);
                    s.ai.set_hp_ratio(hp);
                    s.ai.set_hit_dir(Vec3::new(0.0, 0.0, if r.next_f32() < 0.5 { 1.0 } else { -1.0 }));
                    s.ai.set_playing(s.current);
                    if s.ai.armoured(s.current, 0, None) {
                        s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
                        if s.ai.ends_action() {
                            s.busy = 0.0;
                        }
                    } else {
                        Behaviour::on_hit(&s.ai, if r.next_f32() < 0.15 { HitNews::Clash } else { HitNews::Damage });
                        s.busy = 34.0;
                    }
                    let _ = s.ai.take_now();
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
            assert!(s.ai.io.borrow().x.enraged, "seed {seed}: it enraged");
        }
    }

    /// The Large Body through the host's rules (`crate::enemy::host::model`, what `kh1-recom`'s `win::kh1_enemy`
    /// decides) with its class map (`ground::largebody::CLASS.attack_states`: punch, slam, charge in state 2, the
    /// stomp in state 3) and the asset step's looping clips.
    fn hosted(s: Sim) -> Model<Consts> {
        let Sim { ai, pos, .. } = s;
        let mut m = Model::new(Box::new(ai), consts(), frames(), loops(), vec![2, 2, 2, 3]);
        m.pos = pos;
        m
    }

    /// The charge in the host: 207 asked for once and played in state 2 (looped clip) until it reaches its goal past
    /// Sora, never released; the stomp's 230 in state 3 for its 182 frames.
    #[test]
    fn hosted_the_charge_and_the_stomp() {
        let s = with(12, |cx| async move {
            charge(&cx).await;
        });
        let mut m = hosted(s);
        m.run(Vec3::new(0.0, 0.0, 1200.0), 500);
        assert_eq!(m.released, 0);
        assert_eq!(m.attacks_of(req::CHARGE).len(), 1, "{:?}", m.attacks_of(req::CHARGE));
        let run = m.longest_run(req::CHARGE);
        assert!(run > 60, "207 unbroken: {run}");
        assert!(m.trace.iter().all(|t| t.1 != Some(req::CHARGE) || t.0 == 2));
        let s = with(13, |cx| async move {
            stomp(&cx).await;
            play(&cx, req::IDLE, 8.0);
        });
        let mut m = hosted(s);
        m.run(Vec3::new(0.0, 0.0, 600.0), 260);
        assert_eq!(m.released, 0);
        let run = m.longest_run(req::STOMP);
        assert!((180..=182).contains(&run), "230's 182 frames: {run}");
        assert!(m.trace.iter().all(|t| t.1 != Some(req::STOMP) || t.0 == 3));
        assert_eq!(m.current, Some(req::IDLE));
    }

    /// A long fight in the host (12 seeds, 20 000 frames, Sora circling 2–7 m away): no attack is released, none is
    /// asked for again within two frames.
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
            for w in issued.windows(2) {
                assert!(w[1].1 != w[0].1 || w[1].0 > w[0].0 + 2, "seed {seed}: {} asked again at {} after {}", w[0].1, w[1].0, w[0].0);
            }
            let acts = m.log.iter().filter(|(_, c)| matches!(c, ECmd::Action { attack: Some(_), .. } | ECmd::Attack { .. })).count();
            assert!(acts > 10, "seed {seed}: it attacks ({acts})");
        }
    }
}
