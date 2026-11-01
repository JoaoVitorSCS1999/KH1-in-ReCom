//! KH1's command menu (M7 stage 7.3; `crate::command_menu`, `docs/recom-m7-command-menu.md`),
//! drawn by Re:CoM.
//!
//! - **Objects:** one `D2::C2DSeqCtrl` per visible row (Re:CoM's `operator new` and ctor), with
//!   one layer of `Cockpit:00` (Re:CoM's text plate, or the same plate in the highlight colour:
//!   the two groups the asset step added after the MP bar's) and a `C2DFnt` with KH1's word,
//!   attached as Re:CoM's `CStockFolder` attaches its labels to these plates (`0x1401658F0`: font
//!   0xE, flags 0x80, vtable `+0x60` at (0, 0) with the screen offset − 4). Placed with Re:CoM's
//!   layout position (`0x140248660`, the 512 × 384 space) at KH1's coordinates, shifted by the
//!   player gauge's x offset as every HUD layer (`+0xA0`). Re:CoM's 2D manager animates and draws
//!   them; the fade is Re:CoM's control colour (`0x140248590`, `+0x138` = 1) and the text colour
//!   (`C2DFnt +0x34`).
//! - **When:** made with the player gauge (the MP bar's wrapper on `CPlayerGauge` init), hidden
//!   and shown with Re:CoM's HUD (its second-bar state wrapper), deleted before the gauge's own
//!   controls (its dtor wrapper). Each frame of Sora's at the KH1 setting the request-bits wrapper
//!   runs KH1's menu step ([`frame`]): pad and build, then the rows are rebuilt only when what
//!   they show changed (a cursor move, a list opened, a word changed), and only colours are
//!   written while a fade or a brightness change runs.
//! - Only at Combat = KH1 in Sora's story (`super::mp::active`), with the asset step's
//!   `kh1-command-menu.json` and the plate groups in the loaded `Cockpit:00`.

use super::movement::{self as mv, body, func, p, rd, rd_vec, wr};
use super::{base, log};
use crate::command_menu::{self as cm, Event, Kind, Menu, Pad, RowView, Scene, Words, World};
use crate::sites::COMMAND_MENU_PTRS;
use crate::units::to_kh1;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// A menu is built on some live gauge (the per-frame step's only test otherwise).
static LIVE: AtomicBool = AtomicBool::new(false);
static WORDS: OnceLock<Words> = OnceLock::new();
static ALL_SPELLS: AtomicBool = AtomicBool::new(false);
static LOG_LEFT: AtomicU32 = AtomicU32::new(60);
/// Row rebuilds so far (each deletes the shown rows and makes the new ones): the 2D census is
/// logged every 100th.
static REBUILDS: AtomicU32 = AtomicU32::new(0);
/// KH1's MP as last read or written (`super::mp`), packed; bit 63 = known.
static MP_VIEW: AtomicU64 = AtomicU64::new(0);
static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static STATE: Mutex<State> = Mutex::new(State { menu: None, uis: Vec::new() });

// Re:CoM functions (RVAs; first bytes checked in `sites`).
const NEW: u32 = 0x004A_CE54;
const SEQ_CTOR: u32 = 0x0024_7260;
const LAYOUT_POS: u32 = 0x0024_8660;
const SCALE: u32 = 0x0024_86D0;
const COLOUR: u32 = 0x0024_8590;
const FNT_CTOR: u32 = 0x0024_3F10;
const SOUND: u32 = 0x002A_A360;
const PAD_FRESH: u32 = 0x0029_B2F0;
const PAD_CANCEL: u32 = 0x0029_B410;
const PAD_CONFIRM: u32 = 0x0029_B490;
/// CGimmickDoor's type getter (its vtable `+0x28`); the others are `mov eax, imm32; ret`.
const DOOR_TYPE: u32 = 0x001B_8A10;
const SEQ_CTRL_VTABLE: u32 = 0x0060_7340;

