//! M6 stage 3, bosses at the KH1 setting (decisions 29 / 30; `docs/recom-m6-battles.md` §
//! Stage 3): the testable part. The runtime is in `win/battle.rs` (P5, the boss logs) and
//! `win/deck.rs` (the enemy card UI reads).
//!
//! At Combat = KH1, in Sora's story:
//!
//! - **P5, "Sora is attacking".** Re:CoM's getter `0x14017F720` answers Sora's action
//!   (`player+0x40C`). Twelve boss reads compare it with `0xB` (Re:CoM's card attack); KH1's
//!   swings, air swings and Counterattack run as action `0xA`. A 32-byte in-place rewrite of the
//!   getter answers `0xB` for `0xA`, written once per battle when Re:CoM builds its battle enemy
//!   manager (stage 1's switch), Re:CoM's exact bytes otherwise.
//! - **Human bosses keep their decks as a hidden move list** (decision 30): their revolver, CBTL
//!   and message 2 run as in Re:CoM. Their side-2 cards, the enemy card cue, the enemy revolver's
//!   stock row (sleight name and stocked value) and Marluxia 3's stock folder are hidden through
//!   Re:CoM's own event-hide mask (system variable (1, 0xE2)) at the reads that draw them, as
//!   stage 2 hides Sora's deck HUD.
//!
//! Every offset is Steam, read in the Re:CoM decomp and checked in the exe where noted.

/// Sora's action ids the getter's readers compare with. Checked (decomp + exe: the 15 callers).
pub mod action {
    /// KH1's Attack: ground and air swings and Counterattack all start through Re:CoM's
    /// field-swing init `0x1401F0F70` (`crate::attack`), which sets this action.
    pub const SWING: u32 = 0xA;
    /// Re:CoM's card attack (`initAttack` `0x1401F0D90`).
    pub const CARD_ATTACK: u32 = 0xB;
    /// Damage (`0x1401E6DE0`): Ansem's and his shadows' reads.
    pub const DAMAGE: u32 = 0xC;
    /// Riku's-story dark action: Lexaeus's read.
    pub const RIKU_DARK: u32 = 0x22;
    /// Sora's card duel (`0x140220C40` / `0x1402184D0`), started by hit reaction 0x21.
    pub const CARD_DUEL: u32 = 0x24;
}

/// What the getter answers without a main player.
pub const NO_PLAYER: u32 = 2;

/// What `0x14017F720` answers: Re:CoM's `player+0x40C` (2 without a main player); with P5, `0xA`
/// becomes `0xB`. Exactly what the rewritten bytes do (`sites::M6_BOSS_PATCHES`).
pub fn answer(action: Option<u32>, p5: bool) -> u32 {
    match action {
        None => NO_PLAYER,
        Some(action::SWING) if p5 => action::CARD_ATTACK,
        Some(a) => a,
    }
}

/// One caller of the getter: where it calls, whose code it is, what it compares the answer with,
/// what it does then, and whether it runs in Sora's story. Checked (exe: the 15 `call`s, each
/// followed by `cmp eax,imm8`; decomp; RTTI owners).
#[derive(Clone, Copy, Debug)]
pub struct Reader {
    pub call: u32,
    pub owner: &'static str,
    pub compares: u32,
    pub does: &'static str,
    pub sora_story: bool,
}

