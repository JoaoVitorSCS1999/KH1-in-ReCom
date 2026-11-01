# M8 stage 8a.1: KH1's items (the Items list, slots, stock and the use)

Owner (2026-11-01, "Finish everything up to M9"; M7–M9 choices in
`design-philosophy-instructions.md`): items are **sold in Re:CoM's Moogle shop and dropped by
enemies** (8a.3 / 8a.4); **healing amounts: Re:CoM's own item-card amounts** (see open question 1:
Re:CoM's item cards heal no HP). This stage: KH1's Items list in the command menu, Sora's slots and
stock in our save block, and the use, run by Re:CoM's own item action `0xE`. Not in this stage: the
camp Items page (8a.2), the shop (8a.3), the drops (8a.4).

At Combat = KH1, in Sora's story, field and battle. At Combat = Re:CoM, or in Riku's story, nothing
of this runs: Re:CoM's item cards are untouched (our `exeItem` hook passes them through).

Code: `crates/kh1-recom/src/items.rs` (KH1's rules, the save layout, the use task; tested),
`src/win/items.rs` (Re:CoM's side), the Items list in `src/command_menu.rs` and
`src/win/command_menu.rs`, the use command in `movement.rs` (`Kh1Sora::frame`) and
`movement/tests.rs`. Asset step: `kh1-recom-assets` `command_menu::NAMES` (the item words) and
`kh1_hud::menu` (KH1's Items list panels).

Steam addresses (image base 0x140000000), both exes. Tags: **Checked (decomp)**, **Checked (exe)**
= bytes / disassembly of the Steam exe, **Checked (files)** = the owner's files (KH1 `btltbl.bin`,
`xa_ex_0010.mset`, `Word.bin`, `exchange/UK_uibin_item.bin`; Re:CoM `PL0001.ANN`,
`SY0002.BIN`), **Assumption**.

## KH1 (Steam)

| what | where | tag |
|---|---|---|
| **Item rows**: `btltbl.bin` file `0x1A58 + (id − 1) × 0x14` (lookup `0x14028F970` on `0x142D22D38`): `+0` name message, `+2` kind (0 = battle item), `+4` bit 0 party, `+6` amount, `+8` buy, `+0xA` sell. Battle items: 1 Potion (30 HP; 60 / 12 munny), 2 Hi-Potion (60; 150 / 30), 3 Ether (3 MP; 200 / 40), 4 Elixir (255; – / 80), 6 Mega-Potion (party, 30; – / 40), 7 Mega-Ether (party, 3; – / 60), 8 Megalixir (party, 255; – / 100). Row 5 is empty. | `0x14028F970` | Checked (files + decomp) |
| **Effect** `0x1402CD780` (via `0x1402CD580`): items 1 / 2 / 6 → HP add `0x1402CD5E0(amount)`; 3 / 7 → MP add `0x1402CD6B0(amount)`; 4 / 8 → both with **9999**. On a live actor the adds run only **below max** (`+0x3C < +0x40` HP, `+0x44 < +0x48` MP), through `0x1402A4920` / `0x1402A4A00` (MP points, clamped; the charge untouched). | `0x1402CD780`, `0x1402CD5E0`, `0x1402CD6B0` | Checked (decomp) |
| **Can-use** `0x1402CD3A0` (HP below max for HP items, …) is called only by the pause menu's item screens (`0x1402E8920` & al.), not by the battle menu: in battle an item is used at full HP too (and consumed). | `0x1402CD3A0` | Checked (decomp: callers) |
| **Slots / stock**: Sora's party record (`save + 4 + member × 0x74`; Sora `save + 4`): `+0x21` slot count (save `+0x25`), `+0x22..` slot item ids (save `+0x26..`); stock one byte per item at `save + 0x499 + id`. | `0x140294220`, `0x1402C2760`, `0x140284E60` | Checked (decomp) |
| **New game** `0x1402852C0`: stock `+0x499..` zeroed; Sora's record from btltbl `0x39D8` (record stride 0x17): slot count = byte `0x16` = **3**, the slot ids zeroed. So 3 empty slots, no items. | `0x1402852C0` | Checked (decomp + files) |
| **Slot growth**: level-up bonus kind 6 = record `+0x25` + 1 (`0x1402B5660` case 6; kind 7 is the accessory slots). Sora's table btltbl `0x3AC0` (byte `[L − 2]` for level L): kind 6 at **18, 30, 42**; each Dive path's second table (`0x140528D10[save+0x48C]` → `0x3B28` / `0x3B90` / `0x3BF8`): kind 6 at **54** (all three) and at **3** (`0x3B90` only). | `0x1402B51B0`, `0x1402B5660` | Checked (decomp + files); which table is the owner's path: not read |
| **Pickup / return** `0x140284E60(item, member)`: a battle item (kind 0) goes into the member's first empty slot; else (or slots full) `save + 0x499 + item` + 1 while below 99 (returns 0xC); else refused (0xFF). | `0x140284E60` | Checked (decomp) |
| **Menu build** `0x140294220` (with save `+0x874 & 0x10` clear: items not disabled): row 2 = Items (3), unlocked; the Items list (type 2 at `0x140528970`) = the filled slots in slot order, command `item + 0x1E`, `0x142D53A90[k]` = the slot, grey bits cleared per entry. Entries: cursor clamped; state without flag `0x40` → row 2 greyed (**not locked**) and all entries greyed and locked. No entries (or `+0x874 & 2`): row 2 greyed and locked. | `0x140294220` | Checked (decomp) |
| **Item commands** `0x1F..0x26` (command table `+0xA58`, 16-byte records): `+0` flags 2 (face the target), `+4` message = the command id (31 Potion … 38 Megalixir), `+6` type 3, `+7` the item, `+8` the list it opens: **4 (the party list)**, 6 for the Mega items. | btltbl `0xA58` | Checked (files) |
| **Commit** `0x140293480` for an item: locked → sound 4 (fresh press); a command with a submenu opens it (the party list); from the party list: the target; then KH1's gate: refused in the take-off (`+0x70` = 1), when `0x1402C16E0`, unless free or in a combo window (`0x1402A2EC0`; items have no exception); else the command word (type 3, item << 8), `0x142D539E4` = the Items list entry's slot; the stack unwinds. No sound. | `0x140293480` | Checked (decomp) |
| **Use** `0x1402A3600` type 3: states 0 / 6 / 0x18 → ground `0x1402A3DB0`; the air (state 2) and other states drop it. `0x1402A3DB0` case 3: motion `0x100003E` (on self; `0x100003F` on another) with blend 8.0, `0x40000000` (release, idle) blend 8.0, then `0x1402C2760(actor, target, item)`. Facing `0x1402A38E0` → `0x1402A3B00` with the target = self: the facing kept, speed 0. | `0x1402A3600`, `0x1402A3DB0`, `0x1402A38E0`, `0x1402A3B00` | Checked (decomp) |
| **Motions** (`xa_ex_0010.mset`): `0x3E` → entry 58, **82 frames, free from 74**, no root travel; `0x3F` → entry 59, 72 frames, free from 62. Both a type-6 track at 30.84 (meaning not read). | `kh1_mset.py` | Checked (files) |
| **Take** `0x1402C2760`: the task `0x1402C2880` on task list `0x142DB34B0` (run by `0x1402AEDA0` at **priority 139999**, after the actors), its wait = btltbl G `+0x40 + member × 2` (self; `+0x41` other; G = btltbl + 0x58): Sora **34** (self) / 36. Unless the item word's bit 10: Sora's slot `save + 0x26 + 0x142D539E4` = 0 (others: `0x140284CE0`). Nothing refills the slot from the stock. | `0x1402C2760` | Checked (decomp + files) |
| **The use task** `0x1402C2880`, each run: wait −= frame step; Sora not alive (`0x140291FF0`: `+0x374` bit 0 and not bit 1) or `0x140297950` (`+0x444` bit 0) → `0x140284E60(item, member)` (the item back) and end. State 0: when the wait is **below 0** (`<= 0 && != 0`): if the motion playing is still the one at the first run, effect **1 of `always.dpx`** at bone **0x1C** (0x1D for member 6, scale 3 for member 9); then state 1, wait = **30**. State 1: (a dead target → back); when below 0: the heal `0x1402C2B50` on the target (no target: every party member), Sora → journal counter `0x14028C660(0xC)`; end. So at 60 fps: the hand effect on the run 34 frames after the command, the heal 31 runs later (**frame 65**). | `0x1402C2880` | Checked (decomp) |
| **`+0x444` bit 0** is set / cleared only by the actor message handler `0x14028E6E0` cases 0xE / 0xD (zeroed at actor set-up): an event's hold, not a hit. **A hit does not return the item; the heal still comes** (the motion check only skips the hand effect). The brief's research had "hit before the heal → item returned (Assumption)": not KH1's. | `0x14028E6E0` | Checked (decomp: every writer of `+0x444`) |
| **Heal** `0x1402C2B50`: on a target not flagged (`*target+0x6C` record bit 2): `0x1402CD580` (the effect above), a reaction message `0x140297CA0(target, actor, item | 0x10000)`, and with target `+0 & 0x8000` the recovery effect **`0x14052A0A7[item]`** of `always.dpx`, following the target: Potion 3, Hi-Potion 4, Ether 5, Elixir 6, Mega-Potion 4, Mega-Ether 5, Megalixir 6. | `0x1402C2B50` | Checked (decomp + exe) |

## Re:CoM (Steam)

| what | where | tag |
|---|---|---|
| **Item cards** 0x2A..0x30 (Potion, Hi-Potion, Mega-Potion, Ether, Mega-Ether, Elixir, Megalixir). `BattleCardList` rows: power 0 in all four columns, `+8` 3, `+0xA` 30 / 40 / 45 / 20 / 35 / 45 / 50 (their CP), `+0xB` 5. | `SY0002.BIN` `BattleCardList.bin` | Checked (files) |
| **`initItem`** `0x1401F28F0` (action `0xE`): prev = action; action (8 bytes) = 0xE; sub 0; `0x1401A38D0` (velocity 0); `+0x428 &= 0xFFF7FF77`; `+0x424 &= ~8`; `+0x418 &= 0xFD7FFFFF`; `+0x63C` = its argument; `+0x634`, `+0x644` (8), `+0x64C` (8), `+0x7B4`, `+0x4E0` = 0; `+0x5D8` = base gravity; `+0x604` = 3; sub 1; **`+0x65C = 0x1407C7830[BattleCardList[+0x7F4] + 8]`** (the card table, loaded only for a card battle). | `0x1401F28F0` | Checked (decomp + disasm) |
| **`exeItem`** `0x1401ED6E0`: sub 1: in the air the fall, on the ground sub 2 and **row 0x2D** (`p001_032`); sub 2: at the clip's **frame 56** (`cmp edx,0x38`) sub 3, the card's effect (switch on `+0x7F4`: Potion `always` 3, Hi-Potion 4, Mega-Potion `itemplus` 0, Ether `always` 5, Mega-Ether `itemplus` 1, Elixir `always` 6, Megalixir `itemplus` 2) through `0x1401A2D50(player, name, id, bone 0, &(0, 0.85, 0, 1), 0, 0.0)` kept on him (`+0x4C |= 4`), then **the card effect `0x14016C910(revolver, 0, …)`**: a **reload** (`0x14016A450`: Potion types reload attack cards, Ether types magic cards, Elixirs all). **No HP is healed.** sub 3: the end `0x1401F6660` when the clip ends. | `0x1401ED6E0` | Checked (decomp + disasm) |
| The Battle exe calls it directly (`0x1401F9A13`); the **Field exe has no case for 0xE** (its table entry `0x1401FA628` = the default `0x1401FA599`). | `0x1401F9840`, `0x1401FA5F4` | Checked (exe + decomp) |
| `p001_032` is KH1 entry 58's bake (the assets' validate test `BAKES`: slot 32 ↔ entry 58): 41 frames at 30 fps (= 82), bone 0 constant 0 (no root travel). | `PL0001.ANN` | Checked (files: `recom_ann.py`) |
| The heal `0x1401E59A0(player, amount, 0, 1, 0)`: HP + amount clamped to max, the "Recover" popup (`0x14020C9A0(…, 0)`), the HUD's HP setter. | `0x1401E59A0` | Checked (decomp) |
| Re:CoM's damage check `0x1401E6DE0` treats action 0xE as 0x13 (both end on a card break `+0x428 & 0x20`; no card breaks at KH1). | `0x1401E6DE0` | Checked (decomp) |

## What runs (at the KH1 setting)

1. **Menu** (`command_menu.rs`): root row 2 is Items; Right / confirm opens KH1's Items list of
   the filled slots (KH1's open sound 6), at x + 11, at y 304 (one entry) or 348 − 22 n. Greying
   exactly as `0x140294220` (none for full HP). Confirm on an entry: greyed → error sound; KH1's
   gate refuses → nothing (no sound, the list stays); else **used on Sora at once** (KH1 opens the
   party list: no party yet, as the owner chose for Cure / Aero; open question 2), no sound, the
   menu back on Attack. Log / trace: `item Potion slot 1 used`, `open items (…)`, `menu i:<c>
   ig<grey>`.
2. **Command** (`Kh1Sora::command_item`, the same frame): KH1's gate again, ground only (the air:
   refused, `state`), facing kept and Sora stopped, Re:CoM's attack ended if a swing played (a use
   from a combo window), then `Host::start_item`.
