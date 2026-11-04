# Sleights outside battle (research; option (a) built in 0.110.0: § Built)

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
| Our patches already in this table: action **0x13** → cave `0x1401FB0E1` (magic, `crate::magic::field_case`); action **0xE** → cave `0x1401FAEA2` (items, `crate::items::field_case`). A free 15-byte int3 cave follows the Field-exe area at **`0x1401FA6C1`** (no site of ours uses it: grep). **Correction (0.110.0)**: the Attack's Battle case uses it (`ATTACK_DATA`); the build uses `0x1401FB7B2` (§ Built). | | Checked (exe + code) |
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

## Built (0.110.0, Stage 222): option (a), both steps at once

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Sleights outside battle): **(a) Plain field
strike**; **free in the field**; **Re:CoM's special field reactions kept, for sleights and spells**
(KH1 Thunder and Stop in the field get Re:CoM's paralyse / freeze); **which**: all sleights, the KH1
specials (this lifts "only in battle") and the LV3 summons. Combat = KH1 in Sora's story only.
Code: `crates/kh1-recom/src/field_sleights.rs` (pure, tested), `src/win/field_sleights.rs`, the gate
in `sleights.rs` / `summons.rs` / `command_menu.rs`, the starts in `win/summons.rs` /
`win/specials.rs`, the field strikes in `win/magic.rs`. Shipped as **one version** (the 9 pack-less
sleights and the pack ones together): the pack part only acts when a pack sleight is chosen in the
field, and without its hook the pack sleights simply stay greyed.

### What runs

| piece | what | where | tag |
|---|---|---|---|
| **Field case 0x16** | The Field exe's table entry `0x1401FA648` → a 13-byte stub at **`0x1401FB7B2`**: `mov rcx,rbx; call 0x140218950 (exeStockSkill); jmp 0x1401FA599`. The cave the research named, `0x1401FA6C1`, **is not free**: the Attack's Battle case (`attack::battle_case`, `ATTACK_DATA`) has used it since M4 (the research's grep missed `ATTACK_DATA`'s entry). `0x1401FB7B2` (14 × int3 after `pop r14; ret`, before `0x1401FB7C0`; no branch into it: exe scan) was M9 9c.1's first Field case, unpatched since 9c.1's "Only in battle". The direct call reaches our `exeStockSkill` hook (Re:CoM's sleights pass through; a KH1 special runs its KH1 frame). Patched at start, as the magic / items cases; nothing sets 0x16 in the field at Combat = Re:CoM. | `sites::FIELD_SLEIGHT_DATA` | Checked (exe + decomp) |
| **List pointers** | At each field start (`win::field_sleights::prepare`, from the summons' / Sleights list's / specials' start): `0x1402A43E0(1, "BattleCardList", 0)` → `0x1402A49C0` into `*0x14087A808`, the same for `"StockSkillList"` into `*0x14087A810` (the `CCardBattle` ctor's own two calls, `0x140160830..`). Written every field start (a stale pointer after a battle gets a fresh value; the next battle's ctor writes them again). Not found: the start is refused (logged). BattleCardList too: the summon actors' power pick `0x140226BB0` and the magic sleights' shot code read it (no reader tests either for null). | `field_sleights::lists` | Checked (decomp: every reader); that both lists stay loaded in field rooms: **Assumption** (the log says if not) |
| **Packs in field rooms** | A wrapper on the PLM init `0x140210100` (13 bytes checked; once per PLM = once per room build): after Re:CoM's init, for a PLM whose `+0x88 & 1` Re:CoM left clear, at Combat = KH1 in Sora's story, where the field's own ring condition holds (world ≠ 10, not rooms 2000–2999: `0x1402339C0(1)`, `0x140235240()`, as `0x1401AF841..`) and the ring exists (`*0x14087B6C0 > 0`): the battle branch's writes (`+0x88 \|= 1`, `+0x98` ring, `+0xA0` ring + `*0x1407C7AE8`, `+0xA8` size, each slot's `+8` = `+0xA0`). Not copied: `+0x58` = `0x1401607A0()` (it **builds a `CCardBattle`** if none; nothing reads PLM `+0x58`) and `0x14087B6B0` = 1 (read only by `0x14020E6F0`, a battle start position rule). | `field_sleights::packs`, `sites::FIELD_SLEIGHT_HOOKS` / `_CHECKED` | Checked (decomp + disasm) |
| **Gate** | `summons::Gate` gains `field` (a field room where sleights run: the case in, the main player the KH1 `CPlayerSora`, `player+0x390` = 0) and `packs` (this room's PLM `+0x88 & 1`). `field_sleights::room_ok`: battle, or a field room for the 27 sleight abilities and the listed specials (not friend sleights: `exeStockSkill`'s friend branch reads `player+0x3E8` without a null test), a pack one only with `packs`. Summons: field + packs (every LV3 has a pack). **No MP in a field room** (`field_sleights::free`): the lists, the shortcuts, the special offer and the starts skip it. | `sleights::usable` / `special_usable`, `summons::usable`, `win::field_sleights::gate` | Checked (tests) |
| **Which need a pack** | `field_sleights::pack` = `0x14020F720`'s switch. Without packs (100 Acre Wood, rooms 2000–2999): Fire / Blizzard / Thunder / Reflect Raid, Judgment, Homing Blizzara, Warpinator, Warp, Teleport and Strike Raid; greyed there: the other 18 sleights, Sonic Blade (pack 2), Ars Arcanum (3), Ragnarok (4), every summon. | | Checked (decomp) |
| **Specials** | The offer (Triangle) and the Sleights list's specials: a field room too; their target may be a field Heartless (`win::magic::is_target_enemy`: Re:CoM's `CMapEnemy`, type 0x1002, lockable `+0x154 & 0x10000`, as its field lock-on `0x1401CD810` takes them); free. `start_special` through the same room check; Ragnarok's shots home on field Heartless too. | `win::command_menu`, `win::specials` | Checked (code) |
| **Preloads** | The Summon / Sleights list's cursor preload and the shortcut list's: in battle, or a field room whose PLM loads packs (never a load where Re:CoM's step can't finish it). The "already asked" mark is the room's PLM. | `win::summons::preload_room` | Checked (code) |
| **Running marks** | `sleight_running` and the summon's `out` no longer need the battle room (action 0x16, the player). At each battle start (the battle manager ctor's existing wrapper) the specials' running player and the summons' / Sleights list's marks are dropped. | | Checked (code) |

### What a field hit does (all Re:CoM's)

- A sleight's, special's or summon's hit on a field Heartless goes through Re:CoM's own collision,
  and its handler `0x14019E020` decides by the hit's kind (§ 4): most give the field strike (pause,
  then the battle with the first-strike bonus); Thunder Raid (kind 0x1D) paralyses; Teleport (8)
  freezes. Nothing of ours in it.
- **KH1 Thunder and Stop in the field** (owner: Re:CoM's paralyse / freeze, "instead of starting the
  battle at once"). Finding: **before 0.110.0 they did not touch field Heartless at all**. Their
  strike lists and targets took only Re:CoM's battle enemies (type 0x1004 with HP,
  `win::magic::alive_enemy` / `lock_on_list`); a field Heartless's type is **0x1002**
  (`0x14019D050`, Checked (exe)), and the bolts' and Stop shots' own collision is held off (their one
  hit is our direct strike). So nothing started a battle from them; Fire, Blizzard and Aero did
  (their shots collide through Re:CoM's pass with the KH1 rows' kind 1). Built: in a field room
  their targets and strike lists also take field Heartless (`target_enemy`, `lock_on_targets`), each
  struck through Re:CoM's contact and apply (`strike_body`, the battle path) with **Re:CoM's own kind**
  (`crate::magic::field_kind`): Thunder / Thundara / Thundaga the Re:CoM Thunder rows' 0x1C / 0x1D /
  0x1E (→ state 5, paralysed, no battle), Stop's 8 (→ state 6, frozen, no battle). Battle enemies keep
  the KH1 rows' kinds (map enemies exist only in field rooms). Checked (decomp + exe rows + code); in
  game: not seen.
- Found, not changed (not asked): **KH1 Gravity in the field hits nothing either** (the same
  battle-only lists). Re:CoM's Gravity kind 0xD would be the plain field strike (state 4).

### The battle start cutting a sleight

- **The room dim is cleared by Re:CoM itself.** The encounter sends the field Sora to its Wait state
  (`0x1401EA290`: `+0x418 |= 0x1101`, player state 2; the action stays 0x16), then the field is torn
  down. Re:CoM's player dtor `0x1401E4EB0` has a dim exit (`0x1401E51AD`): with `+0x418` bit 31 set it
  clears the bit and fades colour slot 0 (mask 0x1808) back to white **in 0 frames**
  (`0x140232170(0x1808, 0.0, white, 0, 0)`). Checked (decomp + exe; site "player dtor dim exit"). No
  fix needed; nothing of ours. (The colour slots are global and survive scenes, `0x14029D4B0` /
  `0x140231CF0`; `CGameMain`'s main state 6 `0x1401A9FC0`, the one the field-running state 5 switches
  to, does not reset them itself, unlike state 7 `0x1401ACAA0`. Checked (decomp); that state 6 is the
  battle entry's path: **Assumption**.)
- **The sleight's i-frames end**: Re:CoM's protection bits (`+0x418` bits 0, 0x1000, …) are the field
  player's, which is destroyed; the battle builds a new player. Ours: the dim shield is battle-only
  and per player and action; the summon invincibility per player and action 0x16; and the battle
  start drops our running marks (above). Checked (code); that the field player's dtor runs before
  the battle's player exists: **Assumption** (`recom-battle-entry.md`: the battle room loads after
  the field's objects are destroyed).
- The cut's log line: `field sleights: <name> (<id>): cut by the battle start …` with slot 0's mask
  at the battle start (0 = back to white).

### Summons, Idyll Romp and Cross-slash+ in the field

Built (free, where the packs load). Static scan of the summon code (`0x140226000..0x14022F000`):

- the list pointers (`0x140226BB0`: both lists, no null test): filled at the field start;
- `CCardBattle` uses: through `0x14015D8A0` (null-tested: in the field the summon's damage message
  is simply not sent; field Heartless take no damage anyway) and the task getter `0x1402AD300(0x1B)`
  (null-tested);
- the PLM and player globals (null-tested); targets through Re:CoM's lock-on pick `0x1401CDB90`
  (no type filter: field Heartless count);
- `0x14022EE30` reads the `CCardBattle` task (`0x1402AD300(0x16)`) without a null test, but it is a
  state of the card-prize object (`0x14022F7E0`, set up from `0x1401682D0`), not a summon's.

No blocker found. **Not read**: each summon's whole state code (their camera, Bambi's and Tinker
Bell's own targets): **Assumption** that they run in a field room as in battle. To test first.

### Per frame

- The menu's build (already per frame): `win::field_sleights::gate` (one atomic load; in a field
  room the KH1-Sora test and the PLM's flag: about five loads), the preload's room mark (one swap).
  Not measured.
- The PLM wrapper: once per room build. The list fill: once per field start. The stub: only during
  action 0x16 in the field (Re:CoM's own exe).

### Sites (bytes re-read from `recomsteam.exe`, `check_sites_rs.py`: 1267 sites, 0 mismatches)

| site | bytes | use |
|---|---|---|
| Field table entry `0x1401FA648` | `99 A5 1F 00 99 A5 1F 00` | patched (→ `0x1FB7B2`) |
| cave `0x1401FB7B2` | `CC` × 14 | patched (the stub) |
| PLM init `0x140210100` | `48 89 5C 24 18 56 57 41 56 48 83 EC 20` | hook (no branch into it: the scan's one hit, `0x14020F106`, is the immediate of `or [rcx+0x88],0x1000`) |
| lookup `0x1402A43E0`, data `0x1402A49C0` | 16 / 15 bytes | called |
| `CCardBattle` lookups `0x140160830` | 25 bytes | checked (the calls we repeat) |
| room test `0x140235240`; field ring allocation `0x1401AF841` | 20 / 31 bytes | called / checked (its condition) |
| PLM init ring writes `0x1402103E6` | 39 bytes | checked (the writes we repeat) |
| player dtor dim exit `0x1401E51AD` | 57 bytes | relied on |
| map enemy type `0x14019D050` | `B8 02 10 00 00 C3` | checked (the field Heartless' type) |
| also: current world `0x1402339C0`, exeStockSkill `0x140218950`, the Field exe's switch `0x1401FA1B0` and default `0x1401FA599` | other lists' | checked again at install |

### Risks

- **A pack queued when the battle starts** (a pack sleight chosen in the field, still loading, and a
  field Heartless touches Sora): the field PLM's dtor releases only loaded packs (state 2); the
  queued file load and the ring then follow Re:CoM's teardown. Re:CoM's battles end the same way with
  a stock's preload in flight. **Assumption**: the scene change waits for the loader (the field
  states poll its idle test `0x14023CA80`).
- **The camp opened mid-sleight**: Re:CoM's camp gate (`0x1401A96C0` → `0x1401AAA90`) reads no player
  action (Checked (decomp)), so Start opens the camp during a field sleight, as during a field swing
  or a KH1 cast. Not tried.
- **Dashing sleights and specials touching first**: a field Heartless's contact starts a plain battle
  (type 0, no first-strike bonus) whenever pause mode 4 is off (§ 4).
- The sleight use counter (`0x14020E590`) counts field uses; Re:CoM's field-strike counter (trophy
  0x28) counts sleight strikes.
- A Combat switch in the camp applies to the packs at the next room build (the PLM is built with the
  room): switching to KH1 inside a room leaves its pack sleights greyed until the next room.
- Summons: § above (static scan only).
- For the merge (`wt-cmd`'s Party list): friend sleights must not run in the field
  (`field_sleights::in_field` says no; a Party start should go through `win::field_sleights::start_room`
  or stay battle-only).

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
- 0.110.0 (§ Built): Checked (decomp + exe) every address it patches, calls or relies on; tests for
  the stub, the pack table, the room / MP gate, the field kinds and the menu; the Windows build.
  **Not checked in game**: anything (no field sleight, field Thunder / Stop on a field Heartless, or
  cut sleight seen yet).
