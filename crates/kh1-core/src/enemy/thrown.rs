//! A KH1 enemy's thrown object (`sys 94`'s child script, M11): the Bandit's scimitar
//! (`ex_2090_00.bd`, part 3 of `xa_ex_2090.mdls`; Checked (files): the child script disassembled with
//! `tools/kh1-ai-script.py`, its syscalls read in the decomp). The host draws, moves, collides and
//! ends it as one of Re:CoM's own bullets; this is KH1's flight it is moved by each frame.
//!
//! The child (`0x18`, set up by `0x348`): it starts at the thrower's bone code 28 (`sys 19`) flying
//! along the thrower's facing (`*(actor + 0x30)`) at 10 a frame; each frame, while its time is under
//! its life (300): past 120 frames its speed changes by −0.5 × step (so it stops at 140 and comes
//! back, faster and faster); it turns toward its target's lock point 0 (`sys 119`) by at most 3° ×
//! step (`sys 24` → `0x1402A85C0`: about the axis `dir × to_target`, the angle clamped to
//! `rad(3) × step`); it moves by `dir × speed × step` (`sys 18`); its attack object follows it. A hit
//! (its object's `+4 & 0x45`) ends it; a repel (`& 0x102` / `& 0x30`) turns it back with a random tilt
//! (`dir = −dir`, `y −= rand`, `x += (rand − 0.5) × 2`, `z += (rand − 0.5) × 2`, normalised), fades it
//! over 60 frames (`sys 26`) and slides it on, its speed × 0.9 a frame, for 60 frames, then it ends.
//! The life's end ends it too (`sys 97`).

use crate::math::Vec3;

/// The child's numbers (its init `0x348`: `hp[0x54]` life, `hp[0x58]` turn-back time, `hp[0x44]` speed,
/// `hp[0x48]` the change past the turn-back time, `hp[0x40]` the turn in degrees, `hp[0x5C]` the
/// repelled slide's length; `0x2A2` the slide's 0.9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Numbers {
    pub life: f32,
    pub turn_back: f32,
    pub speed: f32,
    pub accel: f32,
    pub turn_deg: f32,
    pub slide: f32,
    pub slide_drag: f32,
}

/// Where the flight is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Flying,
    /// Repelled: the slide's frames played.
    Sliding(f32),
    Ended,
}

/// One thrown object's flight (KH1 units, y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thrown {
    pub pos: Vec3,
    pub dir: Vec3,
    pub speed: f32,
    pub time: f32,
    pub phase: Phase,
    pub n: Numbers,
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x)
}

/// `0x1402A85C0(v, to, limit)`: `v` turned toward `to` by the angle between them, clamped to
/// `limit` (radians), about `normalize(v × to)` (Rodrigues; Assumption: KH1's rotation
/// `0x1402A7D40` turns `v` toward `to`, the direction the script uses it for). The length is kept.
pub fn turn_toward(v: Vec3, to: Vec3, limit: f32) -> Vec3 {
    let (lv, lt) = (v.length(), to.length());
    if lv == 0.0 || lt == 0.0 {
        return v;
    }
    let a = (v.dot(to) / (lv * lt)).clamp(-1.0, 1.0).acos();
    let t = a.clamp(-limit, limit);
    let k = cross(v, to).normalized();
    if k == Vec3::ZERO || t == 0.0 {
        return v;
    }
    let (s, c) = t.sin_cos();
    v * c + cross(k, v) * s + k * (k.dot(v) * (1.0 - c))
}

impl Thrown {
    /// Thrown from `at` along `facing` (the thrower's facing yaw: KH1's actor `+0x30` is the
    /// horizontal facing vector).
    pub fn new(at: Vec3, facing: f32, n: Numbers) -> Thrown {
        Thrown { pos: at, dir: Vec3::from_yaw(facing), speed: n.speed, time: 0.0, phase: Phase::Flying, n }
    }

    /// Thrown from `at` along the unit direction `dir` (M11, the Fat Bandit's fireball: `sys 143`'s head direction,
    /// `kh1_core::fatbandit::fire_dir`, its height included).
    pub fn aimed(at: Vec3, dir: Vec3, n: Numbers) -> Thrown {
        Thrown { pos: at, dir: dir.normalized(), speed: n.speed, time: 0.0, phase: Phase::Flying, n }
    }

    /// Sent off again along `dir` from where it is, its clock back to 0 (the Fat Bandit's fireball after a repel:
    /// `ex_2060_00.bd` `0x40` → `0x1B8` aims it anew, `hp[0x44]` = 0, and `0x72` flies it its whole life again;
    /// Checked (files)).
    pub fn retarget(&mut self, dir: Vec3) {
        self.dir = dir.normalized();
        self.speed = self.n.speed;
        self.time = 0.0;
        self.phase = Phase::Flying;
    }

