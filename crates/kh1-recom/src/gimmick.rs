//! Re:CoM's gimmick cards at Combat = KH1 (owner, 2026-11-01: "certain bosses drop "gimmick cards"
//! in their battles. I want these to drop, and when sora picks them up, i want there to be a
//! triangle prompt that says "Stagger," and pressing triangle will activate the card"): the pure
//! rules. Re:CoM's side is `win::gimmick`; notes `docs/recom-gimmick-cards.md`.
//!
//! Re:CoM (Steam, Checked (decomp + exe) unless tagged):
//! - **The cards** are card kinds 0x3A..=0x41, type 6 (card-info row byte 0 = 0x66 in both copies
//!   of the table, `0x14059F2E0` and `0x140528C30`): Guard Armor, Trickmaster, Jafar, Oogie Boogie,
//!   Parasite Cage, Ursula, Dragon Maleficent, Darkside. Each boss's own code drops its card as a
//!   prize (`0x1401A89C0`, descriptor kind = the card) after naming itself the card's receiver
//!   (`0x140163740`: CBTL `+0x200`).
//! - **The pickup** (`0x1401F4AB0`, type 5 / 6): into Sora's battle revolver (`0x140166300`), which
//!   takes one only while CBTL's gimmick count `+0x1D0` is 0 and then adds 1 (`0x14015D5B0(1)`):
//!   one gimmick card at a time. The card is never added to the collection; it lives and ends with
//!   the battle (the revolver and CBTL are the battle's).
//! - **The play** (`0x14016CEE0`, type 6): CBTL `+0x1E4` = the card dword and `+0x208 |= 0x8000`
//!   (`0x140161D60(&card, 1)`); CBTL's message 1 reaches Sora's card handler (`0x1401F5580`,
//!   vtable `+0x48`), which keeps the kind in `+0x7F4` and sets `+0x428 |= 0x200`; the request
//!   builder (`0x1401F8150`, battle branch) makes that `+0x424 |= 0x40`; the move exe
//!   (`0x1401EEDA0`) starts Re:CoM's gimmick action `0x19` (`0x1401F25D0`), whose exe
//!   (`0x1401ECA70`) plays Sora's card motion, posts message 6 to the receiver after 50 frames
//!   (`0x140161D60(0, 0)`), waits for the boss to clear `+0x1E4`, ends the card
//!   (`0x14015E0E0(CBTL, 1)`) and returns to idle. The boss's message 6 runs its vtable `+0x3F8`:
//!   its own gimmick reaction. The used card leaves the revolver (`0x1401501B0`: count − 1).
//!   Gimmick cards can't be stocked (`0x14016DB60`), so no sleights.
//!
//! Here, with no deck at KH1 (decision 29): Re:CoM's pickup runs except its revolver add; the card
//! is held beside CBTL's own gimmick count (Re:CoM's one-at-a-time rule and the battle's life); the
//! context tab offers [`STAGGER`] while one is held; Triangle makes the play's own writes (the
//! revolver step `0x140161D60(&card, 1)`, the count − 1, Sora's card handler with message 1) and
//! Re:CoM's builder conversion (`+0x428 & 0x200` → `+0x424 |= 0x40`); Re:CoM then runs the rest.

/// The gimmick cards' type (card-info row byte 0, low nibble).
pub const TYPE: u8 = 6;
/// The first and last gimmick card kinds.
pub const FIRST: u8 = 0x3A;
pub const LAST: u8 = 0x41;

/// The bosses whose cards they are (the card-info rows' names, `0x1405063B8..`). Checked (exe).
pub const BOSSES: [(u8, &str); 8] = [
    (0x3A, "Guard Armor"),
    (0x3B, "Trickmaster"),
    (0x3C, "Jafar"),
    (0x3D, "Oogie Boogie"),
    (0x3E, "Parasite Cage"),
    (0x3F, "Ursula"),
    (0x40, "Dragon Maleficent"),
    (0x41, "Darkside"),
];

