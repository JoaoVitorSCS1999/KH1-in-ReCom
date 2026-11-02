//! Boss revenge at Combat = KH1 (`crate::revenge`, `docs/recom-boss-revenge.md`).
//!
//! - **Counter:** a 0x20-byte tail after each revenge boss body (`crate::revenge::tail`), made by
//!   raising the size Re:CoM's enemy factory `0x140188F40` allocates for the class (one 5-byte
//!   `mov ecx, size` each, written once at install) and set up when Re:CoM's stats init
//!   `0x14017FF50` runs for the boss (our existing wrapper). A tail from another battle or class is
//!   set up again on its first use (its magic and battle number).
//! - **Add:** [`enemy_hit`] wraps the existing KH1 enemy-hit wrapper (`super::attack::enemy_hit`,
//!   Re:CoM's `0x14017F7E0`): a landed hit adds its value (`crate::revenge::value_of`; the source is
//!   noted by `super::attack`). Marluxia's parts add to Marluxia's body. Re:CoM's boss damage
//!   falloff (`+0x288` / `+0x28C` / `+0x290`, written by `0x14017F7E0`) is put back after the call
//!   for bosses whose revenge acts (owner: off for them).
//! - **Decay:** lazy, 1 per 60 Hz frame of the game clock ([`frame`], Sora's own dt added in the
//!   existing request-bits wrapper), minus the frames the boss spent in its flinch / blow-away.
//! - **Check:** hooks on Re:CoM's flinch step `0x140184100` (state 11) and blow-away step
//!   `0x140182E50` (state 12), shared by every enemy: they return at once for other enemies.
//!   At KH2's frame 20 (Hades 30) of the state, or at its end when Re:CoM's state is shorter,
//!   limit ≤ counter starts the boss's own attack state with the transition Re:CoM's message 2
//!   makes (the cardless card play P1's: `crate::battle::cardless_flags`, `state_address`).
//!
//! Only in a KH1 battle (Combat = KH1, Sora's story) with `boss_revenge = on`.

use super::movement::{func, orig, rd, wr};
use super::{base, log};
use crate::revenge::{self as rv, flag, tail, Boss, Counter, Source};
use crate::sites::{REVENGE_HOOKS, REVENGE_PTRS};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

pub static ORIG_FLINCH: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_BLOW: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static ENABLED: AtomicBool = AtomicBool::new(true);
/// The game clock: Re:CoM time units (50 a 60 Hz frame), Sora's dt summed each frame.
static CLOCK: AtomicU64 = AtomicU64::new(0);
/// The source of the hit being taken (0 none; 1 << 16 | row (0xFFFF: no row); 2 << 16 | spell).
static NOTE: AtomicU32 = AtomicU32::new(0);
/// The last Marluxia 2 / 3 body set up (its parts' hits count on it).
static MARLUXIA_BODY: AtomicUsize = AtomicUsize::new(0);
static MARLUXIA_BATTLE: AtomicU32 = AtomicU32::new(u32::MAX);
static LINES: AtomicU32 = AtomicU32::new(0);
const LINE_CAP: u32 = 200;

type StepFn = unsafe extern "win64" fn(usize);
type MaxHpFn = unsafe extern "win64" fn(usize) -> i32;
/// Re:CoM's enemy max-HP getter (hooked by `super::stats`; first bytes checked there).
const ENEMY_MAX_HP: u32 = 0x0017_F5A0;
/// Sora's dt (Re:CoM time units; `super::movement` `p::DT`).
const PL_DT: usize = 0x460;
/// The enemy's dt (`CEntity +0x14`, what the enemy timers subtract; `0x140185320`).
const EN_DT: usize = 0x14;
const EN_STATE: usize = 0x60;
const EN_NEXT: usize = 0x68;
const EN_FLAGS: usize = 0x260;
const EN_HP: usize = 0x274;
const EN_CHOSEN: usize = 0x278;
/// Re:CoM's boss damage falloff (`0x14017F7E0` for ids 100..199: hit count, reset timer,
/// multiplier). Checked (decomp).
const FALLOFF_COUNT: usize = 0x288;
const FALLOFF_TIMER: usize = 0x28C;
const FALLOFF_MULT: usize = 0x290;

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

