//! Boss revenge at Combat = KH1 (owner, 2026-11-01): KH2 FM's revenge value on Re:CoM's humanoid
//! bosses, so Sora can't combo them endlessly. `docs/recom-boss-revenge.md`.
//!
//! KH2's rule (KH2 FM Steam exe, Checked (decomp + exe + files)):
//! - every hit adds its attack row's revenge value (ATKP `+0x2A`) to the enemy's counter
//!   (`0x1403DB730`);
//! - the counter decays by 1 per 60 Hz frame while the enemy runs an AI action, not while it is in
//!   a damage motion (`0x1403DB440`, `0x1403CA250`); nothing else resets it;
//! - 20 frames into a damage motion (Hades: 30; motion trigger 28, `0x14040DD70` case `0x1C`),
//!   limit ≤ counter starts the boss AI's own "revenge" action;
//! - the limit is set by the boss's AI (`trap_enemy_set_karma_limit`, table 2 #76,
//!   `0x140431E30`); 100 when the AI sets none.
//!
//! Here (Re:CoM's own systems do the work): the counter lives in a 0x30-byte tail added to each
//! boss body's allocation (Re:CoM's enemy factory `0x140188F40` allocates it: our patch raises the
//! size immediate); the add is in the enemy-hit wrapper; the decay is worked out lazily at each
//! hit and check from a game clock, minus the frames the boss spent in its flinch / blow-away
//! states (counted in their step hooks); the check runs in the step of Re:CoM's flinch (state 11,
//! `0x140184100`) and blow-away (state 12, `0x140182E50`); revenge starts the boss's own Re:CoM
//! attack state with the transition Re:CoM's message 2 makes (as the cardless card play P1 does).
//!
//! Revenge moves untouchable (owner, 2026-11-02): while the boss plays the attack state its revenge
//! started, Re:CoM's own rule for an attacking enemy holds: its hurt bodies go off at the state's
//! begin (`0x14017DA80`, decision 26's skip not taken for this attack) and its hit handler drops
//! every hit (`+0x260 & 1` without the card-interrupt bit: decision 14's bit not set for it). The
//! window ends with the state (Re:CoM's `0x14017DA90`: bits 0 / 2 cleared, bodies back on).

/// The tail's size (added to each boss body's allocation) and its fields.
pub const TAIL: u32 = 0x30;
pub mod tail {
    /// `RVG1`, written when the tail is set up for this battle.
    pub const MAGIC: usize = 0;
    /// The counter (f32).
    pub const COUNTER: usize = 4;
    /// The game clock (60 Hz frames, f64) at the last decay.
    pub const LAST: usize = 8;
    /// Frames spent in the flinch / blow-away states since the last decay (f32).
    pub const FLINCH_SINCE: usize = 0x10;
    /// Frames into the current flinch / blow-away (f32).
    pub const FLINCH_ELAPSED: usize = 0x14;
    /// [`flag`] bits (u32).
    pub const FLAGS: usize = 0x18;
    /// The battle number the tail belongs to (u32).
    pub const BATTLE: usize = 0x1C;
    /// The attack state the last revenge started (i32).
    pub const STATE: usize = 0x20;
    /// What the untouchable window ignored: [KH1 swings, KH1 spells, other hits, Guard reactions]
    /// (u8 each, saturating; [`Ignored`]).
    pub const IGNORED: usize = 0x24;
    /// The game clock (60 Hz frames, f32) when the window began.
    pub const START: usize = 0x28;
}
pub const MAGIC: u32 = u32::from_le_bytes(*b"RVG1");
pub mod flag {
    /// The current flinch already decided (revenge started, or the flinch ended below the limit).
    pub const CHECKED: u32 = 1;
    /// Hades' limit dropped to 50 (KH2 never raises it again).
    pub const HADES_LOW: u32 = 2;
    /// Revenge wrote its attack state as the next state; Re:CoM hasn't begun it yet.
    pub const PENDING: u32 = 4;
    /// The boss is in the attack state its revenge started (the untouchable window).
    pub const UNTOUCHABLE: u32 = 8;
    /// The open window is the boss's own use of its revenge move (owner, 2026-11-02: "untouchable
    /// whenever played"), not one revenge started.
    pub const OWN: u32 = 0x10;
}

