# The KH1 HUD option (decision 39)

Owner (2026-10-31): "Would we be able to instead have an option where the Re:CoM [HUD] is
replaced entirely by the KH1 hud? ... use the KH1 decomp to get exact positioning and graphics".
Details (decision 39): HP ring on Re:CoM's raw HP (KH1's rule), party gauges hidden until they
join, menu words in KH1's font converted into a Re:CoM font, field / battle command menu looks,
Re:CoM's enemy gauges hidden and KH1's lock-on cursor used, Re:CoM's low-HP sound, Re:CoM's
extras hidden for now (the owner places them later).

Stages: H0 dump (done, outside the repo) → **H1 the `kh1hud` set** → **H2 Sora's cluster** →
**H3 hiding Re:CoM's HUD** → **H2b the cluster's motions** → H4 command menu look → H5 LOCK ON! +
target gauge → H6 party gauges → H7 lock-on cursor → H8 popups.

Steam addresses (image base 0x140000000), both exes. Tags: **Checked (decomp)**, **Checked
(exe)**, **Checked (files)** = the owner's files (the H0 dump of 2026-10-31), **Assumption**.

## KH1's HUD data (H0 dump, Checked (files) + decomp)

- KH1's HUD pieces are sprite objects in `exchange/<LANG>_uibin_*.bin` (UK, GR, FR, SP, IT on PC;
  English is `UK`: `0x1400D7380` maps the language index to JP, UK, UK, GR, FR, SP, IT, FM).
  Header `"TTUI"`, 2, then four (offset, size) pairs: objects (0xC0), colour keys (0x20), vertex
  colour keys (0x50), position keys (0x20) (`0x14027D9D0`). Object: `+0x00` **x, y, w, h**
  (`0x14027EA50` draws (x, y)–(x + w, y + h)); `+0x10` u0, v0, u1, v1 inclusive (u1 < u0
  mirrors, `0x14027E7D0`); `+0x20` RGBA or four vertex colours (flag 0x20); `+0x60` flags (0x02
  colour keys, 0x40 vertex-colour keys, 0x04 position keys; +1 bit each while running); key
  players `+0x64`, `+0x78`, `+0x8C`; `+0xA0` the name. The research note's "(x, y, z, w)" and
  "0x50 keys = scale / rotation" were wrong: the 0x50 keys are four keyed vertex colours.
- Positions are absolute in KH1's 512 × 384 space; the code adds the size-class offset and the
  widescreen margin (`0x140269190`, `0x14026CE90`), and for the straight segments (−1, +1)
  (`0x1402698C0`, `0x14026BEE0`, `0x14026C6E0`; the charge's (0, +1), `0x14026A970`).
- The 4:3 space is centred on a 16:9 screen: left pieces at x − 86, right ones at x + 86
  (`0x14232DCC8` / `0x14232DCCC`). The owner's KH1 screenshot (2048 × 1152) puts Sora's face at
  1838 px = 612.7 units = 443 + 86 + 85.33. Checked (files).
- `command2/uitex.bin`'s PC assets: one per picture, named `-<pixel offset>.dds` (picture 0
  512 × 512, picture 2 256 × 384 with PC-only rows below, the others 2×; DXT5 or A8R8G8B8), plus
  per language `<LANG>_uitex_bin40.dds`, `…10780.dds` (picture 7: "mp" / "HP"), `…16c40.dds`,
  `…20c0.png` (picture 1, words). Checked (files).
- Faces: section 2 of `xa_ex_0010.mdls` (Sora, 192 × 64, three cells), `xa_ex_0040.mdls`
  (Donald, 48 × 48), `xa_ex_0050.mdls` (Goofy, 48 × 48), PC `-<pixel offset>.dds` at 2×.
  Checked (files).
- Fonts: the PC menu font is `<lang>/font/menufont.tm2` / `.inf` / `.cod` in `Image/dt/Theater.pkg`
  (16 MB TIM2), the slot-1 font of the table `0x1404D9150` the command menu's text uses
  (`0x14027A920` → `0x1402D1CD0`, font 1). For H4.

## Re:CoM's PC 2D files (Checked (files + decomp))

- `FO0002.SPR` holds `gauge.lad`, `Cockpit.lad`, `layout_card.lad`; its PKG entry's PC assets
  are `<LANG>_Cockpit.lad` / `<LANG>_gauge.lad` (FR, GR, IT, SP, UK). `FO0002.RTM` holds
  `Cockpit.imz`, `layout_card.imz`; its PC assets are `<LANG>_Cockpit.imz`: a 1024 × 1024
  32-bit IMGD (PSMCT32, linear, RGBA with PS2 alpha 0x80 = opaque).
- Re:CoM takes a PC asset whose name contains the file's name and ends in `.lad` / `.sqd` / `.spd`
  (`0x14029FCC0`: `strstr`, then the extension) in place of the file. The PC layouts' UVs
  address the PC atlas in its own texels (`UK_Cockpit.lad`'s sprite 1: PS2 (3, 160)–(126, 177),
  PC (520, 92)–(924, 150): a re-laid atlas, not a plain × 2).
- A `.lad`'s registration (`0x140244AD0`) stores its two layout tables and registers each SEQD
  as `<name>:NN`; nothing else in it is needed to make a set.

## H1: the `kh1hud` set (asset step + DLL check)

