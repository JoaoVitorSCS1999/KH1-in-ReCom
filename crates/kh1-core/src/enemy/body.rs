//! An enemy actor's movement physics: what KH1's actor task does every frame, whatever the
//! script does (moved from the Shadow's port, M10; unchanged). Checked (decomp) unless marked.

use super::Engine;
use crate::arc::Arc;
use crate::math::{turn_toward, Vec3};

/// The engine side of an enemy's movement: turning toward the target yaw, the ground walk the
/// script set this frame, the air arc with its horizontal velocity, and the push slide of a
/// flinch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Body {
    /// Facing (`+0x3C`) and the yaw the engine turns to (`+0x150`) at the turn rate.
    pub yaw: f32,
    pub yaw_target: f32,
    /// This frame's walk velocity (`+0x20` × `+0x2C`): set by the AI's moves, zeroed each frame
    /// (KH1's actor task zeroes `+0x2C`, `0x140295E50` in the Epic notes; Assumption for Steam).
    pub walk: Vec3,
    /// Movement state 2 (in the air): the arc and the air velocity `+0x3C0`.
    pub arc: Option<Arc>,
    pub air_vel: Vec3,
    /// The push slide (`+0x3F0` direction, `+0x3FC` speed; `0x1402C7300` / `0x1402C7070`).
    pub push: Option<(Vec3, f32)>,
    /// KH1's actor flag `0x1000` (a motion request with bit 23, `0x14029F980`): while it is set,
    /// the air step (`0x1402AF6A0`) moves nothing itself and keeps the arc's base at the body's
    /// height (the motion's root moves it). The host clears it when that motion ends. Not used by
    /// the Shadow (M11).
    pub held: bool,
    /// KH1's movement state 4, flight (`0x1402CC000`, registered by `0x1402CBF80` with flags 2;
    /// entered by `sys 135` → `0x1402BACB0`), for the flying enemies (M11 fliers). While it is set,
    /// [`Body::walk`] is the script's `+0x20` × `+0x2C` (direction and speed, its `y` too) and the
    /// flight velocity `air_vel` (`+0x3C0`) follows it: horizontally `v·(1 − a) + want·a` with `a`
    /// the stats block's `+0x14` (`air_accel`; `0x140386B70` → `0x1400E7CB0(out, want, v, a)`,
    /// Checked (decomp + disasm)), vertically `want.y` at once (`+0x3C4`); the displacement is
    /// `v × step`. No gravity, no ground walk. An arc ([`Body::jump`]) leaves it (state 2).
    pub flying: bool,
    /// The turn rate a script set (`sys 125` → `0x1402BABA0`: the stats block's `+0xD0` = degrees
    /// × π / 180), else the manifest's (`Engine::turn_rate`). M11 fliers.
    pub turn: Option<f32>,
}

impl Body {
    /// `0x1402AF960(h)` + the horizontal: a jump of `h` with air velocity `v`.
    pub fn jump<C: Engine + ?Sized>(&mut self, h: f32, v: Vec3, y: f32, c: &C) {
        self.arc = Some(Arc::hop(h, y, c.gravity()));
        self.air_vel = v;
        // `0x1402AF960` → movement state 2: flight (state 4) ends.
        self.flying = false;
    }

    /// `sys 135` (`0x1402BACB0`): movement state 4 (flight, `0x1402B1960(actor, 4)`) with the
    /// flight velocity `+0x3C0` = `v` (the script's vector × its fourth word, `0x140386D00`).
    pub fn fly(&mut self, v: Vec3) {
        self.flying = true;
        self.arc = None;
        self.held = false;
        self.air_vel = v;
    }

    /// `0x1402C7300(dir, p)`: a push slide totalling `p` (speed `p × −ln 0.8`, ×0.8 a frame).
    pub fn push(&mut self, dir: Vec3, p: f32) {
        self.air_vel = Vec3::ZERO;
        self.push = Some((dir.flat(), p * 0.223_143_55));
    }

    pub fn airborne(&self) -> bool {
        self.arc.is_some()
    }

