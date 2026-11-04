//! KH1's items (M8 stage 8a.1): the Items list, Sora's item slots and stock, and the use, inside
//! Re:CoM's own item action `0xE`. The testable part; the Re:CoM side is `win/items.rs`, the notes
//! `docs/recom-m8-items.md`.
//!
//! KH1 (Steam; Checked (decomp + exe + files) unless tagged):
//!
//! - **Items** are `btltbl.bin` rows (file `0x1A58`, 0x14 bytes, lookup `0x14028F970`): `+0` the
//!   name's message id, `+2` kind (0 = usable in battle), `+4` bit 0 party, `+6` amount, `+8` buy,
//!   `+0xA` sell ([`DATA`]). The effect (`0x1402CD780`): Potion / Hi-Potion / Mega-Potion add HP
//!   (`0x1402CD5E0`, only below max), Ether / Mega-Ether add MP points (`0x1402CD6B0`, only below
//!   max, through `0x1402A4A00`), Elixir / Megalixir add 9999 of both (full; the MP charge is not
//!   touched).
//! - **Slots and stock**: Sora's party record (`save + 4`) `+0x21` slot count (save `+0x25`),
//!   `+0x22..` the slots' item ids (save `+0x26..`); one stock byte per item (`save + 0x499 + id`,
//!   at most 99). A new game (`0x1402852C0`) gives Sora **3 slots, all empty**, and no stock
//!   (btltbl `0x39D8` record 0's slot byte = 3; `+0x499..` zeroed). A pickup or a returned item
//!   (`0x140284E60`) fills the first empty slot when the item is a battle item, else (or with the
//!   slots full) adds to the stock (below 99), else is refused. Using an item clears its slot
//!   (`0x1402C2760`); **nothing refills a slot from the stock** on use.
//! - **The command menu** (`0x140294220`): root row 2 is Items; its list (type 2, `0x140528970`)
//!   holds the filled slots in slot order (command `item + 0x1E`; `0x142D53A90[k]` = the slot);
//!   no filled slot: row 2 greyed and locked; a state without the item flag `0x40`: row 2 greyed
//!   (not locked) and every entry greyed and locked. KH1 doesn't grey an entry for full HP.
//! - **The use** (`0x1402A3600` type 3 → ground `0x1402A3DB0`, states 0 / 6 / 0x18; the air drops
//!   it): motion `0x3E` (on self; `0x3F` on another) with blend 8, the idle queued (release, blend
//!   8); then `0x1402C2760` takes the item and starts the use task `0x1402C2880` (task list
//!   `0x142DB34B0`, run at priority 139999, after the actors). See [`UseTask`].
//!
//! Re:CoM's side: the use is KH1's frame on Re:CoM's item action `0xE` (as a cast is on `0x13`):
//! `Host::start_item` writes `initItem`'s (`0x1401F28F0`) fields cardlessly, Re:CoM plays its own
//! item clip (row 0x2D, `p001_032`: KH1 entry 58's bake), and the use task's heal is Re:CoM's
//! heal `0x1401E59A0` with Re:CoM's own item effect.

use crate::movement::{Action, Host, Kh1Sora};
use kh1_core::controller::state;
use kh1_core::motion::{req, request, FlagKind, MotionInfo, MotionSet};
use kh1_core::Vec3;

/// KH1's items the mod has (item ids = KH1's: `btltbl.bin` item rows `0x1A58 + (id − 1) × 0x14`):
/// the battle items 1..8 (5 is an empty row) and (M8c stage 2) the Tent and the Camping Set.
pub mod id {
    pub const POTION: u8 = 1;
    pub const HI_POTION: u8 = 2;
    pub const ETHER: u8 = 3;
    pub const ELIXIR: u8 = 4;
    pub const MEGA_POTION: u8 = 6;
    pub const MEGA_ETHER: u8 = 7;
    pub const MEGALIXIR: u8 = 8;
    /// KH1's row 142 (kind 12: a menu item, never in a slot). Checked (files).
    pub const TENT: u8 = 142;
    /// KH1's row 143 (kind 12). Checked (files).
    pub const CAMPING_SET: u8 = 143;
    /// The battle items, in KH1's order.
    pub const ALL: [u8; 7] = [POTION, HI_POTION, ETHER, ELIXIR, MEGA_POTION, MEGA_ETHER, MEGALIXIR];
    /// The camp items (M8c stage 2), in KH1's order.
    pub const CAMP: [u8; 2] = [TENT, CAMPING_SET];
    /// Every item the store can hold, in KH1's order (the Sell list's order).
    pub const HELD: [u8; 9] = [POTION, HI_POTION, ETHER, ELIXIR, MEGA_POTION, MEGA_ETHER, MEGALIXIR, TENT, CAMPING_SET];
    /// Removed at Combat = KH1 (owner, 2026-11-04: "Lets remove mega potions, mega ethers, and mega
    /// elixirs"; M8c stage 3): never dropped, sold or handed out; a save's are converted once
    /// ([`super::convert_removed`]).
    pub const REMOVED: [u8; 3] = [MEGA_POTION, MEGA_ETHER, MEGALIXIR];
    /// The items in play at Combat = KH1: [`HELD`] without [`REMOVED`].
    pub const IN_PLAY: [u8; 6] = [POTION, HI_POTION, ETHER, ELIXIR, TENT, CAMPING_SET];
}

/// Whether an item is one of [`id::REMOVED`] (the Mega items).
pub fn removed(item: u8) -> bool {
    id::REMOVED.contains(&item)
}

/// What a removed item becomes in a save that holds it (M8c stage 3): Mega-Potion → Hi-Potion,
/// Mega-Ether → Ether, Megalixir → Elixir.
pub fn replacement(item: u8) -> Option<u8> {
    Some(match item {
        id::MEGA_POTION => id::HI_POTION,
        id::MEGA_ETHER => id::ETHER,
        id::MEGALIXIR => id::ELIXIR,
        _ => return None,
    })
}

/// One KH1 item row (`btltbl.bin` `0x1A58 + (id − 1) × 0x14`). Checked (files: the owner's
/// `btltbl.bin`, read with `kh1_mset.py`-style tooling; the fields as `0x1402CD3A0`,
/// `0x1402CD780`, `0x140284E60` read them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemData {
    pub id: u8,
    /// `+0`: the name's message id (also the command id's message: command `id + 0x1E`).
    pub name: u16,
    /// `+2`: 0 = a battle item (slots; the menu's Items list); 12 = a menu item ([`KIND_MENU`]).
    pub kind: u8,
    /// `+4` bit 0: the whole party (Mega-Potion, Mega-Ether, Megalixir, Tent, Camping Set).
    pub party: bool,
    /// `+6`: HP or MP points (Elixirs 255; their effect adds 9999).
    pub amount: i16,
    /// `+8` / `+0xA`: munny (the shop, stage 8a.3).
    pub buy: u16,
    pub sell: u16,
    /// `+0x10`: the sort key the pause menu's stock lists sort by (`0x14030EEB0`, then the id).
    pub sort: u16,
}

