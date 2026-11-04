# Command menu pages, the pointing arrow and Party (0.111.0)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Command menu pages and Party), with a KH2 FM
recording:

> "I like our command menu, but I think it should have the pointing arrow that follows your selection
> on the command menu so people know that you can press left on the D Pad to switch between the two
> pages. Also, Page 1 (the default) should be Attack, Magic, Items, Sleights. Page 2 should be Attack,
> Summon, Party, Sleights The "Party" section should have "Magic LV3" stylized as "Donald Magic"
> "Stardust Blitz" "Goofy Tornado LV3" stylized as "Goofy Tornado" "Goofy Smash" "Wild Crush" "Lucky
> Bounty" "Sandstorm LV3" stylized as "Sandstorm" "Surprise! LV3" stylized as "Surprise!" "Spiral Wave
> LV3" Stylized as "Spiral Wave" "Hummingbird LV3" Stylized as "Hummingbird" and "Furious Volley LV3"
> Stylized as "Furious Volley" These friend sleights should ONLY appear if the friend cards drop in
> their respective battles. (You shouldn't be able to use sandstorm in traverse town for example) Also,
> if any of these sleights require an additional unlock requirement, they shouldn't show up until that
> requirement is met. Lastly, friend sleights should be able to be put in the shortcuts."

Answers: "Re:CoM's own arrow"; "Whole battle, if possible"; "Same MP as sleights".

Steam addresses (image base `0x140000000`). Tags: **Checked (decomp)** read in the Re:CoM Steam decomp
this session; **Checked (exe)** bytes / tables read from `recomsteam.exe`; **Checked (files)** the
owner's Re:CoM / KH1 files in the session; **Measured (recording)** the owner's KH2 FM recording;
**Assumption** not confirmed. Nothing here was tried in game.

## 1. Research: Re:CoM's Friend cards and friend sleights

### 1.1 Which Friend cards can drop in which battle

| what | where | tag |
|---|---|---|
| The drop roll `0x1401602A0(out, chance)`; its one caller `0x14017E190` (the hit handler `0x14017FC9D` passes −1 = the row's chance; the battle setup's room-card 0xAF case `0x140180366` / `0x1401803F4` passes 1000). At Combat = KH1 the roll is skipped (M6, `recom-m6-battles.md` § Friend cards). | decomp | Checked (decomp + exe) |
| **Pluto first**, in every world: Sora's story (`0x1401AAAE0` = 0), not a minigame room (`0x140235360`: map room 1200..1255), fewer than 3 Plutos held this battle (`CBTL + 0x1D8`, counted by the revolver add `0x140166300`), a 20 % draw, Pluto met (int var (0, 0x22) bit 0x100), fewer than 10 cards in the revolver (`0x14016A200`): card 0x39. | `0x1401602A0` | Checked (decomp) |
| **Then the table**, while fewer than 5 Friend cards are held this battle (`CBTL + 0x1D4`): row = the world `0x1402339C0(0)` (at most 13); room type `0x140233860()` 0 or 99 (the save block's byte +1, copied from `0x14087B836` by `0x140237CE0`; `0x1402369D0` sets 0 / 99 on the way up / down between floors: Castle Oblivion's halls, Assumption from the code path) → row 13; Riku's story + 13. Chance (row byte 0, ‰; or the caller's), then a % draw over the row's three weights (bytes 1..3) picks one of its three cards (bytes 4..6). Donald / Goofy not met → no drop; Aladdin … Beast not met → the draw is repeated; the King (0x38) only by the sign bit (Riku's). | `0x1401602A0` | Checked (decomp) |
| **The table** `0x1407C1F10`, 26 × 8 bytes (`sites::PARTY_CHECKED`, `crate::party::DROP_TABLE`). | exe | Checked (exe) |
| Friends-met bits (int var (0, 0x22), `0x1401B08C0`): Donald 1, Goofy 2, Aladdin 4, Ariel 8, Jack 0x10, Peter Pan 0x20, Beast 0x40, the King 0x80, Pluto 0x100. Set by the event scripts (the opening sets Donald and Goofy, `recom-story-skips.md`); which events set the others: not read (only `WORLD00` / `WORLD01` scripts are in the session). | decomp | Checked (decomp); the setters: Assumption (wiki: when Sora meets them) |

