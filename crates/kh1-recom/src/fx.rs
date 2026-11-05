//! M13 stage 3: KH1's effects and sounds for Sora's combos and finishers (`docs/recom-m13-plan.md`
//! § Stage 3 built). The testable part.
//!
//! The asset step (`kh1-recom-assets` `kh1_fx`) writes KH1's trail, finisher effects and swing sounds
//! as Re:CoM's own `.EET` / `.SES` events at the variant [`Manifest::variant`] on the moves it covers,
//! and lists them in `kh1-fx.json`. Re:CoM's motion start keeps the events of the variant the motion
//! request passes (`0x1401A3470` → `0x14017A240` / `0x140239260`; the Keyblade plays the same index
//! with the same variants, `0x1401F7180`). Checked (decomp). So the DLL's only runtime part is the
//! two variant arguments of the motion requests it already makes for a KH1 move ([`variants`]: `play`
//! and the swing's row pick, `win::attack::apply_row`), and
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
    /// KH1 requests with sound events at the variant (the Kingdom Key's).
    pub sounds: Vec<u16>,
    /// M13 stage 4: the other Keyblades' sound variants and the requests each has events for (sorted).
    pub sound_variants: Vec<(i32, Vec<u16>)>,
    /// M13 stage 4: the KH1 Keyblade groups with their own files.
    pub keyblades: Vec<KeybladeFx>,
    /// Every request the step wrote KH1 events for (trails, effects or sounds at any variant): KH1's
    /// moves. At Effects = KH1 they all pass the EET variant, so Re:CoM's own events of a Re:CoM clip
    /// a KH1 move plays (e.g. `p001_103`'s Sonic Rave effects on request 208) never show (0.131.0).
    pub moves: Vec<u16>,
    /// The finisher effects on Sora's or the blade's clip (`effects`: request → KH1 effect ids), for
    /// the per-battle log line (`win::fx`).
    pub finishers: Vec<(u16, Vec<u32>)>,
}

/// One KH1 Keyblade group's own files (M13 stage 4, `kh1-fx.json` `keyblades`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeybladeFx {
    /// Re:CoM card kind.
    pub card: u16,
    /// Its PLAYER group.
    pub group: u16,
    /// Its `.EET` (`PLAYER/00GG/PL00GG.EET`).
    pub eet: String,
    /// Its `.EFF` (`PLAYER/00GG/PL00GG.EFF`).
    pub eff: String,
    /// Its sounds' SES variant (None: the Kingdom Key's KH1 sounds).
    pub sound_variant: Option<i32>,
}

impl Manifest {
    /// Group `group`'s own files, when the step wrote them.
    pub fn keyblade_files(&self, group: u16) -> Option<&KeybladeFx> {
        self.keyblades.iter().find(|k| k.group == group)
    }
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
    let mut finishers: Vec<(u16, Vec<u32>)> = Vec::new();
    for e in v.get("effects").and_then(|a| a.as_array()).unwrap_or(&[]) {
        let Some(req) = e.get("request").and_then(|x| x.as_u32()).filter(|&x| x <= u16::MAX as u32) else { continue };
        let ids: Vec<u32> = e.get("ids").and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|x| x.as_u32()).collect()).unwrap_or_default();
        match finishers.iter_mut().find(|f| f.0 == req as u16) {
            Some(f) => f.1.extend(ids),
            None => finishers.push((req as u16, ids)),
        }
    }
    for f in &mut finishers {
        f.1.sort_unstable();
        f.1.dedup();
    }
    finishers.sort_by_key(|f| f.0);
    // Sound rows by variant (a row without one is the Kingdom Key's, stage 3's manifests).
    let mut sounds: Vec<u16> = Vec::new();
    let mut sound_variants: Vec<(i32, Vec<u16>)> = Vec::new();
    for r in v.get("sounds").and_then(|a| a.as_array()).unwrap_or(&[]) {
        let Some(req) = r.get("request").and_then(|x| x.as_u32()).filter(|&x| x <= u16::MAX as u32) else { continue };
        let var = r.get("variant").and_then(|x| x.as_f64()).map_or(variant, |x| x as i32);
        if var == variant {
            sounds.push(req as u16);
        } else if var > 11 {
            match sound_variants.iter_mut().find(|s| s.0 == var) {
                Some(s) => s.1.push(req as u16),
                None => sound_variants.push((var, vec![req as u16])),
            }
        }
    }
    sounds.sort_unstable();
    sounds.dedup();
    for s in &mut sound_variants {
        s.1.sort_unstable();
        s.1.dedup();
    }
    sound_variants.sort_by_key(|s| s.0);
    let mut moves: Vec<u16> = effects.iter().chain(&sounds).copied().chain(sound_variants.iter().flat_map(|s| s.1.iter().copied())).collect();
    moves.sort_unstable();
    moves.dedup();
    let mut keyblades = Vec::new();
    for k in v.get("keyblades").and_then(|a| a.as_array()).unwrap_or(&[]) {
        let n = |key: &str| k.get(key).and_then(|x| x.as_u32()).filter(|&x| x <= u16::MAX as u32).map(|x| x as u16);
        let t = |key: &str| k.get(key).and_then(|x| x.as_str()).map(str::to_string);
        let (Some(card), Some(group), Some(eet), Some(eff)) = (n("card"), n("group"), t("eet"), t("eff")) else {
            return Err(format!("{MANIFEST}: a keyblades row lacks card / group / eet / eff"));
        };
        if eet != crate::keyblade_models::resource_path(group, "EET") || eff != crate::keyblade_models::resource_path(group, "EFF") {
            return Err(format!("{MANIFEST}: group {group}'s files {eet:?} / {eff:?} are not its own paths"));
        }
        let sound_variant = k.get("sound_variant").and_then(|x| x.as_f64()).map(|x| x as i32).filter(|&x| x > 11);
        keyblades.push(KeybladeFx { card, group, eet, eff, sound_variant });
    }
    Ok(Manifest { variant, trail_fade, effects, sounds, sound_variants, keyblades, moves, finishers })
}

