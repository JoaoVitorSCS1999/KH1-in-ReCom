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
//! - **Re-target** (7.4b, decision 36): a ball cast with no target gets KH1's pick each frame
//!   ([`seek_on`], [`kh1_ball_exe`]): its own flight state's exe (Re:CoM's `CStateTemplate<CBullet>`
//!   at `bullet+0x2B0`, exe `+0x2D0` = the vcall thunk `0x1401560B8` → `CHomingBullet`'s homing
//!   step `0x1401575D0`) points at our wrapper, which writes KH1's pick (`crate::magic::retarget`)
//!   into Re:CoM's homing target `+0x2F4` and runs Re:CoM's step. Re:CoM's own shots never pass
//!   through it.
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
/// Our shots: (Re:CoM shot id, spell, level, the cast's shared hit serial or 0), the last 32 (a
/// Blizzard makes 7).
static SHOTS: Mutex<Vec<(u32, u8, u8, u32)>> = Mutex::new(Vec::new());
/// Our Blizzard shards (7.5): (shot id, its vertical step dy, m per frame), the last 32.
static SHARDS: Mutex<Vec<(u32, f32)>> = Mutex::new(Vec::new());
/// Thunder (7.6): what the command saw (the target's handle if it was an alive enemy, and the area
/// centre then), for the release.
static THUNDER_CMD: Mutex<Option<(u32, [f32; 3])>> = Mutex::new(None);
/// Thunder's strike threads while they run (KH1 runs each cast's thread to its end, so a second
/// Thunder's runs beside the first's; at most 8), and a flag the per-frame check reads.
static STRIKES: Mutex<Vec<ThunderRun>> = Mutex::new(Vec::new());
/// Any spell thread is running (Thunder's strikes, Gravity's falls): the per-frame check reads it.
static STRIKING: AtomicBool = AtomicBool::new(false);
/// Gravity (7.8): what the command saw (the target's handle if it was an alive enemy, and the
/// offset along Sora's facing used without one), for the release.
static GRAVITY_CMD: Mutex<Option<(u32, [f32; 3])>> = Mutex::new(None);
/// Gravity's falling threads while they run (at most 8).
static FALLS: Mutex<Vec<GravityRun>> = Mutex::new(Vec::new());

/// A Gravity cast's sphere in its fall.
struct GravityRun {
    fall: mg::Fall,
    level: u8,
    /// Re:CoM's sphere (its handle / shot id), and the impact's centre (the marker point + 0.1 m).
    shot: u32,
    centre: [f32; 3],
    /// The sphere's effect scale at its spawn (Re:CoM's), the base of KH1's growth.
    base_scale: Option<f32>,
    kind: u32,
    attr: u32,
    ap: i32,
}

/// A Thunder (7.6) or Stop (7.9) cast's strikes in progress.
struct ThunderRun {
    s: mg::Strikes,
    /// Thunder or Stop, and the area its bodies test (KH1's sys 90).
    spell: u8,
    area: mg::Area,
    level: u8,
    centre: [f32; 3],
    /// Re:CoM's kind / attribute / AP of the cast (the magic action's, at the release).
    kind: u32,
    attr: u32,
    ap: i32,
}
/// KH1's re-target (7.4b) is set up: the homing slot and Re:CoM's lock-on list are as expected.
static RETARGET: AtomicBool = AtomicBool::new(false);
/// Our balls cast with no target: (shot id, the last pick's handle, 0 for none), the last 16.
static SEEKERS: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());
static RETARGET_LINES: AtomicU32 = AtomicU32::new(40);
static BOLT_LINES: AtomicU32 = AtomicU32::new(60);
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
/// The vcall thunk `mov rax,[rcx]; jmp [rax+0xA8]` that the bullet init (`0x140157E60`, at
/// `0x1401585D5` / `0x140158609`) stores as every bullet's flight-state exe; for a
/// `CHomingBullet` it reaches the homing step `0x1401575D0`. Checked (disasm).
const HOMING_THUNK: u32 = 0x0015_60B8;
/// `0x140156770(bullet)`: its attack's hit serial (`coll+0x70`), which Re:CoM's Blizzard case passes
/// to its other shards (`desc+0x74`). The apply `0x14016F1C0` skips a body whose ring holds the
/// serial (`0x14016F960`) and records it on the body it hits (`0x14016FE20`); the enemy's hit
/// handler (`0x140154EB0`) then records it on all the enemy's bodies and takes one hit body per
/// run. So with one serial, a cast hits an enemy once, as KH1's shared hit bit. Checked (decomp
/// + exe).
const SHOT_SERIAL: u32 = 0x0015_6770;
/// Re:CoM's Blizzard side-shard effect sets (`0x1405A1790` = the row's set + 1). Checked (decomp
/// `0x140222170` case `0x23`).
const EFFECT_SETS_NEXT: u64 = 0x1_405A_1790;
/// `0x1401A6B20(actor, effect name, id, &position, &angles, 0.0)`: Re:CoM's Thunder case plays its
/// `_thunder_all` effects with it (`0x1402227A2`, `0x14022282C`). Checked (disasm).
const EFFECT_AT: u32 = 0x001A_6B20;
/// Re:CoM's Thunder bolt (`0x140222170` case `0x24`, Checked (disasm `0x140222748..0x14022293C`)):
/// the bolt effect `set − 10` at the target's x / z and Sora's height (`+0x504`) − 6 m, then a
/// `CBullet` (flags `0x47`, life 800 ticks) from Sora's height there, pitched straight down
/// (π/2), its radius `row[4]` × 0.1.
const THUNDER_DROP: f32 = 6.0;
const THUNDER_BOLT_FLAGS: u64 = 0x47;
const THUNDER_BOLT_LIFE: i32 = 800;
/// Re:CoM's own "collision held off" for a shot (flag `0x200000` with `desc+0x8C` ticks: the init
/// `0x140157E60` switches its attack off, `0x1401573C0` back on when the count runs out): our bolts
/// keep it off for good; their one hit is the direct strike. Checked (decomp).
const COLLISION_HELD: u64 = 0x20_0000;
const HELD_TICKS: i32 = 1_000_000;
/// The player's position vector (`+0x500`, its y read by the Thunder case). Checked (disasm).
const PLAYER_POS: usize = 0x500;
/// `CCollAttack`'s "off" (vtable +8: `+0x10` = 0) and apply (vtable +0x38). Checked (exe + decomp).
const COLL_OFF: u32 = 0x0014_DE80;
const COLL_APPLY: u32 = 0x0016_F1C0;
/// Re:CoM's `LockOnManager` (`0x14087B1E0`, made on first use by its users; its list `+0x88`:
/// nodes `{u32 handle, …, +0x10 next}`, the last one a sentinel with no next). Checked (decomp
/// + disasm `0x1401CDB90`, Re:CoM's own nearest-target pick for homing shots).
const LOCK_ON_MANAGER: u64 = 0x1_4087_B1E0;

/// Bullet and lock-on fields the re-target reads or writes. Checked (decomp `0x140155EB0`,
/// `0x140157E60`, `0x1401575D0`, `0x140156740`, `0x1401CDB90`).
mod rt {
    /// The flight state's exe member-function pointer (`CStateTemplate<CBullet>` at `+0x2B0`,
    /// its exe `+0x20` / this-adjust `+0x28`, run by `0x140156740` each frame).
    pub const STATE_EXE: usize = 0x2D0;
    pub const STATE_EXE_ADJUST: usize = 0x2D8;
    /// The homing step returns at once while this is set.
    pub const HOLD: usize = 0x2F0;
    /// The life (ticks), counted down by Re:CoM.
    pub const LIFE: usize = 0x160;
    pub const LIST: usize = 0x88;
    pub const NODE_NEXT: usize = 0x10;
    /// The lock-on flag every entry of Re:CoM's own pick must have (`+0x154 & 0x10000`).
    pub const ACTOR_FLAGS: usize = 0x154;
    pub const LOCKABLE: u32 = 0x1_0000;
    /// Actor position (its mover's `+0x30`).
    pub const MOVER: usize = 0xF0;
    pub const POSITION: usize = 0x30;
    /// A mover's collision object and its test parameter (`win::movement::body`).
    pub const MOVER_COLL: usize = 0x158;
    pub const MOVER_COLL_PARAM: usize = 0x134;
}

type PlFn = unsafe extern "win64" fn(usize);
type PlIntFn = unsafe extern "win64" fn(usize, i32);
type PlOutFn = unsafe extern "win64" fn(usize, *mut [f32; 4]);
type AnglesFn = unsafe extern "win64" fn(usize) -> *const [f32; 4];
type DescInitFn = unsafe extern "win64" fn(*mut u8, i32, i32);
type FactoryFn = unsafe extern "win64" fn() -> usize;
type ManagerFn = unsafe extern "win64" fn(usize, *const u8, usize) -> usize;
type MoverFn = unsafe extern "win64" fn(usize, f32, f32, i32, i32);
type GroundFn = unsafe extern "win64" fn(usize) -> u32;
type SerialFn = unsafe extern "win64" fn(usize) -> u32;

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

