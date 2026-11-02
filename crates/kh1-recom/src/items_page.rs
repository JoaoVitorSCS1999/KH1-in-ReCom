//! M8 stage 8a.2: KH1's pause-menu Items screen, the testable part. The page is Re:CoM's camp list
//! (`win::keyblades`, the "Items" row of the "KH1" entry's submenu, `crate::keyblades::sub`), the
//! store is our save block's items extension (`crate::items`, stage 8a.1). Notes:
//! `docs/recom-m8-items.md` § Stage 8a.2 and § The Items menu with Sora's equipment.
//!
//! Since the owner's ask of 2026-11-01 ("one menu inside of the "KH1" menu for "Items" in "items,"
//! i want a slot for Keyblade, and all the item slots for items") Sora's list is his equipment
//! list ([`SORA_KINDS`], [`equipment`]): the Keyblade slot first (confirm: [`Outcome::Keyblade`],
//! the page's Keyblade list), then his item slots with KH1's Items-screen commands. Accessory slots
//! (later) are one more [`SlotKind`] between the two.
//!
//! KH1 (Steam; Checked (decomp + exe + files) unless tagged), the screen's windows and their
//! globals (`0x142E9C1F8` the state, `0x142E9C1FC` the source owner, `0x142E9C208` / `0x142E9C20C`
//! the source slot / item, `0x142E9C210` the command, `0x142E9C218` the destination owner):
//!
//! - **State 0, the owner list** `0x14030D160`: the party members, then "Stock" (sysmsg `0x38`).
//!   A member with no slots or an empty stock list is greyed; confirming it is the error sound.
//! - **State 1, the owner's items**: a member's slots `0x14030FA10` (one row a slot, an empty
//!   slot named by sysmsg `0x97`), or the stock list `0x14030F330` (the items with a count, two
//!   columns, the count drawn when above 1). Confirm on any row (an empty slot too) opens the
//!   commands.
//! - **The stock lists** `0x14030EEB0`: every item with a stock byte; the list shown holds the
//!   item kinds 0, 12, 13 (`0x1402E8F80`, table `0x140545320`); the list to equip from only kind 0
//!   (`0x1402E8F50`); both sorted by the row's sort key `+0x10` (then the id).
//! - **State 2, the commands** `0x14030CB50` (words sysmsg 0x91, 0x94, 0x92 / 0x95, 0x93):
//!   0 Use: an item that can be used now (`0x1402E8B20` → `0x1402CD3A0` on any member); a
//!   single-target item opens the target list, a party item is used at once (`0x14030F160`).
//!   1 Move: a kind-0 item, or an empty slot (`0x1402E8A00`); opens the destination owners.
//!   2 Remove (0x92, a filled slot: back to the stock, refused at 99 with sysmsg 0xA1) or Equip
//!   (0x95, an empty slot: the kind-0 stock list; none → sysmsg 0xA2); greyed for the stock.
//!   3 Discard: a filled row whose kind isn't 13 (`0x1402E8AA0`); a yes / no window (sysmsg 0xA3,
//!   `0x14030C8D0`, its cursor on row 0, yes) then the stock − 1 or the slot cleared.
//!   A greyed command: the error sound.
//! - **State 3, the destination owners** (Move, `0x14030D7B0` case 3): the members, then the stock
//!   for a slot's item only (greyed when no kind-0 item is in stock). **State 4, the destination's list**: a member's slots (`0x14030FA10`) or
//!   the kind-0 stock list (`0x14030F330`). The exchanges are below ([`Screen::confirm`]); none
//!   checks for duplicates: the same item can sit in several slots.
//! - **State 5**: the lists are rebuilt and the screen goes back to the source's list (the owner
//!   list when the source was the stock and the stock list is now empty), its cursor kept; after an
//!   Equip the slot cursor moves to the member's first empty slot.
//! - Cancel steps back one window (the destination list → the destination owners, or the commands
//!   for Equip; the owners → the commands; the commands → the list; the list → the owners).
//!
//! Ours: Sora is the only member (no party yet, as the owner chose for Cure / Aero and the battle
//! menu's items), so a single-target Use is used on Sora at once (KH1 shows the target list).

use crate::items::{self as it, Inventory};

/// A kind of slot in Sora's list (the owner's equipment list), in [`SORA_KINDS`]' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotKind {
    /// One row: the equipped Keyblade (`crate::keyblades`, the save byte).
    Keyblade,
    /// One row per item slot (`Inventory::slot_count`): KH1's Items screen.
    Item,
}

/// Sora's list, top to bottom: the Keyblade, then the item slots. Accessories (not built) go
/// between them: one more kind here and its rows in [`equipment`].
pub const SORA_KINDS: &[SlotKind] = &[SlotKind::Keyblade, SlotKind::Item];

/// A row of Sora's list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Keyblade,
    /// Item slot `k` (0-based).
    Item(u8),
}

/// Sora's item slots shown (KH1's slot count, at most [`it::MAX_SLOTS`]).
fn item_slots(inv: &Inventory) -> usize {
    (inv.slot_count as usize).min(it::MAX_SLOTS)
}

/// Sora's list: every kind of [`SORA_KINDS`] with its rows.
pub fn equipment(inv: &Inventory) -> Vec<Slot> {
    SORA_KINDS
        .iter()
        .flat_map(|k| match k {
            SlotKind::Keyblade => vec![Slot::Keyblade],
            SlotKind::Item => (0..item_slots(inv)).map(|k| Slot::Item(k as u8)).collect(),
        })
        .collect()
}

/// The row of `slot` in Sora's list.
pub fn row_of(inv: &Inventory, slot: Slot) -> Option<usize> {
    equipment(inv).iter().position(|&s| s == slot)
}

/// An owner row: KH1's party members (Sora only for now) and the stock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Sora,
    Stock,
}

/// What a command acts on: Sora's slot `k`, or an item of the stock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Slot(u8),
    Stock(u8),
}

impl Source {
    /// The item it holds (0: an empty slot).
    pub fn item(self, inv: &Inventory) -> u8 {
        match self {
            Source::Slot(k) => inv.slots.get(k as usize).copied().unwrap_or(0),
            Source::Stock(i) => i,
        }
    }

    pub fn owner(self) -> Owner {
        match self {
            Source::Slot(_) => Owner::Sora,
            Source::Stock(_) => Owner::Stock,
        }
    }
}

