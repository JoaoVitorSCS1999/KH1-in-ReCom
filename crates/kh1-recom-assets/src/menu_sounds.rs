//! KH1's command menu sounds (owner, 2026-11-01: "play KH1's exact menu sounds"): the asset step
//! takes KH1's system sound bank, `se000.win32.scd` (the one PC asset of the player's
//! `amusic/se000.dat`), unchanged, as `kh1-menu-sounds.bin` + `kh1-menu-sounds.json`; the DLL
//! (`crates/kh1-recom` `win/menu_sounds.rs`) gives it to Re:CoM's own sound-bank registration as
//! bank 9000 and plays its entry `n` as Re:CoM id `9000000 + n` with Re:CoM's own play function.
//! Notes: `docs/kh1-recom-assets.md` § KH1's menu sounds, `docs/recom-m7-command-menu.md`
//! § Sounds.
//!
//! KH1 (Steam; Checked (decomp + exe)):
//! - The menu plays `0x14017A040(id)` = `SslSeCreate(2, id)` (`0x1400D2FD0` → `0x1400DE460`) then
//!   `SslSePlay(2, id, …)`.
//! - Bank 2 is KH1's "system" bank (`0x1404D5EB8` = 2 in `.data`, never written). For it
//!   `SslSeCreate` takes the bank entry with bank 2 and number 0 and creates **sound `id` of
//!   that one SCD**: `CreateSound(handle, id, 0, 1, 0)` (`0x1400DE503`: `mov r8d,ebp`).
//! - At start (`0x140170630` / `0x140171410`) KH1 loads `amusic/se000.dat` (`0x1401791D0`) and
//!   registers its PC assets as bank 2 (`0x14017A080(2, …)` → `0x1400D30F0` → `0x1400DB910` →
//!   `0x1400E0F00`); the entry's number is `atoi(name + 2)` (`0x1400DB1F0`): `se000.win32.scd` →
//!   0. The owner's `se000.dat` has exactly that one asset (asset step run, 0.39.0).
//! - Re:CoM's create (`0x1402F3080`, at `0x1402F3133..0x1402F314E`) makes the sound with the
//!   same call, `CreateSound(handle, id % 1000, 0, 1, 0)`, through the same driver function
//!   (`0x14004C430` = KH1's `0x14004DBF0`, the same body). So Re:CoM's id `9000000 + n` plays
//!   the entry KH1's id `n` plays. Checked (exe).
//!
//! The file is an SQEX SCD (`SEDBSSCF`), read by the same driver in both exes (`ScdHeader::
//! GetScdHeaderSize` takes versions 1–4: KH1 `0x1400748F0`, Re:CoM `0x140073130`; Checked
//! (decomp)). It is passed on whole and unchanged: no SCD is rebuilt, and every entry keeps the
//! index KH1 plays it by.

use crate::{Error, Result};
use kh1_formats::egs::HdAsset;

/// The KH1 file holding KH1's system sound bank (bank 2).
pub const KH1_FILE: &str = "amusic/se000.dat";
/// KH1's number of that bank's entry (`atoi(name + 2)` of `se000.win32.scd`).
pub const KH1_NUMBER: i64 = 0;
/// The manifest the step writes (next to the other manifests).
pub const MANIFEST: &str = "kh1-menu-sounds.json";
/// The SCD, unchanged (next to the manifest).
pub const PACK: &str = "kh1-menu-sounds.bin";
/// The manifest's `format`.
pub const FORMAT: &str = "kh1-recom-assets menu-sounds 2";
/// Re:CoM's bank for it: entry `n` is Re:CoM id `GROUP × 1000 + n`. The DLL maps that group to
/// a bank of its own (Re:CoM's map `0x1402A8560` gives no bank for groups ≥ 8000).
pub const GROUP: u32 = 9000;
/// Where the step writes the menu entries as WAVs for listening (MS ADPCM streams only; not
/// part of the mod).
pub const PREVIEW_DIR: &str = "kh1-menu-sounds-preview";

/// KH1's command menu sounds (`0x14017A040` ids, Checked (decomp): `0x1402931B0`,
/// `0x1402937F0`, `0x140293480`, `0x1402948B0`): what each is for.
pub const SOUNDS: [(u32, &str); 4] = [(1, "move"), (3, "back"), (4, "error"), (6, "open")];

/// Longest name Re:CoM's bank entry keeps (`+0x30..+0x50`, `0x1402F0080`).
pub const NAME_MAX: usize = 31;

