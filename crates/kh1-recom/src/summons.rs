//! M8b: summons (owner, 2026-11-01: "I want the summon section on the command menu to work like
//! KH1, but each summon should do their lv 3 recom summon equivalent, cloud included"). The
//! testable part: the table, KH1's usable rule, the learn order and its save block. The runtime is
//! `win/summons.rs`; the menu's part is `crate::command_menu`; notes `docs/recom-summons.md`.
//!
//! - **KH1's Summon list** (Steam, Checked (decomp) unless tagged): the Magic list's last entry
//!   (command 6, `0x1402944C0`), shown as the root's 4th row; its list (type 3, `0x1405289A8`) holds
//!   `0x2F + summon` for each summon of the save's list `+0x7D0` (7 bytes, learn order: event
//!   syscall 568 `0x1401B1250` → `0x1402850B0` appends to the first empty slot).
//! - **Usable** (`0x140288F30`; grey = lock): battle mode (`0x142867364 & 1`), the movement state's
//!   flag 0x80 (state 0 only), Sora's whole MP ≥ the cost, and both party slots filled and alive
//!   (**skipped until the party exists**, owner: [`party_ready`]). KH1's once-per-battle rule
//!   (`0x14298C7AC`) is **not used** (owner, 2026-11-02: "summons should be able to be used as long
//!   as you have MP"); one summon at a time (a second sleight can't start mid-sleight).
//! - **The list** (owner, 2026-11-02: "cloud should also be at the top of the summons list"): Cloud
//!   first when learned, then the others in learn order ([`list_order`]).
//! - **Each summon runs Re:CoM's LV3 sleight** of its card (the same card ×3, Re:CoM's
//!   recognition `0x1401669F0`), started without a card through Sora's vtable `+0x130`
//!   (`0x1402247A0`, Re:CoM's own sleight start from its request bits `0x1401F8150`): Re:CoM
//!   loads, spawns, runs and ends it.
//! - **MP** (owner): from Re:CoM's CP (`BattleCardList +0xA`): 2 MP for the 20-CP cards, 3 for the
//!   25-CP ones; paid at once on the command as KH1's `0x140289210` pays (whole points).

/// How many summons: KH1's six and Re:CoM's Cloud.
pub const COUNT: usize = 7;
/// KH1's first summon command (`0x2F + summon`; the command table, `btltbl.bin` `+0xA58`). Checked
/// (files).
pub const FIRST_COMMAND: u8 = 0x2F;
/// Cloud's word: KH1 has none (its `Word.bin` has no "Cloud"); the asset step writes "Cloud" under
/// this id of ours (`kh1-recom-assets` `command_menu::CLOUD_NAME`, the same value): Re:CoM's own
/// card-name text is not in our word system. 0x8000 = ours, 0x29 = Re:CoM's Cloud card.
pub const CLOUD_NAME: u16 = 0x8029;

/// One summon: KH1's side (its index, word, command) and Re:CoM's (its card, its LV3 sleight).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summon {
    pub name: &'static str,
    /// KH1's message for the command (Word.bin 286..291; Cloud: [`CLOUD_NAME`]). Checked (files).
    pub word: u16,
    /// Re:CoM's summon card kind (`0x1405256D0`, type 3: 0x23 Simba .. 0x29 Cloud). Checked (exe).
    pub card: u16,
    /// Re:CoM's LV3 sleight (`SY0002.CTD` order from id 4; `initStockSkill` `0x140220DF0`; the
    /// recognition `0x1401669F0`: the same card ×3). Checked (decomp + files).
    pub sleight: u16,
    pub sleight_name: &'static str,
    /// The card's CP (`BattleCardList +0xA`, the session's `SY0002.BIN`). Checked (files).
    pub cp: u8,
    /// Re:CoM's attack id for the sleight (`initStockSkill`: `+0x638` = sleight + 0x4D, Twinkle
    /// and Cloud + 0x50). Checked (decomp).
    pub attack_id: u16,
    /// The sleight's resource group (`0x14020F720`) and its PLAYER pack (`0x14020FB10`,
    /// `PLAYER/<pack>/PL<pack>.*`). Checked (decomp).
    pub group: u8,
    pub pack: u16,
    /// Re:CoM's summon actor class (`0x14020EE80` by `0x14020FF40`). Checked (decomp + RTTI names).
    pub class: &'static str,
}

