//! M13 stage 3: Sora's combos and finishers with KH1's effects and sounds, as Re:CoM's own per-motion
//! events (owner, 2026-11-04: "He doesn't have his keyblade trail, gravity break doesnt have it
//! effects/sound, neither does ripple drive or stun impact"; `docs/recom-m13-plan.md` § Stage 3).
//!
//! Re:CoM plays a motion's `.EET` / `.SES` events itself when the motion starts (`0x1401A3470` →
//! `0x14017A240` / `0x140239260`), keeping those whose variant is the one the motion request passes;
//! the Keyblade plays the same motion index with the same variants (`0x1401F7180`). Checked (decomp).
//! So KH1's version of each move is a set of events at the variant [`VARIANT`], and the DLL passes it
//! for the moves this step covers ([`MANIFEST`]) when the toggles say KH1. Re:CoM's own events
//! (variant 0) stay as they are, for Re:CoM.
//!
//! KH1 (Steam; Checked (decomp + files), `docs/recom-m13-kh1-fx.md`):
//! - **Trail**: every row with a trail id (`+2` ≥ 0: all the combo rows, Guard, Counterattack) starts
//!   the weapon's effect 5 on the first frame the motion's type-7 flag is on, after the row's first
//!   frame (`0x1402C1F20`), once; when the flag goes off it stops emitting and its particles finish
//!   (stop byte `+0x91`); a motion change kills it (no flag 0x800). The Kingdom Key's set is
//!   byte-identical to Re:CoM's `ex_5010.dpx` (Checked (files)), so the trail is Re:CoM's own effect 5
//!   on Re:CoM's own three nodes (its `w001_055` record's), at KH1's frames. KH1's end and kill are
//!   given to the DLL by the fade [`TRAIL_FADE`] (Re:CoM's list B can only fade or kill).
//! - **Sounds**: the motion's type-6 keys: op 0x19 = weapon `+0x34` + arg, op 0 = an absolute id, at
//!   their frames (60 Hz, as SES frames); the ops 1..7 footsteps and the voices stay Re:CoM's (stages
//!   8 and voices).
//! - **Finisher effects** (`0x1402A68F0`, at the row's start, Sora's own set `kh1_ex_0010.dpx`):
//!   row 3 Stun Impact 0, 1; row 4 Gravity Break 3 (and 2 on the blade at fraction 0.8, code 0x30,
//!   motion-locked); row 5 Zantetsuken 4, 5; row 7 Ripple Drive 6, 7; all but 2 with 0x800 (they
//!   outlive the motion). Where each goes is its program's attach code (`+0x104`, `0x14029EA00` /
//!   `0x14029EB20`): `0x18001` = Sora's position and facing, placed once (1, 3, 4, 7); bone code 8
//!   (6); `0x8003` = **the target's contact point** (0, 5).

use crate::eet::{self, Point, Record, Trail};
use crate::ses;
use crate::{Error, Result};
use kh1_formats::btltbl::ActionRow;
use kh1_formats::mset::{track, MotionSet};