/// `CPlayerGauge` fields (as `win/mp_bar.rs`): the x offset, the display state.
const G_XOFF: usize = 0xA0;
const G_STATE: usize = 0xAA;
const STATE_SHOW: u32 = 1;
const STATE_HIDDEN: u32 = 7;
/// `C2DSeqCtrl`: draw priority, "use the colour" (`0x14024BB10` multiplies by `+0x120` when set).
const C_PRIORITY: usize = 0x16E;
const C_COLOUR_ON: usize = 0x138;
/// The root rows at the HUD's priority (the HP gauge's 2), the Magic list over them.
const PRIORITY_ROOT: u8 = 2;
const PRIORITY_SUB: u8 = 3;
/// `C2DFnt`: its colour (`0x140258BC0`), as `CStockFolder` makes it (`0x1401658F0`: font 0xE,
/// flags 0x80, white 0x80808080) and attaches it ((0, 0), screen offset −4 − the HUD's).
const F_COLOUR: usize = 0x34;
const FONT: i32 = 0xE;
const FONT_FLAGS: i32 = 0x80;
const TEXT_DX: f32 = -4.0;
/// Player fields: the action-target handles the field action reads (`0x1401E80C0`), the
/// treasure box's opened bit (`0x1401E80C0`: `+0x2C1 & 1`).
const ACTION_TARGET: usize = 0x5FC;
const BOX_OPENED: usize = 0x2C1;

type NewFn = unsafe extern "win64" fn(usize) -> usize;
type CtorFn = unsafe extern "win64" fn(usize) -> usize;
type VecFn = unsafe extern "win64" fn(usize, *const f32);
type FntCtorFn = unsafe extern "win64" fn(usize, i32, i32, *const u32, i32, *const u8, i32) -> usize;
type SoundFn = unsafe extern "win64" fn(u32, i32) -> u64;
type PadFn = unsafe extern "win64" fn(u32, u32) -> u32;
type PadBtnFn = unsafe extern "win64" fn(u32) -> u32;

#[repr(C, align(16))]
struct V4([f32; 4]);

struct Row {
    ctrl: usize,
    fnt: usize,
    view: RowView,
}

struct Ui {
    gauge: usize,
    /// Re:CoM's draw list the gauge was made on: the rows' layers are added and released on it
    /// (`super::d2`).
    list: i32,
    xoff: f32,
    hidden: bool,
    rows: Vec<Row>,
    shown: Option<Scene>,
}

struct State {
    menu: Option<Menu>,
    uis: Vec<Ui>,
}

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("command menu: {}", line()));
    }
}

fn event(e: String) {
    if let Ok(mut g) = EVENTS.lock() {
        if g.len() < 16 {
            g.push(e);
        }
    }
}

/// Reads the words, checks the vtable slot. The menu needs the MP bar's wrappers and its plates.
pub fn install(b: usize, manifest: Option<&std::path::Path>, s: &crate::settings::Settings) -> bool {
    let Some(path) = manifest else {
        log("command menu: no kh1-command-menu.json (asset step not run?): no command menu");
        return false;
    };
    let words = match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| cm::parse_words(&t, &s.menu_language)) {
        Ok(w) => w,
        Err(e) => {
            log(&format!("command menu: {}: {e}; no command menu", path.display()));
            return false;
        }
    };
    // SAFETY: reads of the mapped image (the build was checked at start).
    unsafe {
        for p in COMMAND_MENU_PTRS {
            let have: usize = rd(b + p.rva as usize);
            if have != b + p.target as usize {
                log(&format!("command menu: {} holds {have:#x}, not {:#x}; no command menu", p.name, b + p.target as usize));
                return false;
            }
        }
    }
    // 0.17.x checked `menu_groups()` here, which is None until a bar is built (later, on the
    // first player gauge): the menu was never installed (owner, 2026-10-31: "The command menu
    // isnt there").
    if !super::mp_bar::has_menu_groups() {
        log("command menu: kh1-mp-bar.json has no plate groups (an older asset step): no command menu");
        return false;
    }
    ALL_SPELLS.store(s.kh1_all_spells, Ordering::Relaxed);
    let folded: String = words.folded.iter().collect();
    // SAFETY: a static pointer of the image.
    let lang_index = unsafe {
        let g: usize = rd(super::va(base(), crate::sites::data::FONT_GLOBAL));
        if g == 0 {
            -1
        } else {
            rd::<i32>(g + 0x274)
        }
    };
    log(&format!(
        "command menu: KH1's command menu at combat = kh1 in Sora's story (M7 stage 7.3): words {} ({} entries{}) from {}; Re:CoM language index {lang_index}; kh1_all_spells = {}",
        words.lang,
        words.by_id.len(),
        if folded.is_empty() { String::new() } else { format!(", written without accents: {folded}") },
        path.display(),
        crate::settings::switch_str(s.kh1_all_spells)
    ));
    let _ = WORDS.set(words);
    INSTALLED.store(true, Ordering::Release);
    true
}

