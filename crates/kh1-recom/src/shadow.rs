//! M10, KH1's Shadow in Re:CoM's battles (`docs/recom-m10-shadow.md`): the testable part. The
//! runtime is `win/shadow.rs`; the behaviour itself is `kh1_core::shadow` (engine-agnostic).
//!
//! At Combat = KH1 in Sora's story with `enemy_ai = kh1`, each battle `CEnemyShadow` gets a copy of
//! its own vtable whose state slots run KH1's Shadow (decision M10 (a): native states):
//!
//! - state 0 (appearance): Re:CoM's own begin / end and its battle-start handshake; KH1's rise;
//! - state 1: KH1's AI and body each frame (the behaviour polled once per frame, paused while an
//!   action motion plays, as KH1's script is);
//! - states 2 / 3 / 4: the claw, the leap claw, the swipe: Re:CoM attack states (entered as P1 enters
//!   one), KH1's motions with the new attack records of `EN0001.ASD` on them;
//! - states 11 / 12: KH1's reactions (flinch, knock-back, launch, air reactions, landing, lying,
//!   get-up) for every hit Re:CoM's hit handler sends there, and KH1's clash recoil for a Guard;
//! - state 30: Re:CoM's defeat (prizes, EXP, removal) with the killing hit's KH1 reaction motion;
//! - the think slot `+0x3D0`: nothing (Re:CoM's think never picks a card attack for it).
//!
//! Status states (Stop, freeze, Gravity's flatten, first-strike stun) stay Re:CoM's.

use crate::json;
use kh1_core::shadow::{self as ks, Attack, Cmd, Consts};
use kh1_core::Vec3;

/// The asset step's manifest and its format (`kh1_recom_assets::shadow::FORMAT`).
pub const MANIFEST: &str = "kh1-shadow.json";
pub const FORMAT: &str = "kh1-shadow 1";

/// `CEnemyShadow`'s vtable and its slot count (Checked (exe): RTTI COL at `vt − 8`, the next
/// class's COL after 139 slots).
pub const VTABLE: u32 = 0x0054_D728;
pub const VTABLE_SLOTS: usize = 139;

/// The enemy states our copy runs (`enemy + 0x4C0 + k × 0x40`; begin / step / end at vtable
/// `+0xE8 + 0x18 k` / `+8` / `+0x10`, called through the state objects' thunks: Checked (decomp:
/// `0x14017AF00`, the member-pointer tables `0x14087A910` / `0x14087AB00` / `0x14087ACF0`)).
pub mod state {
    pub const APPEAR: i32 = 0;
    pub const THINK: i32 = 1;
    pub const CLAW: i32 = 2;
    pub const LEAP: i32 = 3;
    pub const SWIPE: i32 = 4;
    pub const FLINCH: i32 = 11;
    pub const BLOW: i32 = 12;
    pub const DEFEAT: i32 = 30;
}

/// The vtable offset of state `k`'s begin, step, end.
pub const fn begin_slot(k: i32) -> usize {
    0xE8 + 0x18 * k as usize
}
pub const fn step_slot(k: i32) -> usize {
    begin_slot(k) + 8
}
pub const fn end_slot(k: i32) -> usize {
    begin_slot(k) + 0x10
}
/// The think (`0x140196EC0`, Re:CoM's random card attack pick).
pub const THINK_SLOT: usize = 0x3D0;

/// The Re:CoM attack state each KH1 attack runs in.
pub fn attack_state(a: Attack) -> i32 {
    match a {
        Attack::Claw => state::CLAW,
        Attack::LeapClaw => state::LEAP,
        Attack::Swipe => state::SWIPE,
    }
}

/// Our 0x20-byte tail after `CEnemyShadow` (the factory's `mov ecx, 0xCE0` at `0x140188FAD`
/// raised, as the revenge bosses' sizes): magic, battle number, the KH1 runtime's index.
pub mod tail {
    pub const AT: usize = 0xCE0;
    pub const SIZE: u32 = 0x20;
    pub const MAGIC: u32 = 0x4853_314B; // "K1SH"
    pub const OFF_MAGIC: usize = 0;
    pub const OFF_BATTLE: usize = 4;
    pub const OFF_INDEX: usize = 8;
}
/// Re:CoM's allocation size for `CEnemyShadow` (Checked (exe): `0x140188FAD` `B9 E0 0C 00 00`).
pub const ALLOC: u32 = 0xCE0;

pub fn alloc_bytes(size: u32) -> [u8; 5] {
    let s = size.to_le_bytes();
    [0xB9, s[0], s[1], s[2], s[3]]
}

