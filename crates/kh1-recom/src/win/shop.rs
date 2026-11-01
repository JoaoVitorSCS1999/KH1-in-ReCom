//! M8 stage 8a.3: the Items tab in Re:CoM's Moogle shop pack screen (`crate::shop`,
//! `docs/recom-m8-items.md` § Stage 8a.3).
//!
//! A wrapper on the pack screen's update (`CMoogleShopCore` mode 4, `0x1403A6930`). Re:CoM runs
//! the screen; the wrapper only acts in its input state (2) on the tab row's edges (Left on tab 0,
//! Right on tab 3) and while the Items tab (index 4) is up, and when the screen is left with it up:
//!
//! - **Entering** the Items tab: Re:CoM's packs are released as its own state 0x65 releases them
//!   (`+0x6C` = 1: each plays its own closing and deletes itself), three new packs are made with
//!   Re:CoM's own constructor (`0x14045A9E0`, as state 0 makes them: cells 0..2, the item's price,
//!   not sold out), the pack count `+0x348` becomes 3 (the fourth cell empty: Re:CoM's own move rule
//!   stops there), each pack's picture layer is hidden and Re:CoM's own card picture (`C2DCard`) of
//!   the item's card (Potion 0x2A, Hi-Potion 0x2B, Ether 0x2D) is attached to it.
//! - **On the Items tab**: Re:CoM's input state, run for the tab's cells (`crate::shop::items_step`:
//!   Re:CoM's reads in Re:CoM's order, its sounds, help lines, cursors and button messages); a buy
//!   pays through Re:CoM's own payment (state 4's tally and trophy, the Moogle Point setter
//!   `0x14023A1D0`, the Moogle Point window) and gives the item by KH1's pickup rule.
//! - **Leaving** for Re:CoM's tab: the cards go, our packs are released, Re:CoM's packs for that
//!   tab are made again as state 0 makes them, then Re:CoM's own refresh (`0x1403A92F0`) sets their
//!   pictures, prices and sold-out marks, the tab art and the cursors.
//! - **Leaving the screen** with the Items tab up (cancel, Square): Re:CoM's states 100 / 0x65 close
//!   and release our packs as theirs (the cards are deleted first, at state 100); after 0x65 the
//!   count is Re:CoM's again.
//!
//! Only at Combat = KH1 in Sora's story (KH1's items live: `super::mp::active`). Nothing of ours
//! runs outside the pack screen.

use super::movement::{func, orig, rd, wr};
use super::{base, log};
use crate::shop::{self as sh, Buy, Pad, Step};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_PACK_SCREEN: AtomicUsize = AtomicUsize::new(0);

static INSTALLED: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(60);
/// The Items tab, while it is up (one shop at a time).
static TAB: Mutex<Option<ItemsTab>> = Mutex::new(None);

struct ItemsTab {
    shop: usize,
    /// Re:CoM's pack count, given back when the tab goes.
    count: i32,
    /// Our card pictures (deleted before their packs are released).
    cards: [usize; 3],
}

// Re:CoM functions (RVAs; first bytes checked: `sites::SHOP_CALLEES`, and `CALLEES`,
// `COMMAND_MENU_CALLEES`, `M6_DECK_CALLEES`, `KH1_HUD_CALLEES` for the shared ones).
const NEW: u32 = 0x004A_CE54;
const SOUND: u32 = 0x002A_A360;
const PAD_REPEAT: u32 = 0x0029_B2E0;
const PAD_FRESH: u32 = 0x0029_B2F0;
const PAD_CANCEL: u32 = 0x0029_B410;
const PAD_CONFIRM: u32 = 0x0029_B490;
const REFRESH: u32 = 0x003A_92F0;
const PACK_CTOR: u32 = 0x0045_A9E0;
const PACK_PRICE: u32 = 0x0045_D120;
const PACK_SOLD_OUT: u32 = 0x0045_D3F0;
const HELP_LINE: u32 = 0x0045_73A0;
const MESSAGE_PLACE: u32 = 0x0045_9AF0;
const CURSOR_PLACE: u32 = 0x0045_9A00;
const CURSOR_VISIBLE: u32 = 0x0045_97D0;
const POINT_WINDOW: u32 = 0x0045_98A0;
const POINTS_SET: u32 = 0x0023_A1D0;
const VAR_GET: u32 = 0x001B_08C0;
const VAR_SET: u32 = 0x001B_16C0;
const TROPHY: u32 = 0x002B_A6F0;
const CARD_CTOR: u32 = 0x0024_32A0;
const CARD_SET: u32 = 0x0024_3560;
const CARD_PRIORITY: u32 = 0x0024_3380;
const CARD_ATTACH: u32 = 0x0024_3DD0;
const CARD_VISIBLE: u32 = 0x0024_3A60;
const SLOT_VISIBLE: u32 = 0x0024_7E50;
const SPRITE_POS: u32 = 0x0024_85B0;
/// `C2DCard`'s vtable (`0x1402432A0` writes it); its slot 0 is the deleting dtor `0x1402073D0`.
/// Checked (disasm `0x1402432AE`; exe).
const CARD_VTABLE: usize = 0x0060_6BF8;
const CARD_DTOR: usize = 0x0020_73D0;