pub const READERS: [Reader; 15] = [
    Reader { call: 0x0019_9B2B, owner: "CHumanBoss card play 0x140199A00 (Leon, Cloud, Hades, Vexen, Riku, Larxene, Axel, Hook, Marluxia 1, Zexion)", compares: action::CARD_ATTACK, does: "holds its card back when it is below Sora's playing value 0x140160910(1): -1 at KH1, so never", sora_story: true },
    Reader { call: 0x003A_9CFD, owner: "CTutorialLeon think 0x1403A9CE0", compares: action::CARD_ATTACK, does: "plays his card (his only attack), then message 2 sets event flag (1, 0x1B)", sora_story: true },
    Reader { call: 0x003A_FA7F, owner: "CBossCloud state 1 0x1403AF8E0", compares: action::CARD_ATTACK, does: "1 in 10 per call, in range: dodge (0x1403AEAD0, motion e106_005)", sora_story: true },
    Reader { call: 0x003A_FDB1, owner: "CBossCloud state 1 0x1403AF8E0", compares: action::CARD_ATTACK, does: "while turning: the same dodge check", sora_story: true },
    Reader { call: 0x003B_7E06, owner: "CBossRiku state 1 0x1403B7C80", compares: action::CARD_ATTACK, does: "close (< 25): 1 in 50 a reaction", sora_story: true },
    Reader { call: 0x003B_D744, owner: "CBossLarxene state 1 0x1403BD320", compares: action::CARD_ATTACK, does: "close (<= 9): 1 in 100 a reaction", sora_story: true },
    Reader { call: 0x003C_1998, owner: "CBossAxel state 1 0x1403C1760", compares: action::CARD_ATTACK, does: "1 in 80 a reaction (motion e118_003)", sora_story: true },
    Reader { call: 0x003C_273A, owner: "CBossParasiteCage think 0x1403C26F0", compares: action::CARD_ATTACK, does: "below 30% HP: an attack request at once (else 1 in 30)", sora_story: true },
    Reader { call: 0x003C_9F1A, owner: "CBossHook state 1 0x1403C9AF0", compares: action::CARD_ATTACK, does: "close (<= 9): 1 in 25 a reaction", sora_story: true },
    Reader { call: 0x0041_2B4E, owner: "CBossMarluxia state 1 0x140412850", compares: action::CARD_ATTACK, does: "close (<= 6.25): 1 in 10 a reaction instead of a move", sora_story: true },
    Reader { call: 0x0041_4F1E, owner: "CBossLexaeus card play 0x140414D80", compares: action::CARD_ATTACK, does: "as the CHumanBoss card play", sora_story: false },
    Reader { call: 0x0041_801F, owner: "CBossLexaeus 0x140417F80", compares: action::RIKU_DARK, does: "Riku's action 0x22", sora_story: false },
    Reader { call: 0x0042_4BCD, owner: "CBossAnsem card play 0x140424A50", compares: action::CARD_ATTACK, does: "as the CHumanBoss card play", sora_story: false },
    Reader { call: 0x0042_6890, owner: "CBossAnsem 0x1404266C0", compares: action::DAMAGE, does: "a longer wait while the player is in damage", sora_story: false },
    Reader { call: 0x0045_FF38, owner: "CEnemyAnsemShadowUppercut 0x14045FF00", compares: action::DAMAGE, does: "no shot while the player is in damage", sora_story: false },
];

/// Re:CoM's event-hide mask bits for the enemy side (system variable (1, 0xE2)). Checked (decomp
/// `0x140150340`, `0x140169CA0`, `0x14016DE70`, `0x140169070`).
pub mod mask {
    /// Every side-2 `CBattleCard` (`0x140150340`: 0x20 for side 1, 0x40 otherwise).
    pub const ENEMY_CARDS: u32 = 0x40;
    /// The side-2 revolver's stock row and Marluxia 3's `CStockFolder` (their reads test 4 for
    /// side 2, 2 for side 1; the folder only 4). Also nonzero for the cue's whole-mask tests.
    pub const ENEMY_REVOLVER: u32 = 4;
}

/// The player's side (`CBattleCard +0x26C`, a revolver's `+0x5C`).
pub const SIDE_PLAYER: u8 = 1;

/// The bits a card's mask read adds for an enemy card: 0x40 for any side-2 card while this battle
/// hides the enemy card UI. Side-1 cards get stage 2's 0x20 (`crate::deck::card_bits`).
pub fn enemy_card_bits(side: u8, hide: bool) -> u32 {
    if hide && side != SIDE_PLAYER {
        mask::ENEMY_CARDS
    } else {
        0
    }
}

/// The bits the enemy-UI reads add (the cue, the stock row, the stock folder): 4 while this
/// battle hides the enemy card UI. These reads either test 4 for the enemy side (the stock row
/// tests 2 for Sora's own revolver, so it is never touched) or treat any nonzero mask as "hide".
/// They take no object: at `0x14016E2B1` rbx holds the bit, not a pointer.
pub fn enemy_ui_bits(hide: bool) -> u32 {
    if hide {
        mask::ENEMY_REVOLVER
    } else {
        0
    }
}

/// What the enemy-UI read groups hide, for the logs and the doc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiPart {
    /// The card-play cue sprite over the attacking enemy (CBTL `+0x220`).
    Cue,
    /// The side-2 revolver's stock row: sleight name `+0x250`, stocked value digits.
    StockRow,
    /// Marluxia 3's `CStockFolder` (frame and digits; its cards are side-2 `CBattleCard`s).
    StockFolder,
}

impl UiPart {
    pub fn as_str(self) -> &'static str {
        match self {
            UiPart::Cue => "enemy card cue",
            UiPart::StockRow => "enemy stock row (sleight name)",
            UiPart::StockFolder => "Marluxia 3's stock folder",
        }
    }
}