/// Who opened an untouchable window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Revenge started the move (its start marked the tail pending).
    Revenge,
    /// The boss chose its revenge move itself (its think, a card play).
    Own,
}

/// At an attack state's begin: whether the window opens, and why. `current` is the state beginning
/// (`+0x60` at the begin), `move_state` the boss's revenge-move state object. A pending revenge start
/// opens it for its own state; any begin of the revenge-move state opens it otherwise (owner,
/// 2026-11-02). `None`: an ordinary attack (decisions 14 / 26 stay).
pub fn window_opens(tail_flags: u32, current: usize, move_state: usize) -> Option<Origin> {
    if current != move_state {
        return None;
    }
    Some(if tail_flags & flag::PENDING != 0 { Origin::Revenge } else { Origin::Own })
}

/// The enemy's "in an attack" bit (`+0x260 & 1`; Re:CoM's hit handler drops hits while it is set
/// without the card-interrupt bit `0x2000000`, `0x14017F7E0` at `0x14017F8CB`). Checked (decomp).
pub const ATTACKING: u64 = 1;

/// Whether Re:CoM's own rule makes the boss untouchable now: the window is open (or its start is
/// pending), Re:CoM still has the attack running (bit 0; its attack end `0x14017E540` and state end
/// `0x14017DA90` clear it) and the boss lives.
pub fn holds(tail_flags: u32, enemy_flags: u64, hp: i32) -> bool {
    tail_flags & (flag::PENDING | flag::UNTOUCHABLE) != 0 && enemy_flags & ATTACKING != 0 && hp > 0
}

/// Whether the attack state Re:CoM is beginning (its current state object, `+0x60`, already the new
/// one when the begin runs: `0x140242F50`) is the one revenge started.
pub fn begins_revenge(tail_flags: u32, current: usize, revenge_state: usize) -> bool {
    tail_flags & flag::PENDING != 0 && current == revenge_state
}

/// What one untouchable window ignored (the tail's four counters).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ignored {
    pub swings: u8,
    pub spells: u8,
    pub other: u8,
    pub guards: u8,
}

/// What an ignored hit was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IgnoredKind {
    Swing,
    Spell,
    Other,
    Guard,
}

impl Ignored {
    pub fn from_bytes(b: [u8; 4]) -> Ignored {
        Ignored { swings: b[0], spells: b[1], other: b[2], guards: b[3] }
    }
    pub fn to_bytes(self) -> [u8; 4] {
        [self.swings, self.spells, self.other, self.guards]
    }
    pub fn add(&mut self, k: IgnoredKind) {
        let c = match k {
            IgnoredKind::Swing => &mut self.swings,
            IgnoredKind::Spell => &mut self.spells,
            IgnoredKind::Other => &mut self.other,
            IgnoredKind::Guard => &mut self.guards,
        };
        *c = c.saturating_add(1);
    }
    pub fn hits(&self) -> u32 {
        self.swings as u32 + self.spells as u32 + self.other as u32
    }
}

/// KH2's check frame in a damage motion (trigger 28 at frame 20; Hades' flinches at 30).
/// Checked (files: every boss `.mset` read).
pub const CHECK_FRAME: f32 = 20.0;
pub const CHECK_FRAME_HADES: f32 = 30.0;
/// Re:CoM time units per 60 Hz frame (`docs/recom-timing.md`).
pub const TICKS_PER_FRAME: f64 = 50.0;

/// A boss's limit rule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Limit {
    Fixed(f32),
    /// `B_HE030`: 80 from its init, 50 once its HP is under 30 % (checked on each hit, never back).
    Hades,
}

