//! Oogie Boogie at Combat = KH1 (owner, 2026-11-03: "oogie boogie is impossible to defeat. you're
//! supposed to break one of his attacks which lowers a gate where you can hit him. can we make it so
//! that if he uses the attack that you're supposed to break twice without being broken, the gate
//! lowers?"; then the ask re-sent without "twice" and "After once": each time it ends unbroken).
//! `docs/recom-m6-battles.md` § Oogie Boogie. The runtime is
//! `win/battle.rs` (`attack_end`, `oogie_broken_begin`, `install_oogie`).
//!
//! Re:CoM (Checked (decomp + exe) unless tagged):
//! - `CBossBoogie` (vtable `0x140670A60`). His **state 2** is the dice throw (begin `0x1403D7830`:
//!   `+0xCD8 |= 1`, motion 0x33; step `0x1403D9340`: the three dice at motion frame 0xED8, then the
//!   attack end `0x14017E540(boss, 0, 6000)` at `0x1403D9402` once a die has landed (`+0xC88 & 2`)
//!   or all are gone, or after his stomp in the low-HP variant). It is the only attack whose card
//!   break lowers the gate: his **state 13 begin** `0x1403D7A40` (the card-break stagger,
//!   `0x14017BAA0`) does the gate work only while `+0xCD8 & 1` (state 2's mark; his state 1 begin
//!   `0x1403D7EE0` clears it): the dice out (or thrown if none) and marked `+0xC88 |= 0x41` (they
//!   may drop his gimmick card when destroyed, `0x1403D7C90`), then **the gate's count**
//!   (`*(boss+0xD10)+0x2E0`) + 1, at most 3; at 3 his down time `+0xCF8` = `HDownTime`, the bit
//!   `+0x260 & 0x2000` cleared and `+0x154 |= 0x10000`.
//! - The gate's own step (`0x1403DA7C0`, the exe of `CGimmickBBGHedge`, vtable `0x140671318`) lowers
//!   one step (motions 2, 3, 4) each time the count passes its shown step `+0x2E4`, and turns its
//!   collision off after the third (`0x1401A64B0(hedge, 0)`); count back to 0: Sora is pushed out
//!   (z ≥ −7.5), collision on, motion 5 (it rises).
//! - The count goes back to 0 when he jumps back after his down time (`0x1403D5B70` step 5) and at
//!   the fight's script start (state 0, event variable 0xE1 = 4). His gimmick card (`0x1403D8FA0`)
//!   adds 3 at once.
//! - Re:CoM's card break of a boss is CBTL's message 3 to his handler (base `0x140181620`, his vtable
//!   `+0x48`): case 3 → next state 13 (`boss + 0x800`).
//!
//! At Combat = KH1 nothing breaks him: Sora plays no cards, and our KH1 break triggers (finisher
//! interrupt, Guard block) and the HP drops are for the five card-break bosses only
//! (`win::gimmick`, by vtable), not him. So the gate never lowered.
//!
//! Built: when his state 2 ends through Re:CoM's attack end unbroken, the count of unbroken throws
//! goes up; at [`THROWS_TO_LOWER`] (1, owner) Re:CoM's own card break runs at once: his handler
//! (`+0x48`) is called with message 3 and no record, as CBTL's message manager delivers it. His
//! state 13 then runs from his state 2 with the mark still set, so Re:CoM's state-13 begin lowers
//! the gate one step (three steps open it, as three breaks in Re:CoM) and staggers him, as a break.

/// `CBossBoogie`'s vtable (RVA) and RTTI name. Checked (exe: the RTTI locator at `vt − 8`).
pub const VTABLE: u32 = 0x0067_0A60;
pub const RTTI: &str = ".?AVCBossBoogie@@";

/// His dice throw (state 2) and the card-break stagger (state 13).
pub const DICE_STATE: usize = 2;
pub const BROKEN_STATE: usize = crate::gimmick::STATE_BROKEN;

/// His vtable slot of the state-13 begin (`+0xE8 + 0x18 · 13`), ours (`win::battle::oogie_broken_begin`,
/// which calls his `0x1403D7A40`: for its log lines and the count).
pub const BROKEN_BEGIN_SLOT: usize = crate::gimmick::STAGGER_SLOT as usize;

/// Re:CoM's card-break message (the resolver `0x14015EAC0` posts 3 for a broken card).
pub const BREAK_MESSAGE: i32 = crate::gimmick::MESSAGE_BROKEN as i32;

/// State 2's mark (`+0xCD8 & 1`): state 13's begin lowers the gate only while it is set.
pub const MARK_OFF: usize = 0xCD8;
pub const MARK_BIT: u32 = 1;

/// The gate object (`+0xD10`), its count (`+0x2E0`, 0..=3) and the step it shows (`+0x2E4`).
pub const GATE_OFF: usize = 0xD10;
pub const GATE_COUNT: usize = 0x2E0;
pub const GATE_SHOWN: usize = 0x2E4;
/// The count at which the gate is fully down (Re:CoM's clamp `cmp eax,3` at `0x1403D7AF1`).
pub const GATE_DOWN: i32 = 3;
/// His down time (`+0xCF8`, seconds as a float, counted down by his update `0x1403DB770`).
pub const DOWN_TIME_OFF: usize = 0xCF8;

/// Unbroken dice throws that lower the gate one step (owner, 2026-11-03: "After once").
pub const THROWS_TO_LOWER: u32 = 1;

