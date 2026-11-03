//! The shared KH1 enemy framework (M11, `docs/recom-m11-enemies.md`): what KH1's engine does the
//! same way for every ordinary Heartless, taken out of the Shadow's port (M10) so each enemy's own
//! module holds only its own AI script.
//!
//! - [`body`]: the actor's movement physics (turning, the walk the script set, the air arc and its
//!   horizontal velocity, the push slide), from the enemy update `0x1402B7260`.
//! - [`react`]: KH1's hit reactions (`0x1402B8E10` and its cases: flinch, knock-back, launch, air
//!   reactions, landing, lying down, get-up, the clash recoil), one rule set for every enemy;
//!   what differs per enemy is only which motions its model has ([`react::Motions`]).
//! - [`script`]: the AI VM's shape (one behaviour thread polled once a frame, paused while an
//!   action motion plays: `0x1402CC620`, `0x1402BCEC0`, `0x140297870`) as Rust `async` code, the
//!   commands it gives the host ([`script::Cmd`]) and the helper routines KH1's enemy scripts
//!   share byte for byte (move toward, walk to, approach, random ranges and offsets, the frame
//!   waits, facing); each cites the script offsets it was read from in the Shadow's `ex_2020.bd`
//!   and the Soldier's `ex_2010.bd` (Checked (files): the 26 byte-identical routines of the two
//!   scripts, `docs/recom-m11-enemies.md` § Shared script routines).
//! - [`host`]: the host's rules for a running action (the DLL's states call them; the behaviours'
//!   tests drive a behaviour through them).
//! - [`Behaviour`]: the one interface the host (`kh1-recom`'s `win::kh1_enemy`) drives every KH1
//!   enemy through.
//!
//! Every number is data ([`Numbers`]): the asset step reads it from the player's files; KH1's exe
//! values it can't read from a file are listed with their addresses (Checked (decomp + exe)).

pub mod body;
pub mod guard;
pub mod host;
pub mod react;
pub mod script;
pub mod thrown;

pub use body::Body;
pub use react::{get_up, landing, react, react_flying, Hit, Motions, Phase, ReactionStart};
pub use script::{Cmd, Draw, Reward, World};

/// Whether a behaviour's body is in KH1's flight (movement state 4, [`Body::flying`]).
pub fn flying(b: &dyn Behaviour) -> bool {
    b.body().flying
}

use crate::math::Vec3;
use crate::rng::Kh1Rand;

/// Named numbers (KH1 units, 60 Hz frames): a behaviour's constants by name.
pub trait Numbers {
    /// The number called `name` (NaN when missing; [`Table::new`] checks the names).
    fn num(&self, name: &str) -> f32;
}

/// The numbers the shared engine code reads, by name (the same names in every enemy's
/// manifest). Implemented for every [`Numbers`].
pub trait Engine: Numbers {
    /// The stats block's walk / run speeds (`+4` / `+8`, set by each script's init).
    fn walk_speed(&self) -> f32 {
        self.num("walk_speed")
    }
    fn run_speed(&self) -> f32 {
        self.num("run_speed")
    }
    /// KH1's stats-block defaults (`0x1402A5D70`: `+0x14` air acceleration, `+0x18` air drag,
    /// `+0xD0` turn rate).
    fn air_accel(&self) -> f32 {
        self.num("air_accel")
    }
    fn air_drag(&self) -> f32 {
        self.num("air_drag")
    }
    fn turn_rate(&self) -> f32 {
        self.num("turn_rate")
    }
    /// btltbl's globals the reactions read (G+0x14 / +0x16 / +0x18, G+0x2C, G+0x38, G+0x3C) and
    /// the reaction code's own numbers (`0x1402B8530`'s down time, the 20 hop).
    fn launch_min(&self) -> f32 {
        self.num("launch_min")
    }
    fn launch_max(&self) -> f32 {
        self.num("launch_max")
    }
    fn launch_hop(&self) -> f32 {
        self.num("launch_hop")
    }
    fn flinch_push(&self) -> f32 {
        self.num("flinch_push")
    }
    fn air_flinch_speed(&self) -> f32 {
        self.num("air_flinch_speed")
    }
    fn air_knock_speed(&self) -> f32 {
        self.num("air_knock_speed")
    }
    fn down_time(&self) -> f32 {
        self.num("down_time")
    }
    fn air_knock_hop(&self) -> f32 {
        self.num("air_knock_hop")
    }
    /// The enemy's stat record `+0x18`: the knock-back factor (decision M10: KH1's).
    fn weight(&self) -> f32 {
        self.num("weight")
    }
    /// KH1's gravity (`0x1403EAB54`) and fall cap (`0x1402AF6A0`).
    fn gravity(&self) -> f32 {
        self.num("gravity")
    }
    fn fall_cap(&self) -> f32 {
        self.num("fall_cap")
    }
    /// The blend the host gives the motion a reaction queues (the scripts' 8).
    fn attack_blend(&self) -> f32 {
        self.num("attack_blend")
    }
}

