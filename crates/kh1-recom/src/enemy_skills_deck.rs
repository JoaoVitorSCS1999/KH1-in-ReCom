//! Enemy Skills, plan stage 5 (`docs/recom-enemy-skills.md` § "Stage 5 (built)"): the skills whose
//! Re:CoM effect acts on the deck (reloads, card breaks, sleights, card values), given a KH1 meaning on
//! our KH1 systems at Combat = KH1 in Sora's story: KH1's MP (`crate::mp`, `win::mp`), Sora's
//! Sleights list (`crate::sleights`, `win::summons`), KH1's clashes (`crate::kh1_enemy::sweep`,
//! `win::kh1_enemy`), the damage check's no-reaction path (`win::attack::damage_check`). The rules are
//! here, tested; the glue is `win::enemy_skills_deck`. Every number not the owner's is ours (marked).
//!
//! | card | Re:CoM (research § 2.3 / 2.4) | at KH1 |
//! |---|---|---|
//! | Tornado Step (0x61) | Reload Haste: the reload counter reset to 1 | [`TORNADO_BONUS`]: +0.75 on KH1's hit-charge factor (half MP Haste's +1.5), same 250 % cap |
//! | Aquatank (0x54) | Auto-Reload: an empty deck reloads by itself | once a battle, the payment that leaves 0 MP points refills MP to full ([`after_pay`]) |
//! | Barrel Spider (0x50) | Quickload: the reload completes at once | once a battle, the payment that empties the charge gauge (below max MP) fills it at once: the next point the next frame |
//! | Air Soldier (0x4D) | Reload Kinesis: reload while moving | [`RUN_DT_PER_UNIT`]: 1 charge unit per 20 frames of running in battle |
//! | Jafar (0x69) | Attack Bracer: attack plays can't be broken | a KH1 swing takes no reaction (Re:CoM's no-reaction bit for the check, the roll's path) |
//! | Air Pirate (0x58) | Item Bracer: item plays can't be broken | a KH1 item use takes no reaction (the same) |
//! | Riku (0x6E) | Sleight Lock | a sleight costs 1 MP point less, at least 1 ([`discount`]); Party sleights too |
//! | Marluxia (0x72) | Double Sleight: `player+0x428` bit 0x2000 replays the sleight | a Sleights-list sleight plays twice: Re:CoM's own sleight start again, free, when the first ends |
//! | Luxord (0x7B) | Omni Break: plays never broken | Sora's KH1 swing wins every clash with a KH1 enemy's attack row ([`omni_break`]) |
//! | Black Fungus (0x5F) | Random Flush: the slot becomes a random enemy card | at each KH1 battle start one random built skill not equipped counts as equipped for that battle ([`random_skill`]) |

use crate::abilities::{self as ab, Bits};
use crate::kh1_enemy::{EnemyGets, Met};
use std::sync::atomic::{AtomicBool, Ordering};

pub mod card {
    pub const AIR_SOLDIER: u16 = 0x4D;
    pub const BARREL_SPIDER: u16 = 0x50;
    pub const AQUATANK: u16 = 0x54;
    pub const AIR_PIRATE: u16 = 0x58;
    pub const BLACK_FUNGUS: u16 = 0x5F;
    pub const TORNADO_STEP: u16 = 0x61;
    pub const JAFAR: u16 = 0x69;
    pub const RIKU: u16 = 0x6E;
    pub const MARLUXIA: u16 = 0x72;
    pub const LUXORD: u16 = 0x7B;
    /// Every stage-5 card.
    pub const ALL: [u16; 10] = [AIR_SOLDIER, BARREL_SPIDER, AQUATANK, AIR_PIRATE, BLACK_FUNGUS, TORNADO_STEP, JAFAR, RIKU, MARLUXIA, LUXORD];
}

/// The ability bit of an Enemy Skill's card (0 when the card has none).
pub fn bit_of(card: u16) -> Bits {
    crate::enemy_skills::of_card(card).map_or(0, |s| 1 << crate::enemy_skills::ability_of(s))
}

/// `equipped` holds the skill of `card`.
pub fn has(equipped: Bits, card: u16) -> bool {
    equipped & bit_of(card) != 0
}

// ---- Tornado Step: MP Haste's model ----

