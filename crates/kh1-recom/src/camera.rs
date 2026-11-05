//! M12 camera: KH1's (and later KH2's) player camera as new modes of Re:CoM's own player camera
//! (`PlayerCamera::CPlayerCamera`). The testable part; `win::camera` plugs it in.
//! `docs/recom-m12-camera-plan.md`.
//!
//! Owner, 2026-11-04: "Lets also have an option in the config to switch between Re:CoM camera, KH1
//! Camera, And KH2 camera"; answers (`design-philosophy-instructions.md` § Camera answers): a
//! Settings > Camera page independent of Combat; sleights, summons and specials keep Re:CoM's
//! camera moves, then hand back through Re:CoM's restore; boss fights keep Re:CoM's boss cameras
//! (`BCam0000`); after an event the KH1 camera re-places behind Sora; Re:CoM's inversion rows drive
//! it with KH1's own "Normal"; KH1's own speeds; KH1's built-in default numbers; Riku's story keeps
//! Re:CoM's camera; KH1's mouse rule on keyboard and mouse.
//!
//! **Native shape** (plan § 7): the camera's mode field `+0x178` takes new numbers ([`mode`]); each
//! has a record in Re:CoM's own record format ([`Record`]) that Re:CoM's own mode setters
//! `0x1401FC4D0` / `0x1401FBE60` read through their table read (a stub gives the record for modes
//! ≥ 6: Re:CoM's table `0x14087B660` has 6 slots); the camera's field and battle execs
//! (`0x1401FEEB0` / `0x1401FECE0`, reached only through the init's two `lea`s) are pointed at a
//! dispatcher that runs our step for our modes, then Re:CoM's own tail (its collision object, the
//! eye node, its view write); every other mode goes to Re:CoM's exec untouched. Sleights keep
//! asking for Re:CoM's modes 3 / 4 / 5 through `0x1401FBDE0`, which saves our mode in `+0x17C`
//! and gives it back on −1.

use crate::units;
use kh1_core::math::wrap_angle;
use kh1_core::Vec3;
use std::f32::consts::PI;

/// Re:CoM's mode numbers we add (`CPlayerCamera+0x178`). Re:CoM's own: 1 field follow, 2 timed
/// free view, 3 battle follow, 4 battle lock-on, 5 close-up (sleights). Checked (decomp `0x1401FB530`,
/// `0x1401FECE0`, `0x1401FBDE0`).
pub mod mode {
    pub const KH1_FOLLOW: i32 = 6;
    pub const KH1_LOCK: i32 = 7;
    pub const KH2_FOLLOW: i32 = 8;
    pub const KH2_LOCK: i32 = 9;
    /// The first of ours and how many (the stub's range check: 6 ≤ mode < 10).
    pub const FIRST: i32 = 6;
    pub const COUNT: usize = 4;
    /// Re:CoM's base modes: field follow, battle follow.
    pub const RECOM_FIELD: i32 = 1;
    pub const RECOM_BATTLE: i32 = 3;
    pub const RECOM_LOCK: i32 = 4;

    pub fn ours(m: i32) -> bool {
        (FIRST..FIRST + COUNT as i32).contains(&m)
    }
}

/// Which camera a mode belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Recom,
    Kh1,
}

impl Family {
    pub fn of(m: i32) -> Family {
        match m {
            mode::KH1_FOLLOW | mode::KH1_LOCK => Family::Kh1,
            _ => Family::Recom,
        }
    }

    /// The family's base mode (its follow); Re:CoM's by room kind.
    pub fn base(self, battle: bool) -> i32 {
        match self {
            Family::Kh1 => mode::KH1_FOLLOW,
            Family::Recom if battle => mode::RECOM_BATTLE,
            Family::Recom => mode::RECOM_FIELD,
        }
    }

    pub fn from_setting(c: crate::settings::Camera) -> Family {
        match c {
            crate::settings::Camera::Recom => Family::Recom,
            crate::settings::Camera::Kh1 => Family::Kh1,
        }
    }
}

/// Which family the camera should run. Re:CoM's at Camera = Re:CoM, in Riku's story (owner's
/// answer Q11 (a)) and in a battle whose boss carries Re:CoM's boss camera (`BCam0000`: the boss
/// enemy init `0x1401FEC50` sets the count `0x14087B468` only when the pack has it; Q10 (b)).
pub fn wanted(setting: Family, riku: bool, boss_camera: bool) -> Family {
    if riku || boss_camera {
        Family::Recom
    } else {
        setting
    }
}

/// What the dispatcher does with a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dispatch {
    /// Re:CoM's exec as it is.
    Recom,
    /// Our step (the mode is ours).
    Ours,
    /// Switch to this mode with Re:CoM's mode setter keeping the camera where it is
    /// (`0x1401FBE60`), then dispatch again.
    Switch(i32),
}

/// The dispatcher's rule. `external`: a sleight's request holds the mode (`+0x188 & 0x2000`);
/// `subject`: a subject override is set (`+0x2B4 > 0`). Re:CoM's modes are only replaced from its
/// base follows (1 / 3 / 4), never during a sleight's request or its timed view 2.
pub fn dispatch(m: i32, want: Family, battle: bool, external: bool, subject: bool) -> Dispatch {
    if mode::ours(m) {
        if Family::of(m) == want {
            Dispatch::Ours
        } else {
            Dispatch::Switch(want.base(battle))
        }
    } else if want != Family::Recom && !external && !subject && matches!(m, mode::RECOM_FIELD | mode::RECOM_BATTLE | mode::RECOM_LOCK) {
        Dispatch::Switch(want.base(battle))
    } else {
        Dispatch::Recom
    }
}

/// A mode record in Re:CoM's `PCamData` format (0x30 bytes; header 0x40, five records in the
/// owner's `PL0001.BIN`): `+0` FOV in degrees (`0x14024FCB0`), `+4` collision radius (the setters'
/// `0x140172690`, the tail's × 0.1), `+8` ease rate, `+0xC` close-wall distance, `+0x10` eye offset,
/// `+0x20` aim offset (both from Re:CoM's subject point, the player's vtable `+0x68`: his root
/// node + 0.5 m, `0x1401F0510`). Checked (files + decomp).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Record {
    pub fov: f32,
    pub radius: f32,
    pub ease: f32,
    pub wall: f32,
    pub eye: [f32; 4],
    pub aim: [f32; 4],
}

