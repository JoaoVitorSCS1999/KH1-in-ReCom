//! M8c: KH1's Moogle shop as a list (`docs/recom-m8-items.md` § "M8c: KH1 Moogle shop list").
//! The pure rules; Re:CoM's side is `win::shop`.
//!
//! At Combat = KH1 in Sora's story, Re:CoM's `CMoogleShopCore` mode 3 (its two-choice menu, card
//! packs or the card trade) runs our list instead: rows **Buy**, **Sell** and **Card Packs**,
//! drawn by Re:CoM's own shop pieces (a `CTextWnd` for the rows, the shop's finger cursor, its
//! text window, Moogle Point window and sounds). Card Packs goes on to Re:CoM's own mode 4 (the
//! pack screen) exactly as Re:CoM's mode 3 does, or to its mode 2 when Sora has no room for 5
//! more cards (Re:CoM's own rule, `0x1403A53F0`).
//!
//! The owner's shop (2026-11-04, `docs/design-philosophy-instructions.md` § KH1 Moogle shop):
//! stock by Re:CoM's floor tiers, prices in Moogle Points, one item per confirm with KH1's pickup
//! rule, sell price = buy ÷ 5.

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

/// What the shop can sell (owner, 2026-11-04). Tent and Camping Set are in the table but not
/// built yet (no item of ours): they are never listed until they are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ware {
    Potion,
    HiPotion,
    Ether,
    Elixir,
    /// Not built yet (KH1 item 142).
    Tent,
    /// Not built yet (KH1 item 143).
    CampingSet,
}

impl Ware {
    /// Our item id, or None while the item isn't built.
    pub fn item(self) -> Option<u8> {
        match self {
            Ware::Potion => Some(id::POTION),
            Ware::HiPotion => Some(id::HI_POTION),
            Ware::Ether => Some(id::ETHER),
            Ware::Elixir => Some(id::ELIXIR),
            Ware::Tent | Ware::CampingSet => None,
        }
    }

    /// The buy price in Moogle Points (owner, 2026-11-04).
    pub fn price(self) -> i32 {
        match self {
            Ware::Potion => 10,
            Ware::HiPotion => 30,
            Ware::Ether => 40,
            Ware::Elixir => 200,
            Ware::Tent => 100,
            Ware::CampingSet => 150,
        }
    }

    /// The ware of an item.
    pub fn of(item: u8) -> Option<Ware> {
        STOCK.iter().map(|s| s.0).find(|w| w.item() == Some(item))
    }
}

/// The stock, in list order, with the first tier each ware is sold at (owner, 2026-11-04): 1F
/// Potion, Ether, Tent; from 2F also Hi-Potion and Camping Set; from 7F also Elixir. The list keeps
/// KH1's item order (Potion, Hi-Potion, Ether, Elixir; then Tent, Camping Set).
pub const STOCK: [(Ware, Tier); 6] = [
    (Ware::Potion, Tier::First),
    (Ware::HiPotion, Tier::Early),
    (Ware::Ether, Tier::First),
    (Ware::Elixir, Tier::Mid),
    (Ware::Tent, Tier::First),
    (Ware::CampingSet, Tier::Early),
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

/// The Sell list: every item Sora has, with its count, in KH1's item order (`id::ALL`).
pub fn sell_list(inv: &it::Inventory) -> Vec<(u8, u32)> {
    id::ALL.iter().map(|&i| (i, owned(inv, i))).filter(|s| s.1 > 0 && sell_price(s.0).is_some()).collect()
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

/// One item: from the stock first, else from the last slot holding it (so the battle slots stay
/// filled longest; **our rule**: KH1's sell source not read). The points go up by the sell price.
pub fn sell(item: u8, points: i32, inv: &mut it::Inventory) -> Sell {
    let Some(price) = sell_price(item) else { return Sell::None };
    let from = if inv.stock_of(item) > 0 {
        let s = &mut inv.stock[item as usize - 1];
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

// ---- The list ----

/// The list's pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Root,
    Buy,
    Sell,
}

/// The root rows, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Root {
    Buy,
    Sell,
    CardPacks,
}

pub const ROOT: [Root; 3] = [Root::Buy, Root::Sell, Root::CardPacks];

/// One frame's presses: Re:CoM's reads in its mode 2 list's order (`0x1403A8770` case 1: cancel
/// `0x14029B410`, confirm `0x14029B490`, Up `0x101000`, Down `0x204000` through the repeat read).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pad {
    pub cancel: bool,
    pub confirm: bool,
    pub up: bool,
    pub down: bool,
}

/// What the list knows of Sora this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ctx {
    pub buy: Vec<u8>,
    pub sell: Vec<(u8, u32)>,
}

