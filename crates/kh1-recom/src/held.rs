//! M11, the Neoshadow's grab from below (owner, 2026-11-02: "the grab as a new Re:CoM player reaction
//! ("held") playing KH1's held clip as a new Sora motion slot, freed by a hit"): the pure rules of Sora's new
//! held reaction. `docs/recom-kh1-attack.md` § The held reaction, `docs/recom-m11-enemies.md` § The Neoshadow.
//!
//! KH1 (`ex_2420.bd` `0x1AF6`; Checked (files + decomp)): when its grab test passes (`0x3FAA`), the Neoshadow puts
//! Sora into request 200 of its own `.mset` player set (`sys 113` `0x1402BA8C0`, `sys 13` with `0x10000C8`: bit 24
//! the action lock, `0x14029F980` sets actor flag 4; blend 4; the lock's release `0x40000000` queued): 126 frames,
//! no damage, no button escape; a damaging hit's reaction replaces the motion and so frees him.
//!
//! Re:CoM (Checked (decomp + exe)): an enemy reaches Sora only through a hit; its attack record's `+0x68` indexes
//! the kind table `0x14052A220` (21 kinds; the getter `0x140179410`, its only reader), the kind lands in Sora's
//! `+0x710`, and the damage check `0x1401E6DE0` picks the reaction from it. The held reaction is built on that:
//!
//! - a new kind, [`HELD_KIND`], at a new index of the kind table ([`HELD_INDEX`], the table copied and extended:
//!   [`kind_table`]); the Neoshadow's grab record carries the index (the asset step);
//! - the damage check's new case ([`on_hit`]): the kind does no damage (as Re:CoM's Stop kind 8 takes no HP), Re:CoM's
//!   own check runs for it with its no-reaction exit (its gates: dead, protected `+0x418 & 1`, the hit-immunity
//!   timer `+0x7BC`, `+0x7B4`, Sora's guard), and when it took the hit ([`taken`]) Sora enters the held reaction;
//! - the held reaction is Re:CoM's status action `0x18` in its sub-state `0xC` ([`STATUS_ACTION`], [`HELD_SUB`]),
//!   with the held clip's row: the status exe `0x1401EB0F0` ends sub-state `0xC` itself when its clip has played
//!   (`0x1401A57D0`, then `0x1401F3180(pl, 0)`: Re:CoM's own return to standing), takes no button, and Re:CoM's
//!   damage check runs every frame of it: a damaging hit puts him in Re:CoM's hit reaction (action `0xC`), and
//!   nothing brings the hold back (the reaction's end goes back to the status only for `+0x420 & 0x8810023`, none
//!   of which the held reaction sets). No code of ours runs during the hold.

/// Re:CoM's kind table (`0x14052A220`, 21 entries: the `.ASD` record's `+0x68` → Sora's `+0x710`). Checked (exe).
pub const RECOM_KINDS: [u32; 21] = [1, 2, 3, 36, 37, 4, 5, 26, 7, 38, 6, 10, 39, 40, 8, 41, 28, 29, 30, 42, 32];

/// The held reaction's kind in Sora's `+0x710`: above every kind Re:CoM's damage check switches on (at most
/// `0x28`), the kind table's (at most 42) and the kinds its enemies pass to `0x14017F0E0` themselves. Its only
/// reader is the damage check (Checked (decomp): `+0x710` is read nowhere else on the player).
pub const HELD_KIND: u32 = 0x40;

/// The held kind's index in the extended kind table: the first after Re:CoM's 21.
pub const HELD_INDEX: u32 = RECOM_KINDS.len() as u32;

/// Re:CoM's status action (`0x1401F1200` init, `0x1401EB0F0` exe).
pub const STATUS_ACTION: u32 = 0x18;

/// The status action's sub-state that plays a clip to its end and then stands Sora up (`0x1401EB0F0` case
/// `0xC`; Re:CoM's own use: the get-up after stun, row `0x28`).
pub const HELD_SUB: u32 = 0xC;

