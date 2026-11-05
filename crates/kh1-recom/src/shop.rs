//! M8d: KH1's items in Re:CoM's own Moogle shop screens (`docs/recom-m8-items.md` § M8d). The
//! pure rules; Re:CoM's side is `win::shop`.
//!
//! At Combat = KH1 in Sora's story Re:CoM's shop runs as Re:CoM's: its two-Moogle menu (mode 3)
//! unpatched, the left Moogle its pack screen (mode 4), the right one its card trade (mode 5). The
//! pack screen gets an **Items tab first**, then Re:CoM's four pack tabs (owner, 2026-11-04: "Items
//! tab, then packs"), and opens on it: its cells are Re:CoM's own packs (Re:CoM's constructor,
//! price plates) with the pack art hidden and Re:CoM's own item card picture (`C2DCard`) on them;
//! one item per confirm by KH1's pickup rule, paid through Re:CoM's payment. Stock by Re:CoM's floor
//! tiers, prices in Moogle Points (owner).
//!
//! Re:CoM's input order, sounds, moves and places are Re:CoM's (`0x1403A6930`, Checked (decomp +
//! disasm + exe)); the Items tab replays them for its cells (8a.3's design, M8d's tab place).

use crate::items::{self as it, id};

// ---- Floors and stock ----

/// Re:CoM's floor tiers (`0x1403A5B10`: the pack screen reads `MogFloor`'s row for floor 1,
/// 2..6, 7..10 or 11+). Checked (decomp).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// 1F.
    First,
    /// 2F..6F.
    Early,
    /// 7F..10F.
    Mid,
    /// 11F and up.
    Late,
}

/// The floor from Re:CoM's floor byte (`0x1402337C0` = `*[0x14087B110]`): byte + 1, and 0xFF
/// counts as 10 (floor 11), as `0x1403A5B10` does. Checked (decomp + exe).
pub fn floor_of(byte: u8) -> u32 {
    if byte == 0xFF {
        11
    } else {
        byte as u32 + 1
    }
}

/// Re:CoM's tier of a floor (`0x1403A5B10`: `< 2`, `< 7`, `< 11`, else). Checked (decomp).
pub fn tier(floor: u32) -> Tier {
    match floor {
        0..=1 => Tier::First,
        2..=6 => Tier::Early,
        7..=10 => Tier::Mid,
        _ => Tier::Late,
    }
}

/// What the shop can sell (owner, 2026-11-04). The Tent and the Camping Set (M8c stage 2) were
/// removed from the game in M8d (owner: "just remove tents and camping sets from the game").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ware {
    Potion,
    HiPotion,
    Ether,
    Elixir,
}

impl Ware {
    /// Our item id (None would be a ware not built yet: none now).
    pub fn item(self) -> Option<u8> {
        match self {
            Ware::Potion => Some(id::POTION),
            Ware::HiPotion => Some(id::HI_POTION),
            Ware::Ether => Some(id::ETHER),
            Ware::Elixir => Some(id::ELIXIR),
        }
    }

    /// The buy price in Moogle Points (owner, 2026-11-04).
    pub fn price(self) -> i32 {
        match self {
            Ware::Potion => 10,
            Ware::HiPotion => 30,
            Ware::Ether => 40,
            Ware::Elixir => 200,
        }
    }

    /// The ware of an item.
    pub fn of(item: u8) -> Option<Ware> {
        STOCK.iter().map(|s| s.0).find(|w| w.item() == Some(item))
    }
}

/// The stock, in list order, with the first tier each ware is sold at (owner, 2026-11-04): 1F
/// Potion, Ether; from 2F also Hi-Potion; from 7F also Elixir. KH1's item order. Lucky Bounty at
/// KH1 uses these tiers too (`crate::party::lucky_bounty`).
pub const STOCK: [(Ware, Tier); 4] = [
    (Ware::Potion, Tier::First),
    (Ware::HiPotion, Tier::Early),
    (Ware::Ether, Tier::First),
    (Ware::Elixir, Tier::Mid),
];

/// The items the Buy list shows on a tier: the built wares sold there, in [`STOCK`]'s order.
pub fn buy_list(t: Tier) -> Vec<u8> {
    STOCK.iter().filter(|s| s.1 <= t).filter_map(|s| s.0.item()).collect()
}

/// An item's buy price, when the shop sells it.
pub fn buy_price(item: u8) -> Option<i32> {
    Ware::of(item).map(Ware::price)
}

/// KH1's sell price ÷ buy price (`btltbl.bin` item rows: Potion 12 / 60, Hi-Potion 30 / 150,
/// Ether 40 / 200). Checked (files).
pub const SELL_DIVISOR: i32 = 5;

/// An item's sell price: the shop's buy price ÷ 5 (owner); an item the shop never sells (the
/// Mega items, which KH1 also never sells: buy price 0) takes KH1's own sell price (Mega-Potion
/// 40, Mega-Ether 60, Megalixir 100; `btltbl.bin` `+0xA`, Checked (files)).
pub fn sell_price(item: u8) -> Option<i32> {
    if let Some(p) = buy_price(item) {
        return Some(p / SELL_DIVISOR);
    }
    let d = it::data(item)?;
    match d.buy {
        0 => Some(d.sell as i32),
        b => Some(b as i32 / SELL_DIVISOR),
    }
}

