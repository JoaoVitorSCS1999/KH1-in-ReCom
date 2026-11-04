//! Sleights outside battle (owner, 2026-11-04, `design-philosophy-instructions.md` § Sleights outside
//! battle: "(a) Plain field strike", "Free in the field", "Re:CoM's special field reactions kept, for
//! sleights and spells", "all sleights, the KH1 specials … and the LV3 summons"). The testable part;
//! Re:CoM's side is `win/field_sleights.rs`, the research and what was built
//! `docs/recom-field-sleights.md`.
//!
//! Re:CoM's own sleight code runs in the field room. Four things stopped it there, each now given
//! natively:
//!
//! 1. **The Field exe's case for action 0x16** ([`field_case`]): its jump table's entry
//!    `0x1401FA648` sends the action to a 13-byte stub in the int3 cave `0x1401FB7B2`: `mov rcx,rbx;
//!    call exeStockSkill 0x140218950; jmp 0x1401FA599`, as the Battle exe's case calls it and as the
//!    magic (0x13) and items (0xE) field cases are built. Unreachable at Combat = Re:CoM: nothing sets
//!    0x16 in the field there (a sleight comes only from a card play).
//! 2. **The list pointers** ([`lists`]): `StockSkillList` (`*0x14087A810`) and `BattleCardList`
//!    (`*0x14087A808`), set only by the `CCardBattle` ctor, filled by Re:CoM's own lookup at a field
//!    start (`0x1402A43E0(1, name, 0)` → `0x1402A49C0`, the ctor's two calls).
//! 3. **Sleight packs in the field** ([`packs`]): Re:CoM's PLM init `0x140210100` turns pack loading
//!    on only in battle and minigame rooms; a wrapper does the battle branch's own writes for a field
//!    room where the field allocated the ring (not in world 10, 100 Acre Wood, nor rooms 2000–2999).
//! 4. **Our menu gate** (`crate::sleights::usable`, `crate::summons::usable`): a field room too
//!    (`Gate::field`), the pack ones only where the packs load (`Gate::packs`), no MP there.

/// The Field exe's case for action 0x16 (Re:CoM's sleight action, `initStockSkill`'s
/// `0x1401F8D70(player, 0x16)`). Checked (exe: the table entry and the cave's bytes; decomp: the
/// Field exe's switch `0x1401FA1B0` keeps the player in rbx and goes on at `0x1401FA599`).
pub mod field_case {
    /// The Field exe's jump table `0x1401FA5F4` (index = action − 1): action 0x16's entry.
    pub const TABLE_ENTRY_RVA: u32 = 0x001F_A648;
    /// int3 padding after a `pop r14; ret` (`0x1401FB7B1`), before `0x1401FB7C0`: 14 bytes. The cave
    /// the research named (`0x1401FA6C1`) holds the Attack's Battle case since M4
    /// (`crate::attack::battle_case`); this one was M9 9c.1's first Field case, unpatched since.
    pub const CAVE_RVA: u32 = 0x001F_B7B2;
    pub const CAVE_LEN: usize = 14;
    /// `exeStockSkill` (Sora's vtable `+0x140`; the Battle exe's case calls it through the vtable).
    pub const EXE_STOCK_RVA: u32 = 0x0021_8950;
    /// The Field exe's default (the timers after the action step).
    pub const DEFAULT_RVA: u32 = 0x001F_A599;
    pub const STUB_LEN: usize = 13;
    /// Re:CoM's sleight action.
    pub const ACTION: u32 = 0x16;

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

    /// The table entry's new value (an RVA, as the table holds).
    pub fn entry() -> [u8; 4] {
        CAVE_RVA.to_le_bytes()
    }
}

