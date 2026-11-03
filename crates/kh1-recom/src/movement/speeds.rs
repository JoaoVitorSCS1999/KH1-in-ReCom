//! Sora's run speed, jump and air speed at `combat = kh1`, each KH1's or Re:CoM's (owner,
//! 2026-11-02: "have sora run at the same speed as he does in recom"; then "have run speed, jump
//! height, and air speed all be separate toggles in the settings"). The ini keys `run_speed`,
//! `jump_height`, `air_speed` (`recom | kh1`); `docs/recom-kh1-movement.md` § Speed toggles.
//!
//! Re:CoM's numbers. Checked (decomp + exe) unless marked:
//!
//! - Sora's parameter block (`player+0x3C0` = `0x1407C5B60`): walk `+0x10` 1.3 m/s, run `+0x14`
//!   6.2 m/s, jump speed `+0x1C` 28.0, its higher jump `+0x20` 32.5.
//! - Stick class `0x1401E9630` on Re:CoM's stick magnitude `+0x384` (the pad wrapper's
//!   `|(x, y)| / 4096`, at most 1, `0x1401FF150`): under 0.3 stand, under 0.9 walk (1.3 m/s), else
//!   **run = magnitude × 6.2 m/s**; the move step `0x1401EF3C0` moves at that / 60 a frame. The run
//!   row (3) plays at its clip's own rate: no reader of the speed `+0x4DC` sets a motion rate.
//! - Jump (`0x1401EDA60`, init `0x1401F29F0`): take-off speed = jump speed × `0x14087B39C`
//!   (0.6 × 0.008166667) m a frame, gravity `0x14087B398` (−0.6 × 0.008166667 m a frame², halved
//!   while CBTL `+0x208 & 0x4000`, `0x1401FAB40`). The take-off frame moves at the take-off speed;
//!   each later frame adds gravity first. Rising, the jump button up and 12 frames after the
//!   take-off (`+0x5DC` ≥ `+0x24` 100 + 600 units): the speed × 0.43 (`0x1407C5B3C`) and the
//!   fall begins; else the first frame the speed turns downward it is set to 0 (the top).
//! - Air (`0x1401EDA60` phases 2 / 3): the horizontal speed `+0x5CC` starts at 0.8 × the ground
//!   speed at the jump (`0x1401F29F0`; the whole speed after walking off, `0x1401F1F20`). With the
//!   stick: the facing eases toward the stick by 0.1 × step (`0x1401D4670`), f = 1 − |stick −
//!   facing| / π, the speed eases toward ground speed × f × 0.8 by 5 % × step, and when that left
//!   it above the target it is multiplied by f; Sora moves along the stick. Without: the speed
//!   eases to 0 by 5 % × step, along the facing.

use crate::settings::Mode;

/// The three toggles. `Default`: all KH1's (the KH1 controller as the oracle tests check it); the
/// runtime sets them from the ini.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Speeds {
    pub run: Mode,
    pub jump: Mode,
    pub air: Mode,
}

impl Default for Speeds {
    fn default() -> Speeds {
        Speeds { run: Mode::Kh1, jump: Mode::Kh1, air: Mode::Kh1 }
    }
}

/// Re:CoM's own numbers the toggles use, as Re:CoM holds them (the runtime reads them live).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecomNumbers {
    /// m/s (`params+0x14`).
    pub run_mps: f32,
    /// Take-off speeds (`params+0x1C`, `+0x20`).
    pub jump: f32,
    pub high_jump: f32,
    /// m a frame per unit of jump speed (`0x14087B39C`) and gravity, m a frame² (`0x14087B398`).
    pub rise_scale: f32,
    pub gravity: f32,
}

