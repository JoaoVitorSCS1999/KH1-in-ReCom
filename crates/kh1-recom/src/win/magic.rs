//! KH1's cardless cast and Fire inside Re:CoM's own magic action (M7 stage 7.4; `crate::magic`,
//! `docs/recom-m7-magic.md`).
//!
//! - **Command:** the command menu's spell (and, in 7.12, a shortcut) calls [`issue`]; the next
//!   KH1 frame of Sora's (the same frame: the request-bits wrapper runs before the state exe)
//!   takes it as KH1's magic command.
//! - **Start** ([`start_cast`]): `initMagic`'s (`0x1402203D0`) own writes for a cast, cardless:
//!   Re:CoM's action `0x13`, its fields, `+0x3C8` → the KH1 row (Re:CoM's Fire / Fira / Firaga row
//!   with KH1's ball, `crate::magic::fire_row_bytes`). `initMagic` itself reads the played card
//!   (`+0x7F4` into `BattleCardList`, loaded only with a card battle, `0x1401607A0`), so it
//!   isn't called; the rest of what it does is done here with Re:CoM's own functions
//!   (`0x140209800`, `0x1401F68A0`), as M5's row pick does for swings.
//! - **Each frame** of the cast: Re:CoM's Battle exe calls `exeMagic` through Sora's vtable, the
//!   Field exe through a new case (`crate::magic::field_case`); our hook on `exeMagic` runs KH1's
//!   frame instead (as the swing exe's does): the cast motions, the windows, the script.
//! - **Release** ([`release`]): Re:CoM's spawn's Fire case (`0x140222170` case `0x22`) as a new
//!   KH1 case: the same Re:CoM calls (hand point `0x1401F0B30`, descriptor `0x140156790`, angles
//!   `0x1401A4A00`, factory `0x1401A8A80`, shot manager `0x140158DD0`, mover set-up
//!   `0x140157DA0`) with the KH1 row's numbers and KH1's life; then KH1's MP is paid
//!   (`super::mp::pay`, 30 units). Re:CoM's `CHomingBullet` then steps, homes, draws, collides
//!   and ends the ball.
//! - **Hits:** the enemy-hit wrapper ([`take_hit`]) recognises a hit part whose attacker is one of
//!   our balls and gives it KH1's spell damage (`super::stats::kh1_spell_hit`).
//! - **End** ([`end_cast`]): Re:CoM's `0x1401F6660`, as `exeMagic` ends its action.

use super::movement::{self as mv, func, orig, p, rd, rd_vec, wr};
use super::{base, log, va};
use crate::magic::{self as mg, CastEvent, CastStart, MagicCommand};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_EXE_MAGIC: AtomicUsize = AtomicUsize::new(0);

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The KH1 magic rows (`crate::magic::kh1_magic_table`). Never freed: `+0x3C8` points into it.
static TABLE: OnceLock<&'static [[u8; 16]]> = OnceLock::new();
/// The command issued this frame (the menu's), for the KH1 frame.
static PENDING: Mutex<Option<MagicCommand>> = Mutex::new(None);
/// Our balls: (Re:CoM shot id, spell, level), the last 16.
static SHOTS: Mutex<Vec<(u32, u8, u8)>> = Mutex::new(Vec::new());
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
static HIT_LINES: AtomicU32 = AtomicU32::new(40);
static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

// Re:CoM functions and data (RVAs; first bytes checked in `sites::MAGIC_*`).
const DESC_INIT: u32 = 0x0015_6790;
const HAND_POINT: u32 = 0x001F_0B30;
const ANGLES: u32 = 0x001A_4A00;
const FACTORY: u32 = 0x001A_8A80;
const SHOT_MANAGER: u32 = 0x0015_8DD0;
const SHOT_MOVER: u32 = 0x0015_7DA0;
const HOMING_TARGET: u32 = 0x001F_68A0;
const CURSOR_BUSY: u32 = 0x0020_9800;
const END_ACTION: u32 = 0x001F_6660;
const GROUND_TEST: u32 = 0x001F_3C00;
/// Re:CoM's base gravity (`initMagic` writes it to `+0x5D8`). Checked (decomp).
const BASE_GRAVITY: u64 = 0x1_4087_B398;
/// The effect sets (`0x1405A1730`, 0x60 bytes each; Fire 3, Fira 4, Firaga 5 hold `fire_all`).
const EFFECT_SETS: u64 = 0x1_405A_1730;
/// Re:CoM's attack rows.
const ROWS: u64 = crate::attack::RECOM_ATTACK_ROWS;

