# Enemy Skills: Re:CoM's enemy cards as KH1 abilities (plan)

The owner's decisions are in `design-philosophy-instructions.md` § "Enemy cards as abilities" (2026-11-03). The
research is below. Plan first; no code until the owner's go-ahead.

## Design (native)

- **Learning:** a skill unlocks at its card's first pickup, from Re:CoM's discovery state, as spells, summons and
  Keyblades do. It arrives unequipped and gets a notice.
- **Abilities page:** a third owner row, "Enemy Skills", with the Sora and Shared rows. Each skill is named after
  its card, has the owner's AP cost and a one-line description on the help line. The equipped skills are saved in a
  new extension of our save block.
- **Effects** run through Re:CoM's own code, by the three routes in § 3 of the research:
  1. **Play-time setters** (about 17 cards). For each equipped skill, Re:CoM's own player-side apply
     `0x140163C00(CBTL, id)` (or its setters) runs at a battle's start and again after every Sora-side clear
     `0x140161E10`. The boss cases' elemental affinity call `0x1401F8810` is skipped, so a boss skill gives the
     skill only.
  2. **The hit rewrite** `0x14015D5F0` (15 cards). A wrapper runs Re:CoM's own rewrite once per equipped skill of
     that group, each with its id in Sora's slot, which at KH1 is otherwise empty (Sora plays no cards). The slot is
     put back afterwards.
  3. **Direct compares** (about 43 sites). Each compare `slot == id` also answers true for an equipped skill (a
     checked patch per site, in batches).
- **Durations:** none. An equipped skill is permanent, except Auto-Life (once per battle). Second Chance is KH1's
  per-hit rule (already built). Re:CoM's counters touch only the slot, so a skill is never counted down.
