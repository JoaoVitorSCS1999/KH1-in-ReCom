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
const FLAGS: u64 = 0x100 | 4;
/// The Fat Bandit's own fireball's effect sets (its state 3 `0x14018DF20`: descriptor `+0xA8` and `+0xB8`; slot 1 of
/// the first names `"_ex_2060"` (`sites::FATBANDIT_FIRE_PTRS`)). Checked (decomp + exe).
pub const FIRE_EFFECT_SET: u64 = 0x1_4053_E290;
pub const FIRE_EFFECT_PLACES: u64 = 0x1_4053_E300;
/// The Defender's own fireball's effect set (its state 3 `0x14018DAF0`: descriptor `+0xA8`, none at `+0xB8`; slot 1 names
/// `"_ex_2340"` (`sites::DEFENDER_FIRE_PTRS`)). Checked (decomp + exe).
pub const DEFENDER_FIRE_EFFECT_SET: u64 = 0x1_4053_CF30;

/// A class's own fireball look (descriptor `+0xA8`, `+0xB8`) when its effect set is where the exe had it: the Fat Bandit's
/// (`0x14053E290` / `0x14053E300`), the Defender's (`0x14053CF30`); None (no look) otherwise.
pub(super) unsafe fn fire_look(class: &str) -> Option<(u64, Option<u64>)> {
    let ok = |sites: &[crate::sites::PtrSite]| sites.iter().all(|p| rd::<usize>(base() + p.rva as usize) == base() + p.target as usize);
    match class {
        "defender" => ok(crate::sites::DEFENDER_FIRE_PTRS).then_some((DEFENDER_FIRE_EFFECT_SET, None)),
        _ => ok(crate::sites::FATBANDIT_FIRE_PTRS).then_some((FIRE_EFFECT_SET, Some(FIRE_EFFECT_PLACES))),
    }
}
/// The range from its spawn (m; KH1's life ends it), the homing gate (m) after a turn-back.
const RANGE: f32 = 255.0;
const GATE: f32 = 255.0;
/// The fade after its end (ticks, as our Fire's).
const FADE: i32 = 1000;
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
    if has("fire_life") && n == fire_n {
        return fire(en, class, m, yaw);
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
        g.push(Blade { kind: Kind::Blade, batted: false, home: None, id: bid, shooter, flight, turned: false });
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
unsafe fn fire(en: usize, class: &str, m: &Manifest, yaw: f32) -> Option<u32> {
    let c = |k: &str| m.consts.get(k);
    let Some(at) = manifest_point(en, m, "fire_at") else {
        line(|| format!("{class}: no bone for the fireball's start ({})", c("fire_at_bone")));
        return None;
    };
    let target = sora_target();
    // `sys 143` (`0x1402BAE10`): the head's direction while its look is on (the Fat Bandit's, from its `eye` point), else
    // the facing (the Defender's script never turns a head look on).
    let dir = if c("eye_bone").is_finite() {
        let Some(eye) = manifest_point(en, m, "eye") else {
            line(|| format!("{class}: no bone for the fireball's aim ({})", c("eye_bone")));
            return None;
        };
        kh1_core::fatbandit::fire_dir(yaw, crate::units::to_kh1(eye), target, c("look_cone_out"), c("look_up"), c("look_down"), c("look_left"), c("look_right"))
    } else {
        Vec3::from_yaw(yaw)
    };
    // Its speed: the Fat Bandit's 45, the Defender's drawn in [8, 10) (`0x418(8, 10)`: rand × (10 − 8) + 8).
    let speed = if c("fire_speed").is_finite() {
        c("fire_speed")
    } else {
        let r = RNG.lock().ok().and_then(|mut g| g.get_or_insert_with(|| Kh1Rand::new(0x2090_0001)).next_f32().into()).unwrap_or(0.5);
        r * (c("fire_speed_max") - c("fire_speed_min")) + c("fire_speed_min")
    };
    let n = Numbers { life: c("fire_life"), turn_back: c("fire_life"), speed, accel: 0.0, turn_deg: c("fire_turn"), slide: 0.0, slide_drag: 0.0 };
    let flight = Thrown::aimed(crate::units::to_kh1(at), dir, n);
    let id = c("fire_recom_id") as i32;
    // Re:CoM's own fireball's power: `0x14017F0E0(enemy, 1, 1, 0)` (no kind out, flag 0), `0x14017F050(enemy, 1)`.
    let ap = func::<ApFn>(f::ATTACK_AP)(en, id, c("fire_kind") as i32, 0, std::ptr::null_mut(), 0) as i32;
    let attr = func::<AttrFn>(f::ATTACK_ATTR)(en, id);
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
    let w = |o: usize| dp.add(o);
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
    match fire_look(class) {
        Some((set, places)) => {
            std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), set));
            if let Some(p) = places {
                std::ptr::write_unaligned(w(0xB8) as *mut usize, va(base(), p));
            }
        }
        None => line(|| format!("{class}: its fireball effect set is not where the exe had it: the fireball has no look")),
    }
    std::ptr::write_unaligned(w(0x78) as *mut f32, c("fire_radius") * crate::units::METRES_PER_UNIT);
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
        g.push(Blade { kind: Kind::Fire, batted: false, home: None, id: bid, shooter, flight, turned: false });
        FLYING.store(true, Ordering::Relaxed);
    }
    wr(bullet + rt::STATE_EXE, blade_exe as *const () as usize);
    let dd = flight.dir;
    line(|| format!("{class} e{shooter}: fireball {bid} from ({:.2},{:.2},{:.2}) along ({:.3},{:.3},{:.3}) (KH1's aim at {:?}), {:.2} a frame turning {}° for {} frames, AP {ap} attribute {attr} (Re:CoM's fireball, id {id}), radius {:.2} m", at.x, at.y, at.z, dd.x, dd.y, dd.z, target.map(|t| (t.x as i32, t.y as i32, t.z as i32)), flight.n.speed, flight.n.turn_deg, flight.n.life, c("fire_radius") * 0.01));
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
        let mut end = false;
        if let Ok(mut g) = BLADES.lock() {
            if let Some(b) = g.iter_mut().find(|b| b.id == id) {
                // A fireball turns toward Sora's aim point at its own rate (the Fat Bandit's 0: straight; the Defender's 1°);
                // a batted one toward its new target (`0x1B8`'s `hp[0x30]`) while that lives (`sys 120`), on the enemies' side.
                if b.batted {
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

/// The battle ended (or a new one began): no blade of the last one is ours any more.
pub(super) fn forget() {
    if let Ok(mut g) = BLADES.lock() {
        g.clear();
    }
    FLYING.store(false, Ordering::Relaxed);
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
        if b.kind == Kind::Fire {
            // KH1's projectile clash: sent to the swinger's side and aimed anew (`0x1B8`): at Sora's target while it
            // lives, else at its thrower (`sys 32` of the parent: its lock point), straight, its whole life again.
            if !super::guard::send_back(bullet) {
                continue;
            }
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
            let pl: usize = rd(va(b0, crate::sites::data::MAIN_PLAYER));
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
            if mover != 0 {
                let p = rd_vec(mover + rt::POSITION);
                super::attack::spark_at([p.x, p.y, p.z, 1.0]);
            }
            let (id, d) = (b.id, b.flight.dir);
            line(|| format!("fireball {id}: Sora's swing bats it back along ({:.2},{:.2},{:.2}) (KH1's projectile clash), on the enemies' side", d.x, d.y, d.z));
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

/// The trace's token: the blades flying (id, frames, speed, turned back).
pub fn trace_token() -> String {
    let Ok(g) = BLADES.try_lock() else { return String::new() };
    g.iter().map(|b| format!("{} {}:{:.0}:{:.1}{}", if b.kind == Kind::Fire { "fire" } else { "blade" }, b.id, b.flight.time, b.flight.speed, if b.turned { ":back" } else if b.batted { ":batted" } else { "" })).collect::<Vec<_>>().join(" ")
}
