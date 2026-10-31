//! KH1's command menu (M7 stage 7.3, decision 34): the testable part.
//!
//! KH1 (Steam, Checked (decomp) unless tagged; research `m7-menus-mp.md` § 1, kept with the
//! decomp tools):
//! - **Lists** are 0x38-byte records (`0x140528900` + type × 0x38): `+4` grey mask (drawn grey),
//!   `+8` lock mask (can't run), `+0xC` cursor, `+0x10` count, `+0x14` command ids, `+0x24`
//!   per-row brightness, `+0x34` the row a submenu was opened from. Root (type 0, 4 rows: Attack /
//!   Magic / Items / context) and Magic (type 1) are this stage's.
//! - **Build, every frame** (`0x140294220`, `0x1402944C0`, `0x1402948F0`): row 0 Attack (1), or
//!   Counterattack (0x63) while the guard recoil 0x6E plays against a target with the ability;
//!   grey = lock = `!(state flags >> 4) | !1` (Attack needs the movement state's flag 0x10), rows
//!   1..3 the blank command 0x42. With spells learned: row 1 Magic (2), not locked, greyed when no
//!   spell is usable (usable: the state's flag 0x20, MP `0x1402A4C10`, and for the command flag
//!   8 spells the target within the command's range `+0xE`); the Magic list holds the learned
//!   spells (`level + (spell + 2) × 3`), grey = lock for the unusable ones. Row 2 Items (3),
//!   greyed and locked with no item slots. Row 3 the context command (0 = none), ungreyed when
//!   one is set (`0x140293000`).
//! - **Cursor** (`0x1402931B0`), on the fresh-press word compared for equality: Up / Down move and
//!   wrap; Right opens the row's submenu (sound 6), a locked row sounds 4; Left or cancel closes a
//!   submenu (sound 3; cancel also puts the root cursor on Attack); Left at the root puts the
//!   cursor on Attack (sound 1). No auto-repeat.
//! - **Step** (`0x1402937F0`): at the root a fresh jump is the jump (no menu input that frame);
//!   Triangle runs the context row when its tab is fully in (`0x140277CF0` state 2); Square
//!   (without Triangle) is the roll / guard (no cursor that frame); then the cursor; then confirm
//!   (and Right on the root's row 3) runs the row (`0x140293410` → `0x140293480`): a locked row
//!   sounds 4, a row with a submenu opens it, a command is issued and the root cursor returns to
//!   Attack. Confirm on the root's row 3 is the error sound (only Summon is opened from there).
//! - **Draw** (`0x14027B870`): root rows at (16, 260), 22 apart; the Magic list at x + 11, at the
//!   Magic row's y when it has fewer than 3 rows, else bottom-anchored at 348 − 22 n. The top
//!   list's rows ease ±32 brightness a frame between 0x73 (grey) and 0xE6; when the top list
//!   changes, every row is set at once (the lists under it to 0x73). The menu fades in / out at
//!   0.1 a frame (`0x140278AC0`) and hides / shows at once for events (`0x140293750`).
//! - **Context tab** (`0x140277CF0`): row 3 slides in over 14 frames (X scale +1/7 and x −10 → 0
//!   by 10/7 per half-frame step), out by the scale.
//!
//! Re:CoM side: `win/command_menu.rs`; notes `docs/recom-m7-command-menu.md`.

use crate::json;

/// The words manifest's format line (`kh1-recom-assets` `command_menu::manifest_json`).
pub const FORMAT: &str = "kh1-recom-assets command-menu 1";

/// KH1 command ids (the command table, `btltbl.bin` `+0xA58`; Checked (files)).
pub mod cmd {
    /// No command (the empty context row).
    pub const NONE: u8 = 0;
    pub const ATTACK: u8 = 1;
    pub const MAGIC: u8 = 2;
    pub const ITEMS: u8 = 3;
    pub const TALK: u8 = 4;
    pub const LIFT: u8 = 0x1C;
    pub const OPEN: u8 = 0x37;
    /// The blank command rows 1..3 hold until filled (message 305, a dash).
    pub const BLANK: u8 = 0x42;
    pub const JUMP_ON: u8 = 0x45;
    pub const COUNTER: u8 = 0x63;
    /// Treated as "no context command" by the tab (`0x140277CF0`).
    pub const NOT_CONTEXT: u8 = 0x7C;

    /// A spell's command: `level + (spell + 2) × 3` (spell 0..6, level 1..3).
    pub const fn spell(spell: u8, level: u8) -> u8 {
        6 + 3 * spell + level
    }

    /// (spell, level) of a spell command.
    pub fn spell_of(c: u8) -> Option<(u8, u8)> {
        (7..=27).contains(&c).then(|| ((c - 7) / 3, (c - 7) % 3 + 1))
    }

    /// The command's name (its record's `+4` message id).
    pub fn name(c: u8) -> u16 {
        match c {
            LIFT => 28,
            OPEN => 294,
            BLANK => 305,
            JUMP_ON => 313,
            COUNTER => 343,
            c => c as u16,
        }
    }

    /// The list a command opens (record `+8`): Magic 1, Items 2 (Cure / Aero open the party list
    /// 4: with the casts, stage 7.4 / 7.7).
    pub fn submenu(c: u8) -> u8 {
        match c {
            MAGIC => 1,
            ITEMS => 2,
            _ => 0,
        }
    }

    /// The commands KH1 puts in row 0 instead of Attack (`0x62C0183` over 0x46.., and Talk).
    pub fn row0_special(c: u8) -> bool {
        c == TALK || (c.wrapping_sub(0x46) < 0x1B && (0x062C_0183u32 >> (c - 0x46)) & 1 != 0)
    }
}