/// One Re:CoM boss body with revenge.
#[derive(Clone, Copy, Debug)]
pub struct Boss {
    pub name: &'static str,
    /// Re:CoM's enemy id (factory `0x140188F40`).
    pub id: u16,
    /// The class's vtable (RVA), RTTI-checked at install.
    pub vtable: u32,
    pub rtti: &'static str,
    /// The factory's `mov ecx, size` for this class (RVA) and the size it holds.
    pub alloc: u32,
    pub size: u32,
    pub limit: Limit,
    /// The Re:CoM attack state revenge starts (2..=9), with what it is; `None`: the class has no
    /// attack state of its own (its counter runs, nothing starts).
    pub attack: Option<(i32, &'static str)>,
    /// The KH2 counterpart the numbers come from.
    pub kh2: &'static str,
    /// The untouchable window ends before the state does (owner, 2026-11-02: Hades only in his
    /// wind-up); `None`: the whole state.
    pub windup: Option<WindUp>,
}

/// A revenge state whose untouchable window is only its wind-up: it ends when the state's own exe
/// moves its sub-step field off 0 (the same call that starts the next motion and its timer).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindUp {
    /// The class's vtable slot for the state's exe (RVA) and the exe it holds (RVA).
    pub exe_slot: u32,
    pub exe: u32,
    /// The sub-step field (i32) the exe keeps.
    pub step: usize,
    pub what: &'static str,
}

/// Hades' state 5 (fire-up): begin `0x1403B0750` plays `e107_054` and sets `+0xC80 = 0`; the exe
/// `0x1403B1DB0` (vtable `+0x168`), once that motion ends (`0x1401A57D0`), plays the spin `e107_055`,
/// sets the timer `+0x160` = (rand % 3 + 1) × 12000 and `+0xC80 = 1`. Checked (decomp + exe).
pub const HADES_WINDUP: WindUp = WindUp { exe_slot: 0x0066_21A0, exe: 0x003B_1DB0, step: 0xC80, what: "wind-up e107_054 over: sub-step +0xC80 left 0, spin e107_055 started" };

/// Whether the wind-up is over (the sub-step field left 0).
pub fn windup_over(step: i32) -> bool {
    step != 0
}

/// Re:CoM's enemy state machine (`CFSM` at `+0x58`: `+0x60` current, `+0x68` next). Its update
/// `0x140242F50` runs the old state's end, then its vtable `+0x28` (`0x1401A3B80` for every revenge
/// boss), then sets current = next, zeroes next, and begins it; next is re-read after `+0x28`
/// (`0x140242F75`). Checked (decomp + disasm + exe: each boss class's FSM vtable at COL offset 0x58).
pub const FSM_AT: usize = 0x58;

/// At a state change (the `+0x28` callback): whether a flinch / blow-away that hasn't decided yet is
/// being left for something that isn't a new reaction (a hit's new flinch has its own check). This is
/// "the reaction ended before KH2's frame": its own end, the boss's think, a stop, anything.
pub fn reaction_left(old_is_reaction: bool, next_is_reaction: bool) -> bool {
    old_is_reaction && !next_is_reaction
}

/// Re:CoM's state-change callback every revenge boss class's FSM vtable holds at `+0x28` (RVA):
/// it zeroes the mover's velocity. Ours calls it, then decides.
pub const FSM_CHANGE: u32 = 0x001A_3B80;

impl Boss {
    pub fn check_frame(&self) -> f32 {
        match self.limit {
            Limit::Hades => CHECK_FRAME_HADES,
            Limit::Fixed(_) => CHECK_FRAME,
        }
    }
    /// Re:CoM's boss damage falloff is off for bosses whose revenge can act (owner).
    pub fn falloff_off(&self) -> bool {
        self.attack.is_some()
    }
}

