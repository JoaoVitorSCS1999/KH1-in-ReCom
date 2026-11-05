//! M13 stage 3: KH1's effects and sounds for Sora's moves (`crate::fx`; `docs/recom-m13-plan.md`
//! § Stage 3 built).
//!
//! The asset step's events are Re:CoM's own `.EET` / `.SES` events at KH1's variant; Re:CoM starts,
//! steps, draws and ends them. Ours:
//! - [`variants`]: the two variant arguments of the motion requests Sora's KH1 controller already makes
//!   (`win::movement` `play`, and every swing's in `win::attack::apply_row` since 0.131.0: before, swings
//!   passed 0 and played Re:CoM's events), by the Effects / Sounds settings and the manifest's moves. Once
//!   per motion, not per frame. [`swing_started`] logs, once per battle per finisher, what Re:CoM bound.
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
//! - M13 stage 4: the manifest is read before the Keyblade model table ([`preload`]), which names each
//!   KH1 Keyblade group's own `PL00GG.EET` / `PL00GG.EFF` when they are on disk ([`own_files`]); the SES
//!   variant follows the equipped Keyblade (`stats::equipped_card`, one pointer read per motion).

use super::hook;
use super::log;
use super::movement::{func, rd};
use crate::fx;
use crate::settings::Mode;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::OnceLock;

static FX: OnceLock<fx::Manifest> = OnceLock::new();
/// The KH1 Keyblade groups whose own files were found next to the manifest.
static OWN: OnceLock<Vec<(u16, crate::keyblade_models::OwnFiles)>> = OnceLock::new();
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

/// The per-battle swing lines (0.131.0): the battle's number and the swings already logged in it.
static BATTLE: AtomicU32 = AtomicU32::new(0);
static LOGGED: AtomicU64 = AtomicU64::new(0);
/// An entity's EET player (`CEffectMotion`, `0x1401A3470` starts it from `+0x108`); in it the group the
/// EET was found by (`+0x10`, `0x140179A30`), the bound events' count (`+0x24`) and the 32 slots
/// (`+0x830`; a slot's `+8` = list-A event, `+0x10` = list-B event, `0x14017A240`). Checked (decomp).
const ENT_EET: usize = 0x108;
const CEM_GROUP: usize = 0x10;
const CEM_COUNT: usize = 0x24;
const CEM_SLOTS: usize = 0x830;
const CEM_MAX: i32 = 32;
/// An event's effect id and variant (lists A and B alike).
const EVENT_ID: usize = 0x20;
const EVENT_VARIANT: usize = 0x5C;
/// Re:CoM's resource lookup (type, name, case) and its data (`sites::FIELD_SLEIGHT_CHECKED`); the ESE type.
const RES_LOOKUP: u32 = 0x002A_43E0;
const RES_DATA: u32 = 0x002A_49C0;
const TYPE_ESE: u32 = 0x3E;

type ByNameFn = unsafe extern "win64" fn(*const u8) -> usize;
type LookupFn = unsafe extern "win64" fn(u32, *const u8, i32) -> usize;
type DataFn = unsafe extern "win64" fn(usize) -> usize;

type FadeFn = unsafe extern "win64" fn(usize, f32);
type StopFn = unsafe extern "win64" fn(usize);
type OfHandleFn = unsafe extern "win64" fn(i32) -> usize;
type MotionStopFn = unsafe extern "win64" fn(usize);

fn note(bit: usize, msg: impl FnOnce() -> String) {
    if NOTED.fetch_or(1 << bit, Ordering::Relaxed) & (1 << bit) == 0 {
        log(&msg());
    }
}

/// Reads `kh1-fx.json` (before the Keyblade model table: stage 4's group files) and checks which
/// group files are on disk next to it (`PLAYER/00GG/PL00GG.EET` loose, `raw/PLAYER/00GG/PL00GG.EFF`).
pub fn preload(manifest: Option<&Path>) {
    let Some(path) = manifest else { return };
    let m = match std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display())).and_then(|t| fx::parse(&t)) {
        Ok(m) => m,
        Err(e) => {
            log(&format!("fx (M13): {e}; Re:CoM's effects and sounds"));
            return;
        }
    };
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let at = |rel: &str| rel.split('/').fold(dir.clone(), |p, s| p.join(s));
    let mut own = Vec::new();
    let mut missing = Vec::new();
    for k in &m.keyblades {
        let o = crate::keyblade_models::OwnFiles { eet: at(&k.eet).is_file(), eff: at(&format!("raw/{}", k.eff)).is_file() };
        // Both or neither: a trail event naming a set that isn't loaded draws nothing (Re:CoM: handle 0).
        if o.eet && o.eff {
            own.push((k.group, o));
        } else {
            missing.push(k.group);
        }
    }
    if !m.keyblades.is_empty() {
        log(&format!(
            "fx (M13 stage 4): {} KH1 Keyblade groups with their own trail and set {:?}{}",
            own.len(),
            own.iter().map(|o| o.0).collect::<Vec<_>>(),
            if missing.is_empty() { String::new() } else { format!("; files not found for {missing:?} (installed? they keep the Kingdom Key's trail)") }
        ));
    }
    let _ = OWN.set(own);
    let _ = FX.set(m);
}

/// Group `g`'s own files for the PLAYER table (none without the manifest or the files).
pub fn own_files(g: u16) -> crate::keyblade_models::OwnFiles {
    OWN.get().and_then(|v| v.iter().find(|o| o.0 == g)).map(|o| o.1).unwrap_or_default()
}

