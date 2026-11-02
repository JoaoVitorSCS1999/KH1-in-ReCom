//! M10 / M11: KH1's enemies on Re:CoM's own enemy states (`crate::kh1_enemy`,
//! `docs/recom-m10-shadow.md`, `docs/recom-m11-enemies.md`). The shared runtime, generalized from
//! M10's Shadow runtime; one table (`crate::kh1_enemy::CLASSES`) drives it.
//!
//! - **Class swap.** A wrapper on each class's ctor (its only caller is the enemy factory
//!   `0x140188F40`): in a KH1 battle (Combat = KH1, Sora's story, decided by the battle enemy
//!   manager's ctor) with `enemy_ai = kh1`, the new enemy's vtable becomes our copy of Re:CoM's (the
//!   RTTI locator kept at `−8`), whose state slots run its KH1 behaviour. A 0x20-byte tail after the
//!   body (the factory's size raised) marks it (magic, battle, our index). Field enemies and Re:CoM
//!   battles are never touched.
//! - **States** (`crate::kh1_enemy::state`, each class's attack states): Re:CoM's state machine
//!   calls them; each runs KH1's frame for this enemy: the behaviour (`kh1_core::enemy::Behaviour`)
//!   once a frame when no action plays, KH1's body physics (turn, walk, arc, air steering, push
//!   slide) written into Re:CoM's mover (`+0xF0`: yaw `+0x54`, velocity `+0x80`), KH1's motions
//!   played through Re:CoM's own motion play (vtable `+0x70` `0x14017D750`: the clip, its sounds
//!   (`.SES`) and its attack records (`.ASD`), AP from Re:CoM's `@ENP`).
//! - **Reactions.** Re:CoM's hit handler `0x14017F7E0` takes the hit, applies Re:CoM's damage and
//!   picks state 11 / 12 / 30 as always; our state 11 / 12 begin turns the hit into KH1's reaction
//!   (`kh1_core::enemy::react`), state 30 runs Re:CoM's defeat with the KH1 motion.
//! - **Guard.** `super::guard` hands a blocked enemy to [`guard_block`]: KH1's clash recoil.
//!
//! Per frame: one tail check per converted enemy's state call, the behaviour's frame, a few reads
//! and writes. The Shadow's runtime is M10's, unchanged (its log lines and trace token too).

use super::movement::{func, orig, rd, rd_vec, wr};
use super::{base, log};
use crate::kh1_enemy::{self as ce, state, tail, Appear, Manifest, CLASSES};
use crate::sites::data;
use kh1_core::enemy::{self as ke, Behaviour, Cmd, Hit, HitNews, Phase, World};
use kh1_core::Vec3;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

const N: usize = CLASSES.len();

#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicUsize = AtomicUsize::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const OFF: AtomicBool = AtomicBool::new(false);
#[allow(clippy::declare_interior_mutable_const)]
const NOLINE: AtomicU32 = AtomicU32::new(0);

pub static ORIG_CTOR: [AtomicUsize; N] = [ZERO; N];
static INSTALLED: [AtomicBool; N] = [OFF; N];
static MANIFEST: [OnceLock<Manifest>; N] = [const { OnceLock::new() }; N];
/// Our vtable copies (slot 0's address) and Re:CoM's slots.
static COPY: [AtomicUsize; N] = [ZERO; N];
static ORIGINAL: [OnceLock<Vec<usize>>; N] = [const { OnceLock::new() }; N];
static LINES: [AtomicU32; N] = [NOLINE; N];
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
    /// Re:CoM's max-HP getter (`0x14017F5A0`, checked in the stats lists).
    pub const MAX_HP: u32 = 0x0017_F5A0;
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
    pub const HP: usize = 0x274;
    pub const CHOSEN: usize = 0x278;
    /// Re:CoM's state 0 alternate path (`0x140192850` / `0x14017B590`: `+0x260 & 0x20000000008`).
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
type MaxHpFn = unsafe extern "win64" fn(usize) -> i32;

/// A running action (an attack, or a jump start, an appearance): the AI waits for it, unless it
/// is `steered` (KH1's plain request made with attack rows, `kh1_core::enemy::Cmd::Attack`: the AI
/// keeps running and steering the body while it plays; M11 fliers).
#[derive(Clone, Copy, Debug)]
struct Act {
    request: u16,
    then: u16,
    attack: Option<u8>,
    blend: f32,
    steered: bool,
}

struct Runtime {
    class: usize,
    enemy: usize,
    battle: u32,
    ai: Box<dyn Behaviour>,
    current: Option<u16>,
    queued: Option<(u16, f32)>,
    action: Option<Act>,
    /// The action motion has started (its clock now counts).
    action_playing: bool,
    reaction: Option<Phase>,
    /// A launch's lie request (`base + 3` / `+ 8`) while lying.
    lie: Option<u16>,
    clash: Option<u8>,
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
    /// The motion playing: its clock's wrap (a clip Re:CoM loops), its age.
    watch: ce::ClipWatch,
    /// KH1 frames since the action was requested (until its motion starts), then since its motion
    /// started: the safety net (`ce::watchdog`).
    action_age: f32,
    /// Re:CoM's state at the last KH1 frame (the log of a state change while unhittable).
    last_state: Option<usize>,
    /// A Guard blocked an attack whose recoil the model lacks: the next state 11 is Re:CoM's own
    /// flinch (owner 2026-11-02, "Soldier choices"), not KH1's reaction.
    recom_flinch: bool,
    /// Re:CoM's own flinch runs (state 11's Re:CoM begin / step); ends at the next state 1.
    recom_state: bool,
    /// We asked for the next state 1 (an attack's end, a reaction's end, the appearance's end):
    /// any other state 1 begin follows a state Re:CoM owns (stun, Stop, freeze, a Re:CoM flinch,
    /// the first-strike stun) and the KH1 frame is resumed cleanly (`take_over`).
    handing_back: bool,
    /// KH1 frames in state 1 with the AI polled and issuing nothing (`ce::watchdog::QUIET_LIMIT`).
    quiet: f32,
    /// KH1 frames the running reaction has taken (`ce::watchdog::REACTION_LIMIT`).
    reaction_age: f32,
    /// The battle floor's height (KH1 y) for a flying body's `sys 89` (M11 fliers): Sora's height
    /// the last time he stood on the floor (Assumption: Re:CoM's battle floors are flat).
    floor: Option<f32>,
    /// Re:CoM's own model scale, read before our first `Cmd::Scale` (KH1's scale is a factor on it).
    base_scale: Option<[f32; 4]>,
    /// KH1's hit counter (`actor+0x3E8`): one more per hit record (clashes too, `0x1402BFE60`),
    /// zeroed at the end of a KH1 frame with no action playing (`0x1402B7260`: actor flag 4
    /// clear: no action of ours waiting and no reaction). The behaviour hears it as the record's
    /// `+0x24` (`Behaviour::set_hit_count`; M11, the Bandit).
    hits: u16,
}

// SAFETY: a runtime (its behaviour holds `Rc`s that never leave it) is only touched under `RT`'s
// lock; the ctor may run on a loading thread and the states on the game thread.
unsafe impl Send for Runtime {}

/// Every KH1 enemy's runtime (this battle's; older ones are dropped at the next ctor).
static RT: Mutex<Vec<Runtime>> = Mutex::new(Vec::new());

fn line(k: usize, s: impl FnOnce() -> String) {
    if LINES[k].fetch_add(1, Ordering::Relaxed) < LINE_CAP {
        log(&format!("{}: {}", CLASSES[k].name, s()));
    }
}

pub fn installed(k: usize) -> bool {
    INSTALLED[k].load(Ordering::Relaxed)
}

fn any_installed() -> bool {
    (0..N).any(installed)
}

fn live_battle(k: usize) -> bool {
    installed(k) && super::battle::kh1_battle() && super::settings().enemy_ai == crate::settings::Mode::Kh1
}

/// `super::stats`: our vtable copies stand for their classes' vtables.
pub(super) fn canonical_vtable(vt: usize) -> usize {
    for (k, c) in CLASSES.iter().enumerate() {
        let copy = COPY[k].load(Ordering::Relaxed);
        if copy != 0 && vt == copy {
            return base() + c.vtable as usize;
        }
    }
    vt
}

