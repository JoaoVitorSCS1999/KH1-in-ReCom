//! Which Re:CoM action row (Sora's action table `0x1407C5E80`, played by `0x1401F7180`) plays a
//! KH1 motion request, by the owner's rule (decision 11, 2026-10-30): where Re:CoM has the same
//! kind of move, Re:CoM's own animation; KH1's converted clips (the asset step's new `PL0001.ANN`
//! slots) only for moves Re:CoM lacks. `docs/recom-kh1-movement.md` § Motion rows.
//!
//! Re:CoM's rows and clips (Checked (exe): the table's names): 1 `p001_001` field idle, 2 / 0x12
//! `p001_002` walk, 3 `p001_003` / 0x13 `p001_023` run, 4 `p001_004` jump (KH1 entry 8's bake),
//! 5 `p001_005` jump top, 6 `p001_006` fall, 7 `p001_007` land (entry 11's bake), 0x16 `p001_070`
//! Dodge Roll (entry 117's bake), 0x1B–0x1E `p001_154`–`p001_157` glide, glide fast, glide turns
//! (entries 75–78), 0x2A / 0x2B / 0x2C `p001_031` / `p001_020` / `p001_024` battle-stance idle,
//! walk, run. Re:CoM's own picker `0x1401F9380` chooses among the idle / walk / run rows; [`row_for`]
//! does the same.
//!
//! High Jump (decision 17, 0.6.0): Re:CoM's own higher-jump rows 0x14 / 0x15 (`p001_028` /
//! `p001_029`, played by Re:CoM's jump init when `player+0x41C & 0x200000`, and for its springs)
//! play KH1's High Jump take-off (request 9) and top (request 0xA). Its fall and land share the
//! normal jump's rows (KH1 entries 10 / 11).
//!
//! KH1-only (new rows appended to the table, each naming a converted clip from the manifest):
//! Guard (entry 110, request 212) and Counterattack (entry 111, request 213). The guard-success
//! recoil (request 0x6E, entry 72) is Re:CoM's own `p001_081` (row 0x24, its bake).

use kh1_core::motion::request;

/// Rows in Re:CoM's table before its null terminator. Checked (exe).
pub const RECOM_ROWS: u32 = 0x47;

pub mod row {
    pub const IDLE: u32 = 1;
    pub const WALK: u32 = 2;
    pub const RUN: u32 = 3;
    pub const JUMP: u32 = 4;
    pub const JUMP_TOP: u32 = 5;
    pub const FALL: u32 = 6;
    pub const LAND: u32 = 7;
    pub const WALK_B: u32 = 0x12;
    pub const RUN_B: u32 = 0x13;
    pub const ROLL: u32 = 0x16;
    pub const GLIDE: u32 = 0x1B;
    pub const GLIDE_FAST: u32 = 0x1C;
    pub const GLIDE_TURN_A: u32 = 0x1D;
    pub const GLIDE_TURN_B: u32 = 0x1E;
    pub const STANCE_IDLE: u32 = 0x2A;
    pub const STANCE_WALK: u32 = 0x2B;
    pub const STANCE_RUN: u32 = 0x2C;
    /// Re:CoM's higher jump take-off and top (`p001_028` / `p001_029`).
    pub const HIGH_JUMP: u32 = 0x14;
    pub const HIGH_JUMP_TOP: u32 = 0x15;
    /// Re:CoM's `p001_081` (KH1 entry 72's bake, Checked (files): the recoil 0x6E).
    pub const RECOIL: u32 = 0x24;
    /// Re:CoM's Fire cast clips (`initMagic` plays the attack row's `+0` + `+0x478`, `exeMagic`
    /// the next row): `p001_062` / `p001_063` ground, `p001_066` / `p001_067` air. Checked (exe:
    /// the action table's names; the attack row 0x22's `+0` is 0x37).
    pub const FIRE_WINDUP: u32 = 0x37;
    pub const FIRE_RELEASE: u32 = 0x38;
    pub const FIRE_AIR_WINDUP: u32 = 0x3B;
    pub const FIRE_AIR_RELEASE: u32 = 0x3C;
    /// Re:CoM's Blizzard / Gravity clip (`initMagic` plays the attack row 0x23's `+0` = 0x3A, air
    /// + 4): `p001_065` (43 frames at 30 fps = KH1's 86) and `p001_069` (39 = 78). Checked (exe:
    /// the action table's names; the row's `+0`).
    pub const BLIZZARD: u32 = 0x3A;
    pub const BLIZZARD_AIR: u32 = 0x3E;
    /// Re:CoM's Thunder / Cure / Stop / Aero clip (the attack row 0x24's `+0` = 0x39, air + 4):
    /// `p001_064` and `p001_068` (34 frames at 30 fps = KH1's 68). Checked (exe).
    pub const THUNDER: u32 = 0x39;
    pub const THUNDER_AIR: u32 = 0x3D;
    /// Re:CoM's item clip (`exeItem` `0x1401ED6E0` plays row 0x2D): `p001_032`, KH1 entry 58's bake
    /// (request 0x3E, 82 frames). Checked (exe: the action table's name; files: the bake test).
    pub const ITEM: u32 = 0x2D;
}