/// Re:CoM's attributes of the spells (the magic cards' BattleCardList `+0xB`: Fire 2, Blizzard 3,
/// Thunder 4, Cure 6, Gravity 5, Stop 5, Aero 1; research `m7-magic.md` § 3.9, Checked (files)).
/// Re:CoM's own resistance reads them for enemies without KH1 numbers.
const RECOM_ATTR: [u32; 7] = [2, 3, 4, 6, 5, 5, 1];

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
    let read = |ids: [usize; 3]| {
        let mut live = [[0u8; 16]; 3];
        for (r, id) in live.iter_mut().zip(ids) {
            *r = rd(va(b, ROWS) + 16 * id);
        }
        live
    };
    // Each built spell's Re:CoM rows (its three levels), which must be the Steam exe's.
    let want = mg::recom_rows();
    let mut live = Vec::with_capacity(want.len());
    for (s, rows) in want.iter().enumerate() {
        let ids = [1u8, 2, 3].map(|lv| mg::recom_base_row(s as u8, lv).unwrap_or(0));
        let got = read(ids);
        if got != *rows {
            log(&format!("magic: Re:CoM's {} rows {:#x} / {:#x} / {:#x} are not the Steam exe's; no KH1 cast", mg::spell::NAMES[s], ids[0], ids[1], ids[2]));
            return false;
        }
        live.push(got);
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
    // 7.4b: KH1's re-target needs CHomingBullet's homing slot to be Re:CoM's homing step (the
    // thunk our wrapper calls goes through it). Without it the balls home as Re:CoM's.
    let slot_ok = crate::sites::MAGIC_RETARGET_PTRS.iter().all(|p| rd::<usize>(b + p.rva as usize) == b + p.target as usize);
    RETARGET.store(slot_ok, Ordering::Release);
    if !slot_ok {
        log("magic: CHomingBullet's homing slot is not 0x1401575D0; Fire balls cast with no target fly as Re:CoM's (no KH1 re-target)");
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
    row(s, level).is_some() && mg::castable(s) && super::mp::can_pay(mg::cost(s))
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
    if c.spell == mg::spell::THUNDER {
        thunder_command(pl);
    }
    if c.spell == mg::spell::CURE {
        // `initMagic` case 0x1D's charge: the set's effect id 0 on the Keyblade's bone 2.
        charge_on_weapon(pl, r, 0);
    }
    if c.spell == mg::spell::STOP {
        // `initMagic` case 0x1F's charge: the set's effect id 0 on Sora's bone 0 at (0, 0.85, 0),
        // kept on him; and the command's target for KH1's centre.
        effect_on_bone(pl, pl, 0, set_name(r, 0), 0, &BODY_OFFSET, true);
        if let Ok(mut g) = STOP_CMD.lock() {
            *g = Some(rd(pl + f::TARGET));
        }
    }
    if c.spell == mg::spell::GRAVITY {
        // `initMagic` case 0x1E's charge (the set's `+0x30` entry) and KH1's marker point.
        charge_on_weapon(pl, r, 0x18);
        gravity_command(pl, r);
    }
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
    let ap = match c.spell {
        // Cure: Re:CoM's heal (decision 34), its AP rule for the card / sleight (power / 10).
        mg::spell::CURE => mg::cure::heal(c.level),
        _ => rd::<i32>(pl + f::ATTACK_STAT),
    };
    wr(pl + f::AP, ap);
    wr(pl + f::KIND, r[0xB] as u32);
    wr(pl + f::ATTR, RECOM_ATTR[c.spell as usize % 7]);
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
    match c.spell {
        mg::spell::FIRE => release_fire(pl, c),
        mg::spell::BLIZZARD => release_blizzard(pl, c),
        mg::spell::THUNDER => release_thunder(pl, c),
        mg::spell::CURE => release_cure(pl, c),
        mg::spell::GRAVITY => release_gravity(pl, c),
        mg::spell::STOP => release_stop(pl, c),
        _ => None,
    }
}

/// Re:CoM's Stop shot flags with a target (`mov qword [rbp−0x50],0x25`). Checked (disasm
/// `0x140222BEC`).
const STOP_FLAGS: u64 = 0x25;
/// Stop (7.9): the command's target handle, for the release.
static STOP_CMD: Mutex<Option<u32>> = Mutex::new(None);

/// KH1's Stop release (7.9, `xm_stop01.mag`): the centre (the target's aim point if it is alive,
/// else Sora's position), Re:CoM's lock-on enemies sorted nearest first from it, the strike thread
/// (one enemy inside the 2.5 / 3.5 / 5 m, ±3 m cylinder every 4 frames, [`stop_strike`]), the
/// first run now, then the MP (two points). The first strike's shot id.
unsafe fn release_stop(pl: usize, c: &CastStart) -> Option<u32> {
    let h = STOP_CMD.lock().ok().and_then(|mut g| g.take()).unwrap_or(0);
    let centre = match alive_enemy(h) {
        Some(e) => aim_point(e),
        None => sora_pos_dir(pl).0,
    };
    let enemies: Vec<(u32, [f32; 3])> = lock_on_list(0, false).into_iter().filter_map(|(h, _)| Some((h, position(alive_enemy(h)?)?))).collect();
    let order = mg::thunder_order(centre, enemies);
    let n = order.len();
    let run = ThunderRun { s: mg::Strikes::every(order, mg::stop::INTERVAL), spell: mg::spell::STOP, area: mg::stop::area(c.level), level: c.level, centre, kind: rd(pl + f::KIND), attr: rd(pl + f::ATTR), ap: rd(pl + f::AP) };
    let first = match STRIKES.lock() {
        Ok(mut g) => {
            if g.len() >= 8 {
                g.remove(0);
            }
            g.push(run);
            STRIKING.store(true, Ordering::Release);
            let i = g.len() - 1;
            drop(g);
            strike_step(pl, 0.0, Some(i)).into_iter().next()
        }
        Err(_) => None,
    };
    // KH1 pays after the thread starts (sys 20, then sys 47).
    super::mp::pay(mg::stop::COST, "Stop release");
    note(|| format!("release stop{}: centre ({:.2},{:.2},{:.2}) r {} m ±3 m, {n} enemies listed, first {first:?}", c.level, centre[0], centre[1], centre[2], mg::stop::RADIUS[c.level.clamp(1, 3) as usize - 1] / 100.0));
    first
}

/// One Stop strike (KH1's sys 60 with the Stop data): **Re:CoM's Stop case for this enemy**
/// (`0x140222170` case `0x27` with a target, Checked (disasm `0x140222BBA..0x140222CB0`)): its
/// `CBullet` (flags `0x25`, speed 0, the row's range / life / radius × 0.1, its `stop_all` set) at the
/// enemy's aim point, its attack held off; then Re:CoM's apply onto the enemy's first live hurt
/// body (7.6's mechanism). Re:CoM's hit handler then runs its kind 8 (the stop state) with KH1's
/// duration (`spell_hit`). The shot's id.
#[allow(clippy::too_many_arguments)]
unsafe fn stop_strike(pl: usize, enemy: usize, h: u32, level: u8, kind: u32, attr: u32, ap: i32) -> Option<u32> {
    let r = row(mg::spell::STOP, level)?;
    let aim = aim_point(enemy);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    let ang: [f32; 4] = if ang.is_null() { [0.0; 4] } else { std::ptr::read_unaligned(ang) };
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 1, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x68) as *mut u32, kind);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x0C) as *mut u32, 0x8000_0000);
    std::ptr::write_unaligned(w(0x60) as *mut u64, STOP_FLAGS | COLLISION_HELD);
    std::ptr::write_unaligned(w(0x8C) as *mut i32, HELD_TICKS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, 0.0);
    std::ptr::write_unaligned(w(0x34) as *mut f32, r[0xC] as f32);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, r[0xE] as i32 * 100);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [aim[0], aim[1], aim[2], 1.0]);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], ang);
    std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    std::ptr::write_unaligned(w(0x78) as *mut f32, r[4] as f32 * 0.1);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        bolt_line(|| format!("stop on {h:#x}: Re:CoM made no shot"));
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    hold_off(bullet);
    let id: u32 = rd(bullet + 0x20);
    remember_shot(id, mg::spell::STOP, level, 0);
    let res = apply_to(bullet, enemy);
    bolt_line(|| format!("stop on {h:#x}: shot {id}, Re:CoM's apply -> {res:?}"));
    Some(id)
}

