//! M10: KH1's Shadow in Re:CoM's battles (`crate::shadow`, `docs/recom-m10-shadow.md`).
//!
//! - **Class swap.** A wrapper on `CEnemyShadow`'s ctor `0x140192770` (its only caller is the
//!   enemy factory `0x140188F40`, id 1): in a KH1 battle (Combat = KH1, Sora's story, decided by
//!   the battle enemy manager's ctor) with `enemy_ai = kh1`, the new Shadow's vtable becomes our
//!   copy of Re:CoM's (139 slots, the RTTI locator kept at `−8`), whose state slots run KH1's
//!   Shadow. A 0x20-byte tail after the body (the factory's size raised at `0x140188FAD`) marks it
//!   (magic, battle, our index). Field Shadows (`CMapEnemyShadow`) and Re:CoM battles are never
//!   touched.
//! - **States** (`crate::shadow::state`): Re:CoM's state machine calls them; each runs KH1's
//!   frame for this enemy: the behaviour (`kh1_core::shadow::Ai`) once a frame when no action
//!   plays, KH1's body physics (turn, walk, arc, air steering, push slide) written into Re:CoM's
//!   mover (`+0xF0`: yaw `+0x54`, velocity `+0x80`), KH1's motions played through Re:CoM's own
//!   motion play (vtable `+0x70` `0x14017D750`: the clip, its sounds `EN0001.SES` and its attack
//!   records `EN0001.ASD`, AP from Re:CoM's `@ENP`).
//! - **Reactions.** Re:CoM's hit handler `0x14017F7E0` takes the hit, applies Re:CoM's damage and
//!   picks state 11 / 12 / 30 as always; our state 11 / 12 begin turns the hit into KH1's reaction
//!   (`kh1_core::shadow::react`), state 30 runs Re:CoM's defeat with the KH1 motion.
//! - **Guard.** `super::guard` hands a blocked Shadow to [`guard_block`]: KH1's clash recoil.
//!
//! Per frame: one tail check per Shadow state call, the behaviour's frame, a few reads and writes.

use super::movement::{func, orig, rd, rd_vec, wr};
use super::{base, log};
use crate::shadow::{self as sh, state, tail, Manifest};
use crate::sites::{data, SHADOW_ALLOC, SHADOW_PTRS};
use kh1_core::shadow::{self as ks, Ai, Attack, Cmd, Hit, Phase, World};
use kh1_core::Vec3;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_CTOR: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static MANIFEST: OnceLock<Manifest> = OnceLock::new();
/// Our vtable copy (its slot 0's address) and Re:CoM's slots.
static COPY: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL: OnceLock<Vec<usize>> = OnceLock::new();
/// Re:CoM's hurt-part bones (head, hips) in `EN0001.MDL` for KH1's parts 0 / 1.
static PART_BONES: OnceLock<[i32; 2]> = OnceLock::new();
static LINES: AtomicU32 = AtomicU32::new(0);
const LINE_CAP: u32 = 20_000;
static SEED: AtomicU32 = AtomicU32::new(0x1234_5678);

/// Re:CoM functions (bytes checked: `sites::SHADOW_CALLEES` and the lists named there).
mod f {
    pub const ATTACK_END: u32 = 0x0017_E540;
    pub const DEFAULT_STATE: u32 = 0x0017_D6F0;
    pub const INTRO_ROOM: u32 = 0x001A_1680;
    pub const INTRO_FLAG: u32 = 0x001A_1670;
    pub const INTRO_COUNT: u32 = 0x001A_14B0;
    pub const HURT_OFF: u32 = 0x001A_2D70;
    /// `0x1401A2D90` / `0x1401A2DD0`: the pair Re:CoM's dig switches with the hurt bodies
    /// (`0x140192E60` case 0 / 4); checked in `M6`'s / our lists.
    pub const OTHER_OFF: u32 = 0x001A_2D90;
    pub const HURT_ON: u32 = 0x001A_2DB0;
    pub const OTHER_ON: u32 = 0x001A_2DD0;
    pub const CLOCK: u32 = 0x001A_47C0;
    pub const BONE: u32 = 0x002A_BB20;
    pub const HANDLE_OBJ: u32 = 0x002A_D350;
    pub const SCALE: u32 = 0x001A_4A20;
    pub const SCALE_SET: u32 = 0x001A_7050;
}
/// Enemy fields (Checked (decomp): `0x14017F7E0`, `0x14017C530`, `0x140185320`).
mod e {
    pub const ID: usize = 0x20;
    pub const DT: usize = 0x14;
    pub const STATE: usize = 0x60;
    pub const NEXT: usize = 0x68;
    pub const MOVER: usize = 0xF0;
    pub const HIT_INFO: usize = 0xF8;
    pub const FLAGS: usize = 0x260;
    pub const CHOSEN: usize = 0x278;
    /// Re:CoM's state 0 alternate path (`0x140192850`: `+0x260 & 0x20000000008`).
    pub const APPEAR_ALT: u64 = 0x200_0000_0008;
}
mod mv {
    pub const POS: usize = 0x30;
    pub const YAW: usize = 0x54;
    pub const VEL: usize = 0x80;
    pub const ACCEL: usize = 0x90;
    pub const CONTACT: usize = 0x120;
    pub const SPEED_SCALE: usize = 0x13C;
}
/// The hit info (`enemy+0xF8`): the hit attack `+0x50` (its Re:CoM kind `+0x80`, attacker handle
/// `+0x98`), the body parts `+0x40` (count `+0x38`, 0x120 each; contact point `+0x50`).
mod hi {
    pub const ATTACK: usize = 0x50;
    pub const KIND: usize = 0x80;
    pub const OWNER: usize = 0x98;
    pub const PARTS: usize = 0x40;
    pub const COUNT: usize = 0x38;
    pub const CONTACT: usize = 0x50;
}

