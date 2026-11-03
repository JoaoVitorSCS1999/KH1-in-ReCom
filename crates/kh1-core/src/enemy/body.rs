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
    /// KH1's movement state 3, a jump of fixed time (`sys 82` → `0x1402AF150`, step `0x1402AFAE0`;
    /// the Powerwild's hops, M11). Its velocity is [`Body::air_vel`], `y` included.
    pub timed: Option<Timed>,
    /// The last landing ended a [`Timed`] jump: KH1's state 3 lands straight to state 0
    /// (`0x1402AFAD0`), with no landing motion.
    pub landed_timed: bool,
    /// The flight's easing a script set (the stats block's `+0x14`, `0x1402CC000`'s `a`), else the
    /// manifest's (`Engine::air_accel`). M11 (the Darkball writes it: 0.05 / 0.1 / 0.2 / 1).
    pub accel: Option<f32>,
    /// A warp (`sys 86`, [`super::script::warp`]): this frame's displacement to the new place; the
    /// flight velocity and the walk are zeroed with it (`0x140292BC0`). M11.
    pub warp: Option<Vec3>,
    /// KH1's movement state 7 (`sys 169` → `0x1402BB030`: `0x1402B1960(actor, 7)`, the velocity
    /// `+0x3C0` = the script's vector × its fourth word; registered by `0x1402B1B80(7, flags 1, …,
    /// 0x1402CC000, …)`, Checked (decomp)): [`Body::flying`]'s step (the flight velocity follows
    /// the script's `+0x20` × `+0x2C`) with the air state's flags (`& 3 == 1`): airborne for the
    /// reactions (`0x1402B1B10`), never grounded. The Bandit's leap (M11). An arc, a timed jump,
    /// flight or `sys 88` ([`Body::ground`]) ends it.
    pub air_flight: bool,
    /// KH1's movement state 5 (`sys 82` from flight: `0x1402AF150` with `+0x70` = 4 enters state 5 and keeps the height
    /// above the floor it left from, `+0x3BC = +0x3D4`; Checked (decomp)): a [`Timed`] jump of a flier, `Some(that
    /// height)`. Its step is state 3's (`0x1402AFAE0`); its flags are flight's (2, `0x1402AF2E0`: not airborne for the
    /// reactions, `0x1402B1B10`), so [`Body::flying`] stays set; its end (`0x1402AFAA0`) goes back to flight (state 4),
    /// the velocity kept, once it falls (`+0x3C4 ≥ 0`) to its starting height or touches the floor
    /// ([`Body::flight_hop_done`]). M11, the Blue Rhapsody's hops.
    pub flight_hop: Option<f32>,
}

/// Movement state 3's numbers (`actor+0x3B0` time, `+0x3B4` gravity, `+0x3B8` half the time,
/// `+0x3CC` the horizontal drag).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Timed {
    pub t: f32,
    pub g: f32,
    pub half: f32,
    pub drag: f32,
}

impl Body {
    /// `0x1402AF960(h)` + the horizontal: a jump of `h` with air velocity `v`.
    pub fn jump<C: Engine + ?Sized>(&mut self, h: f32, v: Vec3, y: f32, c: &C) {
        self.arc = Some(Arc::hop(h, y, c.gravity()));
        self.air_vel = v;
        // `0x1402AF960` → movement state 2: flight (states 4 / 7) and state 3 / 5 end.
        self.flying = false;
        self.air_flight = false;
        self.timed = None;
        self.flight_hop = None;
    }

    /// `sys 82` → `0x1402AF150(actor, v, drag, time)`: movement state 3. `v`: the horizontal
    /// velocity in `x` / `z` and the height in `y`. Half the time is the rise; gravity
    /// `2h / half²`, the first vertical speed `−g × half` (up, KH1 y down). All zero: time 30, drag
    /// 0.9, height 500 (the function's own default). Checked (decomp).
    pub fn timed_jump(&mut self, v: Vec3, drag: f32, time: f32) {
        let (mut v, mut drag, mut time) = (v, drag, time);
        if v == Vec3::ZERO && drag == 0.0 && time == 0.0 {
            time = 30.0;
            drag = 0.9;
            v.y = 500.0;
        }
        let half = time * 0.5;
        let g = (v.y + v.y) / (half * half);
        self.air_vel = Vec3::new(v.x, -(g * half), v.z);
        self.timed = Some(Timed { t: 0.0, g, half, drag });
        self.arc = None;
        self.flying = false;
        self.air_flight = false;
        self.flight_hop = None;
    }