- **Asset step** (`kh1-recom-assets` `kh1_hud`, `uibin`; bin `kh1-recom-build`): reads KH1's
  `command2/uitex.bin` (+ PC assets), `exchange/UK_uibin_g_s.bin`, `UK_uibin_g_red.bin`, the three
  faces, Re:CoM's `FO0002.SPR` (after the MP bar's patch) and `FO0002.RTM`, and writes
  - `kh1hud.lad` into `FO0002.SPR` (one SEQD → `kh1hud:00`) and `<LANG>_kh1hud.lad` assets (the
    same records with the PC atlas' UVs) for every language Re:CoM has `<LANG>_Cockpit.lad` for;
  - `kh1hud.imz` into `FO0002.RTM` (PS2 atlas 1024 × 512, 32-bit) and `<LANG>_kh1hud.imz`
    (PC atlas 2048 × 1024, 32-bit, that language's KH1 picture 7);
  - `raw/FORM/0002/FO0002.RTM` (new) and `raw/FORM/0002/FO0002.SPR` (as before, now with
    `kh1hud.lad`), and `kh1-hud.json` (format `kh1-recom-assets kh1-hud 1`, the group count, the
    languages, the inputs' sha256). Re:CoM's own files and records stay byte for byte.
- **Atlas:** KH1's pictures 5, 6, 7 and the three faces, PS2 and PC; and KH1's arcs baked: the
  asset step runs KH1's own fan (`0x14026B790`, vertices `0x14026E450`) over the ring textures
  and stores each piece a frame needs (§ H2). One texel of gap (two on PC) around each box.
- **DLL** (`kh1_hud`, `win/kh1_hud.rs`): reads `kh1-hud.json` (refuses another format or group
  count) after the MP bar and command menu install; at each player gauge's init (the MP bar's
  wrapper `0x140208360`) checks once that Re:CoM's 2D registry holds `kh1hud:00` with the
  manifest's group count and logs it.
- **Switch:** `hud = recom | kh1` in `kh1-recom.ini`, default `recom`; interim until the KH1 camp
  entry has its row.
- Checked (files): the set built from the owner's dump and drawn by an independent script (SEQD
  parse, PC atlas) over the owner's KH1 screenshot: ring radii, positions, labels, face, segment
  pitch line up with KH1's.
- Assumption: Re:CoM's IMGD reader takes a 32-bit picture in the PS2 (non-PC) file too (only
  used when no language copy matches).

## H2: Sora's gauge cluster (bottom right)

### KH1's rules (Checked (decomp + exe) unless tagged)

| what | where |
|---|---|
| Slot 0's objects: `+0x10` face disc `g_s_fra_a_a..d` + "HP" / "mp" labels (picture 7; "mp" skipped without MP), `+0x50` face frame `g_s_fra_a_wa..wd` (picture 6), `+0x78` / `+0x80` max-MP ticks (objects 37, 38: past 90° / 180°), `+0x90..+0xC8` the straight pieces: colour HP 31, MP 32, charge 33; back HP 16 (first) / 18 (further, x + 26), MP 17; caps 20 (x := 0x19E), 21 (x := 0x1B8) | index lists `0x1405270E0`, `0x140527138`, `0x140527168`, `0x140527180`; `0x14026E630` |
| Centre (443, 316) + size class offset (24, 24) / (0, 24) / (0, 0): class 0 when max HP < 19 and max MP < 4, 1 when max HP < 37 and max MP < 7 | `0x1405272B0`, `0x140527300`, `0x1402690D0` |
| Draw order: face frame, disc + labels, HP back arc (to min(max HP, 120)·5°) + back segments + cap, MP back arc (min(max MP, 20)·30°) + segments + cap, MP arc, danger flash, HP arc + segments, charge arc + segments, ticks, face | `0x140269190`, `0x1402698C0`, `0x140269F50`, `0x14026CF90` |
| Arcs: a triangle fan from (centre x, centre y + 1) over the square of half-size R; vertex at θ: d = trunc(tan t · R) (1/16 units), right / bottom halves one unit out; UVs fixed in the quarter-ring texture; rings (1/16): HP back (744, 1288) r688, MP back (1720, 1512) r912 (picture 7), HP (680, 1592) r656, MP (904, 904) r880, charge (1816, 904) r880 (picture 5) | `0x14026B790`, `0x14026E450`, `0x1405273B0` |
| Straight segments at (−1 − 26 i, +1) from the object (charge: (−26 i, +1)); a partial one is the object with x + cut, w − cut, UV kept: cut = (int)((1 − rem / per) · w + 0.5), 1..w−1 (HP per 6 HP, w 26; charge per 30, w 24) | `0x1402698C0`, `0x14026BEE0`, `0x14026A970` |
| MP fade: the changed arc RGB 0 → 128 (gain) / 128 → 0 (loss), 12.8 a frame, 10 frames; charge: alpha the same way; ring part only | `0x14026C6E0`, `0x14026A770` |
| Danger: HP > 0 and HP / max < 0.2 (max > 44) else HP < 9; flash = `g_red`'s `g_s_bar_b_hp_red` keys: colour (128, 0, 0, a), a 0 → 128 over 8 key units and back over 8 (16 + 16 frames), restarted when done, on the max-HP arc and segments under the HP arc | `0x14026E180`, `0x14026D570`, `0x14026BCF0` |
| Face: cell = expression of the h × h strip centred on the gauge centre; 1 for 40 / 50 / 60 frames after a hit (new / old HP ≥ 0.8 / ≥ 0.5 / < 0.5), else 2 in danger, else 0 | `0x14027F470`, `0x140268D60`, `0x14029BBD0` |

