# M9: KH1's abilities (9a: the combo modifiers and the ability state; 9b: the equip screen with AP)

> Stage 9b (below, [Stage 9b](#stage-9b-kh1s-ability-screen-with-ap)) replaces three 9a rules:
> new unlocks arrive **unequipped** (KH1), the block is **layout v2**, and Gravity Break unlocks at
> **Re:CoM level 30** (owner). The movement abilities moved from the ini into the ability state.
>
> The [9b follow-ups](#9b-follow-ups-owner-2026-11-01) (at the end) replace 9b's sources: Guard and
> Dodge Roll (equipped) from the start, High Jump / Glide / Superglide from **Re:CoM's own ability
> variable**, Combo Master at level 50; the ini's story keys no longer unlock; **layout v3**; the
> Abilities rows are Re:CoM's blue row.

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
   Re:CoM's level for Vortex, Aerial Sweep, Hurricane Blast, Ripple Drive. (9b: Gravity Break at
   Re:CoM level 30, owner.)
3. (9a only) **Equipped = unlocked**: a new unlock was equipped at once. 9b: KH1's rule.
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
| `0x269F` | 1 | version: **0 = never written** (older save, new game) → nothing unlocked; 1 = written by 9a; **2 = 9b** (bits 13–24 below, KH1's equip rules); **3 = the 9b follow-ups** (same fields; the sources below) |
| `0x26A0` | 4 | unlocked, bit = `abilities::ix` (0 Sliding Dash, 1 Blitz, 2 Stun Impact, 3 Zantetsuken, 4 Vortex, 5 Aerial Sweep, 6 Hurricane Blast, 7 Ripple Drive, 8 Gravity Break, 9 Strike Raid, 10 Sonic Blade, 11 Ars Arcanum, 12 Ragnarok; 9b: 13 Guard, 14 Counterattack, 15 Dodge Roll, 16 High Jump, 17 Glide, 18 Superglide, 19 Combo Master, 20–22 Combo Plus ×3, 23–24 Air Combo Plus ×2) |
| `0x26A4` | 4 | equipped (a subset of unlocked) |
| `0x26A8` | 1 | **M12 (layout v5)**: the AP Up picks (`abilities::field::AP_UPS`). Before v5 a reserved zero byte nothing read (Checked (code)). |
| `0x26A9` | 5 | zero |

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
- At Damage = Re:CoM (owner, 2026-11-01): Gravity Break and Zantetsuken keep KH1's % (same rule,
  record, Strength, `resist[8]` and floor) on the enemy's **Re:CoM** current HP instead of the
  Keyblade card's column 2: `recom-kh1-attack.md` § Gravity Break and Zantetsuken.
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

## Owner questions (9a; all answered 2026-11-01, built in 9b)

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

## Stage 9b: KH1's ability screen with AP

Owner (M7–M9 choices, M9 follow-ups, 2026-11-01): "an equip screen with KH1's AP (KH1's costs, AP +2
at KH1's levels, on Re:CoM's level)"; Gravity Break unlocks at **Re:CoM level 30**; the learn list is
**KH1's Rod list**; a newly unlocked ability arrives **unequipped**, as in KH1. Answers 9a's owner
questions 1–3. Code: `crates/kh1-recom/src/abilities.rs` (rules, AP, the screen; tested),
`src/win/abilities.rs` (the block, sync, migration, the live read), `src/win/keyblades.rs`
(`Content::Abilities`), `src/win/movement.rs` (`abilities()`), `src/keyblades.rs` (`sub::Row::Abilities`).

### KH1 (Steam). Checked (decomp + files) unless tagged

| what | where |
|---|---|
| **AP is the character record's `+9`**: the new game fills it from the template btltbl `0x39D8` byte 2 = **3** (`0x1402852C0`); the Dive adds the choice's fourth byte (`0x140284B70`; keep Rod / give up Shield, choice 5: **+0**); level-up bonus kind 3 adds **2**, capped at 100 (`0x1402B5660`); the AP Up item 0x9A adds its amount, capped at 100 (`0x1402CD780`); the battle stats copy it to `+0xE4` (`0x1402A5130`). | as listed |
| Sora's bonus list btltbl `0x3AC0` (`0x140528CC0[0]`, the same for every Dive choice; byte `[L − 2]` for reaching L): kind 3 at **11, 17, 20, 23, 29, 32, 35, 41, 44, 47, 53, 56, 59, 65, 68, 71, 77, 80, 83, 86, 89, 92, 95, 98** (24 levels). So max AP = 3 + 2 × those reached: 3 at level 1, 13 at 30, **51** at 99. | files |
| AP cost: `0x1402E8120(id)` = byte +0 of `0x14028F8F0(id)` = `*0x142D36D48 + (id − 1) × 12`, and `0x142D36D48 = 0x142D26118` = btltbl base `0x142D22D40` + **`0x33D8`**. 9a's "row id − 1" Assumption is now Checked. | decomp `0x140280000` |
| AP left: `0x1402E8660(char)` = the menu's max AP (`0x142E942C4 + char × 0x100`) − the costs of the character's **equipped** abilities (list `+0x44`, bit 7 clear), never below 0. The shared list is not counted. That the menu's max AP is record `+9`: Assumption (the cache's writer not read; KH1 has no AP accessory). | `0x1402E8660`, `0x1402E8560` |
| Learning adds an ability **unequipped** (`0x140284E10` writes `id | 0x80`); character lists `+0x44` (48 bytes), the shared list `+0x599` (list index ≥ 10). | `0x140284E10` |
| **The screen**: owner list (`0x140310B60`: each party member, then "Shared Abilities", message 0xC0); a character's list (`0x140311C20`: rows in **list order**, each icon, name, cost; a row is **greyed when unequipped and its cost is over the AP left**); the shared list (`0x1403120F0`, no cost column); the panel (`0x1403108E0`): the character's name, "AP", **AP left / max AP**. | as listed |
| **Confirm on an ability** opens a command window (`0x140310650`): **Equip / Remove** (message 0xBA / 0xB9 by bit 7), **Move** (0xBB: pick another row, the two swap, sound 0x4A), **Sort** (0xBC: three orders, `0x140310400` → quicksort `0x1403119F0` with keys `0x140310260` (sort key, then equipped), `0x140310290` (cost, sort key, equipped), `0x1403102E0` (equipped first, then sort key); the sort key is the record's u16 `+2`, `0x1402E81B0`). Equip with too little AP: **error sound 4** and message window 0xCA; else bit 7 cleared, sound 5. Remove: bit 7 set, sound 0x4B. A **shared** ability toggles (`^ 0x80`) with **no AP test**. | `0x140310650`, `0x140310400`, `0x140310EF0` |
| Ability ids (Checked: `0x1402A6380` / `0x1402A5130` / `0x1402A7510`): High Jump **1**, Glide **3**, Superglide **4** (all three counted in the **shared** list: `0x1402A6734`.. pass list 0xB), Combo Plus **6**, Air Combo Plus **7**, Counterattack **19**, Guard **21**, Dodge Roll **22**, Combo Master **0x41**. Costs / sort keys (btltbl `0x33D8`): Guard 2 / 101, Counterattack 2 / 102, Dodge Roll 1 / 100, Combo Plus 1 / 201, Air Combo Plus 1 / 202, High Jump / Glide / Superglide 0. **Combo Master has no row** (row 65 is zero: cost 0, no name record; Checked (files)). | decomp + files |
| **KH1 FM's Superglide is an ability** (id 4, shared, bit 0x100). | decomp |
| The Rod list (`0x3BF8`) gives: Guard **33**, Counterattack **57**, Air Combo Plus **24, 45**, Combo Plus **87, 93, 99** (and the 9a levels). **High Jump, Glide, Superglide, Dodge Roll, Combo Master are in none of the three lists** (story / other rewards in KH1: which event not traced). | files |

### What 9b builds (Combat = KH1, Sora's story)

**Which abilities, and from where** (`abilities::ALL`):

| ability | KH1 id | AP | unlocks |
|---|---|---|---|
| the nine modifiers | 9a's | 9a's | 9a's (Re:CoM's sleights; Rod levels); **Gravity Break: Re:CoM level 30** (owner) |
| Guard | 21 | 2 | Rod list: level **33** (follow-ups: **from the start**) |
| Counterattack | 19 | 2 | Rod list: level **57** |
| Combo Plus ×3 | 6 | 1 each | Rod list: **87, 93, 99** |
| Air Combo Plus ×2 | 7 | 1 each | Rod list: **24, 45** |
| Dodge Roll | 22 | 1 | **story in KH1**: while `dodge_roll = on` in the ini (follow-ups: **from the start, equipped**) |
| High Jump, Glide, Superglide (shared) | 1, 3, 4 | 0 | **story in KH1**: while their ini key is on (follow-ups: **Re:CoM's own ability variable**) |
| Combo Master | 0x41 | 0 (no row in KH1's table) | **story / other in KH1**: while `combo_master = on` (follow-ups: **Re:CoM level 50**) |

The specials (Strike Raid, Sonic Blade, Ars Arcanum, Ragnarok; KH1 ids 13 / 11 / 12 / 14, costs
3 / 3 / 4 / 4) keep their unlock state but are **not on the screen** until 9c gives them a use.

**AP** (`abilities::max_ap`): ~~3 + 2 at each of KH1's 24 kind-3 levels reached~~ **replaced by M12**
(owner, 2026-11-03): KH1's start 3 + 2 per "AP Up" pick on Re:CoM's level-up screen at Combat = KH1,
no cap, nothing else (`docs/recom-m12-sleights.md` § 8). Used = the costs of Sora's equipped
abilities; left = max − used, never below 0.

**Equipping** (`abilities::page::Screen::confirm`): equipped → removed; unequipped → equipped when
its cost ≤ AP left (shared: always), else **refused**: Re:CoM's error sound (camp sound 5) and the
help line "Not enough AP to equip this ability." (KH1's message 0xCA, our words).

**Unlocks** (`abilities::sync`): from Re:CoM's sleights and levels, **unequipped** (KH1); never
taken back. The story abilities follow their ini keys both ways, and one switched on arrives
**equipped** (as the ini has them today) when the AP allows.

**The ini keys** (chosen: initial state + stand-in, not debug overrides):
- `high_jump`, `glide`, `superglide`, `dodge_roll`, `combo_master`: the story abilities' unlock
  (stand-in until the owner picks their Re:CoM source).
- `guard`, `counterattack`, `combo_plus`, `air_combo_plus`: read **once per save**, at its
  migration (below): what they had on stays equipped where it is learned by then.
- With no save block (title screen) or at Re:CoM's setting, the ini stands as before (the KH1 frame's
  fallback).
- `kh1_all_abilities = on` stays the debug override: everything on the screen equipped (3 Combo
  Plus, 2 Air Combo Plus), AP ignored, nothing written.

**Migration** (`abilities::migrate`, a block of version 0 or 1, at its first sync): the sync's
unlocks; then everything that was equipped stays equipped where still unlocked: 9a's equipped set
(v1: 9a equipped every unlock) and the ini's `guard` / `counterattack` / first `combo_plus` /
`air_combo_plus` copies; then, over KH1's max AP, **unequipped from the most expensive down** (ties:
the later in `ALL`). Written as v2; logged (`abilities: block v1 -> v2 at Re:CoM level N (max AP
M): equipped […]; over KH1's AP, unequipped […]`). A new game (v0) gets only the ini's story
abilities.

**Use** (`win::abilities::equipped_live`, `win::movement::abilities`): every KH1 frame reads the
block's equipped word (one 15-byte read; a block older than v2 is migrated once) and passes
`abilities::to_core(equipped)` to KH1's handlers and pickers: movement (High Jump, Glide, Superglide,
Dodge Roll, Guard, Counterattack), the Combo Plus / Air Combo Plus counts, Combo Master and the
modifiers (those with rows). So a change in the camp applies as soon as the camp closes, and a
loaded save is followed with nothing of ours at load time. The F9 trace's ability bits follow it.