/// Re:CoM's two list pointers its sleight code reads (`crate::damage::BATTLE_CARD_LIST` /
/// `STOCK_SKILL_LIST`). The only writer is the `CCardBattle` ctor (`0x14015CE80` and its inline copy
/// in `0x1401607A0`): `0x1402A43E0(1, "BattleCardList", 0)` / `(1, "StockSkillList", 0)`, then
/// `0x1402A49C0(node)` (the node's data, `[[node+8]+8]`), 0 when not found. Readers: `initStockSkill`
/// (5), `exeStockSkill` (34), the summon actors' power pick `0x140226BB0` (both lists), the magic
/// sleights' shots (`0x140221DC0`, `0x1402203D0`, …); none tests them for null (Checked (decomp: every
/// reader; no `== 0` test)). Null until the session's first battle and kept (perhaps stale) after one:
/// a field sleight before any battle would read address 0 + offset. Our field start writes what
/// Re:CoM's own lookup finds, every time (a fresh value for a stale one too; the next battle's ctor
/// writes them again); not found: the sleight is refused (logged).
pub mod lists {
    pub const LOOKUP_RVA: u32 = 0x002A_43E0;
    pub const DATA_RVA: u32 = 0x002A_49C0;
    /// The resource group (SYS, `0x140AC6730[1]`) and the names, NUL-terminated, as the ctor's.
    pub const GROUP: u32 = 1;
    pub const BATTLE_CARD_NAME: &[u8] = b"BattleCardList\0";
    pub const STOCK_SKILL_NAME: &[u8] = b"StockSkillList\0";
}

/// Sleight packs in field rooms. Re:CoM's PLM (`PLM::CPlayerManager`, `player+0x3B8`, built for the
/// field by `CGameSubField`'s state 4 `0x1401AFDA0` and for battle by `0x1401AEBF0`) loads a pack
/// with `0x140211790` (slot state 1, the file queued) and its step `0x140210A10` marks it loaded (2)
/// only while `+0x88 & 1`. Its init `0x140210100` sets that bit and the ring fields only when
/// `0x14087B858 == 1` (in battle) or in a minigame room; the field leaves them 0, so a pack never
/// finishes and a pack sleight waits in `exeStockSkill` sub-state 4 for ever. The ring itself is
/// allocated in the field too (`CGameSubField` state 0 `0x1401AF810` → `0x140210BD0`), except in
/// world 10 (100 Acre Wood) and rooms 2000–2999 (`0x140235240`), before the PLM is built (state 4).
/// Checked (decomp + disasm).
///
/// The wrapper, after Re:CoM's init, for a field PLM ([`wanted`]): the battle branch's writes
/// (`0x1402102BE..0x140210407`): `+0x88 |= 1`, `+0x98` = the ring, `+0xA0` = ring + size, `+0xA8` =
/// size, and each of the 16 slots' `+8` = `+0xA0` (the init's tail copies `+0xA0` there). Not done:
/// the branch's `+0x58` = `0x1401607A0()` (it **builds a `CCardBattle`** when there is none: a card
/// battle's object; nothing of the PLM reads `+0x58`: Checked (decomp: its only access is the init's
/// write)) and `0x14087B6B0` = 1 (read only by `0x14020E6F0`, which then skips setting Sora's start
/// position from the room's map object: a battle room's start; Assumption from its reader).
pub mod packs {
    pub const PLM_INIT_RVA: u32 = 0x0021_0100;
    /// `*0x14087B6C0`: the ring (`0x140210BD0`'s `_aligned_malloc`; freed and zeroed by
    /// `0x140211610` at scene changes).
    pub const RING: u64 = 0x1_4087_B6C0;
    /// `*0x1407C7AE8` (u32): its size (2 / 2.5 MiB, less when memory is short).
    pub const RING_SIZE: u64 = 0x1_407C_7AE8;
    /// `0x1402339C0(1)`: the world; `0x140235240()`: a room 2000–2999.
    pub const WORLD_RVA: u32 = 0x0023_39C0;
    pub const ROOM_2000_RVA: u32 = 0x0023_5240;
    /// 100 Acre Wood: no ring (Checked (decomp `0x1401AF810`)).
    pub const NO_RING_WORLD: u32 = 10;

    /// PLM fields.
    pub const FLAGS: usize = 0x88;
    pub const LOADING: u32 = 1;
    pub const CURSOR: usize = 0x98;
    pub const END: usize = 0xA0;
    pub const SPACE: usize = 0xA8;
    pub const SLOTS: usize = 0xB0;
    pub const SLOT_LEN: usize = 0x18;
    pub const SLOT_PTR: usize = 8;
    pub const SLOT_COUNT: usize = 16;

