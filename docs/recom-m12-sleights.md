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
- **Customize** (`crates/kh1-recom/src/customize.rs`, page in `win/keyblades.rs`; sleights since step 6, § 8.6): three slots,
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

## 8. Built (M12 steps 1 and 2, owner's go-ahead 2026-11-03)

### 8.1 AP Up replaces CP Up at Combat = KH1 (step 1)

Owner: "On level up, instead of having the choice to level up CP, when the combat is set to KH1, it
should be AP up, and it should give you 2 AP each level. this should be the only way to obtain AP,
aside from the AP you start the game with"; answers: no cap, old saves' CP Ups count as AP Ups.

| what | where | tag |
|---|---|---|
| **AP** = KH1's start 3 + 2 × AP Up picks, nothing else, no cap (`abilities::max_ap`, `State::max_ap`). KH1's level grants (`AP_UP_LEVELS`, the 24 kind-3 levels) are gone. | `crates/kh1-recom/src/abilities.rs` | Checked (decomp + files): KH1's start 3 (`0x1402852C0`, btltbl `0x39D8` byte 2) |
| **The count**: `save + 0x26A8` (u8, our ability block's first reserved byte; layout v5). Nothing read or wrote it before (it was the block's zero reserve; the block sits in Re:CoM's spare tail `0x246B..0x2817`, which no Re:CoM code touches, `recom-keyblade-equip.md`). | `abilities::field::AP_UPS` | Checked (code) |
| **Offer** (state 0 / 1 → 2, the init block): Re:CoM offers CP while CP < 1625 (`cmp [stats+0xC],0x659` at `0x14010F210`); at KH1 row 1 (`+0x23C`) is always offered; the cursor goes on it when HP isn't offered (Re:CoM's first-offered-row rule). | `win::level_up`, in the existing level-up wrapper (`0x14010EF10`) | Checked (decomp + exe) |
| **Words**: row 1's label `+0xD8` "AP Up", value `+0xE0` "Max AP +2"; the status panel's row 1 label `+0x180` "Max AP" (Re:CoM's "Max CP" there) and value `+0x188`: the max AP as Re:CoM prints CP (`"%4d"`; highlighted, `"%4d \xF9Y\x81\xA8\xF9R%4d"` now → +2, the exe's format `0x140512C70`), re-set after Re:CoM re-prints it (states 3, 4, 6); the result line `+0x1E0` "Max AP increased by 2." Through Re:CoM's text copy `0x140258DE0` (its items keep a borrowed pointer until then: ctor `0x140258BC0`). | `crate::level_up` | Checked (decomp + exe) for the items; **Assumption**: our words fit Re:CoM's (its `0xA807xx` messages aren't in the session), and row 1's icon (levelup.lad items `0x3C` / `0x3E`) isn't CP-specific art |
| **Pick** (6 → 7, cursor on row 1): +1 AP Up (`win::abilities::add_ap_up`, which syncs first so an older block is migrated from the CP before the pick). Re:CoM's own +25 CP (`0x14010F6AB`) is **kept, unseen** (owner, 2026-11-03, step 1/2 follow-ups: "AP Up also gives Re:CoM's +25 CP unseen (CP keeps growing for a switch back)"; 0.98.0 undid it) while CP < 1625 before the pick, the cap at which Re:CoM itself greys its CP row (`0x14010F209`); at 1625 and above it is undone (`level_up::keeps_recom_cp`), so Sora's CP never passes what Re:CoM alone can reach (275 + 25 × 54 = 1625). | `win::level_up`, `win::stats::set_sora_cp` | Checked (decomp + exe) |
| **Old saves** (block < v5): AP Ups = (CP − 275) ÷ 25, at most level − 1, read once at the block's migration, logged (`abilities: block vN -> v5 …: Re:CoM's CP C: K past CP Up pick(s) counted as AP Up(s)`). **275** = Sora's new-game CP: the new game `0x1401AD2A0` → `0x14023A040(1, 0, 0)` → CardInit record `index[0]` via `0x14023A250` (`SY0001.BIN` entry `CardInit.BIN`: level 1, HP 80, CP 275). Sora's CP has no other writer than the pick and the unreferenced debug givers `0x1400D1A00` (exe scan of every `[*0x14087CBF8 + 0xC]` access), so the base CP is 275 at every level. | `abilities::ap_ups_from_cp` | Checked (files + decomp + exe) |
| **Over the AP after the migration**: unequipped from the most expensive down (ties: the later in `ALL`), the existing `fit_ap`. KH1 has no rule for this (its AP never drops). | `abilities::migrate` | Checked (decomp: KH1 only adds AP) |
| **Re:CoM setting / Riku**: the screen is Re:CoM's (no change outside Combat = KH1 in Sora's story). | `super::mp::active()` | Checked (code) |

**Who else reads CP** (exe scan, Checked (exe + decomp)): the camp's deck screen (`0x1400E13C0`),
the deck check `0x140159620` (deck cost ≤ CP, from the camp `0x1400D9C80`), the camp status page
(`0x1400E6DF0`), the level-up panel (`0x140110C60`). No event script command, Moogle shop or
battle code reads it. At KH1 there is no deck in battle (decision 29), so the CP changes nothing
there. Since the step 1/2 follow-ups the CP grows with each AP Up as with Re:CoM's CP Up (up to
Re:CoM's own 1625), so **switching back to Re:CoM combat gives the deck limit a Re:CoM-only save
would have** with the same picks (the deck check's rule, cost ≤ CP, unchanged).

**Where CP is still seen at Combat = KH1** (Checked (decomp)): the level-up screen shows none (its
row 1 and its panel's row 1 carry our AP words, set again whenever Re:CoM re-prints the panel:
states 3, 4, 6; the panel is not re-printed after the pick, `0x140110C60` modes 3 / 4 only play
its close). The camp's **Deck** screen still prints "deck cost / CP" (`0x1400E13C0`: the cost total
`+0x21B8` against `[stats + 0xC]`, red when over) and the camp status page reads it: the camp's
Deck entry is Re:CoM's own and stays at KH1 (the deck is kept for a switch back). Hiding it belongs
to the open question "the deck at KH1 setting" (owner question in the step-4 report).

Owner test lines (step 1):
- Combat = KH1, Sora's story, level up: the middle choice reads "AP Up" / "Max AP +2", the panel's
  second row "Max AP" with the AP; pick it: "Max AP increased by 2."; the camp's Abilities page shows
  2 more AP; the camp's Deck screen shows CP + 25 (step 1/2 follow-ups). Screenshot the screen (our words beside
  Re:CoM's, the row's icon).
- A save with CP ≥ 1625: AP Up is still offered.
- An older save: the log line `abilities: block v4 -> v5 … past CP Up pick(s) counted as AP Up(s)`;
  its AP = 3 + 2 × those picks.
- Combat = Re:CoM (and Riku's story): the CP choice as before (+25 CP).

### 8.2 "Sora learned X" on Re:CoM's sleight-learned screen (step 2)

Owner: notices on "Re:CoM's sleight-learned screen (`CLevelUp` sleight mode, its "ability"
wording), without its card-stock help page at KH1".

| what | where | tag |
|---|---|---|
| **Queue**: `win::notices::notify_learned(Learned)` (`Learned::Ability(ix)` / `Learned::Sleight(id)` for later steps); once each while waiting, 32 at most. | `crate::notices`, `win::notices` | Checked (code) |
| **What gets one now**: every ability a sync newly unlocks from a **level** (`Source::Level`: Combo Plus ×3, Air Combo Plus ×2, and the KH1 Rod-list / owner-level modifiers Ripple Drive 12, Hurricane Blast 18, Gravity Break 30, Aerial Sweep 36, Combo Master 50, Counterattack 57, Vortex 84). Not on a save's very first sync (block never written: everything would be "new"). Abilities from Re:CoM's own sleights / movement abilities already come with Re:CoM's screens. | `notices::level_abilities`, `win::abilities::sync` | Checked (code) |
| **The screen**: Re:CoM's `CLevelUp` sleight-learned mode (`+0x258` = id, states 0xE..0x11): an ability opens it with id **0x5A** (Re:CoM's High Jump: from 0x5A the line takes Re:CoM's ability wording `0xA80710` in Sora's story, `cmp [rbx+0x258],0x5A` at `0x1401102DD`; the id only picks the wording) and `+0x250` (the `%s` of the line, Re:CoM's sprintf `0x14004FAA0` at `0x14011032C`) pointed at our name (the Abilities page's, e.g. "Combo Plus") when it leaves state 1 for 0xE. | `win::notices` | Checked (decomp + exe); **Assumption**: `0xA80710`'s text has one `%s` for the name (the texts aren't in the session) |
| **When, after a level-up**: the level-up wrapper syncs the abilities when the last level's screen ends (0xB → 0xD), then turns the same task to sleight-learned mode for the next notice (state 1 with `+0x258` set; Re:CoM's case 1 re-inits and enters 0xE; its resources are still loaded). Also chained after any of Re:CoM's own sleight-learned screens (0x11 → 0xD). The level-up's game step (`0x1401AE340`) waits for the task as for Re:CoM's screen. | `win::level_up`, `win::notices::after_step` | Checked (decomp) |
| **Otherwise** (e.g. levels gained at Combat = Re:CoM): at the next KH1 frame of Sora in the field where Re:CoM's own camp gate holds (`0x1401A96C0` without its Start press: game state `*0x14087B0B0`, `+0x18 & 0x8008` and `+0x5A0` bit 11 clear, not flags (1, 0x33) and (1, 0x34), `0x1401AAA90`: `+0x5A4` (any `CLevelUp` clears it while up), flag (1, 6) clear, `0x140234570`, `+0x5A0 & 0x80201C01`; `+0x580` = 9): `CLevelUp(new 0x2C0, 0x1402AD300(1), 0x5A)` as event `0xAD 7` makes it. Per frame: one atomic load when nothing waits. | `win::notices::frame` (from the request-bits wrapper) | Checked (decomp + exe); **Assumption**: the field (enemies) holds still under the screen as it does for Re:CoM's event and chest screens (those come while an event / chest holds Sora) |
| **No `CTechHelp`** at Combat = KH1: when a sleight-learned screen goes 0xF → 0x10 it has just made `CTechHelp` (`0x14012BF80`), the last child of its task list (`+0x48` tail node, `+8` task; vtable `0x14051B4A8` checked): set to state 5 and page 0, so Re:CoM's update ends it on its first run (default branch) with nothing loaded or unloaded, and its dtor sets `0x14087A4B0`, which state 0x10 waits for. Applies to Re:CoM's own sleight-learned screens at KH1 too (event / chest sleights). **Also after the level-up's Sleight pick** (owner, 2026-11-03, follow-ups: "skip the card help page after the level-up Sleight pick too"): case 8, once the result line has played, makes `CTechHelp(this, id)` for a Sora Sleight pick (`+0x230` = 0, `+0x220` = 2) and goes 8 → 9, which waits for `0x14087A4B0` (its ctor clears it), then 10: the same skip at 8 → 9. | `win::notices::skip_tech_help` | Checked (decomp + exe); Assumption: a child task first runs after its parent's update returns |

Owner test lines (step 2):
- Combat = KH1: reach level 24 (Air Combo Plus) or 87 / 93 / 99 (Combo Plus): after the level-up
  screen, Re:CoM's "learned" box names the ability with Re:CoM's ability wording, then closes on its
  own or on a press, with **no card help page**; the Abilities page lists it (unequipped). Log:
  `notices: queued ability Air Combo Plus`, `notices: … on Re:CoM's sleight-learned screen`.
- Level up at Combat = Re:CoM past such a level, switch to KH1, walk in the field: the box comes when
  the camp could open; tell us if enemies keep moving or can hit Sora under it.
- A Re:CoM sleight from an event or chest at KH1 (e.g. Freeze, Magnet Spiral): its box as before,
  but no card help page after it. At Combat = Re:CoM the help page stays.

Owner test lines (step 1/2 follow-ups):
- Combat = KH1, level up and pick AP Up: still "AP Up" / "Max AP +2", AP + 2 on the Abilities page;
  the log says `Re:CoM's +25 CP kept, unseen: CP a -> a + 25`. Switch Combat to Re:CoM: the camp's
  Deck screen shows the higher CP (as if each AP Up had been a CP Up).
- Combat = KH1, a level-up whose Sleight row is offered (e.g. level 32, Lethal Frame): pick it: no
  card help page after the result line; the log says `level-up (its Sleight pick) …: CTechHelp …
  ended before its page`. At Combat = Re:CoM the help page stays.

Not checked (8.1 / 8.2): anything in game; Re:CoM's message texts; the row icon.

### 8.4 The sleight abilities (step 4)

Owner ("M12 answers", 2026-11-03): "the 25 magic sleights plus Idyll Romp and Cross-slash+" become
abilities with the owner's AP costs; "A sleight is **added when Re:CoM teaches it** (its learned bit
via `0x14023AA60`: level-up, event, chest, first card); the notice fires then". This step adds the
abilities, the save layout, the page and the notices; performing them comes with the Sleights menu
(next step). Combat = Re:CoM and Riku's story: unchanged (nothing here runs there but the
"shown by Re:CoM" mark below, a read).

