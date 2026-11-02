//! Re:CoM's own Dodge Roll at `combat = kh1` (owner, 2026-11-02: "make dodge roll work how it does
//! in ReCoM, not KH1"; "The whole roll"). `docs/recom-kh1-movement.md` § Re:CoM's roll.
//!
//! KH1's input rule still decides *when* the roll is asked for (KH1's Square on the ground, with
//! Guard equipped only at the run tilt; Attack's and the other commands' order in the frame; KH1's
//! locks). Then the KH1 frame hands Sora to Re:CoM's own avoid init `0x1401F10C0` (through its
//! trampoline, no hook runs), and Re:CoM's action 6 runs whole in its `CPlayerSora` exe
//! `0x140217310`: nothing of ours writes Sora until Re:CoM's action changes; the next KH1 frame
//! then takes over fresh. Checked (decomp) unless marked:
//!
//! - init `0x1401F10C0`: (footing case `+0x420 & 0x10400000 == 0x10000000` → `0x1401F2310(pl, 2)`
//!   instead); `+0x428 &= ~8`; **`+0x418 |= 1`** (its untouchable bit); action 6, sub 0, time 0;
//!   body velocity zeroed (`0x1401A38D0`); with the stick at 0.3 or more (`+0x384`) the facing
//!   turns to the stick's world yaw (`+0x38C`) at once; row 0x16 (`p001_070`) at the clip's own
//!   blend (−1; the clip stores 0) **with root motion**; vertical speed 0, phase 3, gravity
//!   `+0x5D8` = `0x14087B398`.
//! - exe `0x140217310` (each frame): keeps the body's horizontal velocity (zero: root motion
//!   moves Sora), integrates the vertical speed with gravity in the air (clamps it at 0 on the
//!   ground); **clears `+0x418` bit 0 once the motion clock (`ctrl+0x3C`, read at its start)
//!   reaches `params+0x28` = 600 units**; on the ground with the clock at `params+0x2C` = 1400
//!   units or more and the stick at 0.3 or more, it cancels into Re:CoM's stand / walk / run
//!   (`0x1401E9630`, `0x1401F8D70`, `0x1401F8AC0`); otherwise, when the clip ends (`ctrl+0x50 &
//!   3`), `0x1401F2890(pl, −1, 1)`: on the ground Re:CoM's stand init `0x1401F3180(pl, 1)`
//!   (stand / walk / run by the stick), in the air the fall `0x1401F1F20` (= jump init(pl, 1)).
//! - `params` = `player+0x3C0` = `0x1407C5B60`, whose first 0x30 bytes `CPlayerSora`'s constructor
//!   `0x140216CE0` copies from `0x1407C7CB0`: `+0x28` = 600, `+0x2C` = 1400. Checked (exe +
//!   decomp). (Riku's `0x1407C7BF0`: 2200 / 2200.)
//! - The clip `p001_070` (Re:CoM's `PL0001.ANN` slot 70): 30 fps, 19 frames, plays once, blend 0;
//!   root travel 3.5 m straight ahead, 0.3564 m a clip frame for clip frames 0–6, 0.2270 m for
//!   6–12, none after: 2.138 m in the first 12 game frames, 1.362 m in the next 12. That is KH1's
//!   roll travel (motion 0xDC: 213.82 + 136.18 cm in frames 0–12 / 12–24; `kh1-sora-data.md`):
//!   Re:CoM's clip is a bake of KH1's. Checked (files: Re:CoM's own `PL0001.ANN`).
//! - The damage check `0x1401E6DE0` runs in both state exes after the request bits and before the
//!   action switch (`0x1401FA0F0`, `0x1401F9840`), and skips a hit while `+0x418 & 1` (no damage,
//!   no reaction). So the bit set by the init (in the switch) first protects the next frame, and
//!   the exe that clears it runs after that frame's damage check.
//! - A clip started by a request does not advance on its first update (`docs/recom-kh1-movement.md`
//!   § Motions): the exe of game frame k (the press frame is 0) reads 50 × (k − 1) units at step 1,
//!   so the bit is cleared in frame 13's exe (13 protected frames: 1–13) and the moving cancel is
//!   first possible in frame 29; the clip (19 × 100 units, one 30 fps frame less as it plays once:
//!   1800 units) ends in frame 37. Derived from the above; the log reports the frames in game.

/// Re:CoM's avoid action and its roll sub-action (`player+0x40C` / `+0x414`).
pub const ACTION: u32 = super::action::AVOID;
pub const SUB: u32 = super::action::SUB_ROLL;
/// `player+0x418` bit 0: Re:CoM's untouchable bit, set by its avoid init, cleared by its exe.
pub const UNTOUCHABLE: u32 = 1;
/// Sora's `params+0x28` / `+0x2C` in Re:CoM motion units (50 a 60 Hz frame). Checked (exe).
pub const UNTOUCHABLE_UNITS: i32 = 600;
pub const CANCEL_UNITS: i32 = 1400;
/// Re:CoM's row and clip for it.
pub const ROW: u32 = 0x16;
pub const CLIP: &str = "p001_070";
/// The stick magnitude (`player+0x384`) from which Re:CoM's init faces the stick and its exe
/// cancels into movement.
pub const STICK: f32 = 0.3;

