//! The MP bar (M7 stage 7.2, decision 34): the testable part.
//!
//! A Re:CoM-style straight bar under Re:CoM's HP bar, placed like Riku's Dark gauge, in KH1's
//! colours (read from the player's KH1 files by the asset step), 26 units per max-MP point. Re:CoM
//! draws it: its 2D code plays the bar's animation groups, which the asset step added to
//! `Cockpit:00` (`kh1-recom-assets` `mp_bar`), on a `C2DSeqCtrl` of the player gauge
//! (`win/mp_bar.rs`). This module turns KH1's MP state into those layers' frames and into KH1's
//! change fades. `docs/recom-m7-mp-bar.md`.
//!
//! KH1's gauge (Steam, Checked (decomp)):
//! - MP (`0x14026C6E0`): the ring up to min(MP × 30, 270)°, then one 26-unit straight segment per
//!   MP past 9 (at most 20 MP).
//! - Charge (`0x14026A770` / `0x14026A970`): a sweep along the same path from 0: c units of the
//!   ring (c ≤ 270), then 26-unit segments per 30 units, the last one partial (c mod 30) / 30.
//!   So charge c sits at c / 30 MP slots, and point k is made when it reaches slot k's end.
//! - Changes fade on the ring part only (≤ 270 units): MP by colour (0 ↔ 128, opaque), charge by
//!   alpha, 12.8 a frame (`0x14027F760` / `0x14027F780`): 10 frames. A new change restarts the
//!   fade from the last drawn value (the one before it is dropped).
//!
//! Here, along one straight bar (frame units = KH1's: 30 per slot):
//! - layers: track (frame 30 × max MP), MP (30 × MP), charge (charge), all at most 600;
//! - a gain of MP: the MP layer shows the new value, a black piece over the gained slots fades out
//!   (screen = colour × t, KH1's rising colour);
//! - a loss of MP: the MP layer shows the new value, a piece of the lost slots fades to black;
//! - a gain of charge: the charge layer keeps the old value, a piece of the gained part fades in
//!   and stays (past 270 units: no fade, it stays at once);
//! - a loss of charge: the charge layer shows the new value, a piece of the lost part fades out.

use crate::json;

/// KH1 fades only the ring part of its gauge: 9 MP × 30 units. Checked (decomp `0x14026C6E0`,
/// `0x14026A770`: `min(…, 270.0)`).
pub const ARC_UNITS: i32 = 270;

/// The manifest's format line (`kh1-recom-assets` `mp_bar::manifest_json`).
pub const FORMAT: &str = "kh1-recom-assets mp-bar 1";

/// Our animation groups after `first_group` (the asset step's `mp_bar::group`).
pub mod group {
    pub const TRACK: u32 = 0;
    pub const MP: u32 = 1;
    pub const CHARGE: u32 = 2;
    pub const MP_IN: u32 = 3;
    pub const MP_OUT: u32 = 4;
    pub const CHARGE_IN: u32 = 5;
    pub const CHARGE_OUT: u32 = 6;
    pub const CHARGE_STATIC: u32 = 7;
    pub const COUNT: u32 = 8;
}

/// The bar's layout, from `kh1-mp-bar.json`.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// The sprite set (`Cockpit:00`).
    pub set: String,
    /// Re:CoM's own group count, and our first group (= it).
    pub base_groups: u32,
    pub first_group: u32,
    pub slots: i32,
    pub units_per_slot: i32,
    pub pitch: i32,
    pub slot_width: i32,
    pub x0: i32,
    pub frame_max: i32,
    pub fade_frames: i32,
}

impl Layout {
    /// The group count the loaded set must have for our groups to be there.
    pub fn total_groups(&self) -> u32 {
        self.first_group + group::COUNT
    }
}