/// The EET and SES variants the motion request passes for KH1 request `request`: KH1's for a move the
/// manifest covers when its toggle is KH1, else 0 (Re:CoM's own events). The EET variant goes to every
/// KH1 move the step wrote events for ([`Manifest::moves`]), with effects of its own or not: KH1 shows
/// none of Re:CoM's effects on its moves (0.131.0). `ses_other` is a variant a
/// caller already chose (a cast's voice): it wins. `card`: the equipped Keyblade (M13 stage 4): its own
/// sound variant when it has events for the request; else the Kingdom Key's (the KH2 Keyblades, a
/// Keyblade whose `.se` wasn't found); else 0. The EET variant is the same for every Keyblade (each
/// group's own `.EET` carries its trail at it).
pub fn variants(m: Option<&Manifest>, request: u16, effects: Mode, sounds: Mode, ses_other: i32, card: u16) -> (i32, i32) {
    let Some(m) = m else { return (0, ses_other) };
    let eet = if effects == Mode::Kh1 && m.moves.binary_search(&request).is_ok() { m.variant } else { 0 };
    let own = m.keyblades.iter().find(|k| k.card == card).and_then(|k| k.sound_variant).filter(|v| m.sound_variants.iter().any(|s| s.0 == *v && s.1.binary_search(&request).is_ok()));
    let ses = if ses_other != 0 {
        ses_other
    } else if sounds != Mode::Kh1 {
        0
    } else if let Some(v) = own {
        v
    } else if m.sounds.binary_search(&request).is_ok() {
        m.variant
    } else {
        0
    };
    (eet, ses)
}

/// Whether `ses` is one of KH1's sound variants (the Kingdom Key's or another Keyblade's).
pub fn is_kh1_sound_variant(m: &Manifest, ses: i32) -> bool {
    ses == m.variant || m.sound_variants.iter().any(|s| s.0 == ses)
}

/// Whether a list-B event's fade is the KH1 trail's (exactly the manifest's value).
pub fn is_kh1_trail_fade(m: &Manifest, fade: f32) -> bool {
    fade == m.trail_fade
}

/// An EET event Re:CoM bound at a motion start (`win::fx`, read from the entity's EET player).
#[derive(Clone, Debug, PartialEq)]
pub struct Bound {
    /// List A (one-node effect) or B (trail).
    pub list: char,
    pub dpx: String,
    pub id: i32,
    pub variant: i32,
    /// What Re:CoM has under the set's name (None: not looked up).
    pub set: Option<SetInfo>,
}

/// A set as Re:CoM has it loaded.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SetInfo {
    pub loaded: bool,
    /// The set's effect ids.
    pub entries: Vec<u32>,
    /// A `.ese` of the same name is loaded.
    pub ese: bool,
    /// The event's effect's sound in that `.ese` (Re:CoM id).
    pub sound: Option<u32>,
}

