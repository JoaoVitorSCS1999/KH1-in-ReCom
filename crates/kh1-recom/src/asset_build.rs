//! The first-launch build of the game-made files (owner, 2026-11-05: "DLL builds on first
//! launch"; `docs/openkh-github-install.md`). The rules here are plain Rust (tested on Linux);
//! the Windows side (the helper process, the console, the message box) is `win::assets`.
//!
//! - **Where the files are used:** Panacea's live folder `<mod_path>/recom` (the folder
//!   Panacea's `OnInit` names). Panacea lists that folder once, when the game sets its replace
//!   path, after every DLL's `OnInit` (Checked (source): `Panacea.cpp` `FakePackageFile`,
//!   `LoadDLLs` from `DllMain`). So files placed there during `OnInit` are used in the same run.
//! - **Where they are kept:** OpenKH's Build deletes the live folder (`ModsService.RunPatcherAsync`,
//!   Checked (source)), so the build goes to a cache Build never touches: `generated/` in the
//!   mod's own folder ([`find_mod_folder`]; ignored by git, so OpenKH's update keeps it), else
//!   `kh1-recom-generated/` in the game folder. Each start copies whatever a Build removed back
//!   into the live folder ([`kh1_recom_assets::stamp::place`]).
//! - **When it builds:** when the cache has no stamp for this step and these inputs
//!   ([`decide`]): first launch, a mod update that changed the step, a game update.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kh1_recom_assets::stamp::{self, Input, Stamp, FAILED_FILE, STAMP_FILE};
use kh1_recom_assets::step;

/// The cache folder's name in the mod's own folder (in the repo's `.gitignore`).
pub const CACHE_DIR: &str = "generated";
/// The cache folder's name in the game folder, when the mod's own folder is not found.
pub const GAME_CACHE_DIR: &str = "kh1-recom-generated";
/// The build's own log, in the game folder (next to `kh1-recom.log`).
pub const BUILD_LOG: &str = "kh1-recom-build.log";
/// The job the DLL hands its helper process, in the game folder.
pub const JOB_FILE: &str = "kh1-recom-build.job";
/// The GitHub repository OpenKH installs (`<mods>/Recom/<owner>/<repo>`).
pub const REPO: (&str, &str) = ("PM2CRAZY", "KH1-in-ReCom");
/// The DLL as the repository carries it (the CI bot's build), and as the owner's local mod does.
pub const DLL_SOURCES: [&str; 2] = ["openkh/kh1-recom.dll", "kh1-recom.dll"];

/// A top-level `key: value` of a YAML file (OpenKH's `mods-manager.yml`), quotes removed.
pub fn yaml_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let l = l.trim_end_matches('\r');
        let (k, v) = l.split_once(':')?;
        if k != key {
            return None;
        }
        let v = v.trim();
        let v = if v.len() >= 2 && ((v.starts_with('\'') && v.ends_with('\'')) || (v.starts_with('"') && v.ends_with('"'))) { &v[1..v.len() - 1] } else { v };
        let v = v.replace("''", "'");
        (!v.is_empty() && v != "~" && v != "null").then_some(v)
    })
}

/// The folders OpenKH keeps Re:CoM's installed mods in, given OpenKH's own folder: as the Mods
/// Manager's `InstalledModsPath` computes it from `mods-manager.yml` (`installedModsPath`, unless
/// it equals `compiledModPath`), then the default `<openkh>/mods/Recom`. Checked (source:
/// `ConfigurationService.InstalledModsPath`, `LaunchGame` "Recom").
pub fn installed_mods_dirs(openkh: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(t) = std::fs::read_to_string(openkh.join("mods-manager.yml")) {
        let inst = yaml_value(&t, "installedModsPath");
        let comp = yaml_value(&t, "compiledModPath");
        match inst {
            Some(i) if comp.as_deref() != Some(i.as_str()) => out.push(PathBuf::from(i).join("Recom")),
            Some(i) => out.push(PathBuf::from(i).join("mods").join("Recom")),
            None => {}
        }
    }
    let d = openkh.join("mods").join("Recom");
    if !out.contains(&d) {
        out.push(d);
    }
    out
}