fn live() -> bool {
    INSTALLED.load(Ordering::Relaxed) && ENABLED.load(Ordering::Relaxed) && super::battle::kh1_battle()
}

/// Sora's frame (from the request-bits wrapper): the clock advances by his dt.
pub(super) unsafe fn frame(pl: usize) {
    if INSTALLED.load(Ordering::Relaxed) {
        CLOCK.fetch_add(rd::<i32>(pl + PL_DT).clamp(0, 1000) as u64, Ordering::Relaxed);
    }
}

fn now() -> f64 {
    CLOCK.load(Ordering::Relaxed) as f64 / rv::TICKS_PER_FRAME
}

/// `super::attack`: the hit being taken is a KH1 swing with this row.
pub(super) fn note_swing(row: Option<usize>) {
    NOTE.store(1 << 16 | row.map_or(0xFFFF, |r| (r as u32) & 0xFFFF), Ordering::Relaxed);
}

/// `super::attack`: the hit being taken is a KH1 spell's.
pub(super) fn note_spell(s: u8) {
    NOTE.store(2 << 16 | s as u32, Ordering::Relaxed);
}

fn take_note() -> Source {
    let n = NOTE.swap(0, Ordering::Relaxed);
    match n >> 16 {
        1 if n & 0xFFFF == 0xFFFF => Source::Swing(None),
        1 => Source::Swing(Some((n & 0xFFFF) as usize)),
        2 => Source::Spell((n & 0xFF) as u8),
        _ => Source::Other,
    }
}

unsafe fn vt_rva(e: usize) -> u32 {
    let vt: usize = rd(e);
    vt.wrapping_sub(base()) as u32
}

unsafe fn boss_of(e: usize) -> Option<&'static Boss> {
    if e == 0 {
        return None;
    }
    rv::boss_by_vtable(vt_rva(e))
}

/// The tail's counter (set up again when it is another battle's or never was).
unsafe fn counter(e: usize, b: &Boss) -> Counter {
    let t = e + b.size as usize;
    let battle = super::battle::battle_number();
    if rd::<u32>(t + tail::MAGIC) != rv::MAGIC || rd::<u32>(t + tail::BATTLE) != battle {
        setup(e, b);
    }
    Counter {
        value: rd(t + tail::COUNTER),
        last: rd(t + tail::LAST),
        flinch_since: rd(t + tail::FLINCH_SINCE),
        flinch_elapsed: rd(t + tail::FLINCH_ELAPSED),
        flags: rd(t + tail::FLAGS),
    }
}

unsafe fn store(e: usize, b: &Boss, c: &Counter) {
    let t = e + b.size as usize;
    wr(t + tail::COUNTER, c.value);
    wr(t + tail::LAST, c.last);
    wr(t + tail::FLINCH_SINCE, c.flinch_since);
    wr(t + tail::FLINCH_ELAPSED, c.flinch_elapsed);
    wr(t + tail::FLAGS, c.flags);
}

unsafe fn setup(e: usize, b: &Boss) {
    let t = e + b.size as usize;
    std::ptr::write_bytes(t as *mut u8, 0, rv::TAIL as usize);
    wr(t + tail::MAGIC, rv::MAGIC);
    wr(t + tail::BATTLE, super::battle::battle_number());
    store(e, b, &Counter::new(now()));
    if rv::owns_parts(b) {
        MARLUXIA_BODY.store(e, Ordering::Relaxed);
        MARLUXIA_BATTLE.store(super::battle::battle_number(), Ordering::Relaxed);
    }
}

