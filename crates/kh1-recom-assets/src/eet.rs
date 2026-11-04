//! Re:CoM's per-motion effect events (`.EET`: `PLAYER/0001/PL0001.EET`, `PLAYER/0002/PL0002.EET`),
//! read and written whole (M13 stage 3).
//!
//! Format (Checked (files + decomp: reader `0x140179B10`, start `0x14017A240`, lists `0x14017A710` /
//! `0x14017AB70`); `docs/recom-m13-recom-fx.md` § 1): `"@EET"`, u32 version 2, u32 record count, u32
//! record table offset; 0x30-byte records `{name[0x20], i32 motion index (−1 in the file, bound at load
//! by name), i16 list-A count, i16 list-B count, u32 list-A offset, u32 list-B offset}`; 0xA0-byte events,
//! every list A first, then every list B.
//! `0x14017A240` walks **every** record bound to the motion and takes the events whose variant is the
//! one passed (list A `+0x5C`, list B `+0x5C`), so records of the same name may be appended: Re:CoM's
//! own records keep their bytes and only move.

use crate::{Error, Result};

/// One record's size.
pub const RECORD: usize = 0x30;
/// One event's size.
pub const EVENT: usize = 0xA0;

/// One record: its name and its events (raw).
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// The clip name (`w001_055`), at most 31 bytes.
    pub name: String,
    /// List A: one-node effects.
    pub a: Vec<[u8; EVENT]>,
    /// List B: three-node effects (trails).
    pub b: Vec<[u8; EVENT]>,
    /// The name field as read (Square leaves bytes after the NUL; kept so Re:CoM's records are
    /// written back byte for byte). None for a new record (zero-filled).
    pub raw_name: Option<[u8; 0x20]>,
}

fn u32_at(d: &[u8], o: usize) -> Result<u32> {
    d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| Error::Format(format!("EET: short at {o:#x}")))
}

/// Reads an `.EET`.
pub fn parse(d: &[u8]) -> Result<Vec<Record>> {
    if d.get(..4) != Some(b"@EET".as_slice()) || u32_at(d, 4)? & 0x7FFF_FFFF != 2 {
        return Err(Error::Format("EET: not an @EET version 2 file".into()));
    }
    let n = u32_at(d, 8)? as usize;
    let off = u32_at(d, 12)? as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let r = off + RECORD * i;
        let head = d.get(r..r + RECORD).ok_or_else(|| Error::Format("EET: record outside the file".into()))?;
        let end = head[..0x20].iter().position(|&c| c == 0).unwrap_or(0x20);
        let name = String::from_utf8_lossy(&head[..end]).into_owned();
        let na = i16::from_le_bytes([head[0x24], head[0x25]]).max(0) as usize;
        let nb = i16::from_le_bytes([head[0x26], head[0x27]]).max(0) as usize;
        let (oa, ob) = (u32_at(head, 0x28)? as usize, u32_at(head, 0x2C)? as usize);
        let list = |o: usize, k: usize| -> Result<Vec<[u8; EVENT]>> {
            (0..k).map(|j| d.get(o + EVENT * j..o + EVENT * (j + 1)).map(|b| b.try_into().unwrap()).ok_or_else(|| Error::Format(format!("EET: {name}: event outside the file")))).collect()
        };
        let raw_name: [u8; 0x20] = head[..0x20].try_into().unwrap();
        out.push(Record { a: list(oa, na)?, b: list(ob, nb)?, name, raw_name: Some(raw_name) });
    }
    Ok(out)
}