// Re:CoM data (RVAs). Checked (exe: the rip operands in `sites::SHOP_CHECKED`'s payment and
// confirm bytes; the class rows).
/// `*0x14087CBF8` = the save's stats; Moogle Points at `+0x20`.
const STATS_PTR: usize = 0x0087_CBF8;
const POINTS: usize = 0x20;
/// The trophy bit `& 0x10` (set once the 20000-points trophy is given) and the trophy object.
const TROPHY_FLAGS: usize = 0x00AF_7389;
const TROPHY_OBJ: usize = 0x00AF_7468;
/// Card class rows (`0x14068E0B0 + kind × 16`, byte 0).
const CARD_CLASS: usize = 0x0068_E0B0;

/// `CMoogleShopCore` fields (Checked (decomp + disasm: `0x1403A4220`, `0x1403A6930`,
/// `0x1403A92F0`, `0x1403A5B10`)).
mod f {
    pub const NEXT_MODE: usize = 0x25C;
    pub const STATE: usize = 0x260;
    /// The packs' task parent (the constructor's second argument).
    pub const TASKS: usize = 0x268;
    /// Input lock: no reads while above 0.
    pub const LOCK: usize = 0x274;
    pub const TAB: usize = 0x280;
    pub const CELL: usize = 0x284;
    /// 4 × 4 arrays `[tab × 4 + cell]`: prices, sold-out.
    pub const PRICES: usize = 0x2C8;
    pub const SOLD_OUT: usize = 0x308;
    /// Packs per tab (the same for every tab).
    pub const COUNT: usize = 0x348;
    /// 0 = the tab row, 1 = the grid.
    pub const IN_GRID: usize = 0x34C;
    pub const POINT_WINDOW: usize = 0x3C0;
    /// The finger cursor, and the second cursor marking the tab while in the grid.
    pub const CURSOR: usize = 0x3D0;
    pub const TAB_MARK: usize = 0x3D8;
    pub const HELP: usize = 0x3E8;
    /// The button messages by the tab and by the cell (`CMShopButtonMessage`; `+0x60` its show
    /// state, `0x1404598D0`).
    pub const TAB_MESSAGE: usize = 0x410;
    pub const CELL_MESSAGE: usize = 0x420;
    pub const PACKS: usize = 0x428;
}
/// `CMShopButtonMessage +0x60`: 0 show, −1 hide (`0x1404598D0` writes it).
const MESSAGE_SHOW: usize = 0x60;
/// `CMShopCardPack`: its picture control (layer 0 the frame, layer 1 the pack's art,
/// `0x14045A9E0`), its release flag (`+0x6C`: closes and deletes itself, `0x14045C910`), its size.
const PACK_PICTURE: usize = 0x78;
const PACK_RELEASE: usize = 0x6C;
const PACK_SIZE: usize = 0xB8;
const PICTURE_ART_LAYER: i32 = 1;
/// `C2DCard`: its size (`0x14045BF50` allocates 400), the shop's card priority (0x12, one over the
/// packs' 0x11), "use the colour" `+0x138`.
const CARD_SIZE: usize = 400;
const CARD_PRIORITY_SHOP: u32 = 0x12;
const C_COLOUR_ON: usize = 0x138;

/// The pack screen's states (`0x1403A6930`).
const STATE_FIRST: i32 = 0;
const STATE_INPUT: i32 = 2;
const STATE_CLOSE: i32 = 100;
const STATE_RELEASE: i32 = 0x65;

