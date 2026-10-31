# M7 stage 7.2: the MP bar (decision 34)

Owner (2026-10-31): "MP bar: Re:CoM-style straight bar under Re:CoM's HP bar, in KH1 blue (built
like Riku's Dark gauge)"; "MP bar grows with max MP (KH1's 26 units per MP); colour read from the
player's KH1 files at the asset step".

At Combat = KH1, in Sora's story, field and battle: a straight bar under Re:CoM's HP bar, where
Riku's Dark gauge sits in Riku's story. It has one 24-unit slot per max-MP point, 26 units apart
(KH1's straight segments), filled in KH1's MP blue up to Sora's MP, with KH1's orange charge
line along its top, and KH1's change fades. Re:CoM draws it: the sprites and animations are
Re:CoM data the asset step adds to Re:CoM's own HUD file, and Re:CoM's 2D code plays them on a
layer object of Re:CoM's player gauge. At Combat = Re:CoM, or in Riku's story, nothing is made
and Re:CoM's HUD (the Dark gauge included) is untouched.

Code: `crates/kh1-recom-assets/src/mp_bar.rs` (the data), `crates/kh1-recom/src/mp_bar.rs`
(frames and fades, tested), `crates/kh1-recom/src/win/mp_bar.rs` (the wrappers), sites in
`sites.rs` (`MP_BAR_*`). Research: `m7-menus-mp.md` § 1.6, § 2.1, § 4.1, § 7 (kept with the decomp
tools, not in the repo).

Steam addresses (image base 0x140000000), both exes. Tags: **Checked (decomp)**, **Checked
(exe)** = bytes / disassembly of the Steam exe, **Checked (files)** = the owner's files (KH1
`command2/uitex.bin`, Re:CoM `FO0002.SPR` / `.RTM` from the OpenKH extract), **Assumption**.

## KH1's MP gauge

| what | KH1 (Steam) | tag |
|---|---|---|
| MP: the ring up to min(MP × 30, 270)°, then one straight segment per MP past 9, `0x1A` = 26 units apart, at most 20 (`0x14`). | `0x14026C6E0` | Checked (decomp) |
| Charge: a sweep along the same path from 0: c units of the ring (c ≤ 270), then 26-unit segments per 30 units, the last partial ((c − 270) mod 30) / 30, capped at 600 (`0x258`). So charge c sits at c / 30 MP slots, and point k is made (`0x1402A4CB0`) when the sweep reaches the end of slot k. | `0x14026A770`, `0x14026A970` | Checked (decomp) |
| Draw order: MP, then the charge over it, then the max-MP background pieces (ring only, past 3 / 6 max MP). | `0x14026CF90`, `0x14026DB00` | Checked (decomp) |
| Changes fade on the ring part only (≤ 270 units): `0x14027F760(fade, start, ±12.8, …, 128)`, stepped by the frame delta in `0x14027F780` the same frame. MP: the changed arc's RGB goes 0 → 128 (gain) or 128 → 0 (loss), alpha stays 0x80; charge: its alpha does. Drawn values 12.8 … 115.2 (or 115.2 … 12.8), done at the 10th frame (the fade then draws nothing and the main arc takes the new value). A new change restarts the fade from the last drawn value. Straight segments (past 9 MP / 270 units) never fade. | `0x14026C6E0`, `0x14026A770` | Checked (decomp) |
| Colours come from the texture: vertex colour 0x80 (×1) for both. | `0x14026B790` callers | Checked (decomp) |
| The gauge texture is `command2/uitex.bin` picture 5 (offset 5 × 0x2080 = 0xA280, 128 × 128, 8-bit); the background, picture 7 (0x10740). | `0x140266580`-area loader, `_DAT_1427F1520` / `_DAT_1427F14C0` | Checked (decomp + files) |
| In picture 5: a blue 24 × 12 strip at (30, 102), rows `(45,137,224)` → `(71,65,187)`; an orange 24 × 4 strip at (30, 116), rows `(254,136,1)` → `(255,173,2)`; every row one colour. In picture 7: a 26 × 15 background piece at (8, 85): 2 black rows `(0,0,0)`, navy `(24,28,42)`, 2 black rows. | the owner's `uitex.bin` | Checked (files) |
| That those strips are the straight MP segment (gauge `+0x98`) and charge segment (`+0xA0`): their records are in `exchange/<lang>_uibin_g_s.bin` (not in this session). They are the only straight blue / orange strips, their width + 2 = the 26-unit pitch, and the rings beside them are the MP / charge arcs (`0x1405273B0 + 0x30 / + 0x40`: centres (56.5, 56.5) / (113.5, 56.5), radius 55). | | Assumption (strong) |

