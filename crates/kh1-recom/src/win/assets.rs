//! The first-launch build on Windows (`crate::asset_build` has the rules;
//! `docs/openkh-github-install.md`).
//!
//! Panacea calls our `OnInit` from inside its own `DllMain` (the loader lock is held), before the
//! game lists Panacea's folder (Checked (source): `dllmain.cpp` → `OpenKH::Initialize` →
//! `Panacea::Initialize` → `LoadDLLs`; the listing is `FakePackageFile`, made in the game's
//! `SetReplacePath`). So:
//!
//! - the build runs **before** `OnInit` returns (a worker thread would race the listing, and
//!   waiting for a thread under the loader lock deadlocks);
//! - it runs in a **helper process**: `rundll32.exe "<this DLL>",BuildAssets <job>`, which runs
//!   [`BuildAssetsW`] (this same DLL: no separate exe). The helper has its own console window
//!   with the step's progress, and its own message box on failure; a crash of the step ends only
//!   the helper. The game waits for it (the wait needs no lock the helper takes).
//! - If the helper cannot start (e.g. blocked), the step runs inside the game, guarded by a failed
//!   marker so a crash is not repeated at the next start.

use super::sys::*;
use super::{log, module_path, START};
use crate::asset_build::{self, Decision, Job, Say};
use kh1_recom_assets::stamp::{self, STEP_HASH};
use std::ffi::c_void;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// The helper's longest run before the game gives up on it.
const HELPER_TIMEOUT_MS: u32 = 30 * 60 * 1000;
/// The exit code of a process whose console window was closed (`STATUS_CONTROL_C_EXIT`).
const CLOSED: u32 = 0xC000_013A;
const TITLE: &str = "KH1 Combat in Re:CoM";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// At start, before anything reads the game-made files: builds them when needed and copies them
/// into Panacea's live folder `live`.
pub(super) fn ensure(game: &Path, live: &Path, dll: &Path, enabled: bool) {
    let t0 = Instant::now();
    // `live` is `<mod_path>/recom`; OpenKH's folder holds `mod_path` (its default layout).
    let openkh = live.parent().and_then(Path::parent);
    let mods = openkh.map(asset_build::installed_mods_dirs).unwrap_or_default();
    let dll_size = std::fs::metadata(dll).ok().map(|m| m.len());
    let mod_folder = asset_build::find_mod_folder(&mods, dll_size);
    let cache = asset_build::cache_dir(mod_folder.as_deref(), game);
    match &mod_folder {
        Some(m) => log(&format!("assets: the mod's own folder {}; built files kept in {}", m.display(), cache.display())),
        None => log(&format!("assets: the mod's own folder not found under {:?}; built files kept in {}", mods, cache.display())),
    }
    let inputs = stamp::inputs(game);
    let mut decision = asset_build::decide(enabled, &cache, STEP_HASH, &inputs);
    if let Decision::Build(why) = &decision {
        log(&format!("assets: building the KH1 files: {why} (step {STEP_HASH}, {} input files)", inputs.len()));
        build(game, &cache, why, &stamp::inputs_hash(&inputs));
        decision = asset_build::decide(enabled, &cache, STEP_HASH, &stamp::inputs(game));
        if let Decision::Build(why) = &decision {
            log(&format!("assets: the build did not finish ({why}); it runs again at the next start. The game runs without the newly built KH1 files."));
            return;
        }
    }
    match decision {
        Decision::Off => log("assets: asset_build = off in the ini: Panacea's folder is left as OpenKH's Build made it"),
        Decision::Failed(why) => log(&format!(
            "assets: not built: {} (this build failed before; it is tried again when the mod or the game changes, or after deleting {}). The game runs without the KH1 files.",
            why.replace('\n', " / "),
            cache.join(stamp::FAILED_FILE).display()
        )),
        Decision::Place(st) => match stamp::place(&cache, live, &st) {
            Ok(p) => log(&format!("assets: {} built files in {}: {} copied ({:.1} MB), {} already there ({:.2} s)", st.files.len(), live.display(), p.copied, p.bytes as f64 / 1e6, p.kept, t0.elapsed().as_secs_f64())),
            Err(e) => log(&format!("assets: could not copy the built files into {}: {e}", live.display())),
        },
        Decision::Build(_) => {}
    }
}