/// How many of an item Sora has: the stock plus the slots holding it.
pub fn owned(inv: &it::Inventory, item: u8) -> u32 {
    inv.stock_of(item) as u32 + inv.filled().iter().filter(|s| s.1 == item).count() as u32
}

/// The Sell list: every item in play Sora has (`id::IN_PLAY`), with its count, in KH1's item order.
/// A save's removed items (the Mega items, the Tent, the Camping Set) are converted before the shop
/// reads the store (`crate::items::convert_removed`): they never show at Combat = KH1.
pub fn sell_list(inv: &it::Inventory) -> Vec<(u8, u32)> {
    id::IN_PLAY.iter().map(|&i| (i, owned(inv, i))).filter(|s| s.1 > 0 && sell_price(s.0).is_some()).collect()
}

// ---- Buying and selling ----

/// Re:CoM's Moogle Point cap (`0x14023A1D0`: 0 ..= 99999). Checked (decomp).
pub const MAX_POINTS: i32 = 99_999;

/// Points after paying (`0x1403A7041`: points − price, clamped to 0 ..= 99999).
pub fn pay(points: i32, price: i32) -> i32 {
    (points - price).clamp(0, MAX_POINTS)
}

/// Points after a sale (Re:CoM's setter clamps to 0 ..= 99999).
pub fn earn(points: i32, price: i32) -> i32 {
    points.saturating_add(price).clamp(0, MAX_POINTS)
}

/// Re:CoM's spent-points tally on a buy (`0x1403A7076`: system variable (0, 0xB6) += price; its
/// trophy the first time it reaches 20000, while the trophy bit `0x140AF7389 & 0x10` is clear).
pub const SPENT_VAR: (u32, u32) = (0, 0xB6);
pub const SPENT_TROPHY_AT: i32 = 20_000;
pub const SPENT_TROPHY: u32 = 99;

/// The tally after a buy and whether the trophy unlocks now.
pub fn spent(before: i32, price: i32) -> (i32, bool) {
    let after = before + price;
    (after, after >= SPENT_TROPHY_AT && before < SPENT_TROPHY_AT)
}

/// A buy's outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Buy {
    /// Paid `price`; Sora's points are now `points`; the item went `to`.
    Bought { item: u8, price: i32, points: i32, to: it::Picked },
    /// Not enough Moogle Points (nothing changes).
    Poor { item: u8, price: i32 },
    /// No empty slot and 99 in stock (nothing paid).
    NoRoom { item: u8 },
    /// The shop doesn't sell it.
    NotSold,
}

/// One item: the price check first (Re:CoM's pack rule), then KH1's pickup on `inv` (refused:
/// nothing paid), then the payment.
pub fn buy(item: u8, points: i32, inv: &mut it::Inventory) -> Buy {
    // `Ware::of` finds only built wares (their item ids).
    let Some(price) = buy_price(item) else { return Buy::NotSold };
    if price > points {
        return Buy::Poor { item, price };
    }
    match inv.pick_up(item) {
        Some(to) => Buy::Bought { item, price, points: pay(points, price), to },
        None => Buy::NoRoom { item },
    }
}

/// Where a sold item came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoldFrom {
    /// The stock, which now holds this many.
    Stock(u8),
    /// Slot k (now empty).
    Slot(u8),
}

/// A sale's outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sell {
    Sold { item: u8, price: i32, points: i32, from: SoldFrom, left: u32 },
    /// Sora has none (or it has no price).
    None,
}

/// One item: from the stock first, else from the last slot holding it (owner, 2026-11-04: "stock
/// then slots"; KH1's sell source not read). The points go up by the sell price.
pub fn sell(item: u8, points: i32, inv: &mut it::Inventory) -> Sell {
    let Some(price) = sell_price(item) else { return Sell::None };
    let from = if inv.stock_of(item) > 0 {
        let s = inv.stock_mut(item).expect("an item in stock has a stock byte");
        *s -= 1;
        SoldFrom::Stock(*s)
    } else if let Some(&(k, _)) = inv.filled().iter().rev().find(|s| s.1 == item) {
        inv.slots[k as usize] = 0;
        SoldFrom::Slot(k)
    } else {
        return Sell::None;
    };
    Sell::Sold { item, price, points: earn(points, price), from, left: owned(inv, item) }
}

// ---- The pack screen's Items tab ----

/// The Items tab's index in the pack screen's tab field `+0x280` (Re:CoM's are 0..3; drawn first).
pub const ITEMS_TAB: i32 = 4;
/// Re:CoM's tab count (`0x1403A6930`: wrap at > 3 and < 0). Checked (disasm `0x1403A6D3A`).
pub const RECOM_TABS: i32 = 4;
/// Cells in the grid (2 × 2).
pub const CELLS: usize = 4;

