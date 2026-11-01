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

/// Spells cast so far (Fire 7.4, Blizzard 7.5, Thunder 7.6, Cure 7.7, Gravity 7.8, Stop 7.9; Aero: 7.10,
/// each its own rows and script numbers).
pub fn castable(s: u8) -> bool {
    matches!(s, spell::FIRE | spell::BLIZZARD | spell::THUNDER | spell::CURE | spell::GRAVITY | spell::STOP)
}

/// A spell's MP cost (its data's `+0x30`, the same at every level): charge units below 100,
/// whole points × 100 from 100 on (`0x1402A4C10` / `0x1402A5B60`). Checked (files).
pub fn cost(s: u8) -> i32 {
    use crate::mp::cost::*;
    [FIRE, BLIZZARD, THUNDER, CURE, GRAVITY, STOP, AERO].get(s as usize).copied().unwrap_or(i32::MAX)
}

/// KH1's cast motion requests (`0x1402A3DB0` / `0x1402A3600` / the fast path in `0x1402A7510`).
/// Checked (decomp).
pub mod cast_req {
    /// Fire's wind-up and release, ground and air.
    pub const FIRE_WINDUP: u16 = 0x36;
    pub const FIRE_RELEASE: u16 = 0x3D;
    pub const FIRE_AIR_WINDUP: u16 = 0x83;
    pub const FIRE_AIR_RELEASE: u16 = 0x8A;
    /// Blizzard's one cast motion, ground and air (`0x36 + 1`, `0x83 + 1`; entries 55 / 80).
    pub const BLIZZARD: u16 = 0x37;
    pub const BLIZZARD_AIR: u16 = 0x84;
    /// Thunder's one cast motion (`0x36 + 2`, `0x83 + 2`; entries 56 / 81).
    pub const THUNDER: u16 = 0x38;
    pub const THUNDER_AIR: u16 = 0x85;
    /// Cure's one cast motion (`0x36 + 3`, `0x83 + 3`; entries 56 / 81, Thunder's). Checked (files).
    pub const CURE: u16 = 0x39;
    pub const CURE_AIR: u16 = 0x86;
    /// Gravity's one cast motion (`0x36 + 4`, `0x83 + 4`; entries 55 / 80, Blizzard's). Checked
    /// (files).
    pub const GRAVITY: u16 = 0x3A;
    pub const GRAVITY_AIR: u16 = 0x87;
    /// Stop's one cast motion (`0x36 + 5`, `0x83 + 5`; entries 56 / 81, Thunder's). Checked (files).
    pub const STOP: u16 = 0x3B;
    pub const STOP_AIR: u16 = 0x88;

    /// The first cast motion a spell's normal cast plays: `0x1402A3DB0` plays `0x36 + spell`
    /// (Fire: `0x36` then `0x3D`), the air branch of `0x1402A3600` `0x83 + spell` (Fire: `0x83`
    /// then `0x8A`). Checked (decomp + disasm `0x1402A3E92..0x1402A3ED7`).
    pub fn first(spell: u8, air: bool) -> u16 {
        if air {
            FIRE_AIR_WINDUP + spell as u16
        } else {
            FIRE_WINDUP + spell as u16
        }
    }

    /// The motion the spell's script requires at its release check (`xm_fire01`: `0x3D` / `0x8A`;
    /// `xm_blizzard01`: 55 / 132 = `0x37` / `0x84`). Checked (files).
    pub fn release(spell: u8, air: bool) -> u16 {
        match (spell, air) {
            (super::spell::FIRE, false) => FIRE_RELEASE,
            (super::spell::FIRE, true) => FIRE_AIR_RELEASE,
            _ => first(spell, air),
        }
    }
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
        // Blizzard 0x23 (set 6), Blizzara 0x2B (set 8), Blizzaga 0x2C (set 10): `initStockSkill`'s
        // -ra / -ga sleights 0x0E / 0x15. Checked (exe: the sets hold `bliz_all`).
        (spell::BLIZZARD, 1) => Some(0x23),
        (spell::BLIZZARD, 2) => Some(0x2B),
        (spell::BLIZZARD, 3) => Some(0x2C),
        // Thunder 0x24 (set 12), Thundara 0x2E (13), Thundaga 0x2F (14): `initStockSkill`'s
        // sleights 0x0F / 0x16. Checked (exe: the sets hold `_thunder_all`).
        (spell::THUNDER, 1) => Some(0x24),
        (spell::THUNDER, 2) => Some(0x2E),
        (spell::THUNDER, 3) => Some(0x2F),
        // Cure 0x25 (set 15), Cura 0x36 (16), Curaga 0x37 (17): `initStockSkill`'s sleights 0x10 /
        // 0x17. Checked (exe: the rows' clip 0x39, sets 15–17 of `cure_all`).
        (spell::CURE, 1) => Some(0x25),
        (spell::CURE, 2) => Some(0x36),
        (spell::CURE, 3) => Some(0x37),
        // Gravity 0x26 (set 18), Gravira 0x38 (19), Graviga 0x39 (20): `initStockSkill`'s sleights
        // 0x11 / 0x18. Checked (exe: clip 0x3A, sets 18–20 of `grav_all`).
        (spell::GRAVITY, 1) => Some(0x26),
        (spell::GRAVITY, 2) => Some(0x38),
        (spell::GRAVITY, 3) => Some(0x39),
        // Stop 0x27 (set 21), Stopra 0x3A (22), Stopga 0x3B (23): `initStockSkill`'s sleights 0x12 /
        // 0x19. Checked (exe: clip 0x39, sets 21–23 of `stop_all`, kind 8).
        (spell::STOP, 1) => Some(0x27),
        (spell::STOP, 2) => Some(0x3A),
        (spell::STOP, 3) => Some(0x3B),
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
    /// A ball cast with no target picks the best enemy within this range of the ball each frame
    /// (`ind+0x40`, sys 78; [`super::retarget`]).
    pub const RETARGET: f32 = 500.0;
    /// The fast path's wait: the release 8 frames after the command (`L006fa(8.0)`).
    pub const FAST_WAIT: f32 = 8.0;
    /// MP cost (spell data `+0x30`, all levels): 30 charge units.
    pub const COST: i32 = crate::mp::cost::FIRE;
    /// The release motion takes a new Fire from this frame (`0x140293480`). Checked (decomp).
    pub const RECAST_FROM: f32 = 14.0;
}

