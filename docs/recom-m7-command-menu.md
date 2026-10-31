# M7 stage 7.3: KH1's command menu (decision 34)

Owner (2026-10-31): "Command menu: Re:CoM's own menu pieces (buttons, cursor, text), laid out like
KH1's"; "shown in the field and in battle, as in KH1"; "the highlighted row uses another colour of
Re:CoM's command plate; row spacing KH1's 22 units"; "menu words: KH1's own text from the player's
KH1 message files".

At Combat = KH1, in Sora's story, field and battle: KH1's root list (Attack / Magic / Items / the
context row) bottom-left at KH1's place, and the Magic list. Re:CoM draws it: each row is a
`C2DSeqCtrl` playing one of two groups the asset step added to Re:CoM's `Cockpit:00` (Re:CoM's
text plate, and the same plate in another colour for the highlighted row) with a `C2DFnt`
carrying KH1's word, attached as Re:CoM attaches its own labels to these plates. KH1's rules run
the menu: its cursor, its greying, its fades, its context tab. Confirm runs the highlighted row:
on Attack it is today's Attack (unchanged); Magic opens the Magic list; choosing a spell plays
Re:CoM's refused sound and logs "the cast comes in 7.4" (stage 7.4 hooks it to Re:CoM's magic
action). Items is greyed (M8). At Combat = Re:CoM, or in Riku's story, nothing is made and nothing
runs.