/// KH1 requests (and mset entries) that play a clip of their own, in the order their rows are
/// appended: Guard, Counterattack, then the combo modifiers (M9 stage 9a): Sliding Dash 208,
/// Hurricane Blast 209, Blitz 210, Vortex 211, Aerial Sweep 214, Ripple Drive 215, Stun Impact 216,
/// Gravity Break 217, Zantetsuken 218. An entry whose clip the asset step added (a KH1-only clip,
/// slot ≥ 161) gets an appended row; an entry Re:CoM already has (its bake: Sliding Dash
/// `p001_103`, Stun Impact `p001_101`, Zantetsuken `p001_104`, by the manifest) plays Re:CoM's own
/// action row of that clip ([`Extension::with_recom`]).
///
/// M11 (the shared enemy-guard step): KH1's strong recoil 0x6F (entry 73, a KH1-only clip,
/// `p001_209` with the owner's files) and air recoil 0x70 (entry 74, Re:CoM's bake `p001_045`, its row
/// 0x26), for a swing that clashes on an enemy's guard row.
pub const KH1_ONLY: [(u16, u32); 13] = [(212, 110), (213, 111), (208, 106), (209, 107), (210, 108), (211, 109), (214, 112), (215, 113), (216, 114), (217, 115), (218, 116), (STRONG_RECOIL, 73), (AIR_RECOIL, 74)];

/// KH1 Guard's request.
pub const GUARD: u16 = 212;
/// KH1 Counterattack's request (row 0x3E).
pub const COUNTERATTACK: u16 = 213;
/// KH1's guard-success recoil (kind 6, `0x14029D720`).
pub const RECOIL: u16 = 0x6E;
/// KH1's strong recoil (kind 7 on the ground, `0x14029D5F0`; 64 frames, 118 cm root travel back).
pub const STRONG_RECOIL: u16 = 0x6F;
/// KH1's air recoil (kinds 6 / 7 in the air, `0x14029D510`; 40 frames, the fall queued).
pub const AIR_RECOIL: u16 = 0x70;

/// The appended rows: `rows[i]` is the row id of `KH1_ONLY[i]`, when its KH1-only clip exists;
/// `recom[i]` Re:CoM's own action row playing that entry's clip, when the manifest maps the entry
/// to one of Re:CoM's clips; `xl[e]` (stage 9c.1) the row of KH1's specials' motion `e` of
/// `xl_limit.dat` (`kh1_core::data::xl`: Sonic Blade's wind-up, dash, finisher), appended after
/// the others when the asset step converted it (the manifest's `xl_limit` section).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Extension {
    pub rows: [Option<u32>; KH1_ONLY.len()],
    pub recom: [Option<u32>; KH1_ONLY.len()],
    pub xl: [Option<u32>; XL_ENTRIES.len()],
    /// M11 (the Neoshadow's grab): the row of KH1's held clip (request 200 of the Neoshadow's `.mset` player
    /// set, the manifest's `held` section), appended after the xl rows; Sora's held reaction plays it
    /// (`crate::held`).
    pub held: Option<u32>,
}

