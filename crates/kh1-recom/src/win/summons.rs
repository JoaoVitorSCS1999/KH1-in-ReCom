//! M8b: KH1's summons on Re:CoM's own summon sleights (`crate::summons`, `docs/recom-summons.md`).
//!
//! - **The command**: KH1's Summon list (`crate::command_menu`) issues a summon; the menu's step
//!   (from the request-bits wrapper, where Re:CoM's own `0x1401F8150` starts a played sleight)
//!   calls [`start`] the same frame, as KH1's command step runs `0x140289210` the frame it is
//!   issued.
//! - **The start** ([`start`]): battle only; a live swing's weapon volume ends (as the specials
//!   do); `player+0x82C` = the summon's LV3 sleight id; then **Re:CoM's own sleight start**, Sora's
//!   vtable `+0x130` (`0x1402247A0`: its use counter, `initStockSkill`, `+0x838` = 10). Re:CoM then
//!   runs everything: its tint and "summon" effect, its resource wait (motion 0x2E), the summon
//!   actor (`CSummon*`), its hits and damage, and its end (`0x1401F6660`). KH1's MP (the owner's
//!   CP-based cost) is paid once Re:CoM's action is `0x16`.
//! - **While it runs**: our `exeStockSkill` wrapper (when the specials are in) passes Re:CoM's
//!   sleights through; Re:CoM's Battle exe steps it. The summon counts as out ([`out`]: from our
//!   start while Sora's action is `0x16` in the battle room): the menu's row 3 greyed and locked (a
//!   second sleight can't start mid-sleight), and **Sora is invincible** (owner, 2026-11-02): the
//!   damage-check wrapper (`attack::damage_check`, on Re:CoM's `0x1401E6DE0`) holds Re:CoM's own
//!   protection bit `player+0x418 & 1` for each check (its exit: the hit's value zeroed, no damage,
//!   no reaction; the roll's and Sonic Blade's path). The Battle exe runs the check every frame of
//!   action `0x16` (`0x1401F9840`, after the request bits), so it covers the start frame, the
//!   attached summons (Simba, Genie, Dumbo, Cloud) and the standing ones (Bambi, Tinker Bell,
//!   Mushu), and ends with the action: Re:CoM's end action `0x1401F6660`, its win action at a
//!   battle's end, or the room's end (the field: `player+0x390` = 0).
//! - **After it**: Re:CoM's end action leaves Sora in its own move action; our move-exe wrapper sees
//!   that its last KH1 frame no longer matches and takes Sora back fresh (`movement::continues`),
//!   as after Re:CoM's hit reactions and its win action.
//! - **No once-per-battle rule** (owner, 2026-11-02): usable again whenever the MP pays.
//! - **The learn order**: our save block (`crate::summons::BLOCK_FIRST`), synced from Re:CoM's
//!   discovery state by the unlocks wrapper on `0x14015C7F0` and at the first read of a save.
//! - **Preload**: when the Summon list's cursor lands on a summon in battle, Re:CoM's sleight
//!   resource load `0x140211790(player+0x3B8, sleight, 0)`, as Re:CoM loads a sleight's resources
//!   when its cards form it in the stock (`0x1401669F0`).
//!
//! - **M12 step 5, the Sleights list** ([`start_sleight`]): the same start ([`begin`]) for an
//!   equipped sleight ability's Re:CoM id, then the owner's MP by tier (all of it for Idyll Romp /
//!   Cross-slash+). Re:CoM's air-start sleights (`crate::sleights::AIR_START`) also start mid-air:
//!   Re:CoM's own start then takes their air version (owner, "Sleights menu follow-ups"). No invincibility of ours: Re:CoM's `exeStockSkill` sets its own protection bits
//!   where its sleight has them. While any sleight runs ([`sleight_running`]) both rows lock. The
//!   bottom row's choice ([`bottom_row`]) is the summon block's byte `save + 0x26B6`.
//!
//! Nothing of ours runs per frame besides the menu's build (a save-block read and a few loads).

