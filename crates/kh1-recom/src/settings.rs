//! `kh1-recom.ini`, next to the game. Written with the defaults when missing, read at start.
//!
//! Every setting switches between Re:CoM's behaviour and KH1's. The rows we add to Re:CoM's
//! Config menu change these values and rewrite this file at once, so the file stays the backup
//! (and the only store: Re:CoM's own config bytes live in its save data, see
//! `docs/recom-config-menu.md`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Recom,
    Kh1,
}

impl Mode {
    fn parse(v: &str) -> Option<Mode> {
        match v.trim().to_ascii_lowercase().as_str() {
            "recom" => Some(Mode::Recom),
            "kh1" => Some(Mode::Kh1),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Recom => "recom",
            Mode::Kh1 => "kh1",
        }
    }
}

fn parse_switch(v: &str) -> Option<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "on" | "1" | "true" | "yes" => Some(true),
        "off" | "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

pub fn switch_str(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Sora's combat: Re:CoM's, or KH1's (from M4: KH1's movement).
    pub combat: Mode,
    /// Enemy reactions to Sora's KH1 Guard (decision 27; only at `combat = kh1`): a blocked melee
    /// attacker plays Re:CoM's own flinch, a blocked shot flies back and can hurt enemies.
    pub guard_reactions: bool,
    /// How often enemies attack in a crowd at `combat = kh1` (decision 32). `Recom`: Re:CoM's
    /// crowd sharing stays (each enemy's think rate is multiplied by the number of enemies,
    /// `0x14017F380`); `Kh1`: each enemy attacks as if alone (decision 30, patch P4).
    pub enemy_pace: Mode,
    /// Our rows in Re:CoM's Config menu. `off` leaves the menu untouched (a safety switch while
    /// the rows are untested in game; the settings then come from this file only).
    pub config_menu: bool,
    /// KH1 abilities Sora has at `combat = kh1`, until abilities are built into Re:CoM's
    /// progression (roadmap M9).
    pub abilities: Kh1Abilities,
    /// One-line logs of the Attack's decisions in `kh1-recom.log` (first 200 lines).
    pub debug_attack: bool,
    /// Debug (M7, until stage 7.11 earns spells): Sora has all 7 spells at level 3.
    pub kh1_all_spells: bool,
    /// Debug (M8 stage 8a.1, until the shop and drops give items): a test inventory: whenever
    /// Sora's slots are all empty as a room or battle starts, they get Potion, Hi-Potion, Ether,
    /// and the stock at least 9 of each battle item (written to the save).
    pub kh1_test_items: bool,
    /// The language of the command menu's KH1 words (`kh1-command-menu.json`): us, uk, fr, gr,
    /// it or sp.
    pub menu_language: String,
    /// The HUD (decision 39): Re:CoM's, or KH1's elements drawn by Re:CoM from the `kh1hud` set
    /// (`kh1-hud.json`). Interim switch until the KH1 camp entry has its row (stage 7.13).
    pub hud: Mode,
    /// Which KH1 HUD pieces show at `hud = kh1`, and which of Re:CoM's are hidden then (owner,
    /// 2026-11-01: "I want to be able to pick and choose which elements i want").
    pub hud_parts: HudParts,
}