/// The bosses (owner, 2026-11-01: story / Absent Silhouette numbers; every humanoid but Genie
/// Jafar; Riku's story keeps Re:CoM's). Vtables Checked (exe RTTI); allocation sites Checked
/// (disasm of `0x140188F40`); attack states: each class's own vtable states (Checked (exe)), which
/// state is "nearest" KH2's revenge move is an Assumption from the state's motions and code.
pub const BOSSES: &[Boss] = &[
    Boss { name: "hooded figure", id: 100, vtable: 0x0065_CB08, rtti: "CTutorialHood", alloc: 0x0018_9517, size: 0xCE0, limit: Limit::Fixed(100.0), attack: None, kh2: "none (KH2's default 100)", windup: None },
    Boss { name: "Leon", id: 101, vtable: 0x0065_DDE8, rtti: "CTutorialLeon", alloc: 0x0018_9541, size: 0xD10, limit: Limit::Fixed(100.0), attack: None, kh2: "N_HB530_BTL2 (owner: 100)", windup: None },
    Boss { name: "Cloud", id: 106, vtable: 0x0066_0BA8, rtti: "CBossCloud", alloc: 0x0018_9595, size: 0xD30, limit: Limit::Fixed(100.0), attack: Some((4, "charge (e106_053: aims, then a straight dash; 2 times, 4 below half HP)")), kh2: "N_HB550_BTL2 (100)", windup: None },
    Boss { name: "Hades", id: 107, vtable: 0x0066_2038, rtti: "CBossHades", alloc: 0x0018_95BF, size: 0xD80, limit: Limit::Hades, attack: Some((5, "fire-up (unflinching, flames)")), kh2: "B_HE030 (80, 50 below 30 % HP)", windup: Some(HADES_WINDUP) },
    Boss { name: "Captain Hook", id: 112, vtable: 0x0066_AE18, rtti: "CBossHook", alloc: 0x0018_9613, size: 0xD90, limit: Limit::Fixed(100.0), attack: Some((3, "sword slash")), kh2: "Barbossa B_CA010 (default 100)", windup: None },
    Boss { name: "Vexen", id: 114, vtable: 0x0066_36B0, rtti: "CBossVexen", alloc: 0x0018_963D, size: 0xD70, limit: Limit::Fixed(150.0), attack: Some((6, "ice needles")), kh2: "N_CM040_BTL (AS 150)", windup: None },
    Boss { name: "Riku", id: 116, vtable: 0x0066_4C50, rtti: "CBossRiku", alloc: 0x0018_9667, size: 0xD70, limit: Limit::Fixed(100.0), attack: Some((4, "slash (p003_039)")), kh2: "none (KH2's default 100)", windup: None },
    Boss { name: "Larxene", id: 117, vtable: 0x0066_6598, rtti: "CBossLarxene", alloc: 0x0018_9691, size: 0xDA0, limit: Limit::Fixed(100.0), attack: Some((4, "hop back, then lightning from range")), kh2: "B_EX400 (AS 100)", windup: None },
    Boss { name: "Axel", id: 118, vtable: 0x0066_7CD8, rtti: "CBossAxel", alloc: 0x0018_96BB, size: 0xD70, limit: Limit::Fixed(92.0), attack: Some((4, "warp, then a flame burst around him")), kh2: "B_EX110 (unit arg 1 = 0: 92)", windup: None },
    Boss { name: "Marluxia", id: 119, vtable: 0x0067_EB60, rtti: "CBossMarluxia", alloc: 0x0018_9A2D, size: 0xD50, limit: Limit::Fixed(100.0), attack: Some((2, "scythe cut")), kh2: "N_CM000_BTL (AS 100)", windup: None },
    Boss { name: "Marluxia 2", id: 120, vtable: 0x0067_AF90, rtti: "CBossMarluxia2", alloc: 0x0018_995B, size: 0xDF0, limit: Limit::Fixed(100.0), attack: Some((2, "death scythe")), kh2: "N_CM000_BTL (AS 100)", windup: None },
    Boss { name: "Marluxia 3", id: 132, vtable: 0x0067_85B8, rtti: "CBossMarluxia3", alloc: 0x0018_99D9, size: 0xF50, limit: Limit::Fixed(100.0), attack: Some((2, "scythe strike")), kh2: "N_CM000_BTL (AS 100)", windup: None },
];

/// Parts of a boss whose hits count on the body (owner: Marluxia's parts). Vtables Checked (exe
/// RTTI). The body is the last Marluxia 2 / 3 body set up in this battle.
pub struct Part {
    pub vtable: u32,
    pub rtti: &'static str,
}
pub const PARTS: &[Part] = &[
    Part { vtable: 0x0067_D218, rtti: "CBossMarluxia2Funnel" },
    Part { vtable: 0x0067_B428, rtti: "CBossMarluxia2Scythe" },
    Part { vtable: 0x0067_8A50, rtti: "CBossMarluxia3Specter" },
];
/// The bodies whose parts count ([`PARTS`]).
pub fn owns_parts(b: &Boss) -> bool {
    matches!(b.id, 120 | 132)
}

