//! KH1's command menu sounds (owner, 2026-11-01: "play KH1's exact menu sounds"): the asset step
//! takes KH1's own sound files from the player's KH1 archives, unchanged, into
//! `kh1-menu-sounds.bin` + `kh1-menu-sounds.json`; the DLL (`crates/kh1-recom`
//! `win/menu_sounds.rs`) gives each one to Re:CoM's own sound-bank registration as a new bank and
//! plays it with Re:CoM's own play function. Notes: `docs/kh1-recom-assets.md` § KH1's menu
//! sounds, `docs/recom-m7-command-menu.md` § Sounds.
//!
//! KH1 (Steam; Checked (decomp + exe)):
//! - The menu plays `0x14017A040(id)` = `SslSeCreate(2, id)` (`0x1400D2FD0` → `0x1400DE460`) then
//!   `SslSePlay(2, id, 0, 0x3FFF, 0x3FFF)` (`0x1400D3060` → `0x1400DECF0`). The two 0x3FFF words
//!   are turned into floats (/ 16383) but `SslSePlay` reads only bank / no / id: the sound plays
//!   at its file's own volume.
//! - `SslSeCreate` with a bank other than the map bank looks through every loaded bank entry for
//!   the one whose number (`+0x14`) is `id`, and makes sound 0 of its SCD.
//! - A bank entry is made for every PC asset of a loaded file whose name holds `.win32.scd` and
//!   `se` (or `music`) (`0x1400E0A50` → `0x1400E0F00`); its number is `atoi(name + 2)`
//!   (`0x1400DB1F0`). So KH1's sound `n` is the asset named `se<n, 6 digits>.win32.scd`
//!   (`xa_ex_0010.mdls`'s are `se011001.win32.scd` …; Checked (files)).
//! - `amusic/se000.dat` is loaded at start (`0x1401791D0`); that the menu sounds are its assets
//!   is an Assumption (strong) the step checks: it names any id it can't find.
//!
//! The files are SQEX SCDs (`SEDBSSCF`), read by the same driver in both exes (`ScdHeader::
//! GetScdHeaderSize` takes versions 1–4: KH1 `0x1400748F0`, Re:CoM `0x140073130`; Checked
//! (decomp)).

use crate::{Error, Result};
use kh1_formats::egs::HdAsset;

/// The KH1 file holding KH1's system sounds (Assumption, strong: checked per id at build time).
pub const KH1_FILE: &str = "amusic/se000.dat";
/// The manifest the step writes (next to the other manifests).
pub const MANIFEST: &str = "kh1-menu-sounds.json";
/// The SCDs, back to back (next to the manifest).
pub const PACK: &str = "kh1-menu-sounds.bin";
/// The manifest's `format`.
pub const FORMAT: &str = "kh1-recom-assets menu-sounds 1";
/// Re:CoM's sound ids for these banks: KH1 sound `n` is played as id `(GROUP_BASE + n) × 1000`
/// (bank `GROUP_BASE + n`, sound 0). The DLL maps that group range to a bank of its own (Re:CoM's
/// map `0x1402A8560` gives no bank for groups ≥ 8000).
pub const GROUP_BASE: u32 = 9000;

/// KH1's command menu sounds (`0x14017A040` ids, Checked (decomp): `0x1402931B0`,
/// `0x1402937F0`, `0x140293480`, `0x1402948B0`): what each is for.
pub const SOUNDS: [(u32, &str); 4] = [(1, "move"), (3, "back"), (4, "error"), (6, "open")];

/// Longest name Re:CoM's bank entry keeps (`+0x30..+0x50`, `0x1402F0080`).
pub const NAME_MAX: usize = 31;

/// KH1's bank number for a PC asset name (`0x1400E0A50`'s filter, then `0x1400DB1F0`'s
/// `atoi(name + 2)`), or None when KH1 makes no bank of it.
/// (`"title"`, `music…` and `bgm_…` entries get −1 there: no sound number.)
pub fn kh1_number(name: &str) -> Option<i64> {
    if !name.contains(".win32.scd") || !name.contains("se") || name == "title" || name.contains("music") || name.contains("bgm_") {
        return None;
    }
    Some(atoi(name.as_bytes().get(2..).unwrap_or(&[])))
}

/// C's `atoi`: leading white space, a sign, then digits.
fn atoi(b: &[u8]) -> i64 {
    let mut i = 0;
    while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) {
        i += 1;
    }
    let neg = match b.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    let mut v: i64 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = (v * 10 + (b[i] - b'0') as i64).min(i64::from(i32::MAX) + 1);
        i += 1;
    }
    if neg {
        -v
    } else {
        v
    }
}