/// Re:CoM's stats init ran for this enemy (`super::stats::stat_init`): a boss body's tail is set
/// up (the allocation's tail holds whatever the heap had).
pub(super) unsafe fn on_init(e: usize) {
    if !INSTALLED.load(Ordering::Relaxed) {
        return;
    }
    if let Some(b) = boss_of(e) {
        setup(e, b);
    }
}

unsafe fn max_hp(e: usize) -> i32 {
    func::<MaxHpFn>(ENEMY_MAX_HP)(e)
}

/// The limit now (Hades: re-checked against his HP, as KH2 does on each hit).
unsafe fn limit_now(e: usize, b: &Boss, c: &mut Counter) -> f32 {
    let (l, low) = rv::limit(b, rd(e + EN_HP), max_hp(e), c.flags & flag::HADES_LOW != 0);
    if low {
        c.flags |= flag::HADES_LOW;
    }
    l
}

fn line(f: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < LINE_CAP).then_some(n + 1)).is_ok() {
        log(&f());
    }
}

/// The trace's hits column: `rv <counter>` on this enemy's hit.
fn trace_tag(e: usize, value: f32) {
    if let Ok(mut g) = super::attack::HIT_LOG.lock() {
        let key = format!("e{:x}", e & 0x00FF_FFFF);
        let n = g.len();
        match g.last_mut() {
            Some(l) if l.starts_with(&key) => l.push_str(&format!(":rv {value:.1}")),
            _ if n < 16 => g.push(format!("{key}:rv {value:.1}")),
            _ => {}
        }
    }
}

/// The enemy-hit hook (Re:CoM's `0x14017F7E0`): the KH1 wrapper, then the revenge add.
pub unsafe extern "win64" fn enemy_hit(enemy: usize) -> u64 {
    if !live() {
        return super::attack::enemy_hit(enemy);
    }
    let (body, b) = match boss_of(enemy) {
        Some(b) => (enemy, b),
        None if rv::is_part(vt_rva(enemy)) => {
            let same = MARLUXIA_BATTLE.load(Ordering::Relaxed) == super::battle::battle_number();
            let body = if same { MARLUXIA_BODY.load(Ordering::Relaxed) } else { 0 };
            match boss_of(body).filter(|b| rv::owns_parts(b)) {
                Some(b) => (body, b),
                None => return super::attack::enemy_hit(enemy),
            }
        }
        None => return super::attack::enemy_hit(enemy),
    };
    let falloff = b.falloff_off().then(|| (rd::<i32>(enemy + FALLOFF_COUNT), rd::<i32>(enemy + FALLOFF_TIMER), rd::<f32>(enemy + FALLOFF_MULT)));
    NOTE.store(0, Ordering::Relaxed);
    let r = super::attack::enemy_hit(enemy);
    let src = take_note();
    if let Some((n, t, m)) = falloff {
        wr(enemy + FALLOFF_COUNT, n);
        wr(enemy + FALLOFF_TIMER, t);
        wr(enemy + FALLOFF_MULT, m);
    }
    // A landed hit (Re:CoM applied it). Held hits of a stopped enemy count when they land.
    if r != 0 && !super::magic::stopped(enemy) {
        let v = rv::value_of(src);
        let mut c = counter(body, b);
        if body == enemy {
            c.hit(now(), v);
        } else {
            // A part's hit: on the body's counter; the body's own flinch goes on.
            c.decay(now());
            c.value += v;
        }
        limit_now(body, b, &mut c);
        store(body, b, &c);
        trace_tag(enemy, c.value);
    }
    r
}

