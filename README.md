# KH1 Combat in Re:Chain of Memories (JoaoVitorSCS1999's fork)

A mod for Kingdom Hearts Re:Chain of Memories (PC, Steam, KH 1.5+2.5). It brings Kingdom Hearts Final Mix's
combat to Sora, and in this fork to Riku in Reverse/Rebirth too. Everything is built into Re:CoM's own
systems: its state table, command menu, attack and hit tables, motions and HUD.

The mod is by **PM2CRAZY** ([`PM2CRAZY/KH1-in-ReCom`](https://github.com/PM2CRAZY/KH1-in-ReCom); its
release is [`PM2CRAZY/KH1-Combat-in-ReCoM`](https://github.com/PM2CRAZY/KH1-Combat-in-ReCoM)). All
credit for the mod goes to them. This fork adds fixes (offered to the author as pull requests, see
[PATCHED.md](PATCHED.md)) and its own work for Riku. Each setting switches a system between Re:CoM's
and KH1's. On this fork's branch, keeping Re:CoM's originality comes ahead of copying KH1 1:1.

## What this fork adds

**Fixes** (also sent to the author, [PATCHED.md](PATCHED.md)):
- The KH1 HUD works with KH1 texture mods.
- Button glyphs follow the controller type.

**Riku in Reverse/Rebirth** (Combat = KH1):
- **Moves and fights**: KH1's movement (`riku_kh1_movement`, off by default while it is tested). KH1's
  Attack and swings with his own hit effect. KH1's MP. A command menu with his Sleights list and
  shortcuts. The KH1 HUD.
- **Guard**: KH1 Riku's own guard. It blocks, turns shots back, cancels magic shots and allows
  Counterattack.
- **Combat rules**: Sora's KH1 rules for him: post-hit invulnerability, no flinch, no card duel, R1
  lock-on and the damage cap.
- **Items and skills**: KH1's items (drops, the Items list, the camp Items page, the Moogle shop), his
  Enemy Skills from his own cards, and an Abilities page.
- **Dark Mode**: his Dark Points start and end Re:CoM's own Dark Mode, shown on Re:CoM's Dark gauge,
  once the story gives him Dark Mode. Dark Riku fights with KH1 combat. Dark Riku's own sleights (Dark
  Break, Dark Firaga, Dark Aura, Inverse Burst) cost DP instead of MP. They are marked with Re:CoM's
  Heartless emblem in the Sleights list and the shortcut panel.
- **Costumes** (cosmetic, on the Costumes page):
  - Riku's Dark Mode costume and the Keyblade of Heart unlock at his first Dark Mode.
  - Riku's Halloween Town costume unlocks on reaching Halloween Town and works in every world.
  - The costume changes in place with Riku's transformation.

**For Sora and Riku**:
- KH1 FM's **Auto Lock** and **Target Lock** (Settings > Camera). With Auto Lock on and nothing locked,
  swings aim at the nearest enemy, and at `hud = kh1` KH1's **orange auto-target ring** marks it.
- Sora's Halloween Town costume in every world, unlocked on reaching Halloween Town.
- **Talk** next to NPCs as in KH1: Talk takes the first row, and Attack is dimmed and buzzes.
- **HUD**: the KH1 HUD hides at once when Re:CoM hides its own HUD. Donald's and Goofy's KH1 gauges have no
  rings (Re:CoM's friends have no HP or MP).

The full list, newest first and with what was checked, is in [AGENTS.md](AGENTS.md) (the stages
marked "fork branch"). Every release lists its changes.

## Requirements

- **KINGDOM HEARTS HD 1.5+2.5 ReMIX on Steam (PC)**, with **Kingdom Hearts Final Mix** and
  **Re:Chain of Memories** both installed in the same collection folder. The mod builds its KH1 files
  from your own KH1 and Re:CoM files. No game files are shipped.
- **OpenKH Mods Manager** set up for the PC version, with **Panacea** installed (the Mods Manager's
  setup wizard does both).

## Install (OpenKH, from GitHub)

1. Open the OpenKH Mods Manager and select **Kingdom Hearts Re:Chain of Memories** in its game list (the
   mod is installed for the game selected at that moment).
2. Mods → **Install a new mod** → type `JoaoVitorSCS1999/KH1-in-ReCom` → Install. If you also have the
   original (`PM2CRAZY/...`), untick it: don't enable both at once.
3. Tick the mod and press **Build** (or **Build and Run**).
4. Start Re:CoM. **On the first start the mod builds the KH1 files** from your games. The title screen shows
   "KH1 in Re:CoM: building files from your games... N %" and the step it is on, and waits a minute or two.
   When it says "done", the title goes on by itself. No second Build or restart is needed.
5. Next to the game exe you'll find `kh1-recom.log` (what the mod did), `kh1-recom-build.log` (the build's
   details) and `kh1-recom.ini` (the settings, also in the camp menu's KH1 entry).

**Or from a release's zip**: download **KH1-in-ReCom.zip** from the
[Releases](https://github.com/JoaoVitorSCS1999/KH1-in-ReCom/releases). In the Mods Manager, with Re:CoM
selected: Mods → **Install a new mod** → **Select and install Mod Archive** → the zip. Tick it and press
**Build**.

What the first start does: it reads KH1 Final Mix's and Re:CoM's archives (`Image/dt`) and makes the files
the mod plays with (Sora's KH1 motions, the KH1 Keyblades, enemies, effects, sounds, HUD pieces). It keeps
them in the mod's folder (`generated/`) and copies them into OpenKH's mod folder. A later Build removes
that copy, and the next start puts it back in a moment. The build (and its line on the title) comes back
only when an update of the mod changes how the files are made, or when the game itself is updated.

If KH1 Final Mix is not installed, the title says "KH1 FM not found: playing without KH1 files" and the
game goes on without them.

**Updating:** in the Mods Manager, check for updates and update the mod, then Build. Each
[release](https://github.com/JoaoVitorSCS1999/KH1-in-ReCom/releases) says what changed.

Details for developers: `docs/openkh-github-install.md`.

## Settings

Everything is in `kh1-recom.ini` (each key is explained in the file) and in the camp menu's KH1 entry.
Some of them:

| Key | Default | What it does |
|---|---|---|
| `combat` | `kh1` | KH1's or Re:CoM's combat |
| `hud` | `kh1` | KH1's or Re:CoM's HUD |
| `auto_lock` | `on` | KH1's Auto Lock: with no lock, attacks aim at the nearest enemy (the orange ring at `hud = kh1`) |
| `target_lock` | `auto` | KH1's Target Lock: `auto` (R1 again unlocks) or `manual` (unlocks when R1 is let go) |
| `riku_kh1_movement` | `off` | KH1's movement for Riku in Reverse/Rebirth (being tested) |
| `riku_costume`, `riku_keyblade`, `sora_costume` | `default`, `soul_eater`, `default` | the cosmetic costumes and Keyblade, once unlocked |
| `all_unlock` | `off` | **for debug only**: unlocks every costume and cosmetic Keyblade |

## Build

- Tests: `cargo test` (or `tools/test-affected.sh` for only the crates a change touches).
- Windows DLL from Linux: `tools/build-win.sh` (MinGW). It writes `openkh/kh1-recom.dll`, which the root
  `mod.yml` installs. The build bot commits it.
- The asset step by hand: `kh1-recom-build --game <game folder>` (`docs/kh1-recom-assets.md`). Set
  `asset_build = off` in `kh1-recom.ini` so the DLL leaves its files alone.
- Releases: every push to `main` runs the tests, builds the DLL and publishes a GitHub Release
  (`.github/workflows/patched-release.yml`). The release notes list the changes since the previous
  release.

This repository contains no game files, no extracted data and no decompiled code.