/// The P5 part of the battle-start line.
pub fn p5_part(kh1: bool, switched: Option<usize>) -> String {
    match switched {
        Some(n) if kh1 => format!("ours, Sora's action 0xA (KH1 swings, Counterattack) reads as 0xB ({n} written)"),
        Some(n) => format!("Re:CoM's ({n} written)"),
        None => "UNCHANGED: the getter held unexpected bytes (see above)".into(),
    }
}

/// One side-2 card set in a KH1 battle (a human boss's attack, sleight or reload card, or a
/// Heartless that took Re:CoM's card path).
pub struct CardLine<'a> {
    pub battle: u32,
    pub n: u32,
    pub class: &'a str,
    pub id: u32,
    pub revolver_card: bool,
    pub action: u32,
    pub answer: u32,
    pub duel_mode: u32,
    pub hidden: bool,
}

pub fn card_line(l: &CardLine) -> String {
    format!(
        "boss card (battle {}, #{}): {} id={} sets a side-2 {} ({}); Sora's action {:#x}, bosses read {:#x}; CBTL duel mode {}",
        l.battle,
        l.n,
        l.class,
        l.id,
        if l.revolver_card { "revolver card" } else { "new card" },
        if l.hidden { "hidden" } else { "SHOWN: the enemy card reads are Re:CoM's this battle" },
        l.action,
        l.answer,
        l.duel_mode
    )
}

/// Sora's card duel started (hit reaction 0x21; Marluxia 3's countdown) in a KH1 battle.
pub fn duel_line(battle: u32, cards: i16) -> String {
    format!(
        "Sora's card duel (action 0x24, battle {battle}) starts at KH1: Re:CoM's countdown runs; Sora's revolver holds {cards} card(s) besides the Reload card, which the duel play refuses, so he cannot answer and the countdown ends it as lost. Open question, docs § Stage 3"
    )
}

/// The F9 trace's stage-3 token.
pub fn trace_token(p5: &str, action: Option<u32>, answer: u32, duel_mode: Option<u32>, hidden: bool) -> String {
    format!(
        "m6boss p5={p5} act={} ans={answer:#x} duel={} ui={}",
        action.map_or("none".to_string(), |a| format!("{a:#x}")),
        duel_mode.map_or("none".to_string(), |d| d.to_string()),
        if hidden { "hidden" } else { "shown" }
    )
}

/// Log caps: per battle, and in all (boss fights come late in the game, after stage 1's caps).
pub const BATTLE_LINES: u32 = 40;
pub const TOTAL_LINES: u32 = 1200;
pub const DUEL_LOG_LINES: u32 = 20;

/// Whether a capped boss line logs: lines left in this battle and in all.
pub fn boss_line(in_battle: u32, total: u32) -> bool {
    in_battle < BATTLE_LINES && total < TOTAL_LINES
}

