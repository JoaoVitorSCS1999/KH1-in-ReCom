//! Bosses untouchable during the moves that dim the room (owner, 2026-11-03: "anytime bosses use a
//! move that dims the room, just make them invulnerable for that move"), at Combat = KH1.
//! `docs/recom-boss-dim.md`.
//!
//! The dim is Re:CoM's scene colour fade (Checked (decomp + exe)): `0x140232170(mask, frames,
//! &rgba, start, slot)` sets one of `CSceneEnvTask`'s four colour slots (the table at
//! `[0x14087B7F0]`, 0x40 bytes each), which the task fades over `frames` 60 Hz frames; `CMap`
//! (`0x1401CFDC0`, layers 11 and 12) and `CSky` (`0x1401D0710`, layer 12) multiply their colour by
//! every slot whose layer mask holds their layer (`0x140231E30`). A boss move dims the room by
//! asking for a colour under 1.0 on those layers (0.5 grey on mask 0x1800 for most), and its state
//! end asks for 1.0 again.
//!
//! Here: a boss attack state during which the boss's own code asks Re:CoM for a dark room is the
//! boss's untouchable window (the revenge move's: `crate::revenge`, Re:CoM's own rule for an
//! attacking enemy), from that request to the state's end. The request is seen by comparing the
//! slot before and after the boss's own code runs: its state begin (a snapshot at the state
//! machine's change callback, compared when the begin's "hurt bodies off" runs) and, for the four
//! states that ask in their step ([`EXES`]), its step. Marluxia 3 has a slot of its own (1) that its
//! idle think darkens before some of its attacks: an attack state that begins while it is dark is a
//! dim move too ([`own_slot`]).

/// The scene colour table's global (`CSceneEnvTask`'s four slots), RVA. Checked (exe: the setter's
/// `add rax,[rip+0x649670]` at `0x140232179`).
pub const ENV_TABLE: u32 = 0x0087_B7F0;
pub const SLOTS: usize = 4;
pub const SLOT_SIZE: usize = 0x40;
/// A slot's fields (Checked (decomp `0x140232170`, `0x140232060`)).
pub mod slot {
    /// The colour the fade starts from (rgba f32).
    pub const START: usize = 0;
    /// The colour asked for.
    pub const TARGET: usize = 0x10;
    /// The colour now.
    pub const CURRENT: usize = 0x20;
    /// The layers it colours (bit per layer; cleared when a fade back to 1.0 ends).
    pub const MASK: usize = 0x30;
    /// The fade's progress, 0 .. 1 (0 on every request).
    pub const T: usize = 0x34;
    /// 1 / frames.
    pub const RATE: usize = 0x38;
    /// Bit 0: a request not yet finished.
    pub const FLAGS: usize = 0x3C;
}
/// The room's layers: `CMap`'s 11 and 12 (`CSky` is 12). Checked (decomp).
pub const ROOM_LAYERS: u32 = 0x1800;

/// One colour slot, read from its 0x40 bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub target: [f32; 4],
    pub mask: u32,
    pub t: f32,
    pub flags: u32,
}

fn f32_at(b: &[u8; SLOT_SIZE], at: usize) -> f32 {
    f32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}
fn u32_at(b: &[u8; SLOT_SIZE], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

impl Slot {
    pub fn parse(b: &[u8; SLOT_SIZE]) -> Slot {
        let t = slot::TARGET;
        Slot { target: [f32_at(b, t), f32_at(b, t + 4), f32_at(b, t + 8), f32_at(b, t + 12)], mask: u32_at(b, slot::MASK), t: f32_at(b, slot::T), flags: u32_at(b, slot::FLAGS) }
    }

    /// The slot darkens the room: it colours the room's layers and asks for a colour under 1.0.
    pub fn dark(&self) -> bool {
        self.mask & ROOM_LAYERS != 0 && self.target[..3].iter().any(|&c| c < 1.0)
    }

    /// The darkest channel asked for (for the log).
    pub fn level(&self) -> f32 {
        self.target[..3].iter().cloned().fold(1.0, f32::min)
    }
}

/// Whether the code that ran between `before` and `after` asked Re:CoM for a dark room in this slot:
/// the slot changed, holds a fresh request (`0x140232170` zeroes the progress and sets bit 0) and the
/// request is dark.
pub fn requested(before: &[u8; SLOT_SIZE], after: &[u8; SLOT_SIZE]) -> bool {
    let a = Slot::parse(after);
    before != after && a.t == 0.0 && a.flags & 1 != 0 && a.dark()
}

/// Why a window opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The state's begin asked for the dark room.
    Begin,
    /// The state's step asked for it.
    Step,
    /// The boss's own slot was already dark when the attack state began (Marluxia 3).
    OwnSlot,
}