pub fn boss_by_vtable(vt_rva: u32) -> Option<&'static Boss> {
    BOSSES.iter().find(|b| b.vtable == vt_rva)
}
pub fn is_part(vt_rva: u32) -> bool {
    PARTS.iter().any(|p| p.vtable == vt_rva)
}

/// The factory's `mov ecx, imm32` and ours (the size plus the tail).
pub fn alloc_bytes(size: u32) -> [u8; 5] {
    let s = size.to_le_bytes();
    [0xB9, s[0], s[1], s[2], s[3]]
}
pub fn alloc_bytes_ours(size: u32) -> [u8; 5] {
    alloc_bytes(size + TAIL)
}

/// The limit now. `low` is whether Hades' limit already dropped; returns (limit, low).
pub fn limit(b: &Boss, hp: i32, max_hp: i32, low: bool) -> (f32, bool) {
    match b.limit {
        Limit::Fixed(v) => (v, false),
        Limit::Hades => {
            let low = low || (max_hp > 0 && (hp as i64) * 100 < (max_hp as i64) * 30);
            (if low { 50.0 } else { 80.0 }, low)
        }
    }
}

/// KH2's revenge values for Sora's attacks (00battle ATKP `+0x2A`, Checked (files)): ground
/// swings (126..128) 8, ground finisher (129 / 130) 24, air swings (131 / 132) 12, air finisher
/// (133 / 134) 28; Fire 8 (502's last hit), Blizzard 16 (248), Thunder 32 (249..251). KH1 hits
/// without a KH2 twin (Gravity, Stop, Aero, the modifiers, specials, Counterattack, a shot Guard
/// turned back): KH2's most common value, 10 (Assumption, the coordinator's choice).
pub mod value {
    pub const GROUND_SWING: f32 = 8.0;
    pub const GROUND_FINISHER: f32 = 24.0;
    pub const AIR_SWING: f32 = 12.0;
    pub const AIR_FINISHER: f32 = 28.0;
    pub const FIRE: f32 = 8.0;
    pub const BLIZZARD: f32 = 16.0;
    pub const THUNDER: f32 = 32.0;
    pub const OTHER: f32 = 10.0;
}

/// A KH1 swing row's value (`kh1_core::row`).
pub fn swing_value(row: usize) -> f32 {
    use kh1_core::data::row;
    match row {
        row::SWING_1 | row::SWING_2 | 12 | row::LUNGE | row::JUMP_UP | row::HOP => value::GROUND_SWING,
        row::GROUND_FINISHER => value::GROUND_FINISHER,
        row::AIR_SWING_1 | row::AIR_SWING_2 => value::AIR_SWING,
        row::AIR_FINISHER => value::AIR_FINISHER,
        _ => value::OTHER,
    }
}

/// A KH1 spell's value (`crate::magic::spell`).
pub fn spell_value(s: u8) -> f32 {
    use crate::magic::spell;
    match s {
        spell::FIRE => value::FIRE,
        spell::BLIZZARD => value::BLIZZARD,
        spell::THUNDER => value::THUNDER,
        _ => value::OTHER,
    }
}

/// The source of the hit the enemy-hit wrapper is taking (set by `win::attack`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Swing(Option<usize>),
    Spell(u8),
    Other,
}

pub fn value_of(s: Source) -> f32 {
    match s {
        Source::Swing(Some(r)) => swing_value(r),
        Source::Swing(None) | Source::Other => value::OTHER,
        Source::Spell(sp) => spell_value(sp),
    }
}

/// One boss's counter (the tail's fields).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Counter {
    pub value: f32,
    pub last: f64,
    pub flinch_since: f32,
    pub flinch_elapsed: f32,
    pub flags: u32,
}

impl Counter {
    pub fn new(now: f64) -> Counter {
        Counter { value: 0.0, last: now, flinch_since: 0.0, flinch_elapsed: 0.0, flags: 0 }
    }

