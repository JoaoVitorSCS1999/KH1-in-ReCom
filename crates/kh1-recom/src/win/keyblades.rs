//! The camp menu's 8th entry, "KH1" (M7 stage 7.13; "Keyblades" before it, decision 25), its
//! submenu (Items, Customize) and their pages, run by Re:CoM's own camp code. Pure rules:
//! `crate::keyblades`, `crate::customize`. Notes: `docs/recom-keyblade-equip.md` (7.13 section).
//!
//! - Entry: a `C2DSeqCtrl` + `C2DFnt` made with the calls Re:CoM's entry build `0x1400E3DE0`
//!   makes for its own 7, at index 7. Four wrappers make the camp treat index 7 as one more entry:
//!   the camp update `0x1400D9C80` (states 7 / 0xD / 0xF / 0x11), the entry build (create, delete,
//!   animations 4 / 5 / 6), the cursor set `0x1400DEA10` (highlight, help line, 8-entry wrap) and
//!   the teardown `0x1400D7C20` (delete ours).
//! - Submenu (stage 7.13): run inside Re:CoM's Config states 0x4A..0x4C by the Config input hook
//!   (`menu::cfg_input` → [`page`]) when our entry was the one confirmed (`camp+0xBC`). Re:CoM's
//!   own 7 entry buttons, relabelled with our rows as Re:CoM's Deck submenu relabels them (level
//!   1), with the Deck submenu's input, cursor set, sounds, breadcrumb (header item 1) and fades
//!   (update states 0x10 / 0x1D / 0x22 / 0x23 / 0x34 / 0x35). Leaving it, Re:CoM's state 0x4C
//!   relabels its 7 entries itself (level 1 → 0).
//! - Pages: Items (M8 stage 8a.2; Sora's list starts with his Keyblade slot, which shows the
//!   Keyblade list on the same page) and Customize follow the entry-4 page (`0x1400EB7E0` / draw
//!   `0x1400E6DF0`) with Re:CoM's classes: two panels, `CButton` rows, a `CButtonCursor`, a
//!   `CScrollBar` (the Keyblade list), label / value rows and the help line. Their state is ours ([`SUB`]); they never
//!   write camp fields Re:CoM's pages use.
//!
//! Nothing here runs in field or battle frames: the camp exists only while the pause menu is
//! open, and without our entry (the camp that opens straight onto the map) every wrapper is one
//! load and the original.
//! - Riku's story (owner, 2026-11-03): the same entry, wrappers and submenu, with only Combat and
//!   Movement; Re:CoM's own per-story tables (labels, help ids, titles) keep its 7 entries as
//!   they are (`kb::Story`, `top::RECOM_HELP`).
//!
//! Every offset and call below is Checked (decomp + disasm) unless tagged otherwise.

use super::movement::{func, orig, rd, wr};
use super::{base, log, va};
use crate::command_menu as cm;
use crate::customize;
use crate::items_page as ip;
use crate::abilities::page as ap;
use crate::keyblades::{self as kb, list, sub, text, top};
use crate::sites::data;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_UPDATE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_BUILD: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_CURSOR: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_TEARDOWN: AtomicUsize = AtomicUsize::new(0);

/// The camp our entry belongs to, and our entry's `C2DSeqCtrl`.
static ENTRY_CAMP: AtomicUsize = AtomicUsize::new(0);
/// The asset step's item row (`crate::camp_rows`): our list table, Re:CoM's 7 entries of
/// `0x1407C11F0` (read from the image) and the new group of the loaded language's copy (set before
/// each set-up from the loaded set's group count); and every copy's group from
/// `kh1-camp-rows.json`. Set once at install; Re:CoM's row set-up keeps the table's pointer, so it
/// lives as long as the DLL (`AtomicU16` has `u16`'s layout).
static ITEM_TABLE: OnceLock<[AtomicU16; crate::camp_rows::RECOM_ENTRIES + 1]> = OnceLock::new();
static ITEM_ROWS: OnceLock<Vec<u16>> = OnceLock::new();
static ITEM_SET_LOGGED: AtomicBool = AtomicBool::new(false);
static ENTRY_BTN: AtomicUsize = AtomicUsize::new(0);
static SPARE_LOGGED: AtomicBool = AtomicBool::new(false);
static ENTRY_LOGGED: AtomicBool = AtomicBool::new(false);
static NO_NUMBERS_LOGGED: AtomicBool = AtomicBool::new(false);
static ODD_STATE_LOGGED: AtomicBool = AtomicBool::new(false);
/// The submenu (and the page open in it) of the camp our entry belongs to.
static SUB: Mutex<Option<Sub>> = Mutex::new(None);

/// `CCamp` fields (Checked, decomp `0x1400D9C80` and the functions named).
mod camp {
    /// Frame time (int; the pages animate their cursor by `dt × 0.02`, `0x1400E6DF0` mode 3).
    pub const DT: usize = 0x14;
    /// Update state.
    pub const STATE: usize = 0x6C;
    /// Layout (`camp.lad`) and sprite set name (`"camp:00"`).
    pub const LAYOUT: usize = 0x78;
    pub const SET: usize = 0x80;
    /// Fresh-press flag left by the camp's direction read `0x140111A70`.
    pub const FRESH: usize = 0x94;
    /// Text colour of list rows and the header title.
    pub const TEXT_COLOUR: usize = 0x98;
    /// Non-zero: no text offsets (`0x1400E6DF0` picks 0.0 instead of 1.0 / 2.0).
    pub const NO_SCALE: usize = 0x9C;
    /// Menu level: 0 top list, 1 Deck submenu, 2 in a page.
    pub const LEVEL: usize = 0xB4;
    pub const CURSOR: usize = 0xB8;
    /// The top index saved on confirm.
    pub const SAVED: usize = 0xBC;
    /// Header item 0's mode (the update's trailer runs `0x1400E4220(camp, [+0xC4], 0)`).
    pub const HEADER_MODE: usize = 0xC4;
    pub const MAIN_PANEL: usize = 0x130;
    pub const PANEL_140: usize = 0x140;
    /// Entry buttons (`C2DSeqCtrl`, 7) and their labels (`C2DFnt`, 7).
    pub const BUTTONS: usize = 0x1B8;
    pub const LABELS: usize = 0x1F0;
    /// Header items 0 and 1 (`C2DSeqCtrl`, the breadcrumb) and their titles (`C2DFnt`), 8 apart.
    pub const HEADER: usize = 0x228;
    pub const HEADER_TITLE: usize = 0x238;
    pub const HELP: usize = 0x9D0;
    /// Non-zero: the camp opens straight into the map page and never shows the list.
    pub const DIRECT_MAP: usize = 0x20A4;
    /// `0x1401AAAE0()` at ctor: Riku's story (Re:CoM's index for its per-story tables).
    pub const RIKU: usize = 0x21BC;
    pub const COUNT: usize = 0x21C4;
    /// Copied each update from a frame counter (`% 7000`); the list rows' marker sprites get it.
    pub const BLINK: usize = 0x21D8;
}

// Re:CoM functions (RVAs; first bytes checked in `sites`).
const SOUND: u32 = 0x002A_A360;
const NEW: u32 = 0x004A_CE54;
const CBUTTON_CTOR: u32 = 0x0011_2A90;
const CBUTTON_SETUP: u32 = 0x0011_2F10;
const LAYOUT: u32 = 0x0024_6100;
const TEXT_SET: u32 = 0x0025_9000;
const SPRITE_POS: u32 = 0x0024_85B0;
const SEQ_ENDED: u32 = 0x0024_80C0;
const PAD_REPEAT: u32 = 0x0029_B2E0;
const PAD_FRESH: u32 = 0x0029_B2F0;
const PAD_CANCEL: u32 = 0x0029_B410;
const PAD_CONFIRM: u32 = 0x0029_B490;
const GUIDE: u32 = 0x002B_57A0;
const CONFIRM_FLASH: u32 = 0x000D_3A70;
const PRIORITY: u32 = 0x0024_7AA0;
const VISIBLE: u32 = 0x0024_7DC0;
const HIGHLIGHT_BAR: u32 = 0x000E_07D0;
const GLIDE_TARGET: u32 = 0x000E_3D30;
const MSG: u32 = 0x0024_0160;
const HEADER: u32 = 0x000E_4220;
const FNT_CTOR: u32 = 0x0024_3F10;
const SEQ_CTOR: u32 = 0x0024_7260;
const LAYOUT_POS: u32 = 0x0024_8660;
const CAMP_PANEL: u32 = 0x000E_6700;
const SLOT_VISIBLE: u32 = 0x0024_7E50;
const SIDE_PANEL: u32 = 0x000E_46D0;
const GLIDE_STEP: u32 = 0x000D_E100;
const CAMP_DIR: u32 = 0x0011_1A70;
const CAMP_CANCEL: u32 = 0x0011_1B90;
const CAMP_DECIDE: u32 = 0x0011_1C30;
const CURSOR_CTOR: u32 = 0x000D_25F0;
const CURSOR_GLIDE: u32 = 0x000D_E010;
const CURSOR_HIDE: u32 = 0x000E_5CF0;
const BAR_CTOR: u32 = 0x0011_2AB0;
const BAR_SETUP: u32 = 0x0011_30C0;
const BAR_UPDATE: u32 = 0x0011_2CB0;
const TEXT_COPY: u32 = 0x0025_8DE0;
const EVER_OBTAINED: u32 = 0x0015_B3F0;
const CARD_NAME: u32 = 0x0016_4CA0;

// Re:CoM data (Steam addresses).
/// The message tables (`CCtdList` owner) passed to `0x140240160`.
const MSG_TABLES: u64 = 0x1_4087_CD78;
/// The empty text the cursor set clears the help line with (`mov rdx,[0x1407C12E8]`).
const HELP_CLEAR: u64 = 0x1_407C_12E8;
/// The help message id the entry-4 page restores when it closes (`0x1400E6DF0` mode 4).
const CLOSE_HELP_ID: u64 = 0x1_407C_106C;
/// Sora's card collection (`save + 0x1A74`), read by `0x14015B3F0`.
const COLLECTION: u64 = 0x1_4087_A7E8;
const STATUS_SEQ: u64 = 0x1_407C_11D0;
const LIST_SEQ: u64 = 0x1_407C_11F0;
const BAR_SEQ: u64 = 0x1_407C_1228;

// Values Re:CoM's own code uses (Checked, exe / disasm).
/// Entry button position: (86.0, 88.0 + 26.0 × index) (`0x140509430`, `0x140509270`).
const ENTRY_X: f32 = 86.0;
const ENTRY_Y0: f32 = 88.0;
const ENTRY_STEP: f32 = 26.0;
/// Label colours: normal, highlighted (`0x1400E3DE0`, `0x1400DEA10`).
const LABEL_NORMAL: u32 = 0x8040_4040;
const LABEL_HIGHLIGHT: u32 = 0x80C0_C0C0;
/// The status rows' value colour (`0x1400E6DF0`, rows 0..4).
const VALUE_COLOUR: u32 = 0x8004_6060;
/// The list cursor's second sprite: + (284, 168) from the row (`0x140509530`, `0x140509520`).
const CURSOR_DX: f32 = 284.0;
const CURSOR_DY: f32 = 168.0;
/// The list rows' marker sprite (`0x1400E6DF0` mode 9: Re:CoM's "seen" 0x1D / "new" 0x1E mark,
/// at (170, −13)). We mark the equipped Keyblade's row with 0x1D. Assumption: it reads as a mark
/// on this list (to check in game; 0x1E is the other choice).
const EQUIPPED_MARK: u32 = 0x1D;
const MARK_X: f32 = 170.0;
const MARK_Y: f32 = -13.0;

