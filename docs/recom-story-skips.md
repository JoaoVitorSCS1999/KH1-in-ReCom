# Story skips at Combat = KH1: the intro tutorial and Leon's training battle

Ask (owner, 2026-11-01): "If a player has KH1 Combat enabled, have the player skip the leon training
battle. Also the intro tutorial."

Status: **research done, no code.** Both tutorials are battles that event scripts start by
battle-room number, and neither script has an "already done" branch. So the skip needs a fallback
design. The options are in § 4, waiting for a choice.

Tags: Checked (exe / decomp / files) or Assumption. Steam addresses. "Files" means the owner's
`WORLD00` / `WORLD01` `.ABC` / `.CTD` / `.ESD` files, read in the session only. Nothing from them is in
the repo.

## 1. Event scripts (exe and decomp)

- A room's event script is an `.ABC` file. Its container is the archive layout `SY0001.BIN` uses
  (`{u32 offset, name[0x18], u32 size | 0x80000000}`), holding one `/abc` program. The loader
  `0x14014B8A0` (first bytes `48 89 5C 24 08 57 48 83 EC 20 81 3A 2F 61 62 63`) checks the magic
  `0x6362612F`. Program layout: entry count, 64 entry ints, instruction count, 8-byte instructions,
  data size and bytes, thread count, 0x1C0-byte thread blocks. Checked (decomp + files).
- `CAtel` (`0x1401AD7B0`) loads the room's script: `0x1402A46F0` → `0x140135010` → VM `0x140141830` →
  `0x14014B8A0`. It is stepped by `0x140143290`. Checked (decomp).
- Instruction: `{u16 op, i16 frame-relative flag, i32 operand}`. `0x140143320` dispatches:
  - below 0x3E to `0x14014A2F0`: stack and flow. 0x1C push, 0x06 add immediate, 0x1A / 0x22 load /
    store, 0x20 pop, 0x29 jmp, 0x2C / 0x2D jz / jnz, 0x30 / 0x31 call / ret, 0x39 end;
  - 0x3E..0x63 to `0x1401436F0`. Its variable ops are 0x53..0x58 (set 0 int, byte and bit get / set)
    and 0x5F..0x63 (set 1 int / byte set and get, bit get);
  - 0x6E..0xA7 to `0x140144840`. 0x6E is the **set-1 bit write** (`0x1401B1650(1, id, v)`);
  - 0xA8..0x112 to the syscall switch `0x140145C50`.

  Checked (decomp; ops decoded against the files).
- Syscalls used here (`0x140145C50`), Checked (decomp):
  - **0xEE(room)**: battle entry to battle room `room` in the current world (`0x140236690(room)`).
    It also sets (1, 5) when the current room is below 2000.
  - **0xF7(a, b, room)**: room change `0x1402360F0(a, b, room)`; b = −1 → `0x140233AE0()`.
  - 0xFE(x, op, y): bit maths (or / and / xor / shifts / and-not).
  - 0xAD(sub, a, b): sub 5 gives a card, 7 shows a card-get screen, 0x10 makes `CAtelTutorial`,
    0x1E `0x14023C260(a, b)`; sub 0xC has no case (does nothing).
- Only save-variable set 0 is saved; set 1 is session-only (research note `recom-cards-save.md` §4,
  kept with the decomp tools). Checked (decomp).

## 2. Where the two tutorials are (files)