/// One request's clip in Re:CoM's `EN0001.ANN`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clip {
    pub request: u16,
    pub slot: u32,
    /// KH1's length (60 Hz frames).
    pub frames: u32,
    pub root: bool,
    pub looped: bool,
}

/// `kh1-shadow.json`.
#[derive(Clone, Debug)]
pub struct Manifest {
    pub consts: Consts,
    pub clips: Vec<Clip>,
    /// The clash recoil request of each attack row (claw, swipe, leap claw right, left).
    pub recoil: [u16; 4],
}

impl Manifest {
    pub fn clip(&self, r: u16) -> Option<&Clip> {
        self.clips.iter().find(|c| c.request == r)
    }

    /// `sys 43`'s motion lengths for the behaviour.
    pub fn frames(&self) -> Vec<(u16, f32)> {
        self.clips.iter().map(|c| (c.request, c.frames as f32)).collect()
    }

    /// The clash recoil of an attack (`0x1402B9140` plays the row's `+0x40`).
    pub fn recoil_of(&self, a: Attack) -> u16 {
        match a {
            Attack::Claw => self.recoil[0],
            Attack::Swipe => self.recoil[1],
            Attack::LeapClaw => self.recoil[2],
        }
    }
}

impl ks::Motions for Manifest {
    fn has(&self, request: u16) -> bool {
        self.clip(request).is_some()
    }
}

pub fn parse(text: &str) -> Result<Manifest, String> {
    let v = json::parse(text)?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(format!("not a \"{FORMAT}\" manifest (an older asset step? run it again)"));
    }
    let json::Value::Obj(c) = v.get("consts").ok_or("no \"consts\"")? else { return Err("\"consts\" is not an object".into()) };
    let mut pairs = Vec::new();
    for (k, x) in c {
        let f = x.as_f64().ok_or_else(|| format!("const {k} is not a number"))? as f32;
        if !f.is_finite() {
            return Err(format!("const {k} = {f}"));
        }
        pairs.push((k.clone(), f));
    }
    let consts = Consts::new(pairs)?;
    let list = v.get("requests").and_then(|a| a.as_array()).ok_or("no \"requests\"")?;
    let mut clips = Vec::with_capacity(list.len());
    for r in list {
        let n = |k: &str| r.get(k).and_then(|x| x.as_u32()).ok_or_else(|| format!("request entry without \"{k}\""));
        let b = |k: &str| match r.get(k) {
            Some(json::Value::Bool(b)) => Ok(*b),
            _ => Err(format!("request entry without \"{k}\"")),
        };
        let request = n("request")?;
        let slot = n("slot")?;
        if request > 0xFFFF || slot >= 1000 {
            return Err(format!("request {request} slot {slot}"));
        }
        clips.push(Clip { request: request as u16, slot, frames: n("frames")?, root: b("root")?, looped: b("loop")? });
    }
    let rec = v.get("recoil").and_then(|a| a.as_array()).ok_or("no \"recoil\"")?;
    if rec.len() != 4 {
        return Err(format!("\"recoil\" has {} entries, not 4", rec.len()));
    }
    let mut recoil = [0u16; 4];
    for (i, x) in rec.iter().enumerate() {
        recoil[i] = x.as_u32().filter(|&r| r <= 0xFFFF).ok_or("bad recoil")? as u16;
    }
    let m = Manifest { consts, clips, recoil };
    // What the behaviour plays must be there.
    for r in [ks::req::IDLE, ks::req::WALK, ks::req::RUN, ks::req::JUMP, ks::req::FALL, ks::req::LAND, ks::req::FLINCH, ks::req::RISE, ks::req::SINK, ks::req::CLAW, ks::req::LEAP_CLAW, ks::req::UNDERGROUND, ks::req::TAUNT, ks::req::SWIPE] {
        if m.clip(r).is_none() {
            return Err(format!("no clip for KH1 request {r}"));
        }
    }
    for r in m.recoil {
        if m.clip(r).is_none() {
            return Err(format!("no clip for the clash recoil {r}"));
        }
    }
    Ok(m)
}

/// Re:CoM's reaction kind of a hit (the hit attack's `+0x80`, the picker `0x14017D260`'s argument)
/// → KH1's enemy reaction kind, for hits that are not a KH1 swing: 1 (flinch) → 3, the blow-away
/// kinds (2, 3, 0x12, 0x1D, 0x1E; Re:CoM's picker sends them to state 12) → 5 (launch).
/// Assumption: the nearest KH1 kind; status kinds never reach states 11 / 12.
pub fn kh1_kind_of_recom(kind: u32) -> u8 {
    match kind {
        2 | 3 | 0x12 | 0x1D | 0x1E => 5,
        _ => 3,
    }
}