type VoidFn = unsafe extern "win64" fn(*mut u8);
type BuildFn = unsafe extern "win64" fn(*mut u8, i32, u64) -> u64;
type CursorSetFn = unsafe extern "win64" fn(*mut u8, i32, i32) -> u64;
type CampIntFn = unsafe extern "win64" fn(*mut u8, i32, i32) -> u64;
type HeaderFn = unsafe extern "win64" fn(*mut u8, u32, u32) -> u64;
type SoundFn = unsafe extern "win64" fn(i32, i32) -> u64;
type NewFn = unsafe extern "win64" fn(usize) -> *mut u8;
type CtorFn = unsafe extern "win64" fn(*mut u8) -> *mut u8;
type DeleteFn = unsafe extern "win64" fn(*mut u8, u32) -> *mut u8;
/// `C2DSeqCtrl` vtable `+0x8`: (ctrl, set, sequence, 0.0, 0).
type SeqInitFn = unsafe extern "win64" fn(*mut u8, *const u8, u32, f32, u32) -> u64;
/// `+0x20`: (ctrl, slot, set, sequence, 0.0, 0).
type SeqAnimFn = unsafe extern "win64" fn(*mut u8, u32, *const u8, u32, f32, u32) -> u64;
/// `+0x60` (`0x1402476F0`): add a text item with an offset (x, y) and a third float.
type AttachFn = unsafe extern "win64" fn(*mut u8, *const u8, *mut u8, f32, f32, f32) -> u64;
/// `+0x68` (`0x1402481A0`): replace slot `n`'s item with a new text item.
type ReplaceFn = unsafe extern "win64" fn(*mut u8, u32, *const u8, *mut u8) -> u64;
/// `+0x40` (`0x140247A50`): add a sequence at an offset; returns the new sequence object.
type MarkFn = unsafe extern "win64" fn(*mut u8, *const u8, u32, f32, f32, f32, u32) -> *mut u8;
/// Sequence object vtable `+0x30`, as `0x1400E6DF0` calls it with `camp+0x21D8`.
type ObjIntFn = unsafe extern "win64" fn(*mut u8, u32) -> u64;
type FntCtorFn = unsafe extern "win64" fn(*mut u8, i32, i32, *const u32, i32, *const u8, i32) -> *mut u8;
#[allow(clippy::type_complexity)]
type SetupFn = unsafe extern "win64" fn(*mut u8, *const u8, u32, *const u16, *const f32, *mut u8, *const u8, i32, f32, f32, i32, *const u32, i32);
type LayoutFn = unsafe extern "win64" fn(*const u8, u32, u64, *mut f32, i32) -> u64;
type PosFn = unsafe extern "win64" fn(*mut u8, *const f32);
type SprIntFn = unsafe extern "win64" fn(*mut u8, u32);
type SlotVisFn = unsafe extern "win64" fn(*mut u8, u32, u32);
type EndedFn = unsafe extern "win64" fn(*mut u8, u32) -> u64;
type PadFn = unsafe extern "win64" fn(u32, u32) -> u32;
type PadBtnFn = unsafe extern "win64" fn(u32) -> u32;
type GuideFn = unsafe extern "win64" fn(u32, u32);
type MsgFn = unsafe extern "win64" fn(usize, u32) -> *const u8;
type CampDirFn = unsafe extern "win64" fn(*mut u8, u32, u32) -> u32;
type CampBtnFn = unsafe extern "win64" fn(*mut u8, u32) -> u32;
type CursorCtorFn = unsafe extern "win64" fn(*mut u8, u32, *mut u8, u32) -> *mut u8;
type GlideFn = unsafe extern "win64" fn(*mut u8, f32);
type BarSetupFn = unsafe extern "win64" fn(*mut u8, *const u8, u32, *const u16, *mut f32, f32, *mut u8);
type BarUpdateFn = unsafe extern "win64" fn(*mut u8, f32, f32);
type TextFn = unsafe extern "win64" fn(*mut u8, *const u8);
type EverFn = unsafe extern "win64" fn(u32) -> u32;
type NameFn = unsafe extern "win64" fn(*const u8) -> *const u8;

#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
struct Vec4([f32; 4]);

unsafe fn at<T: Copy>(p: *mut u8, off: usize) -> T {
    rd::<T>(p as usize + off)
}

unsafe fn put<T: Copy>(p: *mut u8, off: usize, v: T) {
    wr::<T>(p as usize + off, v)
}

unsafe fn vfn<T: Copy>(obj: *mut u8, slot: usize) -> T {
    let vt: usize = rd(obj as usize);
    let f: usize = rd(vt + slot);
    std::mem::transmute_copy(&f)
}

unsafe fn set_of(camp: *mut u8) -> *const u8 {
    at(camp, camp::SET)
}

/// Reads `kh1-camp-rows.json` (the asset step's item row, `crate::camp_rows`) and builds our list
/// table. Without it the item rows keep Re:CoM's green row 0x131.
pub fn load_item_row(file: Option<&std::path::Path>) {
    let Some(path) = file else {
        log("camp rows: no kh1-camp-rows.json: item rows keep Re:CoM's green row 0x131 (Mickey symbol)");
        return;
    };
    let rows = match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| crate::camp_rows::parse(&t)) {
        Ok(r) => r,
        Err(e) => {
            log(&format!("camp rows: {}: {e}: item rows keep Re:CoM's green row 0x131", path.display()));
            return;
        }
    };
    // SAFETY: Re:CoM's camp list table in the image (its bytes checked at start-up, `sites`).
    let recom: [u16; crate::camp_rows::RECOM_ENTRIES] = std::array::from_fn(|i| unsafe { rd::<u16>(va(base(), LIST_SEQ) + 2 * i) });
    let t = crate::camp_rows::list_table(recom, rows[0]);
    let _ = ITEM_TABLE.set(t.map(AtomicU16::new));
    log(&format!(
        "camp rows: {} item rows = camp:00 groups {rows:x?} (base file, language copies; variant {}, our table {t:x?}); used when the loaded camp:00 ends with one",
        path.display(),
        kb::row_look::ITEM
    ));
    let _ = ITEM_ROWS.set(rows);
}

/// The item row of the loaded camp set (`set`, Re:CoM's NUL-terminated name): its last group when
/// that is one of the manifest's, its group count read from Re:CoM's 2D registry. Logged once
/// when none (and once with the row found).
unsafe fn item_row_loaded(set: *const u8, rows: &[u16]) -> Option<u16> {
    if set.is_null() {
        return None;
    }
    let mut name = Vec::with_capacity(16);
    for k in 0..32 {
        let c = *set.add(k);
        name.push(c);
        if c == 0 {
            break;
        }
    }
    if name.last() != Some(&0) {
        return None;
    }
    let n = super::mp_bar::set_groups(&name);
    let row = crate::camp_rows::loaded_row(n, rows);
    if !ITEM_SET_LOGGED.swap(true, Ordering::Relaxed) {
        let set = String::from_utf8_lossy(&name[..name.len() - 1]);
        let groups = n.map_or("no".to_string(), |n| n.to_string());
        log(&match row {
            Some(r) => format!("camp rows: {set} has {groups} groups: item row = group {r:#x}"),
            None => format!(
                "camp rows: {set} has {groups} groups, none of {rows:?} + 1 (Panacea serves Re:CoM's own FORM/0100/FO0100.SPR, or this language's copy was not patched): item rows keep Re:CoM's green row 0x131"
            ),
        });
    }
    row
}

/// `vt+0x20(obj, 0, set, sequence, 0.0, 0)`, as Re:CoM gives its menu sprites a sequence.
unsafe fn anim(obj: *mut u8, set: *const u8, seq: u32) {
    if !obj.is_null() {
        vfn::<SeqAnimFn>(obj, 0x20)(obj, 0, set, seq, 0.0, 0);
    }
}

unsafe fn delete(obj: *mut u8) {
    if !obj.is_null() {
        vfn::<DeleteFn>(obj, 0)(obj, 1);
    }
}

unsafe fn sound(id: i32) {
    func::<SoundFn>(SOUND)(id, -1);
}

unsafe fn ended(obj: *mut u8) -> bool {
    !obj.is_null() && func::<EndedFn>(SEQ_ENDED)(obj, 0) as u32 != 0
}

unsafe fn message(id: u32) -> *const u8 {
    let tables: usize = rd(va(base(), MSG_TABLES));
    func::<MsgFn>(MSG)(tables, id)
}

/// Re:CoM's fonts (`0x14087CFC0`, 0x158 bytes each, `0x14087CFB8` of them; a `C2DFnt` draws with
/// font `+0x40` bit 12, `0x14025AACE`) and the flag its set-up gives the "sysfont" (`+0x150` = 1,
/// `0x14025C400`): only that font has the button sheet the glyph picker draws escapes from
/// (`0x14025A0B4`: glyphs 0x55C..0x610). Checked (decomp + exe).
const FONTS: u64 = 0x1_4087_CFC0;
const FONT_COUNT: u64 = 0x1_4087_CFB8;
const FONT_SIZE: usize = 0x158;
const FONT_BUTTONS: usize = 0x150;
const FNT_FLAGS: usize = 0x40;
/// The list and status rows' font: `CBUTTON_SETUP`'s flags 0x40 (bit 12 clear).
const ROW_FONT: u32 = 0;

/// Re:CoM's font `idx` draws button escapes (its button sheet is loaded).
fn button_glyphs(idx: u32) -> bool {
    // SAFETY: static data of the image (Re:CoM's font table).
    let ok = unsafe {
        let n: u32 = rd(va(base(), FONT_COUNT));
        idx < n.min(2) && rd::<u32>(va(base(), FONTS) + idx as usize * FONT_SIZE + FONT_BUTTONS) == 1
    };
    if !GLYPHS_LOGGED.swap(true, Ordering::Relaxed) {
        log(&format!(
            "customize: slot labels {} (Re:CoM's font {idx} {} a button sheet)",
            if ok { "as Re:CoM's button glyphs" } else { "as button names" },
            if ok { "has" } else { "has no" }
        ));
    }
    ok
}
static GLYPHS_LOGGED: AtomicBool = AtomicBool::new(false);

/// The camp's help line draws button escapes (its `C2DFnt`'s font has the button sheet).
fn help_glyphs(camp: *mut u8) -> bool {
    // SAFETY: the camp's help item, read as `set_help` writes it.
    unsafe {
        let help: *mut u8 = at(camp, camp::HELP);
        !help.is_null() && button_glyphs((at::<u32>(help, FNT_FLAGS) >> 12) & 1)
    }
}

unsafe fn set_help(camp: *mut u8, s: *const u8) {
    let help: *mut u8 = at(camp, camp::HELP);
    if !help.is_null() && !s.is_null() {
        func::<TextFn>(TEXT_SET)(help, s);
    }
}

