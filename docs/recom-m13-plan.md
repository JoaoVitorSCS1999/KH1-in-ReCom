# M13: KH1 effects, sounds and voices (plan)

Owner's ask (2026-11-03): "Let's work on m13". Roadmap: "KH1's swing trails, hit effects, spell effects,
sounds and voices, built into Re:CoM's effect programs and sound banks."

The research is in `recom-m13-kh1-fx.md` (KH1), `recom-m13-recom-fx.md` (Re:CoM) and the inventory below.
The owner's answers are in `design-philosophy-instructions.md` § "M13 effects, sounds and voices".

## Key finding

KH1 and Re:CoM use the **same effect format (DPX `0x82`, PPP `0x96`), at the same version**:
- Re:CoM's `ex_5010.dpx` (the Kingdom Key trail and sparks, in `EFFECT/1000/EF1000.EFF`) is **byte-identical**
  to KH1's `xw_ex_5010.wpn` section 0. Checked (files).
- Re:CoM's `ex_0010.dpx` is an edited copy of KH1's Sora set with KH1's ids 6 and 7 (Ripple Drive) removed.
  Checked (files).
- Re:CoM's op table (`0x140859DD0`, 242 ops) starts with KH1's 235 ops (`0x14054FF50`), in the same order.
  Checked (exe).
- The one data difference: KH1 keeps an effect's sound in its program header (`+0xFC` / `+0xFE`); Re:CoM keeps
  it in a `.ese` table of the same name. Checked (files).

So KH1's effect sets go into Re:CoM as Re:CoM's own `.EFF` files (renamed `kh1_*`, since effect names are global),
with `.ese` tables written by the asset step. Re:CoM loads, steps, draws and ends them.

## How it is built (native)

1. **Asset step.** From the player's files, the asset step writes:
   - a new Re:CoM `.EFF` holding KH1's sets: Sora's 8-program set, each Keyblade's `.wpn` set, `always.dpx`,
     the spells' `.mag` sets and the ported enemies' sets;
   - the sets' `.ese` tables;
   - the sets' HD textures;
   - and loads it with Sora's group (a LINK row, or an EFF Sora already loads).
2. **Sound banks.** KH1's sounds registered as Re:CoM banks: Sora's `.mdls` SCDs, the Keyblade and effect
   sounds (`se000`, the world `se<ww>` packs) and the voices (`voice/<ww>_sora`). This uses the route the KH1
   menu sounds already use (bank 9000: `0x1402F5D70` registration, the `0x1402A8560` extension). A KH1 → Re:CoM
   sound-id map goes with them.
3. **Per-move events.** For every KH1 move, the asset step writes EET and SES records into Sora's pack;
   Re:CoM's own `0x14017A240` / `0x140239260` fire them. They follow KH1's frames:
   - trails on the mset type-7 windows;
   - sounds on the type-6 triggers;
   - voices by KH1's weighted groups.
   Each set of records is chosen by the toggle's variant.
4. **Toggles** (KH1 > Combat): Effects, Sounds and Voices, each KH1 / Re:CoM, plus decision 34's spell-effects
   toggle. At Re:CoM nothing changes.

## Stages (one system per version)

1. **Groundwork.**
   - Confirm Re:CoM's PPP tick rate against KH1's 30 Hz (`0x1401ECB50`).
   - Confirm whether Re:CoM's player reads program `+0xFC`.
   - Build the KH1 `.EFF` and load it with Sora. Nothing visible changes; the log shows it loaded.
2. **KH1 sound banks** and the id map.
3. **Sora's combos and finishers.** KH1 trails and swing sounds; KH1 voices (groups 4 / 5 by KH1's weights);
   the KH1-only moves' effects (Blitz, Ripple Drive, Hurricane Blast, Gravity Break, …).
4. **Per-Keyblade trails.** One EET per Keyblade group, each with its own `.wpn` set.
5. **Hit sparks.** KH1's by hit kind (normal, hard, critical, wall); Guard and clash sparks from `always.dpx`
   10 / 11.
6. **Spells.** Decision 34's toggle; Re:CoM's spell sounds kept; Fire first.
7. **Specials.** KH1's effects and voices for Sonic Blade, Strike Raid, Ars Arcanum and Ragnarok (owner,
   2026-11-03).
