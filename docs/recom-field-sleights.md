# Sleights outside battle (research, no code)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Customize page and long names), verbatim:
"lets sora use sleights outside of battle". Context: the field's SHORT CUT panel shows a bound
sleight (Homing Blizzara) greyed. The owner kept Re:CoM's battle transitions ("Battle transitions
stay Re:CoM's"), and Sora in the field already moves, swings and casts KH1's magic natively.

Steam addresses (image base 0x140000000). Tags: **Checked (decomp)** = read in the Re:CoM Steam
decomp this session; **Checked (exe)** = bytes / tables read from `recomsteam.exe` this session;
**Assumption** = inferred, not confirmed. Nothing here was tried in game.

## Short answer

- Re:CoM's sleight code (`initStockSkill` / `exeStockSkill`) has almost no battle-only parts. Four
  things stop it in the field:
  1. the Field exe has no case for action 0x16;
  2. the `StockSkillList` pointer is only set when a card battle is built;
  3. the sleight-pack loader only finishes loads in battle rooms;
  4. our own menu gate (`battle` only).
- KH1 magic already crossed the same line in the field. It uses a Field-exe case stub, Re:CoM's
  shots and collision, and Re:CoM's battle effect set loaded in the field. The items' case is
  built the same way.
- A sleight hit on a field Heartless is a plain Re:CoM field strike. What it does depends on the
  hit's kind:
  - most kinds give the strike pause, then the battle with the first-strike bonus;
  - Thunder Raid's kind paralyses and weakens the enemy without starting the battle;
  - Teleport's kind freezes it.
- The battle start puts Sora in Re:CoM's Wait state, so **a sleight is cut** when its first strike
  turns into the battle.

## 1. Field player vs battle player

