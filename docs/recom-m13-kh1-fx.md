# M13 research: how KH1 FM (Steam) makes its swing trails, hit effects, spell effects, sounds and voices

Research only. Nothing in the repo was changed. Every address is from the **Steam** build (image base
0x140000000), read from the Steam decomp `re/kh1/kh1/` and the Steam exe. Tags:
**Checked (decomp)**, **Checked (exe)** (bytes read with pefile), **Checked (files)** (local KH1 files),
**Assumption**. No decompiled code is copied here.

Files used: `xa_ex_0010.mdls` / `.mset`, `btltbl.bin`, the 19 `xm_*.mag`, the 20 Keyblade `xw_*.wpn`
(`/mnt/user-data/uploads/re/kb-models/`), the 491 enemy and NPC `.mdls`, and `/tmp/magtool/magdis.py` output.
Scratch parsers: `/home/claude/scratch-m13/` (`ppp.py`, `effset.py`, `magfx.py`, `trig.py`, `ppptab.txt`).
They are not in the repo.

---

## 0. The effect system in one picture

Every visual effect in KH1 is a **PPP particle program** inside an **effect set** (KH1 calls the file `.dpx`).
Swing trails, hit sparks, spells, dust and summons all use it. An effect's sound is a field of its program. The
renderer is KH1's own: no Keyblade "trail mesh" code exists outside this system.

