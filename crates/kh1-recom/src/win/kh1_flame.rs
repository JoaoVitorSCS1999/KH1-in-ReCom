//! M11: the Fat Bandit's flame (KH1's child `ex_2060_01.bd`, `docs/recom-m11-enemies.md` § The Fat Bandit) as
//! Re:CoM's own bullets (owner 2026-11-02: "the fire breath: KH1's flame as Re:CoM attack records / bullets per KH1's
//! numbers").
//!
//! - **KH1** (Checked (files + decomp)): the child makes one attack object from the parent's row `0x15C` in mode 3
//!   (tested every frame) with shape 2: KH1's attack pass lays it as a chain of spheres from its start (radius `+0xC`)
//!   to its end (radius `+0x3C`), one every (r0 + r1) / 2 (`0x1402B3800` → `0x1402B3D50`); its hit-once mask clears
//!   every 60 frames (`+0x50`, `0x1402B3380`): it can hit Sora again a second later. Each frame, for at most 140
//!   frames while 207 plays: from bone code 0 at t 3, 25 lower, to bone code 0 at t 15, 75 lower, radii 23 and 60;
//!   its first 60 frames the line and the radii grow by t / 60 (`kh1_core::fatbandit::flame_spheres`). A wall on the
//!   line (`sys 77`) cuts it (not built: Re:CoM's battle arenas are open floors, Assumption).
//! - **Here**: one Re:CoM `CBullet` per sphere, made as Re:CoM's Aero shot is (flags 7: a shot that stays through its
//!   hits, Checked (disasm `0x140222DFE`); `win::magic`'s Aero wind), sharing one hit serial (Re:CoM's Blizzard rule,
//!   `desc+0x74`); each frame from the Fat Bandit's own update ([`step`]) each one is put on KH1's sphere (its mover's
//!   place, its radius through Re:CoM's own setter `0x140172690`) and its life set to a few frames (so a flame we stop
//!   stepping ends by itself in Re:CoM); every 60 frames the serial is renewed through Re:CoM's own `0x14016F7D0` (KH1's
//!   re-hit). Re:CoM collides the spheres with Sora, applies the damage (Re:CoM's own fireball's power, id 1: the
//!   attack of the same kind, Assumption) and draws each with its own fireball's effects (`0x14053E290`).
//! - Ends when 207 stops playing, after 140 frames, at the defeat or a new battle (the shots' lives run out).
//! - **The Defender's flame** (M11, `ex_2340_00.bd`, Checked (files)), the same chain with its own numbers (the manifest's,
//!   [`Shape`]): it waits 110 frames while 212 plays (it never burns if 212 stops first), then burns 140 frames from bone
//!   105's line (`0x1069` at t 3 / 15, both 25 lower; Re:CoM's `lb_wep0`), radii 20 → 40, growing over its first 30
//!   frames, **placed where the line was 7 frames before** (KH1's 8-slot ring `0x7A6`: the flame trails the shield),
//!   re-hitting every 10 frames; once 212 stops it burns 10 frames more. Its look: the Defender's own fireball effect set.
//!
//! Per frame: nothing while no flame burns; one bone read and ≤ 16 bullet writes per burning flame.

use super::magic::{rt, Desc, DescInitFn, FactoryFn, ManagerFn, MoverFn, PlFn, SerialFn, DESC_INIT, FACTORY, NEW_SERIAL, SHOT_MANAGER, SHOT_MOVER, SHOT_SERIAL};
use super::movement::{func, rd, wr};
use super::{base, log, va};
use crate::kh1_enemy::Manifest;
use kh1_core::Vec3;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

