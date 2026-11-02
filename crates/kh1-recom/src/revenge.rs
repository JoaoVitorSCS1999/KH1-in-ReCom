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
//! Here (Re:CoM's own systems do the work): the counter lives in a 0x20-byte tail added to each
//! boss body's allocation (Re:CoM's enemy factory `0x140188F40` allocates it: our patch raises the
//! size immediate); the add is in the enemy-hit wrapper; the decay is worked out lazily at each
//! hit and check from a game clock, minus the frames the boss spent in its flinch / blow-away
//! states (counted in their step hooks); the check runs in the step of Re:CoM's flinch (state 11,
//! `0x140184100`) and blow-away (state 12, `0x140182E50`); revenge starts the boss's own Re:CoM
//! attack state with the transition Re:CoM's message 2 makes (as the cardless card play P1 does).

/// The tail's size (added to each boss body's allocation) and its fields.
pub const TAIL: u32 = 0x20;
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
}
pub const MAGIC: u32 = u32::from_le_bytes(*b"RVG1");
pub mod flag {
    /// The current flinch already decided (revenge started, or the flinch ended below the limit).
    pub const CHECKED: u32 = 1;
    /// Hades' limit dropped to 50 (KH2 never raises it again).
    pub const HADES_LOW: u32 = 2;
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
}

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
    Boss { name: "hooded figure", id: 100, vtable: 0x0065_CB08, rtti: "CTutorialHood", alloc: 0x0018_9517, size: 0xCE0, limit: Limit::Fixed(100.0), attack: None, kh2: "none (KH2's default 100)" },
    Boss { name: "Leon", id: 101, vtable: 0x0065_DDE8, rtti: "CTutorialLeon", alloc: 0x0018_9541, size: 0xD10, limit: Limit::Fixed(100.0), attack: None, kh2: "N_HB530_BTL2 (owner: 100)" },
    Boss { name: "Cloud", id: 106, vtable: 0x0066_0BA8, rtti: "CBossCloud", alloc: 0x0018_9595, size: 0xD30, limit: Limit::Fixed(100.0), attack: Some((6, "Omnislash (bushin dashes)")), kh2: "N_HB550_BTL2 (100)" },
    Boss { name: "Hades", id: 107, vtable: 0x0066_2038, rtti: "CBossHades", alloc: 0x0018_95BF, size: 0xD80, limit: Limit::Hades, attack: Some((5, "fire-up (unflinching, flames)")), kh2: "B_HE030 (80, 50 below 30 % HP)" },
    Boss { name: "Captain Hook", id: 112, vtable: 0x0066_AE18, rtti: "CBossHook", alloc: 0x0018_9613, size: 0xD90, limit: Limit::Fixed(100.0), attack: Some((3, "sword slash")), kh2: "Barbossa B_CA010 (default 100)" },
    Boss { name: "Vexen", id: 114, vtable: 0x0066_36B0, rtti: "CBossVexen", alloc: 0x0018_963D, size: 0xD70, limit: Limit::Fixed(150.0), attack: Some((6, "ice needles")), kh2: "N_CM040_BTL (AS 150)" },
    Boss { name: "Riku", id: 116, vtable: 0x0066_4C50, rtti: "CBossRiku", alloc: 0x0018_9667, size: 0xD70, limit: Limit::Fixed(100.0), attack: Some((4, "slash (p003_039)")), kh2: "none (KH2's default 100)" },
    Boss { name: "Larxene", id: 117, vtable: 0x0066_6598, rtti: "CBossLarxene", alloc: 0x0018_9691, size: 0xDA0, limit: Limit::Fixed(100.0), attack: Some((4, "hop back, then lightning from range")), kh2: "B_EX400 (AS 100)" },
    Boss { name: "Axel", id: 118, vtable: 0x0066_7CD8, rtti: "CBossAxel", alloc: 0x0018_96BB, size: 0xD70, limit: Limit::Fixed(92.0), attack: Some((4, "warp, then a flame burst around him")), kh2: "B_EX110 (unit arg 1 = 0: 92)" },
    Boss { name: "Marluxia", id: 119, vtable: 0x0067_EB60, rtti: "CBossMarluxia", alloc: 0x0018_9A2D, size: 0xD50, limit: Limit::Fixed(100.0), attack: Some((2, "scythe cut")), kh2: "N_CM000_BTL (AS 100)" },
    Boss { name: "Marluxia 2", id: 120, vtable: 0x0067_AF90, rtti: "CBossMarluxia2", alloc: 0x0018_995B, size: 0xDF0, limit: Limit::Fixed(100.0), attack: Some((2, "death scythe")), kh2: "N_CM000_BTL (AS 100)" },
    Boss { name: "Marluxia 3", id: 132, vtable: 0x0067_85B8, rtti: "CBossMarluxia3", alloc: 0x0018_99D9, size: 0xF50, limit: Limit::Fixed(100.0), attack: Some((2, "scythe strike")), kh2: "N_CM000_BTL (AS 100)" },
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