/// Re:CoM's enemy fields Stop reads or writes (Checked (decomp `0x14017DC50`, `0x14017D260`,
/// `0x14017C740`, `0x1401843A0`)): the param block (`+0x308`; its `+0x2C` is Re:CoM's Stop gate),
/// the next state (`+0x68`), the stop state (`+0x940`), the state flags (`+0x260`, bit 19 while
/// stopped).
pub mod enemy_stop {
    pub const PARAM: usize = 0x308;
    pub const STOP_GATE: usize = 0x2C;
    pub const NEXT_STATE: usize = 0x68;
    pub const STOP_STATE: usize = 0x940;
    pub const FLAGS: usize = 0x260;
    pub const STOPPED: u64 = 0x8_0000;
}

/// `0x1401A2D90(enemy, -1)`: every attack body of the enemy off (`+0x94` = 0), as Re:CoM's own
/// reaction states do (`0x14017C350`). Checked (decomp).
const ATTACK_BODIES_OFF: u32 = 0x001A_2D90;

/// The stopped enemy's own attack bodies off (KH1 skips the attacks a stopped actor owns,
/// `0x1402B47A0`): once, when Re:CoM's hit handler has just sent it to its stop state.
pub unsafe fn stop_attacks_off(enemy: usize) -> bool {
    use enemy_stop::*;
    if rd::<usize>(enemy + NEXT_STATE) != enemy + STOP_STATE {
        return false;
    }
    func::<PlIntFn>(ATTACK_BODIES_OFF)(enemy, -1);
    true
}

/// Re:CoM's Stop gate (param `+0x2C`) for one hit: KH1's `resist[9]` decides instead (open when it
/// is above 0). The old value, to put back.
pub unsafe fn stop_gate(enemy: usize, open: bool) -> Option<(usize, i32)> {
    use enemy_stop::*;
    let param: usize = rd(enemy + PARAM);
    if param == 0 {
        return None;
    }
    let old: i32 = rd(param + STOP_GATE);
    wr(param + STOP_GATE, open as i32);
    Some((param, old))
}

/// Puts Re:CoM's Stop gate back.
pub unsafe fn stop_gate_back(g: Option<(usize, i32)>) {
    if let Some((param, old)) = g {
        wr(param + enemy_stop::STOP_GATE, old);
    }
}

/// Whether Re:CoM's stop state holds this enemy (its hits are held, `0x1401843A0`).
pub unsafe fn stopped(enemy: usize) -> bool {
    rd::<u64>(enemy + enemy_stop::FLAGS) & enemy_stop::STOPPED != 0
}

/// Re:CoM's Gravity case's descriptor `+0xB8` (`lea rax,[0x1407C78D0]`). Checked (disasm
/// `0x140222B09`).
const GRAVITY_DESC_B8: u64 = 0x1_407C_78D0;
/// Re:CoM's Gravity sphere flags (`mov qword [rbp−0x50],0x41E1`). Checked (disasm `0x140222AB7`).
const GRAVITY_FLAGS: u64 = 0x41E1;
/// A bullet's effect handle (the bullet init's `0x1401A2D50` result, `+0x244`); the effect
/// object's scale `+0x10..+0x18` (the init writes `desc+0x90` there). Checked (decomp
/// `0x140157E60`).
const BULLET_EFFECT: usize = 0x244;
const EFFECT_SCALE: usize = 0x10;

/// Sora's position (body `+0x30`) and facing direction (Re:CoM's yaw: (sin, 0, cos)).
unsafe fn sora_pos_dir(pl: usize) -> ([f32; 3], [f32; 3]) {
    let b: usize = rd(pl + p::BODY);
    let v = if b != 0 { rd_vec(b + 0x30) } else { kh1_core::Vec3::ZERO };
    let ang = func::<AnglesFn>(ANGLES)(pl);
    let yaw = if ang.is_null() { 0.0 } else { std::ptr::read_unaligned(ang)[1] };
    ([v.x, v.y, v.z], [yaw.sin(), 0.0, yaw.cos()])
}

/// Gravity's command (`L0025a`): the target (Re:CoM's `+0x84C`) if it is an alive enemy, else the
/// offset 2 m along Sora's facing (taken now, KH1's `sp+0x10`); Re:CoM's marker (its `initMagic`
/// case 0x1E's effect id 2 of the set's `+0x30` entry) at KH1's sphere start.
unsafe fn gravity_command(pl: usize, r: &[u8; 16]) {
    let h: u32 = rd(pl + f::TARGET);
    let (pos, dir) = sora_pos_dir(pl);
    let ahead = mg::gravity::AHEAD / 100.0;
    let off = [dir[0] * ahead, 0.0, dir[2] * ahead];
    let (h, point) = match alive_enemy(h) {
        Some(e) => (h, aim_point(e)),
        None => (0, [pos[0] + off[0], pos[1], pos[2] + off[2]]),
    };
    if let Ok(mut g) = GRAVITY_CMD.lock() {
        *g = Some((h, off));
    }
    let up = mg::gravity::START_UP / 100.0;
    effect_at(pl, set_name(r, 0x18), 2, [point[0], point[1] + up, point[2], 1.0]);
}

/// Re:CoM's `0x1401A6B20(actor, name, id, &position, &angles, 0.0)` with Sora's angles (the
/// Gravity case's hand effect, `initMagic`'s marker).
unsafe fn effect_at(pl: usize, name: usize, id: u32, at: [f32; 4]) {
    type EffectFn = unsafe extern "win64" fn(usize, usize, u32, *const [f32; 4], u64, f32) -> i32;
    let ang = func::<AnglesFn>(ANGLES)(pl);
    if name == 0 || ang.is_null() {
        return;
    }
    let ang: [f32; 4] = std::ptr::read_unaligned(ang);
    func::<EffectFn>(EFFECT_AT)(pl, name, id, &at, &ang as *const [f32; 4] as u64, 0.0);
}

/// KH1's Gravity release (7.8, `xm_gravity01.mag`): the marker point now (the target's aim point
/// if it is still alive, else Sora + the command's offset), **Re:CoM's Gravity case**
/// (`0x140222170` case `0x26`, Checked (disasm `0x1402229E0..0x140222B90`)) with KH1's numbers: its
/// hand effect (id 1), its sphere (`CHomingBullet`, flags `0x41E1`, `+0xB8` = `0x1407C78D0`, the
/// row's set / `+5` / `+0xD`) **2.4 m above the point, pitched straight down, falling 2.3 m over
/// N frames** (speed 2.3 / N m a frame, no target: no homing), its attack held off (as 7.6's
/// bolts: KH1 strikes at the impact, not on contact), life N + 2 frames; the falling thread
/// ([`gravity_frame`]); then the MP (one point). The sphere's id.
unsafe fn release_gravity(pl: usize, c: &CastStart) -> Option<u32> {
    let r = row(c.spell, c.level)?;
    let (h, off) = GRAVITY_CMD.lock().ok().and_then(|mut g| g.take()).unwrap_or((0, [0.0; 3]));
    let point = match alive_enemy(h) {
        Some(e) => aim_point(e),
        None => {
            let (pos, _) = sora_pos_dir(pl);
            [pos[0] + off[0], pos[1], pos[2] + off[2]]
        }
    };
    let up = mg::gravity::START_UP / 100.0;
    let fall = mg::gravity::FALL / 100.0;
    let start = [point[0], point[1] + up, point[2], 1.0];
    let centre = [point[0], point[1] + up - fall, point[2]];
    let (_, n) = mg::gravity::of(c.level);
    // The hand effect (id 1 of the set's `+0x30` entry at the hand point).
    let body: usize = rd(pl + p::BODY);
    let mut hand = [0f32; 4];
    if body != 0 {
        let b = rd_vec(body + 0x30);
        hand = [b.x, b.y, b.z, 1.0];
    }
    func::<PlOutFn>(HAND_POINT)(pl, &mut hand);
    effect_at(pl, set_name(r, 0x18), 1, hand);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    let ang: [f32; 4] = if ang.is_null() { [0.0; 4] } else { std::ptr::read_unaligned(ang) };
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
    let w = |o: usize| dp.add(o);
    let (kind, attr, ap): (u32, u32, i32) = (rd(pl + f::KIND), rd(pl + f::ATTR), rd(pl + f::AP));
    std::ptr::write_unaligned(w(0x68) as *mut u32, kind);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x0C) as *mut u32, 0x8000_0000);
    std::ptr::write_unaligned(w(0x60) as *mut u64, GRAVITY_FLAGS | 4 | COLLISION_HELD);
    std::ptr::write_unaligned(w(0x8C) as *mut i32, HELD_TICKS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, fall / n);
    std::ptr::write_unaligned(w(0x34) as *mut f32, r[0xC] as f32);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, ((n + 2.0) * 50.0) as i32);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], start);
    std::ptr::write_unaligned(w(0x1C) as *mut f32, r[5] as f32 * 0.01);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [std::f32::consts::FRAC_PI_2, ang[1], ang[2], ang[3]]);
    std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    std::ptr::write_unaligned(w(0xB8) as *mut usize, va(base(), GRAVITY_DESC_B8));
    std::ptr::write_unaligned(w(0x78) as *mut f32, mg::gravity::impact_radius_m(c.level));
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        note(|| format!("release gravity{}: Re:CoM's shot manager made no sphere (no MP paid)", c.level));
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, r[0xD] as f32 * 0.01, 0.0, 0, -1);
    hold_off(bullet);
    let id: u32 = rd(bullet + 0x20);
    remember_shot(id, c.spell, c.level, 0);
    let run = GravityRun { fall: mg::Fall::new(c.level), level: c.level, shot: id, centre, base_scale: None, kind, attr, ap };
    if let Ok(mut g) = FALLS.lock() {
        if g.len() >= 8 {
            g.remove(0);
        }
        g.push(run);
        STRIKING.store(true, Ordering::Release);
    }
    // KH1 pays after the falling thread starts (sys 20, then sys 47).
    super::mp::pay(mg::gravity::COST, "Gravity release");
    note(|| format!("release gravity{}: sphere {id} from ({:.2},{:.2},{:.2}) falls {fall} m over {n} f to ({:.2},{:.2},{:.2}), impact radius {:.2} m, target {h:#x}", c.level, start[0], start[1], start[2], centre[0], centre[1], centre[2], mg::gravity::impact_radius_m(c.level)));
    Some(id)
}

