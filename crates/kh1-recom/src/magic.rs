//! KH1's magic inside Re:CoM's own magic action (M7 stage 7.4, decision 34): the cardless cast
//! and Fire. The testable part; the Re:CoM side is `win/magic.rs`, the notes
//! `docs/recom-m7-magic.md`.
//!
//! **The cast is KH1's frame on Re:CoM's magic action `0x13`**, as a swing is KH1's frame on
//! Re:CoM's attack action `0xA` (M5):
//!
//! - KH1's command (`0x1402A7510` type 2 → `0x1402A3600` → `0x1402A3DB0` / its air branch, and
//!   the Fire fast path) runs in [`Kh1Sora::frame`]: KH1's commit gate, its cast motions and
//!   their windows (the KH1 motion record, as for every KH1 move), the air hop, the facing.
//! - Re:CoM runs the action: [`Host::start_cast`] makes Sora's action Re:CoM's magic action with
//!   a **KH1 row** in Re:CoM's attack-row table format ([`fire_row_bytes`]: Re:CoM's own Fire /
//!   Fira / Firaga row with KH1's numbers), Re:CoM's clips play the cast motions, and at KH1's
//!   release frame [`Host::release_cast`] has Re:CoM's own shot runtime (`CHomingBullet`) fire the
//!   ball from that row: Re:CoM steps, homes, draws, collides and ends it, with Re:CoM's Fire
//!   effects and sounds (decision 34).
//! - **KH1's script decides the release** ([`Kh1Sora::cast_step`], `xm_fire01.mag` decoded): on
//!   the wind-up path, the frame the wind-up request (`0x36` / `0x83`) is no longer playing; on
//!   the fast path, 8 frames after the command. At that frame the release request (`0x3D` /
//!   `0x8A`) must be playing, else nothing is cast and **no MP is paid**; otherwise the ball is
//!   launched and the MP paid (KH1 syscall 47 after the launch).
//!
//! Units as [`crate::movement`]: KH1 frames (60 Hz) and cm; Re:CoM rows hold metres × 100 and
//! ticks / 100 (50 ticks per 60 Hz frame, `docs/recom-timing.md`).

use crate::movement::{Action, Host, Kh1Sora};
use kh1_core::controller::state;
use kh1_core::damage::{compute_damage, AttackerStats, LiveAttack};
use kh1_core::motion::{req, request, FlagKind, MotionInfo, MotionSet};
use kh1_core::{AttackData, DamageCurve, DefenderStats, Kh1Rand, TargetPart, Vec3, WeaponData};

/// KH1's spells (`0x142D22B80` order; the command `level + (spell + 2) × 3`).
pub mod spell {
    pub const FIRE: u8 = 0;
    pub const BLIZZARD: u8 = 1;
    pub const THUNDER: u8 = 2;
    pub const CURE: u8 = 3;
    pub const GRAVITY: u8 = 4;
    pub const STOP: u8 = 5;
    pub const AERO: u8 = 6;
    pub const NAMES: [&str; 7] = ["fire", "blizzard", "thunder", "cure", "gravity", "stop", "aero"];
}

/// Spells this stage casts (Blizzard … Aero: 7.5–7.10, each its own row and script numbers).
pub fn castable(s: u8) -> bool {
    s == spell::FIRE
}

/// KH1's cast motion requests (`0x1402A3DB0` / `0x1402A3600` / the fast path in `0x1402A7510`).
/// Checked (decomp).
pub mod cast_req {
    /// Fire's wind-up and release, ground and air.
    pub const FIRE_WINDUP: u16 = 0x36;
    pub const FIRE_RELEASE: u16 = 0x3D;
    pub const FIRE_AIR_WINDUP: u16 = 0x83;
    pub const FIRE_AIR_RELEASE: u16 = 0x8A;
}

/// Re:CoM's magic action and its attack ids (`initMagic` `0x1402203D0` sets `+0x638` and
/// `+0x414` to them; Fire `0x22`). Checked (decomp).
pub const RECOM_MAGIC_ACTION: u32 = 0x13;
/// `player+0x604` as `initMagic` sets it.
pub const MAGIC_PHASE: u32 = 3;