/// KH1's four commands, in KH1's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Use,
    Move,
    /// Remove (a filled slot) or Equip (an empty slot).
    EquipRemove,
    Discard,
}

pub const COMMANDS: [Command; 4] = [Command::Use, Command::Move, Command::EquipRemove, Command::Discard];

/// KH1's windows (its states 0..4 and the discard question).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// State 0.
    Owners,
    /// State 1.
    List(Owner),
    /// State 2.
    Commands(Source),
    /// State 3 (Move): where to.
    MoveTo(Source),
    /// State 4: the destination's list. `equip`: KH1's Equip (the kind-0 stock list straight from
    /// the commands), else Move.
    Pick { from: Source, to: Owner, equip: bool },
    /// The discard question (`0x14030C8D0`).
    Discard(Source),
}

/// A row's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Label {
    Owner(Owner),
    /// Sora's Keyblade slot (the page writes the equipped Keyblade's name).
    Keyblade,
    /// A slot's item (0: empty).
    Slot(u8),
    /// A stock item and its count.
    Stocked(u8, u8),
    /// A command; `remove`: the third command reads Remove (a filled slot), else Equip.
    Command(Command, bool),
    Yes,
    No,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub label: Label,
    pub grey: bool,
}

/// Sora's state the Use rule reads (`0x1402CD3A0`): HP below max, MP below max.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ctx {
    pub hp_low: bool,
    pub mp_low: bool,
}

/// KH1's message windows the screen opens on a refusal (shown on our help line).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Msg {
    /// sysmsg 0xA1: the stock of that item is full (99).
    StockFull,
    /// sysmsg 0xA2: nothing in the stock to equip.
    NoStock,
}

/// What an exchange did (for the log; `Used` asks the page to heal).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Used { item: u8, from: Source },
    Removed { item: u8, slot: u8 },
    Equipped { item: u8, slot: u8 },
    /// Two of Sora's slots exchanged.
    Swapped { a: u8, b: u8 },
    /// A stock item went into a slot; the slot's old item (0: none) went to the stock.
    Exchanged { into: u8, slot: u8, out: u8 },
    Discarded { item: u8, from: Source },
}

/// What a confirm or cancel did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing, no sound (KH1's stock-to-stock Move pick).
    Nothing,
    /// A greyed row: the error sound.
    Refused,
    /// The error sound and KH1's message.
    Error(Msg),
    /// A window opened (KH1's confirm sound).
    Opened,
    /// Sora's Keyblade slot confirmed: the page opens its Keyblade list (confirm sound); the
    /// screen stays on Sora's list, on that row.
    Keyblade,
    /// Back one window (KH1's back sound).
    Back,
    /// Cancel on the owner list: the page closes.
    Close,
    /// The inventory changed; back to the source's list.
    Done(Effect),
}

/// KH1's item kinds the stock list shows (`0x140545320`: 0, 12, 13). Our store only ever holds the
/// battle items (kind 0); ids 9..16 (kind 13) have no data here and are left out. Checked (exe).
pub fn stock_list(inv: &Inventory) -> Vec<u8> {
    sorted((1..=it::STOCK_IDS as u8).filter(|&i| inv.stock_of(i) > 0 && it::data(i).is_some()))
}

/// The list to equip from (`0x1402E8F50`): kind 0 with a count.
pub fn equippable(inv: &Inventory) -> Vec<u8> {
    sorted((1..=it::STOCK_IDS as u8).filter(|&i| inv.stock_of(i) > 0 && it::is_battle_item(i)))
}

/// KH1's sort (`0x14030EEB0`: the row's `+0x10` key, then the id). The battle items' keys are their
/// ids (1, 2, 3, 4, 6, 7, 8). Checked (files: the owner's `btltbl.bin`).
fn sorted(ids: impl Iterator<Item = u8>) -> Vec<u8> {
    let mut v: Vec<u8> = ids.collect();
    v.sort_by_key(|&i| (it::data(i).map_or(u16::MAX, |d| d.sort), i));
    v
}

/// KH1's `0x1402CD3A0` (the pause menu's can-use): HP items while HP is below max, Ethers while MP
/// is below max, Elixirs while either is.
pub fn can_use(item: u8, ctx: Ctx) -> bool {
    match item {
        it::id::POTION | it::id::HI_POTION | it::id::MEGA_POTION => ctx.hp_low,
        it::id::ETHER | it::id::MEGA_ETHER => ctx.mp_low,
        it::id::ELIXIR | it::id::MEGALIXIR => ctx.hp_low || ctx.mp_low,
        _ => false,
    }
}

/// KH1's discard test (`0x1402E8AA0`: kind isn't 13); all battle items can be discarded.
fn discardable(item: u8) -> bool {
    it::data(item).is_some_and(|d| d.kind != 13)
}

/// The screen: KH1's window stack, each window with its cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Screen {
    stack: Vec<(Mode, i32)>,
}

impl Default for Screen {
    fn default() -> Self {
        Screen::new()
    }
}

impl Screen {
    /// The page opens on the owner list, Sora highlighted.
    pub fn new() -> Screen {
        Screen { stack: vec![(Mode::Owners, 0)] }
    }

    pub fn mode(&self) -> Mode {
        self.stack.last().map_or(Mode::Owners, |s| s.0)
    }

    /// The cursor, clamped to the window's rows.
    pub fn cursor(&self, inv: &Inventory, ctx: Ctx) -> i32 {
        let n = self.rows(inv, ctx).len() as i32;
        self.stack.last().map_or(0, |s| s.1).clamp(0, (n - 1).max(0))
    }

    pub fn set_cursor(&mut self, c: i32) {
        if let Some(s) = self.stack.last_mut() {
            s.1 = c;
        }
    }

    fn push(&mut self, m: Mode, cursor: i32) -> Outcome {
        self.stack.push((m, cursor));
        Outcome::Opened
    }

