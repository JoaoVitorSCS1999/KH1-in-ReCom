# Re:CoM's Config menu, and our rows in it

Steam addresses. Tags: Checked (decomp / disasm / exe) means read; Assumption means inferred.
Built in `crates/kh1-recom` (`src/menu.rs`, `src/win/menu.rs`). Since 2026-11-03 our rows are no
longer in Re:CoM's Config screen: they are the KH1 camp entry's settings pages, drawn with this
screen's code (§ The KH1 entry's settings pages).

> **Since 0.104.0 (owner, 2026-11-04)** the "KH1" entry is gone. In Sora's story (both Combat
> settings) Re:CoM's **Settings** entry opens a short list: **Game Settings** (this Config screen,
> Re:CoM's own input `0x1400D3EB0` called unchanged through `win::menu::recom_config`), **Combat**
> and **Movement** (the pages below). In Riku's story Settings opens this screen directly, as
> unmodded. `docs/recom-keyblade-equip.md` § The KH1 camp list.

## Where it is

- Re:CoM has two option menus:
  - the PC port's launcher settings (`SettingMenu` namespace: display, sound, pad, keyboard;
    `config28.dat` / `config1525.dat`). Not the game's; not touched. Checked (RTTI, strings).
  - the game's own camp menu (`CCamp`, vtable `0x140509010`, constructor `0x1400D2730`, layout
    `camp.lad`, sprite set `camp:00`, update `0x1400D9C80`). Checked (RTTI + decomp).
- Camp menu items 0..6 go to states `1D 16 41 1A 44 47 4A` (`0x1400D9C80`, case `0x11`).
  Item 6 → states `0x4A..0x4C` → the **Config screen** `0x1400D3EB0`. Checked (decomp).

## The Config screen

- **Input / state** `0x1400D3EB0(camp)`, state at `camp+0xA8` (0 open, 1 opening, 2 active,
  3 closing). Checked (decomp + disasm).
  - State 2, each frame: draw mode 3 (animate), then confirm/cancel (`0x140111B90`) closes,
    else up `0x101000` / down `0x204000` move the cursor `camp+0xBE8` over **5 rows (0..4,
    wrapping)**, else for rows 0..3 left `0x408000` / right `0x802000` flip the row's byte
    `camp+0xBED + row` (0/1, wrapping). After any change: sound `0x1402AA360(1, -1)` and draw
    mode 9. Pad reads (`0x140111A70`) are pure reads of the pad state (`0x14029C220/240`).
    Checked.
- **Draw** `0x1400DF1B0(camp, mode)`. Mode 9 rebuilds the rows:
  - 5 labels (message ids `0x500100 / 101 / 103 / 105 / 106`) and 9 values (rows 0..3: two each,
    ids `0x500107..0x500113`; row 4: one, `0x500114 + byte 0xBF1`, display only). Text comes
    from the message table `0x140240160(DAT_14087CD78, id)`; it lives in message files, not in
    the exe. Checked.
  - Each item is a `CButton` (`operator new 0x1404ACE54(0x30)`, ctor `0x140112A90`, set up by
    `0x140112F10`, 13 arguments, see `src/win/menu.rs`), stored in **21 slots at
    `camp+0xB38`**. Re:CoM fills slots 0..13; **14..20 are never written** (only zeroed in the
    constructor and deleted in draw modes 6/9 and the teardown `0x1400D7C20`). Checked (every
    reference to `camp+0xB38..0xBE0`).
  - Positions come from `camp.lad` group `0xB` through `0x140246100(layout, 0xB, element, out,
    0)`: labels (x, y) elements (2,2) (3,3) (5,4) (7,5) (8,6); value x elements
    9,10,11,12,15,16,20,21,22, value y elements 9, B, D, F, 11 per row. Checked (disasm
    `0x1400DF5B7..0x1400DF878`).
  - The selected value of the cursor row gets the cursor sprite `camp+0xBE0` (animation `0xFA`,
    `0x100` for row 4), moved to the item's sprite position + 150.0 in x, and the help line
    `camp+0x9D0` gets help id `0x50041A + item` (`0x140259000`). Checked (disasm
    `0x1400DF931..0x1400DF9EF`).
- **Values and their store.** On opening the camp, bytes `0xBED..0xBF1` are read from
  `0x1401B0800(0, id)` with ids `65 66 63 67 61` (`0x140507C2C`); the camp's destructor writes
  them back with `0x1401B15D0`. That is a byte array in **save data** (set 0 of `0x14087B0E8`),
  shared with story flags: 73 functions read it with many ids, and event scripts may too, so a
  free byte can't be proven unused. Checked (decomp). **Our values therefore stay in
  `kh1-recom.ini`.**
