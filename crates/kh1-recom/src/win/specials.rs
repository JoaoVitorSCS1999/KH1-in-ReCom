//! KH1's specials on Re:CoM's own sleight action `0x16` (M9 stage 9c.1, Sonic Blade;
//! `crate::specials`, `docs/recom-m9-abilities.md` § 9c.1).
//!
//! - **Command:** the command menu's context tab (Triangle) calls [`issue`]; Sora's KH1 frame of
//!   the same frame takes it (`crate::specials` on `Kh1Sora`), else it is dropped at the next frame.
//! - **Start** ([`start_special`]): KH1's MP paid on the press frame (`0x14028D800` →
//!   `0x1402A5B60`); then `initStockSkill`'s (`0x140220DF0`) own writes for Sonic Blade (case 9),
//!   cardless (it reads the played card's `BattleCardList` row for the AP, loaded only for a card
//!   battle): Re:CoM's action `0x16`, its attack id 0xE, its sleight resources
//!   (`0x140211790(+0x3B8, 9, 0)`), and Re:CoM's own Sonic Blade start effect (`limit1` effect 0 on
//!   the bone `+0x3A4`, as its `exeStockSkill` plays it at sub-state 0x8C).
//! - **Each row** ([`special_row`]): the last row's weapon volume ends, `+0x3C8` = our KH1 row,
//!   `+0x638` = Re:CoM's Sonic Blade id of it (0xF dash, 0x10 finisher: its own hit whoosh and
//!   effects), the AP / kind fields, the hit fields cleared.
//! - **Each KH1 motion** ([`motion_events`]): base Re:CoM's voices, sounds and effects: the
//!   events of Re:CoM's own sleight motion for it (start, dash, finisher), started on Sora's sleight
//!   event players as `exeStockSkill`'s `0x1401F73A0` → `0x1401A3580` starts them
//!   (`crate::specials::recom_events`); stopped at the end.
//! - **Battle only** (owner, 2026-11-01): Re:CoM's Battle exe calls `exeStockSkill` through Sora's
//!   vtable (the Field exe has no case for it and is not patched; the menu offers no special
//!   outside battle, [`start_special`] refuses one); our wrapper runs KH1's frame for a KH1
//!   special and then Re:CoM's own `procAttackCollision` (`0x1401F5DC0`) on the row with the
//!   motion's frame, as `exeStockSkill` and the swing exe do (its window, the weapon's attack, our
//!   KH1 spheres in the pair dispatch, KH1's damage in the enemy-hit wrapper); Re:CoM's
//!   `exeStockSkill` for anything else (its sleight cards, untouched).
//! - **The end**: Re:CoM's `0x1401F6660`, as `exeStockSkill` ends a sleight.

