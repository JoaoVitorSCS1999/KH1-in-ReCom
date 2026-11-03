//! KH1 FM's Yellow Opera (`xa_ex_2130.mdls` part 5, `ex_2130.bd`), engine-agnostic: its AI script on the shared enemy
//! framework (`crate::enemy`), flying (KH1's movement state 4, `crate::enemy::Body::flying`), with the casting family's
//! shared routines (`crate::caster`: the idle leg, fly-to, the stuck test) and its spell child's wait ([`Strike`]:
//! `ex_2130_00.bd` the Thunder; part 3). M11 (`docs/recom-m11-enemies.md` § The Yellow Opera). Re:CoM runs it natively:
//! each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), its charge in Re:CoM's own rush state
//! (3, its `.ASD` records), its Thunder one of Re:CoM's own bullets placed by KH1's numbers (`win::kh1_thrown`).
//!
//! Every number is data ([`NAMES`], [`THUNDER_NAMES`], read by the asset step from the player's `ex_2130.bd` and its child
//! at checked offsets). Script offsets are the player's own (Checked (files): the scripts disassembled with
//! `tools/kh1-ai-script.py`, the parent's code from `0x308`, the child's from `0x18`; the syscalls named are Checked
//! (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x1F3C`: init `0x1FBC` → `0xDBA` (records LV1 `0x14` / LV2 `0x80`; the stats block's `+0x14` = 0.2 (the
//!   flight's easing), `+0x18` = 0.95; flight with no velocity (`sys 135`); the state and search objects; home; seen
//!   cleared); the handlers (hit `0x2000`, message `0x20BC`: answers 0, the hit-effect filter `0x1FE0`: a Thunder hit's
//!   effect 13, element: left out); a yield; the idle start `0xA26` → `0xA36`; then each frame the idle search (`0xB78`: in
//!   state 1, Sora within 1000 → engaged) and a state request restarts the behaviour (1 idle `0xA02` → `0xA1A`, the target
//!   cleared; 2 engaged `0x34A` → `0x36E`; 3 hit `0x308` → `0x32C`; both with the target). No `sys 125`: KH1's π/16.
//! - Every motion it plays is a plain request (no action bit) except its appearance (69): the script never pauses for its
//!   own motions, only for KH1's reactions. Its charge 209 makes two rows on itself (`sys 23`: `0xEC` the hit, `0x15C` a
//!   type-2 body row that only clashes, mode 2) with one hit serial and the rows' hit callback `0x79A` (`wp+0` = 2: the
//!   rush stops once it lands): a steered attack ([`ECmd::Attack`]); its Thunder's row `0x23C` is its child's.
//! - the hit handler `0x2000`: seen; an attribute-8 hit's EXP bonus (`0xF58`; attributes: not carried); a Thunder hit
//!   (attribute 4, `0xEEE`): no damage, HP to max, no reaction (element: left out); `wp+0` = 1 (the rush's "hit" mark); the
//!   damage (`0x192A`: a clash or a repel −1; killed 0 → its death `0x15EE`); −1: the hit state only for a clash (or
//!   attributes 8 / 10, `0x158C`); alive: the hit state. KH1's reaction plays for every hit (the handler zeroes the kind
//!   only for a Thunder hit).
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0xEA0` (`0x1158`: 1500 / 750), the notice `0xE40` for a
//! room-placed one (spawn type 0: battle spawns play KH1's appearance, owner 2026-11-02), the search object (`sys 48`:
//! Sora is the only target; `sys 139`'s out-of-sight test taken as in sight, as every port), the element rules
//! (attributes are not carried, owner), effects (`sys 75` / `sys 225`, the child's 2056 / 9 / 2058 / 14 / 11 / 12, M13),
//! the appearance's flag 0x80, the death routine (Re:CoM's defeat, prizes, EXP).

use crate::caster::{fly_to, norm, play, step_strikes, stuck, wander_leg, FlyTo, Strike, Wander};
use crate::enemy::script::{fly, frame, height, motion_done, poll, pos, push, rand, random_offset, set_air_accel, set_state, set_target, steer, take_state, target, tick_clock, wait_frames};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Yellow Opera's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The Thunder's charge (`0x466`, from frame 8), 104 frames, root motion; its hold 206 (queued from frame 2), 70; its
    /// cast 207 (queued from frame 3), 164, root motion: the child marks 80 frames into 207 and strikes 30 later.
    pub const CAST: u16 = 205;
    pub const HOLD: u16 = 206;
    pub const STRIKE: u16 = 207;
    /// The charge's wind-up (`0x508`, from frame 8), 126 frames, root motion; its rush 209 (looping, 40 frames, its rows'
    /// HIT window the whole clip), up to 150 frames; its recovery 210 (from frame 2), 54 frames, root motion.
    pub const WIND_UP: u16 = 208;
    pub const RUSH: u16 = 209;
    pub const RECOVER: u16 = 210;
    /// The rush rows' clash recoil (row `+0x40`), 80 frames.
    pub const RUSH_RECOIL: u16 = 211;
    /// The appearance (`0xFBA`: an action, 219 queued), 90 frames, root motion.
    pub const APPEAR: u16 = 69;
    /// The notice (`0xE40`, a room-placed one only), 158 frames.
    pub const NOTICE: u16 = 214;
    /// The idle hover (`0xA64`), 200 frames.
    pub const WANDER: u16 = 219;
    /// The flight and rest motion (entry 0's motion), 200 frames.
    pub const REST: u16 = 220;
}

/// The Yellow Opera's attack (the host's attack list, `kh1-recom`'s class row).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    /// 209 with rows `0xEC` / `0x15C`.
    Charge,
}

impl Attack {
    pub const ALL: [Attack; 1] = [Attack::Charge];

    pub fn index(self) -> u8 {
        match self {
            Attack::Charge => 0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Charge => "charge",
        }
    }
}

/// Its spell child (`sys 94`'s sub-object index): the host's strike ([`ECmd::Strike`]).
pub mod child {
    /// `ex_2130_00.bd`: the Thunder (`0x466`, row `0x23C`).
    pub const THUNDER: u8 = 0;
}

/// The names of the Yellow Opera's numbers (besides `crate::enemy::ENGINE_NAMES` and [`THUNDER_NAMES`]).
pub const NAMES: &[&str] = &[
    "close_range",
    "mid_range",
    "thunder_chance",
    "charge_chance",
    "cast_blend",
    "cast_start",
    "hold_blend",
    "hold_start",
    "strike_blend",
    "strike_start",
    "rest_blend",
    "rest_start",
    "thunder_child",
    "charge_blend",
    "charge_start",
    "rush_blend",
    "rush_time",
    "rush_stop",
    "rush_high",
    "rush_speed",
    "rush_stuck",
    "recover_blend",
    "recover_start",
    "flee_legs",
    "flee_min",
    "flee_vert",
    "flee_last_up",
    "flee_speed",
    "flee_stop",
    "flee_up",
    "flee_high",
    "flee_time",
    "drop_probe",
    "drop_floor_min",
    "drop_up",
    "drop_stop",
    "move_min",
    "move_vert",
    "move_speed",
    "move_stop",
    "move_up",
    "move_high",
    "move_time",
    "move_pause",
    "point_band",
    "point_up",
    "pause_blend",
    "wander_turn",
    "wander_sink",
    "wander_low",
    "flip",
    "wander_min",
    "wander_max",
    "wander_blend",
    "wander_speed",
    "idle_search",
    "fly_no_motion",
    "fly_blend",
    "fly_stuck",
    "stuck_half",
    "air_accel_set",
    "notice_world",
    "notice_blend",
    "wait_motion",
    "appear_fade",
    "appear_blend",
    "appear_queue_blend",
    "exp_div",
    "exp_add",
];

/// Its child's numbers (`ex_2130_00.bd`): the wait in the cast (80 frames) and the delay to the strike (30) the behaviour
/// runs ([`Strike`]); the area (radius 50, half-height 128 centred 128 above the mark) the host's; the cloud's height
/// (200 above the mark) the host's warning (`ECmd::Warn`: a Re:CoM effect there until the strike, owner 2026-11-03).
pub const THUNDER_NAMES: &[&str] = &["thunder_wait", "thunder_cloud", "thunder_delay", "thunder_radius", "thunder_half", "thunder_mid"];

/// Every name the manifest must hold for the behaviour.
pub fn all_names() -> Vec<&'static str> {
    NAMES.iter().chain(THUNDER_NAMES).copied().collect()
}

/// The Yellow Opera's numbers.
pub type Consts = Table;

/// The state objects (`wp+0x20`), KH1's own numbers.
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 3;
}

/// What `wp+0` holds during the rush (`0x5AC`: 0 at its start; the hit handler writes 1, the rows' hit callback `0x79A` 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mark {
    #[default]
    None,
    Hit,
    Landed,
}

/// The Yellow Opera's own work area: `wp+0` ([`Mark`]), `wp+0x38` (seen: no notice), the appearance, the boot (`0xA26`),
/// its waiting spell child, the world, what the handler does at once.
#[derive(Clone, Debug, Default)]
struct Wp {
    mark: Mark,
    seen: bool,
    spawned: bool,
    appeared: bool,
    boot: bool,
    dying: bool,
    strikes: Vec<Strike>,
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

/// `0x16CE` (`0x1A58(actor, target)`): the distance to the target (3D).
fn dist(cx: &Cx) -> f32 {
    (tgt(cx) - pos(cx)).length()
}

/// `0x1C76` (`sys 36`): the target yaw toward a point (the engine turns).
fn yaw_to(cx: &Cx, p: Vec3) {
    let d = p - pos(cx);
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

fn yaw_tgt(cx: &Cx) {
    let t = tgt(cx);
    yaw_to(cx, t);
}

/// `0x1CA0(actor, target)` (`sys 11`): facing the target at once.
fn face_now(cx: &Cx) {
    let d = tgt(cx) - pos(cx);
    let y = d.x.atan2(d.z);
    let mut io = cx.borrow_mut();
    io.body.yaw = y;
    io.body.yaw_target = y;
    io.out.push(ECmd::Yaw(y));
}

/// `sys 13` (`0x1402B9F80` → `0x14029F980`): a request written whatever plays (the playing one starts again), from
/// `start` frames (the behaviour's clock starts there: [`motion_done`] counts the motion's own frames).
fn request(cx: &Cx, r: u16, blend: f32, start: f32) {
    let again = {
        let mut io = cx.borrow_mut();
        let again = io.w.motion == Some(r);
        io.clock = (Some(r), start);
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

/// `0x169C`: request 6 plays (it also ends the waits).
fn wait_motion_plays(cx: &Cx) -> bool {
    cx.borrow().w.motion == Some(k(cx, "wait_motion") as u16)
}

/// `0x1668`: until the motion asked for has played to its end (`sys 172`) or request 6 plays, turning to the target each
/// frame (`0x1C76`).
async fn wait_motion(cx: &Cx, last: u16) {
    loop {
        if cx.borrow().w.motion == Some(last) && motion_done(cx) {
            break;
        }
        if wait_motion_plays(cx) {
            break;
        }
        yaw_tgt(cx);
        frame().await;
    }
}

/// `0x1C58`: a yield, then until the motion playing has played to its end (`sys 172`).
async fn wait_end(cx: &Cx) {
    loop {
        frame().await;
        if motion_done(cx) {
            break;
        }
    }
}

/// The floor's height under a point (`0x1760` / the child's `0x18`: `sys 77` straight down; the battle floor here:
/// Re:CoM's battle floors are flat, Assumption as every flier's), KH1 y.
fn floor_under(cx: &Cx, p: Vec3) -> f32 {
    cx.borrow().floor.unwrap_or(p.y)
}

/// `0x934(a, vert)`: a point [a, a + 100) from the target in a random direction (`0x18C6`), 50 above the target's place;
/// then `vert` farther on the other side of that height from the Yellow Opera's (KH1 y down: below it when it is level or
/// higher, else above it).
fn point_by(cx: &Cx, a: f32, vert: f32) -> Vec3 {
    let off = random_offset(cx, a, a + k(cx, "point_band"));
    let mut p = tgt(cx) + off;
    p.y -= k(cx, "point_up");
    let me = pos(cx).y;
    if me - p.y <= 0.0 {
        p.y += vert;
    } else if me - p.y > 0.0 {
        p.y -= vert;
    }
    p
}

/// `0xBE2(220, point, speed, stop, up, high, time)` with the routine's own numbers, to a fixed point.
async fn fly_to_point(cx: &Cx, p: Vec3, n: &str, time: f32) {
    let f = FlyTo {
        request: req::REST,
        speed: k(cx, &format!("{n}_speed")),
        stop: k(cx, &format!("{n}_stop")),
        up: k(cx, &format!("{n}_up")),
        high: k(cx, &format!("{n}_high")),
        time,
        blend: k(cx, "fly_blend"),
        stuck: k(cx, "fly_stuck"),
        stuck_half: k(cx, "stuck_half"),
        no_motion: k(cx, "fly_no_motion") as u16,
    };
    fly_to(cx, f, move |_| p).await;
}

/// `0x8D6`: the reposition: a point 200..300 from the target, the other side of its head height by 150 (`0x934(200,
/// 150)`), flown to at 15 (stop 10, 30 frames, 220 playing); turning to the target (`0x1C76`); `0x9E2(25)`: 220 (`sys 12`,
/// blend 8) and 25 frames.
async fn reposition(cx: &Cx) {
    let p = point_by(cx, k(cx, "move_min"), k(cx, "move_vert"));
    fly_to_point(cx, p, "move", k(cx, "move_time")).await;
    yaw_tgt(cx);
    play(cx, req::REST, k(cx, "pause_blend"));
    let n = k(cx, "move_pause");
    wait_frames(cx, n).await;
}

/// `0x7AA`: the flee (the hit state): three legs to points 200..300 from the target on the other side of its head height
/// by 250 (`0x934(200, 250)`), the last 50 higher, at 40 (stop 10, 20 frames each, a yield between them); then down to 100
/// above the floor under it (probed 20 above it, `0x1760`) at 40 (stop 30, 20 frames), when that floor is above KH1's world
/// height −400 (read as a height above the battle floor, owner 2026-11-02: the battle floor itself, 0 > −400: always).
async fn flee(cx: &Cx) {
    let mut n = k(cx, "flee_legs");
    while n > 0.0 {
        let mut p = point_by(cx, k(cx, "flee_min"), k(cx, "flee_vert"));
        if n - 1.0 <= 0.0 {
            p.y -= k(cx, "flee_last_up");
        }
        fly_to_point(cx, p, "flee", k(cx, "flee_time")).await;
        n -= 1.0;
        frame().await;
    }
    let me = pos(cx);
    let probe = Vec3::new(me.x, me.y - k(cx, "drop_probe"), me.z);
    let f = floor_under(cx, probe);
    // The world height as a height above the battle floor (the floor found: 0 above it).
    let above = cx.borrow().floor.map_or(f, |b| f - b);
    if above - k(cx, "drop_floor_min") > 0.0 {
        let p = Vec3::new(me.x, f - k(cx, "drop_up"), me.z);
        let n = FlyTo {
            request: req::REST,
            speed: k(cx, "flee_speed"),
            stop: k(cx, "drop_stop"),
            up: k(cx, "flee_up"),
            high: k(cx, "flee_high"),
            time: k(cx, "flee_time"),
            blend: k(cx, "fly_blend"),
            stuck: k(cx, "fly_stuck"),
            stuck_half: k(cx, "stuck_half"),
            no_motion: k(cx, "fly_no_motion") as u16,
        };
        fly_to(cx, n, move |_| p).await;
    }
}

/// `0x466`: the Thunder: its child (`sys 94(actor, 0, target, 0x23C, 0)`), 205 (blend 8, from frame 8), 206 queued (blend
/// 0, from frame 2) and 207 queued (blend 0, from frame 3; asked for here when 206 starts: the host has one queued slot);
/// until 207 has played (or request 6 plays) turning to the target; then 220 (blend 8, from frame 8).
async fn thunder(cx: &Cx) {
    let s = Strike::new(child::THUNDER, req::CAST, req::HOLD, req::STRIKE, k(cx, "thunder_wait"), k(cx, "thunder_delay"));
    cx.borrow_mut().x.strikes.push(s);
    push(cx, ECmd::Note("sys 94: the Thunder waits on the cast (its effects 2056 / 9 / 2058: M13)"));
    request(cx, req::CAST, k(cx, "cast_blend"), k(cx, "cast_start"));
    push(cx, ECmd::QueueAt { request: req::HOLD, blend: k(cx, "hold_blend"), start: k(cx, "hold_start") });
    let mut seen = (false, false);
    loop {
        let m = cx.borrow().w.motion;
        // The queued requests start from their frames (KH1's `sys 14` start word): the clock with them.
        if m == Some(req::HOLD) && !seen.0 {
            seen.0 = true;
            cx.borrow_mut().clock.1 += k(cx, "hold_start");
            push(cx, ECmd::QueueAt { request: req::STRIKE, blend: k(cx, "strike_blend"), start: k(cx, "strike_start") });
        }
        if m == Some(req::STRIKE) && !seen.1 {
            seen.1 = true;
            cx.borrow_mut().clock.1 += k(cx, "strike_start");
        }
        if m == Some(req::STRIKE) && motion_done(cx) {
            break;
        }
        if wait_motion_plays(cx) {
            break;
        }
        yaw_tgt(cx);
        frame().await;
    }
    request(cx, req::REST, k(cx, "rest_blend"), k(cx, "rest_start"));
}

/// `0x508`: the charge: 208 (blend 8, from frame 8) to its end (turning to the target); then 209 (`sys 12`, blend 0) with
/// its two rows: a steered attack; up to 150 frames toward where the target stood then (fixed), at 25 a frame, its height
/// followed (straight down while above 500 over the floor with the target below it), until within 30 (flat: the speed set
/// to 0), the rows landed (`wp+0` = 2), a hit came (`wp+0` = 1) or stuck (12.5); unless a hit came: 210 (blend 0, from
/// frame 2) to its end, then 220 (blend 8, from frame 8).
async fn charge(cx: &Cx) {
    request(cx, req::WIND_UP, k(cx, "charge_blend"), k(cx, "charge_start"));
    wait_motion(cx, req::WIND_UP).await;
    cx.borrow_mut().clock = (Some(req::RUSH), 0.0);
    push(cx, ECmd::Attack { request: req::RUSH, attack: Attack::Charge, blend: k(cx, "rush_blend") });
    cx.borrow_mut().x.mark = Mark::None;
    let goal = tgt(cx);
    let mut t = k(cx, "rush_time");
    let mut first = true;
    while t > 0.0 && cx.borrow().x.mark == Mark::None {
        let mut v = goal - pos(cx);
        yaw_to(cx, goal);
        if Vec3::new(v.x, 0.0, v.z).length() - k(cx, "rush_stop") < 0.0 {
            // `0x650`: the speed `+0x2C` = 0 (its last direction kept).
            steer(cx, Vec3::ZERO, 0.0);
            break;
        }
        if v.y > 0.0 {
            if pos(cx).y - goal.y > 0.0 {
                v.y = 0.0;
            } else if height(cx) - k(cx, "rush_high") > 0.0 {
                v.x = 0.0;
                v.z = 0.0;
            }
        }
        let v = norm(v);
        steer(cx, v, k(cx, "rush_speed"));
        // `sys 12(209)` each frame: nothing while it plays; the frame it is asked for, the host's attack state plays it
        // at its begin (the next update).
        if !std::mem::take(&mut first) {
            play(cx, req::RUSH, k(cx, "rush_blend"));
        }
        t -= step(cx);
        frame().await;
        if stuck(cx, k(cx, "rush_stuck"), k(cx, "stuck_half")) {
            break;
        }
    }
    if cx.borrow().x.mark != Mark::Hit {
        request(cx, req::RECOVER, k(cx, "recover_blend"), k(cx, "recover_start"));
        wait_end(cx).await;
        request(cx, req::REST, k(cx, "rest_blend"), k(cx, "rest_start"));
    }
}

/// `0x438` (under 250 and under 500): a reposition, then 50 % the charge, else another reposition.
async fn near_round(cx: &Cx) {
    reposition(cx).await;
    if rand(cx, "charge") - k(cx, "charge_chance") < 0.0 {
        charge(cx).await;
    } else {
        reposition(cx).await;
    }
}

/// `0x408` (farther): 60 % the Thunder; then two repositions.
async fn far_round(cx: &Cx) {
    if rand(cx, "thunder") - k(cx, "thunder_chance") < 0.0 {
        thunder(cx).await;
    }
    reposition(cx).await;
    reposition(cx).await;
}

/// `0xA64`'s numbers.
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

/// The idle behaviour (`0xA1A`): idle legs, a yield between them.
async fn idle(cx: Cx) {
    loop {
        let n = wander(&cx);
        wander_leg(&cx, n).await;
        frame().await;
    }
}

/// `0xE40`: the notice (a room-placed one, spawn type 0, outside KH1's world 11): turning to the target, 214 (blend 8)
/// until it has played, then the idle 220 (blend 0).
async fn notice(cx: &Cx) {
    if cx.borrow().x.world == Some(k(cx, "notice_world") as u8) {
        return;
    }
    yaw_tgt(cx);
    request(cx, req::NOTICE, k(cx, "notice_blend"), 0.0);
    wait_motion(cx, req::NOTICE).await;
    request(cx, req::REST, 0.0, 0.0);
}

/// The engaged behaviour (`0x36E`): seen (the notice once for a placed one); a yield; one round by the distance: under
/// 250 or under 500 the near round, farther the far round; then engaged again (the search and the target, `0xD80`: Sora).
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
        near_round(&cx).await;
    } else {
        far_round(&cx).await;
    }
    set_state(&cx, st::ENGAGED);
}