impl Record {
    pub fn bytes(&self) -> [u8; 0x30] {
        let mut b = [0u8; 0x30];
        let v = [self.fov, self.radius, self.ease, self.wall, self.eye[0], self.eye[1], self.eye[2], self.eye[3], self.aim[0], self.aim[1], self.aim[2], self.aim[3]];
        for (i, f) in v.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
        }
        b
    }
}

/// Re:CoM's subject point is the player's root node + 0.5 m (`0x1401F0510`). Checked (decomp).
pub const SUBJECT_LIFT_M: f32 = 0.5;

/// The records of modes 6..9. Only Re:CoM's setters read them (FOV, radius, the rest vector and
/// distance, the setter's first view); our steps place the camera themselves.
/// - FOV: KH1's follow sets 53° (`0x14028CAF0` with `0x1403E9E44` = 53.0, Checked (exe)); KH1's and
///   Re:CoM's FOV setters use the same `256 / tan(fov / 2)` on a 512-wide frame (plan § 5), so 53
///   goes in as 53.
/// - Radius: Re:CoM's follow records' 0.05 (the tail's sweep 0.005 as Re:CoM's).
/// - Offsets: KH1's built-in look point 1.0 m above Sora's feet and his eye 4.0 m away at 10°
///   (`0x140506CA0` / `0x140506CB0`), from Re:CoM's subject point; so the rest distance `+0x2A0`
///   Re:CoM's setter derives is KH1's 4.0 m.
pub fn records() -> [Record; mode::COUNT] {
    let look = kh1::LOOK_HEIGHT_M - SUBJECT_LIFT_M;
    let (s, c) = kh1::PITCH_DEFAULT.sin_cos();
    let d = kh1::DIST_DEFAULT * units::METRES_PER_UNIT;
    let kh1 = Record { fov: kh1::FOV, radius: 0.05, ease: 1.0, wall: 0.0, eye: [0.0, look + d * s, d * c, 1.0], aim: [0.0, look, 0.0, 1.0] };
    // Modes 7..9: KH1's follow numbers until their stages give them their own.
    [kh1; mode::COUNT]
}

/// KH1 FM's follow camera, mode 1 of its camera module (`0x1401E1920`, mode table `0x140506CE0`),
/// at the PC's 60 Hz frame step 1.0 (`0x14233FBDC`; gains written for 2.0 are halved below 2.0).
/// Every number from the KH1 Steam decomp / exe (tags on each). KH1 units: centimetres, y down;
/// yaw `atan2(x, z)`. Re:CoM's world converts at the edges (`crate::units`).
pub mod kh1 {
    use super::*;

    /// Built-in defaults (`0x140506CA0`: look offset (0, −100, −50) in Sora's frame; `0x140506CB0`
    /// pitch 10° (bits 0x3E32B8C3); `0x140506CBC` distance 400). Checked (exe). Owner's answer Q8 (a):
    /// these everywhere (Re:CoM's rooms have none of KH1's room camera blocks).
    pub const OFFSET: Vec3 = Vec3::new(0.0, -100.0, -50.0);
    pub const PITCH_DEFAULT: f32 = 0.174_532_94;
    pub const DIST_DEFAULT: f32 = 400.0;
    /// The look height above Sora's feet, in metres (the offset's −100).
    pub const LOOK_HEIGHT_M: f32 = 1.0;
    /// FOV of the follow (`0x1403E9E44`). Checked (exe).
    pub const FOV: f32 = 53.0;

    /// The look point's spring (`0x1401E1920`): 20 % of the horizontal gap, vertically 10 % up /
    /// 50 % down, all halved below a frame step of 2.0. Checked (decomp).
    pub const LOOK_H: f32 = 0.2;
    pub const LOOK_UP: f32 = 0.1;
    pub const LOOK_DOWN: f32 = 0.5;

    /// The stick (`0x1401E7320`): pad maximum 2.5° (bits 0x3D32B8C3), acceleration 0.15°, decay
    /// 0.24°; mouse acceleration 0.3°, decay 4°, maximum 2.5° + 0.0875° × sensitivity; pitch
    /// clamped −15°..+50°. Checked (decomp).
    pub const PAD_MAX: f32 = 0.043_633_23;
    pub const PAD_ACCEL: f32 = 0.002_617_994;
    pub const PAD_DECAY: f32 = 0.004_188_790_5;
    pub const MOUSE_ACCEL: f32 = 0.005_235_988;
    pub const MOUSE_DECAY: f32 = 0.069_813_17;
    /// KH1's PC mouse sensitivity (0..40); its default 20 (Epic notes; owner Q7 (a): the default).
    pub const MOUSE_SENSITIVITY: f32 = 20.0;
    pub const PITCH_MIN: f32 = -0.261_799_4;
    pub const PITCH_MAX: f32 = 0.872_664_63;

    /// The recenter (`0x1401E62A0`): 60 frames, 30° wall steps; the "swap" recenter below 5° does
    /// nothing; its pitch rises up to 60° × 0.7. Checked (decomp).
    pub const RECENTER_FRAMES: f32 = 60.0;
    pub const RECENTER_STEP: f32 = 0.523_598_8;
    pub const SWAP_MIN: f32 = 0.087_266_47;
    pub const SWAP_PITCH: f32 = 1.047_197_6;

    /// The Manual branch's wall ray: 60 past the eye, the eye 60 from the wall, the look point at
    /// least 20 from the eye. Checked (decomp).
    pub const RAY_PAST: f32 = 60.0;
    pub const NEAR_LOOK: f32 = 20.0;
    /// The re-place's sphere walk (`0x1401E4820`, `0x140196560` with 40.0). Checked (decomp).
    pub const SPHERE: f32 = 40.0;