/// The variant of KH1's events (EET and SES), passed by the DLL for a covered move.
pub const VARIANT: i32 = 1301;
/// A KH1 trail's fade: Re:CoM kills an effect stopped with a fade ≤ 0 (`0x140254D60`), which is KH1's
/// motion-change kill; at the window's end the DLL's wrapper of the list-B stop sees this value and
/// stops the emission instead (KH1's stop byte). Re:CoM's own fades are 10..15 (Checked (files)).
pub const TRAIL_FADE: f32 = -1301.0;
/// The manifest the step writes.
pub const MANIFEST: &str = "kh1-fx.json";
/// The manifest's `format`.
pub const FORMAT: &str = "kh1-recom-assets fx 1";
/// The Keyblade's effect set and its trail id (KH1 row `+2`).
pub const TRAIL_SET: &str = "ex_5010.dpx";
/// Re:CoM's own trail record the nodes are taken from (combo swing 1's Keyblade clip).
pub const TRAIL_TEMPLATE: &str = "w001_055";
/// Sora's KH1 effect set (stage 1).
pub const SORA_SET: &str = crate::effects::SORA_SET;
/// Re:CoM's weapon bones the blade runs along (`wep0`, `k9`: KH1's 282 / 283; Checked (files)).
pub const BLADE_BONES: (usize, usize) = (2, 3);
/// Re:CoM's Sora bone standing for KH1's bone 8 (`belt1`, Checked (files): KH1 7 and 8 coincide at rest;
/// `kh1-recom` `attack::recom_body_bone`).
pub const RECOM_BONE_FOR_KH1_8: i32 = 4;
/// EET list-A flag bit 0: detached (spawned in the world from the bone's matrix, not following).
/// Checked (decomp `0x14017A710`).
pub const DETACHED: u32 = 0x1;
/// EET list-A flag bit 3: set on almost every Re:CoM event (copied; not read by `CEffectMotion`).
pub const FLAG_3: u32 = 0x8;
/// EET list-A flag bit 5: survives the motion change (`0x14017A430` leaves flag-0x21 events alone).
pub const LINGER: u32 = 0x20;

/// One KH1 effect a finisher row starts, and where (`0x1402A68F0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    /// `0x18001`: Sora's position and facing, once (the effect then stays in the world).
    SoraOnce,
    /// Bone code 8 (KH1's bone 8), following it.
    Bone8,
    /// Code 0x30 at fraction 0.8: on the blade (the Keyblade's clip), following it.
    Blade(f32),
    /// `0x8003`: the target's contact point. No EET event can place an effect at another actor.
    Target,
}

/// The finisher rows' effects: row, effect id, place, outlives the motion (0x800). Checked (decomp:
/// `0x1402A68F0`'s table `03 00 01 04 03 FF 05 04 05 07 06 07`, the extra id 2 call with r9d 0x30 and
/// 0.8 at `0x1402A6BDA`; files: the programs' `+0x104`).
pub const FINISHER_FX: [(usize, u32, Place, bool); 8] = [
    (3, 0, Place::Target, true),
    (3, 1, Place::SoraOnce, true),
    (4, 3, Place::SoraOnce, true),
    (4, 2, Place::Blade(0.8), false),
    (5, 4, Place::SoraOnce, true),
    (5, 5, Place::Target, true),
    (7, 6, Place::Bone8, true),
    (7, 7, Place::SoraOnce, true),
];

/// A KH1 sound key of a motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kh1Sound {
    /// Op 0x19: the weapon's sound `arg` (weapon `+0x34` + arg).
    Weapon(u16),
    /// Op 0: sound `id`.
    Absolute(u16),
}

/// One KH1 move the step covers.
#[derive(Clone, Debug, PartialEq)]
pub struct Move {
    /// KH1's request (the row's motion).
    pub request: u16,
    /// The first KH1 row playing it.
    pub row: usize,
    /// Re:CoM's clip for it (`p001_NNN`).
    pub clip: String,
    /// The Keyblade's clip.
    pub weapon: Option<String>,
    /// The trail's start and end frame (60 Hz), when the row has a trail and the motion a type-7 window.
    pub trail: Option<(i32, i32)>,
    /// The sound keys: frame, sound.
    pub sounds: Vec<(i32, Kh1Sound)>,
    /// Whether the row is a finisher (Re:CoM's fallback whoosh).
    pub finisher: bool,
}

/// The first type-7 window of a motion as KH1's trail uses it: the first whole frame ≥ 1 at which the
/// flag is on, and the first whole frame after that at which it is off (the motion's end when it never
/// goes off).
pub fn trail_window(keys: &[(f32, bool)], frames: u32) -> Option<(i32, i32)> {
    let on = |t: f32| keys.iter().filter(|k| k.0 <= t).next_back().is_some_and(|k| k.1);
    let start = (1..frames as i32).find(|&f| on(f as f32))?;
    let end = (start + 1..=frames as i32).find(|&f| !on(f as f32)).unwrap_or(frames as i32);
    Some((start, end))
}

