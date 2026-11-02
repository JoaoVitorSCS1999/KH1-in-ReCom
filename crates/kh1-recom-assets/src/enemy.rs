//! M11: the shared asset step for KH1's enemies in Re:CoM's battles (`docs/recom-m11-enemies.md`).
//! From the player's own files, for one enemy ([`Spec`]), it makes Re:CoM's `ENEMY/NNNN` files able
//! to play the KH1 twin natively:
//!
//! - `ENNNNN.ANN`: KH1's motions (`.mset` on the `.mdls` skeleton) as new 60 fps clips at slot
//!   100 + KH1 entry (`eNNN_1EE`; Re:CoM's name lookup `0x1401A47E0` takes the slot from the
//!   name's digits); Re:CoM's slots stay byte-identical; an entry Re:CoM has as an exact bake
//!   plays Re:CoM's clip (decision M10).
//! - `ENNNNN.ASD`: Re:CoM's attack records (`@COP` v3, 0xA0 each; loader `0x140179340`, bound by
//!   motion name at load `0x14014DBE0`, ticked by `0x14014E830`) with one new record per KH1
//!   attack row: the row's hit slot window (row `+0x34` picks the `.mset` HIT track's sub:
//!   `0x1402B2DC0` writes it to the object's bits 4..5, the hit test `0x1402B0000.c` reads
//!   `0x10000 << sub`; Checked (decomp)), its points (bone code → bone, `t` along it) in Re:CoM's
//!   frame of the bone's parent, its radius (× the actor scale 1: Assumption), the Re:CoM attack id
//!   it takes its power from (`@ENP`, decision M10).
//! - `ENNNNN.SES`: Re:CoM's entries kept; each new clip KH1's matcher pairs with a Re:CoM clip
//!   that has sounds gets those (same frames); the other attack clips get their template record's
//!   sounds at the same place relative to the hit window (a frame-0 sound stays at 0). Assumption:
//!   KH1's own enemy sounds are not mapped (decision 22: Re:CoM's sounds).
//! - `kh1-<enemy>.json` (`kh1-enemy 1`): every number of the behaviour (read from the player's
//!   script at checked offsets, `btltbl.bin`'s globals, the stat records, KH1's exe numbers), the
//!   attacks (request, clash recoil), the hurt parts' Re:CoM bones, the request → slot map.
//!
//! The Shadow (M10) keeps its own step (`crate::shadow`, unchanged output) on the helpers here.

use kh1_formats::mset::MotionSet;

use crate::ann::{name16, Ann, Slot};
use crate::build::{cycle_error, LOOP_TOLERANCE_M};
use crate::convert::{ClipSpec, Converter};
use crate::matching::{decode_pack, match_all, MatchKind, Query};
use crate::mdl::RecomSkeleton;
use crate::shadow::{attack_row, bone_codes, AttackRow};
use crate::{bad, Result};

/// New clips sit at this + KH1 entry.
pub const SLOT_BASE: usize = 100;
/// KH1's filler requests (0x141..) map entry 0 without root motion; no enemy script requests them
/// (Checked (files): the Shadow's and the Soldier's scripts).
pub const LAST_REQUEST: usize = 0x140;
/// The manifest's format; the DLL refuses others.
pub const FORMAT: &str = "kh1-enemy 1";
const REC: usize = 0xA0;

/// One attack row of a KH1 attack.
#[derive(Clone, Copy, Debug)]
pub struct RowSpec {
    /// The row's script offset (0x70 bytes, `sys 23`).
    pub at: usize,
    /// The Re:CoM attack id (`@ENP` attack entry `+0x18`) it takes its power from; its record is
    /// the template (`+0x64`).
    pub recom_id: u32,
}

/// One KH1 attack (an action request made with attack rows).
#[derive(Clone, Copy, Debug)]
pub struct AttackSpec {
    /// For the manifest and the log.
    pub name: &'static str,
    /// Its KH1 motion request.
    pub request: u16,
    /// The rows the script makes with it.
    pub rows: &'static [RowSpec],
}

/// One enemy's step.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// The manifest's `enemy` (the DLL's class name).
    pub name: &'static str,
    /// Re:CoM's enemy id (`ENEMY/NNNN`).
    pub recom_id: u16,
    /// KH1's model (`.mdls`, `.mset`) without the extension.
    pub kh1: &'static str,
    /// The script's name in its header (`+4`).
    pub script: &'static [u8],
    /// Re:CoM's clip name prefix (`e002_`) and slot count (Checked (files)).
    pub prefix: &'static str,
    /// Re:CoM's slot count in the `.ANN`.
    pub recom_slots: usize,
    /// Re:CoM's `.ASD`: record count, first record, trailing section (Checked (files)).
    pub asd: (usize, usize, usize),
    /// Where each constant is pushed (a name with several offsets must read the same at each).
    pub consts: &'static [(&'static str, &'static [usize])],
    /// Request words the script must push at these offsets (the behaviour plays them).
    pub requests: &'static [(u32, usize)],
    /// The stat records (their `+0x18`, the knock-back weight, must agree).
    pub records: &'static [u32],
    /// Its attacks, in the behaviour's order.
    pub attacks: &'static [AttackSpec],
    /// The manifest file (recom root).
    pub manifest: &'static str,
}

