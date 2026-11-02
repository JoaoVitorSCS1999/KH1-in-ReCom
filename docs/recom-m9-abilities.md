# M9 stage 9a: KH1's combo-modifier abilities and the ability state

Steam addresses (KH1 and Re:CoM images at `0x140000000`). Tags: Checked (decomp / exe / files /
tests) or Assumption. Code: `crates/kh1-recom/src/abilities.rs` (pure), `win/abilities.rs`,
`attack.rs` (`MODIFIER_ROWS`, `kh1_table`, `recom_kind_of`, `recom_body_bone`, `kh1_damage_at`),
`rows.rs` (`KH1_ONLY`, `Extension::with_recom`), `win/attack.rs` (pair dispatch, enemy hit),
`crates/kh1-core/src/data.rs` (the modifiers' motions and records), `combo.rs` (tests),
`crates/kh1-recom-assets/src/ses.rs` (`add_clips`).

Owner's choice (M7–M9 choices, `design-philosophy-instructions.md`): abilities unlock with Re:CoM
(its level-up sleights Sliding Dash, Blitz, Stun Impact, Zantetsuken, Strike Raid, Sonic Blade,
Ars Arcanum, Ragnarok); the five with no Re:CoM source (Vortex, Aerial Sweep, Hurricane Blast,
Ripple Drive, Gravity Break) at KH1's learn levels on Re:CoM's level; an equip screen with KH1's
AP next (9b).

## What 9a builds

At Combat = KH1, in Sora's story:

1. **The ability state** in our block of Re:CoM's save (below): unlocked and equipped bits, AP
   reserved for 9b.
2. **Unlocks**: Re:CoM's own learned-sleight state for the eight it has; KH1's learn level on
   Re:CoM's level for Vortex, Aerial Sweep, Hurricane Blast, Ripple Drive. **Gravity Break has no
   source yet** (owner question).
3. **Equipped = unlocked** until 9b's equip screen: a new unlock is equipped at once.
4. The equipped modifiers become KH1's ability bits in the `kh1_core::Abilities` the KH1 frame
   passes to **KH1's own pickers** (`0x1402A6EE0` ground, `0x1402A6C70` air), which choose KH1's rows.
5. **The rows** (KH1 rows 0, 3–7, 9, 10, 15) in Re:CoM's attack-row layout through `kh1_row_bytes`,
   each playing the modifier's own clip on Re:CoM's attack action `0xA`.
6. **Hit shapes and damage**: Stun Impact's body sphere and Ripple Drive's hand sphere through
   Re:CoM's own collision primitives; KH1's damage including the % of HP element and Magic × 3;
   KH1's stun element mapped to Re:CoM's stun kind.

The movement abilities (High Jump, Glide, Superglide, Dodge Roll, Guard, Counterattack, Combo Plus,
Air Combo Plus, Combo Master) stay in `kh1-recom.ini` until 9b.

## KH1 (Steam)

### Where the modifiers act. Checked (decomp; `kh1-attack-combo.md`, `kh1_core::combo`)

| Ability (id, bit `0x142D5EC10`) | Picker condition | Row / motion | Hit (frames) | Record |
|---|---|---|---|---|
| Stun Impact (57, `0x8000000`) | finisher press, draw r < 0.3 | 3 / 216 | 34–50 | `0x4EF8`: stun element 10, kind 3, **bone 8**, power 4, radius **250** |
| Gravity Break (58, `0x10000000`) | finisher, next 0.3 slice | 4 / 217 | 32–38 | `0x4F68`: **% of HP** element 8, reaction 8, power 40 (+ Strength), crit 200 |
| Zantetsuken (59, `0x20000000`) | finisher, next 0.3 slice | 5 / 218 | 36–50 | `0x4FD8`: % of HP, no stat term (source 2), power **250**, crit 200 |
| Blitz (20, `0x8`) | the plain finisher flagged and no side enemy (f[11] = 0) | 6 / 210 | 30–38 | `0x4D38`: launch, power 6, radius 10 |
| Ripple Drive (56, `0x4000000`) | finisher press (beside the finisher) | 7 / 215 | 34–46 | `0x4E88`: launch, power 10 + **max MP × 3** (source 4), **frac 0 / 0**, radius **200** |
| Sliding Dash (54, `0x1000000`) | locked target at gap ≥ 300, under 160 above | 9 / 208 | 26–58 | `0x50B8`: flinch, radius 30 |
| Vortex (17, `0x1`) | the lunge (with row 14), or the side swing in the scan (f[10] = f[11]) | 10 / 211 | 18–42 | `0x4CC8`: flinch, power 2 |
| Aerial Sweep (18, `0x2`) | the jump-up attack (target 160..reach + 150 above, gap < 300) | 0 / 214 | 6–28 | `0x4CC8` |
| Hurricane Blast (55, `0x2000000`) | air finisher, target not more than 100 above, state 2 | 15 / 209 | 4–54 | `0x5048`: launch, power 6, hop 40, air speed 12 |

- Lowest flagged row wins (`first_flagged`): Stun Impact (3) < Gravity Break (4) < Zantetsuken (5) <
  Blitz (6) < Ripple Drive (7) < finisher (8); Vortex (10) < side swing (11) < lunge (14).
- Motions, windows, root travel and records: Checked (files: `xa_ex_0010.mset`, `btltbl.bin`;
  `kh1-data`'s `matches_kh1_core_defaults` and `documented_numbers` compare `kh1_core`'s
  defaults with the files frame for frame, 19 rows now). Root travel at the end: Sliding Dash
  654.24, Blitz 200, Vortex 226.61, Ripple Drive 76.96 (out 107.81 and back), Stun Impact 44,
  Gravity Break 150, Zantetsuken 350; Aerial Sweep has the root flag and no travel; Hurricane
  Blast none (air).
- Found on the way: KH1's recoil 0x6E (entry 72) has a trail flag 0–6 that `kh1_core` lacked
  (added; nothing reads trail flags at run time).
- Effects `0x1402AE160` (rows 3, 4, 5, 7) and `0x1402ADC20` (row 4) are M13: not built.

### Learning. Checked (decomp + files)

- Level-up `0x1402B51B0`: Sora's ability list is btltbl `*(u32*)(0x140528D10 + 4 × save[0x48C])`:
  `0x3B28`, `0x3B90`, `0x3BF8`. Each level-up reads the entry at the level cursor (`record + 0x38`)
  − 1 and grants it (0 = nothing, 1–7 = stat / slot bonuses, else ability `byte & 0x7F` via
  `0x140284E10`). Entry i is level i + 2: the same rule as the bonus list `0x3AC0`, whose MP + 1
  entries fall on 12 / 24 / 36 / 48 / 60 (the rule `crate::mp` already uses). Assumption: the
  cursor starts at 1 on a new game (not traced).
- **Which list:** `save[0x48C]` is set by event syscall 479 (`0x1401B0380` → `0x140285730`), the
  EXP curve `save[0x48D]` by 480. Assumption: 0x48C is the kept item, 0 / 1 / 2 = Sword / Shield /
  Rod (Stage 13's reading of the Dive script). The mod's Dive path keeps the Rod (decision 18),
  so the **Rod list `0x3BF8`** is used.
- Rod list, the modifiers: Ripple Drive **12**, Hurricane Blast **18**, Aerial Sweep **36**,
  Vortex **84** (also Stun Impact 6, Sliding Dash 51, Blitz 78 — those unlock with Re:CoM here).
  Sword list: Sliding Dash 6, Hurricane Blast 27, Blitz 33, Ripple Drive 39, Stun Impact 51,
  Vortex 57, Aerial Sweep 60. Shield list: Stun Impact 18, Sliding Dash 39, Hurricane Blast 51,
  Blitz 60, Ripple Drive 69, Vortex 78, Aerial Sweep 84.
- **Gravity Break (58) and Zantetsuken (59) are in none of the three lists** (no `0xBA` / `0xBB`).
  KH1 gives them some other way (events / rewards: `0x140284E10`'s other callers, the event
  syscalls `0x1401B1110` / `0x1401B13E0` and `0x14030B9F0`; not traced).
- KH1 adds a learned ability **unequipped** (`0x140284E10` writes `id | 0x80`; the list's bit 7 =
  not equipped).
- AP costs: btltbl `0x33D8`, 12 bytes per ability, u16 cost at +0, row = id − 1 (the costs match:
  Vortex 1, Aerial Sweep 2, Blitz 2, Sliding Dash 1, Hurricane Blast 2, Ripple Drive 3, Stun
  Impact 2, Gravity Break 2, Zantetsuken 2). Checked (files); the id − 1 indexing: Assumption
  (the lookup `0x14028F950` only adds btltbl's base). For 9b.

## Re:CoM (Steam)

- **Learned sleights**: two bits per sleight id at `*0x14087CC08 = save + 0x2840`
  (`0x140230000` area: `DAT_14087cc08 = DAT_14087b110 + 0x2840`). `0x14023AB00(u16 id)`: in Sora's
  story (`0x1401AAB00` ≠ 0) ids 4..0x56 and 0x5A..0x5C, state ≠ 0 → learned. Checked (decomp +
  exe).
- **Level-up sleights** (`0x14010EF10`'s table): level 2 → 4, 7 → 6, 12 → 8, 17 → 5, 22 → 7,
  27 → 9, 32 → 0x27, 37 → 0x29, 42 → 10, 47 → 0x33, 52 → 0xB, 57 → 0x22. Checked (decomp).
- **Sleight ids**: `initStockSkill` `0x140220DF0`: 4 → attack row 8, 5 → 9, 6 → 0xB, 7 → 0xC,
  8 → 0xD, 9 → 0xE, 10 → 0x11, 11 → 0x1F. Names from `SY0002.CTD`'s list in order (Sliding Dash,
  Blitz, Stun Impact, Zantetsuken, Strike Raid, Sonic Blade, Ars Arcanum, Ragnarok, Trinity Limit,
  Fira = 0xD as `m7-magic.md` has it). Rows 8 / 0xB / 0xC play `p001_103` / `p001_101` / `p001_104`
  (action rows 0x45 / 0x44 / 0x46), the asset step's matches for KH1's Sliding Dash (entry 106),
  Stun Impact (114) and Zantetsuken (116). Checked (decomp + exe + files).
- **Stun**: the reaction picker `0x14017D260` sends kind 7 to the enemy's stun state (`+0x900`)
  when the enemy's param (`+0x308`) `+0x28` ≠ 0, else to the flinch; the damage `0x14017DC50`
  takes kind 7 as a plain hit; Re:CoM's Stun Impact row (attack row 0xB) carries kind 7. On map
  enemies `0x14019E020` sends kind 7 to their stun state 7. Checked (decomp + exe). The stun's
  length is Re:CoM's state code's (not read).

## What runs (Combat = KH1, Sora's story)

### The save: our ability block (layout v1)

Right after the items extension, inside Re:CoM's spare tail `save + 0x246B ..= 0x2817`
(`recom-keyblade-equip.md`); the Keyblades page's spare-range check ignores it.

| save + | size | field |
|---|---|---|
| `0x269F` | 1 | version: **0 = never written** (older save, new game) → nothing unlocked; 1 = written |
| `0x26A0` | 4 | unlocked, bit = `abilities::ix` (0 Sliding Dash, 1 Blitz, 2 Stun Impact, 3 Zantetsuken, 4 Vortex, 5 Aerial Sweep, 6 Hurricane Blast, 7 Ripple Drive, 8 Gravity Break, 9 Strike Raid, 10 Sonic Blade, 11 Ars Arcanum, 12 Ragnarok) |
| `0x26A4` | 4 | equipped (a subset of unlocked) |
| `0x26A8` | 6 | zero, kept for 9b (KH1's AP) |

Fields never move. Re:CoM saves, checksums, loads, snapshots and zeroes it with its save block.
An older save unlocks what it has at its first Attack press.

### Sync (`win::abilities::sync`)

- On an **Attack press** (the request-bits wrapper sees `+0x424 & 4` after KH1's jump rule), not
  per frame: for each ability not yet unlocked, its source is asked: `0x14023AB00(sleight)` or
  Re:CoM's level (`*0x14087CBF8 + 0x1C`) ≥ KH1's learn level. New ones are unlocked and equipped,
  the block written, the log names them. Never taken back.
- The pickers get the equipped modifiers **whose rows the attack table holds**
  (`attack::modifier_rows_ready`): a modifier whose clip the motion pack lacks is never chosen.
- `kh1_all_abilities = on` (debug ini key): every modifier equipped, nothing written.
- The specials' unlocks (Strike Raid … Ragnarok) are kept now; nothing reads them until 9c.

### Clips and action rows (`rows::KH1_ONLY`, `win::mod::kh1_clips`)

- The asset step already converts every KH1 Sora motion Re:CoM lacks into `PL0001.ANN` slots
  161+ (Stage 6) and `PL0011.ANN` (Stage 23); the manifest maps every KH1 entry to its slot.
  On the owner's files (manifest of 2026-10-30): Hurricane Blast `p001_222`, Blitz `p001_223`,
  Vortex `p001_224`, Aerial Sweep `p001_227`, Ripple Drive `p001_228`, Gravity Break `p001_229`
  are KH1-only; Sliding Dash, Stun Impact and Zantetsuken are Re:CoM's bakes `p001_103` / `101` /
  `104`. Checked (files).
- KH1-only clips get **appended action rows** after Guard (0x47) and Counterattack (0x48), as
  `p001_226` did; Re:CoM's bakes play **Re:CoM's own action rows** of those clips (0x45 / 0x44 /
  0x46), found by name in Sora's action table at start. `PL0011.ANN` holds `p001_101` / `103` /
  `104` too (Checked (files)), so Halloween Town plays them.
- Each modifier row's idle after it is the battle-stance idle, as after Re:CoM's swings
  (`Extension::has_row`).

### Rows (`attack::kh1_table`, `MODIFIER_ROWS`). Checked (tests)

A copy of the Re:CoM row of the same kind (finishers → row 4, Sliding Dash → 1, Vortex → 2,
Aerial Sweep → 5, Hurricane Blast → 7), with `+0` the modifier's clip row, KH1's hit window one
frame earlier (`HIT_LEAD`, as every KH1 row), KH1's combo window, no homing / push, flag 0x80.

| KH1 row | clip (owner's files) | `+4/+5` | window | Re:CoM kind |
|---|---|---|---|---|
| 0 Aerial Sweep | appended `p001_227` | 5–27 | 20–44 | 1 |
| 3 Stun Impact | 0x44 `p001_101` | 33–49 | – | **7 (stun)** |
| 4 Gravity Break | appended `p001_229` | 31–37 | – | 1 |
| 5 Zantetsuken | 0x46 `p001_104` | 35–49 | – | 1 |
| 6 Blitz | appended `p001_223` | 29–37 | – | 2 |
| 7 Ripple Drive | appended `p001_228` | 33–45 | – | 2 |
| 9 Sliding Dash | 0x45 `p001_103` | 25–57 | 28–86 | 1 |
| 10 Vortex | appended `p001_224` | 17–41 | 38–66 | 1 |
| 15 Hurricane Blast | appended `p001_222` | 3–53 | – | 2 |

- `+0x638` / `+0x630` take the template row's id (1, 2, 4, 5, 7), not Re:CoM's sleight ids: Re:CoM
  refers to `+0x638` 138 times in the decomp and some readers test sleight ids, which must not
  run for KH1 rows.
- Travel is the clip's root motion (KH1's request map has the root bit for all but Hurricane
  Blast, an air row with KH1's air speed 12 and hop 40 from the row exec, as the air finisher).

### Hit shapes (`win::attack::pair_dispatch`)

- **Blade records** (bone `0x30`) as before. **Ripple Drive** is one: frac 0 / 0 puts A = B at
  t = 0 (bone 282, the hand), radius 200 + the Keyblade's 40 = **2.4 m**, tested with Re:CoM's
  sphere / column primitives. Checked (tests: `stun_impact_is_a_body_sphere`).
- **Stun Impact** (bone 8): a 2.5 m sphere at Sora's bone `belt1` (Re:CoM index 4, read with
  `0x1402ABB20(player, 4) + 0xD0`, the call Re:CoM makes with the player), during the row's hit
  window, through the same primitives and Re:CoM's own apply. KH1's bone 8 and Re:CoM's `belt1`
  are the same bone at rest (`skel_map.py`: KH1 7 and 8 coincide; Checked (files)); that it stays
  there in the clip is an Assumption (bone 8 is a flags-1 bone with no offset; the radius hides any
  difference). The swept chain (clash type 6 sweeps) follows the bone from the last frame.

### Damage (`attack::kh1_damage_at`, `win::stats::kh1_hit`)

- KH1's `0x1402BFA80` through kh1-core, now with the enemy's **current HP** (`+0x274`) and Sora's
  **KH1 max MP** (`win::mp::current`, else KH1's default 2):
  - Gravity Break: (40 + Strength) % of the current HP × `resist[8]`, at least the floor; always a
    critical with the Kingdom Key (crit 200), bonus 0. Checked (tests).
  - Zantetsuken: 250 % of the current HP × `resist[8]`: an ordinary Heartless dies; bosses take
    their `resist[8]` share (Re:CoM-only bosses: decision 34's 2 %).
  - Ripple Drive: power 10 + max MP × 3 through the curve (Strength doesn't count).
  - Stun Impact: Strength + 4 through the curve, × `resist[10]` (KH1's own: an enemy with stun
    resistance 0 takes 1).
- Re:CoM's extras (back bonus, weak part, boss falloff) on top, as every KH1 hit (decision 13).
- MP charge: the records' 3 / 6 through the existing KH1 rule.

### Sounds (asset step, `ses::add_clips`)

The KH1-only modifier clips get entries in `PL0001.SES` and `PL0011.SES` (decision 22: Re:CoM's
sounds): KH1's swing-sound cues (sound track op 0x19) as Re:CoM's whoosh at KH1's frames (KH1's
args 7–12 / 17 → Re:CoM 232–238, from Re:CoM's own swing clips: Checked (files); other args →
the finisher's 235 or swing 1's 232: Assumption), KH1's footsteps as Re:CoM's step 1 at their
frames (Assumption), the row's voice at 0 (finisher or swing grunts). Re:CoM's bakes keep their
own Re:CoM sounds. Checked (tests, also on the owner's `PL0011.SES` and `xa_ex_0010.mset` from
the test data).

## Sites (`sites::ABILITY_CALLEES`, bytes from the Steam exe)

| Site | First bytes | Use |
|---|---|---|
| `0x14023AB00` sleight learned | `40 53 48 83 EC 20 0F B7 D9 E8 F2 FF F6 FF 85 C0` (call → `0x1401AAB00`) | called, not hooked |

No new hook. Already-checked callees used: `0x1402ABB20` (actor bone). All 446 byte sets and 13
pointer sites re-read from the Steam exe with pefile: 0 mismatches.

## Per frame

- KH1 frames: two more atomic loads (`win::abilities::equipped`).
- Attack presses: one sync (at most 13 small Re:CoM calls, only for abilities not yet unlocked).
- Nothing at the Re:CoM setting or in Riku's story (`win::mp::active` gates the sync; the rows are
  only reached by KH1's frame).

## Logs

- `attack: modifier rows ready: […]; missing (no clip in the motion pack): […]` at start.
- `movement: manifest …; KH1-only clips […]; Re:CoM's rows for KH1 requests [(208, 45), …]`.
- `abilities: level N; unlocked […]; equipped […]; modifiers the pickers get […]` (first press),
  `abilities: unlocked and equipped at Re:CoM level N: […]` (each new unlock).
- The F9 trace's ability bits include the equipped modifiers (KH1's bit layout).

## 1:1 checks

- Checked (decomp + tests): the picker's conditions for all nine (kh1-core tests
  `finisher_modifiers`, `approach_modifiers`, `hurricane_blast`).
- Checked (files + tests): every modifier motion's length, hit / combo / trail windows and root
  travel, every record, against the player's KH1 files (`kh1-data` tests).
- Checked (tests): the rows' bytes, the damage numbers, the ability block, the unlock rules.
- Not checked: anything in game; a KH1 trace of the modifiers.

## Not KH1's (by design or yet)

- **Effects** (KH1's `0x1402AE160` / `0x1402ADC20` on rows 3, 4, 5, 7): M13.
- **Stun length and gate**: Re:CoM's stun state and its enemy gate (`+0x28`); KH1 stuns for
  `resist[10] × 1800` (`0x1402BEEE0` → `0x1402B8730`, timer `+0x3E4`; units not traced) when
  `0x14029F010(actor, 0x42)`. Re:CoM's state length is not read.
- **Gravity Break's reaction 8** is a flinch carrying KH1's number, as the Gravity spell (M7 7.8);
  KH1's flatten is the owner's open Gravity question.
- **Launches** (Blitz, Ripple Drive, Hurricane Blast): Re:CoM's blow-away, as the finishers.
- **Without a lock**, KH1's scan sees only Re:CoM's current target (M12): Vortex's side swing and
  Blitz's "no side enemy" test look at that one enemy.
- **Equipped = unlocked** (KH1: learned unequipped; 9b's equip screen and AP).
- Gravity Break never unlocks by itself (owner question).

## Assumptions

- `save[0x48C]` is the kept item and 2 = Rod (the learn list); the level cursor starts at 1.
- Stun Impact's bone 8 stays on Re:CoM's `belt1` in the clip.
- Whooshes for KH1 args Re:CoM has none for; footsteps as step 1.
- The id − 1 indexing of the AP table (9b).
- `0x1401AAB00` ≠ 0 means Sora's story (its ids 4..0x56 are Sora's sleights).

## Owner questions

1. **Gravity Break's unlock**: KH1 learns it from no level list (Sword, Shield and Rod all lack
   it). (a) at a Re:CoM level you name, (b) with a Re:CoM boss's first defeat (as -ra / -ga spells),
   (c) together with another ability's unlock (e.g. Zantetsuken's sleight), (d) only from 9b's
   equip screen once AP allows. Until then `kh1_all_abilities = on` tests it.
2. **Learn list**: the mod uses KH1's **Rod** list (decision 18: keep the Rod). Confirm.
3. **9b's rule for new unlocks**: KH1 learns abilities unequipped. With the equip screen, should a
   new unlock arrive unequipped (KH1) or keep 9a's "equipped at once"?

## What the owner should test (asset step re-run needed for the sounds)

1. **Re-run the asset step** (`kh1-recom-build`): new `PL0001.SES` / `PL0011.SES` entries for the
   six KH1-only clips. The clips and manifest are unchanged; without the re-run the modifiers work
   silently.
2. Log at start: `attack: modifier rows ready:` should list all nine; `movement: … Re:CoM's rows
   for KH1 requests [(208, 45), (216, 44), (218, 46)]`.
3. With `kh1_all_abilities = on`: Stun Impact / Gravity Break / Zantetsuken / Ripple Drive / Blitz
   as finishers (about a third each for the first three), Sliding Dash at a locked enemy 3 m or more
   away (under 1.6 m above), Vortex at lunge range, Aerial Sweep at a flying enemy, Hurricane Blast as the air
   finisher. F9 traces of each.
4. Without the debug key, on a save with Re:CoM's Lv2 sleight: Sliding Dash works; the log names
   the unlocks at the first Attack press. Level 12+: Ripple Drive.
5. Stun Impact stuns Re:CoM enemies that Re:CoM's own Stun Impact stuns; Zantetsuken kills
   ordinary Heartless; Halloween Town plays every modifier.
6. Combat = Re:CoM: nothing changes; Re:CoM's sleights unchanged.

## Left for 9b and 9c

- **9b**: the equip screen in the "KH1" camp entry (Re:CoM's camp list), KH1's AP (+2 at KH1's
  levels on Re:CoM's level, the bonus list `0x3AC0`'s 3s; costs `0x33D8`), toggling, the movement
  abilities moved from the ini into it, KH1's "learned unequipped" rule (question 3).
- **9c**: the specials (Sonic Blade, Strike Raid, Ars Arcanum, Ragnarok; their KH1 ids, rows and
  commands), whose unlock state is already kept.
