//! The KH1 HUD option (decision 39, `docs/recom-kh1-hud.md`): KH1's HUD elements drawn by Re:CoM's
//! 2D code from the `kh1hud` sprite set the asset step adds to Re:CoM's `FO0002.SPR` / `.RTM`
//! (`kh1-recom-assets` `kh1_hud`). This module holds what is pure: the manifest check and the
//! group numbering (the same as the asset step's).

/// The manifest's format line (as `kh1_recom_assets::kh1_hud::FORMAT`).
/// Format 3 (0.35.0): the menu's FM context tab and Triangle groups (format 2 sets have none:
/// refused, with the log's "run the asset step of this version").
pub const FORMAT: &str = "kh1-recom-assets kh1-hud 3";
/// The sprite set's name in Re:CoM's 2D registry, NUL-terminated.
pub const SET: &[u8] = b"kh1hud:00\0";

/// The animation groups of `kh1hud:00` (as `kh1_recom_assets::kh1_hud::group`).
pub mod group {
    /// Face frame, disc, labels: frame 0 without "mp", 1 with.
    pub const FRAME: u32 = 0;
    /// Back arcs and segments: frame = min(max HP, 120) / min(max MP, 20).
    pub const HP_BACK: u32 = 1;
    /// MP back.
    pub const MP_BACK: u32 = 2;
    /// MP: frame = min(MP, 20).
    pub const MP: u32 = 3;
    /// HP: frame = min(HP, 120).
    pub const HP: u32 = 4;
    /// Charge: frame = min(charge, 600).
    pub const CHARGE: u32 = 5;
    /// Max-MP ticks: frame = min(max MP, 20).
    pub const TICKS: u32 = 6;
    /// Face: frame = expression (0, 1 hurt, 2 danger).
    pub const FACE: u32 = 7;
    /// Hurt face for 40 / 50 / 60 frames then 0 or 2 (played): 8 + 2 i + (danger).
    pub const HURT: u32 = 8;
    /// MP fades of cell j (played): gain 14 + j, loss 23 + j.
    pub const MP_GAIN: u32 = 14;
    /// MP loss fades.
    pub const MP_LOSS: u32 = 23;
    /// Charge fades of [30 j, 30 j + t] (t = 1..30, played): gain 32 + 30 j + t − 1.
    pub const CHARGE_GAIN: u32 = 32;
    /// Charge loss fades: 302 + 30 j + t − 1.
    pub const CHARGE_LOSS: u32 = 302;
    /// Danger flash (looped) of the HP arc piece [90 q, 90 q + 5 k]: 572 + 18 q + k − 1.
    pub const RED_ARC: u32 = 572;
    /// Danger flash of a full straight HP segment.
    pub const RED_SEG: u32 = 626;
    /// Danger flash of a partial one (remainder r = 1..5): 627 + r − 1.
    pub const RED_SEG_PART: u32 = 627;
    /// The fixed groups (the motion units' groups follow: [`super::Motion`]).
    pub const COUNT: u32 = 632;
}

/// H2b: the motion programs of every unit (`kh1_recom_assets::kh1_hud::motion`), in this order.
pub mod program {
    /// The hit shakes `vibe_s`, `vibe_ss`, `vibe_sss` (strength 0, 1, 2).
    pub const VIBE: usize = 0;
    /// KH1's gauge fade out (10 frames).
    pub const FADE_OUT: usize = 3;
    /// KH1's gauge fade in.
    pub const FADE_IN: usize = 4;
    /// Programs every unit has.
    pub const BASE: usize = 5;
    /// The size-class slides (`slide_s_h` to class 0 / 1, `slide_s_v` to class 2; `0x14026D570`).
    pub const SLIDE_H: usize = 5;
    /// To class 2.
    pub const SLIDE_V: usize = 6;
    /// Programs in all.
    pub const COUNT: usize = 7;
    /// The names the manifest must list, in order.
    pub const NAMES: [&str; COUNT] = ["vibe_s", "vibe_ss", "vibe_sss", "fade_out", "fade_in", "slide_s_h", "slide_s_v"];
}