8. **Movement and damage.** Footsteps by surface (`footstep.bin`); jump, land and roll; hurt and death voices
   (groups 0 / 1 / 2); the silent recoils.
9. **Enemies.**
   - The ported Heartless's own sets: warps, the Search Ghost's light, the Fat Bandit's flame, the Gargoyle's
     ball, fades.
   - Sounds for the Red Nocturne, Green Requiem, Yellow Opera and the Air Pirate's swoop.

## Inventory before M13 (v0.101.6)

- No KH1 combat sound or voice plays: every swing, spell, special and enemy uses Re:CoM's. Only the KH1 menu
  sounds are KH1's (bank 9000).
- No KH1 effect is built. The asset step writes no EET, so the KH1-only clips have no motion effects.
- Silent:
  - Sora's strong recoil 0x6F, air recoil 0x70 and the Neoshadow-grab held reaction;
  - the Air Pirate's swoop;
  - every Red Nocturne / Green Requiem / Yellow Opera motion (Re:CoM has no `EN0003/5/6.SES`).
- Stand-ins: Guard and Counterattack sounds; guard and clash sparks; the Bandit's scimitar; the Fat Bandit's
  flame; the Gargoyle's ball; the Yellow Opera warning; the Aero barrier.

## Stage 1 checks (2026-11-03)

- **Re:CoM's effect clock = KH1's: 30 Hz.** Checked (decomp):
  - `0x140258660` sets `0x14087CF40` = task dt × 0.02, which is 1.0 per 60 Hz frame (`recom-timing.md`);
  - `0x140258B00` → `0x140255780` → `0x140475820` stores it at `0x140C42584`;
  - `0x140474F60` adds it to the instance's `+0xD4` and steps the program (`0x1404797F0`, +0x1000 per tick)
    once per 2.0.
  This is the same as KH1's `0x1401ECB50`. In Re:CoM's 30 fps rooms dt is 100, so the step is 2.0: one program
  step per update, still 30 Hz. **KH1's effect lifetimes carry over unconverted.**
- **Re:CoM never reads a program's `+0xFC` sound.** Checked (decomp: KH1 reads it in its instance pool,
  `0x1401E*` around `0x1401EC470`; Re:CoM's decomp has no reader). Re:CoM's sound comes only from the
  `.ese` passed into `0x140255F00` (looked up by `0x1402A4A10`). So the asset step writes one `.ese` row per
  KH1 program with a sound: `{id, group, index, volume}`. The KH1 volume `+0xFE` → `.ese` volume mapping is
  still open.
- **Where Sora's effects load:** PLAYER 1's LINK `31000` = category 3 (EFFECT), group 1000 = `EF1000.EFF`
  (`ex_0010.dpx`, `ex_5010.dpx`, `ex_0010.ese`). Checked (files).

## Stage 1 built (0.102.0)

- **Asset step** (`kh1_recom_assets::effects`): `raw/EFFECT/1000/EF1000.EFF` is Re:CoM's container with its three
  rows byte-identical (only their offsets move, 0x20 per new row). KH1's Sora set `xa_ex_0010.mdls` section 1 is
  added as **`kh1_ex_0010.dpx`**: 126 592 bytes, effect ids 0–7. Its one PC texture, KH1's `-8fb40.dds`, becomes
  `kh1_ex_0010_dpxbb40.dds`. Re:CoM's four PC assets are kept, and so is its date. `kh1-effects.json` lists the set.
  Checked (files: the owner's install, 2026-11-03). The rows, bytes, assets and texture were compared with
  Re:CoM's and KH1's originals.
- **Container** (`0x1402A2A60`): 0x20-byte rows `{offset, name (21 characters), u8, u8, size | 0x80000000}`, read
  until a zero offset. Checked (decomp).
- **PC texture names:** `0x140477BB0` → `0x1402BBBC0` uses `sprintf("%s%x", set name, texture − set)` with
  `.` → `_`, plus `.dds` (`0x1402BBC42`). Checked (decomp + exe). The tag is not used on this path, so the new
  asset copies Square's `0x48…` pattern (Assumption: unused).
- **DLL** (`win::effects`): at the first battle of a session, Re:CoM's effect-by-name `0x1402A4920` (16 bytes
  checked) looks each set up once and writes one log line. Nothing per frame, no hook.
