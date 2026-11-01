# M7 stage 7.11: spell unlocks (decision 34)

Owner (decision 34): "Spells and spell levels are earned from certain bosses / events"; "Spell
unlocks: each spell's base level when Sora first gets that Re:CoM magic card; the -ra / -ga levels at
Re:CoM bosses." The table below is for the owner to review (owner away, 2026-11-01: "keep going").

Code: `crates/kh1-recom/src/unlocks.rs` (the table and the rule, tested), `src/win/unlocks.rs` (the
wrapper and the save), the one-time sync in `src/win/mp.rs` (`spell_state`, formerly `spell_levels`). The command menu
already lists the levels kept in the save (`docs/recom-m7-command-menu.md`); the debug switch
`kh1_all_spells = on` still gives every spell at level 3.

Steam addresses (image base 0x140000000). Tags: **Checked (decomp)**, **Checked (exe)**, **Checked
(files)**, **Assumption**.

## KH1 (what it does)

| what | KH1 (Steam) | tag |
|---|---|---|
| A spell level is learned through event syscall 458 (`0x1401AF730`, table `0x1404FA870`) → `0x14028F560(spell, level)`: writes `save + 0x492 + spell` = level and loads that level's `.mag`. The level is set, not added. | `0x14028F560` | Checked (decomp + exe) |
| Where: in the room archives of this session, `aw02.ard` (argument 1 = Blizzard, Wonderland's keyhole) and `tw02.ard` (6 = Aero, Opposite Armor). The rest are in event files not in this session; the list below is StrategyWiki's. | | Checked (files) for the two; the rest Assumption |

KH1's learns (research `m7-magic.md` § 4.1.3):

| Spell | Level 1 | Level 2 | Level 3 |
|---|---|---|---|
| Fire | Guard Armor (Traverse Town) | Genie Jafar (Agrabah) | Hollow Bastion keyhole |
| Blizzard | Wonderland keyhole (Checked: `aw02`) | Jafar (Agrabah) | Behemoth, Hades Cup |
| Thunder | Coliseum barrel challenge | Giant Ursula (Atlantica) | Cerberus, Hades Cup |
| Cure | Clayton (Deep Jungle) | Neverland ship deck (the Captain Hook fight) | Aerith, Hollow Bastion library |
| Gravity | Phil Cup | Halloween Town keyhole | Hades, Hades Cup |
| Stop | Parasite Cage 2 (Monstro) | Pooh's Swing (100 Acre Wood) | Phantom (Neverland) |
| Aero | Opposite Armor (Checked: `tw02`) | Neverland Trinity chest | 99 Dalmatians |

## Re:CoM's side (what the unlocks read). Checked (decomp + exe)

- **Every card Sora gets** (battle drops and rewards `0x1401F4AB0`, event gifts `0x140145C50`, packs
  `0x14011B900`, Moogle shops `0x1403A66F0`, the new-game deck `0x14015C610`) goes through the add
  `0x140158FC0`; for types 1–4 and 7 that calls `0x14015CDB0`, which turns the card id's discovery
  state 0 → 1 (`0x14015CAE0`) and **1 → 2 = "obtained for the first time"** (`0x14015C7F0`, bytes
  `48 83 EC 28 B8 FF 00 00 00`; the call pair at `0x14015CDFB`). The state (2 bits per id at
  `coll + 0x9AC`, `coll = save + 0x1A74`) is saved and only cleared by a new game; "obtained at
  least once" = state 2 or 3 (`0x14015B3F0`, which the Keyblades page already uses).
  `recom-cards-save.md` § 2–3.
- **Magic cards:** kinds `0x1A`..`0x20` (Fire, Blizzard, Thunder, Cure, Gravity, Stop, Aero), with
  premium versions (`| 0x100`); obtainable in Sora's story (card-info byte 3 bit 1). Checked (exe:
  the card-info table `0x1405256D0`).