### The screen: the KH1 camp entry's third row, "Abilities"

The submenu is **Items, Customize, Abilities** (`keyblades::sub::ROWS`), on the same page type as the
others (Re:CoM's camp list: both panels, `CButton` rows, `CButtonCursor`, label / value rows, help
line, camp sounds 1 move / 2 confirm / 4 back / 5 refused). No new hook, callee or data site.

- **Owner list** (KH1's `0x140310B60`): "Sora", "Shared" (KH1 has the party members too; Sora only
  here, as the Items page). An owner with nothing learned is greyed; confirming it plays sound 5 and
  "No abilities learned yet."
- **Sora's list / the shared list**: the unlocked abilities, each row its English name; equipped
  rows carry Re:CoM's row mark (0x1D, as the equipped Keyblade); **greyed when unequipped and the
  cost is over the AP left** (KH1's rule; the Items page's grey colour). Confirm toggles (above);
  cancel goes back to the owner list on the same owner; cancel there closes the page.
- **Status panel**: "AP" = **AP left / max AP** (KH1's panel), "Cost" = the highlighted ability's
  cost ("-" on the owner list), "AP used" = used / max AP.
- **Help line**: each ability's one-line description (ours), the owner rows' help, or the refusal.
- **Order**: by KH1's sort key (the record's `+2`: Combo Master 0, Dodge Roll 100, Guard 101,
  Counterattack 102, Combo Plus 201, Air Combo Plus 202, Sliding Dash 205, Vortex 206, Aerial Sweep
  207, Blitz 208, Hurricane Blast 209, Ripple Drive 210, Stun Impact 211, Gravity Break 212,
  Zantetsuken 213; shared: High Jump, Glide, Superglide) = KH1's Sort option 0 without its equipped
  tie-break.
- Rows keep Re:CoM's red row look (`keyblades::row_look::RED`); see the owner question on the blue
  one. (Follow-ups: blue; then, 2026-11-01, green with the green Mickey symbol, § Rows below.)

**Not KH1's (by design or yet):**
- KH1's list order is the **learning order** (the list grows at the end), changed by **Move** and
  **Sort** from the command window; ours is fixed in KH1's sort-key order, and confirm toggles at
  once (no Equip / Move / Sort window). Owner question 2.
- Costs are in the status panel, not a column on each row (one text item a row: Re:CoM's
  `CButton`). Names are English (KH1's ability names and descriptions are its sysmsg, not in the
  asset step; Assumption, as the other pages). No ability icons.
- Sounds: Re:CoM's camp sounds (KH1: 5 equip, 0x4B remove, 4 error).
- Only Sora's and the shared lists (no party yet).

### Logs

- `abilities: block vN -> v2 at Re:CoM level L (max AP M): equipped […] (U AP)[; over KH1's AP, unequipped […]]`
- `abilities: learned at Re:CoM level L (unequipped, KH1's rule): […]`
- `abilities: from the ini: on […], off […]`
- `abilities: level L, AP left/max left; unlocked […]; equipped […]; modifiers the pickers get […]` (once)
- `abilities page: opened at Re:CoM level L: AP …; unlocked […]; equipped […]`
- `abilities page: equipped|removed <name> (c AP); AP left/max left; equipped […]`,
  `abilities page: refused <name> (c AP): AP n left (KH1's rule)`

### Per frame