/// Re:CoM's item card of an item, drawn as its picture (`BattleCardList`: 0x2A Potion, 0x2B
/// Hi-Potion, 0x2D Ether, 0x2F Elixir; the card-info table `0x1405256D0` names them). Checked
/// (exe + files).
pub fn card_kind(item: u8) -> Option<u8> {
    Some(match item {
        id::POTION => 0x2A,
        id::HI_POTION => 0x2B,
        id::ETHER => 0x2D,
        id::ELIXIR => 0x2F,
        _ => return None,
    })
}

/// Re:CoM's card value for a card picture (`C2DCard`, `0x140243560`): the kind in byte 0, byte 1 0
/// (not premium), byte 2 0xFF (negative: no number drawn; as the shop's own number-less card
/// `0x11FF0001` in `0x14045BF50`), byte 3 the kind's class from Re:CoM's table (`0x14068E0B0 +
/// kind × 16`; 0x44 for the item cards). Checked (decomp + exe).
pub fn card_value(kind: u8, class: u8) -> u32 {
    kind as u32 | 0xFF << 16 | (class as u32) << 24
}

/// Re:CoM's cell moves (`0x14065BA70`, `[dir + cell × 4]`; 0x40 = leave the grid upwards).
/// Checked (exe).
pub const MOVES: [i8; 16] = [2, 1, 64, 0, 2, 0, 64, -1, 0, 1, -2, 0, 0, 0, -2, -1];

/// Re:CoM's direction order and indices in the grid (`0x1403A6E54`: Down 0x204000, Right 0x802000,
/// Up 0x101000, Left 0x408000).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Down = 0,
    Right = 1,
    Up = 2,
    Left = 3,
}

/// Where a direction takes the grid cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridMove {
    /// No move (no sound).
    Stay,
    To(i32),
    /// Back to the tab row.
    Leave,
}

/// Re:CoM's move rule (`0x1403A6F0D..0x1403A6F60`): the table's step; ≥ 0x40 leaves; a step of 0
/// or onto a cell at or past the count stays.
pub fn grid_move(cell: i32, dir: Dir, count: i32) -> GridMove {
    let Some(&step) = usize::try_from(dir as i32 + cell * 4).ok().and_then(|i| MOVES.get(i)) else { return GridMove::Stay };
    let step = step as i32;
    if step >= 0x40 {
        GridMove::Leave
    } else if step != 0 && cell + step < count {
        GridMove::To(cell + step)
    } else {
        GridMove::Stay
    }
}

/// One frame's presses, Re:CoM's reads (the 0x80 button `0x14029B2F0(0x80, 0)`, cancel
/// `0x14029B410`, confirm `0x14029B490`, the directions `0x14029B2E0(mask, 0)`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pad {
    pub other: bool,
    pub cancel: bool,
    pub confirm: bool,
    pub down: bool,
    pub right: bool,
    pub up: bool,
    pub left: bool,
}

impl Pad {
    /// Re:CoM's grid order: the first pressed of Down, Right, Up, Left.
    fn grid_dir(&self) -> Option<Dir> {
        [(self.down, Dir::Down), (self.right, Dir::Right), (self.up, Dir::Up), (self.left, Dir::Left)].into_iter().find(|p| p.0).map(|p| p.1)
    }
}

/// From one of Re:CoM's tabs: does this frame's input go to the Items tab (Left on tab 0: the Items
/// tab is drawn before it; Right on tab 3: the row wraps)? Re:CoM's earlier reads win: the 0x80
/// button and cancel, and Left over Right. `locked` = Re:CoM's input lock `+0x274 > 0`.
pub fn enters_items(tab: i32, in_grid: bool, pad: &Pad, locked: bool) -> bool {
    if locked || in_grid || pad.other || pad.cancel {
        return false;
    }
    if pad.left {
        return tab == 0;
    }
    pad.right && tab == RECOM_TABS - 1
}

/// What one frame of the Items tab does (Re:CoM's case 2 with the Items tab's cells).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Nothing,
    /// Leave the pack screen (sound, then `+0x25C` = mode, state 100).
    Leave { sound: u32, mode: i32 },
    /// To Re:CoM's tab (sound 1).
    ToTab(i32),
    /// Into the grid at cell 0 (sound 1).
    IntoGrid,
    /// Back to the tab row (sound 1).
    OutOfGrid,
    /// To another cell (sound 1).
    Move(i32),
    /// Confirm on a cell: the buy (or its refusal).
    Confirm(i32),
}

/// The Items tab's frame, in Re:CoM's order; `count` = the cells shown. Left goes to Re:CoM's tab
/// 3 (the row wraps), Right to tab 0 (the next one drawn).
pub fn items_step(in_grid: bool, cell: i32, count: i32, pad: &Pad, locked: bool) -> Step {
    if locked {
        return Step::Nothing;
    }
    if pad.other {
        return Step::Leave { sound: sound::OTHER, mode: mode::TRADE };
    }
    if !in_grid {
        if pad.cancel {
            return Step::Leave { sound: sound::BACK, mode: mode::MENU };
        }
        if pad.left {
            return Step::ToTab(RECOM_TABS - 1);
        }
        if pad.right {
            return Step::ToTab(0);
        }
        if pad.down || pad.confirm {
            return Step::IntoGrid;
        }
        return Step::Nothing;
    }
    if pad.confirm {
        return Step::Confirm(cell);
    }
    if pad.cancel {
        return Step::OutOfGrid;
    }
    match pad.grid_dir().map(|d| grid_move(cell, d, count)) {
        Some(GridMove::Leave) => Step::OutOfGrid,
        Some(GridMove::To(c)) => Step::Move(c),
        _ => Step::Nothing,
    }
}

