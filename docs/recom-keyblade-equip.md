# Equipping Keyblades: the camp menu's 8th entry (decision 25)

> Since M7 stage 7.13 the entry is **"KH1"** and opens a submenu: see
> [Stage 7.13](#stage-713-the-kh1-entry-its-submenu-and-customize). Since the owner's ask of
> 2026-11-01 the submenu is **Items / Customize** and the Keyblade list below opens from the
> **Keyblade slot** at the top of Sora's list on the Items page: see
> [The Keyblade slot in the Items menu](#the-keyblade-slot-in-the-items-menu-owner-2026-11-01) at
> the end. The list itself (rows, mark, stats, scroll bar, equip) is unchanged.

At `combat = kh1`, in Sora's story, Re:CoM's camp (pause) menu gets an 8th top-level entry,
"Keyblades", below Config. Its page lists the Keyblades Sora has earned, marks the equipped one,
shows the highlighted one's Strength, Magic and reach, and equips on confirm. Re:CoM's own camp
code draws and runs all of it. At `combat = recom`, or in Riku's story, there is no entry and the
camp behaves as unmodded.

Code: `crates/kh1-recom/src/keyblades.rs` (rules, tested), `src/win/keyblades.rs` (wrappers and
page), `src/win/menu.rs` (`cfg_input` dispatch), `src/win/stats.rs` (the equipped Keyblade).
Research: the notes `recom-camp-menu.md` and `recom-cards-save.md` (outside the repo).

Steam addresses (image base 0x140000000). Tags: **Checked (decomp)**, **Checked (exe)** = disasm
or bytes read from the Steam exe, **Assumption**.

## The data

| what | where | tag |
|---|---|---|
| Equipped Keyblade | one byte at `*(u64*)0x14087B110 + 0x2674` (= card collection `save + 0x1A74` + 0xC00). 0 = not set (Kingdom Key), else the card kind 1..0x18. Re:CoM saves, checksums, loads, snapshots and zeroes it itself. | Checked (decomp + exe, `0x140158F20`, `0x14012BAE0`, `0x1401B1300`) |
| Its validation | 1..=0x18, not Soul Eater (0x12), with numbers in `kh1-stats.json`; else the Kingdom Key. The Kingdom Key is written as 0, so a save that never leaves it keeps the spare range all zero. | (ours, tested) |
| Readers | the KH1 damage (`kh1_hit`) and the blade test (`win/attack.rs`) read it through `stats::equipped_weapon()` (one pointer read per hit). | — |
| Spare range check | at the first camp open per session the log says whether `save + 0x246B ..= 0x2817` is all zero apart from our bytes (this byte and, since M7 stage 7.1, the MP block `save + 0x2675 ..= 0x2684`, `recom-m7-mp.md`). | — |
| Unlocked | `0x14015B3F0(kind)` or `0x14015B3F0(kind | 0x100)` ("obtained at least once", discovery state 2 or 3), called only when the collection pointer `0x14087A7E8` is set. The Kingdom Key is always listed; Soul Eater never. Only Keyblades with numbers in the stats file are listed (kinds 1..0x11 from KH1's rows, and from step 3 0x13..0x18 from KH2's, `recom-keyblade-models.md` § 12). | Checked (decomp + exe) |
| Names | `0x140164CA0(&kind)`: message `[0x140528560 + kind × 4]` through `0x140240160(*(u64*)0x14087CD78, id)`; it reads only byte 0 for kinds below 0xDB. Re:CoM's own localized card names. The stats file's name is used if the message is empty. | Checked (decomp + exe) |
| Riku's story | `0x1401AAAE0() != 0`, and the camp's own copy `camp+0x21BC`. | Checked (exe) |

## The entry: four wrappers

Re:CoM's camp has a hard limit of 7 entries (fixed arrays `camp+0x1B8[7]` buttons and
`+0x1F0[7]` labels, the `cmp [camp+0xB8],6` confirm gate at `0x1400DAABD`, a 7-byte state table
in state 0x11, a 7-entry jump table). Checked (decomp + disasm). Our entry is a third instance of
Re:CoM's entry button, made by the same calls, held outside those arrays; four wrappers make the
camp's own code treat index 7 as one more entry. Installed all or nothing, after the Config hooks
(the page runs through the Config input hook), never when `config_menu = off`.

| function | first bytes (Steam) | what the wrapper does |
|---|---|---|
| camp update `0x1400D9C80` (vtable slot `0x140509018`) | `48 8B C4 55 41 54 41 55 41 56 41 57` | Re:CoM's update always runs. States 7 / 0xD: when button 0's animation had ended, ours gets animation 5 as Re:CoM's loop gives its own (not in 0xD when the cursor is on ours: Re:CoM's 0xD then re-highlights it with animation 3). State 0xF: a held "down" on entry 6 (Re:CoM stops at its last entry) moves to ours with the cursor set, sound 1 and `0x1400DE100`; confirm on ours runs Re:CoM's confirm sequence (below); Re:CoM's confirm of one of its own entries gives ours animation 6. State 0x11 with the cursor on ours: Re:CoM's table would give 0x2E (a deck-editor state, from the stack's 8th byte); after the original we write 0x4A, so our page opens in Re:CoM's Config states. |
| entry build `0x1400E3DE0(camp, mode, level)` | `48 8B C4 55 56 57 41 54 41 55 41 56` | After the original, only at level 0 (`camp+0xB4`). Mode 0: create ours when wanted and absent, delete it when not wanted, else animation 4. Mode 1 / 2: animation 5 / 6. Created as `0x1400E3DE0` creates Re:CoM's: `C2DSeqCtrl` (`0x140247260`), `+0x16E = 0xC`, `+0x138 = 1`, vtable `+8` sequence 4, position `0x140248660` at (86.0, 88.0 + 26.0 × 7 = 270.0), a `C2DFnt` (`0x140243F10`, colour 0x80404040, font 0xF, 0x40) with "Keyblades", attached with vtable `+0x60` at (0.0, 6.0). Checked (disasm `0x1400E3E80..0x1400E3F8A`). "Wanted" = Combat = KH1, Sora's story, not the direct-map camp (`+0x20A4`), Keyblade numbers loaded. Switching Combat in Config adds / removes it when Re:CoM rebuilds the list on leaving Config (state 0x4C). |
| cursor set `0x1400DEA10(camp, index, show)` | `48 8B C4 48 89 58 18 48 89 70 20 55 57 41 56` | With ours, at level 0: wraps over 8 (−1 → 7, 8 → 0). Moves between Re:CoM's entries go to the original. A move from or to ours does the original's work with Re:CoM's calls: old entry animation 5, label 0x80404040 (`fnt+0x34`), priority `0x140247AA0(btn, 0xC)`, help cleared with `*(char**)0x1407C12E8`; new entry animation 3, label 0x80C0C0C0, priority 0xD, highlight bar `0x1400E07D0`, glove `0x1400E3D30`, `camp+0xB8`, help: ours, or message `0x500400 + index` for Re:CoM's (its table for Sora's top level). |
| teardown `0x1400D7C20` (both dtors, state 0x50) | `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18` | Deletes our page's objects and our entry, then the original. |
| Config input `0x1400D3EB0` (hooked since 0.4.0) | `48 89 5C 24 08 57 48 83 EC 20` | Before anything else: when our entry exists for this camp and `camp+0xB8 == 7`, our page runs in its place (same contract: ≥ 0 running, < 0 closed). |

### The confirm sequence on ours

Re:CoM's state 0xF for an entry with a page (disasm `0x1400DAB0A..0x1400DACAC`), run after the
original's frame (which, for index 7, only eased the glove with `0x1400DE100`; nothing below
changes its target):