type PlFn = unsafe extern "win64" fn(usize);
type PlIntFn = unsafe extern "win64" fn(usize, i32);
type PlOutFn = unsafe extern "win64" fn(usize, *mut [f32; 4]);
type AnglesFn = unsafe extern "win64" fn(usize) -> *const [f32; 4];
type DescInitFn = unsafe extern "win64" fn(*mut u8, i32, i32);
type FactoryFn = unsafe extern "win64" fn() -> usize;
type ManagerFn = unsafe extern "win64" fn(usize, *const u8, usize) -> usize;
type MoverFn = unsafe extern "win64" fn(usize, f32, f32, i32, i32);
type GroundFn = unsafe extern "win64" fn(usize) -> u32;

/// The shot descriptor (`0x140156790` fills 0xE0 bytes; the bullet init reads up to `+0xD8`).
#[repr(C, align(16))]
struct Desc([u8; 0x100]);

/// `CPlayer` fields the cast writes (`initMagic` `0x1402203D0`; `docs/recom-player.md`).
/// Checked (decomp).
mod f {
    pub const ROW_PTR: usize = 0x3C8;
    pub const CURSOR: usize = 0x3F8;
    pub const PREV_ACTION: usize = 0x408;
    pub const ACTION: usize = 0x40C;
    pub const ACTION_TIME: usize = 0x410;
    pub const SUB: usize = 0x414;
    pub const FLAGS_418: usize = 0x418;
    pub const REQUEST: usize = 0x424;
    pub const CARD: usize = 0x428;
    pub const ATTACK_STAT: usize = 0x43C;
    pub const CLIP_OFFSET: usize = 0x478;
    pub const SPEED_4DC: usize = 0x4DC;
    pub const VSPEED: usize = 0x5D4;
    pub const GRAVITY: usize = 0x5D8;
    pub const PHASE: usize = 0x604;
    pub const SITUATION: usize = 0x630;
    pub const PREV_SITUATION: usize = 0x634;
    pub const ATTACK_ID: usize = 0x638;
    pub const COUNTER: usize = 0x63C;
    pub const HIT: usize = 0x644;
    pub const CHAIN: usize = 0x64C;
    pub const RECOVERY: usize = 0x65C;
    pub const AP: usize = 0x660;
    pub const KIND: usize = 0x668;
    pub const ATTR: usize = 0x66C;
    pub const SLEIGHT_670: usize = 0x670;
    pub const CARD_7B4: usize = 0x7B4;
    pub const TARGET: usize = 0x84C;
}

/// Re:CoM's fire attribute (the Fire card's BattleCardList `+0xB` = 2; research `m7-magic.md`
/// § 3.9, Checked (files)).
const RECOM_ATTR_FIRE: u32 = 2;

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic: {}", line()));
    }
}

fn event(e: String) {
    if let Ok(mut g) = EVENTS.lock() {
        if g.len() < 16 {
            g.push(e);
        }
    }
}