    /// The window's rows.
    pub fn rows(&self, inv: &Inventory, ctx: Ctx) -> Vec<Row> {
        let row = |label, grey| Row { label, grey };
        let slots = |inv: &Inventory| (0..item_slots(inv)).map(|k| row(Label::Slot(inv.slots[k]), false)).collect::<Vec<_>>();
        match self.mode() {
            // KH1 greys a member with no slots; Sora always has his Keyblade slot.
            Mode::Owners => vec![row(Label::Owner(Owner::Sora), equipment(inv).is_empty()), row(Label::Owner(Owner::Stock), stock_list(inv).is_empty())],
            Mode::List(Owner::Sora) => equipment(inv)
                .into_iter()
                .map(|s| match s {
                    Slot::Keyblade => row(Label::Keyblade, false),
                    Slot::Item(k) => row(Label::Slot(inv.slots[k as usize]), false),
                })
                .collect(),
            Mode::List(Owner::Stock) => stock_list(inv).into_iter().map(|i| row(Label::Stocked(i, inv.stock_of(i)), false)).collect(),
            Mode::Commands(src) => {
                let item = src.item(inv);
                let from_slot = matches!(src, Source::Slot(_));
                COMMANDS
                    .iter()
                    .map(|&c| {
                        let on = match c {
                            Command::Use => item != 0 && can_use(item, ctx),
                            // `0x14030CB50`: an empty slot counts as movable (`iVar4 = 1`).
                            Command::Move => item == 0 || it::is_battle_item(item),
                            Command::EquipRemove => from_slot,
                            Command::Discard => item != 0 && discardable(item),
                        };
                        row(Label::Command(c, item != 0), !on)
                    })
                    .collect()
            }
            // `0x14030D7B0` case 3: the members, and the stock only for a slot's item.
            Mode::MoveTo(src) => {
                let mut v = vec![row(Label::Owner(Owner::Sora), inv.slot_count == 0)];
                if matches!(src, Source::Slot(_)) {
                    v.push(row(Label::Owner(Owner::Stock), equippable(inv).is_empty()));
                }
                v
            }
            Mode::Pick { to: Owner::Sora, .. } => slots(inv),
            Mode::Pick { to: Owner::Stock, .. } => equippable(inv).into_iter().map(|i| row(Label::Stocked(i, inv.stock_of(i)), false)).collect(),
            Mode::Discard(_) => vec![row(Label::Yes, false), row(Label::No, false)],
        }
    }

    /// Confirm on the highlighted row. Changes `inv` only for a [`Outcome::Done`].
    pub fn confirm(&mut self, inv: &mut Inventory, ctx: Ctx) -> Outcome {
        let rows = self.rows(inv, ctx);
        let c = self.cursor(inv, ctx);
        let Some(r) = rows.get(c as usize).copied() else { return Outcome::Refused };
        if r.grey {
            return Outcome::Refused;
        }
        self.set_cursor(c);
        match (self.mode(), r.label) {
            (Mode::Owners, Label::Owner(o)) => self.push(Mode::List(o), 0),
            (Mode::List(Owner::Sora), _) => match equipment(inv).get(c as usize) {
                Some(Slot::Keyblade) => Outcome::Keyblade,
                Some(&Slot::Item(k)) => self.push(Mode::Commands(Source::Slot(k)), 0),
                None => Outcome::Refused,
            },
            (Mode::List(Owner::Stock), Label::Stocked(i, _)) => self.push(Mode::Commands(Source::Stock(i)), 0),
            (Mode::Commands(src), Label::Command(cmd, _)) => self.command(inv, src, cmd),
            (Mode::MoveTo(src), Label::Owner(o)) => self.push(Mode::Pick { from: src, to: o, equip: false }, 0),
            (Mode::Pick { from, to: Owner::Sora, .. }, _) => self.pick_slot(inv, from, c as u8),
            (Mode::Pick { from, to: Owner::Stock, equip }, Label::Stocked(i, _)) => self.pick_stock(inv, from, i, equip),
            (Mode::Discard(src), Label::Yes) => {
                let item = src.item(inv);
                match src {
                    Source::Stock(i) => inv.stock[i as usize - 1] -= 1,
                    Source::Slot(k) => inv.slots[k as usize] = 0,
                }
                self.done(inv, Effect::Discarded { item, from: src })
            }
            (Mode::Discard(_), Label::No) => self.cancel(),
            _ => Outcome::Refused,
        }
    }

    /// The commands window's confirm (`0x14030CB50`).
    fn command(&mut self, inv: &mut Inventory, src: Source, cmd: Command) -> Outcome {
        let item = src.item(inv);
        match cmd {
            // `0x14030F160` on Sora (the target list skipped: Sora is the only member).
            Command::Use => {
                match src {
                    Source::Stock(i) => inv.stock[i as usize - 1] -= 1,
                    Source::Slot(k) => inv.slots[k as usize] = 0,
                }
                self.done(inv, Effect::Used { item, from: src })
            }
            Command::Move => self.push(Mode::MoveTo(src), 0),
            Command::EquipRemove => {
                let Source::Slot(k) = src else { return Outcome::Refused };
                if item == 0 {
                    if equippable(inv).is_empty() {
                        return Outcome::Error(Msg::NoStock);
                    }
                    return self.push(Mode::Pick { from: src, to: Owner::Stock, equip: true }, 0);
                }
                if inv.stock_of(item) >= it::MAX_STOCK {
                    return Outcome::Error(Msg::StockFull);
                }
                inv.stock[item as usize - 1] += 1;
                inv.slots[k as usize] = 0;
                self.done(inv, Effect::Removed { item, slot: k })
            }
            Command::Discard => self.push(Mode::Discard(src), 0),
        }
    }

    /// A slot of Sora's picked as the destination (`0x14030FA10`, second mode, Move).
    fn pick_slot(&mut self, inv: &mut Inventory, from: Source, b: u8) -> Outcome {
        let x = inv.slots[b as usize];
        match from {
            Source::Slot(a) => {
                inv.slots.swap(a as usize, b as usize);
                self.done(inv, Effect::Swapped { a, b })
            }
            Source::Stock(y) => {
                if x != 0 && inv.stock_of(x) >= it::MAX_STOCK {
                    return Outcome::Error(Msg::StockFull);
                }
                if x != 0 {
                    inv.stock[x as usize - 1] += 1;
                }
                inv.stock[y as usize - 1] -= 1;
                inv.slots[b as usize] = y;
                self.done(inv, Effect::Exchanged { into: y, slot: b, out: x })
            }
        }
    }