The table (Sora's rows; "—" none):

| row | world | chance ‰ | cards (weight %) |
|---|---|---|---|
| 0 | none | 0 | — |
| 1 | Traverse Town | 150 | Donald 50, Goofy 50 |
| 2 | Agrabah | 150 | Donald 30, Goofy 20, **Aladdin 50** |
| 3 | Olympus Coliseum | 150 | Donald 60, Goofy 40 |
| 4 | Wonderland | 150 | Donald 40, Goofy 60 |
| 5 | Monstro | 150 | Donald 50, Goofy 50 |
| 6 | Halloween Town | 150 | Donald 20, Goofy 30, **Jack 50** |
| 7 | Atlantica | 150 | Donald 20, Goofy 20, **Ariel 60** |
| 8 | Neverland | 150 | Donald 20, Goofy 30, **Peter Pan 50** |
| 9 | Hollow Bastion | 150 | Donald 30, Goofy 30, **Beast 40** |
| 10 | 100 Acre Wood | 0 | — |
| 11 | Twilight Town | 150 | Donald 50, Goofy 50 |
| 12 | Destiny Islands | 150 | Donald 50, Goofy 50 |
| 13 | Castle Oblivion (and every hall) | 150 | Donald 50, Goofy 50 |
| 14..25 | Riku's worlds 1..12 | 125 | the King 100 (row 23, 100 Acre Wood: none) |

**Pluto** (Lucky Bounty) can drop in every battle of Sora's story once he is met.

### 1.2 The friend sleights

| owner's name | Re:CoM sleight (id) | friend (`0x14020F4D0` → ctor, RTTI) | cards Re:CoM needs | pack (`0x14020F720`) | extra unlock |
|---|---|---|---|---|---|
| Donald Magic | Magic LV3 (0x45) | 1 Donald `0x1401DD5F0` `CFriendDonald` | Donald ×3 | 0x1C (0x2C in Halloween Town) | learned at the opening tutorial's event `WORLD00` 2001 (with LV2) |
| Stardust Blitz | Stardust Blitz (0x46) | 1 Donald | Donald, Fire | 0x1C / 0x2C | its learned bit (wiki: Twilight Town Room of Rewards) |
| Goofy Tornado | Goofy Tornado LV3 (0x48) | 2 Goofy `0x1401DF4C0` `CFriendGoofy` | Goofy ×3 | 0x1D (0x2D) | `WORLD00` 2001 |
| Goofy Smash | Goofy Smash (0x49) | 2 Goofy | Goofy, Keyblade | 0x1D / 0x2D | `WORLD00` 2001 |
| Wild Crush | Wild Crush (0x4A) | 10: Goofy (`+0x78`) and Donald (`+0x70`) | Goofy, Donald, Keyblade | 6 (+ Donald's and Goofy's: `initStockSkill` loads 0x44, 0x47, 0x4A) | `WORLD00` 2001 |
| Lucky Bounty | Lucky Bounty LV3 (0x56) | 9 Pluto `0x1401E2A70` `CFriendPluto` | Pluto ×3 | 0x24 | `WORLD01` 2003 (Traverse Town) |
| Sandstorm | Sandstorm LV3 (0x4C) | 3 Aladdin `0x1401DBA70` `CFriendAladdin` | Aladdin ×3 | 0x1E | its learned bit (wiki: on meeting him) |
| Surprise! | Surprise! LV3 (0x4E) | 5 Jack `0x1401E0CF0` `CFriendJack` | Jack ×3 | 0x20 | as above |
| Spiral Wave | Spiral Wave LV3 (0x50) | 4 Ariel `0x1401DBEE0` `CFriendAriel` | Ariel ×3 | 0x1F | as above |
| Hummingbird | Hummingbird LV3 (0x52) | 6 Peter Pan `0x1401E1E40` `CFriendPan` | Peter Pan ×3 | 0x21 | as above |
| Furious Volley | Furious Volley LV3 (0x54) | 7 Beast `0x1401DCA90` `CFriendBeast` | Beast ×3 | 0x22 | as above |

- Ids and names: `SY0002.CTD`'s sleight names in id order (id 4 = Sliding Dash … 0x44 Magic LV2, 0x45
  Magic LV3, 0x46 Stardust Blitz, 0x47 / 0x48 Goofy Tornado, 0x49 Goofy Smash, 0x4A Wild Crush, 0x4B /
  0x4C Sandstorm, 0x4D / 0x4E Surprise!, 0x4F / 0x50 Spiral Wave, 0x51 / 0x52 Hummingbird, 0x53 / 0x54
  Furious Volley, 0x55 / 0x56 Lucky Bounty). Checked (files). Recipes and learn sources:
  `recom-m12-sleights.md` § 1.2 (the events Checked (files), the wiki ones Assumption).
