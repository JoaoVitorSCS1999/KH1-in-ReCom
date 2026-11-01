# M7 stage 7.1: KH1's MP (decision 34)

At Combat = KH1, in Sora's story, Sora has KH1's MP: max MP by level (and the equipped
Keyblade), MP points and KH1's charge counter, charge from KH1 hits, KH1's refill and payment
rules, kept per save. No HUD yet (the MP bar is stage 7.2) and nothing pays yet (spells, 7.4+).
At Combat = Re:CoM, or in Riku's story, nothing of this runs and Sora has no MP.

Code: `crates/kh1-recom/src/mp.rs` (rules and save layout, tested), `src/win/mp.rs` (glue).
Research: `m7-menus-mp.md` § 1.5 / 2.4 and `m7-magic.md` § 2.3 / 4.1.2 (kept with the decomp
tools, not in the repo).

Steam addresses (image base 0x140000000). Tags: **Checked (decomp)**, **Checked (exe)** = bytes
or disassembly of the Steam exe, **Checked (files)** = the player's KH1 `btltbl.bin`,
**Assumption**.

## KH1's rules

KH1 actor stats: `+0x44` MP (whole points), `+0x48` max MP, `+0xBE` charge (u16). Save record
(`save + 4 + member × 0x74`): `+0` level, `+3` MP, `+4` base max MP, `+0x38` charge.