type CtorFn = unsafe extern "win64" fn(usize, usize, usize, usize) -> usize;
type StateFn = unsafe extern "win64" fn(usize);
type EndFn = unsafe extern "win64" fn(usize, usize, i32);
type IntFn = unsafe extern "win64" fn() -> i32;
type BodiesFn = unsafe extern "win64" fn(usize, i32);
type ClockFn = unsafe extern "win64" fn(usize) -> i32;
type BoneFn = unsafe extern "win64" fn(usize, i32) -> usize;
type HandleFn = unsafe extern "win64" fn(u32) -> usize;
type ScaleFn = unsafe extern "win64" fn(usize) -> usize;
type ScaleSetFn = unsafe extern "win64" fn(usize, *const [f32; 4]);
type PlayFn = unsafe extern "win64" fn(usize, u32, u64, u64, i32, u32, u32);

/// A running action (an attack, or the leap's jump start): the AI waits for it.
#[derive(Clone, Copy, Debug)]
struct Act {
    request: u16,
    then: u16,
    attack: Option<Attack>,
    blend: f32,
}

struct Runtime {
    enemy: usize,
    battle: u32,
    ai: Ai,
    current: Option<u16>,
    queued: Option<(u16, f32)>,
    action: Option<Act>,
    /// The action motion has started (its clock now counts).
    action_playing: bool,
    reaction: Option<Phase>,
    /// A launch's lie request (`base + 3` / `+ 8`) while lying.
    lie: Option<u16>,
    clash: Option<Attack>,
    last_pos: Vec3,
    /// Re:CoM's own state 0 (its alternate appearance) runs.
    recom_appear: bool,
    appear_sub: u8,
    dead: bool,
    /// Frames since the defeat (the trace stops listing it once Re:CoM removed it).
    dead_frames: u32,
    /// KH1's flag 0x40 clear (Re:CoM's hurt bodies on).
    hittable: bool,
    /// The last launch's down time (frames).
    down: f32,
}

// SAFETY: a runtime (its behaviour holds `Rc`s that never leave it) is only touched under `RT`'s
// lock; the ctor may run on a loading thread and the states on the game thread.
unsafe impl Send for Runtime {}

/// Every KH1 Shadow's runtime (this battle's; older ones are dropped at the next ctor).
static RT: Mutex<Vec<Runtime>> = Mutex::new(Vec::new());

fn line(s: impl FnOnce() -> String) {
    if LINES.fetch_add(1, Ordering::Relaxed) < LINE_CAP {
        log(&format!("shadow: {}", s()));
    }
}

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Relaxed)
}

fn live_battle() -> bool {
    installed() && super::battle::kh1_battle() && super::settings().enemy_ai == crate::settings::Mode::Kh1
}

/// `super::stats`: our vtable copy stands for `CEnemyShadow`'s.
pub(super) fn canonical_vtable(vt: usize) -> usize {
    let c = COPY.load(Ordering::Relaxed);
    if c != 0 && vt == c {
        base() + sh::VTABLE as usize
    } else {
        vt
    }
}

/// Loads the manifest, checks the vtable and the factory size, builds the vtable copy, patches the
/// size. The ctor hook is installed by the caller after this.
pub unsafe fn prepare(bs: usize, manifest: Option<&std::path::Path>) -> bool {
    let Some(p) = manifest else {
        log("shadow: no kh1-shadow.json (run kh1-recom-build): Shadows keep Re:CoM's behaviour");
        return false;
    };
    let text = match std::fs::read_to_string(p) {
        Ok(t) => t,
        Err(e) => {
            log(&format!("shadow: {}: {e}; Shadows keep Re:CoM's behaviour", p.display()));
            return false;
        }
    };
    let (m, parts) = match sh::parse(&text).and_then(|m| parse_parts(&text).map(|p| (m, p))) {
        Ok(x) => x,
        Err(e) => {
            log(&format!("shadow: {}: {e}; Shadows keep Re:CoM's behaviour", p.display()));
            return false;
        }
    };
    let vt = bs + sh::VTABLE as usize;
    if super::stats::rtti_name(bs, vt).as_deref() != Some(".?AVCEnemyShadow@@") {
        log("shadow: CEnemyShadow's vtable is not at 0x14054D728: not installed");
        return false;
    }
    if let Some(p) = SHADOW_PTRS.iter().find(|p| rd::<usize>(bs + p.rva as usize) != bs + p.target as usize) {
        log(&format!("shadow: {} is not the function expected: not installed", p.name));
        return false;
    }
    // The copy: Re:CoM's locator, then its 139 slots with ours in the state slots.
    let original: Vec<usize> = (0..sh::VTABLE_SLOTS).map(|i| rd::<usize>(vt + 8 * i)).collect();
    let mut copy: Vec<usize> = Vec::with_capacity(sh::VTABLE_SLOTS + 1);
    copy.push(rd::<usize>(vt - 8));
    copy.extend_from_slice(&original);
    let mut set = |off: usize, f: usize| copy[1 + off / 8] = f;
    set(sh::begin_slot(state::APPEAR), appear_begin as *const () as usize);
    set(sh::step_slot(state::APPEAR), appear_step as *const () as usize);
    set(sh::begin_slot(state::THINK), think_begin as *const () as usize);
    set(sh::step_slot(state::THINK), think_step as *const () as usize);
    for k in [state::CLAW, state::LEAP, state::SWIPE] {
        set(sh::begin_slot(k), attack_begin as *const () as usize);
        set(sh::step_slot(k), attack_step as *const () as usize);
    }
    for k in [state::FLINCH, state::BLOW] {
        set(sh::begin_slot(k), react_begin as *const () as usize);
        set(sh::step_slot(k), react_step as *const () as usize);
    }
    set(sh::begin_slot(state::DEFEAT), defeat_begin as *const () as usize);
    set(sh::step_slot(state::DEFEAT), defeat_step as *const () as usize);
    set(sh::THINK_SLOT, think_none as *const () as usize);
    let leaked: &'static mut [usize] = Box::leak(copy.into_boxed_slice());
    let at = leaked.as_ptr() as usize + 8;
    // The allocation size (all-or-nothing with the rest: a failed write leaves everything off).
    let a = &SHADOW_ALLOC[0];
    if !super::hook::patch(bs + a.rva as usize, &sh::alloc_bytes(sh::ALLOC), &sh::alloc_bytes(sh::ALLOC + tail::SIZE), a.name) {
        log("shadow: the factory's size held other bytes: not installed");
        return false;
    }
    let _ = ORIGINAL.set(original);
    let _ = PART_BONES.set(parts);
    let _ = MANIFEST.set(m);
    COPY.store(at, Ordering::Release);
    true
}

