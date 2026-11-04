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
//! - **Battle and field** (owner, 2026-11-01: "Only in battle"; lifted 2026-11-04, "Sleights outside
//!   battle"): Re:CoM's Battle exe calls `exeStockSkill` through Sora's vtable, the Field exe through
//!   its case 0x16 (`win::field_sleights`; free there, a hit on a field Heartless Re:CoM's field
//!   strike); our wrapper runs KH1's frame for a KH1
//!   special and then Re:CoM's own `procAttackCollision` (`0x1401F5DC0`) on the row with the
//!   motion's frame, as `exeStockSkill` and the swing exe do (its window, the weapon's attack, our
//!   KH1 spheres in the pair dispatch, KH1's damage in the enemy-hit wrapper); Re:CoM's
//!   `exeStockSkill` for anything else (its sleight cards, untouched).
//! - **The end**: Re:CoM's `0x1401F6660`, as `exeStockSkill` ends a sleight.
//! - **Strike Raid** (M12 step 7): `initStockSkill`'s case 8 (attack id 0xD, no resource load: Re:CoM's
//!   Strike Raid plays Sora's own motions), Re:CoM's Strike Raid start tint (`0x1407C5D70`, 20.0),
//!   no `limit` effect (Re:CoM plays none for it); each throw on id 0xD; the events of Re:CoM's own
//!   `p001_074` / `p001_075` / `p001_076` on KH1's wind-up / throws / catches (Sora's own event
//!   players, [`motion_events`]); a throw's attack ends with its motion ([`special_row_end`]).
//! - **Ars Arcanum** (M12 step 7): `initStockSkill`'s case 10 (attack id 0x11, its resources
//!   `0x140211790(+0x3B8, 10, 0)`), Re:CoM's Ars Arcanum start tint (`0x1407C5D60`, 20.0) and its
//!   `limit2` start effect; KH1's row `0x22 + n` on Re:CoM's id `0x12 + n`; the events of its pack
//!   11010 (motions 0..=13) on KH1's motions.

use super::movement::{self as mv, func, orig, p, rd, rd_vec, wr};
use super::{base, log, va};
use crate::specials::{self as sp, RowStart, SpecialEvent, SpecialStart};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_EXE_STOCK: AtomicUsize = AtomicUsize::new(0);

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// M12 step 7: each special's rows and Re:CoM sites are in (by `crate::specials` index).
static READY: [AtomicBool; 5] = [const { AtomicBool::new(false) }; 5];
/// The player whose sleight action is a KH1 special (set by [`start_special`], cleared by
/// [`end_special`]).
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
/// The command issued this frame (the menu's), for the KH1 frame.
static PENDING: Mutex<Option<u8>> = Mutex::new(None);
/// Re:CoM's `limit1` effect name is where `exeStockSkill` reads it.
static EFFECT_OK: AtomicBool = AtomicBool::new(false);
/// M12 step 7: `"limit2"` is where `exeStockSkill`'s Ars Arcanum start reads it.
static EFFECT2_OK: AtomicBool = AtomicBool::new(false);
/// M12 step 7: `"limit3"` is where Ragnarok's start reads it.
static EFFECT3_OK: AtomicBool = AtomicBool::new(false);
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