/// KH1's bank number for a PC asset name (`0x1400E0A50` / `0x1400DB910`'s filter, then
/// `0x1400DB1F0`'s `atoi(name + 2)`), or None when KH1 makes no numbered bank of it (`"title"`,
/// `music…` and `bgm_…` get −1 there).
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

/// One stream header of an SCD (the table at header `tables + 0x0C`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stream {
    /// Data bytes.
    pub data_size: i32,
    /// Channels.
    pub channels: i32,
    /// Sample rate.
    pub rate: i32,
    /// Codec (0xC = MS ADPCM, 6 = Vorbis, −1 = an empty slot).
    pub codec: i32,
    /// Loop start (bytes; 0 / 0 = none).
    pub loop_start: i32,
    /// Loop end.
    pub loop_end: i32,
    /// Where its header is in the file.
    pub at: usize,
    /// Its codec header (`WAVEFORMATEX` for MS ADPCM) bytes after the 0x20-byte header.
    pub extra: usize,
}

impl Stream {
    /// Length in samples per channel, for MS ADPCM (from its `WAVEFORMATEX`: block align,
    /// samples per block); None for other codecs.
    pub fn samples(&self, scd: &[u8]) -> Option<u64> {
        if self.codec != 0xC || self.extra < 20 {
            return None;
        }
        let w = scd.get(self.at + 0x20..self.at + 0x20 + 20)?;
        let block = u16::from_le_bytes([w[12], w[13]]) as u64;
        let per = u16::from_le_bytes([w[18], w[19]]) as u64;
        (block > 0).then(|| self.data_size as u64 / block * per)
    }

    /// The stream as a standalone `.wav` (its own `WAVEFORMATEX` and data, undecoded): MS ADPCM
    /// only.
    pub fn wav(&self, scd: &[u8]) -> Option<Vec<u8>> {
        if self.codec != 0xC || self.extra < 18 {
            return None;
        }
        let fmt = scd.get(self.at + 0x20..self.at + 0x20 + self.extra)?;
        let data = scd.get(self.at + 0x20 + self.extra..self.at + 0x20 + self.extra + self.data_size as usize)?;
        let mut w = Vec::with_capacity(fmt.len() + data.len() + 28);
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&((4 + 8 + fmt.len() + 8 + data.len()) as u32).to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
        w.extend_from_slice(fmt);
        w.extend_from_slice(b"data");
        w.extend_from_slice(&(data.len() as u32).to_le_bytes());
        w.extend_from_slice(data);
        Some(w)
    }
}

/// What the step reads from an SCD's header (for the checks and the report).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScdInfo {
    /// Header `+8`.
    pub version: u32,
    /// The count at `tables + 0`: the sound table a play index picks from (Assumption: the
    /// first table, as the 1-sound files have 1).
    pub sounds: u16,
    /// The count at `tables + 2`.
    pub tracks: u16,
    /// Stream headers (count at `tables + 4`, offsets at `tables + 0x0C`).
    pub streams: Vec<Stream>,
}

/// Checks an SCD the way both drivers read its header (little-endian `SEDBSSCF`, version 1–4,
/// the file-size word, the tables) and reads its stream headers.
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
    let (sounds, tracks, n) = (u16_at(t)?, u16_at(t + 2)?, u16_at(t + 4)? as usize);
    if sounds == 0 || n == 0 {
        return Err(Error::Format(format!("SCD has {sounds} sounds, {n} streams")));
    }
    let table = u32_at(t + 0xC)? as usize;
    let mut streams = Vec::with_capacity(n);
    for k in 0..n {
        let s = u32_at(table + 4 * k)? as usize;
        let i = |o: usize| u32_at(s + o).map(|v| v as i32);
        let st = Stream { data_size: i(0)?, channels: i(4)?, rate: i(8)?, codec: i(0xC)?, loop_start: i(0x10)?, loop_end: i(0x14)?, at: s, extra: i(0x18)?.max(0) as usize };
        if st.codec != -1 && (st.data_size < 0 || s + 0x20 + st.extra + st.data_size as usize > d.len()) {
            return Err(Error::Format(format!("SCD stream {k} ({} bytes at {s:#x}) runs past the file", st.data_size)));
        }
        streams.push(st);
    }
    Ok(ScdInfo { version, sounds, tracks, streams })
}

/// KH1's system bank among `se000.dat`'s PC assets: the one KH1 numbers 0 (`se000.win32.scd`).
pub fn find(assets: &[HdAsset]) -> Option<&HdAsset> {
    assets.iter().find(|a| kh1_number(&a.name) == Some(KH1_NUMBER))
}

/// The bank taken.
#[derive(Clone, Debug)]
pub struct Taken {
    /// KH1's asset name (kept as Re:CoM's bank entry name).
    pub name: String,
    /// The SCD, unchanged.
    pub data: Vec<u8>,
    /// Its header.
    pub info: ScdInfo,
}