/// Re:CoM's attack id (`+0x638` / `+0x414`) for a spell: Fire 0x22 … Aero 0x28.
pub fn recom_attack_id(s: u8) -> u32 {
    0x22 + s as u32
}

/// Re:CoM's base row (`0x1407C6C70 + id × 16`) whose effect set the KH1 spell level plays:
/// Fire `0x22`, Fira `0x29` (set 4), Firaga `0x2A` (set 5). Checked (exe: the sets hold
/// `fire_all` entries; `initStockSkill`'s switch maps the -ra / -ga sleights to 0x29 / 0x2A).
pub fn recom_base_row(s: u8, level: u8) -> Option<usize> {
    match (s, level) {
        (spell::FIRE, 1) => Some(0x22),
        (spell::FIRE, 2) => Some(0x29),
        (spell::FIRE, 3) => Some(0x2A),
        _ => None,
    }
}

/// Re:CoM's rows 0x22 / 0x29 / 0x2A as in the Steam exe (the KH1 rows' templates; the runtime
/// refuses to build them from other bytes). Checked (exe).
pub const RECOM_FIRE_ROWS: [[u8; 16]; 3] = [
    [0x37, 0x01, 0x03, 0x00, 0x14, 0x4B, 0x00, 0x00, 0x0A, 0x0F, 0x00, 0x01, 0x0E, 0x14, 0x14, 0x00],
    [0x37, 0x01, 0x04, 0x00, 0x1E, 0x64, 0x00, 0x00, 0x0B, 0x0F, 0x00, 0x02, 0x0E, 0x14, 0x19, 0x00],
    [0x37, 0x01, 0x05, 0x00, 0x28, 0x7D, 0x00, 0x00, 0x0C, 0x0F, 0x00, 0x03, 0x0E, 0x14, 0x1E, 0x00],
];

/// KH1's Fire ball (`xm_fire01..03.mag`, the same script at every level; Checked (files: decoded
/// with `magdis.py`; the syscalls in the KH1 decomp)).
pub mod fire {
    /// The ball's speed, cm per 60 Hz frame (`ind+0x34`, sys 18 `pos += dir × speed × step`).
    pub const SPEED: f32 = 40.0;
    /// Its life in frames (the thread's `ind+0x30` = 30, −frame step per frame).
    pub const LIFE: f32 = 30.0;
    /// Its homing turn, degrees per frame (`ind+0x38`, sys 24).
    pub const TURN_DEG: f32 = 3.0;
    /// It starts fading this many frames before its end (`ind+0x3C`).
    pub const FADE: f32 = 20.0;
    /// With no live target, it picks the best enemy within this range of the ball each frame
    /// (`ind+0x40`, sys 78). Not built (open question 1).
    pub const RETARGET: f32 = 500.0;
    /// The fast path's wait: the release 8 frames after the command (`L006fa(8.0)`).
    pub const FAST_WAIT: f32 = 8.0;
    /// MP cost (spell data `+0x30`, all levels): 30 charge units.
    pub const COST: i32 = crate::mp::cost::FIRE;
    /// The release motion takes a new Fire from this frame (`0x140293480`). Checked (decomp).
    pub const RECAST_FROM: f32 = 14.0;
}

/// KH1's air cast (`0x1402A3600` state 2): a new jump arc of G`+0xE8` = 30 cm and the air
/// velocity × G`+0xF0` = 0.5 (the fast path only scales). Checked (decomp + files).
pub const AIR_HOP: f32 = 30.0;
pub const AIR_SPEED_SCALE: f32 = 0.5;
/// The fast path plays the release motion with an 8-frame blend (`movss xmm2,[0x14042F94C]`
/// = 8.0); the air cast's queued fall too. Checked (exe).
pub const FAST_BLEND: f32 = 8.0;
pub const AIR_FALL_BLEND: f32 = 8.0;

/// KH1's Fire attack data (btltbl `0x5F28` / `0x5F98` / `0x6008`, read through `kh1-formats`
/// from the player's `btltbl.bin`). Checked (files).
pub fn fire_attack(level: u8) -> AttackData {
    let power = match level {
        2 => 28,
        3 => 36,
        _ => 20,
    };
    AttackData {
        stat_source: 1,
        element: 2,
        kind_party: 3,
        kind: 3,
        bone: -1,
        power_party: power,
        mp_charge: 0,
        clash: 5,
        priority: 1,
        recoil_request: 110,
        power,
        radius: 50.0,
        ..Default::default()
    }
}