    /// A stock item picked as the destination (`0x14030F330`, second mode): Move or Equip into the
    /// source slot.
    fn pick_stock(&mut self, inv: &mut Inventory, from: Source, y: u8, equip: bool) -> Outcome {
        // KH1 (`0x14030F330`): a stock source picking from the stock does nothing (unreachable: the
        // destination owners list no stock row for a stock item).
        let Source::Slot(a) = from else { return Outcome::Nothing };
        let x = inv.slots[a as usize];
        if !equip && x != 0 && inv.stock_of(x) >= it::MAX_STOCK {
            return Outcome::Error(Msg::StockFull);
        }
        inv.slots[a as usize] = y;
        inv.stock[y as usize - 1] -= 1;
        if equip {
            let r = self.done(inv, Effect::Equipped { item: y, slot: a });
            // KH1: the slot cursor goes to the member's first empty slot (if any), its row in
            // Sora's list.
            if let Some(r) = (0..item_slots(inv)).find(|&k| inv.slots[k] == 0).and_then(|k| row_of(inv, Slot::Item(k as u8))) {
                self.set_cursor(r as i32);
            }
            return r;
        }
        if x != 0 {
            inv.stock[x as usize - 1] += 1;
        }
        self.done(inv, Effect::Exchanged { into: y, slot: a, out: x })
    }

    /// KH1's state 5: back to the source's list (the owners when the stock list is now empty).
    fn done(&mut self, inv: &Inventory, e: Effect) -> Outcome {
        let src = match e {
            Effect::Used { from, .. } | Effect::Discarded { from, .. } => from,
            Effect::Removed { slot, .. } | Effect::Equipped { slot, .. } => Source::Slot(slot),
            Effect::Swapped { a, .. } => Source::Slot(a),
            Effect::Exchanged { .. } => match self.stack.iter().find_map(|s| if let Mode::Commands(f) = s.0 { Some(f) } else { None }) {
                Some(f) => f,
                None => Source::Slot(0),
            },
        };
        let list = Mode::List(src.owner());
        while self.stack.len() > 1 && self.mode() != list {
            self.stack.pop();
        }
        if src.owner() == Owner::Stock && stock_list(inv).is_empty() {
            self.stack.truncate(1);
        }
        Outcome::Done(e)
    }

    /// Cancel: back one window; on the owner list the page closes.
    pub fn cancel(&mut self) -> Outcome {
        if self.stack.len() <= 1 {
            return Outcome::Close;
        }
        self.stack.pop();
        Outcome::Back
    }

    /// The source being acted on (the commands' and later windows'), for the status panel.
    pub fn source(&self) -> Option<Source> {
        self.stack.iter().find_map(|s| if let Mode::Commands(f) = s.0 { Some(f) } else { None })
    }
}

impl Screen {
    /// The highlighted row of Sora's list (None in the other windows).
    pub fn slot(&self, inv: &Inventory, ctx: Ctx) -> Option<Slot> {
        match self.mode() {
            Mode::List(Owner::Sora) => equipment(inv).get(self.cursor(inv, ctx) as usize).copied(),
            _ => None,
        }
    }

    /// The status panel's subject: the owner and the item (0: none) the screen is about: on the
    /// owner list the highlighted owner, on a list the highlighted item, later the source.
    pub fn status(&self, inv: &Inventory, ctx: Ctx) -> (Owner, u8) {
        let c = self.cursor(inv, ctx) as usize;
        match self.mode() {
            Mode::Owners => (if c == 1 { Owner::Stock } else { Owner::Sora }, 0),
            Mode::List(o) => {
                let item = match self.rows(inv, ctx).get(c).map(|r| r.label) {
                    Some(Label::Slot(i)) | Some(Label::Stocked(i, _)) => i,
                    _ => 0,
                };
                (o, item)
            }
            _ => self.source().map_or((Owner::Sora, 0), |s| (s.owner(), s.item(inv))),
        }
    }

    /// The help line: KH1's message after a refusal, else the window's and row's line.
    pub fn help(&self, rows: &[Row], cursor: i32, msg: Option<Msg>) -> &'static [u8] {
        match msg {
            Some(Msg::StockFull) => return text::STOCK_FULL,
            Some(Msg::NoStock) => return text::NO_STOCK,
            None => {}
        }
        match self.mode() {
            Mode::Owners => text::OWNERS_HELP[cursor.clamp(0, 1) as usize],
            Mode::List(Owner::Sora) => match rows.get(cursor.max(0) as usize) {
                Some(Row { label: Label::Keyblade, .. }) => text::KEYBLADE_SLOT_HELP,
                _ => text::SLOT_HELP,
            },
            Mode::List(Owner::Stock) => text::STOCK_HELP,
            Mode::Commands(_) => match rows.get(cursor.max(0) as usize) {
                Some(&Row { label: Label::Command(c, remove), grey }) => match c {
                    Command::Use => text::USE_HELP[grey as usize],
                    Command::Move => text::MOVE_HELP[grey as usize],
                    Command::EquipRemove if grey => text::EQUIP_REMOVE_GREY,
                    Command::EquipRemove if remove => text::REMOVE_HELP,
                    Command::EquipRemove => text::EQUIP_HELP,
                    Command::Discard => text::DISCARD_HELP[grey as usize],
                },
                _ => text::SLOT_HELP,
            },
            Mode::MoveTo(_) => text::MOVE_TO_HELP,
            Mode::Pick { to: Owner::Sora, .. } => text::PICK_SLOT_HELP,
            Mode::Pick { to: Owner::Stock, .. } => text::PICK_STOCK_HELP,
            Mode::Discard(_) => text::DISCARD_ASK,
        }
    }
}

/// A stock row's text: the item's word, then its count when above 1 (KH1 draws the count beside
/// the name only then, `0x14030F330`). `word` is NUL-terminated.
pub fn stocked_text(word: &[u8], count: u8) -> Vec<u8> {
    let mut v: Vec<u8> = word.iter().copied().take_while(|&b| b != 0).collect();
    if count > 1 {
        v.extend_from_slice(format!(" x{count}").as_bytes());
    }
    v.push(0);
    v
}

/// The row's look on Re:CoM's camp list (`crate::keyblades::row_look`; owner, 2026-11-01: "lets
/// make "stock" have the item symbol, and make the box green", and every item slot of Sora's
/// list likewise): the Stock row and Sora's item slots (filled or empty) take Re:CoM's green row.
/// Its icon is the green Mickey symbol, not the item symbol (owner, in game, 2026-11-01): Re:CoM
/// has no camp row with the item symbol (`docs/recom-m8-items.md` § Row looks); open. The
/// Keyblade slot, the Sora row and the other windows keep the red row.
pub fn row_look(l: Label) -> i32 {
    use crate::keyblades::row_look::{GREEN, RED};
    match l {
        Label::Owner(Owner::Stock) | Label::Slot(_) => GREEN,
        _ => RED,
    }
}