    /// KH1's PC right stick: the raw byte's dead zone 60 of 127.5 removed, radial
    /// (`0x14028B4F0`, `0x14298CB00` = 60.0); with the mouse 1.0. Checked (decomp).
    pub const DEAD_ZONE: f32 = 60.0;
    pub const MOUSE_DEAD_ZONE: f32 = 1.0;

    /// KH1's collision as Re:CoM's camera collision object answers it (`win::camera`).
    pub trait World {
        /// The first contact of a sphere of `radius` moved from `from` to `to`: where it stops, or
        /// None when the way is clear. A `radius` of [`RAY`] stands for KH1's ray and segment tests
        /// (`0x140193790`, `0x140193180`).
        fn sweep(&mut self, from: Vec3, to: Vec3, radius: f32) -> Option<Vec3>;
    }

    /// The sphere radius that stands for KH1's ray (Re:CoM's own smallest camera sweep, its tail's
    /// 0.05 × 0.1 m). Assumption: as good as a ray against Re:CoM's walls.
    pub const RAY: f32 = 0.5;

    /// No walls (tests).
    pub struct Open;
    impl World for Open {
        fn sweep(&mut self, _: Vec3, _: Vec3, _: f32) -> Option<Vec3> {
            None
        }
    }

    /// One frame's input, in KH1's terms.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Input {
        /// Sora's feet (his matrix's translation) and facing.
        pub pos: Vec3,
        pub facing: f32,
        /// The right stick (−1..1; y down positive), after KH1's dead zone.
        pub stick: [f32; 2],
        /// Keyboard and mouse in use (Re:CoM's `0x140C2FF64`): KH1's mouse rule (owner Q13 (a)).
        pub mouse: bool,
        /// Sora can act (KH1 `0x140297950` false; Re:CoM: the camera's input lock `+0x188 & 1` clear).
        pub can_act: bool,
        /// The recenter button was pressed this frame (KH1's R3; Re:CoM's reset bit 0x400).
        pub recenter: bool,
        /// KH1's inversion floats (`0x140506CDC` left / right, `0x140506CD8` up / down).
        pub inv: [f32; 2],
        /// The frame step (1.0 a 60 Hz frame).
        pub step: f32,
    }

    impl Input {
        /// KH1's look target: Sora's matrix × the offset (0.5 m behind him, 1 m up), and the point
        /// above him (the offset with x and z zeroed). Assumption: KH1's actor matrix maps local
        /// +z to his facing (`Vec3::rotate_yaw`).
        pub fn target(&self) -> Vec3 {
            self.pos + OFFSET.rotate_yaw(self.facing)
        }
        pub fn above(&self) -> Vec3 {
            self.pos + Vec3::new(0.0, OFFSET.y, 0.0)
        }
    }

    /// KH1's eye offset for a pitch, yaw and distance: `Rx(pitch) · Ry(yaw) · (0, 0, d)` with
    /// KH1's row-vector matrices (`0x1400E6AC0`, `0x1400E6C80`): the eye above for a positive
    /// pitch (y down), and `yaw = atan2(eye − look)` (`0x14028C720`). Checked (decomp).
    pub fn offset(d: f32, pitch: f32, yaw: f32) -> Vec3 {
        let (sp, cp) = pitch.sin_cos();
        let (sy, cy) = yaw.sin_cos();
        Vec3::new(d * cp * sy, -d * sp, d * cp * cy)
    }

    /// KH1's yaw from an eye and a look point (`0x14028C720`).
    pub fn yaw_of(eye: Vec3, look: Vec3) -> f32 {
        (eye.x - look.x).atan2(eye.z - look.z)
    }

    /// A running recenter (`0x142537EE0..0x142537EF8`).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Recenter {
        /// Frames left (`0x142537EEC`); 0 none.
        pub left: f32,
        pub total: f32,
        /// 0 turn negative, 1 turn positive, 2 the swap past a wall.
        pub kind: u32,
        pub from: f32,
        pub delta: f32,
        pub dist: f32,
        pub pitch: f32,
    }

    /// KH1's follow camera state (its globals `0x142537E60..`, `0x1425380EC` / `F0`).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Follow {
        pub look: Vec3,
        pub eye: Vec3,
        pub pitch: f32,
        pub yaw: f32,
        pub dist: f32,
        /// Stick velocities (pitch, yaw), per frame.
        pub vel: [f32; 2],
        pub rc: Recenter,
    }

    impl Default for Follow {
        fn default() -> Self {
            Follow { look: Vec3::ZERO, eye: Vec3::ZERO, pitch: PITCH_DEFAULT, yaw: 0.0, dist: DIST_DEFAULT, vel: [0.0; 2], rc: Recenter::default() }
        }
    }

    impl Follow {
        /// KH1's re-place `0x1401E4820` (through `0x1401E4700`): pitch and distance to the defaults,
        /// the look point to Sora's target, the eye behind it at `yaw`; if a ray from above Sora
        /// to that eye meets a wall, a 40-unit sphere walked from above Sora toward the wall stops
        /// at its contact and is the eye. Checked (decomp); KH1's second test `0x140193650` (when the
        /// ray misses) is not carried (Assumption: Re:CoM's camera collision covers it).
        pub fn replace(&mut self, yaw: f32, inp: &Input, w: &mut dyn World) {
            self.pitch = PITCH_DEFAULT;
            self.dist = DIST_DEFAULT;
            self.look = inp.target();
            self.eye = self.look + offset(self.dist, self.pitch, wrap_angle(yaw));
            let above = inp.above();
            if let Some(hit) = w.sweep(above, self.eye, RAY) {
                self.eye = w.sweep(above, hit, SPHERE).unwrap_or(hit);
            }
            self.yaw = yaw_of(self.eye, self.look);
        }

        /// KH1's mode-1 entry (`0x142537E44` bit 2): the yaw of the current view, then the re-place.
        pub fn enter(&mut self, view_yaw: f32, inp: &Input, w: &mut dyn World) {
            self.replace(view_yaw, inp, w);
        }

        /// Behind Sora (his facing + π): the camera's first frame in a room and the return from an
        /// event camera (owner's answer Q4 (b): KH1's way).
        pub fn behind(&mut self, inp: &Input, w: &mut dyn World) {
            self.replace(wrap_angle(inp.facing + PI), inp, w);
        }

        /// The look point's spring (no mode blend runs: our modes switch through Re:CoM's setters).
        fn spring_look(&mut self, inp: &Input, w: &mut dyn World) {
            let above = inp.above();
            let mut goal = inp.target();
            if w.sweep(above, goal, RAY).is_some() {
                goal = above;
            }
            let mut d = goal - self.look;
            let dy = d.y;
            d.y = 0.0;
            let (n, len) = d.normalize_len();
            let mut m = n * (len * LOOK_H);
            m.y = if dy <= 0.0 { dy * LOOK_UP } else { dy * LOOK_DOWN };
            if inp.step < 2.0 {
                m = m * 0.5;
            }
            self.look += m;
        }

        /// KH1's stick `0x1401E7320`.
        fn stick(&mut self, inp: &Input) {
            if !inp.can_act || self.rc.left > 0.0 {
                return;
            }
            let half = if inp.step < 2.0 { 0.5 } else { 1.0 };
            let (max, accel, decay) = if inp.mouse { ((MOUSE_SENSITIVITY * 0.05 * 1.75 + 2.5) * PI / 180.0, MOUSE_ACCEL, MOUSE_DECAY) } else { (PAD_MAX, PAD_ACCEL, PAD_DECAY) };
            let [sx, sy] = inp.stick;
            let axis = |v: f32, push: f32, held: bool| -> f32 {
                let v = v + push;
                if v == 0.0 {
                    return 0.0;
                }
                let c = if v <= max { v } else { max };
                let mut r = if -max <= c { c } else { -max };
                if !held {
                    let a = (r.abs() - decay * half).max(0.0);
                    r = a * (r / r.abs());
                }
                r
            };
            self.vel[0] = axis(self.vel[0], -accel * sy * half, sy != 0.0);
            self.vel[1] = axis(self.vel[1], accel * sx * half, sx != 0.0);
            if inp.mouse {
                // KH1's mouse multiplier `|dx × 10|` (at least 1) has no Re:CoM source: 1.
                if sy == 0.0 {
                    self.vel[0] = 0.0;
                }
                if sx == 0.0 {
                    self.vel[1] = 0.0;
                }
                if (0.0 < -self.vel[0] && sy < 0.0) || (-self.vel[0] < 0.0 && 0.0 < sy) {
                    self.vel[0] = 0.0;
                }
                if (0.0 < self.vel[1] && sx < 0.0) || (self.vel[1] < 0.0 && 0.0 < sx) {
                    self.vel[1] = 0.0;
                }
            }
            let p = wrap_angle(inp.inv[1] * self.vel[0] * inp.step + self.pitch);
            self.pitch = p.max(PITCH_MIN).min(PITCH_MAX);
            self.yaw = wrap_angle(inp.inv[0] * self.vel[1] * inp.step + self.yaw);
        }

        /// The recenter's blend, each frame while it runs (`0x1401E1920`).
        fn blend(&mut self, step: f32) {
            if self.rc.left <= 0.0 {
                return;
            }
            let t = self.rc.left - step;
            self.rc.left = t;
            let r = &self.rc;
            if r.kind < 2 {
                let s = ((1.0 - t / r.total) * PI * 0.5).sin();
                if r.kind == 1 {
                    self.yaw = wrap_angle(r.delta * s + r.from);
                } else {
                    self.yaw = wrap_angle(r.from - r.delta * s);
                }
                self.pitch = (1.0 - r.left / r.total) * PITCH_DEFAULT + r.pitch * (r.left / r.total);
            } else if SWAP_MIN < r.delta.abs() {
                let c = ((t / r.total) * PI).cos();
                if 0.0 < c {
                    self.yaw = wrap_angle(r.delta + r.from);
                }
                let f = c.abs() * 0.7 + 0.3;
                self.dist = r.dist * f;
                self.pitch = wrap_angle((1.0 - f) * SWAP_PITCH + PITCH_DEFAULT);
            }
            if self.rc.left <= 0.0 {
                self.dist = DIST_DEFAULT;
                self.pitch = PITCH_DEFAULT;
                self.rc.left = 0.0;
            }
        }

        /// KH1's recenter start `0x1401E62A0`: toward Sora's back (facing + π) in 60 frames, the
        /// short way if every 30° step of the swing is clear, else the long way, else the swap.
        pub fn start_recenter(&mut self, facing: f32, w: &mut dyn World) {
            if 0.0 < self.rc.left {
                return;
            }
            let delta = wrap_angle(facing + PI - self.yaw);
            let mut kind = (0.0 < delta) as u32;
            let at = |yaw: f32| self.look + offset(self.dist, self.pitch, wrap_angle(yaw));
            let (kind, amount) = if w.sweep(self.look, at(self.yaw + delta), RAY).is_none() {
                let step = if kind == 0 { -RECENTER_STEP } else { RECENTER_STEP };
                let ad = delta.abs();
                let mut prev = self.eye;
                let mut f = step;
                let mut clear = true;
                while f.abs() <= ad {
                    let next = at(self.yaw + f);
                    if w.sweep(prev, next, RAY).is_some() {
                        clear = false;
                        break;
                    }
                    prev = next;
                    f += step;
                }
                if clear {
                    (kind, ad)
                } else {
                    // The other way round.
                    let ad = 2.0 * PI - ad;
                    let mut prev = self.eye;
                    let mut f = -step;
                    loop {
                        if ad < f.abs() {
                            kind ^= 1;
                            break (kind, ad);
                        }
                        let next = at(self.yaw + f);
                        if w.sweep(prev, next, RAY).is_some() {
                            break (2, ad);
                        }
                        prev = next;
                        f -= step;
                    }
                }
            } else {
                (2, delta)
            };
            self.rc = Recenter { left: RECENTER_FRAMES, total: RECENTER_FRAMES, kind, from: self.yaw, delta: amount, dist: DIST_DEFAULT, pitch: self.pitch };
        }

        /// The Manual branch's eye (`0x1401E1920`, config `+0x8` = 1).
        fn manual_eye(&mut self, w: &mut dyn World) {
            let goal = self.look + offset(self.dist, self.pitch, self.yaw);
            self.eye = goal;
            let (n, _) = (goal - self.look).normalize_len();
            let past = goal + n * RAY_PAST;
            if let Some(hit) = w.sweep(self.look, past, RAY) {
                let (back, len) = (self.look - hit).normalize_len();
                self.eye = if len <= RAY_PAST { hit } else { hit + back * RAY_PAST };
            }
            let d = self.look - self.eye;
            let len = d.length();
            if len < NEAR_LOOK {
                self.look += d.normalized() * (NEAR_LOOK - len);
            }
        }

        /// One frame of KH1's mode 1 (Manual branch; the Auto pieces come with their stage), after
        /// any entry: the look point's spring, the stick, the recenter's blend, the eye and its wall
        /// ray, R3, the distance, the yaw from the result.
        pub fn step(&mut self, inp: &Input, w: &mut dyn World) {
            self.spring_look(inp, w);
            self.stick(inp);
            self.blend(inp.step);
            self.manual_eye(w);
            if inp.recenter && inp.can_act {
                self.start_recenter(inp.facing, w);
            }
            // Manual: the distance stays the default (Auto springs it back 10 % a frame).
            self.dist = DIST_DEFAULT;
            self.yaw = yaw_of(self.eye, self.look);
        }
    }

    /// Re:CoM's right stick ints (±4096, `0x14029B280` / `0x14029B290`) as KH1's camera stick: the
    /// raw bytes rebuilt (`units::byte_from_recom_axis`), then KH1's radial dead zone.
    pub fn stick_from_recom(x: i32, y: i32, mouse: bool) -> [f32; 2] {
        let dz = if mouse { MOUSE_DEAD_ZONE } else { DEAD_ZONE };
        kh1_core::params::stick_from_raw(units::byte_from_recom_axis(x), units::byte_from_recom_axis(y), dz).0
    }

    /// KH1's inversion floats from Re:CoM's camera fields (`+0x2F0` horizontal, `+0x2F4` vertical,
    /// read every frame by `0x1401FCCF0` from its Game Settings rows 0x66 / 0x65): Re:CoM's Normal
    /// is (−1, +1), KH1's Normal (left / right +1, up / down −1: its Config reset, `0x1402FA295`
    /// case 7). Owner Q5 (a): each game's own Normal, so KH1's float is the negated Re:CoM one.
    /// Checked (decomp both).
    pub fn inversion(recom_h: f32, recom_v: f32) -> [f32; 2] {
        [-recom_h, -recom_v]
    }
}