/// KH1's MP as just read or written (`super::mp`): kept for the menu's greying, no lock.
pub fn mp_seen(view: (i32, i32, u16)) {
    let packed = 1u64 << 63 | (view.0 as u8 as u64) << 24 | (view.1 as u8 as u64) << 16 | view.2 as u64;
    MP_VIEW.store(packed, Ordering::Relaxed);
}

fn mp_view() -> Option<(i32, i32, u16)> {
    let v = MP_VIEW.load(Ordering::Relaxed);
    (v >> 63 != 0).then(|| ((v >> 24) as u8 as i32, (v >> 16) as u8 as i32, v as u16))
}

/// A new player gauge (its init and the MP bar have run, on draw list `list`): the menu is made
/// for it, fading in.
pub unsafe fn build(gauge: usize, list: i32) {
    if !INSTALLED.load(Ordering::Acquire) || !super::mp::active() || super::mp_bar::menu_groups().is_none() {
        return;
    }
    if mp_view().is_none() {
        let _ = super::mp::current();
    }
    let hidden = rd::<u16>(gauge + G_STATE) as u32 == STATE_HIDDEN;
    let ui = Ui { gauge, list, xoff: -rd::<f32>(gauge + G_XOFF), hidden, rows: Vec::new(), shown: None };
    if let Ok(mut s) = STATE.lock() {
        for mut old in s.uis.extract_if(.., |u| u.gauge == gauge) {
            old.clear();
        }
        let m = s.menu.get_or_insert_with(Menu::new);
        // A new HUD (a room or a battle): back to the root and Attack, fading in (KH1's
        // `0x140278AC0(1)` when its step starts running again).
        m.hide();
        if !hidden {
            m.fade_in();
        }
        s.uis.push(ui);
        LIVE.store(true, Ordering::Release);
    }
    note(|| format!("built on the player gauge {gauge:#x}{}", if hidden { " (HUD hidden)" } else { "" }));
}

/// Re:CoM's HUD hides (7) or shows (1) the player gauge: KH1's instant hide / show.
pub unsafe fn hud(gauge: usize, state: u32) {
    if state != STATE_HIDDEN && state != STATE_SHOW {
        return;
    }
    let Ok(mut s) = STATE.lock() else { return };
    let State { menu, uis } = &mut *s;
    let was_all_hidden = uis.iter().all(|u| u.hidden);
    let Some(ui) = uis.iter_mut().find(|u| u.gauge == gauge) else { return };
    if state == STATE_HIDDEN && !ui.hidden {
        ui.hidden = true;
        ui.clear();
    } else if state == STATE_SHOW && ui.hidden {
        ui.hidden = false;
    }
    // The menu (shared by every live gauge: a field one may linger while a battle one shows)
    // hides when no gauge shows, and shows when one shows again.
    let all_hidden = uis.iter().all(|u| u.hidden);
    if let Some(m) = menu {
        if all_hidden && !was_all_hidden {
            m.hide();
        } else if !all_hidden && was_all_hidden {
            m.show();
        }
    }
}

/// The gauge goes (its dtor): our controls first.
pub unsafe fn destroy(gauge: usize) {
    if let Ok(mut s) = STATE.lock() {
        for mut u in s.uis.extract_if(.., |u| u.gauge == gauge) {
            u.clear();
        }
        LIVE.store(!s.uis.is_empty(), Ordering::Release);
    }
}

