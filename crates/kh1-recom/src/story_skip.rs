//! Story skips at Combat = KH1 (owner, 2026-11-01: "If a player has KH1 Combat enabled, have the
//! player skip the leon training battle. Also the intro tutorial."; option (a): skip the battle
//! room). `docs/recom-story-skips.md`.
//!
//! Re:CoM's Traverse Town opening is a chain of event rooms and event battle rooms. An event room's
//! script asks for a battle room with syscall 0xEE, which calls Re:CoM's battle-entry request
//! `0x140236690(room)`. The battle room's own script runs the lesson and then leaves with syscall
//! 0xF7, Re:CoM's room change `0x1402360F0(kind, door, room)`. Checked (decomp + the owner's
//! `WORLD01` scripts, read in the session only):
//!
//! | Tutorial | asked for by | battle room | its lesson exit |
//! |---|---|---|---|
//! | intro (hooded figure) | event room 2000 | 1100 | session bits cleared, friends-met (0, 0x22) ∣= 1 ∣ 2, `0xF7(0, 0, 2001)` |
//! | Leon's training battle | event room 2002 | 1101 | session bits cleared, (1, 0x32) = 1, `0xF7(1, −1, 2003)` (door −1 = Re:CoM's `0x140233AE0()`) |
//!
//! At Combat = KH1 in Sora's story, the battle-entry request for those two rooms does that exit's
//! writes, in the script's order, and that exit's room change instead (`plan`). The battle room
//! never loads. Everything else is passed to Re:CoM unchanged.

/// Traverse Town's map world code (`save + 0x1878`; its rooms are `WORLD01/…`). Checked (decomp:
/// the room resource id is `(code + 50) × 10000 + room`, `0x14023C6C0`; files).
pub const WORLD_TRAVERSE_TOWN: i16 = 1;

/// The friends-met bitmask, set-0 int variable 0x22 (bit 1 Donald, bit 2 Goofy; read by Re:CoM's
/// Friend-card drop roll `0x1401602A0`). Checked (decomp + exe scan).
pub const FRIENDS_MET: u32 = 0x22;
/// Set-1 bit 0x32: the next room keeps the music playing (`0x1401AF9D0`, `0x1401ADD20`).
pub const KEEP_MUSIC: u32 = 0x32;
/// Set-1 int 0xE2: Re:CoM's HUD-hide mask.
pub const HUD_HIDE: u32 = 0xE2;

/// One write of a battle script's exit, through Re:CoM's own setters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Write {
    /// `0x1401B1650(1, id, v)` (script op 0x6E).
    Bit1(u32, bool),
    /// `0x1401B16C0(1, id, v)` (script op 0x60).
    Int1(u32, i32),
    /// `v = 0x1401B08C0(0, id) | mask; 0x1401B16C0(0, id, v)` (script ops 0x53, 0xFE or, 0x54).
    Int0Or(u32, u32),
}

/// The door argument of the exit's room change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Door {
    /// A fixed door.
    Fixed(u32),
    /// The script's −1: syscall 0xF7 passes Re:CoM's `0x140233AE0()` (the save's door byte, at most
    /// 3, else 2).
    Saved,
}

/// The battle script's room change `0x1402360F0(kind, door, room)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exit {
    pub kind: u32,
    pub door: Door,
    pub room: i16,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Skip {
    pub name: &'static str,
    /// The event room whose script asks for the battle.
    pub from_room: i16,
    pub battle_room: i16,
    /// The battle script's lesson-exit writes, in its order.
    pub writes: &'static [Write],
    pub exit: Exit,
}

/// Battle room 1100's lesson exit (`tr_1100.abc` instructions 521..626). Checked (files).
const INTRO_WRITES: &[Write] = &[
    Write::Bit1(0x00, false),
    Write::Int1(HUD_HIDE, 0),
    Write::Bit1(0x03, false),
    Write::Bit1(0x04, false),
    Write::Bit1(0x05, false),
    Write::Bit1(0x06, false),
    Write::Bit1(0x08, false),
    Write::Bit1(0x09, false),
    Write::Bit1(0x0A, false),
    Write::Bit1(0x0B, false),
    Write::Bit1(0x0C, false),
    Write::Bit1(0x23, false),
    Write::Bit1(0x2C, false),
    Write::Int0Or(FRIENDS_MET, 1),
    Write::Int0Or(FRIENDS_MET, 2),
];

/// Battle room 1101's lesson exit (`tr_1101.abc` instructions 247..310). Checked (files).
const LEON_WRITES: &[Write] = &[
    Write::Bit1(0x2C, false),
    Write::Bit1(0x0A, false),
    Write::Bit1(0x2F, false),
    Write::Bit1(0x30, false),
    Write::Bit1(KEEP_MUSIC, true),
    Write::Bit1(0x00, false),
    Write::Int1(HUD_HIDE, 0),
    Write::Bit1(0x03, false),
    Write::Bit1(0x04, false),
    Write::Bit1(0x05, false),
    Write::Bit1(0x06, false),
];

pub const SKIPS: &[Skip] = &[
    Skip {
        name: "intro tutorial (hooded figure)",
        from_room: 2000,
        battle_room: 1100,
        writes: INTRO_WRITES,
        exit: Exit { kind: 0, door: Door::Fixed(0), room: 2001 },
    },
    Skip {
        name: "Leon's training battle",
        from_room: 2002,
        battle_room: 1101,
        writes: LEON_WRITES,
        exit: Exit { kind: 1, door: Door::Saved, room: 2003 },
    },
];