/// A shot's attack switched off for good (Re:CoM's own "off", vtable +8, as 7.6's bolts: the bullet
/// init tests the held flag before it fills the count).
unsafe fn hold_off(bullet: usize) {
    let c: usize = rd(bullet + 0x2A8);
    if c == 0 {
        return;
    }
    let vt: usize = rd(c);
    if rd::<usize>(vt + 8) == base() + COLL_OFF as usize {
        func::<PlFn>(COLL_OFF)(c);
    }
}

/// Re:CoM's apply (`CCollAttack` vtable `+0x38`) of a shot's attack on an enemy's first live hurt
/// body: what Re:CoM's collision pass calls once its overlap test passes (7.6). Only through
/// Re:CoM's own slots.
unsafe fn apply_to(bullet: usize, enemy: usize) -> Option<u32> {
    type ApplyFn = unsafe extern "win64" fn(usize, usize) -> u32;
    let c: usize = rd(bullet + 0x2A8);
    let b = bodies(enemy).first().map(|b| b.0)?;
    if c == 0 {
        return None;
    }
    let vt: usize = rd(c);
    if rd::<usize>(vt + 8) != base() + COLL_OFF as usize || rd::<usize>(vt + 0x38) != base() + COLL_APPLY as usize {
        return None;
    }
    func::<PlFn>(COLL_OFF)(c);
    Some(func::<ApplyFn>(COLL_APPLY)(c, b))
}

/// Our shot by its id (Re:CoM's entity handle, `+0x20`, resolved by `0x1402AD350` as the apply
/// resolves bodies' owners), if it is still one of Re:CoM's bullets.
unsafe fn shot_object(id: u32) -> Option<usize> {
    if id == 0 {
        return None;
    }
    let o = func::<mv::HandleFn>(mv::HANDLE_OBJ)(id);
    if o == 0 {
        return None;
    }
    let vt: usize = rd(o);
    let b = base();
    (vt == va(b, crate::guard::bullet::HOMING_VTABLE) || vt == va(b, crate::guard::bullet::VTABLE)).then_some(o)
}

/// One run of Gravity's falling threads (Sora's frame start): the sphere's KH1 growth, and at the
/// impact KH1's strikes (`L0044c`): every alive enemy whose bodies touch the 1.7 × S m sphere at the
/// centre, struck once with the sphere's attack through Re:CoM's apply. If the sphere is gone
/// (Re:CoM ended it on a wall), a held bolt like 7.6's carries the strikes.
unsafe fn gravity_frame(pl: usize, step: f32) {
    let mut due = Vec::new();
    {
        let Ok(mut g) = FALLS.lock() else { return };
        for run in g.iter_mut() {
            let impact = run.fall.run(step);
            if let Some(o) = shot_object(run.shot) {
                grow(o, run);
            }
            if impact {
                due.push((run.shot, run.level, run.centre, run.kind, run.attr, run.ap));
            }
        }
        g.retain(|r| !(r.fall.left <= 0.0 && due.iter().any(|d| d.0 == r.shot)));
    }
    for (shot, level, centre, kind, attr, ap) in due {
        let area = mg::Area::Sphere { radius: mg::gravity::impact_radius_m(level) };
        let carrier = shot_object(shot).or_else(|| carrier_at(pl, level, centre, kind, attr, ap));
        let mut n = 0;
        for (h, _) in lock_on_list(0, false) {
            let Some(e) = alive_enemy(h) else { continue };
            if !bodies(e).iter().any(|(_, b)| mg::body_touches(centre, area, b)) {
                continue;
            }
            let res = carrier.and_then(|c| apply_to(c, e));
            n += 1;
            bolt_line(|| format!("gravity{level} impact on {h:#x}: Re:CoM's apply -> {res:?}"));
        }
        event(format!("impact gravity{level} shot {shot} n{n}"));
    }
}

/// KH1's growth (scale 0.01 → S over the fall) on Re:CoM's sphere effect, from its own spawn scale.
unsafe fn grow(bullet: usize, run: &mut GravityRun) {
    type ObjFn = unsafe extern "win64" fn(u32) -> usize;
    let h: u32 = rd(bullet + BULLET_EFFECT);
    if h == 0 {
        return;
    }
    let o = func::<ObjFn>(EFFECT_OBJECT)(h);
    if o == 0 {
        return;
    }
    let base = *run.base_scale.get_or_insert_with(|| {
        let s: f32 = rd(o + EFFECT_SCALE);
        if s > 0.0 && s < 100.0 {
            s
        } else {
            1.0
        }
    });
    let s = base * mg::gravity::scale_at(run.level, run.fall.fallen());
    wr(o + EFFECT_SCALE, [s, s, s]);
}

/// A held strike carrier at the impact (when Re:CoM already ended the sphere): a `CBullet` of the
/// row's set like 7.6's bolts, its attack off.
unsafe fn carrier_at(pl: usize, level: u8, centre: [f32; 3], kind: u32, attr: u32, ap: i32) -> Option<usize> {
    let r = row(mg::spell::GRAVITY, level)?;
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 1, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x68) as *mut u32, kind);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x0C) as *mut u32, 0x8000_0000);
    std::ptr::write_unaligned(w(0x60) as *mut u64, THUNDER_BOLT_FLAGS | COLLISION_HELD);
    std::ptr::write_unaligned(w(0x8C) as *mut i32, HELD_TICKS);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, 100);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], [centre[0], centre[1], centre[2], 1.0]);
    std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    std::ptr::write_unaligned(w(0x78) as *mut f32, mg::gravity::impact_radius_m(level));
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    hold_off(bullet);
    remember_shot(rd(bullet + 0x20), mg::spell::GRAVITY, level, 0);
    Some(bullet)
}

/// `0x1402ABB20(object, bone)`: a bone of an actor (or the weapon) for effects. Checked by M5.
const BONE: u32 = 0x002A_BB20;
/// `0x1401A2D50(actor, effect name, id, bone, &offset, 0, 0.0)`: Re:CoM's effect on a bone (a
/// thunk to `0x1402577D0` with the actor's `+0x150`); `0x140257EF0(handle)` its object (`+0x4C |=
/// 4` as the Cure / Stop cases do). Checked (disasm `0x140222946..0x1402229B3`).
const EFFECT_ON_BONE: u32 = 0x001A_2D50;
const EFFECT_OBJECT: u32 = 0x0025_7EF0;
/// `0x1401E59A0(player, amount, 0, 1, 0)`: Re:CoM's heal (HP += amount, clamped to the max; the
/// HUD and the heal voice). Checked (decomp).
const HEAL: u32 = 0x001E_59A0;
/// `initMagic`'s charge offset (`0x1407C7E50`: (−0.8, 0, 0, 1)) and the Cure / Stop cases' body
/// offset (`0x1405A0910`: (0, 0.85, 0, 1)). Checked (exe).
const CHARGE_OFFSET: [f32; 4] = [-0.8, 0.0, 0.0, 1.0];
const BODY_OFFSET: [f32; 4] = [0.0, 0.85, 0.0, 1.0];
/// The player's weapon (`+0x3D0`, `initMagic`'s charge goes on its bone 2). Checked (disasm).
const WEAPON: usize = 0x3D0;

