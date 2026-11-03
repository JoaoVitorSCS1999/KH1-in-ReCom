//! KH1's specials at the KH1 setting (roadmap M9 stage 9c; `docs/recom-m9-abilities.md` § 9c.1).
//! Stage 9c.1 builds Sonic Blade; the offer already walks KH1's five in KH1's order.
//!
//! KH1 (Steam; Checked (decomp + exe + files) unless tagged):
//! - **The offer** (`0x14028D0F0`, the last step of the context build `0x1402948F0`): with no
//!   special running (`0x142867364 & 0x10000` clear), Sora in state 0, the specials' file loaded
//!   and the target (lock-on or auto-target, `0x142DB3518`) a live enemy, it walks the five
//!   specials in the order `0x14052834C` (Trinity, Ragnarok, Ars Arcanum, Sonic Blade, Strike
//!   Raid); each equipped one in range (its command's range `+0xE` against the horizontal gap to
//!   the target's lock point minus the point's radius, `0x1402B03F0`; Sonic Blade also refuses a
//!   target more than 160 above Sora) scores 2 when its MP cost (btltbl `+0x14C`) can be paid, else
//!   1; the first with the best score goes into the context entry (`0x140293000(3, cmd)`), greyed
//!   when it scored 1 (`0x140292FB0(3)`). KH1 FM never draws a greyed entry 3: its tab doesn't
//!   show (`0x140277CF0`).
//! - **The start** (the context command run by Triangle, `0x1402A3600` → `0x14028D3D0` →
//!   `0x14028D800`): the special-running bit, the specials' motion set swapped in, the special's
//!   script started, and **its MP paid on the press frame** (`0x1402A5B60`). Sonic Blade's script
//!   (`0x14039EAD0`): KH1's scene tint (skipped, owner), state **0x20** (enter `0x14039EC50`: flags
//!   `0x1200080` = no damage and no reaction for the whole special, and the follow-up offer
//!   `0x14039ED70` as the menu's per-frame callback), the wind-up (request 0xC8, blend 8) with an
//!   end callback that starts dash 1 **without a blend** (`0x14039ED60` → `0x14039ECA0(actor, 0)`).
//! - **Each dash** (`0x14039ECA0`): row `0x1B + n` (`0x1402A3980`: its motion, a new attack object,
//!   the attacking bit `weapon +0x42 & 1`), the blend given, an end callback that returns Sora to
//!   state 0 (`0x14039EC90` → `0x1402B1960(actor, 0)`: the special ends), n + 1; the seventh row is
//!   the finisher (`0x21`).
//! - **The follow-ups** (`0x14039ED70`, every menu build during the special): command 0x4D
//!   **Rave** in the Attack row (0x4E **Blast** once n > 5), greyed unless the motion's combo-window
//!   flag (`+0x1CC & 0x400000`: the dash's frames 10–40) is on and the attacking bit is set. A
//!   Triangle press (its tab in) runs it (`0x14039EA40`): toward the target (none: turn 180°), then
//!   the next dash with blend 8. Confirm on that row is the error sound (`0x1402937F0`); a confirm
//!   the menu refuses or a submenu opened clears the attacking bit (`0x140293410`).
//! - **The physics** (`0x14039EBE0`, `0x14039EBA0`): speed 0 (the travel is the motion's root
//!   motion); Sora in the air falls out (motion 6, blend 8, state 2).
//! - **The end** (`0x14039EC10`): the flags and the follow-up offer off.
//!
//! - **Strike Raid** (M12 step 7, owner 2026-11-03: "in KH1FM are triangle prompts. Let them be
//!   triangle prompts and also show up in the sleights menu"; script `0x14039FFD0`): state
//!   **0x27** (enter `0x1403A01D0`: flags `0x1200080` and the follow-up offer `0x1403A0230`; physics
//!   `0x1403A0170`: speed 0; its air callback `0x1402B1840` keeps the state: no falling out), the
//!   wind-up 0xE6 at blend 0 with the first throw as the next entry's callback (`0x1403A0290`).
//!   **Each throw** (`0x1403A0290`, throw n = 1..=5): toward the target (`0x1402A3B00`), row `0x2E +
//!   n` (`0x1402A3980`: its motion 0xE7, the fifth 0xE8; the attacking bit), the high version 0xEA /
//!   0xEB instead when the target's lock point is more than 200 above Sora, then the catch 0xE9 (high:
//!   0xEC), the wait 0xEE (its combo window 0..52 offers the next throw) and the end (state 0,
//!   `0x1403A0220`). The Keyblade flies in the throw motion itself (KH1's weapon bone: no object).
//!   **The follow-up** (`0x1403A0230`): command 0x5F, 0x60 once n > 3, greyed from n = 5 or outside
//!   the window or without the attacking bit; Triangle throws again (`0x14039FFC0` → `0x1403A0290`).
//!   **A clash** (the reaction `0x14029BBD0`'s case 0x27 → `0x1403A00A0`): the knocked-back catch
//!   0xED, the wait, the end; no recoil. MP 2 (btltbl), range 900, no height rule. Checked (decomp +
//!   files).
//!
//! Re:CoM runs it on its own sleight action `0x16` (`crate::movement::Host::start_special`):
//! `initStockSkill`'s writes without the card, its Sonic Blade attack ids (0xE start, 0xF dash,
//! 0x10 finisher) so its own hit sounds and effects play, the KH1 rows in the attack row pointer,
//! its collision (`procAttackCollision` from our `exeStockSkill` wrapper) and damage path with KH1's
//! numbers.