impl Spec {
    /// `ENEMY/NNNN`.
    pub fn dir(&self) -> String {
        format!("ENEMY/{:04}", self.recom_id)
    }
    /// `ENEMY/NNNN/ENNNNN.<ext>` (`Recom.hed`'s name).
    pub fn path(&self, ext: &str) -> String {
        format!("ENEMY/{:04}/EN{:04}.{ext}", self.recom_id, self.recom_id)
    }
    /// The test / scratch name of an extracted Re:CoM file (`ENEMY__0002__EN0002.ANN`).
    pub fn flat(&self, ext: &str) -> String {
        self.path(ext).replace('/', "__")
    }
}

/// Inputs: KH1's model, Re:CoM's files, KH1's `btltbl.bin`.
#[derive(Clone, Debug)]
pub struct Inputs {
    /// KH1's `.mset`.
    pub mset: Vec<u8>,
    /// KH1's `.mdls` (skeleton, bone codes, collision block, AI script in part 5).
    pub mdls: Vec<u8>,
    /// Re:CoM's `.ANN`.
    pub ann: Vec<u8>,
    /// Re:CoM's `.MDL` (skeleton).
    pub mdl: Vec<u8>,
    /// Re:CoM's `.ASD`.
    pub asd: Vec<u8>,
    /// Re:CoM's `.SES`.
    pub ses: Vec<u8>,
    /// KH1's `btltbl.bin`.
    pub btltbl: Vec<u8>,
}

/// One request's clip.
pub use crate::shadow::RequestClip;

/// The step's output.
#[derive(Clone, Debug)]
pub struct Built {
    /// The extended `.ANN`.
    pub ann: Vec<u8>,
    /// The extended `.ASD`.
    pub asd: Vec<u8>,
    /// The extended `.SES`.
    pub ses: Vec<u8>,
    /// The manifest (`kh1-enemy 1`).
    pub manifest: String,
    /// Every request's clip.
    pub clips: Vec<RequestClip>,
    /// The behaviour's numbers.
    pub consts: Vec<(String, f32)>,
    /// Records added to the ASD (log lines).
    pub records: Vec<String>,
    /// The hurt parts (KH1 bone, Re:CoM bone).
    pub parts: Vec<(usize, usize)>,
}

/// KH1's exe numbers the shared code reads (`kh1_core::enemy::EXE_NUMBERS`, the same values;
/// kept here so this crate needs no kh1-core). Checked (decomp + exe).
pub fn exe_consts() -> Vec<(&'static str, f32)> {
    crate::shadow::exe_consts()
}

/// KH1's stats-block defaults (`0x1402A5D70`: `+4` walk 2, `+8` run 8, `+0x10` hop 190; Checked
/// (decomp)): a constant listed with no offsets takes its value here (a script that never writes
/// it keeps the default; M11 fliers).
pub const STATS_DEFAULTS: &[(&str, f32)] = &[("walk_speed", 2.0), ("run_speed", 8.0), ("hop_height", 190.0)];

/// Reads the constants at `table` (`push`: an opcode word whose low byte is 0x12 / 0x52 (a float)
/// or 0x02 / 0x42 (an integer, as a number), then the value). A name with no offsets is one of
/// [`STATS_DEFAULTS`].
pub fn script_consts(script: &[u8], name: &[u8], table: &[(&str, &[usize])]) -> Result<Vec<(String, f32)>> {
    let sname = String::from_utf8_lossy(name);
    if script.get(4..4 + name.len()) != Some(name) {
        return bad(format!("the model's part 5 is not {sname}"));
    }
    let mut out = Vec::new();
    for (n, offs) in table {
        if offs.is_empty() {
            let d = STATS_DEFAULTS.iter().find(|d| d.0 == *n).ok_or_else(|| crate::Error::Format(format!("{sname}: {n} has no offset and no stats-block default")))?;
            out.push((n.to_string(), d.1));
            continue;
        }
        let mut v: Option<f32> = None;
        for &o in offs.iter() {
            let w = script.get(o..o + 6).ok_or_else(|| crate::Error::Format(format!("{sname}: short at {o:#x}")))?;
            let x = match w[0] & 0xBF {
                0x12 => f32::from_le_bytes(w[2..6].try_into().unwrap()),
                0x02 => i32::from_le_bytes(w[2..6].try_into().unwrap()) as f32,
                _ => return bad(format!("{sname} {o:#x} ({n}): not a push (opcode {:02x}{:02x}); not the script this step knows", w[0], w[1])),
            };
            if !x.is_finite() {
                return bad(format!("{sname} {o:#x} ({n}): {x}"));
            }
            match v {
                Some(p) if p != x => return bad(format!("{sname}: {n} is {p} and {x} at {offs:x?}")),
                _ => v = Some(x),
            }
        }
        out.push((n.to_string(), v.unwrap()));
    }
    Ok(out)
}

