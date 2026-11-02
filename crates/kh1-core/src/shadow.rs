//! KH1 FM's Shadow (`xa_ex_2020.mdls` part 5, `ex_2020.bd`), engine-agnostic: its AI, its
//! attacks, its body physics (walk, jump arc, air steering, push slide, turning) and its hit
//! reactions (flinch, knock-back, launch, air reactions, landing, lying down, get-up). M10
//! (`docs/recom-m10-shadow.md`). Re:CoM runs it natively: each part is the step of a Re:CoM enemy
//! state (`kh1-recom`'s `win::shadow`), Re:CoM plays the motions, collides the body and the
//! attacks, applies the damage.
//!
//! Every number is plain data ([`Consts`]): the asset step reads them from the player's own
//! `ex_2020.bd` (and `btltbl.bin`, `xa_ex_2020.mset`); nothing of the game is in this file. The
//! rules cite the KH1 Steam addresses and script offsets they were read from (Checked
//! (decomp + files) unless marked Assumption).
//!
//! # The AI's shape (Checked (files + decomp): the script, the VM `0x1402CC620`, the threads
//! `0x1402BCEC0` / `0x1402BD640`, the gate `0x140297870`)
//!
//! KH1 runs the script once per frame from the enemy update `0x1402B7260`, only while no
//! "action" motion plays (actor flag 4, set by a request with bit 24): an attack or a reaction
//! pauses the script, which then carries on after the instruction that yielded. Two threads:
//! the supervisor (searches the target, restarts the behaviour thread when the state object
//! `wp+0x20` changes) and one behaviour thread per state (0 idle, 1 engaged, 2 just hit). The
//! port keeps that shape with Rust `async` code polled once per frame ([`Ai::frame`]); the host
//! doesn't poll while its action (attack state, reaction) runs, which is KH1's pause.

use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::rc::Rc;
use std::cell::RefCell;

/// KH1's motion requests the Shadow uses (its `.mset` request map; Checked (files)).
pub mod req {
    pub const IDLE: u16 = 0;
    pub const WALK: u16 = 1;
    pub const RUN: u16 = 2;
    pub const JUMP: u16 = 3;
    pub const FALL: u16 = 4;
    /// Played after the leap claw (`0x40000006`), and the air flinch's queue.
    pub const AFTER_LEAP: u16 = 6;
    /// The landing motion of a plain fall (`param_1[0xf4] + 7`, `0x1402B7260`; base 0).
    pub const LAND: u16 = 7;
    /// Ground flinch: 0x48 / 0x49 hit from the front (part 0 / other), 0x4A / 0x4B from behind.
    pub const FLINCH: u16 = 0x48;
    /// Ground knock-back (kind 4): 0x4C front, 0x4D behind (root motion 400).
    pub const KNOCK: u16 = 0x4C;
    /// Air flinch: 0x4E front, 0x4F behind.
    pub const AIR_FLINCH: u16 = 0x4E;
    /// Air knock-back (kind 4 in the air), when the model has it (the Shadow hasn't).
    pub const AIR_KNOCK: u16 = 0x50;
    /// Launch on the ground: base, +1 air, +2 land, +3 lie, +4 get up; +5..+9 from behind.
    pub const LAUNCH: u16 = 0x52;
    /// Launch in the air (same layout).
    pub const AIR_LAUNCH: u16 = 0x5C;
    pub const RISE: u16 = 69;
    pub const SINK: u16 = 70;
    pub const CLAW: u16 = 200;
    pub const LEAP_CLAW: u16 = 201;
    pub const UNDERGROUND: u16 = 202;
    pub const TAUNT: u16 = 203;
    pub const SWIPE: u16 = 204;
}

/// The three attacks (the script's four 0x70-byte attack rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    /// Request 200, row `+0x884` (right hand). Re:CoM attack id 0 (its claw).
    Claw,
    /// Request 204, row `+0x8F4` (left hand, a line 10 → 0.1 along the bone). Attack id 0.
    Swipe,
    /// Request 201, rows `+0x964` / `+0x9D4` (both hands). Re:CoM attack id 1 (its leap).
    LeapClaw,
}

impl Attack {
    pub const ALL: [Attack; 3] = [Attack::Claw, Attack::Swipe, Attack::LeapClaw];

    pub fn request(self) -> u16 {
        match self {
            Attack::Claw => req::CLAW,
            Attack::Swipe => req::SWIPE,
            Attack::LeapClaw => req::LEAP_CLAW,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Claw => "claw",
            Attack::Swipe => "swipe",
            Attack::LeapClaw => "leap claw",
        }
    }
}

/// Every number the Shadow's code reads, by name. The asset step writes them (from the script's
/// push instructions at checked offsets, from `btltbl.bin`, from the Shadow's stat record and its
/// `.mset`); `kh1-recom` reads them from `kh1-shadow.json`. [`Consts::names`] lists them.
#[derive(Clone, Debug, PartialEq)]
pub struct Consts {
    values: Vec<(String, f32)>,
}

macro_rules! consts {
    ($($name:ident),* $(,)?) => {
        /// The names every [`Consts`] must hold.
        pub const NAMES: &[&str] = &[$(stringify!($name)),*];
        impl Consts {
            $(pub fn $name(&self) -> f32 { self.get(stringify!($name)) })*
        }
    };
}