/// Converts the step's result for Re:CoM's camera (metres, y up).
pub fn to_recom4(v: Vec3) -> [f32; 4] {
    let r = units::to_recom(v);
    [r.x, r.y, r.z, 1.0]
}

/// The stubs at Re:CoM's two record-table reads, given where they sit. Each runs the bytes it
/// replaced, and for a mode 6..9 reads our table instead (and counts the mode set, so our step
/// knows Re:CoM's setter ran: an entry).
pub mod stub {
    /// `0x1401FC513` (in `0x1401FC4D0`): `lea rdx,[rip+0x67F146]` (the table `0x14087B660`);
    /// `mov rsi,rcx`; `mov rdx,[rdx+rax*8]`, 14 bytes, back at `0x1401FC521`. Checked (exe).
    pub const SET_PLACE_READ: u64 = 0x1_401F_C513;
    pub const SET_PLACE_BACK: u64 = 0x1_401F_C521;
    /// `0x1401FBE85` (in `0x1401FBE60`): `lea rdx,[rip+0x67F7D4]`; `mov rdx,[rdx+rax*8]`, 11 bytes,
    /// back at `0x1401FBE90`. Checked (exe).
    pub const SET_KEEP_READ: u64 = 0x1_401F_BE85;
    pub const SET_KEEP_BACK: u64 = 0x1_401F_BE90;
    /// Re:CoM's record table. Checked (decomp: the ctor `0x1401FB140` fills slots 1..5).
    pub const RECOM_TABLE: u64 = 0x1_4087_B660;