/// The request the flame lives with (`ex_2060_01.bd` `0x50C`, the manifest's checked push).
const BREATH: u16 = kh1_core::fatbandit::req::BREATH;
/// At most this many spheres (KH1's chain is ⌈length / spacing⌉: 8 on the player's numbers).
const MAX_SPHERES: usize = 16;
/// Re:CoM's Aero shot's flags (`mov qword [rbp−0x50],7`, Checked (disasm `0x140222DFE`)) and 4 (the life ends it).
const FLAGS: u64 = 7 | 4;
/// The life each frame gives the spheres (ticks): three frames, renewed while the flame burns.
const LIFE: i32 = 150;
/// Re:CoM's collision radius setter (`0x140172690(coll, r)`: a sphere's `+0x70` / `+0x74`; `sites::FLAME_CALLEES`).
const SET_RADIUS: u32 = 0x0017_2690;
type RadiusFn = unsafe extern "win64" fn(usize, f32);
type ApFn = unsafe extern "win64" fn(usize, i32, i32, i16, *mut u32, u16) -> f32;
type AttrFn = unsafe extern "win64" fn(usize, i32) -> u32;
const ATTACK_AP: u32 = 0x0017_F0E0;
const ATTACK_ATTR: u32 = 0x0017_F050;

/// A flame's own rule beyond the Fat Bandit's (the manifest's numbers; absent: the Fat Bandit's).
#[derive(Clone, Copy, Debug)]
struct Shape {
    /// The parent's request it lives with (`flame_request`; the Fat Bandit's 207).
    request: u16,
    /// Frames it waits first while that request plays (`flame_delay`; 0).
    delay: f32,
    /// The ring's slots (`flame_ring`: the line of `ring` − 1 frames before; 0: the line now).
    ring: usize,
    /// Frames it burns on once the request stops (`flame_tail`; none: it ends at once).
    tail: Option<f32>,
    /// Its first `grow` frames it grows (`flame_grow`, or `flame_life` − `flame_grow_until`).
    grow: f32,
}

struct Flame {
    /// The Fat Bandit (its address and Re:CoM id).
    en: usize,
    id: u32,
    shape: Shape,
    /// The frames of its wait left.
    wait: f32,
    /// The ring: the lines (start, end) of its last `ring` frames, oldest first.
    lines: std::collections::VecDeque<(Vec3, Vec3)>,
    /// The tail began (once).
    cut: bool,
    /// The look (descriptor `+0xA8` / `+0xB8`).
    look: Option<(u64, Option<u64>)>,
    /// Its spheres' shot ids, in the chain's order.
    shots: Vec<u32>,
    /// KH1's clock (`sp+0x38`) and the re-hit clock (`+0xA4`).
    t: f32,
    rehit: f32,
    serial: u32,
    ap: i32,
    attr: u32,
}

static FLAMES: Mutex<Vec<Flame>> = Mutex::new(Vec::new());
static BURNING: AtomicBool = AtomicBool::new(false);
static LINES: AtomicU32 = AtomicU32::new(40);

fn line(s: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("flame: {}", s()));
    }
}

/// `sys 94(actor, 1, …, 0x15C)`: the Fat Bandit `en` starts its flame (a new one replaces an older one of the same
/// enemy: Assumption, KH1 would run both).
pub(super) unsafe fn start(en: usize, class: &str, m: &Manifest) {
    let c = |k: &str| m.consts.get(k);
    let has = |k: &str| c(k).is_finite();
    let shape = Shape {
        request: if has("flame_request") { c("flame_request") as u16 } else { BREATH },
        delay: if has("flame_delay") { c("flame_delay") } else { 0.0 },
        ring: if has("flame_ring") { c("flame_ring") as usize } else { 0 },
        tail: has("flame_tail").then(|| c("flame_tail")),
        grow: if has("flame_grow") { c("flame_grow") } else { c("flame_life") - c("flame_grow_until") },
    };
    let look = super::kh1_thrown::fire_look(class);
    let id = m.consts.get("fire_recom_id") as i32;
    let ap = func::<ApFn>(ATTACK_AP)(en, id, m.consts.get("fire_kind") as i32, 0, std::ptr::null_mut(), 0) as i32;
    let attr = func::<AttrFn>(ATTACK_ATTR)(en, id);
    let eid: u32 = rd(en + 0x20);
    if let Ok(mut g) = FLAMES.lock() {
        for f in g.iter().filter(|f| f.en == en) {
            end_shots(f);
        }
        g.retain(|f| f.en != en);
        g.push(Flame { en, id: eid, shape, wait: shape.delay, lines: std::collections::VecDeque::new(), cut: false, look, shots: Vec::new(), t: 0.0, rehit: 0.0, serial: 0, ap, attr });
        BURNING.store(true, Ordering::Relaxed);
    }
    line(|| format!("{class} e{eid}: its flame starts (KH1's child: after {} frames, spheres {} → {} cm for at most {} frames while {} plays, re-hitting every {}, {} frames behind), AP {ap} attribute {attr} (Re:CoM's fireball, id {id})", shape.delay, m.consts.get("flame_r0"), m.consts.get("flame_r1"), m.consts.get("flame_life"), shape.request, m.consts.get("flame_rehit"), shape.ring.saturating_sub(1)));
}