## Re:CoM's side

- **`Gauge::CPlayerGauge`** (0xB8 bytes, vtable `0x1405A6638`: slot 0 the deleting dtor
  `0x140207770`, slot 1 the update `0x14020A0E0`), made by the player's `stateInitial`
  `0x1401FA6E0` in field and battle; ctor `0x140206E50` → init `0x140208360`. Fields: `+0x78` the
  HP `C2DSeqCtrl` (`gauge:01` 0xE / 7 / 8 / 9, `gauge:00`), at `gauge.lad` group 1 (60, 60);
  `+0x80` Riku's Dark gauge (3 controls, Riku's story only, `+0x90 & 4`); `+0xA0` the x offset
  (85.0 at 16:9; each layer is added at its negative); `+0xAA` the display state (7 hidden by
  the HUD mask (1, 0xE2), 1 shown again). Checked (decomp).
- **Riku's Dark gauge** (`0x14020C190`): `C2DSeqCtrl` with priority `+0x16E` = 2, layers added
  with flag 1, parented to `+0x78` (`0x140248650`), `+0x138` = 1, frames by `0x140247C20`. Its
  fill strip is at (1, −10)…(42, 4) from the HP gauge's origin. Checked (decomp + files).
- **`D2::C2DSeqCtrl`** (0x170 bytes, vtable `0x140607340`): `+0x10` its transform; `+0x130` the
  parent's transform; `+0x138` take the parent's colour; `+0x144` up to 10 layer handles,
  `+0x16D` their count, `+0x16E` priority (a clear resets it to 0xFF). Layers are sequence
  instances in Re:CoM's 2D manager, which plays and draws them; a frame set is time = frame × 50
  (`0x140245870`). The world transform is the control's own × the parent's (`0x14024B`-area
  draw: `+0x120` → parent `+0x80`). The x offset given to a layer is added after the transform,
  in screen units (×16, `0x14024B910` → `+0x13C`). Checked (decomp).
- **Sprite sets**: Re:CoM's 2D registry (`[0x14087CE48] + 8`, next `+8`, name `+0x14`) holds
  `Cockpit:00` = SEQD 0 of `Cockpit.lad` in `FORM/0002/FO0002.SPR`; a `C2DSeq` (vtable
  `0x140607260`) keeps its animation-group count at `+0xF4` (`0x140246DFE`), and a layer whose
  group index is at or past it is not made (`0x140246A6B`). Checked (decomp + exe). `Cockpit:00`
  is in use by the player gauge's own `CCommonGauge` in field and battle (research § 7.3).
- **Colour**: Re:CoM multiplies colours as `a × b >> 7` (0x80 = ×1). `Cockpit.imz` (in
  `FO0002.RTM`) has an opaque grey block (128, 128, 128) at (253..255, 45..), which Re:CoM's own
  tinted boxes sample at UV (254, 47)–(255, 48) (`Cockpit:00` sprites 15–18, 42–44) with vertex
  colours. So vertex colour V on that texel draws V × 128 / 128 = V. Checked (files + decomp).
  That the PC renders sprites as texel × vertex / 128 on screen, as on PS2: Assumption.

## The data (asset step)

Added to `Cockpit:00` after Re:CoM's 151 animation groups (the step refuses any other count).
All parts are relative to the HP gauge's origin, the bar starts at `X0` = 1, `Y0` = −10 (the
Dark gauge fill's left and top edge):

