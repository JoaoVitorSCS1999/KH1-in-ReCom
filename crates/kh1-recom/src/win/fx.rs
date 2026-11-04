//! M13 stage 3: KH1's effects and sounds for Sora's moves (`crate::fx`; `docs/recom-m13-plan.md`
//! § Stage 3 built).
//!
//! The asset step's events are Re:CoM's own `.EET` / `.SES` events at KH1's variant; Re:CoM starts,
//! steps, draws and ends them. Ours:
//! - [`variants`]: the two variant arguments of the motion request Sora's KH1 controller already makes
//!   (`win::movement` `play`), by the Effects / Sounds settings and the manifest's moves. Once per
//!   motion, not per frame.
//! - KH1's trail end. Re:CoM's list-B event (`0x14017AB70`) can only fade or kill its effect at its end
//!   frame (`0x140254D60(e, fade)`), and its motion stop (`0x14017A430`) fades or kills the live ones
//!   and leaves the ended ones in the linger list. KH1 stops the trail's emission at the end of its
//!   window and lets its particles finish (stop byte `+0x91`, `0x1402C1F20`), and a motion change kills
//!   it, ended or not (no flag 0x800). So: the end-frame call is retargeted to [`trail_end`], which for
//!   a KH1 trail (its fade is the manifest's marker) stops the emission (Re:CoM's own `0x140255040`),
//!   else calls Re:CoM's fade as before; and the motion stop is wrapped ([`motion_stop`]): after
//!   Re:CoM's, the KH1 trails in that player's linger list are killed (`0x140254D60(e, 0)`). A KH1 trail
//!   still live at a motion change is killed by Re:CoM's own stop (its fade ≤ 0). Each runs once per
//!   trail end / motion change of an effect player; nothing per frame.

use super::hook;
use super::log;
use super::movement::{func, rd};
use crate::fx;
use crate::settings::Mode;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

static FX: OnceLock<fx::Manifest> = OnceLock::new();
/// The motion stop's trampoline.
pub static ORIG_MOTION_STOP: AtomicUsize = AtomicUsize::new(0);
/// The trail end is ours (the call was retargeted).
static TRAILS: AtomicBool = AtomicBool::new(false);
static NOTED: AtomicUsize = AtomicUsize::new(0);

const FADE_OUT: u32 = 0x0025_4D60;
const STOP_EMIT: u32 = 0x0025_5040;
const OF_HANDLE: u32 = 0x0025_7EF0;
/// The effect player's linger list (`0x14017A1A0`): `+0x28` count, `+0x30` 0x20-byte entries
/// `{i32 handle, …, +0x10 list-B event}` (64 at most). Checked (decomp).
const LINGER_COUNT: usize = 0x28;
const LINGER: usize = 0x30;
const LINGER_ENTRY: usize = 0x20;
const LINGER_MAX: i32 = 64;
const EVENT_FADE: usize = 0x60;

type FadeFn = unsafe extern "win64" fn(usize, f32);
type StopFn = unsafe extern "win64" fn(usize);
type OfHandleFn = unsafe extern "win64" fn(i32) -> usize;
type MotionStopFn = unsafe extern "win64" fn(usize);

fn note(bit: usize, msg: impl FnOnce() -> String) {
    if NOTED.fetch_or(1 << bit, Ordering::Relaxed) & (1 << bit) == 0 {
        log(&msg());
    }
}

/// Reads `kh1-fx.json` and puts KH1's trail end in. Without the manifest, Re:CoM's events only.
pub fn install(base: usize, manifest: Option<&Path>) {
    let Some(path) = manifest else {
        log("fx (M13): no kh1-fx.json (asset step not run?): Sora's KH1 moves play Re:CoM's effects and sounds");
        return;
    };
    let m = match std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display())).and_then(|t| fx::parse(&t)) {
        Ok(m) => m,
        Err(e) => {
            log(&format!("fx (M13): {e}; Re:CoM's effects and sounds"));
            return;
        }
    };
    let s = super::settings();
    log(&format!(
        "fx (M13): KH1 events at variant {} for {} moves with effects {:?} and {} with sounds {:?}; effects = {}, sounds = {}",
        m.variant,
        m.effects.len(),
        m.effects,
        m.sounds.len(),
        m.sounds,
        s.effects.as_str(),
        s.sounds.as_str()
    ));
    let _ = FX.set(m);
    // SAFETY: reads of the mapped exe; the patches check their bytes first.
    unsafe { install_trail_end(base) };
}

