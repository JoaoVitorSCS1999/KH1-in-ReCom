//! The build stamp of the DLL's first-launch build (`docs/openkh-github-install.md`).
//!
//! The DLL builds the game-made files into a cache folder of its own (the mod's own folder's
//! `generated/`, which OpenKH's Build never wipes) and copies them into Panacea's live folder
//! (`<mod_path>/recom`, which Build wipes). The stamp says which build the cache holds:
//!
//! - `step`: [`STEP_HASH`], a hash of the step's own sources (any change to the step builds again);
//! - `inputs`: a hash of the game files the step reads, by name, size and time ([`inputs`]);
//! - `file <size> <path>`: every file written, relative to the cache, with `/`.
//!
//! It is written last, after the step succeeded, so a cache without one is never used.

use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// The step's source hash (`step_hash.rs`).
pub const STEP_HASH: &str = env!("KH1_ASSET_STEP_HASH");
/// The stamp's file name, in the cache folder.
pub const STAMP_FILE: &str = "kh1-assets.stamp";
/// Written instead of the stamp when a build failed: the same build is not tried again until the
/// step or the inputs change (or the file is deleted).
pub const FAILED_FILE: &str = "kh1-assets.failed";
const MAGIC: &str = "kh1-recom-assets stamp 1";

/// The KH 1.5+2.5 executables the step may read the key table from.
pub const EXES: [&str; 2] = ["KINGDOM HEARTS Re_Chain of Memories.exe", "KINGDOM HEARTS FINAL MIX.exe"];

/// One input file: its name (relative to the game folder, `/`), size and modified time (s).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    /// `Image/dt/kh1_first.hed`, …
    pub name: String,
    /// Bytes.
    pub size: u64,
    /// Seconds since 1970 (0 when unknown).
    pub time: u64,
}

/// Whether an `Image/dt` file is one the step reads: Re:CoM's, KH1's and KH2's archives.
pub fn is_input_archive(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    (n.starts_with("recom.") || n.starts_with("kh1_") || n.starts_with("kh2_")) && (n.ends_with(".hed") || n.ends_with(".pkg"))
}

/// The step's inputs in a game folder, sorted by name: the archives ([`is_input_archive`]) and
/// the executables ([`EXES`]) that are there.
pub fn inputs(game: &Path) -> Vec<Input> {
    let mut out = Vec::new();
    let meta = |p: &Path| std::fs::metadata(p).ok().filter(|m| m.is_file());
    let time = |m: &std::fs::Metadata| m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    if let Ok(rd) = std::fs::read_dir(game.join("Image").join("dt")) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !is_input_archive(&name) {
                continue;
            }
            if let Some(m) = meta(&e.path()) {
                out.push(Input { name: format!("Image/dt/{name}"), size: m.len(), time: time(&m) });
            }
        }
    }
    for exe in EXES {
        if let Some(m) = meta(&game.join(exe)) {
            out.push(Input { name: exe.to_string(), size: m.len(), time: time(&m) });
        }
    }
    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    out
}

/// A hash of an input list (FNV-1a 64 over name, size and time; names compared without case).
pub fn inputs_hash(list: &[Input]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |b: &[u8]| {
        for &x in b {
            h ^= x as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for i in list {
        eat(i.name.to_ascii_lowercase().as_bytes());
        eat(&[0]);
        eat(&i.size.to_le_bytes());
        eat(&i.time.to_le_bytes());
    }
    format!("{h:016x}")
}

/// Whether the inputs hold what the step needs at all: Re:CoM's `Recom.hed` and KH1's
/// `kh1_*.hed`. `Err` names what is missing.
pub fn inputs_usable(list: &[Input]) -> Result<(), String> {
    let has = |f: &dyn Fn(&str) -> bool| list.iter().any(|i| f(&i.name.to_ascii_lowercase()));
    let mut missing = Vec::new();
    if !has(&|n| n == "image/dt/recom.hed") {
        missing.push("Re:CoM's Image/dt/Recom.hed");
    }
    if !has(&|n| n.starts_with("image/dt/kh1_") && n.ends_with(".hed")) {
        missing.push("KH1 Final Mix's Image/dt/kh1_*.hed");
    }
    if !has(&|n| EXES.iter().any(|e| e.eq_ignore_ascii_case(n))) {
        missing.push("the Re:CoM or KH1 executable");
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("not found: {}", missing.join(", ")))
    }
}

/// A build's stamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    /// [`STEP_HASH`] of the build.
    pub step: String,
    /// [`inputs_hash`] of the build.
    pub inputs: String,
    /// The files, relative to the cache (`/`), with their sizes.
    pub files: Vec<(String, u64)>,
}

impl Stamp {
    /// The stamp's text.
    pub fn to_text(&self) -> String {
        let mut s = format!("{MAGIC}\nstep {}\ninputs {}\n", self.step, self.inputs);
        for (p, n) in &self.files {
            s.push_str(&format!("file {n} {p}\n"));
        }
        s
    }