impl RecomNumbers {
    /// The Steam exe's values. Checked (exe: `0x1407C5B74` 6.2, `0x1407C5B7C` 28.0, `0x1407C5B80`
    /// 32.5, `0x1407C5B38` 0.6; decomp `0x1401FAB40`).
    pub const EXE: RecomNumbers = RecomNumbers { run_mps: 6.2, jump: 28.0, high_jump: 32.5, rise_scale: 0.6 * 0.008_166_667, gravity: -0.6 * 0.008_166_667 };
}

impl Default for RecomNumbers {
    fn default() -> RecomNumbers {
        RecomNumbers::EXE
    }
}

/// KH1 units a metre (`crate::units`).
const UNITS: f32 = 100.0;
/// Re:CoM's air numbers (`0x1401EDA60`, `0x1401F29F0`).
pub const AIR_KEEP: f32 = 0.8;
pub const AIR_EASE: f32 = 0.05;
pub const AIR_TURN: f32 = 0.1;
/// The jump cut (`0x1407C5B3C`) and its earliest frame after the take-off.
pub const CUT_KEEP: f32 = 0.43;
pub const CUT_AFTER: f32 = 12.0;
/// The mover's lowest vertical speed (`CMovement+0x144` = −0.5 m a frame), KH1 units a frame.
/// Checked (decomp, `docs/recom-kh1-movement.md` § The mover).
pub const FALL_LIMIT: f32 = 50.0;

/// Re:CoM's stick magnitude from its stick ints (`0x1401FF150`: `|(x, y)| × 2⁻¹²`, at most 1).
pub fn recom_tilt(x: i32, y: i32) -> f32 {
    let (x, y) = (x as f32 * 0.000_244_140_62, y as f32 * 0.000_244_140_62);
    (x * x + y * y).sqrt().min(1.0)
}

/// `run_speed = recom`: Re:CoM's run for its stick magnitude, KH1 units a frame.
pub fn recom_run(n: &RecomNumbers, tilt: f32) -> f32 {
    tilt.clamp(0.0, 1.0) * n.run_mps * UNITS / 60.0
}

/// `0x1401D4670(current, target, rate, step)`: eases an angle toward another by `rate × step` of
/// the gap (at once from 1 on, not at all at 0 or below; snaps when the move is below 0.1 % of
/// the rate). Symmetric, so it works in KH1's yaw as in Re:CoM's (π − yaw).
pub fn ease_angle(current: f32, target: f32, rate: f32, step: f32) -> f32 {
    use kh1_core::math::wrap_angle;
    let (c, t) = (wrap_angle(current), wrap_angle(target));
    if c == t {
        return t;
    }
    let k = rate * step;
    if k <= 0.0 {
        return c;
    }
    if k >= 1.0 {
        return t;
    }
    let d = wrap_angle(t - c);
    if k * 0.001 <= (d * k).abs() {
        wrap_angle(d * k + c)
    } else {
        t
    }
}

/// `air_speed = recom`: one frame of Re:CoM's horizontal air control. `speed`: the horizontal
/// speed now (KH1 units a frame); `stick`: the stick's yaw and Sora's ground speed for it (KH1
/// units a frame), None below the stand line. Returns the new speed, facing and the yaw Sora
/// moves along.
pub fn recom_air(speed: f32, facing: f32, stick: Option<(f32, f32)>, step: f32) -> (f32, f32, f32) {
    let Some((yaw, ground)) = stick else {
        let s = if speed != 0.0 { (0.0 - speed) * step * AIR_EASE + speed } else { speed };
        return (s, facing, facing);
    };
    let facing = ease_angle(facing, yaw, AIR_TURN, step);
    let f = 1.0 - kh1_core::math::wrap_angle(yaw - facing).abs() / std::f32::consts::PI;
    let target = ground * f * AIR_KEEP;
    let mut s = speed;
    if s != target {
        s = (target - s) * step * AIR_EASE + s;
        if target < s {
            s *= f;
        }
    }
    (s, facing, yaw)
}