| rule | KH1 (Steam) | tag |
|---|---|---|
| **Point k is worth 30·k charge.** Each frame, when charge ≥ (MP + 1) × 30: MP + 1 (clamped at max) and charge − (MP + 1) × 30. One point per frame. Called from the actor pre-step `0x1402A43A0`. | `0x1402A4CB0` | Checked (decomp) |
| Add points: MP + n, at most max, at least 0; writes record `+3`. (A status `+0x88 & 0x20` without ability 9 scales the change by G`+0x88` = 0.5: not reachable for our Sora, not ported.) | `0x1402A4A00` | Checked (decomp + files) |
| Add charge: + n, capped at max × 30; when negative, borrows a point (MP − 1, charge + old MP × 30) or becomes 0 with no point; no cap after a borrow; writes record `+0x38`. Only for party members with a record (Sora). | `0x1402A4AE0` | Checked (decomp) |
| **Can pay:** cost ≥ 100: cost / 100 ≤ MP. Cost < 100: cost ≤ (MP + 1)·MP / 2 · 30 + charge. | `0x1402A4C10` | Checked (decomp) |
| **Pay:** cost ≥ 100: MP − cost / 100 (truncated; disasm uses the ×0xAE147AE1 division). Cost < 100: charge − cost; negative and MP > 0: MP − 1, charge = (charge − cost) + old MP × 30; negative and MP 0: charge 0 (the local starts at 0); not negative: capped at max × 30. It doesn't test "can pay" itself. | `0x1402A5B60` | Checked (decomp) |
| The spells pay through `.mag` syscall 47: table `0x140529180` entry 47 (16-byte entries) = `0x1402BA250` → `0x1402B37F0` → `0x1402A5B60(actor, spell data +0x30)`. | | Checked (exe + decomp) |
| **Costs** (spell data `+0x30`, btltbl offsets from the level table `0x140528620`): Fire 30, Blizzard 30, Thunder 100, Cure 100, Gravity 100, Stop 200, Aero 200, the same at all three levels. So Fire / Blizzard cost a third of point 3 at 3 MP, half of point 2 at 2, all of point 1 at 1. | btltbl `0x5F28..0x67E8` | Checked (files + exe) |
| **Charge per hit:** attack data `+0x2C` × f, f = (enemy level − Sora's level) × G`+0x94` / 100 + 1 in `f32` (`mulss`, `divss` 100.0, `addss` 1.0; constants at `0x14042F984` / `0x14042F820`), + G`+0x95` / 100 with ability block `+0x184` bit 0; truncated (`cvttss2si`), at least 1, at most G`+0x96` × `+0x2C` / 100. Only when the hit makes a damage record (`0x1402BF630` non-null), the target is valid, the attacker has a party record, and the **target's level `+0xDC` > 0** (else no call at all). G = btltbl + 0x58 (`0x14028F9D0`): `+0x94` = 20, `+0x95` = 150, `+0x96` = 250. | `0x1402BF330` | Checked (decomp + exe + files) |
| `+0x2C` per row: swings 3 (`0x4B78`, `0x4BE8`, `0x4CC8`, `0x50B8`, `0x5128`), finishers 6 (`0x4C58`, `0x4D38`, `0x5048`), Counterattack 24 (`0x4E18`), Guard 0 (`0x4DA8`). kh1-core's `Moveset::kh1_defaults` has the same numbers. | btltbl | Checked (files) |
| Charge from damage taken: HP lost × ((`+0x184` >> 1) & 7), an ability count (ability 0x18, set by `0x1402A6380`). The hit bonus bit 0 is ability 0x17. **Both need abilities: M9.** Names (MP Rage / MP Haste): Assumption. | `0x14029BBD0`, `0x1402A6380` | Checked (decomp) |
| **Max MP:** stats build = record `+4` + the weapon's `+0x38` + accessories; MP = min(record MP, max); charge = min(record charge, max × 30). KH1 has no Magic stat: weapon `+0x38` is the MP bonus. | `0x1402A5130` | Checked (decomp) |
| **Level-up:** `0x1402B51B0` applies Sora's table btltbl `0x3AC0` (`0x140528CC0[0]`), byte [L − 2] for reaching L, through `0x1402B5660`; kind 2 = record max MP + 1 and, live, stats max + 1 then add points (+1). Kind 2 only at levels 12, 24, 36, 48, 60. No MP refill at level-up (`0x14029BA50` after it is the revive helper). Start: 3 (new-game record btltbl `0x39D8`; the owner's Dive path keeps it). | | Checked (decomp + files) |
| **Refills:** full restore (MP < max → max, charge 0; at full nothing changes) `0x1402A56F0`, from event syscall `0x1401B12D0` and the party swap `0x1403A1540`. A save point (object `0x1402ACE20`, handler `0x1402AD0B0`, opens the save menu on Triangle) gives, while Sora stands in it in a ground state (`+0x70` 0 or 0x18), HP + 2 every frame and, when MP < max, MP + 1 with the charge emptied every third frame (the update steps a counter 0 → 1 → 2 → 0, `0x1402AD081..0x1402AD0A2`; the MP part runs at 0, `0x1402AD208`): 20 MP a second at 60 fps. It loops over the three party slots. Built 0.41.0: the MP part on Re:CoM's save point (`win::mp::save_point`, below). `0x1403A0BF0` empties every member's MP and charge (an event). That the `0x1402ACE20` object is the save point: Assumption (its handler opens the save / world-map menu `0x140196CD0`). | | Checked (decomp) |
| No KH2-style MP Charge. | | Checked (decomp: no other writer), `m7-menus-mp.md` |

## In Re:CoM

- **Store:** a block of Re:CoM's own save (below). Re:CoM saves, checksums, loads, snapshots and
  zeroes it at new game with the rest of its 0x2AD4-byte save block (`recom-keyblade-equip.md`).
