//! M11: the Bouncywild's banana peels as Re:CoM's own bullets (`docs/recom-m11-enemies.md` § The Bouncywild; owner
//! 2026-11-02: "Re:CoM bullets moved by KH1's numbers, drawn with a Re:CoM effect (its own)").
//!
//! - **KH1** (Checked (files + decomp)): `sys 94` makes `ex_2040_01.bd` (the toss, 80 frames into 200: along the
//!   thrower's facing at rand [8, 10) a frame, rand [10, 15) up) or `ex_2040_02.bd` (the run-away's drop, at once: toward
//!   its target's place at rand [3, 10), rand [5, 10) up), from `sys 19(parent, 28, 1)` (the left hand). Each peel
//!   (`0x96`) falls (its speed's y + 0.8 × step a frame) through a radius-30 sweep (`sys 140`) until it touches the
//!   ground, then lies 120 frames (effect 11); each frame a party member that can be hit (`sys 141`), stands on the
//!   ground and has no flag 0x400 within 50 of it slips (`0x29A`): Sora takes request 200 of the Bouncywild's player set
//!   as an action, flag 0x80 for 30 frames, loses the munny of the thrower's table `0x2A4` (`sys 142(−sys 155)`: 30 × 1 +
//!   10 × 5 = 80 at KH1's Jackpot 1) which drops there as prizes (`sys 122`); an enemy gets message 1001. The peel ends.
//!   The peel is no attack (no row): KH1's swings and Guard never meet it.
//! - **Here**: one of Re:CoM's own `CHomingBullet`s per peel (as the Gargoyle's ball), its attack held off while it falls
//!   (`CCollAttack` off) and on once it lies (`sites::COLL_ON_PTRS`), its life KH1's; its flight-state exe ([`peel_exe`])
//!   steps KH1's peel ([`kh1_core::bouncywild::Peel`]) and writes the velocity that puts the bullet on KH1's place (the
//!   battle floor: the thrower's height when it throws; the peel lies with its sphere's centre 30 above it, so Re:CoM's
//!   1 cm mover never touches the floor), then Re:CoM's own step collides it. **Its hit is Re:CoM's own slip**: no
//!   power (KH1's peel does no damage) and Re:CoM's Bouncywild banana's reaction kind 0x18 (`0x14018C870`; Checked
//!   (decomp + exe: `sites::BOUNCYWILD_CHECKED`)), which Re:CoM's damage check `0x1401E6DE0` plays as Sora's slip: row
//!   0x29 `p001_049`, an exact bake of KH1's slip clip (Checked (files): the asset step's matcher, KH1's request 200 of
//!   `xa_ex_2040.mset`'s player set 0 against `PL0001.ANN`). **A slip loses nothing** (owner 2026-11-03): Re:CoM's
//!   case then calls `0x1401F6190` (a tenth of his Moogle points scattered as prizes); that one call is retargeted
//!   ([`install_slip_scatter`], `sites::SLIP_SCATTER_CALLS`) to [`slip_scatter`], which skips it while the damage-check
//!   wrapper takes a slip from one of our bullets ([`before_damage`]). A hit ends the
//!   bullet (Re:CoM's flag `0x100`). Its reach: KH1's 50 as the bullet's radius against Sora's hurt body (KH1 measures
//!   to his root: Assumption, it reaches a little farther by his body's width).
//! - Not built: an enemy slipping (KH1's message 1001: Re:CoM's bullet tests only Sora's side); KH1's "on the ground"
//!   and flag tests (a Sora jumping over the floor-high sphere doesn't touch it: Assumption it agrees); Sora's KH1
//!   Guard blocks it as any Re:CoM shot (KH1's never meets it): it is used up, not turned back (`super::guard`).
//!
//! Per frame: one atomic load while no peel lies or falls; one KH1 step per peel.