/// Reads `kh1-mp-bar.json`.
pub fn parse_manifest(text: &str) -> Result<Layout, String> {
    let v = json::parse(text)?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(format!("not a \"{FORMAT}\" manifest"));
    }
    let n = |k: &str| v.get(k).and_then(|x| x.as_f64()).ok_or_else(|| format!("no \"{k}\""));
    let i = |k: &str| n(k).map(|x| x as i32);
    let l = Layout {
        set: v.get("set").and_then(|s| s.as_str()).ok_or("no \"set\"")?.to_string(),
        base_groups: n("base_groups")? as u32,
        first_group: n("first_group")? as u32,
        slots: i("slots")?,
        units_per_slot: i("units_per_slot")?,
        pitch: i("pitch")?,
        slot_width: i("slot_width")?,
        x0: i("x0")?,
        frame_max: i("frame_max")?,
        fade_frames: i("fade_frames")?,
    };
    if n("group_count")? as u32 != group::COUNT || l.first_group != l.base_groups {
        return Err("group numbering is not this DLL's".into());
    }
    // KH1's numbers (`kh1-recom-assets` checks them against the player's files).
    if (l.slots, l.units_per_slot, l.pitch, l.frame_max) != (20, 30, 26, 600) || l.slot_width <= 0 || l.slot_width > l.pitch || l.set.is_empty() || l.set.len() > 0x17 {
        return Err(format!("unexpected layout {l:?}"));
    }
    Ok(l)
}

/// KH1's MP state as drawn: (MP, max MP, charge).
pub type View = (i32, i32, u16);

/// A fade piece: which group, and where on the bar (x from the HP gauge's origin, the X scale of
/// one slot-wide piece).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    pub group: u32,
    pub x: f32,
    pub scale: f32,
}

/// What to show for a change.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// The three layers' frames: track, MP, charge (None: leave it, a fade on it runs on).
    pub frames: [Option<i32>; 3],
    /// The MP pieces now (None: keep the running ones).
    pub mp: Option<Vec<Piece>>,
    /// The charge pieces now (None: keep the running ones).
    pub charge: Option<Vec<Piece>>,
}

/// The pieces covering frame units `lo..hi` of the bar, one per slot (partial ones X-scaled).
pub fn pieces(group: u32, lo: i32, hi: i32, l: &Layout) -> Vec<Piece> {
    let u = l.units_per_slot;
    let mut out = Vec::new();
    if hi <= lo {
        return out;
    }
    for i in lo / u..(hi + u - 1) / u {
        let (f0, f1) = (lo.max(u * i), hi.min(u * (i + 1)));
        if f1 > f0 {
            let x = l.x0 as f32 + (l.pitch * i) as f32 + l.slot_width as f32 * (f0 - u * i) as f32 / u as f32;
            out.push(Piece { group, x, scale: (f1 - f0) as f32 / u as f32 });
        }
    }
    out
}