    /// What the PLM init's caller sees of the room.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Room {
        /// Combat = KH1 in Sora's story (field sleights exist only there).
        pub kh1: bool,
        /// Re:CoM's init left `+0x88 & 1` clear (not battle, not a minigame room).
        pub loading_off: bool,
        pub world: u32,
        pub room_2000: bool,
        /// `*0x14087B6C0` > 0 (Re:CoM's own test, `(longlong)ring < 1` = none).
        pub ring: bool,
    }

    /// The field PLM gets the battle branch's writes: a field room at Combat = KH1 where the field
    /// allocated the ring (Re:CoM's own condition `0x1401AF841..0x1401AF859`, and the ring is there).
    pub fn wanted(r: &Room) -> bool {
        r.kh1 && r.loading_off && r.world != NO_RING_WORLD && !r.room_2000 && r.ring
    }

    /// The battle branch's ring fields: (`+0x98`, `+0xA0`, `+0xA8`).
    pub fn fields(ring: u64, size: u32) -> (u64, u64, u32) {
        (ring, ring + size as u64, size)
    }
}

/// Re:CoM's pack of a sleight id: `0x14020F720`'s switch (0 = none), Checked (decomp), for the ids
/// we start (5..=0x43; 0x44.. are Riku's and the friends').
pub fn pack(id: u16) -> u32 {
    match id {
        5 => 1,
        9 => 2,
        10 => 3,
        0xB => 4,
        0xC => 5,
        0x20 => 0xC,
        0x21 => 0xD,
        0x22 => 0xE,
        0x23 => 0xF,
        0x25 => 0x10,
        0x26 => 0x11,
        0x27 => 0x12,
        0x28 => 0x13,
        0x29 => 0x14,
        0x2A => 0x15,
        0x2D => 0x16,
        0x2E => 0x17,
        0x2F => 0x18,
        0x30 => 0x19,
        0x31 => 0x1A,
        0x33 => 0x1B,
        0x34 | 0x35 => 0x25,
        0x36 | 0x37 => 0x28,
        0x38..=0x3A => 0x27,
        0x3B | 0x3C => 0x2A,
        0x3D | 0x3E => 0x26,
        0x3F | 0x40 => 0x29,
        0x41..=0x43 => 0x2B,
        _ => 0,
    }
}

/// Sleight `id` needs a pack (it waits for it in `exeStockSkill`, or, a KH1 special, plays its
/// events from it).
pub fn needs_pack(id: u16) -> bool {
    pack(id) != 0
}

/// Sleight `id` may run in a field room (owner: "all sleights, the KH1 specials … and the LV3
/// summons"): the 27 sleight abilities and the listed KH1 specials. Not the friend sleights:
/// `exeStockSkill`'s friend branch (category 5) reads the active card `player+0x3E8` without a null
/// test (Checked (decomp)), which only a card battle has.
pub fn in_field(id: u16) -> bool {
    crate::abilities::of_sleight(id).is_some() || crate::sleights::special_index(id).is_some()
}

/// A start in this room is free (owner: "no MP for a sleight used outside battle").
pub fn free(g: &crate::summons::Gate) -> bool {
    g.field && !g.battle
}

