//! M8c: KH1's Moogle shop as a list, in Re:CoM's own `CMoogleShopCore` (`crate::shop`,
//! `docs/recom-m8-items.md` § "M8c: KH1 Moogle shop list").
//!
//! Re:CoM's shop is a state machine of 7 modes, each a record (init / update / end) in the shop
//! object at `+0x98 + 0x40 · mode` that Re:CoM's own `CFSM` step (`0x140242F50`) runs. At Combat =
//! KH1 in Sora's story mode 3 (Re:CoM's two-choice menu: card packs or the card trade) becomes our
//! list:
//!
//! - **The init** (`0x1403A53F0`, wrapped): Re:CoM's own first-visit rule first (with room for 5
//!   cards and the free pack not yet given, Re:CoM's init sends the shop to mode 1, unchanged);
//!   then the shop's own mode-3 update slot (`+0x178`, this shop object's data) is pointed at
//!   [`list_update`] and the windows are set up as Re:CoM's mode-3 init sets them for its first
//!   choice (frame 2, the Moogle Point window, help bar 2, the Moogle models, text window message
//!   2, the cursors hidden). Without room for 5 cards the list still opens (items need no card
//!   room); Re:CoM's init would go to mode 2.
//! - **The update** ([`list_update`], run by Re:CoM's state machine as mode 3's update): Re:CoM's
//!   mode-3 states 0 (wait for the windows, open the rows window), 2 (input), 3 / 4 (close, then
//!   switch to the next mode as Re:CoM does: `+0x258` and the pending state `+0x68`), 100 / 0x65
//!   (leave the shop: mode 6). The rows are a `CTextWnd` (Re:CoM's text window class) opened with
//!   Re:CoM's explicit open `0x1402B1420`; the cursor is the shop's own finger (`+0x3D0`); the
//!   highlighted row's line goes into the shop's text window (`+0x3E8`) as Re:CoM's own help line
//!   call does (`0x1404573A0`: `0x1402B0EF0` on its `CTextWnd`).
//! - **Card Packs** goes to Re:CoM's mode 4 (its pack screen, unpatched), or to its mode 2 when
//!   there's no room for 5 cards (Re:CoM's rule). Both come back to mode 3: our list again.
//! - **Buy** pays through Re:CoM's own payment (8a.3: the spent tally and its trophy, the setter
//!   `0x14023A1D0`, the Moogle Point window); **Sell** adds through the setter.
//!
//! At Combat = Re:CoM or in Riku's story the wrapper calls Re:CoM's init and puts Re:CoM's update
//! back if a KH1 shop left ours there: Re:CoM's shop as it is. Nothing of ours runs outside the
//! shop's mode 3.

use super::movement::{func, orig, rd, wr};
use super::{base, log};
use crate::shop::{self as sh, Act, Buy, Ctx, Menu, Note, Pad, Sell};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_MODE3_INIT: AtomicUsize = AtomicUsize::new(0);

static INSTALLED: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(80);
/// The list of the shop that is open (one shop at a time).
static LIST: Mutex<Option<List>> = Mutex::new(None);

struct List {
    shop: usize,
    menu: Menu,
    /// Our rows window (a `CTextWnd`), 0 when none.
    rows: usize,
    /// The texts Re:CoM's windows point at now, and the ones before (Re:CoM's text set keeps the
    /// pointer, `0x140259000`; the old ones go once a newer one is set).
    rows_text: Vec<u8>,
    help_text: Vec<u8>,
    old: Vec<Vec<u8>>,
    note: Option<Note>,
    /// The mode Card Packs goes to.
    next: i32,
    finger_row: Option<usize>,
}

impl List {
    fn new(shop: usize, root: usize) -> List {
        List { shop, menu: Menu::new(root), rows: 0, rows_text: Vec::new(), help_text: Vec::new(), old: Vec::new(), note: None, next: sh::mode::PACKS, finger_row: None }
    }

