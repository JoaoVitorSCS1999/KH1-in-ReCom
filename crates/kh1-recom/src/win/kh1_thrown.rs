//! M11: a KH1 enemy's thrown object as one of Re:CoM's own bullets (`docs/recom-m11-enemies.md` §
//! The Bandit's thrown scimitar; owner 2026-11-02: "a Re:CoM homing bullet moved by KH1's numbers,
//! drawn with a Re:CoM effect"). The Bandit's scimitar (`sys 94`'s child `ex_2090_00.bd`):
//!
//! - **Made** as Re:CoM's own enemies make theirs (the Aquatank's `0x14018AC50`; Checked (decomp +
//!   disasm)): a shot descriptor (`0x140156790(desc, 2 = CHomingBullet, 3)`, its side word `+0xC` left
//!   at the init's `0x80000002`, so the bullet's attack tests the player's side, `0x14016F720`), the
//!   power Re:CoM gives the thrower's attack id (`0x14017F0E0(enemy, id, kind, 0, &kind, flag)` →
//!   AP, `0x14017F050(enemy, id)` → attribute, as its attack records get theirs, `0x14014E320`), the
//!   factory `0x1401A8A80`, the manager `0x140158DD0` with the thrower (its `+0x238` shooter), the
//!   mover set-up `0x140157DA0`; at the thrower's bone (KH1's bone code 28 as Re:CoM's bone), along its
//!   facing.
//! - **Looks** like Re:CoM's Aero shot: the effect set `0x1405A2030` (Aero level 1: `airo_all` 5 while it
//!   flies), one of Sora's spell sets Re:CoM loads with every battle.
//! - **Moves** by KH1's numbers ([`kh1_core::enemy::thrown`]): its flight state's exe (the vcall thunk
//!   every bullet runs) becomes [`blade_exe`], which steps KH1's flight (10 a frame, back past 120
//!   frames, 3° a frame toward Sora's aim point) and writes the velocity, then runs Re:CoM's own step.
//!   Re:CoM collides it, hits Sora with it (ended by the hit, flag `0x100`) and ends it at KH1's life
//!   (300 frames, flag 4).
//! - **Guarded** by Sora's KH1 Guard: `super::guard`'s turn-back (decision 28: its collision moves to
//!   the enemies' side, its velocity reverses); from then on Re:CoM's own homing takes it back at the
//!   thrower (its homing target `+0x2F4`, 3° a frame) and our exe only passes on.
//!
//! - **Swung at** (owner 2026-11-02, on 0.75.2: "Sora's swings can knock the thrown scimitar away"):
//!   [`swing_at_blades`], from the swing's pair test (`win::attack::pair_dispatch`, while Sora's
//!   KH1 swing is live): its spheres against each flying blade's own collision shape through Re:CoM's
//!   overlap tests. KH1 (`0x1402B4A20` → `0x1402B4BC0`, Checked (decomp)): the blade's row `0x1CC`
//!   has priority 0, so Sora's side gets nothing (no record: `b`'s priority 0) and the blade's gets
//!   clash bit 0x10 (owner turned to the swinger); its child script (`ex_2090_00.bd` `0x1A0..0x2D0`,
//!   Checked (files)) then turns back with a random tilt, fades over 60 frames and slides on (× 0.9 a
//!   frame) for 60 frames without moving its attack again, and ends. Here: [`Thrown::repel`], the
//!   bullet's attack off (Re:CoM's own off), Re:CoM's guard spark at the contact, the slide on our
//!   flight step, then Re:CoM's end.
//!
//! - **The Fat Bandit's fireball** (M11, `ex_2060_00.bd`; owner 2026-11-02: "Re:CoM bullets moved by KH1's numbers ...
//!   drawn with a Re:CoM effect (its own fireball's)"): the same bullet and exe ([`Kind::Fire`]), made where KH1's child
//!   starts (bone code 4 at t 2.5: the manifest's `fire_at` point on Re:CoM's `neck`), along the head's direction to
//!   Sora's aim point (`kh1_core::fatbandit::fire_dir` from the `eye` point), flying straight at 45 a frame (no turn)
//!   for 240 frames; its power Re:CoM's own fireball's (`0x14017F0E0(enemy, 1, 1, 0)`, `0x14018DF20`), its look Re:CoM's
//!   own fireball's effect sets (`0x14053E290` / `0x14053E300`). Sora's Guard turns it back (decision 28); **his swing
//!   bats it** (KH1: its row `0x1CC` has priority 0, a projectile's clash turns it to the swinger, `0x40` → `0x1B8` aims it
//!   at the swinger's target, else at its thrower, and flies it its whole life again): Guard's turn-back
//!   (`super::guard::send_back`: it then hurts enemies), aimed at Sora's target or the Fat Bandit, straight again.
//!
//! - **The Defender's fireball** (M11, `ex_2340_01.bd`; owner 2026-11-02: "Re:CoM bullets moved by KH1's numbers, drawn
//!   with a Re:CoM effect (its own)"): the same bullet and exe ([`Kind::Fire`]), made where KH1's child starts (bone
//!   `0x106C` at t 3: the manifest's `fire_at` point on Re:CoM's `chin`), along the Defender's facing (`sys 143` with no
//!   head look), at a speed drawn in [8, 10) a frame (`0x418`: KH1's `rand`; here a stream of our own, Assumption), for
//!   100 frames, **turning at most 1° a frame toward Sora's aim point** while he is there (`sys 24`, `0x2F8`); its power
//!   Re:CoM's own fireball's (`0x14017F0E0(enemy, 1, 1, 0)`, `0x14018DAF0`), its look Re:CoM's own Defender fireball's
//!   effect set (`0x14053CF30`). Sora's Guard turns it back (decision 28: Re:CoM's homing at 1° a frame takes it at its
//!   thrower); his swing bats it (`0x1B8`: aimed half-way between its thrower and Sora's target when he has one, then
//!   turning 1° a frame toward that target; else at its thrower, turning toward it). Either way it hurts enemies
//!   ([`own_shot`]: the Defender's handler takes its own fireball on its shield as KH1's big hit).
//!
//! - **The Red Nocturne's Fire and big Fire** (M11, `ex_2110_00.bd` / `ex_2110_01.bd`; the Defender's fireball library, its
//!   flight, re-aim and homing routines byte for byte: Checked (files)): the same bullet and exe ([`Kind::Fire`]), one
//!   per child (prefixes `fire` / `bigfire`), made when the child's wait in its parent's cast ends (the behaviour's
//!   `kh1_core::caster::Cast`) at KH1's start, `sys 19(parent, 0x8001, −0.5)` then 80 / 70 up: 50 ahead of the Red
//!   Nocturne's place along its facing ([`kh1_core::rednocturne::fire_start`]), along its facing, 12 / 22 a frame,
//!   turning 1° / 2° a frame toward Sora's aim point, 90 frames; its power Re:CoM's own Red Nocturne shots' (id 0 / 1,
//!   kind 1, `0x140191C00` / `0x140191CC0`), its look their effect sets (`0x140548EF0` / `0x140548F60`); radius KH1's
//!   30 / 40. His swing bats it (the Defender's `0x1B8` = its `0x288`), with KH1's batted row's radius 60; **so does his
//!   Guard** (KH1's guard row meets the fire's type-0, priority-0 free-standing row through `0x1402B4BC0` as a swing does,
//!   and the child reads only `+4 & 0x30` before its re-aim: the Gargoyle's rule, owner 2026-11-03; [`GUARD_BATS`]).
//!   Either way it hurts enemies; its own kind's handler takes it as its own row (`own_shot`: KH1's `sys 188`).
//!
//! - **The Blue Rhapsody's Blizzard** (M11, `ex_2120_00.bd`; Checked (files)): the same bullet and exe ([`Kind::Fire`]),
//!   made when the child's wait in its parent's cast 55 ends (84 frames, `kh1_core::caster::Cast::direct`) where KH1's child
//!   starts (`sys 19(parent, 0, 0)`: bone code 0's parent, the manifest's `blizzard_at` point on Re:CoM's bone), along the
//!   Blue Rhapsody's facing (`+0x30`), straight at 25 a frame (`sys 18`; no turn) for 60 frames; its power Re:CoM's own
//!   Blue Rhapsody shot's (id 0, kind 1, `0x14018C2D0`), its look that shot's effect set (`0x1407C2F10`); radius KH1's
//!   row's 40. KH1's child ends as soon as its row's `+4` is set (`0x34E`: a hit, or a clash with a swing or a guard, which
//!   meet its type-0, priority-0 free-standing row through `0x1402B4BC0`; no re-aim, no batted row): **a swing or Sora's
//!   Guard ends it** ([`ENDS_ON_CLASH`]), Re:CoM's guard spark at it.
//!
//! - **The Yellow Opera's Thunder** (M11, `ex_2130_00.bd`): a strike, not a flight: Re:CoM's own Yellow Opera bolt placed
//!   by KH1's numbers ([`strike`], [`bolt_exe`]; the section at the end of this file).
//!
//! Runs once a frame per flying blade (at most [`MAX`]); nothing otherwise.

use super::magic::{rt, Desc, DescInitFn, FactoryFn, ManagerFn, MoverFn, PlFn, DESC_INIT, FACTORY, HOMING_THUNK, SHOT_MANAGER, SHOT_MOVER};
use super::movement::{func, rd, rd_vec, wr};
use super::{base, log, va};
use crate::guard::bullet as gb;
use crate::kh1_enemy::Manifest;
use kh1_core::enemy::thrown::{Numbers, Phase, Thrown};
use kh1_core::rng::Kh1Rand;
use kh1_core::Vec3;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

/// Re:CoM's functions (bytes checked: `sites::THROWN_CALLEES`, `sites::MAGIC_CALLEES`).
mod f {
    /// `0x14017F0E0(enemy, attack id, kind, status, &kind out, flag) -> f32`: an enemy attack's AP
    /// (`@ENP +4` × the difficulty × the record's `+0x10`, then Re:CoM's enemy-card rewrite).
    pub const ATTACK_AP: u32 = 0x0017_F0E0;
    /// `0x14017F050(enemy, attack id) -> attribute`.
    pub const ATTACK_ATTR: u32 = 0x0017_F050;
    /// `0x140179410(record) -> kind`: Re:CoM's kind table `0x14052A220[record +0x68]`.
    pub const RECORD_KIND: u32 = 0x0017_9410;
    /// `0x1402ABB20(actor, bone)`: a bone (`+0xD0` its world place).
    pub const BONE: u32 = 0x002A_BB20;
}

/// The Aero level-1 shot's effect set (`0x1405A1730 + 24 × 0x60`; slot 1 `airo_all` 5). Checked (exe:
/// its slot 1 names `"airo_all"` at `0x1405A3150`, id 5; `sites::THROWN_PTRS`).
pub const BLADE_EFFECT_SET: u64 = 0x1_405A_2030;
/// Its flags: `0x100` (a hit, a wall, the range or the life ends it) and 4 (the life ends it); no
/// `0x8000` (our exe writes the velocity; the angles are only its look). Checked (decomp `0x140156E20`,
/// `0x1401569B0`).
pub(super) const FLAGS: u64 = 0x100 | 4;
/// The Fat Bandit's own fireball's effect sets (its state 3 `0x14018DF20`: descriptor `+0xA8` and `+0xB8`; slot 1 of
/// the first names `"_ex_2060"` (`sites::FATBANDIT_FIRE_PTRS`)). Checked (decomp + exe).
pub const FIRE_EFFECT_SET: u64 = 0x1_4053_E290;
pub const FIRE_EFFECT_PLACES: u64 = 0x1_4053_E300;
/// The Defender's own fireball's effect set (its state 3 `0x14018DAF0`: descriptor `+0xA8`, none at `+0xB8`; slot 1 names
/// `"_ex_2340"` (`sites::DEFENDER_FIRE_PTRS`)). Checked (decomp + exe).
pub const DEFENDER_FIRE_EFFECT_SET: u64 = 0x1_4053_CF30;

