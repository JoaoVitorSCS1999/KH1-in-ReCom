//! Enemy Skills stage 5: the deck skills' KH1 meanings on our KH1 systems (rules in
//! `crate::enemy_skills_deck`, `docs/recom-enemy-skills.md` § "Stage 5 (built)").
//!
//! - **Battle start** ([`battle_start`], the battle manager's ctor wrapper, before stage 4's arming):
//!   Black Fungus's skill for the battle (Re:CoM's random float `0x14029CF00`), then what is armed
//!   from the equipped skills and that one: Aquatank's and Barrel Spider's one use, Riku's discount
//!   (`crate::enemy_skills_deck::set_sleight_lock`), the rest as flags. Combat = KH1 in Sora's story;
//!   otherwise nothing is armed.
//! - **Black Fungus's skill** counts as equipped for everything that reads the equipped skills
//!   (`super::abilities::equipped_live` adds [`live_bonus`] while the main player is in battle).
//! - **Per frame** ([`frame`], from the request-bits wrapper): one atomic load unless Air Soldier is
//!   armed or a Marluxia second play waits.
//! - **Hooks into our own code only**: `super::mp::pay` ([`after_pay`]), `super::mp::on_kh1_hit`
//!   ([`tornado`]), `super::attack::damage_check` ([`no_flinch`]), `super::kh1_enemy::swing_meets_attacks`
//!   ([`luxord`]), `super::summons::sleight_running` ([`sleight_ended`]).

use super::movement::{p, rd};
use super::{base, log, va};
use crate::abilities::Bits;
use crate::enemy_skills_deck::{self as ed, card, Refill, Replay};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::Mutex;

/// Black Fungus's skill this battle: its ability index + 1 (0: none).
static BONUS: AtomicU32 = AtomicU32::new(0);
/// The armed stage-5 cards (bit = index in `card::ALL`).
static ARMED: AtomicU32 = AtomicU32::new(0);
/// Aquatank's and Barrel Spider's one use this battle is left.
static AQUATANK_LEFT: AtomicBool = AtomicBool::new(false);
static BARREL_LEFT: AtomicBool = AtomicBool::new(false);
/// Air Soldier's running time carried (Re:CoM time units).
static RUN_ACC: AtomicI32 = AtomicI32::new(0);
/// [`frame`] has work: Air Soldier armed or a second play waiting.
static FRAME_ON: AtomicBool = AtomicBool::new(false);
/// Marluxia: the waiting second play (player, sleight, frames left), and the play running now is a
/// second play (its end asks for none).
static REPLAY: Mutex<Option<(usize, u16, u8)>> = Mutex::new(None);
static REPLAYING: AtomicBool = AtomicBool::new(false);
static LINES: AtomicU32 = AtomicU32::new(60);

fn note(line: impl FnOnce() -> String) {
    if LINES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("enemy skills: {}", line()));
    }
}

fn armed(c: u16) -> bool {
    card::ALL.iter().position(|&x| x == c).is_some_and(|i| ARMED.load(Ordering::Relaxed) & (1 << i) != 0)
}

/// The main player, in a battle room.
unsafe fn main_in_battle() -> Option<usize> {
    let pl: usize = rd(va(base(), crate::sites::data::MAIN_PLAYER));
    (pl != 0 && rd::<i32>(pl + p::BATTLE) != 0).then_some(pl)
}

fn name(i: usize) -> &'static str {
    crate::abilities::ALL.get(i).map_or("?", |a| a.name)
}

/// The battle manager ctor (once per battle). `kh1`: Combat = KH1 in Sora's story.
pub fn battle_start(kh1: bool) {
    BONUS.store(0, Ordering::Relaxed);
    let e = if kh1 { super::abilities::equipped_live().unwrap_or(0) } else { 0 };
    let mut all = e;
    if ed::has(e, card::BLACK_FUNGUS) {
        // SAFETY: Re:CoM's random float (byte-checked with the drops' callees).
        let r = unsafe { super::drops::random() };
        match ed::random_skill(e, r) {
            Some(i) => {
                BONUS.store(i as u32 + 1, Ordering::Relaxed);
                all |= 1 << i;
                note(|| format!("Black Fungus (Random Flush): {} counts as equipped this battle (random {r:.3})", name(i)));
            }
            None => note(|| "Black Fungus (Random Flush): every built skill is equipped already; nothing added".into()),
        }
    }
    let mut bits = 0u32;
    for (i, &c) in card::ALL.iter().enumerate() {
        if ed::has(all, c) {
            bits |= 1 << i;
        }
    }
    ARMED.store(bits, Ordering::Relaxed);
    AQUATANK_LEFT.store(ed::has(all, card::AQUATANK), Ordering::Relaxed);
    BARREL_LEFT.store(ed::has(all, card::BARREL_SPIDER), Ordering::Relaxed);
    ed::set_sleight_lock(ed::has(all, card::RIKU));
    RUN_ACC.store(0, Ordering::Relaxed);
    REPLAYING.store(false, Ordering::Relaxed);
    if let Ok(mut g) = REPLAY.lock() {
        *g = None;
    }
    FRAME_ON.store(ed::has(all, card::AIR_SOLDIER), Ordering::Relaxed);
    if bits != 0 {
        let names: Vec<&str> = card::ALL.iter().enumerate().filter(|(i, _)| bits & (1 << i) != 0).filter_map(|(_, &c)| crate::enemy_skills::of_card(c).map(|s| s.effect)).collect();
        note(|| format!("battle start: armed {}", names.join(", ")));
    }
}

/// Black Fungus's skill of this battle (for stage 4's arming, before the battle's player exists).
pub fn bonus_bits() -> Bits {
    match BONUS.load(Ordering::Relaxed) {
        0 => 0,
        n => 1 << (n - 1),
    }
}