impl<T: Numbers + ?Sized> Engine for T {}

/// The names every enemy's manifest holds for the shared code ([`Engine`]).
pub const ENGINE_NAMES: &[&str] = &[
    "walk_speed",
    "run_speed",
    "air_accel",
    "air_drag",
    "turn_rate",
    "launch_min",
    "launch_max",
    "launch_hop",
    "flinch_push",
    "air_flinch_speed",
    "air_knock_speed",
    "down_time",
    "air_knock_hop",
    "weight",
    "gravity",
    "fall_cap",
    "attack_blend",
];

/// KH1's exe numbers the shared code reads that no file holds (Checked (decomp + exe), the same
/// values `kh1-core` uses for Sora): the stats-block defaults (`0x1402A5D70`: air acceleration 0.1,
/// air drag 0.95, turn rate π/16), gravity (`0x1403EAB54`), the fall cap 20 (`0x1402AF6A0`), the
/// launch's down time 30 (`0x1402B8530`), the 20 hop of the air reactions and clash
/// (`0x1402B8B90`, `0x1402B77D0`, `0x1402B9140`).
pub const EXE_NUMBERS: &[(&str, f32)] = &[
    ("air_accel", 0.1),
    ("air_drag", 0.95),
    ("turn_rate", std::f32::consts::PI / 16.0),
    ("gravity", crate::params::GRAVITY),
    ("fall_cap", 20.0),
    ("down_time", 30.0),
    ("air_knock_hop", 20.0),
];

/// KH1's walk / run motion blend in the move-toward routine (`0x140299D60` plays request 1 / 2
/// with `0x41000000` = 8.0). Checked (decomp).
pub const MOVE_BLEND: f32 = 8.0;

/// A table of named numbers (a manifest's `consts`), checked against an enemy's name list.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    values: Vec<(String, f32)>,
}

impl Table {
    /// From name / value pairs; every name of `names` (and of [`ENGINE_NAMES`]) must be there.
    pub fn new(values: Vec<(String, f32)>, names: &[&str]) -> Result<Table, String> {
        for n in names.iter().chain(ENGINE_NAMES) {
            if !values.iter().any(|(k, _)| k == n) {
                return Err(format!("constant {n} missing"));
            }
        }
        Ok(Table { values })
    }

    pub fn get(&self, n: &str) -> f32 {
        self.values.iter().find(|(k, _)| k == n).map(|v| v.1).unwrap_or(f32::NAN)
    }

    pub fn pairs(&self) -> &[(String, f32)] {
        &self.values
    }
}

impl Numbers for Table {
    fn num(&self, name: &str) -> f32 {
        self.get(name)
    }
}

/// What the host tells a behaviour that looks from one of its bones each frame (M11, the Search Ghost's
/// child object `ex_2230_00.bd`, the cone that finds its target): KH1 units, y down.
///
/// - `eye`: `sys 19(actor, code, 0)`, the code's bone's parent's world position (`0x14029EB20` →
///   `0x1401D6320` with t = 0: the lerp `0x1400E64D0` gives the parent; Checked (decomp));
/// - `axis`: `sys 115(actor, (1, 0, 0, 0), code, out)`, that same parent's world matrix row 0
///   (`0x14029EA00` takes the code's parent, `0x1401D62B0`; the vector times the matrix
///   `0x1400E5AD0`; Checked (decomp)): any length;
/// - `target`: the target's KH1 lock point 0 (`sys 32` → `0x1402B04C0(actor, 0)`), or none.
///
/// The host reads them from Re:CoM's own animated bones (the asset step's `sight_bone`; Sora's
/// `chest1`, KH1's lock point 0).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sight {
    pub eye: Vec3,
    pub axis: Vec3,
    pub target: Option<Vec3>,
}