type ScreenFn = unsafe extern "win64" fn(usize);
type NewFn = unsafe extern "win64" fn(usize) -> usize;
type SoundFn = unsafe extern "win64" fn(u32, i32) -> u64;
type PadMaskFn = unsafe extern "win64" fn(u32, u32) -> u32;
type PadBtnFn = unsafe extern "win64" fn(u32) -> u32;
type PackCtorFn = unsafe extern "win64" fn(usize, usize, i32, i32, i32) -> usize;
type PackIntFn = unsafe extern "win64" fn(usize, i32) -> u64;
type PackSoldFn = unsafe extern "win64" fn(usize, i32, i32) -> u64;
type PlaceFn = unsafe extern "win64" fn(usize, f32, f32);
type CursorPlaceFn = unsafe extern "win64" fn(usize, f32, f32, i32);
type VisibleFn = unsafe extern "win64" fn(usize, u32);
type PointsSetFn = unsafe extern "win64" fn(i32) -> u64;
type VarGetFn = unsafe extern "win64" fn(u32, u32) -> u64;
type VarSetFn = unsafe extern "win64" fn(u32, u32, i32);
type TrophyFn = unsafe extern "win64" fn(usize, u32);
type CtorFn = unsafe extern "win64" fn(usize) -> usize;
type CardSetFn = unsafe extern "win64" fn(usize, u32);
type AttachFn = unsafe extern "win64" fn(usize, usize);
type SlotVisibleFn = unsafe extern "win64" fn(usize, i32, u32);
type PosFn = unsafe extern "win64" fn(usize, *const f32);
type DeleteFn = unsafe extern "win64" fn(usize, u32) -> usize;

#[repr(C, align(16))]
struct V4([f32; 4]);

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("shop: {}", line()));
    }
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

/// The Items tab is offered: the wrapper is in and KH1's items are live (Combat = KH1, Sora's
/// story, a save).
fn live() -> bool {
    INSTALLED.load(Ordering::Acquire) && super::items::installed() && super::mp::active()
}

fn tab_up(shop: usize) -> bool {
    TAB.lock().map(|g| g.as_ref().is_some_and(|t| t.shop == shop)).unwrap_or(false)
}

unsafe fn sound(id: u32) {
    func::<SoundFn>(SOUND)(id, -1);
}

unsafe fn points() -> i32 {
    let s: usize = rd(base() + STATS_PTR);
    if s == 0 {
        return 0;
    }
    rd(s + POINTS)
}

/// This frame's presses, Re:CoM's reads (pure).
unsafe fn read_pad() -> Pad {
    let dir = |mask: u32| func::<PadMaskFn>(PAD_REPEAT)(mask, 0) != 0;
    Pad {
        other: func::<PadMaskFn>(PAD_FRESH)(0x80, 0) != 0,
        cancel: func::<PadBtnFn>(PAD_CANCEL)(0) != 0,
        confirm: func::<PadBtnFn>(PAD_CONFIRM)(0) != 0,
        down: dir(0x0020_4000),
        right: dir(0x0080_2000),
        up: dir(0x0010_1000),
        left: dir(0x0040_8000),
    }
}

unsafe fn pack(shop: usize, i: usize) -> usize {
    rd(shop + f::PACKS + i * 8)
}

/// Releases the packs in the array (as state 0x65: `+0x6C` = 1, the entry cleared).
unsafe fn release_packs(shop: usize, n: usize) {
    for i in 0..n.min(4) {
        let p = pack(shop, i);
        if p != 0 {
            wr::<u32>(p + PACK_RELEASE, 1);
            wr::<usize>(shop + f::PACKS + i * 8, 0);
        }
    }
}

/// A pack as state 0 makes it (`0x1403A6AD0`): 0xB8 bytes, (task parent, cell, price, sold out).
unsafe fn make_pack(shop: usize, cell: i32, price: i32, sold_out: i32) -> usize {
    let mem = func::<NewFn>(NEW)(PACK_SIZE);
    if mem == 0 {
        return 0;
    }
    func::<PackCtorFn>(PACK_CTOR)(mem, rd(shop + f::TASKS), cell, price, sold_out)
}

