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
}

impl Body {
    /// `0x1402AF960(h)` + the horizontal: a jump of `h` with air velocity `v`.
    pub fn jump<C: Engine + ?Sized>(&mut self, h: f32, v: Vec3, y: f32, c: &C) {
        self.arc = Some(Arc::hop(h, y, c.gravity()));
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
        self.yaw = turn_toward(self.yaw, self.yaw_target, c.turn_rate(), step);
        let mut d = self.walk * step;
        self.walk = Vec3::ZERO;
        if self.held {
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