consts! {
    // Distances (KH1 units), chances, timers (60 Hz frames) of the script.
    close_range, mid_range, swipe_range, claw_range, claw_chance, close_taunt_chance,
    after_taunt_chance, after_dig_chance, far_fraction, far_approach_chance, idle_search,
    engaged_search, taunt_blend, taunt_wait_min, taunt_wait_max, approach_timeout,
    approach_retarget, idle_dig_a_min, idle_dig_a_max, idle_dig_b_min, idle_dig_b_max,
    wander_inner, wander_outer, wander_band, wander_timeout, wander_idle_chance,
    wander_idle_min, wander_idle_max, wander_home_max, near_dig_min, near_dig_max, near_dig_stop,
    near_dig_target, travel_timeout, stuck_factor, weave_amplitude, weave_rand, weave_base,
    under_speed, sink_extra, rise_trim, first_rise_fade, leap_height, leap_max_speed,
    leap_offset, leap_divisor, leap_attack_after, air_steer_speed, attack_blend,
    // The stats block the script sets (`+4` walk, `+8` run, `+0x10` hop) and KH1's defaults
    // (`0x1402A5D70`: `+0x14` air acceleration, `+0x18` air drag, `+0xD0` turn rate).
    walk_speed, run_speed, hop_height, air_accel, air_drag, turn_rate,
    // btltbl's globals (`btltbl.bin + 0x58`, `*0x142D22D30`) the reactions read: G+0x14 / +0x16
    // / +0x18 (launch), G+0x2C (ground flinch push), G+0x38 (air flinch), G+0x3C (air knock-back);
    // and the reaction code's own numbers (`0x1402B8530`'s down time 30, the 20 hop of
    // `0x1402B8B90` / `0x1402B77D0` / `0x1402B9140`).
    launch_min, launch_max, launch_hop, flinch_push, air_flinch_speed, air_knock_speed, down_time,
    air_knock_hop,
    // The Shadow's stat record `+0x18` (r6): the knock-back factor ("weight", decision M10: KH1's).
    weight,
    // KH1's gravity (`0x1403EAB54`) and fall cap (`0x1402AF6A0`).
    gravity, fall_cap,
}

impl Consts {
    /// From name / value pairs; every name of [`NAMES`] must be there.
    pub fn new(values: Vec<(String, f32)>) -> Result<Consts, String> {
        for n in NAMES {
            if !values.iter().any(|(k, _)| k == n) {
                return Err(format!("shadow constant {n} missing"));
            }
        }
        Ok(Consts { values })
    }

    fn get(&self, n: &str) -> f32 {
        self.values.iter().find(|(k, _)| k == n).map(|v| v.1).unwrap_or(f32::NAN)
    }

    pub fn names() -> &'static [&'static str] {
        NAMES
    }

    pub fn pairs(&self) -> &[(String, f32)] {
        &self.values
    }
}

// ---------------------------------------------------------------------------------------------
// Body physics and reactions: the shared engine code (M11, `crate::enemy`), unchanged.
// ---------------------------------------------------------------------------------------------

pub use crate::enemy::body::Body;
pub use crate::enemy::react::{get_up, landing, react, Hit, Motions, Phase, ReactionStart};

/// The clash recoil a blocked attack plays (attack row `+0x40`; `0x1402B8E10` case 6 / 7):
/// on the ground the request with bit 24 and the idle queued; in the air the request with a
/// 20 hop and request 6 queued (`0x40000006`). The push of a negative-damage record is not
/// built (Assumption: a block's record carries no negative damage).
pub fn clash(attack: Attack, airborne: bool, body: &mut Body, y: f32, c: &Consts, recoil: [u16; 3]) -> ReactionStart {
    let r = recoil[Attack::ALL.iter().position(|&a| a == attack).unwrap_or(0)];
    crate::enemy::react::clash(r, airborne, body, y, c)
}

// ---------------------------------------------------------------------------------------------
// The AI.
// ---------------------------------------------------------------------------------------------

/// What the AI asks of the host this frame (the shared commands, `crate::enemy::Cmd`, with the
/// Shadow's attacks).
pub type Cmd = crate::enemy::Cmd<Attack>;
pub use crate::enemy::{Draw, World};

/// The Shadow's own work area: weave phase (`wp+0x38`), underground (`wp+0x3C`), first rise
/// done (`wp+0x40`).
#[derive(Clone, Copy, Debug, Default)]
struct Wp {
    phase: f32,
    underground: bool,
    risen: bool,
}

type Cx = crate::enemy::script::Cx<Consts, Attack, Wp>;
type Task = crate::enemy::script::Task;

use crate::enemy::script::{approach, dist_to_target, face_target, frame, motion_frames, move_toward, poll, pos, push, rand, rand_range, random_offset, set_state, set_target, target, wait_grounded};

impl crate::enemy::Numbers for Consts {
    fn num(&self, name: &str) -> f32 {
        self.get(name)
    }
}