/// KH1's item rows: the battle items, then (M8c stage 2) the Tent and the Camping Set (`+2` kind
/// 12, `+4` party, `+6` 255 / 3, `+8` 250 / 500, `+0xA` 50 / 100, `+0x10` 101 / 102). Checked
/// (files: the owner's `btltbl.bin` rows 142 / 143 at `0x255C` / `0x2570`).
pub const DATA: [ItemData; 9] = [
    ItemData { id: id::POTION, name: 0x1F, kind: 0, party: false, amount: 30, buy: 60, sell: 12, sort: 1 },
    ItemData { id: id::HI_POTION, name: 0x20, kind: 0, party: false, amount: 60, buy: 150, sell: 30, sort: 2 },
    ItemData { id: id::ETHER, name: 0x21, kind: 0, party: false, amount: 3, buy: 200, sell: 40, sort: 3 },
    ItemData { id: id::ELIXIR, name: 0x22, kind: 0, party: false, amount: 255, buy: 0, sell: 80, sort: 4 },
    ItemData { id: id::MEGA_POTION, name: 0x24, kind: 0, party: true, amount: 30, buy: 0, sell: 40, sort: 6 },
    ItemData { id: id::MEGA_ETHER, name: 0x25, kind: 0, party: true, amount: 3, buy: 0, sell: 60, sort: 7 },
    ItemData { id: id::MEGALIXIR, name: 0x26, kind: 0, party: true, amount: 255, buy: 0, sell: 100, sort: 8 },
    ItemData { id: id::TENT, name: 172, kind: KIND_MENU, party: true, amount: 255, buy: 250, sell: 50, sort: 101 },
    ItemData { id: id::CAMPING_SET, name: 173, kind: KIND_MENU, party: true, amount: 3, buy: 500, sell: 100, sort: 102 },
];

/// KH1's item kind 12: an item used from the pause menu only (the Tents; `0x1402CD3A0` accepts
/// kinds 0 and 12, the slots and the battle menu kind 0 only). Checked (decomp + files).
pub const KIND_MENU: u8 = 12;

pub fn data(item: u8) -> Option<&'static ItemData> {
    DATA.iter().find(|d| d.id == item)
}

/// A battle item (row kind 0): it goes into a slot (`0x140284E60`) and the Items list.
pub fn is_battle_item(item: u8) -> bool {
    data(item).is_some_and(|d| d.kind == 0)
}

/// KH1's command for an item (`item + 0x1E`, the command table's type-3 records 0x1F..0x26:
/// `+6` = 3, `+7` = the item, `+8` = 4 (the party list; 6 for the Mega items), `+0` bit 1 = face
/// the target). Its message id is the same number (Potion 31 … Megalixir 38). Checked (files).
pub const fn command(item: u8) -> u8 {
    item + 0x1E
}

/// The item of a command (0x1F..=0x26 with a battle item's row; the camp items have no command).
pub fn item_of_command(c: u8) -> Option<u8> {
    c.checked_sub(0x1E).filter(|&i| is_battle_item(i))
}

/// The names for the log (KH1's English words).
pub fn name(item: u8) -> &'static str {
    match item {
        id::POTION => "potion",
        id::HI_POTION => "hi-potion",
        id::ETHER => "ether",
        id::ELIXIR => "elixir",
        id::MEGA_POTION => "mega-potion",
        id::MEGA_ETHER => "mega-ether",
        id::MEGALIXIR => "megalixir",
        id::TENT => "tent",
        id::CAMPING_SET => "camping set",
        _ => "?",
    }
}

// ---- What an item does ----

/// The HP part of an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HpHeal {
    /// This many of Re:CoM's HP.
    Amount(i32),
    /// To full (KH1's 9999).
    Full,
}

/// What an item restores: HP (Re:CoM's HP) and MP (KH1's MP points; [`MP_FULL`] = to max).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemEffect {
    pub hp: Option<HpHeal>,
    pub mp: Option<i32>,
}

/// KH1's Elixirs add 9999 MP points (`0x1402CD780` cases 4 / 8 / 0x90): to max through
/// `0x1402A4A00`'s clamp, the charge untouched.
pub const MP_FULL: i32 = 9999;

/// Re:CoM HP per KH1 HP: Re:CoM's HP cap 560 (`0x230`) over KH1's 120 (`0x78`), the ratio the KH1
/// HUD already shows Re:CoM's HP with (Stage 73, Checked (decomp)).
pub const RECOM_HP_PER_KH1: (i32, i32) = (560, 120);

/// **Provisional, owner question 1.** The owner chose "healing amounts: Re:CoM's own item-card
/// amounts", but Re:CoM's item cards heal no HP: they reload cards (Checked (decomp): `exeItem`
/// `0x1401ED6E0` → `0x14016C910` → `0x14016A450`; `BattleCardList` 0x2A..0x30 hold power 0 and
/// only their CP cost). Until the owner picks, the HP items heal KH1's amount on Re:CoM's HP scale
/// (× 560 / 120: Potion and Mega-Potion 140, Hi-Potion 280), the Elixirs to full.
pub fn recom_hp(kh1_hp: i32) -> i32 {
    kh1_hp * RECOM_HP_PER_KH1.0 / RECOM_HP_PER_KH1.1
}

/// KH1's Camping Set (`0x1402CD780` case 0x8F): HP + 9999 and MP + its row's amount, **3 MP
/// points** (not full; Checked (decomp + files)). The owner's design (2026-11-04, § KH1 Moogle
/// shop: "Camping Set = full HP and MP", "Camp-only full heal") restores MP to full: this is that
/// choice (owner question: KH1's 3 points instead).
pub const CAMPING_SET_FULL_MP: bool = true;

/// The item's effect (`0x1402CD780`): HP items KH1's amount on Re:CoM's scale ([`recom_hp`],
/// provisional), Ethers KH1's 3 MP points (Re:CoM has no MP: KH1's MP is ours), Elixirs both to
/// full. M8c stage 2: the Tent HP to full (KH1's case 0x8E: + 9999), the Camping Set HP to full
/// and MP to full ([`CAMPING_SET_FULL_MP`]; KH1's row amount 3 when off).
pub fn effect(item: u8) -> Option<ItemEffect> {
    let d = data(item)?;
    Some(match item {
        id::POTION | id::HI_POTION | id::MEGA_POTION => ItemEffect { hp: Some(HpHeal::Amount(recom_hp(d.amount as i32))), mp: None },
        id::ETHER | id::MEGA_ETHER => ItemEffect { hp: None, mp: Some(d.amount as i32) },
        id::ELIXIR | id::MEGALIXIR => ItemEffect { hp: Some(HpHeal::Full), mp: Some(MP_FULL) },
        id::TENT => ItemEffect { hp: Some(HpHeal::Full), mp: None },
        id::CAMPING_SET => ItemEffect { hp: Some(HpHeal::Full), mp: Some(if CAMPING_SET_FULL_MP { MP_FULL } else { d.amount as i32 }) },
        _ => return None,
    })
}