3. **Start** (`win::items::start_item`): **KH1's `0x1402C2760`**: the item leaves its slot in our
   save block (refused if the slot no longer holds it) and KH1's use task starts; then
   **`initItem`'s writes**, cardless (as `win::magic::start_cast` does for `initMagic`): action
   0xE, sub 2, phase 3, `+0x65C` = 0 (no card; table index 0 is 0). `initItem` itself isn't called
   (its card read).
4. **Motion**: KH1's `0x3E` (82 frames, free from 74) with blend 8, the idle queued (blend 8),
   played by **Re:CoM's own item clip, row 0x2D** (`rows::row_for`). Sora is locked to frame 74
   (KH1's free key), then free; the action ends (`0x1401F6660`, as `exeItem` ends it) when the
   motion ends into the idle or another command takes over.
5. **Each frame of the motion**: the Battle exe calls `exeItem`, the Field exe through **a new
   case** (`items::field_case`: table entry `0x1401FA628` → a 13-byte stub `mov rcx,rbx; call
   0x1401ED6E0; jmp 0x1401FA599` in the int3 cave `0x1401FAEA2`). Our hook on `exeItem` runs
   KH1's frame for a KH1 use (marked by `win::items::ACTIVE`) and Re:CoM's `exeItem` for anything
   else: **Re:CoM's frame-56 effect and its card reload never run for a KH1 item.**
6. **The use task** (`win::items::task_frame`, from the request-bits wrapper at the start of each
   frame of Sora's = KH1's run after the actors of the frame before, as `cast_step` for spells):
   `crate::items::UseTask` = `0x1402C2880`: the hand moment 34 frames after the command, the heal
   31 runs later (frame 65 at 60 fps; at Re:CoM's 30 fps rooms 2 a run: the same frames). It runs
   on its own: a hit during the use doesn't stop it (KH1). Sora dead before the heal (HP 0 or
   Re:CoM's death path `+0x7B0`, KH1's alive test) → the item back by KH1's pickup rule.
7. **Heal** (`win::items::heal` = `0x1402C2B50` on Sora): **Re:CoM's own effect for the item**
   (`exeItem`'s: the set and id above, on Sora's bone 0 at (0, 0.85, 0), kept on him; the four
   base items' ids are KH1's recovery effect ids too); HP through **Re:CoM's heal**
   `0x1401E59A0(player, amount, 0, 1, 0)` **only below max** (KH1's `0x1402CD5E0`); MP through
   **`win::mp::add_points`** (KH1's `0x1402CD6B0` → `0x1402A4A00`: below max, MP + n clamped, the
   charge untouched). The Mega items heal the whole party: Sora alone for now.

### What each item does here

| item | HP (Re:CoM's) | MP (KH1's points) |
|---|---|---|
| Potion, Mega-Potion | **140** (KH1's 30 × 560 / 120: provisional, open question 1) | – |
| Hi-Potion | **280** (60 × 560 / 120: provisional) | – |
| Ether, Mega-Ether | – | **+3** (KH1's amount; Re:CoM has no MP) |
| Elixir, Megalixir | **full** (KH1's 9999) | **full** (9999 → max; the charge untouched) |

`560 / 120` is Re:CoM's HP cap over KH1's, the ratio the KH1 HUD already shows Re:CoM's HP with
(Stage 73). One table: `crate::items::effect`.

## The save: our block's layout v2

Our block (`docs/recom-m7-mp.md` § The save block) was 16 bytes (`save + 0x2675 ..= 0x2684`,
layout v1). **Layout v2 adds the items extension right after it**, `save + 0x2685 ..= 0x269E`
(26 bytes), still inside the spare tail `save + 0x246B ..= 0x2817` that no Re:CoM code uses
(`docs/recom-keyblade-equip.md`); the Keyblades page's spare-range check ignores it.

| save + | size | field |
|---|---|---|
| `0x2685` | 1 | the extension's version: **0 = never written** (a layout v1 save, or a new game) → KH1's new game: 3 empty slots, no stock; 1 = written |
| `0x2686` | 1 | slot count (KH1 party record `+0x21`) |
| `0x2687` | 8 | the slots' item ids (record `+0x22..`), 0 = empty |
| `0x268F` | 16 | stock of item ids 1..=16 (KH1 `save + 0x499 + id`), at most 99 |

- **Migration from layout v1:** nothing to do: a v1 save's extension is all zero and reads as
  KH1's new game; it is written on the first change (a use, a pickup, the debug inventory). The MP
  block's own version byte keeps its meaning (0 = full MP never written) and isn't raised.
- Fields never move; a later version only adds and raises the version byte.
- Re:CoM saves, checksums, loads, snapshots and zeroes it with its 0x2AD4-byte save block (as the
  MP block): a Continue after a game over gives the items back as they were at the checkpoint.

## API for 8a.2–8a.4 (`win/items.rs`)

`slots()` (the slots and count), `pick_up(item, why)` (KH1's `0x140284E60`: the first empty slot
for a battle item, else the stock below 99; `None` when refused or not live). Pure:
`crate::items::{Inventory, read, write, data, effect, command}`.

## Debug: `kh1_test_items`

`kh1-recom.ini`: `kh1_test_items = on | off` (default **off**). When on, each time a room or a
battle starts (a new player HUD), if every item slot is empty they are filled with Potion,
Hi-Potion and Ether (as many as there are slots: 3) and the stock is raised to at least 9 of each
battle item. **Written to the save** (it stays when the switch goes off). Not KH1's: KH1 never
refills a slot. Log: `items: kh1_test_items: slots [1, 2, 3] (of 3), stock 9 …`.

## Assets

- `kh1-command-menu.json` gets the item words 31..34, 36..38 (Potion … Megalixir) in all six
  languages; the KH1 look gets a word group for each. New letters: `Ä` (0xCF, GR "Äther"), `É`
  (0xD2, SP "Éter"), `ó` (0xF2, SP "Poción"), `è` (0xE8). Checked (files: the owner's six
  `Word.bin`s decode, test `owner_words`); the byte ↔ letter table's layout: Assumption (it fits all
  seven known letters).
- KH1's Items list panels: `exchange/UK_uibin_item.bin` (17 objects, the Magic list's in the same
  order, green (58, 86, 66), its strip 78 wide), baked as the Magic list's six groups (header +
  title "ITEM", row, last row, selection, the potion icon) after the words; `kh1-hud.json`'s
  `menu.items_list` (optional: the DLL draws the Items list with the Magic list's panels, logged
  once, without it). Checked (files: the dump test builds it, groups 5777..5782 before the shortcut
  panel). Not rendered offline.

**The owner's asset step must be re-run** for the item words and the Items list panels (no new
option, the same command as before).

## Sites (`sites::ITEM_*`, bytes from the Steam exe)

| address | bytes | use |
|---|---|---|
| `0x1401ED6E0` exeItem | `48 89 5C 24 10 57 48 83 EC 60` | **hooked** (3 whole instructions, no RIP-relative) |
| `0x1401FA628` Field exe table, action 0xE (and 0xF) | `99 A5 1F 00 99 A5 1F 00` | patched: → `0x1401FAEA2` |
| `0x1401FAEA2` int3 cave (after `pop rdi; ret`) | 14 × `CC` | patched: the 13-byte stub |
| `0x1401F28F0` initItem (to its action write) | 70 bytes | checked only (replicated) |
| `0x1401F29A3` initItem's card read | 48 bytes | checked only |
| `0x1401ED7F8` exeItem frame 56 | `83 FA 38 0F 8C …` (21) | checked only |
| `0x1401ED82E` exeItem effect switch | 39 bytes | checked only |
| `0x1401ED8F4` exeItem effect call | 53 bytes (→ `0x1401A2D50`, `0x140257EF0`) | checked only (the heal's effect is the same call) |
| `0x1401ED929` exeItem card reload | 51 bytes (→ `0x14016C910`) | checked only (never reached for a KH1 item) |
| `0x1401F9A10` Battle exe case 0xE | `48 8B CF E8 C8 3C FF FF E9 AC 02 00 00` | checked only |

Called, already checked by earlier stages: `0x1401E59A0` (heal), `0x1401F6660` (end action),
`0x1401F3C00` (ground test), `0x1401A2D50` / `0x1402ABB20` / `0x140257EF0` (effect on a bone).
Read at run time and checked: the effect names `0x1407C5DD8` → `"always"`, `0x1407C5DE0` →
`"itemplus"` (pointers, relocated; without them the heal plays no effect, logged). Every byte set
compared with `recomsteam.exe` (pefile script: 375 sites, 0 mismatches).

## Per frame

At the Re:CoM setting, in Riku's story, or with no use going: two atomic loads in the
request-bits wrapper (the pending command, the task flag). While the menu runs: one 26-byte read
of the save block for the slots. During a use: KH1's frame (as any KH1 move) and one task run a
frame. No new per-frame hook: `exeItem` only runs in action 0xE.

## Logs and trace

- Start: `items: KH1's items at combat = kh1 in Sora's story (M8 stage 8a.1): slots and stock at
  save+0x2685..=0x269e …; Field exe case 0xE -> exeItem; kh1_test_items = off`, plus the hook
  line `hooked exeItem (0x1401ED6E0) …` and the two patch lines.
- Up to 80 lines: `items: use(potion slot 1)`, `use-refused(ether state | not now | not in its
  slot)`, `use potion: KH1's hand moment at f34`, `use potion: heal at f65: HP 120 -> 260 (+140,
  max 300)`, `mp: points +3 (ether): …`, `items: potion returned (Sora not alive before the heal):
  slot 1`, `dropped (no KH1 frame ran …)`.
- Command menu: `open items (2 filled slots [0, 2], grey 0xfc)`, `item Potion slot 1 used [0x1f]`.
- F9 trace, events column: `…; item -` or `item potion f<age> s<state> d<wait>` and this frame's
  item events (`use(…)`, `hand potion`, `heal potion <HP>`, `return potion`); the menu token `menu
  i:<cursor> g… ig<grey>`.

## 1:1 checks

- **Checked (decomp + files)**, replayed by tests (`items::tests`, `command_menu::tests`
  `items_list_*`, `movement::tests` "M8 stage 8a.1", `rows::tests`): the rows and commands; KH1's
  Items list (filled slots in slot order, greying in the air, none for full HP, the list's place:
  304 for one entry, 348 − 22 n); the commit (gate, no sound, unwind to Attack, the entry's
  slot); ground only; the facing kept; motion 0x3E locked to 74, Re:CoM's action ended once at
  82; the task's waits (hand on run 34, heal on run 65 at 60 fps; 17 / 33 at 30 fps; strictly below
  0); a cut motion skips only the hand moment; dead → returned; the pickup rule (first empty slot,
  then stock to 99, no refill on use); the save extension (v1 migration, round trip, clamps).
- **Checked (exe)**: every site's bytes; the Field case stub; KH1's recovery effect ids.
- **Checked (files)**: `p001_032` = KH1 entry 58 (82 frames, no root travel).
- **Not checked**: in game; no KH1 recording of an item use (a frame-stepped KH1 video of a Potion
  would confirm the hand at 34 and the heal at 65 on screen).

## Not 1:1 by design or yet

1. **HP amounts** (open question 1): Re:CoM's item cards have none; provisional KH1 × 560 / 120.
2. **No party list** (open question 2): used on Sora at once (KH1 opens the party list; the
   owner's Cure / Aero choice applied). The Mega items heal Sora only.
3. **KH1's hand effect** (`always.dpx` 1 at bone 0x1C on the 34th run) is not drawn: Re:CoM has no
   effect at that moment (its own item effect plays at the use) and what Re:CoM's `always.dpx`
   id 1 looks like is not known. Open question 3. The heal's effect is Re:CoM's own per-item one.
4. **Slot growth** is deferred: 3 slots always. KH1: + 1 at Sora's levels 18, 30, 42, 54 (and 3 on
   one Dive path), on Re:CoM's level when built (open question 4).
5. **KH1's journal counter** (`0x14028C660(0xC)`, items used) has no Re:CoM place: not kept.
6. A room or battle change before the heal returns the item (KH1: an event's hold returns it;
   whether KH1's map change ends its tasks the same way: Assumption).
7. Re:CoM's `exeItem` passes 1 as the motion request's 5th argument (controller flag 8, a root
   reset: meaning not read); ours passes 0 as for every KH1 motion (the clip has no root travel).

## Assumptions

1. Re:CoM never sets action 0xE in the field (its items are card plays, battle only), so the new
   Field case changes nothing at the Re:CoM setting (as magic's 0x13 case).
2. The request-bits wrapper runs once a frame of Sora's, before his state exe, field and battle
   (Checked (decomp) for both exes); that its early-out covers Re:CoM's events (the task pauses
   rather than returns): Assumption.
3. The frame step for the task's run is the player's `+0x460` at the wrapper (as the menu's tick):
   ±1 frame at variable steps.
4. KH1's alive test on Sora = Re:CoM HP > 0 and no death path (`+0x7B0`) (as Cure's target test).
5. The effect names stay `always` / `itemplus` (read and checked at run time).

## Open questions (owner)

1. **Healing amounts.** Re:CoM's item cards reload cards and heal no HP (Checked (decomp + files):
   `exeItem` → `0x14016C910` → `0x14016A450`; `BattleCardList` power 0). Which amounts?
   (a) KH1's on Re:CoM's HP scale × 560 / 120: Potion / Mega-Potion 140, Hi-Potion 280, Elixirs full
   (built, provisional); (b) KH1's raw numbers (30 / 60 HP); (c) Re:CoM's Cure numbers: Potion 50,
   Hi-Potion 150, Elixirs full (Cure / Cura / Curaga's 50 / 150 / 300); (d) your own numbers.
2. **Party list**: (a) used on Sora at once (built; as Cure / Aero); (b) wait for the party
   milestone.
3. **KH1's hand effect at 34**: (a) none (built); (b) Re:CoM's `always.dpx` id 1 at the hand, if it
   turns out to look right in game; (c) Re:CoM's item effect at the hand moment instead of at the
   heal.
4. **Slot growth**: (a) KH1's levels (18, 30, 42, 54, and 3 on the shield path) on Re:CoM's level,
   applied as the MP level-ups are (when read); (b) later, with M9's level-ups.

## What the first in-game test must check

Set `kh1_test_items = on` in `kh1-recom.ini` (debug), re-run the asset step, Combat = KH1.

1. Log: the `items:` start line, `hooked exeItem`, the two patch lines, then on entering a room
   `items: kh1_test_items: slots [1, 2, 3] (of 3) …`.
2. Command menu: Items is white; Right / confirm opens a green list (KH1's look) of Potion,
   Hi-Potion, Ether at the Items row, KH1's words. In the air the list greys (still opens).
3. Get hurt, Items → Potion: Sora stops (no turn), plays Re:CoM's item animation (the KH1 use),
   about 1.1 s after the command (65 frames) Re:CoM's Potion effect on him and **+140 HP**
   (provisional); Sora free at about 1.2 s (74 frames). The Potion is gone from the list (the slot
   stays empty: no refill from stock). Log: `use(potion slot 1)`, `heal at f65: HP … (+140 …)`.
4. Ether: MP + 3 points (MP bar), at full MP nothing (still used). Hi-Potion at full HP: used, no
   heal, Re:CoM's effect plays.
5. Get hit during the use (before about 1 s): the heal still comes (KH1). Die during it: the item
   comes back (log `returned`).
6. The same in the field (no enemies) and in battle; in the field the Re:CoM action 0xE runs
   through the new case (no freeze at the end).
7. Save, quit, load: the slots and stock come back. A save that never had items shows 3 empty
   slots (Items greyed with the debug switch off).
8. Combat = Re:CoM: Re:CoM's item cards reload cards as before (our hook passes them); Riku's
   story: unchanged.
9. F9 trace: the `item potion f… s… d…` token, `menu i:…`.