- **Not yet:** nothing plays the KH1 set. Stage 3 writes the per-move events, and stage 2 writes the `.ese` sound rows.
- **Keyblade sets** move to stage 4: each goes with its own Keyblade group, so only the equipped one is loaded.
- **To check in game:**
  - the log line says `kh1_ex_0010.dpx loaded by Re:CoM (8 entries …)`;
  - Re:CoM's own Kingdom Key trail and Sora's sleight effects still look and sound the same, which checks the
    moved rows;
  - at Combat = Re:CoM nothing changes. The container is served at both settings, but Re:CoM's own rows are untouched.

## Stage 2 built (0.108.0): KH1 sound banks and the id map

Owner, 2026-11-04: "also, lets add all of soras effects for real this time. He doesn't have his keyblade trail,
gravity break doesnt have it effects/sound, neither does ripple drive or stun impact,".

- **How KH1 finds a sound** (Checked (decomp)): every id ≥ 1000 is **its own one-sound SCD**, a PC asset named
  `se0<id>.win32.scd`. KH1 registers a file's SCD assets (`0x1400E0A50` / `0x1400DB910` → `0x1400E0F00`), numbers
  each entry `atoi(name + 2)` (`0x1400DB1F0`), and makes id `n` from **entry 0** of the entry numbered `n`
  (`0x1400DE460`, the branch for banks other than the system bank). Ids under 1000 are entries of the system bank 2
  (`0x1404D5EB8` = 2: Checked (exe)), `se000.win32.scd`, entry = id. Checked (files): Sora's 21 SCDs
  (`se011001` … `se011017`, `se011023`, `se011510`, `se011511`, `se014430`) hold one sound each.
- **How Re:CoM plays one** (Checked (decomp)): `0x1402F3080` takes the first bank entry whose bank is the id's
  (`0x1402A8560`) and makes sound `id % 1000` of it. So each KH1 SCD is **one Re:CoM bank** (9001 …), played as
  `bank × 1000`; KH1's system bank stays the menu sounds' bank 9000 (entry = KH1 id).
- **Asset step** (`kh1_recom_assets::sounds`, `main.rs` `sounds_build`): takes every SCD of Sora's
  `xa_ex_0010.mdls` and the Kingdom Key's 40 weapon ids (btltbl weapon row of item 0x51, `+0x34` = `0x2B10`), from
  Sora's file first, then the world packs `amusic/se<ww>.dat` in KH1's table order (`0x1404DCCA0`) plus
  `seworldmap` / `sezz`; each SCD checked as both drivers read it and copied unchanged into **`kh1-sounds.bin`**;
  **`kh1-sounds.json`** gives each KH1 id its bank, offset, size and source, and lists the ids not found. A copy of
  the same id in a later file that differs is noted (the first kept).
- **Effect sounds** (`kh1_recom_assets::effects`): KH1's Sora set's programs carry sounds 81–88 at `+0xFC` with
  volume offset 0 (Checked (files)); the step writes **`kh1_ex_0010.ese`** into `EF1000.EFF` next to the set, rows
  `{id k, group 9000, number 81 + k}` for k = 0..7 (Re:CoM looks the `.ese` up by the set's own name:
  `0x140256A90` → `0x1402A4A10(name)`, Checked (decomp)). Only when the menu sounds' system bank was taken (it
  holds the entries); a program with a non-zero volume offset would get no row (the `.ese` has no field for it).