/// Our entry's button when `camp` is the camp it was made for.
fn entry_of(camp: *mut u8) -> *mut u8 {
    if camp.is_null() || ENTRY_CAMP.load(Ordering::Relaxed) != camp as usize {
        return std::ptr::null_mut();
    }
    ENTRY_BTN.load(Ordering::Relaxed) as *mut u8
}

/// True when the Config input hook must run our submenu: our entry exists for this camp and it was
/// the top entry confirmed (`camp+0xBC`, which Re:CoM's state 0xF sets on every confirm and our
/// confirm sets to 7). The cursor `camp+0xB8` can't tell: inside the submenu it is the submenu's
/// row, as in Re:CoM's Deck submenu.
pub fn page_active(camp: *mut u8) -> bool {
    // SAFETY: a live camp (Re:CoM just passed it to its own Config input function).
    !entry_of(camp).is_null() && unsafe { at::<i32>(camp, camp::SAVED) } == top::OURS
}

/// Whether the camp should show our entry now: the normal top list, whatever the Combat setting
/// (owner, 2026-11-03: "always be there"), in Sora's story only (owner, 2026-11-03, on 0.93.4: Riku's
/// story "Hide it": none of its settings acts there; 0.93.4's per-story help ids stay). The camp that opens straight into the map page never
/// builds the list (state 3 skips the entry build when `camp+0x20A4` is set; Checked (decomp)).
unsafe fn wanted(camp: *mut u8) -> bool {
    // Owner (2026-11-03, on 0.93.4): "Hide it" in Riku's story (none of the nine settings acts there).
    at::<i32>(camp, camp::DIRECT_MAP) == 0 && story(camp) != kb::Story::Riku
}

/// The camp's story, as Re:CoM's camp code reads it for its own tables.
unsafe fn story(camp: *mut u8) -> kb::Story {
    kb::Story::from_camp(at::<i32>(camp, camp::RIKU))
}

/// Whether the submenu shows KH1's pages (Items, Customize, Abilities) besides the settings
/// (`sub::kh1_pages`): Combat = KH1, Keyblade numbers to list, Sora's story. Read when the
/// submenu opens and again when a settings page closes (Combat may have changed there).
unsafe fn kh1_pages(camp: *mut u8) -> bool {
    let story = story(camp);
    let combat = super::settings().combat == crate::settings::Mode::Kh1;
    if story == kb::Story::Riku || !combat {
        return false;
    }
    let listed = !super::stats::keyblade_cards().is_empty();
    if !listed && !NO_NUMBERS_LOGGED.swap(true, Ordering::Relaxed) {
        log("keyblades: kh1-stats.json has no Keyblades (format 3, kh1-recom-build); the KH1 entry shows only Combat and Movement");
    }
    kb::sub::kh1_pages(story, combat, listed)
}

/// Logs, once per session, whether `save + 0x246B ..= 0x2817` is zero apart from our bytes (the
/// check that the spare range really is unused, `recom-cards-save.md` § 4).
unsafe fn spare_check() {
    if SPARE_LOGGED.swap(true, Ordering::Relaxed) {
        return;
    }
    let save: usize = rd(va(base(), kb::SAVE_PTR));
    if save == 0 {
        log("keyblades: save block not there at the first camp open; spare range not checked");
        return;
    }
    let range = std::slice::from_raw_parts((save + kb::SPARE_FIRST) as *const u8, kb::SPARE_LAST - kb::SPARE_FIRST + 1);
    let ours: u8 = rd(save + kb::EQUIP_OFFSET);
    let nz = kb::spare_range_nonzero(range);
    if nz.is_empty() {
        log(&format!(
            "keyblades: save+{:#x}..={:#x} is all zero apart from our bytes (save+{:#x} = {ours:#x}; MP block save+{:#x}..={:#x} v{}; items save+{:#x}..={:#x} v{})",
            kb::SPARE_FIRST,
            kb::SPARE_LAST,
            kb::EQUIP_OFFSET,
            crate::mp::BLOCK_FIRST,
            crate::mp::BLOCK_LAST,
            rd::<u8>(save + crate::mp::BLOCK_FIRST),
            crate::items::EXT_FIRST,
            crate::items::EXT_LAST,
            rd::<u8>(save + crate::items::EXT_FIRST)
        ));
    } else {
        let first: Vec<String> = nz.iter().take(16).map(|o| format!("+{o:#x}={:#x}", rd::<u8>(save + o))).collect();
        log(&format!(
            "keyblades: WARNING: save+{:#x}..={:#x} has {} non-zero bytes besides ours (save+{:#x} = {ours:#x}): {}",
            kb::SPARE_FIRST,
            kb::SPARE_LAST,
            nz.len(),
            kb::EQUIP_OFFSET,
            first.join(" ")
        ));
    }
}

/// Our entry, made as `0x1400E3DE0` makes Re:CoM's (mode 0, first time) at index 7.
unsafe fn create_entry(camp: *mut u8) {
    let set = set_of(camp);
    let btn = func::<NewFn>(NEW)(0x170);
    if btn.is_null() {
        return;
    }
    func::<CtorFn>(SEQ_CTOR)(btn);
    put::<u8>(btn, 0x16E, 0xC);
    put::<u8>(btn, 0x138, 1);
    vfn::<SeqInitFn>(btn, 0x8)(btn, set, 4, 0.0, 0);
    let pos = Vec4([ENTRY_X, ENTRY_Y0 + ENTRY_STEP * top::OURS as f32, 0.0, 1.0]);
    func::<PosFn>(LAYOUT_POS)(btn, pos.0.as_ptr());
    let mut fnt = func::<NewFn>(NEW)(0xB0);
    if !fnt.is_null() {
        let colour = LABEL_NORMAL;
        fnt = func::<FntCtorFn>(FNT_CTOR)(fnt, 0, 0, &colour, 0xF, text::ENTRY_LABEL.as_ptr(), 0x40);
    }
    vfn::<AttachFn>(btn, 0x60)(btn, set, fnt, 0.0, 6.0, 0.0);
    ENTRY_CAMP.store(camp as usize, Ordering::Relaxed);
    ENTRY_BTN.store(btn as usize, Ordering::Relaxed);
    ENTRY_FNT.store(fnt as usize, Ordering::Relaxed);
    if !ENTRY_LOGGED.swap(true, Ordering::Relaxed) {
        log(&format!("kh1 entry: camp entry 8 \"KH1\" added (Keyblades equipped: card {:#x})", super::stats::equipped_card()));
    }
}

/// Our entry's label (the button's text item), for the highlight colour.
static ENTRY_FNT: AtomicUsize = AtomicUsize::new(0);

/// Deletes our entry. Re:CoM's teardown deletes only its buttons; their text items go with the
/// buttons' sequences (Re:CoM's relabel path clears the old sequence's item pointer before
/// releasing it, `0x140246E90`). Assumption (inferred, not traced): the same holds for ours.
unsafe fn delete_entry() {
    let btn = ENTRY_BTN.swap(0, Ordering::Relaxed) as *mut u8;
    ENTRY_FNT.store(0, Ordering::Relaxed);
    ENTRY_CAMP.store(0, Ordering::Relaxed);
    delete(btn);
}

/// Hook on `0x1400E3DE0(camp, mode, level)` (the entry build): after Re:CoM's own work, ours.
pub unsafe extern "win64" fn entry_build(camp: *mut u8, mode: i32, level: u64) -> u64 {
    let o: BuildFn = orig(&ORIG_BUILD);
    let r = o(camp, mode, level);
    if camp.is_null() {
        return r;
    }
    spare_check();
    if at::<i32>(camp, camp::LEVEL) != 0 {
        // Deck submenu: ours stays as Re:CoM leaves its unused 7th button (faded by the confirm).
        return r;
    }
    let ours = entry_of(camp);
    match mode {
        0 => {
            let want = wanted(camp);
            if want && ours.is_null() {
                if ENTRY_CAMP.load(Ordering::Relaxed) != 0 {
                    // A camp we never saw torn down: forget its entry (not ours to delete).
                    ENTRY_CAMP.store(0, Ordering::Relaxed);
                    ENTRY_BTN.store(0, Ordering::Relaxed);
                }
                create_entry(camp);
            } else if !want && !ours.is_null() {
                delete_entry();
                log("keyblades: camp entry removed (the map-only camp)");
            } else if !ours.is_null() {
                anim(ours, set_of(camp), 4);
            }
        }
        1 if !ours.is_null() => anim(ours, set_of(camp), 5),
        2 if !ours.is_null() => anim(ours, set_of(camp), 6),
        _ => {}
    }
    r
}

/// The button and label of top entry `i` (ours at 7).
unsafe fn entry_parts(camp: *mut u8, i: i32) -> (*mut u8, *mut u8) {
    if i == top::OURS {
        return (ENTRY_BTN.load(Ordering::Relaxed) as *mut u8, ENTRY_FNT.load(Ordering::Relaxed) as *mut u8);
    }
    if !(0..top::RECOM_ENTRIES).contains(&i) {
        return (std::ptr::null_mut(), std::ptr::null_mut());
    }
    (at(camp, camp::BUTTONS + 8 * i as usize), at(camp, camp::LABELS + 8 * i as usize))
}

/// What `0x1400DEA10` does, for a move from or to our entry: the old entry back to animation 5,
/// label 0x80404040, priority 0xC, help cleared; the new one (when `show`) animation 3, label
/// 0x80C0C0C0, priority 0xD, highlight bar and glove at its row, the cursor index and its help.
unsafe fn cursor_set_ours(camp: *mut u8, old: i32, new: i32, show: bool) -> u64 {
    let set = set_of(camp);
    let (b, l) = entry_parts(camp, old);
    if !b.is_null() {
        anim(b, set, 5);
        if !l.is_null() {
            put::<u32>(l, 0x34, LABEL_NORMAL);
        }
        func::<SprIntFn>(PRIORITY)(b, 0xC);
    }
    set_help(camp, rd::<*const u8>(va(base(), HELP_CLEAR)));
    if show {
        let (b, l) = entry_parts(camp, new);
        if !b.is_null() {
            anim(b, set, 3);
            if !l.is_null() {
                put::<u32>(l, 0x34, LABEL_HIGHLIGHT);
            }
            func::<SprIntFn>(PRIORITY)(b, 0xD);
        }
        func::<CampIntFn>(HIGHLIGHT_BAR)(camp, new, 0);
        func::<CampIntFn>(GLIDE_TARGET)(camp, new, 0);
        put::<i32>(camp, camp::CURSOR, new);
        if new == top::OURS {
            set_help(camp, text::entry_help(story(camp), kh1_pages(camp)).as_ptr());
        } else if let Some(id) = top::recom_help(story(camp), new) {
            // Re:CoM's own help id for its entry in this story (Riku's entries 0 and 5 differ).
            set_help(camp, message(id));
        }
    }
    1
}

/// Hook on `0x1400DEA10(camp, index, show)` (the camp's cursor set): with our entry, 8 entries.
pub unsafe extern "win64" fn cursor_set(camp: *mut u8, idx: i32, show: i32) -> u64 {
    let o: CursorSetFn = orig(&ORIG_CURSOR);
    if entry_of(camp).is_null() || at::<i32>(camp, camp::LEVEL) != 0 {
        return o(camp, idx, show);
    }
    let old = at::<i32>(camp, camp::CURSOR);
    let new = top::wrap(idx);
    if old != top::OURS && new != top::OURS {
        return o(camp, new, show);
    }
    cursor_set_ours(camp, old, new, show != 0)
}