unsafe fn step(e: usize, o: StepFn) {
    if !live() {
        return o(e);
    }
    let Some(b) = boss_of(e) else { return o(e) };
    let dt = rd::<i32>(e + EN_DT).clamp(0, 1000) as f32 / rv::TICKS_PER_FRAME as f32;
    o(e);
    let mut c = counter(e, b);
    c.flinch_frame(dt);
    let ending = rd::<usize>(e + EN_NEXT) != rd::<usize>(e + EN_STATE);
    if c.due(b.check_frame(), ending) {
        let l = limit_now(e, b, &mut c);
        let at = c.flinch_elapsed;
        if c.decide(now(), l) {
            let value = c.value;
            match b.attack {
                Some((state, what)) if super::battle::may_start_attack(e) => {
                    let flags: u64 = rd(e + EN_FLAGS);
                    wr(e + EN_FLAGS, crate::battle::cardless_flags(flags));
                    wr(e + EN_CHOSEN, state);
                    wr(e + EN_NEXT, crate::battle::state_address(e, state));
                    line(|| format!("revenge: {} {e:#x} counter {value:.1} >= {l} at frame {at:.0} of its {} -> state {state} ({what})", b.name, if ending { "reaction's end" } else { "reaction" }));
                }
                Some((state, _)) => line(|| format!("revenge: {} {e:#x} counter {value:.1} >= {l}, but it may not attack now (dead or CBTL locked): state {state} not started", b.name)),
                None => line(|| format!("revenge: {} {e:#x} counter {value:.1} >= {l}: no attack state of its own, nothing starts", b.name)),
            }
        }
    }
    store(e, b, &c);
}

/// Re:CoM's flinch step (state 11, `0x140184100`).
pub unsafe extern "win64" fn flinch_step(e: usize) {
    step(e, orig(&ORIG_FLINCH))
}

/// Re:CoM's blow-away step (state 12, `0x140182E50`).
pub unsafe extern "win64" fn blow_step(e: usize) {
    step(e, orig(&ORIG_BLOW))
}

/// Checks the classes and their state steps, hooks the two steps and raises the bosses'
/// allocation sizes: all or nothing.
pub unsafe fn install(bs: usize) -> bool {
    for b in rv::BOSSES {
        let want = format!(".?AV{}@@", b.rtti);
        match super::stats::rtti_name(bs, bs + b.vtable as usize) {
            Some(n) if n == want => {}
            n => {
                log(&format!("revenge: {} vtable {:#x} is {n:?}, not {want}: not installed", b.name, 0x1_4000_0000u64 + b.vtable as u64));
                return false;
            }
        }
    }
    for p in rv::PARTS {
        let want = format!(".?AV{}@@", p.rtti);
        if super::stats::rtti_name(bs, bs + p.vtable as usize).as_deref() != Some(want.as_str()) {
            log(&format!("revenge: part vtable {:#x} is not {want}: not installed", 0x1_4000_0000u64 + p.vtable as u64));
            return false;
        }
    }
    if let Some(p) = REVENGE_PTRS.iter().find(|p| rd::<usize>(bs + p.rva as usize) != bs + p.target as usize) {
        log(&format!("revenge: {} is not the state step we hook: not installed", p.name));
        return false;
    }
    let site = |name: &str| REVENGE_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        super::hook::Hook { site: site("enemy flinch step (0x140184100)"), ours: flinch_step as *const () as usize, original: &ORIG_FLINCH },
        super::hook::Hook { site: site("enemy blow-away step (0x140182E50)"), ours: blow_step as *const () as usize, original: &ORIG_BLOW },
    ];
    if !super::hook::install_all(bs, &hooks) {
        return false;
    }
    let mut done: Vec<&Boss> = Vec::new();
    for b in rv::BOSSES {
        if super::hook::patch(bs + b.alloc as usize, &rv::alloc_bytes(b.size), &rv::alloc_bytes_ours(b.size), b.rtti) {
            done.push(b);
        } else {
            for d in done {
                super::hook::patch(bs + d.alloc as usize, &rv::alloc_bytes_ours(d.size), &rv::alloc_bytes(d.size), d.rtti);
            }
            log("revenge: an allocation size held other bytes; sizes left as Re:CoM's, revenge off (the step hooks pass through)");
            return false;
        }
    }
    INSTALLED.store(true, Ordering::Release);
    true
}
