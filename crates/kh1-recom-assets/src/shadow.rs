//! M10: KH1's Shadow in Re:CoM's battles (`docs/recom-m10-shadow.md`). From the player's own
//! files, the step makes Re:CoM's Shadow (`ENEMY/0001`) able to play KH1's Shadow natively:
//!
//! - `EN0001.ANN`: KH1's Shadow motions (`xa_ex_2020.mset` on `xa_ex_2020.mdls`'s skeleton) as new
//!   60 fps clips at slot 100 + KH1 entry (`e001_1NN`; Re:CoM's name lookup `0x1401A47E0` takes
//!   the slot from the name's digits). Re:CoM's 76 slots stay byte-identical. An entry Re:CoM
//!   already has as a bake (Checked (files): only entry 6 = `e001_005`) plays Re:CoM's clip
//!   (decision M10: a Re:CoM clip only where it bakes exactly the same KH1 motion).
//! - `EN0001.ASD`: Re:CoM's attack records (`@COP` v3, 0xA0 each; loader `0x140179340`, bound by
//!   motion name at load `0x14014DBE0`, ticked by `0x14014E830`) with new records for KH1's claw,
//!   swipe and leap claw on the new clips: KH1's HIT windows (the `.mset` HIT track, × 50 ticks),
//!   KH1's attack points (the script's attack rows: bone code → bone, `t` along it) in Re:CoM's
//!   hand bones, KH1's radius 35 (× the actor scale 1: Assumption). Re:CoM's records, body and
//!   trailing bytes stay byte-identical (the trailing section's offset moves).
//! - `EN0001.SES`: Re:CoM's two entries kept; the new attack clips get Re:CoM's Shadow attack
//!   sounds at the matching place of KH1's hit window (Assumption: KH1's own Shadow sounds are
//!   not mapped).
//! - `kh1-shadow.json`: every number the behaviour reads ([`kh1_core::shadow::Consts`]): read
//!   from the player's `ex_2020.bd` (`.mdls` part 5) at checked instruction offsets, from
//!   `btltbl.bin`'s globals and the Shadow's stat records; the request → slot map with KH1's
//!   frame counts; the clash recoil requests.

use kh1_formats::mset::MotionSet;

use crate::ann::name16;
#[cfg(test)]
use crate::ann::Ann;
use crate::convert::Converter;
use crate::mdl::RecomSkeleton;
use crate::{bad, Result};

/// Re:CoM's Shadow files (`Recom.hed` names).
pub const ANN_PATH: &str = "ENEMY/0001/EN0001.ANN";
/// Re:CoM's Shadow skeleton.
pub const MDL_PATH: &str = "ENEMY/0001/EN0001.MDL";
/// Re:CoM's Shadow attack records.
pub const ASD_PATH: &str = "ENEMY/0001/EN0001.ASD";
/// Re:CoM's Shadow motion sounds.
pub const SES_PATH: &str = "ENEMY/0001/EN0001.SES";
/// KH1's Shadow (`kh1_first.pkg`).
pub const KH1_MSET: &str = "xa_ex_2020.mset";
/// KH1's Shadow model (skeleton, bone codes, AI script in part 5).
pub const KH1_MDLS: &str = "xa_ex_2020.mdls";
/// The manifest the DLL reads, at the recom root.
pub const MANIFEST: &str = "kh1-shadow.json";
/// The manifest's format; the DLL refuses others.
pub const FORMAT: &str = "kh1-shadow 1";
/// Re:CoM Steam's slot count in `EN0001.ANN` (Checked (files)).
pub const RECOM_SLOTS: usize = 76;
/// New clips sit at this + KH1 entry.
pub const SLOT_BASE: usize = 100;
/// The script's name in its header (`+4`).
pub const SCRIPT_NAME: &[u8] = b"ex_2020.bd";
/// KH1's filler requests (0x141..) map entry 0 without root motion; the Shadow's code never
/// requests them (Checked (files + script)).
pub const LAST_REQUEST: usize = 0x140;

