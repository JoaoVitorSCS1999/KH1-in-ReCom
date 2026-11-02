# Story skips at Combat = KH1: the intro tutorial and Leon's training battle

Ask (owner, 2026-11-01): "If a player has KH1 Combat enabled, have the player skip the leon training
battle. Also the intro tutorial."

Status: **research only, no code.** Both tutorials are run by Re:CoM's event scripts (`.ABC` files).
Those scripts aren't in this session, so the skip can't be built natively yet. This note gives what
the exe shows, the plan, and the exact files needed.

Tags: Checked (exe / decomp / files) or Assumption. Steam addresses.

## 1. What runs the tutorials (exe and decomp)

### Event scripts

- A room's event script is a `/abc` file. The loader `0x14014B8A0` (first bytes
  `48 89 5C 24 08 57 48 83 EC 20 81 3A 2F 61 62 63`) checks the magic `0x6362612F` ("/abc"). Its
  layout: entry count, 64 entry ints, an instruction count, the 8-byte instructions, a data size and
  its bytes, then a thread count and 0x1C0-byte thread blocks. Checked (decomp + exe).
- The `CAtel` event object is built per room by `0x1401AD7B0`. It takes the room resource's script
  through `0x1402A46F0` → `0x140135010` (VM `0x140141830`, load `0x14014B8A0`). The VM steps once
  per frame through `0x140143290`. Checked (decomp).
- Instruction: `{u16 op, i16 relative, i32 operand}`. `0x140143320` sends ops below 0x3E to
  `0x14014A2F0`, 0x3E..0x63 to `0x1401436F0`, 0x6E..0xA7 to `0x140144840` and 0xA8..0x112 to the
  syscall switch `0x140145C50`. Checked (decomp).
- Variable ops (`0x1401436F0`): 0x53 / 0x54 get / set int (set 0), 0x55 / 0x56 byte (set 0),
  0x57 / 0x58 bit (set 0), 0x5F / 0x60 int (set 1), 0x61 / 0x62 byte (set 1), 0x63 get bit (set 1).
  The set-1 bit write op (used for (1, 0x2C) and others) is outside this function: not located.
  Checked (decomp) for the listed ops.
- **Only save-variable set 0 is saved; set 1 is session-only** (research note
  `recom-cards-save.md` §4, kept with the decomp tools). All the tutorial flags below are set 1.
  Checked (decomp).
- Script files in the package, from the exe's file lists and `Recom.hed` (md5 of the path):
  `WORLDww/rrrr/WOwwrrrr.ABC`. World 00 (Castle Oblivion): 123 `.ABC` (rooms 0001..0225, event
  battle rooms 1050..1058 and 1100..1103, event rooms 2001..2123), plus 2104 found only in the hed.
  World 01 (Traverse Town): 42 `.ABC` plus 0024, 1002, 1003, 1500 found only in the hed. Checked
  (exe strings + hed). **ABC = the event script: Checked** (the loader's magic); which room holds
  which scene: Assumption.

### Event battles

- Syscall 0xAD sub 10 starts an event battle: `0x1401A00D0(group, 0)` writes the room's battle record
  (`0x140233920()`, `+0x44` / `+0x45` the group, `+0x46` 0), then `0x140236690(0)` requests the move to
  the battle room. First bytes: `0x1401A00D0` `48 89 5C 24 08 57 48 83 EC 20 0F B6 FA 8B D9`,
  `0x140236690` `48 89 5C 24 08 57 48 83 EC 20 0F B7 F9`. Checked (decomp + exe).
- Battle rooms 1050..1198 are event battle rooms: the battle manager `0x1401871D0` case 1 does **not**
  end those battles when the enemies are gone (`room − 0x41A > 0x94` test). Their own script ends them
  (syscall in `0x140145C50` → `0x140237160`). Checked (decomp). The tutorial rooms are in this range:
  Assumption (World 00 and World 01 both have an `.ABC` for room 1050).
- Enemy ids: **100 = `CTutorialHood`** (ctor `0x1403A94F0`), **101 = `CTutorialLeon`** (ctor
  `0x1403A9BA0`), from the enemy factory `0x140188F40`. Checked (decomp).

### What the tutorial enemies and objects do (all scripted)

- `CTutorialHood` update `0x1403A97E0`: steered by the script's int variable **(1, 0xE1)** (steps 1, 2,
  3, 6, 7, 9 move, turn, dodge or make it unhittable; 7 also calls `0x1401E6990(player)`); sets bit
  (1, 0x18) after Sora's target cursor has held it (60 units of `dt × 0.02`). Checked (decomp).
- `CTutorialLeon`: plays a card only while Sora's action is 0xB (`0x1403A9CE0`); its message handler
  `0x1403A9F40` sets (1, 0x1B). Checked (decomp).
- `CAtelTutorial` (`0x140137CB0`, script 0xAD sub 0x10) sets (1, 0xE) roll, (1, 0xD) jump, (1, 0x11)
  hand empty. The deck sets (1, 0x12..0x17). (1, 0x2C) = tutorial deck: **read** by the exe at
  `0x140164E4B`, `0x140165E9B`, `0x1401F4413`, **never written by the exe**, so a script sets it.
  Checked (decomp + exe call scan). (`recom-m6-battles.md` § The tutorial battles.)
