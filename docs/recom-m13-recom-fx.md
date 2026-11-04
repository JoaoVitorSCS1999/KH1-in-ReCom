# M13 research: how Re:CoM plays effects, sounds, voices and trails (Steam)

Research only. Nothing in the repo was changed. Sources:
- the Re:CoM Steam decomp (`/home/claude/re/out/recom`);
- the Steam exe, read with pefile and capstone (helper scripts in `/home/claude/m13tools/`, outside the repo);
- the samples `PL0001.EET/SES`, `PL0002.EET` and `Recom.hed`. The `.hed` is OpenKH's MD5 index; name lookups work against it (`m13tools/hed.py`).

Tags: **C-d** = Checked (decomp), **C-x** = Checked (exe), **C-f** = Checked (files), **A** = Assumption.
"Frame" means a 60 Hz motion frame. Re:CoM's motion time counts **50 ticks per frame**: EET and SES compare `frame × 0x32` with `0x1401A47C0(entity)` (C-d). This agrees with the existing `+0x654` note.

---

## 0. Resource types (needed by everything below)

The type ↔ extension table is at `0x140840BE0`, in rows `{char* "(XXX_TYPEn)", char* ext, u64 type}`, ended by a null row (C-x). The types that matter here:

| type | ext | role |
|---|---|---|
| 5 | `.dpx` | effect program pack (particle programs plus embedded textures) |
| 0xA | `.dpd` | effect texture pack (`texcommon.dpd` is registered globally) |
| 0xE | `.EFF` | effect container file (EFFECT table rows) |
| 0x1A | `.EET` | per-motion effect events |
| 0x1D | `.ANN` | motion set |
| 0x27 | `.LIN` | LINK row (`"CGGGG"` = also load category C, group G) |
| 0x2C / 0x2D | `.SND` | sound container (holds `.seb`, plus PC assets `*.win32.scd`) |
| 0x2E / 0x2F | `.RSD` | 0x2F is looked up by name `vo%04d` for streamed voices |
| 0x30 | `.SES` | per-motion sound events |
| 0x3E | `.ESE` | **per-effect sound table** (same base name as its `.dpx`) |

- Name lookup is `0x1402A43E0(type, name, ci)`. It is a linear search of the loaded resources of that type, and it compares only up to the `.`, so `"rec_heart.dpx"` matches the resource `rec_heart…` (C-d).
- Lookups for effects (`0x1402A4920`) and ESE (`0x1402A4A10`) retry the name with a leading `_` added or removed (C-d).
- **Every effect name is global** across all loaded files, so new KH1 names must be unique, for example a `kh1_` prefix.

The loaded groups (resource tables, C-x):

| group | contents |
|---|---|
| PLAYER 1 (Sora) | `PL0001.{RTM,TXA,MDL,ANN,BIN,EET,SES,SND}`, LINK 10002 / 31000 / 30508 / 30510 |
| PLAYER 2 (Keyblade) | `PL0002.{RTM,MDL,ANN,EET}`, LINK `30509` (EF0509.EFF) |
| PLAYER 1001 (battle set) | 23 LINKs to spell and battle EFF groups |
| SYS 1 | `SY0001.SND`, LINK 30001 |
| SYS 2 | LINK 30100 / 30500 |
| SYS 3 | `SY0003.SND`, LINK 30102 / 30505 / 30507 / 30508 / 30510 |

The EFFECT table (`0x140810500`, pointer `0x1407EBF38`) has 329 groups, mostly one `.EFF` each (C-x).

---

## 1. EET (per-motion effect events)

### Format (C-f on PL0001 / PL0002, read code C-d)

**Header:** `"@EET"`, `u32 version = 2` (high bit set after relocation), `u32 recordCount`, `u32 recordsOffset`.

**Motion record, 0x30 bytes:**