    fn retire(&mut self, v: Vec<u8>) {
        if !v.is_empty() {
            self.old.push(v);
        }
        // Keep one generation back of each text.
        while self.old.len() > 2 {
            self.old.remove(0);
        }
    }
}

// Re:CoM functions (RVAs; first bytes checked: `sites::SHOP_CALLEES`, and `CALLEES` etc. for the
// shared ones).
const NEW: u32 = 0x004A_CE54;
const SOUND: u32 = 0x002A_A360;
const PAD_REPEAT: u32 = 0x0029_B2E0;
const PAD_CANCEL: u32 = 0x0029_B410;
const PAD_CONFIRM: u32 = 0x0029_B490;
const OPENED: u32 = 0x003A_60F0;
const CLOSED: u32 = 0x003A_61F0;
const CLOSE: u32 = 0x003A_6640;
const SHOP_FLAGS: u32 = 0x0011_4180;
const FLOOR: u32 = 0x0023_37C0;
const FRAME_SET: u32 = 0x0045_98C0;
const HELP_BAR: u32 = 0x0045_9CC0;
const MODEL_MODE: u32 = 0x0045_F3A0;
const MOOGLE_POSE: u32 = 0x0045_F1E0;
const BOTTLE_POSE: u32 = 0x0045_F130;
const TEXT_MESSAGE: u32 = 0x0045_95A0;
const BG_CLOSE: u32 = 0x0045_9C30;
const BG_CLOSED: u32 = 0x0045_82B0;
const LEAVE_FADE: u32 = 0x002A_A9B0;
const CURSOR_LOOK: u32 = 0x0045_9D60;
const CURSOR_PRIORITY: u32 = 0x0045_9890;
const CURSOR_PLACE: u32 = 0x0045_9A00;
const CURSOR_VISIBLE: u32 = 0x0045_97D0;
const POINT_WINDOW: u32 = 0x0045_98A0;
const SHOW_STATE: u32 = 0x0045_98D0;
const TEXTWND_CTOR: u32 = 0x002A_FF30;
const TEXTWND_OPEN: u32 = 0x002B_1420;
const TEXTWND_START: u32 = 0x002B_2C80;
const TEXTWND_SPEED: u32 = 0x002B_1980;
const TEXTWND_TEXT: u32 = 0x002B_0EF0;
const TEXTWND_CLOSE: u32 = 0x002B_1310;
const POINTS_SET: u32 = 0x0023_A1D0;
const VAR_GET: u32 = 0x001B_08C0;
const VAR_SET: u32 = 0x001B_16C0;
const TROPHY: u32 = 0x002B_A6F0;
/// Re:CoM's mode-3 update (the ctor writes it at `+0x178`, `0x1403A4440`). Checked (exe).
const RECOM_MODE3_UPDATE: usize = 0x003A_8EF0;

// Re:CoM data (RVAs). Checked (exe: the rip operands in `sites::SHOP_CHECKED`).
/// `*0x14087CBF8` = the save's stats; Moogle Points at `+0x20`.
const STATS_PTR: usize = 0x0087_CBF8;
const POINTS: usize = 0x20;
/// `*0x14087A7E8 + 6` (u16): the card limit mode 3's room rule compares with.
const CARD_LIMIT_PTR: usize = 0x0087_A7E8;
const CARD_LIMIT: usize = 6;
/// The trophy bit `& 0x10` (set once the 20000-points trophy is given) and the trophy object.
const TROPHY_FLAGS: usize = 0x00AF_7389;
const TROPHY_OBJ: usize = 0x00AF_7468;