use super::movement::{self as mv, func, orig, p, rd, wr};
use super::{base, log, va};
use crate::specials::{self as sp, RowStart, SpecialEvent, SpecialStart};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_EXE_STOCK: AtomicUsize = AtomicUsize::new(0);

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The player whose sleight action is a KH1 special (set by [`start_special`], cleared by
/// [`end_special`]).
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
/// The command issued this frame (the menu's), for the KH1 frame.
static PENDING: Mutex<Option<u8>> = Mutex::new(None);
/// Re:CoM's `limit1` effect name is where `exeStockSkill` reads it.
static EFFECT_OK: AtomicBool = AtomicBool::new(false);
/// Re:CoM's start tint and its end are as read (`sites::SPECIAL_TINT_CHECKED`).
static TINT_OK: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Re:CoM's end action and ground test (checked by earlier stages); `procAttackCollision` and the
/// sleight resource load (`sites::SPECIAL_CHECKED`).
const END_ACTION: u32 = 0x001F_6660;
const GROUND_TEST: u32 = 0x001F_3C00;
const PROC_ATTACK: u32 = 0x001F_5DC0;
const SLEIGHT_LOAD: u32 = 0x0021_1790;
/// The homing target (`0x1401F68A0`, `sites::MAGIC_CALLEES`) and the weapon's attack end
/// (`0x140224FB0`, `sites::ATTACK_CALLEES`).
const HOMING: u32 = 0x001F_68A0;
const WEAPON_END: u32 = 0x0022_4FB0;
/// Re:CoM's base gravity (`initStockSkill` writes it to `+0x5D8`) and its recovery value for a
/// sleight (`DAT_1407C784C` = 4000 → `+0x65C`). Checked (decomp + exe).
const BASE_GRAVITY: u64 = 0x1_4087_B398;
const RECOVERY: u64 = 0x1_407C_784C;
/// Re:CoM's attack rows (`+0x3C8` = `0x1407C6C70 + id × 16`, `initStockSkill`): the wind-up keeps
/// its row 0xE (no window: no hit, as KH1's wind-up). Checked (decomp + exe).
const RECOM_ROWS: u64 = crate::attack::RECOM_ATTACK_ROWS;
/// `"limit1"`'s pointer, as `exeStockSkill` passes it (`0x140218EEC`: `mov rdx,[0x1407C7E10]`).
/// Checked (exe: the string at `0x1405A30D8`).
const LIMIT1_PTR: u64 = 0x1_407C_7E10;

/// Re:CoM's screen tint `0x1401F7D80(player, colour, time, flags)` and what its Sonic Blade start
/// passes (`exeStockSkill` `0x140218E05` → `0x140218D0E`): the colour `0x1407C5D50` (0.5, 0.5, 0.75,
/// 1), the time `[0x1406C1208]` = 20.0 (`xmm6`, `0x140218CA9`), flags 0. Its end action
/// `0x1401F6660` (`end_special`) fades back (`0x1401F66D4`). Checked (decomp + disasm + exe).
const TINT: u32 = 0x001F_7D80;
const TINT_COLOUR: u64 = 0x1_407C_5D50;
const TINT_TIME: u64 = 0x1_406C_1208;
type TintFn = unsafe extern "win64" fn(usize, usize, f32, u32);

/// Sora's event players and the functions `0x1401A3580` starts a sleight motion's events with
/// (`crate::specials::recom_events`; `sites::SPECIAL_CHECKED`). Checked (decomp + exe disasm of
/// `0x1401A3580`).
const EET_SET: u32 = 0x0017_9A30;
const EET_START: u32 = 0x0017_A240;
const SES_SET: u32 = 0x0023_8710;
const SES_START: u32 = 0x0023_9260;
/// Whether a sleight's resources are loaded (`0x1402106E0(+0x3B8, sleight)` = 1), as
/// `exeStockSkill` tests before its start (logged only: KH1's timing can't wait).
const RES_READY: u32 = 0x0021_06E0;
mod ev {
    pub const OWN_EET: usize = 0x108;
    pub const SLEIGHT_EET: usize = 0x110;
    pub const OWN_SES: usize = 0x118;
    pub const SLEIGHT_SES: usize = 0x120;
    pub const MUTE: usize = 0x154;
    pub const NO_EET: u32 = 0x400;
    pub const NO_SES: u32 = 0x4000;
    /// The motion controller's flags: `0x200` = a sub-pack motion plays (`0x14024C760`).
    pub const CTRL_FLAGS: usize = 0x52;
    pub const SUB_PACK: u16 = 0x200;
}

/// `CPlayer` fields `initStockSkill` writes (Checked (decomp `0x140220DF0`)).
mod f {
    pub const PREV_ACTION: usize = 0x408;
    pub const ACTION: usize = 0x40C;
    pub const ACTION_TIME: usize = 0x410;
    pub const SUB: usize = 0x414;
    pub const FLAGS_418: usize = 0x418;
    pub const REQUEST: usize = 0x424;
    pub const CARD: usize = 0x428;
    pub const F454: usize = 0x454;
    pub const F478: usize = 0x478;
    pub const F498: usize = 0x498;
    pub const SPEED_4DC: usize = 0x4DC;
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
    pub const F670: usize = 0x670;
    pub const CARD_7B4: usize = 0x7B4;
    pub const SLEIGHTS: usize = 0x3B8;
    pub const EFFECT_BONE: usize = 0x3A4;
    pub const ROW_PTR: usize = 0x3C8;
    pub const WEAPON: usize = 0x3D0;
    pub const ATTACK_STAT: usize = 0x43C;
}

type PlFn = unsafe extern "win64" fn(usize);
type PlIntFn = unsafe extern "win64" fn(usize, i32) -> u64;
type GroundFn = unsafe extern "win64" fn(usize) -> u32;
type LoadFn = unsafe extern "win64" fn(usize, u32, u32);
type SetPackFn = unsafe extern "win64" fn(usize, u32) -> u64;
type EvStartFn = unsafe extern "win64" fn(usize, u32, u32, u32) -> u8;
type ReadyFn = unsafe extern "win64" fn(usize, u32) -> i32;

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("specials: {}", line()));
    }
}