/// One call of Re:CoM's attack end on an enemy, read after it.
#[derive(Clone, Copy, Debug)]
pub struct End {
    /// This battle is a KH1 one (Combat = KH1, Sora's story).
    pub kh1_battle: bool,
    /// The enemy is Oogie (Re:CoM's own vtable, RTTI checked).
    pub oogie: bool,
    /// His current state (`+0x60`) is state 2, the dice throw: its step is the only caller of the
    /// attack end in that state (`0x1403D9402`).
    pub dice_state: bool,
    /// The attack really ended (`+0x260 & 1` clear after the call: not Re:CoM's enchant-0x72 repeat).
    pub ended: bool,
    /// HP above 0 (Re:CoM's handler ignores a defeated enemy).
    pub alive: bool,
    /// State 2's mark is set (state 13's begin needs it to lower the gate).
    pub marked: bool,
    /// The gate's count (`+0x2E0`).
    pub gate: i32,
}

/// What follows the attack end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Not his unbroken dice throw: nothing.
    Pass,
    /// An unbroken throw counted, not yet enough.
    Count(u32),
    /// The gate is already fully down: nothing to lower (a break would only restart his down time).
    Down,
    /// Re:CoM's card break now (message 3 → state 13 → the gate one step down).
    Break,
}

/// The rule on an attack end: returns the new count of unbroken throws and the step. The count
/// starts again after each break (ours or a real one).
pub fn after_end(e: End, count: u32) -> (u32, Step) {
    if !(e.kh1_battle && e.oogie && e.dice_state && e.ended && e.alive && e.marked) {
        return (count, Step::Pass);
    }
    if e.gate >= GATE_DOWN {
        return (0, Step::Down);
    }
    let n = count + 1;
    if n >= THROWS_TO_LOWER {
        (0, Step::Break)
    } else {
        (n, Step::Count(n))
    }
}

/// His state-13 begin, read before Re:CoM's: ours (posted by [`after_end`]) or a real break. Either
/// way the count of unbroken throws starts again.
pub fn after_break(_ours: bool, _count: u32) -> u32 {
    0
}

pub fn count_line(battle: u32, n: u32, gate: i32) -> String {
    format!("oogie (battle {battle}): his dice throw (state 2) ended unbroken: {n} of {THROWS_TO_LOWER}; gate count {gate} of {GATE_DOWN}")
}

pub fn break_line(battle: u32, gate: i32, shown: i32, sent: bool) -> String {
    if sent {
        format!(
            "oogie (battle {battle}): his dice throw (state 2) ended unbroken ({THROWS_TO_LOWER} of {THROWS_TO_LOWER}): Re:CoM's card break (his handler, message 3 → state 13) so his state-13 begin 0x1403D7A40 lowers the gate (count {gate} of {GATE_DOWN}, shown step {shown})"
        )
    } else {
        format!("oogie (battle {battle}): his dice throw ended unbroken but his handler did not take message 3 (next state not 13): the gate stays (count {gate} of {GATE_DOWN})")
    }
}

pub fn down_line(battle: u32, gate: i32) -> String {
    format!("oogie (battle {battle}): his dice throw ended unbroken with the gate already down (count {gate} of {GATE_DOWN}): nothing to lower")
}

pub fn broken_line(battle: u32, ours: bool, marked: bool, before: i32, after: i32, down_time: f32) -> String {
    let who = if ours { "ours (an unbroken throw)" } else { "a real card break" };
    if !marked {
        return format!("oogie (battle {battle}): state 13 (card break, {who}) outside his dice throw: Re:CoM's stagger only, the gate stays (count {after} of {GATE_DOWN})");
    }
    let down = if after >= GATE_DOWN { format!("; the gate is down: his down time {down_time:.1} (HDownTime), he can be hit") } else { String::new() };
    format!("oogie (battle {battle}): state 13 (card break, {who}): Re:CoM's state-13 begin lowered the gate: count {before} -> {after} of {GATE_DOWN}{down}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end() -> End {
        End { kh1_battle: true, oogie: true, dice_state: true, ended: true, alive: true, marked: true, gate: 0 }
    }

    #[test]
    fn one_unbroken_throw_breaks_and_starts_again() {
        assert_eq!(THROWS_TO_LOWER, 1);
        assert_eq!(after_end(end(), 0), (0, Step::Break));
        // Three throws, three breaks: each lowers one step (Re:CoM's state 13 does the counting).
        let mut c = 0;
        for gate in 0..GATE_DOWN {
            let (n, s) = after_end(End { gate, ..end() }, c);
            assert_eq!(s, Step::Break);
            c = n;
        }
        assert_eq!(c, 0);
    }

    #[test]
    fn only_his_unbroken_dice_throw_counts() {
        for e in [
            End { kh1_battle: false, ..end() },
            End { oogie: false, ..end() },
            End { dice_state: false, ..end() },
            End { ended: false, ..end() },
            End { alive: false, ..end() },
            End { marked: false, ..end() },
        ] {
            assert_eq!(after_end(e, 0), (0, Step::Pass));
        }
    }

    #[test]
    fn nothing_to_lower_when_down() {
        assert_eq!(after_end(End { gate: GATE_DOWN, ..end() }, 0), (0, Step::Down));
    }

    #[test]
    fn a_break_starts_the_count_again() {
        assert_eq!(after_break(true, 0), 0);
        assert_eq!(after_break(false, 0), 0);
    }

    #[test]
    fn slots_and_states() {
        assert_eq!(BROKEN_BEGIN_SLOT, 0x220);
        assert_eq!(crate::gimmick::state_at(BROKEN_STATE), 0x800);
        assert_eq!(crate::gimmick::state_at(DICE_STATE), 0x540);
    }
}