/// Where each constant is pushed in `ex_2020.bd` (`push f32`: an opcode word whose low byte is
/// 0x12 / 0x52, then the value). A name with several offsets must read the same at each. The
/// offsets are the script's own (Checked (files): the player's script disassembled; the function
/// each sits in is in `docs/recom-m10-shadow.md`).
pub const SCRIPT_CONSTS: &[(&str, &[usize])] = &[
    ("close_range", &[0xBA2, 0xC5A]),
    ("mid_range", &[0xBBC, 0xC4A]),
    ("swipe_range", &[0xC76]),
    ("claw_range", &[0xC90]),
    ("claw_chance", &[0xCA8]),
    ("close_taunt_chance", &[0xCD4]),
    ("after_taunt_chance", &[0xD00]),
    ("after_dig_chance", &[0xD20]),
    ("far_fraction", &[0xC26]),
    ("far_approach_chance", &[0xC3C]),
    ("idle_search", &[0x2728]),
    ("engaged_search", &[0xBE4]),
    ("taunt_blend", &[0xDF4]),
    ("taunt_wait_min", &[0xDFC]),
    ("taunt_wait_max", &[0xE02]),
    ("approach_timeout", &[0xFD2]),
    ("approach_retarget", &[0xFD8]),
    ("idle_dig_a_min", &[0xD66]),
    ("idle_dig_a_max", &[0xD6C]),
    ("idle_dig_b_min", &[0xD86]),
    ("idle_dig_b_max", &[0xD8C]),
    ("wander_inner", &[0xDC4]),
    ("wander_outer", &[0xDCA]),
    ("wander_band", &[0x1BD8, 0x1BFC]),
    ("wander_timeout", &[0x1C26, 0x1C40, 0x1CC0]),
    ("wander_idle_chance", &[0x1C56]),
    ("wander_idle_min", &[0x1C76]),
    ("wander_idle_max", &[0x1C7C]),
    ("wander_home_max", &[0x1CA4]),
    ("near_dig_min", &[0xFF2]),
    ("near_dig_max", &[0xFF8]),
    ("near_dig_stop", &[0x1012]),
    ("near_dig_target", &[0x1026]),
    ("travel_timeout", &[0x105E]),
    ("stuck_factor", &[0x1096]),
    ("weave_amplitude", &[0x10EA]),
    ("weave_rand", &[0x110A]),
    ("weave_base", &[0x1112]),
    ("under_speed", &[0x1128]),
    ("sink_extra", &[0x139C]),
    ("rise_trim", &[0x1222]),
    ("first_rise_fade", &[0x11A0]),
    ("leap_height", &[0xE18]),
    ("leap_max_speed", &[0xE1E]),
    ("leap_offset", &[0xE24]),
    ("leap_divisor", &[0xE2A]),
    ("leap_attack_after", &[0xEA0]),
    ("air_steer_speed", &[0xE86]),
    ("attack_blend", &[0xF44, 0xF9A, 0x1340, 0x11D0, 0x1C6E]),
    ("walk_speed", &[0x280C]),
    ("run_speed", &[0x281E]),
    ("hop_height", &[0x2830]),
];

/// Numbers of KH1's exe the behaviour reads (not in any file): KH1's stats-block defaults
/// (`0x1402A5D70`: air acceleration `+0x14` 0.1, air drag `+0x18` 0.95, turn rate `+0xD0` π/16),
/// gravity (`0x1403EAB54`) and the fall cap 20 (`0x1402AF6A0`), the launch's down time 30
/// (`0x1402B8530`) and the 20 hop of the air reactions and clash (`0x1402B8B90`, `0x1402B77D0`,
/// `0x1402B9140`). Checked (decomp + exe); the same values `kh1-core` uses for Sora.
pub fn exe_consts() -> Vec<(&'static str, f32)> {
    vec![
        ("air_accel", 0.1),
        ("air_drag", 0.95),
        ("turn_rate", std::f32::consts::PI / 16.0),
        ("gravity", kh1_core_gravity()),
        ("fall_cap", 20.0),
        ("down_time", 30.0),
        ("air_knock_hop", 20.0),
    ]
}

fn kh1_core_gravity() -> f32 {
    // `0x1403EAB54` (Checked (exe)); kept here so this crate needs no kh1-core.
    0.484_693_88
}