1. `0x1402B57A0(0, 0)`; 2. `0x1400D3A70(camp)`; 3. sound `0x1402AA360(2, −1)`;
4. the cursor set's work for (7, show 0);
5. every Re:CoM button and ours: vtable `+0x20(btn, 0, set, 6, 0.0, 0)`;
6. header item 0: `0x1400E4220` mode 1's work (header present) with our title where Re:CoM puts
   `0x1400D3890`'s ("" for index 7): animation 7, old title `C2DFnt` deleted, a new one made with
   "Keyblades" (colour `camp+0x98`, font 0xF, 0x40), vtable `+0x68(hdr, 1, set, fnt)`,
   `0x140247DC0(hdr, 1)`, `camp+0xC4 = 2`; then `0x140247DC0(camp+0x228, 1)`;
7. `camp+0x140` sequence 0xC; 8. `0x1400E6700(camp, 2)`; 9. `0x140247E50(camp+0x130, 2, 0)`;
10. state 0x11; 11. `camp+0xBC = 7`, `0x1400E46D0(camp, 4)`.

Checked (decomp + disasm). The only difference from Re:CoM's order: the update's per-frame trailer
(`0x1400E4220` ×2, `0x1400DEC60` ×2, `0x1400E8760(9)`) ran before these steps this frame instead
of after. Its header call is mode 999 (idle) before and mode 2 after; mode 2 first waits for the
header's new animation 7 to end, so it does nothing on this frame either way. Checked (decomp).