/// The shots' lives run out (Re:CoM ends and fades them).
unsafe fn end_shots(f: &Flame) {
    for &s in &f.shots {
        if let Some(o) = shot(s) {
            wr(o + rt::LIFE, 0i32);
        }
    }
}

/// A shot id's bullet while it is still a `CBullet` of ours.
unsafe fn shot(id: u32) -> Option<usize> {
    let o = func::<super::movement::HandleFn>(super::movement::HANDLE_OBJ)(id);
    (o != 0 && rd::<usize>(o) == va(base(), crate::guard::bullet::VTABLE)).then_some(o)
}

/// One sphere's bullet (Re:CoM's Aero shot's way, `win::magic::aero_shot`): a `CBullet`, flags 7, no speed, its life a
/// few frames, the Fat Bandit's fireball look, the flame's power, the chain's serial (`desc+0x74`, 0 for the first).
unsafe fn make(f: &Flame, at: Vec3, yaw: f32, r: f32) -> Option<u32> {
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 1, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, f.attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, f.ap);
    std::ptr::write_unaligned(w(0x74) as *mut u32, f.serial);
    std::ptr::write_unaligned(w(0x60) as *mut u64, FLAGS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, 0.0);
    std::ptr::write_unaligned(w(0x34) as *mut f32, 255.0);
    std::ptr::write_unaligned(w(0x38) as *mut i32, LIFE);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, LIFE);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [at.x, at.y, at.z, 1.0]);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [0.0, crate::units::yaw_to_recom(yaw), 0.0, 0.0]);
    if let Some((set, places)) = f.look {
        std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), set));
        if let Some(p) = places {
            std::ptr::write_unaligned(w(0xB8) as *mut usize, va(base(), p));
        }
    }
    std::ptr::write_unaligned(w(0x78) as *mut f32, r.max(0.001));
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, f.en);
    if bullet == 0 {
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    Some(rd(bullet + 0x20))
}