use crate::movement::{Action, Host, Kh1Sora};
use kh1_core::controller::state;
use kh1_core::data::{row, xl};
use kh1_core::motion::{req, request, MotionSet};
use kh1_core::{TargetPart, Vec3};

/// KH1's special commands (the command table `btltbl.bin +0xA58`; Checked (files)).
pub mod cmd {
    pub const SONIC_BLADE: u8 = 0x4B;
    pub const RAVE: u8 = 0x4D;
    pub const BLAST: u8 = 0x4E;
    pub const TRINITY: u8 = 0x52;
    pub const ARS_ARCANUM: u8 = 0x57;
    pub const RAGNAROK: u8 = 0x5A;
    pub const STRIKE_RAID: u8 = 0x5E;
    /// M12 step 7: Strike Raid's follow-ups (`0x1403A0230`: `'_' + (n > 3)`).
    pub const RAID: u8 = 0x5F;
    pub const RAID_LAST: u8 = 0x60;

    /// The commands built so far (the starts and their follow-ups).
    pub fn is_ours(c: u8) -> bool {
        matches!(c, SONIC_BLADE | RAVE | BLAST | STRIKE_RAID | RAID | RAID_LAST)
    }

    /// The special a built start command starts (KH1's `0x14028D3D0`: 0x4B → 0, 0x5E → 2).
    pub fn start_of(c: u8) -> Option<usize> {
        match c {
            SONIC_BLADE => Some(super::SONIC_BLADE),
            STRIKE_RAID => Some(super::STRIKE_RAID),
            _ => None,
        }
    }

    /// The command's name (its record's `+4`): Sonic 319, Rave 321, Blast 322, Trinity 326, Ars
    /// Arcanum 331, Ragnarok 334, Strike Raid 338, its follow-ups 339 / 340. Checked (files).
    pub fn name(c: u8) -> Option<u16> {
        Some(match c {
            SONIC_BLADE => 319,
            RAVE => 321,
            BLAST => 322,
            TRINITY => 326,
            ARS_ARCANUM => 331,
            RAGNAROK => 334,
            STRIKE_RAID => 338,
            RAID => 339,
            RAID_LAST => 340,
            _ => return None,
        })
    }
}

/// KH1's English words of the built specials after Sonic Blade's (US / UK `Word.bin`; the asset step
/// writes each language's): Strike Raid 338 "Strike", 339 "Raid", 340 "Judgment". Checked (files).
pub const ENGLISH_WORDS: &[(u16, &str)] = &[(338, "Strike"), (339, "Raid"), (340, "Judgment")];

/// KH1's specials by index (`0x140528340`: (ability id, command) pairs): 0 Sonic Blade, 1 Ars
/// Arcanum, 2 Strike Raid, 3 Ragnarok, 4 Trinity. Checked (exe).
pub const TABLE: [(u8, u8); 5] = [(11, cmd::SONIC_BLADE), (12, cmd::ARS_ARCANUM), (13, cmd::STRIKE_RAID), (14, cmd::RAGNAROK), (15, cmd::TRINITY)];
/// The offer's walk (`0x14052834C`): Trinity, Ragnarok, Ars Arcanum, Sonic Blade, Strike Raid.
/// Checked (exe).
pub const ORDER: [usize; 5] = [4, 3, 1, 0, 2];
/// MP costs (btltbl `+0x14C + 2 × index`, read by `0x14028D0F0` and paid by `0x14028D800`):
/// 200 / 300 / 200 / 300 / 300 = 2 / 3 / 2 / 3 / 3 MP (`crate::mp`: ≥ 100 = whole points).
/// Checked (files).
pub const COST: [i32; 5] = [200, 300, 200, 300, 300];
/// The commands' ranges (record `+0xE`, KH1 units): 600 / 300 / 900 / 600 / 1200. Checked (files).
pub const RANGE: [f32; 5] = [600.0, 300.0, 900.0, 600.0, 1200.0];
/// Sonic Blade refuses a target more than this above Sora (`0x14028D0F0`, command 'K').
pub const SONIC_MAX_ABOVE: f32 = 160.0;
pub const SONIC_BLADE: usize = 0;
pub const ARS_ARCANUM: usize = 1;
pub const STRIKE_RAID: usize = 2;
pub const RAGNAROK: usize = 3;
/// The specials' names (logs).
pub const NAME: [&str; 5] = ["Sonic Blade", "Ars Arcanum", "Strike Raid", "Ragnarok", "Trinity"];
/// The KH1 rows each built special plays (Sonic Blade's dashes and finisher; M12 step 7, Strike
/// Raid's five throws); an empty range: not built.
#[allow(clippy::reversed_empty_ranges)]
pub const ROWS_OF: [std::ops::RangeInclusive<usize>; 5] = [row::SONIC_BLADE_DASH..=row::SONIC_BLADE_FINISHER, 1..=0, row::STRIKE_RAID_THROW..=row::STRIKE_RAID_FINISHER, 1..=0, 1..=0];