/// The Bouncywild's own homing banana's effect set (its state 3 `0x14018C870`: descriptor `+0xA8`, none at `+0xB8`; slot 1
/// names `"_ex_2040"` (`sites::BOUNCYWILD_FIRE_PTRS`)). Checked (decomp + exe).
pub const BOUNCYWILD_BANANA_EFFECT_SET: u64 = 0x1_407C_3010;

/// A class's own fireball look (descriptor `+0xA8`, `+0xB8`) when its effect set is where the exe had it: the Fat Bandit's
/// (`0x14053E290` / `0x14053E300`), the Defender's (`0x14053CF30`), the Bouncywild's banana (`0x1407C3010`, its peels too); None (no look) otherwise.
pub(super) unsafe fn fire_look(class: &str, prefix: &str) -> Option<(u64, Option<u64>)> {
    let ok = |sites: &[crate::sites::PtrSite]| sites.iter().all(|p| rd::<usize>(base() + p.rva as usize) == base() + p.target as usize);
    match (class, prefix) {
        ("defender", _) => ok(crate::sites::DEFENDER_FIRE_PTRS).then_some((DEFENDER_FIRE_EFFECT_SET, None)),
        ("bouncywild", _) => ok(crate::sites::BOUNCYWILD_FIRE_PTRS).then_some((BOUNCYWILD_BANANA_EFFECT_SET, None)),
        ("rednocturne", "bigfire") => ok(crate::sites::REDNOCTURNE_FIRE_PTRS).then_some((REDNOCTURNE_BIG_EFFECT_SET, None)),
        ("rednocturne", _) => ok(crate::sites::REDNOCTURNE_FIRE_PTRS).then_some((REDNOCTURNE_FIRE_EFFECT_SET, None)),
        ("bluerhapsody", _) => ok(crate::sites::BLUERHAPSODY_FIRE_PTRS).then_some((BLUERHAPSODY_BLIZZARD_EFFECT_SET, None)),
        _ => ok(crate::sites::FATBANDIT_FIRE_PTRS).then_some((FIRE_EFFECT_SET, Some(FIRE_EFFECT_PLACES))),
    }
}

/// The Red Nocturne's own shots' effect sets (`0x140191A30`'s descriptor `+0xA8`, none at `+0xB8`: its state 2's small shot
/// `0x140548EF0`, its state 3's big shot `0x140548F60`; slot 1 of each names `"_ex_2110"` (`sites::REDNOCTURNE_FIRE_PTRS`)).
/// Checked (decomp + exe).
pub const REDNOCTURNE_FIRE_EFFECT_SET: u64 = 0x1_4054_8EF0;
pub const REDNOCTURNE_BIG_EFFECT_SET: u64 = 0x1_4054_8F60;

/// The Blue Rhapsody's own Blizzard shot's effect set (its state 2 `0x14018C2D0`: descriptor `+0xA8`, none at `+0xB8`; slot 1
/// names `"_ex_2120"` (`sites::BLUERHAPSODY_FIRE_PTRS`)). Checked (decomp + exe).
pub const BLUERHAPSODY_BLIZZARD_EFFECT_SET: u64 = 0x1_407C_2F10;

/// The classes whose shot KH1's child ends at any clash (a swing or a guard meeting its row; `ex_2120_00.bd` `0x34E`: `+4 ≠
/// 0` ends the flight, no re-aim; Checked (files)): the Blue Rhapsody's Blizzard.
pub const ENDS_ON_CLASH: &[&str] = &["bluerhapsody"];

/// The classes whose fires Sora's Guard sends back KH1's way, as a swing does (the fire's row is KH1's type 0, priority 0,
/// free-standing: KH1's guard meets it through `0x1402B4BC0` as a swing does and hands it to Sora; its child reads only
/// `+4 & 0x30` before its re-aim; Checked (decomp + files), the Gargoyle's rule, owner 2026-11-03). The Fat Bandit's and
/// the Defender's keep decision 28 (Re:CoM's homing turn-back), as built.
pub const GUARD_BATS: &[&str] = &["rednocturne"];
/// The classes whose fire makes Sora slip (Re:CoM's slip kind 0x18 on the bullet, `crate::ground::bouncywild::SLIP_KIND`):
/// the Bouncywild's banana (owner 2026-11-03: "its thrown banana makes Sora slip too", Re:CoM's banana rule: its own
/// state 3 `0x14018C870` writes 0x18 to the descriptor's `+0x68`, `0x14018C924`; Checked (decomp + exe)). Its slip loses no
/// Moogle points (`super::kh1_peel::slip_scatter`).
pub const SLIPS: &[&str] = &["bouncywild"];
/// The range from its spawn (m; KH1's life ends it), the homing gate (m) after a turn-back.
pub(super) const RANGE: f32 = 255.0;
pub(super) const GATE: f32 = 255.0;
/// The fade after its end (ticks, as our Fire's).
pub(super) const FADE: i32 = 1000;
/// At most this many blades fly at once (older ones are forgotten: Re:CoM's own step flies them).
const MAX: usize = 8;

type ApFn = unsafe extern "win64" fn(usize, i32, i32, i16, *mut u32, u16) -> f32;
type AttrFn = unsafe extern "win64" fn(usize, i32) -> u32;
type KindFn = unsafe extern "win64" fn(*const u8) -> u32;
type BoneFn = unsafe extern "win64" fn(usize, i32) -> usize;

/// Which child flies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// The Bandit's scimitar (`blade_*`).
    Blade,
    /// The Fat Bandit's fireball (`fire_*`).
    Fire,
}

struct Blade {
    kind: Kind,
    /// Its constants' prefix (`blade`, `fire`, `bigfire`).
    prefix: &'static str,
    /// KH1's batted row's radius (KH1 units) when the manifest gives it (the Red Nocturne's 60).
    bat_radius: Option<f32>,
    /// Sora's Guard sends it back KH1's way ([`GUARD_BATS`]).
    guard_bats: bool,
    /// A swing or Sora's Guard ends it ([`ENDS_ON_CLASH`]).
    ends_on_clash: bool,
    /// A fireball Sora's swing sent back: KH1's flight at its new aim (no Re:CoM homing).
    batted: bool,
    /// What a batted fireball turns toward (KH1's `hp[0x30]` after `0x1B8`: Sora's target, else its thrower), while it lives.
    home: Option<u32>,
    id: u32,
    /// The thrower's id (Re:CoM's `+0x20`, the bullet's `+0x238`).
    shooter: u32,
    flight: Thrown,
    /// Sora's Guard turned it back (decision 28): Re:CoM's homing flies it from here.
    turned: bool,
    /// It carries Re:CoM's slip kind ([`SLIPS`]).
    slips: bool,
}

static BLADES: Mutex<Vec<Blade>> = Mutex::new(Vec::new());
/// A blade may be flying (set at a throw, cleared when none is left): the swing test's only cost
/// otherwise.
static FLYING: AtomicBool = AtomicBool::new(false);
/// The repel's random draws (KH1's `sys 25`; Assumption: a stream of our own, seeded once).
static RNG: Mutex<Option<Kh1Rand>> = Mutex::new(None);
static LINES: AtomicU32 = AtomicU32::new(60);

fn line(s: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("thrown: {}", s()));
    }
}

/// The manifest's numbers of thrown object `n` (`kh1_core::bandit::BLADE_NAMES`).
fn numbers(m: &Manifest) -> Numbers {
    let c = |k: &str| m.consts.get(k);
    Numbers { life: c("blade_life"), turn_back: c("blade_turn_back"), speed: c("blade_speed"), accel: c("blade_accel"), turn_deg: c("blade_turn"), slide: c("blade_slide"), slide_drag: c("blade_slide_drag") }
}

/// Sora's aim point (Re:CoM's vtable `+0x50`; Assumption: it stands for KH1's lock point 0, `sys 119`)
/// in KH1 units, while he is there with HP left (`sys 120`).
unsafe fn sora_target() -> Option<Vec3> {
    let pl: usize = rd(va(base(), crate::sites::data::MAIN_PLAYER));
    if pl == 0 || rd::<usize>(pl + 0xF0) == 0 {
        return None;
    }
    let p = super::magic::aim_point(pl);
    Some(crate::units::to_kh1(Vec3::new(p[0], p[1], p[2])))
}