/// Where an item can be used: a battle item (kind 0) from the command menu (in battle and the
/// field) and the camp Items page; a camp item (kind 12: the Tents) from the camp Items page only
/// (KH1: the pause menu; owner, 2026-11-04: "Camp-only full heal"). The camp opens only in the
/// field (Re:CoM's own rule), so a camp item is never used in battle.
pub fn camp_only(item: u8) -> bool {
    data(item).is_some_and(|d| d.kind == KIND_MENU)
}

/// KH1's heal step for HP (`0x1402CD5E0` on a live actor): only below max. The amount Re:CoM's
/// heal is given (it clamps itself), or None (no heal).
pub fn hp_to_add(heal: HpHeal, hp: i32, max: i32) -> Option<i32> {
    if hp >= max {
        return None;
    }
    Some(match heal {
        HpHeal::Amount(a) => a,
        HpHeal::Full => max,
    })
}

/// Which of Re:CoM's effect sets an item effect is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectSet {
    /// `always.dpx` (`0x1407C5DD8` points at its name).
    Always,
    /// `itemplus.dpx` (`0x1407C5DE0`).
    ItemPlus,
}

/// Re:CoM's own effect for an item card (`exeItem` `0x1401ED6E0`'s switch on the card,
/// `0x1401ED853..`): Potion always 3, Hi-Potion 4, Ether 5, Elixir 6, Mega-Potion itemplus 0,
/// Mega-Ether 1, Megalixir 2; on the player's bone 0 at (0, 0.85, 0), kept on him. Checked
/// (decomp). KH1 plays its own recovery effect at the heal (`0x1402C2B50`, ids `0x14052A0A7[item]`:
/// 3, 4, 5, 6, then 4, 5, 6 for the Mega items: Checked (exe)): the four base items' ids are the
/// same numbers in Re:CoM's `always.dpx` (whether the effects look alike: Assumption).
pub fn recom_effect(item: u8) -> Option<(EffectSet, u32)> {
    Some(match item {
        id::POTION => (EffectSet::Always, 3),
        id::HI_POTION => (EffectSet::Always, 4),
        id::ETHER => (EffectSet::Always, 5),
        id::ELIXIR => (EffectSet::Always, 6),
        id::MEGA_POTION => (EffectSet::ItemPlus, 0),
        id::MEGA_ETHER => (EffectSet::ItemPlus, 1),
        id::MEGALIXIR => (EffectSet::ItemPlus, 2),
        _ => return None,
    })
}

/// KH1's recovery effect ids (`0x14052A0A7[item]`). Checked (exe).
pub const KH1_EFFECT_IDS: [u8; 9] = [0, 3, 4, 5, 6, 7, 4, 5, 6];

// ---- Slots and stock ----

/// Slots a party record can hold (`+0x22..` before the next field). KH1's growth: + 1 at Sora's
/// levels 18, 30, 42 (kind 6 in btltbl `0x3AC0`) and 54 (each Dive path's table `0x3B28` /
/// `0x3B90` / `0x3BF8`), + 1 at level 3 on one path (`0x3B90`). Checked (files + decomp
/// `0x1402B5660` case 6). The growth is not built yet (deferred: `docs/recom-m8-items.md`).
pub const MAX_SLOTS: usize = 8;
/// A new game's slot count (`0x1402852C0`: btltbl `0x39D8` record 0's byte `0x16`). Checked
/// (files + decomp).
pub const START_SLOTS: u8 = 3;
/// KH1's stock cap (`0x140284E60`: below 99 → + 1).
pub const MAX_STOCK: u8 = 99;
/// Stock bytes kept. Bytes 0..=7: items 1..=8 (`[id − 1]`, the battle items; byte 4 is KH1's empty
/// row 5); M8c stage 2: byte 8 the Tent (KH1 item 142), byte 9 the Camping Set (143); bytes 10..=15
/// spare. Bytes 8..=15 were "items 9..=16" before 0.105.0, which nothing could store (the pickup
/// was only ever given built items: the shop's wares, the battle-item drops, a returned use;
/// Checked (code)), so every save has them at 0.
pub const STOCK_IDS: usize = 16;

/// The stock byte of an item (see [`STOCK_IDS`]).
pub fn stock_index(item: u8) -> Option<usize> {
    match item {
        1..=8 => Some(item as usize - 1),
        id::TENT => Some(8),
        id::CAMPING_SET => Some(9),
        _ => None,
    }
}

/// Where a picked-up (or returned) item went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Picked {
    /// Into slot k.
    Slot(u8),
    /// Into the stock, which now holds this many.
    Stock(u8),
}

/// Sora's items: KH1's slot count and slots (party record `+0x21`, `+0x22..`) and the stock
/// (`save + 0x499 + id`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inventory {
    pub slot_count: u8,
    /// Item ids, 0 = empty.
    pub slots: [u8; MAX_SLOTS],
    /// Stock bytes, by [`stock_index`].
    pub stock: [u8; STOCK_IDS],
}

impl Default for Inventory {
    fn default() -> Self {
        Inventory::new_game()
    }
}

impl Inventory {
    /// KH1's new game (`0x1402852C0`): 3 empty slots, no stock.
    pub fn new_game() -> Inventory {
        Inventory { slot_count: START_SLOTS, slots: [0; MAX_SLOTS], stock: [0; STOCK_IDS] }
    }

    fn count(&self) -> usize {
        (self.slot_count as usize).min(MAX_SLOTS)
    }

    /// The filled slots in slot order: (slot, item). KH1's Items list (`0x140294220`).
    pub fn filled(&self) -> Vec<(u8, u8)> {
        (0..self.count()).filter(|&k| self.slots[k] != 0).map(|k| (k as u8, self.slots[k])).collect()
    }

    /// The stock of an item.
    pub fn stock_of(&self, item: u8) -> u8 {
        stock_index(item).map_or(0, |i| self.stock[i])
    }

    /// The stock byte of an item (None: the store has no byte for it).
    pub fn stock_mut(&mut self, item: u8) -> Option<&mut u8> {
        stock_index(item).map(|i| &mut self.stock[i])
    }

    /// Sets an item's stock (no byte: nothing).
    pub fn set_stock(&mut self, item: u8, n: u8) {
        if let Some(s) = self.stock_mut(item) {
            *s = n;
        }
    }