/// By our index = KH1's summon index (`0x140528198`'s order: 0 Dumbo, 1 Bambi, 2 Genie, 3 Tinker
/// Bell, 4 Mushu, 5 Simba), and 6 = Cloud, in the slot KH1 kept for Bahamut (its command 0x35;
/// unusable in KH1: its handler entry is not code). Checked (files + exe).
pub const ALL: [Summon; COUNT] = [
    Summon { name: "Dumbo", word: 286, card: 0x26, sleight: 0x37, sleight_name: "Splash LV3", cp: 20, attack_id: 0x84, group: 0x28, pack: 302, class: "CSummonDumbo" },
    Summon { name: "Bambi", word: 287, card: 0x25, sleight: 0x39, sleight_name: "Paradise LV3", cp: 20, attack_id: 0x86, group: 0x27, pack: 303, class: "CSummonBambi" },
    Summon { name: "Genie", word: 288, card: 0x24, sleight: 0x3E, sleight_name: "Showtime LV3", cp: 20, attack_id: 0x8B, group: 0x26, pack: 305, class: "CSummonGenie" },
    Summon { name: "Tinker Bell", word: 289, card: 0x27, sleight: 0x40, sleight_name: "Twinkle LV3", cp: 20, attack_id: 0x90, group: 0x29, pack: 306, class: "CSummonTink" },
    Summon { name: "Mushu", word: 290, card: 0x28, sleight: 0x3C, sleight_name: "Flare Breath LV3", cp: 25, attack_id: 0x89, group: 0x2A, pack: 304, class: "CSummonMushu" },
    Summon { name: "Simba", word: 291, card: 0x23, sleight: 0x35, sleight_name: "Proud Roar LV3", cp: 20, attack_id: 0x82, group: 0x25, pack: 301, class: "CSummonSinba" },
    Summon { name: "Cloud", word: CLOUD_NAME, card: 0x29, sleight: 0x42, sleight_name: "Omnislash", cp: 25, attack_id: 0x92, group: 0x2B, pack: 307, class: "CSummonCloud" },
];

/// Re:CoM's premium bit of a card id.
pub const PREMIUM: u16 = 0x100;

/// The owner's MP cost from the CP (2026-11-01): 20 CP → 2 MP, 25 CP → 3 MP (CP / 10, up).
pub const fn mp_from_cp(cp: u8) -> i32 {
    (cp as i32 + 9) / 10
}

/// The MP cost of summon `i` (whole points).
pub fn mp_cost(i: usize) -> i32 {
    ALL.get(i).map_or(i32::MAX, |s| mp_from_cp(s.cp))
}

/// The summon actors (`Summon::CSummon*`, made by `0x14020EE80`), in [`ALL`]'s order: (class, its
/// vtable RVA, the vtable's slot `+0` RVA). Every one's slot `+0x28` (the actor type getter) is
/// `0x1401A4A30`, which returns 0x1000, so Re:CoM's bullet manager `0x140158DD0` writes the summon's
/// own id `+0x20` as a shot's shooter `+0x238`; its melee hits come from its own attack holder
/// `+0x100` (the actor base's `0x14014D420(holder, actor)`, as an enemy's). `+0x248` is the player
/// it was summoned for (its set-up `0x1402271D0`, from `0x1402261D0`, copies it from `*0x14087B6B8 +
/// 0x60`; the summon's own code reads Sora's `+0x418` / `+0x3D8` / `+0x854` through it). Checked
/// (exe: RTTI `.?AVCSummon…@Summon@@`, the complete object locators and vtables; decomp).
pub const ACTOR_VTABLES: [(&str, u32, u32); COUNT] = [
    ("CSummonDumbo", 0x005A_EF98, 0x0022_ABD0),
    ("CSummonBambi", 0x005A_CED8, 0x0022_7EC0),
    ("CSummonGenie", 0x005A_FFD8, 0x0022_B460),
    ("CSummonTink", 0x005B_2F68, 0x0022_DAB0),
    ("CSummonMushu", 0x005B_0FC0, 0x0022_C540),
    ("CSummonSinba", 0x005B_1FA8, 0x0022_D130),
    ("CSummonCloud", 0x005A_DF70, 0x0022_8F00),
];
/// The actor type getter in every summon vtable's slot `+0x28` (returns 0x1000).
pub const ACTOR_TYPE_GETTER: u32 = 0x001A_4A30;
/// The summon's player (`+0x248`).
pub const ACTOR_PLAYER: usize = 0x248;

/// The name of a sleight whose hits are scaled at Damage = KH1 (`win::summons::scale_sleight_hit`):
/// a sleight ability's, or a summon's LV3 sleight's.
pub fn hit_name(id: u16) -> &'static str {
    if crate::abilities::of_sleight(id).is_some() {
        return crate::sleights::name(id);
    }
    ALL.iter().find(|s| s.sleight == id).map_or("?", |s| s.sleight_name)
}

