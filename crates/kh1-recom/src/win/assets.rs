//! The first-launch build on Windows (`crate::asset_build` has the rules;
//! `docs/openkh-github-install.md` § 8), with its progress in the game window.
//!
//! Panacea calls our `OnInit` from inside its own `DllMain` (the loader lock is held), lists
//! `<mod_path>/recom` once when the game sets its replace path (after every `OnInit`), and reads a
//! listed file's content from disk at each open (Checked (source): `Panacea.cpp`
//! `FakePackageFile`, `LoadFile*`, `GetFileSize`). Re:CoM's loader takes a file's size through the
//! hooked `GetFileSize` and then loads it with the hooked `LoadFileWithSize` (`0x1402427D0`), so a
//! listed name's size at the listing is never used (Checked (decomp)). So:
//!
//! - **At `OnInit`** ([`ensure`]): when a build is needed, the names `Recom.hed` lacks that the build
//!   may write are made as empty files (so the listing has them), a small thread starts the build
//!   (no process is made under the loader lock), and Re:CoM's title is hooked. Everything else of the
//!   DLL's start that reads the built files waits (`super::install_deferred`).
//! - **The build** runs in a helper process, `rundll32.exe "<this DLL>",BuildAssets <job>`
//!   ([`BuildAssetsW`], this same DLL: no separate exe), in a job object that ends it with the game. It
//!   writes its log, copies the built files into Panacea's folder and reports its progress in
//!   `kh1-recom-build.progress` ([`asset_build::Shown`]). A crash of the step ends only the helper.
//!   If the helper cannot start, the step runs on that thread inside the game, guarded by a failed
//!   marker so a crash is not repeated at the next start.
//! - **At the title** ([`title_wait`], a wrapper on `CTitle`'s state-1 update): while the build runs
//!   the title waits in its "press" sub-state (Re:CoM's update is not run, so no press is taken) and a
//!   line of Re:CoM's own text, made as the title's own alert text is made, shows the progress. When
//!   the build ends, on the game's thread: the DLL's deferred start runs (it now finds the files),
//!   "Done" or the failure line shows for a moment, then the title goes on. No game file of ours is
//!   opened before (Checked (exe): of the groups that load our outputs, FORM 2 comes with the world
//!   groups (category 14), FORM 100 / 109 and EFFECT 1000 with the camp, the shop and the player; the
//!   title's own groups (0, 0xD / 0xE) link FORM 505 / 507 / 111 only).

use super::movement::{func, orig, rd, wr};
use super::sys::*;
use super::{base, hook, log, module_path, START};
use crate::asset_build::{self, title, Decision, Job, Say, Shown};
use kh1_recom_assets::stamp::{self, Stamp, STEP_HASH};
use std::ffi::c_void;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The helper's longest run before the game gives up on it.
const HELPER_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const TITLE: &str = "KH1 Combat in Re:CoM";
/// How long the end line shows before the title goes on.
const DONE_FOR: Duration = Duration::from_millis(2500);
const FAILED_FOR: Duration = Duration::from_secs(7);
/// The progress file is read every this many title frames.
const POLL_FRAMES: u32 = 6;

// Re:CoM functions (RVAs; first bytes checked in `sites`, as the command menu's).
const NEW: u32 = 0x004A_CE54;
const SEQ_CTOR: u32 = 0x0024_7260;
const LAYOUT_POS: u32 = 0x0024_8660;
const FNT_CTOR: u32 = 0x0024_3F10;
const TEXT_COPY: u32 = 0x0025_8DE0;
const SEQ_CTRL_VTABLE: u32 = 0x0060_7340;
/// `C2DSeqCtrl`: its draw priority (the camp's 0xC draws over the field; Assumption: over the title).
const C_PRIORITY: usize = 0x16E;
const PRIORITY: u8 = 0xC;
/// The title's alert text (`0x1403A23D0`): a `C2DFnt` of size 0xF (0x10 when the language global's
/// `+0x274` is set), centred (flags 2), Re:CoM's default colour, attached to `aleart.lad` set 0
/// (`C2DSeqCtrl` `+0x30` names it `aleart:00`, format `%s:%02d` at `0x140606EC8`). Checked (decomp +
/// disasm). The sets tried, in order (Assumption: `aleart` is loaded with the title's FORM 111).
const SETS: [&[u8]; 2] = [b"aleart:00\0", b"title:00\0"];
const FONT_SIZE: i32 = 0xF;
const FONT_CENTRED: i32 = 2;
/// Where the lines go (Re:CoM's 2D space, 512 x 384, the layout position's origin at its centre:
/// `0x140248660`), the second line below the first, and the width they are squeezed to.
/// Assumption: below the title's logo, in the "press" text's place.
const AT: (f32, f32) = (256.0, 330.0);
const LINE: f32 = 20.0;
const BOX: f32 = 480.0;