### Native build

- **Data** (asset step, in the H1 set): the arcs are baked by running KH1's fan in the asset step
  (the same vertices, triangles, UVs and sampling points as the GS draws, PS2 and PC pictures) into
  pieces: per quadrant every 5° (HP and HP back, 54 + 54 pieces), every 30° (MP back), per 30° cell
  (MP), per cell and degree (charge, 270). Held groups take KH1's values as frames (frame →
  pieces, compiled into Re:CoM animations). Fades, the hurt face and the danger flash are played
  groups (`crate::kh1_hud::group`).
- **Controls** (`win/kh1_hud.rs`): at `hud = kh1` in Sora's story, at the player gauge's init:
  three `C2DSeqCtrl` (Re:CoM's `operator new` `0x1404ACE54` + ctor `0x140247260`), no parent,
  layout position `0x140248660` = the size-class offset, every layer at KH1's right margin (86 at
  16:9, 0 at 4:3, from the gauge's `+0xA0`) with flag 1: back (priority 2: frame, HP back, MP back,
  MP), front (3: HP, charge, ticks), face (3). Pieces (fades, flash) are pooled controls, flag 0
  (played), priority 2 (MP fades, flash: under the HP ring) or 3 (charge fades).
- **Events only:** Re:CoM's HP setters `0x14020D6A0` / `0x14020D8E0` (wrapped, `sites::KH1_HUD_HOOKS`:
  `48 89 5C 24 10 57 48 83 EC 40`, `45 33 C0 0F B7 C2 85 D2`) give HP and max HP (`+0xA4`,
  `+0xA8`: Re:CoM's raw HP, decision 39); KH1's MP reads / writes (`win/mp.rs`) give MP and charge;
  the gauge's HUD state (7 / 1) hides and shows ours; the dtor deletes ours. Nothing per frame.
- **The 7.2 MP bar** is not made at `hud = kh1` (the command menu still finds its groups).
- **Re:CoM's HP gauge** keeps running (its low-HP sound 9 at ≤ 25 % is the owner's choice); H3
  hides it.

### Not 1:1 yet (to ask / later; H2b below builds 1, 2 (soft edge), 4 and settles 5)

1. **Hit shake and size-class slide** (`g_act`: `vibe_s/ss/sss`, `slide_s_h/v`, position keys):
   they move every piece at once; Re:CoM's 2D animates per layer, so a native shake needs the shake
   keys in every held group or the controls moved over the shake's frames (per frame while it
   runs). Not built; the class offset changes at once.
2. **The arcs' end strips** (`0x14026B790` mode 1–3: a piece at the back arcs' end; mode 4: a
   0.7° soft edge on the HP, charge and flash arcs): their angles are float arguments the decomp
   lost; not baked.
3. **Charge fade overlap:** a fading piece starts at its 30° cell's start, so for 10 frames it is
   drawn over the part of that cell already shown (same texels; only edge texels get a little
   more opaque). KH1 fades exactly [old, new].
4. **The gauge's own fade** (`+0xA9C`, ±0.1 a frame, menus / events) is not reproduced: ours shows
   and hides with Re:CoM's HUD state at once.
