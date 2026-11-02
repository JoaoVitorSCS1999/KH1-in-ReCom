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
//! Runs once a frame per flying blade (at most [`MAX`]); nothing otherwise.

use super::magic::{rt, Desc, DescInitFn, FactoryFn, ManagerFn, MoverFn, PlFn, DESC_INIT, FACTORY, HOMING_THUNK, SHOT_MANAGER, SHOT_MOVER};
use super::movement::{func, rd, rd_vec, wr};
use super::{base, log, va};
use crate::guard::bullet as gb;
use crate::kh1_enemy::Manifest;
use kh1_core::enemy::thrown::{Numbers, Thrown};
use kh1_core::Vec3;
use std::sync::atomic::{AtomicU32, Ordering};
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

struct Blade {
    id: u32,
    /// The thrower's id (Re:CoM's `+0x20`, the bullet's `+0x238`).
    shooter: u32,
    flight: Thrown,
    /// Sora's Guard turned it back (decision 28): Re:CoM's homing flies it from here.
    turned: bool,
}

static BLADES: Mutex<Vec<Blade>> = Mutex::new(Vec::new());
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

/// `sys 94`: enemy `en` (class manifest `m`, KH1 facing `yaw`) throws its object `n`. The bullet's id.
pub(super) unsafe fn throw(en: usize, class: &str, m: &Manifest, n: u8, yaw: f32) -> Option<u32> {
    if n != 0 {
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
        g.push(Blade { id: bid, shooter, flight, turned: false });
    }
    wr(bullet + rt::STATE_EXE, blade_exe as *const () as usize);
    line(|| format!("{class} e{shooter}: throws its blade: shot {bid} at ({:.2},{:.2},{:.2}) yaw {yaw:.2}, AP {ap} kind {kind_out} attribute {attr} (Re:CoM attack id {id}), radius {:.2} m, life {} frames", at.x, at.y, at.z, c("blade_radius") * 0.01, flight.n.life));
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
                if turned {
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

/// The battle ended (or a new one began): no blade of the last one is ours any more.
pub(super) fn forget() {
    if let Ok(mut g) = BLADES.lock() {
        g.clear();
    }
}

/// The trace's token: the blades flying (id, frames, speed, turned back).
pub fn trace_token() -> String {
    let Ok(g) = BLADES.try_lock() else { return String::new() };
    g.iter().map(|b| format!("blade {}:{:.0}:{:.1}{}", b.id, b.flight.time, b.flight.speed, if b.turned { ":back" } else { "" })).collect::<Vec<_>>().join(" ")
}