/// The mover velocity (Re:CoM units per 60 Hz frame) that moves a body by `d` over a step of `t`
/// frames with the mover's acceleration `a` (`0x1401D4CF0`: `pos += v t + a t² / 2`).
pub fn velocity(d: Vec3, a: Vec3, t: f32) -> Vec3 {
    if t <= 0.0 {
        return Vec3::ZERO;
    }
    d * (1.0 / t) - a * (t * 0.5)
}

/// KH1's hurt part a contact point is nearer to: 0 the head (KH1's part on bone 30), 1 the hips
/// (bone 2). KH1 tells its parts apart by the part record that was hit; Re:CoM's Shadow has one
/// hurt body (decision 24 keeps Re:CoM's shapes), so the contact point's distance to each part's
/// bone decides (decision M10: "flinch by hurt part derived from the hit's height").
pub fn part_of(contact: Vec3, head: Vec3, hips: Vec3) -> u8 {
    if (contact - head).length() <= (contact - hips).length() {
        0
    } else {
        1
    }
}

/// KH1's push direction for Sora's swing (`0x1402BF0D0`'s record `+0x10`, `kh1_core::combat`):
/// toward the enemy, turned at most `clamp` from Sora's facing.
pub fn swing_dir(sora: Vec3, sora_yaw: f32, enemy: Vec3, clamp: f32) -> Vec3 {
    use kh1_core::math::{clamp_sym, wrap_angle};
    let to = (enemy - sora).flat().normalized();
    let yaw = wrap_angle(clamp_sym(wrap_angle(to.yaw() - sora_yaw), clamp) + sora_yaw);
    Vec3::from_yaw(yaw)
}

/// Blend frames → Re:CoM's motion clock units (50 a frame).
pub fn blend_units(frames: f32) -> i32 {
    crate::units::frames_to_units(frames)
}

/// A behaviour command as a log token.
pub fn cmd_token(c: &Cmd) -> String {
    match c {
        Cmd::Motion { request, blend } => format!("motion {request} b{blend}"),
        Cmd::Queue { request, blend } => format!("queue {request} b{blend}"),
        Cmd::Action { request, attack, then, .. } => format!("action {request}{} then {then}", attack.map_or(String::new(), |a| format!(" ({})", a.name()))),
        Cmd::Yaw(y) => format!("yaw {:.2}", y),
        Cmd::Jump { height, velocity } => format!("jump h{height} v({:.1},{:.1})", velocity.x, velocity.z),
        Cmd::Underground(u) => format!("underground {u}"),
        Cmd::Hittable(h) => format!("hittable {h}"),
        Cmd::Height(h) => format!("height {h:.2}"),
        Cmd::FadeIn(f) => format!("fade in {f}"),
        Cmd::Target(t) => format!("target {t}"),
        Cmd::State(s) => format!("state {}", ["idle", "engaged", "hit"].get(*s as usize).unwrap_or(&"?")),
        // M11's shared commands; the Shadow never gives them.
        Cmd::Held => "held".into(),
        Cmd::Attack { request, attack, .. } => format!("attack {request} ({})", attack.name()),
        Cmd::AttackAt { request, attack, start, .. } => format!("attack {request} ({}) from {start}", attack.name()),
        Cmd::Scale(k) => format!("scale {k:.2}"),
        Cmd::Visible(v) => format!("visible {v}"),
        Cmd::MotionAt { request, blend, start } => format!("motion {request} b{blend} from {start}"),
        Cmd::Guard(g) => format!("guard {g:?}"),
        Cmd::Throw(n) => format!("throw {n}"),
        Cmd::Note(n) => format!("note: {n}"),
        Cmd::HealFull => "heal full".into(),
        Cmd::Support { child, draw } => format!("support {child} draw {draw}"),
        Cmd::Reward(r) => format!("reward {r:?}"),
        Cmd::QueueAt { request, start, .. } => format!("queue {request} from {start}"),
        Cmd::Strike { child, at } => format!("strike {child} at {at:?}"),
        Cmd::Warn { child, at } => format!("warn {child} at {at:?}"),
        Cmd::Restart { request, blend, start } => format!("motion {request} b{blend} again from {start}"),
    }
}