/// The room lets sleight `id` run: battle, or a field room where it may run and (a pack one) its
/// pack loads.
pub fn room_ok(g: &crate::summons::Gate, id: u16) -> bool {
    g.battle || (g.field && in_field(id) && (g.packs || !needs_pack(id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::summons::Gate;

    #[test]
    fn the_stub_calls_exe_stock_skill_and_goes_on() {
        let s = field_case::stub();
        assert_eq!(&s[..4], &[0x48, 0x8B, 0xCB, 0xE8]);
        let call = i32::from_le_bytes(s[4..8].try_into().unwrap()) as i64;
        assert_eq!((field_case::CAVE_RVA as i64 + 8 + call) as u32, 0x0021_8950);
        assert_eq!(s[8], 0xE9);
        let jmp = i32::from_le_bytes(s[9..13].try_into().unwrap()) as i64;
        assert_eq!((field_case::CAVE_RVA as i64 + 13 + jmp) as u32, 0x001F_A599);
        assert!(field_case::STUB_LEN <= field_case::CAVE_LEN);
        assert_eq!(field_case::entry(), [0xB2, 0xB7, 0x1F, 0x00]);
        // The table's base + (0x16 − 1) × 4.
        assert_eq!(0x001F_A5F4 + (field_case::ACTION - 1) * 4, field_case::TABLE_ENTRY_RVA);
        // Clear of the other caves (the Attack's Battle case, magic's and items' Field cases).
        for (other, len) in [(crate::attack::battle_case::CAVE_RVA, 15u32), (crate::magic::field_case::CAVE_RVA, 15), (crate::items::field_case::CAVE_RVA, 14)] {
            assert!(field_case::CAVE_RVA + field_case::CAVE_LEN as u32 <= other || other + len <= field_case::CAVE_RVA);
        }
    }

    /// The research's split: 9 of the 27 need no pack; the other 18 (16 magic sleights, Idyll Romp,
    /// Cross-slash+) and every summon's LV3 do; Strike Raid none, Sonic Blade / Ars Arcanum /
    /// Ragnarok theirs (11009 / 11010 / 11011).
    #[test]
    fn packs_are_recoms() {
        let ids: Vec<u16> = crate::abilities::SLEIGHTS.filter_map(crate::abilities::sleight_of).collect();
        assert_eq!(ids.len(), 27);
        let none: Vec<&str> = ids.iter().filter(|&&id| !needs_pack(id)).map(|&id| crate::sleights::name(id)).collect();
        assert_eq!(none, ["Fire Raid", "Blizzard Raid", "Thunder Raid", "Reflect Raid", "Judgment", "Homing Blizzara", "Warpinator", "Warp", "Teleport"]);
        assert_eq!(ids.iter().filter(|&&id| needs_pack(id)).count(), 18);
        for s in &crate::summons::ALL {
            assert!(needs_pack(s.sleight), "{}", s.name);
        }
        use crate::abilities::sleight as sl;
        assert!(!needs_pack(sl::STRIKE_RAID));
        assert_eq!([pack(sl::SONIC_BLADE), pack(sl::ARS_ARCANUM), pack(sl::RAGNAROK)], [2, 3, 4]);
    }

    #[test]
    fn the_field_takes_the_27_and_the_specials_not_the_friends() {
        for id in crate::abilities::SLEIGHTS.filter_map(crate::abilities::sleight_of) {
            assert!(in_field(id), "{id:#x}");
        }
        for i in crate::sleights::SPECIALS {
            assert!(in_field(crate::sleights::special_sleight(i).unwrap()));
        }
        // Re:CoM's friend sleights (0x44..) and Riku's: no.
        for id in [0x44u16, 0x4C, 0x60, 0x2, 0x0] {
            assert!(!in_field(id), "{id:#x}");
        }
    }

    #[test]
    fn rooms_and_mp() {
        let battle = Gate { battle: true, ground: true, mp: 0, ..Default::default() };
        let field = Gate { field: true, ground: true, mp: 0, ..Default::default() };
        let no_packs = Gate { packs: false, ..field };
        let field = Gate { packs: true, ..field };
        use crate::abilities::sleight as sl;
        assert!(room_ok(&battle, sl::HOLY) && room_ok(&battle, 0x44), "battle: Re:CoM's rules as before");
        assert!(room_ok(&field, sl::HOLY) && room_ok(&field, sl::FIRE_RAID));
        assert!(!room_ok(&no_packs, sl::HOLY), "100 Acre Wood / rooms 2000–2999: pack sleights greyed");
        assert!(room_ok(&no_packs, sl::FIRE_RAID), "the 9 pack-less ones run there");
        assert!(!room_ok(&no_packs, sl::SONIC_BLADE) && room_ok(&no_packs, sl::STRIKE_RAID));
        assert!(!room_ok(&field, 0x44), "friend sleights never in the field");
        assert!(!room_ok(&Gate::default(), sl::FIRE_RAID), "neither room (Combat = Re:CoM, not installed)");
        assert!(free(&field) && !free(&battle));
    }

    #[test]
    fn the_field_plm_gets_the_battle_branch_writes() {
        let r = packs::Room { kh1: true, loading_off: true, world: 3, room_2000: false, ring: true };
        assert!(packs::wanted(&r));
        assert!(!packs::wanted(&packs::Room { kh1: false, ..r }), "Combat = Re:CoM: Re:CoM's field PLM");
        assert!(!packs::wanted(&packs::Room { loading_off: false, ..r }), "battle / minigame: Re:CoM already did it");
        assert!(!packs::wanted(&packs::Room { world: 10, ..r }), "100 Acre Wood");
        assert!(!packs::wanted(&packs::Room { room_2000: true, ..r }));
        assert!(!packs::wanted(&packs::Room { ring: false, ..r }));
        assert_eq!(packs::fields(0x1000_0000, 0x20_0000), (0x1000_0000, 0x1020_0000, 0x20_0000));
        assert_eq!(packs::SLOTS + packs::SLOT_LEN * packs::SLOT_COUNT, 0x230, "the 16 slots end where +0x230 starts");
    }
}