/// The spells, in KH1's order: Fire, Blizzard, Thunder, Cure, Gravity, Stop, Aero.
pub mod spell {
    pub const COUNT: usize = 7;
    /// MP costs (spell data `+0x30`; `crate::mp::cost`). Checked (files).
    pub const COSTS: [i32; COUNT] = [crate::mp::cost::FIRE, crate::mp::cost::BLIZZARD, crate::mp::cost::THUNDER, crate::mp::cost::CURE, crate::mp::cost::GRAVITY, crate::mp::cost::STOP, crate::mp::cost::AERO];
    /// Command flag 8 (greyed when the target is beyond the range): Thunder, Gravity, Stop.
    /// Checked (files: flags 0xA / 0xE / 0xA).
    pub const RANGED: [bool; COUNT] = [false, false, true, false, true, true, false];

    /// The command's range `+0xE` (KH1 cm). Checked (files).
    pub fn range(spell: u8, level: u8) -> f32 {
        match (spell, level) {
            (1, 1) => 700.0,
            (1, 2) => 900.0,
            (0 | 1, _) => 1200.0,
            (2 | 4 | 5, _) => 1100.0,
            _ => 0.0,
        }
    }
}

/// KH1's movement-state flags (`0x142DB3840 + state × 0x40`, registrations read; research
/// `kh1-movement-motion.md`): 0x10 Attack, 0x20 magic, 0x40 items. Checked (decomp). States not
/// in that table: 0.
pub fn state_flags(state: u32) -> u32 {
    match state {
        0 => 0xFC,
        1 => 0x34,
        2 => 0x3D,
        3 => 0x05,
        4 | 5 => 0x02,
        7 => 0x01,
        8 => 0x76,
        9..=11 => 0x101,
        0xD | 0xE | 0xF | 0x11 => 0x01,
        0x15 => 0x05,
        0x1E => 0x18,
        _ => 0,
    }
}

/// What KH1's build reads, from Re:CoM's side.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct World {
    /// Sora's KH1 movement state.
    pub state: u32,
    /// The Counterattack is offered (row 0 becomes Counter).
    pub counter: bool,
    /// Spell levels, 0 = not learned.
    pub spells: [u8; spell::COUNT],
    /// KH1's MP (MP, max, charge), None when MP isn't live.
    pub mp: Option<(i32, i32, u16)>,
    /// Distance to the target (KH1 cm), when there is one.
    pub target_dist: Option<f32>,
    /// The context command for row 3 (0: none).
    pub context: u8,
}

/// The fresh presses of one frame (Re:CoM's pad, mapped to KH1's buttons).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pad {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// Re:CoM's confirm button (KH1's Cross: run the row).
    pub confirm: bool,
    /// Re:CoM's cancel button (KH1's Circle: back, and the jump).
    pub cancel: bool,
    pub triangle: bool,
    pub square: bool,
    /// Any other button (L1, R1, …): makes the word differ from a lone direction.
    pub other: bool,
}

impl Pad {
    /// KH1's logical fresh word (`0x14028B0B0`'s table `0x140528290`; Checked (decomp + exe)).
    pub fn word(&self) -> u32 {
        let mut w = 0;
        for (on, bits) in [(self.up, 0x20), (self.down, 0x40), (self.left, 0x80), (self.right, 0x100), (self.confirm, 0x21_0001), (self.cancel, 6), (self.triangle, 0x408_0008), (self.square, 0x90_0010), (self.other, 0x4_0000)] {
            if on {
                w |= bits;
            }
        }
        w
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Root,
    Magic,
}

/// One KH1 list.
#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub kind: Kind,
    pub cmds: Vec<u8>,
    pub grey: u32,
    pub lock: u32,
    pub cursor: i32,
    /// Per-row text brightness (0x73 grey … 0xE6), eased by the draw.
    pub bright: Vec<f32>,
}

impl List {
    fn new(kind: Kind) -> List {
        List { kind, cmds: Vec::new(), grey: !0, lock: !0, cursor: 0, bright: Vec::new() }
    }
    fn count(&self) -> i32 {
        self.cmds.len() as i32
    }
    fn locked(&self, row: i32) -> bool {
        (0..32).contains(&row) && self.lock >> row & 1 != 0
    }
    fn grey_row(&self, row: usize) -> bool {
        row < 32 && self.grey >> row & 1 != 0
    }
}

/// KH1's menu sounds (`0x14017A040`): 1 move, 3 back, 4 error, 6 open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Move,
    Back,
    Error,
    Open,
}

impl Sound {
    /// Re:CoM's menu sound for it (`0x1402AA360(id, -1)`): 1 cursor, 4 cancel, 5 refused, 2
    /// decide. Assumption (ids from how Re:CoM's camp uses them: 1 on a move, 2 on a confirm, 4
    /// on a close, 5 when a confirm is refused).
    pub fn recom_id(self) -> u32 {
        match self {
            Sound::Move => 1,
            Sound::Back => 4,
            Sound::Error => 5,
            Sound::Open => 2,
        }
    }
}

/// What happened, for the log and the trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Opened(Kind),
    Closed,
    /// A spell row was chosen: the cast comes with stage 7.4. (command, locked)
    Spell(u8, bool),
    /// A locked row, or a row with nothing to run.
    Refused(u8),
    /// The context command went to Re:CoM's Triangle action.
    Context(u8),
}

/// Re:CoM's request bits after the menu: which of the pad's actions stay.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    /// Re:CoM's Attack request (`+0x424 & 4`) may stay: the confirm was the Attack row's.
    pub attack: bool,
    /// The jump (`& 1`) may stay: KH1 jumps only from the root list.
    pub jump: bool,
    /// Re:CoM's Triangle action (`& 0x200`) may stay. KH1 runs the context row with Triangle only
    /// from the root list and once its tab is in; with no context command shown, Re:CoM's own
    /// action is left to decide (an object this menu doesn't recognise keeps its Triangle).
    pub triangle: bool,
    pub sounds: Vec<Sound>,
    pub events: Vec<Event>,
}

/// The context tab (`0x140277CF0`): 0 hidden, 1 sliding in, 2 in, 3 sliding out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tab {
    pub state: u8,
    /// x offset (−10 → 0) and X scale (0 → 1).
    pub x: f32,
    pub scale: f32,
    /// The command it shows (`0x142866D10`).
    pub last: u8,
}