/// KH1's Fire re-target (M7 stage 7.4b, decision 36). The ball thread's loop (`xm_fire01.mag`
/// `L000c2`, the same at every level; Checked (files: decoded with `magdis.py`)) reads the cast's
/// target from the thread's arguments every frame:
///
/// - **no target** (the command had none): it picks one with sys 78 (`0x1402BA440` →
///   `0x1402B0E90(caster, ball position, direction with w = 2, 500)`) and homes on it this frame;
///   the pick is not kept, so the next frame picks again;
/// - **a target that is no longer alive** (sys 120 `0x1402BAB10` → `0x140291FF0`: the actor's
///   `+0x374` bit 0 set and bit 1 clear): no homing this frame and **no new pick** (it flies
///   straight);
/// - a live target: homes on its part (sys 119).
///
/// The pick `0x1402B0E90` (Checked (decomp)): every actor of the actor array (`0x140291AB0`:
/// live, not flagged `+0 & 0x200`, on a side `+0x374 >> 5 & 3` that is not 0 and not the
/// caster's), `v` = its position (`+0x10`) − the ball's, `d` = |v| (3D, `0x1402A9240`); taken
/// only when `d < 500`; score `d × ((1 − cos) × w + 1)` with `cos` = the clamped cosine between
/// `v` and the ball's direction (`0x1402A8C70`, 0 for a zero vector) and `w` = 2; the first
/// candidate in range is taken, later ones only with a smaller score. Then (in the same frame) the
/// ball turns ≤ 3° toward the pick's part and moves.
pub mod retarget {
    /// The range, Re:CoM metres (KH1's 500 cm).
    pub const RANGE_M: f32 = super::fire::RETARGET / 100.0;
    /// The off-axis weight (the script writes 2.0 into the direction's `w`).
    pub const WEIGHT: f32 = 2.0;

    fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
    }
    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    /// `0x1402A8C70`: the cosine between two vectors, clamped to −1..1, 0 when either is zero.
    pub fn cos(a: [f32; 3], b: [f32; 3]) -> f32 {
        let (la, lb) = (dot(a, a).abs().sqrt(), dot(b, b).abs().sqrt());
        if la == 0.0 || lb == 0.0 {
            return 0.0;
        }
        (dot(a, b) / (lb * la)).clamp(-1.0, 1.0)
    }

    /// One candidate's distance and score, or None when it is out of range. Positions in Re:CoM
    /// metres: KH1's (x, y, z) cm is Re:CoM's (x, −y, −z) / 100, a rotation and a uniform scale,
    /// so distances scale by 1/100 and the cosine is the same.
    pub fn score(ball: [f32; 3], dir: [f32; 3], at: [f32; 3]) -> Option<(f32, f32)> {
        let v = sub(at, ball);
        let d = dot(v, v).abs().sqrt();
        (d < RANGE_M).then(|| (d, d * ((1.0 - cos(v, dir)) * WEIGHT + 1.0)))
    }

    /// `0x1402B0E90`: the best candidate `(id, position)` for a ball at `ball` flying along `dir`,
    /// with its distance. The first one in range is taken, later ones with a smaller score
    /// (KH1's `lVar2 == 0 || score < best`).
    pub fn pick(ball: [f32; 3], dir: [f32; 3], cands: impl IntoIterator<Item = (u32, [f32; 3])>) -> Option<(u32, f32)> {
        let mut best: Option<(u32, f32, f32)> = None;
        for (id, at) in cands {
            if let Some((d, s)) = score(ball, dir, at) {
                if best.is_none_or(|b| s < b.2) {
                    best = Some((id, d, s));
                }
            }
        }
        best.map(|b| (b.0, b.1))
    }
}

/// KH1's Blizzard (`xm_blizzard01..03.mag`, decoded with `magdis.py`: one script, the level's shard
/// speed passed to it; Checked (files), the syscalls in the KH1 decomp).
pub mod blizzard {
    /// The script waits this long after the command (`L0086e(28)`), then needs its cast motion
    /// (`0x37` / `0x84`) still playing, else ends: no spell, no MP.
    pub const WAIT: f32 = 28.0;
    /// Shard speed by level, cm per frame (`L006dc(args, 20 / 30 / 40)`).
    pub const SPEED: [f32; 3] = [20.0, 30.0, 40.0];
    /// Shard life, frames (`L001dc`: `ind+0x0` = 26).
    pub const LIFE: f32 = 26.0;
    /// The height each shard keeps above the floor it finds below it (`ind+0x8` = 50).
    pub const HEIGHT: f32 = 50.0;
    /// The first cone direction (local: +z forward, x sideways, y vertical), the script's own
    /// values (`*(sp+0x30)` = (0.383, 0, 0.924): 22.5° off the axis); each next shard turns it
    /// 60° about the axis (sys 31). Six cone shards, then one straight ahead (0, 0, 1).
    pub const CONE: [f32; 3] = [0.383, 0.0, 0.924];
    pub const RING_DEG: f32 = 60.0;
    pub const CONE_SHARDS: usize = 6;
    /// A cone shard starts its direction's sideways part × 50 cm off the axis (sys 93 then sys
    /// 10 / 7: 0.383 × 50 = 19.15 cm); the straight one at the Keyblade point.
    pub const OFFSET: f32 = 50.0;
    /// Attack radius by level (data `+0x5C`: 70 / 90 / 120 cm).
    pub const RADIUS: [f32; 3] = [70.0, 90.0, 120.0];
    /// MP cost (data `+0x30`): 30 charge units.
    pub const COST: i32 = crate::mp::cost::BLIZZARD;
}

/// The shards of one Blizzard in Re:CoM's frame (metres, y up), for Sora facing `yaw` (Re:CoM's
/// body yaw: forward = (sin yaw, 0, cos yaw)): `(start offset from the Keyblade point, unit
/// direction)`, the six cone shards first, then the straight one, as the script makes them.
///
/// KH1 rotates its local vectors by the actor's yaw (sys 10) in its own axes (y down); the six
/// cone shards sit every 60° about the forward axis, a set that is the same under left / right and
/// up / down mirroring, so it maps onto Re:CoM's axes without regard to which local axis is which.
pub fn blizzard_shards(yaw: f32) -> [([f32; 3], [f32; 3]); 7] {
    use blizzard::*;
    let f = [yaw.sin(), 0.0, yaw.cos()];
    let r = [yaw.cos(), 0.0, -yaw.sin()];
    let u = [0.0, 1.0, 0.0];
    let comb = |a: f32, b: f32, c: f32| [r[0] * a + u[0] * b + f[0] * c, r[1] * a + u[1] * b + f[1] * c, r[2] * a + u[2] * b + f[2] * c];
    let mut out = [([0.0; 3], f); 7];
    for (i, o) in out.iter_mut().take(CONE_SHARDS).enumerate() {
        let phi = (RING_DEG * i as f32).to_radians();
        // sys 31: the cone vector turned about the forward (local z) axis.
        let (lx, ly) = (CONE[0] * phi.cos() - CONE[1] * phi.sin(), CONE[0] * phi.sin() + CONE[1] * phi.cos());
        let off = comb(lx * OFFSET / 100.0, ly * OFFSET / 100.0, 0.0);
        *o = (off, comb(lx, ly, CONE[2]));
    }
    out
}