/// KH1's specials' motions the asset step converts (`xl_limit.dat` entries; stage 9c.1: Sonic
/// Blade's three), in the order their rows are appended.
pub const XL_ENTRIES: [usize; kh1_core::data::xl::ENTRIES.len()] = kh1_core::data::xl::ENTRIES;

impl Extension {
    /// Row ids for the KH1 entries the pack holds, in `KH1_ONLY` order, from `RECOM_ROWS` on.
    pub fn plan(have: impl Fn(u32) -> bool) -> Extension {
        let mut e = Extension::default();
        let mut next = RECOM_ROWS;
        for (i, &(_, entry)) in KH1_ONLY.iter().enumerate() {
            if have(entry) {
                e.rows[i] = Some(next);
                next += 1;
            }
        }
        e
    }

    /// The xl entries the pack holds, appended after the KH1-only rows (stage 9c.1).
    pub fn with_xl(mut self, have: impl Fn(usize) -> bool) -> Extension {
        let mut next = RECOM_ROWS + self.count() as u32;
        for (i, &e) in XL_ENTRIES.iter().enumerate() {
            if have(e) {
                self.xl[i] = Some(next);
                next += 1;
            }
        }
        self
    }

    /// The held clip's row (M11, the Neoshadow's grab), appended after the KH1-only and xl rows when the
    /// pack holds it.
    pub fn with_held(mut self, have: bool) -> Extension {
        self.held = have.then(|| RECOM_ROWS + self.count() as u32 + self.xl.iter().flatten().count() as u32);
        self
    }

    /// The action row playing xl entry `e` (stage 9c.1).
    pub fn xl_row(&self, e: usize) -> Option<u32> {
        XL_ENTRIES.iter().position(|&x| x == e).and_then(|i| self.xl[i])
    }

    /// Entries without a KH1-only clip that Re:CoM's own action table plays: `recom_row(entry)` is
    /// Re:CoM's row whose clip the manifest names for the entry.
    pub fn with_recom(mut self, recom_row: impl Fn(u32) -> Option<u32>) -> Extension {
        for (i, &(_, entry)) in KH1_ONLY.iter().enumerate() {
            if self.rows[i].is_none() {
                self.recom[i] = recom_row(entry).filter(|&r| r < RECOM_ROWS);
            }
        }
        self
    }

    /// The action row playing a KH1 request's own clip: ours appended, or Re:CoM's.
    pub fn get(&self, req: u16) -> Option<u32> {
        KH1_ONLY.iter().position(|&(r, _)| r == req).and_then(|i| self.rows[i].or(self.recom[i]))
    }

    pub fn count(&self) -> usize {
        self.rows.iter().flatten().count()
    }

    /// Whether `row` plays one of the KH1 requests above (our appended rows, or Re:CoM's rows of
    /// the modifiers' bakes, or the specials' rows): they end in the battle stance like Re:CoM's
    /// swing rows.
    pub fn has_row(&self, row: i32) -> bool {
        self.rows.iter().chain(self.recom.iter()).chain(self.xl.iter()).flatten().any(|&r| r as i32 == row)
    }
}

/// What the row picker reads from the player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stance {
    /// `player+0x428 & 1`: battle stance.
    pub battle: bool,
    /// `player+0x418 & 2`: picks rows 2 / 3 over 0x12 / 0x13 (`0x1401F9380`).
    pub flag_418_2: bool,
    /// `player+0x470` / `+0x474`: the previous and current row (`0x1401F7180` keeps them).
    pub prev_row: i32,
    pub row: i32,
}

impl Stance {
    /// `0x1401E6AC0`: the idle row is the battle-stance one in battle stance, or while the
    /// previous or current row is a swing / stance row. Checked (decomp).
    pub fn idle_battle(&self) -> bool {
        let swingish = |r: i32| (0x24..=0x25).contains(&r) || (0x30..=0x33).contains(&r);
        self.battle || swingish(self.prev_row) || swingish(self.row) || self.row == 0x2A
    }
}