/// `sys 94`: enemy `en` (class manifest `m`, KH1 facing `yaw`) makes its child `n`: the Bandit's blade (0), the Fat
/// Bandit's fireball (0) or flame (1, `super::kh1_flame`). The bullet's id.
pub(super) unsafe fn throw(en: usize, class: &str, m: &Manifest, n: u8, yaw: f32) -> Option<u32> {
    let has = |k: &str| m.consts.get(k).is_finite();
    // Its children's indices (`sys 94`'s second word): the manifest's when it names them (the Defender: the flame 0, the
    // fireball 1), else the Fat Bandit's (the fireball 0, the flame 1).
    let index = |k: &str, d: u8| if has(k) { m.consts.get(k) as u8 } else { d };
    let (fire_n, flame_n) = (index("fire_child", 0), index("flame_child", 1));
    // The Bouncywild's peels (`ex_2040_01.bd` tossed, `_02.bd` dropped; `super::kh1_peel`).
    if has("peel_lie") && (n == index("peel_child", 1) || n == index("drop_child", 2)) {
        super::kh1_peel::start(en, class, m, n == index("peel_child", 1), yaw);
        return None;
    }
    if has("ball_life") && n == 0 {
        ball_start(en, class, m);
        return None;
    }
    // M11, the Blue Rhapsody's Blizzard (`ex_2120_00.bd`).
    if has("blizzard_life") && n == index("blizzard_child", 0) {
        return fire(en, class, m, yaw, "blizzard");
    }
    if has("fire_life") && n == fire_n {
        return fire(en, class, m, yaw, "fire");
    }
    // M11, the Red Nocturne's second fire child (`ex_2110_01.bd`, the big Fire).
    if has("bigfire_life") && n == index("bigfire_child", 1) {
        return fire(en, class, m, yaw, "bigfire");
    }
    if has("flame_life") && n == flame_n {
        super::kh1_flame::start(en, class, m);
        return None;
    }
    if !(n == 0 && has("blade_life")) {
        line(|| format!("{class}: no thrown object {n}"));
        return None;
    }
    let c = |k: &str| m.consts.get(k);
    let bone = func::<BoneFn>(f::BONE)(en, c("blade_bone") as i32);
    if bone == 0 {
        line(|| format!("{class}: no bone {} for the throw", c("blade_bone")));
        return None;
    }
    let at = rd_vec(bone + 0xD0);
    let id = c("blade_recom_id") as i32;
    // The power Re:CoM gives its attack id, as its own attack records get it (`0x14014E320`).
    let mut rec = [0u8; 0x70];
    rec[0x68..0x6C].copy_from_slice(&(c("blade_kind_index") as i32).to_le_bytes());
    let kind = func::<KindFn>(f::RECORD_KIND)(rec.as_ptr());
    let mut kind_out = kind;
    let ap = func::<ApFn>(f::ATTACK_AP)(en, id, kind as i32, 0, &mut kind_out, (c("blade_flag2") != 0.0) as u16) as i32;
    let attr = func::<AttrFn>(f::ATTACK_ATTR)(en, id);
    let flight = Thrown::new(crate::units::to_kh1(at), yaw, numbers(m));
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x68) as *mut u32, kind_out);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x60) as *mut u64, FLAGS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, flight.speed * crate::units::METRES_PER_UNIT);
    let life = crate::units::frames_to_units(flight.n.life);
    std::ptr::write_unaligned(w(0x38) as *mut i32, life);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, life);
    std::ptr::write_unaligned(w(0x34) as *mut f32, RANGE);
    std::ptr::write_unaligned(w(0x44) as *mut i32, 0);
    std::ptr::write_unaligned(w(0x48) as *mut f32, GATE);
    std::ptr::write_unaligned(w(0x4C) as *mut f32, flight.n.turn_deg);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [at.x, at.y, at.z, 0.5]);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [0.0, crate::units::yaw_to_recom(yaw), 0.0, 0.0]);
    // The set's slot 1 must name `airo_all` (`sites::THROWN_PTRS`); else no look (the blade still flies).
    let set = crate::sites::THROWN_PTRS.iter().all(|p| rd::<usize>(base() + p.rva as usize) == base() + p.target as usize);
    if set {
        std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), BLADE_EFFECT_SET));
    } else {
        line(|| format!("{class}: the Aero shot's effect set is not where 0x1405A2030 was: the blade has no look"));
    }
    std::ptr::write_unaligned(w(0x78) as *mut f32, c("blade_radius") * crate::units::METRES_PER_UNIT);
    std::ptr::write_unaligned(w(0x94) as *mut i32, FADE);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        line(|| format!("{class}: no entity factory: no blade"));
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, en);
    if bullet == 0 {
        line(|| format!("{class}: Re:CoM's shot manager made no blade"));
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    let bid: u32 = rd(bullet + 0x20);
    let b = base();
    let vt: usize = rd(bullet);
    let exe: usize = rd(bullet + rt::STATE_EXE);
    let adj: i32 = rd(bullet + rt::STATE_EXE_ADJUST);
    if vt != va(b, gb::HOMING_VTABLE) || exe != b + HOMING_THUNK as usize || adj != 0 {
        line(|| format!("{class}: blade {bid} is not a plain CHomingBullet in flight (vt {vt:#x}, exe {exe:#x}): Re:CoM's straight flight"));
        return Some(bid);
    }
    let shooter: u32 = rd(en + 0x20);
    if let Ok(mut g) = BLADES.lock() {
        g.retain(|b| b.id != bid);
        if g.len() >= MAX {
            g.remove(0);
        }
        g.push(Blade { kind: Kind::Blade, prefix: "blade", bat_radius: None, guard_bats: false, ends_on_clash: false, batted: false, home: None, id: bid, shooter, flight, turned: false, slips: false });
        FLYING.store(true, Ordering::Relaxed);
    }
    wr(bullet + rt::STATE_EXE, blade_exe as *const () as usize);
    line(|| format!("{class} e{shooter}: throws its blade: shot {bid} at ({:.2},{:.2},{:.2}) yaw {yaw:.2}, AP {ap} kind {kind_out} attribute {attr} (Re:CoM attack id {id}), radius {:.2} m, life {} frames", at.x, at.y, at.z, c("blade_radius") * 0.01, flight.n.life));
    Some(bid)
}

/// A Re:CoM bone's world point from an offset in its frame (metres), as Re:CoM places its own attack records
/// (`0x14014D740` → `0x1402B4A30(out, bone+0xA0, offset)`: x·row0 + y·row1 + z·row2 + row3, `0x1402ECEB0`; Checked
/// (decomp)).
unsafe fn bone_point(en: usize, bone: i32, off: [f32; 3]) -> Option<Vec3> {
    let b = func::<BoneFn>(f::BONE)(en, bone);
    if b == 0 {
        return None;
    }
    let r = |i: usize| rd_vec(b + 0xA0 + 0x10 * i);
    Some(r(0) * off[0] + r(1) * off[1] + r(2) * off[2] + r(3))
}

/// The manifest's bone point `p` (`<p>_bone`, `<p>_x / _y / _z`; the asset step's `T_POINTS`) on enemy `en`, Re:CoM
/// metres.
pub(super) unsafe fn manifest_point(en: usize, m: &Manifest, p: &str) -> Option<Vec3> {
    let c = |k: &str| m.consts.get(&format!("{p}_{k}"));
    bone_point(en, c("bone") as i32, [c("x"), c("y"), c("z")])
}

/// The Fat Bandit's fireball (`ex_2060_00.bd`, module doc): a `CHomingBullet` as the blade, at the `fire_at` point,
/// aimed by KH1's head direction from the `eye` point at Sora's aim point, flying KH1's straight 45 a frame for 240
/// frames; Re:CoM's own fireball's power (id 1, kind 1, `0x14018DF20`) and look; KH1's radius (the row's 30).
unsafe fn fire(en: usize, class: &str, m: &Manifest, yaw: f32, p: &'static str) -> Option<u32> {
    let c = |k: &str| m.consts.get(&format!("{p}_{k}"));
    let start = if c("at_bone").is_finite() {
        manifest_point(en, m, &format!("{p}_at"))
    } else {
        // M11, the Red Nocturne: `sys 19(parent, 0x8001, t)` then `up`, from its own place and facing.
        super::kh1_enemy::kh1_place(en).map(|pos| crate::units::to_recom(kh1_core::rednocturne::fire_start(pos, yaw, c("t"), c("up"))))
    };
    let Some(at) = start else {
        line(|| format!("{class}: no place for the {p}'s start ({})", c("at_bone")));
        return None;
    };
    let target = sora_target();
    // `sys 143` (`0x1402BAE10`): the head's direction while its look is on (the Fat Bandit's, from its `eye` point), else
    // the facing (the Defender's script never turns a head look on).
    let dir = if m.consts.get("eye_bone").is_finite() {
        let Some(eye) = manifest_point(en, m, "eye") else {
            line(|| format!("{class}: no bone for the fireball's aim ({})", m.consts.get("eye_bone")));
            return None;
        };
        kh1_core::fatbandit::fire_dir(yaw, crate::units::to_kh1(eye), target, m.consts.get("look_cone_out"), m.consts.get("look_up"), m.consts.get("look_down"), m.consts.get("look_left"), m.consts.get("look_right"))
    } else {
        Vec3::from_yaw(yaw)
    };
    // Its speed: the Fat Bandit's 45, the Defender's drawn in [8, 10) (`0x418(8, 10)`: rand × (10 − 8) + 8).
    let speed = if c("speed").is_finite() {
        c("speed")
    } else {
        let r = RNG.lock().ok().and_then(|mut g| g.get_or_insert_with(|| Kh1Rand::new(0x2090_0001)).next_f32().into()).unwrap_or(0.5);
        r * (c("speed_max") - c("speed_min")) + c("speed_min")
    };
    // Its turn (the Blue Rhapsody's Blizzard has none: `sys 18` only, straight).
    let turn = if c("turn").is_finite() { c("turn") } else { 0.0 };
    let n = Numbers { life: c("life"), turn_back: c("life"), speed, accel: 0.0, turn_deg: turn, slide: 0.0, slide_drag: 0.0 };
    let flight = Thrown::aimed(crate::units::to_kh1(at), dir, n);
    let id = c("recom_id") as i32;
    // Re:CoM's own fireball's power: `0x14017F0E0(enemy, 1, 1, 0)` (no kind out, flag 0), `0x14017F050(enemy, 1)`.
    let ap = func::<ApFn>(f::ATTACK_AP)(en, id, c("kind") as i32, 0, std::ptr::null_mut(), 0) as i32;
    let attr = func::<AttrFn>(f::ATTACK_ATTR)(en, id);
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
    let w = |o: usize| dp.add(o);
    // The Bouncywild's banana: Re:CoM's slip kind, as its own banana's descriptor (`0x14018C924`); the others keep the
    // init's kind.
    let slips = p == "fire" && SLIPS.contains(&class);
    if slips {
        std::ptr::write_unaligned(w(0x68) as *mut u32, crate::ground::bouncywild::SLIP_KIND);
    }
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x60) as *mut u64, FLAGS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, flight.speed * crate::units::METRES_PER_UNIT);
    let life = crate::units::frames_to_units(flight.n.life);
    std::ptr::write_unaligned(w(0x38) as *mut i32, life);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, life);
    std::ptr::write_unaligned(w(0x34) as *mut f32, RANGE);
    std::ptr::write_unaligned(w(0x44) as *mut i32, 0);
    std::ptr::write_unaligned(w(0x48) as *mut f32, GATE);
    // Re:CoM's homing rate, used only once Sora's Guard turned it back (decision 28): KH1's own turn (the Fat Bandit's 0,
    // the Defender's 1° a frame).
    std::ptr::write_unaligned(w(0x4C) as *mut f32, flight.n.turn_deg);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [at.x, at.y, at.z, 0.5]);
    let v = crate::units::to_recom(flight.dir);
    let (pitch, ryaw) = crate::guard::aim_angles([v.x, v.y, v.z]).unwrap_or((0.0, crate::units::yaw_to_recom(yaw)));
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [pitch, ryaw, 0.0, 0.0]);
    match fire_look(class, p) {
        Some((set, places)) => {
            std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), set));
            if let Some(p) = places {
                std::ptr::write_unaligned(w(0xB8) as *mut usize, va(base(), p));
            }
        }
        None => line(|| format!("{class}: its fireball effect set is not where the exe had it: the fireball has no look")),
    }
    std::ptr::write_unaligned(w(0x78) as *mut f32, c("radius") * crate::units::METRES_PER_UNIT);
    std::ptr::write_unaligned(w(0x94) as *mut i32, FADE);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        line(|| format!("{class}: no entity factory: no fireball"));
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, en);
    if bullet == 0 {
        line(|| format!("{class}: Re:CoM's shot manager made no fireball"));
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    let bid: u32 = rd(bullet + 0x20);
    let b = base();
    let vt: usize = rd(bullet);
    let exe: usize = rd(bullet + rt::STATE_EXE);
    let adj: i32 = rd(bullet + rt::STATE_EXE_ADJUST);
    if vt != va(b, gb::HOMING_VTABLE) || exe != b + HOMING_THUNK as usize || adj != 0 {
        line(|| format!("{class}: fireball {bid} is not a plain CHomingBullet in flight (vt {vt:#x}, exe {exe:#x}): Re:CoM's straight flight"));
        return Some(bid);
    }
    let shooter: u32 = rd(en + 0x20);
    if let Ok(mut g) = BLADES.lock() {
        g.retain(|b| b.id != bid);
        if g.len() >= MAX {
            g.remove(0);
        }
        g.push(Blade { kind: Kind::Fire, prefix: p, bat_radius: Some(c("bat_radius")).filter(|r| r.is_finite()), guard_bats: GUARD_BATS.contains(&class), ends_on_clash: ENDS_ON_CLASH.contains(&class), batted: false, home: None, id: bid, shooter, flight, turned: false, slips });
        FLYING.store(true, Ordering::Relaxed);
    }
    wr(bullet + rt::STATE_EXE, blade_exe as *const () as usize);
    let dd = flight.dir;
    line(|| format!("{class} e{shooter}: {p} {bid} from ({:.2},{:.2},{:.2}) along ({:.3},{:.3},{:.3}) (KH1's aim at {:?}), {:.2} a frame turning {}° for {} frames, AP {ap} attribute {attr} (Re:CoM's shot, id {id}){}, radius {:.2} m", at.x, at.y, at.z, dd.x, dd.y, dd.z, target.map(|t| (t.x as i32, t.y as i32, t.z as i32)), flight.n.speed, flight.n.turn_deg, flight.n.life, if slips { ", Re:CoM's slip (kind 0x18, no Moogle points lost)" } else { "" }, c("radius") * 0.01));
    Some(bid)
}