- **DLL** (`crate::sounds`, `win::sounds`; `win::menu_sounds` shares its map hook, `_aligned_malloc` check, SCD
  reader and registration): reads the manifest at start; at each battle start with Combat = KH1 (the battle manager
  ctor wrapper) one walk of Re:CoM's bank list, then each missing SCD goes to Re:CoM's `0x1402F5D70` (bank 9000
  too). One log line. No new hook or site (the map hook `0x1402A8560` is the menu sounds'); nothing per frame. KH1's
  system bank is now read even without the KH1 command menu look (it carries the effects' sounds).
- **Nothing plays them yet**: stage 3's SES / `.ese` records use the map. At Combat = Re:CoM nothing is registered.
- **Local run** (the session's files: Sora's `.mdls` with its 21 SCDs, a 1280-byte-short local copy of
  `se000.win32.scd` padded for the run): 21 banks 9001..9021; **the Kingdom Key's 40 not found** (no world pack data
  in the session); `kh1_ex_0010.ese` with 8 rows 9000081 … 9000088; the DLL parser reads the output
  (`sounds::tests::the_steps_output_parses`, `KH1_SOUNDS_JSON`).
- **Assumption**: the Kingdom Key's SCDs are in the world packs. The owner's run says which (the step prints the
  source of each id and lists what it could not find).

## Stage 3 built (0.109.0): Sora's combos and finishers

Owner, 2026-11-04: "He doesn't have his keyblade trail, gravity break doesnt have it effects/sound, neither does
ripple drive or stun impact,".

- **How it plays** (Checked (decomp)): Re:CoM's motion start keeps the `.EET` / `.SES` events of the variant the motion
  request passes (`0x1401A3470` → `0x14017A240` walks every record bound to the motion; `0x140239260`); the Keyblade
  plays the same index with the same variants (`0x1401F7180`). So KH1's version of each move is a set of Re:CoM events
  at **variant 1301**, written by the asset step (`kh1_recom_assets::kh1_fx`, `eet`); the DLL passes 1301 in the
  motion request it already makes for the move (`win::fx::variants` in `Kh1Sora::play`), per toggle, only for the moves
  `kh1-fx.json` lists. Re:CoM's own events (variant 0) stay byte-identical (Checked (files): the step rebuilds Re:CoM's
  `PL0001.EET` / `PL0002.EET` byte for byte; its layout puts every list A before every list B).
- **Trail** (`PL0002.EET`, list B): every KH1 row with a trail (`+2` ≥ 0: combo rows, finishers, KH1-only moves, Guard,
  Counterattack), on its Keyblade clip: Re:CoM's own effect 5 of `ex_5010.dpx` (= KH1's Kingdom Key set, Checked
  (files)) on Re:CoM's own three nodes (its `w001_055` record's), from the first whole frame ≥ 1 with the motion's
  type-7 flag on to the first frame it is off (`0x1402C1F20`). Windows from the player's mset: 200 4–24, 201 12–32,
  202 18–30, 203 8–38, 204 2–20, 205 4–20, 206 6–38, 207 2–20, 209 2–54, 210 22–38, 211 14–42, 213 4–24, 214 2–28,
  215 26–36, 217 28–38, 218 30–50; none on 208, 212, 216 (Checked (files): matches `recom-m13-kh1-fx.md` § 1).