use super::movement::{func, p, rd, wr};
use super::{base, log, va};
use crate::summons::{self as sm, BLOCK_FIRST, BLOCK_LEN};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The player whose Re:CoM summon sleight is out (KH1's in-progress flag), and which summon.
static OUT: AtomicUsize = AtomicUsize::new(0);
static OUT_SUMMON: AtomicU8 = AtomicU8::new(0);
/// The hits skipped during the summon out (for the end's log line).
static SKIPPED: AtomicU32 = AtomicU32::new(0);
/// The last sleight preloaded (its Re:CoM id + 1; 0 none): a summon's LV3 or (M12 step 5) a
/// Sleights list entry.
static PRELOADED: AtomicU32 = AtomicU32::new(0);
/// M12 step 5: the player whose Sleights-list sleight is out (Re:CoM's action 0x16 from our
/// start; no invincibility of ours), and which sleight; for the end's log line.
static SLEIGHT_OUT: AtomicUsize = AtomicUsize::new(0);
static SLEIGHT_ID: AtomicU32 = AtomicU32::new(0);
/// M12 step 5: the bottom row's choice with no save to keep it in (0 Summon, 1 Sleights).
static BOTTOM_SESSION: AtomicU8 = AtomicU8::new(0);
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);

/// Re:CoM's sleight start (Sora's vtable `+0x130`), the sleight resource load, the weapon's attack
/// end (`sites::SUMMON_CHECKED`).
const SLEIGHT_START: u32 = 0x0022_47A0;
const SLEIGHT_LOAD: u32 = 0x0021_1790;
const WEAPON_END: u32 = 0x0022_4FB0;
/// Re:CoM's sleight action (`initStockSkill` `0x1401F8D70(player, 0x16)`).
const STOCK_ACTION: u32 = 0x16;

mod f {
    /// The vtable slot of the sleight start.
    pub const VT_START: usize = 0x130;
    /// The played card record's sleight id (`+0x824` short, copied to `+0x82C` by `0x1401F5580`;
    /// `initStockSkill` and `exeStockSkill` read `+0x82C`).
    pub const SLEIGHT: usize = 0x82C;
    /// The sleight resources object (`initStockSkill`'s `0x140211790(+0x3B8, …)`).
    pub const SLEIGHTS: usize = 0x3B8;
    pub const WEAPON: usize = 0x3D0;
    /// The weapon's attack is live.
    pub const WEAPON_LIVE: usize = 0x23C;
    /// Sora's state bits (`+0x418`; [`super::AIR_VERSION`]).
    pub const FLAGS_418: usize = 0x418;
}

/// `initStockSkill`'s air version of a sleight started in the air (`+0x418 |= 0x800000`, only for
/// `crate::sleights::AIR_START` and the -ra / -ga tiers). Checked (decomp `0x140220DF0`).
const AIR_VERSION: u32 = 0x80_0000;

type StartFn = unsafe extern "win64" fn(usize) -> u64;
type LoadFn = unsafe extern "win64" fn(usize, u32, u32);
type PlFn = unsafe extern "win64" fn(usize);

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("summons: {}", line()));
    }
}

/// Checks the Re:CoM code the summons call (no hook, no patch).
pub unsafe fn install(b: usize) -> bool {
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    for c in crate::sites::SUMMON_CHECKED {
        if have(b + c.rva as usize, c.expect.len()) != c.expect {
            log(&format!("summons: {} is not as expected; no KH1 summons", c.name));
            return false;
        }
    }
    for c in crate::sites::all_sites().filter(|c| crate::sites::SUMMON_ALSO.contains(&c.rva)) {
        if have(b + c.rva as usize, c.expect.len()) != c.expect {
            log(&format!("summons: {} is not as expected; no KH1 summons", c.name));
            return false;
        }
    }
    for p in crate::sites::SUMMON_PTRS {
        if rd::<usize>(b + p.rva as usize) != b + p.target as usize {
            log(&format!("summons: {} does not point at {:#x}; no KH1 summons", p.name, 0x1_4000_0000u64 + p.target as u64));
            return false;
        }
    }
    INSTALLED.store(true, Ordering::Release);
    true
}

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

pub fn start_line() -> String {
    let t: Vec<String> = sm::ALL.iter().map(|s| format!("{} -> {} (sleight {:#x}, {} MP)", s.name, s.sleight_name, s.sleight, sm::mp_from_cp(s.cp))).collect();
    format!(
        "summons: KH1's Summon list at combat = kh1 in Sora's story (M8b), battle only, each Re:CoM's LV3 sleight through Sora's sleight start 0x1402247A0: {}; learn order at save+{:#x}..={:#x} (layout v{}); KH1's party rule skipped until the party exists",
        t.join(", "),
        BLOCK_FIRST,
        sm::BLOCK_LAST,
        sm::BLOCK_V1
    )
}