    /// `0x1402C2760`: the use takes the item out of its slot (Sora: `save + 0x26 + slot` = 0). None
    /// when that slot doesn't hold `item`.
    pub fn take(&mut self, slot: u8, item: u8) -> Option<u8> {
        let k = slot as usize;
        if k >= self.count() || self.slots[k] != item || item == 0 {
            return None;
        }
        self.slots[k] = 0;
        Some(item)
    }

    /// `0x140284E60(item, member)`: a battle item fills the first empty slot; otherwise (or with
    /// every slot full) the stock, while below 99; else refused (None). KH1 returns an unused item
    /// the same way (the use task, `0x1402C2880`). The shop (8a.3) and drops (8a.4) use it.
    pub fn pick_up(&mut self, item: u8) -> Option<Picked> {
        if item == 0 {
            return None;
        }
        if is_battle_item(item) {
            if let Some(k) = (0..self.count()).find(|&k| self.slots[k] == 0) {
                self.slots[k] = item;
                return Some(Picked::Slot(k as u8));
            }
        }
        let s = self.stock_mut(item)?;
        if *s >= MAX_STOCK {
            return None;
        }
        *s += 1;
        Some(Picked::Stock(*s))
    }
}

// ---- The save: our block's layout v2 (the items extension) ----

/// The items extension of our save block: right after the MP block (`crate::mp::BLOCK_LAST`),
/// inside Re:CoM's spare tail `save + 0x246B ..= 0x2817` that no Re:CoM code reads or writes
/// (`docs/recom-keyblade-equip.md`, `docs/recom-m7-mp.md`). Our block's layout v2 = layout v1's 16
/// bytes + this extension.
pub const EXT_FIRST: usize = crate::mp::BLOCK_LAST + 1;
pub const EXT_LEN: usize = 26;
pub const EXT_LAST: usize = EXT_FIRST + EXT_LEN - 1;

/// Offsets inside the extension. Fields never move.
pub mod ext {
    /// 0 = never written (a layout v1 save, or a new game): KH1's new-game inventory; 1 = written;
    /// 2 = written with the Mega items converted (0.105.0, [`super::convert_removed`]).
    pub const VERSION: usize = 0;
    /// KH1's slot count (party record `+0x21`).
    pub const SLOT_COUNT: usize = 1;
    /// 8 slots (party record `+0x22..`), item ids, 0 = empty.
    pub const SLOTS: usize = 2;
    /// 16 stock bytes ([`super::stock_index`]: items 1..=8, the Tent, the Camping Set; KH1's
    /// `save + 0x499 + id`).
    pub const STOCK: usize = SLOTS + super::MAX_SLOTS;
}

pub const EXT_V1: u8 = 1;
/// M8c stage 3: the Mega items converted ([`convert_removed`]); what every write now sets.
pub const EXT_V2: u8 = 2;

const _: () = assert!(EXT_LAST <= crate::keyblades::SPARE_LAST && ext::STOCK + STOCK_IDS == EXT_LEN);

/// The inventory a block holds. An extension never written (a layout v1 save: the migration) is
/// KH1's new game: 3 empty slots, no stock.
pub fn read(block: &[u8; EXT_LEN]) -> Inventory {
    if block[ext::VERSION] == 0 {
        return Inventory::new_game();
    }
    let mut inv = Inventory { slot_count: block[ext::SLOT_COUNT].min(MAX_SLOTS as u8), ..Inventory::new_game() };
    inv.slots.copy_from_slice(&block[ext::SLOTS..ext::SLOTS + MAX_SLOTS]);
    inv.stock.copy_from_slice(&block[ext::STOCK..ext::STOCK + STOCK_IDS]);
    for s in inv.stock.iter_mut() {
        *s = (*s).min(MAX_STOCK);
    }
    inv
}

/// Writes the inventory and the extension's version (never lowered). Every write is v2: the
/// caller converts a v1 block first ([`convert_removed`]), and nothing adds a Mega item after.
pub fn write(inv: &Inventory, block: &mut [u8; EXT_LEN]) {
    block[ext::VERSION] = block[ext::VERSION].max(EXT_V2);
    block[ext::SLOT_COUNT] = inv.slot_count;
    block[ext::SLOTS..ext::SLOTS + MAX_SLOTS].copy_from_slice(&inv.slots);
    block[ext::STOCK..ext::STOCK + STOCK_IDS].copy_from_slice(&inv.stock);
}

/// What [`convert_removed`] did (for the log).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Converted {
    /// Slots whose Mega item became its replacement.
    pub slots: u8,
    /// Mega items moved from the stock to their replacement's stock.
    pub stock: u32,
    /// Mega items past the replacement's 99 (lost).
    pub lost: u32,
}

/// M8c stage 3, once per save (a v1 block; Combat = KH1 in Sora's story, the only time the store
/// is read): every Mega item becomes its [`replacement`], in its slot (a battle item for a battle
/// item) or in the stock (added to the replacement's, up to 99; the rest is lost, logged), and the
/// block becomes v2. A v0 block (never written: KH1's new game) and a v2 block are left alone.
/// None when nothing was to do.
///
/// Chosen over hiding them (keeping them sellable): a hidden Mega item in a slot would hold that
/// slot while the page and the command menu showed it empty (the pickup skips a filled slot), and
/// the page, the command menu and the shop would each need their own filter. The conversion is
/// one rule at one place, and the Items page, the command menu and the shop never meet a Mega item.
pub fn convert_removed(block: &mut [u8; EXT_LEN]) -> Option<Converted> {
    if block[ext::VERSION] != EXT_V1 {
        return None;
    }
    let mut inv = read(block);
    let mut c = Converted::default();
    let n = (inv.slot_count as usize).min(MAX_SLOTS);
    for s in inv.slots[..n].iter_mut() {
        if let Some(r) = replacement(*s) {
            *s = r;
            c.slots += 1;
        }
    }
    for m in id::REMOVED {
        let have = inv.stock_of(m);
        if have == 0 {
            continue;
        }
        let r = replacement(m).expect("a removed item has a replacement");
        let room = MAX_STOCK - inv.stock_of(r).min(MAX_STOCK);
        let moved = have.min(room);
        inv.set_stock(r, inv.stock_of(r) + moved);
        inv.set_stock(m, 0);
        c.stock += moved as u32;
        c.lost += (have - moved) as u32;
    }
    write(&inv, block);
    Some(c)
}

// ---- The debug test inventory (`kh1_test_items`) ----

/// The slots the test inventory fills, in order.
pub const TEST_SLOTS: [u8; 3] = [id::POTION, id::HI_POTION, id::ETHER];
/// The stock the test inventory keeps of each item in play (at least).
pub const TEST_STOCK: u8 = 9;