- **KH1's trail end** (`win::fx`): Re:CoM's list B can only fade or kill at its end, and leaves ended effects past a
  motion change; KH1 stops the emission at the window's end and kills the trail at a motion change. The KH1 events
  carry the fade −1301 (Re:CoM's are 10–15): the end-frame call `0x14017ABEF` (`call 0x140254D60`) is retargeted to
  `trail_end`, which stops the emission (`0x140255040`) for that fade and fades as before otherwise; the EET motion stop
  `0x14017A430` (vtable slot `0x14052A338`) is wrapped: after Re:CoM's, the KH1 trails in that player's linger list
  (`+0x28` / `+0x30`) are killed (`0x140254D60(e, 0)`); a live one is killed by Re:CoM's own stop (fade ≤ 0). 10 bytes
  moved (whole instructions, nothing rip-relative, no branch into them: exe scan). Once per trail end / motion change.
- **Sounds** (`PL0001.SES`, `PL0011.SES`): at variant 1301, each KH1 sound key (op 0x19 = Kingdom Key `0x2B10` + arg,
  op 0 absolute) at its frame through the stage 2 map; the clip's own Re:CoM steps and voices copied to the variant;
  Re:CoM's whoosh kept for a KH1 id with no bank. The Kingdom Key's SCDs are the PC assets of **`xw_ex_5010.se`**
  (Checked (files): the owner's install, 40 SCDs `se011018`..`se011022` and `se011024`..); the step reads every SCD of
  the Keyblade's `.se` and no longer reads the world packs (they hold none: the owner's 0.108.0 run).
- **Finisher effects** (`PL0001.EET`, `PL0002.EET`, list A, `kh1_ex_0010.dpx`, frame 0 = KH1's row start,
  `0x1402A68F0`): Stun Impact 1; Gravity Break 3, and 2 on the blade (code 0x30 at fraction 0.8: t = −2.96 on Re:CoM's
  `wep0` → `k9`, from `PL0002.MDL`; motion-locked); Zantetsuken 4; Ripple Drive 6 (KH1 bone 8 = Re:CoM `belt1`) and 7.
  `0x18001` = Sora's position and facing, once: detached from Root's matrix (EET flags 0x29); 0x800 = EET's linger bit.
  Their sounds come from `kh1_ex_0010.ese` (stage 2).
- **Their size (checked 0.113.0, owner: "gravity's visual is really small")**: not off by KH1's cm ↔ Re:CoM's m.
  Re:CoM's own `ex_0010.dpx` is KH1's Sora set edited (Checked (files: `EF1000.EFF`)): its program 2 differs from
  KH1's program 2 in 12 words, none a size; program 3's bounds are 183 / 3.6 against KH1's 207 / 10 (Re:CoM's edit,
  the same order of magnitude), so Re:CoM draws program data in KH1's units. Re:CoM's own `ex_0010` events in
  `PL0001.EET` (`p001_101`, `p001_104`) have scale 1.0, as ours. KH1 starts Gravity Break's 3 / 2 with no scale
  (`0x1402AE160`; `0x1402ADC20`'s last argument is the blade fraction 0.8, link `+0xC`) (Checked (decomp)). So the
  finisher effects, the trail (byte-identical) and Stun Impact / Ripple Drive are KH1's size relative to Sora: no
  change. Assumption: both runtimes draw a program unit at the same size relative to the model (Re:CoM ships KH1's
  numbers for its own Sora set). The small visual was the KH1 Gravity spell's sphere (`recom-m7-magic.md` § 7.8,
  The sphere's size).
- ~~**Not built, native gap**: Stun Impact's **0** and Zantetsuken's **5** go to `0x8003` = the target's contact point.~~
  **Wrong** (corrected 0.112.0): `0x8003` is Sora's own body point; built as EET events (§ 0.112.0 built).
- **Toggles**: ini `effects` / `sounds` (kh1 | recom, default kh1), read at each motion start; at Re:CoM nothing changes
  (variant 0). Not yet on a camp page: the Combat page holds Re:CoM's 5 rows already (`menu::MAX_ROWS`): owner question.
- **Voices: not in this stage.** No KH1 voice file is in the session, and KH1's groups (up to 5 clips, an equal-chance
  pool, the name call 0xFD, priorities) don't fit one SES random event of 4 weighted numbers without a design choice.
- Halloween Town: the step adds the same events to `PL0011.EET` / `PL0012.EET` / `PL0011.SES` when Recom.hed has them.
- Assumptions: Re:CoM's particle runtime applies a program's start delay `+0x108` as KH1's (the effects start at frame 0;
  KH1's ids 1, 3, 4, 5, 7 have 34 / 36 / 10 / 28 / 34) and starts the `.ese` sound at the spawn, not after the delay;
  Re:CoM's `0x140474EB0` is KH1's stop byte (the particles finish); KH1's frame and Re:CoM's motion time agree within a
  frame; a detached event outlives the motion's end.

## 0.112.0 built: Stun Impact's effect 0 and Zantetsuken's effect 5 (KH1 code `0x8003`)

Owner, 2026-11-04: "Start at the target". This is **corrected from the question's wrong premise** (Checked (KH1 decomp +
files)). KH1 places these two on **Sora himself**, not on the struck enemy, so they are built as KH1 does it.

- **KH1** (Checked (decomp)):
  - the row exec `0x1402A68F0` starts both from Sora's own set with `0x1402AE160(Sora, id | 0x800)`;
  - that links the effect **to Sora** (`0x1402AE7F0` → `0x1402AE880`, link mode 2, the program's attach code `+0x104` =
    `0x8003`, no bit 16), and the link task `0x1402AE910` re-places it every frame: it **follows him**;
  - its lock is 0xFFFF (flag 0x800): it outlives the motion.