/// KH1's shard height step (`xm_blizzard01.mag` `L0029e`, Re:CoM metres, y up): the shard has
/// moved sideways this frame; a ray from its height straight down by `HEIGHT − dy` (sys 77 →
/// `0x140299530`, any surface) finds `floor` (the hit's height) → the shard sits `HEIGHT` above it;
/// no hit → it climbs or sinks by its own `dy` (its direction's vertical part × speed).
pub fn blizzard_glide_y(y: f32, dy: f32, floor: Option<f32>) -> f32 {
    match floor {
        Some(h) => h + blizzard::HEIGHT / 100.0,
        None => y + dy,
    }
}

/// The glide ray's length below the shard, metres: `HEIGHT + dy_down` in KH1's y-down terms.
pub fn blizzard_ray(dy: f32) -> f32 {
    blizzard::HEIGHT / 100.0 - dy
}

/// KH1's Blizzard attack data (btltbl `0x6078` / `0x60E8` / `0x6158`, Checked (files)): power
/// 22 / 27 / 34 + max MP, ice (element 3), flinch, radius 70 / 90 / 120, no MP charge, no re-hit.
pub fn blizzard_attack(level: u8) -> AttackData {
    let i = level.clamp(1, 3) as usize - 1;
    let power = [22, 27, 34][i];
    AttackData {
        stat_source: 1,
        element: 3,
        kind_party: 3,
        kind: 3,
        bone: -1,
        power_party: power,
        mp_charge: 0,
        clash: 5,
        priority: 1,
        recoil_request: 110,
        power,
        radius: blizzard::RADIUS[i],
        ..Default::default()
    }
}

/// KH1's attack data for a spell level (the hit's damage).
pub fn spell_attack(s: u8, level: u8) -> Option<AttackData> {
    match s {
        spell::FIRE => Some(fire_attack(level)),
        spell::BLIZZARD => Some(blizzard_attack(level)),
        spell::THUNDER => Some(thunder_attack(level)),
        spell::GRAVITY => Some(gravity_attack(level)),
        spell::STOP => Some(stop_attack(level)),
        // Cure heals (Re:CoM's amounts, decision 34): no hit, no damage.
        _ => None,
    }
}

/// Re:CoM's Blizzard rows 0x23 / 0x2B / 0x2C as in the Steam exe (clip 0x3A, effect sets 6 / 8 /
/// 10 of `bliz_all`; the KH1 rows' templates). Checked (exe).
pub const RECOM_BLIZZARD_ROWS: [[u8; 16]; 3] = [
    [0x3A, 0x01, 0x06, 0x1E, 0x0A, 0x32, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0x0A, 0x00, 0x0A, 0x00],
    [0x3A, 0x01, 0x08, 0x1E, 0x0F, 0x4B, 0x00, 0x00, 0x00, 0x14, 0x00, 0x02, 0x0A, 0x00, 0x0F, 0x00],
    [0x3A, 0x01, 0x0A, 0x1E, 0x14, 0x64, 0x00, 0x00, 0x00, 0x14, 0x00, 0x03, 0x0A, 0x00, 0x14, 0x00],
];

/// The KH1 Blizzard row: Re:CoM's row of that level (clip, blend, effect set: decision 34) with
/// KH1's shard, in the fields Re:CoM's Blizzard case (`0x140222170` case `0x23`, Checked (decomp))
/// reads: `+4` radius × 0.01 m (70 / 90 / 120), `+9` speed × 0.01 m a frame (20 / 30 / 40), `+0xB`
/// kind 1 (flinch; KH1 kind 3), `+0xC` range 255 m (none: the life ends it), `+0xE` life × 100
/// ticks (13 = 26 frames), `+0xF` = `0x80` (a KH1 row); `+5` (the case's `desc+0x1C`) kept.
pub fn blizzard_row_bytes(recom: &[u8; 16], level: u8) -> [u8; 16] {
    let i = level.clamp(1, 3) as usize - 1;
    let mut b = *recom;
    b[3] = 0;
    b[4] = blizzard::RADIUS[i] as u8;
    b[9] = blizzard::SPEED[i] as u8;
    b[0xB] = 1;
    b[0xC] = 255;
    b[0xD] = 0;
    b[0xE] = (blizzard::LIFE * 50.0 / 100.0) as u8;
    b[0xF] |= FLAG_KH1;
    b
}

/// The shard's descriptor flags: Re:CoM's Blizzard `0x4000000` (fade, not vanish, when Re:CoM kills
/// a shot) with KH1's ends: `0x100` (a hit or a wall ends it, as KH1's shard ends on its attack's
/// first hit) and `4` (the life ends it). No homing. Checked (decomp `0x140222170` case `0x23`,
/// `0x140156E20`, `0x1401569B0`).
pub const KH1_BLIZZARD_FLAGS: u64 = 0x400_0000 | 0x100 | 4;