/// What a frame of the list does (the caller plays the sound and acts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Nothing,
    /// The cursor moved (sound 1).
    Moved,
    /// A page opened (sound 2).
    Opened(Page),
    /// Back to the root (sound 4).
    Back,
    /// Refused (sound 5): Sell with nothing to sell.
    Refused,
    /// Card Packs (sound 2): on to Re:CoM's mode 4 (or 2).
    CardPacks,
    /// Cancel on the root (sound 4): leave the shop as Re:CoM's mode 3 does.
    Leave,
    /// Confirm on a Buy row: this item.
    Buy(u8),
    /// Confirm on a Sell row: this item.
    Sell(u8),
}

/// The list's cursor state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Menu {
    pub page: Page,
    /// The root's cursor (kept while a page is open, and across the pack screen).
    pub root: usize,
    /// The open page's cursor.
    pub row: usize,
}

impl Menu {
    pub fn new(root: usize) -> Menu {
        Menu { page: Page::Root, root: root.min(ROOT.len() - 1), row: 0 }
    }

    /// The rows the page shows.
    pub fn len(&self, c: &Ctx) -> usize {
        match self.page {
            Page::Root => ROOT.len(),
            Page::Buy => c.buy.len(),
            Page::Sell => c.sell.len(),
        }
    }

    /// The highlighted row's index.
    pub fn cursor(&self) -> usize {
        match self.page {
            Page::Root => self.root,
            _ => self.row,
        }
    }

    fn set_cursor(&mut self, i: usize) {
        match self.page {
            Page::Root => self.root = i,
            _ => self.row = i,
        }
    }

    /// Keeps the cursor on a row after the list changed (a sale emptied a row); an emptied Sell
    /// list goes back to the root, on Sell.
    pub fn settle(&mut self, c: &Ctx) {
        let n = self.len(c);
        if n == 0 && self.page != Page::Root {
            self.page = Page::Root;
            self.row = 0;
            return;
        }
        if self.cursor() >= n {
            self.set_cursor(n.saturating_sub(1));
        }
    }

    /// One frame in Re:CoM's order: cancel, confirm, Up, Down (the first pressed); moves stop at
    /// the ends (Re:CoM's mode 2 list never wraps, `0x1403A8770`).
    pub fn step(&mut self, pad: &Pad, c: &Ctx) -> Act {
        self.settle(c);
        let n = self.len(c);
        if pad.cancel {
            return match self.page {
                Page::Root => Act::Leave,
                _ => {
                    self.page = Page::Root;
                    self.row = 0;
                    Act::Back
                }
            };
        }
        if pad.confirm {
            return match self.page {
                Page::Root => match ROOT[self.root] {
                    Root::Buy => {
                        self.page = Page::Buy;
                        self.row = 0;
                        Act::Opened(Page::Buy)
                    }
                    Root::Sell if c.sell.is_empty() => Act::Refused,
                    Root::Sell => {
                        self.page = Page::Sell;
                        self.row = 0;
                        Act::Opened(Page::Sell)
                    }
                    Root::CardPacks => Act::CardPacks,
                },
                Page::Buy => c.buy.get(self.row).map_or(Act::Nothing, |&i| Act::Buy(i)),
                Page::Sell => c.sell.get(self.row).map_or(Act::Nothing, |s| Act::Sell(s.0)),
            };
        }
        let i = self.cursor();
        if pad.up && i > 0 {
            self.set_cursor(i - 1);
            return Act::Moved;
        }
        if pad.down && i + 1 < n {
            self.set_cursor(i + 1);
            return Act::Moved;
        }
        Act::Nothing
    }
}