type TitleFn = unsafe extern "win64" fn(usize);
type NewFn = unsafe extern "win64" fn(usize) -> usize;
type CtorFn = unsafe extern "win64" fn(usize) -> usize;
type VecFn = unsafe extern "win64" fn(usize, *const f32);
type FntCtorFn = unsafe extern "win64" fn(usize, i32, i32, *const u32, i32, *const u8, i32) -> usize;
type TextFn = unsafe extern "win64" fn(usize, *const u8);
type DeleteFn = unsafe extern "win64" fn(usize, u32) -> usize;

#[repr(C, align(16))]
struct V4([f32; 4]);

/// The title waits (set at `OnInit` when a build starts; cleared when the end line has shown).
static HOLD: AtomicBool = AtomicBool::new(false);
static ORIG_TITLE: AtomicUsize = AtomicUsize::new(0);
static BUILD: Mutex<Option<Build>> = Mutex::new(None);
static SCREEN: Mutex<Screen> = Mutex::new(Screen { ctrl: 0, list: 0, fnt: [0; 2], text: (String::new(), String::new()), frames: 0, shown: None, end_at: None, failed_logged: false });

/// What ensure decided for the rest of the start.
pub(super) enum Ready {
    /// Go on now (nothing to build, built, or the title could not be hooked).
    Now,
    /// The rest waits for the build, at the title (`super::install_deferred`).
    AtTitle,
}

/// The build in flight.
struct Build {
    job: Job,
    job_path: PathBuf,
    /// The empty files made before Panacea's listing.
    placeholders: Vec<PathBuf>,
    runner: Runner,
    t0: Instant,
}

enum Runner {
    /// The starter thread has not started anything yet.
    Starting,
    /// The helper process (its handle).
    Helper(usize),
    /// The step on the starter thread: its end and exit-code-like result.
    InGame(Arc<(AtomicBool, AtomicU32)>),
}

