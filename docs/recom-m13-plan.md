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

## Open (ask at their stage)

- No hit-stop at KH1 (none found in KH1: `recom-m13-kh1-fx.md` § 2; this corrects the clash "hit-stop" in
  `recom-m11-enemies.md`).
- KH1's room dust and splashes.
- Clash EXP.
- Effects without flag 0x800 end on a motion change (KH1's rule): part of stage 3's 1:1 check.