/// Debug (`kh1_test_items = on`, until the shop and drops exist): when every slot is empty, fill
/// them with Potion, Hi-Potion, Ether (as many as there are slots), and keep at least 9 of each
/// item in play ([`id::IN_PLAY`]: the battle items and the Tents; never a Mega item, M8c stage 3)
/// in stock. Not KH1's: KH1 never refills a slot. True when it changed something.
pub fn test_inventory(inv: &mut Inventory) -> bool {
    let mut changed = false;
    if inv.filled().is_empty() {
        for (k, &item) in TEST_SLOTS.iter().enumerate().take(inv.count()) {
            inv.slots[k] = item;
            changed = true;
        }
    }
    for item in id::IN_PLAY {
        if inv.stock_of(item) < TEST_STOCK {
            inv.set_stock(item, TEST_STOCK);
            changed = true;
        }
    }
    changed
}

// ---- The use task (`0x1402C2880`) ----

/// The task's first wait on self (btltbl G `+0x40 + member × 2`, G = btltbl + 0x58: Sora's byte
/// `0x98` = 34; on another member `0x99` = 36). Checked (files + decomp `0x1402C2760`).
pub const SELF_DELAY: f32 = 34.0;
pub const OTHER_DELAY: f32 = 36.0;
/// The wait from the hand effect to the heal (`0x41F00000`). Checked (decomp).
pub const HEAL_DELAY: f32 = 30.0;

/// What a run of the task did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStep {
    Wait,
    /// The first wait ran out: KH1 spawns `always.dpx` effect 1 at the hand (bone 0x1C) when the
    /// motion is still the one the task started with, then waits 30 more.
    Hand { same_motion: bool },
    /// The second wait ran out: the heal (`0x1402C2B50`), the task ends.
    Heal,
    /// Sora is not alive (or an event took him: `+0x444` bit 0): the item goes back through
    /// `0x140284E60`, the task ends.
    Return,
}

/// KH1's use task (`0x1402C2880`, started by `0x1402C2760`): it runs once a frame after the actors
/// (priority 139999), independent of Sora's motion: a hit during the use doesn't stop it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UseTask {
    pub item: u8,
    pub slot: u8,
    /// The float at `task + 0x2C`.
    pub delay: f32,
    /// `*piVar2`: 0 waiting for the hand, 1 waiting for the heal.
    pub state: u8,
    /// The motion playing at the first run (`piVar2[0x10]` = actor `+0x164`); Re:CoM's row here.
    pub start_motion: Option<i32>,
    /// Frames since the command at the latest run (the first run, in the command's frame, is 0;
    /// log and trace).
    pub age: f32,
    pub done: bool,
}

impl UseTask {
    /// On Sora himself (the only target: no party yet).
    pub fn new(item: u8, slot: u8) -> UseTask {
        UseTask { item, slot, delay: SELF_DELAY, state: 0, start_motion: None, age: 0.0, done: false }
    }

    /// One run for a frame of `step`: `alive` = KH1's `0x140291FF0` on Sora; `stopped` = KH1's
    /// `0x140297950` (`+0x444` bit 0, set by the actor message 0xE); `motion` = the motion playing
    /// now. KH1 tests the waits with `<= 0 && != 0`, i.e. strictly below 0. Checked (decomp).
    pub fn run(&mut self, step: f32, alive: bool, stopped: bool, motion: i32) -> TaskStep {
        if self.done {
            return TaskStep::Wait;
        }
        if self.start_motion.is_none() {
            self.start_motion = Some(motion);
        } else {
            self.age += step;
        }
        self.delay -= step;
        if !alive || stopped {
            self.done = true;
            return TaskStep::Return;
        }
        match self.state {
            0 => {
                if self.delay < 0.0 {
                    self.state = 1;
                    self.delay = HEAL_DELAY;
                    return TaskStep::Hand { same_motion: self.start_motion == Some(motion) };
                }
                TaskStep::Wait
            }
            _ => {
                if self.delay < 0.0 {
                    self.done = true;
                    return TaskStep::Heal;
                }
                TaskStep::Wait
            }
        }
    }
}

// ---- KH1's item command in the handler frame ----

/// KH1's use motions: on self `0x3E` (entry 58, 82 frames, free from 74, no root travel), on
/// another `0x3F` (entry 59, 72, free from 62). Checked (files: `kh1_mset.py` on
/// `xa_ex_0010.mset`). Re:CoM's own item clip `p001_032` (row 0x2D, what `exeItem` plays) is entry
/// 58's bake (`kh1-recom-assets` validate test `BAKES`: slot 32 ↔ entry 58). Checked (files).
pub const USE_SELF: u16 = 0x3E;
pub const USE_OTHER: u16 = 0x3F;
pub const USE_FRAMES: f32 = 82.0;
pub const USE_FREE: f32 = 74.0;
/// `0x1402A3DB0` case 3: `0x14029F980(actor, 0x100003E, 8.0)`, `0x14029FA30(actor, 0x40000000,
/// 8.0)`. Checked (decomp).
pub const USE_BLEND: f32 = 8.0;

/// Adds KH1's use motion to the motion set.
pub fn add_motions(m: &mut MotionSet) {
    m.add(&[USE_SELF], MotionInfo::new(USE_FRAMES).with_flag(FlagKind::Free, USE_FREE, f32::INFINITY));
}

/// Re:CoM's item action (`initItem` `0x1401F28F0`: `+0x40C` = 0xE), the sub `exeItem` plays the
/// item clip in (2), and `initItem`'s phase (`+0x604` = 3). Checked (decomp).
pub const RECOM_ITEM_ACTION: u32 = 0xE;
pub const ITEM_SUB: u32 = 2;
pub const ITEM_PHASE: u32 = 3;

/// A KH1 item command (the menu's): the item and the slot it is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemCommand {
    pub item: u8,
    pub slot: u8,
}

/// What the KH1 frame did with an item command (log and trace).
#[derive(Clone, Debug, PartialEq)]
pub enum ItemEvent {
    Started { item: u8, slot: u8 },
    Refused { item: u8, why: &'static str },
}

impl ItemEvent {
    pub fn token(&self) -> String {
        match self {
            ItemEvent::Started { item, slot } => format!("use({} slot {})", name(*item), slot + 1),
            ItemEvent::Refused { item, why } => format!("use-refused({} {why})", name(*item)),
        }
    }
}

impl Kh1Sora {
    /// `0x140293480` for an item: not in the take-off; free or in a combo window (`0x1402A2EC0`).
    /// Checked (decomp: type 3 has no exception, unlike Fire's re-cast).
    pub fn accepts_item(&self) -> bool {
        self.state != state::TAKE_OFF && (!self.lock || self.in_window())
    }

    /// Re:CoM's action while the use motion plays (its item action, as `initItem` sets it).
    pub(crate) fn item_action(&self) -> Option<Action> {
        self.item.as_ref()?;
        (self.rec.head() == USE_SELF).then_some(Action { id: RECOM_ITEM_ACTION, sub: ITEM_SUB, phase: ITEM_PHASE })
    }

