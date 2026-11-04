//! True squeeze to fit for our texts drawn in Re:CoM's own text (`crate::text_fit`; notes
//! `docs/recom-m12-sleights.md` § 8.8).
//!
//! [`fit`] measures one of our `C2DFnt`s with Re:CoM's width routine (`0x14025A380`) and, when it is
//! wider than its box, gives it one of our copies of Re:CoM's C2DFnt vtable (the scale in the copy).
//! Re:CoM's 2D manager then draws it as any text, through the copy's `+0x18` ([`draw`]: Re:CoM's draw,
//! then the vertices it wrote scaled in x about the text's x). Re:CoM's own texts keep Re:CoM's
//! vtable: no Re:CoM code byte changes and nothing of ours runs for them.
//!
//! Every offset and call below is Checked (decomp + disasm; `sites::TEXT_FIT_CHECKED` /
//! `TEXT_FIT_PTRS`) unless tagged otherwise.

use super::movement::{func, rd, wr};
use super::{base, log};
use crate::text_fit::recom::{DRAW, EXPAND, EXPAND_BUF, FNT_SIZE, MEASURE, PACKET_HOLDER, TEXT, VTABLE, WIDTH, X};
use crate::text_fit::{self as tf, vtable};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// A C2DFnt draw's packet larger than this (bytes) is not touched (a 2048-character text writes
/// about 160 KiB).
const PACKET_MAX: usize = 1 << 20;

type DrawFn = unsafe extern "win64" fn(usize);
type MeasureFn = unsafe extern "win64" fn(usize, *mut i32, *mut i32, *mut i32) -> i32;
type WidthFn = unsafe extern "win64" fn(*mut i32, usize, *mut i32, *mut i32) -> i32;
type ExpandFn = unsafe extern "win64" fn(*const u8, *mut u8) -> i16;
/// Texts this long or longer are not measured (an expansion at most doubles a text; Re:CoM's
/// buffer is 2048).
const EXPAND_MAX: usize = 900;

/// `sites::TEXT_FIT_PTRS` checked once.
static READY: OnceLock<bool> = OnceLock::new();
/// One vtable copy per level (0: not made yet), leaked: a text may hold it as long as it lives.
static POOL: [AtomicUsize; tf::ONE as usize] = [const { AtomicUsize::new(0) }; tf::ONE as usize];
static POOL_LOCK: Mutex<()> = Mutex::new(());
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
static DRAW_LOG_LEFT: AtomicU32 = AtomicU32::new(5);
/// The (text, box, level) lines already logged.
static LOGGED: Mutex<Vec<(Vec<u8>, u32, u32)>> = Mutex::new(Vec::new());

fn ready() -> bool {
    *READY.get_or_init(|| {
        let b = base();
        // SAFETY: the exe's own data section (pointer-sized vtable words).
        let bad = crate::sites::TEXT_FIT_PTRS.iter().find(|p| unsafe { rd::<usize>(b + p.rva as usize) } != b + p.target as usize);
        match bad {
            Some(p) => log(&format!("text fit: {} does not point at {:#x}; long names are not squeezed in Re:CoM's text", p.name, 0x1_4000_0000u64 + p.target as u64)),
            None => log("text fit: Re:CoM's C2DFnt vtable checked (COL + 7 slots): long names in Re:CoM's text squeeze to fit"),
        }
        bad.is_none()
    })
}

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("text fit: {}", line()));
    }
}

/// The text's bytes (up to `max`, without its NUL).
unsafe fn text_of_n(fnt: usize, max: usize) -> Vec<u8> {
    let p: usize = rd(fnt + TEXT);
    let mut v = Vec::new();
    if p == 0 {
        return v;
    }
    for k in 0..max {
        match rd::<u8>(p + k) {
            0 => break,
            c => v.push(c),
        }
    }
    v
}