## The page

Runs inside Re:CoM's Config states: 0x4A (`camp+0xB4 = 2`, first call), 0x4B (each frame until
it returns < 0, then Re:CoM closes the header), 0x4C (Re:CoM rebuilds the list and returns to
state 0xD, which re-highlights our entry through the cursor set). Its state is ours (`PAGE`); it
writes no camp field Re:CoM's pages use. It follows the entry-4 page (`0x1400EB7E0` / draw
`0x1400E6DF0`), with Re:CoM's classes and calls. Checked (decomp + disasm):

| part | made as Re:CoM makes it |
|---|---|
| status panel | `C2DSeqCtrl`, `+0x16E = 0x10`, `+0x138 = 1`, layout group 0x11 element 3, sequences 0x125 open / 0x126 idle / 0x127 close |
| list panel | `C2DSeqCtrl`, kind 0x11, layout group 0x11 element 2, sequences 0x14C / 0x14D / 0x14E |
| cursor | `CButtonCursor` (0x1C0 bytes, ctor `0x1400D25F0(obj, 0x12, list panel, 0x12D)`), shown when the opening ends, glides by `0x1400DE010(cursor, dt × 0.02)`, hidden on close by `0x1400E5CF0` |
| scroll bar | `CScrollBar` (0x28, ctor `0x140112AB0`, setup `0x1401130C0(bar, set, 0x14, seqs 0x1407C1228, (133, −56), 157.0, list panel)`, update `0x140112CB0(bar, min(1, top/n), min(1, 8/n))`) |
| list rows | 8 visible `CButton`s (`0x140112A90`, setup `0x140112F10`, kind 0x11, seqs `0x1407C11F0`, layout group 0x12 element 2+i, font 0xF (0xE for languages 3..5 or `camp+0x9C`), offsets 1.0 / 2.0, colour `camp+0x98`, variant 0), rebuilt on each move (draw mode 9). Top row: the cursor stays in the 8 visible rows. |
| equipped mark | the row sprite's marker, vtable `+0x40(sprite, set, 0x1D, 170.0, −13.0, 0.0, 0)` then the new object's `+0x30(obj, camp+0x21D8)`, as Re:CoM marks its rows ("seen" 0x1D / "new" 0x1E). **Assumption**: 0x1D reads as a mark here. |
| status rows | 3 `CButton`s, kind 0x10, seqs `0x1407C11D0` (variant 0..2 = sequence 0x12C), layout group 0x11 elements 4..6, y + 8.0, font 0xE; a value `C2DFnt` (colour 0x80046060, font 0xE, 0x41), text copied with `0x140258DE0`, attached at (220.0, 1.0). Labels "Strength", "Magic", "Reach". Rebuilt for the highlighted Keyblade on each move (Re:CoM builds its status rows once). |
| help line | ours while the page is open; on close the message `*(u32*)0x1407C106C`, as the entry-4 page restores. |

Input (sub-state 2), in `0x1400EB7E0`'s order: cancel `0x140111B90` closes (both panels' close
sequences, cursor hidden, sound 4) and the page returns −1 when the status panel's close ends,
deleting its objects in Re:CoM's order (draw mode 6); confirm `0x140111C30` equips the
highlighted Keyblade (save byte, sound 2, rows redrawn so the mark moves); up / down through
`0x140111A70` (repeat; the fresh flag it leaves in `camp+0x94` decides the wrap at the ends);
page up / down on pad bits 4 / 8 (`0x14029B2E0` repeat, `0x14029B2F0` fresh), with Re:CoM's
±8 and top-row rules. Any move plays sound 1.

Values: Strength and Magic are the KH1 weapon row's bonuses ("+3"; for the six KH2 Keyblades
KH2's stat row's); reach is the blade span `|far − near| × fraction scale` over the Kingdom Key's
("x1.00"), a display figure only.

