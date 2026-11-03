//! KH1 FM's Blue Rhapsody (`xa_ex_2120.mdls` part 5, `ex_2120.bd`), engine-agnostic: its AI script on the shared enemy
//! framework (`crate::enemy`), flying (KH1's movement state 4, `crate::enemy::Body::flying`; its hops movement state 5,
//! `crate::enemy::Body::flight_hop`), with the casting family's shared routines (`crate::caster`) and its spell child's
//! wait ([`Cast::direct`]: `ex_2120_00.bd` the Blizzard; part 3). M11 (`docs/recom-m11-enemies.md` § The Blue Rhapsody).
//! Re:CoM runs it natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), its melee in
//! Re:CoM's own melee state (3, its `.ASD` records), its Blizzard one of Re:CoM's own bullets moved by KH1's numbers
//! (`win::kh1_thrown`).
//!
//! Every number is data ([`NAMES`], [`BLIZZARD_NAMES`], read by the asset step from the player's `ex_2120.bd` and its
//! child at checked offsets). Script offsets are the player's own (Checked (files): the scripts disassembled with
//! `tools/kh1-ai-script.py`, the parent's code from `0x298`, the child's from `0x18`; the syscalls named are Checked
//! (decomp)).
//!
//! # The script's shape
//!
//! - entry `0x1E1A`: init `0x1E9A` → `0xBB4` (records LV1 `0x14` / LV2 `0x80`; the stats block's `+0x14` = 0.2 (the
//!   flight's easing), `+0x18` = 0.95; flight with no velocity (`sys 135`); the state and search objects; home; seen
//!   cleared); the handlers (hit `0x1EFA`, message `0x1FD2`: answers 0, the hit-effect filter `0x1EDA`); the stats block's
//!   `+0x10` = 500 (the hop height of `sys 50`, which it never calls: unused); a yield; the idle start `0x820` → `0x830`;
//!   then each frame the idle search (`0x972`: in state 1, Sora within 1000 → engaged) and a state request restarts the
//!   behaviour (1 idle `0x7FC` → `0x814`, the target cleared; 2 engaged `0x2F0` → `0x314`; 3 hit `0x298` → `0x2BC`; both
//!   with the target). No `sys 125`: KH1's π/16.
//! - Every motion it plays is a plain request (no action bit) except its appearance (69): the script never pauses for
//!   its own motions, only for KH1's reactions. Its melee 215 makes two rows on itself (`sys 23`: `0xEC` the hit, `0x15C` a
//!   type-2 body row that only clashes) with one hit serial: a steered attack ([`ECmd::AttackAt`]); its Blizzard's row
//!   `0x1CC` is its child's.
//! - the hit handler `0x1EFA`: seen; a Blizzard hit (attribute 3, `0xCE8`): no damage, HP to max, no reaction (element:
//!   left out); the damage (`0x1774`: a clash or a repel −1; killed 0 → its death `0x1438`); a Fire hit (attribute 2) or
//!   attribute 8: KH1's EXP bonus `EXP / 10 + 2` (`0xD52`, element: left out); −1: the hit state only for a clash (or
//!   attributes 8 / 10, `0x13D6`); alive: a Fire hit's dizzy (`sys 179(1800)`, element: left out) and the hit state. KH1's
//!   reaction plays for every hit (the handler zeroes the kind only for a Blizzard hit).
//!
//! Not ported (Re:CoM's battles only, decision M10): the leash `0xF52` (`0xC9A`: 1500 / 750), the notice `0xC3A` for a
//! room-placed one (spawn type 0: battle spawns play KH1's appearance, owner 2026-11-02), the search object (`sys 48`:
//! Sora is the only target; `sys 139`'s out-of-sight test taken as in sight, as every port), the element rules
//! (attributes are not carried, owner), effects (`sys 75` / `sys 225`, the child's 2054 / 2055, M13), the appearance's
//! flag 0x80, the death routine (Re:CoM's defeat, prizes, EXP).

use crate::caster::{fly_to, play, rotdeg, step_casts, wander_leg, Cast, FlyTo, Wander};
use crate::enemy::script::{fly, frame, height, motion_done, motion_frames, poll, pos, push, rand, set_air_accel, set_state, set_target, take_state, target, tick_clock, wait_frames};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;