/// `0x10C6(goal)`: one underground step: the goal offset weaves by sin(phase) × 60°, the phase
/// advances by rand × 2 + 3 degrees, then a glide toward it at `under_speed`.
fn under_step(cx: &Cx, goal: Vec3) -> f32 {
    let (amp, wr, wb, sp) = {
        let io = cx.borrow();
        (io.c.weave_amplitude(), io.c.weave_rand(), io.c.weave_base(), io.c.under_speed())
    };
    let p = pos(cx);
    let phase = cx.borrow().x.phase;
    let ang = (phase.to_radians().sin() * amp).to_radians();
    let off = goal - p;
    let (s, co) = ang.sin_cos();
    let w = Vec3::new(off.x * co + off.z * s, off.y, -off.x * s + off.z * co) + p;
    let add = rand(cx, "weave") * wr + wb;
    cx.borrow_mut().x.phase += add;
    move_toward(cx, w, 2, sp)
}

/// `0x1056(goal, near)`: underground travel for at most `travel_timeout` frames, until stuck
/// (`0x160C(3)`: moved less than step × 3 × 0.5) or within `near`.
async fn travel(cx: &Cx, goal: Vec3, near: f32) {
    let (timeout, stuck) = {
        let io = cx.borrow();
        (io.c.travel_timeout(), io.c.stuck_factor())
    };
    let mut t = timeout;
    while t > 0.0 {
        let r = under_step(cx, goal);
        let s = cx.borrow().w.step;
        t -= s;
        frame().await;
        let (moved, step) = {
            let io = cx.borrow();
            (io.w.moved, io.w.step)
        };
        if moved - step * stuck * 0.5 < 0.0 || r - near < 0.0 {
            break;
        }
    }
}

/// `0x1328`: sink. Waits for the ground, plays 70 then 202, ramps the visibility 1 → 0 over
/// len(70) + `sink_extra` frames (facing the target), then goes underground.
async fn sink(cx: &Cx) {
    wait_grounded(cx).await;
    let c = cx.borrow().c.clone();
    cx.borrow_mut().x.underground = true;
    push(cx, Cmd::Motion { request: req::SINK, blend: c.attack_blend() });
    push(cx, Cmd::Queue { request: req::UNDERGROUND, blend: 0.0 });
    // `0x1366..0x138C`: flags |= 0x20, 0x40, 0x10 on the sink's first frame.
    push(cx, Cmd::Hittable(false));
    let len = motion_frames(cx, req::SINK) + c.sink_extra();
    let mut t = 0.0;
    while t < len {
        if target(cx).is_some() {
            face_target(cx);
        }
        push(cx, Cmd::Height(1.0 - t / len));
        t += cx.borrow().w.step;
        frame().await;
    }
    push(cx, Cmd::Height(1.0));
    push(cx, Cmd::Underground(true));
}

/// `0x114A`: rise. Waits for the ground, comes out (69 then the idle), ramps the height 0 → 1
/// over len(69) − `rise_trim` frames facing the target; the first rise (the battle appearance,
/// `wp+0x40` = 0) faces Sora at once, fades in over `first_rise_fade` frames (`0x2570`) and
/// keeps facing Sora.
async fn rise(cx: &Cx) {
    wait_grounded(cx).await;
    let c = cx.borrow().c.clone();
    push(cx, Cmd::Underground(false));
    let first = !cx.borrow().x.risen;
    if first {
        let to = {
            let io = cx.borrow();
            io.w.target.map(|t| t - io.w.pos)
        };
        if let Some(p) = to {
            let y = p.x.atan2(p.z);
            let mut io = cx.borrow_mut();
            io.body.yaw = y;
            io.body.yaw_target = y;
            io.out.push(Cmd::Yaw(y));
        }
        push(cx, Cmd::FadeIn(c.first_rise_fade()));
    }
    push(cx, Cmd::Motion { request: req::RISE, blend: 0.0 });
    push(cx, Cmd::Queue { request: req::IDLE, blend: c.attack_blend() });
    let len = motion_frames(cx, req::RISE) - c.rise_trim();
    let mut t = 0.0;
    while t < len {
        if first {
            let sora = cx.borrow().w.target;
            if let Some(p) = sora {
                let d = p - pos(cx);
                cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
            }
        } else {
            face_target(cx);
        }
        t += cx.borrow().w.step;
        push(cx, Cmd::Height(t / len));
        frame().await;
    }
    {
        let mut io = cx.borrow_mut();
        io.x.risen = true;
        io.x.underground = false;
    }
    push(cx, Cmd::Height(1.0));
    // `0x12D8..0x1314`: flags &= ~0x10, ~0x20, ~0x40 (the last unless `wp+0x44`, a scripted-room
    // flag `0x2C04` sets only in one Traverse Town room; Assumption: no Re:CoM battle needs it).
    push(cx, Cmd::Hittable(true));
    wait_grounded(cx).await;
}

/// `0xDDA`: taunt: face the target, play 203, wait rand(15, 75) frames facing it (stops if the
/// motion playing is 6).
async fn taunt(cx: &Cx) {
    let c = cx.borrow().c.clone();
    face_target(cx);
    push(cx, Cmd::Motion { request: req::TAUNT, blend: c.taunt_blend() });
    let n = rand_range(cx, c.taunt_wait_min(), c.taunt_wait_max(), "taunt wait");
    let mut t = 0.0;
    while t < n {
        if cx.borrow().w.motion == Some(req::AFTER_LEAP) {
            break;
        }
        face_target(cx);
        t += cx.borrow().w.step;
        frame().await;
    }
}