impl Default for Tab {
    fn default() -> Self {
        Tab { state: 0, x: -10.0, scale: 0.0, last: 0 }
    }
}

/// KH1's numbers for the brightness, the fade and the tab. Checked (decomp).
pub const BRIGHT_GREY: f32 = 115.0; // 0x73
pub const BRIGHT_ON: f32 = 230.0; // 0xE6
pub const BRIGHT_STEP: f32 = 32.0;
pub const FADE_STEP: f32 = 0.1;
/// The tab's step is half a frame step (`0x14233FBE0`; research § 1.2). Checked (decomp, as read
/// in the research); that it is half the frame step on PC: Assumption.
pub const TAB_STEP: f32 = 0.5;

/// The menu.
#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub root: List,
    pub magic: List,
    /// The Magic list is open (the stack is root → Magic).
    pub open: bool,
    /// The fade (`0x14285280C`) and its speed.
    pub alpha: f32,
    pub alpha_speed: f32,
    /// The list type last drawn on top (`0x142852790`).
    top: Option<Kind>,
    pub tab: Tab,
    /// The name the tab last showed (it keeps it while sliding out).
    pub tab_cmd: u8,
}

impl Default for Menu {
    fn default() -> Self {
        Menu::new()
    }
}

impl Menu {
    pub fn new() -> Menu {
        let mut root = List::new(Kind::Root);
        root.cmds = vec![cmd::ATTACK, cmd::BLANK, cmd::BLANK, cmd::NONE];
        Menu { root, magic: List::new(Kind::Magic), open: false, alpha: 0.0, alpha_speed: 0.0, top: None, tab: Tab::default(), tab_cmd: 0 }
    }

    /// The menu appears (the player's HUD is built): KH1's fade-in (`0x140278AC0(1)`).
    pub fn fade_in(&mut self) {
        self.alpha = 0.0;
        self.alpha_speed = FADE_STEP;
    }

    /// Re:CoM's HUD hides (events): KH1's instant hide, and the stack back to the root with the
    /// cursor on Attack when it shows again (`0x1402937F0` with `0x142D53A4C & 8`).
    pub fn hide(&mut self) {
        self.alpha = 0.0;
        self.alpha_speed = 0.0;
        self.open = false;
        self.root.cursor = 0;
        self.tab = Tab::default();
    }

    /// Re:CoM's HUD shows again: KH1's instant show (`0x140278AC0(2)`).
    pub fn show(&mut self) {
        self.alpha = 1.0;
        self.alpha_speed = 0.0;
    }

    /// KH1's per-frame build of the root and Magic lists.
    pub fn build(&mut self, w: &World) {
        let flags = state_flags(w.state);
        let r = &mut self.root;
        r.cmds = vec![if w.counter { cmd::COUNTER } else { cmd::ATTACK }, cmd::BLANK, cmd::BLANK, cmd::BLANK];
        r.grey = !(flags >> 4) | 0xFFFF_FFFE;
        r.lock = r.grey;
        // Magic (`0x1402944C0`).
        let learned: Vec<(u8, u8)> = (0..spell::COUNT as u8).filter_map(|s| Some((s, w.spells[s as usize])).filter(|x| (1..=3).contains(&x.1))).collect();
        let mut usable = !0u32;
        if flags & 0x20 != 0 {
            for (i, &(s, lv)) in learned.iter().enumerate() {
                let pays = w.mp.is_some_and(|(mp, max, c)| crate::mp::Mp::new(mp, max, c).can_pay(spell::COSTS[s as usize]));
                let in_range = !spell::RANGED[s as usize] || w.target_dist.is_none_or(|d| d <= spell::range(s, lv));
                if pays && in_range {
                    usable &= !(1 << i);
                }
            }
        }
        self.magic.cmds = learned.iter().map(|&(s, lv)| cmd::spell(s, lv)).collect();
        self.magic.grey = usable;
        self.magic.lock = usable;
        if !learned.is_empty() {
            r.grey &= !2;
            r.lock &= !2;
            r.cmds[1] = cmd::MAGIC;
            if usable == !0 {
                r.grey |= 2;
            }
        }
        // Items (M8): no item slots yet, greyed and locked.
        r.cmds[2] = cmd::ITEMS;
        r.grey |= 4;
        r.lock |= 4;
        // The context row (`0x1402948F0` clears it, `0x140293000` sets it).
        r.cmds[3] = cmd::NONE;
        if w.context != cmd::NONE {
            r.cmds[3] = w.context;
            r.grey &= !8;
            r.lock &= !8;
        }
        for l in [&mut self.root, &mut self.magic] {
            l.bright.resize(l.cmds.len(), BRIGHT_GREY);
        }
    }

    fn row3_valid(&self) -> bool {
        let c = self.root.cmds[3];
        c != cmd::NONE && c != cmd::BLANK && c != cmd::NOT_CONTEXT && !self.root.grey_row(3)
    }

    fn row0_offers(&self) -> bool {
        cmd::row0_special(self.root.cmds[0]) && !self.root.grey_row(0)
    }