/// Takes KH1's system bank from `assets` (KH1's `amusic/se000.dat`), checks that every menu id
/// is an entry of it, and says so per id in `notes`.
pub fn take(assets: &[HdAsset], notes: &mut Vec<String>) -> Result<Taken> {
    let names: Vec<&str> = assets.iter().map(|a| a.name.as_str()).collect();
    let a = find(assets).ok_or_else(|| Error::Format(format!("{KH1_FILE}: no asset KH1 numbers 0 (its PC assets: {names:?})")))?;
    if a.name.len() > NAME_MAX || a.name.contains('\0') {
        return Err(Error::Format(format!("{KH1_FILE}: asset name {:?} too long for Re:CoM's bank entry", a.name)));
    }
    let info = scd_info(&a.data).map_err(|e| Error::Format(format!("{KH1_FILE}: {}: {e}", a.name)))?;
    for (id, what) in SOUNDS {
        if id >= info.sounds as u32 {
            notes.push(format!("{}: {} sounds, so no entry {id} ({what}); Re:CoM's sound stays for it", a.name, info.sounds));
        }
    }
    Ok(Taken { name: a.name.clone(), data: a.data.clone(), info })
}

/// The menu ids the bank has an entry for.
pub fn present(t: &Taken) -> Vec<(u32, &'static str)> {
    SOUNDS.iter().copied().filter(|(id, _)| *id < t.info.sounds as u32).collect()
}

/// Stream `n`, as JSON (the stream with the entry's index: that sound `n` plays stream `n` is an
/// Assumption the owner's listening checks).
fn stream_json(t: &Taken, n: usize) -> String {
    match t.info.streams.get(n) {
        Some(s) => format!(
            "{{\"channels\": {}, \"rate\": {}, \"codec\": {}, \"data_size\": {}, \"samples\": {}, \"loop\": [{}, {}]}}",
            s.channels,
            s.rate,
            s.codec,
            s.data_size,
            s.samples(&t.data).map_or("null".to_string(), |x| x.to_string()),
            s.loop_start,
            s.loop_end
        ),
        None => "null".into(),
    }
}

/// The pack (the SCD itself) and its manifest.
pub fn build(t: &Taken, sources: &[(&str, String)]) -> (Vec<u8>, String) {
    use std::fmt::Write;
    let rows: Vec<String> = present(t)
        .iter()
        .map(|(id, what)| format!("    {{\"kh1_id\": {id}, \"use\": {what:?}, \"entry\": {id}, \"recom_id\": {}, \"stream\": {}}}", GROUP * 1000 + id, stream_json(t, *id as usize)))
        .collect();
    let mut s = String::new();
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"format\": \"{FORMAT}\",");
    let _ = writeln!(s, "  \"pack\": \"{PACK}\",");
    let _ = writeln!(s, "  \"pack_size\": {},", t.data.len());
    let _ = writeln!(s, "  \"pack_sha256\": {:?},", kh1_formats::egs::sha256_hex(&t.data));
    let _ = writeln!(s, "  \"group\": {GROUP},");
    let _ = writeln!(s, "  \"name\": {:?},", t.name);
    let _ = writeln!(s, "  \"source\": \"{KH1_FILE}\",");
    let _ = writeln!(s, "  \"scd\": {{\"version\": {}, \"sounds\": {}, \"tracks\": {}, \"streams\": {}}},", t.info.version, t.info.sounds, t.info.tracks, t.info.streams.len());
    let _ = writeln!(s, "  \"sounds\": [\n{}\n  ],", rows.join(",\n"));
    let src: Vec<String> = sources.iter().map(|(k, v)| format!("{k:?}: {v:?}")).collect();
    let _ = writeln!(s, "  \"sources\": {{{}}}", src.join(", "));
    let _ = writeln!(s, "}}");
    (t.data.clone(), s)
}

