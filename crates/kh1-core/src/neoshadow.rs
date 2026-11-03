//! KH1 FM's Neoshadow (`xa_ex_2420.mdls` part 5, `ex_2420.bd`), engine-agnostic: its AI script on the
//! shared enemy framework (`crate::enemy`). M11 (`docs/recom-m11-enemies.md` § The Neoshadow). Re:CoM runs
//! it natively: each part is the step of a Re:CoM enemy state (`kh1-recom`'s `win::kh1_enemy`), Re:CoM
//! plays the motions, collides the body and the attacks, applies the damage.
//!
//! Every number is data ([`NAMES`], read by the asset step from the player's `ex_2420.bd` at checked
//! offsets); nothing of the game is in this file. Script offsets are the player's `ex_2420.bd`'s (Checked
//! (files): the script disassembled with `tools/kh1-ai-script.py`, code from `0x36A`; the VM's ops and the
//! syscalls named, Checked (decomp)).
//!
//! # The script's shape
//!
//! - The Neoshadows of a room share one work block (`sys 138` → `0x1402BC620`: `DAT_142DB7B5C`, 0x80
//!   bytes, zeroed by the room set-up `0x140287C60`; Checked (decomp)): [`Group`]. Its first word counts
//!   the Neoshadows that came (each init adds one, each death takes one); most of the behaviour is chosen
//!   by that count: five or more gather in the air and dive at Sora one after another; three or four sink
//!   together and rise slashing out of nine places of a formation; with two, one of them goes underground
//!   for good and ambushes Sora from below (KH1 grabs him there: not built, see [`grab_or_miss`]) while
//!   the other fights; alone, it goes to the middle of the room, calls more Neoshadows and fights.
//! - entry `0x443E`: init `0x46F8`, a yield; a Neoshadow made while another is calling (`+0x50` = 1) rises
//!   as a called one (`0xFC8`), the others go idle (`0xF34`); then each frame a state request restarts the
//!   behaviour thread (1 idle `0xF34`, 2 engaged `0x724`, 3 `0x680` (never asked for), 4 dive `0x5C6`,
//!   5 hit `0x3DE`), and while it travels underground (`wp+0x50`) nothing else.
//! - the hit handler `0x498C` (see [`Ai::on_hit`] and [`Behaviour::armoured`]).
//!
//! # Where KH1's room shows through (owner question, `docs/recom-m11-enemies.md` § Neoshadow owner questions)
//!
//! KH1 FM places this model only in End of the World's room 29 (Checked (files): of the player's 214
//! `.ard` rooms only `ew29.ard` names `ex_2420`; KH1's world 16 is `ew`, the world code table
//! `0x1404DCCA0`, Checked (exe)). Its script names points of that room: the nine places of three
//! formations (`0x2430`, picked by Sora's x), the spot the last one goes to (`0xAE6`), the one the hidden
//! one comes up at (`0xA32`), the one a stuck traveller turns to (`0x58AE`), and a height (`0x3736`). The
//! port runs the script as it runs there (world 16) and takes that room's points about the battle: the
//! middle formation's first place ([`ROOM_REF`]) stands for where Sora started the battle, heights from the
//! battle floor ([`Group::centre`], Assumption).
//!
//! Not ported: KH1's effects and their positions (`sys 74` / `75` / `97` / `15` / `16` / `21` / `107`, the
//! `sys 1.3` removals: M13); the hurt-part masks (`sys 116` / `117`); `sys 124` (a `+0x130` block's `+0x18`,
//! written 0 during its attacks and 100 after: not identified, not the knock-back weight the reactions read,
//! stats `+0xC4`; Checked (decomp)); `sys 129` / `173` / `208` / `214` (flags); the pad rumble `sys 206`;
//! the spawn of the room's sleeping objects 600..606 (`sys 37` with `0xB0000 | id`, `0x140290D60` →
//! `0x140287850`: Re:CoM's battles have none, so KH1's lookup finds nothing; Checked (decomp)); the grab.

use crate::enemy::guard::HELD;
use crate::enemy::script::{face_target, frame, motion_done, motion_frames, poll, pos, push, rand, set_state, set_target, take_state, target, tick_clock, wait_frames, wait_grounded, walk_to, warp};
use crate::enemy::{Behaviour, Body, Cmd as ECmd, Draw, HitNews, Numbers, Table, World};
use crate::math::Vec3;
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// KH1's motion requests the Neoshadow's script plays (its `.mset` map; Checked (files)).
pub mod req {
    pub use crate::enemy::react::req::*;
    /// Its death (`0x5304`: request 72 facing the attacker): KH1's front flinch 0x48.
    pub const DEATH: u16 = 72;
    /// The strafe's two sides (`0x5354`: 85° to the left 200, to the right 201).
    pub const STRAFE_L: u16 = 200;
    pub const STRAFE_R: u16 = 201;
    /// The claw (`0x1878`), 74 frames, root motion, its hands' HIT slots 0 [40, 54) and 1 [52, 68).
    pub const CLAW: u16 = 202;
    /// The crouch before its leap (`0x1950` / `0x3036`), 66 frames, 204 queued (the air, 30 frames, looped).
    pub const CROUCH: u16 = 203;
    pub const HOVER: u16 = 204;
    /// The dive (`0x3326`), 40 frames, looped, its body's HIT slot 0 on throughout.
    pub const DIVE: u16 = 205;
    /// The landing after the dive (`0x35B0`), 48 frames, 6 queued.
    pub const LAND_DIVE: u16 = 206;
    /// The rising slash out of the floor (`0x1568`, `0x1FB0`), 120 frames, its body's HIT slot 0 [26, 50).
    pub const RISE_SLASH: u16 = 207;
    /// KH1's grab from below (`0x1D66`, not built) and the miss (`0x1EB6`), 126 / 136 frames.
    pub const GRAB: u16 = 208;
    pub const MISS: u16 = 209;
    /// The call (`0x12C0`), 240 frames.
    pub const CALL: u16 = 210;
    /// A called one's rise (`0x10F0` / `0x1110` / `0x11C2`): 211 (82), 212 (150, looped, 4 times), 213
    /// (an action, 122).
    pub const CALLED_RISE: u16 = 211;
    pub const CALLED_HOLD: u16 = 212;
    pub const CALLED_END: u16 = 213;
    /// Sinking (`0x5A98`), 60 frames, 217 queued (underground, 32 frames, looped); coming up (`0x596A`), 70.
    pub const SINK: u16 = 216;
    pub const UNDER: u16 = 217;
    pub const COME_UP: u16 = 218;
}

/// The Neoshadow's attacks, in the order the host indexes them (the manifest's `attacks`). All three are
/// plain requests made with their rows (`Cmd::Attack`: the script keeps running while they play).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attack {
    /// 202, rows `0x1D0` (code 28) / `0x240` (code 29).
    Claw,
    /// 207, row `0x160` (code 8, a line 1 → 0).
    RiseSlash,
    /// 205, row `0xF0` (code 8).
    Dive,
}

impl Attack {
    pub const ALL: [Attack; 3] = [Attack::Claw, Attack::RiseSlash, Attack::Dive];

    pub fn request(self) -> u16 {
        match self {
            Attack::Claw => req::CLAW,
            Attack::RiseSlash => req::RISE_SLASH,
            Attack::Dive => req::DIVE,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Attack::Claw => "claw",
            Attack::RiseSlash => "rise slash",
            Attack::Dive => "dive",
        }
    }

    /// The damaging attack rows (script offsets, 0x70 bytes, `sys 23`) made with it.
    pub fn rows(self) -> &'static [usize] {
        match self {
            Attack::Claw => &[0x1D0, 0x240],
            Attack::RiseSlash => &[0x160],
            Attack::Dive => &[0xF0],
        }
    }

    pub fn index(self) -> u8 {
        Attack::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8
    }
}

/// The body row `0x80` (code 8, radius 80, clash type 4, priority 100, cone cos −1: all round), made in
/// KH1's mode 3 (`sys 213(row, 3)`: on every frame until `sys 213(row, 0)`) while the Neoshadow leaps,
/// dives and rises slashing (`wp+0x58` = 1): Sora's swings bounce off it (his kind 7 recoil; a side at 100
/// is never moved, so the Neoshadow hears nothing). The manifest's only guard row.
pub const SHIELD: u8 = 0;
/// The shield row's script offset.
pub const SHIELD_ROW: usize = 0x80;

/// The names of the Neoshadow's numbers (besides `crate::enemy::ENGINE_NAMES`).
pub const NAMES: &[&str] = &[
    // The count thresholds the script compares the group's count with.
    "crowd", "group_min", "pair", "gather_max", "lone",
    "search_range",
    // The hit state `0x3DE`.
    "counter_rise_chance", "counter_away_min", "counter_away_max",
    // The rounds `0xC3A`.
    "rounds", "lone_rounds", "near_range", "near_claw_chance", "near_away_min", "near_away_max", "regroup_min", "regroup_max", "regroup_time",
    "near_rise_chance", "mid_range", "mid_chance", "mid_rise_chance", "strafe_speed", "strafe_near", "strafe_time", "approach_timeout",
    "approach_retarget", "approach_dist", "far_run_chance", "run_time", "run_near", "run_steer_speed", "far_near_min", "far_near_max",
    // The claw `0x1878`.
    "claw_blend", "claw_extra", "claw_face",
    // The rising slash `0x1568`.
    "rise_wait", "rise_near_max", "shield_end", "idle_blend",
    // Underground: `0x552C` / `0x55D2` / `0x5652`.
    "under_time", "under_speed", "under_fast_speed", "under_near", "weave_turn", "weave_flip", "weave_flip_chance", "weave_flip_low",
    "weave_flip_high", "stuck_after", "stuck_factor", "stuck_flip", "stuck_angle_min", "stuck_angle_max", "stuck_dist_min", "stuck_dist_max",
    "under_blend",
    // Sinking `0x5A98`, coming up `0x596A`.
    "sink_blend", "sink_extra", "sink_hide", "sink_ramp", "come_up_trim",
    // The run at Sora `0x5BE8`.
    "run_blend", "run_stuck_after", "run_stuck", "run_end_wait",
    // The strafe `0x5354`.
    "strafe_stuck_after", "strafe_side_chance", "strafe_angle", "strafe_blend", "strafe_stuck_factor",
    // The leap and the dive `0x1950` / `0x3036` / `0x3326`.
    "crouch_blend", "crouch_extra", "lift_window", "lift_height", "lift_speed", "launch_wait", "dive_blend", "dive_row_after", "dive_cycles",
    "dive_reaim", "dive_near", "dive_level", "dive_speed", "dive_stuck_after", "dive_stuck", "dive_accel_after", "land_trim",
    // The aim `0x3664`, the walk home after a dive `0x372C`.
    "aim_above", "aim_factor", "aim_min", "aim_far", "home_height", "home_time",
    // The formation `0x1FB0` / `0x2430`.
    "form_wait", "form_warp_wait", "form_rise_after", "layout_low", "layout_high", "slot_draw_min", "slot_draw_max",
    // Two left: the hidden one `0x7C2` / `0xBA2` / `0x1AF6`.
    "special_wait_min", "special_wait_max", "grab_follow", "grab_range", "grab_wait", "miss_depth",
    // Alone `0x8FA`: the hidden one's spot, the spot it goes to, the call `0x11FA`.
    "special_x", "special_y", "special_z", "special_rise_wait", "lone_x", "lone_y", "lone_z", "call_blend", "call_extra", "call_window",
    // A called one's rise `0xFC8`, a called one vanishing `0x5C6`.
    "called_wait", "called_hold", "vanish_fade",
    // The stuck traveller's spot `0x58AE`.
    "stuck_x", "stuck_y", "stuck_z",
    // The formations' places `0x2430` (layout, place, axis; the first word of each `0x4404` call).
    "slot_0_0_x", "slot_0_0_y", "slot_0_0_z", "slot_0_1_x", "slot_0_1_y", "slot_0_1_z", "slot_0_2_x", "slot_0_2_y", "slot_0_2_z",
    "slot_0_3_x", "slot_0_3_y", "slot_0_3_z", "slot_0_4_x", "slot_0_4_y", "slot_0_4_z", "slot_0_5_x", "slot_0_5_y", "slot_0_5_z",
    "slot_0_6_x", "slot_0_6_y", "slot_0_6_z", "slot_0_7_x", "slot_0_7_y", "slot_0_7_z", "slot_0_8_x", "slot_0_8_y", "slot_0_8_z",
    "slot_1_0_x", "slot_1_0_y", "slot_1_0_z", "slot_1_1_x", "slot_1_1_y", "slot_1_1_z", "slot_1_2_x", "slot_1_2_y", "slot_1_2_z",
    "slot_1_3_x", "slot_1_3_y", "slot_1_3_z", "slot_1_4_x", "slot_1_4_y", "slot_1_4_z", "slot_1_5_x", "slot_1_5_y", "slot_1_5_z",
    "slot_1_6_x", "slot_1_6_y", "slot_1_6_z", "slot_1_7_x", "slot_1_7_y", "slot_1_7_z", "slot_1_8_x", "slot_1_8_y", "slot_1_8_z",
    "slot_2_0_x", "slot_2_0_y", "slot_2_0_z", "slot_2_1_x", "slot_2_1_y", "slot_2_1_z", "slot_2_2_x", "slot_2_2_y", "slot_2_2_z",
    "slot_2_3_x", "slot_2_3_y", "slot_2_3_z", "slot_2_4_x", "slot_2_4_y", "slot_2_4_z", "slot_2_5_x", "slot_2_5_y", "slot_2_5_z",
    "slot_2_6_x", "slot_2_6_y", "slot_2_6_z", "slot_2_7_x", "slot_2_7_y", "slot_2_7_z", "slot_2_8_x", "slot_2_8_y", "slot_2_8_z",
];