| offset | field |
|---|---|
| +0x00 | `char name[0x20]`: the clip name, e.g. `p001_022` / `w001_055` |
| +0x20 | `i32 motionIndex`: −1 in the file, filled at load |
| +0x24 | `i16 nA` |
| +0x26 | `i16 nB` |
| +0x28 | `u32 offA` |
| +0x2C | `u32 offB` |

**List A event, 0xA0 bytes, a one-node effect** (read in `0x14017A710`):

| offset | field |
|---|---|
| +0x00 | `char dpx[0x20]` |
| +0x20 | effect id |
| +0x24 | bone index |
| +0x28 | offset xyz (f32) |
| +0x34 | start frame |
| +0x38 | end frame (−1 = until the motion ends or loops) |
| +0x3C | rotation xyz, degrees |
| +0x48 | scale xyz, copied to effect `+0x10` |
| +0x54 | flags (below) |
| +0x58 | f32 fade-out frames |
| +0x5C | **variant** |
| +0x60 | f32 time skip |
| +0x64 | repeat count (≤0 = −1) |
| +0x68 | u16 tag, through `0x140255380`: effect `+0x82`, owner `+0x20` → effect `+0x84`; its use is not traced |

**List A flags:**

| bit | meaning |
|---|---|
| 0 | detached: spawn in the world and don't follow |
| 1 | with bit 0: position only (`0x1401A6B20`); otherwise a full matrix (`0x1401A6B00`); with bit 0 clear, attached to the bone (`0x1401A2D50`); also sets effect `+0x4C` bit 2 |
| 2 | effect `+0x4C` bit 3 |
| 4 | on stop, cut the emission (`0x140255040`) instead of fading (`0x140254D60(fade)`) |
| 5 | survives the motion change (the "linger" list) |
| 6 | adds a dynamic light (`0x140255180`, `CLightUnit`) |
| 7 | together with `+0x68` |
| 8 | inverted, into `0x140255360` |
| 9 | effect `+0x4C` bit 7 |
| 3 | set on almost every event, never read by CEffectMotion (A: unused here) |

**List B event, 0xA0 bytes, a three-node effect = trails** (read in `0x14017AB70`):

| offset | field |
|---|---|
| +0x00 | `dpx[0x20]` |
| +0x20 | effect id |
| +0x24 / +0x28 | bone A / offset A xyz |
| +0x34 / +0x38 | bone B / offset B |
| +0x44 / +0x48 | bone C / offset C |
| +0x54 | start frame |
| +0x58 | end frame |
| +0x5C | variant |
| +0x60 | f32 fade |

Spawned by `0x1401A2C80` → `0x1402573F0`, mode 3: nodes 1 and 2 follow bones B and C.

### Reader and runtime (C-d)