/// KH1's Thunder (`xm_thunder01..03.mag`, decoded with `magdis.py`: the same script at every level;
/// Checked (files), the syscalls in the KH1 decomp).
///
/// At the command (`L004b6`): the area's centre is the command target's part if it is alive, else
/// Sora's position (the area marker effect alone is drawn 2 m ahead along the camera); the
/// distances are measured from the target (alive), else Sora. Then a screen tint to 50 % over 20
/// frames (`L00064`, sys 52), **wait 30**, the release check (`0x38` / `0x85`; else the tint goes
/// back and nothing is cast, no MP), the strike thread, **the MP** (sys 47, 100 = one point),
/// then 40 frames and the tint back over 20.
///
/// The strike thread (`0x17E`): the centre moves to the target's part if it is still alive; every
/// enemy (sys 65, up to 30 slots) with its distance from the target / Sora goes into a list sorted
/// nearest first (`L0069c`: selection sort, a later entry replaces the minimum only when nearer);
/// then down the list: an enemy whose hurt bodies overlap the area (sys 90 → `0x1402B6AF0`: a
/// vertical cylinder, type 2, radius 600, half-height 500, around the centre) **is struck
/// directly** (sys 160 → `0x1402BBB70`: the attack applied onto it, no collision) and the thread
/// waits 8 frames (`L00c8e(8)`); one outside it is passed over at once. Each enemy once.
pub mod thunder {
    pub const WAIT: f32 = 30.0;
    /// The area: a vertical cylinder, cm (`L004b6`: 600, height 1000 → ±500).
    pub const RADIUS: f32 = 600.0;
    pub const HALF_HEIGHT: f32 = 500.0;
    /// The list's slots.
    pub const MAX: usize = 30;
    /// The wait after each strike, frames.
    pub const INTERVAL: f32 = 8.0;
    /// MP cost (data `+0x30` = 100): one whole point.
    pub const COST: i32 = crate::mp::cost::THUNDER;
    /// The screen tint (not built: Re:CoM has no world-only tint we found; open question): to
    /// this level over `DIM_FRAMES`, held until 40 frames after the release, back over 20.
    pub const DIM: f32 = 0.5;
    pub const DIM_FRAMES: f32 = 20.0;
}

/// KH1's Thunder attack data (btltbl `0x61C8` / `0x6238` / `0x62A8`, Checked (files)): power 16 / 20
/// / 26 + **max MP × 2** (stat source 3), thunder (element 4), flinch, no radius (struck directly),
/// no MP charge, no re-hit.
pub fn thunder_attack(level: u8) -> AttackData {
    let power = [16, 20, 26][level.clamp(1, 3) as usize - 1];
    AttackData {
        stat_source: 3,
        element: 4,
        kind_party: 3,
        kind: 3,
        bone: -1,
        power_party: power,
        mp_charge: 0,
        clash: 6,
        priority: 0,
        recoil_request: 110,
        power,
        radius: 0.0,
        ..Default::default()
    }
}

/// Re:CoM's Thunder rows 0x24 / 0x2E / 0x2F as in the Steam exe (clip 0x39, effect sets 12 / 13 / 14
/// of `_thunder_all`, Re:CoM's bolt: fall speed `+9`, radius `+4` × 0.1 m, range `+0xC`). Checked
/// (exe).
pub const RECOM_THUNDER_ROWS: [[u8; 16]; 3] = [
    [0x39, 0x00, 0x0C, 0x1E, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x1C, 0x08, 0x00, 0x00, 0x00],
    [0x39, 0x00, 0x0D, 0x1E, 0x14, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x1D, 0x08, 0x00, 0x00, 0x00],
    [0x39, 0x00, 0x0E, 0x1E, 0x28, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x1E, 0x08, 0x00, 0x00, 0x00],
];

/// The KH1 Thunder row: Re:CoM's row (clip, blend, effect set, its bolt's speed / radius / range:
/// decision 34, Re:CoM's visual) with `+3` = 0 (KH1's script decides the release), `+0xB` = 1
/// (flinch; KH1 kind 3 — Re:CoM's Thunder kinds 0x1C–0x1E are not used) and `+0xF` = 0x80.
pub fn thunder_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[0xB] = 1;
    b[0xF] |= FLAG_KH1;
    b
}

/// KH1's Cure (`xm_cure01..03.mag`, decoded with `magdis.py`: the same script at every level;
/// Checked (files), the syscalls in the KH1 decomp) with **Re:CoM's heal amounts** (decision 34):
///
/// - the command: the charge effect on the Keyblade, **wait 36** (`L004e8(36)`), the release check
///   (the request must still be `0x39` / `0x86`, else it ends: no heal, no MP);
/// - the heal thread (`L00056`): the target is the command's party member, else the caster
///   (`sp+0x1c`); it is skipped (no heal, **no MP**) when the target is not alive (sys 120), has
///   flag `0x200` or lacks `0x8000` (`L00276`; Assumption: defeated / healable); else the data's
///   attack is struck onto it (sys 23 + sys 60: KH1's HP += 15 / 27 / 36 + max MP), then **the MP**
///   (sys 47, 100 = one point), then the heal effect follows the target.
///
/// At the KH1 setting the heal is Re:CoM's: its Cure case (`0x140222170` case `0x25`) heals
/// `0x1401E59A0(player, AP, 0, 1)` with AP = `BattleCardList[0x1D]` 500 / 10 = **50**, Cura
/// (`StockSkillList[0x10]` 1500 / 10) **150**, Curaga (`[0x17]` 3000 / 10) **300** (`SY0002.BIN`,
/// research `m7-magic.md` § 3.9 / 4.1.4; Checked (files + decomp)). Sora is the only target (no
/// party members yet).
pub mod cure {
    pub const WAIT: f32 = 36.0;
    /// Re:CoM's heal per level (decision 34).
    pub const HEAL: [i32; 3] = [50, 150, 300];
    /// MP cost (data `+0x30` = 100): one whole point.
    pub const COST: i32 = crate::mp::cost::CURE;

    /// The heal of a level.
    pub fn heal(level: u8) -> i32 {
        HEAL[level.clamp(1, 3) as usize - 1]
    }

    /// The thread's target test (`L00056`): the target alive, not flagged `0x200`, healable
    /// (`0x8000`). For Sora in Re:CoM: HP left and not in Re:CoM's death path (`+0x7B0`).
    pub fn target_ok(hp: i32, dead: bool) -> bool {
        hp > 0 && !dead
    }
}

/// Re:CoM's Cure rows 0x25 / 0x36 / 0x37 as in the Steam exe (clip 0x39, effect sets 15 / 16 / 17
/// of `cure_all`, kind 0x19, release `+3` 30). Checked (exe).
pub const RECOM_CURE_ROWS: [[u8; 16]; 3] = [
    [0x39, 0x00, 0x0F, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00],
    [0x39, 0x00, 0x10, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00],
    [0x39, 0x00, 0x11, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00],
];

/// The KH1 Cure row: Re:CoM's row (clip, blend, `cure_all` set: decision 34) with `+3` = 0 (KH1's
/// script decides the release) and `+0xF` = 0x80. Its kind is not read (no hit).
pub fn cure_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[0xF] |= FLAG_KH1;
    b
}