/// The top list's pad for this frame, read as state 0xF reads it (all pure reads).
unsafe fn top_pad() -> top::Pad {
    let rep = func::<PadFn>(PAD_REPEAT);
    let fresh = func::<PadFn>(PAD_FRESH);
    top::Pad {
        start: fresh(0x800, 0) != 0,
        cancel: func::<PadBtnFn>(PAD_CANCEL)(0) & 0xFF != 0,
        confirm: func::<PadBtnFn>(PAD_CONFIRM)(0) & 0xFF != 0,
        up: rep(0x10_1000, 0) != 0,
        up_fresh: fresh(0x10_1000, 0) != 0,
        down: rep(0x20_4000, 0) != 0,
        down_fresh: fresh(0x20_4000, 0) != 0,
    }
}

/// Header item `idx` (0 the entry, 1 the submenu row: the breadcrumb) with our title:
/// `0x1400E4220` mode 1 (header present), with our text where Re:CoM puts `0x1400D3890`'s ("" for
/// index 7; the Deck's row titles for item 1).
unsafe fn header_title(camp: *mut u8, idx: usize, title: &[u8]) {
    let set = set_of(camp);
    let hdr: *mut u8 = at(camp, camp::HEADER + 8 * idx);
    let title_at = camp::HEADER_TITLE + 8 * idx;
    if hdr.is_null() {
        // Re:CoM makes both items at camp open (state 3); not seen missing. Its own call, then our
        // text on the new item. Assumption: the item reads its text pointer when drawn.
        func::<HeaderFn>(HEADER)(camp, 1, idx as u32);
        let t: *mut u8 = at(camp, title_at);
        if !t.is_null() {
            func::<TextFn>(TEXT_SET)(t, title.as_ptr());
        }
        log("kh1 entry: header was missing at confirm; made by Re:CoM, title set afterwards");
        return;
    }
    anim(hdr, set, 7);
    delete(at(camp, title_at));
    put::<usize>(camp, title_at, 0);
    let mut fnt = func::<NewFn>(NEW)(0xB0);
    if !fnt.is_null() {
        let colour: u32 = at(camp, camp::TEXT_COLOUR);
        fnt = func::<FntCtorFn>(FNT_CTOR)(fnt, 0, 0, &colour, 0xF, title.as_ptr(), 0x40);
    }
    put::<usize>(camp, title_at, fnt as usize);
    vfn::<ReplaceFn>(hdr, 0x68)(hdr, 1, set, fnt);
    func::<SprIntFn>(VISIBLE)(hdr, 1);
    put::<i32>(camp, camp::HEADER_MODE + 4 * idx, 2);
}

/// Re:CoM's confirm sequence for an entry with a page (state 0xF, `0x1400DAB0A..0x1400DACAC`),
/// for ours, without step 9 (as for its Deck entry, whose submenu keeps the main panel's layer 2).
/// Re:CoM's own update ran this frame and already eased the glove (`0x1400DE100`, which no step
/// here changes the target of).
unsafe fn confirm_ours(camp: *mut u8) {
    let set = set_of(camp);
    func::<GuideFn>(GUIDE)(0, 0);
    func::<VoidFn>(CONFIRM_FLASH)(camp);
    sound(2);
    cursor_set_ours(camp, top::OURS, top::OURS, false);
    let n = at::<i32>(camp, camp::COUNT).clamp(0, top::RECOM_ENTRIES);
    for i in 0..n {
        anim(at(camp, camp::BUTTONS + 8 * i as usize), set, 6);
    }
    anim(ENTRY_BTN.load(Ordering::Relaxed) as *mut u8, set, 6);
    header_title(camp, 0, text::TITLE);
    let hdr: *mut u8 = at(camp, camp::HEADER);
    if !hdr.is_null() {
        func::<SprIntFn>(VISIBLE)(hdr, 1);
    }
    anim(at(camp, camp::PANEL_140), set, 0xC);
    func::<CampIntFn>(CAMP_PANEL)(camp, 2, 0);
    // Step 9 (the main panel's layer 2 hidden) is Re:CoM's for its page entries; ours opens a
    // submenu, which (as Re:CoM's Deck entry) keeps it: the page confirm in the submenu hides it.
    put::<i32>(camp, camp::STATE, 0x11);
    put::<i32>(camp, camp::SAVED, top::OURS);
    func::<CampIntFn>(SIDE_PANEL)(camp, 4, 0);
}

/// Hook on `0x1400D9C80` (the camp update). Re:CoM's update always runs; with our entry, it
/// adds what Re:CoM's 7-entry code can't do for index 7.
pub unsafe extern "win64" fn camp_update(camp: *mut u8) {
    let o: VoidFn = orig(&ORIG_UPDATE);
    let ours = entry_of(camp);
    if ours.is_null() {
        return o(camp);
    }
    let state = at::<i32>(camp, camp::STATE);
    let cursor = at::<i32>(camp, camp::CURSOR);
    match state {
        // Re:CoM gives every entry animation 5 once button 0's animation has ended (then, in 0xD,
        // re-highlights the cursor's entry).
        7 | 0xD => {
            let b0: *mut u8 = at(camp, camp::BUTTONS);
            let was_ended = ended(b0);
            o(camp);
            if was_ended && !(state == 0xD && at::<i32>(camp, camp::CURSOR) == top::OURS) {
                anim(ours, set_of(camp), 5);
            }
        }
        0xF => {
            let fix = top::fix(cursor, top_pad());
            o(camp);
            let now = at::<i32>(camp, camp::STATE);
            if now == 0xF && at::<i32>(camp, camp::CURSOR) == cursor {
                match fix {
                    top::Fix::MoveToOurs => {
                        cursor_set_ours(camp, cursor, top::OURS, true);
                        sound(1);
                        func::<VoidFn>(GLIDE_STEP)(camp);
                    }
                    top::Fix::ConfirmOurs => confirm_ours(camp),
                    top::Fix::None => {}
                }
            } else if (now == 0x10 || now == 0x11) && cursor != top::OURS {
                // Re:CoM confirmed one of its entries: step 5 (every button animation 6) for ours.
                anim(ours, set_of(camp), 6);
            }
        }
        // State 0x11 reads a 7-byte table (`1D 16 41 1A 44 47 4A`): index 7 would read 0x2E, a
        // deck-editor state. Ours opens through the Config states 0x4A..0x4C.
        0x11 if cursor == top::OURS => {
            o(camp);
            let now = at::<i32>(camp, camp::STATE);
            if now != 0x11 {
                if now != 0x2E && !ODD_STATE_LOGGED.swap(true, Ordering::Relaxed) {
                    log(&format!("keyblades: state 0x11 left to {now:#x} for index 7 (0x2E expected); opening our page anyway"));
                }
                put::<i32>(camp, camp::STATE, 0x4A);
            }
        }
        _ => o(camp),
    }
}

/// Hook on `0x1400D7C20` (the camp teardown, both dtors and state 0x50): ours go with Re:CoM's.
pub unsafe extern "win64" fn teardown(camp: *mut u8) {
    let o: VoidFn = orig(&ORIG_TEARDOWN);
    if !camp.is_null() {
        if let Ok(mut g) = SUB.lock() {
            if g.as_ref().is_some_and(|s| s.camp == camp as usize) {
                if let Some(s) = g.take() {
                    s.delete_all();
                }
            }
        }
        if !entry_of(camp).is_null() {
            delete_entry();
        }
    }
    o(camp)
}

// ---------------------------------------------------------------------------------------------
// The submenu (M7 stage 7.13)

/// Where the submenu is (our state; never in camp fields Re:CoM's pages use).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SubState {
    /// Re:CoM's state 0x1D: the buttons play animation 4 (appear); when button 0's ends, idle
    /// (entry build mode 1) and the cursor set on the row.
    Opening,
    /// Re:CoM's state 0x22.
    Input,
    /// After a confirm: the buttons play animation 6 (fade); when button 0's ends, the page opens.
    ToPage,
    InPage,
    /// After a cancel: the buttons fade; when button 0's ends, back to the top list (−1).
    Closing,
}

/// The page open in the submenu: a list page (Re:CoM's entry-4 page) or a settings page (Re:CoM's
/// Config screen, `super::menu::SettingsPage`).
enum Open {
    List(Page),
    Settings(super::menu::SettingsPage),
}

impl Open {
    unsafe fn step(&mut self) -> i32 {
        match self {
            Open::List(p) => p.step(),
            Open::Settings(p) => p.step(),
        }
    }

    /// At the camp's teardown: a list page deletes its objects; a settings page's objects are the
    /// Config screen's, which Re:CoM's teardown `0x1400D7C20` deletes itself (`camp+0xB30`, the 21
    /// slots, `+0xBE0`). Checked (decomp).
    unsafe fn delete_all(&self) {
        if let Open::List(p) = self {
            p.delete_all();
        }
    }
}

/// The submenu and the page open in it.
struct Sub {
    camp: usize,
    state: SubState,
    row: i32,
    /// The rows shown (`sub::rows`), chosen at open and after a settings page.
    rows: &'static [sub::Row],
    /// The camp's story (Riku's: only Combat and Movement, with their Riku help lines).
    story: kb::Story,
    page: Option<Open>,
}