/// Ours (brief: "modelled on KH1's MP Haste, pick a factor"): +75 % on KH1's charge factor per hit
/// (MP Haste's G`+0x95` is +150 %, `0x1402BF330`). Under KH1's 250 % cap: at equal levels a swing
/// (3) charges 5 (MP Haste 7), a finisher (6) 10 (MP Haste 15); with both, the cap.
pub const TORNADO_BONUS: u8 = 75;

/// KH1's charge for a hit (`0x1402BF330`, kh1-core's `mp_charge`) with MP Haste and Tornado Step:
/// the bonuses add on the factor before the truncation, the floor 1 and the cap. None when the
/// enemy's level is not above 0 (KH1 charges nothing).
pub fn hit_units(record_charge: i32, enemy_level: i32, sora_level: u32, mp_haste: bool, tornado: bool) -> Option<i32> {
    use kh1_core::damage::{mp_charge, AttackerStats, MpChargeParams};
    if enemy_level <= 0 {
        return None;
    }
    let base = MpChargeParams::default();
    let bonus = if mp_haste { base.bonus as u32 } else { 0 } + if tornado { TORNADO_BONUS as u32 } else { 0 };
    let p = MpChargeParams { bonus: bonus.min(u8::MAX as u32) as u8, ..base };
    let attacker = AttackerStats { level: sora_level as i32, mp_charge_bonus: bonus > 0, ..Default::default() };
    Some(mp_charge(&p, record_charge, enemy_level, &attacker))
}

// ---- Aquatank / Barrel Spider: after a payment ----

/// What a payment's result asks for (`win::mp::pay`, KH1's `0x1402A5B60`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refill {
    None,
    /// Aquatank: MP to full (KH1's `0x1402A56F0`: MP = max, the charge 0).
    Full,
    /// Barrel Spider: this many charge units (the gauge to the next point's (MP + 1) × 30).
    Charge(i32),
}

/// After a payment left `mp` points of `max` and `charge` units: Aquatank (available) when the
/// points ran out; else Barrel Spider (available) when the charge gauge is empty below max MP.
pub fn after_pay(mp: i32, max: i32, charge: u16, aquatank: bool, barrel: bool) -> Refill {
    if aquatank && mp <= 0 && max > 0 {
        Refill::Full
    } else if barrel && charge == 0 && mp < max {
        Refill::Charge((mp + 1) * crate::mp::UNITS_PER_MP)
    } else {
        Refill::None
    }
}

// ---- Air Soldier ----

/// Ours (brief: "MP charges slowly while Sora runs, pick a rate"): 1 charge unit per 1000 Re:CoM
/// time units of running (20 frames at 60 Hz; 50 a frame, `player+0x460`): 3 units a second, half
/// what steady swinging gives at equal levels (a swing charges 3); the first point after 10 s from 0.
pub const RUN_DT_PER_UNIT: i32 = 1000;

/// Air Soldier's step: `acc` Re:CoM time units carried, `dt` this frame; the units made.
pub fn run_units(acc: &mut i32, dt: i32) -> i32 {
    *acc += dt.max(0);
    let n = *acc / RUN_DT_PER_UNIT;
    *acc -= n * RUN_DT_PER_UNIT;
    n
}

// ---- Riku ----

/// Riku's Sleight Lock is armed for this battle (`win::enemy_skills_deck::battle_start`).
static SLEIGHT_LOCK: AtomicBool = AtomicBool::new(false);

pub fn set_sleight_lock(on: bool) {
    SLEIGHT_LOCK.store(on, Ordering::Relaxed);
}

pub fn sleight_lock() -> bool {
    SLEIGHT_LOCK.load(Ordering::Relaxed)
}

/// A sleight's whole MP points with Riku: 1 less, at least 1 (brief).
pub fn discount(points: i32, on: bool) -> i32 {
    if on {
        (points - 1).max(1)
    } else {
        points
    }
}

// ---- Luxord ----