/// KH1's Gravity (`xm_gravity01..03.mag`, decoded with `magdis.py`: one script, the level passes
/// (final scale, fall frames) = (1.0, 60) / (1.5, 70) / (1.6, 60); Checked (files), the syscalls in
/// the KH1 decomp):
///
/// - the command (`L008cc`): the charge effect on the Keyblade; the marker thread (`0x25A`, 28
///   frames): the point is the target's part if it is alive, else the caster's position + **200 cm
///   along his facing** (taken at the command); the sphere's start is that point **240 cm up**
///   (`ind+0x4` = −230, then −10); **wait 28** (`L009e2(ind+0x1c = 28)`); the release check
///   (`0x3A` / `0x87`, else end: no spell, no MP); the falling thread (`0x21`); **the MP** (sys 47,
///   100); the hand effect;
/// - the fall (`0x44..0x16C`): the sphere starts at scale 0.01 and falls **230 cm** over N = 60 /
///   70 / 60 frames, its scale growing linearly to S = 1.0 / 1.5 / 1.6; each frame the enemies whose
///   bodies touch a sphere of **170 × the current scale** are slowed (`L0055c`: `actor+0x88 |= 4`);
/// - the impact (`L0044c`, the run after the N-th): **one direct strike** (sys 23 + 60) on every
///   enemy whose bodies touch a sphere of **170 × S** cm (170 / 255 / 272) at the sphere's centre
///   (sys 90 type 1); then the sphere grows × 1.02 a frame while it fades for 20 frames.
///
/// Damage (`0x1402BFA80`, element 8): `current HP × (power + 5 × max MP)% × resist[8]`, raised to
/// the record's floor (at least 1); no Defence, no cap. Power 40 / 55 / 70 (btltbl `0x6468` /
/// `0x64D8` / `0x6548`, stat source 5). Checked (files + decomp).
pub mod gravity {
    pub const WAIT: f32 = 28.0;
    /// The hit sphere's radius at scale 1, cm.
    pub const RADIUS: f32 = 170.0;
    /// The fall, cm, and the start above the marker point (230 + 10).
    pub const FALL: f32 = 230.0;
    pub const START_UP: f32 = 240.0;
    /// With no target: the marker point this far along the caster's facing, cm.
    pub const AHEAD: f32 = 200.0;
    /// The start scale and, per level, the final scale and the fall's frames.
    pub const START_SCALE: f32 = 0.01;
    pub const SCALE: [f32; 3] = [1.0, 1.5, 1.6];
    pub const FRAMES: [f32; 3] = [60.0, 70.0, 60.0];
    /// The fade after the impact: frames, growth a frame.
    pub const FADE: f32 = 20.0;
    pub const FADE_GROWTH: f32 = 1.02;
    /// MP cost (data `+0x30` = 100): one whole point.
    pub const COST: i32 = crate::mp::cost::GRAVITY;

    /// A level's (final scale, fall frames).
    pub fn of(level: u8) -> (f32, f32) {
        let i = level.clamp(1, 3) as usize - 1;
        (SCALE[i], FRAMES[i])
    }

    /// The impact's hit radius, Re:CoM metres (170 × S cm).
    pub fn impact_radius_m(level: u8) -> f32 {
        RADIUS * of(level).0 / 100.0
    }

    /// The sphere's scale `t` frames into the fall (linear, `sp+0x24` a frame).
    pub fn scale_at(level: u8, t: f32) -> f32 {
        let (s, n) = of(level);
        START_SCALE + (s - START_SCALE) * (t / n).clamp(0.0, 1.0)
    }
}

/// The falling thread's clock (`xm_gravity01.mag` `0x44..`): the release's run counts the first
/// frame; each later run counts one more until the N-th; the run after it is the impact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fall {
    pub left: f32,
    pub total: f32,
}

impl Fall {
    /// The release frame's run (the first of the fall).
    pub fn new(level: u8) -> Fall {
        let n = gravity::of(level).1;
        Fall { left: n - 1.0, total: n }
    }

    /// One later run: true when it is the impact (the fall's frames are over).
    pub fn run(&mut self, step: f32) -> bool {
        if self.left <= 0.0 {
            return true;
        }
        self.left -= step;
        false
    }

    /// Frames fallen so far.
    pub fn fallen(&self) -> f32 {
        self.total - self.left.max(0.0)
    }
}

/// KH1's Gravity attack data (btltbl `0x6468` / `0x64D8` / `0x6548`, Checked (files)): power 40 / 55
/// / 70 + **max MP × 5** (stat source 5), element 8 (the % of HP path), kind 8, struck directly.
pub fn gravity_attack(level: u8) -> AttackData {
    let power = [40, 55, 70][level.clamp(1, 3) as usize - 1];
    AttackData {
        stat_source: 5,
        element: 8,
        kind_party: 8,
        kind: 8,
        bone: -1,
        power_party: power,
        mp_charge: 0,
        clash: 6,
        priority: 0,
        recoil_request: 110,
        power,
        radius: 0.0,
        ..Default::default()
    }
}

/// Re:CoM's Gravity rows 0x26 / 0x38 / 0x39 as in the Steam exe (clip 0x3A, effect sets 18 / 19 /
/// 20 of `grav_all`, release `+3` 32, its sphere's radius 1.5 m, speed 0.05, kind 0xD). Checked
/// (exe).
pub const RECOM_GRAVITY_ROWS: [[u8; 16]; 3] = [
    [0x3A, 0x00, 0x12, 0x20, 0x96, 0x04, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0D, 0x08, 0x0A, 0x32, 0x00],
    [0x3A, 0x00, 0x13, 0x20, 0x96, 0x06, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0D, 0x08, 0x14, 0x32, 0x00],
    [0x3A, 0x00, 0x14, 0x20, 0x96, 0x08, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0D, 0x08, 0x1E, 0x32, 0x00],
];

/// The KH1 Gravity row: Re:CoM's row (clip, blend, `grav_all` set, its sphere's `+5` / `+0xD`:
/// decision 34) with `+3` = 0, **`+0xB` = 1** (Re:CoM's flinch: Re:CoM's kind 0xD is its own %-of-HP
/// damage with an integer % and a 3-second squash, `0x14017DC50` / state `+0xAC0`; KH1's number
/// is written as the hit's value instead) and `+0xF` = 0x80.
pub fn gravity_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[0xB] = 1;
    b[0xF] |= FLAG_KH1;
    b
}

