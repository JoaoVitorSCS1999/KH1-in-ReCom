# M8b: summons (KH1's Summon list, Re:CoM's LV3 summon sleights)

Owner (2026-11-01): "Lets get summons working. I want the summon section on the command menu to
work like KH1, but each summon should do their lv 3 recom summon equivalent, cloud included".
Choices (owner, 2026-11-01; `design-philosophy-instructions.md`, "Summons now"): **MP from Re:CoM's
CP** (2 MP for the 20-CP cards, 3 for the 25-CP ones), paid at once on the command with KH1's
"whole MP ≥ cost" rule; **KH1's party rule skipped until the party exists**; **Re:CoM's summon
damage at both Damage settings**; the Summon row greyed and locked while one is out (no Dismiss);
Cloud with Omnislash; battle rooms only (lifted in 0.110.0: field rooms too, free, where Re:CoM's
sleight packs load: `docs/recom-field-sleights.md` § Built); each summon unlocks
with its first Re:CoM summon card; learn order (older saves: KH1's index order, Cloud last); the
start through Re:CoM's own sleight start `0x1402247A0` (its use counter counts it).

Follow-ups (owner, 2026-11-02, verbatim): "summons should be able to be used as long as you have
MP" → **KH1's once-per-battle rule dropped**; "also, sora should have iframes through all summon
animations" → **Sora invincible from the start to Re:CoM's end action**; "cloud should also be at
the top of the summons list" → **Cloud first, then the others in learn order** (§ Follow-ups).

Code: `crates/kh1-recom/src/summons.rs` (table, KH1's usable rule, learn order, save block; tested),
`src/command_menu.rs` (the Summon entry and list; tested), `src/win/summons.rs` (Re:CoM's side),
`src/win/command_menu.rs` (the world it reads, the issue, the preload), `src/win/unlocks.rs` (the
learn on a card's first pickup), assets `command_menu.rs` (the words) and `kh1_hud.rs` (KH1's
Summon panel, optional).

Steam addresses (image base 0x140000000), both exes. Tags: **Checked (decomp)**, **Checked (exe)**
= bytes of the Steam exe, **Checked (files)** = the owner's files in this session (KH1 `btltbl.bin`
and its Word.bin assets; Re:CoM `SY0002.BIN` / `SY0002.CTD`), **Assumption**.

## KH1 (Steam)

### The Summon entry and list

| what | where | tag |
|---|---|---|
| Summon (command 6, message 6) is appended as the Magic list's last entry by the Magic build, only when Sora has a spell (the list exists) and the save's magic seals are off (`save+0x874` bits 1 / 8), and only when at least one summon is learned. While a summon is out (`0x14298C7A0 & 8`) the entry is Dismiss (`0x4F`, message 323) instead. | `0x1402944C0` | Checked (decomp) |
| The Summon row (root row 3 in FM): greyed and locked with `save+0x874 & 4`, a summon in progress (`0x142867364 & 0x2000`) or `0x142D53A38 & 2`; else greyed (still opens) when no summon is usable. | `0x1402944C0` | Checked (decomp) |
| The Summon list: record type 3 at `0x1405289A8`; entries `0x2F + summon` for each byte of the save's list `+0x7D0` (7 bytes, `0xFF` = empty), grey = lock = not usable. | `0x1402944C0` | Checked (decomp + exe) |
| Learning: event syscall 568 (table `0x1404FA870`, 8-byte entries) → `0x1401B1250` → `0x1402850B0`: the summon goes into the first empty byte of `+0x7D0` (learn order). | | Checked (decomp + exe) |
| KH1's indexes: 0 Dumbo, 1 Bambi, 2 Genie, 3 Tinker Bell, 4 Mushu, 5 Simba, 6 Bahamut (words 286..292). Bahamut is unusable: the handler table `0x140528198`'s entry 6 is the string "xs_dumbo", not code. | Word.bin, `0x140528198` | Checked (files + exe) |
| Opening: confirm or Right on root row 3 runs the Magic list's last entry (`0x140293480` on it): locked → sound 4; else the Summon list opens (sound 6). Without the entry: sound 4. | `0x1402937F0` | Checked (decomp) |
| Issuing: `0x140293480`'s commit gate for every command but 0x47: refused (silently) in the take-off and unless Sora is free or in a combo window (`0x1402A2EC0`); no sound when issued; the stack unwinds, the root cursor on Attack. | `0x140293480` | Checked (decomp) |
| Place: the root's draw sets the list origin (16, 260) and "row 3" for the Summon list; it is drawn at x + 11, at row 3's y (326) when 280 + (3 + n) × 22 < 369 (one entry), else bottom-anchored at 348 − 22 n. With Summon in the Magic list, the Magic list's "fewer than 3" test counts it (its rows still end at 348). | `0x14027B870` cases 0, 1, 3 | Checked (decomp) |
| Panels: KH1's Summon list is built from `uibin_sum.bin` (string `0x1403EE5F8`; `0x14027AB70(3, 0x142859860, …)`). | | Checked (decomp + exe); its objects: not in this session |
| No help line: the menu draw writes only the commands' words (and the tab, the shortcut panel). | `0x14027B870` | Checked (decomp) |

### Usable (`0x140288F30`)

A summon is usable when all hold (grey = lock otherwise). Checked (decomp) for the tests; the meaning
of the unnamed flags is an Assumption.

- not `0x142D53A38 & 4`, not `0x142867364 & 0x10000` (event / seal flags: Assumption);
- Sora's movement state has flag 0x80 (`0x142DB3840`: only state 0, ground);
- **battle mode** `0x142867364 & 1` (the red command menu: `kh1-hud.md`);
- Sora's **whole MP** (`+0x44`) ≥ the cost (the charge doesn't count);
- not in world 3 room 0x14 (`tw21`; which room: Assumption) — there the next two are skipped;
- **not used in this battle**: bit `summon` of `0x14298C7AC` (set by the cast, cleared when battle
  mode ends, `0x140298940`; also by `0x140289410` on room changes). Ability 16 "Cheer"'s help text
  says "more time and attacks per battle" (Checked (files));
- **the party**: both party slots filled, without status `+0x88 & 0x30` (`0x14029BBC0`), alive
  (`0x1402B0030`). No character id is tested.

### Costs (btltbl params `+0x98 + 8 × summon`, byte 0; Checked (files + decomp))

| Dumbo | Bambi | Genie | Tinker Bell | Mushu | Simba | (Bahamut) |
|---|---|---|---|---|---|---|
| 3 | 1 | 2 | 3 | 3 | 2 | 3 |

Paid on the command in whole points (`0x140289210` → `0x1402A4A00(actor, −cost)`). Bytes 1 / 2 are the
gauge's drain per frame and per action.

### The cast (research; not built: Re:CoM's sleight replaces it)

`0x1402A3DB0` case 6 → `0x140289210`: flags `0x102000` (`0x100000` hides the menu), gauge = (Cheer
count + max MP) × 3000, MP paid, the summon's files loaded, a task (52000) running the timeline
`0x140528230`: +60 frames `0x140289C30` (Sora state 0x28, motion 0x40, effect 0x1C, FOV 53°, lock-on
dropped, the party's gauges hidden and Donald / Goofy flagged `0x1000000`, 50-frame fade, 40-frame
camera move); +30 `0x14028A190` (`simba.dpx` / `genie.dpx`, the other actors frozen); +60
`0x140289750` (Sora idle, the summon's entrance); then each frame the summon's handler
(`0x140395800` …) until it is done: Donald and Goofy fade back with their gauges (`0x140289620`).
Ends: the gauge at 0, Dismiss (`0x140289510`), Tinker Bell reviving Sora (`0x14029BBD0`), the summon
falling off, a room change. Checked (decomp + exe) for the calls; what each callee does: Assumption.

## Re:CoM (Steam)

| what | where | tag |
|---|---|---|
| Summon cards 0x23..0x29 (Simba, Genie, Bambi, Dumbo, Tinker Bell, Mushu, Cloud), type byte 0x33, premium, Sora's story. Friend cards 0x31..0x39 are type 5 (friends, not summons): Cloud is the only Re:CoM summon KH1 lacks. | card info `0x1405256D0` | Checked (exe) |
| Sleight recognition: the same summon ×3 → LV3 (Simba 0x35 … Cloud 0x42 Omnislash); two of the same (+ any third) → LV2 (Cloud: Cross-slash 0x41); Cloud + Stop + a Keyblade card → Cross-slash+ 0x43; Simba + 2 Keyblades → Shock Impact 0x28; Bambi + 2 Keyblades → Idyll Romp 0x3A. | `0x1401669F0` | Checked (decomp) |
| Names: `SY0002.CTD`'s list in order from id 4 (Proud Roar LV2 0x34 … Cross-slash+ 0x43), consistent with the resource map and the unlock pairs. | | Checked (files + decomp) |
| A summon card's first pickup teaches its LV2 and LV3 (Cloud: 0x41, 0x42). | `0x14015C7F0` | Checked (decomp) |
| Sleight start (Sora's vtable `+0x130` = `0x1402247A0`): the use counter `0x14020E590` (variable `sleight + 0x44`, an LV3 counted under its LV2), `initStockSkill` (vtable `+0x138`), `+0x838` = 10; 1 = started. Re:CoM calls it from its request bits `0x1401F8150` for a played sleight. | `0x1402247A0` | Checked (decomp + exe) |
| `initStockSkill` for a summon sleight: action 0x16, attack id `+0x638` = sleight + 0x4D (Twinkle, Cloud + 0x50), the resource request `0x140211790(+0x3B8, group sleight)`, AP 0 (the summon actor computes its own). | `0x140220DF0` | Checked (decomp) |
| `exeStockSkill` sub-state 1: `StockSkillList[+9]` = 3 → the summon branch: tint (0.5, 0.5, 0.5, 1) `0x1407C5D40`, motion 0x2E, Re:CoM's "summon" effect on Sora; sub-state 0x4BC waits for the resources (`0x1402106E0` = 1), spawns the actor (`0x14020EE80` by `0x14020FF40`: `CSummon*` at `+0x3B8 → +0x80`, out while `+0x88 & 0x20`), then runs it. | `0x140218950` | Checked (decomp) |
| Sora during it: Simba, Genie, Dumbo, Cloud: attached to the summon, Keyblade away (`vt+0x100`); Bambi, Tinker Bell: stands and turns (`vt+0x180` = `0x1401EF120`); Mushu: `0x140224870` mode 2. Not controllable. | `0x140218950` | Checked (decomp); "turns": Assumption |
| End: when the actor clears `+0x88 & 0x20` (or sets `player+0x418` bit 27): sub-states 0x5C0 / 0x5BF → 0x5C1 → `0x1401F6660` (Re:CoM's end action). | | Checked (decomp) |
| Damage: the actor's AP = its row column × 10 / 100 (`0x140226BB0` picks `BattleCardList` / `StockSkillList` rows; e.g. Simba `0x14022D180`), not Sora's attack stat. | | Checked (decomp) |
| Resources: on demand into a 16-slot, 2.5 MiB ring (`0x140211790`, budget `0x1407C7AE8`); **not tied to the deck**. Re:CoM loads them when cards form the sleight in the stock (`0x1401669F0`'s end), at `initStockSkill`, at `initSummon`. Pack sizes 0.56–1.43 MiB (`0x1407C7AF0`). | | Checked (decomp + exe) |
| The Field exe has no case for 0x16 or 0x1B (`0x1401FA648`, `0x1401FA65C`: the default); `StockSkillList` is battle-only. | | Checked (exe) |

## What runs (Combat = KH1, Sora's story)

### The menu (`crate::command_menu`)

- **Row 3**: with a spell and a summon learned, Summon (KH1's word 6), 0xE6 / 0x73 by its grey bit;
  else KH1's blank dash, as before. The context command keeps entry 3 (the tab's); row 3 is not it.
- **Greyed** when no summon is usable; **greyed and locked** while one is out (owner: no Dismiss).
- **Confirm or Right on row 3**: locked → KH1's error sound; else the Summon list (KH1's open sound).
- **The list**: Cloud first when learned (owner), then the learned summons in learn order, each
  greyed and locked when KH1's rule (without its once-per-battle test, owner) says no: battle
  (Re:CoM's battle room), KH1 state 0, whole MP ≥ the owner's cost, none out (a second sleight can't
  start mid-sleight: Re:CoM's action 0x16 is busy until its end action); the party counts as present (`summons::party_ready`, the hook for the party milestone). Up /
  Down wrap; Left / cancel close it (cancel: the root back on Attack); its cursor stays as KH1's lists'.
- **Confirm on an entry**: greyed → error sound; KH1's commit gate (free or a combo window) may refuse
  it silently; else the summon is issued (no menu sound), the stack back to Attack.
- **Place**: x 27; one entry at 326 (beside row 3; Cloud alone too), more bottom-anchored at
  348 − 22 n (Cloud on the top row). The Magic list
  with two spells and Summon sits at 304 (KH1's count includes Summon).
- **Words**: KH1's own 286..291 from the player's Word.bin (asset step); Cloud: "Cloud" (KH1 has no
  word; Re:CoM's card-name text is not in our word system) under our id `0x8029`. A manifest from an
  older asset step: English names in Re:CoM's look, none in KH1's look (re-run the step).
- **KH1's look**: KH1's Summon panels from `uibin_sum.bin` when the asset step finds its 17 objects
  laid out as the Magic list's; else the Magic list's panels without the MAGIC title (logged once).

### The summon (`win::summons`)

- **Issued** from the menu's step, inside the request-bits wrapper (where Re:CoM's own
  `0x1401F8150` starts a played sleight), the frame it is chosen (KH1's command step runs
  `0x140289210` that frame): battle only; a live swing's weapon volume ends (`0x140224FB0`, the
  specials' rule); `player+0x82C` = the LV3 sleight; **Sora's vtable `+0x130` is called** (checked to
  be `0x1402247A0`). When Re:CoM's action is then 0x16: KH1's MP paid (whole points), the summon
  out.
- **Re:CoM runs the rest**: its tint, motion 0x2E and "summon" effect, its resource wait, the
  `CSummon*` actor, the hits and damage (Re:CoM's at both Damage settings), its end action. Our
  `exeStockSkill` wrapper (with the specials in) passes it through untouched.
- **After it**: Re:CoM's end action leaves Sora in its own action; our move-exe wrapper's sync test
  (action, previous action and time against our last KH1 frame, `movement::sync_ok`) fails, so the
  KH1 frame takes Sora back fresh, as after Re:CoM's hit reactions and its win action. Checked
  (code); in game: to test.
- **Out** (`win::summons::out`): from our start while Sora's action is 0x16 in the battle room
  (`player+0x390` ≠ 0); the first check that sees otherwise ends it (Re:CoM's end action, its win
  action, the room's end, another player). Row 3 greyed and locked meanwhile.
- **Invincible** (follow-up, below) for exactly that span.
- **Preload** (optional, built): the Summon list's cursor landing on a summon, in battle, calls
  Re:CoM's `0x140211790(player+0x3B8, sleight, 0)` once, as Re:CoM preloads a sleight its stock
  forms. Re:CoM's cache decides what stays.

### Follow-ups (owner, 2026-11-02)

1. **No once-per-battle rule.** `summons::usable` = battle, KH1 state 0 (flag 0x80), whole MP ≥ cost,
   none out (and the party hook). The used-this-battle bits and their reset are removed (KH1's
   `0x14298C7AC` is not modelled). The row stays locked while one is out: a second LV3 can't start
   while Re:CoM's sleight action runs.
2. **Sora invincible through the whole summon.** Re:CoM's player damage check `0x1401E6DE0` (already
   wrapped: `attack::damage_check`) has a protection exit: with a hit arrived (`+0x41C & 0x200`) and
   `+0x418 & 1` set (or the hit-immunity timer `+0x7BC`, or `+0x7B4`), it zeroes the hit's value
   (`+0x722`) and returns: no damage, no reaction. That bit is Re:CoM's roll protection and our Sonic
   Blade's invincibility (`docs/recom-m9-abilities.md` § 9c.1). Checked (decomp). While the summon is
   out, the wrapper sets the bit for each check and restores it after (left set when it already was:
   Re:CoM's attached summons set `+0x418 |= 0x1001` themselves and clear it at their end,
   `exeStockSkill` sub-states 0x50A.. / 0x5C0; the standing ones, Bambi / Tinker Bell / Mushu, don't).
   - **Coverage**: the Battle exe (`0x1401F9840`) runs the request bits (our start) and then the damage
     check every frame unless the action is 0x17, 0x1C, 0x1D (or 0x20 without a flag): action 0x16 is
     checked every frame, from the start frame on, whether Sora is attached (Simba, Genie, Dumbo,
     Cloud) or standing (Bambi, Tinker Bell, Mushu). Checked (decomp).
   - **The end**: "out" is read in the check itself (two loads): it ends the frame Sora's action is no
     longer 0x16 (Re:CoM's end action `0x1401F6660`; its win action at a battle's end, 0x1C, where the
     check doesn't run anyway), or the battle room ends, or the player changes (a room change). The
     bit is ours only inside the check, so nothing is left set.
   - Not covered: a card break of Sora's sleight (`+0x428 & 0x20`, the check's first block, which ends
     actions 0x16 / 0x1B): not a hit; at KH1 enemies play no cards (decision 29; human bosses keep
     hidden decks: whether their cards can break Sora's cardless sleight: Assumption, untested).
   - Log: `summons: a hit on Sora during the summon (value n): skipped …` (first 8), and the hits
     skipped in the end line.
3. **Cloud first** (`summons::list_order`, applied in the menu's build): Cloud, then the others in
   learn order; the save's learn order is unchanged. KH1's placement holds (tests: one entry, Cloud
   alone included, at 326; three bottom-anchored at 282, Cloud on top).

### Learning (`summons::sync`, `win::summons::sync`)

- **Our save block** `save + 0x26AE ..= 0x26B6` (after the ability block, in the spare tail): `+0`
  version (1), `+1..+7` KH1's list (summon index + 1, 0 = empty), `+8` zero.
- **A summon card's first pickup** (the unlocks wrapper on `0x14015C7F0`, either Combat setting, Sora's
  story) appends the summon (KH1's `0x1402850B0` rule).
- **An older save** (block never written): at the first read, every summon whose card (plain or
  premium) was ever obtained (`0x14015B3F0`), in KH1's index order (Dumbo, Bambi, Genie, Tinker Bell,
  Mushu, Simba), Cloud last. Re:CoM keeps no order, so the true one can't be recovered.

## The table as built

| KH1 summon | Re:CoM card | LV3 sleight (combo) | attack id | actor | resources (group → `PLAYER/<pack>`) | LV3 `StockSkillList` cols (attr) | MP (CP) |
|---|---|---|---|---|---|---|---|
| Dumbo | 0x26 | Splash LV3 0x37 (Dumbo ×3) | 0x84 | `CSummonDumbo` | 0x28 → 0302 | 200, 0, 0, 30000 (3) | 2 (20) |
| Bambi | 0x25 | Paradise LV3 0x39 (Bambi ×3) | 0x86 | `CSummonBambi` | 0x27 → 0303 | 10, 0, 0, 0 (1) | 2 (20) |
| Genie | 0x24 | Showtime LV3 0x3E (Genie ×3) | 0x8B | `CSummonGenie` | 0x26 → 0305 | 600, 40, 24000, 3 (5) | 2 (20) |
| Tinker Bell | 0x27 | Twinkle LV3 0x40 (Tinker Bell ×3) | 0x90 | `CSummonTink` | 0x29 → 0306 | 22, 15, 100, 15000 (6) | 2 (20) |
| Mushu | 0x28 | Flare Breath LV3 0x3C (Mushu ×3) | 0x89 | `CSummonMushu` | 0x2A → 0304 | 100, 0, 0, 35 (2) | 3 (25) |
| Simba | 0x23 | Proud Roar LV3 0x35 (Simba ×3) | 0x82 | `CSummonSinba` | 0x25 → 0301 | 500, 0, 0, 0 (1) | 2 (20) |
| Cloud (Re:CoM) | 0x29 | Omnislash 0x42 (Cloud ×3) | 0x92 | `CSummonCloud` | 0x2B → 0307 | 600, 600, 800, 0 (1) | 3 (25) |

Commands `0x2F + index` (Cloud in KH1's unused Bahamut slot, 0x35). Ids, ids' arithmetic, classes and
packs: Checked (decomp); class names: Checked (decomp, the vtables' RTTI names, `CSummonSinba` and
`CSummonTink` as Re:CoM spells them); columns and CP: Checked (files, the session's `SY0002.BIN`; the
DLL reads nothing of them: Re:CoM's actor reads its live tables).

## Sites (`sites::SUMMON_CHECKED`, `SUMMON_ALSO`, `SUMMON_PTRS`; bytes re-read with pefile)

No hook and no patch: calls only, all checked at start (a mismatch: no summons, logged).

| site | bytes | use |
|---|---|---|
| sleight start `0x1402247A0` | `40 53 48 83 EC 20 48 8B D9 8B 89 2C 08 00 00 8D 41 FC 83 F8 55 77 4B` | called (through Sora's vtable) |
| Sora vtable `+0x130` `0x1405A9A68` | → `0x1402247A0` | checked at start and at each start |
| summon class of a sleight `0x14020FF40` | `45 33 C0 83 C2 CC 83 FA 0F 77 57` | checked (the id range 0x34..0x43) |
| card info 0x23..0x29 `0x140525900..0x140525960` | `33 0x yy 03 00 00 00 00` each | checked (the summon cards) |
| `initStockSkill` `0x140220DF0`, sleight resources `0x140211790`, weapon attack end `0x140224FB0` | their lists' entries | checked (`SUMMON_ALSO`) |

`check_sites_rs.py` on the Steam exe: 539 sites, 0 mismatches (Field table and cave unchanged).

## Per frame

The menu's build (already per frame): a 9-byte read of our save block and two loads (the battle
flag, the action while one is out). The preload: one compare unless the cursor moved onto a summon.
The damage-check wrapper (already per frame): one load, and while a summon is out three more and the
bit's set / restore.

## Logs and trace

- start: `summons: KH1's Summon list at combat = kh1 … Dumbo -> Splash LV3 (sleight 0x37, 2 MP), …`.
- learning: `summons: learned Simba (Proud Roar LV3, card 0x23): Simba card 0x23 obtained for the first
  time` (older saves: `…, an older save: KH1's index order, Cloud last`).
- each summon: `summons: Simba -> Re:CoM's Proud Roar LV3 (sleight 0x35, attack 0x82, CSummonSinba
  from PLAYER/0301), 2 MP, from action 0x…; Sora invincible until it ends`; refusals (`not in battle`,
  `MP short`, `Re:CoM's sleight start refused …`); hits skipped; the end (`Simba: Re:CoM's Proud Roar
  LV3 ended (action 0x…); invincibility off (n hits skipped); the KH1 frame takes Sora back`).
- the menu's events (log and F9 trace): `open summon (3 learned, grey 0x4)`, `summon Simba issued /
  greyed / not-now`, `refused Summon`; the menu token gains `sum`, `sum:grey` or `sum:lock`, and
  `menu s:<cursor> … sg<grey>` while the list is open.

## 1:1 checks

- Checked (decomp + exe + files; tests `summons::tests`, `command_menu::tests::summon_*`): the entry's
  condition (a spell, a summon), row 3's word and brightness, the open / error sounds, the list's
  order (Cloud first), greying (MP whole points, battle, state flag 0x80, one out), the commit gate,
  the list's place (326 / 348 − 22 n) and the Magic list's with Summon (304 for two spells), the
  learn-order rule, KH1's words 286..291 in all six languages (`owner_words` with the session's
  Word.bin files).
- Checked (decomp): Re:CoM's LV3 recognition, ids, start path, resource loading, end; the damage
  check's protection exit and that the Battle exe runs it every frame of action 0x16.
- Not checked: in game; KH1's `uibin_sum.bin` objects (the asset step prints what it found); the
  durations and hit counts of Re:CoM's LV3 sleights (Re:CoM's own, not changed).

## Not KH1's (by design or yet)

- **The summon itself is Re:CoM's LV3 sleight** (owner): no KH1 entrance, no summon partner staying
  with a gauge, no summon commands, no Dismiss; Sora is Re:CoM's (not controllable) until it ends.
- **MP** from Re:CoM's CP (owner), not KH1's costs above.
- **No once-per-battle rule** (owner): KH1's `0x14298C7AC` test is dropped.
- **Cloud first** in the list (owner); KH1 lists in learn order only.
- **Sora invincible** through the summon (owner); KH1's Sora stays hittable beside his summon.
- **The party rule** is skipped (owner) until the party exists (`summons::party_ready`).
- **The menu stays up** during the sleight (row 3 greyed and locked; owner); KH1 hides it during its
  summon's entrance (`0x142867364` bits 20 / 14).
- KH1's event seals (`save+0x874 & 4`, `0x142D53A38`, `0x10000`) and the tw21 exception have no
  Re:CoM counterpart: not modelled.

## Assumptions

- Calling `0x1402247A0` from our request-bits wrapper (after Re:CoM's own request bits ran) is
  equivalent to Re:CoM's own call inside `0x1401F8150`; no card record (`+0x808..`) is needed by the
  summon sleights (they read `+0x82C`, the row, the target and the resources).
- `player+0x390` ≠ 0 exactly in Re:CoM's battle rooms (as the specials use it).
- Every hit on Sora goes through the damage check `0x1401E6DE0` (decision 31's reading); a hit kept
  pending by the protection exit is dropped by Re:CoM as after a roll (Re:CoM's own rule).
- Bambi / Tinker Bell "stand and turn" (from `0x1401EF120`'s code).
- "Cloud" is Cloud's name in every language.

## What the owner should test (re-run the asset step first)

1. `kh1-recom-build`: the command-menu words now include 286..291 (and Cloud); the KH1 HUD step prints
   `KH1 HUD: Summon list (M8b) from group …` or a note saying why `uibin_sum.bin` wasn't used (please
   send that line).
2. Load a save with a Re:CoM summon card and a magic card. The log: `summons: learned …` (an older
   save: KH1's index order, Cloud last).
3. Field: row 3 reads **Summon**, greyed; Up to it, confirm: the list opens with every summon greyed;
   confirm on one: the error sound. Left / cancel close it.
4. Battle, Sora standing, enough MP: confirm on a summon: MP drops by 2 (3 for Mushu / Cloud) at once,
   Re:CoM's LV3 plays exactly as Re:CoM's own (tint, "summon" effect, the summon, its attack). Compare
   with the same LV3 sleight from cards at Combat = Re:CoM.
5. While it runs: row 3 greyed and locked (error sound). After it: Sora walks, attacks and casts at once
   (the KH1 frame took him back).
6. Invincible: let enemies hit Sora during each summon (attached: Simba, Genie, Dumbo, Cloud; standing:
   Bambi, Tinker Bell, Mushu): no HP lost, no flinch; the log's `a hit on Sora during the summon …`
   lines. Right after it ends he can be hit again (no lingering invincibility).
7. The same summon again in the same battle: usable as long as the MP pays (no once-per-battle).
8. Cloud is the top entry of the list whenever learned; the others follow in learn order. One entry
   sits beside row 3; more end at the bottom row.
9. Not enough MP (e.g. 2 MP and Cloud): greyed; in the air: all greyed.
10. Mid-combo (in a combo window): a summon is taken; mid-swing outside the window: nothing (no sound).
11. Damage: Re:CoM's summon damage at `damage = recom`; at `damage = kh1` scaled by the enemy's KH1-scale max HP / its Re:CoM max HP (since the owner's "KH1 damage scaling", 2026-11-03: `docs/recom-m12-sleights.md` § 8.5.2).
12. Cloud: Omnislash. The list's word "Cloud".
13. Ending a battle with a summon (its attack kills the last enemy): no stuck invincibility in the next
    battle.
14. Send `kh1-recom.log` (the `summons:` lines) and, for one summon, an F9 trace from the command to
    a few frames after it ends.