/// The attack rows (script offsets, 0x70 bytes; `attack_new` = `sys 23` → `0x1402B2DC0`): claw,
/// swipe, leap claw (right), leap claw (left).
pub const ATTACK_ROWS: [usize; 4] = [0x884, 0x8F4, 0x964, 0x9D4];
/// Their requests (the motion each is made with in the script: `0xF6E`, `0xF18`, `0xE12`).
pub const ATTACK_REQUESTS: [u16; 4] = [200, 204, 201, 201];
/// KH1's Shadow hurt parts' bones: part 0 (the head ellipsoid) on bone 30, part 1 (the hips) on
/// bone 2 (Checked (files): `xa_ex_2020.mdls`'s collision block).
pub const HURT_PART_BONES: [usize; 2] = [30, 2];
/// Re:CoM attack id (`@ENP` attack entry `+0x18`) each takes its power from (decision M10: the
/// claw and the swipe Re:CoM's claw, the leap claw Re:CoM's leap; Re:CoM's records: `e001_050`
/// id 0, `e001_052` id 1, Checked (files)).
pub const ATTACK_IDS: [u32; 4] = [0, 0, 1, 1];

/// One attack row as the port uses it.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackRow {
    /// Bone code (`+0x10`) and the KH1 bone it names (the model's code table).
    pub code: u32,
    /// The KH1 bone.
    pub bone: usize,
    /// `t` of the two ends along the bone (`+0x54`, `+0x58`) and the radius (`+0x5C`).
    pub t: [f32; 2],
    /// The radius (KH1 units).
    pub radius: f32,
    /// The clash recoil (`+0x40`: request in the low 16 bits, bit 22 the push).
    pub recoil: u32,
}

/// Reads an attack row (the script must push its address).
pub fn attack_row(script: &[u8], at: usize, codes: &[i32]) -> Result<AttackRow> {
    let r = script.get(at..at + 0x70).ok_or_else(|| crate::Error::Format(format!("ex_2020.bd: no attack row at {at:#x}")))?;
    if !crate::stats::pushes_address(script, at as u32) {
        return bad(format!("ex_2020.bd: the script never pushes the attack row {at:#x}"));
    }
    let u = |o: usize| u32::from_le_bytes(r[o..o + 4].try_into().unwrap());
    let f = |o: usize| f32::from_le_bytes(r[o..o + 4].try_into().unwrap());
    let code = u(0x10);
    let bone = *codes.get(code as usize).ok_or_else(|| crate::Error::Format(format!("attack row {at:#x}: bone code {code} outside the table")))?;
    if bone < 0 {
        return bad(format!("attack row {at:#x}: bone code {code} names no bone"));
    }
    let (t, radius) = ([f(0x54), f(0x58)], f(0x5C));
    // `t` may be negative (a point past the parent, away from the bone: the Air Soldier's spin row
    // `0x4DC` runs from 1 to −1; M11 fliers).
    if !(t.iter().all(|x| x.is_finite() && (-100.0..=100.0).contains(x)) && radius > 0.0 && radius < 1000.0) {
        return bad(format!("attack row {at:#x}: t {t:?} radius {radius}"));
    }
    Ok(AttackRow { code, bone: bone as usize, t, radius, recoil: u(0x40) })
}

/// The model's bone code table (`MOBJ` → model at `MOBJ + [MOBJ + 0x20]`; the model's header
/// (joint count, joint table, pair table, mesh count), 16-byte mesh headers, 0x20 bytes, then 4
/// floats and one i32 bone per code). Checked (files + decomp: KH1's bone point `sys 19` /
/// `0x1402B2DC0` read `+0x10 + code × 4`). Each id must be a joint or −1.
pub fn bone_codes(mdls: &[u8]) -> Result<Vec<i32>> {
    let u = |o: usize| -> Result<u32> { mdls.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| crate::Error::Format(format!("xa_ex_2020.mdls: short at {o:#x}"))) };
    let mobj = 0x80;
    if mdls.get(mobj..mobj + 4) != Some(b"MOBJ".as_slice()) {
        return bad("xa_ex_2020.mdls: no MOBJ at 0x80");
    }
    let model = mobj + u(mobj + 0x20)? as usize;
    let joints = u(model)? as usize;
    let meshes = u(model + 0xC)? as usize;
    if !(1..=512).contains(&joints) || meshes > 64 {
        return bad(format!("xa_ex_2020.mdls: model header {joints} joints, {meshes} meshes"));
    }
    let table = model + 0x10 + 0x10 * meshes + 0x20;
    let ids: Vec<i32> = (0..56).map(|c| u(table + 0x10 + 4 * c).map(|v| v as i32)).collect::<Result<_>>()?;
    if let Some(bad_id) = ids.iter().find(|&&v| v != -1 && !(0..joints as i32).contains(&v)) {
        return bad(format!("xa_ex_2020.mdls: bone code table has {bad_id} (joints {joints})"));
    }
    Ok(ids)
}