    /// `0x1402A3600` type 3 → `0x1402A3DB0` (states 0 / 6 / 0x18; others drop it). The command's
    /// target is Sora (no party list yet: the owner's choice for Cure / Aero, applied to items),
    /// so `0x1402A38E0` → `0x1402A3B00` keeps the facing and stops him.
    pub(crate) fn command_item(&mut self, h: &mut dyn Host, dir: &mut Vec3, w: &mut f32, c: ItemCommand) {
        let refuse = |me: &mut Kh1Sora, why| me.item_log.push(ItemEvent::Refused { item: c.item, why });
        if self.state != state::GROUND {
            return refuse(self, "state");
        }
        *dir = Vec3::from_yaw(self.facing);
        self.target_yaw = self.facing;
        *w = 0.0;
        // A swing playing (a use from a combo window): Re:CoM ends its attack action first.
        self.leave_swing(h);
        if self.recom_took {
            return refuse(self, "Re:CoM's win");
        }
        if !h.start_item(&c) {
            return refuse(self, "not in its slot");
        }
        self.play(USE_SELF as u32 | req::ACTION, USE_BLEND);
        self.append(req::RELEASE | request::IDLE as u32, USE_BLEND);
        self.item = Some(c);
        self.item_log.push(ItemEvent::Started { item: c.item, slot: c.slot });
    }

    /// The use motion is no longer KH1's playing motion (it ended into its idle, or another command
    /// took over after its free frame 74): Re:CoM ends its item action (`0x1401F6660`, as `exeItem`
    /// ends it) unless a swing or a cast already took Re:CoM's action over. The use task goes on
    /// (KH1's task doesn't watch the motion but for its hand effect).
    pub(crate) fn leave_item(&mut self, h: &mut dyn Host) {
        if self.item.is_none() || self.rec.head() == USE_SELF {
            return;
        }
        self.item = None;
        if self.swing.is_none() && self.cast.is_none() && h.end_item(self.state != state::GROUND) {
            self.recom_took = true;
        }
    }

    /// The item events (drained).
    pub fn take_item_log(&mut self) -> Vec<ItemEvent> {
        std::mem::take(&mut self.item_log)
    }
}

/// The Field exe (`0x1401FA0F0`) has no case for action `0xE` (its jump table `0x1401FA5F4` sends
/// it to the default `0x1401FA599`: Re:CoM uses items only as cards, in battle). KH1 uses items in
/// the field, so the entry is pointed at a stub in an int3 cave that runs `exeItem` on the player
/// (`rbx` in the switch) and goes on at the default, as magic's case does. Checked (exe: the
/// table's entry and the cave's 14 int3 bytes after `0x1401FAEA1`'s `ret`).
pub mod field_case {
    pub const TABLE_ENTRY_RVA: u32 = 0x001F_A628;
    pub const CAVE_RVA: u32 = 0x001F_AEA2;
    pub const EXE_ITEM_RVA: u32 = 0x001E_D6E0;
    pub const DEFAULT_RVA: u32 = 0x001F_A599;
    pub const STUB_LEN: usize = 13;