/// The Items tab's line in the Moogle's text window (in place of Re:CoM's card-pack lines,
/// messages 4 / 6: `0x1403A5AA0`). On the tab row the tab's line; on a cell what the item does,
/// its price and how many Sora has, or why the last confirm was refused.
pub fn help_text(in_grid: bool, item: Option<u8>, note: Option<Note>, inv: &it::Inventory) -> Vec<u8> {
    let s = match (note, in_grid, item) {
        (Some(Note::Poor), ..) => text::POOR.to_string(),
        (Some(Note::NoRoom), ..) => text::NO_ROOM.to_string(),
        (_, true, Some(i)) => format!("{} {} Moogle Points. Sora has {}.", text::what(i), buy_price(i).unwrap_or(0), owned(inv, i)),
        _ => text::TAB.to_string(),
    };
    c_text(&s)
}

/// A refusal shown on the line until the next move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    Poor,
    NoRoom,
}

pub mod text {
    /// The Items tab's line (the owner's example: "Buy items with Moogle Points.").
    pub const TAB: &str = "Buy items with Moogle Points.";
    pub const POOR: &str = "Not enough Moogle Points.";
    pub const NO_ROOM: &str = "Sora can't carry any more of this.";

    /// What an item does (Sora only: our items heal Sora); ours in English (KH1's help texts are
    /// not in the asset step).
    pub fn what(item: u8) -> &'static str {
        use crate::items::id::*;
        match item {
            POTION | MEGA_POTION => "Restores HP.",
            HI_POTION => "Restores more HP.",
            ETHER | MEGA_ETHER => "Restores MP.",
            ELIXIR | MEGALIXIR => "Restores all HP and MP.",
            _ => "",
        }
    }
}

/// ASCII bytes + NUL (anything outside printable ASCII and `\n` becomes `?`).
pub fn c_text(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.bytes().map(|b| if b == b'\n' || (0x20..0x7F).contains(&b) { b } else { b'?' }).collect();
    v.push(0);
    v
}

// ---- Places ----

/// Re:CoM's tab x (`0x14065BA40`: 64, 128, 192, 256), the tab cursor's y (`0x14065BCE0`), the tab
/// button message's y (`0x14065BCE4`), the cursor's offsets (`0x1406C1204` 16, `0x14050DCA4` 32),
/// the grid cells' places (`0x14065BA50` x, `0x14065BA60` y). Checked (exe).
pub const RECOM_TAB_X: [f32; 4] = [64.0, 128.0, 192.0, 256.0];
pub const TAB_CURSOR_Y: f32 = 53.0;
pub const TAB_MESSAGE_Y: f32 = 69.0;
pub const CURSOR_DX: f32 = 16.0;
pub const CURSOR_DY: f32 = 32.0;
pub const CELL_X: [f32; 4] = [74.0, 224.0, 74.0, 224.0];
pub const CELL_Y: [f32; 4] = [167.0, 167.0, 252.0, 252.0];

/// The tab row's places: with the asset step's five-tab art, the Items tab at place 0 and Re:CoM's
/// tab t at place t + 1, 52 apart from Re:CoM's first x (64, 116, 168, 220, 272: the art's
/// notches); without it Re:CoM's places and the Items tab after them at 320 (Re:CoM's step of 64
/// continued: **Assumption**, no art to place it by).
pub fn tab_x(tab: i32, five: bool) -> f32 {
    let place = |p: i32| RECOM_TAB_X[0] + tabs::STEP as f32 * p as f32;
    match (tab, five) {
        (ITEMS_TAB, true) => place(0),
        (ITEMS_TAB, false) => RECOM_TAB_X[3] + 64.0,
        (t, true) => place(t.clamp(0, 3) + 1),
        (t, false) => RECOM_TAB_X[t.clamp(0, 3) as usize],
    }
}