/// Puts the size back (the ctor hook failed).
pub unsafe fn unprepare(bs: usize) {
    let a = &SHADOW_ALLOC[0];
    super::hook::patch(bs + a.rva as usize, &sh::alloc_bytes(sh::ALLOC + tail::SIZE), &sh::alloc_bytes(sh::ALLOC), a.name);
    COPY.store(0, Ordering::Release);
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

pub fn start_line() -> String {
    let m = MANIFEST.get();
    format!(
        "shadow: KH1's Shadow (M10) at combat = kh1 in Sora's story with enemy_ai = kh1 (read at each battle start): ctor 0x140192770 wrapped, vtable copy at {:#x} (states 0, 1, 2, 3, 4, 11, 12, 30, think 0x3D0), size 0xCE0 -> 0xD00 at 0x140188FAD; {} clips, recoil {:?}",
        COPY.load(Ordering::Relaxed),
        m.map_or(0, |m| m.clips.len()),
        m.map(|m| m.recoil)
    )
}

fn parse_parts(text: &str) -> Result<[i32; 2], String> {
    let v = crate::json::parse(text)?;
    let a = v.get("parts").and_then(|a| a.as_array()).ok_or("no \"parts\"")?;
    if a.len() != 2 {
        return Err("\"parts\" needs 2 bones".into());
    }
    let n = |i: usize| a[i].as_u32().filter(|&b| b < 256).ok_or("bad part bone").map(|b| b as i32);
    Ok([n(0)?, n(1)?])
}

// ---------------------------------------------------------------------------------------------
// The ctor and the runtime lookup.
// ---------------------------------------------------------------------------------------------

/// `CEnemyShadow`'s ctor: Re:CoM's, then (KH1 battle, `enemy_ai = kh1`) our vtable and tail.
pub unsafe extern "win64" fn ctor(this: usize, a: usize, b: usize, c: usize) -> usize {
    let r = orig::<CtorFn>(&ORIG_CTOR)(this, a, b, c);
    if this == 0 || !live_battle() {
        return r;
    }
    let copy = COPY.load(Ordering::Acquire);
    let Some(m) = MANIFEST.get() else { return r };
    if copy == 0 || rd::<usize>(this) != base() + sh::VTABLE as usize {
        return r;
    }
    let battle = super::battle::battle_number();
    let seed = SEED.fetch_add(0x9E37_79B9, Ordering::Relaxed) ^ (this as u32);
    // Re:CoM's ctor doesn't make the mover (`+0xF0` is still 0 here: 0.67.0's crash at every battle
    // with a Shadow, owner 2026-11-02); the facing is taken again at state 0's begin (`appear_begin`).
    let m0: usize = rd(this + e::MOVER);
    let yaw = if m0 != 0 { crate::units::yaw_to_kh1(rd::<f32>(m0 + mv::YAW)) } else { 0.0 };
    let ai = Ai::new(m.consts.clone(), m.frames(), seed, yaw, false);
    let idx = (|| {
        let Ok(mut v) = RT.lock() else { return None };
        v.retain(|x| x.battle == battle && x.enemy != this);
        v.push(Runtime {
            enemy: this,
            battle,
            ai,
            current: None,
            queued: None,
            action: None,
            action_playing: false,
            reaction: None,
            lie: None,
            clash: None,
            last_pos: Vec3::ZERO,
            recom_appear: false,
            appear_sub: 0,
            dead: false,
            dead_frames: 0,
            hittable: true,
            down: 0.0,
        });
        Some(v.len() - 1)
    })();
    let Some(idx) = idx else { return r };
    wr(this, copy);
    wr(this + tail::AT + tail::OFF_MAGIC, tail::MAGIC);
    wr(this + tail::AT + tail::OFF_BATTLE, battle);
    wr(this + tail::AT + tail::OFF_INDEX, idx as u32);
    line(|| format!("battle {battle}: Shadow {this:#x} gets KH1's AI (seed {seed:#x})"));
    r
}

/// Whether an enemy is one of ours (our vtable and tail, this battle).
pub(super) unsafe fn ours(en: usize) -> bool {
    let c = COPY.load(Ordering::Relaxed);
    c != 0 && rd::<usize>(en) == c && rd::<u32>(en + tail::AT + tail::OFF_MAGIC) == tail::MAGIC
}

/// Runs `f` on the enemy's runtime; None when it has none (then the caller runs Re:CoM's slot).
unsafe fn with_rt<T>(en: usize, f: impl FnOnce(&mut Runtime) -> T) -> Option<T> {
    if !ours(en) {
        return None;
    }
    let battle: u32 = rd(en + tail::AT + tail::OFF_BATTLE);
    let idx: u32 = rd(en + tail::AT + tail::OFF_INDEX);
    // `try_lock`: a state call re-entered from inside another (never expected) runs Re:CoM's slot.
    let mut v = RT.try_lock().ok()?;
    let r = v.get_mut(idx as usize).filter(|r| r.enemy == en && r.battle == battle)?;
    Some(f(r))
}

unsafe fn original(off: usize) -> usize {
    ORIGINAL.get().map_or(0, |o| o[off / 8])
}

unsafe fn call_original(en: usize, off: usize) {
    let f = original(off);
    if f != 0 {
        let g: StateFn = std::mem::transmute_copy(&f);
        g(en);
    }
}

// ---------------------------------------------------------------------------------------------
// The KH1 frame.
// ---------------------------------------------------------------------------------------------

fn manifest() -> &'static Manifest {
    MANIFEST.get().expect("installed")
}