- **Bosses:** each Re:CoM boss has an enemy card (type 7) that Sora gets for beating it: `0x63`
  Guard Armor, `0x64` Parasite Cage, `0x65` Trickmaster, `0x66` Darkside, `0x68` Hades, `0x69` Jafar,
  `0x6A` Oogie Boogie, `0x6B` Ursula, `0x6C` Hook, `0x6D` Dragon Maleficent, `0x6E` Riku, `0x6F` Axel,
  `0x70` Larxene, `0x71` Vexen, `0x72` Marluxia (Sora's story: byte 3 bit 1). Checked (exe: names and
  bits). **That the card comes with the boss's first defeat (and not before, e.g. from a pack) is an
  Assumption** (the boss rewards are data: event scripts and Rooms1, not in this session). So the
  enemy card's first pickup is how a boss win is seen: Re:CoM's own reward, through the same
  wrapper, and it is in the save for older games.

## The rule

- **Base level (1)** when the spell's Re:CoM magic card (normal or premium) has been obtained at
  least once.
- **-ra / -ga:** each spell has **two bosses** (table below). Once Sora has the spell, **each of its
  bosses beaten raises it one level** (to 3 at most). Re:CoM lets the player choose the order of
  floors 2–6 and 7–10, so a fixed "this boss gives level 3" could give level 3 before 2 (KH1 sets
  the level outright); "+1 per boss" keeps KH1's order (2, then 3) in any floor order. A boss beaten
  before the spell's card counts when the card comes.
- Kept in our save block (`save + 0x267A`, 7 bytes, `recom-m7-mp.md`), raised, never lowered.

## The table (owner to review)

How it is built (the ask: "following KH1's order and same-boss pairings where they exist, gaps
filled by Re:CoM floor bosses in story order"):

1. **Same boss:** KH1 gives the level for a boss Re:CoM has too.
2. **Same world:** KH1 gives it at the world's keyhole, right after the world's boss that Re:CoM
   has (Halloween Town, Hollow Bastion).
3. **Gaps:** the other seven KH1 learns, **in KH1's order** (Aerora, Stopra, Curaga, Stopga,
   Blizzaga, Thundaga, Aeroga: Neverland, 100 Acre Wood, the Hollow Bastion library, the Phantom,
   the Hades Cup, the Dalmatians; Assumption: the usual play order), on the Re:CoM bosses left, **in
   Re:CoM's story order**: Axel (1F), Trickmaster and Parasite Cage (floors 2–6; Wonderland before
   Monstro as in KH1), Larxene I, Riku Replica I, Vexen I, Darkside (12F).
4. Not used: **Guard Armor** (KH1's Guard Armor gives level-1 Fire; Re:CoM's base levels come from
   the cards) and **Marluxia** (the last fight: nothing is left to cast after it).

