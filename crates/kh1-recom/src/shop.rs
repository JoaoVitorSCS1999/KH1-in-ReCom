//! M8 stage 8a.3: KH1's item shop as a fifth "Items" tab of Re:CoM's Moogle shop pack screen
//! (`docs/recom-m8-items.md` § Stage 8a.3). The pure rules; Re:CoM's side is `win::shop`.
//!
//! Re:CoM's pack screen (`CMoogleShopCore` mode 4, update `0x1403A6930`, Checked (decomp + disasm))
//! has four tabs (`+0x280`, wrap 0..3) over a 2 × 2 grid of card packs (`+0x284`, `+0x348` of them,
//! the same count for every tab). Its input state (case 2) reads, in this order: a button 0x80
//! (sound 3, leaves to mode 5), then in the tab row cancel (sound 4, leaves to mode 3), Left (tab −
//! 1), Right (tab + 1), Down or confirm (into the grid, sound 1; no packs: sound 5); in the grid
//! confirm (the buy, else sound 5), then Down / Right / Up / Left (the first pressed) and cancel,
//! which wins over a direction (back to the tab row, sound 1), a move by the table `0x14065BA70`
//! only to a cell below the count (sound 1).
//!
//! The Items tab (owner, 2026-11-01): index 4, after Re:CoM's tab 3 (Right from tab 3, Left from
//! tab 0); Potion, Hi-Potion and Ether in cells 0..2 (the fourth cell empty), one item per buy,
//! priced 1 Moogle Point per KH1 munny (KH1's buy prices, `btltbl.bin` item rows `+8`: 60, 150,
//! 200), never sold out, no room-for-5-cards check and no card draw. The item goes where KH1's
//! pickup puts it (`crate::items::Inventory::pick_up`: the first empty slot, else the stock up to
//! 99); refused when neither has room.

use crate::items::{self as it, id};

/// The Items tab's index (Re:CoM's are 0..3).
pub const ITEMS_TAB: i32 = 4;
/// Re:CoM's tab count (`0x1403A6930`: wrap at > 3 and < 0). Checked (disasm `0x1403A6D3A`).
pub const RECOM_TABS: i32 = 4;

/// The items the tab sells, by grid cell: what KH1 sells at every item-shop stage (records 0..7 at
/// `0x1404FEE00` all start Potion, Hi-Potion, Ether; Checked (exe + files)). The fourth cell is
/// empty.
pub const ITEMS: [u8; 3] = [id::POTION, id::HI_POTION, id::ETHER];

/// Re:CoM's item card kinds shown as each item's picture (`BattleCardList` 0x2A Potion, 0x2B
/// Hi-Potion, 0x2D Ether). Checked (files: `SY0002.BIN`).
pub const CARD_KINDS: [u8; 3] = [0x2A, 0x2B, 0x2D];

/// A cell's item.
pub fn item_at(cell: i32) -> Option<u8> {
    usize::try_from(cell).ok().and_then(|c| ITEMS.get(c)).copied()
}

/// The price in Moogle Points: KH1's munny price, 1:1 (owner, 2026-11-01).
pub fn price(item: u8) -> i32 {
    it::data(item).map_or(0, |d| d.buy as i32)
}

/// Re:CoM's card value for a card picture (`C2DCard`, `0x140243560`): the kind in byte 0, byte 1 0
/// (not premium), byte 2 0xFF (negative: no number drawn; as the shop's own number-less card
/// `0x11FF0001` in `0x14045BF50`), byte 3 the kind's class from Re:CoM's table (`0x14068E0B0 +
/// kind × 16`, as `0x1403A66F0` builds its cards; 0x44 for the item cards). Checked (decomp + exe).
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

/// The cells the Items tab shows.
pub const COUNT: i32 = ITEMS.len() as i32;