/// The hit behaviour (`0x32C`): the flee, then the Thunder (whatever the distance); then engaged.
async fn hit(cx: Cx) {
    flee(&cx).await;
    thunder(&cx).await;
    set_state(&cx, st::ENGAGED);
}

/// The idle start (`0xA36`): a spawned one's appearance (`0xFBA`: the 30-frame fade-in, 69 as an action with the idle
/// hover 219 queued, then facing Sora at once); then idle.
async fn idle_start(cx: Cx) {
    if cx.borrow().x.spawned {
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        push(&cx, ECmd::Note("effect 2, flag 0x80 (M13)"));
        push(&cx, ECmd::Action { request: req::APPEAR, attack: None, then: req::WANDER, blend: k(&cx, "appear_blend") });
        frame().await;
        // `0x1032`: `0x1CA0(actor, 0x17C8())`: Sora (party 0) at once, whether or not he is its target yet.
        face_now(&cx);
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The supervisor (entry `0x1F3C`): the init's yield, the idle start (`0xA26`); then each frame the idle search (`0xB78`)
/// and the state requests.
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

/// The Yellow Opera's AI for one enemy.
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

    /// Its spell child's wait this frame (before the script: a child made this frame waits from the next).
    fn children(&mut self) {
        let mut io = self.io.borrow_mut();
        let (m, s, t, f) = (io.w.motion, io.w.step, io.w.target, io.floor);
        let out = step_strikes(&mut io.x.strikes, m, s, t, f);
        io.out.extend(out);
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
            // `0xDBA`: the flight's easing 0.2, flight with no velocity.
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
    /// KH1's children run while the script waits (a reaction plays): the Thunder's wait.
    fn side_frame(&mut self, w: World) -> Vec<ECmd<u8>> {
        {
            let mut io = self.io.borrow_mut();
            io.w = w;
        }
        self.children();
        std::mem::take(&mut self.io.borrow_mut().out).iter().map(|c| c.map(Attack::index)).collect()
    }
    /// After its death, while Re:CoM keeps it (KH1's dying Yellow Opera fades 30 frames before it is removed, `0x15EE` →
    /// `0x1AE6` → `0x1D8E(actor, 30)`): its Thunder goes on by its own rule (before the mark the killing hit's reaction
    /// ends it; after the mark nothing tests its parent, so it strikes).
    fn dead_frame(&mut self, w: World) -> Vec<ECmd<u8>> {
        self.side_frame(w)
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
    /// The hit handler (`0x2000`) without the element rules (attributes are not carried): seen; `wp+0` = 1 (the rush's
    /// "hit" mark, `0x203A`, before the damage); a living damaging hit or a clash asks for the hit state; a repel record
    /// without a clash asks for nothing.
    fn on_hit(&self, news: HitNews) {
        let mut io = self.io.borrow_mut();
        io.x.seen = true;
        io.x.mark = Mark::Hit;
        match news {
            HitNews::Damage | HitNews::Clash => io.state = (st::HIT, true),
            HitNews::Repelled => {}
        }
    }
    /// The rush rows' hit callback (`0x79A`: `wp+0` = 2): the rush stops.
    fn on_landed(&self, attack: u8) {
        if attack == Attack::Charge.index() {
            self.io.borrow_mut().x.mark = Mark::Landed;
        }
    }
    fn take_now(&self) -> Vec<ECmd<u8>> {
        std::mem::take(&mut self.io.borrow_mut().x.now).iter().map(|c| c.map(Attack::index)).collect()
    }
    /// The killing hit: its death follows (`0x15EE`: Re:CoM's defeat here).
    fn on_defeat(&self) {
        let mut io = self.io.borrow_mut();
        io.x.seen = true;
        io.x.mark = Mark::Hit;
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
    /// Re:CoM's Stop, stun, freeze or flatten held it (its own state, decision M10): its Thunder ends, a marked one too
    /// (owner 2026-11-03: Re:CoM's Stop keeps cancelling a caster's spell; KH1's marked Thunder tests nothing of its parent).
    fn on_recom_state(&self) {
        self.io.borrow_mut().x.strikes.clear();
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

#[cfg(test)]
mod tests;
