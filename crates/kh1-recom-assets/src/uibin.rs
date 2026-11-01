//! KH1's HUD layouts, `exchange/<lang>_uibin_*.bin` (and `<lang>_com_battle.bin`, `danger.bin`,
//! …): sprite objects KH1's 2D code draws 1 texel = 1 unit (`docs/recom-kh1-hud.md`).
//!
//! Read as KH1's own code reads them (Steam decomp; Checked (decomp) unless noted):
//! - header (`0x14027D9D0`): `+0x00` four bytes (`"TTUI"` in every file seen, Checked (files)),
//!   `+0x04` one int (2), `+0x08..+0x27` four (offset, size) pairs: objects (0xC0 each), keys A
//!   (0x20), keys B (0x50), keys C (0x20).
//! - object (0xC0): `+0x00` x, y, w, h (`0x14027EA50`: the quad (x, y)–(x + w, y + h)); `+0x10`
//!   u0, v0, u1, v1, inclusive texels, u1 < u0 (v1 < v0) drawn mirrored (`0x14027E7D0`); `+0x20`
//!   RGBA (0x80 = ×1), or with flag 0x20 four vertex colours `+0x20..+0x5F`; `+0x60` flags (0x02
//!   colour keys A, 0x40 vertex-colour keys B, 0x04 position keys C; 0x08 / 0x80 / 0x10 running);
//!   key players `+0x64` A, `+0x78` B, `+0x8C` C (time f32, key index, count, first key, status);
//!   `+0xA0` name (32 bytes).
//! - key: `+0x00` duration (players step 0.5 a 60 Hz frame: 2 frames a unit), `+0x10` 4 floats
//!   (A: r, g, b, a; C: x, y, w, h) or 16 floats (B: four RGBA).

use crate::{bad, Result};

/// One object's size.
pub const OBJ: usize = 0xC0;

/// A sprite object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Obj {
    /// x, y, w, h (KH1 2D units, 512 × 384 screen).
    pub rect: [i32; 4],
    /// u0, v0, u1, v1, inclusive (mirrored when u1 < u0 / v1 < v0).
    pub uv: [i32; 4],
    /// The colour (RGBA, 0x80 = ×1); with flag 0x20 the four vertex colours.
    pub colours: [[i32; 4]; 4],
    /// Flags (`+0x60`).
    pub flags: u32,
    /// Key player A (colour): first key and count.
    pub keys_a: (i32, i32),
    /// The name.
    pub name: String,
}

/// A key: its duration and values.
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    /// Duration in key units (2 frames at 60 Hz each).
    pub dur: i32,
    /// The values (4 or 16).
    pub vals: Vec<f32>,
}

/// A layout file.
#[derive(Clone, Debug, PartialEq)]
pub struct Uibin {
    /// The objects.
    pub objs: Vec<Obj>,
    /// Colour keys (A).
    pub keys_a: Vec<Key>,
}

fn i32_at(d: &[u8], at: usize) -> Result<i32> {
    d.get(at..at + 4).map(|b| i32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| crate::Error::Format(format!("uibin: read past the end at {at:#x}")))
}