/// The text's bytes (up to 64, for logs and the once-only record).
unsafe fn text_of(fnt: usize) -> Vec<u8> {
    text_of_n(fnt, 64)
}

fn shown(t: &[u8]) -> String {
    t.iter().map(|&c| if (0x20..0x7F).contains(&c) { (c as char).to_string() } else { format!("\\x{c:02x}") }).collect()
}

/// Re:CoM's width of `fnt`'s text as it is set now (1/16 units, the pen's extent at scale 1), or
/// None (no text, or Re:CoM's font not loaded: its routine then writes nothing). Measured as the
/// draw sees it: the draw first expands the button escapes with `0x1403480C0` into its 2048-byte
/// buffer (`0x14025AC2B`) and the width routine reads the raw text, so the text is expanded the
/// same way and measured on a copy of the object (the routine reads `+0x3C`, `+0x40`, `+0x44`,
/// `+0x58` and writes nothing to it: Checked (decomp)); the object itself is not touched.
pub unsafe fn width(fnt: usize) -> Option<i32> {
    if fnt == 0 {
        return None;
    }
    let raw = text_of_n(fnt, EXPAND_MAX);
    if raw.is_empty() || raw.len() >= EXPAND_MAX {
        return None;
    }
    let mut src = raw;
    src.push(0);
    let mut expanded = vec![0u8; EXPAND_BUF];
    func::<ExpandFn>(EXPAND)(src.as_ptr(), expanded.as_mut_ptr());
    let mut copy = [0u64; FNT_SIZE / 8];
    std::ptr::copy_nonoverlapping(fnt as *const u8, copy.as_mut_ptr() as *mut u8, FNT_SIZE);
    let c = copy.as_mut_ptr() as usize;
    wr::<usize>(c + TEXT, expanded.as_ptr() as usize);
    let mut out = [0i32; 2];
    let lines = func::<WidthFn>(WIDTH)(out.as_mut_ptr(), c, std::ptr::null_mut(), std::ptr::null_mut());
    (lines > 0 && out[0] > 0).then_some(out[0])
}

/// The copy of Re:CoM's vtable for `level` (made once), or None (no memory).
fn copy_for(level: u32) -> Option<usize> {
    let slot = POOL.get(level as usize)?;
    let have = slot.load(Ordering::Acquire);
    if have != 0 {
        return Some(have);
    }
    let _g = POOL_LOCK.lock().ok()?;
    let have = slot.load(Ordering::Acquire);
    if have != 0 {
        return Some(have);
    }
    let b = base();
    let vt = b + VTABLE as usize;
    // SAFETY: Re:CoM's vtable words (checked by `ready`).
    let (col, recom) = unsafe { (rd::<u64>(vt - 8), std::array::from_fn(|i| rd::<u64>(vt + 8 * i))) };
    let words = vtable::copy(level, col, recom, draw as *const () as u64, measure as *const () as u64);
    let leaked: &'static mut [u64; vtable::WORDS] = Box::leak(Box::new(words));
    let vptr = leaked.as_ptr() as usize + 8 * vtable::VPTR_AT;
    slot.store(vptr, Ordering::Release);
    Some(vptr)
}

/// The level a text's vtable carries: ours, or [`tf::ONE`] for Re:CoM's.
unsafe fn level_of_vptr(vt: usize) -> Option<u32> {
    if vt == base() + VTABLE as usize {
        return Some(tf::ONE);
    }
    // A vtable pointer that is neither Re:CoM's C2DFnt one nor one of ours: refused before reading.
    let ours = POOL.iter().any(|p| p.load(Ordering::Relaxed) == vt);
    ours.then(|| vtable::level_of([rd(vt - 24), rd(vt - 16)])).flatten()
}

/// The level of the copy `vt` (only our copies' slots call [`draw`] / [`measure`], so `vt` is one;
/// its marker is checked anyway); [`tf::ONE`] otherwise.
unsafe fn copy_level(vt: usize) -> u32 {
    vtable::level_of([rd(vt - 24), rd(vt - 16)]).unwrap_or(tf::ONE)
}