/// The Re:CoM row for a KH1 request, and whether it is played with root motion. `None`: no row
/// (a KH1-only clip the pack lacks): the caller keeps the current motion.
pub fn row_for(req: u16, s: &Stance, ext: &Extension) -> Option<(u32, bool)> {
    let r = match req {
        request::IDLE => {
            // Our KH1-only rows (Guard, Counterattack) end in the battle stance like Re:CoM's swing
            // rows, so `0x1401E6AC0`'s rule counts them as swing rows: the stance idle 0x2A after
            // them, as after a swing (owner, 2026-10-31: the field Guard snapped to the field idle).
            if s.idle_battle() || ext.has_row(s.row) || ext.has_row(s.prev_row) {
                row::STANCE_IDLE
            } else {
                row::IDLE
            }
        }
        // `0x1401F9380`: walk 2 (or 0x12), run 3 (or 0x13), 0x2B / 0x2C in battle stance.
        request::WALK => {
            if s.battle {
                row::STANCE_WALK
            } else if s.flag_418_2 {
                row::WALK
            } else {
                row::WALK_B
            }
        }
        request::RUN => {
            if s.battle {
                row::STANCE_RUN
            } else if s.flag_418_2 {
                row::RUN
            } else {
                row::RUN_B
            }
        }
        request::JUMP => row::JUMP,
        request::APEX => row::JUMP_TOP,
        // FALL and the High Jump's fall share KH1 entry 10.
        request::FALL | 0x0B => row::FALL,
        // LAND and the High Jump's land share KH1 entry 11.
        request::LAND | request::HIGH_LAND => row::LAND,
        // Decision 17: Re:CoM's own higher jump.
        request::HIGH_JUMP => row::HIGH_JUMP,
        0x0A => row::HIGH_JUMP_TOP,
        request::GLIDE => row::GLIDE,
        request::GLIDE_FAST => row::GLIDE_FAST,
        request::GLIDE_TURN_A => row::GLIDE_TURN_A,
        request::GLIDE_TURN_B => row::GLIDE_TURN_B,
        request::DODGE_ROLL => return Some((row::ROLL, true)),
        // KH1's Guard motion has the root bit (no travel).
        GUARD => return ext.get(req).map(|r| (r, true)),
        // KH1's recoil has no travel; Re:CoM's bake of it is row 0x24.
        RECOIL => row::RECOIL,
        // M11: the strong recoil travels back (request 0x6F's root bit); the air recoil has none.
        STRONG_RECOIL => return ext.get(req).map(|r| (r, true)),
        AIR_RECOIL => return ext.get(req).map(|r| (r, false)),
        COUNTERATTACK => return ext.get(req).map(|r| (r, true)),
        // KH1's Fire cast (M7 stage 7.4): Re:CoM's own cast clips (decision 10: the same moves,
        // KH1's frame counts). The release motions have the root bit (KH1: back 15 / 42.4 cm),
        // as `exeMagic` plays its release clip with root motion.
        crate::magic::cast_req::FIRE_WINDUP => row::FIRE_WINDUP,
        crate::magic::cast_req::FIRE_RELEASE => return Some((row::FIRE_RELEASE, true)),
        crate::magic::cast_req::FIRE_AIR_WINDUP => row::FIRE_AIR_WINDUP,
        crate::magic::cast_req::FIRE_AIR_RELEASE => return Some((row::FIRE_AIR_RELEASE, true)),
        // Blizzard (M7 stage 7.5): KH1's entries 55 / 80 have no root travel.
        crate::magic::cast_req::BLIZZARD => row::BLIZZARD,
        crate::magic::cast_req::BLIZZARD_AIR => row::BLIZZARD_AIR,
        // Thunder (M7 stage 7.6): KH1's entries 56 / 81 have no root travel.
        crate::magic::cast_req::THUNDER => row::THUNDER,
        crate::magic::cast_req::THUNDER_AIR => row::THUNDER_AIR,
        // Cure (M7 stage 7.7): Re:CoM's Cure rows play the same clip as Thunder (row 0x25's `+0` =
        // 0x39); KH1's entries 56 / 81 have no root travel.
        crate::magic::cast_req::CURE => row::THUNDER,
        crate::magic::cast_req::CURE_AIR => row::THUNDER_AIR,
        // Gravity (M7 stage 7.8): Re:CoM's Gravity rows play Blizzard's clip (row 0x26's `+0` =
        // 0x3A); KH1's entries 55 / 80 have no root travel.
        crate::magic::cast_req::GRAVITY => row::BLIZZARD,
        crate::magic::cast_req::GRAVITY_AIR => row::BLIZZARD_AIR,
        // Stop (M7 stage 7.9): Re:CoM's Stop rows play Thunder's clip (row 0x27's `+0` = 0x39).
        crate::magic::cast_req::STOP => row::THUNDER,
        crate::magic::cast_req::STOP_AIR => row::THUNDER_AIR,
        // Aero (M7 stage 7.10): Re:CoM's Aero rows play Thunder's clip (row 0x28's `+0` = 0x39).
        crate::magic::cast_req::AERO => row::THUNDER,
        crate::magic::cast_req::AERO_AIR => row::THUNDER_AIR,
        // An item on self (M8 stage 8a.1): Re:CoM's own item clip, KH1's entry 58 baked (no root
        // travel in either).
        crate::items::USE_SELF => row::ITEM,
        // KH1's specials' motions (stage 9c.1), keyed by xl entry: their converted clips, with
        // root motion where the xl set's request map has the bit (the dash and the finisher).
        r if kh1_core::data::xl::entry_of(r).is_some() => {
            let e = kh1_core::data::xl::entry_of(r)?;
            let root = kh1_core::data::xl::motion(e)?.root_motion;
            return ext.xl_row(e).map(|row| (row, root));
        }
        _ => return None,
    };
    Some((r, false))
}