    /// Reads a stamp (or a failed marker's first lines); `None` when it is not one.
    pub fn parse(text: &str) -> Option<Stamp> {
        let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
        if lines.next()? != MAGIC {
            return None;
        }
        let (mut step, mut inputs, mut files) = (None, None, Vec::new());
        for l in lines {
            if let Some(v) = l.strip_prefix("step ") {
                step = Some(v.to_string());
            } else if let Some(v) = l.strip_prefix("inputs ") {
                inputs = Some(v.to_string());
            } else if let Some(v) = l.strip_prefix("file ") {
                let (n, p) = v.split_once(' ')?;
                if p.is_empty() || p.split('/').any(|c| c.is_empty() || c == "." || c == "..") {
                    return None;
                }
                files.push((p.to_string(), n.parse().ok()?));
            }
        }
        Some(Stamp { step: step?, inputs: inputs?, files })
    }

    /// Whether the stamp is for this step and these inputs.
    pub fn is_for(&self, step: &str, inputs: &str) -> bool {
        self.step == step && self.inputs == inputs
    }

    /// The stamp of a finished build: the files written under `root` (others are left out).
    pub fn of_build(step: &str, inputs: &str, root: &Path, written: &[PathBuf]) -> Stamp {
        let mut files = Vec::new();
        for p in written {
            let Ok(rel) = p.strip_prefix(root) else { continue };
            let rel = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join("/");
            if let Ok(m) = std::fs::metadata(p) {
                if !files.iter().any(|(f, _): &(String, u64)| f == &rel) {
                    files.push((rel, m.len()));
                }
            }
        }
        Stamp { step: step.to_string(), inputs: inputs.to_string(), files }
    }
}

/// A failed marker's text: the stamp's head (no files) and the reason.
pub fn failed_text(step: &str, inputs: &str, why: &str) -> String {
    let mut s = Stamp { step: step.into(), inputs: inputs.into(), files: Vec::new() }.to_text();
    for l in why.lines() {
        s.push_str(&format!("error {l}\n"));
    }
    s
}

/// The reason in a failed marker's text.
pub fn failed_reason(text: &str) -> String {
    text.lines().filter_map(|l| l.strip_prefix("error ")).collect::<Vec<_>>().join("\n")
}

/// A path under a folder from a stamp's `/` path.
pub fn under(root: &Path, rel: &str) -> PathBuf {
    rel.split('/').fold(root.to_path_buf(), |p, c| p.join(c))
}

/// Whether every file of the stamp is in the cache with its size.
pub fn cache_complete(cache: &Path, st: &Stamp) -> Result<(), String> {
    for (rel, n) in &st.files {
        match std::fs::metadata(under(cache, rel)) {
            Ok(m) if m.is_file() && m.len() == *n => {}
            Ok(m) => return Err(format!("{rel}: {} bytes, the stamp says {n}", m.len())),
            Err(e) => return Err(format!("{rel}: {e}")),
        }
    }
    Ok(())
}

/// What [`place`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Placed {
    /// Files copied into the live folder.
    pub copied: usize,
    /// Files already there (same size and time as the cache's).
    pub kept: usize,
    /// Bytes copied.
    pub bytes: u64,
}

/// Copies the stamp's files from the cache into the live folder, each one only when the live
/// copy is missing or differs in size or modified time (a Build's copy of an older file, say).
/// The copy gets the cache file's time, so the next start keeps it.
pub fn place(cache: &Path, live: &Path, st: &Stamp) -> io::Result<Placed> {
    place_with(cache, live, st, |_, _| {})
}

