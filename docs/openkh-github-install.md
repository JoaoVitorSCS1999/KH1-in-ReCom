# Installing from GitHub through OpenKH, and the first-launch build

Owner, 2026-11-05, verbatim: "I also want people to be able to install the mod from openKH by using
the GitHub". Answers (`design-philosophy-instructions.md`, last entry): players install with OpenKH
Mods Manager's GitHub install (`PM2CRAZY/KH1-in-ReCom`) from the default branch `testing`; the DLL
builds the game-made files on first launch from the player's own KH1 and Re:CoM files, and again
after an update changes the asset step.

Sources read: OpenKH's own source, `github.com/OpenKH/OpenKh` at `b67a743` (2026-10-22), cloned
outside the repo: `OpenKh.Tools.ModsManager` (Services `ModsService`, `RepositoryService`,
`ConfigurationService`, `OpenkhInstallation`; ViewModels `MainViewModel`), `OpenKh.Patcher`
(`PatcherProcessor`, `Metadata`) and `OpenKh.Research.Panacea` (`dllmain.cpp`, `OpenKH.cpp`,
`Panacea.cpp`). Tags: Checked (source / GitHub API / code) or Assumption. No OpenKH binary was run
here and nothing was tried in game.

## 1. How OpenKH's GitHub install works

- **Which branch.** "Install a new mod" with `owner/repo` (`ModsService.InstallModFromGit`): with no
  branch given it first asks `raw.githubusercontent.com/<repo>/main/mod.yml`; when that is not
  there it asks the GitHub API for the repository's `default_branch` and checks
  `<default_branch>/mod.yml`. A third path part picks a branch (`owner/repo/branch`, or the
  dialog's branch field). Checked (source).
  - `PM2CRAZY/KH1-in-ReCom`: public, default branch `testing`, the only branch; `main/mod.yml` and
    (before this change) `testing/mod.yml` both answered 404. Checked (GitHub API, 2026-11-05).
    So the install takes `testing`. Typing `PM2CRAZY/KH1-in-ReCom/testing` pins it even if a
    `main` branch ever appears.
  - **`mod.yml` must be at the repository root** (the check is `<branch>/mod.yml`). Checked (source).
- **Where the clone goes.** `GetModPath(name)` = `InstalledModsPath/<owner>/<repo>`, and
  `InstalledModsPath` = `<installedModsPath or the OpenKH folder>/mods/<game selected in the Mods
  Manager>`. So with Re:CoM selected: `<openkh>/mods/Recom/PM2CRAZY/KH1-in-ReCom` (the name as
  typed). If a mod of that name exists, OpenKH asks to overwrite and then **deletes the old folder**
  first (`CleanModFiles`). Checked (source). The game used is the one selected **at install
  time**: `mod.yml`'s `game:` does not move it (only `MoveFromCollection` reads it). Checked
  (source). So players must pick Re:CoM in the Mods Manager before installing.
- **How the clone is made.** LibGit2Sharp `Repository.Clone`, the whole history (no depth), the one
  branch, submodules on. Checked (source). The repository is 76 MB (GitHub API `size` 76,475 KB,
  2026-11-05), mostly the build bot's DLL commits; it grows with each version.
- **How update works.** "Check for updates" (`FetchUpdates`): `git fetch`, counts commits behind
  the tracked branch. "Update" (`RepositoryService.FetchAndResetUponOrigin`): `git fetch`, then a
  **hard reset** to the tracked branch's tip with forced checkout. Checked (source). A hard reset
  rewrites tracked files and leaves untracked and ignored files alone (libgit2 removes them only
  with `REMOVE_UNTRACKED` / `REMOVE_IGNORED`, which are not set). Assumption (libgit2's rules; not
  run here). Nothing is built on install or update: no script hook exists. Checked (source).
- **`mod.yml` fields** (`OpenKh.Patcher.Metadata`, camelCase YAML): `title`, `originalAuthor`,
  `description`, optional `game`, `dependencies`, `isCollection`, `collectionGames`, `assets`. An
  optional `icon.png` (128 × 128) and `preview.png` (≤ 512 × 288) at the root are shown in the
  list. Checked (source, `docs/tool/GUI.ModsManager/creatingMods.md`). For Re:CoM: `game: Recom`
  (the folder name). The patcher's game filter compares `game.ToLower()` with a list holding
  `"Recom"`, so for Re:CoM it never skips the mod. Checked (source).

## 2. What Build does

- **Wipes the live folder first.** `RunPatcherAsync` deletes `CompiledModPath` =
  `<compiledModPath or the OpenKH folder>/mod/<game>` (`<openkh>/mod/Recom`) and then copies every
  enabled mod's assets into it. Checked (source). Panacea's `mod_path` is that folder's parent
  (`MainViewModel` writes `panacea_settings.txt` with `CompiledModPath/..`). Checked (source).
  So anything the DLL writes into `mod/Recom` is gone after the next Build.
- **A `copy` asset whose source is missing is skipped with a warning**: "File not found: … Skipping.
  Please check your mod install of: <title>", and the rest of the build goes on. Checked (source,
  `PatcherProcessor.Patch`, the `needToCopyOriginalFile` branch). `CopyFile` would throw, but it is
  not reached for a missing mod source. Checked (source).