/// Row flag `0x80` (unused by Re:CoM's 188 rows; `crate::attack::FLAG_KH1`): a KH1 row.
pub const FLAG_KH1: u8 = crate::attack::FLAG_KH1;

/// The KH1 Fire row in Re:CoM's 16-byte attack-row layout: Re:CoM's row of that level (its clip,
/// blend and effect set: Re:CoM's visuals and sounds, decision 34) with KH1's ball. The fields as
/// Re:CoM's spawn reads them (`0x140222170` case `0x22`, Checked (disasm `0x14022220B..`)):
///
/// | byte | Re:CoM reads | KH1 value |
/// |---|---|---|
/// | `+4` | radius × 0.01 m | 50 (KH1's 50 cm) |
/// | `+8` | the homing distance gate (m; homes while the target is nearer) | 255 (KH1 homes at any distance) |
/// | `+9` | speed × 0.01 m per frame | 40 (KH1's 40 cm a frame) |
/// | `+0xB` | Re:CoM kind | 1, flinch (KH1 kind 3) |
/// | `+0xC` | range from the spawn point (m) | 255 (KH1's ball has no range: its life ends it) |
/// | `+0xD` | homing turn, degrees per frame (`0x1401D7F00`: × dt × 0.02) | 3 |
/// | `+0xE` | life × 100 ticks | 15 = 1500 ticks = 30 frames (read by our spawn case; Re:CoM's Fire case uses 12000) |
/// | `+0xF` | flags | `0x80`: a KH1 row |
pub fn fire_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[4] = 50;
    b[8] = 255;
    b[9] = fire::SPEED as u8;
    b[0xB] = 1;
    b[0xC] = 255;
    b[0xD] = fire::TURN_DEG as u8;
    b[0xE] = (fire::LIFE * 50.0 / 100.0) as u8;
    b[0xF] |= FLAG_KH1;
    b
}

/// The KH1 magic rows, `[spell × 3 + level − 1]` (21; this stage fills Fire's three), from
/// Re:CoM's Fire rows (which must be [`RECOM_FIRE_ROWS`]).
pub fn kh1_magic_table(recom_fire: &[[u8; 16]; 3]) -> Vec<[u8; 16]> {
    let mut t = vec![[0u8; 16]; 21];
    for (i, r) in recom_fire.iter().enumerate() {
        t[i] = fire_row_bytes(r);
    }
    t
}

/// The index of a spell level in [`kh1_magic_table`].
pub fn table_index(s: u8, level: u8) -> Option<usize> {
    (s < 7 && (1..=3).contains(&level)).then(|| s as usize * 3 + level as usize - 1)
}

/// The shot descriptor's numbers (`0x140156790`'s 0xE0-byte block, filled as Re:CoM's Fire case
/// fills it), from a KH1 row. Re:CoM ticks: 50 per 60 Hz frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShotNumbers {
    /// `+0x60` (u64): the bullet's flags.
    pub flags: u64,
    /// `+0x30`: speed, m per frame.
    pub speed: f32,
    /// `+0x34`: range from the spawn point, m.
    pub range: f32,
    /// `+0x3C`: life, ticks (ends the ball with flag 4).
    pub life: i32,
    /// `+0x48`: the homing distance gate, m.
    pub gate: f32,
    /// `+0x4C`: the homing turn, degrees per frame.
    pub turn: f32,
    /// `+0x78`: radius, m.
    pub radius: f32,
    /// `+0x94`: the fade after the end, ticks (Re:CoM's default 1500).
    pub fade: i32,
}

/// Re:CoM's Fire shot flags `0x9100` (Checked (disasm `0x140222254`)): `0x8000` steers by angles
/// (`0x1401D7F00`), `0x1000` homes only beyond `+0x1C` (0.5 m) from the spawn point, `0x100`
/// stops and fades on a hit, a wall, the range or the life. KH1: homing from its first frame (no
/// `0x1000`), and the life ends it (`4`, `0x140156E20`). Checked (decomp `0x1401575D0`,
/// `0x140156E20`).
pub const RECOM_FIRE_FLAGS: u64 = 0x9100;
pub const KH1_FIRE_FLAGS: u64 = (RECOM_FIRE_FLAGS & !0x1000) | 4;