use super::kh1_thrown::{fire_look, hold_on, manifest_point, FADE, FLAGS, GATE, RANGE};
use super::magic::{rt, Desc, DescInitFn, FactoryFn, ManagerFn, MoverFn, PlFn, DESC_INIT, FACTORY, HOMING_THUNK, SHOT_MANAGER, SHOT_MOVER};
use super::movement::{func, rd, rd_vec, wr};
use super::{base, log, va};
use crate::guard::bullet as gb;
use crate::kh1_enemy::Manifest;
use kh1_core::bouncywild::{Peel, PeelNumbers};
use kh1_core::rng::Kh1Rand;
use kh1_core::Vec3;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Re:CoM's slip (`crate::ground::bouncywild::SLIP_KIND`): the reaction kind its own Bouncywild banana carries, played by
/// the player damage check `0x1401E6DE0` (case 0x18: row 0x29 and `0x1401F6190`). Checked (decomp + exe).
pub const SLIP_KIND: u32 = crate::ground::bouncywild::SLIP_KIND;
/// Re:CoM's Moogle point scatter (`sites::SLIP_SCATTER_CHECKED`), called by [`slip_scatter`] for every slip not ours.
const SCATTER: u32 = 0x001F_6190;
/// Sora's pending hit: its kind (`+0x710`), pending (`+0x720`), its attacker's handle (his body `+0x690`'s `+0x98`,
/// `0x14016F1C0`'s first write: the attack's owner, the bullet itself for a shot). Checked (decomp).
const HIT_KIND: usize = 0x710;
const HIT_PENDING: usize = 0x720;
/// The player whose damage check is taking a slip from one of our bullets (0: none).
static KEEP_POINTS: AtomicUsize = AtomicUsize::new(0);
/// The scatter call is retargeted.
static SCATTER_ON: AtomicBool = AtomicBool::new(false);
/// The enemy attack attribute getter (`0x14017F050`, `sites::THROWN_CALLEES`).
const ATTACK_ATTR: u32 = 0x0017_F050;
type AttrFn = unsafe extern "win64" fn(usize, i32) -> u32;
/// At most this many peels at once (older ones are forgotten: Re:CoM's own step keeps them still).
const MAX: usize = 16;
/// The peel's mover radius (`0x140157DA0`'s second word): small, so Re:CoM's mover never touches the floor under it.
const MOVER_RADIUS: f32 = 0.01;
/// The bullet's life (frames): longer than any fall plus KH1's 120 frames lying; [`peel_exe`] ends it at KH1's end.
const LIFE: f32 = 600.0;

struct PeelRec {
    id: u32,
    shooter: u32,
    peel: Peel,
    n: PeelNumbers,
    floor: f32,
    on: bool,
}

static PEELS: Mutex<Vec<PeelRec>> = Mutex::new(Vec::new());
static ANY: AtomicBool = AtomicBool::new(false);
/// KH1's `sys 25` draws (Assumption: a stream of our own, seeded once, as the Defender's fireball speed).
static RNG: Mutex<Option<Kh1Rand>> = Mutex::new(None);
static LINES: AtomicU32 = AtomicU32::new(60);

fn line(s: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("peel: {}", s()));
    }
}

fn draw() -> f32 {
    RNG.lock().ok().map(|mut g| g.get_or_insert_with(|| Kh1Rand::new(0x2040_0001)).next_f32()).unwrap_or(0.5)
}

/// `func_33a(a, b)`: rand × (b − a) + a.
fn between(a: f32, b: f32) -> f32 {
    draw() * (b - a) + a
}