    /// `sys 82` (`0x1402AF150`) whatever the movement state: from flight (state 4) KH1 enters state 5 with `height`, the
    /// height above the floor it leaves from (`+0x3D4`, [`Body::flight_hop`]); otherwise state 3 ([`Body::timed_jump`]).
    pub fn timed_hop(&mut self, v: Vec3, drag: f32, time: f32, height: f32) {
        let from_flight = self.flying && self.arc.is_none() && self.timed.is_none() && !self.air_flight;
        self.timed_jump(v, drag, time);
        if from_flight {
            self.flying = true;
            self.flight_hop = Some(height);
        }
    }

    /// State 5's end (`0x1402AFAA0`, Checked (exe bytes)): while rising (`+0x3C4 < 0`, y down) it stays; falling, it goes
    /// back to flight (state 4) when its height above the floor is down to the one it left from (`+0x3BC ≥ +0x3D4`) or it
    /// touches the floor (the check's second argument 0). `height`: the height above the floor now (none: no floor known,
    /// only the touch counts). True when it ended now (flight goes on with the velocity it has).
    pub fn flight_hop_done(&mut self, height: Option<f32>, touch: bool) -> bool {
        let Some(h0) = self.flight_hop else { return false };
        if self.timed.is_none() || self.air_vel.y < 0.0 {
            return false;
        }
        if height.is_some_and(|h| h0 >= h) || touch {
            self.timed = None;
            self.flight_hop = None;
            self.flying = true;
            return true;
        }
        false
    }

    /// `0x1402AF630`: six frames before the top of a [`Timed`] jump (`+0x3B0 + 6 ≥ +0x3B8`), KH1
    /// plays the request base + 5 and queues base + 6 when base + 4 is playing (the base is
    /// `actor+0x3D0`, `sys 83`'s request − 3).
    pub fn timed_apex(&self) -> bool {
        self.timed.is_some_and(|j| j.half <= j.t + 6.0)
    }

    /// `sys 135` (`0x1402BACB0`): movement state 4 (flight, `0x1402B1960(actor, 4)`) with the
    /// flight velocity `+0x3C0` = `v` (the script's vector × its fourth word, `0x140386D00`).
    pub fn fly(&mut self, v: Vec3) {
        self.flying = true;
        self.air_flight = false;
        self.arc = None;
        self.timed = None;
        self.flight_hop = None;
        self.held = false;
        self.air_vel = v;
    }

    /// `sys 86`: moved by `d` this frame, facing `yaw`, no velocity.
    pub fn warp(&mut self, d: Vec3, yaw: f32) {
        self.warp = Some(self.warp.unwrap_or(Vec3::ZERO) + d);
        self.air_vel = Vec3::ZERO;
        self.walk = Vec3::ZERO;
        self.yaw = yaw;
        self.yaw_target = yaw;
    }

    /// `sys 169` (`0x1402BB030`): movement state 7 ([`Body::air_flight`]) with the flight velocity
    /// `v` (the script's vector × its fourth word, `0x140386D00`).
    pub fn air_fly(&mut self, v: Vec3) {
        self.air_flight = true;
        self.flying = false;
        self.arc = None;
        self.timed = None;
        self.flight_hop = None;
        self.held = false;
        self.air_vel = v;
    }

    /// `sys 88(actor, 0)` (`0x1402BA5B0` → `0x1402B1960(actor, 0)`): movement state 0; flight
    /// (states 4 / 7) ends. Checked (decomp).
    pub fn ground(&mut self) {
        self.flying = false;
        self.air_flight = false;
        if self.flight_hop.take().is_some() {
            self.timed = None;
        }
    }

    /// `0x1402C7300(dir, p)`: a push slide totalling `p` (speed `p × −ln 0.8`, ×0.8 a frame).
    pub fn push(&mut self, dir: Vec3, p: f32) {
        self.air_vel = Vec3::ZERO;
        self.push = Some((dir.flat(), p * 0.223_143_55));
    }

    /// In the air: movement state 2 (an arc) or 3 (a [`Timed`] jump; `0x1402B1B10`: the state
    /// table's flags `& 3 == 1`, state 3's are 5).
    pub fn airborne(&self) -> bool {
        // State 5 (a flier's timed hop, [`Body::flight_hop`]) has flight's flags: not airborne.
        self.arc.is_some() || (self.timed.is_some() && self.flight_hop.is_none()) || self.air_flight
    }