/// A greyed row's text colour: Re:CoM's colour at half brightness, its alpha kept. Assumption (no
/// greyed camp row of Re:CoM's was read).
pub fn grey_colour(c: u32) -> u32 {
    (c & 0xFF00_0000) | ((c & 0x00FE_FEFE) >> 1)
}

/// Texts: NUL-terminated ASCII for Re:CoM's text items (no `%`: the help line's formatter treats it
/// as a code). KH1's own words for these (its sysmsg) are not in the asset step: ours (Assumption).
pub mod text {
    pub const LABEL: &[u8] = b"Items\0";
    pub const HELP: &[u8] = b"Sora's Keyblade and items: equip, use, move and discard.\0";
    pub const SORA: &[u8] = b"Sora\0";
    pub const STOCK: &[u8] = b"Stock\0";
    /// KH1's empty slot (sysmsg 0x97, not read).
    pub const EMPTY: &[u8] = b"Empty\0";
    pub const USE: &[u8] = b"Use\0";
    pub const MOVE: &[u8] = b"Move\0";
    pub const EQUIP: &[u8] = b"Equip\0";
    pub const REMOVE: &[u8] = b"Remove\0";
    pub const DISCARD: &[u8] = b"Discard\0";
    pub const YES: &[u8] = b"Yes\0";
    pub const NO: &[u8] = b"No\0";
    /// The status panel's labels.
    pub const STATUS: [&[u8]; 3] = [b"Owner\0", b"Item\0", b"In stock\0"];
    pub const NONE: &[u8] = b"-\0";

    pub const OWNERS_HELP: [&[u8]; 2] = [b"Sora's Keyblade and item slots.\0", b"The items you carry beyond Sora's slots.\0"];
    /// Sora's Keyblade slot.
    pub const KEYBLADE_SLOT_HELP: &[u8] = b"Sora's Keyblade. Confirm to change it.\0";
    pub const SLOT_HELP: &[u8] = b"Choose a slot.\0";
    pub const STOCK_HELP: &[u8] = b"Choose an item.\0";
    /// The commands (KH1: sysmsg 0x81..0x90, an enabled and a greyed line each, not read).
    pub const USE_HELP: [&[u8]; 2] = [b"Use this item on Sora now.\0", b"This item can't be used now.\0"];
    pub const MOVE_HELP: [&[u8]; 2] = [b"Move this item to a slot, or swap it with one.\0", b"This item can't be moved.\0"];
    pub const EQUIP_HELP: &[u8] = b"Put an item from the stock in this slot.\0";
    pub const REMOVE_HELP: &[u8] = b"Put this item back in the stock.\0";
    pub const EQUIP_REMOVE_GREY: &[u8] = b"Items in the stock are equipped from a slot.\0";
    pub const DISCARD_HELP: [&[u8]; 2] = [b"Throw this item away.\0", b"There is nothing to throw away.\0"];
    pub const MOVE_TO_HELP: &[u8] = b"Move it where?\0";
    pub const PICK_SLOT_HELP: &[u8] = b"Choose the slot.\0";
    pub const PICK_STOCK_HELP: &[u8] = b"Choose the item from the stock.\0";
    pub const DISCARD_ASK: &[u8] = b"Throw this item away?\0";
    /// KH1's sysmsg 0xA1 / 0xA2 (not read).
    pub const STOCK_FULL: &[u8] = b"The stock can't hold more of this item (99).\0";
    pub const NO_STOCK: &[u8] = b"There is nothing in the stock to equip.\0";

