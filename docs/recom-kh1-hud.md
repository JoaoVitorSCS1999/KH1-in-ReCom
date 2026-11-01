# The KH1 HUD option (decision 39)

Owner (2026-10-31): "Would we be able to instead have an option where the Re:CoM [HUD] is
replaced entirely by the KH1 hud? ... use the KH1 decomp to get exact positioning and graphics".
Details (decision 39): HP ring on Re:CoM's raw HP (KH1's rule), party gauges hidden until they
join, menu words in KH1's font converted into a Re:CoM font, field / battle command menu looks,
Re:CoM's enemy gauges hidden and KH1's lock-on cursor used, Re:CoM's low-HP sound, Re:CoM's
extras hidden for now (the owner places them later).

Stages: H0 dump (done, outside the repo) → **H1 the `kh1hud` set** → **H2 Sora's cluster** →
**H3 hiding Re:CoM's HUD** → H4 command menu look → H5 LOCK ON! + target gauge → H6 party gauges →
H7 lock-on cursor → H8 popups.

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

### Not 1:1 yet (to ask / later)

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