/// `sys 94(actor, 1 / 2, …)`: the Bouncywild `en` (facing `yaw`, KH1) tosses its peels (`tossed`) or drops them toward
/// Sora.
pub(super) unsafe fn start(en: usize, class: &str, m: &Manifest, tossed: bool, yaw: f32) {
    let c = |k: &str| m.consts.get(k);
    let Some(at) = manifest_point(en, m, "fire_at") else {
        line(|| format!("{class}: no bone for the peel's start ({})", c("fire_at_bone")));
        return;
    };
    let at = crate::units::to_kh1(at);
    let mover: usize = rd(en + rt::MOVER);
    if mover == 0 {
        return;
    }
    // The battle floor: the thrower stands on it when it throws (both throws wait for the ground; Assumption: flat).
    let floor = crate::units::to_kh1(rd_vec(mover + rt::POSITION)).y;
    let n = PeelNumbers { sweep: c("peel_sweep"), gravity: c("peel_gravity"), lie: c("peel_lie") };
    let sora = super::kh1_enemy::sora_pos();
    let shooter: u32 = rd(en + 0x20);
    let attr = func::<AttrFn>(ATTACK_ATTR)(en, c("fire_recom_id") as i32);
    // `ex_2040_01.bd` / `_02.bd` `0x502`: `+24` peels, the angle 0 then ± spread × rand [1.5, 2.5) alternating; per peel
    // the speed's draw, then the height's, then (after the peel is made) the spread's.
    let (count, mut angle, mut spread) = (if tossed { c("peel_count") } else { c("drop_count") }, 0.0f32, c("peel_spread"));
    let mut k = 0.0;
    while k < count {
        let v = if tossed {
            let s = between(c("peel_speed_min"), c("peel_speed_max"));
            let up = between(c("peel_up_min"), c("peel_up_max"));
            Peel::tossed(yaw, angle, s, up)
        } else {
            let s = between(c("drop_speed_min"), c("drop_speed_max"));
            let up = between(c("drop_up_min"), c("drop_up_max"));
            // KH1 aims from the hand at its target's place (`*(target)+0x10`, his feet); with no target, along its facing
            // (Assumption: KH1 always has one here).
            let to = sora.unwrap_or(at + Vec3::from_yaw(yaw) * 100.0);
            let mut d = Peel::dropped(at, to, s, up);
            if !d.x.is_finite() {
                d = Peel::tossed(yaw, 0.0, s, up);
            }
            d
        };
        let d = Vec3::from_yaw(yaw + angle.to_radians());
        let peel = Peel::new(at, v);
        match bullet(en, class, m, &peel, attr) {
            Some(id) => {
                if let Ok(mut g) = PEELS.lock() {
                    // Peels Re:CoM has ended (Sora slipped on one) are forgotten.
                    g.retain(|p| {
                        let o = func::<super::movement::HandleFn>(super::movement::HANDLE_OBJ)(p.id);
                        o != 0 && rd::<usize>(o + rt::STATE_EXE) == peel_exe as *const () as usize && rd::<i32>(o + rt::LIFE) > 0
                    });
                    if g.len() >= MAX {
                        g.remove(0);
                    }
                    g.push(PeelRec { id, shooter, peel, n, floor, on: false });
                    ANY.store(true, Ordering::Relaxed);
                }
                line(|| format!("{class} e{shooter}: {} peel {id} from ({:.0},{:.0},{:.0}) at ({:.1},{:.1},{:.1}) a frame (facing ({:.2},{:.2})), floor {floor:.0}, reach {:.2} m, Re:CoM's slip (kind {SLIP_KIND:#x}, no power, no Moogle points lost)", if tossed { "tossed" } else { "dropped" }, at.x, at.y, at.z, v.x, v.y, v.z, d.x, d.z, c("peel_reach") * 0.01));
            }
            None => line(|| format!("{class} e{shooter}: no peel made")),
        }
        spread = -spread * between(c("peel_spread_min"), c("peel_spread_max"));
        angle += spread;
        k += 1.0;
    }
}