- **Max MP** = 3 + one per level 12 / 24 / 36 / 48 / 60 reached by **Sora's Re:CoM level** (the
  menu's, `*0x14087CBF8 + 0x1C`, as his KH1 Strength, decision 18) + the equipped Keyblade's KH1
  `+0x38` (the stats file's `magic` for the 16 KH1 Keyblades: Crabclaw, Fairy Harp, Lionheart,
  Oathkeeper +1; Spellbinder, Lady Luck, Ultima Weapon +2; Diamond Dust +3; Metal Chocobo,
  Oblivion −1; One-Winged Angel −2; the rest 0). Checked (files: `kh1-stats.json` matches KH1's
  weapon rows). The six KH2 Keyblades: 0 (their `magic` is KH2's Magic stat; owner question).
- **Level-ups** come from Re:CoM: its level-up screen `0x14010EF10` raises the save's level one at
  a time (`DAT_14087CBF8[7]`); in battle the player's own copy `player+0x444` rises first
  (`0x1401E5730`) and the save's follows at the screen. Checked (decomp). The block stores the
  level it was written at; when read at a higher level, KH1's kind-2 bonus (max + 1, MP + 1) is
  applied for each 12 / 24 / 36 / 48 / 60 passed (no hook; the level-up is seen at the next MP
  read, and stored at the next MP write).
- **Charge from hits:** in the existing enemy-hit wrapper (`0x14017F7E0`), for each KH1 hit an
  enemy takes (`take_kh1_hit`: one per enemy part per swing, as KH1's one damage record per
  target per attack): Sora's current KH1 row's attack data `+0x2C`, the enemy's **KH1 twin's
  level** (decision 21's record, `win/stats.rs`) and Sora's Re:CoM level, through kh1-core's
  `mp_charge` (`0x1402BF330`'s port). Twin level 0 (KH1's Destiny Islands Shadow) charges
  nothing, as KH1. **Enemies with no twin** (Re:CoM-only enemies and every boss) use Sora's own
  level (factor 1.0): provisional, owner question.
- **Refill:** the existing request-bits wrapper (`0x1401F8150`, called by Re:CoM's Field
  `0x1401FA0F0` and Battle `0x1401F9840` exes each frame, after their early-out
  `0x1401B0870(1)`) runs one KH1 refill step per frame while a point is waiting; otherwise one
  atomic load. Checked (decomp); that the early-out is Re:CoM's pause / event stop: Assumption.
- **No new hook**, no byte patch, no new site.

## The save block (layout v1)

`save = *(u64*)0x14087B110`. The block is `save + 0x2675 ..= save + 0x2684` (16 bytes), right after
the equipped-Keyblade byte (`save + 0x2674`), inside the spare tail `save + 0x246B ..= 0x2817`
that no Re:CoM code reads or writes (Checked (decomp + exe scan); a pointer from elsewhere:
Assumption, logged at the first camp open; the owner's level-99 save had the range all zero).

| save + | size | field |
|---|---|---|
| `0x2675` | 1 | layout version: **0 = never written** (full MP, no charge, at the current level), 1 = this layout |
| `0x2676` | 1 | MP, whole points (KH1 record `+3`) |
| `0x2677` | 2 | charge, u16 little-endian (KH1 record `+0x38`) |
| `0x2679` | 1 | Sora's Re:CoM level when the block was last written |
| `0x267A` | 7 | stage 7.11: spell levels, Fire, Blizzard, Thunder, Cure, Gravity, Stop, Aero (KH1 save `+0x492 + spell`); 0 = not learned. Written by the unlocks (`recom-m7-unlocks.md`), raised only. |
| `0x2681` | 3 | stage 7.12: shortcut slots 1..3 (KH1 save `+0x844..0x846`); 0 = none, else spell + 1. Filled by KH1's learn rule (the first empty slot), once from the spells already learned for an older save (`recom-m7-command-menu.md` § 7.12). |
| `0x2684` | 1 | bit 0 (stage 7.11): the spell levels were synced from Re:CoM's card collection for this save (0 in every older save: the first read at the KH1 setting syncs them). Bit 1 (stage 7.12): the shortcut slots were set up (0 in every older save: the next sync fills them once). Other bits zero. |

- **Layout v2 (M8 stage 8a.1):** the items extension follows this block at `save + 0x2685 ..=
  0x269E` (Sora's item slots and stock, its own version byte; `docs/recom-m8-items.md` § The save).
  These 16 bytes and their version byte are unchanged.
- **All-zero compatible:** a save that never used KH1 MP reads as full MP and no charge, and
  reading never writes. The block is written only when MP or charge changes (a hit, a refill, a
  payment).
- **KH1's record semantics:** as KH1, only the fields an operation writes go back (a charge write
  leaves the stored MP; a point change writes the MP); the live values are the stats build's
  min(stored, max). So, as in KH1, MP above the max of a lower-max Keyblade comes back when the
  Keyblade is changed back before MP changes.
- **Versions:** fields never move; a later layout only adds fields and raises the version; a
  write never lowers it.
- Re:CoM's retry snapshots copy the whole save block: a Continue after a game over restores the
  MP from before the checkpoint, like Re:CoM's own deck edits. Checked (decomp); when snapshots
  happen: Assumption.
- The Keyblades page's spare-range check now ignores this block too.

## API (for later stages; `win/mp.rs`)

`current() -> Option<(mp, max, charge)>`, `can_pay(cost)`, `pay(cost, why)`,
`gain_charge(units, why)`, `restore_full(why)`; costs in KH1's encoding (`mp::cost`: ≥ 100 whole
points, < 100 charge units). None / false / no-op at the Re:CoM setting, in Riku's story or with
no save block. `pay` follows KH1 and doesn't test `can_pay`. Stage 7.4 uses them: `can_pay` in
the cast's usable test, `pay(30)` at Fire's release frame (`recom-m7-magic.md`).

## Logs and trace

- Start: `mp: KH1 MP at combat = kh1 in Sora's story (M7 stage 7.1): save+0x2675..=0x2684 layout v1; …`
- First read per session: `mp: save+0x2675: layout v0 (never written: full MP), Sora level 1, Keyblade MP bonus +0: MP 3/3, charge 0`
- Up to 80 lines per session: `mp: charge +3 (row 0xd +0x2C 3, enemy level 6 (twin), Sora level 3): MP 0/3 c27 -> 0/3 c30`,
  `mp: refill: point 1 made: MP 1/3 c0`, `mp: paid 30 (…)`, and a twin of level 0.
- F9 trace, events column: `…; mp <MP>/<max> c<charge> [+<units>] [pt<k>] [pay<cost>] [+0lv0]`.
- The camp's spare-range line: `… all zero apart from our bytes (save+0x2674 = …; MP block save+0x2675..=0x2684 v1)`.

## 1:1 checks

- Checked (decomp + exe + files): every rule and number in the table above; the unit tests
  replay them (point k needs 30·k, one point per frame, hits capped before the frame's refill,
  banked charge at full MP, Fire's borrow from the top point, the 16-bit store, the hit charge
  in `f32` incl. 1.2f × 6 = 7 and 1.2f × 24 = 28, the cap and floor, level 0).
- Not checked: no KH1 trace of MP and `+0xBE` (would confirm Fire / Blizzard at 30 units in play
  and the per-frame refill on screen); nothing seen in game.
- Not 1:1 by design or yet: Sora's level is Re:CoM's (decision 18); enemies without a twin
  (provisional rule); the hit's place in Re:CoM's frame (the refill may come one frame earlier or
  later than KH1's when Re:CoM updates enemies before Sora: Assumption, ±1 frame); a level-up's
  point is applied at the next MP read instead of mid-battle.

## Assumptions and risks

1. The request builder runs once per frame of Sora's update in field and battle (both exes call
   it unconditionally after their early-out `0x1401B0870(1)`: Checked (decomp)); that the
   early-out covers the moments KH1's actors don't update (events, menus): Assumption.
2. Boss classes that use their own hit handler instead of `0x14017F7E0` get no KH1 damage today
   and therefore no charge either (same path).
3. A hit consumed after its swing ended reads the next swing's row (as the KH1 damage does).
4. KH1's global flag `0x142867364 & 0x800` (no MP change) and the `+0x88 & 0x20` halving are not
   ported (not reachable for our Sora: Assumption).

## Open questions (owner)

1. **Charge from enemies with no KH1 twin** (Re:CoM-only enemies, and every boss, which has no
   twin record yet): (a) Sora's own level, factor 1.0 (built, provisional); (b) no charge (KH1's
   level-0 rule); (c) the world's KH1 boss level (needs boss records).
2. **Refill to full in Re:CoM** (KH1: save points over time, story restores): Re:CoM fills HP at
   the level-up screen's HP choice (`0x14010EF10`), in `0x1401AC9D0` (rooms outside
   0x4B0..0x4E7) and in `0x140237CE0` (with the retry snapshot); which of these is the save point
   isn't confirmed. (a) none for now; (b) wherever Re:CoM fills HP; (c) at save points only, once
   found.