/// `0xF18` / `0xF6E`: the swipe / the claw: on the ground, face the target, the attack with its
/// motion (an action), 203 queued.
async fn strike(cx: &Cx, a: Attack) {
    wait_grounded(cx).await;
    face_target(cx);
    let blend = cx.borrow().c.attack_blend();
    push(cx, Cmd::Action { request: a.request(), attack: Some(a), then: req::TAUNT, blend });
    frame().await;
}

/// `0xE12`: the leap claw. `0x17D4`: on the ground, face the target, the horizontal speed
/// min((dist − offset) / divisor, max); `0x2142`: the jump start (3, an action, 4 queued held),
/// then the hop. In the air: the steering toward the target each frame (`+0x20`, speed
/// `air_steer_speed`), and from `leap_attack_after` frames the claw (201, an action, 6 queued).
async fn leap(cx: &Cx) {
    let c = cx.borrow().c.clone();
    wait_grounded(cx).await;
    let p = pos(cx);
    let to = target(cx).map_or(Vec3::from_yaw(cx.borrow().body.yaw), |t| t - p);
    let (dir, len) = to.normalize_len();
    cx.borrow_mut().body.yaw_target = dir.x.atan2(dir.z);
    let speed = ((len - c.leap_offset()) / c.leap_divisor()).min(c.leap_max_speed());
    push(cx, Cmd::Action { request: req::JUMP, attack: None, then: req::FALL, blend: 0.0 });
    frame().await;
    let v = dir * speed;
    let y = pos(cx).y;
    cx.borrow_mut().body.jump(c.leap_height(), Vec3::new(v.x, 0.0, v.z), y, &c);
    push(cx, Cmd::Jump { height: c.leap_height(), velocity: v });
    // `0xE3E`: while in the air (the jump sets movement state 2 at once, so the first pass is
    // this frame): steer toward the target; from `leap_attack_after` frames the claw (an action:
    // the loop waits for it, then goes on while still airborne).
    let mut t = 0.0;
    let mut first = true;
    loop {
        if !(first || cx.borrow().w.airborne) {
            break;
        }
        first = false;
        let p = pos(cx);
        if let Some(tg) = target(cx) {
            let (d, _) = (tg - p).normalize_len();
            let mut io = cx.borrow_mut();
            io.body.yaw_target = d.x.atan2(d.z);
            io.steer = Some(d * c.air_steer_speed());
        }
        if t - c.leap_attack_after() >= 0.0 {
            push(cx, Cmd::Action { request: req::LEAP_CLAW, attack: Some(Attack::LeapClaw), then: req::AFTER_LEAP, blend: 0.0 });
        }
        t += cx.borrow().w.step;
        frame().await;
    }
}

/// `0x1BC4(home, outer, inner)`: KH1's wander, the shared routine (`crate::enemy::script::wander`,
/// the Soldier's `0x15BA`): two random points about the Shadow's **current** position (`sys 7` adds
/// `actor+0x10`), walk to each, sometimes stand, home when too far. (M10's port took the points about
/// home; owner 2026-11-02: KH1's rule.)
async fn wander(cx: &Cx) {
    let c = cx.borrow().c.clone();
    crate::enemy::script::wander(cx, c.wander_inner(), c.wander_outer(), c.wander_band(), c.wander_timeout(), c.wander_idle_chance(), c.wander_idle_min(), c.wander_idle_max(), c.wander_home_max(), c.attack_blend()).await;
}

/// The idle behaviour (`0xD38` → `0xD50`): forever wander, then dig and come up elsewhere
/// (`0xD5E`: two random underground points).
async fn idle(cx: Cx) {
    set_target(&cx, false);
    loop {
        wander(&cx).await;
        let c = cx.borrow().c.clone();
        sink(&cx).await;
        let p = pos(&cx);
        let a = p + random_offset(&cx, c.idle_dig_a_min(), c.idle_dig_a_max());
        let b = p + random_offset(&cx, c.idle_dig_b_min(), c.idle_dig_b_max());
        travel(&cx, a, 0.0).await;
        travel(&cx, b, 0.0).await;
        rise(&cx).await;
        frame().await;
    }
}