    fn rel32(from_end: u64, to: u64) -> [u8; 4] {
        let d = to as i64 - from_end as i64;
        assert!(i32::try_from(d).is_ok(), "rel32 out of reach");
        (d as i32).to_le_bytes()
    }

    /// The stub at `at`: `[mov rsi,rcx]`; `cmp eax,6; jl recom; cmp eax,10; jae recom;
    /// inc dword [counter]; lea rdx,[ours − 0x30]; mov rdx,[rdx+rax*8]; jmp back;
    /// recom: lea rdx,[table]; mov rdx,[rdx+rax*8]; jmp back`.
    pub fn table_read(at: u64, back: u64, ours: u64, counter: u64, mov_rsi: bool) -> Vec<u8> {
        let mut c: Vec<u8> = Vec::new();
        if mov_rsi {
            c.extend([0x48, 0x8B, 0xF1]);
        }
        c.extend([0x83, 0xF8, 0x06, 0x7C, 0x00]);
        let jl = c.len() - 1;
        c.extend([0x83, 0xF8, 0x0A, 0x73, 0x00]);
        let jae = c.len() - 1;
        c.extend([0xFF, 0x05]);
        let e = at + c.len() as u64 + 4;
        c.extend(rel32(e, counter));
        c.extend([0x48, 0x8D, 0x15]);
        let e = at + c.len() as u64 + 4;
        c.extend(rel32(e, ours - 8 * super::mode::FIRST as u64));
        c.extend([0x48, 0x8B, 0x14, 0xC2, 0xE9]);
        let e = at + c.len() as u64 + 4;
        c.extend(rel32(e, back));
        let recom = c.len();
        c[jl] = (recom - (jl + 1)) as u8;
        c[jae] = (recom - (jae + 1)) as u8;
        c.extend([0x48, 0x8D, 0x15]);
        let e = at + c.len() as u64 + 4;
        c.extend(rel32(e, RECOM_TABLE));
        c.extend([0x48, 0x8B, 0x14, 0xC2, 0xE9]);
        let e = at + c.len() as u64 + 4;
        c.extend(rel32(e, back));
        c
    }