3. **KH2 Keyblades' MP:** (a) 0 (built); (b) their KH2 Magic as KH1's MP bonus.

## What the first in-game test must check

1. Log: the `mp:` start line, and the first-read line with v0 and full MP for Sora's level.
2. At level 1: max 3; at level 12+: 4, … 60+: 8. Equip Diamond Dust: max + 3; One-Winged Angel:
   − 2 (MP clamps down, comes back with the Kingdom Key if nothing changed in between).
3. F9 trace while swinging at a Traverse Town Shadow: `+3` per swing hit, `+6` per finisher hit,
   `+24` per Counterattack hit (at Sora's level = the twin's; other levels by KH1's factor);
   `pt1` when the charge reaches 30, `pt2` at 60 more, `pt3` at 90 more; MP stops at max while the
   charge keeps rising to max × 30.
4. Save, quit, load: MP and charge come back. A save that never used KH1: camp spare-range line
   shows `v0`.
5. Combat = Re:CoM: no `mp:` lines, the trace shows `mp -`.
6. Riku's story: no `mp:` lines.

## Refills and the KH2 Keyblades (0.15.2, decision 35)

Owner: MP refills "where HP refills"; the KH2 Keyblades' max-MP bonus is their KH2 Magic.

- Re:CoM sets HP = max HP (`*0x14087CBF8 = [1]`) in three places (Checked, decomp + exe):
  - `0x1401AC9D0`, a game-state step: when its fade task reports 6, outside rooms
    0x4B0..0x4E7 (`0x14087B838`, read by `0x140233700`) and unless `0x140235F60()` sends it to
    `0x1401A9D20`, it fills HP and sets `+0x580 = 8`. Wrapper (`40 53 48 83 EC 20 48 8B D9`):
    after it, a transition into 8 outside those rooms with HP full → MP full.
  - `0x140237CE0`: when `0x14087B84C & 2`, after the retry snapshot `0x1401B1410(0)`, it calls
    `0x1402374C0(0)` at `0x140237F3A` and fills HP. That call (`E8 81 F5 FF FF`, context checked:
    `33 C9 E8 D8 94 F7 FF 33 C9` before, `48 8B 0D B2 4C 64 00 8B 41 04 89 01` after; no branch
    lands inside it, `.text` scan) is retargeted to ours, which calls `0x1402374C0` and then fills
    MP. The function's own prologue can't be hooked (its second instruction is RIP-relative).
  - The level-up screen `0x14010EF10`: the HP choice raises max HP by 15 and fills HP. Wrapper
    (`48 8B C4 55 41 56 41 57 48 8D 68 A1`): max HP raised during the call and HP full → MP full.
- Assumption: these are all of Re:CoM's full heals (no other `HP = max HP` writer found); what
  each moment is in play (save room, floor change, continue) is not traced.
- MP is filled after the snapshot, as HP is, so a retry restores the earlier MP with the HP.
- KH2 Keyblades: `keyblade_bonus` now uses every Keyblade's `magic` (KH2's Magic for the six).