/// The KH1 HUD's pieces (each only at `hud = kh1`). Defaults: the owner's picks, 2026-11-01 (the
/// baked KH1 gauge dropped: "Lets just keep the KH1 command menu and the triangle prompts. For the
/// health bar ... get the Re:CoM health bar, place it in the same place as the KH1 health bar";
/// MP "Around the ring").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudParts {
    /// KH1's gauge cluster (HP / MP rings, face, hurt face, low-HP flash, shake); off: Re:CoM's
    /// HP gauge (placed by [`HudParts::recom_hp_place`]) and the MP ring or the 7.x MP bar.
    pub gauge: bool,
    /// With the KH1 gauge off: Re:CoM's HP gauge at KH1's place (bottom right), its ring and bar
    /// turned 180° as KH1's (the face upright); off: Re:CoM's place (top left).
    pub recom_hp_place: bool,
    /// With the KH1 gauge off and Re:CoM's gauge at KH1's place: KH1's MP ring (MP, max MP,
    /// charge, KH1's fades and straight segments) around Re:CoM's HP ring; off: the 7.2 MP bar.
    pub mp_ring: bool,
    /// KH1's command menu look (panels, font, icons, the green Triangle tab); off: 7.3's look.
    pub menu: bool,
    /// Donald's and Goofy's KH1 gauges (drawn with the gauge cluster).
    pub party: bool,
    /// Re:CoM's HP gauge hidden.
    pub hide_hp: bool,
    /// Re:CoM's minimap hidden.
    pub hide_minimap: bool,
    /// Re:CoM's FRIENDS row hidden.
    pub hide_friends: bool,
    /// Re:CoM's EXP / NEXT LV / Moogle point pop-ups hidden.
    pub hide_prizes: bool,
    /// With Re:CoM's HP gauge at KH1's place (bottom right): Re:CoM's room plate (the room card
    /// and its name, bottom right) moved to the top left (owner, 2026-11-01: "the room indicator
    /// should be in the top left of the screen"; 0.40.x: top right, under the minimap); off:
    /// Re:CoM's place. Key `hud_room_plate_top_left`; 0.40.x's `hud_room_plate_top_right` is read
    /// as the same switch.
    pub room_plate_top: bool,
}

impl Default for HudParts {
    fn default() -> Self {
        HudParts { gauge: false, recom_hp_place: true, mp_ring: true, menu: true, party: true, hide_hp: false, hide_minimap: false, hide_friends: true, hide_prizes: false, room_plate_top: true }
    }
}

impl HudParts {
    /// (ini key, value) for each piece, in the file's order.
    pub fn keys(self) -> [(&'static str, bool); 10] {
        [
            ("hud_kh1_gauge", self.gauge),
            ("hud_recom_hp_kh1_place", self.recom_hp_place),
            ("hud_kh1_mp_ring", self.mp_ring),
            ("hud_kh1_menu", self.menu),
            ("hud_kh1_party", self.party),
            ("hud_hide_recom_hp", self.hide_hp),
            ("hud_hide_minimap", self.hide_minimap),
            ("hud_hide_friends", self.hide_friends),
            ("hud_hide_prizes", self.hide_prizes),
            ("hud_room_plate_top_left", self.room_plate_top),
        ]
    }

    fn set(&mut self, key: &str, v: bool) -> bool {
        match key {
            "hud_kh1_gauge" => self.gauge = v,
            "hud_recom_hp_kh1_place" => self.recom_hp_place = v,
            "hud_kh1_mp_ring" => self.mp_ring = v,
            "hud_kh1_menu" => self.menu = v,
            "hud_kh1_party" => self.party = v,
            "hud_hide_recom_hp" => self.hide_hp = v,
            "hud_hide_minimap" => self.hide_minimap = v,
            "hud_hide_friends" => self.hide_friends = v,
            "hud_hide_prizes" => self.hide_prizes = v,
            "hud_room_plate_top_left" | "hud_room_plate_top_right" => self.room_plate_top = v,
            _ => return false,
        }
        true
    }
}

/// The languages the asset step writes KH1's menu words in.
pub const MENU_LANGUAGES: &[&str] = &["us", "uk", "fr", "gr", "it", "sp"];

/// The movement abilities the KH1 handlers read (`kh1_core::Abilities`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kh1Abilities {
    pub high_jump: bool,
    pub glide: bool,
    pub superglide: bool,
    pub dodge_roll: bool,
    pub guard: bool,
    pub counterattack: bool,
    /// Copies of Combo Plus / Air Combo Plus (each adds a swing to the combo), and Combo Master.
    pub combo_plus: u8,
    pub air_combo_plus: u8,
    pub combo_master: bool,
}

impl Default for Kh1Abilities {
    fn default() -> Self {
        Kh1Abilities { high_jump: false, glide: false, superglide: false, dodge_roll: true, guard: true, counterattack: true, combo_plus: 0, air_combo_plus: 0, combo_master: false }
    }
}

