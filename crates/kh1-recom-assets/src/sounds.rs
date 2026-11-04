//! M13 stage 2: KH1's own sound banks for Re:CoM (owner, 2026-11-04: "lets add all of soras effects
//! for real this time"; `docs/recom-m13-plan.md` § Stage 2).
//!
//! KH1 (Steam; Checked (decomp)):
//! - Every KH1 sound id of 1000 and up is **its own SCD**, a PC asset named `se0<id>.win32.scd`
//!   (Sora's model sounds are `xa_ex_0010.mdls`'s assets `se011001.win32.scd` …). KH1 registers a
//!   file's SCD assets (`0x1400E0A50` / `0x1400DB910` → `0x1400E0F00`); the bank entry's number is
//!   `atoi(name + 2)` (`0x1400DB1F0`), and a sound of that id is made from **entry 0** of the
//!   entry with that number (`0x1400DE460`, the branch for banks other than 2:
//!   `CreateSound(handle, 0, …)`). Checked (files): each of Sora's 21 SCDs holds one sound.
//! - Ids under 1000 are entries of the system bank 2 (`se000.win32.scd`, entry = id), which the
//!   menu sounds already give Re:CoM as bank 9000 (`menu_sounds`).
//! - Sora's swing, finisher, special, Guard and Counterattack sounds are **weapon sounds**: motion
//!   trigger op 0x19 = the weapon row's `+0x34` + arg (`0x140295420`), and the row's hit sounds
//!   the same base + row `+0xC..+0xE`. The Kingdom Key's base is `0x2B10` (btltbl, item 0x51);
//!   each Keyblade owns 40 ids. Checked (decomp + files). They are in no `.mdls`, `.wpn` or `.mag`
//!   (Checked (files)); the world packs `amusic/se<ww>.dat` (KH1 loads its world's at room set-up,
//!   `0x140178E80`) carry 30–120 MB of PC assets each (Checked (files: the hed sizes)), so the step
//!   looks for them there. That they are there is an **Assumption** until the owner's run says so.
//!
//! Re:CoM (Steam; Checked (decomp)): a sound id is `group × 1000 + n`; the bank is the id → bank
//! map's (`0x1402A8560`, which the DLL extends with groups 9000..9999, each its own bank) and the
//! sound is `CreateSound(first bank entry with that bank, n)` (`0x1402F3080`). So each KH1 SCD
//! becomes **one Re:CoM bank** (9001 …) played as `bank × 1000` (entry 0), registered by
//! Re:CoM's own `0x1402F5D70` as the menu sounds' bank 9000 is. The step writes the SCDs unchanged
//! into `kh1-sounds.bin` and the id map into `kh1-sounds.json`; nothing is decoded or rebuilt.

use crate::menu_sounds::{kh1_number, scd_info};
use crate::{Error, Result};
use kh1_formats::egs::HdAsset;

/// The manifest the step writes (next to the other manifests).
pub const MANIFEST: &str = "kh1-sounds.json";
/// The SCDs, back to back (next to the manifest).
pub const PACK: &str = "kh1-sounds.bin";
/// The manifest's `format`.
pub const FORMAT: &str = "kh1-recom-assets sounds 1";
/// KH1's system bank (ids under 1000) as the menu sounds give it to Re:CoM.
pub const SYSTEM_GROUP: u32 = crate::menu_sounds::GROUP;
/// The first Re:CoM bank for a KH1 SCD; one past the last (the DLL's map range).
pub const FIRST_BANK: u32 = 9001;
/// One past the last bank the DLL's map gives its own bank.
pub const BANK_END: u32 = 10_000;
/// Alignment of each SCD in the pack.
pub const ALIGN: usize = 16;
/// Sora's KH1 model file (his own sounds are its SCD assets).
pub const SORA_MDLS: &str = "xa_ex_0010.mdls";
/// KH1's world codes for `amusic/se<ww>.dat` (`0x1404DCCA0`, Checked (decomp): the table the
/// world pack loader `0x140178E80` reads, with its two `xx` slots left out), then the world map's
/// pack and `sezz` (both in the player's hed: Checked (files)).
pub const WORLD_PACKS: [&str; 17] = [
    "amusic/sedh.dat",
    "amusic/sedi.dat",
    "amusic/sedc.dat",
    "amusic/setw.dat",
    "amusic/seaw.dat",
    "amusic/setz.dat",
    "amusic/sepo.dat",
    "amusic/seal.dat",
    "amusic/selm.dat",
    "amusic/senm.dat",
    "amusic/sehe.dat",
    "amusic/sepi.dat",
    "amusic/sepp.dat",
    "amusic/sepc.dat",
    "amusic/seew.dat",
    "amusic/seworldmap.dat",
    "amusic/sezz.dat",
];
/// Ids per Keyblade (weapon `+0x34` + 0..40: the swing args 7..0x22 and the hit bytes; Checked
/// (files): the largest arg in Sora's mset is 0x22).
pub const WEAPON_IDS: u32 = 40;
/// The Kingdom Key's item id (its weapon row's `+0x34` is the base the step takes).
pub const KINGDOM_KEY_ITEM: u16 = 0x51;