/// KH1's special state (`0x14039EAD0` registers it).
pub const STATE: u32 = 0x20;
/// Each special's KH1 state (its script's `0x1402B1B80` first argument: Sonic Blade / Ars Arcanum
/// 0x20, Strike Raid 0x27, Ragnarok 0x25, Trinity 0x29). Checked (decomp).
pub const STATE_OF: [u32; 5] = [0x20, 0x20, 0x27, 0x25, 0x29];

/// Whether `s` is a built special's KH1 state.
pub fn is_state(s: u32) -> bool {
    s == STATE || s == STATE_OF[STRIKE_RAID]
}

/// Strike Raid's throw rows start at `0x2E + n` (`0x1403A0290`), n = 1..=5; the fifth is the
/// finishing throw.
pub const RAID_THROWS: u8 = 5;
/// A target more than this above Sora makes a throw the high one (`0x1403A0290`: lock point −
/// Sora's position, y < −200; KH1 y is down).
pub const RAID_HIGH_ABOVE: f32 = 200.0;
/// The wind-up's blend and each Rave's (`0x41000000` = 8.0); the first dash has none.
pub const BLEND: f32 = 8.0;
/// The finisher's row is the seventh (`0x14039ECA0`: `0x1B + n`).
pub const ROWS: usize = row::SONIC_BLADE_FINISHER - row::SONIC_BLADE_DASH + 1;

/// Re:CoM's sleight action (`initStockSkill` `0x140220DF0`: `0x1401F8D70(player, 0x16)`), the sub
/// it leaves (`+0x414` = 1) and its phase (`+0x604` = 3). Checked (decomp).
pub const RECOM_ACTION: u32 = 0x16;
pub const RECOM_SUB: u32 = 1;
pub const RECOM_PHASE: u32 = 3;

/// Re:CoM's Sonic Blade sleight (id 9, `initStockSkill` case 9: attack id 0xE; `exeStockSkill`
/// `0x140218950` "SONICRAVE": the start 0x8C, dashes 0x8D–0x92 on attack row 0xF, the finisher
/// 0x93 on 0x10). Its attack ids give Re:CoM's own Sonic Blade sounds and effects to our rows (its
/// hit whoosh `0x140224610`: id 0xF → group 1 sound 0xD1). Checked (decomp + exe).
pub mod recom {
    pub const SLEIGHT: u16 = 9;
    pub const ID_START: u32 = 0xE;
    pub const ID_DASH: u32 = 0xF;
    pub const ID_FINISHER: u32 = 0x10;
    /// M12 step 7: Re:CoM's Strike Raid sleight (id 8, `initStockSkill` case 8: attack id 0xD, no
    /// resource load; its `exeStockSkill` sub-states 0x82 (`p001_074`, action row 0x3F), 0x83 (the
    /// throw `p001_075`, row 0x40, the weapon thrown as an object `0x140226160`), 0x84 (the catch
    /// `p001_076`, row 0x41)). Every KH1 throw takes its attack id. Checked (decomp + exe).
    pub const RAID_SLEIGHT: u16 = 8;
    pub const ID_RAID: u32 = 0xD;
}

/// What the offer sees of the target: the horizontal gap to its lock point minus the point's
/// radius (never below 0, `0x1402B03F0`), and how far the point is above Sora (KH1 y is down:
/// Sora's y − the point's y).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OfferTarget {
    pub gap: f32,
    pub above: f32,
}

impl OfferTarget {
    pub fn new(sora: Vec3, t: &TargetPart) -> OfferTarget {
        let gap = ((t.centre - sora).flat().length() - t.radius).max(0.0);
        OfferTarget { gap, above: sora.y - t.centre.y }
    }
}