/// The extended kind table: Re:CoM's 21 kinds (as read from the running exe, which must be Re:CoM's) and
/// [`HELD_KIND`] at [`HELD_INDEX`].
pub fn kind_table(recom: &[u32]) -> Option<Vec<u32>> {
    if recom != RECOM_KINDS {
        return None;
    }
    let mut t = recom.to_vec();
    t.push(HELD_KIND);
    Some(t)
}

/// What Re:CoM's damage check and our wrapper see of the main player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SoraNow {
    /// Combat = KH1 and the main player is Sora (our KH1 Sora).
    pub kh1: bool,
    /// The held reaction is ready: the held clip's row in the action table and the kind table extended.
    pub row: Option<u32>,
    /// `+0x7B0`: dead.
    pub dead: bool,
    /// `+0x418 & 1`: Re:CoM's protection (its roll, Sonic Blade, our summons).
    pub protected: bool,
    /// `+0x7BC > 0`: Re:CoM's hit-immunity timer (KH1's 30 post-hit frames at the KH1 setting).
    pub immune: bool,
    /// `+0x7B4 != 0`.
    pub busy_7b4: bool,
    /// KH1's no-flinch frames (the KH1 roll's).
    pub no_flinch: bool,
    /// Sora's KH1 Guard is up.
    pub guarding: bool,
    /// His mover touches the floor (`+0x120 & 5`).
    pub grounded: bool,
    /// `+0x40C`, `+0x414`, `+0x474`: the action, its sub-state, the action-table row playing.
    pub action: u32,
    pub sub: u32,
    pub row_now: i32,
}

/// Sora is in the held reaction: the status action's clip sub-state playing the held clip's row.
pub fn is_held(s: &SoraNow) -> bool {
    s.row.is_some_and(|r| s.action == STATUS_ACTION && s.sub == HELD_SUB && s.row_now == r as i32)
}

/// KH1's grab test (`0x3FAA`) on Re:CoM's Sora: he can be hit (`sys 141` `0x140291920`: not in KH1's post-hit /
/// no-flinch frames, flag `0x80`; here Re:CoM's damage check would take a hit: not dead, not protected, no
/// hit-immunity, not the no-flinch frames), he stands on the floor (`sys 218`, `sys 89`, `+0x70` = 0), and the
/// held reaction is ready. Not while his KH1 Guard is up (Assumption: KH1's flag `0x400` test there is not
/// identified; Re:CoM's guard would block the grab's hit), nor in Re:CoM's status action (already held, or one of
/// Re:CoM's own statuses: stun, Vexen's freeze, Stop, which KH1 doesn't have; Assumption: the grab misses then).
pub fn grabbable(s: &SoraNow) -> bool {
    s.kh1 && s.row.is_some() && !s.dead && !s.protected && !s.immune && !s.busy_7b4 && !s.no_flinch && !s.guarding && s.grounded && s.action != STATUS_ACTION
}

/// The damage check's case for a pending hit of `kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnHit {
    /// Not the held kind: Re:CoM's check as ever.
    Recom,
    /// The held kind, and Sora can be held: Re:CoM's check runs with the value 0 and its no-reaction exit;
    /// when it took the hit ([`taken`]) he enters the held reaction with this row.
    Hold { row: u32 },
    /// The held kind, but no hold (Re:CoM's Sora, no held clip, already held or in another of Re:CoM's statuses,
    /// the no-flinch frames): Re:CoM's check runs with the value 0 and its no-reaction exit (no damage, no
    /// reaction).
    Nothing,
}

/// The held kind's case (KH1: the grab does no damage; a second grab of a held Sora changes nothing).
pub fn on_hit(kind: u32, s: &SoraNow) -> OnHit {
    if kind != HELD_KIND {
        return OnHit::Recom;
    }
    match s.row {
        Some(row) if s.kh1 && !s.no_flinch && s.action != STATUS_ACTION => OnHit::Hold { row },
        _ => OnHit::Nothing,
    }
}