/// M12 step 7: Re:CoM's Strike Raid start tint: `exeStockSkill`'s start for attack id 0xD (× 10 =
/// 0x82, `0x140218E62` → `0x140218F28`: `lea rdx,[0x1407C5D70]` (0.6, 0.5, 0.75, 1), then
/// `0x140218F4A`: the same call with xmm6 = 20.0, flags 0; `sites::SPECIAL_SR_CHECKED`). Checked
/// (decomp + disasm + exe).
const TINT_COLOUR_RAID: u64 = 0x1_407C_5D70;
/// M12 step 7: Re:CoM's Ars Arcanum start tint: its start row 0x11 has flag 0x20 (`+0xF` = 0x60):
/// the colour by id × 10 = 0xAA (`0x140218CF2` → `0x140218DF9`: `lea rdx,[0x1407C5D60]` (0.75,
/// 0.5, 0.6, 1)), then the call at `0x140218D0E` (20.0, flags 0); `sites::SPECIAL_AA_CHECKED`.
const TINT_COLOUR_ARCANUM: u64 = 0x1_407C_5D60;
/// `"limit2"`'s pointer, as `exeStockSkill`'s 0xAA start passes it (`PTR_s_limit2_1407c7e18`).
const LIMIT2_PTR: u64 = 0x1_407C_7E18;
/// M12 step 7: Re:CoM's Ragnarok start tint (id × 10 = 0x136: `0x140218CFB` → `0x140218E29`: `lea
/// rdx,[0x1407C5D90]` (0.5, 0.75, 0.5, 1)) and its start effect `"limit3"` (`0x1407C7E20` → the
/// string at `0x1405A30E8`); `sites::SPECIAL_RAG_CHECKED`. Checked (decomp + disasm + exe).
const TINT_COLOUR_RAGNAROK: u64 = 0x1_407C_5D90;
const LIMIT3_PTR: u64 = 0x1_407C_7E20;
/// Re:CoM's shot class (2 = `CHomingBullet`) and descriptor kind (3), as its Ragnarok burst passes
/// them (`exeStockSkill`: `0x140156790(desc, 2, 3)`). Checked (decomp).
const RAGNAROK_CLASS: i32 = 2;
/// Sora's own action table's per-variant row lists (`0x1401F7180`: `[0x1407C6300 + 8 ×
/// player+0x398]` + row × 16 + 8 = the row's slot in his pack). Checked (disasm `0x1401F71BB..`).
const OWN_ROW_LISTS: u64 = 0x1_407C_6300;
const MOTION_VARIANT: usize = 0x398;
/// Re:CoM's empty attack row 0xE (Sonic Blade's start: no window), for a row whose attack is over.
const EMPTY_ROW: u32 = 0xE;

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
    for i in sp::BUILT {
        let ok = super::attack::special_rows_ready(i);
        READY[i].store(ok, Ordering::Release);
        if !ok {
            log(&format!("specials: {}'s rows are not in the attack table (no xl_limit clips: re-run kh1-recom-build); no {} at combat = kh1", sp::NAME[i], sp::NAME[i]));
        }
    }
    if !READY.iter().any(|r| r.load(Ordering::Acquire)) {
        log("specials: no special's rows are in the attack table; no KH1 specials");
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
    // M12 step 7: Strike Raid's Re:CoM bits (its tint, its row 0xD, Sora's own motion play whose
    // event part `motion_events` copies): a mismatch leaves Strike Raid out.
    if READY[sp::STRIKE_RAID].load(Ordering::Acquire) && !crate::sites::SPECIAL_SR_CHECKED.iter().all(|c| have(b + c.rva as usize, c.expect.len()) == c.expect) {
        READY[sp::STRIKE_RAID].store(false, Ordering::Release);
        log("specials: Re:CoM's Strike Raid code is not as read (sites::SPECIAL_SR_CHECKED); no KH1 Strike Raid");
    }
    if READY[sp::ARS_ARCANUM].load(Ordering::Acquire) && !crate::sites::SPECIAL_AA_CHECKED.iter().all(|c| have(b + c.rva as usize, c.expect.len()) == c.expect) {
        READY[sp::ARS_ARCANUM].store(false, Ordering::Release);
        log("specials: Re:CoM's Ars Arcanum code is not as read (sites::SPECIAL_AA_CHECKED); no KH1 Ars Arcanum");
    }
    if READY[sp::RAGNAROK].load(Ordering::Acquire) && !crate::sites::SPECIAL_RAG_CHECKED.iter().all(|c| have(b + c.rva as usize, c.expect.len()) == c.expect) {
        READY[sp::RAGNAROK].store(false, Ordering::Release);
        log("specials: Re:CoM's Ragnarok code is not as read (sites::SPECIAL_RAG_CHECKED); no KH1 Ragnarok");
    }
    let name3: usize = rd(va(b, LIMIT3_PTR));
    let ok3 = name3 != 0 && have(name3, 7) == b"limit3\0";
    EFFECT3_OK.store(ok3, Ordering::Release);
    if !ok3 {
        log("specials: Re:CoM's effect name at 0x1407C7E20 is not limit3: Ragnarok without Re:CoM's start effect");
    }
    let name2: usize = rd(va(b, LIMIT2_PTR));
    let ok2 = name2 != 0 && have(name2, 7) == b"limit2\0";
    EFFECT2_OK.store(ok2, Ordering::Release);
    if !ok2 {
        log("specials: Re:CoM's effect name at 0x1407C7E18 is not limit2: Ars Arcanum without Re:CoM's start effect");
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

/// Special `index` is in (its rows and Re:CoM sites; M12 step 7).
pub fn ready(index: usize) -> bool {
    installed() && READY.get(index).is_some_and(|r| r.load(Ordering::Acquire))
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

/// A battle starts (its manager's ctor): no KH1 special runs across it (owner, 2026-11-04: a field
/// special its first strike cut; the field player is gone, and the battle's could take its address).
pub fn forget_running() {
    if ACTIVE.swap(0, Ordering::AcqRel) != 0 {
        note(|| "a battle starts: the special that was running is over (its player is gone)".into());
    }
}

/// Whether this player's sleight action is a KH1 special.
pub unsafe fn kh1_special(pl: usize) -> bool {
    pl != 0 && ACTIVE.load(Ordering::Acquire) == pl
}

/// KH1's start (`0x14028D800`: the MP on the press frame), then `initStockSkill`'s writes for
/// Sonic Blade (case 9) or Strike Raid (case 8), cardless, in its order (Checked (decomp
/// `0x140220DF0`)). False (nothing changed) when KH1's MP can't pay or the special isn't in.
pub unsafe fn start_special(pl: usize, s: &SpecialStart) -> bool {
    if !ready(s.index) {
        note(|| format!("special {}: not installed", s.index));
        return false;
    }
    let raid = s.index == sp::STRIKE_RAID;
    let arcanum = s.index == sp::ARS_ARCANUM;
    let ragnarok = s.index == sp::RAGNAROK;
    // `initStockSkill`'s case: Sonic Blade 9, Ars Arcanum 10, Strike Raid 8.
    let sleight = match s.index {
        sp::ARS_ARCANUM => sp::recom::ARCANUM_SLEIGHT,
        sp::STRIKE_RAID => sp::recom::RAID_SLEIGHT,
        sp::RAGNAROK => sp::recom::RAGNAROK_SLEIGHT,
        _ => sp::recom::SLEIGHT,
    };
    // Owner (2026-11-04, "Sleights outside battle": "this lifts the old 'only in battle' rule"): a
    // field room too (the Field exe's case 0x16, `win::field_sleights`), free there.
    let Some(field) = super::field_sleights::start_room(pl, sleight, sp::NAME[s.index]) else { return false };
    if !field && (!super::mp::can_pay(s.cost) || !super::mp::pay(s.cost, sp::NAME[s.index])) {
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
    // Case 9: the attack id 0xE and the sleight's resources (its effects). Case 8 (Strike Raid): the
    // attack id 0xD, no resources (its motions are Sora's own).
    let start_id = match s.index {
        sp::ARS_ARCANUM => sp::recom::ID_ARCANUM_START,
        sp::STRIKE_RAID => sp::recom::ID_RAID,
        sp::RAGNAROK => sp::recom::ID_RAGNAROK,
        _ => sp::recom::ID_START,
    };
    wr(pl + f::ATTACK_ID, start_id);
    let sleights: usize = rd(pl + f::SLEIGHTS);
    if sleights != 0 && !raid {
        func::<LoadFn>(SLEIGHT_LOAD)(sleights, sleight as u32, 0);
        // Re:CoM waits for them (`exeStockSkill` sub-state 4, motion 0x2E); KH1 starts at once:
        // until they are in, Re:CoM's sleight events find nothing (logged).
        let ready = func::<ReadyFn>(RES_READY)(sleights, sleight as u32);
        if ready != 1 {
            note(|| format!("start: Re:CoM's {} resources not loaded yet ({ready}): its sleight events may be missing at first", sp::NAME[s.index]));
        }
    }
    // Ragnarok: Re:CoM's row 0x1F holds its strike's window; KH1's wind-up hits nothing, so the row
    // pointer waits on the empty row 0xE until the first hit's row (`special_row`).
    let row_id = if ragnarok { EMPTY_ROW } else { start_id };
    wr(pl + f::ROW_PTR, va(base(), RECOM_ROWS) + row_id as usize * 16);
    // Re:CoM's cardless AP (the attack stat), as KH1 swings: at Damage = KH1 KH1's damage replaces
    // it per hit; at Damage = Re:CoM `initStockSkill`'s own AP for the start row (no hit window).
    wr(pl + f::AP, rd::<i32>(pl + f::ATTACK_STAT));
    wr(pl + f::KIND, 0u32);
    wr(pl + f::ATTR, 0u32);
    super::damage::special(pl, start_id);
    wr(pl + f::REQUEST, rd::<u32>(pl + f::REQUEST) & 0xFFFB_FFFF);
    wr(pl + f::PREV_SITUATION, 8u32);
    wr(pl + f::SUB, sp::RECOM_SUB);
    wr(pl + f::F670, 0u64);
    ACTIVE.store(pl, Ordering::Release);
    // Re:CoM's start tint (owner, 2026-11-01: "Add Re:CoM's tint"), as its SONICRAVE start makes it
    // on the press, before its `limit1` effect: `0x1401F7D80(player, 0x1407C5D50, 20.0, 0)`. It sets
    // `+0x418` bit 31; Re:CoM's end action (`end_special` → `0x1401F6660`), and every other Re:CoM
    // path that tests the bit, fade it back.
    // Strike Raid: its own colour (`0x140218F28`), the same time and call (its sites are part of
    // `SPECIAL_SR_CHECKED`, checked at start with the special).
    if TINT_OK.load(Ordering::Acquire) {
        let time: f32 = rd(va(base(), TINT_TIME));
        let colour = match s.index {
            sp::ARS_ARCANUM => TINT_COLOUR_ARCANUM,
            sp::STRIKE_RAID => TINT_COLOUR_RAID,
            sp::RAGNAROK => TINT_COLOUR_RAGNAROK,
            _ => TINT_COLOUR,
        };
        func::<TintFn>(TINT)(pl, va(base(), colour), time, 0);
        event("tint".into());
    }
    // Re:CoM's own Sonic Blade start effect (`exeStockSkill` sub-state 0x8C: `0x1401F7F90(player,
    // "limit1", 0, bone +0x3A4)`, kept on Sora), at KH1's start (owner: Re:CoM's sleight effects).
    // Re:CoM's Strike Raid plays no start effect (its 0x82 start only tints).
    // Ars Arcanum: `limit2` the same way (`exeStockSkill`'s 0xAA start).
    if arcanum && EFFECT2_OK.load(Ordering::Acquire) {
        let name: usize = rd(va(base(), LIMIT2_PTR));
        super::magic::effect_on_bone(pl, pl, rd::<i32>(pl + f::EFFECT_BONE), name, 0, &[0.0, 0.0, 0.0, 1.0], true);
    }
    // Ragnarok: `limit3` the same way (`exeStockSkill`'s 0x136 start).
    if ragnarok && EFFECT3_OK.load(Ordering::Acquire) {
        let name: usize = rd(va(base(), LIMIT3_PTR));
        super::magic::effect_on_bone(pl, pl, rd::<i32>(pl + f::EFFECT_BONE), name, 0, &[0.0, 0.0, 0.0, 1.0], true);
    }
    if EFFECT_OK.load(Ordering::Acquire) && !raid && !arcanum && !ragnarok {
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

/// M12 step 7: a throw's attack is over (its motion ended into the catch, or a clash): the weapon's
/// volume ends and the attack row pointer goes to Re:CoM's empty row 0xE (no window), so Re:CoM's
/// collision on the catch and the wait finds nothing (KH1's attack object goes with the throw).
pub unsafe fn special_row_end(pl: usize) {
    if ACTIVE.load(Ordering::Acquire) != pl {
        return;
    }
    let w: usize = rd(pl + f::WEAPON);
    if w != 0 && rd::<i32>(w + 0x23C) != 0 {
        func::<PlFn>(WEAPON_END)(w);
    }
    wr(pl + f::ROW_PTR, va(base(), RECOM_ROWS) + EMPTY_ROW as usize * 16);
    event("row end".into());
}

/// A KH1 motion of the special started (`RecomHost::play`, right after Re:CoM's motion request):
/// the events of Re:CoM's motion it stands for (`crate::specials::recom_events::of`). Sonic Blade: as
/// `0x1401A3580` starts them after a sleight motion: Sora's own event players stopped (the clip just
/// played is one of his own: `ctrl+0x52 & 0x200` clear), the sleight's players set to the Sonic
/// Blade pack and started on the motion with the variants `exeStockSkill` passes. Strike Raid: as
/// Sora's own motion play `0x1401A3470` starts them for Re:CoM's own clip (its slot from his action
/// table row), on his own players. Base Re:CoM's voices, sounds and effects (owner), on KH1's clip
/// and clock.
pub unsafe fn motion_events(pl: usize, xl_entry: usize) {
    use sp::recom_events as re;
    if ACTIVE.load(Ordering::Acquire) != pl {
        return;
    }
    let Some((source, eet_variant, ses_variant)) = re::of(xl_entry) else { return };
    let c: usize = rd(pl + p::CTRL);
    if c == 0 {
        return;
    }
    let (pack, motion) = match source {
        re::Source::Sleight { pack, motion } => (pack, motion),
        re::Source::Own { row } => return own_events(pl, xl_entry, row, eet_variant, ses_variant),
    };
    if rd::<u16>(c + ev::CTRL_FLAGS) & ev::SUB_PACK == 0 {
        for at in [ev::OWN_EET, ev::OWN_SES] {
            stop_player(rd(pl + at));
        }
    }
    let mute: u32 = rd(pl + ev::MUTE);
    let (mut eet, mut ses) = (false, false);
    let e: usize = rd(pl + ev::SLEIGHT_EET);
    if e != 0 {
        func::<SetPackFn>(EET_SET)(e, pack);
        let e: usize = rd(pl + ev::SLEIGHT_EET);
        if e != 0 && mute & ev::NO_EET == 0 {
            eet = func::<EvStartFn>(EET_START)(e, motion, eet_variant, 1) & 1 != 0;
        }
    }
    let s: usize = rd(pl + ev::SLEIGHT_SES);
    if s != 0 {
        func::<SetPackFn>(SES_SET)(s, pack);
        let s: usize = rd(pl + ev::SLEIGHT_SES);
        if s != 0 && mute & ev::NO_SES == 0 {
            ses = func::<EvStartFn>(SES_START)(s, motion, ses_variant, 1) & 1 != 0;
        }
    }
    event(format!("ev m{motion}{}{}", if eet { " eet" } else { "" }, if ses { " ses" } else { "" }));
    note(|| format!("Re:CoM's sleight events: pack {pack} motion {motion} (xl entry {xl_entry}): effects {}, sounds {}", if eet { "started" } else { "none" }, if ses { "started" } else { "none" }));
}

/// M12 step 7: the events of Sora's own Re:CoM clip of action row `row` (its slot: `0x1401F7180`'s
/// lookup), started on his own players as `0x1401A3470` does (`EET start(+0x108, slot, eet, 1)`
/// unless `+0x154 & 0x400`, `SES start(+0x118, slot, ses, 1)` unless `& 0x4000`).
unsafe fn own_events(pl: usize, xl_entry: usize, row: u32, eet_variant: u32, ses_variant: u32) {
    let lists: usize = rd(va(base(), OWN_ROW_LISTS) + 8 * rd::<i32>(pl + MOTION_VARIANT).clamp(0, 7) as usize);
    if lists == 0 {
        return;
    }
    let slot: i32 = rd(lists + row as usize * 16 + 8);
    if slot < 0 {
        note(|| format!("Re:CoM's Strike Raid events: row {row:#x} has no slot"));
        return;
    }
    let mute: u32 = rd(pl + ev::MUTE);
    let (mut eet, mut ses) = (false, false);
    let e: usize = rd(pl + ev::OWN_EET);
    if e != 0 && mute & ev::NO_EET == 0 {
        eet = func::<EvStartFn>(EET_START)(e, slot as u32, eet_variant, 1) & 1 != 0;
    }
    let s: usize = rd(pl + ev::OWN_SES);
    if s != 0 && mute & ev::NO_SES == 0 {
        ses = func::<EvStartFn>(SES_START)(s, slot as u32, ses_variant, 1) & 1 != 0;
    }
    event(format!("ev r{row:#x} s{slot}{}{}", if eet { " eet" } else { "" }, if ses { " ses" } else { "" }));
    note(|| format!("Re:CoM's Strike Raid events: Sora's row {row:#x} (slot {slot}, xl entry {xl_entry}, variants {eet_variant} / {ses_variant}): effects {}, sounds {}", if eet { "started" } else { "none" }, if ses { "started" } else { "none" }));
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
/// after a sleight (`0x1401A3470` with the sub-pack flag set; ours never sets it). Sora's own
/// players (Strike Raid's) are Re:CoM's next own motion's to restart.
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

/// M12 step 7: Ragnarok's shots (`0x14039F330`): each a Re:CoM `CHomingBullet` made as Re:CoM's
/// own Ragnarok burst makes its shots (`exeStockSkill`: descriptor class 2 / kind 3, the hand point
/// `0x1401F0B30`, the angles `0x1401A4A00`, effect set `0x1405A1730 + row 0x20[2] × 0x60`, the
/// shot manager `0x140158DD0`; Re:CoM steps, draws, collides, hits and ends it) with KH1's numbers:
/// its ring of directions, 12 units a frame, life 180 + rand × 60 frames, radius 50, one hit. Its
/// flight state's exe becomes [`kh1_ragnarok_exe`] (KH1's bend and 5° turn, `0x14039FBB0`), which
/// hands Re:CoM's own homing step the point to steer to. Its hit takes KH1's damage of record
/// `0x5748` at Damage = KH1 ([`take_shot_hit`]), Re:CoM's row 0x20 AP at Damage = Re:CoM.
const HAND_POINT: u32 = 0x001F_0B30;
const ANGLES: u32 = 0x001A_4A00;
/// The homing step's fields (`0x1401575D0`, Checked (decomp)): flags `+0x230` (`0x80000`: steer to
/// the point `+0x300`; `0x8000`: by angles, `0x1401D7F00` at `+0x170` degrees a frame), the gate
/// `+0x2F8` (it turns while the point is nearer).
const SHOT_FLAGS: u64 = 0x8000 | 0x100 | 4;
const POINT_FLAG: u64 = 0x8_0000;
const SHOT_TURN: usize = 0x170;
const SHOT_GATE: usize = 0x2F8;
/// Our turn is KH1's (at most 5° a frame, done before): Re:CoM's step snaps to it.
const SNAP_DEG: f32 = 180.0;

#[derive(Clone, Copy)]
struct Shot {
    id: u32,
    bend: f32,
    target: u32,
}

static SHOTS: Mutex<Vec<Shot>> = Mutex::new(Vec::new());
static SHOT_LINES: AtomicU32 = AtomicU32::new(40);

fn shot_line(line: impl FnOnce() -> String) {
    if SHOT_LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("specials: {}", line()));
    }
}

/// KH1's `0x1403A3A20`-style draw in [0, 1) for the shots' life and bend (KH1's own generator
/// state isn't Re:CoM's: Assumption, the distribution is KH1's).
fn draw() -> f32 {
    use std::sync::atomic::AtomicU64;
    static SEED: AtomicU64 = AtomicU64::new(0x2545_F491_4F6C_DD1D);
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    (x >> 40) as f32 / (1u64 << 24) as f32
}

/// "Impact": the ring of shots from Sora's hand.
pub unsafe fn ragnarok_fire(pl: usize, f: &sp::ragnarok::FireStart) {
    use crate::guard::bullet as gb;
    use crate::units::to_recom;
    use kh1_core::Vec3;
    use sp::ragnarok as rg;
    type PlOutFn = unsafe extern "win64" fn(usize, *mut [f32; 4]);
    type AnglesFn = unsafe extern "win64" fn(usize) -> *const [f32; 4];
    if ACTIVE.load(Ordering::Acquire) != pl || !ready(sp::RAGNAROK) {
        return;
    }
    // Damage = Re:CoM: row 0x20's AP, kind and attribute into the player's fields (as Re:CoM's
    // burst computes them), read into each shot; at Damage = KH1 the hit's value is KH1's.
    wr(pl + f::AP, rd::<i32>(pl + f::ATTACK_STAT));
    wr(pl + f::KIND, rd::<u8>(va(base(), RECOM_ROWS) + sp::recom::ID_RAGNAROK_SHOT as usize * 16 + 0xB) as u32);
    wr(pl + f::ATTR, 0u32);
    super::damage::special(pl, sp::recom::ID_RAGNAROK_SHOT);
    let body: usize = rd(pl + p::BODY);
    let mut hand = [0f32; 4];
    if body != 0 {
        let b = rd_vec(body + 0x30);
        hand = [b.x, b.y, b.z, 1.0];
    }
    func::<PlOutFn>(HAND_POINT)(pl, &mut hand);
    let ang = func::<AnglesFn>(ANGLES)(pl);
    let base_ang = if ang.is_null() { [0f32; 4] } else { std::ptr::read_unaligned(ang) };
    // KH1's ring in KH1's frame: up (0, −1, 0), turned about Sora's facing; his local x is (cos ψ, 0,
    // −sin ψ).
    let psi = f.facing;
    let dirs = rg::ring(Vec3::new(0.0, -1.0, 0.0), Vec3::new(psi.cos(), 0.0, -psi.sin()), f.shots);
    let target = f.target.filter(|&h| super::magic::is_target_enemy(h)).unwrap_or(0);
    let row = va(base(), RECOM_ROWS) + sp::recom::ID_RAGNAROK_SHOT as usize * 16;
    let set = rd::<u8>(row + 2) as usize;
    let mut made = 0;
    for d in dirs {
        let r = to_recom(d);
        let Some((pitch, yaw)) = crate::guard::aim_angles([r.x, r.y, r.z]) else { continue };
        let life = rg::shot_life(draw());
        let bend = rg::shot_bend(draw());
        let mut desc = super::magic::Desc([0; 0x100]);
        let dp = desc.0.as_mut_ptr();
        func::<super::magic::DescInitFn>(super::magic::DESC_INIT)(dp, RAGNAROK_CLASS, 3);
        let at = |o: usize| dp.add(o);
        std::ptr::write_unaligned(at(0x68) as *mut u32, rd::<u32>(pl + f::KIND));
        std::ptr::write_unaligned(at(0x6C) as *mut u32, rd::<u32>(pl + f::ATTR));
        std::ptr::write_unaligned(at(0x70) as *mut i32, rd::<i32>(pl + f::AP));
        std::ptr::write_unaligned(at(0x0C) as *mut u32, 0x8000_0000);
        std::ptr::write_unaligned(at(0x60) as *mut u64, SHOT_FLAGS);
        std::ptr::write_unaligned(at(0x30) as *mut f32, rg::SHOT_SPEED * crate::units::METRES_PER_UNIT);
        std::ptr::write_unaligned(at(0x3C) as *mut i32, (life * crate::units::UNITS_PER_FRAME) as i32);
        std::ptr::write_unaligned(at(0x34) as *mut f32, 1000.0);
        std::ptr::write_unaligned(at(0x44) as *mut i32, target as i32);
        std::ptr::write_unaligned(at(0x48) as *mut f32, 1000.0);
        std::ptr::write_unaligned(at(0x4C) as *mut f32, SNAP_DEG);
        std::ptr::write_unaligned(at(0x10) as *mut [f32; 4], [hand[0], hand[1], hand[2], 0.0]);
        std::ptr::write_unaligned(at(0x20) as *mut [f32; 4], [pitch, yaw, base_ang[2], base_ang[3]]);
        std::ptr::write_unaligned(at(0xA8) as *mut usize, va(base(), super::magic::EFFECT_SETS) + set * 0x60);
        std::ptr::write_unaligned(at(0x78) as *mut f32, rg::shot_record().radius * crate::units::METRES_PER_UNIT);
        let factory = func::<super::magic::FactoryFn>(super::magic::FACTORY)();
        if factory == 0 {
            break;
        }
        let bullet = func::<super::magic::ManagerFn>(super::magic::SHOT_MANAGER)(factory, dp, pl);
        if bullet == 0 {
            continue;
        }
        let id: u32 = rd(bullet + 0x20);
        let b = base();
        let vt: usize = rd(bullet);
        let exe: usize = rd(bullet + super::magic::rt::STATE_EXE);
        let adj: i32 = rd(bullet + super::magic::rt::STATE_EXE_ADJUST);
        if vt == va(b, gb::HOMING_VTABLE) && exe == b + super::magic::HOMING_THUNK as usize && adj == 0 {
            wr(bullet + gb::FLAGS, rd::<u64>(bullet + gb::FLAGS) | POINT_FLAG);
            wr(bullet + SHOT_TURN, SNAP_DEG);
            wr(bullet + SHOT_GATE, 1000.0f32);
            wr(bullet + gb::HOMING_POINT, [hand[0] + r.x, hand[1] + r.y, hand[2] + r.z, 1.0]);
            wr(bullet + super::magic::rt::STATE_EXE, kh1_ragnarok_exe as *const () as usize);
        } else {
            shot_line(|| format!("Ragnarok shot {id} is not a plain CHomingBullet (vt {vt:#x}): Re:CoM's homing"));
        }
        if let Ok(mut g) = SHOTS.lock() {
            if g.len() >= 64 {
                g.remove(0);
            }
            g.push(Shot { id, bend, target });
        }
        made += 1;
    }
    event(format!("fire {made}/{} t{}", f.shots, target));
    shot_line(|| format!("Ragnarok: {made} of {} shots from ({:.2},{:.2},{:.2}), target {target}", f.shots, hand[0], hand[1], hand[2]));
}

/// The flight-state exe of Ragnarok's shots: KH1's step for this frame
/// (`crate::specials::ragnarok::shot_step` in KH1's units: the target's aim point, else the nearest
/// live enemy within 1000 of the shot, `0x1402B0E90`), its new direction handed to Re:CoM's homing
/// step as the point to steer to (a full turn: KH1's 5° limit is in the step), then Re:CoM's own step
/// (the thunk → `0x1401575D0`: the angles, the velocity at the shot's speed). Once a frame a shot.
pub unsafe extern "win64" fn kh1_ragnarok_exe(bullet: usize) {
    use crate::guard::bullet as gb;
    use crate::units::{to_kh1, to_recom};
    use kh1_core::Vec3;
    use super::magic::rt;
    let mover: usize = rd(bullet + gb::MOVER);
    let live = rd::<i32>(bullet + rt::HOLD) == 0 && rd::<i32>(bullet + rt::LIFE) > 0 && mover != 0;
    let id: u32 = rd(bullet + 0x20);
    let shot = SHOTS.lock().ok().and_then(|g| g.iter().find(|s| s.id == id).copied());
    if let (true, Some(mut s)) = (live, shot) {
        let p = rd_vec(mover + rt::POSITION);
        let fw = crate::guard::forward(rd(mover + gb::PITCH), rd(mover + gb::YAW));
        let pos = to_kh1(p);
        let dir = to_kh1(Vec3::new(fw[0], fw[1], fw[2])).normalized();
        if !super::magic::is_target_enemy(s.target) {
            // `0x1402B0E90(Sora, pos, dir, 1000)`: the nearest live enemy within 1000 of the shot.
            let mut best: Option<(f32, u32)> = None;
            // Re:CoM's lock-on list's live enemies (with or without its lock-on flag), by position.
            for (h, a) in super::magic::lock_on_targets(bullet) {
                let d = (to_kh1(Vec3::new(a[0], a[1], a[2])) - pos).length();
                if d <= sp::ragnarok::SEARCH && best.is_none_or(|b| d < b.0) {
                    best = Some((d, h));
                }
            }
            s.target = best.map_or(0, |b| b.1);
        }
        let obj = if s.target != 0 { func::<mv::HandleFn>(mv::HANDLE_OBJ)(s.target) } else { 0 };
        let aim = (obj != 0).then(|| {
            let a = super::magic::aim_point(obj);
            to_kh1(Vec3::new(a[0], a[1], a[2]))
        });
        let (_, d) = sp::ragnarok::shot_step(pos, dir, aim, s.bend, 1.0);
        let r = to_recom(d);
        wr(bullet + gb::FLAGS, rd::<u64>(bullet + gb::FLAGS) | POINT_FLAG);
        wr(bullet + gb::HOMING_POINT, [p.x + r.x * 100.0, p.y + r.y * 100.0, p.z + r.z * 100.0, 1.0]);
        if let Ok(mut g) = SHOTS.lock() {
            if let Some(e) = g.iter_mut().find(|e| e.id == id) {
                e.target = s.target;
            }
        }
    }
    func::<PlFn>(super::magic::HOMING_THUNK)(bullet)
}

/// One of Ragnarok's shots hit this enemy this frame (a hit part whose attacker `+0x98` is one of
/// them, as `super::magic::take_hit`): the part; the shot is forgotten (one hit, KH1's).
pub unsafe fn take_shot_hit(enemy: usize) -> Option<usize> {
    let info: usize = rd(enemy + 0xF8);
    if info == 0 {
        return None;
    }
    let mut g = SHOTS.lock().ok()?;
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
        if let Some(k) = g.iter().position(|s| s.id == by) {
            g.remove(k);
            return Some(part);
        }
    }
    None
}

/// A Ragnarok shot's hit: KH1's damage of its record (`0x5748`: Magic × 2, power 20) on the enemy's
/// twin or proxy as the hit part's value at Damage = KH1 (Re:CoM's resistance not on top, as a KH1
/// spell's); at Damage = Re:CoM the part keeps row 0x20's AP. Returns the KH1 damage written.
pub unsafe fn shot_damage(enemy: usize, part: usize) -> Option<i32> {
    if !super::damage::kh1() {
        return None;
    }
    let max_mp = super::mp::current().map(|v| v.1)?;
    let hp: i32 = rd(enemy + 0x274);
    let d = super::stats::kh1_spell_hit(enemy, &sp::ragnarok::shot_record(), max_mp, hp)?;
    wr(part + 0x92, d.clamp(0, i16::MAX as i32) as i16);
    wr(part + super::magic::PART_ATTR, super::magic::NEUTRAL_ATTR);
    shot_line(|| format!("Ragnarok shot hit {:#x}: KH1 {d} (max MP {max_mp})", enemy & 0x00FF_FFFF));
    Some(d)
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
    let raid = if ready(sp::STRIKE_RAID) {
        "; KH1's Strike Raid (M12 step 7): offered in the context tab (gap < 900, 2 MP) and in the Sleights list when equipped, on Re:CoM's sleight action 0x16 (initStockSkill case 8, cardless; Re:CoM's Strike Raid tint), KH1's wind-up / 5 throws (rows 0x2F..=0x33, the high ones over a target 200+ above) / catches / wait on the xl_limit clips, the next throw in the Attack row; Re:CoM's Strike Raid id 0xD, Sora's own p001_074 / 075 / 076 events on KH1's motions"
    } else {
        "; Strike Raid OFF (no clips or Re:CoM's code not as read)"
    };
    let arcanum = if ready(sp::ARS_ARCANUM) {
        "; KH1's Ars Arcanum (M12 step 7): offered in the context tab (gap < 300, 3 MP) and in the Sleights list when equipped, on Re:CoM's sleight action 0x16 (initStockSkill case 10, cardless; Re:CoM's tint and limit2 start effect), KH1's wind-up / 7 hits by themselves / 5 by Triangle / finisher (rows 0x22..=0x2E) on the xl_limit clips; Re:CoM's Ars Arcanum ids 0x12..=0x1E, its pack 11010's motion events on KH1's motions"
    } else {
        "; Ars Arcanum OFF (no clips or Re:CoM's code not as read)"
    };
    format!(
        "specials: KH1's Sonic Blade{} at combat = kh1 in Sora's story, in battle (M9 stage 9c.1): offered in the context tab (KH1's 0x14028D0F0: equipped, target gap < 600, not 160 above, 2 MP) and in the Sleights list when equipped, Triangle starts it on Re:CoM's sleight action 0x16 (initStockSkill's writes, cardless; MP on the press), KH1's wind-up / 6 dashes / finisher (rows 0x1B..=0x21) on the xl_limit clips, Rave / Blast in the Attack row; Re:CoM's Sonic Blade ids 0xE / 0xF / 0x10 (its hit sounds and effects), its limit1 start effect{}, its sleight motions' sound and effect events (pack 11009 motions 0 / 1 / 2) on KH1's motions; the Battle exe's own case 0x16 -> exeStockSkill{raid}{arcanum}",
        if ready(sp::SONIC_BLADE) { "" } else { " (OFF: no clips)" },
        if EFFECT_OK.load(Ordering::Acquire) { "" } else { " (OFF: name not found)" }
    )
}