/// Summon `i`'s command (`0x2F + i`).
pub const fn command(i: usize) -> u8 {
    FIRST_COMMAND + i as u8
}

/// The summon of a command.
pub fn of_command(c: u8) -> Option<usize> {
    let i = c.wrapping_sub(FIRST_COMMAND) as usize;
    (i < COUNT).then_some(i)
}

/// The summon of a Re:CoM card id (plain or premium).
pub fn of_card(id: u16) -> Option<usize> {
    let kind = id & 0xFF;
    ALL.iter().position(|s| s.card == kind)
}

/// KH1's party test in `0x140288F30` (both party slots filled, without status `+0x88 & 0x30`,
/// alive by `0x1402B0030`). **Party hook:** the owner chose to skip it until persistent Donald and
/// Goofy exist (2026-11-01): they count as present and alive. When the party milestone lands, this
/// takes their live state.
pub const fn party_ready() -> bool {
    true
}

/// What KH1's usable test reads, from Re:CoM's side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gate {
    /// KH1's battle mode: Re:CoM's battle room (`player+0x390`; battle only, as Re:CoM's sleights).
    pub battle: bool,
    /// KH1's movement state's flag 0x80 (state 0).
    pub ground: bool,
    /// M12 follow-ups (the Sleights list's air start): KH1's movement state lets a command start
    /// while Sora is off the ground: its magic flag 0x20 without the ground flag 0x80 (take-off 1,
    /// air 2, flight 8; `crate::command_menu::state_flags`). The summons never read it (ground only).
    pub air: bool,
    /// Sora's whole MP (KH1's `+0x44`; the charge doesn't count).
    pub mp: i32,
    /// A summon is out (Re:CoM's sleight running; KH1's in-progress flag `0x142867364 & 0x2000`):
    /// none can start until it ends.
    pub out: bool,
    /// Sleights outside battle (owner, 2026-11-04): a field room where Re:CoM's sleight code runs
    /// (Combat = KH1 in Sora's story, the Field exe's case 0x16 in: `crate::field_sleights`).
    pub field: bool,
    /// Re:CoM's sleight packs load in this room (`crate::field_sleights::packs`: always in battle;
    /// in a field room where the field PLM got the battle branch's writes).
    pub packs: bool,
}

/// `0x140288F30` for summon `i`, without KH1's once-per-battle test (owner). Outside battle (owner,
/// 2026-11-04: the LV3 summons too, free): a field room where the packs load, no MP.
pub fn usable(g: &Gate, i: usize) -> bool {
    i < COUNT
        && (g.battle || (g.field && g.packs))
        && g.ground
        && !g.out
        && (crate::field_sleights::free(g) || g.mp >= mp_cost(i))
        && party_ready()
}

/// Our index of Cloud.
pub const CLOUD: usize = 6;

/// The Summon list's order (owner, 2026-11-02): Cloud first when learned, then the others in learn
/// order.
pub fn list_order(learned: &[usize]) -> Vec<usize> {
    let mut v: Vec<usize> = learned.iter().copied().filter(|&i| i == CLOUD).take(1).collect();
    v.extend(learned.iter().copied().filter(|&i| i != CLOUD));
    v
}

// ---- the learn order and its save block ----

/// Our summon block: right after the ability block, inside Re:CoM's spare tail
/// `save + 0x246B ..= 0x2817` that no Re:CoM code reads or writes (`docs/recom-keyblade-equip.md`).
pub const BLOCK_FIRST: usize = crate::abilities::BLOCK_LAST + 1;
pub const BLOCK_LEN: usize = 9;
pub const BLOCK_LAST: usize = BLOCK_FIRST + BLOCK_LEN - 1;

/// Offsets inside the block. Fields never move.
pub mod field {
    /// 0 = never written (an older save, or a new game), 1 = this layout.
    pub const VERSION: usize = 0;
    /// KH1's list `+0x7D0`: 7 bytes, summon index + 1 (0 = empty), in learn order.
    pub const ORDER: usize = 1;
    /// The command menu's page (0.111.0, `crate::command_menu::Page`: 2 = page 2, anything else
    /// page 1). M12 step 5 kept its bottom row here (0 Summon, 1 Sleights): those saves read page 1.
    /// It was the block's zero reserve; older builds' sync copies it unchanged ([`sync`] starts from
    /// the old block).
    pub const PAGE: usize = 8;
}

pub const BLOCK_V1: u8 = 1;

