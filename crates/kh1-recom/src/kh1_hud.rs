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
}