/// `CMoogleShopCore` fields (Checked (decomp + disasm: `0x1403A4220`, `0x1403A53F0`,
/// `0x1403A8EF0`, `0x140242F50`)).
mod f {
    /// The `CFSM`'s pending state (a record; the step runs its init next frame).
    pub const PENDING: usize = 0x68;
    /// The mode records: `+0x98 + 0x40 · mode`; mode 3's update slot.
    pub const RECORDS: usize = 0x98;
    pub const RECORD: usize = 0x40;
    pub const MODE3_UPDATE: usize = RECORDS + 3 * RECORD + 0x20;
    pub const MODE: usize = 0x258;
    pub const NEXT_MODE: usize = 0x25C;
    pub const STATE: usize = 0x260;
    /// The modes' task parent (`CMShopModeTaskParent`); `+0x50` its live children.
    pub const TASKS: usize = 0x268;
    pub const TASKS_LIVE: usize = 0x50;
    /// Input lock: no reads while above 0.
    pub const LOCK: usize = 0x274;
    /// `CMShopChargeCardData`; `+0x44` Sora's card count for the room rule.
    pub const CARD_DATA: usize = 0x350;
    pub const CARD_COUNT: usize = 0x44;
    pub const MOOGLE: usize = 0x3A0;
    pub const BOTTLE: usize = 0x3A8;
    pub const BG: usize = 0x3B0;
    pub const FRAME: usize = 0x3B8;
    pub const POINT_WINDOW: usize = 0x3C0;
    pub const HELP_BAR: usize = 0x3C8;
    /// The finger cursor and the shop's other two cursors.
    pub const CURSORS: [usize; 3] = [0x3D0, 0x3D8, 0x3E0];
    /// `CMShopTextWindow` (the Moogle's text).
    pub const TEXT: usize = 0x3E8;
}
/// `CMShopTextWindow`: `+0x58` 1 open, −1 closed; `+0x60` its main `CTextWnd`.
const TW_STATE: usize = 0x58;
const TW_MAIN: usize = 0x60;
/// `CTextWnd`: its size (`0x1404595A0` allocates 0x148) and state `+0x88` (5 shown, 6.. closing,
/// over 8 closed; `0x1402B1310`, `0x140459440`).
const TEXTWND_SIZE: usize = 0x148;
const TEXTWND_STATE: usize = 0x88;
const SHOWN: i32 = 5;
const GONE: i32 = 8;
/// The explicit open's arguments as Re:CoM's own caller `0x1402995A0` passes them (frame 0 = no
/// `CWnd` box drawn, colour 0x80808080, offset (0, 0), 0, flags 0x300B; Checked (disasm)), but
/// placement 0: `0x1402B1F70` keeps the given x for placement 0 and replaces it for 1 (x 32), 2
/// (centred: 256 − w / 2, `0x1402995A0`'s) and 3 (right: 480 − w). Checked (decomp).
const ROWS_FRAME: u32 = 0;
const ROWS_COLOUR: u32 = 0x8080_8080;
const ROWS_PLACEMENT: u32 = 0;
const ROWS_FLAGS: u32 = 0x300B;
/// Mode 3's frame and help-bar ids and text message for its first choice (`0x1403A53F0`), the
/// finger's look and priority as mode 2's list sets them (`0x1403A51C0`). Checked (disasm).
const FRAME_MENU: i32 = 2;
const HELP_MENU: i32 = 2;
const MODELS_MENU: i32 = 2;
const MESSAGE_MENU: i32 = 2;
const FINGER_LOOK: i32 = 0;
const FINGER_PRIORITY: u32 = 0x16;

/// The states (`+0x260`) of Re:CoM's mode 3, which ours keeps.
const S_OPEN: i32 = 0;
const S_SHOWN: i32 = 1;
const S_INPUT: i32 = 2;
const S_CLOSE: i32 = 3;
const S_SWITCH: i32 = 4;
const S_LEAVE: i32 = 100;
const S_LEFT: i32 = 0x65;