/// The blade's flight-state exe: KH1's flight this frame ([`Thrown::step`] from where Re:CoM put it,
/// toward Sora's aim point), its velocity (m per 60 Hz frame: KH1's displacement ÷ step) and look
/// angles, no Re:CoM homing target; then Re:CoM's own step (the thunk). Once Sora's Guard turned it
/// back (its collision now on the enemies' side, `shape+100 & 0xE == 0`), Re:CoM's homing takes it back
/// at the thrower.
pub unsafe extern "win64" fn blade_exe(bullet: usize) {
    let mover: usize = rd(bullet + gb::MOVER);
    let live = rd::<i32>(bullet + rt::HOLD) == 0 && rd::<i32>(bullet + rt::LIFE) > 0 && mover != 0;
    let id: u32 = rd(bullet + 0x20);
    if live {
        let coll: usize = rd(bullet + gb::COLL);
        let shape: usize = if coll != 0 { rd(coll + crate::guard::coll::SHAPE) } else { 0 };
        let turned = shape != 0 && rd::<u32>(shape + crate::guard::coll::SHAPE_LIST_FLAGS) & 0xE == 0;
        let target = if turned { None } else { sora_target() };
        let dt: i32 = rd(bullet + 0x14);
        let step = crate::units::step_from_dt(dt.clamp(0, 1000));
        let mut log_turn = None;
        let mut note = None;
        let mut end = false;
        if let Ok(mut g) = BLADES.lock() {
            if let Some(b) = g.iter_mut().find(|b| b.id == id) {
                if turned && b.ends_on_clash {
                    // Sora's Guard met it (Re:CoM's block turned it): KH1's child ends at the clash ([`ENDS_ON_CLASH`]).
                    end_on_clash(bullet);
                    note = Some(format!("{} {id}: Sora's Guard met it: it ends (KH1's clash)", b.prefix));
                    end = true;
                } else if turned && !b.turned && !b.batted && b.guard_bats && b.kind == Kind::Fire {
                    // Sora's Guard blocked it (Re:CoM's block: its list now the enemies'): KH1's re-aim, as a swing
                    // ([`GUARD_BATS`]). It flies on as a batted fire from this frame.
                    note = Some(reaim_fire(b, bullet, "Guard"));
                }
                // A fireball turns toward Sora's aim point at its own rate (the Fat Bandit's 0: straight; the Defender's 1°);
                // a batted one toward its new target (`0x1B8`'s `hp[0x30]`) while that lives (`sys 120`), on the enemies' side.
                if end {
                } else if b.batted {
                    if step > 0.0 {
                        b.flight.pos = crate::units::to_kh1(rd_vec(mover + rt::POSITION));
                        let home = b.home.and_then(|h| alive_aim(h));
                        if home.is_none() {
                            b.home = None;
                        }
                        let d = b.flight.step(home, step);
                        let v = crate::units::to_recom(d * (1.0 / step));
                        wr(mover + gb::VELOCITY, [v.x, v.y, v.z, 0.0f32]);
                        if let Some((p, y)) = crate::guard::aim_angles([v.x, v.y, v.z]) {
                            wr(mover + gb::PITCH, p);
                            wr(mover + gb::YAW, y);
                        }
                        wr(bullet + gb::HOMING_TARGET, 0i32);
                        end = b.flight.ended();
                    }
                } else if turned {
                    if !b.turned {
                        b.turned = true;
                        wr(bullet + gb::HOMING_TARGET, b.shooter as i32);
                        log_turn = Some(b.shooter);
                    }
                } else if step > 0.0 {
                    b.flight.pos = crate::units::to_kh1(rd_vec(mover + rt::POSITION));
                    let d = b.flight.step(target, step);
                    let v = crate::units::to_recom(d * (1.0 / step));
                    wr(mover + gb::VELOCITY, [v.x, v.y, v.z, 0.0f32]);
                    if let Some((p, y)) = crate::guard::aim_angles([v.x, v.y, v.z]) {
                        wr(mover + gb::PITCH, p);
                        wr(mover + gb::YAW, y);
                    }
                    wr(bullet + gb::HOMING_TARGET, 0i32);
                    end = b.flight.ended();
                }
            }
            if end {
                g.retain(|b| b.id != id);
                if g.is_empty() {
                    FLYING.store(false, Ordering::Relaxed);
                }
            }
        }
        if end {
            // KH1's life is over (`sys 97`): Re:CoM's life ends it (flag 4) on its next step.
            wr(bullet + rt::LIFE, 0i32);
        }
        if let Some(s) = log_turn {
            line(|| format!("blade {id}: Sora's Guard turned it back: Re:CoM's homing takes it at e{s}"));
        }
        if let Some(n) = note {
            line(|| n);
        }
    }
    func::<PlFn>(HOMING_THUNK)(bullet)
}

/// An enemy's aim point (KH1 units) while it is there with HP (KH1's `sys 120` / `sys 119` / `sys 32`: Sora's target
/// `player+0x84C`, a fireball's thrower), else None.
unsafe fn alive_aim(h: u32) -> Option<Vec3> {
    if h == 0 {
        return None;
    }
    let o = func::<super::movement::HandleFn>(super::movement::HANDLE_OBJ)(h);
    if o == 0 || rd::<usize>(o) == 0 || rd::<i32>(o + crate::guard::enemy::HP) <= 0 {
        return None;
    }
    let p = super::magic::aim_point(o);
    Some(crate::units::to_kh1(Vec3::new(p[0], p[1], p[2])))
}

/// The thrower of fireball `id` once it is on the enemies' side (batted by Sora's swing or turned back by his Guard), for
/// the enemy-hit wrapper (`super::kh1_enemy`, the Defender's own-row test: KH1's `sys 188` of its batted row `0x31C`).
pub(super) fn own_shot(id: u32) -> Option<u32> {
    if !FLYING.load(Ordering::Relaxed) {
        return None;
    }
    let g = BLADES.try_lock().ok()?;
    g.iter().find(|b| b.id == id && b.kind == Kind::Fire && (b.batted || b.turned)).map(|b| b.shooter)
}

/// Whether shot `id` is one of our fires carrying Re:CoM's slip ([`SLIPS`]: the Bouncywild's banana), for the damage
/// check's slip without Moogle points (`super::kh1_peel::before_damage`). Kept after Re:CoM's hit ends it (the entry
/// stays until the battle's end or [`MAX`] newer throws).
/// Called only for a pending slip (rare): `FLYING` is not consulted, the entry outlives the flight.
pub(super) fn slip_shot(id: u32) -> bool {
    BLADES.try_lock().map(|g| g.iter().any(|b| b.id == id && b.slips)).unwrap_or(false)
}

/// The battle ended (or a new one began): no blade of the last one is ours any more.
pub(super) fn forget() {
    if let Ok(mut g) = BLADES.lock() {
        g.clear();
    }
    FLYING.store(false, Ordering::Relaxed);
    if let Ok(mut g) = BALLS.lock() {
        g.clear();
    }
    BALLS_ANY.store(false, Ordering::Relaxed);
    forget_bolts();
    super::kh1_peel::forget();
}

/// Sora's live KH1 swing against the flying blades (`meets(shape)`: the swing's spheres against a
/// Re:CoM collision shape, `win::attack::pair_dispatch`). A blade it meets is knocked away as KH1's
/// child script does on its clash bit (module doc): turned back with KH1's random tilt, its attack
/// off, sliding 60 frames at × 0.9, then ended. Blades Sora's Guard turned back are left to Re:CoM.
pub(super) unsafe fn swing_at_blades(meets: impl Fn(usize) -> bool) {
    if !FLYING.load(Ordering::Relaxed) {
        return;
    }
    let Ok(mut g) = BLADES.try_lock() else { return };
    let b0 = base();
    for b in g.iter_mut().filter(|b| !b.turned && !b.batted && b.flight.phase == Phase::Flying) {
        let bullet = func::<super::movement::HandleFn>(super::movement::HANDLE_OBJ)(b.id);
        // Still our blade in flight (the handle may have been reused after Re:CoM ended it).
        if bullet == 0 || rd::<usize>(bullet) != va(b0, gb::HOMING_VTABLE) || rd::<usize>(bullet + rt::STATE_EXE) != blade_exe as *const () as usize || rd::<i32>(bullet + rt::LIFE) <= 0 {
            continue;
        }
        let coll: usize = rd(bullet + gb::COLL);
        let shape: usize = if coll != 0 { rd(coll + crate::guard::coll::SHAPE) } else { 0 };
        if shape == 0 || !meets(shape) {
            continue;
        }
        if b.ends_on_clash {
            // KH1's child ends at the clash (the Blue Rhapsody's Blizzard: no re-aim, [`ENDS_ON_CLASH`]).
            end_on_clash(bullet);
            b.flight.phase = Phase::Ended;
            let mover: usize = rd(bullet + gb::MOVER);
            if mover != 0 {
                let p = rd_vec(mover + rt::POSITION);
                super::attack::spark_at([p.x, p.y, p.z, 1.0]);
            }
            let (id, p) = (b.id, b.prefix);
            line(|| format!("{p} {id}: Sora's swing met it: it ends (KH1's clash)"));
            continue;
        }
        if b.kind == Kind::Fire {
            // KH1's projectile clash: sent to the swinger's side and aimed anew (`0x1B8`): at Sora's target while it
            // lives, else at its thrower (`sys 32` of the parent: its lock point), straight, its whole life again.
            if !super::guard::send_back(bullet) {
                continue;
            }
            let n = reaim_fire(b, bullet, "swing");
            let mover: usize = rd(bullet + gb::MOVER);
            if mover != 0 {
                let p = rd_vec(mover + rt::POSITION);
                super::attack::spark_at([p.x, p.y, p.z, 1.0]);
            }
            line(|| n);
            continue;
        }
        let r = {
            let mut rng = RNG.lock().ok();
            let r = rng.as_mut().map(|o| o.get_or_insert_with(|| Kh1Rand::new(0x2090_0001)));
            match r {
                Some(r) => [r.next_f32(), r.next_f32(), r.next_f32()],
                None => [0.0, 0.5, 0.5],
            }
        };
        b.flight.repel(r);
        super::magic::hold_off(bullet);
        // KH1's slide restarts its clock (`hp[0x50] = 0`): Re:CoM's life must outlast it (our step
        // ends it after the slide).
        let slide = crate::units::frames_to_units(b.flight.n.slide + 2.0);
        if rd::<i32>(bullet + rt::LIFE) < slide {
            wr(bullet + rt::LIFE, slide);
        }
        let mover: usize = rd(bullet + gb::MOVER);
        if mover != 0 {
            let p = rd_vec(mover + rt::POSITION);
            super::attack::spark_at([p.x, p.y, p.z, 1.0]);
        }
        let (id, d) = (b.id, b.flight.dir);
        line(|| format!("blade {id}: Sora's swing knocked it away: back along ({:.2},{:.2},{:.2}), its attack off, a 60-frame slide (KH1's clash)", d.x, d.y, d.z));
    }
}

