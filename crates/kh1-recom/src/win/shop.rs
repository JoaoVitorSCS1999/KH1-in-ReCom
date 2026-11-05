//! M8d: KH1's items in Re:CoM's own Moogle shop screens (`crate::shop`, `docs/recom-m8-items.md`
//! § M8d).
//!
//! Re:CoM's shop runs as Re:CoM's: its two-Moogle menu (mode 3) unpatched. Two wrappers, both
//! acting only at Combat = KH1 in Sora's story (KH1's items live: `super::mp::active`):
//!
//! - **The pack screen** (mode 4's update `0x1403A6930`): an Items tab drawn first, before
//!   Re:CoM's four pack tabs, which the screen opens on. Entering it, Re:CoM's packs are released
//!   as its own state 0x65 releases them, Re:CoM's own pack constructor makes one pack per item
//!   (its price plate, never sold out), each pack's art layer is hidden and Re:CoM's own card
//!   picture (`C2DCard`) of the item's card is attached to it. On the Items tab Re:CoM's input state
//!   is replayed for its cells (`crate::shop::items_step`), a buy pays through Re:CoM's own payment
//!   and gives the item by KH1's pickup rule. Leaving for Re:CoM's tab, Re:CoM's packs are made
//!   again and Re:CoM's own refresh runs. With the asset step's five-tab art (`kh1-shop-tabs.json`)
//!   Re:CoM's tab tables are pointed at the art's places for the screen (`0x14065BA40`,
//!   `0x140690420`: written at each screen start, Re:CoM's own values at Combat = Re:CoM) and the
//!   icons' sequence plays the art's icons.
//! - **The card trade** (mode 5's update `0x1403A73E0`): its Square goes to the pack screen also
//!   without room for 5 cards (Re:CoM's needs room), so items can be bought with a full deck; its
//!   Square button hint is made then too, as Re:CoM makes it.
//!
//! Nothing of ours runs outside those two screens.

use super::movement::{func, orig, rd, wr};
use super::{base, log};
use crate::shop::{self as sh, Buy, Note, Pad, Step};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_PACK_SCREEN: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_TRADE: AtomicUsize = AtomicUsize::new(0);

static INSTALLED: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
/// The five-tab art's first group in `m_shop:01` (0: no art).
static ART: AtomicU32 = AtomicU32::new(0);
/// The Items tab, while it is up (one shop at a time).
static TAB: Mutex<Option<ItemsTab>> = Mutex::new(None);

struct ItemsTab {
    shop: usize,
    /// Re:CoM's pack count, given back when the tab goes.
    count: i32,
    /// The items in the cells.
    items: Vec<u8>,
    /// Our card pictures (deleted before their packs are released).
    cards: [usize; sh::CELLS],
    note: Option<Note>,
    /// The line Re:CoM's text window points at now, and the ones before (Re:CoM's text set keeps
    /// the pointer).
    help: Vec<u8>,
    old: Vec<Vec<u8>>,
    /// The art's window was played for the Items tab.
    window: bool,
}

// Re:CoM functions (RVAs; first bytes checked: `sites::SHOP_CALLEES`, and `CALLEES` etc. for the
// shared ones).
const NEW: u32 = 0x004A_CE54;
const SOUND: u32 = 0x002A_A360;
const PAD_REPEAT: u32 = 0x0029_B2E0;
const PAD_FRESH: u32 = 0x0029_B2F0;
const PAD_CANCEL: u32 = 0x0029_B410;
const PAD_CONFIRM: u32 = 0x0029_B490;
const FLOOR: u32 = 0x0023_37C0;
const REFRESH: u32 = 0x003A_92F0;
const PACK_CTOR: u32 = 0x0045_A9E0;
const PACK_PRICE: u32 = 0x0045_D120;
const PACK_SOLD_OUT: u32 = 0x0045_D3F0;
const MESSAGE_PLACE: u32 = 0x0045_9AF0;
const CURSOR_PLACE: u32 = 0x0045_9A00;
const CURSOR_VISIBLE: u32 = 0x0045_97D0;
const POINT_WINDOW: u32 = 0x0045_98A0;
const SHOW_STATE: u32 = 0x0045_98D0;
const BUTTON_CTOR: u32 = 0x0045_A940;
const TEXTWND_TEXT: u32 = 0x002B_0EF0;
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