/// KH1's Gravity on an enemy (`0x1402BFA80` element 8): current HP × (power + 5 × max MP)% ×
/// resist[8], at least the floor (1 when the floor is under 2).
pub fn gravity_damage(level: u8, max_mp: i32, hp: i32, resist8: f32, floor: u16) -> i32 {
    let def = DefenderStats { hp, max_hp: hp, min_damage: floor, hp_percent_scale: resist8, ..Default::default() };
    spell_damage(&gravity_attack(level), max_mp, &def, &mut Kh1Rand::new(1))
}

/// KH1's Stop (`xm_stop01..03.mag`, decoded with `magdis.py`: one script, the level passes the
/// radius 250 / 350 / 500; Checked (files), the syscalls in the KH1 decomp):
///
/// - the command (`L00890`): **wait 4**, the charge effect, then **each frame for 24 frames** the
///   release check (`0x3B` / `0x88`; else the effect fades and it ends: no spell, no MP); so the
///   release is at **28**; the thread; **the MP** (sys 47, 200 = two points);
/// - the thread (`0x44..0x206`): the centre is the target's part if alive, else the caster's
///   position; every enemy (up to 30) sorted nearest first from the centre (`L0040e`, the same
///   selection sort as Thunder's); down the list, an enemy whose bodies touch a vertical cylinder of
///   **250 / 350 / 500 cm, ±300 cm** (sys 90 type 2) gets the Stop effect and **one direct strike**
///   (sys 60), then **4 frames** (`L009a2(4)`); one outside is passed over at once.
///
/// The strike (`0x1402BEEE0` element 9): if the enemy's `resist[9]` > 0, it is stopped for
/// **(2 + max MP) × 60 × resist[9] frames** (power 2 + max MP, stat source 1; the same at every
/// level); `resist[9]` = 0: immune. While stopped KH1 queues the hits it takes and lands them when
/// Stop ends, and the stopped actor's own attacks are skipped (`0x140297FE0`, `0x1402B47A0`).
/// Checked (files + decomp).
pub mod stop {
    pub const WAIT: f32 = 28.0;
    /// The area: radius by level, half-height, cm.
    pub const RADIUS: [f32; 3] = [250.0, 350.0, 500.0];
    pub const HALF_HEIGHT: f32 = 300.0;
    /// The wait after each strike, frames.
    pub const INTERVAL: f32 = 4.0;
    /// The power (data `+0x48`, all levels) and the frames per point.
    pub const POWER: i32 = 2;
    pub const FRAMES_PER_POINT: f32 = 60.0;
    /// MP cost (data `+0x30` = 200): two whole points.
    pub const COST: i32 = crate::mp::cost::STOP;

    /// The area of a level (Re:CoM metres).
    pub fn area(level: u8) -> super::Area {
        super::Area::Cylinder { radius: RADIUS[level.clamp(1, 3) as usize - 1] / 100.0, half: HALF_HEIGHT / 100.0 }
    }

    /// KH1's Stop duration in frames for a `resist[9]` (0 = immune: None).
    pub fn frames(max_mp: i32, resist9: f32) -> Option<f32> {
        (resist9 > 0.0).then(|| (POWER + max_mp) as f32 * FRAMES_PER_POINT * resist9)
    }

    /// The duration as Re:CoM's stop timer (`enemy+0x2D8` → the stop state's `+0x160`, ticks: 50
    /// a frame), carried as the hit's value (an i16: at most 32767; KH1's longest is 600 frames =
    /// 30000).
    pub fn ticks(max_mp: i32, resist9: f32) -> Option<i16> {
        frames(max_mp, resist9).map(|f| (f * 50.0).round().clamp(1.0, i16::MAX as f32) as i16)
    }
}

/// KH1's Stop attack data (btltbl `0x65B8` / `0x6628` / `0x6698`, Checked (files)): power 2 + max MP
/// (stat source 1), element 9 (the damage returns the power: the duration's points), kind 0.
pub fn stop_attack(_level: u8) -> AttackData {
    AttackData {
        stat_source: 1,
        element: 9,
        kind_party: 0,
        kind: 0,
        bone: -1,
        power_party: stop::POWER,
        mp_charge: 0,
        clash: 6,
        priority: 0,
        recoil_request: 110,
        power: stop::POWER,
        radius: 0.0,
        ..Default::default()
    }
}

/// Re:CoM's Stop rows 0x27 / 0x3A / 0x3B as in the Steam exe (clip 0x39, effect sets 21 / 22 / 23 of
/// `stop_all`, kind 8, its shot's radius / range / life). Checked (exe).
pub const RECOM_STOP_ROWS: [[u8; 16]; 3] = [
    [0x39, 0x00, 0x15, 0x1E, 0x05, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x08, 0x08, 0x00, 0x08, 0x00],
    [0x39, 0x00, 0x16, 0x1E, 0x14, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x08, 0x08, 0x00, 0x08, 0x00],
    [0x39, 0x00, 0x17, 0x1E, 0x28, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x08, 0x08, 0x00, 0x08, 0x00],
];

/// The KH1 Stop row: Re:CoM's row (clip, `stop_all` set, its shot: decision 34) with `+3` = 0 and
/// `+0xF` = 0x80. **`+0xB` stays 8**: Re:CoM's own stop kind, which puts the enemy in Re:CoM's stop
/// state (`0x14017D260` case 8 → `+0x940`) for the hit's value in ticks (`0x14017DC50` case 8).
pub fn stop_row_bytes(recom: &[u8; 16]) -> [u8; 16] {
    let mut b = *recom;
    b[3] = 0;
    b[0xF] |= FLAG_KH1;
    b
}

/// Re:CoM's stop kind (`0x14017DC50` case 8 / `0x14017D260` case 8). Checked (decomp).
pub const RECOM_STOP_KIND: u32 = 8;

/// An area KH1's sys 90 tests a body against (Re:CoM metres): a vertical cylinder (type 2: radius,
/// half-height) or a sphere (type 1: radius).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Area {
    Cylinder { radius: f32, half: f32 },
    Sphere { radius: f32 },
}

/// The distance from a point to an area around `centre` (0 inside).
fn area_distance(p: [f32; 3], centre: [f32; 3], a: Area) -> f32 {
    match a {
        Area::Cylinder { radius, half } => cylinder_distance(p, centre, radius, half),
        Area::Sphere { radius } => {
            let d = [p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]];
            ((d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() - radius).max(0.0)
        }
    }
}