/// The Neoshadow's numbers.
pub type Consts = Table;

/// The point of KH1's room 29 that stands for the battle's middle (where Sora started it), on its floor:
/// the middle formation's first place (`slot_2_0`, `0x275E`: (8800, 5480, 100), the place nearest the
/// middle of the three formations and of the room's spots). Assumption (the mapping, owner question).
pub const ROOM_REF: (usize, usize) = (2, 0);

/// The Neoshadow's state objects (`wp+0x20`).
pub mod st {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const ENGAGED: u8 = 2;
    /// `0x680` (a search, then idle): no script asks for it.
    pub const STATE3: u8 = 3;
    pub const DIVE: u8 = 4;
    pub const HIT: u8 = 5;
}

/// The hit state's mark (`wp+0x3C`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mark {
    #[default]
    None,
    /// 100: a knock-back / launch, or the fourth flinch in a row: the counter (`0x3F8`).
    Counter,
}

/// The block the Neoshadows of a battle share (KH1's `sys 138` block, its words by offset).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Group {
    /// `+0`: how many came (each init adds one unless it is a called one, each death takes one).
    pub count: i32,
    /// `+4`: how many are up in the air for the gathered dive (`0x31B0`).
    pub ready: i32,
    /// `+8`: one diver at a time (`0x32FA`).
    pub token: i32,
    /// `+0xC`: the gathered dive started (`0x322C`).
    pub launched: i32,
    /// `+0x10`: how many sank for the formation (`0x2114`).
    pub sunk: i32,
    /// `+0x14`: the formation goes (`0x219A`).
    pub go: i32,
    /// `+0x18`: the formation's effects made (`0x21D6`, M13).
    pub effects: i32,
    /// `+0x1C..+0x3C`: the nine places taken (`0x28DC`..).
    pub slots: [bool; 9],
    /// `+0x40` / `+0x44`: the formation picked (`0x245A`) and which (`0x247E`..).
    pub layout_chosen: bool,
    pub layout: usize,
    /// `+0x48`: two left: one went underground for good (`0x7D2`).
    pub special: bool,
    /// `+0x4C`: KH1's grab holds Sora: the others dive at him (`0x1CC4`; never set: the grab is not built).
    pub grab: bool,
    /// `+0x50`: one is calling (`0x1226`); a Neoshadow made then comes up as a called one.
    pub calling: bool,
    /// `+0x54`: called ones still coming up (`0x48C6`, `0x117E`).
    pub coming: i32,
    /// `+0x58`: the last counted one died (`0x4EC6`): the called ones vanish.
    pub last_died: bool,
    /// The port's: Sora's place when the first Neoshadow of the battle first saw him (KH1 units), the
    /// battle's stand-in for [`ROOM_REF`].
    pub centre: Option<Vec3>,
}

/// The battles' groups: the last battle's only (a new battle starts a new one; KH1's block is zeroed with
/// each room).
static GROUPS: Mutex<Option<(u32, Arc<Mutex<Group>>)>> = Mutex::new(None);

/// The group of `battle` (the one its first Neoshadow made).
pub fn group_for_battle(battle: u32) -> Arc<Mutex<Group>> {
    let mut g = GROUPS.lock().unwrap_or_else(|e| e.into_inner());
    match g.as_ref() {
        Some((b, a)) if *b == battle => a.clone(),
        _ => {
            let a = Arc::new(Mutex::new(Group::default()));
            *g = Some((battle, a.clone()));
            a
        }
    }
}

/// The Neoshadow's own work area: the hit state's mark (`wp+0x3C`), no reaction while sinking or after the
/// fourth flinch (`wp+0x40`), the diver's token (`wp+0x44`), the hidden one (`wp+0x48`), flinches in a row
/// (`wp+0x4C`), travelling underground (`wp+0x50`), a called one coming up (`wp+0x54`), leaping / diving /
/// rising slashing with its shield (`wp+0x58`); the port's: the group, the floor it last stood on, Sora on
/// the floor, the last hit's kind and push, whether the next damaging hit takes KH1's push only.
#[derive(Debug)]
struct Wp {
    mark: Mark,
    armour: bool,
    token: bool,
    special: bool,
    flinches: i32,
    travelling: bool,
    called: bool,
    shielded: bool,
    shield_on: bool,
    group: Arc<Mutex<Group>>,
    floor: Option<f32>,
    sora_grounded: bool,
    hit_kind: u8,
    hit_dir: Vec3,
    push_only: bool,
    started: bool,
    defeated: bool,
}