/// KH1's motion requests the Blue Rhapsody's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// The hop's crouch (`0x4DE` / `0x18E2`: `sys 13(3)`, its length waited), 16 frames, root motion; then 4 (bit 16) as
    /// it leaves, 8 frames; six frames before the top KH1 plays 5 and queues 6 (`sys 83(3)`: base 0, `0x1402AF630`).
    pub const CROUCH: u16 = 3;
    pub const RISE: u16 = 4;
    pub const APEX: u16 = 5;
    pub const DROP: u16 = 6;
    /// The Blizzard's cast (`0x424`), 148 frames; its child acts 84 frames in.
    pub const CAST: u16 = 55;
    /// The appearance (`0xDB4`: an action, 219 queued), 90 frames, root motion.
    pub const APPEAR: u16 = 69;
    /// The notice (`0xC3A`, a room-placed one only), 158 frames.
    pub const NOTICE: u16 = 214;
    /// The melee (`0x47E`, from frame 8), 148 frames; its clash recoil 216 (row `+0x40`), 52 frames.
    pub const MELEE: u16 = 215;
    pub const MELEE_RECOIL: u16 = 216;
    /// The idle hover (`0x85E`), 200 frames.
    pub const WANDER: u16 = 219;
    /// The idle the script asks for after a cast or the melee and while it flies to its height (entry 0's motion).
    pub const REST: u16 = 220;
}

/// The Blue Rhapsody's attack (the host's attack list, `kh1-recom`'s class row).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    /// 215 with rows `0xEC` / `0x15C`.
    Melee,
}

impl Attack {
    pub const ALL: [Attack; 1] = [Attack::Melee];

    pub fn index(self) -> u8 {
        match self {
            Attack::Melee => 0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Melee => "melee",
        }
    }
}

/// Its spell child (`sys 94`'s sub-object index): the host's thrown object ([`ECmd::Throw`]).
pub mod child {
    /// `ex_2120_00.bd`: the Blizzard (`0x424`, row `0x1CC`).
    pub const BLIZZARD: u8 = 0;
}

/// The names of the Blue Rhapsody's numbers (besides `crate::enemy::ENGINE_NAMES` and [`BLIZZARD_NAMES`]).
pub const NAMES: &[&str] = &[
    "close_range",
    "mid_range",
    "hit_close_range",
    "hop_side_chance",
    "double_hop_chance",
    "hop_turn",
    "flip",
    "hop_speed",
    "hop_height",
    "hop_drag",
    "hop_time",
    "back_turn",
    "back_speed",
    "back_height",
    "back_drag",
    "back_time",
    "approach_count",
    "approach_speed",
    "approach_height",
    "approach_drag",
    "approach_time",
    "climb_speed",
    "climb_height",
    "climb_drag",
    "climb_time",
    "approach_stop",
    "rise_up",
    "floor_probe",
    "rise_speed",
    "rise_stop",
    "rise_aim",
    "rise_high",
    "rise_time",
    "wander_turn",
    "wander_sink",
    "wander_low",
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
    "cast_blend",
    "cast_start",
    "melee_blend",
    "melee_start",
    "rest_blend",
    "rest_start",
    "exp_div",
    "exp_add",
    "dizzy_time",
    "blizzard_child",
];

/// Its child's numbers (`ex_2120_00.bd`): the wait in the cast (84 frames) the behaviour runs ([`Cast::direct`]); the
/// start t, flight speed and life the host's.
pub const BLIZZARD_NAMES: &[&str] = &["blizzard_wait", "blizzard_t", "blizzard_life", "blizzard_speed"];

/// Every name the manifest must hold for the behaviour.
pub fn all_names() -> Vec<&'static str> {
    NAMES.iter().chain(BLIZZARD_NAMES).copied().collect()
}

/// The Blue Rhapsody's numbers.
pub type Consts = Table;

/// The state objects (`wp+0x20`), KH1's own numbers.
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    pub const HIT: u8 = 3;
}

