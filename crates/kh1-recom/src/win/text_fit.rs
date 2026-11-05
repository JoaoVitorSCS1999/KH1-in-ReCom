//! True squeeze to fit for our texts drawn in Re:CoM's own text (`crate::text_fit`; notes
//! `docs/recom-m12-sleights.md` § 8.8).
//!
//! [`fit`] measures one of our `C2DFnt`s with Re:CoM's width routine (`0x14025A380`) and, when it is
//! wider than its box, gives it one of our copies of Re:CoM's C2DFnt vtable (the scale in the copy).
//! Re:CoM's 2D manager then draws it as any text, through the copy's `+0x18` ([`draw`]: Re:CoM's draw,
//! then the vertices it wrote scaled in x about the text's x). Re:CoM's own texts keep Re:CoM's
//! vtable: no Re:CoM code byte changes and nothing of ours runs for them.
//!
//! Two of Re:CoM's text classes share that draw and measure: `C2DFnt` (rows, labels) and, since
//! 0.132.0, `CStrPrim` (a `CTextWnd`'s text: the Moogle's speech bubble, [`set_level`]). A copy
//! keeps its own class's COL and slots.
//!
//! Every offset and call below is Checked (decomp + disasm; `sites::TEXT_FIT_CHECKED` /
//! `TEXT_FIT_PTRS` / `TEXT_FIT_STR_PTRS`) unless tagged otherwise.

use super::movement::{func, rd, wr};
use super::{base, log};
use crate::text_fit::recom::{COL, DRAW, EXPAND, EXPAND_BUF, FNTM2_SIZE, MEASURE, PACKET_HOLDER, STR_COL, STR_VTABLE, TEXT, VTABLE, WIDTH, X};
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

/// Re:CoM's text classes a copy is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// `D2::C2DFnt` (`0x140606C98`).
    Fnt,
    /// `CStrPrim` (`0x140508FD0`), a `CTextWnd`'s text.
    Str,
}

impl Class {
    fn index(self) -> usize {
        self as usize
    }
    fn vtable(self) -> u32 {
        match self {
            Class::Fnt => VTABLE,
            Class::Str => STR_VTABLE,
        }
    }
    fn of_col(col: u64) -> Option<Class> {
        let c = col.checked_sub(base() as u64)?;
        [Class::Fnt, Class::Str].into_iter().find(|k| c == k.col() as u64)
    }
    fn col(self) -> u32 {
        match self {
            Class::Fnt => COL,
            Class::Str => STR_COL,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Class::Fnt => "C2DFnt",
            Class::Str => "CStrPrim",
        }
    }
}

/// `sites::TEXT_FIT_PTRS` / `TEXT_FIT_STR_PTRS` checked once each.
static READY: [OnceLock<bool>; 2] = [OnceLock::new(), OnceLock::new()];
/// One vtable copy per class and level (0: not made yet), leaked: a text may hold it as long as it
/// lives.
static POOL: [[AtomicUsize; tf::ONE as usize]; 2] = [const { [const { AtomicUsize::new(0) }; tf::ONE as usize] }; 2];
static POOL_LOCK: Mutex<()> = Mutex::new(());
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
static DRAW_LOG_LEFT: AtomicU32 = AtomicU32::new(5);
/// The (text, box, level) lines already logged.
static LOGGED: Mutex<Vec<(Vec<u8>, u32, u32)>> = Mutex::new(Vec::new());
/// Texts squeezed by [`set_level`] and the text each was squeezed for: when Re:CoM sets another
/// text on the same object (its text call measures it through our measure), Re:CoM's vtable goes
/// back (Re:CoM's own lines are never squeezed). At most [`OWNED_MAX`].
static OWNED: Mutex<Vec<(usize, Vec<u8>)>> = Mutex::new(Vec::new());
const OWNED_MAX: usize = 16;