| new | what |
|---|---|
| 4 sprites | the neutral texel's UV with vertex colours: black (KH1's border), navy (KH1's background), MP blue (top row → bottom row, TL / TR / BL / BR), charge orange. Vertex = KH1 texel × 128 / Re:CoM texel, read from both files. |
| 5 parts | border (−1, −12)…(25, 4); track fill, MP, black (0, −10)…(24, 2); charge (0, −10)…(24, −6). |
| group 151, track | 20 static pieces (border + fill), slot i at x = 1 + 26i, shown from frame 30(i+1) to 600. |
| 152, MP | 20 static MP pieces, the same frames. |
| 153, charge | slot i: X scale 0 → 1 over frames 30i…30i+30 (KH1's partial segment), then a full piece from 30i+31. |
| 154, MP gain | black over one slot, alpha 115 → 13 over frames 0…8, then 0 (frames 9…10). |
| 155, MP loss | one MP slot, RGB 115 → 13 over 0…8, then transparent. |
| 156, charge gain | one charge slot, alpha 13 → 115 over 0…8, then full from frame 9 on (held). |
| 157, charge loss | alpha 115 → 13 over 0…8, then transparent. |
| 158, charge past the ring | one charge slot, full, held. |

The animation records copy their flags from Re:CoM animations doing the same thing: 0x77FF1
(`gauge:01`'s static HP pieces), 0x77DB1 (the HP bar's straight part, X scale 0 → 1), 0x77B71
(`Cockpit:00`'s colour fades); groups play once, group word +6 copied from `gauge:01` 0x12 /
`Cockpit:00` 0x38 (not read by Re:CoM's instance set-up `0x140248F40`). Checked (files + decomp).
Re:CoM's own records are unchanged (Checked (files): the step's reader writes Re:CoM's
`FO0002.SPR`, every `.lad` and every SEQD back byte for byte; after the patch the first 134
sprites, 196 parts, 111 sprite groups, 640 animations and 151 groups are identical and
`gauge.lad` / `layout_card.lad` are unchanged).

Frame units are KH1's charge units: slot i is 30 units, so the MP layer's frame is 30 × MP, the
track's 30 × max MP, the charge layer's the charge (all at most 600).

## Runtime (DLL)

- **Wrappers** (first bytes checked, `MP_BAR_HOOKS`):

| site | bytes moved | what ours does |
|---|---|---|
| `CPlayerGauge` init `0x140208360` | `48 8B C4 56 57 41 56 48 83 EC 70` | after Re:CoM's: at Combat = KH1 in Sora's story, with `Cockpit:00` holding 159 groups, makes the bar: a `C2DSeqCtrl` (Re:CoM's `operator new` `0x1404ACE54` + ctor `0x140247260`), priority 2, layers groups 151 / 152 / 153 at x offset −`+0xA0` with flag 1, parent `+0x78`, `+0x138` = 1, frames from `mp::current()`; hidden if `+0xAA` = 7. |
| second-bar state `0x14020C190` | `40 55 57 48 83 EC 48` | after Re:CoM's (the Dark gauge): state 7 hides the bar (`0x140247EC0(ctrl, 0)`) and drops running fades; state 1 shows it and rescrubs. |
| deleting dtor `0x140207770` | `40 57 48 83 EC 30` | deletes our controls (vtable slot 0 `0x14015D230(ctrl, 1)`, as Re:CoM deletes `+0x78`), then Re:CoM's. |

- **Callees** (checked, `MP_BAR_CALLEES`): `0x140247C20` frame, `0x140247C90` clear,
  `0x140247EC0` all layers visible, `0x140248650` parent, `0x1402486D0` scale (16-byte aligned
  argument), `0x140247850` add layer (vtable +8), `0x14015D230` deleting dtor (vtable +0), and
  the already checked `0x1404ACE54`, `0x140247260`, `0x1402485B0` (position). Vtable slots
  checked at install (`MP_BAR_PTRS`): `0x140607340` → `0x14015D230`, `0x140607348` →
  `0x140247850`, `0x140607270` → `0x140246A40`. Re:CoM code relied on, checked:
  `0x140245A20` (registry walk), `0x140245A80` (name at +0x14), `0x140246DFE` (+0xF4),
  `0x140246A40` / `0x140246A6B` (the group bound).
- **When it runs**: never per frame. `win/mp.rs`'s every MP read or write ends with
  `mp_bar::sync(view)`: one lock and a compare; when (MP, max, charge) differs from what the bar
  shows, the frames are set and the fades placed. MP changes come from KH1 hits (the enemy-hit
  wrapper), the refill step (only while a point is waiting), spells later. Max MP changes
  (level-up, Keyblade) show at the next read: the camp's close (`0x1401A9590` wrapper,
  `win/models.rs`), the HUD showing again, or any MP event. The Config row (Combat) and the
  camp close also make or remove bars on the live player gauges.
- **Fades** (`crate::mp_bar::plan`), KH1's rules on one straight bar (only the first 270 units,
  KH1's ring part):
  - MP gain: the MP layer shows the new value; a black piece per gained slot fades out (screen
    = colour × t, KH1's rising RGB);
  - MP loss: the MP layer shows the new value; a piece per lost slot goes to black, then goes;
  - charge gain: the charge layer stays at the old value; a piece per slot touched (X-scaled for
    partial slots) fades in and stays; past 270 units the pieces are there at once;
  - charge loss: the charge layer shows the new value; the lost part fades out;
  - a new change of the same kind drops the running fade (KH1 restarts from the last drawn
    value); a change of the other kind leaves it.
  Each piece is a small `C2DSeqCtrl` (pooled per bar, parented like the bar), placed with
  `0x1402485B0` and X-scaled with `0x1402486D0`; its group plays over time (flag 0).
- **Logs** (60 lines at most): `mp bar: KH1 MP bar on Re:CoM's player gauge … groups 151..158
  from …`, `mp bar: built on the player gauge …: MP 3/3 c0, x offset -85`, each fade
  `mp bar: 1/3 c60 -> 2/3 c0: frames [Some(90), Some(60), Some(0)], fade pieces MP 1, charge 2`,
  and once `mp bar: Cockpit:00 has 151 groups, not 159 (…): no MP bar` when the served file isn't
  ours.

## The asset step

`kh1-recom-build` (the same run as the other assets) reads KH1's `command2/uitex.bin` (from the
`kh1_*` archives; prints its PC asset names, if any), Re:CoM's `FORM/0002/FO0002.RTM` (the
neutral texel) and the whole `FORM/0002/FO0002.SPR` PKG entry with its PC assets, adds the
groups to `Cockpit.lad` in the file and in every asset that is a `Cockpit.lad` (any language
prefix) or a whole container, keeps the other assets as they are, and writes:

- `<mod_path>/recom/raw/FORM/0002/FO0002.SPR`: a plain PKG entry (the patched file, then each
  asset with its name, tag and the original date), served by Panacea's raw route like the
  Keyblade RTMs (`recom-keyblade-models.md` § 8);
- `<mod_path>/recom/kh1-mp-bar.json` (format `kh1-recom-assets mp-bar 1`): set, base and first
  group, slot geometry, the KH1 and vertex colours, which files were patched, the inputs'
  sha256. Without it the DLL makes no bar. A run that cannot build the bar removes an older one.

`--dry-run` reports without writing. Command and install: `kh1-recom-assets.md` § The MP bar.

## 1:1 checks

- Checked (decomp): the slot pitch 26, the 20-slot and 600-unit caps, 30 charge units per slot,
  the charge's partial last slot, the ring-only fades, their 12.8-per-frame steps and 10-frame
  length, MP by colour and charge by alpha, the restart rule, the draw order.
- Checked (files): KH1's colours and sizes (24 × 12 MP, 24 × 4 charge, 2-row border) read from
  the owner's `uitex.bin`; Re:CoM's neutral texel 128; Re:CoM's file layout round trip.
- Checked (exe): every new site's bytes and the three vtable slots, against the Steam exe.
- Tests: `kh1-recom` `mp_bar` (frames, fades, partial slots, the ring limit, manifest) and
  `sites::mp_bar_sites`; `kh1-recom-assets` `mp_bar` (container / LAYD / SEQD round trip, the
  groups and their frames, KH1 strip reading, the PS2 swizzle, vertex colours; with
  `KH1_RECOM_FORM` set, the owner's three files).
- Not checked: nothing seen in game; no KH1 recording of the gauge (a KH1 video of a refill and
  a Fire cast would confirm the fades frame by frame).
- Not 1:1 by design (decision 34): the shape is Re:CoM's straight bar, not KH1's ring; the
  first 9 slots fade as KH1's ring part does. KH1's partial charge segment on its straight part
  shows the slot's far end (it grows from the left edge of a leftward slot); ours grows along
  the bar like Re:CoM's bars. KH1 has no max-MP track past its ring: ours shows one slot of
  KH1's background colours per max-MP point (the owner's "grows with max MP").

## Assumptions

1. The PC draws Re:CoM sprites as texel × vertex / 128 and KH1's as texel × 0x80 / 128: KH1's
   on-screen MP blue = its texel. If the bar looks twice too bright or dark, the vertex colours
   are off by the PC's colour scale.
2. KH1 PC shows the PS2 picture's colours (its HD assets for `uitex.bin`, if any, keep them).
3. Sprite vertex order (TL, TR, BL, BR): from Re:CoM's digit sprites (`gauge:01` 23–32: top
   white, bottom grey). If the gradient runs left → right in game, the order is (TL, BL, TR, BR).
4. Draw order: our pieces (made later) draw over the bar's layers, same priority 2. So an MP fade
   piece also covers the charge line under it for 10 frames (KH1 draws the charge on top).
5. A time-played group holds its last frame when it ends (play once), as `Cockpit:00` 0x36's
   fade-in must; the charge-gain pieces don't rely on it (held to frame 10,000,000).
6. The first frame Re:CoM draws a new layer at time 0 (so our fades show KH1's 115.2 … 12.8 from
   the first frame); else they are one frame late.
7. Panacea's raw route serves `raw/FORM/0002/FO0002.SPR` with its assets for an existing name,
   and the language copy the game loads is among the assets the step patched. The DLL checks
   the loaded `Cockpit:00`'s group count and logs it when not.
8. Placement: (1, −10) from the HP gauge's origin, i.e. where the Dark gauge's fill is: under the
   HP bar's straight part. Until a screenshot.

## Open questions (owner)

1. **Track look past MP:** (a) KH1's background colours (navy, black border), one slot per max
   MP (built); (b) Re:CoM's HP-bar background sprite stretched to 26 × max; (c) no track (only
   the filled slots show, as KH1's straight part).
2. **Charge partial growth on the straight bar:** (a) along the bar, left to right (built);
   (b) KH1's literal straight-part rule (each partial slot shows its far part).
3. **If the colour is off on screen:** (a) halve / double the vertex colours (the PC's scale),
   (b) read KH1's HD texture instead, (c) the owner picks the value from a screenshot.

## What the first in-game test must check

1. Log: `mp bar: KH1 MP bar on Re:CoM's player gauge … groups 151..158`, then on entering a room
   `mp bar: built on the player gauge …: MP 3/3 c0, x offset -85` (no `has 151 groups` line).
2. **Screenshot** at level 1 in the field, full MP: three blue slots with gaps under the HP
   bar's straight part; the blue lighter at the top, darker at the bottom; KH1's MP blue.
3. Swing at a Shadow at full MP: an orange line grows along the top of the slots (3 units of 30
   per swing hit, at the Shadow's level), up to the end of the last slot (KH1 banks max × 30).
4. After at least one hit with the Kingdom Key (so the save block holds MP 3), equip Diamond
   Dust (+3 max MP) in the camp: when it closes the track grows to 6 slots, 3 filled (KH1 keeps
   the record's MP). Swing: when the line reaches the end of slot 4 (120
   units), slot 4 rises from black over 1/6 s and the line's lost part fades out.
5. Level 12 (4 max MP): the track has 4 slots.
6. Events / cutscenes that hide the HP gauge hide the bar too; it comes back with it.
7. A battle, then back to the field: the bar is there in both.
8. Combat = Re:CoM in the Config: the bar goes when the camp closes; back to KH1: it comes back.
9. Riku's story: no bar; the Dark gauge works as in Re:CoM.

## Language copies (0.16.1)

- Running the asset step on the owner's files (2026-10-31) showed that `FO0002.SPR`'s language
  copies `FR_ / GR_ / IT_ / SP_ / UK_Cockpit.lad` have **157** animation groups of their own, the
  base file 151. Checked (files). 0.16.0 refused them and built no bar.
- Now each copy gets the bar's 8 groups after its own (151..158 in the base file, 157..164 in the
  copies); `kh1-mp-bar.json` lists every first group (`first_groups`), and the DLL picks ours by
  the loaded set's group count (`Layout::first_for`: 159 → 151, 165 → 157). A set that already
  ends with the bar's sprites is refused (fed-back output).
- The owner's mod was built with it on their PC: `raw/FORM/0002/FO0002.SPR` and
  `kh1-mp-bar.json` in the mod folder and `mod.yml`.

## The command menu's groups (M7 stage 7.3)

- The same patch now adds the command menu's two plate groups after the bar's 8 (159, 160 in the
  base file; 165, 166 in the language copies), and `kh1-mp-bar.json` says so (`"extra_groups":
  2`). The DLL expects the loaded `Cockpit:00` to have `first + 8 + extra` groups (161 / 167);
  a 7.2 manifest (no key) keeps 159 / 165. A 0.16.1 DLL with the new files finds 161 and makes
  no bar: the DLL and the asset step's files go together. `docs/recom-m7-command-menu.md`.

## HD atlas in the language copies (0.17.1)

- Dumping the owner's `FO0002.SPR` (2026-10-31): the language copies of `Cockpit.lad` are the PC's
  HD layouts; every sprite UV is the base file's × 2 (group 0's [254, 49, 255, 50] →
  [508, 98, 510, 100]; the text plate's cap [229, 110, 255, 135] → [458, 220, 510, 270]), and the
  group numbering below 0x28 is the same. Checked (files).
- The asset step now reads each set's atlas scale (`mp_bar::uv_scale`) and writes the bar's and
  the plates' sprites at that scale, and finds the text plate in every copy. Assumption: the HD
  atlas's texel at the doubled neutral UV is the same grey (the bar's colours are computed from the
  PS2 texel 128); if the bar looks twice too dark or bright on screen, that is why.
- Rebuilt on the owner's PC; `kh1-command-menu.json` added to the mod and `mod.yml`.