impl Kh1Abilities {
    pub fn to_core(self) -> kh1_core::Abilities {
        kh1_core::Abilities {
            high_jump: self.high_jump,
            glide: self.glide,
            superglide: self.superglide,
            dodge_roll: self.dodge_roll,
            guard: self.guard,
            counterattack: self.counterattack,
            combo_plus: self.combo_plus,
            air_combo_plus: self.air_combo_plus,
            combo_master: self.combo_master,
            ..Default::default()
        }
    }

    /// KH1's ability bits (`0x142D5EC10`), as the trace writes them.
    pub fn bits(self) -> u32 {
        (self.high_jump as u32) << 9 | (self.glide as u32) << 7 | (self.superglide as u32) << 8 | (self.guard as u32) << 4 | (self.dodge_roll as u32) << 5 | (self.counterattack as u32) << 2
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings { combat: Mode::Kh1, guard_reactions: true, enemy_pace: Mode::Recom, config_menu: true, abilities: Kh1Abilities::default(), debug_attack: false, kh1_all_spells: false, kh1_test_items: false, menu_language: "us".into(), hud: Mode::Recom, hud_parts: HudParts::default() }
    }
}

pub const DEFAULT_FILE: &str = "\
; KH1 combat in Re:Chain of Memories -- settings
; The Config menu (camp menu > Config) changes these and rewrites this file.
;
; combat = kh1 | recom
;   kh1:   Sora fights as in KH1 (no cards in battle).
;   recom: Sora fights with Re:CoM's cards, as in the original game.
;   kh1 moves Sora with KH1's movement (walk, run, jump, glide, Dodge Roll, Guard).
combat = kh1
;
; guard_reactions = on | off (only at combat = kh1)
;   on:  when Sora's Guard blocks an enemy, the attacker flinches (Re:CoM's own flinch), and a
;        blocked shot flies back and can hurt enemies (Re:CoM's damage).
;   off: a blocked enemy carries on; a blocked shot just vanishes.
guard_reactions = on
;
; enemy_pace = recom | kh1 (only at combat = kh1)
;   recom: a crowd of enemies shares one attack budget, as in Re:CoM (four enemies each attack
;          about four times less often than one alone).
;   kh1:   each enemy attacks as often as when it is alone.
enemy_pace = recom
;
; config_menu = on | off
;   off: don't add our rows to Re:CoM's Config menu (use this file instead).
config_menu = on
;
; KH1 abilities at combat = kh1 (on | off), until abilities are built into the game:
high_jump = off
glide = off
superglide = off
dodge_roll = on
guard = on
counterattack = on
; Combo Plus / Air Combo Plus: how many are equipped (0-9); combo_master = on | off
combo_plus = 0
air_combo_plus = 0
combo_master = off
;
; debug_attack = on | off: log the Attack's decisions (first 200 lines) in kh1-recom.log
debug_attack = off
;
; kh1_all_spells = on | off (debug, until spells are earned in the game): Sora has all 7 KH1
;   spells at level 3 in the command menu's Magic list.
kh1_all_spells = off
;
; kh1_test_items = on | off (debug, until items come from the shop and drops): whenever Sora's
;   item slots are all empty as a room or battle starts, they are filled with Potion, Hi-Potion
;   and Ether, and the stock keeps at least 9 of each item. Written to the save.
kh1_test_items = off
;
; menu_language = us | uk | fr | gr | it | sp: the language of the command menu's KH1 words.
menu_language = us
;
; hud = recom | kh1 (needs the asset step's kh1-hud.json)
;   recom: Re:CoM's HUD.
;   kh1:   KH1's HUD elements, drawn by Re:CoM (stage by stage; the rest stays Re:CoM's).
hud = recom
;
; At hud = kh1, each piece on or off:
;   hud_kh1_gauge: KH1's own HP / MP rings and face (off: Re:CoM's HP gauge, see the next two)
;   hud_recom_hp_kh1_place: with hud_kh1_gauge off, Re:CoM's HP gauge bottom right where KH1's
;     is, its ring and bar turned 180 degrees as KH1's, Sora's face upright (off: top left)
;   hud_kh1_mp_ring: with hud_kh1_gauge off, hud_recom_hp_kh1_place on and combat = kh1, KH1's
;     MP ring around Re:CoM's HP ring (KH1's MP, charge and fades; past 9 MP KH1's straight
;     segments along the bottom) (off: the Re:CoM-style MP bar under the HP bar)
;   hud_kh1_menu:  KH1's command menu look and green Triangle tab (off: the Re:CoM-styled menu)
;   hud_kh1_party: Donald's and Goofy's KH1 gauges (shown with hud_kh1_gauge)
;   hud_hide_recom_hp / hud_hide_minimap / hud_hide_friends / hud_hide_prizes: hide Re:CoM's
;     HP gauge / minimap / FRIENDS row / EXP, NEXT LV and Moogle point pop-ups
;   hud_room_plate_top_left: with hud_recom_hp_kh1_place on, Re:CoM's room plate (the room card
;     and its name) top left, off the gauge (off: bottom right)
hud_kh1_gauge = off
hud_recom_hp_kh1_place = on
hud_kh1_mp_ring = on
hud_kh1_menu = on
hud_kh1_party = on
hud_hide_recom_hp = off
hud_hide_minimap = off
hud_hide_friends = on
hud_hide_prizes = off
hud_room_plate_top_left = on
";