type ShopFn = unsafe extern "win64" fn(usize);
type ShopIntFn = unsafe extern "win64" fn(usize, i32);
type ShopTestFn = unsafe extern "win64" fn(usize, i32) -> i32;
type ShopClosedFn = unsafe extern "win64" fn(usize) -> i32;
type NoArgFn = unsafe extern "win64" fn() -> u32;
type NewFn = unsafe extern "win64" fn(usize) -> usize;
type SoundFn = unsafe extern "win64" fn(u32, i32) -> u64;
type FadeFn = unsafe extern "win64" fn(i32, i32);
type PadMaskFn = unsafe extern "win64" fn(u32, u32) -> u32;
type PadBtnFn = unsafe extern "win64" fn(u32) -> u32;
type PlaceFn = unsafe extern "win64" fn(usize, f32, f32, i32);
type VisibleFn = unsafe extern "win64" fn(usize, u32);
type PriorityFn = unsafe extern "win64" fn(usize, u32);
type CtorFn = unsafe extern "win64" fn(usize, usize, u32) -> usize;
#[allow(clippy::type_complexity)]
type OpenFn = unsafe extern "win64" fn(usize, *const i32, *const i32, u32, *const u8, u32, u32, u32, *const i32, u32, u32);
type TextFn = unsafe extern "win64" fn(usize, *const u8, usize);
type WndFn = unsafe extern "win64" fn(usize);
type DeleteFn = unsafe extern "win64" fn(usize, u32) -> usize;
type PointsSetFn = unsafe extern "win64" fn(i32) -> u64;
type VarGetFn = unsafe extern "win64" fn(u32, u32) -> u64;
type VarSetFn = unsafe extern "win64" fn(u32, u32, i32);
type TrophyFn = unsafe extern "win64" fn(usize, u32);

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("shop: {}", line()));
    }
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

/// The KH1 list is offered: the wrapper is in and KH1's items are live (Combat = KH1, Sora's
/// story, a save).
fn live() -> bool {
    INSTALLED.load(Ordering::Acquire) && super::items::installed() && super::mp::active()
}

fn lock() -> std::sync::MutexGuard<'static, Option<List>> {
    LIST.lock().unwrap_or_else(|p| p.into_inner())
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

/// Re:CoM's room rule (mode 3's init, `0x1403A53F0`). No card data or limit: no room (Re:CoM
/// would read through them; not seen null in the shop).
unsafe fn room(shop: usize) -> bool {
    let limit: usize = rd(base() + CARD_LIMIT_PTR);
    let data: usize = rd(shop + f::CARD_DATA);
    if limit == 0 || data == 0 {
        return false;
    }
    sh::room_for_pack(rd::<u16>(limit + CARD_LIMIT), rd::<i32>(data + f::CARD_COUNT))
}

/// The free pack was given (`0x140114180()` bit 16, which Re:CoM's mode 1 sets).
unsafe fn free_pack_given() -> bool {
    func::<NoArgFn>(SHOP_FLAGS)() >> 16 & 1 != 0
}

/// Re:CoM's floor tier now (`0x1402337C0`): the shop's stock, and Lucky Bounty at KH1.
pub(super) unsafe fn tier() -> sh::Tier {
    let byte = func::<NoArgFn>(FLOOR)() as u8;
    sh::tier(sh::floor_of(byte))
}

/// An item's name: KH1's word as the command menu's Items list shows it, when it is plain ASCII;
/// else our English name.
fn item_name(item: u8) -> String {
    let english = || String::from_utf8_lossy(crate::items_page::text::english(item)).trim_end_matches('\0').to_string();
    let Some(w) = crate::items::data(item).and_then(|d| super::command_menu::word(d.name)) else { return english() };
    let w: Vec<u8> = w.into_iter().take_while(|&b| b != 0).collect();
    if !w.is_empty() && w.iter().all(|b| (0x20..0x7F).contains(b)) {
        String::from_utf8_lossy(&w).into_owned()
    } else {
        english()
    }
}