- **Where `0x8003` is** (`0x14029EA00` / `0x14029EB20`):
  - on the effect's own actor, Sora, it takes his `+0x134` table;
  - that is his **CLS** table: `.mdls` section 4, bound by `0x140287E40` from the file's fifth offset;
  - the table's body record (`[+0x14]`; group 1's first record) goes through `0x1402B5E50`: its code is evaluated on
    Sora, then the record's `+8` is taken off y (KH1's y runs down, so this is a lift);
  - the rotation is Sora's facing (`+0xD0..+0xF0`).
- **Sora's record** (Checked (files): the player's `xa_ex_0010.mdls`): `{0, 1, code 0x8002, fraction 0.0, lift 85.0,
  r 30.0, h 55.0}`. Code `0x8002` = his `+0x50` plus (0, 0, fraction × 100) turned by his facing. So the point is
  **Sora's position, 0.85 m up**, facing his way.
- **Built** (asset step only: `kh1_fx::sora_body_point`, `Place::SoraBody`):
  - the step reads the record from the player's file;
  - Sora's `PL0001.EET` (and Halloween's `PL0011.EET`) gets a list-A event on Re:CoM's `Root` (bone 0: identity rest at
    the feet, Y up; Checked (files: `PL0001.MDL`)), with offset (0, 0.85, 0);
  - flags 0x28: attached, so it follows (`0x1401A2D50`); linger 0x20, KH1's 0x800; bit 3 as Re:CoM's;
  - frame 0 (KH1's row start), variant 1301, at Stun Impact's 216 (`p001` clip) and Zantetsuken's 218;
  - their sounds 81 / 86 come from `kh1_ex_0010.ese` (stage 2), which Re:CoM's spawn looks up by the set's name
    (`0x1402588B0` / `0x1402577D0` → `0x1402A4A10`, Checked (decomp)).
- **DLL: unchanged.** Requests 216 and 218 were already in `kh1-fx.json`'s effect list (ids 1 and 4).
  - **Per-move cost: nothing of ours.** Re:CoM's motion start binds one more list-A event per move (its 32-slot list),
    and its EET step spawns it.
  - No hook, no call, no site.
- **Limit of the step**: it decodes only a code-`0x8002` record with fraction 0, which is Sora's. Any other code or
  fraction is refused: the two effects are left out and the step logs it.
- **Assumptions:**
  - KH1's `+0x50` is Sora's position (the warp `0x140290CA0` copies `+0x10` into it) and Re:CoM's `Root` follows it;
  - Sora's `+0x374 & 0x2000` (scaled records) is clear;
  - Re:CoM's runtime applies effect 5's start delay (28 ticks) as stage 3 assumes.
- **To check in game** (Combat = KH1, Effects = KH1):
  - Stun Impact's ring and Zantetsuken's flash appear on Sora, at his waist, and move with him;
  - their sounds play (81 / 86);
  - at Effects = Re:CoM nothing changes.

## Stage 4 plan: per-Keyblade trails and sounds (built 0.114.0, see § Stage 4 built)

Research for the stage (2026-11-04); the open choices below go to the owner first.

- **Each Keyblade's data** (Checked (files): btltbl weapon rows, the player's hed):
  - its effect set is its `.wpn` section 0 (trail id 5; Ultima's lifetime 9 and attach code 0x30, the Kingdom Key's 12
    and 0: `recom-m13-kh1-fx.md` § 1);
  - its sounds are its own `<model>.se` (the Kingdom Key's `xw_ex_5010.se`: 40 SCDs), from its weapon row's `+0x34`
    base, 40 ids each (bases every 0x28: Kingdom Key `0x2B10`, Oathkeeper `0x3887`, Ultima `0x2C00`, …; a few ids such
    as 11018–11019 are in several `.se` files);
  - the hit sounds are the same base + row `+0xC..+0xE` (stage 5).
- **Where Re:CoM keeps them:** our converted Keyblade groups 21–36 / 38–43 share Re:CoM's `PL0002.EET` by name
  (`recom-keyblade-models.md` § 10.2; group 2 must stay its first loader). A per-Keyblade trail needs:
  - one EET per group (`PL00GG.EET`, the group row's EET name made its own);
  - its set loaded with it.
- **Sounds:** Sora's `PL0001.SES` is per motion, not per Keyblade. So each Keyblade's ids would go in at their own
  variant (1301 + k), the DLL picking k by the equipped Keyblade, and only the equipped Keyblade's banks registered.
- **Open for the owner:**
  1. Where the 16 KH1 sets load:
     - (a) with Sora in `EF1000.EFF` (always in memory, about 2 MB plus textures; simplest);
     - (b) one new `EF09GG.EFF` per Keyblade group, through a new EFFECT-table row and the group's LINK (only the
       equipped one in memory, as the Keyblade models; more DLL table work).
  2. The trail's length on longer blades: Re:CoM's three nodes fit the Kingdom Key (`w001_055`: −0.11 / −0.78 m on
     `wep0`). KH1's own trail follows its program's tracked point (`pppPObjPoint` / `pppRyjMegaBirth` data, not decoded).
     Options:
     - (a) the same nodes for every Keyblade (Re:CoM's look today);
     - (b) nodes scaled by each Keyblade's KH1 blade reach (W `+0x24`; Ultima −5.6 against −3.7);
     - (c) decode the PPP point ops first (research before building).
  3. The six KH2 Keyblades have no KH1 set: (a) the Kingdom Key's trail; (b) Re:CoM's.

## Stage 4 built (0.114.0): per-Keyblade trails and sounds

Owner's answers (2026-11-04): each Keyblade's KH1 set loads with its own Keyblade group, only when equipped; trail length
and look follow KH1's own trail data (1:1); the six KH2 Keyblades swing with Re:CoM's trail at KH1 Effects.

- **Where the trail's blade comes from** (Checked (decomp + files)):
  - KH1's blade point for code `0x30` (`0x14029EB20`) is `parent + t × (bone − parent)` on Sora's blade bone
    (`0x1401D6320`), with `t = near + (far − near) × fraction` from the equipped weapon row (`+0x28` near, `+0x24` far).
  - Re:CoM's own Kingdom Key trail (`w001_055`) has node B at −0.78 m and node C at 0 on `wep0`. That is the Kingdom Key's
    KH1 far −3.7 and near 0 times `k9`'s 0.2111 m (−0.781), so Square made its nodes from KH1's own row.
  - So each Keyblade's nodes B / C are its own far / near on that segment. Every converted model keeps `PL0002`'s
    skeleton, and KH1 uses Sora's bone for every Keyblade.
- **Each trail program** is the Keyblade's own (`.wpn` section 0, id 5), run by Re:CoM unchanged: its lifetime, look and
  emission are KH1's data.
  - Attach code 0 in 15 sets; `0x30` in Diamond Dust (`xw_ex_5190`) and One-Winged Angel (`xw_ex_5200`, lifetime 9).
    Checked (files: all 18 `.wpn` in the session).
  - For a `0x30` program, node A (the effect's own matrix in Re:CoM's mode-3 spawn) is KH1's blade-bone matrix, the
    segment's t = 1 end: `k9` at offset 0.
  - Code 0 keeps Re:CoM's node A (−0.11 m on `wep0`): Square's choice for the Kingdom Key's identical code-0 program.
- **Correction** (Checked (files: btltbl)): § Stage 4 plan and `recom-m13-kh1-fx.md` § 1 called `xw_ex_5200` "Ultima".
  It is **One-Winged Angel** (item 0x66, far −5.2, sound base 0x3887). Ultima is `xw_ex_5160` (item 0x64, far −5.6, base
  0x2C00), and its trail is like the Kingdom Key's (lifetime 12, attach 0). Oathkeeper's base is 0x380E.
- **Asset step** (`kh1_fx::KeybladeTrail`, `effects::keyblade_set_name` / `group_eff` / `program_attach`, `main.rs`
  `fx_eets`), for each KH1 Keyblade group 21–36:
  - `PLAYER/00GG/PL00GG.EET` (loose): Re:CoM's `PL0002.EET` (variant 0 untouched) plus KH1's events at variant 1301,
    with the trail on its own set `kh1_ex_5xxx.dpx` id 5 at its reach, and Gravity Break's effect 2 at its own blade
    point (fraction 0.8 of its reach);
  - `raw/PLAYER/00GG/PL00GG.EFF`: a container holding the set and its PC textures (`kh1_ex_5xxx_dpx<off>.dds`); the
    sets have no program sounds, so no `.ese` (Checked (files));
  - `kh1-fx.json` `keyblades`: card, group, set, both paths, reach, sound variant.
  - A Keyblade whose files don't read is logged and keeps the shared `PL0002.EET` (the Kingdom Key's trail).
- **Sounds:**
  - every KH1 Keyblade's `<model>.se` SCDs go into the pack, each tagged with the cards whose `.se` holds it (`cards`;
    Sora's own untagged);
  - `PL0001.SES` / `PL0011.SES` get each Keyblade's KH1 sound events at its own variant (`kh1_fx::sound_variant`:
    1301 + card − 1; the Kingdom Key's stays 1301), from its own weapon base (btltbl `+0x34`: KK 0x2B10, Three Wishes
    0x2CC8, …, OWA 0x3887);
  - Re:CoM's whoosh is kept for an id with no SCD, as stage 3 did.