/// What the host tells a behaviour about a hit its hit handler took (KH1's script hit handler
/// receives the hit record; Re:CoM's handler has already applied Re:CoM's damage).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitNews {
    /// A damaging hit from an attacker that didn't defeat it.
    Damage,
    /// Sora's Guard blocked its attack (KH1's reaction kinds 6 / 7: the clash).
    Clash,
    /// One of its guard rows repelled an attack (`0x1402B4BC0`: a type 1 / 3 / 4 row with the higher
    /// priority): KH1 gives it the attacker's hit record flagged 0x20 (`0x1402BF630`), which its
    /// script's hit handler hears like any record, but which does no damage (`0x1402BEEE0` returns at
    /// once) and plays no reaction (`0x1402B8E10` skips records with `& 0x22`). Checked (decomp).
    /// M11 (the shared enemy-guard step's repel).
    Repelled,
}

/// One KH1 enemy's behaviour as the host drives it: the AI frame (only while no action runs, as
/// KH1's script pauses), its body, its random stream, the hit news. Commands name attacks by
/// their index in the enemy's attack list ([`script::Cmd`]`<u8>`).
pub trait Behaviour {
    /// One AI frame: the commands.
    fn frame(&mut self, w: World) -> Vec<Cmd<u8>>;
    /// The body physics (the host steps it every frame).
    fn body(&self) -> std::cell::RefMut<'_, Body>;
    /// The air steering the script set this frame (cleared each frame).
    fn take_steer(&self) -> Option<Vec3>;
    /// The random draws since the last call (the log).
    fn take_draws(&self) -> Vec<Draw>;
    /// The behaviour's random stream (KH1's one `rand`; reactions draw from it too).
    fn rng(&self) -> Kh1Rand;
    fn set_rng(&self, r: Kh1Rand);
    /// The script's hit handler.
    fn on_hit(&self, news: HitNews);
    /// Whether the battle appearance is done.
    fn risen(&self) -> bool;
    /// The appearance was Re:CoM's own: no KH1 appearance.
    fn skip_rise(&self);
    /// The enemy's HP / max HP after a hit (Re:CoM's HP, decision M10), for scripts that read it.
    fn set_hp_ratio(&self, _r: f32) {}
    /// The state names for the log (`Cmd::State`).
    fn state_name(&self, s: u8) -> &'static str;
    /// The host's safety net (the AI issued nothing for a long time in Re:CoM's state 1): the
    /// behaviour starts again from its engaged state, as its supervisor does on a state request.
    fn reset(&self) {}
    /// The floor's height (KH1 y) under the body, from the host each frame before
    /// [`Behaviour::frame`] (`sys 89`, M11 fliers).
    fn set_floor(&self, _y: f32) {}
    /// One of its attacks met Sora (Re:CoM's `CCollAttack` apply answered 1, `0x14016F1C0`): KH1's
    /// attack object's hit callback (`+0x38`, M11 fliers), by the attack's index.
    fn on_landed(&self, _attack: u8) {}
    /// The KH1 reaction kind of the hit the next [`Behaviour::on_hit`] reports (the hit record's
    /// `+0xC & 0x1F`, which KH1's script hit handlers read: `hp[0xC] & 31`; M11 fliers).
    fn set_hit_kind(&self, _kind: u8) {}
    /// A launch its hit handler asks for in place of KH1's reaction (`sys 38`; the Powerwild),
    /// read after [`Behaviour::on_hit`]; the host plays it over the reaction KH1 chose first.
    fn scripted_launch(&self) -> Option<react::ScriptedLaunch> {
        None
    }
    /// Whether Sora's target is this enemy (KH1's `*(party 0 + 0x74)`; Re:CoM's attack target
    /// `player+0x84C`), each frame before [`Behaviour::frame`] (M11 fliers).
    fn set_targeted(&self, _by_sora: bool) {}
    /// Whether Sora stands on the floor, each frame before [`Behaviour::frame`]: the scripts read
    /// their target's movement state `*(target + 0x70)` (Checked (decomp): `0x1402B1960` keeps the
    /// state there; 0 on the ground, 2 in the air) and Re:CoM's is his mover's floor contact
    /// (`+0x120 & 5`; Assumption: KH1's other player states (glide, swim) count as not on the ground).
    /// M11 (the Air Pirate).
    fn set_target_grounded(&self, _on_ground: bool) {}
    /// Whether the hit the last [`Behaviour::on_hit`] reported keeps its reaction: KH1's script hit
    /// handler may zero the hit record's reaction kind (`hp[0xC] &= ~0x1F`, then no reaction and the
    /// action goes on; the Darkball's `0x1E20` after its sixth light hit in a row, M11).
    fn keeps_reaction(&self) -> bool {
        true
    }