/// Black Fungus's skill while the main player is in a battle room (`super::abilities::equipped_live`).
/// One atomic load without it.
pub fn live_bonus() -> Bits {
    if BONUS.load(Ordering::Relaxed) == 0 {
        return 0;
    }
    // SAFETY: a static pointer of the image and the player's battle field.
    if unsafe { main_in_battle() }.is_some() {
        bonus_bits()
    } else {
        0
    }
}

/// Tornado Step armed (`super::mp::on_kh1_hit`: KH1's charge factor + 0.75).
pub fn tornado() -> bool {
    armed(card::TORNADO_STEP)
}

/// Luxord armed (`super::kh1_enemy::swing_meets_attacks`: Sora's swing wins every clash).
pub fn luxord() -> bool {
    armed(card::LUXORD)
}

/// Jafar (a KH1 swing) or Air Pirate (a KH1 item use): this hit on Sora takes no reaction
/// (`super::attack::damage_check`'s no-reaction path, the roll's). Only when a hit arrived.
pub unsafe fn no_flinch(pl: usize) -> bool {
    let a = ARMED.load(Ordering::Relaxed);
    if a == 0 {
        return false;
    }
    (armed(card::JAFAR) && super::attack::kh1_swing(pl)) || (armed(card::AIR_PIRATE) && rd::<u32>(pl + p::ACTION) == crate::items::RECOM_ITEM_ACTION)
}

/// After a KH1 payment (`super::mp::pay`) left (MP, max MP, charge) `a`, in battle: Aquatank's full
/// refill or Barrel Spider's gauge, once a battle each.
pub fn after_pay(a: (i32, i32, u16)) {
    let (aq, br) = (AQUATANK_LEFT.load(Ordering::Relaxed), BARREL_LEFT.load(Ordering::Relaxed));
    // SAFETY: a static pointer of the image and the player's battle field.
    if !(aq || br) || unsafe { main_in_battle() }.is_none() {
        return;
    }
    match ed::after_pay(a.0, a.1, a.2, aq, br) {
        Refill::None => {}
        Refill::Full => {
            AQUATANK_LEFT.store(false, Ordering::Relaxed);
            super::mp::restore_full("Aquatank (Auto-Reload): MP ran out, refilled once this battle");
            note(|| format!("Aquatank: MP {}/{} ran out: refilled to full (once this battle)", a.0, a.1));
        }
        Refill::Charge(n) => {
            BARREL_LEFT.store(false, Ordering::Relaxed);
            super::mp::gain_charge(n, "Barrel Spider (Quickload): the empty gauge filled at once");
            note(|| format!("Barrel Spider: the charge gauge emptied at MP {}/{}: filled at once (+{n}, once this battle)", a.0, a.1));
        }
    }
}

/// A Sleights-list sleight ended (`super::summons::sleight_running`): Marluxia's second play waits,
/// unless this was the second play (or a Party sleight).
pub fn sleight_ended(pl: usize, id: u16) {
    if REPLAYING.swap(false, Ordering::Relaxed) || !armed(card::MARLUXIA) || crate::party::index(id).is_some() {
        return;
    }
    if let Ok(mut g) = REPLAY.lock() {
        *g = Some((pl, id, ed::REPLAY_WAIT));
        FRAME_ON.store(true, Ordering::Relaxed);
    }
}

/// Once per frame of Sora's at Combat = KH1 (the request-bits wrapper): Air Soldier's running
/// charge, Marluxia's second play. One atomic load while neither is on.
pub unsafe fn frame(pl: usize) {
    if !FRAME_ON.load(Ordering::Relaxed) {
        return;
    }
    let battle = rd::<i32>(pl + p::BATTLE) != 0;
    let action: u32 = rd(pl + p::ACTION);
    if armed(card::AIR_SOLDIER) && battle && action == crate::movement::action::RUN {
        let mut acc = RUN_ACC.load(Ordering::Relaxed);
        let n = ed::run_units(&mut acc, rd::<i32>(pl + p::DT));
        RUN_ACC.store(acc, Ordering::Relaxed);
        if n > 0 && super::mp::current().is_some_and(|(mp, max, _)| mp < max) {
            super::mp::gain_charge(n, "Air Soldier (Reload Kinesis): running");
        }
    }
    let pending = REPLAY.lock().ok().and_then(|g| *g);
    let Some((who, id, left)) = pending else {
        if !armed(card::AIR_SOLDIER) {
            FRAME_ON.store(false, Ordering::Relaxed);
        }
        return;
    };
    let dead = rd::<i32>(pl + DEAD) != 0;
    let step = if who != pl { Replay::Drop } else { ed::replay(action, battle, dead, left) };
    let next = match step {
        Replay::Wait => Some((who, id, left - 1)),
        Replay::Drop => {
            note(|| format!("Marluxia: the second {} didn't come (action {action:#x})", crate::sleights::name(id)));
            None
        }
        Replay::Start => {
            if super::summons::replay_sleight(pl, id) {
                REPLAYING.store(true, Ordering::Relaxed);
                note(|| format!("Marluxia (Double Sleight): {} plays again (free; Re:CoM's sleight start)", crate::sleights::name(id)));
            }
            None
        }
    };
    if let Ok(mut g) = REPLAY.lock() {
        *g = next;
    }
    if next.is_none() && !armed(card::AIR_SOLDIER) {
        FRAME_ON.store(false, Ordering::Relaxed);
    }
}

/// Re:CoM's "Sora is dead" (`player+0x7B0`, set by the damage check's death path). Checked (decomp).
const DEAD: usize = 0x7B0;