| what | where | tag |
|---|---|---|
| Sora's player state machine picks **state 0 Field** (exe `0x1401FA0F0`) or **state 1 Battle** (exe `0x1401F9840`) by `player+0x390` (0 in field rooms). Sora's vtable `0x1405A9938`: `+0xA0` = Field exe, `+0xB8` = Battle exe. | `recom-player.md`; vtable | Checked (exe) |
| Both exes run the same frame set-up (dt from `+0x14`, `+0x464` = dt × 0.02), the request bits `0x1401F8150` and the damage check `0x1401E6DE0`. Then each switches on the action `+0x40C`. | `0x1401FA0F0`, `0x1401F9840` | Checked (decomp) |
| **Battle-only extras before the switch**: card-use input `0x1401E8FF0`; the sleight / card-break check `0x1401E7C20`. The Field exe calls neither. | | Checked (decomp) |
| **Battle exe cases**: 1–4, 6–9, 0xB (card attack), 0xC/0xD, 0xE, 0x12 (reload), **0x13 magic → vt `+0x150`**, **0x16 sleight → vt `+0x140`**, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C (win), 0x1D, 0x20–0x24. | `0x1401F9840` | Checked (decomp) |
| **Field exe cases**: 1–4, 0x1F, 5–9, **0xA (field swing)**, 0xC/0xD, 0x10, 0x11, 0x14, 0x15, 0x18, 0x1D, 0x1E. Its table `0x1401FA5F4` (index = action − 1, up to 0x1F) sends everything else to the default `0x1401FA599`. | switch `0x1401FA1B0` | Checked (exe: table dumped) |
| **Action 0x16's entry is `0x1401FA648` → `0x1401FA599`** (the default), so the Field exe never runs `exeStockSkill`. Re:CoM needs no case: a sleight comes only from a card play, and card plays exist only in card battles (`recom-player.md` § Card path). | `0x1401FA648` = `99 A5 1F 00` | Checked (exe); "only from a card play": Checked (decomp) for the request path, Assumption that nothing else sets 0x16 |
| Our patches already in this table: action **0x13** → cave `0x1401FB0E1` (magic, `crate::magic::field_case`); action **0xE** → cave `0x1401FAEA2` (items, `crate::items::field_case`). A free 15-byte int3 cave follows the Field-exe area at **`0x1401FA6C1`** (no site of ours uses it: grep). | | Checked (exe + code) |
| Sora's sleight functions: start `+0x130` = `0x1402247A0`, `initStockSkill` `+0x138` = `0x140220DF0`, `exeStockSkill` `+0x140` = `0x140218950`. | vtable | Checked (exe) |
| Built with the player in **both** room kinds (`0x1401F42B0`): camera `+0x3D8` (`CPlayerCamera`), target cursor `+0x3F8`, gauge `+0x3E0`. The escape gauge `+0x400` exists **only in battle**, and `exeStockSkill` never reads it. | `0x1401F42B0` | Checked (decomp) |
| `PLM::CPlayerManager` (`player+0x3B8`, the sleight-resource owner): built for the field too (`CGameSubField`'s state at `0x1401AFDA0`) as well as for battle (`0x1401AEBF0`). | `0x14020E1E0` | Checked (decomp) |

## 2. How KH1 magic already runs in the field

These are all native pieces, already built (`recom-m7-magic.md`):

- **Stepping.** The Field exe's 0x13 entry points at a 13-byte stub (`mov rcx,rbx; call exeMagic
  0x140217EE0; jmp 0x1401FA599`). Our `exeMagic` hook runs KH1's frame on Re:CoM's magic action.
  At Combat = Re:CoM the case can't be reached, because nothing sets 0x13 in the field.
- **The spell.** Re:CoM's shot (`CHomingBullet` and the other shot classes) is made through its
  descriptor `0x140156790`, entity factory `0x1401A8A80` and shot manager `0x140158DD0`. Re:CoM's
  actor and collision systems step it, draw it, collide it and end it, in the field exactly as in
  battle.
- **Effects.** Re:CoM's battle effect set (PLAYER group 1001) is loaded at each field room load
  at Combat = KH1 (castfix: `0x1401AF933` → `field_load` → `0x140211960(0)`). It is kept when the
  camp opens.
- **Keyblade.** Re:CoM's own draw `0x1401F5D30(Sora, 1)` at the command.
- **What a hit does.** The field Heartless's own hit handler decides (§ 4). Our KH1 rows carry
  Re:CoM kind 1 (`magic.rs` row builders), which is Re:CoM's field strike, so the battle starts.
- The same path suits sleights: one more Field-exe case, and Re:CoM's own `exeStockSkill` does the
  rest. Our existing `exeStockSkill` wrapper (for the KH1 specials) passes everything else
  through.

## 3. What a sleight needs at run time, and whether the field has it

| need | battle | field | tag |
|---|---|---|---|
| **A case for action 0x16** | Battle exe → vt `+0x140` | **missing** (§ 1) | Checked (exe) |
| **`StockSkillList` pointer `*0x14087A810`**, read by `initStockSkill` (AP, attribute, status) and 34× in `exeStockSkill` (category `+9` etc.) | set by the `CCardBattle` ctor `0x14015CE80`: `0x1402A43E0(1, "StockSkillList")` → `0x1402A49C0` | **null until the session's first battle.** The only writer is that ctor; its dtor clears `0x14087A818` (CBTL) but not this pointer, so it is kept but possibly stale after a battle. A field start before any battle would **read address 0 + offset: crash**. | Checked (decomp: the only writer) |
| The list's data itself: `SY0002.BIN` = SYS group 2 (SYS table `0x140804110`), linked by Sora's PLAYER group 1 (LINK `10002`, PLAYER table `0x1408055D0`) | loaded | loaded with Sora, so Re:CoM's own lookup would find it | Checked (exe tables); that a LINK row keeps it resident in field rooms: **Assumption** |
| **Sleight packs** (motions `.ANN`, effects `.EET`, sounds `.SES` in PLAYER/10xx, e.g. Firaga Burst 0x20 → pack 0xC → group 1032): Re:CoM's load `0x140211790(PLM, id)` marks a slot "queued" (1) and queues the file. The PLM step `0x140210A10` marks it "loaded" (2) only when `PLM+0x88 & 1` is set. `exeStockSkill` waits on `0x1402106E0(PLM, id) == 1` (sub-state 4) before the sleight's motion. | the PLM init `0x140210100` sets `+0x88 \|= 1` and the ring fields `+0x98` (cursor) / `+0xA0` (end) / `+0xA8` (space) **only when** `0x14087B858 == 1` (in battle) **or** the room is a minigame room (`0x1402102A8..0x1402102B8`) | **bit 0 clear, ring fields 0**: a pack never finishes loading, so a pack sleight **hangs in sub-state 4** (Sora held in motion 0x2E) | Checked (decomp + disasm) |
| The ring memory `*0x14087B6C0` (2 / 2.5 MiB, `0x140210BD0`) | allocated | **also allocated in the field**: `CGameSubField`'s state 0 `0x1401AF810` calls `0x140210BD0`, except in world 10 (100 Acre Wood) and rooms 2000–2999. Freed at scene changes (`0x140211610` from `0x1401AA000` / `0x1401A9D20`) and with `CGameMain`. | Checked (decomp); what rooms 2000–2999 are: **Assumption** (not read) |
| Which of the 27 need a pack | | **No pack** (`0x14020F720` returns 0): Fire / Blizzard / Thunder / Reflect Raid, Judgment, Homing Blizzara, Warpinator, Warp, Teleport. **Pack**: the other 16 magic sleights, plus Idyll Romp (PLAYER/0303) and Cross-slash+ (0307). Their rows carry flag `0x20` (wait for the pack). | Checked (decomp `0x14020F720`; exe rows) |
| Effects by name (`0x1401A2D50` / `0x1401A6B20` → `0x1402A4920`): "start", "limit1..3", "summon", the row's effect set | the battle set + the pack's `.EET` | the battle set is in the field at KH1 (castfix); the pack effects come with the pack (above) | Checked (decomp); which names live in which pack: **Assumption** |
| Sora's camera `+0x3D8` (sleight camera modes `0x1401FBDE0`), target cursor `+0x3F8`, homing target `0x1401F68A0` / `+0x84C` | yes | yes (built in both) | Checked (decomp); that the field cursor picks field Heartless as targets: **Assumption** |
| Shots / collision (`0x140156790`, `0x140158DD0`, the hit pass) | yes | yes (KH1 Fire uses them in the field) | Checked (decomp); in game: Fire in the field drew and flew (owner's 0.45.0 log); a field Fire **hit** on a Heartless: not seen in any trace |
| The room dim `0x140232170` (mask 0x1800: `CMap` / `CSky`) | yes | the field has a map and sky too | **Assumption** (field map class not read) |
| The active-card pointer `player+0x3E8` | battle | **read without a null test only in the friend branch** (`exeStockSkill` category 5: `+0x3E8 → +0x1E8`). A friend sleight in the field would crash. Friend sleights wait for the party anyway. | Checked (decomp) |
| The battle enemy manager (task 0x19) | yes | absent; `exeStockSkill`'s one use (a hit count, 2..16) tests for null and falls back to 2 | Checked (decomp) |
| The end action `0x1401F6660` | yes | handles the field (its battle-only cursor work is behind `+0x390 != 0`; `0x1401F8E60` the same) | Checked (decomp) |
| The sleight start `0x1402247A0` | | no card is read. Its use counter `0x14020E590` adds 1 to the save variable (0, id + 0x44), up to 9999, so a field use would count. | Checked (decomp); what reads that counter (journal, trophy): **Assumption** |
| Ticks | 50 / frame (dt `+0x14`) | the same code in both exes | Checked (decomp) |

## 4. What a sleight hit on a field Heartless does

Field Heartless are `CMapEnemy` (vtable `0x14055ACB0`; states at `+0x280 + 0x40 × i`, handlers
vtable `+0x98 + 0x18 × i`). They never lose HP in the field. Their hit handler `0x14019E020`
switches on the hit's kind (`body+0x80`). Hits are ignored while the enemy is already in state 4
or 8. Checked (decomp + exe).

| hit kind | state | what Re:CoM does | battle it gives (`0x14019BED0` → type, applied by `0x1401802A0`) |
|---|---|---|---|
| **any other** (1, 2, 3, …) | **4** `0x14019B4B0` / `0x14019E8F0` | The **field strike**: plays its hit motion (`@ENP +0x5C` / `+0x64`) and waits half its length (3000 ticks = 60 frames without an `@ENP`). Pause mode 4 (`0x140232260`): map enemies (update group 2) freeze, Sora (group 1) keeps moving. Game flag (1, 5) set; its hurt bodies off; Re:CoM's field-strike counter (0, 0xB8) + 1 (trophy 0x28 at 150). Then the encounter. | **type 2**: HP × 0.9 (× 0.2 with room card 0xB1), a card-drop roll `0x14017E190`, enemy flag `+0x260 \| 8` |
| 0x1C–0x1E, 0x24–0x26 (Re:CoM's Thunder card rows 0x24 / 0x2E / 0x2F carry 0x1C / 0x1D / 0x1E) | **5** `0x14019B6A0` / `0x14019E9E0` | "abnormal" effect, held for `@ENP`-timed steps, then back to roaming. **No battle.** Flags 0x10, and 0x40 for 0x1C–0x1E / 0x26. | on contact later: **type 3** (HP halved, then × 0.9) |
| 8 (Re:CoM's Stop rows 0x27 / 0x3A / 0x3B) | **6** `0x14019B960` / `0x14019EC20` | frozen for its `@ENP` time, flag 0x20, **no battle** | on contact: **type 4**: the enemy starts the battle stopped, 12000 ticks (4 s) |
| 7, 0x2A (Stun Impact's row 0xB) | **7** `0x14019B3F0` / `0x14019E8B0` | stunned (its stun motion), flag 0x80, **no battle** | on contact: **type 6** (a battle flag) |
| 0x29 | — | ignored | — |

What each type means in play beyond these writes (enemy flag 8, the type-6 flag) is an
**Assumption**. Re:CoM itself never casts in the field. States 5–7 are Re:CoM's own reactions to
its Thunder, Stop and Stun kinds, and no vanilla field attack produces them (Assumption: no field
source found).

**The encounter cuts Sora's action.** `0x14019C210` checks Sora isn't dead (`player+0x7B0`),
writes the battle record (`0x1401A0060`) and requests the battle (`0x1402368C0`). Then
`0x1401EA290` sets `+0x418 |= 0x1101` and sends Sora to player state 2, **Wait**, whose exe
`0x1401FAB10` is empty. The field is then torn down, and the battle room builds a new player.
So a field sleight stops at the encounter: about half the struck enemy's hit motion after its
first strike. Re:CoM's end action never runs for it. Checked (decomp).

**Contact.** A field Heartless that touches Sora starts a plain battle (type 0, no bonus)
whenever pause mode 4 is off (`0x140231DE0 & 4`). Sora's protection bits don't stop it. A
sleight that dashes Sora into an enemy can touch before its first hit lands. Checked (decomp).

**Our KH1 field hits today**:

- KH1 swings run on Re:CoM's field swing (action 0xA, collision `0x1401F5DC0`) with Re:CoM kinds
  1 / 2 (`recom-attack-design.md` § 6).
- Our KH1 spell rows write kind 1 (`magic.rs`: Fire, Blizzard, Thunder, Gravity…).
- So every KH1 field hit is state 4, the battle with first strike (owner decision 3), natively.
- Our KH1 Thunder and Stop do **not** use Re:CoM's latent states 5 / 6 (see § Found).

**Each sleight's hit kind** (its attack row `+0x638` from `initStockSkill`; table `0x1407C6C70`,
kind at `+0xB`). Checked (exe + decomp). That every hit of the sleight carries its row's kind is an
**Assumption**: some sleights build shots or status kinds from `+0x668` / `0x1401E66A0`.

| id | sleight | row | kind | field state | pack |
|---|---|---|---|---|---|
| 0x1B | Fire Raid | 0x30 | 1 | 4 | no |
| 0x1C | Blizzard Raid | 0x31 | 1 | 4 | no |
| 0x1D | Thunder Raid | 0x32 | **0x1D** | **5** (paralyse, no battle) | no |
| 0x1E | Reflect Raid | 0x34 | 1 | 4 | no |
| 0x1F | Judgment | 0x33 | 1 | 4 | no |
| 0x20 | Firaga Burst | 0x45 | 3 | 4 | yes |
| 0x21 | Raging Storm | 0x47 | 3 | 4 | yes |
| 0x22 | Mega Flare | 0x48 | 3 | 4 | yes |
| 0x23 | Freeze | 0x49 | 0xC | 4 | yes |
| 0x24 | Homing Blizzara | 0x2D | 2 | 4 | no |
| 0x25 | Aqua Splash | 0x4A | 2 | 4 | yes |
| 0x26 | Magnet Spiral | 0x4B | 0x14 | 4 | yes |
| 0x27 | Lethal Frame | 0x4D | 0x17 | 4 | yes |
| 0x28 | Shock Impact | 0x50 | 0x10 | 4 | yes |
| 0x29 | Tornado | 0x51 | 0x11 | 4 | yes |
| 0x2A | Quake | 0x52 | 0x12 | 4 | yes |
| 0x2B | Warpinator | 0x3E | 0xE | 4 | no |
| 0x2C | Warp | 0x3F | 0xF | 4 | no |
| 0x2D | Bind | 0x40 | 9 | 4 | yes |
| 0x2E | Confuse | 0x41 | 0xA | 4 | yes |
| 0x2F | Terror | 0x42 | 0xB | 4 | yes |
| 0x30 | Synchro | 0x43 | 0x13 | 4 | yes |
| 0x31 | Gifted Miracle | 0x53 | 0x19 | (heals Sora; enemy hits: Assumption, none) | yes |
| 0x32 | Teleport | 0x44 | **8** | **6** (freeze, no battle) | no |
| 0x33 | Holy | 0x54 | 3 | 4 | yes |
| 0x3A | Idyll Romp | summon branch (`CSummonBambi`) | — | the actor's hits: not read | yes |
| 0x43 | Cross-slash+ | summon branch (`CSummonCloud`) | — | the actor's hits: not read | yes |

In the field, the battle-only effects of Warp, Synchro, Quake, Bind and the others don't exist:
Re:CoM's battle-enemy handler `0x14017DC50` isn't the field Heartless's. There they are only a
first strike.

## 5. Which sleights make sense in the field

- **Healing**: Gifted Miracle (heals Sora). Re:CoM's heal `0x1401E59A0` works outside battle
  (Cure in the field uses it). Assumption that Gifted Miracle uses the same call.
- **Movement sleights**: none among the 27. Warp, Warpinator and Teleport act on enemies, not on
  Sora (`abilities.rs` help lines, `recom-m12-sleights.md` § 8.4.1).
- **Attack sleights as a battle opener**: 23 of them give Re:CoM's field strike, so the battle
  starts with the first-strike bonus. Thunder Raid paralyses and Teleport freezes without a battle.
- **Summon-type** (Idyll Romp, Cross-slash+) and the LV3 summons: they spawn `CSummon*` actors.
  Whether those actors find field Heartless is not read. The friend sleights would crash in the
  field (§ 3) and wait for the party anyway.
- **KH1 reference**: KH1 has no separate battle rooms. Its magic and specials work wherever Sora
  is; only its summons need battle mode (`0x140288F30`, `0x142867364 & 1`; from
  `recom-summons.md`, not re-read this session). KH1 has no Re:CoM sleights, so there is no KH1
  number to match. Re:CoM's sleights stay Re:CoM's own motion, timing and effects.

## Options

Every option shares the same plumbing, all Re:CoM's own code:

- **Field case 0x16**: the Field exe's entry `0x1401FA648` points at a 13-byte stub in the cave
  `0x1401FA6C1`: `mov rcx,rbx; call 0x140218950; jmp 0x1401FA599`, as magic's and items' cases.
  It can't be reached at Combat = Re:CoM (no field card plays).
- **`StockSkillList`**: when `*0x14087A810` is null at a field start, Re:CoM's own lookup
  (`0x1402A43E0(1, "StockSkillList")` → `0x1402A49C0`, the `CCardBattle` ctor's two calls) fills
  it once.
- **Sleight packs in the field**: a wrapper on the PLM init `0x140210100` (first bytes
  `48 89 5C 24 18 56 57 41 56 48 83 EC 20`). It applies only for the field PLM, at Combat = KH1 in
  Sora's story, and never in world 10 or rooms 2000–2999. It does the battle branch's own writes:
  `+0x88 |= 1`, `+0x98` = the ring, `+0xA0` = ring + size, `+0xA8` = size. The ring is already
  allocated there. Once per field room build.
- **Our gate**: `summons::Gate.battle` becomes "battle room, or a field room at KH1" for the
  Sleights list and its shortcuts; `sleight_running` counts field 0x16. MP: the owner's tiers.

### (a) Sleights in the field; a hit is Re:CoM's field strike (recommended)

- **In play.** The Sleights list and the SHORT CUT slots work in the field. Each sleight plays
  exactly as in battle: Re:CoM's motion, effects, dim and sounds, with MP by tier.
  - Gifted Miracle heals.
  - Most attack sleights hit a field Heartless with Re:CoM's field strike. The other field
    Heartless freeze, Sora keeps moving for about a second, then the battle starts with the
    first-strike bonus.
  - The sleight stops there: MP is spent, and the battle starts normally.
  - Thunder Raid paralyses and weakens the Heartless, and Teleport freezes it, without a battle.
    Touching it later starts the battle with Re:CoM's halved HP, or with the enemy stopped for 4 s.
  - A miss just plays out.
- **Re:CoM systems added to**: the Field exe (a new case), the PLM (field pack loading), the
  `StockSkillList` pointer (Re:CoM's own lookup). Our menu gate.
- **Risk**:
  - **The dim and protection bits of a cut sleight.** The end action never runs. Re:CoM's
    colour slot may stay grey into the battle room (the battle player is new; the global colour
    slot `[0x14087B7F0]` is not). This needs an in-game look. The fix would be Re:CoM's own fade
    back to white (`0x140232170(0x1808, …)`, the end action's call) at the encounter.
  - Dashing sleights touching first (type 0, no bonus): the same race KH1 swings have today.
  - Room-load time with packs queued in the field. The PLM dtors (`0x14020E240` /
    `0x14020E3F0`) release them at teardown (Checked (decomp)). Re:CoM's ring free at scene changes while a field PLM
    points at it: **Assumption** that the PLM is always destroyed first, as in battle.
  - The sleight use counter counts field uses.
  - The camp opening mid-sleight: Re:CoM's camp gate `0x1401A96C0` was not read for action 0x16.
- **Per frame**: nothing new. The stub runs only during action 0x16 (Re:CoM's own exe and our
  existing pass-through wrapper). The PLM wrapper runs once per room build, the pointer fill once,
  and the gate is one compare in the menu build that already runs.
- **Unknown**: whether SY0002 stays at the same address across field/battle (the pointer). Each
  sleight's real hit kinds. Whether field Heartless are targets for homing sleights (Homing
  Blizzara's turn). The summon-type sleights' actors in the field. How long a pack takes to load
  mid-field.
- **Staging** (one system per version): (1) the case, the pointer, the gate and the 9 pack-less
  sleights; (2) the PLM field gate for the 16 pack sleights; (3) Gifted Miracle's and Thunder
  Raid's / Teleport's checks in game.

### (b) As (a), and a sleight that starts the battle carries into it

- **In play.** As (a), but when a field sleight's strike starts the battle, Sora opens the battle
  room by performing the same sleight from its start, not paid again. The first-strike bonus still
  applies. The battle opens with the sleight.
- **Re:CoM systems added to**: (a)'s, plus a pending-opener byte that our battle-entry hook reads.
  At the battle player's first frame, Re:CoM's sleight start `0x1402247A0` runs for that id.
- **Risk**: the battle intro (Re:CoM's encounter effect and the enemies' entrance) overlaps the
  sleight. First-strike stuns plus a full sleight make field openers very strong. Pack reload in
  the battle ring (freed at the scene change) may delay the start. Bosses are unaffected, because
  scripted fights don't come from field strikes (Assumption).
- **Per frame**: (a)'s, plus one test at battle start.
- **Unknown**: whether Re:CoM's battle player accepts a sleight before its intro ends. That KH1
  has no equivalent makes this a pure design choice.

### (c) As (a), but field sleight hits never start the battle at once

- **In play.** A sleight plays to its end in the field. Its hits put field Heartless into Re:CoM's
  own field reactions instead of the strike: paralysed (state 5), frozen (6) or stunned (7). The
  battle starts when Sora touches one or swings at it, with that reaction's Re:CoM bonus (halved
  HP, enemy stopped 4 s, or the stun flag).
- **Re:CoM systems added to**: (a)'s, plus a wrapper on the field Heartless's hit handler
  `0x14019E020`. For hits whose owner is Sora's running field sleight (the owner test of
  `win::summons::owner_of`), it picks Re:CoM's state 5, 6 or 7 by a per-sleight table instead of
  state 4.
- **Risk**: it changes Re:CoM's own field-strike rule for sleights, which would need a Config
  toggle (a feel change). The halved-HP bonus (state 5 → type 3: HP × 0.5 × 0.9) is much bigger
  than a strike's × 0.9. The "abnormal" effect those states play may not be loaded for every field Heartless
  (Assumption).
- **Per frame**: one owner test per hit on a field Heartless while a field sleight runs.
- **Unknown**: the per-sleight table (a design choice); state 5's hold time per enemy (`@ENP`).

## Recommendation

**(a).** It is the path KH1 magic and items already use in the field, and every piece is Re:CoM's:
its exe, its pack loader and its field-strike rules. It adds nothing per frame, and it keeps
decision 3 (a KH1-side field hit is Re:CoM's first strike) for sleights too. Its one real cost (the
sleight cut at the battle start, MP spent) matches what a field Fire does today. (b) and (c) can
come later on top of it if the cut feels bad in play.

## Open questions for the owner

1. **Which option**: (a), (b) or (c)?
2. **Which sleights** in the field: all 25 magic sleights, or a subset (e.g. Gifted Miracle only,
   or no Thunder Raid / Teleport freezes)? Idyll Romp and Cross-slash+ (summon actors in the
   field: untested)?
3. **MP in the field**: the same tiers as in battle, or free / cheaper outside battle? (MP refills
   at save points: decision 35.)
4. **What a hit does** (only if not (c)): keep Re:CoM's own kinds (Thunder Raid paralyses and
   Teleport freezes without a battle), or make every sleight hit a plain field strike?
5. **The KH1 specials** (Sonic Blade, Strike Raid, Ars Arcanum, Ragnarok): the owner said Sonic
   Blade is "only in battle" (2026-11-01). They run on the same action 0x16, so the new case would
   allow them. Keep them battle-only, or let them follow?
6. **The LV3 summons** in the field (KH1's own rule needs battle mode)?

## Found while researching (not asked; ask before adding)

- **Re:CoM's latent field reactions to magic** (`0x14019E020`): its Thunder kinds paralyse a field
  Heartless (state 5, halved HP at the battle), its Stop kind freezes it (state 6, stopped 4 s at
  the battle start), and Stun Impact's kind stuns it (state 7). Re:CoM never casts in the field, so
  vanilla never uses them.
  - Our KH1 Thunder and Stop in the field send kind 1 (a strike, battle at once).
  - Whether KH1 field Thunder / Stop should use these: it changes how field magic plays
    (owner's call).
  - KH1 itself has no field/battle split, so KH1 gives no answer.

## 1:1 checks

- **Checked**:
  - the decomp, for the Field / Battle exe switches, `initStockSkill`'s battle reads, the PLM
    gate, the ring allocation, the field Heartless's hit states, the encounter and Sora's Wait
    state;
  - the exe, for Sora's vtable slots, the Field table entries, the free cave, the sleight rows'
    kinds and the resource tables.
- **Not checked**:
  - anything in game: no trace of a field strike by a KH1 spell yet, and no sleight run in the
    field;
  - `SY0002`'s residency across rooms;
  - the field colour slot after a cut sleight;
  - KH1's battle-mode rule (taken from `recom-summons.md`, not re-read).
- **No KH1 numbers** are involved: the sleights stay Re:CoM's own (owner's M12 decisions).