/// Checks the request words the script pushes (integer pushes) at their offsets.
pub fn check_requests(script: &[u8], name: &[u8], table: &[(u32, usize)]) -> Result<()> {
    let sname = String::from_utf8_lossy(name);
    for &(want, o) in table {
        let w = script.get(o..o + 6).ok_or_else(|| crate::Error::Format(format!("{sname}: short at {o:#x}")))?;
        let got = u32::from_le_bytes(w[2..6].try_into().unwrap());
        if w[0] & 0xBF != 0x02 || got != want {
            return bad(format!("{sname} {o:#x}: request {got:#x}, not {want:#x}: not the script this step knows"));
        }
    }
    Ok(())
}

/// The knock-back weight (stat record `+0x18`) from every record; they must agree.
pub fn weight(script: &[u8], records: &[u32]) -> Result<f32> {
    let mut w: Option<f32> = None;
    for &at in records {
        let o = at as usize + 0x18;
        let x = script.get(o..o + 4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| crate::Error::Format(format!("no record at {at:#x}")))?;
        match w {
            Some(p) if p != x => return bad(format!("knock-back weight {p} and {x} in different records")),
            _ => w = Some(x),
        }
    }
    w.ok_or_else(|| crate::Error::Format("no stat records".into()))
}

/// The btltbl globals the shared reactions read, as the behaviour's numbers.
pub fn btltbl_consts(btltbl: &[u8]) -> Result<Vec<(String, f32)>> {
    let g = kh1_formats::btltbl::Btltbl::parse(btltbl.to_vec())?.globals()?;
    Ok(vec![
        ("launch_min".into(), g.launch_speed_min as f32),
        ("launch_max".into(), g.launch_speed_max as f32),
        ("launch_hop".into(), g.launch_height_factor),
        ("flinch_push".into(), g.reaction_pushes[4]),
        ("air_flinch_speed".into(), g.air_knockback[0]),
        ("air_knock_speed".into(), g.air_knockback[1]),
    ])
}

/// KH1's hurt parts (the collision block's group 2, `.mdls` part 4 `CLS\x03`, 0x14-byte entries;
/// byte 2 a bone code) on the code's bone's parent, as Re:CoM bones (the nearest mapped
/// ancestor). Checked (files): for the Shadow this gives KH1 bones 30 / 2 (M10's checked parts).
pub fn hurt_parts(mdls: &[u8], conv: &Converter) -> Result<Vec<(usize, usize)>> {
    let c = crate::stats::mdls_part(mdls, 4).ok_or_else(|| crate::Error::Format("no collision block (part 4)".into()))?;
    if c.get(0..4) != Some(b"CLS\x03".as_slice()) {
        return bad("collision block is not CLS\\x03");
    }
    let n: Vec<usize> = (0..4).map(|i| c[4 + i] as usize).collect();
    let first = 0x20 + (n[0] + n[1]) * 0x14;
    let codes = bone_codes(mdls)?;
    let rig = conv.rig();
    let mut out = Vec::new();
    for i in 0..n[2] {
        let e = c.get(first + i * 0x14..first + (i + 1) * 0x14).ok_or_else(|| crate::Error::Format("hurt parts past the end".into()))?;
        let code = e[2] as usize;
        let bone = *codes.get(code).filter(|b| **b >= 0).ok_or_else(|| crate::Error::Format(format!("hurt part {i}: bone code {code} names no bone")))? as usize;
        let mut k = rig.parent(bone).unwrap_or(bone);
        let recom = loop {
            if let Some(r) = conv.map().iter().position(|m| *m == Some(k)) {
                break r;
            }
            match rig.parent(k) {
                Some(p) => k = p,
                None => return bad(format!("hurt part {i}: KH1 bone {bone} has no mapped ancestor")),
            }
        };
        out.push((k, recom));
    }
    if out.is_empty() {
        return bad("no hurt parts");
    }
    Ok(out)
}