/// The slide KH1 plays for a change to `new_class` (`0x14026D570`: slot `+0x120` below class 2,
/// `+0x1E0` for class 2).
pub fn slide_program(new_class: usize) -> usize {
    if new_class < 2 {
        program::SLIDE_H
    } else {
        program::SLIDE_V
    }
}

/// A motion program: the frame it is played from and its length (frames).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Program {
    /// The group frame to play from.
    pub start: i32,
    /// Frames until it is over.
    pub frames: i32,
}

/// The motion units: each a played group per program, shown instead of the held layers while a
/// motion runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Motion {
    /// The programs ([`program`] order).
    pub programs: Vec<Program>,
    /// Each unit's first group and program count.
    pub units: Vec<(u32, u32)>,
    /// (held group, f0, f1, unit): the unit shows while the held group's frame is in [f0, f1].
    pub runs: Vec<(u32, i32, i32, u32)>,
}

impl Motion {
    /// The units held group `held` shows at `frame`, in the held group's order.
    pub fn units_at(&self, held: u32, frame: i32) -> Vec<usize> {
        self.runs.iter().filter(|r| r.0 == held && r.1 <= frame && frame <= r.2).map(|r| r.3 as usize).collect()
    }

    /// Unit `u`'s group for program `p` (None when the unit has no such program: the slides).
    pub fn group(&self, u: usize, p: usize) -> Option<u32> {
        let &(first, n) = self.units.get(u)?;
        ((p as u32) < n).then_some(first + p as u32)
    }
}

/// H4: the command menu look's groups (`kh1_recom_assets::kh1_hud::menu`).
pub mod menu {
    /// Header frame, title, row, last row, selection, icons a / b / c: a look's groups.
    pub const HEADER: u32 = 0;
    /// Title.
    pub const TITLE: u32 = 1;
    /// A row's frame.
    pub const ROW: u32 = 2;
    /// The last row.
    pub const ROW_LAST: u32 = 3;
    /// The selection.
    pub const SEL: u32 = 4;
    /// Icon a (Attack); b, c follow.
    pub const ICON: u32 = 5;
    /// Groups a look; the battle look follows the field's.
    pub const LOOK: u32 = 8;
    /// The title turning red (played from frame 1).
    pub const TITLE_TO_BATTLE: u32 = 2 * LOOK;
    /// The Magic list's header, title, row, last row, selection, icon.
    pub const MAGIC: u32 = 2 * LOOK + 1;
    /// FM's context tab: shadow, green plate, band, stripe, at the tab's origin
    /// (`crate::command_menu::tab`).
    pub const TAB: u32 = MAGIC + 6;
    /// The Triangle (the PC's PlayStation glyph; empty when the asset step had no `Icon.tm2`).
    pub const TRIANGLE: u32 = TAB + 1;
    /// The first word group (the words are listed in the manifest).
    pub const WORDS: u32 = TAB + 2;
    /// KH1's root list origin (`0x14027B870`: (0x10, 0x104)), its header's height and row pitch,
    /// the selection's offset, the words' offsets (`0x140278B40`, `0x14027A920` adds (16, 2)).
    pub const HEADER_H: f32 = 20.0;
    /// Row pitch.
    pub const PITCH: f32 = 22.0;
    /// The selection's (x, y) from the list origin + 22 c.
    pub const SEL_AT: (f32, f32) = (11.0, 18.0);
    /// A row's word from the list origin + 22 i.
    pub const WORD_AT: (f32, f32) = (16.0, 22.0);
    /// The cursor row's word.
    pub const SEL_WORD_AT: (f32, f32) = (29.0, 25.0);
}