/// A boss class, by its RTTI name (the per-battle boss lines; other classes keep stage 1's caps).
pub fn is_boss_class(name: &str) -> bool {
    name.starts_with("CBoss") || name.starts_with("CTutorial")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hookplan::call_target;
    use crate::sites::{M6_BOSS_PATCHES, M6_GETTER_CALLERS};

    #[test]
    fn p5_answers_0xb_only_for_0xa() {
        for a in 0..0x40u32 {
            let want = if a == action::SWING { action::CARD_ATTACK } else { a };
            assert_eq!(answer(Some(a), true), want);
            assert_eq!(answer(Some(a), false), a);
        }
        assert_eq!(answer(None, true), NO_PLAYER);
        assert_eq!(answer(None, false), NO_PLAYER);
    }

    #[test]
    fn no_reader_changes_but_the_0xb_ones() {
        // Each reader's test (answer == its value) changes only for Sora's action 0xA, and only
        // where the value is 0xB. No reader compares with 0xA.
        for r in READERS {
            assert_ne!(r.compares, action::SWING, "{}", r.owner);
            for a in 0..0x40u32 {
                let before = answer(Some(a), false) == r.compares;
                let after = answer(Some(a), true) == r.compares;
                if before != after {
                    assert_eq!((a, r.compares), (action::SWING, action::CARD_ATTACK), "{}", r.owner);
                }
            }
        }
        // Twelve 0xB reads, ten of them in Sora's story (Cloud has two); five reads in all are
        // Riku's story's (Lexaeus, Ansem, Ansem's shadow).
        assert_eq!(READERS.iter().filter(|r| r.compares == action::CARD_ATTACK).count(), 12);
        assert_eq!(READERS.iter().filter(|r| r.compares == action::CARD_ATTACK && r.sora_story).count(), 10);
        assert_eq!(READERS.iter().filter(|r| !r.sora_story).count(), 5);
    }

    #[test]
    fn readers_are_the_checked_callers() {
        // The same 15 call sites, each a call to the getter followed by `cmp eax,<value>`.
        let getter = 0x1_4000_0000u64 + M6_BOSS_PATCHES[0].site.rva as u64;
        assert_eq!(M6_GETTER_CALLERS.len(), READERS.len());
        for (s, r) in M6_GETTER_CALLERS.iter().zip(READERS) {
            assert_eq!(s.rva, r.call, "{}", s.name);
            assert_eq!(call_target(0x1_4000_0000 + s.rva as u64, &s.expect[..5]), Some(getter), "{}", s.name);
            let e = s.expect;
            assert_eq!(&e[e.len() - 3..e.len() - 1], &[0x83, 0xF8], "{}", s.name);
            assert_eq!(e[e.len() - 1] as u32, r.compares, "{}", s.name);
        }
    }

    #[test]
    fn p5_bytes() {
        let p = &M6_BOSS_PATCHES[0];
        assert_eq!(p.site.rva, 0x0017_F720);
        // The whole function (25 bytes) and its 7 int3: the next function starts at +0x20.
        assert_eq!(p.site.expect.len(), 32);
        assert_eq!(p.new.len(), 32);
        assert!(p.site.expect[25..].iter().all(|&b| b == 0xCC));
        // The first instruction (RIP-relative load of the main player) and the null test stay.
        assert_eq!(&p.new[..10], &p.site.expect[..10]);
        // je +0x0E lands on `mov eax,2; ret`; jne +2 lands on the `ret` after `mov al,0xB`.
        assert_eq!((p.new[10], p.new[11]), (0x74, 0x0E));
        assert_eq!(&p.new[12 + 0x0E..], &[0xB8, 0x02, 0x00, 0x00, 0x00, 0xC3]);
        assert_eq!(&p.new[12..18], &p.site.expect[12..18], "mov eax,[rax+0x40C]");
        assert_eq!(&p.new[18..21], &[0x83, 0xF8, action::SWING as u8]);
        assert_eq!((p.new[21], p.new[22]), (0x75, 0x02));
        assert_eq!(&p.new[23..26], &[0xB0, action::CARD_ATTACK as u8, 0xC3]);
    }

    #[test]
    fn enemy_bits() {
        assert_eq!(enemy_card_bits(2, true), mask::ENEMY_CARDS);
        assert_eq!(enemy_card_bits(2, false), 0);
        assert_eq!(enemy_card_bits(SIDE_PLAYER, true), 0);
        assert_eq!(enemy_ui_bits(true), 4);
        assert_eq!(enemy_ui_bits(false), 0);
        // The enemy bits never reach a side-1 test (0x20 cards, 2 cockpit / Sora's stock row).
        assert_eq!(mask::ENEMY_CARDS & (0x20 | 2), 0);
        assert_eq!(mask::ENEMY_REVOLVER & (0x20 | 2), 0);
    }

    #[test]
    fn lines() {
        assert!(p5_part(true, Some(1)).starts_with("ours, Sora's action 0xA"));
        assert_eq!(p5_part(false, Some(0)), "Re:CoM's (0 written)");
        assert!(p5_part(true, None).starts_with("UNCHANGED"));
        let l = card_line(&CardLine { battle: 3, n: 2, class: "CBossCloud", id: 101, revolver_card: true, action: 0xA, answer: 0xB, duel_mode: 0, hidden: true });
        assert_eq!(l, "boss card (battle 3, #2): CBossCloud id=101 sets a side-2 revolver card (hidden); Sora's action 0xa, bosses read 0xb; CBTL duel mode 0");
        assert!(duel_line(4, 0).contains("holds 0 card(s)"));
        assert_eq!(trace_token("ours", Some(0xA), 0xB, Some(0), true), "m6boss p5=ours act=0xa ans=0xb duel=0 ui=hidden");
        assert_eq!(trace_token("recom", None, 2, None, false), "m6boss p5=recom act=none ans=0x2 duel=none ui=shown");
        assert!(boss_line(0, 0));
        assert!(!boss_line(BATTLE_LINES, 0));
        assert!(!boss_line(0, TOTAL_LINES));
        assert!(is_boss_class("CBossMarluxia3") && is_boss_class("CTutorialLeon"));
        assert!(!is_boss_class("CEnemyShadow") && !is_boss_class("CHeartless"));
        assert_eq!(UiPart::StockFolder.as_str(), "Marluxia 3's stock folder");
    }
}