KH1 frames: one atomic load, one pointer read and a 15-byte read of the block (9a: two atomic
loads); the KH1 frame already rebuilt its abilities each frame. Attack presses and the page's
opening: one sync (Re:CoM's sleight test only for abilities not yet unlocked). Nothing at the Re:CoM
setting.

### 1:1 checks (9b)

- Checked (decomp + files + tests, `KH1_TEST_DATA`'s `btltbl.bin`): the start AP 3, the Dive's +0,
  the 24 AP levels, every cost and sort key, the Rod levels of every level-sourced ability, Combo
  Master's empty row (`abilities::tests::ap_tables_match_btltbl`).
- Checked (decomp + tests): the refusal rule, the grey rule, shared abilities free, AP left clamped
  at 0, new unlocks unequipped (`the_screen_follows_kh1s_rules`, `new_unlocks_arrive_unequipped`).
- Not checked: anything in game; that KH1's menu max AP equals record `+9` (Assumption, no AP
  accessory in KH1).

### Found on the way (not built; for the owner)

- **The Rod list also gives stats**: its non-ability entries go through the same bonus function
  `0x1402B5660`: **max MP + 1 at levels 30 and 66** (entry 2) and entry 7 (record `+0x1C` + 1) at
  54. M7's max MP (`crate::mp::MAX_MP_UP_LEVELS` = 12, 24, 36, 48, 60, the bonus list only) misses the
  Rod's two. Checked (decomp + files). Not changed (outside 9b).
- KH1's Rod list also teaches abilities the mod has no use for yet (Checked (files): ids and
  levels; the names: Assumption, the Archipelago list): id 5 (Treasure Magnet; 9, 72), 10 (Scan;
  15), 23 (21), 24 (42, 75), 25 (90), 26 (60), 27 (63), 28 (48), 53 (69), 60 (27, 81), 62 (39), and
  8 (Critical Plus; 96, 100), which `kh1_core` reads (crit factor) but the screen doesn't list.
  Owner: add any of them (each needs its effect built)?

### Owner questions (9b; 1, 3, 4, 5 answered 2026-11-01, built in the follow-ups below)

1. **Story abilities' Re:CoM source**: High Jump, Glide, Superglide, Dodge Roll and Combo Master
   are in no KH1 level list (KH1 gives them by story). Until you pick, their ini keys unlock them
   (and they arrive equipped). (a) keep the ini, (b) a Re:CoM event or boss each (name them), (c)
   a Re:CoM level each.
2. **KH1's Equip / Move / Sort window**: ours toggles at confirm in a fixed order. (a) keep it, (b)
   add KH1's command window (Equip / Remove, Move, Sort with KH1's three orders) and keep the order in
   the save.
3. **Guard at level 33**: following the Rod list, a new game has no Guard until Re:CoM level 33,
   and Counterattack at 57. Saves past 33 keep Guard equipped (migration). Confirm.
4. **Ability rows' look**: they keep the red row; Re:CoM's camp list also has a blue row with the
   magic icon (`row_look::BLUE`, sequence 0x133). Change? (not changed without asking)
5. **The Rod's MP +1 at 30 and 66** (above): add to M7's max MP?

### What the owner should test

1. Camp → KH1: three rows (Items, Customize, Abilities); help "Equip Sora's abilities with AP."
2. Abilities → "Sora" / "Shared"; the status panel's AP (left / max, e.g. 13/13 at level 30 with
   nothing equipped beyond the story ones).
3. Sora's list: learned abilities in the order above, the mark on equipped rows, greyed rows when the
   AP is short; confirm equips / removes (sound 2), a greyed one is refused (error sound, the help
   line says so); "Cost" and "AP used" follow.
4. Close the camp: the changes apply at once (e.g. remove Guard: Square rolls; equip Combo Plus: one
   more swing; remove a modifier: its finisher stops).
5. Shared: High Jump / Glide / Superglide toggle freely (with their ini keys on).
6. An older save (9a): the log's `block v1 -> v2` line; what was equipped stays, within the AP.
7. Save and reload: the equipped set stays. Combat = Re:CoM, Riku's story: no entry, nothing changes.

## 9b follow-ups (owner, 2026-11-01)

Owner: "**Guard from the start** (still costing AP); High Jump, Glide and Superglide unlock **where
Re:CoM unlocks them**; **Dodge Roll equipped from the start**; **Combo Master at Re:CoM level 50**;
KH1's Rod-path **max MP +1 at levels 30 and 66** added; the Abilities page rows use Re:CoM's
**blue** box." Answers 9b's owner questions 1, 3, 4, 5 (question 2, KH1's Equip / Move / Sort
window, stays open). The max MP: `recom-m7-mp.md` § The Rod list's max MP.

### Where Re:CoM gives Sora High Jump, Glide and Superglide

Re:CoM keeps Sora's three movement abilities as bits of a save **int variable, set 0, id 0x24**
(read by `0x1401B08C0(0, 0x24)`; save variables: `notes/recom-level-world.md` § 0). Its player
setup `0x1401F0C50` (called at each player build) copies them into the player's ability word
`+0x41C`, which its movement code reads. Checked (decomp + exe):