/// One peel's bullet: a `CHomingBullet` at KH1's place, standing still, no power, Re:CoM's slip kind, its attack off.
unsafe fn bullet(en: usize, class: &str, m: &Manifest, p: &Peel, attr: u32) -> Option<u32> {
    let at = crate::units::to_recom(p.pos);
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x68) as *mut u32, SLIP_KIND);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, 0);
    std::ptr::write_unaligned(w(0x60) as *mut u64, FLAGS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, 0.0);
    let life = crate::units::frames_to_units(LIFE);
    std::ptr::write_unaligned(w(0x38) as *mut i32, life);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, life);
    std::ptr::write_unaligned(w(0x34) as *mut f32, RANGE);
    std::ptr::write_unaligned(w(0x44) as *mut i32, 0);
    std::ptr::write_unaligned(w(0x48) as *mut f32, GATE);
    std::ptr::write_unaligned(w(0x4C) as *mut f32, 0.0);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [at.x, at.y, at.z, 0.5]);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [0.0, 0.0, 0.0, 0.0]);
    match fire_look(class, "peel") {
        Some((set, places)) => {
            std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), set));
            if let Some(pl) = places {
                std::ptr::write_unaligned(w(0xB8) as *mut usize, va(base(), pl));
            }
        }
        None => line(|| format!("{class}: its banana's effect set is not where the exe had it: the peel has no look")),
    }
    std::ptr::write_unaligned(w(0x78) as *mut f32, m.consts.get("peel_reach") * crate::units::METRES_PER_UNIT);
    std::ptr::write_unaligned(w(0x94) as *mut i32, FADE);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, en);
    if bullet == 0 {
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, MOVER_RADIUS, 0.0, 0, -1);
    let bid: u32 = rd(bullet + 0x20);
    let bs = base();
    let vt: usize = rd(bullet);
    let exe: usize = rd(bullet + rt::STATE_EXE);
    let adj: i32 = rd(bullet + rt::STATE_EXE_ADJUST);
    if vt != va(bs, gb::HOMING_VTABLE) || exe != bs + HOMING_THUNK as usize || adj != 0 {
        line(|| format!("peel {bid} is not a plain CHomingBullet in flight (vt {vt:#x}, exe {exe:#x}): ended"));
        wr(bullet + rt::LIFE, 0i32);
        return None;
    }
    // KH1's peel slips nobody while it falls: Re:CoM's attack held off until it lies.
    super::magic::hold_off(bullet);
    wr(bullet + rt::STATE_EXE, peel_exe as *const () as usize);
    Some(bid)
}