/// Runs the build (the helper, else in the game) and logs its end.
fn build(game: &Path, cache: &Path, why: &str, inputs_hash: &str) {
    let job = Job { game: game.to_path_buf(), cache: cache.to_path_buf(), log: game.join(asset_build::BUILD_LOG), inputs: inputs_hash.to_string(), reason: why.to_string() };
    let job_path = game.join(asset_build::JOB_FILE);
    if let Err(e) = std::fs::write(&job_path, job.to_text()) {
        log(&format!("assets: could not write {}: {e}; building inside the game", job_path.display()));
        in_game(&job);
        return;
    }
    let t0 = Instant::now();
    match helper(&job_path, game) {
        Ok(code) => {
            let secs = t0.elapsed().as_secs_f64();
            match code {
                0 => log(&format!("assets: built in {secs:.0} s (log: {})", job.log.display())),
                1 => log(&format!("assets: the build failed after {secs:.0} s (the reason is in {} and in the message the player saw)", job.log.display())),
                CLOSED => log("assets: the build window was closed before the end; it runs again at the next start"),
                c => {
                    log(&format!("assets: the build stopped (exit code {c:#x}) after {secs:.0} s; see {}", job.log.display()));
                    // Not repeated at every start: the failed marker, unless the helper wrote one.
                    if !cache.join(stamp::FAILED_FILE).is_file() {
                        let _ = std::fs::create_dir_all(cache);
                        let _ = std::fs::write(cache.join(stamp::FAILED_FILE), stamp::failed_text(STEP_HASH, inputs_hash, &format!("the build stopped (exit code {c:#x})")));
                    }
                }
            }
        }
        Err(e) => {
            log(&format!("assets: the build helper did not start ({e}); building inside the game"));
            in_game(&job);
        }
    }
    let _ = std::fs::remove_file(&job_path);
}

/// Starts `rundll32.exe "<dll>",BuildAssets <job>` and waits for it; its exit code.
fn helper(job_path: &Path, game: &Path) -> Result<u32, String> {
    let dll = module_path(super::MODULE.load(std::sync::atomic::Ordering::Relaxed) as *mut c_void);
    let mut sysdir = vec![0u16; 512];
    // SAFETY: the buffer length is passed.
    let n = unsafe { GetSystemDirectoryW(sysdir.as_mut_ptr(), sysdir.len() as u32) } as usize;
    if n == 0 || n >= sysdir.len() {
        return Err("no system folder".into());
    }
    let exe = PathBuf::from(String::from_utf16_lossy(&sysdir[..n])).join("rundll32.exe");
    let app = wide(&exe.to_string_lossy());
    // Panacea's path can mix separators (`mod/recom\dll\…`): backslashes only for rundll32.
    let back = |p: &Path| p.to_string_lossy().replace('/', "\\");
    let mut cmd = wide(&format!("\"{}\" \"{}\",BuildAssets {}", back(&exe), back(&dll), back(job_path)));
    let dir = wide(&game.to_string_lossy());
    let mut si = StartupInfoW { cb: std::mem::size_of::<StartupInfoW>() as u32, ..Default::default() };
    let mut pi = ProcessInformation::default();
    // SAFETY: every pointer is a live, NUL-terminated buffer or a zeroed out-struct of the right size.
    let ok = unsafe { CreateProcessW(app.as_ptr(), cmd.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), 0, 0, std::ptr::null_mut(), dir.as_ptr(), &mut si, &mut pi) };
    if ok == 0 {
        // SAFETY: no arguments.
        return Err(format!("CreateProcess error {}", unsafe { GetLastError() }));
    }
    log(&format!("assets: build helper started (process {})", pi.process_id));
    // SAFETY: `pi`'s handles are ours until closed below.
    unsafe {
        let mut code = 0u32;
        if WaitForSingleObject(pi.process, HELPER_TIMEOUT_MS) != 0 {
            TerminateProcess(pi.process, 2);
            WaitForSingleObject(pi.process, 5000);
            log("assets: the build helper ran 30 minutes and was stopped");
        }
        GetExitCodeProcess(pi.process, &mut code);
        CloseHandle(pi.thread);
        CloseHandle(pi.process);
        Ok(code)
    }
}