/// The moves of Sora's rows that have a trail (KH1 `+2` ≥ 0), each request once (the first row
/// playing it), with Re:CoM's clip names from the motion build (`clip_of(request)`).
pub fn moves(set: &MotionSet, rows: &[ActionRow], clip_of: impl Fn(u16) -> Option<(String, Option<String>)>) -> Vec<Move> {
    let mut out: Vec<Move> = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        if row.trail < 0 || row.motion < 0 {
            continue;
        }
        let request = row.motion as u16;
        if out.iter().any(|m| m.request == request) {
            continue;
        }
        let (Some((clip, weapon)), Some((_, m))) = (clip_of(request), set.motion_for(request)) else { continue };
        let keys: Vec<(f32, bool)> = m.tracks.iter().filter(|t| t.kind == track::TRAIL && t.sub == 0).flat_map(|t| t.keys.iter().map(|k| (k.time, k.value() != 0))).collect();
        let mut sounds = Vec::new();
        for t in m.tracks.iter().filter(|t| t.kind == track::SOUND && t.sub == 0) {
            for k in &t.keys {
                let (op, arg, _) = k.args();
                match op {
                    0x19 => sounds.push((k.time as i32, Kh1Sound::Weapon(arg))),
                    0 => sounds.push((k.time as i32, Kh1Sound::Absolute(arg))),
                    _ => {}
                }
            }
        }
        let finisher = matches!(r, 3..=8 | 15 | 16 | 19 | 22 | 0x3E);
        out.push(Move { request, row: r, clip, weapon, trail: trail_window(&keys, m.frames), sounds, finisher });
    }
    out
}

/// What the step added.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Added {
    /// Requests whose Keyblade clip got a KH1 trail.
    pub trails: Vec<(u16, String, i32, i32)>,
    /// Requests whose clip got KH1 effects (ids).
    pub effects: Vec<(u16, String, Vec<u32>)>,
    /// Effects left out (request, id, why).
    pub effects_out: Vec<(u16, u32, &'static str)>,
    /// Requests whose clip got KH1 sounds: KH1 ids played, KH1 ids not in a Re:CoM bank (Re:CoM's
    /// whoosh kept for those).
    pub sounds: Vec<(u16, String, Vec<u32>, Vec<u32>)>,
}

/// The Keyblade's `.EET` with KH1's trail at [`VARIANT`] for each move with a trail window: one record
/// per Keyblade clip, Re:CoM's own records kept as they are. The nodes are Re:CoM's own trail's
/// ([`TRAIL_TEMPLATE`]).
pub fn weapon_eet(recom: &[u8], moves: &[Move], blade_point: Option<[f32; 3]>, added: &mut Added) -> Result<Vec<u8>> {
    let mut recs = eet::parse(recom)?;
    let template_ev = recs.iter().find(|r| r.name == TRAIL_TEMPLATE).and_then(|r| r.b.first()).copied().ok_or_else(|| Error::Format(format!("EET: no {TRAIL_TEMPLATE} trail to take the nodes from")))?;
    let template = Trail::read(&template_ev);
    if template.dpx != TRAIL_SET || template.id != 5 {
        return Err(Error::Format(format!("EET: {TRAIL_TEMPLATE}'s trail is {} {}, not {TRAIL_SET} 5", template.dpx, template.id)));
    }
    for m in moves {
        let Some(w) = &m.weapon else { continue };
        let mut rec = Record { name: w.clone(), a: Vec::new(), b: Vec::new(), raw_name: None };
        if let Some((start, end)) = m.trail {
            rec.b.push(Trail { start, end, variant: VARIANT, fade: TRAIL_FADE, ..template.clone() }.write(&template_ev)?);
            added.trails.push((m.request, w.clone(), start, end));
        }
        // Gravity Break's effect 2 on the blade (row 4).
        for (row, id, place, _) in FINISHER_FX {
            if let (true, Place::Blade(_), Some(p)) = (row == m.row, place, blade_point) {
                rec.a.push(Point { dpx: SORA_SET.into(), id: id as i32, bone: BLADE_BONES.0 as i32, offset: p, start: 0, end: -1, rot: [0.0; 3], scale: [1.0; 3], flags: FLAG_3, fade: 0.0, variant: VARIANT }.write()?);
                if let Some(e) = added.effects.iter_mut().find(|e| e.0 == m.request) {
                    e.2.push(id);
                } else {
                    added.effects.push((m.request, w.clone(), vec![id]));
                }
            }
        }
        if !rec.a.is_empty() || !rec.b.is_empty() {
            recs.push(rec);
        }
    }
    eet::write(&recs)
}