/// Reads the constants at [`SCRIPT_CONSTS`].
pub fn script_consts(script: &[u8]) -> Result<Vec<(String, f32)>> {
    if script.get(4..4 + SCRIPT_NAME.len()) != Some(SCRIPT_NAME) {
        return bad("xa_ex_2020.mdls part 5 is not ex_2020.bd");
    }
    let mut out = Vec::new();
    for (name, offs) in SCRIPT_CONSTS {
        let mut v: Option<f32> = None;
        for &o in offs.iter() {
            let w = script.get(o..o + 6).ok_or_else(|| crate::Error::Format(format!("ex_2020.bd: short at {o:#x}")))?;
            if w[0] & 0xBF != 0x12 {
                return bad(format!("ex_2020.bd {o:#x} ({name}): not a push (opcode {:02x}{:02x}); not the script this step knows", w[0], w[1]));
            }
            let x = f32::from_le_bytes(w[2..6].try_into().unwrap());
            if !x.is_finite() {
                return bad(format!("ex_2020.bd {o:#x} ({name}): {x}"));
            }
            match v {
                Some(p) if p != x => return bad(format!("ex_2020.bd: {name} is {p} and {x} at {offs:x?}")),
                _ => v = Some(x),
            }
        }
        out.push((name.to_string(), v.unwrap()));
    }
    Ok(out)
}

/// The Shadow's knock-back weight (stat record `+0x18`, r6) from every per-world record; they must
/// agree (Checked (files): 1.0 in all 14).
pub fn weight(script: &[u8]) -> Result<f32> {
    let mut w: Option<f32> = None;
    for at in crate::stats::shadow_record_offsets() {
        let o = at as usize + 0x18;
        let x = script.get(o..o + 4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| crate::Error::Format(format!("ex_2020.bd: no record at {at:#x}")))?;
        match w {
            Some(p) if p != x => return bad(format!("ex_2020.bd: knock-back weight {p} and {x} in different worlds")),
            _ => w = Some(x),
        }
    }
    w.ok_or_else(|| crate::Error::Format("no Shadow records".into()))
}

/// Inputs: KH1's Shadow, Re:CoM's Shadow, KH1's `btltbl.bin`.
#[derive(Clone, Debug)]
pub struct Inputs {
    /// KH1 `xa_ex_2020.mset`.
    pub mset: Vec<u8>,
    /// KH1 `xa_ex_2020.mdls`.
    pub mdls: Vec<u8>,
    /// Re:CoM `EN0001.ANN`.
    pub ann: Vec<u8>,
    /// Re:CoM `EN0001.MDL`.
    pub mdl: Vec<u8>,
    /// Re:CoM `EN0001.ASD`.
    pub asd: Vec<u8>,
    /// Re:CoM `EN0001.SES`.
    pub ses: Vec<u8>,
    /// KH1 `btltbl.bin`.
    pub btltbl: Vec<u8>,
}

/// One request's clip.
#[derive(Clone, Debug, PartialEq)]
pub struct RequestClip {
    /// KH1 motion request.
    pub request: u16,
    /// KH1 mset entry.
    pub entry: usize,
    /// Re:CoM slot.
    pub slot: usize,
    /// Re:CoM clip name.
    pub name: String,
    /// KH1's length (60 Hz frames; `sys 43`).
    pub frames: u32,
    /// Root motion (request-map bit 15).
    pub root: bool,
    /// The clip loops.
    pub looped: bool,
    /// "recom" (a bake Re:CoM has) or "kh1".
    pub source: &'static str,
}

/// The step's output.
#[derive(Clone, Debug)]
pub struct Built {
    /// `EN0001.ANN`.
    pub ann: Vec<u8>,
    /// `EN0001.ASD`.
    pub asd: Vec<u8>,
    /// `EN0001.SES`.
    pub ses: Vec<u8>,
    /// `kh1-shadow.json`.
    pub manifest: String,
    /// Every request's clip.
    pub clips: Vec<RequestClip>,
    /// The behaviour's numbers.
    pub consts: Vec<(String, f32)>,
    /// Records added to the ASD: (motion name, bone, offset, radius, start, end ticks, id).
    pub records: Vec<String>,
}