/// Writes records: the header, the records, every record's list-A events, then every record's list-B
/// events (Re:CoM's layout).
pub fn write(records: &[Record]) -> Result<Vec<u8>> {
    let mut d = Vec::new();
    d.extend_from_slice(b"@EET");
    d.extend_from_slice(&2u32.to_le_bytes());
    d.extend_from_slice(&(records.len() as u32).to_le_bytes());
    d.extend_from_slice(&0x10u32.to_le_bytes());
    // Re:CoM's own files put every record's list A first, then every record's list B (Checked
    // (files): `PL0002.EET`'s first list B sits after all of its list-A events).
    let mut ev = 0x10 + RECORD * records.len();
    let mut ev_b = ev + EVENT * records.iter().map(|r| r.a.len()).sum::<usize>();
    for r in records {
        if r.name.is_empty() || r.name.len() > 0x1F || !r.name.is_ascii() || r.a.len() > i16::MAX as usize || r.b.len() > i16::MAX as usize {
            return Err(Error::Format(format!("EET: record {:?} does not fit", r.name)));
        }
        let mut h = [0u8; RECORD];
        match r.raw_name {
            Some(raw) if raw.starts_with(r.name.as_bytes()) && raw.get(r.name.len()) == Some(&0) => h[..0x20].copy_from_slice(&raw),
            _ => h[..r.name.len()].copy_from_slice(r.name.as_bytes()),
        }
        h[0x20..0x24].copy_from_slice(&(-1i32).to_le_bytes());
        h[0x24..0x26].copy_from_slice(&(r.a.len() as i16).to_le_bytes());
        h[0x26..0x28].copy_from_slice(&(r.b.len() as i16).to_le_bytes());
        let oa = if r.a.is_empty() { 0 } else { ev };
        ev += EVENT * r.a.len();
        let ob = if r.b.is_empty() { 0 } else { ev_b };
        ev_b += EVENT * r.b.len();
        h[0x28..0x2C].copy_from_slice(&(oa as u32).to_le_bytes());
        h[0x2C..0x30].copy_from_slice(&(ob as u32).to_le_bytes());
        d.extend_from_slice(&h);
    }
    for e in records.iter().flat_map(|r| r.a.iter()).chain(records.iter().flat_map(|r| r.b.iter())) {
        d.extend_from_slice(e);
    }
    Ok(d)
}

/// A list-B event's fields the step writes (`0x14017AB70`): effect set name and id, three nodes
/// (bone, offset), start / end frame (60 Hz, as KH1's), variant, fade.
#[derive(Clone, Debug, PartialEq)]
pub struct Trail {
    /// The effect set (`ex_5010.dpx`).
    pub dpx: String,
    /// The effect id in the set.
    pub id: i32,
    /// Node A, B, C: bone and offset.
    pub nodes: [(i32, [f32; 3]); 3],
    /// Start frame.
    pub start: i32,
    /// End frame.
    pub end: i32,
    /// The variant the motion start must pass.
    pub variant: i32,
    /// The fade on stop (frames; ≤ 0 kills it).
    pub fade: f32,
}

impl Trail {
    /// Reads a list-B event.
    pub fn read(e: &[u8; EVENT]) -> Trail {
        let i = |o: usize| i32::from_le_bytes(e[o..o + 4].try_into().unwrap());
        let f = |o: usize| f32::from_le_bytes(e[o..o + 4].try_into().unwrap());
        let end = e[..0x20].iter().position(|&c| c == 0).unwrap_or(0x20);
        let node = |b: usize| (i(b), [f(b + 4), f(b + 8), f(b + 12)]);
        Trail { dpx: String::from_utf8_lossy(&e[..end]).into_owned(), id: i(0x20), nodes: [node(0x24), node(0x34), node(0x44)], start: i(0x54), end: i(0x58), variant: i(0x5C), fade: f(0x60) }
    }

    /// The event, starting from `template` (a list-B event of Re:CoM's: its other bytes kept).
    pub fn write(&self, template: &[u8; EVENT]) -> Result<[u8; EVENT]> {
        if self.dpx.is_empty() || self.dpx.len() > 0x1F {
            return Err(Error::Format(format!("EET: effect name {:?}", self.dpx)));
        }
        let mut e = *template;
        e[..0x20].fill(0);
        e[..self.dpx.len()].copy_from_slice(self.dpx.as_bytes());
        let mut put = |o: usize, b: [u8; 4]| e[o..o + 4].copy_from_slice(&b);
        put(0x20, self.id.to_le_bytes());
        for (k, (bone, off)) in self.nodes.iter().enumerate() {
            let b = 0x24 + 0x10 * k;
            put(b, bone.to_le_bytes());
            for (j, v) in off.iter().enumerate() {
                put(b + 4 + 4 * j, v.to_le_bytes());
            }
        }
        put(0x54, self.start.to_le_bytes());
        put(0x58, self.end.to_le_bytes());
        put(0x5C, self.variant.to_le_bytes());
        put(0x60, self.fade.to_le_bytes());
        Ok(e)
    }
}