/// Puts KH1's trail end in (the manifest read by [`preload`]). Without the manifest, Re:CoM's events only.
pub fn install(base: usize) {
    let Some(m) = FX.get() else {
        log("fx (M13): no kh1-fx.json (asset step not run?): Sora's KH1 moves play Re:CoM's effects and sounds");
        return;
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
    let card = super::stats::equipped_card();
    let (eet, ses) = fx::variants(Some(m), request, s.effects, s.sounds, ses_other, card);
    if s.sounds == Mode::Kh1 && fx::is_kh1_sound_variant(m, ses) {
        // The banks of the Keyblade whose sounds play: the Kingdom Key's at its variant (the KH2
        // Keyblades, a Keyblade without its own), else the equipped one's.
        let owner = if ses == m.variant { crate::keyblades::KINGDOM_KEY } else { card };
        // SAFETY: on Re:CoM's update thread (the motion request's).
        unsafe { super::sounds::ensure(owner) };
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

/// A battle starts (the battle manager's ctor wrapper): its swing lines start again.
pub fn battle_start() {
    BATTLE.fetch_add(1, Ordering::Relaxed);
    LOGGED.store(0, Ordering::Relaxed);
}

/// The lookups the swing line calls, their bytes checked once.
fn lookups_ok() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| {
        let b = super::base();
        let all = crate::sites::EFFECT_CHECKED.iter().chain(crate::sites::FIELD_SLEIGHT_CHECKED);
        [crate::sites::EFFECT_CHECKED[0].rva, RES_LOOKUP, RES_DATA].iter().all(|&rva| {
            all.clone().find(|c| c.rva == rva).is_some_and(|c| {
                // SAFETY: a read of the mapped exe.
                unsafe { std::slice::from_raw_parts((b + c.rva as usize) as *const u8, c.expect.len()) == c.expect }
            })
        })
    })
}

/// The events an entity's EET player bound at its last motion start.
unsafe fn bound(ent: usize) -> Option<(u32, Vec<fx::Bound>)> {
    let cem: usize = rd(ent + ENT_EET);
    if cem == 0 {
        return None;
    }
    let mut out = Vec::new();
    for k in 0..rd::<i32>(cem + CEM_COUNT).clamp(0, CEM_MAX) as usize {
        let slot: usize = rd(cem + CEM_SLOTS + 8 * k);
        if slot == 0 {
            continue;
        }
        let (a, b): (usize, usize) = (rd(slot + 8), rd(slot + 0x10));
        let (list, ev) = if a != 0 { ('A', a) } else if b != 0 { ('B', b) } else { continue };
        let name: [u8; 0x20] = rd(ev);
        let n = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        out.push(fx::Bound { list, dpx: String::from_utf8_lossy(&name[..n]).into_owned(), id: rd(ev + EVENT_ID), variant: rd(ev + EVENT_VARIANT), set: None });
    }
    Some((rd(cem + CEM_GROUP), out))
}

/// What Re:CoM has under a set's name: Re:CoM's own effect-by-name (`0x1402A4920` → the set's
/// wrapper, whose first word is the set: `0x14029F9C0` / `0x140255100`, read by `0x1402553B0`) and the
/// `.ese` of the same name (type 0x3E, as `0x1402A4A10`). Checked (decomp).
unsafe fn set_info(dpx: &str, id: i32) -> fx::SetInfo {
    let name = crate::effects::lookup_name(dpx);
    let wrapper = func::<ByNameFn>(crate::sites::EFFECT_CHECKED[0].rva)(name.as_ptr());
    let data: usize = if wrapper != 0 { rd(wrapper) } else { 0 };
    let ids: Vec<u32> = if data != 0 { (0..rd::<i32>(data + 0xC).clamp(0, 256) as usize).map(|k| rd::<u32>(data + 0x10 + 0x20 * k + 8)).collect() } else { Vec::new() };
    let res = func::<LookupFn>(RES_LOOKUP)(TYPE_ESE, name.as_ptr(), 0);
    let ese = if res != 0 { func::<DataFn>(RES_DATA)(res) } else { 0 };
    let sound = (ese != 0)
        .then(|| (0..rd::<u16>(ese + 0xE).min(256) as usize).map(|k| ese + 0x10 + 8 * k).find(|&r| rd::<u16>(r) as i32 == id).map(|r| rd::<u16>(r + 2) as u32 * 1000 + rd::<u16>(r + 4) as u32));
    fx::SetInfo { loaded: data != 0, entries: ids, ese: ese != 0, sound: sound.flatten() }
}

/// After a KH1 swing's motion request (`win::attack::apply_row`): once per battle for each finisher with
/// KH1 effects and for the battle's first swing, one line with the variants passed and the events
/// Re:CoM bound for Sora and the Keyblade (their sets and `.ese` looked up). Not per frame.
pub unsafe fn swing_started(pl: usize, request: u16, row: u8, eet: i32, ses: i32) {
    let Some(m) = FX.get() else { return };
    let finisher = m.finishers.iter().find(|f| f.0 == request);
    let bit = 1u64 << if finisher.is_some() { 1 + request as u32 % 63 } else { 0 };
    if LOGGED.fetch_or(bit, Ordering::Relaxed) & bit != 0 {
        return;
    }
    let ok = lookups_ok();
    let look = |b: Option<(u32, Vec<fx::Bound>)>| {
        b.map(|(g, mut evs)| {
            if ok {
                for e in &mut evs {
                    e.set = Some(set_info(&e.dpx, e.id));
                }
            }
            (g, evs)
        })
    };
    let sora = look(bound(pl));
    let w: usize = rd(pl + crate::attack::field::WEAPON);
    let weapon = look(if w != 0 { bound(w) } else { None });
    let card = super::stats::equipped_card();
    log(&fx::swing_line(BATTLE.load(Ordering::Relaxed), request, row, finisher.map(|f| f.1.as_slice()), eet, ses, card, sora.as_ref(), weapon.as_ref()));
}