/// `0x14028D0F0`'s score of special `i` on the target: 0 out of range (or Sonic Blade's height
/// rule), 1 in range but MP short (greyed), 2 offered.
pub fn score(i: usize, t: &OfferTarget, can_pay: &mut dyn FnMut(i32) -> bool) -> u8 {
    if t.gap < RANGE[i] {
        if i == SONIC_BLADE && SONIC_MAX_ABOVE < t.above {
            0
        } else {
            1 + can_pay(COST[i]) as u8
        }
    } else {
        0
    }
}

/// `0x14028D0F0`: the special offered in the context entry, and whether it is greyed (in range,
/// MP short). `equipped[i]` by special index (only the built ones count); `can_pay(cost)` is KH1's
/// `0x1402A4C10`.
pub fn offer(equipped: &[bool; 5], target: Option<OfferTarget>, can_pay: &mut dyn FnMut(i32) -> bool) -> Option<(u8, bool)> {
    let t = target?;
    let mut best = 0u8;
    let mut pick = None;
    for &i in &ORDER {
        if !equipped[i] {
            continue;
        }
        let score = score(i, &t, can_pay);
        if best < score {
            best = score;
            pick = Some(i);
        }
    }
    pick.map(|i| (TABLE[i].1, best != 2))
}

/// `0x14039ED70`: the follow-up in the Attack row: Rave, or Blast once n > 5 (`'M' + (n > 5)`).
pub fn follow_cmd(counter: u8) -> u8 {
    if counter > 5 {
        cmd::BLAST
    } else {
        cmd::RAVE
    }
}

/// `0x1403A0230`: Strike Raid's follow-up: 0x5F, or 0x60 once n > 3 (`'_' + (n > 3)`).
pub fn raid_cmd(counter: u8) -> u8 {
    if counter > 3 {
        cmd::RAID_LAST
    } else {
        cmd::RAID
    }
}

/// Our request of a special row's motion (the xl set's 0xC9 / 0xCA / 0xE7 / 0xE8 by xl entry).
pub fn row_request(m: &kh1_core::Moveset, r: usize) -> Option<u16> {
    if !ROWS_OF.iter().any(|rows| rows.contains(&r)) {
        return None;
    }
    m.row(r).and_then(|rw| xl::entry_of_kh1_request(rw.request)).map(xl::request)
}

/// Adds KH1's specials' motions to the motion set (keyed by xl entry, `xl::request`).
pub fn add_motions(m: &mut MotionSet) {
    for e in xl::ENTRIES {
        if let Some(info) = xl::motion(e) {
            m.add(&[xl::request(e)], info);
        }
    }
}

/// Where the special is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    WindUp,
    /// A dash or the finisher is playing (`Special::counter` rows started).
    Row,
}

/// A running special (KH1's state 0x20 and its script's variables).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Special {
    /// The special's index (0 Sonic Blade, 2 Strike Raid).
    pub index: usize,
    pub phase: Phase,
    /// Rows started (Sonic Blade: KH1's `0x142F10750`, 1..=6 the dashes, 7 the finisher; Strike
    /// Raid: `0x142F114CC`, the throws 1..=5).
    pub counter: u8,
    /// KH1's attacking bit (`weapon +0x42 & 1`): set at each row start, cleared by a refused
    /// menu confirm.
    pub attack_bit: bool,
    /// Frames since the press (log and trace).
    pub age: f32,
}

/// A row for Re:CoM to start (`Host::special_row`): the KH1 row, Re:CoM's attack id for it, our
/// request of its motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowStart {
    pub kh1_row: usize,
    pub recom_id: u32,
    pub request: u16,
    pub counter: u8,
}

/// Re:CoM's sleight action for a KH1 special (`Host::start_special`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecialStart {
    pub index: usize,
    /// KH1's cost, paid on the press frame.
    pub cost: i32,
}