/// A shot KH1's child ends at a clash ([`ENDS_ON_CLASH`]): its attack off (Re:CoM's own off) and its life over, so Re:CoM's
/// step ends it (flag 4) before it can hit anything again.
unsafe fn end_on_clash(bullet: usize) {
    super::magic::hold_off(bullet);
    wr(bullet + rt::LIFE, 0i32);
}

/// KH1's projectile clash on a fire already on the enemies' side (Sora's swing through `super::guard::send_back`, or his
/// Guard's block for a [`GUARD_BATS`] class): aimed anew (`0x1B8`, the Red Nocturne's `0x288`: the same bytes) at Sora's
/// target while it lives, else at its thrower, its whole life again, turning toward that target; KH1's batted row's radius
/// when the manifest gives it (Re:CoM's radius setter). The log line.
unsafe fn reaim_fire(b: &mut Blade, bullet: usize, by: &str) -> String {
    let mover: usize = rd(bullet + gb::MOVER);
    if mover != 0 {
        b.flight.pos = crate::units::to_kh1(rd_vec(mover + rt::POSITION));
    }
    // `0x1B8`: the way to its thrower (`sys 32`) while it lives, else the swinger's facing (Sora's back along the
    // flight: Assumption); with Sora's target alive, its new target is that one and, for a turning fireball, the
    // way is half-way between the two (normalised sum), else straight at it; else its thrower stays the target.
    let to = |t: Vec3| (t - b.flight.pos).normalized();
    let shooter_aim = alive_aim(b.shooter);
    let mut dir = shooter_aim.map(to).unwrap_or(-b.flight.dir);
    let pl: usize = rd(va(base(), crate::sites::data::MAIN_PLAYER));
    let sora_t: u32 = if pl != 0 { rd(pl + 0x84C) } else { 0 };
    match alive_aim(sora_t).filter(|_| sora_t != 0) {
        Some(t) => {
            dir = if b.flight.n.turn_deg == 0.0 { to(t) } else { (dir + to(t)).normalized() };
            b.home = Some(sora_t);
        }
        None => b.home = shooter_aim.map(|_| b.shooter),
    }
    if dir.length() < 1e-3 {
        dir = -b.flight.dir;
    }
    b.flight.retarget(dir);
    b.batted = true;
    let life = crate::units::frames_to_units(b.flight.n.life + 2.0);
    wr(bullet + rt::LIFE, life);
    wr(bullet + gb::HOMING_TARGET, 0i32);
    let coll: usize = rd(bullet + gb::COLL);
    if let Some(r) = b.bat_radius.filter(|_| coll != 0) {
        func::<RadiusFn>(SET_RADIUS)(coll, r * crate::units::METRES_PER_UNIT);
    }
    let (id, d, p) = (b.id, b.flight.dir, b.prefix);
    format!("{p} {id}: Sora's {by} bats it back along ({:.2},{:.2},{:.2}) (KH1's projectile clash), on the enemies' side{}", d.x, d.y, d.z, b.bat_radius.map_or(String::new(), |r| format!(", radius {:.2} m", r * 0.01)))
}

/// The trace's token: the blades flying (id, frames, speed, turned back).
pub fn trace_token() -> String {
    let Ok(g) = BLADES.try_lock() else { return String::new() };
    let mut t: Vec<String> = g.iter().map(|b| format!("{} {}:{:.0}:{:.1}{}", b.prefix, b.id, b.flight.time, b.flight.speed, if b.turned { ":back" } else if b.batted { ":batted" } else { "" })).collect();
    drop(g);
    if let Ok(g) = BALLS.try_lock() {
        t.extend(g.iter().map(|b| format!("ball e{}:{}:{:?}:{:.0}:{:.0}{}", b.shooter, b.bullet.unwrap_or(0), b.ball.phase, b.ball.time, b.ball.bounces, if b.guarded { ":guarded" } else if b.ball.batted { ":batted" } else { "" })));
    }
    if let Ok(g) = BOLTS.try_lock() {
        t.extend(g.iter().map(|b| format!("thunder e{}:{}:{}", b.shooter, b.id, b.frames)));
    }
    let p = super::kh1_peel::trace_token();
    if !p.is_empty() {
        t.push(p);
    }
    t.join(" ")
}

// ---------------------------------------------------------------------------------------------------------------------
// M11, the Gargoyle's ball (`ex_2220_00.bd`, `kh1_core::gargoyle::Ball`; owner 2026-11-02: "Re:CoM bullet, KH1
// numbers").
//
// - **KH1** (Checked (files + decomp)): `sys 94` makes the child with 219; it waits 70 frames, then shows its model at
//   the parent's chin point 15 up for 160 frames, eases it toward bone code 28 (the left hand) for 14, then makes its
//   attack (row `0x3FC`: radius 30, priority 0) and flies: tossed up along the parent's facing at 10 a frame, falling
//   0.5 a frame faster each frame; each floor contact re-aims it at its target's lock point (10..20 a frame by the
//   distance, tilted up by the height gap); the third contact ends it; a hit ends it. Before the launch it ends when the
//   parent no longer plays 219 (a hit's reaction). A swing that meets it sends it back (`0x748`: owned by the swinger
//   with row `0x46C`, radius 60, aimed between the parent and the swinger's target, one more flight).
// - **Here**: one of Re:CoM's own `CHomingBullet`s (as the Fat Bandit's fireball), made when KH1's model appears, at the
//   chin point, its attack held off (`CCollAttack` off) until KH1's launch (on: `sites::COLL_ON_PTRS`), its life KH1's
//   charge + ease + flight; its power Re:CoM's own Gargoyle shot's (`0x14017F0E0(enemy, 1, 1, 0)`, `0x14018E350`), its
//   look that shot's effect set `0x14053F630` (`sites::GARGOYLE_BALL_PTRS`). Its flight-state exe ([`ball_exe`]) steps
//   KH1's child and writes the velocity that puts it on KH1's place (kept 2 cm above the battle floor: Re:CoM's mover
//   must not touch it, flag 0x100 would end it; Assumption ±2 cm), then Re:CoM's own step collides it and hits Sora
//   (a hit ends it). The 70-frame wait and the parent's place come from the parent's update ([`ball_parent_step`]).
// - His swing bats it (KH1's `0x748`, [`swing_at_balls`]: Guard's turn-back to the enemies' side, radius 60, KH1's re-aim
//   and second flight); **so does his Guard** (owner 2026-11-03: "Sora's Guard sends the Gargoyle's ball back KH1's
//   way"): KH1's guard row is a clash object of type 2 that meets the ball's type-0 row through the same
//   `0x1402B4BC0` as a swing (the ball, priority 0 and free-standing, is handed to Sora and gets `+4 |= 0x10 / 0x20`;
//   Checked (decomp)), and the child reads only `+4 & 0x30` before `0x748` (Checked (files)). Here Re:CoM's block
//   (`super::guard`'s turn-back, its enemies'-side list) is seen by [`ball_exe`], which sends it back as the swing does
//   ([`send_back_kh1`]). No Re:CoM homing.
//
// Per frame: one atomic load while no ball lives; one KH1 step per ball.

/// At most this many balls (older ones are forgotten: Re:CoM's own step flies them).
const MAX_BALLS: usize = 8;
/// The ball's request (`ex_2220_00.bd` checks 219 after each yield before its launch).
const BALL_REQUEST: u16 = kh1_core::gargoyle::req::BALL;
/// The ball's mover radius (`0x140157DA0`'s second word: its background shape; Re:CoM's shots pass 0.1): small, so the
/// ball can come down to 2 cm over the floor before KH1's contact without Re:CoM's mover touching it.
const BALL_MOVER_RADIUS: f32 = 0.01;
/// KH1 units above the battle floor the ball is kept (Re:CoM's mover radius 1 cm + 1 cm).
const BALL_FLOOR_CLEAR: f32 = 2.0;
/// Re:CoM's collision radius setter (`0x140172690`; `sites::FLAME_CALLEES`).
const SET_RADIUS: u32 = 0x0017_2690;
type RadiusFn = unsafe extern "win64" fn(usize, f32);

struct BallRec {
    /// The Gargoyle (its address and Re:CoM id).
    en: usize,
    shooter: u32,
    class: &'static str,
    ball: kh1_core::gargoyle::Ball,
    /// Its bullet once KH1's model appears.
    bullet: Option<u32>,
    /// From the parent's update: it plays 219, its points (KH1 units), its facing, the battle floor; frames since.
    parent_plays: bool,
    hold: Vec3,
    at: Vec3,
    facing: Vec3,
    floor: Option<f32>,
    stale: u32,
    /// The parent stepped it this frame (its bullet was just made): the exe skips one step.
    skip: bool,
    ap: i32,
    attr: u32,
    /// Its row's radius and the batted row's (KH1 units: 30, 60).
    radius: f32,
    bat_radius: f32,
    /// Sora's Guard sent it back (KH1's `0x748`, as a swing): for the log and the trace.
    guarded: bool,
    /// After a bat: KH1's new target (Sora's target, or the Gargoyle).
    bat_target: u32,
}

static BALLS: Mutex<Vec<BallRec>> = Mutex::new(Vec::new());
static BALLS_ANY: AtomicBool = AtomicBool::new(false);

/// `sys 94(actor, 0, target, 0x3FC, 0x46C)`: the Gargoyle `en` makes its ball (KH1's child: nothing for its first 70
/// frames).
unsafe fn ball_start(en: usize, class: &str, m: &Manifest) {
    let c = |k: &str| m.consts.get(k);
    let id = c("ball_recom_id") as i32;
    let ap = func::<ApFn>(f::ATTACK_AP)(en, id, c("ball_kind") as i32, 0, std::ptr::null_mut(), 0) as i32;
    let attr = func::<AttrFn>(f::ATTACK_ATTR)(en, id);
    let shooter: u32 = rd(en + 0x20);
    let class: &'static str = crate::kh1_enemy::CLASSES.iter().find(|k| k.name == class).map_or("gargoyle", |k| k.name);
    let ball = kh1_core::gargoyle::Ball::new(kh1_core::gargoyle::BallNumbers::from(&m.consts));
    if let Ok(mut g) = BALLS.lock() {
        if g.len() >= MAX_BALLS {
            g.remove(0);
        }
        g.push(BallRec { en, shooter, class, ball, bullet: None, parent_plays: true, hold: Vec3::ZERO, at: Vec3::ZERO, facing: Vec3::ZERO, floor: None, stale: 0, skip: false, ap, attr, radius: c("ball_radius"), bat_radius: c("ball_bat_radius"), guarded: false, bat_target: 0 });
        BALLS_ANY.store(true, Ordering::Relaxed);
    }
    line(|| format!("{class} e{shooter}: its ball (KH1's child: {} frames, then held {} at its chin, eased {}, flown {} frames), AP {ap} attribute {attr} (Re:CoM's Gargoyle shot, id {id})", c("ball_wait"), c("ball_charge"), c("ball_ease"), c("ball_life")));
}