| variable bit | `+0x41C` | what Re:CoM does with it | ours |
|---|---|---|---|
| 1 | `0x200000` | its higher jump: the jump init `0x1401F29F0` plays row 0x14 and takes Sora's jump speed 32.5 instead of 28.0 (`0x1407C5B60 + 0x20` / `+0x1C`) | **High Jump** |
| 2 | `0x400000` (Sora's story only) | its glide: the jump exe `0x1401EDA60` calls `CPlayerSora::initGlide` (`0x1402202C0`, vtable `+0x1A8`) on a jump press after 900 units (18 frames) in the air when `+0x41C & 0xC00000` | **Glide** |
| 4 | `0x800000` (Sora's story only) | the faster glide: the glide exe `0x140217B50` flies at 8.0 instead of 5.0 (`0x1407C5B48` / `0x1407C5B44`) on row 0x1C instead of 0x1B; it also allows the glide by itself (the `0xC00000` test) | **Superglide** (and Glide) |

Where the bits are set (Checked (decomp)):
- **Bit 4**: Re:CoM's reward pick `0x1401BD9D0` (called by `0x1401BF420`), world 13 (Castle
  Oblivion), the first time (the world's byte `save + 0x18A0 + world` = 0) unless byte variable
  (0, 0x61) is 3: bit 4 set and reward 0x5C.
- **Any bit**: the event script machine `0x140145C50`: command **0x2C** ORs a value into an int
  variable of set 0; command **0x41** sets 0x24 (with 0x21, 0x2D) to all bits.
- No other code writes it: the only other writers (`0x1400D1A00`, `0x1400D1FC0`, `0x1400D20B0`)
  have no callers and no pointers to them in the exe (debug leftovers).
- **Which events**: Assumption (Re:CoM's scripts are not in the session; the KH wiki,
  "Abilities (KHCOM)": High Jump on clearing Monstro's Room of Beginnings, Glide on entering
  Neverland, Super Glide in Castle Oblivion's Room of Rewards). Bit 4 in Castle Oblivion agrees
  with the decomp.

KH1's Superglide (id 4) and Re:CoM's faster glide are not the same move (KH1's is its own
ability, run by KH1's handler at KH1's numbers); the owner's rule makes Re:CoM's Super Glide the
moment ours unlocks. Glide unlocks with either bit, as Re:CoM's glide test does.

### The sources now (`abilities::ALL`, `abilities::Source`)

| ability | source | arrives |
|---|---|---|
| Guard | `Start { equip: false }`: from the start | unequipped (KH1's rule), unless the ini's `guard` was on when a save from before 9b first met the page (so a new game with the default ini has Guard + Dodge Roll = 3 AP of 3) |
| Dodge Roll | `Start { equip: true }` | **equipped** when the AP allows (removable on the page; never equipped again by us) |
| High Jump | `Recom(1)` | unequipped (KH1: story grants arrive unequipped too, event syscall `0x1401B1110` → `0x140284E10`, `id | 0x80`; Checked (decomp)); free to equip (shared) |
| Glide | `Recom(2 | 4)` | unequipped, free |
| Superglide | `Recom(4)` | unequipped, free |
| Combo Master | `Level(50)` (owner) | unequipped, 0 AP |
| Counterattack | `Level(57)` (Rod list; unchanged) | unequipped |
| the rest | as 9b | as 9b |

Never taken back, as every unlock (whether a script ever clears a bit of 0x24 is not read).

### The read

`win::abilities::sync` (Attack presses, the page's opening; never per frame) reads the variable
once through Re:CoM's getter `0x1401B08C0` (a callee already checked by `sites::M6_DECK_CALLEES`:
`83 F9 02 73 1D 8B C1 48 8D 0C 40`); no new hook, callee or data site. All 446 byte sets and 13
pointer sites re-read from the Steam exe with pefile: 0 mismatches.

### The block: layout v3 (`abilities::BLOCK_V3`, `abilities::migrate`)

Same fields. A block below v3 is migrated once (first sync, as 9b's):
- **v2 → v3**: High Jump, Glide, Superglide, Combo Master (9b's ini-given story abilities,
  `V2_STORY_BITS`) are taken back and asked their new source at once; those whose source is met
  keep their equipped state, the others come back later, unequipped. Dodge Roll keeps its state
  (a player who removed it keeps it removed); not yet unlocked → unlocked and equipped.
- **v0 / v1 → v3**: as 9b's migration (the ini's `guard` / `counterattack` / `combo_plus` /
  `air_combo_plus` read once), with the new sources; Dodge Roll is wanted equipped.
- Then KH1's AP rule (over the max AP: unequipped from the most expensive down, Dodge Roll 1 AP
  near the last).

### The ini

`high_jump`, `glide`, `superglide`, `dodge_roll`, `combo_master` **no longer unlock anything**.
They stay readable (old ini files parse; the Config menu writes them back unchanged) and are only
the KH1 frame's fallback while no save block exists (`win::movement::abilities`; not reached in
play). Testing switch: `kh1_all_abilities = on` (unchanged). `guard`, `counterattack`,
`combo_plus`, `air_combo_plus`: unchanged (read once for a v0 / v1 save).

### Rows: Re:CoM's green row with the green Mickey symbol (owner, 2026-11-01)

> "have the abilities be green with the green mickey symbol" (in game, 2026-11-01).

Every row of the Abilities page (the owner rows "Sora" / "Shared" and every ability) takes Re:CoM's
list entry **0x131** through its row set-up `0x140112F10` (variant 2 of the list's checked
sequence table `0x1407C11F0`: green plate 0x80143200, icon sprite 278 tinted 0x801EDC1E), the look
the Items page's item rows had (`abilities::page::ROW_LOOK = keyblades::row_look::GREEN`). Before:
the blue magic row 0x133 (still the Customize page's, `recom-keyblade-equip.md`).

- Checked (in game, owner): sprite 278 in 0x131 is the **green Mickey symbol**, not the item
  symbol (the earlier Assumption in `recom-m8-items.md` § Row looks was wrong).
- Checked (test): `abilities::tests::rows_are_recoms_green_mickey_row` (reads 0x131 from the
  checked table's bytes). No new site. Not seen in game yet.
- The Items page's rows take the asset step's green row with Re:CoM's item symbol instead
  (`recom-m8-items.md` § Row looks / Built); the Abilities rows keep 0x131 (Mickey symbol).

### Logs

- `abilities: block vN -> v3 at Re:CoM level L (max AP M, Re:CoM's ability variable 0x…): equipped
  […] (U AP)[; the ini's, not yet from their source, taken back […]][; over KH1's AP, unequipped […]]`
- `abilities: learned at Re:CoM level L (unequipped, KH1's rule; Re:CoM's ability variable 0x…): […]`
- `abilities: learned at Re:CoM level L and equipped (owner: from the start): ["Dodge Roll"]`
- the first-sync line now names Re:CoM's ability variable.

### 1:1 checks

- Checked (decomp + exe): the variable, its three bits and what Re:CoM's movement does with them;
  the Castle Oblivion writer; the script commands 0x2C / 0x41; KH1's story grants arrive
  unequipped.
- Checked (tests): `learn_levels_are_kh1s_rod_list_and_the_owners`, `new_unlocks_arrive_unequipped`,
  `movement_abilities_follow_recoms_variable`, `migration_keeps_the_old_set_within_kh1s_ap`,
  `v2_story_abilities_move_to_their_new_sources`, `rows_are_recoms_green_mickey_row`; the btltbl
  test (Guard's Rod level 33 still read) with the player's file.
- Not checked: in game; which script events set bits 1 and 2 (Assumption above). The rows' icon
  (sprite 278) is the green Mickey symbol (owner, in game).

### Owner questions (follow-ups)

1. **Guard at the start: equipped or not?** Built as unlocked and **unequipped** (KH1's rule for a
   new ability; only Dodge Roll was asked equipped). Exception: a save from before 9b (or a new
   game) with the ini's `guard = on` (the default) gets it equipped once, so a new game starts with
   Dodge Roll + Guard = 3 of 3 AP. (a) keep, (b) always equipped at the start when the AP allows,
   (c) never from the ini (new games: Dodge Roll only).
2. **High Jump / Glide / Superglide arrive unequipped** (KH1's rule; they're free, one confirm on
   the Shared list). In Re:CoM they work at once. (a) keep KH1's rule, (b) equip them when Re:CoM
   gives them (still removable).

### What the owner should test

1. A **new game** (Sora, Combat = KH1): KH1 > Abilities > Sora lists Dodge Roll (marked) and Guard
   (marked with the default ini); AP 0/3. Square rolls; Guard works.
2. **Monstro's Room of Beginnings cleared** (or the save past it): the Shared list has High Jump;
   equip it; the jump is KH1's High Jump. **Neverland**: Glide. **Castle Oblivion's Room of
   Rewards**: Superglide. The log's `Re:CoM's ability variable 0x…` shows Re:CoM's bits (1, 2, 4).
3. Level 50: Combo Master on Sora's list (0 AP). Level 30: max MP one higher than before (6 at 30
   with the Kingdom Key), level 66: 10.
4. A 9b save with `high_jump = on` in the ini but before Monstro: the log's `block v2 -> v3` line
   takes High Jump back; after Monstro it returns.
5. The Abilities page's rows and the Customize page's slot and spell rows: blue plate, magic icon.
6. Combat = Re:CoM: nothing changes (Re:CoM's own High Jump / glide unchanged).

## Owner change (2026-11-01): Guard and Dodge Roll are Sora's kit

Owner: "just have guard and dodge roll be a part of sora's movement kit from the start, dont add
them as abilities." With it (lead's brief): High Jump, Glide and Superglide **equipped when Re:CoM
gives them** (still removable; once removed, not equipped again). Answers the follow-ups' owner
questions 1 and 2 above.

### What changed (`abilities.rs`, `win/abilities.rs`)

| | before | now |
|---|---|---|
| Guard, Dodge Roll | abilities on the Sora list (2 AP / 1 AP), `Source::Start` | `Source::Kit`: **always on at Combat = KH1**, no AP, not on the Abilities page, not in the AP sum (`KIT_BITS`; `to_core` sets `guard` / `dodge_roll`; `kh1_bits` ORs them in for the trace) |
| High Jump, Glide, Superglide | arrive unequipped | arrive **equipped** (`arrive_equipped_bits`: the `Source::Recom` ones); removable; `sync` equips only on the unlock, so a removed one stays removed |
| the ini's `guard` | read once for a v0 / v1 save | no longer read (Guard is the kit's) |
| `kh1_all_abilities = on` | every ability | every ability but the kit (always on) and the specials not built yet |

At Combat = Re:CoM nothing changes (the kit is the KH1 frame's; Re:CoM's own block and avoid run).

### The block: layout v4 (`BLOCK_V4` = `BLOCK_NOW`)

Same fields. `read` hides the kit bits; a block below v4 is migrated once at its first sync
(`migrate(s, raw, from, …)`):
- every older block: Guard and Dodge Roll leave the unlocked / equipped words (their AP is back:
  up to 3 AP freed); the log names them (`kit_dropped`);
- **v3**: High Jump / Glide / Superglide already unlocked (they arrived unequipped by v3's rule)
  are equipped once;
- v2 and older: as before (v2's ini-given story abilities taken back and asked their source again;
  v0 / v1 read the ini's `counterattack` / `combo_plus` / `air_combo_plus` once);
- then KH1's AP rule (`fit_ap`).

### Checks

- Checked (tests): `guard_and_dodge_roll_are_sora_s_kit`, `new_unlocks_arrive_unequipped` (level
  unlocks still unequipped: KH1's rule), `movement_abilities_follow_recoms_variable` (equipped on
  arrival, not again after removal), `v3_blocks_drop_the_kit_and_equip_recoms_movement`,
  `migration_keeps_the_old_set_within_kh1s_ap`, `v2_story_abilities_move_to_their_new_sources`,
  the screen test (no Guard / Dodge Roll rows), the btltbl test.
- Not checked: in game.

### What the owner should test

1. A v3 save (0.52): the first Attack press or the Abilities page logs `block v3 -> v4 … Sora's kit
   now, out of the block (their AP back) ["Guard", "Dodge Roll"]`; AP used goes down by what they cost; Square still rolls and
   guards; the page no longer lists them; High Jump / Glide / Superglide that were unlocked are now
   marked.
2. Remove High Jump on the page, leave and come back: it stays removed.
3. A new game: Guard and Dodge Roll work at once; AP 0/3 used.

## Stage 9c.1: KH1's Sonic Blade (M9 stage 9c)

Lead's brief: Sonic Blade 1:1 with KH1, native: on the Abilities page, the offer and its
follow-ups in the command menu, run on Re:CoM's own sleight action, KH1's rows and damage, KH1's
motions as new clips, its sounds in Re:CoM's sound events. KH1's scene tint skipped; Re:CoM's
sleight effects (owner).

### KH1 (Steam). Checked (decomp + exe + files) unless tagged

The whole flow is in `crates/kh1-recom/src/specials.rs`'s head (offer `0x14028D0F0`, start
`0x14028D800` / `0x14039EAD0`, rows `0x14039ECA0`, follow-ups `0x14039ED70` / `0x14039EA40`, physics
`0x14039EBE0` / `0x14039EBA0`, end `0x14039EC10`). The numbers:

| what | value | where |
|---|---|---|
| ability → command | Sonic Blade 11 → 0x4B (Ars Arcanum 12 → 0x57, Strike Raid 13 → 0x5E, Ragnarok 14 → 0x5A, Trinity 15 → 0x52) | table `0x140528340` |
| offer order | Trinity, Ragnarok, Ars Arcanum, Sonic Blade, Strike Raid | `0x14052834C` = [4, 3, 1, 0, 2] |
| MP cost | 200 (2 whole MP; ≥ 100 = whole points, `0x1402A4C10`), others 300 / 200 / 300 / 300 | btltbl `+0x14C` |
| range | 600 (gap to the lock point minus its radius); Sonic Blade refuses a target > 160 above | command `+0xE`; `0x14028D0F0` |
| score | 2 can pay, 1 can't (greyed); FM never draws a greyed context entry | `0x14028D0F0`, `0x140277CF0` |
| AP | 3, learned at Re:CoM's sleight 9 (Sonic Blade) | btltbl ability row 11 (cost 3, key 801) |
| state | 0x20; flags `0x1200080` (no damage, no reaction) | `0x14039EC50` |
| wind-up | xl request 0xC8, blend 8, 30 frames, no root, no hit; end → dash 1, blend 0 | `0x14039EAD0`, `0x14039ED60` |
| dash | rows 0x1B..0x20 (request 0xC9, 54 frames, root travel 562.05, hit 4–22, combo window 10–40) | btltbl rows; `xl_limit.dat` |
| finisher | row 0x21 (request 0xCA, 82 frames, root 734.41, hit 16–34) | same |
| follow-up | Rave (0x4D) in the Attack row, Blast (0x4E) once n > 5; greyed outside the window or without the attacking bit; Triangle: re-aim (none: turn 180°), next row, blend 8 | `0x14039ED70`, `0x14039EA40` |
| records | element 10 (stun → Re:CoM kind 7), kind 3, bone 0x30, crit 200, no MP charge, radius 60, power 6 (dash) / 10 (finisher) | btltbl attacks 0x5C18 / 0x5C88 |
| words | 319 "Sonic", 321 "Rave", 322 "Blast" (each language's Word.bin) | command records `+4` |

Tick rates: both run 60 Hz frames here (as every earlier stage; `docs/recom-kh1-movement.md`); the
xl clips are converted at 60 fps. Checked (decomp, earlier stages).

### Re:CoM (Steam). Checked (decomp + exe)

- Sleights run on action **0x16**: `initStockSkill` `0x140220DF0` (case 9 = Sonic Blade: attack id
  0xE, `0x140211790(+0x3B8, 9, 0)` loads its effect group), `exeStockSkill` `0x140218950`
  ("SONICRAVE": start 0x8C with `limit1` effect 0 on the bone `+0x3A4`, dashes 0x8D–0x92 on attack
  id 0xF, finisher 0x93 on 0x10). Sora's vtable `+0x140` (`0x1405A9A78`).
- The Battle exe calls it at `0x1401F9B5E` (`call [rax+0x140]`); the **Field exe has no case**
  (its table `0x1401FA5F4`, entry for 0x16 = the default `0x1401FA599`): Re:CoM never plays a
  sleight in the field.
- Re:CoM's attack row for id 0xE is empty (no window); its weapon step `0x140225840` calls Sora's
  vtable `+0x108` (`0x140224610`) when the weapon's attack starts: id 0xF plays group 1 sound 0xD1
  (Re:CoM's Sonic Blade sound); id 0x10 none.
- **Per dash** (`exeStockSkill` "SONICRAVE", read for the owner's answers): the start (0x8C), six
  dashes (0x8D..0x92, chained by a press in the row's window, "Chain TRG / Entry / OK") and the
  finisher (0x93): the same structure as KH1's (wind-up, rows 0x1B..0x20, row 0x21). Each part
  plays a motion of the sleight's own pack, `0x1401F73A0(player, row[0], 0, 1)`: the start row 0xE
  motion 0, every dash row 0xF motion 1, the finisher row 0x10 motion 2, all with effect / sound
  variants 0 (the start's: row 0xE byte 1 = 0 and Sora's vtable `+0x110(player, 3)` = 0 for id
  0xE, `0x14021FEF0`). Its per-state code plays **no voice, sound or effect of its own** besides
  the `limit1` start effect and the weapon's 0xD1 for id 0xF: **the voices, sounds and effects
  are the sleight motions' own events** (rows: `sites::SPECIAL_CHECKED`, `0x1407C6D50`).
- `0x1401F73A0` → `0x1401A3580(player, pack, motion, …)`, pack = `0x14020FE60(+0x3B8, 9)` + 10000
  = 0x3F1 + 10000 = **11009** (`0x14020F720`: 9 → 2; `0x14020FB10`: 2 → 0x3F1). Besides playing
  the motion, `0x1401A3580` (disassembled): when the playing motion is one of Sora's own
  (`ctrl+0x52 & 0x200` clear) it stops his own event players (`+0x108` effects, `+0x118` sounds,
  vtable `+0x10`), then `0x140179A30(+0x110, pack)` + `0x14017A240(+0x110, motion, EET variant,
  1)` unless `+0x154 & 0x400`, and `0x140238710(+0x120, pack)` + `0x140239260(+0x120, motion, SES
  variant, 1)` unless `+0x154 & 0x4000`. Both players step every update on the motion controller
  (`0x1401A82E0`, vtable `+0x18`); Sora's next own motion stops them (`0x1401A3470`, when the
  sub-pack flag was set).
- Re:CoM waits for the sleight's resources before its start (`0x1402106E0(+0x3B8, 9)` ≠ 1: motion
  0x2E, sub-state 4); it also tints the screen at the start (`0x1401F7D80(player, 0x1407C5D50)`,
  row 0xE flag 0x20) — not per dash; built since the owner's answer (§ Re:CoM's start tint).

### What runs (Combat = KH1, Sora's story)

- **Abilities page**: Sonic Blade on the Sora list (3 AP) once Re:CoM's sleight 9 is learned
  (`Source::Sleight`); the other four specials stay off the page (`SPECIALS_WAITING`).
- **Offer** (`win::command_menu::special_offer`, each menu build): when no object took the context
  entry, Sora is in KH1's state 0 with no special, Sonic Blade is equipped, and Re:CoM's attack
  target is a live enemy: KH1's walk over the five (only Sonic Blade built) → the context tab shows
  "Sonic" (greyed when MP < 2). Assumption: Re:CoM's lock / attack target stands for KH1's target.
- **Start** (Triangle): the menu issues the command (`win::specials::issue`); Sora's KH1 frame takes
  it (`Kh1Sora::command_special`), turns to the target, ends a playing swing's action, then
  `win::specials::start_special`: KH1's MP paid on the press, **`initStockSkill`'s writes for case
  9 without the card** (action 0x16, attack id 0xE, its sleight resources, gravity, phase 3, sub
  1, the homing target; AP = the attack stat, as KH1 swings), Re:CoM's `limit1` start effect on
  Sora. The KH1 frame: state 0x20, the wind-up clip.
- **Battle only** (owner, 2026-11-01: "Only in battle"): the offer needs Re:CoM's battle room
  (`player+0x390`, the field / battle test Re:CoM's own attack route uses), and `start_special`
  refuses outside it. The Field exe is Re:CoM's again (no case, no cave: `0x1401FA648` /
  `0x1401FB7B2` untouched). Assumption: `+0x390` ≠ 0 exactly when the Battle exe runs Sora.
- **Each frame**: Re:CoM's Battle exe calls `exeStockSkill` through Sora's vtable; our wrapper runs KH1's frame and then Re:CoM's own `procAttackCollision` on the
  row with the motion's frame, as Re:CoM's own exes do. Not a KH1 special: Re:CoM's exeStockSkill
  (its sleight cards, untouched).
- **Rows** (`win::specials::special_row`): the last row's volume ends, `+0x3C8` = our KH1 row
  (0x1B..0x21, appended to Re:CoM's attack table; KH1's window one frame earlier as every KH1 row),
  `+0x638` = 0xF / 0x10 (Re:CoM's Sonic Blade ids: its dash sound and hit effects), the hit fields
  cleared, a new KH1 attack (each row hits each enemy once). The pair dispatch and the enemy-hit
  wrapper already key on the KH1 row: KH1's spheres and KH1's damage.
- **Base Re:CoM's voices, sounds and effects** (owner, 2026-11-01: "however it works in base
  ReCoM", for both): each time a KH1 motion of the special starts (`RecomHost::play` →
  `win::specials::motion_events`), right after Re:CoM's motion request, the events of the Re:CoM
  sleight motion it stands for start on Sora's sleight event players exactly as `0x1401A3580`
  starts them: the wind-up → motion 0, each of the six dashes → motion 1, the finisher → motion 2,
  pack 11009, variants 0 (`crate::specials::recom_events`). Re:CoM's own code plays them, on KH1's
  clip clock. At the end they stop (the players' vtable `+0x10`, as Re:CoM's next motion would).
  The weapon's 0xD1 for each dash stays native (id 0xF). Our own SES entries for the three clips
  are gone.
- **Invincible**: KH1's flags `0x1200080` → our damage-check path (`+0x418` bit 0) for the whole
  special.
- **Follow-ups**: the Attack row shows Rave / Blast (greyed outside the window), Triangle runs
  them; X on it is the error sound; a refused confirm clears the attacking bit.
- **End**: after the last row's motion, or Sora airborne (falls, blend 8), or Re:CoM ran another
  action: Re:CoM's end action `0x1401F6660`, KH1's state 0.
- **Root motion**: the root-delay wrapper treats the special like a swing; its safety net is now
  1.0 m a frame (was 0.5 m: the dash travels 0.61 m in a frame, the old net cut it to 6.23 m).

### Asset step (`kh1-recom-build`)

- Reads KH1's `xl_limit.dat` (its motion set at `+4`); converts entries 0, 1, 2 (wind-up, dash,
  finisher) to **new 60 fps clips after the KH1-only entries' slots** (230, 231, 232 with the
  owner's files), played once (KH1 requests them with end callbacks), dash and finisher with root
  motion (the set's request map), Keyblade clips at the same slots, Halloween Town's pack too. The
  manifest gains an `xl_limit` section (xl entry, KH1 request, slot, name, frames). Without the file:
  a note, no clips, no special in game.
- Sounds: **none of ours** for the three clips (owner's answer: base Re:CoM's sleight events play
  on them, above). The step rewrites `PL0001.SES` / `PL0011.SES` from Re:CoM's own, so a re-run
  removes the earlier build's Sonic Blade entries.
- The menu words 319 / 321 / 322 in every language (German ß, KH1 byte 0xE2, written "ss": Re:CoM's
  font codes for it are not known).

### Sites (bytes re-read from the Steam exe with pefile)

| site | bytes | use |
|---|---|---|
| `exeStockSkill` `0x140218950` | `48 8B C4 48 89 58 10 48 89 70 18` | hook (`SPECIAL_HOOKS`) |
| Field table entry `0x1401FA648`, cave `0x1401FB7B2` | `99 A5 1F 00 99 A5 1F 00`, `CC` × 14 | **not patched any more** (battle only; Re:CoM's bytes, re-read) |
| `initStockSkill` `0x140220DF0` | 40 bytes to `BA 16 00` | checked (its writes copied) |
| Battle case `0x1401F9B5E` | `48 8B 07 48 8B CF FF 90 40 01 00 00 E9 5A 01 00 00` | checked |
| `procAttackCollision` `0x1401F5DC0` | `40 53 48 83 EC 30 48 8B D9 48 8B 89 C8 03 00 00 0F B6 41 04` | called |
| sleight resources `0x140211790` | `40 53 41 54 48 83 EC 38 48 8B D9 E8 80 DF FF FF` | called |
| sub-pack motion play `0x1401A3580` | `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 48 83 EC 30` | checked (its event part copied) |
| effect events pack `0x140179A30` | `48 89 5C 24 18 56 48 83 EC 20 45 33 C0 48 8B F1 8B DA` | called |
| effect events start `0x14017A240` | `89 54 24 10 57 41 57 48 83 EC 38 48 8B 41 18 45 8B F8` | called |
| sound events pack `0x140238710` | `41 56 48 83 EC 30 48 C7 44 24 20 FE FF FF FF 48 89 5C 24 40` | called |
| sound events start `0x140239260` | `44 89 44 24 18 89 54 24 10 41 56 41 57 48 83 EC 68 48 83 79` | called |
| sleight resources ready `0x1402106E0` | `40 53 48 83 EC 20 48 8B D9 E8 32 F0 FF FF 85 C0` | called (log only) |
| Re:CoM rows 0xE..0x10 `0x1407C6D50` | 48 bytes (`00…60`, `01 00 01 00 03 22 03 14 0F 14 00 03 00 00 00 64`, `02 00 04 00 0F 3C 0F 20 0F 14 02 03 00 00 00 64`) | read: the sleight motions 0 / 1 / 2 |
| vtable `+0x140` `0x1405A9A78` | → `0x140218950` | checked at start (`SPECIAL_PTRS`) |
| data | `0x1407C7E10` → `"limit1"`; `0x1407C784C` = 4000; row 0xE empty | read; checked at start / by script |

Other callees (`0x1401F6660`, `0x1401F3C00`, `0x1401F68A0`, `0x140224FB0`, `0x1401F7F90`) were
checked by earlier stages' site lists. Every `SPECIAL_CHECKED` entry is also compared at start (a
mismatch: no KH1 specials). All 476 sites of `sites.rs` re-read with pefile: 0 mismatches.

### Per frame

Nothing new unless a special runs: the request-bits wrapper drops an untaken command (one lock),
the menu's offer runs at its builds (only with Sonic Blade equipped and a target). During the
special: our exeStockSkill wrapper (the KH1 frame Sora runs anyway, plus one Re:CoM call), and
four Re:CoM calls at each of its eight motion starts.

### Logs and trace

- start: `specials: KH1's Sonic Blade at combat = kh1 …`; missing clips: `specials: Sonic Blade's
  rows are not in the attack table (no xl_limit clips: re-run kh1-recom-build)`.
- per special (first 80 lines): `specials: special(0x4b)`, `specials: sp-row(0x1b n1 f31)`, `specials: sp-end(done f…)`, `specials: sp-refused(0x4d greyed)`.
- each motion start (first 80 lines): `specials: Re:CoM's Sonic Blade events: pack 11009 motion 1
  (xl entry 1): effects started|none, sounds started|none`; at the start, if Re:CoM's sleight
  resources aren't in yet: `… not loaded yet …`; outside battle: `special 0: not in battle`.
- F9 trace, events column: `sp -` or `sp 0 Row n3 f97 bit`, then that frame's events (`start`,
  `row 0x1c n2`, `ev m1 eet ses`, the tokens above).

### 1:1 checks (9c.1)

- Checked (decomp + files, tests): every number in the KH1 table above; the xl motions' lengths,
  flag windows and root travel at every frame against the player's `xl_limit.dat`
  (`kh1-data::xl_motions_match_the_players_file`); rows 0x1B..0x21 and attacks 0x5C18 / 0x5C88
  against btltbl (`kh1_defaults`); the clips' travel at the end (`validate::h_specials_clips_from_xl_limit`:
  5.62 m / 7.34 m); the offer, follow-ups
  and refusals (`specials::tests`, `command_menu` tests); a full run in the movement fake
  (`sonic_blade_runs_kh1s_rows_on_recoms_sleight_action`: dash 1 at frame 31, rows in order;
  `sonic_blade_ends_after_one_dash_or_in_the_air`: one dash travels 562.05 ± 0.5 and ends at
  frames 83..86); the menu words in all six languages (owner's Word.bin files).
- Not checked: in game; against a KH1 trace of Sonic Blade (none in this session: frame for frame needs one).
- Checked (decomp + exe disasm, tests): Re:CoM's per-dash structure and that its voices, sounds
  and effects come only from the sleight motions' events; the pack and motion numbers
  (`specials::tests::recom_events_follow_the_sleight`, `sites::special_sites` on the rows' bytes);
  the Field exe untouched (`special_sites`).
- Not checked: which voices / sounds / effects those events hold (Re:CoM's sleight pack, group
  1009, is not in the session); in game.
- Assumptions: KH1 starts the wind-up's callback request on the next update (dash 1 at frame 31,
  not 30); Re:CoM's attack target stands for KH1's target; Re:CoM's sleight motions are as long as
  KH1's (their events land on KH1's clips at Re:CoM's frames: if Re:CoM's dash were shorter or
  longer, its events would fall earlier / later in KH1's dash); Re:CoM's sleight resources are
  loaded by the first Sonic Blade (Re:CoM waits for them, KH1 can't: until they are in, its events
  find nothing; logged); `player+0x390` ≠ 0 exactly when the Battle exe runs Sora.

### Not KH1's (by design or yet)

- **Voices, sounds and effects are base Re:CoM's** (owner): KH1's own (its row voices, its bank's
  4003 / 4004, its step and whoosh) are not played.
- KH1's scene tint: skipped (owner). Re:CoM's own start tint: built (owner's answer, below).
- Re:CoM's resource wait before the start (motion 0x2E): not copied (KH1's timing).

### Owner answers (9c.1, 2026-11-01) and what was built

1. Dash voices: "however it works in base ReCoM" → built: Re:CoM's sleight motion events on every
   KH1 motion of the special (above); our SES entries removed.
2. Per-dash effects: "however it works in base ReCoM" → built: the same events (Re:CoM's effects
   come from them; its code adds only the `limit1` start effect, already played).
3. Field use: "Only in battle" → built: no offer and no start outside Re:CoM's battle room; the
   Field exe's case and cave removed (Re:CoM's bytes again).
4. Earlier question 3 (KH1's clips over Re:CoM's own Sonic Blade clip): not answered; KH1's clips
   kept (KH1's frames and travel).

### Owner questions (9c.1, new)

1. **Re:CoM's start tint.** Answered (owner, 2026-11-01): "Add Re:CoM's tint" → built (§ Re:CoM's
   start tint).

### What the owner should test (re-run the asset step first)

1. `kh1-recom-build`: it prints `Sonic Blade (xl_limit.dat): wind-up, dash, finisher on new slots
   [(230, …), (231, …), (232, …)]`, and the SES line says `Sonic Blade's clips: none, Re:CoM's
   sleight events play on them` (the re-run removes the earlier build's entries).
2. Learn Sonic Blade (Re:CoM's sleight), equip it on KH1 > Abilities (3 AP).
3. In battle, near an enemy with 2+ MP: the context tab shows "Sonic"; with less, nothing (FM's
   rule). In the field: never. Triangle: 2 MP gone at once, the wind-up, the first dash.
4. During each dash, "Rave" in the Attack row lights inside the window; Triangle there dashes
   again (toward the target, or back the other way without one); the seventh is "Blast" → the
   finisher. Not pressing: Sora stops after the dash.
5. No damage taken during the whole special; enemies take KH1's damage; Re:CoM's sleight cards
   (Sonic Blade included) still work at Combat = Re:CoM and with cards at Combat = KH1 if the deck
   is there.
6. Voices, sounds and effects: compare with Re:CoM's own Sonic Blade card at Combat = Re:CoM (same
   grunts / whooshes / trails per dash and finisher). The log's `Re:CoM's Sonic Blade events …`
   lines should say `effects started, sounds started`; `none` or `not loaded yet` means Re:CoM's
   sleight resources weren't there.
7. F9 trace of one full Sonic Blade (seven rows): attach it; the `sp` column gives the frames for
   the 1:1 check against KH1.

### Damage at Damage = Re:CoM (owner, 2026-11-01)

> "Sonic Blade does Re:CoM's Sonic Blade sleight damage per dash and finisher"

- **Re:CoM** (Checked (decomp + disasm `0x140219AEE..0x140219BB8`, exe rows)): `exeStockSkill`'s
  SONICRAVE writes, at 0x8C → 0x8D and at each chain, `+0x638` = 0xF (dash) or 0x10 (finisher,
  sub-state 0x93), **AP = `StockSkillList[+0x82C][row+0xA]` × attack stat `+0x43C` ÷ 100**
  (`imul edx,[r14+0x43C]`, signed ÷ 100), `+0x668` = row `+0xB`, `+0x66C` = `StockSkillList +0xA`,
  status `+0xB`, then `0x1401E66A0`. Sonic Blade is sleight 9 (`initStockSkill` case 9); row 0xF has
  column 0, row 0x10 column 2 (`0x1407C6D50`, `sites::SPECIAL_CHECKED`); `initStockSkill` gives the
  start (row 0xE, column 0, no hit window) the same rule. In the session's `SY0002.BIN` sleight 9
  holds 200 / 0 / 400 / 0 (dash 2 × attack, finisher 4 × attack) and attribute 1 (Checked (files);
  the DLL reads the live table).
- **Ours** (`win::specials::start_special` / `special_row` → `win::damage::special`, once per row):
  KH1's six dashes take Re:CoM's dash AP (row 0xF, column 0), the finisher Re:CoM's finisher AP
  (row 0x10, column 2), with the sleight's attribute and Re:CoM's rewrite; the enemy-hit wrapper
  leaves them as is. KH1's power 6 / 10, crit and the stun element are not used for the number;
  the reaction (KH1's stun → Re:CoM kind 7) is unchanged.
- Trace: `:recom <ap> col 0 sleight 0x9` (dashes), `col 2` (finisher). Log: `damage: Sonic Blade
  row 0xf: Re:CoM sleight 0x9 column 0: …`.
- At Damage = KH1: KH1's damage as before.
- **Test:** in battle, Sonic Blade on a Shadow: each dash's hit in the trace carries
  `:recom <2 × attack> col 0 sleight 0x9`, the finisher `<4 × attack> col 2`; compare with Re:CoM's
  Sonic Blade sleight at Combat = Re:CoM on the same enemy.

### Re:CoM's start tint (owner, 2026-11-01: "Add Re:CoM's tint")

- **Re:CoM** (Checked (decomp + disasm + exe)): `exeStockSkill` `0x140218950` sub-state 1 (the
  sleight's first update): with the row's flag 0x20 (row 0xE: `+0xF` = 0x60) it picks a colour by
  attack id × 10; id 0xE (0x8C) → `lea rdx,[0x1407C5D50]` (`0x140218E05`) → `0x140218D0E`: `xor r9d,
  r9d; movaps xmm2,xmm6; call 0x1401F7D80` with `xmm6 = [0x1406C1208]` = 20.0 (`0x140218CA9`). So
  **`0x1401F7D80(player, (0.5, 0.5, 0.75, 1), 20.0, 0)`**, before the resource wait and the `limit1`
  effect. `0x1401F7D80`: blend 0x1808 (flags & 1: 0x187C), `+0x418` bit 31 set unless r + g + b =
  3, `0x140232170(blend, time, colour, 0, 0)` (a fade toward the colour at 1 / time per step).
- **The end**: Re:CoM's end action `0x1401F6660` (`0x1401F66D4..0x1401F6711`): with `+0x418` bit
  31 set it fades to (1, 1, 1, 1) over 20.0 (blend 0x1808) and clears the bit. The only other
  Re:CoM function that clears it, `0x1401E7C20` (not read further), runs as it does for Re:CoM's
  sleights.
- **Ours** (`win::specials::start_special`): the same call with Re:CoM's colour and time, at KH1's
  start, right before the `limit1` effect, as Re:CoM's order; the end is `end_special` →
  `0x1401F6660` (already Re:CoM's end of a sleight), so the fade back is Re:CoM's. Nothing per frame.
  The trace's special events show `tint`.
- Sites (`sites::SPECIAL_TINT_CHECKED`, bytes re-read with pefile; a mismatch leaves only the tint
  out, logged): `0x1401F7D80` (16 bytes, called), `0x140218E05` (the colour's `lea`), `0x140218D0E`
  (the call: flags 0, the time in xmm6), `0x1406C1208` (20.0, 30.0), `0x1407C5D50` (the colour),
  `0x1401F66D4` (the end's reset).
- Not checked: in game; the fade's unit (20.0 "steps" of `0x140232170`'s consumer, not read: it is
  Re:CoM's own value either way).
- **Test:** start Sonic Blade in battle: the screen takes Re:CoM's blue-grey tint as its Sonic
  Blade card does at Combat = Re:CoM, and fades back when the special ends (the finisher's end, or
  stopping after a dash). Compare side by side with Re:CoM's own Sonic Blade sleight.

## M12 step 3: KH1's Rod-list support abilities

Owner (2026-11-03, "M12 sleights and progression"): "As sora levels up in KH1, he receives abilities
like MP Haste, MP Rage, Leaf Bracer, combo plus, air combo plus, second chance and Berserk. I want
those to be added to his ability list at the levels he would normally get them (subject to change,)
regardless of level up choice". Go-ahead 2026-11-03. Research: `recom-m12-sleights.md` § 3.

All five: KH1's **Rod list** (btltbl `0x3BF8`, decision 18) on **Re:CoM's level**, unlocked by
`abilities::sync` (Attack presses, the Abilities page; `Source::Level`), whatever the level-up
choice; they arrive **unequipped** (KH1's `0x140284E10`), cost KH1's AP (btltbl `0x33D8`), and are
on Sora's Abilities page in KH1's sort-key order. KH1 counts them into the actor's ability block
`+0x184` (`0x1402A6380`: `0x140284BE0(id)` = the number of **equipped** copies in Sora's list), not
`0x142D5EC10`, so their `kh1_bit` is 0. Checked (decomp + files; `ap_tables_match_btltbl` reads
the player's btltbl). The learned notice is another step: `win::abilities::on_learned` is its call
point (a TODO for the notices merge).

| ability | KH1 id | `+0x184` | Rod level(s) | AP | sort key | bit (`ix`) |
|---|---|---|---|---|---|---|
| MP Haste | 23 | bit 0 | 21 | 3 | 304 | 25 |
| MP Rage ×2 | 24 | bits 1..3 (count) | 42, 75 | 3 each | 305 | 26, 27 |

### MP Haste

- **KH1** (Checked (decomp + files)): `0x1402BF330`, the MP charge of a hit: factor f = (enemy level
  − Sora's level) × G`+0x94` (20) / 100 + 1, **+ G`+0x95` (150) / 100 with `+0x184` bit 0**, in
  `f32`; charge = trunc(`+0x2C` × f), at least 1, at most G`+0x96` (250) × `+0x2C` / 100. G =
  btltbl + 0x58 (`0x14028F9D0` sets the pointer `0x142D22D30` = `0x142D22D98`). So at equal levels a swing
  (3) charges **7** instead of 3 (7.5, cap 7), a finisher (6) **15**, Counterattack (24) **60**: the
  cap stops it above about +0.5 of factor. It doesn't touch the refill (`0x1402A4CB0`, 30·k charge
  per point) or MP from damage.
- **Ours**: `crate::mp::hit_units_with(…, mp_haste)` (kh1-core's `mp_charge` with its
  `mp_charge_bonus`), from the existing enemy-hit path (`win::mp::on_kh1_hit`, one per KH1 damage
  record); the equipped word is read there (`win::abilities::equipped_live`, a 15-byte read per KH1
  hit). No hook, nothing per frame. The log's charge line ends `, MP Haste +1.5`.
- Checked (tests): `mp::tests::hit_charge_with_mp_haste`, `abilities::tests::rod_support_abilities`.
  Not checked: in game; no KH1 trace with MP Haste.
- **Test:** reach Re:CoM level 21 (Attack once): the Abilities page lists MP Haste (3 AP),
  unequipped. Equip it; swing at a Traverse Town Shadow of your level: the F9 trace's MP events show
  `+7` per swing hit and `+15` per finisher hit (`+3` / `+6` without it).

### MP Rage

- **KH1** (Checked (decomp)): the damage function `0x14029BBD0` keeps the HP at its start; after its
  damage records, when HP went down and `(+0x184 >> 1) & 7` (`0x140284BE0(24)`: the number of
  **equipped** copies; the Rod list gives two, which **add up**) is not 0:
  `0x1402A4AE0(actor, (int)((float)(HP before − HP after) × (float)copies))`, KH1's gauge add (charge
  capped at max MP × 30; 30·k make point k). Also for a hit that kills.
- **Sora's HP is Re:CoM's**, several times KH1's (KH1: 18 at level 1, + 3 at each kind-1 level of
  btltbl `0x3AC0` (23 levels), 87 at 76+; the Rod Dive changes no HP, `0x140284B70`). Built: **the
  HP lost is taken as the same share of KH1's max HP at Sora's level** (`kh1_support::mp_rage`,
  rounded to the nearest point), then KH1's rule. E.g. level 42 (KH1 max HP 51), Re:CoM max HP 400,
  a 40-HP hit: 5 KH1 HP → charge +5 (one copy), +10 (two). **Owner question** below.
- **Ours** (`win::kh1_support::after_damage`): inside the existing player damage-check wrapper
  (`win::attack::damage_check` on `0x1401E6DE0`), only when a hit arrived (its gate `+0x41C &
  0x200`): HP `+0x42C` before and after Re:CoM's check, max HP `+0x430`; then `win::mp::gain_charge`.
  No hook, nothing per frame (one load of `+0x41C` on frames the wrapper already runs).
- Log: `support: MP Rage x2: HP 400 -> 360 of 400 = 5 of KH1's 51 at level 42: charge +10` (40
  lines), and the MP line `charge +10 (MP Rage x2, 5 KH1 HP lost)`.
- Checked (tests): `kh1_support::tests::mp_rage_charge`, `kh1_max_hp_by_level` (the player's btltbl:
  start HP 18, the 23 kind-1 levels), `abilities::tests::rod_support_abilities`. Not checked: in game.
- **Owner question (MP Rage):** (a) the share of KH1's max HP (built: about KH1's MP per hit);
  (b) Re:CoM's HP lost as it is (KH1's code on Re:CoM's numbers: several times KH1's MP).
- **Test:** Re:CoM level 42: MP Rage on the page (3 AP), unequipped; equip it, let a Shadow hit
  Sora: the log's `support: MP Rage` line and the trace's `+n` MP event. Level 75: the second copy;
  with both equipped the charge doubles.