/// The peel's flight-state exe: KH1's peel this frame ([`Peel::step`]), the velocity that puts the bullet on KH1's place;
/// its attack on once it lies; ended at KH1's end; then Re:CoM's own step (the thunk).
pub unsafe extern "win64" fn peel_exe(bullet: usize) {
    let mover: usize = rd(bullet + gb::MOVER);
    let live = rd::<i32>(bullet + rt::HOLD) == 0 && rd::<i32>(bullet + rt::LIFE) > 0 && mover != 0;
    let id: u32 = rd(bullet + 0x20);
    if live {
        let dt: i32 = rd(bullet + 0x14);
        let step = crate::units::step_from_dt(dt.clamp(0, 1000));
        let mut note: Option<String> = None;
        let mut end = false;
        if let Ok(mut g) = PEELS.lock() {
            if let Some(p) = g.iter_mut().find(|p| p.id == id) {
                if step > 0.0 {
                    let was = p.peel.lying.is_some();
                    p.peel.step(&p.n, p.floor, step);
                    if !was && p.peel.lying.is_some() {
                        p.on = hold_on(bullet);
                        let q = p.peel.pos;
                        note = Some(format!("peel {id}: lies at ({:.0},{:.0},{:.0}) for {} frames, its slip {}", q.x, q.y, q.z, p.n.lie, if p.on { "on" } else { "NOT on (CCollAttack's slot +0x10 is not 0x14014DE90)" }));
                    }
                    if p.peel.ended {
                        end = true;
                        note = Some(format!("peel {id}: ends (KH1's {} frames lying, nobody slipped)", p.n.lie));
                    } else {
                        let to = crate::units::to_recom(p.peel.pos);
                        let now = rd_vec(mover + rt::POSITION);
                        let v = (to - now) * (1.0 / step);
                        wr(mover + gb::VELOCITY, [v.x, v.y, v.z, 0.0f32]);
                        wr(bullet + gb::HOMING_TARGET, 0i32);
                    }
                }
            }
            if end {
                g.retain(|p| p.id != id);
                if g.is_empty() {
                    ANY.store(false, Ordering::Relaxed);
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

/// Whether bullet `id` is one of our peels (Sora's Guard leaves it where it is: KH1's never meets it).
pub(super) fn is_peel(id: u32) -> bool {
    if !ANY.load(Ordering::Relaxed) {
        return false;
    }
    PEELS.try_lock().map(|g| g.iter().any(|p| p.id == id)).unwrap_or(false)
}

/// Whether the attacker `owner` is one of our KH1 Bouncywild's slipping bullets: its peels, or its banana (Re:CoM's slip
/// kind too, owner 2026-11-03: `super::kh1_thrown::SLIPS`).
fn slips(owner: u32) -> bool {
    owner != 0 && (is_peel(owner) || super::kh1_thrown::slip_shot(owner))
}

/// The damage-check wrapper, before Re:CoM's check (`super::attack::damage_check`): a pending slip from one of our bullets
/// keeps Sora's Moogle points for this check ([`slip_scatter`]). One atomic load while the call isn't retargeted.
pub(super) unsafe fn before_damage(pl: usize) {
    if !SCATTER_ON.load(Ordering::Relaxed) {
        return;
    }
    let pending = rd::<i16>(pl + HIT_PENDING) != 0;
    let kind: u32 = rd(pl + HIT_KIND);
    if !pending || kind != SLIP_KIND {
        return;
    }
    let owner: u32 = rd(pl + crate::guard::body::SORA + crate::guard::body::ATTACKER);
    if crate::ground::bouncywild::slip_keeps_points(pending, kind, slips(owner)) {
        KEEP_POINTS.store(pl, Ordering::Relaxed);
    }
}

/// After Re:CoM's check: the next slip is Re:CoM's again.
pub(super) fn after_damage() {
    KEEP_POINTS.store(0, Ordering::Relaxed);
}

/// The damage check's slip-case call of `0x1401F6190` (`sites::SLIP_SCATTER_CALLS`, retargeted at install): skipped while
/// [`before_damage`] marked this player's slip as ours (owner 2026-11-03: "a slip loses nothing"); Re:CoM's scatter
/// otherwise, as its call was.
pub unsafe extern "win64" fn slip_scatter(pl: usize) {
    if pl != 0 && KEEP_POINTS.load(Ordering::Relaxed) == pl {
        let owner: u32 = rd(pl + crate::guard::body::SORA + crate::guard::body::ATTACKER);
        let pts: i32 = rd(pl + 0x448);
        line(|| format!("Sora slipped on shot {owner}: Re:CoM's slip without its Moogle point scatter ({pts} points kept)"));
        return;
    }
    func::<PlFn>(SCATTER)(pl)
}

/// Retargets the damage check's slip scatter call to [`slip_scatter`] (once, after the classes are in, only with the KH1
/// Bouncywild's; its context and callee checked first). Without it a slip on our peels or banana scatters Moogle points (logged).
pub unsafe fn install_slip_scatter(bs: usize) {
    if !crate::kh1_enemy::CLASSES.iter().enumerate().any(|(k, c)| c.name == "bouncywild" && super::kh1_enemy::installed(k)) {
        return;
    }
    for c in crate::sites::SLIP_SCATTER_CHECKED {
        if std::slice::from_raw_parts((bs + c.rva as usize) as *const u8, c.expect.len()) != c.expect {
            log(&format!("peel: {} is not as expected: a slip on a KH1 peel scatters Moogle points (Re:CoM's)", c.name));
            return;
        }
    }
    let s = &crate::sites::SLIP_SCATTER_CALLS[0];
    let at = bs + s.rva as usize;
    let call = super::hook::place_near(at, &crate::hookplan::abs_jmp(slip_scatter as *const () as u64)).and_then(|stub| crate::hookplan::call_rel32(at as u64, stub as u64).map(|c| (stub, c)));
    let Some((stub, call)) = call else {
        log("peel: no stub within reach of the slip scatter call: a slip on a KH1 peel scatters Moogle points (Re:CoM's)");
        return;
    };
    if super::hook::patch(at, s.expect, &call, s.name) {
        SCATTER_ON.store(true, Ordering::Relaxed);
        log(&format!("peel: {} retargeted through {stub:#x}: a slip on the KH1 Bouncywild's peels and banana keeps Sora's Moogle points", s.name));
    }
}

/// The battle ended (or a new one began).
pub(super) fn forget() {
    if let Ok(mut g) = PEELS.lock() {
        g.clear();
    }
    ANY.store(false, Ordering::Relaxed);
}

/// The trace's token: the peels (id, thrower, falling or lying frames).
pub fn trace_token() -> String {
    if !ANY.load(Ordering::Relaxed) {
        return String::new();
    }
    let Ok(g) = PEELS.try_lock() else { return String::new() };
    g.iter().map(|p| format!("peel {}:e{}:{}", p.id, p.shooter, p.peel.lying.map_or("fall".to_string(), |t| format!("{t:.0}")))).collect::<Vec<_>>().join(" ")
}