/// Re:CoM's card picture of an item card on `pack`'s picture control: as the shop's own card in
/// `0x14045BF50` (priority 0x12, attached to its window's control, colour on), without its base
/// group, as the "cards got" screen makes its cards (`0x1404589B0`).
unsafe fn make_card(pack: usize, kind: u8) -> usize {
    let ctrl: usize = rd(pack + PACK_PICTURE);
    if ctrl == 0 {
        return 0;
    }
    let mem = func::<NewFn>(NEW)(CARD_SIZE);
    if mem == 0 {
        return 0;
    }
    let card = func::<CtorFn>(CARD_CTOR)(mem);
    if card == 0 || rd::<usize>(card) != base() + CARD_VTABLE {
        note(|| "a card picture's vtable is not C2DCard's; no picture".into());
        return 0;
    }
    let class: u8 = rd(base() + CARD_CLASS + kind as usize * 16);
    func::<CardSetFn>(CARD_SET)(card, sh::card_value(kind, class));
    func::<CardSetFn>(CARD_PRIORITY)(card, CARD_PRIORITY_SHOP);
    func::<AttachFn>(CARD_ATTACH)(card, ctrl);
    func::<PosFn>(SPRITE_POS)(card, V4([0.0, 0.0, 0.0, 1.0]).0.as_ptr());
    wr::<u8>(card + C_COLOUR_ON, 1);
    func::<VisibleFn>(CARD_VISIBLE)(card, 1);
    card
}

/// Deletes a card picture (its own deleting dtor, `C2DCard` vtable slot 0, flag 1).
unsafe fn delete_card(card: usize) {
    if card == 0 {
        return;
    }
    let dtor: usize = rd(rd::<usize>(card));
    if dtor != base() + CARD_DTOR {
        note(|| format!("a card picture's dtor is {dtor:#x}, not 0x1402073D0; left in place"));
        return;
    }
    let delete: DeleteFn = std::mem::transmute::<usize, DeleteFn>(dtor);
    delete(card, 1);
}

/// Into the Items tab from Re:CoM's tab `from` (sound 1 played by the caller).
unsafe fn enter(shop: usize, from: i32) {
    let count: i32 = rd(shop + f::COUNT);
    release_packs(shop, count.clamp(0, 4) as usize);
    let mut cards = [0usize; 3];
    let mut made = 0;
    for (i, &item) in sh::ITEMS.iter().enumerate() {
        let p = make_pack(shop, i as i32, sh::price(item), 0);
        wr::<usize>(shop + f::PACKS + i * 8, p);
        if p == 0 {
            break;
        }
        made += 1;
        func::<SlotVisibleFn>(SLOT_VISIBLE)(rd(p + PACK_PICTURE), PICTURE_ART_LAYER, 0);
        cards[i] = make_card(p, sh::CARD_KINDS[i]);
    }
    wr::<usize>(shop + f::PACKS + 3 * 8, 0);
    if let Ok(mut g) = TAB.lock() {
        *g = Some(ItemsTab { shop, count, cards });
    }
    if made < sh::ITEMS.len() {
        note(|| format!("Items tab: only {made} of 3 packs made (no memory); back to tab {from}"));
        leave_to(shop, from);
        return;
    }
    wr::<i32>(shop + f::COUNT, sh::COUNT);
    wr::<i32>(shop + f::TAB, sh::ITEMS_TAB);
    refresh(shop);
    note(|| format!("Items tab from tab {from}: Re:CoM's {count} packs released, Potion 60 / Hi-Potion 150 / Ether 200; cards {cards:#x?}; {} Moogle Points", points()));
}

/// Takes the Items tab's state, deletes its cards; None when it isn't up for `shop`.
unsafe fn take_tab(shop: usize) -> Option<i32> {
    let t = {
        let mut g = TAB.lock().ok()?;
        g.take_if(|t| t.shop == shop)?
    };
    for c in t.cards {
        delete_card(c);
    }
    Some(t.count)
}

/// From the Items tab to Re:CoM's tab `to`: Re:CoM's packs for it as state 0 makes them, then
/// Re:CoM's refresh.
unsafe fn leave_to(shop: usize, to: i32) {
    let Some(count) = take_tab(shop) else { return };
    release_packs(shop, sh::ITEMS.len());
    let n = count.clamp(0, 4);
    for i in 0..n {
        let k = (to * 4 + i) as usize * 4;
        let p = make_pack(shop, i, rd(shop + f::PRICES + k), rd(shop + f::SOLD_OUT + k));
        wr::<usize>(shop + f::PACKS + i as usize * 8, p);
    }
    wr::<i32>(shop + f::COUNT, count);
    wr::<i32>(shop + f::TAB, to);
    func::<ScreenFn>(REFRESH)(shop);
    note(|| format!("Items tab -> tab {to}: Re:CoM's {count} packs made again"));
}