/// KH1's id for a PC asset that is one sound's SCD (`se0<id>.win32.scd`, numbered ≥ 1000 by
/// KH1's own rule), or None (textures, meshes, the system bank `se000`).
pub fn sound_id(name: &str) -> Option<u32> {
    let n = kh1_number(name)?;
    (1000..=0xFFFF).contains(&n).then_some(n as u32)
}

/// One KH1 file searched, with its PC assets.
#[derive(Clone, Debug)]
pub struct Source {
    /// The file's name in KH1's archives.
    pub file: String,
    /// Its PC assets.
    pub assets: Vec<HdAsset>,
}

/// One KH1 sound taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sound {
    /// KH1's id.
    pub kh1_id: u32,
    /// Re:CoM's bank (its id is `bank × 1000`).
    pub bank: u32,
    /// KH1's asset name (kept as the bank entry's name).
    pub name: String,
    /// Where it was found.
    pub source: String,
    /// The SCD, unchanged.
    pub data: Vec<u8>,
}

impl Sound {
    /// Re:CoM's id for it.
    pub fn recom_id(&self) -> u32 {
        self.bank * 1000
    }
}

/// What the step took.
#[derive(Clone, Debug, Default)]
pub struct Taken {
    /// In KH1 id order, banks 9001 … in that order.
    pub sounds: Vec<Sound>,
    /// Wanted ids found nowhere.
    pub missing: Vec<u32>,
    /// Notes (an id whose copies differ between files: the first is kept).
    pub notes: Vec<String>,
}

/// The ids the step takes: every SCD of Sora's own model file (`sora`) and the Kingdom Key's
/// [`WEAPON_IDS`] from `weapon_base`, sorted and unique.
pub fn wanted(sora: &[HdAsset], weapon_base: u32) -> Vec<u32> {
    let mut v: Vec<u32> = sora.iter().filter_map(|a| sound_id(&a.name)).collect();
    v.extend(weapon_base..weapon_base + WEAPON_IDS);
    v.sort_unstable();
    v.dedup();
    v
}

/// Takes each wanted id from the first source that has it (Sora's file first, then the world
/// packs in KH1's order), checking each SCD the way both drivers read it. Copies of the same id in
/// later files are compared, and a difference is noted (the first is kept).
pub fn take(sources: &[Source], wanted: &[u32]) -> Result<Taken> {
    let mut t = Taken::default();
    let mut found: Vec<(u32, &HdAsset, &str)> = Vec::new();
    for src in sources {
        for a in &src.assets {
            let Some(id) = sound_id(&a.name) else { continue };
            if !wanted.contains(&id) {
                continue;
            }
            match found.iter().find(|f| f.0 == id) {
                Some(f) => {
                    if f.1.data != a.data {
                        t.notes.push(format!("KH1 sound {id}: {} in {} differs from {}'s (the first is kept)", a.name, src.file, f.2));
                    }
                }
                None => {
                    scd_info(&a.data).map_err(|e| Error::Format(format!("{} {}: {e}", src.file, a.name)))?;
                    if a.name.len() > crate::menu_sounds::NAME_MAX || a.name.contains('\0') {
                        return Err(Error::Format(format!("{} {}: name too long for Re:CoM's bank entry", src.file, a.name)));
                    }
                    found.push((id, a, &src.file));
                }
            }
        }
    }
    found.sort_by_key(|f| f.0);
    if found.len() as u32 > BANK_END - FIRST_BANK {
        return Err(Error::Format(format!("{} KH1 sounds: more than the {} banks 9001..9999", found.len(), BANK_END - FIRST_BANK)));
    }
    for (k, (id, a, file)) in found.iter().enumerate() {
        t.sounds.push(Sound { kh1_id: *id, bank: FIRST_BANK + k as u32, name: a.name.clone(), source: file.to_string(), data: a.data.clone() });
    }
    t.missing = wanted.iter().copied().filter(|id| !found.iter().any(|f| f.0 == *id)).collect();
    Ok(t)
}