/// Our context command for a held gimmick card (not a KH1 command: KH1's command table ends below
/// 0x80, Checked (files: `btltbl.bin` `+0xA58`)).
pub const STAGGER: u8 = 0xF0;
/// Its word (ours, not KH1's: KH1's `Word.bin` has no such word, Checked (files)): a message id
/// above KH1's (`Word.bin` holds 510–523 strings). The asset step writes it in every language
/// (`kh1-recom-assets` `command_menu::OUR_WORDS`); the DLL falls back to [`STAGGER_TEXT`].
pub const STAGGER_WORD: u16 = 901;
/// The owner's word. English in every language (no translation was asked for; Assumption that
/// English is wanted everywhere).
pub const STAGGER_TEXT: &str = "Stagger";

/// CBTL (`*0x14087A818`) fields. Checked (decomp + exe: the getters `0x140160760`, `0x1401616C0`,
/// the setters `0x140161D60`, `0x140161D30`, `0x14015D5B0`).
pub mod cbtl {
    /// Sora's gimmick cards held (`0x14015D5B0` adds, `0x140160760` reads).
    pub const COUNT: usize = 0x1D0;
    /// The gimmick card being played (dword; byte 0 the kind), 0 when none.
    pub const NOW: usize = 0x1E4;
    /// The receiver's handle (the boss that dropped the card, `0x140163740`).
    pub const RECEIVER: usize = 0x200;
    /// CBTL's flags; a play is refused while `& LOCKS` (`0x14016CEE0`'s first test).
    pub const FLAGS: usize = 0x208;
    pub const LOCKS: u32 = 0xC000_0000;
}

/// Sora's fields. Checked (decomp: `0x1401F5580`, `0x1401F8150`, `0x1401EEDA0`, `0x1401F4AB0`).
/// The revolver play's event gates (`0x14016CEE0`): the system flag (1, 8) and the event-hide
/// mask (1, 0xE2) bit 2 (Sora's side). Checked (disasm `0x14016CF25..0x14016CF53`).
pub const EVENT_FLAG: (u32, u32) = (1, 8);
pub const HIDE_MASK: (u32, u32) = (1, 0xE2);
pub const HIDE_SORA: u32 = 2;

pub mod player {
    /// Card request bits; `& 4` refuses a card message (Re:CoM's win, `0x1401F5580`).
    pub const CARD: usize = 0x428;
    pub const CARD_GIMMICK: u32 = 0x200;
    pub const CARD_WON: u32 = 4;
    /// Action requests; the gimmick request `0x40` (the move exe starts action 0x19 on it).
    pub const REQUEST: usize = 0x424;
    pub const REQUEST_GIMMICK: u32 = 0x40;
    /// Refuses a card message too (`0x1401F5580`).
    pub const BUSY_7B0: usize = 0x7B0;
    /// The battle revolver (the prize add's card holder).
    pub const REVOLVER: usize = 0x3F0;
    /// The card id the handler keeps.
    pub const CARD_ID: usize = 0x7F4;
    /// Re:CoM's gimmick action.
    pub const GIMMICK_ACTION: u32 = 0x19;
}

/// The five bosses whose card drops only from Re:CoM's card-break stagger (enemy state 13, whose
/// begin is the class's vtable `+0x220`). At Combat = KH1 (owner, 2026-11-01, option (c)): a KH1
/// combo finisher that interrupts one of their attacks gives it Re:CoM's own card break: CBTL's
/// message 3 posted to it as the card-battle resolver posts it, so its own state 13 rolls the drop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreakBoss {
    /// Its gimmick card.
    pub kind: u8,
    /// The class's vtable (RVA, RTTI-checked at install) and its RTTI name.
    pub vtable: u32,
    pub rtti: &'static str,
    /// Its state-13 begin (vtable `+0x220`): the stagger and the drop roll (RVA).
    pub stagger: u32,
}