/// Re:CoM's check took the hit: it clears the pending word `+0x720` only once past its gates (dead, protected,
/// immune, `+0x7B4`, guard: each returns with the hit still pending; Checked (decomp `0x1401E6DE0`)), and Sora
/// is alive after it.
pub fn taken(pending_before: i16, pending_after: i16, dead_after: bool) -> bool {
    pending_before != 0 && pending_after == 0 && !dead_after
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sora() -> SoraNow {
        SoraNow { kh1: true, row: Some(0x5A), grounded: true, action: 2, row_now: 0x2A, ..Default::default() }
    }

    #[test]
    fn the_kind_table_keeps_recoms_and_adds_the_held_kind() {
        let t = kind_table(&RECOM_KINDS).unwrap();
        assert_eq!(&t[..21], &RECOM_KINDS[..]);
        assert_eq!((t.len(), t[HELD_INDEX as usize]), (22, HELD_KIND));
        assert!(kind_table(&[1, 2, 3]).is_none(), "not Re:CoM's table: no patch");
        // Not one of Re:CoM's kinds, nor in its damage check's switch (≤ 0x28).
        assert!(!RECOM_KINDS.contains(&HELD_KIND) && HELD_KIND > 0x28);
    }

    #[test]
    fn the_held_kind_holds_kh1_sora_once() {
        let s = sora();
        assert_eq!(on_hit(1, &s), OnHit::Recom, "Re:CoM's kinds untouched");
        assert_eq!(on_hit(0x21, &s), OnHit::Recom);
        assert_eq!(on_hit(HELD_KIND, &s), OnHit::Hold { row: 0x5A });
        let held = SoraNow { action: STATUS_ACTION, sub: HELD_SUB, row_now: 0x5A, ..s };
        assert!(is_held(&held));
        assert_eq!(on_hit(HELD_KIND, &held), OnHit::Nothing, "already held");
        assert_eq!(on_hit(HELD_KIND, &SoraNow { kh1: false, ..s }), OnHit::Nothing, "Re:CoM's Sora");
        assert_eq!(on_hit(HELD_KIND, &SoraNow { row: None, ..s }), OnHit::Nothing, "no held clip");
        assert_eq!(on_hit(HELD_KIND, &SoraNow { no_flinch: true, ..s }), OnHit::Nothing);
        // Re:CoM's stun get-up (the same sub-state, its own row 0x28) is not the hold, and takes none.
        let getup = SoraNow { action: STATUS_ACTION, sub: HELD_SUB, row_now: 0x28, ..s };
        assert!(!is_held(&getup));
        assert_eq!(on_hit(HELD_KIND, &getup), OnHit::Nothing);
        assert_eq!(on_hit(HELD_KIND, &SoraNow { action: 0xC, ..s }), OnHit::Hold { row: 0x5A }, "a hit reaction: held (KH1's grab test already passed)");
    }

    #[test]
    fn grab_test() {
        let s = sora();
        assert!(grabbable(&s));
        for t in [
            SoraNow { kh1: false, ..s },
            SoraNow { row: None, ..s },
            SoraNow { dead: true, ..s },
            SoraNow { protected: true, ..s },
            SoraNow { immune: true, ..s },
            SoraNow { busy_7b4: true, ..s },
            SoraNow { no_flinch: true, ..s },
            SoraNow { guarding: true, ..s },
            SoraNow { grounded: false, ..s },
            SoraNow { action: STATUS_ACTION, sub: HELD_SUB, row_now: 0x5A, ..s },
            SoraNow { action: STATUS_ACTION, sub: 1, row_now: 0x27, ..s },
        ] {
            assert!(!grabbable(&t), "{t:?}");
        }
    }

    /// Re:CoM's gates leave the hit pending; past them it clears it.
    #[test]
    fn taken_only_past_recoms_gates() {
        assert!(taken(1, 0, false));
        assert!(!taken(1, 1, false), "immune / protected / guarded: still pending");
        assert!(!taken(0, 0, false), "no hit");
        assert!(!taken(1, 0, true), "dead");
    }
}