/// [`place`], calling `on(k, n)` before file `k` of the stamp's `n`.
pub fn place_with(cache: &Path, live: &Path, st: &Stamp, mut on: impl FnMut(usize, usize)) -> io::Result<Placed> {
    let mut done = Placed::default();
    for (k, (rel, _)) in st.files.iter().enumerate() {
        on(k, st.files.len());
        let src = under(cache, rel);
        let dst = under(live, rel);
        let sm = std::fs::metadata(&src)?;
        let st_time = sm.modified()?;
        if let Ok(dm) = std::fs::metadata(&dst) {
            if dm.is_file() && dm.len() == sm.len() && dm.modified().ok() == Some(st_time) {
                done.kept += 1;
                continue;
            }
            if dm.is_file() {
                std::fs::remove_file(&dst)?;
            }
        }
        if let Some(d) = dst.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::copy(&src, &dst)?;
        std::fs::OpenOptions::new().write(true).open(&dst)?.set_modified(st_time)?;
        done.copied += 1;
        done.bytes += sm.len();
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kh1-stamp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_stamp_reads_back() {
        let st = Stamp { step: STEP_HASH.into(), inputs: "00ff".into(), files: vec![("PLAYER/0001/PL0001.ANN".into(), 10), ("kh1-stats.json".into(), 3)] };
        assert_eq!(Stamp::parse(&st.to_text()), Some(st.clone()));
        assert_eq!(Stamp::parse(&st.to_text().replace('\n', "\r\n")), Some(st.clone()));
        assert!(st.is_for(STEP_HASH, "00ff") && !st.is_for(STEP_HASH, "00fe") && !st.is_for("x", "00ff"));
        assert_eq!(STEP_HASH.len(), 16);
        // Never a path that leaves the folder.
        assert_eq!(Stamp::parse(&format!("{MAGIC}\nstep a\ninputs b\nfile 1 ../x\n")), None);
        assert_eq!(Stamp::parse("something else\nstep a\ninputs b\n"), None);
        let f = failed_text("a", "b", "KH1 files: not found\nsecond line");
        assert!(Stamp::parse(&f).is_some_and(|s| s.is_for("a", "b") && s.files.is_empty()));
        assert_eq!(failed_reason(&f), "KH1 files: not found\nsecond line");
    }

    #[test]
    fn inputs_are_the_archives_and_the_exes() {
        let g = tmp("inputs");
        let dt = g.join("Image").join("dt");
        std::fs::create_dir_all(&dt).unwrap();
        for n in ["Recom.hed", "Recom.pkg", "kh1_first.hed", "kh1_first.pkg", "kh2_first.hed", "bbs_first.hed", "kh1_first.txt"] {
            std::fs::write(dt.join(n), n).unwrap();
        }
        assert!(inputs_usable(&inputs(&g)).is_err(), "no exe yet");
        std::fs::write(g.join(EXES[0]), b"MZ").unwrap();
        let list = inputs(&g);
        let names: Vec<&str> = list.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["Image/dt/kh1_first.hed", "Image/dt/kh1_first.pkg", "Image/dt/kh2_first.hed", "Image/dt/Recom.hed", "Image/dt/Recom.pkg", EXES[0]]);
        assert_eq!(inputs_usable(&list), Ok(()));
        let h = inputs_hash(&list);
        // A changed size changes the hash.
        std::fs::write(dt.join("kh1_first.pkg"), b"longer than before").unwrap();
        assert_ne!(inputs_hash(&inputs(&g)), h);
        // No KH1: named.
        std::fs::remove_file(dt.join("kh1_first.hed")).unwrap();
        assert!(inputs_usable(&inputs(&g)).unwrap_err().contains("KH1 Final Mix"));
        let _ = std::fs::remove_dir_all(&g);
    }

    #[test]
    fn place_copies_only_what_differs() {
        let d = tmp("place");
        let (cache, live) = (d.join("cache"), d.join("live"));
        for (rel, body) in [("PLAYER/0001/PL0001.ANN", "pack"), ("raw/FORM/0002/FO0002.SPR", "spr"), ("kh1-stats.json", "{}")] {
            let p = under(&cache, rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        let written: Vec<PathBuf> = ["PLAYER/0001/PL0001.ANN", "raw/FORM/0002/FO0002.SPR", "kh1-stats.json"].iter().map(|r| under(&cache, r)).chain([d.join("elsewhere.bin")]).collect();
        let st = Stamp::of_build("s", "i", &cache, &written);
        assert_eq!(st.files, vec![("PLAYER/0001/PL0001.ANN".to_string(), 4), ("raw/FORM/0002/FO0002.SPR".to_string(), 3), ("kh1-stats.json".to_string(), 2)]);
        assert_eq!(cache_complete(&cache, &st), Ok(()));
        assert_eq!(place(&cache, &live, &st).unwrap(), Placed { copied: 3, kept: 0, bytes: 9 });
        assert_eq!(std::fs::read_to_string(under(&live, "raw/FORM/0002/FO0002.SPR")).unwrap(), "spr");
        // A second start keeps them.
        let mut seen = Vec::new();
        assert_eq!(place_with(&cache, &live, &st, |k, n| seen.push((k, n))).unwrap(), Placed { copied: 0, kept: 3, bytes: 0 });
        assert_eq!(seen, [(0, 3), (1, 3), (2, 3)], "one report before each file");
        // A Build's other copy (same size, other time) and a wiped file are copied again.
        std::fs::write(under(&live, "kh1-stats.json"), "[]").unwrap();
        std::fs::OpenOptions::new().write(true).open(under(&live, "kh1-stats.json")).unwrap().set_modified(UNIX_EPOCH).unwrap();
        std::fs::remove_file(under(&live, "PLAYER/0001/PL0001.ANN")).unwrap();
        assert_eq!(place(&cache, &live, &st).unwrap(), Placed { copied: 2, kept: 1, bytes: 6 });
        assert_eq!(std::fs::read_to_string(under(&live, "kh1-stats.json")).unwrap(), "{}");
        // A damaged cache is reported.
        std::fs::write(under(&cache, "kh1-stats.json"), "{ }").unwrap();
        assert!(cache_complete(&cache, &st).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