/// An attack record's size (the tests read the records).
#[cfg(test)]
const REC: usize = 0xA0;

/// Builds everything.
pub fn build(inp: &Inputs) -> Result<Built> {
    let ms = MotionSet::parse(&inp.mset)?;
    let skel = RecomSkeleton::parse(&inp.mdl)?;
    let conv = Converter::new(&inp.mdls, skel)?;
    let script = crate::stats::mdls_part(&inp.mdls, 5).ok_or_else(|| crate::Error::Format("xa_ex_2020.mdls: no AI script (part 5)".into()))?;
    // The constants.
    let mut consts = script_consts(script)?;
    for (n, v) in exe_consts() {
        consts.push((n.to_string(), v));
    }
    let g = kh1_formats::btltbl::Btltbl::parse(inp.btltbl.clone())?.globals()?;
    consts.push(("launch_min".into(), g.launch_speed_min as f32));
    consts.push(("launch_max".into(), g.launch_speed_max as f32));
    consts.push(("launch_hop".into(), g.launch_height_factor));
    consts.push(("flinch_push".into(), g.reaction_pushes[4]));
    consts.push(("air_flinch_speed".into(), g.air_knockback[0]));
    consts.push(("air_knock_speed".into(), g.air_knockback[1]));
    consts.push(("weight".into(), weight(script)?));

    // Requests → entries (filler requests excluded), root flags; the clips (M11: the shared
    // builder, the same rules).
    let (ann_bytes, clips, _pairs) = crate::enemy::extend_ann(&inp.ann, &ms, &conv, "e001_", RECOM_SLOTS).map_err(|e| crate::Error::Format(format!("EN0001.ANN: {e}")))?;

    // The attack rows → ASD records.
    let codes = bone_codes(&inp.mdls)?;
    let rows: Vec<AttackRow> = ATTACK_ROWS.iter().map(|&a| attack_row(script, a, &codes)).collect::<Result<_>>()?;
    // M11: the shared record writer: the claw and the swipe on Re:CoM's claw record (attack id 0),
    // the leap claw's two rows on its leap record (id 1), every row on hit slot 0 (their `+0x34`).
    let new: Vec<crate::enemy::NewRecord> = (0..ATTACK_ROWS.len()).map(|i| crate::enemy::NewRecord { request: ATTACK_REQUESTS[i], row: &rows[i], sub: 0, recom_id: ATTACK_IDS[i] }).collect();
    let (asd, records) = crate::enemy::extend_asd(&inp.asd, (3, 0x6C, 0x24C), &conv, &ms, &new, &clips).map_err(|e| crate::Error::Format(format!("EN0001.ASD: {e}")))?;
    // Sounds.
    let ses = extend_ses(&inp.ses, &ms, &clips)?;

    let recoil: Vec<u32> = rows.iter().map(|r| r.recoil & 0xFFFF).collect();
    for r in &recoil {
        if !clips.iter().any(|c| c.request as u32 == *r) {
            return bad(format!("clash recoil request {r} has no motion"));
        }
    }
    // Check the constants against the behaviour's list.
    let names: Vec<&str> = consts.iter().map(|c| c.0.as_str()).collect();
    // KH1's hurt parts (part 0 on bone 30, part 1 on bone 2: the `.mdls` collision block's hurt
    // entries, Checked (files)) as Re:CoM bones, for the DLL's "part from the hit's height".
    let parts: Vec<usize> = HURT_PART_BONES
        .iter()
        .map(|&k| conv.map().iter().position(|m| *m == Some(k)).ok_or_else(|| crate::Error::Format(format!("KH1 bone {k} has no Re:CoM bone"))))
        .collect::<Result<_>>()?;
    let manifest = manifest_json(&consts, &clips, &recoil, &parts, &names);
    Ok(Built { ann: ann_bytes, asd, ses, manifest, clips, consts, records })
}

