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

## Logs to send (H1)

- `kh1 hud: kh1hud:00 (632 groups, PC copies ["FR", "GR", "IT", "SP", "UK"]) from …`
- `kh1 hud: Re:CoM registered kh1hud:00 with 632 groups (…); hud = …` on the first room.
