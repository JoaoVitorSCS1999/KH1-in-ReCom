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

    /// The commands this stage runs (the start and Sonic Blade's follow-ups).
    pub fn is_ours(c: u8) -> bool {
        matches!(c, SONIC_BLADE | RAVE | BLAST)
    }

    /// The command's name (its record's `+4`): Sonic 319, Rave 321, Blast 322, Trinity 326, Ars
    /// Arcanum 331, Ragnarok 334, Strike Raid 338. Checked (files).
    pub fn name(c: u8) -> Option<u16> {
        Some(match c {
            SONIC_BLADE => 319,
            RAVE => 321,
            BLAST => 322,
            TRINITY => 326,
            ARS_ARCANUM => 331,
            RAGNAROK => 334,
            STRIKE_RAID => 338,
            _ => return None,
        })
    }
}

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

/// KH1's special state (`0x14039EAD0` registers it).
pub const STATE: u32 = 0x20;
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
        let score = if t.gap < RANGE[i] {
            if i == SONIC_BLADE && SONIC_MAX_ABOVE < t.above {
                0
            } else {
                1 + can_pay(COST[i]) as u8
            }
        } else {
            0
        };
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

/// Our request of a special row's motion (the xl set's 0xC9 / 0xCA by xl entry).
pub fn row_request(m: &kh1_core::Moveset, r: usize) -> Option<u16> {
    if !(row::SONIC_BLADE_DASH..=row::SONIC_BLADE_FINISHER).contains(&r) {
        return None;
    }
    m.row(r).and_then(|rw| xl::entry_of_kh1_request(rw.request)).map(xl::request)
}

/// Adds KH1's specials' motions to the motion set (keyed by xl entry, `xl::request`).
pub fn add_motions(m: &mut MotionSet) {
    for (e, info) in xl::ENTRIES.iter().zip(xl::motions()) {
        m.add(&[xl::request(*e)], info);
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
    /// The special's index (0 Sonic Blade).
    pub index: usize,
    pub phase: Phase,
    /// Rows started (KH1's `0x142F10750`): 1..=6 the dashes, 7 the finisher.
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
    Ended { why: &'static str, frame: f32 },
}

impl SpecialEvent {
    pub fn token(&self) -> String {
        match self {
            SpecialEvent::Started { index } => format!("special({})", TABLE.get(*index).map_or(0, |t| t.1)),
            SpecialEvent::Row { counter, kh1_row, frame } => format!("sp-row({kh1_row:#x} n{counter} f{frame:.0})"),
            SpecialEvent::Refused { command, why } => format!("sp-refused({command:#x} {why})"),
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
        Some((follow_cmd(s.counter), !(window && s.attack_bit)))
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
            cmd::SONIC_BLADE => self.start_sonic_blade(h, target, pos, dir, w),
            cmd::RAVE | cmd::BLAST => {
                let Some(s) = self.special else { return refuse(self, "no special") };
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

    /// `0x14028D800` + `0x14039EAD0`.
    fn start_sonic_blade(&mut self, h: &mut dyn Host, target: Option<TargetPart>, pos: Vec3, dir: &mut Vec3, w: &mut f32) {
        let refuse = |me: &mut Kh1Sora, why| me.special_log.push(SpecialEvent::Refused { command: cmd::SONIC_BLADE, why });
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
        if !h.start_special(&SpecialStart { index: SONIC_BLADE, cost: COST[SONIC_BLADE] }) {
            return refuse(self, "no Re:CoM rows or MP");
        }
        self.set_state(STATE);
        self.special = Some(Special { index: SONIC_BLADE, phase: Phase::WindUp, counter: 0, attack_bit: false, age: 0.0 });
        self.play(xl::request(xl::WINDUP) as u32 | req::ACTION, BLEND);
        // The end callback entry (`0x14029FA30(actor, 0x40000000, 0)` with `0x14039ED60`).
        self.append(req::RELEASE | request::IDLE as u32, 0.0);
        self.special_log.push(SpecialEvent::Started { index: SONIC_BLADE });
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
    /// the queued idle (`0x14039EC90`). True when a new row took the motion (the update stops).
    pub(crate) fn special_motion_end(&mut self, h: &mut dyn Host) -> bool {
        let Some(s) = self.special else { return false };
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
    pub(crate) fn special_fall(&mut self, h: &mut dyn Host) {
        if self.special.is_none() {
            return;
        }
        self.end_special(h, "airborne");
        self.play(request::FALL as u32, BLEND);
    }

    /// `0x14039EC10`: the special ends (state 0 unless the caller sets another), its flags go, and
    /// Re:CoM's sleight action ends (`0x1401F6660`) unless its win took over.
    pub(crate) fn end_special(&mut self, h: &mut dyn Host, why: &'static str) {
        let Some(s) = self.special.take() else { return };
        self.special_log.push(SpecialEvent::Ended { why, frame: s.age });
        if self.state == STATE {
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

/// The Field exe (`0x1401FA0F0`) has no case for action `0x16` (its jump table `0x1401FA5F4`,
/// index action − 1, sends it to the default `0x1401FA599`: Re:CoM plays sleights only in battle).
/// KH1's specials run in the field, so the entry is pointed at a stub in an int3 cave that runs
/// `exeStockSkill` on the player (`rbx` in the switch) and goes on at the default, as magic's and
/// the items' cases do. The Battle exe calls it through Sora's vtable `+0x140`
/// (`0x1401F9B5E`). Checked (exe: the entry, the cave's 14 int3 bytes after `0x1401FB7B1`'s
/// `ret`, nothing branching into them; disasm of both switches).
pub mod field_case {
    pub const TABLE_ENTRY_RVA: u32 = 0x001F_A648;
    pub const CAVE_RVA: u32 = 0x001F_B7B2;
    pub const EXE_STOCK_RVA: u32 = 0x0021_8950;
    pub const DEFAULT_RVA: u32 = 0x001F_A599;
    pub const STUB_LEN: usize = 13;

    /// `mov rcx,rbx; call exeStockSkill; jmp default`.
    pub fn stub() -> [u8; STUB_LEN] {
        let mut b = [0u8; STUB_LEN];
        b[..3].copy_from_slice(&[0x48, 0x8B, 0xCB]);
        b[3] = 0xE8;
        let call = EXE_STOCK_RVA as i64 - (CAVE_RVA as i64 + 8);
        b[4..8].copy_from_slice(&(call as i32).to_le_bytes());
        b[8] = 0xE9;
        let jmp = DEFAULT_RVA as i64 - (CAVE_RVA as i64 + 13);
        b[9..13].copy_from_slice(&(jmp as i32).to_le_bytes());
        b
    }

    pub fn entry() -> [u8; 4] {
        CAVE_RVA.to_le_bytes()
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