- Row 4 (id `0x61`, 4 values, never changed here) is probably the difficulty. Assumption. The
  meaning of rows 0..3 isn't known without the message files. Not needed.

## The window panel

> History: 0.4.1 .. 0.92.0 stretched this panel for our rows. Since 2026-11-03 nothing of ours
> touches it (the rows left Re:CoM's Config screen); the notes stay as findings.

- `camp+0xB30` is a `D2::C2DSeqCtrl` (ctor `0x140247260`, base vtable `0x140607340`), made in
  draw mode 0 with `vtable+8` (`0x140247850`) playing animation **`0xF7`** (open) from the
  camp's sprite set `camp+0x80`. Mode 1 switches it to **`0xF8`** (idle) once `0xF7` ends
  (`0x1402480C0`), mode 4 to **`0xF9`** (close), both with `vtable+0x20` (`0x1402482F0`,
  slot 0); mode 6 deletes it. Every Config item and the cursor are its children (`CButton`
  setup `0x140112F10` parents the item's sprite with `0x140248650` and gives it the raw layout
  position with `0x1402485B0`; the panel itself sits at (-256, -h/2), `0x140248660`, h =
  `DAT_140821ADC`, 384 in the exe image). So layout units are the panel's local units.
  Checked (decomp + disasm).
- Each animation change makes a new `D2::C2DSeqObj` whose list (`+0x20`; first at list `+8`,
  next at object `+8`) holds `D2::C2DSprObj`s (vtable **`0x1406077D8`**, ctor `0x140249FC0`),
  one per layer, each with `+0x142` parts. The ctrl keeps a handle per sequence at `+0x144`
  (count `+0x16D`); **`0x140246210(handle)`** returns the `C2DSeqObj`. Checked (decomp + exe).
- **Re:CoM has a native 9-slice.** A `C2DSprObj` with exactly 9 parts and flag 8 in `+0x144`
  is drawn by its update `0x14024BB10` (called from its vtable slots `0x14024A3F0` /
  `0x14024A7E0`) through **`0x14024AD40`**, which moves the 3×3 parts' corners by four edge
  offsets: left `+0xA0`, top `+0xA4`, right `+0xA8`, bottom `+0xAC` (top row moves by top,
  bottom row by bottom, the middle row stretches between). Checked (decomp + disasm).
  - Re:CoM's own resize, `C2DSprObj` vtable `+0x38` = **`0x14024C270(obj, w, h)`**, sets
    those four offsets so the centre part spans (-w/2, -h/2)..(w/2, h/2) and sets flag 8;
    `0x1402493B0(seqObj, w, h)` calls it on every sprite of a sequence. Its only user is the
    message window `0x1402B3040` (`message.sqd`, after each animation change). Checked
    (disasm: w, h in xmm1, xmm2).
  - The fresh object has flag 8 clear (`+0x142` is set to `00 00 00 FF` as a dword by the
    ctor). Checked (disasm `0x14024A00C`).
- The 9-slice path is **hard-wired to exactly 9 parts** (`cmp byte [obj+0x142], 9` in
  `0x14024C270`; the same check in `0x14024AD40`). Other sprites are drawn from their parts'
  rects as they are. Checked (disasm).
- A part (`0x98` bytes, array at `C2DSprObj+0x120`) has at `+0` a pointer to a 0x14-byte entry
  `{i32 x0, y0, x1, y1; u32 uv index}` in the loaded sprite set (set by `0x14024B910` from
  `0x140249150`), so shared with every other user of the set. Each update `0x14024BB10` reads
  it with `0x14024B380` (corners TL, TR, BL, BR), transforms them into the part's vertices
  `+0x40..+0x5F`, and the draw `0x14024A3F0` uses those. Nothing else reads `+0`. Checked
  (decomp + disasm).
- A 1-part sprite whose UVs are all zero gets a `CScissoring` (`obj+0x128`) instead of a quad:
  its corners become a clip rect (`0x14024A3F0`, `0x140265F00`). Checked (decomp).
- **In game (0.4.1, owner's screenshot and log, 2026-10-30):** the panel's open animation has
  4 sprites with **1, 8, 9, 8** parts. The 9-part one is the dark fill: with the bottom offset
  it reached one row lower, exactly the margin it had below "Game Mode" (so its scale at rest
  is 1, and the row step is **26.0** layout units). The glossy border stayed at the old height:
  the 8-part sprites are the frame (3x3 without the centre) and its highlight. Checked (in game).
  Nothing else in the screenshot ends at the old bottom, so the 1-part sprite needs nothing
  (Assumption; the log now prints its rect and whether it is a scissor).

## Re:CoM's Config screen: back to Re:CoM's own (2026-11-03)

Owner decision (2026-11-03, `design-philosophy-instructions.md` § KH1 camp tab: settings
submenus): Combat, Guard reactions and Enemy pace **leave Re:CoM's Config menu**; the camp
menu's "KH1" entry gets two settings pages, **Combat** and **Movement**.

- What 0.4.1 .. 0.92.0 changed (all undone): a wrapper on the draw `0x1400DF1B0` (our rows in the
  free item slots 14..20 after mode 9; the panel's 9-slice bottom offset and rect copies after
  modes 0 / 1 / 4) and a wrapper on the input `0x1400D3EB0` (cursor edges at rows 0 / 4 leading to
  our rows 5..7, left / right on them). No data patch, no byte of the exe or of the sprite set was
  ever written (the panel's frame used our own rect copies). Checked (the 0.92.0 code).
- Now: **the draw is not hooked** (it is only called, for our pages; its first 10 bytes
  `48 8B C4 55 41 54 41 55 41 56` are still checked at start, `sites::CALLEES`), and the input
  wrapper (`48 89 5C 24 08 57 48 83 EC 20`, 10 bytes moved, unchanged since 0.4) hands every call
  to the original unless our "KH1" entry is the one open (`win::keyblades::page_active`: our entry
  exists for this camp and `camp+0xBC` = 7). So Re:CoM's Config screen runs Re:CoM's code only:
  its 5 rows, cursor wrap 0..4, panel size, help lines. Checked (code). The panel-only callees
  (`0x140246210`, `0x14024AD40`, `0x14024C270`) and the `C2DSprObj` vtable check are gone.
- `config_menu = off` (ini) installs neither the input wrapper nor the KH1 entry: Re:CoM's camp
  menu is wholly untouched.

## The KH1 entry's settings pages (native: Re:CoM's Config screen with our rows)

Built in `crates/kh1-recom` (`src/menu.rs` rules and texts, `src/win/menu.rs` the page,
`src/win/keyblades.rs` the submenu). Opened from the "KH1" camp entry's submenu rows **Combat**
and **Movement** (`docs/recom-keyblade-equip.md` § Settings pages).

| page | row | options (left / right cycle) | style | ini key | takes effect |
|---|---|---|---|---|---|
| Combat | Combat | Re:CoM / KH1 | pair | `combat` | at once (see below) |
| Combat | Guard reactions | Off / On | pair | `guard_reactions` | at once |
| Combat | Enemy pace | Re:CoM / KH1 | pair | `enemy_pace` | next battle |
| Combat | Enemy AI | Re:CoM / KH1 | pair | `enemy_ai` | next battle |
| Combat | Revenge Value | Off / On | pair | `boss_revenge` | at once (next boss fight) |
| Movement | Run speed | Re:CoM / KH1 | pair | `run_speed` | at once |
| Movement | Jump height | Re:CoM / KH1 | pair | `jump_height` | at once |
| Movement | Air speed | Re:CoM / KH1 | pair | `air_speed` | at once |
| Movement | Dodge roll | Re:CoM / KH1 / Off | single | `dodge_roll` | at once |
| Effects & Sound | Effects | Re:CoM / KH1 | pair | `effects` | at once (Sora's next KH1 motion) |
| Effects & Sound | Sounds | Re:CoM / KH1 | pair | `sounds` | at once (Sora's next KH1 motion) |
| Camera | Camera | Re:CoM / KH1 | pair | `camera` | at once (the camera's next frame) |
| Camera | Auto camera | Off / On | pair | `auto_camera` | at once (the camera's next frame) |

Damage stays ini-only (owner). The row "Revenge Value" is the owner's name (2026-11-03) for
`boss_revenge`.

### How a page runs (Checked (decomp `0x1400D3EB0`, `0x1400DF1B0`, `0x1400D9C80` states 0x4A..0x4C))

- The camp's states 0x4A..0x4C call `0x1400D3EB0`; with our entry open the input wrapper runs our
  submenu; its settings row opens a page that is **Re:CoM's Config state machine with our rows**:
  - state 0: Re:CoM's draw mode 0 (makes the window panel `camp+0xB30`, animation 0xF7, and the
    cursor sprite `camp+0xBE0`), then our rows in place of its mode 9;
  - state 1: draw mode 1 each frame until it reports the panel's opening ended (idle 0xF8, cursor
    shown; it also glides the cursor);
  - state 2: draw mode 3, then the pad in Re:CoM's order: `0x140111B90` (confirm or cancel) closes:
    draw mode 4 and sound 4; else up / down (`0x140111A70`, masks `0x101000` / `0x204000`, wrapping)
    or left / right (`0x408000` / `0x802000`) on the cursor row; after any of those sound 1 and our
    rows rebuilt (`menu::step`, `menu::cycle`: left −1, right +1, wrapping over the options; for
    two options both flip, as Re:CoM's bytes do);
  - state 3: draw mode 5 until the close ends, then mode 6 (deletes panel, the 21 item slots and the
    cursor sprite) and back to the submenu (header item 1 closes, buttons reappear).
- **Our rows (Re:CoM's mode 9 with our rows):** the 21 slots `camp+0xB38` emptied as mode 9 empties
  them (each item's vtable `+0` with 1), then from slot 0: per row a label `CButton` (label
  sequences `0x1407C1230`, `camp.lad` group 0xB label elements of Re:CoM's row `r`, font 0xD, or
  0xC when `camp+0x9C` = 0 and the language global `[0x140AF73B0]+0x274` is 3 or 6, selected −1)
  and its value item(s) (font 0xD, at the row's value y element). Pages have at most 5 rows, so
  row `r` is exactly where Re:CoM's row `r` is: the panel needs no resizing.
  - pair rows (two options) are Re:CoM's rows 0..3: value items `2r`, `2r + 1` (sequence ids
    `0x1407C1240[item]`, x elements 9/10, 11/12, 15/16, 20/21); the chosen option has selected = 1
    (Re:CoM: byte == option). A pair on row 4 (Revenge Value) takes row 3's two items at row 4's y,
    as our former Config rows did.
  - single rows (Dodge roll, three options) are Re:CoM's row 4 "Game Mode": value item 8 (x element
    0x16) showing the current option, selected 0.
  - Combat page: 15 slots; Movement page: 11 (of 21).
  - With the cursor on a row: the cursor sprite plays 0xFA (pair, on the chosen value) or 0x100
    (single), is moved to the item's sprite position plus Re:CoM's x offset for that item (0.0 for
    item 0, 150.0 for items 1..8), and the help line `camp+0x9D0` gets the row's help through
    `0x140259000`, as mode 9 does for Re:CoM's rows.
- The Config fields are free while our submenu is open (Re:CoM's Config runs only from its top
  entry 6). Re:CoM's Config cursor `camp+0xBE8` and value bytes `camp+0xBED..0xBF1` (save data) are
  never read or written by our pages: our cursor is ours. If the camp is torn down with a page open,
  Re:CoM's teardown `0x1400D7C20` deletes the panel, the 21 slots and the cursor sprite itself
  (Checked (decomp)).
- A change: `settings::Settings` updated, `kh1-recom.ini` rewritten through `settings::store`
  (other lines and comments kept), and the live readers told (`win::set_item`), log line
  `kh1 entry: <key> = <value> (now | from the next battle)`.
- Text: our labels, options and help are NUL-terminated ASCII handed to the same text items (the
  message table is bypassed). English, as every text of ours in the camp (the Config rows, the KH1
  pages); `menu_language` only picks the command menu's KH1 words from KH1's own files.
  Help lines are kept within 66 characters (`menu::HELP_MAX`; the box cuts at about 69, Checked
  (owner's screenshots, 2026-10-31)).

### When each change takes effect (Checked (code))

- **Combat**: at once. `movement::set_mode` (every KH1 handler, the MP state `mp::active`, the
  command menu, Guard, items and abilities read it live), the MP bar refreshed on the live gauges,
  the Keyblade model prefetched (shown when the camp closes), the KH1 magic effect set loaded in the
  field (`magic_fx`). A battle reads it when it starts (`battle::kh1_battle`), so a field change
  applies to the next battle. This is exactly what the former Config row did. Inside the camp the
  KH1 entry's submenu shows the new row list when the Combat page closes (Items / Customize /
  Abilities appear or go; the cursor stays on Combat).
- **Guard reactions**: at once (`attack::set_guard_reactions`).
- **Enemy pace**: next battle (read at the battle manager's construction, `battle::manager_ctor`).
- **Enemy AI**: next battle (read in each enemy's ctor wrapper when a battle makes it,
  `kh1_enemy`). Nothing else reads it, and the enemy motion packs (`ENEMY/<id>/EN<id>.ANN`) are the
  same files whatever the setting (Panacea serves the mod's copy; no code of ours picks a file), so a
  switch in the same room needs no room reload. Checked (code). An enemy whose own pack lacks its
  class's KH1 clips keeps Re:CoM's AI even at KH1 (`recom-m11-enemies.md` § Enemies whose pack has no
  KH1 clips; owner's T-posed Shadows, 2026-11-03).
- **Revenge Value**: at once (`revenge::set_enabled`; its hooks and boss sizes are installed
  whatever the value, and each hit checks it); in practice the next boss fight.
- **Run speed / Jump height / Air speed / Dodge roll**: at once: each KH1 frame copies them
  (`movement::run_frame`, `stick_class`; `set_speeds` / `set_roll_mode`). Assumption: a jump or
  roll is not in progress while the camp is open (the camp opens from the field's idle states).
- The movement rows only act at Combat = KH1 (Re:CoM's own movement otherwise); their help lines
  say "KH1 only".
- **Effects / Sounds** (M13, 0.116.0; owner, 2026-11-04: "The Effects / Sounds (later Voices) toggles
  go on a new Settings page"): the fourth row of the Settings list, **Effects & Sound**, opens a
  page with these two rows (Voices is added once voices exist). At once: `win::fx::variants` reads
  both at each of Sora's KH1 motions, so the next swing follows; a trail already playing finishes as
  it started. They act only at Combat = KH1. The label's `&` passes Re:CoM's escape expander
  unchanged (`0x1403480C0`: bytes under 0x81 are copied as they are; Checked (decomp)). That
  Re:CoM's font has an `&` glyph is an Assumption, to check in game.
- **Camera** (M12 camera, 0.134.0; owner, 2026-11-04: "a new Settings > Camera page", the fifth
  Settings entry, independent of Combat): one row, Camera Re:CoM / KH1 (KH2 joins at its stage);
  Auto camera Off / On (0.135.0, KH1's Auto pieces). At once: the camera's dispatcher reads the setting every frame
  (`win::camera`) and switches the live camera with Re:CoM's own mode setter `0x1401FBE60`; it acts
  at both Combat settings, in Sora's story only (`docs/recom-m12-camera-plan.md` § Stage 1 built).
- **Effects and Sounds are separate** (current behaviour, kept; owner question): an effect's sound
  rides on the effect (Re:CoM plays it from the effect set's `.ese`, `0x1402546E0`). So:
  - Effects = KH1, Sounds = Re:CoM: KH1's finisher effects keep **KH1's own effect sounds** (81–88,
    `kh1_ex_0010.ese` on bank 9000, which is registered at every KH1 battle start whatever Sounds
    says); the swings, finishers' motion sounds and whooshes are Re:CoM's (SES variant 0).
  - Effects = Re:CoM, Sounds = KH1: Re:CoM's effects with **Re:CoM's effect sounds** (their own
    `.ese`), and KH1's swing / finisher motion sounds (SES variant 1301 and up).
  - The KH1 Keyblade trails have no sound of their own (Checked (files)), so the trails don't
    differ.

### Riku's story

Since the follow-up of 2026-11-03 (owner: "show the KH1 tab in Riku's story too, with only Combat
and Movement") the KH1 entry shows in Riku's story with the Combat and Movement pages only
(`docs/recom-keyblade-equip.md` § Riku's story). The pages are the same; the entry's and the two
rows' help lines say the settings are Sora's story's. **None of the nine rows has an effect in
Riku's story** (Checked (code), each reader's gate): our KH1 battles, Guard, movement and enemies
are Sora's story only; the values are saved and used in Sora's story. Owner question there: keep
them shown in Riku's story, or hide them. The camp that opens straight into the map page still has
no entry (it has no top list).

## Needs in-game testing

- Re:CoM's Config screen: exactly 5 rows, its own panel size, cursor wrap 0..4, help lines.
- KH1 entry at Combat = Re:CoM: shown under Config; its submenu shows Combat and Movement only.
- Each settings page: Re:CoM's panel opens, rows at Re:CoM's row positions, cursor and help on each
  row, left / right change the value (sound 1), the ini changes at once; confirm or cancel closes
  (sound 4) back to the submenu with the cursor on the row.
- Combat flipped on the Combat page: closing the page shows the new row list.
- Opening and closing the camp and the pages repeatedly (item ownership and deletion).