unsafe fn read_pad() -> Pad {
    let dir = |mask: u32| func::<PadMaskFn>(PAD_REPEAT)(mask, 0) != 0;
    Pad {
        cancel: func::<PadBtnFn>(PAD_CANCEL)(0) != 0,
        confirm: func::<PadBtnFn>(PAD_CONFIRM)(0) != 0,
        up: dir(0x0010_1000),
        down: dir(0x0020_4000),
    }
}

unsafe fn ctx() -> (Ctx, crate::items::Inventory) {
    let inv = super::items::inventory().unwrap_or_default();
    (Ctx { buy: sh::buy_list(tier()), sell: sh::sell_list(&inv) }, inv)
}

unsafe fn finger(shop: usize, row: Option<usize>) {
    let c: usize = rd(shop + f::CURSORS[0]);
    if c == 0 {
        return;
    }
    if let Some(i) = row {
        func::<PlaceFn>(CURSOR_PLACE)(c, sh::FINGER_X, sh::row_y(i), 0);
    }
    func::<VisibleFn>(CURSOR_VISIBLE)(c, row.is_some() as u32);
}

/// The window set-up of Re:CoM's mode-3 init for its first choice (`0x1403A5468..0x1403A5565`),
/// then the finger as mode 2's list prepares it (`0x1403A51C0`).
unsafe fn set_up_windows(shop: usize) {
    wr::<i32>(shop + f::LOCK, 0);
    wr::<i32>(shop + f::STATE, S_OPEN);
    let p = |o: usize| -> usize { rd(shop + o) };
    func::<ShopIntFn>(FRAME_SET)(p(f::FRAME), FRAME_MENU);
    func::<ShopIntFn>(SHOW_STATE)(p(f::POINT_WINDOW), 0);
    func::<ShopIntFn>(HELP_BAR)(p(f::HELP_BAR), HELP_MENU);
    func::<ShopIntFn>(SHOW_STATE)(p(f::HELP_BAR), 0);
    func::<ShopIntFn>(MODEL_MODE)(p(f::MOOGLE), MODELS_MENU);
    func::<ShopIntFn>(MODEL_MODE)(p(f::BOTTLE), MODELS_MENU);
    func::<ShopIntFn>(TEXT_MESSAGE)(p(f::TEXT), MESSAGE_MENU);
    func::<ShopIntFn>(MOOGLE_POSE)(p(f::MOOGLE), 0);
    func::<ShopIntFn>(BOTTLE_POSE)(p(f::BOTTLE), 1);
    for c in f::CURSORS {
        func::<VisibleFn>(CURSOR_VISIBLE)(p(c), 0);
    }
    let c = p(f::CURSORS[0]);
    if c != 0 {
        func::<ShopIntFn>(CURSOR_LOOK)(c, FINGER_LOOK);
        func::<PriorityFn>(CURSOR_PRIORITY)(c, FINGER_PRIORITY);
    }
}

/// The wrapper on mode 3's init (`0x1403A53F0`).
///
/// # Safety
/// Called by Re:CoM's state machine with its `CMoogleShopCore`.
pub unsafe extern "win64" fn mode3_init(shop: usize) {
    let run = orig::<ShopFn>(&ORIG_MODE3_INIT);
    let slot = shop + f::MODE3_UPDATE;
    let recom = base() + RECOM_MODE3_UPDATE;
    let ours = list_update as *const () as usize;
    let have: usize = rd(slot);
    if !live() {
        if have == ours {
            wr::<usize>(slot, recom);
        }
        return run(shop);
    }
    if have != recom && have != ours {
        note(|| format!("mode 3's update slot holds {have:#x}, not Re:CoM's 0x1403A8EF0: Re:CoM's menu"));
        return run(shop);
    }
    let room = room(shop);
    if room && !free_pack_given() {
        // Re:CoM's first visit: its init sends the shop to mode 1 (the free pack), which comes
        // back to mode 3 (this init again).
        if have == ours {
            wr::<usize>(slot, recom);
        }
        note(|| "first visit: Re:CoM's free pack (mode 1) before the list".into());
        return run(shop);
    }
    wr::<usize>(slot, ours);
    set_up_windows(shop);
    let mut g = lock();
    // Back from the pack screen or the card trade: the root cursor where it was.
    let root = g.as_ref().filter(|l| l.shop == shop).map_or(0, |l| l.menu.root);
    *g = Some(List::new(shop, root));
    drop(g);
    note(|| format!("KH1 list (mode 3): floor tier {:?}, {} Moogle Points, room for a pack: {room}", tier(), points()));
}