/// Parses the file. Unknown keys and bad values are reported and the default kept.
pub fn parse(text: &str) -> (Settings, Vec<String>) {
    let mut s = Settings::default();
    let mut warnings = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            warnings.push(format!("line {}: expected key = value", n + 1));
            continue;
        };
        let bad = |what: &str| format!("line {}: {what}", n + 1);
        match k.trim().to_ascii_lowercase().as_str() {
            "combat" => match Mode::parse(v) {
                Some(m) => s.combat = m,
                None => warnings.push(bad("combat must be kh1 or recom")),
            },
            // The "KH1 motions test" switch was removed (owner, 2026-10-30): an old line is
            // ignored without a warning.
            "kh1_motions" => {}
            "guard_reactions" => match parse_switch(v) {
                Some(b) => s.guard_reactions = b,
                None => warnings.push(bad("guard_reactions must be on or off")),
            },
            "enemy_pace" => match Mode::parse(v) {
                Some(m) => s.enemy_pace = m,
                None => warnings.push(bad("enemy_pace must be kh1 or recom")),
            },
            "config_menu" => match parse_switch(v) {
                Some(b) => s.config_menu = b,
                None => warnings.push(bad("config_menu must be on or off")),
            },
            k @ ("high_jump" | "glide" | "superglide" | "dodge_roll" | "guard" | "counterattack") => match parse_switch(v) {
                Some(b) => {
                    let a = &mut s.abilities;
                    *match k {
                        "high_jump" => &mut a.high_jump,
                        "glide" => &mut a.glide,
                        "superglide" => &mut a.superglide,
                        "dodge_roll" => &mut a.dodge_roll,
                        "counterattack" => &mut a.counterattack,
                        _ => &mut a.guard,
                    } = b;
                }
                None => warnings.push(bad(&format!("{k} must be on or off"))),
            },
            k @ ("combo_plus" | "air_combo_plus") => match v.trim().parse::<u8>() {
                Ok(n) if n <= 9 => {
                    if k == "combo_plus" {
                        s.abilities.combo_plus = n;
                    } else {
                        s.abilities.air_combo_plus = n;
                    }
                }
                _ => warnings.push(bad(&format!("{k} must be a number from 0 to 9"))),
            },
            "debug_attack" => match parse_switch(v) {
                Some(b) => s.debug_attack = b,
                None => warnings.push(bad("debug_attack must be on or off")),
            },
            "kh1_all_spells" => match parse_switch(v) {
                Some(b) => s.kh1_all_spells = b,
                None => warnings.push(bad("kh1_all_spells must be on or off")),
            },
            "kh1_test_items" => match parse_switch(v) {
                Some(b) => s.kh1_test_items = b,
                None => warnings.push(bad("kh1_test_items must be on or off")),
            },
            "menu_language" => {
                let l = v.trim().to_ascii_lowercase();
                if MENU_LANGUAGES.contains(&l.as_str()) {
                    s.menu_language = l;
                } else {
                    warnings.push(bad("menu_language must be us, uk, fr, gr, it or sp"));
                }
            }
            "hud" => match Mode::parse(v) {
                Some(m) => s.hud = m,
                None => warnings.push(bad("hud must be recom or kh1")),
            },
            k if k.starts_with("hud_") => match parse_switch(v) {
                Some(b) if s.hud_parts.set(k, b) => {}
                Some(_) => warnings.push(bad(&format!("unknown key `{k}`"))),
                None => warnings.push(bad(&format!("{k} must be on or off"))),
            },
            "combo_master" => match parse_switch(v) {
                Some(b) => s.abilities.combo_master = b,
                None => warnings.push(bad("combo_master must be on or off")),
            },
            other => warnings.push(bad(&format!("unknown key `{other}`"))),
        }
    }
    (s, warnings)
}