fn ready(class: Class) -> bool {
    *READY[class.index()].get_or_init(|| {
        let b = base();
        let list = match class {
            Class::Fnt => crate::sites::TEXT_FIT_PTRS,
            Class::Str => crate::sites::TEXT_FIT_STR_PTRS,
        };
        // SAFETY: the exe's own data section (pointer-sized vtable words).
        let bad = list.iter().find(|p| unsafe { rd::<usize>(b + p.rva as usize) } != b + p.target as usize);
        match bad {
            Some(p) => log(&format!("text fit: {} does not point at {:#x}; {} texts are not squeezed", p.name, 0x1_4000_0000u64 + p.target as u64, class.name())),
            None => log(&format!("text fit: Re:CoM's {} vtable checked (COL + 7 slots): long texts of ours in it squeeze to fit", class.name())),
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
/// draw sees it: [`width_of`].
pub unsafe fn width(fnt: usize) -> Option<i32> {
    if fnt == 0 {
        return None;
    }
    let raw = text_of_n(fnt, EXPAND_MAX);
    width_of(fnt, &raw)
}

/// Re:CoM's width of `text` (no NUL) in `fnt`'s font, size, spacing and flags (1/16 units), or None
/// (empty or too long, or Re:CoM's font not loaded). The draw first expands the button escapes with
/// `0x1403480C0` into its 2048-byte buffer (`0x14025AC2B`) and the width routine reads the raw text,
/// so the text is expanded the same way and measured on a copy of the object's `CFntM2` part (the
/// routine reads `+0x3C`, `+0x40`, `+0x44`, `+0x58` and writes nothing to it: Checked (decomp)); the
/// object itself is not touched.
pub unsafe fn width_of(fnt: usize, text: &[u8]) -> Option<i32> {
    if fnt == 0 || text.is_empty() || text.len() >= EXPAND_MAX {
        return None;
    }
    let mut src = text.to_vec();
    src.push(0);
    let mut expanded = vec![0u8; EXPAND_BUF];
    func::<ExpandFn>(EXPAND)(src.as_ptr(), expanded.as_mut_ptr());
    let mut copy = [0u64; FNTM2_SIZE / 8];
    std::ptr::copy_nonoverlapping(fnt as *const u8, copy.as_mut_ptr() as *mut u8, FNTM2_SIZE);
    let c = copy.as_mut_ptr() as usize;
    wr::<usize>(c + TEXT, expanded.as_ptr() as usize);
    let mut out = [0i32; 2];
    let lines = func::<WidthFn>(WIDTH)(out.as_mut_ptr(), c, std::ptr::null_mut(), std::ptr::null_mut());
    (lines > 0 && out[0] > 0).then_some(out[0])
}

/// The copy of Re:CoM's `class` vtable for `level` (made once), or None (no memory).
fn copy_for(class: Class, level: u32) -> Option<usize> {
    let slot = POOL[class.index()].get(level as usize)?;
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
    let vt = b + class.vtable() as usize;
    // SAFETY: Re:CoM's vtable words (checked by `ready`).
    let (col, recom) = unsafe { (rd::<u64>(vt - 8), std::array::from_fn(|i| rd::<u64>(vt + 8 * i))) };
    let words = vtable::copy(level, col, recom, draw as *const () as u64, measure as *const () as u64);
    let leaked: &'static mut [u64; vtable::WORDS] = Box::leak(Box::new(words));
    let vptr = leaked.as_ptr() as usize + 8 * vtable::VPTR_AT;
    slot.store(vptr, Ordering::Release);
    Some(vptr)
}

/// The class and level a text's vtable carries: Re:CoM's (level [`tf::ONE`]) or one of our copies.
unsafe fn level_of_vptr(vt: usize) -> Option<(Class, u32)> {
    for k in [Class::Fnt, Class::Str] {
        if vt == base() + k.vtable() as usize {
            return Some((k, tf::ONE));
        }
    }
    // A vtable pointer that is neither Re:CoM's nor one of ours: refused before reading.
    let ours = POOL.iter().flatten().any(|p| p.load(Ordering::Relaxed) == vt);
    if !ours {
        return None;
    }
    let class = Class::of_col(rd(vt - 8))?;
    vtable::level_of([rd(vt - 24), rd(vt - 16)]).map(|l| (class, l))
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
    if fnt == 0 {
        return tf::ONE;
    }
    let w = width(fnt);
    let level = w.map_or(tf::ONE, |w| tf::level(w, box_units));
    apply(fnt, level, w.map(|w| w as i64), (box_units * 16.0) as i64, None, what)
}

/// The true squeeze at `level` on `fnt` (a `C2DFnt` or a `CStrPrim`) for its next text `text` (no
/// NUL), set before Re:CoM's text call so its measure sees the drawn width; [`tf::ONE`]: Re:CoM's
/// vtable. Re:CoM setting any other text on the object later puts Re:CoM's vtable back ([`measure`]).
/// `width16` / `box16` for the log. The level set.
pub unsafe fn set_level(fnt: usize, level: u32, text: &[u8], width16: i64, box16: i64, what: impl FnOnce() -> String) -> u32 {
    if fnt == 0 {
        return tf::ONE;
    }
    apply(fnt, level, Some(width16), box16, Some(text), what)
}

unsafe fn apply(fnt: usize, level: u32, w: Option<i64>, box16: i64, owned: Option<&[u8]>, what: impl FnOnce() -> String) -> u32 {
    let vt: usize = rd(fnt);
    let Some((class, _)) = level_of_vptr(vt) else {
        note(|| format!("{}: not a C2DFnt / CStrPrim (vtable {vt:#x}); not squeezed", what()));
        return tf::ONE;
    };
    if !ready(class) {
        return tf::ONE;
    }
    let want = if level >= tf::ONE { Some(base() + class.vtable() as usize) } else { copy_for(class, level) };
    let Some(want) = want else {
        note(|| format!("{}: no memory for a vtable copy; not squeezed", what()));
        return tf::ONE;
    };
    if want != vt {
        wr::<usize>(fnt, want);
    }
    if let Ok(mut o) = OWNED.lock() {
        o.retain(|e| e.0 != fnt);
        if let (Some(t), true) = (owned, level < tf::ONE) {
            if o.len() >= OWNED_MAX {
                o.remove(0);
            }
            o.push((fnt, t.to_vec()));
        }
    }
    if level < tf::ONE {
        let t = owned.map(|t| t.to_vec()).unwrap_or_else(|| text_of(fnt));
        let first = LOGGED.lock().map(|mut l| {
            let k = (t.clone(), box16 as u32, level);
            let new = !l.contains(&k);
            if new && l.len() < 512 {
                l.push(k);
            }
            new
        });
        if first.unwrap_or(false) {
            let w = w.unwrap_or(0) as f32 / 16.0;
            note(|| format!("squeezed: {}: \"{}\" {w:.1} units wide, box {:.0}: x scale {:.3} ({level}/1024)", what(), shown(&t), box16 as f32 / 16.0, tf::factor(level)));
        }
    }
    level
}

/// Re:CoM's vtable back on `fnt` if one of our copies is on it (its own lines are never squeezed).
pub unsafe fn unsqueeze(fnt: usize) {
    if fnt == 0 {
        return;
    }
    if let Some((class, l)) = level_of_vptr(rd(fnt)) {
        if l < tf::ONE {
            wr::<usize>(fnt, base() + class.vtable() as usize);
        }
    }
    if let Ok(mut o) = OWNED.lock() {
        o.retain(|e| e.0 != fnt);
    }
}

/// Our vtables' `+0x18`: Re:CoM's draw, then the x of the vertices it wrote scaled about the
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

/// Our vtables' `+0x28`: Re:CoM's measure, its width × the scale (the drawn width). The line table
/// stays Re:CoM's (pen space: the draw reads such a table and our draw scales its result). On an
/// object squeezed by [`set_level`] whose text is no longer ours (Re:CoM's text call set its own:
/// it measures right after), Re:CoM's vtable goes back first and the width is Re:CoM's.
unsafe extern "win64" fn measure(fnt: usize, out: *mut i32, lines: *mut i32, count: *mut i32) -> i32 {
    let n = func::<MeasureFn>(MEASURE)(fnt, out, lines, count);
    let mut level = copy_level(rd(fnt));
    if level < tf::ONE {
        let foreign = OWNED.lock().ok().and_then(|o| o.iter().find(|e| e.0 == fnt).map(|e| e.1.clone())).is_some_and(|t| text_of_n(fnt, t.len() + 1) != t);
        if foreign {
            unsqueeze(fnt);
            level = tf::ONE;
        }
    }
    if n > 0 && !out.is_null() && level < tf::ONE {
        *out = tf::scaled(*out as i64, level) as i32;
    }
    n
}