impl Uibin {
    /// Reads a layout.
    pub fn parse(d: &[u8]) -> Result<Uibin> {
        if d.len() < 0x28 {
            return bad("uibin: shorter than its header");
        }
        let sec = |k: usize| -> Result<(usize, usize)> {
            let (o, z) = (i32_at(d, 8 + 8 * k)?, i32_at(d, 12 + 8 * k)?);
            if o < 0 || z < 0 || o as usize > d.len() || z as usize > d.len() - o as usize {
                return bad(format!("uibin: section {k} ({o:#x}, {z:#x}) outside the file"));
            }
            Ok((o as usize, z as usize))
        };
        let (oo, oz) = sec(0)?;
        let mut objs = Vec::with_capacity(oz / OBJ);
        for i in 0..oz / OBJ {
            let b = &d[oo + i * OBJ..oo + (i + 1) * OBJ];
            let w = |k: usize| i32::from_le_bytes(b[4 * k..4 * k + 4].try_into().unwrap());
            let flags = w(24) as u32;
            let c = |k: usize| [w(k), w(k + 1), w(k + 2), w(k + 3)];
            let colours = if flags & 0x20 != 0 { [c(8), c(12), c(16), c(20)] } else { [c(8); 4] };
            let n = &b[0xA0..0xC0];
            let end = n.iter().position(|&x| x == 0).unwrap_or(n.len());
            objs.push(Obj {
                rect: [w(0), w(1), w(2), w(3)],
                uv: [w(4), w(5), w(6), w(7)],
                colours,
                flags,
                keys_a: (w(28), w(27)),
                name: n[..end].iter().map(|&x| if (0x20..0x7F).contains(&x) { x as char } else { '?' }).collect(),
            });
        }
        let (ao, az) = sec(1)?;
        let keys_a = (0..az / 0x20)
            .map(|i| {
                let at = ao + i * 0x20;
                Ok(Key { dur: i32_at(d, at)?, vals: (0..4).map(|j| i32_at(d, at + 0x10 + 4 * j).map(|v| f32::from_bits(v as u32))).collect::<Result<_>>()? })
            })
            .collect::<Result<_>>()?;
        Ok(Uibin { objs, keys_a })
    }

    /// The object named `name`.
    pub fn find(&self, name: &str) -> Option<&Obj> {
        self.objs.iter().find(|o| o.name == name)
    }
}

impl Obj {
    /// Its texel rect as Re:CoM wants it (u0, v0, u1, v1 with an exclusive end, not mirrored)
    /// and whether it is drawn mirrored in u and v.
    pub fn texels(&self) -> ([i32; 4], bool, bool) {
        let [u0, v0, u1, v1] = self.uv;
        ([u0.min(u1), v0.min(v1), u0.max(u1) + 1, v0.max(v1) + 1], u1 < u0, v1 < v0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(objs: &[([i32; 8], u32, &str)], keys: &[(i32, [f32; 4])]) -> Vec<u8> {
        let mut o = Vec::new();
        for (r, flags, name) in objs {
            let mut b = vec![0u8; OBJ];
            for (k, v) in r.iter().enumerate() {
                b[4 * k..4 * k + 4].copy_from_slice(&v.to_le_bytes());
            }
            for k in 8..12 {
                b[4 * k..4 * k + 4].copy_from_slice(&128i32.to_le_bytes());
            }
            b[0x60..0x64].copy_from_slice(&flags.to_le_bytes());
            b[0xA0..0xA0 + name.len()].copy_from_slice(name.as_bytes());
            o.extend(b);
        }
        let mut k = Vec::new();
        for (dur, v) in keys {
            k.extend(dur.to_le_bytes());
            k.extend([0u8; 12]);
            for x in v {
                k.extend(x.to_le_bytes());
            }
        }
        let mut d = b"TTUI".to_vec();
        d.extend(2i32.to_le_bytes());
        let ko = 0x28 + o.len();
        for (off, size) in [(0x28, o.len()), (ko, k.len()), (ko + k.len(), 0), (ko + k.len(), 0)] {
            d.extend((off as i32).to_le_bytes());
            d.extend((size as i32).to_le_bytes());
        }
        d.extend(o);
        d.extend(k);
        d
    }

    #[test]
    fn reads_objects_and_keys() {
        let d = file(&[([407, 280, 37, 36, 124, 89, 89, 124], 0, "g_s_fra_a_wb"), ([1, 2, 3, 4, 0, 0, 2, 3], 2, "x")], &[(8, [128.0, 0.0, 0.0, 0.0])]);
        let u = Uibin::parse(&d).unwrap();
        assert_eq!(u.objs.len(), 2);
        let o = u.find("g_s_fra_a_wb").unwrap();
        assert_eq!(o.rect, [407, 280, 37, 36]);
        assert_eq!(o.texels(), ([89, 89, 125, 125], true, false));
        assert_eq!(u.keys_a[0].dur, 8);
        assert_eq!(u.keys_a[0].vals[0], 128.0);
        assert!(Uibin::parse(&d[..0x20]).is_err());
    }
}