/// An effect of the row's set (name at the set's `+0x18 + extra`) on a bone, as Re:CoM's cases
/// start them. The effect's handle.
unsafe fn effect_on_bone(pl: usize, bone_of: usize, bone: i32, set_name: usize, id: u32, offset: &[f32; 4], sticky: bool) -> u32 {
    type BoneFn = unsafe extern "win64" fn(usize, i32) -> usize;
    type EffFn = unsafe extern "win64" fn(usize, usize, u32, usize, *const [f32; 4], u64, f32) -> u32;
    type ObjFn = unsafe extern "win64" fn(u32) -> usize;
    if bone_of == 0 || set_name == 0 {
        return 0;
    }
    let b = func::<BoneFn>(BONE)(bone_of, bone);
    let h = func::<EffFn>(EFFECT_ON_BONE)(pl, set_name, id, b, offset, 0, 0.0);
    if sticky {
        let o = func::<ObjFn>(EFFECT_OBJECT)(h);
        if o != 0 {
            wr(o + 0x4C, rd::<u32>(o + 0x4C) | 4);
        }
    }
    h
}

/// The name of an effect set's entry (`0x1405A1730 + set × 0x60 + 0x18 + extra`; Gravity's cases
/// read `+0x30`).
unsafe fn set_name(r: &[u8; 16], extra: usize) -> usize {
    rd(va(base(), EFFECT_SETS) + r[2] as usize * 0x60 + 0x18 + extra)
}

/// `initMagic`'s charge for Cure / Aero (cases 0x1D / 0x20; Gravity's reads the set's `+0x30`):
/// effect id 0 of the row's set on the weapon's bone 2, at (−0.8, 0, 0). Checked (disasm
/// `0x1402206F0..0x140220752`).
unsafe fn charge_on_weapon(pl: usize, r: &[u8; 16], extra: usize) {
    let weapon: usize = rd(pl + WEAPON);
    effect_on_bone(pl, weapon, 2, set_name(r, extra), 0, &CHARGE_OFFSET, false);
}

/// KH1's Cure release (7.7, `xm_cure01.mag`): the heal thread's target test (Sora: alive, not in
/// Re:CoM's death path), then **Re:CoM's Cure case** (`0x140222170` case `0x25`, Checked (disasm
/// `0x140222946..0x1402229CA`)): its `cure_all` effect (id set − 14 on Sora's bone 0 at (0, 0.85,
/// 0), kept on Sora) and its heal `0x1401E59A0(player, AP, 0, 1, 0)` with AP = 50 / 150 / 300; then
/// KH1's MP (sys 47 after the strike). An invalid target heals nothing and pays nothing.
unsafe fn release_cure(pl: usize, c: &CastStart) -> Option<u32> {
    type HealFn = unsafe extern "win64" fn(usize, i32, i32, i32, i32);
    let r = row(c.spell, c.level)?;
    let hp: i32 = rd(pl + HP);
    let dead = rd::<i32>(pl + DEAD) != 0;
    if !mg::cure::target_ok(hp, dead) {
        note(|| format!("release cure{}: Sora can't be healed (HP {hp}, dying {dead}): no heal, no MP", c.level));
        return None;
    }
    effect_on_bone(pl, pl, 0, set_name(r, 0), (r[2] as u32).wrapping_sub(0xE), &BODY_OFFSET, true);
    let ap: i32 = rd(pl + f::AP);
    func::<HealFn>(HEAL)(pl, ap, 0, 1, 0);
    let after: i32 = rd(pl + HP);
    super::mp::pay(mg::cure::COST, "Cure release");
    note(|| format!("release cure{}: Re:CoM's heal {ap} -> HP {hp} -> {after} (max {})", c.level, rd::<i32>(pl + HP + 4)));
    event(format!("heal {ap} hp {after}"));
    // No shot: the trace's release token shows `no-shot`, a heal is its own event.
    None
}

/// Sora's HP (`+0x42C`, max `+0x430`) and Re:CoM's death flag (`+0x7B0`). Checked (decomp
/// `0x1401E59A0`, `0x1401E6DE0`).
const HP: usize = 0x42C;
const DEAD: usize = 0x7B0;

fn remember_shot(id: u32, s: u8, level: u8, serial: u32) {
    if let Ok(mut g) = SHOTS.lock() {
        if g.len() >= 32 {
            g.remove(0);
        }
        g.push((id, s, level, serial));
    }
}

unsafe fn release_fire(pl: usize, c: &CastStart) -> Option<u32> {
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
    // KH1's ball with no target picks one each frame (7.4b); with a target, Re:CoM's homing is
    // KH1's (it homes while the target exists, then flies straight, never re-picking).
    if rd::<i32>(pl + f::TARGET) <= 0 {
        seek_on(bullet, id);
    }
    remember_shot(id, c.spell, c.level, 0);
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

/// KH1's Blizzard release (7.5, `xm_blizzard01.mag` `L006dc` → the spawner thread): seven shards,
/// each Re:CoM's Blizzard shot (`0x140222170` case `0x23`'s calls: descriptor class 2, the hand
/// point, Re:CoM's `bliz_all` set for the straight shard and the next set for the cone ones, the
/// first shard's hit serial passed to the others) with KH1's numbers: the cone and start offsets
/// (`crate::magic::blizzard_shards`), speed, life, radius, a hit or wall ends it; then each shard
/// glides as KH1's ([`kh1_shard_exe`]); then the MP (sys 47 after the thread). The first shard's id.
unsafe fn release_blizzard(pl: usize, c: &CastStart) -> Option<u32> {
    let r = row(c.spell, c.level)?;
    let body: usize = rd(pl + p::BODY);
    let mut hand = [0f32; 4];
    if body != 0 {
        let b = rd_vec(body + 0x30);
        hand = [b.x, b.y, b.z, 1.0];
    }
    func::<PlOutFn>(HAND_POINT)(pl, &mut hand);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    if ang.is_null() {
        return None;
    }
    let ang: [f32; 4] = std::ptr::read_unaligned(ang);
    let speed = r[9] as f32 * 0.01;
    let life = r[0xE] as i32 * 100;
    let mut serial = 0u32;
    let mut first = None;
    let mut made = 0;
    for (k, (off, dir)) in mg::blizzard_shards(ang[1]).iter().enumerate() {
        let Some((pitch, yaw)) = crate::guard::aim_angles(*dir) else { continue };
        let mut d = Desc([0; 0x100]);
        let dp = d.0.as_mut_ptr();
        func::<DescInitFn>(DESC_INIT)(dp, 2, 3);
        let at = |o: usize| dp.add(o);
        std::ptr::write_unaligned(at(0x68) as *mut u32, rd::<u32>(pl + f::KIND));
        std::ptr::write_unaligned(at(0x6C) as *mut u32, rd::<u32>(pl + f::ATTR));
        std::ptr::write_unaligned(at(0x70) as *mut i32, rd::<i32>(pl + f::AP));
        std::ptr::write_unaligned(at(0x0C) as *mut u32, 0x8000_0000);
        std::ptr::write_unaligned(at(0x60) as *mut u64, mg::KH1_BLIZZARD_FLAGS);
        std::ptr::write_unaligned(at(0x30) as *mut f32, speed);
        std::ptr::write_unaligned(at(0x34) as *mut f32, r[0xC] as f32);
        std::ptr::write_unaligned(at(0x38) as *mut i32, life);
        std::ptr::write_unaligned(at(0x3C) as *mut i32, life);
        std::ptr::write_unaligned(at(0x10) as *mut [f32; 4], [hand[0] + off[0], hand[1] + off[1], hand[2] + off[2], r[5] as f32 * 0.01]);
        std::ptr::write_unaligned(at(0x20) as *mut [f32; 4], [pitch, yaw, ang[2], ang[3]]);
        let set = if k < mg::blizzard::CONE_SHARDS { EFFECT_SETS_NEXT } else { EFFECT_SETS };
        std::ptr::write_unaligned(at(0xA8) as *mut usize, va(base(), set) + r[2] as usize * 0x60);
        std::ptr::write_unaligned(at(0x78) as *mut f32, r[4] as f32 * 0.01);
        std::ptr::write_unaligned(at(0x74) as *mut u32, serial);
        let factory = func::<FactoryFn>(FACTORY)();
        if factory == 0 {
            break;
        }
        let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
        if bullet == 0 {
            continue;
        }
        // A small mover (Re:CoM's Fire set-up): the shard glides 50 cm above the floor, its attack
        // keeps KH1's radius.
        func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
        if serial == 0 {
            serial = func::<SerialFn>(SHOT_SERIAL)(bullet);
        }
        let id: u32 = rd(bullet + 0x20);
        first.get_or_insert(id);
        made += 1;
        remember_shot(id, c.spell, c.level, serial);
        shard_on(bullet, id, dir[1] * speed);
    }
    let first = first?;
    // KH1 pays after the spawner thread starts (sys 20, then sys 47).
    super::mp::pay(mg::blizzard::COST, "Blizzard release");
    note(|| format!("release {}{}: {made} shards from ({:.2},{:.2},{:.2}) yaw {:.3}, speed {speed:.2} m/f, life {life} ticks, radius {:.2} m, serial {serial}", mg::spell::NAMES[c.spell as usize], c.level, hand[0], hand[1], hand[2], ang[1], r[4] as f32 * 0.01));
    Some(first)
}

/// An alive enemy for a handle: Re:CoM's enemy (type 0x1004) with HP left. KH1's sys 120 / its
/// actor walk count live actors (`+0x374` bit 0 set, bit 1 clear) not flagged defeated.
unsafe fn alive_enemy(h: u32) -> Option<usize> {
    type TypeFn = unsafe extern "win64" fn(usize) -> u32;
    if h == 0 || h as i32 <= 0 {
        return None;
    }
    let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h);
    if obj == 0 {
        return None;
    }
    let vt: usize = rd(obj);
    let ty: TypeFn = std::mem::transmute_copy(&rd::<usize>(vt + crate::battle::enemy::VT_TYPE));
    (ty(obj) == crate::battle::ENEMY_TYPE && rd::<i32>(obj + crate::battle::enemy::HP) > 0).then_some(obj)
}