/// The frames and fades for a change from `old` (None: nothing shown yet, or shown again) to `new`.
pub fn plan(old: Option<View>, new: View, l: &Layout) -> Plan {
    let u = l.units_per_slot;
    let cap = |f: i32| f.clamp(0, l.frame_max);
    let (mp, max, charge) = (cap(new.0 * u), cap(new.1 * u), cap(new.2 as i32));
    let Some(old) = old else {
        return Plan { frames: [Some(max), Some(mp), Some(charge)], mp: Some(Vec::new()), charge: Some(Vec::new()) };
    };
    let (mp0, c0) = (cap(old.0 * u), cap(old.2 as i32));
    let arc = ARC_UNITS.min(l.frame_max);
    let mp_pieces = match mp.cmp(&mp0) {
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(pieces(group::MP_IN, mp0, mp.min(arc), l)),
        std::cmp::Ordering::Less => Some(pieces(group::MP_OUT, mp, mp0.min(arc), l)),
    };
    let (charge_frame, charge_pieces) = match charge.cmp(&c0) {
        // Unchanged: the layer and a gain still fading in are left as they are.
        std::cmp::Ordering::Equal => (None, None),
        std::cmp::Ordering::Greater if c0 >= arc => (Some(charge), Some(Vec::new())),
        std::cmp::Ordering::Greater => {
            let mut p = pieces(group::CHARGE_IN, c0, charge.min(arc), l);
            p.extend(pieces(group::CHARGE_STATIC, arc, charge, l));
            (Some(c0), Some(p))
        }
        std::cmp::Ordering::Less => (Some(charge), Some(pieces(group::CHARGE_OUT, charge, c0.min(arc), l))),
    };
    Plan { frames: [Some(max), Some(mp), charge_frame], mp: mp_pieces, charge: charge_pieces }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
  "format": "kh1-recom-assets mp-bar 1",
  "set": "Cockpit:00",
  "base_groups": 151,
  "first_group": 151,
  "group_count": 8,
  "slots": 20,
  "units_per_slot": 30,
  "pitch": 26,
  "slot_width": 24,
  "mp_height": 12,
  "charge_height": 4,
  "x0": 1,
  "y0": -10,
  "frame_max": 600,
  "fade_frames": 10,
  "files": [{"name": "FORM/0002/FO0002.SPR", "patched": true}],
  "sources": {"uitex.bin sha256": "00"}
}"#;

    fn layout() -> Layout {
        parse_manifest(MANIFEST).unwrap()
    }

    #[test]
    fn manifest() {
        let l = layout();
        assert_eq!((l.first_group, l.total_groups(), l.set.as_str(), l.x0), (151, 159, "Cockpit:00", 1));
        assert!(parse_manifest(&MANIFEST.replace("\"pitch\": 26", "\"pitch\": 30")).is_err());
        assert!(parse_manifest(&MANIFEST.replace("mp-bar 1", "mp-bar 2")).is_err());
        assert!(parse_manifest(&MANIFEST.replace("\"first_group\": 151", "\"first_group\": 150")).is_err());
    }

    #[test]
    fn frames_are_kh1_units_and_grow_with_max_mp() {
        let l = layout();
        // Sora at level 1: 3 max MP = 78 units of track (frame 90), 2 MP, 45 charge.
        let p = plan(None, (2, 3, 45), &l);
        assert_eq!(p.frames, [Some(90), Some(60), Some(45)]);
        assert_eq!((p.mp, p.charge), (Some(vec![]), Some(vec![])));
        // 26 units per max-MP point: max 4 → 4 slots of track.
        assert_eq!(plan(None, (0, 4, 0), &l).frames[0], Some(120));
        // KH1 draws at most 20; the charge at most 600 (20 × 30).
        assert_eq!(plan(None, (25, 25, 900), &l).frames, [Some(600); 3]);
    }

    #[test]
    fn a_point_made_fades_in_and_the_charge_drops() {
        let l = layout();
        // KH1's refill: MP 1 → 2 when the charge reaches 60, the charge goes 60 → 0.
        let p = plan(Some((1, 3, 60)), (2, 3, 0), &l);
        assert_eq!(p.frames, [Some(90), Some(60), Some(0)]);
        // Black over slot 1 (x = 1 + 26), fading out: KH1's colour rising from 0.
        assert_eq!(p.mp, Some(vec![Piece { group: group::MP_IN, x: 27.0, scale: 1.0 }]));
        // The lost charge (0..60, two slots) fades out.
        assert_eq!(p.charge, Some(vec![Piece { group: group::CHARGE_OUT, x: 1.0, scale: 1.0 }, Piece { group: group::CHARGE_OUT, x: 27.0, scale: 1.0 }]));
    }

    #[test]
    fn charge_gains_fade_in_from_the_old_value_and_partial_slots_scale() {
        let l = layout();
        // A swing hit: +3 at 45 → 48: the charge layer stays at 45, a piece covers 45..48 of slot
        // 1 (from 15/30 of the slot, 3/30 wide).
        let p = plan(Some((2, 3, 45)), (2, 3, 48), &l);
        assert_eq!(p.frames, [Some(90), Some(60), Some(45)]);
        assert_eq!(p.mp, None, "MP unchanged: its fade (if any) runs on");
        assert_eq!(p.charge, Some(vec![Piece { group: group::CHARGE_IN, x: 27.0 + 12.0, scale: 0.1 }]));
        // Counterattack +24 from 50 → 74: rest of slot 1 (50..60) and slot 2 (60..74).
        let p = plan(Some((2, 3, 50)), (2, 3, 74), &l);
        let c = p.charge.unwrap();
        assert_eq!(c.len(), 2);
        assert!((c[0].x - (27.0 + 16.0)).abs() < 1e-4 && (c[0].scale - 10.0 / 30.0).abs() < 1e-6);
        assert!((c[1].x - 53.0).abs() < 1e-4 && (c[1].scale - 14.0 / 30.0).abs() < 1e-6);
    }

    #[test]
    fn only_the_ring_part_fades() {
        let l = layout();
        // MP 9 → 10: past KH1's ring (270): no fade.
        let p = plan(Some((9, 11, 0)), (10, 11, 0), &l);
        assert_eq!((p.frames[1], p.mp), (Some(300), Some(vec![])));
        // MP 8 → 10: slot 8 fades (240..270), slot 9 is at once.
        let p = plan(Some((8, 11, 0)), (10, 11, 0), &l);
        assert_eq!(p.mp, Some(vec![Piece { group: group::MP_IN, x: 1.0 + 26.0 * 8.0, scale: 1.0 }]));
        // Charge 250 → 290: 250..270 fades in, 270..290 stays at once (held), layer at 250.
        let p = plan(Some((8, 11, 250)), (8, 11, 290), &l);
        assert_eq!(p.frames[2], Some(250));
        let c = p.charge.unwrap();
        assert_eq!(c.iter().map(|x| x.group).collect::<Vec<_>>(), vec![group::CHARGE_IN, group::CHARGE_STATIC]);
        assert!((c[1].x - (1.0 + 26.0 * 9.0)).abs() < 1e-4 && (c[1].scale - 20.0 / 30.0).abs() < 1e-6);
        // From 280 up: no fade, the layer shows the new value.
        assert_eq!(plan(Some((8, 11, 280)), (8, 11, 290), &l), Plan { frames: [Some(330), Some(240), Some(290)], mp: None, charge: Some(vec![]) });
    }

    #[test]
    fn spells_paid_fade_out() {
        let l = layout();
        // Thunder (100 = one point) at 3 MP: slot 2 fades to black.
        let p = plan(Some((3, 3, 10)), (2, 3, 10), &l);
        assert_eq!((p.frames[1], p.mp, p.charge), (Some(60), Some(vec![Piece { group: group::MP_OUT, x: 53.0, scale: 1.0 }]), None));
        // ... and the charge layer is not touched (a gain fading in on it runs on).
        assert_eq!(p.frames[2], None);
        // Fire (30 units) at 3 MP, charge 10: KH1 borrows the top point: MP 2, charge 10 - 30 +
        // 90 = 70. The point fades out, the charge rises 10 → 70 from the old value.
        let p = plan(Some((3, 3, 10)), (2, 3, 70), &l);
        assert_eq!(p.frames, [Some(90), Some(60), Some(10)]);
        assert_eq!(p.charge.unwrap().len(), 3);
        // Max MP changes (a Keyblade): the track follows, no fade.
        let p = plan(Some((3, 3, 0)), (3, 6, 0), &l);
        assert_eq!((p.frames, p.mp, p.charge), ([Some(180), Some(90), None], None, None));
    }

    #[test]
    fn pieces_cover_the_range_once() {
        let l = layout();
        for (lo, hi) in [(0, 30), (5, 95), (29, 31), (0, 270), (100, 101)] {
            let p = pieces(9, lo, hi, &l);
            let total: f32 = p.iter().map(|x| x.scale * 30.0).sum();
            assert!((total - (hi - lo) as f32).abs() < 1e-3, "{lo}..{hi}");
            assert!(p.iter().all(|x| x.scale > 0.0 && x.scale <= 1.0));
        }
        assert!(pieces(9, 40, 40, &l).is_empty());
    }
}