/// Builds the KH1 rows from Re:CoM's live Fire rows (which must be the Steam exe's) and patches
/// the Field exe's case for action `0x13`. The exeMagic hook is installed by the caller after.
pub unsafe fn install(b: usize) -> bool {
    let mut live = [[0u8; 16]; 3];
    for (r, id) in live.iter_mut().zip([0x22usize, 0x29, 0x2A]) {
        *r = rd(va(b, ROWS) + 16 * id);
    }
    if live != mg::RECOM_FIRE_ROWS {
        log("magic: Re:CoM's Fire rows 0x22 / 0x29 / 0x2A are not the Steam exe's; no KH1 cast");
        return false;
    }
    let t: &'static [[u8; 16]] = Box::leak(mg::kh1_magic_table(&live).into_boxed_slice());
    if TABLE.set(t).is_err() {
        return false;
    }
    use crate::magic::field_case as fc;
    let cave = b + fc::CAVE_RVA as usize;
    let entry = b + fc::TABLE_ENTRY_RVA as usize;
    let old_entry = fc::DEFAULT_RVA.to_le_bytes();
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    if have(cave, fc::STUB_LEN) != [0xCC; fc::STUB_LEN] || have(entry, 4) != old_entry {
        log("magic: the Field exe's case 0x13 or its cave is not as expected; no KH1 cast");
        return false;
    }
    if !super::hook::patch(cave, &[0xCC; fc::STUB_LEN], &fc::stub(), "magic: Field case stub") {
        return false;
    }
    if !super::hook::patch(entry, &old_entry, &fc::entry(), "magic: Field jump table 0x13") {
        return false;
    }
    true
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

/// A KH1 magic command from the command menu (and the shortcuts, stage 7.12): taken by Sora's
/// KH1 frame this frame, if it runs (else dropped at the next frame's start, as KH1 drops a
/// command Sora can't take).
pub fn issue(spell: u8, level: u8) {
    if !INSTALLED.load(Ordering::Acquire) {
        note(|| format!("{}{level}: no KH1 cast installed", mg::spell::NAMES.get(spell as usize).copied().unwrap_or("?")));
        return;
    }
    if let Ok(mut g) = PENDING.lock() {
        *g = Some(MagicCommand { spell, level });
    }
}

/// The start of Sora's frame (the request-bits wrapper, before the menu): a command no KH1 frame
/// took is gone.
pub fn new_frame() {
    if let Ok(mut g) = PENDING.try_lock() {
        if let Some(m) = g.take() {
            note(|| format!("{}{}: dropped (no KH1 frame ran: Re:CoM's action)", mg::spell::NAMES.get(m.spell as usize).copied().unwrap_or("?"), m.level));
        }
    }
}

/// The command for this KH1 frame (`win::movement::run_frame`).
pub fn take_command() -> Option<MagicCommand> {
    PENDING.try_lock().ok().and_then(|mut g| g.take())
}

/// The KH1 row `+0x3C8` points at, if it points into our magic table: a KH1 cast.
pub unsafe fn kh1_cast(pl: usize) -> bool {
    let Some(t) = TABLE.get() else { return false };
    let ptr: usize = rd(pl + f::ROW_PTR);
    let start = t.as_ptr() as usize;
    ptr >= start && ptr < start + t.len() * 16
}

fn row(s: u8, level: u8) -> Option<&'static [u8; 16]> {
    let t = TABLE.get()?;
    t.get(mg::table_index(s, level)?).filter(|r| r[0] != 0)
}

/// KH1's usable test (`0x14028EFD0`): the spell is learned (the menu offers only learned ones),
/// and MP pays its cost (`0x1402A4C10`).
pub fn can_cast(s: u8, level: u8) -> bool {
    row(s, level).is_some() && s == mg::spell::FIRE && super::mp::can_pay(mg::fire::COST)
}

/// `initMagic`'s writes for a KH1 cast, cardless (Checked (decomp `0x1402203D0`), in its order):
/// the action, the attack fields, the target cursor's busy bit, the homing target, the KH1 row
/// and its AP / kind / attribute. Not written: the air hover (`+0x418 |= 0x2800000`, gravity ×
/// 0.16: KH1's air cast hops instead) and the card modifier (no card). The clip is KH1's motion
/// request, played by the KH1 frame.
pub unsafe fn start_cast(pl: usize, c: &CastStart) -> bool {
    let Some(r) = row(c.spell, c.level) else { return false };
    let cur: u32 = rd(pl + f::ACTION);
    wr(pl + f::PREV_ACTION, cur);
    wr(pl + f::ACTION_TIME, 0u64);
    wr(pl + f::ACTION, mg::RECOM_MAGIC_ACTION);
    wr(pl + f::CARD, rd::<u32>(pl + f::CARD) & 0xFFF7_FEF7);
    wr(pl + f::REQUEST, rd::<u32>(pl + f::REQUEST) & !0x20);
    wr(pl + f::HIT, 0u64);
    wr(pl + f::CHAIN, 0u64);
    wr(pl + f::CARD_7B4, 0u32);
    wr(pl + f::COUNTER, 0u32);
    // `0x1401A38D0`: the body's velocity to zero (the KH1 frame writes its own after).
    let body: usize = rd(pl + p::BODY);
    if body != 0 {
        wr(body + 0x80, [0f32, 0.0, 0.0, 1.0]);
    }
    wr(pl + f::SITUATION, 0u32);
    wr(pl + f::FLAGS_418, rd::<u32>(pl + f::FLAGS_418) & 0xFD7F_FFFF);
    wr(pl + f::GRAVITY, rd::<f32>(va(base(), BASE_GRAVITY)));
    wr(pl + f::PHASE, mg::MAGIC_PHASE);
    wr(pl + f::SPEED_4DC, 0u64);
    wr(pl + f::VSPEED, 0f32);
    let cursor: usize = rd(pl + f::CURSOR);
    if cursor != 0 {
        func::<PlIntFn>(CURSOR_BUSY)(cursor, 8);
    }
    func::<PlFn>(HOMING_TARGET)(pl);
    wr(pl + f::RECOVERY, 0u32);
    // `+0x478` picks the air clips (+4) for Re:CoM's own readers; KH1's motion requests choose
    // the clip here.
    let air = func::<GroundFn>(GROUND_TEST)(pl) != 0;
    wr(pl + f::CLIP_OFFSET, if air { 4u32 } else { 0 });
    let id = mg::recom_attack_id(c.spell);
    wr(pl + f::ATTACK_ID, id);
    wr(pl + f::ROW_PTR, r.as_ptr() as usize);
    // Re:CoM's AP for a target without KH1 numbers: the cardless AP (the attack stat), as M5's
    // swings (KH1's damage replaces it on twins and proxies, `take_hit`).
    wr(pl + f::AP, rd::<i32>(pl + f::ATTACK_STAT));
    wr(pl + f::KIND, r[0xB] as u32);
    wr(pl + f::ATTR, RECOM_ATTR_FIRE);
    wr(pl + f::PREV_SITUATION, 0u32);
    wr(pl + f::SUB, id);
    wr(pl + f::SLEIGHT_670, 0u64);
    wr(pl + f::FLAGS_418, rd::<u32>(pl + f::FLAGS_418) | 0x10_0000);
    wr(pl + f::ACTION_TIME, 0u32);
    true
}