// Re:CoM data (RVAs). Checked (exe: the rip operands in `sites::SHOP_CHECKED`).
/// `*0x14087CBF8` = the save's stats; Moogle Points at `+0x20`.
const STATS_PTR: usize = 0x0087_CBF8;
const POINTS: usize = 0x20;
/// `*0x14087A7E8 + 6` (u16): the card limit of the room rule.
const CARD_LIMIT_PTR: usize = 0x0087_A7E8;
const CARD_LIMIT: usize = 6;
const TROPHY_FLAGS: usize = 0x00AF_7389;
const TROPHY_OBJ: usize = 0x00AF_7468;
/// Card class rows (`0x14068E0B0 + kind × 16`, byte 0).
const CARD_CLASS: usize = 0x0068_E0B0;
/// The tab x table (4 floats) and the window groups by tab (4 i16) then their closes (4 i16), the
/// only readers being the pack screen (`0x1403A6B3E`, `0x1403A9323` / `9434` / `9485`) and its
/// window (`0x14045D282`, `0x14045CCD9` / `CCFD`). Checked (exe scan).
const TAB_X_TABLE: usize = 0x0065_BA40;
const WINDOW_TABLE: usize = 0x0069_0420;
/// The pointer to `"m_shop.lad"` the shop's sequences play from (`0x14045D250`). Checked (decomp).
const SHOP_LAD: usize = 0x0085_9C48;

/// `CMoogleShopCore` fields (Checked (decomp + disasm: `0x1403A4220`, `0x1403A4A90`,
/// `0x1403A6930`, `0x1403A73E0`, `0x1403A92F0`, `0x1403A5B10`)).
mod f {
    pub const NEXT_MODE: usize = 0x25C;
    pub const STATE: usize = 0x260;
    /// The shop's task parent (the constructors' second argument).
    pub const TASKS: usize = 0x268;
    pub const LOCK: usize = 0x274;
    pub const TAB: usize = 0x280;
    pub const CELL: usize = 0x284;
    pub const PRICES: usize = 0x2C8;
    pub const SOLD_OUT: usize = 0x308;
    pub const COUNT: usize = 0x348;
    pub const IN_GRID: usize = 0x34C;
    /// `CMShopChargeCardData`; `+0x44` Sora's card count for the room rule.
    pub const CARD_DATA: usize = 0x350;
    pub const CARD_COUNT: usize = 0x44;
    pub const POINT_WINDOW: usize = 0x3C0;
    pub const CURSOR: usize = 0x3D0;
    pub const TAB_MARK: usize = 0x3D8;
    /// `CMShopTextWindow` (the Moogle's text).
    pub const TEXT: usize = 0x3E8;
    /// The pack screen's icons (`CMShopGSeq`).
    pub const ICONS: usize = 0x408;
    pub const TAB_MESSAGE: usize = 0x410;
    /// `CMShopCardPackWindow`.
    pub const WINDOW: usize = 0x418;
    pub const CELL_MESSAGE: usize = 0x420;
    pub const PACKS: usize = 0x428;
    /// The card trade's Square button hint (`CMShopButtonMessage`, made only with room).
    pub const TRADE_BUTTON: usize = 0x448;
}
/// `CMShopButtonMessage +0x60` / any shop window's show state (`0x1404598D0`).
const SHOW: usize = 0x60;
/// `CMShopCardPack`: its picture control (layer 1 the pack's art), its release flag (`+0x6C`).
const PACK_PICTURE: usize = 0x78;
const PACK_RELEASE: usize = 0x6C;
const PACK_SIZE: usize = 0xB8;
const PICTURE_ART_LAYER: i32 = 1;
/// `C2DCard`: its size (`0x14045BF50` allocates 400), the shop's card priority 0x12, "use the
/// colour" `+0x138`.
const CARD_SIZE: usize = 400;
const CARD_PRIORITY_SHOP: u32 = 0x12;
const C_COLOUR_ON: usize = 0x138;
/// `CMShopCardPackWindow` (`0x14045AD20`, `0x14045D250`, `0x14045CC60`): `+0x58` its phase (1 =
/// shown), `+0x5C` the tab it shows, `+0x60` close, `+0x68` its sequence control.
const WIN_PHASE: usize = 0x58;
const WIN_TAB: usize = 0x5C;
const WIN_CLOSE: usize = 0x60;
const WIN_CTRL: usize = 0x68;
/// `CMShopGSeq` (`0x140455F20`, `0x140457DF0`, `0x1404585B0`): `+0x5C` the group playing (−1 not
/// yet), `+0x68` its control, `+0x70` its sequence set, `+0x72` / `+0x74` / `+0x76` appear /
/// loop / close.
const GSEQ_PLAYING: usize = 0x5C;
const GSEQ_CTRL: usize = 0x68;
const GSEQ_SET: usize = 0x70;
const GSEQ_GROUPS: usize = 0x72;
/// `CMShopTextWindow`: `+0x58` 1 open; `+0x5C` its help id (`0x1404573A0`); `+0x60` its `CTextWnd`.
const TW_STATE: usize = 0x58;
const TW_HELP: usize = 0x5C;
const TW_MAIN: usize = 0x60;
/// `CTextWnd` state `+0x88` (5 shown).
const TEXTWND_STATE: usize = 0x88;
const SHOWN: i32 = 5;
/// The card trade's button hint: 0x88 bytes, kind 1 (`0x1403A73E0` case 5).
const BUTTON_SIZE: usize = 0x88;
const BUTTON_TRADE: u32 = 1;