| ix | id | name | AP | MP (owner) | Re:CoM teaches it (§ 1.2) |
|---|---|---|---|---|---|
| 31 | 0x1B | Fire Raid | 3 | 2 | wiki: Monstro Bounty |
| 32 | 0x1C | Blizzard Raid | 3 | 2 | wiki: Olympus Bounty |
| 33 | 0x1D | Thunder Raid | 4 | 2 | wiki: Neverland Room of Rewards |
| 34 | 0x1E | Reflect Raid | 3 | 2 | wiki: Hollow Bastion Bounty |
| 35 | 0x1F | Judgment | 2 | 1 | wiki: Destiny Islands Bounty |
| 36 | 0x20 | Firaga Burst | 3 | 2 | wiki: 100 AW |
| 37 | 0x21 | Raging Storm | 4 | 2 | wiki: Castle Oblivion Bounty |
| 38 | 0x22 | Mega Flare | 5 | 3 | level-up Lv57 |
| 39 | 0x23 | Freeze | 3 | 2 | event `WORLD00` 2020, screen |
| 40 | 0x24 | Homing Blizzara | 3 | 2 | wiki: Atlantica Bounty |
| 41 | 0x25 | Aqua Splash | 3 | 2 | wiki: Monstro Room of Rewards |
| 42 | 0x26 | Magnet Spiral | 4 | 2 | event `WORLD00` 2016, screen |
| 43 | 0x27 | Lethal Frame | 3 | 2 | level-up Lv32 |
| 44 | 0x28 | Shock Impact | 3 | 2 | wiki: Atlantica Bounty |
| 45 | 0x29 | Tornado | 4 | 2 | level-up Lv37 |
| 46 | 0x2A | Quake | 4 | 2 | wiki: Atlantica Room of Rewards |
| 47 | 0x2B | Warpinator | 2 | 1 | wiki: Twilight Town Bounty |
| 48 | 0x2C | Warp | 2 | 1 | wiki: Agrabah Room of Rewards |
| 49 | 0x2D | Bind | 2 | 1 | wiki: Halloween Town Room of Rewards |
| 50 | 0x2E | Confuse | 2 | 1 | wiki: 100 AW |
| 51 | 0x2F | Terror | 2 | 1 | wiki: entering Halloween Town |
| 52 | 0x30 | Synchro | 2 | 1 | wiki: Wonderland Room of Rewards |
| 53 | 0x31 | Gifted Miracle | 2 | 1 | wiki: Halloween Town Bounty |
| 54 | 0x32 | Teleport | 2 | 1 | wiki: Neverland Bounty |
| 55 | 0x33 | Holy | 5 | 3 | level-up Lv47 |
| 56 | 0x3A | Idyll Romp | 2 | all | wiki: 100 AW |
| 57 | 0x43 | Cross-slash+ | 4 | all | wiki: 100 AW |