/// What the step reads from an SCD (for the report and the checks).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScdInfo {
    /// Header `+8`.
    pub version: u32,
    /// Sound table entries (what a play index picks).
    pub sounds: u16,
    /// Stream headers.
    pub streams: u16,
    /// Stream 0's channels.
    pub channels: i32,
    /// Stream 0's sample rate.
    pub rate: i32,
    /// Stream 0's codec (0xC = MS ADPCM, 6 = Vorbis).
    pub codec: i32,
    /// Stream 0's data bytes.
    pub data_size: i32,
}

/// Checks an SCD the way both drivers read its header (little-endian `SEDBSSCF`, version 1–4,
/// the file-size word, the tables): at least one sound and one stream.
pub fn scd_info(d: &[u8]) -> Result<ScdInfo> {
    let u16_at = |o: usize| d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| Error::Format(format!("SCD too short at {o:#x}")));
    let u32_at = |o: usize| d.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| Error::Format(format!("SCD too short at {o:#x}")));
    if d.get(..8) != Some(b"SEDBSSCF".as_slice()) {
        return Err(Error::Format("not an SCD (no SEDBSSCF)".into()));
    }
    let version = u32_at(8)?;
    if !(1..=4).contains(&version) {
        return Err(Error::Format(format!("SCD version {version} (the driver takes 1..4)")));
    }
    if d[0xC] != 0 {
        return Err(Error::Format("big-endian SCD".into()));
    }
    let size = u32_at(0x10)? as usize;
    if size != d.len() {
        return Err(Error::Format(format!("SCD says {size} bytes, has {}", d.len())));
    }
    let t = u16_at(0xE)? as usize;
    let (sounds, streams) = (u16_at(t)?, u16_at(t + 4)?);
    if sounds == 0 || streams == 0 {
        return Err(Error::Format(format!("SCD has {sounds} sounds, {streams} streams")));
    }
    let table = u32_at(t + 0xC)? as usize;
    let s = u32_at(table)? as usize;
    let i = |o: usize| u32_at(s + o).map(|v| v as i32);
    let (data_size, channels, rate, codec) = (i(0)?, i(4)?, i(8)?, i(0xC)?);
    if data_size < 0 || s + 0x20 + data_size as usize > d.len() {
        return Err(Error::Format(format!("SCD stream 0 ({data_size} bytes at {s:#x}) runs past the file")));
    }
    Ok(ScdInfo { version, sounds, streams, channels, rate, codec, data_size })
}

/// One sound taken.
#[derive(Clone, Debug)]
pub struct Taken {
    /// KH1's sound id (`0x14017A040`).
    pub kh1_id: u32,
    /// What the menu plays it for.
    pub what: &'static str,
    /// KH1's asset name (kept as Re:CoM's bank entry name).
    pub name: String,
    /// The SCD, unchanged.
    pub data: Vec<u8>,
    /// Its header.
    pub info: ScdInfo,
}

/// KH1's asset for sound `id` among a file's PC assets: the first whose KH1 bank number is `id`
/// (KH1 takes the newest bank entry with that number; one file holds one of each).
pub fn find(assets: &[HdAsset], id: u32) -> Option<&HdAsset> {
    assets.iter().find(|a| kh1_number(&a.name) == Some(id as i64))
}

/// Takes the four menu sounds from `assets` (KH1's `amusic/se000.dat`). Each missing or bad one
/// is a note; the result has the ones found.
pub fn take(assets: &[HdAsset], notes: &mut Vec<String>) -> Vec<Taken> {
    let mut out = Vec::new();
    for (id, what) in SOUNDS {
        let Some(a) = find(assets, id) else {
            notes.push(format!("{KH1_FILE}: no asset for KH1 sound {id} ({what}); Re:CoM's sound stays for it"));
            continue;
        };
        if a.name.len() > NAME_MAX || a.name.contains('\0') {
            notes.push(format!("{KH1_FILE}: asset name {:?} too long for Re:CoM's bank entry; sound {id} ({what}) not taken", a.name));
            continue;
        }
        match scd_info(&a.data) {
            Ok(info) => out.push(Taken { kh1_id: id, what, name: a.name.clone(), data: a.data.clone(), info }),
            Err(e) => notes.push(format!("{KH1_FILE}: {} (sound {id}, {what}): {e}; not taken", a.name)),
        }
    }
    out
}