/// The step's lines: the bank, then each menu id's entry.
pub fn lines(t: &Taken) -> Vec<String> {
    let mut out = vec![format!(
        "KH1 menu sounds: {} ({} bytes, SCD v{}, {} sounds, {} tracks, {} streams) → Re:CoM bank {GROUP}",
        t.name,
        t.data.len(),
        t.info.version,
        t.info.sounds,
        t.info.tracks,
        t.info.streams.len()
    )];
    for (id, what) in present(t) {
        let s = match t.info.streams.get(id as usize) {
            Some(s) => {
                let codec = match s.codec {
                    0xC => "MS ADPCM".to_string(),
                    6 => "Vorbis".to_string(),
                    -1 => "empty".to_string(),
                    c => format!("codec {c:#x}"),
                };
                let len = match s.samples(&t.data) {
                    Some(n) if s.rate > 0 => format!("{n} samples = {:.3} s", n as f64 / s.rate as f64),
                    _ => format!("{} bytes", s.data_size),
                };
                format!("stream {id}: {} ch, {} Hz, {codec}, {len}{}", s.channels, s.rate, if s.loop_end > 0 { ", loops" } else { "" })
            }
            None => format!("no stream {id} (the sound table maps it elsewhere)"),
        };
        out.push(format!("KH1 menu sound {id} ({what}): entry {id}, {s} → Re:CoM id {}", GROUP * 1000 + id));
    }
    out
}