    /// One frame: the displacement (KH1 units) for the host to move and collide. `steer`: the
    /// script's air velocity this frame (`+0x20` with speed `+0x2C`, a leap's steering), or none.
    pub fn step<C: Engine + ?Sized>(&mut self, step: f32, y: f32, steer: Option<Vec3>, c: &C) -> Vec3 {
        // The engine's turn (end of `0x1402B7260`): at most turn_rate × step toward +0x150.
        self.yaw = turn_toward(self.yaw, self.yaw_target, self.turn.unwrap_or_else(|| c.turn_rate()), step);
        if let Some(w) = self.warp.take() {
            // `sys 86` zeroed the velocity and the speed: only the warp moves it this frame.
            self.walk = Vec3::ZERO;
            self.air_vel = Vec3::ZERO;
            return w;
        }
        let want = self.walk;
        let mut d = want * step;
        self.walk = Vec3::ZERO;
        // A flier's timed hop (state 5) steps as state 3 (`0x1402AFAE0`): the timed branch, not flight's.
        if (self.flying || self.air_flight) && self.arc.is_none() && self.timed.is_none() {
            // `0x1402CC000`: the flight velocity follows the script's (`+0x20` × `+0x2C`, which the
            // actor task zeroes each frame, `0x140297FE0`, before the script sets it again).
            let a = self.accel.unwrap_or_else(|| c.air_accel());
            let mut v = want * a + self.air_vel * (1.0 - a);
            v.y = want.y;
            self.air_vel = v;
            d = v * step;
        } else if let Some(j) = &mut self.timed {
            // `0x1402AFAE0`: the displacement is the velocity × step (the ground walk is not added:
            // state 3's move function only takes the facing); then x / z × drag (√drag below step 2),
            // the vertical speed + step × g (at most 20), the time + step.
            d = self.air_vel * step;
            let k = if step < 2.0 { j.drag.abs().sqrt() } else { j.drag };
            self.air_vel.x *= k;
            self.air_vel.z *= k;
            self.air_vel.y = (self.air_vel.y + step * j.g).min(20.0);
            j.t += step;
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
        self.landed_timed = self.timed.is_some();
        self.air_flight = false;
        self.arc = None;
        self.timed = None;
        self.flight_hop = None;
        self.air_vel = Vec3::ZERO;
    }

    /// Falling toward a landing (state 2 or 3): a flier's timed hop (state 5) never lands, it ends back in flight
    /// ([`Body::flight_hop_done`]).
    pub fn falling(&self) -> bool {
        !self.held && (self.arc.is_some_and(|a| !a.rising()) || (self.timed.is_some() && self.flight_hop.is_none() && self.air_vel.y > 0.0))
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

    /// Movement state 3 (`0x1402AF150` / `0x1402AFAE0`), the Powerwild's back hop: 20 a frame
    /// backward, height 50, 30 frames. It rises for 15 frames to about 50 (y down), the horizontal speed
    /// falls by √0.9 a frame over its 31 frames of travel, the apex motion comes 6 frames before the top, and it lands as state
    /// 3 (no landing motion).
    #[test]
    fn a_timed_jump_rises_for_half_its_time() {
        let c = consts();
        let mut b = Body::default();
        b.timed_jump(Vec3::new(0.0, 50.0, -20.0), 0.9, 30.0);
        assert!(b.airborne() && !b.falling());
        let (mut y, mut z, mut top, mut apex) = (0.0f32, 0.0f32, 0.0f32, None);
        for f in 1..=40 {
            let d = b.step(1.0, y, None, &c);
            y += d.y;
            z += d.z;
            top = top.min(y);
            if apex.is_none() && b.timed_apex() {
                apex = Some(f);
            }
            if y >= 0.0 && b.falling() {
                b.land();
                assert!(b.landed_timed && !b.airborne());
                // Displacement before the speed update (KH1's order): n·vy₀ + g·n(n−1)/2 = 0 at n = 31.
                assert_eq!(f, 31, "lands on frame 31");
                break;
            }
        }
        assert!((top + 50.0).abs() < 4.0, "top {top}");
        assert_eq!(apex, Some(9), "6 frames before the 15-frame rise");
        let k = 0.9f32.sqrt();
        let want = -20.0 * (1.0 - k.powi(31)) / (1.0 - k);
        assert!((z - want).abs() < 1e-2, "{z} vs {want}");
        // An arc or flight ends it.
        b.timed_jump(Vec3::ZERO, 0.0, 0.0);
        assert_eq!(b.timed.map(|j| (j.half, j.drag)), Some((15.0, 0.9)));
        b.jump(20.0, Vec3::ZERO, 0.0, &c);
        assert!(b.timed.is_none());
    }

    /// Movement state 5 (a flier's `sys 82`, the Blue Rhapsody's hop: 30 a frame ahead, 60 high, drag 0.9, 12 frames from
    /// 35 above the floor): it rises 6 frames, falls back and goes back to flight when down to its starting height (the
    /// 13th / 14th frame, with the velocity it has); it is never airborne for the reactions nor falling toward a landing.
    #[test]
    fn a_flight_hop_ends_back_in_flight_at_its_height() {
        let c = consts();
        let mut b = Body::default();
        b.fly(Vec3::ZERO);
        b.timed_hop(Vec3::new(0.0, 60.0, 30.0), 0.9, 12.0, 35.0);
        assert!(b.flying && b.timed.is_some() && b.flight_hop == Some(35.0));
        assert!(!b.airborne() && !b.falling());
        let (mut y, mut top) = (-35.0f32, 0.0f32);
        let mut ended = None;
        for f in 1..=30 {
            b.walk = Vec3::new(0.0, 0.0, 0.0);
            let d = b.step(1.0, y, None, &c);
            y += d.y;
            top = top.min(y);
            if b.flight_hop_done(Some(0.0 - y), y >= 0.0) {
                ended = Some(f);
                break;
            }
        }
        // n·vy₀ + g·n(n−1)/2 = 0 exactly at n = 13: a tie the float sums break either way (here a hair above: 14).
        assert!(matches!(ended, Some(13) | Some(14)), "{ended:?}");
        // KH1's discrete rise (displacement before the speed update): 6·(−20) + g·15 = −70 above where it left.
        assert!((top + 105.0).abs() < 0.5, "70 above where it left (35 over the floor): {top}");
        assert!(b.flying && b.timed.is_none() && b.flight_hop.is_none());
        assert!(b.air_vel.z > 0.0, "the velocity kept");
        // From the ground (not flying) it is state 3 as before.
        let mut g = Body::default();
        g.timed_hop(Vec3::new(0.0, 60.0, 30.0), 0.9, 12.0, 0.0);
        assert!(!g.flying && g.flight_hop.is_none() && g.airborne());
        // Touching the floor while falling ends it too.
        let mut b = Body::default();
        b.fly(Vec3::ZERO);
        b.timed_hop(Vec3::new(0.0, 60.0, 0.0), 0.9, 12.0, 35.0);
        b.air_vel.y = 1.0;
        assert!(b.flight_hop_done(Some(100.0), true));
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

    /// A script's easing (the stats block's `+0x14`) replaces the manifest's; a warp moves the body
    /// by its displacement alone and stops the flight.
    #[test]
    fn script_easing_and_warp() {
        let c = consts();
        let mut b = Body { accel: Some(0.2), ..Default::default() };
        b.fly(Vec3::ZERO);
        b.walk = Vec3::new(0.0, 0.0, 10.0);
        assert!((b.step(1.0, 0.0, None, &c).z - 2.0).abs() < 1e-5);
        b.walk = Vec3::new(0.0, 0.0, 10.0);
        b.warp(Vec3::new(0.0, -80.0, 0.0), 1.0);
        let d = b.step(1.0, 0.0, None, &c);
        assert_eq!(d, Vec3::new(0.0, -80.0, 0.0));
        assert_eq!((b.air_vel, b.yaw), (Vec3::ZERO, 1.0));
        assert_eq!(b.step(1.0, 0.0, None, &c), Vec3::ZERO);
    }

    /// Movement state 7 (`sys 169`, the Bandit's leap): the flight step, airborne for the
    /// reactions, never falling; a jump (a reaction's hop) or `sys 88` ends it.
    #[test]
    fn air_flight_steps_as_flight_and_is_airborne() {
        let c = consts();
        let mut b = Body::default();
        b.air_fly(Vec3::ZERO);
        assert!(b.air_flight && b.airborne() && !b.falling() && !b.flying);
        b.walk = Vec3::new(0.0, -7.0, 70.0);
        let d = b.step(1.0, 0.0, None, &c);
        assert!((d.z - 7.0).abs() < 1e-4 && (d.y + 7.0).abs() < 1e-5, "{d:?}");
        let d = b.step(1.0, 0.0, None, &c);
        assert!((d.z - 6.3).abs() < 1e-4 && d.y == 0.0, "eases off, no gravity: {d:?}");
        b.ground();
        assert!(!b.airborne());
        b.air_fly(Vec3::ZERO);
        b.jump(20.0, Vec3::ZERO, 0.0, &c);
        assert!(!b.air_flight && b.airborne() && b.arc.is_some());
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