/// Checked (exe: RTTI vtables, the `+0x220` slots; decomp: each begin's drop code).
pub const BREAK_BOSSES: [BreakBoss; 5] = [
    BreakBoss { kind: 0x3A, vtable: 0x0065_F4B8, rtti: "CBossGuardarmor", stagger: 0x003A_B290 },
    BreakBoss { kind: 0x3B, vtable: 0x0066_C5F8, rtti: "CBossTrickMaster", stagger: 0x003C_B1C0 },
    BreakBoss { kind: 0x3C, vtable: 0x0066_E220, rtti: "CBossJafar", stagger: 0x003D_0D80 },
    BreakBoss { kind: 0x3E, vtable: 0x0066_9220, rtti: "CBossParasiteCage", stagger: 0x003C_2B70 },
    BreakBoss { kind: 0x40, vtable: 0x0067_4570, rtti: "CBossMDragon", stagger: 0x003E_1690 },
];

/// The state-13 begin's vtable slot (`+0xE8 + 0x18 × 13`).
pub const STAGGER_SLOT: u32 = 0x220;
/// CBTL's message 3 ("your card was broken"): the enemy's message handler (`0x140181620`, vtable
/// `+0x48`) case 3 sets its next state to state 13 (`enemy + (13 + 0x13) × 0x40` = `+0x800`).
pub const MESSAGE_BROKEN: u32 = 3;
pub const STATE_BROKEN: usize = 13;
/// An enemy's state object (`+0x68` next, `+0x60` current point at one).
pub const fn state_at(s: usize) -> usize {
    (s + 0x13) * 0x40
}
/// KH1's combo-finisher reaction kind (the attack record's `kind`, as `crate::damage::swing_column`).
pub const KIND_FINISHER: u8 = 5;

/// The card-break boss of a vtable.
pub fn break_boss(vtable_rva: u32) -> Option<&'static BreakBoss> {
    BREAK_BOSSES.iter().find(|b| b.vtable == vtable_rva)
}

/// A KH1 row is a combo finisher (its attack record's kind 5: the ground and air finishers,
/// Blitz, Ripple Drive, Hurricane Blast).
pub fn is_finisher(kh1_row: usize, m: &kh1_core::Moveset) -> bool {
    m.row(kh1_row).and_then(|rw| m.attack(rw.attack)).is_some_and(|a| a.kind == KIND_FINISHER)
}

/// What a KH1 hit on an enemy was, for the card break.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hit {
    /// One of [`BREAK_BOSSES`].
    pub boss: bool,
    /// The KH1 row is a combo finisher.
    pub finisher: bool,
    /// The enemy was in an attack (`+0x260 & 1`) without the interrupt bit before the hit.
    pub attacking: bool,
    /// After Re:CoM's hit handler its attack is over (`+0x260 & 1` cleared: `0x14017E1D0` ended
    /// it for the interrupt bit).
    pub ended: bool,
    /// Its HP after the hit is above 0 (Re:CoM's message handler ignores a dead enemy).
    pub alive: bool,
}

/// Option (c): the finisher interrupted one of the five bosses' attacks.
pub fn breaks(h: Hit) -> bool {
    h.boss && h.finisher && h.attacking && h.ended && h.alive
}

/// The drop roll's outcome, watched after the break (for the log): each frame, whether the boss
/// is still there, in state 13, and whether its card's prize was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Roll {
    Wait,
    Dropped,
    /// State 13's begin ran (2 frames in it) and made no card: the roll missed or the boss's own
    /// condition refused it.
    NoDrop,
    /// Never entered state 13 (Re:CoM's handler or the boss refused the message).
    NoStagger,
    Gone,
}

/// Frames in state 13 after which its begin has surely run; frames without state 13 given up.
pub const ROLL_SETTLE: u32 = 2;
pub const ROLL_GIVE_UP: u32 = 120;

/// One frame of the watch: `frames` since the post, `entered` the frame state 13 was first seen.
pub fn roll_step(frames: u32, entered: Option<u32>, present: bool, in_state: bool, dropped: bool) -> Roll {
    if dropped {
        Roll::Dropped
    } else if !present {
        Roll::Gone
    } else if let Some(e) = entered {
        if in_state && frames >= e + ROLL_SETTLE || !in_state {
            Roll::NoDrop
        } else {
            Roll::Wait
        }
    } else if frames > ROLL_GIVE_UP {
        Roll::NoStagger
    } else {
        Roll::Wait
    }
}