Ids and names: `SY0002.CTD`'s list from id 4 (Checked (files); test `sleight_names_match_recoms_ctd`
with `RECOM_SY0002_CTD`). **One difference**: the CTD writes 0x27 "Lethal Flame"; the owner (and the
page) "Lethal Frame". Every other name matches the owner's list.

| what | where | tag |
|---|---|---|
| **The abilities** (`abilities::ALL[31..58]`, `SLEIGHTS`, `Source::Sleight(id)`): no KH1 id, bit or row (nothing in KH1's frame: `to_core` / `kh1_bits` unchanged); sort key 1000 + id, so Sora's page lists them after every KH1 ability, in Re:CoM's id order; help line: what it does and its MP (§ 8.4.1; was "A sleight from Re:CoM. Equip it to use it. Uses n MP." until the owner's "Sleight ability texts"). | `crate::abilities` | Checked (code) |
| **Learning**: the sync (Attack presses, the Abilities page, a level-up's end) asks Re:CoM's own `0x14023AB00(id)` for each one not yet unlocked; a learned one is unlocked **unequipped** (KH1's rule), never taken back. We never call the learner `0x14023AA60` or write `save + 0x2840` (trophy 0x45 stays Re:CoM's). Re:CoM's learners: its level-up pick, event `0xAD 7`, the chest `0x1401C9520`, the card's first pickup (none of these 27). | `abilities::sync`, `win::abilities::sync` | Checked (decomp + code) |
| **Equipping**: the existing AP rules (KH1's `0x140310650` / `0x140311C20`): refused / greyed when the cost is over the AP left. | `abilities::page` | Checked (code + tests) |
| **For the Sleights menu**: `win::abilities::equipped_sleights() -> Vec<u16>` (Re:CoM's ids ascending), `abilities::sleight_mp` (the owner's MP: AP 2 → 1, 3 / 4 → 2, 5 → 3; Idyll Romp, Cross-slash+ all). Nothing reads them yet. | | |
| **Save, layout v6**: the words widen to 64 bits; bits 0..32 stay in the block (`0x26A0` / `0x26A4`), bits 32..64 in an 8-byte extension `save + 0x26B7 ..= 0x26BE` right after the summon block (`0x26AE..=0x26B6`; u32 unlocked high, u32 equipped high), read only from a v6 block. Inside Re:CoM's spare tail (no Re:CoM code reads or writes it); the camp's spare-range report counts it as ours. | `abilities::EXT_FIRST`, `raw` / `read` / `write` | Checked (code) |
| **Migration v5 → v6** (once, at the first sync or KH1 frame): every v5 field kept (the AP Up count too); the extension starts empty; the sleights Re:CoM already taught arrive unequipped **without notices** (they were learned before, with Re:CoM's screens). An older build that writes a v6 block keeps the extension but clears bit 31 (Fire Raid): the next sync of this build finds it again (with a notice). | `abilities::migrate` | Checked (tests) |
| **Notices**: each newly unlocked sleight ability gets `notify_learned(Learned::Sleight(id))`: Re:CoM's own sleight-learned screen with its own name and wording (no card help page at KH1, step 2). None on a save's first sync, none from a block older than v6. | `notices::sleight_abilities` | Checked (code + tests) |
| **No second notice when Re:CoM just showed it** (at any Combat setting, in the existing level-up wrapper `0x14010EF10`, no new hook): (1) a Sora level-up's Sleight pick: case 6's confirm on row 2 (`+0x220` = 2, `+0x230` = 0, `+0x258` = 0) learns `0x14023AA60(list[+0x240].id)` (the stack list, `level_up::SLEIGHT_LIST`): marked at 6 → 7; (2) a sleight-learned screen (event / chest) entering 0xE from 1 / 0 with `+0x258` = id (not our own notice). The id is marked for the session; a notice for it still waiting is taken out of the queue. | `win::notices::shown_by_recom` | Checked (decomp `0x14010EF10` cases 1, 6, 0xE..0x11) |

Per frame: the KH1 frame's equipped read is now 15 + 8 bytes (the extension); nothing else.

Assumptions / known:
- The "shown" mark lasts the session: a sleight Re:CoM taught with its screen in an **earlier session**
  (e.g. at Combat = Re:CoM) that our sync first sees now gets a notice.
- A sleight Re:CoM teaches without a screen (event `0xAD 7` with its second argument 0) gets its notice
  at the next sync (Attack press, the Abilities page, a level-up's end), then at Re:CoM's camp gate.
- Unlock sources tagged "wiki" come from § 1.2 (not the session's files).

Owner test lines (step 4):
- Combat = KH1, a save from 0.98.x (layout v5) that already has Freeze / Magnet Spiral / Lethal Frame
  etc.: the log says `abilities: block v5 -> v6 …` and `sleight abilities Re:CoM taught …: [...]
  (block v5: learned before, no notices)`; no notice boxes; the Abilities page (Sora) lists them after
  the KH1 abilities, unequipped, in Re:CoM's order, with their AP costs. Equip one: the AP goes down by
  its cost; one over the AP left is greyed and refused. Save, reload: still equipped.
- Reach level 32 / 37 / 47 / 57 and pick the Sleight row (Lethal Frame, Tornado, Holy, Mega Flare): no
  extra "learned" box after Re:CoM's level-up (the log: `sleight 0x27 shown by Re:CoM (its level-up
  Sleight pick): no second notice`); the ability is on the page.
- A sleight from an event or chest with Re:CoM's box (Freeze, Magnet Spiral, a Bounty): one box only
  (Re:CoM's), no card help page; the ability is on the page.
- Combat = Re:CoM: everything as before (the camp's KH1 tab shows no Abilities row there).
- Nothing performs the sleights yet (the Sleights menu is the next step).

Not checked (8.4): anything in game; which chest / event gives each "wiki" sleight; Re:CoM's message
texts.

#### 8.4.1 The help lines (owner, 2026-11-03, "Sleight ability texts")

Owner: "a short description for each sleight's help line (what it does + its MP cost); the name stays
"Lethal Frame"". Each sleight's line on the Abilities page (`abilities::page::text::SLEIGHT_HELP`):
what it does in Re:CoM, then "Uses n MP." (the owner's tier) or "Uses all MP." (Idyll Romp,
Cross-slash+); at most 66 characters (`menu::HELP_MAX`, the help box's width); English (Re:CoM's own
descriptions are its messages, not in the session). Before: one generic line per MP cost.

What the lines rest on:
- **Element**: `StockSkillList` `+0xA` (Checked (files)): 2 fire, 3 ice, 4 thunder (the same values
  as Fira / Blizzara / Thundara's rows), 6 Cure's (heal); 0 / 1 / 5 are not named in the lines.
- **Hit kind**: the sleight's attack row (`initStockSkill` `0x140220DF0` → table `0x1407C6C70`, row
  byte `+0xB` → `player+0x668`; Checked (decomp + exe)) and what Re:CoM's enemy hit code does with it
  (`0x14017DC50`, then `0x14017D260` for the state; Checked (decomp)).
- **Look** (how it plays: thrown Keyblade, ring, tornado …): **Assumption (wiki / Re:CoM's sleights
  as players know them)**: the sleights' motions and effects were not read.

| id | name | line (before the MP) | data | tag |
|---|---|---|---|---|
| 0x1B | Fire Raid | Hurl the Keyblade over and over, wreathed in fire. | fire, kind 1 (damage) | element Checked; look Assumption |
| 0x1C | Blizzard Raid | Hurl the Keyblade over and over, wrapped in ice. | ice, kind 1 | as above |
| 0x1D | Thunder Raid | Hurl the Keyblade over and over, charged with thunder. | thunder, kind 0x1D (damage path) | as above |
| 0x1E | Reflect Raid | Inside a barrier, bounce from enemy to enemy. | attribute 0, kind 1 | Assumption |
| 0x1F | Judgment | A rapid flurry of Keyblade blows on enemies. | attribute 1, kind 1 | Assumption |
| 0x20 | Firaga Burst | A ring of fire bursts out around Sora. | fire, kind 3 | element Checked; look Assumption |
| 0x21 | Raging Storm | Flames whirl around Sora as he moves. | fire, kind 3 | as above |
| 0x22 | Mega Flare | A huge blast of fire over the area. | fire, kind 3 | as above |
| 0x23 | Freeze | Freezes the enemies around Sora. Ice. Some resist. | ice, kind 0xC: no effect on an enemy whose `"Freeze"` parameter is < 1; else Re:CoM's freeze state `+0x9C0` | Checked (decomp); "around Sora" Assumption |
| 0x24 | Homing Blizzara | Ice shots that home in on enemies. | ice, kind 2 | element Checked; look Assumption |
| 0x25 | Aqua Splash | Water bursts out around Sora. Ice. | ice (attribute 3), kind 2 | element Checked; look Assumption |
| 0x26 | Magnet Spiral | Pulls enemies in and holds them. Some resist. | kind 0x14: only enemies with the `"Magnet"` parameter; a timed state `+0xB80` | Checked (decomp); "pulls in" Assumption |
| 0x27 | Lethal Frame | Stops enemies in time while Sora strikes them. | kind 0x17: Stop's state `+0x940` for the row's value (9000 ticks = 3 s), without Stop's resistance gate | Checked (decomp); "while Sora strikes" Assumption |
| 0x28 | Shock Impact | Leap up and crash down with a shock wave. | kind 0x10: a timed state `+0xA00` (12000 ticks) | look Assumption; the state not identified |
| 0x29 | Tornado | A tornado that catches enemies and hits many times. | attribute 1, kind 0x11: each hit's damage × 0.1 | Checked (decomp) for the small hits; look Assumption |
| 0x2A | Quake | Shakes the ground: hits only enemies standing on it. | kind 0x12: nothing for an enemy off the ground (its mover `+0x120 & 5`) | Checked (decomp) |
| 0x2B | Warpinator | A strike that warps an enemy away half the time. | kind 0xE: only enemies with parameter `+0x30`, then a 1-in-2 roll (`0x14029CDC0` odd: nothing); the warp state `+0xA80` | Checked (decomp); "out of the battle" not read |
| 0x2C | Warp | Warps enemies out of the battle. Some resist. | kind 0xF: only enemies with parameter `+0x30`; the warp state `+0xA80` | Checked (decomp) for the gate; "out of the battle" Assumption (wiki) |
| 0x2D | Bind | Binds enemies on the ground in place. Some resist. | kind 9: only with parameter `+0x34` and on the ground; `0x14017E240` | Checked (decomp); "in place" Assumption |
| 0x2E | Confuse | Confuses enemies. Some resist. | kind 0xA: parameter `+0x38` 0 → a plain flinch, else `0x14017E350` | gate Checked; "confuses" Assumption (name) |
| 0x2F | Terror | Frightens enemies. Some resist. | kind 0xB: only with parameter `+0x3C`; state `+0x980` | gate Checked; "frightens" Assumption (name) |
| 0x30 | Synchro | Gives other enemies the HP of the one it hits. | kind 0x13: every other enemy with the `"Synchro"` parameter takes the hit enemy's HP `+0x274` | Checked (decomp) |
| 0x31 | Gifted Miracle | A shower of light that restores Sora's HP. | attribute 6 (Cure's), AP mode 2 (heal 50) | heal Checked (files); look Assumption |
| 0x32 | Teleport | Stops the enemies it hits for a long time. Some resist. | kind 8 (Re:CoM's Stop kind, gate: the Stop parameter `+0x2C`), 18000 ticks (6 s; the Stop card's 9000) | Checked (decomp + files); Sora's vanishing not described |
| 0x33 | Holy | Orbs of holy light strike enemies all around. | attribute 1, kind 3, four power columns | look Assumption |
| 0x3A | Idyll Romp | Romp with Bambi; enemies hit drop HP prizes. | summon category, AP mode 0 (its actor computes) | Assumption (wiki) |
| 0x43 | Cross-slash+ | Cloud's Cross-slash: three powerful slashes. | summon category, three 500 columns (as Cross-slash 0x41) | three hits Checked (files); look Assumption |

Test: `abilities::tests::sleight_help_lines_say_what_and_the_mp` (each line its own, ending in the
owner's MP; fits the box). No hook, no site; the page shows the line as before.

Owner test lines (8.4.1):
- Combat = KH1, camp KH1 tab > Abilities > Sora: on each learned sleight the help line describes it
  and ends "Uses n MP." (1 / 2 / 3) or "Uses all MP." (Idyll Romp, Cross-slash+); nothing is cut off
  at the box's right edge (tell us any that is). Lethal Frame keeps its name.
- Tell us any line that doesn't match what the sleight does in game (the look is from memory of
  Re:CoM, not read from its files).

### 8.5 The Sleights row and list (step 5)

Owner ("M12 answers", 2026-11-03): "**D-pad Left toggles Summon / Sleights anywhere on the command
menu** (drops KH1's Left → Attack)"; "In the sleights menu I only want ACTIVELY EQUIPPED sleights to
show in the list"; "**Cost: MP by tier**: AP 2 → 1 MP, 3 → 2 MP, 4 → 2 MP, 5 → 3 MP; Idyll Romp /
Cross-slash+ cost all MP like KH1 summons". Combat = KH1 in Sora's story only (the command menu's
own rule); Combat = Re:CoM and Riku's story unchanged.

| what | where | tag |
|---|---|---|
| **Left at the root** (any of the four rows): the 4th row switches Summon ↔ Sleights, KH1's move sound (1), the cursor stays. KH1's own Left-at-root (cursor to Attack, `0x1402931B0`) is dropped (owner). In an open list Left still closes it (KH1's back); the L1 shortcut list has no cursor (no switch). | `crate::command_menu` (`Menu::cursor`) | Checked (code + tests) |
| **Kept per save**: the summon block's last byte `save + 0x26B6` (`summons::field::BOTTOM_ROW`; 0 Summon, 1 Sleights). It was the block's zero reserve: every older save reads Summon, and older builds' summon sync copies the byte unchanged. Written only on the press; without a save (none loaded) the choice lasts the session. | `win::summons::bottom_row` / `set_bottom_row` | Checked (code + tests) |
| **The row in Sleights mode**: our word "Sleights" (902), always shown (also with nothing equipped), brightness by its grey bit as Summon's; **greyed** when no entry is usable, **greyed and locked** (confirm: KH1's error sound) with nothing equipped or while any Re:CoM sleight runs (Sora's action `0x16` in battle: a summon, a sleight, Sonic Blade). The Summon entry isn't there in this mode (the Magic list's "fewer than 3" placement test doesn't count it). | `Menu::build`, `Menu::scene` | Checked (code + tests) |
| **The list**: `win::abilities::equipped_sleights()` (the equipped sleight abilities, Re:CoM's id order), commands `0x80 + id` (ours; KH1's table ends below 0x80), words `0x8100 + id` (the Abilities page's English names, in every language as Cloud's). Opened by confirm or Right on the row (sound 6), placed and drawn as the Summon list (beside row 3 with one entry, else bottom-anchored at 348 − 22 n; KH1's look: the Summon list's panels without its title). Read only while the row says Sleights. | `crate::sleights`, `win::command_menu` | Checked (code + tests) |
| **Greying** (grey = lock, KH1's list rule): the summons' gate (`summons::usable` = KH1's `0x140288F30` without its once-per-battle test): Re:CoM's battle room, KH1's ground state (flag 0x80; mid-air too for Re:CoM's seven air-start sleights since the follow-ups, § 8.5.1), nothing running, and the sleight's MP in whole points (the charge doesn't count): **Points n** needs MP ≥ n; **All** (Idyll Romp, Cross-slash+) needs MP ≥ 1. Confirm on an entry passes KH1's commit gate (free or a combo window, `0x140293480`), else nothing (no sound, the list stays). | `sleights::usable`, `Menu::run_sleight` | Checked (code + tests); which start in the air: Checked (decomp, § 8.5.1) |
| **The start**: Re:CoM's own sleight start, the summons' path (`win::summons::begin`, shared): Sora's vtable `+0x130` checked to be `0x1402247A0`, a live swing's volume ended (`0x140224FB0`), `player+0x82C` = the id, the start called, action `0x16` checked after. `0x1402247A0`'s case list holds every one of the 27 (0x1B..0x33, 0x3A, 0x43): use counter `0x14020E590`, `initStockSkill` `0x140220DF0` (per id: its attack row `+0x638`, resource load `0x140211790`, AP from `StockSkillList` × attack stat), `+0x838` = 10. Re:CoM then runs `exeStockSkill` `0x140218950`: motion, effects, hits, damage, end. No new hook or site: the callees are `sites::SUMMON_CHECKED`'s. | `win::summons::start_sleight` | Checked (decomp + exe: existing checked sites) |
| **MP**: paid at once in whole points after Re:CoM's start took it (KH1's `0x1402A5B60` with cost ≥ 100, as the summons): the tier's points, or all of Sora's whole MP (the charge stays). The sleight's hits give no MP charge (KH1's charge comes from KH1's hits, `mp::on_kh1_hit`; Re:CoM's sleight hits aren't KH1 rows), as the summons'. | `sleights::pay_points` | Checked (code); that KH1 would charge nothing here: **Assumption** (no KH1 twin) |
| **Invincibility**: none of ours (unlike the summons, owner's earlier ask for them only). Re:CoM's `exeStockSkill` sets its own protection bits in `player+0x418` where its sleight has them (e.g. `|= 0x1001` in one sub-state, cleared at its end); the damage-check wrapper's summon rule (`summons::out`) is not armed for these. | `0x140218950` | Checked (decomp) |
| **Damage**: Re:CoM's own: `initStockSkill`'s AP (power × attack stat `+0x43C` ÷ 100 or the row's mode) through Re:CoM's hit path, as Sonic Blade's at Damage = Re:CoM. These 27 have no KH1 twin; at Damage = KH1 scaled by the enemy's KH1 / Re:CoM max HP since the follow-ups (§ 8.5.2). | | Checked (decomp `0x140220DF0`) |
| **Preload**: Re:CoM's sleight resource load `0x140211790(player+0x3B8, id, 0)` when the Sleights list's cursor lands on an entry, or on the list's first entry while the root's cursor rests on the Sleights row (battle only; once per landing), as the summons' preload and as Re:CoM loads a formed sleight (`0x1401669F0`). | `win::summons::preload_sleight` | Checked (decomp); that a first use never starts before its pack is in: **Assumption** (as Sonic Blade's) |
| **No deck needed** (the brief's check): the start and `initStockSkill` read only `+0x82C`, `StockSkillList` and the attack stat (§ 6). Synchro's effect is an enemy reaction, kind 0x13 in `0x14017DC50`: every enemy whose "Synchro" parameter is set takes the hit enemy's HP (`+0x274`); no card read. Warp, Warpinator, Teleport, Bind, Confuse, Terror: their rows' reaction kinds go through the same enemy handler; their branches were not read one by one. | `0x14017DC50` | Checked (decomp) for Synchro; **Assumption** for the others' details |
| **Words for KH1's look**: the asset step writes "Sleights" (902) and the 27 names (`command_menu::SLEIGHT_WORDS`) in KH1's menu font; a DLL test checks both lists match. Re:CoM's look draws them from the built-in text without the asset step. KH1's look with an older `kh1-hud.json`: the rows have no word (logged once). | `kh1-recom-assets` `command_menu` | Checked (tests) |

Per frame: in Summon mode one more byte read (the bottom row) and two loads (a sleight running)
in the menu's build; in Sleights mode also the equipped read (15 + 8 bytes). Nothing outside the
menu.

Re:CoM differences found: Re:CoM has air versions of some sleights (`initStockSkill` tests
`0x1401F3C00`, Sora airborne, for 0x24 Homing Blizzara, 0x2B..0x2F Warpinator, Warp, Bind, Confuse,
Terror, 0x32 Teleport, and the -ra / -ga tiers), so Re:CoM lets those start in the air. Built since
(owner, "Sleights menu follow-ups"): § 8.5.1.

Owner test lines (step 5):
- Combat = KH1, a save with some sleight abilities equipped (camp KH1 tab > Abilities): in the
  field press D-pad Left on any root row: the bottom row reads "Sleights" (the cursor stays);
  Left again: "Summon". Save, reload: the row keeps the last choice. Log:
  `summons: bottom row -> Sleights (kept in the save at save+0x26B6)`.
- In battle on the ground, Sleights row, Right or confirm: the list shows only the equipped ones,
  in Re:CoM's order; one Sora can't pay is greyed and buzzes. Pick each and watch it play as in
  Re:CoM (motion, effects, hits, end), MP going down by its tier: Fire Raid, Blizzard Raid, Thunder
  Raid, Reflect Raid, Judgment, Firaga Burst, Raging Storm, Mega Flare, Freeze, Homing Blizzara,
  Aqua Splash, Magnet Spiral, Lethal Frame, Shock Impact, Tornado, Quake, Warpinator, Warp, Bind,
  Confuse, Terror, Synchro, Gifted Miracle, Teleport, Holy, Idyll Romp and Cross-slash+ (the last
  two take all MP, usable with 1). Log: `summons: sleight <name> -> Re:CoM's sleight 0x.. … n MP`
  and `… Re:CoM's sleight ended …`. Tell us any that freezes Sora, plays without effects on its
  first use, or ends the battle oddly (Warp, Warpinator, Teleport).
- While one plays, the Summon / Sleights row is greyed and locked; Sora can be hit only where
  Re:CoM's own sleight lets him.
- Nothing equipped: "Sleights" greyed; confirm buzzes. In the field: every entry greyed; in the
  air: all but Re:CoM's air-start ones (§ 8.5.1).
- KH1's look (the KH1 HUD) needs this version's asset step for the words; Re:CoM's look shows them
  without it.
- Combat = Re:CoM: no change.

Not checked (8.5): anything in game; each sleight's branch of `exeStockSkill` one by one (Warp,
Warpinator, Teleport and how they end a battle); the resource wait for the magic-category sleights.

#### 8.5.1 Air start (owner, 2026-11-03, "Sleights menu follow-ups")

Owner: "Re:CoM's air-start sleights (Homing Blizzara, Warpinator, Warp, Bind, Confuse, Terror,
Teleport) usable mid-air (to build)"; the rest stay ground-only.

| what | where | tag |
|---|---|---|
| **Re:CoM's rule**: `initStockSkill` `0x140220DF0`'s case for each of 0x24, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x32 (and the generic tiers 0xD..=0x1A, not ours) asks Re:CoM's airborne test `0x1401F3C00` (no floor contact on Sora's mover `+0x120 & 5` and above the floor; cached per frame at `+0x538` / `+0x53C`) and in the air takes the **air version**: `+0x418 |= 0x800000`, `+0x478` = 4 (the air clips; Teleport −36), then at the init's end the low-gravity bit `+0x418 |= 0x2000000` (gravity × 0.16 in `0x1401F64D0`); Re:CoM's fall / land step `0x1401F6380` (from the Battle exe `0x1401F9840`) plays the fall row (6) while he is up and the landing row (7) when he touches down, clearing `0x800000`. No other of the 27 has an air case (their cases set only the attack row and the resources). These seven are exactly the owner's list. | `0x140220DF0`, `0x1401F3C00`, `0x1401F64D0`, `0x1401F6380` | Checked (decomp) |
| **The list's gate**: on the ground as before; **in the air only these seven** (`sleights::AIR_START`, `sleights::usable`), in a KH1 movement state that lets a command start off the ground (magic flag 0x20 without the ground flag 0x80: take-off 1, air 2, flight 8; `summons::Gate::air`). The others stay greyed / locked in the air (KH1's error sound on confirm). The summons stay ground-only. Battle room, nothing running, the MP: unchanged. | `crate::sleights`, `crate::command_menu` (`Menu::build`) | Checked (code + tests) |
| **The start**: unchanged (Re:CoM's own start through Sora's vtable `+0x130`); Re:CoM picks the air version itself. The log line adds "(in the air: Re:CoM's air version)" when `+0x418 & 0x800000` is set after the start. | `win::summons::start_sleight` | Checked (code) |

