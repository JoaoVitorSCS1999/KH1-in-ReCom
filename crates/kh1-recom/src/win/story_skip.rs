//! Story skips at Combat = KH1 (`crate::story_skip`, `docs/recom-story-skips.md`).
//!
//! - **The wrapper** on Re:CoM's battle-entry request `0x140236690(room)`: when an event room's
//!   script asks for one of the two tutorial battle rooms (`crate::story_skip::SKIPS`) at Combat =
//!   KH1 in Sora's story, in Traverse Town, it does that battle script's lesson exit instead: its
//!   writes through Re:CoM's own setters (`0x1401B1650`, `0x1401B16C0`, `0x1401B08C0`), then its
//!   room change through Re:CoM's own `0x1402360F0`. Every other request goes to Re:CoM unchanged.
//! - If the room change can't find its floor or refuses, Re:CoM's battle entry runs as asked, so
//!   the story never stalls.
//! - Runs once per battle request (a script syscall or a field encounter), nothing per frame. Each
//!   skip is logged.

use super::movement::{self as mv, func, orig, rd};
use super::{base, log, va};
use crate::keyblades::SAVE_PTR;
use crate::story_skip::{self as ss, Door, Write};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub static ORIG_BATTLE_ENTRY: AtomicUsize = AtomicUsize::new(0);
static LOG_LEFT: AtomicU32 = AtomicU32::new(40);

/// Re:CoM's functions (RVAs; first bytes checked: `sites::STORY_SKIP_CALLEES` and the system
/// variable getter / setter of earlier stages).
const ROOM_CHANGE: u32 = 0x0023_60F0;
const SAVED_DOOR: u32 = 0x0023_3AE0;
const FLAG_SET: u32 = 0x001B_1650;
const VAR_GET: u32 = 0x001B_08C0;
const VAR_SET: u32 = 0x001B_16C0;

/// The save block's map world code and room (`0x1402336C0`, `0x1402339B0`). Checked (decomp).
const SAVE_WORLD: usize = 0x1878;
const SAVE_ROOM: usize = 0x187A;

type EntryFn = unsafe extern "win64" fn(u64) -> u64;
type RoomChangeFn = unsafe extern "win64" fn(u32, u32, u32) -> u32;
type DoorFn = unsafe extern "win64" fn() -> u32;
type FlagSetFn = unsafe extern "win64" fn(u32, u32, u32);
type VarGetFn = unsafe extern "win64" fn(u32, u32) -> i32;
type VarSetFn = unsafe extern "win64" fn(u32, u32, i32);

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("story skip: {}", line()));
    }
}

pub fn start_line() -> String {
    let rooms: Vec<String> = ss::SKIPS.iter().map(|s| format!("{} ({} from {} -> {})", s.name, s.battle_room, s.from_room, s.exit.room)).collect();
    format!(
        "story skip: wrapper on Re:CoM's battle-entry request 0x140236690; at Combat = KH1 in Sora's story, Traverse Town's tutorial battle rooms are skipped through their own lesson exit: {}",
        rooms.join(", ")
    )
}

/// What the request is asked in, from the save block (None before there is one).
unsafe fn ask(battle_room: i16) -> Option<ss::Ask> {
    let save: usize = rd(va(base(), SAVE_PTR));
    if save == 0 {
        return None;
    }
    Some(ss::Ask {
        kh1: mv::kh1_on(),
        riku_story: super::stats::riku_story(),
        world: rd(save + SAVE_WORLD),
        room: rd(save + SAVE_ROOM),
        battle_room,
    })
}

unsafe fn apply(w: Write) {
    match w {
        Write::Bit1(id, v) => func::<FlagSetFn>(FLAG_SET)(1, id, v as u32),
        Write::Int1(id, v) => func::<VarSetFn>(VAR_SET)(1, id, v),
        Write::Int0Or(id, mask) => {
            let v = func::<VarGetFn>(VAR_GET)(0, id) | mask as i32;
            func::<VarSetFn>(VAR_SET)(0, id, v);
        }
    }
}

/// Re:CoM's `0x140236690(room)` (the room in `cx`; `bool` in `al`).
pub unsafe extern "win64" fn battle_entry(arg: u64) -> u64 {
    let o: EntryFn = orig(&ORIG_BATTLE_ENTRY);
    let battle_room = arg as u16 as i16;
    // Only the two tutorial rooms are looked at further (other callers pass 0 or 1030).
    if !ss::SKIPS.iter().any(|s| s.battle_room == battle_room) {
        return o(arg);
    }
    let Some(a) = ask(battle_room) else { return o(arg) };
    let Some(skip) = ss::plan(a) else {
        if ss::is_tutorial_ask(a) {
            note(|| format!("battle room {battle_room} asked from room {} (world {}): Re:CoM's battle (Combat = {}, {})", a.room, a.world, if a.kh1 { "KH1" } else { "Re:CoM" }, if a.riku_story { "Riku's story" } else { "Sora's story" }));
        }
        return o(arg);
    };
    let e = skip.exit;
    if e.kind < 5 {
        let slot: i16 = rd(va(base(), ss::FLOOR_SLOTS) + 2 * e.kind as usize);
        if !ss::floor_slot_ok(e.kind, slot) {
            note(|| format!("{}: floor slot {} is {slot}; Re:CoM's battle room {battle_room} runs instead", skip.name, e.kind));
            return o(arg);
        }
    }
    for w in skip.writes {
        apply(*w);
    }
    let door = match e.door {
        Door::Fixed(d) => d,
        Door::Saved => func::<DoorFn>(SAVED_DOOR)() & 0xFF,
    };
    let ok = func::<RoomChangeFn>(ROOM_CHANGE)(e.kind, door, e.room as u16 as u32);
    let friends = func::<VarGetFn>(VAR_GET)(0, ss::FRIENDS_MET);
    if ok != 0 {
        note(|| {
            format!(
                "{} skipped at Combat = KH1: battle room {battle_room} (asked from room {}) not entered; its lesson exit's {} writes done (friends-met (0, {:#x}) = {friends:#x}), Re:CoM's room change 0x1402360F0({}, {door}, {}) -> {ok}",
                skip.name, a.room, skip.writes.len(), ss::FRIENDS_MET, e.kind, e.room
            )
        });
        return 1;
    }
    note(|| format!("{}: Re:CoM's room change 0x1402360F0({}, {door}, {}) refused; Re:CoM's battle room {battle_room} runs instead", skip.name, e.kind, e.room));
    o(arg)
}