unsafe fn mover(en: usize) -> usize {
    rd(en + e::MOVER)
}

unsafe fn sora_pos() -> Option<Vec3> {
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    if pl == 0 {
        return None;
    }
    let m: usize = rd(pl + e::MOVER);
    (m != 0).then(|| crate::units::to_kh1(rd_vec(m + mv::POS)))
}

unsafe fn clock(en: usize) -> i32 {
    func::<ClockFn>(f::CLOCK)(en)
}

/// Plays a KH1 request through Re:CoM's motion play (vtable `+0x70`): its clip, root flag, the
/// blend in Re:CoM's clock units (KH1 frames × 50; Assumption: the play's blend argument counts
/// the same units as the clock).
unsafe fn play(en: usize, rt: &mut Runtime, request: u16, blend: f32) {
    let Some(c) = manifest().clip(request) else {
        line(|| format!("{en:#x}: no clip for KH1 request {request}"));
        return;
    };
    let vt: usize = rd(en);
    let p: PlayFn = std::mem::transmute_copy(&rd::<usize>(vt + 0x70));
    p(en, c.slot, 0, c.root as u64, sh::blend_units(blend), 0, 0);
    rt.current = Some(request);
}

/// The current request's clip has played to KH1's length.
unsafe fn ended(en: usize, rt: &Runtime) -> bool {
    match rt.current.and_then(|r| manifest().clip(r)) {
        Some(c) => clock(en) >= crate::units::frames_to_units(c.frames as f32),
        None => true,
    }
}

/// KH1's hittable flag (actor flag `0x40`, `kh1_core::shadow::Cmd::Hittable`) as Re:CoM's own
/// hurt-body switch: every hurt body off (`0x1401A2D70(enemy, −1)`) or on (`0x1401A2DB0`), what
/// Re:CoM's dig does (`0x140192E60` case 0 / 5). Re:CoM's hit handler, the KH1 swings' pair tests
/// and the spells' overlap tests only meet bodies that are on. The lock-on flag (`+0x154` bit
/// 0x10000, `0x1401CDB90`) stays: KH1's lock button (`0x1402B1540`) can lock a sunk Shadow too.
unsafe fn hurt(en: usize, on: bool) {
    func::<BodiesFn>(if on { f::HURT_ON } else { f::HURT_OFF })(en, -1);
}

unsafe fn bodies(en: usize, on: bool) {
    if on {
        func::<BodiesFn>(f::HURT_ON)(en, -1);
        func::<BodiesFn>(f::OTHER_ON)(en, -1);
    } else {
        func::<BodiesFn>(f::HURT_OFF)(en, -1);
        func::<BodiesFn>(f::OTHER_OFF)(en, -1);
    }
}

/// KH1's `+0x44` (the y of the scale) as Re:CoM's model scale y (`0x1401A4A20` / `0x1401A7050`,
/// mover `+0x70`: what Re:CoM's own dig squashes, `0x140193420`).
unsafe fn set_height(en: usize, h: f32) {
    let p = func::<ScaleFn>(f::SCALE)(en);
    if p == 0 {
        return;
    }
    let mut s: [f32; 4] = rd(p);
    s[1] = h.max(0.0);
    func::<ScaleSetFn>(f::SCALE_SET)(en, &s);
}

/// Enters a Re:CoM attack state as P1's cardless play does (`crate::battle::cardless_flags`).
unsafe fn enter_state(en: usize, k: i32) {
    let flags: u64 = rd(en + e::FLAGS);
    wr(en + e::FLAGS, crate::battle::cardless_flags(flags));
    wr(en + e::CHOSEN, k);
    wr(en + e::NEXT, crate::battle::state_address(en, k));
}