/// The pack (`kh1-menu-sounds.bin`: the SCDs back to back, each at a 16-byte boundary) and its
/// manifest.
pub fn build(taken: &[Taken], sources: &[(&str, String)]) -> (Vec<u8>, String) {
    use std::fmt::Write;
    let mut bin = Vec::new();
    let mut rows = Vec::new();
    for t in taken {
        while bin.len() % 16 != 0 {
            bin.push(0);
        }
        let at = bin.len();
        bin.extend_from_slice(&t.data);
        rows.push(format!(
            "    {{\"kh1_id\": {}, \"use\": {:?}, \"name\": {:?}, \"group\": {}, \"offset\": {at}, \"size\": {}, \"sha256\": {:?}, \"channels\": {}, \"rate\": {}, \"codec\": {}}}",
            t.kh1_id,
            t.what,
            t.name,
            GROUP_BASE + t.kh1_id,
            t.data.len(),
            kh1_formats::egs::sha256_hex(&t.data),
            t.info.channels,
            t.info.rate,
            t.info.codec
        ));
    }
    let mut s = String::new();
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"format\": \"{FORMAT}\",");
    let _ = writeln!(s, "  \"pack\": \"{PACK}\",");
    let _ = writeln!(s, "  \"pack_size\": {},", bin.len());
    let _ = writeln!(s, "  \"group_base\": {GROUP_BASE},");
    let _ = writeln!(s, "  \"source\": \"{KH1_FILE}\",");
    let _ = writeln!(s, "  \"sounds\": [\n{}\n  ],", rows.join(",\n"));
    let src: Vec<String> = sources.iter().map(|(k, v)| format!("{k:?}: {v:?}")).collect();
    let _ = writeln!(s, "  \"sources\": {{{}}}", src.join(", "));
    let _ = writeln!(s, "}}");
    (bin, s)
}