/// KH1's release: Re:CoM's spawn's Fire case (`0x140222170` case `0x22`, Checked (disasm
/// `0x14022220B..0x14022234D`)) with the KH1 row and KH1's life, then KH1's MP payment. The
/// shot's id, if Re:CoM made one.
pub unsafe fn release(pl: usize, c: &CastStart) -> Option<u32> {
    let r = row(c.spell, c.level)?;
    let n = mg::shot_numbers(r);
    let body: usize = rd(pl + p::BODY);
    let mut hand = [0f32; 4];
    if body != 0 {
        let b = rd_vec(body + 0x30);
        hand = [b.x, b.y, b.z, 1.0];
    }
    func::<PlOutFn>(HAND_POINT)(pl, &mut hand);
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    // Class 2 = CHomingBullet, 3 = the descriptor kind Re:CoM's spells pass.
    func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
    let at = |o: usize| dp.add(o);
    std::ptr::write_unaligned(at(0x68) as *mut u32, rd::<u32>(pl + f::KIND));
    std::ptr::write_unaligned(at(0x6C) as *mut u32, rd::<u32>(pl + f::ATTR));
    std::ptr::write_unaligned(at(0x70) as *mut i32, rd::<i32>(pl + f::AP));
    std::ptr::write_unaligned(at(0x0C) as *mut u32, 0x8000_0000);
    std::ptr::write_unaligned(at(0x60) as *mut u64, n.flags);
    std::ptr::write_unaligned(at(0x30) as *mut f32, n.speed);
    std::ptr::write_unaligned(at(0x3C) as *mut i32, n.life);
    std::ptr::write_unaligned(at(0x34) as *mut f32, n.range);
    std::ptr::write_unaligned(at(0x44) as *mut i32, rd::<i32>(pl + f::TARGET));
    std::ptr::write_unaligned(at(0x48) as *mut f32, n.gate);
    std::ptr::write_unaligned(at(0x4C) as *mut f32, n.turn);
    std::ptr::write_unaligned(at(0x10) as *mut [f32; 4], [hand[0], hand[1], hand[2], 0.5]);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    if !ang.is_null() {
        std::ptr::write_unaligned(at(0x20) as *mut [f32; 4], std::ptr::read_unaligned(ang));
    }
    std::ptr::write_unaligned(at(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    std::ptr::write_unaligned(at(0x78) as *mut f32, n.radius);
    std::ptr::write_unaligned(at(0x94) as *mut i32, n.fade);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        note(|| "release: no entity factory: no shot".into());
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        note(|| "release: Re:CoM's shot manager made no shot (no MP paid)".into());
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    let id: u32 = rd(bullet + 0x20);
    if let Ok(mut g) = SHOTS.lock() {
        if g.len() >= 16 {
            g.remove(0);
        }
        g.push((id, c.spell, c.level));
    }
    // KH1 pays after the launch (`xm_fire01.mag`: sys 20, then sys 47).
    super::mp::pay(mg::fire::COST, "Fire release");
    note(|| {
        format!(
            "release {}{}: shot id {id} at ({:.2},{:.2},{:.2}), speed {:.2} m/f, life {} ticks, turn {}°/f, radius {:.2} m, target {}",
            mg::spell::NAMES[c.spell as usize],
            c.level,
            hand[0],
            hand[1],
            hand[2],
            n.speed,
            n.life,
            n.turn,
            n.radius,
            rd::<i32>(pl + f::TARGET)
        )
    });
    Some(id)
}

/// Re:CoM ends its magic action (`0x1401F6660(player, airborne)`, as `exeMagic` does at its end).
pub unsafe fn end_cast(pl: usize, airborne: bool) {
    func::<PlIntFn>(END_ACTION)(pl, airborne as i32);
}

/// The frame's cast events (from `win::movement::run_frame`): log and trace.
pub fn on_events(ev: &[CastEvent]) {
    for e in ev {
        let t = e.token();
        event(t.clone());
        note(|| t);
    }
}

/// `CPlayerSora::exeMagic` (action `0x13`; the Battle exe through Sora's vtable, the Field exe
/// through our case): KH1's frame for a KH1 cast, Re:CoM's exe otherwise.
pub unsafe extern "win64" fn exe_magic(pl: usize) {
    let o: PlFn = orig(&ORIG_EXE_MAGIC);
    if !mv::kh1_sora(pl) || !kh1_cast(pl) {
        return o(pl);
    }
    if !mv::continues(pl) {
        // Re:CoM ran something else in between (our cast isn't what it ran): end its action and
        // take over from Re:CoM's state; the cast is dropped (no spell, no MP if not released).
        end_cast(pl, func::<GroundFn>(GROUND_TEST)(pl) != 0);
        return mv::run_frame(pl, true);
    }
    mv::run_frame(pl, false);
}

/// One of our balls hit this enemy this frame: the hit part (`*(enemy+0xF8)+0x40`, `0x120` bytes
/// each, hit flag `+0x90`, attacker id `+0x98`; Checked (decomp `0x140154EB0`, `0x14016F1C0`)),
/// the spell and level.
pub unsafe fn take_hit(enemy: usize) -> Option<(usize, u8, u8)> {
    let info: usize = rd(enemy + 0xF8);
    if info == 0 {
        return None;
    }
    let g = SHOTS.lock().ok()?;
    if g.is_empty() {
        return None;
    }
    let parts: usize = rd(info + 0x40);
    let n: i32 = rd(info + 0x38);
    for i in 0..n.clamp(0, 32) as usize {
        let part = parts + i * 0x120;
        if rd::<i16>(part + 0x90) == 0 {
            continue;
        }
        let by: u32 = rd(part + 0x98);
        if let Some(&(_, s, l)) = g.iter().find(|&&(id, _, _)| id == by) {
            return Some((part, s, l));
        }
    }
    None
}

/// The hit part's attribute (`+0x84`), which Re:CoM's hit handler reads for its own resistance
/// (`0x14017F440`: attributes 0..4 from the enemy's `@ENP`, 100 % otherwise). Checked (decomp
/// `0x14017F7E0`).
pub const PART_ATTR: usize = 0x84;
/// An attribute Re:CoM's resistance table doesn't cover (100 %): KH1's resistance is already in
/// the KH1 damage.
pub const NEUTRAL_ATTR: u32 = 0xFF;

/// A logged spell hit (capped), for the log and the trace's hits column.
pub fn hit_line(line: impl FnOnce() -> String) {
    if HIT_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic: {}", line()));
    }
}

/// The F9 trace's cast token: the cast in progress and this frame's events.
pub fn trace_token() -> String {
    let ev = EVENTS.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
    let mut t = match mv::SORA.try_lock() {
        Ok(g) => mg::trace_token(g.as_ref().and_then(|s| s.sora.cast.as_ref())),
        Err(_) => "cast ?".into(),
    };
    for e in ev {
        t.push(' ');
        t.push_str(&e);
    }
    t
}

/// Clears the trace's events (a new recording starts).
pub fn clear_events() {
    if let Ok(mut g) = EVENTS.lock() {
        g.clear();
    }
}

/// The start log line.
pub fn start_line() -> String {
    format!(
        "magic: KH1's cardless cast at combat = kh1 in Sora's story (M7 stage 7.4): Fire / Fira / Firaga on Re:CoM's magic action 0x13 with KH1 rows ({} rows), KH1's release ({} frames after the wind-up / {} on the fast path), KH1's ball (speed {} cm/f, life {} f, turn {}°/f, radius 50 cm) through Re:CoM's CHomingBullet, MP {} units paid at the release; Field exe case 0x13 -> exeMagic",
        TABLE.get().map_or(0, |t| t.iter().filter(|r| r[0] != 0).count()),
        28,
        mg::fire::FAST_WAIT,
        mg::fire::SPEED,
        mg::fire::LIFE,
        mg::fire::TURN_DEG,
        mg::fire::COST
    )
}
