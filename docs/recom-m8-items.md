# M8 stage 8a.1: KH1's items (the Items list, slots, stock and the use)

Owner (2026-11-01, "Finish everything up to M9"; M7–M9 choices in
`design-philosophy-instructions.md`): items are **sold in Re:CoM's Moogle shop and dropped by
enemies** (8a.3 / 8a.4); **healing amounts: Re:CoM's own item-card amounts** (see open question 1:
Re:CoM's item cards heal no HP). This stage: KH1's Items list in the command menu, Sora's slots and
stock in our save block, and the use, run by Re:CoM's own item action `0xE`. Not in this stage: the
camp Items page (8a.2, [below](#stage-8a2-kh1s-items-page-in-the-kh1-camp-entry)), the shop (8a.3,
[below](#stage-8a3-kh1s-items-in-recoms-moogle-shop-not-built-findings-and-options)), the drops
(8a.4).

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

Since 8a.2: `inventory()` (a read), `edit(f)` (read, change, write back when changed), `menu_ctx()`
(HP / MP below max for KH1's pause-menu use rule), `use_in_menu(item)` (the effect at once, no
motion). Pure: `crate::items_page`.

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

---

## Stage 8a.2: KH1's Items page in the "KH1" camp entry

Ask (M8 plan row 8a.2): KH1's pause-menu Items screen, which moves items between the stock and
Sora's slots, as a third row "Items" of the KH1 camp submenu, on Re:CoM's camp list machinery
(cursor, sounds, text drawing as Customize) with KH1's item names; KH1's rules where Re:CoM's camp
list can express them.

Code: `crates/kh1-recom/src/items_page.rs` (KH1's screen as a window stack; tested),
`src/keyblades.rs` (`sub::Row::Items`), `src/win/keyblades.rs` (`Content::Items`, `item_word`,
`items_confirm`, `items_read`), `src/win/items.rs` (`inventory`, `edit`, `menu_ctx`,
`use_in_menu`), `src/items.rs` (the rows' sort key).

### KH1's Items screen (Steam)

The screen's windows share globals: `0x142E9C1F8` the state, `0x142E9C1FC` the source owner (a
member index, −1 the stock), `0x142E9C208` / `0x142E9C20C` the source slot / item, `0x142E9C210`
the command, `0x142E9C218` the destination owner. Words are sysmsg ids (KH1's sysmsg file is not in
the asset step: not read).

| what | where | tag |
|---|---|---|
| **State 0, owners**: the party members (names from table `0x1405452B8`), then "Stock" (sysmsg `0x38`), then a row sysmsg `0x39` shown only when `0x142E9C764` ≠ 0 (a count over `0x140225F50` / `0x14020CF40`: not items; not built). A member with 0 slots or the stock with an empty list is greyed and its confirm is error sound 4. | `0x14030D160` | Checked (decomp) |
| **State 1, a member's slots**: one row per slot (record `+0x25` rows), an empty slot named sysmsg `0x97`; the help window shows the item's description (row `+0xC`). Confirm on any row (an empty one too) → state 2 with the slot and its item (0 for empty). | `0x14030FA10` | Checked (decomp) |
| **State 1, the stock**: two columns, the count drawn beside the name when above 1. | `0x14030F330` | Checked (decomp) |
| **Stock lists** (rebuilt after every change): every id 1..255 with a stock byte; the list shown holds kinds 0, 12, 13 (`0x1402E8F80`, table `0x140545320` = 0, 12, 13); the equip list only kind 0 (`0x1402E8F50`); both sorted by row `+0x10` (the sort key) then the id. Battle items' keys are their ids (1, 2, 3, 4, 6, 7, 8). | `0x14030EEB0` | Checked (decomp + exe + files) |
| **State 2, commands**: Use (sysmsg 0x91), Move (0x94), Remove (0x92, a filled slot) / Equip (0x95, an empty slot), Discard (0x93). Help lines sysmsg 0x81..0x90 (enabled / greyed, member / stock source: table `0x140548C20`). | `0x14030CB50`, `0x14030CA40` | Checked (decomp + exe) |
| Use: enabled for an item usable now on any member (`0x1402E8B20` → `0x1402CD3A0`: HP items below max HP, Ethers below max MP, Elixirs either); a single-target item opens the target list (state 3), a party item is used at once. The use (`0x14030F160` → `0x1402CD590`): the effect on the party record, the item leaves its slot or the stock; nothing happened → sysmsg 0xA0. | `0x14030CB50`, `0x14030F160`, `0x1402CD3A0` | Checked (decomp) |
| Move: enabled for a kind-0 item **or an empty slot** (`iVar4 = 1` when the item is 0), needing a slot source or an item. Opens state 3: the members, and **the stock only for a slot's item** (greyed with no kind-0 item in stock). | `0x14030CB50`, `0x14030D7B0` case 3, `0x14030D160` | Checked (decomp) |
| Remove: the slot's item to the stock (+1), the slot cleared; refused at 99 (sysmsg 0xA1, error sound). Equip (empty slot): the kind-0 stock list (state 4); nothing in it → sysmsg 0xA2. Greyed for a stock source. | `0x14030CB50` | Checked (decomp) |
| Discard: enabled for a filled row whose kind isn't 13 (`0x1402E8AA0`); a yes / no window (sysmsg 0xA3), its cursor written 0 (`0x1402F10C0`, row 0 = yes: Assumption on which field is the cursor); yes: stock − 1 or the slot cleared. | `0x14030CB50`, `0x14030C8D0` | Checked (decomp) |
| **State 4, the exchanges** (none checks for duplicates: an item can fill several slots): slot → member slot: the two slots swap. Stock item → member slot: the slot's old item to the stock (refused when that is at 99), the stock item − 1 into the slot. Slot → stock item (Move): the stock item into the slot, the slot's old item (if any) back to the stock (refused at 99). Empty slot → stock item (Equip): the item into the slot, then the slot cursor moves to the member's first empty slot. Stock → stock: nothing. | `0x14030FA10`, `0x14030F330` | Checked (decomp) |
| **State 5**: lists rebuilt; back to the source's list (state 1), its cursor kept; to the owners (state 0) when the source was the stock and the stock list is now empty. | `0x14030D7B0` case 5 | Checked (decomp) |
| **Cancel** steps back one window: the destination list → state 3 (Move) or 2 (Equip); state 3 → 2; 2 → 1; 1 → 0; 0 leaves the screen. Sound 3 (back); confirms sound 2; refusals 4. | `0x14030F330`, `0x14030FA10`, `0x14030D160`, `0x14030CB50` | Checked (decomp) |

### Ours, on Re:CoM's camp list

- **The row**: "Items" (help "Equip, use and discard items."), third in the KH1 entry's submenu
  (`keyblades::sub::ROWS`, Re:CoM's Deck-submenu buttons; Stage 7.13's rule: one more row). The
  entry's help is now "Keyblades, shortcuts and items." Only at Combat = KH1 in Sora's story (the
  entry's gate); at the Re:CoM setting nothing of this exists.
- **The page**: the Keyblades / Customize page type (both panels, `CButton` rows, `CButtonCursor`,
  help line; no scroll bar: at most 8 rows), one window at a time: the list shows KH1's current
  window (owners, Sora's slots, the stock, the commands, the destinations, yes / no); confirm opens
  the next, cancel goes back (KH1's stack, cursors kept per window).
- **Rows**: item names are KH1's words (`kh1-command-menu.json`, the battle menu's), else English;
  a stock row adds " xN" when N > 1 (KH1 draws the number beside the name); greyed rows are drawn
  at half of Re:CoM's text colour (**Assumption**: no greyed camp row of Re:CoM's was read).
- **Status panel** (3 label / value rows): Owner, Item (the highlighted or chosen one), In stock
  (its count). KH1 shows the chosen owner / item / command as a header line instead.
- **Help line**: per window and row (our English for KH1's 0x81..0x90 lines), and KH1's refusal
  messages 0xA1 / 0xA2 as our English until the next move.
- **Sounds**: Re:CoM's camp sounds (1 move, 2 confirm / done, 4 back, 5 refused), as Customize.
- **Use**: on Sora at once (KH1 opens the target list for a single-target item; Sora is the only
  member, the owner's choice for Cure / Aero and the battle menu). Effect: HP through Re:CoM's heal
  `0x1401E59A0(player, amount, 0, 0, 0)` (no effect, no popup; the amounts of 8a.1, open question
  1), then Re:CoM's own player → save stats copy `0x1401F0120` (what the camp runs as it opens, so
  the save's HP agrees); MP through KH1's MP (`win::mp::add_points`). Sora's player is
  `[0x14087B390]`; without one (not seen in the field camp) HP items grey out.
- **Store**: every change reads and writes our save block's items extension (layout v2, 8a.1);
  the slot count stays 3 (growth deferred, 8a.1 open question 4).

### New site (`sites::ITEM_PAGE_CALLEES`)

| address | bytes | use |
|---|---|---|
| `0x1401F0120` player → save stats | `48 83 EC 28 48 8B 05 65 B2 68 00 48 85 C0 75 05` | called after a heal from the page (its `mov rax,[rip+…]` reads `0x14087B390`, the heal's player) |

No new hook. The heal `0x1401E59A0` is already a checked callee (M7). All 376 site byte sets
compared with `recomsteam.exe` (pefile): 0 mismatches.

### Per frame

Nothing outside the pause menu. In it, the page runs as Customize does; the inventory (26 bytes)
and Sora's HP / MP are read only when the page opens and after each confirm or cancel.

### Logs

`items page: opened: slots [..] (of 3), stock [(id, n)..], HP below max .., MP below max ..`,
`items page: Removed { item: 1, slot: 0 }; slots [..], stock [..]`, `items page: Used { .. }: HP
120 -> 260 (+140, max 300)`, `items page: refused: StockFull`.

### 1:1 checks

- **Checked (decomp + exe + files)**, replayed by `items_page::tests`: the owners' greying; the
  stock list's kinds and sort; the commands' greying (Use's can-use rule, Move on an empty slot,
  Equip / Remove only for a slot, Discard needs an item); Remove's 99 refusal; Equip's list and
  its jump to the first empty slot; Move's swap and both stock exchanges with the 99 refusal; the
  stock row only for a slot's Move; stock → stock does nothing; Use from a slot and from the stock
  (back to the owners when the stock empties); the discard question (yes first, no back); every
  cancel's target.
- **Not checked**: in game; KH1's own words (sysmsg) and its windows' look.

### Not KH1's (by design or yet)

1. Re:CoM's camp list, one window at a time (KH1 shows its windows side by side and a header
   line); words ours in English (KH1's sysmsg not in the asset step: **Assumption** that "Use /
   Move / Remove / Equip / Discard / Stock / Empty" match KH1's 0x91..0x97 in meaning).
2. No target list for Use (Sora only); party items heal Sora only.
3. No item descriptions (KH1's row `+0xC` text); the help line describes the command instead.
4. KH1's refusal messages are a help line + error sound, not a window to dismiss.
5. KH1's heal sounds (its 0xE / 7) are Re:CoM's confirm sound.
6. The owners' `0x39` row (not items) is not built.
7. The stock list only ever holds the battle items (our store has no other KH1 item).

### What the owner should test

With `kh1_test_items = on` (slots Potion / Hi-Potion / Ether, 9 of each in stock), Combat = KH1:

1. Camp → KH1: three rows Keyblades / Customize / Items; Items' help line; up / down wrap.
2. Items: rows Sora / Stock; the status panel shows Owner, Item, In stock.
3. Sora → his 3 slots with KH1's names; a slot → Use / Move / Remove / Discard. At full HP, Use is
   greyed for a Potion (error sound on confirm).
4. Remove: the slot shows Empty, the stock count rises (In stock). On the empty slot the third
   command reads Equip; Equip → the stock list → pick Ether: the slot holds it, the cursor goes to
   the next empty slot.
5. Move: Sora → another slot: the two swap; Stock → an item: exchanged (the old one back in stock).
6. Stock → an item → Move → Sora → a slot: exchanged. Discard → Yes: one fewer.
7. Get hurt, open the camp, Use a Potion from a slot: HP rises (HUD and Re:CoM's Status page),
   the slot empties. Ether below max MP: the MP bar fills by 3.
8. Close the camp: the battle menu's Items list shows the new slots. Save, reload: the same.
9. Combat = Re:CoM or Riku's story: no KH1 entry (unchanged).

---

## Stage 8a.3: KH1's items in Re:CoM's Moogle shop (not built: findings and options)

Ask: KH1's shop items as purchasable rows in Re:CoM's own Moogle shop screen, priced in Moogle
Points, bought via `pick_up`, native (extend the shop's data / rows), or stop and report options
if the shop can only list card packs without a large rewrite. **Stopped: it can only list card
packs.** Nothing of 8a.3 is in the code.

### KH1's item shop (Steam)

| what | where | tag |
|---|---|---|
| A shop opens with a list (`0x1403010A0(kind, list, …)`, kind 0 = item shop): count + item ids, 0xD4-byte records at `0x1404FEE00`, the record picked by the event script (`param+0x198[…]`; which record when: script data, not read). | `0x1403010A0`, `0x1401CA610` | Checked (decomp + exe) |
| Item shop records: 0 = Potion, Hi-Potion, Ether, Tent (142); 1 = + 4 accessories; 2 = + Camping Set (143) and more; 3 = + Orichalcum (255) and more; 7 = Potion, Hi-Potion, Ether, Tent, Camping Set. **Every item-shop record sells Potion, Hi-Potion and Ether**; no record sells Elixirs or the Mega items (row buy price 0). | exe `0x1404FEE00` | Checked (exe + files) |
| Prices: row `+8` munny: Potion 60, Hi-Potion 150, Ether 200 (Tent 250, Camping Set 500). | `0x1402E88F0` | Checked (files) |
| Buy (`0x1402FED40` / `0x140301660`): a quantity 1 .. min(munny ÷ price, 99 − stock), then the destination: a member (quantity 1, kind 0 only, into his **first empty slot**, refused when none) or the stock (refused past 99). Not KH1's pickup rule (`0x140284E60`): the player picks where it goes. | `0x1402FED40`, `0x140301660` | Checked (decomp) |

### Re:CoM's Moogle shop (Steam)

| what | where | tag |
|---|---|---|
| `CMoogleShopCore` (ctor `0x1403A4220`): 7 modes (init / update / end triples at `+0x98 + 0x40·i`). | `0x1403A4220` | Checked (decomp) |
| Its top menu (mode 2, `0x1403A8770`): **two fixed entries** (`+0x270` 0..1, wrap at 2, cursor y from `0x14065BA30`; entry 0 the packs, 1 leaves); labels are layout art. | `0x1403A8770` | Checked (decomp + exe) |
| The pack screen (mode 4, `0x1403A6930`): **4 tabs** (`+0x280`, wrap 0..3) × up to 4 packs in a **2 × 2 grid** (`+0x284`, moves from table `0x14065BA70`); each pack a `CMShopCardPack` (`0x14045A9E0`: art from `m_shop.lad` by position, its price, sold-out flag). Fixed 4 × 4 arrays in the object: pack types `+0x288`, prices `+0x2C8` (from `MogCardPack`), sold-out `+0x308` (a 16-bit field, `0x140114180` / `0x1401141D0`); the per-tab count `+0x348` from `MogFloor` by floor (`0x1403A5B10`). | `0x1403A6930`, `0x1403A5B10`, `0x14045A9E0` | Checked (decomp + exe) |
| A buy: room for 5 more cards, not sold out, price ≤ Moogle Points (`stats + 0x20`) → **5 random cards from the pack's tables** (`0x1403A66F0` → `0x1403A62B0` / `MogCardKaku`, each through `0x140158FC0`), the pack marked sold out, the "cards got" screen; then Moogle Points − price (`0x14023A1D0`) and the spent total (save var (0, 0xB6), a trophy at 20000). | `0x1403A6930` states 2..4, `0x1403A66F0` | Checked (decomp) |

So every entry the shop lists is a card pack: its picture, its once-per-visit sold-out bit, its
5-card result and the collection-space check are all pack-specific, the tab and grid sizes are
fixed (4 × 4 arrays inside the object), and there is no quantity picker. `MogFloor`,
`MogCardPack` and `MogCardKaku` themselves are not in this session (not read).

### Options (owner question)

- **(a) A fifth tab "Items" in the pack screen** (KH1 setting): hooks on the shop's set-up
  (`0x1403A5B10`), its update (`0x1403A6930`: tab wrap 0..4, the buy branch for item entries:
  Moogle Points check, `pick_up` or KH1's buy rule, no sold-out bit, no 5 cards, no
  collection-space check), the pack object (`0x14045A9E0`) for item pictures; a side table for the
  fifth tab (the object's 4 × 4 arrays are full); new item pictures in `m_shop.lad` from the
  player's KH1 files (asset step). One item per confirm (no quantity). Large: about 4 hooks + new
  art + an asset step change.
- **(b) A third top-menu entry "Items"** (wrap 2 → 3, a third y, a new label in `m_shop.lad`)
  opening a list screen inside the shop: Re:CoM's shop has no list screen, so one would be built
  from Re:CoM's camp list classes (whose sprite set `camp:00` the shop scene may not load:
  Assumption). Largest and riskiest.
- **(c) Re:CoM's own item card packs also give KH1 items**: at the KH1 setting in Sora's story,
  each item card (Potion … Megalixir, kinds 0x2A..0x30) a Moogle pack gives (`0x1403A66F0`'s calls
  to `0x140158FC0`) also gives the matching KH1 item through `pick_up`. One small hook, no art,
  Re:CoM's cards untouched; not KH1's shop (random, from packs, priced per pack).
- **(d) A "Shop" row in the KH1 camp entry** (8a.2's page type) buying KH1's shop items for Moogle
  Points, KH1's buy rule (quantity, destination), only when the camp is opened in a Moogle room
  (room test to research). Re:CoM's camp list, not Re:CoM's shop screen.

Also to decide with the owner:

1. **Price**: Moogle Points 1:1 with KH1's munny (Potion 60, Hi-Potion 150, Ether 200) is the
   **Assumption** on the table; Re:CoM's pack prices (`MogCardPack`) were not read, so no better
   grounded rate is known.
2. **Which items**: KH1 sells Potion, Hi-Potion and Ether in every item-shop stage (Checked); Tent
   / Camping Set / accessories have no Re:CoM use yet.
3. **Where it goes**: KH1's buy rule (quantity, then a slot or the stock) or the brief's `pick_up`
   rule (first empty slot, else the stock).