| # | Re:CoM boss (enemy card) | where in Re:CoM | spell | KH1's level there | KH1's source | rule |
|---|---|---|---|---|---|---|
| 1 | Jafar (`0x69`) | floors 2–6 (Agrabah) | Fire | 2 (Fira) | Genie Jafar (Agrabah) | same boss |
| 2 | Jafar (`0x69`) | floors 2–6 (Agrabah) | Blizzard | 2 (Blizzara) | Jafar (Agrabah) | same boss |
| 3 | Ursula (`0x6B`) | floors 7–10 (Atlantica) | Thunder | 2 (Thundara) | Giant Ursula | same boss |
| 4 | Captain Hook (`0x6C`) | floors 7–10 (Neverland) | Cure | 2 (Cura) | the ship's deck: Captain Hook | same boss |
| 5 | Hades (`0x68`) | floors 2–6 (Olympus Coliseum) | Gravity | 3 (Graviga) | Hades, Hades Cup | same boss |
| 6 | Oogie Boogie (`0x6A`) | floors 2–6 (Halloween Town) | Gravity | 2 (Gravira) | Halloween Town keyhole | same world |
| 7 | Dragon Maleficent (`0x6D`) | floors 7–10 (Hollow Bastion) | Fire | 3 (Firaga) | Hollow Bastion keyhole | same world |
| 8 | Axel (`0x6F`) | 1F, after Traverse Town (Axel I) | Aero | 2 (Aerora) | Neverland Trinity chest | gap |
| 9 | Trickmaster (`0x65`) | floors 2–6 (Wonderland) | Stop | 2 (Stopra) | Pooh's Swing | gap |
| 10 | Parasite Cage (`0x64`) | floors 2–6 (Monstro) | Cure | 3 (Curaga) | Aerith, HB library | gap |
| 11 | Larxene (`0x70`) | Larxene I | Stop | 3 (Stopga) | Phantom | gap |
| 12 | Riku Replica (`0x6E`) | Riku Replica I | Blizzard | 3 (Blizzaga) | Behemoth, Hades Cup | gap |
| 13 | Vexen (`0x71`) | Vexen I | Thunder | 3 (Thundaga) | Cerberus, Hades Cup | gap |
| 14 | Darkside (`0x66`) | 12F (Destiny Islands) | Aero | 3 (Aeroga) | 99 Dalmatians | gap |

By spell: Fire — Jafar, Dragon Maleficent; Blizzard — Jafar, Riku Replica; Thunder — Ursula, Vexen;
Cure — Captain Hook, Parasite Cage; Gravity — Oogie Boogie, Hades; Stop — Trickmaster, Larxene;
Aero — Axel, Darkside.

Tags: the card ids and names Checked (exe); the floors of the world bosses Checked (decomp: Sora's
world choices `0x14051D572` = 2..6 for floors 2–6, `0x14051D588` = 7..10 for floors 7–10, 11 / 12 /
13 fixed; `recom-level-world.md` § 3); **the floors of Axel I, Larxene I, Riku Replica I and Vexen I
are Assumption** (from the game's story, not read: Rooms1 / event data aren't in this session);
KH1's sources Assumption except Blizzard 1 and Aero 1 (Checked (files)); "Cura at the Hook fight"
Assumption (StrategyWiki says the ship's deck, where KH1's Hook is fought).

## In game

- **The wrapper** on `0x14015C7F0` (`sites::UNLOCK_HOOKS`; the 9 bytes moved are whole instructions,
  no RIP operand): Re:CoM's function first, then, for a magic card or a table boss's card, the sync:
  Re:CoM's discovery state (`0x14015B3F0` for the 7 magic cards, their premiums and the 13 boss
  cards) → `unlocks::levels` → merged into the save (never lowered) → each rise logged. **In Sora's
  story, at either Combat setting** (only our save bytes are written, which Re:CoM never reads; so a
  game played at the Re:CoM setting has its spells when switched to KH1). Nothing in Riku's story.
  Guards: a save and its collection (`*0x14087A7E8 = save + 0x1A74`), else nothing.
- **Older saves:** the first read of the spells at the KH1 setting (the command menu reads them each
  frame through `win::mp::spell_state`) runs the same sync once when the block's spare byte
  (`save + 0x2684`) bit 0 is clear, then sets it. Saves from before this stage have it 0, so they get
  their spells at their first KH1 frame. A new game zeroes the block (Re:CoM zeroes the save).
- **Shortcuts (stage 7.12):** the same sync fills the shortcut slots (`save + 0x2681..0x2683`) as
  KH1's learn (`0x140285260`, Checked (decomp)): each spell learned (level 0 → more) takes the first
  empty slot. A save whose spare bit 1 is clear (every save before 7.12) first gets the spells it
  already had, in KH1's order, first three (owner, 2026-11-01), then the bit is set; the first read
  runs the sync when either bit is missing. Logged `unlocks: shortcut 1 = Fire (…)`.
  `recom-m7-command-menu.md` § 7.12.
