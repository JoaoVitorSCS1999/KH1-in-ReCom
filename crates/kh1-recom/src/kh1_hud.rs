//! The KH1 HUD option (decision 39, `docs/recom-kh1-hud.md`): KH1's HUD elements drawn by Re:CoM's
//! 2D code from the `kh1hud` sprite set the asset step adds to Re:CoM's `FO0002.SPR` / `.RTM`
//! (`kh1-recom-assets` `kh1_hud`). This module holds what is pure: the manifest check and the
//! group numbering (the same as the asset step's).

/// The manifest's format line (as `kh1_recom_assets::kh1_hud::FORMAT`).
pub const FORMAT: &str = "kh1-recom-assets kh1-hud 1";
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
    /// Groups in all.
    pub const COUNT: u32 = 632;
}

/// What `kh1-hud.json` says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// The set's group count.
    pub groups: u32,
    /// The languages the PC copies were built for.
    pub languages: Vec<String>,
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
    if groups != group::COUNT {
        return Err(format!("{groups} groups, this DLL expects {}", group::COUNT));
    }
    let languages = v.get("languages").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|l| l.as_str().map(str::to_string)).collect()).unwrap_or_default();
    Ok(Manifest { groups, languages })
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
        let ok = r#"{"format": "kh1-recom-assets kh1-hud 1", "set": "kh1hud:00", "groups": 632, "languages": ["FR", "UK"], "atlas": [1024, 512, 2048, 1024], "sources": {}}"#;
        let m = parse_manifest(ok).unwrap();
        assert_eq!(m.groups, 632);
        assert_eq!(m.languages, vec!["FR", "UK"]);
        assert!(parse_manifest(&ok.replace("632", "631")).is_err());
        assert!(parse_manifest(&ok.replace("hud 1", "hud 0")).is_err());
        assert!(parse_manifest(&ok.replace("kh1hud:00", "Cockpit:00")).is_err());
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
