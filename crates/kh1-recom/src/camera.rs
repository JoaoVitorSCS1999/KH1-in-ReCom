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
    Kh2,
}

impl Family {
    pub fn of(m: i32) -> Family {
        match m {
            mode::KH1_FOLLOW | mode::KH1_LOCK => Family::Kh1,
            mode::KH2_FOLLOW | mode::KH2_LOCK => Family::Kh2,
            _ => Family::Recom,
        }
    }

    /// The family's base mode (its follow); Re:CoM's by room kind.
    pub fn base(self, battle: bool) -> i32 {
        match self {
            Family::Kh1 => mode::KH1_FOLLOW,
            Family::Kh2 => mode::KH2_FOLLOW,
            Family::Recom if battle => mode::RECOM_BATTLE,
            Family::Recom => mode::RECOM_FIELD,
        }
    }

    pub fn from_setting(c: crate::settings::Camera) -> Family {
        match c {
            crate::settings::Camera::Recom => Family::Recom,
            crate::settings::Camera::Kh1 => Family::Kh1,
            crate::settings::Camera::Kh2 => Family::Kh2,
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
/// `subject`: a subject override is set (`+0x2B4 > 0`); `locked`: Re:CoM's lock-on handle
/// (`player+0x850 > 0`), None while a subject object holds the camera (`+0x2B8`, where Re:CoM's
/// battle exec skips its 3 ↔ 4 switch too). Re:CoM's modes are only replaced from its base follows
/// (1 / 3 / 4), never during a sleight's request or its timed view 2. Our follow and lock modes
/// switch on the lock as Re:CoM's 3 ↔ 4 do, in field rooms too (owner Q12 (a)): KH1's 6 ↔ 7,
/// KH2's 8 ↔ 9 (KH2's own lock start `0x140176910` / end `0x140176A00` set its mode 2 / 0 and
/// nothing else).
pub fn dispatch(m: i32, want: Family, battle: bool, external: bool, subject: bool, locked: Option<bool>) -> Dispatch {
    if mode::ours(m) {
        if Family::of(m) == want {
            match (m, locked) {
                (mode::KH1_FOLLOW, Some(true)) => Dispatch::Switch(mode::KH1_LOCK),
                (mode::KH1_LOCK, Some(false)) => Dispatch::Switch(mode::KH1_FOLLOW),
                (mode::KH2_FOLLOW, Some(true)) => Dispatch::Switch(mode::KH2_LOCK),
                (mode::KH2_LOCK, Some(false)) => Dispatch::Switch(mode::KH2_FOLLOW),
                _ => Dispatch::Ours,
            }
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
/// - Modes 8 / 9 (KH2): FOV [`kh2::recom_fov_degrees`] (KH2's 1.5 rad at Re:CoM's focal, 73.39°);
///   KH2's look point 170 above the feet and its eye at the default 420 on its height curve
///   (81.2 above the look point), × 0.89 (1.513 m, 3.738 m, 0.723 m).
pub fn records() -> [Record; mode::COUNT] {
    let look = kh1::LOOK_HEIGHT_M - SUBJECT_LIFT_M;
    let (s, c) = kh1::PITCH_DEFAULT.sin_cos();
    let d = kh1::DIST_DEFAULT * units::METRES_PER_UNIT;
    let kh1 = Record { fov: kh1::FOV, radius: 0.05, ease: 1.0, wall: 0.0, eye: [0.0, look + d * s, d * c, 1.0], aim: [0.0, look, 0.0, 1.0] };
    let m = kh2::SCALE * units::METRES_PER_UNIT;
    let look2 = kh2::LOOK_UP * m - SUBJECT_LIFT_M;
    let c2 = kh2::DIST_DEFAULT - kh2::DIST_MIN;
    let up2 = (c2 * c2 * kh2::HEIGHT_K - kh2::EYE_BASE) * m;
    let kh2r = Record { fov: kh2::recom_fov_degrees(), radius: 0.05, ease: 1.0, wall: 0.0, eye: [0.0, look2 + up2, kh2::DIST_DEFAULT * m, 1.0], aim: [0.0, look2, 0.0, 1.0] };
    // Mode 7 (KH1's lock camera) places itself; Re:CoM's setter gets KH1's follow numbers.
    [kh1, kh1, kh2r, kh2r]
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
        /// The surface normal of the last contact (KH1's polygon normal `0x140191DD0`; KH1 units,
        /// y down). Default: up.
        fn normal(&mut self) -> Vec3 {
            Vec3::new(0.0, -1.0, 0.0)
        }
        /// KH1's `0x140196300` with a flat circle of `radius` at `at` (the swing-behind's wall
        /// probe): a wall within `radius` horizontally.
        fn near_wall(&mut self, _at: Vec3, _radius: f32) -> bool {
            false
        }
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
        /// KH1's Auto camera (its Config `+0x8` = 0; owner: the Auto camera row).
        pub auto: bool,
        /// Sora's KH1 movement state (`+0x70`: 0 ground, 2 air, 0x15 glide …), for the speed
        /// thresholds and the pitch assist's skips.
        pub state: u32,
        /// Sora's air velocity (`+0x3C0`, units a frame, y down).
        pub vel: Vec3,
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

    /// The swing-behind's ramps (`0x1425380DC..0x1425380E8`).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Swing {
        /// The wall ramp (`0x1425380E0`), the second time ramp (`E4`), the first (`E8`).
        pub wall: f32,
        pub second: f32,
        pub first: f32,
        /// Sora's facing last frame (`0x1425380DC`).
        pub facing: f32,
    }

    /// The wall-avoid's swing (`0x142537ED0..ED8`).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Avoid {
        pub from: f32,
        pub delta: f32,
        pub left: f32,
    }

    /// The pitch assist's timers (`0x1425380D0` drop, `D4` floor under the eye, `D8` standing).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Assist {
        pub drop: f32,
        pub floor: f32,
        pub still: f32,
    }

    /// KH1's follow camera state (its globals `0x142537E60..`, `0x1425380C0..0x1425380F0`).
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
        pub swing: Swing,
        pub avoid: Avoid,
        pub assist: Assist,
        /// Sora's position last frame (KH1's `+0x490`) and his speeds this frame: 3D (`0x142537E50`)
        /// and horizontal (`E54`), units a frame (`0x1401E5D90`).
        pub prev: Option<Vec3>,
        pub speed: f32,
        pub speed_h: f32,
        /// The lock camera's state (mode 3).
        pub lock: Lock,
        /// The camera task's mode-switch blend of the drawn view.
        pub blend: Blend,
    }

    /// KH1's mode-switch blend (`0x1401E5360`, the task `0x1401E5D90`): the drawn eye / look go
    /// from where they were to the new mode's with weight `cos(left / total · π/2)` on the old.
    /// Checked (decomp; `0x140386B70` = `a·t + b·(1 − t)`, `0x1400E7CB0`).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Blend {
        pub from_eye: Vec3,
        pub from_look: Vec3,
        pub left: f32,
        pub total: f32,
    }

    /// The lock camera's own globals: the anchor's height assist (`0x14253810C`), the keep-Sora
    /// counter (`0x142538108`), the re-lock blend (`0x142537F08`, from `0x142537F10`).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Lock {
        pub height: f32,
        pub screen: f32,
        pub relock: f32,
        pub old_look: Vec3,
    }

