//! Re:CoM's own HP gauge turned 180° to KH1's place (owner, 2026-11-01: "get the Re:CoM health
//! bar, place it in the same place as the KH1 health bar, and rotate the ReCoM green health so its
//! in the same orientation"; the mockup "Yes, like the mockup"). `docs/recom-kh1-hud.md` § Re:CoM's HP gauge at KH1's place.
//!
//! Re:CoM's HP ring and bar are `gauge:01` animation groups 7 (max HP), 8 (the red drain) and 9
//! (HP): quarter-ring sprites placed in up to three quadrants with mirrored parts, z-rotation
//! animations over frames 0..270, zero-UV scissor parts (`CScissoring`), then a 200-unit straight
//! bar over frames 271..470 (`0x140208360` adds them as layers 1..3 of the gauge's control `+0x78`;
//! the HP setters `0x14020D6A0` / `0x14020D8E0` set their frames). Checked (decomp + files).
//!
//! Turning a control does not work for them: Re:CoM builds a scissor from the transformed
//! top-left and bottom-right corners of its part as they are (`0x14024A3F0`: no min / max), so a
//! turned scissor part becomes an empty GS scissor (`0x1402643D0` writes `SCISSOR_1` from them).
//! Checked (decomp). So this step adds the three groups again, turned in the data: every image
//! part's corners negated (the same corner keeps its UV, so the picture turns with it), every
//! scissor part negated and reordered (still top-left → bottom-right), every translation and pivot
//! negated; rotation angles and scales kept (a turn by 180° commutes with both). Re:CoM's own groups
//! stay as they are; the DLL puts ours on the gauge's layers 1..3 at `hud = kh1` (Re:CoM's layer
//! replace, vtable `+0x20`), so Re:CoM keeps playing, scrubbing, hiding and drawing them.
//!
//! The disc behind the face (layer 0: `gauge:01` 0xE / 0xF / 0x10 / 10, four mirrored quadrants
//! and a spinning sparkle) and the face (layer 4: `gauge:00`) are drawn about the gauge's origin and
//! are left upright (the face must stay upright: the owner's mockup).

use crate::mp_bar::{Container, Layd, Seqd, AGROUP, ANIM, PART};
use crate::{bad, Result};

/// The layout in `FORM/0002/FO0002.SPR` that holds Re:CoM's HP gauge.
pub const LAYOUT: &str = "gauge.lad";
/// The sprite set (SEQD 1 of `gauge.lad`).
pub const SET: &str = "gauge:01";
/// Its SEQD index in the layout.
pub const SEQ: usize = 1;
/// Re:CoM Steam's animation groups in `gauge:01` (every copy on the owner's files; Checked (files)).
pub const BASE_GROUPS: usize = 47;
/// The groups turned, in layer order (layers 1, 2, 3 of the HP control: max HP, drain, HP).
pub const TURNED: [usize; 3] = [7, 8, 9];

fn i32_at(r: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(r[at..at + 4].try_into().unwrap())
}