/// Our text on the title (main thread only).
struct Screen {
    ctrl: usize,
    list: i32,
    fnt: [usize; 2],
    text: (String, String),
    frames: u32,
    shown: Option<Shown>,
    end_at: Option<Instant>,
    failed_logged: bool,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// At start, before anything reads the game-made files: decides; places a finished build (as
/// 0.141.0), or makes the new names, starts the build and hooks the title ([`Ready::AtTitle`]).
pub(super) fn ensure(game: &Path, live: &Path, dll: &Path, enabled: bool) -> Ready {
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
    let decision = asset_build::decide(enabled, &cache, STEP_HASH, &inputs);
    match decision {
        Decision::Off => log("assets: asset_build = off in the ini: Panacea's folder is left as OpenKH's Build made it"),
        Decision::Failed(why) => log(&format!(
            "assets: not built: {} (this build failed before; it is tried again when the mod or the game changes, or after deleting {}). The game runs without the KH1 files.",
            why.replace('\n', " / "),
            cache.join(stamp::FAILED_FILE).display()
        )),
        Decision::Place(st) => place(&cache, live, &st, t0),
        Decision::Build(why) => {
            log(&format!("assets: building the KH1 files: {why} (step {STEP_HASH}, {} input files)", inputs.len()));
            let job = Job {
                game: game.to_path_buf(),
                cache: cache.clone(),
                log: game.join(asset_build::BUILD_LOG),
                inputs: stamp::inputs_hash(&inputs),
                reason: why,
                live: live.to_path_buf(),
                progress: game.join(asset_build::PROGRESS_FILE),
            };
            return start(job, live);
        }
    }
    Ready::Now
}

fn place(cache: &Path, live: &Path, st: &Stamp, t0: Instant) {
    match stamp::place(cache, live, st) {
        Ok(p) => log(&format!("assets: {} built files in {}: {} copied ({:.1} MB), {} already there ({:.2} s)", st.files.len(), live.display(), p.copied, p.bytes as f64 / 1e6, p.kept, t0.elapsed().as_secs_f64())),
        Err(e) => log(&format!("assets: could not copy the built files into {}: {e}", live.display())),
    }
}

/// Makes the new names, hooks the title and starts the build on a thread of its own.
fn start(job: Job, live: &Path) -> Ready {
    let placeholders = make_placeholders(&job.game, &job.cache, live);
    let _ = std::fs::remove_file(&job.progress);
    let job_path = job.game.join(asset_build::JOB_FILE);
    let hooks = [hook::Hook { site: &crate::sites::TITLE_HOLD_HOOK[0], ours: title_wait as *const () as usize, original: &ORIG_TITLE }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    let hooked = crate::sites::TITLE_HOLD_CHECKED.iter().all(|s| unsafe { std::slice::from_raw_parts((base() + s.rva as usize) as *const u8, s.expect.len()) } == s.expect) && unsafe { hook::install_all(base(), &hooks) };
    if !hooked {
        // Without the title's hold, the build runs before the game starts, as 0.141.0 did (without
        // its console: nothing shows until the game does).
        log("assets: Re:CoM's title could not be hooked: the build runs now and the game starts when it is done (no progress screen)");
        let t0 = Instant::now();
        let code = blocking(&job, &job_path);
        let _ = std::fs::remove_file(&job_path);
        end_log(&job, code, t0);
        if let Decision::Place(st) = asset_build::decide(true, &job.cache, STEP_HASH, &stamp::inputs(&job.game)) {
            place(&job.cache, live, &st, Instant::now());
        }
        remove_empty(&placeholders);
        return Ready::Now;
    }
    if let Ok(mut g) = BUILD.lock() {
        *g = Some(Build { job: job.clone(), job_path: job_path.clone(), placeholders, runner: Runner::Starting, t0: Instant::now() });
    }
    HOLD.store(true, Ordering::Release);
    // A thread made under the loader lock runs once `OnInit` returns: no process is made, and
    // nothing waited for, under the lock.
    std::thread::spawn(move || begin(job, job_path));
    log("assets: the build starts on its own thread; the title waits for it and shows its progress (Re:CoM's text)");
    Ready::AtTitle
}

/// The empty files Panacea's listing must see (`asset_build::placeholders`).
fn make_placeholders(game: &Path, cache: &Path, live: &Path) -> Vec<PathBuf> {
    let expected = kh1_recom_assets::keyblade::new_resource_names();
    let previous: Vec<String> = std::fs::read_to_string(cache.join(stamp::STAMP_FILE)).ok().and_then(|t| Stamp::parse(&t)).map(|s| s.files.into_iter().map(|f| f.0).collect()).unwrap_or_default();
    let arc = kh1_recom_assets::install::recom_archive(game).ok();
    let in_recom = |n: &str| match &arc {
        Some(a) => a.find(n).is_some(),
        // Without Recom.hed only the step's own new names count as new.
        None => !expected.iter().any(|e| asset_build::game_name(e) == Some(n)),
    };
    let names = asset_build::placeholders(&expected, &previous, in_recom, |rel| stamp::under(live, rel).exists());
    let mut made = Vec::new();
    for rel in &names {
        let p = stamp::under(live, rel);
        let ok = p.parent().is_some_and(|d| std::fs::create_dir_all(d).is_ok()) && std::fs::File::create(&p).is_ok();
        if ok {
            made.push(p);
        } else {
            log(&format!("assets: could not make {} for Panacea's listing (its file is used from the next start)", p.display()));
        }
    }
    log(&format!("assets: {} new file names made empty in {} for Panacea's listing (filled in by the build){}", made.len(), live.display(), if arc.is_none() { "; Recom.hed unreadable: only the Keyblade groups' names" } else { "" }));
    made
}

/// Removes the placeholders the build did not fill (a failed build).
fn remove_empty(placeholders: &[PathBuf]) {
    let mut n = 0;
    for p in placeholders {
        if std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.len() == 0) && std::fs::remove_file(p).is_ok() {
            n += 1;
        }
    }
    if n > 0 {
        log(&format!("assets: {n} empty file(s) not filled by the build removed from Panacea's folder"));
    }
}

/// The starter thread: the helper, else the step here.
fn begin(job: Job, job_path: PathBuf) {
    let runner = match std::fs::write(&job_path, job.to_text()).map_err(|e| format!("could not write {}: {e}", job_path.display())).and_then(|()| helper(&job_path, &job.game)) {
        Ok(h) => Runner::Helper(h as usize),
        Err(e) => {
            log(&format!("assets: the build helper did not start ({e}); building inside the game"));
            let flag = Arc::new((AtomicBool::new(false), AtomicU32::new(0)));
            if let Ok(mut g) = BUILD.lock() {
                if let Some(b) = g.as_mut() {
                    b.runner = Runner::InGame(flag.clone());
                }
            }
            let code = in_game(&job);
            flag.1.store(code, Ordering::Release);
            flag.0.store(true, Ordering::Release);
            return;
        }
    };
    if let Ok(mut g) = BUILD.lock() {
        if let Some(b) = g.as_mut() {
            b.runner = runner;
        }
    }
}

/// The build before the game starts (only when the title could not be hooked): the helper and its
/// end, else the step here. Its exit code.
fn blocking(job: &Job, job_path: &Path) -> u32 {
    let started = std::fs::write(job_path, job.to_text()).map_err(|e| e.to_string()).and_then(|()| helper(job_path, &job.game));
    match started {
        Ok(h) => {
            // SAFETY: our process handle; closed after the wait.
            unsafe {
                let mut code = 0u32;
                if WaitForSingleObject(h, HELPER_TIMEOUT.as_millis() as u32) != 0 {
                    TerminateProcess(h, 2);
                    WaitForSingleObject(h, 5000);
                    log("assets: the build helper ran 30 minutes and was stopped");
                }
                GetExitCodeProcess(h, &mut code);
                CloseHandle(h);
                code
            }
        }
        Err(e) => {
            log(&format!("assets: the build helper did not start ({e}); building inside the game"));
            in_game(job)
        }
    }
}

/// The game's job object: its processes end when the game does (its handle is never closed).
fn game_job() -> Option<usize> {
    static JOB: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let j = *JOB.get_or_init(|| {
        // SAFETY: plain calls; the limits buffer has the struct's size and alignment.
        unsafe {
            let j = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
            if j.is_null() {
                return 0;
            }
            let mut info = [0u64; JOB_LIMITS_LEN / 8];
            let flags = (info.as_mut_ptr() as *mut u8).add(JOB_LIMIT_FLAGS_AT) as *mut u32;
            *flags = JOB_KILL_ON_CLOSE;
            if SetInformationJobObject(j, JOB_EXTENDED_LIMITS, info.as_ptr() as *const c_void, JOB_LIMITS_LEN as u32) == 0 {
                CloseHandle(j);
                return 0;
            }
            j as usize
        }
    });
    (j != 0).then_some(j)
}

/// Starts `rundll32.exe "<dll>",BuildAssets <job>` (in the game's job, so it ends with the game);
/// its process handle.
fn helper(job_path: &Path, game: &Path) -> Result<*mut c_void, String> {
    let dll = module_path(super::MODULE.load(Ordering::Relaxed) as *mut c_void);
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
    let ok = unsafe { CreateProcessW(app.as_ptr(), cmd.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), 0, CREATE_SUSPENDED, std::ptr::null_mut(), dir.as_ptr(), &mut si, &mut pi) };
    if ok == 0 {
        // SAFETY: no arguments.
        return Err(format!("CreateProcess error {}", unsafe { GetLastError() }));
    }
    // SAFETY: `pi`'s handles are ours; the thread handle is closed once it runs.
    unsafe {
        let in_job = game_job().is_some_and(|j| AssignProcessToJobObject(j as *mut c_void, pi.process) != 0);
        ResumeThread(pi.thread);
        CloseHandle(pi.thread);
        log(&format!("assets: build helper started (process {}{})", pi.process_id, if in_job { ", ends with the game" } else { "; not in the game's job: it finishes on its own if the game is closed" }));
    }
    Ok(pi.process)
}

