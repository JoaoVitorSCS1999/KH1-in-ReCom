//! "Sora learned X" notices at Combat = KH1 (M12 step 2; owner, 2026-11-03: "I want there to be a
//! notification that lets the player know that they've obtained those abilities"; notices on
//! "Re:CoM's sleight-learned screen (`CLevelUp` sleight mode, its "ability" wording), without its
//! card-stock help page at KH1").
//!
//! A notice is shown by **Re:CoM's own screen** (`crate::level_up`): its sleight-learned mode
//! (`+0x258` = a sleight id, states 0xE..0x11). An ability opens it with
//! [`crate::level_up::ABILITY_WORDING_ID`] (Re:CoM's ability wording) and our name in `+0x250`; a
//! sleight (later steps) with its own id (Re:CoM's sleight wording and name).
//!
//! When (`win::notices`): right after Re:CoM's level-up screen (or its own sleight-learned screen)
//! ends, on the same `CLevelUp` task (state 0xD → 1 in sleight mode), so the game waits for it as
//! for Re:CoM's; otherwise at the next moment Re:CoM itself would let the camp open in the field
//! (its gate `0x1401A96C0`, without the Start press), as Re:CoM's event `0xAD 7` creates the
//! screen (`0x14010E9C0(new 0x2C0, 0x1402AD300(1), id)`).

use std::collections::VecDeque;

/// What Sora learned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Learned {
    /// One of `crate::abilities::ALL` (by `crate::abilities::ix`).
    Ability(usize),
    /// A Re:CoM sleight id (its own name and wording).
    Sleight(u16),
}

impl Learned {
    /// The sleight id Re:CoM's screen is opened with.
    pub fn screen_id(self) -> u16 {
        match self {
            Learned::Ability(_) => crate::level_up::ABILITY_WORDING_ID,
            Learned::Sleight(id) => id,
        }
    }

    /// Our name for the screen's `%s` (NUL-terminated), or None for Re:CoM's own (a sleight).
    /// Abilities: the name the Abilities page shows (`crate::abilities::ALL[i].name`).
    pub fn name(self) -> Option<Vec<u8>> {
        match self {
            Learned::Ability(i) => crate::abilities::ALL.get(i).map(|a| {
                let mut v: Vec<u8> = a.name.bytes().filter(|&b| (0x20..0x7F).contains(&b) && b != b'%').take(NAME_MAX).collect();
                v.push(0);
                v
            }),
            Learned::Sleight(_) => None,
        }
    }
}

/// The longest name we hand Re:CoM (its line buffer `0x14087A410` holds the formatted line).
pub const NAME_MAX: usize = 40;
/// At most this many waiting (a burst beyond it is logged and dropped).
pub const QUEUE_MAX: usize = 32;

/// The waiting notices, in the order learned; a notice already waiting is not added twice.
#[derive(Clone, Debug, Default)]
pub struct Queue(VecDeque<Learned>);

impl Queue {
    pub const fn new() -> Self {
        Queue(VecDeque::new())
    }

    /// False when it was already waiting or the queue is full.
    pub fn push(&mut self, l: Learned) -> bool {
        if self.0.contains(&l) || self.0.len() >= QUEUE_MAX {
            return false;
        }
        self.0.push_back(l);
        true
    }

