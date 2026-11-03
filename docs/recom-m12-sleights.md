# M12: sleights and progression (research)

Owner's ask (2026-11-03, verbatim in `design-philosophy-instructions.md`, "M12 sleights and
progression"): spells learned with their card and upgraded by bosses / events; d-pad Left swaps the
command menu's bottom row from Summon to "Sleights", which lists only the **equipped** sleights; each
sleight unlock (not friend sleights, not the generic magic tiers, not those with a KH1 combo-modifier
/ finisher / special twin) becomes an **ability equipped with AP**, costs by damage; at Combat = KH1
the level-up's **CP choice becomes "AP Up" (+2 AP)**, the only AP source besides the starting AP;
sleights as **Customize shortcuts**; KH1's level-learned abilities (MP Haste, MP Rage, Leaf Bracer,
Combo Plus, Air Combo Plus, Second Chance, Berserk) at KH1's levels **regardless of the choice**,
with a notice; a notice when a sleight "only needing cards" becomes available (Idyll Romp named).

Research only: no code. Steam addresses (both images at `0x140000000`). Tags: **Checked (decomp)**,
**Checked (exe)** (Steam exe bytes), **Checked (files)** (the owner's files in the session: Re:CoM
`SY0002.BIN` / `SY0002.CTD`, `WORLD00` / `WORLD01` `.ABC` event scripts; KH1 `btltbl.bin`),
**Assumption**. Web facts (KH wiki's sleight page, khwiki.com "Sleits") are marked
**Assumption (wiki)**: they are not the exe.

## 0. What is already built (no new work)

- **Spells learned with their Re:CoM magic card, -ra / -ga at bosses** (the ask's first sentence):
  M7 stage 7.11, `docs/recom-m7-unlocks.md` (wrapper on `0x14015C7F0`, two bosses per spell, "+1 per
  boss"). Nothing new is needed unless the owner wants the table changed; KH1's story events that
  Re:CoM lacks are already mapped onto Re:CoM bosses there.
- **Combo Plus ×3 / Air Combo Plus ×2** at KH1's Rod levels (87 / 93 / 99, 24 / 45), arriving
  unequipped (`abilities::ALL`, M9 9b). Missing for M12: the notice.
- **Summons**: Re:CoM's LV3 summon sleights started without a card (`docs/recom-summons.md`).
- **KH1 twins of attack sleights**: Sliding Dash, Blitz, Stun Impact, Zantetsuken (KH1 combo
  modifiers, 9a), Sonic Blade (KH1 special, 9c.1); Strike Raid, Ars Arcanum, Ragnarok keep their
  unlock state (9c, not built).

## 1. Re:CoM's sleights

### 1.1 Where they live. Checked (decomp + exe + files) unless tagged

| what | where |
|---|---|
| **Learned state**: 2 bits per sleight id at `*0x14087CC08 = save + 0x2840` (set by `0x14023AA30`). 0 = not learned, 1 = learned (new), 2 = learned and seen. Sora's ids 4..0x56 and 0x5A..0x5C; Riku's 0x5F..0x6B and 0x6D. Saved with Re:CoM's save block. | `0x14023AA30` |
| **Learn** `0x14023AA60(id)` (`40 53 48 83 EC 20 0F B7 D9 E8 92 00 F7 FF`): range test by story, state 0 → 1 through `0x14023AF10(id, 1)`. `0x14023AF10` also awards **trophy 0x45** the first time every id 4..0x56 is learned (`0x1402BA6F0`, flag `0x140AF7385 & 4`). | `0x14023AA60`, `0x14023AF10` |
| **Learned?** `0x14023AB00(id)` (state ≠ 0). State = 1: `0x14023ABF0`; 1 → 2 ("seen"): `0x14023AFC0` (callers `0x1400D3320`, `0x1400D34A0`: the camp; which page: Assumption). | |
| **Name**: message `0x80100 + id` (`0x14023ABD0`, ids 4..0x6D). The same names in order are `SY0002.CTD`'s list (name of id k = string k + 1 of its offset table at file `0xB0`; Checked (files)). | `0x14023ABD0` |
| **Who learns** (all callers of `0x14023AA60`): the level-up screen `0x14010EF10` (Sleight choice); the card add's first-pickup step `0x14015C7F0` (magic → -ra / -ga, summon → LV2 / LV3); event syscall `0xAD` sub 7 (`0x140145C50`: learn arg a; with arg b ≠ 0 the sleight screen; skipped while bit (1, 0x33) is set or byte (0, 0x61) = 3); the treasure box `CGimmickTreasureBox` (`0x1401C9520`, vtable `0x14058B5B0`: its reward is a sleight → learn + the sleight screen); the CardInit start `0x14023A250` (the first n of the level-up list); `0x14023AD20` (learn everything: a non-story start mode, `0x14023A040` with character 0); the debug givers `0x1400D1A00` / `0x1400D1FC0`. No other writer of `save + 0x2840` exists. | decomp, all `DAT_14087cc08` uses |
| **Recognition** ("Sora can do this sleight"): `0x1401669F0` (`48 89 4C 24 08 56 48 83 EC 70 83 79 5C 01`), on the stock object (`+0x110` the stocked cards' records: byte 0 kind, 2 value, 3 type byte; `+0x148` count; result `+0x14C`). **Every match is gated by `0x14023AB00(id)`**: an unlearned sleight never forms, whatever the cards. Callers `0x140153E80`, `0x14015E5D0`, `0x1401678E0`. | |
| **Start**: Sora's vtable `+0x130` = `0x1402247A0` (case list = every Sora id 4..0x59: the use counter `0x14020E590` (int var (0, id + 0x44), an LV3 counted under its LV2), `initStockSkill` (vtable `+0x138`), `+0x838` = 10). Re:CoM calls it from the request bits `0x1401F8150` when the played record's `+0x834` < 1. | `0x1402247A0`, `0x1401F8150` |
| **`initStockSkill`** `0x140220DF0`: per id the attack row `+0x638` (table `0x1407C6C70`, 16-byte rows), the resource group (`0x140211790(+0x3B8, group)`) and the AP mode: 1 = `StockSkillList[id][row +0xA]` × attack stat `+0x43C` ÷ 100, 2 = power ÷ 10, 3 = power as is, 0 = none (summons, friends, Warp, Warpinator, Synchro, Lucky Bounty: their actors compute). Attribute `+0xA`, status `+0xB` of the list row. | |
| **`StockSkillList`** (`SY0002.BIN` directory entry 2, file offset `0xA60`, 150 × 16 bytes): u16 power columns 0..3, `+8` unknown (0 / 1 / 2), **`+9` category: 1 attack, 2 magic, 3 summon, 5 friend, 0 ability** (exeStockSkill `0x140218950` sub-state 1 branches on 3 and 5), `+0xA` attribute, `+0xB` status nibbles. Checked (files + decomp). | `*0x14087A810` |

### 1.2 The table (Sora's 83 sleights, ids 4..0x56)

Recipes are `0x1401669F0`'s tests, in its card positions (1st, 2nd, 3rd stocked). KB = any attack
(Keyblade) card; "sum" = the three cards' values added. **The code tests positions** (e.g. Raging
Storm only as Aero, Fire, Fire; Firaga Burst only as Fire, Fire, Gravity); the stock is filled in
play order (`0x14016D8A0` appends) and no sort was found. The wiki says the order doesn't matter
(Trinity: "in any order"). Not resolved (Assumption either way). It doesn't matter at Combat = KH1
(no deck).

Unlock: **LvN** = the level-up Sleight choice at Re:CoM level N (`0x14010EF10`'s list; Checked
(decomp)); **card** = the first pickup of the named card (`0x14015C7F0`; Checked (decomp)); **ev**
= event `0xAD 7` in the session's scripts (Checked (files); the boss in brackets: Assumption (wiki));
**wiki:** = KH wiki's source, not in the session's files (a Room of Rewards / Bounty is a treasure
box or reward room, `0x1401C9520` or another world's script: Assumption (wiki)).

Power: `StockSkillList` columns 0..3 (non-zero), mode (1 ×atk / 100, 2 ÷10, 3 as is, 0 none).
Class: **M** = KH1 combo modifier, **S** = KH1 special, **T** = generic magic tier (KH1 spell level),
**Su** = summon level, **F** = friend, **A** = ability candidate (no KH1 twin).

| id | name | cat | recipe (code) | unlock | power (mode) | class / in the mod |
|---|---|---|---|---|---|---|
| 4 | Sliding Dash | 1 | KB ×3, same Keyblade, sum 10–15 | Lv2 | 300 (1) | M, built (9a) |
| 5 | Blitz | 1 | KB ×3, all different, sum 10–15 | Lv17 | 350 / –, 600 (1) | M, built |
| 6 | Stun Impact | 1 | KB ×3, same, sum 20–23 | Lv7 | 100 (1) | M, built |
| 7 | Zantetsuken | 1 | KB ×3, sum 0 or 27 | Lv22 | 100 (1) | M, built |
| 8 | Strike Raid | 1 | KB ×3, sum 24–26 | Lv12 | 200 (1) | S (KH1 id 13), state kept, not built |
| 9 | Sonic Blade | 1 | KB ×3, all different, sum 20–23 | Lv27 | 200 / –, 400 (1) | S, built (9c.1) |
| 0xA | Ars Arcanum | 1 | KB ×3, sum 1–6 | Lv42 | 250 / 300 / 500 (1) | S (KH1 12), not built |
| 0xB | Ragnarok | 1 | KB ×3, sum 7–9 | Lv52 | 400 (1) | S (KH1 14), not built |
| 0xC | Trinity Limit | 1 | Donald, Goofy, KB; Donald/Goofy 1st and 3rd with KB 2nd; KB 1st then Donald/Goofy (not Goofy, Donald, KB) | not found (wiki: not read) | 2500 (1) | S (KH1 15, needs the party) |
| 0xD–0x13 | Fira, Blizzara, Thundara, Cura, Gravira, Stopra, Aerora | 2 | the spell ×2 as cards 1–2 or 2–3 (+ any 3rd) | card (the spell's) | 1000, 700, 600, 1500 (2), 40 (3), 15000 (3), 600 | T, built (M7) |
| 0x14–0x1A | Firaga … Aeroga | 2 | the spell ×3 | card | 1500, 1100, 900, 3000 (2), 70 (3), 21000 (3), 900 | T, built |
| 0x1B | Fire Raid | 2 | Fire, KB, KB | wiki: Monstro Bounty | 500 (1) | A |
| 0x1C | Blizzard Raid | 2 | Blizzard, KB, KB | wiki: Olympus Bounty | 500 (1) | A |
| 0x1D | Thunder Raid | 2 | Thunder, KB, KB | wiki: Neverland Room of Rewards | 1000 (1) | A |
| 0x1E | Reflect Raid | 2 | Cloud, KB, KB | wiki: Hollow Bastion Bounty | 700 (1) | A |
| 0x1F | Judgment | 2 | Aero, KB, KB | wiki: Destiny Islands Bounty | 250 (1) | A |
| 0x20 | Firaga Burst | 2 | Fire, Fire, Gravity | wiki: 100 AW Balloon Glider | 650 (1) | A |
| 0x21 | Raging Storm | 2 | Aero, Fire, Fire | wiki: Castle Oblivion Bounty | 800 (1) | A |
| 0x22 | Mega Flare | 2 | Mushu, Fire, Fire | Lv57 | 2000 (1) | A |
| 0x23 | Freeze | 2 | Blizzard, Blizzard, Stop | ev `WORLD00` 2020, screen (Vexen 10F) | 450 / 1000 (1) | A |
| 0x24 | Homing Blizzara | 2 | Aero, Blizzard, any magic | wiki: Atlantica Bounty | 800 (1) | A |
| 0x25 | Aqua Splash | 2 | Blizzard, Fire, Aero | wiki: Monstro Room of Rewards | 500 (1) | A |
| 0x26 | Magnet Spiral | 2 | Gravity, Gravity, KB | ev `WORLD00` 2016, screen (Riku Replica 8F) | 850 (1) | A |
| 0x27 | Lethal Frame (CTD: "Lethal Flame") | 2 | Stop, KB, KB | Lv32 | 9000 / 350 / 500 (3) | A |
| 0x28 | Shock Impact | 2 | Simba, KB, KB | wiki: Atlantica Bounty | 12000 (3) | A |
| 0x29 | Tornado | 2 | Aero, Gravity, any summon | Lv37 | 1200 (1) | A |
| 0x2A | Quake | 2 | Gravity, Simba, any magic | wiki: Atlantica Room of Rewards | 850 (1) | A |
| 0x2B | Warpinator | 2 | Stop, Gravity, Aero | wiki: Twilight Town Bounty | – (0) | A (utility) |
| 0x2C | Warp | 2 | Stop, Aero, Aero | wiki: Agrabah Room of Rewards | – (0) | A (utility) |
| 0x2D | Bind | 2 | Gravity, Stop, any magic | wiki: Halloween Town Room of Rewards | 10 (1) | A (status) |
| 0x2E | Confuse | 2 | Genie, Tinker Bell, any summon | wiki: 100 AW, find Piglet | 10 (1) | A (status) |
| 0x2F | Terror | 2 | Simba, Mushu, any item; or any summon, any summon, Jack | wiki: entering Halloween Town | 10 (1) | A (status) |
| 0x30 | Synchro | 2 | Cure, Gravity, Aero | wiki: Wonderland Room of Rewards | – (0) | A (utility) |
| 0x31 | Gifted Miracle | 2 | Bambi, Blizzard, any item; or any summon, any magic, Jack | wiki: Halloween Town Bounty | 500 (2: heal 50) | A |
| 0x32 | Teleport | 2 | Stop, Aero, any item; or any magic, any magic, Peter Pan | wiki: Neverland Bounty | 18000 (3) | A (utility) |
| 0x33 | Holy | 2 | Mega-Ether, Megalixir, any item | Lv47 | 800 ×4 cols (1) | A |
| 0x34/0x35 | Proud Roar LV2 / LV3 | 3 | Simba ×2 / ×3 | card (Simba) | 300 / 500 (0) | Su (LV3 built as KH1 Simba) |
| 0x36/0x37 | Splash LV2 / LV3 | 3 | Dumbo ×2 / ×3 | card | 150 / 200 (0) | Su |
| 0x38/0x39 | Paradise LV2 / LV3 | 3 | Bambi ×2 / ×3 | card | 10 (0) | Su |
| 0x3A | **Idyll Romp** | 3 | Bambi, KB, KB | **wiki: 100 AW, Tigger's Jump-a-Thon** (not Bambi's card: `0x14015C7F0` teaches only 0x38 / 0x39) | 10 (0) | A? (summon-type) |
| 0x3B/0x3C | Flare Breath LV2 / LV3 | 3 | Mushu ×2 / ×3 | card | 100 (0) | Su |
| 0x3D/0x3E | Showtime LV2 / LV3 | 3 | Genie ×2 / ×3 | card | 600 / 40 / 24000 (0) | Su |
| 0x3F/0x40 | Twinkle LV2 / LV3 | 3 | Tinker Bell ×2 / ×3 | card | 13 / 22 … (0) | Su |
| 0x41/0x42 | Cross-slash / Omnislash | 3 | Cloud ×2 / ×3 | card | 500 ×3 / 600, 600, 800 (0) | Su (Omnislash built) |
| 0x43 | Cross-slash+ | 3 | Cloud, Stop, KB | wiki: 100 AW Veggie Panic | 500 ×3 (0) | A? (summon-type) |
| 0x44/0x45 | Magic LV2 / LV3 | 5 | Donald ×2 / ×3 | ev `WORLD00` 2001 (no screen) | (0) | F |
| 0x46 | Stardust Blitz | 5 | Donald, Fire (cards 1–2 or 2–3) | wiki: Twilight Town Room of Rewards | (0) | F |
| 0x47/0x48 | Goofy Tornado LV2 / LV3 | 5 | Goofy ×2 / ×3 | ev `WORLD00` 2001 | (0) | F |
| 0x49 | Goofy Smash | 5 | Goofy, KB (cards 1–2 or 2–3) | ev `WORLD00` 2001 | (0) | F |
| 0x4A | Wild Crush | 5 | Goofy, Donald, KB | ev `WORLD00` 2001 | (0) | F |
| 0x4B–0x54 | Sandstorm, Surprise!, Spiral Wave, Hummingbird, Furious Volley LV2 / LV3 | 5 | Aladdin / Jack / Ariel / Peter Pan / Beast ×2 / ×3 | wiki: on meeting the friend | (0) | F |
| 0x55/0x56 | Lucky Bounty LV2 / LV3 | 5 | Pluto ×2 / ×3 | ev `WORLD01` 2003 (Traverse Town) | (0) | F |

Also in Sora's range: 0x5A..0x5C High Jump, Glide, Super Glide (category 0: Re:CoM's ability "sleights";
the mod reads Re:CoM's ability variable instead, M9). 0x57..0x59 have no name and are outside Sora's
learn range (a special stock form, `+0x98 & 0x800`; not Sora's).

**Counts:** 83 = 9 attack (all with a KH1 twin: 4 modifiers + 5 specials incl. Trinity) + 14 generic
magic tiers + 25 other magic-category + 16 summon-category (7 LV3 / Omnislash used by the mod's
Summon list, 7 LV2 / Cross-slash, Idyll Romp, Cross-slash+) + 19 friend. **Ability candidates under
the owner's rule: the 25 magic-category ones, plus Idyll Romp and Cross-slash+ if summon-type sleights
count (question 3).**

### 1.3 "Unlocked by cards alone" does not exist in Re:CoM. Checked (decomp)

Every sleight needs its learned bit (`0x1401669F0` tests `0x14023AB00` on every result). The only
card-driven learns are a magic card's first pickup (its -ra and -ga) and a summon card's (its LV2 and
LV3, Cloud: Cross-slash / Omnislash) — exactly the tiers the owner excludes. **Idyll Romp is a reward
(wiki: Tigger's Jump-a-Thon)**, not cards. So the ask's last sentence needs an owner reading (question
1).

## 2. Re:CoM's level-up (CLevelUp)

Checked (decomp + exe) unless tagged.

| what | where |
|---|---|
| Class `CLevelUp` (vtable `0x140512C10`, ctor `0x14010E9C0(this, parent, sleight)`, 0x2C0 bytes, layout `levelup.lad` / `levelup:00`); `+0x230` character (0 Sora, 1 Riku: `0x1401AAAE0`); **`+0x258` = sleight**: 0 = a level-up, else the **"sleight learned" screen** for that id (event `0xAD 7`, the treasure box; the level-up itself comes from `0x1401AE340` with 0). | `0x14010E9C0` |
| Update `0x14010EF10` (already wrapped by the mod: `win::mp::level_up`, `sites.rs` "level-up screen", bytes `48 8B C4 55 41 56 41 57 48 8D 68 A1`). Case 1: level + 1 (`stats + 0x1C`, trophy at 99), then the offers: **HP** `+0x238` = 0 while max HP < 560; **CP** `+0x23C` = 0 while CP (`stats + 0xC`) < 1625; **Sleight** `+0x240` = the index of the **first unlearned entry** of the level list whose level ≤ the new level (Riku: HP / AP (< 30, by level) / DP (< 99, once his sleight 0x6D is learned)). The cursor `+0x220` starts on the first offered row (HP, else CP, else Sleight). | `0x14010EF10` case 1 |
| The level list (stack literal, also in `0x14023A250`): 2 → 4, 7 → 6, 12 → 8, 17 → 5, 22 → 7, 27 → 9, 32 → 0x27, 37 → 0x29, 42 → 0xA, 47 → 0x33, 52 → 0xB, 57 → 0x22. | |
| Case 6 (confirm, `0x140110480(this, 7)` < 0): sound 0x54; row 0: max HP + 15, HP = max; **row 1 (Sora): CP + 25 (`stats + 0xC`)**; row 2: `0x14023AA60(sleight)`. Case 8: a Sleight pick opens **`CTechHelp`** (`0x14012BF80`, its help page 300 + n for the sleight). Case 0xB: the next level's screen while EXP still reaches (one screen per level). | |
| **The rows** `0x140110480` case 0: three `D2::C2DSeqCtrl` rows (`levelup.lad` items `0x3B + row` and `0x3E + (row ^ 1 for rows 0, 1)`); a row not offered plays item 7 (greyed); each row's two texts are message ids from **`0x1407C1838 + (character × 3 + row) × 8`** (label, value): Sora `0xA80701 / 02` HP, **`0xA80703 / 04` CP**, `0xA80705 / 06` Sleight; Riku `0xA80701 / 07`, **`0xA80708 / 09` AP**, `0xA8070A / 0B` DP. Help line `0x140110140`: `0x1407C1868 + (character × 3 + row) × 4`: Sora `0xA8070D / 0E / 0F`, Riku `0xA8070D / 11 / 13`; the sleight-learned screen uses `0xA8070F` (ids ≥ 0x5A: `0xA80710` in Sora's story). Cursor `0x140110480` case 7: Up / Down skip unoffered rows, sound 1. | exe: the two tables read with pefile |
| The message texts (`0xA807xx`) are in Re:CoM's message files, **not in this session**: the exact words ("CP", Riku's "AP") are not read. Riku's "AP" is his attack power: Assumption (Re:CoM's design). | |

**For "AP Up" at Combat = KH1 (Sora's story):** the row is Sora's row 1. What has to change, natively:
the row's offer rule (`+0x23C`, Re:CoM's CP cap → KH1's AP cap 100), its two texts and help (the ids
above, or our English through the text items as our camp rows do: `recom-config-menu.md`, "the message
table is bypassed"), and the apply (case 6 row 1: no CP, +2 to an AP-Up count in our ability block).
Icons are per row position, shared by Sora and Riku (Assumption: generic, so "AP Up" can keep row 1's).
Our existing wrapper on `0x14010EF10` is where this hooks; no new site.

**The mod's current AP** (to replace): `crates/kh1-recom/src/abilities.rs` `START_AP` = 3,
`AP_UP_LEVELS` (KH1's 24 bonus-kind-3 levels) and `max_ap(level)`, used by `ap_left`, `can_equip`,
`fit_ap`, the migration, the Abilities page (`abilities::page`, `win/keyblades.rs` lines ~1171 and
~1591). The block's 6 zero bytes `save + 0x26A8` were reserved "for AP Up items": the AP-Up count can
live there (layout bump). Checked (code).

**KH1's starting AP = 3.** Checked (decomp + files): the new game `0x1402852C0` copies btltbl `0x39D8`
byte 2 (= 3) to the character record `+9` (the record pointer is the loop's `− 4`: `+4` level = 1, and
**`+0x38` the level-list cursor = 1**, which also settles M9's "cursor starts at 1" Assumption); the
Rod / Shield Dive choice adds 0. KH1 caps the record's AP at 100 (`0x1402B5660` case 3).

## 3. KH1's level-learned abilities

KH1 `0x1402B51B0`: each level-up grants the Dive list's entry (`entry i` = level i + 2) through
`0x140284E10`, **unequipped** (`id | 0x80`). Lists btltbl Sword `0x3B28`, Shield `0x3B90`, **Rod
`0x3BF8`** (the mod's path, decision 18). AP cost btltbl `0x33D8 + 12 × (id − 1)` byte 0. Checked
(decomp + files). Names by id: Assumption (strong; the Archipelago list, KH1's sysmsg not read).

| ability (KH1 id) | Rod levels | Sword | Shield | AP | in the mod |
|---|---|---|---|---|---|
| Combo Plus (6) | 87, 93, 99 | 21, 66, 93 | 63, 87, 96 | 1 | **built** (3 copies) |
| Air Combo Plus (7) | 24, 45 | 72, 96 | 93, 99 | 1 | **built** (2 copies) |
| MP Haste (23) | **21** | 45 | 72 | 3 | no |
| MP Rage (24) | **42, 75** | 63, 87 | 90 (and an entry for level 100) | 3 | no |
| Second Chance (25) | **90** | 48 | 36 | 5 | no |
| Berserk (26) | **60** | 78 | 9 | 1 | no |
| Leaf Bracer (62) | **39** | 69 | 27 | 5 | no |

What the five do in KH1 (ability → actor block `+0x184` bits, `0x1402A6380`; Checked (decomp) for
the code, meanings tagged):

- **MP Haste** (bit 0): the MP charge per hit gets `+ G+0x95 / 100` (= +1.5) on the factor
  (`0x1402BF330`). The mod's `crate::mp` already has this rule's place ("needs abilities: M9").
- **MP Rage** (bits 1–3, a count): charge from damage taken = HP lost × count (`0x14029BBD0`).
  Two Rod copies stack.
- **Second Chance** (bit 4): KH1's damage `0x1402BFA80`: a hit at least the target's HP leaves 1 HP
  (damage = HP − 1; a result of 0 becomes 1, so at 1 HP the hit still kills). In the mod Sora is hit
  by Re:CoM's enemies through Re:CoM's damage, so this goes into Re:CoM's player damage path.
- **Berserk** (bit 5): the attack record `0x1402B2DC0`: Strength + `G+0xFE` (= 4) while
  `0x140268950(party slot)` is true (Assumption: the HUD's low-HP state). At Damage = Re:CoM the mod's
  swings use Re:CoM's attack stat × card power: what +4 means there is a question.
- **Leaf Bracer** (bit 6): `0x14028F140` for list type 3 cast by the actor on itself →
  `0x140297F30(actor, 0)` (sets the actor flag 0x80; Assumption: the cast can't be interrupted, as KH1's
  help says; that type 3 is Cure: Assumption).

KH1's own notice for these: the LEVEL UP!! box's rows (`0x1402B53B0` builds two strings: the stat
bonus text (`0x140528D18`) and the new ability's name from its record `+4`). Checked (decomp). The mod
draws only the box's header today (`recom-kh1-hud.md` H8).

## 4. Notices Re:CoM already has

| notice | where | fits |
|---|---|---|
| **"Sleight learned" screen**: `CLevelUp` with `+0x258` = sleight (`0x14010E9C0(new 0x2C0, 0x1402AD300(1), id)`), states 0xE..0x11: the sleight's name (message `0x80100 + id`), the help line `0xA8070F`, a timer (`+0x260` from 180.0) or a press, then **`CTechHelp`** (`0x14012BF80`, vtable `CTechHelp`, page 300 + n from its id → page table; how to stock the cards). Used by event `0xAD 7` and the treasure box. | `0x14010E9C0`, `0x14010EF10` cases 0xE–0x11 | the "sleight added" notice, as Re:CoM shows it. CTechHelp's page describes cards (wrong at KH1): skip it at KH1 (case 0x10 waits on `0x14087A4B0`). The ids ≥ 0x5A variant (`0xA80710`) is Re:CoM's own "ability learned" wording (High Jump / Glide / Super Glide). |
| The level-up screen itself (one per level, case 0xB) | `0x14010EF10` | KH1's level abilities could be named on it (its help line or a 4th text), since they arrive at a level-up |
| CCommonGauge popups over Sora (`0x14020C9A0(actor, kind 0..6)`: Recover, Guard, Resist, LEVEL UP!, …) | | fixed sprites, no text: not for names |
| Card / prize pickups (`0x1401F4AB0`), event card-get (`0xAD 5`) | | cards, not abilities |
| `0x14010A0C0(4, card)` on a card's first pickup | | journal / help unlocks (save `+0x28xx` bits; Assumption), not a popup |

## 5. Command menu and Customize

- **Row 3 today** (`crates/kh1-recom/src/command_menu.rs`): Summon (KH1 word 6) when a spell and a
  summon are learned, else the blank dash; confirm or **Right** on row 3 opens the Summon list
  (`run_row3`, `run_summon`; list `Kind::Summon`, placed beside row 3 for one entry, else bottom-anchored
  at 348 − 22 n; `Menu::scene`). Checked (code).
- **Left today** (`Menu::cursor`, pad word `0x80`): in a list → close it (sound 3); **at the root →
  the cursor to Attack (sound 1), KH1's rule** (`0x1402931B0`). Right (`0x100`) opens the row's list.
  No other left / right handling. So a Left toggle Summon ↔ Sleights **replaces KH1's Left-at-root**
  (question 4).
- A Sleights list fits as a fourth list kind beside `Kind::Summon` (same place and panels), filled
  with the equipped sleight abilities; names: Re:CoM's (message `0x80100 + id`) or our words (as
  Cloud's `0x8029`).
- **Customize** (`crates/kh1-recom/src/customize.rs`, page in `win/keyblades.rs`): three slots,
  bytes `save + 0x2681..0x2683` (`mp::off::SHORTCUTS`): 0 = none, 1..7 = spell + 1; **any other byte
  reads as none** (`shortcut::slot_spell`), so a sleight range (e.g. ≥ 0x10) is free and old builds
  ignore it. The picker lists learned spells then "None" (KH1 `0x1403041A0`, at most 8 rows in KH1);
  the L1 list casts through the menu's commit path. KH1's shortcuts hold spells only (Checked
  (decomp), `customize.rs` head).

## 6. Performing a sleight at Combat = KH1 (no deck)

The summons' path (`win::summons`): `player+0x82C` = id, call Sora's vtable `+0x130` (checked to be
`0x1402247A0`), battle room only (`player+0x390`), Re:CoM runs action 0x16 to its end action
`0x1401F6660`. **The same start covers every Sora id 4..0x59** (`0x1402247A0`'s case list). Checked
(decomp).

- **No card is read** on the player side: `initStockSkill` reads only `+0x82C`, `StockSkillList` and the
  attack stat; `exeStockSkill` (`0x140218950`, 3216 decomp lines) has no reference to the card-battle
  globals (`0x14087A808` / `0x14087A818`). It reads the side's active enemy card (`player+0x3E8 →
  +0x1E8`, e.g. 0x5E in the friend branch), empty at KH1 (Stage 29). Checked (decomp).
- **What a deck play adds that we skip**: the stock's card loss (Re:CoM removes the stock's first card
  for the battle) lives in the card battle, not in the player: not located (Assumption: in the stock /
  `CCardBattle` play path). The played record's `+0x826 / +0x827` (→ `player+0x830 / +0x834`) gate the
  start (`+0x834` < 1) and a re-attack (`+0x830`); a direct start leaves them 0. Checked (decomp).
- **Battle only**: the Field exe has no case for action 0x16 (`recom-summons.md`). Checked (exe).
- **Damage**: mode-1 sleights scale with Sora's attack stat `+0x43C` (KH1 frames keep it; Stage 18 /
  Damage = Re:CoM). At Damage = KH1 there is no KH1 number for these (no KH1 twin).
- Per kind: attack sleights with a KH1 twin stay KH1's (M9); magic-category and summon-type sleights run
  as Re:CoM made them (motion, effects, hits, end) — the native route, nothing of ours per frame;
  friend sleights need their friend resources and actor (category 5 branch: tint, motion 0x2E,
  "friend" effect, resource wait) — Re:CoM's own too. **Warp, Warpinator, Synchro, Teleport** act on
  the battle (send enemies away, swap or hide; Assumption (wiki)): their branches were not read; they may
  touch battle-end / EXP rules (to check per sleight before building).
- The use counter `0x14020E590` and Re:CoM's resource preload (`0x140211790` at recognition) behave as
  for the summons (the start loads on demand; a first use may wait a moment for the pack). Checked
  (decomp).

## 7. Open questions for the owner

1. **"Sleights that only need cards"**: in Re:CoM every sleight is learned somewhere (§ 1.3); Idyll
   Romp is a 100 Acre Wood reward. Which rule? (a) a sleight is added when Re:CoM teaches it (level-up,
   event, treasure box, minigame) — the notice comes then; (b) added when Re:CoM has taught it **and**
   Sora has ever owned every card of its recipe (cards still matter); (c) added as soon as Sora has ever
   owned its cards, ignoring Re:CoM's learned bit (changes Re:CoM's progression at KH1).
2. **What using a sleight costs** (not in the ask): (a) MP like the summons (a number per sleight: e.g.
   2 / 3 MP by tier); (b) a cooldown; (c) MP plus once per battle; (d) free (AP is the only cost).
3. **Which sleights become abilities**: the 25 magic-category ones are clear. (a) also Idyll Romp and
   Cross-slash+ (summon-type, no KH1 twin); (b) also the LV2 summon sleights / Cross-slash; (c) only the
   25. And Trinity Limit: a KH1 special (needs Donald and Goofy) — wait for the party, or a sleight
   ability now?
4. **Left on the d-pad**: KH1's Left at the root puts the cursor on Attack. (a) Left anywhere at the
   root toggles Summon ↔ Sleights (KH1's rule dropped); (b) Left toggles only with the cursor on row 3
   (KH1's rule kept on rows 0–2). And does the choice stick (saved) or reset to Summon each battle?
5. **Friend sleights and generic tiers**: friend sleights are excluded from abilities: (a) gone at KH1
   (as now: no party); (b) in the Sleights list without AP once learned. Generic tiers stay KH1's spell
   levels (as built).
6. **The KH1-twin specials in the Sleights list**: Strike Raid / Ars Arcanum / Ragnarok become KH1's
   context-tab specials (9c, as Sonic Blade). (a) only there (KH1); (b) also listed under Sleights.
7. **AP for old saves**: the new rule gives 3 AP to a save with no AP-Up picks (today a level-30 save
   has 13). (a) start everyone at 3 (KH1's rule, abilities over budget unequipped); (b) credit past CP
   picks as AP Ups (Re:CoM's CP − the new-game CP, ÷ 25, read once); (c) credit KH1's AP levels already
   passed once.
8. **AP Up cap**: KH1 caps AP at 100 (`0x1402B5660`); with a pick every level 3 + 2 × 98 = 199. Offer
   AP Up while AP < 100 (KH1), or uncapped? And when HP and AP are both capped and no sleight is due,
   Re:CoM's screen has no row to offer (it does not happen with CP until 1625): Re:CoM's behaviour then
   is not read.
9. **Berserk at Damage = Re:CoM**: KH1's +4 Strength has no Re:CoM meaning (swings use Re:CoM's attack
   stat × card power). (a) +4 to the Re:CoM attack stat's KH1 equivalent; (b) a % bonus; (c) only at
   Damage = KH1.
10. **Notices**: (a) Re:CoM's sleight-learned screen (`CLevelUp` sleight mode) for both sleights and
    abilities, without CTechHelp at KH1; (b) KH1's LEVEL UP!! box rows (KH1's own ability notice) for
    the level abilities, Re:CoM's screen for sleights; (c) a line on Re:CoM's level-up screen.
11. **Proposed AP costs** (from `StockSkillList` power, KH1's costs Sonic Blade / Strike Raid 3, Ars
    Arcanum / Ragnarok 4 as the scale; hit counts not read, so this is a proposal): 2 AP — Judgment,
    Bind, Confuse, Terror, Warp, Warpinator, Synchro, Teleport, Gifted Miracle, Idyll Romp; 3 AP — Fire
    Raid, Blizzard Raid, Aqua Splash, Freeze, Firaga Burst, Reflect Raid, Homing Blizzara, Lethal Frame,
    Shock Impact; 4 AP — Thunder Raid, Raging Storm, Magnet Spiral, Quake, Tornado, Cross-slash+; 5 AP —
    Holy, Mega Flare.

## Found on the way (not asked)

- Re:CoM's **trophy 0x45** for learning every sleight (`0x14023AF10`): setting learned bits ourselves
  would affect it; reading them doesn't.
- Event `0xAD 7` is skipped when byte (0, 0x61) = 3 (Assumption: a difficulty / mode; the same byte
  stops Superglide's reward, M9).
- KH1 has partner abilities named like Re:CoM friend sleights (Sandstorm 38, Hummingbird 44, Spiral
  Wave 49; Assumption (names)): relevant for the party milestone, not M12.

## 1:1 checks made / not made

- Checked (decomp + exe + files): Re:CoM's learned state, learners, recognition, start, the level-up
  rows, tables and apply, the sleight-learned screen; `StockSkillList` and the CTD names; the session's
  event-script grants; KH1's learn lists, AP costs, starting AP, the five abilities' code sites.
- Not checked: the Re:CoM message texts (`0xA807xx`, not in the session); other worlds' scripts and
  treasure-box data (sources tagged wiki); the stock's card loss; each special sleight's branch
  (Warp …); anything in game.