/// The engaged behaviour (`0xB72` → `0xB96`), by the distance to the target.
async fn engaged(cx: Cx) {
    set_target(&cx, true);
    let c = cx.borrow().c.clone();
    let d = dist_to_target(&cx);
    if d - c.close_range() < 0.0 {
        // `0xC6A`.
        let d = dist_to_target(&cx);
        if d - c.swipe_range() < 0.0 {
            strike(&cx, Attack::Swipe).await;
        } else if d - c.claw_range() < 0.0 {
            if rand(&cx, "claw or leap") - c.claw_chance() < 0.0 {
                strike(&cx, Attack::Claw).await;
            } else {
                leap(&cx).await;
            }
        } else if rand(&cx, "taunt or leap") - c.close_taunt_chance() < 0.0 {
            taunt(&cx).await;
        } else {
            leap(&cx).await;
        }
        if rand(&cx, "taunt after") - c.after_taunt_chance() < 0.0 {
            taunt(&cx).await;
        }
        if rand(&cx, "dig after") - c.after_dig_chance() < 0.0 {
            // `0xFEA`: sink, come up within `near_dig_target` of the target.
            sink(&cx).await;
            let p = pos(&cx);
            let a = p + random_offset(&cx, c.near_dig_min(), c.near_dig_max());
            travel(&cx, a, c.near_dig_stop()).await;
            if let Some(t) = target(&cx) {
                travel(&cx, t, c.near_dig_target()).await;
            }
            rise(&cx).await;
        }
    } else if d - c.mid_range() < 0.0 {
        // `0xC5A`: run to the close range, then taunt.
        approach(&cx, 1, c.approach_timeout(), c.approach_retarget(), c.close_range()).await;
        taunt(&cx).await;
    } else {
        // `0xC1E`: taunt, travel half the way underground, sometimes run to the mid range.
        taunt(&cx).await;
        let half = dist_to_target(&cx) * c.far_fraction();
        sink(&cx).await;
        if let Some(t) = target(&cx) {
            travel(&cx, t, half).await;
        }
        rise(&cx).await;
        if rand(&cx, "far approach") - c.far_approach_chance() < 0.0 {
            approach(&cx, 1, c.approach_timeout(), c.approach_retarget(), c.mid_range()).await;
        }
    }
    // `0xBDA`: keep the target if within the engaged search; the state is set again, so the
    // supervisor restarts this behaviour (it loops).
    if dist_to_target(&cx) - c.engaged_search() < 0.0 {
        set_target(&cx, true);
    }
    set_state(&cx, 1);
}

/// The "hit" behaviour (`0xB3C` → `0xB60`): the attacker becomes the target, then engaged.
async fn hit(cx: Cx) {
    set_target(&cx, true);
    set_state(&cx, 1);
}

/// The Shadow's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
    started: bool,
}

impl Ai {
    /// A new AI. `seed`: its random stream (KH1's shared CRT stream isn't reproduced: per-enemy
    /// seeds, Assumption). `frames`: KH1 motion lengths by request (`sys 43`, Assumption: the
    /// motion's frame count). `risen`: the battle appearance already played its rise.
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, risen: bool) -> Ai {
        let blend = c.attack_blend();
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, blend, Wp { phase: 0.0, underground: false, risen });
        Ai { io: Rc::new(RefCell::new(io)), main: None, behaviour: None, started: false }
    }

    /// The appearance was not KH1's rise (Re:CoM's own appearance ran): no first rise.
    pub fn skip_rise(&self) {
        self.io.borrow_mut().x.risen = true;
    }

    /// Whether the battle appearance (the first rise) is done.
    pub fn risen(&self) -> bool {
        self.io.borrow().x.risen
    }

    /// The body physics (the host steps it every frame, the AI's frames or not).
    pub fn body(&self) -> std::cell::RefMut<'_, Body> {
        std::cell::RefMut::map(self.io.borrow_mut(), |io| &mut io.body)
    }

    pub fn consts(&self) -> Consts {
        self.io.borrow().c.clone()
    }

    pub fn underground(&self) -> bool {
        self.io.borrow().x.underground
    }

    /// The air steering the script set this frame (cleared each frame).
    pub fn take_steer(&self) -> Option<Vec3> {
        self.io.borrow_mut().steer.take()
    }

    /// The random draws since the last call (the log).
    pub fn take_draws(&self) -> Vec<Draw> {
        std::mem::take(&mut self.io.borrow_mut().draws)
    }

    /// The reaction code's random roll (the launch's down time comes from the same stream).
    pub fn with_rng<T>(&self, f: impl FnOnce(&mut Kh1Rand) -> T) -> T {
        f(&mut self.io.borrow_mut().rng)
    }

    /// The hit handler (`0x2988`): a damaging, non-lethal hit sets state 2.
    pub fn on_hit(&self) {
        set_state(&self.io, 2);
    }

    /// One AI frame (the host calls it only while no action runs): the supervisor (`0x26B2`),
    /// then the behaviour thread. Returns the commands.
    pub fn frame(&mut self, w: World) -> Vec<Cmd> {
        self.io.borrow_mut().w = w;
        if !self.started {
            self.started = true;
            let cx = self.io.clone();
            {
                let mut io = cx.borrow_mut();
                // `0x27F8`: home, the weave phase rand × 360.
                io.home = w.pos;
                let v = io.rng.next_f32();
                io.draws.push(Draw { value: v, what: "weave phase" });
                io.x.phase = v * 360.0;
                io.state = (0, false);
                // `0x28B0..0x28E0`: spawned from the floor: flags |= 0x100, 0x20, 0x40.
                if !io.x.risen {
                    io.out.push(Cmd::Hittable(false));
                }
            }
            self.main = Some(Box::pin(supervisor(cx)));
        }
        poll(&mut self.main);
        // Restart on a state change (the supervisor stores the request).
        let restart = self.io.borrow().out.iter().rev().find_map(|c| match c {
            Cmd::State(s) => Some(*s),
            _ => None,
        });
        if let Some(s) = restart {
            let cx = self.io.clone();
            self.behaviour = Some(match s {
                0 => Box::pin(idle(cx)),
                1 => Box::pin(engaged(cx)),
                _ => Box::pin(hit(cx)),
            });
        }
        poll(&mut self.behaviour);
        std::mem::take(&mut self.io.borrow_mut().out)
    }
}