/// The build inside the game (the helper could not start): the log file and the progress file, a
/// failed marker while it runs. Its exit-code-like result (0 built, 1 failed).
fn in_game(job: &Job) -> u32 {
    let say = file_say(&job.log, false);
    say(&format!("{TITLE}: building the KH1 files from your games ({}), inside the game.", job.reason));
    let w = progress_writer(job.progress.clone());
    let t0 = Instant::now();
    let w2 = w.clone();
    let r = asset_build::run_job_with(job, &say, Some("the build stopped the game the last time (it ran inside the game)"), Box::new(move |s| w2(&s)));
    finish_job(job, r, &say, &w, t0)
}

/// After the step (the helper or the thread): the copy into Panacea's folder and the end record.
fn finish_job(job: &Job, r: Result<Stamp, String>, say: &Say, w: &Arc<dyn Fn(&Shown) + Send + Sync>, t0: Instant) -> u32 {
    match r {
        Ok(st) => match asset_build::place_job(job, &st, &|s| w(&s)) {
            Ok(p) => {
                let secs = t0.elapsed().as_secs() as u32;
                say(&format!("Done: {} files in {secs} s ({} copied into {}).", st.files.len(), p.copied, job.live.display()));
                w(&Shown::Done { files: st.files.len(), secs });
                0
            }
            Err(e) => {
                say(&format!("Built, but {e}"));
                w(&Shown::Failed { reason: e });
                1
            }
        },
        Err(e) => {
            say(&format!("FAILED: {e}"));
            say(&asset_build::failure_notice(&e, &job.game, &job.log));
            w(&Shown::Failed { reason: e });
            1
        }
    }
}