/// The descriptor numbers a KH1 row gives.
pub fn shot_numbers(row: &[u8; 16]) -> ShotNumbers {
    ShotNumbers {
        flags: KH1_FIRE_FLAGS,
        speed: row[9] as f32 * 0.01,
        range: row[0xC] as f32,
        life: row[0xE] as i32 * 100,
        gate: row[8] as f32,
        turn: row[0xD] as f32,
        radius: row[4] as f32 * 0.01,
        // KH1 fades the ball over its last 20 frames while it flies; Re:CoM fades a stopped one
        // after its end. The fade's length is KH1's.
        fade: (fire::FADE * 50.0) as i32,
    }
}

/// KH1's spell damage on an enemy (`0x1402BFA80`, kh1-core): power + max MP (stat source 1),
/// the curve on power − Defence, the record's floor and cap, × the element's resistance (fire:
/// `resist[2]`). No crit (the record's crit is 0; the roll is still drawn, as KH1 does).
pub fn spell_damage(data: &AttackData, max_mp: i32, def: &DefenderStats, rng: &mut Kh1Rand) -> i32 {
    let attacker = AttackerStats { max_mp, ..Default::default() };
    let live = LiveAttack::new(data, &WeaponData::kingdom_key(), &attacker, 1.0);
    compute_damage(&live, &DamageCurve::kh1_defaults(), def, rng).damage
}

/// KH1's cast motions (Sora's mset, read with `kh1-data` from the player's
/// `xa_ex_0010.mset`; Checked (files)): Fire's wind-up `0x36` (entry 54, 28 frames, no flags),
/// release `0x3D` (entry 57, 60 frames, free from 46, root −15 cm over frames 0–10), air
/// wind-up `0x83` (entry 79, 28) and air release `0x8A` (entry 82, 52 frames, free from 46, root
/// −42.4 cm over 0–10). Re:CoM's clips `p001_062`, `p001_063`, `p001_066`, `p001_067` (rows 0x37,
/// 0x38, 0x3B, 0x3C) have the same frame counts at 60 Hz (14, 30, 14, 26 at 30 fps; research
/// `m7-magic.md` § 3.5, Checked (files)).
pub fn add_motions(m: &mut MotionSet) {
    let inf = f32::INFINITY;
    m.add(&[cast_req::FIRE_WINDUP], MotionInfo::new(28.0));
    m.add(&[cast_req::FIRE_RELEASE], MotionInfo::new(60.0).with_flag(FlagKind::Free, 46.0, inf).with_forward_keys(&[(0.0, 0.0), (10.0, -15.0), (60.0, -15.0)]));
    m.add(&[cast_req::FIRE_AIR_WINDUP], MotionInfo::new(28.0));
    m.add(&[cast_req::FIRE_AIR_RELEASE], MotionInfo::new(52.0).with_flag(FlagKind::Free, 46.0, inf).with_forward_keys(&[(0.0, 0.0), (10.0, -42.41), (52.0, -42.41)]));
}

/// Whether a KH1 request is one of the cast motions Re:CoM's magic action plays.
pub fn is_cast_request(r: u16) -> bool {
    matches!(r, cast_req::FIRE_WINDUP | cast_req::FIRE_RELEASE | cast_req::FIRE_AIR_WINDUP | cast_req::FIRE_AIR_RELEASE)
}

fn is_windup(r: u16) -> bool {
    r == cast_req::FIRE_WINDUP || r == cast_req::FIRE_AIR_WINDUP
}

fn is_release(r: u16) -> bool {
    r == cast_req::FIRE_RELEASE || r == cast_req::FIRE_AIR_RELEASE
}

/// A KH1 magic command (the menu's, or a shortcut's in 7.12): spell 0..6, level 1..3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MagicCommand {
    pub spell: u8,
    pub level: u8,
}