/// Re:CoM's tab x (`0x14065BA40`: 64, 128, 192, 256) and the Items tab's: the next step of 64
/// (**Assumption**: Re:CoM's tab art has no fifth tab; the place waits for the tab's art).
pub const TAB_X: [f32; 5] = [64.0, 128.0, 192.0, 256.0, 320.0];
/// The tab cursor's y (`0x14065BCE0`), the tab button message's y (`0x14065BCE4`), the cursor's
/// offsets (`0x1406C1204` 16, `0x14050DCA4` 32). Checked (exe).
pub const TAB_CURSOR_Y: f32 = 53.0;
pub const TAB_MESSAGE_Y: f32 = 69.0;
pub const CURSOR_DX: f32 = 16.0;
pub const CURSOR_DY: f32 = 32.0;
/// The grid cells' places (`0x14065BA50` x, `0x14065BA60` y). Checked (exe).
pub const CELL_X: [f32; 4] = [74.0, 224.0, 74.0, 224.0];
pub const CELL_Y: [f32; 4] = [167.0, 167.0, 252.0, 252.0];

/// Re:CoM's sounds here (`0x1402AA360(id, −1)`; Checked (disasm `0x1403A6930`)).
pub mod sound {
    pub const MOVE: u32 = 1;
    pub const BUY: u32 = 2;
    /// The 0x80 button's (to mode 5).
    pub const OTHER: u32 = 3;
    pub const BACK: u32 = 4;
    pub const ERROR: u32 = 5;
    /// Played right after `BUY` on a pack buy.
    pub const BUY_TOO: u32 = 0x58;
}

/// Re:CoM's help message ids (`0x1403A5AA0` → `0x1404573A0`): 4 the usual line, 6 not enough Moogle
/// Points (5, no room for 5 cards, is about cards: not used by the Items tab).
pub mod help {
    pub const USUAL: i32 = 4;
    pub const NO_ROOM: i32 = 5;
    pub const POOR: i32 = 6;
}

/// The modes the 0x80 button and cancel leave to (`+0x25C`, then state 100). Checked (disasm
/// `0x1403A6C02`, `0x1403A6C4F`).
pub const MODE_OTHER: i32 = 5;
pub const MODE_BACK: i32 = 3;

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

/// From one of Re:CoM's tabs: does this frame's input go to the Items tab (Left on tab 0, Right on
/// tab 3, in the tab row)? Re:CoM's earlier reads win: the 0x80 button and cancel, and Left over
/// Right. `locked` = Re:CoM's input lock `+0x274 > 0` (no reads at all).
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