/// The asset step's five-tab groups in `m_shop:01` (`kh1-recom-assets` `shop_tabs`,
/// `kh1-shop-tabs.json`).
pub mod tabs {
    use crate::json;
    pub const MANIFEST: &str = "kh1-shop-tabs.json";
    pub const FORMAT: &str = "kh1-recom-assets shop-tabs 1";
    pub const SET: &str = "m_shop:01";
    /// Re:CoM Steam's groups in `m_shop:01` (Checked (files)).
    pub const BASE_GROUPS: u32 = 241;
    pub const COUNT: u32 = 13;
    pub const TABS: u32 = 5;
    pub const STEP: i32 = 52;
    /// Our groups' offsets from the first: the window with place p raised (0..5), its close
    /// (5..10), the icons' appear / loop / close.
    pub const WINDOW: u32 = 0;
    pub const CLOSE: u32 = 5;
    pub const ICONS_IN: u32 = 10;
    pub const ICONS: u32 = 11;
    pub const ICONS_OUT: u32 = 12;
    /// Re:CoM's own: the windows by tab (`0x140690420`), their closes (`0x140690428`), the icons
    /// (mode 4's init `0x1403A4A90`: 0x38 / 0x39 / 0x3A). Checked (exe + decomp).
    pub const RECOM_WINDOW: [i16; 4] = [66, 68, 70, 72];
    pub const RECOM_CLOSE: [i16; 4] = [67, 69, 71, 73];
    pub const RECOM_ICONS: [i16; 3] = [0x38, 0x39, 0x3A];

    /// Reads the manifest: our first group, after checking it is this DLL's layout.
    pub fn parse(text: &str) -> Result<u32, String> {
        let v = json::parse(text)?;
        if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
            return Err(format!("not a \"{FORMAT}\" manifest (an older asset step? run it again)"));
        }
        if v.get("set").and_then(|s| s.as_str()) != Some(SET) {
            return Err(format!("not the set {SET}"));
        }
        let n = |k: &str| v.get(k).and_then(|x| x.as_u32()).ok_or_else(|| format!("no \"{k}\""));
        let (base, first, count, tabs, step) = (n("base_groups")?, n("first")?, n("count")?, n("tabs")?, n("step")?);
        if base != BASE_GROUPS || first != BASE_GROUPS || count != COUNT || tabs != TABS || step != STEP as u32 {
            return Err(format!("groups {first}+{count} ({tabs} tabs, step {step}, base {base}); this DLL expects {BASE_GROUPS}+{COUNT} ({TABS} tabs, step {STEP})"));
        }
        Ok(first)
    }

    /// Re:CoM's tab tables at Combat = KH1 with the art: tab t's x, window and close at place t + 1.
    pub fn kh1_tables(first: u32) -> ([f32; 4], [i16; 4], [i16; 4]) {
        let x = std::array::from_fn(|t| super::tab_x(t as i32, true));
        let w = std::array::from_fn(|t| (first + WINDOW + t as u32 + 1) as i16);
        let c = std::array::from_fn(|t| (first + CLOSE + t as u32 + 1) as i16);
        (x, w, c)
    }
}

// ---- Re:CoM's shop: modes, sounds, help ids ----

/// `CMoogleShopCore`'s modes (`+0x258`; records at `+0x98 + 0x40 · mode`, Checked (decomp
/// `0x1403A4220`)).
pub mod mode {
    /// Loading, then mode 3.
    pub const LOAD: i32 = 0;
    /// The first visit's free pack.
    pub const FREE_PACK: i32 = 1;
    /// No room for 5 cards: offers the card trade only.
    pub const NO_ROOM: i32 = 2;
    /// The two-Moogle menu.
    pub const MENU: i32 = 3;
    /// The pack screen.
    pub const PACKS: i32 = 4;
    /// The card trade.
    pub const TRADE: i32 = 5;
    /// Leaving the shop.
    pub const LEAVE: i32 = 6;
}

/// Re:CoM's room rule (`0x1403A53F0`, `0x1403A752A`: `u16 [*0x14087A7E8 + 6] ≥ [shop+0x350]+0x44 + 5`).
pub fn room_for_pack(limit: u16, cards: i32) -> bool {
    limit as i32 >= cards + 5
}

/// The card trade's Square at Combat = KH1: the pack screen (its Items tab) also without room for
/// 5 cards (Re:CoM's needs room, `0x1403A752A`: there the pack screen would be out of reach and
/// items could not be bought with a full deck). The packs keep Re:CoM's own refusal there (help
/// line 5, sound 5). Re:CoM's mode-2 rule (no room: the two-Moogle menu is replaced by the card
/// trade offer) is unchanged.
pub fn trade_square_opens_packs(kh1: bool, room: bool) -> bool {
    kh1 || room
}

/// Re:CoM's sounds here (`0x1402AA360(id, −1)`; Checked (disasm `0x1403A6930`, `0x1403A73E0`)).
pub mod sound {
    pub const MOVE: u32 = 1;
    pub const BUY: u32 = 2;
    /// The 0x80 button's (to the other screen).
    pub const OTHER: u32 = 3;
    pub const BACK: u32 = 4;
    pub const ERROR: u32 = 5;
    /// The card trade's sale (mode 5, `0x1403A809A`).
    pub const SALE: u32 = 6;
    /// Played right after `BUY` on a pack buy (`0x1403A6DFE`).
    pub const BUY_TOO: u32 = 0x58;
}

/// Re:CoM's help message ids (`0x1403A5AA0` → `0x1404573A0`): 4 the usual line, 5 no room for 5
/// cards, 6 not enough Moogle Points.
pub mod help {
    pub const USUAL: i32 = 4;
    pub const NO_ROOM: i32 = 5;
    pub const POOR: i32 = 6;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::{Inventory, Picked};