unsafe fn install_trail_end(base: usize) {
    for c in crate::sites::FX_CHECKED {
        if std::slice::from_raw_parts((base + c.rva as usize) as *const u8, c.expect.len()) != c.expect {
            log(&format!("fx (M13): {} is not as expected: KH1 trails end as Re:CoM's (killed at the window's end, kept past a motion change)", c.name));
            return;
        }
    }
    let p = &crate::sites::FX_PTRS[0];
    if rd::<usize>(base + p.rva as usize) != base + p.target as usize {
        log(&format!("fx (M13): {} does not point at 0x14017A430: KH1 trails end as Re:CoM's", p.name));
        return;
    }
    let hooks = [hook::Hook { site: &crate::sites::FX_HOOKS[0], ours: motion_stop as *const () as usize, original: &ORIG_MOTION_STOP }];
    if !hook::install_all(base, &hooks) {
        log("fx (M13): the EET motion stop is not wrapped: KH1 trails end as Re:CoM's");
        return;
    }
    let s = &crate::sites::FX_CALLS[0];
    let at = base + s.rva as usize;
    let call = hook::place_near(at, &crate::hookplan::abs_jmp(trail_end as *const () as u64)).and_then(|stub| crate::hookplan::call_rel32(at as u64, stub as u64).map(|c| (stub, c)));
    let Some((stub, call)) = call else {
        log("fx (M13): no stub within reach of the list-B stop call: KH1 trails are killed at their window's end");
        return;
    };
    if hook::patch(at, s.expect, &call, s.name) {
        TRAILS.store(true, Ordering::Release);
        log(&format!("fx (M13): {} retargeted through {stub:#x}; EET motion stop wrapped: KH1 trails stop emitting at their window's end and die at a motion change (KH1's)", s.name));
    }
}

/// The variants for KH1 request `request` (`ses_other`: a variant the caller already picked, a cast's
/// voice). With KH1's sounds, KH1's banks are made sure of first (one walk of Re:CoM's bank list after
/// each battle start; an atomic load otherwise).
pub fn variants(request: u16, ses_other: i32) -> (i32, i32) {
    let Some(m) = FX.get() else { return (0, ses_other) };
    let s = super::settings();
    let (eet, ses) = fx::variants(Some(m), request, s.effects, s.sounds, ses_other);
    if ses == m.variant && s.sounds == Mode::Kh1 {
        // SAFETY: on Re:CoM's update thread (the motion request's).
        unsafe { super::sounds::ensure() };
    }
    (eet, ses)
}

/// Re:CoM's list-B end-frame stop (`0x140254D60(e, fade)` at `0x14017ABEF`), with KH1's trail end:
/// a KH1 trail stops emitting; every other event is faded as before.
pub unsafe extern "win64" fn trail_end(effect: usize, fade: f32) {
    if let Some(m) = FX.get() {
        if fx::is_kh1_trail_fade(m, fade) {
            func::<StopFn>(STOP_EMIT)(effect);
            note(0, || "fx (M13): first KH1 trail end: emission stopped (its particles finish)".into());
            return;
        }
    }
    func::<FadeFn>(FADE_OUT)(effect, fade)
}

/// Re:CoM's EET motion stop, then KH1's rule for the trails it leaves lingering: killed.
pub unsafe extern "win64" fn motion_stop(cem: usize) {
    func_at::<MotionStopFn>(&ORIG_MOTION_STOP)(cem);
    if !TRAILS.load(Ordering::Acquire) {
        return;
    }
    let Some(m) = FX.get() else { return };
    let n: i32 = rd(cem + LINGER_COUNT);
    for k in 0..n.clamp(0, LINGER_MAX) as usize {
        let e = cem + LINGER + LINGER_ENTRY * k;
        let event: usize = rd(e + 0x10);
        if event == 0 || !fx::is_kh1_trail_fade(m, rd::<f32>(event + EVENT_FADE)) {
            continue;
        }
        let fx_ptr = func::<OfHandleFn>(OF_HANDLE)(rd::<i32>(e));
        if fx_ptr != 0 {
            func::<FadeFn>(FADE_OUT)(fx_ptr, 0.0);
            note(1, || "fx (M13): first KH1 trail killed at a motion change after its window (KH1's rule)".into());
        }
    }
}

unsafe fn func_at<T: Copy>(a: &AtomicUsize) -> T {
    std::mem::transmute_copy(&a.load(Ordering::Acquire))
}