/// H5: the LOCK ON! plate and the target's HP gauge (`kh1_recom_assets::kh1_hud::target`). KH1's
/// rules (`0x1402747F0`, `0x1402758B0`, `0x140275000`; Checked (decomp)).
pub mod target {
    /// The plate (played), the gauge's head, body (frame = L), cap, bars 0..4 (frame = length),
    /// the marker.
    pub const PLATE: u32 = 0;
    /// Head.
    pub const HEAD: u32 = 1;
    /// Body.
    pub const BODY: u32 = 2;
    /// Cap.
    pub const CAP: u32 = 3;
    /// Bar 0.
    pub const BAR: u32 = 4;
    /// Marker.
    pub const MARKER: u32 = 9;
    /// Groups.
    pub const COUNT: u32 = 10;
    /// The origin (`0x140275000`: left margin + 16, 24; + 28 while the plate shows).
    pub const ORIGIN: (f32, f32) = (16.0, 24.0);
    /// The gauge's drop while the plate shows (`0x1402743E0`).
    pub const BELOW_PLATE: f32 = 28.0;
    /// The body / bars start at the origin + 28 (`0x2C` − 16).
    pub const BODY_X: f32 = 28.0;
    /// The gauge's fade a frame (`0x1402758B0`: ± 0.2).
    pub const GAUGE_FADE: f32 = 0.2;
    /// The plate's fade out a frame (`0x1402747F0` state 4: 1 / 6).
    pub const PLATE_FADE: f32 = 1.0 / 6.0;

    /// The body's length: min(max HP / 2, 150).
    pub fn body(max_hp: i32) -> i32 {
        (max_hp.max(0) / 2).min(150)
    }

    /// The five bars' lengths: bar k = min(HP − 300 k + 1, 300) / 2 while HP ≥ 300 k, else 0.
    pub fn bars(hp: i32) -> [i32; 5] {
        core::array::from_fn(|k| {
            let rest = hp - 300 * k as i32;
            if hp < 0 || rest < 0 {
                0
            } else {
                (rest + 1).min(300) / 2
            }
        })
    }

    /// The marker's bar (the top one with a length) when more than one bar shows.
    pub fn marker(bars: &[i32; 5]) -> Option<usize> {
        (1..5).rev().find(|&k| bars[k] > 0)
    }
}

/// H6: the party slots' offsets (`0x140527330` Donald (0, 0), `0x140527370` Goofy (68, 20);
/// Checked (exe)), added to the objects' places with the right margin.
pub const PARTY_OFFSET: [(f32, f32); 2] = [(0.0, 0.0), (68.0, 20.0)];

/// H8: Re:CoM's actor popups (`0x14020C9A0(actor, kind)`, `CCommonGauge`): kind 4 is the level
/// up (its only caller `0x1401E5730` raises Sora's level, then plays sound 0x36; Checked
/// (decomp)). KH1's LEVEL UP!! box at (left margin + 16, 16) (`0x140272F10`) replaces it.
pub const POPUP_LEVEL_UP: i32 = 4;
/// The LEVEL UP!! box's place (Sora's column, `0x140272F10`).
pub const LEVEL_UP_AT: (f32, f32) = (16.0, 16.0);

/// The menu groups of the manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuGroups {
    /// First group.
    pub first: u32,
    /// The Triangle's group holds the glyph.
    pub tab_icon: bool,
    /// (language, [(message id, group)]).
    pub words: Vec<(String, Vec<(u16, u32)>)>,
}

impl MenuGroups {
    /// The group of word `id` in `lang`, else in "us".
    pub fn word(&self, lang: &str, id: u16) -> Option<u32> {
        let find = |l: &str| self.words.iter().find(|w| w.0 == l).and_then(|w| w.1.iter().find(|x| x.0 == id)).map(|x| x.1);
        find(lang).or_else(|| find("us"))
    }
}

/// What `kh1-hud.json` says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// The set's group count.
    pub groups: u32,
    /// The languages the PC copies were built for.
    pub languages: Vec<String>,
    /// The motion units (H2b).
    pub motion: Motion,
    /// The command menu look (H4), when built.
    pub menu: Option<MenuGroups>,
    /// The LOCK ON! plate and target gauge's first group (H5), when built.
    pub target: Option<u32>,
    /// Donald's and Goofy's gauge groups (H6), when built.
    pub party: Option<[u32; 2]>,
    /// KH1's lock-on cursor group (H7), when built.
    pub cursor: Option<u32>,
    /// KH1's LEVEL UP!! box group (H8), when built.
    pub level_up: Option<u32>,
}