/// The pack screen's states (`0x1403A6930`) and the card trade's (`0x1403A73E0`).
const STATE_FIRST: i32 = 0;
const STATE_INPUT: i32 = 2;
const STATE_CLOSE: i32 = 100;
const STATE_RELEASE: i32 = 0x65;
const TRADE_TABS: i32 = 1;
const TRADE_GRID: i32 = 2;

type ScreenFn = unsafe extern "win64" fn(usize);
type NoArgFn = unsafe extern "win64" fn() -> u32;
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
type ButtonCtorFn = unsafe extern "win64" fn(usize, usize, u32) -> usize;
type SetUpFn = unsafe extern "win64" fn(usize);
type PlayFn = unsafe extern "win64" fn(usize, u32, usize, u32, i32, f32, u32);
type TextFn = unsafe extern "win64" fn(usize, *const u8, usize);
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

/// Reads `kh1-shop-tabs.json` (the asset step's five-tab art, `crate::shop::tabs`). Without it the
/// pack screen keeps Re:CoM's four-tab window and the Items tab's cursor sits right of the tabs.
pub fn load_tabs(file: Option<&std::path::Path>) {
    let Some(path) = file else {
        log("shop: no kh1-shop-tabs.json: the pack screen keeps Re:CoM's four-tab art (the Items tab's cursor at x 320)");
        return;
    };
    match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| sh::tabs::parse(&t)) {
        Ok(first) => {
            ART.store(first, Ordering::Release);
            log(&format!("shop: five-tab art from {}: m_shop:01 groups {first}..{}", path.display(), first + sh::tabs::COUNT - 1));
        }
        Err(e) => log(&format!("shop: {}: {e}: the pack screen keeps Re:CoM's four-tab art", path.display())),
    }
}

fn art() -> Option<u32> {
    match ART.load(Ordering::Acquire) {
        0 => None,
        f => Some(f),
    }
}

/// KH1's items are live (Combat = KH1, Sora's story, a save) and the wrappers are in.
fn live() -> bool {
    INSTALLED.load(Ordering::Acquire) && super::items::installed() && super::mp::active()
}

fn lock() -> std::sync::MutexGuard<'static, Option<ItemsTab>> {
    TAB.lock().unwrap_or_else(|p| p.into_inner())
}

