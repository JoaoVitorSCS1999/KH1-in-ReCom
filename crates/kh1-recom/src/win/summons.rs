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
//! Nothing of ours runs per frame besides the menu's build (a save-block read and two loads).

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
/// The last summon preloaded (+ 1; 0 none).
static PRELOADED: AtomicU8 = AtomicU8::new(0);
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
}

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
    // Sora's own vtable slot, as Re:CoM calls it (`0x1401F8150`).
    let vt: usize = rd(pl);
    let slot: usize = if vt != 0 { rd(vt + f::VT_START) } else { 0 };
    if slot != base() + SLEIGHT_START as usize {
        note(|| format!("{}: Sora's vtable +0x130 is {slot:#x}, not Re:CoM's sleight start", s.name));
        return false;
    }
    // A live swing's volume ends first (the specials' rule; Re:CoM's own card chain ends the
    // weapon's attack before the next card, `0x1401F9520`).
    let w: usize = rd(pl + f::WEAPON);
    if w != 0 && rd::<i32>(w + f::WEAPON_LIVE) != 0 {
        func::<PlFn>(WEAPON_END)(w);
    }
    let before: u32 = rd(pl + p::ACTION);
    wr(pl + f::SLEIGHT, s.sleight as u32);
    let r = std::mem::transmute::<usize, StartFn>(slot)(pl);
    let now: u32 = rd(pl + p::ACTION);
    if r & 0xFF == 0 || now != STOCK_ACTION {
        note(|| format!("{}: Re:CoM's sleight start refused sleight {:#x} (returned {r}, action {now:#x})", s.name, s.sleight));
        return false;
    }
    // KH1 pays at once, in whole points (`0x140289210` → `0x1402A4A00(actor, −cost)`).
    super::mp::pay(cost * 100, s.name);
    OUT_SUMMON.store(i as u8, Ordering::Relaxed);
    SKIPPED.store(0, Ordering::Relaxed);
    OUT.store(pl, Ordering::Relaxed);
    note(|| format!("{} -> Re:CoM's {} (sleight {:#x}, attack {:#x}, {} from PLAYER/{:04}), {cost} MP, from action {before:#x}; Sora invincible until it ends", s.name, s.sleight_name, s.sleight, s.attack_id, s.class, s.pack));
    true
}

/// The Summon list's cursor rests on summon `i` (`None`: the list is closed): Re:CoM's sleight
/// resource load for its LV3 once per landing, in battle (Re:CoM's own cache decides what stays).
pub unsafe fn preload(pl: usize, i: Option<usize>) {
    let tag = i.map_or(0, |i| i as u8 + 1);
    if PRELOADED.swap(tag, Ordering::Relaxed) == tag || !installed() {
        return;
    }
    let Some(s) = i.and_then(|i| sm::ALL.get(i)) else { return };
    if pl == 0 || rd::<i32>(pl + p::BATTLE) == 0 {
        return;
    }
    let res: usize = rd(pl + f::SLEIGHTS);
    if res != 0 {
        func::<LoadFn>(SLEIGHT_LOAD)(res, s.sleight as u32, 0);
    }
}