const _: () = assert!(BLOCK_LAST <= crate::keyblades::SPARE_LAST && field::PAGE + 1 == BLOCK_LEN && field::ORDER + COUNT == field::PAGE);

/// The learned summons in learn order (KH1's list).
pub fn order(block: &[u8; BLOCK_LEN]) -> Vec<usize> {
    block[field::ORDER..field::ORDER + COUNT].iter().take_while(|&&b| b != 0).map(|&b| b as usize - 1).filter(|&i| i < COUNT).collect()
}

/// `0x1402850B0`: summon `i` goes into the first empty slot unless it is already listed. True when
/// it was added.
pub fn learn(block: &mut [u8; BLOCK_LEN], i: usize) -> bool {
    if i >= COUNT {
        return false;
    }
    for k in 0..COUNT {
        let b = &mut block[field::ORDER + k];
        if *b as usize == i + 1 {
            return false;
        }
        if *b == 0 {
            *b = i as u8 + 1;
            return true;
        }
    }
    false
}

/// Brings the block up to Re:CoM's discovery state (`owned(card)`: the card, plain or premium,
/// obtained at least once). A never-written block takes the summons already owned in KH1's index
/// order, Cloud last (an older save has no order to recover; owner); then each newly owned one is
/// appended. Returns the new block and the summons added, in order.
pub fn sync(old: [u8; BLOCK_LEN], mut owned: impl FnMut(u16) -> bool) -> ([u8; BLOCK_LEN], Vec<usize>) {
    let mut b = old;
    let mut added = Vec::new();
    for (i, s) in ALL.iter().enumerate() {
        if (owned(s.card) || owned(s.card | PREMIUM)) && learn(&mut b, i) {
            added.push(i);
        }
    }
    b[field::VERSION] = BLOCK_V1;
    (b, added)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M12 follow-ups ("KH1 damage scaling"): the summons' hits are scaled as a sleight's and logged
    /// by their sleight's name; the summon actors are the table's classes.
    #[test]
    fn scaled_hits_are_named_and_the_actors_are_the_summons_classes() {
        assert_eq!(hit_name(0x42), "Omnislash");
        assert_eq!(hit_name(0x35), "Proud Roar LV3");
        assert_eq!(hit_name(crate::abilities::sleight::FIRE_RAID), "Fire Raid");
        assert_eq!(hit_name(crate::abilities::sleight::IDYLL_ROMP), "Idyll Romp");
        assert_eq!(hit_name(0x200), "?");
        for (s, (class, vt, _)) in ALL.iter().zip(ACTOR_VTABLES) {
            assert_eq!(s.class, class);
            assert!((0x005A_C000..0x005B_4000).contains(&vt), "{class}");
        }
    }

    #[test]
    fn table_is_the_research() {
        // KH1's words 286..291 (Dumbo .. Simba), the commands 0x2F.., Cloud in Bahamut's slot.
        assert_eq!(ALL[..6].iter().map(|s| s.word).collect::<Vec<_>>(), vec![286, 287, 288, 289, 290, 291]);
        assert_eq!((command(0), command(5), command(6)), (0x2F, 0x34, 0x35));
        assert_eq!((of_command(0x2F), of_command(0x35), of_command(0x36), of_command(0x2E)), (Some(0), Some(6), None, None));
        // Re:CoM's cards 0x23..0x29, each once; the LV3 sleights and their attack ids.
        let mut cards: Vec<u16> = ALL.iter().map(|s| s.card).collect();
        cards.sort();
        assert_eq!(cards, (0x23..=0x29).collect::<Vec<_>>());
        for s in &ALL {
            // `initStockSkill`: + 0x4D, Twinkle (0x3F..0x40) and Cloud (0x41..0x43) + 0x50.
            let add = if s.sleight >= 0x3F { 0x50 } else { 0x4D };
            assert_eq!(s.attack_id, s.sleight + add, "{}", s.name);
            assert_eq!(s.pack, pack_of_group(s.group), "{}", s.name);
        }
        assert_eq!(of_card(0x29 | PREMIUM), Some(6));
        assert_eq!(of_card(0x23), Some(5));
        assert_eq!(of_card(0x1A), None);
    }

    /// `0x14020FB10` for the summon groups.
    fn pack_of_group(g: u8) -> u16 {
        match g {
            0x25 => 0x12D,
            0x26 => 0x131,
            0x27 => 0x12F,
            0x28 => 0x12E,
            0x29 => 0x132,
            0x2A => 0x130,
            0x2B => 0x133,
            _ => 0,
        }
    }

    #[test]
    fn mp_from_the_cards_cp() {
        // Owner: 2 MP for the 20-CP cards (Simba, Genie, Bambi, Dumbo, Tinker Bell), 3 for Mushu
        // and Cloud (25 CP).
        let by_name = |n: &str| mp_cost(ALL.iter().position(|s| s.name == n).unwrap());
        for n in ["Simba", "Genie", "Bambi", "Dumbo", "Tinker Bell"] {
            assert_eq!(by_name(n), 2, "{n}");
        }
        assert_eq!((by_name("Mushu"), by_name("Cloud")), (3, 3));
        assert_eq!(mp_cost(7), i32::MAX);
    }

    #[test]
    fn kh1_usable_rule() {
        let g = Gate { battle: true, ground: true, air: false, mp: 2, out: false, ..Default::default() };
        assert!(usable(&g, 5), "Simba with 2 MP");
        assert!(!usable(&g, 4), "Mushu needs 3");
        assert!(!usable(&Gate { battle: false, ..g }, 5), "battle mode only");
        assert!(!usable(&Gate { ground: false, ..g }, 5), "state 0 only");
        assert!(!usable(&Gate { ground: false, air: true, ..g }, 5), "summons stay ground-only");
        assert!(!usable(&Gate { out: true, ..g }, 5), "one out at a time");
        assert!(!usable(&g, 7));
        assert!(party_ready());
    }

    /// Owner (2026-11-04, "Sleights outside battle"): the LV3 summons in the field too, free, where
    /// Re:CoM's packs load (every summon has one: `crate::field_sleights::packs_are_recoms`).
    #[test]
    fn summons_in_the_field_are_free_where_the_packs_load() {
        let f = Gate { field: true, packs: true, ground: true, mp: 0, ..Default::default() };
        for i in 0..COUNT {
            assert!(usable(&f, i), "{}: free in the field", ALL[i].name);
        }
        assert!(!usable(&Gate { packs: false, ..f }, 5), "100 Acre Wood / rooms 2000-2999: greyed");
        assert!(!usable(&Gate { ground: false, ..f }, 5));
        assert!(!usable(&Gate { out: true, ..f }, 5));
        assert!(!usable(&Gate { field: false, ..f }, 5), "neither room");
        // In battle the MP rule stays.
        assert!(!usable(&Gate { battle: true, ..f }, 5));
    }

    #[test]
    fn learn_order_as_kh1s_list() {
        let mut b = [0u8; BLOCK_LEN];
        assert!(learn(&mut b, 5));
        assert!(learn(&mut b, 0));
        assert!(!learn(&mut b, 5), "already listed");
        assert_eq!(order(&b), vec![5, 0]);
        // An older save: KH1's index order, Cloud last.
        let (b, added) = sync([0; BLOCK_LEN], |c| [0x23u16, 0x29, 0x26 | PREMIUM].contains(&c));
        assert_eq!(added, vec![0, 5, 6]);
        assert_eq!(order(&b), vec![0, 5, 6]);
        assert_eq!(b[field::VERSION], BLOCK_V1);
        // A later card is appended; nothing is ever removed.
        let (b2, added) = sync(b, |c| [0x23u16, 0x29, 0x26, 0x25].contains(&c));
        assert_eq!(added, vec![1]);
        assert_eq!(order(&b2), vec![0, 5, 6, 1]);
        let (b3, added) = sync(b2, |_| false);
        assert!(added.is_empty());
        assert_eq!(b3, b2);
        assert_eq!(b3[field::PAGE], 0);
        // The page byte survives every sync (and a never-written block's).
        let mut b4 = b3;
        b4[field::PAGE] = 1;
        assert_eq!(sync(b4, |_| true).0[field::PAGE], 1);
        let mut fresh = [0; BLOCK_LEN];
        fresh[field::PAGE] = 1;
        assert_eq!(sync(fresh, |_| false).0[field::PAGE], 1);
    }

    #[test]
    fn cloud_heads_the_list() {
        assert_eq!(list_order(&[5, 0, 6, 1]), vec![6, 5, 0, 1]);
        assert_eq!(list_order(&[6]), vec![6]);
        assert_eq!(list_order(&[5, 0]), vec![5, 0]);
        assert_eq!(list_order(&[]), Vec::<usize>::new());
        assert_eq!(ALL[CLOUD].name, "Cloud");
    }

    #[test]
    fn the_block_follows_the_ability_block() {
        assert_eq!(BLOCK_FIRST, crate::abilities::BLOCK_LAST + 1);
        assert!(BLOCK_LAST <= crate::keyblades::SPARE_LAST);
    }
}