/// An actor's aim point (vtable `+0x50`, what Re:CoM homes and its Thunder strikes at; KH1's target
/// part, Assumption 5) and its position (mover `+0x30`, KH1's actor `+0x10`).
unsafe fn aim_point(obj: usize) -> [f32; 3] {
    type AimFn = unsafe extern "win64" fn(usize, *mut [f32; 4]);
    let vt: usize = rd(obj);
    let f: AimFn = std::mem::transmute_copy(&rd::<usize>(vt + 0x50));
    let mut p = [0f32; 4];
    f(obj, &mut p);
    [p[0], p[1], p[2]]
}

unsafe fn position(obj: usize) -> Option<[f32; 3]> {
    let mover: usize = rd(obj + rt::MOVER);
    (mover != 0).then(|| {
        let p = rd_vec(mover + rt::POSITION);
        [p.x, p.y, p.z]
    })
}

/// Thunder's command (`L004b6`): the target (Re:CoM's `+0x84C`, the command's) if it is an alive
/// enemy, and the area centre: its aim point, else Sora's position.
unsafe fn thunder_command(pl: usize) {
    let h: u32 = rd(pl + f::TARGET);
    let cmd = match alive_enemy(h) {
        Some(e) => (h, aim_point(e)),
        None => {
            let b: usize = rd(pl + p::BODY);
            let v = if b != 0 { rd_vec(b + 0x30) } else { kh1_core::Vec3::ZERO };
            (0, [v.x, v.y, v.z])
        }
    };
    if let Ok(mut g) = THUNDER_CMD.lock() {
        *g = Some(cmd);
    }
}

/// The hurt bodies of an enemy as Re:CoM's primitives read their shapes (the live ones: body
/// `+0x10` on, shape kind 2 / 4 / 5).
unsafe fn bodies(enemy: usize) -> Vec<(usize, mg::BodyShape)> {
    let mut out = Vec::new();
    let info: usize = rd(enemy + 0xF8);
    if info == 0 {
        return out;
    }
    let parts: usize = rd(info + 0x40);
    let n: i32 = rd(info + 0x38);
    for i in 0..n.clamp(0, 32) as usize {
        let part = parts + i * 0x120;
        let shape: usize = rd(part + 0x18);
        if rd::<i32>(part + 0x10) == 0 || shape == 0 {
            continue;
        }
        let pt = |o: usize| {
            let v: [f32; 4] = rd(shape + o);
            [v[0], v[1] + rd::<f32>(shape + 0x74), v[2]]
        };
        let r: f32 = rd(shape + 0x70);
        match rd::<u32>(shape + 0x60) {
            2 => out.push((part, mg::BodyShape::Sphere { c: pt(0x20), r })),
            4 | 5 => out.push((part, mg::BodyShape::Column { a: pt(0x10), b: pt(0x20), r })),
            _ => {}
        }
    }
    out
}

/// KH1's Thunder release (7.6, `xm_thunder01.mag`): the centre moves to the target if it is still
/// alive; Re:CoM's lock-on enemies, sorted nearest first from the target / Sora
/// (`crate::magic::thunder_order`); the first strike now; the MP (one point); the rest every 8
/// frames from Sora's frame start ([`strikes_frame`]). The first bolt's id, if one struck.
unsafe fn release_thunder(pl: usize, c: &CastStart) -> Option<u32> {
    let (h, mut centre) = THUNDER_CMD.lock().ok().and_then(|mut g| g.take()).unwrap_or((0, [0.0; 3]));
    let target = alive_enemy(h);
    if let Some(t) = target {
        centre = aim_point(t);
    }
    let from = match target {
        Some(t) => position(t),
        None => {
            let b: usize = rd(pl + p::BODY);
            (b != 0).then(|| {
                let v = rd_vec(b + 0x30);
                [v.x, v.y, v.z]
            })
        }
    }
    .unwrap_or(centre);
    let enemies: Vec<(u32, [f32; 3])> = lock_on_list(0, false).into_iter().filter_map(|(h, _)| Some((h, position(alive_enemy(h)?)?))).collect();
    let order = mg::thunder_order(from, enemies);
    let n = order.len();
    // Re:CoM's Thunder case's first effect: id 1 at the hand.
    if let Some(r) = row(c.spell, c.level) {
        let mut hand = rd::<[f32; 4]>(pl + PLAYER_POS);
        func::<PlOutFn>(HAND_POINT)(pl, &mut hand);
        thunder_effect(pl, r, 1, hand);
    }
    let area = mg::Area::Cylinder { radius: mg::thunder::RADIUS / 100.0, half: mg::thunder::HALF_HEIGHT / 100.0 };
    let run = ThunderRun { s: mg::Strikes::new(order), spell: mg::spell::THUNDER, area, level: c.level, centre, kind: rd(pl + f::KIND), attr: rd(pl + f::ATTR), ap: rd(pl + f::AP) };
    let first = match STRIKES.lock() {
        Ok(mut g) => {
            if g.len() >= 8 {
                g.remove(0);
            }
            g.push(run);
            STRIKING.store(true, Ordering::Release);
            let i = g.len() - 1;
            drop(g);
            strike_step(pl, 0.0, Some(i)).into_iter().next()
        }
        Err(_) => None,
    };
    // KH1 pays after the strike thread starts (sys 20, then sys 47).
    super::mp::pay(mg::thunder::COST, "Thunder release");
    note(|| format!("release thunder{}: area centre ({:.2},{:.2},{:.2}) r {} m ±{} m, {n} enemies listed from ({:.2},{:.2},{:.2}), first bolt {first:?}", c.level, centre[0], centre[1], centre[2], mg::thunder::RADIUS / 100.0, mg::thunder::HALF_HEIGHT / 100.0, from[0], from[1], from[2]));
    first
}

/// Re:CoM's effect call as its Thunder case makes it (Sora's angles as the rotation).
unsafe fn thunder_effect(pl: usize, r: &[u8; 16], id: u32, at: [f32; 4]) {
    type EffectFn = unsafe extern "win64" fn(usize, usize, u32, *const [f32; 4], u64, f32) -> i32;
    let name: usize = rd(va(base(), EFFECT_SETS) + r[2] as usize * 0x60 + 0x18);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    if name == 0 || ang.is_null() {
        return;
    }
    let ang: [f32; 4] = std::ptr::read_unaligned(ang);
    func::<EffectFn>(EFFECT_AT)(pl, name, id, &at, &ang as *const [f32; 4] as u64, 0.0);
}

/// Sora's frame start (the request-bits wrapper, every frame at Combat = KH1): one run of a
/// Thunder strike thread, if one is going (one atomic load otherwise).
pub unsafe fn strikes_frame(pl: usize) {
    if !STRIKING.load(Ordering::Acquire) {
        return;
    }
    let step: f32 = rd(pl + p::STEP);
    strike_step(pl, step, None);
    gravity_frame(pl, step);
    let idle = STRIKES.lock().map(|g| g.is_empty()).unwrap_or(true) && FALLS.lock().map(|g| g.is_empty()).unwrap_or(true);
    if idle {
        STRIKING.store(false, Ordering::Release);
    }
}