| address | does |
|---|---|
| `0x140179420` | `CEffectMotion` ctor; 32 active-event slots at `+0x830` (`+0x20` = 0x20), linger list of 64 at `+0x30` |
| `0x140179A30(cem, groupId)` | finds the EET by resource id: type 0x1A, `edx` = id (C-x disasm), then calls `0x140179B10` |
| `0x140179B10` | checks `@EET` v2, relocates offsets, and **binds each record to the motion set by name** (`0x1402A4820(cem+0x10)` = the group's ANN; linear name compare, writes `+0x20`) |
| `0x14017A240(cem, motionIdx, variant, reset)` | on motion start: fills the slots with the events of that motion **whose `+0x5C` == variant** (lists A and B) |
| `0x14017A5E0` | per-frame update: list A `0x14017A710`, list B `0x14017AB70`. Spawns when `start×50 ≤ time` (or the motion ended); stops at `end×50` with fade `+0x58`; with end −1 it lives until the motion ends, repeating `+0x64` times |
| `0x14017A430` | on motion change, fades or stops everything except flag-0x21 events |
| `0x1401A3470(ent, idx, …, eetVariant=p6, sesVariant=p7)` | the entity motion play (player vtable `+0x70`, C-x); starts EET `ent+0x108` and SES `ent+0x118` unless `ent+0x154 & 0x400 / 0x4000` |
| `0x1401A3580` | the same for a second motion set (`+0x110` / `+0x120`), loaded by group id |
| `0x1401F7180` | the player's play: the Keyblade (`player+0x3D0`) plays **the same index with the same variants** (C-x) |

Other details:
- The **variant** is chosen by the caller. For example `initMagic` (`0x1402203D0` → `0x140220B6E`) passes row byte `+1` as the EET variant and `0x14021FEF0(player, 4)` (command `+0x638` → 1/3/4/…) as the SES variant (C-x).
- In `PL0001.EET`, `p001_063` carries Fire ids 9/10/11 on variants 1/2/3, and `p001_064` carries status effects on variants 7–11 (C-f).

### Can the asset step add entries? Yes (C-d)

- The record count comes from the file and there are no fixed tables. Binding is by name against the loaded ANN, so new records named `p001_161…` / `w001_161…` bind to the extended packs. A name with no clip gets −1, which is harmless.
- Limits: 32 concurrent events per entity motion (more are ignored) and 64 lingering.
- New variants work unchanged: any int, matched against what the play call passes.
- Effects named in a record must be loaded (type 5) when the event fires. If not: handle 0, nothing happens (C-d).
- In the samples, Sora's swings have **no** list-A events. Swing visuals are list-B trails in `PL0002.EET` plus the hit effect (§5 and §6) (C-f).

---

## 2. SES (per-motion sound events)

### Format (C-f, read code C-d)

**Header:** `"@SES"`, `u32 version = 1`, `u32 count`, `u32 offset`. Motion records are 0x30 bytes: `name[0x20]`, `+0x20 i32 motionIndex`, `+0x24 u32 n`, `+0x28 u32 off`.

**Event, 0x50 bytes:**

| offset | field |
|---|---|
| +0x00 | `u32 group` |
| +0x04 | `u32 number` |
| +0x08 | bone index |
| +0x0C | offset xyz |
| +0x18 | start frame |
| +0x1C | end frame (−1 = none; at end, stop with fade) |
| +0x20 | **variant** |
| +0x24 | fade frames on stop (≤0 → 5) |
| +0x28 | flags (below) |
| +0x2C | i16 repeat (per motion loop) |
| +0x2E | i16 priority |
| +0x40 | 4 × `{i16 number, i16 weight%}` |

**Flags:**

| bit | meaning |
|---|---|
| 0 | surface step: group and number are rewritten by `0x140239510` (per-world group from `0x1407C80E0[kind][world]`, + ground material × 0x10 from the model's ground probe `model+0x158`; numbers 1–8 alternate L/R) |
| 1 | **voice**: id `| 0x10000000`, one voice slot per entity (`+0xC8`, priority `+0xD4`, kept up to 30000 ticks through motion changes, `0x140239810`) |
| 2 | **random**: `0x140238F50` picks from the 4 weighted numbers and never repeats the last pick (its weight is spread over the others) |

The sound id is `group × 1000 + number`. Examples from `PL0001.SES` (C-f):

| group | sounds | e.g. |
|---|---|---|
| 601 | steps | |
| 1 | whooshes | 1232–1245 |
| 2 | Sora's grunts, random | `p001_055`: 2005 40%, 2006 30%, 2008 30% |
| 0 | jumps / lands | 103 |

### Reader and runtime (C-d)

| address | does |
|---|---|
| `0x140238710(cse, groupId)` / `0x1402387E0` | load by resource id (type 0x30); `@SES` v1; binds by name like the EET; 16 slots |
| `0x140239260(cse, idx, variant, reset)` | motion start: picks the events for (motion, variant) |
| `0x1402399B0` | per-frame update: plays at `start×50` through `0x1401A61C0`, updates the 3D position each frame (`0x1401A6EE0`), stops at end or on loop |
| `0x140239810` | motion change (vtable `+0x10`): **every non-voice sound is stopped with its fade (default 5 frames)**; a voice moves to the voice slot |
| `0x1401A61C0(ent, group, no, bone, offset, voice)` | the entity sound: position = bone world (`+0xD0`) + offset, through `0x1402A9300` / `0x1402A9290` (entity flags `+0x154` bits 19 / 20); then `0x1402AA0F0(id, pos, −1)` |
| `0x1401A62E0(ent, group, no, prio)` | the scripted or AI voice call into the same voice slot |

**Voices:**
- Sora's combat grunts are **ordinary bank sounds** (group 2 → bank 3, below), random-picked by SES flag 2.
- The flag-1 path (`0x10000000`) is the streamed voice path: `vo%04d` type-0x2F resources, from `0x1402AA0F0` voice branch → `0x1402A80F0`. Its SCDs are registered as bank **0xFFFF** (`0x1402F1250` from `0x1402A7F50`) (C-x / C-d). Sora's SES does not use it (C-f).
- The SES fade on motion change is a known difference from KH1 (the repo already notes this as an Assumption on the KH1 side).

**The asset step can add records** for new clips the same way as the EET. Proven: the repo already ships two extra SES records (C-f / C-d).

---

## 3. The effect system

| address | does | tag |
|---|---|---|
| `0x1402580C0` | init: `CEffectSLCManager` pool of **0x400 handles** × 0x20 (`0x1402425E0`); particle buffers 1 MB + 256 KB (`0x140474C40`); registers `texcommon.dpd`; `CEffectTask` | C-d |
| `0x140255F00(dpxRes, id, ownerId, nodes, boneMtx, worldMtx, pos, rot, p9, ese)` | makes a `CEffect` (0xB0): `0x1402553B0` finds the program, `0x140255E00` places it, `0x1402546E0` starts its ESE sound; returns a handle | C-d |
| `0x1402553B0` | dpx data `+0xC` = count, `+0x10` = 0x20-byte entries `{u32 off, i32 x, u32 id, …}`; picks **entry.id == effect id**; makes an N-node skeleton (`0x14023DDD0`) and the particle instance (`0x1404747F0` / `0x140475AF0`) | C-d |
| `0x140254330` (from the type-5 handler `0x14029F9C0`) | at dpx load: relocates entries, then registers the dpx's **embedded textures** with the particle lib (`0x140474990`); a name starting with `_` gets the 0x8C marker | C-d |
| `0x1402577D0` | spawn attached to a bone | C-d |
| `0x1402587D0` | spawn at a world matrix | C-d |
| `0x1402588B0` | spawn at a point | C-d |
| `0x1402573F0` | spawn with 3 nodes (trails) | C-d |
| `0x1401A2D50` / `0x1401A6B00` / `0x1401A6B20` / `0x1401A2C80` | entity wrappers for the four spawns; they pass `ent+0x150` as owner id | C-d |
| `0x140257EF0(h)` | handle → `CEffect*`, or 0 when dead | C-d |
| `0x140254D60(e, fade)` | fade out (≤0 = kill) and stop the ESE sound (`0x1402A8F90`) | C-d |
| `0x140255040` | stop emitting | C-d |
| `0x140255320(e, frames)` | time skip, passes **frames × 0.5** to the particle runtime | C-d; that the runtime counts 30 Hz steps: A |
| `0x140257B30` | draws every effect; an instance whose program finished (`0x140475190` = 0) is freed | C-d |

**ESE (type 0x3E), C-d (`0x1402546E0`, `0x140254E20`):**
- `+0xE` = u16 count.
- `+0x10` = 8-byte rows `{u16 effectId, u16 group, u16 number, u8 fade (0 → 30), u8 ?}`.
- The sound `group×1000+number` plays when the effect starts, follows the effect's node 0 (`0x1402AAC60`) and stops with the effect.
- **So hit and spell sounds ride on the effect.** This is the "clink rides on the effect" in the Guard notes.

**Loading a new effect natively.** Effects come only from loaded `.EFF` groups (§0). There are two native routes:

- **(a) New file.** Write a new `EFFECT/09xx/EF09xx.EFF` (archive container `{u32 off, name[0x18], u32 size|0x80000000}` holding `kh1_*.dpx` + `kh1_*.ese` (+ `.dpd`)). Then:
  - add a new EFFECT-table row (relocate the table at pointer `0x1407EBF38`, as is already done for PLAYER at `0x1407EBF28`);
  - add a LINK row `"3GGGG"` to PLAYER group 1 (always with Sora) or 1001 (battle set).

  Panacea's loose route gives a new name **no PC assets** (`recom-keyblade-models.md` §8). HD textures, if the PC dpx uses them, need the raw route (A).
- **(b) Extend an existing EFF.** Add entries to an EFF Sora already links (EF0508 / EF0510, or EF0509 with the Keyblade). Re:CoM's own entries stay byte-identical.

Either way the **EFF container itself was not seen** (no sample). Whether an EFF holds `.dpx` + `.ese` + `.dpd` as sub-files is A (strong: types and lookups). Its PC asset part is also unseen: stored size > raw size in the `.hed`, e.g. EF0509 57 472 vs 25 024 (C-f).

**Capacity:**
- 1024 live effect handles (C-d);
- 32 EET events per entity motion (C-d);
- particle memory 1 MB + 256 KB, shared (C-d; that it is the practical cap: A).

---

## 4. The sound system

| address | does | tag |
|---|---|---|
| `0x1402AA0F0(id, pos, vol)` | play: bank = `0x1402A8560(id)`, sound = `id % 1000`; vol −1 → 127, /127 → `0x1402F3080` (SslSeCreate2: first `Axa::BankEntry` with that bank → `CreateSound(handle, no)`, sets 3D position `0x140055A90` and volume `0x140055670`); NaN position → far away | C-d / C-x |
| `0x1402A8560` | id → bank: group 0→0, 1→1, 2–3→3, 100–199→2, 200–599→itself, 600–799→0xC–0xF (`%5`), 800–7999→4, else −1 | C-d (hooked by our menu sounds for 9000–9999) |
| `0x1402A8210` | the same map for a `.seb`'s group (header `+8`, u16) | C-d |
| `0x1402AB290` | sub-file handler: `*.seb` → `0x1402A79F0` → `0x1402F0780(data, bank, stem)`. With the handler's flag set, only seb names whose digits 2–5 are 200–599 register | C-d |
| `0x1402F0780` | walks **the PC assets of the same file** (`0x1402BF320`); each asset whose name contains the stem, `.win32.scd` and `se`/`music` → `0x1402F5D70(name, bank, data, size)`. **Before registering, `0x1402F5BB0(bank)` frees that bank's old entries: one file per bank at a time** | C-d |
| `0x1402F5D70` → `0x1402F0080` | `Axa::BankEntry` pushed on `0x140822370` (repo note, re-read) | C-d |
| `0x1402F1250` | voice SCDs → bank 0xFFFF | C-d |

**Which file holds which bank is not visible here.**
- Assumption: group 1 (whooshes) and group 2 (grunts) are `.seb`s inside `PL0001.SND` (banks 1 and 3); bank 0 is `SY0001.SND`; the 600–799 steps are `SY0003.SND` or a world SND (SYS 901–911).
- `PL0001.SND` is 290 752 B raw and 854 368 B stored, so it carries PC assets (C-f via `.hed`).

**Registering a KH1 bank natively:**
- **(a) A new SND with a stub `.seb` plus its `kh1….win32.scd` PC asset**, through Panacea's raw route, with a new SND or LINK row.
  - The PC code reads only the `.seb`'s `+8` group (C-d; that a stub suffices: A).
  - Its group must map to a bank nobody else loads. 200–599 are enemy and battle banks, so the free numbers need the full extract.
  - Alternatively extend the existing `0x1402A8560` hook (and hook `0x1402A8210`) with a new range.
- **(b)** Register the SCD directly with `0x1402F5D70`, as `menu_sounds` already does for bank 9000. It is played by Re:CoM's own path; only the registration is ours.

Then KH1 sounds are just ids in SES / ESE rows. KH1's SCDs play unchanged (same `SoundController` driver; repo note C-x).

Not checked: the voice or channel limit, Re:CoM's SE volume bus applied to new banks, and what `0x1402A9300` / `0x1402A9290` do exactly (listener vs camera space: A).

---

## 5. The Keyblade trail

- **The trail is EET list B in `PL0002.EET`** (C-f). Records `w001_055…061` (combo 1–7):

  | field | value |
  |---|---|
  | effect | `ex_5010.dpx` id 5 |
  | node A | bone 2 (`wep0`), (−0.11, 0, 0) |
  | node B | bone 2, (−0.78, 0, 0) |
  | node C | bone 2, (0, 0, 0) |
  | frames, `w001_055` | 7–17 |
  | frames, `w001_058` | 19–33 |
  | fade | 10 frames (`w001_060`: 12; `w001_058`: 15) |
  | variant | 0 |

- It is played on the weapon entity because CPlayerWeapon plays Sora's motion index (`0x1401F7180`).
- `ex_5010.dpx` is the Kingdom Key's effect pack. That it lives in `EF0509.EFF` (group 2's LINK) is A (strong).
- **Data:**
  - colour, texture and ribbon behaviour are in the dpx program (not decoded; needs EF0509);
  - length and placement are the two node offsets in the EET;
  - on/off is the frame window.
- **Per Keyblade:** our converted Keyblade groups share Re:CoM's `PL0002.EET` (repo §10.2), so every Keyblade gets the Kingdom Key's 0.78 m trail. Halloween's group 12 already has its own `PL0012.EET`. Per-Keyblade lengths therefore mean one EET per group, written by the asset step.
- KH1 trails for the new `w001_161+` clips are new list-B records in that EET.
- **Correction to `recom-keyblade-models.md` §9.6:** `CPlayerWeapon +0x2C0` is **not** the trail.
  - It is the model's ground and wall probe (`model+0x158`, radius 0.25 at `0x140172690`, set in the ctor `0x140224BA0` when `0x140235270()`).
  - `0x140225370` / `0x140225F00` move it along the blade.
  - When it hits a surface steeper than cos⁻¹ 0.6428 (50°), Sora recoils (vtable `+0x108(2)`) and the spark `0x1405A1480[weapon+0x2D0]` plays at the contact (C-d).

---

## 6. Hit effects

| what | where | tag |
|---|---|---|
| Physical hit-effect table: 27 rows × 0x18 `{char* dpx, i32 id, i32 −1, 0}`. 0 none, 1–3 `ex_5010` 0/1/2, 4–7 limit*, 8 `fire_all` 7, 9 `bliz_all` 4, 10 `_thunder_all` 6, …, 18–26 `weapon` / `ex_1580` / `dark_sword` | `0x1405A1480` (no rip-relative refs: reached through image base + RVA) | C-x |
| Attack row `+2` (physical rows) indexes it: combo rows 1/2/3/5/6 → 1 (`ex_5010` 0); finishers 4/7 → 3 (`ex_5010` 2). Copied into the attack's hit struct (`0x1407C7E70`: `+0x18` name, `+0x20` id, `+0x24` sound group = −1, `+0x28` sound no) at `onAttack` | `0x1407C6C70`, `0x140225B..` | C-x / C-d |
| On a hit the victim plays it **at the contact point**: `0x14016FF60` (record `+0xF8` inline name, `+0x108` id, `+0x10C` owner → `0x1402588B0`). Bullets: `0x140156A30` (`+0x18` / `+0x20` effect, `+0x24` / `+0x28` sound) | | C-d |
| Hit sound: the row's sound group is −1, so the clink comes from the effect's ESE | | C-d / A (ESE content unseen) |
| Guard spark: `always` #0xB (#0xA when the enemy data's `+0xC ≤ 0`); `_ex_2050` #0xF for kind 9; `vexen` #6 for kind 0x72 | `0x140182260` | C-d |
| Clash ("duel"): `duel` #2 attached, #3 at a point, scaled per enemy (`0x14052B450` / `B478` / `B4A0`) | `0x140183510` | C-d |

- **By element:** for physical rows no per-element pick was found; the effect is per row. Spell hits use their effect set (§7).
- **Hit-stop: not found in this pass.** No Re:CoM routine that freezes motions on a hit was located. Motion speed writes are `0x1401A65A0`; the `+0x47C = 0` write found is the Stop status. Open: trace `CPlayerWeapon::onAttack` → `0x14016F1C0` → the enemy reaction `0x14017F7E0` in a debugger.

---

## 7. Spell and sleight effects

- **Effect sets:** `0x1405A1730`, **67 sets** × 0x60 = 4 slots × `{char* dpx, i32 id, i32 soundGroup (−1), u32 soundNo|voiceBit31}`, ending at `0x1405A3050` (C-x).
  - Row `+2` of a magic or sleight row picks the set: Fire 3/4/5 `fire_all`, Blizzard 6–11, Thunder 12–14, Cure 15–17, Gravity 18–20, Stop 21–23, Aero 24–26, then the sleights 31–66 (`figa_burst`, `mega_flare`, `holy`, `tornade`, …).
  - Slots by use (C-d sites): `+0x18` the shot or end effect plus its sound (`0x140156A30`), `+0x30` world spawn, `+0x48` bone-attached (`0x140157E60` / `0x1401577A0`). Exact meaning per spell: A.
  - The table has 54 rip-relative references (C-x), so new sets need a relocated table and those sites patched, or a new set id range read through a wrapper.
- **Cast visuals** are EET variants on the cast clips (`p001_063` Fire ids 9/10/11; Keyblade `w001_062` `fire_all` id 0) plus SES on the cast motions. Effect sounds come from each dpx's ESE.
- The packs come from the battle set PLAYER 1001 (already loaded in the field by castfix, repo note).

So for KH1 spells in Re:CoM's native form:
- new `kh1_*.dpx` / `.ese` in an EFF;
- new effect sets (or set slots);
- new EET / SES records or variants on the KH1 cast clips.

---

## What I couldn't check, and files needed from the owner's full extract

1. **`EFFECT/0509/EF0509.EFF`**, plus EF0001 / 0100 / 0102 / 0500 / 0505 / 0507 / 0508 / 0510 / 1000 / 2000–2063. These would show:
   - the EFF container layout;
   - which dpx lives where;
   - the dpx program format (trail colour and texture);
   - the ESE rows (the hit clink ids);
   - the PC assets.
2. **`PLAYER/0001/PL0001.SND`**, `SYS/0001/SY0001.SND`, `SYS/0003/SY0003.SND`, one `SYS/09xx/SY09xx.SND` and one `ENEMY/xxxx/ENxxxx.SND`. These would show the `.seb` header, which groups and banks each file registers, the free bank numbers, and the SCD asset names.
3. One `.RSD` (`PLAYER/0301/PL0301.RSD`), for the voice resource layout.
4. Not checked in game: everything above. Not found: Re:CoM's hit-stop; the meaning of EET flag bit 3 and `+0x68`; the 0x10 bytes of the ESE header.
5. Open, for the owner and the KH1 side: **whether KH1 FM's effect files can be converted to Re:CoM's dpx**.
   - Re:CoM's runtime is the SQEX particle lib at `0x14047xxxx` (`DX`/`DP` stamps in `0x140254330`).
   - The KH1 effect format must be compared using KH1's decomp and files.
   - If they differ, KH1's effects need a dpx writer: a big unknown for M13 (A).

Helper scripts (outside the repo): `/home/claude/m13tools/{pe,hed,fmt,dz,xref}.py`.