/// Opens our rows window with `text` (Re:CoM's text window class, its explicit open).
unsafe fn open_rows(shop: usize, text: &[u8]) -> usize {
    let parent: usize = rd(shop + f::TEXT);
    if parent == 0 {
        return 0;
    }
    let mem = func::<NewFn>(NEW)(TEXTWND_SIZE);
    if mem == 0 {
        return 0;
    }
    let w = func::<CtorFn>(TEXTWND_CTOR)(mem, parent, 0);
    if w == 0 {
        return 0;
    }
    let (pos, size) = sh::rows_window(sh::MAX_ROWS);
    let offset = [0i32; 2];
    func::<OpenFn>(TEXTWND_OPEN)(w, pos.as_ptr(), size.as_ptr(), ROWS_FRAME, text.as_ptr(), ROWS_COLOUR, sh::ROW_LINE, ROWS_PLACEMENT, offset.as_ptr(), 0, ROWS_FLAGS);
    // As the shop's own text windows after their open (`0x1404595A0`: `0x1402B1980(w, 0)` for
    // every message but 0; Assumption: 0 = the text shown at once, not typed out), then started.
    func::<ShopIntFn>(TEXTWND_SPEED)(w, 0);
    func::<WndFn>(TEXTWND_START)(w);
    w
}

/// Closes our rows window; true once it is gone (deleted, as `CMShopTextWindow` deletes its own:
/// vtable slot 0, flag 1, past state 8).
unsafe fn rows_gone(l: &mut List) -> bool {
    if l.rows == 0 {
        return true;
    }
    let state: i32 = rd(l.rows + TEXTWND_STATE);
    if state > GONE {
        let del: DeleteFn = std::mem::transmute::<usize, DeleteFn>(rd::<usize>(rd::<usize>(l.rows)));
        del(l.rows, 1);
        l.rows = 0;
        return true;
    }
    if state < 6 {
        func::<WndFn>(TEXTWND_CLOSE)(l.rows);
    }
    false
}

/// Puts `text` into a shown `CTextWnd` (`0x1402B0EF0`, only in state 5, as Re:CoM's own help line
/// call). True when set.
unsafe fn set_text(w: usize, text: &[u8]) -> bool {
    if w == 0 || rd::<i32>(w + TEXTWND_STATE) != SHOWN {
        return false;
    }
    func::<TextFn>(TEXTWND_TEXT)(w, text.as_ptr(), 0);
    true
}

/// The rows, the help line and the finger after a frame.
unsafe fn refresh(l: &mut List, c: &Ctx, inv: &crate::items::Inventory) {
    let rows = sh::rows_text(&l.menu, c, item_name);
    if rows != l.rows_text && set_text(l.rows, &rows) {
        let old = std::mem::replace(&mut l.rows_text, rows);
        l.retire(old);
    }
    let help = sh::help_text(&l.menu, c, l.note, inv);
    let tw: usize = rd(l.shop + f::TEXT);
    if help != l.help_text && tw != 0 && rd::<i32>(tw + TW_STATE) == 1 && set_text(rd(tw + TW_MAIN), &help) {
        let old = std::mem::replace(&mut l.help_text, help);
        l.retire(old);
    }
    let row = Some(l.menu.cursor());
    if row != l.finger_row {
        finger(l.shop, row);
        l.finger_row = row;
    }
}