// ---- the save block ----

unsafe fn block_at() -> Option<usize> {
    if super::stats::riku_story() {
        return None;
    }
    let save: usize = rd(va(base(), crate::keyblades::SAVE_PTR));
    (save != 0).then_some(save + BLOCK_FIRST)
}

/// Brings the learn order up to Re:CoM's discovery state (`0x14015B3F0`), logging each summon
/// learned. Called by the unlocks wrapper when a summon card is first obtained, and by [`learned`]
/// for a save whose block was never written.
pub unsafe fn sync(why: &str) {
    if !installed() {
        return;
    }
    let Some(at) = block_at() else { return };
    // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
    let old: [u8; BLOCK_LEN] = rd(at);
    let first = old[sm::field::VERSION] == 0;
    let (new, added) = sm::sync(old, |id| super::keyblades::ever_obtained(id));
    for i in &added {
        let s = &sm::ALL[*i];
        note(|| format!("learned {} ({}, card {:#x}){}: {why}", s.name, s.sleight_name, s.card, if first { ", an older save: KH1's index order, Cloud last" } else { "" }));
    }
    if new != old {
        wr(at, new);
    }
}

/// The learned summons in learn order, and their count (the menu's build).
pub unsafe fn learned() -> ([u8; sm::COUNT], u8) {
    let mut out = [0u8; sm::COUNT];
    if !installed() {
        return (out, 0);
    }
    let Some(at) = block_at() else { return (out, 0) };
    let mut b: [u8; BLOCK_LEN] = rd(at);
    if b[sm::field::VERSION] == 0 {
        sync("the first read of this save's summons at the KH1 setting (Re:CoM's discovery state)");
        b = rd(at);
    }
    let order = sm::order(&b);
    for (k, &i) in order.iter().enumerate() {
        out[k] = i as u8;
    }
    (out, order.len() as u8)
}

// ---- the battle ----

/// KH1's usable test's inputs from Re:CoM's side, every menu build: (battle mode, a summon out).
pub unsafe fn gate_state(pl: usize) -> (bool, bool) {
    let battle = rd::<i32>(pl + p::BATTLE) != 0;
    if !battle {
        PRELOADED.store(0, Ordering::Relaxed);
    }
    (battle, out(pl))
}

/// `pl`'s Re:CoM summon sleight (ours) is out: from [`start`] while Sora's action is `0x16` in the
/// battle room. The first call that sees otherwise (Re:CoM's end action `0x1401F6660`, its win
/// action, the room's end, another player) ends it. Two loads unless one is out.
pub unsafe fn out(pl: usize) -> bool {
    let o = OUT.load(Ordering::Relaxed);
    if o == 0 || pl == 0 {
        return false;
    }
    if o == pl && rd::<u32>(pl + p::ACTION) == STOCK_ACTION && rd::<i32>(pl + p::BATTLE) != 0 {
        return true;
    }
    if OUT.compare_exchange(o, 0, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
        let s = &sm::ALL[OUT_SUMMON.load(Ordering::Relaxed) as usize % sm::COUNT];
        let skipped = SKIPPED.swap(0, Ordering::Relaxed);
        if o == pl {
            note(|| format!("{}: Re:CoM's {} ended (action {:#x}{}); invincibility off ({skipped} hits skipped); the KH1 frame takes Sora back", s.name, s.sleight_name, rd::<u32>(pl + p::ACTION), if rd::<i32>(pl + p::BATTLE) == 0 { ", out of the battle room" } else { "" }));
        } else {
            note(|| format!("{}: its player is gone (another player {pl:#x}); invincibility off ({skipped} hits skipped)", s.name));
        }
    }
    false
}

/// The damage check met a hit (`+0x41C & 0x200`) on Sora while his summon is out (the check's
/// protection exit skips it): counted and logged.
pub fn hit_skipped(value: i16) {
    let n = SKIPPED.fetch_add(1, Ordering::Relaxed) + 1;
    if n <= 8 {
        note(|| format!("a hit on Sora during the summon (value {value}): skipped (Re:CoM's +0x418 bit 0: no damage, no reaction)"));
    }
}