fn event(e: String) {
    if let Ok(mut g) = EVENTS.lock() {
        if g.len() < 16 {
            g.push(e);
        }
    }
}

/// Checks the rows and Re:CoM's effect name (no exe patch: battle only, through the Battle exe's
/// own case). The exeStockSkill hook is installed by the caller after.
pub unsafe fn install(b: usize) -> bool {
    if !super::attack::special_rows_ready() {
        log("specials: Sonic Blade's rows are not in the attack table (no xl_limit clips: re-run kh1-recom-build); no KH1 specials");
        return false;
    }
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    // The Re:CoM code the specials call or copy (and its Sonic Blade rows), as read from the Steam
    // exe.
    for c in crate::sites::SPECIAL_CHECKED {
        if have(b + c.rva as usize, c.expect.len()) != c.expect {
            log(&format!("specials: {} is not as expected; no KH1 specials", c.name));
            return false;
        }
    }
    // Re:CoM's start tint (owner, 2026-11-01): only the tint is left out if it isn't as read.
    let tint = crate::sites::SPECIAL_TINT_CHECKED.iter().all(|c| have(b + c.rva as usize, c.expect.len()) == c.expect);
    TINT_OK.store(tint, Ordering::Release);
    if !tint {
        log("specials: Re:CoM's Sonic Blade tint code is not as read: Sonic Blade without Re:CoM's start tint");
    }
    let name: usize = rd(va(b, LIMIT1_PTR));
    let ok = name != 0 && have(name, 7) == b"limit1\0";
    EFFECT_OK.store(ok, Ordering::Release);
    if !ok {
        log("specials: Re:CoM's effect name at 0x1407C7E10 is not limit1: Sonic Blade without Re:CoM's start effect");
    }
    true
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

/// The specials are in (the command menu offers them only then).
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// A special command from the command menu's context tab: taken by Sora's KH1 frame this frame, if
/// it runs (else dropped at the next frame's start).
pub fn issue(c: u8) {
    if !INSTALLED.load(Ordering::Acquire) {
        note(|| format!("{c:#x}: no KH1 specials installed"));
        return;
    }
    if let Ok(mut g) = PENDING.lock() {
        *g = Some(c);
    }
}

/// The start of Sora's frame (the request-bits wrapper, before the menu): a command no KH1 frame
/// took is gone.
pub fn new_frame() {
    if let Ok(mut g) = PENDING.try_lock() {
        if let Some(c) = g.take() {
            note(|| format!("{c:#x}: dropped (no KH1 frame ran: Re:CoM's action)"));
        }
    }
}

/// The command for this KH1 frame (`win::movement::run_frame`).
pub fn take_command() -> Option<u8> {
    PENDING.try_lock().ok().and_then(|mut g| g.take())
}

/// Whether this player's sleight action is a KH1 special.
pub unsafe fn kh1_special(pl: usize) -> bool {
    pl != 0 && ACTIVE.load(Ordering::Acquire) == pl
}

/// KH1's start (`0x14028D800`: the MP on the press frame), then `initStockSkill`'s writes for
/// Sonic Blade, cardless, in its order (Checked (decomp `0x140220DF0`)). False (nothing changed)
/// when KH1's MP can't pay.
pub unsafe fn start_special(pl: usize, s: &SpecialStart) -> bool {
    // Battle only (owner): the Field exe never runs action 0x16.
    if rd::<i32>(pl + p::BATTLE) == 0 {
        note(|| format!("special {}: not in battle", s.index));
        return false;
    }
    if !super::mp::can_pay(s.cost) || !super::mp::pay(s.cost, "Sonic Blade") {
        note(|| format!("special {}: MP short ({})", s.index, s.cost));
        return false;
    }
    let cur: u32 = rd(pl + f::ACTION);
    wr(pl + f::PREV_ACTION, cur);
    wr(pl + f::ACTION, sp::RECOM_ACTION);
    wr(pl + f::ACTION_TIME, 0u32);
    wr(pl + f::CARD, rd::<u32>(pl + f::CARD) & 0xFFF7_FFF7);
    wr(pl + f::FLAGS_418, rd::<u32>(pl + f::FLAGS_418) & 0xF7FD_FFFF);
    wr(pl + f::COUNTER, 0u32);
    wr(pl + f::PREV_SITUATION, 0u32);
    wr(pl + f::HIT, 0u64);
    wr(pl + f::CHAIN, 0u64);
    wr(pl + f::CARD_7B4, 0u32);
    wr(pl + f::RECOVERY, rd::<u32>(va(base(), RECOVERY)));
    wr(pl + f::F454, 0u32);
    wr(pl + f::F498, 0u64);
    // `0x1401A38D0`: the body's velocity to zero (the KH1 frame writes its own after).
    let body: usize = rd(pl + p::BODY);
    if body != 0 {
        wr(body + 0x80, [0f32, 0.0, 0.0, 1.0]);
    }
    wr(pl + f::SITUATION, 8u32);
    wr(pl + f::FLAGS_418, rd::<u32>(pl + f::FLAGS_418) & 0xFD7F_FFFF);
    wr(pl + f::GRAVITY, rd::<f32>(va(base(), BASE_GRAVITY)));
    wr(pl + f::PHASE, sp::RECOM_PHASE);
    wr(pl + f::SPEED_4DC, 0u64);
    wr(pl + f::F478, 0u32);
    func::<PlFn>(HOMING)(pl);
    // Case 9: the attack id 0xE and the sleight's resources (its effects).
    wr(pl + f::ATTACK_ID, sp::recom::ID_START);
    let sleights: usize = rd(pl + f::SLEIGHTS);
    if sleights != 0 {
        func::<LoadFn>(SLEIGHT_LOAD)(sleights, sp::recom::SLEIGHT as u32, 0);
        // Re:CoM waits for them (`exeStockSkill` sub-state 4, motion 0x2E); KH1 starts at once:
        // until they are in, Re:CoM's sleight events find nothing (logged).
        let ready = func::<ReadyFn>(RES_READY)(sleights, sp::recom::SLEIGHT as u32);
        if ready != 1 {
            note(|| format!("start: Re:CoM's Sonic Blade resources not loaded yet ({ready}): its sleight events may be missing at first"));
        }
    }
    wr(pl + f::ROW_PTR, va(base(), RECOM_ROWS) + sp::recom::ID_START as usize * 16);
    // Re:CoM's cardless AP (the attack stat), as KH1 swings: at Damage = KH1 KH1's damage replaces
    // it per hit; at Damage = Re:CoM `initStockSkill`'s own AP for the start row (no hit window).
    wr(pl + f::AP, rd::<i32>(pl + f::ATTACK_STAT));
    wr(pl + f::KIND, 0u32);
    wr(pl + f::ATTR, 0u32);
    super::damage::special(pl, sp::recom::ID_START);
    wr(pl + f::REQUEST, rd::<u32>(pl + f::REQUEST) & 0xFFFB_FFFF);
    wr(pl + f::PREV_SITUATION, 8u32);
    wr(pl + f::SUB, sp::RECOM_SUB);
    wr(pl + f::F670, 0u64);
    ACTIVE.store(pl, Ordering::Release);
    // Re:CoM's start tint (owner, 2026-11-01: "Add Re:CoM's tint"), as its SONICRAVE start makes it
    // on the press, before its `limit1` effect: `0x1401F7D80(player, 0x1407C5D50, 20.0, 0)`. It sets
    // `+0x418` bit 31; Re:CoM's end action (`end_special` → `0x1401F6660`), and every other Re:CoM
    // path that tests the bit, fade it back.
    if TINT_OK.load(Ordering::Acquire) {
        let time: f32 = rd(va(base(), TINT_TIME));
        func::<TintFn>(TINT)(pl, va(base(), TINT_COLOUR), time, 0);
        event("tint".into());
    }
    // Re:CoM's own Sonic Blade start effect (`exeStockSkill` sub-state 0x8C: `0x1401F7F90(player,
    // "limit1", 0, bone +0x3A4)`, kept on Sora), at KH1's start (owner: Re:CoM's sleight effects).
    if EFFECT_OK.load(Ordering::Acquire) {
        let name: usize = rd(va(base(), LIMIT1_PTR));
        super::magic::effect_on_bone(pl, pl, rd::<i32>(pl + f::EFFECT_BONE), name, 0, &[0.0, 0.0, 0.0, 1.0], true);
    }
    event("start".into());
    true
}

/// A special row (`0x14039ECA0`) on Re:CoM's sleight action: the last row's volume ends (a new
/// attack: one hit per enemy per row, KH1's new attack object), our KH1 row, Re:CoM's id of it.
pub unsafe fn special_row(pl: usize, r: &RowStart) {
    let Some(row) = super::attack::row_ptr(r.kh1_row) else {
        note(|| format!("row {:#x}: not in the attack table", r.kh1_row));
        return;
    };
    let w: usize = rd(pl + f::WEAPON);
    if w != 0 && rd::<i32>(w + 0x23C) != 0 {
        func::<PlFn>(WEAPON_END)(w);
    }
    wr(pl + f::ROW_PTR, row);
    wr(pl + f::ATTACK_ID, r.recom_id);
    wr(pl + f::AP, rd::<i32>(pl + f::ATTACK_STAT));
    wr(pl + f::KIND, rd::<u8>(row + 0xB) as u32);
    wr(pl + f::ATTR, 0u32);
    // Damage = Re:CoM (owner, 2026-11-01): Re:CoM's Sonic Blade sleight AP for this part
    // (`exeStockSkill`: dash row 0xF column 0, finisher row 0x10 column 2), its attribute, the
    // enemy-card rewrite.
    super::damage::special(pl, r.recom_id);
    wr(pl + f::HIT, 0u64);
    wr(pl + f::CHAIN, 0u64);
    super::attack::new_attack();
    event(format!("row {:#x} n{}", r.kh1_row, r.counter));
}

/// A KH1 motion of the special started (`RecomHost::play`, right after Re:CoM's motion request):
/// the events of Re:CoM's sleight motion it stands for, as `0x1401A3580` starts them after a
/// sleight motion: Sora's own event players stopped (the clip just played is one of his own:
/// `ctrl+0x52 & 0x200` clear), the sleight's players set to the Sonic Blade pack and started on
/// the motion with the variants `exeStockSkill` passes. Base Re:CoM's voices, sounds and effects
/// (owner), on KH1's clip and clock.
pub unsafe fn motion_events(pl: usize, xl_entry: usize) {
    use sp::recom_events as re;
    if ACTIVE.load(Ordering::Acquire) != pl {
        return;
    }
    let Some(motion) = re::motion_of(xl_entry) else { return };
    let c: usize = rd(pl + p::CTRL);
    if c == 0 {
        return;
    }
    if rd::<u16>(c + ev::CTRL_FLAGS) & ev::SUB_PACK == 0 {
        for at in [ev::OWN_EET, ev::OWN_SES] {
            stop_player(rd(pl + at));
        }
    }
    let mute: u32 = rd(pl + ev::MUTE);
    let (mut eet, mut ses) = (false, false);
    let e: usize = rd(pl + ev::SLEIGHT_EET);
    if e != 0 {
        func::<SetPackFn>(EET_SET)(e, re::PACK);
        let e: usize = rd(pl + ev::SLEIGHT_EET);
        if e != 0 && mute & ev::NO_EET == 0 {
            eet = func::<EvStartFn>(EET_START)(e, motion, re::EET_VARIANT, 1) & 1 != 0;
        }
    }
    let s: usize = rd(pl + ev::SLEIGHT_SES);
    if s != 0 {
        func::<SetPackFn>(SES_SET)(s, re::PACK);
        let s: usize = rd(pl + ev::SLEIGHT_SES);
        if s != 0 && mute & ev::NO_SES == 0 {
            ses = func::<EvStartFn>(SES_START)(s, motion, re::SES_VARIANT, 1) & 1 != 0;
        }
    }
    event(format!("ev m{motion}{}{}", if eet { " eet" } else { "" }, if ses { " ses" } else { "" }));
    note(|| format!("Re:CoM's Sonic Blade events: pack {} motion {motion} (xl entry {xl_entry}): effects {}, sounds {}", re::PACK, if eet { "started" } else { "none" }, if ses { "started" } else { "none" }));
}

/// A player's stop (its vtable `+0x10`, as `0x1401A3470` / `0x1401A3580` call it).
unsafe fn stop_player(obj: usize) {
    type StopFn = unsafe extern "win64" fn(usize);
    if obj != 0 {
        let f: StopFn = std::mem::transmute_copy(&rd::<usize>(rd::<usize>(obj) + 0x10));
        f(obj);
    }
}

/// The special's end: the sleight's event players stop, as Re:CoM's next own motion stops them
/// after a sleight (`0x1401A3470` with the sub-pack flag set; ours never sets it).
unsafe fn stop_events(pl: usize) {
    for at in [ev::SLEIGHT_EET, ev::SLEIGHT_SES] {
        stop_player(rd(pl + at));
    }
}

/// Re:CoM ends its sleight action (`0x1401F6660(player, airborne)`). True when it entered its win
/// action instead.
pub unsafe fn end_special(pl: usize, airborne: bool) -> bool {
    ACTIVE.store(0, Ordering::Release);
    stop_events(pl);
    let w: usize = rd(pl + f::WEAPON);
    if w != 0 && rd::<i32>(w + 0x23C) != 0 {
        func::<PlFn>(WEAPON_END)(w);
    }
    func::<PlIntFn>(END_ACTION)(pl, airborne as i32);
    mv::in_recom_win(pl)
}

/// `exeStockSkill` (`0x140218950`, action `0x16`): KH1's frame and Re:CoM's collision for a KH1
/// special, Re:CoM's exe otherwise (its sleights, untouched).
pub unsafe extern "win64" fn exe_stock(pl: usize) {
    let o: PlFn = orig(&ORIG_EXE_STOCK);
    if !mv::kh1_sora(pl) || ACTIVE.load(Ordering::Acquire) != pl {
        return o(pl);
    }
    if !mv::continues(pl) {
        // Re:CoM ran something else in between: end its action and take over from Re:CoM's state.
        if end_special(pl, func::<GroundFn>(GROUND_TEST)(pl) != 0) {
            return;
        }
        return mv::run_frame(pl, true);
    }
    mv::run_frame(pl, false);
    // Re:CoM's own collision for the row, with the clock of the last motion update, as its exes
    // call it (`procAttackCollision(player, time / 50)`; the row holds KH1's window one frame
    // earlier, `crate::attack::HIT_LEAD`).
    if ACTIVE.load(Ordering::Acquire) == pl && rd::<u32>(pl + p::ACTION) == sp::RECOM_ACTION && super::attack::special_row_live(pl) {
        let c: usize = rd(pl + p::CTRL);
        if c != 0 {
            let frame = rd::<i32>(c + 0x3C) / 50;
            func::<PlIntFn>(PROC_ATTACK)(pl, frame);
        }
    }
}

/// A confirm KH1's commit refused during a special (the menu's): the attacking bit goes.
pub unsafe fn confirm_refused(pl: usize) {
    if let Ok(mut g) = mv::SORA.try_lock() {
        if let Some(s) = g.as_mut().filter(|s| s.player == pl) {
            if s.sora.special.is_some() {
                s.sora.special_confirm_refused();
                event("bit off".into());
            }
        }
    }
}

/// The frame's special events (from `win::movement::run_frame`): log and trace.
pub fn on_events(ev: &[SpecialEvent]) {
    for e in ev {
        let t = e.token();
        event(t.clone());
        note(|| t);
    }
}

/// The F9 trace's special token: the running special (the recorder holds Sora's lock) and this
/// frame's events.
pub fn trace_token(s: Option<&sp::Special>) -> String {
    let ev = EVENTS.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
    let mut t = sp::trace_token(s);
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
        "specials: KH1's Sonic Blade at combat = kh1 in Sora's story, in battle (M9 stage 9c.1): offered in the context tab (KH1's 0x14028D0F0: equipped, target gap < 600, not 160 above, 2 MP), Triangle starts it on Re:CoM's sleight action 0x16 (initStockSkill's writes, cardless; MP on the press), KH1's wind-up / 6 dashes / finisher (rows 0x1B..=0x21) on the xl_limit clips, Rave / Blast in the Attack row; Re:CoM's Sonic Blade ids 0xE / 0xF / 0x10 (its hit sounds and effects), its limit1 start effect{}, its sleight motions' sound and effect events (pack 11009 motions 0 / 1 / 2) on KH1's motions; the Battle exe's own case 0x16 -> exeStockSkill",
        if EFFECT_OK.load(Ordering::Acquire) { "" } else { " (OFF: name not found)" }
    )
}
