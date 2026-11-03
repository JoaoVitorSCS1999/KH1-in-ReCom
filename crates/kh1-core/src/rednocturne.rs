//! KH1 FM's Red Nocturne (`xa_ex_2110.mdls` part 5, `ex_2110.bd`), engine-agnostic: its AI script on the shared enemy
//! framework (`crate::enemy`), flying (KH1's movement state 4, `crate::enemy::Body::flying`), with the casting family's
//! shared routines (`crate::caster`) and its two spell children's waits ([`Cast`]: `ex_2110_00.bd` the Fire,
//! `ex_2110_01.bd` the big Fire; part 3). M11 (`docs/recom-m11-enemies.md` § The Red Nocturne). Re:CoM runs it natively:
//! each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), each fire one of Re:CoM's own
//! bullets moved by KH1's numbers (`win::kh1_thrown`).
//!
//! Every number is data ([`NAMES`], [`FIRE_NAMES`], read by the asset step from the player's `ex_2110.bd` and its two
//! children at checked offsets). Script offsets are the player's own (Checked (files): the scripts disassembled with
//! `tools/kh1-ai-script.py`, the parent's code from `0x308`, the children's from `0x18`; the syscalls named are Checked
//! (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x1E48`: init `0x1EC8` → `0xC94` (records LV1 `0x14` / LV2 `0x80`; the stats block's `+0x14` = 0.2 (the
//!   flight's easing) and `+0x18` = 0.95; flight with no velocity (`sys 135`); the state and search objects; home);
//!   the handlers (hit `0x1F1E`, message `0x20EA`: answers 0, the hit-effect filter `0x1EEC`); a yield; the idle start
//!   `0x910` as its behaviour; then each frame: in the idle state (1) the search (`0xA52`: Sora within 1000 → engaged),
//!   then a state request restarts the behaviour (1 idle `0x8DC` → `0x8F4`, target cleared; 2 engaged `0x360` →
//!   `0x384`; 3 hit `0x308` → `0x32C`; both with the target). It never sets a turn rate (`sys 125`): KH1's π/16.
//! - Every motion it plays is a plain request (no action bit) except its appearance (69): the script never pauses for
//!   its own motions, only for KH1's reactions. It has **no attack rows of its own**: its four rows (`0xEC` / `0x15C`
//!   the Fire and its batted row, `0x1CC` / `0x23C` the big Fire's) are its children's.
//! - the hit handler `0x1F1E`: seen (`wp+0x38`); a hit of one of its own rows (`sys 188` in [10500, 10504]: a fire of
//!   its kind sent back) or a Blizzard (attribute 3): the damage; alive: KH1's EXP bonus (`sys 194(EXP / 10 + 2)`),
//!   `sys 179(1800)` (KH1's dizzy status: not built, owner question) and the hit state; killed: its death. A Fire hit
//!   (attribute 2): no damage, HP to max, no reaction (`0xDC8`; element: left out, owner). Else the damage (`0x1854`):
//!   killed → its death `0x1518` (Re:CoM's defeat here); a clash or a repel (−1): the hit state only for a clash (or
//!   attributes 8 / 10); a living damaging hit: the hit state. KH1's reaction plays for every hit (the handler zeroes
//!   no kind but the Fire's).
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0x1032` (`0xD7A`: 1500 / 750), the notice `0xD1A` for
//! a room-placed one (spawn type 0: battle spawns play KH1's appearance, owner 2026-11-02), the search object
//! (`sys 48`: Sora is the only target; `sys 139`'s out-of-sight test taken as in sight, as every port), the element
//! rules (attributes are not carried), effects (`sys 75` / `sys 225`, M13), the appearance's flag 0x80, the death
//! routine (Re:CoM's defeat, prizes, EXP).

use crate::caster::{fly_to, play, rotdeg, step_casts, steer_y, stuck, turn_step, wander_leg, Cast, Face, FlyTo, Wander};
use crate::enemy::script::{fly, frame, motion_done, poll, pos, push, rand, rand_range, random_offset, set_air_accel, set_state, set_target, take_state, target, tick_clock};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Reward, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Red Nocturne's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The circling hover's motion (`0x686`, `sys 12(2, 8)`): 246 frames.
    pub const HOVER: u16 = 2;
    /// The appearance (`0xE94`: an action, 219 queued), 90 frames, root motion.
    pub const APPEAR: u16 = 69;
    /// The Fire's charge (`0x4FC`), 60 frames, then its cast 201 (queued), 116 frames.
    pub const CHARGE: u16 = 200;
    pub const CAST: u16 = 201;
    /// The big Fire's charge (`0x490`, from frame 8), 120 frames, then its cast 203 (queued), 200 frames.
    pub const BIG_CHARGE: u16 = 202;
    pub const BIG_CAST: u16 = 203;
    /// The notice (`0xD1A`, a room-placed one only), 158 frames.
    pub const NOTICE: u16 = 214;
    /// The flee after a hit (`0x5EA`), 120 frames.
    pub const FLEE: u16 = 217;
    /// The idle hover (`0x93E`), 200 frames.
    pub const WANDER: u16 = 219;
    /// The idle the script asks for after a cast (entry 0's motion, as 0).
    pub const REST: u16 = 220;
}

/// The Red Nocturne has no attack of its own (its rows are its children's): the host's attack list is empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {}

impl Attack {
    pub const ALL: [Attack; 0] = [];

    pub fn index(self) -> u8 {
        match self {}
    }
}

/// Its spell children (`sys 94`'s sub-object index): the host's thrown objects ([`ECmd::Throw`]).
pub mod child {
    /// `ex_2110_00.bd`: the Fire (`0x4FC`, rows `0xEC` / `0x15C`).
    pub const FIRE: u8 = 0;
    /// `ex_2110_01.bd`: the big Fire (`0x490`, rows `0x1CC` / `0x23C`).
    pub const BIG_FIRE: u8 = 1;
}

/// The names of the Red Nocturne's numbers (besides `crate::enemy::ENGINE_NAMES` and [`FIRE_NAMES`]).
pub const NAMES: &[&str] = &[
    "close_range",
    "mid_range",
    "near_fire_chance",
    "far_bigfire_chance",
    "hit_close_range",
    "idle_search",
    "cast_blend",
    "cast_start",
    "bigfire_start",
    "cast_queue_blend",
    "idle_blend",
    "idle_start",
    "flip",
    "strafe_turn",
    "strafe_side_chance",
    "strafe_left",
    "strafe_right",
    "strafe_speed",
    "flee_min",
    "flee_max",
    "flee_speed",
    "flee_stop",
    "flee_up",
    "flee_high",
    "flee_time_min",
    "flee_time_max",
    "approach_speed",
    "approach_stop",
    "approach_up",
    "approach_high",
    "approach_time",
    "hover_turn",
    "hover_side_chance",
    "hover_left",
    "hover_right",
    "hover_mid",
    "hover_time_min",
    "hover_time_max",
    "hover_blend",
    "hover_speed",
    "hover_sink",
    "floor_probe",
    "hover_low_min",
    "hover_low_max",
    "hover_stuck",
    "wander_turn",
    "wander_sink",
    "wander_low",
    "wander_min",
    "wander_max",
    "wander_blend",
    "wander_speed",
    "fly_blend",
    "fly_stuck",
    "fly_no_motion",
    "stuck_half",
    "air_accel_set",
    "notice_blend",
    "notice_world",
    "wait_motion",
    "appear_fade",
    "appear_blend",
    "exp_div",
    "exp_add",
    "own_row_first",
    "own_row_last",
    "dizzy_time",
    "fire_child",
    "bigfire_child",
];

/// Its two children's numbers (`ex_2110_00.bd` `fire_*`, `ex_2110_01.bd` `bigfire_*`): the wait in the cast motion (40 /
/// 90 frames) the behaviour runs ([`Cast`]); the start (`sys 19(parent, 0x8001, t)`, then `up`), the flight (turn,
/// speed, life, fade) the host's.
pub const FIRE_NAMES: &[&str] = &["fire_wait", "fire_t", "fire_up", "fire_turn", "fire_speed", "fire_life", "fire_fade", "bigfire_wait", "bigfire_t", "bigfire_up", "bigfire_turn", "bigfire_speed", "bigfire_life", "bigfire_fade"];

/// Every name the manifest must hold for the behaviour.
pub fn all_names() -> Vec<&'static str> {
    NAMES.iter().chain(FIRE_NAMES).copied().collect()
}

/// The Red Nocturne's numbers.
pub type Consts = Table;

/// The state objects (`wp+0x20`), KH1's own numbers.
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 3;
}

/// The Red Nocturne's own work area: `wp+0` (the hover's side draw, which the next round's strafe reads), `wp+0x38`
/// (seen: no notice), the appearance, the boot (`0x900`), its waiting spell children, the hit handler's inputs and
/// what it does at once.
#[derive(Clone, Debug, Default)]
struct Wp {
    side: f32,
    seen: bool,
    spawned: bool,
    appeared: bool,
    boot: bool,
    dying: bool,
    casts: Vec<Cast>,
    own: bool,
    world: Option<u8>,
    now: Vec<ECmd<Attack>>,
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

/// The actor's target (`+0x74`): Sora (Re:CoM's battles have no other party member).
fn tgt(cx: &Cx) -> Vec3 {
    let io = cx.borrow();
    target(cx).or(io.w.target).unwrap_or(io.w.pos)
}

/// `0x15F8` (`0x1982(actor, target)`): the distance to the target (3D).
fn dist(cx: &Cx) -> f32 {
    (tgt(cx) - pos(cx)).length()
}

/// The facing vector `+0x30`.
fn facing(cx: &Cx) -> Vec3 {
    Vec3::from_yaw(cx.borrow().body.yaw)
}

/// `0x1B82` (`sys 36`): the target yaw toward the target (the engine turns).
fn yaw_tgt(cx: &Cx) {
    let d = tgt(cx) - pos(cx);
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `sys 13` (`0x1402B9F80` → `0x14029F980`): a request written whatever plays (the playing one starts again), from
/// `start` frames.
fn request(cx: &Cx, r: u16, blend: f32, start: f32) {
    let again = {
        let mut io = cx.borrow_mut();
        let again = io.w.motion == Some(r);
        io.clock = (Some(r), 0.0);
        again
    };
    push(
        cx,
        if again {
            ECmd::Restart { request: r, blend, start }
        } else if start != 0.0 {
            ECmd::MotionAt { request: r, blend, start }
        } else {
            ECmd::Motion { request: r, blend }
        },
    );
}

/// `sys 14`: the motion queued.
fn queue(cx: &Cx, r: u16, blend: f32) {
    push(cx, ECmd::Queue { request: r, blend });
}

/// `sys 172` with the cast queued (`0x14029EFC0`: played to its end with nothing queued): the queued request plays and
/// has played to its end.
fn done_after(cx: &Cx, last: u16) -> bool {
    cx.borrow().w.motion == Some(last) && motion_done(cx)
}

/// The floor's height under the target (`0x168A`: `sys 77` straight down from `floor_probe` above it; the battle
/// floor here: Re:CoM's battle floors are flat, Assumption as every flier's), KH1 y.
fn floor_under_target(cx: &Cx) -> f32 {
    let f = cx.borrow().floor;
    f.unwrap_or_else(|| tgt(cx).y)
}

/// `sys 94(actor, n, target, rows…)`: the spell child made; it waits on this actor's motion ([`Cast`]).
fn spell(cx: &Cx, n: u8, charge: u16, cast: u16, wait: &str) {
    let w = k(cx, wait);
    cx.borrow_mut().x.casts.push(Cast::new(n, charge, cast, w));
    push(cx, ECmd::Note("sys 94: a spell child waits on the cast (charge effects: M13)"));
}

/// `0x4FC`: the strafe and Fire: its facing turned −90° (the last hover's side draw `wp+0` under 0.5) or +90° (the turn
/// then −5° a frame), the Fire child, 200 (blend 8) with 201 queued, and until the cast has played: circling at 5° a
/// frame at 8.4, facing the target; then the idle 220.
async fn strafe_fire(cx: &Cx) {
    let mut turn = k(cx, "strafe_turn");
    let mut d = facing(cx);
    if cx.borrow().x.side - k(cx, "strafe_side_chance") < 0.0 {
        d = rotdeg(d, k(cx, "strafe_left"));
    } else {
        d = rotdeg(d, k(cx, "strafe_right"));
        turn *= k(cx, "flip");
    }
    spell(cx, child::FIRE, req::CHARGE, req::CAST, "fire_wait");
    request(cx, req::CHARGE, k(cx, "cast_blend"), k(cx, "cast_start"));
    queue(cx, req::CAST, k(cx, "cast_queue_blend"));
    while !done_after(cx, req::CAST) {
        let t = target(cx);
        turn_step(cx, &mut d, turn, k(cx, "strafe_speed"), Face::At(t));
        frame().await;
    }
    request(cx, req::REST, k(cx, "idle_blend"), k(cx, "idle_start"));
}

/// `0x490`: the big Fire: its child, 202 (blend 8, from frame 8) with 203 queued, and until the cast has played (or
/// request 6 plays: `0x15C6`) turning to the target; then the idle 220.
async fn big_fire(cx: &Cx) {
    spell(cx, child::BIG_FIRE, req::BIG_CHARGE, req::BIG_CAST, "bigfire_wait");
    request(cx, req::BIG_CHARGE, k(cx, "cast_blend"), k(cx, "bigfire_start"));
    queue(cx, req::BIG_CAST, k(cx, "cast_queue_blend"));
    wait_cast(cx, req::BIG_CAST).await;
    request(cx, req::REST, k(cx, "idle_blend"), k(cx, "idle_start"));
}

/// `0x1592`: until the motion (with its queue) has played or request 6 plays, turning to the target each frame.
async fn wait_cast(cx: &Cx, last: u16) {
    loop {
        if done_after(cx, last) {
            break;
        }
        if cx.borrow().w.motion == Some(k(cx, "wait_motion") as u16) {
            break;
        }
        yaw_tgt(cx);
        frame().await;
    }
}

/// `0x686`: the circling hover: its facing turned −90° (80 %: the draw kept in `wp+0`, which the next strafe reads) or
/// +90° (the turn then −2° a frame); high when it is at or above 90 over the target's floor; [90, 120) frames of the
/// hover motion circling at 2° a frame at 3.64 facing the target: high, it sinks (0.2) until [15, 30) over the floor
/// (a draw each frame); low, it rises to 90 over it; stuck: done.
async fn hover(cx: &Cx) {
    let mut turn = k(cx, "hover_turn");
    let mut d = facing(cx);
    let r = rand(cx, "hover side");
    cx.borrow_mut().x.side = r;
    if r - k(cx, "hover_side_chance") < 0.0 {
        d = rotdeg(d, k(cx, "hover_left"));
    } else {
        d = rotdeg(d, k(cx, "hover_right"));
        turn *= k(cx, "flip");
    }
    let mid = floor_under_target(cx) - k(cx, "hover_mid");
    let me = pos(cx).y;
    let high = me - mid <= 0.0;
    let mut t = rand_range(cx, k(cx, "hover_time_min"), k(cx, "hover_time_max"), "hover time");
    while t > 0.0 {
        play(cx, req::HOVER, k(cx, "hover_blend"));
        let tg = target(cx);
        let speed = k(cx, "hover_speed");
        let s = turn_step(cx, &mut d, turn, speed, Face::At(tg));
        let mut vy = k(cx, "hover_sink");
        let fy = floor_under_target(cx);
        let me = pos(cx).y;
        if high {
            let y = fy - rand_range(cx, k(cx, "hover_low_min"), k(cx, "hover_low_max"), "hover low");
            if me - y >= 0.0 {
                vy = 0.0;
            }
        } else {
            let y = fy - k(cx, "hover_mid");
            vy = if me - y >= 0.0 { vy * k(cx, "flip") } else { 0.0 };
        }
        steer_y(cx, s, vy, speed);
        t -= step(cx);
        frame().await;
        if stuck(cx, k(cx, "hover_stuck"), k(cx, "stuck_half")) {
            break;
        }
    }
}

fn fly_numbers(cx: &Cx, request: u16, p: &str, time: f32) -> FlyTo {
    let n = |s: &str| k(cx, &format!("{p}_{s}"));
    FlyTo {
        request,
        speed: n("speed"),
        stop: n("stop"),
        up: n("up"),
        high: n("high"),
        time,
        blend: k(cx, "fly_blend"),
        stuck: k(cx, "fly_stuck"),
        stuck_half: k(cx, "stuck_half"),
        no_motion: k(cx, "fly_no_motion") as u16,
    }
}

/// `0x652`: the approach: toward the target (re-read each frame) aimed 100 above it at 8.4 until within 500, 240 frames
/// at most, the hover motion (2) playing.
async fn approach(cx: &Cx) {
    let n = fly_numbers(cx, req::HOVER, "approach", k(cx, "approach_time"));
    fly_to(cx, n, tgt).await;
}

/// `0x5EA`: the flee: a point [500, 600) away in a random direction, flown to at 14 (217 playing) until within 100,
/// [120, 180) frames at most; then turning to the target.
async fn flee(cx: &Cx) {
    let p = pos(cx) + random_offset(cx, k(cx, "flee_min"), k(cx, "flee_max"));
    let time = rand_range(cx, k(cx, "flee_time_min"), k(cx, "flee_time_max"), "flee time");
    let n = fly_numbers(cx, req::FLEE, "flee", time);
    fly_to(cx, n, move |_| p).await;
    yaw_tgt(cx);
}

/// `0x93E`'s numbers.
fn wander(cx: &Cx) -> Wander {
    Wander {
        request: req::WANDER,
        blend: k(cx, "wander_blend"),
        turn: k(cx, "wander_turn"),
        sink: k(cx, "wander_sink"),
        flip: k(cx, "flip"),
        low: k(cx, "wander_low"),
        min: k(cx, "wander_min"),
        max: k(cx, "wander_max"),
        speed: k(cx, "wander_speed"),
    }
}

/// The idle behaviour (`0x8F4`): idle legs, a yield between them.
async fn idle(cx: Cx) {
    loop {
        let n = wander(&cx);
        wander_leg(&cx, n).await;
        frame().await;
    }
}

/// `0xD1A`: the notice (a room-placed one, spawn type 0, outside KH1's world 11): turning to the target, 214 (blend 8)
/// to its end, then the idle 220 (blend 0).
async fn notice(cx: &Cx) {
    if cx.borrow().x.world == Some(k(cx, "notice_world") as u8) {
        return;
    }
    yaw_tgt(cx);
    request(cx, req::NOTICE, k(cx, "notice_blend"), 0.0);
    wait_cast(cx, req::NOTICE).await;
    request(cx, req::REST, 0.0, 0.0);
}

/// The engaged behaviour (`0x384`): seen (the notice once for a placed one); a yield; then one round by the distance:
/// within 500 or 700 (the same numbers): 40 % the strafe and Fire, then the circling hover; farther: 50 % the big Fire,
/// then the approach; then engaged again (the search and the target, `0xC5A`: Sora).
async fn engaged(cx: Cx) {
    if !cx.borrow().x.seen {
        cx.borrow_mut().x.seen = true;
        if !cx.borrow().x.spawned {
            notice(&cx).await;
        }
    }
    frame().await;
    let d = dist(&cx);
    if d - k(&cx, "close_range") < 0.0 || d - k(&cx, "mid_range") < 0.0 {
        if rand(&cx, "near fire") - k(&cx, "near_fire_chance") < 0.0 {
            strafe_fire(&cx).await;
        }
        hover(&cx).await;
    } else {
        if rand(&cx, "far fire") - k(&cx, "far_bigfire_chance") < 0.0 {
            big_fire(&cx).await;
        }
        approach(&cx).await;
    }
    set_state(&cx, st::ENGAGED);
}

/// The hit behaviour (`0x32C`): within 500, the flee and the big Fire; then engaged.
async fn hit(cx: Cx) {
    if dist(&cx) - k(&cx, "hit_close_range") < 0.0 {
        flee(&cx).await;
        big_fire(&cx).await;
    }
    set_state(&cx, st::ENGAGED);
}

/// The idle start (`0x910`): a spawned one's appearance (`0xE94`: the 30-frame fade-in, 69 as an action with the idle
/// hover 219 queued, then facing the target at once); then idle.
async fn idle_start(cx: Cx) {
    if cx.borrow().x.spawned {
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        push(&cx, ECmd::Note("effect 2, flag 0x80 (M13)"));
        push(&cx, ECmd::Action { request: req::APPEAR, attack: None, then: req::WANDER, blend: k(&cx, "appear_blend") });
        frame().await;
        // `0xF0C`: `0x1BAC(actor, 0x16F2())`: Sora (party 0) at once, whether or not he is its target yet.
        let d = tgt(&cx) - pos(&cx);
        let y = d.x.atan2(d.z);
        let mut io = cx.borrow_mut();
        io.body.yaw = y;
        io.body.yaw_target = y;
        io.out.push(ECmd::Yaw(y));
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The supervisor (entry `0x1E48`): the init's yield, the idle start (`0x900`); then each frame the idle search
/// (`0xA52`) and the state requests.
async fn supervisor(cx: Cx) {
    frame().await;
    cx.borrow_mut().x.boot = true;
    loop {
        let found = {
            let io = cx.borrow();
            io.state.0 == st::IDLE && io.w.target.is_some_and(|t| (t - io.w.pos).length() - io.c.num("idle_search") < 0.0)
        };
        if found {
            set_state(&cx, st::ENGAGED);
        }
        if let Some(s) = take_state(&cx) {
            push(&cx, ECmd::State(s));
        }
        frame().await;
    }
}

/// The Red Nocturne's AI for one enemy.
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

    /// Its spell children's waits this frame (before the script: a child made this frame waits from the next).
    fn children(&mut self) {
        let mut io = self.io.borrow_mut();
        let (m, s) = (io.w.motion, io.w.step);
        let thrown = step_casts(&mut io.x.casts, m, s);
        io.out.extend(thrown);
    }

    /// One AI frame. Returns the commands.
    pub fn frame(&mut self, w: World) -> Vec<Cmd> {
        {
            let mut io = self.io.borrow_mut();
            io.w = w;
            tick_clock(&mut io);
        }
        self.children();
        if !self.started {
            self.started = true;
            self.io.borrow_mut().state = (st::NONE, false);
            // `0xC94`: the flight's easing 0.2, flight with no velocity.
            let a = k(&self.io, "air_accel_set");
            set_air_accel(&self.io, a);
            fly(&self.io, Vec3::ZERO);
            self.main = Some(Box::pin(supervisor(self.io.clone())));
        }
        poll(&mut self.main);
        if std::mem::take(&mut self.io.borrow_mut().x.boot) {
            self.behaviour = Some(Box::pin(idle_start(self.io.clone())));
        }
        let restart = self.io.borrow().out.iter().rev().find_map(|c| match c {
            ECmd::State(s) => Some(*s),
            _ => None,
        });
        if let Some(s) = restart {
            let cx = self.io.clone();
            set_target(&cx, s != st::IDLE);
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
    /// KH1's children run while the script waits (a reaction plays): the spell children's waits.
    fn side_frame(&mut self, w: World) -> Vec<ECmd<u8>> {
        {
            let mut io = self.io.borrow_mut();
            io.w = w;
        }
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
    /// The hit handler (`0x1F1E`): seen; a hit of its own kind's fire sent back: KH1's EXP bonus (`sys 194(EXP / 10 +
    /// 2)`) and, not built, `sys 179(1800)`; a living damaging hit or a clash asks for the hit state; a repel record
    /// without a clash asks for nothing.
    fn on_hit(&self, news: HitNews) {
        let mut io = self.io.borrow_mut();
        io.x.seen = true;
        match news {
            HitNews::Damage => {
                if io.x.own {
                    let r = Reward { exp_div: io.c.num("exp_div") as i32, exp_add: io.c.num("exp_add") as i32, exp_min: 0, prizes: [0; 7] };
                    io.x.now.push(ECmd::Reward(r));
                    io.x.now.push(ECmd::Note("sys 179(1800): KH1's dizzy status, not built (owner question)"));
                }
                io.state = (st::HIT, true);
            }
            HitNews::Clash => io.state = (st::HIT, true),
            HitNews::Repelled => {}
        }
    }
    fn take_now(&self) -> Vec<ECmd<u8>> {
        std::mem::take(&mut self.io.borrow_mut().x.now).iter().map(|c| c.map(Attack::index)).collect()
    }
    /// The killing hit: its death follows (`0x1518`: Re:CoM's defeat here); its own fire's EXP bonus comes only for a
    /// living hit (`0x1F68`).
    fn on_defeat(&self) {
        let mut io = self.io.borrow_mut();
        io.x.seen = true;
        io.x.dying = true;
    }
    fn risen(&self) -> bool {
        self.io.borrow().x.appeared
    }
    fn skip_rise(&self) {
        self.io.borrow_mut().x.spawned = false;
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["none", "idle", "engaged", "hit"].get(s as usize).copied().unwrap_or("?")
    }
    fn set_floor(&self, y: f32) {
        self.io.borrow_mut().floor = Some(y);
    }
    fn set_world(&self, w: Option<u8>) {
        self.io.borrow_mut().x.world = w;
    }
    fn set_hit_own_row(&self, own: bool) {
        self.io.borrow_mut().x.own = own;
    }
    /// The host's quiet watchdog: engaged again, flying with no velocity.
    fn reset(&self) {
        {
            let mut io = self.io.borrow_mut();
            io.x.appeared = true;
            io.x.seen = true;
            io.body.fly(Vec3::ZERO);
        }
        set_state(&self.io, st::ENGAGED);
    }
}

/// `sys 19(parent, 0x8001, t)` then `up` (`ex_2110_00.bd` `0x75E..0x788`): the actor's place plus its matrix's z row
/// (`+0xF0` = −facing, `0x1402A0A40`: `0 − +0x30`) × `t × 100` (`0x14029EB20`'s last case: `(0, 0, t × 100, 0)` through
/// `+0xD0`, Checked (decomp)), then `up` on its height (y down). With t −0.5: 50 ahead along its facing.
pub fn fire_start(pos: Vec3, yaw: f32, t: f32, up: f32) -> Vec3 {
    let z = -Vec3::from_yaw(yaw);
    let mut p = pos + z * (t * 100.0);
    p.y += up;
    p
}

#[cfg(test)]
mod tests;