    /// The patch at a table read: `jmp stub`, NOPs to the replaced length.
    pub fn jump(at: u64, stub: u64, len: usize) -> Vec<u8> {
        let mut c = vec![0xE9];
        c.extend(rel32(at + 5, stub));
        c.resize(len, 0x90);
        c
    }

    /// The init's `lea rax,[rip+…]` pointed at `to`.
    pub fn lea_rax(at: u64, to: u64) -> [u8; 7] {
        let r = rel32(at + 7, to);
        [0x48, 0x8D, 0x05, r[0], r[1], r[2], r[3]]
    }

    /// The block near the exe: the two stubs, the two exec jumps, our record table, the entry
    /// counter, the records.
    pub mod layout {
        pub const STUB_PLACE: usize = 0x000;
        pub const STUB_KEEP: usize = 0x060;
        pub const JUMP_FIELD: usize = 0x0C0;
        pub const JUMP_BATTLE: usize = 0x0D0;
        pub const TABLE: usize = 0x100;
        pub const COUNTER: usize = 0x120;
        pub const RECORDS: usize = 0x140;
        pub const LEN: usize = RECORDS + super::super::mode::COUNT * 0x30;
    }

    /// The whole block for an address.
    pub fn block(at: u64, field_exec: u64, battle_exec: u64) -> Vec<u8> {
        use layout::*;
        let mut b = vec![0xCCu8; LEN];
        let a = table_read(at + STUB_PLACE as u64, SET_PLACE_BACK, at + TABLE as u64, at + COUNTER as u64, true);
        b[STUB_PLACE..STUB_PLACE + a.len()].copy_from_slice(&a);
        let k = table_read(at + STUB_KEEP as u64, SET_KEEP_BACK, at + TABLE as u64, at + COUNTER as u64, false);
        b[STUB_KEEP..STUB_KEEP + k.len()].copy_from_slice(&k);
        b[JUMP_FIELD..JUMP_FIELD + 14].copy_from_slice(&crate::hookplan::abs_jmp(field_exec));
        b[JUMP_BATTLE..JUMP_BATTLE + 14].copy_from_slice(&crate::hookplan::abs_jmp(battle_exec));
        for (i, r) in super::records().iter().enumerate() {
            let ra = at + (RECORDS + i * 0x30) as u64;
            b[TABLE + i * 8..TABLE + i * 8 + 8].copy_from_slice(&ra.to_le_bytes());
            b[RECORDS + i * 0x30..RECORDS + i * 0x30 + 0x30].copy_from_slice(&r.bytes());
        }
        b[COUNTER..COUNTER + 4].copy_from_slice(&0u32.to_le_bytes());
        b
    }
}

#[cfg(test)]
mod tests {
    use super::kh1::*;
    use super::*;

    fn inp() -> Input {
        Input { pos: Vec3::ZERO, facing: 0.0, stick: [0.0; 2], mouse: false, can_act: true, recenter: false, inv: [1.0, -1.0], step: 1.0 }
    }

    #[test]
    fn modes_records_and_dispatch() {
        assert!(mode::ours(6) && mode::ours(9) && !mode::ours(5) && !mode::ours(10));
        assert_eq!((Family::of(6), Family::of(7), Family::of(3)), (Family::Kh1, Family::Kh1, Family::Recom));
        let r = records()[0];
        assert_eq!(r.fov, 53.0);
        // The rest vector's length is KH1's 4.0 m, 10° up; the aim 1.0 m above the feet.
        let rest = [r.eye[0] - r.aim[0], r.eye[1] - r.aim[1], r.eye[2] - r.aim[2]];
        assert!(((rest[0] * rest[0] + rest[1] * rest[1] + rest[2] * rest[2]).sqrt() - 4.0).abs() < 1e-5);
        assert!((rest[1].atan2(rest[2]) - 10f32.to_radians()).abs() < 1e-5);
        assert!((r.aim[1] + SUBJECT_LIFT_M - 1.0).abs() < 1e-6);
        assert_eq!(&r.bytes()[0..4], &53f32.to_le_bytes());
        // Dispatch: Re:CoM's camera untouched at Re:CoM; ours replaces 1 / 3 / 4 only.
        assert_eq!(dispatch(1, Family::Recom, false, false, false), Dispatch::Recom);
        assert_eq!(dispatch(1, Family::Kh1, false, false, false), Dispatch::Switch(6));
        assert_eq!(dispatch(4, Family::Kh1, true, false, false), Dispatch::Switch(6));
        assert_eq!(dispatch(3, Family::Kh1, true, true, false), Dispatch::Recom, "a sleight's mode stays");
        assert_eq!(dispatch(5, Family::Kh1, true, false, false), Dispatch::Recom);
        assert_eq!(dispatch(2, Family::Kh1, false, false, false), Dispatch::Recom, "Re:CoM's timed view");
        assert_eq!(dispatch(3, Family::Kh1, true, false, true), Dispatch::Recom, "a subject override");
        assert_eq!(dispatch(6, Family::Kh1, true, false, false), Dispatch::Ours);
        assert_eq!(dispatch(6, Family::Recom, true, false, false), Dispatch::Switch(3));
        assert_eq!(dispatch(6, Family::Recom, false, false, false), Dispatch::Switch(1));
        assert_eq!(wanted(Family::Kh1, true, false), Family::Recom);
        assert_eq!(wanted(Family::Kh1, false, true), Family::Recom);
        assert_eq!(wanted(Family::Kh1, false, false), Family::Kh1);
    }