/// The menu entries as `.wav` files (`se000_<id>_<use>.wav`) for listening: MS ADPCM only.
pub fn previews(t: &Taken) -> Vec<(String, Vec<u8>)> {
    present(t)
        .iter()
        .filter_map(|(id, what)| t.info.streams.get(*id as usize).and_then(|s| s.wav(&t.data)).map(|w| (format!("se000_{id}_{what}.wav"), w)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An SCD laid out as KH1's (`se011001.win32.scd`: tables at 0x30, stream header offsets at
    /// `tables + 0x0C`), with `n` sounds and `n` MS ADPCM streams of the given payloads.
    pub(crate) fn scd(payloads: &[&[u8]]) -> Vec<u8> {
        let n = payloads.len();
        let table = 0x80usize;
        let mut d = vec![0u8; table + 4 * n];
        d[..8].copy_from_slice(b"SEDBSSCF");
        d[8..12].copy_from_slice(&3u32.to_le_bytes());
        d[0xD] = 4;
        d[0xE..0x10].copy_from_slice(&0x30u16.to_le_bytes());
        for o in [0x30, 0x32, 0x34] {
            d[o..o + 2].copy_from_slice(&(n as u16).to_le_bytes());
        }
        d[0x3C..0x40].copy_from_slice(&(table as u32).to_le_bytes());
        for (k, p) in payloads.iter().enumerate() {
            let at = d.len();
            d[table + 4 * k..table + 4 * k + 4].copy_from_slice(&(at as u32).to_le_bytes());
            for v in [p.len() as i32, 1, 48000, 0xC, 0, 0, 20, 0] {
                d.extend_from_slice(&v.to_le_bytes());
            }
            // WAVEFORMATEX: MS ADPCM, mono, 48000, avg, block align 4, 4 bits, cb 2, 4 samples a block.
            for v in [2u16, 1] {
                d.extend_from_slice(&v.to_le_bytes());
            }
            for v in [48000u32, 24000] {
                d.extend_from_slice(&v.to_le_bytes());
            }
            for v in [4u16, 4, 2, 4] {
                d.extend_from_slice(&v.to_le_bytes());
            }
            d.extend_from_slice(p);
        }
        let len = d.len() as u32;
        d[0x10..0x14].copy_from_slice(&len.to_le_bytes());
        d
    }

    fn asset(name: &str, data: Vec<u8>) -> HdAsset {
        HdAsset { name: name.into(), orig_offset: -1, data }
    }

    #[test]
    fn kh1_numbers_follow_kh1s_atoi_from_the_third_character() {
        assert_eq!(kh1_number("se000.win32.scd"), Some(0));
        assert_eq!(kh1_number("se000001.win32.scd"), Some(1));
        assert_eq!(kh1_number("se011001.win32.scd"), Some(11001));
        assert_eq!(kh1_number("se000.wd"), None);
        assert_eq!(kh1_number("music132.win32.scd"), None);
        assert_eq!(kh1_number("-c0.cvbl"), None);
        assert_eq!(kh1_number("xse4.win32.scd"), Some(0));
    }

    #[test]
    fn scd_checks_read_every_stream_and_refuse_the_rest() {
        let d = scd(&[&[1; 8], &[2; 12]]);
        let i = scd_info(&d).unwrap();
        assert_eq!((i.version, i.sounds, i.streams.len()), (3, 2, 2));
        assert_eq!((i.streams[1].data_size, i.streams[1].channels, i.streams[1].rate, i.streams[1].codec), (12, 1, 48000, 0xC));
        // 12 bytes / block 4 × 4 samples a block.
        assert_eq!(i.streams[1].samples(&d), Some(12));
        let mut bad = d.clone();
        bad[0] = b'X';
        assert!(scd_info(&bad).is_err());
        let mut v9 = d.clone();
        v9[8] = 9;
        assert!(scd_info(&v9).is_err());
        let mut short = d.clone();
        short.pop();
        assert!(scd_info(&short).is_err(), "the file-size word must match");
    }

    #[test]
    fn the_bank_is_the_asset_kh1_numbers_0_and_each_menu_id_is_an_entry() {
        let payloads: Vec<Vec<u8>> = (0..8u8).map(|k| vec![k; 4 * (k as usize + 1)]).collect();
        let refs: Vec<&[u8]> = payloads.iter().map(|p| p.as_slice()).collect();
        let assets = vec![asset("se011001.win32.scd", scd(&refs[..1])), asset("se000.win32.scd", scd(&refs))];
        let mut notes = Vec::new();
        let t = take(&assets, &mut notes).unwrap();
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!((t.name.as_str(), t.info.sounds), ("se000.win32.scd", 8));
        assert_eq!(present(&t).len(), 4);
        let l = lines(&t);
        assert!(l[0].contains("8 sounds") && l[0].ends_with("Re:CoM bank 9000"), "{}", l[0]);
        assert!(l[4].starts_with("KH1 menu sound 6 (open): entry 6, stream 6: 1 ch, 48000 Hz, MS ADPCM, 28 samples") && l[4].ends_with("Re:CoM id 9000006"), "{}", l[4]);
        // Fewer entries than the menu ids: a note for each missing one.
        let few = vec![asset("se000.win32.scd", scd(&refs[..5]))];
        let mut notes = Vec::new();
        let t = take(&few, &mut notes).unwrap();
        assert_eq!((present(&t).len(), notes.len()), (3, 1));
        assert!(notes[0].contains("no entry 6"));
        // No bank 0: an error naming the assets.
        let e = take(&assets[..1], &mut Vec::new()).unwrap_err().to_string();
        assert!(e.contains("se011001.win32.scd"), "{e}");
    }

    #[test]
    fn pack_is_the_scd_unchanged_and_the_manifest_says_each_entry() {
        let payloads: Vec<Vec<u8>> = (0..7u8).map(|k| vec![k; 8]).collect();
        let refs: Vec<&[u8]> = payloads.iter().map(|p| p.as_slice()).collect();
        let d = scd(&refs);
        let t = take(&[asset("se000.win32.scd", d.clone())], &mut Vec::new()).unwrap();
        let (bin, json) = build(&t, &[("se000.dat sha256", "ab".into())]);
        assert_eq!(bin, d);
        assert!(json.contains(&format!("\"format\": \"{FORMAT}\"")));
        assert!(json.contains(&format!("\"pack_size\": {}", d.len())));
        assert!(json.contains("\"group\": 9000,"));
        assert!(json.contains("\"name\": \"se000.win32.scd\""));
        assert!(json.contains("{\"kh1_id\": 4, \"use\": \"error\", \"entry\": 4, \"recom_id\": 9000004, \"stream\": {\"channels\": 1, \"rate\": 48000, \"codec\": 12, \"data_size\": 8, \"samples\": 8, \"loop\": [0, 0]}}"), "{json}");
        let p = previews(&t);
        assert_eq!(p.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), vec!["se000_1_move.wav", "se000_3_back.wav", "se000_4_error.wav", "se000_6_open.wav"]);
        let w = &p[3].1;
        assert!(w.starts_with(b"RIFF") && &w[8..16] == b"WAVEfmt " && w.ends_with(&[6; 8]));
    }

    /// The player's own KH1 files, when given (`KH1_TEST_DATA`): every loose `*.win32.scd` there
    /// passes the header check and its MS ADPCM streams have a length; with `se000.dat`'s PC
    /// asset as `se000.win32.scd`, every menu id is an entry.
    #[test]
    fn owner_files() {
        let Ok(dir) = std::env::var("KH1_TEST_DATA") else { return };
        let dir = std::path::Path::new(&dir);
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.ends_with(".win32.scd") {
                let d = std::fs::read(e.path()).unwrap();
                let i = scd_info(&d).unwrap_or_else(|err| panic!("{n}: {err}"));
                assert!(i.sounds >= 1 && !i.streams.is_empty(), "{n}: {i:?}");
                for s in i.streams.iter().filter(|s| s.codec == 0xC) {
                    assert!(s.samples(&d).is_some_and(|x| x > 0) && s.wav(&d).is_some(), "{n}");
                }
            }
        }
        let Ok(d) = std::fs::read(dir.join("se000.win32.scd")) else { return };
        let mut notes = Vec::new();
        let t = take(&[asset("se000.win32.scd", d)], &mut notes).unwrap();
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(present(&t).len(), 4);
    }
}