/// Sora's `.EET` with KH1's finisher effects at [`VARIANT`] (one record per clip; Re:CoM's own kept).
/// The effects placed at the target are left out (no EET event places at another actor: noted).
pub fn sora_eet(recom: &[u8], moves: &[Move], added: &mut Added) -> Result<Vec<u8>> {
    let mut recs = eet::parse(recom)?;
    for m in moves {
        let mut rec = Record { name: m.clip.clone(), a: Vec::new(), b: Vec::new(), raw_name: None };
        let mut ids = Vec::new();
        for (row, id, place, linger) in FINISHER_FX {
            if row != m.row {
                continue;
            }
            let keep = if linger { LINGER } else { 0 };
            let (bone, flags) = match place {
                Place::SoraOnce => (0, DETACHED | FLAG_3 | keep),
                Place::Bone8 => (RECOM_BONE_FOR_KH1_8, FLAG_3 | keep),
                Place::Blade(_) => continue,
                Place::Target => {
                    added.effects_out.push((m.request, id, "placed at the target's contact point (0x8003): no EET event places an effect at another actor"));
                    continue;
                }
            };
            rec.a.push(Point { dpx: SORA_SET.into(), id: id as i32, bone, offset: [0.0; 3], start: 0, end: -1, rot: [0.0; 3], scale: [1.0; 3], flags, fade: 0.0, variant: VARIANT }.write()?);
            ids.push(id);
        }
        if !rec.a.is_empty() {
            added.effects.push((m.request, m.clip.clone(), ids));
            recs.push(rec);
        }
    }
    eet::write(&recs)
}

fn u32_of(e: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(e[o..o + 4].try_into().unwrap())
}