Sora's opening in Traverse Town (world 1) is a chain of event rooms (2000+) and event battle rooms
(1050..1198, which the battle manager `0x1401871D0` doesn't end on its own). Checked (files + decomp):

| Step | Room (world 1) | Script does |
|---|---|---|
| Opening scene (text `CTtr00`) | event 2000 | sets friend bits (0, 0x22) ∣= 1 ∣ 2 (Donald, Goofy), then **`0xEE(1100)`** at the end of the scene and at the end of a second path (Assumption: the scene-skip path) |
| **Intro tutorial** (hooded figure, text `CTtr01`) | battle 1100 | see below |
| Scene after it (text `CTtr02`) | event 2001 | |
| Scene with Leon (text `CTtr02`) | event 2002 | **`0xEE(1101)`** at the end of the scene and at the end of a second path (Assumption: the scene-skip path) |
| **Leon's training battle** (text `CTtr03`) | battle 1101 | see below |
| Scene after it (text `CTtr04`) | event 2003 | gives the floor's cards and the card-get screens (0xAD 5 / 7) |

- The intro tutorial's battle room 1100 (`tr_1100.abc`):
  - At its start it sets the tutorial session bits (1, 0x2C) (tutorial deck), (1, 0xE1) = 0 and
    (1, 0xE2) = −1, makes `CAtelTutorial`, and clears friend bits (0, 0x22) &= ~1, ~2.
  - Then it runs the lesson.
  - **Lesson exit**: session bits cleared, friend bits set again (∣= 1 ∣ 2), **`0xF7(0, 0, 2001)`**.
  - **Won exit** (bit (1, 2), set by the battle manager's `0x140182240`): session bits cleared,
    `0xF7(0, 0, 2001)`. This exit does not set the friend bits again.
- Leon's battle room 1101:
  - At its start it sets (1, 0x2C), (1, 0x2F), (1, 0xA) and (1, 0xE2) = −1.
  - Then it runs the lesson: card values, card break, stocking.
  - **Lesson exit**: session bits cleared, (1, 0x32) = 1 (`0x1401AF9D0` and `0x1401ADD20` then keep
    the music playing into the next room), **`0xF7(1, −1, 2003)`**.
  - **Won exit** ((1, 2)): the same without (1, 0x32).
- Neither battle script gives a card, item or reward (no 0xAD 5 / 7 / 0x13 / 0x15), and neither
  writes a set-0 variable other than the friend bits (0, 0x22). Checked (files).
- (0, 0x22) is the friends-met bitmask that Re:CoM's Friend-card drop roll `0x1401602A0` reads
  (bit 1 Donald, 2 Goofy, 4..0x40 others). Checked (decomp + exe scan).
- **No "already done" branch:** event 2000 and event 2002 always end in `0xEE`, with no test before
  it. The battle scripts always run their lesson. The chain moves on only by the battle script's own
  `0xF7`. Checked (files).
- Which exit a normal play takes (lesson or won): Assumption, the lesson exit. Whether winning
  either battle by KO gives EXP: Assumption, no (not read).
- Riku's story has its own tutorial battle 1100 in world 0, started by `WORLD00` event 2102. It is
  not touched (Riku's story is out of scope).
- Kept as they are (not battle tutorials): Traverse Town's field help screens (`CTtr11` field
  actions in room 1, `CTtr12` doors, `CTtr13` save point, `CTtr14` Moogle shop). Also kept: every
  cutscene, with Re:CoM's own scene skip.

## 3. What a played tutorial leaves (for saves)

- Set 0 (saved): friend bits (0, 0x22) end the same as after event 2000 (1 ∣ 2 set) on the lesson
  exits. Nothing else. Checked (files).
- Set 1 (session): all cleared on exit. The only exception is (1, 0x32) = 1 after Leon (music), which
  is cleared by the next room's load. Assumption: cleared at the next room start (its reader
  `0x1401AF9D0`, not traced).
- The deck is untouched: the tutorial deck is a fixed table (`0x1407C2450`, used while (1, 0x2C) is
  set), and since Stage 29 the revolver is empty at KH1 anyway. Checked (decomp).
- So a save made after a skip that ends in the same room with (0, 0x22) ∣= 1 ∣ 2 is the same as a
  save after playing the tutorials. Switching Combat back to Re:CoM later is safe: the floor's cards
  come from event 2003, which still plays.

## 4. Fallback options (owner question)

Each acts only at Combat = KH1, in Sora's story, in world 1, for battle rooms 1100 and 1101.
Re:CoM's setting and Riku's story are unchanged.

- **(a) Skip the battle room entirely.** A wrapper on Re:CoM's battle-entry request `0x140236690`
  (first bytes `48 89 5C 24 08 57 48 83 EC 20 0F B7 F9`; called with a room by script 0xEE; the
  other callers pass 0 or 1030 (`0x1401CA750`, a gimmick); Checked (decomp)). For room 1100 it
  calls instead Re:CoM's own `0x1402360F0(0, 0, 2001)` and sets the friend bits as the lesson exit
  does. For 1101 it calls `0x1402360F0(1, 0x140233AE0(), 2003)` with
  (1, 0x32) = 1.
  - The arena never loads: the opening scene fades straight into the next scene.
  - Runs once per battle request, nothing per frame.
  - Risk: event-to-event moves by `0xF7` aren't used anywhere in vanilla (only battle → event and
    event → field). `0x1402360F0`'s own entry test allows it from an event room (Checked (decomp)),
    but it isn't seen in game. If it misbehaves, (b) is the fallback.
- **(b) Let the battle room load and take its own exit at once.** After Re:CoM's battle manager
  constructor (`0x1401859B0`, already wrapped by M6), in rooms 1100 / 1101 Re:CoM's own "battle won"
  `0x140182240` sets (1, 2). The battle script's won exit then runs its own `0xF7`.
  - For 1100 we also set the friend bits that the won exit doesn't restore.
  - Only vanilla transitions are used.
  - Cost: a battle load, a fade and maybe a frame of the first tutorial line.
- **(c) Patch the two event scripts' bytecode at load** (`0x14014B8A0`): replace the `0xEE`
  sequence with the exit's `0xF7` sequence. It acts like (a), but as data. Each patch would be
  checked against the expected instructions (an unknown file version stays unpatched). More
  fragile (instruction offsets, jump targets). Not recommended.

Recommendation: (a). It is cleanest for the player and native (Re:CoM's own room change, with the
battle script's own arguments). Fall back to (b) if the in-game test shows a problem.

## 5. To test once built

A new game at Combat = KH1:
1. After the opening scene, the hooded-figure battle does not start, and the next scene plays.
2. After the Leon scene, his battle does not start, and the next scene plays and gives the cards.
3. Donald and Goofy are back afterwards: their Friend cards drop again at Combat = Re:CoM.
4. Save, then load at Combat = Re:CoM: the game plays on.

Also check at Combat = Re:CoM that both tutorials play as before.