    /// KH2's decay up to `now`: 1 per 60 Hz frame, none for the frames spent in a flinch /
    /// blow-away (`0x1403CA250`: no decay in a damage motion).
    pub fn decay(&mut self, now: f64) {
        let elapsed = (now - self.last).max(0.0) as f32;
        let d = (elapsed - self.flinch_since).max(0.0);
        self.value = (self.value - d).max(0.0);
        self.last = now;
        self.flinch_since = 0.0;
    }

    /// A hit landed: its value added (after the decay), and a new flinch starts.
    pub fn hit(&mut self, now: f64, v: f32) {
        self.decay(now);
        self.value += v;
        self.flinch_elapsed = 0.0;
        self.flags &= !flag::CHECKED;
    }

    /// A step of the flinch / blow-away state, `dt` 60 Hz frames.
    pub fn flinch_frame(&mut self, dt: f32) {
        self.flinch_since += dt;
        self.flinch_elapsed += dt;
    }

    /// Whether this step decides the flinch: its check frame reached, or the state ends before
    /// it (Re:CoM's flinches can be shorter than KH2's damage motions). Once per flinch.
    pub fn due(&self, check_frame: f32, ending: bool) -> bool {
        self.flags & flag::CHECKED == 0 && (self.flinch_elapsed >= check_frame || ending)
    }

    /// Decides at `now`: revenge when limit ≤ counter (KH2's compare). Marks the flinch decided.
    pub fn decide(&mut self, now: f64, limit: f32) -> bool {
        self.decay(now);
        self.flags |= flag::CHECKED;
        limit <= self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_consistent() {
        for b in BOSSES {
            assert!(b.size % 0x10 == 0 && b.size >= 0xCA0, "{}", b.name);
            if let Some((s, _)) = b.attack {
                assert!((2..=9).contains(&s), "{}: attack state {s}", b.name);
            }
            assert_eq!(alloc_bytes(b.size)[0], 0xB9);
            assert_eq!(u32::from_le_bytes(alloc_bytes_ours(b.size)[1..].try_into().unwrap()), b.size + TAIL);
            // The factory writes +0x294 after the ctor (`0x140189B7F`), far below the tail.
            assert!(b.size > 0x298);
        }
        let mut ids: Vec<u16> = BOSSES.iter().map(|b| b.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), BOSSES.len());
        // Every allocation site is listed in sites.rs with the size it holds.
        for b in BOSSES {
            let s = crate::sites::REVENGE_ALLOC.iter().find(|s| s.rva == b.alloc).expect(b.name);
            assert_eq!(s.expect, &alloc_bytes(b.size)[..], "{}", b.name);
        }
        assert_eq!(crate::sites::REVENGE_ALLOC.len(), BOSSES.len());
        // Genie Jafar has no revenge in KH2 (no trigger 28): not listed.
        assert!(BOSSES.iter().all(|b| b.id != 108));
    }

    #[test]
    fn owners_limits() {
        let l = |id: u16| BOSSES.iter().find(|b| b.id == id).unwrap();
        assert_eq!(limit(l(118), 1, 1, false).0, 92.0);
        assert_eq!(limit(l(117), 1, 1, false).0, 100.0);
        assert_eq!(limit(l(114), 1, 1, false).0, 150.0);
        for id in [119, 120, 132, 106, 112, 116, 100, 101] {
            assert_eq!(limit(l(id), 1, 1, false).0, 100.0, "{id}");
        }
        let h = l(107);
        assert_eq!(limit(h, 31, 100, false), (80.0, false));
        assert_eq!(limit(h, 30, 100, false), (80.0, false));
        assert_eq!(limit(h, 29, 100, false), (50.0, true));
        // Sticky, as KH2's (it never sets 80 again).
        assert_eq!(limit(h, 90, 100, true), (50.0, true));
        assert_eq!(h.check_frame(), 30.0);
        assert_eq!(l(118).check_frame(), 20.0);
    }