/// After the Config or the camp: the menu follows the setting; spells may have changed. `gauges`:
/// the live player gauges and their draw lists.
pub fn refresh(gauges: &[(usize, i32)]) {
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    if super::mp::active() {
        for &(g, list) in gauges {
            let have = STATE.lock().map(|s| s.uis.iter().any(|u| u.gauge == g)).unwrap_or(true);
            if !have {
                // SAFETY: a live gauge (the MP bar's list; its dtor removes it).
                unsafe { build(g, list) };
            }
        }
    } else if let Ok(mut s) = STATE.lock() {
        for mut u in s.uis.drain(..) {
            // SAFETY: live controls (removed in the gauge's dtor before Re:CoM frees anything).
            unsafe { u.clear() };
        }
        LIVE.store(false, Ordering::Release);
        note(|| "combat is not KH1 here: menu removed".to_string());
    }
}

impl Ui {
    unsafe fn clear(&mut self) {
        // The ctrl's dtor releases both layers; the text layer's object deletes its `C2DFnt`
        // (`crate::d2`), as `CStockFolder`'s labels go: the font is not deleted again here.
        for r in self.rows.drain(..) {
            super::d2::delete(r.ctrl, self.list);
        }
        self.shown = None;
    }

    unsafe fn make_row(&self, set: *const u8, first: u32, v: &RowView, sc: &Scene, words: &Words) -> Option<Row> {
        let c = func::<NewFn>(NEW)(0x170);
        if c == 0 {
            return None;
        }
        func::<CtorFn>(SEQ_CTOR)(c);
        if rd::<usize>(c) != base() + SEQ_CTRL_VTABLE as usize {
            return None;
        }
        super::d2::made();
        wr::<u8>(c + C_PRIORITY, if v.list == Kind::Magic { PRIORITY_SUB } else { PRIORITY_ROOT });
        wr::<u8>(c + C_COLOUR_ON, 1);
        place(c, v, sc);
        let g = first + if v.highlight { 1 } else { 0 };
        super::d2::add(c, self.list, set, g, self.xoff, 1);
        let mut fnt = 0;
        if let Some(text) = (v.name != 0).then(|| words.get(v.name)).flatten() {
            let f = func::<NewFn>(NEW)(0xB0);
            if f != 0 {
                let colour = cm::text_colour(v.bright, sc.alpha);
                fnt = func::<FntCtorFn>(FNT_CTOR)(f, 0, 0, &colour, FONT, text.as_ptr(), FONT_FLAGS);
                super::d2::attach(c, self.list, set, fnt, 0.0, 0.0, TEXT_DX + self.xoff);
            }
        }
        func::<VecFn>(COLOUR)(c, V4([1.0, 1.0, 1.0, sc.alpha]).0.as_ptr());
        Some(Row { ctrl: c, fnt, view: *v })
    }

    /// Shows `sc`: rebuilds the rows when what they show changed, else writes colours and the
    /// tab's place.
    unsafe fn show(&mut self, sc: &Scene, set: *const u8, first: u32, words: &Words) {
        if self.hidden {
            return;
        }
        let same = self.shown.as_ref().is_some_and(|s| s.same_rows(sc));
        if !same {
            self.clear();
            for v in &sc.rows {
                if let Some(r) = self.make_row(set, first, v, sc, words) {
                    self.rows.push(r);
                }
            }
            self.shown = Some(sc.clone());
            let n = REBUILDS.fetch_add(1, Ordering::Relaxed) + 1;
            if n.is_multiple_of(100) {
                super::d2::census(|| format!("command menu: {n} row rebuilds"));
            }
            return;
        }
        let old = self.shown.as_ref().expect("same");
        let alpha_changed = old.alpha != sc.alpha;
        let tab_moved = old.tab_x != sc.tab_x || old.tab_scale != sc.tab_scale;
        for (r, v) in self.rows.iter_mut().zip(&sc.rows) {
            if alpha_changed {
                func::<VecFn>(COLOUR)(r.ctrl, V4([1.0, 1.0, 1.0, sc.alpha]).0.as_ptr());
            }
            if r.fnt != 0 && (alpha_changed || r.view.bright != v.bright) {
                wr::<u32>(r.fnt + F_COLOUR, cm::text_colour(v.bright, sc.alpha));
            }
            if v.tab && tab_moved {
                place(r.ctrl, v, sc);
            }
            r.view = *v;
        }
        self.shown = Some(sc.clone());
    }
}