/// What the KH1 frame did with a special (log and trace).
#[derive(Clone, Debug, PartialEq)]
pub enum SpecialEvent {
    Started { index: usize },
    Row { counter: u8, kh1_row: usize, frame: f32 },
    Refused { command: u8, why: &'static str },
    /// M12 step 7: Strike Raid's thrown Keyblade clashed (`0x1403A00A0`).
    Clashed { frame: f32 },
    Ended { why: &'static str, frame: f32 },
}

impl SpecialEvent {
    pub fn token(&self) -> String {
        match self {
            SpecialEvent::Started { index } => format!("special({})", TABLE.get(*index).map_or(0, |t| t.1)),
            SpecialEvent::Row { counter, kh1_row, frame } => format!("sp-row({kh1_row:#x} n{counter} f{frame:.0})"),
            SpecialEvent::Refused { command, why } => format!("sp-refused({command:#x} {why})"),
            SpecialEvent::Clashed { frame } => format!("sp-clash(f{frame:.0})"),
            SpecialEvent::Ended { why, frame } => format!("sp-end({why} f{frame:.0})"),
        }
    }
}

impl Kh1Sora {
    /// `0x140293480` for a special command: not in the take-off; free or in a combo window
    /// (`0x1402A2EC0`). Checked (decomp: the record types 0x4B / 0x4D / 0x4E have no other test).
    pub fn accepts_special(&self) -> bool {
        self.state != state::TAKE_OFF && (!self.lock || self.in_window())
    }

    /// Re:CoM's action while a special runs (its sleight action, as `initStockSkill` leaves it).
    pub(crate) fn special_action(&self) -> Option<Action> {
        self.special.as_ref().map(|_| Action { id: RECOM_ACTION, sub: RECOM_SUB, phase: RECOM_PHASE })
    }

    /// The follow-up in the Attack row while a special runs (`0x14039ED70`): its command and
    /// whether it is greyed (outside the combo window, or the attacking bit cleared).
    ///
    /// Read between frames (the menu runs before Sora's frame): a row started by the last frame is
    /// pending for its first update, where KH1's menu already sees it at its frame 0 (KH1 starts
    /// it in the update after its think): no window then.
    pub fn follow_up(&self) -> Option<(u8, bool)> {
        let s = self.special.as_ref()?;
        let window = self.in_window() && !self.rec.start_now;
        Some(match s.index {
            // `0x1403A0230`: no next throw after the fifth.
            STRIKE_RAID => (raid_cmd(s.counter), !(s.counter < RAID_THROWS && window && s.attack_bit)),
            _ => (follow_cmd(s.counter), !(window && s.attack_bit)),
        })
    }

    /// A menu confirm KH1 refused (or a submenu it opened) during the special: the attacking bit
    /// goes (`0x140293410`).
    pub fn special_confirm_refused(&mut self) {
        if let Some(s) = self.special.as_mut() {
            s.attack_bit = false;
        }
    }

    /// A special command this frame (the context tab's Triangle): the start, or a follow-up.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn command_special(&mut self, h: &mut dyn Host, c: u8, target: Option<TargetPart>, pos: Vec3, dir: &mut Vec3, w: &mut f32) {
        let refuse = |me: &mut Kh1Sora, why| me.special_log.push(SpecialEvent::Refused { command: c, why });
        if !self.accepts_special() {
            return refuse(self, "not now");
        }
        match c {
            cmd::SONIC_BLADE | cmd::STRIKE_RAID => self.start_special(h, c, target, pos, dir, w),
            cmd::RAID | cmd::RAID_LAST => {
                let Some(s) = self.special.filter(|s| s.index == STRIKE_RAID) else { return refuse(self, "no special") };
                if self.follow_up() != Some((c, false)) || s.counter >= RAID_THROWS {
                    return refuse(self, "greyed");
                }
                // `0x14039FFC0` → `0x1403A0290`: the next throw (it turns toward the target itself).
                self.raid_throw(h, target, pos);
                *dir = Vec3::from_yaw(self.target_yaw);
                *w = 0.0;
            }
            cmd::RAVE | cmd::BLAST => {
                let Some(s) = self.special.filter(|s| s.index == SONIC_BLADE) else { return refuse(self, "no special") };
                if self.follow_up() != Some((c, false)) || s.counter as usize >= ROWS {
                    return refuse(self, "greyed");
                }
                // `0x14039EA40`: toward the target, else turn 180°; then the next dash, blend 8.
                match target {
                    Some(t) => {
                        let d = (t.centre - pos).flat().normalized();
                        if !d.is_zero_xz() {
                            self.target_yaw = d.yaw();
                        }
                    }
                    None => self.target_yaw = kh1_core::math::wrap_angle(self.facing + std::f32::consts::PI),
                }
                *dir = Vec3::from_yaw(self.target_yaw);
                *w = 0.0;
                self.special_row(h, BLEND);
            }
            _ => refuse(self, "not built"),
        }
    }

    /// `0x14028D800` + the special's script: Sonic Blade `0x14039EAD0`, Strike Raid `0x14039FFD0`.
    fn start_special(&mut self, h: &mut dyn Host, c: u8, target: Option<TargetPart>, pos: Vec3, dir: &mut Vec3, w: &mut f32) {
        let refuse = |me: &mut Kh1Sora, why| me.special_log.push(SpecialEvent::Refused { command: c, why });
        let Some(index) = cmd::start_of(c) else { return refuse(self, "not built") };
        if self.state != state::GROUND || self.special.is_some() {
            return refuse(self, "state");
        }
        // The command record's flag 2 (`0x1402A38E0`): toward the target's lock point.
        if let Some(t) = target {
            let d = (t.centre - pos).flat().normalized();
            if !d.is_zero_xz() {
                *dir = d;
                self.target_yaw = d.yaw();
            }
        }
        *w = 0.0;
        // A swing playing (the press from a combo window): Re:CoM ends its attack action first.
        self.leave_swing(h);
        if self.recom_took {
            return refuse(self, "Re:CoM's win");
        }
        if !h.start_special(&SpecialStart { index, cost: COST[index] }) {
            return refuse(self, "no Re:CoM rows or MP");
        }
        self.set_state(STATE_OF[index]);
        self.special = Some(Special { index, phase: Phase::WindUp, counter: 0, attack_bit: false, age: 0.0 });
        if index == STRIKE_RAID {
            // `0x14039FFD0`: the wind-up 0xE6 at blend 0; the next entry (0xE7) calls the first
            // throw when the wind-up ends (`0x1403A0290`, which plays its row's motion afresh).
            self.play(xl::request(xl::SR_WINDUP) as u32 | req::ACTION, 0.0);
        } else {
            self.play(xl::request(xl::WINDUP) as u32 | req::ACTION, BLEND);
        }
        // The end callback entry (`0x14029FA30(actor, 0x40000000, 0)` with `0x14039ED60`).
        self.append(req::RELEASE | request::IDLE as u32, 0.0);
        self.special_log.push(SpecialEvent::Started { index });
    }

    /// `0x1403A0290`: Strike Raid's next throw (n + 1): toward the target (`0x1402A3B00`: its lock
    /// point, else the facing kept), its row `0x2E + n` (the fifth the finishing throw), the high
    /// motion when the target is more than 200 above, then the catch, the wait and the end queued.
    fn raid_throw(&mut self, h: &mut dyn Host, target: Option<TargetPart>, pos: Vec3) {
        let Some(mut s) = self.special.filter(|s| s.index == STRIKE_RAID) else { return };
        if let Some(t) = target {
            let d = (t.centre - pos).flat().normalized();
            if !d.is_zero_xz() {
                self.target_yaw = d.yaw();
            }
        }
        let n = s.counter + 1;
        let r = row::STRIKE_RAID_THROW + s.counter as usize;
        let high = target.is_some_and(|t| pos.y - t.centre.y > RAID_HIGH_ABOVE);
        let request = match (high, n == RAID_THROWS) {
            (true, true) => xl::request(xl::SR_FINISHER_HIGH),
            (true, false) => xl::request(xl::SR_THROW_HIGH),
            _ => match row_request(&self.moveset, r) {
                Some(q) => q,
                None => return,
            },
        };
        h.special_row(&RowStart { kh1_row: r, recom_id: recom::ID_RAID, request, counter: n });
        self.play(request as u32 | req::ACTION, 0.0);
        self.append(xl::request(if high { xl::SR_CATCH_HIGH } else { xl::SR_CATCH }) as u32, 0.0);
        self.append(xl::request(xl::SR_WAIT) as u32, 0.0);
        self.append(req::RELEASE | request::IDLE as u32, 0.0);
        s.counter = n;
        s.phase = Phase::Row;
        s.attack_bit = true;
        self.special = Some(s);
        self.clashed = false;
        self.special_log.push(SpecialEvent::Row { counter: n, kh1_row: r, frame: s.age });
    }

    /// The reaction `0x14029BBD0` in Strike Raid's state 0x27 (a clash of the thrown Keyblade, kind
    /// 6 / 7: KH1's only reactions through its flags `0x1200080`) → `0x1403A00A0`: the knocked-back
    /// catch 0xED at blend 0, the wait and the end; no recoil, Sora stays in the special. True when
    /// it took the clash.
    pub(crate) fn special_clash(&mut self, h: &mut dyn Host) -> bool {
        let Some(s) = self.special.filter(|s| s.index == STRIKE_RAID) else { return false };
        h.special_row_end();
        self.play(xl::request(xl::SR_CLASH) as u32 | req::ACTION, 0.0);
        self.append(xl::request(xl::SR_WAIT) as u32, 0.0);
        self.append(req::RELEASE | request::IDLE as u32, 0.0);
        self.special_log.push(SpecialEvent::Clashed { frame: s.age });
        true
    }

    /// `0x14039ECA0`: the next row (a dash, the seventh the finisher).
    fn special_row(&mut self, h: &mut dyn Host, blend: f32) {
        let Some(mut s) = self.special else { return };
        let r = row::SONIC_BLADE_DASH + s.counter as usize;
        let Some(request) = row_request(&self.moveset, r) else { return };
        let rs = RowStart { kh1_row: r, recom_id: if r == row::SONIC_BLADE_FINISHER { recom::ID_FINISHER } else { recom::ID_DASH }, request, counter: s.counter + 1 };
        h.special_row(&rs);
        self.play(request as u32 | req::ACTION, blend);
        self.append(req::RELEASE | request::IDLE as u32, 0.0);
        s.counter += 1;
        s.phase = Phase::Row;
        s.attack_bit = true;
        self.special = Some(s);
        self.clashed = false;
        self.special_log.push(SpecialEvent::Row { counter: s.counter, kh1_row: r, frame: s.age });
    }

    /// The special motion's end callback, when the queue moves on from it (from the motion
    /// update): after the wind-up, dash 1 without a blend (`0x14039ED60`); after a row, state 0 and
    /// the queued idle (`0x14039EC90`). Strike Raid: after the wind-up the first throw
    /// (`0x1403A0290`); after a throw its row's attack ends and the catch plays (`0x1403A0440`: its
    /// effect); after the wait state 0 (`0x1403A0220`). True when a new row took the motion (the
    /// update stops).
    pub(crate) fn special_motion_end(&mut self, h: &mut dyn Host, ended: u16) -> bool {
        let Some(s) = self.special else { return false };
        if s.index == STRIKE_RAID {
            let Some(e) = xl::entry_of(ended) else { return false };
            return match e {
                xl::SR_WINDUP => {
                    let pos = h.pos();
                    self.raid_throw(h, self.special_target, pos);
                    true
                }
                xl::SR_THROW | xl::SR_FINISHER | xl::SR_THROW_HIGH | xl::SR_FINISHER_HIGH => {
                    h.special_row_end();
                    false
                }
                xl::SR_WAIT => {
                    self.end_special(h, "done");
                    false
                }
                _ => false,
            };
        }
        match s.phase {
            Phase::WindUp => {
                // The callback's request starts at the next update, as any request of the think;
                // the frame sends it to Re:CoM at its end (`Kh1Sora::issue`). Assumption: KH1
                // starts a callback's request on the update after the one that ran it.
                self.special_row(h, 0.0);
                true
            }
            Phase::Row => {
                self.end_special(h, "done");
                false
            }
        }
    }

    /// `0x14039EBA0`: Sora in the air during the special falls out (motion 6, blend 8, state 2).
    /// Strike Raid's air callback (`0x1402B1840`) only keeps the state: false, nothing ends.
    pub(crate) fn special_fall(&mut self, h: &mut dyn Host) -> bool {
        match self.special {
            None => return false,
            Some(s) if s.index == STRIKE_RAID => return false,
            Some(_) => {}
        }
        self.end_special(h, "airborne");
        self.play(request::FALL as u32, BLEND);
        true
    }

    /// `0x14039EC10`: the special ends (state 0 unless the caller sets another), its flags go, and
    /// Re:CoM's sleight action ends (`0x1401F6660`) unless its win took over.
    pub(crate) fn end_special(&mut self, h: &mut dyn Host, why: &'static str) {
        let Some(s) = self.special.take() else { return };
        self.special_log.push(SpecialEvent::Ended { why, frame: s.age });
        if is_state(self.state) {
            self.set_state(state::GROUND);
        }
        if self.swing.is_none() && self.cast.is_none() && self.item.is_none() && h.end_special(self.state != state::GROUND) {
            self.recom_took = true;
        }
    }

    /// The special's per-frame part: its age.
    pub(crate) fn special_tick(&mut self, step: f32) {
        if let Some(s) = self.special.as_mut() {
            s.age += step;
        }
    }

    /// The special events (drained).
    pub fn take_special_log(&mut self) -> Vec<SpecialEvent> {
        std::mem::take(&mut self.special_log)
    }
}

/// Base Re:CoM's Sonic Blade voices, sounds and effects (owner, 2026-11-01: "however it works in
/// base ReCoM"): Re:CoM's sleight plays them from its own sleight motions' sound and effect events,
/// not from code. Checked (decomp + exe):
/// - `exeStockSkill` (`0x140218950`) plays each part of "SONICRAVE" with `0x1401F73A0(player,
///   row[0], 0, 1)` (stack: EET variant, SES variant): the start (sub-state 0x8C, row 0xE: motion
///   0, variants row[1] = 0 and Sora's vtable `+0x110(player, 3)` = 0 for id 0xE), each of the six
///   dashes (0x8D..0x92, row 0xF: motion 1, variants 0, 0) and the finisher (0x93, row 0x10:
///   motion 2, variants 0, 0). Its per-state code has no effect or sound call of its own (only the
///   `limit1` effect at the start, and the weapon's sound for id 0xF, `0x140224610`).
/// - `0x1401F73A0` → `0x1401A3580(player, pack, motion, …)`, pack = `0x14020FE60(+0x3B8, sleight
///   9)` + 10000 = 0x3F1 + 10000 (`0x14020F720`: 9 → 2; `0x14020FB10`: 2 → 0x3F1). Besides playing
///   the sub-pack motion, `0x1401A3580` stops Sora's own event players (`+0x108` effects, `+0x118`
///   sounds: vtable `+0x10`) and starts the sleight's: `0x140179A30(+0x110, pack)` +
///   `0x14017A240(+0x110, motion, EET variant, 1)` unless `+0x154 & 0x400`, `0x140238710(+0x120,
///   pack)` + `0x140239260(+0x120, motion, SES variant, 1)` unless `+0x154 & 0x4000`. The
///   players step on the motion controller's clock every update (`0x1401A82E0`, vtable `+0x18`);
///   the next own motion stops them (`0x1401A3470`, when the sub-pack flag `ctrl+0x52 & 0x200` is
///   set).
///
/// Re:CoM's structure is KH1's (a start, six dashes, a finisher), so each KH1 motion of the special
/// starts the events of the Re:CoM motion it stands for (`win::specials::motion_events`), on KH1's
/// clip and clock: Re:CoM's events at Re:CoM's frames. Assumption: Re:CoM's sleight motions are
/// as long as KH1's (its pack is not in the session).
pub mod recom_events {
    use kh1_core::data::xl;