/// A cast for Re:CoM to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastStart {
    pub spell: u8,
    pub level: u8,
    /// Started in the air (Re:CoM's air clips, `+0x478` = 4).
    pub air: bool,
    /// KH1's Fire fast path (no wind-up).
    pub fast: bool,
}

/// The cast in progress: KH1's `.mag` thread state that Re:CoM has no place for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cast {
    pub start: CastStart,
    /// The fast path's countdown (`L006fa`), frames.
    pub wait: f32,
    /// Frames since the command (for the log and trace).
    pub age: f32,
    /// The ball is out (MP paid), or the script ended without one.
    pub released: bool,
    pub cancelled: bool,
    /// Re:CoM's shot id (`bullet+0x20`), once launched.
    pub shot: Option<u32>,
    /// Started in this frame (a new cast replacing an older one keeps Re:CoM's action).
    pub fresh: bool,
}

/// What the cast did this frame (log and trace).
#[derive(Clone, Debug, PartialEq)]
pub enum CastEvent {
    /// The command was taken: spell, level, fast path, air.
    Started { spell: u8, level: u8, fast: bool, air: bool },
    /// The command was refused (KH1's gate, the state, the MP, no Re:CoM row).
    Refused { spell: u8, level: u8, why: &'static str },
    /// The release frame: frames since the command, the shot id (None: Re:CoM made no shot).
    Released { spell: u8, level: u8, frame: f32, shot: Option<u32> },
    /// The script's release check failed, or the cast was cut before it: no spell, no MP.
    Interrupted { spell: u8, level: u8, frame: f32, by: u16 },
}

impl CastEvent {
    pub fn token(&self) -> String {
        let n = |s: u8, l: u8| format!("{}{l}", spell::NAMES.get(s as usize).copied().unwrap_or("?"));
        match self {
            CastEvent::Started { spell, level, fast, air } => format!("cast({}{}{})", n(*spell, *level), if *fast { " fast" } else { "" }, if *air { " air" } else { "" }),
            CastEvent::Refused { spell, level, why } => format!("cast-refused({} {why})", n(*spell, *level)),
            CastEvent::Released { spell, level, frame, shot } => format!("release({} f{frame:.0}{})", n(*spell, *level), shot.map_or(" no-shot".to_string(), |s| format!(" shot {s}"))),
            CastEvent::Interrupted { spell, level, frame, by } => format!("cast-cut({} f{frame:.0} req {by:#x})", n(*spell, *level)),
        }
    }
}

/// The trace token of the cast in progress: `cast -`, or `cast fire1 f<age> [fast] [w<wait>]
/// [rel] [shot <id>]`.
pub fn trace_token(c: Option<&Cast>) -> String {
    match c {
        None => "cast -".into(),
        Some(c) => {
            let s = c.start;
            let mut t = format!("cast {}{} f{:.0}", spell::NAMES.get(s.spell as usize).copied().unwrap_or("?"), s.level, c.age);
            if s.fast {
                t.push_str(&format!(" fast w{:.0}", c.wait.max(0.0)));
            }
            if s.air {
                t.push_str(" air");
            }
            if c.released {
                t.push_str(" rel");
            }
            if c.cancelled {
                t.push_str(" cut");
            }
            if let Some(id) = c.shot {
                t.push_str(&format!(" shot {id}"));
            }
            t
        }
    }
}

// ---- KH1's magic command in the handler frame ----

impl Kh1Sora {
    /// `0x140293480` for a spell: not in the take-off; free or in a combo window; else only Fire
    /// while the release motion (`0x3D` / `0x8A`) is at frame 14 or later. Checked (decomp).
    pub fn accepts_spell(&self, s: u8) -> bool {
        if self.state == state::TAKE_OFF {
            return false;
        }
        if !self.lock || self.in_window() {
            return true;
        }
        s == spell::FIRE && is_release(self.rec.req) && fire::RECAST_FROM <= self.rec.frame
    }

    /// The Fire fast path's condition in `0x1402A7510` (Checked (decomp + disasm
    /// `0x1402A75B6`)): a Fire command while the playing request is the release motion, or swing 1
    /// (200) / the lunge (202) inside its combo window.
    pub fn fire_fast_path(&self, s: u8) -> bool {
        s == spell::FIRE && (is_release(self.rec.req) || ((self.rec.req == 200 || self.rec.req == 202) && self.in_window()))
    }