    /// One frame: the displacement (KH1 units) for the host to move and collide. `steer`: the
    /// script's air velocity this frame (`+0x20` with speed `+0x2C`, a leap's steering), or none.
    pub fn step<C: Engine + ?Sized>(&mut self, step: f32, y: f32, steer: Option<Vec3>, c: &C) -> Vec3 {
        // The engine's turn (end of `0x1402B7260`): at most turn_rate × step toward +0x150.
        self.yaw = turn_toward(self.yaw, self.yaw_target, self.turn.unwrap_or_else(|| c.turn_rate()), step);
        let want = self.walk;
        let mut d = want * step;
        self.walk = Vec3::ZERO;
        if self.flying && self.arc.is_none() {
            // `0x1402CC000`: the flight velocity follows the script's (`+0x20` × `+0x2C`, which the
            // actor task zeroes each frame, `0x140297FE0`, before the script sets it again).
            let a = c.air_accel();
            let mut v = want * a + self.air_vel * (1.0 - a);
            v.y = want.y;
            self.air_vel = v;
            d = v * step;
        } else if self.held {
            // `0x1402AF6A0` with flag 0x1000: t (`+0x3B0`) and the rise time (`+0x3B8`) back to 0,
            // the base (`+0x3B4`) at the body's height (the root motion's height is already in
            // it here: the host moved the body), the rise `+0x3BC` kept, no displacement of its own.
            if let Some(arc) = &mut self.arc {
                arc.t = 0.0;
                arc.rise_time = 0.0;
                arc.y0 = y;
            }
        } else if let Some(arc) = &mut self.arc {
            // `0x1402AF6A0`: the horizontal velocity, then the arc's height.
            match steer {
                Some(s) if s.horizontal_length() > 0.0 => {
                    let cur = self.air_vel.horizontal_length();
                    let (dir, speed) = (s.flat().normalized(), s.horizontal_length());
                    let mut v = (dir * (c.air_accel() * speed) + self.air_vel).flat();
                    let (u, len) = v.normalize_len();
                    let cap = if cur <= c.run_speed() { c.run_speed() } else { cur };
                    v = u * len.min(cap);
                    self.air_vel = v;
                }
                _ => {
                    let k = if step < 2.0 { c.air_drag().sqrt() } else { c.air_drag() };
                    self.air_vel = self.air_vel * k;
                }
            }
            d += self.air_vel * step;
            d.y = arc.advance(y, step, c.gravity(), c.fall_cap());
        }
        if let Some((dir, v)) = &mut self.push {
            // `0x1402C7070`: dir × step × v, then v ×= 0.8^step; ends below 1.
            d += *dir * (step * *v);
            *v *= 0.8f32.powf(step);
            if *v < 1.0 {
                self.push = None;
            }
        }
        d
    }

    /// The host found the floor while falling: the arc ends.
    pub fn land(&mut self) {
        self.arc = None;
        self.air_vel = Vec3::ZERO;
    }

    pub fn falling(&self) -> bool {
        !self.held && self.arc.is_some_and(|a| !a.rising())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{Numbers, Table};

    fn consts() -> Table {
        let mut v: Vec<(String, f32)> = crate::enemy::EXE_NUMBERS.iter().map(|(k, x)| (k.to_string(), *x)).collect();
        for (k, x) in [("walk_speed", 8.0), ("run_speed", 8.0), ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("weight", 1.0), ("attack_blend", 8.0)] {
            v.push((k.to_string(), x));
        }
        Table::new(v, &[]).unwrap()
    }

    /// KH1's flight (`0x1402CC000`): the horizontal velocity eases toward the script's at the stats
    /// block's 0.1 a frame, the vertical one is the script's at once; no gravity.
    #[test]
    fn flight_eases_horizontally_and_follows_vertically() {
        let c = consts();
        let mut b = Body::default();
        b.fly(Vec3::ZERO);
        assert!(b.flying && !b.airborne());
        b.walk = Vec3::new(0.0, -2.0, 10.0);
        let d = b.step(1.0, 0.0, None, &c);
        assert!((d.z - 1.0).abs() < 1e-5 && (d.y + 2.0).abs() < 1e-6, "{d:?}");
        // Next frame the script sets nothing (KH1 zeroes +0x2C each frame): it eases off, no fall.
        let d = b.step(1.0, 0.0, None, &c);
        assert!((d.z - 0.9).abs() < 1e-5 && d.y == 0.0, "{d:?}");
        // Twelve frames of the same steering: v = 10 (1 − 0.9^n).
        let mut b = Body::default();
        b.fly(Vec3::ZERO);
        let mut v = 0.0;
        for _ in 0..12 {
            b.walk = Vec3::new(0.0, 0.0, 10.0);
            v = b.step(1.0, 0.0, None, &c).z;
        }
        assert!((v - 10.0 * (1.0 - 0.9f32.powi(12))).abs() < 1e-3, "{v}");
        // A push slide zeroes the flight velocity (`0x1402C7300`), a jump leaves flight (state 2).
        b.push(Vec3::new(1.0, 0.0, 0.0), 80.0);
        assert_eq!(b.air_vel, Vec3::ZERO);
        b.jump(20.0, Vec3::ZERO, 0.0, &c);
        assert!(!b.flying && b.airborne());
        assert_eq!(c.num("air_accel"), 0.1);
    }

    /// `sys 125`'s turn rate replaces the manifest's.
    #[test]
    fn script_turn_rate() {
        let c = consts();
        let mut b = Body { yaw_target: 1.0, ..Default::default() };
        b.step(1.0, 0.0, None, &c);
        assert!((b.yaw - std::f32::consts::PI / 16.0).abs() < 1e-6);
        let mut b = Body { yaw_target: 1.0, turn: Some(7.5f32.to_radians()), ..Default::default() };
        b.step(1.0, 0.0, None, &c);
        assert!((b.yaw - 7.5f32.to_radians()).abs() < 1e-6);
    }
}