/// From the Gargoyle's update each frame (`win::kh1_enemy::tick`): its balls' parent inputs (it plays 219 and lives, its
/// chin and hand points, its facing, the battle floor), and the waiting balls' steps (no bullet yet).
pub(super) unsafe fn ball_parent_step(en: usize, m: &Manifest, playing: Option<u16>, live: bool, yaw: f32, floor: Option<f32>, step: f32) {
    if !BALLS_ANY.load(Ordering::Relaxed) {
        return;
    }
    let Ok(mut g) = BALLS.try_lock() else { return };
    // Balls Re:CoM has ended (a hit, a wall): forgotten (their exe no longer runs).
    g.retain(|b| {
        b.bullet.is_none_or(|id| {
            let o = func::<super::movement::HandleFn>(super::movement::HANDLE_OBJ)(id);
            o != 0 && rd::<usize>(o + rt::STATE_EXE) == ball_exe as *const () as usize && rd::<i32>(o + rt::LIFE) > 0
        })
    });
    if g.is_empty() {
        BALLS_ANY.store(false, Ordering::Relaxed);
        return;
    }
    if !g.iter().any(|b| b.en == en) {
        return;
    }
    let hold = manifest_point(en, m, "ball_hold").map(crate::units::to_kh1);
    let at = manifest_point(en, m, "ball_at").map(crate::units::to_kh1);
    let mut ended = Vec::new();
    for b in g.iter_mut().filter(|b| b.en == en) {
        b.parent_plays = live && playing == Some(BALL_REQUEST);
        b.stale = 0;
        if let Some(h) = hold {
            b.hold = h;
        }
        if let Some(a) = at {
            b.at = a;
        }
        b.facing = Vec3::from_yaw(yaw);
        if floor.is_some() {
            b.floor = floor;
        }
        if b.bullet.is_some() || b.ball.ended() {
            continue;
        }
        let w = kh1_core::gargoyle::BallWorld { step, parent_plays: b.parent_plays, hold: b.hold, at: b.at, facing: b.facing, target: None, floor: b.floor };
        match b.ball.step(&w) {
            kh1_core::gargoyle::BallEvent::Appears => {
                b.bullet = ball_bullet(b);
                b.skip = true;
                if b.bullet.is_none() {
                    ended.push(b.shooter);
                }
            }
            kh1_core::gargoyle::BallEvent::Ended => ended.push(b.shooter),
            _ => {}
        }
    }
    g.retain(|b| !(b.en == en && b.bullet.is_none() && (b.ball.ended() || ended.contains(&b.shooter))));
    if g.is_empty() {
        BALLS_ANY.store(false, Ordering::Relaxed);
    }
    drop(g);
    for s in ended {
        line(|| format!("ball e{s}: ends before it appears (219 {})", if playing == Some(BALL_REQUEST) { "plays" } else { "no longer plays" }));
    }
}

/// The ball's bullet: a `CHomingBullet` as the Fat Bandit's fireball, at KH1's place, its attack held off.
unsafe fn ball_bullet(b: &BallRec) -> Option<u32> {
    let at = crate::units::to_recom(b.ball.pos);
    let n = b.ball.n;
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, b.attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, b.ap);
    std::ptr::write_unaligned(w(0x60) as *mut u64, FLAGS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, 0.0);
    let life = crate::units::frames_to_units(n.charge + n.ease + n.life + 4.0);
    std::ptr::write_unaligned(w(0x38) as *mut i32, life);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, life);
    std::ptr::write_unaligned(w(0x34) as *mut f32, RANGE);
    std::ptr::write_unaligned(w(0x44) as *mut i32, 0);
    std::ptr::write_unaligned(w(0x48) as *mut f32, GATE);
    std::ptr::write_unaligned(w(0x4C) as *mut f32, 0.0);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [at.x, at.y, at.z, 0.5]);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [0.0, crate::units::yaw_to_recom(b.facing.yaw()), 0.0, 0.0]);
    if crate::sites::GARGOYLE_BALL_PTRS.iter().all(|p| rd::<usize>(base() + p.rva as usize) == base() + p.target as usize) {
        std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), BALL_EFFECT_SET));
    } else {
        line(|| format!("{}: the Gargoyle's shot effect set is not where 0x14053F630 was: the ball has no look", b.class));
    }
    let radius = b.radius * crate::units::METRES_PER_UNIT;
    std::ptr::write_unaligned(w(0x78) as *mut f32, radius);
    std::ptr::write_unaligned(w(0x94) as *mut i32, FADE);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        line(|| "no entity factory: no ball".into());
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, b.en);
    if bullet == 0 {
        line(|| "Re:CoM's shot manager made no ball".into());
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, BALL_MOVER_RADIUS, 0.0, 0, -1);
    let bid: u32 = rd(bullet + 0x20);
    let bs = base();
    let vt: usize = rd(bullet);
    let exe: usize = rd(bullet + rt::STATE_EXE);
    let adj: i32 = rd(bullet + rt::STATE_EXE_ADJUST);
    if vt != va(bs, gb::HOMING_VTABLE) || exe != bs + HOMING_THUNK as usize || adj != 0 {
        line(|| format!("ball {bid} is not a plain CHomingBullet in flight (vt {vt:#x}, exe {exe:#x}): ended"));
        wr(bullet + rt::LIFE, 0i32);
        return None;
    }
    // KH1 makes its attack only at the launch: held off until then.
    super::magic::hold_off(bullet);
    wr(bullet + rt::STATE_EXE, ball_exe as *const () as usize);
    let s = b.shooter;
    line(|| format!("ball e{s}: appears as shot {bid} at ({:.2},{:.2},{:.2}), its attack off until the launch, radius {radius:.2} m", at.x, at.y, at.z));
    Some(bid)
}

/// The Gargoyle's own shot's effect set (`0x14018E350`'s descriptor `+0xA8`; `sites::GARGOYLE_BALL_PTRS`).
pub const BALL_EFFECT_SET: u64 = 0x1_4053_F630;

/// `CCollAttack`'s "on" (vtable +0x10, `0x14014DE90`; `sites::COLL_ON_PTRS`): the ball's attack at KH1's launch (the
/// Bouncywild's peels' once they lie).
pub(super) unsafe fn hold_on(bullet: usize) -> bool {
    let c: usize = rd(bullet + gb::COLL);
    if c == 0 {
        return false;
    }
    let on = &crate::sites::COLL_ON_PTRS[0];
    let vt: usize = rd(c);
    if rd::<usize>(vt + 0x10) != base() + on.target as usize {
        return false;
    }
    func::<PlFn>(on.target)(c);
    true
}

/// An enemy's aim point (Re:CoM's vtable `+0x50`, KH1's lock point 0) while it is there with HP left, KH1 units.
unsafe fn enemy_aim(id: u32) -> Option<Vec3> {
    if id == 0 {
        return None;
    }
    let o = func::<super::movement::HandleFn>(super::movement::HANDLE_OBJ)(id);
    if o == 0 || rd::<usize>(o) == 0 || rd::<i32>(o + crate::guard::enemy::HP) <= 0 {
        return None;
    }
    let p = super::magic::aim_point(o);
    Some(crate::units::to_kh1(Vec3::new(p[0], p[1], p[2])))
}

/// The ball's flight-state exe: KH1's child this frame ([`kh1_core::gargoyle::Ball::step`]), the velocity that puts the
/// bullet on KH1's place (kept 2 cm over the floor) and its look angles; then Re:CoM's own step (the thunk). A flying
/// ball Sora's Guard blocked (Re:CoM's block moved it to the enemies' side) is sent back as a swing sends it (KH1's
/// `0x748`, [`send_back_kh1`]) before this frame's step.
pub unsafe extern "win64" fn ball_exe(bullet: usize) {
    use kh1_core::gargoyle::{BallEvent, BallWorld};
    let mover: usize = rd(bullet + gb::MOVER);
    let live = rd::<i32>(bullet + rt::HOLD) == 0 && rd::<i32>(bullet + rt::LIFE) > 0 && mover != 0;
    let id: u32 = rd(bullet + 0x20);
    if live {
        let coll: usize = rd(bullet + gb::COLL);
        let shape: usize = if coll != 0 { rd(coll + crate::guard::coll::SHAPE) } else { 0 };
        let turned = shape != 0 && rd::<u32>(shape + crate::guard::coll::SHAPE_LIST_FLAGS) & 0xE == 0;
        let dt: i32 = rd(bullet + 0x14);
        let step = crate::units::step_from_dt(dt.clamp(0, 1000));
        let mut note: Option<String> = None;
        let mut end = false;
        if let Ok(mut g) = BALLS.lock() {
            if let Some(b) = g.iter_mut().find(|b| b.bullet == Some(id)) {
                if turned && !b.ball.batted && b.ball.flying() {
                    // Sora's Guard blocked it (Re:CoM's block: its list now the enemies'): KH1's `0x748`, as a swing.
                    // (A batted ball is on the enemies' side by our own turn-back and flies on.)
                    b.guarded = true;
                    let n = send_back_kh1(b, bullet, "Guard");
                    line(|| n);
                }
                if step > 0.0 {
                    if b.skip {
                        b.skip = false;
                    } else {
                        b.stale += 1;
                        if b.stale > 3 {
                            // The parent no longer updates (removed): KH1's check of 219 fails.
                            b.parent_plays = false;
                        }
                        let target = if b.ball.batted { enemy_aim(b.bat_target) } else { sora_target() };
                        let w = BallWorld { step, parent_plays: b.parent_plays, hold: b.hold, at: b.at, facing: b.facing, target, floor: b.floor };
                        match b.ball.step(&w) {
                            BallEvent::Launched => {
                                let on = hold_on(bullet);
                                let d = b.ball.dir;
                                note = Some(format!("ball {id}: launched along ({:.2},{:.2},{:.2}) at {} a frame, its attack {}", d.x, d.y, d.z, b.ball.speed, if on { "on" } else { "NOT on (CCollAttack's slot +0x10 is not 0x14014DE90)" }));
                            }
                            BallEvent::Bounced => {
                                let (n, sp) = (b.ball.bounces, b.ball.speed);
                                note = Some(format!("ball {id}: floor contact {n}: re-aimed at {:?} at {sp:.1} a frame", target.map(|t| (t.x as i32, t.y as i32, t.z as i32))));
                            }
                            BallEvent::Ended => {
                                end = true;
                                note = Some(format!("ball {id}: ends (KH1: {})", if b.ball.flying() || b.ball.time > 0.0 { "its third contact or its life" } else { "219 no longer plays" }));
                            }
                            _ => {}
                        }
                        if !end {
                            // KH1's place, kept over the floor; the velocity that takes the bullet there this frame.
                            let mut p = b.ball.pos;
                            if let Some(f) = b.floor {
                                p.y = p.y.min(f - BALL_FLOOR_CLEAR);
                            }
                            let to = crate::units::to_recom(p);
                            let now = rd_vec(mover + rt::POSITION);
                            let v = (to - now) * (1.0 / step);
                            wr(mover + gb::VELOCITY, [v.x, v.y, v.z, 0.0f32]);
                            if v.length() > 1e-4 {
                                if let Some((pt, y)) = crate::guard::aim_angles([v.x, v.y, v.z]) {
                                    wr(mover + gb::PITCH, pt);
                                    wr(mover + gb::YAW, y);
                                }
                            }
                            wr(bullet + gb::HOMING_TARGET, 0i32);
                        }
                    }
                }
            }
            if end {
                g.retain(|b| b.bullet != Some(id));
                if g.is_empty() {
                    BALLS_ANY.store(false, Ordering::Relaxed);
                }
            }
        }
        if end {
            wr(bullet + rt::LIFE, 0i32);
        }
        if let Some(n) = note {
            line(|| n);
        }
    }
    func::<PlFn>(HOMING_THUNK)(bullet)
}