    /// Re:CoM's sleight motion pack id for Sonic Blade (0x3F1 + 10000).
    pub const PACK: u32 = 10_000 + 0x3F1;
    /// Re:CoM's motion for each xl entry (wind-up, dash, finisher): rows 0xE / 0xF / 0x10 byte 0.
    pub const MOTION: [u32; 3] = [0, 1, 2];
    /// The variants `exeStockSkill` passes (EET, SES): 0 for all three.
    pub const EET_VARIANT: u32 = 0;
    pub const SES_VARIANT: u32 = 0;

    /// Re:CoM's sleight motion for a KH1 xl entry (Sonic Blade's).
    pub fn motion_of(xl_entry: usize) -> Option<u32> {
        MOTION.get(xl_entry).copied()
    }

    /// Where a KH1 special motion's Re:CoM events come from.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Source {
        /// A motion of a sleight's own pack, started on Sora's sleight event players
        /// (`player+0x110` / `+0x120`), as `0x1401A3580` does (Sonic Blade).
        Sleight { pack: u32, motion: u32 },
        /// One of Sora's own motions, by its row of Sora's action table (`0x1401F7180` resolves the
        /// row's slot, `0x1407C6300[player+0x398]` row `+8`), started on his own players
        /// (`player+0x108` / `+0x118`), as his own motion play `0x1401A3470` does (M12 step 7:
        /// Re:CoM's Strike Raid plays Sora's own `p001_074` / `p001_075` / `p001_076`).
        Own { row: u32 },
    }