/// `jump_height = recom`: Re:CoM's vertical jump from the take-off. Kept in Re:CoM's metres (its
/// float steps), handed out in KH1 units (y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecomRise {
    /// Upward speed, m a frame (Re:CoM's `+0x5D4`).
    pub up: f32,
    /// Gravity, m a frame² (negative: down; `+0x5D8`).
    pub gravity: f32,
    /// Frames since the take-off frame.
    pub frames: f32,
    /// Still in Re:CoM's rise (phase 2 before the top or the cut).
    pub rising: bool,
}

impl RecomRise {
    pub fn start(n: &RecomNumbers, high: bool) -> RecomRise {
        let v = if high { n.high_jump } else { n.jump };
        RecomRise { up: n.rise_scale * v, gravity: n.gravity, frames: 0.0, rising: true }
    }

    /// Frames from the take-off to the top without a cut (28 / 32.5 with the exe's numbers).
    pub fn rise_frames(&self) -> f32 {
        if self.gravity < 0.0 {
            self.up / -self.gravity
        } else {
            0.0
        }
    }

    /// One frame: the vertical displacement (KH1's y, down positive). `held`: the jump button.
    pub fn advance(&mut self, step: f32, held: bool) -> f32 {
        if self.frames > 0.0 {
            self.up += self.gravity * step;
            if self.rising {
                if !held && CUT_AFTER <= self.frames {
                    self.up *= CUT_KEEP;
                    self.rising = false;
                } else if self.up < 0.0 {
                    self.up = 0.0;
                    self.rising = false;
                }
            }
        }
        self.frames += step;
        if self.up < -FALL_LIMIT / UNITS {
            self.up = -FALL_LIMIT / UNITS;
        }
        -self.up * step * UNITS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::recom_axis_from_byte;
    use std::f32::consts::PI;

    #[test]
    fn run_is_recoms_6_2_mps_times_its_stick() {
        let n = RecomNumbers::EXE;
        assert!((recom_run(&n, 1.0) - 10.333_333).abs() < 1e-4);
        assert!((crate::units::speed_to_recom_mps(recom_run(&n, 1.0)) - 6.2).abs() < 1e-5);
        assert!((crate::units::speed_to_recom_mps(recom_run(&n, 0.9)) - 5.58).abs() < 1e-4);
        assert_eq!(recom_tilt(4096, 0), 1.0);
        assert_eq!(recom_tilt(4096, 4096), 1.0);
        assert!((recom_tilt(2048, 0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_kh1_run_tilt_is_always_faster_than_kh1s_run_at_recom() {
        // Wherever KH1's snap says run (its tilt over 0.8), Re:CoM's magnitude × 6.2 m/s is above
        // KH1's 8 units a frame: the run line checks (`run_speed <= w`) keep their meaning.
        let n = RecomNumbers::EXE;
        let p = kh1_core::Params::kh1_defaults();
        let mut slowest = f32::MAX;
        for bx in 0..=255u8 {
            for by in 0..=255u8 {
                let (ix, iy) = (recom_axis_from_byte(bx), recom_axis_from_byte(by));
                let raw = [crate::units::byte_from_recom_axis(ix), crate::units::byte_from_recom_axis(iy)];
                let (_, tilt) = kh1_core::params::stick_from_raw(raw[0], raw[1], crate::trace::KH1_DEAD_ZONE);
                if tilt > p.run_tilt {
                    slowest = slowest.min(recom_run(&n, recom_tilt(ix, iy)));
                }
            }
        }
        assert!(slowest > p.run_speed, "{slowest}");
    }

    #[test]
    fn jump_is_recoms_28_frames_to_1_99_m() {
        let n = RecomNumbers::EXE;
        let mut r = RecomRise::start(&n, false);
        assert!((r.rise_frames() - 28.0).abs() < 1e-3);
        let (mut y, mut top, mut frames_up) = (0.0f32, 0.0f32, 0);
        for _ in 0..200 {
            let rising = r.rising;
            y += r.advance(1.0, true);
            top = top.min(y);
            if rising {
                frames_up += 1;
            }
        }
        // 28 + 27 + … + 1 steps of 0.49: 198.9 units (KH1: 190).
        assert!((top + 198.94).abs() < 0.1, "{top}");
        // 28 frames of rise, then the frame whose speed is (within a float step of) 0.
        assert!((29..=30).contains(&frames_up), "{frames_up}");
        // The fall settles at the mover's limit.
        assert_eq!(r.up * 100.0, -FALL_LIMIT);
        let mut h = RecomRise::start(&n, true);
        assert!((h.rise_frames() - 32.5).abs() < 1e-3);
        let mut top = 0.0f32;
        let mut y = 0.0f32;
        for _ in 0..80 {
            y += h.advance(1.0, true);
            top = top.min(y);
        }
        // 32.5 + 31.5 + … + 0.5: 544.5 × 0.49 = 266.8 units (KH1: 290).
        assert!((top + 266.8).abs() < 0.1, "{top}");
    }

    #[test]
    fn released_jump_cuts_at_frame_12() {
        let n = RecomNumbers::EXE;
        let mut r = RecomRise::start(&n, false);
        let ups: Vec<f32> = (0..14).map(|_| -r.advance(1.0, false)).collect();
        let g = 0.49;
        assert!((ups[0] - 13.72).abs() < 1e-3);
        assert!((ups[11] - (13.72 - 11.0 * g)).abs() < 1e-3);
        assert!((ups[12] - (13.72 - 12.0 * g) * CUT_KEEP).abs() < 1e-3);
        assert!(!r.rising);
        // Held: no cut.
        let mut r = RecomRise::start(&n, false);
        for _ in 0..13 {
            r.advance(1.0, true);
        }
        assert!(r.rising);
    }

    #[test]
    fn air_eases_to_0_8_of_the_ground_speed() {
        let ground = recom_run(&RecomNumbers::EXE, 1.0);
        // Take-off: 0.8 of the run; straight ahead it stays there.
        let s0 = ground * AIR_KEEP;
        let (s, f, d) = recom_air(s0, 0.3, Some((0.3, ground)), 1.0);
        assert!((s - s0).abs() < 1e-5 && f == 0.3 && d == 0.3);
        // Standing jump: 5 % of the gap a frame.
        let (s, ..) = recom_air(0.0, 0.0, Some((0.0, ground)), 1.0);
        assert!((s - s0 * 0.05).abs() < 1e-5);
        // Stick released: 5 % less a frame, along the facing.
        let (s, f, d) = recom_air(10.0, 1.0, None, 1.0);
        assert!((s - 9.5).abs() < 1e-5 && f == 1.0 && d == 1.0);
        // Stick reversed: the facing eases 10 %, the speed collapses by f.
        let (s, f, d) = recom_air(s0, 0.0, Some((PI - 0.001, ground)), 1.0);
        assert!((f - 0.1 * (PI - 0.001)).abs() < 1e-4 && d == PI - 0.001);
        let k = 1.0 - (PI - 0.001 - f).abs() / PI;
        let want = ((ground * k * AIR_KEEP - s0) * 0.05 + s0) * k;
        assert!((s - want).abs() < 1e-4, "{s} {want}");
    }

    #[test]
    fn ease_angle_is_recoms() {
        assert!((ease_angle(0.0, 1.0, 0.1, 1.0) - 0.1).abs() < 1e-6);
        assert_eq!(ease_angle(0.0, 1.0, 0.1, 10.0), 1.0);
        assert_eq!(ease_angle(0.5, 1.0, 0.0, 1.0), 0.5);
        // Across ±π the short way.
        let a = ease_angle(3.0, -3.0, 0.5, 1.0);
        assert!((a - kh1_core::math::wrap_angle(3.0 + (2.0 * PI - 6.0) * 0.5)).abs() < 1e-5);
    }
}
