//! M13 stage 3: KH1's effects and sounds for Sora's combos and finishers (`docs/recom-m13-plan.md`
//! § Stage 3 built). The testable part.
//!
//! The asset step (`kh1-recom-assets` `kh1_fx`) writes KH1's trail, finisher effects and swing sounds
//! as Re:CoM's own `.EET` / `.SES` events at the variant [`Manifest::variant`] on the moves it covers,
//! and lists them in `kh1-fx.json`. Re:CoM's motion start keeps the events of the variant the motion
//! request passes (`0x1401A3470` → `0x14017A240` / `0x140239260`; the Keyblade plays the same index
//! with the same variants, `0x1401F7180`). Checked (decomp). So the DLL's only runtime part is the
//! two variant arguments of the motion request it already makes for a KH1 move ([`variants`]), and
//! KH1's trail end ([`is_kh1_trail_fade`], `win::fx`).

use crate::json;
use crate::settings::Mode;

/// The manifest's `format` (`kh1-recom-assets` writes it).
pub const FORMAT: &str = "kh1-recom-assets fx 1";
/// The manifest, next to the other asset-step manifests.
pub const MANIFEST: &str = "kh1-fx.json";

/// What `kh1-fx.json` lists.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Manifest {
    /// The variant of KH1's events.
    pub variant: i32,
    /// The fade that marks a KH1 trail event.
    pub trail_fade: f32,
    /// KH1 requests with effect events (trail or finisher effects) at the variant.
    pub effects: Vec<u16>,
    /// KH1 requests with sound events at the variant.
    pub sounds: Vec<u16>,
}

/// Reads `kh1-fx.json`.
pub fn parse(text: &str) -> Result<Manifest, String> {
    let v = json::parse(text)?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(format!("{MANIFEST}: not a \"{FORMAT}\" manifest (an older asset step? run it again)"));
    }
    let variant = v.get("variant").and_then(|x| x.as_u32()).ok_or_else(|| format!("{MANIFEST}: no variant"))? as i32;
    if variant <= 11 {
        // Re:CoM's own variants on Sora's clips run 0..11 (Checked (files): PL0001 / PL0002.EET, PL0001.SES).
        return Err(format!("{MANIFEST}: variant {variant} is one of Re:CoM's own"));
    }
    let trail_fade = v.get("trail_fade").and_then(|x| x.as_f64()).ok_or_else(|| format!("{MANIFEST}: no trail_fade"))? as f32;
    if trail_fade >= 0.0 {
        return Err(format!("{MANIFEST}: trail_fade {trail_fade} must be negative (Re:CoM's own fades are not)"));
    }
    let requests = |key: &str| -> Vec<u16> {
        let mut r: Vec<u16> = v.get(key).and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|x| x.get("request").and_then(|r| r.as_u32())).filter(|&r| r <= u16::MAX as u32).map(|r| r as u16).collect()).unwrap_or_default();
        r.sort_unstable();
        r.dedup();
        r
    };
    let mut effects = requests("trails");
    effects.extend(requests("effects"));
    effects.sort_unstable();
    effects.dedup();
    Ok(Manifest { variant, trail_fade, effects, sounds: requests("sounds") })
}

/// The EET and SES variants the motion request passes for KH1 request `request`: KH1's for a move the
/// manifest covers when its toggle is KH1, else 0 (Re:CoM's own events). `ses_other` is a variant a
/// caller already chose (a cast's voice): it wins.
pub fn variants(m: Option<&Manifest>, request: u16, effects: Mode, sounds: Mode, ses_other: i32) -> (i32, i32) {
    let Some(m) = m else { return (0, ses_other) };
    let eet = if effects == Mode::Kh1 && m.effects.binary_search(&request).is_ok() { m.variant } else { 0 };
    let ses = if ses_other != 0 {
        ses_other
    } else if sounds == Mode::Kh1 && m.sounds.binary_search(&request).is_ok() {
        m.variant
    } else {
        0
    };
    (eet, ses)
}

/// Whether a list-B event's fade is the KH1 trail's (exactly the manifest's value).
pub fn is_kh1_trail_fade(m: &Manifest, fade: f32) -> bool {
    fade == m.trail_fade
}

#[cfg(test)]
mod tests {
    use super::*;

    const J: &str = "{\n  \"format\": \"kh1-recom-assets fx 1\",\n  \"variant\": 1301,\n  \"trail_fade\": -1301.0,\n  \"trails\": [\n    {\"request\": 200, \"clip\": \"w001_055\", \"start\": 4, \"end\": 24},\n    {\"request\": 217, \"clip\": \"w001_229\", \"start\": 30, \"end\": 56}\n  ],\n  \"effects\": [\n    {\"request\": 217, \"clip\": \"w001_229\", \"ids\": [2]},\n    {\"request\": 215, \"clip\": \"p001_228\", \"ids\": [6, 7]}\n  ],\n  \"effects_left_out\": [\n    {\"request\": 216, \"id\": 0, \"why\": \"x\"}\n  ],\n  \"sounds\": [\n    {\"request\": 200, \"clip\": \"p001_055\", \"kh1\": [11031], \"recom_whoosh_for\": []}\n  ],\n  \"sources\": {}\n}\n";

    #[test]
    fn reads_the_steps_manifest() {
        let m = parse(J).unwrap();
        assert_eq!(m.variant, 1301);
        assert_eq!(m.effects, vec![200, 215, 217]);
        assert_eq!(m.sounds, vec![200]);
        assert!(is_kh1_trail_fade(&m, -1301.0) && !is_kh1_trail_fade(&m, 10.0) && !is_kh1_trail_fade(&m, 0.0));
        assert!(parse(&J.replace("fx 1", "fx 0")).is_err());
        assert!(parse(&J.replace("1301,", "3,")).is_err(), "one of Re:CoM's variants");
        assert!(parse(&J.replace("-1301.0", "10.0")).is_err());
    }

    /// The asset step's real output, when `KH1_FX_JSON` names it.
    #[test]
    fn the_steps_output_parses() {
        let Some(p) = std::env::var_os("KH1_FX_JSON") else { return };
        let m = parse(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert!(!m.effects.is_empty());
        eprintln!("variant {}, effects {:?}, sounds {:?}", m.variant, m.effects, m.sounds);
    }

    #[test]
    fn variants_follow_the_toggles_and_the_covered_moves() {
        let m = parse(J).unwrap();
        let k = Mode::Kh1;
        let r = Mode::Recom;
        assert_eq!(variants(Some(&m), 200, k, k, 0), (1301, 1301));
        assert_eq!(variants(Some(&m), 200, r, k, 0), (0, 1301));
        assert_eq!(variants(Some(&m), 200, k, r, 0), (1301, 0));
        assert_eq!(variants(Some(&m), 215, k, k, 0), (1301, 0), "effects but no KH1 sound");
        assert_eq!(variants(Some(&m), 201, k, k, 0), (0, 0), "not covered: Re:CoM's events");
        assert_eq!(variants(Some(&m), 200, k, k, 4), (1301, 4), "a cast's voice variant wins");
        assert_eq!(variants(None, 200, k, k, 0), (0, 0), "no manifest: Re:CoM's");
        // At Re:CoM nothing changes.
        assert_eq!(variants(Some(&m), 200, r, r, 0), (0, 0));
    }
}