fn hit_window(m: &kh1_formats::mset::Motion) -> Result<(f32, f32)> {
    // The HIT track's sub 0 (`kh1-data`'s rule: track HIT, key 1 on, key 0 off).
    let t = m.tracks.iter().find(|t| t.kind == kh1_formats::mset::track::HIT && t.sub == 0).ok_or_else(|| crate::Error::Format("attack motion without a HIT track".into()))?;
    let on = t.keys.iter().find(|k| k.value() != 0).map(|k| k.time).ok_or_else(|| crate::Error::Format("HIT track never on".into()))?;
    let off = t.keys.iter().find(|k| k.value() == 0 && k.time > on).map(|k| k.time).unwrap_or(m.frames as f32);
    Ok((on, off))
}

/// Re:CoM's `EN0001.SES` with Re:CoM's Shadow attack sounds on the new attack clips: the claw's
/// event (`e001_050`, one hit-window sound) at KH1's hit start + 1 on the claw and the swipe
/// (Re:CoM's sits 1 frame after its own window opens: 46 vs 45, Checked (files)); the leap
/// claw's two events (`e001_052`: at 0 and 6 frames after its window opens, 24 vs 18) at 0 and at
/// KH1's hit start + 6. Assumption: KH1's own sounds are not mapped.
fn extend_ses(ses: &[u8], ms: &MotionSet, clips: &[RequestClip]) -> Result<Vec<u8>> {
    let mut entries = crate::ses::parse(ses)?;
    let find = |n: &str| entries.iter().find(|e| e.name() == n).cloned().ok_or_else(|| crate::Error::Format(format!("EN0001.SES: no {n}")));
    let claw = find("e001_050")?;
    let leap = find("e001_052")?;
    let frame_of = |e: &[u8; 0x50]| i32::from_le_bytes(e[0x18..0x1C].try_into().unwrap());
    let set_frame = |mut e: [u8; 0x50], f: i32| {
        e[0x18..0x1C].copy_from_slice(&f.to_le_bytes());
        e
    };
    let clip_of = |r: u16| clips.iter().find(|c| c.request == r).cloned().ok_or_else(|| crate::Error::Format(format!("request {r} has no clip")));
    let mut add = Vec::new();
    for r in [200u16, 204] {
        let c = clip_of(r)?;
        let (on, _) = hit_window(&ms.motions[c.entry])?;
        let ev: Vec<[u8; 0x50]> = claw.events.iter().map(|e| set_frame(*e, on as i32 + 1)).collect();
        add.push((c.name.clone(), claw.head, ev));
    }
    {
        let c = clip_of(201)?;
        let (on, _) = hit_window(&ms.motions[c.entry])?;
        let ev: Vec<[u8; 0x50]> = leap.events.iter().map(|e| if frame_of(e) == 0 { *e } else { set_frame(*e, on as i32 + 6) }).collect();
        add.push((c.name.clone(), leap.head, ev));
    }
    for (name, head, events) in add {
        entries.retain(|e| e.name() != name);
        let mut h = head;
        h[..16].copy_from_slice(&name16(&name));
        entries.push(crate::ses::Entry { head: h, events });
    }
    Ok(crate::ses::write(&entries))
}