/// Re:CoM's refresh (`0x1403A92F0`) for the Items tab: the button messages at the tab and the cell,
/// the packs' prices (never sold out), the cursors. The tab art stays as it was (no Items tab art
/// yet: `docs/recom-m8-items.md` § 8a.3).
unsafe fn refresh(shop: usize) {
    let x = sh::TAB_X[sh::ITEMS_TAB as usize];
    let cell = (rd::<i32>(shop + f::CELL)).clamp(0, 3) as usize;
    let place = func::<PlaceFn>(MESSAGE_PLACE);
    place(rd(shop + f::TAB_MESSAGE), x, sh::TAB_MESSAGE_Y);
    place(rd(shop + f::CELL_MESSAGE), sh::CELL_X[cell], sh::CELL_Y[cell]);
    for (i, &item) in sh::ITEMS.iter().enumerate() {
        let p = pack(shop, i);
        if p != 0 {
            func::<PackIntFn>(PACK_PRICE)(p, sh::price(item));
            func::<PackSoldFn>(PACK_SOLD_OUT)(p, 0, 0);
        }
    }
    let cursor: usize = rd(shop + f::CURSOR);
    let mark: usize = rd(shop + f::TAB_MARK);
    let show = func::<VisibleFn>(CURSOR_VISIBLE);
    let put = func::<CursorPlaceFn>(CURSOR_PLACE);
    if rd::<i32>(shop + f::IN_GRID) != 0 {
        show(mark, 1);
        put(mark, x - sh::CURSOR_DX, sh::TAB_CURSOR_Y, 0);
        show(cursor, 1);
        put(cursor, sh::CELL_X[cell] - sh::CURSOR_DX, sh::CELL_Y[cell] - sh::CURSOR_DY, 0);
    } else {
        show(cursor, 1);
        put(cursor, x - sh::CURSOR_DX, sh::TAB_CURSOR_Y, 0);
        show(mark, 0);
    }
}

/// Re:CoM's payment for `price` (state 4, `0x1403A7041`): the spent tally and its trophy, the
/// points, the Moogle Point window.
unsafe fn pay(shop: usize, price: i32, after: i32) {
    let flags = base() + TROPHY_FLAGS;
    if rd::<u8>(flags) & 0x10 == 0 {
        let (t, v) = sh::SPENT_VAR;
        let before = func::<VarGetFn>(VAR_GET)(t, v) as i32;
        let (spent, trophy) = sh::spent(before, price);
        func::<VarSetFn>(VAR_SET)(t, v, spent);
        if trophy {
            let o: usize = rd(base() + TROPHY_OBJ);
            if o != 0 {
                func::<TrophyFn>(TROPHY)(o, sh::SPENT_TROPHY);
            }
            wr::<u8>(flags, rd::<u8>(flags) | 0x10);
        }
    }
    func::<PointsSetFn>(POINTS_SET)(after);
    let w: usize = rd(shop + f::POINT_WINDOW);
    if w != 0 {
        func::<PackIntFn>(POINT_WINDOW)(w, after);
    }
}

unsafe fn buy(shop: usize, cell: i32) {
    let have = points();
    match super::items::edit(|inv| sh::buy(cell, have, inv)) {
        Some(Buy::Bought { item, price, points: after, to }) => {
            sound(sh::sound::BUY);
            sound(sh::sound::BUY_TOO);
            pay(shop, price, after);
            note(|| format!("bought {} for {price}: {have} -> {after} Moogle Points, {to:?}", crate::items::name(item)));
        }
        r => {
            sound(sh::sound::ERROR);
            note(|| format!("refused on cell {cell} with {have} Moogle Points: {r:?}"));
        }
    }
}