Code: `crates/kh1-recom/src/command_menu.rs` (KH1's rules, tested), `src/win/command_menu.rs`
(Re:CoM's objects), `crates/kh1-recom-assets/src/command_menu.rs` (words and plates, tested).
No hook of its own: it runs from the MP bar's three `CPlayerGauge` wrappers and the request-bits
wrapper. Research: `m7-menus-mp.md` § 1, § 2, § 3, § 7 and `m7-magic.md` § 2.1, § 2.2 (kept with
the decomp tools, not in the repo).

Steam addresses (image base 0x140000000), both exes. Tags: **Checked (decomp)**, **Checked
(exe)** = bytes / disassembly of the Steam exe, **Checked (files)** = the owner's files (KH1
`btltbl.bin` and its PC assets, Re:CoM `FO0002.SPR` from the OpenKH extract), **Assumption**.

## KH1's menu

| what | KH1 (Steam) | tag |
|---|---|---|
| Lists: 0x38-byte records at `0x140528900 + type × 0x38`: `+4` grey, `+8` lock, `+0xC` cursor, `+0x10` count, `+0x14` command ids, `+0x24` brightness, `+0x34` the row a submenu was opened from. Root (type 0) has 4 rows in the exe image, Magic (1) is filled per frame. | `0x140528900` | Checked (exe + decomp) |
| Root build, every frame: row 0 Attack (1); Counterattack (0x63) instead when the ability is on, the guard recoil 0x6E plays and the target is an enemy. Grey = lock = `!(state flags >> 4) \| !1` (Attack needs the movement state's flag 0x10); rows 1..3 the blank command 0x42. | `0x140294220` | Checked (decomp) |
| Magic: the learned spells in KH1's order, command `level + (spell + 2) × 3`. A spell is usable with the state's flag 0x20, MP for its cost (`0x1402A4C10`), and, for command flag 8 (Thunder, Gravity, Stop), the target within the command's range `+0xE` (1100). Grey = lock = the unusable ones. With spells: row 1 = Magic, not locked, greyed when none is usable. With none: row 1 stays the blank command, greyed and locked. | `0x1402944C0` | Checked (decomp + files) |
| Items: row 2 = Items, greyed and locked with no item slots. | `0x140294220` | Checked (decomp) |
| Context: row 3 cleared each frame, set (and ungreyed) by the nearby object's command (`0x140293000`). NPCs (kind 1) put Talk in row 0 instead. | `0x1402948F0` | Checked (decomp) |
| Movement-state flags: 0 ground 0xFC, 1 take-off 0x34, 2 air 0x3D, 3 0x05, 4/5 0x02, 7 0x01, 8 flight 0x76, 9–11 0x101, 0xD–0xF / 0x11 0x01, 0x15 glide 0x05, 0x1E 0x18. | `0x142DB3840` | Checked (decomp, `kh1-movement-motion.md`) |
| Cursor, on the fresh-press word compared for equality (two presses in a frame move nothing): Up / Down wrap; Right opens the row's submenu (sound 6), a locked row sounds 4, a row without one does nothing; Left or cancel closes a submenu (sound 3; cancel also puts the root cursor on Attack); Left at the root puts the cursor on Attack (sound 1); Right on the root's row 3 goes to the run (sound 4 unless Summon). No auto-repeat. | `0x1402931B0` | Checked (decomp) |
| Step order: at the root a fresh jump is the jump and the menu takes no input that frame; Triangle runs the context command when its tab is fully in; Square without Triangle is the roll / guard and no cursor that frame; the cursor; then confirm runs the row (`0x140293410` → `0x140293480`): locked → sound 4; a submenu opens; a command is issued, the stack unwinds and the root cursor returns to Attack. Confirm on the root's row 0 when it holds Talk, or on row 3, is sound 4. | `0x1402937F0` | Checked (decomp) |
| Buttons (logical, `0x14028B0B0` / `0x140528290`): Up 0x20, Down 0x40, Left 0x80, Right 0x100, Cross 0x210001, Circle 6 (cancel 2 + jump 4), Triangle 0x4080008, Square 0x900010, L1 0x40000. | | Checked (decomp + exe) |
| Draw: root at (16, 260), rows 22 apart; Magic at x + 11, at the Magic row's y with fewer than 3 rows, else at 348 − 22 n. Text colour (alpha × 128, b, b, b) (`0x14027A920`). | `0x14027B870` | Checked (decomp) |
| Brightness: the top list's rows ease 32 a frame between 0x73 (grey) and 0xE6; when the top list changes they are set at once and the lists under it go to 0x73. | `0x14027B870` | Checked (decomp) |
| Fades: in / out 0.1 a frame (10 frames, `0x140278AC0` 1 / 0) when the step starts / stops running; events hide and show at once (2 / 3, `0x140293750`), and the stack goes back to the root on Attack. | | Checked (decomp) |
| Context tab: 0 hidden, 1 sliding in (X scale + 1/7, x −10 → 0 by 10/7 per step), 2 in, 3 sliding out (scale − 1/7), restarted when its command changes. Its step is half a frame (14 frames). | `0x140277CF0` | Checked (decomp); the half step: as read in the research (`0x14233FBE0`) |
| Sounds: 1 move, 3 back, 4 error, 6 open. | `0x14017A040` | Checked (decomp) |

## KH1's words

The command record's `+4` message id names it (`0x14027A920` → `0x14028F9B0`); on PC the message
table is refilled from `btltbl.bin`'s PC asset `Word.bin` of the game's language (`0x14028F9D0`).
The asset step decodes the menu's messages from `US_ UK_ FR_ GR_ IT_ SP_Word.bin` into
`kh1-command-menu.json` (`docs/kh1-recom-assets.md` § The command menu). KH1's encoding: 0x01
space, 0x21.. digits, 0x2B.. A–Z, 0x45.. a–z, 0x63 '+', 0x6E '-', 0x71 ''', and the accented
letters the words use (0xDD Ö, 0xE4 á, 0xE9 é, 0xED í). Checked (files): every word, all six
languages. 0x6F (the blank command, message 305) is read as '-' (GR / SP write that entry with
0x6E): Assumption.

The DLL takes the language from `kh1-recom.ini` `menu_language` (us, uk, fr, gr, it, sp; default
us), missing ids from us. Re:CoM's `C2DFnt` takes ASCII (our Config rows, seen in game); its bytes
≥ 0x80 start a 2-byte code (`0x140258EB0`) whose table for accented letters isn't known, so
accented letters are written as their base letters (FR "Gravite", GR "Offnen"); the log names
them.

## Re:CoM's side

- **Plates:** `Cockpit:00` group 0x19 is Re:CoM's text plate (idle): a dark cap (sprite UV
  (229, 110)–(255, 135), part (−13, −13)…(13, 12)), a strip ((0, −13)…(152, 6), X translate 13)
  and a card-type icon. `CStockFolder` (`0x1401658F0`) shows these plates with a `C2DFnt` on
  them. The plate's texels are black (alpha-shaped), so a colour can't be multiplied onto them:
  the highlighted plate draws the strip again on the atlas's neutral texel with the magic
  plate's colour (group 0x1F's icon, vertex 0x80FFA200). Both groups are data the asset step
  adds (Checked (files)).
- **Rows:** `C2DSeqCtrl` (`operator new` `0x1404ACE54`, ctor `0x140247260`), priority `+0x16E` 2
  (the HP gauge's; the Magic list 3), `+0x138` = 1, layout position `0x140248660` at KH1's (x, y)
  + (13, 13) (the plate's cap centre, so its left / top edges are KH1's), one layer (vtable
  `+8`, `0x140247850`) at the gauge's x offset −`+0xA0` with flag 1, as the MP bar's. Text:
  `C2DFnt` (0xB0, ctor `0x140243F10(fnt, 0, 0, &colour, 0xE, text, 0x80)`), attached with vtable
  `+0x60` (`0x1402476F0`) at (0, 0), screen offset −4 − `+0xA0`: what `CStockFolder` passes
  (Checked (exe): `0x140165BEC..0x140165C4C`). Checked (decomp) unless tagged.
- **Colour:** `0x140248590(ctrl, &rgba)` writes the control's colour `+0x120` (floats, 1.0 = ×1),
  which Re:CoM's sprite update multiplies in when `+0x138` is set (`0x14024BB10`, with the
  parent's). Re:CoM's camp uses it. The menu's fade is its alpha; the text's is `C2DFnt +0x34`
  (0xAABBGGRR, 0x80 = ×1), as Re:CoM's camp changes its labels' colour. KH1's brightness bytes
  go there as they are: both games use 0x80 as ×1 (Assumption for the screen result).
- **Tab:** row 3's control gets the tab's x offset (layout position) and X scale (`0x1402486D0`)
  each frame while it slides.
- **Context:** Re:CoM's field action (`0x1401E80C0`, Triangle `+0x424 & 0x200`, field only) acts
  on the object of handle `player+0x5FC` (when > 0) else `+0x84C`, by its type getter (vtable
  `+0x28`): 0x2000 treasure box (`CGimmickTreasureBox`, `…Pooh`; not when `+0x2C1 & 1`), 0x2002
  jump stand, 0x2003 carry (`CGimmickCarry`, `…Tu`), 0x4000 NPC (`ATEL::CAvatar`, the 100 Acre
  minigames), 0x1006 door (`CGimmickDoor`, room / world select). Checked (decomp + exe: RTTI,
  getters `mov eax, imm32; ret`, the door's computed at `0x1401B8A10`). The menu reads the same
  object without calls (`0x1402AD350` for the handle, the getter's bytes) and shows KH1's word:
  chest → Open, door → Open, carry → Lift, NPC → Talk, jump stand → Jump On (the words for Re:CoM's
  objects: Assumption). Battle: no context row (KH1's battle specials are M9).
- **Inputs:** Re:CoM's pad (pure reads, as the camp's): fresh d-pad Up 0x1000 / Down 0x4000 /
  Left 0x8000 / Right 0x2000, Triangle 0x10, Square 0x80 (`0x14029B2F0`), confirm / cancel
  (`0x14029B490` / `0x14029B410`, Re:CoM's configurable buttons), only while Sora's pad is on
  (`player+0x230 & 1`). Checked (decomp); button names from bit order: Assumption.

### What changes in Re:CoM's request bits (KH1 setting, Sora's story, only while the menu is shown)

After Re:CoM's builder (`0x1401F8150`, already wrapped):

| bit | stays when | KH1 |
|---|---|---|
| `+0x424 & 4` Attack | the root list is on top with the cursor on row 0 (Attack, or Counter: the KH1 frame then plays the Counterattack, as before) and the cursor didn't use the press | confirm runs the highlighted row |
| `& 1` jump | the root list is on top | the jump only from the root; cancel closes a submenu |
| `& 0x200` Triangle | the root list is on top, and the context row's tab is in when one is shown | Triangle runs the context command once its tab is in |

Square (roll / guard) and lock-on are untouched. With no context row shown, Re:CoM's own
Triangle action decides as before (an object the menu doesn't know keeps its Triangle). While
Re:CoM's HUD is hidden (events) the menu changes nothing.

## Per frame

Nothing of ours runs per frame at Combat = Re:CoM, in Riku's story or with no menu built (one
atomic load in the request-bits wrapper). With the menu: KH1's step (pad reads, the build from
values already at hand: the movement state, the MP view the MP code stores on each read / write,
the spell levels, the context object, the target distance while a range-limited spell is
learned), then the rows: rebuilt only when what they show changed; colour writes only while the
fade (10 frames) or a brightness change (4 frames) runs; the tab's place only while it slides
(14 frames).

## Logs and trace

- Start: `command menu: KH1's command menu at combat = kh1 in Sora's story (M7 stage 7.3): words
  us (30 entries) from …; Re:CoM language index N; kh1_all_spells = off`.
- Up to 60 lines: `command menu: built on the player gauge 0x…`, `open magic (7 spells, grey
  0x0)`, `close`, `spell Firaga [0x9] chosen: the cast comes in 7.4 (Re:CoM's refused sound)`,
  `refused Items`, `context Open`.
- F9 trace, events column: `…; menu r:<cursor> g<root grey> [m:<cursor> mg<spell grey>]
  [tab<state>:<cmd>] [counter] [events]`, or `menu -`.

## Sites (`sites::COMMAND_MENU_*`, bytes from the Steam exe)

| address | bytes | use |
|---|---|---|
| `0x140248590` C2DSeqCtrl colour | `0F 28 02 66 0F 7F 81 20 01 00 00 C3` | called |
| `0x1402476F0` C2DSeqCtrl add text (vtable `+0x60`) | `40 53 48 83 EC 30 48 8B 01 0F 29 74 24 20` | called through the slot |
| `0x1406073A0` vtable slot | → `0x1402476F0` | compared at install |
| `0x1401E80E8` field action pick: handles | `8B B1 FC 05 00 00 85 F6 7F 11 8B B1 4C 08 00 00 85 F6 7F 07` | checked only |
| `0x1401B8A10` door type getter | `48 83 EC 28 BA 24 00 00 00 8D 4A DD` | checked only (its address is compared) |

Already checked by earlier stages and called: `0x1404ACE54`, `0x140247260`, `0x140247850`,
`0x140248660`, `0x1402486D0`, `0x14015D230`, `0x140243F10`, `0x1402AA360`, `0x14029B2F0`,
`0x14029B410`, `0x14029B490`, `0x1402AD350`. No new hook, no byte patch.

## 1:1 checks

- Checked (decomp): the lists, the build rules, the cursor rules, the step order, the layout
  numbers, the brightness steps, the fade, the tab's state machine, the sounds' meaning. Tests
  (`command_menu::tests`) replay them: wrap, equality, Right / Left / cancel, the Attack row,
  the jump and Square frames, greying by MP (Fire / Blizzard at 30 charge units) and range, the
  14-frame tab and its Triangle gate, the 4-frame greying, the 10-frame fade, KH1's positions.
- Checked (files): the words in six languages; the plates in Re:CoM's base `FO0002.SPR`.
- Checked (exe): every new site's bytes and the vtable slot.
- Not checked: nothing seen in game; no KH1 recording of the menu (a KH1 video of opening Magic
  and of walking up to a chest would confirm the timing frame by frame).

## Not 1:1 by design or yet

- The look is Re:CoM's plates (decision 34), not KH1's panels; the highlighted row is a colour,
  not KH1's cursor.
- KH1 puts Talk in row 0 (instead of Attack) next to NPCs; here every context command is in row 3
  (the owner's "context row"): Attack stays Attack. Open question 2.
- Confirm on a locked Attack row (a state without flag 0x10, e.g. the glide) gives no error sound
  here (today's Attack path is left as it was).
- The Magic list's spells do nothing in 7.3 (refused sound); Cure / Aero's party list comes with
  their casts.
- KH1's slide-in of a newly opened list (`0x14027A0B0`'s "fresh" flag) is not reproduced: lists
  appear at once.
- Text without accents (Re:CoM's codes for them unknown).

## Assumptions

1. KH1's (x, y) is the row's top-left; Re:CoM's 512 × 384 layout space is KH1's 2D space; the
   HUD's −85 x offset matches KH1's widescreen margin. To check on a screenshot.
2. The text sits on the plate where `CStockFolder` puts it ((0, 0), −4); with the icon gone it may
   need a nudge right.
3. Re:CoM's sound ids: 1 cursor, 2 decide, 4 back, 5 refused (from how the camp uses them).
4. Writing `C2DFnt +0x34` and the control colour each frame of a fade is safe (Re:CoM's camp
   writes label colours the same way).
5. Deleting our rows in the HUD-hide wrapper is safe (they are ours, not the gauge's).
6. The menu (and a cursor on the empty row 3) is drawn where KH1 would: row 3's plate is drawn
   empty when the cursor is on it with no context command.
7. A new player gauge (room, battle) resets the stack to the root on Attack and fades in, as
   KH1's step starting again.
8. The context words for Re:CoM's objects (Open for doors, Jump On for jump stands).

## Open questions (owner)

1. **Highlight colour:** (a) the magic plate's blue (built); (b) another of Re:CoM's plate
   colours (red / grey); (c) a value you pick from a screenshot.
2. **Talk next to NPCs:** (a) in the context row (built; Attack stays Attack); (b) KH1's rule:
   row 0 becomes Talk near an NPC in the field (confirm then gives the error sound, Triangle
   talks).
3. **Menu language:** (a) the ini's `menu_language` (built, default us); (b) follow Re:CoM's own
   language automatically (needs one log line from each language: the `Re:CoM language index`).
4. **Accented letters:** (a) base letters (built); (b) find Re:CoM's codes for them (needs Re:CoM's
   message files of a European language at the asset step).

## What the first in-game test must check

1. Log: the start line (words, language index), `mp bar: … groups 151..158` with no "has N
   groups" line, then on entering a room `command menu: built on the player gauge …`.
2. **Screenshot** in the field: four rows bottom-left (Attack highlighted in blue, a dash row
   greyed, Items greyed; row 3 only near something), KH1's words, no overlap with the HP / MP
   bars. Another one with `kh1_all_spells = on` and the Magic list open (7 spells, Firaga …
   Aeroga, x + 11, bottom at 348).
3. Up / Down move the highlight (with Re:CoM's cursor sound), wrap through row 3; two d-pad
   presses at once move nothing; the stick doesn't move it.
4. Confirm on Attack swings as before (combos, air combos, Counterattack after a guard: row 0
   reads "Counter" during the recoil).
5. Cursor on Magic without spells: confirm buzzes. With `kh1_all_spells = on`: Right or confirm
   opens the list; Left closes it (cursor stays on Magic); cancel closes it (cursor back to
   Attack, Sora doesn't jump); a spell: buzz and `spell … the cast comes in 7.4` in the log.
   With MP 0 the spells grey (over 4 frames), Fire / Blizzard come back at 30 charge.
6. In the field next to a chest / NPC / door / box to carry: row 3 slides in (about a quarter
   second) with Open / Talk / Open / Lift; Triangle works once it is in, not before. In battle:
   no row 3.
7. Cutscenes / events: the menu disappears with the HP gauge and comes back with it, on Attack.
8. Combat = Re:CoM in the Config: the menu goes when the camp closes; back to KH1: it comes back.
   Riku's story: no menu.
9. F9 trace: the `menu r:0 ge` token (no spells, no context: rows 1–3 grey), and its events when
   moving / opening.