/// Summon `i` (the menu's command, KH1's usable test passed): Re:CoM's LV3 sleight through Sora's
/// own sleight start, then KH1's MP. False (nothing changed) when it can't start.
pub unsafe fn start(pl: usize, i: usize) -> bool {
    let Some(s) = sm::ALL.get(i) else { return false };
    if !installed() || pl == 0 {
        return false;
    }
    if rd::<i32>(pl + p::BATTLE) == 0 {
        note(|| format!("{}: not in battle", s.name));
        return false;
    }
    let cost = sm::mp_cost(i);
    if !super::mp::can_pay(cost * 100) {
        note(|| format!("{}: MP short ({cost})", s.name));
        return false;
    }
    let Some(before) = begin(pl, s.sleight, s.name) else { return false };
    // KH1 pays at once, in whole points (`0x140289210` → `0x1402A4A00(actor, −cost)`).
    super::mp::pay(cost * 100, s.name);
    OUT_SUMMON.store(i as u8, Ordering::Relaxed);
    SKIPPED.store(0, Ordering::Relaxed);
    OUT.store(pl, Ordering::Relaxed);
    note(|| format!("{} -> Re:CoM's {} (sleight {:#x}, attack {:#x}, {} from PLAYER/{:04}), {cost} MP, from action {before:#x}; Sora invincible until it ends", s.name, s.sleight_name, s.sleight, s.attack_id, s.class, s.pack));
    true
}

/// Re:CoM's own sleight start for `sleight` (the summons' and the Sleights list's): Sora's vtable
/// `+0x130` checked to be `0x1402247A0`, a live swing's volume ended, `player+0x82C` = the id, the
/// start called as Re:CoM calls it (`0x1401F8150`), and Re:CoM's action `0x16` checked after it.
/// Some(the action before) when it started; None (logged) when it refused.
unsafe fn begin(pl: usize, sleight: u16, name: &str) -> Option<u32> {
    // Sora's own vtable slot, as Re:CoM calls it (`0x1401F8150`).
    let vt: usize = rd(pl);
    let slot: usize = if vt != 0 { rd(vt + f::VT_START) } else { 0 };
    if slot != base() + SLEIGHT_START as usize {
        note(|| format!("{name}: Sora's vtable +0x130 is {slot:#x}, not Re:CoM's sleight start"));
        return None;
    }
    // A live swing's volume ends first (the specials' rule; Re:CoM's own card chain ends the
    // weapon's attack before the next card, `0x1401F9520`).
    let w: usize = rd(pl + f::WEAPON);
    if w != 0 && rd::<i32>(w + f::WEAPON_LIVE) != 0 {
        func::<PlFn>(WEAPON_END)(w);
    }
    let before: u32 = rd(pl + p::ACTION);
    wr(pl + f::SLEIGHT, sleight as u32);
    let r = std::mem::transmute::<usize, StartFn>(slot)(pl);
    let now: u32 = rd(pl + p::ACTION);
    if r & 0xFF == 0 || now != STOCK_ACTION {
        note(|| format!("{name}: Re:CoM's sleight start refused sleight {sleight:#x} (returned {r}, action {now:#x})"));
        return None;
    }
    Some(before)
}

/// The Summon list's cursor rests on a summon (its LV3 sleight) or (M12 step 5) a Sleights list
/// entry: Re:CoM's sleight resource load `0x140211790(player+0x3B8, id, 0)` for `id` once per landing
/// (`None`: nothing under the cursor), in battle, as Re:CoM loads a sleight's resources when its
/// cards form it in the stock (`0x1401669F0`). The summons' and (M12 step 5) the Sleights list's.
pub unsafe fn preload_sleight(pl: usize, id: Option<u16>) {
    let tag = id.map_or(0, |id| id as u32 + 1);
    if PRELOADED.swap(tag, Ordering::Relaxed) == tag || !installed() {
        return;
    }
    let Some(id) = id else { return };
    if pl == 0 || rd::<i32>(pl + p::BATTLE) == 0 {
        return;
    }
    let res: usize = rd(pl + f::SLEIGHTS);
    if res != 0 {
        func::<LoadFn>(SLEIGHT_LOAD)(res, id as u32, 0);
    }
}

// ---- M12 step 5: the Sleights row and list ----