    /// One frame of KH1's menu step for this pad: the build, then the input.
    pub fn step(&mut self, w: &World, pad: Pad) -> Outcome {
        self.build(w);
        // The stack (`0x1402931B0`'s walk): the Magic list stays open while its row can open it.
        if self.open && (self.root.locked(1) || self.root.cmds[1] != cmd::MAGIC || self.magic.cmds.is_empty()) {
            self.open = false;
        }
        if self.open && self.magic.cursor >= self.magic.count() {
            self.magic.cursor = self.magic.count() - 1;
        }
        let was_open = self.open;
        let on_attack = !was_open && self.root.cursor == 0;
        let offered = self.row3_valid() || self.row0_offers();
        let mut out = Outcome { attack: on_attack, jump: !was_open, triangle: !was_open && (!offered || self.tab.state == 2), ..Default::default() };
        let word = pad.word();
        // At the root a fresh jump is the jump: no menu input this frame.
        if !was_open && pad.cancel {
            return out;
        }
        // Triangle: the context row, when its tab is in.
        if !was_open && pad.triangle && self.tab.state == 2 && offered {
            let c = if self.row3_valid() { self.root.cmds[3] } else { self.root.cmds[0] };
            out.events.push(Event::Context(c));
            self.root.cursor = 0;
        }
        // Square without Triangle: the roll / guard, no cursor this frame.
        if word & 0x400_0010 == 0x10 {
            return out;
        }
        // The cursor.
        let consumed = self.cursor(word, &mut out);
        if consumed {
            out.attack = false;
            return out;
        }
        // Run the row: confirm, or Right on the root's row 3.
        let at3 = !self.open && self.root.cursor == 3;
        let run = word & (0x1_0000 | if at3 { 0x100 } else { 0 }) != 0;
        if !run {
            return out;
        }
        if !self.open {
            match self.root.cursor {
                0 => {
                    if cmd::row0_special(self.root.cmds[0]) {
                        out.sounds.push(Sound::Error);
                        out.attack = false;
                    }
                    // Attack / Counter: Re:CoM's own Attack request, as before the menu.
                }
                3 => {
                    out.sounds.push(Sound::Error);
                    out.events.push(Event::Refused(self.root.cmds[3]));
                }
                row => {
                    out.attack = false;
                    self.run_root(row, &mut out);
                }
            }
        } else {
            out.attack = false;
            let row = self.magic.cursor;
            let c = self.magic.cmds.get(row as usize).copied().unwrap_or(0);
            let locked = self.magic.locked(row);
            // Stage 7.3: no cast yet (7.4 hooks the spell to Re:CoM's magic action).
            out.sounds.push(Sound::Error);
            out.events.push(Event::Spell(c, locked));
        }
        out
    }

    fn run_root(&mut self, row: i32, out: &mut Outcome) {
        let c = self.root.cmds[row as usize];
        if self.root.locked(row) {
            out.sounds.push(Sound::Error);
            out.events.push(Event::Refused(c));
        } else if cmd::submenu(c) == 1 {
            self.open_magic(out);
        } else {
            out.events.push(Event::Refused(c));
        }
    }

    fn open_magic(&mut self, out: &mut Outcome) {
        self.open = true;
        out.sounds.push(Sound::Open);
        out.events.push(Event::Opened(Kind::Magic));
        if self.magic.cursor >= self.magic.count() || self.magic.cursor < 0 {
            self.magic.cursor = 0;
        }
    }

    /// `0x1402931B0`: true when the word was the cursor's.
    fn cursor(&mut self, word: u32, out: &mut Outcome) -> bool {
        let open = self.open;
        let (count, cur) = if open { (self.magic.count(), self.magic.cursor) } else { (4, self.root.cursor) };
        if word == 0x100 {
            if open || cur != 3 {
                if open {
                    // Spells open no list (Cure / Aero's party list: 7.4 / 7.7).
                    if self.magic.locked(cur) {
                        out.sounds.push(Sound::Error);
                    }
                } else if self.root.locked(cur) {
                    out.sounds.push(Sound::Error);
                } else if cmd::submenu(self.root.cmds[cur as usize]) == 1 {
                    self.open_magic(out);
                }
                return true;
            }
            return false;
        }
        if word == 0x80 || word & 2 != 0 {
            if open {
                self.open = false;
                if word & 2 != 0 && self.root.cursor >= 0 {
                    self.root.cursor = 0;
                }
                out.sounds.push(Sound::Back);
                out.events.push(Event::Closed);
                return true;
            }
            if word & 0x80 != 0 {
                self.root.cursor = 0;
                out.sounds.push(Sound::Move);
                return true;
            }
            return false;
        }
        let next = match word {
            0x20 => Some(if cur < 1 { count - 1 } else { cur - 1 }),
            0x40 => Some(if cur >= count - 1 { 0 } else { cur + 1 }),
            _ => None,
        };
        let Some(n) = next else { return false };
        if open {
            self.magic.cursor = n;
        } else {
            self.root.cursor = n;
        }
        out.sounds.push(Sound::Move);
        true
    }

    /// The draw's per-frame part: the fade, the rows' brightness, the tab. `step` = frames (1.0
    /// at 60 fps).
    pub fn tick(&mut self, step: f32) {
        if self.alpha_speed != 0.0 {
            self.alpha = (self.alpha + step * self.alpha_speed).clamp(0.0, 1.0);
        }
        let top = if self.open { Kind::Magic } else { Kind::Root };
        if self.top != Some(top) {
            self.top = Some(top);
            let (t, under) = if self.open { (&mut self.magic, Some(&mut self.root)) } else { (&mut self.root, None) };
            for (i, b) in t.bright.iter_mut().enumerate() {
                *b = if i < 32 && t.grey >> i & 1 != 0 { BRIGHT_GREY } else { BRIGHT_ON };
            }
            if let Some(u) = under {
                u.bright.iter_mut().for_each(|b| *b = BRIGHT_GREY);
            }
        } else {
            let t = if self.open { &mut self.magic } else { &mut self.root };
            for (i, b) in t.bright.iter_mut().enumerate() {
                *b = if i < 32 && t.grey >> i & 1 != 0 { (*b - step * BRIGHT_STEP).trunc().max(BRIGHT_GREY) } else { (*b + step * BRIGHT_STEP).trunc().min(BRIGHT_ON) };
            }
        }
        self.tab_step(step * TAB_STEP);
    }

    fn tab_restart(&mut self, c: u8) {
        self.tab = Tab { state: 1, x: -10.0, scale: 0.0, last: c };
        self.tab_cmd = c;
    }