5. KH1 restarts a running MP fade from its last value; ours drops it.
6. Assumptions: same-priority controls draw in creation order (the MP bar's assumption 4); a
   played group holds its last frame (5); a group word `+4` of 0 loops (the danger flash);
   `0x14029BBD0`'s `iVar4` is the HP after the hit (strength rule).

### What the first in-game test must check (H2)

1. `hud = kh1` in `kh1-recom.ini`, Combat = KH1, Sora's story. Log: `kh1 hud: Sora's KH1 gauge at
   hud = kh1 (H2)…`, then per room `kh1 hud: built on the player gauge …: HP h/m, MP Some((…)),
   class c, margin 86`.
2. **Screenshot** in the field at full HP / MP: the cluster bottom right as KH1's (ring sizes,
   "mpHP" labels, face, segment pitch); Re:CoM's HUD is still there (H3).
3. Take hits: the green arc shrinks at 5° per HP; Sora's face grimaces for 40–60 frames.
4. Low HP (< 20 % of max, or < 9 HP when max ≤ 44): the max-HP arc flashes red under the green;
   the face shows the exhausted look; Re:CoM's low-HP sound plays.
5. Cast a spell: the MP cell goes black then disappears over 1/6 s; refill: cells rise from black.
6. Swing at enemies: the orange charge line grows along the outer ring, fading in.
7. Events / cutscenes hide it with Re:CoM's HUD; a battle and back: it is rebuilt.
8. `hud = recom`: Re:CoM's HUD and the 7.2 MP bar as before.

## H2b: the cluster's motions baked into the data (decision 39, 2026-11-01)

### KH1's rules (Checked (decomp + exe + files))

| what | where |
|---|---|
| One motion pointer per slot (`+0x110`): `0x14026D990` adds its keyed offset (player C, x / y) to the centre every piece of the cluster is drawn from | `0x14026D990` |
| A hit (`0x14029BBD0` → `0x140268D60(slot, strength)`) starts `vibe_s` / `vibe_ss` / `vibe_sss` (slot `+0x2A0` / `+0x360` / `+0x420`, `g_act` objects 0, 1, 2) **and the hurt-face timer, only when no motion runs** (pointer 0) | `0x140268D60` |
| A size-class change starts `slide_s_h` (new class 0 / 1, slot `+0x120` = `g_act` 3) or `slide_s_v` (class 2, `+0x1E0` = 4), over a running shake; the drawn class stays the old one until the slide ends, then the new one | `0x14026D570`, `0x14026D990` |
| Keys: `vibe_s` y 0, 3, −3, 2, −2, end 0 (1 unit each); `vibe_ss` 0, ±6, ±4, ±3, ±2, ±1 (11 units); `vibe_sss` 0, ±9 … ±1 (19); `slide_s_h` x 0, −4, 32, −36, −18, end −24 (units 2, 2, 4, 1, 1); `slide_s_v` the same in y. Players step 0.5 a frame, drawn after the step: 2 frames a unit (10, 22, 38, 20 frames) | `exchange/UK_uibin_g_act.bin` (Checked (files)), `0x14027E430` |
| The gauge fade (`+0xA9C`, `0x140268E30`): ±0.1 a frame, stepped before drawing (0.9 … 0.0 out, 0.1 … 1.0 in), multiplies every vertex alpha | `0x14026D570`, `0x14026B790` |
| MP and charge fades: every change sets the fade afresh (`0x14027F760`: 128 → 0 for a loss, 0 → 128 for a gain) and draws only the latest change's arc. **H2's gap 5 was wrong**: KH1 does not resume a running fade; H2 already restarts it, as KH1 | `0x14026C6E0`, `0x14026A770`, `0x14027F760` |
| Soft end (mode 4: the HP arc, the max-HP flash, the charge and its fade arcs): when 1 < end < 270 the fan stops 0.7° short and a last triangle (centre, end − 0.7°, end) fades to alpha 0 at the end vertex | `0x14026B790` (`0x14042F7F4` = 0.7; callers push 4: `0x14026BDF6`, `0x14026BE57`, `0x14026A770`) |

### Re:CoM's side (Checked (decomp + exe))

- A layer whose frame was set never advances (`0x140248BB0`: `+0x38` ≥ 0 holds the time), so a
  held layer cannot move by itself. A played layer runs to its group's end; after the end only
  the group's longest animation stays drawn (`0x140248F40` marks it, bit 4 of `+0x144`).
- `0x140248550(ctrl, layer, time)` plays a layer from a time (f32 in xmm2, units of 1/50 frame:
  `0x140246710` → the `C2DSeqObj`'s vtable `+0x30` `0x140249360` sets `+0x38` = −1, `+0x30` =
  time); `0x140248000(ctrl, layer)` returns its time (`0x140246250` → vtable `+0x68`
  `0x140246D60` → `+0x30`). Re:CoM's own code uses both (16 and 3 callers).
- Animation flags (`0x14024B4E0` shuffles them; `0x14024A9xx` reads): bit 0 linear (no curve);
  translation interpolated only when 0x10000 and 0x1000 are clear (else the start offset stays);
  0x8000 = no extra frame after `f1` (end exclusive). Translation: `+0x10` x0, `+0x14` x1, `+0x18`
  y0, `+0x1C` y1, ints in 2D units (+0.25). Re:CoM's `Cockpit:00` has both kinds (0x6B73 moving,
  0x4FB71 exclusive).
- An animation group's first animation and count are read as unsigned 16-bit (`0x140248F40`), so
  a SEQD holds up to 65535 animations.

### Native build

- **Data (asset step):** every distinct piece set of the held groups FRAME … FACE (a *unit*: 852
  on the owner's files) gets one played group per program: `vibe_s`, `vibe_ss`, `vibe_sss` (the
  KH1 keys as moving animations of 2 frames a key unit, flags 0x6EFF1, then the last offset held to
  frame 10,000,000), `fade_out` / `fade_in` (alpha 115 → 0 / 13 → 128 over frames 0..9, then
  held), and for the 423 units a slide can show (HP ≤ 72, charge ≤ 210, the rest: Assumption, a
  bound) `slide_s_h` / `slide_s_v`. The hurt-face groups carry the shake of their strength. The
  manifest (format `kh1-hud 2`) lists the programs (start frame, length), each unit's first group
  and each run (held group, frames, unit). 5738 groups, 43,030 animations, `kh1hud.lad` 6.4 MB (×6
  copies in `FO0002.SPR`).
- **Soft ends** are baked into the HP and charge arcs' last pieces, the charge fade pieces below a
  full cell and the flash pieces below a full quadrant.
- **DLL (events only):** a hit with no motion running → the held back / front controls hide and
  the units of what they show play `vibe_<strength>` (from frame 1: KH1 draws key time 0.5 first),
  with the shaken hurt face; a size-class change → the slide at the old class's place (the held
  controls already at the new one); Re:CoM's HUD hide / show (state 7 / 1) → `fade_out` /
  `fade_in` on the units (face included). A view change while a motion runs rebuilds its units at
  the same time (`0x140248000` on the first unit layer); a view change after it ended gives the
  screen back to the held layers. Nothing per frame.

### Not done / to check

1. **Back arcs' end strips** (modes 1–3, `0x1402694C0` mode 1, `0x140269F50` computed): black
   strips built by `0x14026E270` whose float arguments the decomp lost; not baked yet (needs the
   disassembly of `0x14026B790`'s tail).
2. While a motion runs, the MP / charge fade pieces and the danger flash stay where they are (KH1
   moves them too): visible only when MP or the flash changes during the 10–38 frames.
3. A slide for a change that isn't KH1's (class 0 → 2 or downwards) ends at the slide's last
   offset and snaps to the new class at the next HP / MP event (KH1 snaps at once).
4. The fade-in runs on every Re:CoM show (events, menus); KH1 hides at once for its events
   (world flag 8) and fades for its menus: owner question.
5. Assumptions: Re:CoM's 2D time step is 50 units a 60 Hz frame (as the MP bar's fades); a played
   layer added after the held ones at the same priority draws over them (creation order).

### What the in-game test must check (H2b)

1. Take a hit: the whole cluster (rings, segments, labels, face) shakes up and down for 10 / 22 /
   38 frames (by how much HP the hit took), the face grimaces; a second hit during the shake
   neither restarts it nor the grimace.
2. Open the camp / start an event: the cluster fades out over 1/6 s; closing it fades it back in.
3. Low HP (under 54 HP): the green arc's end is soft over its last 0.7°.
4. Log: `kh1 hud: … built …` as before; no `no slide units` line in normal play.

## H4: KH1's command menu look (`hud = kh1`)

7.3's menu (its rules, cursor, fades, tab, request bits) is unchanged; at `hud = kh1` its rows are
drawn with KH1's own pieces instead of Re:CoM's plates. `hud = recom` keeps 7.3's look.

### KH1's rules (Checked (decomp + exe + files) unless tagged)

| what | where |
|---|---|
| Root list at (16, 260) + left margin; a submenu at its parent's x + 11 (7.3) | `0x14027B870` |
| A list: header (`*_fra_top` and the title) at the origin; rows at y + 20 + 22 i (`*_fra_c`; the last row `*_fra_d` + the connector `*_fra_cd`); the cursor row is not drawn as a frame but as the selection (`*_sel_sha`, `_act`, `_gra`, `_bla` + the command's icon) at (x + 11, y + 18 + 22 c) | `0x140278B40` |
| Words at the row's (x + 16, y + 22 + 22 i), the cursor row's at (x + 29, y + 25 + 22 c) (`0x14027A920` adds (16, 2)) | `0x140278B40`, `0x14027A920` |
| The root's pieces 24 units (and texels) wider (`param_10` of `0x14027A600` / `0x14027A9F0` for list type 0; picture 2's panel art runs to texel 125) | `0x140278B40`; Checked (files) |
| Battle (`0x142867364 & 1` → the root record's `+0x1F8` = 0): the root's panels RGB (109, 30, 32), the selection plate (216, 109, 100) (`0x14027A600` replaces the vertex RGB; `0x140278B40`'s `local_168` / `local_118`); the field look is the objects' own colours | `0x14027A0B0`, `0x14027A600` |
| Title: `com_battle` `com_command_start` in battle (blue → (128, 128, 255, 255) → red (200, 80, 80), 2 + 18 frames), `com_command_end` in the field (back to (86, 126, 223)), restarted when the mode changes; × 0.5 while a submenu is open | `0x14027B870`, `0x140278180`, `0x140278B40` |
| Pictures: the root's panels picture 2, the submenus' 3, titles and icons 1 (PC: `-4140.dds` 256 × 384, `-61c0.dds`, per language `<L>_uitex_bin20c0.png`) | `0x140278180`; Checked (files) |
| Words: font 1 (`menufont` of `<lang>/font/`, Theater: 2048 × 2048 32-bit, 32-texel cells, `.cod` 8-byte glyphs: code, x, y, 0, width) drawn at size 14 (`0x14054534C` over the cell, `0x1402D1DB0`) | Checked (files + decomp); the PC path's scale: Assumption (strong: "Attack" = 42 units here, 43 on the owner's screenshot) |

### Native build

- **Data (asset step):** pictures 1, 2, 3 join the atlas (picture 1 per language from its PNG:
  a small PNG reader, `miniz_oxide` from the lockfile); after the motion groups: the root's field
  look and battle look (header, title, row, last row, selection, three icons each), the red
  title's colour keys (played), the Magic list's six, then one group per word and language: KH1's
  glyphs (32-texel cells into the atlas, the PS2 copy at half size) as parts at the font's
  advances × 14 / 32. `kh1-hud.json` `"menu": {"first", "words": {lang: [[id, group]]}}`.
  Message 6 ("Summon") joins the decoded words.
- **DLL:** `win/command_menu.rs` `show_kh1`: per row a plate control (the frame / last frame /
  selection + icon) and a word control (colour = KH1's brightness / 128, alpha the fade), a header
  control for the root and one for the Magic list; KH1's left margin −86. The look is the battle
  one while `player + 0x390` (Re:CoM's battle room) is set; switching rebuilds and plays the red
  title from frame 1. Root row 3 with no context command reads "Summon", greyed (locked as 7.3's
  blank row). Nothing new per frame: the rows rebuild only when 7.3's did, plus a look change.

### Owner questions (H4)

1. **Font:** KH1's glyphs are Re:CoM sprite parts in the `kh1hud` set (a group per word and
   language), not a `C2DFnt` font: Re:CoM's font format is not reverse-engineered. (a) keep;
   (b) research `C2DFnt`'s font files and convert KH1's into one.
2. **Icons:** a = Keyblade on Attack / Counter, b = hat on Magic, c = potion on Items (by their
   pictures; KH1's choice not read). Summon has none.
3. Root row 3 shows "Summon" greyed when no context command is in it (the brief's choice); KH1
   shows it only once a summon is learned.

### Not 1:1 / to check

1. KH1's slide-in of a list (the objects' C keys −11 → 2 → 0 over 4 frames) is not played
   (as 7.3).
2. The 13 KH1 adds with the root's width (`param_9`, to `0x1428669A0` / `B0`) is not applied:
   its meaning (UV or clip) is not settled. Assumption: the art at +24 texels is what shows.
3. A space in a word advances 9 texels (no glyph in the font). Assumption.
4. The PC atlas's glyph texels are doubled (PS2 range in the font). If the words look too bright
   or dark on screen, that is why.

### What the in-game test must check (H4)

1. `hud = kh1`, field: the KH1 menu bottom-left as the owner's KH1 screenshot (blue in the field):
   COMMAND tab, Attack highlighted with the Keyblade icon, Magic / Items / Summon below in KH1's
   font; Summon greyed.
2. A battle: the tab and panels red, the title turning red over 1/3 s; back in the field: blue.
3. Magic open: the purple Magic list at x + 11 with its header, the root's title dimmed.
4. A chest: row 3 slides in reading Open; Triangle opens it once in (7.3's rules).
5. `hud = recom`: 7.3's Re:CoM plates as before.

## H5: LOCK ON! and the target's HP gauge (top left)

### KH1's rules (Checked (decomp + files) unless tagged)

| what | where |
|---|---|
| The plate: `lock` objects 5..7 (`lockon_plate_a/b/c`, picture 0) and 8..10 (`lockon_text_a/b/c`, picture 1) at (left margin + 16, 24); on a lock their colour keys restart and play (alpha 0 → 150 / 250 over 2 frames, → 100 / 150 over 10), then hold; when the lock ends alpha − 1/6 a frame | `0x1402747F0`, `0x1402740F0` |
| The gauge's origin (left margin + 16, 24), + 28 while the plate shows, (65 / 80 in minigames) | `0x140275000`, `0x1402743E0` |
| `g_enm` 0 (picture 6) + 1, 2, 3 (picture 1: the green "HP") at the origin; 4 (6) + 5 (1) from x + 28, `L` = min(max HP / 2, 150) wide (their texture stretched); the cap 12 (6) + 13 (1) at x + 28 + L | `0x140275000` |
| Five bars, one per 300 HP: bar k = min(HP − 300 k + 1, 300) / 2 wide while HP ≥ 300 k (texture stretched); with more than one bar, the marker (11) at the top bar's end | `0x140275000` |
| Fade: state 1 draws alpha 0, then + 0.2 a frame to 1; the lock's end: 1, then − 0.2 a frame | `0x1402758B0` |

### Native build

- **Data:** picture 0's corner (texels 96..128, PC 4×) joins the atlas; groups after the menu's:
  the plate (each piece's colour keys, played), the head, the body and the five bars as X-scale
  groups over frames 0..150 (frame = width, as the MP bar's partial segment), the cap, the
  marker. `kh1-hud.json` `"target": {"first"}`. The bars' colours by stack index (green, yellow,
  orange, magenta, violet: objects 10, 9, 8, 7, 6): Assumption (KH1's slot order not read; the
  owner's screenshot shows yellow over green).
- **DLL** (`win/kh1_hud.rs` `lock_tick`, called each of Sora's frames from the request-bits
  wrapper before the command menu): reads Re:CoM's lock (in battle, `*(player+0x3F8)+0xE0`, the
  handle `player+0x84C`, an enemy) and the target's HP (`+0x274`); max HP from Re:CoM's getter
  `0x14017F5A0` (KH1's for proxied enemies, `win/stats.rs`). A lock makes the plate (played from
  frame 1) and the gauge controls (KH1's left margin −86); HP / target changes set the frames
  and the cap / marker places; the fades write the controls' colours for their 5 / 6 frames.
  Hidden at once with Re:CoM's HUD, deleted with the player gauge. Per frame otherwise: three
  reads.

### Not 1:1 / to check

1. KH1 shows the gauge while the lock-on cursor reports HP; here while Re:CoM's lock-on is on in
   battle (Re:CoM has no lock in the field).
2. The minigame heights (65 / 80) are not used (Re:CoM's minigames are its own).
3. When the target changes during a lock KH1 keeps the plate; so does ours.

### What the in-game test must check (H5)

1. Lock on in battle: "LOCK ON!" fades in top-left as the owner's KH1 screenshot, the HP gauge
   under it fades in over 5 frames; its length = the enemy's max HP / 2 (≤ 150).
2. Hit the enemy: the bar shortens; an enemy over 300 HP shows stacked bars and the marker.
3. Release the lock: both fade out (6 / 5 frames).
4. Log: `kh1 hud: lock on: target … HP h/m (…)`.

## H6: Donald's and Goofy's gauges (top right)

### KH1 (Checked (decomp + exe + files) unless tagged)

- Slots 1 and 2 draw with Sora's functions; their objects are `g_c` (32) at absolute places
  around (400, 44), plus the slot offset (Donald `0x140527330` (0, 0), Goofy `0x140527370`
  (68, 20)) and the right margin; the face 48 × 48 (`xa_ex_0040` / `0050` section 2) centred on
  `0x1405272C0` (399, 44). Rings: HP r26 (`0x140527400`, picture 5's `g_c_bar_*_hp` quarters),
  MP r32 (`_mp`), backs `g_c_fra_*` (picture 6); straight parts 17 apart past 54 HP / 9 MP.
- Re:CoM (Checked (decomp + strings)): `Friend::CFriend` (ctor `0x1401D9000`, base dtor
  `0x1401D90F0` run by every friend's), `CFriendDonald::beginStateAppear` `0x1401DDC20`,
  `CFriendGoofy::beginStateAppear` `0x1401DF9C0`, `CFriend::beginStateExit` `0x1401D94D0`.
  Re:CoM's friends have no HP or MP of their own.

### Native build

- **Data:** two held groups (Donald, Goofy): face frame, disc, HP back, MP back, MP ring, HP ring
  (the three full quarters each), face. `kh1-hud.json` `"party": [donald, goofy]`.
- **DLL:** per player gauge two controls at the slot offsets (right margin +86), hidden; four
  wrappers (`sites::KH1_HUD_PARTY_HOOKS`, first bytes checked): a friend's appear shows its gauge,
  `CFriend`'s exit or dtor hides it; Re:CoM's HUD hide hides them. Nothing per frame.

### Owner questions (H6)

1. **What the gauges show** (Re:CoM's friends have no HP / MP): built full rings (HP 54 / MP 9:
   the whole ring, no straight part). (a) keep; (b) KH1's party stats for Sora's level; (c) the
   friend card's value as HP.
2. Only while the friend is out (decision 39); with Combat = KH1 there are no friend cards
   (decision 4), so they show only at Combat = Re:CoM with `hud = kh1`.

### Not done / to check

1. KH1's partial rings and straight parts for the party (not needed while they are full).
2. Draw order of the party's pieces: Sora's (Assumption).
3. KH1's `g_help` / `g_lose` circles (a member called / down) are not used.

### What the in-game test must check (H6)

1. `hud = kh1`, Combat = Re:CoM, a battle with a Donald card: his gauge appears top-right at
   KH1's place when he appears, and goes when he leaves. The same for Goofy (68, 20 further).
2. Log: `kh1 hud: Donald out: KH1 gauge shown (H6)` / `gone: gauge hidden`.

## H7: KH1's lock-on cursor in place of Re:CoM's

### KH1 (Checked (decomp + files))

- `0x140274B20`: lock state 3 (a target locked) with sub-state 0 draws `lock` object 2
  (`lockon_c`, picture 10, 38 × 38), sub-states 1 / 2 objects 1 / 0, states 1, 2, 4 object 3
  (`lockon_d`, the orange auto-target ring); centred on the target's projected lock point
  (`0x1402B04C0`, `0x140277510`): the quad at (−w / 2 + kx, −h / 2 + ky), (w, h) from its
  position keys (−4, −4, 46, 46 → 2, 2, 34, 34 → 0, 0, 38, 38, 4 + 4 frames) with its colour keys
  (alpha 0 → 255 → 192), restarted when the kind changes. Not scaled with distance.

### Re:CoM (Checked (decomp + exe))

- `Gauge::CLockonMarker` (ctor `0x140206DF0`, init `0x140208120`): its control `+0x60` (made
  with `0x1402471D0(ctrl, 2)`: a projected control) plays `Cockpit:00` group `+0x80` (0x63,
  0x92 with `+0xF8 & 0x10`, 0x62 with `+0xE8`); its update `0x140209D30` anchors it each frame at
  the lock point `+0x90` (`0x140248570`) and scales it with the distance (1.0 under 3.5 m, 0.5
  past 14 m). Target handle `+0xD0`, found `+0xEC`.

### Native build

- **Data:** picture 10 joins the atlas; one played group: `lockon_c` baked per frame (frame k =
  key time (k + 1) / 2) then held. `kh1-hud.json` `"cursor": group`.
- **DLL:** a wrapper on `0x140209D30` (`sites::KH1_HUD_CURSOR_HOOKS`, `40 57 48 81 EC 80 00 00
  00`): after Re:CoM's update at `hud = kh1`, Re:CoM's cursor layer is set not drawn
  (`0x140247E50(ctrl, 0, 0)` when `0x140248090` says it is), and our projected control
  (`0x1402471D0(c, 2)`, priority 2) gets the same lock point each frame (`0x140248570`); the
  cursor group plays from its start when a lock begins or moves to another target. At `hud =
  recom` Re:CoM's update runs alone (one lock and two loads of ours).

### Not 1:1 / owner questions (H7)

1. Only the locked cursor (`lockon_c`) is used: Re:CoM's marker groups 0x92 / 0x62 (other states
   of its marker) also show `lockon_c`; KH1's auto-target ring (`lockon_d`) has no Re:CoM event
   wired. Owner question: (a) keep; (b) map 0x92 / 0x62 to KH1's other kinds once their meaning
   is read.
2. Per frame while a battle runs: the wrapper copies the lock point (Re:CoM projects and draws).
3. Assumption: Re:CoM's marker layer-add flag 1 and priority 2 suit a projected layer of ours.

### What the in-game test must check (H7)

1. Lock on: KH1's white ring pops in (shrinks from 46 to 34, settles at 38 over 8 frames) on the
   target, the same size near and far; Re:CoM's marker is gone. Switching targets replays it.
2. `hud = recom`: Re:CoM's marker as before.

## H3: Re:CoM's HUD hidden at `hud = kh1` (visual only)

### What owns what (Checked (decomp + exe: RTTI))

| piece | object | how it hides here | its logic |
|---|---|---|---|
| HP gauge (ring, bar, drain, low HP) | `CPlayerGauge` `+0x78` (`gauge:01`, `gauge:00`) | colour alpha 0 | runs: HP drain, Re:CoM's low-HP sound 9 (≤ 25 %, `0x14020A0E0` case 4) |
| FRIENDS row (label, friend cards) | `CFriendGauge` (vtable `0x1405A6788`, `+0x60`): 3 controls `+0x78`, `C2DCard`s `+0x70` parented to them with `+0x138` = 1 | colour alpha 0 on the 3 (the cards take it) | runs |
| EXP + / NEXT LV / TOTAL MOOGLE P popups and their numbers | `CPrizeGauge` (vtable `0x1405A6718`, `+0x70`, global `[0x14087B690]`): controls `+0x70` ×6, `+0x88` ×6, `+0xA0` ×2, `C2DCard` `+0xA8`; `Cockpit:00` groups 0x51, 0x54, 0x69 | colour alpha 0 | runs (the battle end waits on it) |
| enemy HP gauges | `CEnemyGauge` (vtable `0x1405A66A8`), update `0x1402099B0` | its own hide path: mask bit 0x10 (toggles its layers) | — |
| minimap (+ the area layout it plays, the room plate: Assumption) | `CNaviMap` (vtable `0x14059F2A0`), 3D pieces (`+0x58`, lights) | its own hide path: mask bit 8 (`0x1401E4700` ×3, `0x1401E43E0`) | — |
| kept: "Guard" / "Recover" / "Resist" / "LEVEL UP!" | `CCommonGauge` (vtable `0x1405A66E0`, 7 controls `+0x60`: `Cockpit:00` 0xC, 0xD, 0x8F, 0x3D) | not touched (not on the owner's list) | |
| kept: the level-up choice, the command menu, the camp | | not touched | |

- **Why not the mask for all:** with the mask set, `CPrizeGauge`'s update (`0x14020A710`) drops
  its popup state (`+0x5C` = 0) and `CPlayerGauge` goes to state 7, which stops Re:CoM's low-HP
  sound (`0x1402AB700(…, 9)`). So those three are hidden by colour only. Checked (decomp).
- **Colour:** `0x140248590(ctrl, rgba)` writes `+0x120`; with `+0x138` = 1 Re:CoM's sprite update
  multiplies it in, with the parent's (7.3's notes). The gauge classes never write a control's
  colour (no call of `0x140248590`, no write to `+0x120` in their code); they set `+0x138` = 1
  themselves on some. Checked (decomp). The arrays' counts are checked against the count word
  Re:CoM's array new puts before them.
- **Mask reads** (`sites::KH1_HUD_MASK_CALLS`, contexts `KH1_HUD_MASK_CHECKED`): the five
  `call 0x1401B08C0` go to two stubs (near the exe) that jump to our getters: Re:CoM's value,
  plus the bit while the KH1 HUD is up (an atomic set on the gauge's init and the Config / camp
  refresh). At `hud = recom` the getters return Re:CoM's value unchanged and nothing of Re:CoM's
  controls is written.
- **When:** at each player gauge's init (after Re:CoM's) and after the Config / camp refresh.

### Not done / to check

1. The room plate is Assumed to be `CNaviMap`'s area layout (`0x140245940` at its `+0x114`,
   stopped on bit 8). If it still shows: screenshot, and its owner is searched next.
2. Re:CoM's lock-on cursor stays until KH1's (H7).
3. A `CFriendGauge` / `CPrizeGauge` made later by other code (their alternative ctors
   `0x140206D90` / `0x140206C70` have no caller in the decomp) would show.

### What the first in-game test must check (H3)

1. Log: `kh1 hud: enemy-gauge and minimap mask reads (5) through our getters at …`.
2. **Screenshot** at `hud = kh1` in the field and in a battle: no Re:CoM HP gauge, FRIENDS row,
   EXP / Moogle points, minimap, room plate, enemy gauges; the KH1 cluster and the command menu
   are there; "Guard" / "Recover" popups still show.
3. Win a battle: it ends as before (EXP and prizes counted: the camp shows the new EXP).
4. Low HP: Re:CoM's low-HP sound plays with the KH1 ring.
5. Level up: Re:CoM's level-up choice appears and works.
6. `hud = recom`: everything of Re:CoM's is back as before.

## Logs to send (H1)

- `kh1 hud: kh1hud:00 (632 groups, PC copies ["FR", "GR", "IT", "SP", "UK"]) from …`
- `kh1 hud: Re:CoM registered kh1hud:00 with 632 groups (…); hud = …` on the first room.