    /// Re:CoM's action for a cast in progress (its magic action, as `initMagic` sets it).
    pub(crate) fn cast_action(&self) -> Option<Action> {
        let c = self.cast.as_ref()?;
        is_cast_request(self.rec.head()).then(|| Action { id: RECOM_MAGIC_ACTION, sub: recom_attack_id(c.start.spell), phase: MAGIC_PHASE })
    }

    /// `0x1402A7510` type 2 (the Fire fast path), else `0x1402A3600` → `0x1402A3DB0` (ground) / the
    /// air branch. The command first faces its target (`0x1402A38E0`, record flag 2: all spells).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn command_magic(&mut self, h: &mut dyn Host, dir: &mut Vec3, w: &mut f32, target: Option<TargetPart>, pos: Vec3, m: MagicCommand) {
        let refuse = |me: &mut Kh1Sora, why| me.cast_log.push(CastEvent::Refused { spell: m.spell, level: m.level, why });
        if !castable(m.spell) {
            return refuse(self, "spell not in this stage");
        }
        // `0x1402A38E0`: toward the target's part, else the stick (or the facing); no speed.
        if let Some(t) = target {
            let d = (t.centre - pos).flat().normalized();
            if !d.is_zero_xz() {
                *dir = d;
                self.target_yaw = d.yaw();
            }
        } else if dir.is_zero_xz() {
            *dir = Vec3::from_yaw(self.facing);
        }
        *w = 0.0;
        let fast = self.fire_fast_path(m.spell);
        let air = self.state == state::AIR;
        if !fast {
            // `0x1402A3600`: ground (0) and air (2) cast; other states drop the command.
            if self.state != state::GROUND && !air {
                return refuse(self, "state");
            }
            // `0x14028EFD0`: KH1's usable test (learned, MP for the cost).
            if !h.can_cast(m.spell, m.level) {
                return refuse(self, "mp");
            }
        }
        let start = CastStart { spell: m.spell, level: m.level, air, fast };
        // A swing playing (the fast path from swing 1 / the lunge, or a cast from a combo
        // window): Re:CoM ends its attack action first, as the new action starts.
        self.leave_swing(h);
        if !h.start_cast(&start) {
            return refuse(self, "no Re:CoM row");
        }
        use cast_req::*;
        if fast {
            if air {
                self.play(FIRE_AIR_RELEASE as u32 | req::ACTION, FAST_BLEND);
                self.append(req::RELEASE | request::FALL as u32, 0.0);
                self.air_vel = self.air_vel * AIR_SPEED_SCALE;
            } else {
                self.play(FIRE_RELEASE as u32 | req::ACTION, FAST_BLEND);
                self.append(req::RELEASE | request::IDLE as u32, 0.0);
            }
        } else if air {
            self.play(FIRE_AIR_WINDUP as u32 | req::ACTION, 0.0);
            self.append(FIRE_AIR_RELEASE as u32, 0.0);
            self.append(req::RELEASE | request::FALL as u32, AIR_FALL_BLEND);
            // `0x1402AF960(actor, 30)`, then `+0x3C0` × 0.5.
            self.hop(AIR_HOP, pos.y);
            self.air_vel = self.air_vel * AIR_SPEED_SCALE;
        } else {
            self.play(FIRE_WINDUP as u32 | req::ACTION, 0.0);
            self.append(FIRE_RELEASE as u32, 0.0);
            self.append(req::RELEASE | request::IDLE as u32, 0.0);
        }
        self.cast = Some(Cast { start, wait: if fast { fire::FAST_WAIT } else { 0.0 }, age: 0.0, released: false, cancelled: false, shot: None, fresh: true });
        self.cast_log.push(CastEvent::Started { spell: m.spell, level: m.level, fast, air });
    }