/// A list-A event's fields the step writes (`0x14017A710`): effect set name and id, bone, offset, start /
/// end frame, rotation (degrees), scale, flags, fade, variant.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    /// The effect set.
    pub dpx: String,
    /// The effect id.
    pub id: i32,
    /// The bone.
    pub bone: i32,
    /// The offset from it.
    pub offset: [f32; 3],
    /// Start frame.
    pub start: i32,
    /// End frame (−1: until the motion ends).
    pub end: i32,
    /// Rotation (degrees).
    pub rot: [f32; 3],
    /// Scale.
    pub scale: [f32; 3],
    /// Flags (bit 0 detached, 1 position only, 5 survives the motion change, …).
    pub flags: u32,
    /// The fade on stop.
    pub fade: f32,
    /// The variant.
    pub variant: i32,
}

impl Point {
    /// The event (repeat count 0 → Re:CoM's −1, time skip 0, tag 0).
    pub fn write(&self) -> Result<[u8; EVENT]> {
        if self.dpx.is_empty() || self.dpx.len() > 0x1F {
            return Err(Error::Format(format!("EET: effect name {:?}", self.dpx)));
        }
        let mut e = [0u8; EVENT];
        e[..self.dpx.len()].copy_from_slice(self.dpx.as_bytes());
        let mut put = |o: usize, b: [u8; 4]| e[o..o + 4].copy_from_slice(&b);
        put(0x20, self.id.to_le_bytes());
        put(0x24, self.bone.to_le_bytes());
        for j in 0..3 {
            put(0x28 + 4 * j, self.offset[j].to_le_bytes());
            put(0x3C + 4 * j, self.rot[j].to_le_bytes());
            put(0x48 + 4 * j, self.scale[j].to_le_bytes());
        }
        put(0x34, self.start.to_le_bytes());
        put(0x38, self.end.to_le_bytes());
        put(0x54, self.flags.to_le_bytes());
        put(0x58, self.fade.to_le_bytes());
        put(0x5C, self.variant.to_le_bytes());
        Ok(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trail(start: i32, end: i32, variant: i32) -> Trail {
        Trail { dpx: "ex_5010.dpx".into(), id: 5, nodes: [(2, [-0.11, 0.0, 0.0]), (2, [-0.78, 0.0, 0.0]), (2, [0.0; 3])], start, end, variant, fade: 10.0 }
    }

    #[test]
    fn records_round_trip_and_events_read_back() {
        let t = trail(7, 17, 0).write(&[0u8; EVENT]).unwrap();
        let p = Point { dpx: "ex_0010.dpx".into(), id: 0, bone: 37, offset: [0.0; 3], start: 0, end: -1, rot: [0.0, 0.0, 40.0], scale: [1.0; 3], flags: 0xA, fade: 0.0, variant: 0 }.write().unwrap();
        let recs = vec![Record { name: "w001_055".into(), a: vec![], b: vec![t], raw_name: None }, Record { name: "p001_101".into(), a: vec![p], b: vec![], raw_name: None }, Record { name: "w001_055".into(), a: vec![], b: vec![trail(5, 25, 1301).write(&t).unwrap()], raw_name: None }];
        let d = write(&recs).unwrap();
        assert_eq!(&d[..4], b"@EET");
        let back = parse(&d).unwrap();
        assert_eq!(back.iter().map(|r| (&r.name, &r.a, &r.b)).collect::<Vec<_>>(), recs.iter().map(|r| (&r.name, &r.a, &r.b)).collect::<Vec<_>>());
        assert_eq!(write(&back).unwrap(), d);
        assert_eq!(Trail::read(&back[2].b[0]), trail(5, 25, 1301));
        assert_eq!(i32::from_le_bytes(back[1].a[0][0x38..0x3C].try_into().unwrap()), -1);
    }

    #[test]
    fn recoms_own_files_rebuild_byte_for_byte() {
        // Re:CoM's PL0001.EET / PL0002.EET lay out header, records, then each record's events in order
        // (Checked (files), when `KH1_RECOM_DATA` names a folder holding them).
        let Some(dir) = std::env::var_os("KH1_RECOM_DATA") else { return };
        for f in ["PL0001.EET", "PL0002.EET"] {
            let d = std::fs::read(std::path::Path::new(&dir).join(f)).unwrap();
            assert_eq!(write(&parse(&d).unwrap()).unwrap(), d, "{f}");
        }
    }
}