/// Re:CoM's payment for `price` (the pack screen's state 4, `0x1403A7041`): the spent tally and
/// its trophy, the points, the Moogle Point window.
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
    set_points(shop, after);
}

/// Re:CoM's setter `0x14023A1D0` (it clamps) and the Moogle Point window.
unsafe fn set_points(shop: usize, after: i32) {
    func::<PointsSetFn>(POINTS_SET)(after);
    let w: usize = rd(shop + f::POINT_WINDOW);
    if w != 0 {
        func::<ShopIntFn>(POINT_WINDOW)(w, points());
    }
}

unsafe fn buy(l: &mut List, item: u8) {
    let have = points();
    match super::items::edit(|inv| sh::buy(item, have, inv)) {
        Some(Buy::Bought { price, points: after, to, .. }) => {
            sound(sh::sound::CONFIRM);
            sound(sh::sound::BUY_TOO);
            pay(l.shop, price, after);
            l.note = None;
            note(|| format!("bought {} for {price}: {have} -> {} Moogle Points, {to:?}", crate::items::name(item), points()));
        }
        r => {
            sound(sh::sound::ERROR);
            l.note = match r {
                Some(Buy::Poor { .. }) => Some(Note::Poor),
                Some(Buy::NoRoom { .. }) => Some(Note::NoRoom),
                _ => None,
            };
            note(|| format!("refused {} with {have} Moogle Points: {r:?}", crate::items::name(item)));
        }
    }
}

unsafe fn sell(l: &mut List, item: u8) {
    let have = points();
    match super::items::edit(|inv| sh::sell(item, have, inv)) {
        Some(Sell::Sold { price, points: after, from, left, .. }) => {
            sound(sh::sound::SALE);
            set_points(l.shop, after);
            l.note = None;
            note(|| format!("sold {} for {price}: {have} -> {} Moogle Points, from {from:?}, {left} left", crate::items::name(item), points()));
        }
        r => {
            sound(sh::sound::ERROR);
            note(|| format!("sale of {} refused: {r:?}", crate::items::name(item)));
        }
    }
}

/// One frame of the input state.
unsafe fn input(l: &mut List) {
    let shop = l.shop;
    let locked = rd::<i32>(shop + f::LOCK) > 0;
    let pad = if locked { Pad::default() } else { read_pad() };
    let (c, _) = ctx();
    let act = l.menu.step(&pad, &c);
    match act {
        Act::Nothing => {}
        Act::Moved => {
            sound(sh::sound::MOVE);
            l.note = None;
        }
        Act::Opened(_) => {
            sound(sh::sound::CONFIRM);
            l.note = None;
        }
        Act::Back => {
            sound(sh::sound::BACK);
            l.note = None;
        }
        Act::Refused => {
            sound(sh::sound::ERROR);
            l.note = Some(Note::NothingToSell);
        }
        Act::CardPacks => {
            sound(sh::sound::CONFIRM);
            l.next = sh::card_packs_mode(room(shop));
            wr::<i32>(shop + f::NEXT_MODE, l.next);
            wr::<i32>(shop + f::STATE, S_CLOSE);
            note(|| format!("Card Packs: on to Re:CoM's mode {}", l.next));
            return;
        }
        Act::Leave => {
            sound(sh::sound::BACK);
            wr::<i32>(shop + f::STATE, S_LEAVE);
            return;
        }
        Act::Buy(i) => buy(l, i),
        Act::Sell(i) => sell(l, i),
    }
    let (c, inv) = ctx();
    l.menu.settle(&c);
    refresh(l, &c, &inv);
}