/// The command menu's bottom row (`crate::sleights::Bottom`): the save's choice (the summon block's
/// byte `save + 0x26B6`), or this session's without a save. A one-byte read.
pub unsafe fn bottom_row() -> crate::sleights::Bottom {
    let b = match block_at() {
        Some(at) => rd::<u8>(at + sm::field::BOTTOM_ROW),
        None => BOTTOM_SESSION.load(Ordering::Relaxed),
    };
    crate::sleights::Bottom::of_byte(b)
}

/// D-pad Left switched the bottom row: kept in the save (read by every later frame; written only on
/// the press), else for the session.
pub unsafe fn set_bottom_row(b: crate::sleights::Bottom) {
    BOTTOM_SESSION.store(b.byte(), Ordering::Relaxed);
    let saved = match block_at() {
        Some(at) => {
            // SAFETY: inside the 0x2AD4-byte save block (BLOCK_LAST < 0x2818).
            wr::<u8>(at + sm::field::BOTTOM_ROW, b.byte());
            true
        }
        None => false,
    };
    note(|| format!("bottom row -> {b:?} ({})", if saved { "kept in the save at save+0x26B6" } else { "no save: this session only" }));
}

/// Any Re:CoM sleight is running for `pl` (its action `0x16` in the battle room: a summon, a
/// Sleights-list sleight, or Sonic Blade's): the Summon and Sleights rows lock (one at a time). The
/// first call that sees ours end logs it. Two loads.
pub unsafe fn sleight_running(pl: usize) -> bool {
    let running = pl != 0 && rd::<u32>(pl + p::ACTION) == STOCK_ACTION && rd::<i32>(pl + p::BATTLE) != 0;
    let o = SLEIGHT_OUT.load(Ordering::Relaxed);
    if o != 0 && !(o == pl && running) && SLEIGHT_OUT.compare_exchange(o, 0, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
        let id = SLEIGHT_ID.load(Ordering::Relaxed) as u16;
        note(|| format!("sleight {} ({id:#x}): Re:CoM's sleight ended (action {:#x}); the KH1 frame takes Sora back", crate::sleights::name(id), if pl != 0 { rd::<u32>(pl + p::ACTION) } else { 0 }));
    }
    running
}

/// M12 step 5: a Sleights list entry (the menu's command, the usable test passed): Re:CoM's own
/// sleight start for `id` (as a summon's, [`begin`]), then the owner's MP (KH1's whole-point
/// payment `0x1402A5B60`: the tier's points, or all of Sora's whole MP). Re:CoM runs the rest: its
/// motion, effects, hits and damage, its own protection bits (`+0x418`, set by `exeStockSkill`
/// where its sleights have them; nothing of ours), its end. False (nothing changed) when it can't.
pub unsafe fn start_sleight(pl: usize, id: u16) -> bool {
    let name = crate::sleights::name(id);
    if !installed() || pl == 0 {
        return false;
    }
    if rd::<i32>(pl + p::BATTLE) == 0 {
        note(|| format!("sleight {name}: not in battle"));
        return false;
    }
    let mp = super::mp::current().map_or(0, |m| m.0);
    let Some(points) = crate::sleights::pay_points(id, mp) else {
        note(|| format!("sleight {name}: MP short ({:?}, MP {mp})", crate::sleights::cost(id)));
        return false;
    };
    let Some(before) = begin(pl, id, name) else { return false };
    super::mp::pay(points * 100, name);
    SLEIGHT_ID.store(id as u32, Ordering::Relaxed);
    SLEIGHT_OUT.store(pl, Ordering::Relaxed);
    // M12 follow-ups (owner, "Sleights menu follow-ups"): an air-start sleight started mid-air takes
    // Re:CoM's air version (`initStockSkill` sets `+0x418 & 0x800000`; `crate::sleights::AIR_START`).
    let air = rd::<u32>(pl + f::FLAGS_418) & AIR_VERSION != 0;
    note(|| format!("sleight {name} -> Re:CoM's sleight {id:#x} through Sora's sleight start{}, {points} MP ({:?}), from action {before:#x}; Re:CoM's own protection only", if air { " (in the air: Re:CoM's air version)" } else { "" }, crate::sleights::cost(id).unwrap_or(crate::sleights::Cost::Points(0))));
    true
}