/// Every window a HIT track is on (KH1's attack objects hit while the slot's bit is set,
/// `0x1402B47A0`): one Re:CoM record each, as Re:CoM's own models have one record per window (its
/// Powerwild's `e007_050`: two records, Checked (files)). The Powerwild's flurry turns its right
/// hand's slot on three times (M11).
pub fn hit_windows(m: &kh1_formats::mset::Motion, sub: u16) -> Result<Vec<(f32, f32)>> {
    let t = m.tracks.iter().find(|t| t.kind == kh1_formats::mset::track::HIT && t.sub == sub).ok_or_else(|| crate::Error::Format(format!("attack motion without a HIT track {sub}")))?;
    let mut out = Vec::new();
    let mut on: Option<f32> = None;
    for k in &t.keys {
        match (on, k.value() != 0) {
            (None, true) => on = Some(k.time),
            (Some(a), false) if k.time > a => {
                out.push((a, k.time));
                on = None;
            }
            _ => {}
        }
    }
    if let Some(a) = on {
        out.push((a, m.frames as f32));
    }
    if out.is_empty() {
        return bad("HIT track never on");
    }
    Ok(out)
}

/// The HIT track's first window of hit slot `sub` (key ≠ 0 on, then 0 off; `kh1-data`'s rule): the
/// sound events' anchor.
pub fn hit_window(m: &kh1_formats::mset::Motion, sub: u16) -> Result<(f32, f32)> {
    let t = m.tracks.iter().find(|t| t.kind == kh1_formats::mset::track::HIT && t.sub == sub).ok_or_else(|| crate::Error::Format(format!("attack motion without a HIT track {sub}")))?;
    let on = t.keys.iter().find(|k| k.value() != 0).map(|k| k.time).ok_or_else(|| crate::Error::Format("HIT track never on".into()))?;
    let off = t.keys.iter().find(|k| k.value() == 0 && k.time > on).map(|k| k.time).unwrap_or(m.frames as f32);
    Ok((on, off))
}

/// The ANN extended with KH1's motions: every entry a request (≤ [`LAST_REQUEST`]) names, at
/// slot [`SLOT_BASE`] + entry unless Re:CoM has it as an exact bake. Returns the ANN, every
/// request's clip and, per new clip, the Re:CoM clip the matcher pairs it with (bake or edit).
pub fn extend_ann(ann_in: &[u8], ms: &MotionSet, conv: &Converter, prefix: &str, recom_slots: usize) -> Result<(Vec<u8>, Vec<RequestClip>, Vec<(String, String)>)> {
    let mut ann = Ann::parse(ann_in)?;
    if ann.slots.len() != recom_slots {
        return bad(format!("{} slots, Re:CoM Steam's has {recom_slots} (already extended?)", ann.slots.len()));
    }
    let mut by_entry: Vec<(usize, Vec<u16>, bool)> = Vec::new();
    for (rid, w) in ms.requests.iter().enumerate().take(LAST_REQUEST + 1) {
        for e in w.variants() {
            let e = e as usize;
            if e >= ms.motions.len() {
                return bad(format!("request {rid:#x} names entry {e}, past the {} motions", ms.motions.len()));
            }
            match by_entry.iter_mut().find(|x| x.0 == e) {
                Some(x) => {
                    if x.2 != w.root_motion() {
                        return bad(format!("entry {e} played with and without root motion (request {rid:#x})"));
                    }
                    x.1.push(rid as u16);
                }
                None => by_entry.push((e, vec![rid as u16], w.root_motion())),
            }
        }
    }
    by_entry.sort();
    if let Some(e) = by_entry.iter().find(|x| SLOT_BASE + x.0 > 999) {
        return bad(format!("entry {} past slot 999", e.0));
    }
    let decoded = decode_pack(&ann);
    let qs: Vec<Query> = by_entry.iter().map(|(e, _, root)| Query { entry: *e, motion: &ms.motions[*e], root_motion: *root }).collect();
    let found = match_all(conv, &qs, &decoded);
    let mut clips = Vec::new();
    let mut pairs = Vec::new();
    for ((e, reqs, root), fm) in by_entry.iter().zip(found) {
        let m = &ms.motions[*e];
        let (slot, name, looped, source) = match fm.found.as_ref() {
            Some((sc, MatchKind::Bake)) => {
                let s = &ann.slots[sc.slot];
                let looped = s.clip.as_ref().is_some_and(|c| c.looped());
                (sc.slot, s.name_str(), looped, "recom")
            }
            other => {
                let slot = SLOT_BASE + e;
                let name = format!("{prefix}{slot:03}");
                if let Some((sc, _)) = other {
                    pairs.push((name.clone(), ann.slots[sc.slot].name_str()));
                }
                let looped = cycle_error(conv, m) <= LOOP_TOLERANCE_M;
                let clip = conv.convert(m, &ClipSpec { name: name.clone(), looped, root_motion: *root, blend: 8 })?;
                while ann.slots.len() < slot {
                    ann.slots.push(Slot { name: [0; 16], clip: None });
                }
                if ann.slots.len() > slot {
                    return bad(format!("slot {slot} taken"));
                }
                ann.slots.push(Slot { name: name16(&name), clip: Some(clip) });
                (slot, name, looped, "kh1")
            }
        };
        for &r in reqs {
            clips.push(RequestClip { request: r, entry: *e, slot, name: name.clone(), frames: m.frames, root: *root, looped, source });
        }
    }
    clips.sort_by_key(|c| c.request);
    ann.pad_tail_like_recom();
    Ok((ann.to_bytes(), clips, pairs))
}