/// One run of the strike threads (`only`: the release's own run of the new thread, step 0: its
/// wait then counts that run). The bolts' ids.
unsafe fn strike_step(pl: usize, step: f32, only: Option<usize>) -> Vec<u32> {
    let mut due = Vec::new();
    {
        let Ok(mut g) = STRIKES.lock() else { return Vec::new() };
        for (i, run) in g.iter_mut().enumerate() {
            if only.is_some_and(|o| o != i) {
                continue;
            }
            let (centre, area) = (run.centre, run.area);
            let hit = run.s.run(if step > 0.0 { step } else { 1.0 }, |h| {
                let Some(e) = alive_enemy(h) else { return false };
                bodies(e).iter().any(|(_, b)| mg::body_touches(centre, area, b))
            });
            if let Some(h) = hit {
                due.push((h, run.spell, run.level, run.kind, run.attr, run.ap, run.s.struck));
            }
        }
        g.retain(|r| !r.s.done());
    }
    let mut ids = Vec::new();
    for (h, sp, level, kind, attr, ap, n) in due {
        let Some(e) = alive_enemy(h) else { continue };
        let (id, what) = if sp == mg::spell::STOP { (stop_strike(pl, e, h, level, kind, attr, ap), "stop") } else { (strike(pl, e, h, level, kind, attr, ap), "bolt") };
        event(format!("{what} {n} e{h:x}{}", id.map_or(" -".into(), |i| format!(" shot {i}"))));
        ids.extend(id);
    }
    ids
}

/// One bolt (KH1's sys 160): Re:CoM's Thunder case for this enemy (its bolt effect, its `CBullet`
/// with the cast's kind / attribute / AP, the attack held off), then **Re:CoM's own hit path
/// directly**: the bolt's attack applied to the enemy's first live hurt body with the attack's
/// apply (`CCollAttack` vtable `+0x38` = `0x14016F1C0`), the call Re:CoM's collision pass
/// (`0x14016EAD0`) makes once its overlap test passes. Re:CoM then runs the enemy's hit (its
/// handler, our KH1 damage on it, the reaction, the hit effect). The bolt's id.
#[allow(clippy::too_many_arguments)]
unsafe fn strike(pl: usize, enemy: usize, h: u32, level: u8, kind: u32, attr: u32, ap: i32) -> Option<u32> {
    type ApplyFn = unsafe extern "win64" fn(usize, usize) -> u32;
    let r = row(mg::spell::THUNDER, level)?;
    let aim = aim_point(enemy);
    let sora: [f32; 4] = rd(pl + PLAYER_POS);
    let mut at = [aim[0], sora[1] - THUNDER_DROP, aim[2], sora[3]];
    thunder_effect(pl, r, (r[2] as u32).wrapping_sub(10), at);
    at[1] += THUNDER_DROP;
    let ang = func::<AnglesFn>(ANGLES)(pl);
    let ang: [f32; 4] = if ang.is_null() { [0.0; 4] } else { std::ptr::read_unaligned(ang) };
    let mut d = Desc([0; 0x100]);
    let dp = d.0.as_mut_ptr();
    func::<DescInitFn>(DESC_INIT)(dp, 1, 3);
    let w = |o: usize| dp.add(o);
    std::ptr::write_unaligned(w(0x68) as *mut u32, kind);
    std::ptr::write_unaligned(w(0x6C) as *mut u32, attr);
    std::ptr::write_unaligned(w(0x70) as *mut i32, ap);
    std::ptr::write_unaligned(w(0x0C) as *mut u32, 0x8000_0000);
    std::ptr::write_unaligned(w(0x60) as *mut u64, THUNDER_BOLT_FLAGS | COLLISION_HELD);
    std::ptr::write_unaligned(w(0x8C) as *mut i32, HELD_TICKS);
    std::ptr::write_unaligned(w(0x30) as *mut f32, r[9] as f32 * 0.01);
    std::ptr::write_unaligned(w(0x3C) as *mut i32, THUNDER_BOLT_LIFE);
    std::ptr::write_unaligned(w(0x34) as *mut f32, r[0xC] as f32);
    std::ptr::write_unaligned(w(0x20) as *mut [f32; 4], [std::f32::consts::FRAC_PI_2, ang[1], ang[2], ang[3]]);
    std::ptr::write_unaligned(w(0x10) as *mut [f32; 4], at);
    std::ptr::write_unaligned(w(0xA8) as *mut usize, va(base(), EFFECT_SETS) + r[2] as usize * 0x60);
    std::ptr::write_unaligned(w(0x78) as *mut f32, r[4] as f32 * 0.1);
    let factory = func::<FactoryFn>(FACTORY)();
    if factory == 0 {
        return None;
    }
    let bullet = func::<ManagerFn>(SHOT_MANAGER)(factory, dp, pl);
    if bullet == 0 {
        bolt_line(|| format!("bolt on {h:#x}: Re:CoM made no shot"));
        return None;
    }
    func::<MoverFn>(SHOT_MOVER)(bullet, 0.1, 0.0, 0, -1);
    let id: u32 = rd(bullet + 0x20);
    remember_shot(id, mg::spell::THUNDER, level, 0);
    let coll: usize = rd(bullet + 0x2A8);
    let body = bodies(enemy).first().map(|b| b.0);
    let res = match (coll, body) {
        (0, _) | (_, None) => None,
        (c, Some(b)) => {
            // The bolt's own collision stays off (Re:CoM's own "off", the slot its init calls for a
            // held collision): its one hit is this strike. Only through Re:CoM's own slots.
            let vt: usize = rd(c);
            let (off, apply) = (rd::<usize>(vt + 8), rd::<usize>(vt + 0x38));
            if off != base() + COLL_OFF as usize || apply != base() + COLL_APPLY as usize {
                bolt_line(|| format!("bolt on {h:#x}: the shot's attack is not a CCollAttack (off {off:#x}, apply {apply:#x}); no strike"));
                None
            } else {
                func::<PlFn>(COLL_OFF)(c);
                Some(func::<ApplyFn>(COLL_APPLY)(c, b))
            }
        }
    };
    bolt_line(|| format!("bolt on {h:#x}: shot {id}, Re:CoM's apply -> {res:?} (1 hit, 2 / 3 guarded, 0 already hit)"));
    Some(id)
}

fn bolt_line(line: impl FnOnce() -> String) {
    if BOLT_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic: {}", line()));
    }
}

/// A shard's flight state's exe becomes [`kh1_shard_exe`] (as [`seek_on`]: only on a plain
/// `CHomingBullet` whose exe is Re:CoM's thunk).
unsafe fn shard_on(bullet: usize, id: u32, dy: f32) {
    let b = base();
    let vt: usize = rd(bullet);
    let exe: usize = rd(bullet + rt::STATE_EXE);
    let adj: i32 = rd(bullet + rt::STATE_EXE_ADJUST);
    if vt != va(b, crate::guard::bullet::HOMING_VTABLE) || exe != b + HOMING_THUNK as usize || adj != 0 {
        rt_line(|| format!("blizzard: shard {id} is not a plain CHomingBullet in flight: Re:CoM's straight flight"));
        return;
    }
    if let Ok(mut g) = SHARDS.lock() {
        if g.len() >= 32 {
            g.remove(0);
        }
        g.push((id, dy));
    }
    wr(bullet + rt::STATE_EXE, kh1_shard_exe as *const () as usize);
}

/// The flight-state exe of our Blizzard shards (7.5): KH1's glide for this frame
/// (`crate::magic::blizzard_glide_y`): Re:CoM's segment test (`0x140170330` on the shard mover's
/// own collision object, as Re:CoM's movers test their moves) from the shard's height straight down
/// by `0.5 m − dy`, at where the shard is going this frame; the vertical velocity then lands it 0.5
/// m above that floor, or on its own slope. Then Re:CoM's own step (the thunk: no target, so the
/// homing step only runs its checks). Runs only for these shards, once a frame each.
pub unsafe extern "win64" fn kh1_shard_exe(bullet: usize) {
    use crate::guard::bullet as gb;
    let mover: usize = rd(bullet + gb::MOVER);
    let live = rd::<i32>(bullet + rt::HOLD) == 0 && rd::<i32>(bullet + rt::LIFE) > 0 && mover != 0;
    let id: u32 = rd(bullet + 0x20);
    let dy = SHARDS.lock().ok().and_then(|g| g.iter().find(|e| e.0 == id).map(|e| e.1));
    if let (true, Some(dy)) = (live, dy) {
        let pos = rd_vec(mover + rt::POSITION);
        let v: [f32; 4] = rd(mover + gb::VELOCITY);
        let floor = floor_below(mover, [pos.x + v[0], pos.y, pos.z + v[2]], mg::blizzard_ray(dy));
        let y = mg::blizzard_glide_y(pos.y, dy, floor);
        wr(mover + gb::VELOCITY + 4, y - pos.y);
    }
    func::<PlFn>(HOMING_THUNK)(bullet)
}