- **Per frame:** one byte test inside the read the menu already makes. The wrapper runs only when
  Re:CoM adds a card.
- **Sites:** hooked `0x14015C7F0` `48 83 EC 28 B8 FF 00 00 00`; checked only `0x14015CDFB` (`0F B7 CB
  E8 DD FC FF FF 0F B7 CB E8 E5 F9 FF FF`: the add's two calls, → `0x14015CAE0`, → `0x14015C7F0`);
  called `0x14015B3F0` (Keyblades' site, its RIP load is `0x14087A7E8`). Checked (exe: every literal
  site of `sites.rs` compared with the Steam exe, 327 / 327; `sites::tests::unlock_sites`).
- **Logs:** start `unlocks: KH1 spell unlocks in Sora's story (M7 stage 7.11): …`; each learn
  (60 lines): `unlocks: learned Fire (spell 0 level 0 -> 1): card 0x1a (Fire magic card) obtained
  for the first time`, `unlocks: Fire -> Fira (spell 0 level 1 -> 2): card 0x69 (Jafar enemy card:
  Jafar beaten) obtained for the first time`, or `…: the first read of this save's spells at the KH1
  setting (Re:CoM's discovery state)`.

## 1:1 checks

- Checked (decomp + exe + tests: `unlocks::tests`): two bosses a spell and a level-2 and a level-3
  row each; the cards are Re:CoM's Sora-story boss cards; one card per boss but Jafar's two; base
  level from normal or premium cards; a boss before the card counts later; any floor order gives 2
  then 3; Guard Armor / Marluxia raise nothing; the save keeps the highest; the wrapper's site and
  the add path.
- Not 1:1 by design: KH1's sources (Deep Jungle, the Coliseum cups, the Dalmatians, the Trinity
  chest, Aerith, the Phantom) don't exist in Re:CoM; the base levels come with Re:CoM's cards
  (decision 34), not KH1's level-1 events.
- **Assumptions:** (1) each boss's enemy card is first obtained at its first defeat (Re:CoM's
  reward) and not earlier (packs, shops); (2) the Organization fights' floors in the table; (3) KH1's
  order of the gap learns; (4) Re:CoM's new game zeroes our block before the new-game deck is added
  (if not, the first-obtain wrapper re-syncs anyway).
- **Not checked:** in game.

## Owner questions

1. **The table** (above): keep, or move rows (any boss, any spell; two per spell).
2. **The order rule:** (a) "+1 per boss" (built: always 2 then 3, whatever the floor order);
   (b) each row sets its own level (KH1's way: beating Hades before Oogie gives Graviga at once, and
   Oogie then gives nothing).
3. **Org fights fought more than once** (Axel, Larxene, Riku Replica, Vexen, Marluxia): the card,
   and so the learn, comes at the first fight only. Move a learn to a later fight instead? (Needs a
   different signal than the card: the boss battle's end with that boss class.)

## In-game checklist

1. Log: `unlocks: KH1 spell unlocks in Sora's story (M7 stage 7.11): …` after the `magic:` start line.
2. An older save at the KH1 setting: at the first frame with the command menu, `unlocks: learned …:
   the first read of this save's spells …` for each spell whose card Sora ever had, and the levels
   from the bosses already beaten; the Magic list shows them (with `kh1_all_spells = off`).
3. A new game: no Magic until the first magic card; picking up Fire: `unlocks: learned Fire …: card
   0x1a …`, Fire in the Magic list at once.
4. Beat Jafar with Fire and Blizzard owned: `Fire -> Fira`, `Blizzard -> Blizzara`; the Magic list
   shows Fira and Blizzara.
5. Beat a boss before having its spell's card: nothing; when the card comes, the level jumps (e.g.
   `learned Fira`).
6. Save, quit, load: the levels stay. Riku's story: no `unlocks:` lines. At the Re:CoM setting:
   learns are still logged (the save remembers them for KH1); Re:CoM's play unchanged.