/// Re:CoM's relabel of its entry buttons (`0x1400E3DE0` mode 0 with a new level,
/// `0x1400E3FF3..0x1400E40C4`), with our texts: old label deleted, a new `C2DFnt` (colour
/// 0x80404040, font 0xF, 0x40) stored in `camp+0x1F0[i]` and put in the button's slot 1 with its
/// vtable `+0x68(btn, 1, set, fnt)`. Checked (disasm).
unsafe fn relabel(camp: *mut u8, labels: &[&'static [u8]]) {
    let set = set_of(camp);
    for (i, text) in labels.iter().enumerate().take(sub::MAX_ROWS) {
        let btn: *mut u8 = at(camp, camp::BUTTONS + 8 * i);
        if btn.is_null() {
            continue;
        }
        delete(at(camp, camp::LABELS + 8 * i));
        put::<usize>(camp, camp::LABELS + 8 * i, 0);
        let mut fnt = func::<NewFn>(NEW)(0xB0);
        if !fnt.is_null() {
            let colour = LABEL_NORMAL;
            fnt = func::<FntCtorFn>(FNT_CTOR)(fnt, 0, 0, &colour, 0xF, text.as_ptr(), 0x40);
        }
        put::<usize>(camp, camp::LABELS + 8 * i, fnt as usize);
        vfn::<ReplaceFn>(btn, 0x68)(btn, 1, set, fnt);
    }
}

/// Re:CoM's entry build (the original, past our wrapper): mode 0 at the current level only plays
/// animation 4 on the `count` buttons; mode 1 / 2 animation 5 / 6.
unsafe fn build(camp: *mut u8, mode: i32, level: u64) {
    let o: BuildFn = orig(&ORIG_BUILD);
    o(camp, mode, level);
}

/// Re:CoM's cursor set (the original: at level 1 our wrapper passes it through anyway).
unsafe fn recom_cursor(camp: *mut u8, idx: i32, show: bool) {
    let o: CursorSetFn = orig(&ORIG_CURSOR);
    o(camp, idx, show as i32);
}

unsafe fn button0_ended(camp: *mut u8) -> bool {
    ended(at(camp, camp::BUTTONS))
}

/// Main panel layer 2: Re:CoM hides it when a page opens (state 0xF step 9 for entries 2 / 4..6)
/// and shows it when the page is done (state 0x4C); its Deck submenu keeps it.
unsafe fn main_layer2(camp: *mut u8, show: bool) {
    let main: *mut u8 = at(camp, camp::MAIN_PANEL);
    if !main.is_null() {
        func::<SlotVisFn>(SLOT_VISIBLE)(main, 2, show as u32);
    }
}

impl Sub {
    fn p(&self) -> *mut u8 {
        self.camp as *mut u8
    }

    /// The page's first call (state 0x4A, after Re:CoM's state 0x11 saw the top buttons fade):
    /// what Re:CoM's state 0x10 does for the Deck at level 1, with our rows.
    unsafe fn open(camp: *mut u8) -> Sub {
        let story = story(camp);
        let rows = sub::rows(kh1_pages(camp));
        let labels: Vec<&'static [u8]> = rows.iter().map(|r| r.label()).collect();
        relabel(camp, &labels);
        put::<i32>(camp, camp::COUNT, rows.len() as i32);
        // Re:CoM's Deck submenu level; the cursor set and entry build read it (`< 2`: help).
        put::<i32>(camp, camp::LEVEL, 1);
        // Our wrapper reads the cursor as the top index; Re:CoM's cursor set unhighlights the
        // button at `camp+0xB8` first, which must be one of the submenu's.
        put::<i32>(camp, camp::CURSOR, 0);
        build(camp, 0, 1);
        log(&format!("kh1 entry: submenu opened ({} rows: {:?}; {})", rows.len(), rows, if story == kb::Story::Riku { "Riku's story" } else { "Sora's story" }));
        Sub { camp: camp as usize, state: SubState::Opening, row: 0, rows, story, page: None }
    }

    /// After a settings page: the rows for the Combat setting now (Combat may have changed on the
    /// Combat page). When they differ, Re:CoM's buttons are relabelled and the count set before
    /// they appear again (entry build mode 0 animates the first `count`; the others stay faded, as
    /// in Re:CoM's Deck submenu), and the cursor stays on the same row.
    unsafe fn refresh_rows(&mut self) {
        let rows = sub::rows(kh1_pages(self.p()));
        if rows == self.rows {
            return;
        }
        let camp = self.p();
        let was = self.rows.get(self.row as usize).copied().unwrap_or(sub::Row::Combat);
        let labels: Vec<&'static [u8]> = rows.iter().map(|r| r.label()).collect();
        relabel(camp, &labels);
        put::<i32>(camp, camp::COUNT, rows.len() as i32);
        self.rows = rows;
        self.row = sub::row_in(rows, was);
        log(&format!("kh1 entry: submenu rows now {rows:?} (Combat changed)"));
    }

    unsafe fn help(&self) {
        if let Some(r) = self.rows.get(self.row as usize) {
            set_help(self.p(), r.help_in(self.story).as_ptr());
        }
    }

    /// One frame (states 0x4A / 0x4B). ≥ 0 running, < 0 back to the top list.
    unsafe fn step(&mut self) -> i32 {
        let camp = self.p();
        match self.state {
            SubState::Opening => {
                if button0_ended(camp) {
                    build(camp, 1, 0);
                    put::<i32>(camp, camp::CURSOR, self.row);
                    recom_cursor(camp, self.row, true);
                    self.help();
                    self.state = SubState::Input;
                }
            }
            SubState::Input => {
                match sub::step(self.row, self.rows.len(), top_pad()) {
                    sub::Act::Close => {
                        recom_cursor(camp, self.row, false);
                        build(camp, 2, 0);
                        sound(4);
                        self.state = SubState::Closing;
                    }
                    sub::Act::Open(i) => {
                        // State 0x22's confirm: sound 2, header item 1 (the breadcrumb) with the
                        // row's title, buttons fade, highlight bar and glove hidden; Re:CoM's page
                        // entries' confirm also hides the main panel's layer 2.
                        sound(2);
                        header_title(camp, 1, self.rows[i].label());
                        build(camp, 2, 0);
                        func::<CampIntFn>(HIGHLIGHT_BAR)(camp, -1, 0);
                        func::<CampIntFn>(GLIDE_TARGET)(camp, -1, 0);
                        main_layer2(camp, false);
                        self.state = SubState::ToPage;
                    }
                    sub::Act::Move(i) => {
                        recom_cursor(camp, i, true);
                        sound(1);
                        self.row = i;
                        self.help();
                    }
                    sub::Act::None => {}
                }
                func::<VoidFn>(GLIDE_STEP)(camp);
            }
            SubState::ToPage => {
                if button0_ended(camp) {
                    let row = self.rows.get(self.row as usize).copied().unwrap_or(sub::Row::Combat);
                    self.page = Some(match row.settings_page() {
                        Some(sp) => Open::Settings(super::menu::SettingsPage::open(camp, sp)),
                        None => Open::List(Page::open(camp, row)),
                    });
                    self.state = SubState::InPage;
                }
            }
            SubState::InPage => {
                let r = self.page.as_mut().map_or(-1, |p| p.step());
                if r < 0 {
                    // Re:CoM's return from a Deck page (states 0x34 / 0x35): header item 1
                    // closes, the buttons appear again (entry build mode 0 at the same level),
                    // then idle and the cursor on the row (`Opening`).
                    let settings = matches!(self.page, Some(Open::Settings(_)));
                    self.page = None;
                    if settings {
                        self.refresh_rows();
                    }
                    func::<HeaderFn>(HEADER)(camp, 4, 1);
                    build(camp, 0, 1);
                    main_layer2(camp, true);
                    self.state = SubState::Opening;
                }
            }
            SubState::Closing => {
                if button0_ended(camp) {
                    // Back to the top list: Re:CoM's state 0x4B closes header item 0, 0x4C
                    // relabels its 7 entries (level 1 → 0, `0x1400E3DE0`), and 0xD highlights
                    // the cursor's entry: ours (`cursor_set`).
                    put::<i32>(camp, camp::CURSOR, top::OURS);
                    log("kh1 entry: submenu closed");
                    return -1;
                }
            }
        }
        0
    }

    unsafe fn delete_all(&self) {
        if let Some(p) = &self.page {
            p.delete_all();
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The pages

/// What a page lists.
enum Content {
    /// The Keyblade list, shown from the Items page's Keyblade slot (that page's content waits in
    /// [`Page::under`]).
    Keyblades {
        cards: Vec<u16>,
        equipped: u16,
    },
    /// KH1's Customize (`crate::customize`): the slot list, or (`picking` = the slot) the picker.
    Customize {
        levels: [u8; 7],
        slots: [u8; 3],
        picks: Vec<customize::Pick>,
        picking: Option<usize>,
    },
    /// M8 stage 8a.2: KH1's pause-menu Items screen (`crate::items_page`) on Sora's inventory in
    /// our save block (`win::items`). `inv`, `ctx` and `rows` are read again after every change.
    Items {
        screen: ip::Screen,
        /// KH1's message after a refusal, on the help line until the next move.
        msg: Option<ip::Msg>,
        inv: crate::items::Inventory,
        ctx: ip::Ctx,
        rows: Vec<ip::Row>,
    },
    /// M9 stage 9b: KH1's ability screen (`crate::abilities::page`) on the ability state in our
    /// save block (`win::abilities`). `state`, `level` and `rows` are read again after every change.
    Abilities {
        screen: ap::Screen,
        state: crate::abilities::State,
        level: u32,
        rows: Vec<ap::Row>,
        /// KH1's message after a refusal (not enough AP), on the help line until the next move.
        msg: Option<&'static [u8]>,
        /// The ability state is live (the KH1 setting, Sora's story, a save).
        live: bool,
    },
}

/// KH1's one picker cursor for every slot (`0x142E9A040`), kept for the session.
static LAST_PICK: AtomicUsize = AtomicUsize::new(0);

/// A page's objects and state (never in camp fields).
struct Page {
    camp: usize,
    sub: u8,
    stats_panel: usize,
    list_panel: usize,
    cursor: usize,
    bar: usize,
    rows: [usize; 8],
    stat_rows: [usize; 3],
    list: list::List,
    content: Content,
    /// The Items screen while its Keyblade list is shown; cancel there brings it back.
    under: Option<Content>,
}

/// An item's name: KH1's word as the command menu's Items list shows it (`kh1-command-menu.json`),
/// else our English name; 0 is KH1's empty slot.
fn item_word(item: u8) -> Vec<u8> {
    if item == 0 {
        return ip::text::EMPTY.to_vec();
    }
    crate::items::data(item).and_then(|d| super::command_menu::word(d.name)).unwrap_or_else(|| ip::text::english(item).to_vec())
}

/// The words of an Items row.
fn item_label(l: ip::Label) -> Vec<u8> {
    use ip::{Command as C, Label as L, Owner as O};
    let t: &[u8] = match l {
        // SAFETY: Re:CoM's card-name call (reads its message tables), as the Keyblade list's rows.
        L::Keyblade => return unsafe { card_name(super::stats::equipped_card()) },
        L::Owner(O::Sora) => ip::text::SORA,
        L::Owner(O::Stock) => ip::text::STOCK,
        L::Slot(i) => return item_word(i),
        L::Stocked(i, n) => return ip::stocked_text(&item_word(i), n),
        L::Command(C::Use, _) => ip::text::USE,
        L::Command(C::Move, _) => ip::text::MOVE,
        L::Command(C::EquipRemove, true) => ip::text::REMOVE,
        L::Command(C::EquipRemove, false) => ip::text::EQUIP,
        L::Command(C::Discard, _) => ip::text::DISCARD,
        L::Yes => ip::text::YES,
        L::No => ip::text::NO,
    };
    t.to_vec()
}

/// A spell's name at its level: KH1's word as the command menu shows it, else our English name.
fn spell_word(spell: u8, level: u8) -> Vec<u8> {
    super::command_menu::word(customize::name_id(spell, level)).unwrap_or_else(|| kb::c_text(crate::unlocks::level_name(spell, level.clamp(1, 3))))
}

impl Page {
    fn p(&self) -> *mut u8 {
        self.camp as *mut u8
    }

    /// Draw mode 6 of `0x1400E6DF0`: panels, rows, cursor, scroll bar (in Re:CoM's order).
    unsafe fn delete_all(&self) {
        delete(self.stats_panel as *mut u8);
        delete(self.list_panel as *mut u8);
        for r in self.stat_rows.iter().chain(self.rows.iter()) {
            delete(*r as *mut u8);
        }
        delete(self.cursor as *mut u8);
        delete(self.bar as *mut u8);
    }

    unsafe fn layout(&self, group: u32, elem: u64, flag: i32) -> Option<Vec4> {
        let lay: *const u8 = at(self.p(), camp::LAYOUT);
        if lay.is_null() {
            return None;
        }
        let mut v = Vec4::default();
        (func::<LayoutFn>(LAYOUT)(lay, group, elem, v.0.as_mut_ptr(), flag) != 0).then_some(v)
    }

    /// A `C2DSeqCtrl` panel at a layout position (draw mode 0).
    unsafe fn panel(&self, kind: u8, elem: u64, seq: u32) -> usize {
        let c = func::<NewFn>(NEW)(0x170);
        if c.is_null() {
            return 0;
        }
        func::<CtorFn>(SEQ_CTOR)(c);
        put::<u8>(c, 0x16E, kind);
        put::<u8>(c, 0x138, 1);
        let pos = self.layout(0x11, elem, 0).unwrap_or_default();
        func::<PosFn>(LAYOUT_POS)(c, pos.0.as_ptr());
        vfn::<SeqInitFn>(c, 0x8)(c, set_of(self.p()), seq, 0.0, 0);
        c as usize
    }

    /// Draw mode 0 for the submenu's row: the two panels, the cursor, the rows.
    unsafe fn open(camp: *mut u8, row: sub::Row) -> Page {
        let (content, list) = match row {
            sub::Row::Customize => {
                let (levels, slots) = super::command_menu::spell_view();
                let picks = customize::picks(levels);
                (Content::Customize { levels, slots, picks, picking: None }, list::open_on(customize::SLOTS as i32, 0))
            }
            sub::Row::Items => {
                let content = Content::Items { screen: ip::Screen::new(), msg: None, inv: Default::default(), ctx: Default::default(), rows: Vec::new() };
                (content, list::open_on(2, 0))
            }
            sub::Row::Abilities | sub::Row::Combat | sub::Row::Movement => {
                // Combat / Movement open a settings page (`Open::Settings`), never a list page.
                let content = Content::Abilities { screen: ap::Screen::default(), state: Default::default(), level: 1, rows: Vec::new(), msg: None, live: false };
                (content, list::open_on(2, 0))
            }
        };
        let mut p = Page { camp: camp as usize, sub: 1, stats_panel: 0, list_panel: 0, cursor: 0, bar: 0, rows: [0; 8], stat_rows: [0; 3], list, content, under: None };
        p.items_read();
        p.abilities_read();
        p.stats_panel = p.panel(0x10, 3, 0x125);
        p.list_panel = p.panel(0x11, 2, 0x14C);
        let c = func::<NewFn>(NEW)(0x1C0);
        if !c.is_null() {
            p.cursor = func::<CursorCtorFn>(CURSOR_CTOR)(c, 0x12, p.list_panel as *mut u8, 0x12D) as usize;
        }
        p.redraw();
        match &p.content {
            Content::Keyblades { .. } => {}
            Content::Customize { levels, slots, .. } => log(&format!("customize: page opened: spell levels {levels:?}, shortcut slots {slots:?} (0 none, else spell + 1)")),
            Content::Items { inv, ctx, .. } => log(&format!(
                "items page: opened: slots {:?} (of {}), stock {:?}, HP below max {}, MP below max {}{}",
                &inv.slots[..(inv.slot_count as usize).min(crate::items::MAX_SLOTS)],
                inv.slot_count,
                ip::stock_list(inv).iter().map(|&i| (i, inv.stock_of(i))).collect::<Vec<_>>(),
                ctx.hp_low,
                ctx.mp_low,
                if super::items::inventory().is_none() { " (KH1's items not live: nothing can change)" } else { "" }
            )),
            Content::Abilities { state, level, live, .. } => log(&format!(
                "abilities page: opened at Re:CoM level {level}: AP {}/{} left; unlocked {:?}; equipped {:?}{}",
                crate::abilities::ap_left(state.equipped, *level),
                crate::abilities::max_ap(*level),
                crate::abilities::names(state.unlocked),
                crate::abilities::names(state.equipped),
                if *live { "" } else { " (the ability state isn't live: nothing can change)" }
            )),
        }
        p
    }

    /// The Items page's Keyblade slot confirmed: the list shows the Keyblade list (the Keyblades
    /// page of before: the earned Keyblades, the equipped one marked, its numbers, the scroll bar),
    /// on the equipped one; the Items screen waits in `under`.
    unsafe fn open_keyblades(&mut self) {
        let cards = kb::unlocked(super::stats::keyblade_cards(), |k| ever_obtained(k));
        let equipped = super::stats::equipped_card();
        let at_eq = cards.iter().position(|&c| c == equipped).unwrap_or(0) as i32;
        self.list = list::open_on(cards.len() as i32, at_eq);
        log(&format!("keyblades: list opened from the Items page: {} Keyblades listed {:x?}, equipped {:#x}", cards.len(), cards, equipped));
        self.under = Some(std::mem::replace(&mut self.content, Content::Keyblades { cards, equipped }));
        // The entry-4 page's scroll bar (`0x140112AB0` / `0x1401130C0`) on the list panel, as the
        // Keyblades page made it.
        let b = func::<NewFn>(NEW)(0x28);
        if !b.is_null() {
            let bar = func::<CtorFn>(BAR_CTOR)(b);
            let mut pos = Vec4([133.0, -56.0, 0.0, 1.0]);
            func::<BarSetupFn>(BAR_SETUP)(bar, set_of(self.p()), 0x14, va(base(), BAR_SEQ) as *const u16, pos.0.as_mut_ptr(), 157.0, self.list_panel as *mut u8);
            self.bar = bar as usize;
        }
        self.redraw();
        self.help();
    }

    /// Cancel in the Keyblade list: back to the Items screen (Sora's list, on the Keyblade slot,
    /// now naming the equipped one); the scroll bar deleted as the page's close deletes it.
    unsafe fn back_from_keyblades(&mut self) {
        let Some(under) = self.under.take() else { return };
        self.content = under;
        delete(self.bar as *mut u8);
        self.bar = 0;
        self.items_read();
        self.redraw();
        self.help();
    }

    /// The text of list row `idx`.
    fn row_text(&self, idx: usize) -> Vec<u8> {
        match &self.content {
            // SAFETY: Re:CoM's card-name call (reads its message tables).
            Content::Keyblades { cards, .. } => unsafe { card_name(cards[idx]) },
            Content::Customize { levels, picks, picking: Some(_), .. } => match picks.get(idx).copied().flatten() {
                Some(s) => spell_word(s, levels[s as usize]),
                None => customize::text::NONE.to_vec(),
            },
            Content::Customize { picking: None, .. } => customize::text::slot_label(idx, button_glyphs(ROW_FONT), super::command_menu::confirm_is_circle()).to_vec(),
            Content::Items { rows, .. } => rows.get(idx).map_or_else(|| ip::text::NONE.to_vec(), |r| item_label(r.label)),
            Content::Abilities { rows, .. } => match rows.get(idx).map(|r| r.label) {
                Some(ap::Label::Owner(ap::Owner::Sora)) => ap::text::SORA.to_vec(),
                Some(ap::Label::Owner(ap::Owner::Shared)) => ap::text::SHARED.to_vec(),
                Some(ap::Label::Ability(i)) => kb::c_text(crate::abilities::ALL[i].name),
                None => ip::text::NONE.to_vec(),
            },
        }
    }

    /// A greyed row (Items, Abilities: KH1's greyed rows).
    fn row_grey(&self, idx: usize) -> bool {
        match &self.content {
            Content::Items { rows, .. } => rows.get(idx).is_some_and(|r| r.grey),
            Content::Abilities { rows, .. } => rows.get(idx).is_some_and(|r| r.grey),
            _ => false,
        }
    }

    /// The row's look on Re:CoM's list (`CBUTTON_SETUP`'s variant, `crate::keyblades::row_look`):
    /// the Items page's Stock row and item slots and the Abilities page's rows are Re:CoM's green
    /// row (green Mickey symbol); the Customize page's shortcut slots and spells are Re:CoM's blue
    /// magic row (owner, 2026-11-01); everything else keeps the red row.
    fn row_look(&self, idx: usize, item_row: bool) -> i32 {
        match &self.content {
            Content::Items { rows, .. } => rows.get(idx).map_or(kb::row_look::RED, |r| ip::row_look(r.label, item_row)),
            Content::Abilities { .. } => ap::ROW_LOOK,
            Content::Customize { picks, picking, .. } => customize::row_look(picking.is_some(), picks.get(idx).copied().flatten()),
            Content::Keyblades { .. } => kb::row_look::RED,
        }
    }

    /// Abilities: reads the ability state again (a sync: new unlocks from Re:CoM), the
    /// window's rows, and the list from the screen's cursor. Nothing for the other pages.
    fn abilities_read(&mut self) {
        let Content::Abilities { screen, state, level, rows, live, .. } = &mut self.content else { return };
        match super::abilities::page_state() {
            Some((s, l)) => {
                (*state, *level, *live) = (s, l, true);
            }
            None => *live = false,
        }
        *rows = screen.rows(state, *level);
        let n = rows.len() as i32;
        let c = screen.cursor().clamp(0, (n - 1).max(0));
        screen.set_cursor(c);
        self.list = list::open_on(n, c);
    }

    /// Items: reads the inventory and Sora's state again, the window's rows, and the list from the
    /// screen's cursor. Nothing for the other pages.
    fn items_read(&mut self) {
        let Content::Items { screen, inv, ctx, rows, .. } = &mut self.content else { return };
        *inv = super::items::inventory().unwrap_or_default();
        *ctx = super::items::menu_ctx();
        *rows = screen.rows(inv, *ctx);
        let c = screen.cursor(inv, *ctx);
        screen.set_cursor(c);
        let n = rows.len() as i32;
        self.list = list::open_on(n, c);
    }

    /// Whether row `idx` has Re:CoM's row mark: the equipped Keyblade, the picker's row holding the
    /// slot's spell, or an equipped ability.
    fn marked(&self, idx: usize) -> bool {
        match &self.content {
            Content::Keyblades { cards, equipped } => cards.iter().position(|c| c == equipped) == Some(idx),
            Content::Customize { slots, picks, picking: Some(k), .. } => customize::row_of(picks, slots[*k]) == Some(idx),
            Content::Customize { picking: None, .. } => false,
            Content::Items { .. } => false,
            Content::Abilities { rows, .. } => rows.get(idx).is_some_and(|r| r.marked),
        }
    }

    /// The status panel's three rows: labels and values.
    fn status(&self) -> ([&'static [u8]; 3], [Vec<u8>; 3]) {
        match &self.content {
            Content::Keyblades { cards, .. } => {
                let values = cards
                    .get(self.list.cursor as usize)
                    .and_then(|&c| super::stats::keyblade(c))
                    .map(kb::stat_values)
                    .unwrap_or_else(|| [String::new(), String::new(), String::new()]);
                (text::STAT_LABELS, values.map(|v| kb::c_text(&v)))
            }
            Content::Customize { levels, slots, .. } => {
                let v = |k: usize| match cm::shortcut::slot_spell(slots[k]) {
                    Some(s) => spell_word(s, levels.get(s as usize).copied().unwrap_or(1)),
                    None => customize::text::NONE.to_vec(),
                };
                (customize::text::slot_labels(button_glyphs(ROW_FONT), super::command_menu::confirm_is_circle()), [v(0), v(1), v(2)])
            }
            // Sora's Keyblade slot: the equipped Keyblade's numbers, as the Keyblade list shows them.
            Content::Items { screen, inv, ctx, .. } if screen.slot(inv, *ctx) == Some(ip::Slot::Keyblade) => {
                let values = super::stats::keyblade(super::stats::equipped_card()).map(kb::stat_values).unwrap_or_else(|| [String::new(), String::new(), String::new()]);
                (text::STAT_LABELS, values.map(|v| kb::c_text(&v)))
            }
            Content::Items { screen, inv, ctx, .. } => {
                let (owner, item) = screen.status(inv, *ctx);
                let o = item_label(ip::Label::Owner(owner));
                let (name, count) = if item == 0 { (ip::text::NONE.to_vec(), ip::text::NONE.to_vec()) } else { (item_word(item), kb::c_text(&inv.stock_of(item).to_string())) };
                (ip::text::STATUS, [o, name, count])
            }
            Content::Abilities { screen, state, level, .. } => (ap::text::STATUS, screen.status(state, *level).map(|v| kb::c_text(&v))),
        }
    }

    /// The help line for the page's current mode and row.
    unsafe fn help(&self) {
        let t: &[u8] = match &self.content {
            Content::Keyblades { .. } => text::PAGE_HELP,
            Content::Customize { picking: Some(k), .. } => customize::text::pick_help(*k, help_glyphs(self.p()), super::command_menu::confirm_is_circle()),
            Content::Customize { picking: None, .. } => customize::text::slot_help(self.list.cursor.max(0) as usize, help_glyphs(self.p()), super::command_menu::confirm_is_circle()),
            Content::Items { screen, rows, msg, .. } => screen.help(rows, self.list.cursor, *msg),
            Content::Abilities { msg: Some(m), .. } => m,
            Content::Abilities { rows, .. } => match rows.get(self.list.cursor.max(0) as usize) {
                Some(ap::Row { label: ap::Label::Owner(_), grey: true, .. }) | None => ap::text::EMPTY_HELP,
                Some(ap::Row { label: ap::Label::Owner(ap::Owner::Sora), .. }) => ap::text::OWNER_HELP[0],
                Some(ap::Row { label: ap::Label::Owner(ap::Owner::Shared), .. }) => ap::text::OWNER_HELP[1],
                Some(ap::Row { label: ap::Label::Ability(i), .. }) => ap::text::help(*i),
            },
        };
        set_help(self.p(), t.as_ptr());
    }

    /// Draw mode 9 (rows, cursor, scroll bar), plus the status rows.
    unsafe fn redraw(&mut self) {
        let camp = self.p();
        let set = set_of(camp);
        self.list.top = list::top_for(self.list);
        let no_scale = at::<i32>(camp, camp::NO_SCALE) != 0;
        let font_global: *const u8 = rd(va(base(), data::FONT_GLOBAL));
        let lang = if font_global.is_null() { 0 } else { rd::<i32>(font_global as usize + 0x274) };
        let (font, f9, f10) = if no_scale {
            (0xE, 0.0, 0.0)
        } else if (lang - 3) as u32 <= 2 {
            (0xE, 1.0, 2.0)
        } else {
            (0xF, 1.0, 2.0)
        };
        let colour: u32 = at(camp, camp::TEXT_COLOUR);
        let blink: u32 = at(camp, camp::BLINK);
        let setup = func::<SetupFn>(CBUTTON_SETUP);
        // The item row needs our table (Re:CoM's + the new group) and a loaded set that has it.
        let item_table = ITEM_TABLE.get().zip(ITEM_ROWS.get()).and_then(|(t, rows)| {
            let r = item_row_loaded(set, rows)?;
            t[crate::camp_rows::RECOM_ENTRIES].store(r, Ordering::Relaxed);
            Some(t)
        });
        let seqs = item_table.map_or(va(base(), LIST_SEQ) as *const u16, |t| t.as_ptr() as *const u16);
        let item_row = item_table.is_some();
        for i in 0..list::VISIBLE as usize {
            delete(self.rows[i] as *mut u8);
            self.rows[i] = 0;
            let idx = self.list.top + i as i32;
            if idx >= self.list.count {
                continue;
            }
            let pos = self.layout(0x12, 2 + i as u64, 0).unwrap_or_default();
            let btn = func::<NewFn>(NEW)(0x30);
            if btn.is_null() {
                continue;
            }
            func::<CtorFn>(CBUTTON_CTOR)(btn);
            self.rows[i] = btn as usize;
            let name = self.row_text(idx as usize);
            let c = if self.row_grey(idx as usize) { ip::grey_colour(colour) } else { colour };
            setup(btn, set, 0x11, seqs, pos.0.as_ptr(), self.list_panel as *mut u8, name.as_ptr(), font, f9, f10, 0x40, &c, self.row_look(idx as usize, item_row));
            let spr: *mut u8 = at(btn, 0x18);
            if self.marked(idx as usize) && !spr.is_null() {
                let m = vfn::<MarkFn>(spr, 0x40)(spr, set, EQUIPPED_MARK, MARK_X, MARK_Y, 0.0, 0);
                if !m.is_null() {
                    vfn::<ObjIntFn>(m, 0x30)(m, blink);
                }
            }
        }
        self.place_cursor();
        if self.bar != 0 {
            let (t, s) = list::bar(self.list);
            func::<BarUpdateFn>(BAR_UPDATE)(self.bar as *mut u8, t, s);
        }
        self.stats();
    }

    /// The cursor onto the highlighted row (draw mode 9's tail, `0x1400E7ABF..0x1400E7B6F`).
    unsafe fn place_cursor(&self) {
        let cur = self.cursor as *mut u8;
        let r = (self.list.cursor - self.list.top) as usize;
        if cur.is_null() || r >= self.rows.len() || self.rows[r] == 0 {
            return;
        }
        let spr: *mut u8 = at(self.rows[r] as *mut u8, 0x18);
        if spr.is_null() {
            return;
        }
        func::<PosFn>(SPRITE_POS)(cur, spr.add(0xF0) as *const f32);
        if at::<usize>(cur, 0x170) == 0 {
            return;
        }
        let v: Vec4 = at(spr, 0xF0);
        let t = Vec4([v.0[0] + CURSOR_DX, v.0[1] + CURSOR_DY, v.0[2], v.0[3]]);
        put::<u64>(cur, 0x1B0, 1);
        put::<Vec4>(cur, 0x190, t);
        let src: Vec4 = if at::<i32>(cur, 0x1B8) != 0 { at(cur, 0x180) } else { t };
        put::<Vec4>(cur, 0x1A0, src);
    }

    /// The status rows (draw mode 0's label / value rows, layout group 0x11 elements 4.., y + 8),
    /// rebuilt on each change as the list rows are.
    unsafe fn stats(&mut self) {
        let camp = self.p();
        let set = set_of(camp);
        let no_scale = at::<i32>(camp, camp::NO_SCALE) != 0;
        let y = if no_scale { 0.0 } else { 1.0 };
        let (labels, values) = self.status();
        let setup = func::<SetupFn>(CBUTTON_SETUP);
        let seqs = va(base(), STATUS_SEQ) as *const u16;
        for (i, value) in values.iter().enumerate() {
            delete(self.stat_rows[i] as *mut u8);
            self.stat_rows[i] = 0;
            let mut pos = self.layout(0x11, 4 + i as u64, 1).unwrap_or_default();
            pos.0[1] += 8.0;
            let btn = func::<NewFn>(NEW)(0x30);
            if btn.is_null() {
                continue;
            }
            func::<CtorFn>(CBUTTON_CTOR)(btn);
            self.stat_rows[i] = btn as usize;
            setup(btn, set, 0x10, seqs, pos.0.as_ptr(), self.stats_panel as *mut u8, labels[i].as_ptr(), 0xE, 0.0, y, 0x40, std::ptr::null(), i as i32);
            let mut v = func::<NewFn>(NEW)(0xB0);
            if !v.is_null() {
                let colour = VALUE_COLOUR;
                v = func::<FntCtorFn>(FNT_CTOR)(v, 0, 0, &colour, 0xE, std::ptr::null(), 0x41);
                func::<TextFn>(TEXT_COPY)(v, value.as_ptr());
            }
            let spr: *mut u8 = at(btn, 0x18);
            if !spr.is_null() {
                vfn::<AttachFn>(spr, 0x60)(spr, set, v, 220.0, y, 0.0);
            }
        }
    }

    /// Draw mode 3: the cursor glides (`0x1400DE010(cursor, dt × 0.02)`).
    unsafe fn glide(&self) {
        if self.cursor != 0 {
            let dt = at::<i32>(self.p(), camp::DT) as f32 * 0.020_000_001;
            func::<GlideFn>(CURSOR_GLIDE)(self.cursor as *mut u8, dt);
        }
    }

    /// Sub-state 1 (draw mode 1): when the status panel's opening ends, both panels idle, the
    /// cursor shows, the help line is ours.
    unsafe fn opening(&mut self) -> bool {
        let done = ended(self.stats_panel as *mut u8);
        if done {
            let set = set_of(self.p());
            anim(self.stats_panel as *mut u8, set, 0x126);
            anim(self.list_panel as *mut u8, set, 0x14D);
            let cur = self.cursor as *mut u8;
            if !cur.is_null() {
                func::<SprIntFn>(VISIBLE)(cur, 1);
                let child: *mut u8 = at(cur, 0x170);
                if !child.is_null() {
                    func::<SprIntFn>(VISIBLE)(child, 1);
                }
            }
            self.help();
        }
        self.glide();
        done
    }

    /// Draw mode 4, sound 4, sub-state 3: the page closes.
    unsafe fn close(&mut self) {
        let camp = self.p();
        let set = set_of(camp);
        anim(self.stats_panel as *mut u8, set, 0x127);
        anim(self.list_panel as *mut u8, set, 0x14E);
        if self.cursor != 0 {
            func::<VoidFn>(CURSOR_HIDE)(self.cursor as *mut u8);
        }
        set_help(camp, message(rd::<u32>(va(base(), CLOSE_HELP_ID))));
        sound(4);
        self.sub = 3;
    }

    /// Customize: the list shows the slots (`slot` highlighted) or the picker for a slot.
    unsafe fn customize_mode(&mut self, picking: Option<usize>, cursor: i32) {
        let Content::Customize { picking: p, picks, .. } = &mut self.content else { return };
        *p = picking;
        let count = if picking.is_some() { picks.len() as i32 } else { customize::SLOTS as i32 };
        self.list = list::open_on(count, cursor);
        self.redraw();
        self.help();
    }

    /// Confirm on the highlighted row.
    unsafe fn confirm(&mut self) {
        let cursor = self.list.cursor;
        match &self.content {
            Content::Keyblades { cards, .. } => {
                let Some(card) = cards.get(cursor as usize).copied() else { return };
                super::stats::set_equipped(card);
                let now = super::stats::equipped_card();
                if let Content::Keyblades { equipped, .. } = &mut self.content {
                    *equipped = now;
                }
                // Its model starts loading now; the weapon is rebuilt when the camp closes.
                super::models::prefetch();
                sound(2);
                self.redraw();
                log(&format!("keyblades: equipped card {card:#x} (save byte {:#x})", kb::save_byte_for(card)));
            }
            Content::Customize { picking: None, picks, .. } => {
                // KH1's slot list → its picker, on the picker's last row (`0x142E9A040`).
                let at = customize::picker_cursor(LAST_PICK.load(Ordering::Relaxed) as i32, picks.len() as i32);
                sound(2);
                self.customize_mode(Some(cursor as usize), at);
            }
            Content::Customize { picking: Some(k), picks, slots, levels } => {
                let (k, slots, levels) = (*k, *slots, *levels);
                let pick = picks.get(cursor as usize).copied().flatten();
                let new = customize::set(slots, k, pick);
                let name = |p: customize::Pick| p.map_or_else(|| "None".to_string(), |s| crate::unlocks::level_name(s, levels[s as usize].clamp(1, 3)).to_string());
                if super::mp::set_shortcuts(new) {
                    if let Content::Customize { slots: s, .. } = &mut self.content {
                        *s = new;
                    }
                    sound(2);
                    log(&format!("customize: shortcut {} = {} (was {}); slots {new:?}", k + 1, name(pick), name(cm::shortcut::slot_spell(slots[k]))));
                } else {
                    sound(5);
                    log("customize: the save block isn't there (MP not live); shortcut not set");
                }
                self.customize_mode(None, k as i32);
            }
            Content::Items { .. } => self.items_confirm(),
            Content::Abilities { .. } => self.abilities_confirm(),
        }
    }

    /// Abilities: confirm on the highlighted row (`crate::abilities::page::Screen::confirm`, KH1's
    /// rule), written to our save block, with Re:CoM's camp sounds (2 opened / equipped / removed,
    /// 5 refused or nothing to open).
    unsafe fn abilities_confirm(&mut self) {
        let cursor = self.list.cursor;
        let Content::Abilities { screen, state, level, msg, live, .. } = &mut self.content else { return };
        screen.set_cursor(cursor);
        *msg = None;
        let mut s = *state;
        let out = screen.confirm(&mut s, *level);
        match out {
            ap::Outcome::Opened => sound(2),
            ap::Outcome::Equipped(i) | ap::Outcome::Removed(i) => {
                if !*live || !super::abilities::set_equipped(s.equipped) {
                    sound(5);
                    log("abilities page: the ability state isn't live (no save block, or not Combat = KH1 in Sora's story); nothing changed");
                } else {
                    sound(2);
                    let lv = *level;
                    log(&format!(
                        "abilities page: {} {} ({} AP); AP {}/{} left; equipped {:?}",
                        if matches!(out, ap::Outcome::Equipped(_)) { "equipped" } else { "removed" },
                        crate::abilities::ALL[i].name,
                        crate::abilities::ALL[i].ap,
                        crate::abilities::ap_left(s.equipped, lv),
                        crate::abilities::max_ap(lv),
                        crate::abilities::names(s.equipped)
                    ));
                }
            }
            ap::Outcome::Refused(i) => {
                *msg = Some(ap::text::NO_AP);
                sound(5);
                log(&format!("abilities page: refused {} ({} AP): AP {} left (KH1's rule)", crate::abilities::ALL[i].name, crate::abilities::ALL[i].ap, crate::abilities::ap_left(s.equipped, *level)));
            }
            ap::Outcome::Empty => {
                *msg = Some(ap::text::EMPTY_HELP);
                sound(5);
            }
            ap::Outcome::Nothing | ap::Outcome::Back | ap::Outcome::Close => {}
        }
        self.abilities_read();
        self.redraw();
        self.help();
    }

    /// Items: confirm on the highlighted row (`crate::items_page::Screen::confirm` on our save
    /// block's inventory), with Re:CoM's camp sounds (2 opened / done, 5 refused).
    unsafe fn items_confirm(&mut self) {
        let cursor = self.list.cursor;
        let Content::Items { screen, msg, ctx, inv: shown, .. } = &mut self.content else { return };
        screen.set_cursor(cursor);
        let ctx = *ctx;
        // The Keyblade slot needs no item store: tried on copies when the items aren't live.
        let out = match super::items::edit(|inv| screen.confirm(inv, ctx)) {
            Some(out) => out,
            None if screen.clone().confirm(&mut shown.clone(), ctx) == ip::Outcome::Keyblade => ip::Outcome::Keyblade,
            None => {
                sound(5);
                log("items page: KH1's items aren't live (no save block, or not Combat = KH1 in Sora's story); nothing changed");
                return;
            }
        };
        *msg = None;
        if out == ip::Outcome::Keyblade {
            sound(2);
            self.open_keyblades();
            return;
        }
        match out {
            ip::Outcome::Nothing => {}
            ip::Outcome::Refused => sound(5),
            ip::Outcome::Error(m) => {
                *msg = Some(m);
                sound(5);
                log(&format!("items page: refused: {m:?}"));
            }
            ip::Outcome::Opened | ip::Outcome::Keyblade => sound(2),
            ip::Outcome::Back => sound(4),
            ip::Outcome::Close => {}
            ip::Outcome::Done(e) => {
                sound(2);
                let extra = match e {
                    ip::Effect::Used { item, .. } => format!(": {}", super::items::use_in_menu(item)),
                    _ => String::new(),
                };
                let inv = super::items::inventory().unwrap_or_default();
                log(&format!(
                    "items page: {e:?}{extra}; slots {:?}, stock {:?}",
                    &inv.slots[..(inv.slot_count as usize).min(crate::items::MAX_SLOTS)],
                    ip::stock_list(&inv).iter().map(|&i| (i, inv.stock_of(i))).collect::<Vec<_>>()
                ));
            }
        }
        if out == ip::Outcome::Close {
            self.close();
            return;
        }
        self.items_read();
        self.redraw();
        self.help();
    }

    /// Cancel: the picker goes back to the slot list (no write), the Keyblade list to the Items
    /// screen; the Items screen steps back a window; anything else closes the page.
    unsafe fn cancel(&mut self) {
        if matches!(self.content, Content::Keyblades { .. }) && self.under.is_some() {
            sound(4);
            self.back_from_keyblades();
            return;
        }
        if let Content::Customize { picking: Some(k), .. } = self.content {
            sound(4);
            self.customize_mode(None, k as i32);
            return;
        }
        if let Content::Abilities { screen, msg, .. } = &mut self.content {
            *msg = None;
            if screen.cancel() == ap::Outcome::Back {
                sound(4);
                self.abilities_read();
                self.redraw();
                self.help();
                return;
            }
        }
        if let Content::Items { screen, msg, .. } = &mut self.content {
            *msg = None;
            if screen.cancel() == ip::Outcome::Back {
                sound(4);
                self.items_read();
                self.redraw();
                self.help();
                return;
            }
        }
        self.close();
    }

    /// Sub-state 2: the list's input, in `0x1400EB7E0`'s order. Returns the page's result.
    unsafe fn input(&mut self) -> i32 {
        let camp = self.p();
        self.glide();
        if func::<CampBtnFn>(CAMP_CANCEL)(camp, 0) & 0xFF != 0 {
            self.cancel();
            return 1;
        }
        if func::<CampBtnFn>(CAMP_DECIDE)(camp, 0) & 0xFF != 0 {
            self.confirm();
            return 0;
        }
        let dir = func::<CampDirFn>(CAMP_DIR);
        let mut pad = list::Pad::default();
        if dir(camp, 0x10_1000, 0) != 0 {
            pad.up = true;
            pad.up_fresh = at::<i32>(camp, camp::FRESH) != 0;
        } else if dir(camp, 0x20_4000, 0) != 0 {
            pad.down = true;
            pad.down_fresh = at::<i32>(camp, camp::FRESH) != 0;
        } else {
            let rep = func::<PadFn>(PAD_REPEAT);
            let fresh = func::<PadFn>(PAD_FRESH);
            if rep(4, 0) != 0 {
                pad.page_up = true;
                pad.page_up_fresh = fresh(4, 0) != 0;
            } else if rep(8, 0) != 0 {
                pad.page_down = true;
                pad.page_down_fresh = fresh(8, 0) != 0;
            }
        }
        let (n, moved) = list::step(self.list, pad);
        self.list = n;
        if let Content::Items { screen, msg, .. } = &mut self.content {
            screen.set_cursor(n.cursor);
            if moved {
                *msg = None;
            }
        }
        if let Content::Abilities { screen, msg, .. } = &mut self.content {
            screen.set_cursor(n.cursor);
            if moved {
                *msg = None;
            }
        }
        if moved {
            sound(1);
            self.redraw();
            match self.content {
                Content::Customize { picking: Some(_), .. } => LAST_PICK.store(n.cursor.max(0) as usize, Ordering::Relaxed),
                Content::Customize { picking: None, .. } | Content::Items { .. } | Content::Abilities { .. } => self.help(),
                Content::Keyblades { .. } => {}
            }
        }
        0
    }

    /// One frame: sub-state 1 opening, 2 input, 3 closing (−1 when the status panel's close
    /// ends, its objects deleted).
    unsafe fn step(&mut self) -> i32 {
        match self.sub {
            1 => {
                if self.opening() {
                    self.sub = 2;
                }
                0
            }
            2 => self.input().min(0),
            _ => {
                if ended(self.stats_panel as *mut u8) {
                    self.delete_all();
                    self.stats_panel = 0;
                    self.list_panel = 0;
                    self.cursor = 0;
                    self.bar = 0;
                    self.rows = [0; 8];
                    self.stat_rows = [0; 3];
                    -1
                } else {
                    0
                }
            }
        }
    }
}

/// The Keyblade's localized card name (`0x140164CA0`, message `[0x140528560 + kind × 4]`), or the
/// stats file's name when that message isn't loaded.
unsafe fn card_name(card: u16) -> Vec<u8> {
    let id = [card as u8, 0, 0, 0];
    let p = func::<NameFn>(CARD_NAME)(id.as_ptr());
    if !p.is_null() && *p != 0 {
        let s = std::ffi::CStr::from_ptr(p as *const std::ffi::c_char);
        let mut v = s.to_bytes().to_vec();
        v.push(0);
        return v;
    }
    kb::c_text(super::stats::keyblade(card).map(|k| k.name.as_str()).unwrap_or("?"))
}

/// Re:CoM's "obtained at least once" (`0x14015B3F0`), when the collection is there.
pub(super) unsafe fn ever_obtained(id: u16) -> bool {
    let coll: usize = rd(va(base(), COLLECTION));
    coll != 0 && func::<EverFn>(EVER_OBTAINED)(id as u32) & 0xFF != 0
}

/// Our entry's submenu and pages, called by the Config input hook in place of `0x1400D3EB0` when
/// [`page_active`]: the same contract (≥ 0 running, < 0 closed). Re:CoM's states 0x4A..0x4C around
/// it.
pub unsafe fn page(camp: *mut u8) -> u64 {
    let Ok(mut g) = SUB.lock() else { return (-1i32) as u32 as u64 };
    if g.as_ref().is_some_and(|s| s.camp != camp as usize) {
        if let Some(stale) = g.take() {
            stale.delete_all();
        }
    }
    let r: i32 = match g.as_mut() {
        // State 0x4A's call: the submenu's buttons.
        None => {
            *g = Some(Sub::open(camp));
            0
        }
        Some(s) => s.step(),
    };
    if r < 0 {
        *g = None;
    }
    r as u32 as u64
}