/// This mod's own folder among the installed Re:CoM mods (`<dir>/<owner>/<repo>` with a
/// `mod.yml`): the one whose DLL source has the running DLL's size (Build copies it unchanged),
/// else OpenKH's GitHub name for it ([`REPO`]). Only folders whose `mod.yml` names
/// `kh1-recom.dll` count.
pub fn find_mod_folder(mods_dirs: &[PathBuf], dll_size: Option<u64>) -> Option<PathBuf> {
    let mut best: Option<(u8, PathBuf)> = None;
    for root in mods_dirs {
        let Ok(owners) = std::fs::read_dir(root) else { continue };
        for o in owners.flatten() {
            let Ok(repos) = std::fs::read_dir(o.path()) else { continue };
            for r in repos.flatten() {
                let dir = r.path();
                let Ok(yml) = std::fs::read_to_string(dir.join("mod.yml")) else { continue };
                if !yml.contains("kh1-recom.dll") {
                    continue;
                }
                let named = o.file_name().to_string_lossy().eq_ignore_ascii_case(REPO.0) && r.file_name().to_string_lossy().eq_ignore_ascii_case(REPO.1);
                let same_dll = dll_size.is_some_and(|n| DLL_SOURCES.iter().any(|s| std::fs::metadata(dir.join(s)).is_ok_and(|m| m.len() == n)));
                let score = (same_dll as u8) * 2 + named as u8;
                if score > 0 && best.as_ref().is_none_or(|(b, _)| score > *b) {
                    best = Some((score, dir));
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// The cache folder: `generated/` in the mod's own folder, else `kh1-recom-generated/` in the
/// game folder.
pub fn cache_dir(mod_folder: Option<&Path>, game: &Path) -> PathBuf {
    match mod_folder {
        Some(m) => m.join(CACHE_DIR),
        None => game.join(GAME_CACHE_DIR),
    }
}

/// What to do at start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// `asset_build = off`.
    Off,
    /// The cache holds this build: copy what the live folder lacks.
    Place(Stamp),
    /// Build now (why, for the log and the window).
    Build(String),
    /// This very build failed before (its reason): not tried again until the step or the inputs
    /// change, or the marker is deleted.
    Failed(String),
}

/// The decision for a cache, the step's hash and the game's inputs.
pub fn decide(enabled: bool, cache: &Path, step_hash: &str, inputs: &[Input]) -> Decision {
    if !enabled {
        return Decision::Off;
    }
    let ih = stamp::inputs_hash(inputs);
    let old = std::fs::read_to_string(cache.join(STAMP_FILE)).ok().and_then(|t| Stamp::parse(&t));
    if let Some(st) = &old {
        if st.is_for(step_hash, &ih) {
            return match stamp::cache_complete(cache, st) {
                Ok(()) => Decision::Place(st.clone()),
                Err(e) => Decision::Build(format!("the built files are incomplete ({e})")),
            };
        }
    }
    if let Ok(t) = std::fs::read_to_string(cache.join(FAILED_FILE)) {
        if Stamp::parse(&t).is_some_and(|s| s.is_for(step_hash, &ih)) {
            return Decision::Failed(stamp::failed_reason(&t));
        }
    }
    Decision::Build(match old {
        None => "the KH1 files have not been built yet".into(),
        Some(st) if st.step != step_hash => "the mod's asset step changed (a mod update)".into(),
        Some(_) => "the game's files changed (a game update)".into(),
    })
}

/// The helper process's job: what to build, from where, to where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    /// The KH 1.5+2.5 folder.
    pub game: PathBuf,
    /// The cache folder the files go to.
    pub cache: PathBuf,
    /// The build's log.
    pub log: PathBuf,
    /// [`stamp::inputs_hash`] of the inputs the decision saw.
    pub inputs: String,
    /// Why it builds.
    pub reason: String,
}

impl Job {
    /// The job file's text (`key=value` lines).
    pub fn to_text(&self) -> String {
        format!("game={}\ncache={}\nlog={}\ninputs={}\nreason={}\n", self.game.display(), self.cache.display(), self.log.display(), self.inputs, self.reason.replace('\n', " "))
    }

    /// Reads a job file's text.
    pub fn parse(text: &str) -> Option<Job> {
        let get = |k: &str| text.lines().map(|l| l.trim_end_matches('\r')).find_map(|l| l.strip_prefix(k)?.strip_prefix('=')).map(str::to_string);
        Some(Job { game: get("game")?.into(), cache: get("cache")?.into(), log: get("log")?.into(), inputs: get("inputs")?, reason: get("reason").unwrap_or_default() })
    }
}

/// The line sink: the console and the build log (`win::assets`).
pub type Say = Arc<dyn Fn(&str) + Send + Sync>;

/// Runs a job: checks the inputs, empties the cache, runs the whole asset step into it and
/// writes its stamp (or, on failure, the failed marker with the reason). `say` gets every line.
/// `guard`: a failed marker written before the step runs and removed when it succeeds, for a run
/// inside the game, where a crash of the step would end the game (the next start then skips it).
pub fn run_job(job: &Job, say: &Say, guard: Option<&str>) -> Result<Stamp, String> {
    let inputs = stamp::inputs(&job.game);
    let r = run_job_inner(job, &inputs, say, guard);
    if let Err(e) = &r {
        let _ = std::fs::create_dir_all(&job.cache);
        let _ = std::fs::write(job.cache.join(FAILED_FILE), stamp::failed_text(stamp::STEP_HASH, &job.inputs, e));
    }
    r
}

fn run_job_inner(job: &Job, inputs: &[Input], say: &Say, guard: Option<&str>) -> Result<Stamp, String> {
    stamp::inputs_usable(inputs).map_err(|e| format!("{e} in {}", job.game.display()))?;
    if stamp::inputs_hash(inputs) != job.inputs {
        say("note: the game's files changed since the start; building from them as they are now");
    }
    // Only our own folder (its fixed name) is emptied: files of an older build go with it.
    if job.cache.is_dir() {
        std::fs::remove_dir_all(&job.cache).map_err(|e| format!("{}: {e}", job.cache.display()))?;
    }
    std::fs::create_dir_all(&job.cache).map_err(|e| format!("{}: {e}", job.cache.display()))?;
    if let Some(g) = guard {
        let _ = std::fs::write(job.cache.join(FAILED_FILE), stamp::failed_text(stamp::STEP_HASH, &job.inputs, g));
    }
    let mut opt = step::Options::new(job.game.clone(), step::Dest::Root(job.cache.clone()));
    opt.previews = false;
    let to = say.clone();
    step::set_sink(Some(Box::new(move |l: &str| to(l))));
    let r = step::run(&opt);
    step::set_sink(None);
    let report = r?;
    let st = Stamp::of_build(stamp::STEP_HASH, &stamp::inputs_hash(inputs), &job.cache, &report.written);
    let tmp = job.cache.join(format!("{STAMP_FILE}.tmp"));
    std::fs::write(&tmp, st.to_text()).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, job.cache.join(STAMP_FILE)).map_err(|e| format!("{}: {e}", job.cache.display()))?;
    let _ = std::fs::remove_file(job.cache.join(FAILED_FILE));
    Ok(st)
}

/// The text the player sees when a build failed.
pub fn failure_notice(reason: &str, game: &Path, log: &Path) -> String {
    let lacks_kh1 = reason.contains("KH1 Final Mix");
    let mut s = String::from("KH1 Combat in Re:CoM could not build its KH1 files from your games.\n\n");
    if lacks_kh1 {
        s.push_str(&format!("KINGDOM HEARTS FINAL MIX's files were not found in\n{}\\Image\\dt.\nThe mod needs KH1 Final Mix installed in the same KINGDOM HEARTS HD 1.5+2.5 ReMIX folder (Steam).\n\n", game.display()));
    } else {
        s.push_str(&format!("Reason: {reason}\n\n"));
    }
    s.push_str(&format!("The game starts now without the KH1 files (Re:CoM's own models, motions and sounds stay).\nIt tries again when the mod or the game is updated.\n\nDetails: {}", log.display()));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kh1-ab-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn mods_manager_paths() {
        assert_eq!(yaml_value("gameEdition: 2\ninstalledModsPath: 'D:\\Mods'\r\ncompiledModPath: \n", "installedModsPath").as_deref(), Some("D:\\Mods"));
        assert_eq!(yaml_value("installedModsPath: \"C:\\a b\"\n", "installedModsPath").as_deref(), Some("C:\\a b"));
        assert_eq!(yaml_value("  installedModsPath: x\n", "installedModsPath"), None, "top level only");
        assert_eq!(yaml_value("compiledModPath: ~\n", "compiledModPath"), None);
        let d = tmp("yml");
        assert_eq!(installed_mods_dirs(&d), vec![d.join("mods").join("Recom")]);
        std::fs::write(d.join("mods-manager.yml"), "installedModsPath: /x/mods\ncompiledModPath: /x/mod\n").unwrap();
        assert_eq!(installed_mods_dirs(&d), vec![PathBuf::from("/x/mods/Recom"), d.join("mods").join("Recom")]);
        std::fs::write(d.join("mods-manager.yml"), "installedModsPath: /x\ncompiledModPath: /x\n").unwrap();
        assert_eq!(installed_mods_dirs(&d)[0], PathBuf::from("/x/mods/Recom"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_mod_folder_is_found_by_its_dll_then_its_name() {
        let d = tmp("find");
        let mods = d.join("mods").join("Recom");
        let mk = |owner: &str, repo: &str, yml: &str, dll: Option<(&str, usize)>| {
            let p = mods.join(owner).join(repo);
            std::fs::create_dir_all(p.join("openkh")).unwrap();
            std::fs::write(p.join("mod.yml"), yml).unwrap();
            if let Some((at, n)) = dll {
                std::fs::write(p.join(at), vec![0u8; n]).unwrap();
            }
            p
        };
        let other = mk("someone", "other-mod", "assets:\n- name: dll/other.dll\n", Some(("openkh/kh1-recom.dll", 7)));
        assert_eq!(find_mod_folder(&[mods.clone()], Some(7)), None, "another mod's mod.yml");
        let _ = other;
        let named = mk("pm2crazy", "kh1-in-recom", "- name: dll/kh1-recom.dll\n", Some(("openkh/kh1-recom.dll", 5)));
        assert_eq!(find_mod_folder(&[mods.clone()], Some(9)), Some(named.clone()), "by name (case as typed)");
        let fork = mk("fork", "KH1-in-ReCom", "- name: dll/kh1-recom.dll\n  source:\n  - name: kh1-recom.dll\n", Some(("kh1-recom.dll", 9)));
        assert_eq!(find_mod_folder(&[mods.clone()], Some(9)), Some(fork), "the running DLL's own source wins");
        assert_eq!(find_mod_folder(&[d.join("nowhere")], Some(9)), None);
        assert_eq!(cache_dir(Some(&named), &d), named.join("generated"));
        assert_eq!(cache_dir(None, &d), d.join("kh1-recom-generated"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn decide_builds_once_then_places() {
        let d = tmp("decide");
        let cache = d.join("generated");
        let ins = vec![Input { name: "Image/dt/Recom.hed".into(), size: 1, time: 2 }];
        let ih = stamp::inputs_hash(&ins);
        assert_eq!(decide(false, &cache, "s", &ins), Decision::Off);
        assert!(matches!(decide(true, &cache, "s", &ins), Decision::Build(r) if r.contains("not been built")));
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("a.bin"), b"abc").unwrap();
        let st = Stamp { step: "s".into(), inputs: ih.clone(), files: vec![("a.bin".into(), 3)] };
        std::fs::write(cache.join(STAMP_FILE), st.to_text()).unwrap();
        assert_eq!(decide(true, &cache, "s", &ins), Decision::Place(st.clone()));
        assert!(matches!(decide(true, &cache, "t", &ins), Decision::Build(r) if r.contains("mod update")));
        let ins2 = vec![Input { name: "Image/dt/Recom.hed".into(), size: 1, time: 3 }];
        assert!(matches!(decide(true, &cache, "s", &ins2), Decision::Build(r) if r.contains("game update")));
        std::fs::write(cache.join("a.bin"), b"ab").unwrap();
        assert!(matches!(decide(true, &cache, "s", &ins), Decision::Build(r) if r.contains("incomplete")));
        // A failed build of the same step and inputs is not tried again; another one is.
        std::fs::write(cache.join(FAILED_FILE), stamp::failed_text("t", &ih, "KH1 files: none")).unwrap();
        assert_eq!(decide(true, &cache, "t", &ins), Decision::Failed("KH1 files: none".into()));
        assert!(matches!(decide(true, &cache, "u", &ins), Decision::Build(_)));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_job_reads_back_and_fails_cleanly_without_the_games() {
        let d = tmp("job");
        let job = Job { game: d.join("game"), cache: d.join("generated"), log: d.join(BUILD_LOG), inputs: "00".into(), reason: "first\nlaunch".into() };
        let back = Job::parse(&job.to_text()).unwrap();
        assert_eq!(back.reason, "first launch");
        assert_eq!(Job { reason: "first launch".into(), ..job.clone() }, back);
        std::fs::create_dir_all(job.cache.join("old")).unwrap();
        let lines = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let l2 = lines.clone();
        let say: Say = Arc::new(move |l: &str| l2.lock().unwrap().push(l.to_string()));
        let e = run_job(&job, &say, None).unwrap_err();
        assert!(e.contains("KH1 Final Mix") && e.contains("Recom.hed"), "{e}");
        assert!(job.cache.join("old").is_dir(), "nothing is deleted when the inputs are missing");
        let t = std::fs::read_to_string(job.cache.join(FAILED_FILE)).unwrap();
        assert!(Stamp::parse(&t).is_some_and(|s| s.is_for(stamp::STEP_HASH, "00")));
        assert!(failure_notice(&stamp::failed_reason(&t), &job.game, &job.log).contains("KINGDOM HEARTS FINAL MIX's files were not found"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_repo_root_is_an_openkh_mod() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        // OpenKH's GitHub install checks raw.githubusercontent.com/<repo>/<branch>/mod.yml (the root).
        let yml = std::fs::read_to_string(root.join("mod.yml")).unwrap();
        assert!(yml.contains("\ntitle: ") || yml.starts_with("title: "));
        assert!(yml.contains("\ngame: Recom\n"));
        // The DLL goes to <mod_path>/recom/dll/ (Panacea's LoadDLLs), from the build bot's file.
        assert!(yml.contains("- name: dll/kh1-recom.dll\n  method: copy\n  source:\n  - name: openkh/kh1-recom.dll\n"), "{yml}");
        assert!(root.join(DLL_SOURCES[0]).is_file());
        // No game-made file is listed (the DLL places them), and none is tracked.
        assert_eq!(yml.matches("- name:").count(), 2, "one asset, one source");
        let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        for p in ["/generated/", "/PLAYER/", "/ENEMY/", "/raw/", "/kh1-*.json", "/kh1-*.bin", "/kh1-menu-sounds-preview/", "/kh1-recom.dll"] {
            assert!(ignore.lines().any(|l| l == p), "{p} not in .gitignore");
        }
        assert_eq!(CACHE_DIR, "generated");
    }
}