Differences from the entry-4 page, by design: no tab bar (`camp+0xC08`, sequences 0x14F..) and no
`0x1400DEC60(camp, 1, 0)` side element (the Config page, whose states we run in, has neither); up
past the first row wraps to the last (Re:CoM's goes to its tab bar).

## Per frame

The camp exists only while the pause menu is open: nothing of this runs in field or battle
frames. With no entry (Re:CoM setting, Riku's story) each wrapper is one load and the original.
With the entry, the update reads the pad (pure reads) in state 0xF and a few fields; the page's
code runs only while it is open, like Re:CoM's pages.

## Assumptions and risks

1. **Room on screen** at y = 270 (highlight bar 262, glove 264): the owner's 720p screenshot
   shows the 8th entry would sit at screen y ≈ 485–530, with about 28 px above the bottom HUD and
   clear of Sora's status box. Not seen in game yet.
2. **Text items own nothing we delete**: Re:CoM's teardown deletes its buttons, never their
   labels; the replace path `0x140246E90` clears the old sequence object's item pointer before
   releasing it, so the items go with their sequences. We delete only buttons, panels, the cursor
   and the scroll bar, as Re:CoM does. Inferred from the decomp, not traced: watch for leaks or a
   crash on closing the page or the camp.
3. **The equipped mark** (0x1D) and the **status rows' look** (sequence 0x12C, rebuilt on each
   move): to judge in game.
4. **ASCII texts** ("Keyblades", help lines, "Strength", "+3", "x1.00") render as on the Config
   row; `+` and `x` are not seen yet. Card names are Re:CoM's localized messages.
5. **The spare range** is unused by vanilla code (decomp + exe scans); a pointer from elsewhere
   can't be ruled out statically. The log reports it at the first camp open each session.
6. **Retry snapshots** (`0x1401B03C0` / `0x1401B1410`) copy the whole save block: a Continue after
   a game over can restore the Keyblade equipped before the checkpoint, like Re:CoM's own deck
   edits. Checked (decomp); when snapshots happen is an Assumption.

## What the first in-game test must check

1. The 8th entry at y ≈ 270: clear of the bottom HUD, label "Keyblades", help "Equip a
   Keyblade.", highlight bar and glove on it.
2. Cursor rules: held down from Settings stops on Keyblades; held down there stops; a fresh
   press wraps to Review Decks; a fresh up on Review Decks wraps to Keyblades.
3. Confirm: sound, buttons fade, header title "Keyblades", the page opens (not the deck editor).
4. The page: both panels, rows with the card names, the mark on the equipped one, the cursor,
   the scroll bar (17 rows > 8), the status rows' values, the help line. Confirm moves the mark;
   cancel closes back to the list with Keyblades highlighted.
5. The log: "keyblades: camp wrappers in", the spare-range line, "page opened", "equipped".
6. Damage changes with the equipped Keyblade's Strength (KH1 setting, a twin enemy).
7. Combat = Re:CoM in Config: leaving Config removes the entry; back to KH1 adds it.
8. Riku's story: no entry.
9. Close the camp with Start while on the entry, and quit to title with the page open: no crash.

## In game, 0.8.0 (owner, 2026-10-31)

- Owner: "it all looks good". The entry, the page, equipping, removing the entry at the Re:CoM
  setting and putting it back all work. Checked (owner + log).
- Log on the owner's late-game save (Sora at level 99): `save+0x246b..=0x2817 is all zero apart
  from our byte`. The spare range was unused in a real save. Checked (log).
- The log also shows 17 Keyblades listed (cards 1..0x11, all KH1's, all ever obtained) and
  equips from 0x1 up to 0x11 saved in the byte. Checked (log).

## Stage 7.13: the "KH1" entry, its submenu and Customize

Decision 34: "Customize: one "KH1" camp entry (our entry renamed) with a submenu: Keyblades,
Customize (later Abilities, Items)". Plan row 7.13 (`recom-m7-plan.md`). Code:
`crates/kh1-recom/src/keyblades.rs` (`text`, `sub`), `src/customize.rs` (KH1's Customize rules,
tested), `src/win/keyblades.rs` (`Sub`, `Page`, `Content`), `src/win/mp.rs` (`set_shortcuts`),
`src/win/command_menu.rs` (`spell_view`, `word`, `confirm_is_circle`).

### The entry

Label "KH1", header title "KH1", help "Keyblades and shortcuts." Same wrappers, gate (Combat = KH1,
Sora's story, not the direct-map camp, Keyblade numbers loaded) and place as before. One change to
the confirm sequence: **step 9 (`0x140247E50(camp+0x130, 2, 0)`, the main panel's layer 2
hidden) is no longer run at the top confirm**: Re:CoM runs it only for its page entries (2, 4, 5,
6), not for its Deck entry, whose submenu keeps the layer. It now runs when a page opens from our
submenu, and the layer comes back when the page closes (as Re:CoM's state 0x4C does). Checked
(decomp `0x1400D9C80` state 0xF and 0x4C).

The Config input hook now runs our code when `camp+0xBC == 7` (the top index Re:CoM's state 0xF
saves on every confirm; ours sets 7) instead of `camp+0xB8 == 7`: inside the submenu `camp+0xB8`
is the submenu's row, as in Re:CoM's Deck submenu. Checked (decomp: among the camp's functions
`+0xBC` is written only by state 0xF's confirm path and read only by state 0x23).

### The submenu: Re:CoM's Deck submenu, our rows

Re:CoM's Deck entry (top index 0) opens a level-1 submenu made of **the same 7 entry buttons**,
relabelled. Checked (decomp `0x1400D9C80` states 0x10, 0x1D, 0x22, 0x23, 0x34, 0x35; disasm of
the relabel `0x1400E3FF3..0x1400E40C4`). Ours does the same with Re:CoM's calls, inside the Config
states 0x4A..0x4C our page already uses (Re:CoM's Deck states are tied to its deck panels):

| step | Re:CoM's Deck | ours |
|---|---|---|
| open (top buttons faded) | state 0x10: `0x1400E3DE0(camp, 0, 1)`: count 6, each label deleted, a new `C2DFnt` (0x80404040, font 0xF, 0x40) in `camp+0x1F0[i]`, button vtable `+0x68(btn, 1, set, fnt)`, `camp+0xB4 = 1`, animation 4 | the same relabel with our row labels, count = rows (2), `camp+0xB4 = 1`, `camp+0xB8 = 0`, then Re:CoM's `0x1400E3DE0(camp, 0, 1)` (same level: animation 4 only) |
| idle | state 0x1D: when button 0's animation ends, mode 1 (animation 5), cursor set `(0, 1)` | the same (cursor set's original: at level 1 our wrapper passes it through), then our help line |
| input | state 0x22: cancel `0x14029B410`, confirm `0x14029B490`, up / down repeat (0x101000 / 0x204000), held stops at the ends, fresh wraps (cursor set wraps over the count); sound 1; `0x1400DE100` every frame | the same (`keyblades::sub::step`, tested) |
| confirm a row | sound 2, `0x1400E4220(camp, 1, 1)` + `(7, 1)` (header item 1, the breadcrumb, with the row's title), mode 2 (animation 6), highlight bar / glove hidden (`0x1400E07D0(−1)`, `0x1400E3D30(−1)`) | the same, header item 1 titled with the row's label (as item 0 gets "KH1"), plus step 9 (main panel layer 2 hidden); when button 0's fade ends the page opens |
| page done | states 0x34 / 0x35: `0x1400E4220(camp, 4, 1)`, `0x1400E3DE0(camp, 0, 1)` (animation 4), then idle with the cursor on **the same row** | the same, plus layer 2 shown |
| cancel | state 0x22: cursor set `(row, 0)`, mode 2, `0x1400E4220(4)`, sound 4; state 0x23: when button 0's fade ends, back to the top list (`0x1400E3DE0(0, 0)` relabels the 7 entries) | cursor set `(row, 0)`, mode 2, sound 4; when button 0's fade ends `camp+0xB8 = 7` and the page returns −1: Re:CoM's 0x4B closes header item 0 (`0x1400E4220(camp, 4, 0)`, disasm `0x1400DC4B0`), 0x4C relabels its 7 entries (level 1 → 0) and 0xD highlights ours |

M8 stage 8a.2 added a third row, **Items** (KH1's Items screen, `docs/recom-m8-items.md` § Stage
8a.2); the entry's help is now "Keyblades, shortcuts and items." Since 2026-11-01 the rows are
**Items, Customize** (the Keyblades row folded into Items) and the help "Sora's Keyblade and
items, and the spell shortcuts.": see the last section.

Differences from the Deck submenu: header item 0 closes when the submenu's fade ends, not at the
cancel press (the Config states close it on the page's −1; calling it twice would restart its
animation); Re:CoM's Deck-only panels (`0x1400E0870`, `0x1400E13C0`) and side elements
(`0x1400DEC60`) are not used. Rows: `keyblades::sub::ROWS`; a row (M9's Abilities) is one more
`Row` with a label, help and page (up to Re:CoM's 7 buttons).

No new hook and no new callee: every call above was already a checked site of the Keyblades entry
(`sites::KEYBLADE_HOOKS` / `KEYBLADE_CALLEES`); the entry build and cursor set are called through
our wrappers' trampolines (Re:CoM's originals). All 354 site byte lists re-read from the Steam exe
(pefile): 0 mismatches.

### Customize (KH1's shortcut page)

KH1 (Steam, Checked (decomp + exe)):

| what | where |
|---|---|
| slot list: 3 rows, labels sysmsg `0xCE / 0xCF / 0xD0` (table `0x140548060`, read from the exe), each with its slot's spell name or sysmsg `0xD4` ("none") | `0x140302950` |
| a slot's spell, its name at the spell's current level (command `level + (spell + 2) × 3`) | get `0x140285090` (`save + 0x844 + slot`), name `0x14028F290` |
| picker: the learned spells in KH1's order, then "none" (the last row), at most 8 | fill `0x1403041A0`, list `0x142E9A018`, count `0x142E9A00C` |
| the picker opens on its last cursor (one for every slot) | `0x142E9A040` (window cursor `+0xFE` set from it at `0x140302950`) |
| confirm writes the slot, **no duplicate check**, back to the slot list; cancel back without writing | `0x140302740` → `0x1402857C0(slot, spell or −1)` |

Ours, on Re:CoM's camp list (the Keyblades page's pieces: both panels, `CButton` rows, the
`CButtonCursor`, the label / value rows, the help line; no scroll bar, at most 8 rows):

- **Slot list**: list rows = the button each slot casts with (Triangle, Square, confirm), as
  Re:CoM's own button glyphs (below; until the castfix build: "Shortcut 1 / 2 / 3"); the status
  panel shows the three slots (the same glyph, spell name or "None"), rebuilt after every write.
  Help: "Hold [L1] and press [button] to cast this spell." (7.12's buttons, glyphs too).
- **Picker** (confirm on a slot, sound 2): list rows = the learned spells at their level, then
  "None"; Re:CoM's row mark (0x1D, as the equipped Keyblade's) on the row holding the slot's
  spell; opens on KH1's last picker row (kept for the session). Confirm (sound 2) writes the slot
  in our save block (`save + 0x2681..0x2683`, 0 = none, else spell + 1) and returns to the slot
  list on that slot; cancel (sound 4) returns without writing; cancel on the slot list closes the
  page.
- Spells and levels: the command menu's (`spell_state`, and `kh1_all_spells` as the menu applies
  it). Names: KH1's words from `kh1-command-menu.json` in `menu_language` (the command menu's
  source), else the English names of `crate::unlocks::level_name`.
- The command menu reads the slots every frame (`spell_state`), so the L1 list shows the change at
  once. A save never synced gets its one-time sync before our write (`set_shortcuts` reads first).
- Sounds: Re:CoM's camp sounds (1 move, 2 confirm, 4 back, 5 when the save block isn't there).

1:1 with KH1: the picker's contents and order, "None" last, names at the current level, the write
without a duplicate check, the return to the slot list, the picker's opening row. Checked
(decomp). Not KH1's: the screen is Re:CoM's camp (by design); the labels "Shortcut 1..3" and
"None" are our English words (**Assumption**: KH1's sysmsg 0xCE..0xD0 / 0xD4 texts not read; its
sysmsg file isn't in the asset step); the rows show the buttons as Re:CoM's glyphs (below), not
KH1's words; KH1's per-spell help lines (sysmsg `0xD4 + 3·spell + level`, `0xEA`) are not shown;
the slot list opens on slot 1 each time (**Assumption**: KH1's slot-list window cursor start not
traced).

### The slot rows show their buttons (owner, 2026-11-01)

> "for the customize menu, instead of saying 'shortcut 1, shortcut 2 etc' it should be the
> corresponding button that it would be mapped to."

**Re:CoM's font has button escapes, and they follow the active controller and the keyboard.**
Checked (decomp + exe):

| what | where |
|---|---|
| A `C2DFnt` text is copied for drawing by `0x1403480C0`: a byte ≥ 0x81 starts a two-byte code; `0x140345B40` replaces codes `0xF564..0xF584` with the glyph code in a table at `0x14083F510..` (escapes 0xF574..0xF576 give 2 or 4 glyphs with the keyboard). | `0x1403480C0`, `0x140345B40` (jump table `0x140345F18`) |
| The table is refilled for the active device: a pad (`0x140349120`: the pad type at the controller config `+0x1C` (1, 2, 3: three icon sets), confirm / cancel swapped by `+0x1E`), or the keyboard (`0x1403488E0`: each entry `0x140348900(bound key)`). | `0x140349120`, `0x1403488E0` |
| Each entry is an action's button: the key bindings (`0x14034B7D0`, `0x14034BC90`) tie key `0x14083F43C` to the confirm pad bit `[0x14083F3F0]` (default 0x4000, Cross), `0x14083F464` to `[0x14083F408]` = 0x8000 (Square), `0x14083F44C` to `[0x14083F40C]` = 0x1000 (Triangle), `0x14083F46C` to `[0x14083F424]` = 0x400 (L1); the glyph entries `0x14083F510` / `F518` / `F51A` / `F51C` are filled from those same keys. | exe data `0x14083F3F0..0x14083F434` (PS2 pad order) |
| So: `F5 64` = the confirm button (Cross, or Circle when Re:CoM's confirm is Circle), `F5 66` = Square, `F5 67` = Triangle, `F5 68` = L1, each as the current controller's icon or the bound key. | the cases `0x140345B67`, `0x140345C43`, `0x140345C59`, `0x140345B93` |
| Glyph codes 0x55C..0x610 are drawn from the font's button sheet only when the font has one (`+0x150`, `0x14025A0B4`); the font set-up gives it to the font named "sysfont" (`0x14025C400`). A `C2DFnt`'s font is its flags `+0x40` bit 12 (`0x14025AACE`: table `0x14087CFC0`, 0x158 a font, `0x14087CFB8` fonts). | `0x14025A0B4`, `0x14025C400`, `0x14025AACE` |

**Built:** the slot rows and the status panel's labels are the escapes (slot 1 `F5 67`, slot 2
`F5 66`, slot 3 `F5 64`), and the help lines "Hold `F5 68` and press `F5 67` to cast this
spell." / "Choose the spell for `F5 68` + `F5 67`." Re:CoM draws the icon of the button the slot
casts with, on the controller in use (or the bound key with the keyboard), and slot 3 follows
Re:CoM's confirm setting by itself. The rows use font 0 (`CButton` set-up flags 0x40, bit 12
clear); the help line its own `C2DFnt`'s font. When that font has no button sheet (`+0x150` ≠ 1),
the page uses the buttons' names instead: "Triangle", "Square", "Cross" / "Circle" (from
`command_menu::confirm_is_circle`), "Hold L1 and press …". The log says which, once:
`customize: slot labels as Re:CoM's button glyphs (Re:CoM's font 0 has a button sheet)`.

- Checked (decomp + exe): the escapes, their table, the actions behind them, the font flag; 9 new
  checked sites (`sites::CUSTOMIZE_GLYPHS_CHECKED`), tests (`customize::tests`,
  `sites::tests::customize_glyph_sites`).
- **Assumptions:** (1) font 0 is the "sysfont" (Re:CoM's own help and tutorial texts are drawn in
  it; the page checks the flag at run time and falls back to words); (2) the pad's confirm swap
  `+0x1E` and the confirm pad bit `0x14083F3F0` are set together by Re:CoM's options, so `F5 64`
  shows the button slot 3 casts with (the command menu reads that bit); (3) with the keyboard,
  Re:CoM's key glyphs (`0x140348900`) are drawn from the same sheet; (4) the glyph's width in a
  list row: one icon where "Shortcut 1" stood.
- Not KH1's: KH1's own slot labels (sysmsg 0xCE..0xD0) are not read; whether KH1 shows the L1
  with the button in those rows: not checked (KH1's sysmsg file isn't in the session).

### Per frame

Nothing outside the pause menu. In it, the submenu reads the pad and one sequence state a frame
(as Re:CoM's state 0x22 does); the pages as before.

### Logs

`kh1 entry: camp entry 8 "KH1" added`, `kh1 entry: submenu opened (2 rows …)`, `kh1 entry: submenu
closed`, `keyblades: page opened …`, `customize: page opened: spell levels […], shortcut slots […]`,
`customize: shortcut k = <spell> (was <spell>); slots […]`.

### Assumptions and risks

1. **Relabelling Re:CoM's buttons from the Config states**: Re:CoM does exactly this for the Deck
   (same calls, same objects); state 0x4C's relabel back runs because `camp+0xB4` is 1 there.
   Not seen in game.
2. **Status values' width**: spell names at x 220 of the status panel ("Blizzaga", "Gravité"):
   to judge in game.
3. **Header item 1** (the breadcrumb) titled "Keyblades" / "Customize" through the same
   replace path as item 0's "KH1".
4. The row mark (0x1D) on the picker's current spell: as on the Keyblades page (seen there).

### What the owner should test in game

1. Camp at Combat = KH1 (Sora's story): the 8th entry reads "KH1", help "Keyblades and shortcuts."
2. Confirm: the entries fade, header "KH1", two buttons "Keyblades" / "Customize" appear where
   Re:CoM's entries were (as the Deck submenu), cursor bar and glove on Keyblades, help per row;
   up / down move and wrap (held stops at the ends); sounds 1 / 2 / 4.
3. Keyblades: header shows "KH1 > Keyblades"; the page behaves as before (equip, mark, stats,
   scroll); cancel goes back to the submenu on Keyblades.
4. Customize: the rows show the Triangle, Square and Cross (or Circle) icons of the controller in
   use (with the keyboard: the bound keys); the help line shows the L1 icon and the row's button.
   Switch controller type or Re:CoM's confirm button and reopen the page: the icons follow. The
   three slots and their spells (status panel); confirm a slot → the picker (learned
   spells at their level + None, the mark on the current one); pick one → back to the slot list,
   the status panel shows it; the same spell in two slots is allowed; cancel in the picker writes
   nothing.
5. Close the camp; hold L1: the shortcut list shows the new slots and casts them.
6. Cancel from the submenu: back to the top list with "KH1" highlighted and Re:CoM's 7 labels
   right; enter Re:CoM's Deck and Config afterwards: both work as unmodded.
7. Save, reload: the slots stay (per save). Combat = Re:CoM, Riku's story: no entry.

## The Keyblade slot in the Items menu (owner, 2026-11-01)

Ask (owner, 2026-11-01, verbatim): "I would like one menu inside of the "KH1" menu for "Items" in
"items," i want a slot for Keyblade, and all the item slots for items. eventually if we add
accessories as well we will have slots for them there but dont worry about that for now".

- The submenu's rows are **Items, Customize** (`keyblades::sub::ROWS`); there is no Keyblades row.
  Re:CoM's Deck-submenu machinery is unchanged (the rows are data; the count is 2).
- Items → Sora: the first row is the **Keyblade slot** (the equipped Keyblade's card name; status
  panel Strength / Magic / Reach), then Sora's item slots. Its kinds come from
  `items_page::SORA_KINDS` (Keyblade, Item); accessory slots will be one more kind between them.
- Confirm on the Keyblade slot (sound 2) shows **the Keyblade list on the same page**
  (`win::keyblades::Page::open_keyblades`): the same content, rows, mark, status rows, help and
  confirm (equip, stay in the list) as the Keyblades page above; its `CScrollBar` is made then, as
  the page made it at open. Cancel (sound 4, `back_from_keyblades`) deletes the scroll bar and goes
  back to Sora's list on the Keyblade slot, which now names the new Keyblade. The page's panels,
  cursor and help line stay up throughout (one page, two lists).
- No new hook, callee or data site; the save byte, its validation and every reader are unchanged.
  All 425 site byte sets and 13 pointers re-read from the Steam exe (pefile): 0 mismatches. Checked
  (exe).
- **Assumption**: the scroll bar made and deleted while the page stays open looks right (Re:CoM
  makes it with its page). Not seen in game. Details, tests and the owner's test list:
  `docs/recom-m8-items.md` § The Items menu with Sora's equipment.

## The Abilities row (M9 stage 9b) and the rows' look

- The submenu's rows are **Items, Customize, Abilities** (`keyblades::sub::ROWS`); the entry's help
  "Sora's Keyblade and items, spell shortcuts and abilities." The Abilities page runs on the same
  page type (`win::keyblades::Content::Abilities`): KH1's ability screen with AP, rules in
  `crate::abilities::page`, notes in `docs/recom-m9-abilities.md` § Stage 9b. Re:CoM's row mark
  (0x1D) now marks **every** equipped row there (`Page::marked(idx)`), as one row before.
- **A row's look is Re:CoM's**: the last argument of its `CButton` set-up `0x140112F10` is the row's
  variant, which picks the sequence `seqs[variant]` from the list's table `0x1407C11F0` (Checked
  (decomp)): Re:CoM's Sleights page passes each sleight's category there (`0x1400E6DF0` mode 9,
  from the list entry's second byte, filled in mode 10). The pages used variant 0 everywhere; they
  now pass `keyblades::row_look` per row (`Page::row_look`): the Items page's Stock row and item
  slots green (owner, 2026-11-01; `docs/recom-m8-items.md` § Row looks), everything else red as
  before. No new site (the set-up was already a checked callee).
- `camp.lad` (`FO0100.SPR`, Checked (files)): sequence 0x12E red plate (vertex colour 0x8000003C)
  + icon sprite 167; 0x133 blue plate (0x803C1E00) + icon 168; **0x131 green plate (0x80143200) +
  icon 278 tinted green (0x801EDC1E)**; 0x132 grey + icon 279; 0x12F / 0x160 / 0x130 Riku's.