/// Writes each new progress record (a new stage or percentage) to `path` whole (a temporary file,
/// then a rename), for the game to read.
fn progress_writer(path: PathBuf) -> Arc<dyn Fn(&Shown) + Send + Sync> {
    let last = Mutex::new(String::new());
    Arc::new(move |s: &Shown| {
        if path.as_os_str().is_empty() {
            return;
        }
        let text = s.to_text();
        let Ok(mut l) = last.lock() else { return };
        if *l == text {
            return;
        }
        let tmp = path.with_extension("progress.tmp");
        if std::fs::write(&tmp, &text).is_ok() && std::fs::rename(&tmp, &path).is_ok() {
            *l = text;
        }
    })
}

/// Lines to the build log (and stdout in the helper, which has none shown).
fn file_say(log_path: &Path, truncate: bool) -> Say {
    let mut o = std::fs::OpenOptions::new();
    o.create(true);
    if truncate {
        o.write(true).truncate(true);
    } else {
        o.append(true);
    }
    let out = Mutex::new(o.open(log_path).ok());
    Arc::new(move |l: &str| {
        if let Ok(mut g) = out.lock() {
            if let Some(f) = g.as_mut() {
                let _ = writeln!(f, "{l}");
            }
        }
    })
}

/// The game log's line for a build's end (as 0.141.0), and a failed marker for a crashed helper.
fn end_log(job: &Job, code: u32, t0: Instant) {
    let secs = t0.elapsed().as_secs_f64();
    match code {
        0 => log(&format!("assets: built in {secs:.0} s (log: {})", job.log.display())),
        1 => log(&format!("assets: the build failed after {secs:.0} s (the reason is in {} and on the title)", job.log.display())),
        c => {
            log(&format!("assets: the build stopped (exit code {c:#x}) after {secs:.0} s; see {}", job.log.display()));
            // Not repeated at every start: the failed marker, unless the helper wrote one.
            if !job.cache.join(stamp::FAILED_FILE).is_file() {
                let _ = std::fs::create_dir_all(&job.cache);
                let _ = std::fs::write(job.cache.join(stamp::FAILED_FILE), stamp::failed_text(STEP_HASH, &job.inputs, &format!("the build stopped (exit code {c:#x})")));
            }
        }
    }
}

/// The wrapper on `CTitle`'s state-1 update (`0x1403A3820`): Re:CoM's update, except while the
/// build holds the title in its "press" sub-state (10): then ours ([`hold_frame`]) instead.
unsafe extern "win64" fn title_wait(this: usize) {
    let f: TitleFn = orig(&ORIG_TITLE);
    if !HOLD.load(Ordering::Acquire) || rd::<i32>(this + title::SUB) != title::WAIT || !hold_frame() {
        f(this);
    }
}