/// At an attack state's begin: whether the begin dimmed the room (`before`: slot 0 at the state
/// change, `after`: now), or the boss's own slot is dark ([`own_slot`]).
pub fn at_begin(before: Option<&[u8; SLOT_SIZE]>, after: &[u8; SLOT_SIZE], own: Option<&[u8; SLOT_SIZE]>) -> Option<Cause> {
    if before.is_some_and(|b| requested(b, after)) {
        return Some(Cause::Begin);
    }
    own.is_some_and(|o| Slot::parse(o).dark()).then_some(Cause::OwnSlot)
}

/// The slot the bosses' begins and steps use (`[rsp+0x20]` = 0 at each call). Checked (exe).
pub const BOSS_SLOT: usize = 0;

/// A boss's own colour slot in battle (Marluxia 3, id 132: slot 1; in battle only its code and its
/// part `CBossMarluxia3AllRangeBit` write it, the other writers are field menus: `CRoomCreation`,
/// `CWorldSelect` and their neighbours). Checked (exe: every call's slot argument).
pub fn own_slot(boss_id: u16) -> Option<usize> {
    (boss_id == 132).then_some(1)
}

/// A boss attack state whose step asks for the dark room partway through: its exe, written in the
/// class's vtable (`+0x120 + (state − 2) × 0x18`), runs ours, which runs Re:CoM's and compares the
/// slot.
#[derive(Clone, Copy, Debug)]
pub struct Exe {
    pub boss_id: u16,
    /// The class's vtable (RVA).
    pub vtable: u32,
    pub state: i32,
    /// Re:CoM's exe (RVA).
    pub exe: u32,
    /// The colour slot it asks in.
    pub slot: usize,
    pub what: &'static str,
}

impl Exe {
    /// The vtable slot (RVA) holding the exe.
    pub fn slot_rva(&self) -> u32 {
        exe_slot(self.vtable, self.state)
    }
}

/// The vtable slot of an attack state's exe (states 2..=10: begin `+0x118`, exe `+0x120`, end
/// `+0x128`, 0x18 a state). Checked (exe: Re:CoM's state handlers `jmp [rax+0x138]` (3),
/// `[rax+0x150]` (4), `[rax+0x198]` (7), `[rax+0x1C8]` (9)).
pub fn exe_slot(vtable: u32, state: i32) -> u32 {
    vtable + 0x120 + (state as u32 - 2) * 0x18
}

/// Checked (decomp + exe): each exe's call of the dim (Larxene's through her helper `0x1403B8AB0`,
/// grey 0.5 over 30 frames on mask 0x1800; Marluxia 3's direct, 0.5 on mask 0x1000, slot 1).
pub const EXES: [Exe; 4] = [
    Exe { boss_id: 117, vtable: 0x0066_6598, state: 3, exe: 0x003B_BAF0, slot: 0, what: "combo (e117_050..053), the room dims for its lightning end (sub-step 4, 68 frames into the last swing)" },
    Exe { boss_id: 117, vtable: 0x0066_6598, state: 4, exe: 0x003B_C1D0, slot: 0, what: "hop back / close in, the room dims at its lightning (e117_055 / 056)" },
    Exe { boss_id: 132, vtable: 0x0067_85B8, state: 7, exe: 0x003F_5F90, slot: 1, what: "the room (slot 1) dims after its first motion" },
    Exe { boss_id: 132, vtable: 0x0067_85B8, state: 9, exe: 0x003F_6960, slot: 1, what: "wind (WindPow / WinsTime): the room (slot 1) dims when it starts" },
];

/// The moves the decomp shows dimming the room, for the log (Checked (decomp): each class's state
/// begin / step calls `0x140232170` with a colour under 1.0; the names are the state's motions /
/// parameters, what each move is: Assumption).
pub const MOVES: &[(u16, i32, &str)] = &[
    (116, 2, "e116_050 (dims only when its spawn value +0x294 > 0)"),
    (116, 5, "p003_041 / p003_042"),
    (116, 6, "p003_043"),
    (116, 7, "p003_044..048 (DACount / DASpeed: Dark Aura)"),
    (117, 3, "combo e117_050..053, the room dims at its lightning end"),
    (117, 4, "hop back, then lightning e117_055 / 056"),
    (117, 5, "e117_057"),
    (117, 8, "e117_008..010 / e117_065"),
    (118, 4, "warp, then a flame burst around him (e118_003 / 004 / 060 / 061)"),
    (118, 5, "e118_062..066 (Firecount / Boundcount: bouncing fire)"),
    (119, 5, "e119_057"),
    (119, 6, "e119_058..061"),
    (119, 7, "e119_063..066"),
    (120, 6, "state 6 (16-frame dim)"),
    (132, 7, "state 7 (slot 1)"),
    (132, 9, "wind (WindPow / WinsTime, slot 1)"),
];