- Rewards a script can give (syscall 0xAD): sub 5 a card (`0x140159950` → `0x1401F4AB0(id, n, 2)`),
  sub 6 `0x140159B20`, sub 7 the level-up / new-ability screen `0x14010E9C0`, sub 0x13 `0x14023AB90`,
  sub 0x15 `0x1402369D0`; each is skipped while bit (1, 0x33) is set (set by the theater mode
  `0x140454970`). Checked (decomp). What each gives exactly: Assumption.
- New game `0x1403A24B0` → `0x1401AD2A0`: Sora / Riku byte (0, 0x60), starting deck `0x14015A7A0`,
  stats `0x14023A040`, then floor index 0, door 2 (`0x140236440(0, 2, -1)`). No tutorial option in
  the new-game data (only (0, 0x61), (0, 99) and the story byte). Checked (decomp); "no option":
  Assumption (the new-game menu text wasn't read).

### What is not known without the scripts

1. Which rooms and which script steps start the intro tutorial and Leon's battle (the group passed to
   0xAD sub 10, or another path).
2. What the scripts **check** to know the tutorial is done (a set-0 int or bit, a room or floor
   step), and what they **write** when it ends: story variables, cards, the Traverse Town card, the
   starting deck, friend cards, Moogle / shop unlocks, the move to the next room.
3. Whether the tutorial's instruction screens (`gamehelp.lad` and others) are opened by the field
   script or by the battle room's script.
4. Whether any later script reads a value only the tutorial leaves.

Without 2 the skip would guess what to write. A wrong guess can re-trigger the battle on every room
entry, soft-lock the opening, or leave a save that differs from a played one. So no code was written.

## 2. Plan (once the files are in the session)

1. Disassemble the opening rooms' and Traverse Town's scripts (op table above). Find the tutorial
   steps, the 0xAD sub 10 calls and the set-0 variables written after each tutorial.
2. **Take Re:CoM's own "already done" path.** Room scripts have to cope with a player who comes back
   to a room after its event, so each scene is gated by a saved variable. Preferred design: at
   Combat = KH1, in Sora's story, set those gate variables (and give exactly what the tutorial gives)
   **through Re:CoM's own setters** (`0x1401B16C0` / `0x1401B1650` / `0x1401B15D0`) and reward
   functions, at the moment the script would start the tutorial. The script then follows its own
   "already done" branch. One hook point, not per frame: either Re:CoM's event-battle start
   (`0x1401A00D0`, only for the tutorial groups) or the script load `0x14014B8A0` for the tutorial
   rooms. Both first-byte sets are above (exe-checked, ≥ 10 bytes each).
3. If a script has no such branch (the tutorial is one straight scene), the fallback is still native:
   at KH1, the event-battle start for the tutorial group returns as Re:CoM's own battle exit does,
   and the variables the battle room's script would have written are written the same way. Asked
   before shipping if it comes to this.
4. Cutscenes and dialogue that are not the tutorial battles stay. Instruction screens that belong to
   the tutorial are skipped with it (their steps are part of the skipped scene). Final choice
   documented here once the scripts are read.
5. Combat = Re:CoM: nothing is written and the hook passes through. Riku's story: untouched.
6. Saves: the skip writes only what playing the tutorial writes, in the same set-0 variables, so a
   save made after the skip is the same as one made after playing it. Switching back to Re:CoM later
   is then safe. Set-1 tutorial flags (0x11..0x18, 0x1B, 0x2C, 0xE1) are not saved, so they don't
   matter for saves. To be confirmed against the scripts.
7. Tests: the pure part (which groups / rooms, which writes) in unit tests. In-game check: a new game
   at Combat = KH1 goes from the opening straight on without the card-battle tutorial, Traverse Town
   without Leon's battle; the story continues; a save made there loads at Combat = Re:CoM and plays
   on.

## 3. Files needed (from the owner's Re:CoM install, `Image/dt/Recom.pkg`)

Not for the repo; for the session only.

- Required: every `.ABC` of World 00 and World 01, that is `WORLD00/*/WO00*.ABC` and
  `WORLD01/*/WO01*.ABC` (170 files in the hed: about 150 KB in the package, about 1 MB
  unpacked). They include
  `WORLD00/2104/WO002104.ABC`, `WORLD01/0024/WO010024.ABC`, `WORLD01/1002/WO011002.ABC`,
  `WORLD01/1003/WO011003.ABC` and `WORLD01/1500/WO011500.ABC`, which are in the hed but not in the
  exe's lists.
- Helpful: the `.ESD` (enemy sets) of those worlds' battle rooms, at least
  `WORLD00/1050/WO001050.ESD` and `WORLD01/1050/WO011050.ESD` (they show which group holds enemy
  100 / 101), and the `.CTD` (text) of the same rooms, to tell the help screens from the story
  dialogue.
- Already in the session: `SYS/0001/SY0001.BIN` (Rooms1, EventRooms1, CardInit).