/// Sora's `.SES` entries with KH1's sounds at [`VARIANT`] for each move with a sound key: the clip's
/// own variant-0 events (footsteps, voices: Re:CoM's) copied to the variant, except its whooshes (group
/// 1), then each KH1 key as an event at its frame (Re:CoM's whoosh event as the template: its bone,
/// offset and fade). A KH1 sound with no Re:CoM bank (`map` None) keeps Re:CoM's whoosh for it
/// (`ses::whoosh_index`). Moves where no KH1 sound maps get nothing (Re:CoM's events play).
pub fn sora_ses(entries: &mut Vec<ses::Entry>, moves: &[Move], weapon_base: u32, map: impl Fn(u32) -> Option<u32>, added: &mut Added) -> Result<()> {
    let whoosh = entries.iter().find(|e| e.name() == "p001_060").and_then(|e| e.events.iter().find(|x| u32_of(&x[..], 0) == 1).copied()).ok_or_else(|| Error::Format("SES: p001_060 has no whoosh to copy".into()))?;
    for m in moves {
        if m.sounds.is_empty() {
            continue;
        }
        let ids: Vec<(i32, u32, u16)> = m
            .sounds
            .iter()
            .map(|&(f, s)| match s {
                Kh1Sound::Weapon(a) => (f, weapon_base + a as u32, a),
                Kh1Sound::Absolute(id) => (f, id as u32, u16::MAX),
            })
            .collect();
        if !ids.iter().any(|x| map(x.1).is_some()) {
            continue;
        }
        if !entries.iter().any(|e| e.name() == m.clip) {
            // A clip with no entry yet: one of its own (the head as Re:CoM's `p001_055`'s, renamed).
            let mut head = entries.iter().find(|e| e.name() == "p001_055").map(|e| e.head).ok_or_else(|| Error::Format("SES: no p001_055".into()))?;
            head[..16].fill(0);
            let n = m.clip.len().min(15);
            head[..n].copy_from_slice(&m.clip.as_bytes()[..n]);
            entries.push(ses::Entry { head, events: Vec::new() });
        }
        let Some(entry) = entries.iter_mut().find(|e| e.name() == m.clip) else { continue };
        let mut new = Vec::new();
        for e in entry.events.iter().filter(|x| u32_of(&x[..], 0x20) == 0 && u32_of(&x[..], 0) != 1) {
            let mut c = *e;
            c[0x20..0x24].copy_from_slice(&VARIANT.to_le_bytes());
            new.push(c);
        }
        let (mut played, mut missing) = (Vec::new(), Vec::new());
        for (f, id, arg) in ids {
            let (group, index) = match map(id) {
                Some(r) => {
                    played.push(id);
                    (r / 1000, r % 1000)
                }
                None if arg != u16::MAX => {
                    missing.push(id);
                    (1, ses::whoosh_index(arg, m.finisher))
                }
                None => {
                    missing.push(id);
                    continue;
                }
            };
            let mut e = whoosh;
            e[0..4].copy_from_slice(&group.to_le_bytes());
            e[4..8].copy_from_slice(&index.to_le_bytes());
            e[0x18..0x1C].copy_from_slice(&f.to_le_bytes());
            e[0x1C..0x20].copy_from_slice(&(-1i32).to_le_bytes());
            e[0x20..0x24].copy_from_slice(&VARIANT.to_le_bytes());
            new.push(e);
        }
        entry.events.retain(|x| u32_of(&x[..], 0x20) as i32 != VARIANT);
        entry.events.extend(new);
        added.sounds.push((m.request, m.clip.clone(), played, missing));
    }
    Ok(())
}