/// Whether a command is worth a log line (state changes, actions, jumps, underground; not the
/// per-frame walk / yaw / height).
pub fn logged(c: &Cmd) -> bool {
    matches!(c, Cmd::Action { .. } | Cmd::Jump { .. } | Cmd::Underground(_) | Cmd::Hittable(_) | Cmd::State(_) | Cmd::Target(_) | Cmd::FadeIn(_)) || matches!(c, Cmd::Motion { request, .. } if *request != ks::req::WALK && *request != ks::req::RUN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_text() -> String {
        let consts: Vec<String> = Consts::names().iter().map(|n| format!("\"{n}\": 1.0")).collect();
        let reqs = [0u16, 1, 2, 3, 4, 6, 7, 0x48, 0x49, 0x4A, 0x4B, 69, 70, 200, 201, 202, 203, 204, 207, 208];
        let r: Vec<String> = reqs.iter().map(|q| format!("{{\"request\": {q}, \"entry\": 0, \"slot\": {}, \"name\": \"e001_100\", \"frames\": 30, \"root\": false, \"loop\": false, \"source\": \"kh1\"}}", 100 + q % 50)).collect();
        format!("{{\"format\": \"kh1-shadow 1\", \"consts\": {{{}}}, \"recoil\": [207, 207, 208, 208], \"requests\": [{}]}}", consts.join(", "), r.join(", "))
    }

    #[test]
    fn manifest_parses_and_checks() {
        let m = parse(&manifest_text()).unwrap();
        assert_eq!(m.clip(200).unwrap().frames, 30);
        assert_eq!(m.recoil_of(Attack::LeapClaw), 208);
        assert!(ks::Motions::has(&m, 0x48) && !ks::Motions::has(&m, 0x50));
        assert!(parse(&manifest_text().replace("\"kh1-shadow 1\"", "\"kh1-shadow 0\"")).is_err());
        assert!(parse(&manifest_text().replace("\"close_range\": 1.0, ", "")).is_err());
        assert!(parse(&manifest_text().replace("{\"request\": 204,", "{\"request\": 1204,")).is_err());
    }

    /// The asset step's real manifest (`KH1_SHADOW_JSON`: a `kh1-shadow.json` built from the
    /// player's files) parses here.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_SHADOW_JSON") else { return };
        let m = parse(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!(m.clip(6).unwrap().slot, 5, "entry 6 is Re:CoM's bake e001_005");
        assert_eq!(m.clip(200).unwrap().slot, 145);
        assert_eq!(m.recoil, [207, 207, 208, 208]);
        assert!(!ks::Motions::has(&m, 0x50), "the Shadow has no air knock-back");
        assert_eq!(m.consts.close_range(), 450.0);
    }

    #[test]
    fn slots_and_states() {
        assert_eq!((begin_slot(0), step_slot(1), end_slot(2)), (0xE8, 0x108, 0x128));
        assert_eq!((begin_slot(11), begin_slot(12), begin_slot(30)), (0x1F0, 0x208, 0x3B8));
        assert_eq!(attack_state(Attack::Swipe), 4);
        assert_eq!(alloc_bytes(ALLOC), [0xB9, 0xE0, 0x0C, 0, 0]);
        assert_eq!(alloc_bytes(ALLOC + tail::SIZE), [0xB9, 0x00, 0x0D, 0, 0]);
        assert_eq!(tail::AT as u32, ALLOC);
    }

    #[test]
    fn velocity_cancels_gravity() {
        let a = Vec3::new(0.0, -0.01, 0.0);
        let d = Vec3::new(0.05, 0.02, 0.0);
        let v = velocity(d, a, 1.0);
        // pos += v t + a t² / 2 gives d.
        let moved = v + a * 0.5;
        assert!((moved - d).length() < 1e-6);
        assert_eq!(velocity(d, a, 0.0), Vec3::ZERO);
    }

    #[test]
    fn kinds_parts_dirs() {
        assert_eq!((kh1_kind_of_recom(1), kh1_kind_of_recom(2), kh1_kind_of_recom(0x1E)), (3, 5, 5));
        let head = Vec3::new(0.0, 1.0, 0.0);
        let hips = Vec3::new(0.0, 0.4, 0.0);
        assert_eq!(part_of(Vec3::new(0.1, 0.9, 0.0), head, hips), 0);
        assert_eq!(part_of(Vec3::new(0.1, 0.5, 0.0), head, hips), 1);
        // Enemy 90° to Sora's right: the push turns only 45° from his facing.
        let d = swing_dir(Vec3::ZERO, 0.0, Vec3::new(100.0, 0.0, 0.0), std::f32::consts::FRAC_PI_4);
        assert!((d.yaw() - std::f32::consts::FRAC_PI_4).abs() < 1e-5);
        assert_eq!(blend_units(8.0), 400);
    }

    #[test]
    fn log_tokens() {
        assert!(logged(&Cmd::State(1)) && !logged(&Cmd::Yaw(0.0)) && !logged(&Cmd::Motion { request: 1, blend: 8.0 }));
        assert_eq!(cmd_token(&Cmd::Action { request: 200, attack: Some(Attack::Claw), then: 203, blend: 8.0 }), "action 200 (claw) then 203");
    }
}