No new hook, site or per-frame work (one more flag test in the menu's build).

Assumptions: Sora's vertical speed at the start is what the KH1 frame last wrote into Re:CoM's
`+0x5D4` (Re:CoM's own jump would leave its own there); after an air sleight Re:CoM's fall / land step brings Sora
down and the KH1 frame takes him back fresh (as after Re:CoM's reactions in the air).
Not read: what Re:CoM does when a ground-only sleight is played in the air (the owner's rule keeps
those ground-only).

Owner test lines (8.5.1):
- Combat = KH1, in battle, equip Homing Blizzara, Warp, Bind (or any of the seven) and Fire Raid
  (or any other): jump, open Sleights in the air: the seven are usable, the others greyed (confirm
  buzzes). Use each of the seven mid-air: it plays Re:CoM's air version, MP goes down by its tier,
  Sora lands after it; the log says `… through Sora's sleight start (in the air: Re:CoM's air
  version) …`. Tell us if Sora floats, drops oddly, or the KH1 controls don't come back on landing.
- On the ground all equipped sleights work as before; the summons stay ground-only.

#### 8.5.2 Damage = KH1: the enemy's KH1 / Re:CoM max HP (owner, 2026-11-03, "Sleights menu follow-ups")

Owner: "at Damage = KH1 sleight damage scaled by the enemy's KH1 max HP / its Re:CoM max HP (to
build)". At Damage = KH1 the enemies have KH1's HP (the stats init), while a sleight hits for
Re:CoM's AP: so each sleight hit is Re:CoM's × (that enemy's KH1 max HP / its Re:CoM max HP), keeping
the share of the enemy's HP a hit takes as in Re:CoM. Damage = Re:CoM: unchanged.

| what | where | tag |
|---|---|---|
| **Where**: the existing enemy-hit wrapper on Re:CoM's handler `0x14017F7E0`, in its path for hits that are neither a KH1 swing's nor a KH1 spell shot's, just before Re:CoM's handler: on the hurt body Re:CoM's gather `0x140154EB0` takes (the first with a hit; it reads the value `+0x92` through `0x14016FDB0` into the hit Re:CoM then applies), the value is scaled. Re:CoM's handler then applies its resistance %, weak part, frozen bonus and boss falloff to it as to any hit (all multiply, so the ratio carries through). No new hook or site. | `win::summons::scale_sleight_hit`, `win::attack::kh1_enemy_hit` | Checked (decomp `0x14017F7E0`, `0x140154EB0`) |
| **Which hits**: the current Sleights-list sleight's or (since "KH1 damage scaling", below) summon's (from our start until the next one starts or the battle ends): one whose owner (the body's `+0x98`, the attack's `+0x20` id, `0x14016F1C0`) is **Sora or his weapon while that sleight or summon runs** (his action `0x16` from our start), **Sora's summon actor**, or a **Re:CoM shot** (`CBullet` `0x140525430` / `CHomingBullet` `0x140525530`) whose shooter `+0x238` is Sora or his summon (shots still flying after he lands count). Others (an enemy's, e.g. under Confuse) are untouched; an owner we don't count while the sleight plays is logged once per line budget. | `win::summons::owner_of` | Checked (decomp: the owner / shooter fields); that every one of the 27 hits through Sora's collisions or Re:CoM shots: **Assumption** (the log names any other) |
| **Damage kinds only**: Re:CoM's own list of kinds whose value isn't damage stays as is (the handler's mask `0x912100`: Stop ticks 8 (Teleport), Gravity % 0xD, the timed states 0x10 (Shock Impact) / 0x17 (Lethal Frame), Magnet 0x14 (Magnet Spiral)); the durations stay Re:CoM's. | `sleights::is_damage_kind` | Checked (decomp `0x14017F7E0`, `0x14017DC50`) |
| **The numbers**: KH1 max HP = the twin's KH1 HP the stats init set for that enemy (`kh1-stats.json`, Damage = KH1); Re:CoM max HP = Re:CoM's getter `0x14017F5A0` (its `@ENP` row HP × the difficulty %, the original under our max-HP wrapper). Value × KH1 / Re:CoM, rounded, at least 1, at most 32767. | `win::stats::twin_max_hps`, `sleights::kh1_scaled` | Checked (decomp `0x14017F5A0`; code + tests) |
| **No KH1 twin** (Re:CoM-only enemies, bosses): since "KH1 damage scaling" (below) scaled by their **KH1-scale HP** too. Until then Re:CoM's value stood. | `win::stats::kh1_scale_hps` | Checked (code) |

Per frame: one atomic load per enemy-hit call while no Sleights-list sleight was used this battle;
after one, a read of the enemy's hurt bodies per call (as the other hit checks; not measured), and
the owner lookup only when a hit waits.

Owner test lines (8.5.2):
- Damage = KH1 (ini `damage = kh1`), Combat = KH1, a battle with ordinary Heartless (Shadows,
  Soldiers …): use Fire Raid, Homing Blizzara or Holy: each hit takes about the same share of the
  enemy's HP bar as in Re:CoM (not a one-shot). Log: `summons: sleight <name> on CEnemy…: Re:CoM's hit
  a (kind k) -> b (KH1 max HP x / Re:CoM max HP y)`.
- On a boss or a Re:CoM-only enemy: scaled too since "KH1 damage scaling" (§ 8.5.3).
- Teleport / Lethal Frame / Shock Impact / Magnet Spiral: their stop / hold times as in Re:CoM.
- A line `… by an object we don't count as Sora's (vtable …)`: send us the log (that sleight hits
  through something else).
- Damage = Re:CoM: no change.

#### 8.5.3 Bosses, Re:CoM-only enemies and summons (owner, 2026-11-03, "KH1 damage scaling")

Owner: "at Damage = KH1, sleights also scale on enemies without a KH1 twin (bosses, Re:CoM-only
enemies) by their KH1-scale HP / Re:CoM HP; summons get the same rule." Damage = Re:CoM: unchanged.

| what | where | tag |
|---|---|---|
| **(a) The KH1-scale HP of an enemy without a twin**: the HP the mod already cuts it to at Damage = KH1, in the same stats-init wrapper (`0x14017FF50`, `docs/recom-kh1-attack.md` § KH1 numbers for enemies without a twin): a **proxy** (Re:CoM-only enemies: its own Re:CoM `@ENP` row HP × the world's ratio from the twins or the world's boss twin), a **proxy boss**, or a **boss twin** (a boss with a KH1 equivalent: its own KH1 record). Its hits: Re:CoM's value × that HP / Re:CoM's max HP (Re:CoM's getter `0x14017F5A0`, the original under our wrapper: row HP × the difficulty %), the same formula, damage kinds only (Teleport / Lethal Frame / Shock Impact / Magnet Spiral / Gravity values stay Re:CoM's). An enemy whose HP the init didn't set (Riku's story, not an enemy-category object, no proxy record) keeps Re:CoM's value, logged `… has no KH1-scale HP …`. The log names the source: `(KH1 twin)`, `(proxy)`, `(proxy boss)`, `(boss twin)`. | `win::stats::kh1_scale_hps` (was `twin_max_hps`, twins only), `win::summons::scale_sleight_hit` | Checked (code: the init writes `KH1_HP` for twins, proxies and boss twins alike at Damage = KH1) |
| **(b) Summons**: KH1's Summon list (the seven LV3 summon sleights, Cloud's Omnislash) set the scaled sleight at their start, as the Sleights list does. Their hits come from the **summon actor** (`Summon::CSummon*`), not Sora: told by its vtable (`crate::summons::ACTOR_VTABLES`, 7 classes; `sites::SUMMON_ACTOR_PTRS` checks each vtable's slot `+0` and type getter `+0x28` at the summons' install; a mismatch only turns this off, logged) and its player `+0x248` = Sora. Its shots: Re:CoM's bullet manager `0x140158DD0` writes the shooter id only for actor types 0x1000.. / 0x2000.. / 0x4000; the summon's type getter `0x1401A4A30` returns 0x1000, so the shooter is the summon's own id: counted when it resolves to Sora's summon. Sora's own collisions while the summon plays count too. The same covers the summon-type Sleights-list sleights Idyll Romp (Bambi's actor) and Cross-slash+ (Cloud's), whose actor hits were not counted before. | `win::summons::{start, owner_of, is_our_summon}` | Checked (exe: RTTI, vtables, relocated pointers; decomp: `0x14020EE80` makes the actors, `0x1402271D0` sets `+0x248` from `*0x14087B6B8 + 0x60`, `0x1401A4xxx`'s `0x14014D420(holder, actor)` makes the attack holder `+0x100` the summon's melee hits use (`0x140226890`), `0x140158DD0`'s shooter rule) |
| **Same exceptions**: Re:CoM's own non-damage kinds (`sleights::is_damage_kind`, the handler's mask `0x912100`). Which kinds each summon's hits carry: not read one by one (Re:CoM's mask decides at the hit). | | **Assumption** (per summon) |

Assumptions: a summon's shot landing after the summon actor is gone doesn't resolve its shooter
(Re:CoM's damage, logged as another object); `*0x14087B6B8 + 0x60` is Sora's player in Sora's
story (the summon's code uses it as Sora's: `+0x418`, `+0x3D8`, `+0x854`); multi-part bosses: a hit
on a part object whose stats init set no KH1 HP keeps Re:CoM's value (logged).

Per frame: nothing new (the owner lookup and one more vtable compare run only when a hit waits
after a sleight or summon was used this battle).

Owner test lines (8.5.3):
- Damage = KH1 (ini `damage = kh1`), Combat = KH1: on a boss (e.g. Guard Armor, Axel) and a
  Re:CoM-only Heartless, use Fire Raid / Holy: each hit takes about the share of the HP bar it takes
  in Re:CoM. Log: `summons: sleight <name> on CBoss…: Re:CoM's hit a (kind k) -> b (KH1-scale max HP
  x (boss twin | proxy boss | proxy) / Re:CoM max HP y)`.
- Summon Simba / Genie / Cloud on ordinary Heartless and on a boss: the same share as in Re:CoM, not
  a one-shot. Log: `summons: sleight Proud Roar LV3 on …` (or Omnislash …).
- Idyll Romp / Cross-slash+ from the Sleights list: scaled the same way now.
- A line `… by an object we don't count as Sora's (vtable …)`: send us the log.
- Damage = Re:CoM: no change (summons and sleights at Re:CoM's damage).

### 8.6 Sleights as shortcuts (step 6)

Owner ("M12 sleights and progression", 2026-11-03): "I also would like sleights to be able to be
added to the customize menu, for shortcuts." KH1's shortcuts hold spells only (Checked (decomp),
`customize.rs` head: `0x1402857C0` writes a spell or −1); the sleights are ours, on KH1's slots.
Combat = KH1 in Sora's story only (the camp's Customize page and the command menu's own rule);
Combat = Re:CoM and Riku's story unchanged.

| what | where | tag |
|---|---|---|
| **The slot byte**: `save + 0x2681..0x2683` (`mp::off::SHORTCUTS`), as before: 0 none, 1..=7 spell + 1, and now **a sleight ability's Re:CoM id** (0x1B..=0x33, 0x3A, 0x43: the 27, all ≥ 8). Every other byte stays "none". Before this step any byte ≥ 8 read as none (`shortcut::slot_spell`), so an older build shows such a slot empty and keeps the byte unless that slot is set there. No layout change. | `command_menu::shortcut::{slot_sleight, slot_of, byte_of}` | Checked (code + tests) |
| **KH1's fill-on-learn** (`0x140285260`: a learned spell goes into the first empty slot): a sleight slot counts as set, never overwritten; the one-time set-up of a pre-7.12 save keeps it too. | `unlocks::fill_slot`, `unlocks::shortcuts` | Checked (code + tests) |
| **The picker** (Customize, a slot confirmed): KH1's learned spells, then **the equipped sleights** in Re:CoM's order (the Sleights list's `sleights::list`), then None; the blue magic row for both (the page scrolls past 8 rows as the Abilities page does). Names: the command menu's sleight word (`0x8100 + id`), else the Abilities page's English name. Help lines now say "spell or sleight" / "use this shortcut". | `customize::picks`, `win::keyblades` | Checked (code + tests) |
| **The L1 list**: a sleight slot shows its command `0x80 + id` (the Sleights list's word), greyed and locked unless the **Sleights list's own test** passes (`sleights::usable`: Re:CoM's battle room, KH1's ground state or Re:CoM's seven air-start sleights in a KH1 air state that takes a command, nothing running, the owner's MP by tier / all MP with ≥ 1). A sleight slot makes the list available with or without a spell learned (KH1's rule "a slot is set and a spell is learned" for spell slots). Works with the bottom row on Summon or Sleights. | `Menu::build` | Checked (code + tests) |
| **Pressing it**: Triangle / Square / confirm as for a spell slot: greyed → KH1's error sound; KH1's commit gate refusing (Sora busy) → dropped silently; else **the Sleights list's start**: `Outcome::sleight` → `win::summons::start_sleight` (Re:CoM's own sleight start `0x1402247A0` via Sora's vtable `+0x130`, the MP paid after Re:CoM took it, Re:CoM's air version when mid-air, the Damage = KH1 scaling of § 8.5.2). | `Menu::step_shortcut`, `win::command_menu` | Checked (code + tests) |
| **Preload**: when the L1 list comes on top in battle, Re:CoM's sleight resource load `0x140211790(player+0x3B8, id, 0)` for each sleight slot (at most three; it returns at once for a pack already in its 16-entry table), as the Sleights list's cursor preloads its entry. | `win::summons::preload_sleights` | Checked (decomp); a first use never before its pack is in: **Assumption** (as the Sleights list) |
| **Unequipped later** (owner: "follow what the page does for an item you no longer have": the Items page greys rows it can't use): the byte is kept; on the Customize page that slot's row is **greyed**, its value still names the sleight, and its help line reads "Not equipped: equip this sleight on the Abilities page."; it isn't in the picker (no row mark). In the L1 list it shows **the blank dash, greyed** (KH1's look for a slot whose spell isn't learned), and pressing it is the error sound. Equipping it again brings the shortcut back. | `customize::unequipped`, `Page::row_grey` / `help` | Checked (code + tests) |

Per frame: the equipped read (15 + 8 bytes) now also while a shortcut slot holds a sleight; one
byte test per slot. No hook, site or asset change: the sleight words in KH1's font already come
with 0.100.0's asset step (Re:CoM's look needs none).

Owner test lines (8.6):
- Combat = KH1, camp KH1 tab > Customize: confirm a slot: the picker lists the learned spells, then
  the equipped sleights (Re:CoM's order), then None. Pick Fire Raid for Square: the slot shows
  "Fire Raid"; the log says `customize: shortcut 2 = sleight Fire Raid (0x1b) …`.
- In battle on the ground, hold L1: the panel shows Fire Raid; Square plays Re:CoM's Fire Raid, MP
  down by 2 (log: `summons: sleight Fire Raid -> Re:CoM's sleight 0x1b …`). With 1 MP, in the
  field, or while a summon / sleight plays: greyed, Square buzzes.
- A mid-air shortcut: Homing Blizzara (or Warp, Bind …) plays its air version; Fire Raid buzzes.
- Unequip Fire Raid on the Abilities page: Customize shows Square's row greyed with "Not equipped …";
  the L1 panel shows a grey dash and Square buzzes. Equip it again: the shortcut is back.
- Learn a new spell while a slot holds a sleight: the sleight stays (the spell goes to an empty
  slot, if any).
- Combat = Re:CoM: no change.

Not checked (8.6): anything in game.

#### 8.5.4 KH1's specials in the Sleights list (owner, 2026-11-03, "M12 answers": "Strike raid are arcanum and ragnarok in KH1FM are triangle prompts. Let them be triangle prompts and also show up in the sleights menu")

| what | where | tag |
|---|---|---|
| **Listed**: an equipped KH1 special of `sleights::SPECIALS` (M12 step 7: Strike Raid, Ars Arcanum, Ragnarok) joins the equipped sleights, under Re:CoM's sleight id of the same name (Strike Raid 8: command `0x88`; Ars Arcanum 10: `0x8A`; Ragnarok 11: `0x8B`), so Re:CoM's order puts it before the magic sleights. Sonic Blade is not listed (the owner named the other three). | `crate::sleights`, `win::command_menu::world` | Checked (code + tests) |
| **Its name**: KH1's own word of its command (Strike Raid 338 "Strike", Ars Arcanum 331 "Arcanum", Ragnarok 334 "Ragnarok"), each language's from the asset step. | `sleights::word` | Checked (files + tests) |
| **Usable** when its own prompt would offer it (`0x14028D0F0`'s score 2 for that special alone: a live Re:CoM attack target in range, its KH1 MP), in a battle room, on the ground, no special or sleight running; greyed and locked otherwise (KH1's error sound). | `sleights::special_usable`, `win::command_menu::sleight_specials` | Checked (code + tests) |
| **Performed** exactly as the prompt's Triangle: confirm issues its KH1 command (0x5E / 0x57 / 0x5A), taken by Sora's KH1 frame (`crate::specials`); not Re:CoM's sleight start; its MP is KH1's (Strike Raid 2, Ars Arcanum 3, Ragnarok 3), not the tier rule. | `Menu::run_sleight` | Checked (code + tests) |

Owner question: Sonic Blade in the list too?