/// Re:CoM runs the roll of this player now: the KH1 frame handed it over and Re:CoM's action is
/// still its roll.
pub fn recom_owns(owner: usize, player: usize, action: u32, sub: u32) -> bool {
    owner != 0 && owner == player && action == ACTION && sub == SUB
}

/// Most roll start / end lines written to `kh1-recom.log` (a roll is a start and an end line).
pub const LOG_LINES: u32 = 1000;

/// What one Re:CoM roll did, for its two log lines (start, end). Not read by anything else.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Watch {
    /// The action the roll started from.
    pub from: u32,
    /// 60 Hz frames since the press frame: the sum of Re:CoM's step (`player+0x464`) over each of
    /// Re:CoM's exe calls.
    pub frames: f32,
    /// Exe calls (game frames at any step).
    pub calls: u32,
    /// `frames` at the exe call that cleared Re:CoM's untouchable bit: the hits of frames 1 ..=
    /// this one were skipped by its damage check.
    pub untouchable: Option<f32>,
    /// The motion clock (units) the clearing exe read.
    pub untouchable_at: Option<i32>,
    /// The motion clock the last exe read.
    pub clock: i32,
}

impl Watch {
    pub fn start(from: u32) -> Watch {
        Watch { from, ..Default::default() }
    }

    /// One call of Re:CoM's exe: the step, the untouchable bit before / after it, and the motion
    /// clock it read.
    pub fn exe(&mut self, step: f32, bit_before: bool, bit_after: bool, clock: i32) {
        self.frames += step;
        self.calls += 1;
        self.clock = clock;
        if bit_before && !bit_after && self.untouchable.is_none() {
            self.untouchable = Some(self.frames);
            self.untouchable_at = Some(clock);
        }
    }

    pub fn start_line(&self, facing: f32, stick: f32, bit: bool) -> String {
        format!(
            "roll: Re:CoM's roll started (its avoid init 0x1401F10C0 from action {:#x}): action 6 sub 0, row {ROW:#x} {CLIP} with root motion, facing {facing:.3} rad (stick {stick:.2}; Re:CoM turns to it from {STICK}), untouchable bit {}",
            self.from,
            if bit { "set" } else { "NOT set" }
        )
    }

    /// `now`: Re:CoM's action after the roll (2 / 3 / 4 stand / walk / run, 7 the fall, else what
    /// took over), and `why`.
    pub fn end_line(&self, now: u32, why: &str) -> String {
        let inv = match (self.untouchable, self.untouchable_at) {
            (Some(f), Some(u)) => format!("untouchable on {f:.0} frames (Re:CoM cleared the bit at motion clock {u} units = {:.1} frames; its limit {UNTOUCHABLE_UNITS})", crate::units::units_to_frames(u)),
            _ => "the untouchable bit was not cleared by Re:CoM's exe".to_string(),
        };
        format!(
            "roll: Re:CoM's roll ended ({why}): {:.0} frames after the press frame ({} exe calls, last motion clock {} units = {:.1} frames), {inv}; Re:CoM's action now {now:#x}",
            self.frames,
            self.calls,
            self.clock,
            crate::units::units_to_frames(self.clock)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recoms_numbers_in_frames() {
        assert_eq!(crate::units::units_to_frames(UNTOUCHABLE_UNITS), 12.0);
        assert_eq!(crate::units::units_to_frames(CANCEL_UNITS), 28.0);
        assert_eq!((ACTION, SUB), (6, 0));
    }

    #[test]
    fn only_recoms_roll_of_this_player() {
        assert!(recom_owns(0x10, 0x10, 6, 0));
        assert!(!recom_owns(0x10, 0x10, 6, 1), "our Guard is action 6 sub 1");
        assert!(!recom_owns(0x10, 0x10, 2, 0), "Re:CoM's roll is over");
        assert!(!recom_owns(0x10, 0x20, 6, 0), "another player");
        assert!(!recom_owns(0, 0, 6, 0), "no roll handed over");
    }

    /// The exe's clock as Re:CoM runs it at step 1: frame k's exe reads 50 × (k − 1) (the first
    /// update doesn't advance), clears the bit once it reads 600.
    #[test]
    fn watch_counts_the_untouchable_frames() {
        let mut w = Watch::start(3);
        let mut bit = true;
        for k in 1..=37 {
            let clock = 50 * (k - 1);
            let after = bit && clock < UNTOUCHABLE_UNITS;
            w.exe(1.0, bit, after, clock);
            bit = after;
        }
        assert_eq!(w.untouchable, Some(13.0));
        assert_eq!(w.untouchable_at, Some(600));
        assert_eq!((w.frames, w.calls), (37.0, 37));
        let l = w.end_line(2, "Re:CoM's stand init");
        assert!(l.contains("37 frames") && l.contains("untouchable on 13 frames") && l.contains("action now 0x2"), "{l}");
        // At Re:CoM's 30 fps step (2.0) the same clock is reached in half the calls.
        let mut w = Watch::start(4);
        let mut bit = true;
        for k in 1..=19 {
            let clock = 100 * (k - 1);
            let after = bit && clock < UNTOUCHABLE_UNITS;
            w.exe(2.0, bit, after, clock);
            bit = after;
        }
        assert_eq!(w.untouchable, Some(14.0));
        assert!(w.start_line(0.5, 1.0, true).contains("bit set"));
    }
}