/// Sora's live KH1 swing against the flying balls (`meets(shape)`, as [`swing_at_blades`]): a ball it meets is sent back
/// as KH1's `0x748` does (its row priority 0: Sora's swing gets nothing): Guard's turn-back to the enemies' side, its
/// radius KH1's batted row's (60), aimed between the Gargoyle and Sora's target (or the Gargoyle), tossed again for
/// one more flight.
pub(super) unsafe fn swing_at_balls(meets: impl Fn(usize) -> bool) {
    if !BALLS_ANY.load(Ordering::Relaxed) {
        return;
    }
    let Ok(mut g) = BALLS.try_lock() else { return };
    let b0 = base();
    for b in g.iter_mut().filter(|b| !b.ball.batted && b.ball.flying()) {
        let Some(bid) = b.bullet else { continue };
        let bullet = func::<super::movement::HandleFn>(super::movement::HANDLE_OBJ)(bid);
        if bullet == 0 || rd::<usize>(bullet) != va(b0, gb::HOMING_VTABLE) || rd::<usize>(bullet + rt::STATE_EXE) != ball_exe as *const () as usize || rd::<i32>(bullet + rt::LIFE) <= 0 {
            continue;
        }
        let coll: usize = rd(bullet + gb::COLL);
        let shape: usize = if coll != 0 { rd(coll + crate::guard::coll::SHAPE) } else { 0 };
        if shape == 0 || !meets(shape) {
            continue;
        }
        if !super::guard::send_back(bullet) {
            continue;
        }
        let n = send_back_kh1(b, bullet, "swing");
        if mover_of(bullet) != 0 {
            let p = rd_vec(mover_of(bullet) + rt::POSITION);
            super::attack::spark_at([p.x, p.y, p.z, 1.0]);
        }
        line(|| n);
    }
}

unsafe fn mover_of(bullet: usize) -> usize {
    rd(bullet + gb::MOVER)
}

/// KH1's `0x748` on a ball already on the enemies' side (Re:CoM's turn-back: Sora's swing through
/// `super::guard::send_back`, or his Guard's block): its radius KH1's batted row's (60, Re:CoM's radius setter), its KH1
/// place, aimed between the Gargoyle and Sora's target (Re:CoM's `player+0x84C` while it lives) or at the Gargoyle, tossed
/// again for one more flight with its life anew. The log line.
unsafe fn send_back_kh1(b: &mut BallRec, bullet: usize, by: &str) -> String {
    let bid: u32 = rd(bullet + 0x20);
    let coll: usize = rd(bullet + gb::COLL);
    if coll != 0 {
        func::<RadiusFn>(SET_RADIUS)(coll, b.bat_radius * crate::units::METRES_PER_UNIT);
    }
    let mover = mover_of(bullet);
    if mover != 0 {
        let p = crate::units::to_kh1(rd_vec(mover + rt::POSITION));
        if p.y < b.ball.pos.y {
            // Its KH1 place (it may sit 2 cm under the bullet's).
            b.ball.pos = p;
        }
    }
    let parent = enemy_aim(b.shooter);
    let pl: usize = rd(va(base(), crate::sites::data::MAIN_PLAYER));
    let sora_t: u32 = if pl != 0 { rd(pl + 0x84C) } else { 0 };
    let st = enemy_aim(sora_t);
    b.bat_target = if st.is_some() { sora_t } else { b.shooter };
    let back = -b.ball.dir;
    b.ball.bat(parent, st, back);
    wr(bullet + rt::LIFE, crate::units::frames_to_units(b.ball.n.life + 2.0));
    wr(bullet + gb::HOMING_TARGET, 0i32);
    let (d, t) = (b.ball.dir, b.bat_target);
    format!("ball {bid}: Sora's {by} sends it back along ({:.2},{:.2},{:.2}) at e{t} (KH1's 0x748), on the enemies' side, radius {:.2} m", d.x, d.y, d.z, b.bat_radius * 0.01)
}

// ---------------------------------------------------------------------------------------------------------------------
// M11, the Yellow Opera's Thunder (`ex_2130_00.bd`, `kh1_core::caster::Strike`; owner rule: "KH1's Thunder is a strike from
// above at Sora's position: research how KH1 places it and build the nearest native Re:CoM equivalent").
//
// - **KH1** (Checked (files + decomp)): the child marks Sora's place on the floor 80 frames into its parent's 207 and, 30
//   frames later, makes the row `0x23C` free-standing at the mark and tests each opponent once against a cylinder of
//   radius 50 and half-height 128 centred 128 above the mark (`sys 90` → `0x1402B6AF0` → `0x1402B5EF0`, type 2: |Δy| <
//   the two half-heights and the flat distance < the two radii); each one inside gets the row's hit (`sys 60` →
//   `0x1402BF330` → `0x1402BF630`). No clash pass runs for it, so neither a swing nor Sora's Guard meets it (Guard is a
//   type-2 row of the clash pass, `0x1402B4BC0`; the record maker tests no guard).
// - **Re:CoM's own** (its state 2 `0x140196A80`, Checked (decomp)): at clip time 0xD15 of `e005_052` a `CBullet` 4 m above
//   Sora's place (`0x1401F0AB0`), pitched straight down (π/2), 5 m a frame, radius 0.25, reaction kind 0x1C (Re:CoM's
//   damage check takes it as its flinch, kind 1: `0x1401E6DE0`'s switch), its AP `0x14017F0E0(enemy, 0, 1)`, its effect
//   set `0x140557560` (`"_ex_2130"` 11 / 12: KH1's own bolt and hit effects' numbers). Its swept sphere (`+0x10` → `+0x20`)
//   passes down through Sora in one frame.
// - **Here**: that bolt, placed by KH1's numbers: one of Re:CoM's `CHomingBullet`s (as every port's) at KH1's strike frame,
//   at the cylinder's top above the mark (256 above the floor), Re:CoM's own kind, power and effect set, KH1's radius 50,
//   swept down to 2 cm over the floor in its first frame ([`bolt_exe`]: KH1's whole cylinder, its rounded top 50 higher;
//   Re:CoM's mover radius 1 cm so its floor touch can't end it first), its attack off after that frame, then ended.
//   Re:CoM's collision pass hits Sora with it (one hit: its serial). Sora's Guard doesn't meet it ([`unguarded`]: his body
//   guard off for Re:CoM's apply of this attack only); it isn't in [`BLADES`], so no swing meets it.
//
// Per frame: one atomic load in the pair test while no bolt lives; two exe frames per bolt.

/// Re:CoM's own Yellow Opera Thunder's effect set (its state 2 `0x140196A80`: descriptor `+0xA8`; slots 0 / 1 name
/// `"_ex_2130"`, `sites::YELLOWOPERA_THUNDER_PTRS`). Checked (decomp + exe).
pub const YELLOWOPERA_THUNDER_EFFECT_SET: u64 = 0x1_4055_7560;
/// Re:CoM's own bolt's reaction kind (`0x140196A80`: descriptor `+0x68` = 0x1C). Checked (decomp).
pub const THUNDER_KIND: u32 = 0x1C;
/// Re:CoM's own bolt's pitch (`*0x1401A4A00(bullet) = π/2`, straight down). Checked (decomp).
const THUNDER_PITCH: f32 = std::f32::consts::FRAC_PI_2;
/// The bolt's mover radius (Re:CoM's shots pass 0.1; the Gargoyle's ball's 1 cm: its floor touch would end it).
const BOLT_MOVER_RADIUS: f32 = 0.01;

struct Bolt {
    id: u32,
    coll: usize,
    shooter: u32,
    /// KH1 units: the top it starts from, the bottom its first frame sweeps to.
    top: Vec3,
    bottom: Vec3,
    /// Its exe frames so far.
    frames: u32,
}

static BOLTS: Mutex<Vec<Bolt>> = Mutex::new(Vec::new());
static BOLTS_ANY: AtomicBool = AtomicBool::new(false);

/// Whether a Re:CoM attack collision is one of our live bolts (the pair test's question: Sora's Guard doesn't meet it).
/// Bolts Re:CoM has ended (a hit ends one) are forgotten here: their bullet no longer runs [`bolt_exe`] with that
/// collision.
pub(super) unsafe fn unguarded(coll: usize) -> bool {
    if !BOLTS_ANY.load(Ordering::Relaxed) {
        return false;
    }
    let Ok(mut g) = BOLTS.try_lock() else { return false };
    g.retain(|b| {
        let o = func::<super::movement::HandleFn>(super::movement::HANDLE_OBJ)(b.id);
        o != 0 && rd::<usize>(o + rt::STATE_EXE) == bolt_exe as *const () as usize && rd::<usize>(o + gb::COLL) == b.coll && rd::<i32>(o + rt::LIFE) > 0
    });
    if g.is_empty() {
        BOLTS_ANY.store(false, Ordering::Relaxed);
    }
    g.iter().any(|b| b.coll == coll)
}

// The Yellow Opera's warning before its bolt (owner 2026-11-03: "a Re:CoM effect over the marked spot now as a stand-in for
// KH1's warning cloud").
//
// - **KH1** (Checked (files: `ex_2130_00.bd` `0x342..0x3B0`)): at the mark the child plays its model's effect 14 with
//   `sys 74(effect group, 14, 20000, point)` at a point 200 above the mark (the cloud), counts 30 frames, then plays
//   effect 11 (the bolt) the same way at the mark and strikes. The cloud is never removed by the script (its own length).
// - **Re:CoM's own Yellow Opera** shows no warning by code (Checked (decomp): its state 2 begin `0x140196980` plays
//   `e005_052`, its step `0x140196A80` only makes the bolt at clip time 0xD15); its effect set `0x140557560` holds only
//   `"_ex_2130"` 11 (slot 0, played where the shot starts: the bolt) and 12 (slot 1, played on the flying shot,
//   `0x140157C20` / `0x1401577A0`): the same numbers as KH1's bolt and hit effects (KH1's script plays 11 / 12 / 14 from
//   one group, `*(child+12)`). Its motion's own effect events (`EN0005.EET`) are not in the session (Assumption: none).
// - **Here**: Re:CoM's effect-at-actor `0x1401A6B20(yellow opera, "_ex_2130", id, point, 0, 0.0)` (the call Re:CoM's
//   shots use; the actor non-null: it reads its `+0x150` group) at KH1's cloud point (`thunder_cloud` above the mark),
//   first effect 14 (KH1's cloud number; Assumption: Re:CoM's `_ex_2130` pack kept KH1's numbering, as 11 and 12 show),
//   and when Re:CoM's pack has no 14 (the call returns no handle: `0x1402553B0` finds no entry, Checked (decomp)) the set's
//   own 12; the log says which. Ended by Re:CoM's effect end `0x1402585B0(handle, 0.0)` (at once) at the strike, when
//   Re:CoM's Stop / stun cancels the marked Thunder, or at the next warning of another battle. Nothing per frame.