/// Mode 3's update at KH1 (Re:CoM's state machine calls it through the shop's own mode-3 record,
/// which our init points here).
///
/// # Safety
/// Called by Re:CoM's state machine with its `CMoogleShopCore`.
pub unsafe extern "win64" fn list_update(shop: usize) {
    if rd::<i32>(shop + f::MODE) != sh::mode::MENU {
        return;
    }
    let mut g = lock();
    let Some(l) = g.as_mut().filter(|l| l.shop == shop) else {
        // Not seen: no list for this shop. Re:CoM's own update from its state 0 (its windows are
        // set up as its init sets them for its first choice).
        drop(g);
        wr::<usize>(shop + f::MODE3_UPDATE, base() + RECOM_MODE3_UPDATE);
        wr::<i32>(shop + f::STATE, S_OPEN);
        note(|| "mode 3 update without a list: Re:CoM's menu".into());
        return;
    };
    let p = |o: usize| -> usize { rd(shop + o) };
    let idle = || rd::<i32>(p(f::TASKS) + f::TASKS_LIVE) < 1;
    match rd::<i32>(shop + f::STATE) {
        S_OPEN => {
            if func::<ShopTestFn>(OPENED)(shop, 1) != 0 {
                let (c, _) = ctx();
                l.menu.settle(&c);
                let rows = sh::rows_text(&l.menu, &c, item_name);
                l.rows = open_rows(shop, &rows);
                let old = std::mem::replace(&mut l.rows_text, rows);
                l.retire(old);
                wr::<i32>(shop + f::STATE, S_SHOWN);
                if l.rows == 0 {
                    note(|| "the rows window couldn't be made: leaving the shop".into());
                    wr::<i32>(shop + f::STATE, S_LEAVE);
                }
            }
        }
        S_SHOWN => {
            if rd::<i32>(l.rows + TEXTWND_STATE) >= SHOWN {
                wr::<i32>(shop + f::STATE, S_INPUT);
                let (c, inv) = ctx();
                l.finger_row = None;
                refresh(l, &c, &inv);
            }
        }
        S_INPUT => input(l),
        S_CLOSE => {
            finger(shop, None);
            func::<ShopIntFn>(CLOSE)(shop, 0);
            rows_gone(l);
            wr::<i32>(shop + f::STATE, S_SWITCH);
        }
        S_SWITCH => {
            if func::<ShopClosedFn>(CLOSED)(shop) != 0 && idle() && rows_gone(l) {
                let next: i32 = rd(shop + f::NEXT_MODE);
                wr::<i32>(shop + f::MODE, next);
                wr::<usize>(shop + f::PENDING, shop + f::RECORDS + next as usize * f::RECORD);
                l.finger_row = None;
            }
        }
        S_LEAVE => {
            finger(shop, None);
            func::<ShopIntFn>(CLOSE)(shop, 1);
            func::<ShopIntFn>(BG_CLOSE)(p(f::BG), -1);
            func::<FadeFn>(LEAVE_FADE)(-1, -1);
            rows_gone(l);
            wr::<i32>(shop + f::STATE, S_LEFT);
        }
        S_LEFT => {
            if func::<ShopClosedFn>(CLOSED)(shop) != 0 && idle() && func::<ShopClosedFn>(BG_CLOSED)(p(f::BG)) != 0 && rows_gone(l) {
                wr::<i32>(shop + f::MODE, sh::mode::LEAVE);
                wr::<usize>(shop + f::PENDING, shop + f::RECORDS + sh::mode::LEAVE as usize * f::RECORD);
                // The shop ends: its texts were Re:CoM's windows' last; they go with the list.
                *g = None;
                note(|| "shop left from the KH1 list".into());
            }
        }
        _ => {}
    }
}

pub fn start_line() -> String {
    "shop: KH1's Moogle shop list (M8c): at Combat = KH1 in Sora's story Re:CoM's shop mode 3 is a list Buy / Sell / Card Packs (Re:CoM's text window, finger cursor, sounds and payment); stock by Re:CoM's floor tier (1F Potion, Ether; 2F+ Hi-Potion; 7F+ Elixir), Potion 10 / Hi-Potion 30 / Ether 40 / Elixir 200 Moogle Points, sell = buy / 5; Card Packs opens Re:CoM's pack screen".into()
}