    /// One run of KH1's spell script (`xm_fire01.mag` main) for the last frame. KH1 runs the spell
    /// threads in their task (`0x14028F6B0`, priority 137000) after the actors' update, so a run
    /// sees that frame's motion update: here, right after [`Kh1Sora::frame`] has replayed it at
    /// the start of the next handler. The wind-up path waits while the wind-up plays, the fast
    /// path 8 frames (`L006fa`); then the release motion must be playing, else nothing is cast and
    /// no MP is paid; else the ball goes and the MP is paid (sys 20, then sys 47). `age` counts
    /// the runs from 0 in the command's frame, so the ball's KH1 frame is the age of its run.
    pub(crate) fn cast_step(&mut self, h: &mut dyn Host, step: f32) {
        let head = self.rec.head();
        let Some(c) = self.cast.as_mut() else { return };
        let age = c.age;
        c.age += step;
        c.fresh = false;
        if c.released || c.cancelled {
            return;
        }
        if c.start.fast {
            if c.wait > 0.0 {
                c.wait -= step;
                return;
            }
        } else if is_windup(head) {
            return;
        }
        let s = c.start;
        if !is_release(head) {
            c.cancelled = true;
            self.cast_log.push(CastEvent::Interrupted { spell: s.spell, level: s.level, frame: age, by: head });
            return;
        }
        c.released = true;
        let shot = h.release_cast(&s);
        if let Some(c) = self.cast.as_mut() {
            c.shot = shot;
        }
        self.cast_log.push(CastEvent::Released { spell: s.spell, level: s.level, frame: age, shot });
    }

    /// The cast's motions are no longer KH1's playing motion: Re:CoM ends its magic action
    /// (`0x1401F6660`, as `exeMagic` ends it) unless a new swing or cast already took Re:CoM's
    /// action over. A cast cut before its release casts nothing and pays nothing.
    pub(crate) fn leave_cast(&mut self, h: &mut dyn Host) {
        let Some(c) = self.cast else { return };
        if is_cast_request(self.rec.head()) {
            return;
        }
        self.cast = None;
        if !c.released && !c.cancelled {
            self.cast_log.push(CastEvent::Interrupted { spell: c.start.spell, level: c.start.level, frame: c.age, by: self.rec.head() });
        }
        if self.swing.is_none() {
            h.end_cast(self.state != state::GROUND);
        }
    }

    /// The cast record and its events, for the frame's caller (drained).
    pub fn take_cast_log(&mut self) -> Vec<CastEvent> {
        std::mem::take(&mut self.cast_log)
    }
}

/// The Field exe (`0x1401FA0F0`) has no case for action `0x13`: Re:CoM never casts in the field
/// (its jump table `0x1401FA5F4` sends the id to the default `0x1401FA599`). KH1 casts in the
/// field, so the entry is pointed at a stub in an int3 cave that runs `exeMagic` on the player
/// (`rbx` in the Field exe's switch) and goes on at the default, as the table's other cases do
/// (`mov rcx,rbx; call exe; jmp default`). The Battle exe already calls it through Sora's vtable
/// (`0x1401F9B4D`). Checked (disasm `0x1401FA1B0`, `0x1401FA599`).
pub mod field_case {
    pub const TABLE_ENTRY_RVA: u32 = 0x001F_A63C;
    pub const CAVE_RVA: u32 = 0x001F_B0E1;
    pub const EXE_MAGIC_RVA: u32 = 0x0021_7EE0;
    pub const DEFAULT_RVA: u32 = 0x001F_A599;
    pub const STUB_LEN: usize = 13;

    /// The 13-byte stub at the cave.
    pub fn stub() -> [u8; STUB_LEN] {
        let mut b = [0u8; STUB_LEN];
        b[..3].copy_from_slice(&[0x48, 0x8B, 0xCB]);
        b[3] = 0xE8;
        let call = EXE_MAGIC_RVA as i64 - (CAVE_RVA as i64 + 8);
        b[4..8].copy_from_slice(&(call as i32).to_le_bytes());
        b[8] = 0xE9;
        let jmp = DEFAULT_RVA as i64 - (CAVE_RVA as i64 + 13);
        b[9..13].copy_from_slice(&(jmp as i32).to_le_bytes());
        b
    }

    /// The table entry's new value (an RVA, as the table holds).
    pub fn entry() -> [u8; 4] {
        CAVE_RVA.to_le_bytes()
    }
}

#[cfg(test)]
mod tests;