/// Loads the class's manifest, checks its vtable and factory size, builds the vtable copy, patches
/// the size. The ctor hook is installed by the caller after this.
pub unsafe fn prepare(bs: usize, k: usize, manifest: Option<&std::path::Path>) -> bool {
    let c = &CLASSES[k];
    let Some(p) = manifest else {
        if k == 0 {
            log("shadow: no kh1-shadow.json (run kh1-recom-build): Shadows keep Re:CoM's behaviour");
        } else {
            log(&format!("{}: no {} (run kh1-recom-build): Re:CoM's behaviour kept", c.name, c.manifest));
        }
        return false;
    };
    let text = match std::fs::read_to_string(p) {
        Ok(t) => t,
        Err(e) => {
            log(&format!("{}: {}: {e}; Re:CoM's behaviour kept", c.name, p.display()));
            return false;
        }
    };
    let m = match ce::parse_for(k, &text) {
        Ok(m) => m,
        Err(e) => {
            log(&format!("{}: {}: {e}; Re:CoM's behaviour kept", c.name, p.display()));
            return false;
        }
    };
    let vt = bs + c.vtable as usize;
    if super::stats::rtti_name(bs, vt).as_deref() != Some(c.rtti) {
        log(&format!("{}: {}'s vtable is not at {:#x}: not installed", c.name, c.rtti, 0x1_4000_0000u64 + c.vtable as u64));
        return false;
    }
    if let Some(p) = c.ptrs.iter().find(|p| rd::<usize>(bs + p.rva as usize) != bs + p.target as usize) {
        log(&format!("{}: {} is not the function expected: not installed", c.name, p.name));
        return false;
    }
    // The copy: Re:CoM's locator, then its slots with ours in the state slots.
    let original: Vec<usize> = (0..c.slots).map(|i| rd::<usize>(vt + 8 * i)).collect();
    let mut copy: Vec<usize> = Vec::with_capacity(c.slots + 1);
    copy.push(rd::<usize>(vt - 8));
    copy.extend_from_slice(&original);
    let mut set = |off: usize, f: usize| copy[1 + off / 8] = f;
    set(ce::begin_slot(state::APPEAR), appear_begin as *const () as usize);
    set(ce::step_slot(state::APPEAR), appear_step as *const () as usize);
    set(ce::begin_slot(state::THINK), think_begin as *const () as usize);
    set(ce::step_slot(state::THINK), think_step as *const () as usize);
    for &s in c.attack_states {
        set(ce::begin_slot(s), attack_begin as *const () as usize);
        set(ce::step_slot(s), attack_step as *const () as usize);
    }
    for s in [state::FLINCH, state::BLOW] {
        set(ce::begin_slot(s), react_begin as *const () as usize);
        set(ce::step_slot(s), react_step as *const () as usize);
    }
    set(ce::begin_slot(state::DEFEAT), defeat_begin as *const () as usize);
    set(ce::step_slot(state::DEFEAT), defeat_step as *const () as usize);
    set(ce::THINK_SLOT, think_none as *const () as usize);
    let leaked: &'static mut [usize] = Box::leak(copy.into_boxed_slice());
    let at = leaked.as_ptr() as usize + 8;
    // The allocation size (all-or-nothing with the rest: a failed write leaves everything off).
    let a = c.alloc_site;
    if !super::hook::patch(bs + a.rva as usize, &ce::alloc_bytes(c.alloc), &ce::alloc_bytes(c.alloc + tail::SIZE), a.name) {
        log(&format!("{}: the factory's size held other bytes: not installed", c.name));
        return false;
    }
    let _ = ORIGINAL[k].set(original);
    let _ = MANIFEST[k].set(m);
    COPY[k].store(at, Ordering::Release);
    true
}

/// Puts the size back (the ctor hook failed).
pub unsafe fn unprepare(bs: usize, k: usize) {
    let c = &CLASSES[k];
    let a = c.alloc_site;
    super::hook::patch(bs + a.rva as usize, &ce::alloc_bytes(c.alloc + tail::SIZE), &ce::alloc_bytes(c.alloc), a.name);
    COPY[k].store(0, Ordering::Release);
}

pub fn mark_installed(k: usize) {
    INSTALLED[k].store(true, Ordering::Release);
}

pub fn start_line(k: usize) -> String {
    let m = MANIFEST[k].get();
    let c = &CLASSES[k];
    if k == 0 {
        return format!(
            "shadow: KH1's Shadow (M10) at combat = kh1 in Sora's story with enemy_ai = kh1 (read at each battle start): ctor 0x140192770 wrapped, vtable copy at {:#x} (states 0, 1, 2, 3, 4, 11, 12, 30, think 0x3D0), size 0xCE0 -> 0xD00 at 0x140188FAD; {} clips, recoil {:?}",
            COPY[k].load(Ordering::Relaxed),
            m.map_or(0, |m| m.clips.len()),
            m.map(|m| [m.recoil[0], m.recoil[1], m.recoil[2], m.recoil[2]])
        );
    }
    let states: Vec<String> = ce::replaced_states(c).iter().map(|s| s.to_string()).collect();
    format!(
        "{}: KH1's {} (M11) at combat = kh1 in Sora's story with enemy_ai = kh1 (read at each battle start): ctor {:#x} wrapped, vtable copy at {:#x} (states {}, think 0x3D0), size {:#X} -> {:#X} at {:#x}; {} clips, recoil {:?}",
        c.name,
        c.name,
        0x1_4000_0000u64 + c.ctor.rva as u64,
        COPY[k].load(Ordering::Relaxed),
        states.join(", "),
        c.alloc,
        c.alloc + tail::SIZE,
        0x1_4000_0000u64 + c.alloc_site.rva as u64,
        m.map_or(0, |m| m.clips.len()),
        m.map(|m| m.recoil.clone())
    )
}

// ---------------------------------------------------------------------------------------------
// The ctors and the runtime lookup.
// ---------------------------------------------------------------------------------------------

/// Class `K`'s ctor: Re:CoM's, then (KH1 battle, `enemy_ai = kh1`) our vtable and tail.
pub unsafe extern "win64" fn ctor<const K: usize>(this: usize, a: usize, b: usize, c: usize) -> usize {
    // `ctor_ptr` names a wrapper for 40 indices; only installed classes' are ever called.
    let (Some(o), Some(cl)) = (ORIG_CTOR.get(K), CLASSES.get(K)) else { return 0 };
    let r = orig::<CtorFn>(o)(this, a, b, c);
    if this == 0 || !live_battle(K) {
        return r;
    }
    let copy = COPY[K].load(Ordering::Acquire);
    let Some(m) = MANIFEST[K].get() else { return r };
    if copy == 0 || rd::<usize>(this) != base() + cl.vtable as usize {
        return r;
    }
    let battle = super::battle::battle_number();
    let seed = SEED.fetch_add(0x9E37_79B9, Ordering::Relaxed) ^ (this as u32);
    // Re:CoM's ctor doesn't make the mover (`+0xF0` is still 0 here: 0.67.0's crash at every battle
    // with a Shadow, owner 2026-11-02); the facing is taken again at state 0's begin.
    let m0: usize = rd(this + e::MOVER);
    let yaw = if m0 != 0 { crate::units::yaw_to_kh1(rd::<f32>(m0 + mv::YAW)) } else { 0.0 };
    let ai = ce::make(K, m, seed, yaw);
    let idx = (|| {
        let Ok(mut v) = RT.lock() else { return None };
        v.retain(|x| x.battle == battle && x.enemy != this);
        v.push(Runtime {
            class: K,
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
            watch: ce::ClipWatch::default(),
            action_age: 0.0,
            last_state: None,
            recom_flinch: false,
            recom_state: false,
            handing_back: false,
            quiet: 0.0,
            reaction_age: 0.0,
            floor: None,
            base_scale: None,
            hits: 0,
        });
        Some(v.len() - 1)
    })();
    let Some(idx) = idx else { return r };
    wr(this, copy);
    wr(this + cl.alloc as usize + tail::OFF_MAGIC, cl.magic);
    wr(this + cl.alloc as usize + tail::OFF_BATTLE, battle);
    wr(this + cl.alloc as usize + tail::OFF_INDEX, idx as u32);
    let name = cl.title;
    line(K, || format!("battle {battle}: {name} {this:#x} gets KH1's AI (seed {seed:#x})"));
    r
}