    fn pad(f: impl FnOnce(&mut Pad)) -> Pad {
        let mut p = Pad::default();
        f(&mut p);
        p
    }

    #[test]
    fn floors_and_tiers_are_recoms() {
        assert_eq!(floor_of(0), 1);
        assert_eq!(floor_of(5), 6);
        assert_eq!(floor_of(0xFF), 11);
        assert_eq!([1, 2, 6, 7, 10, 11, 13].map(tier), [Tier::First, Tier::Early, Tier::Early, Tier::Mid, Tier::Mid, Tier::Late, Tier::Late]);
    }

    #[test]
    fn stock_by_floor() {
        use id::*;
        // The owner's: 1F Potion, Ether; 2F+ Hi-Potion; 7F+ Elixir.
        assert_eq!(buy_list(tier(1)), [POTION, ETHER]);
        assert_eq!(buy_list(tier(2)), [POTION, HI_POTION, ETHER]);
        assert_eq!(buy_list(tier(6)), [POTION, HI_POTION, ETHER]);
        assert_eq!(buy_list(tier(7)), [POTION, HI_POTION, ETHER, ELIXIR]);
        assert_eq!(buy_list(tier(13)), [POTION, HI_POTION, ETHER, ELIXIR]);
        assert!(STOCK.iter().all(|s| s.0.item().is_some()), "every ware is built");
        // The removed items (Mega, Tent, Camping Set) are never in the shop.
        for t in [Tier::First, Tier::Early, Tier::Mid, Tier::Late] {
            for m in id::REMOVED {
                assert!(!buy_list(t).contains(&m));
            }
        }
    }

    #[test]
    fn prices() {
        use id::*;
        assert_eq!([POTION, HI_POTION, ETHER, ELIXIR].map(buy_price), [Some(10), Some(30), Some(40), Some(200)]);
        assert_eq!([TENT, CAMPING_SET].map(buy_price), [None, None]);
        assert_eq!([POTION, HI_POTION, ETHER, ELIXIR].map(sell_price), [Some(2), Some(6), Some(8), Some(40)]);
        // The Mega items: KH1 has no buy price, so KH1's sell price.
        assert_eq!([MEGA_POTION, MEGA_ETHER, MEGALIXIR].map(buy_price), [None, None, None]);
        assert_eq!([MEGA_POTION, MEGA_ETHER, MEGALIXIR].map(sell_price), [Some(40), Some(60), Some(100)]);
        // KH1's own ratio, which the owner's prices keep.
        for d in it::DATA.iter().filter(|d| d.buy != 0) {
            assert_eq!(d.buy as i32 / d.sell as i32, SELL_DIVISOR, "{}", d.id);
        }
        assert_eq!(sell_price(0), None);
        assert_eq!(sell_price(5), None);
    }

    #[test]
    fn buying_one_per_confirm_with_kh1s_pickup() {
        let mut inv = Inventory::new_game();
        assert_eq!(buy(id::POTION, 9, &mut inv), Buy::Poor { item: id::POTION, price: 10 });
        assert_eq!(inv, Inventory::new_game());
        assert_eq!(buy(id::POTION, 10, &mut inv), Buy::Bought { item: id::POTION, price: 10, points: 0, to: Picked::Slot(0) });
        assert_eq!(buy(id::ELIXIR, 500, &mut inv), Buy::Bought { item: id::ELIXIR, price: 200, points: 300, to: Picked::Slot(1) });
        assert_eq!(buy(id::ETHER, 500, &mut inv), Buy::Bought { item: id::ETHER, price: 40, points: 460, to: Picked::Slot(2) });
        assert_eq!(buy(id::ETHER, 500, &mut inv), Buy::Bought { item: id::ETHER, price: 40, points: 460, to: Picked::Stock(1) });
        inv.set_stock(id::POTION, 99);
        let before = inv;
        assert_eq!(buy(id::POTION, 500, &mut inv), Buy::NoRoom { item: id::POTION });
        assert_eq!(inv, before);
        assert_eq!(buy(id::MEGA_POTION, 500, &mut inv), Buy::NotSold);
        assert_eq!(inv, before);
    }

    #[test]
    fn the_tents_are_neither_bought_nor_sold() {
        let mut inv = Inventory::new_game();
        assert_eq!(buy(id::TENT, 500, &mut inv), Buy::NotSold);
        assert_eq!(buy(id::CAMPING_SET, 500, &mut inv), Buy::NotSold);
        // A stock holding them (before the store's conversion): no Sell rows.
        inv.set_stock(id::TENT, 2);
        inv.set_stock(id::CAMPING_SET, 1);
        inv.set_stock(id::ETHER, 1);
        assert_eq!(sell_list(&inv), [(id::ETHER, 1)]);
        assert_eq!(text::what(id::TENT), "");
    }