/// The build inside the game (the helper could not start): a console while it runs, the reason
/// in the log and the console on failure.
fn in_game(job: &Job) {
    // SAFETY: no arguments; a console the game already has is kept (AllocConsole then fails).
    let own = unsafe { AllocConsole() } != 0;
    let say = console_say(&job.log);
    say(&format!("{TITLE}: building the KH1 files from your games ({}). The game starts when it is done.", job.reason));
    let r = asset_build::run_job(job, &say, Some("the build stopped the game the last time (it ran inside the game)"));
    match &r {
        Ok(st) => say(&format!("done: {} files", st.files.len())),
        Err(e) => {
            say(&asset_build::failure_notice(e, &job.game, &job.log));
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }
    if own {
        // SAFETY: the console we made.
        unsafe { FreeConsole() };
    }
}

/// Lines to the console window (when there is one) and the build log.
fn console_say(log_path: &Path) -> Say {
    // SAFETY: plain calls; the code page makes the console show UTF-8 as written.
    unsafe {
        SetConsoleOutputCP(65001);
        SetConsoleTitleW(wide(&format!("{TITLE}: building the KH1 files")).as_ptr());
    }
    let con = std::fs::OpenOptions::new().write(true).open("CONOUT$").ok();
    let file = std::fs::OpenOptions::new().create(true).append(true).open(log_path).ok();
    let out = Mutex::new((con, file));
    Arc::new(move |l: &str| {
        if let Ok(mut g) = out.lock() {
            if let Some(c) = g.0.as_mut() {
                let _ = writeln!(c, "{l}");
            }
            if let Some(f) = g.1.as_mut() {
                let _ = writeln!(f, "{l}");
            }
        }
    })
}

/// The helper process's entry (`rundll32.exe "<dll>",BuildAssets <job file>`; rundll32 calls the
/// `W` export with the rest of the command line). Builds, then ends the process: 0 built, 1 failed
/// (the failed marker written, the player told).
///
/// # Safety
/// Called by rundll32 with a NUL-terminated wide command line (or null).
#[no_mangle]
pub unsafe extern "system" fn BuildAssetsW(_hwnd: *mut c_void, _inst: *mut c_void, cmd: *const u16, _show: i32) {
    // Never the game's start-up in this process (DllMain's fallback thread).
    START.call_once(|| {});
    let line = super::wide_to_path(cmd).map(|p| p.to_string_lossy().trim().trim_matches('"').to_string()).unwrap_or_default();
    let job = std::fs::read_to_string(&line).ok().and_then(|t| Job::parse(&t));
    let Some(job) = job else {
        // SAFETY: plain call.
        unsafe { ExitProcess(3) };
    };
    // SAFETY: plain call; the helper is a GUI process with no console of its own.
    unsafe { AllocConsole() };
    let _ = std::fs::write(&job.log, "");
    let say = console_say(&job.log);
    say(&format!("{TITLE}: building the KH1 files from your own KH1 and Re:CoM files."));
    say(&format!("Why: {}. This takes a minute or two; the game starts by itself when it is done.", job.reason));
    say("Please don't close this window.");
    say("");
    let t0 = Instant::now();
    let r = asset_build::run_job(&job, &say, None);
    let code = match &r {
        Ok(st) => {
            say("");
            say(&format!("Done: {} files in {:.0} s. The game starts now.", st.files.len(), t0.elapsed().as_secs_f64()));
            std::thread::sleep(std::time::Duration::from_secs(2));
            0
        }
        Err(e) => {
            say(&format!("FAILED: {e}"));
            let text = wide(&asset_build::failure_notice(e, &job.game, &job.log));
            // SAFETY: NUL-terminated strings; a plain modal box in the helper (no loader lock here).
            unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), wide(TITLE).as_ptr(), MB_OK | MB_ICONWARNING | MB_TOPMOST | MB_SETFOREGROUND) };
            1
        }
    };
    // SAFETY: plain call; ends the helper.
    unsafe { ExitProcess(code) };
}