/// Class `k`'s ctor wrapper (`ctor::<K>` for every class index; room for 40 classes so a new class
/// needs no line here).
pub fn ctor_ptr(k: usize) -> usize {
    const _: () = assert!(N <= 40);
    match k {
        0 => ctor::<0> as *const () as usize,
        1 => ctor::<1> as *const () as usize,
        2 => ctor::<2> as *const () as usize,
        3 => ctor::<3> as *const () as usize,
        4 => ctor::<4> as *const () as usize,
        5 => ctor::<5> as *const () as usize,
        6 => ctor::<6> as *const () as usize,
        7 => ctor::<7> as *const () as usize,
        8 => ctor::<8> as *const () as usize,
        9 => ctor::<9> as *const () as usize,
        10 => ctor::<10> as *const () as usize,
        11 => ctor::<11> as *const () as usize,
        12 => ctor::<12> as *const () as usize,
        13 => ctor::<13> as *const () as usize,
        14 => ctor::<14> as *const () as usize,
        15 => ctor::<15> as *const () as usize,
        16 => ctor::<16> as *const () as usize,
        17 => ctor::<17> as *const () as usize,
        18 => ctor::<18> as *const () as usize,
        19 => ctor::<19> as *const () as usize,
        20 => ctor::<20> as *const () as usize,
        21 => ctor::<21> as *const () as usize,
        22 => ctor::<22> as *const () as usize,
        23 => ctor::<23> as *const () as usize,
        24 => ctor::<24> as *const () as usize,
        25 => ctor::<25> as *const () as usize,
        26 => ctor::<26> as *const () as usize,
        27 => ctor::<27> as *const () as usize,
        28 => ctor::<28> as *const () as usize,
        29 => ctor::<29> as *const () as usize,
        30 => ctor::<30> as *const () as usize,
        31 => ctor::<31> as *const () as usize,
        32 => ctor::<32> as *const () as usize,
        33 => ctor::<33> as *const () as usize,
        34 => ctor::<34> as *const () as usize,
        35 => ctor::<35> as *const () as usize,
        36 => ctor::<36> as *const () as usize,
        37 => ctor::<37> as *const () as usize,
        38 => ctor::<38> as *const () as usize,
        39 => ctor::<39> as *const () as usize,
        _ => 0,
    }
}

/// The class of an enemy that is one of ours (our vtable and tail).
pub(super) unsafe fn ours(en: usize) -> Option<usize> {
    let vt: usize = rd(en);
    (0..N).find(|&k| {
        let c = COPY[k].load(Ordering::Relaxed);
        c != 0 && vt == c && rd::<u32>(en + CLASSES[k].alloc as usize + tail::OFF_MAGIC) == CLASSES[k].magic
    })
}

/// Runs `f` on the enemy's runtime; None when it has none (then the caller runs Re:CoM's slot).
unsafe fn with_rt<T>(en: usize, f: impl FnOnce(&mut Runtime) -> T) -> Option<T> {
    let k = ours(en)?;
    let at = CLASSES[k].alloc as usize;
    let battle: u32 = rd(en + at + tail::OFF_BATTLE);
    let idx: u32 = rd(en + at + tail::OFF_INDEX);
    // `try_lock`: a state call re-entered from inside another (never expected) runs Re:CoM's slot.
    let mut v = RT.try_lock().ok()?;
    let r = v.get_mut(idx as usize).filter(|r| r.enemy == en && r.battle == battle)?;
    Some(f(r))
}

unsafe fn original(k: usize, off: usize) -> usize {
    ORIGINAL[k].get().map_or(0, |o| o[off / 8])
}

/// Calls Re:CoM's own slot of the enemy's class (`k`: its class; the Shadow's when unknown).
unsafe fn call_original(en: usize, off: usize) {
    let k = ours(en).unwrap_or(0);
    let f = original(k, off);
    if f != 0 {
        let g: StateFn = std::mem::transmute_copy(&f);
        g(en);
    }
}

// ---------------------------------------------------------------------------------------------
// The KH1 frame.
// ---------------------------------------------------------------------------------------------

fn manifest(k: usize) -> &'static Manifest {
    MANIFEST[k].get().expect("installed")
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
    let Some(c) = manifest(rt.class).clip(request) else {
        line(rt.class, || format!("{en:#x}: no clip for KH1 request {request}"));
        return;
    };
    let vt: usize = rd(en);
    let p: PlayFn = std::mem::transmute_copy(&rd::<usize>(vt + 0x70));
    p(en, c.slot, 0, c.root as u64, crate::shadow::blend_units(blend), 0, 0);
    if !rt.hittable && rt.current != Some(request) {
        // Diagnostic (the sunk Shadow that shows standing): every motion change while unhittable.
        let from = rt.current;
        line(rt.class, || format!("e{}: plays {request} (slot {}) after {from:?} while unhittable", rd::<u32>(en + e::ID), c.slot));
    }
    rt.current = Some(request);
    rt.watch.restart();
}

/// The current request's clip has played to KH1's length, or (a clip Re:CoM loops) its clock
/// wrapped (`ce::ClipWatch`).
unsafe fn ended(en: usize, rt: &mut Runtime) -> bool {
    match rt.current.and_then(|r| manifest(rt.class).clip(r)) {
        Some(c) => rt.watch.ended(clock(en), c.frames as f32),
        None => true,
    }
}

/// The safety net: the running action is stuck (`ce::watchdog`), with the loud log line's detail.
unsafe fn stuck(en: usize, rt: &Runtime) -> Option<(ce::watchdog::Stuck, String)> {
    let a = rt.action?;
    if a.steered && rt.action_playing {
        // The AI runs during a steered attack (M11 fliers): nothing waits on it.
        return None;
    }
    let frames = manifest(rt.class).clip(a.request).map_or(0.0, |c| c.frames as f32);
    let why = ce::watchdog::check(rt.action_playing, rt.action_age, frames)?;
    let st = crate::guard::state_index(en, rd(en + e::STATE)).map_or("?".to_string(), |s| s.to_string());
    let detail = match why {
        ce::watchdog::Stuck::NeverStarted => match a.attack {
            Some(i) => format!("Re:CoM state {st}, its attack state {} never began, {:.0} frames", CLASSES[rt.class].attack_states.get(i as usize).copied().unwrap_or(2), rt.action_age),
            None => format!("Re:CoM state {st}, {:.0} frames", rt.action_age),
        },
        ce::watchdog::Stuck::NeverEnded => format!(
            "Re:CoM state {st}, playing {:?}, clock {} of {} units, {:.0} of {frames:.0} frames",
            rt.current,
            clock(en),
            crate::units::frames_to_units(frames),
            rt.action_age
        ),
    };
    Some((why, detail))
}

/// Logs a released action (always, past the class's line cap).
unsafe fn release_line(en: usize, rt: &Runtime, why: ce::watchdog::Stuck, detail: &str) {
    let req = rt.action.map_or(0, |a| a.request);
    log(&ce::watchdog::line(CLASSES[rt.class].name, rd(en + e::ID), req, why, detail));
}

/// KH1's hittable flag (actor flag `0x40`, `kh1_core::enemy::Cmd::Hittable`) as Re:CoM's own
/// hurt-body switch: every hurt body off (`0x1401A2D70(enemy, −1)`) or on (`0x1401A2DB0`), what
/// Re:CoM's dig does (`0x140192E60` case 0 / 5). Re:CoM's hit handler, the KH1 swings' pair tests
/// and the spells' overlap tests only meet bodies that are on. The lock-on flag (`+0x154` bit
/// 0x10000, `0x1401CDB90`) stays: KH1's lock button (`0x1402B1540`) can lock it too.
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

/// KH1's scale (`+0x40..+0x48`, `kh1_core::enemy::Cmd::Scale`) as a factor on Re:CoM's own model
/// scale (read before the first write), through Re:CoM's scale getter / setter (`0x1401A4A20` /
/// `0x1401A7050`, as [`set_height`]). A factor under 0.001 is written as 0.001 (Assumption: a zero
/// scale would leave Re:CoM's model matrix singular; the model is as invisible at 0.001).
unsafe fn set_scale(en: usize, rt: &mut Runtime, k: f32) {
    let p = func::<ScaleFn>(f::SCALE)(en);
    if p == 0 {
        return;
    }
    let base = *rt.base_scale.get_or_insert_with(|| rd(p));
    let k = if k.is_finite() { k.max(0.001) } else { 1.0 };
    let s = [base[0] * k, base[1] * k, base[2] * k, base[3]];
    func::<ScaleSetFn>(f::SCALE_SET)(en, &s);
}