    #[test]
    fn selling_one_per_confirm() {
        let mut inv = Inventory::new_game();
        inv.slots[0] = id::POTION;
        inv.slots[2] = id::POTION;
        inv.set_stock(id::POTION, 1);
        inv.set_stock(id::ELIXIR, 1);
        assert_eq!(sell_list(&inv), [(id::POTION, 3), (id::ELIXIR, 1)]);
        // The stock first, then the last slot holding it.
        assert_eq!(sell(id::POTION, 100, &mut inv), Sell::Sold { item: id::POTION, price: 2, points: 102, from: SoldFrom::Stock(0), left: 2 });
        assert_eq!(sell(id::POTION, 102, &mut inv), Sell::Sold { item: id::POTION, price: 2, points: 104, from: SoldFrom::Slot(2), left: 1 });
        assert_eq!(sell(id::POTION, 104, &mut inv), Sell::Sold { item: id::POTION, price: 2, points: 106, from: SoldFrom::Slot(0), left: 0 });
        assert_eq!(sell(id::POTION, 106, &mut inv), Sell::None);
        // The row left the list.
        assert_eq!(sell_list(&inv), [(id::ELIXIR, 1)]);
        assert_eq!(sell(id::ELIXIR, MAX_POINTS - 10, &mut inv), Sell::Sold { item: id::ELIXIR, price: 40, points: MAX_POINTS, from: SoldFrom::Stock(0), left: 0 });
        assert!(sell_list(&inv).is_empty());
    }

    #[test]
    fn payment_and_tally() {
        assert_eq!(pay(100, 60), 40);
        assert_eq!(pay(10, 60), 0);
        assert_eq!(earn(99_990, 40), MAX_POINTS);
        assert_eq!(spent(19_990, 10), (20_000, true));
        assert_eq!(spent(20_010, 10), (20_020, false));
    }

    #[test]
    fn item_cards_and_values() {
        assert_eq!([id::POTION, id::HI_POTION, id::ETHER, id::ELIXIR].map(card_kind), [Some(0x2A), Some(0x2B), Some(0x2D), Some(0x2F)]);
        assert_eq!(card_kind(id::TENT), None);
        assert!(STOCK.iter().all(|s| s.0.item().and_then(card_kind).is_some()), "every ware has its card");
        assert_eq!(card_value(0x2A, 0x44), 0x44FF_002A);
        // The shop's own number-less card.
        assert_eq!(card_value(1, 0x11), 0x11FF_0001);
        assert!(STOCK.len() <= CELLS, "one page of cells");
    }

    #[test]
    fn recom_grid_moves() {
        assert_eq!(grid_move(0, Dir::Down, 4), GridMove::To(2));
        assert_eq!(grid_move(0, Dir::Right, 4), GridMove::To(1));
        assert_eq!(grid_move(0, Dir::Up, 4), GridMove::Leave);
        assert_eq!(grid_move(0, Dir::Left, 4), GridMove::Stay);
        assert_eq!(grid_move(1, Dir::Up, 4), GridMove::Leave);
        assert_eq!(grid_move(1, Dir::Left, 4), GridMove::To(0));
        assert_eq!(grid_move(3, Dir::Up, 4), GridMove::To(1));
        assert_eq!(grid_move(3, Dir::Left, 4), GridMove::To(2));
        assert_eq!(grid_move(2, Dir::Down, 4), GridMove::Stay);
        // 1F's two items: Ether's cell 1 is the last.
        assert_eq!(grid_move(0, Dir::Down, 2), GridMove::Stay);
        assert_eq!(grid_move(1, Dir::Down, 2), GridMove::Stay);
        assert_eq!(grid_move(0, Dir::Right, 2), GridMove::To(1));
        // 2F's three.
        assert_eq!(grid_move(0, Dir::Down, 3), GridMove::To(2));
        assert_eq!(grid_move(2, Dir::Right, 3), GridMove::Stay);
    }

    #[test]
    fn entering_the_items_tab() {
        let right = pad(|p| p.right = true);
        let left = pad(|p| p.left = true);
        // The Items tab is drawn first: Left from tab 0; the row wraps: Right from tab 3.
        assert!(enters_items(0, false, &left, false));
        assert!(enters_items(3, false, &right, false));
        assert!(!enters_items(2, false, &right, false));
        assert!(!enters_items(3, false, &left, false));
        assert!(!enters_items(0, false, &right, false));
        assert!(!enters_items(3, true, &right, false));
        assert!(!enters_items(3, false, &right, true));
        assert!(!enters_items(3, false, &pad(|p| { p.right = true; p.cancel = true }), false));
        assert!(!enters_items(3, false, &pad(|p| { p.right = true; p.other = true }), false));
        // Left is read before Right.
        assert!(!enters_items(3, false, &pad(|p| { p.right = true; p.left = true }), false));
        assert!(enters_items(0, false, &pad(|p| { p.right = true; p.left = true }), false));
    }