/// Reads `kh1-hud.json`; refuses another format or group count than this DLL's.
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let v = crate::json::parse(text)?;
    let format = v.get("format").and_then(|x| x.as_str()).ok_or("no format")?;
    if format != FORMAT {
        return Err(format!("format {format:?}, this DLL reads {FORMAT:?} (run the asset step of this version)"));
    }
    let set = v.get("set").and_then(|x| x.as_str()).ok_or("no set")?;
    if set.as_bytes() != &SET[..SET.len() - 1] {
        return Err(format!("set {set:?}, not kh1hud:00"));
    }
    let groups = v.get("groups").and_then(|x| x.as_u32()).ok_or("no groups")?;
    let base = v.get("base_groups").and_then(|x| x.as_u32()).ok_or("no base_groups")?;
    if base != group::COUNT {
        return Err(format!("{base} fixed groups, this DLL expects {}", group::COUNT));
    }
    let languages = v.get("languages").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|l| l.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let mut programs = Vec::new();
    for (i, p) in v.get("programs").and_then(|x| x.as_array()).ok_or("no programs")?.iter().enumerate() {
        let name = p.get("name").and_then(|x| x.as_str()).unwrap_or("");
        if program::NAMES.get(i) != Some(&name) {
            return Err(format!("program {i} is {name:?}, this DLL expects {:?}", program::NAMES.get(i)));
        }
        let start = p.get("start").and_then(|x| x.as_u32()).ok_or("program without start")? as i32;
        let frames = p.get("frames").and_then(|x| x.as_u32()).ok_or("program without frames")? as i32;
        programs.push(Program { start, frames });
    }
    if programs.len() != program::COUNT {
        return Err(format!("{} programs, this DLL expects {}", programs.len(), program::COUNT));
    }
    let nums = |x: &crate::json::Value| -> Option<Vec<u32>> { x.as_array()?.iter().map(|n| n.as_u32()).collect() };
    let mut units = Vec::new();
    for u in v.get("units").and_then(|x| x.as_array()).ok_or("no units")? {
        match nums(u).as_deref() {
            Some(&[first, n]) if (n as usize == program::BASE || n as usize == program::COUNT) && first >= group::COUNT && first + n <= groups => units.push((first, n)),
            _ => return Err("a bad unit".into()),
        }
    }
    let mut runs = Vec::new();
    for r in v.get("runs").and_then(|x| x.as_array()).ok_or("no runs")? {
        match nums(r).as_deref() {
            Some(&[g, f0, f1, u]) if g <= group::FACE && (u as usize) < units.len() => runs.push((g, f0 as i32, f1 as i32, u)),
            _ => return Err("a bad run".into()),
        }
    }
    let menu = match v.get("menu") {
        None => None,
        Some(m) => {
            let first = m.get("first").and_then(|x| x.as_u32()).ok_or("menu without first")?;
            if first + menu::WORDS > groups {
                return Err("menu groups past the set".into());
            }
            let mut words = Vec::new();
            for lang in ["us", "uk", "fr", "gr", "it", "sp"] {
                if let Some(a) = m.get("words").and_then(|w| w.get(lang)).and_then(|x| x.as_array()) {
                    let mut list = Vec::new();
                    for e in a {
                        match nums(e).as_deref() {
                            Some(&[id, g]) if g < groups && id <= u16::MAX as u32 => list.push((id as u16, g)),
                            _ => return Err("a bad menu word".into()),
                        }
                    }
                    words.push((lang.to_string(), list));
                }
            }
            let tab_icon = matches!(m.get("tab_icon"), Some(crate::json::Value::Bool(true)));
            Some(MenuGroups { first, tab_icon, words })
        }
    };
    let target = match v.get("target") {
        None => None,
        Some(t) => {
            let first = t.get("first").and_then(|x| x.as_u32()).ok_or("target without first")?;
            if first + target::COUNT > groups {
                return Err("target groups past the set".into());
            }
            Some(first)
        }
    };
    let party = match v.get("party").map(|p| nums(p)) {
        None => None,
        Some(Some(p)) if p.len() == 2 && p.iter().all(|&g| g < groups) => Some([p[0], p[1]]),
        Some(_) => return Err("a bad party entry".into()),
    };
    let cursor = match v.get("cursor").map(|c| c.as_u32()) {
        None => None,
        Some(Some(c)) if c < groups => Some(c),
        Some(_) => return Err("a bad cursor entry".into()),
    };
    let level_up = match v.get("level_up").map(|c| c.as_u32()) {
        None => None,
        Some(Some(c)) if c < groups => Some(c),
        Some(_) => return Err("a bad level_up entry".into()),
    };
    Ok(Manifest { groups, languages, motion: Motion { programs, units, runs }, menu, target, party, cursor, level_up })
}