- **DLL:**
  - `win::fx::preload` reads `kh1-fx.json` before the model table, and checks each group's two files next to it;
  - `keyblade_models::plan_table` gives such a group its own EET row and a sixth row, type 0xE `PLAYER/00GG/PL00GG.EFF`.
    Re:CoM's row loader `0x14023B7A0` / `0x14023B9D0` takes any type (Checked (decomp)), so the set loads and unloads
    with the group: only the equipped one is in memory;
  - `fx::variants` passes the equipped Keyblade's (`stats::equipped_card`) own SES variant where it has events for the
    move, else the Kingdom Key's 1301 (the KH2 Keyblades; a Keyblade whose `.se` wasn't found), else 0;
  - `win::sounds::ensure(card)` registers Sora's banks and those of the Keyblade whose sounds play.
- **The six KH2 Keyblades** (38–43): unchanged. They share `PL0002.EET`, with Re:CoM's own trail (`ex_5010` 5 on
  Re:CoM's nodes) at KH1's windows, and play the Kingdom Key's KH1 sounds (stage 3's behaviour; no KH1 `.se` exists for
  them).
- **Per frame: nothing.** Per KH1 motion: one pointer read (the equipped byte) and a few small list lookups. No hook, no
  call site, no byte site.
- **Assumptions:**
  - the code-0 programs read the blade from nodes B and C, as the Kingdom Key's identical program does in Re:CoM;
  - Re:CoM's `.EFF` handler works the same from a PLAYER row as from an EFFECT row (sub-files tagged with the group,
    unloaded with it);
  - the weapon's EET lookup by our group's id finds the group's own `PL00GG.EET`;
  - banks registered earlier in a session stay registered (Re:CoM owns them; at most about 0.9 MB per Keyblade).