- `dll/…` is copied like any file to `<mod_path>/recom/dll/…`. The patcher's "non-game file" names
  are `scripts/` and `dlls/` (plural), which only changes `patch-package-map.txt`. The owner's
  working `dll/kh1-recom.dll` loads, so the copy is enough. Checked (source + the owner's logs).

## 3. How Panacea finds the mod files

- Panacea runs from the game's `DBGHELP.dll` / `VERSION.dll` proxy. In its `DllMain`
  (`DLL_PROCESS_ATTACH`) it reads `panacea_settings.txt`, appends `/recom` to `mod_path`, and
  runs `Panacea::Initialize`, which installs its hooks and calls `LoadDLLs(mod_path)`: every
  `<mod_path>/recom/dll/*.dll` is `LoadLibrary`'d, then each one's `OnInit(folder)` is called with
  `folder` = `<mod_path>/recom`. **All of this runs under the loader lock**, before the game's own
  code starts. Checked (source: `dllmain.cpp`, `OpenKH.cpp`, `Panacea.cpp`).
- **New file names come from one listing made at start.** The game's `SetReplacePath` (hooked)
  creates `FakePackageFile`, which scans `<mod_path>/recom` (and `raw/`) **once** and keeps each
  name and size. A name Re:CoM's archives lack (`PLAYER/0021/PL0021.MDL`, …) exists for the game
  only if it was there at that scan. Checked (source). `SetReplacePath` is the game's own start-up
  code, so it runs after every `DllMain` / `OnInit`. Assumption (strong: it is called from the exe's
  code, which starts after the loader has run every DLL's attach).
- **Replacements of names in `Recom.hed` are looked up at each open.** `LoadFile*` / `GetFileSize`
  check `<mod_path>/recom/raw/<name>` and `<mod_path>/recom/<name>` with `GetFileAttributes` every
  time. Checked (source). For a `raw/` name that is not in the listing, `LoadFile` dereferences a
  null file-info (`fileinfo->CurrentFileName`). Checked (source). So a file written while the game
  runs is not safe to use.
- **Conclusion.** The files must be in `<mod_path>/recom` **before `OnInit` returns**. A worker
  thread can't do that: it would race the listing, and waiting for a thread under the loader lock
  deadlocks (the thread can't start until the lock is free). Written there during `OnInit`, they
  are used in the **same run**: no restart, no Build.
- `dev_path=` / `extract_path=` (Panacea settings, also listed and looked up) are global folders the
  player sets, not per mod. Not used.

## 4. The DLL and its folders

- Panacea passes `OnInit(<mod_path>/recom)`. That is the live folder Build makes. The DLL is
  `<mod_path>/recom/dll/kh1-recom.dll`, and the game folder is the exe's folder. Checked (source + code).
- The mod's own folder (the clone) is not passed. The DLL finds it (`asset_build::find_mod_folder`):
  OpenKH's folder = `mod_path`'s parent (the default layout), with `mods-manager.yml`'s
  `installedModsPath` / `compiledModPath` read as `ConfigurationService.InstalledModsPath` reads
  them, else `<openkh>/mods/Recom`. In there it takes the `<owner>/<repo>` whose `mod.yml` names
  `kh1-recom.dll` and whose DLL (`openkh/kh1-recom.dll`, or `kh1-recom.dll` in a hand-made folder) has
  the running DLL's size (Build copies it unchanged), else the one named `PM2CRAZY/KH1-in-ReCom`.
  Checked (code, tests). When none is found (a custom `compiledModPath`), the cache goes to the game
  folder instead.

## 5. The flow that was built (0.141.0)

**Repository root** (Checked: test `asset_build::tests::the_repo_root_is_an_openkh_mod`):

- `mod.yml`: `title`, `originalAuthor`, `game: Recom`, `description`, and one asset:
  `dll/kh1-recom.dll` copied from `openkh/kh1-recom.dll`. That is the build bot's file;
  `.github/workflows/build.yml` is unchanged. The old `openkh/mod.yml` (0.1.0 text) is removed.
- **The game-made files are not listed in `mod.yml`.** Listing them would be safe (a missing
  source is skipped), but:
  - the first Build, before the first launch, would print one "File not found … check your mod
    install" warning per file (about 180);
  - the list would have to follow the step's outputs by hand;
  - the DLL has to place them anyway after every Build (below), so the list would add nothing.
- `.gitignore`: `/generated/` (the cache and its stamp), plus the asset step's outputs as a
  hand-made mod folder holds them (`/PLAYER/`, `/ENEMY/`, `/raw/`, `/kh1-*.json`, `/kh1-*.bin`,
  `/kh1-menu-sounds-preview/`, `/kh1-recom.dll`). So a git update never meets them.
- No `icon.png` / `preview.png` yet (none drawn; game art must not go in the repo).

**The DLL at start** (`win::assets::ensure`, from `win::run` right after the exe check, before
anything reads the files):

1. **Decide** (`asset_build::decide`):
   - `asset_build = off` in `kh1-recom.ini` → nothing.
   - The cache holds a stamp for this step and these inputs, with every file at its size →
     **place**.
   - A failed marker for this step and these inputs → log only, no new try.
   - Else → **build**: no build yet, the step changed (a mod update), or the game files changed.
2. **The stamp** (`kh1_recom_assets::stamp`):
   - `step`: a hash of the step's own sources (`kh1-recom-assets`, `kh1-formats`, `kh1-anim`,
     `kh2-formats-lite` `src/`, `\r` dropped), made by the crate's build script `step_hash.rs`;
   - `inputs`: a hash of the names, sizes and times of `Image/dt/Recom.*`, `kh1_*`, `kh2_*`
     `.hed` / `.pkg` and the two exes;
   - one line per file written, with its size.

   It is written last (`.tmp`, then renamed). Any change to the step's code builds again after an
   update; no version number has to be bumped by hand.
3. **Build** (`win::assets::build`): the DLL writes a job file (`kh1-recom-build.job` in the game
   folder) and starts `rundll32.exe "<this DLL>",BuildAssets <job>`. `rundll32` calls the DLL's
   export `BuildAssetsW`: the same code as the game's DLL, so no exe is shipped. The game waits
   for it (30 minutes at most). A process wait needs no lock the helper takes. The helper:
   - opens a **console window** titled "KH1 Combat in Re:CoM: building the KH1 files", with the
     reason, "the game starts by itself when it is done", "please don't close this window", and
     every line of the step, also written to `kh1-recom-build.log` in the game folder;
   - checks the inputs (`Recom.hed`, `kh1_*.hed`, an exe), empties the cache (only its own
     `generated/` folder), and runs the whole step into it (`step::Dest::Root`, without the WAV
     previews);
   - on success writes the stamp, shows "Done: N files in S s. The game starts now." for 2 s, and
     exits 0;
   - on failure writes the failed marker (with the reason) and shows a **message box**. Without KH1
     it says "KINGDOM HEARTS FINAL MIX's files were not found in …\Image\dt. The mod needs KH1 Final
     Mix installed in the same … folder", then "The game starts now without the KH1 files … It
     tries again when the mod or the game is updated". It exits 1.

   The game's log (`kh1-recom.log`) gets the outcome:
   - exit 0: built;
   - exit 1: failed;
   - `0xC000013A`: the window was closed, so it runs again at the next start;
   - any other code (a crash of the step ends only the helper): the failed marker, so it is not
     repeated.

   If `rundll32` can't start (blocked), the step runs inside the game with a console, guarded by a
   failed marker written first and removed on success. A crash there ends the game once, and the
   next start skips the build.
4. **Place** (`stamp::place`): each stamped file is copied from the cache into the live folder
   when it is missing there or differs in size or time. The copy gets the cache file's time. After
   a Build (which wiped them) this restores them at the next start; otherwise nothing is copied.
   This runs before Panacea's listing, so the files are used in that same run.
5. The KH1 features that read the files (`manifest_dirs`, unchanged) then find them in the live
   folder as before. With no files (no KH1, a failed build, `off`) they keep their existing
   fallbacks (Re:CoM's own models, motions, sounds).

**What the player sees:**
- First launch: a console window for a minute or two, then the game. No restart, no Build.
- After a Build: nothing (a quick copy).
- After an update that changed the step, or a game update: the console window again.
- Without KH1 FM: one message box, then the game without the KH1 files, and no box at later starts.

**Size.** Linking the step costs about **2.1 MB** of DLL. Same toolchain (`-Zbuild-std`): 0.139.4's
code 4,447,217 bytes; 0.141.0, on 0.140.0, 6,583,300 bytes. The bot's 0.139.4 build is 4,659,027 bytes.
Checked (builds here).

**Not checked:**
- in game (no Windows here);
- `rundll32` and `CreateProcess` from inside Panacea's `DllMain`;
- the step end to end (this session's game folder is a sparse copy without `PL0001.SES`, so the
  step stops at its first read, as the command line does);
- how long a full build takes on a player's PC;
- how much the live copy costs after a Build. The step writes several hundred MB on the owner's
  files (Assumption, not measured).

## 6. Steps for players (also in `README.md`)

1. Steam: KINGDOM HEARTS HD 1.5+2.5 ReMIX with KH1 Final Mix and Re:CoM installed.
2. OpenKH Mods Manager, set up for the PC version with **Panacea** installed (its setup wizard).
3. Select **Kingdom Hearts Re:Chain of Memories** in the Mods Manager's game list.
4. Mods → "Install a new mod" → `PM2CRAZY/KH1-in-ReCom` → Install.
5. Tick the mod, **Build** (or "Build and Run").
6. Start Re:CoM. On the first start a window builds the KH1 files (a minute or two) and the game
   then starts by itself.
7. Updates: the Mods Manager's update for the mod, then Build. The next start rebuilds only when the
   update changed the asset step.

## 7. The owner's own install

- The owner's working folder `mods/Recom/PM2CRAZY/KH1-in-ReCom/` (hand-made, 183 assets) keeps
  working. With 0.141.0's DLL:
  - the first start finds that folder (its `mod.yml` names `kh1-recom.dll`; its `kh1-recom.dll`
    has the running DLL's size);
  - it builds once into its `generated/`;
  - from then on it copies the built files over whatever Build copied, where they differ.
- To keep the manual asset step in charge (e.g. testing a step that isn't pushed yet), set
  `asset_build = off` in `kh1-recom.ini`.
- A fresh GitHub install to test: move that folder out of `mods/Recom/PM2CRAZY/` (or rename it), then
  follow § 6.