/// The held clip's appended row (M11, the Neoshadow's grab): `(row id, clip name)`, after
/// [`appended_rows`]' rows.
pub fn held_row(ext: &Extension, name: Option<String>) -> Option<(u32, String)> {
    Some((ext.held?, name?))
}

/// The table rows to append after Re:CoM's 0x47: `(row id, clip name)`, from the manifest's KH1
/// entries (`motions::clip_for_entry`) and its xl entries (stage 9c.1).
pub fn appended_rows(ext: &Extension, name_of_entry: impl Fn(u32) -> Option<String>, name_of_xl: impl Fn(usize) -> Option<String>) -> Vec<(u32, String)> {
    let mut v: Vec<(u32, String)> = KH1_ONLY
        .iter()
        .zip(ext.rows.iter())
        .filter_map(|(&(_, entry), r)| Some((r.as_ref().copied()?, name_of_entry(entry)?)))
        .collect();
    v.extend(XL_ENTRIES.iter().zip(ext.xl.iter()).filter_map(|(&e, r)| Some(((*r)?, name_of_xl(e)?))));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locomotion_rows_follow_recoms_picker() {
        let field = Stance { flag_418_2: true, ..Default::default() };
        let ext = Extension::default();
        assert_eq!(row_for(0, &field, &ext), Some((row::IDLE, false)));
        // After the Guard or Counterattack row (ours), the stance idle, as after a swing.
        let both = Extension::plan(|_| true);
        let guard = both.get(GUARD).expect("guard row");
        let after_guard = Stance { row: guard as i32, ..field };
        assert_eq!(row_for(0, &after_guard, &both), Some((row::STANCE_IDLE, false)));
        let after_guard_prev = Stance { prev_row: guard as i32, row: row::IDLE as i32, ..field };
        assert_eq!(row_for(0, &after_guard_prev, &both), Some((row::STANCE_IDLE, false)));
        let counter = both.get(COUNTERATTACK).expect("counterattack row");
        assert_eq!(row_for(0, &Stance { row: counter as i32, ..field }, &both), Some((row::STANCE_IDLE, false)));
        assert_eq!(row_for(0, &field, &both), Some((row::IDLE, false)), "a plain field idle stays Re:CoM's");
        assert_eq!(row_for(1, &field, &ext), Some((row::WALK, false)));
        assert_eq!(row_for(2, &field, &ext), Some((row::RUN, false)));
        let b = Stance { flag_418_2: false, ..Default::default() };
        assert_eq!(row_for(1, &b, &ext).unwrap().0, row::WALK_B);
        assert_eq!(row_for(2, &b, &ext).unwrap().0, row::RUN_B);
        let battle = Stance { battle: true, ..Default::default() };
        assert_eq!(row_for(0, &battle, &ext).unwrap().0, row::STANCE_IDLE);
        assert_eq!(row_for(1, &battle, &ext).unwrap().0, row::STANCE_WALK);
        assert_eq!(row_for(2, &battle, &ext).unwrap().0, row::STANCE_RUN);
        // Right after a swing row the idle is the stance idle (0x1401E6AC0).
        let after_swing = Stance { row: 0x31, ..Default::default() };
        assert_eq!(row_for(0, &after_swing, &ext).unwrap().0, row::STANCE_IDLE);
    }

    #[test]
    fn jump_glide_roll_rows() {
        let s = Stance::default();
        let none = Extension::default();
        assert_eq!(row_for(4, &s, &none), Some((row::JUMP, false)));
        assert_eq!(row_for(5, &s, &none), Some((row::JUMP_TOP, false)));
        assert_eq!(row_for(6, &s, &none), Some((row::FALL, false)));
        assert_eq!(row_for(0xB, &s, &none), Some((row::FALL, false)));
        assert_eq!(row_for(7, &s, &none), Some((row::LAND, false)));
        assert_eq!(row_for(0xC, &s, &none), Some((row::LAND, false)));
        assert_eq!(row_for(0x71, &s, &none), Some((0x1B, false)));
        assert_eq!(row_for(0x74, &s, &none), Some((0x1E, false)));
        assert_eq!(row_for(0xDC, &s, &none), Some((row::ROLL, true)));
        // High Jump plays Re:CoM's own higher jump (decision 17); Guard needs its converted clip.
        assert_eq!(row_for(9, &s, &none), Some((row::HIGH_JUMP, false)));
        assert_eq!(row_for(0xA, &s, &none), Some((row::HIGH_JUMP_TOP, false)));
        assert_eq!(row_for(GUARD, &s, &none), None);
        assert_eq!(row_for(200, &s, &none), None);
    }

    #[test]
    fn fire_cast_rows_are_recoms_own_clips() {
        let s = Stance::default();
        let none = Extension::default();
        assert_eq!(row_for(0x36, &s, &none), Some((0x37, false)));
        assert_eq!(row_for(0x3D, &s, &none), Some((0x38, true)));
        assert_eq!(row_for(0x83, &s, &none), Some((0x3B, false)));
        assert_eq!(row_for(0x8A, &s, &none), Some((0x3C, true)));
        // Each release clip is the row after its wind-up (`exeMagic` plays `+0x474 + 1`), and the
        // air clips are the ground ones + 4 (`initMagic`'s `+0x478`).
        assert_eq!(row::FIRE_RELEASE, row::FIRE_WINDUP + 1);
        assert_eq!(row::FIRE_AIR_WINDUP, row::FIRE_WINDUP + 4);
        assert_eq!(row::FIRE_AIR_RELEASE, row::FIRE_AIR_WINDUP + 1);
        assert_eq!(crate::magic::RECOM_FIRE_ROWS[0][0] as u32, row::FIRE_WINDUP);
        // Blizzard: Re:CoM's row 0x23's clip, + 4 in the air; no root travel.
        assert_eq!(row_for(0x37, &s, &none), Some((0x3A, false)));
        assert_eq!(row_for(0x84, &s, &none), Some((0x3E, false)));
        assert_eq!(crate::magic::RECOM_BLIZZARD_ROWS[0][0] as u32, row::BLIZZARD);
        assert_eq!(row::BLIZZARD_AIR, row::BLIZZARD + 4);
        // Thunder: Re:CoM's row 0x24's clip, + 4 in the air.
        assert_eq!(row_for(0x38, &s, &none), Some((0x39, false)));
        assert_eq!(row_for(0x85, &s, &none), Some((0x3D, false)));
        assert_eq!(crate::magic::RECOM_THUNDER_ROWS[0][0] as u32, row::THUNDER);
        assert_eq!(row::THUNDER_AIR, row::THUNDER + 4);
        // Cure: Re:CoM's row 0x25's clip (the same as Thunder's), + 4 in the air.
        assert_eq!(row_for(0x39, &s, &none), Some((0x39, false)));
        assert_eq!(row_for(0x86, &s, &none), Some((0x3D, false)));
        assert_eq!(crate::magic::RECOM_CURE_ROWS[0][0] as u32, row::THUNDER);
        // Gravity: Re:CoM's row 0x26's clip (Blizzard's), + 4 in the air.
        assert_eq!(row_for(0x3A, &s, &none), Some((0x3A, false)));
        assert_eq!(row_for(0x87, &s, &none), Some((0x3E, false)));
        assert_eq!(crate::magic::RECOM_GRAVITY_ROWS[0][0] as u32, row::BLIZZARD);
        // Stop: Re:CoM's row 0x27's clip (Thunder's), + 4 in the air.
        assert_eq!(row_for(0x3B, &s, &none), Some((0x39, false)));
        assert_eq!(row_for(0x88, &s, &none), Some((0x3D, false)));
        assert_eq!(crate::magic::RECOM_STOP_ROWS[0][0] as u32, row::THUNDER);
        // Aero: Re:CoM's row 0x28's clip (Thunder's), + 4 in the air.
        assert_eq!(row_for(0x3C, &s, &none), Some((0x39, false)));
        assert_eq!(row_for(0x89, &s, &none), Some((0x3D, false)));
        assert_eq!(crate::magic::RECOM_AERO_ROWS[0][0] as u32, row::THUNDER);
    }

    #[test]
    fn item_use_plays_recoms_item_clip() {
        let s = Stance::default();
        assert_eq!(row_for(crate::items::USE_SELF, &s, &Extension::default()), Some((row::ITEM, false)));
    }

    /// M9 9a: the modifiers Re:CoM has baked (the manifest maps them to Re:CoM's clips) play
    /// Re:CoM's own rows; the KH1-only ones get appended rows after Guard and Counterattack.
    #[test]
    fn modifier_clips_take_recoms_rows_or_appended_ones() {
        // The owner's manifest: entries 106 / 114 / 116 are Re:CoM's p001_103 / 101 / 104 (rows
        // 0x45 / 0x44 / 0x46 of Re:CoM's table, Checked (exe)); the rest are KH1-only.
        let recom = |e: u32| match e {
            106 => Some(0x45),
            114 => Some(0x44),
            116 => Some(0x46),
            _ => None,
        };
        let e = Extension::plan(|e| recom(e).is_none()).with_recom(recom);
        assert_eq!((e.get(GUARD), e.get(COUNTERATTACK)), (Some(0x47), Some(0x48)));
        assert_eq!((e.get(208), e.get(216), e.get(218)), (Some(0x45), Some(0x44), Some(0x46)));
        assert_eq!([e.get(209), e.get(210), e.get(211), e.get(214), e.get(215), e.get(217)], [Some(0x49), Some(0x4A), Some(0x4B), Some(0x4C), Some(0x4D), Some(0x4E)]);
        // M11 (the shared enemy-guard step): the strong and air recoils after them. In the
        // owner's pack entry 74 is Re:CoM's own bake (slot 45, Checked (files): the BAKES table of
        // `validate.rs`), so it takes Re:CoM's row; this fake manifest has neither as Re:CoM's.
        assert_eq!((e.get(STRONG_RECOIL), e.get(AIR_RECOIL)), (Some(0x4F), Some(0x50)));
        assert_eq!(e.count(), 10);
        assert!(e.has_row(0x44) && e.has_row(0x4E) && !e.has_row(0x30));
        // A row id past Re:CoM's table is never taken as Re:CoM's.
        assert_eq!(Extension::plan(|_| false).with_recom(|_| Some(0x47)).get(208), None);
        // Swings never go through `row_for` (Re:CoM's attack action plays their rows).
        assert_eq!(row_for(216, &Stance::default(), &e), None);
    }

    #[test]
    fn extension_rows_are_appended_in_order() {
        let all = Extension::plan(|_| true);
        assert_eq!(all.rows[..2], [Some(0x47), Some(0x48)]);
        assert_eq!(all.get(218), Some(0x47 + 10));
        let s = Stance::default();
        // The converted High Jump clips are no longer used, even when the pack has them.
        assert_eq!(row_for(9, &s, &all), Some((row::HIGH_JUMP, false)));
        assert_eq!(row_for(GUARD, &s, &all), Some((0x47, true)));
        assert_eq!(row_for(COUNTERATTACK, &s, &all), Some((0x48, true)));
        assert_eq!(row_for(RECOIL, &s, &all), Some((row::RECOIL, false)));
        let no_guard = Extension::plan(|e| e != 110);
        assert_eq!(no_guard.rows[..2], [None, Some(0x47)], "the counterattack moves up when the guard clip is missing");
        let rows = appended_rows(&all, |e| Some(format!("p001_{}", 150 + e)), |_| None);
        assert_eq!(rows[..2], [(0x47, "p001_260".to_string()), (0x48, "p001_261".to_string())]);
        assert_eq!(rows.len(), KH1_ONLY.len());
        assert!(appended_rows(&all, |_| None, |_| None).is_empty());
    }

    /// Stage 9c.1: Sonic Blade's xl clips get rows after the KH1-only ones; our xl requests play
    /// them (wind-up without root motion, dash and finisher with), and the idle after them is the
    /// stance idle.
    #[test]
    fn xl_rows_follow_the_kh1_only_rows() {
        use kh1_core::data::xl;
        let e = Extension::plan(|e| e != 110).with_xl(|_| true);
        let first = RECOM_ROWS + KH1_ONLY.len() as u32 - 1;
        assert_eq!(e.xl[..3], [Some(first), Some(first + 1), Some(first + 2)]);
        // M12 step 7: Strike Raid's nine after Sonic Blade's, in `xl::ENTRIES` order.
        assert_eq!(e.xl_row(xl::SR_WINDUP), Some(first + 3));
        assert_eq!(e.xl_row(xl::SR_WAIT), Some(first + 11));
        let s = Stance::default();
        assert_eq!(row_for(xl::request(xl::WINDUP), &s, &e), Some((first, false)));
        assert_eq!(row_for(xl::request(xl::DASH), &s, &e), Some((first + 1, true)));
        assert_eq!(row_for(xl::request(xl::FINISHER), &s, &e), Some((first + 2, true)));
        assert_eq!(row_for(xl::request(xl::DASH), &s, &Extension::default()), None);
        assert_eq!(row_for(0, &Stance { row: (first + 1) as i32, ..s }, &e), Some((row::STANCE_IDLE, false)));
        let names = appended_rows(&e, |_| Some("k".into()), |x| Some(format!("p001_{}", 230 + x)));
        let n0 = names.len() - kh1_core::data::xl::ENTRIES.len();
        assert_eq!(names[n0..n0 + 3], [(first, "p001_230".to_string()), (first + 1, "p001_231".to_string()), (first + 2, "p001_232".to_string())]);
        // Without the KH1-only rows the xl rows start right after Re:CoM's.
        assert_eq!(Extension::default().with_xl(|x| x == xl::DASH).xl[..3], [None, Some(RECOM_ROWS), None]);
    }

    /// M11 (the Neoshadow's grab): the held clip's row follows the KH1-only and xl rows; no clip, no row; no KH1
    /// request plays it (only the held reaction does).
    #[test]
    fn the_held_row_follows_the_others() {
        let e = Extension::plan(|e| e != 110).with_xl(|_| true).with_held(true);
        let last_xl = e.xl.iter().flatten().copied().max().unwrap();
        assert_eq!(e.held, Some(last_xl + 1));
        assert_eq!(held_row(&e, Some("p001_233".into())), Some((last_xl + 1, "p001_233".to_string())));
        assert_eq!(held_row(&e, None), None);
        assert_eq!(Extension::default().with_held(true).held, Some(RECOM_ROWS));
        assert_eq!(Extension::plan(|_| true).with_held(false).held, None);
        let s = Stance::default();
        assert!((0..=0x200u16).all(|r| row_for(r, &s, &e).map(|x| x.0) != e.held), "no KH1 request plays the held row");
        assert!(!e.has_row(e.held.unwrap() as i32), "Re:CoM's own idle rule after it");
    }
}