/// The Blue Rhapsody's own work area: `wp+0` (the hops' side: 1 turns them −20°), `wp+0x38` (seen: no notice), the
/// appearance, the boot (`0x820`), its waiting spell child, the world, what the handler does at once.
#[derive(Clone, Debug, Default)]
struct Wp {
    side: bool,
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

fn step(cx: &Cx) -> f32 {
    cx.borrow().w.step
}

/// The actor's target (`+0x74`): Sora (Re:CoM's battles have no other party member).
fn tgt(cx: &Cx) -> Vec3 {
    let io = cx.borrow();
    target(cx).or(io.w.target).unwrap_or(io.w.pos)
}

/// `0x1518` (`0x18A2(actor, target)`): the distance to the target (3D).
fn dist(cx: &Cx) -> f32 {
    (tgt(cx) - pos(cx)).length()
}

/// The facing vector `+0x30`.
fn facing(cx: &Cx) -> Vec3 {
    Vec3::from_yaw(cx.borrow().body.yaw)
}

/// `0x1B54` (`sys 36`): the target yaw toward the target (the engine turns).
fn yaw_tgt(cx: &Cx) {
    let d = tgt(cx) - pos(cx);
    cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
}

/// `0x1B7E(actor, target)` (`sys 11`): facing the target at once.
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

/// `0x14B2`: until the motion asked for has played to its end (`sys 172`) or request 6 plays (`0x14E6`), turning to the
/// target each frame (`0x1B54`).
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

/// The floor's height under the target (`0x1386`: `sys 77` straight down from `floor_probe` above it; the battle floor
/// here: Re:CoM's battle floors are flat, Assumption as every flier's), KH1 y.
fn floor_under_target(cx: &Cx) -> f32 {
    let f = cx.borrow().floor;
    f.unwrap_or_else(|| tgt(cx).y)
}

/// `sys 92` (`0x1402BA630` → `0x140299390`): the floor probe ahead (2 × its radius along `v`, down from its middle); 1
/// when the floor there is higher than its middle (a step up), which the approach climbs (Checked (decomp)). Re:CoM's
/// battle floors are flat (Assumption, as every flier's): never.
fn step_up_ahead(_cx: &Cx, _v: Vec3) -> bool {
    false
}

/// The hop itself: the crouch 3 (`sys 13`, blend 0) for its length (`0x1FEA`), 4 with bit 16 (a plain request here,
/// Assumption as the Powerwild's), `sys 82(v, drag, time)` (from flight: KH1's movement state 5, [`Body::flight_hop`]
/// with its height above the floor), `sys 83(3)` (six frames before the top 5 plays and 6 is queued: `0x1402AF630`, run in
/// the engine's step after the script, here at the next frame's script: Assumption ±1 frame, as the Powerwild's); then
/// until it is back in flight (`+0x70` 3 or 5), facing the target at once each frame when `face` (`0x4DE`; `0x18E2`
/// doesn't).
async fn hop_with(cx: &Cx, v: Vec3, drag: f32, time: f32, face: bool) {
    request(cx, req::CROUCH, 0.0, 0.0);
    let n = motion_frames(cx, req::CROUCH);
    wait_frames(cx, n).await;
    request(cx, req::RISE, 0.0, 0.0);
    let h = height(cx);
    cx.borrow_mut().body.timed_hop(v, drag, time, h);
    let mut apex = false;
    while cx.borrow().body.timed.is_some() {
        let (at_top, cur) = {
            let io = cx.borrow();
            (io.body.timed_apex(), io.w.motion)
        };
        if !apex && at_top && cur == Some(req::RISE) {
            apex = true;
            play(cx, req::APEX, 0.0);
            push(cx, ECmd::Queue { request: req::DROP, blend: 0.0 });
        }
        if face {
            face_now(cx);
        }
        frame().await;
    }
}

/// `0x4DE`: a hop: facing the target at once, its facing turned 20° (−20° when `wp+0` is 1), × 30, 60 high, drag 0.9 over
/// 12 frames, facing the target while in the air.
async fn hop(cx: &Cx) {
    face_now(cx);
    let mut deg = k(cx, "hop_turn");
    if cx.borrow().x.side {
        deg *= k(cx, "flip");
    }
    let d = rotdeg(facing(cx), deg) * k(cx, "hop_speed");
    let v = Vec3::new(d.x, k(cx, "hop_height"), d.z);
    hop_with(cx, v, k(cx, "hop_drag"), k(cx, "hop_time"), true).await;
}

/// `0x600`: the back-off: facing the target at once, its facing turned 180°, × 35, 250 high, drag 0.9 over 40 frames
/// (`0x18E2`); then to its height.
async fn back_off(cx: &Cx) {
    face_now(cx);
    let d = rotdeg(facing(cx), k(cx, "back_turn")) * k(cx, "back_speed");
    let v = Vec3::new(d.x, k(cx, "back_height"), d.z);
    hop_with(cx, v, k(cx, "back_drag"), k(cx, "back_time"), false).await;
    to_height(cx).await;
}

/// `0x78E`: to 35 above the floor under the target, straight up or down from where it is (`0x9DC(220, point, 6, 20, 0,
/// 500, 300)`, the point read once).
async fn to_height(cx: &Cx) {
    let me = pos(cx);
    let p = Vec3::new(me.x, floor_under_target(cx) - k(cx, "rise_up"), me.z);
    let n = FlyTo {
        request: req::REST,
        speed: k(cx, "rise_speed"),
        stop: k(cx, "rise_stop"),
        up: k(cx, "rise_aim"),
        high: k(cx, "rise_high"),
        time: k(cx, "rise_time"),
        blend: k(cx, "fly_blend"),
        stuck: k(cx, "fly_stuck"),
        stuck_half: k(cx, "stuck_half"),
        no_motion: k(cx, "fly_no_motion") as u16,
    };
    fly_to(cx, n, move |_| p).await;
}

/// `0x666`: the approach: to its height; then up to 121 rounds: facing the target at once, a hop 20 ahead, 120 high, 0.9
/// over 25 frames (`0x18E2`); a step up ahead (`sys 92` = 1): a climb (5 ahead, 500 high, 1.0 over 60 frames) and the next
/// round at once; else done within 750, else the next round after a yield (the count down by the step); then to its
/// height.
async fn approach(cx: &Cx) {
    to_height(cx).await;
    let mut t = k(cx, "approach_count");
    while t >= 0.0 {
        face_now(cx);
        let d = facing(cx) * k(cx, "approach_speed");
        let v = Vec3::new(d.x, k(cx, "approach_height"), d.z);
        hop_with(cx, v, k(cx, "approach_drag"), k(cx, "approach_time"), false).await;
        if step_up_ahead(cx, v) {
            let d = crate::caster::norm(v) * k(cx, "climb_speed");
            let c = Vec3::new(d.x, k(cx, "climb_height"), d.z);
            hop_with(cx, c, k(cx, "climb_drag"), k(cx, "climb_time"), false).await;
            continue;
        }
        if dist(cx) - k(cx, "approach_stop") <= 0.0 {
            break;
        }
        t -= step(cx);
        frame().await;
    }
    to_height(cx).await;
}

/// `0x424`: the Blizzard: its child (`sys 94(actor, 0, target, 0x1CC, 0)`), 55 (blend 0, from frame 0), until it has
/// played (or request 6 plays) turning to the target; then the idle 220 (blend 0, from frame 2) and to its height.
async fn blizzard(cx: &Cx) {
    let w = k(cx, "blizzard_wait");
    cx.borrow_mut().x.casts.push(Cast::direct(child::BLIZZARD, req::CAST, w));
    push(cx, ECmd::Note("sys 94: the Blizzard waits on the cast (its effects 2054 / 2055: M13)"));
    request(cx, req::CAST, k(cx, "cast_blend"), k(cx, "cast_start"));
    wait_motion(cx, req::CAST).await;
    request(cx, req::REST, k(cx, "rest_blend"), k(cx, "rest_start"));
    to_height(cx).await;
}

/// `0x47E`: the melee: its two rows with one hit serial and 215 (blend 8, from frame 8): a steered attack; until it has
/// played (or request 6 plays) turning to the target; then the idle 220 (blend 0, from frame 2) and to its height.
async fn melee(cx: &Cx) {
    let start = k(cx, "melee_start");
    cx.borrow_mut().clock = (Some(req::MELEE), start);
    push(cx, ECmd::AttackAt { request: req::MELEE, attack: Attack::Melee, blend: k(cx, "melee_blend"), start });
    wait_motion(cx, req::MELEE).await;
    request(cx, req::REST, k(cx, "rest_blend"), k(cx, "rest_start"));
    to_height(cx).await;
}

/// `0x3BC`: the hops and melee: the side (`wp+0` = 0 under 0.5, else 1), 60 % two hops (else one), the melee.
async fn hops_and_melee(cx: &Cx) {
    let side = rand(cx, "hop side") - k(cx, "hop_side_chance") >= 0.0;
    cx.borrow_mut().x.side = side;
    if rand(cx, "double hop") - k(cx, "double_hop_chance") < 0.0 {
        hop(cx).await;
    }
    hop(cx).await;
    melee(cx).await;
}

/// `0x85E`'s numbers.
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

/// The idle behaviour (`0x814`): idle legs, a yield between them.
async fn idle(cx: Cx) {
    loop {
        let n = wander(&cx);
        wander_leg(&cx, n).await;
        frame().await;
    }
}

/// `0xC3A`: the notice (a room-placed one, spawn type 0, outside KH1's world 11): turning to the target, 214 (blend 8)
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

/// The engaged behaviour (`0x314`): seen (the notice once for a placed one); a yield; to its height; one round by the
/// distance: under 250 the back-off and Blizzard, under 500 the hops and melee, farther the approach; to its height; then
/// engaged again (the search and the target, `0xB7A`: Sora).
async fn engaged(cx: Cx) {
    if !cx.borrow().x.seen {
        cx.borrow_mut().x.seen = true;
        if !cx.borrow().x.spawned {
            notice(&cx).await;
        }
    }
    frame().await;
    to_height(&cx).await;
    let d = dist(&cx);
    if d - k(&cx, "close_range") < 0.0 {
        back_off(&cx).await;
        blizzard(&cx).await;
    } else if d - k(&cx, "mid_range") < 0.0 {
        hops_and_melee(&cx).await;
    } else {
        approach(&cx).await;
    }
    to_height(&cx).await;
    set_state(&cx, st::ENGAGED);
}

/// The hit behaviour (`0x2BC`): within 500, the back-off and the Blizzard; then engaged.
async fn hit(cx: Cx) {
    if dist(&cx) - k(&cx, "hit_close_range") < 0.0 {
        back_off(&cx).await;
        blizzard(&cx).await;
    }
    set_state(&cx, st::ENGAGED);
}

/// The idle start (`0x830`): a spawned one's appearance (`0xDB4`: the 30-frame fade-in, 69 as an action with the idle
/// hover 219 queued, then facing Sora at once); then idle.
async fn idle_start(cx: Cx) {
    if cx.borrow().x.spawned {
        push(&cx, ECmd::FadeIn(k(&cx, "appear_fade")));
        push(&cx, ECmd::Note("effect 2, flag 0x80 (M13)"));
        push(&cx, ECmd::Action { request: req::APPEAR, attack: None, then: req::WANDER, blend: k(&cx, "appear_blend") });
        frame().await;
        // `0xE2C`: `0x1B7E(actor, 0x1612())`: Sora (party 0) at once, whether or not he is its target yet.
        face_now(&cx);
    }
    cx.borrow_mut().x.appeared = true;
    set_state(&cx, st::IDLE);
}

/// The supervisor (entry `0x1E1A`): the init's yield, the idle start (`0x820`); then each frame the idle search
/// (`0x972`) and the state requests.
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

/// The Blue Rhapsody's AI for one enemy.
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
            // `0xBB4`: the flight's easing 0.2, flight with no velocity.
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
    /// The hit handler (`0x1EFA`) without the element rules (attributes are not carried): seen; a living damaging hit or
    /// a clash asks for the hit state; a repel record without a clash asks for nothing.
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
    /// The killing hit: its death follows (`0x1438`: Re:CoM's defeat here).
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