/// The boss a gimmick card belongs to.
pub fn boss(kind: u32) -> Option<&'static str> {
    BOSSES.iter().find(|b| b.0 as u32 == kind).map(|b| b.1)
}

/// A gimmick card kind with its card-info type byte (`0x1401F4AB0` reads it).
pub fn is_gimmick(kind: u32, type_byte: u8) -> bool {
    (FIRST as u32..=LAST as u32).contains(&kind) && type_byte & 0xF == TYPE
}

/// The card dword the revolver play passes to `0x140161D60`: the kind in byte 0, value 0, the
/// card-info type byte in byte 3 (`0x14016D008..0x14016D03D`).
pub fn card(kind: u8, type_byte: u8) -> u32 {
    (type_byte as u32) << 24 | kind as u32
}

/// The kind of a card dword.
pub fn kind_of(card: u32) -> u8 {
    card as u8
}

/// CBTL's 0x24-byte card-play record as Sora's card handler copies it (`+0x808..+0x82B`): the
/// card, no sleight (`+0x824` = 0), no charge bytes (`+0x828`, `+0x829`).
pub fn record(card: u32) -> [u32; 9] {
    let mut r = [0u32; 9];
    r[0] = card;
    r
}

/// The request the builder leaves at KH1 (its field branch, `win::attack::request_bits`), with
/// the battle branch's own conversion of a pending gimmick card (`0x1401F85F1`: `+0x428` bit 9 →
/// `+0x424 |= 0x40`).
pub fn request(card_bits: u32, req: u32) -> u32 {
    if card_bits & player::CARD_GIMMICK != 0 {
        req | player::REQUEST_GIMMICK
    } else {
        req
    }
}

/// A held gimmick card: the battle's CBTL and the card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    pub cbtl: usize,
    pub card: u32,
}

/// What a pickup did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickUp {
    /// Held: CBTL's count goes up by one (Re:CoM's `0x14015D5B0(1)`).
    Held(Held),
    /// Re:CoM's rule: one at a time (the count is above 0); the card is lost, as Re:CoM's revolver
    /// add drops it.
    AlreadyHolding,
    /// No CBTL: nothing is held and Re:CoM's prize add runs as it is. (Re:CoM's revolver add reads
    /// −1 from `0x140160760` then; Assumption that it never happens: the bosses drop only in
    /// battle rooms, which have a CBTL.)
    NoBattle,
}

/// Re:CoM's revolver add's rule (`0x140166300`, side 1, type 6).
pub fn pick_up(cbtl: usize, count: i32, card: u32) -> PickUp {
    if cbtl == 0 {
        return PickUp::NoBattle;
    }
    if count > 0 {
        return PickUp::AlreadyHolding;
    }
    PickUp::Held(Held { cbtl, card })
}

/// The card held now: the one picked up in this battle's CBTL, while CBTL's count says one is
/// held. A new battle (another CBTL, or the count back at 0) ends it, as Re:CoM's battle-only
/// cards end with the battle.
pub fn held(h: Option<Held>, cbtl_now: usize, count_now: i32) -> Option<u32> {
    let h = h?;
    (cbtl_now != 0 && h.cbtl == cbtl_now && count_now > 0).then_some(h.card)
}

/// What the offer and the use read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// A card is held ([`held`]).
    pub held: bool,
    /// Re:CoM's battle room (`player+0x390`).
    pub battle: bool,
    /// CBTL `+0x208 & 0xC0000000` (`0x14016CEE0` refuses).
    pub locked: bool,
    /// CBTL `+0x1E4` (a gimmick card is being played; `0x14016CEE0` refuses).
    pub playing: bool,
    /// `player+0x428 & 4` or `player+0x7B0` (Sora's card handler refuses a message).
    pub refused: bool,
    /// Re:CoM's events forbid Sora's card plays now: the system flag (1, 8), or bit 2 of the
    /// event-hide mask (1, 0xE2) (the revolver play's own tests, `0x14016CF25`, `0x14016CF48`).
    pub event: bool,
}