/// Re:CoM's layout position (and X scale for the tab) of a row: KH1's top-left + the plate's
/// anchor, the tab's x offset and scale.
unsafe fn place(c: usize, v: &RowView, sc: &Scene) {
    let (dx, sx) = if v.tab { (sc.tab_x, sc.tab_scale) } else { (0.0, 1.0) };
    func::<VecFn>(LAYOUT_POS)(c, V4([v.x + cm::PLATE_ANCHOR.0 + dx, v.y + cm::PLATE_ANCHOR.1, 0.0, 1.0]).0.as_ptr());
    func::<VecFn>(SCALE)(c, V4([sx, 1.0, 1.0, 1.0]).0.as_ptr());
}

/// Re:CoM's pad, this frame's fresh presses (pure reads), when Sora takes input.
unsafe fn read_pad(pl: usize) -> Pad {
    if rd::<u32>(pl + p::PAD) & 1 == 0 {
        return Pad::default();
    }
    let fresh = func::<PadFn>(PAD_FRESH);
    let btn = |a: u32| func::<PadBtnFn>(a)(0) & 0xFF != 0;
    Pad {
        up: fresh(0x1000, 0) != 0,
        down: fresh(0x4000, 0) != 0,
        left: fresh(0x8000, 0) != 0,
        right: fresh(0x2000, 0) != 0,
        confirm: btn(PAD_CONFIRM),
        cancel: btn(PAD_CANCEL),
        triangle: fresh(0x10, 0) != 0,
        square: fresh(0x80, 0) != 0,
        // L2, R2, L1, R1, Select.
        other: fresh(0x10F, 0) != 0,
    }
}

/// Re:CoM's field action under Sora (as `0x1401E80C0` finds it), as KH1's context command:
/// a treasure box not yet opened → Open, a carry gimmick → Lift, an NPC (`ATEL::CAvatar`) → Talk,
/// a door → Open, a jump stand → Jump On. Pure reads (the type getters are `mov eax, imm32; ret`,
/// the door's is recognised by its address). The words for Re:CoM's objects: Assumption.
unsafe fn context(pl: usize) -> u8 {
    if rd::<i32>(pl + p::BATTLE) != 0 {
        return cm::cmd::NONE;
    }
    let mut h: i32 = rd(pl + ACTION_TARGET);
    if h <= 0 {
        h = rd(pl + p::LOCK_TARGET);
    }
    if h <= 0 {
        return cm::cmd::NONE;
    }
    let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h as u32);
    if obj == 0 {
        return cm::cmd::NONE;
    }
    let getter: usize = rd(rd::<usize>(obj) + 0x28);
    let kind = if getter == base() + DOOR_TYPE as usize {
        0x1006
    } else if rd::<u8>(getter) == 0xB8 && rd::<u8>(getter + 5) == 0xC3 {
        rd::<u32>(getter + 1)
    } else {
        0
    };
    match kind {
        0x2000 if rd::<u8>(obj + BOX_OPENED) & 1 == 0 => cm::cmd::OPEN,
        0x1006 => cm::cmd::OPEN,
        0x2002 => cm::cmd::JUMP_ON,
        0x2003 => cm::cmd::LIFT,
        0x4000 => cm::cmd::TALK,
        _ => cm::cmd::NONE,
    }
}

/// The spells Sora has: the debug switch, else the save block's levels (stage 7.11 writes them).
fn spells() -> [u8; 7] {
    if ALL_SPELLS.load(Ordering::Relaxed) {
        [3; 7]
    } else {
        super::mp::spell_levels()
    }
}

unsafe fn world(pl: usize) -> World {
    let mut w = World { mp: mp_view(), spells: spells(), ..Default::default() };
    let mut in_recoil = false;
    if let Ok(g) = mv::SORA.try_lock() {
        if let Some(s) = g.as_ref().filter(|s| s.player == pl) {
            w.state = s.sora.state;
            in_recoil = s.sora.abilities.counterattack && s.sora.clashed && s.sora.rec.head() == crate::rows::RECOIL;
            // KH1's commit gate for spells (`0x140293480`), on the KH1 frame as last left, and
            // only while Re:CoM's action is one KH1's frame runs (M7 stage 7.4). One update
            // stale: the frame itself checks again (`Kh1Sora::accepts_spell`).
            if mv::sync_ok(pl, s) {
                w.gate_any = s.sora.accepts_spell(crate::magic::spell::BLIZZARD);
                w.gate_fire = s.sora.accepts_spell(crate::magic::spell::FIRE);
            }
        }
    }
    let target = if in_recoil || (0..7).any(|i| cm::spell::RANGED[i] && w.spells[i] > 0) { super::attack::attack_target(pl) } else { None };
    w.counter = in_recoil && target.is_some();
    if let Some(t) = target {
        let b: usize = rd(pl + p::BODY);
        if b != 0 {
            w.target_dist = Some((to_kh1(rd_vec(b + body::POS)) - t.centre).length());
        }
    }
    w.context = context(pl);
    w
}