impl Default for Wp {
    fn default() -> Wp {
        Wp {
            mark: Mark::None,
            armour: false,
            token: false,
            special: false,
            flinches: 0,
            travelling: false,
            called: false,
            shielded: false,
            shield_on: false,
            group: Arc::new(Mutex::new(Group::default())),
            floor: None,
            sora_grounded: true,
            hit_kind: 0,
            hit_dir: Vec3::ZERO,
            push_only: false,
            started: false,
            defeated: false,
        }
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

fn grounded(cx: &Cx) -> bool {
    cx.borrow().w.grounded
}

/// The group block (locked for `f`).
fn group<T>(cx: &Cx, f: impl FnOnce(&mut Group) -> T) -> T {
    let a = cx.borrow().x.group.clone();
    let mut g = a.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g)
}

fn count(cx: &Cx) -> i32 {
    group(cx, |g| g.count)
}

/// `count <= k(name)`, `count >= k(name)`, `count == k(name)` (the thresholds are the script's pushes).
fn count_le(cx: &Cx, n: &str) -> bool {
    count(cx) as f32 - k(cx, n) <= 0.0
}
fn count_ge(cx: &Cx, n: &str) -> bool {
    count(cx) as f32 - k(cx, n) >= 0.0
}
fn count_is(cx: &Cx, n: &str) -> bool {
    count(cx) as f32 == k(cx, n)
}

/// A gathered dive is waiting for its divers (`+4 != 0 && +0xC == 0`): the rounds and the runs stop for it.
fn gathering(cx: &Cx) -> bool {
    group(cx, |g| g.ready != 0 && g.launched == 0)
}

/// A plain motion request (`sys 12` / `sys 13` without bit 24); `sys 172`'s clock restarts for a request
/// that plays anew.
fn play(cx: &Cx, r: u16, blend: f32) {
    {
        let mut io = cx.borrow_mut();
        if io.w.motion != Some(r) {
            io.clock = (Some(r), 0.0);
        }
    }
    push(cx, ECmd::Motion { request: r, blend });
}

/// `sys 13` of an attack's request (its rows made just before): the motion plays from its start, the
/// script keeps running.
fn attack_now(cx: &Cx, a: Attack, blend: f32) {
    cx.borrow_mut().clock = (Some(a.request()), 0.0);
    push(cx, ECmd::Attack { request: a.request(), attack: a, blend });
}

/// `sys 172` (`0x14029EFC0`): the motion has played to its end this cycle (KH1 loops a plain request at its
/// end: each cycle's end; the Bandit's rule).
fn done_now(cx: &Cx) -> bool {
    if !motion_done(cx) {
        return false;
    }
    let io = cx.borrow();
    let Some(r) = io.clock.0 else { return true };
    let len = motion_frames(cx, r);
    if len <= 0.0 {
        return true;
    }
    len <= io.w.step + io.clock.1 % len
}

/// `0x3FF4`: a yield, then until the motion has played to its end (`sys 172`).
async fn wait_done(cx: &Cx) {
    loop {
        frame().await;
        if done_now(cx) {
            break;
        }
    }
}

/// The rest of the behaviour thread after a state request (`misc8`): the supervisor replaces it at its
/// next run.
async fn halt() {
    loop {
        frame().await;
    }
}

/// `sys 11` toward a point: the facing at once.
fn face_now_to(cx: &Cx, t: Vec3) {
    let d = t - pos(cx);
    if d.x == 0.0 && d.z == 0.0 {
        return;
    }
    let y = d.x.atan2(d.z);
    let mut io = cx.borrow_mut();
    io.body.yaw = y;
    io.body.yaw_target = y;
    io.out.push(ECmd::Yaw(y));
}

/// `0x403C(actor, target)`: face the target now (none: nothing; KH1's `sys 66` debug print).
fn face_target_now(cx: &Cx) {
    if let Some(t) = target(cx) {
        face_now_to(cx, t);
    }
}

/// `sys 89`: the height above the floor (KH1 y down), from the floor it last stood on (Assumption: Re:CoM's
/// battle floors are flat; KH1 probes the floor under it, `0x1402999A0`).
fn height(cx: &Cx) -> f32 {
    let io = cx.borrow();
    io.x.floor.map_or(0.0, |f| f - io.w.pos.y)
}

/// The battle floor (KH1 y): the floor it last stood on, else where it is.
fn floor(cx: &Cx) -> f32 {
    let io = cx.borrow();
    io.x.floor.unwrap_or(io.w.pos.y)
}

/// One of KH1's room-29 points as a point of this battle: about the group's centre ([`ROOM_REF`] ↔ where
/// Sora started), its height from the battle floor (Assumption, owner question).
fn room_point(cx: &Cx, x: f32, y: f32, z: f32) -> Vec3 {
    let (rx, ry, rz) = slot(cx, ROOM_REF.0, ROOM_REF.1);
    let c = group(cx, |g| g.centre).unwrap_or_else(|| pos(cx));
    Vec3::new(c.x + (x - rx), floor(cx) + (y - ry), c.z + (z - rz))
}

/// A formation place's numbers (`0x2430`: layout, place).
fn slot(cx: &Cx, layout: usize, place: usize) -> (f32, f32, f32) {
    let n = |a: &str| k(cx, &format!("slot_{layout}_{place}_{a}"));
    (n("x"), n("y"), n("z"))
}

/// Sora's x in KH1's room-29 axes (the formation picks its layout by it, `0x245A`).
fn sora_room_x(cx: &Cx) -> f32 {
    let (rx, _, _) = slot(cx, ROOM_REF.0, ROOM_REF.1);
    let c = group(cx, |g| g.centre).unwrap_or_else(|| pos(cx));
    let s = cx.borrow().w.target.unwrap_or(c);
    rx + (s.x - c.x)
}

/// The shield row (`0x80`, mode 3) on, unless it is (`0x16BE`: `wp+0x90` null).
fn shield_on(cx: &Cx) {
    if !cx.borrow().x.shield_on {
        cx.borrow_mut().x.shield_on = true;
        push(cx, ECmd::Guard(Some((SHIELD, HELD))));
    }
}

/// `sys 213(wp+0x90, 0)`: the shield row ends.
fn shield_off(cx: &Cx) {
    if cx.borrow().x.shield_on {
        cx.borrow_mut().x.shield_on = false;
        push(cx, ECmd::Guard(None));
    }
}

/// The steering `+0x20` × `+0x2C` for this frame (the actor task zeroes `+0x2C` each frame).
fn steer(cx: &Cx, dir: Vec3, speed: f32) {
    cx.borrow_mut().body.walk = dir * speed;
}

/// `sys 10(v, deg)` on the facing: a direction turned by `deg`.
fn facing_turned(cx: &Cx, deg: f32) -> f32 {
    cx.borrow().body.yaw + deg.to_radians()
}

/// `0x3D0A(out, a, b)`: a random offset of length [a, b) at a random angle (the shared routine).
fn random_offset(cx: &Cx, a: f32, b: f32) -> Vec3 {
    crate::enemy::script::random_offset(cx, a, b)
}

/// `0x3D50(a, b)`: rand × (b − a) + a.
fn rand_range(cx: &Cx, a: f32, b: f32, what: &'static str) -> f32 {
    crate::enemy::script::rand_range(cx, a, b, what)
}

/// The battle's middle, once: where Sora stands when the first Neoshadow of the battle first sees him.
fn note_centre(cx: &Cx) {
    let s = cx.borrow().w.target;
    if let Some(s) = s {
        group(cx, |g| {
            if g.centre.is_none() {
                g.centre = Some(s);
            }
        });
    }
}

// ---------------------------------------------------------------------------------------------
// Sinking, coming up, underground.
// ---------------------------------------------------------------------------------------------

/// `0x5A98`: sink. On the ground; 216 (blend 8), 217 queued; `wp+0x40` = 1 (no reaction); for len(216) + 6
/// frames; from frame 40 + 6: once, KH1's flags 0x20 / 0x40 / 0x10 (unhittable), `wp+0x40` = 0, and the
/// height ramps 1 → 0 over 20 frames.
async fn sink(cx: &Cx) {
    wait_grounded(cx).await;
    play(cx, req::SINK, k(cx, "sink_blend"));
    push(cx, ECmd::Queue { request: req::UNDER, blend: 0.0 });
    cx.borrow_mut().x.armour = true;
    let extra = k(cx, "sink_extra");
    let total = motion_frames(cx, req::SINK) + extra;
    let hide = k(cx, "sink_hide") + extra;
    let ramp = k(cx, "sink_ramp");
    let (mut t, mut r, mut first) = (0.0f32, 0.0f32, true);
    while t - total < 0.0 {
        if t - hide > 0.0 {
            if first {
                first = false;
                cx.borrow_mut().x.armour = false;
                push(cx, ECmd::Hittable(false));
                push(cx, ECmd::Underground(true));
            }
            push(cx, ECmd::Height(1.0 - r / ramp));
            r += step(cx);
        }
        t += step(cx);
        frame().await;
    }
}

/// `0x596A`: come up. While in the air, nothing; 218 (blend 0), the idle queued (blend 8); the height ramps
/// 0 → 1 over len(218) − 2 frames; then height 1, `wp+0x50` = 0, flags 0x10 / 0x20 / 0x40 off (hittable);
/// while in the air.
async fn come_up(cx: &Cx) {
    wait_grounded(cx).await;
    play(cx, req::COME_UP, 0.0);
    push(cx, ECmd::Queue { request: req::IDLE, blend: k(cx, "idle_blend") });
    let len = motion_frames(cx, req::COME_UP) - k(cx, "come_up_trim");
    let mut t = 0.0;
    while t - len < 0.0 {
        t += step(cx);
        push(cx, ECmd::Height((t / len).min(1.0)));
        frame().await;
    }
    push(cx, ECmd::Height(1.0));
    cx.borrow_mut().x.travelling = false;
    push(cx, ECmd::Underground(false));
    push(cx, ECmd::Hittable(true));
    wait_grounded(cx).await;
}

/// `0x5652(goal, time, speed, near, motion, straight)`: underground travel. Each frame the motion (blend 8);
/// stops within `near` of the goal or when a gathered dive waits; straight (`straight`) at the goal, else
/// along its own facing turned 4° a frame (it circles; the turn's side flips every 60 frames, 120 once it
/// got stuck); from 30 frames on, a frame that moved under speed × 0.25 × step turns it toward KH1's room
/// spot ([`room_point`] `stuck_x..`) ±60°, 700–1000 away. Stops when another motion plays. Then the speed 0
/// and the idle (blend 8).
async fn travel(cx: &Cx, goal: Vec3, time: f32, speed: f32, near: f32, motion: u16, straight: bool) {
    let mut goal = goal;
    let mut left = time;
    let (mut since, mut flip_t) = (0.0f32, 0.0f32);
    let mut turn = k(cx, "weave_turn");
    let mut flip_at = k(cx, "weave_flip");
    let mut side = rand(cx, "weave side");
    if side - k(cx, "weave_flip_chance") > 0.0 {
        turn = -turn;
    }
    while left - 0.0 > 0.0 {
        let me = pos(cx);
        let d = goal - me;
        play(cx, motion, k(cx, "under_blend"));
        if d.length() - near < 0.0 || gathering(cx) {
            break;
        }
        let (u, _) = d.normalize_len();
        if !straight {
            let y = facing_turned(cx, turn);
            cx.borrow_mut().body.yaw_target = y;
            steer(cx, Vec3::from_yaw(y), speed);
        } else {
            cx.borrow_mut().body.yaw_target = u.x.atan2(u.z);
            steer(cx, u, speed);
        }
        left -= step(cx);
        since += step(cx);
        flip_t += step(cx);
        frame().await;
        if cx.borrow().w.motion != Some(motion) {
            break;
        }
        if flip_t - flip_at > 0.0 {
            side = if side - k(cx, "weave_flip_chance") > 0.0 { k(cx, "weave_flip_low") } else { k(cx, "weave_flip_high") };
            turn = -turn;
            flip_t = 0.0;
        }
        if since - k(cx, "stuck_after") > 0.0 && cx.borrow().w.moved - speed * k(cx, "stuck_factor") * step(cx) < 0.0 {
            flip_at = k(cx, "stuck_flip");
            let me = pos(cx);
            let spot = room_point(cx, k(cx, "stuck_x"), k(cx, "stuck_y"), k(cx, "stuck_z"));
            let to = spot - me;
            let a = rand_range(cx, k(cx, "stuck_angle_min"), k(cx, "stuck_angle_max"), "stuck turn");
            let y = to.x.atan2(to.z) + a.to_radians();
            let len = rand_range(cx, k(cx, "stuck_dist_min"), k(cx, "stuck_dist_max"), "stuck distance");
            goal = me + Vec3::from_yaw(y) * len;
            goal.y = me.y;
            since = 0.0;
        }
    }
    steer(cx, Vec3::ZERO, 0.0);
    play(cx, req::IDLE, k(cx, "under_blend"));
}

/// `0x552C(centre, lo, hi, straight)`: unless a gathered dive waits with more than `gather_max` of them:
/// `wp+0x50` = 1, sink, travel to a random point [lo, hi) about `centre` at its own height (180 frames,
/// speed 12, near 50, the underground motion), come up.
async fn sink_travel(cx: &Cx, centre: Vec3, lo: f32, hi: f32, straight: bool) {
    let ok = group(cx, |g| g.ready == 0) || count_le(cx, "gather_max");
    if !ok {
        return;
    }
    cx.borrow_mut().x.travelling = true;
    sink(cx).await;
    let mut p = centre + random_offset(cx, lo, hi);
    p.y = pos(cx).y;
    let (t, s, n) = (k(cx, "under_time"), k(cx, "under_speed"), k(cx, "under_near"));
    travel(cx, p, t, s, n, req::UNDER, straight).await;
    come_up(cx).await;
}

/// `0x55D2(at, 0, near)`: unless a gathered dive waits with more than `gather_max` of them: straight to a
/// random point within [0, near) of `at` (180 frames, speed 30, near 50, KH1's run motion 2 as it goes).
async fn under_to(cx: &Cx, at: Vec3) {
    let ok = group(cx, |g| g.ready == 0) || count_le(cx, "gather_max");
    if !ok {
        return;
    }
    let p = at + random_offset(cx, 0.0, k(cx, "rise_near_max"));
    let (t, s, n) = (k(cx, "under_time"), k(cx, "under_fast_speed"), k(cx, "under_near"));
    travel(cx, p, t, s, n, req::RUN, true).await;
}

// ---------------------------------------------------------------------------------------------
// Attacks.
// ---------------------------------------------------------------------------------------------

/// `0x1878`: the claw. Its rows `0x1D0` / `0x240` and 202 (blend 8), the idle queued (blend 8: played here
/// when 202 has played); for len(202) + 6 frames, turning to the target while more than 40 are left.
async fn claw(cx: &Cx) {
    attack_now(cx, Attack::Claw, k(cx, "claw_blend"));
    let len = motion_frames(cx, req::CLAW);
    let total = len + k(cx, "claw_extra");
    let mut t = total;
    let mut idle = false;
    while t - 0.0 > 0.0 {
        if t - k(cx, "claw_face") > 0.0 {
            face_target(cx);
        }
        if !idle && total - t >= len {
            idle = true;
            play(cx, req::IDLE, k(cx, "claw_blend"));
        }
        t -= step(cx);
        frame().await;
    }
}

/// `0x1568`: the rising slash. On the ground; sink; hidden; 20 frames; Sora on the floor: straight under
/// him (`sys 86` to his place), else underground to within 50 of him at its own height; height 1; the
/// shield (`wp+0x58` = 1); shown; facing the target now; its row `0x160` and 207 (blend 0); hittable; for
/// len(207) frames: 70 frames before its end the shield ends (`wp+0x58` = 0, the formation's effects
/// flag `+0x18` = 0); then the idle (blend 8).
async fn rise_slash(cx: &Cx) {
    wait_grounded(cx).await;
    sink(cx).await;
    push(cx, ECmd::Visible(false));
    wait_frames(cx, k(cx, "rise_wait")).await;
    let (sora, on_floor) = {
        let io = cx.borrow();
        (io.w.target, io.x.sora_grounded)
    };
    if let Some(s) = sora {
        if on_floor {
            let yaw = cx.borrow().body.yaw;
            warp(cx, s, yaw);
        } else {
            let mut at = s;
            at.y = pos(cx).y;
            under_to(cx, at).await;
        }
    }
    push(cx, ECmd::Height(1.0));
    rise_slash_out(cx).await;
}

/// The rising slash out of the floor (`0x16AA..0x1876` / `0x223C..0x242E`): the shield (`wp+0x58` = 1), shown,
/// facing the target now, the row `0x160` and 207 (blend 0), hittable; for len(207) frames, the shield ending
/// 70 frames before the end; the idle (blend 8) and a yield.
async fn rise_slash_out(cx: &Cx) {
    cx.borrow_mut().x.shielded = true;
    shield_on(cx);
    push(cx, ECmd::Visible(true));
    face_target_now(cx);
    attack_now(cx, Attack::RiseSlash, 0.0);
    push(cx, ECmd::Underground(false));
    push(cx, ECmd::Hittable(true));
    let mut t = motion_frames(cx, req::RISE_SLASH);
    while t - 0.0 > 0.0 {
        if t - k(cx, "shield_end") < 0.0 && cx.borrow().x.shielded {
            shield_off(cx);
            cx.borrow_mut().x.shielded = false;
            group(cx, |g| g.effects = 0);
        }
        t -= step(cx);
        frame().await;
    }
    play(cx, req::IDLE, k(cx, "idle_blend"));
    frame().await;
}

/// The leap before a dive (`0x1950` / `0x3036`): turning to the target (`0x1950` only), the crouch 203
/// (blend 8), 204 queued; for len(203) + 6 frames; in the last 14: on the floor, `wp+0x58` = 1, movement
/// state 7 with no velocity (`sys 169`), the shield; under 120 above the floor, rising at 20 a frame.
async fn leap_up(cx: &Cx, face: bool) {
    if face {
        face_target(cx);
    }
    play(cx, req::CROUCH, k(cx, "crouch_blend"));
    push(cx, ECmd::Queue { request: req::HOVER, blend: 0.0 });
    let mut t = motion_frames(cx, req::CROUCH) + k(cx, "crouch_extra");
    while t - 0.0 > 0.0 {
        if t - k(cx, "lift_window") < 0.0 {
            if grounded(cx) {
                cx.borrow_mut().x.shielded = true;
                cx.borrow_mut().body.air_fly(Vec3::ZERO);
                shield_on(cx);
            }
            if height(cx) - k(cx, "lift_height") < 0.0 {
                steer(cx, Vec3::new(0.0, -1.0, 0.0), k(cx, "lift_speed"));
            }
        }
        t -= step(cx);
        frame().await;
    }
}

/// `0x1950`: a dive on its own: the leap, until 204 has played, turning to the target, then the dive (its
/// own, no group words).
async fn single_dive(cx: &Cx) {
    leap_up(cx, true).await;
    wait_done(cx).await;
    face_target(cx);
    dive(cx, true).await;
}

/// `0x3036`: the gathered dive. The leap; `+4` += 1 (one more up); until the dive starts (`+0xC`), turning
/// to the target, it starts once all are up or there are at most `gather_max`; 30 frames; until it holds the
/// token (`+8`: one diver at a time), turning to the target; then its dive.
async fn gathered_dive(cx: &Cx) {
    leap_up(cx, false).await;
    group(cx, |g| g.ready += 1);
    loop {
        if group(cx, |g| g.launched != 0) {
            break;
        }
        face_target(cx);
        still(cx);
        let (ready, n) = group(cx, |g| (g.ready, g.count));
        if ready == n || count_le(cx, "gather_max") {
            group(cx, |g| g.launched = 1);
        }
        frame().await;
    }
    wait_frames(cx, k(cx, "launch_wait")).await;
    loop {
        face_target(cx);
        still(cx);
        if cx.borrow().x.token {
            break;
        }
        let took = group(cx, |g| {
            if g.token == 0 {
                g.token = 1;
                true
            } else {
                false
            }
        });
        if took {
            cx.borrow_mut().x.token = true;
        }
        frame().await;
    }
    dive(cx, false).await;
}

/// A frame the script waits in the air on its group (the gathered dive, one diver at a time): KH1 issues
/// nothing; the port says its facing so the host's quiet watchdog (`kh1-recom` `ce::watchdog::QUIET_LIMIT`)
/// doesn't take a long wait of KH1's own for a stuck AI (four divers before it: up to about 1600 frames).
fn still(cx: &Cx) {
    let y = cx.borrow().body.yaw;
    push(cx, ECmd::Yaw(y));
}

/// `0x3664`: where the dive aims: the target (80 above it when it stands on the floor or within 80 of it),
/// 1.4 times as far, 800 when that is under 500.
fn aim(cx: &Cx) -> Vec3 {
    let me = pos(cx);
    let Some(mut p) = cx.borrow().w.target else { return me };
    let fl = floor(cx);
    let sora_h = fl - p.y;
    if cx.borrow().x.sora_grounded || sora_h - k(cx, "aim_above") <= 0.0 {
        p.y -= k(cx, "aim_above");
    }
    let (u, len) = (p - me).normalize_len();
    let mut l = len * k(cx, "aim_factor");
    if l - k(cx, "aim_min") < 0.0 {
        l = k(cx, "aim_far");
    }
    me + u * l
}

/// `0x3326(single)`: the dive. The aim; 205 (blend 8); 8 frames; its row `0xF0` (mode 2); for len(205) × 9
/// frames: farther than 600 from the target, aim again; within 250 of the aim, stop; else turning to it,
/// flying at it at 80 (level under 80 above the floor); from 30 frames on, a frame that moved under 40 × step
/// stops it (or KH1's `+0x90` = 1: not identified, Assumption never). Then (gathered) the token back and one
/// fewer up; the flight's easing 0.4 from now on (stats `+0x14`); no speed; the shield ends; 206 (blend 0),
/// 6 queued, for len(206) − 4 frames; movement state 0 (`sys 88`); none up: the dive is over (`+0xC` = 0);
/// a yield; `wp+0x58` = 0; on the floor; `0x372C`.
async fn dive(cx: &Cx, single: bool) {
    let mut at = aim(cx);
    let mut since = 0.0f32;
    attack_now(cx, Attack::Dive, k(cx, "dive_blend"));
    // KH1 makes the row 8 frames into 205 (`0x335C`); the port's record is on 205 from its start (Assumption).
    wait_frames(cx, k(cx, "dive_row_after")).await;
    let mut left = motion_frames(cx, req::DIVE) * k(cx, "dive_cycles");
    while left - 0.0 > 0.0 {
        let me = pos(cx);
        let d = target(cx).map_or(0.0, |t| (t - me).length());
        if d - k(cx, "dive_reaim") > 0.0 {
            at = aim(cx);
        }
        let v = at - me;
        if v.length() - k(cx, "dive_near") < 0.0 {
            steer(cx, Vec3::ZERO, 0.0);
            break;
        }
        cx.borrow_mut().body.yaw_target = v.x.atan2(v.z);
        let (mut u, _) = v.normalize_len();
        if height(cx) - k(cx, "dive_level") < 0.0 {
            u.y = 0.0;
        }
        steer(cx, u, k(cx, "dive_speed"));
        left -= step(cx);
        since += step(cx);
        frame().await;
        if since - k(cx, "dive_stuck_after") > 0.0 && cx.borrow().w.moved - k(cx, "dive_stuck") * step(cx) < 0.0 {
            break;
        }
    }
    if !single {
        cx.borrow_mut().x.token = false;
        group(cx, |g| {
            g.token = 0;
            g.ready -= 1;
        });
    }
    crate::enemy::script::set_air_accel(cx, k(cx, "dive_accel_after"));
    steer(cx, Vec3::ZERO, 0.0);
    shield_off(cx);
    play(cx, req::LAND_DIVE, 0.0);
    push(cx, ECmd::Queue { request: req::AIR, blend: 0.0 });
    wait_frames(cx, motion_frames(cx, req::LAND_DIVE) - k(cx, "land_trim")).await;
    cx.borrow_mut().body.ground();
    group(cx, |g| {
        if g.ready == 0 {
            g.launched = 0;
        }
    });
    frame().await;
    cx.borrow_mut().x.shielded = false;
    wait_grounded(cx).await;
    // `0x372C`: higher than KH1's room height (5380, y down) it runs home for 60 frames: here, higher above the
    // battle floor than that height is above the room's floor (Assumption).
    let (_, ry, _) = slot(cx, ROOM_REF.0, ROOM_REF.1);
    if height(cx) - (ry - k(cx, "home_height")) > 0.0 {
        let home = cx.borrow().home;
        walk_to(cx, home, 1, k(cx, "home_time")).await;
    }
}

// ---------------------------------------------------------------------------------------------
// Moving on the floor.
// ---------------------------------------------------------------------------------------------

/// `0x5354(target, speed, near, time)`: strafe: 85° to one side of its facing (left 200 or right 201, blend
/// 8, half and half once), turning to the target, at `speed`; stops within `near`, when a gathered dive waits,
/// when another motion plays, or (from 30 frames before the end of… KH1: from the first 30) a frame that moved
/// under speed × 0.25 × step. Then the speed 0 and the idle (blend 8).
async fn strafe(cx: &Cx, speed: f32, near: f32, time: f32) {
    let side = rand(cx, "strafe side");
    let mut left = time;
    let after = time - k(cx, "strafe_stuck_after");
    let ang = k(cx, "strafe_angle");
    let blend = k(cx, "strafe_blend");
    let r = if side - k(cx, "strafe_side_chance") > 0.0 { req::STRAFE_L } else { req::STRAFE_R };
    while left - 0.0 > 0.0 {
        let d = target(cx).map_or(1.0e8, |t| (t - pos(cx)).length());
        if d - near < 0.0 || gathering(cx) {
            break;
        }
        let y = if r == req::STRAFE_L { facing_turned(cx, ang) } else { facing_turned(cx, -ang) };
        play(cx, r, blend);
        face_target(cx);
        steer(cx, Vec3::from_yaw(y), speed);
        left -= step(cx);
        frame().await;
        let m = cx.borrow().w.motion;
        if m != Some(req::STRAFE_L) && m != Some(req::STRAFE_R) {
            break;
        }
        if left - after < 0.0 && cx.borrow().w.moved - speed * k(cx, "strafe_stuck_factor") * step(cx) < 0.0 {
            break;
        }
    }
    steer(cx, Vec3::ZERO, 0.0);
    play(cx, req::IDLE, blend);
    frame().await;
}

/// `0x5BE8(time, near, speed, motion)`: run at the target (its place on the floor): the run motion (blend
/// 16), turning to it, along the facing at `speed`; stops within `near`, when a gathered dive waits, when
/// another motion plays, or from 30 frames on a frame that moved under 2 × step. Then the speed 0, the idle
/// (blend 16), 14 frames.
async fn run_at(cx: &Cx, time: f32, near: f32, speed: f32) {
    let mut left = time;
    let mut since = 0.0f32;
    let blend = k(cx, "run_blend");
    while left - 0.0 > 0.0 {
        let me = pos(cx);
        let Some(mut t) = target(cx) else { break };
        t.y = floor(cx);
        let v = t - me;
        let d = target(cx).map_or(1.0e8, |t| (t - me).length());
        if d - near < 0.0 || v.length() - near < 0.0 || gathering(cx) {
            break;
        }
        play(cx, req::RUN, blend);
        face_target(cx);
        let y = cx.borrow().body.yaw;
        steer(cx, Vec3::from_yaw(y), speed);
        left -= step(cx);
        since += step(cx);
        frame().await;
        if cx.borrow().w.motion != Some(req::RUN) {
            break;
        }
        if since - k(cx, "run_stuck_after") > 0.0 && cx.borrow().w.moved - k(cx, "run_stuck") * step(cx) < 0.0 {
            break;
        }
    }
    steer(cx, Vec3::ZERO, 0.0);
    play(cx, req::IDLE, blend);
    wait_frames(cx, k(cx, "run_end_wait")).await;
}

/// `0x4084(actor, run, 180, 20, 300)`: the shared approach (run, re-reading the target every 20 frames).
async fn approach(cx: &Cx) {
    let (t, r, d) = (k(cx, "approach_timeout"), k(cx, "approach_retarget"), k(cx, "approach_dist"));
    crate::enemy::script::approach(cx, 1, t, r, d).await;
}

// ---------------------------------------------------------------------------------------------
// The behaviours.
// ---------------------------------------------------------------------------------------------

/// `0xC3A(n)`: `n` rounds (a mid round counts two), on the floor first; a gathered dive waiting ends them.
/// - near (< 500): half the time the claw; sink and travel 600–1000 away (circling); with others, run to a
///   point 800–900 away (180 frames); alone, half the time the rising slash.
/// - mid (< 900): with two or fewer half the time, or alone: the rising slash or a dive (half and half);
///   else strafe (3 a frame, to 300, 150 frames) and the approach.
/// - far: more than 40 % or more than two of them: run at Sora (240 frames, to 150, at 12) and the claw; else
///   alone a dive, else sink, travel near Sora (200–300) and the rising slash.
async fn rounds(cx: &Cx, n: f32) {
    let mut i = 0.0f32;
    wait_grounded(cx).await;
    while i - n < 0.0 {
        if gathering(cx) {
            break;
        }
        let d = target(cx).map_or(1.0e8, |t| (t - pos(cx)).length());
        if d - k(cx, "near_range") < 0.0 {
            if rand(cx, "near claw") - k(cx, "near_claw_chance") > 0.0 {
                claw(cx).await;
            }
            let me = pos(cx);
            sink_travel(cx, me, k(cx, "near_away_min"), k(cx, "near_away_max"), false).await;
            if count(cx) as f32 - k(cx, "lone") > 0.0 {
                let p = pos(cx) + random_offset(cx, k(cx, "regroup_min"), k(cx, "regroup_max"));
                walk_to(cx, p, 1, k(cx, "regroup_time")).await;
            } else if rand(cx, "near rise") - k(cx, "near_rise_chance") > 0.0 {
                rise_slash(cx).await;
            }
            i += 1.0;
        } else if d - k(cx, "mid_range") < 0.0 {
            let r = rand(cx, "mid");
            if (count_le(cx, "pair") && r - k(cx, "mid_chance") > 0.0) || count_is(cx, "lone") {
                if rand(cx, "mid rise") - k(cx, "mid_rise_chance") > 0.0 {
                    rise_slash(cx).await;
                } else {
                    single_dive(cx).await;
                }
            } else {
                strafe(cx, k(cx, "strafe_speed"), k(cx, "strafe_near"), k(cx, "strafe_time")).await;
                approach(cx).await;
            }
            i += 2.0;
        } else {
            let r = rand(cx, "far");
            if r - k(cx, "far_run_chance") > 0.0 || count(cx) as f32 - k(cx, "pair") > 0.0 {
                run_at(cx, k(cx, "run_time"), k(cx, "run_near"), k(cx, "run_steer_speed")).await;
                claw(cx).await;
            } else if count_is(cx, "lone") {
                single_dive(cx).await;
            } else {
                if let Some(t) = target(cx) {
                    sink_travel(cx, t, k(cx, "far_near_min"), k(cx, "far_near_max"), false).await;
                }
                rise_slash(cx).await;
            }
            i += 1.0;
        }
        frame().await;
    }
}

/// `0x1FB0`: the formation (three or four of them). The places and the layout cleared; on the floor; sink;
/// hidden; 20 frames; while a formation still goes, wait (two or fewer: go on); one more sunk (`+0x10`);
/// until it goes (`+0x14`: all sunk, or two or fewer), then 6 frames; a free place of the layout (`0x2430`);
/// one fewer sunk (none: it no longer goes); height 1; 20 frames; the rising slash out of the place.
async fn formation(cx: &Cx) {
    cx.borrow_mut().x.flinches = 0;
    group(cx, |g| {
        g.slots = [false; 9];
        g.layout_chosen = false;
        g.layout = 0;
    });
    wait_grounded(cx).await;
    sink(cx).await;
    push(cx, ECmd::Visible(false));
    wait_frames(cx, k(cx, "form_wait")).await;
    while group(cx, |g| g.go != 0) {
        if count_le(cx, "pair") {
            break;
        }
        frame().await;
    }
    group(cx, |g| g.sunk += 1);
    loop {
        if group(cx, |g| g.go != 0) {
            break;
        }
        let (sunk, n) = group(cx, |g| (g.sunk, g.count));
        if sunk == n || count_le(cx, "pair") {
            group(cx, |g| g.go = 1);
        }
        still(cx);
        frame().await;
    }
    wait_frames(cx, k(cx, "form_warp_wait")).await;
    take_place(cx).await;
    group(cx, |g| {
        g.effects = 1;
        g.sunk -= 1;
        if g.sunk == 0 {
            g.go = 0;
        }
    });
    push(cx, ECmd::Height(1.0));
    wait_frames(cx, k(cx, "form_rise_after")).await;
    rise_slash_out(cx).await;
}

/// `0x2430`: the layout (the first to come picks it by Sora's x in KH1's room: under 8250 the first, over
/// 9200 the second, else the third); then each frame a draw in [1, 10): the first free place at or after the
/// draw's (place k for a draw under k + 2) is taken and it moves there (`sys 86`, facing 0); else a yield and
/// another draw.
async fn take_place(cx: &Cx) {
    if !group(cx, |g| g.layout_chosen) {
        let x = sora_room_x(cx);
        let l = if x - k(cx, "layout_low") < 0.0 {
            0
        } else if x - k(cx, "layout_high") > 0.0 {
            1
        } else {
            2
        };
        group(cx, |g| {
            g.layout_chosen = true;
            g.layout = l;
        });
    }
    let l = group(cx, |g| g.layout);
    loop {
        let r = rand_range(cx, k(cx, "slot_draw_min"), k(cx, "slot_draw_max"), "formation place");
        let pick = group(cx, |g| (0..9).find(|&p| r - (p as f32 + 2.0) < 0.0 && !g.slots[p]));
        if let Some(p) = pick {
            group(cx, |g| g.slots[p] = true);
            let (x, y, z) = slot(cx, l, p);
            let to = room_point(cx, x, y, z);
            warp(cx, to, 0.0);
            return;
        }
        frame().await;
    }
}

/// `0xBA2`: [120, 180) frames, then the grab (KH1) or its miss.
async fn special_round(cx: &Cx) {
    let n = rand_range(cx, k(cx, "special_wait_min"), k(cx, "special_wait_max"), "hidden wait");
    wait_frames(cx, n).await;
    grab_or_miss(cx).await;
}

/// `0x1AF6`: the hidden one's ambush. While Sora is off the floor (unless it is alone, 13 m away, or 180
/// frames passed), wait; 20 frames under Sora, facing his way; then KH1 grabs him from below when he stands on
/// the floor and can be hit: he plays its motion 200 from its own `.mset` (`sys 113`, `0x1402BA8C0`, request
/// `0x10000C8`) while it plays 208 and the others dive at him (`+0x4C`). **Not built** (owner question): the
/// port always takes KH1's other branch, the miss: at his feet, facing his way, shown, 209 until it has played,
/// hidden again.
async fn grab_or_miss(cx: &Cx) {
    let mut waited = 0.0f32;
    loop {
        let off_floor = !cx.borrow().x.sora_grounded;
        if !off_floor {
            break;
        }
        let d = target(cx).map_or(1.0e8, |t| (t - pos(cx)).length());
        if count_le(cx, "lone") || d - k(cx, "grab_range") >= 0.0 || waited - k(cx, "grab_wait") > 0.0 {
            break;
        }
        waited += step(cx);
        still(cx);
        frame().await;
    }
    let mut t = k(cx, "grab_follow");
    while t - 0.0 > 0.0 {
        follow_sora(cx);
        t -= step(cx);
        frame().await;
    }
    follow_sora(cx);
    push(cx, ECmd::Note("Neoshadow grab from below not built (owner question): KH1's miss (209) instead"));
    let sora = cx.borrow().w.target;
    if let Some(s) = sora {
        let mut p = s;
        p.y = floor(cx) + k(cx, "miss_depth");
        let yaw = cx.borrow().body.yaw;
        warp(cx, p, yaw);
    }
    push(cx, ECmd::Visible(true));
    play(cx, req::MISS, 0.0);
    wait_done(cx).await;
    push(cx, ECmd::Visible(false));
}

/// `0x1BF2..0x1C1C`: under Sora (`sys 86` to his place), facing his way (`sys 11` with his facing: the
/// host gives his place only; Assumption: facing him from where it was).
fn follow_sora(cx: &Cx) {
    let s = cx.borrow().w.target;
    if let Some(s) = s {
        let me = pos(cx);
        let d = s - me;
        let yaw = if d.x == 0.0 && d.z == 0.0 { cx.borrow().body.yaw } else { d.x.atan2(d.z) };
        warp(cx, s, yaw);
    }
}

/// `0x11FA`: the call. `+0x50` = 1; 210 (blend 8), the idle queued; for len(210) + 6 frames (in the last 80
/// KH1 wakes the room's sleeping Neoshadows 600..606 around it: Re:CoM's battles have none).
async fn call(cx: &Cx) {
    group(cx, |g| g.calling = true);
    play(cx, req::CALL, k(cx, "call_blend"));
    push(cx, ECmd::Queue { request: req::IDLE, blend: 0.0 });
    let mut t = motion_frames(cx, req::CALL) + k(cx, "call_extra");
    let mut noted = false;
    while t - 0.0 > 0.0 {
        if !noted && t - k(cx, "call_window") < 0.0 {
            noted = true;
            push(cx, ECmd::Note("KH1 wakes the room's sleeping Neoshadows 600..606 here: none in a Re:CoM battle"));
        }
        t -= step(cx);
        frame().await;
    }
}

/// `0x8FA`: alone. The hidden one first: the group's words cleared, no longer hidden (`wp+0x48` = 0), to KH1's
/// room spot (`special_x..`, `sys 86`, facing 0), 30 frames, shown, come up, the call. Then while alone: with no
/// called one still coming, underground to KH1's room spot (`lone_x..`, straight) and the call; five rounds.
async fn lone(cx: &Cx) {
    if cx.borrow().x.special {
        group(cx, |g| {
            let (count, centre, calling, coming, last) = (g.count, g.centre, g.calling, g.coming, g.last_died);
            *g = Group { count, centre, calling, coming, last_died: last, ..Group::default() };
        });
        cx.borrow_mut().x.special = false;
        let to = room_point(cx, k(cx, "special_x"), k(cx, "special_y"), k(cx, "special_z"));
        warp(cx, to, 0.0);
        wait_frames(cx, k(cx, "special_rise_wait")).await;
        push(cx, ECmd::Visible(true));
        come_up(cx).await;
        call(cx).await;
    }
    loop {
        if group(cx, |g| g.coming == 0) {
            let spot = room_point(cx, k(cx, "lone_x"), k(cx, "lone_y"), k(cx, "lone_z"));
            sink_travel(cx, spot, 0.0, 0.0, true).await;
            call(cx).await;
        }
        rounds(cx, k(cx, "lone_rounds")).await;
        if !count_is(cx, "lone") {
            return;
        }
        frame().await;
    }
}

/// The engaged behaviour (`0x6E4` → `0x724`), each pass by the group's count:
/// - five or more (`0xBCC`): unless a gathered dive is on, the gathered dive (on the floor after it, turning
///   to the target); still five or more: three rounds;
/// - three or four: join a gathered dive that waits, else the formation;
/// - two: the first to come goes underground for good (`+0x48`, `wp+0x48`): sink, hidden; it ambushes Sora
///   ([`special_round`]); the other: no gathering, three rounds;
/// - fewer: [`lone`].
async fn engaged(cx: Cx) {
    set_target(&cx, true);
    loop {
        if count_ge(&cx, "crowd") {
            if group(&cx, |g| g.launched == 0) {
                gathered_dive(&cx).await;
                wait_grounded(&cx).await;
                face_target(&cx);
            }
            if count_ge(&cx, "crowd") {
                rounds(&cx, k(&cx, "rounds")).await;
            }
        } else if count_ge(&cx, "group_min") {
            if gathering(&cx) {
                gathered_dive(&cx).await;
                wait_grounded(&cx).await;
            } else {
                formation(&cx).await;
            }
        } else if count_is(&cx, "pair") {
            let first = group(&cx, |g| {
                if !g.special {
                    g.special = true;
                    true
                } else {
                    false
                }
            });
            if first {
                cx.borrow_mut().x.special = true;
                sink(&cx).await;
                push(&cx, ECmd::Visible(false));
                push(&cx, ECmd::Height(1.0));
            }
            if cx.borrow().x.special {
                special_round(&cx).await;
            } else {
                group(&cx, |g| g.ready = 0);
                rounds(&cx, k(&cx, "rounds")).await;
            }
        } else {
            lone(&cx).await;
        }
        frame().await;
        if count_le(&cx, "gather_max") {
            set_target(&cx, true);
        }
    }
}

/// The idle behaviour (`0xF34`): the search (Sora within 50 m; at most four of them: Sora anyway), facing it
/// now, engaged.
async fn idle(cx: Cx) {
    set_target(&cx, true);
    face_target_now(&cx);
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// State 3 (`0x680`, no script asks for it): the search, idle.
async fn state3(cx: Cx) {
    set_target(&cx, true);
    set_state(&cx, st::IDLE);
    halt().await;
}

/// State 4 (`0x5C6`): with the last counted one alive, on the floor, a dive, engaged; else (a called one when
/// the last counted one died) it vanishes: unhittable, a 30-frame fade, gone (KH1 frees it; the port leaves it
/// hidden and unhittable).
async fn dive_state(cx: Cx) {
    if !group(&cx, |g| g.last_died) {
        wait_grounded(&cx).await;
        single_dive(&cx).await;
        set_state(&cx, st::ENGAGED);
        halt().await;
    } else {
        cx.borrow_mut().x.called = false;
        push(&cx, ECmd::Hittable(false));
        wait_frames(&cx, k(&cx, "vanish_fade")).await;
        push(&cx, ECmd::Visible(false));
        push(&cx, ECmd::Note("a called Neoshadow vanishes (the last counted one died)"));
        halt().await;
    }
}

/// The hit state (`0x3DE`): after a counter mark (a knock-back or launch, or the fourth flinch in a row): with
/// other than three or four of them and no gathering, two or fewer half the time the rising slash, else sink and
/// travel 600–1000 away; no longer armoured (`wp+0x40` = 0). Else, no gathering: five or more the claw, three or
/// four turning to the target, two or fewer the rising slash. Then the search (Sora) and engaged.
async fn hit(cx: Cx) {
    let mark = std::mem::take(&mut cx.borrow_mut().x.mark);
    if mark == Mark::Counter {
        let n = count(&cx) as f32;
        let three_or_four = n == k(&cx, "group_min") || n == k(&cx, "gather_max");
        if !three_or_four && group(&cx, |g| g.ready == 0) {
            if rand(&cx, "counter") - k(&cx, "counter_rise_chance") > 0.0 && count_le(&cx, "pair") {
                rise_slash(&cx).await;
            } else {
                let me = pos(&cx);
                sink_travel(&cx, me, k(&cx, "counter_away_min"), k(&cx, "counter_away_max"), false).await;
            }
            cx.borrow_mut().x.armour = false;
        }
    } else if group(&cx, |g| g.ready == 0) {
        if count_ge(&cx, "crowd") {
            claw(&cx).await;
        } else if count_ge(&cx, "group_min") {
            face_target(&cx);
        } else if count_le(&cx, "pair") {
            rise_slash(&cx).await;
        }
    }
    set_target(&cx, true);
    set_state(&cx, st::ENGAGED);
    halt().await;
}

/// `0xFC8`: a called one comes up: 30 frames; while in the air; shown, hittable; facing Sora now; 211 until it
/// has played; 212 for 4 times its length; no longer coming (`+0x54` = 0 when it was set); one more counted;
/// action 213 with the idle queued; idle.
async fn called(cx: Cx) {
    // Its init hid it (KH1's flags at `0x4890..0x48B0`); Re:CoM's state-0 end turns its bodies on again after our
    // first frame (`0x140190400`): hidden and unhittable again here.
    push(&cx, ECmd::Hittable(false));
    push(&cx, ECmd::Visible(false));
    wait_frames(&cx, k(&cx, "called_wait")).await;
    cx.borrow_mut().x.called = true;
    // `0xFDC`: "while in the air and 6 plays and 7 plays and above the floor": never (a motion can't be both;
    // KH1's own test), so no wait. KH1 clears its flag 0x40 here (`0x1062`) but its hit handler drops every hit
    // while `wp+0x54` is set, before the damage (`0x49B2`): Re:CoM can't take a hit without its damage, so the
    // port keeps it unhittable until it is counted (Assumption).
    push(&cx, ECmd::Visible(true));
    let s = cx.borrow().w.target;
    if let Some(s) = s {
        face_now_to(&cx, s);
    }
    play(&cx, req::CALLED_RISE, 0.0);
    wait_done(&cx).await;
    play(&cx, req::CALLED_HOLD, 0.0);
    frame().await;
    wait_frames(&cx, motion_frames(&cx, req::CALLED_HOLD) * k(&cx, "called_hold")).await;
    cx.borrow_mut().x.called = false;
    group(&cx, |g| {
        if g.coming != 0 {
            g.coming = 0;
        }
        g.count += 1;
    });
    push(&cx, ECmd::Hittable(true));
    push(&cx, ECmd::Action { request: req::CALLED_END, attack: None, then: req::IDLE, blend: 0.0 });
    frame().await;
    set_state(&cx, st::IDLE);
    halt().await;
}

/// The supervisor (`0x443E`): the init (`0x46F8`), a yield; a called one (`+0x50` at its init) comes up, the
/// others go idle; then each frame a state request restarts the behaviour thread, a yield, and while it travels
/// underground nothing else. KH1's two other checks (`0x453C`: KH1's grab holding Sora sends a free Neoshadow to
/// state 4; `0x468E`: the last counted one died, a called one coming up vanishes) follow.
async fn supervisor(cx: Cx) {
    frame().await;
    let calling = cx.borrow().x.called;
    set_state(&cx, if calling { st::NONE } else { st::IDLE });
    if calling {
        // `0x446E` restarts the behaviour at `0xFC8` (`fb8`): the supervisor's own state object stays.
        cx.borrow_mut().state = (st::NONE, false);
        push(&cx, ECmd::State(st::NONE));
    }
    loop {
        if let Some(s) = take_state(&cx) {
            push(&cx, ECmd::State(s));
        }
        frame().await;
        while cx.borrow().x.travelling {
            frame().await;
        }
        let (grab, last) = group(&cx, |g| (g.grab, g.last_died));
        let busy = matches!(cx.borrow().w.motion, Some(req::CROUCH | req::HOVER | req::DIVE | req::LAND_DIVE | req::SINK | req::COME_UP | req::RISE_SLASH | req::UNDER));
        if grab && !cx.borrow().x.special && !busy && grounded(&cx) {
            group(&cx, |g| g.grab = false);
            set_state(&cx, st::DIVE);
        } else if last && cx.borrow().x.called {
            set_state(&cx, st::DIVE);
        }
    }
}

/// The Neoshadow's AI for one enemy.
pub struct Ai {
    io: Cx,
    main: Option<Task>,
    behaviour: Option<Task>,
}

impl Ai {
    /// A new AI. `seed`: its random stream (Assumption, as every port: per-enemy seeds). `frames`: KH1 motion
    /// lengths by request. Its group is its own until [`Behaviour::set_battle`] (or [`Ai::with_group`]).
    pub fn new(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32) -> Ai {
        let io = crate::enemy::script::Core::new(c, frames, seed, yaw, crate::enemy::MOVE_BLEND, Wp::default());
        Ai { io: Rc::new(RefCell::new(io)), main: None, behaviour: None }
    }

    /// The same, sharing `group` with the battle's other Neoshadows (tests).
    pub fn with_group(c: Consts, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, group: Arc<Mutex<Group>>) -> Ai {
        let ai = Ai::new(c, frames, seed, yaw);
        ai.io.borrow_mut().x.group = group;
        ai
    }

    pub fn consts(&self) -> Consts {
        self.io.borrow().c.clone()
    }

    /// The group block (a copy).
    pub fn group(&self) -> Group {
        group(&self.io, |g| g.clone())
    }

    /// Whether it is the hidden one (two of them).
    pub fn special(&self) -> bool {
        self.io.borrow().x.special
    }

    /// `0x46F8`, the init's group part: the shared words `+4..+0x4C` cleared; unless one is calling, one more
    /// counted (`+0x50`, `+0x54` cleared); else a called one: unhittable, hidden, one more coming; the last
    /// counted one alive again (`+0x58` = 0). Home is where it stands.
    fn init(&mut self, w: World) {
        let mut io = self.io.borrow_mut();
        io.home = w.pos;
        io.state = (st::NONE, false);
        let calling = {
            let a = io.x.group.clone();
            let mut g = a.lock().unwrap_or_else(|e| e.into_inner());
            let (count, centre, calling, coming) = (g.count, g.centre, g.calling, g.coming);
            *g = Group { count, centre, calling, coming, ..Group::default() };
            if !calling {
                g.count += 1;
                g.calling = false;
                g.coming = 0;
            } else {
                g.coming += 1;
            }
            g.last_died = false;
            calling
        };
        io.x.called = calling;
        io.x.started = true;
        if calling {
            io.out.push(ECmd::Hittable(false));
            io.out.push(ECmd::Visible(false));
        }
    }

    /// One AI frame (the host calls it only while no action runs, or in a steered attack's state).
    pub fn frame(&mut self, w: World) -> Vec<Cmd> {
        {
            let mut io = self.io.borrow_mut();
            io.w = w;
            tick_clock(&mut io);
            if w.grounded {
                io.x.floor = Some(w.pos.y);
            }
        }
        if !self.io.borrow().x.started {
            self.init(w);
            self.main = Some(Box::pin(supervisor(self.io.clone())));
        }
        note_centre(&self.io);
        poll(&mut self.main);
        let restart = self.io.borrow().out.iter().rev().find_map(|c| match c {
            ECmd::State(s) => Some(*s),
            _ => None,
        });
        if let Some(s) = restart {
            let cx = self.io.clone();
            self.behaviour = Some(match s {
                st::NONE => Box::pin(called(cx)),
                st::IDLE => Box::pin(idle(cx)),
                st::ENGAGED => Box::pin(engaged(cx)),
                st::STATE3 => Box::pin(state3(cx)),
                st::DIVE => Box::pin(dive_state(cx)),
                _ => Box::pin(hit(cx)),
            });
        }
        poll(&mut self.behaviour);
        std::mem::take(&mut self.io.borrow_mut().out)
    }

    /// The hit handler's early outs (`0x498C`): a called one coming up (`wp+0x54`), the hidden one
    /// (`wp+0x48`), its shield on (`wp+0x58`), the call (210) or sinking's first frames / after the fourth
    /// flinch (`wp+0x40`): no reaction, no state; 213 playing: KH1's push slide only (`sys 42`).
    fn no_reaction(&self, playing: Option<u16>) -> Option<bool> {
        let io = self.io.borrow();
        if io.x.called || io.x.special || io.x.shielded || playing == Some(req::CALL) || io.x.armour {
            return Some(false);
        }
        (playing == Some(req::CALLED_END)).then_some(true)
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
    /// The hit handler (`0x498C`) for a hit that takes a reaction (the early outs take none:
    /// [`Behaviour::armoured`]): a clash (kinds 6 / 7) or a guard repel: the early outs, else the hit state. A
    /// damaging hit: a knock-back or launch (kinds 4 / 5) marks the counter (`wp+0x3C` = 100) and clears the
    /// flinch run; a flinch (kind 3) counts one more in a row, the fourth marks the counter and no reaction until
    /// it runs (`wp+0x40` = 1); the hit state. (KH1's hit types 8..11 and 1 are other attackers'; Sora's are
    /// type 0, as every port's. A killing hit is [`Behaviour::on_defeat`].)
    fn on_hit(&self, news: HitNews) {
        let playing = self.io.borrow().w.motion;
        if self.no_reaction(playing).is_some() {
            return;
        }
        if news == HitNews::Damage {
            let kind = self.io.borrow().x.hit_kind;
            let mut io = self.io.borrow_mut();
            match kind {
                4 | 5 => {
                    io.x.mark = Mark::Counter;
                    io.x.flinches = 0;
                }
                3 => {
                    io.x.flinches += 1;
                    if io.x.flinches >= 4 {
                        io.x.armour = true;
                        io.x.mark = Mark::Counter;
                        io.x.flinches = 0;
                    }
                }
                _ => {}
            }
        }
        set_state(&self.io, st::HIT);
    }
    /// The hit handler's early outs: no reaction (Re:CoM's super armour for the hit); the next
    /// [`Behaviour::on_armoured_hit`] gets KH1's push slide only for 213 (`sys 42`).
    fn armoured(&self, playing: Option<u16>, _launch_base: u16) -> bool {
        match self.no_reaction(playing) {
            Some(push) => {
                self.io.borrow_mut().x.push_only = push;
                true
            }
            None => false,
        }
    }
    fn on_armoured_hit(&self, dir: Vec3) {
        let mut io = self.io.borrow_mut();
        if std::mem::take(&mut io.x.push_only) {
            use crate::enemy::Engine;
            let p = io.c.flinch_push() * io.c.weight();
            io.body.push(dir, p);
        }
    }
    /// KH1's death (`0x4B86`, `0x4E1C`): one fewer counted; the last: the called ones vanish (`+0x58`); the
    /// drops and EXP are Re:CoM's; it turns to Sora (`0x5304`) for its death motion.
    fn on_defeat(&self) {
        let first = !std::mem::replace(&mut self.io.borrow_mut().x.defeated, true);
        if first {
            group(&self.io, |g| {
                g.count -= 1;
                if g.count == 0 {
                    g.last_died = true;
                }
            });
        }
        // The host drops its guard row at the defeat.
        self.io.borrow_mut().x.shield_on = false;
        let s = self.io.borrow().w.target;
        if let Some(s) = s {
            face_now_to(&self.io, s);
        }
    }
    fn death_motion(&self) -> Option<u16> {
        Some(req::DEATH)
    }
    fn risen(&self) -> bool {
        // KH1's room Neoshadow has no appearance (`0x46F8`: only a called one comes up, `0xFC8`, a state).
        true
    }
    fn skip_rise(&self) {}
    fn set_hit_kind(&self, kind: u8) {
        self.io.borrow_mut().x.hit_kind = kind;
    }
    fn set_hit_dir(&self, dir: Vec3) {
        self.io.borrow_mut().x.hit_dir = dir;
    }
    fn set_target_grounded(&self, on_ground: bool) {
        self.io.borrow_mut().x.sora_grounded = on_ground;
    }
    fn set_battle(&self, battle: u32) {
        let g = group_for_battle(battle);
        self.io.borrow_mut().x.group = g;
    }
    fn state_name(&self, s: u8) -> &'static str {
        ["called", "idle", "engaged", "state 3", "dive", "hit"].get(s as usize).copied().unwrap_or("?")
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
        // The player's `ex_2420.bd` / `btltbl.bin` / KH1 exe numbers as the asset step reads them.
        let v: &[(&str, f32)] = &[
            ("walk_speed", 2.0), ("run_speed", 12.0), ("crowd", 5.0), ("group_min", 3.0), ("pair", 2.0), ("gather_max", 4.0),
            ("lone", 1.0), ("search_range", 5000.0), ("counter_rise_chance", 0.5), ("counter_away_min", 600.0), ("counter_away_max", 1000.0), ("rounds", 3.0),
            ("lone_rounds", 5.0), ("near_range", 500.0), ("near_claw_chance", 0.5), ("near_away_min", 600.0), ("near_away_max", 1000.0), ("regroup_min", 800.0),
            ("regroup_max", 900.0), ("regroup_time", 180.0), ("near_rise_chance", 0.5), ("mid_range", 900.0), ("mid_chance", 0.5), ("mid_rise_chance", 0.5),
            ("strafe_speed", 3.0), ("strafe_near", 300.0), ("strafe_time", 150.0), ("approach_timeout", 180.0), ("approach_retarget", 20.0), ("approach_dist", 300.0),
            ("far_run_chance", 0.4000000059604645), ("run_time", 240.0), ("run_near", 150.0), ("run_steer_speed", 12.0), ("far_near_min", 200.0), ("far_near_max", 300.0),
            ("claw_blend", 8.0), ("claw_extra", 6.0), ("claw_face", 40.0), ("rise_wait", 20.0), ("rise_near_max", 50.0), ("shield_end", 70.0),
            ("idle_blend", 8.0), ("under_time", 180.0), ("under_speed", 12.0), ("under_fast_speed", 30.0), ("under_near", 50.0), ("weave_turn", 4.0),
            ("weave_flip", 60.0), ("weave_flip_chance", 0.5), ("weave_flip_low", 0.4000000059604645), ("weave_flip_high", 0.6000000238418579), ("stuck_after", 30.0), ("stuck_factor", 0.25),
            ("stuck_flip", 120.0), ("stuck_angle_min", -60.0), ("stuck_angle_max", 60.0), ("stuck_dist_min", 700.0), ("stuck_dist_max", 1000.0), ("under_blend", 8.0),
            ("sink_blend", 8.0), ("sink_extra", 6.0), ("sink_hide", 40.0), ("sink_ramp", 20.0), ("come_up_trim", 2.0), ("run_blend", 16.0),
            ("run_stuck_after", 30.0), ("run_stuck", 2.0), ("run_end_wait", 14.0), ("strafe_stuck_after", 30.0), ("strafe_side_chance", 0.5), ("strafe_angle", 85.0),
            ("strafe_blend", 8.0), ("strafe_stuck_factor", 0.25), ("crouch_blend", 8.0), ("crouch_extra", 6.0), ("lift_window", 14.0), ("lift_height", 120.0),
            ("lift_speed", 20.0), ("launch_wait", 30.0), ("dive_blend", 8.0), ("dive_row_after", 8.0), ("dive_cycles", 9.0), ("dive_reaim", 600.0),
            ("dive_near", 250.0), ("dive_level", 80.0), ("dive_speed", 80.0), ("dive_stuck_after", 30.0), ("dive_stuck", 40.0), ("dive_accel_after", 0.4000000059604645),
            ("land_trim", 4.0), ("aim_above", 80.0), ("aim_factor", 1.399999976158142), ("aim_min", 500.0), ("aim_far", 800.0), ("home_height", 5380.0),
            ("home_time", 60.0), ("form_wait", 20.0), ("form_warp_wait", 6.0), ("form_rise_after", 20.0), ("layout_low", 8250.0), ("layout_high", 9200.0),
            ("slot_draw_min", 1.0), ("slot_draw_max", 10.0), ("special_wait_min", 120.0), ("special_wait_max", 180.0), ("grab_follow", 20.0), ("grab_range", 1300.0),
            ("grab_wait", 180.0), ("miss_depth", 8.0), ("special_x", 9000.0), ("special_y", 5300.0), ("special_z", 100.0), ("special_rise_wait", 30.0),
            ("lone_x", 8900.0), ("lone_y", 5300.0), ("lone_z", 100.0), ("call_blend", 8.0), ("call_extra", 6.0), ("call_window", 80.0),
            ("called_wait", 30.0), ("called_hold", 4.0), ("vanish_fade", 30.0), ("stuck_x", 8700.0), ("stuck_y", 5300.0), ("stuck_z", 100.0),
            ("slot_0_0_x", 8100.0), ("slot_0_0_y", 5460.0), ("slot_0_0_z", 200.0), ("slot_0_1_x", 7800.0), ("slot_0_1_y", 5460.0), ("slot_0_1_z", 200.0),
            ("slot_0_2_x", 8100.0), ("slot_0_2_y", 5460.0), ("slot_0_2_z", -100.0), ("slot_0_3_x", 7800.0), ("slot_0_3_y", 5460.0), ("slot_0_3_z", -100.0),
            ("slot_0_4_x", 8100.0), ("slot_0_4_y", 5460.0), ("slot_0_4_z", 500.0), ("slot_0_5_x", 7800.0), ("slot_0_5_y", 5460.0), ("slot_0_5_z", 500.0),
            ("slot_0_6_x", 8100.0), ("slot_0_6_y", 5470.0), ("slot_0_6_z", -400.0), ("slot_0_7_x", 7800.0), ("slot_0_7_y", 5470.0), ("slot_0_7_z", -400.0),
            ("slot_0_8_x", 7500.0), ("slot_0_8_y", 5474.0), ("slot_0_8_z", 50.0), ("slot_1_0_x", 9650.0), ("slot_1_0_y", 5470.0), ("slot_1_0_z", 300.0),
            ("slot_1_1_x", 9300.0), ("slot_1_1_y", 5470.0), ("slot_1_1_z", 300.0), ("slot_1_2_x", 9650.0), ("slot_1_2_y", 5470.0), ("slot_1_2_z", 0.0),
            ("slot_1_3_x", 9300.0), ("slot_1_3_y", 5470.0), ("slot_1_3_z", 0.0), ("slot_1_4_x", 9650.0), ("slot_1_4_y", 5470.0), ("slot_1_4_z", 600.0),
            ("slot_1_5_x", 9300.0), ("slot_1_5_y", 5467.0), ("slot_1_5_z", 600.0), ("slot_1_6_x", 9650.0), ("slot_1_6_y", 5470.0), ("slot_1_6_z", -300.0),
            ("slot_1_7_x", 9300.0), ("slot_1_7_y", 5467.0), ("slot_1_7_z", -300.0), ("slot_1_8_x", 10000.0), ("slot_1_8_y", 5474.0), ("slot_1_8_z", 150.0),
            ("slot_2_0_x", 8800.0), ("slot_2_0_y", 5480.0), ("slot_2_0_z", 100.0), ("slot_2_1_x", 8800.0), ("slot_2_1_y", 5467.0), ("slot_2_1_z", 550.0),
            ("slot_2_2_x", 8800.0), ("slot_2_2_y", 5480.0), ("slot_2_2_z", -350.0), ("slot_2_3_x", 9200.0), ("slot_2_3_y", 5480.0), ("slot_2_3_z", 100.0),
            ("slot_2_4_x", 9200.0), ("slot_2_4_y", 5466.0), ("slot_2_4_z", 550.0), ("slot_2_5_x", 9200.0), ("slot_2_5_y", 5470.0), ("slot_2_5_z", -350.0),
            ("slot_2_6_x", 8400.0), ("slot_2_6_y", 5465.0), ("slot_2_6_z", 100.0), ("slot_2_7_x", 8400.0), ("slot_2_7_y", 5475.0), ("slot_2_7_z", 550.0),
            ("slot_2_8_x", 8400.0), ("slot_2_8_y", 5465.0), ("slot_2_8_z", -350.0),
            ("air_accel", 0.1), ("air_drag", 0.95), ("fall_cap", 20.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("launch_min", 20.0),
            ("launch_max", 35.0), ("launch_hop", 1.3499999), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("weight", 1.0),
            ("attack_blend", 8.0),
        ];
        let mut v: Vec<(String, f32)> = v.iter().map(|(k, x)| (k.to_string(), *x)).collect();
        v.push(("gravity".into(), crate::params::GRAVITY));
        v.push(("turn_rate".into(), std::f32::consts::PI / 16.0));
        Table::new(v, NAMES).unwrap()
    }

    /// The player's `xa_ex_2420.mset` lengths by request.
    fn frames() -> Vec<(u16, f32)> {
        vec![
            (0, 280.0), (1, 36.0), (2, 36.0), (6, 40.0), (7, 48.0), (66, 110.0), (67, 120.0), (68, 108.0), (72, 40.0), (73, 52.0), (74, 40.0), (75, 52.0),
            (76, 58.0), (77, 58.0), (78, 36.0), (79, 36.0), (80, 36.0), (81, 36.0), (200, 72.0), (201, 72.0), (202, 74.0), (203, 66.0), (204, 30.0),
            (205, 40.0), (206, 48.0), (207, 120.0), (208, 126.0), (209, 136.0), (210, 240.0), (211, 82.0), (212, 150.0), (213, 122.0), (215, 62.0),
            (216, 60.0), (217, 32.0), (218, 70.0),
        ]
    }

    /// The asset step's looping clips (KH1's last pose equals its first).
    fn loops() -> Vec<u16> {
        vec![0, 1, 2, 6, 204, 205, 212, 217]
    }

    fn len(r: u16) -> f32 {
        frames().iter().find(|x| x.0 == r).map_or(30.0, |x| x.1)
    }

    /// One Neoshadow on a flat floor at y = 0: an action holds its AI for its motion; a motion plays at once,
    /// a queued one when the playing one has played; a body on the ground state above the floor falls.
    struct One {
        ai: Ai,
        pos: Vec3,
        busy: f32,
        current: Option<u16>,
        age: f32,
        queued: Option<u16>,
        moved: f32,
        log: Vec<(usize, Cmd)>,
        visible: bool,
        hittable: bool,
        dead: bool,
    }

    impl One {
        fn new(ai: Ai, pos: Vec3) -> One {
            One { ai, pos, busy: 0.0, current: None, age: 0.0, queued: None, moved: 0.0, log: Vec::new(), visible: true, hittable: true, dead: false }
        }

        fn step(&mut self, f: usize, sora: Vec3) {
            if self.dead {
                return;
            }
            self.age += 1.0;
            if let Some(r) = self.current {
                if self.age >= len(r) {
                    if let Some(q) = self.queued.take() {
                        self.current = Some(q);
                        self.age = 0.0;
                    }
                }
            }
            let (airborne, flying) = {
                let b = self.ai.body();
                (b.airborne(), b.flying)
            };
            let w = World { step: 1.0, pos: self.pos, target: Some(sora), grounded: self.pos.y >= 0.0 && !airborne && !flying, airborne, moved: self.moved, motion: self.current };
            if self.busy > 0.0 {
                self.busy -= 1.0;
            } else {
                for c in self.ai.frame(w) {
                    match c {
                        ECmd::Action { request, then, .. } => {
                            self.busy = len(request);
                            self.current = Some(request);
                            self.age = 0.0;
                            self.queued = Some(then);
                        }
                        ECmd::Motion { request, .. } | ECmd::Attack { request, .. } => {
                            if self.current != Some(request) || matches!(c, ECmd::Attack { .. }) {
                                self.current = Some(request);
                                self.age = 0.0;
                                self.queued = None;
                            }
                        }
                        ECmd::Queue { request, .. } => self.queued = Some(request),
                        ECmd::Visible(v) => self.visible = v,
                        ECmd::Hittable(h) => self.hittable = h,
                        _ => {}
                    }
                    self.log.push((f, c));
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
            let (airborne, flying) = {
                let b = self.ai.body();
                (b.airborne(), b.flying)
            };
            if !airborne && !flying && self.pos.y < 0.0 {
                self.pos.y = (self.pos.y + 20.0).min(0.0);
            }
        }

        fn attacks(&self) -> Vec<(usize, Attack)> {
            self.log.iter().filter_map(|(f, c)| if let ECmd::Attack { attack, .. } = c { Some((*f, *attack)) } else { None }).collect()
        }

        fn motions(&self) -> Vec<u16> {
            self.log.iter().filter_map(|(_, c)| if let ECmd::Motion { request, .. } | ECmd::Attack { request, .. } = c { Some(*request) } else { None }).collect()
        }

        fn first(&self, want: impl Fn(&Cmd) -> bool) -> Option<usize> {
            self.log.iter().find(|(_, c)| want(c)).map(|x| x.0)
        }
    }

    /// `k` Neoshadows of one battle (one group), at `places`.
    struct Sim {
        n: Vec<One>,
        f: usize,
    }

    impl Sim {
        fn new(seed: u32, places: &[Vec3]) -> Sim {
            let g = Arc::new(Mutex::new(Group::default()));
            let n = places.iter().enumerate().map(|(i, p)| One::new(Ai::with_group(consts(), frames(), seed * 31 + i as u32, 0.0, g.clone()), *p)).collect();
            Sim { n, f: 0 }
        }

        fn run(&mut self, sora: Vec3, frames: usize) {
            for _ in 0..frames {
                for o in self.n.iter_mut() {
                    o.step(self.f, sora);
                }
                self.f += 1;
            }
        }

        fn group(&self) -> Group {
            self.n[0].ai.group()
        }
    }

    #[test]
    fn the_names_cover_the_numbers() {
        assert!(Table::new(vec![], NAMES).is_err());
        let c = consts();
        assert_eq!((c.num("crowd"), c.num("group_min"), c.num("pair"), c.num("gather_max"), c.num("lone")), (5.0, 3.0, 2.0, 4.0, 1.0));
        assert_eq!((c.num("slot_2_0_x"), c.num("slot_2_0_y"), c.num("slot_2_0_z")), (8800.0, 5480.0, 100.0));
        assert_eq!((c.num("dive_speed"), c.num("dive_cycles"), c.num("lift_height"), c.num("aim_factor")), (80.0, 9.0, 120.0, 1.4));
    }

    /// The battle's group: one per battle, a new one with the next battle (KH1's block is zeroed per room).
    #[test]
    fn one_group_per_battle() {
        let a = group_for_battle(900_001);
        a.lock().unwrap().count = 3;
        assert_eq!(group_for_battle(900_001).lock().unwrap().count, 3);
        assert_eq!(group_for_battle(900_002).lock().unwrap().count, 0);
        let ai = Ai::new(consts(), frames(), 1, 0.0);
        Behaviour::set_battle(&ai, 900_003);
        let mut s = One::new(ai, Vec3::ZERO);
        s.step(0, Vec3::new(0.0, 0.0, 500.0));
        assert_eq!(group_for_battle(900_003).lock().unwrap().count, 1, "its init counts it in the battle's group");
    }

    /// `0xF34`: no appearance; the idle state targets Sora and engages at once.
    #[test]
    fn idle_engages_at_once() {
        let mut s = Sim::new(1, &[Vec3::ZERO]);
        s.run(Vec3::new(0.0, 0.0, 400.0), 3);
        let states: Vec<u8> = s.n[0].log.iter().filter_map(|(_, c)| if let ECmd::State(x) = c { Some(*x) } else { None }).collect();
        assert_eq!(states, vec![st::IDLE, st::ENGAGED]);
        assert!(s.n[0].ai.risen());
        assert_eq!(s.group().count, 1);
        assert_eq!(s.group().centre, Some(Vec3::new(0.0, 0.0, 400.0)), "the battle's middle: where Sora stood");
    }

    /// `0x8FA`: alone, it sinks, travels straight to KH1's room spot (here 1 m along x from where Sora started:
    /// (8900, 100) against the reference (8800, 100)), comes up and calls (210); then its rounds.
    #[test]
    fn alone_it_goes_to_the_room_spot_and_calls() {
        let mut s = Sim::new(2, &[Vec3::new(-800.0, 0.0, 0.0)]);
        s.run(Vec3::ZERO, 900);
        let o = &s.n[0];
        let sink = o.first(|c| matches!(c, ECmd::Motion { request: req::SINK, .. })).expect("it sinks");
        let hide = o.first(|c| *c == ECmd::Hittable(false)).unwrap();
        assert_eq!(hide - sink, 47, "unhittable from frame 46 + 1 of the sink (its 6 frames past 40)");
        let up = o.first(|c| matches!(c, ECmd::Motion { request: req::COME_UP, .. })).expect("it comes up");
        let call = o.first(|c| matches!(c, ECmd::Motion { request: req::CALL, .. })).expect("it calls");
        assert!(up < call);
        assert!(o.log.iter().any(|(_, c)| matches!(c, ECmd::Note(n) if n.contains("600..606"))));
        // It came up within 50 of the spot (100, 0, 0).
        let at_up = o.log.iter().filter(|(f, _)| *f < up).count();
        assert!(at_up > 0);
        let spot = Vec3::new(100.0, 0.0, 0.0);
        let d = (o.pos - spot).length();
        assert!(d < 1500.0, "{:?}", o.pos);
        // Then the rounds: something of its own after the call.
        let after: Vec<u16> = o.log.iter().filter(|(f, _)| *f > call + 246).filter_map(|(_, c)| if let ECmd::Motion { request, .. } | ECmd::Attack { request, .. } = c { Some(*request) } else { None }).collect();
        assert!(!after.is_empty(), "its rounds go on");
    }

    /// Two of them: the first goes underground for good (hidden, unhittable), comes up at Sora's feet and misses
    /// (209: KH1's grab is not built); the other fights. When the other dies, the hidden one comes up at KH1's
    /// spot and calls.
    #[test]
    fn two_of_them_one_hides_and_ambushes() {
        let mut s = Sim::new(3, &[Vec3::new(-300.0, 0.0, 0.0), Vec3::new(300.0, 0.0, 0.0)]);
        let sora = Vec3::new(0.0, 0.0, 600.0);
        s.run(sora, 700);
        let specials: Vec<bool> = s.n.iter().map(|o| o.ai.special()).collect();
        assert_eq!(specials.iter().filter(|x| **x).count(), 1, "{specials:?}");
        let h = specials.iter().position(|x| *x).unwrap();
        let o = &s.n[h];
        assert!(!o.hittable && !o.visible || o.current == Some(req::MISS));
        let miss = o.first(|c| matches!(c, ECmd::Motion { request: req::MISS, .. })).expect("the miss at Sora's feet");
        assert!(o.log.iter().any(|(f, c)| *f == miss && *c == ECmd::Visible(true)));
        assert!(o.log.iter().any(|(_, c)| matches!(c, ECmd::Note(n) if n.contains("grab"))));
        assert!(!o.log.iter().any(|(_, c)| *c == ECmd::Hittable(true)), "never hittable while two are left");
        let other = &s.n[1 - h];
        assert!(other.motions().iter().any(|r| [req::CLAW, req::SINK, req::STRAFE_L, req::STRAFE_R, req::RUN, req::CROUCH].contains(r)), "{:?}", other.motions());
        // The other dies: the hidden one is alone.
        s.n[1 - h].ai.on_defeat();
        s.n[1 - h].dead = true;
        assert_eq!(s.group().count, 1);
        let n0 = s.n[h].log.len();
        s.run(sora, 900);
        let o = &s.n[h];
        let later = &o.log[n0..];
        assert!(later.iter().any(|(_, c)| matches!(c, ECmd::Motion { request: req::COME_UP, .. })));
        assert!(later.iter().any(|(_, c)| *c == ECmd::Hittable(true)));
        assert!(later.iter().any(|(_, c)| matches!(c, ECmd::Motion { request: req::CALL, .. })));
        assert!(!o.ai.special());
    }

    /// Three of them: they sink, wait for each other, take three different places of a formation and rise
    /// slashing together, the shield (row `0x80`, held) on from the rise to 70 frames before 207's end.
    #[test]
    fn three_rise_together_from_a_formation() {
        let mut s = Sim::new(4, &[Vec3::new(-400.0, 0.0, 0.0), Vec3::new(400.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -400.0)]);
        let sora = Vec3::new(0.0, 0.0, 300.0);
        s.run(sora, 400);
        let rises: Vec<usize> = s.n.iter().map(|o| o.attacks().iter().find(|a| a.1 == Attack::RiseSlash).map(|a| a.0).expect("it rises slashing")).collect();
        assert!(rises.iter().all(|&f| f == rises[0]), "together: {rises:?}");
        let places: Vec<Vec3> = s.n.iter().map(|o| o.pos).collect();
        for i in 0..3 {
            for j in i + 1..3 {
                assert!((places[i] - places[j]).length() > 100.0, "different places {places:?}");
            }
        }
        for o in &s.n {
            let on = o.first(|c| *c == ECmd::Guard(Some((SHIELD, HELD)))).unwrap();
            let off = o.first(|c| *c == ECmd::Guard(None)).unwrap();
            assert_eq!(on, rises[0]);
            assert_eq!(off - on, 51, "120 − 70 frames, the frame after it ends");
        }
        let g = s.group();
        assert!(g.layout_chosen && g.slots.iter().filter(|x| **x).count() == 3);
        assert_eq!((g.sunk, g.go), (0, 0));
    }

    /// Five of them: all leap (movement state 7, 120 above the floor), the dive starts once all five are up,
    /// and they dive one at a time (the token), their shields on through the leap and the dive.
    #[test]
    fn five_gather_and_dive_one_at_a_time() {
        let places: Vec<Vec3> = (0..5).map(|i| Vec3::new(-800.0 + 400.0 * i as f32, 0.0, 0.0)).collect();
        let mut s = Sim::new(5, &places);
        let sora = Vec3::new(0.0, 0.0, 600.0);
        s.run(sora, 90);
        assert!(s.n.iter().all(|o| o.ai.body().air_flight), "all up");
        assert!(s.n.iter().all(|o| o.pos.y <= -110.0), "{:?}", s.n.iter().map(|o| o.pos.y).collect::<Vec<_>>());
        assert_eq!(s.group().ready, 5);
        s.run(sora, 2400);
        let mut dives: Vec<(usize, usize)> = Vec::new();
        for o in &s.n {
            let start = o.attacks().iter().find(|a| a.1 == Attack::Dive).map(|a| a.0).expect("each dives");
            let end = o.log.iter().find(|(f, c)| *f > start && matches!(c, ECmd::Motion { request: req::LAND_DIVE, .. })).map(|x| x.0).unwrap();
            assert!(o.log.iter().any(|(f, c)| *f < start && *c == ECmd::Guard(Some((SHIELD, HELD)))));
            dives.push((start, end));
        }
        dives.sort();
        for w in dives.windows(2) {
            assert!(w[1].0 > w[0].1, "one at a time: {dives:?}");
        }
    }

    /// The hit handler: the fourth flinch in a row and a knock-back mark the counter; the shielded and the
    /// hidden one take no reaction; 213 only KH1's push.
    #[test]
    fn the_hit_rules() {
        let mut s = Sim::new(6, &[Vec3::ZERO]);
        s.run(Vec3::new(0.0, 0.0, 3000.0), 2);
        let ai = &s.n[0].ai;
        for i in 0..4 {
            ai.set_hit_kind(3);
            assert!(!ai.armoured(None, 0));
            Behaviour::on_hit(ai, HitNews::Damage);
            assert_eq!(ai.io.borrow().x.mark == Mark::Counter, i == 3);
        }
        assert!(ai.io.borrow().x.armour && ai.armoured(None, 0), "no reaction until the counter runs");
        ai.io.borrow_mut().x.armour = false;
        ai.io.borrow_mut().x.mark = Mark::None;
        ai.set_hit_kind(4);
        Behaviour::on_hit(ai, HitNews::Damage);
        assert_eq!(ai.io.borrow().x.mark, Mark::Counter);
        ai.io.borrow_mut().x.shielded = true;
        assert!(ai.armoured(Some(req::DIVE), 0));
        ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
        assert!(ai.body().push.is_none(), "no push");
        ai.io.borrow_mut().x.shielded = false;
        assert!(ai.armoured(Some(req::CALL), 0));
        assert!(ai.armoured(Some(req::CALLED_END), 0));
        ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
        assert!(ai.body().push.is_some(), "213: KH1's push slide (sys 42)");
        assert!(!ai.armoured(Some(req::CLAW), 0));
    }

    /// Its death: one fewer counted, the last sets the called ones vanishing; the death motion is 72.
    #[test]
    fn its_death() {
        let mut s = Sim::new(7, &[Vec3::ZERO, Vec3::new(500.0, 0.0, 0.0)]);
        s.run(Vec3::new(0.0, 0.0, 600.0), 3);
        assert_eq!(s.group().count, 2);
        s.n[0].ai.on_defeat();
        s.n[0].ai.on_defeat();
        assert_eq!((s.group().count, s.group().last_died), (1, false), "once per enemy");
        s.n[1].ai.on_defeat();
        assert_eq!((s.group().count, s.group().last_died), (0, true));
        assert_eq!(Behaviour::death_motion(&s.n[1].ai), Some(72));
    }

    /// A called one (made while another calls): hidden and unhittable, it comes up (211, 212 four times, the
    /// action 213), is counted, then idles.
    #[test]
    fn a_called_one_comes_up() {
        let g = Arc::new(Mutex::new(Group { count: 1, calling: true, ..Group::default() }));
        let mut o = One::new(Ai::with_group(consts(), frames(), 9, 0.0, g.clone()), Vec3::ZERO);
        o.step(0, Vec3::new(0.0, 0.0, 600.0));
        assert!(!o.visible && !o.hittable);
        assert_eq!(g.lock().unwrap().coming, 1);
        for f in 1..1000 {
            o.step(f, Vec3::new(0.0, 0.0, 600.0));
        }
        let m = o.motions();
        assert!(m.contains(&req::CALLED_RISE) && m.contains(&req::CALLED_HOLD), "{m:?}");
        assert!(o.log.iter().any(|(_, c)| matches!(c, ECmd::Action { request: req::CALLED_END, .. })));
        let g = g.lock().unwrap();
        assert_eq!((g.count, g.coming), (2, 0));
    }

    /// A long fight (alone and two of them): each acts at least every 600 frames, Sora circling 2–7 m away and
    /// a hit every 150–400 frames (12 seeds, 20 000 frames).
    #[test]
    fn a_long_fight_never_stalls() {
        for k in [1usize, 2] {
            for seed in 0..12u32 {
                let places: Vec<Vec3> = (0..k).map(|i| Vec3::new(-300.0 + 600.0 * i as f32, 0.0, 0.0)).collect();
                let mut s = Sim::new(seed + 100, &places);
                let mut r = Kh1Rand::new(seed + 7);
                let (mut last, mut worst, mut next_hit) = (vec![0usize; k], vec![0usize; k], 300usize);
                for f in 0..20_000usize {
                    let a = f as f32 / 600.0;
                    let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
                    let sora = Vec3::new(a.sin() * rad, 0.0, a.cos() * rad);
                    if f == next_hit {
                        let i = (r.next_f32() * k as f32) as usize % k;
                        let o = &mut s.n[i];
                        if o.hittable && o.visible {
                            o.ai.set_hit_kind([3u8, 3, 4, 5][(r.next_f32() * 4.0) as usize % 4]);
                            if o.ai.armoured(o.current, 0) {
                                o.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
                            } else {
                                Behaviour::on_hit(&o.ai, HitNews::Damage);
                                o.busy = 40.0;
                            }
                        }
                        next_hit = f + 150 + (r.next_f32() * 250.0) as usize;
                    }
                    let n: Vec<usize> = s.n.iter().map(|o| o.log.len()).collect();
                    s.run(sora, 1);
                    for (i, o) in s.n.iter().enumerate() {
                        if o.log[n[i]..].iter().any(|(_, c)| !matches!(c, ECmd::Motion { request: 1 | 2, .. })) || o.busy > 0.0 || o.ai.body().walk != Vec3::ZERO {
                            worst[i] = worst[i].max(f - last[i]);
                            last[i] = f;
                        }
                    }
                }
                for i in 0..k {
                    assert!(worst[i] < 600, "{k} of them, seed {seed}, #{i}: {} frames without acting", worst[i]);
                }
            }
        }
    }

    /// The Neoshadow through the host's rules (`crate::enemy::host::model`, what `kh1-recom`'s `win::kh1_enemy`
    /// decides) with its class map (`ground::neoshadow::CLASS.attack_states`: claw 2, rise slash 3, dive 3).
    fn hosted(seed: u32) -> Model<Consts> {
        Model::new(Box::new(Ai::new(consts(), frames(), seed, 0.0)), consts(), frames(), loops(), vec![2, 3, 3])
    }

    /// Alone in the host for 20 000 frames (12 seeds, Sora circling 2–7 m away): it attacks, nothing is
    /// released, no attack is asked for again within two frames; the claw plays in state 2, the rise slash and
    /// the dive in state 3; the dive's looping 205 plays unbroken until the landing replaces it.
    #[test]
    fn hosted_a_long_fight() {
        for seed in 0..12u32 {
            let mut m = hosted(seed);
            for f in 0..20_000usize {
                let a = f as f32 / 600.0;
                let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
                m.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
            }
            assert_eq!(m.released, 0, "seed {seed}");
            let issued: Vec<(usize, u16)> = m.log.iter().filter_map(|(f, c)| if let ECmd::Attack { request, .. } = c { Some((*f, *request)) } else { None }).collect();
            assert!(issued.len() > 10, "seed {seed}: it attacks ({})", issued.len());
            for w in issued.windows(2) {
                assert!(w[1].1 != w[0].1 || w[1].0 > w[0].0 + 2, "seed {seed}: {} asked again at {} after {}", w[0].1, w[1].0, w[0].0);
            }
            assert!(m.trace.iter().all(|t| t.1 != Some(req::CLAW) || t.0 == 2 || t.0 == 1), "seed {seed}: 202 outside state 2");
            assert!(m.trace.iter().all(|t| !matches!(t.1, Some(req::DIVE) | Some(req::RISE_SLASH)) || t.0 == 3 || t.0 == 1), "seed {seed}: 205 / 207 outside state 3");
        }
    }

    /// The rows and the shield.
    #[test]
    fn rows() {
        assert_eq!((Attack::Claw.rows(), Attack::RiseSlash.rows(), Attack::Dive.rows()), (&[0x1D0, 0x240][..], &[0x160][..], &[0xF0][..]));
        assert_eq!((Attack::Dive.index(), Attack::ALL.len(), SHIELD_ROW), (2, 3, 0x80));
        assert_eq!(ROOM_REF, (2, 0));
    }
}