fn manifest_json(consts: &[(String, f32)], clips: &[RequestClip], recoil: &[u32], parts: &[usize], _names: &[&str]) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"format\": \"{FORMAT}\",\n"));
    s.push_str("  \"consts\": {\n");
    let c: Vec<String> = consts.iter().map(|(k, v)| format!("    \"{k}\": {}", fmt_f(*v))).collect();
    s.push_str(&c.join(",\n"));
    s.push_str("\n  },\n");
    s.push_str(&format!("  \"recoil\": [{}],\n", recoil.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ")));
    s.push_str(&format!("  \"parts\": [{}],\n", parts.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ")));
    s.push_str("  \"requests\": [\n");
    let r: Vec<String> = clips
        .iter()
        .map(|c| format!("    {{\"request\": {}, \"entry\": {}, \"slot\": {}, \"name\": \"{}\", \"frames\": {}, \"root\": {}, \"loop\": {}, \"source\": \"{}\"}}", c.request, c.entry, c.slot, c.name, c.frames, c.root, c.looped, c.source))
        .collect();
    s.push_str(&r.join(",\n"));
    s.push_str("\n  ]\n}\n");
    s
}

fn fmt_f(v: f32) -> String {
    // Shortest text that reads back as the same f32.
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The player's files: `KH1_SHADOW_DIR` (KH1's `xa_ex_2020.mset` / `.mdls` and Re:CoM's
    /// `ENEMY__0001__EN0001.*`) and `KH1_TEST_DATA` (`btltbl.bin`).
    fn inputs() -> Option<Inputs> {
        let d = std::path::PathBuf::from(std::env::var_os("KH1_SHADOW_DIR")?);
        let t = std::path::PathBuf::from(std::env::var_os("KH1_TEST_DATA")?);
        let r = |n: &str| std::fs::read(d.join(n)).ok();
        Some(Inputs {
            mset: r("xa_ex_2020.mset")?,
            mdls: r("xa_ex_2020.mdls")?,
            ann: r("ENEMY__0001__EN0001.ANN")?,
            mdl: r("ENEMY__0001__EN0001.MDL")?,
            asd: r("ENEMY__0001__EN0001.ASD")?,
            ses: r("ENEMY__0001__EN0001.SES")?,
            btltbl: std::fs::read(t.join("btltbl.bin")).ok()?,
        })
    }

    #[test]
    fn builds_from_the_players_files() {
        let Some(inp) = inputs() else { return };
        let b = build(&inp).unwrap();
        // Every behaviour constant is there, and the script's numbers are the decomp's.
        let consts = kh1_core::shadow::Consts::new(b.consts.clone()).unwrap();
        assert_eq!((consts.close_range(), consts.mid_range(), consts.swipe_range(), consts.claw_range()), (450.0, 800.0, 150.0, 300.0));
        assert_eq!((consts.leap_height(), consts.leap_max_speed(), consts.leap_attack_after()), (150.0, 8.0, 16.0));
        assert_eq!((consts.walk_speed(), consts.run_speed(), consts.under_speed()), (2.0, 5.0, 6.0));
        assert_eq!((consts.flinch_push(), consts.air_flinch_speed(), consts.air_knock_speed(), consts.weight()), (80.0, 4.0, 16.0, 1.0));
        // Entry 6 is Re:CoM's bake e001_005; the attacks are new clips.
        let c = |r: u16| b.clips.iter().find(|c| c.request == r).unwrap().clone();
        assert_eq!((c(6).slot, c(6).source), (5, "recom"));
        assert_eq!((c(200).slot, c(200).name.as_str(), c(200).frames, c(200).root), (145, "e001_145", 92, true));
        assert_eq!((c(69).frames, c(70).frames), (80, 60));
        // Re:CoM's 76 slots unchanged; the new clips parse.
        let a0 = Ann::parse(&inp.ann).unwrap();
        let a1 = Ann::parse(&b.ann).unwrap();
        for i in 0..RECOM_SLOTS {
            assert_eq!(a0.slots[i], a1.slots[i], "slot {i}");
        }
        assert!(a1.slots[145].clip.is_some());
        // ASD: Re:CoM's bytes kept, 4 records added (claw, swipe, leap ×2), tail moved.
        assert_eq!(&b.asd[..0x10], &inp.asd[..0x10]);
        assert_eq!(&b.asd[0x6C..0x24C], &inp.asd[0x6C..0x24C]);
        assert_eq!(u32::from_le_bytes(b.asd[0x10..0x14].try_into().unwrap()), 7);
        assert_eq!(&b.asd[0x24C + 4 * REC..], &inp.asd[0x24C..]);
        assert_eq!(b.records.len(), 4, "{:?}", b.records);
        let rec = &b.asd[0x24C..0x24C + REC];
        assert_eq!(&rec[0x40..0x48], b"e001_145");
        assert_eq!(u32::from_le_bytes(rec[0x5C..0x60].try_into().unwrap()), 2600);
        assert_eq!(u32::from_le_bytes(rec[0x60..0x64].try_into().unwrap()), 3200);
        // SES: Re:CoM's two entries and three new ones.
        let s = crate::ses::parse(&b.ses).unwrap();
        assert_eq!(s.len(), 5);
        // The manifest is the DLL's format.
        assert!(b.manifest.contains("\"format\": \"kh1-shadow 1\""));
        let head = b.manifest.find("\"parts\": [").unwrap();
        println!("{}", &b.manifest[head..head + 20]);
        std::fs::write(std::env::temp_dir().join("kh1-shadow-test.json"), &b.manifest).unwrap();
        println!("{}", b.records.join("\n"));
    }
}