- **The extra unlock is Re:CoM's learned bit** (`0x14023AB00(id)`): Re:CoM never forms an unlearned
  sleight (`recom-m12-sleights.md` § 1.3). The LV3 ids are learned with their LV2 (the friends' first
  meeting or the tutorial event, Assumption for the world friends). Checked (decomp); which event
  teaches each world friend's: not read.

### 1.3 How Re:CoM plays a friend sleight

All Checked (decomp) unless tagged:

1. **Start**: the sleight start `0x1402247A0` (Sora's vtable `+0x130`) covers ids 4..0x59 (its
   `lea eax,[rcx-4]; cmp eax,0x55`): the use counter, `initStockSkill` `0x140220DF0` (the attack row
   `id + 0x1A` / `+0x1D` / `+0x1E` / `+0x23`; AP mode 0: the friend computes) and **the friend's pack
   load** `0x140211790(player+0x3B8, pack)`.
2. **`exeStockSkill` `0x140218950`, category 5** (`StockSkillList +9`): the tint `0x1407C5D40` (any
   colour but white: the dim bit, so `crate::sleights::dim_shield` makes Sora untouchable while it
   shows), motion 0x2E, the "friend" effect, the player's vtable `+0x120`, and **a wait for the pack**
   (row flag 0x20: `0x1402106E0(PLM, id) == 1`). It reads the card battle's active card
   (`player+0x3E8 → +0x1E8`, White Mushroom's 0x5E Hyper Healing) **without a null test**.
3. **Sub-state 0x34F**: once the pack is in, `0x14020E950(PLM, friend, player)` builds the friend
   actor (PLM `+0x70`; Wild Crush's Goofy at `+0x78`), Re:CoM plays the friend's "friend" effect and
   the friend's own sleight entry (its vtable `+0x128` with the attack row); the friend acts and leaves
   (`CFriend::beginStateExit` `0x1401D94D0`); the sleight ends through Re:CoM's end action
   `0x1401F6660`.
4. **What it needs loaded**: only its pack, **loaded on demand** by the start (or our cursor
   preload): not per battle, not tied to the world or to the card. Re:CoM picks the costume packs
   itself (PLM `+0x88` bit 0x100 in Halloween Town rooms, set by the PLM init `0x140210100`). The pack
   ring only finishes loads in battle rooms (`recom-field-sleights.md` § 3).
5. **Lucky Bounty with no cards**: Pluto's find (`0x1401E2B50` / `0x1401E2D90` / `0x1401E2E60`) uses
   its "fewer than 7 cards" weights (5 / 5 / 90) when the revolver is nearly empty; outcome 2 (90 %)
   deals cards from the revolver's deck (`0x140166930`): **at KH1 the deck is empty, so 9 times in 10
   Pluto finds nothing**. Checked (decomp + exe: weights at `0x1407C5960`); in game: not seen.
   **Answered and built (0.118.0, owner: "a random KH1 item (Potion, Hi-Potion, Ether, Elixir by the
   floor's tier) instead of a deck card")**: § Lucky Bounty at KH1 below.

**So the native route works with no card and no special loading**: Re:CoM's own start plays any
friend sleight in a battle room, as the Sleights list does. Battle only (point 2's null read, point
4's pack loads). The field: the wt-fsl2 work makes sleights usable in the field; friend sleights
must not follow (they would read a null card battle).

## 2. Built

### 2.1 Pages (`crate::command_menu::Page`)

- Page 1 (default): Attack, Magic, Items, Sleights. Page 2: Attack, Summon, Party, Sleights.
- **D-pad Left at the root** switches the pages, on any row, the cursor staying on its row;
  **Re:CoM's cursor sound** (`0x1402AA360(1)`) in either menu look (`Sound::Page`). Replaces M12 step
  5's Summon / Sleights switch of the 4th row.
- Saved in the summon block's byte `save + 0x26B6` (`summons::field::PAGE`): 2 = page 2; anything
  else page 1, so saves from M12 step 5 (0 Summon, 1 Sleights) open on page 1. Without a save: the
  session's.
- **Row 3 is Sleights on both pages** (M12 step 5's rules: greyed when none usable, greyed and locked
  with none equipped or while a sleight runs). Without sleights it shows "Sleights" greyed (KH1 FM
  showed the blank dash there before summons).
- **Summon** (page 2 row 1): KH1's Summon list as before (Cloud first, KH1's greying); it is no
  longer the Magic list's last entry, so **no spell is needed** and the Magic list's "fewer than 3"
  place test no longer counts it. Greyed and locked until a summon is learned.
- **Lists open beside their own row**, bottom-anchored at 348 when they wouldn't fit: KH1's rule for
  each of its lists (`0x14027B870`) is `min(row y, 348 − 22 n)` (Checked (decomp): Magic case 1,
  Items case 2, Summon case 3 all reduce to it). Magic row 1, Items 2, Summon 1, Party 2, Sleights 3.
- A list whose row the page no longer shows closes (KH1's stack rule).

### 2.2 The pointing arrow (`crate::command_menu::arrow`)

- **Art: Re:CoM's own.** `Cockpit:00` (loaded with the HUD in the field and in battle) has three
  pointer triangles in its lock-on cursor (group 98, spinning: sprite 82 a large one, sprite 84 the
  pair). The arrow is sprite 84 (UV (16, 49)–(32, 64), white), found by its UV in each copy (× 2 in
  the PC language copies), drawn 16 × 10 and turned a quarter clockwise (−π/2 in the file: Re:CoM
  negates it, `kh1_hud::Step`) so it points left. Checked (files: the owner's `FO0002.SPR` / `.RTM`,
  the atlas and every sprite rendered offline). Other arrows found: the small grey ▲ (sprite 128,
  group 146), no left / right scroll arrows in `Cockpit:00`; `camp.lad` (FO0100) isn't loaded in the
  field or battle and its atlas isn't in the session.
- **A group the asset step adds** (`kh1_recom_assets::command_menu::group::ARROW`, the third after the
  command menu's plates; `kh1-mp-bar.json` `extra_groups` = 3): Re:CoM plays and draws it. The DLL makes
  one `C2DSeqCtrl` with that layer beside the root's cursor row when the rows are made (each cursor
  move), so Re:CoM plays the swing-in from frame 0, then loops the sway: nothing of ours per frame.
- **Motion: KH2's, measured** (Measured (recording): `Recording_2026-11-04_125817.mp4`, 448 × 300,
  30 fps, all 335 frames; the arrow's luminance centroid; one row = 37.7 px = 22 units):
  - **it doesn't blink**: the arrow is drawn in every frame (the contact sheet's "missing" arrows are
    on other rows: the cursor had moved);
  - it **sways** ±1.5 units about its rest place, period 31.5 frames at 60 Hz (0.53 s);
  - after each cursor move it **swings in** from 6 units right of its rest, overshoots 4.8 left (frame
    13), 2.7 right (30), 2.7 left (51), then sways.
  - Baked as integer X keys (Re:CoM's format; ±0.5 unit): intro (0, 6), (3, 7), (13, −4), (30, 3),
    (51, −2), (67, 2), then the sway (0, 2), (6, 1), (10, 0), (16, −1), (22, 0), (26, 1), (32, 2)
    from frame 67, looped (group word +4 = 0, loop start +8 = 67; Checked (decomp `0x140248F40`,
    `0x140248BB0`)). The sway's keys are where a sine through −1 … +2 crosses each integer.
- **Place**: left of the cursor row's plate, its right edge 4 units from the plate (the recording's
  7 px). KH1's look: centre at the row + (2, 30) (its selection plate at + 11); Re:CoM's look: + (−9,
  12) (the plate's left edge at the row's x). The plates' heights: Assumption (to judge in game).
- Shown while the root is on top (not with a list or the shortcut list open), on both pages, in the
  field and in battle, in both menu looks; faded with the menu.

### 2.3 Party (`crate::party`, `win::party`)

- **The list**: the owner's eleven, in his order, each with its Re:CoM id (§ 1.2), our command
  `0x80 + id` (0xC5..0xD6) and word `0x8100 + id`; laid out as the Sleights list (KH1's look: the
  Summon list's panels, no title; Re:CoM's look: Re:CoM's plates).
- **Shown only when available in this battle**: Re:CoM taught it (`0x14023AB00`) **and** every card
  it needs can drop here by § 1.1's rule (`party::can_drop`: Wild Crush needs both Donald and Goofy;
  Lucky Bounty needs Pluto met). Read **once per battle** (the battle's player) at the menu's first
  build, from Re:CoM's own getters; kept for the whole battle (owner: "Whole battle, if possible").
  Not read on page 1 unless a shortcut holds a Party sleight.
- **Greying**: an entry is usable as a Sleights-list sleight: battle room, on the ground (none has an
  air start: `initStockSkill` has no air case for 0x44..0x56), nothing running, its MP. The Party row
  is greyed when none is usable, greyed and locked when none is available or a sleight runs.
- **Running**: confirm passes KH1's commit gate, then `win::summons::start_party`: battle room and a
  card battle object only, then Re:CoM's own sleight start (the Sleights list's `start_sleight`). The
  cursor preloads the pack (`0x140211790`) as the Sleights list does.
- **Cost** (owner: "Same MP as sleights"): a tier each (Assumption, ours: friend sleights have no AP):
  2 MP for the attacks (the 3- / 4-AP tier), 3 MP for Wild Crush (the 5-AP tier: two friends), 1 MP
  for Lucky Bounty (the 2-AP tier: no damage). Owner question.
- **Their hits**: the friend actor's hits count as the sleight's (`win::summons::owner_of`: PLM
  `+0x70` / `+0x78` while a Party sleight runs): Damage = KH1 scales them, and they break into an
  attacking enemy as Stage 210's sleight hits do. Sora is untouchable while Re:CoM's tint dims the
  room (Stage 209's rule; the friend branch's tint is not white).
- **Long names squeeze to fit** (0.107.0's rule; KH1's menu font widths from the asset step, measured
  on the owner's font, units): Donald Magic 87.1, Stardust Blitz 98.8, Goofy Tornado 98.0, Goofy Smash
  86.3, Wild Crush 73.1, Lucky Bounty 91.0, Sandstorm 72.3, Surprise! 65.3, Spiral Wave 80.9,
  Hummingbird 87.9, Furious Volley 100.3, Party 40.4. In a list row (box 88): Stardust Blitz ×0.89,
  Goofy Tornado ×0.90, Lucky Bounty ×0.97, Furious Volley ×0.88; on the cursor row (67) and the
  SHORT CUT panel (68) every name but Surprise!. Checked (files: the owner's `US_font_data_tbl.bin`,
  the asset dump test; the font has '!').

### 2.4 Shortcuts

- A slot byte holds a Party sleight as its Re:CoM id (0x45..0x56, all ≥ 8: never a spell; older
  builds read it empty and keep it).
- The Customize picker lists, after the spells and the equipped sleights, the Party sleights Re:CoM
  has taught (Party order). A slot whose Party sleight isn't taught: greyed, help line "Not learned
  yet: Re:CoM teaches this friend sleight."
- The L1 list: a taught one shows its name, usable where the Party list would offer it (greyed in
  the field and where its card can't drop); the blank dash, greyed, when not taught.

## 3. Sites, hooks, per frame

- No new hook, no patch. `sites::PARTY_CHECKED`: the room-type getter `0x140233860` (35 bytes) and the
  drop table `0x1407C1F10` (208 bytes, no pointer); the others are existing checked callees
  (`0x1402339C0`, `0x1401AAAE0`, `0x140235360`, `0x1401B08C0`, `0x14023AB00`). `win::party::install`
  checks all seven; `check_sites_rs.py`: 1258 sites, 0 mismatches.
- Per frame: one atomic load for the battle's Party set; outside battle, while page 2 shows or a slot
  holds a Party sleight, Re:CoM's learned test for the eleven (not measured). The arrow is Re:CoM's
  animation.

## 4. Asset step (must be re-run)

- `FORM/0002/FO0002.SPR` (`Cockpit.lad` and its language copies): one more group after the plates
  (the arrow; an empty group in a copy without the triangle). `kh1-mp-bar.json`: `extra_groups` 3.
- `kh1-command-menu.json`: words 903 "Party" and the eleven names (without it the DLL has the English
  built in for Re:CoM's look).
- `kh1-hud.json` / the `kh1hud` set: the same words in KH1's menu font, and their widths.
- Older outputs: no arrow (logged once), Party names without KH1-font words in KH1's look (logged).

## 5. Checks

- 1:1: not a KH1 feature (KH1's menu has no pages or arrow; its party commands differ). The arrow's
  motion is KH2's, measured on the recording (±1 recorded frame, ±0.5 unit); KH2's own data not read
  (no KH2 decomp in the session). Re:CoM's drop rule and friend sleights: Checked (decomp + exe).
- Not checked: in game (all of it); the plates' heights for the arrow's place; Lucky Bounty's
  outcome with no deck; whether each world friend's LV3 is taught with its LV2.

## 6. Questions for the owner

1. **Lucky Bounty** finds nothing 9 times in 10 at KH1 (no deck to draw from): keep, or give Pluto's
   card find another prize (KH1 items, Moogle points)?
2. **MP**: 2 MP each, Wild Crush 3, Lucky Bounty 1 (our tiers): or one cost for all?
3. **The arrow in an open list**: KH2's arrow shows on the top list only (as built: the root only), or
   also beside the open list's cursor row?
4. **Row 3 with nothing equipped** now reads "Sleights" greyed on both pages (KH1 FM showed a dash
   before summons): fine?

## Lucky Bounty at KH1 (0.118.0)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Party and field follow-ups): "Lucky Bounty at
KH1 brings a random KH1 item (Potion, Hi-Potion, Ether, Elixir by the floor's tier) instead of a deck
card (the deck is empty at KH1)".

- **Re:CoM** (Checked (decomp + exe)): Pluto's action begin (`0x1401E2B50`, also `0x1401E2D90`) rolls
  the outcome into `+0x472` with Re:CoM's random `0x14029CDC0` (weights `0x1407C5960`: 20 / 60 / 20,
  or 5 / 5 / 90 with fewer than 7 cards in the revolver, `0x14016A200`); his find `0x1401E2E60`
  (called only from `0x1401E338E` / `0x1401E36A9`; no other reference: exe scan) acts on it:
  - 0: a "nonos" shot (`CHomingBullet`, `pluto` effect set);
  - 1: Re:CoM's `pluto` effect 1 and HP + Moogle-point prizes (kinds 0xD3 / 0xD5), 3 / 7 / 10 each by level;
  - 2: cards dealt from the deck: `0x140166930(player+0x248 → +0x3F0)` gives the count (3 / 6 / 9 by
    level `+0x240`), 0 → nothing, else the `pluto` effect and `0x140168650` per card. **At KH1 the
    deck is empty: nothing.**
- **Built** (`party::lucky_bounty`, `win::party::pluto_find`; sites `PLUTO_HOOKS` (9 bytes, no branch
  in: exe scan), `PLUTO_CHECKED` (the outcome switch, outcome 2's deal call, the friend effect point
  `0x1401D9DB0`)): a wrapper on the find runs Re:CoM's; then, in a KH1 battle (Combat = KH1, Sora's
  story), when the outcome was 2: one KH1 item, picked with Re:CoM's random float (`0x14029CF00`)
  equally among the floor tier's items (the shop's `STOCK` tiers through Re:CoM's floor `0x1402337C0`:
  1F Potion, Ether; 2F+ Hi-Potion; 7F+ Elixir; never a Tent or Camping Set); Re:CoM's `pluto` find
  effect 1 at his effect point (as outcomes 1 / 2 play it; the name pointer `0x1405A2F90` read and
  checked to be "pluto"); the item as a **KH1 item prize** (kind 0xD8, Re:CoM's prize look row,
  `win::drops::spawn_item_prize`) thrown from where Re:CoM throws his prizes (bone 0 `+0xD0`, 0.5 m
  up); Re:CoM collects it and its prize add gives it by KH1's pickup (first empty slot, else the
  stock). Without the drops' hooks: picked up at once (logged). Outcomes 0 and 1 stay Re:CoM's (its
  weights unchanged: at KH1 the deck is always under 7 cards, so 90 % item, 5 % shot, 5 % HP / points).
- Combat = Re:CoM and Riku's story: Re:CoM's find (the wrapper only calls it).
- Per use: one wrapper call per Pluto find; nothing per frame.
- ~~**Decided without asking**: one item per find (the owner's "a random KH1 item"; Re:CoM deals 3 / 6 / 9
  cards). Owner question: one item, or Re:CoM's count by level (3 / 6 / 9 at LV1 / 2 / 3; LV3 = 9).~~
  Answered (owner, 2026-11-04: "1-2 items"): 0.123.0 below.
- Assumptions: a KH1 battle's deck deals 0 (cardless battles), so Re:CoM's own outcome 2 did nothing
  before ours; Re:CoM's `pluto` effect 1 is the "found something" sparkle.
- Not checked: in game.

### 1–2 items per find (0.123.0)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Gravity and Lucky Bounty answers, verbatim
"1 1-2 items"): Lucky Bounty brings 1–2 KH1 items per find.

- **The split**: two at 50 % (`party::lucky_bounty::count`: Re:CoM's random float under one half),
  else one. Re:CoM's level count gives no split of its own: the Party list plays only Lucky Bounty LV3
  (0x56), whose find deals 9 cards in Re:CoM (`0x140166930` by `+0x240`; Checked (decomp), § above).
- **Each item** picked on its own as before (`pick`: equally among the floor tier's items, Re:CoM's
  random float `0x14029CF00` each; two of the same item can come), each **a KH1 item prize of its own**
  thrown from his prize point (bone 0, 0.5 m up), each with Re:CoM's own throw scatter (two random
  floats each in `win::drops::spawn_item_prize`); Re:CoM's `pluto` find effect once. KH1's pickup for
  each (first empty slot, else the stock; a full stock refuses: as any KH1 item prize).
- Draw order: the count first, then one float per item (`find`). Combat = Re:CoM, Riku's story,
  outcomes 0 / 1: unchanged. Nothing per frame; no new hook or site.
- Assumption: Re:CoM's random float is uniform on [0, 1) (so 50 % exactly).
- Checked: tests (`party::tests::lucky_bounty_brings_the_floors_items`: count, find, draws). Not
  checked: in game.