    #[test]
    fn tab_row() {
        assert_eq!(items_step(false, 0, 4, &pad(|p| p.left = true), false), Step::ToTab(3));
        assert_eq!(items_step(false, 0, 4, &pad(|p| p.right = true), false), Step::ToTab(0));
        assert_eq!(items_step(false, 0, 4, &pad(|p| p.down = true), false), Step::IntoGrid);
        assert_eq!(items_step(false, 0, 4, &pad(|p| p.confirm = true), false), Step::IntoGrid);
        assert_eq!(items_step(false, 0, 4, &pad(|p| p.up = true), false), Step::Nothing);
        assert_eq!(items_step(false, 0, 4, &pad(|p| p.cancel = true), false), Step::Leave { sound: sound::BACK, mode: mode::MENU });
        assert_eq!(items_step(false, 0, 4, &pad(|p| { p.other = true; p.cancel = true }), false), Step::Leave { sound: sound::OTHER, mode: mode::TRADE });
        assert_eq!(items_step(false, 0, 4, &pad(|p| p.left = true), true), Step::Nothing);
    }

    #[test]
    fn grid() {
        assert_eq!(items_step(true, 1, 4, &pad(|p| p.confirm = true), false), Step::Confirm(1));
        assert_eq!(items_step(true, 1, 4, &pad(|p| { p.confirm = true; p.cancel = true }), false), Step::Confirm(1));
        assert_eq!(items_step(true, 1, 4, &pad(|p| { p.confirm = true; p.other = true }), false), Step::Leave { sound: sound::OTHER, mode: mode::TRADE });
        assert_eq!(items_step(true, 2, 4, &pad(|p| { p.up = true; p.cancel = true }), false), Step::OutOfGrid);
        assert_eq!(items_step(true, 0, 4, &pad(|p| p.up = true), false), Step::OutOfGrid);
        assert_eq!(items_step(true, 0, 4, &pad(|p| p.right = true), false), Step::Move(1));
        assert_eq!(items_step(true, 2, 4, &pad(|p| p.up = true), false), Step::Move(0));
        assert_eq!(items_step(true, 2, 3, &pad(|p| p.right = true), false), Step::Nothing);
        assert_eq!(items_step(true, 0, 4, &pad(|p| { p.right = true; p.down = true }), false), Step::Move(2));
    }

    #[test]
    fn help_lines() {
        let mut inv = Inventory::new_game();
        inv.set_stock(id::ETHER, 2);
        assert_eq!(help_text(false, None, None, &inv), b"Buy items with Moogle Points.\0");
        assert_eq!(help_text(true, Some(id::ETHER), None, &inv), b"Restores MP. 40 Moogle Points. Sora has 2.\0");
        assert_eq!(help_text(true, Some(id::ETHER), Some(Note::Poor), &inv), b"Not enough Moogle Points.\0");
        assert_eq!(help_text(true, Some(id::ETHER), Some(Note::NoRoom), &inv), b"Sora can't carry any more of this.\0");
        for i in [id::POTION, id::HI_POTION, id::ETHER, id::ELIXIR] {
            let t = help_text(true, Some(i), None, &Inventory { stock: [99; crate::items::STOCK_IDS], ..Inventory::new_game() });
            assert!(t.len() - 1 <= crate::menu::HELP_MAX, "{i}");
        }
        assert_eq!(c_text("a\u{e9}b"), b"a??b\0");
    }

    #[test]
    fn tab_places() {
        // With the five-tab art: Items first, then Re:CoM's four, 52 apart.
        assert_eq!([ITEMS_TAB, 0, 1, 2, 3].map(|t| tab_x(t, true)), [64.0, 116.0, 168.0, 220.0, 272.0]);
        // Without: Re:CoM's places, the Items tab's cursor after them.
        assert_eq!([0, 1, 2, 3, ITEMS_TAB].map(|t| tab_x(t, false)), [64.0, 128.0, 192.0, 256.0, 320.0]);
        let (x, w, c) = tabs::kh1_tables(241);
        assert_eq!(x, [116.0, 168.0, 220.0, 272.0]);
        assert_eq!(w, [242, 243, 244, 245]);
        assert_eq!(c, [247, 248, 249, 250]);
        assert_eq!(tabs::CLOSE - tabs::WINDOW, tabs::TABS);
    }

    #[test]
    fn the_tabs_manifest() {
        let ok = r#"{"format": "kh1-recom-assets shop-tabs 1", "set": "m_shop:01", "base_groups": 241, "first": 241, "count": 13, "tabs": 5, "step": 52, "files": [], "sources": {}}"#;
        assert_eq!(tabs::parse(ok), Ok(241));
        assert!(tabs::parse(&ok.replace("\"step\": 52", "\"step\": 64")).is_err());
        assert!(tabs::parse(&ok.replace("shop-tabs 1", "shop-tabs 0")).is_err());
        assert!(tabs::parse(&ok.replace("\"first\": 241", "\"first\": 250")).is_err());
    }

    #[test]
    fn rooms_and_the_trade_square() {
        assert!(room_for_pack(100, 95));
        assert!(!room_for_pack(100, 96));
        assert!(trade_square_opens_packs(true, false), "KH1: items need no card room");
        assert!(trade_square_opens_packs(false, true));
        assert!(!trade_square_opens_packs(false, false), "Re:CoM's rule at Combat = Re:CoM");
    }
}