/// Re:CoM's request bits the menu lets through this frame: (Attack, jump, Triangle).
pub struct Keep {
    pub attack: bool,
    pub jump: bool,
    pub triangle: bool,
}

/// KH1's menu step for this frame of Sora's (from the request-bits wrapper, after Re:CoM's
/// builder). None when there is no menu (nothing changes).
pub unsafe fn frame(pl: usize) -> Option<Keep> {
    if !LIVE.load(Ordering::Acquire) {
        return None;
    }
    let (set, first) = super::mp_bar::menu_groups()?;
    let words = WORDS.get()?;
    if !super::mp::active() {
        return None;
    }
    let pad = read_pad(pl);
    let w = world(pl);
    let step = (rd::<i32>(pl + p::DT).max(0) as f32) / 50.0;
    let (out, token_events) = {
        let mut s = STATE.lock().ok()?;
        let State { menu, uis } = &mut *s;
        let m = menu.as_mut()?;
        if uis.iter().all(|u| u.hidden) {
            return None;
        }
        let out = m.step(&w, pad);
        m.tick(step);
        let sc = m.scene();
        for u in uis.iter_mut() {
            u.show(&sc, set, first, words);
        }
        let ev: Vec<String> = out.events.iter().map(|e| describe(e, m)).collect();
        (out, ev)
    };
    for snd in &out.sounds {
        func::<SoundFn>(SOUND)(snd.recom_id(), -1);
    }
    for (e, d) in out.events.iter().zip(&token_events) {
        event(d.clone());
        match e {
            Event::Spell(c, _) => note(|| format!("{d} [{c:#x}] greyed (Re:CoM's refused sound)")),
            Event::NotNow(c) => note(|| format!("{d} [{c:#x}]: KH1's gate refused it this frame (Sora busy)")),
            Event::Cast(c) => note(|| format!("{d} [{c:#x}]: issued")),
            _ => note(|| d.clone()),
        }
    }
    // The spell issued (M7 stage 7.4): KH1's magic command for this frame's KH1 frame.
    if let Some((s, lv)) = out.cast.and_then(cm::cmd::spell_of) {
        super::magic::issue(s, lv);
    }
    Some(Keep { attack: out.attack, jump: out.jump, triangle: out.triangle })
}

fn describe(e: &Event, m: &Menu) -> String {
    let name = |c: u8| {
        WORDS.get().and_then(|w| w.get(cm::cmd::name(c))).map(|b| String::from_utf8_lossy(&b[..b.len().saturating_sub(1)]).into_owned()).unwrap_or_else(|| format!("{c:#x}"))
    };
    match *e {
        Event::Opened(_) => format!("open magic ({} spells, grey {:#x})", m.magic.cmds.len(), m.magic.grey & 0x7F),
        Event::Closed => "close".into(),
        Event::Spell(c, _) => format!("spell {}", name(c)),
        Event::Cast(c) => format!("cast {}", name(c)),
        Event::NotNow(c) => format!("not-now {}", name(c)),
        Event::Refused(c) => format!("refused {}", if c == 0 { "-".into() } else { name(c) }),
        Event::Context(c) => format!("context {}", name(c)),
    }
}

/// The F9 trace's menu token: list and cursor, greys, the tab, this frame's events.
pub fn trace_token() -> String {
    let ev = EVENTS.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
    match STATE.try_lock().ok().and_then(|s| s.menu.clone()) {
        Some(m) if LIVE.load(Ordering::Relaxed) => m.trace_token(&ev),
        _ => "menu -".to_string(),
    }
}

/// Clears the trace's events (a new recording starts).
pub fn clear_events() {
    if let Ok(mut g) = EVENTS.lock() {
        g.clear();
    }
}