fn put_i32(r: &mut [u8], at: usize, v: i32) {
    r[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn f32_at(r: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(r[at..at + 4].try_into().unwrap())
}

/// A part Re:CoM draws as a scissor (`0x14024B910`): its sprite's UVs are all 0 and it is the only
/// part of its sprite group. Checked (decomp).
fn is_scissor(seq: &Seqd, sprite: usize, parts_in_group: usize) -> bool {
    parts_in_group == 1 && seq.sprites.get(sprite).is_some_and(|s| (0..4).all(|k| i32_at(s, 4 + 4 * k) == 0))
}

/// The part turned 180° about the origin: an image part's corners negated (each corner keeps its
/// UV); a scissor part's too, then reordered so it still runs top-left → bottom-right.
pub fn turn_part(p: &[u8; PART], scissor: bool) -> [u8; PART] {
    let [x0, y0, x1, y1] = [0, 4, 8, 12].map(|at| i32_at(p, at));
    let rect = if scissor { [-x1.max(x0), -y1.max(y0), -x0.min(x1), -y0.min(y1)] } else { [-x0, -y0, -x1, -y1] };
    let mut r = *p;
    for (k, v) in rect.into_iter().enumerate() {
        put_i32(&mut r, 4 * k, v);
    }
    r
}

/// The animation turned 180°: translations (`+0x10..+0x1C`) and pivots (`+0x20..+0x2C`, ints,
/// `0x14024B4E0`) negated, sprite group `sg`. Refused when it bounces (`+0x70..+0x7C`, a positional
/// wobble this step does not turn; none in Re:CoM's groups 7..9).
fn turn_anim(a: &[u8; ANIM], sg: usize) -> Result<[u8; ANIM]> {
    if (0x70..0x80).step_by(4).any(|at| f32_at(a, at) != 0.0) {
        return bad("gauge:01: an animation of the HP ring bounces (not turned by this step)");
    }
    let mut r = *a;
    put_i32(&mut r, 4, sg as i32);
    for at in (0x10..0x30).step_by(4) {
        let v = i32_at(a, at);
        put_i32(&mut r, at, v.wrapping_neg());
    }
    Ok(r)
}

/// Appends group `g` turned 180° (new parts, sprite groups and animations; Re:CoM's records kept)
/// and returns the new group's index.
pub fn add_turned(seq: &mut Seqd, g: usize) -> Result<usize> {
    let ag = *seq.agroups.get(g).ok_or_else(|| crate::Error::Format(format!("{SET}: no group {g}")))?;
    let a0 = u16::from_le_bytes([ag[0], ag[1]]) as usize;
    let n = u16::from_le_bytes([ag[2], ag[3]]) as usize;
    let mut sg_map: Vec<(usize, usize)> = Vec::new();
    let mut anims = Vec::with_capacity(n);
    for k in a0..a0 + n {
        let a = *seq.anims.get(k).ok_or_else(|| crate::Error::Format(format!("{SET}: group {g} past the animations")))?;
        let sg = i32_at(&a, 4) as usize;
        let new_sg = match sg_map.iter().find(|m| m.0 == sg) {
            Some(m) => m.1,
            None => {
                let s = *seq.sgroups.get(sg).ok_or_else(|| crate::Error::Format(format!("{SET}: no sprite group {sg}")))?;
                let p0 = i16::from_le_bytes([s[0], s[1]]) as usize;
                let pn = i16::from_le_bytes([s[2], s[3]]) as usize;
                let first = seq.parts.len();
                for j in p0..p0 + pn {
                    let p = *seq.parts.get(j).ok_or_else(|| crate::Error::Format(format!("{SET}: no part {j}")))?;
                    let scissor = is_scissor(seq, i32_at(&p, 16) as usize, pn);
                    if scissor && (0x30..0x48).step_by(4).any(|at| f32_at(&a, at) != 0.0) {
                        return bad(format!("{SET}: group {g} rotates a scissor part (not turned by this step)"));
                    }
                    seq.parts.push(turn_part(&p, scissor));
                }
                let ns = seq.sgroups.len();
                seq.sgroups.push(crate::mp_bar::sgroup(first, pn));
                sg_map.push((sg, ns));
                ns
            }
        };
        anims.push(turn_anim(&a, new_sg)?);
    }
    let first = seq.anims.len();
    seq.anims.extend(anims);
    let mut r: [u8; AGROUP] = ag;
    r[0..2].copy_from_slice(&(first as u16).to_le_bytes());
    seq.agroups.push(r);
    if seq.anims.len() > u16::MAX as usize || seq.parts.len() > i16::MAX as usize {
        return bad("seqd: too many records for 16-bit indices");
    }
    Ok(seq.agroups.len() - 1)
}

/// Adds the turned groups to a `gauge.lad` (its `gauge:01`); returns the layout and the first new
/// group. A layout that is not Re:CoM Steam's (another group count, already patched) is refused.
pub fn patch_layout(lad: &[u8]) -> Result<(Vec<u8>, usize)> {
    let mut l = Layd::parse(lad)?;
    let raw = l.seqs.get(SEQ).ok_or_else(|| crate::Error::Format(format!("{LAYOUT}: no {SET}")))?;
    let mut s = Seqd::parse(raw)?;
    if s.agroups.len() != BASE_GROUPS {
        let note = if s.agroups.len() == BASE_GROUPS + TURNED.len() { " (already patched?)" } else { "" };
        return bad(format!("{SET} has {} animation groups, Re:CoM Steam's has {BASE_GROUPS}{note}", s.agroups.len()));
    }
    let first = s.agroups.len();
    for g in TURNED {
        add_turned(&mut s, g)?;
    }
    // The table offsets and the layout's other sets are rewritten as they were.
    l.seqs[SEQ] = s.write();
    Ok((l.write(), first))
}

/// A file of `FO0002.SPR` (the container, or a PC asset) this step patches: the container's
/// `gauge.lad`, or a `<LANG>_gauge.lad` asset.
pub fn is_gauge_asset(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == LAYOUT || (n.len() == LAYOUT.len() + 3 && n.ends_with(LAYOUT) && n.as_bytes()[2] == b'_')
}

/// Patches the `gauge.lad` of an `FO0002.SPR` container and every `<LANG>_gauge.lad` PC asset;
/// returns the first new group and the files patched. Every copy must give the same first group.
pub fn patch_spr(spr: &mut Vec<u8>, assets: &mut [kh1_formats::egs::HdAsset]) -> Result<(usize, Vec<String>)> {
    let mut c = Container::parse(spr)?;
    let e = c.find_mut(LAYOUT).ok_or_else(|| crate::Error::Format(format!("FO0002.SPR: no {LAYOUT}")))?;
    let (lad, first) = patch_layout(&e.data)?;
    e.data = lad;
    *spr = c.write();
    let mut files = vec![LAYOUT.to_string()];
    for a in assets.iter_mut().filter(|a| is_gauge_asset(&a.name) && a.data.starts_with(b"LAYD")) {
        let (lad, f) = patch_layout(&a.data).map_err(|e| crate::Error::Format(format!("{}: {e}", a.name)))?;
        if f != first {
            return bad(format!("{}: first group {f}, the base file's {first}", a.name));
        }
        a.data = lad;
        files.push(a.name.clone());
    }
    Ok((first, files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp_bar::{agroup, anim, part, sgroup, sprite};

    fn i(r: &[u8], at: usize) -> i32 {
        i32_at(r, at)
    }

    /// A tiny `gauge:01`: 47 groups, group 7 = a scissor part, then a mirrored image part with a
    /// translation, group 8 the image part, group 9 the scissor.
    fn seq() -> Seqd {
        let mut s = Seqd { version: 0x160, sprites: Vec::new(), parts: Vec::new(), sgroups: Vec::new(), anims: Vec::new(), agroups: Vec::new() };
        s.sprites.push(sprite([0, 0, 0, 0], [0x8080_8080; 4])); // scissor
        s.sprites.push(sprite([94, 5, 137, 48], [0x8080_8080; 4])); // quarter ring
        s.parts.push(part([0, -42, 43, 0], 0));
        s.parts.push(part([0, 0, -43, 43], 1));
        s.parts.push(part([27, 3, 44, 0], 1));
        s.sgroups.push(sgroup(0, 1));
        s.sgroups.push(sgroup(1, 2));
        s.anims.push(anim(0x77FF1, 0, (0, 90), 0, (1.0, 1.0), (0x8080_8080, 0x8080_8080)));
        let mut a = anim(0x6FD1, 1, (271, 470), 0, (1.0, 1.0), (0x8080_8080, 0x8080_8080));
        a[0x14..0x18].copy_from_slice(&200i32.to_le_bytes());
        a[0x40..0x44].copy_from_slice(&(-4.71239f32).to_le_bytes());
        s.anims.push(a);
        for g in 0..BASE_GROUPS {
            match g {
                7 => s.agroups.push(agroup(0, 2, false, 947)),
                8 => s.agroups.push(agroup(1, 1, false, 1011)),
                _ => s.agroups.push(agroup(0, 1, false, 1011)),
            }
        }
        s
    }

    #[test]
    fn parts_turn_about_the_origin() {
        let img = part([0, 0, -43, 43], 1);
        assert_eq!([0, 4, 8, 12].map(|at| i(&turn_part(&img, false), at)), [0, 0, 43, -43]);
        let sc = part([-43, -43, 0, 0], 0);
        assert_eq!([0, 4, 8, 12].map(|at| i(&turn_part(&sc, true), at)), [0, 0, 43, 43]);
        let sc = part([0, -42, 43, 0], 0);
        assert_eq!([0, 4, 8, 12].map(|at| i(&turn_part(&sc, true), at)), [-43, 0, 0, 42]);
    }

    #[test]
    fn groups_turn_and_keep_re_coms() {
        let mut s = seq();
        let before = s.clone();
        let g = add_turned(&mut s, 7).unwrap();
        assert_eq!(g, BASE_GROUPS);
        // Re:CoM's records are untouched; ours follow.
        assert_eq!(s.sprites, before.sprites);
        assert_eq!(&s.parts[..3], &before.parts[..]);
        assert_eq!(&s.anims[..2], &before.anims[..]);
        assert_eq!(&s.agroups[..BASE_GROUPS], &before.agroups[..]);
        // The group: two animations from 2 on, word kept.
        let ag = s.agroups[g];
        assert_eq!((u16::from_le_bytes([ag[0], ag[1]]), u16::from_le_bytes([ag[2], ag[3]])), (2, 2));
        assert_eq!(ag[4..], before.agroups[7][4..]);
        // The scissor: normalised; the image parts: negated; the translation negated, the angle kept.
        let a0 = s.anims[2];
        let sg = i(&a0, 4) as usize;
        let p = s.parts[i16::from_le_bytes([s.sgroups[sg][0], s.sgroups[sg][1]]) as usize];
        assert_eq!([0, 4, 8, 12].map(|at| i(&p, at)), [-43, 0, 0, 42]);
        let a1 = s.anims[3];
        assert_eq!((i(&a1, 0x10), i(&a1, 0x14), f32_at(&a1, 0x40)), (0, -200, -4.71239));
        let sg = i(&a1, 4) as usize;
        let p0 = i16::from_le_bytes([s.sgroups[sg][0], s.sgroups[sg][1]]) as usize;
        assert_eq!([0, 4, 8, 12].map(|at| i(&s.parts[p0], at)), [0, 0, 43, -43]);
        assert_eq!([0, 4, 8, 12].map(|at| i(&s.parts[p0 + 1], at)), [-27, -3, -44, 0]);
        assert_eq!(i(&s.parts[p0], 16), 1, "same sprite");
    }

    #[test]
    fn layout_patch_adds_three_and_refuses_a_second_run() {
        let s = seq();
        let other = Seqd { agroups: s.agroups[..4].to_vec(), ..s.clone() };
        // A LAYD with gauge:00, gauge:01 (as the base file).
        let seqs = [other.write(), s.write()];
        let mut d = b"LAYD".to_vec();
        for v in [0x100u32, 0, 0x20, 0, 0x20, seqs.len() as u32, 0x20] {
            d.extend(v.to_le_bytes());
        }
        let mut at = 0x20 + 4 * seqs.len() as u32;
        for q in &seqs {
            d.extend(at.to_le_bytes());
            at += q.len() as u32;
        }
        for q in &seqs {
            d.extend(q);
        }
        let (out, first) = patch_layout(&d).unwrap();
        assert_eq!(first, BASE_GROUPS);
        let l = Layd::parse(&out).unwrap();
        assert_eq!(l.seqs[0], seqs[0]);
        assert_eq!(Seqd::parse(&l.seqs[1]).unwrap().agroups.len(), BASE_GROUPS + 3);
        assert!(patch_layout(&out).is_err());
        assert!(is_gauge_asset("UK_gauge.lad") && is_gauge_asset("gauge.lad") && !is_gauge_asset("UK_monstro_gauge.lad") && !is_gauge_asset("UK_Cockpit.lad"));
    }

    /// The owner's `FO0002.SPR` with its PC assets (`KH1_HUD_DUMP`): every copy patched, Re:CoM's
    /// records kept, scissors still top-left → bottom-right.
    #[test]
    fn owner_gauge_files() {
        let Ok(dir) = std::env::var("KH1_HUD_DUMP") else { return };
        let r = std::path::Path::new(&dir).join("recom/FORM/0002");
        let mut spr = std::fs::read(r.join("FO0002.SPR")).unwrap();
        let mut assets: Vec<kh1_formats::egs::HdAsset> = std::fs::read_dir(r.join("FO0002.SPR.hd"))
            .unwrap()
            .flatten()
            .map(|e| kh1_formats::egs::HdAsset { name: e.file_name().to_string_lossy().into_owned(), orig_offset: 0xFFFF, data: std::fs::read(e.path()).unwrap() })
            .collect();
        let before = assets.clone();
        let (first, files) = patch_spr(&mut spr, &mut assets).unwrap();
        assert_eq!(first, BASE_GROUPS);
        eprintln!("gauge:01 turned groups {first}..{} in {files:?}", first + 2);
        if let Ok(out) = std::env::var("KH1_HUD_OUT") {
            for a in assets.iter().filter(|a| a.name == "UK_gauge.lad") {
                std::fs::write(std::path::Path::new(&out).join(&a.name), &a.data).unwrap();
            }
        }
        assert_eq!(files.len(), 6);
        for (a, b) in assets.iter().zip(&before) {
            if !is_gauge_asset(&a.name) {
                assert_eq!(a.data, b.data);
                continue;
            }
            let (la, lb) = (Layd::parse(&a.data).unwrap(), Layd::parse(&b.data).unwrap());
            let (sa, sb) = (Seqd::parse(&la.seqs[SEQ]).unwrap(), Seqd::parse(&lb.seqs[SEQ]).unwrap());
            assert_eq!(&sa.agroups[..BASE_GROUPS], &sb.agroups[..]);
            assert_eq!(&sa.parts[..sb.parts.len()], &sb.parts[..]);
            assert_eq!(&sa.anims[..sb.anims.len()], &sb.anims[..]);
            for (k, s) in la.seqs.iter().enumerate().filter(|k| k.0 != SEQ) {
                assert_eq!(s, &lb.seqs[k]);
            }
            for p in &sa.parts[sb.parts.len()..] {
                let [x0, y0, x1, y1] = [0, 4, 8, 12].map(|at| i(p, at));
                if is_scissor(&sa, i(p, 16) as usize, 1) {
                    assert!(x0 <= x1 && y0 <= y1, "{}: scissor {x0},{y0},{x1},{y1}", a.name);
                }
            }
        }
    }
}