/// One frame of the Items tab's input state.
unsafe fn items_frame(shop: usize) {
    let in_grid = rd::<i32>(shop + f::IN_GRID) != 0;
    let cell: i32 = rd(shop + f::CELL);
    let locked = rd::<i32>(shop + f::LOCK) > 0;
    let help: usize = rd(shop + f::HELP);
    if help != 0 {
        func::<PackIntFn>(HELP_LINE)(help, sh::help_line(in_grid, cell, points()));
    }
    let pad = if locked { Pad::default() } else { read_pad() };
    match sh::items_step(in_grid, cell, &pad, locked) {
        Step::Leave { sound: s, mode } => {
            sound(s);
            wr::<i32>(shop + f::NEXT_MODE, mode);
            wr::<i32>(shop + f::STATE, STATE_CLOSE);
            return;
        }
        Step::ToTab(t) => {
            sound(sh::sound::MOVE);
            leave_to(shop, t);
            return;
        }
        Step::IntoGrid => {
            sound(sh::sound::MOVE);
            wr::<i32>(shop + f::IN_GRID, 1);
            wr::<i32>(shop + f::CELL, 0);
            message_show(shop, 0);
        }
        Step::OutOfGrid => {
            sound(sh::sound::MOVE);
            wr::<i32>(shop + f::IN_GRID, 0);
            message_show(shop, -1);
        }
        Step::Move(c) => {
            sound(sh::sound::MOVE);
            wr::<i32>(shop + f::CELL, c);
        }
        Step::Confirm(c) => buy(shop, c),
        Step::Nothing => {}
    }
    refresh(shop);
}

/// The cell's button message's show state (`0x1404598D0`: one field write).
unsafe fn message_show(shop: usize, v: i32) {
    let m: usize = rd(shop + f::CELL_MESSAGE);
    if m != 0 {
        wr::<i32>(m + MESSAGE_SHOW, v);
    }
}

/// The wrapper on the pack screen's update (`0x1403A6930`).
///
/// # Safety
/// Called by Re:CoM with its `CMoogleShopCore`.
pub unsafe extern "win64" fn pack_screen(shop: usize) {
    let run = orig::<ScreenFn>(&ORIG_PACK_SCREEN);
    let state: i32 = rd(shop + f::STATE);
    let up = tab_up(shop);
    match state {
        STATE_FIRST if up => {
            // A new screen on the same object with our tab still recorded: it never passed
            // state 0x65 (not seen in Re:CoM's code). Its cards went with the old screen.
            if let Ok(mut g) = TAB.lock() {
                *g = None;
            }
            note(|| "a new pack screen with the Items tab still recorded: forgotten".into());
            run(shop)
        }
        STATE_INPUT if up => items_frame(shop),
        STATE_INPUT if live() => {
            let tab: i32 = rd(shop + f::TAB);
            let in_grid = rd::<i32>(shop + f::IN_GRID) != 0;
            let locked = rd::<i32>(shop + f::LOCK) > 0;
            if (0..sh::RECOM_TABS).contains(&tab) && !in_grid && !locked && sh::enters_items(tab, in_grid, &read_pad(), locked) {
                // Re:CoM's state 2 sets the help line before its reads; the tab row's is the usual one.
                let help: usize = rd(shop + f::HELP);
                if help != 0 {
                    func::<PackIntFn>(HELP_LINE)(help, sh::help::USUAL);
                }
                sound(sh::sound::MOVE);
                enter(shop, tab);
            } else {
                run(shop)
            }
        }
        STATE_CLOSE if up => {
            // Our cards go before Re:CoM starts closing the packs they hang from.
            if let Ok(mut g) = TAB.lock() {
                if let Some(t) = g.as_mut().filter(|t| t.shop == shop) {
                    for c in std::mem::take(&mut t.cards) {
                        delete_card(c);
                    }
                }
            }
            run(shop)
        }
        STATE_RELEASE if up => {
            run(shop);
            if rd::<i32>(shop + f::STATE) != STATE_RELEASE {
                // Re:CoM released our packs (the count was ours): its count back.
                if let Some(count) = take_tab(shop) {
                    wr::<i32>(shop + f::COUNT, count);
                    note(|| format!("pack screen left from the Items tab: Re:CoM's count {count} back"));
                }
            }
        }
        _ => run(shop),
    }
}

pub fn start_line() -> String {
    "shop: KH1's items in the Moogle shop (M8 stage 8a.3): a fifth tab \"Items\" after Re:CoM's four (Right from the last, Left from the first) with Potion 60 / Hi-Potion 150 / Ether 200 Moogle Points, Re:CoM's item card pictures, KH1's pickup rule; at Combat = KH1 in Sora's story".into()
}