    #[test]
    fn offsets_and_conversions() {
        // 10° pitch: the eye above (−y in KH1), behind for yaw 0 (+z).
        let o = offset(400.0, PITCH_DEFAULT, 0.0);
        assert!((o.y + 400.0 * PITCH_DEFAULT.sin()).abs() < 1e-3 && o.z > 0.0);
        assert!((yaw_of(o, Vec3::ZERO)).abs() < 1e-6);
        let o = offset(400.0, 0.3, 1.2);
        assert!((yaw_of(o, Vec3::ZERO) - 1.2).abs() < 1e-5);
        // In Re:CoM: the eye above (+y) and 4 m away.
        let r = to_recom4(offset(400.0, PITCH_DEFAULT, 0.7));
        assert!(r[1] > 0.0 && ((r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt() - 4.0).abs() < 1e-4);
        // The target: 1 m up, 0.5 m behind Sora (facing +z).
        let t = inp().target();
        assert_eq!((t.x, t.y, t.z), (0.0, -100.0, -50.0));
        // Inversion: Re:CoM's Normal (h −1, v +1) is KH1's Normal (+1, −1).
        assert_eq!(inversion(-1.0, 1.0), [1.0, -1.0]);
        assert_eq!(inversion(1.0, -1.0), [-1.0, 1.0]);
        // The stick: full tilt right is KH1's 1.0; Re:CoM's dead zone stays inside KH1's.
        assert!((stick_from_recom(4096, 0, false)[0] - 1.0).abs() < 1e-6);
        assert_eq!(stick_from_recom(1000, 0, false)[0], 0.0);
        assert!(stick_from_recom(1000, 0, true)[0] > 0.0);
        assert!(stick_from_recom(0, 4096, false)[1] > 0.99);
    }

    #[test]
    fn behind_places_kh1s_default_eye() {
        let mut f = Follow::default();
        let i = Input { pos: Vec3::new(100.0, 0.0, 200.0), facing: 0.5, ..inp() };
        f.behind(&i, &mut Open);
        assert_eq!(f.look, i.target());
        let d = f.eye - f.look;
        assert!((d.length() - 400.0).abs() < 1e-2);
        assert!((wrap_angle(f.yaw - (0.5 + PI))).abs() < 1e-5);
        assert!((-d.y / 400.0 - PITCH_DEFAULT.sin()).abs() < 1e-5);
    }

    #[test]
    fn the_look_point_springs_at_kh1s_rates() {
        let mut f = Follow::default();
        let i = inp();
        f.behind(&i, &mut Open);
        // Sora moves 100 along +x: 10 % of the horizontal gap a frame (20 % halved).
        let i2 = Input { pos: Vec3::new(100.0, 0.0, 0.0), ..i };
        let before = f.look;
        f.step(&i2, &mut Open);
        assert!((f.look.x - before.x - 10.0).abs() < 1e-3, "{:?}", f.look);
        // Up 100 (y −100): 5 % a frame; down: 25 %.
        let mut g = Follow::default();
        g.behind(&i, &mut Open);
        g.step(&Input { pos: Vec3::new(0.0, -100.0, 0.0), ..i }, &mut Open);
        assert!((g.look.y - (-100.0 - 5.0)).abs() < 1e-3, "{:?}", g.look);
        let mut h = Follow::default();
        h.behind(&i, &mut Open);
        h.step(&Input { pos: Vec3::new(0.0, 100.0, 0.0), ..i }, &mut Open);
        assert!((h.look.y - (-100.0 + 25.0)).abs() < 1e-3, "{:?}", h.look);
    }

    #[test]
    fn the_stick_accelerates_to_2_5_degrees_a_frame() {
        let mut f = Follow::default();
        let i = inp();
        f.behind(&i, &mut Open);
        let y0 = f.yaw;
        let full = Input { stick: [1.0, 0.0], ..i };
        let mut frames = 0;
        let mut last = y0;
        let mut rates = Vec::new();
        for _ in 0..40 {
            f.step(&full, &mut Open);
            rates.push(wrap_angle(f.yaw - last));
            last = f.yaw;
            frames += 1;
        }
        // 0.075° a frame more each frame, up to 2.5° (34 frames).
        assert!((rates[0] - 0.075f32.to_radians()).abs() < 1e-5, "{}", rates[0].to_degrees());
        assert!((rates[1] - 0.15f32.to_radians()).abs() < 1e-5);
        assert!((rates[39] - PAD_MAX).abs() < 1e-5);
        assert_eq!(frames, 40);
        // Released: 0.12° a frame less.
        f.step(&i, &mut Open);
        let r = wrap_angle(f.yaw - last);
        assert!((r - (PAD_MAX - 0.12f32.to_radians())).abs() < 1e-5, "{}", r.to_degrees());
        // Pitch: KH1's Normal (−1) with the stick down raises the eye, clamped at 50°.
        let mut g = Follow::default();
        g.behind(&i, &mut Open);
        for _ in 0..200 {
            g.step(&Input { stick: [0.0, 1.0], ..i }, &mut Open);
        }
        assert!((g.pitch - PITCH_MAX).abs() < 1e-5, "{}", g.pitch);
        for _ in 0..400 {
            g.step(&Input { stick: [0.0, -1.0], ..i }, &mut Open);
        }
        assert!((g.pitch - PITCH_MIN).abs() < 1e-5);
        // No turning while Sora can't act.
        let y = g.yaw;
        g.step(&Input { stick: [1.0, 0.0], can_act: false, ..i }, &mut Open);
        assert!((g.yaw - y).abs() < 1e-6);
    }

    #[test]
    fn the_mouse_rule() {
        let mut f = Follow::default();
        let i = Input { mouse: true, ..inp() };
        f.behind(&i, &mut Open);
        let mut last = f.yaw;
        let mut r = 0.0;
        for _ in 0..30 {
            f.step(&Input { stick: [1.0, 0.0], ..i }, &mut Open);
            r = wrap_angle(f.yaw - last);
            last = f.yaw;
        }
        assert!((r - 4.25f32.to_radians()).abs() < 1e-5, "{}", r.to_degrees());
        // The mouse stops: the turn stops at once.
        f.step(&i, &mut Open);
        assert!(wrap_angle(f.yaw - last).abs() < 1e-6);
    }

    #[test]
    fn r3_swings_behind_in_60_frames() {
        let mut f = Follow::default();
        let i = inp();
        f.behind(&i, &mut Open);
        // Sora turns 90°; R3.
        let i2 = Input { facing: PI / 2.0, ..i };
        f.step(&Input { recenter: true, ..i2 }, &mut Open);
        assert_eq!(f.rc.left, 60.0);
        assert_eq!(f.rc.kind, 1);
        let from = f.rc.from;
        for n in 1..=60 {
            f.step(&i2, &mut Open);
            let want = wrap_angle(from + f.rc.delta * ((n as f32 / 60.0) * PI * 0.5).sin());
            assert!(wrap_angle(f.yaw - want).abs() < 2e-3, "frame {n}: {} {}", f.yaw, want);
        }
        assert_eq!(f.rc.left, 0.0);
        assert!(wrap_angle(f.yaw - (PI / 2.0 + PI)).abs() < 2e-3, "{}", f.yaw);
    }

    /// A wall plane at z = `z` (KH1 units) for the collision tests.
    struct Wall(f32);
    impl World for Wall {
        fn sweep(&mut self, from: Vec3, to: Vec3, r: f32) -> Option<Vec3> {
            let s = (to.z - from.z).signum();
            if to.z == from.z {
                return None;
            }
            let t = (self.0 - s * r - from.z) / (to.z - from.z);
            (0.0..=1.0).contains(&t).then(|| from + (to - from) * t)
        }
    }

    #[test]
    fn the_manual_ray_keeps_the_eye_60_off_a_wall() {
        let mut f = Follow::default();
        let i = inp();
        f.behind(&i, &mut Open);
        // A wall 200 behind the look point (the eye would be ~394 behind, at −z: Sora faces +z).
        let wall = f.look.z - 200.0;
        let mut w = Wall(wall);
        f.step(&i, &mut w);
        // The ray stops at the wall (less its 0.5 radius); the eye is 60 back toward the look point.
        let n = (f.eye - f.look).normalized();
        let hit_z = wall + RAY;
        assert!((f.eye.z - (hit_z - RAY_PAST * n.z)).abs() < 1e-2, "{:?} {:?}", f.eye, f.look);
        assert!((f.eye - f.look).length() > NEAR_LOOK);
        // No wall: the eye at 400.
        let mut g = Follow::default();
        g.behind(&i, &mut Open);
        g.step(&i, &mut Open);
        assert!(((g.eye - g.look).length() - 400.0).abs() < 1e-2);
    }

    #[test]
    fn stubs_and_block() {
        use super::stub::*;
        let at = 0x1_3FF0_0000u64;
        let s = table_read(at, SET_PLACE_BACK, at + 0x100, at + 0x120, true);
        assert_eq!(&s[..3], &[0x48, 0x8B, 0xF1]);
        assert!(s.len() <= layout::STUB_KEEP);
        let k = table_read(at + 0x60, SET_KEEP_BACK, at + 0x100, at + 0x120, false);
        assert!(k.len() <= layout::JUMP_FIELD - layout::STUB_KEEP);
        // The jl / jae land on the Re:CoM path, which reads 0x14087B660.
        let jl = s[7] as usize + 8;
        assert_eq!(&s[jl..jl + 3], &[0x48, 0x8D, 0x15]);
        let disp = i32::from_le_bytes(s[jl + 3..jl + 7].try_into().unwrap()) as i64;
        assert_eq!((at as i64 + jl as i64 + 7 + disp) as u64, RECOM_TABLE);
        let jae = s[12] as usize + 13;
        assert_eq!(jae, jl);
        // Our path reads the table minus 0x30 (mode 6 → slot 0).
        let ours = 13 + 6;
        assert_eq!(&s[ours..ours + 3], &[0x48, 0x8D, 0x15]);
        let disp = i32::from_le_bytes(s[ours + 3..ours + 7].try_into().unwrap()) as i64;
        assert_eq!((at as i64 + ours as i64 + 7 + disp) as u64, at + 0x100 - 0x30);
        let b = block(at, 0x7FF0_0000_1000, 0x7FF0_0000_2000);
        assert_eq!(b.len(), layout::LEN);
        let rec0 = u64::from_le_bytes(b[layout::TABLE..layout::TABLE + 8].try_into().unwrap());
        assert_eq!(rec0, at + layout::RECORDS as u64);
        assert_eq!(&b[layout::RECORDS..layout::RECORDS + 0x30], &records()[0].bytes());
        assert_eq!(jump(SET_KEEP_READ, at, 11).len(), 11);
        assert_eq!(lea_rax(0x1_401F_B556, 0x1_401F_EEB0), [0x48, 0x8D, 0x05, 0x53, 0x39, 0x00, 0x00], "Re:CoM's own lea back");
        assert_eq!(lea_rax(0x1_401F_B587, 0x1_401F_ECE0), [0x48, 0x8D, 0x05, 0x52, 0x37, 0x00, 0x00]);
    }

    /// Stage 1's per-frame cost of the KH1 step's own arithmetic (Re:CoM's sweeps excluded: two a
    /// frame, against one in Re:CoM's follow). Run with `--ignored --nocapture`.
    #[test]
    #[ignore]
    fn cost_of_a_step() {
        let mut f = Follow::default();
        let i = inp();
        f.behind(&i, &mut Open);
        let n = 1_000_000;
        let t = std::time::Instant::now();
        for k in 0..n {
            let j = Input { pos: Vec3::new((k % 100) as f32, 0.0, 0.0), stick: [((k / 50) % 3) as f32 - 1.0, 0.0], ..i };
            f.step(&j, &mut Open);
        }
        let ns = t.elapsed().as_nanos() as f64 / n as f64;
        println!("KH1 follow step: {ns:.1} ns a frame (math only)");
        assert!(f.yaw.is_finite());
    }
}