// -------------------------------------------------------------------------------------------------
// H2: Sora's gauge cluster. KH1's rules (Checked (decomp) unless tagged), on Re:CoM's raw HP
// (decision 39) and KH1's MP (`crate::mp`).

/// KH1 numbers (as the asset step's `kh1_hud::kh1`).
pub mod kh1 {
    /// HP drawn at most (`0x78`), HP filling the ring, HP a straight segment.
    pub const HP_CAP: i32 = 120;
    /// HP filling the ring (270° at 5° a point).
    pub const HP_RING: i32 = 54;
    /// HP a straight segment.
    pub const HP_PER_SEG: i32 = 6;
    /// MP drawn at most (`0x14`); MP filling the ring.
    pub const MP_CAP: i32 = 20;
    /// MP filling the ring.
    pub const MP_RING: i32 = 9;
    /// Charge units drawn at most (`0x258`); units filling the ring.
    pub const CHARGE_CAP: i32 = 600;
    /// Charge units filling the ring.
    pub const CHARGE_RING: i32 = 270;
    /// Straight segment pitch.
    pub const PITCH: f32 = 26.0;
    /// Size-class offsets `0x140527300` (class 0, 1, 2).
    pub const CLASS_OFFSET: [(f32, f32); 3] = [(24.0, 24.0), (0.0, 24.0), (0.0, 0.0)];
    /// Hurt face frames by hit strength (`0x1405270FC`, `0x140527134`, `0x14052726C`).
    pub const HURT_FRAMES: [i32; 3] = [40, 50, 60];
    /// KH1's right widescreen margin at 16:9: ceil(((16 / 9) / (4 / 3) · 0.5 − 0.5) · 512)
    /// (`0x140121210`, rounded up `0x1401703C0`) = ceil(85.33).
    pub const MARGIN_16_9: f32 = 86.0;
}

/// What the cluster shows (KH1's values: Re:CoM's HP as it is, KH1's MP).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View {
    /// HP and max HP (Re:CoM's gauge `+0xA4` / `+0xA8`).
    pub hp: i32,
    /// Max HP.
    pub max_hp: i32,
    /// KH1 MP (MP, max, charge), None without KH1 MP (Combat = Re:CoM): no MP pieces, no label.
    pub mp: Option<(i32, i32, u16)>,
}

/// The held groups' frames (FRAME, HP_BACK, MP_BACK, MP, HP, CHARGE, TICKS), in that order.
pub fn frames(v: &View) -> [(u32, i32); 7] {
    let (mp, max, c) = v.mp.map(|(a, b, c)| (a.clamp(0, kh1::MP_CAP), b.clamp(0, kh1::MP_CAP), (c as i32).min(kh1::CHARGE_CAP))).unwrap_or((0, 0, 0));
    [
        (group::FRAME, v.mp.is_some() as i32),
        (group::HP_BACK, v.max_hp.clamp(0, kh1::HP_CAP)),
        (group::MP_BACK, max),
        (group::MP, mp),
        (group::HP, v.hp.clamp(0, kh1::HP_CAP)),
        (group::CHARGE, c),
        (group::TICKS, max),
    ]
}

/// KH1's size class (`0x1402690D0`): 0 when max HP < 19 and max MP < 4; 1 when max HP < 37 and
/// max MP < 7; else 2.
pub fn size_class(max_hp: i32, max_mp: i32) -> usize {
    if max_hp < 0x25 && max_mp < 7 {
        if max_hp < 0x13 && max_mp < 4 {
            0
        } else {
            1
        }
    } else {
        2
    }
}

