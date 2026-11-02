# Re:CoM's gimmick cards at Combat = KH1 ("Stagger")

Owner (2026-11-01): "Also, certain bosses drop "gimmick cards" in their battles. I want these to drop,
and when sora picks them up, i want there to be a triangle prompt that says "Stagger," and pressing
triangle will activate the card".

Steam addresses (Re:CoM image at `0x140000000`). Tags: Checked (decomp / exe / files / tests) or
Assumption. Code: `crates/kh1-recom/src/gimmick.rs` (pure), `win/gimmick.rs`, `win/drops.rs`
(`prize_add`), `win/attack.rs` (`request_bits`), `command_menu.rs` / `win/command_menu.rs` (the
context tab), `sites.rs` (`GIMMICK_*`), `crates/kh1-recom-assets/src/command_menu.rs` (`OUR_WORDS`).
Research helpers (RTTI vtable map, disassembly) stay with the decomp tools, outside the repo.

KH1 has no gimmick cards: this is a Re:CoM mechanic kept at Combat = KH1. There is no KH1 number
to match; every number below is Re:CoM's own (Checked (decomp)), and Re:CoM's own code runs it.

## 1. Which cards, which bosses

The gimmick cards are card kinds **0x3A..=0x41, type 6** (card-info row byte 0 = `0x66` in both
copies of the table, `0x14059F2E0` (the prize add's) and `0x140528C30` (the revolver play's);
byte 3 = 6: obtainable in Sora's and Riku's stories). Names from the rows' name pointers
(`0x1405063B8..`). Checked (exe).