/// Whether a hurt body touches an area (its shape's nearest point within its radius).
pub fn body_touches(centre: [f32; 3], area: Area, body: &BodyShape) -> bool {
    match *body {
        BodyShape::Sphere { c, r } => area_distance(c, centre, area) <= r,
        BodyShape::Column { a, b, r } => {
            let at = |t: f32| area_distance([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t], centre, area);
            let (mut lo, mut hi) = (0.0f32, 1.0f32);
            for _ in 0..40 {
                let (m1, m2) = (lo + (hi - lo) / 3.0, hi - (hi - lo) / 3.0);
                if at(m1) <= at(m2) {
                    hi = m2;
                } else {
                    lo = m1;
                }
            }
            at(0.0).min(at(1.0)).min(at((lo + hi) * 0.5)) <= r
        }
    }
}

/// A Re:CoM hurt body's shape as the area test sees it (metres): a sphere (kind 2: centre `+0x20`,
/// its y + `+0x74`, radius `+0x70`) or a column (kinds 4 / 5: from `+0x10` to `+0x20`, each y +
/// `+0x74`, radius `+0x70`), as Re:CoM's own primitives `0x1401777E0` / `0x140176FA0` read them.
/// Checked (decomp).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyShape {
    Sphere { c: [f32; 3], r: f32 },
    Column { a: [f32; 3], b: [f32; 3], r: f32 },
}

/// The distance from a point to KH1's area cylinder (0 inside).
fn cylinder_distance(p: [f32; 3], centre: [f32; 3], radius: f32, half: f32) -> f32 {
    let (dx, dz) = (p[0] - centre[0], p[2] - centre[2]);
    let h = ((dx * dx + dz * dz).sqrt() - radius).max(0.0);
    let v = ((p[1] - centre[1]).abs() - half).max(0.0);
    (h * h + v * v).sqrt()
}

/// Whether a hurt body overlaps KH1's Thunder area (the vertical cylinder, Re:CoM metres): its
/// shape's nearest point to the cylinder within its radius. The distance from a segment's points
/// to a convex solid is convex along it, so a ternary search finds the column's nearest point.
pub fn body_in_area(centre: [f32; 3], body: &BodyShape) -> bool {
    body_touches(centre, Area::Cylinder { radius: thunder::RADIUS / 100.0, half: thunder::HALF_HEIGHT / 100.0 }, body)
}