fn put_name(r: &mut [u8], at: usize, s: &str) {
    r[at..at + 16].copy_from_slice(&name16(s));
}
fn put_u32(r: &mut [u8], at: usize, v: u32) {
    r[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
fn put_f32(r: &mut [u8], at: usize, v: f32) {
    r[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// One new ASD record: a row of an attack.
pub struct NewRecord<'a> {
    /// The attack's request (the clip it binds to).
    pub request: u16,
    /// The KH1 row.
    pub row: &'a AttackRow,
    /// The row's hit slot (`+0x34`).
    pub sub: u16,
    /// The Re:CoM attack id (power, template record).
    pub recom_id: u32,
}

/// The ASD with the new records added after Re:CoM's (Re:CoM's header, body and records
/// byte-identical, the trailing section moved after them, its offset `+0x18` updated; Re:CoM's
/// loader `0x140179340` / binder `0x14014DBE0` read only the counts and offsets `+8..+0x14`). The
/// template of each is Re:CoM's record with its attack id. Returns the ASD, log lines and each
/// record's window (on, off frames).
pub fn extend_asd(asd: &[u8], want: (usize, usize, usize), conv: &Converter, ms: &MotionSet, recs: &[NewRecord], clips: &[RequestClip]) -> Result<(Vec<u8>, Vec<String>)> {
    let u = |o: usize| -> Result<u32> { asd.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| crate::Error::Format("ASD: short".into())) };
    if asd.get(0..4) != Some(b"@COP".as_slice()) || u(4)? != 3 {
        return bad("ASD: not @COP version 3");
    }
    let (n, off, tail) = (u(0x10)? as usize, u(0x14)? as usize, u(0x18)? as usize);
    if (n, off, tail) != want || off + REC * n != tail || tail > asd.len() {
        return bad(format!("ASD: {n} attack records at {off:#x}, tail {tail:#x}; Re:CoM Steam's has {} at {:#x}, tail {:#x}", want.0, want.1, want.2));
    }
    let rec = |i: usize| -> [u8; REC] { asd[off + REC * i..off + REC * (i + 1)].try_into().unwrap() };
    let skel = conv.skeleton();
    let mut out_recs = Vec::new();
    let mut notes = Vec::new();
    for nr in recs {
        let tmpl = (0..n).map(rec).find(|r| u32::from_le_bytes(r[0x64..0x68].try_into().unwrap()) == nr.recom_id).ok_or_else(|| crate::Error::Format(format!("ASD: no Re:CoM record with attack id {}", nr.recom_id)))?;
        let row = nr.row;
        let c = clips.iter().find(|c| c.request == nr.request).ok_or_else(|| crate::Error::Format(format!("request {} has no clip", nr.request)))?;
        let m = &ms.motions[c.entry];
        for (on, off_) in hit_windows(m, nr.sub)? {
        let mut rig = conv.rig();
        let parent = rig.parent(row.bone).ok_or_else(|| crate::Error::Format(format!("KH1 bone {} has no parent", row.bone)))?;
        let hand = conv.map().iter().position(|k| *k == Some(parent)).ok_or_else(|| crate::Error::Format(format!("KH1 bone {parent} has no Re:CoM bone")))?;
        rig.start_motion(m);
        let p0 = conv.kh1_point_in(&mut rig, m, on, hand, row.bone, row.t[0] as f64);
        let p1 = conv.kh1_point_in(&mut rig, m, on, hand, row.bone, row.t[1] as f64);
        let mut r = tmpl;
        let hname = skel.bones[hand].name.clone();
        put_name(&mut r, 0x00, &hname);
        put_u32(&mut r, 0x10, hand as u32);
        for (k, v) in p0.iter().enumerate() {
            put_f32(&mut r, 0x14 + 4 * k, *v as f32);
        }
        let line = (row.t[0] - row.t[1]).abs() > 1e-6;
        if line {
            // A line between the two points: Re:CoM's shape index 2 (type 7, `0x1407C1C98`).
            put_name(&mut r, 0x20, &hname);
            put_u32(&mut r, 0x30, hand as u32);
            for (k, v) in p1.iter().enumerate() {
                put_f32(&mut r, 0x34 + 4 * k, *v as f32);
            }
            put_u32(&mut r, 0x54, 2);
        } else {
            put_name(&mut r, 0x20, "Root");
            put_u32(&mut r, 0x30, 0);
            for k in 0..3 {
                put_f32(&mut r, 0x34 + 4 * k, 0.0);
            }
            put_u32(&mut r, 0x54, 1);
        }
        put_name(&mut r, 0x40, &c.name);
        put_u32(&mut r, 0x50, u32::MAX);
        put_f32(&mut r, 0x58, row.radius * 0.01);
        put_u32(&mut r, 0x5C, (on * 50.0).round() as u32);
        put_u32(&mut r, 0x60, (off_ * 50.0).round() as u32);
        put_u32(&mut r, 0x64, nr.recom_id);
        put_u32(&mut r, 0x68, 0);
        r[0x98..REC].fill(0);
        notes.push(format!(
            "{} (request {}) on {hname} at ({:.4}, {:.4}, {:.4}){} r {:.2} m, hit slot {} frames {on}..{off_}, attack id {}",
            c.name,
            nr.request,
            p0[0],
            p0[1],
            p0[2],
            if line { format!(" → ({:.4}, {:.4}, {:.4})", p1[0], p1[1], p1[2]) } else { String::new() },
            row.radius * 0.01,
            nr.sub,
            nr.recom_id
        ));
        out_recs.push(r);
        }
    }
    let mut d = asd[..tail].to_vec();
    for r in &out_recs {
        d.extend_from_slice(r);
    }
    let new_tail = d.len();
    d.extend_from_slice(&asd[tail..]);
    d[0x10..0x14].copy_from_slice(&((n + out_recs.len()) as u32).to_le_bytes());
    d[0x18..0x1C].copy_from_slice(&(new_tail as u32).to_le_bytes());
    Ok((d, notes))
}

/// The record of an ASD whose attack id is `id`: (its motion name, its window start in frames).
fn template_of(asd: &[u8], id: u32) -> Option<(String, f32)> {
    let u = |o: usize| asd.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let (n, off) = (u(0x10)? as usize, u(0x14)? as usize);
    (0..n).map(|i| &asd[off + REC * i..off + REC * (i + 1)]).find(|r| u32::from_le_bytes(r[0x64..0x68].try_into().unwrap()) == id).map(|r| {
        let e = r[0x40..0x50].iter().position(|&c| c == 0).unwrap_or(16);
        (String::from_utf8_lossy(&r[0x40..0x40 + e]).into_owned(), u32::from_le_bytes(r[0x5C..0x60].try_into().unwrap()) as f32 / 50.0)
    })
}

/// The SES with Re:CoM's sounds on the new clips (Re:CoM's entries kept): a new clip paired with a
/// Re:CoM clip that has sounds gets them (same frames); every other attack clip gets its first
/// row's template record's sounds, moved by (KH1's hit start − Re:CoM's window start), a sound at
/// frame 0 kept at 0 (the Shadow's rule, M10).
pub fn extend_ses(ses: &[u8], asd: &[u8], ms: &MotionSet, clips: &[RequestClip], pairs: &[(String, String)], attacks: &[(u16, u32, u16)]) -> Result<Vec<u8>> {
    let mut entries = crate::ses::parse(ses)?;
    let find = |es: &[crate::ses::Entry], n: &str| es.iter().find(|e| e.name() == n).cloned();
    let set_frame = |mut e: [u8; 0x50], f: i32| {
        e[0x18..0x1C].copy_from_slice(&f.to_le_bytes());
        e
    };
    let frame_of = |e: &[u8; 0x50]| i32::from_le_bytes(e[0x18..0x1C].try_into().unwrap());
    let mut add: Vec<(String, [u8; 0x30], Vec<[u8; 0x50]>)> = Vec::new();
    for (new, old) in pairs {
        if let Some(src) = find(&entries, old) {
            add.push((new.clone(), src.head, src.events.clone()));
        }
    }
    for &(req, id, sub) in attacks {
        let c = clips.iter().find(|c| c.request == req).ok_or_else(|| crate::Error::Format(format!("request {req} has no clip")))?;
        if c.source == "recom" || add.iter().any(|a| a.0 == c.name) {
            continue;
        }
        let Some((tm, t_on)) = template_of(asd, id) else { continue };
        let Some(src) = find(&entries, &tm) else { continue };
        let (on, _) = hit_window(&ms.motions[c.entry], sub)?;
        let ev = src.events.iter().map(|e| if frame_of(e) == 0 { *e } else { set_frame(*e, (frame_of(e) as f32 - t_on + on).round().max(0.0) as i32) }).collect();
        add.push((c.name.clone(), src.head, ev));
    }
    for (name, head, events) in add {
        entries.retain(|e| e.name() != name);
        let mut h = head;
        h[..16].copy_from_slice(&name16(&name));
        entries.push(crate::ses::Entry { head: h, events });
    }
    Ok(crate::ses::write(&entries))
}

fn fmt_f(v: f32) -> String {
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

/// Builds everything for one enemy.
pub fn build(spec: &Spec, inp: &Inputs) -> Result<Built> {
    let ms = MotionSet::parse(&inp.mset)?;
    let skel = RecomSkeleton::parse(&inp.mdl)?;
    let conv = Converter::new(&inp.mdls, skel)?;
    let script = crate::stats::mdls_part(&inp.mdls, 5).ok_or_else(|| crate::Error::Format(format!("{}.mdls: no AI script (part 5)", spec.kh1)))?;
    let mut consts = script_consts(script, spec.script, spec.consts)?;
    check_requests(script, spec.script, spec.requests)?;
    for (n, v) in exe_consts() {
        consts.push((n.to_string(), v));
    }
    consts.extend(btltbl_consts(&inp.btltbl)?);
    consts.push(("weight".into(), weight(script, spec.records)?));
    let (ann, clips, pairs) = extend_ann(&inp.ann, &ms, &conv, spec.prefix, spec.recom_slots)?;
    // The attack rows → ASD records.
    let codes = bone_codes(&inp.mdls)?;
    let mut rows: Vec<(usize, AttackRow, u16)> = Vec::new();
    for a in spec.attacks {
        for r in a.rows {
            if rows.iter().any(|x| x.0 == r.at) {
                continue;
            }
            let row = attack_row(script, r.at, &codes)?;
            let sub = script.get(r.at + 0x34..r.at + 0x38).map(|b| u32::from_le_bytes(b.try_into().unwrap())).filter(|&s| s < 4).ok_or_else(|| crate::Error::Format(format!("attack row {:#x}: hit slot outside 0..3", r.at)))? as u16;
            rows.push((r.at, row, sub));
        }
    }
    let row_of = |at: usize| rows.iter().find(|x| x.0 == at).expect("read above");
    let mut new = Vec::new();
    let mut sound_rows = Vec::new();
    for a in spec.attacks {
        for (i, r) in a.rows.iter().enumerate() {
            let (_, row, sub) = row_of(r.at);
            new.push(NewRecord { request: a.request, row, sub: *sub, recom_id: r.recom_id });
            if i == 0 {
                sound_rows.push((a.request, r.recom_id, *sub));
            }
        }
    }
    let (mut asd, records) = extend_asd(&inp.asd, spec.asd, &conv, &ms, &new, &clips)?;
    let mut ses = extend_ses(&inp.ses, &inp.asd, &ms, &clips, &pairs, &sound_rows)?;
    let (mut ann, mut clips) = (ann, clips);
    apply_starts(spec.name, &ms, &conv, &mut ann, &mut clips, &mut asd, spec.asd.0, &mut ses)?;
    let parts = hurt_parts(&inp.mdls, &conv)?;
    // The attacks and their clash recoils (row `+0x40`, request in the low 16 bits; the first row's,
    // as KH1's clash reads the record that met the Guard: Assumption that an attack's rows agree,
    // checked below).
    let mut attacks = Vec::new();
    for a in spec.attacks {
        let rs: Vec<u32> = a.rows.iter().map(|r| row_of(r.at).1.recoil & 0xFFFF).collect();
        if rs.iter().any(|&x| x != rs[0]) {
            return bad(format!("{}: its rows' clash recoils differ ({rs:?})", a.name));
        }
        if !clips.iter().any(|c| c.request == a.request) {
            return bad(format!("{}: request {} has no motion", a.name, a.request));
        }
        attacks.push((a.name, a.request, rs[0], a.rows.iter().map(|r| r.at).collect::<Vec<_>>()));
    }
    let manifest = manifest_json(spec, &consts, &clips, &attacks, &parts);
    Ok(Built { ann, asd, ses, manifest, clips, consts, records, parts })
}

fn manifest_json(spec: &Spec, consts: &[(String, f32)], clips: &[RequestClip], attacks: &[(&str, u16, u32, Vec<usize>)], parts: &[(usize, usize)]) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"format\": \"{FORMAT}\",\n  \"enemy\": \"{}\",\n  \"recom_id\": {},\n  \"kh1\": \"{}\",\n", spec.name, spec.recom_id, spec.kh1));
    s.push_str("  \"consts\": {\n");
    let c: Vec<String> = consts.iter().map(|(k, v)| format!("    \"{k}\": {}", fmt_f(*v))).collect();
    s.push_str(&c.join(",\n"));
    s.push_str("\n  },\n  \"attacks\": [\n");
    let a: Vec<String> = attacks.iter().map(|(n, r, rc, rows)| format!("    {{\"name\": \"{n}\", \"request\": {r}, \"recoil\": {rc}, \"rows\": [{}]}}", rows.iter().map(|x| format!("\"{x:#x}\"")).collect::<Vec<_>>().join(", "))).collect();
    s.push_str(&a.join(",\n"));
    s.push_str("\n  ],\n");
    s.push_str(&format!("  \"parts\": [{}],\n", parts.iter().map(|p| p.1.to_string()).collect::<Vec<_>>().join(", ")));
    s.push_str(&format!("  \"parts_kh1\": [{}],\n", parts.iter().map(|p| p.0.to_string()).collect::<Vec<_>>().join(", ")));
    s.push_str("  \"requests\": [\n");
    let r: Vec<String> = clips
        .iter()
        .map(|c| format!("    {{\"request\": {}, \"entry\": {}, \"slot\": {}, \"name\": \"{}\", \"frames\": {}, \"root\": {}, \"loop\": {}, \"source\": \"{}\"}}", c.request, c.entry, c.slot, c.name, c.frames, c.root, c.looped, c.source))
        .collect();
    s.push_str(&r.join(",\n"));
    s.push_str("\n  ]\n}\n");
    s
}