/// Luxord's Omni Break: Sora's KH1 swing wins every meeting with a KH1 enemy's attack row (brief:
/// "always win clashes"). The enemy gets KH1's clash record on the row it attacked with (kind 7
/// when its row is more than 40 under Sora's, KH1's `0x1402B4BC0` rule, else 6) and its row is
/// marked for the frame (it meets no hurt body); Sora gets no record and isn't marked (his swing
/// goes on into the enemy's body that frame). KH1's own rule would leave a row at 98+ unmoved and
/// recoil Sora against a higher one (`kh1_core::enemy::guard::met`): ours.
pub fn omni_break(m: Met, attack: u8, enemy_priority: i32, sora_priority: i32) -> Met {
    use kh1_core::enemy::guard::STRONG;
    let diff = enemy_priority - sora_priority;
    let kind = if diff < -STRONG { 7 } else { 6 };
    Met { gets: EnemyGets::Clash { attack, kind, diff }, stops: true, ..m }
}

// ---- Black Fungus ----

/// The skills Black Fungus may pick: every built Enemy Skill (`crate::abilities::CARD_BITS` not in
/// `UNBUILT_BITS`) not equipped, not Black Fungus itself. Re:CoM's Random Flush draws any enemy card
/// whatever the collection (`0x140163170` case 0x5F), so learned or not.
pub fn fungus_candidates(equipped: Bits) -> Vec<usize> {
    let pool = ab::CARD_BITS & !ab::UNBUILT_BITS & !equipped & !bit_of(card::BLACK_FUNGUS);
    (0..ab::ALL.len()).filter(|&i| pool & (1 << i) != 0).collect()
}

/// Black Fungus's pick for a battle from Re:CoM's random float `r` in [0, 1) (`0x14029CF00`).
pub fn random_skill(equipped: Bits, r: f32) -> Option<usize> {
    let c = fungus_candidates(equipped);
    if c.is_empty() {
        return None;
    }
    let k = ((r.clamp(0.0, 0.999_999) * c.len() as f32) as usize).min(c.len() - 1);
    Some(c[k])
}

// ---- Marluxia ----

/// Re:CoM actions after which a Double Sleight's second play doesn't come (a hit reaction, the KO,
/// Re:CoM's win): `0xC`, `0xD`, `0x1D`, `0x1C`. Checked (decomp: `0x1401E6DE0`, `0x1401F17F0`,
/// `crate::battle::end`).
pub const REPLAY_CUT: [u32; 4] = [0xC, 0xD, 0x1D, 0x1C];
/// Re:CoM's stand / walk / run (`crate::movement::action`): the second play starts from these.
pub const REPLAY_FROM: [u32; 3] = [2, 3, 4];
/// Frames a waiting second play may wait for Sora to be free (ours).
pub const REPLAY_WAIT: u8 = 30;

/// What a waiting second play does this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Replay {
    Start,
    Wait,
    Drop,
}