/// KH1's strike list (Thunder's `L0069c`, Stop's `L0040e`): the first 30 enemies with their
/// distance from `from`, sorted nearest first by KH1's selection sort (a later entry replaces the
/// minimum only when strictly nearer, then swaps).
pub fn thunder_order(from: [f32; 3], enemies: impl IntoIterator<Item = (u32, [f32; 3])>) -> Vec<u32> {
    let mut l: Vec<(u32, f32)> = enemies
        .into_iter()
        .take(thunder::MAX)
        .map(|(id, p)| {
            let d = [p[0] - from[0], p[1] - from[1], p[2] - from[2]];
            (id, (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt())
        })
        .collect();
    for i in 0..l.len() {
        let mut m = i;
        for j in i + 1..l.len() {
            if l[j].1 - l[m].1 < 0.0 {
                m = j;
            }
        }
        if m != i {
            l.swap(i, m);
        }
    }
    l.into_iter().map(|e| e.0).collect()
}

/// The strike thread's state (KH1's script; Re:CoM has no place for it).
#[derive(Clone, Debug, PartialEq)]
pub struct Strikes {
    pub order: Vec<u32>,
    pub next: usize,
    /// `L00c8e(8)`'s countdown (Stop: `L009a2(4)`).
    pub wait: f32,
    pub struck: u32,
    /// The wait after each strike (Thunder 8, Stop 4).
    pub interval: f32,
}

impl Strikes {
    pub fn new(order: Vec<u32>) -> Strikes {
        Strikes::every(order, thunder::INTERVAL)
    }

    /// A strike list with its own wait after each strike.
    pub fn every(order: Vec<u32>, interval: f32) -> Strikes {
        Strikes { order, next: 0, wait: 0.0, struck: 0, interval }
    }

    /// One run of the thread: a run inside the wait counts it down; otherwise the next listed
    /// enemy `inside` the area is struck (returned) and the wait starts, counting this run (KH1's
    /// `L00c8e` decrements once before it yields); enemies outside are passed over in the same
    /// run. So strikes come 8 runs apart.
    pub fn run(&mut self, step: f32, mut inside: impl FnMut(u32) -> bool) -> Option<u32> {
        if self.wait > 0.0 {
            self.wait -= step;
            return None;
        }
        while let Some(&id) = self.order.get(self.next) {
            self.next += 1;
            if inside(id) {
                self.wait = self.interval - step;
                self.struck += 1;
                return Some(id);
            }
        }
        None
    }

    /// The list is done (the thread's last wait over).
    pub fn done(&self) -> bool {
        self.next >= self.order.len() && self.wait <= 0.0
    }
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

/// Re:CoM's rows of the spells built so far, by spell (`[spell][level − 1]`), as in the Steam exe:
/// the templates of the KH1 rows.
pub fn recom_rows() -> Vec<[[u8; 16]; 3]> {
    vec![RECOM_FIRE_ROWS, RECOM_BLIZZARD_ROWS, RECOM_THUNDER_ROWS, RECOM_CURE_ROWS, RECOM_GRAVITY_ROWS, RECOM_STOP_ROWS]
}

/// The KH1 magic rows, `[spell × 3 + level − 1]` (21; the spells built so far), from Re:CoM's rows
/// by spell (which must be [`recom_rows`]).
pub fn kh1_magic_table(recom: &[[[u8; 16]; 3]]) -> Vec<[u8; 16]> {
    let mut t = vec![[0u8; 16]; 21];
    for (s, rows) in recom.iter().enumerate().take(7) {
        for (i, r) in rows.iter().enumerate() {
            let lv = i as u8 + 1;
            t[s * 3 + i] = match s as u8 {
                spell::FIRE => fire_row_bytes(r),
                spell::BLIZZARD => blizzard_row_bytes(r, lv),
                spell::THUNDER => thunder_row_bytes(r),
                spell::CURE => cure_row_bytes(r),
                spell::GRAVITY => gravity_row_bytes(r),
                spell::STOP => stop_row_bytes(r),
                _ => [0; 16],
            };
        }
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
/// release `0x3D` (entry 57, 60 frames, free from 46, root −15 cm over frames 0–10, then 5 cm
/// forward to −10 cm at frame 60: linear keys, read with `kh1_mset.py`'s root helper), air
/// wind-up `0x83` (entry 79, 28) and air release `0x8A` (entry 82, 52 frames, free from 46, root
/// −42.4 cm over 0–10). Re:CoM's clips `p001_062`, `p001_063`, `p001_066`, `p001_067` (rows 0x37,
/// 0x38, 0x3B, 0x3C) have the same frame counts at 60 Hz (14, 30, 14, 26 at 30 fps; research
/// `m7-magic.md` § 3.5, Checked (files)).
pub fn add_motions(m: &mut MotionSet) {
    let inf = f32::INFINITY;
    m.add(&[cast_req::FIRE_WINDUP], MotionInfo::new(28.0));
    m.add(&[cast_req::FIRE_RELEASE], MotionInfo::new(60.0).with_flag(FlagKind::Free, 46.0, inf).with_forward_keys(&[(0.0, 0.0), (10.0, -15.0), (60.0, -10.0)]));
    m.add(&[cast_req::FIRE_AIR_WINDUP], MotionInfo::new(28.0));
    m.add(&[cast_req::FIRE_AIR_RELEASE], MotionInfo::new(52.0).with_flag(FlagKind::Free, 46.0, inf).with_forward_keys(&[(0.0, 0.0), (10.0, -42.4096), (52.0, -42.4096)]));
    // Blizzard (7.5): entry 55, 86 frames, free from 68, no root travel; air entry 80, 78 frames,
    // free from 68. Re:CoM's `p001_065` / `p001_069` (rows 0x3A / 0x3E): 43 / 39 frames at 30 fps.
    m.add(&[cast_req::BLIZZARD], MotionInfo::new(86.0).with_flag(FlagKind::Free, 68.0, inf));
    m.add(&[cast_req::BLIZZARD_AIR], MotionInfo::new(78.0).with_flag(FlagKind::Free, 68.0, inf));
    // Thunder (7.6): entry 56, 68 frames, no free key (locked to its end), no root travel; air
    // entry 81, 68. Re:CoM's `p001_064` / `p001_068` (rows 0x39 / 0x3D): 34 frames at 30 fps.
    m.add(&[cast_req::THUNDER], MotionInfo::new(68.0));
    m.add(&[cast_req::THUNDER_AIR], MotionInfo::new(68.0));
    // Cure (7.7): Thunder's entries 56 / 81 (68 frames, locked to the end, no root travel). Checked
    // (files: requests 0x39 / 0x86 map to them).
    m.add(&[cast_req::CURE], MotionInfo::new(68.0));
    m.add(&[cast_req::CURE_AIR], MotionInfo::new(68.0));
    // Gravity (7.8): Blizzard's entries 55 / 80 (86 / 78 frames, free from 68, no root travel).
    // Checked (files: requests 0x3A / 0x87 map to them).
    m.add(&[cast_req::GRAVITY], MotionInfo::new(86.0).with_flag(FlagKind::Free, 68.0, inf));
    m.add(&[cast_req::GRAVITY_AIR], MotionInfo::new(78.0).with_flag(FlagKind::Free, 68.0, inf));
    // Stop (7.9): Thunder's entries 56 / 81 (68 frames, locked to the end). Checked (files: requests
    // 0x3B / 0x88 map to them).
    m.add(&[cast_req::STOP], MotionInfo::new(68.0));
    m.add(&[cast_req::STOP_AIR], MotionInfo::new(68.0));
}

/// Whether a KH1 request is one of the cast motions Re:CoM's magic action plays.
pub fn is_cast_request(r: u16) -> bool {
    (0..7u8).filter(|&s| castable(s)).any(|s| [false, true].iter().any(|&a| r == cast_req::first(s, a) || r == cast_req::release(s, a)))
}

/// Fire's wind-up (its script waits while it plays).
fn is_windup(r: u16) -> bool {
    r == cast_req::FIRE_WINDUP || r == cast_req::FIRE_AIR_WINDUP
}

/// Fire's release motion (the Fire re-cast and fast path read it).
fn is_release(r: u16) -> bool {
    r == cast_req::FIRE_RELEASE || r == cast_req::FIRE_AIR_RELEASE
}

/// The motion a spell's script requires at its release check, ground or air.
fn is_release_of(s: u8, r: u16) -> bool {
    r == cast_req::release(s, false) || r == cast_req::release(s, true)
}

/// The script's wait from the command to its release check (Fire's wind-up path waits on the
/// motion instead: 0).
pub fn release_wait(s: u8) -> f32 {
    match s {
        spell::BLIZZARD => blizzard::WAIT,
        spell::THUNDER => thunder::WAIT,
        spell::CURE => cure::WAIT,
        spell::GRAVITY => gravity::WAIT,
        // Stop: `L009a2(4)`, then the check each frame for 24: the release at 28 (a cut motion
        // before it ends the cast through `leave_cast` / the resync, as KH1's per-frame check).
        spell::STOP => stop::WAIT,
        _ => 0.0,
    }
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
            } else if c.wait > 0.0 {
                t.push_str(&format!(" w{:.0}", c.wait));
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
            // `0x1402A3600` state 2: `0x83 + spell` (Fire: then `0x8A`), the fall (blend 8).
            self.play(cast_req::first(m.spell, true) as u32 | req::ACTION, 0.0);
            if m.spell == spell::FIRE {
                self.append(FIRE_AIR_RELEASE as u32, 0.0);
            }
            self.append(req::RELEASE | request::FALL as u32, AIR_FALL_BLEND);
            // `0x1402AF960(actor, 30)`, then `+0x3C0` × 0.5.
            self.hop(AIR_HOP, pos.y);
            self.air_vel = self.air_vel * AIR_SPEED_SCALE;
        } else {
            // `0x1402A3DB0`: `0x36 + spell` (Fire: then `0x3D`), then idle.
            self.play(cast_req::first(m.spell, false) as u32 | req::ACTION, 0.0);
            if m.spell == spell::FIRE {
                self.append(FIRE_RELEASE as u32, 0.0);
            }
            self.append(req::RELEASE | request::IDLE as u32, 0.0);
        }
        let wait = if fast { fire::FAST_WAIT } else { release_wait(m.spell) };
        self.cast = Some(Cast { start, wait, age: 0.0, released: false, cancelled: false, shot: None, fresh: true });
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
        // The fast path's `L006fa(8)`, Blizzard's `L0086e(28)`: count down, a frame at a time.
        if c.wait > 0.0 {
            c.wait -= step;
            return;
        }
        if c.start.spell == spell::FIRE && !c.start.fast && is_windup(head) {
            return;
        }
        let s = c.start;
        if !is_release_of(s.spell, head) {
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