/// Requests a script only ever plays from a later frame (KH1's request word `+4`: the start time of
/// the queue entry `0x14029F980` writes, added when the motion switches, `0x14029FC40`; Checked
/// (decomp); M11): (enemy, request, KH1 frame). Their clip is the motion from that frame
/// ([`Converter::convert_from`]), their attack records' windows and their sounds move by it, their
/// manifest length is the rest. The Air Soldier's slash: `0x22F2` `sys 13(wp, 201, 8, 20)`.
pub const STARTS: &[(&str, u16, u32)] = &[("airsoldier", 201, 20)];

/// [`STARTS`]' requests of an enemy.
pub fn starts_of(name: &str) -> Vec<(u16, u32)> {
    STARTS.iter().filter(|s| s.0 == name).map(|s| (s.1, s.2)).collect()
}

/// Applies [`STARTS`] to a built enemy: the clip (each such request must be its entry's only one),
/// the new records of `first..` bound to it (their windows move back by the start; a window that
/// would start before 0 begins at 0), the clip's sounds (frames move back, a frame-0 sound stays),
/// the manifest length.
#[allow(clippy::too_many_arguments)]
fn apply_starts(name: &str, ms: &MotionSet, conv: &Converter, ann: &mut Vec<u8>, clips: &mut [RequestClip], asd: &mut [u8], first: usize, ses: &mut Vec<u8>) -> Result<()> {
    let starts = starts_of(name);
    if starts.is_empty() {
        return Ok(());
    }
    let mut a = Ann::parse(ann)?;
    let mut sounds = crate::ses::parse(ses)?;
    for (req, start) in starts {
        let Some(i) = clips.iter().position(|c| c.request == req) else { return bad(format!("{name}: request {req} has no clip")) };
        let c = clips[i].clone();
        if c.source != "kh1" || clips.iter().any(|d| d.entry == c.entry && d.request != req) {
            return bad(format!("{name}: request {req} (entry {}) shares its clip; it can't start at {start}", c.entry));
        }
        let m = &ms.motions[c.entry];
        let clip = conv.convert_from(m, &ClipSpec { name: c.name.clone(), looped: c.looped, root_motion: c.root, blend: 8 }, start)?;
        a.slots[c.slot].clip = Some(clip);
        clips[i].frames = m.frames - start;
        let u = |d: &[u8], o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap());
        let (n, off) = (u(asd, 0x10) as usize, u(asd, 0x14) as usize);
        for r in first..n {
            let at = off + REC * r;
            if asd[at + 0x40..at + 0x50] == name16(&c.name) {
                let shift = start * 50;
                let on = u(asd, at + 0x5C).saturating_sub(shift);
                let offw = u(asd, at + 0x60).saturating_sub(shift);
                asd[at + 0x5C..at + 0x60].copy_from_slice(&on.to_le_bytes());
                asd[at + 0x60..at + 0x64].copy_from_slice(&offw.to_le_bytes());
            }
        }
        for e in sounds.iter_mut().filter(|e| e.name() == c.name) {
            for ev in e.events.iter_mut() {
                let f = i32::from_le_bytes(ev[0x18..0x1C].try_into().unwrap());
                if f != 0 {
                    ev[0x18..0x1C].copy_from_slice(&(f - start as i32).max(0).to_le_bytes());
                }
            }
        }
    }
    a.pad_tail_like_recom();
    *ann = a.to_bytes();
    *ses = crate::ses::write(&sounds);
    Ok(())
}

/// Every enemy this step builds (M11); the Shadow (M10) is `crate::shadow`.
pub const SPECS: &[&Spec] = &[&crate::soldier::SPEC, &crate::airsoldier::SPEC];