/// KH1's actor flag `0x1` (the model drawn, `kh1_core::enemy::Cmd::Visible`) as Re:CoM's own hide
/// bit `enemy+0x154` bit 0: the actor draw `0x1401A39B0` (every actor class's slot) draws nothing
/// while it is set, as the player's hide state sets it (`0x1401E5E80`) and `CEnemyShadow`'s state 0
/// clears it (`0x140192850`). Checked (decomp).
unsafe fn set_visible(en: usize, on: bool) {
    let f: u32 = rd(en + HIDE);
    wr(en + HIDE, if on { f & !1 } else { f | 1 });
}

/// Re:CoM's actor flags holding the hide bit (bit 0).
const HIDE: usize = 0x154;

/// Enters a Re:CoM attack state as P1's cardless play does (`crate::battle::cardless_flags`).
unsafe fn enter_state(en: usize, k: i32) {
    let flags: u64 = rd(en + e::FLAGS);
    wr(en + e::FLAGS, crate::battle::cardless_flags(flags));
    wr(en + e::CHOSEN, k);
    wr(en + e::NEXT, crate::battle::state_address(en, k));
}

unsafe fn apply(en: usize, rt: &mut Runtime, c: &Cmd<u8>) {
    match *c {
        Cmd::Motion { request, blend } => {
            if rt.current != Some(request) {
                rt.queued = None;
                play(en, rt, request, blend);
            }
        }
        Cmd::Queue { request, blend } => rt.queued = Some((request, blend)),
        Cmd::Action { request, attack, then, blend } => {
            rt.action = Some(Act { request, then, attack, blend, steered: false });
            rt.action_playing = false;
            rt.action_age = 0.0;
            match attack {
                // An attack: Re:CoM's attack state; its begin plays the motion.
                Some(a) => enter_state(en, CLASSES[rt.class].attack_states.get(a as usize).copied().unwrap_or(2)),
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
            // KH1's alpha fade-in (`0x2570` / `0x2010`): no Re:CoM model alpha found to write yet
            // (`docs/recom-m10-shadow.md` § Not built).
            let _ = n;
        }
        // KH1's flag 0x1000: the action's root motion carries the body (`kh1_core::enemy::Body::held`).
        Cmd::Held => rt.ai.body().held = true,
        // A plain request with attack rows (M11 fliers): Re:CoM's attack state, the AI keeps
        // running; its begin plays the motion (no queue: KH1's request replaces the queue).
        Cmd::Attack { request, attack, blend } => {
            rt.action = Some(Act { request, then: ke::react::req::IDLE, attack: Some(attack), blend, steered: true });
            rt.action_playing = false;
            rt.action_age = 0.0;
            enter_state(en, CLASSES[rt.class].attack_states.get(attack as usize).copied().unwrap_or(2));
        }
        Cmd::Scale(k) => set_scale(en, rt, k),
        Cmd::Visible(v) => set_visible(en, v),
        Cmd::Yaw(_) | Cmd::Jump { .. } | Cmd::Target(_) | Cmd::State(_) => {}
    }
    if ce::logged(c) {
        let id: u32 = rd(en + e::ID);
        let k = rt.class;
        let tok = ce::cmd_token(c, CLASSES[k].attack_names, &|s| rt.ai.state_name(s));
        line(k, || format!("e{id}: {tok}"));
    }
}

/// The action that played ended: the AI goes on; KH1's flag 0x1000 goes with its motion
/// (Assumption: cleared when the next motion, requested without bit 23, starts).
fn end_action(rt: &mut Runtime) {
    rt.action = None;
    rt.action_playing = false;
    rt.ai.body().held = false;
}

/// One KH1 frame of this enemy: landing, the motion queue, the action's end, the behaviour (when
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
    let flying = rt.ai.body().flying;
    if flying {
        floor_note(rt);
    }
    rt.watch.tick(step);
    // Diagnostic (owner 2026-11-02: a sunk Shadow sometimes shows standing mid-travel): a Re:CoM
    // state change while it cannot be hit (sinking, underground, rising) is logged with the KH1
    // motion playing.
    let st = crate::guard::state_index(en, rd(en + e::STATE));
    if st != rt.last_state {
        if !rt.hittable && rt.last_state.is_some() {
            let (from, cur) = (rt.last_state, rt.current);
            line(rt.class, || format!("e{}: Re:CoM state {from:?} -> {st:?} while unhittable (KH1 motion {cur:?})", rd::<u32>(en + e::ID)));
        }
        rt.last_state = st;
    }
    if let Some(a) = rt.action.filter(|a| a.attack.is_some() && !rt.action_playing) {
        // An attack whose Re:CoM attack state has not begun: asked again each frame while Re:CoM's
        // next state is another (something overwrote it, e.g. a default state in the same frame).
        let want = CLASSES[rt.class].attack_states.get(a.attack.unwrap_or(0) as usize).copied().unwrap_or(2);
        let is = |v: usize| crate::guard::state_index(en, v).map(|k| k as i32);
        if is(rd(en + e::NEXT)) != Some(want) && is(rd(en + e::STATE)) != Some(want) && rt.action_age > 0.0 {
            line(rt.class, || format!("e{}: attack state {want} asked again (Re:CoM's next state was {:?})", rd::<u32>(en + e::ID), is(rd(en + e::NEXT))));
            enter_state(en, want);
        }
    }
    if poll && rt.action.is_some_and(|a| a.attack.is_some()) && rt.action_playing {
        // An attack plays only in its Re:CoM attack state: here (state 0 / 1) its state was left
        // without its end (a Re:CoM state took over and handed back): released.
        let detail = format!("its Re:CoM attack state was left without its end; now Re:CoM state {:?}", st);
        release_line(en, rt, ce::watchdog::Stuck::NeverEnded, &detail);
        end_action(rt);
        rt.queued = None;
        play(en, rt, ke::react::req::IDLE, manifest(rt.class).consts.get("attack_blend"));
    }
    if rt.action.is_some() {
        rt.action_age += step;
        // The safety net (an attack that plays is `attack_step`'s: it leaves the attack state).
        if let Some((why, detail)) = stuck(en, rt) {
            if why == ce::watchdog::Stuck::NeverStarted || rt.action.is_some_and(|a| a.attack.is_none()) {
                release_line(en, rt, why, &detail);
                // As if its motion had finished: the queued motion plays, the AI goes on (a steered
                // attack queues nothing: the AI plays its next motion itself).
                let then = rt.action.filter(|a| !a.steered).map(|a| (a.then, a.blend));
                end_action(rt);
                if let Some((q, b)) = then {
                    rt.queued = None;
                    play(en, rt, q, b);
                }
            }
        }
    }
    if contact && rt.ai.body().falling() {
        rt.ai.body().land();
        landed(en, rt);
    }
    // The motion that ended: the queued one plays; an action ends.
    if ended(en, rt) {
        if let Some(a) = rt.action.filter(|_| rt.action_playing) {
            if rt.current == Some(a.request) && a.attack.is_none() {
                end_action(rt);
            }
        }
        if let Some((q, b)) = rt.queued.take() {
            play(en, rt, q, b);
        }
    }
    let airborne = rt.ai.body().airborne();
    if poll && rt.action.is_none_or(|a| a.steered) {
        // KH1's movement state 4 (flight) is neither state 0 nor state 2.
        let grounded = contact && !airborne && !flying;
        if let Some(f) = rt.floor {
            rt.ai.set_floor(f);
        }
        rt.ai.set_targeted(sora_targets(en));
        rt.ai.set_target_grounded(sora_grounded());
        let w = World { step, pos, target: sora_pos(), grounded, airborne, moved, motion: rt.current };
        let cmds = rt.ai.frame(w);
        for c in &cmds {
            apply(en, rt, c);
        }
        // A flier's script that steers its body this frame (`+0x20` × `+0x2C`) is acting too (the
        // Darkball's invisible glide issues no command for its whole length; M11).
        let steering = rt.ai.body().walk != Vec3::ZERO;
        // The safety net for a silent AI (no command at all for `QUIET_LIMIT` frames, no action and
        // no reaction pending): logged past the cap, the behaviour restarts from its engaged state.
        if !cmds.is_empty() || steering || rt.reaction.is_some() || st != Some(state::THINK as usize) {
            rt.quiet = 0.0;
        } else {
            rt.quiet += step;
            if rt.quiet >= ce::watchdog::QUIET_LIMIT {
                rt.quiet = 0.0;
                log(&format!("{}: e{}: the AI issued nothing for {:.0} frames in Re:CoM state {st:?} (playing {:?}), restarted", CLASSES[rt.class].name, rd::<u32>(en + e::ID), ce::watchdog::QUIET_LIMIT, rt.current));
                rt.ai.reset();
            }
        }
        let draws = rt.ai.take_draws();
        if !draws.is_empty() {
            let id: u32 = rd(en + e::ID);
            line(rt.class, || format!("e{id}: rand {}", draws.iter().map(|d| format!("{}={:.4}", d.what, d.value)).collect::<Vec<_>>().join(", ")));
        }
    }
    body_step(en, rt, step, pos);
    // KH1's hit counter goes back to 0 at the end of an update with no action playing (actor flag
    // 4: an action of ours the AI waits on, or a reaction).
    if rt.reaction.is_none() && rt.action.is_none_or(|a| a.steered) {
        rt.hits = 0;
    }
}