/// The step's line for one sound.
pub fn line(t: &Taken) -> String {
    let codec = match t.info.codec {
        0xC => "MS ADPCM".to_string(),
        6 => "Vorbis".to_string(),
        c => format!("codec {c:#x}"),
    };
    format!(
        "KH1 menu sound {} ({}): {} ({} bytes, {} sound(s), {} ch, {} Hz, {codec}) → Re:CoM id {}",
        t.kh1_id,
        t.what,
        t.name,
        t.data.len(),
        t.info.sounds,
        t.info.channels,
        t.info.rate,
        (GROUP_BASE + t.kh1_id) * 1000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-sound SCD laid out as KH1's per-sound files are (`se011001.win32.scd`: tables at
    /// 0x30, stream header at 0x70 → 0x80, MS ADPCM extradata, then the data).
    pub(crate) fn scd(payload: &[u8]) -> Vec<u8> {
        let mut d = vec![0u8; 0x80];
        d[..8].copy_from_slice(b"SEDBSSCF");
        d[8..12].copy_from_slice(&3u32.to_le_bytes());
        d[0xD] = 4;
        d[0xE..0x10].copy_from_slice(&0x30u16.to_le_bytes());
        d[0x30..0x32].copy_from_slice(&1u16.to_le_bytes());
        d[0x32..0x34].copy_from_slice(&1u16.to_le_bytes());
        d[0x34..0x36].copy_from_slice(&1u16.to_le_bytes());
        d[0x38..0x3C].copy_from_slice(&0x60u32.to_le_bytes());
        d[0x3C..0x40].copy_from_slice(&0x70u32.to_le_bytes());
        d[0x60..0x64].copy_from_slice(&0x80u32.to_le_bytes());
        d[0x70..0x74].copy_from_slice(&0x80u32.to_le_bytes());
        let mut h = Vec::new();
        for v in [payload.len() as i32, 1, 48000, 0xC, 0, 0, 0, 0] {
            h.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&h);
        d.extend_from_slice(payload);
        let n = d.len() as u32;
        d[0x10..0x14].copy_from_slice(&n.to_le_bytes());
        d
    }

    fn asset(name: &str, data: Vec<u8>) -> HdAsset {
        HdAsset { name: name.into(), orig_offset: -1, data }
    }

    #[test]
    fn kh1_numbers_follow_kh1s_atoi_from_the_third_character() {
        assert_eq!(kh1_number("se000001.win32.scd"), Some(1));
        assert_eq!(kh1_number("se000006.win32.scd"), Some(6));
        assert_eq!(kh1_number("se011001.win32.scd"), Some(11001));
        // No SCD, or music: no bank number.
        assert_eq!(kh1_number("se000001.wd"), None);
        assert_eq!(kh1_number("music132.win32.scd"), None);
        assert_eq!(kh1_number("-c0.cvbl"), None);
        // atoi stops at the first non-digit, from the third character whatever it is.
        assert_eq!(kh1_number("xse4.win32.scd"), Some(0));
        assert_eq!(kh1_number("se4.win32.scd"), Some(4));
    }

    #[test]
    fn scd_checks_take_kh1s_layout_and_refuse_the_rest() {
        let d = scd(&[1, 2, 3, 4]);
        let i = scd_info(&d).unwrap();
        assert_eq!((i.version, i.sounds, i.streams, i.channels, i.rate, i.codec, i.data_size), (3, 1, 1, 1, 48000, 0xC, 4));
        let mut bad = d.clone();
        bad[0] = b'X';
        assert!(scd_info(&bad).is_err());
        let mut v9 = d.clone();
        v9[8] = 9;
        assert!(scd_info(&v9).is_err());
        let mut short = d.clone();
        short.pop();
        assert!(scd_info(&short).is_err(), "the file-size word must match");
        let mut none = d.clone();
        none[0x30] = 0;
        assert!(scd_info(&none).is_err());
    }

    #[test]
    fn take_finds_the_four_ids_and_names_what_is_missing() {
        let assets = vec![
            asset("se000001.win32.scd", scd(&[1; 10])),
            asset("se000002.win32.scd", scd(&[2; 10])),
            asset("se000004.win32.scd", scd(&[4; 7])),
            asset("se000006.win32.scd", scd(&[6; 3])),
            asset("se000003.win32.scd", b"junk".to_vec()),
        ];
        let mut notes = Vec::new();
        let t = take(&assets, &mut notes);
        assert_eq!(t.iter().map(|t| (t.kh1_id, t.what)).collect::<Vec<_>>(), vec![(1, "move"), (4, "error"), (6, "open")]);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].contains("sound 3") && notes[0].contains("not taken"), "{notes:?}");
        let mut notes = Vec::new();
        assert!(take(&assets[..1], &mut notes).len() == 1 && notes.len() == 3);
    }

    #[test]
    fn pack_and_manifest_say_where_each_sound_is() {
        let assets = vec![asset("se000001.win32.scd", scd(&[1; 10])), asset("se000006.win32.scd", scd(&[6; 3]))];
        let t = take(&assets, &mut Vec::new());
        let (bin, json) = build(&t, &[("se000.dat sha256", "ab".into())]);
        let second = (t[0].data.len() + 15) / 16 * 16;
        assert_eq!(&bin[..t[0].data.len()], &t[0].data[..]);
        assert_eq!(&bin[second..], &t[1].data[..]);
        assert!(json.contains(&format!("\"format\": \"{FORMAT}\"")));
        assert!(json.contains(&format!("\"kh1_id\": 6, \"use\": \"open\", \"name\": \"se000006.win32.scd\", \"group\": 9006, \"offset\": {second}, \"size\": {}", t[1].data.len())));
        assert!(json.contains(&format!("\"pack_size\": {}", bin.len())));
        assert!(line(&t[1]).ends_with("Re:CoM id 9006000"), "{}", line(&t[1]));
    }

    /// The player's own KH1 files, when given (`KH1_TEST_DATA`): every loose `*.win32.scd` there
    /// (e.g. `xa_ex_0010.mdls`'s PC assets) passes the header check; and with `se000.dat`'s PC
    /// assets as `se000.dat.assets/<name>`, every menu sound is there and is an SCD.
    #[test]
    fn owner_files() {
        let Ok(dir) = std::env::var("KH1_TEST_DATA") else { return };
        let dir = std::path::Path::new(&dir);
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.ends_with(".win32.scd") {
                let i = scd_info(&std::fs::read(e.path()).unwrap()).unwrap_or_else(|err| panic!("{n}: {err}"));
                assert!(i.sounds >= 1 && i.streams >= 1, "{n}: {i:?}");
                assert!(kh1_number(&n).is_some(), "{n}");
            }
        }
        let Ok(rd) = std::fs::read_dir(dir.join("se000.dat.assets")) else { return };
        let assets: Vec<HdAsset> = rd.flatten().map(|e| asset(&e.file_name().to_string_lossy(), std::fs::read(e.path()).unwrap())).collect();
        let mut notes = Vec::new();
        let t = take(&assets, &mut notes);
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(t.len(), 4);
    }
}