// ---- Texts (ASCII, NUL-terminated: Re:CoM's text set `0x140259000` draws them) ----

pub mod text {
    pub const ROOT: [&str; 3] = ["Buy", "Sell", "Card Packs"];
    pub const ROOT_HELP: [&str; 3] = ["Buy items with Moogle Points.", "Sell items for Moogle Points.", "Buy card packs."];
    pub const NOTHING_TO_SELL: &str = "Sora has nothing to sell.";
    pub const POOR: &str = "Not enough Moogle Points.";
    pub const NO_ROOM: &str = "Sora can't carry any more of this.";

    /// What an item does (Sora only: our items heal Sora).
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

/// A refusal shown on the help line until the next move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    Poor,
    NoRoom,
    NothingToSell,
}

/// The rows' text: one line per row (`\n` between them; Re:CoM's text window splits lines at
/// `\n`, `0x1402B1520`), NUL-terminated. `name` gives an item's name (KH1's word or English).
/// Buy rows read "name / price", Sell rows "name xN / price" (the owner's list: names, counts and
/// prices).
pub fn rows_text(m: &Menu, c: &Ctx, name: impl Fn(u8) -> String) -> Vec<u8> {
    let lines: Vec<String> = match m.page {
        Page::Root => text::ROOT.iter().map(|s| s.to_string()).collect(),
        Page::Buy => c.buy.iter().map(|&i| format!("{} / {}", name(i), buy_price(i).unwrap_or(0))).collect(),
        Page::Sell => c.sell.iter().map(|&(i, n)| format!("{} x{n} / {}", name(i), sell_price(i).unwrap_or(0))).collect(),
    };
    c_text(&lines.join("\n"))
}

/// The help line (the Moogle's text window): the highlighted row's, or the refusal.
pub fn help_text(m: &Menu, c: &Ctx, note: Option<Note>, inv: &it::Inventory) -> Vec<u8> {
    let s = match note {
        Some(Note::Poor) => text::POOR.to_string(),
        Some(Note::NoRoom) => text::NO_ROOM.to_string(),
        Some(Note::NothingToSell) => text::NOTHING_TO_SELL.to_string(),
        None => match m.page {
            Page::Root => text::ROOT_HELP[m.root.min(2)].to_string(),
            Page::Buy => c.buy.get(m.row).map_or(String::new(), |&i| {
                format!("{} {} Moogle Points. Sora has {}.", text::what(i), buy_price(i).unwrap_or(0), owned(inv, i))
            }),
            Page::Sell => c.sell.get(m.row).map_or(String::new(), |&(i, n)| {
                format!("{} Sells for {} Moogle Points. Sora has {n}.", text::what(i), sell_price(i).unwrap_or(0))
            }),
        },
    };
    c_text(&s)
}

/// ASCII bytes + NUL (anything outside printable ASCII and `\n` becomes `?`).
pub fn c_text(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.bytes().map(|b| if b == b'\n' || (0x20..0x7F).contains(&b) { b } else { b'?' }).collect();
    v.push(0);
    v
}

// ---- Re:CoM's shop: modes, sounds, places ----

/// `CMoogleShopCore`'s modes (`+0x258`; records at `+0x98 + 0x40 · mode`, Checked (decomp
/// `0x1403A4220`)).
pub mod mode {
    /// Loading, then mode 3.
    pub const LOAD: i32 = 0;
    /// The first visit's free pack.
    pub const FREE_PACK: i32 = 1;
    /// No room for 5 cards: offers the card trade only.
    pub const NO_ROOM: i32 = 2;
    /// The two-choice menu (ours at KH1).
    pub const MENU: i32 = 3;
    /// The pack screen.
    pub const PACKS: i32 = 4;
    /// The card trade.
    pub const TRADE: i32 = 5;
    /// Leaving the shop.
    pub const LEAVE: i32 = 6;
}