    /// `mov rcx,rbx; call exeItem; jmp default`.
    pub fn stub() -> [u8; STUB_LEN] {
        let mut b = [0u8; STUB_LEN];
        b[..3].copy_from_slice(&[0x48, 0x8B, 0xCB]);
        b[3] = 0xE8;
        let call = EXE_ITEM_RVA as i64 - (CAVE_RVA as i64 + 8);
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

/// The trace token of the use task: `item -` or `item <name> f<age> s<state> d<delay>`.
pub fn trace_token(t: Option<&UseTask>) -> String {
    match t {
        None => "item -".into(),
        Some(t) => format!("item {} f{:.0} s{} d{:.0}", name(t.item), t.age, t.state, t.delay.max(0.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_and_commands_are_kh1s() {
        assert_eq!(data(id::POTION).map(|d| (d.amount, d.buy, d.sell)), Some((30, 60, 12)));
        assert_eq!(data(id::MEGALIXIR).map(|d| d.party), Some(true));
        assert_eq!(data(5), None, "item row 5 is empty in KH1");
        assert_eq!(command(id::POTION), 0x1F);
        assert_eq!(command(id::MEGALIXIR), 0x26);
        assert_eq!(item_of_command(0x24), Some(id::MEGA_POTION));
        assert_eq!(item_of_command(0x23), None);
        assert_eq!(item_of_command(7), None, "a spell command");
        // The command's message id is its number (Word.bin: 31 Potion … 38 Megalixir).
        for d in DATA {
            assert_eq!(d.name, command(d.id) as u16);
        }
    }

    #[test]
    fn the_tents_are_kh1s_rows_142_and_143() {
        let t = data(id::TENT).unwrap();
        assert_eq!((t.name, t.kind, t.party, t.amount, t.buy, t.sell, t.sort), (172, 12, true, 255, 250, 50, 101));
        let c = data(id::CAMPING_SET).unwrap();
        assert_eq!((c.name, c.kind, c.party, c.amount, c.buy, c.sell, c.sort), (173, 12, true, 3, 500, 100, 102));
        // Menu items: no slot, no command, no use in the command menu.
        assert!(!is_battle_item(id::TENT) && !is_battle_item(id::CAMPING_SET));
        assert!(camp_only(id::TENT) && camp_only(id::CAMPING_SET) && !camp_only(id::POTION));
        assert_eq!(item_of_command(command(id::TENT)), None);
        assert_eq!(item_of_command(command(id::CAMPING_SET)), None);
        assert_eq!(recom_effect(id::TENT), None, "never used by the battle use task");
        assert_eq!(name(id::CAMPING_SET), "camping set");
        // The Tent: HP to full (KH1's 9999); the Camping Set: HP and MP to full (the owner's).
        assert_eq!(effect(id::TENT), Some(ItemEffect { hp: Some(HpHeal::Full), mp: None }));
        assert_eq!(effect(id::CAMPING_SET), Some(ItemEffect { hp: Some(HpHeal::Full), mp: Some(MP_FULL) }));
        assert!(CAMPING_SET_FULL_MP);
        // KH1's own Camping Set adds its row's 3 MP points (case 0x8F).
        assert_eq!(c.amount, 3);
    }

    #[test]
    fn the_tents_stack_in_the_stock_only() {
        assert_eq!([1, 4, 8, 142, 143].map(stock_index), [Some(0), Some(3), Some(7), Some(8), Some(9)]);
        assert_eq!([0, 9, 16, 141, 144, 255].map(stock_index), [None; 6]);
        // Every held item has its own byte.
        let mut seen: Vec<usize> = id::HELD.iter().map(|&i| stock_index(i).unwrap()).collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), id::HELD.len());
        assert!(seen.iter().all(|&i| i < STOCK_IDS));
        let mut inv = Inventory::new_game();
        assert_eq!(inv.pick_up(id::TENT), Some(Picked::Stock(1)), "empty slots, but a Tent never goes in one");
        assert_eq!(inv.pick_up(id::CAMPING_SET), Some(Picked::Stock(1)));
        assert_eq!(inv.pick_up(id::TENT), Some(Picked::Stock(2)));
        assert!(inv.filled().is_empty());
        assert_eq!((inv.stock[8], inv.stock[9]), (2, 1));
        inv.set_stock(id::TENT, 99);
        assert_eq!(inv.pick_up(id::TENT), None, "99 is the cap");
        // An item with no byte is refused.
        assert_eq!(inv.pick_up(9), None);
        assert_eq!(inv.pick_up(233), None);
        // The save round-trips them.
        let mut b = [0u8; EXT_LEN];
        write(&inv, &mut b);
        assert_eq!((b[ext::STOCK + 8], b[ext::STOCK + 9]), (99, 1));
        assert_eq!(read(&b), inv);
    }

    #[test]
    fn a_saves_mega_items_are_converted_once() {
        let mut inv = Inventory::new_game();
        inv.slots[..3].copy_from_slice(&[id::MEGA_POTION, id::POTION, id::MEGALIXIR]);
        inv.set_stock(id::MEGA_POTION, 4);
        inv.set_stock(id::HI_POTION, 97);
        inv.set_stock(id::MEGA_ETHER, 2);
        inv.set_stock(id::TENT, 3);
        let mut b = [0u8; EXT_LEN];
        write(&inv, &mut b);
        b[ext::VERSION] = EXT_V1;
        assert_eq!(convert_removed(&mut b), Some(Converted { slots: 2, stock: 4, lost: 2 }));
        assert_eq!(b[ext::VERSION], EXT_V2);
        let after = read(&b);
        assert_eq!(after.slots[..3], [id::HI_POTION, id::POTION, id::ELIXIR]);
        assert_eq!(after.stock_of(id::HI_POTION), 99, "97 + 2 of the 4: the cap");
        assert_eq!(after.stock_of(id::ETHER), 2);
        assert_eq!(after.stock_of(id::TENT), 3, "untouched");
        assert!(id::REMOVED.iter().all(|&m| after.stock_of(m) == 0));
        // Once: a v2 block is left alone, even holding one (it can't: nothing adds one).
        let mut again = b;
        again[ext::SLOTS] = id::MEGALIXIR;
        assert_eq!(convert_removed(&mut again), None);
        assert_eq!(again[ext::SLOTS], id::MEGALIXIR);
        // A never-written block stays never written (KH1's new game, not 0 slots).
        let mut z = [0u8; EXT_LEN];
        assert_eq!(convert_removed(&mut z), None);
        assert_eq!(z, [0u8; EXT_LEN]);
        // A v1 block without any: just marked.
        let mut plain = [0u8; EXT_LEN];
        write(&Inventory::new_game(), &mut plain);
        plain[ext::VERSION] = EXT_V1;
        assert_eq!(convert_removed(&mut plain), Some(Converted::default()));
        assert_eq!((plain[ext::VERSION], read(&plain)), (EXT_V2, Inventory::new_game()));
        // The replacements.
        assert_eq!([id::MEGA_POTION, id::MEGA_ETHER, id::MEGALIXIR].map(replacement), [Some(id::HI_POTION), Some(id::ETHER), Some(id::ELIXIR)]);
        assert_eq!(replacement(id::POTION), None);
        assert!(id::IN_PLAY.iter().all(|&i| !removed(i)) && id::REMOVED.iter().all(|&i| removed(i)));
    }

    #[test]
    fn effects_and_heal_rule() {
        assert_eq!(effect(id::POTION), Some(ItemEffect { hp: Some(HpHeal::Amount(140)), mp: None }));
        assert_eq!(effect(id::HI_POTION).unwrap().hp, Some(HpHeal::Amount(280)));
        assert_eq!(effect(id::MEGA_POTION).unwrap().hp, Some(HpHeal::Amount(140)));
        assert_eq!(effect(id::ETHER), Some(ItemEffect { hp: None, mp: Some(3) }));
        assert_eq!(effect(id::MEGALIXIR), Some(ItemEffect { hp: Some(HpHeal::Full), mp: Some(MP_FULL) }));
        // KH1 heals only below max.
        assert_eq!(hp_to_add(HpHeal::Amount(140), 100, 300), Some(140));
        assert_eq!(hp_to_add(HpHeal::Amount(140), 300, 300), None);
        assert_eq!(hp_to_add(HpHeal::Full, 12, 300), Some(300));
        assert_eq!(recom_effect(id::ETHER), Some((EffectSet::Always, 5)));
        assert_eq!(recom_effect(id::MEGA_ETHER), Some((EffectSet::ItemPlus, 1)));
        // KH1's ids of the base four are Re:CoM's always ids for the same items.
        for i in [id::POTION, id::HI_POTION, id::ETHER, id::ELIXIR] {
            assert_eq!(recom_effect(i).map(|e| e.1), Some(KH1_EFFECT_IDS[i as usize] as u32));
        }
    }

    #[test]
    fn pickup_fills_the_first_empty_slot_then_the_stock() {
        let mut inv = Inventory::new_game();
        assert_eq!(inv.slot_count, 3);
        assert!(inv.filled().is_empty());
        assert_eq!(inv.pick_up(id::POTION), Some(Picked::Slot(0)));
        assert_eq!(inv.pick_up(id::ETHER), Some(Picked::Slot(1)));
        assert_eq!(inv.pick_up(id::POTION), Some(Picked::Slot(2)));
        assert_eq!(inv.pick_up(id::POTION), Some(Picked::Stock(1)));
        assert_eq!(inv.filled(), vec![(0, id::POTION), (1, id::ETHER), (2, id::POTION)]);
        // Using slot 1 empties it; the stock never refills it; the next pickup does.
        assert_eq!(inv.take(1, id::ETHER), Some(id::ETHER));
        assert_eq!(inv.filled(), vec![(0, id::POTION), (2, id::POTION)]);
        assert_eq!(inv.stock_of(id::POTION), 1);
        assert_eq!(inv.take(1, id::ETHER), None, "the slot is empty now");
        assert_eq!(inv.take(0, id::ETHER), None, "slot 0 holds a Potion");
        assert_eq!(inv.pick_up(id::HI_POTION), Some(Picked::Slot(1)));
        // The stock stops at 99.
        inv.stock[id::ETHER as usize - 1] = 98;
        assert_eq!(inv.pick_up(id::ETHER), Some(Picked::Stock(99)));
        assert_eq!(inv.pick_up(id::ETHER), None);
        // A slot past the count is not used.
        let mut one = Inventory { slot_count: 1, ..Inventory::new_game() };
        one.pick_up(id::POTION);
        assert_eq!(one.pick_up(id::POTION), Some(Picked::Stock(1)));
        assert_eq!(one.take(1, 0), None);
    }

    #[test]
    fn the_extension_migrates_from_layout_v1_and_round_trips() {
        // A layout v1 save: the extension is all zero, read as KH1's new game, nothing written.
        let z = [0u8; EXT_LEN];
        assert_eq!(read(&z), Inventory::new_game());
        let mut inv = Inventory::new_game();
        inv.pick_up(id::HI_POTION);
        inv.stock[0] = 12;
        let mut b = [0u8; EXT_LEN];
        write(&inv, &mut b);
        assert_eq!(b[ext::VERSION], EXT_V2);
        assert_eq!(b[ext::SLOT_COUNT], 3);
        assert_eq!(b[ext::SLOTS], id::HI_POTION);
        assert_eq!(b[ext::STOCK], 12);
        assert_eq!(read(&b), inv);
        // A later version is never lowered; out-of-range bytes are clamped.
        b[ext::VERSION] = 3;
        b[ext::SLOT_COUNT] = 200;
        b[ext::STOCK + 1] = 250;
        write(&read(&b), &mut b);
        assert_eq!(b[ext::VERSION], 3);
        assert_eq!(b[ext::SLOT_COUNT], MAX_SLOTS as u8);
        assert_eq!(b[ext::STOCK + 1], MAX_STOCK);
        // The extension follows the MP block and stays in the spare range.
        assert_eq!(EXT_FIRST, crate::mp::BLOCK_LAST + 1);
        assert!(EXT_LAST <= crate::keyblades::SPARE_LAST);
    }

    #[test]
    fn the_test_inventory_refills_only_empty_slots() {
        let mut inv = Inventory::new_game();
        assert!(test_inventory(&mut inv));
        assert_eq!(inv.filled(), vec![(0, id::POTION), (1, id::HI_POTION), (2, id::ETHER)]);
        assert!(id::IN_PLAY.iter().all(|&i| inv.stock_of(i) == TEST_STOCK));
        // M8c stage 3: never a Mega item.
        assert!(id::REMOVED.iter().all(|&i| inv.stock_of(i) == 0));
        assert!(TEST_SLOTS.iter().all(|i| !removed(*i)));
        assert!(!test_inventory(&mut inv), "nothing to do");
        inv.take(0, id::POTION);
        assert!(!test_inventory(&mut inv), "a slot still holds an item");
        inv.take(1, id::HI_POTION);
        inv.take(2, id::ETHER);
        assert!(test_inventory(&mut inv));
        assert_eq!(inv.filled().len(), 3);
    }

    #[test]
    fn the_use_task_has_kh1s_timing() {
        // At 60 fps: the hand effect on run 34 (34 − 35 < 0), the heal 31 runs later (30 − 31 < 0).
        let mut t = UseTask::new(id::POTION, 0);
        let mut hand = None;
        let mut heal = None;
        for k in 0..100 {
            match t.run(1.0, true, false, 0x2D) {
                TaskStep::Hand { same_motion } => {
                    assert!(same_motion);
                    hand = Some(k);
                }
                TaskStep::Heal => heal = Some(k),
                TaskStep::Return => panic!("returned"),
                TaskStep::Wait => {}
            }
        }
        assert_eq!((hand, heal), (Some(34), Some(65)));
        assert_eq!(t.age, 65.0, "the heal's run is 65 frames after the command's");
        // A hit (another motion) before the hand: no hand effect, the heal still comes (KH1's task
        // doesn't watch the motion but for that effect).
        let mut t = UseTask::new(id::ETHER, 1);
        for _ in 0..20 {
            t.run(1.0, true, false, 0x2D);
        }
        let mut saw = Vec::new();
        for _ in 0..60 {
            let s = t.run(1.0, true, false, 0x30);
            if s != TaskStep::Wait {
                saw.push(s);
            }
        }
        assert_eq!(saw, vec![TaskStep::Hand { same_motion: false }, TaskStep::Heal]);
        // Sora dies (or an event takes him) before the heal: the item goes back.
        let mut t = UseTask::new(id::POTION, 0);
        for _ in 0..40 {
            t.run(1.0, true, false, 0x2D);
        }
        assert_eq!(t.run(1.0, false, false, 0x2D), TaskStep::Return);
        assert_eq!(t.run(1.0, true, false, 0x2D), TaskStep::Wait, "ended");
        // At 30 fps (step 2): the hand on run 17 (34 − 36 < 0), the heal 16 runs later.
        let mut t = UseTask::new(id::POTION, 0);
        let steps: Vec<_> = (0..40).map(|_| t.run(2.0, true, false, 0x2D)).collect();
        assert_eq!(steps.iter().position(|s| matches!(s, TaskStep::Hand { .. })), Some(17));
        assert_eq!(steps.iter().position(|s| *s == TaskStep::Heal), Some(33));
    }

    #[test]
    fn field_stub_bytes() {
        let s = field_case::stub();
        assert_eq!(s[..4], [0x48, 0x8B, 0xCB, 0xE8]);
        let call = i32::from_le_bytes(s[4..8].try_into().unwrap()) as i64;
        assert_eq!(field_case::CAVE_RVA as i64 + 8 + call, field_case::EXE_ITEM_RVA as i64);
        assert_eq!(s[8], 0xE9);
        let jmp = i32::from_le_bytes(s[9..13].try_into().unwrap()) as i64;
        assert_eq!(field_case::CAVE_RVA as i64 + 13 + jmp, field_case::DEFAULT_RVA as i64);
        // The table entry for action 0xE is the switch's base + (0xE − 1) × 4.
        assert_eq!(0x001F_A5F4 + (RECOM_ITEM_ACTION - 1) * 4, field_case::TABLE_ENTRY_RVA);
    }

    #[test]
    fn motion_is_kh1s_entry_58() {
        let mut m = MotionSet::default();
        add_motions(&mut m);
        let i = m.info(USE_SELF).expect("added");
        assert_eq!(i.length, 82.0);
        assert_eq!(i.flags_at(73.0) & kh1_core::motion::flag::FREE, 0);
        assert_ne!(i.flags_at(74.0) & kh1_core::motion::flag::FREE, 0);
    }

    #[test]
    fn tokens() {
        assert_eq!(ItemEvent::Started { item: 1, slot: 0 }.token(), "use(potion slot 1)");
        assert_eq!(ItemEvent::Refused { item: 3, why: "state" }.token(), "use-refused(ether state)");
        assert_eq!(trace_token(None), "item -");
        let t = UseTask::new(2, 1);
        assert_eq!(trace_token(Some(&t)), "item hi-potion f0 s0 d34");
    }
}