    /// The items' English names, when KH1's words (`kh1-command-menu.json`) aren't loaded.
    pub fn english(item: u8) -> &'static [u8] {
        use crate::items::id::*;
        match item {
            POTION => b"Potion\0",
            HI_POTION => b"Hi-Potion\0",
            ETHER => b"Ether\0",
            ELIXIR => b"Elixir\0",
            MEGA_POTION => b"Mega-Potion\0",
            MEGA_ETHER => b"Mega-Ether\0",
            MEGALIXIR => b"Megalixir\0",
            _ => b"?\0",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_and_item_slots_take_the_green_row() {
        use crate::keyblades::row_look::{GREEN, RED};
        assert_eq!(row_look(Label::Owner(Owner::Stock)), GREEN);
        assert_eq!(row_look(Label::Slot(0)), GREEN, "an empty slot too");
        assert_eq!(row_look(Label::Slot(crate::items::id::POTION)), GREEN);
        assert_eq!(row_look(Label::Owner(Owner::Sora)), RED);
        assert_eq!(row_look(Label::Keyblade), RED);
        assert_eq!(row_look(Label::Stocked(crate::items::id::POTION, 3)), RED);
        assert_eq!(row_look(Label::Yes), RED);
    }
    use crate::items::id::*;

    fn inv(slots: &[u8], stock: &[(u8, u8)]) -> Inventory {
        let mut v = Inventory::new_game();
        v.slots[..slots.len()].copy_from_slice(slots);
        for &(i, n) in stock {
            v.stock[i as usize - 1] = n;
        }
        v
    }

    const LOW: Ctx = Ctx { hp_low: true, mp_low: true };
    const FULL: Ctx = Ctx { hp_low: false, mp_low: false };

    fn labels(s: &Screen, v: &Inventory, ctx: Ctx) -> Vec<(Label, bool)> {
        s.rows(v, ctx).into_iter().map(|r| (r.label, r.grey)).collect()
    }

    /// Moves the cursor to `row` and confirms.
    fn pick(s: &mut Screen, v: &mut Inventory, ctx: Ctx, row: i32) -> Outcome {
        s.set_cursor(row);
        s.confirm(v, ctx)
    }

    #[test]
    fn owners_grey_and_open_as_kh1() {
        let mut v = inv(&[POTION, 0, 0], &[]);
        let mut s = Screen::new();
        assert_eq!(labels(&s, &v, LOW), vec![(Label::Owner(Owner::Sora), false), (Label::Owner(Owner::Stock), true)]);
        assert_eq!(pick(&mut s, &mut v, LOW, 1), Outcome::Refused, "an empty stock: the error sound");
        assert_eq!(pick(&mut s, &mut v, LOW, 0), Outcome::Opened);
        assert_eq!(s.mode(), Mode::List(Owner::Sora));
        assert_eq!(labels(&s, &v, LOW), vec![(Label::Keyblade, false), (Label::Slot(POTION), false), (Label::Slot(0), false), (Label::Slot(0), false)]);
        assert_eq!(s.cancel(), Outcome::Back);
        assert_eq!(s.cancel(), Outcome::Close);
    }

    #[test]
    fn soras_list_is_the_keyblade_then_the_item_slots() {
        let mut v = inv(&[0, ETHER, 0], &[]);
        assert_eq!(SORA_KINDS, &[SlotKind::Keyblade, SlotKind::Item]);
        assert_eq!(equipment(&v), vec![Slot::Keyblade, Slot::Item(0), Slot::Item(1), Slot::Item(2)]);
        assert_eq!(row_of(&v, Slot::Item(2)), Some(3));
        assert_eq!(row_of(&v, Slot::Item(3)), None);
        // KH1 greys a member with no slots: Sora always has his Keyblade slot.
        let mut none = v;
        none.slot_count = 0;
        assert_eq!(equipment(&none), vec![Slot::Keyblade]);
        assert!(!Screen::new().rows(&none, LOW)[0].grey);
        // The Keyblade row: the page's Keyblade list, the screen stays on it; cancel steps back.
        let mut s = Screen::new();
        pick(&mut s, &mut v, LOW, 0);
        assert_eq!(s.slot(&v, LOW), Some(Slot::Keyblade));
        assert_eq!(s.help(&s.rows(&v, LOW), 0, None), text::KEYBLADE_SLOT_HELP);
        let before = v;
        assert_eq!(pick(&mut s, &mut v, LOW, 0), Outcome::Keyblade);
        assert_eq!(v, before);
        assert_eq!((s.mode(), s.cursor(&v, LOW)), (Mode::List(Owner::Sora), 0));
        assert_eq!(s.status(&v, LOW), (Owner::Sora, 0));
        // An item slot: KH1's commands on that slot.
        assert_eq!(pick(&mut s, &mut v, LOW, 2), Outcome::Opened);
        assert_eq!(s.mode(), Mode::Commands(Source::Slot(1)));
        assert_eq!(s.slot(&v, LOW), None);
        assert_eq!(s.cancel(), Outcome::Back);
        assert_eq!((s.slot(&v, LOW), s.help(&s.rows(&v, LOW), 2, None)), (Some(Slot::Item(1)), text::SLOT_HELP));
        // A Move's destination slots are the item slots only.
        s.set_cursor(2);
        s.confirm(&mut v, LOW);
        s.set_cursor(1);
        s.confirm(&mut v, LOW);
        assert_eq!(pick(&mut s, &mut v, LOW, 0), Outcome::Opened);
        assert_eq!(labels(&s, &v, LOW), vec![(Label::Slot(0), false), (Label::Slot(ETHER), false), (Label::Slot(0), false)]);
        assert_eq!(pick(&mut s, &mut v, LOW, 0), Outcome::Done(Effect::Swapped { a: 1, b: 0 }));
        assert_eq!(v.slots[..3], [ETHER, 0, 0]);
        assert_eq!((s.mode(), s.cursor(&v, LOW)), (Mode::List(Owner::Sora), 2), "back on the source's row");
    }

    #[test]
    fn the_stock_list_is_sorted_with_counts() {
        let v = inv(&[], &[(MEGALIXIR, 1), (ETHER, 12), (POTION, 3)]);
        assert_eq!(stock_list(&v), vec![POTION, ETHER, MEGALIXIR]);
        let mut s = Screen::new();
        s.stack.push((Mode::List(Owner::Stock), 0));
        assert_eq!(labels(&s, &v, LOW)[1], (Label::Stocked(ETHER, 12), false));
        assert_eq!(equippable(&v), vec![POTION, ETHER, MEGALIXIR]);
    }

    #[test]
    fn commands_grey_as_kh1() {
        let v = inv(&[POTION, 0, ETHER], &[(POTION, 2)]);
        let mut s = Screen::new();
        s.stack.push((Mode::Commands(Source::Slot(0)), 0));
        assert_eq!(
            labels(&s, &v, LOW),
            vec![
                (Label::Command(Command::Use, true), false),
                (Label::Command(Command::Move, true), false),
                (Label::Command(Command::EquipRemove, true), false),
                (Label::Command(Command::Discard, true), false)
            ]
        );
        // Full HP: a Potion can't be used from the menu (`0x1402CD3A0`); an Ether at full MP neither.
        assert!(s.rows(&v, FULL)[0].grey);
        assert!(!s.rows(&v, Ctx { hp_low: true, mp_low: false })[0].grey);
        s.stack.push((Mode::Commands(Source::Slot(2)), 0));
        assert!(s.rows(&v, Ctx { hp_low: true, mp_low: false })[0].grey);
        // An empty slot: Use and Discard greyed, Move and Equip open.
        s.stack.push((Mode::Commands(Source::Slot(1)), 0));
        assert_eq!(s.rows(&v, LOW).iter().map(|r| r.grey).collect::<Vec<_>>(), vec![true, false, false, true]);
        assert_eq!(s.rows(&v, LOW)[2].label, Label::Command(Command::EquipRemove, false), "reads Equip");
        // From the stock: Equip / Remove greyed.
        s.stack.push((Mode::Commands(Source::Stock(POTION)), 0));
        assert_eq!(s.rows(&v, LOW).iter().map(|r| r.grey).collect::<Vec<_>>(), vec![false, false, true, false]);
    }

    #[test]
    fn remove_returns_to_the_stock_up_to_99() {
        let mut v = inv(&[POTION, HI_POTION, 0], &[(HI_POTION, 99)]);
        let mut s = Screen::new();
        pick(&mut s, &mut v, LOW, 0);
        pick(&mut s, &mut v, LOW, 1);
        assert_eq!(pick(&mut s, &mut v, LOW, 2), Outcome::Done(Effect::Removed { item: POTION, slot: 0 }));
        assert_eq!((v.slots[0], v.stock_of(POTION)), (0, 1));
        assert_eq!(s.mode(), Mode::List(Owner::Sora), "back to Sora's slots");
        assert_eq!(s.cursor(&v, LOW), 1, "the cursor kept (slot 1's row, under the Keyblade)");
        // Hi-Potion: the stock is full.
        pick(&mut s, &mut v, LOW, 2);
        let before = v;
        assert_eq!(pick(&mut s, &mut v, LOW, 2), Outcome::Error(Msg::StockFull));
        assert_eq!(v, before);
        assert_eq!(s.mode(), Mode::Commands(Source::Slot(1)));
    }

    #[test]
    fn equip_fills_an_empty_slot_and_moves_to_the_next_empty_one() {
        let mut v = inv(&[0, POTION, 0], &[(ETHER, 2), (HI_POTION, 1)]);
        let mut s = Screen::new();
        pick(&mut s, &mut v, LOW, 0);
        pick(&mut s, &mut v, LOW, 1);
        assert_eq!(pick(&mut s, &mut v, LOW, 2), Outcome::Opened);
        assert_eq!(s.mode(), Mode::Pick { from: Source::Slot(0), to: Owner::Stock, equip: true });
        assert_eq!(labels(&s, &v, LOW), vec![(Label::Stocked(HI_POTION, 1), false), (Label::Stocked(ETHER, 2), false)]);
        assert_eq!(pick(&mut s, &mut v, LOW, 0), Outcome::Done(Effect::Equipped { item: HI_POTION, slot: 0 }));
        assert_eq!((v.slots, v.stock_of(HI_POTION)), ([HI_POTION, POTION, 0, 0, 0, 0, 0, 0], 0));
        assert_eq!(s.mode(), Mode::List(Owner::Sora));
        assert_eq!(s.cursor(&v, LOW), 3, "the first empty slot (slot 2's row)");
        // The same item in two slots is KH1's rule (no duplicate check).
        pick(&mut s, &mut v, LOW, 3);
        pick(&mut s, &mut v, LOW, 2);
        pick(&mut s, &mut v, LOW, 0);
        assert_eq!(v.slots[..3], [HI_POTION, POTION, ETHER]);
        assert_eq!(s.cursor(&v, LOW), 3, "no empty slot: the cursor stays");
        // Equip with nothing to equip: KH1's message.
        v.slots[2] = 0;
        v.stock = [0; it::STOCK_IDS];
        pick(&mut s, &mut v, LOW, 3);
        assert_eq!(pick(&mut s, &mut v, LOW, 2), Outcome::Error(Msg::NoStock));
        // Cancel from Equip's list goes back to the commands (KH1: state 2).
        v.stock[0] = 1;
        assert_eq!(pick(&mut s, &mut v, LOW, 2), Outcome::Opened);
        assert_eq!(s.cancel(), Outcome::Back);
        assert_eq!(s.mode(), Mode::Commands(Source::Slot(2)));
    }

    #[test]
    fn move_swaps_slots_and_exchanges_with_the_stock() {
        let mut v = inv(&[POTION, 0, ETHER], &[(HI_POTION, 3), (POTION, 4)]);
        let mut s = Screen::new();
        pick(&mut s, &mut v, LOW, 0);
        pick(&mut s, &mut v, LOW, 1);
        assert_eq!(pick(&mut s, &mut v, LOW, 1), Outcome::Opened);
        assert_eq!(s.mode(), Mode::MoveTo(Source::Slot(0)));
        // Slot 1 → slot 3: swapped.
        pick(&mut s, &mut v, LOW, 0);
        assert_eq!(pick(&mut s, &mut v, LOW, 2), Outcome::Done(Effect::Swapped { a: 0, b: 2 }));
        assert_eq!(v.slots[..3], [ETHER, 0, POTION]);
        assert_eq!(s.mode(), Mode::List(Owner::Sora));
        // Slot 1 (Ether) ↔ a Hi-Potion of the stock: the Ether goes to the stock.
        pick(&mut s, &mut v, LOW, 1);
        pick(&mut s, &mut v, LOW, 1);
        pick(&mut s, &mut v, LOW, 1);
        assert_eq!(s.mode(), Mode::Pick { from: Source::Slot(0), to: Owner::Stock, equip: false });
        assert_eq!(pick(&mut s, &mut v, LOW, 1), Outcome::Done(Effect::Exchanged { into: HI_POTION, slot: 0, out: ETHER }));
        assert_eq!((v.slots[0], v.stock_of(HI_POTION), v.stock_of(ETHER)), (HI_POTION, 2, 1));
        // A stock item into a slot of Sora's: the slot's item goes to the stock.
        let mut s = Screen::new();
        pick(&mut s, &mut v, LOW, 1);
        assert_eq!(labels(&s, &v, LOW)[0].0, Label::Stocked(POTION, 4));
        pick(&mut s, &mut v, LOW, 0);
        pick(&mut s, &mut v, LOW, 1);
        pick(&mut s, &mut v, LOW, 0);
        assert_eq!(pick(&mut s, &mut v, LOW, 0), Outcome::Done(Effect::Exchanged { into: POTION, slot: 0, out: HI_POTION }));
        assert_eq!((v.slots[0], v.stock_of(POTION), v.stock_of(HI_POTION)), (POTION, 3, 3));
        assert_eq!(s.mode(), Mode::List(Owner::Stock), "back to the stock");
        // A stock item's destinations are the members only (KH1's state 3).
        pick(&mut s, &mut v, LOW, 0);
        pick(&mut s, &mut v, LOW, 1);
        assert_eq!(labels(&s, &v, LOW), vec![(Label::Owner(Owner::Sora), false)]);
        // Cancel from a Move's list goes to the destination owners (KH1: state 3).
        pick(&mut s, &mut v, LOW, 0);
        assert_eq!(s.cancel(), Outcome::Back);
        assert_eq!(s.mode(), Mode::MoveTo(Source::Stock(POTION)));
        // KH1's stock-to-stock pick does nothing.
        let before = v;
        let mut t = Screen { stack: vec![(Mode::Owners, 1), (Mode::Pick { from: Source::Stock(POTION), to: Owner::Stock, equip: false }, 0)] };
        assert_eq!(t.confirm(&mut v, LOW), Outcome::Nothing);
        assert_eq!(v, before);
    }

    #[test]
    fn move_refuses_when_the_returning_item_is_at_99() {
        let mut v = inv(&[ETHER, 0, 0], &[(ETHER, 99), (POTION, 1)]);
        let mut s = Screen::new();
        s.stack = vec![(Mode::Owners, 0), (Mode::List(Owner::Sora), 0), (Mode::Commands(Source::Slot(0)), 1), (Mode::MoveTo(Source::Slot(0)), 1), (Mode::Pick { from: Source::Slot(0), to: Owner::Stock, equip: false }, 0)];
        let before = v;
        assert_eq!(pick(&mut s, &mut v, LOW, 0), Outcome::Error(Msg::StockFull));
        assert_eq!(v, before);
        // The destination stock row greys when nothing kind 0 is in stock.
        let e = inv(&[POTION, 0, 0], &[]);
        let s = Screen { stack: vec![(Mode::MoveTo(Source::Slot(0)), 0)] };
        assert_eq!(labels(&s, &e, LOW), vec![(Label::Owner(Owner::Sora), false), (Label::Owner(Owner::Stock), true)]);
    }

    #[test]
    fn use_and_discard() {
        let mut v = inv(&[POTION, 0, 0], &[(ETHER, 1)]);
        let mut s = Screen::new();
        pick(&mut s, &mut v, LOW, 0);
        pick(&mut s, &mut v, LOW, 1);
        assert_eq!(pick(&mut s, &mut v, LOW, 0), Outcome::Done(Effect::Used { item: POTION, from: Source::Slot(0) }));
        assert_eq!(v.slots[0], 0);
        // The last Ether of the stock used: the stock list is empty, back to the owners.
        let mut s = Screen::new();
        pick(&mut s, &mut v, LOW, 1);
        pick(&mut s, &mut v, LOW, 0);
        assert_eq!(pick(&mut s, &mut v, LOW, 0), Outcome::Done(Effect::Used { item: ETHER, from: Source::Stock(ETHER) }));
        assert_eq!(v.stock_of(ETHER), 0);
        assert_eq!(s.mode(), Mode::Owners);
        // Discard: yes / no; no goes back to the commands.
        let mut v = inv(&[HI_POTION, 0, 0], &[]);
        let mut s = Screen::new();
        pick(&mut s, &mut v, LOW, 0);
        pick(&mut s, &mut v, LOW, 1);
        assert_eq!(pick(&mut s, &mut v, LOW, 3), Outcome::Opened);
        assert_eq!(s.cursor(&v, LOW), 0, "opens on yes");
        assert_eq!(pick(&mut s, &mut v, LOW, 1), Outcome::Back);
        assert_eq!(s.mode(), Mode::Commands(Source::Slot(0)));
        pick(&mut s, &mut v, LOW, 3);
        assert_eq!(pick(&mut s, &mut v, LOW, 0), Outcome::Done(Effect::Discarded { item: HI_POTION, from: Source::Slot(0) }));
        assert_eq!(v.slots[0], 0);
        // A greyed command: refused, nothing changes.
        pick(&mut s, &mut v, LOW, 1);
        assert_eq!(pick(&mut s, &mut v, LOW, 3), Outcome::Refused);
        assert_eq!(s.source(), Some(Source::Slot(0)));
    }

    #[test]
    fn status_help_and_row_texts() {
        let v = inv(&[POTION, 0, 0], &[(ETHER, 5)]);
        let mut s = Screen::new();
        assert_eq!(s.status(&v, LOW), (Owner::Sora, 0));
        s.set_cursor(1);
        assert_eq!(s.status(&v, LOW), (Owner::Stock, 0));
        assert_eq!(s.help(&s.rows(&v, LOW), 1, None), text::OWNERS_HELP[1]);
        let mut v2 = v;
        s.set_cursor(0);
        s.confirm(&mut v2, LOW);
        assert_eq!(s.status(&v, LOW), (Owner::Sora, 0), "the Keyblade row: the page shows the Keyblade");
        s.set_cursor(1);
        assert_eq!(s.status(&v, LOW), (Owner::Sora, POTION));
        s.set_cursor(2);
        assert_eq!(s.status(&v, LOW), (Owner::Sora, 0), "an empty slot");
        s.confirm(&mut v2, LOW);
        let rows = s.rows(&v, LOW);
        assert_eq!(s.help(&rows, 0, None), text::USE_HELP[1], "Use greyed on an empty slot");
        assert_eq!(s.help(&rows, 2, None), text::EQUIP_HELP);
        assert_eq!(s.help(&rows, 2, Some(Msg::NoStock)), text::NO_STOCK);
        assert_eq!(s.status(&v, LOW), (Owner::Sora, 0));
        assert_eq!(stocked_text(b"Ether\0", 5), b"Ether x5\0".to_vec());
        assert_eq!(stocked_text(b"Ether\0", 1), b"Ether\0".to_vec());
        assert_eq!(stocked_text(b"Ether", 12), b"Ether x12\0".to_vec());
        assert_eq!(grey_colour(0x8080_8080), 0x8040_4040);
        assert_eq!(text::english(MEGALIXIR), b"Megalixir\0");
    }

    #[test]
    fn can_use_is_kh1s_menu_rule() {
        let hp = Ctx { hp_low: true, mp_low: false };
        let mp = Ctx { hp_low: false, mp_low: true };
        assert!(can_use(POTION, hp) && !can_use(POTION, mp));
        assert!(can_use(MEGA_ETHER, mp) && !can_use(ETHER, hp));
        assert!(can_use(ELIXIR, hp) && can_use(MEGALIXIR, mp) && !can_use(ELIXIR, FULL));
        assert!(!can_use(5, LOW));
    }

    #[test]
    fn texts_fit_the_help_box() {
        let all = [
            text::LABEL,
            text::HELP,
            text::SORA,
            text::STOCK,
            text::EMPTY,
            text::USE,
            text::MOVE,
            text::EQUIP,
            text::REMOVE,
            text::DISCARD,
            text::YES,
            text::NO,
            text::NONE,
            text::KEYBLADE_SLOT_HELP,
            text::SLOT_HELP,
            text::STOCK_HELP,
            text::EQUIP_HELP,
            text::REMOVE_HELP,
            text::EQUIP_REMOVE_GREY,
            text::MOVE_TO_HELP,
            text::PICK_SLOT_HELP,
            text::PICK_STOCK_HELP,
            text::DISCARD_ASK,
            text::STOCK_FULL,
            text::NO_STOCK,
        ]
        .into_iter()
        .chain(text::STATUS)
        .chain(text::OWNERS_HELP)
        .chain(text::USE_HELP)
        .chain(text::MOVE_HELP)
        .chain(text::DISCARD_HELP);
        for s in all {
            assert_eq!(s.last(), Some(&0));
            assert!(s[..s.len() - 1].iter().all(|&b| (0x20..0x7F).contains(&b) && b != b'%'), "{:?}", std::str::from_utf8(s));
            assert!(s.len() - 1 <= crate::menu::HELP_MAX, "{:?}", std::str::from_utf8(s));
        }
    }
}