/// One held title frame (the game's thread): polls the build, shows its line; false once the end
/// line has shown (the title goes on).
unsafe fn hold_frame() -> bool {
    let Ok(mut s) = SCREEN.lock() else { return false };
    s.frames = s.frames.wrapping_add(1);
    if s.end_at.is_none() && (s.frames == 1 || s.frames % POLL_FRAMES == 0) {
        poll(&mut s);
    }
    if s.end_at.is_some_and(|t| Instant::now() >= t) {
        if s.ctrl != 0 {
            super::d2::delete(s.ctrl, s.list);
            s.ctrl = 0;
            s.fnt = [0; 2];
        }
        HOLD.store(false, Ordering::Release);
        log("assets: the title goes on");
        return false;
    }
    let shown = s.shown.clone().unwrap_or_else(Shown::starting);
    let lines = shown.lines();
    if s.ctrl == 0 && !s.failed_logged {
        make_screen(&mut s);
    }
    if s.ctrl != 0 && s.text != lines {
        for (i, t) in [&lines.0, &lines.1].into_iter().enumerate() {
            let f = s.fnt[i];
            if f != 0 {
                let z: Vec<u8> = t.bytes().chain(std::iter::once(0)).collect();
                func::<TextFn>(TEXT_COPY)(f, z.as_ptr());
                super::text_fit::fit(f, BOX, || format!("the build's progress line {i}"));
            }
        }
        s.text = lines;
    }
    true
}

/// Reads the build's progress; at its end, finishes it (once).
unsafe fn poll(s: &mut Screen) {
    let Ok(mut g) = BUILD.lock() else { return };
    let Some(b) = g.as_mut() else { return };
    if let Some(p) = std::fs::read_to_string(&b.job.progress).ok().and_then(|t| Shown::parse(&t)) {
        s.shown = Some(p);
    }
    let code = match &b.runner {
        Runner::Starting => None,
        Runner::Helper(h) => {
            let h = *h as *mut c_void;
            if WaitForSingleObject(h, 0) == 0 {
                let mut c = 0u32;
                GetExitCodeProcess(h, &mut c);
                CloseHandle(h);
                Some(c)
            } else if b.t0.elapsed() > HELPER_TIMEOUT {
                TerminateProcess(h, 2);
                WaitForSingleObject(h, 5000);
                CloseHandle(h);
                log("assets: the build helper ran 30 minutes and was stopped");
                Some(2)
            } else {
                None
            }
        }
        Runner::InGame(f) => f.0.load(Ordering::Acquire).then(|| f.1.load(Ordering::Acquire)),
    };
    let Some(code) = code else { return };
    let b = g.take().expect("there");
    drop(g);
    s.shown = Some(finish(&b, code, s.shown.take()));
    s.end_at = Some(Instant::now() + if matches!(s.shown, Some(Shown::Done { .. })) { DONE_FOR } else { FAILED_FOR });
}

/// The build's end on the game's thread: the log, the files placed (when built), the empty names
/// left removed, then the DLL's deferred start. What the title shows.
fn finish(b: &Build, code: u32, last: Option<Shown>) -> Shown {
    let _ = std::fs::remove_file(&b.job_path);
    end_log(&b.job, code, b.t0);
    let t = Instant::now();
    let shown = match asset_build::decide(true, &b.job.cache, STEP_HASH, &stamp::inputs(&b.job.game)) {
        // Built: the helper copied the files; this only checks them (and copies what it could not).
        Decision::Place(st) => match stamp::place(&b.job.cache, &b.job.live, &st) {
            Ok(p) => {
                log(&format!("assets: {} built files in {}: {} copied now ({:.1} MB), {} already there ({:.2} s)", st.files.len(), b.job.live.display(), p.copied, p.bytes as f64 / 1e6, p.kept, t.elapsed().as_secs_f64()));
                match last {
                    Some(d @ Shown::Done { .. }) => d,
                    _ => Shown::Done { files: st.files.len(), secs: b.t0.elapsed().as_secs() as u32 },
                }
            }
            Err(e) => {
                log(&format!("assets: could not copy the built files into {}: {e}", b.job.live.display()));
                Shown::Failed { reason: format!("could not copy the built files: {e}") }
            }
        },
        Decision::Failed(why) => Shown::Failed { reason: why },
        _ => match last {
            Some(f @ Shown::Failed { .. }) => f,
            _ => {
                log("assets: the build did not finish; it runs again at the next start. The game runs without the newly built KH1 files.");
                Shown::Failed { reason: format!("the build stopped (exit code {code:#x})") }
            }
        },
    };
    remove_empty(&b.placeholders);
    log(&format!("assets: the title shows: {:?}", shown.lines()));
    // The rest of the DLL's start (it reads the built files), on the game's thread at the title.
    super::install_deferred();
    shown
}