/// "Stagger" is offered (and Triangle plays the card) when Re:CoM's own play would take it.
pub fn offer(s: State) -> bool {
    s.held && s.battle && !s.locked && !s.playing && !s.refused && !s.event
}

/// The context entry (`command_menu::Menu::build`): an object's command first (KH1's
/// `0x140293000` callers set it), then a held gimmick card, then KH1's special offer (last in KH1:
/// `0x14028D0F0` takes only an empty entry). Returns (command, from the special offer).
pub fn context_entry(object: u8, gimmick: u8, special: u8) -> (u8, bool) {
    if object != 0 {
        (object, false)
    } else if gimmick != 0 {
        (gimmick, false)
    } else {
        (special, special != 0)
    }
}

/// A log line's card name.
pub fn describe(card: u32) -> String {
    let k = kind_of(card);
    match boss(k as u32) {
        Some(b) => format!("{b} card ({k:#x})"),
        None => format!("card {k:#x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_eight_cards() {
        assert_eq!(BOSSES.len(), (LAST - FIRST + 1) as usize);
        for (i, (k, _)) in BOSSES.iter().enumerate() {
            assert_eq!(*k, FIRST + i as u8);
            assert!(is_gimmick(*k as u32, 0x66));
        }
        assert_eq!(boss(0x3A), Some("Guard Armor"));
        assert_eq!(boss(0x41), Some("Darkside"));
        // Riku's story's type-0xE cards (0x42, 0x43) and other kinds are not ours.
        assert!(!is_gimmick(0x42, 0xE6) && !is_gimmick(0x39, 0x66) && !is_gimmick(0x3A, 0x61));
        assert_eq!(boss(0x42), None);
    }

    #[test]
    fn card_dword_and_record_as_re_com_builds_them() {
        // The revolver play: word = kind, byte 3 = the type byte (0x66).
        let c = card(0x3B, 0x66);
        assert_eq!(c, 0x6600_003B);
        assert_eq!(kind_of(c), 0x3B);
        assert_eq!(c.to_le_bytes(), [0x3B, 0, 0, 0x66]);
        // The handler reads +0x808 (kind), +0x80B >> 4 (type 6), +0x824 (no sleight), +0x828/829.
        let r = record(c);
        let bytes: Vec<u8> = r.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(bytes.len(), 0x24);
        assert_eq!((bytes[0], bytes[3] >> 4), (0x3B, TYPE));
        assert_eq!((bytes[0x1C], bytes[0x1D], bytes[0x20], bytes[0x21]), (0, 0, 0, 0));
        assert_eq!(describe(c), "Trickmaster card (0x3b)");
    }

    #[test]
    fn the_builder_conversion() {
        assert_eq!(request(0x200, 0x1), 0x41);
        assert_eq!(request(0x10, 0x4), 0x4);
        assert_eq!(request(0x200 | 0x400000, 0), 0x40);
    }

    #[test]
    fn one_at_a_time_and_the_battle_s_life() {
        let a = 0x1000usize;
        let c = card(0x3D, 0x66);
        let h = match pick_up(a, 0, c) {
            PickUp::Held(h) => h,
            p => panic!("{p:?}"),
        };
        assert_eq!(held(Some(h), a, 1), Some(c));
        // A second one while holding: lost, as Re:CoM's revolver add drops it.
        assert_eq!(pick_up(a, 1, card(0x3F, 0x66)), PickUp::AlreadyHolding);
        // Used (count back to 0), or the battle ended (another CBTL, or none): not held.
        assert_eq!(held(Some(h), a, 0), None);
        assert_eq!(held(Some(h), 0x2000, 1), None);
        assert_eq!(held(Some(h), 0, 1), None);
        assert_eq!(held(None, a, 1), None);
        assert_eq!(pick_up(0, 0, c), PickUp::NoBattle);
    }

    #[test]
    fn the_offer_follows_re_com_s_play_refusals() {
        let ok = State { held: true, battle: true, ..Default::default() };
        assert!(offer(ok));
        for s in [
            State { held: false, ..ok },
            State { battle: false, ..ok },
            State { locked: true, ..ok },
            State { playing: true, ..ok },
            State { refused: true, ..ok },
            State { event: true, ..ok },
        ] {
            assert!(!offer(s), "{s:?}");
        }
    }

    #[test]
    fn the_context_entry_priority() {
        // An object first, then Stagger, then KH1's special offer (Sonic Blade).
        assert_eq!(context_entry(0x37, STAGGER, 0x4B), (0x37, false));
        assert_eq!(context_entry(0, STAGGER, 0x4B), (STAGGER, false));
        assert_eq!(context_entry(0, 0, 0x4B), (0x4B, true));
        assert_eq!(context_entry(0, 0, 0), (0, false));
    }

    #[test]
    fn the_five_card_break_bosses() {
        let kinds: Vec<u8> = BREAK_BOSSES.iter().map(|b| b.kind).collect();
        assert_eq!(kinds, [0x3A, 0x3B, 0x3C, 0x3E, 0x40]);
        for b in BREAK_BOSSES {
            assert!(boss(b.kind as u32).is_some());
            assert_eq!(break_boss(b.vtable), Some(&b));
        }
        // Oogie, Ursula, Darkside drop without a card break: not here.
        assert!(BREAK_BOSSES.iter().all(|b| ![0x3D, 0x3F, 0x41].contains(&b.kind)));
        assert_eq!(state_at(STATE_BROKEN), 0x800);
        assert_eq!(0xE8 + 0x18 * STATE_BROKEN as u32, STAGGER_SLOT);
        // None of them is a revenge boss (Stage 106: humanoid bosses only).
        for b in BREAK_BOSSES {
            assert!(crate::revenge::boss_by_vtable(b.vtable).is_none(), "{}", b.rtti);
        }
    }

    #[test]
    fn only_an_interrupting_finisher_on_a_live_boss_breaks() {
        let ok = Hit { boss: true, finisher: true, attacking: true, ended: true, alive: true };
        assert!(breaks(ok));
        for h in [Hit { boss: false, ..ok }, Hit { finisher: false, ..ok }, Hit { attacking: false, ..ok }, Hit { ended: false, ..ok }, Hit { alive: false, ..ok }] {
            assert!(!breaks(h), "{h:?}");
        }
    }

    #[test]
    fn finishers_are_kind_5_rows() {
        let m = kh1_core::Moveset::kh1_defaults();
        // The ground finisher (row 8) and the air finisher (row 16) are; swings 1 / 2 aren't.
        assert!(is_finisher(8, &m) && is_finisher(16, &m));
        assert!(!is_finisher(13, &m) && !is_finisher(11, &m) && !is_finisher(9999, &m));
    }

    #[test]
    fn the_drop_roll_watch() {
        assert_eq!(roll_step(1, None, true, false, false), Roll::Wait);
        assert_eq!(roll_step(3, None, true, false, true), Roll::Dropped);
        assert_eq!(roll_step(3, None, false, false, false), Roll::Gone);
        assert_eq!(roll_step(3, Some(3), true, true, false), Roll::Wait);
        assert_eq!(roll_step(5, Some(3), true, true, false), Roll::NoDrop);
        assert_eq!(roll_step(ROLL_GIVE_UP + 1, None, true, false, false), Roll::NoStagger);
    }

    #[test]
    fn stagger_is_no_kh1_command() {
        use crate::command_menu::cmd;
        assert!(STAGGER >= 0x80);
        assert!(cmd::spell_of(STAGGER).is_none() && cmd::submenu(STAGGER) == 0 && !cmd::row0_special(STAGGER));
        assert!(!crate::specials::cmd::is_ours(STAGGER));
        assert_eq!(cmd::name(STAGGER), STAGGER_WORD);
    }
}