/// Sets `key = value` in the file text, keeping every other line (and the comment after the
/// value). A missing key is appended.
pub fn set_key(text: &str, key: &str, value: &str) -> String {
    let mut out = String::with_capacity(text.len() + 32);
    let mut found = false;
    for line in text.lines() {
        let (body, comment) = match line.find(';') {
            Some(i) => (&line[..i], &line[i..]),
            None => (line, ""),
        };
        let is_key = body.split_once('=').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(key));
        if is_key && !found {
            found = true;
            out.push_str(key);
            out.push_str(" = ");
            out.push_str(value);
            if !comment.is_empty() {
                out.push(' ');
                out.push_str(comment);
            }
        } else if is_key {
            // A repeated key would override ours on the next read: comment it out.
            out.push_str("; ");
            out.push_str(line);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !found {
        out.push_str(key);
        out.push_str(" = ");
        out.push_str(value);
        out.push('\n');
    }
    out
}

/// Writes every setting into the file text.
pub fn store(text: &str, s: &Settings) -> String {
    let t = set_key(text, "combat", s.combat.as_str());
    let t = set_key(&t, "guard_reactions", switch_str(s.guard_reactions));
    let t = set_key(&t, "enemy_pace", s.enemy_pace.as_str());
    let mut t = set_key(&t, "config_menu", switch_str(s.config_menu));
    let a = s.abilities;
    for (k, v) in [("high_jump", a.high_jump), ("glide", a.glide), ("superglide", a.superglide), ("dodge_roll", a.dodge_roll), ("guard", a.guard), ("counterattack", a.counterattack)] {
        t = set_key(&t, k, switch_str(v));
    }
    let t = set_key(&t, "combo_plus", &a.combo_plus.to_string());
    let t = set_key(&t, "air_combo_plus", &a.air_combo_plus.to_string());
    let t = set_key(&t, "combo_master", switch_str(a.combo_master));
    let t = set_key(&t, "debug_attack", switch_str(s.debug_attack));
    let t = set_key(&t, "kh1_all_spells", switch_str(s.kh1_all_spells));
    let t = set_key(&t, "kh1_test_items", switch_str(s.kh1_test_items));
    let t = set_key(&t, "menu_language", &s.menu_language);
    let mut t = set_key(&t, "hud", s.hud.as_str());
    for (k, v) in s.hud_parts.keys() {
        t = set_key(&t, k, switch_str(v));
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_file_parses_to_defaults() {
        let (s, w) = parse(DEFAULT_FILE);
        assert_eq!(s, Settings::default());
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn reads_values_and_reports_bad_lines() {
        let (s, w) = parse("combat = ReCoM ; comment\nfoo = 1\nnonsense\ncombat=kh2\nconfig_menu=maybe\n");
        assert_eq!(s.combat, Mode::Recom);
        assert!(s.config_menu);
        assert_eq!(w.len(), 4);
    }

    #[test]
    fn old_motions_line_is_ignored_silently() {
        let old = "combat = recom\n;\n; kh1_motions = off | on\nkh1_motions = on\nKH1_Motions = nonsense\nconfig_menu = off\n";
        let (s, w) = parse(old);
        assert_eq!(s, Settings { combat: Mode::Recom, config_menu: false, ..Default::default() });
        assert!(w.is_empty(), "{w:?}");
        // Rewriting keeps the old line (harmless) and still reads back clean.
        let t = store(old, &Settings { combat: Mode::Kh1, config_menu: false, ..Default::default() });
        assert!(t.contains("kh1_motions = on"));
        let (back, w) = parse(&t);
        assert_eq!(back.combat, Mode::Kh1);
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn abilities_read_and_map_to_kh1_bits() {
        let (s, w) = parse("high_jump = on\nglide = yes\nsuperglide = 1\ndodge_roll = off\nguard = no\nglide = maybe\n");
        assert_eq!(w.len(), 1, "{w:?}");
        assert_eq!(s.abilities, Kh1Abilities { high_jump: true, glide: true, superglide: true, dodge_roll: false, guard: false, ..Default::default() });
        assert_eq!(s.abilities.bits(), 0x200 | 0x80 | 0x100 | 0x4);
        assert_eq!(Kh1Abilities::default().bits(), 0x10 | 0x20 | 0x4);
        let (off, _) = parse("counterattack = off\n");
        assert!(!off.abilities.counterattack && !off.abilities.to_core().counterattack);
        let c = s.abilities.to_core();
        assert!(c.high_jump && c.glide && c.superglide && !c.dodge_roll && !c.guard);
    }

    #[test]
    fn set_key_keeps_the_rest() {
        let t = set_key("; head\ncombat = kh1 ; why\nother = 1\n", "combat", "recom");
        assert_eq!(t, "; head\ncombat = recom ; why\nother = 1\n");
        let t = set_key("combat=kh1\n", "config_menu", "on");
        assert_eq!(t, "combat=kh1\nconfig_menu = on\n");
        let t = set_key("combat = kh1\ncombat = recom\n", "combat", "kh1");
        assert_eq!(parse(&t).0.combat, Mode::Kh1);
    }

    #[test]
    fn store_round_trips() {
        let s = Settings { combat: Mode::Recom, guard_reactions: false, enemy_pace: Mode::Kh1, config_menu: false, abilities: Kh1Abilities { high_jump: true, glide: true, ..Default::default() }, debug_attack: true, kh1_all_spells: true, kh1_test_items: true, menu_language: "fr".into(), hud: Mode::Kh1, hud_parts: HudParts { gauge: false, recom_hp_place: false, mp_ring: true, menu: true, party: false, hide_hp: false, hide_minimap: true, hide_friends: false, hide_prizes: true, room_plate_top: false } };
        let t = store(DEFAULT_FILE, &s);
        let (back, w) = parse(&t);
        assert_eq!(back, s);
        assert!(w.is_empty(), "{w:?}");
        assert!(t.contains("; combat = kh1 | recom"), "comments survive");
        // An empty file gets every key.
        assert_eq!(parse(&store("", &s)).0, s);
    }
}

#[cfg(test)]
mod guard_reaction_tests {
    use super::*;

    #[test]
    fn enemy_pace_default_recom_and_round_trip() {
        assert_eq!(Settings::default().enemy_pace, Mode::Recom);
        assert!(DEFAULT_FILE.contains("\nenemy_pace = recom\n"));
        assert_eq!(parse(DEFAULT_FILE).0.enemy_pace, Mode::Recom);
        // An older file without the key keeps the default.
        assert_eq!(parse("combat = kh1\n").0.enemy_pace, Mode::Recom);
        let (s, w) = parse("enemy_pace = kh1\nenemy_pace = fast\n");
        assert_eq!(s.enemy_pace, Mode::Kh1);
        assert_eq!(w.len(), 1);
        let t = store(DEFAULT_FILE, &Settings { enemy_pace: Mode::Kh1, ..Default::default() });
        assert_eq!(t.lines().filter(|l| l.starts_with("enemy_pace")).count(), 1);
        assert_eq!(parse(&t).0.enemy_pace, Mode::Kh1);
    }

    #[test]
    fn guard_reactions_default_on_and_round_trip() {
        assert!(Settings::default().guard_reactions);
        assert!(DEFAULT_FILE.contains("\nguard_reactions = on\n"));
        // An old ini without the key keeps the default (on), without a warning.
        let (s, w) = parse("combat = kh1\nconfig_menu = on\n");
        assert!(s.guard_reactions && w.is_empty(), "{w:?}");
        let (s, w) = parse("guard_reactions = off\nguard_reactions = maybe\n");
        assert!(!s.guard_reactions);
        assert_eq!(w.len(), 1, "{w:?}");
        // Rewriting an old file appends the key; flipping it changes only that line.
        let t = store("combat = kh1\n", &Settings { guard_reactions: false, ..Default::default() });
        assert!(t.contains("guard_reactions = off"));
        assert!(!parse(&t).0.guard_reactions);
        let t2 = store(DEFAULT_FILE, &Settings { guard_reactions: false, ..Default::default() });
        assert_eq!(t2.lines().filter(|l| l.starts_with("guard_reactions")).count(), 1);
        assert!(t2.contains("; guard_reactions = on | off"), "comments survive");
        assert_eq!(parse(&t2).0, Settings { guard_reactions: false, ..Default::default() });
    }
}

#[cfg(test)]
mod menu_tests {
    use super::*;

    #[test]
    fn menu_switches_default_off_and_us() {
        let d = Settings::default();
        assert!(!d.kh1_all_spells && !d.kh1_test_items && d.menu_language == "us");
        let (s, w) = parse("kh1_all_spells = on\nmenu_language = GR\nmenu_language = jp\n");
        assert!(s.kh1_all_spells && s.menu_language == "gr");
        let (s, w2) = parse("kh1_test_items = on\n");
        assert!(s.kh1_test_items && w2.is_empty());
        assert_eq!(w.len(), 1, "{w:?}");
        // An older file without the keys keeps the defaults without a warning.
        let (s, w) = parse("combat = kh1\n");
        assert!(!s.kh1_all_spells && s.menu_language == "us" && w.is_empty());
    }
}

#[cfg(test)]
mod combo_tests {
    use super::*;

    #[test]
    fn combo_abilities_read_and_reach_kh1() {
        let (s, w) = parse("combo_plus = 2\nair_combo_plus = 1\ncombo_master = on\ncombo_plus = 12\n");
        assert_eq!(w.len(), 1, "{w:?}");
        assert_eq!((s.abilities.combo_plus, s.abilities.air_combo_plus, s.abilities.combo_master), (2, 1, true));
        let c = s.abilities.to_core();
        assert_eq!((c.combo_plus, c.air_combo_plus, c.combo_master), (2, 1, true));
        let back = parse(&store(DEFAULT_FILE, &s)).0;
        assert_eq!(back.abilities, s.abilities);
    }

    #[test]
    fn hud_parts_default_and_parse() {
        assert_eq!(parse(DEFAULT_FILE).0.hud_parts, HudParts::default());
        let (s, w) = parse("hud_kh1_gauge = on\nhud_hide_minimap = on\nhud_bogus = on\n");
        assert_eq!((s.hud_parts.gauge, s.hud_parts.hide_minimap, w.len()), (true, true, 1));
        // The owner's ini (2026-11-01) has neither of the two new keys: both on.
        let (s, w) = parse("hud = kh1\nhud_kh1_gauge = off\nhud_kh1_menu = on\nhud_kh1_party = off\nhud_hide_recom_hp = off\nhud_hide_friends = on\n");
        assert!(w.is_empty());
        assert!(!s.hud_parts.gauge && s.hud_parts.recom_hp_place && s.hud_parts.mp_ring && !s.hud_parts.hide_hp);
        let (s, _) = parse("hud_recom_hp_kh1_place = off\nhud_kh1_mp_ring = off\n");
        assert!(!s.hud_parts.recom_hp_place && !s.hud_parts.mp_ring);
        assert!(s.hud_parts.room_plate_top, "on unless the file says off");
        let (s, w) = parse("hud_room_plate_top_left = off\n");
        assert!(!s.hud_parts.room_plate_top && w.is_empty());
        // 0.40.x's key is the same switch.
        let (s, w) = parse("hud_room_plate_top_right = off\n");
        assert!(!s.hud_parts.room_plate_top && w.is_empty());
    }

    #[test]
    fn hud_default_recom_and_round_trip() {
        assert_eq!(parse(DEFAULT_FILE).0.hud, Mode::Recom);
        let (s, w) = parse("hud = kh1\n");
        assert!(w.is_empty());
        assert_eq!(s.hud, Mode::Kh1);
        assert_eq!(parse(&store(DEFAULT_FILE, &s)).0.hud, Mode::Kh1);
        assert_eq!(parse("hud = maybe\n").1.len(), 1);
    }
}