    /// One frame (`step` KH1 frames): the displacement. `target`: the target's lock point while it
    /// lives (`sys 120`), else none (no turn). Ends at the life's end or after the repelled slide.
    pub fn step(&mut self, target: Option<Vec3>, step: f32) -> Vec3 {
        match self.phase {
            Phase::Ended => Vec3::ZERO,
            Phase::Flying => {
                if self.time >= self.n.life {
                    self.phase = Phase::Ended;
                    return Vec3::ZERO;
                }
                if self.n.turn_back < self.time {
                    self.speed += self.n.accel * step;
                }
                if let Some(t) = target {
                    self.dir = turn_toward(self.dir, t - self.pos, self.n.turn_deg.to_radians() * step);
                }
                let d = self.dir * (self.speed * step);
                self.pos += d;
                self.time += step;
                d
            }
            Phase::Sliding(t) => {
                if t >= self.n.slide {
                    self.phase = Phase::Ended;
                    return Vec3::ZERO;
                }
                self.speed *= self.n.slide_drag;
                let d = self.dir * (self.speed * step);
                self.pos += d;
                self.phase = Phase::Sliding(t + step);
                d
            }
        }
    }

    /// The repel (`0x1B8..0x222`): `r` are the script's three draws (`sys 25`, in its order).
    pub fn repel(&mut self, r: [f32; 3]) {
        let mut d = -self.dir;
        d.y -= r[0];
        d.x += (r[1] - 0.5) * 2.0;
        d.z += (r[2] - 0.5) * 2.0;
        self.dir = d.normalized();
        self.time = 0.0;
        self.phase = Phase::Sliding(0.0);
    }

    pub fn ended(&self) -> bool {
        self.phase == Phase::Ended
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n() -> Numbers {
        // The player's `ex_2090_00.bd` (`0x3D4..0x406`, `0x2A2`).
        Numbers { life: 300.0, turn_back: 120.0, speed: 10.0, accel: -0.5, turn_deg: 3.0, slide: 60.0, slide_drag: 0.9 }
    }

    /// Straight out at 10 for 120 frames (+ 20 slowing to 0 at 140), then back past its start.
    #[test]
    fn it_flies_out_and_comes_back() {
        let mut t = Thrown::new(Vec3::ZERO, 0.0, n());
        let mut far = 0.0f32;
        let mut frames = 0;
        while !t.ended() && frames < 1000 {
            t.step(None, 1.0);
            far = far.max(t.pos.z);
            frames += 1;
        }
        assert_eq!(frames, 301, "300 frames of flight, ended on the next");
        // Times 0..=120 at 10 (KH1 slows only past 120), then 9.5 + 9 + … + 0 (times 121..140).
        assert!((far - (1210.0 + 95.0)).abs() < 1.0, "{far}");
        assert!(t.pos.z < -1000.0, "it comes back past where it started: {}", t.pos.z);
    }

    /// The turn is at most 3° a frame (× step) toward the target.
    #[test]
    fn it_turns_toward_its_target_three_degrees_a_frame() {
        let mut t = Thrown::new(Vec3::ZERO, 0.0, n());
        t.step(Some(Vec3::new(1000.0, 0.0, 0.0)), 1.0);
        assert!((t.dir.x.atan2(t.dir.z).to_degrees() - 3.0).abs() < 1e-3, "{:?}", t.dir);
        assert!((t.dir.length() - 1.0).abs() < 1e-5);
        let mut u = Thrown::new(Vec3::ZERO, 0.0, n());
        u.step(Some(Vec3::new(1000.0, 0.0, 0.0)), 2.0);
        assert!((u.dir.x.atan2(u.dir.z).to_degrees() - 6.0).abs() < 1e-3);
        // A target 1° off: it turns the 1°.
        let mut v = Thrown::new(Vec3::ZERO, 0.0, n());
        v.step(Some(Vec3::new(1f32.to_radians().sin() * 100.0, 0.0, 1f32.to_radians().cos() * 100.0)), 1.0);
        assert!((v.dir.x.atan2(v.dir.z).to_degrees() - 1.0).abs() < 1e-2);
    }

    /// The Fat Bandit's fireball (`ex_2060_00.bd`: 45 a frame, no turn, 240 frames): straight along its aim, height
    /// included, whatever its target does; a retarget flies it its whole life again.
    #[test]
    fn an_aimed_shot_flies_straight() {
        let n = Numbers { life: 240.0, turn_back: 240.0, speed: 45.0, accel: 0.0, turn_deg: 0.0, slide: 0.0, slide_drag: 0.0 };
        let d = Vec3::new(0.0, 30.0, 400.0).normalized();
        let mut t = Thrown::aimed(Vec3::ZERO, d, n);
        let mut k = 0;
        while !t.ended() {
            t.step(Some(Vec3::new(500.0, 0.0, 0.0)), 1.0);
            k += 1;
        }
        assert_eq!(k, 241);
        assert!((t.pos - d * (45.0 * 240.0)).length() < 0.5, "{:?}", t.pos);
        let mut t = Thrown::aimed(Vec3::ZERO, d, n);
        for _ in 0..100 {
            t.step(None, 1.0);
        }
        t.retarget(-d);
        assert_eq!((t.time, t.phase), (0.0, Phase::Flying));
        for _ in 0..100 {
            t.step(None, 1.0);
        }
        assert!(t.pos.length() < 1e-2, "back where it started: {:?}", t.pos);
    }

    #[test]
    fn a_repel_slides_sixty_frames_and_ends() {
        let mut t = Thrown::new(Vec3::ZERO, 0.0, n());
        t.step(None, 1.0);
        t.repel([0.0, 0.5, 0.5]);
        assert_eq!(t.dir, Vec3::new(0.0, 0.0, -1.0));
        let mut k = 0;
        while !t.ended() {
            t.step(None, 1.0);
            k += 1;
        }
        assert_eq!(k, 61);
    }
}