unsafe fn apply(en: usize, rt: &mut Runtime, c: &Cmd) {
    match *c {
        Cmd::Motion { request, blend } => {
            if rt.current != Some(request) {
                rt.queued = None;
                play(en, rt, request, blend);
            }
        }
        Cmd::Queue { request, blend } => rt.queued = Some((request, blend)),
        Cmd::Action { request, attack, then, blend } => {
            rt.action = Some(Act { request, then, attack, blend });
            rt.action_playing = false;
            match attack {
                // An attack: Re:CoM's attack state; its begin plays the motion.
                Some(a) => enter_state(en, sh::attack_state(a)),
                None => {
                    play(en, rt, request, blend);
                    rt.queued = Some((then, 0.0));
                    rt.action_playing = true;
                }
            }
        }
        // Log only: what can hit it is KH1's flag 0x40 (`Hittable`), not the underground state.
        Cmd::Underground(_) => {}
        Cmd::Hittable(h) => {
            hurt(en, h);
            rt.hittable = h;
        }
        Cmd::Height(h) => set_height(en, h),
        Cmd::FadeIn(n) => {
            // KH1's 30-frame alpha fade-in of the first rise (`0x2570`): no Re:CoM model alpha found
            // to write yet (`docs/recom-m10-shadow.md` § Not built); the height ramp shows the rise.
            let _ = n;
        }
        Cmd::Yaw(_) | Cmd::Jump { .. } | Cmd::Target(_) | Cmd::State(_) => {}
    }
    if sh::logged(c) {
        let id: u32 = rd(en + e::ID);
        line(|| format!("e{id}: {}", sh::cmd_token(c)));
    }
}

/// One KH1 frame of this Shadow: landing, the motion queue, the action's end, the behaviour (when
/// `poll`), the body physics into Re:CoM's mover.
unsafe fn tick(en: usize, rt: &mut Runtime, poll: bool) {
    let mvr = mover(en);
    if mvr == 0 {
        return;
    }
    let dt: i32 = rd(en + e::DT);
    let step = crate::units::step_from_dt(dt.clamp(0, 1000));
    let pos = crate::units::to_kh1(rd_vec(mvr + mv::POS));
    let moved = (pos - rt.last_pos).length();
    rt.last_pos = pos;
    let contact = rd::<u32>(mvr + mv::CONTACT) & 5 != 0;
    if contact && rt.ai.body().falling() {
        rt.ai.body().land();
        landed(en, rt);
    }
    // The motion that ended: the queued one plays; an action ends.
    if ended(en, rt) {
        if let Some(a) = rt.action.filter(|_| rt.action_playing) {
            if rt.current == Some(a.request) && a.attack.is_none() {
                rt.action = None;
                rt.action_playing = false;
            }
        }
        if let Some((q, b)) = rt.queued.take() {
            play(en, rt, q, b);
        }
    }
    let airborne = rt.ai.body().airborne();
    if poll && rt.action.is_none() {
        let w = World { step, pos, target: sora_pos(), grounded: contact && !airborne, airborne, moved, motion: rt.current };
        let cmds = rt.ai.frame(w);
        for c in &cmds {
            apply(en, rt, c);
        }
        let draws = rt.ai.take_draws();
        if !draws.is_empty() {
            let id: u32 = rd(en + e::ID);
            line(|| format!("e{id}: rand {}", draws.iter().map(|d| format!("{}={:.4}", d.what, d.value)).collect::<Vec<_>>().join(", ")));
        }
    }
    body_step(en, rt, step, pos);
}

/// KH1's body physics for the frame, written into Re:CoM's mover: the facing, and the velocity
/// that moves it by KH1's displacement (horizontal always; vertical while in a KH1 arc, with
/// Re:CoM's acceleration cancelled).
unsafe fn body_step(en: usize, rt: &mut Runtime, step: f32, pos: Vec3) {
    let mvr = mover(en);
    let c = manifest().consts.clone();
    let steer = rt.ai.take_steer();
    let airborne = rt.ai.body().airborne();
    let d = rt.ai.body().step(step, pos.y, steer, &c);
    let yaw = rt.ai.body().yaw;
    wr(mvr + mv::YAW, crate::units::yaw_to_recom(yaw));
    let dr = crate::units::to_recom(d);
    let accel = rd_vec(mvr + mv::ACCEL);
    let v = sh::velocity(dr, accel, step);
    let scale: f32 = rd(mvr + mv::SPEED_SCALE);
    let k = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let mut vel: [f32; 4] = rd(mvr + mv::VEL);
    vel[0] = v.x * k;
    vel[2] = v.z * k;
    if airborne {
        vel[1] = v.y;
    }
    wr(mvr + mv::VEL, vel);
}