    #[test]
    fn values() {
        use kh1_core::data::row;
        assert_eq!(swing_value(row::SWING_1), 8.0);
        assert_eq!(swing_value(row::SWING_2), 8.0);
        assert_eq!(swing_value(row::GROUND_FINISHER), 24.0);
        assert_eq!(swing_value(row::AIR_SWING_1), 12.0);
        assert_eq!(swing_value(row::AIR_FINISHER), 28.0);
        assert_eq!(swing_value(row::COUNTERATTACK), 10.0);
        assert_eq!(swing_value(row::BLITZ), 10.0);
        assert_eq!(swing_value(row::SONIC_BLADE_DASH), 10.0);
        assert_eq!(spell_value(crate::magic::spell::FIRE), 8.0);
        assert_eq!(spell_value(crate::magic::spell::BLIZZARD), 16.0);
        assert_eq!(spell_value(crate::magic::spell::THUNDER), 32.0);
        assert_eq!(spell_value(crate::magic::spell::GRAVITY), 10.0);
        assert_eq!(value_of(Source::Other), 10.0);
    }

    #[test]
    fn decay_one_a_frame_but_not_in_flinches() {
        let mut c = Counter::new(0.0);
        c.hit(0.0, 24.0);
        // 20 frames in the flinch: no decay for them.
        for _ in 0..20 {
            c.flinch_frame(1.0);
        }
        c.decay(20.0);
        assert_eq!(c.value, 24.0);
        // 10 more frames out of it: 10 off.
        c.decay(30.0);
        assert_eq!(c.value, 14.0);
        c.decay(100.0);
        assert_eq!(c.value, 0.0);
    }