/// Squeeze to fit: our text `fnt` (a `C2DFnt` our code made or Re:CoM's row set-up made for our
/// row) drawn no wider than `box_units` from its aligned edge. Its text must be set. Fits (or
/// unknown width): Re:CoM's own vtable (as it was made). The level set.
pub unsafe fn fit(fnt: usize, box_units: f32, what: impl FnOnce() -> String) -> u32 {
    if fnt == 0 || !ready() {
        return tf::ONE;
    }
    let vt: usize = rd(fnt);
    if level_of_vptr(vt).is_none() {
        note(|| format!("{}: not a C2DFnt (vtable {vt:#x}); not squeezed", what()));
        return tf::ONE;
    }
    let w = width(fnt);
    let level = w.map_or(tf::ONE, |w| tf::level(w, box_units));
    let want = if level >= tf::ONE { Some(base() + VTABLE as usize) } else { copy_for(level) };
    let Some(want) = want else {
        note(|| format!("{}: no memory for a vtable copy; not squeezed", what()));
        return tf::ONE;
    };
    if want != vt {
        wr::<usize>(fnt, want);
    }
    if level < tf::ONE {
        let t = text_of(fnt);
        let box16 = (box_units * 16.0) as u32;
        let first = LOGGED.lock().map(|mut l| {
            let k = (t.clone(), box16, level);
            let new = !l.contains(&k);
            if new && l.len() < 512 {
                l.push(k);
            }
            new
        });
        if first.unwrap_or(false) {
            let w = w.unwrap_or(0) as f32 / 16.0;
            note(|| format!("squeezed: {}: \"{}\" {w:.1} units wide, box {box_units:.0}: x scale {:.3} ({level}/1024)", what(), shown(&t), tf::factor(level)));
        }
    }
    level
}

/// Our C2DFnt vtable `+0x18`: Re:CoM's draw, then the x of the vertices it wrote scaled about the
/// text's x. Runs only for our squeezed texts, once per frame each (Re:CoM's 2D manager).
unsafe extern "win64" fn draw(fnt: usize) {
    let b = base();
    let level = copy_level(rd(fnt));
    let holder = b + PACKET_HOLDER as usize;
    let buf: usize = rd(holder);
    let p0: usize = if buf != 0 { rd(buf) } else { 0 };
    func::<DrawFn>(DRAW)(fnt);
    if level >= tf::ONE || buf == 0 || rd::<usize>(holder) != buf {
        return;
    }
    let p1: usize = rd(buf);
    if p1 <= p0 || p1 - p0 > PACKET_MAX || p0 % 8 != 0 {
        return;
    }
    // SAFETY: the qwords Re:CoM's draw wrote just now, from its packet buffer's write pointer before
    // the draw to after (`0x140281180` reserves from it, `0x140280F30` leaves it past the end).
    let q = std::slice::from_raw_parts_mut(p0 as *mut u64, (p1 - p0) / 8);
    if let Err(e) = tf::packet::squeeze(q, rd::<i32>(fnt + X), level) {
        if DRAW_LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
            log(&format!("text fit: draw of {fnt:#x}: {e}; drawn unsqueezed"));
        }
    }
}

/// Our C2DFnt vtable `+0x28`: Re:CoM's measure, its width × the scale (the drawn width). The line
/// table stays Re:CoM's (pen space: the draw reads such a table and our draw scales its result).
unsafe extern "win64" fn measure(fnt: usize, out: *mut i32, lines: *mut i32, count: *mut i32) -> i32 {
    let n = func::<MeasureFn>(MEASURE)(fnt, out, lines, count);
    let level = copy_level(rd(fnt));
    if n > 0 && !out.is_null() && level < tf::ONE {
        *out = tf::scaled(*out as i64, level) as i32;
    }
    n
}