    /// The lock target as Re:CoM gives it: its point (Re:CoM's lock-on point `player+0x870`, its
    /// `CamTrgOfs` point) and KH1's lock radius for it (the twin's, `win::stats::lock_radius`).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Target {
        pub point: Vec3,
        pub radius: f32,
    }

    /// The lock camera's numbers (`0x1401E2650`; Checked (decomp + exe)): the anchor
    /// (0, −100, 350) in Sora's frame turned by the camera's yaw (`0x140506DF0`), the look-past
    /// (radius + 100), the eye's 75 dead zone and 90 % close, the ±5° yaw band at half the error
    /// up to 10° a frame, the point 100 ahead (`0x140506E00`), the keep-Sora counter (to 40, −2 a
    /// frame inside 80 % of the half FOV, 80 % pull), the re-lock blend of 60, the height assist
    /// (floor under the eye: 30 / 150 → up to −170; a target over 200 above the eye → up to 20).
    pub mod lockcam {
        use kh1_core::Vec3;
        pub const ANCHOR: Vec3 = Vec3::new(0.0, -100.0, 350.0);
        pub const AHEAD: Vec3 = Vec3::new(0.0, 0.0, -100.0);
        pub const PAST: f32 = 100.0;
        pub const DEAD: f32 = 75.0;
        pub const CLOSE: f32 = 0.9;
        pub const BAND: f32 = 0.087_266_47;
        pub const TURN_MAX: f32 = 0.174_532_94;
        pub const SCREEN_MAX: f32 = 40.0;
        pub const SCREEN_PULL: f32 = 0.8;
        pub const RELOCK: f32 = 60.0;
        /// Sora's body point (code `0x8003`: his record {0x8002, 0, 85}; `recom-m13-kh1-fx.md`).
        pub const BODY: f32 = 85.0;
        pub const ENTER_BLEND: f32 = 60.0;
        pub const EXIT_BLEND: f32 = 45.0;
        pub const FLOOR_PROBE: f32 = 10000.0;
    }

    impl Default for Follow {
        fn default() -> Self {
            Follow { look: Vec3::ZERO, eye: Vec3::ZERO, pitch: PITCH_DEFAULT, yaw: 0.0, dist: DIST_DEFAULT, vel: [0.0; 2], rc: Recenter::default(), swing: Swing::default(), avoid: Avoid::default(), assist: Assist::default(), prev: None, speed: 0.0, speed_h: 0.0, lock: Lock::default(), blend: Blend::default() }
        }
    }

    /// Sora "moves" for the camera: faster than his state's threshold (1.0 a frame on the
    /// ground, 7.0 in states 8 / 0x18, else 0.1; `0x1401E6080`). Checked (decomp).
    pub fn moving(state: u32, speed: f32) -> bool {
        let t = match state {
            0 => 1.0,
            8 | 0x18 => 7.0,
            _ => 0.1,
        };
        t < speed
    }

    /// The Auto pieces' numbers (Checked (decomp)): the swing-behind `0x1401E6FF0` (ramps of 60
    /// then 90 frames, a wall ramp of 60, with the mouse 0 / 40 / 20; gains 2 % + 2 % of the angle
    /// to Sora's back, faded past 120°, none at 180°; a facing change of 30° resets); its wall
    /// probe radius 250; the wall-avoid `0x1401E6C10` (side rays 50 off, ±30° rays of 300, a
    /// 40-frame sin swing); the pitch assist `0x1401E6670` (3° a frame × its ramp of 30, at most
    /// 85°, the eye's floor ray 55 with 49.5, back to the default pitch 3 % a frame; the standing
    /// ledge look after 60 frames: 100 ahead, then 500 down from 30 ahead 100 up; in the air 1000
    /// down, or along a fall faster than 5 for 250); the eye walk (a 40 sphere, re-placed past
    /// default + 300, a jump at 1000); the visibility re-place `0x1401E7780` (5 segments from the
    /// eye: above Sora, ±15° of it, ±50 up and down); the auto recenter (eye within 40
    /// horizontally, pitch under 60°, Sora moving); the distance back 10 % a frame.
    pub mod auto {
        pub const RAMP1: f32 = 60.0;
        pub const RAMP2: f32 = 90.0;
        pub const WALL_RAMP: f32 = 60.0;
        pub const MOUSE_RAMP1: f32 = 0.0;
        pub const MOUSE_RAMP2: f32 = 40.0;
        pub const MOUSE_WALL_RAMP: f32 = 20.0;
        pub const GAIN: f32 = 0.02;
        pub const FACING_RESET: f32 = 0.523_598_8;
        pub const FADE_FROM: f32 = 2.094_395_2;
        pub const FADE_OVER: f32 = 1.047_197_6;
        pub const PROBE: f32 = 250.0;
        pub const SIDE: f32 = 50.0;
        pub const FAN: f32 = 300.0;
        pub const FAN_ANGLE: f32 = 0.523_598_8;
        pub const AVOID_FRAMES: f32 = 40.0;
        pub const ASSIST_STEP: f32 = 0.052_359_88;
        pub const ASSIST_RAMP: f32 = 30.0;
        pub const PITCH_TOP: f32 = 1.483_529_8;
        pub const EYE_FLOOR: f32 = 55.0;
        pub const EYE_FLOOR_NEAR: f32 = 49.5;
        pub const PITCH_BACK: f32 = 0.03;
        pub const STILL_FRAMES: f32 = 60.0;
        pub const LEDGE_AHEAD: f32 = 100.0;
        pub const LEDGE_FROM: f32 = 30.0;
        pub const LEDGE_DOWN: f32 = 500.0;
        pub const AIR_DOWN: f32 = 1000.0;
        pub const FALL_FAST: f32 = 5.0;
        pub const FALL_AHEAD: f32 = 250.0;
        pub const REPLACE_PAST: f32 = 300.0;
        pub const JUMP: f32 = 1000.0;
        pub const SEE_TURN: f32 = 0.261_799_4;
        pub const SEE_UP: f32 = 50.0;
        pub const SQUEEZE: f32 = 40.0;
        pub const SQUEEZE_PITCH: f32 = 1.047_197_6;
        pub const DIST_BACK: f32 = 0.1;
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

        /// The look point's spring; during a mode blend it snaps to the target.
        fn spring_look(&mut self, inp: &Input, w: &mut dyn World) {
            if 0.0 < self.blend.left {
                self.look = inp.target();
                return;
            }
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
            if !inp.can_act || self.blend.left > 0.0 || self.rc.left > 0.0 {
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

        /// One frame of KH1's mode 1, after any entry (`0x1401E1920`): Sora's speeds
        /// (`0x1401E5D90`), the look point's spring, the stick, the recenter's blend; then Auto
        /// (the swing-behind, the wall-avoid, the pitch assist, the eye walk, the visibility
        /// re-place, the auto recenter) or Manual (the pitch assist, the eye's wall ray); R3, the
        /// distance, the yaw from the result.
        pub fn step(&mut self, inp: &Input, w: &mut dyn World) {
            let d = inp.pos - self.prev.unwrap_or(inp.pos);
            self.speed = d.length() / inp.step;
            self.speed_h = d.flat().length() / inp.step;
            self.spring_look(inp, w);
            self.stick(inp);
            self.blend(inp.step);
            let auto = inp.auto && inp.state != 0x1B;
            if auto {
                self.swing_behind(inp, w);
                self.wall_avoid(inp, w);
                self.pitch_assist(inp, w);
                self.auto_eye(inp, w);
                let h = (self.eye - self.look).horizontal_length();
                if h < auto::SQUEEZE && self.pitch < auto::SQUEEZE_PITCH && moving(inp.state, self.speed_h) {
                    self.start_recenter(inp.facing, w);
                }
            } else {
                self.pitch_assist(inp, w);
                self.manual_eye(w);
            }
            if inp.recenter && inp.can_act {
                self.start_recenter(inp.facing, w);
            }
            // Auto: the distance springs back 10 % a frame toward the default; Manual: the default.
            self.dist = if auto {
                let now = (self.eye - self.look).length();
                now + (DIST_DEFAULT - now) * auto::DIST_BACK
            } else {
                DIST_DEFAULT
            };
            self.yaw = yaw_of(self.eye, self.look);
            self.prev = Some(inp.pos);
        }

        /// The swing-behind `0x1401E6FF0`.
        fn swing_behind(&mut self, inp: &Input, w: &mut dyn World) {
            let (r1, r2, rw) = if inp.mouse { (auto::MOUSE_RAMP1, auto::MOUSE_RAMP2, auto::MOUSE_WALL_RAMP) } else { (auto::RAMP1, auto::RAMP2, auto::WALL_RAMP) };
            let sw = &mut self.swing;
            if auto::FACING_RESET <= (sw.facing - inp.facing).abs() || !moving(inp.state, self.speed_h) {
                sw.first = 0.0;
                sw.second = 0.0;
            } else {
                sw.first += inp.step;
                if r1 < sw.first {
                    sw.second += inp.step;
                    sw.first = r1;
                    if r2 < sw.second {
                        sw.second = r2;
                    }
                }
            }
            if 0.0 < self.avoid.left || sw.first <= 0.0 {
                sw.facing = inp.facing;
                return;
            }
            if w.near_wall(self.look, auto::PROBE) || w.near_wall(self.eye, auto::PROBE) {
                let sw = &mut self.swing;
                sw.wall = (sw.wall + inp.step).min(rw);
                if sw.second != 0.0 {
                    sw.second = r2;
                }
            } else {
                let sw = &mut self.swing;
                sw.wall = (sw.wall - inp.step * 0.3).max(0.0);
            }
            let sw = &mut self.swing;
            let delta = wrap_angle(inp.facing + PI - self.yaw);
            let mut g = ((sw.wall / rw) * auto::GAIN + (sw.second / r2) * auto::GAIN) * delta;
            if auto::FADE_FROM <= delta.abs() {
                g *= 1.0 - (delta.abs() - auto::FADE_FROM) / auto::FADE_OVER;
            }
            sw.facing = inp.facing;
            self.yaw += g;
        }

        /// The wall-avoid `0x1401E6C10`: a wall beside the line from the look point to the eye,
        /// and one of two ±30° rays from the eye blocked: swing (40 frames) to run along the wall.
        fn wall_avoid(&mut self, inp: &Input, w: &mut dyn World) {
            let a = &mut self.avoid;
            if 0.0 < a.left {
                self.yaw = ((1.0 - a.left / auto::AVOID_FRAMES) * PI * 0.5).sin() * a.delta + a.from;
                a.left -= inp.step;
                return;
            }
            a.left = 0.0;
            let to_look = self.look - self.eye;
            let back = self.eye - self.look;
            let mut s = back.normalized();
            s.y = 0.0;
            let mut side = s.rotate_yaw(PI / 2.0) * auto::SIDE;
            let mut hit = w.sweep(self.look + side, self.look + side + back, RAY);
            if hit.is_none() {
                side = side * -1.0;
                hit = w.sweep(self.look + side, self.look + side + back, RAY);
            }
            if let Some(at) = hit {
                let normal = w.normal();
                let c = to_look.normalized() * auto::FAN;
                let t1 = w.sweep(self.eye, self.eye + c.rotate_yaw(auto::FAN_ANGLE), RAY).is_some();
                let t2 = w.sweep(self.eye, self.eye + c.rotate_yaw(-auto::FAN_ANGLE), RAY).is_some();
                if t1 && t2 {
                    return;
                }
                if t1 != t2 && moving(inp.state, self.speed_h) {
                    let b = (at - self.eye).flat().normalized();
                    let n = normal.flat().normalized();
                    let mut ang = n.x.atan2(n.z);
                    let cross_y = b.z * n.x - b.x * n.z;
                    ang += if cross_y < 0.0 { -PI / 2.0 } else { PI / 2.0 };
                    let mut d = wrap_angle(ang) - wrap_angle(self.yaw);
                    if PI < d.abs() {
                        d += if 0.0 <= d { -2.0 * PI } else { 2.0 * PI };
                    }
                    self.avoid = Avoid { from: wrap_angle(self.yaw), delta: d, left: auto::AVOID_FRAMES };
                }
            }
            // The right stick turning the camera (KH1's logical 0x1000 / 0x2000 held) cancels it.
            if inp.stick[0] != 0.0 {
                self.avoid.left = 0.0;
            }
        }

        /// The pitch assist `0x1401E6670` (Auto and Manual alike).
        fn pitch_assist(&mut self, inp: &Input, w: &mut dyn World) {
            let slow = !moving(inp.state, self.speed);
            let mut raise = false;
            if slow {
                if inp.auto && inp.state != 0x1B {
                    if self.assist.still <= auto::STILL_FRAMES {
                        self.assist.still += inp.step;
                    } else {
                        let fwd = Vec3::from_yaw(inp.facing);
                        if w.sweep(inp.pos, inp.pos + fwd * auto::LEDGE_AHEAD, RAY).is_none() {
                            let mut from = fwd * auto::LEDGE_FROM + inp.pos;
                            from.y -= 100.0;
                            raise = w.sweep(from, from + Vec3::new(0.0, auto::LEDGE_DOWN, 0.0), RAY).is_none();
                        }
                    }
                }
            } else {
                if inp.state == 2 {
                    if w.sweep(self.look, self.look + Vec3::new(0.0, auto::AIR_DOWN, 0.0), RAY).is_none() {
                        raise = true;
                    } else if auto::FALL_FAST < inp.vel.y {
                        raise = w.sweep(inp.pos, inp.pos + inp.vel.normalized() * auto::FALL_AHEAD, RAY).is_none();
                    }
                }
                self.assist.still = 0.0;
            }
            let skip = matches!(inp.state, 4 | 5 | 8 | 0x15 | 0x18 | 0x1A);
            let a = &mut self.assist;
            if !raise || skip {
                a.drop = 0.0;
            } else {
                self.pitch = wrap_angle(self.pitch + (a.drop / auto::ASSIST_RAMP) * auto::ASSIST_STEP).min(auto::PITCH_TOP);
                a.drop = (a.drop + inp.step).min(auto::ASSIST_RAMP);
            }
            let floor = w.sweep(self.eye, self.eye + Vec3::new(0.0, auto::EYE_FLOOR, 0.0), RAY);
            match floor {
                Some(at) if !skip => {
                    let len = (at - self.eye).length();
                    if len < auto::EYE_FLOOR_NEAR {
                        let a = &mut self.assist;
                        self.pitch = wrap_angle(self.pitch + (1.0 - len / auto::EYE_FLOOR) * (a.floor / auto::ASSIST_RAMP) * auto::ASSIST_STEP).min(auto::PITCH_TOP);
                        a.floor = (a.floor + inp.step).min(auto::ASSIST_RAMP);
                    }
                }
                _ => {
                    let a = &mut self.assist;
                    if !slow {
                        a.floor -= inp.step * 0.3;
                    }
                    if a.floor < 0.0 {
                        a.floor = 0.0;
                    }
                    if a.floor <= 0.0 {
                        self.pitch += wrap_angle(PITCH_DEFAULT - self.pitch) * auto::PITCH_BACK;
                    }
                }
            }
        }

        /// The Auto branch's eye: the goal at the pitch, yaw and distance; re-placed when the
        /// distance has grown past the default + 300; walked there as a 40-unit sphere stopping at
        /// contact (KH1's 20-unit steps as one Re:CoM sweep), or jumped there from 1000 away; then
        /// re-placed when no segment from it reaches Sora (`0x1401E7780`).
        fn auto_eye(&mut self, inp: &Input, w: &mut dyn World) {
            let mut goal = self.look + offset(self.dist, self.pitch, wrap_angle(self.yaw));
            if DIST_DEFAULT + auto::REPLACE_PAST < (goal - self.look).length() {
                // `0x1401E4820` into the goal (the look point and the defaults reset); the walk
                // still starts from the eye.
                let eye = self.eye;
                self.replace(self.yaw, inp, w);
                goal = self.eye;
                self.eye = eye;
            }
            if (goal - self.eye).length() < auto::JUMP {
                self.eye = w.sweep(self.eye, goal, SPHERE).unwrap_or(goal);
            } else {
                self.eye = goal;
            }
            if !self.sees(inp, w) {
                self.replace(self.yaw, inp, w);
            }
        }

        /// `0x1401E7780`: a clear segment from the eye to the point above Sora, to it turned ±15°
        /// about the eye, or to it 50 up or down.
        fn sees(&self, inp: &Input, w: &mut dyn World) -> bool {
            let above = inp.above();
            let v = above - self.eye;
            let targets = [above, self.eye + v.rotate_yaw(auto::SEE_TURN), self.eye + v.rotate_yaw(-auto::SEE_TURN), above + Vec3::new(0.0, auto::SEE_UP, 0.0), above - Vec3::new(0.0, auto::SEE_UP, 0.0)];
            targets.iter().any(|&t| w.sweep(self.eye, t, RAY).is_none())
        }
    }

    impl Follow {
        /// KH1's mode switch `0x1401E5360` with a blend of `frames`: from the drawn view (the
        /// current blend's if one runs, else the mode's own eye / look).
        pub fn switch_blend(&mut self, frames: f32) {
            if self.blend.left <= 0.0 {
                self.blend.from_eye = self.eye;
                self.blend.from_look = self.look;
            } else {
                let (e, l) = self.view_now();
                self.blend.from_eye = e;
                self.blend.from_look = l;
            }
            self.blend.left = frames;
            self.blend.total = frames;
        }

        fn view_now(&self) -> (Vec3, Vec3) {
            let b = &self.blend;
            if b.left <= 0.0 || b.total <= 0.0 {
                return (self.eye, self.look);
            }
            let f = ((b.left / b.total) * PI * 0.5).cos();
            let mix = |from: Vec3, to: Vec3| from * (1.0 - f) + to * f;
            (mix(b.from_eye, self.eye), mix(b.from_look, self.look))
        }

        /// The task's end of frame (`0x1401E5D90`): the blend steps and the drawn eye / look.
        pub fn view(&mut self, step: f32) -> (Vec3, Vec3) {
            if 0.0 < self.blend.left {
                self.blend.left -= step;
                if 0.0 < self.blend.left {
                    return self.view_now();
                }
                self.blend.left = 0.0;
            }
            (self.eye, self.look)
        }

        /// KH1's lock-on start `0x1401E41A0`: in the lock camera already, the look point blends from
        /// where it was over 60 frames (a new target); else the switch to it with a 60 blend.
        pub fn lock_on(&mut self, already: bool) {
            self.lock.old_look = self.look;
            if already {
                self.lock.relock = lockcam::RELOCK;
            } else {
                self.switch_blend(lockcam::ENTER_BLEND);
            }
        }

        /// KH1's unlock `0x1401E4110`: the follow with a 45 blend, its entry at the drawn view's yaw.
        pub fn unlock(&mut self, inp: &Input, w: &mut dyn World) {
            self.switch_blend(lockcam::EXIT_BLEND);
            let (e, l) = self.view_now();
            self.enter(yaw_of(e, l), inp, w);
        }

        /// One frame of KH1's lock camera, mode 3 (`0x1401E2650`).
        pub fn lock_step(&mut self, inp: &Input, t: &Target, w: &mut dyn World) {
            use lockcam::*;
            let mut anchor = inp.pos + (ANCHOR + Vec3::new(0.0, self.lock.height, 0.0)).rotate_yaw(self.yaw);
            let mut v = t.point - anchor;
            let dy = v.y;
            v.y = 0.0;
            let (n, _) = v.normalize_len();
            let mut v = n * (t.radius + PAST);
            v.y = dy * -0.5;
            let past = t.point + v;
            let wv = past - anchor;
            let hh = wv.horizontal_length();
            let a = wv.y.abs();
            if hh < a + a {
                let mut y = wv.y;
                if a < hh {
                    y *= ((a + a) - hh) / a;
                }
                anchor.y += y * 0.5;
            }
            // The eye closes on the anchor: 90 % of the horizontal gap past 75, 90 % vertically.
            let mut d = anchor - self.eye;
            let dyy = d.y;
            d.y = 0.0;
            let (n, len) = d.normalize_len();
            let len = len - DEAD;
            if 0.0 < len {
                self.eye += n * (len * CLOSE);
            }
            self.eye.y += dyy * CLOSE;
            // A ray from Sora's body point to the eye.
            let body = inp.pos + Vec3::new(0.0, -BODY, 0.0);
            let blocked = w.sweep(body, self.eye, RAY);
            if let Some(h) = blocked {
                self.eye = h;
            }
            // The yaw: half the error outside a ±5° band about the line to a point ahead of
            // Sora, at most 10° a frame.
            let ahead = inp.pos + AHEAD.rotate_yaw(self.yaw);
            let a1 = wrap_angle(self.yaw - yaw_of(self.eye, ahead));
            let mut a2 = wrap_angle(yaw_of(self.eye, past) - self.yaw);
            if a1 + BAND < a2 {
                a2 -= a1 + BAND;
            } else if a2 < a1 - BAND {
                a2 += BAND - a1;
            } else {
                a2 = 0.0;
            }
            let turn = (a2 * 0.5).max(-TURN_MAX).min(TURN_MAX);
            self.yaw = wrap_angle(turn + self.yaw);
            let to = past - self.eye;
            self.look = self.eye + Vec3::new(0.0, to.y, -to.horizontal_length()).rotate_yaw(self.yaw);
            self.yaw = yaw_of(self.eye, self.look);
            // Keep Sora on screen: his feet outside the half FOV raise a counter (to 40) that pulls
            // the look point up to 80 % toward his body point; it falls 2 a frame inside 80 %.
            let half = FOV * PI / 180.0 * 0.5;
            let ang = angle_between(self.look - self.eye, inp.pos - self.eye);
            let pull = if ang.abs() <= half {
                if ang.abs() < half * 0.8 {
                    self.lock.screen -= inp.step * 2.0;
                    if self.lock.screen < 0.0 {
                        self.lock.screen = 0.0;
                    }
                }
                0.0 < self.lock.screen
            } else {
                self.lock.screen = (self.lock.screen + inp.step).min(SCREEN_MAX);
                true
            };
            if pull && 0.0 < self.lock.screen {
                let k = (self.lock.screen / SCREEN_MAX) * SCREEN_PULL;
                self.look = body * k + self.look * (1.0 - k);
            }
            if 0.0 < self.lock.relock {
                self.lock.relock -= inp.step;
                if 0.0 <= self.lock.relock {
                    let c = ((self.lock.relock / RELOCK) * PI * 0.5).cos();
                    self.look = self.look * c + self.lock.old_look * (1.0 - c);
                } else {
                    self.lock.relock = 0.0;
                }
            }
            let skip = matches!(inp.state, 4 | 5 | 8 | 0x15 | 0x18 | 0x1A);
            if !skip && blocked.is_none() {
                // `0x1401E5A20`: the floor under the eye (a ray from 10 above it, 10000 down).
                let from = self.eye + Vec3::new(0.0, -10.0, 0.0);
                let floor = w.sweep(from, self.eye + Vec3::new(0.0, FLOOR_PROBE, 0.0), RAY).unwrap_or(self.eye + Vec3::new(0.0, FLOOR_PROBE, 0.0));
                let over = inp.pos.y - floor.y;
                self.lock.height = if over <= 30.0 {
                    let up = t.point.y - self.eye.y;
                    if -200.0 <= up {
                        0.0
                    } else {
                        ((up.max(-400.0) + 200.0).abs() / 200.0) * 20.0
                    }
                } else {
                    -((over.min(150.0) - 30.0).abs() / 120.0) * 170.0
                };
            }
            self.prev = Some(inp.pos);
        }
    }

    /// The angle between two vectors (`0x1402A91F0`: acos of the normalized dot, 0 / π past ±1).
    pub fn angle_between(a: Vec3, b: Vec3) -> f32 {
        let (la, lb) = (a.length(), b.length());
        if la == 0.0 || lb == 0.0 {
            return 0.0;
        }
        let c = a.dot(b) / (la * lb);
        if 1.0 < c {
            0.0
        } else if c < -1.0 {
            PI
        } else {
            c.acos()
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

/// KH2 FM's field camera (Steam `KINGDOM HEARTS II FINAL MIX.exe`; its camera object
/// `0x140718C60`, per-frame update `0x140166100`): mode 0 the follow (`0x140165110`), mode 2 the
/// lock-on (`0x140176C70`), then the view step (`0x140165700`: the springs and the wall test).
/// Every number from the KH2 Steam decomp / exe (tags on each). KH2 units are KH1's (centimetres,
/// y down, yaw `atan2(x, z)`); the owner's answer Q9 (b) scales KH2's lengths ×0.89 for KH1's Sora
/// ([`SCALE`]): the step runs in KH2's own numbers on a world shrunk by 1 / 0.89, so **every
/// length below is applied × 0.89** (angles, rates and screen margins as they are).
pub mod kh2 {
    use super::kh1::World;
    use super::*;

    /// The owner's ×0.89 (`design-philosophy-instructions.md` § Camera answers, Q9 (b)): KH1 Sora's
    /// size against KH2 Sora's (hips 78.9 / 88.5, KH2-Combat-in-KH1's `camera_scale`).
    pub const SCALE: f32 = 0.89;

    /// Row 0 of KH2's parameter rows (`0x140718C00`, 6 floats; loader `0x140165A70`: `+0xE0` FOV,
    /// `+0xCC` default distance, `+0xD0` lock distance, `+0xD4` minimum, `+0xD8` maximum, `+0xDC`
    /// height coefficient). Checked (exe + decomp). × 0.89: 373.8 / 338.2 / 222.5 / 445 units;
    /// the coefficient ÷ 0.89 (it multiplies a squared length into a length).
    pub const FOV: f32 = 1.5;
    pub const DIST_DEFAULT: f32 = 420.0;
    pub const DIST_LOCK: f32 = 380.0;
    pub const DIST_MIN: f32 = 250.0;
    pub const DIST_MAX: f32 = 500.0;
    pub const HEIGHT_K: f32 = 0.008;
    /// The look point 170 above the feet (`0x140165110`, `0x1401644E0`, `0x140164FA0`, the lock's
    /// chest `0x140176EC0`); the eye 150 + the curve (`0x140164520`: eye.y = look.y + 150 −
    /// (d − min)² · k). Checked (decomp). × 0.89: 151.3 / 133.5.
    pub const LOOK_UP: f32 = 170.0;
    pub const EYE_BASE: f32 = 150.0;
    /// The yaw turns toward its goal at most 10° a frame × the step (`0x140718BE4`). Checked (exe).
    pub const TURN_MAX: f32 = 0.174_532_92;
    /// The right stick: 3° a frame × its tilt (`0x140718BF0`, written to `cam+0` each frame by
    /// `0x140166100`; the lock's `0x14071A470` is also 3°), zoom 8 a frame × its tilt × `cam+4`
    /// (1 on the pad) (`0x1401654D0`, `0x140176BC0`). Checked (exe + decomp). × 0.89: zoom 7.12.
    pub const STICK_TURN: f32 = 0.052_359_88;
    pub const LOCK_TURN: f32 = 0.052_359_88;
    pub const ZOOM: f32 = 8.0;
    /// The swing-behind (`0x140165110`): turns toward Sora's back by at most
    /// `2 (|lx| + 1)(|ly| + 1) · tilt · ((run − 8) · 0.00017453 + 0.0043633) · step` a frame
    /// (`0x140718BEC` / `0x140718BE8`), the distance back toward 420 at 0.5 a frame. Checked (exe +
    /// decomp). × 0.89: 0.445 a frame.
    pub const SWING_K0: f32 = 0.004_363_323;
    pub const SWING_K1: f32 = 0.000_174_532_92;
    pub const DRIFT: f32 = 0.5;
    /// KH2 Sora's `+0x12C` (the run speed the camera reads). Assumption: 8 in his base form
    /// (KH2-Combat-in-KH1's notes; the writer was not found in the decomp): the run term is 0.
    pub const RUN_SPEED: f32 = 8.0;
    /// The swing needs the left stick not pulled back past 0.9 (`+0x34 < 0.9`). Checked (decomp).
    pub const BACK: f32 = 0.9;
    /// The view's springs (`0x140165700` → `0x140165BD0(cur, goal, last goal, rate, 2.5, 0.5,
    /// 1.0)`): rate 0.07 + (run − 8) · 0.0006 (mouse 0.4 / 0.0018), halved below a frame step of
    /// 2; the look point's rate + 0.03. Checked (decomp). × 0.89: the 1.0 rest gap 0.89.
    pub const RATE: f32 = 0.07;
    pub const RATE_RUN: f32 = 0.0006;
    pub const RATE_MOUSE: f32 = 0.4;
    pub const RATE_RUN_MOUSE: f32 = 0.0018;
    pub const RATE_LOOK: f32 = 0.03;
    pub const SPRING_GAP: f32 = 2.5;
    pub const SPRING_VEL: f32 = 0.5;
    pub const SPRING_REST: f32 = 1.0;
    /// The wall test `0x140164940`: a ray from the look point to the eye; the eye at the hit + 5
    /// along the polygon's normal. Checked (decomp). × 0.89: 4.45.
    pub const WALL_PUSH: f32 = 5.0;
    /// The lock camera (`0x140176EC0` with 0.3 from `0x140176C70`): the look point 30 % of the
    /// way from Sora's chest to the target; Sora's screen point 100 above his feet; the look stays
    /// at the chest when Sora's point is over 300 from it and the look is steeply above (> 0.5 of
    /// the unit vector); the distance to 380 at 1 a frame, − 2 a frame for each of the two at the
    /// bottom edge; ±20° (`0.34906584`) when one leaves the screen sideways. Checked (decomp + exe).
    /// × 0.89: 89 / 267 / 0.89 / 1.78.
    pub const LOCK_FRACTION: f32 = 0.3;
    pub const LOCK_SORA: f32 = 100.0;
    pub const LOCK_FAR: f32 = 300.0;
    pub const LOCK_STEEP: f32 = 0.5;
    pub const LOCK_DRIFT: f32 = 1.0;
    pub const LOCK_SHRINK: f32 = 2.0;
    pub const OVERSHOOT: f32 = 0.349_065_84;
    /// The screen test `0x140176710` on KH2's 512 × 416 frame (`0x140166A20`: GS offsets 1792 /
    /// 1840) with the margins `0x14071A478` = (32, 480, 8, 408) (ints). Checked (exe + decomp).
    pub const SCREEN: [f32; 4] = [32.0, 480.0, 8.0, 408.0];
    pub const FRAME: [f32; 2] = [512.0, 416.0];
    /// KH2's sticks (`0x14039C180`): each axis's byte − 127.5 less a dead zone of 60
    /// (`0x140BF3338` = 60.0, set by `0x14039B8B0`), over 127.5 − 60; a direction bit while past
    /// it; the tilt the length of the two, at most 1. The mouse: a dead zone of 1. Checked (decomp).
    pub const DEAD_ZONE: f32 = 60.0;
    pub const MOUSE_DEAD_ZONE: f32 = 1.0;

    /// KH2's projection (`0x14019DDE0` with the view's 0.8 / 0.8667 `0x14019D050`): a horizontal
    /// FOV on a 640 × 480 frame, focal 320 / tan(fov / 2). Re:CoM's FOV setter `0x14024FCB0` gives
    /// focal 256 · cot(fov / 2) on its own 640 × 480 (`0x14023E720` with 512 / 0.8, 384 / 0.8,
    /// `0x140821ADC` = 384). Checked (decomp + exe both). The same focal: Re:CoM's record FOV =
    /// 2 atan(0.8 tan(0.75)) = 73.39° (69.87° vertical in both). That both PC ports widen the frame
    /// the same way: Assumption (as KH1's 53, plan § 5).
    pub fn recom_fov_degrees() -> f32 {
        (2.0 * (0.8 * (FOV * 0.5).tan()).atan()).to_degrees()
    }

    /// One KH2 stick (`pad+0x20..0x2C` right, `+0x30..0x3C` left): the axes past the dead zone,
    /// the tilt, and the direction bits (x: left / right, y: up / down).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Stick {
        pub x: f32,
        pub y: f32,
        pub tilt: f32,
        pub left: bool,
        pub right: bool,
        pub up: bool,
        pub down: bool,
    }

    impl Stick {
        /// `0x14039C180` on two raw bytes (y down positive).
        pub fn from_bytes(bx: u8, by: u8, dz: f32) -> Stick {
            let axis = |b: u8| -> (f32, bool, bool) {
                let v = b as f32 - 127.5;
                if v <= 0.0 {
                    let a = v + dz;
                    if a <= 0.0 {
                        (a, true, false)
                    } else {
                        (0.0, false, false)
                    }
                } else {
                    let a = v - dz;
                    if 0.0 <= a {
                        (a, false, true)
                    } else {
                        (0.0, false, false)
                    }
                }
            };
            let (x, l, r) = axis(bx);
            let (y, u, d) = axis(by);
            let (x, y) = (x / (127.5 - dz), y / (127.5 - dz));
            Stick { x, y, tilt: (x * x + y * y).sqrt().min(1.0), left: l, right: r, up: u, down: d }
        }

        /// Re:CoM's stick ints (±4096) as KH2's: the raw bytes rebuilt
        /// (`units::byte_from_recom_axis`), then KH2's dead zone (the mouse's 1 with keyboard and
        /// mouse: Assumption, Re:CoM's PC port feeds the mouse into the right stick, plan § 13).
        pub fn from_recom(x: i32, y: i32, mouse: bool) -> Stick {
            Stick::from_bytes(units::byte_from_recom_axis(x), units::byte_from_recom_axis(y), if mouse { MOUSE_DEAD_ZONE } else { DEAD_ZONE })
        }
    }

    /// One frame's input, in KH1 units (converted to KH2's by [`SCALE`] inside).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Input {
        /// Sora's feet and facing (KH1's yaw convention, which is KH2's: behind = facing + π,
        /// `0x140164E90`).
        pub pos: Vec3,
        pub facing: f32,
        pub right: Stick,
        pub left: Stick,
        /// Keyboard and mouse (KH2's mouse mode `0x142B44A88`; Re:CoM's `0x140C2FF64`): the
        /// springs' 0.4 rate.
        pub mouse: bool,
        /// Sora can act (KH2: the actor's vtable `+0x78`; Re:CoM: the camera's input lock clear).
        pub can_act: bool,
        /// The reset button pressed (KH2's logical `0x20000`, raw R3 `0x4` through the pad table
        /// `0x1405C34F0`; Re:CoM's reset bit 0x400, plan § 2.4).
        pub reset: bool,
        /// KH2's option bits 7 / 8 (`0x1402E5330` / `0x1402E5350`): left / right and up / down
        /// swapped. From Re:CoM's inversion rows with KH2's own Normal (owner Q5 (a)).
        pub swap_h: bool,
        pub swap_v: bool,
        pub step: f32,
        /// KH2's auto camera (`cam+0x42`; its Manual option bit 4, `0x1402E5310`): the Auto
        /// camera row.
        pub auto: bool,
    }

    impl Input {
        fn feet(&self) -> Vec3 {
            self.pos * (1.0 / SCALE)
        }
        fn look_goal(&self) -> Vec3 {
            self.feet() + Vec3::new(0.0, -LOOK_UP, 0.0)
        }
        fn behind(&self) -> f32 {
            wrap_angle(self.facing + PI)
        }
        /// `0x140178AB0` / `0x140178B10` / `0x140178B50` / `0x140178B80` with the option swaps:
        /// turn +, turn −, zoom in, zoom out.
        fn turn_plus(&self) -> bool {
            if self.swap_h {
                self.right.left
            } else {
                self.right.right
            }
        }
        fn turn_minus(&self) -> bool {
            if self.swap_h {
                self.right.right
            } else {
                self.right.left
            }
        }
        fn zoom_in(&self) -> bool {
            if self.swap_v {
                self.right.down
            } else {
                self.right.up
            }
        }
        fn zoom_out(&self) -> bool {
            if self.swap_v {
                self.right.up
            } else {
                self.right.down
            }
        }
    }

    /// KH1's world (Re:CoM's sweep) seen in KH2's units.
    struct Scaled<'a>(&'a mut dyn World);
    impl Scaled<'_> {
        fn sweep(&mut self, from: Vec3, to: Vec3) -> Option<Vec3> {
            self.0.sweep(from * SCALE, to * SCALE, kh1::RAY).map(|h| h * (1.0 / SCALE))
        }
        fn normal(&mut self) -> Vec3 {
            self.0.normal()
        }
    }

    /// `0x140165BD0`: `cur` moved along (cur − goal) by
    /// `((v · w) / |v| · 0.5 + (rest − |v|) · 2.5) · rate`, `w` = (last goal − goal) · rate.
    pub fn spring(cur: Vec3, goal: Vec3, last: Vec3, rate: f32, rest: f32) -> Vec3 {
        let v = cur - goal;
        let w = (last - goal) * rate;
        let (n, len) = v.normalize_len();
        if len == 0.0 {
            return cur;
        }
        cur + n * (((v.dot(w) / len) * SPRING_VEL + (rest - len) * SPRING_GAP) * rate)
    }

    /// `0x1403B74C0`: `from` turned toward `to` by at most `max`.
    fn turn(from: f32, to: f32, max: f32) -> f32 {
        wrap_angle(from + kh1_core::math::clamp_sym(wrap_angle(to - from), max))
    }

    /// The lock target: Re:CoM's lock-on point (KH2's `0x1403BDBE0`, the target's camera shape),
    /// in KH1 units.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Target {
        pub point: Vec3,
    }

    /// KH2's camera state (`cam+0x08` drawn look, `+0x18` drawn eye, `+0x58` distance, `+0x5C`
    /// yaw, `+0x60` goal yaw, `+0x64` eye goal, `+0x74` last eye goal, `+0x84` look goal, `+0x94`
    /// last look goal; `+0x42` auto), in KH2 units.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Cam {
        pub look: Vec3,
        pub eye: Vec3,
        pub dist: f32,
        pub yaw: f32,
        pub goal: f32,
        pub eye_goal: Vec3,
        pub last_eye_goal: Vec3,
        pub look_goal: Vec3,
        pub last_look_goal: Vec3,
        pub auto: bool,
        /// Sora's feet last frame (`0x1403B8940`: he moved when x or z differs).
        pub prev: Option<Vec3>,
    }

    impl Default for Cam {
        fn default() -> Self {
            Cam { look: Vec3::ZERO, eye: Vec3::ZERO, dist: DIST_DEFAULT, yaw: 0.0, goal: 0.0, eye_goal: Vec3::ZERO, last_eye_goal: Vec3::ZERO, look_goal: Vec3::ZERO, last_look_goal: Vec3::ZERO, auto: true, prev: None }
        }
    }

    impl Cam {
        /// `0x140164520`: the yaw to its goal (at once when `hard`, else ≤ 10° a frame), the eye goal
        /// at the distance behind `from`, its height on the curve; `hard` snaps the view.
        fn place(&mut self, from: Vec3, hard: bool, step: f32) {
            self.yaw = if hard { self.goal } else { turn(self.yaw, self.goal, step * TURN_MAX) };
            let mut e = from + Vec3::new(0.0, 0.0, self.dist).rotate_yaw(self.yaw);
            let c = self.dist - DIST_MIN;
            e.y = from.y + EYE_BASE - c * c * HEIGHT_K;
            self.eye_goal = e;
            if hard {
                self.snap();
            }
        }

        /// `0x140164D70`: the drawn view and the last goals to the goals.
        fn snap(&mut self) {
            self.last_eye_goal = self.eye_goal;
            self.eye = self.eye_goal;
            self.last_look_goal = self.look_goal;
            self.look = self.look_goal;
        }

        /// `0x140164940` (its ray `0x140171210`): the eye at the hit + 5 along the normal. KH2's
        /// camera volumes (`0x140167A50`) and its attribute-5 polygons are map data Re:CoM has none
        /// of (not carried).
        fn wall(from: Vec3, eye: &mut Vec3, w: &mut Scaled) -> bool {
            match w.sweep(from, *eye) {
                Some(hit) => {
                    *eye = hit + w.normal() * WALL_PUSH;
                    true
                }
                None => false,
            }
        }

        /// KH2's re-place `0x140164FA0`: the yaw and its goal behind Sora, the look goal above his
        /// feet, the eye goal at the distance, the view snapped. Its room start
        /// (`0x140164C60` → the loader's distance 420, then `0x140164CB0`): `room`.
        pub fn behind(&mut self, inp: &Input, room: bool) {
            if room {
                self.dist = DIST_DEFAULT;
            }
            self.goal = inp.behind();
            self.yaw = self.goal;
            self.look_goal = inp.look_goal();
            self.place(self.look_goal, false, inp.step);
            self.snap();
            self.prev = Some(inp.feet());
        }

        /// Our mode set by Re:CoM's setters (a sleight's restore, a switch from Re:CoM's camera):
        /// KH2 has no entry of its own (its mode number changes, its state carries on); here the
        /// view carries on from Re:CoM's drawn one (KH1 units), the yaw and its goal from it.
        pub fn carry_on(&mut self, eye: Vec3, look: Vec3) {
            self.eye = eye * (1.0 / SCALE);
            self.look = look * (1.0 / SCALE);
            self.yaw = kh1::yaw_of(self.eye, self.look);
            self.goal = self.yaw;
            self.eye_goal = self.eye;
            self.look_goal = self.look;
            self.last_eye_goal = self.eye;
            self.last_look_goal = self.look;
        }

        /// The right stick's turn, as both modes do: yaw and goal ± tilt · speed · step.
        fn stick_turn(&mut self, inp: &Input, speed: f32) {
            let d = if inp.turn_plus() {
                1.0
            } else if inp.turn_minus() {
                -1.0
            } else {
                return;
            };
            let y = wrap_angle(self.yaw + d * inp.right.tilt * speed * inp.step);
            self.yaw = y;
            self.goal = y;
        }

        /// The follow's zoom `0x1401654D0`: ± tilt · 8 · step, clamped to the range.
        fn zoom(&mut self, inp: &Input) {
            if inp.zoom_in() {
                self.dist = (self.dist - inp.right.tilt * ZOOM * inp.step).max(DIST_MIN);
            } else if inp.zoom_out() {
                self.dist = (self.dist + inp.right.tilt * ZOOM * inp.step).min(DIST_MAX);
            }
        }

        /// The distance back toward `to` by `rate` (`0x140165110`'s drift, `0x140164290`): only
        /// when 1 or more away; clamped to the range.
        fn drift(&mut self, to: f32, rate: f32) {
            if 1.0 <= (self.dist - to).abs() {
                self.dist = if self.dist <= to { (self.dist + rate).min(DIST_MAX) } else { (self.dist - rate).max(DIST_MIN) };
            }
        }

        /// KH2's follow, mode 0 (`0x140165110`).
        fn follow(&mut self, inp: &Input, w: &mut Scaled) {
            let mut hard = false;
            if inp.can_act {
                // Select's first person (logical 0x400000 with `0x1403BA300`): not carried (scope).
                if inp.reset {
                    // Logical 0x20000: the goal behind Sora, the default distance, the drawn eye to
                    // the eye goal; a cut when the left stick is held.
                    self.goal = inp.behind();
                    self.dist = DIST_DEFAULT;
                    self.eye = self.eye_goal;
                    hard = inp.left.tilt != 0.0;
                } else {
                    self.stick_turn(inp, STICK_TURN);
                }
                // The zoom (KH2's no-zoom area flag `0x1403BABF0` is map data: not carried).
                self.zoom(inp);
                // KH2's R2 held swaps Auto / Manual (`0x140178AE0`): not carried (owner question).
                self.auto = inp.auto;
            }
            let feet = inp.feet();
            let moved = self.prev.map_or(false, |p| p.x != feet.x || p.z != feet.z);
            if !hard && self.auto && moved && inp.left.tilt != 0.0 && inp.right.tilt == 0.0 && inp.left.y < BACK {
                let k = 2.0 * (inp.left.x.abs() + 1.0) * (inp.left.y.abs() + 1.0) * inp.left.tilt * ((RUN_SPEED - 8.0) * SWING_K1 + SWING_K0) * inp.step;
                let y = turn(self.yaw, inp.behind(), k);
                self.yaw = y;
                self.goal = y;
                self.drift(DIST_DEFAULT, inp.step * DRIFT);
            }
            self.look_goal = inp.look_goal();
            self.place(self.look_goal, hard, inp.step);
            let (from, mut eye) = (self.look_goal, self.eye_goal);
            Self::wall(from, &mut eye, w);
            self.eye_goal = eye;
        }

        /// KH2's lock-on, mode 2 (`0x140176C70` with `0x140176EC0(cam, point, 0.3)`).
        fn lock(&mut self, inp: &Input, t: &Target, w: &mut Scaled) {
            let feet = inp.feet();
            let chest = inp.look_goal();
            let point = t.point * (1.0 / SCALE);
            let mut look = chest + (point - chest) * LOCK_FRACTION;
            let line_from = self.look_goal;
            let sora = feet + Vec3::new(0.0, -LOCK_SORA, 0.0);
            let u1 = screen(self.eye, self.look, point);
            let u2 = screen(self.eye, self.look, sora);
            let b1 = u1 & 3 == 2;
            let b2 = u2 & 3 == 2;
            if b1 {
                self.dist -= inp.step * LOCK_SHRINK;
            }
            if b2 {
                self.dist -= inp.step * LOCK_SHRINK;
            }
            self.dist = self.dist.max(DIST_MIN).min(DIST_MAX);
            let v = sora - look;
            let n = v.normalized();
            if v.length() <= LOCK_FAR || n.y <= LOCK_STEEP {
                if !b1 && !b2 {
                    self.drift(DIST_LOCK, inp.step * LOCK_DRIFT);
                }
                for u in [u1, u2] {
                    if u != 0 {
                        let o = if u & 4 != 0 {
                            OVERSHOOT
                        } else if u & 8 != 0 {
                            -OVERSHOOT
                        } else {
                            0.0
                        };
                        self.goal = wrap_angle(line_yaw(line_from, feet) + o);
                        break;
                    }
                }
            } else {
                look = chest;
            }
            self.look_goal = look;
            if inp.can_act {
                self.stick_turn(inp, LOCK_TURN);
                // The lock's zoom `0x140176BC0` (the follow's with no `cam+4`, 1 here), clamped.
                self.zoom(inp);
                if inp.reset {
                    self.goal = line_yaw(line_from, feet);
                    self.dist = DIST_LOCK;
                }
            }
            // The eye from the chest (`0x140164520(cam, chest, 0)`), its wall test from the chest.
            self.place(chest, false, inp.step);
            let mut eye = self.eye_goal;
            Self::wall(chest, &mut eye, w);
            self.eye_goal = eye;
        }

        /// The view step `0x140165700`: the springs, then mode 0's wall test on the drawn pair
        /// (a hit also becomes the eye goal) or mode 2's from Sora's chest (`0x140176E40`: the
        /// drawn eye only).
        fn view(&mut self, inp: &Input, lock: bool, w: &mut Scaled) {
            let (mut rate, k) = if inp.mouse { (RATE_MOUSE, RATE_RUN_MOUSE) } else { (RATE, RATE_RUN) };
            rate += (RUN_SPEED - 8.0) * k;
            if inp.step < 2.0 {
                rate *= 0.5;
            }
            self.look = spring(self.look, self.look_goal, self.last_look_goal, rate + RATE_LOOK, SPRING_REST);
            self.eye = spring(self.eye, self.eye_goal, self.last_eye_goal, rate, SPRING_REST);
            let mut eye = self.eye;
            if lock {
                Self::wall(inp.look_goal(), &mut eye, w);
                self.eye = eye;
            } else if Self::wall(self.look, &mut eye, w) {
                self.eye = eye;
                self.eye_goal = eye;
            }
        }

        /// One frame (`0x140166100`): the last goals, the mode's step (mode 2, the lock, with a
        /// target; mode 0 the follow), the view. Returns the drawn eye and look in KH1 units.
        pub fn step(&mut self, inp: &Input, target: Option<&Target>, w: &mut dyn World) -> (Vec3, Vec3) {
            let mut w = Scaled(w);
            self.last_look_goal = self.look_goal;
            self.last_eye_goal = self.eye_goal;
            match target {
                Some(t) => self.lock(inp, t, &mut w),
                None => self.follow(inp, &mut w),
            }
            self.view(inp, target.is_some(), &mut w);
            self.prev = Some(inp.feet());
            (self.eye * SCALE, self.look * SCALE)
        }
    }

    /// `0x140176A70`: the yaw of the line from the last look goal through Sora's feet.
    fn line_yaw(from: Vec3, feet: Vec3) -> f32 {
        let v = feet - from;
        v.x.atan2(v.z)
    }

    /// `0x140176710`: where `p` falls on KH2's 512 × 416 frame seen from `eye` toward `look`:
    /// bit 4 left of 32, 8 right of 480, 1 above 8, 2 below 408, 0x10 at or behind the eye.
    /// The axes are Re:CoM's look-at (`0x14024FA40`: x = up × forward, Checked (decomp)) on the
    /// points turned into Re:CoM's frame (KH2's handedness is KH1's: Assumption); the depth bit
    /// for a depth under 1 (KH2's near plane, Assumption: its projected z < 1).
    pub fn screen(eye: Vec3, look: Vec3, p: Vec3) -> u32 {
        let r = |v: Vec3| Vec3::new(v.x, -v.y, -v.z);
        let f = r(look - eye).normalized();
        let up = Vec3::new(0.0, 1.0, 0.0);
        let x = Vec3::new(up.y * f.z - up.z * f.y, up.z * f.x - up.x * f.z, up.x * f.y - up.y * f.x).normalized();
        let y = Vec3::new(f.y * x.z - f.z * x.y, f.z * x.x - f.x * x.z, f.x * x.y - f.y * x.x);
        let d = r(p - eye);
        let (xc, yc, zc) = (d.dot(x), d.dot(y), d.dot(f));
        let (px, py) = if zc != 0.0 { (xc / zc, yc / zc) } else { (xc, yc) };
        let t = (FOV * 0.5).tan();
        let sx = FRAME[0] * 0.5 + FRAME[0] * 0.5 * px / t;
        let sy = FRAME[1] * 0.5 - FRAME[1] * 0.5 * py / (t * 0.75);
        let mut u = 0;
        if sx < SCREEN[0] {
            u |= 4;
        }
        if SCREEN[1] < sx {
            u |= 8;
        }
        if sy < SCREEN[2] {
            u |= 1;
        }
        if SCREEN[3] < sy {
            u |= 2;
        }
        if zc < 1.0 {
            u |= 0x10;
        }
        u
    }

    /// KH2's option swaps from Re:CoM's camera inversion fields (`+0x2F0` / `+0x2F4`): KH2's
    /// Normal turns + with the stick right and zooms in with it up, as KH1's Normal does
    /// ([`kh1::inversion`] (+1, −1)); each game's own Normal (owner Q5 (a)).
    pub fn swaps(recom_h: f32, recom_v: f32) -> (bool, bool) {
        let [h, v] = kh1::inversion(recom_h, recom_v);
        (h < 0.0, 0.0 < v)
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
        Input { pos: Vec3::ZERO, facing: 0.0, stick: [0.0; 2], mouse: false, can_act: true, recenter: false, inv: [1.0, -1.0], step: 1.0, auto: false, state: 0, vel: Vec3::ZERO }
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
        assert_eq!(dispatch(1, Family::Recom, false, false, false, Some(false)), Dispatch::Recom);
        assert_eq!(dispatch(1, Family::Kh1, false, false, false, Some(false)), Dispatch::Switch(6));
        assert_eq!(dispatch(4, Family::Kh1, true, false, false, Some(false)), Dispatch::Switch(6));
        assert_eq!(dispatch(3, Family::Kh1, true, true, false, Some(false)), Dispatch::Recom, "a sleight's mode stays");
        assert_eq!(dispatch(5, Family::Kh1, true, false, false, Some(false)), Dispatch::Recom);
        assert_eq!(dispatch(2, Family::Kh1, false, false, false, Some(false)), Dispatch::Recom, "Re:CoM's timed view");
        assert_eq!(dispatch(3, Family::Kh1, true, false, true, Some(false)), Dispatch::Recom, "a subject override");
        assert_eq!(dispatch(6, Family::Kh1, true, false, false, Some(false)), Dispatch::Ours);
        assert_eq!(dispatch(6, Family::Recom, true, false, false, Some(false)), Dispatch::Switch(3));
        assert_eq!(dispatch(6, Family::Recom, false, false, false, Some(false)), Dispatch::Switch(1));
        // The lock switches 6 ↔ 7 (field too); a subject object holds the mode.
        assert_eq!(dispatch(6, Family::Kh1, false, false, false, Some(true)), Dispatch::Switch(7));
        assert_eq!(dispatch(7, Family::Kh1, true, false, false, Some(false)), Dispatch::Switch(6));
        assert_eq!(dispatch(7, Family::Kh1, true, false, false, Some(true)), Dispatch::Ours);
        assert_eq!(dispatch(6, Family::Kh1, true, false, false, None), Dispatch::Ours);
        assert_eq!(dispatch(7, Family::Recom, true, false, false, Some(true)), Dispatch::Switch(3));
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
        // The stick's clamp, then the pitch assist's 3 % back toward 10° the same frame
        // (`0x1401E6670`, Manual too): KH1 rests at 48.8° holding the stick down.
        let back = |p: f32| p + (PITCH_DEFAULT - p) * auto::PITCH_BACK;
        assert!((g.pitch - back(PITCH_MAX)).abs() < 1e-5, "{}", g.pitch.to_degrees());
        for _ in 0..400 {
            g.step(&Input { stick: [0.0, -1.0], ..i }, &mut Open);
        }
        assert!((g.pitch - back(PITCH_MIN)).abs() < 1e-5);
        // Released: back toward 10° at 3 % a frame.
        let p0 = g.pitch;
        for _ in 0..100 {
            g.step(&i, &mut Open);
        }
        assert!((g.pitch - PITCH_DEFAULT).abs() < (p0 - PITCH_DEFAULT).abs() * 0.1);
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

    /// Sora running along +x at KH1's run speed (8 a frame), the camera 90° off his back.
    fn running(f: &mut Follow, w: &mut dyn World, frames: usize, x0: f32) -> Vec<f32> {
        let mut yaws = Vec::new();
        for k in 0..frames {
            let i = Input { pos: Vec3::new(x0 + 8.0 * (k + 1) as f32, 0.0, 0.0), facing: PI / 2.0, auto: true, ..inp() };
            f.step(&i, w);
            yaws.push(f.yaw);
        }
        yaws
    }

    #[test]
    fn the_swing_behind_ramps_as_kh1s() {
        let mut f = Follow::default();
        // Camera behind facing 0 (yaw π); Sora turns to face +x (π/2) and runs: his back is −π/2.
        f.behind(&Input { auto: true, ..inp() }, &mut Open);
        f.prev = Some(Vec3::ZERO);
        let yaws = running(&mut f, &mut Open, 62, 0.0);
        // Frame 1: his facing changed by 90° (a reset); then the first ramp's 60 frames, no swing.
        for y in &yaws[..61] {
            assert!(wrap_angle(*y - PI).abs() < 1e-4, "{y}");
        }
        assert!((f.swing.first - 60.0).abs() < 1e-6 && (f.swing.second - 1.0).abs() < 1e-6, "{:?}", f.swing);
        // Frame 62: 2 % × 1/90 of the angle to his back (−90°).
        let want = PI + (0.02 / 90.0) * wrap_angle(PI / 2.0 + PI - PI);
        assert!(wrap_angle(yaws[61] - want).abs() < 1e-5, "{} {}", yaws[61], want);
        // A wall near: the wall ramp, and the second ramp at its 90 at once.
        struct Walled;
        impl World for Walled {
            fn sweep(&mut self, _: Vec3, _: Vec3, _: f32) -> Option<Vec3> {
                None
            }
            fn near_wall(&mut self, _: Vec3, _: f32) -> bool {
                true
            }
        }
        let y = f.yaw;
        running(&mut f, &mut Walled, 1, 8.0 * 62.0);
        assert_eq!((f.swing.wall, f.swing.second), (1.0, 90.0));
        let delta = wrap_angle(PI / 2.0 + PI - y);
        assert!(wrap_angle(f.yaw - (y + (0.02 / 60.0 + 0.02) * delta)).abs() < 1e-5);
    }

    #[test]
    fn auto_eye_distance_and_visibility() {
        let i = Input { auto: true, ..inp() };
        // A wall 200 behind the look point stops the eye's 40 sphere; the distance then springs
        // 10 % back toward 400.
        let mut f = Follow::default();
        f.behind(&i, &mut Open);
        let mut w = Wall(f.look.z - 200.0);
        f.step(&i, &mut w);
        let d = (f.eye - f.look).length();
        assert!(d < 300.0, "{d}");
        assert!((f.dist - (d + (400.0 - d) * 0.1)).abs() < 1e-3);
        // Nothing reaches Sora from the eye: re-placed at the same yaw with the defaults.
        struct Blind(u32);
        impl World for Blind {
            fn sweep(&mut self, from: Vec3, to: Vec3, _: f32) -> Option<Vec3> {
                self.0 += 1;
                Some(from + (to - from) * 0.999)
            }
        }
        let mut g = Follow::default();
        g.behind(&i, &mut Open);
        g.pitch = 0.5;
        g.step(&i, &mut Blind(0));
        assert!((g.pitch - PITCH_DEFAULT).abs() < 1e-3, "{}", g.pitch);
    }

    #[test]
    fn the_pitch_assist_lifts_an_eye_near_the_floor() {
        // A floor 30 below the eye (KH1 y down: +30): the ramp grows 1 a frame to 30 and lifts the
        // pitch by (1 − 30/55) × ramp/30 × 3°.
        struct Floor;
        impl World for Floor {
            fn sweep(&mut self, from: Vec3, to: Vec3, _: f32) -> Option<Vec3> {
                // Only the eye's 55 ray down meets it.
                (to.y - from.y == 55.0).then(|| from + Vec3::new(0.0, 30.0, 0.0))
            }
        }
        let i = inp();
        let mut f = Follow::default();
        f.behind(&i, &mut Open);
        let p0 = f.pitch;
        f.step(&i, &mut Floor);
        assert!((f.pitch - p0).abs() < 1e-6, "the ramp starts at 0");
        assert_eq!(f.assist.floor, 1.0);
        let p1 = f.pitch;
        f.step(&i, &mut Floor);
        assert!((f.pitch - (p1 + (1.0 - 30.0 / 55.0) * (1.0 / 30.0) * auto::ASSIST_STEP)).abs() < 1e-6);
    }

    #[test]
    fn the_lock_camera_frames_sora_and_the_target() {
        let i = inp();
        let mut f = Follow::default();
        f.behind(&i, &mut Open);
        // A target 5 m ahead, 1 m up, radius 0: the anchor 3.5 m behind Sora, the look 1 m past it.
        let t = Target { point: Vec3::new(0.0, -100.0, 500.0), radius: 0.0 };
        f.lock_on(false);
        for _ in 0..200 {
            f.lock_step(&i, &t, &mut Open);
            f.view(1.0);
        }
        // The eye rests within KH1's 75 dead zone of the anchor (here at its edge, from behind).
        assert!((f.eye.x).abs() < 1e-2 && ((f.eye.z + 350.0).abs() - 75.0).abs() < 1e-2, "{:?}", f.eye);
        assert!((f.eye.y + 100.0).abs() < 1e-2, "{:?}", f.eye);
        assert!((f.look - Vec3::new(0.0, -100.0, 600.0)).length() < 1e-1, "{:?}", f.look);
        assert!(wrap_angle(f.yaw - PI).abs() < 1e-4);
        // The eye closes 90 % of the gap past 75 a frame.
        let mut g = Follow::default();
        g.behind(&i, &mut Open);
        g.eye = Vec3::new(0.0, -100.0, -350.0 - 175.0);
        g.yaw = PI;
        g.lock_step(&i, &t, &mut Open);
        assert!((g.eye.z - (-525.0 + 0.9 * 100.0)).abs() < 1e-2, "{:?}", g.eye);
        // A target to the side: the yaw turns half the error past the ±5° band, at most 10°.
        let mut h = f;
        let side = Target { point: Vec3::new(600.0, -100.0, 0.0), radius: 0.0 };
        let y0 = h.yaw;
        h.lock_step(&i, &side, &mut Open);
        let turn = wrap_angle(h.yaw - y0);
        assert!(turn.abs() <= lockcam::TURN_MAX + 1e-4 && turn.abs() > 0.1, "{}", turn.to_degrees());
    }

    #[test]
    fn mode_blends_and_relock() {
        let i = inp();
        let mut f = Follow::default();
        f.behind(&i, &mut Open);
        let (e0, l0) = (f.eye, f.look);
        f.lock_on(false);
        assert_eq!((f.blend.left, f.blend.total), (60.0, 60.0));
        // Move the internal eye; the drawn one follows with weight cos(left / 60 · π/2).
        f.eye = e0 + Vec3::new(100.0, 0.0, 0.0);
        let (e, _) = f.view(1.0);
        let w = ((59.0f32 / 60.0) * PI * 0.5).cos();
        assert!((e.x - (e0.x + 100.0 * w)).abs() < 1e-3, "{:?}", e);
        for _ in 0..59 {
            f.view(1.0);
        }
        assert_eq!(f.view(1.0).0, f.eye);
        // A new target while locked: the look point blends from the old one over 60 frames.
        let t = Target { point: Vec3::new(0.0, -100.0, 500.0), radius: 0.0 };
        f.lock_step(&i, &t, &mut Open);
        f.lock_on(true);
        assert_eq!(f.lock.relock, 60.0);
        let old = f.lock.old_look;
        let t2 = Target { point: Vec3::new(300.0, -100.0, 500.0), radius: 0.0 };
        f.lock_step(&i, &t2, &mut Open);
        assert!((f.look - old).length() < 30.0, "first frame near the old look: {:?} {:?}", f.look, old);
        // Unlock: KH1's follow entry with a 45 blend.
        f.unlock(&i, &mut Open);
        assert_eq!(f.blend.total, 45.0);
        assert!((f.pitch - PITCH_DEFAULT).abs() < 1e-6 && (f.dist - DIST_DEFAULT).abs() < 1e-6);
        let _ = l0;
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

    fn k2() -> kh2::Input {
        kh2::Input { pos: Vec3::ZERO, facing: 0.0, right: kh2::Stick::default(), left: kh2::Stick::default(), mouse: false, can_act: true, reset: false, swap_h: false, swap_v: false, step: 1.0, auto: true }
    }

    #[test]
    fn kh2_records_and_rest_pose() {
        use kh2::*;
        // FOV: KH2's 1.5 rad horizontal at its focal 320 / tan(0.75) is Re:CoM's 73.39°.
        let fov = recom_fov_degrees();
        assert!((fov - 73.393).abs() < 0.001, "{fov}");
        let r = records()[2];
        assert_eq!(r, records()[3]);
        assert!((r.fov - fov).abs() < 1e-6);
        // Look 1.513 m above the feet, the eye 3.738 m back and 0.7227 m over the look (× 0.89).
        assert!((r.aim[1] + SUBJECT_LIFT_M - 1.513).abs() < 1e-4, "{:?}", r);
        assert!((r.eye[2] - 3.738).abs() < 1e-4 && (r.eye[1] - r.aim[1] - 0.7227).abs() < 1e-3, "{:?}", r);
        // The live re-place gives the same pose.
        let mut c = Cam::default();
        let i = Input { pos: Vec3::new(100.0, 0.0, 200.0), facing: 0.5, ..k2() };
        c.behind(&i, true);
        let (e, l) = (c.eye * SCALE, c.look * SCALE);
        assert!((l - (i.pos + Vec3::new(0.0, -151.3, 0.0))).length() < 1e-2, "{:?}", l);
        assert!(((e - l).horizontal_length() - 373.8).abs() < 1e-2);
        assert!(((l.y - e.y) - 72.268).abs() < 1e-2, "{} {}", l.y, e.y);
        assert!(wrap_angle(kh1::yaw_of(e, l) - (0.5 + PI)).abs() < 1e-5);
        // A step with no input keeps it (the springs at rest, no wall).
        let (e1, l1) = c.step(&i, None, &mut Open);
        assert!((e1 - e).length() < 1e-3 && (l1 - l).length() < 1e-3);
        assert_eq!((Family::of(8), Family::of(9), Family::Kh2.base(true)), (Family::Kh2, Family::Kh2, 8));
        assert_eq!(dispatch(1, Family::Kh2, false, false, false, Some(false)), Dispatch::Switch(8));
        assert_eq!(dispatch(6, Family::Kh2, true, false, false, Some(false)), Dispatch::Switch(8));
        assert_eq!(dispatch(8, Family::Kh1, true, false, false, Some(false)), Dispatch::Switch(6));
        assert_eq!(dispatch(8, Family::Recom, true, false, false, Some(false)), Dispatch::Switch(3));
        assert_eq!(dispatch(8, Family::Kh2, true, false, false, Some(false)), Dispatch::Ours);
        assert_eq!(swaps(-1.0, 1.0), (false, false), "Re:CoM's Normal is KH2's");
        assert_eq!(swaps(1.0, -1.0), (true, true));
    }

    #[test]
    fn kh2_sticks_are_per_axis_past_60() {
        use kh2::Stick;
        // Inside 60 of 127.5 on an axis: nothing.
        assert_eq!(Stick::from_bytes(128 + 59, 128, 60.0), Stick { x: 0.0, y: 0.0, tilt: 0.0, left: false, right: false, up: false, down: false });
        let s = Stick::from_bytes(255, 128, 60.0);
        assert!(s.right && !s.left && (s.x - 1.0).abs() < 1e-6 && (s.tilt - 1.0).abs() < 1e-6);
        // Each axis on its own: a diagonal sets both bits, the tilt the clamped length.
        let s = Stick::from_bytes(0, 0, 60.0);
        assert!(s.left && s.up && (s.tilt - 1.0).abs() < 1e-6 && (s.x + 1.0).abs() < 1e-6 && (s.y + 1.0).abs() < 1e-6);
        let s = Stick::from_bytes(128 + 60 + 33, 128, 60.0);
        assert!(s.right && ((s.x - 33.5 / 67.5).abs() < 1e-5), "{s:?}");
        // Re:CoM's full tilt.
        let s = Stick::from_recom(0, 4096, false);
        assert!(s.down && (s.tilt - 1.0).abs() < 1e-6);
        assert!(Stick::from_recom(1000, 0, true).right, "the mouse's dead zone of 1");
        assert!(!Stick::from_recom(1000, 0, false).right);
    }

    #[test]
    fn kh2_turns_3_degrees_and_zooms_8_on_its_height_curve() {
        use kh2::*;
        let mut c = Cam::default();
        let i = k2();
        c.behind(&i, true);
        let y0 = c.yaw;
        let right = Input { right: Stick::from_bytes(255, 128, DEAD_ZONE), ..i };
        c.step(&right, None, &mut Open);
        assert!((wrap_angle(c.yaw - y0) - 3f32.to_radians()).abs() < 1e-5);
        assert_eq!(c.yaw, c.goal);
        // Half tilt past the dead zone: half the speed.
        let half = Input { right: Stick::from_bytes(128 + 60 + 33, 128, DEAD_ZONE), ..i };
        let y1 = c.yaw;
        c.step(&half, None, &mut Open);
        assert!((wrap_angle(c.yaw - y1) - 3f32.to_radians() * 33.5 / 67.5).abs() < 1e-5);
        // Up: 8 a frame closer, to 250; there the eye is 150 below the look point (KH2 units).
        let up = Input { right: Stick::from_bytes(128, 0, DEAD_ZONE), ..i };
        let d0 = c.dist;
        c.step(&up, None, &mut Open);
        assert!((c.dist - (d0 - 8.0)).abs() < 1e-4);
        for _ in 0..40 {
            c.step(&up, None, &mut Open);
        }
        assert_eq!(c.dist, DIST_MIN);
        assert!((c.eye_goal.y - c.look_goal.y - 150.0).abs() < 1e-3);
        let down = Input { right: Stick::from_bytes(128, 255, DEAD_ZONE), ..i };
        for _ in 0..80 {
            c.step(&down, None, &mut Open);
        }
        assert_eq!(c.dist, DIST_MAX);
        // Swapped up / down (Re:CoM's vertical row reversed): up zooms out.
        let mut d = Cam::default();
        d.behind(&i, true);
        d.step(&Input { swap_v: true, ..up }, None, &mut Open);
        assert!((d.dist - 428.0).abs() < 1e-4);
    }

    #[test]
    fn kh2_reset_turns_at_most_10_degrees_or_cuts() {
        use kh2::*;
        let mut c = Cam::default();
        c.behind(&k2(), true);
        let i = Input { facing: PI / 2.0, ..k2() };
        let y0 = c.yaw;
        c.dist = 300.0;
        c.step(&Input { reset: true, ..i }, None, &mut Open);
        assert_eq!(c.dist, DIST_DEFAULT);
        assert!((wrap_angle(c.goal - (PI / 2.0 + PI))).abs() < 1e-5);
        assert!((wrap_angle(c.yaw - y0).abs() - 10f32.to_radians()).abs() < 1e-5);
        for _ in 0..8 {
            c.step(&i, None, &mut Open);
        }
        assert!(wrap_angle(c.yaw - c.goal).abs() < 1e-5, "90° in 9 frames");
        // With the left stick held: a cut, the view snapped.
        let mut d = Cam::default();
        d.behind(&k2(), true);
        let held = Input { reset: true, left: Stick::from_bytes(128, 0, DEAD_ZONE), ..i };
        d.step(&held, None, &mut Open);
        assert!(wrap_angle(d.yaw - d.goal).abs() < 1e-5);
    }

    #[test]
    fn kh2_swings_behind_a_running_sora() {
        use kh2::*;
        let mut c = Cam::default();
        c.behind(&k2(), true);
        c.dist = 300.0;
        // Sora faces +x and runs; the stick pushed forward (y −1): 2 · 1 · 2 · 1 · 0.0043633 rad
        // = 1° a frame toward his back; the distance 0.5 a frame back toward 420.
        let fwd = Stick::from_bytes(128, 0, DEAD_ZONE);
        let run = |k: usize| Input { pos: Vec3::new(8.0 * k as f32, 0.0, 0.0), facing: PI / 2.0, left: fwd, ..k2() };
        let y0 = c.yaw;
        c.step(&run(1), None, &mut Open);
        let want = (2.0 * 1.0 * 2.0 * 1.0 * SWING_K0) * wrap_angle(PI / 2.0 + PI - y0).signum();
        assert!((wrap_angle(c.yaw - y0) - want).abs() < 1e-6, "{} {}", c.yaw, want);
        assert!((c.dist - 300.5).abs() < 1e-5);
        // Not with the right stick in use, not pulled back, not standing, not with Auto off.
        for i in [Input { right: Stick::from_bytes(128, 255, DEAD_ZONE), ..run(2) }, Input { left: Stick::from_bytes(128, 255, DEAD_ZONE), ..run(2) }, Input { auto: false, ..run(2) }] {
            let mut d = c;
            d.prev = Some(Vec3::new(8.0, 0.0, 0.0) * (1.0 / SCALE));
            let y = d.yaw;
            d.step(&i, None, &mut Open);
            assert_eq!(d.yaw, y, "{i:?}");
        }
        let mut d = c;
        let y = d.yaw;
        d.step(&run(1), None, &mut Open);
        assert_eq!(d.yaw, y, "Sora didn't move");
    }

    #[test]
    fn kh2_springs_close_the_gap_at_its_rates() {
        use kh2::*;
        // A still goal: the eye closes 0.035 · 2.5 = 8.75 % of (gap − 1) a frame, the look point
        // 0.065 · 2.5 = 16.25 %; they rest 1 away. With the mouse: 0.2 · 2.5.
        let g = Vec3::new(0.0, 0.0, 0.0);
        let cur = Vec3::new(101.0, 0.0, 0.0);
        let e = spring(cur, g, g, RATE * 0.5, SPRING_REST);
        assert!((e.x - (101.0 - 100.0 * 0.0875)).abs() < 1e-3, "{e:?}");
        let l = spring(cur, g, g, RATE * 0.5 + RATE_LOOK, SPRING_REST);
        assert!((l.x - (101.0 - 100.0 * 0.1625)).abs() < 1e-3, "{l:?}");
        let mut p = cur;
        for _ in 0..400 {
            p = spring(p, g, g, RATE * 0.5, SPRING_REST);
        }
        assert!((p.x - 1.0).abs() < 1e-3);
        // Sora steps 100 (KH2 units) in a frame: the goals jump 100 and the springs' moving-goal
        // term (the last goal − the goal, · rate · 0.5 along the gap) holds them back a little:
        // the look point (6.5 · 0.5 + (1 − 100) · 2.5) · 0.065 = 15.88 instead of 16.09.
        let mut c = Cam::default();
        c.behind(&k2(), true);
        let before = c.eye;
        let i = Input { pos: Vec3::new(89.0, 0.0, 0.0), ..k2() };
        c.step(&i, None, &mut Open);
        let want = -((6.5 * 0.5 + (1.0 - 100.0) * 2.5) * 0.065);
        assert!((c.look.x - want).abs() < 1e-3, "{:?} {want}", c.look);
        assert!(c.eye.x - before.x > 0.0 && c.eye.x - before.x < 100.0 * 0.0875, "{:?}", c.eye);
    }

    #[test]
    fn kh2_wall_puts_the_eye_5_off_the_wall() {
        use kh2::*;
        struct WallN(f32);
        impl World for WallN {
            fn sweep(&mut self, from: Vec3, to: Vec3, r: f32) -> Option<Vec3> {
                if to.z >= self.0 || from.z <= self.0 {
                    return None;
                }
                let t = (self.0 + r - from.z) / (to.z - from.z);
                Some(from + (to - from) * t)
            }
            fn normal(&mut self) -> Vec3 {
                Vec3::new(0.0, 0.0, 1.0)
            }
        }
        // Sora faces +z; his eye would be 373.8 behind: a wall 200 behind the feet.
        let mut c = Cam::default();
        let i = k2();
        c.behind(&i, true);
        let mut w = WallN(-200.0);
        let (e, _) = c.step(&i, None, &mut w);
        // The eye goal at the wall (+ the ray's radius) + 5 × 0.89 along the normal; the drawn eye
        // tested again on its way there.
        assert!((c.eye_goal.z * SCALE - (-200.0 + kh1::RAY + WALL_PUSH * SCALE)).abs() < 1e-2, "{:?}", c.eye_goal);
        assert!(e.z >= -200.0, "{e:?}");
    }

    #[test]
    fn kh2_screen_test_on_its_512_by_416_frame() {
        use kh2::*;
        // The eye 420 behind and above a look point 170 up, looking +z (KH2 units).
        let (eye, look) = (Vec3::new(0.0, -251.2, -420.0), Vec3::new(0.0, -170.0, 0.0));
        assert_eq!(screen(eye, look, look), 0, "the look point is the frame's centre");
        assert_eq!(screen(eye, look, Vec3::new(0.0, -170.0, -500.0)) & 0x10, 0x10, "behind the eye");
        // Re:CoM's look-at (x = up × forward) puts KH2's +x on the left looking +z.
        assert_eq!(screen(eye, look, Vec3::new(2000.0, -170.0, 0.0)), 4);
        assert_eq!(screen(eye, look, Vec3::new(-2000.0, -170.0, 0.0)), 8);
        assert_eq!(screen(eye, look, Vec3::new(0.0, 400.0, 0.0)) & 3, 2, "below the frame");
        assert_eq!(screen(eye, look, Vec3::new(0.0, -1500.0, 0.0)) & 3, 1, "above it");
        // The margins: 32 of 512 at the sides (the FOV edge is 256 from the centre).
        let t = (FOV * 0.5).tan();
        // A point on the look point's depth plane: x on screen = 256 + 256 · (x / z) / tan.
        let z = (look - eye).length();
        let px = |sx: f32| Vec3::new(-(sx - 256.0) / 256.0 * t * z, -170.0, 0.0);
        assert_eq!(screen(eye, look, px(470.0)) & 0xC, 0);
        assert_eq!(screen(eye, look, px(490.0)) & 0xC, 8);
        assert_eq!(screen(eye, look, px(40.0)) & 0xC, 0);
        assert_eq!(screen(eye, look, px(20.0)) & 0xC, 4);
    }

    #[test]
    fn kh2_lock_camera_frames_sora_and_the_target() {
        use kh2::*;
        let i = k2();
        let mut c = Cam::default();
        c.behind(&i, true);
        // A target 5 m ahead, 1 m up (KH1 units): the look goal 30 % of the way from Sora's chest
        // (170 × 0.89 up) to it; the eye from the chest on the height curve; nothing turns while
        // both are on screen; the distance to 380 at 1 a frame (× 0.89 out).
        let t = Target { point: Vec3::new(0.0, -100.0, 500.0) };
        let y0 = c.yaw;
        c.step(&i, Some(&t), &mut Open);
        let chest = Vec3::new(0.0, -LOOK_UP, 0.0);
        let p = t.point * (1.0 / SCALE);
        assert!((c.look_goal - (chest + (p - chest) * 0.3)).length() < 1e-3, "{:?}", c.look_goal);
        assert_eq!((c.yaw, c.goal), (y0, y0), "both on screen: no turn");
        assert!((c.dist - 419.0).abs() < 1e-4);
        let cc = c.dist - DIST_MIN;
        assert!((c.eye_goal.y - (chest.y + EYE_BASE - cc * cc * HEIGHT_K)).abs() < 1e-3);
        assert!(((c.eye_goal - chest).horizontal_length() - c.dist).abs() < 1e-3);
        for _ in 0..60 {
            c.step(&i, Some(&t), &mut Open);
        }
        assert!((c.dist - DIST_LOCK).abs() < 1.0, "{}", c.dist);
        // The target off the screen's left side: the goal yaw on the line from the last look goal
        // through Sora + 20°; the yaw turns 10° a frame toward it.
        let side = Target { point: Vec3::new(2500.0, -100.0, 100.0) };
        let from = c.look_goal;
        let y1 = c.yaw;
        assert_eq!(screen(c.eye, c.look, side.point * (1.0 / SCALE)) & 4, 4);
        c.step(&i, Some(&side), &mut Open);
        let line = { let v = Vec3::ZERO - from; v.x.atan2(v.z) };
        assert!(wrap_angle(c.goal - (line + OVERSHOOT)).abs() < 1e-4, "{} {}", c.goal, line);
        assert!((wrap_angle(c.yaw - y1).abs() - 10f32.to_radians()).abs() < 1e-4 || wrap_angle(c.yaw - c.goal).abs() < 1e-4);
        // The reset: the goal on the line, the lock distance.
        let mut d = c;
        d.dist = 300.0;
        let from = d.look_goal;
        d.step(&Input { reset: true, ..i }, Some(&t), &mut Open);
        let line = { let v = Vec3::ZERO - from; v.x.atan2(v.z) };
        assert!(wrap_angle(d.goal - line).abs() < 1e-4);
        assert_eq!(d.dist, DIST_LOCK);
        // A target high above and far: the look stays at the chest.
        let mut e = Cam::default();
        e.behind(&i, true);
        let high = Target { point: Vec3::new(0.0, -3000.0, 300.0) };
        e.step(&i, Some(&high), &mut Open);
        assert!((e.look_goal - chest).length() < 1e-3, "{:?}", e.look_goal);
        // Dispatch: 8 ↔ 9 on Re:CoM's lock, a subject object holds the mode.
        assert_eq!(dispatch(8, Family::Kh2, false, false, false, Some(true)), Dispatch::Switch(9));
        assert_eq!(dispatch(9, Family::Kh2, true, false, false, Some(false)), Dispatch::Switch(8));
        assert_eq!(dispatch(9, Family::Kh2, true, false, false, Some(true)), Dispatch::Ours);
        assert_eq!(dispatch(8, Family::Kh2, true, false, false, None), Dispatch::Ours);
        assert_eq!(dispatch(9, Family::Kh1, true, false, false, Some(true)), Dispatch::Switch(6));
    }

    #[test]
    fn kh2_lock_shrinks_the_distance_at_the_bottom_edge() {
        use kh2::*;
        let i = k2();
        let mut c = Cam::default();
        c.behind(&i, true);
        // A target low under the frame's bottom (close below the eye's view): − 2 a frame, no drift.
        let low = Target { point: Vec3::new(0.0, 300.0, 120.0) };
        let u = screen(c.eye, c.look, low.point * (1.0 / SCALE));
        assert_eq!(u & 3, 2, "{u:#x}");
        let d0 = c.dist;
        let sora = screen(c.eye, c.look, Vec3::new(0.0, -LOCK_SORA, 0.0)) & 3 == 2;
        c.step(&i, Some(&low), &mut Open);
        let want = d0 - 2.0 - if sora { 2.0 } else { 0.0 };
        assert!((c.dist - want).abs() < 1e-3, "{} {want}", c.dist);
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

    /// Stage 4's per-frame cost of the KH2 step's own arithmetic (Re:CoM's sweeps excluded: two a
    /// frame, the goal's wall ray and the drawn eye's, against one in Re:CoM's follow).
    #[test]
    #[ignore]
    fn cost_of_a_kh2_step() {
        let mut c = kh2::Cam::default();
        let i = k2();
        c.behind(&i, true);
        let n = 1_000_000;
        let t = std::time::Instant::now();
        for k in 0..n {
            let j = kh2::Input { pos: Vec3::new((k % 100) as f32, 0.0, 0.0), right: kh2::Stick::from_recom((((k / 50) % 3) as i32 - 1) * 4096, 0, false), left: kh2::Stick::from_recom(0, -4096, false), ..i };
            c.step(&j, None, &mut Open);
        }
        let ns = t.elapsed().as_nanos() as f64 / n as f64;
        println!("KH2 follow step: {ns:.1} ns a frame (math only)");
        // Stage 5: the lock step (two screen tests more).
        let t2 = kh2::Target { point: Vec3::new(0.0, -100.0, 500.0) };
        let t = std::time::Instant::now();
        for k in 0..n {
            let j = kh2::Input { pos: Vec3::new((k % 100) as f32, 0.0, 0.0), ..i };
            c.step(&j, Some(&t2), &mut Open);
        }
        let ns = t.elapsed().as_nanos() as f64 / n as f64;
        println!("KH2 lock step: {ns:.1} ns a frame (math only)");
        assert!(c.yaw.is_finite());
    }
}