    /// The KH1 world the battle stands for (`sys 105`'s world id: Re:CoM's world mapped as
    /// decision 21 maps its records, `kh1-recom`'s `kh1_enemy::kh1_world`; None for a Re:CoM world
    /// KH1 doesn't have), once before the first frame. Scripts with world rules read it (M11: the
    /// Bandit's Agrabah rules, world 8).
    fn set_world(&self, _kh1_world: Option<u8>) {}
    /// The hit's place in the enemy's run of hits, before [`Behaviour::on_hit`]: KH1's hit record
    /// `+0x24`, which `0x1402BFE60` sets from the actor's counter `+0x3E8` (one more per record,
    /// clashes included) and which the enemy update zeroes whenever no action plays (`0x1402B7260`:
    /// actor flag 4 clear). Checked (decomp). M11: the Bandit's counter reads it.
    fn set_hit_count(&self, _n: u16) {}
    /// The reaction kind of a damaging hit once the script's hit handler has run: KH1 runs the
    /// handler before the reaction (`0x1402B7B90` → `0x1402BCD60`, then `0x1402B8E10`'s switch
    /// reads the record's kind), and a handler may rewrite it (`hp[0xC] & ~0x1F | k`). Checked
    /// (decomp). M11: the Bandit's kind 4 → 5. The default keeps it.
    fn reaction_kind(&self, kind: u8) -> u8 {
        kind
    }
    /// The bone sight (M11, the Search Ghost's cone), each frame before [`Behaviour::frame`] /
    /// [`Behaviour::side_frame`], for a class whose manifest names a `sight_bone`.
    fn set_sight(&self, _s: Sight) {}
    /// A frame in which the host doesn't poll [`Behaviour::frame`] (an action or a reaction plays:
    /// KH1's script pauses): KH1's child objects (`sys 94` → `0x14028F0A0`, the model's sub-object
    /// table `+0x13C`) run their own scripts every frame all the same (M11, the Search Ghost's cone).
    /// The default: no children.
    fn side_frame(&mut self, _w: World) -> Vec<Cmd<u8>> {
        Vec::new()
    }
    /// The killing hit (Re:CoM's defeat, state 30), before Re:CoM's defeat prizes: KH1's script hit
    /// handler runs for it too, and some act before they apply its damage (the Search Ghost's reward
    /// at `0x28A6` comes before `sys 44` and its death `0x2A14`). The default: nothing.
    fn on_defeat(&self) {}
    /// What a handler ([`Behaviour::on_hit`], [`Behaviour::on_defeat`]) did that KH1 does inside the
    /// handler itself (M11, the Search Ghost's reward): the host applies these right after the call,
    /// not at the next AI frame (the AI is paused while the reaction plays).
    fn take_now(&self) -> Vec<Cmd<u8>> {
        Vec::new()
    }
    /// The push direction of the hit the next [`Behaviour::on_hit`] reports (KH1's hit record `+0x10`:
    /// target − attacker, flat, unit; [`react::Hit::dir`]), for a script hit handler that reads it (M11,
    /// the Pirate's `0x151E`: `sys 100` of its facing against it picks its get-up slash).
    fn set_hit_dir(&self, _dir: Vec3) {}
    /// Whether KH1's script hit handler takes no reaction for a damaging hit now: `sys 42`
    /// (`0x1402BBD80` → `0x1402B7E60`: the flinch's push slide, the hit record marked done `*hit |= 2`,
    /// so `0x1402B8E10` plays nothing) in place of a state request. `playing`: the request playing (KH1's
    /// `+0x164`); `launch_base`: the base of its last launch (`+0x3EA`, written only by the launches
    /// `0x1402B79F0` / `0x1402B8530`; 0 before any: Assumption, the actor's memory starts zeroed).
    /// M11, the Pirate's `0x267E` (its attack motions, and `sys 70` `0x1402B7EE0`: `(+0x164 − +0x3EA)`
    /// under 10 and `% 5 == 3`, lying after a launch). Checked (decomp). The host keeps Re:CoM's state
    /// for such a hit through Re:CoM's own super armour ([`host::RECOM_ARMOUR`]) and tells the behaviour
    /// with [`Behaviour::on_armoured_hit`] instead of [`Behaviour::on_hit`].
    fn armoured(&self, _playing: Option<u16>, _launch_base: u16) -> bool {
        false
    }
    /// A damaging hit taken while [`Behaviour::armoured`]: the hit handler's `sys 42` path (KH1's push
    /// slide along `dir`, the push direction; no state request, the action goes on).
    fn on_armoured_hit(&self, _dir: Vec3) {}
}