/// The per-battle swing line (0.131.0): the variants the motion request passed and the events Re:CoM
/// bound for Sora and the Keyblade, each with its set as loaded.
#[allow(clippy::too_many_arguments)]
pub fn swing_line(battle: u32, request: u16, row: u8, finisher: Option<&[u32]>, eet: i32, ses: i32, card: u16, sora: Option<&(u32, Vec<Bound>)>, weapon: Option<&(u32, Vec<Bound>)>) -> String {
    let events = |b: Option<&(u32, Vec<Bound>)>| match b {
        None => "no EET player".to_string(),
        Some((g, evs)) => {
            let list = if evs.is_empty() {
                "nothing bound".to_string()
            } else {
                evs.iter()
                    .map(|e| {
                        let set = match &e.set {
                            None => String::new(),
                            Some(s) if !s.loaded => " [set NOT loaded]".to_string(),
                            Some(s) => format!(
                                " [set loaded, {} entries, id {}{}{}]",
                                s.entries.len(),
                                if s.entries.contains(&(e.id as u32)) { "in it" } else { "MISSING" },
                                if s.ese { ", .ese loaded" } else { "" },
                                s.sound.map_or(String::new(), |x| format!(", sound {x}"))
                            ),
                        };
                        format!("{} {} {} at {}{set}", e.list, e.dpx, e.id, e.variant)
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            format!("EET group {g}: {list}")
        }
    };
    format!(
        "fx (M13): battle {battle}, KH1 request {request} on row {row:#x}{}: EET variant {eet}, SES variant {ses}, Keyblade card {card:#x}; Sora's {}; the Keyblade's {}",
        finisher.map_or(" (the battle's first swing)".to_string(), |ids| format!(" (finisher, KH1 effects {ids:?})")),
        events(sora),
        events(weapon)
    )
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
        eprintln!("variant {}, effects {:?}, sounds {:?}, other variants {:?}, keyblades {:?}", m.variant, m.effects, m.sounds, m.sound_variants, m.keyblades);
        // Every finisher passes KH1's EET variant at Effects = KH1, whichever Keyblade (each group's own
        // EET carries its events at it); the SES variant is the equipped Keyblade's own where it has one.
        let k = Mode::Kh1;
        for f in &m.finishers {
            for kb in &m.keyblades {
                let (eet, ses) = variants(Some(&m), f.0, k, k, 0, kb.card);
                assert_eq!(eet, m.variant, "request {} card {}", f.0, kb.card);
                assert!(is_kh1_sound_variant(&m, ses), "request {} card {}: SES {ses}", f.0, kb.card);
            }
            assert_eq!(variants(Some(&m), f.0, k, k, 0, 0x13).0, m.variant, "a KH2 Keyblade");
        }
    }

    const J4: &str = "{\n  \"format\": \"kh1-recom-assets fx 1\",\n  \"variant\": 1301,\n  \"trail_fade\": -1301.0,\n  \"trails\": [\n    {\"request\": 200, \"clip\": \"w001_055\", \"start\": 4, \"end\": 24}\n  ],\n  \"effects\": [\n  ],\n  \"effects_left_out\": [\n  ],\n  \"sounds\": [\n    {\"request\": 200, \"variant\": 1301, \"clip\": \"p001_055\", \"kh1\": [11031], \"recom_whoosh_for\": []},\n    {\"request\": 201, \"variant\": 1301, \"clip\": \"p001_056\", \"kh1\": [11032], \"recom_whoosh_for\": []},\n    {\"request\": 200, \"variant\": 1302, \"clip\": \"p001_055\", \"kh1\": [11431], \"recom_whoosh_for\": []}\n  ],\n  \"keyblades\": [\n    {\"card\": 2, \"group\": 21, \"set\": \"kh1_ex_5030.dpx\", \"eet\": \"PLAYER/0021/PL0021.EET\", \"eff\": \"PLAYER/0021/PL0021.EFF\", \"reach\": [-0.8022, 0.0000], \"sound_variant\": 1302},\n    {\"card\": 3, \"group\": 22, \"set\": \"kh1_ex_5060.dpx\", \"eet\": \"PLAYER/0022/PL0022.EET\", \"eff\": \"PLAYER/0022/PL0022.EFF\", \"reach\": [-0.8233, 0.0000], \"sound_variant\": null}\n  ],\n  \"sources\": {}\n}\n";

    #[test]
    fn each_keyblade_plays_its_own_sounds_where_it_has_them() {
        let m = parse(J4).unwrap();
        assert_eq!(m.sounds, vec![200, 201]);
        assert_eq!(m.sound_variants, vec![(1302, vec![200])]);
        assert_eq!(m.keyblade_files(21).map(|k| (k.card, k.sound_variant)), Some((2, Some(1302))));
        assert_eq!(m.keyblade_files(22).map(|k| k.sound_variant), Some(None));
        assert!(m.keyblade_files(23).is_none());
        let k = Mode::Kh1;
        assert_eq!(variants(Some(&m), 200, k, k, 0, 2), (1301, 1302), "Three Wishes' own");
        assert_eq!(variants(Some(&m), 201, k, k, 0, 2), (1301, 1301), "no event of its own for 201: the Kingdom Key's; a KH1 move: KH1's EET variant");
        assert_eq!(variants(Some(&m), 200, k, k, 0, 3), (1301, 1301), "no .se: the Kingdom Key's");
        assert_eq!(variants(Some(&m), 200, k, k, 0, 0x13), (1301, 1301), "a KH2 Keyblade: the Kingdom Key's");
        assert_eq!(variants(Some(&m), 200, k, Mode::Recom, 0, 2), (1301, 0));
        assert!(is_kh1_sound_variant(&m, 1302) && is_kh1_sound_variant(&m, 1301) && !is_kh1_sound_variant(&m, 4));
        assert!(parse(&J4.replace("PLAYER/0021/PL0021.EET", "PLAYER/0002/PL0002.EET")).is_err(), "only its own paths");
    }

    #[test]
    fn the_swing_line_names_the_variants_and_what_re_com_bound() {
        let set = SetInfo { loaded: true, entries: (0..8).collect(), ese: true, sound: Some(9_000_084) };
        let sora = (1u32, vec![Bound { list: 'A', dpx: "kh1_ex_0010.dpx".into(), id: 3, variant: 1301, set: Some(set) }]);
        let weapon = (31u32, vec![Bound { list: 'B', dpx: "kh1_ex_5070.dpx".into(), id: 5, variant: 1301, set: Some(SetInfo { loaded: true, entries: vec![0, 1, 2, 5], ..Default::default() }) }, Bound { list: 'A', dpx: "kh1_ex_0010.dpx".into(), id: 2, variant: 1301, set: Some(SetInfo::default()) }]);
        let l = swing_line(3, 217, 0x95, Some(&[2, 3]), 1301, 1312, 0xC, Some(&sora), Some(&weapon));
        assert!(l.contains("battle 3, KH1 request 217 on row 0x95 (finisher, KH1 effects [2, 3]): EET variant 1301, SES variant 1312, Keyblade card 0xc"), "{l}");
        assert!(l.contains("Sora's EET group 1: A kh1_ex_0010.dpx 3 at 1301 [set loaded, 8 entries, id in it, .ese loaded, sound 9000084]"), "{l}");
        assert!(l.contains("the Keyblade's EET group 31: B kh1_ex_5070.dpx 5 at 1301 [set loaded, 4 entries, id in it]; A kh1_ex_0010.dpx 2 at 1301 [set NOT loaded]"), "{l}");
        assert!(swing_line(1, 200, 1, None, 0, 0, 1, Some(&(1, vec![])), None).contains("(the battle's first swing): EET variant 0, SES variant 0, Keyblade card 0x1; Sora's EET group 1: nothing bound; the Keyblade's no EET player"));
    }

    #[test]
    fn variants_follow_the_toggles_and_the_covered_moves() {
        let m = parse(J).unwrap();
        let k = Mode::Kh1;
        let r = Mode::Recom;
        assert_eq!(variants(Some(&m), 200, k, k, 0, 1), (1301, 1301));
        assert_eq!(variants(Some(&m), 200, r, k, 0, 1), (0, 1301));
        assert_eq!(variants(Some(&m), 200, k, r, 0, 1), (1301, 0));
        assert_eq!(variants(Some(&m), 215, k, k, 0, 1), (1301, 0), "effects but no KH1 sound");
        assert_eq!(m.finishers, vec![(215, vec![6, 7]), (217, vec![2])]);
        // A move with KH1 sounds but no KH1 effect (208 plays Re:CoM's `p001_103`): KH1's EET variant
        // all the same, so Re:CoM's Sonic Rave effects of that clip don't show (0.131.0).
        let m2 = parse(&J.replace("{\"request\": 200, \"clip\": \"p001_055\", \"kh1\"", "{\"request\": 208, \"clip\": \"p001_103\", \"kh1\"")).unwrap();
        assert_eq!(m2.moves, vec![200, 208, 215, 217]);
        assert_eq!(variants(Some(&m2), 208, k, k, 0, 1), (1301, 1301));
        assert_eq!(variants(Some(&m2), 208, r, r, 0, 1), (0, 0));
        assert_eq!(variants(Some(&m), 201, k, k, 0, 1), (0, 0), "not covered: Re:CoM's events");
        assert_eq!(variants(Some(&m), 200, k, k, 4, 1), (1301, 4), "a cast's voice variant wins");
        assert_eq!(variants(None, 200, k, k, 0, 1), (0, 0), "no manifest: Re:CoM's");
        // At Re:CoM nothing changes.
        assert_eq!(variants(Some(&m), 200, r, r, 0, 1), (0, 0));
    }
}