    pub fn pop(&mut self) -> Option<Learned> {
        self.0.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Takes `l` out if it waits (Re:CoM's own screen just showed it). True when it was waiting.
    pub fn remove(&mut self, l: Learned) -> bool {
        let n = self.0.len();
        self.0.retain(|&x| x != l);
        self.0.len() != n
    }
}

/// The abilities a sync newly unlocked that get a notice: those Sora gets at a level (KH1's level
/// abilities: Combo Plus, Air Combo Plus and the other `Source::Level` ones). The others come with
/// a screen of Re:CoM's (its level-up sleight pick, its High Jump / Glide events).
pub fn level_abilities(new: crate::abilities::Bits) -> Vec<Learned> {
    crate::abilities::ALL
        .iter()
        .enumerate()
        .filter(|(i, a)| new & (1 << i) != 0 && matches!(a.source, crate::abilities::Source::Level(_)))
        .map(|(i, _)| Learned::Ability(i))
        .collect()
}

/// M12 step 4: the sleight abilities a sync newly unlocked that get a notice (Re:CoM's own wording
/// and name, `Learned::Sleight(id)`): each one Re:CoM taught (its learned bit) **unless Re:CoM's
/// own screen just showed it** (`shown(id)`: its level-up Sleight pick, its sleight-learned screen
/// from an event or a chest), and none from a block older than layout v6 (`from`: the block's
/// version before the sync; a v0 block is a save's first sync, a v1..=v5 block learned them before
/// this build, with Re:CoM's screens).
pub fn sleight_abilities(new: crate::abilities::Bits, from: u8, shown: &mut dyn FnMut(u16) -> bool) -> Vec<Learned> {
    if from < crate::abilities::BLOCK_V6 {
        return Vec::new();
    }
    crate::abilities::SLEIGHTS
        .filter(|&i| new & (1 << i) != 0)
        .filter_map(crate::abilities::sleight_of)
        .filter(|&id| !shown(id))
        .map(Learned::Sleight)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{ix, ALL};

    #[test]
    fn abilities_use_recoms_ability_wording_and_our_name() {
        let l = Learned::Ability(ix::COMBO_PLUS[1]);
        assert_eq!(l.screen_id(), 0x5A);
        assert_eq!(l.name().unwrap(), b"Combo Plus\0");
        assert_eq!(Learned::Ability(ix::AIR_COMBO_PLUS[0]).name().unwrap(), b"Air Combo Plus\0");
        assert_eq!(Learned::Sleight(0x23).screen_id(), 0x23);
        assert_eq!(Learned::Sleight(0x23).name(), None);
        assert_eq!(Learned::Ability(99).name(), None);
        for i in 0..ALL.len() {
            let n = Learned::Ability(i).name().unwrap();
            assert!(n.len() > 1 && n.len() <= NAME_MAX + 1 && n.last() == Some(&0));
        }
    }

    #[test]
    fn queue_keeps_order_once_each() {
        let mut q = Queue::default();
        assert!(q.push(Learned::Ability(20)));
        assert!(q.push(Learned::Ability(23)));
        assert!(!q.push(Learned::Ability(20)), "already waiting");
        assert_eq!(q.len(), 2);
        assert_eq!(q.pop(), Some(Learned::Ability(20)));
        assert!(q.push(Learned::Ability(20)), "shown: may come again");
        for k in 0..QUEUE_MAX as u16 {
            q.push(Learned::Sleight(k + 100));
        }
        assert_eq!(q.len(), QUEUE_MAX);
        assert!(!q.push(Learned::Sleight(1)));
    }

    /// Owner's ask: the level abilities (Combo Plus ×3, Air Combo Plus ×2, …) get the notice; an
    /// ability from a Re:CoM sleight or Re:CoM's movement abilities don't (Re:CoM shows its own).
    #[test]
    fn only_level_abilities() {
        let new = 1 << ix::COMBO_PLUS[0] | 1 << ix::AIR_COMBO_PLUS[1] | 1 << ix::SLIDING_DASH | 1 << ix::HIGH_JUMP | 1 << ix::RIPPLE_DRIVE;
        assert_eq!(level_abilities(new), vec![Learned::Ability(ix::RIPPLE_DRIVE), Learned::Ability(ix::COMBO_PLUS[0]), Learned::Ability(ix::AIR_COMBO_PLUS[1])]);
        assert!(level_abilities(0).is_empty());
    }

    /// M12 step 4: a sleight ability Re:CoM taught gets Re:CoM's own sleight wording and name,
    /// unless Re:CoM's screen just showed it; none from before layout v6 (or a first sync).
    #[test]
    fn sleight_abilities_unless_recom_showed_them() {
        use crate::abilities::{of_sleight, sleight, BLOCK_V5, BLOCK_V6};
        let i = |id| of_sleight(id).unwrap();
        let new = 1 << i(sleight::FREEZE) | 1 << i(sleight::HOLY) | 1 << i(sleight::IDYLL_ROMP) | 1 << ix::COMBO_PLUS[0] | 1 << ix::SLIDING_DASH;
        assert_eq!(sleight_abilities(new, BLOCK_V6, &mut |_| false), vec![Learned::Sleight(0x23), Learned::Sleight(0x33), Learned::Sleight(0x3A)]);
        // Holy came with Re:CoM's level-up Sleight pick (level 47): no second notice.
        assert_eq!(sleight_abilities(new, BLOCK_V6, &mut |id| id == sleight::HOLY), vec![Learned::Sleight(0x23), Learned::Sleight(0x3A)]);
        assert!(sleight_abilities(new, BLOCK_V5, &mut |_| false).is_empty(), "learned before v6");
        assert!(sleight_abilities(new, 0, &mut |_| false).is_empty(), "a save's first sync");
        assert_eq!(Learned::Sleight(0x23).screen_id(), 0x23, "Re:CoM's own wording and name");
        // Level abilities are not sleight notices, and sleights are not level notices.
        assert_eq!(level_abilities(new), vec![Learned::Ability(ix::COMBO_PLUS[0])]);
    }

    #[test]
    fn queue_remove() {
        let mut q = Queue::default();
        q.push(Learned::Sleight(0x23));
        q.push(Learned::Ability(20));
        assert!(q.remove(Learned::Sleight(0x23)));
        assert!(!q.remove(Learned::Sleight(0x23)));
        assert_eq!(q.pop(), Some(Learned::Ability(20)));
    }
}
