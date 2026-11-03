//! KH1 FM's Green Requiem (`xa_ex_2140.mdls` part 5, `ex_2140.bd`), engine-agnostic: its AI script on the shared enemy
//! framework (`crate::enemy`), flying (KH1's movement state 4, `crate::enemy::Body::flying`), with the casting family's
//! shared routines (`crate::caster`) and its spell child's wait ([`Cast`]: `ex_2140_00.bd` the Cure; part 3). M11
//! (`docs/recom-m11-enemies.md` § The Green Requiem). Re:CoM runs it natively: each part is the step of a Re:CoM enemy
//! state (`kh1-recom`'s `win::kh1_enemy`); its Cure is Re:CoM's own Green Requiem heal on the ally KH1 picks
//! (`win::kh1_cure`).
//!
//! Every number is data ([`NAMES`], [`CURE_NAMES`], read by the asset step from the player's `ex_2140.bd` and its child at
//! checked offsets). Script offsets are the player's own (Checked (files): the scripts disassembled with
//! `tools/kh1-ai-script.py`, the parent's code from `0x1B8`, the child's from `0x18`; the syscalls named are Checked
//! (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x1BBE`: init `0x1C3E` → `0x9EA` (records LV1 `0x14` / LV2 `0x80`; the stats block's `+0x14` = 0.2 (the
//!   flight's easing), `+0x18` = 0.95; flight with no velocity (`sys 135`); the state object; the search object with its
//!   table `0x190`; home; seen cleared); the handlers (hit `0x1CAA`, message `0x1D80`: answers 0, the hit-effect filter
//!   `0x1C62`: element effects); a yield; the idle start `0x656` → `0x666`; then each frame the idle search (`0x7A8`: in
//!   state 1, Sora within 1000 → engaged) and a state request restarts the behaviour (1 idle `0x632` → `0x64A`, the target
//!   cleared; 2 engaged `0x1FA` → `0x21E`; 3 hit `0x1B8` → `0x1DC`, the hitter or Sora). No `sys 125`: KH1's π/16. The
//!   actor sits at `wp+0` (the Red Nocturne's and Blue Rhapsody's at `wp+4`): its copies of the family's idle leg
//!   (`0x694`, its numbers in locals), fly-to (`0x812`) and stuck test (`0xD10`) are the same routines as theirs but for
//!   that offset (Checked (files): `0x812..0x9B0` against the Red Nocturne's `0xABC`, 6 bytes differ, each the actor's
//!   load), so [`crate::caster`]'s serve.
//! - Every motion it plays is a plain request except its appearance (69): the script never pauses for its own motions.
//!   It makes no attack row that hits Sora; its one row (`0xEC`: power kind 0, **attribute 11**, reaction kind 0,
//!   free-standing) is its child's, which hands it straight to the target it was given (`sys 60`).
//! - the hit handler `0x1CAA`: a Fire / Blizzard / Thunder hit (attributes 2 / 3 / 4, `0xB1E`): no damage, HP to max, no
//!   reaction (element: left out); attribute 8: KH1's EXP bonus (`0xB88`, `EXP / 10 + 2`; attributes: not carried); seen;
//!   the damage (`0x15CA`: a clash or a repel −1; killed 0 → its death `0x126E`); −1: the hit state only for a clash or
//!   attributes 8 / 10 (`0x120C`); alive: the hit state. KH1's reaction plays for every hit (the handler zeroes the kind
//!   only for the element hits).
//!
//! # Its Cure (Checked (files + decomp))
//!
//! `0x2CA`: a point [500, 600) from itself (`0x1566`, two draws; never read again: KH1's own leftover); **the pick**
//! `sys 102(actor, entry 1, 5000, 1)` ([`pick`]); its child `sys 94(actor, 0, picked or itself, 0xEC, 0xEC)`; 212 at blend
//! 8 from frame 8 with 213 queued at blend 8 from frame 8; then until the cast has played ([`cast_flight`]); a yield; its
//! target yaw toward its target. The child (`ex_2140_00.bd` `0x2EE`): effect 2052; waits while the parent plays 212; then
//! the parent must play 213 (else it ends); effects 2053 / 2054; 186 frames, each needing 213; then its target must still
//! exist (`sys 120`); then a thread (`0x2C`) makes the row `0xEC` on the parent (`sys 23`) and applies it to the target
//! at once (`sys 60` → `0x1402BF330` → `0x1402BF630`: a hit record on it, no collision, only if it can be hit,
//! `0x140292070`), and an effect (`sys 15(…, 7, 20000)`) follows the target's bone code 8. KH1's damage rule for
//! attribute 11 (`0x1402BFA80`: `0x2C0 − 0x281 < 0x80`) returns the row's `+0x14` (1000) plus its power (kind 0: the
//! caster's stats `+0x4C`), and the target's `0x1402BEEE0` adds it to its HP (`0x1402A4920` with `+n`): a full heal of
//! every KH1 Heartless. The heal here is Re:CoM's own Green Requiem's (owner rule: "each KH1 spell takes the effect /
//! power of the Re:CoM spell of the same kind"): the host's.
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0xAD0` (`0xD88`: 1500 / 750), the notice `0xA70` for a
//! room-placed one (spawn type 0: battle spawns play KH1's appearance, owner 2026-11-02), the search object's other users
//! (`sys 48`: Sora is the only target; `sys 139`'s out-of-sight test taken as in sight, as every port), the element rules
//! (attributes are not carried, owner), effects (`sys 75` / `sys 225`, the child's 2052..2054 and its effect object 7:
//! M13; Re:CoM's own heal effect plays), the appearance's flag 0x80, the death routine (Re:CoM's defeat, prizes, EXP).

use crate::caster::{fly_to, norm, step_casts, wander_leg, Cast, FlyTo, Wander};
use crate::enemy::script::{fly, frame, height, motion_done, poll, pos, push, rand, rand_range, random_offset, set_air_accel, set_state, set_target, steer, take_state, target, tick_clock};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Green Requiem's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// Its flight (`0x584` / `0x5CA`: `0x812(2, …)`), 246 frames.
    pub const HOVER: u16 = 2;
    /// The appearance (`0xBEA`: an action, 219 queued), 90 frames, root motion.
    pub const APPEAR: u16 = 69;
    /// The Cure's charge (`0x2CA`, from frame 8: its clip starts there), 128 frames; then its cast 213 (queued, from frame
    /// 8), 242 frames; the child acts 186 frames into 213.
    pub const CHARGE: u16 = 212;
    pub const CAST: u16 = 213;
    /// The notice (`0xA70`, a room-placed one only), 158 frames.
    pub const NOTICE: u16 = 214;
    /// The idle hover (`0x694`), 200 frames.
    pub const WANDER: u16 = 219;
    /// The idle after the notice (entry 0's motion).
    pub const REST: u16 = 220;
}

/// The Green Requiem has no attack of its own (its one row is its Cure's, applied to an ally): the host's attack list is
/// empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {}

impl Attack {
    pub const ALL: [Attack; 0] = [];

    pub fn index(self) -> u8 {
        match self {}
    }
}

/// Its spell child (`sys 94`'s sub-object index): the host's support spell ([`ECmd::Support`], [`ECmd::Throw`]).
pub mod child {
    /// `ex_2140_00.bd`: the Cure (`0x2CA`, row `0xEC`).
    pub const CURE: u8 = 0;
}

/// The names of the Green Requiem's numbers (besides `crate::enemy::ENGINE_NAMES` and [`CURE_NAMES`]).
pub const NAMES: &[&str] = &[
    "cure_chance",
    "move_chance",
    "cure_point_min",
    "cure_point_max",
    "pick_entry",
    "pick_range",
    "pick_ratio",
    "pick_weight_0",
    "pick_weight_1",
    "pick_weight_2",
    "pick_weight_3",
    "cure_child",
    "cast_blend",
    "cast_start",
    "cast_speed",
    "cast_stop",
    "cast_up",
    "cast_high",
    "cast_point_min",
    "cast_point_max",
    "rise_up",
    "floor_probe",
    "rise_speed",
    "rise_stop",
    "rise_aim",
    "rise_high",
    "rise_time",
    "move_min",
    "move_max",
    "move_speed_min",
    "move_speed_max",
    "move_stop",
    "move_up",
    "move_high",
    "move_time",
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

/// Its child's numbers (`ex_2140_00.bd`): the wait in the cast (186 frames) the behaviour runs ([`Cast`]).
pub const CURE_NAMES: &[&str] = &["cure_wait"];

/// Every name the manifest must hold for the behaviour.
pub fn all_names() -> Vec<&'static str> {
    NAMES.iter().chain(CURE_NAMES).copied().collect()
}

/// The Green Requiem's numbers.
pub type Consts = Table;

/// The state objects (`wp+0x20`), KH1's own numbers.
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 3;
}

/// One of the caster's allies as KH1's `sys 102` sees it (`0x1402B80A0`: the actors `0x140291B60` lists, alive (`+0x374`
/// bit 0 set, bit 1 clear), of its side, not itself; their stats `+0x3C` / `+0x40`, their distance `+0x10`): the host's
/// list (Re:CoM's battle enemies of its enemy type, in its manager's order; their Re:CoM HP, decision M10).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ally {
    /// Re:CoM's id of the enemy (`+0x20`, the handle `0x1402AD350` looks up).
    pub id: u32,
    pub hp: i32,
    pub max: i32,
    /// From the caster (KH1 units, 3D).
    pub dist: f32,
}

/// KH1's `sys 102` (`0x1402BC0B0` → `0x1402B80A0`, Checked (decomp)): every ally with max HP above 0 whose HP ratio is at
/// most `ratio` and that is within `range` is listed with the key `(int)(HP / max × 100)`; the list is sorted by the key,
/// lowest first (KH1's CRT `qsort` with `0x1402B8DB0`: `a − b`; [`crt_qsort`]); then one draw: `(int)(draw × 100) − w0`
/// under 0 takes the first; else under `w1` (or with fewer than three listed) the second when there are two or more
/// (else the first); else the third. None when nobody is listed (the caster then casts on itself: `0x350`).
pub fn pick(allies: &[Ally], draw: f32, range: f32, ratio: f32, w0: i32, w1: i32) -> Option<u32> {
    let mut list: Vec<(i32, u32)> = Vec::new();
    for a in allies {
        if a.max > 0 {
            let r = a.hp as f32 / a.max as f32;
            if r <= ratio && a.dist <= range {
                list.push(((r * 100.0) as i32, a.id));
            }
        }
    }
    crt_qsort(&mut list, |a, b| a.0 - b.0);
    let n = list.len();
    let mut out = list.first().map(|x| x.1);
    let i = (draw * 100.0) as i32 - w0;
    if i >= 0 {
        if i - w1 < 0 || n < 3 {
            if n > 1 {
                out = Some(list[1].1);
            }
        } else {
            out = Some(list[2].1);
        }
    }
    out
}

/// The CRT's `qsort` (KH1 imports it from `api-ms-win-crt-utility-l1-1-0.dll`, Checked (exe)), whose order of equal keys
/// decides which of several equally hurt allies [`pick`] takes: the UCRT's algorithm (Assumption: the published UCRT
/// source; the session has no `ucrtbase.dll` decomp): eight elements or fewer by its `shortsort` (the largest of the run
/// found from the front, `comp(p, max) > 0`, swapped to its end), more by its median-of-three partition.
pub fn crt_qsort<T: Copy>(v: &mut [T], comp: impl Fn(&T, &T) -> i32) {
    const CUTOFF: usize = 8;
    if v.len() < 2 {
        return;
    }
    let shortsort = |v: &mut [T], lo: usize, mut hi: usize| {
        while hi > lo {
            let mut max = lo;
            for p in lo + 1..=hi {
                if comp(&v[p], &v[max]) > 0 {
                    max = p;
                }
            }
            v.swap(max, hi);
            hi -= 1;
        }
    };
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let (mut lo, mut hi) = (0usize, v.len() - 1);
    loop {
        let size = hi - lo + 1;
        if size <= CUTOFF {
            shortsort(v, lo, hi);
        } else {
            let mut mid = lo + size / 2;
            if comp(&v[lo], &v[mid]) > 0 {
                v.swap(lo, mid);
            }
            if comp(&v[lo], &v[hi]) > 0 {
                v.swap(lo, hi);
            }
            if comp(&v[mid], &v[hi]) > 0 {
                v.swap(mid, hi);
            }
            let (mut loguy, mut higuy) = (lo as isize, hi as isize);
            let (lo_i, hi_i) = (lo as isize, hi as isize);
            loop {
                if (mid as isize) > loguy {
                    loop {
                        loguy += 1;
                        if !(loguy < mid as isize && comp(&v[loguy as usize], &v[mid]) <= 0) {
                            break;
                        }
                    }
                }
                if (mid as isize) <= loguy {
                    loop {
                        loguy += 1;
                        if !(loguy <= hi_i && comp(&v[loguy as usize], &v[mid]) <= 0) {
                            break;
                        }
                    }
                }
                loop {
                    higuy -= 1;
                    if !(higuy > mid as isize && comp(&v[higuy as usize], &v[mid]) > 0) {
                        break;
                    }
                }
                if higuy < loguy {
                    break;
                }
                v.swap(loguy as usize, higuy as usize);
                if mid as isize == higuy {
                    mid = loguy as usize;
                }
            }
            higuy += 1;
            if (mid as isize) < higuy {
                loop {
                    higuy -= 1;
                    if !(higuy > mid as isize && comp(&v[higuy as usize], &v[mid]) == 0) {
                        break;
                    }
                }
            }
            if (mid as isize) >= higuy {
                loop {
                    higuy -= 1;
                    if !(higuy > lo_i && comp(&v[higuy as usize], &v[mid]) == 0) {
                        break;
                    }
                }
            }
            if higuy - lo_i >= hi_i - loguy {
                if lo_i < higuy {
                    stack.push((lo, higuy as usize));
                }
                if loguy < hi_i {
                    lo = loguy as usize;
                    continue;
                }
            } else {
                if loguy < hi_i {
                    stack.push((loguy as usize, hi));
                }
                if lo_i < higuy {
                    hi = higuy as usize;
                    continue;
                }
            }
        }
        match stack.pop() {
            Some((l, h)) => {
                lo = l;
                hi = h;
            }
            None => return,
        }
    }
}

/// The Green Requiem's own work area: `wp+0x38` (seen: no notice), the appearance, the boot (`0x656`), its waiting spell
/// child, the world, what the handler does at once.
#[derive(Clone, Debug, Default)]
struct Wp {
    seen: bool,
    spawned: bool,
    appeared: bool,
    boot: bool,
    dying: bool,
    casts: Vec<Cast>,
    world: Option<u8>,
    now: Vec<ECmd<Attack>>,
}

type Cx = crate::enemy::script::Cx<Consts, Attack, Wp>;
type Task = crate::enemy::script::Task;
pub type Cmd = ECmd<Attack>;

fn k(cx: &Cx, n: &str) -> f32 {
    cx.borrow().c.num(n)
}

/// The actor's target (`+0x74`): Sora (Re:CoM's battles have no other party member).
fn tgt(cx: &Cx) -> Vec3 {
    let io = cx.borrow();
    target(cx).or(io.w.target).unwrap_or(io.w.pos)
}

/// `0x18F8` (`sys 36`): the target yaw toward the target (the engine turns).
fn yaw_tgt(cx: &Cx) {
    let d = tgt(cx) - pos(cx);
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `0x1922(actor, target)` (`sys 11`): facing the target at once.
fn face_now(cx: &Cx) {
    let d = tgt(cx) - pos(cx);
    let y = d.x.atan2(d.z);
    let mut io = cx.borrow_mut();
    io.body.yaw = y;
    io.body.yaw_target = y;
    io.out.push(ECmd::Yaw(y));
}

/// `sys 13` (`0x1402B9F80` → `0x14029F980`): a request written whatever plays (the playing one starts again).
fn request(cx: &Cx, r: u16, blend: f32) {
    let again = {
        let mut io = cx.borrow_mut();
        let again = io.w.motion == Some(r);
        io.clock = (Some(r), 0.0);
        again
    };
    push(cx, if again { ECmd::Restart { request: r, blend, start: 0.0 } } else { ECmd::Motion { request: r, blend } });
}

/// `sys 172` with the cast queued (`0x14029EFC0`: played to its end with nothing queued): the queued request plays and
/// has played to its end.
fn done_after(cx: &Cx, last: u16) -> bool {
    cx.borrow().w.motion == Some(last) && motion_done(cx)
}

/// `0x12E8`: until the motion asked for has played to its end (`sys 172`) or request 6 plays (`0x131C`), turning to the
/// target each frame (`0x18F8`).
async fn wait_motion(cx: &Cx, last: u16) {
    loop {
        if cx.borrow().w.motion == Some(last) && motion_done(cx) {
            break;
        }
        if cx.borrow().w.motion == Some(k(cx, "wait_motion") as u16) {
            break;
        }
        yaw_tgt(cx);
        frame().await;
    }
}

/// The floor's height under a point (`0x13C6`: `sys 77` straight down from it; `0x11BC` probes from `floor_probe` above
/// the target): the battle floor here (Re:CoM's battle floors are flat, Assumption as every flier's), KH1 y.
fn floor_under(cx: &Cx, p: Vec3) -> f32 {
    cx.borrow().floor.unwrap_or(p.y)
}

/// `0x2CA`: the Cure: a point KH1 never reads (`0x1566`: its two draws); the pick (`sys 102`, the host's
/// [`ECmd::Support`]); its child (`sys 94`: [`Cast`], waiting through 212 and 186 frames of 213); 212 (its clip from frame
/// 8) with 213 queued; the cast's flight until the cast has played; a yield; the target yaw toward the target.
async fn cure(cx: &Cx) {
    let _ = random_offset(cx, k(cx, "cure_point_min"), k(cx, "cure_point_max"));
    let draw = rand(cx, "sys 102 pick");
    let child = k(cx, "cure_child") as u8;
    push(cx, ECmd::Support { child, draw });
    let w = k(cx, "cure_wait");
    cx.borrow_mut().x.casts.push(Cast::new(child, req::CHARGE, req::CAST, w));
    push(cx, ECmd::Note("sys 94: the Cure waits on the cast (its effects 2052 / 2053 / 2054: M13)"));
    request(cx, req::CHARGE, k(cx, "cast_blend"));
    push(cx, ECmd::Queue { request: req::CAST, blend: k(cx, "cast_blend") });
    cast_flight(cx).await;
    frame().await;
    yaw_tgt(cx);
}

/// `0x3EA(999, target, 6, 10, 180, 180)`: until the cast has played (`sys 172`, checked first): toward 180 above the floor
/// under its point (first the target's live place), from where it is; within 10 a new point [600, 800) from the target at
/// its own height (this frame still steers along the short way); while the aim is below it: no descent when it is below
/// the point itself, else (higher than 180 above the floor) the way scaled by the speed (no effect once normalised; kept);
/// facing the target at once (`0x1922`); steering at 6.
async fn cast_flight(cx: &Cx) {
    let (speed, stop, up, high) = (k(cx, "cast_speed"), k(cx, "cast_stop"), k(cx, "cast_up"), k(cx, "cast_high"));
    let mut point: Option<Vec3> = None;
    loop {
        if done_after(cx, req::CAST) {
            break;
        }
        let p = point.unwrap_or_else(|| tgt(cx));
        let me = pos(cx);
        let mut v = Vec3::new(p.x, floor_under(cx, p) - up, p.z) - me;
        if v.length() - stop < 0.0 {
            let mut q = tgt(cx) + random_offset(cx, k(cx, "cast_point_min"), k(cx, "cast_point_max"));
            q.y = me.y;
            point = Some(q);
        } else if v.y > 0.0 {
            if me.y - p.y > 0.0 {
                v.y = 0.0;
            } else if height(cx) - high > 0.0 {
                v = v * speed;
            }
        }
        let v = norm(v);
        face_now(cx);
        steer(cx, v, speed);
        frame().await;
    }
}

/// `0x812`'s numbers with the routine's own (the blend, the stuck factor, "no motion").
fn fly_numbers(cx: &Cx, speed: f32, stop: f32, up: f32, high: f32, time: f32) -> FlyTo {
    FlyTo {
        request: req::HOVER,
        speed,
        stop,
        up,
        high,
        time,
        blend: k(cx, "fly_blend"),
        stuck: k(cx, "fly_stuck"),
        stuck_half: k(cx, "stuck_half"),
        no_motion: k(cx, "fly_no_motion") as u16,
    }
}

/// `0x584`: to 180 above the floor under the target (`0x11BC`, the point read once), `0x812(2, point, 6, 100, 0, 180,
/// 300)`.
async fn to_height(cx: &Cx) {
    let t = tgt(cx);
    let p = Vec3::new(t.x, floor_under(cx, t) - k(cx, "rise_up"), t.z);
    let n = fly_numbers(cx, k(cx, "rise_speed"), k(cx, "rise_stop"), k(cx, "rise_aim"), k(cx, "rise_high"), k(cx, "rise_time"));
    fly_to(cx, n, move |_| p).await;
}

/// `0x5CA`: the move: a point [500, 600) from itself on the floor under it, `0x812(2, point, [6, 7.2), 10, 180, 180,
/// 240)`.
async fn move_about(cx: &Cx) {
    let mut p = pos(cx) + random_offset(cx, k(cx, "move_min"), k(cx, "move_max"));
    p.y = floor_under(cx, p);
    let speed = rand_range(cx, k(cx, "move_speed_min"), k(cx, "move_speed_max"), "move speed");
    let n = fly_numbers(cx, speed, k(cx, "move_stop"), k(cx, "move_up"), k(cx, "move_high"), k(cx, "move_time"));
    fly_to(cx, n, move |_| p).await;
}

/// `0x280`: one round: 80 % a Cure, then 30 % a move, then a Cure and a move.
async fn round(cx: &Cx) {
    if rand(cx, "cure") - k(cx, "cure_chance") < 0.0 {
        cure(cx).await;
    }
    if rand(cx, "move") - k(cx, "move_chance") < 0.0 {
        move_about(cx).await;
    }
    cure(cx).await;
    move_about(cx).await;
}

/// `0x694`'s numbers.
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

/// The idle behaviour (`0x64A`): idle legs, a yield between them.
async fn idle(cx: Cx) {
    loop {
        let n = wander(&cx);
        wander_leg(&cx, n).await;
        frame().await;
    }
}

/// `0xA70`: the notice (a room-placed one, spawn type 0, outside KH1's world 11): turning to the target, 214 (blend 8)
/// until it has played, then the idle 220 (blend 0); then the leash (not ported).
async fn notice(cx: &Cx) {
    if cx.borrow().x.world == Some(k(cx, "notice_world") as u8) {
        return;
    }
    yaw_tgt(cx);
    request(cx, req::NOTICE, k(cx, "notice_blend"));
    wait_motion(cx, req::NOTICE).await;
    request(cx, req::REST, 0.0);
}

/// The engaged behaviour (`0x21E` → `0x25A`): seen (the notice once for a placed one); a yield; to its height; one round;
/// to its height; then engaged again (the search and the target, `0x9B0`: Sora).
async fn engaged(cx: Cx) {
    if !cx.borrow().x.seen {
        cx.borrow_mut().x.seen = true;
        if !cx.borrow().x.spawned {
            notice(&cx).await;
        }
    }
    frame().await;
    to_height(&cx).await;
    round(&cx).await;
    to_height(&cx).await;
    set_state(&cx, st::ENGAGED);
}

/// The hit behaviour (`0x1DC`): a Cure; to its height; then engaged.
async fn hit(cx: Cx) {
    cure(&cx).await;
    to_height(&cx).await;
    set_state(&cx, st::ENGAGED);
}

/// The idle start (`0x666`): a spawned one's appearance (`0xBEA`: the 30-frame fade-in, 69 as an action with the idle
/// hover 219 queued, then facing Sora at once); then idle.
async fn idle_start(cx: Cx) {
    if cx.borrow().x.spawned {
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        push(&cx, ECmd::Note("effect 2, flag 0x80 (M13)"));
        push(&cx, ECmd::Action { request: req::APPEAR, attack: None, then: req::WANDER, blend: k(&cx, "appear_blend") });
        frame().await;
        // `0xC62`: `0x1922(actor, 0x142E())`: Sora (party 0) at once, whether or not he is its target yet.
        face_now(&cx);
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The supervisor (entry `0x1BBE`): the init's yield, the idle start (`0x656`); then each frame the idle search
/// (`0x7A8`) and the state requests.
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

/// The Green Requiem's AI for one enemy.
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
            // `0x9EA`: the flight's easing 0.2, flight with no velocity.
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
    /// KH1's children run while the script waits (a reaction plays): the spell child's wait.
    fn side_frame(&mut self, w: World) -> Vec<ECmd<u8>> {
        {
            let mut io = self.io.borrow_mut();
            io.w = w;
        }
        self.children();
        std::mem::take(&mut self.io.borrow_mut().out).iter().map(|c| c.map(Attack::index)).collect()
    }
    /// After its death, while Re:CoM keeps it: its child goes on by its own rule (`Cast`: the cast motion still playing;
    /// the child tests its target, never its parent's HP), so the killing hit's reaction ends a waiting Cure.
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
    /// The hit handler (`0x1CAA`) without the element and attribute rules (attributes are not carried): seen; a living
    /// damaging hit or a clash asks for the hit state; a repel record without a clash asks for nothing.
    fn on_hit(&self, news: HitNews) {
        let mut io = self.io.borrow_mut();
        io.x.seen = true;
        match news {
            HitNews::Damage | HitNews::Clash => io.state = (st::HIT, true),
            HitNews::Repelled => {}
        }
    }
    fn take_now(&self) -> Vec<ECmd<u8>> {
        std::mem::take(&mut self.io.borrow_mut().x.now).iter().map(|c| c.map(Attack::index)).collect()
    }
    /// The killing hit: its death follows (`0x126E`: Re:CoM's defeat here).
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