    /// `0x140277CF0`.
    fn tab_step(&mut self, half: f32) {
        if self.alpha == 0.0 {
            self.tab = Tab::default();
            return;
        }
        let r3 = self.row3_valid();
        let gone = !r3 && !self.row0_offers();
        let c = if r3 { self.root.cmds[3] } else { self.root.cmds[0] };
        match self.tab.state {
            0 => {
                if !gone {
                    self.tab_restart(c);
                }
            }
            1 => {
                if gone {
                    self.tab.last = 0;
                    self.tab.state = 3;
                    return;
                }
                if c != self.tab.last && self.tab.last != 0 {
                    self.tab_restart(c);
                }
                if self.tab.scale < 1.0 {
                    self.tab.scale = (self.tab.scale + half * 0.142_857_15).min(1.0);
                }
                if self.tab.x > 0.0 {
                    return;
                }
                self.tab.x += half * 1.428_571_5;
                if self.tab.x <= 0.0 {
                    return;
                }
                self.tab = Tab { state: 2, x: 0.0, scale: 1.0, last: c };
                self.tab_cmd = c;
            }
            2 => {
                if gone {
                    self.tab.state = 3;
                    self.tab.last = 0;
                } else if c != self.tab.last && self.tab.last != 0 {
                    self.tab_restart(c);
                }
            }
            _ => {
                if gone {
                    self.tab.scale -= half * 0.142_857_15;
                    if self.tab.scale <= 0.0 {
                        self.tab = Tab::default();
                    }
                } else {
                    self.tab.last = 0;
                    self.tab.state = 1;
                }
            }
        }
    }

    /// What to draw.
    pub fn scene(&self) -> Scene {
        let mut rows = Vec::new();
        let top_root = !self.open;
        for i in 0..4 {
            let tab = i == 3;
            let cursor_here = top_root && self.root.cursor == i as i32;
            if tab && self.tab.state == 0 && !cursor_here {
                continue;
            }
            let c = if tab { if self.tab.state == 0 { cmd::NONE } else { self.tab_cmd } } else { self.root.cmds[i] };
            rows.push(RowView {
                list: Kind::Root,
                index: i as u8,
                x: ROOT_X,
                y: ROOT_Y + PITCH * i as f32,
                name: if c == cmd::NONE { 0 } else { cmd::name(c) },
                highlight: cursor_here,
                bright: self.root.bright.get(i).copied().unwrap_or(BRIGHT_GREY) as u8,
                tab: tab && self.tab.state != 0,
            });
        }
        if self.open {
            let n = self.magic.cmds.len();
            let y0 = if n < 3 { ROOT_Y + PITCH } else { BOTTOM - PITCH * n as f32 };
            for (i, &c) in self.magic.cmds.iter().enumerate() {
                rows.push(RowView {
                    list: Kind::Magic,
                    index: i as u8,
                    x: ROOT_X + SUB_DX,
                    y: y0 + PITCH * i as f32,
                    name: cmd::name(c),
                    highlight: self.magic.cursor == i as i32,
                    bright: self.magic.bright.get(i).copied().unwrap_or(BRIGHT_GREY) as u8,
                    tab: false,
                });
            }
        }
        Scene { rows, alpha: self.alpha, tab_x: self.tab.x, tab_scale: self.tab.scale }
    }

    /// The F9 trace token.
    pub fn trace_token(&self, events: &[String]) -> String {
        let mut t = if self.open { format!("menu m:{}", self.magic.cursor) } else { format!("menu r:{}", self.root.cursor) };
        t.push_str(&format!(" g{:x}", self.root.grey & 0xF));
        if self.open {
            t.push_str(&format!(" mg{:x}", self.magic.grey & ((1u32 << self.magic.cmds.len()) - 1)));
        }
        if self.tab.state != 0 {
            t.push_str(&format!(" tab{}:{:#x}", self.tab.state, self.tab_cmd));
        }
        if self.root.cmds[0] == cmd::COUNTER {
            t.push_str(" counter");
        }
        for e in events {
            t.push(' ');
            t.push_str(e);
        }
        t
    }
}

/// KH1's layout (`0x14027B870`), in the 512 × 384 2D space. Checked (decomp).
pub const ROOT_X: f32 = 16.0;
pub const ROOT_Y: f32 = 260.0;
pub const PITCH: f32 = 22.0;
pub const SUB_DX: f32 = 11.0;
pub const BOTTOM: f32 = 348.0;
/// Re:CoM's plate is drawn around its cap's centre: its left edge 13 units left, its top 13
/// up (parts (−13, −13)…(165, 12)). KH1's (x, y) is taken as the row's top-left. Assumption.
pub const PLATE_ANCHOR: (f32, f32) = (13.0, 13.0);

/// One row to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowView {
    pub list: Kind,
    pub index: u8,
    /// KH1's row position (top-left, 512 × 384).
    pub x: f32,
    pub y: f32,
    /// The KH1 message id of its text (0: none).
    pub name: u16,
    pub highlight: bool,
    pub bright: u8,
    /// The context tab (its x and X scale follow the tab).
    pub tab: bool,
}

impl RowView {
    /// What needs the row rebuilt when it changes (not the brightness).
    pub fn key(&self) -> (Kind, u8, i32, i32, u16, bool, bool) {
        (self.list, self.index, self.x as i32, self.y as i32, self.name, self.highlight, self.tab)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub rows: Vec<RowView>,
    pub alpha: f32,
    pub tab_x: f32,
    pub tab_scale: f32,
}

impl Scene {
    pub fn same_rows(&self, o: &Scene) -> bool {
        self.rows.len() == o.rows.len() && self.rows.iter().zip(&o.rows).all(|(a, b)| a.key() == b.key())
    }
}

/// A text colour for Re:CoM's `C2DFnt` (`+0x34`, 0xAABBGGRR, 0x80 = ×1): KH1's
/// `0x14027A920` passes (alpha × 128, b, b, b).
pub fn text_colour(bright: u8, alpha: f32) -> u32 {
    let a = (alpha.clamp(0.0, 1.0) * 128.0) as u32;
    let b = bright as u32;
    a << 24 | b << 16 | b << 8 | b
}

/// The words (`kh1-command-menu.json`) of one language, as Re:CoM's font bytes (NUL-terminated),
/// by message id.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Words {
    pub lang: String,
    pub by_id: Vec<(u16, Vec<u8>)>,
    /// Characters Re:CoM's font bytes don't carry, written as their base letters.
    pub folded: Vec<char>,
}