pub fn what(boss_id: u16, state: i32) -> &'static str {
    MOVES.iter().find(|m| m.0 == boss_id && m.1 == state).map_or("a move not in the table", |m| m.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot_bytes(target: [f32; 4], mask: u32, t: f32, flags: u32) -> [u8; SLOT_SIZE] {
        let mut b = [0u8; SLOT_SIZE];
        for (i, c) in target.iter().enumerate() {
            b[slot::TARGET + i * 4..slot::TARGET + i * 4 + 4].copy_from_slice(&c.to_le_bytes());
        }
        b[slot::MASK..slot::MASK + 4].copy_from_slice(&mask.to_le_bytes());
        b[slot::T..slot::T + 4].copy_from_slice(&t.to_le_bytes());
        b[slot::FLAGS..slot::FLAGS + 4].copy_from_slice(&flags.to_le_bytes());
        b
    }

    const GREY: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
    const WHITE: [f32; 4] = [1.0; 4];

    #[test]
    fn a_dark_request_on_the_room_is_a_dim() {
        let idle = slot_bytes(WHITE, 0, 1.0, 0);
        assert!(requested(&idle, &slot_bytes(GREY, 0x1800, 0.0, 1)));
        // Marluxia's 0x1808 and Marluxia 3's 0x1000 colour the room too.
        assert!(requested(&idle, &slot_bytes([0.6, 0.6, 0.6, 1.0], 0x1808, 0.0, 1)));
        assert!(requested(&idle, &slot_bytes(GREY, 0x1000, 0.0, 1)));
        // Ansem's near-black.
        assert!(requested(&idle, &slot_bytes([0.15, 0.13, 0.15, 1.0], 0x1800, 0.0, 1)));
        assert_eq!(Slot::parse(&slot_bytes([0.6, 0.5, 0.7, 1.0], 0x1800, 0.0, 1)).level(), 0.5);
    }

    #[test]
    fn restores_and_other_layers_are_not() {
        let dim = slot_bytes(GREY, 0x1800, 1.0, 0);
        // The state end's restore (1.0).
        assert!(!requested(&dim, &slot_bytes(WHITE, 0x1800, 0.0, 1)));
        // An actor's own fade (Guard Armor's mask 2, alpha 0) is not the room.
        assert!(!requested(&dim, &slot_bytes([1.0, 1.0, 1.0, 0.0], 2, 0.0, 1)));
        // Nothing asked: the same bytes, or a fade under way (progress past 0).
        assert!(!requested(&dim, &dim));
        assert!(!requested(&slot_bytes(WHITE, 0, 1.0, 0), &slot_bytes(GREY, 0x1800, 0.3, 1)));
    }

    #[test]
    fn begin_and_own_slot() {
        let before = slot_bytes(WHITE, 0, 1.0, 0);
        let dimmed = slot_bytes(GREY, 0x1800, 0.0, 1);
        assert_eq!(at_begin(Some(&before), &dimmed, None), Some(Cause::Begin));
        // A dim already up before the begin (Sora's sleight) is not the boss's.
        assert_eq!(at_begin(Some(&dimmed), &dimmed, None), None);
        assert_eq!(at_begin(None, &dimmed, None), None);
        // Marluxia 3: its own slot dark at the begin.
        let own_dark = slot_bytes(GREY, 0x1000, 1.0, 0);
        assert_eq!(at_begin(Some(&before), &before, Some(&own_dark)), Some(Cause::OwnSlot));
        assert_eq!(at_begin(Some(&before), &before, Some(&slot_bytes(WHITE, 0x1000, 0.5, 1))), None);
        assert_eq!(own_slot(132), Some(1));
        assert_eq!(own_slot(117), None);
    }

    #[test]
    fn exe_slots() {
        // Checked (exe): the four slots hold these exes.
        let rvas: Vec<(u32, u32)> = EXES.iter().map(|x| (x.slot_rva(), x.exe)).collect();
        assert_eq!(rvas, vec![(0x0066_66D0, 0x003B_BAF0), (0x0066_66E8, 0x003B_C1D0), (0x0067_8750, 0x003F_5F90), (0x0067_8780, 0x003F_6960)]);
        for x in &EXES {
            let b = crate::revenge::BOSSES.iter().find(|b| b.id == x.boss_id).expect("a revenge boss");
            assert_eq!(b.vtable, x.vtable, "{}", x.what);
            assert_eq!(x.slot, own_slot(x.boss_id).unwrap_or(BOSS_SLOT));
            assert!(MOVES.iter().any(|m| m.0 == x.boss_id && m.1 == x.state));
        }
        // Every boss in the move table has a tail (a revenge boss class).
        for m in MOVES {
            assert!(crate::revenge::BOSSES.iter().any(|b| b.id == m.0), "{}", m.2);
        }
        assert_eq!(what(118, 5), MOVES[9].2);
        assert_eq!(what(106, 4), "a move not in the table");
    }
}