/// KH1's danger test (`0x14026E180`): HP > 0 and HP / max < 0.2 when max > 44, else HP < 9.
pub fn danger(hp: i32, max: i32) -> bool {
    if hp <= 0 {
        return false;
    }
    if max > 0x2C {
        (hp as f32) / (max as f32) < 0.2
    } else {
        hp < 9
    }
}

/// KH1's hit strength (`0x14029BBD0` → `0x140268D60`): new / old HP < 0.5 → 2, < 0.8 → 1, < 1 → 0;
/// None when HP didn't fall.
pub fn hurt_strength(old: i32, new: i32) -> Option<usize> {
    if old <= 0 || new >= old {
        return None;
    }
    let r = new as f32 / old as f32;
    Some(if r < 0.5 {
        2
    } else if r < 0.8 {
        1
    } else {
        0
    })
}

/// The hurt face group: expression 1 for 40 / 50 / 60 frames, then 2 (danger) or 0.
pub fn hurt_group(strength: usize, danger: bool) -> u32 {
    group::HURT + 2 * strength.min(2) as u32 + danger as u32
}

/// The face's held expression: 2 in danger, else 0 (`0x14026D570`).
pub fn face_frame(danger: bool) -> i32 {
    if danger {
        2
    } else {
        0
    }
}

/// KH1's MP fade cells (`0x14026C6E0`: the ring part only, 10 frames): gained cells (black
/// fading off them) or lost ones (fading to black).
pub fn mp_fade(old: i32, new: i32) -> Vec<u32> {
    let (a, b) = (old.clamp(0, kh1::MP_RING), new.clamp(0, kh1::MP_RING));
    if b > a {
        (a..b).map(|j| group::MP_GAIN + j as u32).collect()
    } else {
        (b..a).map(|j| group::MP_LOSS + j as u32).collect()
    }
}

/// The charge change (`0x14026A770`: alpha fade of the changed ring part, 10 frames): the main
/// layer's frame and the fade pieces ([30 j, 30 j + t] each; the first one also covers the cell's
/// part below the change, drawn the same under it). Past the ring no fade (KH1's straight part).
pub fn charge_plan(old: i32, new: i32) -> (i32, Vec<u32>) {
    let (old, new) = (old.clamp(0, kh1::CHARGE_CAP), new.clamp(0, kh1::CHARGE_CAP));
    let ring = kh1::CHARGE_RING;
    if old == new || old.min(new) >= ring {
        return (new, Vec::new());
    }
    let (lo, hi, gain) = if new > old { (old, new.min(ring), true) } else { (new, old.min(ring), false) };
    let mut pieces = Vec::new();
    let mut j = lo / 30;
    while 30 * j < hi {
        let t = (hi - 30 * j).min(30);
        if t > 0 && 30 * j + t > lo {
            pieces.push(if gain { group::CHARGE_GAIN } else { group::CHARGE_LOSS } + (30 * j + t - 1) as u32);
        }
        j += 1;
    }
    // A gain shows the old value under its fading-in pieces (they hold); crossing the ring's end
    // shows the new value at once (the straight part never fades).
    let main = if gain && new <= ring { old } else { new };
    (main, pieces)
}

/// The danger flash pieces of the max-HP arc and segments (`0x14026BCF0`, `0x14026BEE0` in state
/// 2): (group, x offset) each.
pub fn red_pieces(max_hp: i32) -> Vec<(u32, f32)> {
    let m = max_hp.clamp(0, kh1::HP_CAP);
    let a = (m * 5).min(270);
    let mut v = Vec::new();
    for q in 0..3 {
        let k = ((a - 90 * q).min(90)) / 5;
        if k > 0 {
            v.push((group::RED_ARC + 18 * q as u32 + k as u32 - 1, 0.0));
        }
    }
    if m > kh1::HP_RING {
        let (n, rem) = ((m - kh1::HP_RING) / kh1::HP_PER_SEG, (m - kh1::HP_RING) % kh1::HP_PER_SEG);
        for i in 0..n {
            v.push((group::RED_SEG, -kh1::PITCH * i as f32));
        }
        if rem > 0 {
            v.push((group::RED_SEG_PART + rem as u32 - 1, -kh1::PITCH * n as f32));
        }
    }
    v
}