/// The Items tab's frame, in Re:CoM's order.
pub fn items_step(in_grid: bool, cell: i32, pad: &Pad, locked: bool) -> Step {
    if locked {
        return Step::Nothing;
    }
    if pad.other {
        return Step::Leave { sound: sound::OTHER, mode: MODE_OTHER };
    }
    if !in_grid {
        if pad.cancel {
            return Step::Leave { sound: sound::BACK, mode: MODE_BACK };
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
    match pad.grid_dir().map(|d| grid_move(cell, d, COUNT)) {
        Some(GridMove::Leave) => Step::OutOfGrid,
        Some(GridMove::To(c)) => Step::Move(c),
        _ => Step::Nothing,
    }
}

/// The help line for the Items tab: the usual one, or "not enough Moogle Points" on an item that
/// costs more than Sora has (Re:CoM's rule for a pack, `0x1403A5AA0`).
pub fn help_line(in_grid: bool, cell: i32, points: i32) -> i32 {
    match item_at(cell) {
        Some(i) if in_grid && price(i) > points => help::POOR,
        _ => help::USUAL,
    }
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
    /// The cell holds no item.
    Empty,
}

/// Re:CoM's Moogle Point cap (`0x14023A1D0`: 0 ..= 99999). Checked (decomp).
pub const MAX_POINTS: i32 = 99_999;

/// Points after paying (`0x1403A7041`: points − price, clamped to 0 ..= 99999).
pub fn pay(points: i32, price: i32) -> i32 {
    (points - price).clamp(0, MAX_POINTS)
}

/// A buy on `cell` with `points`: the price check first (Re:CoM's), then KH1's pickup on `inv`
/// (refused: nothing paid), then the payment.
pub fn buy(cell: i32, points: i32, inv: &mut it::Inventory) -> Buy {
    let Some(item) = item_at(cell) else { return Buy::Empty };
    let price = price(item);
    if price > points {
        return Buy::Poor { item, price };
    }
    match inv.pick_up(item) {
        Some(to) => Buy::Bought { item, price, points: pay(points, price), to },
        None => Buy::NoRoom { item },
    }
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
    fn items_and_prices() {
        assert_eq!(ITEMS.map(price), [60, 150, 200]);
        assert_eq!(item_at(0), Some(id::POTION));
        assert_eq!(item_at(2), Some(id::ETHER));
        assert_eq!(item_at(3), None);
        assert_eq!(item_at(-1), None);
        assert_eq!(card_value(0x2A, 0x44), 0x44FF_002A);
        // The shop's own number-less card.
        assert_eq!(card_value(1, 0x11), 0x11FF_0001);
    }

    #[test]
    fn recom_grid_moves() {
        // Re:CoM's table with 4 packs.
        assert_eq!(grid_move(0, Dir::Down, 4), GridMove::To(2));
        assert_eq!(grid_move(0, Dir::Right, 4), GridMove::To(1));
        assert_eq!(grid_move(0, Dir::Up, 4), GridMove::Leave);
        assert_eq!(grid_move(0, Dir::Left, 4), GridMove::Stay);
        assert_eq!(grid_move(1, Dir::Up, 4), GridMove::Leave);
        assert_eq!(grid_move(1, Dir::Left, 4), GridMove::To(0));
        assert_eq!(grid_move(3, Dir::Up, 4), GridMove::To(1));
        assert_eq!(grid_move(3, Dir::Left, 4), GridMove::To(2));
        assert_eq!(grid_move(2, Dir::Down, 4), GridMove::Stay);
        // The Items tab's three: the fourth cell is never reached.
        assert_eq!(grid_move(1, Dir::Down, COUNT), GridMove::Stay);
        assert_eq!(grid_move(2, Dir::Right, COUNT), GridMove::Stay);
        assert_eq!(grid_move(0, Dir::Down, COUNT), GridMove::To(2));
        assert_eq!(grid_move(2, Dir::Up, COUNT), GridMove::To(0));
    }

    #[test]
    fn entering_the_items_tab() {
        let right = pad(|p| p.right = true);
        let left = pad(|p| p.left = true);
        assert!(enters_items(3, false, &right, false));
        assert!(enters_items(0, false, &left, false));
        assert!(!enters_items(2, false, &right, false));
        assert!(!enters_items(3, false, &left, false));
        assert!(!enters_items(0, false, &right, false));
        // Not from the grid, not locked, the earlier reads win.
        assert!(!enters_items(3, true, &right, false));
        assert!(!enters_items(3, false, &right, true));
        assert!(!enters_items(3, false, &pad(|p| { p.right = true; p.cancel = true }), false));
        assert!(!enters_items(3, false, &pad(|p| { p.right = true; p.other = true }), false));
        // Left is read before Right: on tab 3 with both, Re:CoM goes to tab 2.
        assert!(!enters_items(3, false, &pad(|p| { p.right = true; p.left = true }), false));
        assert!(enters_items(0, false, &pad(|p| { p.right = true; p.left = true }), false));
    }

    #[test]
    fn tab_row() {
        assert_eq!(items_step(false, 0, &pad(|p| p.left = true), false), Step::ToTab(3));
        assert_eq!(items_step(false, 0, &pad(|p| p.right = true), false), Step::ToTab(0));
        assert_eq!(items_step(false, 0, &pad(|p| p.down = true), false), Step::IntoGrid);
        assert_eq!(items_step(false, 0, &pad(|p| p.confirm = true), false), Step::IntoGrid);
        assert_eq!(items_step(false, 0, &pad(|p| p.up = true), false), Step::Nothing);
        assert_eq!(items_step(false, 0, &pad(|p| p.cancel = true), false), Step::Leave { sound: sound::BACK, mode: MODE_BACK });
        assert_eq!(items_step(false, 0, &pad(|p| { p.other = true; p.cancel = true }), false), Step::Leave { sound: sound::OTHER, mode: MODE_OTHER });
        assert_eq!(items_step(false, 0, &pad(|p| p.left = true), true), Step::Nothing);
    }

    #[test]
    fn grid() {
        assert_eq!(items_step(true, 1, &pad(|p| p.confirm = true), false), Step::Confirm(1));
        // Confirm wins over everything but the 0x80 button.
        assert_eq!(items_step(true, 1, &pad(|p| { p.confirm = true; p.cancel = true }), false), Step::Confirm(1));
        assert_eq!(items_step(true, 1, &pad(|p| { p.confirm = true; p.other = true }), false), Step::Leave { sound: sound::OTHER, mode: MODE_OTHER });
        // Cancel wins over a direction.
        assert_eq!(items_step(true, 2, &pad(|p| { p.up = true; p.cancel = true }), false), Step::OutOfGrid);
        assert_eq!(items_step(true, 0, &pad(|p| p.up = true), false), Step::OutOfGrid);
        assert_eq!(items_step(true, 0, &pad(|p| p.right = true), false), Step::Move(1));
        assert_eq!(items_step(true, 2, &pad(|p| p.up = true), false), Step::Move(0));
        assert_eq!(items_step(true, 2, &pad(|p| p.right = true), false), Step::Nothing);
        // Down before Right.
        assert_eq!(items_step(true, 0, &pad(|p| { p.right = true; p.down = true }), false), Step::Move(2));
    }

    #[test]
    fn buying() {
        let mut inv = Inventory::new_game();
        // Too poor: nothing changes.
        assert_eq!(buy(0, 59, &mut inv), Buy::Poor { item: id::POTION, price: 60 });
        assert_eq!(inv, Inventory::new_game());
        // KH1's pickup: the slots first, then the stock.
        assert_eq!(buy(0, 60, &mut inv), Buy::Bought { item: id::POTION, price: 60, points: 0, to: Picked::Slot(0) });
        assert_eq!(buy(1, 1000, &mut inv), Buy::Bought { item: id::HI_POTION, price: 150, points: 850, to: Picked::Slot(1) });
        assert_eq!(buy(2, 1000, &mut inv), Buy::Bought { item: id::ETHER, price: 200, points: 800, to: Picked::Slot(2) });
        assert_eq!(buy(2, 1000, &mut inv), Buy::Bought { item: id::ETHER, price: 200, points: 800, to: Picked::Stock(1) });
        // Full: nothing paid.
        inv.stock[id::POTION as usize - 1] = 99;
        let before = inv;
        assert_eq!(buy(0, 1000, &mut inv), Buy::NoRoom { item: id::POTION });
        assert_eq!(inv, before);
        assert_eq!(buy(3, 1000, &mut inv), Buy::Empty);
    }

    #[test]
    fn payment_and_tally() {
        assert_eq!(pay(100, 60), 40);
        assert_eq!(pay(10, 60), 0);
        assert_eq!(pay(200_000, 60), MAX_POINTS);
        assert_eq!(spent(19_950, 60), (20_010, true));
        assert_eq!(spent(20_010, 60), (20_070, false));
        assert_eq!(spent(100, 60), (160, false));
    }

    #[test]
    fn help_lines() {
        assert_eq!(help_line(false, 2, 0), help::USUAL);
        assert_eq!(help_line(true, 2, 199), help::POOR);
        assert_eq!(help_line(true, 2, 200), help::USUAL);
        assert_eq!(help_line(true, 3, 0), help::USUAL);
    }
}
