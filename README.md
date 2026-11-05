# KH1 Combat in Re:Chain of Memories

A mod that brings Kingdom Hearts Final Mix's combat to Sora in Kingdom Hearts Re:Chain of Memories
(PC, Steam, KH 1.5+2.5), built into Re:CoM's own systems. Work happens on the `testing` branch;
the design rules are in `docs/`.

## Requirements

- **KINGDOM HEARTS HD 1.5+2.5 ReMIX on Steam (PC)**, with **Kingdom Hearts Final Mix** and
  **Re:Chain of Memories** both installed (the same collection folder). The mod builds its KH1
  files from your own KH1 and Re:CoM files; no game files are shipped.
- **OpenKH Mods Manager** set up for the PC version, with **Panacea** installed (the Mods
  Manager's setup wizard does both).

## Install (OpenKH, from GitHub)

1. Open the OpenKH Mods Manager and select **Kingdom Hearts Re:Chain of Memories** in its game list
   (the mod is installed for the game selected at that moment).
2. Mods → **Install a new mod** → type `PM2CRAZY/KH1-in-ReCom` → Install. That installs the
   **release** (the `main` branch, updated only when the owner says so). For the latest test
   builds, install `PM2CRAZY/KH1-in-ReCom/testing` instead.
3. Tick the mod and press **Build** (or **Build and Run**).
4. Start Re:CoM. **On the first start the mod builds the KH1 files** from your games: the title
   screen shows "KH1 in Re:CoM: building files from your games... N %" and the step it is on, and
   waits (a minute or two). When it says "done", the title goes on by itself. Nothing else is
   needed: no second Build, no restart.
5. Next to the game exe you'll find `kh1-recom.log` (what the mod did), `kh1-recom-build.log` (the
   build's details) and `kh1-recom.ini` (settings, also in the camp menu's KH1 entry).

What the first start does: it reads KH1 Final Mix's and Re:CoM's archives (`Image/dt`) and makes
the files the mod plays with (Sora's KH1 motions, the KH1 Keyblades, enemies, effects, sounds, HUD
pieces). It keeps them in the mod's folder (`generated/`) and copies them into OpenKH's mod folder.
A later Build removes that copy; the next start puts it back in a moment. The build (and its line
on the title) comes back only when an update of the mod changes how the files are made, or when the
game itself is updated.

If KH1 Final Mix is not installed, the title says "KH1 FM not found: playing without KH1 files" and
the game goes on without them.

**Updating:** in the Mods Manager, check for updates and update the mod, then Build.

Details for developers: `docs/openkh-github-install.md`.

## KH1 playground (demo)

`kh1-playground` is a 1:1 Rust rewrite of KH1 FM's movement and Attack, with a mannequin Sora
posed by his real skeleton and motions and four training dummies. Everything is read from your own
KH 1.5+2.5 install. It finds the install by itself, or you can pass `--game-dir "<folder>"`.

- Keyboard: WASD move, Shift walk, Space jump/glide, J or Enter Attack, K Dodge Roll/Guard, L lock
  on, arrows/mouse camera, 1–8 abilities, Tab hitboxes, R reset, F1 help.
- Gamepad (KH1 default layout), PS5 DualSense or Xbox: ✕ / A Attack, ○ / B jump, □ / X
  roll/guard, R1 / RB lock on. The HUD's `pad:` line shows the pad it sees and the buttons held.
- `--trace out.csv` logs every frame for 1:1 comparison with KH1.

## Build

- Tests: `cargo test`
- Windows DLL from Linux: `tools/build-win.sh` (MinGW; writes `openkh/kh1-recom.dll`, which the
  root `mod.yml` installs; the build bot commits it on `testing`).
- The asset step by hand: `kh1-recom-build --game <game folder>` (`docs/kh1-recom-assets.md`). Set
  `asset_build = off` in `kh1-recom.ini` so the DLL leaves its files alone.
- Playground: `cargo run --release -p kh1-playground`.

This repository contains no game files, no extracted data and no decompiled code.