/// Where Card Packs goes: Re:CoM's pack screen, or its no-room screen when Sora's cards have no
/// room for 5 more (Re:CoM's mode 3 init's rule, `0x1403A53F0`).
pub fn card_packs_mode(room: bool) -> i32 {
    if room {
        mode::PACKS
    } else {
        mode::NO_ROOM
    }
}

/// Re:CoM's room rule (`0x1403A53F0`: `u16 [*0x14087A7E8 + 6] ≥ [shop+0x350]+0x44 + 5`).
pub fn room_for_pack(limit: u16, cards: i32) -> bool {
    limit as i32 >= cards + 5
}

/// Re:CoM's sounds here (`0x1402AA360(id, −1)`; Checked (disasm `0x1403A8770`, `0x1403A8EF0`,
/// `0x1403A6D74`, `0x1403A8095`)).
pub mod sound {
    pub const MOVE: u32 = 1;
    pub const CONFIRM: u32 = 2;
    pub const BACK: u32 = 4;
    pub const ERROR: u32 = 5;
    /// The card trade's sale (mode 5, `0x1403A809A`).
    pub const SALE: u32 = 6;
    /// Played right after `CONFIRM` on a pack buy (`0x1403A6DFE`).
    pub const BUY_TOO: u32 = 0x58;
}

/// The finger cursor's x and Re:CoM's row pitch in its mode 2 list (`0x1403A51C0`: x 126.0;
/// `0x14065BA30`: y 210.0, 233.0). Checked (decomp + exe).
pub const FINGER_X: f32 = 126.0;
pub const RECOM_ROW_Y: [f32; 2] = [210.0, 233.0];
pub const ROW_PITCH: f32 = RECOM_ROW_Y[1] - RECOM_ROW_Y[0];
/// Our first row's y: above Re:CoM's mode 2 rows so nine rows fit on the 512 × 384 screen
/// (**Assumption**: the place; to judge in game).
pub const FIRST_ROW_Y: f32 = 96.0;
/// The rows' text window (a `CTextWnd` opened by `0x1402B1420`): left edge just right of the
/// finger, the top half a row above the first row's y, the width, the line pitch (**Assumption**:
/// the text's first line sits at the window's top; to judge in game).
pub const ROWS_X: i32 = 142;
pub const ROWS_WIDTH: i32 = 240;
pub const ROW_LINE: u32 = ROW_PITCH as u32;
/// Most rows any page has: 7 sellable items + Tent and Camping Set later.
pub const MAX_ROWS: usize = 9;

/// The finger's y on row `i`.
pub fn row_y(i: usize) -> f32 {
    FIRST_ROW_Y + ROW_PITCH * i.min(MAX_ROWS - 1) as f32
}