| What | Where | Tag |
|---|---|---|
| **Effect set ("dpx")**: u32 `0x82`, +0xC count, then 0x20-byte entries `{u32 blob offset (made a handle at load), u32 program index in blob, u32 effect id, u32 0, u32 maskA, u32 maskB, u64 maskC}`. Then a list of blobs to relocate. | relocate `0x1401E0080`; pick by masks `0x1401DFFE0`; start by id `0x1401E0100` | Checked (decomp + files) |
| **Where sets live**: `.mdls` section 1 (the model's own set, bound to actor `+0x68` by `0x140287E40`); `.wpn` section 0 (Keyblade; the weapon record `+0x18`); `.mag` section 1 (spell); `always.dpx` (global, `0x142D6B4B0`, pointer `0x142D66A00`) and `prize.dpx` (`0x142DAB4B0`), loaded by `0x1402AE790`; the room file's section 7 (surface effects, `0x14232E090`, set in `0x140180000.c` near line 474). | | Checked (decomp + files) |
| **PPP blob** (header u32 `0x96`, u32 program count, offsets; then the tables of shapes, textures and models): program record = 0x110-byte header, then the program. | `0x1401EC770`, `0x14031C2F0` | Checked (decomp + files) |
| **Program header**: +0xF0 cost (the pool caps the sum at 0x100000), +0xF8 skip-when-culled byte, **+0xFC u16 sound id, +0xFE u16 sound volume offset**, **+0x104 attach code**, +0x108 f32 start delay. Program +0 lifetime (×0x1000 per tick), +4 bit 0 = loop, +8 the op list, +0x10 the particle-node chain (each node holds its ops `{u32 op, u16, u16, data, data}`). | `0x14031D2C0`, `0x1401ECB50` | Checked (decomp + files); "+0x108 = delay in frames" is an Assumption |
| **Op table**: 235 named ops × 0x50 bytes at `0x14054FF50` (`pppPoint`, `pppPObjPoint`, `pppRyjMegaBirth`, `pppKeShpTail`…`pppKeShpTailLc`, `pppKeThTp`, `pppDrawShapeField`, `pppDrawMdlSemi3`, `pppKeLnsArndT`, …). Slots: +0x08 per-particle, +0x20 init, +0x28 per-frame, +0x30 per-tick. Full dump in `ppptab.txt`. | | Checked (exe) |
| **Pools**: 256 handles × 0x3C at `0x142D678B0` (a ring); 64 instances × 0xE0 at `0x142678280`. When full, lower-priority instances are evicted (`0x1401EC340`, `0x1401ECAA0`; priority = 3rd argument of the start call: 100000 / 20000 / 10000 / −10000). | `0x1402AE5B0` | Checked (decomp) |
| **Clock**: `0x1401ECB50` adds the frame step (`0x14233FBDC`, 1.0 at 60 fps) to the instance's `+0xBC`, and advances the program **once per 2.0 units**. So **PPP programs tick at 30 Hz** on the 60 fps PC build; the per-frame op pass (`0x14031CBE0`) still runs every frame. Lifetimes below are in 30 Hz ticks. | | Checked (decomp). Assumption: `0x1401ECB50` runs once per 60 Hz frame (the KH2-Combat video note "fades over ~18 frames (30 fps)" agrees) |
| **Start variants**: `0x1402AE650(set, id, prio, point)` at a point; `0x1402ADC20(set, id, actor, code, scale)` on a bone or point; `0x1402AE160(actor, id)` from the actor's own set; `0x1402AE290(actor, set, id)` and `0x1402ADCF0` attach to an actor; `0x1402AE5B0` starts one unplaced. The id's low 11 bits are the effect id. **Flag 0x800** keeps the effect alive across motion changes; **0x1000** makes it follow the actor's tint. | | Checked (decomp) |
| **Attach link** (0x70-byte record, task `0x1402AE910`): +4 mode (1 point + offset, 2 bone matrix, 3 both), +6 motion lock (0xFFFE becomes the current motion on the first frame, and the effect is **killed when the actor's motion changes**; 0xFFFF never), +8 code, +0xC blade fraction, +0x10 offset. Code bit 16 places the effect once and drops the link. The effect also dies when the actor is hidden (`0x140291FF0` = 0). | | Checked (decomp) |
| **Attach codes** (`0x14029EA00`, `0x14029EB20`): `0x8000\|n` = point *n* from the model's point table (actor `+0x134`); `0x2000\|n` = raw bone *n*; `0x30` = the Keyblade blade, with its fraction scaled by W `+0x24` / `+0x28`; any other value = a named bone through `0x14029FAB0`. | | Checked (decomp); "point table = `.mdls` CLS section" is an Assumption |
| **HD textures**: a `.wpn`'s effect textures are HD `.dds` assets tagged by file offset (Kingdom Key `-4310.dds`, flag `0x2000000`, sits in its effect blob). | | Assumption (strong): offset and flag only |

---

## 1. Swing trails

| Address | What it does | Tag |
|---|---|---|
| `0x1402A3980` (row reader) | If the weapon record (actor `+0x14C`) has an effect table (`+0x18`): row byte **+2 ≥ 0** → `0x1402C2310(actor, n)`; row byte +3 ≥ 0 → weapon effect `0x1402ADCF0(table, n, actor, 0x30)` on the blade. Every combo, special, Guard and Counterattack row has **trail 5, weapon effect −1**. | Checked (decomp + files) |
| `0x1402C2310` | Kills any live trail (weapon `+0x28`), copies the table (`+0x18` → `+0x30`), and stores `+0x38 = id \| 0x40000000` ("new this row"). | Checked (decomp) |
| `0x1402C1F20` (from the actor task `0x140297FE0`, every frame) | First call after the row: clears the "new" bit only. After that, on the first frame with **`+0x1CC & 0x800000`** (the mset flag track of type 7) while Sora is shown, it starts the trail `0x1402AE290(actor, table, 5)` and sets `+0x38 = −1`: **one trail per row**. While it lives: when the flag goes off (or actor flags `& 0xC001 ≠ 0xC001`), it sets the instance's stop byte (`+0x91`): no new particles, and it ends when its particles die. | Checked (decomp) |
| Lifetime rule | Trail id 5 has no 0x800 flag, so its link is motion-locked: **a motion change kills the trail at once**. | Checked (decomp) |
| Trail windows | Flag-track type 7 per motion (`kh1-data-values.md` §4, Checked (files)): 200: 4–24, 201: 12–32, 202: 18–30, 203: 8–38, 204: 2–20, 205: 4–20, 206: 6–38, 207: 2–20, 209: 2–54, 210: 22–38, 211: 14–42, 213: 4–24, 214: 2–28, 215: 26–36. None on 208, 212 or 216. The trail starts one frame after the window opens (the "new" bit). | Checked (files + decomp) |
| **Trail data, Kingdom Key** (`xw_ex_5010.wpn`, set entry id 5 → program 3) | Lifetime 12 ticks, **loops**, attach code 0, no sound, cost 576. Ops: `pppEiWfacc`, `pppPoint`, `pppPObjPoint`, **`pppRyjMegaBirth`**: an emitter that spawns particles of a shape from the blob's shape table along the tracked point. It is **not** a ribbon (`pppKeShpTail` is not used by any Keyblade). | Checked (files); the look of MegaBirth is an Assumption (its data, 44 bytes and 1 key, is not decoded) |
| **Per-Keyblade differences** | Ultima (`xw_ex_5200`): trail lifetime **9** ticks, attach code **0x30** (the blade), and its other effects come from a second blob. Each `.wpn` has its own trail program, so colour, texture and length are per Keyblade data. | Checked (files) for the ids, codes and lifetimes; colour, texture and length not decoded |
| Trail sound | None (`+0xFC = 0`). The swing whoosh is a motion trigger (§5). | Checked (files) |

**Not found:** the trail's colour, texture, emission rate and fade. They are in the MegaBirth / PObjPoint
key data and the blob's shape and texture tables. Decoding them needs the op functions (`pppRyjMegaBirth`
`0x1403212F0` / `0x140320DF0` / `0x140321520`, `pppPObjPoint` `0x14032BB00`) read in full; this is the next
research step.

---

## 2. Hit effects

| Address | What it does | Tag |
|---|---|---|
| `0x1402B4060` (an attack hits a body) | Kind: **1** normal; **4** when the target has `& 0x1000080` (searched as 2); **0x40** critical (record `+0xC & 0x20`). Sound byte: +0xA8 (row +0xC) for 1 and 4, **+0xAA (row +0xE) for a critical**. Spark `0x1402B4320` at the contact point, turned to the hit direction (attacker → target, clamped to ±45° of the attacker's facing), scaled by the target stats `+0x64` (`0x1402C22A0`). Sound `0x140296160(owner, 0, byte)` = **weapon `+0x34` + byte**, only on rows (attack flag 0x400000). | Checked (decomp); "+0x64 = scale" is an Assumption |
| `0x1402B4320` | `0x1401DFFE0(attack +8 set, maskA = kind, maskB = 1 << data +0x28, 0)` → `0x1402AE650(set, id, −10000, point)`. The attack's set is the **weapon's** for Sora's rows (`0x1402B3270`), otherwise the **owner's own** `.mdls` set (`0x1402B2DC0`: attack +8 = owner `+0x68`). | Checked (decomp) |
| Keyblade sparks (both `.wpn` checked) | **id 0** maskA 0x81: normal body hit; **id 1** 0x102: kinds 2 (from 4) and 0x100 (a wall); **id 2** 0x41 (Ultima 0xC1): critical. All have maskB 1 (data +0x28 = 0 on every Sora row). Lifetimes: Kingdom Key 24 / 30 / 24 ticks, Ultima 30 / 30 / 30. **No sound on the spark**: the hit sound is the row byte. | Checked (files) |
| `0x1402B3A00` (a wall or the ground) | Surface class (`0x140191D70`): 0 → **0x100**, 5 → **8**, else **2**. Sound weapon + row **+0xD** for 0x100 and 2. Spark: same pick (Keyblade id 1; **none for 8**), turned by −normal. Clash record kind 6 → recoil 0x6E, except for kind 8. | Checked (decomp) |
| `0x1402B4A20` / `0x1402B4BC0` (attack meets attack: clash, guard) | Kind **0x10**, or **0x20** when ≥ 40 priority under. Each side's own spark for that kind. The Keyblade has no 0x10 / 0x20 entry, so Sora's Guard and clash give **`always.dpx` id 10 (0x10) or 11 (0x20)** at the midpoint of the two spheres (prio 20000). | Checked (decomp + files) |
| **Guard spark** | As above: the Guard row (0x3D, data 0x4DA8, +0x28 = 0) clashes; the spark is `always.dpx` 10 or 11 unless the enemy's own set has a 0x10 / 0x20 entry. The clink is sound `0x2AFD` on the recoil motion (`re/notes/guard-counter-sounds.md`). | Checked (decomp + files) |
| **When Sora is hit** | The spark comes from the **enemy's own `.mdls` set**, and **the hit sound rides on that spark** (`+0xFC`). Shadow `xa_ex_2020`: id 4 (maskB 1, sound 1008), id 5 (maskB 2, sound 1009), id 6 (maskB 4, sound 1010); maskB = 1 << the attack's data +0x28. Soldier-type `xa_ex_2010`: ids 4–8, sounds 1001–1005. | Checked (decomp + files) |
| Sora's hurt extras | `0x14029BBD0`: pad rumble `0x14028F8D0` (kind 3 → pattern 1, kinds 4–5 → 0, 6 → 6, 7 → 5; `btlvibe.dat`). With **Aero** on (`+0x88 & 8`) and damage taken: the Aero `.mag`'s effect **id 5** at point 0x8002. Hurt voices §6. | Checked (decomp) |
| **Hit-stop** | **None found.** The global time scale `0x14233FBE4` is written only at init and on a debug path (`0x140170000.c` ~12020). The "10-frame hit-stop + rumble on a clash" in `docs/recom-m11-enemies.md` (line ~510 and owner question 1) cites `0x14028C660(10)`, which is a **play-statistics counter** (`DAT_142868BA0 + 0x848 + 2n`; trophies through `0x1400D3920`). `0x140270990` / `0x140284990` there are an **EXP pop-up and EXP grant**, not a hit-stop. A per-actor freeze on hit was not traced. | Checked (decomp) for what those functions do; "no hit-stop" is an Assumption (absence not proven) |

---

## 3. Effect system: loader, player, bones, lifetime, per-character files

See §0. In addition:

| Item | Detail | Tag |
|---|---|---|
| Sora's own set (`xa_ex_0010.mdls` section 1) | 8 programs (ids 0–7) with **sounds 81–88**. Attach codes 0x8003, 0x18001, 0, 0x18001, 0x18001, 0x8003, bone 8, 0x18001. Lifetimes 60 / 45 / 30 / 45 / 40 / 30 / 50 / 35 ticks. | Checked (files) |
| Who plays Sora's set | Row exec `0x1402A68F0` (triple table on the stack): **row 3 Stun Impact → ids 0, 1; row 4 Gravity Break → 3, plus id 2 on Sora; row 5 Zantetsuken → 4, 5; row 7 Ripple Drive → 6, 7**, all with flag 0x800. Also the motion-start hook `0x1402A0120` / `0x14029FC40`: request-map word bits 16–27 = effect id + 1 (empty in Sora's mset). | Checked (decomp + files) |
| Motion effect triggers (mset flag-track **type 4**) | `0x1402C2660(actor, key u16 +4, key u16 +6 = bone)`: surface effects (dust, splash) from the **room's set** (`0x14232E090`), picked by `1 << surface` (actor `+0x39E`), key bits, and `1 << class[char]` (table `0x14052A090`); coloured by actor `+0x350`. Sora's walk / run / jump / land / roll keys use bones 0x16 / 0x30 / 0x4E / 0xCE. | Checked (decomp + files); the bone names are an Assumption |
| Global effects (`always.dpx`) | 10 / 11 clash, 0x14 (actor task `0x140297FE0`, state 8), 0x15 (`0x1402B5360`, called from the EXP grant: level-up, Assumption), 0x16 (`0x1402ABEA0` / `0x1402AC080`), 0x1C (`0x140289C30`), ids 3–7 by kind (`0x1402C2B50`, table `0x14052A0A8`: 3, 4, 5, 6, 7, 4, 5, 6; item / prize handover, Assumption). | Checked (decomp + exe); the meanings are Assumptions |

---

## 4. Spell effects (`xm_*.mag`: section 0 = the script, section 1 = the effect set)

Each entry is effect id → sound id, with the lifetime in 30 Hz ticks; **L** = loop; masks are given where set.
Ids 100+ repeat 0–5 (the script adds a base of 0 or 100, `ind+0x44` / `sp+0x84`; what picks it was not
traced). All values Checked (files: `magfx.py`). Every spell's sound is **its effect's own sound**: the
scripts never call sound sys 67 or voice sys 168 (Checked, `magdis` output).

| Spell | Lv1 | Lv2 | Lv3 | Script use (magdis) |
|---|---|---|---|---|
| Fire | 0→100 (32), 1→101 (75 L), 2→133 (40, m102), 3→102 (50, m81), 4→– (41) | 119, 120, 134, 121 (60), – | 141, 142, 135, 143, – | 0 on the casting bone (sys 21); 1 = the ball (sys 15); 4 at a point (sys 74); 2 / 3 = the ball's hit sparks (hard / body) |
| Blizzard | 0→103 (40), 1→104 (41 L), 2→105 (m81), 3→136 (m102), 4→– (24), 5→104 (30 L) | 122, 123, 124, 137, –, – | 144, 145, 146, 138, –, – | shards: sys 15 (id from data); 0 bone; 4 point |
| Thunder | 0→106 (45), 1→107 (45), 2→108 (60, m80), 3→– (33) | 125, 126, 127 (55) | 147, 148, 149 | 1 loop marker; 2 at the strike point; 0 bone; 3 point |
| Cure | 0→109 (47), 1→110 (65, m1) | 131, 132 (60) | 139, 140 (55) | 0 loop (charge); 1 on the target's bone |
| Gravity | 0→111 (35), 1→– (45), 2→112 (60), 3→– (300), 4→113, 5→113 | 150, –, 151, – (315), 152, 152 | 154, –, 155, –, 156, 156 | 3 / 4 / 5 / 2 / 1 loops; 0 bone |
| Stop | 0→128 (45), 1→– (20), 2→129 (30, m1), 3→130 (30, m80) | 170, –, 171, 172 | 173, –, 174, 175 | 2 at a point; 0 on bone 12; per-enemy effect from data |
| Aero (`xm_air01` only) | 0→95 (30, code 0x30), 1→– (45), 2→96 (20), 3→97 (45 L, 0x8001), 4→– (45 L), 5→98 (30, 0x8001), 12→– (10), 21→99 (25, m81/2) | | | 0 and 12 on point 0x8002; 2 loop; 1 at a point; 5 = barrier hit (`0x14029BBD0`) |

- The spell sounds 95–175 are **below 1000**, so they go to sound bank 2 (§5). Checked (decomp).
- Spell voice: the command start `0x1402A3600` plays voice group **7 + spell index** (Fire 7 … Aero 13),
  with the spell's target as `other` (§6). Checked (decomp + files).

---

## 5. Sounds

| Address | What it does | Tag |
|---|---|---|
| `0x140179AC0` (`/B30` `/BA0` `/C10` `/C80` are variants) | Queues `{id, owner key, position pointer, volume offset}` into 32-byte records (`0x14232D560`, count `0x14232D554`). Bank field = **2 for id < 1000, −1 for id ≥ 1000**. The variants set the falloff class (+0x1E: 0 / 2 / 1 …). | Checked (decomp) |
| `0x1401791E0` (per frame) | Plays and updates every queued record. Volume: `0x140178030` gives L and R = **16383 − d² / K** (clamped 0..0x3FFF), d = distance to two ear points = camera × (±7500, 0, 1500) (mode 5: (±400, 0, 400)) (`0x1404DC3C0`..`0x1404DC400`). **K = 10000** by default (`0x14232D500` / `0x14232D520`), overridden per room (`0x140170000.c` ~10477). Volume offset 0x3FFF = full and not positional. Ids `0xE2D8`..`0xE2DB` get a floor of 0x2400. | Checked (decomp + exe) |
| `0x1402961F0` | 128 sound slots × 0x30 at `0x142D557E0`. One instance per (id, owner): the same pair stops the old one first (`0x140179D90`). | Checked (decomp) |
| Motion triggers (flag-track **type 6**) `0x14029FB30` → `0x140295420(actor, op, arg, bit)` | Skipped when the actor's `+0x9C` gates the bit, sound is muted (`0x142867364 & 0x200`), the game is paused (`0x14017AF20`), or the actor is not drawn. **op 0**: absolute id `arg`. **op 0x19**: weapon `+0x34` + `arg`. **Other ops**: code = byte `0x140528A88[op − 1]` (Checked, exe: `04 05 00 01 09 0A 0B … 18 1A 1B 1C 1D 00 19`), then `0x140295B40(actor, code)`. Code 10 is skipped on water (surface 0xB / 0xC). | Checked (decomp + exe) |
| `0x140295B40` (surface sounds) | id = **`footstep.bin`** u16 `[world (0x14233FE84)][group][surface (actor +0x39E)]` + code. The table is loaded at `0x142D56FE0` by `0x140294CC0`, 15 u16 per row; group = char 0 / 1 / 2, char 4 → 1, 0x11 → 2, else 3. Codes 0–7 = footsteps (left and right alternate through `+0x39C`). Codes 0xE–0x10 and 0x11–0x17 come from the actor or model (`+0x2DE` / `+0x30C`, model `+0x90` / `+0x5C`). Codes 0x1A–0x1D use row entry 14. Two more tables in the same file (`0x142D579B6`, `0x142D58D68`) are read by `0x140295E00`. | Checked (decomp); the table contents need the file |
| **Sora's movement sounds** (Checked, files `trig.py`) | walk (req 1): f16 op 2 (code 5), f30 weapon + 0x15, f46 op 1 (code 4). run (2): f9 weapon + 0xE, f10 op 4 (code 1), f27 weapon + 0x16 / 0xD, f28 op 3 (code 0). **jump** (4): f2 op 5 (**code 9**), f14.5 weapon + 0x15. **land** (7 / 0xC): f0 op 6 (**code 10**). high jump (9): f4 op 7 (code 0xB), f8.82 weapon + 0xE, f27.54 weapon + 0x15. **roll** (0xDC): f2 code 9, f6 op 0x1A (code 0x19), f22 code 4, f28 code 1. Glide (0x71–0x74): **no triggers**. Hurt and knock-back (0x48–0x51, 0x6E–0x70): absolute ids `0x2AFD`..`0x2B09`. Swings 200–206: f0 weapon + 7 / 8 / 9 / 0xA / 0xB / 0xC / 0x11, plus steps (`guard-counter-sounds.md` §1b). Code-driven: landing code 8 (`0x1402A9740`, `0x1402CC1A0`), water code 10 (`0x1402C12B0`, `0x1402C21D0`). | Checked (files + decomp) |
| Hit sounds | weapon `+0x34` + row +0xC / +0xD / +0xE (§2). Each Keyblade owns 40 ids: Kingdom Key **0x2B10**, Oathkeeper 0x3887, Ultima 0x2C00, … (`kh1_weapons.py`). | Checked (files + decomp) |
| Effect sounds | Program `+0xFC` id with `+0xFE` volume, played when the effect starts (after its delay) at the effect; **paused and resumed** with it (`0x140179A80`); **faded out** when the effect is killed (`0x1401EC470` → `0x140179A20(id, handle, 0x3C)`). | Checked (decomp) |
| Script sound | `sys0.67` = `0x1402BC290(id, owner, pos)` → `0x1402955E0`. | Checked (exe + decomp) |
| **Where sounds live** | Model sounds: `.mdls` **section 6 = a "SeSep" list** (Sora: `0x2AF9`–`0x2B09`, `0x2CF6`, `0x2CF7`, `0x2B0F`, `0x385E`; enemies 1001+ and 5000+), registered by `0x140296480` / `0x140179DD0`; on PC their audio is the `.scd` HD assets of the `.mdls`. World pack **`amusic/se<ww>.dat`** (`0x140178E80`, codes at `0x1404DCCA0`: dh, di, dc, tw, aw, tz, po, xx, al, lm, nm, he, pi, pp, xx, pc, ew), registered to bank −1. System pack **`amusic/se000.dat`** (`0x1401791D0`). The PC loader maps names containing `se0000` → bank 0 and `se7777` → bank 7777 (`0x140120000.c` ~10998; `sound/se7777.win32.scd`). | Checked (decomp + files) for the paths and lists. Assumptions: the Keyblade ids `0x2B10`+ and the effect ids < 1000 are in the world and system packs (they are in no local `.mdls`, `.wpn` or `.mag`: Checked (files)), and bank 2 = `se000` |
| Menu sounds | **Not traced.** They likely go to the system bank directly. | – |

---

## 6. Voices

| Address | What it does | Tag |
|---|---|---|
| `0x1402901F0(actor, group, other)` | Table = btltbl + `0x140528860[char]` (Sora `0xBC48`, Donald `0xBD28`, Goofy `0xBE58`, then `0xBF88`, `0xC0B8`…; Checked exe). A group is 11 bytes: priority, then 5 × {clip, weight}. Roll r = rand × 100. A pair with weight > 0: r −= weight, and the clip is taken when r < 0. Weight 0: the clip joins an equal-chance pool, drawn if nothing was taken. **0xFD**: if `other` is a party member (char < 10) who is not the speaker and r < weight, the clip played is **`other`'s character id** (the name call, Assumption: "Donald!" / "Goofy!"). 0xFE: silence. 0xFF: empty. One room (world 3, `fe8c` 2, `fe90` 2) clears `other`. | Checked (decomp) |
| `0x140290930(actor, clip, prio)` | Per-speaker slot (4 × 0x28 at `0x142D371D0`, chosen by stats `+0xBD`): when a voice is playing, its level is 2 if it is the same actor and 1 otherwise. The new voice plays only if that level < the group priority. **No cooldown and no random skip** beyond the group weights. The mouth is always moved (`0x1402A10B0(actor, 0x23)`). | Checked (decomp) |
| `0x140290080` | Party member: bank = `0x142D37270[char]`. Others: the clip index into actor `+0x4A8` (`.mdls` **section 7**, count + offsets; bound in `0x140287E40`). Gated by `0x142867364 & 0x100000`. | Checked (decomp) |
| Bank files `0x140290390` / `0x1402905F0` | **`voice/<world>_<name>.vsb`** for Sora, Donald and Goofy (world code + `_` + `sora` / `donald` / `goofy`; names at `0x140528B30`), `voice/<name>.vsb` for the guests. Sora has 1 bank slot; the others share 2. On PC the loader rewrites `…_vsb` → `.win32.scd` (`0x1400D0000.c` ~2259). Event voices: `voice/event/%s_%03d.vset` with `vset_table.bin`. | Checked (decomp); the exact PC file name is an Assumption |
| Callers and groups | Attack row +0xF at the row start (`0x1402A3980`). Hurt (`0x14029BBD0`): HP ≤ 0 → **2**; kind 3 → **0**; kinds 4–5 → **1**; a non-Sora member crossing half HP → **20**. Hit by a party member → **23**. Status recovery → **24 / 25** (`0x14029CA60`). Command start → its command-table voice, or **7 + spell** (`0x1402A3600`). Summons → 5, 15, 17 (`0x14039…`). Script → **sys0.168** (clip) and **sys0.216** (Sora's group 2). | Checked (decomp) |
| **Sora's groups** (btltbl `0xBC48`; **20 groups**, so ids ≥ 20 read Donald's table) | 0 {10, 11, 12}; 1 {13, 14}; 2 {16}; 3 {FD:100, 18:50, 19:50}; 4 swings {21, 22, 23, 24, 28}; 5 finishers {25, 26, 27, 33}; 6 {29, 31, 32}; 7 Fire {40:75, 30:25}; 8 Blizzard {41:75, 30:25}; 9 Thunder {42:75, 18:25}; 10 Cure {FD:100, 43:75, 19:25}; 11 Gravity {44:75, 30}; 12 Stop {45:75, 18}; 13 Aero {FD:100, 46:50, 47:50}; 14 {34, 36, 29, 31}; 15 {35, 38}; 16 {FD:100}, prio 2; 17 summon {37:75, 30:25}; 18 {38}; 19 Guard {24}. All prio 3 except 16. | Checked (files) |
| Donald / Goofy | Prio-2 groups with explicit silence (FE:60) → about 40 % chance to speak; groups 20–26 used. | Checked (files) |

---

## 7. Enemies (brief)

- Script syscalls (table `0x140529180`, scanned against the decomp): effects **sys 15** (start, unplaced),
  **16** position, **55** rotation, **26** fade, **97 / 154** kill, **21** on a bone or point (`0x1402ADC20`),
  **74** at a point (`0x1402AE650`), **75** own set by id (`0x1402AE160`), **106 / 109 / 209 / 225 / 250**
  other start variants. **Sound sys 67**; **voice sys 168** (clip), **216** (Sora's KO group). Checked (exe + decomp).
- Hit sparks: the enemy's own set, picked by kind and the attack's data +0x28 bit; **each spark carries its hit
  sound** (§2). Surface sounds: `0x140295B40` / `0x140295E00` with `footstep.bin`. Voices: `.mdls` section 7
  via actor `+0x4A8`. Model sounds: `.mdls` section 6 SeSep. Checked (decomp + files).

---

## 8. KH1 vs Re:CoM effect files: same DPX format? (the coordinator's question)

Files compared: Re:CoM `EFFECT_1000_EF1000.EFF` (`ex_0010.dpx`, `ex_5010.dpx`, `ex_0010.ese`) against KH1
`xw_ex_5010.wpn` section 0 and `xa_ex_0010.mdls` section 1. Re:CoM exe: `re/recom-exe/recomsteam.exe`; Re:CoM
decomp: `re/out/recom/`. Only layout facts are given here.

**Answer: yes, it is the same format and version. KH1 keeps its effects as DPX sets inside the `.mdls`
(section 1), the `.wpn` (section 0) and the `.mag` (section 1). There are no `.pax` files in KH1.**

| Point | KH1 | Re:CoM | Tag |
|---|---|---|---|
| Container | an `.mdls` / `.wpn` / `.mag` section; global files `always.dpx` and `prize.dpx` | an `.EFF` holding `{u32 off, char[0x18] name, u32 size \| 0x80000000}` rows, with `.dpx` sets and `.ese` sound tables (`texcommon.dpd` holds shared textures) | Checked (files) |
| Set header | u32 `0x82`, two zero u32, u32 count, then 0x20-byte entries `{blob off, program index, id, 0, maskA, maskB, u64 maskC}`, then the relocation list (count + blob offsets) | the same | Checked (files) |
| PPP blob | u32 `0x96`, program count, offsets; then 4 tables (Kingdom Key: 1 / 6 / 5 / 7 entries); each program = 0x110-byte header + program (lifetime, flags, op list, node chain) | the same | Checked (files) |
| **Kingdom Key** | `xw_ex_5010.wpn` section 0 (0x9980 bytes with padding) | `ex_5010.dpx` (0x9950 bytes) is **byte-identical** to KH1's data over its whole length; KH1's extra 0x30 bytes are zero padding to 0x80 | Checked (files) |
| HD texture tags | KH1 `-4310.dds` = section base 0x80 + 0x4290 | Re:CoM `ex_5010_dpx4290.dds`: the **same offset inside the set** | Checked (files: names and offsets only) |
| **Sora's set** | 8 programs (ids 0–7); sounds 81–88 in program `+0xFC` | `ex_0010.dpx`: **6 programs (ids 0–5)** with KH1's op lists, attach codes (0x8003, 0x18001, 0, 0x18001, 0x18001, 0x8003) and lifetimes (60 / 45 / 30 / 45 / 40 / 30) for KH1 programs 0–5; **`+0xFC` = 0**; the bytes differ elsewhere (Re:CoM edited data). KH1's ids 6 and 7 (Ripple Drive) are absent. | Checked (files) |
| **Effect sound** | in the program header (`+0xFC` id, `+0xFE` volume) | in a separate **`.ese`**: `"ESE\0"`, u32 0, u32 0, u16 1, u16 count, then 8-byte rows `{u16 effect id, u16 group, u16 index, u8 volume, u8}` (Re:CoM sound id = group × 1000 + index). `ex_0010.ese` maps ids 0–5 → **1056–1061**. | Checked (files); row layout agrees with `guard-counter-sounds.md` §3 (Re:CoM decomp) |
| **Op table** (op index → function) | 235 entries × 0x50 at `0x14054FF50` | 242 entries × 0x50 at **`0x140859DD0`**: **the first 235 names match KH1's in order**; Re:CoM adds `pppVertexApLcLoop`, `pppSnoScaleAll`, `pppSnoMdlDttNorm`, `pppRyjMegaBirthUserCtrl` / `2` / `3`, `pppEiDrawShipoly` | Checked (exe, both) |
| Program clock | +0x1000 per tick, capped at 0x6FFFF000 (`0x14031C4F0`), one tick per 2.0 frame-step units = 30 Hz | +0x1000 per tick, same cap (`0x140479460` and its neighbours) | Checked (decomp); **Re:CoM's tick rate (per frame or per 2 frames) not checked** |

What this means for M13 (facts only; the plan is yours to make):

- KH1's effect sets can go into a Re:CoM `.EFF` **as they are**. Re:CoM already ships KH1's Kingdom Key set
  unchanged. Every op index a KH1 program uses exists in Re:CoM's table with the same name. That the functions
  behind the names behave the same is an **Assumption (strong)**: the same middleware, with an unchanged
  Kingdom Key file.
- The one data change: **KH1's program `+0xFC` sounds must become `.ese` rows**. Whether Re:CoM's player still
  reads `+0xFC` was not checked (its shipped files have 0 there). The KH1 ids (81–88, 95–175, 1001+, …) also need
  to be in a Re:CoM sound bank, by the KH1 → Re:CoM id mapping.
- Re:CoM's Sora set (`ex_0010.dpx`) is **an edited KH1 set**, not KH1's. KH1's Sora set (8 programs, with its
  Ripple Drive effects 6 and 7) would be a new `.dpx` (or new ids), so Re:CoM's own stays intact.
- Things still to check before relying on this: Re:CoM's tick rate vs KH1's 30 Hz; the meaning of Re:CoM's
  masks and attach codes (`0x8003`, `0x18001`) on Re:CoM's actors (the same codes appear in its files);
  `.mag` / `always.dpx` sets against Re:CoM's `always.dpx` (in `EF0100`, not in this session); and whether
  Re:CoM's texture tables differ when the textures are in `texcommon.dpd`.

## Related KH1 mechanics found (not asked; please confirm before adding any)

1. **Clash EXP** (`0x1402B4BC0` → `0x140270990` pop-up + `0x140284990` grant): when Sora's attack beats an enemy
   attack in a clash, the party gains EXP = that attack's `+0xB0` × (ability count + 1), unless ability 0x40 is
   on. It affects progression, not feel. Checked (decomp); the ability's meaning is an Assumption.
2. **Motion-locked effects**: any effect without flag 0x800 dies on a motion change. This matters for 1:1
   trail and spark length in cancels.
3. **Effect budget and eviction by priority**: under load, KH1 drops low-priority sparks. Minor.
4. **Room-specific dust and splash, and room sound falloff K**: per-room data. Visible in the field.
5. **Hit-stop**: none found in KH1. If Re:CoM has one, KH1 setting = none (please confirm).

## What I could not check

- The PPP op data (colour, texture, size, emission) for the trail and sparks. Needs the op functions read in full.
  A renderer-level port also needs the shape and texture tables decoded.
- Bank 2 = `se000` and where ids `0x2B10`+ live. Menu sounds. The exact PC voice file names.
- Who sets the spell base 0 / 100. The meanings of always.dpx ids 0x14 / 0x16 / 0x1C.
- Anything heard in game: every sound role here comes from where the sound is played (no audio here).

## Files needed from the owner's PC (KH1 extract)

1. `always.dpx`, `prize.dpx` (global sparks, the clash and Guard spark 10 / 11).
2. `footstep.bin`.
3. `amusic/se000.dat` and the `amusic/se<ww>.dat` packs, with their `.win32.scd` HD assets (for example
   `sound/se0000.win32.scd`), plus a search of the whole install for SeSep ids `0x2B10`–`0x2B37` (Kingdom Key).
4. `voice/<ww>_sora.vsb` → its `.scd` for one world (Traverse Town `tw`), and Donald's and Goofy's.
5. Sora's `xa_ex_0010.mdls` HD `.scd` assets (section 6 sounds `0x2AF9`+) and one room or map file (its section
   7 surface set).
6. Re:CoM `EFFECT_0100_EF0100.EFF` (`always.dpx` / `always.ese`), to compare it with KH1's `always.dpx`.
7. A short KH1 capture (video with audio) of a 3-hit combo on a Shadow, a Guard, and one Fire, to tie roles to ids.

## Epic → Steam conversions (KH2-Combat notes re-found in the Steam decomp)

`0x14028E060` → **`0x1402901F0`**; `0x14028E7A0` → **`0x140290930`**; table `0x140529560` → **`0x140528860`**;
voice slot `0x142D37BD0` → **`0x142D371D0`**; `0x1402A17F0` → **`0x1402A3980`**; spark `0x1402B2190` →
**`0x1402B4320`**; effect pick `0x1401DDE80` → **`0x1401DFFE0`**. Each was confirmed by reading the Steam function.

Correction to `kh2c/docs/kh1-voice.md`: clip 0xFD does not play `other`'s line. It plays **the speaker's clip
numbered by `other`'s character id**.