/// The body touched the floor at the end of a KH1 arc (`0x1402B7260`'s landing).
unsafe fn landed(en: usize, rt: &mut Runtime) {
    match rt.reaction {
        Some(Phase::Air { base, behind }) => {
            let (r, then, lying) = ks::landing(base, behind);
            rt.queued = None;
            play(en, rt, r, 0.0);
            rt.queued = Some((then, 0.0));
            rt.reaction = Some(match (lying, base) {
                (true, Some(b)) => {
                    rt.lie = Some(then);
                    Phase::Down { base: b, behind, left: rt.down }
                }
                _ => Phase::Motion,
            });
        }
        _ => {
            // A plain landing (the leap): KH1's request 7 then the queue (`0x1402B7260`).
            if rt.action.is_none() && rt.reaction.is_none() {
                let q = rt.queued.take();
                play(en, rt, ks::req::LAND, 0.0);
                rt.queued = q.or(Some((ks::req::IDLE, 0.0)));
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The states.
// ---------------------------------------------------------------------------------------------

/// State 0 begin: Re:CoM's alternate appearance runs as Re:CoM's; else Re:CoM's begin without its
/// motion (bodies off until the rise), and KH1's rise starts with the behaviour.
pub unsafe extern "win64" fn appear_begin(en: usize) {
    let alt = rd::<u64>(en + e::FLAGS) & e::APPEAR_ALT != 0;
    let done = with_rt(en, |rt| {
        rt.recom_appear = alt;
        rt.appear_sub = 0;
        let m = mover(en);
        if m != 0 {
            rt.last_pos = crate::units::to_kh1(rd_vec(m + mv::POS));
            // The facing the spawn gave it (set after the ctor).
            let y = crate::units::yaw_to_kh1(rd(m + mv::YAW));
            let mut b = rt.ai.body();
            b.yaw = y;
            b.yaw_target = y;
            drop(b);
        }
        if alt {
            rt.ai.skip_rise();
        }
    });
    if done.is_none() || alt {
        return call_original(en, sh::begin_slot(state::APPEAR));
    }
    // `0x140192850`'s writes but its motion: `+0x154 &= ~1`, both body sets off.
    let f154: u32 = rd(en + 0x154);
    wr(en + 0x154, f154 & !1);
    bodies(en, false);
    line(|| format!("e{}: appears (KH1's rise)", rd::<u32>(en + e::ID)));
}

/// State 0 step: KH1's rise; when it is done, Re:CoM's battle-start handshake (`0x140192B00`).
pub unsafe extern "win64" fn appear_step(en: usize) {
    let r = with_rt(en, |rt| {
        if rt.recom_appear {
            return None;
        }
        match rt.appear_sub {
            0 => {
                tick(en, rt, true);
                if !rt.ai.risen() {
                    return Some(());
                }
                if func::<IntFn>(f::INTRO_ROOM)() != 0 && func::<IntFn>(f::INTRO_FLAG)() != 0 {
                    func::<IntFn>(f::INTRO_COUNT)();
                    rt.appear_sub = 1;
                } else {
                    func::<StateFn>(f::DEFAULT_STATE)(en);
                }
            }
            _ => {
                if func::<IntFn>(f::INTRO_FLAG)() == 0 {
                    func::<StateFn>(f::DEFAULT_STATE)(en);
                }
            }
        }
        Some(())
    });
    if r.flatten().is_none() {
        call_original(en, sh::step_slot(state::APPEAR));
    }
}

/// State 1 begin: no Re:CoM idle / walk pick; KH1's motion carries on.
pub unsafe extern "win64" fn think_begin(en: usize) {
    if with_rt(en, |rt| {
        if rt.current.is_none() {
            play(en, rt, ks::req::IDLE, 8.0);
        }
    })
    .is_none()
    {
        call_original(en, sh::begin_slot(state::THINK));
    }
}

/// State 1 step: KH1's frame with the behaviour.
pub unsafe extern "win64" fn think_step(en: usize) {
    if with_rt(en, |rt| tick(en, rt, true)).is_none() {
        call_original(en, sh::step_slot(state::THINK));
    }
}

/// The think (`+0x3D0`): nothing; KH1's behaviour picks the attacks.
pub unsafe extern "win64" fn think_none(_en: usize) {}

/// States 2 / 3 / 4 begin: the attack's KH1 motion (its records attack through `EN0001.ASD`).
pub unsafe extern "win64" fn attack_begin(en: usize) {
    let k = crate::guard::state_index(en, rd(en + e::NEXT)).or_else(|| crate::guard::state_index(en, rd(en + e::STATE)));
    let done = with_rt(en, |rt| {
        let Some(a) = rt.action.filter(|a| a.attack.is_some()) else { return false };
        play(en, rt, a.request, a.blend);
        rt.queued = Some((a.then, a.blend));
        rt.action_playing = true;
        true
    });
    if done != Some(true) {
        // Not started by our behaviour (never expected): Re:CoM's begin.
        let s = k.map_or(state::CLAW, |k| k as i32);
        call_original(en, sh::begin_slot(s));
    }
}

/// States 2 / 3 / 4 step: KH1's frame without the behaviour; at the motion's end, Re:CoM's attack
/// end (`0x14017E540`: bit 0 off, the default state).
pub unsafe extern "win64" fn attack_step(en: usize) {
    let r = with_rt(en, |rt| {
        let end = rt.action.is_some() && ended(en, rt);
        if end {
            rt.action = None;
            rt.action_playing = false;
        }
        tick(en, rt, false);
        end
    });
    match r {
        Some(true) => func::<EndFn>(f::ATTACK_END)(en, 0, 0),
        Some(false) => {}
        None => {
            let s = crate::guard::state_index(en, rd(en + e::STATE)).map_or(state::CLAW, |k| k as i32);
            call_original(en, sh::step_slot(s));
        }
    }
}

/// The hit Re:CoM's handler took (its hit info): KH1's kind (a KH1 swing's from its row, else from
/// Re:CoM's kind), the push direction, the hurt part.
unsafe fn hit_of(en: usize) -> Option<Hit> {
    let info: usize = rd(en + e::HIT_INFO);
    if info == 0 {
        return None;
    }
    let atk: usize = rd(info + hi::ATTACK);
    if mover(en) == 0 {
        return None;
    }
    let me = crate::units::to_kh1(rd_vec(mover(en) + mv::POS));
    let swing = take_swing(en);
    let kind = swing.or_else(|| (atk != 0).then(|| sh::kh1_kind_of_recom(rd(atk + hi::KIND))))?;
    // The attacker: its handle's actor (`0x14017C530`'s rule), else Sora.
    let owner = if atk != 0 { func::<HandleFn>(f::HANDLE_OBJ)(rd(atk + hi::OWNER)) } else { 0 };
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    let dir = if owner != 0 && owner == pl {
        let pm: usize = rd(pl + e::MOVER);
        let sp = crate::units::to_kh1(rd_vec(pm + mv::POS));
        let sy = crate::units::yaw_to_kh1(rd(pm + mv::YAW));
        sh::swing_dir(sp, sy, me, std::f32::consts::FRAC_PI_4)
    } else {
        let from = if owner != 0 { crate::units::to_kh1(rd_vec(func::<BoneFn>(f::BONE)(owner, 0) + 0xD0)) } else { sora_pos().unwrap_or(me) };
        let d = (me - from).flat();
        if d.horizontal_length() > 1e-3 {
            d.normalized()
        } else {
            Vec3::from_yaw(with_rt(en, |rt| rt.ai.body().yaw).unwrap_or(0.0) + std::f32::consts::PI)
        }
    };
    // The part: the contact point against the head and hip bones.
    let n: i32 = rd(info + hi::COUNT);
    let parts: usize = rd(info + hi::PARTS);
    let part = match (PART_BONES.get(), n > 0 && parts != 0) {
        (Some(b), true) => {
            let contact = rd_vec(parts + hi::CONTACT);
            let bone = |i: i32| rd_vec(func::<BoneFn>(f::BONE)(en, i) + 0xD0);
            sh::part_of(contact, bone(b[0]), bone(b[1]))
        }
        _ => 0,
    };
    Some(Hit { kind, dir, part })
}

/// KH1 swings noted by the enemy-hit wrapper (`note_swing`), by enemy.
static SWING: std::sync::Mutex<Vec<(usize, u8)>> = std::sync::Mutex::new(Vec::new());

/// `super::attack`: a KH1 swing hit this enemy with KH1's reaction kind `kind`.
pub(super) unsafe fn note_swing(en: usize, kind: u8) {
    if !installed() || !ours(en) {
        return;
    }
    if let Ok(mut g) = SWING.lock() {
        g.retain(|x| x.0 != en);
        if g.len() < 64 {
            g.push((en, kind));
        }
    }
}

fn take_swing(en: usize) -> Option<u8> {
    let mut g = SWING.lock().ok()?;
    let i = g.iter().position(|x| x.0 == en)?;
    Some(g.swap_remove(i).1)
}

/// Starts KH1's reaction for the hit (or the clash) on this Shadow; false: nothing to play.
unsafe fn start_reaction(en: usize, rt: &mut Runtime, hit: Option<Hit>) -> bool {
    let m = manifest();
    let c = m.consts.clone();
    let y = crate::units::to_kh1(rd_vec(mover(en) + mv::POS)).y;
    let airborne = rt.ai.body().airborne();
    rt.action = None;
    rt.action_playing = false;
    if let Some(a) = rt.clash.take() {
        let s = ks::clash(a, airborne, &mut rt.ai.body(), y, &c, [m.recoil_of(Attack::Claw), m.recoil_of(Attack::Swipe), m.recoil_of(Attack::LeapClaw)]);
        rt.queued = None;
        play(en, rt, s.request, 0.0);
        rt.queued = Some((s.then, c.attack_blend()));
        rt.reaction = Some(if airborne { Phase::Air { base: None, behind: false } } else { Phase::Motion });
        line(|| format!("e{}: clash recoil {} ({})", rd::<u32>(en + e::ID), s.request, a.name()));
        return true;
    }
    let Some(hit) = hit else { return false };
    let lying = matches!(rt.reaction, Some(Phase::Down { .. })) && rt.current == rt.lie;
    let current = rt.current;
    // The behaviour's random stream (KH1's one `rand`; the launch's down time draws from it).
    let mut rng = rt.ai.with_rng(|r| r.clone());
    let yaw = rt.ai.body().yaw;
    let start = {
        let mut body = rt.ai.body();
        ks::react(hit, yaw, current, airborne, lying, m, &c, &mut rng, &mut body, y)
    };
    rt.ai.with_rng(|r| *r = rng);
    let id: u32 = rd(en + e::ID);
    match start {
        None => {
            line(|| format!("e{id}: hit kind {} part {} while lying: push only", hit.kind, hit.part));
            true
        }
        Some(s) => {
            rt.down = s.down;
            rt.queued = None;
            play(en, rt, s.request, 0.0);
            rt.queued = Some((s.then, c.attack_blend()));
            let behind = s.base.is_some_and(|b| s.request >= b + 5);
            rt.reaction = Some(if rt.ai.body().airborne() { Phase::Air { base: s.base, behind } } else { Phase::Motion });
            line(|| format!("e{id}: reaction kind {} part {} dir ({:.2},{:.2}) -> {} then {}{}", hit.kind, hit.part, hit.dir.x, hit.dir.z, s.request, s.then, if s.base.is_some() { format!(" down {:.1}", s.down) } else { String::new() }));
            true
        }
    }
}

/// States 11 / 12 begin: KH1's reaction.
pub unsafe extern "win64" fn react_begin(en: usize) {
    let hit = if ours(en) { hit_of(en) } else { None };
    let done = with_rt(en, |rt| {
        // KH1's hit handler (`0x2988`): a damaging hit sets the "hit" state; a clash is no hit.
        if rt.clash.is_none() {
            rt.ai.on_hit();
        }
        start_reaction(en, rt, hit)
    });
    match done {
        Some(true) => {}
        Some(false) => func::<StateFn>(f::DEFAULT_STATE)(en),
        None => {
            let s = crate::guard::state_index(en, rd(en + e::NEXT)).map_or(state::FLINCH, |k| k as i32);
            call_original(en, sh::begin_slot(s));
        }
    }
}

/// States 11 / 12 step: KH1's frame without the behaviour; the reaction's phases.
pub unsafe extern "win64" fn react_step(en: usize) {
    let r = with_rt(en, |rt| {
        let dt: i32 = rd(en + e::DT);
        let step = crate::units::step_from_dt(dt.clamp(0, 1000));
        let mut done = false;
        match rt.reaction {
            Some(Phase::Motion) if ended(en, rt) && rt.queued.is_none() => done = true,
            Some(Phase::Motion) => {
                // The reaction's motion ended this frame: `tick` plays the queued one; then done.
                if ended(en, rt) {
                    done = true;
                }
            }
            Some(Phase::Down { base, behind, left }) => {
                if rt.current == rt.lie {
                    let l = left - step;
                    if l <= 0.0 {
                        rt.queued = None;
                        play(en, rt, ks::get_up(base, behind), 0.0);
                        rt.queued = Some((ks::req::IDLE, manifest().consts.attack_blend()));
                        rt.reaction = Some(Phase::Motion);
                        rt.lie = None;
                    } else {
                        rt.reaction = Some(Phase::Down { base, behind, left: l });
                    }
                }
            }
            _ => {}
        }
        tick(en, rt, false);
        if done {
            rt.reaction = None;
        }
        done
    });
    match r {
        Some(true) => func::<StateFn>(f::DEFAULT_STATE)(en),
        Some(false) => {}
        None => {
            let s = crate::guard::state_index(en, rd(en + e::STATE)).map_or(state::FLINCH, |k| k as i32);
            call_original(en, sh::step_slot(s));
        }
    }
}

/// State 30 begin: Re:CoM's defeat (prizes, EXP, its removal timer), then the killing hit's KH1
/// reaction motion.
pub unsafe extern "win64" fn defeat_begin(en: usize) {
    let hit = if ours(en) { hit_of(en) } else { None };
    call_original(en, sh::begin_slot(state::DEFEAT));
    with_rt(en, |rt| {
        rt.dead = true;
        if !start_reaction(en, rt, hit) {
            play(en, rt, ks::req::FLINCH, 0.0);
            rt.reaction = Some(Phase::Motion);
        }
        line(|| format!("e{}: defeated (Re:CoM's defeat, KH1's motion {:?})", rd::<u32>(en + e::ID), rt.current));
    });
}

/// State 30 step: KH1's body (the launch arc of the last hit), then Re:CoM's step (removal).
pub unsafe extern "win64" fn defeat_step(en: usize) {
    with_rt(en, |rt| {
        rt.dead_frames = rt.dead_frames.saturating_add(1);
        tick(en, rt, false)
    });
    call_original(en, sh::step_slot(state::DEFEAT));
}

/// `super::guard`: Sora's Guard blocked this enemy's attack. A KH1 Shadow plays KH1's clash recoil
/// (`0x1402B8E10` cases 6 / 7 → `0x1402B9140`) through state 11; true when handled.
pub(super) unsafe fn guard_block(en: usize) -> bool {
    if !installed() || !ours(en) {
        return false;
    }
    let attack = with_rt(en, |rt| {
        let a = rt.action.and_then(|a| a.attack);
        rt.clash = a;
        a
    })
    .flatten();
    let Some(a) = attack else { return false };
    // Re:CoM's attack end with state 11 next (bit 0 off, its card end).
    func::<EndFn>(f::ATTACK_END)(en, crate::battle::state_address(en, state::FLINCH), 0);
    line(|| format!("e{}: Guard blocked its {}", rd::<u32>(en + e::ID), a.name()));
    true
}

/// The trace's token (F9 recording): every KH1 Shadow's request, reaction, action.
pub unsafe fn trace_token() -> String {
    if !installed() {
        return String::new();
    }
    {
        let Ok(v) = RT.try_lock() else { return String::new() };
        v.iter()
            // Re:CoM removes a defeated enemy after its 1500-unit (30-frame) timer: not listed after.
            .filter(|r| r.battle == super::battle::battle_number() && !(r.dead && r.dead_frames > 31))
            .take(8)
            .map(|r| {
                format!(
                    "{:x}:r{:?}:{:?}:{}:h{}",
                    r.enemy & 0xFFFFFF,
                    r.current,
                    r.reaction.map(|p| match p {
                        Phase::Motion => 'm',
                        Phase::Air { .. } => 'a',
                        Phase::Down { .. } => 'd',
                    }),
                    r.action.map_or(0, |a| a.request),
                    r.hittable as u8
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