- **Not checked:** in game; the real `.se` files (not in the session); a trail's look against KH1's (no KH1 capture).

## The Effects & Sound page built (0.116.0)

Owner, 2026-11-04: "The Effects / Sounds (later Voices) toggles go on a new Settings page".

- The camp's Settings list (Sora's story) has a fourth row, **Effects & Sound** (`keyblades::sub::Row::EffectsSound`).
  It opens a page in our settings pages' own style: Re:CoM's Config screen with our rows (`menu::Page::EffectsSound`).
- The page has two rows, **Effects** and **Sounds**, each Re:CoM / KH1, written to the same ini keys `effects` /
  `sounds`. Voices is left out until voices exist.
- They take effect at once: `win::fx::variants` reads both at each of Sora's KH1 motions.
- Current behaviour, kept: an effect's sound rides on the effect. At Effects = KH1, Sounds = Re:CoM, KH1's finisher
  effects keep their own KH1 effect sounds; at Effects = Re:CoM, Sounds = KH1, Re:CoM's effects keep theirs
  (`recom-config-menu.md` § When each change takes effect). Owner question, below.
- No hook, site or per-frame work. Assumption: Re:CoM's font has an `&` glyph.

## Open (ask at their stage)

- No hit-stop at KH1 (none found in KH1: `recom-m13-kh1-fx.md` § 2; this corrects the clash "hit-stop" in
  `recom-m11-enemies.md`).
- KH1's room dust and splashes.
- Clash EXP.
- Effects without flag 0x800 end on a motion change (KH1's rule): part of stage 3's 1:1 check.
- With Effects = KH1 and Sounds = Re:CoM, should KH1's effects play Re:CoM's effect sounds (today they keep KH1's own,
  which ride on the effect)? And the reverse? Owner question.