/// The rows' window: (x, y) and (w, h) for `n` rows.
pub fn rows_window(n: usize) -> ([i32; 2], [i32; 2]) {
    let n = n.clamp(1, MAX_ROWS) as i32;
    ([ROWS_X, (FIRST_ROW_Y - ROW_PITCH / 2.0) as i32], [ROWS_WIDTH, n * ROW_PITCH as i32])
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
    fn stock_by_floor_shows_only_built_items() {
        use id::*;
        assert_eq!(buy_list(tier(1)), [POTION, ETHER]);
        assert_eq!(buy_list(tier(2)), [POTION, HI_POTION, ETHER]);
        assert_eq!(buy_list(tier(6)), [POTION, HI_POTION, ETHER]);
        assert_eq!(buy_list(tier(7)), [POTION, HI_POTION, ETHER, ELIXIR]);
        assert_eq!(buy_list(tier(13)), [POTION, HI_POTION, ETHER, ELIXIR]);
        // The placeholders are in the table, at their tiers, but not built.
        assert!(STOCK.contains(&(Ware::Tent, Tier::First)));
        assert!(STOCK.contains(&(Ware::CampingSet, Tier::Early)));
        assert_eq!(Ware::Tent.item(), None);
        assert_eq!(Ware::CampingSet.item(), None);
        // The Mega items are never in the shop.
        for t in [Tier::First, Tier::Early, Tier::Mid, Tier::Late] {
            for m in [MEGA_POTION, MEGA_ETHER, MEGALIXIR] {
                assert!(!buy_list(t).contains(&m));
            }
        }
    }

    #[test]
    fn prices() {
        use id::*;
        assert_eq!([POTION, HI_POTION, ETHER, ELIXIR].map(buy_price), [Some(10), Some(30), Some(40), Some(200)]);
        assert_eq!([Ware::Tent.price(), Ware::CampingSet.price()], [100, 150]);
        assert_eq!([POTION, HI_POTION, ETHER, ELIXIR].map(sell_price), [Some(2), Some(6), Some(8), Some(40)]);
        // The Mega items: KH1 has no buy price, so KH1's sell price.
        assert_eq!([MEGA_POTION, MEGA_ETHER, MEGALIXIR].map(buy_price), [None, None, None]);
        assert_eq!([MEGA_POTION, MEGA_ETHER, MEGALIXIR].map(sell_price), [Some(40), Some(60), Some(100)]);
        // KH1's own ratio, which the owner's prices keep.
        for d in &it::DATA[..3] {
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
        inv.stock[id::POTION as usize - 1] = 99;
        let before = inv;
        assert_eq!(buy(id::POTION, 500, &mut inv), Buy::NoRoom { item: id::POTION });
        assert_eq!(inv, before);
        assert_eq!(buy(id::MEGA_POTION, 500, &mut inv), Buy::NotSold);
        assert_eq!(inv, before);
    }

    #[test]
    fn selling_one_per_confirm() {
        let mut inv = Inventory::new_game();
        inv.slots[0] = id::POTION;
        inv.slots[2] = id::POTION;
        inv.stock[id::POTION as usize - 1] = 1;
        inv.stock[id::MEGALIXIR as usize - 1] = 1;
        assert_eq!(sell_list(&inv), [(id::POTION, 3), (id::MEGALIXIR, 1)]);
        // The stock first, then the last slot holding it.
        assert_eq!(sell(id::POTION, 100, &mut inv), Sell::Sold { item: id::POTION, price: 2, points: 102, from: SoldFrom::Stock(0), left: 2 });
        assert_eq!(sell(id::POTION, 102, &mut inv), Sell::Sold { item: id::POTION, price: 2, points: 104, from: SoldFrom::Slot(2), left: 1 });
        assert_eq!(sell(id::POTION, 104, &mut inv), Sell::Sold { item: id::POTION, price: 2, points: 106, from: SoldFrom::Slot(0), left: 0 });
        assert_eq!(sell(id::POTION, 106, &mut inv), Sell::None);
        // The row left the list.
        assert_eq!(sell_list(&inv), [(id::MEGALIXIR, 1)]);
        assert_eq!(sell(id::MEGALIXIR, MAX_POINTS - 10, &mut inv), Sell::Sold { item: id::MEGALIXIR, price: 100, points: MAX_POINTS, from: SoldFrom::Stock(0), left: 0 });
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

    fn ctx() -> Ctx {
        Ctx { buy: buy_list(Tier::Early), sell: vec![(id::POTION, 2), (id::ETHER, 1)] }
    }

    #[test]
    fn root_rows_and_moves() {
        let c = ctx();
        let mut m = Menu::new(0);
        let up = pad(|p| p.up = true);
        let down = pad(|p| p.down = true);
        assert_eq!(m.step(&up, &c), Act::Nothing);
        assert_eq!(m.step(&down, &c), Act::Moved);
        assert_eq!(m.step(&down, &c), Act::Moved);
        assert_eq!(m.root, 2);
        // No wrap at the end (Re:CoM's list).
        assert_eq!(m.step(&down, &c), Act::Nothing);
        assert_eq!(m.step(&pad(|p| p.confirm = true), &c), Act::CardPacks);
        assert_eq!(m.step(&pad(|p| p.cancel = true), &c), Act::Leave);
        // Cancel before confirm, confirm before a direction.
        assert_eq!(m.step(&pad(|p| { p.cancel = true; p.confirm = true }), &c), Act::Leave);
        assert_eq!(m.step(&pad(|p| { p.confirm = true; p.up = true }), &c), Act::CardPacks);
        assert_eq!(Menu::new(7).root, 2);
    }

    #[test]
    fn buy_and_sell_pages() {
        let mut c = ctx();
        let mut m = Menu::new(0);
        assert_eq!(m.step(&pad(|p| p.confirm = true), &c), Act::Opened(Page::Buy));
        assert_eq!(m.step(&pad(|p| p.down = true), &c), Act::Moved);
        assert_eq!(m.step(&pad(|p| p.confirm = true), &c), Act::Buy(id::HI_POTION));
        assert_eq!(m.step(&pad(|p| p.cancel = true), &c), Act::Back);
        assert_eq!((m.page, m.root), (Page::Root, 0));
        m.step(&pad(|p| p.down = true), &c);
        assert_eq!(m.step(&pad(|p| p.confirm = true), &c), Act::Opened(Page::Sell));
        m.step(&pad(|p| p.down = true), &c);
        assert_eq!(m.step(&pad(|p| p.confirm = true), &c), Act::Sell(id::ETHER));
        // The Ether row left: the cursor settles on the last row.
        c.sell.pop();
        assert_eq!(m.step(&Pad::default(), &c), Act::Nothing);
        assert_eq!(m.row, 0);
        // The list emptied: back to the root on Sell.
        c.sell.clear();
        m.settle(&c);
        assert_eq!((m.page, m.root), (Page::Root, 1));
        // Nothing to sell: refused.
        assert_eq!(m.step(&pad(|p| p.confirm = true), &c), Act::Refused);
        assert_eq!(m.page, Page::Root);
    }

    #[test]
    fn texts() {
        let c = ctx();
        let name = |i: u8| String::from_utf8_lossy(crate::items_page::text::english(i)).trim_end_matches('\0').to_string();
        let mut m = Menu::new(0);
        assert_eq!(rows_text(&m, &c, name), b"Buy\nSell\nCard Packs\0");
        m.page = Page::Buy;
        assert_eq!(rows_text(&m, &c, name), b"Potion / 10\nHi-Potion / 30\nEther / 40\0");
        m.page = Page::Sell;
        assert_eq!(rows_text(&m, &c, name), b"Potion x2 / 2\nEther x1 / 8\0");
        let inv = Inventory::new_game();
        assert_eq!(help_text(&m, &c, None, &inv), b"Restores HP. Sells for 2 Moogle Points. Sora has 2.\0");
        m.page = Page::Buy;
        assert_eq!(help_text(&m, &c, None, &inv), b"Restores HP. 10 Moogle Points. Sora has 0.\0");
        assert_eq!(help_text(&m, &c, Some(Note::Poor), &inv), b"Not enough Moogle Points.\0");
        assert_eq!(c_text("a\u{e9}b"), b"a??b\0");
    }

    #[test]
    fn modes_and_places() {
        assert_eq!(card_packs_mode(true), mode::PACKS);
        assert_eq!(card_packs_mode(false), mode::NO_ROOM);
        assert!(room_for_pack(100, 95));
        assert!(!room_for_pack(100, 96));
        assert_eq!(ROW_PITCH, 23.0);
        assert_eq!(row_y(0), FIRST_ROW_Y);
        assert_eq!(row_y(8), FIRST_ROW_Y + 8.0 * 23.0);
        // Nine rows stay on the 384-high screen.
        assert!(row_y(MAX_ROWS - 1) < 384.0);
        assert_eq!(rows_window(3), ([ROWS_X, 84], [ROWS_WIDTH, 69]));
        // Every page fits: 7 items to sell + the two placeholders.
        assert!(id::ALL.len() + 2 <= MAX_ROWS);
        assert!(STOCK.len() <= MAX_ROWS);
    }
}