    #[test]
    fn axel_revenges_on_the_third_chained_combo() {
        // KH1's basic ground combo: swing, swing, finisher (8 + 8 + 24), hits 12 frames apart in
        // one long flinch chain, the finisher's flinch reaching the check.
        let axel = BOSSES.iter().find(|b| b.id == 118).unwrap();
        let mut c = Counter::new(0.0);
        let mut now = 0.0;
        let mut fired = None;
        'outer: for combo in 0..4 {
            for v in [8.0, 8.0, 24.0] {
                c.hit(now, v);
                for _ in 0..12 {
                    c.flinch_frame(1.0);
                    now += 1.0;
                }
                if v == 24.0 {
                    // the finisher's flinch runs on to frame 20
                    for _ in 12..20 {
                        c.flinch_frame(1.0);
                        now += 1.0;
                    }
                    if c.due(axel.check_frame(), false) && c.decide(now, limit(axel, 1, 1, false).0) {
                        fired = Some(combo);
                        break 'outer;
                    }
                }
            }
        }
        assert_eq!(fired, Some(2), "{c:?}");
    }

    #[test]
    fn window_opens_for_the_revenge_move_whoever_started_it() {
        assert_eq!(window_opens(flag::PENDING, 0x1000, 0x1000), Some(Origin::Revenge));
        assert_eq!(window_opens(0, 0x1000, 0x1000), Some(Origin::Own));
        assert_eq!(window_opens(flag::CHECKED | flag::HADES_LOW, 0x1000, 0x1000), Some(Origin::Own));
        // Another attack: no window, whether a revenge start was pending or not.
        assert_eq!(window_opens(flag::PENDING, 0x1040, 0x1000), None);
        assert_eq!(window_opens(0, 0x1040, 0x1000), None);
        // An own-choice window holds as a revenge one (bit 0, HP).
        assert!(holds(flag::UNTOUCHABLE | flag::OWN, 1, 10));
        assert!(!holds(flag::OWN, 1, 10));
    }

    #[test]
    fn untouchable_window_follows_recoms_attack_bit() {
        // Pending (start written, not begun) and open windows hold while bit 0 is set and HP > 0.
        assert!(holds(flag::PENDING, 1 | 4, 100));
        assert!(holds(flag::UNTOUCHABLE | flag::CHECKED, 1, 1));
        // Re:CoM's attack end cleared bit 0: hits land again.
        assert!(!holds(flag::UNTOUCHABLE, 4, 100));
        // Dead, or no window.
        assert!(!holds(flag::UNTOUCHABLE, 1, 0));
        assert!(!holds(flag::CHECKED | flag::HADES_LOW, 1, 100));
        // Begins only for the state revenge started.
        assert!(begins_revenge(flag::PENDING, 0x1000, 0x1000));
        assert!(!begins_revenge(flag::PENDING, 0x1040, 0x1000));
        assert!(!begins_revenge(flag::UNTOUCHABLE, 0x1000, 0x1000));
    }

    #[test]
    fn clouds_revenge_is_his_charge() {
        // Owner (2026-11-02): state 4 (begin 0x1403AE680, exe 0x1403AF190), not Omnislash (6).
        let c = BOSSES.iter().find(|b| b.id == 106).unwrap();
        assert_eq!(c.attack.unwrap().0, 4);
        assert!(c.windup.is_none(), "the window covers every repeat, to Re:CoM's attack end");
    }

    #[test]
    fn hades_window_is_his_wind_up_only() {
        let h = BOSSES.iter().find(|b| b.id == 107).unwrap();
        let w = h.windup.expect("Hades");
        // His state-5 exe in his vtable's state-5 exe slot (+0x118 + 3 × 0x18 + 8).
        assert_eq!(w.exe_slot, h.vtable + 0x118 + 3 * 0x18 + 8);
        assert_eq!(h.attack.unwrap().0, 5);
        assert!(!windup_over(0));
        assert!(windup_over(1));
        // Everyone else: the whole state.
        assert!(BOSSES.iter().filter(|b| b.id != 107).all(|b| b.windup.is_none()));
        // Its slot is listed and checked.
        assert!(crate::sites::REVENGE_WINDUP_SLOTS.iter().any(|p| p.rva == w.exe_slot && p.target == w.exe));
    }

    #[test]
    fn reaction_end_decides_unless_a_new_reaction_follows() {
        assert!(reaction_left(true, false));
        assert!(!reaction_left(true, true), "a hit's new flinch has its own check");
        assert!(!reaction_left(false, false));
        // At the end, once per reaction: decided unless the frame-20 check already did.
        let mut c = Counter::new(0.0);
        c.hit(0.0, 10.0);
        c.flinch_frame(8.0);
        assert!(!c.due(20.0, false), "frame 8: not yet");
        assert!(c.due(20.0, true), "the think cut it short at frame 8: decide now");
        c.decide(8.0, 100.0);
        assert!(!c.due(20.0, true));
        // The FSM slots: one per boss class.
        for b in BOSSES {
            assert_eq!(crate::sites::REVENGE_FSM_SLOTS.iter().filter(|p| p.name.starts_with(b.rtti) && p.name[b.rtti.len()..].starts_with(' ')).count(), 1, "{}", b.name);
        }
        assert_eq!(crate::sites::REVENGE_FSM_SLOTS.len(), BOSSES.len());
        assert!(crate::sites::REVENGE_FSM_SLOTS.iter().all(|p| p.target == FSM_CHANGE));
    }

    #[test]
    fn ignored_counts_saturate() {
        let mut i = Ignored::default();
        i.add(IgnoredKind::Swing);
        i.add(IgnoredKind::Spell);
        i.add(IgnoredKind::Guard);
        assert_eq!(i.hits(), 2);
        assert_eq!(Ignored::from_bytes(i.to_bytes()), i);
        for _ in 0..300 {
            i.add(IgnoredKind::Other);
        }
        assert_eq!(i.other, 255);
        // The tail's fields fit in it.
        assert!(tail::START + 4 <= TAIL as usize && tail::IGNORED + 4 <= tail::START);
    }

    #[test]
    fn checked_once_per_flinch_and_early_end_decides() {
        let mut c = Counter::new(0.0);
        c.hit(0.0, 100.0);
        c.flinch_frame(5.0);
        assert!(!c.due(20.0, false));
        assert!(c.due(20.0, true), "a flinch shorter than 20 frames decides at its end");
        assert!(c.decide(5.0, 100.0));
        assert!(!c.due(20.0, true), "once");
        c.hit(6.0, 1.0);
        assert!(c.due(20.0, true), "a new hit, a new flinch");
    }
}