impl Words {
    pub fn get(&self, id: u16) -> Option<&[u8]> {
        self.by_id.iter().find(|x| x.0 == id).map(|x| x.1.as_slice())
    }
}

/// Text as Re:CoM's `C2DFnt` bytes: ASCII as it is (as our Config rows, seen in game); Re:CoM's
/// codes for accented letters are not known (its text bytes ≥ 0x80 start a 2-byte code,
/// `0x140258EB0`): they are written as their base letters. Returns the bytes and the folded
/// characters.
pub fn font_bytes(s: &str) -> (Vec<u8>, Vec<char>) {
    let mut b = Vec::with_capacity(s.len() + 1);
    let mut folded = Vec::new();
    for c in s.chars() {
        if c.is_ascii() && !c.is_ascii_control() {
            b.push(c as u8);
            continue;
        }
        let base = match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'Ö' => 'O',
            'Ä' => 'A',
            'Ü' => 'U',
            'ñ' => 'n',
            'ç' => 'c',
            _ => '?',
        };
        folded.push(c);
        b.push(base as u8);
    }
    b.push(0);
    (b, folded)
}

/// Reads `kh1-command-menu.json` for `lang` (us, uk, fr, gr, it, sp); ids missing there come
/// from `us`.
pub fn parse_words(text: &str, lang: &str) -> Result<Words, String> {
    let v = json::parse(text)?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(format!("not a \"{FORMAT}\" manifest"));
    }
    let words = v.get("words").ok_or("no \"words\"")?;
    let us = words.get("us").ok_or("no \"us\" words")?;
    let want = words.get(lang).unwrap_or(us);
    let mut out = Words { lang: if words.get(lang).is_some() { lang.to_string() } else { "us".into() }, ..Default::default() };
    let json::Value::Obj(base) = us else { return Err("bad \"us\" words".into()) };
    for (k, _) in base {
        let Ok(id) = k.parse::<u16>() else { continue };
        let Some(t) = want.get(k).or_else(|| us.get(k)).and_then(|x| x.as_str()) else { continue };
        let (b, f) = font_bytes(t);
        out.folded.extend(f);
        out.by_id.push((id, b));
    }
    if [1u16, 2, 3].iter().any(|id| out.get(*id).is_none()) {
        return Err("Attack / Magic / Items missing".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        World { state: 0, mp: Some((3, 3, 0)), ..Default::default() }
    }

    fn all_spells() -> [u8; 7] {
        [3; 7]
    }

    fn press(f: impl FnOnce(&mut Pad)) -> Pad {
        let mut p = Pad::default();
        f(&mut p);
        p
    }

    #[test]
    fn root_without_spells_greys_magic_as_kh1() {
        let mut m = Menu::new();
        m.build(&world());
        // Row 1 stays KH1's blank command (message 305), greyed and locked; Items greyed and
        // locked; no context.
        assert_eq!(m.root.cmds, vec![cmd::ATTACK, cmd::BLANK, cmd::ITEMS, cmd::NONE]);
        assert_eq!(m.root.grey & 0xF, 0b1110);
        assert_eq!(m.root.lock & 0xF, 0b1110);
        // Confirm on it: the error sound, no list.
        m.root.cursor = 1;
        let o = m.step(&world(), press(|p| p.confirm = true));
        assert!(!m.open && o.sounds == vec![Sound::Error] && !o.attack);
    }

    #[test]
    fn spells_by_level_and_greyed_when_mp_cant_pay() {
        let mut w = World { spells: all_spells(), mp: Some((0, 3, 29)), ..world() };
        let mut m = Menu::new();
        m.build(&w);
        assert_eq!(m.root.cmds[1], cmd::MAGIC);
        assert_eq!(m.magic.cmds, vec![9, 12, 15, 18, 21, 24, 27]);
        // 0 MP and 29 charge: nothing is payable; the Magic row greys but stays openable.
        assert_eq!(m.magic.grey & 0x7F, 0x7F);
        assert!(m.root.grey & 2 != 0 && m.root.lock & 2 == 0);
        // 30 charge: Fire and Blizzard (30 units) can pay.
        w.mp = Some((0, 3, 30));
        m.build(&w);
        assert_eq!(m.magic.grey & 0x7F, 0x7F & !0b11);
        assert!(m.root.grey & 2 == 0);
        // 2 MP: all but Stop and Aero (2 points need MP ≥ 2: yes) → all.
        w.mp = Some((2, 3, 0));
        m.build(&w);
        assert_eq!(m.magic.grey & 0x7F, 0);
        // Thunder, Gravity, Stop beyond their range (1100) grey.
        w.target_dist = Some(1150.0);
        m.build(&w);
        assert_eq!(m.magic.grey & 0x7F, 0b0110100);
        // In a state without flag 0x20 (glide 0x15): none.
        w.state = 0x15;
        m.build(&w);
        assert_eq!(m.magic.grey & 0x7F, 0x7F);
        assert!(m.root.grey & 1 != 0, "no Attack in the glide (flags 5)");
        // Names: Firaga, Thundaga; level 1 Fire.
        assert_eq!((cmd::name(9), cmd::name(15)), (9, 15));
        let mut one = [0; 7];
        one[0] = 1;
        m.build(&World { spells: one, ..world() });
        assert_eq!(m.magic.cmds, vec![7]);
        assert_eq!(cmd::spell_of(27), Some((6, 3)));
    }

    #[test]
    fn kh1_cursor_rules() {
        let w = World { spells: all_spells(), ..world() };
        let mut m = Menu::new();
        // Up from Attack wraps to the context row; Down back to Attack.
        let o = m.step(&w, press(|p| p.up = true));
        assert_eq!((m.root.cursor, o.sounds.clone()), (3, vec![Sound::Move]));
        assert!(!o.attack);
        m.step(&w, press(|p| p.down = true));
        assert_eq!(m.root.cursor, 0);
        // Two presses in one frame: nothing (the word is compared for equality).
        m.step(&w, press(|p| {
            p.down = true;
            p.up = true
        }));
        assert_eq!(m.root.cursor, 0);
        // Down to Magic, Right opens it (sound "open"), Down moves in it, Left closes it with
        // the root cursor kept on Magic.
        m.step(&w, press(|p| p.down = true));
        let o = m.step(&w, press(|p| p.right = true));
        assert!(m.open && o.sounds == vec![Sound::Open]);
        m.step(&w, press(|p| p.down = true));
        assert_eq!(m.magic.cursor, 1);
        // Up from the first spell wraps to the last.
        m.step(&w, press(|p| p.up = true));
        m.step(&w, press(|p| p.up = true));
        assert_eq!(m.magic.cursor, 6);
        let o = m.step(&w, press(|p| p.left = true));
        assert!(!m.open && o.sounds == vec![Sound::Back] && m.root.cursor == 1);
        // Confirm opens it too; cancel closes it and puts the root cursor on Attack, and is no
        // jump.
        m.step(&w, press(|p| p.confirm = true));
        assert!(m.open);
        let o = m.step(&w, press(|p| p.cancel = true));
        assert!(!m.open && m.root.cursor == 0 && !o.jump);
        // Left at the root: the cursor to Attack.
        m.root.cursor = 2;
        let o = m.step(&w, press(|p| p.left = true));
        assert!(m.root.cursor == 0 && o.sounds == vec![Sound::Move]);
        // The magic cursor is kept when the list opens again (KH1 keeps its list's cursor).
        m.magic.cursor = 4;
        m.root.cursor = 1;
        m.step(&w, press(|p| p.right = true));
        assert_eq!(m.magic.cursor, 4);
    }

    #[test]
    fn confirm_attacks_only_from_the_attack_row() {
        let w = World { spells: all_spells(), ..world() };
        let mut m = Menu::new();
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(o.attack && o.sounds.is_empty() && o.jump);
        // On Magic: no attack, the list opens.
        m.root.cursor = 1;
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(!o.attack && m.open);
        // In the list: a spell is chosen; 7.3 buzzes (the cast is 7.4), no attack, no jump.
        let o = m.step(&w, press(|p| p.confirm = true));
        assert_eq!(o.events, vec![Event::Spell(9, false)]);
        assert!(!o.attack && !o.jump && o.sounds == vec![Sound::Error] && m.open);
        // A greyed spell: buzz, flagged locked.
        let mut w2 = w;
        w2.mp = Some((0, 3, 0));
        let o = m.step(&w2, press(|p| p.confirm = true));
        assert_eq!(o.events, vec![Event::Spell(9, true)]);
        // On Items (locked): the error sound.
        let mut m = Menu::new();
        m.root.cursor = 2;
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(!o.attack && o.sounds == vec![Sound::Error] && !m.open);
        // Confirm on row 3: the error sound (only Summon opens from there).
        m.root.cursor = 3;
        let o = m.step(&w, press(|p| p.confirm = true));
        assert!(!o.attack && o.sounds == vec![Sound::Error]);
    }

    #[test]
    fn jump_and_square_skip_the_cursor() {
        let w = World { spells: all_spells(), ..world() };
        let mut m = Menu::new();
        // A jump at the root: no cursor move that frame.
        let o = m.step(&w, press(|p| {
            p.cancel = true;
            p.down = true
        }));
        assert!(o.jump && m.root.cursor == 0);
        // Square: the roll; no cursor.
        let o = m.step(&w, press(|p| {
            p.square = true;
            p.down = true
        }));
        assert!(m.root.cursor == 0 && o.sounds.is_empty());
    }

    #[test]
    fn the_magic_list_closes_when_its_row_locks() {
        let mut w = World { spells: all_spells(), ..world() };
        let mut m = Menu::new();
        m.root.cursor = 1;
        m.step(&w, press(|p| p.right = true));
        assert!(m.open);
        w.spells = [0; 7];
        m.step(&w, Pad::default());
        assert!(!m.open);
    }

    #[test]
    fn counterattack_replaces_attack() {
        let mut m = Menu::new();
        m.build(&World { counter: true, ..world() });
        assert_eq!(m.root.cmds[0], cmd::COUNTER);
        assert_eq!(cmd::name(cmd::COUNTER), 343);
        assert!(m.trace_token(&[]).contains("counter"));
    }

    #[test]
    fn brightness_eases_by_kh1s_steps_and_resets_on_a_new_top_list() {
        let w = World { spells: all_spells(), ..world() };
        let mut m = Menu::new();
        m.build(&w);
        m.tick(1.0);
        // First draw: set at once (Attack and Magic bright, Items grey, row 3 grey).
        assert_eq!(m.root.bright, vec![BRIGHT_ON, BRIGHT_ON, BRIGHT_GREY, BRIGHT_GREY]);
        // MP runs out: Magic greys over 4 frames (230 → 198 → 166 → 134 → 115).
        let dry = World { mp: Some((0, 3, 0)), ..w };
        let mut seen = Vec::new();
        for _ in 0..5 {
            m.build(&dry);
            m.tick(1.0);
            seen.push(m.root.bright[1]);
        }
        assert_eq!(seen, vec![198.0, 166.0, 134.0, 115.0, 115.0]);
        // Opening Magic: its rows at once, the root's all grey.
        m.build(&w);
        m.root.cursor = 1;
        m.step(&w, press(|p| p.right = true));
        m.tick(1.0);
        assert!(m.root.bright.iter().all(|&b| b == BRIGHT_GREY));
        assert!(m.magic.bright.iter().all(|&b| b == BRIGHT_ON));
    }

    #[test]
    fn fade_in_takes_ten_frames() {
        let mut m = Menu::new();
        m.fade_in();
        for _ in 0..9 {
            m.tick(1.0);
        }
        assert!(m.alpha < 1.0 && m.alpha > 0.85);
        m.tick(1.0);
        assert!((m.alpha - 1.0).abs() < 1e-5);
        m.hide();
        assert_eq!(m.alpha, 0.0);
        m.show();
        assert_eq!(m.alpha, 1.0);
        assert_eq!(text_colour(0xE6, 1.0), 0x80E6_E6E6);
        assert_eq!(text_colour(0x73, 0.5), 0x4073_7373);
    }

    #[test]
    fn context_tab_slides_in_over_14_frames_and_gates_triangle() {
        let w = World { context: cmd::OPEN, ..world() };
        let mut m = Menu::new();
        m.show();
        m.build(&w);
        let mut frames = 0;
        while m.tab.state != 2 {
            // Triangle before the tab is in: Re:CoM's action waits.
            let o = m.step(&w, press(|p| p.triangle = true));
            assert!(!o.triangle);
            m.tick(1.0);
            frames += 1;
            assert!(frames < 30);
        }
        // The frame it appears starts the tab, then 14 frames of slide (half-frame steps of 10/7).
        assert_eq!(frames, 15);
        assert_eq!((m.tab.x, m.tab.scale, m.tab_cmd), (0.0, 1.0, cmd::OPEN));
        let o = m.step(&w, press(|p| p.triangle = true));
        assert!(o.triangle && o.events == vec![Event::Context(cmd::OPEN)]);
        // No context command: Re:CoM's own Triangle action decides (nothing to show).
        let mut plain = Menu::new();
        plain.show();
        assert!(plain.step(&world(), press(|p| p.triangle = true)).triangle);
        // In the Magic list Triangle does nothing.
        let ws = World { spells: all_spells(), ..w };
        m.root.cursor = 1;
        m.step(&ws, press(|p| p.right = true));
        assert!(!m.step(&ws, press(|p| p.triangle = true)).triangle);
        // The object goes: the tab slides out by its scale (14 frames), keeping its name.
        let gone = World { context: 0, ..world() };
        m.step(&gone, Pad::default());
        m.tick(1.0);
        assert_eq!(m.tab.state, 3);
        let mut n = 1;
        while m.tab.state != 0 {
            m.step(&gone, Pad::default());
            m.tick(1.0);
            n += 1;
        }
        assert_eq!(n, 15, "the frame it goes, then 14 frames of shrinking");
        // Hidden HUD: reset.
        m.build(&w);
        m.hide();
        m.tick(1.0);
        assert_eq!(m.tab, Tab::default());
    }

    #[test]
    fn scene_positions_are_kh1s() {
        let w = World { spells: all_spells(), context: cmd::TALK, ..world() };
        let mut m = Menu::new();
        m.show();
        m.step(&w, Pad::default());
        m.tick(1.0);
        let s = m.scene();
        // Rows 0..2 and the sliding tab.
        assert_eq!(s.rows.iter().map(|r| (r.x, r.y)).collect::<Vec<_>>(), vec![(16.0, 260.0), (16.0, 282.0), (16.0, 304.0), (16.0, 326.0)]);
        assert!(s.rows[0].highlight && s.rows[3].tab && s.rows[3].name == 4);
        // Magic with 7 spells: bottom-anchored at 348 − 22 × 7 = 194, x + 11.
        m.root.cursor = 1;
        m.step(&w, press(|p| p.right = true));
        let s = m.scene();
        let mg: Vec<_> = s.rows.iter().filter(|r| r.list == Kind::Magic).collect();
        assert_eq!((mg[0].x, mg[0].y, mg.len()), (27.0, 194.0, 7));
        assert!(mg[0].highlight && !s.rows[1].highlight, "only the top list's cursor row");
        // Two spells: at the Magic row's y.
        let mut two = [0; 7];
        two[0] = 2;
        two[3] = 1;
        m.step(&World { spells: two, ..w }, Pad::default());
        let s = m.scene();
        let mg: Vec<_> = s.rows.iter().filter(|r| r.list == Kind::Magic).collect();
        assert_eq!((mg[0].y, mg[1].y, mg[0].name, mg[1].name), (282.0, 304.0, 8, 16));
        // No context, cursor elsewhere: row 3 isn't drawn; on it: drawn empty.
        let mut m = Menu::new();
        m.show();
        m.step(&world(), Pad::default());
        assert_eq!(m.scene().rows.len(), 3);
        m.root.cursor = 3;
        let s = m.scene();
        assert_eq!((s.rows.len(), s.rows[3].name, s.rows[3].highlight), (4, 0, true));
    }

    #[test]
    fn words_manifest_and_font_bytes() {
        let j = r#"{"format": "kh1-recom-assets command-menu 1", "plates": {}, "words": {"us": {"1": "Attack", "2": "Magic", "3": "Items", "19": "Gravity"}, "fr": {"1": "Attaquer", "19": "Gravité"}}, "sources": {}}"#;
        let fr = parse_words(j, "fr").unwrap();
        assert_eq!(fr.get(1), Some(&b"Attaquer\0"[..]));
        assert_eq!(fr.get(2), Some(&b"Magic\0"[..]), "missing ids come from us");
        assert_eq!(fr.get(19), Some(&b"Gravite\0"[..]));
        assert_eq!(fr.folded, vec!['é']);
        let xx = parse_words(j, "xx").unwrap();
        assert_eq!((xx.lang.as_str(), xx.get(19)), ("us", Some(&b"Gravity\0"[..])));
        assert!(parse_words(&j.replace("menu 1", "menu 9"), "us").is_err());
        assert_eq!(font_bytes("Brasier +").0, b"Brasier +\0");
        assert_eq!(font_bytes("Öffnen").0, b"Offnen\0");
    }

    #[test]
    fn pad_word_is_kh1s() {
        assert_eq!(press(|p| p.up = true).word(), 0x20);
        assert_eq!(press(|p| p.right = true).word(), 0x100);
        assert_eq!(press(|p| p.cancel = true).word() & 2, 2);
        assert_eq!(
            press(|p| {
                p.up = true;
                p.confirm = true
            })
            .word(),
            0x21_0021
        );
        assert!(cmd::row0_special(cmd::TALK) && cmd::row0_special(0x58) && !cmd::row0_special(cmd::ATTACK) && !cmd::row0_special(cmd::OPEN));
        assert_eq!(state_flags(0) & 0x70, 0x70);
    }
}