/// Re:CoM's segment test from `from` straight down by `len` m with this mover's collision object
/// (`mover+0x158`, its segment `+0x18`: ends `+0x10` / `+0x20`, hit `+0x30`; the parameter
/// `mover+0x134`), as `win::movement`'s floor probe for Sora. The hit's height, any surface (KH1's
/// sys 77 takes any).
unsafe fn floor_below(mover: usize, from: [f32; 3], len: f32) -> Option<f32> {
    if len <= 0.0 {
        return None;
    }
    let coll: usize = rd(mover + rt::MOVER_COLL);
    if coll == 0 {
        return None;
    }
    let seg: usize = rd(coll + 0x18);
    if seg == 0 {
        return None;
    }
    for (o, y) in [(0x10, from[1]), (0x20, from[1] - len)] {
        wr(seg + o, [from[0], y, from[2], 1.0f32]);
    }
    let hit = func::<mv::CollideFn>(mv::COLLIDE)(coll, rd::<i32>(mover + rt::MOVER_COLL_PARAM)) != 0;
    let seg: usize = rd(coll + 0x18);
    (hit && seg != 0).then(|| rd::<f32>(seg + 0x34))
}

/// Our ball cast with no target: its flight state's exe becomes [`kh1_ball_exe`] (only when it is
/// Re:CoM's homing thunk with no this-adjust, on a `CHomingBullet`).
unsafe fn seek_on(bullet: usize, id: u32) {
    if !RETARGET.load(Ordering::Acquire) {
        return;
    }
    let b = base();
    let vt: usize = rd(bullet);
    let exe: usize = rd(bullet + rt::STATE_EXE);
    let adj: i32 = rd(bullet + rt::STATE_EXE_ADJUST);
    if vt != va(b, crate::guard::bullet::HOMING_VTABLE) || exe != b + HOMING_THUNK as usize || adj != 0 {
        rt_line(|| format!("re-target: shot {id} is not a plain CHomingBullet in flight (vt {vt:#x}, exe {exe:#x}, adjust {adj}): Re:CoM's homing"));
        return;
    }
    if let Ok(mut g) = SEEKERS.lock() {
        if g.len() >= 16 {
            g.remove(0);
        }
        g.push((id, 0));
    }
    wr(bullet + rt::STATE_EXE, kh1_ball_exe as *const () as usize);
    rt_line(|| format!("re-target: shot {id} cast with no target: KH1's pick each frame (best enemy within {} m of the ball)", mg::retarget::RANGE_M));
}

fn rt_line(line: impl FnOnce() -> String) {
    if RETARGET_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("magic: {}", line()));
    }
}

/// Re:CoM's enemies in its lock-on list (`0x1401CDB90`'s walk and filter: the handle resolves, the
/// object is not the shot, `+0x154 & 0x10000`), kept to enemies (type 0x1004) with HP left, with
/// their positions. At most 64.
unsafe fn lock_on_enemies(skip: usize) -> Vec<(u32, [f32; 3])> {
    lock_on_list(skip, true)
}

/// Re:CoM's lock-on list's enemies (type 0x1004, HP left), with or without its lock-on flag
/// (`+0x154 & 0x10000`: Thunder's area test reads the bodies instead, as KH1's sys 90 does).
unsafe fn lock_on_list(skip: usize, lockable_only: bool) -> Vec<(u32, [f32; 3])> {
    type TypeFn = unsafe extern "win64" fn(usize) -> u32;
    let mut out = Vec::new();
    let m: usize = rd(va(base(), LOCK_ON_MANAGER));
    if m == 0 {
        return out;
    }
    let mut node: usize = rd(m + rt::LIST);
    let mut guard = 0;
    while node != 0 && guard < 256 && out.len() < 64 {
        let next: usize = rd(node + rt::NODE_NEXT);
        if next == 0 {
            break;
        }
        let h: u32 = rd(node);
        let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h);
        if obj != 0 && obj != skip && (!lockable_only || rd::<u32>(obj + rt::ACTOR_FLAGS) & rt::LOCKABLE != 0) {
            let vt: usize = rd(obj);
            let ty: TypeFn = std::mem::transmute_copy(&rd::<usize>(vt + crate::battle::enemy::VT_TYPE));
            let mover: usize = rd(obj + rt::MOVER);
            if ty(obj) == crate::battle::ENEMY_TYPE && rd::<i32>(obj + crate::battle::enemy::HP) > 0 && mover != 0 {
                let p = rd_vec(mover + rt::POSITION);
                out.push((h, [p.x, p.y, p.z]));
            }
        }
        node = next;
        guard += 1;
    }
    out
}

/// The flight-state exe of our balls cast with no target (7.4b): KH1's pick for this frame
/// (`crate::magic::retarget::pick` on the ball's position and its direction before this frame's
/// turn) into Re:CoM's homing target `+0x2F4` (0: none, the ball flies straight this frame), then
/// Re:CoM's own homing step (the thunk → `0x1401575D0`: the ≤ 3° turn toward the pick's aim
/// point, the velocity). Runs only for these balls, once a frame each.
pub unsafe extern "win64" fn kh1_ball_exe(bullet: usize) {
    use crate::guard::bullet as gb;
    let mover: usize = rd(bullet + gb::MOVER);
    let live = rd::<i32>(bullet + rt::HOLD) == 0 && rd::<i32>(bullet + rt::LIFE) > 0 && mover != 0;
    let pick = if live {
        let p = rd_vec(mover + rt::POSITION);
        let dir = crate::guard::forward(rd(mover + gb::PITCH), rd(mover + gb::YAW));
        mg::retarget::pick([p.x, p.y, p.z], dir, lock_on_enemies(bullet))
    } else {
        None
    };
    wr(bullet + gb::HOMING_TARGET, pick.map_or(0, |p| p.0 as i32));
    if live {
        let id: u32 = rd(bullet + 0x20);
        let h = pick.map_or(0, |p| p.0);
        let changed = SEEKERS
            .lock()
            .ok()
            .and_then(|mut g| g.iter_mut().find(|e| e.0 == id).map(|e| std::mem::replace(&mut e.1, h) != h))
            .unwrap_or(false);
        if changed {
            let t = match pick {
                Some((h, d)) => format!("rt shot {id} -> {h} ({d:.2} m)"),
                None => format!("rt shot {id} -> none"),
            };
            event(t.clone());
            rt_line(|| format!("re-target: {t}"));
        }
    }
    func::<PlFn>(HOMING_THUNK)(bullet)
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
        if let Some(&(_, s, l, _)) = g.iter().find(|&&(id, _, _, _)| id == by) {
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
        "magic: KH1's cardless cast at combat = kh1 in Sora's story (M7 stages 7.4-7.9): Fire / Fira / Firaga on Re:CoM's magic action 0x13 with KH1 rows ({} rows), KH1's release ({} frames after the wind-up / {} on the fast path), KH1's ball (speed {} cm/f, life {} f, turn {}°/f, radius 50 cm) through Re:CoM's CHomingBullet, MP {} units paid at the release; Thunder / Thundara / Thundaga (7.6): at 30, every enemy whose bodies touch a {} m, ±{} m cylinder, nearest first, struck through Re:CoM's own hit path one per {} frames, 1 MP; Blizzard / Blizzara / Blizzaga (7.5): 7 shards on Re:CoM's Blizzard shots, KH1's cone, speed 20/30/40 cm/f, life {} f, 50 cm glide, one hit per enemy per cast; Cure / Cura / Curaga (7.7): at {}, Re:CoM's heal {:?} HP on Sora, 1 MP; Gravity / Gravira / Graviga (7.8): at {}, Re:CoM's sphere 2.4 m above the target falls 2.3 m over {:?} f growing to {:?}, then strikes every enemy within 170 x scale cm once (current HP x (power + 5 x max MP)% x resist[8]), 1 MP; Stop / Stopra / Stopga (7.9): at {}, every enemy in a {:?} cm, +-300 cm cylinder, nearest first, one every {} f, into Re:CoM's stop state for (2 + max MP) x 60 x resist[9] f (0: immune), 2 MP; Field exe case 0x13 -> exeMagic; KH1's re-target for balls cast with no target (best enemy within {} m of the ball, each frame): {}",
        TABLE.get().map_or(0, |t| t.iter().filter(|r| r[0] != 0).count()),
        28,
        mg::fire::FAST_WAIT,
        mg::fire::SPEED,
        mg::fire::LIFE,
        mg::fire::TURN_DEG,
        mg::fire::COST,
        mg::thunder::RADIUS / 100.0,
        mg::thunder::HALF_HEIGHT / 100.0,
        mg::thunder::INTERVAL,
        mg::blizzard::LIFE,
        mg::cure::WAIT,
        mg::cure::HEAL,
        mg::gravity::WAIT,
        mg::gravity::FRAMES,
        mg::gravity::SCALE,
        mg::stop::WAIT,
        mg::stop::RADIUS,
        mg::stop::INTERVAL,
        mg::retarget::RANGE_M,
        if RETARGET.load(Ordering::Acquire) { "on" } else { "off" }
    )
}