/// Our two lines on the title: a `C2DSeqCtrl` with two `C2DFnt` text layers, made as the title's
/// alert text is (`0x1403A23D0`).
unsafe fn make_screen(s: &mut Screen) {
    let c = func::<NewFn>(NEW)(0x170);
    if c == 0 {
        return;
    }
    func::<CtorFn>(SEQ_CTOR)(c);
    if rd::<usize>(c) != base() + SEQ_CTRL_VTABLE as usize {
        s.failed_logged = true;
        log("assets: Re:CoM's 2D control is not as expected: no progress line (the title still waits)");
        return;
    }
    super::d2::made();
    wr::<u8>(c + C_PRIORITY, PRIORITY);
    func::<VecFn>(LAYOUT_POS)(c, V4([AT.0, AT.1, 0.0, 1.0]).0.as_ptr());
    let list = super::d2::current_list();
    let global: usize = rd(super::va(base(), crate::sites::data::FONT_GLOBAL));
    let size = FONT_SIZE + (global != 0 && rd::<i32>(global + 0x274) != 0) as i32;
    let mut used = None;
    for i in 0..2 {
        let mem = func::<NewFn>(NEW)(0xB0);
        if mem == 0 {
            continue;
        }
        let f = func::<FntCtorFn>(FNT_CTOR)(mem, 0, 0, std::ptr::null(), size, std::ptr::null(), FONT_CENTRED);
        func::<TextFn>(TEXT_COPY)(f, b" \0".as_ptr());
        let sets: Vec<&[u8]> = match used {
            Some(k) => vec![SETS[k]],
            None => SETS.to_vec(),
        };
        let mut on = None;
        for set in sets {
            if super::d2::attach(c, list, set.as_ptr(), f, 0.0, LINE * i as f32, 0.0) != 0 {
                on = SETS.iter().position(|x| *x == set);
                break;
            }
        }
        match on {
            Some(k) => {
                used = Some(k);
                s.fnt[i] = f;
            }
            // Not attached: the font is still ours.
            None => {
                let del: DeleteFn = std::mem::transmute_copy(&rd::<usize>(rd::<usize>(f)));
                del(f, 1);
            }
        }
    }
    s.ctrl = c;
    s.list = list;
    s.text = (String::new(), String::new());
    match used {
        Some(k) => log(&format!(
            "assets: the progress lines are Re:CoM's text on the title (set {}, size {size:#x}, draw list {list})",
            String::from_utf8_lossy(&SETS[k][..SETS[k].len() - 1])
        )),
        None => {
            s.failed_logged = true;
            super::d2::delete(c, list);
            s.ctrl = 0;
            log("assets: no Re:CoM layout to put the progress lines on (aleart / title not loaded?): the title waits without them");
        }
    }
}

/// The helper process's entry (`rundll32.exe "<dll>",BuildAssets <job file>`; rundll32 calls the
/// `W` export with the rest of the command line). Builds, copies the files into Panacea's folder,
/// writes its progress for the game, then ends the process: 0 built, 1 failed (the failed marker
/// written). No window of its own: the game shows its progress.
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
    let say = file_say(&job.log, true);
    say(&format!("{TITLE}: building the KH1 files from your own KH1 and Re:CoM files."));
    say(&format!("Why: {}. The game's title shows the progress and waits until it is done.", job.reason));
    say("");
    let w = progress_writer(job.progress.clone());
    w(&Shown::starting());
    let t0 = Instant::now();
    let w2 = w.clone();
    let r = asset_build::run_job_with(&job, &say, None, Box::new(move |s| w2(&s)));
    let code = finish_job(&job, r, &say, &w, t0);
    // SAFETY: plain call; ends the helper.
    unsafe { ExitProcess(code) };
}