/// The manifest: the variant, the trail marker, and per move what it got.
pub fn manifest_json(a: &Added, sources: &[(&str, String)]) -> String {
    use std::fmt::Write;
    let mut j = String::new();
    let _ = writeln!(j, "{{");
    let _ = writeln!(j, "  \"format\": \"{FORMAT}\",");
    let _ = writeln!(j, "  \"variant\": {VARIANT},");
    let _ = writeln!(j, "  \"trail_fade\": {TRAIL_FADE:.1},");
    let rows = |v: Vec<String>| v.join(",\n");
    let _ = writeln!(j, "  \"trails\": [\n{}\n  ],", rows(a.trails.iter().map(|(r, w, s, e)| format!("    {{\"request\": {r}, \"clip\": {w:?}, \"start\": {s}, \"end\": {e}}}")).collect()));
    let _ = writeln!(j, "  \"effects\": [\n{}\n  ],", rows(a.effects.iter().map(|(r, c, ids)| format!("    {{\"request\": {r}, \"clip\": {c:?}, \"ids\": {ids:?}}}")).collect()));
    let _ = writeln!(j, "  \"effects_left_out\": [\n{}\n  ],", rows(a.effects_out.iter().map(|(r, id, why)| format!("    {{\"request\": {r}, \"id\": {id}, \"why\": {why:?}}}")).collect()));
    let _ = writeln!(j, "  \"sounds\": [\n{}\n  ],", rows(a.sounds.iter().map(|(r, c, p, m)| format!("    {{\"request\": {r}, \"clip\": {c:?}, \"kh1\": {p:?}, \"recom_whoosh_for\": {m:?}}}")).collect()));
    let src: Vec<String> = sources.iter().map(|(k, v)| format!("{k:?}: {v:?}")).collect();
    let _ = writeln!(j, "  \"sources\": {{{}}}", src.join(", "));
    let _ = writeln!(j, "}}");
    j
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trail_windows_follow_kh1s_first_on_frame_after_the_row_start() {
        // 200: on 4..24 (Checked (files): `kh1-data-values.md` § 4).
        assert_eq!(trail_window(&[(4.0, true), (24.0, false)], 48), Some((4, 24)));
        // On from frame 0: the trail starts on frame 1 (the row's first frame only clears the "new" bit).
        assert_eq!(trail_window(&[(0.0, true), (20.0, false)], 40), Some((1, 20)));
        // Never off: to the motion's end; never on: none.
        assert_eq!(trail_window(&[(2.0, true)], 30), Some((2, 30)));
        assert_eq!(trail_window(&[(0.0, false)], 30), None);
        assert_eq!(trail_window(&[], 30), None);
        // Fractional keys: the first whole frame at or after.
        assert_eq!(trail_window(&[(3.5, true), (10.2, false)], 30), Some((4, 11)));
    }

    fn whoosh_ev(group: u32, index: u32, frame: i32, variant: i32) -> [u8; 0x50] {
        let mut e = [0u8; 0x50];
        e[0..4].copy_from_slice(&group.to_le_bytes());
        e[4..8].copy_from_slice(&index.to_le_bytes());
        e[0x18..0x1C].copy_from_slice(&frame.to_le_bytes());
        e[0x20..0x24].copy_from_slice(&variant.to_le_bytes());
        e[0x24..0x28].copy_from_slice(&15i32.to_le_bytes());
        e
    }

    fn entry(name: &str, events: Vec<[u8; 0x50]>) -> ses::Entry {
        let mut head = [0u8; 0x30];
        head[..name.len()].copy_from_slice(name.as_bytes());
        ses::Entry { head, events }
    }

    #[test]
    fn kh1_sounds_go_to_the_variant_with_recoms_steps_and_voices() {
        let mut es = vec![entry("p001_055", vec![whoosh_ev(601, 1, 19, 0), whoosh_ev(1, 232, 1, 0), whoosh_ev(2, 0, 5, 0)]), entry("p001_060", vec![whoosh_ev(1, 237, 2, 0)])];
        let mv = Move { request: 200, row: 13, clip: "p001_055".into(), weapon: Some("w001_055".into()), trail: Some((4, 24)), sounds: vec![(0, Kh1Sound::Weapon(7)), (3, Kh1Sound::Weapon(9))], finisher: false };
        let mut a = Added::default();
        // Only weapon + 7 has a bank.
        sora_ses(&mut es, std::slice::from_ref(&mv), 0x2B10, |id| (id == 0x2B17).then_some(9_030_000), &mut a).unwrap();
        let evs: Vec<(u32, u32, i32, i32)> = es[0].events.iter().map(|e| (u32_of(e, 0), u32_of(e, 4), i32::from_le_bytes(e[0x18..0x1C].try_into().unwrap()), u32_of(e, 0x20) as i32)).collect();
        assert_eq!(
            evs,
            vec![(601, 1, 19, 0), (1, 232, 1, 0), (2, 0, 5, 0), (601, 1, 19, VARIANT), (2, 0, 5, VARIANT), (9030, 0, 0, VARIANT), (1, 234, 3, VARIANT)],
            "Re:CoM's own events untouched; at the KH1 variant: Re:CoM's step and voice, KH1's sound, Re:CoM's whoosh where KH1's has no bank"
        );
        assert_eq!(a.sounds, vec![(200, "p001_055".into(), vec![0x2B17], vec![0x2B19])]);
        // Run again: the KH1 events are replaced, not added twice.
        sora_ses(&mut es, std::slice::from_ref(&mv), 0x2B10, |id| (id == 0x2B17).then_some(9_030_000), &mut a).unwrap();
        assert_eq!(es[0].events.len(), 7);
        // No bank for any of its sounds: nothing at the variant (Re:CoM's events play).
        let mut es2 = vec![entry("p001_055", vec![whoosh_ev(1, 232, 1, 0)]), entry("p001_060", vec![whoosh_ev(1, 237, 2, 0)])];
        sora_ses(&mut es2, &[mv], 0x2B10, |_| None, &mut Added::default()).unwrap();
        assert_eq!(es2[0].events.len(), 1);
    }

    fn recom_weapon_eet() -> Vec<u8> {
        let t = Trail { dpx: TRAIL_SET.into(), id: 5, nodes: [(2, [-0.11, 0.0, 0.0]), (2, [-0.78, 0.0, 0.0]), (2, [0.0; 3])], start: 7, end: 17, variant: 0, fade: 10.0 };
        eet::write(&[Record { name: TRAIL_TEMPLATE.into(), a: vec![], b: vec![t.write(&[0u8; eet::EVENT]).unwrap()], raw_name: None }]).unwrap()
    }

    #[test]
    fn the_keyblade_gets_kh1s_trail_on_recoms_nodes_and_gravity_breaks_blade_effect() {
        let moves = vec![
            Move { request: 200, row: 13, clip: "p001_055".into(), weapon: Some("w001_055".into()), trail: Some((4, 24)), sounds: vec![], finisher: false },
            Move { request: 217, row: 4, clip: "p001_229".into(), weapon: Some("w001_229".into()), trail: Some((30, 56)), sounds: vec![], finisher: true },
        ];
        let mut a = Added::default();
        let out = eet::parse(&weapon_eet(&recom_weapon_eet(), &moves, Some([-0.78, 0.0, 0.0]), &mut a).unwrap()).unwrap();
        assert_eq!(out.len(), 3, "Re:CoM's record, then one per Keyblade clip");
        assert_eq!(Trail::read(&out[0].b[0]).variant, 0, "Re:CoM's own trail untouched");
        let t = Trail::read(&out[1].b[0]);
        assert_eq!((t.start, t.end, t.variant, t.fade, t.nodes[1].1[0]), (4, 24, VARIANT, TRAIL_FADE, -0.78));
        assert_eq!(out[2].a.len(), 1, "Gravity Break's effect 2 on the blade");
        assert_eq!(a.effects, vec![(217, "w001_229".into(), vec![2])]);
    }

    #[test]
    fn soras_finisher_effects_by_kh1s_attach_codes() {
        let moves: Vec<Move> = [(216u16, 3usize), (217, 4), (218, 5), (215, 7)].iter().map(|&(r, row)| Move { request: r, row, clip: format!("p001_{r}"), weapon: None, trail: None, sounds: vec![], finisher: true }).collect();
        let mut a = Added::default();
        let out = eet::parse(&sora_eet(&eet::write(&[]).unwrap(), &moves, &mut a).unwrap()).unwrap();
        let ids: Vec<(u16, Vec<u32>)> = a.effects.iter().map(|e| (e.0, e.2.clone())).collect();
        assert_eq!(ids, vec![(216, vec![1]), (217, vec![3]), (218, vec![4]), (215, vec![6, 7])]);
        assert_eq!(a.effects_out.iter().map(|e| (e.0, e.1)).collect::<Vec<_>>(), vec![(216, 0), (218, 5)], "the target-placed ones");
        let flags = |rec: &Record, k: usize| u32_of(&rec.a[k], 0x54);
        let bone = |rec: &Record, k: usize| i32::from_le_bytes(rec.a[k][0x24..0x28].try_into().unwrap());
        assert_eq!((bone(&out[0], 0), flags(&out[0], 0)), (0, DETACHED | FLAG_3 | LINGER));
        assert_eq!((bone(&out[3], 0), flags(&out[3], 0)), (RECOM_BONE_FOR_KH1_8, FLAG_3 | LINGER), "Ripple Drive's 6 on KH1's bone 8");
    }
}