    /// The Re:CoM events of a KH1 special motion: their source and the (EET, SES) variants
    /// `exeStockSkill` passes for them. Checked (decomp `0x140218950`):
    /// - Sonic Blade: start / dashes / finisher → pack 11009 motions 0 / 1 / 2, variants 0.
    /// - Strike Raid (sub-states 0x82 → 0x83 → 0x84): the wind-up → row 0x3F (`p001_074`, `row 0xD
    ///   [0] + +0x478`) with EET variant `row 0xD [1]` = 1 and SES variant Sora's vtable `+0x110(3)` =
    ///   0 (`0x14021FEF0` lists no 0xD); each throw (and its high version) → row 0x40 (`p001_075`),
    ///   EET 1, SES 0; each catch → row 0x41 (`p001_076`), 0, 0. KH1's knocked-back catch 0xED →
    ///   the catch's too (Assumption: Re:CoM has no other); the wait 0xEE: none (Re:CoM has no wait).
    pub fn of(xl_entry: usize) -> Option<(Source, u32, u32)> {
        Some(match xl_entry {
            xl::WINDUP | xl::DASH | xl::FINISHER => (Source::Sleight { pack: PACK, motion: motion_of(xl_entry)? }, EET_VARIANT, SES_VARIANT),
            xl::SR_WINDUP => (Source::Own { row: 0x3F }, 1, 0),
            xl::SR_THROW | xl::SR_FINISHER | xl::SR_THROW_HIGH | xl::SR_FINISHER_HIGH => (Source::Own { row: 0x40 }, 1, 0),
            xl::SR_CATCH | xl::SR_CATCH_HIGH | xl::SR_CLASH => (Source::Own { row: 0x41 }, 0, 0),
            _ => return None,
        })
    }
}

/// The trace token: `sp -` or `sp <index> <phase> n<counter> f<age> [bit]`.
pub fn trace_token(s: Option<&Special>) -> String {
    match s {
        None => "sp -".into(),
        Some(s) => format!("sp {} {:?} n{} f{:.0}{}", s.index, s.phase, s.counter, s.age, if s.attack_bit { " bit" } else { "" }),
    }
}

#[cfg(test)]
mod tests;