fn tab_up(shop: usize) -> bool {
    lock().as_ref().is_some_and(|t| t.shop == shop)
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

/// Re:CoM's room rule (`0x1403A752A`). No card data or limit: no room.
unsafe fn room(shop: usize) -> bool {
    let limit: usize = rd(base() + CARD_LIMIT_PTR);
    let data: usize = rd(shop + f::CARD_DATA);
    if limit == 0 || data == 0 {
        return false;
    }
    sh::room_for_pack(rd::<u16>(limit + CARD_LIMIT), rd::<i32>(data + f::CARD_COUNT))
}

/// Re:CoM's floor tier now (`0x1402337C0`): the shop's stock, and Lucky Bounty at KH1.
pub(super) unsafe fn tier() -> sh::Tier {
    let byte = func::<NoArgFn>(FLOOR)() as u8;
    sh::tier(sh::floor_of(byte))
}

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

// ---- The five-tab art ----

fn table_bytes(x: [f32; 4], w: [i16; 4], c: [i16; 4]) -> ([u8; 16], [u8; 16]) {
    let mut a = [0u8; 16];
    let mut b = [0u8; 16];
    for i in 0..4 {
        a[4 * i..4 * i + 4].copy_from_slice(&x[i].to_le_bytes());
        b[2 * i..2 * i + 2].copy_from_slice(&w[i].to_le_bytes());
        b[8 + 2 * i..10 + 2 * i].copy_from_slice(&c[i].to_le_bytes());
    }
    (a, b)
}

/// Re:CoM's tab tables as Re:CoM has them, or at the art's places (`crate::shop::tabs`). Written
/// only over one of the two (anything else: left alone, logged).
unsafe fn set_tables(kh1: Option<u32>) {
    let recom = table_bytes(sh::RECOM_TAB_X, sh::tabs::RECOM_WINDOW, sh::tabs::RECOM_CLOSE);
    let ours = kh1.map(|first| {
        let (x, w, c) = sh::tabs::kh1_tables(first);
        table_bytes(x, w, c)
    });
    let want = ours.unwrap_or(recom);
    for (at, recom, ours, want, what) in [
        (TAB_X_TABLE, recom.0, ours.map(|o| o.0), want.0, "shop tab x table"),
        (WINDOW_TABLE, recom.1, ours.map(|o| o.1), want.1, "shop window groups"),
    ] {
        let at = base() + at;
        let have: [u8; 16] = rd(at);
        if have == want {
            continue;
        }
        if have != recom && Some(have) != ours && !(at == base() + WINDOW_TABLE && closes_only_differ(&have, &want)) {
            note(|| format!("{what} at {at:#x} holds {have:02x?}: left alone"));
            continue;
        }
        super::hook::patch(at, &have, &want, what);
    }
}

/// The window table with only its close entries changed by us (the Items tab's close, below).
fn closes_only_differ(have: &[u8; 16], want: &[u8; 16]) -> bool {
    have[..8] == want[..8]
}

/// The window's close for the Items tab: Re:CoM's close plays `closes[window tab]`
/// (`0x14045CC60`), so the window is given tab 0 and tab 0's close entry the art's place-0 close
/// (until the next screen start writes the tables again).
unsafe fn items_close(shop: usize, first: u32) {
    let w: usize = rd(shop + f::WINDOW);
    if w == 0 {
        return;
    }
    wr::<i32>(w + WIN_TAB, 0);
    let at = base() + WINDOW_TABLE + 8;
    let have: [u8; 2] = rd(at);
    let want = ((first + sh::tabs::CLOSE) as i16).to_le_bytes();
    if have != want {
        super::hook::patch(at, &have, &want, "shop window close for the Items tab");
    }
}

/// Plays the art's window with place 0 raised for the Items tab, once the window is shown
/// (`0x14045D250`'s rule), and marks it as showing no Re:CoM tab (any Re:CoM tab plays its own
/// again).
unsafe fn items_window(shop: usize, t: &mut ItemsTab, first: u32) {
    let w: usize = rd(shop + f::WINDOW);
    if t.window || w == 0 || rd::<i32>(w + WIN_PHASE) != 1 || rd::<i32>(w + WIN_CLOSE) != 0 {
        return;
    }
    let ctrl: usize = rd(w + WIN_CTRL);
    let lad: usize = rd(base() + SHOP_LAD);
    if ctrl == 0 || lad == 0 {
        return;
    }
    let play: PlayFn = std::mem::transmute::<usize, PlayFn>(rd::<usize>(rd::<usize>(ctrl) + 0x18));
    play(ctrl, 0, lad, 1, (first + sh::tabs::WINDOW) as i32, 0.0, 0);
    wr::<i32>(w + WIN_TAB, sh::ITEMS_TAB);
    t.window = true;
}

/// The icons' sequence plays the art's icons (Re:CoM's mode-4 init gave it 0x38 / 0x39 / 0x3A).
unsafe fn icons(shop: usize, first: u32) {
    let g: usize = rd(shop + f::ICONS);
    if g == 0 {
        return;
    }
    let have: [i16; 3] = rd(g + GSEQ_GROUPS);
    if have != sh::tabs::RECOM_ICONS {
        return;
    }
    let ours = [sh::tabs::ICONS_IN, sh::tabs::ICONS, sh::tabs::ICONS_OUT].map(|o| (first + o) as i16);
    wr::<[i16; 3]>(g + GSEQ_GROUPS, ours);
    if rd::<i32>(g + GSEQ_PLAYING) != -1 {
        // Already started on Re:CoM's: our appear from the start.
        let ctrl: usize = rd(g + GSEQ_CTRL);
        let lad: usize = rd(base() + SHOP_LAD);
        if ctrl != 0 && lad != 0 {
            let play: PlayFn = std::mem::transmute::<usize, PlayFn>(rd::<usize>(rd::<usize>(ctrl) + 0x18));
            play(ctrl, 0, lad, rd::<i16>(g + GSEQ_SET) as u32, ours[0] as i32, 0.0, 0);
        }
    }
}

// ---- The packs and their cards ----

unsafe fn pack(shop: usize, i: usize) -> usize {
    rd(shop + f::PACKS + i * 8)
}

/// Releases the packs in the array (as state 0x65: `+0x6C` = 1, the entry cleared).
unsafe fn release_packs(shop: usize, n: usize) {
    for i in 0..n.min(sh::CELLS) {
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

/// Re:CoM's card picture of an item card on `pack`'s picture control, as the shop's own card in
/// `0x14045BF50` (priority 0x12, colour on), as the "cards got" screen makes its cards.
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

/// Into the Items tab from Re:CoM's tab `from` (its packs, `released` of them, already made).
unsafe fn enter(shop: usize, from: i32, count: i32, released: usize) {
    release_packs(shop, released);
    let items = sh::buy_list(tier());
    let mut cards = [0usize; sh::CELLS];
    let mut made = 0;
    for (i, &item) in items.iter().take(sh::CELLS).enumerate() {
        let p = make_pack(shop, i as i32, sh::buy_price(item).unwrap_or(0), 0);
        wr::<usize>(shop + f::PACKS + i * 8, p);
        if p == 0 {
            break;
        }
        made += 1;
        func::<SlotVisibleFn>(SLOT_VISIBLE)(rd(p + PACK_PICTURE), PICTURE_ART_LAYER, 0);
        if let Some(k) = sh::card_kind(item) {
            cards[i] = make_card(p, k);
        }
    }
    for i in made..sh::CELLS {
        wr::<usize>(shop + f::PACKS + i * 8, 0);
    }
    let n = items.len().min(sh::CELLS);
    *lock() = Some(ItemsTab { shop, count, items: items.clone(), cards, note: None, help: Vec::new(), old: Vec::new(), window: false });
    if made < n {
        note(|| format!("Items tab: only {made} of {n} packs made (no memory); back to tab {from}"));
        wr::<i32>(shop + f::COUNT, made as i32);
        leave_to(shop, from.clamp(0, 3));
        return;
    }
    wr::<i32>(shop + f::COUNT, n as i32);
    wr::<i32>(shop + f::TAB, sh::ITEMS_TAB);
    wr::<i32>(shop + f::CELL, rd::<i32>(shop + f::CELL).clamp(0, n as i32 - 1));
    refresh(shop);
    note(|| format!("Items tab from tab {from}: Re:CoM's {count} packs released, {:?}; cards {cards:#x?}; {} Moogle Points", items.iter().map(|&i| crate::items::name(i)).collect::<Vec<_>>(), points()));
}

/// Takes the Items tab's state, deletes its cards; None when it isn't up for `shop`.
unsafe fn take_tab(shop: usize) -> Option<ItemsTab> {
    let t = lock().take_if(|t| t.shop == shop)?;
    for c in t.cards {
        delete_card(c);
    }
    Some(t)
}

/// From the Items tab to Re:CoM's tab `to`: Re:CoM's packs for it as state 0 makes them, then
/// Re:CoM's refresh. Re:CoM's text window line is Re:CoM's again (its help id forgotten).
unsafe fn leave_to(shop: usize, to: i32) {
    let Some(t) = take_tab(shop) else { return };
    let ours: i32 = rd(shop + f::COUNT);
    release_packs(shop, ours.clamp(0, sh::CELLS as i32) as usize);
    let n = t.count.clamp(0, 4);
    for i in 0..n {
        let k = (to * 4 + i) as usize * 4;
        let p = make_pack(shop, i, rd(shop + f::PRICES + k), rd(shop + f::SOLD_OUT + k));
        wr::<usize>(shop + f::PACKS + i as usize * 8, p);
    }
    wr::<i32>(shop + f::COUNT, t.count);
    wr::<i32>(shop + f::TAB, to);
    wr::<i32>(shop + f::CELL, rd::<i32>(shop + f::CELL).clamp(0, (t.count - 1).max(0)));
    forget_help(shop);
    func::<ScreenFn>(REFRESH)(shop);
    note(|| format!("Items tab -> tab {to}: Re:CoM's {} packs made again", t.count));
}

/// Re:CoM's help line call sets its message only when the id changes (`0x1404573A0`): forgetting
/// it makes Re:CoM's next call set Re:CoM's line again.
unsafe fn forget_help(shop: usize) {
    let tw: usize = rd(shop + f::TEXT);
    if tw != 0 {
        wr::<i32>(tw + TW_HELP, -1);
    }
}

/// Re:CoM's refresh (`0x1403A92F0`) for the Items tab: the button messages at the tab and the
/// cell, the packs' prices (never sold out), the cursors, the window.
unsafe fn refresh(shop: usize) {
    let five = art();
    let x = sh::tab_x(sh::ITEMS_TAB, five.is_some());
    let n = rd::<i32>(shop + f::COUNT).clamp(0, sh::CELLS as i32);
    let cell = rd::<i32>(shop + f::CELL).clamp(0, (n - 1).max(0)) as usize;
    let place = func::<PlaceFn>(MESSAGE_PLACE);
    place(rd(shop + f::TAB_MESSAGE), x, sh::TAB_MESSAGE_Y);
    place(rd(shop + f::CELL_MESSAGE), sh::CELL_X[cell], sh::CELL_Y[cell]);
    let items = lock().as_ref().filter(|t| t.shop == shop).map(|t| t.items.clone()).unwrap_or_default();
    for (i, &item) in items.iter().take(n as usize).enumerate() {
        let p = pack(shop, i);
        if p != 0 {
            func::<PackIntFn>(PACK_PRICE)(p, sh::buy_price(item).unwrap_or(0));
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
    if let Some(first) = five {
        let mut g = lock();
        if let Some(t) = g.as_mut().filter(|t| t.shop == shop) {
            items_window(shop, t, first);
        }
    }
    help(shop);
}

/// The Items tab's line in the Moogle's text window (Re:CoM's text window, its own text call).
unsafe fn help(shop: usize) {
    let in_grid = rd::<i32>(shop + f::IN_GRID) != 0;
    let cell = rd::<i32>(shop + f::CELL).max(0) as usize;
    let inv = super::items::inventory().unwrap_or_default();
    let mut g = lock();
    let Some(t) = g.as_mut().filter(|t| t.shop == shop) else { return };
    let line = sh::help_text(in_grid, t.items.get(cell).copied(), t.note, &inv);
    if line == t.help {
        return;
    }
    let tw: usize = rd(shop + f::TEXT);
    if tw == 0 || rd::<i32>(tw + TW_STATE) != 1 {
        return;
    }
    let main: usize = rd(tw + TW_MAIN);
    if main == 0 || rd::<i32>(main + TEXTWND_STATE) != SHOWN {
        return;
    }
    func::<TextFn>(TEXTWND_TEXT)(main, line.as_ptr(), 0);
    wr::<i32>(tw + TW_HELP, -1);
    let old = std::mem::replace(&mut t.help, line);
    if !old.is_empty() {
        t.old.push(old);
    }
    while t.old.len() > 2 {
        t.old.remove(0);
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
        func::<PackIntFn>(POINT_WINDOW)(w, points());
    }
}

unsafe fn buy(shop: usize, cell: i32) {
    let item = lock().as_ref().filter(|t| t.shop == shop).and_then(|t| t.items.get(cell.max(0) as usize).copied());
    let Some(item) = item else {
        sound(sh::sound::ERROR);
        return;
    };
    let have = points();
    let r = super::items::edit(|inv| sh::buy(item, have, inv));
    let n = match r {
        Some(Buy::Bought { price, points: after, to, .. }) => {
            sound(sh::sound::BUY);
            sound(sh::sound::BUY_TOO);
            pay(shop, price, after);
            note(|| format!("bought {} for {price}: {have} -> {} Moogle Points, {to:?}", crate::items::name(item), points()));
            None
        }
        Some(Buy::Poor { .. }) => Some(Note::Poor),
        Some(Buy::NoRoom { .. }) => Some(Note::NoRoom),
        _ => None,
    };
    if !matches!(r, Some(Buy::Bought { .. })) {
        sound(sh::sound::ERROR);
        note(|| format!("refused {} on cell {cell} with {have} Moogle Points: {r:?}", crate::items::name(item)));
    }
    if let Some(t) = lock().as_mut().filter(|t| t.shop == shop) {
        t.note = n;
    }
}

fn clear_note(shop: usize) {
    if let Some(t) = lock().as_mut().filter(|t| t.shop == shop) {
        t.note = None;
    }
}

/// One frame of the Items tab's input state.
unsafe fn items_frame(shop: usize) {
    let in_grid = rd::<i32>(shop + f::IN_GRID) != 0;
    let cell: i32 = rd(shop + f::CELL);
    let count: i32 = rd(shop + f::COUNT);
    let locked = rd::<i32>(shop + f::LOCK) > 0;
    let pad = if locked { Pad::default() } else { read_pad() };
    match sh::items_step(in_grid, cell, count, &pad, locked) {
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
            clear_note(shop);
        }
        Step::OutOfGrid => {
            sound(sh::sound::MOVE);
            wr::<i32>(shop + f::IN_GRID, 0);
            message_show(shop, -1);
            clear_note(shop);
        }
        Step::Move(c) => {
            sound(sh::sound::MOVE);
            wr::<i32>(shop + f::CELL, c);
            clear_note(shop);
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
        wr::<i32>(m + SHOW, v);
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
        STATE_FIRST => {
            if up {
                // A new screen on the same object with our tab still recorded: it never passed
                // state 0x65 (not seen in Re:CoM's code). Its cards went with the old screen.
                *lock() = None;
                note(|| "a new pack screen with the Items tab still recorded: forgotten".into());
            }
            let kh1 = live();
            set_tables(art().filter(|_| kh1));
            if !kh1 {
                return run(shop);
            }
            if let Some(first) = art() {
                icons(shop, first);
            }
            // The screen opens on the Items tab: Re:CoM's state 0 makes no packs (count 0), ours
            // are made as it moves on.
            let count: i32 = rd(shop + f::COUNT);
            wr::<i32>(shop + f::COUNT, 0);
            run(shop);
            wr::<i32>(shop + f::COUNT, count);
            if rd::<i32>(shop + f::STATE) != STATE_FIRST {
                enter(shop, 0, count, 0);
            }
        }
        STATE_INPUT if up => items_frame(shop),
        STATE_INPUT if live() => {
            let tab: i32 = rd(shop + f::TAB);
            let in_grid = rd::<i32>(shop + f::IN_GRID) != 0;
            let locked = rd::<i32>(shop + f::LOCK) > 0;
            if (0..sh::RECOM_TABS).contains(&tab) && !in_grid && !locked && sh::enters_items(tab, in_grid, &read_pad(), locked) {
                sound(sh::sound::MOVE);
                let count: i32 = rd(shop + f::COUNT);
                enter(shop, tab, count, count.clamp(0, 4) as usize);
            } else {
                run(shop)
            }
        }
        STATE_CLOSE if up => {
            // Our cards go before Re:CoM starts closing the packs they hang from; the window
            // closes as the Items tab's.
            if let Some(t) = lock().as_mut().filter(|t| t.shop == shop) {
                for c in std::mem::take(&mut t.cards) {
                    delete_card(c);
                }
            }
            if let Some(first) = art() {
                items_close(shop, first);
            }
            run(shop)
        }
        STATE_RELEASE if up => {
            run(shop);
            if rd::<i32>(shop + f::STATE) != STATE_RELEASE {
                // Re:CoM released our packs (the count was ours): its count back.
                if let Some(t) = take_tab(shop) {
                    wr::<i32>(shop + f::COUNT, t.count);
                    note(|| format!("pack screen left from the Items tab: Re:CoM's count {} back", t.count));
                }
            }
        }
        _ => run(shop),
    }
}

/// The card trade's Square button hint, made as Re:CoM's case 5 makes it (`0x1403A7F8E`), when
/// Re:CoM made none (no room for 5 cards).
unsafe fn trade_button(shop: usize) {
    if rd::<usize>(shop + f::TRADE_BUTTON) != 0 {
        return;
    }
    let mem = func::<NewFn>(NEW)(BUTTON_SIZE);
    if mem == 0 {
        return;
    }
    let b = func::<ButtonCtorFn>(BUTTON_CTOR)(mem, rd(shop + f::TASKS), BUTTON_TRADE);
    if b == 0 {
        return;
    }
    wr::<usize>(shop + f::TRADE_BUTTON, b);
    let set_up: SetUpFn = std::mem::transmute::<usize, SetUpFn>(rd::<usize>(rd::<usize>(b) + 0x38));
    set_up(b);
    func::<PackIntFn>(SHOW_STATE)(b, 0);
}

/// The wrapper on the card trade's update (`0x1403A73E0`): at Combat = KH1 its Square opens the
/// pack screen also without room for 5 cards (`crate::shop::trade_square_opens_packs`).
///
/// # Safety
/// Called by Re:CoM with its `CMoogleShopCore`.
pub unsafe extern "win64" fn trade(shop: usize) {
    let run = orig::<ScreenFn>(&ORIG_TRADE);
    if !sh::trade_square_opens_packs(live(), room(shop)) || room(shop) {
        return run(shop);
    }
    let state: i32 = rd(shop + f::STATE);
    match state {
        STATE_FIRST => {
            trade_button(shop);
            run(shop)
        }
        TRADE_TABS | TRADE_GRID => {
            // Re:CoM's own reads before the Square (`0x1403A752A`): its lock decrement ran at the
            // function's head, so the lock is read as Re:CoM's Square test reads it (≤ 0 after).
            let lock_now = rd::<i32>(shop + f::LOCK);
            let cancel_first = lock_now < 2 && func::<PadBtnFn>(PAD_CANCEL)(0) != 0;
            if !cancel_first && lock_now < 2 && func::<PadMaskFn>(PAD_FRESH)(0x80, 0) != 0 {
                if lock_now > 0 {
                    wr::<i32>(shop + f::LOCK, lock_now - 1);
                }
                sound(sh::sound::OTHER);
                wr::<i32>(shop + f::NEXT_MODE, sh::mode::PACKS);
                wr::<i32>(shop + f::STATE, STATE_CLOSE);
                note(|| "card trade: Square to the pack screen without room for 5 cards (its Items tab)".into());
                return;
            }
            run(shop)
        }
        _ => run(shop),
    }
}

pub fn start_line() -> String {
    "shop: KH1's items in Re:CoM's own Moogle shop screens (M8d): at Combat = KH1 in Sora's story the pack screen opens on an Items tab drawn before Re:CoM's four (Re:CoM's packs with Re:CoM's item cards and price plates; 1F Potion 10 / Ether 40, 2F+ Hi-Potion 30, 7F+ Elixir 200 Moogle Points; KH1's pickup); the card trade's Square opens it also without room for 5 cards".into()
}