/// The framework's interface (M11, `crate::enemy::Behaviour`): the same calls the M10 host made.
impl crate::enemy::Behaviour for Ai {
    fn frame(&mut self, w: World) -> Vec<crate::enemy::Cmd<u8>> {
        Ai::frame(self, w).iter().map(|c| c.map(|a| Attack::ALL.iter().position(|&x| x == a).unwrap_or(0) as u8)).collect()
    }
    fn body(&self) -> std::cell::RefMut<'_, Body> {
        Ai::body(self)
    }
    fn take_steer(&self) -> Option<Vec3> {
        Ai::take_steer(self)
    }
    fn take_draws(&self) -> Vec<Draw> {
        Ai::take_draws(self)
    }
    fn rng(&self) -> Kh1Rand {
        self.with_rng(|r| r.clone())
    }
    fn set_rng(&self, r: Kh1Rand) {
        self.with_rng(|x| *x = r);
    }
    /// The hit handler (`0x2988`): a damaging hit sets state 2; a clash is no hit (M10).
    fn on_hit(&self, news: crate::enemy::HitNews) {
        if news == crate::enemy::HitNews::Damage {
            Ai::on_hit(self);
        }
    }
    fn risen(&self) -> bool {
        Ai::risen(self)
    }
    fn skip_rise(&self) {
        Ai::skip_rise(self)
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["idle", "engaged", "hit"].get(s as usize).copied().unwrap_or("?")
    }
}