/// KH1's widescreen margin for a right-anchored piece, from the player gauge's x offset
/// (`+0xA0`: 85.0 at 16:9, 0 at 4:3; Re:CoM has no other ratio, Checked (decomp `0x140208360`)).
pub fn right_margin(gauge_xoff: f32) -> f32 {
    if gauge_xoff > 0.0 {
        kh1::MARGIN_16_9
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trip_and_refusals() {
        let ok = r#"{"format": "kh1-recom-assets kh1-hud 3", "set": "kh1hud:00", "groups": 644, "base_groups": 632, "programs": [{"name": "vibe_s", "start": 1, "frames": 10}, {"name": "vibe_ss", "start": 1, "frames": 22}, {"name": "vibe_sss", "start": 1, "frames": 38}, {"name": "fade_out", "start": 0, "frames": 10}, {"name": "fade_in", "start": 0, "frames": 10}, {"name": "slide_s_h", "start": 1, "frames": 20}, {"name": "slide_s_v", "start": 1, "frames": 20}], "units": [[632, 7], [639, 5]], "runs": [[4, 1, 10000000, 0], [4, 2, 2, 1]], "languages": ["FR", "UK"], "atlas": [1024, 512, 2048, 1024], "sources": {}}"#;
        let m = parse_manifest(ok).unwrap();
        assert_eq!(m.groups, 644);
        assert_eq!(m.languages, vec!["FR", "UK"]);
        assert_eq!(m.motion.programs[2], Program { start: 1, frames: 38 });
        assert_eq!(m.motion.units_at(group::HP, 2), vec![0, 1]);
        assert_eq!(m.motion.units_at(group::HP, 3), vec![0]);
        assert_eq!(m.motion.group(1, program::FADE_IN), Some(643));
        assert_eq!(m.motion.group(1, program::SLIDE_H), None);
        assert_eq!(m.motion.group(0, program::SLIDE_V), Some(638));
        assert!(parse_manifest(&ok.replace("\"base_groups\": 632", "\"base_groups\": 631")).is_err());
        assert!(parse_manifest(&ok.replace("[639, 5]", "[639, 6]")).is_err());
        assert!(parse_manifest(&ok.replace("vibe_ss", "vibe_xx")).is_err());
        assert!(parse_manifest(&ok.replace("hud 3", "hud 2")).is_err(), "format 2 has no context tab");
        assert_eq!(m.menu, None);
        let with = ok.replace("\"languages\"", "\"menu\": {\"first\": 630, \"tab_icon\": true, \"words\": {\"us\": [[1, 643], [6, 642]], \"fr\": [[1, 641]]}}, \"languages\"");
        assert!(parse_manifest(&with).is_err(), "menu groups before the fixed ones would run past");
        let with = with.replace("\"groups\": 644", "\"groups\": 700");
        let mm = parse_manifest(&with).unwrap().menu.unwrap();
        assert_eq!((mm.word("fr", 1), mm.word("fr", 6), mm.word("gr", 1), mm.word("us", 2)), (Some(641), Some(642), Some(643), None));
        assert!(mm.tab_icon && menu::TAB == 23 && menu::WORDS == 25);
        assert!(parse_manifest(&ok.replace("kh1hud:00", "Cockpit:00")).is_err());
    }

    #[test]
    fn target_gauge_rules() {
        assert_eq!(target::body(120), 60);
        assert_eq!(target::body(1000), 150);
        assert_eq!(target::bars(0), [0, 0, 0, 0, 0]);
        assert_eq!(target::bars(1), [1, 0, 0, 0, 0]);
        assert_eq!(target::bars(299), [150, 0, 0, 0, 0]);
        assert_eq!(target::bars(420), [150, 60, 0, 0, 0]);
        assert_eq!(target::bars(1500), [150, 150, 150, 150, 150]);
        assert_eq!(target::marker(&target::bars(420)), Some(1));
        assert_eq!(target::marker(&target::bars(200)), None);
    }

    #[test]
    fn group_numbering_matches_the_asset_step() {
        // 8 held + 6 hurt + 2 × 9 MP fades + 2 × 270 charge fades + 54 + 1 + 5 flashes.
        assert_eq!(group::HURT + 6, group::MP_GAIN);
        assert_eq!(group::MP_GAIN + 9, group::MP_LOSS);
        assert_eq!(group::MP_LOSS + 9, group::CHARGE_GAIN);
        assert_eq!(group::CHARGE_GAIN + 270, group::CHARGE_LOSS);
        assert_eq!(group::CHARGE_LOSS + 270, group::RED_ARC);
        assert_eq!(group::RED_ARC + 54, group::RED_SEG);
        assert_eq!(group::RED_SEG_PART + 5, group::COUNT);
    }

    #[test]
    fn frames_follow_kh1_caps() {
        let v = View { hp: 200, max_hp: 300, mp: Some((25, 30, 700)) };
        assert_eq!(frames(&v), [(0, 1), (1, 120), (2, 20), (3, 20), (4, 120), (5, 600), (6, 20)]);
        let v = View { hp: 30, max_hp: 40, mp: None };
        assert_eq!(frames(&v), [(0, 0), (1, 40), (2, 0), (3, 0), (4, 30), (5, 0), (6, 0)]);
    }

    #[test]
    fn classes_danger_and_hurt() {
        assert_eq!(size_class(18, 3), 0);
        assert_eq!(size_class(19, 3), 1);
        assert_eq!(size_class(36, 6), 1);
        assert_eq!(size_class(37, 3), 2);
        assert_eq!(size_class(20, 7), 2);
        assert!(danger(8, 44) && !danger(9, 44));
        assert!(danger(9, 50) && !danger(10, 50));
        assert!(!danger(0, 50));
        assert_eq!(hurt_strength(100, 85), Some(0));
        assert_eq!(hurt_strength(100, 79), Some(1));
        assert_eq!(hurt_strength(100, 49), Some(2));
        assert_eq!(hurt_strength(100, 100), None);
        assert_eq!(hurt_group(1, true), group::HURT + 3);
    }

    #[test]
    fn mp_and_charge_fades() {
        assert_eq!(mp_fade(3, 5), vec![group::MP_GAIN + 3, group::MP_GAIN + 4]);
        assert_eq!(mp_fade(12, 8), vec![group::MP_LOSS + 8]);
        assert!(mp_fade(10, 12).is_empty());
        // Gain 25 → 65: cells [0, 30] (piece t = 30) and [30, 60] and [60, 65]; main stays 25.
        let (m, p) = charge_plan(25, 65);
        assert_eq!(m, 25);
        assert_eq!(p, vec![group::CHARGE_GAIN + 29, group::CHARGE_GAIN + 59, group::CHARGE_GAIN + 64]);
        // Loss 65 → 40: [30, 60] and [60, 65] fade out; main shows 40.
        let (m, p) = charge_plan(65, 40);
        assert_eq!(m, 40);
        assert_eq!(p, vec![group::CHARGE_LOSS + 59, group::CHARGE_LOSS + 64]);
        // Crossing 270: no wait for the straight part.
        assert_eq!(charge_plan(260, 300).0, 300);
        assert_eq!(charge_plan(300, 330), (330, vec![]));
    }

    #[test]
    fn red_flash_covers_the_max_arc() {
        assert_eq!(red_pieces(10), vec![(group::RED_ARC + 9, 0.0)]);
        let v = red_pieces(70);
        assert_eq!(&v[..3], &[(group::RED_ARC + 17, 0.0), (group::RED_ARC + 35, 0.0), (group::RED_ARC + 53, 0.0)]);
        assert_eq!(&v[3..], &[(group::RED_SEG, 0.0), (group::RED_SEG, -26.0), (group::RED_SEG_PART + 3, -52.0)]);
        assert!(red_pieces(54).len() == 3);
    }
}