/// What Re:CoM's battle-entry request `0x140236690(battle_room)` is asked in.
#[derive(Debug, Clone, Copy)]
pub struct Ask {
    /// Combat = KH1 (the movement handlers carry the setting).
    pub kh1: bool,
    pub riku_story: bool,
    /// `save + 0x1878`, the map world code.
    pub world: i16,
    /// `save + 0x187A`, the room asking.
    pub room: i16,
    /// The battle room asked for (the request's argument, `cx`).
    pub battle_room: i16,
}

/// The skip for this request, or None (Re:CoM's battle entry runs).
pub fn plan(a: Ask) -> Option<&'static Skip> {
    if !a.kh1 || a.riku_story || a.world != WORLD_TRAVERSE_TOWN {
        return None;
    }
    SKIPS.iter().find(|s| s.battle_room == a.battle_room && s.from_room == a.room)
}

/// A tutorial battle room asked for in Traverse Town, skipped or not (for the log).
pub fn is_tutorial_ask(a: Ask) -> bool {
    a.world == WORLD_TRAVERSE_TOWN && SKIPS.iter().any(|s| s.battle_room == a.battle_room)
}

/// `0x1402360F0`'s floor lookup for kinds below 5 (`0x140237850`) reads the i16 table
/// `0x14087CBE2[kind]` and dereferences the floor it names; a negative entry means no floor (it
/// would read null). The skip calls the room change only when the entry is not negative.
pub const FLOOR_SLOTS: u64 = 0x1_4087_CBE2;

/// Whether the room change can find its floor: kinds 5 and up don't use the table.
pub fn floor_slot_ok(kind: u32, slot: i16) -> bool {
    kind >= 5 || slot >= 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(battle_room: i16, room: i16) -> Ask {
        Ask { kh1: true, riku_story: false, world: 1, room, battle_room }
    }

    #[test]
    fn only_the_two_tutorials_in_traverse_town_at_kh1_in_soras_story() {
        assert_eq!(plan(ask(1100, 2000)).unwrap().exit, Exit { kind: 0, door: Door::Fixed(0), room: 2001 });
        assert_eq!(plan(ask(1101, 2002)).unwrap().exit, Exit { kind: 1, door: Door::Saved, room: 2003 });
        // Combat = Re:CoM, Riku's story, another world (Riku's tutorial is world 0's 1100).
        assert!(plan(Ask { kh1: false, ..ask(1100, 2000) }).is_none());
        assert!(plan(Ask { riku_story: true, ..ask(1101, 2002) }).is_none());
        assert!(plan(Ask { world: 0, ..ask(1100, 2102) }).is_none());
        assert!(plan(Ask { world: 0, ..ask(1100, 2000) }).is_none());
        // Other battle rooms, other callers' arguments (0, 1030), the wrong asking room.
        for (b, r) in [(1102, 2002), (1050, 2000), (1150, 2004), (0, 2000), (1030, 2000), (1100, 2002), (1101, 2000), (1100, 1)] {
            assert!(plan(ask(b, r)).is_none(), "{b} {r}");
        }
        assert!(is_tutorial_ask(Ask { kh1: false, ..ask(1101, 2002) }));
        assert!(!is_tutorial_ask(Ask { world: 0, ..ask(1100, 2102) }));
    }

    #[test]
    fn exits_match_the_battle_scripts() {
        let intro = plan(ask(1100, 2000)).unwrap();
        // The friends-met bits end set (Donald, Goofy), as on the lesson exit; nothing else in set 0.
        let set0: Vec<_> = intro.writes.iter().filter(|w| matches!(w, Write::Int0Or(..))).collect();
        assert_eq!(set0, [&Write::Int0Or(FRIENDS_MET, 1), &Write::Int0Or(FRIENDS_MET, 2)]);
        assert!(!intro.writes.contains(&Write::Bit1(KEEP_MUSIC, true)));
        assert_eq!(intro.writes.len(), 15);
        let leon = plan(ask(1101, 2002)).unwrap();
        assert!(leon.writes.iter().all(|w| !matches!(w, Write::Int0Or(..))));
        assert_eq!(leon.writes.iter().filter(|w| **w == Write::Bit1(KEEP_MUSIC, true)).count(), 1);
        assert_eq!(leon.writes.len(), 11);
        // Every session bit the exits write is cleared, except the music bit; the HUD mask to 0.
        for s in SKIPS {
            for w in s.writes {
                match *w {
                    Write::Bit1(id, v) => assert_eq!(v, id == KEEP_MUSIC, "{} {id:#x}", s.name),
                    Write::Int1(id, v) => assert_eq!((id, v), (HUD_HIDE, 0)),
                    Write::Int0Or(id, m) => assert!(id == FRIENDS_MET && (m == 1 || m == 2)),
                }
            }
            assert!(s.writes.contains(&Write::Bit1(0x2C, false)), "tutorial deck flag cleared: {}", s.name);
        }
    }

    #[test]
    fn floor_slot_guard() {
        assert!(floor_slot_ok(0, 0));
        assert!(floor_slot_ok(1, 3));
        assert!(!floor_slot_ok(0, -1));
        assert!(!floor_slot_ok(1, -1));
        assert!(floor_slot_ok(5, -1));
    }
}