/// The supervisor (`0x26B2`): after the init frame, the first rise when spawned from the floor
/// (`mdl_5a`; every Re:CoM battle appearance), the idle behaviour, then each frame: nothing while
/// underground; idle with Sora within `idle_search` → engaged; a state change restarts the
/// behaviour thread.
async fn supervisor(cx: Cx) {
    frame().await;
    if !cx.borrow().x.risen {
        rise(&cx).await;
    }
    push(&cx, Cmd::State(0));
    loop {
        if cx.borrow().x.underground {
            frame().await;
            continue;
        }
        let c = cx.borrow().c.clone();
        let st = cx.borrow().state.0;
        if st == 0 {
            let tg = cx.borrow().w.target;
            if let Some(t) = tg {
                if (t - pos(&cx)).length() - c.idle_search() < 0.0 {
                    set_target(&cx, true);
                    set_state(&cx, 1);
                }
            }
        }
        let change = {
            let mut io = cx.borrow_mut();
            let ch = io.state.1;
            io.state.1 = false;
            ch.then_some(io.state.0)
        };
        if let Some(s) = change {
            push(&cx, Cmd::State(s));
        }
        frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consts() -> Consts {
        // Test numbers (the script's own come from the player's file at the asset step).
        let v: &[(&str, f32)] = &[
            ("close_range", 450.0), ("mid_range", 800.0), ("swipe_range", 150.0), ("claw_range", 300.0), ("claw_chance", 0.9),
            ("close_taunt_chance", 0.3), ("after_taunt_chance", 0.8), ("after_dig_chance", 0.7), ("far_fraction", 0.5),
            ("far_approach_chance", 0.1), ("idle_search", 1500.0), ("engaged_search", 3000.0), ("taunt_blend", 8.0),
            ("taunt_wait_min", 15.0), ("taunt_wait_max", 75.0), ("approach_timeout", 120.0), ("approach_retarget", 10.0),
            ("idle_dig_a_min", 300.0), ("idle_dig_a_max", 500.0), ("idle_dig_b_min", 700.0), ("idle_dig_b_max", 1000.0),
            ("wander_inner", 300.0), ("wander_outer", 400.0), ("wander_band", 100.0), ("wander_timeout", 300.0),
            ("wander_idle_chance", 0.4), ("wander_idle_min", 90.0), ("wander_idle_max", 180.0), ("wander_home_max", 700.0),
            ("near_dig_min", 200.0), ("near_dig_max", 300.0), ("near_dig_stop", 100.0), ("near_dig_target", 150.0),
            ("travel_timeout", 60.0), ("stuck_factor", 3.0), ("weave_amplitude", 60.0), ("weave_rand", 2.0), ("weave_base", 3.0),
            ("under_speed", 6.0), ("sink_extra", 45.0), ("rise_trim", 30.0), ("first_rise_fade", 30.0), ("leap_height", 150.0),
            ("leap_max_speed", 8.0), ("leap_offset", 100.0), ("leap_divisor", 20.0), ("leap_attack_after", 16.0),
            ("air_steer_speed", 8.0), ("attack_blend", 8.0), ("walk_speed", 2.0), ("run_speed", 5.0), ("hop_height", 150.0),
            ("air_accel", 0.1), ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0), ("launch_min", 20.0),
            ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0),
            ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("weight", 1.0),
            ("gravity", crate::params::GRAVITY), ("fall_cap", 20.0),
        ];
        Consts::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect()).unwrap()
    }

    fn frames() -> Vec<(u16, f32)> {
        vec![(req::RISE, 80.0), (req::SINK, 60.0), (req::TAUNT, 180.0)]
    }

    /// A flat floor at y = 0, Sora standing at `sora`.
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
                } else {
                    for c in self.ai.frame(w) {
                        if let Cmd::Action { .. } = c {
                            self.busy = 30.0;
                        }
                        self.log.push(c);
                    }
                }
                let c = self.ai.consts();
                let steer = self.ai.take_steer();
                let d = self.ai.body().step(1.0, self.pos.y, steer, &c);
                self.pos += d;
                if self.pos.y >= 0.0 && self.ai.body().falling() {
                    self.pos.y = 0.0;
                    self.ai.body().land();
                }
            }
        }
    }

    fn sim(seed: u32) -> Sim {
        Sim { ai: Ai::new(consts(), frames(), seed, 0.0, true), pos: Vec3::ZERO, busy: 0.0, log: Vec::new() }
    }

    #[test]
    fn close_target_attacks() {
        // Sora 1 m away: the swipe (< 150), from the engaged behaviour, with its motion.
        let mut s = sim(5);
        s.run(Vec3::new(0.0, 0.0, 100.0), 3);
        assert!(s.log.contains(&Cmd::State(1)), "{:?}", s.log);
        assert!(s.log.iter().any(|c| matches!(c, Cmd::Action { attack: Some(Attack::Swipe), request: 204, then: 203, .. })), "{:?}", s.log);
    }

    #[test]
    fn claw_range_draws_and_picks() {
        // 2 m: claw 90 % / leap 10 %, one draw "claw or leap".
        let mut claws = 0;
        let mut leaps = 0;
        for seed in 0..200 {
            let mut s = sim(seed);
            s.run(Vec3::new(0.0, 0.0, 200.0), 3);
            if s.log.iter().any(|c| matches!(c, Cmd::Action { attack: Some(Attack::Claw), .. })) {
                claws += 1;
            }
            if s.log.iter().any(|c| matches!(c, Cmd::Action { request: req::JUMP, .. })) {
                leaps += 1;
            }
        }
        assert_eq!(claws + leaps, 200);
        assert!((150..200).contains(&claws), "claws {claws}");
    }

    #[test]
    fn mid_range_runs_in() {
        let mut s = sim(1);
        s.run(Vec3::new(0.0, 0.0, 600.0), 4);
        assert!(s.log.iter().any(|c| matches!(c, Cmd::Motion { request: req::RUN, .. })), "{:?}", s.log);
        // It covers ground at the run speed (5 a frame).
        let before = s.pos;
        s.run(Vec3::new(0.0, 0.0, 600.0), 10);
        assert!((s.pos - before).length() > 30.0);
    }

    #[test]
    fn far_target_sinks() {
        let mut s = sim(2);
        s.run(Vec3::new(0.0, 0.0, 1000.0), 260);
        assert!(s.log.iter().any(|c| matches!(c, Cmd::Motion { request: req::TAUNT, .. })));
        assert!(s.log.iter().any(|c| matches!(c, Cmd::Motion { request: req::SINK, .. })), "{:?}", s.log);
        assert!(s.log.contains(&Cmd::Underground(true)));
    }

    #[test]
    fn unhittable_from_the_sink_to_the_rise_ramps_end() {
        // KH1's flag 0x40: set with request 70, cleared rise frame len(69) − 30 = 50 later.
        let mut s = sim(2);
        let sora = Vec3::new(0.0, 0.0, 1000.0);
        let mut at = |s: &mut Sim, n: usize| -> Vec<(usize, Cmd)> {
            let mut out = Vec::new();
            for f in 0..n {
                let k = s.log.len();
                s.run(sora, 1);
                out.extend(s.log[k..].iter().cloned().map(|c| (f, c)));
            }
            out
        };
        let log = at(&mut s, 600);
        let f = |want: &Cmd| log.iter().find(|(_, c)| c == want).map(|x| x.0);
        let sink = log.iter().find(|(_, c)| matches!(c, Cmd::Motion { request: req::SINK, .. })).unwrap().0;
        assert_eq!(f(&Cmd::Hittable(false)), Some(sink), "same frame as request 70");
        let rise = log.iter().find(|(_, c)| matches!(c, Cmd::Motion { request: req::RISE, .. })).unwrap().0;
        assert_eq!(f(&Cmd::Hittable(true)), Some(rise + 50), "rise frame 50");
        assert!(!log.iter().any(|(t, c)| *c == Cmd::Hittable(true) && *t < rise));
    }

    #[test]
    fn idle_without_target_wanders() {
        let mut s = sim(3);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 30);
        assert!(s.log.contains(&Cmd::State(0)));
        assert!(s.log.iter().any(|c| matches!(c, Cmd::Motion { request: req::WALK, .. })));
    }

    #[test]
    fn hit_restarts_engaged() {
        let mut s = sim(4);
        s.run(Vec3::new(0.0, 0.0, 5000.0), 5);
        s.ai.on_hit();
        s.run(Vec3::new(0.0, 0.0, 100.0), 3);
        let i2 = s.log.iter().position(|c| *c == Cmd::State(2)).unwrap();
        assert!(s.log[i2..].contains(&Cmd::State(1)));
    }

    #[test]
    fn leap_hops_and_claws_in_the_air() {
        // 3.5 m, mostly the leap (70 %): find a seed that leaps.
        for seed in 0..50 {
            let mut s = sim(seed);
            s.run(Vec3::new(0.0, 0.0, 350.0), 3);
            if !s.log.iter().any(|c| matches!(c, Cmd::Action { request: req::JUMP, .. })) {
                continue;
            }
            s.run(Vec3::new(0.0, 0.0, 350.0), 70);
            assert!(s.log.iter().any(|c| matches!(c, Cmd::Jump { height, .. } if *height == 150.0)));
            assert!(s.log.iter().any(|c| matches!(c, Cmd::Action { attack: Some(Attack::LeapClaw), then: 6, .. })), "{:?}", s.log);
            return;
        }
        panic!("no seed leapt");
    }

    #[test]
    fn battle_appearance_rises_first() {
        // Not risen: init frame, then 69 with the fade-in, 50 frames of height (80 − 30), then
        // the idle state and (Sora near) engaged.
        let mut s = Sim { ai: Ai::new(consts(), frames(), 7, 0.0, false), pos: Vec3::ZERO, busy: 0.0, log: Vec::new() };
        s.run(Vec3::new(0.0, 0.0, 100.0), 2);
        assert!(s.log.contains(&Cmd::Motion { request: req::RISE, blend: 0.0 }));
        assert!(s.log.contains(&Cmd::FadeIn(30.0)));
        assert!(!s.log.iter().any(|c| matches!(c, Cmd::State(_))));
        assert!(s.log.contains(&Cmd::Hittable(false)), "spawned unhittable");
        assert!(!s.log.contains(&Cmd::Hittable(true)), "until the rise's ramp ends");
        s.run(Vec3::new(0.0, 0.0, 100.0), 52);
        let heights = s.log.iter().filter(|c| matches!(c, Cmd::Height(_))).count();
        assert_eq!(heights, 51, "50 ramp frames + the final 1.0");
        let i0 = s.log.iter().position(|c| *c == Cmd::State(0)).expect("idle after the rise");
        let ih = s.log.iter().position(|c| *c == Cmd::Hittable(true)).expect("hittable at the ramp's end");
        assert!(ih < i0);
        assert_eq!(s.log[i0 + 1..].iter().find(|c| matches!(c, Cmd::State(_))), Some(&Cmd::State(1)));
        assert!(s.ai.risen());
    }

    struct All;
    impl Motions for All {
        fn has(&self, _: u16) -> bool {
            true
        }
    }
    struct Shadow;
    impl Motions for Shadow {
        fn has(&self, r: u16) -> bool {
            !(0x50..=0x51).contains(&r)
        }
    }

    #[test]
    fn reactions_follow_kh1() {
        let c = consts();
        let mut rng = Kh1Rand::new(1);
        let mut b = Body::default();
        // Facing +z, pushed toward −z (hit from the front): 0x48 part 0, 0x49 part 1.
        let front = Vec3::new(0.0, 0.0, -1.0);
        let r = react(Hit { kind: 3, dir: front, part: 0 }, 0.0, None, false, false, &All, &c, &mut rng, &mut b, 0.0).unwrap();
        assert_eq!((r.request, r.then), (0x48, 0));
        assert!(b.push.is_some(), "the flinch slides 80 × weight");
        let r = react(Hit { kind: 3, dir: front, part: 1 }, 0.0, None, false, false, &All, &c, &mut rng, &mut b, 0.0).unwrap();
        assert_eq!(r.request, 0x49);
        // From behind: 0x4A; the same as the one playing alternates to 0x4B.
        let back = Vec3::new(0.0, 0.0, 1.0);
        assert_eq!(react(Hit { kind: 3, dir: back, part: 0 }, 0.0, Some(0x4A), false, false, &All, &c, &mut rng, &mut b, 0.0).unwrap().request, 0x4B);
        // Launch from the front: 0x52 then 0x53, speed 35 (weight 1), hop 47.25.
        let mut b = Body::default();
        let r = react(Hit { kind: 5, dir: front, part: 0 }, 0.0, None, false, false, &All, &c, &mut rng, &mut b, 0.0).unwrap();
        assert_eq!((r.request, r.then, r.base), (0x52, 0x53, Some(0x52)));
        assert!((b.air_vel.length() - 35.0).abs() < 1e-3);
        assert!((b.arc.unwrap().rise + 47.25).abs() < 1e-3);
        assert!((0.0..30.0).contains(&r.down));
        // Kind 4 in the air without 0x50: the air launch (the Shadow's case).
        let mut b = Body::default();
        let r = react(Hit { kind: 4, dir: back, part: 0 }, 0.0, None, true, false, &Shadow, &c, &mut rng, &mut b, 0.0).unwrap();
        assert_eq!(r.request, 0x5C + 5);
        assert_eq!(landing(Some(0x52), false), (0x54, 0x55, true));
        assert_eq!(get_up(0x52, true), 0x5B);
    }

    #[test]
    fn push_slide_totals_its_distance() {
        let c = consts();
        let mut b = Body::default();
        b.push(Vec3::new(1.0, 0.0, 0.0), 80.0);
        let mut x = 0.0;
        for _ in 0..200 {
            x += b.step(1.0, 0.0, None, &c).x;
        }
        assert!(b.push.is_none());
        // KH1 moves by the speed, then decays it: 80 × 0.2231 / (1 − 0.8), less the tail.
        assert!((80.0..89.3).contains(&x), "{x}");
    }
}