/// Re:CoM's effect at a point with an actor's group (`0x1401A6B20`, `sites::SITES`' "effect at actor").
const EFFECT_AT: u32 = 0x001A_6B20;
/// Re:CoM's effect end by handle (`0x1402585B0(handle, fade)`: fade ≤ 0 ends it now, `0x140254D60`; `sites::THROWN_CALLEES`).
const EFFECT_END: u32 = 0x0025_85B0;
type EffectAtFn = unsafe extern "win64" fn(usize, usize, u32, *const [f32; 4], u64, f32) -> i32;
type EffectEndFn = unsafe extern "win64" fn(i32, f32);
/// KH1's cloud (effect 14 of its model's group), then Re:CoM's own Thunder set's effect 12.
pub const WARN_EFFECTS: [u32; 2] = [14, 12];

struct Warning {
    en: usize,
    child: u8,
    battle: u32,
    handle: i32,
}

static WARNINGS: Mutex<Vec<Warning>> = Mutex::new(Vec::new());

unsafe fn end_warning(w: &Warning) {
    func::<EffectEndFn>(EFFECT_END)(w.handle, 0.0);
}

/// `Cmd::Warn`: enemy `en`'s child `n` marked `at` (KH1 units, the floor): the stand-in for KH1's cloud over it until the
/// strike (section above).
pub(super) unsafe fn warn(en: usize, class: &str, m: &Manifest, n: u8, at: Vec3) {
    let battle = super::battle::battle_number();
    let Ok(mut g) = WARNINGS.lock() else { return };
    // Another battle's: ended (its enemy is gone).
    g.retain(|w| {
        let keep = w.battle == battle;
        if !keep {
            end_warning(w);
        }
        keep
    });
    let ok = crate::sites::YELLOWOPERA_THUNDER_PTRS.iter().all(|p| rd::<usize>(base() + p.rva as usize) == base() + p.target as usize);
    let name: usize = if ok { rd(va(base(), YELLOWOPERA_THUNDER_EFFECT_SET)) } else { 0 };
    if name == 0 || std::slice::from_raw_parts(name as *const u8, 9) != b"_ex_2130\0" {
        line(|| format!("{class}: the Yellow Opera's effect set is not where 0x140557560 was: no warning at the mark"));
        return;
    }
    let up = m.consts.get("thunder_cloud");
    let p = crate::units::to_recom(Vec3::new(at.x, at.y - if up.is_finite() { up } else { 0.0 }, at.z));
    let point = [p.x, p.y, p.z, 1.0];
    let shooter: u32 = rd(en + 0x20);
    for id in WARN_EFFECTS {
        let h = func::<EffectAtFn>(EFFECT_AT)(en, name, id, &point, 0, 0.0);
        if h != 0 {
            if g.len() >= 2 * MAX {
                let w = g.remove(0);
                end_warning(&w);
            }
            g.push(Warning { en, child: n, battle, handle: h });
            line(|| format!("{class} e{shooter}: Thunder {n}'s mark: Re:CoM's \"_ex_2130\" effect {id} at ({:.2},{:.2},{:.2}) as the stand-in for KH1's cloud (effect 14, {up:.0} above the mark), until the strike", p.x, p.y, p.z));
            return;
        }
    }
    line(|| format!("{class} e{shooter}: Thunder {n}'s mark: Re:CoM's \"_ex_2130\" pack made none of effects {WARN_EFFECTS:?}: no warning"));
}

/// Ends enemy `en`'s warnings: its child `n`'s oldest (its strike), or all of them (`None`: Re:CoM's Stop / stun cancelled
/// its marked Thunder).
pub(super) unsafe fn end_warnings(en: usize, n: Option<u8>) {
    let Ok(mut g) = WARNINGS.lock() else { return };
    match n {
        Some(n) => {
            if let Some(i) = g.iter().position(|w| w.en == en && w.child == n) {
                let w = g.remove(i);
                end_warning(&w);
            }
        }
        None => g.retain(|w| {
            let keep = w.en != en;
            if !keep {
                end_warning(w);
            }
            keep
        }),
    }
}

/// `Cmd::Strike`: enemy `en` (class manifest `m`) strikes with its child `n` at KH1's mark `at` (KH1 units): the Yellow
/// Opera's Thunder (module section above). The bullet's id.
pub(super) unsafe fn strike(en: usize, class: &str, m: &Manifest, n: u8, at: Vec3) -> Option<u32> {
    end_warnings(en, Some(n));
    let c = |k: &str| m.consts.get(&format!("thunder_{k}"));
    if !(c("half").is_finite() && n as f32 == m.consts.get("thunder_child")) {
        line(|| format!("{class}: no strike child {n}"));
        return None;
    }
    // The cylinder's top and the floor (KH1 y down).
    let top = Vec3::new(at.x, at.y - (c("mid") + c("half")), at.z);
    let bottom = Vec3::new(at.x, at.y - BALL_FLOOR_CLEAR, at.z);
    let from = crate::units::to_recom(top);
    let id = c("recom_id") as i32;
    let ap = func::<ApFn>(f::ATTACK_AP)(en, id, c("kind") as i32, 0, std::ptr::null_mut(), 0) as i32;
    let attr = func::<AttrFn>(f::ATTACK_ATTR)(en, id);
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x68) as *mut u32, THUNDER_KIND);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x60) as *mut u64, FLAGS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, 0.0);
    let life = crate::units::frames_to_units(4.0);
    std::ptr::write_unaligned(w(0x38) as *mut i32, life);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, life);
    std::ptr::write_unaligned(w(0x34) as *mut f32, RANGE);
    std::ptr::write_unaligned(w(0x44) as *mut i32, 0);
    std::ptr::write_unaligned(w(0x48) as *mut f32, GATE);
    std::ptr::write_unaligned(w(0x4C) as *mut f32, 0.0);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [from.x, from.y, from.z, 0.5]);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [THUNDER_PITCH, 0.0, 0.0, 0.0]);
    if crate::sites::YELLOWOPERA_THUNDER_PTRS.iter().all(|p| rd::<usize>(base() + p.rva as usize) == base() + p.target as usize) {
        std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), YELLOWOPERA_THUNDER_EFFECT_SET));
    } else {
        line(|| format!("{class}: the Yellow Opera's Thunder effect set is not where 0x140557560 was: the bolt has no look"));
    }
    let radius = c("radius") * crate::units::METRES_PER_UNIT;
    std::ptr::write_unaligned(w(0x78) as *mut f32, radius);
    std::ptr::write_unaligned(w(0x94) as *mut i32, FADE);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        line(|| format!("{class}: no entity factory: no Thunder"));
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, en);
    if bullet == 0 {
        line(|| format!("{class}: Re:CoM's shot manager made no Thunder"));
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, BOLT_MOVER_RADIUS, 0.0, 0, -1);
    let bid: u32 = rd(bullet + 0x20);
    let b = base();
    let vt: usize = rd(bullet);
    let exe: usize = rd(bullet + rt::STATE_EXE);
    let adj: i32 = rd(bullet + rt::STATE_EXE_ADJUST);
    if vt != va(b, gb::HOMING_VTABLE) || exe != b + HOMING_THUNK as usize || adj != 0 {
        line(|| format!("{class}: Thunder {bid} is not a plain CHomingBullet in flight (vt {vt:#x}, exe {exe:#x}): ended"));
        super::magic::hold_off(bullet);
        wr(bullet + rt::LIFE, 0i32);
        return None;
    }
    let shooter: u32 = rd(en + 0x20);
    let coll: usize = rd(bullet + gb::COLL);
    if let Ok(mut g) = BOLTS.lock() {
        g.retain(|x| x.id != bid);
        if g.len() >= MAX {
            g.remove(0);
        }
        g.push(Bolt { id: bid, coll, shooter, top, bottom, frames: 0 });
        BOLTS_ANY.store(true, Ordering::Relaxed);
    }
    wr(bullet + rt::STATE_EXE, bolt_exe as *const () as usize);
    line(|| format!("{class} e{shooter}: Thunder {bid} at KH1's mark ({:.0},{:.0},{:.0}): from ({:.2},{:.2},{:.2}) down to 2 cm over the floor in one frame, radius {radius:.2} m, AP {ap} attribute {attr} kind {THUNDER_KIND:#x} (Re:CoM's Yellow Opera Thunder, id {id}); Guard doesn't meet it", at.x, at.y, at.z, from.x, from.y, from.z));
    Some(bid)
}

/// The bolt's flight-state exe: its first frame the velocity that sweeps it from the cylinder's top to the floor (Re:CoM's
/// step moves it and its collision; the pass hits Sora inside); its second its attack off and its life over (Re:CoM ends
/// it); then Re:CoM's own step (the thunk).
pub unsafe extern "win64" fn bolt_exe(bullet: usize) {
    let mover: usize = rd(bullet + gb::MOVER);
    let id: u32 = rd(bullet + 0x20);
    let dt: i32 = rd(bullet + 0x14);
    let step = crate::units::step_from_dt(dt.clamp(0, 1000));
    if mover != 0 && rd::<i32>(bullet + rt::LIFE) > 0 && step > 0.0 {
        let mut note = None;
        if let Ok(mut g) = BOLTS.lock() {
            let mut gone = false;
            if let Some(b) = g.iter_mut().find(|b| b.id == id) {
                b.frames += 1;
                if b.frames == 1 {
                    let now = rd_vec(mover + rt::POSITION);
                    let v = (crate::units::to_recom(b.bottom) - now) * (1.0 / step);
                    wr(mover + gb::VELOCITY, [v.x, v.y, v.z, 0.0f32]);
                    wr(mover + gb::PITCH, THUNDER_PITCH);
                    wr(bullet + gb::HOMING_TARGET, 0i32);
                } else {
                    wr(mover + gb::VELOCITY, [0.0f32; 4]);
                    super::magic::hold_off(bullet);
                    wr(bullet + rt::LIFE, 0i32);
                    gone = true;
                    let (s, t) = (b.shooter, b.top);
                    note = Some(format!("thunder {id} (e{s}): its frame is over (swept from KH1 y {:.0})", t.y));
                }
            }
            if gone {
                g.retain(|b| b.id != id);
            }
            if g.is_empty() {
                BOLTS_ANY.store(false, Ordering::Relaxed);
            }
        }
        if let Some(n) = note {
            line(|| n);
        }
    }
    func::<PlFn>(HOMING_THUNK)(bullet)
}

/// The battle ended: no bolt of the last one is ours any more.
pub(super) fn forget_bolts() {
    if let Ok(mut g) = BOLTS.lock() {
        g.clear();
    }
    BOLTS_ANY.store(false, Ordering::Relaxed);
}