/// One frame of enemy `en`'s flame (from its update, `win::kh1_enemy::tick`): `playing` its KH1 request, `live` it is
/// alive in one of our states; `yaw` its KH1 facing.
pub(super) unsafe fn step(en: usize, m: &Manifest, playing: Option<u16>, live: bool, yaw: f32, step: f32) {
    if !BURNING.load(Ordering::Relaxed) {
        return;
    }
    let Ok(mut g) = FLAMES.try_lock() else { return };
    let Some(i) = g.iter().position(|f| f.en == en) else { return };
    let c = |k: &str| m.consts.get(k);
    let sh = g[i].shape;
    // The Defender's child first waits while the request plays (`0x72E`: it ends if the request stops first).
    let waiting = g[i].wait - 0.0 > 0.0;
    let stopped = playing != Some(sh.request) && (sh.tail.is_none() || waiting);
    if !live || stopped || g[i].t - c("flame_life") >= 0.0 {
        let f = g.remove(i);
        end_shots(&f);
        let (id, t) = (f.id, f.t);
        line(|| format!("e{id}: its flame ends after {t:.0} frames ({} {}, alive {live}{})", sh.request, if playing == Some(sh.request) { "plays" } else { "no longer plays" }, if waiting { ", before it burned" } else { "" }));
        if g.is_empty() {
            BURNING.store(false, Ordering::Relaxed);
        }
        return;
    }
    if waiting {
        g[i].wait -= step;
        return;
    }
    let (Some(p0), Some(p1)) = (super::kh1_thrown::manifest_point(en, m, "flame0"), super::kh1_thrown::manifest_point(en, m, "flame1")) else { return };
    let a = crate::units::to_kh1(p0) + Vec3::new(0.0, c("flame_lift0"), 0.0);
    let b = crate::units::to_kh1(p1) + Vec3::new(0.0, c("flame_lift1"), 0.0);
    let f = &mut g[i];
    // KH1's ring (`0x7A6`): the line of `ring` − 1 frames before (its first frame's until then).
    let (a, b) = if sh.ring > 1 {
        if f.lines.is_empty() {
            f.lines.extend(std::iter::repeat((a, b)).take(sh.ring));
        } else {
            f.lines.push_back((a, b));
            f.lines.pop_front();
        }
        f.lines[0]
    } else {
        (a, b)
    };
    let spheres = kh1_core::fatbandit::flame_spheres(a, b, c("flame_r0"), c("flame_r1"), f.t, sh.grow, c("flame_grow_div"));
    let n = spheres.len().min(MAX_SPHERES);
    while f.shots.len() < n {
        let (centre, r) = spheres[f.shots.len()];
        let Some(s) = make(f, crate::units::to_recom(centre), yaw, r * crate::units::METRES_PER_UNIT) else { break };
        if f.serial == 0 {
            if let Some(o) = shot(s) {
                f.serial = func::<SerialFn>(SHOT_SERIAL)(o);
            }
        }
        f.shots.push(s);
    }
    // KH1's re-hit (`0x1402B3380`): the mask clears each time the clock passes 60.
    f.rehit += step;
    let again = f.rehit - c("flame_rehit") >= 0.0;
    if again {
        f.rehit -= c("flame_rehit");
    }
    let mut renewed = None;
    for (k, &s) in f.shots.iter().enumerate() {
        let Some(o) = shot(s) else { continue };
        let mover: usize = rd(o + rt::MOVER);
        let coll: usize = rd(o + crate::guard::bullet::COLL);
        // A sphere the chain has none for this frame (it shortened): parked inside the first, radius nil.
        let (centre, r) = spheres.get(k).copied().filter(|_| k < n).unwrap_or((a, 0.0));
        let p = crate::units::to_recom(centre);
        if mover != 0 {
            let w: [f32; 4] = rd(mover + rt::POSITION);
            wr(mover + rt::POSITION, [p.x, p.y, p.z, w[3]]);
        }
        if coll != 0 {
            func::<RadiusFn>(SET_RADIUS)(coll, (r * crate::units::METRES_PER_UNIT).max(0.001));
            if again {
                match renewed {
                    None => {
                        func::<PlFn>(NEW_SERIAL)(coll);
                        renewed = Some(rd::<u32>(coll + crate::guard::coll::SERIAL));
                    }
                    Some(sv) => wr(coll + crate::guard::coll::SERIAL, sv),
                }
            }
        }
        wr(o + rt::LIFE, LIFE);
    }
    if let Some(sv) = renewed {
        f.serial = sv;
    }
    if f.t == 0.0 || again {
        let (id, t, k) = (f.id, f.t, f.shots.len());
        line(|| format!("e{id}: flame frame {t:.0}: {n} spheres ({k} shots){}", if again { ", its hit-once mask cleared (KH1's re-hit)" } else { "" }));
    }
    f.t += step;
    // The Defender's tail (`0x6A4`): the parent no longer plays the request: 10 frames more, once.
    if let Some(tail) = sh.tail {
        if playing != Some(sh.request) && !f.cut {
            f.cut = true;
            let life = c("flame_life");
            if life - f.t - tail > 0.0 {
                f.t = life - tail;
            }
            let id = f.id;
            line(|| format!("e{id}: its flame's request stopped: {tail} frames more (KH1's tail)"));
        }
    }
}

/// The battle ended (or a new one began): the flames are gone.
pub(super) fn forget() {
    if let Ok(mut g) = FLAMES.lock() {
        g.clear();
    }
    BURNING.store(false, Ordering::Relaxed);
}

/// The trace's token: the flames burning (enemy, frames, spheres).
pub fn trace_token() -> String {
    let Ok(g) = FLAMES.try_lock() else { return String::new() };
    g.iter().map(|f| format!("flame e{}:{:.0}:{}", f.id, f.t, f.shots.len())).collect::<Vec<_>>().join(" ")
}