| Kind | Card | Boss class (Sora's story world) |
|---|---|---|
| 0x3A | Guard Armor | `CBossGuardarmor` (Traverse Town) |
| 0x3B | Trickmaster | `CBossTrickMaster` (Wonderland) |
| 0x3C | Jafar | `CBossJafar` (Agrabah) |
| 0x3D | Oogie Boogie | `CBossBoogie`, dropped by `CBossBoogieDice` (Halloween Town) |
| 0x3E | Parasite Cage | `CBossParasiteCage` (Monstro) |
| 0x3F | Ursula | `CBossUrsula`, dropped by `CBossUrsulaTentacle` (Atlantica) |
| 0x40 | Dragon Maleficent | `CBossMDragon` (Hollow Bastion) |
| 0x41 | Darkside | `CBossDarkSide` (Destiny Islands) |

(Worlds: Assumption from the names.) Kinds 0x42 / 0x43 are type `0xE` (no name; 0x42 Riku's story
only): Riku's story's, not touched.

### How each drop is decided (Checked (decomp), RTTI vtables from the exe)

Every drop is the boss's own code: it names itself the card's receiver (`0x140163740`: CBTL
`+0x200` = its handle), then makes the card's prize with Re:CoM's prize factory `0x1401A89C0`
(descriptor `+0x20` = the kind, launched like EXP orbs). The chance is a percentage against
`rand() % 100` (`0x14029CDC0`), some read from the enemy's parameter file by name
(`0x14017F4E0`; those files are `ENEMY/<id>/EN<id>.EPD` in `Recom.pkg`, **not in this session**:
the named values are not checked).

The enemy state slots: state `s` begins through vtable `+0xE8 + 0x18·s` (`0x14087A910..` table;
Checked (disasm of its 31 stubs)). State 13 is the **card-broken stagger** (CBTL message 3 →
`enemy+0x800`); state 30 is the **defeat** (its base `0x14017BEB0` → `0x14017B310`: journal,
kill count).

| Card | Where (function, slot) | When | Chance |
|---|---|---|---|
| Guard Armor | `0x1403AB290`, state 13 begin (`+0x220`) | Sora breaks its card | 30 % (literal) |
| Trickmaster | `0x1403CB1C0`, state 13 begin | Sora breaks its card, with its table object's `+0x2C8 & 1` | param `GMKCardRate` |
| Jafar | `0x1403D0D80`, state 13 begin → `0x1403CFE10` | Sora breaks its card, Jafar not in state 15 | param `SGCardRate` (Sora) / `RGCardRate` (Riku) |
| Jafar | `0x1403D1F80` (`+0x3D8`), from the hit handler `0x14017F7E0` on a hit to a guarding body (`+8 & 0x18`) | a hit of attribute 3 (Blizzard's, `BattleCardList +0xB`), Sora's story only | 100 % |
| Parasite Cage | `0x1403C2B70`, state 13 begin | Sora breaks its card, its card not used yet (`+0xCD4 & 2`) | 30 % (literal) |
| Dragon Maleficent | `0x1403E1690`, state 13 begin | Sora breaks its card, no footing of its own out (`+0xD18`) | param `GimmcikCard` (sic), read by `0x1403E2300` (its set-up) |
| Oogie Boogie | `0x1403D7C90`, a dice's defeat (state 30 begin, `+0x3B8`) | a dice with `+0xC88 & 1` is defeated | param `GMCPar` (Oogie's) |
| Ursula | `0x1403DC6B0`, a tentacle's defeat (state 30 begin) | a tentacle's HP ≤ 0 | param `GCardRate` (Ursula's) |
| Darkside | `0x1403E9CB0`, its state 2 exe (`+0x120`) | after its state-2 attack spawned its 5 Heartless (`+0xCE8` = 1 at `0x1403EA2D0`), once none is alive and none of its 6 parts has `+0x2C0` | 100 % |

## 2. Using one in Re:CoM (Checked (decomp + disasm) unless tagged)

1. **Pickup**: the prize's reward tail-calls the prize add `0x1401F4AB0(kind, arg, 1)`. Type 5 / 6
   (`0x1401F4BDA`): Sora's battle revolver (`player+0x3F0`) takes it (`0x140166300`): for type 6
   only while CBTL's gimmick count `+0x1D0` is ≤ 0, into its pending slot 0, count + 1
   (`0x14015D5B0(1)`). **One gimmick card at a time**; a second is lost. Then `0x14010A0C0(4, kind)`
   (the journal / trophy notice) and the event variable (1, 0xF) = 1. Gimmick cards never enter the
   collection.
2. **Play** (the use-card button → `0x14016CEE0`): refused while CBTL `+0x208 & 0xC0000000`, the
   revolver isn't live (`+0x60`), the system flag (1, 8) is set, the event-hide mask (1, 0xE2) has
   Sora's bit 2, or a gimmick card is already playing (CBTL `+0x1E4`). Type 6: CBTL `+0x1E4` = the
   card dword (kind, value 0, type byte), `+0x208 |= 0x8000` (`0x140161D60(&card, 1)`); then CBTL
   plays it on side 1. Gimmick cards **can't be stocked** (`0x14016DB60` skips types 6, 8, 0xE),
   so **no sleights** use them.
3. **Message 1** (CBTL → Sora's card handler `0x1401F5580`, vtable `+0x48`): `+0x7F4` = the kind,
   `+0x428 |= 0x200` (type 6 case `0x1401F58E8`). The used card leaves the revolver (`0x1401501B0`:
   gimmick count − 1).
4. **Request**: the builder's battle branch makes `+0x424 |= 0x40` (`0x1401F85F1`); the move exe
   (`0x1401EEE39`) starts the **gimmick action 0x19** (`0x1401F25D0`); the Battle exe's case 0x19
   runs its exe `0x1401ECA70`.
5. **The action** (`0x1401ECA70`): sub 1 plays Sora's card motion (`0x1401F7290(pl, 0xFF0, …)` in
   Sora's story: Assumption that it is the card-raise clip); sub 2, at motion frame 50 (or action
   time 5000 = 100 frames), posts **message 6** to the receiver (`0x140161D60(0, 0)`: data = CBTL
   `+0x1E4`); sub 3 waits until the boss clears `+0x1E4` (`0x140161D30`), then ends the card
   (`0x14015E0E0(CBTL, 1)`: "resetNowPlaying", also sets event variable (1, 0x14)) and returns to
   idle (`0x1401F6660`). Sora can't act during it.
6. **The boss** (message 6 → its vtable `+0x3F8`; the base `0x1401815F0` does nothing):

| Card | Handler | What it does (code: Checked; what it looks like: Assumption) | How long |
|---|---|---|---|
| Guard Armor | `0x1403AD160` | enters its state 17 (`0x14017D1F0(boss, 0x11)`): its parts' collapse | its state 17's own length |
| Trickmaster | `0x1403CC340` | its two table objects (`+0xCE8`, `+0xCF0`) to state 2 for `TblEffTime` | param `TblEffTime` |
| Jafar | `0x1403D1F30` | sound 0x3FBB6, state 15 for 60.0 (`0x1403D0150(boss, 0xF, 60.0)`): stunned | 60.0 (units of its state: not read) |
| Oogie Boogie | `0x1403D8FA0` | its machine `+0xD10`: count `+0x2E0` + 3 (≤ 3); down time `HDownTime` + `BonusTime1..3` by `+0x2E4`; its 3 effects end; effect `_nm_3000` | params |
| Parasite Cage | `0x1403C43C0` | its parts' states, `+0xCD4 |= 2` (no more drops), sound 0x41321, effect `rec_pi_boss.dpx`, **lets Sora go** (`0x1401EA150(Sora, 0)`), timer `+0x160` = 30000 | 600 frames (30000 / 50) |
| Ursula | `0x1403DD7E0` | `+0xCD8 |= 8` (her state code reacts and clears `+0x1E4` later, `0x1403DF5B0`) | her code's |
| Dragon Maleficent | `0x1403E3680` | makes a **footing** object (kind 0xB, id 0x3B6) near Sora (`FootingWait` / `FootingRide`), Sora rides it to the head | params |
| Darkside | `0x1403E9590` | with no part busy: its attack collision ends (`0x140179D90(+0x108, 10.0)`), `+0xCD0` = 1 (its state 2 then ends at once, `0x1403E9CB0`) | its code's |

## 3. At Combat = KH1 before this change (Checked (decomp) + the mod's code)

- (Before option (c).) **Card-break drops never happen.** Guard Armor, Trickmaster, Jafar (its card-break drop),
  Parasite Cage and Dragon Maleficent drop only from **state 13**, which only CBTL's **message 3**
  (a broken card) enters (the writers of `enemy+0x68` = `enemy+0x800`: only that case; text search
  of the decomp). At KH1 the Heartless bosses attack without cards (Stage 28's P1) and Sora has none
  (decision 29), so nothing is ever broken. **Owner question (§ 7).**
- **The other drops still happen**: Oogie's dice and Ursula's tentacles die to KH1 hits through
  Re:CoM's defeat path; Darkside's state 2 runs through P1; Jafar's Blizzard-on-guard path needs
  attribute 3 on the hit (Assumption that our KH1 Blizzard keeps Re:CoM's Blizzard attribute).
- **Pickup**: the prize add put the card into the battle revolver, which at KH1 is the empty one
  (0 cards + Reload, decision 29) with its HUD hidden; the use-card button is skipped at KH1, so the
  card sat there unplayable until the battle ended. CBTL's count stayed 1, so a second card was
  lost.

## 4. What is built (Combat = KH1, Sora's story; Riku's story and Combat = Re:CoM unchanged)

- **Drops: Re:CoM's own.** A wrapper on the prize factory `0x1401A89C0` (15 bytes checked) only
  logs a gimmick card's drop, in every mode.
- **Pickup: held on Re:CoM's own count.** In the prize add wrapper (`win::drops::prize_add`,
  installed by the drops or by the gimmick cards alone): for kinds 0x3A..0x41 with type 6, main
  Sora at KH1: Re:CoM's rule on CBTL `+0x1D0` (≤ 0 takes it, `0x14015D5B0(1)`; else lost, as the
  revolver drops it); the card is kept with its battle's CBTL; then Re:CoM's prize add runs **with
  `player+0x3F0` hidden for the call**, so its type 5 / 6 branch skips only the revolver add (no
  deck at KH1) and still runs the notice and the event variable. Held cards end with the battle as
  Re:CoM's: a new CBTL (or the count back at 0) forgets it.
- **Prompt "Stagger"**: our command `0xF0` (KH1's command table ends below 0x80; Checked (files)),
  word id 901 (ours, above KH1's `Word.bin` ids). Offered while held, in Re:CoM's battle room, and
  where Re:CoM's play would take it: no CBTL lock (`& 0xC0000000`), no card playing (`+0x1E4`), not
  after the win (`+0x428 & 4`) or `+0x7B0`, no event gate (flag (1, 8), mask (1, 0xE2) bit 2).
- **Priority** in the context tab: **an object's command (Open, Talk, Save, …) → Stagger → KH1's
  special offer (Sonic Blade)**. KH1 puts event / object commands in the context entry
  (`0x140293000`) and its special offer comes last, into an entry nothing took (`0x14028D0F0`);
  a held gimmick card is the boss fight's event command, so it goes before Sonic Blade. In battle
  rooms there are no objects, so in practice Stagger wins over Sonic Blade while a card is held.
- **Triangle** (`win::gimmick::use_card`, from the menu's step): the play's own writes in Re:CoM's
  order: `0x140161D60(&card, 1)` (the revolver play's type-6 step), `0x14015D5B0(-1)` (the used
  card's count), then **Re:CoM's card handler `0x1401F5580(Sora, 1, record)`** (CBTL's message 1;
  record = the card, no sleight). If the handler refused it (no `+0x428 & 0x200`), everything is
  undone and the card stays held. The request-bits wrapper then makes **Re:CoM's builder
  conversion** `+0x424 |= 0x40` (only when the KH1 Attack took the builder's field branch, which
  skips it). From there it is all Re:CoM's: its move exe starts action 0x19, the motion, message 6,
  the boss's own reaction, the card's end. A press while Sora is busy waits as a played Re:CoM
  card waits (the request stays until Sora is back in the move exe).
- **The word**: English "Stagger" in every language (no translation asked for). The asset step
  writes it into `kh1-command-menu.json` and bakes it in KH1's menu font for the KH1 look; without
  a re-run the DLL uses its built-in text in the Re:CoM look, and the KH1 look's tab shows no word.
- **Per frame**: two loads in the menu's world while no card is held; nothing else.
- **Logs** (`gimmick:` lines): `drop:` (kind, place, prize, receiver), `pick-up:` (held or lost),
  `prompt:` (shown / gone), `use:` (played, the handler's refusal, action 0x19 started, the boss took
  it, the action ended).

### The five card-break bosses (Stage after 108; owner, 2026-11-01: option (c))

At Combat = KH1, in Sora's story: when a **KH1 combo finisher** (a KH1 row whose attack record has
kind 5: the ground finisher, the air finisher, Blitz, Ripple Drive, Hurricane Blast) **interrupts
the attack** of Guard Armor, Trickmaster, Jafar, Parasite Cage or Dragon Maleficent, the boss gets
**Re:CoM's own card break**, exactly as the card-battle resolver gives it:

- **Where**: the KH1 enemy-hit wrapper (`win::attack::enemy_hit`), after Re:CoM's hit handler
  `0x14017F7E0`, calls `win::gimmick::after_kh1_hit`. "Interrupts": the boss was in an attack
  (`+0x260 & 1`, without the interrupt bit) so the wrapper set the interrupt bit `0x2000000` for
  the handler (decision 14), and after the handler its attack is over (`+0x260 & 1` cleared by
  `0x14017E1D0`). Not for a hit that kills it (Re:CoM's message handler ignores a dead enemy), not
  for spells, not for any other enemy. Only the five classes, by vtable, RTTI-checked at start:
  `CBossGuardarmor` `0x14065F4B8`, `CBossTrickMaster` `0x14066C5F8`, `CBossJafar` `0x14066E220`,
  `CBossParasiteCage` `0x140669220`, `CBossMDragon` `0x140674570`.
- **What**: Re:CoM's message post `0x1401D2770(CBTL +0x58 (its CMessageManager), delay 0, CBTL's
  handle +0x20, the boss's handle +0x20, message 3, no record)`: the same call the resolver
  `0x14015EAC0` makes for a broken card (its posts at `0x14015ED6D..`, delay 0, from CBTL's handle).
  The record is left out: nothing in the five bosses' state 13 (begin `+0x220`, exe `+0x228`, end
  `+0x230`) or `+0x418` reads the copy at `+0x318` (Checked (decomp)), and at KH1 CBTL's side-2
  record holds no card of theirs. Re:CoM's message manager then delivers it to the boss's handler
  `0x140181620` (all five use the base handler: Checked (exe)), whose **case 3** (`0x140181739`):
  unless `+0x260 & 4`, vtable `+0x418`; clears `+0x260` bits 0x201 and 0x10; ends its attack
  collision (`0x140179D90(+0x108, 10.0)`); next state = **state 13** (`enemy+0x800`). The boss's
  **state-13 begin** is the stagger with the drop roll (§ 1 table: Guard Armor `0x1403AB290`,
  Trickmaster `0x1403CB1C0`, Jafar `0x1403D0D80`, Parasite Cage `0x1403C2B70`, Dragon `0x1403E1690`):
  the chance, its conditions, the receiver and the prize are all its own.
- **Not done**: the resolver's other card-break work (the enemy card's UI, CBTL's slot, the clash
  effects) has no card at KH1 to act on; only the boss's message is sent.
- **Revenge (Stage 106)**: none of the five is a revenge boss (the revenge list holds the humanoid
  bosses only; Checked (test `the_five_card_break_bosses`)). Re:CoM's boss falloff and every other
  hit rule are unchanged.
- **Per frame**: nothing, except a few loads while a posted break is watched for its log line
  (≤ 120 frames).
- **Logs**: `break:` lines: the post (finisher row, boss, HP, handle), "entered its state 13", then
  the roll: "the card dropped" / "no card (its chance missed, or its own condition refused)" / "no
  state 13 within 120 frames" / "the boss is gone"; a finisher on an attacking boss that didn't
  break says why.
- Assumption: Re:CoM's message manager is stepped in KH1 battles as in card battles (CBTL's task;
  Stage 108's message 6 relies on it too); that bosses stagger "about as often" as in Re:CoM
  (not measured).

### More triggers for the five (owner, 2026-11-01: "interrupting an attack with a finisher is very rare")

Same five bosses, Combat = KH1, Sora's story; the finisher interrupt above stays as it is.

**(a) Guard.** When Sora's KH1 Guard blocks an attack of one of the five, the boss gets the same
card break (CBTL's message 3 posted on CBTL's queue, its own state 13 staggers it and rolls the
card).

- **Where**: the block is decided in `win::guard::on_result` (the pair-dispatch wrapper
  `0x14016EAD0`): Re:CoM's dispatch returned 2 on the player side's list (`*0x14087A838`), Sora's
  KH1 Guard is live, his body (`player+0x690`) has the guarded bit `+0x9C & 0x20` and `+0x98` = the
  attack's owner (decision 20; Checked (decomp) in `recom-kh1-attack.md`). That detection no longer
  needs the **Guard reactions** toggle: the toggle now only gates the reactions (flinch, turned-back
  shots). Right after it, `win::gimmick::on_guard_block(owner, collision)`.
- **What counts as "an attack of the boss"**: the blocked collision's owner is the boss itself
  (its melee `CAttackMotion` entries and any other collision of its own), **or** a Re:CoM shot
  (`CBullet` `0x140525430` / `CHomingBullet` `0x140525530`) whose shooter (`+0x238`) is the boss.
  So **blocked projectiles count**, as our Guard code blocks them. Not counted: shots of the bosses'
  own projectile classes (e.g. `CBossMDragonFiraBall`, Dragon's fire): their owner is that class,
  not a `CBullet`, and our Guard code doesn't trace them back (Assumption that the five use them
  for some attacks: not listed). A turned-back shot hitting the boss later is a hit, not a block:
  it doesn't break.
- **Once per blocked attack**: the collision and its hit serial (`+0x70`) of the last break are
  kept; the same pair again does nothing. Never a boss whose current (`+0x60`) or next (`+0x68`)
  state is state 13, a dead one (HP ≤ 0), or one whose posted break is still on its way.
- **No new per-frame code**: it runs only on a block.

**(d) HP thresholds.** Each of the five drops its card **without a stagger** when its HP falls to
75 %, 50 % or 25 % of its max HP.

- **Where**: the KH1 enemy-hit wrapper (`0x14017F7E0`), after every hit at Combat = KH1 (swings,
  spells, turned-back shots: every path through Re:CoM's hit handler, which applies the damage;
  Assumption that no other path lowers these bosses' HP): `win::gimmick::after_hp(enemy, hp before)`.
  HP is `+0x274`, the max Re:CoM's getter `0x14017F5A0` (Re:CoM's at Damage = Re:CoM, KH1's at
  Damage = KH1: our stats hook answers it).
- **Once per threshold per fight**, per boss handle (a new fight is a new handle). The first hit
  seen in a fight marks the thresholds it already was below as done (no card for HP lost before).
- **One card per hit**: a hit that crosses several thresholds drops one card and uses all of them
  (several cards at once could not be held: Re:CoM's one-at-a-time rule loses the second).
- **Not on the killing hit** (HP ≤ 0 after it).
- **Already held or on the floor: it still drops**, as the bosses' own drop code does (it never
  looks); Re:CoM's one-at-a-time rule then loses the second on pick-up.
- **The boss's own conditions** (its drop code's tests other than the chance) are kept; a threshold
  they refuse **waits** and drops on a later hit when they allow it:
  - Guard Armor (`0x1403AB290`): none.
  - Trickmaster (`0x1403CB1F4`): its table object (`+0xCE8`) has `+0x2C8 & 1`.
  - Jafar (`0x1403CFE29`): not in its state 15 (`+0xE5C` ≠ 0xF).
  - Parasite Cage (`0x1403C2BDB`): its card not used yet (`+0xCD4 & 2` clear): once the card has
    been used, it never drops again.
  - Dragon Maleficent (`0x1403E16BD`): no footing of its own out (`+0xD18` a live handle).
- **The spawn is the boss's own** (Checked (decomp + disasm + exe constants)): the receiver
  `0x140163740(boss handle)`, then Re:CoM's prize factory `0x1401A89C0(0x1401A8A80(), &desc, 0)` with
  the descriptor its drop code builds:

| Boss | Position | Angles (pitch, yaw, 0, 1) | Normalised (`0x140293F20`) | Speed | Flags `+0x2C` |
|---|---|---|---|---|---|
| Guard Armor | boss (`0x1401A49B0`) + y 5.0 | −70°, random × 360° | yes | 0.2 | 1 |
| Parasite Cage | boss + y 5.0 | −70°, random × 360° | yes | 0.2 | 1 |
| Trickmaster | boss + y 4.5 | −70°, random × 360° | yes | 0.2 | 0x10001 |
| Jafar | (0, 20, 0, 1) | (0, 0, 0, 1) | no | 0.2 | 0x10001 |
| Dragon Maleficent | boss + y 5.0 | −70°, atan2(Sora's point `0x1401F0AB0` − boss) + random in ±45° | yes | 0.2 | 0x10001 |

  (Random: Re:CoM's random float `0x14029CF00`; Dragon's spread is `0x1401A6530`'s formula
  r × |hi − lo| + lo on it. The chance itself is not rolled: a threshold always drops.)
- **Logs**: `gimmick: guard break:` (the boss, melee or shot, collision and serial, HP, handle, or
  why not), then the same `break:` watch lines; `gimmick: hp drop 75/50/25:` (crossed at HP / max,
  waits and why, made at where, prize).
- Assumption: Re:CoM's `atan2f` (`0x1404E5898`) and Rust's agree; the random draws match the boss's
  in kind, not in sequence (ours draw from the same generator at another time).

## 5. Sites (`sites::GIMMICK_*`, bytes from the Steam exe, pefile-checked)

- Hook: prize factory `0x1401A89C0` (moved here from `DROP_CALLEES`).
- Card break: called `0x1401D2770` (`GIMMICK_BREAK_CALLEES`); checked the enemy handler's case 3
  `0x140181739`, its table entry `0x1401819B0`, the next-state store `0x140181994`
  (`GIMMICK_BREAK_CHECKED`); the five classes' `+0x48` (= `0x140181620`) and `+0x220` (their drop
  begins) as relocated pointers (`GIMMICK_BREAK_PTRS`).
- Called: `0x140161D60`, `0x140161D30`, `0x14015D5B0`, `0x1401F5580` (+ `CPlayerSora` vtable
  `+0x48` = it, `0x1405A9980`); reused: `0x1401B0870`, `0x1401B08C0`.
- Checked: `0x140160760`, `0x1401616C0`, the handler's type 6 case `0x1401F58E8` and table
  `0x1401F5CEC`, the builder's conversion `0x1401F85F1`, the move exe's branch `0x1401EEE39`, the
  init `0x1401F25D0` and exe `0x1401ECA70`, the revolver play's step `0x14016D008` and event gates
  `0x14016CF25`, the prize add's type 5 / 6 branch `0x1401F4BDA`, the eight card-info rows
  (`0x140528C30 + kind × 16`).

- Triggers (a), (d): called `0x140163740`, `0x1401F0AB0` (`GIMMICK_HP_CALLEES`); checked the five
  drops' code (receiver, position, y offsets, speed, yaw, pitch, kinds, conditions, Jafar's
  descriptor, Dragon's spread) and their constants (`GIMMICK_HP_CHECKED`).

## 6. Checked / Assumption

- Checked (decomp + disasm + exe): every address and field above, the drop sites and handlers, the
  card kinds, one-at-a-time, no stocking, the play path, the move exe / builder / Battle exe
  dispatch; Checked (tests): the pure rules (`gimmick.rs`), the menu's priority, the word fallback,
  the sites' targets.
- Assumption: the visible meaning of each boss's reaction (§ 2 table); the chances read from
  `.EPD` parameters (files not in session); worlds by name; `0xFF0` is the card-raise motion; no
  Re:CoM code relies on the gimmick card sitting in the revolver at KH1 (the revolver add is the
  only step skipped); KH1 Blizzard keeps attribute 3 on Jafar's guarding body.
- Option (c): Checked (decomp + disasm + exe + tests): the resolver's post and its arguments, the
  enemy handler's case 3 → state 13, the five classes' handler and state-13 slots, kind-5 rows
  (rows 8, 16 finishers; 11, 13 not), no revenge overlap. Assumption: the message manager runs in
  KH1 battles; the stagger's visible feel.
- Triggers (a), (d): Checked (decomp + disasm + exe + tests): the block detection's place, the
  bullet shooter field, the five drop calls' arguments and conditions, the threshold rules.
  Assumption: the bosses' own projectile classes aren't counted as theirs; no HP path outside the
  hit handler; atan2 agreement.
- Not checked: in game.

## 7. The five card-break drops (answered: (c), built in § 4)

Guard Armor, Trickmaster, Jafar, Parasite Cage and Dragon Maleficent drop their card only when Sora
**breaks the boss's card** (state 13). At Combat = KH1 there are no card breaks, so these five never
drop (Jafar also has its Blizzard path). Options:

- **(a)** Roll Re:CoM's chance when a KH1 hit **interrupts the boss's attack** (the moment KH1's
  "card priority" ends its attack, `0x14017E1D0` in the hit handler; Re:CoM's card break is the
  same moment in a card battle). The boss keeps its KH1-style reaction (no stagger state); the
  drop code would be ours (the boss's state-13 begin also plays the stagger, so it can't be called
  alone): the same chance (literal or parameter), receiver and prize as the boss's own code.
- **(b)** At that moment the boss gets **Re:CoM's own card-break stagger (state 13)**, whose begin
  rolls the drop itself, all Re:CoM's code. Bosses would stagger much more often than in Re:CoM
  (every interrupted attack), which changes the fights.
- **(c)** Like (b), but only a **combo finisher** that interrupts the boss's attack breaks it (a
  closer rate to Re:CoM's card breaks; still Re:CoM's own stagger and drop code).

## 8. Owner test list

Combat = KH1, Sora's story, `kh1-recom.log` open:

1. Re-run the asset step (`kh1-recom-build`) for the word in the KH1 look (the Re:CoM look shows it
   either way).
2. **Oogie Boogie** (Halloween Town): break dice until a card drops (`gimmick: drop:` line), pick it
   up (`pick-up: … held`). The green tab shows **Stagger** (`prompt:` line). Triangle: Sora plays his
   card motion and Oogie goes down (`use:` lines: played, action 0x19 started, the boss took it,
   ended). The tab goes.
3. **Ursula** (Atlantica): the same with a tentacle's card.
4. **Darkside**: after its Heartless summon, kill the summoned Heartless; the card drops (100 %).
5. While holding one, pick up a second: lost (`pick-up: … lost`), as Re:CoM.
6. Hold one and win the battle without using it: next battle, no Stagger (`no longer held`).
7. Press Triangle on Stagger mid-combo or in the air: the card plays when Sora lands / finishes.
8. With Sonic Blade equipped and a card held: Stagger shows, not Sonic; after using the card, Sonic
   returns.
9. Combat = Re:CoM: the card goes into the deck as before (log: `drop: … Re:CoM's`), no Stagger.
   Riku's story: unchanged.
10. **Card break, option (c)**: Guard Armor (Traverse Town): hit it with a combo finisher while it
    attacks. It staggers (Re:CoM's card-break stagger) and the log shows `break: KH1 finisher row …
    interrupted CBossGuardarmor's attack`, `entered its state 13`, then the roll (30 %: a card
    drops about one break in three). Swings 1 / 2 during its attack, or a finisher while it isn't
    attacking: no stagger, no break line (or "no break" with the reason).
11. The same with Trickmaster, Jafar, Parasite Cage and Dragon Maleficent (their chances are their
    parameter files'; Parasite Cage stops dropping once its card was used; Dragon only with no
    platform of its own out).
12. A finisher that kills one of them: no break. Other bosses and Heartless: never a break line.
13. Stagger with a card from a break: the boss's own reaction (§ 2) as with the other cards.
14. **Guard break (a)**: Guard Armor: block its attack with Guard: it staggers at once (`gimmick:
    guard break: Guard blocked CBossGuardarmor (its attack …)`, then `entered its state 13` and the
    roll). Block again while it is staggered: no second break (`already in its card-break
    stagger`). With Config "Guard reactions" Off: still breaks.
15. A blocked shot of one of the five (a Re:CoM bullet): `guard break: … (its shot …)`.
16. **HP drops (d)**: Guard Armor: bring it under 75 %: a card drops without a stagger (`gimmick:
    hp drop 75: CBossGuardarmor at … HP`, then `made at …`); the same at 50 % and 25 %, once each.
    A big hit across two thresholds: one card (`hp drop 50/25`).
17. Parasite Cage after its card was used: thresholds log `waits: its card was used already`, no
    card. Trickmaster: a threshold may wait for its table (`waits: its table's +0x2C8 & 1 is
    clear`) and drop on a later hit.
18. Jafar's HP card appears at the arena's (0, 20, 0) and falls; Dragon's flies toward Sora.
19. Damage = KH1 (ini): the thresholds follow KH1's max HP.

## 9. HP drop at 50 % only (0.64.1)

Owner (2026-11-02): "only drop one at 50% and occasionally when guarding". Trigger (d) now has one
threshold, **50 %** of the boss's max HP (`gimmick::HP_THRESHOLDS = [50]`): one card, once per fight,
without a stagger, with the same waiting conditions and spawn as before. The Guard trigger (a) is
unchanged: each block posts the card-break and the boss's own drop roll decides (Guard Armor 30 %,
Parasite Cage 30 % until its card is used, the others their `.EPD` chance: Assumption). Test: bring a
boss from full to below half: one `gimmick: hp drop 50:` line and one card; none at 75 % or 25 %.