/// KH1's body physics for the frame, written into Re:CoM's mover: the facing, and the velocity
/// that moves it by KH1's displacement (horizontal always; vertical while in a KH1 arc, with
/// Re:CoM's acceleration cancelled). While KH1's flag 0x1000 holds the body, Re:CoM's root motion
/// moves it (the clip's root flag) and our velocity is only what cancels the mover's acceleration.
unsafe fn body_step(en: usize, rt: &mut Runtime, step: f32, pos: Vec3) {
    let mvr = mover(en);
    let c = &manifest(rt.class).consts;
    let steer = rt.ai.take_steer();
    let airborne = rt.ai.body().airborne();
    let flying = rt.ai.body().flying;
    let d = rt.ai.body().step(step, pos.y, steer, c);
    let yaw = rt.ai.body().yaw;
    wr(mvr + mv::YAW, crate::units::yaw_to_recom(yaw));
    let dr = crate::units::to_recom(d);
    let accel = rd_vec(mvr + mv::ACCEL);
    let v = crate::shadow::velocity(dr, accel, step);
    let scale: f32 = rd(mvr + mv::SPEED_SCALE);
    let k = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let mut vel: [f32; 4] = rd(mvr + mv::VEL);
    vel[0] = v.x * k;
    vel[2] = v.z * k;
    if airborne {
        vel[1] = v.y;
    } else if flying {
        // KH1's flight moves the body vertically too (`0x1402CC000`), at the same speed scale.
        vel[1] = v.y * k;
    }
    wr(mvr + mv::VEL, vel);
}

/// Whether Sora's target (Re:CoM's attack target `player+0x84C`, a handle; KH1's party 0 `+0x74`)
/// is this enemy.
unsafe fn sora_targets(en: usize) -> bool {
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    if pl == 0 {
        return false;
    }
    let h: i32 = rd(pl + 0x84C);
    h > 0 && func::<HandleFn>(f::HANDLE_OBJ)(h as u32) == en
}

/// Whether Sora stands on the floor (his mover's contact `+0x120 & 5`, as [`floor_note`] reads it):
/// KH1's target movement state `*(target + 0x70)` = 0 (M11, the Air Pirate). No Sora: on the floor.
unsafe fn sora_grounded() -> bool {
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    if pl == 0 {
        return true;
    }
    let m: usize = rd(pl + e::MOVER);
    m == 0 || rd::<u32>(m + mv::CONTACT) & 5 != 0
}

/// The battle floor for a flying body (`sys 89`): Sora's height while he stands on the floor
/// (his mover's contact `+0x120 & 5`), kept from the last time (Assumption: flat battle floors;
/// Re:CoM's fliers keep no floor height: their gravity is off, `+0x260 & 0x100`).
unsafe fn floor_note(rt: &mut Runtime) {
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    if pl == 0 {
        return;
    }
    let m: usize = rd(pl + e::MOVER);
    if m != 0 && rd::<u32>(m + mv::CONTACT) & 5 != 0 {
        let y = crate::units::to_kh1(rd_vec(m + mv::POS)).y;
        if y.is_finite() {
            rt.floor = Some(y);
        }
    }
}