/// The Re:CoM id that plays KH1 sound `kh1_id`: a system-bank entry (under 1000) as bank 9000's
/// entry, an SCD the step took as its bank's entry 0; None when it was not found.
pub fn recom_id(t: &Taken, kh1_id: u32) -> Option<u32> {
    if kh1_id < 1000 {
        return Some(SYSTEM_GROUP * 1000 + kh1_id);
    }
    t.sounds.iter().find(|s| s.kh1_id == kh1_id).map(Sound::recom_id)
}

/// The pack (the SCDs back to back, each at a multiple of [`ALIGN`]) and its manifest.
pub fn build(t: &Taken, weapon_base: u32, sources: &[(&str, String)]) -> (Vec<u8>, String) {
    use std::fmt::Write;
    let mut pack = Vec::new();
    let mut rows = Vec::new();
    for s in &t.sounds {
        while pack.len() % ALIGN != 0 {
            pack.push(0);
        }
        rows.push(format!(
            "    {{\"kh1_id\": {}, \"bank\": {}, \"recom_id\": {}, \"name\": {:?}, \"offset\": {}, \"size\": {}, \"source\": {:?}}}",
            s.kh1_id,
            s.bank,
            s.recom_id(),
            s.name,
            pack.len(),
            s.data.len(),
            s.source
        ));
        pack.extend_from_slice(&s.data);
    }
    let mut j = String::new();
    let _ = writeln!(j, "{{");
    let _ = writeln!(j, "  \"format\": \"{FORMAT}\",");
    let _ = writeln!(j, "  \"pack\": \"{PACK}\",");
    let _ = writeln!(j, "  \"pack_size\": {},", pack.len());
    let _ = writeln!(j, "  \"system_group\": {SYSTEM_GROUP},");
    let _ = writeln!(j, "  \"weapon_base\": {weapon_base},");
    let _ = writeln!(j, "  \"sounds\": [\n{}\n  ],", rows.join(",\n"));
    let _ = writeln!(j, "  \"missing\": {:?},", t.missing);
    let src: Vec<String> = sources.iter().map(|(k, v)| format!("{k:?}: {v:?}")).collect();
    let _ = writeln!(j, "  \"sources\": {{{}}}", src.join(", "));
    let _ = writeln!(j, "}}");
    (pack, j)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A one-sound SCD the drivers' header check accepts (SEDBSSCF v3, one table entry, one
    /// stream of `n` bytes). Test data only.
    pub fn scd(n: usize, fill: u8) -> Vec<u8> {
        let mut d = vec![0u8; 0x80 + n];
        d[..8].copy_from_slice(b"SEDBSSCF");
        d[8] = 3;
        d[0xE] = 0x30;
        let len = d.len() as u32;
        d[0x10..0x14].copy_from_slice(&len.to_le_bytes());
        // tables at 0x30: 1 sound, 1 track, 1 stream; stream offsets at 0x40 -> stream at 0x50.
        d[0x30] = 1;
        d[0x32] = 1;
        d[0x34] = 1;
        d[0x3C..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        d[0x40..0x44].copy_from_slice(&0x50u32.to_le_bytes());
        d[0x50..0x54].copy_from_slice(&(n as u32).to_le_bytes());
        d[0x5C..0x60].copy_from_slice(&0xCu32.to_le_bytes());
        for b in &mut d[0x70..] {
            *b = fill;
        }
        d
    }

    fn asset(name: &str, data: Vec<u8>) -> HdAsset {
        HdAsset { name: name.into(), orig_offset: -1, data }
    }

    #[test]
    fn ids_follow_kh1s_atoi_rule() {
        assert_eq!(sound_id("se011001.win32.scd"), Some(11001));
        assert_eq!(sound_id("se014430.win32.scd"), Some(14430));
        assert_eq!(sound_id("se000.win32.scd"), None, "the system bank is bank 9000's");
        assert_eq!(sound_id("8fb40.dds"), None);
        assert_eq!(sound_id("se011001.ps3.scd"), None);
        assert_eq!(sound_id("music132.win32.scd"), None);
    }

    #[test]
    fn wanted_is_soras_own_and_the_kingdom_keys_forty() {
        let sora = [asset("se011001.win32.scd", scd(4, 1)), asset("se011023.win32.scd", scd(4, 2)), asset("100.dds", vec![1])];
        let w = wanted(&sora, 0x2B10);
        assert_eq!(w.len(), 42);
        assert_eq!((w[0], w[1], w[2]), (11001, 11023, 11024));
        assert_eq!(*w.last().unwrap(), 0x2B10 + 39);
    }

    #[test]
    fn first_file_wins_banks_in_id_order_and_differences_are_noted() {
        let sora = Source { file: SORA_MDLS.into(), assets: vec![asset("se011005.win32.scd", scd(8, 5)), asset("se011001.win32.scd", scd(8, 1))] };
        let tw = Source { file: "amusic/setw.dat".into(), assets: vec![asset("se011031.win32.scd", scd(8, 31)), asset("se011001.win32.scd", scd(8, 9)), asset("se099999.win32.scd", scd(8, 0))] };
        let t = take(&[sora, tw], &[11001, 11005, 11031, 11032]).unwrap();
        let got: Vec<(u32, u32, &str)> = t.sounds.iter().map(|s| (s.kh1_id, s.bank, s.source.as_str())).collect();
        assert_eq!(got, vec![(11001, 9001, SORA_MDLS), (11005, 9002, SORA_MDLS), (11031, 9003, "amusic/setw.dat")]);
        assert_eq!(t.missing, vec![11032]);
        assert_eq!(t.notes.len(), 1, "setw's 11001 differs from Sora's");
        assert_eq!(recom_id(&t, 11031), Some(9_003_000));
        assert_eq!(recom_id(&t, 81), Some(9_000_081), "effect sounds: the system bank's entries");
        assert_eq!(recom_id(&t, 11032), None);
    }

    #[test]
    fn a_bad_scd_is_refused() {
        let s = Source { file: "x".into(), assets: vec![asset("se011001.win32.scd", b"not an scd".to_vec())] };
        assert!(take(&[s], &[11001]).is_err());
    }

    #[test]
    fn the_pack_holds_each_scd_aligned_as_the_manifest_says() {
        let s = Source { file: SORA_MDLS.into(), assets: vec![asset("se011001.win32.scd", scd(5, 1)), asset("se011002.win32.scd", scd(7, 2))] };
        let t = take(&[s], &[11001, 11002]).unwrap();
        let (pack, j) = build(&t, 0x2B10, &[("x sha256", "ab".into())]);
        let a = &t.sounds[0].data;
        let b = &t.sounds[1].data;
        let off_b = a.len().div_ceil(ALIGN) * ALIGN;
        assert_eq!(&pack[..a.len()], a.as_slice());
        assert_eq!(&pack[off_b..off_b + b.len()], b.as_slice());
        assert!(j.contains(&format!("\"offset\": {off_b}, \"size\": {}", b.len())));
        assert!(j.contains("\"format\": \"kh1-recom-assets sounds 1\""));
        assert!(j.contains("\"bank\": 9002, \"recom_id\": 9002000"));
    }
}