pub fn replay(action: u32, battle: bool, dead: bool, left: u8) -> Replay {
    if !battle || dead || REPLAY_CUT.contains(&action) || left == 0 {
        Replay::Drop
    } else if REPLAY_FROM.contains(&action) {
        Replay::Start
    } else {
        Replay::Wait
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_are_skills() {
        for c in card::ALL {
            assert!(crate::enemy_skills::of_card(c).is_some(), "{c:#x}");
            assert_ne!(bit_of(c), 0);
        }
    }

    /// KH1's base and MP Haste unchanged; Tornado Step between; both under the cap.
    #[test]
    fn tornado_on_kh1s_charge() {
        for (rc, lv, sl) in [(3, 10, 10), (6, 10, 10), (24, 10, 10), (3, 20, 10), (3, 5, 30)] {
            assert_eq!(hit_units(rc, lv, sl, false, false), crate::mp::hit_units(rc, lv, sl), "base");
            assert_eq!(hit_units(rc, lv, sl, true, false), crate::mp::hit_units_with(rc, lv, sl, true), "MP Haste");
        }
        assert_eq!(hit_units(3, 10, 10, false, true), Some(5));
        assert_eq!(hit_units(6, 10, 10, false, true), Some(10));
        assert_eq!(hit_units(3, 10, 10, true, true), Some(7), "the cap 250 %");
        assert_eq!(hit_units(6, 10, 10, true, true), Some(15));
        assert_eq!(hit_units(3, 0, 10, true, true), None, "level 0: nothing (KH1)");
        // Low enemy level: the floor of 1 stays.
        assert_eq!(hit_units(3, 1, 99, false, true), Some(1));
    }

    #[test]
    fn refills() {
        assert_eq!(after_pay(0, 5, 10, true, true), Refill::Full, "Aquatank first");
        assert_eq!(after_pay(0, 5, 0, false, true), Refill::Charge(30));
        assert_eq!(after_pay(2, 5, 0, false, true), Refill::Charge(90));
        assert_eq!(after_pay(2, 5, 4, true, true), Refill::None);
        assert_eq!(after_pay(5, 5, 0, true, true), Refill::None, "full MP");
        assert_eq!(after_pay(0, 5, 0, false, false), Refill::None);
        // Barrel's charge makes exactly one point through KH1's refill step.
        let mut m = crate::mp::Mp::new(2, 5, 0);
        if let Refill::Charge(n) = after_pay(2, 5, 0, false, true) {
            m.gain(n);
        }
        assert!(m.refill_step());
        assert_eq!((m.mp, m.charge), (3, 0));
    }

    #[test]
    fn running_charge() {
        let mut acc = 0;
        let made: i32 = (0..60).map(|_| run_units(&mut acc, 50)).sum();
        assert_eq!(made, 3, "3 units a second at 60 Hz");
        let mut acc = 0;
        assert_eq!((0..600).map(|_| run_units(&mut acc, 50)).sum::<i32>(), 30, "10 s: the first point from 0 MP");
        assert_eq!(run_units(&mut acc, -5), 0);
    }

    #[test]
    fn sleight_lock_discount() {
        assert_eq!(discount(3, true), 2);
        assert_eq!(discount(1, true), 1, "at least 1");
        assert_eq!(discount(2, false), 2);
        // Through `crate::sleights::pay_points_with`.
        use crate::sleights::pay_points_with as pay;
        let holy = crate::abilities::sleight::HOLY;
        assert_eq!(pay(holy, 2, false), None);
        assert_eq!(pay(holy, 2, true), Some(2), "Holy 3 → 2");
        assert_eq!(pay(crate::abilities::sleight::IDYLL_ROMP, 4, true), Some(3), "all MP − 1");
        assert_eq!(pay(crate::abilities::sleight::IDYLL_ROMP, 1, true), Some(1));
        assert_eq!(pay(0x45, 1, true), Some(1), "Party Magic LV3: 2 → 1");
        assert_eq!(pay(0x45, 1, false), None);
        assert_eq!(pay(0x56, 1, true), Some(1), "Lucky Bounty 1 stays 1");
    }

    #[test]
    fn luxord_wins() {
        let m = Met { enemy: 1, coll: 2, gets: EnemyGets::Nothing, stops: false };
        assert_eq!(omni_break(m, 3, 100, 30), Met { enemy: 1, coll: 2, gets: EnemyGets::Clash { attack: 3, kind: 6, diff: 70 }, stops: true });
        let r = omni_break(Met { gets: EnemyGets::Repelled { attack: 1 }, ..m }, 1, 10, 60);
        assert_eq!(r.gets, EnemyGets::Clash { attack: 1, kind: 7, diff: -50 });
    }

    #[test]
    fn black_fungus_picks() {
        let c = fungus_candidates(0);
        assert!(!c.is_empty());
        assert!(c.iter().all(|&i| ab::built(i) && ab::CARD_BITS & (1 << i) != 0));
        assert!(!c.contains(&crate::enemy_skills::ability_of(crate::enemy_skills::of_card(card::BLACK_FUNGUS).unwrap())));
        let first = c[0];
        assert_eq!(random_skill(0, 0.0), Some(first));
        assert_eq!(random_skill(0, 0.999_999_9), Some(*c.last().unwrap()));
        assert!(!fungus_candidates(1 << first).contains(&first), "equipped ones aren't picked");
        let all = c.iter().fold(0 as Bits, |b, &i| b | 1 << i);
        assert_eq!(random_skill(all, 0.5), None);
    }

    #[test]
    fn marluxia_replay() {
        assert_eq!(replay(2, true, false, 30), Replay::Start);
        assert_eq!(replay(0x16, true, false, 30), Replay::Wait);
        assert_eq!(replay(0xC, true, false, 30), Replay::Drop);
        assert_eq!(replay(2, false, false, 30), Replay::Drop, "not in battle");
        assert_eq!(replay(2, true, true, 30), Replay::Drop);
        assert_eq!(replay(7, true, false, 0), Replay::Drop, "waited too long");
    }
}