/// The body touched the floor at the end of a KH1 arc (`0x1402B7260`'s landing).
unsafe fn landed(en: usize, rt: &mut Runtime) {
    match rt.reaction {
        Some(Phase::Air { base, behind }) => {
            let (r, then, lying) = ke::landing(base, behind);
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
        // Movement state 3 (a timed jump, the Powerwild's hops) lands straight to state 0
        // (`0x1402AFAD0`): no landing motion; the queue the jump set plays on.
        _ if rt.ai.body().landed_timed => {}
        _ => {
            // A plain landing (a leap, a held attack's end): KH1's request 7 then the queue
            // (`0x1402B7260`).
            if rt.action.is_none() && rt.reaction.is_none() {
                let q = rt.queued.take();
                play(en, rt, ke::react::req::LAND, 0.0);
                rt.queued = q.or(Some((ke::react::req::IDLE, 0.0)));
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The states.
// ---------------------------------------------------------------------------------------------

/// State 0 begin: Re:CoM's alternate appearance runs as Re:CoM's; else the class's appearance:
/// the Shadow's begin without its motion (bodies off until the rise) and KH1's rise; a base class's
/// Re:CoM begin as it is, then KH1's appearance.
pub unsafe extern "win64" fn appear_begin(en: usize) {
    let alt = rd::<u64>(en + e::FLAGS) & e::APPEAR_ALT != 0;
    let world = super::stats::recom_world();
    let done = with_rt(en, |rt| {
        rt.recom_appear = alt;
        rt.appear_sub = 0;
        // The KH1 world its script's world rules read (decision 21's mapping).
        let w = crate::kh1_enemy::kh1_world(world);
        rt.ai.set_world(w);
        let k = rt.class;
        line(k, || format!("e{}: KH1 world {w:?} (Re:CoM world {world})", rd::<u32>(en + e::ID)));
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
        CLASSES[rt.class].appear
    });
    match done {
        None => return call_original(en, ce::begin_slot(state::APPEAR)),
        Some(_) if alt => return call_original(en, ce::begin_slot(state::APPEAR)),
        Some(Appear::Base) => {
            // `0x14017B590` as it is (its writes, the battle message 0x96, its appear motion if the
            // class has one); KH1's appearance replaces the motion from the next frame.
            call_original(en, ce::begin_slot(state::APPEAR));
            let k = ours(en).unwrap_or(1);
            line(k, || format!("e{}: appears (KH1's appearance)", rd::<u32>(en + e::ID)));
        }
        Some(Appear::Shadow) => {
            // `0x140192850`'s writes but its motion: `+0x154 &= ~1`, both body sets off.
            let f154: u32 = rd(en + 0x154);
            wr(en + 0x154, f154 & !1);
            bodies(en, false);
            line(0, || format!("e{}: appears (KH1's rise)", rd::<u32>(en + e::ID)));
        }
    }
}

/// State 0 step: KH1's appearance; when it is done, the Shadow's battle-start handshake
/// (`0x140192B00`) or, for a base class, Re:CoM's default state.
pub unsafe extern "win64" fn appear_step(en: usize) {
    let r = with_rt(en, |rt| {
        if rt.recom_appear {
            return None;
        }
        let kind = CLASSES[rt.class].appear;
        match rt.appear_sub {
            0 => {
                tick(en, rt, true);
                if !rt.ai.risen() {
                    return Some(false);
                }
                // An attack the behaviour asked for in this same frame (its first after the rise)
                // already set Re:CoM's next state: the default state would overwrite it and the
                // attack state would never begin (owner 2026-11-02: a Shadow froze after its first
                // claw, log `e213`). It leaves state 0 for the attack state instead.
                let pending = rt.action.is_some_and(|a| a.attack.is_some()) && !rt.action_playing;
                if kind == Appear::Shadow && func::<IntFn>(f::INTRO_ROOM)() != 0 && func::<IntFn>(f::INTRO_FLAG)() != 0 {
                    func::<IntFn>(f::INTRO_COUNT)();
                    rt.appear_sub = 1;
                } else if !pending {
                    rt.handing_back = true;
                    return Some(true);
                }
            }
            _ => {
                if func::<IntFn>(f::INTRO_FLAG)() == 0 {
                    rt.handing_back = true;
                    return Some(true);
                }
            }
        }
        Some(false)
    });
    match r.flatten() {
        // Re:CoM's default state outside our lock (its state 1 begin is ours: `think_begin`).
        Some(true) => func::<StateFn>(f::DEFAULT_STATE)(en),
        Some(false) => {}
        None => call_original(en, ce::step_slot(state::APPEAR)),
    }
}

/// State 1 begin: no Re:CoM idle / walk pick; KH1's motion carries on.
pub unsafe extern "win64" fn think_begin(en: usize) {
    if with_rt(en, |rt| {
        if !std::mem::take(&mut rt.handing_back) {
            take_over(en, rt);
        }
        rt.recom_state = false;
        rt.quiet = 0.0;
        if rt.current.is_none() {
            play(en, rt, ke::react::req::IDLE, 8.0);
        }
    })
    .is_none()
    {
        call_original(en, ce::begin_slot(state::THINK));
    }
}

/// A state Re:CoM owns ran (stun, Stop, freeze, Gravity's flatten, a Re:CoM flinch, the
/// first-strike stun) and handed back to state 1 (owner 2026-11-02, 0.68.2: "once they got out of
/// the stun they are doing the same issue of just standing still": the kick the stun cut was still
/// the running action, so the AI was never polled again). As in KH1, where a status ends the
/// current action: the action, the reaction, the queued motion and the clash are dropped, the
/// motion Re:CoM played is forgotten (the idle plays), and the AI goes on from its wait.
unsafe fn take_over(en: usize, rt: &mut Runtime) {
    let had = (rt.action.map(|a| a.request), rt.reaction.is_some(), rt.queued.map(|q| q.0));
    if had != (None, false, None) {
        let st = crate::guard::state_index(en, rd(en + e::STATE));
        log(&format!(
            "{}: e{}: back in state 1 after a Re:CoM state ({st:?}): action {:?}, reaction {}, queued {:?} dropped; the AI goes on",
            CLASSES[rt.class].name,
            rd::<u32>(en + e::ID),
            had.0,
            had.1,
            had.2
        ));
    }
    end_action(rt);
    rt.reaction = None;
    rt.lie = None;
    rt.clash = None;
    rt.queued = None;
    rt.recom_flinch = false;
    rt.current = None;
}

/// State 1 step: KH1's frame with the behaviour.
pub unsafe extern "win64" fn think_step(en: usize) {
    if with_rt(en, |rt| tick(en, rt, true)).is_none() {
        call_original(en, ce::step_slot(state::THINK));
    }
}

/// The think (`+0x3D0`): nothing; KH1's behaviour picks the attacks.
pub unsafe extern "win64" fn think_none(_en: usize) {}

/// An attack state's begin: the attack's KH1 motion (its records attack through the `.ASD`).
pub unsafe extern "win64" fn attack_begin(en: usize) {
    let k = crate::guard::state_index(en, rd(en + e::NEXT)).or_else(|| crate::guard::state_index(en, rd(en + e::STATE)));
    let done = with_rt(en, |rt| {
        let Some(a) = rt.action.filter(|a| a.attack.is_some()) else { return false };
        play(en, rt, a.request, a.blend);
        // An action queues its `then`; KH1's plain request (a steered attack) replaces the queue.
        rt.queued = (!a.steered).then_some((a.then, a.blend));
        rt.action_playing = true;
        rt.action_age = 0.0;
        true
    });
    if done != Some(true) {
        // Not started by our behaviour (never expected): Re:CoM's begin.
        let s = k.map_or(2, |k| k as i32);
        call_original(en, ce::begin_slot(s));
    }
}

/// An attack state's step: KH1's frame without the behaviour; at the motion's end, Re:CoM's attack
/// end (`0x14017E540`: bit 0 off, the default state). A steered attack (M11 fliers) runs the
/// behaviour and ends when the behaviour plays another motion (KH1's rows end with their request,
/// `0x1402B47A0`) or starts something else, or when a one-shot clip has played; it doesn't end at a
/// looping clip's wrap (the script decides how long it loops) and the watchdog's end margin doesn't
/// apply to it (the AI is not waiting on it).
pub unsafe extern "win64" fn attack_step(en: usize) {
    let r = with_rt(en, |rt| {
        let steered = rt.action.is_some_and(|a| a.steered && rt.action_playing);
        let mut end = false;
        if steered {
            let looped = rt.current.and_then(|q| manifest(rt.class).clip(q)).is_some_and(|c| c.looped);
            if !looped && ended(en, rt) {
                end = true;
            }
        } else {
            end = rt.action.is_some() && ended(en, rt);
            if !end {
                // The safety net: an attack that never ends, or none at all in an attack state (its
                // begin was Re:CoM's): leave the attack state as at the motion's end.
                match stuck(en, rt) {
                    Some((why, detail)) => {
                        release_line(en, rt, why, &detail);
                        end = true;
                    }
                    None if rt.action.is_none() => {
                        let st = crate::guard::state_index(en, rd(en + e::STATE)).map_or("?".to_string(), |s| s.to_string());
                        log(&ce::watchdog::line(CLASSES[rt.class].name, rd(en + e::ID), 0, ce::watchdog::Stuck::NeverStarted, &format!("Re:CoM attack state {st} without a KH1 action")));
                        end = true;
                    }
                    None => {}
                }
            }
        }
        if end {
            end_action(rt);
            rt.handing_back = true;
        }
        let poll = steered && !end;
        let req = rt.action.map(|a| a.request);
        tick(en, rt, poll);
        if poll {
            match rt.action {
                // A new attack the behaviour started: `tick` asks for its state again.
                Some(a) if a.attack.is_some() && !rt.action_playing => end = true,
                Some(a) if a.steered && Some(a.request) == req && rt.current == req => {}
                // A plain action (a jump start…): it carries on in state 1.
                Some(a) if a.attack.is_none() => end = true,
                // Another motion replaced it.
                Some(_) => {
                    end_action(rt);
                    end = true;
                }
                None => end = true,
            }
            // Our own leave to state 1 (merge of Stage 127's hand-back with the fliers' steered
            // attacks): not a Re:CoM takeover, so the pending action / queue are kept.
            if end {
                rt.handing_back = true;
            }
        }
        let again = rt.action.filter(|a| a.attack.is_some() && !rt.action_playing).and_then(|a| CLASSES[rt.class].attack_states.get(a.attack.unwrap_or(0) as usize).copied());
        (end, again)
    });
    match r {
        Some((true, again)) => {
            func::<EndFn>(f::ATTACK_END)(en, 0, 0);
            if let Some(k) = again {
                enter_state(en, k);
            }
        }
        Some((false, _)) => {}
        None => {
            let s = crate::guard::state_index(en, rd(en + e::STATE)).map_or(2, |k| k as i32);
            call_original(en, ce::step_slot(s));
        }
    }
}

/// The hit Re:CoM's handler took (its hit info): KH1's kind (a KH1 swing's from its row, else from
/// Re:CoM's kind), the push direction, the hurt part.
unsafe fn hit_of(en: usize, k: usize) -> Option<Hit> {
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
    let kind = swing.or_else(|| (atk != 0).then(|| crate::shadow::kh1_kind_of_recom(rd(atk + hi::KIND))))?;
    // The attacker: its handle's actor (`0x14017C530`'s rule), else Sora.
    let owner = if atk != 0 { func::<HandleFn>(f::HANDLE_OBJ)(rd(atk + hi::OWNER)) } else { 0 };
    let pl: usize = rd(super::va(base(), data::MAIN_PLAYER));
    let dir = if owner != 0 && owner == pl {
        let pm: usize = rd(pl + e::MOVER);
        let sp = crate::units::to_kh1(rd_vec(pm + mv::POS));
        let sy = crate::units::yaw_to_kh1(rd(pm + mv::YAW));
        crate::shadow::swing_dir(sp, sy, me, std::f32::consts::FRAC_PI_4)
    } else {
        let from = if owner != 0 { crate::units::to_kh1(rd_vec(func::<BoneFn>(f::BONE)(owner, 0) + 0xD0)) } else { sora_pos().unwrap_or(me) };
        let d = (me - from).flat();
        if d.horizontal_length() > 1e-3 {
            d.normalized()
        } else {
            Vec3::from_yaw(with_rt(en, |rt| rt.ai.body().yaw).unwrap_or(0.0) + std::f32::consts::PI)
        }
    };
    // The part: the contact point against the hurt parts' bones.
    let n: i32 = rd(info + hi::COUNT);
    let parts: usize = rd(info + hi::PARTS);
    let bones = &manifest(k).parts;
    let part = if n > 0 && parts != 0 && !bones.is_empty() {
        let contact = rd_vec(parts + hi::CONTACT);
        let at: Vec<Vec3> = bones.iter().map(|&b| rd_vec(func::<BoneFn>(f::BONE)(en, b) + 0xD0)).collect();
        ke::react::nearest_part(contact, &at)
    } else {
        0
    };
    Some(Hit { kind, dir, part })
}

/// KH1 swings noted by the enemy-hit wrapper (`note_swing`), by enemy.
static SWING: std::sync::Mutex<Vec<(usize, u8)>> = std::sync::Mutex::new(Vec::new());

/// `super::attack`: a KH1 swing hit this enemy with KH1's reaction kind `kind`.
pub(super) unsafe fn note_swing(en: usize, kind: u8) {
    if !any_installed() || ours(en).is_none() {
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

/// Starts KH1's reaction for the hit (or the clash) on this enemy; false: nothing to play.
unsafe fn start_reaction(en: usize, rt: &mut Runtime, hit: Option<Hit>) -> bool {
    let m = manifest(rt.class);
    let c = &m.consts;
    let y = crate::units::to_kh1(rd_vec(mover(en) + mv::POS)).y;
    let airborne = rt.ai.body().airborne();
    end_action(rt);
    if let Some(a) = rt.clash.take() {
        let names = CLASSES[rt.class].attack_names;
        let aname = names.get(a as usize).copied().unwrap_or("?");
        let Some(recoil) = m.recoil_of(a) else {
            // Not expected (`guard_block` hands such a block to Re:CoM's flinch): the idle.
            rt.queued = None;
            play(en, rt, ke::react::req::IDLE, c.get("attack_blend"));
            rt.reaction = Some(Phase::Motion);
            line(rt.class, || format!("e{}: clash on its {aname}: no recoil motion ({:?}); idle", rd::<u32>(en + e::ID), m.recoil.get(a as usize)));
            return true;
        };
        let s = ke::react::clash(recoil, airborne, &mut rt.ai.body(), y, c);
        rt.queued = None;
        play(en, rt, s.request, 0.0);
        rt.queued = Some((s.then, c.get("attack_blend")));
        rt.reaction = Some(if airborne { Phase::Air { base: None, behind: false } } else { Phase::Motion });
        line(rt.class, || format!("e{}: clash recoil {} ({aname})", rd::<u32>(en + e::ID), s.request));
        return true;
    }
    let Some(hit) = hit else { return false };
    let lying = matches!(rt.reaction, Some(Phase::Down { .. })) && rt.current == rt.lie;
    let current = rt.current;
    // The behaviour's random stream (KH1's one `rand`; the launch's down time draws from it).
    let mut rng = rt.ai.rng();
    let yaw = rt.ai.body().yaw;
    let mut start = {
        let mut body = rt.ai.body();
        if body.flying {
            // Movement state 4 (M11 fliers): KH1's ground forms, kind 5 as kind 4.
            ke::react_flying(hit, yaw, current, m, c, &mut rng, &mut body, y)
        } else {
            ke::react(hit, yaw, current, airborne, lying, m, c, &mut rng, &mut body, y)
        }
    };
    // A launch the script's hit handler asked for (`sys 38`, the Powerwild) replaces the reaction.
    // KH1 runs the handler first (`0x1402B7B90` → `0x1402BCD60`, then `0x1402B8E10`'s switch) and
    // `0x1402B79F0` marks the record (`*hit |= 2`) so KH1's own reaction is skipped, its random
    // draw too (Checked (decomp), M11 Bandit pass); here KH1's reaction is chosen first and
    // replaced, so its draw stays drawn (a known difference, `docs/recom-m11-enemies.md`).
    if let Some(l) = rt.ai.scripted_launch() {
        let mut body = rt.ai.body();
        if let Some(s) = ke::react::scripted_launch(hit, l, airborne, yaw, m, c, &mut body, y) {
            start = Some(s);
        }
    }
    rt.ai.set_rng(rng);
    let id: u32 = rd(en + e::ID);
    match start {
        None => {
            line(rt.class, || format!("e{id}: hit kind {} part {} while lying: push only", hit.kind, hit.part));
            true
        }
        Some(s) => {
            rt.down = s.down;
            rt.queued = None;
            play(en, rt, s.request, 0.0);
            rt.queued = Some((s.then, c.get("attack_blend")));
            let behind = s.base.is_some_and(|b| s.request >= b + 5);
            rt.reaction = Some(if rt.ai.body().airborne() { Phase::Air { base: s.base, behind } } else { Phase::Motion });
            line(rt.class, || format!("e{id}: reaction kind {} part {} dir ({:.2},{:.2}) -> {} then {}{}", hit.kind, hit.part, hit.dir.x, hit.dir.z, s.request, s.then, if s.base.is_some() { format!(" down {:.1}", s.down) } else { String::new() }));
            true
        }
    }
}

/// Re:CoM's HP / max HP (`+0x274`, `0x14017F5A0`), for scripts that read their HP.
unsafe fn hp_ratio(en: usize) -> f32 {
    let hp: i32 = rd(en + e::HP);
    let max = func::<MaxHpFn>(f::MAX_HP)(en);
    if max > 0 {
        hp as f32 / max as f32
    } else {
        1.0
    }
}

/// States 11 / 12 begin: KH1's reaction.
pub unsafe extern "win64" fn react_begin(en: usize) {
    // Re:CoM's own flinch for a blocked attack without a KH1 recoil motion (owner 2026-11-02): as
    // before M10, Re:CoM's state 11 begin and step (`0x14017C530` / `0x140184100`); the KH1 frame
    // waits (no velocity of ours, no KH1 motion) until the next state 1.
    let recom = with_rt(en, |rt| {
        rt.recom_state = false;
        if !std::mem::take(&mut rt.recom_flinch) {
            return false;
        }
        end_action(rt);
        rt.clash = None;
        rt.queued = None;
        rt.current = None;
        rt.reaction = None;
        rt.recom_state = true;
        rt.hits = rt.hits.saturating_add(1);
        rt.ai.set_hit_count(rt.hits);
        rt.ai.on_hit(HitNews::Clash);
        let k = rt.class;
        line(k, || format!("e{}: clash without a KH1 recoil motion: Re:CoM's flinch", rd::<u32>(en + e::ID)));
        true
    });
    if recom == Some(true) {
        return call_original(en, ce::begin_slot(state::FLINCH));
    }
    let hit = ours(en).and_then(|k| hit_of(en, k));
    let done = with_rt(en, |rt| {
        // KH1's hit handler: a damaging hit or a clash (the Shadow ignores a clash).
        if CLASSES[rt.class].reads_hp {
            rt.ai.set_hp_ratio(hp_ratio(en));
        }
        if let Some(h) = hit {
            rt.ai.set_hit_kind(h.kind);
        }
        rt.hits = rt.hits.saturating_add(1);
        rt.ai.set_hit_count(rt.hits);
        let clash = rt.clash.is_some();
        rt.ai.on_hit(if clash { HitNews::Clash } else { HitNews::Damage });
        // The handler ran before the reaction and may have rewritten the record's kind.
        let hit = if clash { hit } else { hit.map(|h| Hit { kind: rt.ai.reaction_kind(h.kind), ..h }) };
        if rt.clash.is_none() && !rt.ai.keeps_reaction() {
            // KH1's hit handler zeroed the reaction kind (M11, the Darkball's armour): no reaction.
            // A reaction already running carries on in this state; otherwise back to state 1 (Re:CoM's
            // handler has already cut the attack it hit: the AI goes on).
            let id: u32 = rd(en + e::ID);
            let k = rt.class;
            let running = rt.reaction.is_some();
            line(k, || format!("e{id}: hit without a reaction (KH1's hit handler){}", if running { ": the reaction carries on" } else { "" }));
            if !running {
                rt.handing_back = true;
            }
            return running;
        }
        rt.reaction_age = 0.0;
        let started = start_reaction(en, rt, hit);
        if !started {
            rt.handing_back = true;
        }
        started
    });
    match done {
        Some(true) => {}
        Some(false) => func::<StateFn>(f::DEFAULT_STATE)(en),
        None => {
            let s = crate::guard::state_index(en, rd(en + e::NEXT)).map_or(state::FLINCH, |k| k as i32);
            call_original(en, ce::begin_slot(s));
        }
    }
}

/// States 11 / 12 step: KH1's frame without the behaviour; the reaction's phases.
pub unsafe extern "win64" fn react_step(en: usize) {
    if with_rt(en, |rt| rt.recom_state) == Some(true) {
        return call_original(en, ce::step_slot(state::FLINCH));
    }
    let r = with_rt(en, |rt| {
        let dt: i32 = rd(en + e::DT);
        let step = crate::units::step_from_dt(dt.clamp(0, 1000));
        let mut done = false;
        rt.reaction_age += step;
        if rt.reaction_age >= ce::watchdog::REACTION_LIMIT {
            // The safety net: a reaction that never ends (a launch that never lands, a motion that
            // never ends) is released as if it had ended.
            log(&format!("{}: e{}: reaction {:?} never ended ({:.0} frames, playing {:?}), released", CLASSES[rt.class].name, rd::<u32>(en + e::ID), rt.reaction, rt.reaction_age, rt.current));
            rt.reaction = None;
            rt.lie = None;
            rt.queued = None;
            play(en, rt, ke::react::req::IDLE, manifest(rt.class).consts.get("attack_blend"));
            rt.handing_back = true;
            return true;
        }
        let motion_ended = matches!(rt.reaction, Some(Phase::Motion)) && ended(en, rt);
        match rt.reaction {
            // The reaction's motion ended (this frame: `tick` plays the queued one): done.
            Some(Phase::Motion) => {
                if motion_ended {
                    done = true;
                }
            }
            Some(Phase::Down { base, behind, left }) => {
                if rt.current == rt.lie {
                    let l = left - step;
                    if l <= 0.0 {
                        rt.queued = None;
                        play(en, rt, ke::get_up(base, behind), 0.0);
                        rt.queued = Some((ke::react::req::IDLE, manifest(rt.class).consts.get("attack_blend")));
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
            rt.handing_back = true;
        }
        done
    });
    match r {
        Some(true) => func::<StateFn>(f::DEFAULT_STATE)(en),
        Some(false) => {}
        None => {
            let s = crate::guard::state_index(en, rd(en + e::STATE)).map_or(state::FLINCH, |k| k as i32);
            call_original(en, ce::step_slot(s));
        }
    }
}

/// State 30 begin: Re:CoM's defeat (prizes, EXP, its removal timer), then the killing hit's KH1
/// reaction motion.
pub unsafe extern "win64" fn defeat_begin(en: usize) {
    let hit = ours(en).and_then(|k| hit_of(en, k));
    call_original(en, ce::begin_slot(state::DEFEAT));
    with_rt(en, |rt| {
        // KH1's hit handler rewrites the killing hit's kind too (it runs before the reaction).
        let hit = hit.map(|h| Hit { kind: rt.ai.reaction_kind(h.kind), ..h });
        rt.dead = true;
        rt.recom_flinch = false;
        rt.recom_state = false;
        if !start_reaction(en, rt, hit) {
            play(en, rt, ke::react::req::FLINCH, 0.0);
            rt.reaction = Some(Phase::Motion);
        }
        let cur = rt.current;
        line(rt.class, || format!("e{}: defeated (Re:CoM's defeat, KH1's motion {:?})", rd::<u32>(en + e::ID), cur));
    });
}

/// State 30 step: KH1's body (the launch arc of the last hit), then Re:CoM's step (removal).
pub unsafe extern "win64" fn defeat_step(en: usize) {
    with_rt(en, |rt| {
        rt.dead_frames = rt.dead_frames.saturating_add(1);
        tick(en, rt, false)
    });
    call_original(en, ce::step_slot(state::DEFEAT));
}

/// `super::guard`: Sora's Guard blocked this enemy's attack. A KH1 enemy plays KH1's clash recoil
/// (`0x1402B8E10` cases 6 / 7 → `0x1402B9140`) through state 11; its class's title when handled.
pub(super) unsafe fn guard_block(en: usize) -> Option<&'static str> {
    if !any_installed() {
        return None;
    }
    let k = ours(en)?;
    let attack = with_rt(en, |rt| {
        let a = rt.action.and_then(|a| a.attack);
        rt.clash = a;
        a
    })
    .flatten();
    let a = attack?;
    // Owner 2026-11-02: a blocked attack whose recoil the model lacks gets Re:CoM's flinch, as
    // before M10 (`super::guard`'s picker path); our state 11 then runs Re:CoM's.
    if manifest(k).recoil_of(a).is_none() {
        with_rt(en, |rt| {
            rt.clash = None;
            rt.recom_flinch = true;
        });
        let name = CLASSES[k].attack_names.get(a as usize).copied().unwrap_or("?");
        line(k, || format!("e{}: Guard blocked its {name} (no KH1 recoil motion): Re:CoM's flinch", rd::<u32>(en + e::ID)));
        return None;
    }
    // Re:CoM's attack end with state 11 next (bit 0 off, its card end).
    func::<EndFn>(f::ATTACK_END)(en, crate::battle::state_address(en, state::FLINCH), 0);
    let name = CLASSES[k].attack_names.get(a as usize).copied().unwrap_or("?");
    line(k, || format!("e{}: Guard blocked its {name}", rd::<u32>(en + e::ID)));
    Some(CLASSES[k].title)
}

/// `super::guard`: Re:CoM's picker ran for a block `guard_block` handed back; without its flinch the
/// mark is dropped (a later hit is KH1's reaction again).
pub(super) unsafe fn recom_flinch_settled(en: usize, flinched: bool) {
    if !flinched && any_installed() {
        with_rt(en, |rt| rt.recom_flinch = false);
    }
}

/// `super::guard`: an attack collision owned by `owner` (a handle) hit Sora: a KH1 enemy's
/// behaviour hears it for the attack it runs (KH1's attack object hit callback, `+0x38`).
pub(super) unsafe fn attack_landed(owner: u32) {
    if !any_installed() {
        return;
    }
    let en = func::<HandleFn>(f::HANDLE_OBJ)(owner);
    if en == 0 || ours(en).is_none() {
        return;
    }
    with_rt(en, |rt| {
        if let Some(a) = rt.action.and_then(|a| a.attack) {
            rt.ai.on_landed(a);
            let name = CLASSES[rt.class].attack_names.get(a as usize).copied().unwrap_or("?");
            line(rt.class, || format!("e{}: its {name} hit Sora", rd::<u32>(en + e::ID)));
        }
    });
}

/// The trace's tokens (F9 recording): per class (`shadow …`, `soldier …`), every KH1 enemy's
/// request, reaction, action and hittable flag.
pub unsafe fn trace_token() -> String {
    if !any_installed() {
        return String::new();
    }
    let Ok(v) = RT.try_lock() else { return String::new() };
    let mut out = Vec::new();
    for (k, c) in CLASSES.iter().enumerate() {
        let t: Vec<String> = v
            .iter()
            // Re:CoM removes a defeated enemy after its 1500-unit (30-frame) timer: not listed after.
            .filter(|r| r.class == k && r.battle == super::battle::battle_number() && !(r.dead && r.dead_frames > 31))
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
            .collect();
        if !t.is_empty() {
            out.push(format!("{} {}", c.name, t.join(" ")));
        }
    }
    out.join("; ")
}