## KH1's save point MP refill (0.41.0; owner, 2026-11-01: "standing on a save point should refill MP")

- **KH1** (Checked (decomp + exe)): above (refills row): 1 MP every third frame and the charge
  emptied, while MP < max; HP + 2 a frame; all three party members.
- **Re:CoM** (Checked (decomp)): `CGimmickSavePoint` update `0x1401C5180` (vtable slot
  `0x140584768`): Sora on it = within 0.8 m and under 0.2 m above it; the frame he steps on it
  (`+0x2C0` 0 → 1) it heals his HP to full at once (`0x1401E59A0`) and plays `rec_svpt.dpx`;
  leaving clears `+0x2C0`. No MP.
- **Built:** a wrapper on `0x1401C5180` (13 bytes checked: `40 57 48 83 EC 40 80 B9 C3 02 00 00
  01`): Re:CoM's update, then, every third call while `+0x2C0` = 1 and KH1's MP is live,
  `Mp::save_point_tick` (MP + 1, charge emptied, below max). The HUD hears it as any MP change.
- **Not KH1's:** Re:CoM's on-it test stands for KH1's ground-state test (Assumption); one counter
  for the room (KH1's is per save point; a room has one); in Re:CoM's 30 fps rooms the wrapper
  runs per game frame (Assumption: none has a save point). KH1's HP part and the party refill are
  not added (Re:CoM heals HP itself; no persistent party yet): owner question.
