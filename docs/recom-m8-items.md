# M8 stage 8a.1: KH1's items (the Items list, slots, stock and the use)

Owner (2026-11-01, "Finish everything up to M9"; M7–M9 choices in
`design-philosophy-instructions.md`): items are **sold in Re:CoM's Moogle shop and dropped by
enemies** (8a.3 / 8a.4); **healing amounts: Re:CoM's own item-card amounts** (see open question 1:
Re:CoM's item cards heal no HP). This stage: KH1's Items list in the command menu, Sora's slots and
stock in our save block, and the use, run by Re:CoM's own item action `0xE`. Not in this stage: the
camp Items page (8a.2, [below](#stage-8a2-kh1s-items-page-in-the-kh1-camp-entry)), the shop (8a.3,
[below](#stage-8a3-kh1s-items-in-recoms-moogle-shop-a-fifth-items-tab)), the drops
(8a.4, [below](#stage-8a4-kh1s-item-drops-from-enemies)).

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
- M9's ability block follows at `save + 0x269F ..= 0x26AD` (`recom-m9-abilities.md`).
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

> Since the owner's ask of 2026-11-01 the KH1 submenu is **Items / Customize** (no Keyblades row)
> and Sora's list on the Items page starts with his **Keyblade slot**: see
> [The Items menu with Sora's equipment](#the-items-menu-with-soras-equipment-owner-2026-11-01).
> KH1's item rules below are unchanged.

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

### The Items menu with Sora's equipment (owner, 2026-11-01)

Ask (owner, 2026-11-01, verbatim): "I would like one menu inside of the "KH1" menu for "Items" in
"items," i want a slot for Keyblade, and all the item slots for items. eventually if we add
accessories as well we will have slots for them there but dont worry about that for now".

Code: `crates/kh1-recom/src/items_page.rs` (`SlotKind`, `SORA_KINDS`, `Slot`, `equipment`,
`row_of`, `Label::Keyblade`, `Outcome::Keyblade`, `Screen::slot`; tested),
`src/keyblades.rs` (`sub::ROWS`, texts), `src/win/keyblades.rs` (`Page::open_keyblades`,
`back_from_keyblades`, `Page::under`).

- **Submenu**: two rows, **Items** then **Customize** (`keyblades::sub::ROWS`); the Keyblades row
  is gone. Re:CoM's Deck-submenu buttons, input, fades and breadcrumb as before (Stage 7.13). Help
  lines: the entry "Sora's Keyblade and items, and the spell shortcuts.", Items "Sora's Keyblade
  and items: equip, use, move and discard."
- **Owners** stay KH1's: Sora / Stock (the stock is an owner in KH1's Items screen, so KH1's flow
  stays whole: owner → row → command; Stock → item → Use / Move / Discard). Sora's owner help: "Sora's
  Keyblade and item slots."
- **Sora's list is his equipment list**, built from a list of slot kinds (`SORA_KINDS`): the
  **Keyblade** (one row), then the **item slots** (one row per slot, KH1's count). Accessories
  (not built) are one more kind between the two; nothing else changes for them.
- **The Keyblade row** shows the equipped Keyblade's name (Re:CoM's card name, as the Keyblade
  list's rows); while it is highlighted the status panel shows its Strength / Magic / Reach and
  the help line reads "Sora's Keyblade. Confirm to change it." Confirm (sound 2) shows **the
  Keyblade list on the same page**: everything the Keyblades page did (earned Keyblades, the
  equipped one marked, the highlighted one's numbers, the scroll bar, confirm equips and stays in
  the list, help "Choose the Keyblade Sora fights with."). Cancel (sound 4) goes back to Sora's
  list on the Keyblade row, now naming the new Keyblade. The Keyblade row needs no item store: it
  opens even when KH1's items aren't live.
- **The item rows** are KH1's as before (confirm → Use / Move / Remove / Equip / Discard; an empty
  slot → Equip from the stock). A Move's destination list ("Choose the slot.") lists the item slots
  only. After an Equip the cursor goes to the first empty slot's row (under the Keyblade row);
  after any change back on the source slot's row.
- **Greying**: KH1 greys a member with no slots (`0x14030D160`); Sora always has his Keyblade
  slot, so his owner row is never greyed now (it was greyed with 0 item slots).
- **Scroll bar**: the Keyblade list's (`0x140112AB0` / `0x1401130C0`, as the Keyblades page made
  it) is created when the list is shown and deleted when it goes back; the Items lists have none
  (Sora's list is 4 rows with KH1's 3 slots).

Not KH1's (by the owner's design): KH1's Items screen has no Keyblade row; KH1 equips weapons on
its own pause-menu screen, not read for this change (**Assumption**: its look and its return after
an equip are not matched; the Keyblade list keeps our Keyblades page's rules, decision 25).

Sites: no new hook, callee or data site (every call was a checked site of the Keyblades page).
All 425 site byte sets and 13 pointers re-read from `recomsteam.exe` (pefile): 0 mismatches.
Checked (exe).

Per frame: unchanged (nothing outside the pause menu); Sora's list reads the equipped Keyblade
(one save-byte read) on each redraw.

Logs: `kh1 entry: submenu opened (2 rows: [Items, Customize])`, `keyblades: list opened from the
Items page: N Keyblades listed [..], equipped 0x..`, `keyblades: equipped card 0x.. (save byte
0x..)`.

1:1 checks: the item rules are 8a.2's (Checked (decomp), replayed by `items_page::tests`, which
now run through Sora's list with the Keyblade row first); the new test
`soras_list_is_the_keyblade_then_the_item_slots` checks the kinds' order, the Keyblade row's
outcome (no change to the inventory, the screen kept), its help, the item rows' commands, the
Move destinations (item slots only) and the cursor's return. Not checked: in game.

Assumptions and risks:

1. Creating / deleting the `CScrollBar` while the page is open (Re:CoM makes it when its page
   opens and deletes it when the page closes): its look when it appears mid-page. Not seen in game.
2. The Keyblade's name in Sora's list as plain text (no "Keyblade" label or icon): the help line
   and the status panel say what it is. To judge in game.

What the owner should test (`kh1_test_items = on` for items in stock):

1. Camp → KH1: two buttons, **Items** then **Customize**; help lines; up / down wrap (held stops at
   the ends); sounds 1 / 2 / 4. Customize works as before.
2. Items → Sora / Stock (unchanged). Sora → first row the equipped Keyblade's name, then the three
   item slots. On the Keyblade row the status panel shows Strength / Magic / Reach.
3. Confirm the Keyblade row: the Keyblade list with its scroll bar, the mark on the equipped one,
   numbers per row; confirm equips (the mark moves); cancel → Sora's list on the Keyblade row with
   the new name. Close the camp: Sora holds the new Keyblade; damage follows its Strength.
4. Item slots: everything of 8a.2's test (Use / Move / Remove / Equip / Discard, the stock) works
   as before; after Equip the cursor lands on the next empty slot; Move → Sora lists only the item
   slots.
5. Close the camp from the Keyblade list with Start, and quit to title with it open: no crash.
6. Combat = Re:CoM or Riku's story: no KH1 entry (unchanged).

---

## Stage 8a.3: KH1's items in Re:CoM's Moogle shop: a fifth "Items" tab

> **Replaced in 0.103.0 by [M8c: KH1 Moogle shop list](#m8c-kh1-moogle-shop-list).** The Items
> tab and its wrapper on the pack screen's update (`0x1403A6930`) are gone: at Combat = KH1 the
> shop opens on a Buy / Sell / Card Packs list, and Card Packs shows Re:CoM's pack screen
> unpatched. The findings below stay true of Re:CoM; the payment (state 4) is still the one the
> list's Buy uses.

Owner (2026-11-01), after the findings below: **a fifth "Items" tab in Re:CoM's pack screen** with
Potion / Hi-Potion / Ether, **one item per purchase**, priced **1 Moogle Point per KH1 munny**
(Potion 60, Hi-Potion 150, Ether 200). Only at Combat = KH1 in Sora's story; the Re:CoM setting and
Riku's story untouched.

Code: `crates/kh1-recom/src/shop.rs` (the rules; tested), `src/win/shop.rs` (Re:CoM's side), sites
`sites::SHOP_HOOKS`, `SHOP_CALLEES`, `SHOP_CHECKED` (test `sites::tests::shop_sites`). No asset step
change.

### KH1's item shop (Steam)

| what | where | tag |
|---|---|---|
| A shop opens with a list (`0x1403010A0(kind, list, …)`, kind 0 = item shop): count + item ids, 0xD4-byte records at `0x1404FEE00`, the record picked by the event script (`param+0x198[…]`; which record when: script data, not read). | `0x1403010A0`, `0x1401CA610` | Checked (decomp + exe) |
| Item shop records: 0 = Potion, Hi-Potion, Ether, Tent (142); 1 = + 4 accessories; 2 = + Camping Set (143) and more; 3 = + Orichalcum (255) and more; 7 = Potion, Hi-Potion, Ether, Tent, Camping Set. **Every item-shop record sells Potion, Hi-Potion and Ether**; no record sells Elixirs or the Mega items (row buy price 0). | exe `0x1404FEE00` | Checked (exe + files) |
| Prices: row `+8` munny: Potion 60, Hi-Potion 150, Ether 200 (Tent 250, Camping Set 500). | `0x1402E88F0` | Checked (files) |
| Buy (`0x1402FED40` / `0x140301660`): a quantity 1 .. min(munny ÷ price, 99 − stock), then the destination: a member (quantity 1, kind 0 only, into his **first empty slot**, refused when none) or the stock (refused past 99). Not KH1's pickup rule (`0x140284E60`): the player picks where it goes. | `0x1402FED40`, `0x140301660` | Checked (decomp) |
| KH1's item pictures: none per item. The menus' item layout (`exchange/UK_uibin_item.bin`) has one 16 × 16 icon (`ite_sel_ico_d`) for every item; KH1's shop is a text list. | `UK_uibin_item.bin` | Checked (files: the H0 dump) |

### Re:CoM's Moogle shop (Steam)

`CMoogleShopCore` (ctor `0x1403A4220`): 7 modes, each a record (init / update / end) at `+0x98 +
0x40·i`; `+0x258` the mode, `+0x25C` the next mode, `+0x260` the state. Mode 3 (`0x1403A53F0` /
`0x1403A8EF0`) is the two-choice menu (packs or mode 5); without room for 5 more cards it goes to
mode 2 (`0x1403A8770`), which offers mode 5 only. Mode 4 is the pack screen. Checked (decomp).

| what | where | tag |
|---|---|---|
| **The pack screen** (mode 4: init `0x1403A4A90`, update `0x1403A6930`, end a bare `ret`): tab `+0x280` (0..3; init sets it and `+0x27C` to 0), cell `+0x284`, `+0x34C` 0 = tab row / 1 = grid, input lock `+0x274` (no reads while > 0). | `0x1403A4A90`, `0x1403A6930`, `0x1403A59B0` | Checked (decomp + disasm) |
| **The data**: 4 × 4 arrays `[tab × 4 + cell]` in the object: pack types `+0x288`, prices `+0x2C8`, sold-out `+0x308`; packs per tab `+0x348`, **the same count for every tab** (`MogFloor` by floor: 1, 2..6, 7..10, 11+), set once at load (`0x1403A5B10`). Tab t holds types 4t..4t+3 (a card category per tab: the card draw `0x1403A62B0` falls back to card 1, 0x1D, 0x2A for tabs 0, 1, 2), so all four tabs are used. | `0x1403A5B10`, `0x1403A62B0` | Checked (decomp) |
| **The packs** `CMShopCardPack` (`0x14045A9E0(mem 0xB8, shop+0x268, cell, price, sold out)`, made in state 0 for cells 0..count−1, `+0x428[cell]`): picture control `+0x78` (layer 0 the frame / its appear and close groups 0x11..0x13, layer 1 the pack art, group `0x140690400[type]` of `m_shop:00`, set by `0x14045D0B0`), sold-out stamp `+0x80`, price plate `+0x90` with its number `+0x98` (`0x14045D120`), `+0x68` close, `+0x6C` release: the pack plays its close and deletes itself (`0x14045C910`). State 0x65 releases them (`+0x6C` = 1, entry cleared). | `0x14045A9E0`, `0x14045C910`, `0x1403A6ABC`, `0x1403A72B8` | Checked (decomp + disasm) |
| **The tab art** `CMShopCardPackWindow` (`+0x418`): one control playing `m_shop:01` group `0x140690420[tab]` (66, 68, 70, 72: the window with tab t in front) and on close `0x140690428[tab]` (67..73), the tab in `+0x5C` (`0x14045D250` replays only on a change). | `0x14045AD20`, `0x14045CC60`, `0x14045D250` | Checked (decomp + exe) |
| **Input state** (case 2): the help line (`0x1403A5AA0` → `0x1404573A0`: 4 usual, 5 no room for 5 cards, 6 not enough points); Square (`0x14029B2F0(0x80)`): sound 3, mode 5, state 100. Tab row: cancel → sound 4, mode 3, state 100; Left (0x408000) tab − 1, Right (0x802000) tab + 1 (sound 1), wrap 3 ↔ 0; Down (0x204000) or confirm → the grid at cell 0 (sound 1; no packs: sound 5). Grid: confirm → room for 5 cards, not sold out, price ≤ points → sounds 2 and 0x58, the Moogle's animation, state 3; else sound 5. Then Down / Right / Up / Left (first pressed), cancel winning over them (back to the tab row, sound 1); moves by the table `0x14065BA70` (0x40 = up out of the grid), only to a cell below the count (sound 1). Then the refresh `0x1403A92F0`. | `0x1403A6BC9..0x1403A6F74` | Checked (disasm) |
| **Refresh** `0x1403A92F0`: the tab's button message at (`0x14065BA40[tab]`, 69), the tab art, the cell's button message at (`0x14065BA50[cell]`, `0x14065BA60[cell]`), each pack's type / price / sold-out from the tab's arrays, the cursors: in the grid the tab mark `+0x3D8` at (tab x − 16, 53) and the finger `+0x3D0` at (cell x − 16, cell y − 32); in the tab row the finger at (tab x − 16, 53), the mark hidden. Tab x 64 / 128 / 192 / 256, cells (74, 167), (224, 167), (74, 252), (224, 252). | `0x1403A92F0`, exe `0x14065BA40..0x14065BA7F`, `0x14065BCE0` | Checked (disasm + exe) |
| **A buy** (states 3, 4): the pack marked sold out, **5 random cards** (`0x1403A66F0` → `0x1403A62B0` / `MogCardKaku`, each through `0x140158FC0`), the "cards got" screen; then the payment: points − price clamped 0..99999; while the trophy bit `0x140AF7389 & 0x10` is clear, the spent tally (system variable (0, 0xB6), `0x1401B08C0` / `0x1401B16C0`) += price, the trophy (`0x1402BA6F0(*0x140AF7468, 99)`) when it first reaches 20000, and the bit set; Re:CoM's setter `0x14023A1D0` (`stats + 0x20`, clamped; its own `0x1400D3830(0x2B)` on first passing 9999); the Moogle Point window `0x1404598A0(shop+0x3C0, points)`. | `0x1403A6F74..0x1403A7106`, `0x14023A1D0` | Checked (decomp + disasm) |
| **Re:CoM's card picture** `C2DCard` (`0x1402432A0`, 400 bytes, vtable `0x140606BF8`, deleting dtor `0x1402073D0`): set a card `0x140243560(card, value)` (byte 0 kind, byte 2 negative = no number, byte 3 the class from `0x14068E0B0 + kind × 16`: 0x44 for 0x2A / 0x2B / 0x2D), priority `0x140243380`, attach to a control `0x140243DD0`, show `0x140243A60`. The shop uses it itself (`0x14045BF50`: a number-less card `0x11FF0001` at priority 0x12 on its window; the "cards got" screen `0x1404589B0`). | `0x140243560`, `0x14045BF50`, `0x1404589B0` | Checked (decomp + disasm + exe) |

`MogFloor`, `MogCardPack` and `MogCardKaku` (`FORM/0109/FO0109.BIN`) and `m_shop.lad` /
`m_shop.imz` (`FO0109.SPR` / `.RTM`) are **not in this session** (the H0 dump has `FORM/0109` listed,
not dumped): pack counts per floor, the pack prices and the art are not read.

### What runs (at the KH1 setting, in Sora's story)

A wrapper on the pack screen's update `0x1403A6930`. Re:CoM runs the screen; the wrapper acts only
in the input state (2) at the tab row's edges and while the Items tab is up, and as the screen is
left with it up.

1. **Into the Items tab**: in the tab row, Left on tab 0 or Right on tab 3 (Re:CoM's earlier reads
   win: Square, cancel, Left over Right; nothing while `+0x274 > 0`). Sound 1 (Re:CoM's tab sound),
   the usual help line. Re:CoM's packs are released as state 0x65 releases them (`+0x6C` = 1: each
   plays its own close and deletes itself); three packs are made with **Re:CoM's own constructor**
   as state 0 makes them (cells 0..2, the item's price, not sold out); the count `+0x348` becomes 3
   (the fourth cell stays empty: Re:CoM's move rule never reaches a cell at the count), the tab
   `+0x280` = 4. Each pack's art layer (picture control layer 1) is hidden and **Re:CoM's own card
   picture of the item's card** (`C2DCard`: Potion 0x2A, Hi-Potion 0x2B, Ether 0x2D, no number) is
   attached to its picture control at (0, 0), priority 0x12 (the shop's own card priority).
2. **On the Items tab** (`crate::shop::items_step`, Re:CoM's case 2 in Re:CoM's order): Square
   leaves to mode 5, cancel to mode 3 (sounds 3 / 4, state 100); Left → Re:CoM's tab 3, Right → tab
   0; Down or confirm → the grid at cell 0; in the grid confirm buys, cancel or Up from the top row
   goes back to the tab row, moves by Re:CoM's table over 3 cells. The help line: 6 (not enough
   Moogle Points) on an item Sora can't afford, else 4. Each frame Re:CoM's refresh is replayed for
   the Items tab: button messages, prices (never sold out), cursors at x 320.
3. **A buy** (confirm on a cell): price ≤ Moogle Points (else sound 5, nothing changes); the item by
   **KH1's pickup rule** (`Inventory::pick_up`: the first empty slot, else the stock up to 99; else
   sound 5, nothing paid); then sounds 2 and 0x58 (Re:CoM's buy sounds) and **Re:CoM's payment**
   (the spent tally and its trophy, `0x14023A1D0`, the Moogle Point window). One item per confirm,
   never sold out. No room-for-5-cards check, no card draw, no Moogle animation, no "cards got"
   screen.
4. **Back to Re:CoM's tab**: the cards are deleted, our packs released, Re:CoM's packs for that tab
   made again as state 0 makes them (its prices and sold-out marks), the count and tab given back,
   then **Re:CoM's own refresh** `0x1403A92F0` (pictures, tab art, cursors).
5. **Leaving the screen from the Items tab** (Square, cancel): the cards are deleted at state 100
   (before Re:CoM starts closing the packs they hang from); Re:CoM's states 100 / 0x65 close and
   release our packs as its own; after 0x65 the count is Re:CoM's again. The next visit starts on
   tab 0 (the mode's init).

### The tab art (not built)

Re:CoM's tab art is one picture per tab (the window with that tab in front, `m_shop:01` groups
66..73); a fifth tab needs new art, and `m_shop.lad` / `.imz` are not in this session. Until then
the **window keeps the art it showed** (the tab the player came from) while the Items tab is up;
the finger cursor and the tab's button message sit at x 320 (Re:CoM's tab spacing of 64 continued:
**Assumption**). Next step, to plan with the owner: a read-only dump of `FORM/0109` (the H0 tool's
FORM dump) to see the tab groups and design the Items tab (owner question 1).

### Pictures: Re:CoM's item cards

KH1 has no per-item picture (one 16 × 16 icon for every item: Checked (files)), so the cells show
**Re:CoM's own Potion / Hi-Potion / Ether card faces**, drawn by Re:CoM's card picture class with
its own card art (the shop already draws cards: the "cards got" screen, the stock window). No
asset step. **Assumption**: the card sets (`card_00..02`) are loaded on the pack screen (the "cards
got" screen, reached from it, draws them); the card's size and centre against the pack frame are
not seen (the card is attached at the picture control's origin).

### Sites

| address | bytes | use |
|---|---|---|
| `0x1403A6930` pack screen update | `48 8B C4 56 57 41 54 41 56 41 57 48 83 EC 50` (15) | **hooked** (whole instructions, no RIP-relative) |
| `0x1403A92F0` refresh, `0x14045A9E0` pack ctor, `0x14045D120` price, `0x14045D3F0` sold out, `0x1404573A0` help line, `0x140459AF0` message place, `0x140459A00` cursor place, `0x1404597D0` cursor show, `0x1404598A0` point window, `0x1404598D0` show state (written directly), `0x14023A1D0` points set, `0x1401B16C0` variable set, `0x1402BA6F0` trophy, `0x1402432A0` / `0x140243560` / `0x140243380` / `0x140243DD0` / `0x140243A60` / `0x1402073D0` C2DCard | 8..18 bytes each | called (`SHOP_CALLEES`) |
| `0x1403A6BC9`, `0x1403A6C1B`, `0x1403A6D74`, `0x1403A6E54`, `0x1403A7041`, `0x1403A6ABC`, `0x1403A72B8` | the input state, the payment, the packs' making and release | checked only (replicated; the test reads their calls, masks, sounds and constants) |
| `0x14065BA40` (64), `0x14065BCE0` (8), card class rows `0x14068E350` / `E360` / `E380` (8 each) | the places and moves, the cursor and message y, the classes | checked only |

Reused, already checked: `0x1404ACE54` (new), `0x1402AA360` (sound), `0x14029B2E0` / `B2F0` /
`B410` / `B490` (pad), `0x1401B08C0` (variable get), `0x140247E50` (layer visible), `0x1402485B0`
(position). Read at run time: the card's vtable after its ctor (`0x140606BF8`) and its slot 0
(`0x1402073D0`) before the delete. All 408 site byte sets compared with `recomsteam.exe` (pefile):
0 mismatches.

### Per frame

Nothing outside the shop's pack screen. In it, at Re:CoM's tabs: one mutex read a frame, and in the
input state the gate (`mp::active`) and, at tab 0 / 3 in the tab row, Re:CoM's pad reads once more.
On the Items tab: Re:CoM's input state and refresh replayed (a few calls a frame).

### Logs

`shop: KH1's items in the Moogle shop (M8 stage 8a.3): …` and `hooked pack screen update …` at
start; then (60 lines at most) `shop: Items tab from tab 3: Re:CoM's 2 packs released, …; cards
[…]; 500 Moogle Points`, `shop: bought potion for 60: 500 -> 440 Moogle Points, Slot(0)`, `shop:
refused on cell 2 with 100 Moogle Points: Poor { … }` / `NoRoom { … }`, `shop: Items tab -> tab 0:
Re:CoM's 2 packs made again`, `shop: pack screen left from the Items tab: Re:CoM's count 2 back`.

### 1:1 checks

- **Checked (decomp + disasm + exe)**, replayed by tests (`shop::tests`, `sites::tests::shop_sites`):
  Re:CoM's input order (Square, cancel, Left, Right, Down / confirm; confirm, then cancel over the
  directions in the grid), its sounds (1, 2 + 0x58, 3, 4, 5), the move table and the 0x40 exit, the
  places, the help ids, the payment (clamp, tally, the 20000 trophy once); KH1's prices (60 / 150 /
  200, Checked (files)), KH1's pickup rule (slots, then stock to 99, refused: nothing paid); the
  items KH1 sells everywhere.
- **Not checked**: in game (the cards' look and place on the packs, the window art while the Items
  tab is up, the packs' close / appear when switching tabs).

### Not KH1's (by the owner's choice or not yet)

1. KH1's buy: a quantity and a destination (a member's slot or the stock). Here one item per
   confirm, by KH1's pickup rule (owner: one per purchase; the destination window has no Re:CoM
   counterpart in the pack screen).
2. Prices in Moogle Points 1:1 with munny (owner).
3. No Items tab art yet (above); the cards are Re:CoM's item cards.
4. The buy plays Re:CoM's pack-buy sounds (2, 0x58) without the Moogle's animation.
5. The spent tally and its trophy count item buys too (Re:CoM's payment path kept whole: owner
   question 2).

### Assumptions

1. The pack screen is always left through states 100 → 0x65 → 0x66 (every exit in its code does);
   a shop torn down another way (a reset) while the Items tab is up would leave the cards' parent
   controls freed under them.
2. The card sets are loaded on the pack screen; the card sits well at the picture control's origin.
3. x 320 for the Items tab (no art to place it by).
4. The pad reads (`0x14029B2E0`, `0x14029B2F0`, `0x14029B410`, `0x14029B490`) are pure: reading them
   before Re:CoM's update changes nothing (Re:CoM itself reads each several times a frame).

### Open questions (owner)

1. **The Items tab's art**: (a) plan a read-only dump of `FORM/0109` (`m_shop`) and design a fifth
   tab from Re:CoM's own tab pieces with KH1's word "Items" (asset step); (b) keep the window as it
   is (the cursor alone marks the fifth tab).
2. **The spent tally / trophy**: (a) count item buys (built: Re:CoM's payment as it is); (b) packs
   only.
3. **No room for 5 cards**: Re:CoM then never offers the pack screen (mode 3 → mode 2), so the Items
   tab is out of reach too. (a) leave it (built); (b) offer the pack screen anyway at the KH1
   setting (the packs refuse as Re:CoM does, the Items tab works).
4. **The refusal when Sora's items are full**: sound 5 only (Re:CoM's message 5 is about cards).
   (a) keep; (b) a help line of ours.

### What the owner should test

Combat = KH1, Sora's story, a Moogle room with some Moogle Points (`kh1_test_items` off to see the
slots fill; on to see the stock rise).

1. Log: the `shop:` start line and `hooked pack screen update (0x1403A6930)`.
2. Moogle → the packs: on the last tab press Right (or Left on the first): a pack-switch sound, the
   packs close and three open with Potion, Hi-Potion and Ether **card pictures** and prices 60, 150,
   200; the finger at a fifth place right of the tabs. Log `Items tab from tab …`.
3. Down → the grid at Potion; moves among the three; the fourth cell can't be reached; Up from the
   top row and cancel go back to the tabs. With too few points the help line reads Re:CoM's "not
   enough points" line on that item.
4. Confirm on Potion: Re:CoM's buy sounds, the Moogle Points drop by 60 at once, log `bought potion
   …, Slot(0)`. Buy until the slots fill: then `Stock(n)`. With 99 in stock and full slots: the
   error sound, no points taken. Too few points: the error sound.
5. Right / Left from the Items tab: back to Re:CoM's tabs with their packs and sold-out marks as
   before. Square and cancel from the Items tab leave as from any tab; come back: tab 1, Re:CoM's
   packs.
6. Close the camp's Items page / battle menu: the bought items are there; save, reload: kept.
7. Combat = Re:CoM or Riku's story: four tabs, Re:CoM's shop as before.
8. Report how the cards look on the packs (size, place) and the window art while on the Items tab.

## Stage 8a.4: KH1's item drops from enemies

Ask (M8 plan row 8a.4; owner's M7–M9 choice "dropped by enemies (KH1's rates)"): when a Re:CoM
enemy is defeated at Combat = KH1 in Sora's story, roll its KH1 twin's item drops as KH1 does and
make each item a prize through Re:CoM's own prize system; its pickup is KH1's rule (`pick_up`).
Enemies without a twin drop nothing. The Re:CoM setting is untouched.

Code: `crates/kh1-recom/src/drops.rs` (KH1's roll, the new prize kind, the look row, the prize
descriptor; tested), `src/win/drops.rs` (Re:CoM's side), `src/attack.rs` (`Kh1Stats::drops`, the
stats file's `drops`), `src/win/stats.rs` (`drops(enemy)`), `src/win/hook.rs` (`place_data_near`).
Asset step: `kh1-recom-assets` `stats` (`Twin::drops`, `DropTable`, `drop_table`,
`drop_offset_for`; `kh1-stats.json` format 6).

### KH1 (Steam)

| what | where | tag |
|---|---|---|
| **The drop**: an enemy's AI script, on defeat, hands its prize table and its position to syscall **122** (`0x1402BC640`, `param_3` = `0x14298C9A0 ≠ 0`) or **215** (`0x1402BC6B0`, `param_3` = 1); both call `0x1402ABC40(table, position, param_3)` (syscall table `0x140529180` entries 122 / 215, 2 arguments). | `0x1402BC640`, `0x1402BC6B0` | Checked (decomp + exe) |
| **`0x1402ABC40`**: nothing while `0x142D60FAC` ≠ 0 (set by `0x1402AB8E0`; its meaning not read); seven i32 prize counts at `+0x00..+0x1B` (each × the Jackpot factor `0x142D60FA4`, HP / MP / munny orbs from the prize pool `0x142D60FB0`); then **three (f32 chance, i32 item) pairs** at `+0x1C`, `+0x24`, `+0x2C`, in order: `movss xmm6,[0x142D60FA8]; mulss xmm6,[rbx−4]; call 0x1401E0EB0; comiss xmm6,xmm0; jbe` → the item `0x1402BFF00(item, position, param_3)` when **random < chance × Lucky Strike**. One random draw per pair. | `0x1402ABC40` (`0x1402ABE50..0x1402ABE7C`) | Checked (decomp + exe) |
| **Random** `0x14031BA30`: three `rand()` bytes → a 24-bit integer × 2^−24 (5.96e−8): [0, 1). | `0x14031BA30` | Checked (decomp) |
| **Lucky Strike** `0x142D60FA8` = 1 + 0.5 × `0x140284BE0(0x1C, member)` per party member (`0x1402A6380`; the Jackpot factor the same with `0x1B`). The ability names are an Assumption (the brief's). | `0x1402A6380` | Checked (decomp) |
| **The item** `0x1402BFF00`: at most 8 dropped items (`0x142DE8E20`, 0x50 each; with 8 out, the oldest is removed); the model from the item row's byte 3; a random scatter. They don't time out. | `0x1402BFF00`, `0x1402C0160` | Checked (decomp) |
| **The pickup** `0x1402C08D0`: `0x140284E60(item, Sora's member)` (8a.1's rule); taken → the "obtained" window `0x140273410` and sound `0x1B`; refused → the "full" notice and a retry every 60 frames (the item stays). | `0x1402C08D0` | Checked (decomp) |
| **Objects** (barrels, pots) drop through `0x1402AB940` from `map_prize.bin` (13-byte rows: 7 count bytes, 3 × (chance %, item)), not enemies. Not built (no Re:CoM objects). | `0x1402AB940`, `0x1402ABBB0` | Checked (decomp) |

### Where each twin's table is. Checked (files)

The table is script data: the defeat routine pushes its address (`pusha pc:X`, the word `0x00E2`)
and calls a 2-argument subroutine that runs `sys32` (the actor's position) then `sys122`. Found by
evaluating each twin's script (the research notes' VM, `/home/claude/re/scratch/estats`, extended
to stop at syscall 122 / 215; outside the repo) from every routine that reaches a `sys122`, for
every KH1 world × room 0..63 × variant 0..25 × LV1 / LV2. **The defeat path is the one that also
runs `sys167` (EXP on defeat).** No table depends on the LV2 bit; only the Shadow's depends on
the world.

| twin | table | KH1 battle items (rate) | other KH1 items (not made here) |
|---|---|---|---|
| Shadow | `0xA44`; Destiny Islands `0xAAC`; Dive to the Heart `0xA78`; End of the World rooms ≥ 37 `0xAE0` | Potion 1 %; Destiny Islands Potion 5 %; Dive none | #233 3 % |
| Soldier | `0x38C` | Potion 2 % | #17 1 %, #236 6 % |
| Large Body | `0x3FC` | Hi-Potion 4 % | #18 1 %, #236 10 % |
| Red Nocturne | `0x2AC` | Ether 1 % | #21 0.5 %, #241 6 % |
| Blue Rhapsody | `0x23C` | Ether 1 % | #24 0.5 %, #243 12 % |
| Yellow Opera | `0x2AC` | Ether 1 % | #27 0.5 %, #245 6 % |
| Green Requiem | `0x15C` | Ether 4 % | #18 0.5 %, #248 10 % |
| Powerwild | `0x3FC` | Potion 2 % | #18 0.5 %, #238 4 % |
| Bouncywild | `0x23C` | Hi-Potion 2 % | #18 1 %, #238 8 % |
| Air Soldier | `0x54C` | Potion 2 %, Hi-Potion 1 % | #237 4 % |
| Bandit | `0x46C` | Potion 2 % | #18 0.5 %, #242 4 % |
| Fat Bandit | `0x3FC` | Hi-Potion 4 % | #22 1 %, #242 8 % |
| Barrel Spider | `0x70C` | – | #142 1 %, #143 0.5 %, #253 1 % |
| Search Ghost | `0x54C` | Hi-Potion 2 %, Mega-Potion 1 % | #249 4 % |
| Pirate | `0x54C` | Hi-Potion 2 %, Mega-Potion 1 % | #239 4 % |
| Air Pirate | `0x1CC` | Hi-Potion 2 %, Mega-Potion 1 % | #239 4 % |
| Wight Knight | `0x5BC` | Hi-Potion 1 %, Mega-Potion 0.5 % | #234 2 % |
| Gargoyle | `0x4DC` | Ether 1 %, Mega-Ether 0.5 % | #234 2 % |
| Darkball | `0x2A8` (End of the World ≥ 37: `0x2DC`) | Hi-Potion 1 % | #235 1 % |
| Defender | `0x46C` | Elixir 1 % | #131 0.2 %, #250 2 % |
| Wizard | `0x38C` | Ether 2 % | #115 0.2 %, #247 2 % |
| Sea Neon | `0x23C` | Ether 0.5 % | #25 0.1 %, #244 2 % |
| Aquatank | `0x31C` | Ether 4 % | #25 1 %, #244 8 % |
| Screwdiver | `0x31C` | Mega-Potion 4 % | #28 1 %, #246 8 % |
| Wyvern | `0x31C` | Hi-Potion 2 %, Elixir 0.5 % | #240 2 % |
| Black Fungus | `0x1D0` (all zero) | – | – |
| Creeper Plant | `0x4DC` (End of the World ≥ 37: `0x510`) | Hi-Potion 2 % | #252 4 % |
| Neoshadow | `0x2E4` | – | #16 35 % |
| White Mushroom, Spade / Heart Card Soldier | none | – | – |

- Re:CoM worlds KH1 lacks (Twilight Town, Castle Oblivion) take the general table (the tables
  don't depend on LV2). End of the World and Dive to the Heart are no Re:CoM world, so their
  tables are never used.
- **Second tables, not the defeat**: Powerwild `0x430` / Bouncywild `0x270` (20 HP and 5 MP
  orbs, Ether 20 %, Mega-Ether 4 %) are handed over by a routine with no EXP (what triggers it is
  not read); White Mushroom's only table `0x254` (#159 100 %) likewise; Black Fungus's `0x204` /
  `0x238` / `0x26C` go by the finishing move (its spell game). Not built (owner question 2).
- Item rows (btltbl `0x1A58`): 1..8 battle items (kind 0); 9..16 kind 13; 17..29 kind 1
  (accessories); 142 / 143 kind 12 (Tents); 233..255 kind 13 (synthesis materials). Checked (files).

### Re:CoM (Steam)

| what | where | tag |
|---|---|---|
| **Defeat prizes** (`0x14017BBC0` and the linked-part path `0x140181300`; skipped with `+0x260` bit 22): the EXP orbs `0x1401810A0`, the roulette prizes `0x140186480` (0xD6) / `0x1401867F0` (0xD7), then **the card prize `0x140180D20(enemy, &point)`** last. Its card drops only when `0x1401860A0` allows (the manager's enemy count at 0, or a test on its `+0xF4` object; not read further), but it is called on every defeat; every prize of Re:CoM's needs the enemy's `@ENP` row `+0x310`. | `0x14017BBC0`, `0x140181300`, `0x140180D20` | Checked (decomp) |
| **Prize descriptor** (the EXP drop's, `0x14018112D..`): `+0x00` the point, `+0x10` angles (pitch = (−70 − r × 10)°, yaw = r × 360°, 0, 1; `0x140293F20` normalises them), `+0x20` kind, `+0x24` argument, `+0x28` speed 0.25, `+0x2C` 1. Random `0x14029CF00` = u32 × 2^−32. Then `0x1401A89C0(0x1401A8A80(), &desc, 0)`. | `0x1401810A0` | Checked (decomp + exe) |
| **`CPrize`** (factory `0x1401A89C0`: 0x300 bytes, ctor `0x14022E8D0`, init `0x1402315B0`; parent: the battle manager task 0x19 when there is one). Init: `+0x244` kind, `+0x248` argument, `+0x24C` speed; kinds ≥ 0xCB: the look from **`0x1405B3150[kind − 0xCB]`** (16-byte rows: the set name `rec_prise`, effect id, scale; 13 rows) and the state machine **`0x140231010`**. | `0x1402315B0` | Checked (decomp + exe) |
| **`0x140231010`**: state 0: 30000 ticks (10 s) on the ground, then a 1500-tick fade and gone; collected when Sora comes in range (`0x140230C70`) → state 1: flies to him, the pickup sound `0x140230F00` (0xCB..0xD5 their own, **any other kind 6**), at the end the reward `0x140230F90` → **`0x1401F4AB0(kind, +0x248, 1)`**; then gone. The bounce sound `0x140230E70` only for 0xCB..0xD5. | `0x140231010` | Checked (decomp + exe) |
| **Prize add `0x1401F4AB0`**: kinds ≤ 0xCA cards; 0xCB..0xD7 a switch (`add r9d,−0xCB; cmp r9d,0xC; ja default`): EXP, HP, Moogle points, the roulette flags; **above 0xD7: the default, nothing**. | `0x1401F4AB0`, `0x1401F503D` | Checked (decomp + exe) |
| The look rows (kinds 0xCB..0xD7 → `rec_prise` ids 8, 7, 0, 0xA, 9, 1, 2, 0xB, 3, 0xC, 4, 5, 6; scales 0.25..0.425). The 16 bytes after them (`0x1405B3220`) are other data (`0x140220000.c` reads them as floats): the table can't grow in place. Only `0x1402315B0` and `0x1402306D0` (uncalled) index it. | `0x1405B3150` | Checked (exe + decomp) |
| **No item notice**: Re:CoM shows a card got (`0x14020D940`, cards only) or a heal's "Recover"; it has no notice for anything else a prize gives. | `0x1401F4AB0` | Checked (decomp) |

### What runs (at the KH1 setting)

1. **The look row** (once, at start): Re:CoM's 13 look rows are copied (each checked to name
   `rec_prise`) with a 14th row after them, kind **0xD8**'s, a copy of **0xD6's** row (`rec_prise`
   effect 5, scale 0.25); the copy sits within rel32 reach and the two `lea rax,[rip+…]`
   (`0x140231676` in the prize init, `0x140230700` in its uncalled look set-up) point at it.
   Re:CoM's kinds read the same rows as before (at either setting).
2. **The roll** (wrapper on `0x140180D20`, after Re:CoM's own): at Combat = KH1 in Sora's story,
   for an enemy with an `@ENP` row whose class has a KH1 twin with a table for Re:CoM's current
   world: KH1's roll (one Re:CoM random float a pair, in order; drops when below chance × Lucky
   Strike = 1, no ability yet).
3. **The prize**: each **battle item** that drops (Potion … Megalixir; other KH1 items are logged,
   not made) is made by Re:CoM's factory at the enemy's prize point, launched as Re:CoM's EXP
   orbs are, kind 0xD8, argument = KH1's item id. Re:CoM steps, draws, collects, times out and
   deletes it (`0x140231010`), as a child of the battle manager like its other prizes, so a battle
   ends as it does with EXP orbs (collected or 10.5 s at most).
4. **The pickup** (wrapper on `0x1401F4AB0`): kind 0xD8 → `win::items::pick_up(item)` (KH1's
   `0x140284E60`: the first empty slot, else the stock below 99); every other kind → Re:CoM's.
   Re:CoM's pickup sound (6, the cards') plays in its own state machine. No notice (Re:CoM has
   none for this; KH1's "obtained" window is out of scope).

### Sites (`sites::DROP_*`, bytes from the Steam exe)

| address | bytes | use |
|---|---|---|
| `0x140180D20` defeat card prize | `40 55 56 57 48 8D 6C 24 B9` | **hooked** (push rbp; push rsi; push rdi; lea rbp,[rsp−0x47]) |
| `0x1401F4AB0` prize add | `48 89 5C 24 10 57 48 83 EC 30` | **hooked** (mov [rsp+0x10],rbx; push rdi; sub rsp,0x30) |
| `0x140231676` prize init's look `lea` | `48 8D 05 D3 1A 38 00` | patched: → the table copy |
| `0x140230700` look set-up's `lea` | `48 8D 05 49 2A 38 00` | patched: → the table copy |
| `0x1401A89C0` prize factory | `40 57 48 83 EC 30 48 C7 44 24 20 FE FF FF FF` | called |
| `0x140293F20` angle normalise | `40 53 48 83 EC 20 F3 0F 10 01 48 8B D9` | called |
| `0x14029CF00` random float | `48 83 EC 28 E8 C7 FE FF FF 0F 57 C0 8B C0 F3 48 0F 2A C0` | called |
| `0x140231664`, `0x14023167D`, `0x1402316D6`, `0x1402306F9`, `0x140230707` | the look index / row reads | checked only |
| `0x140230F90` reward (40 bytes), `0x1401F503D` the add's switch (17), `0x140230E70` / `0x140230F00` the sound switches (20 / 22), `0x14018112D` the EXP drop's descriptor (20) | | checked only |

The entity factory getter `0x1401A8A80` was already checked (`MAGIC_CALLEES`). All 393 site byte
sets compared with `recomsteam.exe` (pefile, `python3 -I`): 0 mismatches. Read at run time: the 13
look rows (each must name `rec_prise`).

### Per frame

Nothing. The roll runs once per enemy defeat (a class lookup and three random draws), the pickup
once per collected prize (one compare for every other prize kind).

### Logs

- Start: `drops: KH1's item drops (M8 stage 8a.4) at combat = kh1 in Sora's story: N twin classes
  with KH1 defeat tables; …; Re:CoM prize kind 0xd8 (look: kind 0xd6's rec_prise row, table at
  0x…)`, the two `hooked …` lines. Without format 6: `drops:
  kh1-stats.json has no drop tables (format 6: re-run kh1-recom-build); no KH1 drops`.
- Up to 120 lines: `drops: CEnemySoldier (world 1; potion 2%, #17 1%, #236 6%): potion dropped
  at (x, y, z): prize 0x…`, `… KH1 item #236 dropped; not one of the mod's items: no prize`,
  `… refused …: lost`; and 8a.1's `items: potion picked up (enemy drop): slot 2`.
- The battle-end log (Stage 75) lists a waiting prize's kind: `kind 0xd8` is ours.

### 1:1 checks

- **Checked (decomp + exe)**: the table layout, the roll (one draw a pair, in order, strictly
  below chance × Lucky Strike, Lucky Strike 1 without the ability), the pickup rule; replayed by
  `drops::tests` (the Air Soldier's table, Lucky Strike, the rate over 10 000 uniform draws = the
  chance).
- **Checked (files)**: every twin's defeat table, read from the session's KH1 files and validated
  as the asset step does (the script pushes it, counts 0..255, chances 0..1, items 0..255);
  `stats::tests` on synthetic scripts.
- **Not checked**: in game; KH1's random generator is not reproduced (Re:CoM's random float
  instead; the rates are KH1's, not the sequence).

### Not KH1's (by design or yet)

1. **The look**: Re:CoM's `rec_prise` effect of the rare roulette prize (0xD6), not KH1's item
   model. Re:CoM has no item prize. **Assumption** that it reads as an item (owner question 1).
2. **The scatter and the life**: Re:CoM's launch (as its EXP orbs) and Re:CoM's 10 s on the
   ground, then gone; KH1's items scatter its own way, stay until picked up, at most 8 at once.
3. **Refused pickup** (stock at 99 and every slot full): the prize is gone (Re:CoM collects a
   prize once); KH1 leaves it on the ground with a "full" notice.
4. **No "obtained" notice** (Re:CoM has none for this; KH1's window is out of scope), and the
   pickup sound is Re:CoM's (6), not KH1's `0x1B`.
5. **Only battle items** are made: KH1's other drops (synthesis materials, accessories, Tents, the
   rare #115 / #131, #16) have no place in the mod yet (rolled, logged, not made).
6. **Boss twins drop nothing** (their KH1 tables, e.g. Trickmaster's and Parasite Cage's
   `sys122` calls, were not read; owner question 3); proxies (enemies without a twin) drop
   nothing (the brief).
7. KH1's prize counts (HP / MP / munny orbs) are not used: Re:CoM's own EXP / HP / Moogle prizes
   stay Re:CoM's.
8. A drop happens on every defeat path Re:CoM gives prizes on (`+0x260` bit 22 clear and an
   `@ENP` row); KH1 skips drops while `0x142D60FAC` is set (meaning not read).

### Assumptions

1. 0xD6's look reads as "an item" (Re:CoM's `rec_prise.dpx` not seen).
2. Re:CoM never makes a prize of kind 0xD8 itself (its prize add has no case and its look table no
   row above 0xD7: Checked (exe); the factory's 40 callers' kinds and the event VM's prize ids
   were not all read).
3. The ability names Lucky Strike (`0x1C`) / Jackpot (`0x1B`) (the brief's; the factors are
   Checked).
4. The tables' meaning "defeat" = the path that also grants EXP (`sys167`).

### Open questions (owner)

1. **The look**: (a) 0xD6's `rec_prise` effect (built); (b) 0xD7's (effect 6); (c) an HP orb's
   look tinted (needs new effect data); (d) KH1's item models, converted at the asset step (a new
   effect / model resource for Re:CoM; larger).
2. **Powerwild / Bouncywild's second tables** (Ether 20 % with HP / MP orbs, not on defeat): (a)
   leave out (built); (b) research when KH1 hands them over and build it.
3. **Bosses with a KH1 equivalent**: (a) no drops (built); (b) their KH1 defeat tables (to read).
4. **Non-battle items** (synthesis materials etc.): (a) not made until the mod has them (built);
   (b) keep them in the stock bytes for later (the save extension holds ids 1..16 only).

### What the owner should test

Re-run the asset step (`kh1-recom-build`, same command): `kh1-stats.json` must say `stats 6`, and
the console lists `drops <class> : items …` lines. Combat = KH1, Sora's story.

1. Log: the `drops: KH1's item drops …` start line (with the table's address), `hooked defeat card
   prize`, `hooked prize add`.
2. Fight Soldiers / Air Soldiers / Large Bodies until something drops (1–4 % each: about one in
   30–50 defeats): a `rec_prise` effect pops out where the enemy fell, among the EXP orbs. Log
   `drops: … potion dropped …`.
3. Walk into it: Re:CoM's pickup sound, then the Items list (or the camp's Items page) has it
   (first empty slot, else the stock). Log `items: potion picked up (enemy drop): slot …`.
4. Leave one on the ground: it fades after about 10 s; the battle ends normally (Sora returns to
   the field), whether picked up or not. If Sora freezes at the battle end, send the log (the
   battle-end lines list each waiting `CPrize` with its kind).
5. Fill all slots and 99 of an item in stock, pick that item up: log `refused … lost`.
6. Combat = Re:CoM: no item prizes; EXP / HP / Moogle prizes and card drops as before.

## Row looks: the item symbol and the green box (owner, 2026-11-01)

Ask (owner, 2026-11-01, verbatim): "lets make "stock" have the item symbol, and make the box green";
and every item slot row of Sora's list likewise. The Keyblade slot and the "Sora" row stay.

- **How Re:CoM picks a camp row's box and icon** (Checked (decomp)): the list rows are `CButton`s
  set up by `0x140112F10`; its last argument (a short) is the row's **variant**, and the row plays
  sequence `seqs[variant]` of the table it is given (`*(u16*)(seqs + 2 × variant)`, the
  `+8` call of its `C2DSeqCtrl`). The camp list's table is `0x1407C11F0` = 0x12E, 0x133, 0x131,
  0x132, 0x12F, 0x160, 0x130 (Checked (exe)). Re:CoM's Sleights page sets each row's variant from its
  category (`0x1400E6DF0` mode 9 reads the list entry's second byte; mode 10 fills it from the
  sleight's kind: Sora 0..3, Riku 4..7). Our pages passed 0 for every row: the red box.
- **What the sequences draw** (`camp.lad` in `FO0100.SPR`, Checked (files)): each is the same
  plate (sprites 70 / 71) tinted by its animation's colour, plus an icon sprite:
  0x12E red (0x8000003C) + icon 167; 0x133 blue (0x803C1E00) + icon 168; **0x131 green
  (0x80143200) + icon 278, tinted green (0x801EDC1E)**; 0x132 grey + icon 279; 0x12F / 0x160 / 0x130
  Riku's (red + 287, grey + 278, purple + 279).
- **Built** (`items_page::row_look`, `keyblades::row_look`, `win::keyblades::Page::row_look`): the
  Stock owner row and Sora's item slots (filled or empty) set up with **variant 2 (sequence
  0x131)**; the Keyblade slot, the Sora row, the stock list, the command and Yes / No windows keep
  variant 0 (0x12E). Re:CoM draws the box and the icon itself; nothing of ours is drawn. No new
  site (`0x140112F10` was already a checked callee). Tests:
  `items_page::tests::stock_and_item_slots_take_the_green_row`.
- ~~Assumption: icon sprite 278 is the item symbol~~ **Wrong** (owner, in game, 2026-11-01): "for
  the item slots, the color is correct, but they have the green mickey symbol, but lets use the
  green item symbol". Sprite 278 is the green **Mickey** symbol. Below.
- `items_page::tests::stock_and_item_slots_take_the_green_row` (renamed). Since then the item rows
  take the asset step's group with the item symbol when it is loaded (below, § Built).
- The Abilities page's rows now take 0x131 too (owner, same day: "have the abilities be green with
  the green mickey symbol"; `docs/recom-m9-abilities.md` § Rows).

### The item symbol (owner, 2026-11-01): where it is, and why no camp row has it

- **Re:CoM's item symbol is sprite 166** of `camp.lad` (sprite group 83; atlas rect (175, 50) –
  (187, 65), 12 × 15). Checked (decomp + files), from the Deck page's card-type tabs:
  - The Deck page's card list filter `0x1400D88A0(camp, char, tab, …)` puts a card in tab 1 for
    cards 0x01..0x18 / 0x101..0x118 (attack), tab 2 for 0x1A..0x29 / 0x11A..0x129 (magic),
    **tab 3 for 0x2A..0x30 (the item cards)**, tab 4 for 0x44..0x7E (enemy); tab 0 all (Checked
    (decomp)).
  - The tab strip plays `seqs[tab × 2 + lit]` of `0x1407C10D0` = 0xE7 0xE8 / 0xE9 0xEA / 0xEF 0xF0 /
    0xED 0xEE / 0xEB 0xEC (`0x1400E0870` cases 0xB / 0xC / 0x14; Checked (decomp + exe)). In
    `camp.lad` each of those sequences lights one tab (sprite groups 146 / 149 / 158 / 155 / 152 at
    x −82..−25 / −27..−1 / −3..23 / 21..47 / 45..71) and draws the four type icons over the tabs at
    x −14 / 11 / 35 / 57: sprite 167 (attack), 168 (magic), **166**, 169. Tab 3 (items, 0xED) lights
    sprite group 155 (x 21..47), under sprite 166; tab 4 (enemy, 0xEB) lights 152, under 169
    (Checked (files)). The Deck summary 0xD0 / 0xD1 draws the same four icons in the same order
    beside four counters.
  - Re:CoM draws sprite 166 untinted (colour 0x80808080) everywhere, as it does 167 / 168; 278 is
    always tinted (green 0x801EDC1E in 0x131, white, purple, pink elsewhere). Assumption: 166 is
    already green in the texture (the atlas `FO0100.RTM` is not in the session).
- **No camp row draws it** (Checked (files)): sprite 166 is used only by 0xD0, 0xD1 and 0xE7..0xF0
  (whole tab panels), in no other sequence of `camp.lad`, and in no other `.lad` of the session
  (`worldmap.lad`, `card.lad`, `gauge.lad`, `Cockpit.lad`, `layout_card.lad`, `pause.lad`). Every
  row sequence of Re:CoM's lists has a plate and one icon: the camp list's `0x1407C11F0` (0x12E red
  + 167, 0x133 blue + 168, 0x131 green + 278, 0x132 grey + 279, Riku's 0x12F / 0x160 / 0x130), the
  by-kind sleight rows `0x1407C1150` (0xD9..0xE6: 167, 168, 278, 287, 279) and the sleight tabs
  0x14F..0x157 (167, 168, 278, 279, 287). None has the item symbol: Re:CoM never shows an item card
  as a camp row.
- **Setting the icon after set-up** (Checked (decomp)): the row's `C2DSeqCtrl` holds up to 10
  layers (`+0x16D` count, `+0x144` handles); vtable `+8` (`0x140247850`) adds a layer playing a
  sequence by lad name and id (`0x140245A20` looks the lad up by name in Re:CoM's loaded list),
  `+0x20` (`0x1402482F0`) replaces layer *i*'s sequence. There is no call that swaps one sprite or
  sprite group inside a playing sequence. So a second layer could draw an item icon over the row,
  but only from a sequence that draws just that icon, and Re:CoM has none (above). Changing
  0x131's icon in the loaded data would change Re:CoM's own green sleight rows (variant 2 of the
  Sleights page) too.
- **So the item symbol on a green row needs one new sequence in `camp.lad`**: a copy of 0x131
  (green plate) whose icon animation draws sprite group 83 (sprite 166, as the Deck tabs draw it)
  instead of 85. Owner: "Yes, add it" (2026-11-01). Built below.

### Built: the asset step's item row (owner, 2026-11-01: "Yes, add it")

- **Asset step** (`kh1-recom-assets` `camp_rows`, run by `kh1-recom-build`): reads Re:CoM's
  `FORM/0100/FO0100.SPR` PKG entry with its PC assets, and appends **one animation group** to
  `camp:00` (SEQD 0 of `camp.lad`) in the base file and in every PC asset that is a `camp.lad`
  (any language prefix, e.g. `UK_camp.lad`) or a whole container holding one: 0x131's four
  animations copied (empty piece, green plate sprite group 270 tinted 0x80143200, glow, icon), the
  icon's sprite group **85 → 83**; frames, place (13, 10), tint 0x801EDC1E, flags and the group
  record's own words as 0x131's. Re:CoM's 353 groups and every other record stay byte for byte.
  The new group is **0x161** (353). It refuses a set that is not Re:CoM Steam's (another group
  count, already patched, 0x131 without exactly one icon on sprite group 85, sprite groups 83 / 85
  not one part each, the Deck's item tab 0xED not drawing 83), and every copy must give the same
  group number. Writes `raw/FORM/0100/FO0100.SPR` (a plain PKG entry, same asset names, tags and
  date, as `FO0002.SPR`) and `kh1-camp-rows.json` (format `kh1-recom-assets camp-rows 1`: set,
  `base_groups` 353, `item_row` 353, `copy_of` 305, `icon_sprite_group` {from 85, to 83}, the files
  patched, the base file's sha256). Without the entry (or on a refusal) it prints a note and removes
  an older `kh1-camp-rows.json`.
- **How a row gets it** (Checked (decomp)): `0x140112F10` keeps the table pointer it is given
  (`CButton+0x10`) and plays `table[variant]` (`*(u16*)(table − 2 + 2 × (variant + 1))`); the
  state change `0x140112E00` reads the kept table again for a new variant. Re:CoM's camp table
  `0x1407C11F0` has 7 entries and a 0, so the new group needs its own entry: the DLL keeps an
  8-entry table in its own memory (`win::keyblades::ITEM_TABLE`: Re:CoM's 7 entries read from the
  image, then 0x161) for the life of the DLL and gives it to the set-up for every list row of our
  pages while the item row is in use; variants 0..6 still mean Re:CoM's sequences, variant 7
  (`keyblades::row_look::ITEM`) is ours. The Stock row and Sora's item slots take variant 7
  (`items_page::row_look(label, item_row)`). No new hook or call site (`0x140112F10` and the table
  site were already checked).
- **Fallbacks**: no `kh1-camp-rows.json`, a manifest this DLL does not expect (`camp_rows::parse`),
  or a loaded `camp:00` without exactly 354 groups (read from Re:CoM's 2D registry, as the MP bar
  checks `Cockpit:00`; Panacea serving Re:CoM's own file, or an unpatched language copy) → Re:CoM's
  own table and the green row 0x131 (Mickey symbol), as before. Logs: `camp rows: … item row =
  camp:00 group 0x161 …` at start; `camp rows: camp:00 has N groups, not 354 …` once when the
  loaded set lacks it.
- Checked (files): `camp.lad` and `camp:00` of the session's `FO0100.SPR` (the base file, no PC
  assets) read back byte for byte with the step's readers; patched, the new group 0x161 draws
  sprite groups [46, 270, 46, 83] where 0x131 draws [46, 270, 46, 85]
  (`camp_rows::tests::owner_camp_file` with `KH1_FO0100_SPR` = that file). Synthetic tests:
  `camp_rows::tests` (asset crate: the copy, refusals, container + language copies, manifest) and
  `camp_rows::tests` (DLL: manifest, our table = the checked bytes of `0x1407C11F0` + 0x161, the
  loaded-set check), `items_page::tests::stock_and_item_slots_take_the_green_row`.
- **Not checked**: the PC assets of the `FO0100.SPR` entry (which language copies exist, whether
  they have Re:CoM's 353 groups): not in the session; the step prints them (`PC assets of the
  entry: […]`) and which it patched. Assumption: Re:CoM draws `camp:00` from one of those copies
  per language, as `Cockpit:00` (so all are patched). Not seen in game. Assumption: sprite 166
  tinted 0x801EDC1E reads as the green item symbol (the atlas is not in the session; the Deck
  tabs draw it untinted). If it looks wrong, the copy can keep 0x131's tint off for the icon
  (Re:CoM's untinted draw) instead.

### Language copies of `camp.lad` (0.56.1)

- The owner's asset step run (0.56.0) refused the item row: `FORM/0100/FO0100.SPR asset FR_camp.lad:
  camp:00 has 367 animation groups, Re:CoM Steam's has 353`. Checked (the run's log): the entry's
  language copies `FR_` / `GR_` / `IT_` / `SP_` / `UK_camp.lad` have 14 more groups than the base file,
  as `FO0002.SPR`'s `<LANG>_Cockpit.lad` have 157 to the base file's 151 (the MP bar already handles
  that per copy).
- Now each copy gets the new group after its own groups (base file 353; a copy with 367 → 367). The
  base file must still have Re:CoM Steam's 353; a copy must have at least 353, pass the same checks on
  0x131 / sprite groups 83, 85 / the Deck's item tab, and not already end with our group.
- `kh1-camp-rows.json` is format `camp-rows 2`: `item_row` (the base file's), `item_rows` (every copy's,
  sorted) and `files` with each one's group. The DLL keeps our table's last entry in its own memory
  and, before each list set-up, sets it to the loaded set's last group when that is one of
  `item_rows` (`camp_rows::loaded_row`; the count from Re:CoM's 2D registry, as before). Logged once:
  `camp rows: camp:00 has N groups: item row = group 0x…`.
- Assumption: a language's 14 extra groups come after Re:CoM's 353 and 0x131 is the same green row
  in every copy (Re:CoM's table `0x1407C11F0` names 0x131 for every language, and the step's checks on
  0x131's icon pass or it refuses). Checked: tests. Not checked: the owner's PC assets (next run's log).

### Stock items green too (0.56.2)

Owner (2026-11-01, screenshot of the Stock list, its rows red with the Keyblade symbol): "have all
the items also be green and have the items symbol". Every stock item row (`items_page::Label::Stocked`,
in the Stock list and in the pick from it) now takes the same look as Sora's item slots: the asset
step's green row with the item symbol (variant 7), else Re:CoM's green row 0x131. The Keyblade slot,
the Sora row, the commands and Yes / No stay red. No new address. Checked: tests. Not checked: in game.

---

## M8c: KH1 Moogle shop list

> **Replaced in 0.128.0 by [M8d: the shop on Re:CoM's own screens](#m8d-the-shop-on-recoms-own-screens).**
> The Buy / Sell / Card Packs list and its mode-3 init wrapper (`0x1403A53F0`) are gone: Re:CoM's
> two-Moogle menu runs unpatched. The research below (the 7 modes, the text windows, the payment)
> stays true of Re:CoM; the stock, prices and buy / sell rules are M8d's.

Owner (2026-11-04, `design-philosophy-instructions.md` § KH1 Moogle shop): at Combat = KH1 the
Moogle shop becomes a list like KH1's (Re:CoM assets allowed): rows **Buy**, **Sell** and **Card
Packs** ("A 'Card Packs' row": Re:CoM's pack screen as before); the 8a.3 Items tab goes. Stage 1
(0.103.0): the list, stock by floor, buy and sell. Tent / Camping Set, their camp use and the
drops of Re:CoM-only enemies are later stages (Tent / Camping Set and the Mega items' removal:
[stages 2–3](#m8c-stages-23-tent-and-camping-set-the-mega-items-removed), 0.105.0).

Code: `crates/kh1-recom/src/shop.rs` (stock, prices, the sell list, buy / sell rules, the list's
cursor and texts; tested), `src/win/shop.rs` (Re:CoM's side), sites `sites::SHOP_HOOKS`,
`SHOP_CALLEES`, `SHOP_CHECKED` (test `sites::tests::shop_sites`). No asset step change.

### Research: Re:CoM's Moogle shop flow (Steam)

**Start.** The event script's command 0x38 (the script interpreter `0x140145C50`) allocates 0x470
bytes and calls `CMoogleShopCore`'s ctor `0x1403A4220(mem, 0x1402AD300(1))` (a task under the
script's task parent). The ctor runs only when no shop is up (`0x14087A450` = 0, then the shop's
task id): it copies the player's stats to the save (`0x1401F0120`), pauses the field
(`0x1402919B0`, `0x1401A9CC0(0)`, `0x140232260(0xFFFEBFFF)`, `0x140232270(1)`, `0x1402A9E50`),
fills the 7 mode records and makes `CMShopModeTaskParent` (`+0x268`), and points the state
machine's pending state (`+0x68`) at mode 0. Checked (decomp).

**The state machine.** `CMoogleShopCore` is a `CFSM` at `+0x58` (current state `+0x60`, pending
`+0x68`); its step `0x140242F50` ends the current record (vtable `+0x18` → the record's `+0x30`
function), inits the pending one (`+8` → `+0x10`) and runs the current one's update (`+0x10` →
`+0x20`) every frame. The 7 modes are records at `+0x98 + 0x40·mode` (`+8` owner, `+0x10` init,
`+0x20` update, `+0x30` end; record vtable `0x14065BC50`); `+0x258` the mode, `+0x25C` the next
mode, `+0x260` the mode's state. A mode switches by writing `+0x258` and `+0x68`. Checked (decomp
+ disasm `0x1403A4431..0x1403A4486`).

| mode | init / update / end | what it is | tag |
|---|---|---|---|
| 0 | `0x1403A52E0` / `0x1403A8AA0` / `0x1403A59F0` | Loading: asks for file 0x6D, waits (`0x14023CA80`), sound 0x49, makes the card data `CMShopChargeCardData` (`+0x350`), reads `MogFloor` (`0x1403A5B10`), makes the background `CMShopBG` (`+0x3B0`), frame `CMShopFrame` (`+0x3B8`), Moogle Point window (`+0x3C0`), help bar `CMShopHelpMessage` (`+0x3C8`), the two Moogle models (`+0x3A0`, `+0x3A8` the bottle Moogle), three `CMShopFingerCursor`s (`+0x3D0`, `+0x3D8`, `+0x3E0`) and the Moogle's text window `CMShopTextWindow` (`+0x3E8`); then mode 3. | Checked (decomp) |
| 1 | `0x1403A5320` / `0x1403A8DA0` / `0x1403A5A00` | The first visit's free pack: on confirm draws pack type `+0x278` (`0x1403A66F0`), sets the shop flag bit 16 (`0x1401141D0`), shows the cards; then mode 3. | Checked (decomp) |
| 2 | `0x1403A51C0` / `0x1403A8770` / `0x1403A59E0` | No room for 5 more cards: the text window's message 1 (a question and a two-line choice), the finger at x 126, y 210 / 233 (`0x14065BA30`); Up / Down (no wrap), confirm on row 0 → mode 5; row 1 or cancel → state 100 → mode 6 (leaves the shop). | Checked (decomp + exe) |
| 3 | `0x1403A53F0` / `0x1403A8EF0` / `0x1403A5A10` | **The two-choice menu.** Init: without room for 5 cards (`u16 [*0x14087A7E8 + 6] < [+0x350]+0x44 + 5`) → mode 2; first visit (`0x140114180()` bit 16 clear) → mode 1; else frame 2, help bar 2 + choice, Moogle models' mode 2, text window message 2 + choice, the Moogle or the bottle Moogle posed by the choice. Update: state 0 makes `CMShopSelectCursor` (`+0x3F8`, two `m_shop.lad` buttons, groups `0x14068EEB8`, places layout element 0x11 entries 0 / 1); 2 the input: cancel → sound 4, state 100; confirm → sound 2, `+0x25C` = 4 + choice; Left / Right set the choice `+0x27C` (sound 1); 3 / 4 close and switch; 100 / 0x65 close and go to mode 6. | Checked (decomp + disasm) |
| 4 | `0x1403A4A90` / `0x1403A6930` / `0x1403A59B0` | The pack screen (8a.3's findings); cancel → mode 3, Square → mode 5. | Checked (decomp) |
| 5 | `0x1403A4D90` / `0x1403A73E0` / `0x1403A59C0` | **The card trade** (Re:CoM's "Charge", sell cards for points): `CMShopChargePointWnd`, `ChargeCardRankWnd`, `StockCardWnd`, `CardInfoWnd`, `CardCountWnd` (`+0x448..0x468`); 5 card-category tabs (`+0x35C`, Left / Right, wrap), a scrolling 3-wide card grid (`+0x360` / `+0x364`), a count, then a yes / no in the text window (message 7, the finger at x 232, y 214 / 237); a sale: sound 6, points added by Re:CoM's `0x140239FA0`, the card leaves (`0x140159B20`); cancel → mode 3, Square (with room) → mode 4. | Checked (decomp) |
| 6 | `0x1403A5180` / `0x1403A8760` / `0x1403A59D0` | Leaving: `0x1401F0BC0`, `0x1402AB410`, then the task ends itself (vtable `+0x20`). | Checked (decomp) |

**Mode 3's two choices** are card packs (choice 0, mode 4) and the card trade (choice 1, mode 5).

**End.** Mode 6 ends the task; the dtor (`0x1403A47A0` / `0x1403A4950`) undoes the ctor's pause
(`0x1402AC0F0`, deletes `+0x350`, `0x140245710(0)`, `0x140291A30`, `0x1401A9CC0(1)`,
`0x1402A5790(2)`, `0x140232260(0)`, `0x140232270(0)`, `0x14027FE40(0)`, `0x14087A450` = 0,
`0x1401B15D0(1, 0x140)`): the field and the event script carry on. Checked (decomp). How the
script command waits for the shop task: Assumption (not read).

**Floor.** `0x1403A5B10` reads `0x1402337C0()` (`*[0x14087B110]`, the save's first byte), 0xFF →
10, + 1; tiers 1, 2..6, 7..10, 11+. Checked (decomp + exe).

**Text.** The shop's text windows are Re:CoM's `CTextWnd` (ctor `0x1402AFF30`, 0x148 bytes,
state `+0x88`: 5 shown, 6.. closing, past 8 closed; close `0x1402B1310`; delete vtable slot 0).
`CMShopTextWindow::0x1404595A0(win, id)` opens one from a message record (`0x1402B1520`, ids at
`0x14068EF58`), a second one for choices (`0x14068EF78`: ids 1 and 7); its `+0x58` is 1 when both
are open, −1 when closed. Re:CoM's own explicit open `0x1402B1420(w, &pos, &size, frame, string,
colour, line pitch, placement, &offset, 0, flags)` (callers `0x1402995A0`, `0x14013F040`,
`0x1402997F0`) takes any string; placement 0 keeps the x given, 1 / 2 / 3 replace it (x 32, centred
on 256, right at 480); frame 0 hides its `CWnd` box. `0x1402B0EF0(w, string, 0)` replaces a shown
window's text (the pack screen's help line `0x1404573A0` does it). Lines split at `\n`
(`0x1402B1520`). The text goes through Re:CoM's text set `0x140259000`, which keeps the pointer
(no copy) on a plain font: the string must live while shown. Checked (decomp).

### Which Re:CoM list UI holds the list

- **(a) The camp list page our KH1 pages use: not workable without a large risk.** The page's
  pieces are the camp's: our pages call Re:CoM's camp functions on the `CCamp` object (panels
  `0x1400E6700`, highlight bar `0x1400E07D0`, header `0x1400E4220`, side panel `0x1400E46D0`), the
  camp's input reads (`0x140111A70` / `0x140111C30` read camp fields) and the camp's layout
  `camp.lad` loaded by the camp's own state 3 (`0x140245940(camp+0x78, 8)`). A `CCamp` exists only
  as the field's camp task: made by the field game object (`0x1401A9990`, `0x1401A9590`,
  `0x1401AD4C0`: 0x2250 bytes, ctor `0x1400D2730`, the singleton `0x14087A300`), its id at field
  `+0x5A8`, and its end noticed by the field's own state (`0x1401AAFB0`: the id's task gone →
  `0x1401A9590(field, 0)` resumes the field's tasks). Opening one from the shop would need a second
  owner for that life cycle, and the camp's loads and pause on top of the shop's. Checked (decomp).
- **(b) Mode 3's own menu given a third choice: needs art not in the session.** Its two buttons are
  `m_shop.lad` art (`0x14068EEB8` groups) at `m_shop.lad`'s layout element 0x11 entries 0 / 1; a
  third needs a new button and a third layout entry (`FORM/0109`, not dumped), and Buy / Sell would
  still need a list. Checked (decomp + exe).
- **(c) Built: the shop's own list pattern, as mode 3.** Mode 2 (and mode 5's yes / no) show a list
  as a `CTextWnd`'s lines with the shop's finger cursor stepping over them; the list runs as mode 3
  of Re:CoM's own state machine. Every piece is Re:CoM's: its text window class opened with its
  explicit open, the shop's finger, text window, frame, help bar, Moogle Point window, sounds and
  payment; Re:CoM draws, steps and closes them. The rules are ours (the feature).

### What runs (Combat = KH1, Sora's story)

1. **Mode 3's init** (`0x1403A53F0`, wrapped; 10 bytes moved, nothing branches into them): with
   room for 5 cards and the free pack not given, Re:CoM's init runs (mode 1, then mode 3 again).
   Else the shop's own mode-3 update slot (`+0x178`, the object's data, checked to hold Re:CoM's
   `0x1403A8EF0` first) gets our update, and the windows are set up as Re:CoM's mode-3 init sets
   them for choice 0 (frame 2, point window shown, help bar 2, models' mode 2, text window message
   2, the Moogle posed, cursors hidden), the finger as mode 2 prepares it (look 0, priority 0x16).
   Without room for 5 cards the list opens anyway (items need no card room). At Combat = Re:CoM or
   in Riku's story: Re:CoM's init, and Re:CoM's update put back if a KH1 shop left ours.
2. **Our update** keeps Re:CoM's mode-3 states: 0 waits for the windows (`0x1403A60F0(shop, 1)`)
   and opens the rows window (`0x1402AFF30` under the shop's text window, `0x1402B1420` at (142, 84),
   240 × 207, pitch 23, frame 0, placement 0, Re:CoM's colour and flags, `0x1402B1980(w, 0)`,
   `0x1402B2C80`); 1 waits for it; 2 the input; 3 / 4 close (`0x1403A6640(shop, 0)`, ours closed and
   deleted) and switch to `+0x25C` once Re:CoM's windows are closed (`0x1403A61F0`) and the task
   parent is idle; 100 / 0x65 leave (`0x1403A6640(shop, 1)`, `0x140459C30(bg, −1)`,
   `0x1402AA9B0(−1, −1)`, then mode 6), as Re:CoM's states 3 / 4 / 100 / 0x65.
3. **Input** (Re:CoM's mode 2 list's order: cancel, confirm, Up, Down; no wrap; the lock `+0x274`):
   - Root: **Buy** / **Sell** open their page (sound 2); Sell with nothing owned: sound 5 and "Sora
     has nothing to sell."; **Card Packs**: sound 2, `+0x25C` = 4 (mode 2 without room for 5 cards:
     Re:CoM's own rule), the windows close; cancel: sound 4, the shop closes (Re:CoM's mode 3).
   - Pages: cancel → the root (sound 4), the root cursor kept (also across the pack screen).
   - The finger at (126, 96 + 23·row); the highlighted row's line in the Moogle's text window
     (`0x1402B0EF0` on its main window while `+0x58` = 1).
4. **Buy** (one per confirm): price ≤ Moogle Points, else sound 5 and "Not enough Moogle Points.";
   KH1's pickup (`Inventory::pick_up`: the first empty slot, else the stock to 99), else sound 5 and
   "Sora can't carry any more of this." (nothing paid); then sounds 2 + 0x58 and Re:CoM's payment
   (8a.3: the spent tally and its 20000 trophy, the setter `0x14023A1D0`, the Moogle Point window).
5. **Sell** (one per confirm): from the stock first, else the last slot holding it; sound 6 (the
   card trade's sale sound); the points + price through the setter `0x14023A1D0` (it clamps to
   99999) and the Moogle Point window. A row at 0 leaves the list; an empty list goes back to the
   root.

### Data (owner, 2026-11-04)

| item | from | buy | sell | tag |
|---|---|---|---|---|
| Potion | 1F | 10 | 2 | owner's prices; sell = buy ÷ 5 (KH1's ratio, Checked (files)) |
| Ether | 1F | 40 | 8 | ″ |
| Hi-Potion | 2F | 30 | 6 | ″ |
| Elixir | 7F | 200 | 40 | ″ (KH1 never sells it: buy 0) |
| Tent | 1F | 100 | 20 | owner's price; built in 0.105.0 ([M8c stages 2–3](#m8c-stages-23-tent-and-camping-set-the-mega-items-removed)) |
| Camping Set | 2F | 150 | 30 | ″ |
| Mega-Potion / Mega-Ether / Megalixir | never | – | 40 / 60 / 100 | KH1 has no buy price (0), so **KH1's own sell price** (`btltbl.bin` `+0xA`, Checked (files)). Since 0.105.0 a save's are converted (stage 3), so they never show |

Rows read "Potion / 10" (Buy) and "Potion x3 / 2" (Sell); names are KH1's words when plain ASCII,
else English. The help line: "Restores HP. 10 Moogle Points. Sora has 2." / "Restores HP. Sells for
2 Moogle Points. Sora has 3."

### Sites

| address | bytes | use |
|---|---|---|
| `0x1403A53F0` mode 3 init | `48 89 5C 24 18 57 48 83 EC 20` (10) | **hooked** |
| `0x1403A60F0`, `0x1403A61F0`, `0x1403A6640`, `0x140114180`, `0x1402337C0`, `0x1404598C0`, `0x140459CC0`, `0x14045F3A0`, `0x14045F1E0`, `0x14045F130`, `0x1404595A0`, `0x140459C30`, `0x1404582B0`, `0x1402AA9B0`, `0x140459D60`, `0x140459890`, `0x140459A00`, `0x1404597D0`, `0x1404598A0`, `0x1404598D0`, `0x1402AFF30`, `0x1402B1420`, `0x1402B1980`, `0x1402B2C80`, `0x1402B0EF0`, `0x1402B1310`, `0x14023A1D0`, `0x1401B16C0`, `0x1402BA6F0` | 7..18 bytes each | called (`SHOP_CALLEES`) |
| `0x1403A4431` (85), `0x1403A53FA` (363), `0x1403A9152` (277), `0x1403A6D74` (224), `0x1403A7041` (197), `0x1403A8095` (13), `0x14065BA30` (8) | | checked only: the mode-3 record, mode 3's init after the hook and its leaving states, the pack buy's sounds and payment, the sale sound, mode 2's rows |

Removed with the Items tab: the pack-screen hook `0x1403A6930` and its callees / checked sites
(the refresh, the pack class, the `C2DCard` calls, the tab and grid code, the tables). All 37 shop
site byte sets (and every one of the 714 byte sites a script parsed from `sites.rs`) compared with
`recomsteam.exe` (pefile): 0 mismatches. Checked (exe).

### Per frame

Nothing outside the shop's mode 3. In it: Re:CoM's state machine calls our update instead of its
own; in the input state four pad reads, the inventory read, and the texts set only when they change.

### Logs

`shop: KH1's Moogle shop list (M8c): …` and `hooked shop mode 3 init …` at start; then (80 lines at
most) `KH1 list (mode 3): floor tier Early, 500 Moogle Points, room for a pack: true`, `bought
potion for 10: 500 -> 490 Moogle Points, Slot(0)`, `refused …`, `sold ether for 8: … from Stock(2),
2 left`, `Card Packs: on to Re:CoM's mode 4`, `shop left from the KH1 list`, `first visit: Re:CoM's
free pack (mode 1) before the list`.

### 1:1 checks

- **Checked (decomp + disasm + exe)**, by tests (`shop::tests`, `sites::tests::shop_sites`): the
  floor tiers; the mode-3 record slots; mode 3's room rule, first-visit rule and window set-up (call
  by call); its leaving states; the list's read order and no-wrap (mode 2's); the sounds (1, 2,
  2 + 0x58, 4, 5, 6); the payment (clamp, tally, trophy); KH1's pickup; KH1's sell ratio and the
  Mega items' KH1 sell prices.
- **Not KH1's by the owner's choice**: the prices, the stock by Re:CoM's floor tiers, the list's
  look (Re:CoM's assets).
- **Not checked**: in game. The rows window's place and the finger's line-up with the text
  (Assumption: the first line at the window's top, pitch 23 as mode 2's rows); the help line's fit
  in the Moogle's text window (its message record is in a file not in the session); KH1's own sell
  source (stock or slots: not read; ours stock first).

### Assumptions

1. The rows' place (142, 84) and the finger at (126, 96 + 23·row) line up; the CTextWnd's type 0
   and `0x1402B1980(w, 0)` show the text at once.
2. The text window's main `CTextWnd` shows Re:CoM's mode-3 message 2 for its opening frames
   before our line replaces it (`0x1402B0EF0` needs state 5).
3. The shop is always left through Re:CoM's states (mode 6); a torn-down shop takes our rows
   window with its text window (its task parent).
4. KH1's words for item names are plain ASCII (else English).

### Open questions (owner)

1. **Card Packs without room for 5 cards**: Re:CoM's mode 2 (offers the card trade; "No" leaves
   the shop). (a) keep (built: Re:CoM's rule); (b) refuse with sound 5 and a line; (c) straight to
   the card trade.
2. **The card trade** (Re:CoM's "Charge", mode 5) has no row: it stays reachable from the pack
   screen (Square) and the no-room screen. (a) keep; (b) a fourth row "Trade Cards".
3. **Selling equipped items**: built: the stock first, then the item slots. (a) keep; (b) the stock
   only (slot items not sellable).

### What the owner should test

Combat = KH1, Sora's story, a Moogle room (`kh1_test_items = on` puts items in stock to sell).

1. Log: `shop: KH1's Moogle shop list (M8c)` and `hooked shop mode 3 init (0x1403A53F0)`.
2. Talk to the Moogle (not the first visit): three rows **Buy / Sell / Card Packs**, the finger on
   Buy, the Moogle's line "Buy items with Moogle Points." Up / Down move (sound 1, no wrap).
3. Buy on 1F: Potion / 10, Ether / 40; on 2F–6F also Hi-Potion / 30; 7F and up also Elixir / 200.
   Confirm: buy sounds, the Moogle Points drop at once; the line counts up "Sora has N". Too few
   points or nothing more fits: the error sound and the line says why, nothing paid.
4. Sell: every item Sora has with "xN / price" (Mega items too: 40 / 60 / 100); confirm: sound 6,
   points up, the count drops; a row at 0 goes; selling everything returns to the root. Sell with
   nothing: the error sound.
5. Card Packs: Re:CoM's pack screen with its four tabs, no Items tab; buy a pack as usual. Cancel:
   back to the list with the finger on Card Packs. Square in the pack screen: the card trade, its
   cancel back to the list.
6. Cancel on the root: the shop closes as before. Save, reload: bought items and points kept.
7. First visit to a Moogle: Re:CoM's free pack first, then the list.
8. Combat = Re:CoM or Riku's story: Re:CoM's shop (packs / card trade), unchanged.
9. Report where the rows and the finger sit, and whether the Moogle's line fits.

## M8c stages 2–3: Tent and Camping Set; the Mega items removed

> **The Tent and the Camping Set were removed again in 0.124.0** ([M8d step 1](#m8d-step-1-the-tent-and-the-camping-set-removed)):
> not sold, dropped, picked up, used or listed; a save's become Elixirs. The Mega items' removal
> below stands. The KH1 findings below stay true of KH1.

Owner (2026-11-04, `design-philosophy-instructions.md` § KH1 Moogle shop): "The very early game
shops should only sell potions, ethers, and tents. Tents and camping sets should have their
descriptions changed to say sora instead of "to the party" and should have their cost reduced";
"Lets remove mega potions, mega ethers, and mega elixirs"; answers to the plan: Tent = full HP,
Camping Set = full HP and MP, used from the KH1 camp Items page outside battle ("Camp-only full
heal"). Version 0.105.0. Combat = KH1, Sora's story only (the store is read only then); Combat =
Re:CoM and Riku's story unchanged.

Code: `crates/kh1-recom/src/items.rs` (`id::TENT` / `CAMPING_SET` / `CAMP` / `HELD` / `REMOVED` /
`IN_PLAY`, the two `DATA` rows, `effect`, `camp_only`, `stock_index`, `convert_removed`, the test
inventory), `items_page.rs` (stock list, `can_use`, English names), `shop.rs` (the wares built,
the Sell list order, the help lines), `drops.rs` (`spawnable`, `Dropped::Removed`),
`win/items.rs` (the conversion at the store's read, the log lines), `win/drops.rs` (the log line).
Asset step: `kh1-recom-assets` `command_menu::NAMES` + 172, 173.

### KH1 (Steam)

| what | where | tag |
|---|---|---|
| **Rows 142 / 143** (`btltbl.bin` `0x255C` / `0x2570`): `+0` name message 172 / 173, `+2` kind **12**, `+4` party 1, `+6` 255 / 3, `+8` buy 250 / 500, `+0xA` sell 50 / 100, `+0x10` sort 101 / 102. Row 144 (message 174, kind 12, 255, sell 150) is a third Tent-like item, never sold (buy 0; a Cottage: Assumption from its numbers). | `0x14028F970` | Checked (files) |
| **Kind 12** is a menu item: the pause menu's can-use `0x1402CD3A0` accepts kinds 0 and 12 only; the slots and the battle menu hold kind 0 only (`0x140284E60`, `0x1402E8F50`), so the pickup puts a Tent in the stock; Move (`0x1402E8A00`) is kind 0 only; Discard (`0x1402E8AA0`) any kind but 13. | `0x1402CD3A0`, `0x140284E60` | Checked (decomp) |
| **Effect** `0x1402CD780`: case 0x8E (Tent) HP + 9999 (full); case **0x8F (Camping Set) HP + 9999 and MP + the row's `+6` = 3 points** (not full MP); case 0x90 (row 144) both + 9999. On the whole party (the row's party bit). | `0x1402CD780` | Checked (decomp + files) |
| **Can-use** (`0x1402CD3A0` bit 0): 0x8E while HP is below max; 0x8F (and 0x90) while HP or MP is below max. | `0x1402CD3A0` | Checked (decomp) |
| **Drops**: the Barrel Spider's defeat table drops #142 at 1 % and #143 at 0.5 % (8a.4's table); no other twin drops them. | 8a.4 | Checked (files) |
| **Mega items in drops**: the Search Ghost, Pirate, Air Pirate (Mega-Potion 1 %), Wight Knight (0.5 %), Screwdiver (4 %), Gargoyle (Mega-Ether 0.5 %). | 8a.4 | Checked (files) |

KH1's help texts (the item descriptions) are not in the session or the asset step: the lines below
are ours.

### What runs (Combat = KH1, Sora's story)

**Stage 2, the Tents.**

- **Items**: our ids are KH1's (142, 143), KH1's rows as `DATA` (kind 12). Not battle items: no
  slot, no command (`item_of_command` takes battle items only: command 0xAC / 0xAD is never an
  item), not in the command menu's Items list, never in the battle use task. `items::camp_only`.
- **Storage**: the stock only (KH1's rule for kind 12), up to 99. The stock bytes (16, unchanged
  save layout): bytes 0..7 items 1..8 as before, **byte 8 the Tent, byte 9 the Camping Set**
  (`stock_index`), 10..15 spare. Before 0.105.0 bytes 8..15 meant items 9..16, which nothing could
  store (the pickup was only given built items: shop wares, battle-item drops, a returned use;
  Checked (code)): every save has them at 0.
- **Camp Items page** (8a.2's page): they show in the Stock list after the battle items (KH1's sort
  keys 101 / 102), with Use, Move greyed (KH1), Equip / Remove greyed (stock), Discard. Use while
  Sora's HP is below max (Tent) or his HP or MP (Camping Set), as KH1's `0x1402CD3A0`; at full the
  Use row is greyed and its confirm is sound 5, nothing used (the page's rule for every item; that
  KH1 refuses at this window rather than at its target list: Assumption). A use: Re:CoM's camp
  sound 2, the item leaves the stock, the heal through `win::items::use_in_menu`: HP through
  Re:CoM's heal `0x1401E59A0(player, max, 0, 0, 0)` and its stats sync `0x1401F0120` (the Potion /
  Elixir path), MP through our MP block (`mp::add_points(MP_FULL)`, as the Elixir). The camp opens
  only in the field (Re:CoM's own rule), so they are never used in battle.
- **Effect**: Tent HP to full (KH1's); Camping Set HP and MP to full (**the owner's design**;
  KH1's own adds 3 MP points: `items::CAMPING_SET_FULL_MP`, owner question 1).
- **Shop**: `Ware::Tent` / `CampingSet` built: Buy on 1F Potion, Ether, Tent; 2F–6F + Hi-Potion,
  Camping Set; 7F+ + Elixir (`Tent / 100`, `Camping Set / 150`). A buy goes to the stock (KH1's
  pickup); refused at 99 ("Sora can't carry any more of this."). Sell: `Tent xN / 20`,
  `Camping Set xN / 30` (the owner's buy ÷ 5; KH1's own ratio 250 / 50, 500 / 100 too), after the
  battle items.
- **Help lines** (the shop's Moogle line, ours): Tent "Fully restores Sora's HP.", Camping Set
  "Fully restores Sora's HP and MP." (each within `menu::HELP_MAX`), e.g. "Fully restores Sora's HP.
  100 Moogle Points. Sora has 2." The camp page shows no item descriptions (none of its items has
  one: unchanged).
- **Names**: the other items' source: KH1's words (`kh1-command-menu.json`, the asset step's
  `NAMES` now with 172 / 173), else English "Tent" / "Camping Set" (`items_page::text::english`)
  until the asset step is re-run. That message 172 / 173 is "Tent" / "Camping Set" in `Word.bin`:
  Assumption (rows 1..8's `+0` are their `Word.bin` ids, Checked (files); `Word.bin` not in the
  session).
- **Drops**: `drops::spawnable` = the items in play (`id::IN_PLAY`): the Barrel Spider's KH1 Tent
  (1 %) and Camping Set (0.5 %) now drop (KH1's table), picked up into the stock.
- **Test inventory** (`kh1_test_items`): 9 of each item in play in stock, the Tents included.

**Stage 3, the Mega items.**

- **Drops**: a roll that gives a Mega item gives **nothing** (`Dropped::Removed`, logged "removed
  at Combat = KH1 (M8c stage 3): no prize"), not its plain item: nothing is closest to "removed",
  and the plain item would raise those enemies' plain-item rates above KH1's.
- **A save's Mega items: converted once** (`items::convert_removed`): Mega-Potion → Hi-Potion,
  Mega-Ether → Ether, Megalixir → Elixir, in their slot (a battle item stays a battle item) or in
  the stock (added to the plain item's, up to 99; any past 99 is lost and logged). Run at the
  store's first read at Combat = KH1 of a block at version 1; the block becomes **version 2**
  (`ext::VERSION`; every write is now v2, a never-written v0 block stays v0 = KH1's new game). One
  log line: `items: save block v1 -> v2: Mega items removed at Combat = KH1 (…): n slot(s), n from
  the stock, n past 99 lost`. Older DLLs read a v2 block as before (they test only for 0).
  - Chosen over hiding them (sellable, not on the Items page or the command menu): a hidden Mega
    item in a slot would hold that slot while the page and the command menu showed it empty (the
    pickup skips a filled slot), and the page, the command menu, the Move / Equip lists and the
    shop would each need their own filter. The conversion is one pure rule at one place, and none
    of them ever meets a Mega item. Cost: it changes the save (the conversion can't be undone),
    and a Megalixir is worth an Elixir (sell 40, not 100).
- **Shop**: never sold (as before); their KH1 sell prices stay in `shop::sell_price` (unreachable
  after the conversion).
- **Test inventory**: never hands them out (`id::IN_PLAY`); `TEST_SLOTS` Potion, Hi-Potion, Ether.

### Sites, hooks, per frame

No new hook, site or patch. Per frame: nothing new (the conversion is one byte compare inside the
store's read, which runs only when something reads the items).

### Logs

`items: save block v1 -> v2: …` (once per save), `drops: … KH1's mega-potion dropped; removed at
Combat = KH1 (M8c stage 3): no prize`, `items: tent picked up (enemy drop): stock (1)`, `items
page: Used { item: 142, … }: HP 300 -> 560 (+560, max 560)`, the shop's `bought tent for 100: …`.

### 1:1 checks

- **Checked (decomp + files)**, by tests (`items::tests::the_tents_*`,
  `a_saves_mega_items_are_converted_once`, `items_page::tests::the_tents_are_used_*`,
  `shop::tests::the_tents_*`, `drops::tests::only_the_items_in_play_*`): KH1's rows 142 / 143,
  kind 12's rules (stock only, no Move, Discard), the can-use rule, the Tent's full HP, the
  Barrel Spider's rates, the sort order.
- **Not KH1's (the owner's)**: the prices; Sora only (KH1 heals the party); the Camping Set's full
  MP (KH1: + 3 points); the Mega items gone; English help lines.
- **Not checked**: in game; KH1's own help texts and its `Word.bin` 172 / 173; where exactly
  KH1's pause menu refuses a Tent at full HP (the Use command or its target list).

### Assumptions

1. `Word.bin` 172 / 173 are "Tent" / "Camping Set" (the rows' name pattern).
2. The camp never opens in battle (Re:CoM's own rule), so "outside battle" needs no test of ours.
3. The longest Sell help line ("Fully restores Sora's HP and MP. Sells for 30 Moogle Points. Sora
   has 99.", 73 characters) fits the Moogle's text window (the stage-1 lines were up to 64; the
   window's width is not read).

### Open questions (owner)

1. **The Camping Set's MP**: built as the design says, MP to full. KH1's own Camping Set adds 3 MP
   points (case 0x8F, its row's amount). (a) keep full; (b) KH1's 3 points (one constant,
   `items::CAMPING_SET_FULL_MP`; the help line would then read "Fully restores Sora's HP and
   restores 3 MP.").
2. **The Barrel Spider's Tent drops** (KH1's 1 % / 0.5 %): built (they follow "drop items how they
   do in KH1" now that the Tents exist). (a) keep; (b) shop only.
3. **Item descriptions on the camp Items page** (KH1's item screen shows the highlighted item's):
   none of the page's items has one now. (a) leave; (b) the shop's lines on the Stock list.

### What the owner should test

Combat = KH1, Sora's story.

1. A save with Mega items (an older `kh1_test_items` save has 9 of each): load it, open the camp
   Items page: no Mega items; the Hi-Potion / Ether / Elixir counts rose by the Mega counts (to 99
   at most); a Mega item in a slot is its plain item. Log: `items: save block v1 -> v2: …` once;
   save, reload: no second line.
2. Moogle shop on 1F: Buy lists Potion / 10, Ether / 40, **Tent / 100**; on 2F–6F also Hi-Potion /
   30 and **Camping Set / 150**; 7F+ also Elixir. The Moogle's line on the Tent: "Fully restores
   Sora's HP. 100 Moogle Points. Sora has N." Buy a few: they go to the stock (the slots stay as
   they were). Sell: `Tent xN / 20`, `Camping Set xN / 30`, after the battle items; no Mega rows.
3. Camp Items page → Stock: Tent and Camping Set after the battle items. With full HP the Tent's
   Use is greyed and confirming it buzzes (sound 5); Move and Equip greyed. Take damage, open the
   camp, Use a Tent: the confirm sound, HP to full on the status panel and after closing the camp,
   one Tent less. Camping Set with full HP but MP spent: usable, MP to full. Discard works.
4. The command menu's Items list never shows a Tent; nothing in battle uses one.
5. Barrel Spiders (KH1's 1 % / 0.5 %) may drop a Tent or a Camping Set (rare; a log line
   per drop). Enemies whose KH1 table has a Mega item (Search Ghost, Pirate, Air Pirate, Wight
   Knight, Screwdiver, Gargoyle) never drop one: the log says "removed … no prize".
6. With the asset step re-run, the names come from KH1's words (same text in English; the other
   languages KH1's).
7. Combat = Re:CoM, or Riku's story: nothing of this; Re:CoM's shop and item cards as before.

## M8d: the shop on Re:CoM's own screens

Owner (2026-11-04, `design-philosophy-instructions.md` § Moogle shop redesign at KH1): "the item
shop looks REALLY bad. … you can keep how the original menu looks, without the "buy, sell, card
packs" Ideally, selecting the left side (usually to buy card packs) it should bring up a menu
similar to screenshot 4, with all "card packs" replaced with the item card graphics of each item
sold. I'm not sure how to re organize screenshot 5 (the sell page,) but Ideally i would like to
keep the feel of the page, with the functionality to sell items of course"; answers: "Items tab,
then packs", "Count window, stock then slots", "just remove tents and camping sets from the game".
Combat = KH1 in Sora's story only; Combat = Re:CoM and Riku's story keep Re:CoM's shop unchanged.
Step 1 (0.124.0) removes the Tent and the Camping Set; the screens follow.

### M8d step 1: the Tent and the Camping Set removed

Version 0.124.0. The way the Mega items were removed in 0.105.0 (§ M8c stages 2–3, stage 3).

| what | where | tag |
|---|---|---|
| `id::REMOVED` = Mega-Potion, Mega-Ether, Megalixir, **Tent (142), Camping Set (143)**; `id::IN_PLAY` = Potion, Hi-Potion, Ether, Elixir. | `crate::items` | Checked (code) |
| **Not sold**: `shop::STOCK` = the four (1F Potion, Ether; 2F+ Hi-Potion; 7F+ Elixir). Lucky Bounty (`party::lucky_bounty`) reads the same tiers: unchanged (it never gave them). | `crate::shop` | Checked (code + tests) |
| **Not dropped**: `drops::spawnable` = `IN_PLAY`; the Barrel Spider's KH1 Tent (1 %) and Camping Set (0.5 %) give `Dropped::Removed`: nothing, logged. | `crate::drops` | Checked (code + tests) |
| **Never added**: `Inventory::pick_up` refuses any removed item (the shop, drops, Lucky Bounty and a returned use all go through it). | `crate::items` | Checked (code + tests) |
| **Not usable**: `items::effect` gives none; the Items page's `can_use` is false for them (the camp-only use path `camp_only` / `CAMPING_SET_FULL_MP` is gone). | `crate::items`, `crate::items_page` | Checked (code + tests) |
| **Not listed**: the Items page's Stock and Equip lists and the shop's Sell list read `IN_PLAY`. | `crate::items_page`, `crate::shop` | Checked (code + tests) |

**Owned ones.** The Mega conversion (`items::convert_removed`, checked) turns each removed item
into its replacement in its slot, or adds it to the replacement's stock up to 99 (the rest is lost
and logged), once, at the store's first read, and raises the block's version. It now runs on a v1
**or v2** block and writes **v3** (`EXT_V3`; v0, never written, stays KH1's new game; a v3 block
is left alone; older DLLs test the version only for 0). **Tent → Elixir, Camping Set → Elixir**:
the Mega rule (the item in play that does at least what it did for Sora): the Tent filled his HP,
the Camping Set his HP and MP (as built in 0.105.0), and only the Elixir fills HP. The Tents only
ever sat in the stock (kind 12), so they go to the Elixir stock. Log: `items: save block v2 -> v3:
removed items converted at Combat = KH1 (…): 0 slot(s), n from the stock, n past 99 lost`.

Kept: KH1's rows 142 / 143 in `items::DATA`, their stock bytes 8 / 9 (read by the conversion),
their English names, and the asset step's words 172 / 173 (unused now; no re-run needed).

No hook, site or patch; nothing per frame.

**Assumption**: a save opened in an older DLL (0.105.0..0.123.0) after this one could buy Tents
into a v3 block, which is never converted again (they would sit unlisted in the stock bytes).

**Owner question**: the replacement (built: both to Elixir). Other choices: (a) Tent → Hi-Potion,
Camping Set → Elixir; (b) refund their sell price (20 / 30 Moogle Points) instead.

**What to test** (Combat = KH1, Sora's story): load a save that has Tents / Camping Sets (an
`kh1_test_items` save from 0.105.0+ has 9 of each): the log line `save block v2 -> v3` once; the
camp Items page shows no Tent or Camping Set and the Elixir count rose by their number (to 99 at
most); the Moogle shop's Buy has no Tent / Camping Set (1F Potion / Ether; 2F+ Hi-Potion; 7F+
Elixir); Sell lists none; Barrel Spiders never drop them (log "removed … no prize"). Save, reload:
no second conversion line.

### M8d step 2: the buy screen (mode 4) with an Items tab first

Version 0.128.0. Combat = KH1 in Sora's story only (KH1's items live); Combat = Re:CoM and Riku's
story: Re:CoM's shop, unchanged (the tab tables are written back to Re:CoM's at each pack screen).

Code: `crates/kh1-recom/src/shop.rs` (stock, prices, the Items tab's input and places, the art's
manifest; tested), `src/win/shop.rs` (Re:CoM's side), sites `sites::SHOP_HOOKS`, `SHOP_CALLEES`,
`SHOP_CHECKED` (test `sites::tests::shop_sites`); the asset step `kh1-recom-assets` `shop_tabs`.

**Mode 3** (the two-Moogle menu): unpatched (the M8c list and its hook removed). Left Moogle →
mode 4, right → mode 5, first visit → Re:CoM's free pack, no room for 5 cards → Re:CoM's mode 2
(card trade offer): all Re:CoM's.

**Items with a full deck.** Re:CoM's mode-2 rule stays (no room: mode 3 offers only the card
trade), and Re:CoM's card trade reaches the pack screen only with room (its Square, `0x1403A752A`).
At Combat = KH1 the card trade's Square opens the pack screen also without room (a wrapper on mode
5's update `0x1403A73E0`: Re:CoM's sound 3, next mode 4, state 100, with Re:CoM's lock and cancel
read first), and its Square button hint (`CMShopButtonMessage` kind 1 at `+0x448`) is made as
Re:CoM's case 5 makes it (`0x1403A8561`) when Re:CoM made none. In the pack screen the packs keep
Re:CoM's own refusal (help line 5, sound 5); the Items tab sells. Path with a full deck: Moogle →
"trade cards" (mode 2's Yes) → Square → the pack screen on its Items tab.

**Mode 4, the wrapper on its update `0x1403A6930`** (8a.3's design; Re:CoM's states and order):

| what | how | tag |
|---|---|---|
| Opens on the Items tab | State 0 runs with the pack count `+0x348` at 0 (Re:CoM makes no packs), then our packs, the count = the cells, tab `+0x280` = 4 (ours), our refresh. | Checked (decomp: state 0 makes `+0x348` packs, waits for the windows) |
| The cells | Re:CoM's own `CMShopCardPack` ctor `0x14045A9E0(0xB8, +0x268, cell, price, 0)` per item of the floor's tier (1F Potion, Ether; 2F+ Hi-Potion; 7F+ Elixir: one page, ≤ 4 cells); the pack's art layer (picture control `+0x78` layer 1) hidden; Re:CoM's `C2DCard` of the item's card (0x2A Potion, 0x2B Hi-Potion, 0x2D Ether, 0x2F Elixir; class row `0x14068E0B0 + kind × 16`, 0x44) attached at (0, 0), priority 0x12; Re:CoM's price plate (`0x14045D120`) with the owner's price; never sold out. | Checked (decomp + exe); the card's look on the pack: not seen |
| Input | Re:CoM's state 2 replayed for the cells (`shop::items_step`): 0x80 → card trade (sound 3), cancel → mode 3 (sound 4); Left → Re:CoM's tab 3 (the row wraps), Right → tab 0; Down / confirm → the grid; in the grid confirm buys, cancel / Up from the top row back; moves by Re:CoM's table `0x14065BA70` over the cells. From Re:CoM's tabs: Left on tab 0 or Right on tab 3 → the Items tab (Re:CoM's earlier reads win). | Checked (disasm) |
| A buy | Price ≤ Moogle Points, KH1's pickup (first empty slot, else the stock to 99), else sound 5 and the reason on the line (nothing paid); sounds 2 + 0x58 and Re:CoM's payment (the spent tally and its 20000 trophy, `0x14023A1D0`, the Moogle Point window). One per confirm, never sold out. | Checked (decomp) |
| The Moogle's line | Re:CoM's card-pack lines (help 4 / 6, `0x1403A5AA0`) replaced on the Items tab through Re:CoM's text call `0x1402B0EF0` on the text window's `CTextWnd`: tab row "Buy items with Moogle Points."; on a cell "Restores HP. 10 Moogle Points. Sora has 2."; refused "Not enough Moogle Points." / "Sora can't carry any more of this.". The window's help id (`+0x5C`) is cleared so Re:CoM's tabs set theirs again. | Checked (decomp `0x1404573A0`) |
| Not changed | The help bar at the bottom (`CMShopHelpMessage` message 0x580B00: its text is in a message file not in the session; Assumption: button hints). | Assumption |
| Leaving | To Re:CoM's tab: cards deleted, our packs released (`+0x6C` = 1), Re:CoM's packs for the tab made as state 0 makes them, the count back, Re:CoM's refresh `0x1403A92F0`. The screen left from the Items tab (state 100): cards deleted first; state 0x65 releases our packs as its own, then Re:CoM's count back. | Checked (decomp) |

**The fifth tab's art** (asset step `shop_tabs`, Checked (files: the owner's `FO0109.SPR` and its
HD `UK_m_shop.lad`, decoded and rendered offline; decomp)):

- Re:CoM's tab row is two pieces of `m_shop:01` (SEQD 1 of `m_shop.lad`): the window with one tab's
  notch raised (`CMShopCardPackWindow` `0x14045AD20` plays groups 66 / 68 / 70 / 72 by tab from
  `0x140690420`, closes 67 / 69 / 71 / 73 from `0x140690428`; fill sprite group 139, frame 143) and
  the four pack icons (`CMShopGSeq` at `+0x408`, groups 56 / 57 / 58 = appear / loop / close: one
  tinted pack sprite at x 64, 128, 192 and the "all" stack about 256). The tab x table
  `0x14065BA40` (64, 128, 192, 256) places the cursor and the tab's button message; the only
  readers of both tables are the pack screen and its window (exe scan).
- The step appends 13 groups (Re:CoM's 241 unchanged): for 5 places 52 apart (64, 116, 168, 220,
  272) the window and its close (tab 0's parts copied, the notch's parts moved, the top border's
  two ends at the notch moved with it), and the icons' appear / loop / close: Re:CoM's 4 icons one
  place right and, first, **Re:CoM's own item icon** (the green bottle of the card trade's tab
  strip: group 181's 13th animation's sprite group 100, a 12 × 15 part drawn × 2.5), untinted. Only
  sprites already in `m_shop`'s atlas: **no texture edit** (`m_shop.imz` untouched). In the base
  file and in every `<LANG>_m_shop.lad` PC asset (each must have Re:CoM's 241 groups; the owner's
  UK copy has, with its own HD UVs).
- **Not built: a Potion card face as the tab's picture** (the owner's example): it would need the
  card's pixels added to `m_shop.imz`'s PS2 picture (8-bit, swizzled, its CLUT) and to each HD
  picture; the item icon Re:CoM already draws for item cards is used instead (owner question).
- The DLL (`kh1-shop-tabs.json`, format `kh1-recom-assets shop-tabs 1`): at each pack screen's
  state 0, at Combat = KH1 with the art, the tab x table → 116, 168, 220, 272 and the window table
  → the art's places 1..4 for Re:CoM's tabs 0..3 (`hook::patch` over Re:CoM's or our own values
  only); else Re:CoM's values. The icons' sequence gets the art's three groups (written over
  Re:CoM's 0x38 / 0x39 / 0x3A at `+0x72..+0x76`; if it already started, our appear is played). On
  the Items tab the window plays the art's place-0 window through its own control
  (`+0x68` → vtable `+0x18`, as `0x14045D250`) and its tab `+0x5C` = 4, so a Re:CoM tab replays
  its own; leaving the screen from it, the window's tab is 0 and tab 0's close entry the art's
  place-0 close (written back at the next screen).
- Without the art (no manifest, older output): Re:CoM's four-tab window, the Items tab's cursor
  and button message at x 320 (Re:CoM's step continued: Assumption), the window keeps the art of
  the tab the player came from.

**Sites**: hooks `0x1403A6930` (15 bytes) and `0x1403A73E0` (12 bytes; no branch into them, only
the ctor's record `lea`s: exe scan); callees: 8a.3's (refresh, pack ctor, price, sold out,
message / cursor place, cursor visible, point window, show state, payment, `C2DCard` × 6) + the
floor getter, the text call, the button ctor; checked: 8a.3's (state 2 head, tab row, grid
confirm, moves, payment, pack creation and release, the class rows of the four item cards, the
place tables, cursor y) + the window's tab play `0x14045D250`, the window table `0x140690420`, the
icons' set-up stores `0x140457EC1`, the card trade's Square `0x1403A752A`, its button creation
`0x1403A8561`, the sale sound. `check_sites_rs.py`: 1318 sites, 0 mismatches. Removed: the mode-3
init hook and the list's callees / checks.

**Per frame**: nothing outside the pack screen and the card trade. In the pack screen: one mutex
read; at Re:CoM's tabs in the input state the gate and (at tab 0 / 3 in the tab row) the pad reads
once more; on the Items tab Re:CoM's input state and refresh replayed (a few calls). In the card
trade without room at Combat = KH1: two pad reads in states 1 / 2.

**Logs**: `shop: KH1's items in Re:CoM's own Moogle shop screens (M8d): …`, `shop: five-tab art
from … m_shop:01 groups 241..253` (or `no kh1-shop-tabs.json`), then (80 lines at most) `Items tab
from tab 0: Re:CoM's 0 packs released, ["potion", "ether"] …`, `bought ether for 40: 500 -> 460
Moogle Points, Slot(0)`, `refused …`, `Items tab -> tab 3: Re:CoM's 2 packs made again`, `card
trade: Square to the pack screen without room for 5 cards (its Items tab)`.

**1:1 checks**: Checked (decomp + disasm + exe), by tests (`shop::tests`, `sites::tests::shop_sites`,
`shop_tabs::tests` incl. the owner's FO0109 files): Re:CoM's input order, sounds, moves, places,
payment, room rule; KH1's pickup; the art's parts (notch and border moves, icon places; the
patched files parse, refuse a second patch, keep Re:CoM's records). Not KH1's (the owner's): the
screens are Re:CoM's, the prices, the tiers. **Not checked**: in game (the cards' size and place on
the packs, the art in Re:CoM's draw, the icons' and window's timing at the screen's open).

**Assumptions**: the pack screen is always left through states 100 → 0x65 (8a.3's); the
window's control plays a group with Re:CoM's arguments (0, the lad, 1, group, 0.0, 0); the icon
sequence's groups are read only after its set-up (`0x1404585B0`); the help bar holds no card-pack
words; x 320 without the art.

**Open questions (owner)**: (1) the Items tab's picture: Re:CoM's green item icon (built), or a
Potion card face (needs a texture edit of `m_shop.imz`, PS2 and HD); (2) the bottom help bar's
text (Re:CoM's, not read).

**mod.yml** (the step writes `raw/FORM/0109/FO0109.SPR` and `kh1-shop-tabs.json`):

```yaml
- {name: raw/FORM/0109/FO0109.SPR, method: copy, source: [{name: raw/FORM/0109/FO0109.SPR}]}
- {name: kh1-shop-tabs.json, method: copy, source: [{name: kh1-shop-tabs.json}]}
```

**What to test** (Combat = KH1, Sora's story, a Moogle room, some Moogle Points):

1. Log: the M8d start line and `five-tab art from …` (after the asset step and mod.yml).
2. Talk to the Moogle: Re:CoM's two-Moogle menu (no Buy / Sell / Card Packs list). Left Moogle:
   the pack screen opens on the **first tab, the green bottle**, with Potion / Ether card pictures
   on packs with price plates 10 / 40 (2F+ also Hi-Potion 30; 7F+ Elixir 200); the line "Buy items
   with Moogle Points."; the tab row has five notches.
3. Down: the cursor on Potion; the line names it, its price and how many Sora has. Confirm: buy
   sounds, points drop, the count rises. Too few points / full: error sound and the reason.
4. Right: Re:CoM's red pack tab with its packs; Left from it back to the bottle; Left on the bottle
   → the "all" tab; Right on it → the bottle. Packs buy as before (cards got screen).
5. Square from any tab → the card trade; cancel → the menu.
6. With a full deck (no room for 5 cards): the Moogle offers the card trade; Yes; Square (its hint
   shows) → the pack screen on the bottle tab; items buy; packs refuse (Re:CoM's line).
7. Combat = Re:CoM or Riku's story: Re:CoM's four tabs, Re:CoM's packs, the tabs where Re:CoM puts
   them (also after a KH1 visit in the same session).
8. Report: the cards on the packs (size, place), the art (notches, icons) and the bottom help bar.

### M8d step 3 (0.130.0): the sell screen — an Items tab first in Re:CoM's card trade

Owner, 2026-11-04 (answers to the step-3 question): (1) "our own Up/Down count step on Re:CoM's
yes/no, 1..owned, the points box shows the total, Re:CoM's yes/no sells them, stock first then
slots"; (2) the Items tab "uses the same green bottle icon, placed **first** in the trade's strip"
(new strip groups from the asset step, as step 2), help line "Sell items"; (3) Re:CoM's green item
icon kept, no texture edit; (4) the Tent / Camping Set conversion: unanswered, Elixir kept. Combat
= KH1 in Sora's story; Combat = Re:CoM and Riku's story: Re:CoM's card trade untouched.

**Re:CoM's card trade** (mode 5: init `0x1403A4D90`, update `0x1403A73E0`), Checked (decomp +
disasm + exe). Its windows (by RTTI and ctor): `+0x450` `CMShopChargePointWnd` (the points a sale
gives, `0x14045D2C0`), `+0x458` `CMShopChargeCardRankWnd` (the 0–9 box and a small card,
`0x14045D9C0`), `+0x460` `CMShopCardInfoWnd` (the big card, `0x14045A260(kind)`), `+0x468`
`CMShopStockCardWnd` (the 3 × 3 grid, the scroll bar, the tab strip from `0x1406904D0[tab]` on its
control `+0x80`, the "cards in" counts; filled by `0x14045DA80(window, data, tab, scroll, 0)`),
`+0x448` the Square hint. Its data `+0x350` (`CMShopChargeCardData`, built by `0x14045D510`): `+8`
298 records of 16 bytes (u16 kind, u16 count, category byte, the count per value 0..9), `+0x10 +
tab × 4` the cards per tab (4 = ALL), `+0x24..+0x30` per category, `+0x44` the cards for the room
rule. Fields: `+0x358` grid, `+0x35C` tab (opens on 4 = ALL), `+0x360` scroll, `+0x364` cell,
`+0x368` value, `+0x270` yes / no, `+0x274` the input lock; cursors `+0x3D0` (the finger),
`+0x3D8` (the grid finger), `+0x3E0` (the value finger); `+0x3C8` the help bar; `+0x3E8` the text
window. States: 1 the tab row / the grid (cancel leaves the trade from both; on the tab row Left
`0x408000` / Right `0x802000` repeats, sound 1, wrap 0..4; a fresh Up / Down `0x305000` enters the
grid, sound 0xE: Down at cell 0 (`0x1403A76D8` clears scroll and cell as one qword), Up at the last
(`0x14045BAE0`); in the grid confirm (sound 2, state 2), Up / Down / Left / Right repeats with
wraps on fresh presses, `0x14045BB90` / `0x14045C890`); 2 the 0–9 box (confirm: sound 2, the text
window's message 7, the finger at 232 / `0x14065BA38` (214, 237), state 3); 3 Re:CoM's yes / no
(Yes: sound 6, state 5; No / cancel: state 4 → 2); 5 the sale once the text window is closed
(`+0x58` = −1): its price `0x14015A6B0`, the points `0x140239FA0` (adds, caps at 99999, its 9999
check), the point window `0x1404598A0`, the card taken `0x140159B20`, the data rebuilt. Its tab
finger `0x14045B950`: the layout point − 62 for ALL (`subss xmm1, [0x1405092A8]` at `0x14045B9D7`),
− 14 + 24 t for tab t.

**Built** (`crate::shop::sell`, `win::shop`; the hook `0x1403A73E0` of step 2):

- **The Items tab**, first in the strip, before ALL; the trade **opens on it** (as the pack
  screen's): in the grid when Sora has items (Re:CoM opens in the grid when ALL has cards). It holds
  Sora's items (`shop::sell_list`: Potion, Hi-Potion, Ether, Elixir that Sora has) as Re:CoM's item
  cards: a card list of ours in Re:CoM's data layout (records kind 0x2A / 0x2B / 0x2D / 0x2F, the
  count, category 2; the per-category counts copied from Re:CoM's, so the window's "cards in"
  numbers stay Re:CoM's), handed to **Re:CoM's own fill** `0x14045DA80` as tab 4 (every record);
  Re:CoM's big card (`0x14045A260`) and small-card box (`0x14045D9C0`, the record's kind, the value
  cells blank: items have no values) for the cell; with the art, the Items-lit strip played on the
  window's strip control after the fill.
- **Entering / leaving**: Left on ALL or Right on tab 3 (the row wraps through it) → the Items tab;
  from it Left → tab 3, Right → ALL (Re:CoM's tab move: its fill, its blank box, its finger, the
  help bar's id 5 back). Re:CoM's earlier reads win (cancel, the 0x80 button), Left over Right.
- **On the Items tab** Re:CoM's state 1 replayed for our list (`shop::sell::tab_row` / `grid`:
  Re:CoM's order, sounds 1 / 0xE / 2 / 4 / 3 / 5, wraps, the step back onto a card in a shorter
  row, the lock), Re:CoM's cursors (`0x140459960`, `0x140459D60`, `0x1404597D0`, the cell finger
  `0x14045B8B0`, the box's spin `0x14045B810`), the charge point window blank; the help bar's line
  "Sell items" (owner) through Re:CoM's text set `0x140258DE0` on its text (`+0x78`). Cancel leaves
  the trade (as Re:CoM's), Square opens the pack screen (with or without room at KH1).
- **The count step** (Re:CoM's state 2 replaced while the tab is up; `shop::sell::count`): Up one
  more, Down one less (held: stops at 1 / the count owned; fresh at an end: wraps), sound 0xE; the
  charge point window shows the total (sell price × count); the grid finger stays in Re:CoM's
  chosen look (4). Cancel: back to the grid (sound 4). Confirm (sound 2): **Re:CoM's yes / no** as
  Re:CoM opens it (message 7, the finger at 232 / 214, state 3), its main line ours once shown
  (Re:CoM's text call `0x1402B0EF0`): "Sell Potion x3 for 6 Moogle Points?". Re:CoM's states 3 / 4
  run as Re:CoM's (Yes: its sound 6 and the Moogle's animation; No: back to the count).
- **The sale** (Re:CoM's state 5 replaced while the tab is up, once its text window is closed):
  the count sold one by one by `shop::sell` (the stock first, then the last slot holding it),
  the total through **Re:CoM's own add** `0x140239FA0` and its Moogle Point window; the list and
  Re:CoM's windows refreshed; back to the grid at the same cell (one back when the last item sold
  out), or to the tab row with nothing left. Re:CoM's own sale returns to its 0–9 box while the
  card has copies; ours returns to the grid (KH1's shop goes back to its list after a sale).
- **The art** (asset step `shop_tabs` § `strip`, 6 more groups: `m_shop:01` 254..259): Re:CoM's
  five strips with ALL's place split in two, the Items cell (−82..−58, Re:CoM's bottle at −70, tab
  0's fill) and ALL (−58..−26, its fill's middle shortened, its text at −42 narrowed to 28 wide);
  Re:CoM's four tabs untouched; the outline per layout style (the PS2 layout: Re:CoM's outline and
  a separator at −58; the PC layouts: the Items and ALL pieces cut from Re:CoM's own pieces as new
  sprite records over the same atlas rectangles, the bar line stretched as Re:CoM stretches it);
  plus the Items-lit strip. At each trade start (state 0) the DLL writes Re:CoM's strip table
  `0x1406904D0` to the art's groups 254..258 and points Re:CoM's ALL finger instruction
  `0x14045B9D7` at Re:CoM's own float 50.0 (`0x140509298`, beside the 62.0) — Re:CoM's ALL finger
  in the new ALL cell — and draws the stock window again when the table changed; Re:CoM's values
  at Combat = Re:CoM. The Items finger: Re:CoM's ALL place − 20 (the bottle). Without the art:
  Re:CoM's strip (ALL lit while on the Items tab), the Items finger one tab step right of tab 3
  (Assumption).

**Sites**: no new hook. Callees: the stock window fill, the box, the big card, the charge point
window, the tab and cell finger places, the box spin, the cursor move and look, the text window's
message, the help bar, the Moogle Point add (the text set is checked by the HUD's). Checked: the
tab row `0x1403A75A5`, the grid confirm `0x1403A7857`, the yes / no open `0x1403A7D5E`, the sale's
points `0x1403A8295`, the ALL finger instruction `0x14045B9D7` and its two floats, the strip table
`0x1406904D0`, the yes / no finger's y `0x14065BA38` and x `0x14065BCEC`. `check_sites_rs.py`:
1370 sites, 0 mismatches.

**Per frame**: nothing outside the card trade; in it at Combat = KH1 one mutex read; at Re:CoM's
tab row the pad read once more (ALL / tab 3); on the Items tab Re:CoM's state 1 / 2 replayed (a few
calls of Re:CoM's).

**Logs** (80 lines at most): `card trade: Items tab (opened on), ["potion x3", …], in the grid
true; 120 Moogle Points`, `card trade: sell Potion x3 for 6? (…)`, `card trade: sold potion x3 of 3
for 6: 120 -> 126 Moogle Points`, `card trade: Items tab -> Re:CoM's tab 4`, `card trade left from
its Items tab`.

**1:1 checks**: Checked (decomp + disasm + exe), by tests (`shop::tests` incl. `entering_the_sell_tab`,
`sell_tab_row`, `sell_grid_moves_as_recoms`, `the_count_step`, `sale_totals_and_lines`;
`sites::tests::shop_sites`; `shop_tabs::tests` incl. the owner's files): Re:CoM's input order,
sounds, grid moves, wraps, the yes / no's opening and places, the points add, the strip's groups
and finger constant. KH1's: the sale prices (step 1: buy ÷ 5; KH1's for the Mega items, not in
play). Not KH1's (the owner's): the screen is Re:CoM's, the count step ours. **Not checked**: in
game (the card list in Re:CoM's window, the cut pieces in Re:CoM's draw, the finger places, the
text on the yes / no, the help bar's text object).

**Assumptions**: the stock window's fill and the box read nothing of the data beyond what
`0x14045D510` writes (Re:CoM's fields copied for `+0x24..+0x4C`); the strip control plays with
Re:CoM's arguments (as step 2's window); the finger's x is in the strip's coordinates (Re:CoM's tab
fingers sit at their icons); the help bar's `+0x78` text object takes our line as Re:CoM's message
text (ASCII); the PC layouts of every language have the UK pieces' rectangles (the asset step
refuses otherwise); the trade is always left through state 100.

**mod.yml**: no new entries (the step-2 outputs `raw/FORM/0109/FO0109.SPR` and
`kh1-shop-tabs.json` now carry the strips; their format is `shop-tabs 2`: **re-run the asset
step**, an older manifest turns both Items tabs' art off).

**What to test** (Combat = KH1, Sora's story, a Moogle room, a few Potions / Ethers):

1. Log: `shop: tab art from … m_shop:01 groups 241..259` (after the asset step).
2. Right Moogle (the card trade): it opens on the **first cell, the green bottle**, the cursor on
   the first item card in the grid, the big card and small card showing it; the help bar "Sell
   items". The strip: six cells (bottle, ALL, Re:CoM's four).
3. Move in the grid (Re:CoM's moves and sounds). Up from the top row: the finger on the bottle.
   Left: tab 3 (the heart); Right from the heart: the bottle; Right from the bottle: ALL; Left from
   ALL: the bottle. Re:CoM's tabs show their cards as before.
4. On an item: confirm; Up / Down change the count (1..owned, wraps); the points box shows the
   total. Confirm: Re:CoM's yes / no with "Sell Potion x3 for 6 Moogle Points?". Yes: the sale
   sound, the points rise by the total, the count drops (stock first, then slots: the item menu's
   slots keep their items until the stock is gone). No: back to the count.
5. Sell an item out: the grid closes up; sell everything: the tab row.
6. Square → the pack screen; cancel → the two-Moogle menu.
7. Combat = Re:CoM or Riku's story: Re:CoM's five-cell strip, Re:CoM's ALL finger place, cards
   sell as before (also after a KH1 visit in the same session).
8. Report: the strip's look (cells, separator, ALL text), the finger places (bottle, ALL), the
   card pictures in the grid, the box / big card on an item, the yes / no line.

**Open (owner)**: the Tent / Camping Set conversion (question 4).


### M8d: the bubble's lines fit (0.132.0)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Shop text, lock-on and camera), verbatim:
"The moogle shop text is over extending past the text bubbles." (screenshot: the buy screen's
Items-tab line "Restores HP. 10 Moogle Points. Sora has 11" past both sides of the orange bubble).
Decided: wrap onto the bubble's lines as Re:CoM's own two-line lines do; squeeze only if a line
still can't fit. Combat = KH1 in Sora's story (where our lines exist); Re:CoM's own lines untouched.

**How Re:CoM's bubble works** (Checked (decomp + disasm + exe); `sites::BUBBLE_CHECKED`):

| what | where |
|---|---|
| The Moogle's text (`CMShopTextWindow`, `+0x60` / `+0x68` two `CTextWnd`s) opens a `CTextWnd` per message id (`0x1404595A0(n)`: ids `0x14068EF58[n]` = 0x580200, 0x580300, 0x580000, 0x580100, 0x580500, 0x581000, 0x581100, 0x580600; a second window `0x14068EF78[n]` for 1 (0x580400) and 7 (0x580700, the yes / no); style 0x11, 0x15 for 7). A help id change on the same window only sets a new line on it (`0x1404573A0` → `0x1402B0EF0`). | `0x1404595A0`, `0x1404573A0` |
| `CTextWnd`'s open `0x1402B1520` reads its message's record (`0x140240080`): **the bubble's size** = record `+0x1A` / `+0x1C` (rounded up to even) → `+0xB8` / `+0xBC`, the bubble `CWnd` (`message.sqd`) drawn at that size (`0x1402B1F70`); **the text margins** `+0xB0` / `+0xB4` (record `+0x28` / `+0x2A`); font size (record `+0x22`), spacing and line gap (`+0x24` / `+0x26`). | `0x1402B1520`, `0x1402B1F70` |
| Its `CText` (`+0xD0`, `0x1402B3AD0` → `0x1402B1D50`) sits at the window + the margins with the window's size; its text is a `CStrPrim` (`+0x38`; a CFntM2 with vtable `0x140508FD0`): size × 16 at `+0x3C`, spacing `+0x44`, gap `+0x48`; it measures the text (vtable `+0x28`) and keeps the line count (`+0x54`). It places the text each frame (`0x1402B0820`): x = the CText's x, + its width for right-aligned, + half for centred (`+0x40 & 3`); the owner's screenshot (past both sides) fits centred. | `0x1402B1D50`, `0x1402B0820` |
| **Re:CoM's text call `0x1402B0EF0` sets a new line on the same `CText` and never resizes the bubble**: a line longer than the message's own runs past it. That is the overflow. | `0x1402B0EF0` → `0x1402B11D0` |
| **A new line is byte 10** (`\n`): Re:CoM's width routine `0x14025A380` starts a new line on it (height = n × size + (n − 1) × gap) and the draw follows its line table. Re:CoM's messages are authored with their breaks (the owner's two-line "Which one do you want to buy, kupo?"). Nothing wraps on its own. | `0x14025A380` |
| `%` in a line is expanded by the message format (`0x1402402B0`); without `%` Re:CoM keeps **our pointer** (the line stays alive in our state, as before). Our lines have no `%` (test). | `0x140259000`, `0x1402402B0` |

**Built** (`crate::text_fit::wrap`, `win::shop::bubble_line`, `win::text_fit::{width_of, set_level, unsqueeze}`):

- **Our lines** (`crate::shop`), a `\n` where we prefer a new line (as Re:CoM's two-line lines):
  - pack screen, Items tab: "Buy items with Moogle Points."; on an item **"Restores HP." / "10 points.
    Sora has 11."** (Hi-Potion "Restores more HP.", Ether "Restores MP.", Elixir "Restores HP and
    MP." (was "Restores all HP and MP.")); refusals "Not enough Moogle Points.", "Sora can't carry
    any more of this.";
  - card trade, yes / no: **"Sell Potion x3" / "for 6 Moogle Points?"**.
- **The box**, read from the bubble the line goes in (when it is shown): width = the bubble's width
  − both text margins − 2 units each side; lines = as many as its height − both margins holds at
  its size and gap, or the lines Re:CoM's own text in that bubble had, if more (read before our
  first line there). Assumption: the margins are the same on both sides; logged once per bubble
  size so a screenshot can check it: `shop: bubble (pack screen, Items tab): W x H, text margins
  mx / my, font s / gap g: box B units, N lines (by its height n, Re:CoM's own text there k)`.
- **The layout** (Re:CoM's width routine `0x14025A380` on the bubble's own `CStrPrim`, the text
  expanded as the draw does; one call per candidate line, only when the line changes, never per
  frame): (1) our lines, each wrapped at the box at spaces; (2) else the whole line wrapped at the
  box (our breaks dropped); (3) else the **true squeeze** (0.113.0's): the narrowest wrap that takes
  the bubble's lines (ours kept when there are no more of them than it holds), drawn at box /
  its widest line. Logged: `shop: bubble (…): "…" -> "…", n line(s), widest w units of B[,
  squeezed x s]`.
- **The squeeze on the bubble's text**: a copy of `CStrPrim`'s vtable (`0x140508FD0`: COL and 7 slots
  kept, draw `+0x18` and measure `+0x28` ours, as C2DFnt's copies; both classes share Re:CoM's
  draw `0x140289450` and measure `0x140289440`; `sites::TEXT_FIT_STR_PTRS`, checked lazily: a
  mismatch turns only this squeeze off), set before Re:CoM's text call so its measure sees the
  drawn width. **Re:CoM's own lines are never squeezed**: Re:CoM's vtable goes back when we hand the
  bubble back (`forget_help`), and our measure puts it back itself when Re:CoM sets any other text
  on that object (its text call measures right after setting). Every code reference to the
  vtable is a store (13; no absolute pointer: Checked (exe scan)), so no Re:CoM code tells the class
  by it.
- Not changed: the bottom help bar's "Sell items" (short; Re:CoM's own help-bar lines are longer).

**Longest numbers** (test `shop::tests::every_line_fits_a_two_line_bubble_at_the_longest_numbers`):
107 owned (99 in the stock + 8 slots), Elixir's 200, a 107-item sale (Elixir 4280). In a model
bubble only as wide as the narrower two-line split of Re:CoM's own "Which one do you want to buy,
kupo?" (10 units a glyph, 5 a space: Re:CoM's fonts are not in the session), every line takes two
lines; the longest ("200 points. Sora has 107.", "for 4280 Moogle Points?") squeeze to about 0.72
there. The real widths are Re:CoM's, measured in game (the log lines above).

**Sites**: `TEXT_FIT_STR_PTRS` (8 vtable words), `BUBBLE_CHECKED` (4 byte sets: the margins, the size,
the CText's font fields and line count, the text call's CText and shown state). No hook, no patch.
`check_sites_rs.py`: 1382 sites, 0 mismatches.

**Per frame**: nothing new (the layout runs when a line changes; a squeezed bubble draws through
our draw: about 40 instructions a glyph, as 0.113.0's).

**1:1 checks**: Checked (decomp + disasm + exe): the bubble's fields, the newline code, the class
and its vtable's users; tests (`text_fit::tests` wrap / lines, `shop::tests`, `sites::tests::bubble_sites`).
Not KH1's (KH1's shop has no bubble): Re:CoM's own look. **Not checked**: in game (the bubble's real
width, the margins' symmetry, Re:CoM's font widths, the lines each line wraps to).

**What to test** (Combat = KH1, Sora's story, a Moogle room):

1. Pack screen (left Moogle), Items tab: the tab-row line, then each item (Down): the line in two
   lines inside the orange bubble ("Restores HP." / "10 points. Sora has n."); the refusals.
2. Card trade (right Moogle), Items tab: an item, a count, confirm: "Sell Potion x3" / "for 6 Moogle
   Points?" inside the bubble.
3. With 99 + full slots of an item (e.g. the test inventory) the longest lines still inside.
4. Re:CoM's own lines (other tabs, card packs, Combat = Re:CoM) as before, never narrowed.
5. Report: the log's `bubble (…)` lines (box and lines) and a screenshot of each bubble.