- **The five KH1 matches** take the KH1 ability's place: its level grant goes, the card gives it. Soldier gives one
  Combo Plus (the Rod level-87 copy); Hook gives Second Chance; Hades Berserk; Creeper Plant Leaf Bracer;
  Bouncywild Treasure Magnet (KH1's; not built yet).
- **Combat = Re:CoM and Riku's story:** enemy cards as Re:CoM has them, untouched.

## Stages (one system per version)

1. The Enemy Skills section: learning, save, names, AP, descriptions, notices; the five KH1 matches (Treasure Magnet
   built as KH1's).
2. Route 1: the play-time skills.
3. Route 2: the hit-rewrite skills.
4. Route 3: the direct-compare skills, in batches.
5. The deck-to-KH1 meanings (MP recharge, no-interrupt, sleight MP, clash wins).

## Left out (owner's choice 1. C, or no meaning at KH1)

- Card values: Shadow, Screwdiver, Powerwild, Sea Neon, Pirate.
- Need an enemy card to work against: Value Break, Dispel, Mimic.
- Needs the party: Hyper Healing.
- Riku's story only: Darkball.
- Open: Wyvern (Reload Lock).

## Research: Re:CoM enemy cards: how their effects are implemented (research report)

Status: complete (research only; nothing in the repo changed). Sources: Re:CoM Steam decomp
(`/home/claude/re/out/recom/`), Steam exe (capstone), `SY0002.BIN` → `BattleCardList.bin`
(owner's extract, `/home/claude/re/recomsys/`).

## 1. Mechanism

### 1.1 The active slot (one per side)

- `CCardBattle` ("CBTL") singleton `*0x14087A818`; the player's `+0x3E8` points at the same object
  (Sora's damage check and others read `player+0x3E8 → +0x1E8`). Checked (decomp).
- Per side s = side − 1 (side 1 = Sora's side, side 2 = enemies' side):
  - `CBTL+0x1E8 + s*4` (dword, the byte at +0 = enemy card id 0x44..0x7E; copied whole from the card
    object's `+0x260`) = the **active enemy card**, "NowEnchant". 0 = none.
  - `CBTL+0x1F0 + s*4` (int) = **remaining count**.
  - `CBTL+0x178 + s*8` = the card object that was played; `CBTL+0x250 + s*8` = the on-screen
    enchant gauge object (0x88 bytes, built by `0x140165030`, updated by `0x1401677F0`).
  - Checked (decomp `0x140163170`, `0x140161E10`).
- **Concurrency: exactly one active enemy card per side.** Playing a new one while one is active
  first clears the old one (`0x140163170` calls the clear `0x140161E10` when `+0x178` is set), then
  overwrites the slot. Sora can never have two enemy-card effects at once in Re:CoM. Checked (decomp).

### 1.2 Set: `0x140163170` "CBTL set enemy card" (side, card object)

Callers `0x14016CEE0` (the card-play path) and itself (Random Flush / Mimic re-entry). It:
1. returns if `CBTL+0xB8+s*4` ≠ 0 (a play in progress);
2. clears the previous card (`0x140161E10`) if any; stores the card object and the id dword;
3. plays sound 0x20 (`0x1402AA360(0x20)`), builds the gauge object;
4. **count = `BattleCardList[id].+9`** → `CBTL+0x1F0+s*4`;
5. **side 1 only (Sora's side): calls `0x140163C00(CBTL, id)`**, the player-side "apply" switch;
6. then a per-card switch for side-independent set-up (both sides). Checked (decomp + exe).

Cases in step 6 (Checked (decomp)):

| id | what it does at play |
|---|---|
| 0x4B, 0x57 | revolver scroll `0x14016C580(rev, 0, 1, 300)` (card-value cards, not wanted) |
| 0x55 Wight Knight | `CBTL+0x208 \|= 0x4000` |
| 0x59 Darkball | if `0x1401AAAE0()` ≠ 0: `CBTL+0x208 \|= 0x80000` |
| 0x5C Wizard | `0x14016D200(rev, 1, 3)` (rev = card object `+0x2C8`) |
| 0x5D Neoshadow | `CBTL+0x208 \|= 0x1000 << (other side)`; `CBTL+0x1F8 + other*4 = 0` |
| 0x5F Black Fungus | re-rolls a random card id (`rand % 0x32 + 0x44`, or `% 0x39` with story flag `0x1401B08C0(0,0x2D)`), never 0x5F / 0x66 / (0x59 without `+0x208 & 0x20000`), writes it into the slot and re-enters `0x140163170` |
| 0x62 Crescendo | `0x14016D200(rev, 1, 2)` |
| 0x64 Parasite Cage | clears the **other** side's active enemy card (`0x140161E10(other)`), then clears its own slot |
| 0x66 Darkside | if the other side has an active card, copies its id into own slot and re-enters `0x140163170` (Mimic); else clears its own slot |
| 0x6A Oogie Boogie | `CBTL+0x208 \|= 0x400 << s`; `CBTL+0x1F8 + s*4 = 0` |

### 1.3 Player-side apply: `0x140163C00` (side 1 only)

Writes Sora's fields at play time (Checked (decomp)). Also called from `0x140164C20`
(re-apply with `CBTL+0x1E8`, side 1). Player = `*0x14087B380`.

| id | write |
|---|---|
| 0x45 Soldier | `player+0x640` (combo max) = 4 (default 3) |
| 0x46 Large Body | `0x1401F8FF0(player, 1)` |
| 0x4C Bouncywild | `player+0x54C` = 30.0f, `+0x550` = 10.0f (defaults 5.0f / 5.0f) |
| 0x4E Bandit | `player+0x640` = 1 |
| 0x4F Fat Bandit | `player+0x420 \|= 0x100000` |
| 0x56 Gargoyle | `0x1401F7EC0(player, 1)` |
| 0x63 Guard Armor | `player+0x420 \|= 0x80` |
| 0x67 Card Soldier | `player+0x420 \|= 0x100` |
| 0x68..0x7B bosses (Hades, Hook, Riku, Axel, Larxene, Vexen, Marluxia, Lexaeus, Ansem, Zexion, Xemnas, Xigbar, Xaldin, Saïx, Demyx, Luxord) | `0x1401F8810(player, rec)`: six floats → `player+0x7C4..+0x7D8` = **damage-taken % per attribute** (see § 1.6) |
| 0x6D Dragon Maleficent | if `0x1401AAB00()` ≠ 0: revolver `+0x8C` = 0x43 (default 100) |
| 0x76 Xemnas | also `player+0x420 \|= 0x1000` |
| 0x78 Xaldin | also `0x1401F6FB0(player, 1)` |
| 0x79 Saïx | also `player+0x420 \|= 0x40000000` |
| 0x7C Roxas | `player+0x420 \|= 0x20000000` (its affinity record is all 100) |

### 1.4 Clear: `0x140161E10` "CBTL::resetNowEnchant" (side)

Side 1: resets everything 0x140163C00 set (affinities to `0x1407C5B90` = all 100, `+0x640` = 3,
`+0x54C/+0x550` = 5.0f, `0x1401F7EC0(p,0)`, clears `+0x420` bits 0x100000 / 0x80 / 0x100 /
0x20000000 / 0x40000000 / 0x1000, `0x1401F8FF0(p,0)`, `0x1401F6FB0(p,0)`, revolver `+0x8C` = 100 and
`0x14016D200(rev,0,0)`), and `CBTL+0x208 &= 0xFFF7BFFF` (0x4000 and 0x80000). Both sides: clears
`+0x208` bits `0x1000 << other` and `0x400 << s`; zeroes `+0x1E8`, `+0x1EB`, `+0x1F0`; frees the gauge;
`+0x178` = `+0x250` = 0. Callers: `0x14015E010`, `0x14015E0E0`, `0x14015E5D0`, `0x140161FC0`,
`0x140162250`, `0x140163170`, `0x140163EF0`. Checked (decomp).

### 1.5 Counting: `0x14015E010(side, kind, effect)`

`BattleCardList` (`*0x14087A808`, 16-byte rows; file `BattleCardList.bin`) for enemy cards:
`+0 u16` = effect index (id − 0x43), **`+8` = count kind**, **`+9` = count**, `+0xA` = CP cost
(Assumption: value fits), `+0xB` = 5. Checked (files).

`0x14015E010`: if side's slot ≠ 0, `BattleCardList[slot].+8 == kind`, and (`effect` == 0 or
`effect == slot − 0x43`), and the gauge exists: `+0x1F0` −= 1, update gauge; at < 1 → clear
(`0x140161E10`). Checked (decomp). `0x14015E0E0` / `0x140161FC0` ("resetNowPlaying", the end of a
card play) do the same for kind `'*'` (0x2A) inline.

Count kinds and their counting sites (Checked (exe): argument registers before each call):

| kind | sites (call address: side, kind, effect) | meaning |
|---|---|---|
| 0x24 `$` | `0x14015F1A3`, `0x14015F1D2`, `0x14015F21A` (resolver `0x14015EAC0`; side r13, 0): only when the played card's type nibble = 0x10 (attack card) | per attack card played. Checked (exe) |
| 0x25 `%` | `0x14015F34A`, `0x14015FB53` (resolver, after `0x140163760`) | per sleight. Assumption (strong: `0x140163760` is on the sleight path; to confirm) |
| 0x26 `&` | `0x14015ED59`, `0x14015EEAB`, `0x14015F142`, `0x14015FDC3`, `0x14015FF16`, `0x140160047` (resolver) | per card break / clash result. Assumption (sites are in the clash resolver beside the break sounds) |
| 0x27 `'` | `0x1401E73BE` (Sora's damage check, side 1, any), `0x1401F915E` (side r14, any), `0x14017DB5B`, `0x14017DC04` (side 2, any; enemy damage) | per hit taken. Checked (decomp: in the damage checks) |
| 0x28 `(` | per-card triggers with an explicit effect index: `0x1401E7233` (1, 0x28, 0x29 = Hook), `0x1401F914F` (r14, 0x28, 0x29 = Hook), `0x140160161` / `0x140161909` (1, 0x28, 0x27 = Oogie), `0x14016019D` / `0x140161942` (2, 0x28, 0x27 = Oogie), `0x14018097D` (2, 0x28, 0x29 = Hook), `0x140180AE3` (2, 0x28, 0x2E = Vexen), `0x1401EC369` (1, 0x28, 0x2E = Vexen) | per use of that card's own trigger. Checked (exe) |
| 0x29 `)` | `0x140168376`, `0x14016A5C3`, `0x14016B40C` (revolver functions; side = revolver `+0x5C`) | per reload / revolver event. Assumption (to confirm) |
| 0x2A `*` | `0x14015E0E0` / `0x140161FC0` inline; `0x14015FC69` (resolver, when a combo count > 1) | per card play ended. Checked (decomp) |
| 0 | `0x14016BF63` | never matches an enemy card (all have kind ≠ 0) |

### 1.6 The boss affinity record (`player+0x7C4..+0x7D8`)

Read in Sora's damage check `0x1401E6DE0` (after the hit rewrite): index = the hit's attribute
`player+0x714` (0..5 → `+0x7C4 + 4·attr`; 6 and up → `+0x7D8`).
Value v: 0 → no damage (reaction 5); ≥ 1000 → 100 % and `+0x420 |= 1`, reaction 3; else damage ×
v / 100. Attributes 2 Fire, 3 Blizzard, 4 Thunder (Checked (notes, m7-magic)); 0, 1, 5 not
identified. Checked (decomp + exe for the values below).


### 1.7 The hit rewrite `0x14015D5F0(CBTL, rec)` (one function, 17 cards)

A 0x20-byte record (built by `0x140160F90` + the caller): `+0` side, `+4` handle, `+8` reaction
kind (in/out), `+0xC` attribute, `+0x10` mode (1 = this side's outgoing hit, 2 = damage this side
takes), `+0x12` u16 category bits (card `+9` nibbles → `1 << (n−1)`), `+0x14` AP / damage (in/out),
`+0x16` HP, `+0x18` max HP, `+0x1A` "sleight" (attack kind 4 / 7), `+0x1C` out: guarded, `+0x1E`
"attack card" (attack kinds 1..7, or the side's current play is one attack card). Checked (decomp +
exe). It reads the slot once at **`0x14015D60A`** (`movzx edx, byte ptr [r10+0x1E8]`,
`41 0F B6 92 E8 01 00 00`), returns at once if < 0x44, and re-reads it at `0x14015D6AE` (same bytes).
Callers (Checked (decomp)): Sora's row pick `0x1401F7520` (KH1 attack-card swing), `0x1401E66A0`
(37 sites: Sora's magic / summon / sleight / item AP, mode 1), `0x1401E67B0` (mode 2), Sora's damage
check `0x1401E6DE0` (mode 2), `0x14015D8A0` (18 sites, spells / sleight shots, mode 1, side 1),
`0x14017F0E0` (enemy side). ×1.5 and ×1.3 are `imul 0x96` / `imul 0x82` then a truncating ÷100.

## 2. Per-card sweep

Tags: **C-d** = Checked (decomp), **C-x** = Checked (exe: address and bytes read with capstone),
**C-f** = Checked (files: `BattleCardList.bin`), **A** = Assumption. "Count" = `+9` with kind `+8`
(§ 1.5; C-f). Side 1 = Sora. Only Sora-side code is listed (the enemy side has its own separate
compares, e.g. `0x14017C6E2` Axel, `0x14017E56E` Marluxia, `0x14017EFB3` / `0x140185653` Larxene,
`0x140180949` Hook, `0x140180AA3` Vexen, all on `CBTL+0x1EC`; C-x).

Effect names: the exe's pointer table `0x1407C2278` (8 bytes per effect, index = id − 0x44; e.g.
`+0x28·8` → "Second Chance" for 0x6C, `+0x38·8` → "Double Strike" for 0x7C). C-x.

### 2.1 Cards that act through the play-time writes (§ 1.3 / § 1.2), read elsewhere natively

| id | card | effect | where Sora's code tests it | what it does (Re:CoM numbers) | count |
|---|---|---|---|---|---|
| 0x45 | Soldier | Combo Plus | set `player+0x640` = 4 in `0x140163C00`; read by Re:CoM's combo chain (`0x1401E9180`: index `+0x63C` < `+0x640`) | combo length 3 → 4 cards. C-d | `)` 3 |
| 0x46 | Large Body | Guard | `0x1401F8FF0(p,1)`: `+0x420 \|= 0x200`, barrier `+0x72C \|= 2`, class mask `+0x718 \|= 1`, half-angle `+0x73C` = π/2. Tested by the barrier test `0x14016EED0` (struct `player+0x690`: `+0x9C & 2`, `+0x88` bit[hit class `+0x50`], `\|angle\|` ≤ `+0xAC`) | blocks class-0 hits from the front 180°. C-d; "class 0 = physical": A | `)` 1 |
| 0x4C | Bouncywild | Draw | `+0x54C` = 30.0, `+0x550` = 10.0 (default 5.0 / 5.0); `0x14022F23A` `cmp cl,0x4C` (`80 F9 4C`) in `0x14022EE30` picks radius² = `+0x550`² (100) instead of the constant 2.25 | pick-up / attraction radius 1.5 → 10 (units: A). C-x | `)` 5 |
| 0x4E | Bandit | Combo Finish | `player+0x640` = 1 | every card is the combo's last (finisher). C-d (meaning "finisher": A) | `)` 1 |
| 0x4F | Fat Bandit | Back Attack | `+0x420 \|= 0x100000`; read at `0x1401F5533` (`bt eax,0x14`) in `0x1401F54B0` and at `0x1401F5E6F` in `0x1401F5DC0` → `CPlayerWeapon::setBackAttack` `0x140225320` (weapon `+0x2CC \|= 0x10`, `+0x28C` = 1) | weapon's back-attack mode; the bonus itself is read from weapon `+0x28C` (not traced: A) | `)` 2 |
| 0x56 | Gargoyle | Vanish | `0x1401F7EC0(p,1)`: `+0x420 \|= 0x40`; read per frame in `0x1401F9840` (`0x1401F9D11` `test byte [rdi+0x420],0x40`), `0x1401EB6E0`, `0x1401F0AB0` | Sora's alpha pulses (cos of `+0x7DC`·0.0006, 0.1–0.95); the position stored at `0x1407C5B10` only refreshes when alpha > 0.9 (enemies aiming at it: A) | `)` 1 |
| 0x63 | Guard Armor | Wide Attack | `+0x420 \|= 0x80`; read at `0x1401F5518` (`test al,al; jns`) in `0x1401F54B0` (and `0x1401F5DC0`) → `CPlayerWeapon::setWideAttack` `0x1402257C0`: weapon `+0x2CC \|= 1`, collider scale (`0x140172690`) 0.5 → 1.0 | weapon hit size ×2. C-d / C-x. Only for attack kinds `+0x638` < 8 or 0x97..0xA4 | `$` 30 |
| 0x67 | Card Soldier | Attack Haste | `+0x420 \|= 0x100`; read in the row pick `0x1401F7A27` (`test dword [rbx+0x420],0x100`, `F7 83 20 04 00 00 00 01 00 00`): motion speed `+0x47C` = 2.0f (`0x1401A65A0`, also the weapon `+0x3D0`); also `0x140221FAC` (Sora `setAttack_inFriend`), `0x1402053CD` (Riku) | swing motion ×2.0. C-x | `$` 30 |
| 0x6D | Dragon Maleficent | Overdrive | rewrite (attack card: AP ×1.5, `0x14015D6D3` `cmp dl,0x6D`); and (Sora's story, `0x1401AAB00` = flag 0x60 clear: A "Sora's story") revolver `+0x8C` = 67 (default 100), read in `0x140167AA0` (reload charge += `+0x8C`·dt/100) | attack-card AP ×1.5; Reload charges at 67 % speed. C-d | `$` 30 |
| 0x76 | Xemnas | Quick Barrier | `+0x420 \|= 0x1000`; read in Sora's damage check `0x1401E73C3` (`test dword [rdi+0x420],0x1000`, `F7 87 20 04 00 00 00 10 00 00`) | each hit taken while no barrier: barrier timer `+0x7E4` = 6000 dt (= 2.0 s at 3000 dt/s), barrier on all classes 0..5 (`+0x718 \|= 0x3F`), half-angle π (all round); a hit that still reaches the check while it runs ends it (A: blocked hits don't reach it); timer counted down in `0x1401F9840`. C-d | `)` 3 |
| 0x78 | Xaldin | Aero Guard | `0x1401F6FB0(p,1)`: `+0x7E8` = 6000, `+0x420 \|= 0x80000000`; per frame `0x1401F9840` / `0x1401EA4C0` keep a wind object (`0x140158DD0`, power = Sora's `+0x43C`) around Sora, re-hitting every 6000 dt | orbiting wind attack. C-d (the object's hit data: not traced) | `'` 3 (hits taken) |
| 0x79 | Saïx | Combo Boost | `+0x420 \|= 0x40000000`; read in initAttack `0x1401F0EBA` (`F7 83 20 04 00 00 00 00 00 40`) → `+0x664` += 20 per attack-card in the combo (added to the card power) | +20 power per combo step (cumulative). C-x | `$` 20 |
| 0x7C | Roxas | Double Strike | `+0x420 \|= 0x20000000`; read at `0x1401F5550` (`bt eax,0x1D`) → `0x140225420` (spawns a second weapon hit) | second strike per swing. C-d (its numbers not traced) | `$` 20 |
| 0x68–0x7B bosses | (secondary) | elemental affinity | `0x1401F8810` → `player+0x7C4..0x7D8`, read in `0x1401E6DE0` | % per attribute [0..5] (2 Fire, 3 Blizzard, 4 Thunder): Hades 100/100/50/1000/100/100, Hook 100/100/1000/100/50/100, Riku 100/100/50/50/50/100, Axel 100/100/0/1000/100/100, Larxene 100/150/100/100/0/100, Vexen 100/100/1000/0/100/100, Marluxia 150/50/50/50/50/100, Lexaeus 50/150/100/0/100/100, Ansem 100/100/50/50/50/100, Zexion / Xemnas / Xigbar / Xaldin / Saïx / Luxord 100/50/50/50/50/100, Demyx 100/50/100/0/50/100, Roxas all 100. 0 = immune, ≥1000 = 100 % with reaction 3 and `+0x420 \|= 1` (A: "weak / stagger"). C-x (floats) / C-d | — |

### 2.2 Cards tested by the hit rewrite `0x14015D5F0` (§ 1.7)

| id | card | effect | test site (bytes) | what it does | count |
|---|---|---|---|---|---|
| 0x47 | Red Nocturne | Fire Boost | jump table `0x14015D79D` `add eax,-0x47` (`83 C0 B9`), case → `0x14015D7C6` `test byte [rbx+0x12],8` | non-attack-card hit with category bit 8: AP ×1.5 | `)` 1 |
| 0x48 | Blue Rhapsody | Blizzard Boost | case → `0x14015D7F1` (bit 0x10) | ×1.5 | `)` 1 |
| 0x49 | Yellow Opera | Thunder Boost | case → `0x14015D7F7` (bit 0x20) | ×1.5 | `)` 1 |
| 0x4A | Green Requiem | Cure Boost | case → `0x14015D7FD` (bit 0x40) | ×1.5 (Cure's power) | `)` 1 |
| 0x51 | Search Ghost | Drain | `0x14015D6C1` `cmp dl,0x51` (`80 FA 51`) | attack-card hit: reaction kind = 0x15 (drain: A, the 0x15 handler not traced) | `)` 1 |
| 0x5A | Defender | Protect | `0x14015D630` `cmp dl,0x5A` (`80 FA 5A`), mode 2 | damage taken with attribute 0: ÷2 (`sar` after `cdq; sub`), rec `+0x1C` = 1 | `)` 1 |
| 0x5C | Wizard | Magic Boost | case → `0x14015D803` (bit 4) | ×1.3; also dims Sora's type-3 cards (`0x14016D200(rev,1,3)`: card `+0x264 \|= 0x80000`, alpha → 0.4) and refuses them at `0x14016D003` / `0x14016DC3D` / `0x14016D95B` (`cmp cl/al,0x5C`) | `)` 1 |
| 0x62 | Crescendo | Summon Boost | case → `0x14015D815` (bit 0x80) | ×1.3; dims / refuses type-2 cards (`0x14016CFD5`, `0x14016DC27`, `0x14016D96A`). Type 2 = Magic, 3 = Summon per notes (A here: the pairing reads backwards; verify in game) | `)` 1 |
| 0x68 | Hades | Berserk | `0x14015D6CA` `cmp dl,0x68` (`80 FA 68`) | attack-card hit with HP `+0x16` < max HP `+0x18` ÷ 4: AP ×2 | `$` 30 |
| 0x6B | Ursula | Shell | `0x14015D635` `cmp dl,0x6B` (`80 FA 6B`), mode 2 | damage taken with attribute 2..4 (Fire / Blizzard / Thunder): ÷2 | `'` 5 |
| 0x6D | Dragon Maleficent | Overdrive | `0x14015D6D3` | see 2.1 | |
| 0x73 | Lexaeus | Warp Break | `0x14015D6DC` `cmp dl,0x73` (`80 FA 73`) | attack-card **sleight** hit (`+0x1A` > 0): `rand % 100 < 67` → reaction kind 0x1F (A: card-break / "warp" reaction) | `$` 50 |
| 0x75 | Zexion | Confu-Strike | `0x14015D6E1` `cmp dl,0x75` (`80 FA 75`) | attack-card hit: `rand % 100 < 25` → reaction kind 10 (confuse: A) | `$` 50 |
| 0x77 | Xigbar | Shot Charge | case → `0x14015D81B` (bits 0x118) | ×1.5 | `)` 2 |
| 0x7A | Demyx | Water Charge | case → `0x14015D826` (bits 0x210) | ×2 | `)` 2 |

### 2.3 Cards with their own compare sites in Sora's code

| id | card | effect | site(s) (bytes) | what it does | count |
|---|---|---|---|---|---|
| 0x4D | Air Soldier | Reload Kinesis | `0x1401E6C98` `cmp cl,0x4D` (`80 F9 4D`) in `0x1401E6C40` (reload start) | Sora's story: a reload request with param 0 (A: "while not idle / moving") is refused unless it is active | `)` 3 |
| 0x50 | Barrel Spider | Quickload | `0x140167C9B` `cmp edi,0xD` (effect index, after `0x140167C31` `sub edi,0x43`) in `0x140167AA0` | on reload: `rev+0x152` = 0x50 and `0x14016A450` (instant reload: A) | `)` 3 |
| 0x54 | Aquatank | Auto-Reload | `0x1401695E3` `cmp r8b,0x54` (`41 80 F8 54`) in `0x140169450` | empty revolver → reload `0x14016B2B0` by itself | `)` 1 |
| 0x58 | Air Pirate | Item Bracer | `0x14015DAE8` `cmp al,0x58` (`3C 58`, side 1) / `0x14015DBB8` (side 2) in `0x14015D970` | a play containing an Item card (type 0x40): `CBTL+0x208 \|= 0x100 << side` = that play can't be broken (`0x140164910` / `0x140164A50`) | `)` 3 |
| 0x5B | Wyvern | Reload Lock | `0x140167C34` `cmp edi,0x18` (`83 FF 18`), `0x14016839C` `cmp eax,0x18` (`83 F8 18`) | the reload counter `rev+0x80` doesn't grow (each reload takes 3000·(n−1) dt more otherwise) | `)` 1 |
| 0x5E | White Mushroom | Hyper Healing | `0x140203EEA`, `0x140215AEA`, `0x14022007A` (initFriend of Riku-dark / Riku / Sora), `0x140218B75` (Sora battle exe) — all `cmp cl,0x5E` (`80 F9 5E`) | on a Friend card: heal max HP ÷ 4 (`0x1401E59A0(p, max/4, 6000, 1, 0)`) | `)` 3 |
| 0x60 | Creeper Plant | Leaf Bracer | `0x14015DAEC` `cmp al,0x60` (`3C 60`) / `0x14015DBBC` | a play containing card 0x1D (Cure) can't be broken | `)` 1 |
| 0x61 | Tornado Step | Reload Haste | `0x140167C54` `cmp edi,0x1E` (`83 FF 1E`), `0x1401683BD` `cmp eax,0x1E` (`83 F8 1E`) | reload counter reset to 1 (fastest reload) | `)` 2 |
| 0x65 | Trickmaster | Value Break | `0x14016376E` `cmp byte [rcx+rax*4+0x1E8],0x65` (`80 BC 81 E8 01 00 00 65`) in `0x140163760` (resolver) | own play value is subtracted from the opponent's cards one by one (`0x140152150`) | `%` 10 |
| 0x69 | Jafar | Attack Bracer | `0x14015DAF0` `cmp al,0x69` (`3C 69`) / `0x14015DBC0` | a play containing an Attack card can't be broken | `&` 20 |
| 0x6C | Hook | Second Chance | `0x1401E7222` `cmp al,0x6C` (`3C 6C`) in Sora's damage check; **`0x1401F913D` `cmp ecx,0x6C` (`83 F9 6C`) in `0x1401F90B0`** (a second HP-loss path, caller `0x14041E630`) | new HP < 1 and old HP > 1 → HP 1; counted `(1,0x28,0x29)` | `(` 3 |
| 0x6E | Riku | Sleight Lock | `0x14015266F` `cmp ecx,0x2B` (`83 F9 2B`) in `0x140152490` (`BC::setPlayStock`) | stocked card (types other than 1..3): card `+0x264 \|= 0x20` | `*` 5 |
| 0x6F | Axel | Quick Recovery | `0x1401E73A8` `cmp edx,0x2C` (`83 FA 2C`) in Sora's damage check: `+0x420` bit 0x4000 on/off each hit; read at `0x1401EC1E6` (`bt eax,0xE`) in the hit-reaction state `0x1401EBD80` and decomp `0x1401E0000.c:8060` | lets Sora leave the hit reaction early (to move `0x1401E93A0`) (exact window: A) | `'` 10 |
| 0x70 | Larxene | Dash | `0x1401E9672` `cmp dl,0x70` (`80 FA 70`) in the move-speed function `0x1401E9630` | ground speed factor 1.0 → **1.7** (float at `0x1405A08BC`; default 1.0 at `0x1406C10C0`) × walk `+0x10` / run `+0x14` of the param table | `&` 15 |
| 0x71 | Vexen | Auto-Life | `0x1401F1A85` `cmp al,0x71` (`3C 71`) in `0x1401F17F0` (KO); `0x140211032` (`3C 71`) in `0x140210F10` (KO state: music fade, flag 0x10); `0x1401A1A13` `cmp cl,0x71` (`80 F9 71`) in `0x1401A16E0` (game-over controller, `+0x88` = 1); revive in `0x1401EC250` | on KO: no game over; revive with max HP ÷ 4 (`0x1401E59A0(p, max/4, 6000, 1)`), sound 0x5C, motion 0x28, then 3000 dt (1 s) of `+0x7BC` invulnerability; counted `(1,0x28,0x2E)` at `0x1401EC369` | `(` 1 |
| 0x72 | Marluxia | Double Sleight | `0x1401F5B22` `cmp cl,0x72` (`80 F9 72`) in the card-message handler `0x1401F5580` | a sleight play also sets `+0x428` bit 0x2000 (second sleight) | `*` 3 |
| 0x74 | Ansem | Sleightblind | `0x140153F48` `cmp r10d,0x31` (`41 83 FA 31`) in `0x140153E80`; `0x14016DEAA` `cmp dl,0x74` (`80 FA 74`) in `0x14016DE70` (side-2 revolver only) | card `+0x264 \|= 0x10`; the enemy revolver's sleight preview is hidden | `*` 10 |
| 0x7B | Luxord | Omni Break | `0x14015DAF4` (`3C 7B`) / `0x14015DBC4` (`CBTL+0x208 \|= 0x100000` for side 1); `0x140164999`, `0x140164B43` (`41 80 BC 9x E8 01 00 00 7B`) in the break tests | the side's plays are never broken | `&` 15 |
| 0x7D | Gold Card | Premium Guard | `0x14016D0B9` `cmp cl,0x7D` (`80 F9 7D`) in the play path `0x14016CEE0` | premium cards are played as normal cards (the premium branch is skipped) | `)` 5 |
| 0x7E | Platinum Card | Invincible | `0x1401E71F3` `cmp cl,0x7E` (`80 F9 7E`) in Sora's damage check; **`0x1401F9129` `cmp al,0x7E` (`3C 7E`)** in `0x1401F90B0` | HP isn't lowered by the hit | `&` 20 |

### 2.4 Cards handled at play time only (CBTL flags / slot tricks)

| id | card | effect | where | what | count |
|---|---|---|---|---|---|
| 0x55 | Wight Knight | Float | `CBTL+0x208 \|= 0x4000` (`0x140163170`); **central getter `0x1401615E0`** (returns 1 if the bit is set), called by `0x1401A59A0` (every actor's gravity scale 1.0 → 0.5 unless actor flag 0x200000) and `0x1401FAB40` (Sora's gravity `+0x468` × 0.5) | gravity halved for everyone (either side's card) | `)` 3 |
| 0x5D | Neoshadow | Bio | `CBTL+0x208 \|= 0x1000 << other side` | Sora's: every 1000 dt each enemy object (type 0x1004) gets `vt+0xD8(…,1)` (in `0x1401617F0`; 1 damage: A) | `)` 1 |
| 0x5F | Black Fungus | Random Flush | `0x140163170` | the slot becomes a random enemy card (0x44..0x75, or 0x44..0x7C with story flag 0x2D), never 0x5F / 0x66 / 0x59 | `&` 1 (the new card's own count then applies) |
| 0x64 | Parasite Cage | Dispel | `0x140163170` | clears the opponent's active enemy card, then its own | `&` 1 |
| 0x66 | Darkside | Mimic | `0x140163170` | copies the opponent's active enemy card | `&` 1 |
| 0x6A | Oogie Boogie | Regen | `CBTL+0x208 \|= 0x400 << s`; tick in `0x1401600D0` / `0x1401617F0` | every 6000 dt (2 s), not KO'd, HP < max: heal max(1, (max − HP) ÷ 8) (`0x1401E59A0(p, n, 3000, 1, 0)`), counted `(1,0x28,0x27)` | `(` 10 |

### 2.5 Not in the wanted list / not found

- 0x44 Shadow (Incrementor), 0x4B Powerwild (Retrograde), 0x52 Sea Neon (Random Values),
  0x53 Screwdiver (Decrementor), 0x57 Pirate (All Zeros): card-value effects (`0x140151A70`
  switch on id − 0x44, revolver scroll at play). Not swept. Note: the brief's table lists 0x53, the
  design doc excludes Screwdiver.
- **0x59 Darkball (Duel Trigger)**: not in the brief's table; `CBTL+0x208 \|= 0x80000` (Sora's
  story), read by the break tests `0x140164910` / `0x140164A50` with `+0x20C` == 1 (duel). Card-only.
- **Not traced to the end** (the test site is found, the downstream number is not): Drain's reaction
  0x15, Lexaeus' 0x1F, Zexion's 10; Back Attack's bonus on weapon `+0x28C`; Double Strike's second
  hit `0x140225420`; Aero Guard's wind object hit data; Quick Recovery's exact window; Quickload's
  `0x14016A450`; Bio's damage amount.

## 3. Choke point assessment

**There is no single getter.** Re:CoM keeps one byte per side (`CBTL+0x1E8` for Sora) and reads it
in three different ways (Checked (decomp + exe), all sites in § 2):

1. **Play-time writers** `0x140163170` (both sides) and `0x140163C00` (Sora only) turn 0x45, 0x46,
   0x4C, 0x4E, 0x4F, 0x55, 0x56, 0x5D, 0x63, 0x67, 0x6A, 0x6D (reload), 0x76, 0x78, 0x79, 0x7C and
   the boss affinities into **player fields / flag bits / CBTL flags**. Their readers never look at the
   card: weapon setters `0x1401F54B0` / `0x1401F5DC0` (Wide, Back Attack, Double Strike), row pick
   `0x1401F7A27` (Haste), initAttack `0x1401F0EBA` (Combo Boost), the damage check (Xemnas,
   affinities), per-frame `0x1401F9840` (Vanish, Aero Guard, barrier timer), combo code (`+0x640`),
   the Float getter `0x1401615E0`, the Regen / Bio tick `0x1401600D0` / `0x1401617F0`.
   - Native lever: **call the same Re:CoM setters** (`0x1401F8FF0`, `0x1401F7EC0`, `0x1401F6FB0`,
     `0x1401F8810`, the `+0x420` bits, `+0x640`, `+0x54C/+0x550`, `CBTL+0x208` bits) for an equipped
     skill; no compare site needs a patch. Re:CoM's own `0x140164C20` already re-applies
     `0x140163C00` from the slot to a new player object, so "apply this card's player-side effect"
     exists natively as one call: `0x140163C00(CBTL, id)` (entry `lea eax,[rdx-0x45]; cmp eax,0x37`
     at `0x140163C14`, `8D 42 BB 83 F8 37`).
   - **Risk** (Checked (decomp)): the side-1 clear `0x140161E10` resets *all* of those fields at once
     (affinities to 100, `+0x640` = 3, radius 5.0, clears `+0x420` bits 0x100000 / 0x80 / 0x100 /
     0x1000 / 0x20000000 / 0x40000000, Guard / Vanish / Aero off, reload rate 100, `CBTL+0x208` bits
     0x4000 / 0x80000 / 0x400 / 0x2000). It runs whenever Sora's slot ends or is replaced, at battle
     end (`0x140162250`), and when an **enemy plays Parasite Cage (Dispel)**, which clears the
     opponent's side (`0x140163170` case 0x64). So field-based skills need re-applying after every
     side-1 clear (one wrapper on `0x140161E10`, or Re:CoM's re-apply after it), or the clear must
     learn to restore equipped skills.
   - Note: `0x140163C00` also applies the boss **affinity** record; an equipped boss skill would bring
     the boss's elemental resistances/weaknesses with it unless the skill skips `0x1401F8810`.

2. **The hit rewrite `0x14015D5F0`**: one function, one slot read (`0x14015D60A`, re-read at
   `0x14015D6AE`), covering 0x47–0x4A, 0x51, 0x5A, 0x5C, 0x62, 0x68, 0x6B, 0x6D, 0x73, 0x75, 0x77,
   0x7A. This is the closest thing to a choke point: every Sora hit (row pick, magic / sleight AP via
   `0x1401E66A0` and `0x14015D8A0`) and Sora's damage check go through it. But it handles **one card
   per call**: a set of equipped skills means running its body once per equipped skill (e.g. a
   wrapper that calls it with each skill's id in a scratch slot), or rewriting its tests as bit tests.

3. **Direct compare sites**: one compare per site, `cmp <reg8>, id` after
   `movzx <reg>, byte [<CBTL>+0x1E8]` (or `+0x1E4 + side·4`, or effect index `id − 0x43`). Per card:
   Second Chance 2 (`0x1401E7222`, `0x1401F913D`), Invincible 2 (`0x1401E71F3`, `0x1401F9129`),
   Auto-Life 3 (+ revive count), Hyper Healing 4, Quick Recovery 1 (sets a bit read once), Dash 1,
   Reload Kinesis 1, Draw 1 (+ fields), Double Sleight 1, Item / Leaf / Attack Bracer 2 each
   (`0x14015D970`, sides 1 / 2), Omni Break 4 (`0x14015DAF4`, `0x14015DBC4`, `0x140164999`, `0x140164B43`), Value Break 1, Reload Lock 2, Reload Haste 2,
   Quickload 1, Auto-Reload 1, Sleight Lock 1, Sleightblind 2, Gold 1, Wizard 3, Crescendo 3 (plus their rewrite cases and play-time card dimming).
   Each would need its own patch (a jump to "slot == id **or** skill equipped") — about **43 sites**
   for the whole list, each a 2–8-byte compare (`3C xx`, `80 F9 xx`, `80 FA xx`, `83 F9 xx`, …), most
   with the `cmp <reg8>,0x43; jbe` guard just before.

**Concurrency** (Checked (decomp)): one active card per side. Equipped skills (many at once) can't
be expressed through the slot. Native options are (a) the setter route for class 1, plus (b) a
per-skill "equipped" mask tested next to each class-2/3 compare; the counters (`0x14015E010`,
kinds `$ % & ' ( ) *`) only ever touch the slot, so a permanently equipped skill is never counted
down, and a real enemy card played meanwhile keeps its own count.

**Counting summary** (§ 1.5): `$` attack cards played, `%` sleights (A), `&` breaks (A), `'` hits
taken, `(` the card's own trigger, `)` revolver / reload events (A), `*` card plays ended. Durations
are counts, not time; the only timers are Regen's 6000 dt tick, Bio's 1000 dt tick, Xemnas'
2 s barrier, Aero Guard's 6000 dt re-hit and Auto-Life's 1 s invulnerability (dt = 3000 per second,
`docs/recom-timing.md`).

## 4. What I could not check

- No in-game trace: every number is from the decomp and exe, none confirmed in play.
- Meanings tagged A above (reaction kinds 0x15 / 0x1F / 10, attribute classes 0 / 1 / 5, the barrier
  class mask, the `)` / `%` / `&` count meanings, Wizard / Crescendo card-type pairing).
- Wide Attack's bit is read only through `test al,al; jns` in the two weapon setters; a broader
  hunt for other readers found none (scans of every `+0x420` / `+0x3C8` test / bt / byte access).
- The enemy-side implementations (`CBTL+0x1EC`) were listed only where met, not swept.
