# M12 camera: a Config option Re:CoM / KH1 / KH2 camera (research and plan)

Owner, 2026-11-04, verbatim: "Lets also have an option in the config to switch between Re:CoM
camera, KH1 Camera, And KH2 camera." (`design-philosophy-instructions.md` § Shop text, lock-on
and camera: "Research both KH1's and KH2's cameras (decomps) and Re:CoM's camera system first;
plan with the owner before building.") Lock-on on R1 at Combat = KH1 is another agent's work;
this plan only covers what the camera does while a lock is on.

**Status:** the owner answered § 10 (`design-philosophy-instructions.md` § Camera answers) and
asked for the five stages. Built: stage 1 (0.134.0, § 13), stage 2 (0.135.0, § 14), stage 3 (0.136.0, § 15),
stage 4 (0.138.0, § 16).

Steam addresses for all three games. Tags: **Checked (exe / decomp / files)** = read;
**Assumption** = inferred, to check. KH1 roles marked "(Epic notes)" come from
KH2-Combat-in-KH1's `docs/kh1-camera.md` (Epic build); the Steam function named next to them was
found in the Steam decomp at the same place (same caller, same call order, same constants) and
read there, unless the row says otherwise.

Units: Re:CoM metres, y up; KH1 centimetres, y down (`crate::units`: a KH1 point (x, y, z) is
Re:CoM (x, −y, −z) / 100; KH1 yaw ψ is Re:CoM yaw π − ψ). KH2 is also y down (its camera
subtracts 170 for "above the feet", below); that KH2 units are centimetres like KH1's is an
**Assumption** (same engine family; KH2-Combat-in-KH1 treats them so).

---

## 1. Summary

- **Re:CoM** has one player camera object, `PlayerCamera::CPlayerCamera`, a small state machine
  (field state, battle state) with a **mode number** and a **record per mode** (FOV, collision
  radius, ease rate, eye and aim offsets) read from the player's `PCamData` file
  (`PLAYER/0001/PL0001.BIN`). Field rooms run mode 1 (a follow camera that is dragged behind
  Sora, no auto-turn), battle rooms mode 3 (the same follow with closer numbers) and mode 4 when
  locked on. Sleights, summons and specials switch it through one request function (36
  functions, 82 call sites) and give it back with "mode −1". Events, cutscenes, battle intros and
  Sora's death are **other** camera objects pushed on Re:CoM's camera stack; they never touch
  the player camera's mode.
- **KH1** has a camera module with a mode table (mode 1 follow, 2 first person, 3 lock-on, 4–8
  movement-state cameras, 9 area cameras, …). Its follow camera: 4.0 m, 10° pitch, look point
  1.0 m up, FOV 53°, a stick with acceleration (2.5°/frame max), an R3 recenter, and in "Auto"
  a swing-behind, a wall-avoid and a pitch assist.
- **KH2** has a camera with a parameter table (distance 4.2 m, range 2.5–5.0 m, no pitch: the
  height follows a curve of the distance), a digital right stick (3°/frame turn, zoom), a
  swing-behind while running, and a lock-on camera that only turns when Sora or the target
  leaves the screen.
- **Native design:** four new modes in Re:CoM's own `CPlayerCamera` (6 KH1 follow, 7 KH1
  lock-on, 8 KH2 follow, 9 KH2 lock-on), each with a record in Re:CoM's own record format, run by
  Re:CoM's own state machine, collision object, view output and sleight hand-over. Sleights keep
  asking for Re:CoM's modes 3 / 4 / 5 and give the camera back with Re:CoM's own "previous mode"
  slot, which then holds ours.
- **Option:** a new **Camera** page in the camp's Settings list (Re:CoM / KH1 / KH2), independent
  of Combat (owner question).

---

## 2. Re:CoM's camera (Checked unless tagged)

### 2.1 Objects, the camera stack, who renders

| What | Address | Tag |
|---|---|---|
| `CCamera` base ctor (11 callers): registers the camera with priority `+0x80` (−1 → 0x80) | `0x14024F5B0` | Checked (decomp) |
| Camera stack: 16 slots at `0x140AC4370`, sorted by priority, a new camera goes in front of those with lower **or equal** priority; slot 0 is the one drawn | insert `0x14028F270` | Checked (decomp); "slot 0 is drawn": Assumption (not read where the view is taken) |
| FOV setter: `+0x84 = 256 · cot(fov / 2) − 512` (PS2-style, 512-wide frame) | `0x14024FCB0` | Checked (decomp) |
| View write from eye / aim (look-at matrix into the camera's node) | `0x14024FA40` | Checked (decomp) |
| `.cam` path cameras (`CPathCamera` family): Sora's death `PDead000..002`, `cut001`, `world_cr_0x`, Riku's `riku_cardget02`, event ones; each makes its own `CCamera` at priority 0x80 | `0x140283F10` (10 callers) | Checked (decomp) |
| Event cameras `CAtelCamera` (ATEL event system) | ctor near `0x140135E20` | Checked (RTTI + decomp); its priority: Assumption (0x80 like the others) |
| Minigame cameras (`CMini100Acre*Camera`) | — | Checked (RTTI); not read |

So cutscenes, event cameras, battle intro `.cam` paths and Sora's death camera are separate
cameras that sit in front of the player camera while they run. The player camera keeps running
underneath (Assumption: its update is not paused; it is not drawn).

### 2.2 `CPlayerCamera`

| What | Address | Tag |
|---|---|---|
| ctor (callers: the player build `0x1401F42B0` (field and battle), `0x1401FA6E0`); global pointer `0x14087B460`; the player keeps it at `player+0x3D8` | `0x1401FB140` | Checked (decomp) |
| init: field / battle state records (field pre `0x1401FCE80`, field exec `0x1401FEEB0`; battle pre `0x1401FCE50`, battle exec `0x1401FECE0`), inversion, pitch limits, base mode (field 1, battle 3) | `0x1401FB530` (`4C 8B DC 49 89 5B 08 …`) | Checked (decomp + disasm) |
| The two exec functions are referenced **only** by the init's two `lea`s, at `0x1401FB556` and `0x1401FB587` | — | Checked (exe xref scan) |
| Per-frame update (vtable): inversion re-read, state exec, fade timer | `0x1401FCCF0` | Checked (decomp) |
| Mode set with re-placement (eye / aim rebuilt behind the subject) | `0x1401FC4D0(cam, mode, place, angles)` | Checked (decomp) |
| Mode set keeping the camera where it is | `0x1401FBE60(cam, mode)` | Checked (decomp) |
| **External request** (sleights, actions): mode ≥ 0 saves the current mode in `+0x17C` (once, bit `+0x188 & 0x2000`) and sets it; **−1 gives back `+0x17C`** | `0x1401FBDE0(cam, mode, place)` (36 functions, 82 sites) | Checked (decomp) |
| Subject / target override by actor handle (`+0x2B4` subject, `+0x2C0` look target) | `0x1401FBF80` (31 functions) | Checked (decomp) |
| Sleight record override: modes 3 / 4 read a sleight's own records (exe tables such as `0x1407C7C20`, `0x1407C58E0`, `0x1407C7CE0`…) until `(0, 0)` restores | `0x1401FEB50` (18 functions) | Checked (decomp + exe) |
| Boss battle camera: a boss's pack may hold `BCam0000`; enemy init for ids ≥ 100 turns it on and it replaces the mode 3 / 4 records | `0x1401FEC50` (callers `0x1401805A0`, `0x1403DD360`) | Checked (decomp) |
| Mode → record table: 6 slots at `0x14087B660` (slot 0 empty, 1..5 = `PCamData` + 0x30·(n−1)); the current record pointer is `+0x170`, the mode `+0x178` | — | Checked (decomp) |

**State fields** (Checked (decomp)): aim `+0x220`, eye `+0x230`, aim goal `+0x240`, eye goal
`+0x250`, pitch `+0x260`, yaw `+0x264`, geometric angles `+0x270`, eye − aim at rest `+0x290`,
distance `+0x2A0`, flags `+0x188` (bit 0 input locked, 2 / 8 / 0x800 yaw kept within ±60° /
±90° / ±30° of Sora's back during some actions, 0x80 tilt-down after a close wall, 0x1000 fixed
overhead, 0x2000 external mode active), camera collision object `+0x190` (`CCollMapCamera`),
inversion `+0x2F0` (horizontal) / `+0x2F4` (vertical), pitch limits `+0x2F8` / `+0x2FC`, extra
height `+0x304`, keyboard-mouse flag `+0x308`.

### 2.3 The mode records (`PCamData`, the owner's `PL0001.BIN`)

Record (0x30 bytes): `+0` FOV (degrees), `+4` collision radius ("size" in Re:CoM's debug print),
`+8` ease rate per frame, `+0xC` close-wall distance (follow) / lock gate (mode 4), `+0x10` eye
offset, `+0x20` aim offset. Checked (files: header 0x40, 5 records; decomp of the readers).

| Mode | Used for | FOV | radius | ease | +0xC | eye offset (m) | aim offset (m) |
|---|---|---|---|---|---|---|---|
| 1 | field follow | 50 | 0.05 | 0.12 | 0.8 | (0, 1.0, 4.6) | (0, 0.8, 0) |
| 2 | a timed free view (stick pans the eye) | 45 | 0.10 | 0.08 | 3.0 | (8, 4, 8) | (0, 0.9, 0) |
| 3 | battle follow | 55 | 0.05 | 0.20 | 0.7 | (0, 0.9, 4.0) | (0, 0.1, 0) |
| 4 | battle lock-on | 55 | 0.05 | 0.20 | 5.0 | (0, 1.0, 4.0) | (0, 0, 0) |
| 5 | close-up (sleights) | 55 | 0.05 | 0.20 | 0 | (0.2, 0.1, 1.2) | (0, 0.4, 0) |

Who enters mode 2: not found (no external request uses 2). Assumption: room code through the
mode setter. Not needed.

### 2.4 Per-frame update and the follow (modes 1 / 3)

Time: the camera's own `dt` (`+0x14`, 1/3000 s) × 0.02 = 1.0 per 60 Hz frame (Checked (decomp),
as `recom-timing.md`). All rates below are per 60 Hz frame.

- **Field exec** `0x1401FEEB0`: mode 1 → follow `0x1401FD750`; mode 2 → `0x1401FE5D0` (after
  2999 ticks, a fade and back to `+0x180`); else → `0x1401FDF70`. Then the common tail: camera
  collision `0x1401FC0D0` (sweep with radius `rec+4 × 0.1`), the eye written to the node, the
  view `0x14024FA40`. **Battle exec** `0x1401FECE0`: restores sleight records when their actor
  is gone; **mode 3 ↔ 4 by Re:CoM's lock-on handle `player+0x850`** (locked and not mode 5 → 4,
  unlocked → 3); mode 4 → lock step `0x1401FCF90`, else follow `0x1401FD750`; same tail.
  Checked (decomp).
- **Follow** `0x1401FD750` (Checked (decomp + disasm)):
  - Aim goal = Sora's camera point (player vtable `+0x68`) + the record's aim offset turned by
    Sora's facing; the aim and the eye ease to their goals at `rec+8` per frame (field 12 %,
    battle 20 %; **80 % when keyboard / mouse is in use**, `+0x308`).
  - **Right stick** (pad `+0x6C` / `+0x70`, ÷ 4096): dead zone 0.25 (0.025 with keyboard /
    mouse); yaw **3° per frame** × tilt × inversion; pitch **1.5° per frame** × tilt × inversion,
    clamped field −50°..+12°, battle −45°..+30° (−50° with a boss camera).
  - **No input:** the yaw eases toward the yaw the moved aim now has from the old eye (10 % a
    frame): the camera is **dragged** behind Sora like a string, it never swings behind him on
    its own. The pitch eases back to 0 at 2 % a frame while Sora is on the ground
    (`0x1401F3D80`; its meaning "airborne": Assumption).
  - Close wall: if the swept eye lands nearer than `rec+0xC` (0.8 m field) the camera tilts
    down 3°/frame for 30 frames.
  - Reset button: pad "pressed" bit `0x400` (`0x14029C240(0x400, 0)`) re-places the camera
    behind Sora at once (`0x1401FC4D0` with the current mode). Which button: **Assumption R3**
    (Re:CoM's pad word follows the SCE order in which L1 = 4 and Circle = 0x20,
    `recom-m7-command-menu.md`; R3 = 0x400 there).
- **Lock-on** `0x1401FCF90` (Checked (decomp)): aim = the midpoint of Sora's camera point and the
  lock target's camera point (`player+0x870`; an enemy's is its position + `CamTrgOfs` from its
  parameter file, `0x14017F770`); eye 4 m beyond on the line from the target through Sora, 1 m
  up; the stick still turns it (same speeds); with no input it eases back to that line.
- Other: `+0x304` adds 0.4 m to eye and aim in Riku's story outside a range of rooms
  (`0x1401AAAE0` = the story, `recom-keyblade-equip.md`; Checked (decomp); "Riku is taller":
  Assumption).

### 2.5 Input, options

- Inversion: Re:CoM's own **Game Settings rows 0 and 1** are the camera's: the camera reads
  save bytes `0x65` (vertical, `+0x2F4`) and `0x66` (horizontal, `+0x2F0`) through
  `0x1401B0800`, the same ids the Config screen edits as its rows 0 / 1 (`recom-config-menu.md`).
  Checked (decomp); their on-screen labels: Assumption (message files not read).
- Keyboard / mouse: `0x140C2FF64` (via `0x1403471C0`) is the "keyboard / mouse in use" flag (the
  button-glyph picker switches to key glyphs on it). Checked (decomp). That the mouse feeds the
  right-stick fields `+0x6C` / `+0x70`: Assumption (not read).
- No camera speed option, no auto / manual option. Checked (Config screen rows,
  `recom-config-menu.md`).

### 2.6 Where new modes live natively

- The mode is a plain number in the camera (`+0x178`) with a record pointer (`+0x170`); the
  execs dispatch on it; sleights save and restore it through `+0x17C`. New modes = new numbers
  whose records and steps Re:CoM's own state machine runs.
- The record table has only 6 slots (`0x14087B660`, read by `0x1401FC4D0` and `0x1401FBE60`):
  modes ≥ 6 need their record pointer given at those two reads.
- The two execs are reached only through the init's two `lea`s (2.2): pointing them at a
  dispatcher sends modes ≥ 6 to the new steps and every other mode to Re:CoM's exec unchanged.

---

## 3. KH1 FM's camera (Steam)

### 3.1 Module (Checked (decomp + exe) unless tagged)

| What | Address |
|---|---|
| Camera task (copies the internal pair to the public one, runs the mode-switch blend `1 − cos(t/T · π/2)`) | `0x1401E5D90` |
| Mode table (16 bytes per mode: update, exit): 1 `0x1401E1920` follow, 2 `0x1401E2390` first person, 3 `0x1401E2650` lock-on, 4 `0x1401E2DE0`, 5 `0x1401E2FB0`, 6 `0x1401E30E0`, 7 `0x1401E32B0`, 8 `0x1401E34F0`, 9 `0x1401E39B0`, 10 `0x1401E39F0`, 11 `0x1401E3B30` | `0x140506CE0` |
| Mode switch (blend frames) | `0x1401E5360` |
| Mode, public eye / target / FOV | `0x14299BB08`, `0x14299BAE0`, `0x14299BAF0`, `0x14299BB00` |
| Internal eye / target, pitch, yaw, distance | `0x142537E60`, `0x142537E70`, `0x142537E80`, `0x142537E84`, `0x142537E8C` |
| Built-in defaults: target offset (0, −100, −50) in Sora's frame; pitch 10°, distance 400 | `0x140506CA0`, `0x140506CB0` (copied by `0x1401E50D0` / its init) |
| Room's camera block (pitch `+0x10`, distance `+0x14`, target height `+0x18`, fixed / rail cameras and zones) | loader `0x1401E5110` (from the room load `0x1402888B0`) |
| Presets (pitch, distance, height): (−0.1 rad, 400, −120), (−0.15 rad, 480, −150) | `0x140506CC0` (`0x1401E4FE0`) |
| Inversion floats (written by the Config setter: up/down `2v − 1`, left/right `1 − 2v`) | `0x140506CD8`, `0x140506CDC` |
| Config block (`+0x8` field camera 0 / 1, `+0x14` up/down, `+0x18` left/right) | `*0x142E1F4D8`; `+0x8` read by `0x1402FB0D0`. Label "Auto = 0, Manual = 1": Assumption (Epic notes, med) |
| FOV setter (`256 / tan(fov/2)`); follow and first person set **53°** | `0x14028CAF0`; constant `0x1403E9E44` |
| Frame step (1.0 per 60 Hz frame on PC; several gains are halved when it is below 2.0) | `0x14233FBDC` (`/home/claude/re/notes/kh1-tick.md`) |
| Logical pad, pressed: R3 = `0x200`, Select = `0x8000` | `0x14298CA98` |
| Right stick x / y (−1..1, dead zone 60 of 127.5 removed, Epic notes) | `0x14298CAC0` / `0x14298CAC4` |
| Mouse mode / mouse sensitivity (KH1 PC settings, 0..40, default 20: Epic notes) | `0x140139590` / `0x140138E50` |

### 3.2 Follow camera, mode 1 (`0x1401E1920`) at 60 Hz (Checked (decomp))

- **Entry:** pitch, distance from the room's defaults, yaw from the current view, FOV 53°,
  eye re-placed behind that yaw with a wall check.
- **Look point** = Sora's matrix × target offset (1.0 m up, 0.5 m on his local −z: which side
  that is in Re:CoM's frame: Assumption, to settle at build). It eases to that point by
  **10 % of the horizontal gap per frame**, and vertically **25 % per frame going down, 5 %
  going up** (the 20 % / 50 % / 10 % of the code, halved because the frame step is 1.0 < 2.0).
  It snaps while a mode blend runs.
- **Stick** (`0x1401E7320`): angular velocities with the pad: acceleration **0.075°/frame²**
  × tilt (0.15° halved), maximum **2.5°/frame**, decay **0.12°/frame²** on release (0.24°
  halved); pitch clamped **−15°..+50°**; yaw unclamped; times the inversion floats; only while
  Sora can act, no blend and no recenter run. Mouse: acceleration 0.3°, decay 4°, maximum
  2.5° + 0.0875° × sensitivity, velocity zeroed when the mouse stops.
- **Recenter** (`0x1401E62A0`): R3 swings to behind Sora (facing + π) in **60 frames**, sin
  eased, pitch back to 10°, distance back to the default; it tests 30° steps for walls and turns
  the other way if blocked. In Auto it also starts by itself when the eye has been squeezed to
  within 40 units (horizontally) of the look point, the pitch is under 60° and Sora moves faster
  than his state's threshold (1.0 a frame standing, 7.0 in states 8 / 0x18, else 0.1). (The Epic
  notes said "stands still"; the Steam code tests "moving". Checked (decomp).)
- **Auto branch** (config `+0x8` = 0 and not movement state 0x1B):
  - swing-behind `0x1401E6FF0`: once Sora keeps one facing (changes under 30°) and moves faster
    than a per-state threshold, a ramp (60 frames, then a second to 90; with a mouse 0 and 40)
    and a third ramp (60, mouse 20) that rises while a 250-unit probe at the look point or the eye
    touches geometry (`0x140196300`) give a yaw gain of up to 2 % + 2 % of the angle to his back
    per frame, weaker past 120° and none at 180°;
  - wall-avoid `0x1401E6C10` and pitch assist `0x1401E6670` (roles: Epic notes; numbers not
    re-read on Steam: wall swing 40 frames, pitch back to the default at 3 % a step, ≤ 85°);
  - eye = look point + (distance, pitch, yaw); if it grew past default + 300 it is re-placed;
    otherwise it walks to its goal in ≤ **20-unit steps** with a **40-unit sphere sweep**
    (`0x140196560`; Epic `0x140194410`, matched by its call position and the radius 40.0),
    stopping on contact; a jump ≥ 1000 teleports; `0x1401E7780` re-places it behind if Sora
    can't be seen from it.
- **Manual branch:** eye = look point + offset; a ray (`0x140193790`) from the look point to 60
  past the eye clamps it; the look point is pushed away if closer than 20.
- **Exit:** in Auto the distance springs back 10 % a frame toward the default; yaw recomputed
  from eye / target; the movement-state camera switch `0x1401E5480` (modes 4, 5, 6, 8 with blend
  60).
- **Select** (movement states 0, 8, 0x18): first person (mode 2, blend 30). First person
  (`0x1401E2390`): left stick pitches −80°..+60° at 3°/frame, R3 or Select leaves.

### 3.3 Lock-on camera, mode 3 (`0x1401E2650`) (Checked (decomp + exe))

- Entered from the R1 lock (`0x1401E41A0`, blend **60**; a re-lock in mode 3 blends the look
  point back over 60 frames); left with blend **45** (`0x1401E4110`).
- An anchor 3.5 m behind Sora (`Ry(yaw) × (0, −100, 350)`, `0x140506E10`, plus a height assist
  of up to 1.7 m from his height over the floor); the lock point with its radius (`0x1402B02D0`
  on the lock pair `0x142DB3518`); the camera looks past the lock point (radius + 1 m beyond it).
- The eye closes on the anchor to within 75 units at **90 % per frame**, vertically 90 %;
  a ray from Sora's body point (`0x8003`) clamps it.
- The yaw turns toward the look direction with a **±5° dead band**, half the remaining error
  per frame, at most **10° per frame**.
- If Sora leaves the half-FOV, a counter rises to 40 frames and pulls the look point up to 80 %
  toward Sora; it falls 2 per frame once he is back within 80 % of the half-FOV.
- **No stick turning** in lock-on. Select still enters first person.

### 3.4 KH1's room camera data

KH1's rooms carry their own default pitch, distance and look height, fixed cameras, rails and
trigger zones (3.1). Re:CoM's rooms have none of KH1's. In Re:CoM the KH1 camera can only use
KH1's **built-in** defaults (10°, 4.0 m, 1.0 m up) or numbers taken from KH1 rooms by the asset
step (owner question Q8). KH1's map files are not in the session (Assumption: typical KH1 rooms
change the distance and pitch).

---

## 4. KH2 FM's camera (Steam)

| What | Address | Tag |
|---|---|---|
| Parameter rows, 6 floats each: row 0 (field and battle) **FOV 1.5 rad, default distance 420, lock distance 380, minimum 250, maximum 500, height coefficient 0.008**; rows 1–3: FOV field 0, 600, 520, 400, 700 (row 2: 633), 0.005 | `0x140718C00`, loader `0x140165A70` | Checked (exe + decomp) |
| Look point **170 above the feet** | `0x1401644E0` and six more | Checked (decomp) |
| Eye height above the look point = **0.008 · (d − 250)² − 150** (so 2.5 m: 0.20 m above the feet, looking up; 4.2 m: 2.51 m above, looking down ~11°; 5.0 m: 5.2 m above) | `0x140164520` | Checked (decomp) |
| Yaw turns toward its goal at most **10° per frame** (`0x140718BE4`) | `0x140164520` | Checked (decomp + exe) |
| Right stick: digital bits (past half tilt), turn **3°/frame** × tilt (`0x140718BF0`), zoom by up/down; options swap the pairs | `0x140178810` family (`0x140178AB0` / `B10` / `B50` / `B80`) | Checked (decomp) |
| Options: config word `+0x41A4` of `0x140152380()`: bit 7 horizontal reverse, bit 8 vertical reverse; bit 4 read by `0x140178AE0` (from the follow functions) | `0x1402E5330`, `0x1402E5350`, `0x1402E5310` | Checked (decomp); bit 4 = "auto / manual camera": Assumption |
| Swing-behind while running: `2 (|x| + 1)(|y| + 1) · tilt · ((run speed − 8) · 0.00017453 + 0.0043633)` rad per frame; distance drifts back to the default at 0.5 per frame | `0x1401642F0` (`0x140718BE8` / `BEC`) | Checked (decomp + exe) |
| Lock-on: Sora's chest (170 up) toward the target by a fraction (KH2-Combat-in-KH1: 30 %), Sora's point 100 up; distance drifts to 380 at 1 per frame, shrinks 2 per frame while Sora or the target sits at the bottom edge; ±20° overshoot when one escapes sideways; a 300 limit for high targets | `0x140176EC0`, `0x140164290` | Checked (decomp) for the constants; the 30 % and the screen-margin test: KH2-Combat-in-KH1 notes, not re-read |
| A ±70° pitch clamp and a 350 forward point (`0x140718BF8` / `BFC`) | `0x140165E30` | Checked (decomp); what state it is (first person?): Assumption |
| Frame step `0x140717480` (static 1.0) | — | Checked (exe); 1.0 per 60 Hz frame on PC: Assumption |
| Spring of the drawn eye / look point (KH2-Combat-in-KH1: look point ~16 %, eye ~9 % a frame, ~1-unit dead band), wall ray (+5 along the normal), R3 reset (F on keyboard), lock-on target choice | — | KH2-Combat-in-KH1 notes only (Assumption here) |

**FOV units differ**: KH2's row holds radians (1.5); whether it is horizontal on a 4:3 frame
(KH2-Combat-in-KH1 reads 1.5 rad horizontal = 69.9° vertical) was not re-read: Assumption.

**Size:** KH2-Combat-in-KH1 scales KH2's distances and heights by 0.89 for KH1's Sora
(`camera_scale`, KH1 Sora's hips against KH2 Sora's). Re:CoM's Sora is KH1's skeleton
(`crate::units`), so the same question applies here (Q9).

---

## 5. Tick, rates and units

| Game | Camera step | Tag |
|---|---|---|
| Re:CoM | `dt × 0.02` = 1.0 per 60 Hz frame (`+0x14`) | Checked (decomp) |
| KH1 | `0x14233FBDC` = 1.0 per 60 Hz frame on PC; gains written for 30 fps (2.0) are halved | Checked (decomp, `kh1-tick.md`) |
| KH2 | `0x140717480` multiplies the turn and drift rates | Checked (decomp); its value at 60 fps: Assumption 1.0 |

So per-frame numbers carry over 1:1 at 60 Hz, using KH1's **halved** gains (the PC values). All
three cameras are updated once per frame by their tasks (Re:CoM: Assumption, not traced).
Lengths: KH1 / KH2 units × 0.01 = metres; y and z flip (`crate::units`). Angles: KH1 yaw ψ →
Re:CoM π − ψ. FOV: KH1's and Re:CoM's use the same `256 / tan(fov/2)` projection on a 512-wide
frame (Checked (decomp) both), so KH1's 53° goes into a Re:CoM record as 53; that both PC ports
widen the picture the same way is an Assumption (to check by screenshots).

---

## 6. Interactions

| Thing | What happens today | With a KH1 / KH2 mode |
|---|---|---|
| Events, cutscenes, `.cam` paths (battle intros, death, card get), event cameras | Other `CCamera`s on the stack in front of the player camera (2.1) | Unchanged (Re:CoM's). On their end our mode is still the player camera's mode; whether to re-place it like KH1's event return: Q4 |
| Sleights, summons, the four KH1 specials (Re:CoM's sleight start) | `0x1401FBDE0` with modes 3 / 4 / 5, record overrides `0x1401FEB50`, subject overrides `0x1401FBF80`; −1 restores `+0x17C` | Recommended: unchanged, so their camera moves stay Re:CoM's; Re:CoM's restore then returns to our mode (Q3) |
| Battle rooms (arena) | mode 3 / 4 | KH1: no battle camera of its own (KH1 fights in the field): its follow everywhere. Whether the arena edge collides with the camera sweep: Assumption |
| Boss fights | `BCam0000` records for modes 3 / 4 | Q10 |
| First-strike pause, battle start | The player camera is built in battle state (mode 3); intros are `.cam` cameras | Our base mode at the battle's camera build; intros unchanged |
| Field lock-on | Re:CoM's field exec ignores the lock | KH1 locks in the field too: Q12 |
| Riku's story | `+0x304` + 0.4 m | Recommended: Re:CoM's camera always (decision 6) (Q11) |
| Combat = Re:CoM | — | The option is its own (Q2); at Camera = Re:CoM nothing of ours runs except one compare |
| Sora's movement | KH1 movement reads the view yaw (`recom-kh1-movement.md`) | Follows any camera (Assumption: `0x14028EA00` reads the drawn view) |
| Ledge hang / climb (Re:CoM's kept) | Re:CoM's follow with action-based yaw limits (`+0x188` bits) | KH1's ledge camera (mode 6) is a scope question (§ 11) |

---

## 7. Native design

1. **Four new modes in `CPlayerCamera`:** 6 KH1 follow, 7 KH1 lock-on, 8 KH2 follow, 9 KH2
   lock-on, in the camera's own mode field `+0x178`.
2. **Their records** in Re:CoM's record format (FOV, collision radius, ease, wall distance,
   eye / aim offsets), held by the DLL (numbers from the exes, with addresses; no game file), so
   Re:CoM's own mode setters `0x1401FC4D0` / `0x1401FBE60` set the FOV, the rest vector, the
   collision radius and the first view from them. A small wrapper at the two table reads gives
   the record for modes ≥ 6 (the table has 6 slots). Alternative: the asset step appends the
   four records to `PCamData` (data Re:CoM loads; needs a re-run).
3. **Dispatch:** the init's two `lea`s point the field and battle execs at our dispatcher:
   mode ≥ 6 → our step, then Re:CoM's own tail (collision object `+0x190`, eye node, view
   `0x14024FA40`), exactly as Re:CoM's execs end; any other mode → Re:CoM's exec, untouched.
   The lock toggle for our modes (6 ↔ 7, 8 ↔ 9) by Re:CoM's lock handle `player+0x850` lives in
   our dispatcher, as Re:CoM's 3 ↔ 4 lives in its battle exec.
4. **Base mode:** at the camera's init (its final mode-set call in `0x1401FB530`) Camera = KH1
   gives 6 instead of 1 / 3, KH2 gives 8; Riku's story and Camera = Re:CoM keep Re:CoM's. A
   change on the Settings page re-sets the live camera (`*0x14087B460`) with Re:CoM's own
   `0x1401FC4D0`.
5. **Steps** (the KH1 / KH2 mode code): each frame, KH1's (3.2 / 3.3) or KH2's (§ 4) rules in
   KH1 / KH2 units converted at the edges, state kept in the camera's own fields (pitch `+0x260`,
   yaw `+0x264`, distance `+0x2A0`, eye / aim) so Re:CoM's mode-change, restore and sleight code
   see a coherent camera. Input from Re:CoM's pad (right stick `+0x6C` / `+0x70`, the reset bit
   0x400, the keyboard-mouse flag) and Re:CoM's inversion rows; KH1's stick curve rebuilt from
   Re:CoM's axis (`units::byte_from_recom_axis`). Collision through Re:CoM's own camera collision
   object with KH1's 0.40 m radius (KH1's 20 cm steps as repeated sweeps).
6. **Sleights:** they keep asking for Re:CoM's 3 / 4 / 5 through `0x1401FBDE0`; Re:CoM saves our
   mode in `+0x17C` and gives it back on −1 (Checked (decomp)). Nothing of ours in the sleights.
7. **Not native, not offered:** a camera of our own beside Re:CoM's that writes its result into
   Re:CoM's view.

**Per frame:** at Camera = Re:CoM one compare in the dispatcher. At KH1 / KH2 our step replaces
Re:CoM's step (Re:CoM's tail still runs once): a few vector operations and 1–3 sweeps (KH1's eye
walk can take more on a large move; KH1 also makes 5 occlusion tests a frame in Auto). To be
measured against Re:CoM's own step at stage 1.

---

## 8. Where the option goes

- The Combat page is full (5 rows). Movement has 4 (one left). The Settings list uses 4 of the 7
  buttons of Re:CoM's Deck submenu (`recom-config-menu.md`), so a fifth entry fits.
- **Proposed: a new "Camera" page** (5th Settings entry, Sora's story), drawn like the others
  (Re:CoM's Config screen with our rows): row 1 **Camera: Re:CoM / KH1 / KH2** (a single row like
  Dodge roll); room for **Auto camera: On / Off** (Q6) and later rows (first person, KH2 size).
  Ini key `camera = recom | kh1 | kh2`, default `recom`. Inversion stays on Re:CoM's Game Settings
  rows 0 / 1 (Q5).
- Takes effect at once (the live camera is re-set when the page closes).
- **Independent of Combat** (recommended; Q2): the camera is its own setting with its own
  Re:CoM / KH1 / KH2 values, so Combat = Re:CoM with Camera = Re:CoM plays exactly like Re:CoM.

---

## 9. Stages (one system each; each a version after a go-ahead)

| Stage | Adds | Per frame | Risks | Stays unknown |
|---|---|---|---|---|
| 1 | The Camera page and ini key (Re:CoM / KH1; KH2 shown once built); modes and records; the dispatcher; **KH1 follow** (mode 6): look point and its springs, distance / pitch / yaw, the stick (pad and mouse), the pitch clamp, R3 recenter, Manual-branch collision, KH1's FOV; field and battle rooms; sleights hand back | one compare at Re:CoM; KH1's step at KH1 | the record-table reads (6 slots); Re:CoM's sweep vs KH1's push-out; the battle arena's walls; the reset button's identity | in-game look; KH1's per-room numbers (built-in defaults) |
| 2 | KH1's **Auto** pieces (if wanted, Q6): swing-behind, wall-avoid, pitch assist, the automatic recenter, the eye walk with 20 cm steps and the occlusion re-place | 5 occlusion tests + sweeps | cost of the tests in Re:CoM's collision | wall-avoid / pitch-assist numbers re-read on Steam at this stage |
| 3 | **KH1 lock-on** (mode 7): KH1's anchor, look-past, 90 % eye, ±5° / 10° yaw, keep-Sora-on-screen, the 60 / 45-frame blends (KH1's `1 − cos` blend as a mode-change ease of ours on Re:CoM's camera), field and battle per Q12 | KH1's lock step | the lock point (Re:CoM's `CamTrgOfs` point for KH1's bone lock points) | KH1 lock points of KH1-twin enemies |
| 4 | **KH2 follow** (mode 8): row 0, the height curve, 10° turn cap, digital stick turn / zoom, swing-behind, distance drift, reset, the wall ray; the KH2 row value on the page | KH2's step | units and size (Q9); FOV convention | the spring rates re-read in the KH2 decomp at this stage |
| 5 | **KH2 lock-on** (mode 9) | KH2's lock step | the screen-edge tests on Re:CoM's projection | — |
| 6+ | Scope items the owner picks (§ 11): first person, movement-state cameras, a KH1 camera trace for 1:1 checks | — | — | — |

Each stage: tests (pure rules: springs, stick curves, clamps, unit conversions frame by frame
against the decomp numbers), the site check for every patched byte (`check_sites_rs.py`), the
Windows build. 1:1 check per stage: the KH1 / KH2 decomp numbers; a KH1 trace only if the trace
recorder gains camera columns (eye, target, FOV; today it records the view yaw only,
`kh1-trace.md`): ask before that crate change.

---

## 10. Open questions for the owner

- **Q1 Where.** (a) a new "Camera" page in Settings (recommended); (b) a fifth row on Movement;
  (c) somewhere else.
- **Q2 With Combat.** (a) independent, default Re:CoM (recommended); (b) follows Combat (KH1
  camera at Combat = KH1; then KH2 needs its own switch); (c) independent, but only active at
  Combat = KH1.
- **Q3 Sleights, summons, specials.** (a) Re:CoM's own sleight / summon camera moves stay, then
  back to the KH1 / KH2 camera (recommended); (b) the KH1 / KH2 camera stays on through them (the
  sleights' close-ups and framing are lost); (c) Re:CoM's for sleights and summons, the KH1 / KH2
  camera for the four KH1 specials (whether KH1 itself moves its camera for them: not read).
- **Q4 After an event or cutscene.** (a) carry on from where the camera was (Re:CoM's way);
  (b) KH1's way: re-placed behind Sora at the default pitch and distance (KH2 resets behind him
  too).
- **Q5 Inversion.** (a) Re:CoM's Game Settings rows 0 / 1 drive the KH1 / KH2 camera too, each
  game's own "Normal" (recommended); (b) the same rows applied with Re:CoM's signs (same stick →
  screen feel as Re:CoM's camera); (c) two rows of our own on the Camera page.
- **Q6 KH1 / KH2 Auto camera.** (a) Auto only (KH1's default, Assumption); (b) an
  "Auto camera: On / Off" row on the Camera page, for KH1 and KH2; (c) Manual only.
- **Q7 Sensitivity.** KH1 has none on the pad; its mouse uses the PC sensitivity (default 20).
  (a) KH1's / KH2's own speeds, mouse at the default 20 (recommended); (b) a sensitivity row.
- **Q8 KH1's room numbers.** (a) KH1's built-in defaults everywhere (10°, 4.0 m, 1.0 m)
  (recommended); (b) per Re:CoM world, the numbers of a KH1 room of that world, read from the
  player's KH1 map files by the asset step (we'd agree the rooms); (c) built-in in the field,
  KH1's second preset (480, −0.15 rad, 1.5 m) in battle rooms.
- **Q9 KH2's size.** (a) KH2's numbers as they are (1:1 values; Sora looks smaller in the
  frame); (b) scaled 0.89 for KH1's Sora, as KH2-Combat-in-KH1 does.
- **Q10 Boss fights.** (a) the KH1 / KH2 camera in boss fights too; (b) Re:CoM's boss cameras
  (`BCam0000`) kept in boss fights.
- **Q11 Riku's story.** (a) Re:CoM's camera always (recommended, decision 6); (b) the option
  applies to Riku too.
- **Q12 Lock-on in field rooms.** (a) KH1's / KH2's lock camera in the field and in battle (1:1);
  (b) battle only (as Re:CoM).
- **Q13 Keyboard and mouse.** (a) KH1's mouse rule (acceleration 0.3°, decay 4°) whenever
  Re:CoM says the keyboard / mouse is in use (1:1); (b) the pad rule always.

Decided without asking (no change to play): the reset button stays Re:CoM's (it does KH1's
recenter / KH2's reset in those modes); KH1's halved PC gains (the 60 Hz values) are used.

---

## 11. Found, not asked for (scope; ask before adding)

- **KH1's first-person view** (Select, mode 2; `0x1401E2390`): Re:CoM has none. Matters for 1:1
  feel in exploration only.
- **KH1's movement-state cameras** (modes 4, 5, 6, 8: vines / climbing, ledge, swimming…):
  Re:CoM keeps its own ledge hang (decision 16), so only the ledge camera could apply.
- **KH2's camera rows 1–3** (large rooms, 6.0 m): which Re:CoM rooms, if any.
- **Target switching** (KH1: R1 cycles the lock list; KH2: R2 + right stick) and **KH1's Scan
  HP bar** (ROADMAP M12): lock-on side, with the R1 lock-on work.
- **A camera trace** (eye, target, FOV in `kh1-trace` and our trace) for frame-by-frame 1:1
  checks.

---

## 12. Sources and what wasn't checked

- Read: Re:CoM Steam decomp and exe (`recomsteam.exe`), the owner's `PL0001.BIN`; KH1 Steam
  decomp and exe (`kh1steam.exe`); KH2 Steam decomp and the owner's KH2 FM exe; KH2-Combat-in-KH1's
  `docs/kh1-camera.md`, `camera-model.md`, `kh1-lockon.md` (Epic build, converted through its
  `steam_map.in` where it had entries; the rest found by position in the Steam decomp).
- Not checked: anything in game; KH1's wall-avoid and pitch-assist numbers on Steam; KH2's
  springs, wall ray, reset and the lock-on's 30 % / screen tests in the KH2 decomp; which button
  Re:CoM's camera reset is; how Re:CoM's PC port feeds the mouse; whether Re:CoM's player camera
  keeps updating under an event camera; KH1's map camera blocks (no KH1 map files in the session).

---

## 13. Stage 1 built (0.134.0): the Camera page and the KH1 follow camera (mode 6)

Owner, 2026-11-04 (§ Camera answers): independent of Combat, a Settings > Camera page, sleights /
summons / specials and boss cameras Re:CoM's, the rest as recommended.

### What runs

- **Page** (`menu::Page::Camera`, `keyblades::sub::Row::Camera`): the fifth Settings entry, one
  row **Camera: Re:CoM / KH1** (ini `camera`, default `recom`; `kh2` warns and reads as Re:CoM's
  until stage 4). At once.
- **Records** (`camera::records`): modes 6..9 in Re:CoM's format, in a block near the exe. Mode 6:
  FOV 53, radius 0.05 (Re:CoM's follow value, so its tail's sweep is Re:CoM's), aim 0.5 m above
  Re:CoM's subject point (= KH1's 1.0 m above the feet: the subject point is the root node + 0.5 m,
  `0x1401F0510`, Checked (decomp)), eye 4.0 m at 10° from it. Modes 7..9 carry mode 6's until
  their stages.
- **Record reads** (`sites::CAMERA_PATCHES`): `0x1401FC513` (14 bytes, in `0x1401FC4D0`) and
  `0x1401FBE85` (11 bytes, in `0x1401FBE60`), the only indexed readers of `0x14087B660` (Checked
  (decomp: every reference)), jump to stubs (`camera::stub::table_read`) that replay the replaced
  bytes for modes < 6 or ≥ 10 and read our table for 6..9, counting each such mode set (the step's
  "entry").
- **Dispatcher**: the init's `lea`s `0x1401FB556` / `0x1401FB587` point at two jumps in the block
  to `win::camera::field_exec` / `battle_exec`. Camera = Re:CoM and a Re:CoM mode: Re:CoM's exec
  (one atomic load, one compare). Otherwise `camera::dispatch`: Re:CoM's 1 / 3 / 4 switch to 6
  with Re:CoM's `0x1401FBE60` (never during a sleight's request `+0x188 & 0x2000`, a subject
  override `+0x2B4`, or Re:CoM's mode 2), and `+0x17C` takes 6 so a sleight's −1 gives it back;
  mode 6 at Re:CoM / Riku / a boss camera switches back to 1 / 3.
- **Init** (`sites::CAMERA_HOOKS`, `0x1401FB530`, 7 bytes): after Re:CoM's init, Riku's story is
  read (`0x1401AAAE0`) and, when wanted, `0x1401FC4D0(cam, 6, 0, 0)` and `+0x17C` = 6; the first
  step places the camera behind Sora.
- **Boss cameras**: Re:CoM's count `0x14087B468` (> 0 only when the boss's pack holds `BCam0000`,
  `0x1401FEC50`; cleared by the camera's dtor; Checked (decomp)) keeps Re:CoM's camera in that
  battle. A boss without `BCam0000` gets the KH1 camera (Assumption: no camera of its own then).
- **The step** (`camera::kh1::Follow::step`, KH1 units, converted at the edges): in the battle
  exec first Re:CoM's own record restore (`+0xD0` actor gone → `0x1401FEB50(0, 0)`, what its battle
  exec does), then KH1's mode 1 in KH1's order: the look point's spring, the stick, the recenter's
  blend, the Manual-branch eye and its wall ray, R3, the distance, the yaw from the result. Then
  aim / eye (and their goals) into `+0x220` / `+0x230`, Re:CoM's own angles of them
  (`0x1401FB890` into `+0x270`, yaw `+0x264` = its result, pitch `+0x260` = 0: what Re:CoM's follow
  eases toward, so a hand-over to Re:CoM's modes starts where we are), and Re:CoM's tail as its
  execs end (`0x1401FC0D0`, the node's eye, `0x14024FA40`).
- **Entries**: a new camera → behind Sora (facing + π, KH1's re-place `0x1401E4820`); a mode set of
  ours by Re:CoM's setters (a sleight's restore, our switch) → KH1's entry, the yaw of the current
  view; the camera drawn again after another camera was (`0x140AC4370` slot 0: events, `.cam`
  paths, battle intros) → behind Sora (Q4 (b)).

### KH1 numbers (KH1 Steam; Checked (decomp) unless tagged)

| What | Value | Where |
|---|---|---|
| Look offset, pitch, distance | (0, −100, −50), 10° (bits `0x3E32B8C3`), 400 | `0x140506CA0` / `CB0` / `CBC` (Checked (exe)) |
| FOV | 53 | `0x1403E9E44` (Checked (exe)) |
| Look spring | 20 % horizontal, 10 % up / 50 % down, × 0.5 at step < 2 (so 10 / 5 / 25 %) | `0x1401E1920`; the look point falls back to above Sora when a segment to it is blocked (`0x140193180`) |
| Stick, pad | +0.15° × tilt × 0.5 a frame, at most 2.5°, −0.24° × 0.5 on release | `0x1401E7320` |
| Stick, mouse | +0.3° × 0.5, at most 2.5 + 0.0875 × 20 = 4.25°, −4° × 0.5; stops at once on release; reversing zeroes | `0x1401E7320` |
| Pitch clamp | −15°..+50° | `0x1401E7320` |
| Stick input | KH1's radial dead zone 60 of 127.5 (mouse 1.0), on the bytes rebuilt from Re:CoM's ints | `0x14028B4F0` |
| Inversion | KH1 Normal: left / right +1, up / down −1 (its Config reset `0x1402FA295` case 7; setters `0x1402FAF40` / `F70`); Re:CoM's Normal is (−1, +1) (`0x1401FB530`): KH1's = −Re:CoM's | Checked (decomp both) |
| Recenter | facing + π in 60 frames, sin-eased; 30° wall steps, the long way, else the swap (snap at half, distance × (0.3..1), pitch + up to 42°) | `0x1401E62A0`, `0x1401E1920` |
| Manual eye | ray to 60 past the eye; eye at the hit or 60 back from it; look ≥ 20 from the eye; distance = default | `0x1401E1920` |
| Re-place | look to Sora's target, defaults, eye at the yaw; a 40-unit sphere walked to a ray's hit | `0x1401E4820`, `0x1401E4700` |
| Frame step | 1.0 a 60 Hz frame both (`0x14233FBDC`; Re:CoM's camera `+0x14` × 0.02) | `kh1-tick.md`, `recom-timing.md` |

### Found at build

- **The look target's side** (§ 3.2's Assumption): KH1's offset z −50 is on Sora's local −z, and
  his matrix maps local +z to his facing (`kh1_core::Vec3::rotate_yaw`, same Assumption as the
  movement code): the look point is **0.5 m behind** Sora.
- **The eye's pitch sign**: `0x1400E6AC0`'s rows give `(0, 0, d)` → `(0, −d sin p, d cos p)`: a
  positive pitch puts the eye above (y down). Checked (decomp).
- **Re:CoM's angle convention** (`0x1401FB890`): yaw of the eye from the aim, pitch positive with
  the eye above. Re:CoM's follow eases `+0x264` toward `+0x274` and `+0x260` to 0 with no input
  (`0x1401FDD0A` / `0x1401FDD4E`, Checked (disasm)).
- **§ 3.3's anchor** (stage 3): `0x140506E10` reads (0, −100, **300**), not 350 (Checked (exe)):
  to settle at stage 3.

### Not carried, Assumptions

- KH1's mouse multiplier `|dx × 10|` (`0x140138B30`): no Re:CoM source; 1. Re:CoM's axes are taken
  as KH1's mouse stick at keyboard / mouse (Assumption: Re:CoM's PC port feeds the mouse there).
- KH1's ray (`0x140193790`) and segment test (`0x140193180`) as Re:CoM's camera sweep
  (`0x1401FBC60`) with a 0.005 m sphere; the re-place's second test `0x140193650` not carried.
- R3 = Re:CoM's reset bit `0x400` (plan § 2.4); "Sora can act" = the camera's input lock
  `+0x188 & 1` clear (KH1 `0x140297950`: `+0x444 & 1`).
- The camera keeps updating under another camera; slot 0 is the drawn one.
- At stage 1 the KH1 follow stays on while locked on (KH1's lock camera is stage 3); Re:CoM's
  3 ↔ 4 lock switch does not run in mode 6.

### Per frame

- Camera = Re:CoM: one atomic load and one compare in the dispatcher, then Re:CoM's exec.
- Camera = KH1: our step replaces Re:CoM's step; Re:CoM's tail runs once as before. The KH1 math
  measured at **76 ns a frame** (Linux, release, `camera::tests::cost_of_a_step`, 10⁶ frames);
  Re:CoM's camera sweep runs **2 times a frame** (the look point's segment test and the eye's ray)
  against **1** in Re:CoM's follow, plus a mutex lock and Re:CoM's angle function. An R3 press adds
  up to 13 sweeps on that frame. Not measured in game.

### Tests

`camera::tests` (modes, records, dispatch, offsets and conversions, the re-place, the look spring's
rates, the stick's acceleration / maximum / decay / clamp, the mouse rule, R3's 60-frame swing, the
Manual ray against a wall, the stubs and the block), `sites::tests::camera_sites`, `menu` /
`keyblades` / `settings` tests for the page; `check_sites_rs.py`.

---

## 14. Stage 2 built (0.135.0): KH1's Auto camera behind the Auto camera row

Owner, 2026-11-04 (§ Camera answers Q6 (b): an "Auto camera: On / Off" row; § Lock-on rules and
camera walls: the near-wall ramp "(a) 8 + 8 flat 2.5 m rays").

- **Row**: the Camera page's second row **Auto camera: Off / On** (ini `auto_camera`, default on:
  KH1's Config field camera 0 = Auto, its reset value `0x1402FA295` case 7, Checked (decomp); the
  label "Auto": Assumption, Epic notes). At once.
- **Order** (`0x1401E1920`, Checked (decomp)): Sora's speeds (`0x1401E5D90`: 3D and horizontal,
  units a frame, from his position last frame), the look spring, the stick, the recenter's blend;
  **Auto** (Config Auto and not KH1's push state 0x1B): the swing-behind, the wall-avoid, the pitch
  assist, the eye walk, the visibility re-place, the auto recenter; **Manual**: the pitch assist,
  the eye's wall ray; then R3, the distance (Auto: back toward 400 at 10 % a frame; Manual: 400),
  the yaw.
- **Fix from stage 1**: KH1 runs the pitch assist in **Manual too** (the Manual branch calls
  `0x1401E6670`), so the pitch returns to 10° at 3 % a frame whenever no floor is within 55 under the
  eye. Stage 1 lacked it (the stick's pitch stayed). Now with a stick held down KH1 rests at 48.8°
  (the clamp, then 3 % back the same frame), as KH1.

| Piece | KH1 (Checked (decomp)) | In Re:CoM |
|---|---|---|
| Swing-behind `0x1401E6FF0` | ramps: 60 frames, then a second to 90 (mouse 0 / 40); a wall ramp to 60 (mouse 20), −0.3 a frame away from walls; near a wall the second ramp jumps to its top; gain (wall / 60 × 2 % + second / 90 × 2 %) × the angle to Sora's back, faded past 120°; reset when his facing moves 30° in a frame or he slows to his threshold (1.0 a frame on the ground, 0.1 in the air) | as KH1; the wall probe `0x140196300` (a flat 250 circle at the look point and at the eye) is the owner's (a): 8 flat 2.5 m rays from each with Re:CoM's sweep, only while the swing runs (walls within 2.31 m always caught) |
| Wall-avoid `0x1401E6C10` | a ray 50 to either side of the look→eye line; then the ±30° rays of 300 from the eye; exactly one blocked and Sora moving: a 40-frame sin swing to run along the wall (from the side ray's polygon normal ± 90°); the right stick's x held cancels it | Re:CoM's sweep; the normal from the sweep's shape `cam+0x1A8 → +0x40` (Re:CoM's solver writes it with the contact, its stuck default (0, 1, 0): Assumption that it is the surface normal) |
| Pitch assist `0x1401E6670` | standing over 60 frames (Auto): no wall 100 ahead and no floor within 500 under a point 30 ahead 100 up → the pitch rises 3° × ramp/30 a frame (to 85°); in the air: no floor 1000 under the look point, or falling faster than 5 with nothing 250 along the fall → the same; the eye 55 over a floor (< 49.5): + (1 − d/55) × ramp/30 × 3°; else back to 10° at 3 % once its ramp has decayed (0.3 a frame while moving); none of it in states 4 / 5 / 8 / 0x15 / 0x18 / 0x1A | Re:CoM's sweep; Sora's state: 0x15 in Re:CoM's glide (action 8), 2 in the air (Re:CoM's ground test `0x1401F3C00` read from its cache, or its rule, without running it), else 0; his air velocity = his mover's `+0x80` |
| Eye walk | to the goal as a 40 sphere in 20-unit steps stopping at contact; re-placed (`0x1401E4820`) when the distance passed 400 + 300; a jump from 1000 | one Re:CoM sweep of the 40 sphere from the eye to the goal (same stop, Assumption: no tunnelling in Re:CoM's sweep) |
| Visibility `0x1401E7780` | 5 segments from the eye: above Sora, it turned ±15° about the eye, it ±50 up / down; all blocked → re-place at the yaw | Re:CoM's sweep, stopping at the first clear one |
| Auto recenter | eye within 40 horizontally of the look point, pitch under 60°, Sora moving → `0x1401E62A0` | as KH1 |

- Not carried: KH1's global flag `0x142867368 & 0x10` (skips the wall-avoid / pitch assist; taken
  as clear); KH1's states other than 0 / 2 / 0x15 (hit, flight, ladders) are not told apart
  (Assumption).
- **Per frame** (Camera = KH1, Auto on; not measured in game): standing, about 5 Re:CoM sweeps
  (look check, a side ray, the eye's floor ray, the eye walk, one visibility segment), + 2 in the
  ledge look after 60 still frames; running straight, + 16 for the wall probe; a wall-avoid start
  + 2. Re:CoM's own follow: 1. Auto off: stage 1's 2 + the eye's floor ray.
- Tests: `camera::tests::the_swing_behind_ramps_as_kh1s` (the 60-frame ramp, the second ramp's
  first gain, the wall ramp), `auto_eye_distance_and_visibility`,
  `the_pitch_assist_lifts_an_eye_near_the_floor`, the stick test with the pitch assist (48.8°,
  the return to 10°).
- L2 / R2: KH1's camera module reads only R3, Select, the right stick's x held and (first person)
  0x10 / 0x40, never L2 / R2 (Checked (decomp + the pad table `0x140528290`)): no clash with KH1's
  L2 target step.

---

## 15. Stage 3 built (0.136.0): KH1's lock-on camera (mode 7)

Owner's answers: KH1's lock camera in field rooms too (Q12 (a)); the lock button and KH1's lock
rules are the R1 lock-on work's (Re:CoM's lock state is what the camera reads).

- **Switch** (`camera::dispatch`): mode 6 ↔ 7 on Re:CoM's lock handle `player+0x850`, as Re:CoM's
  battle exec switches 3 ↔ 4, in field and battle rooms, not while a subject object holds the camera
  (`+0x2B8`, Re:CoM's own rule), through Re:CoM's `0x1401FBE60`. Our battle step also writes
  `+0x2A4` = the handle as Re:CoM's exec does.
- **Blends** (KH1's mode switch `0x1401E5360` and the camera task `0x1401E5D90`, Checked
  (decomp)): to the lock camera 60 frames (`0x1401E41A0`), back to the follow 45 (`0x1401E4110`),
  the drawn eye / look = old × (1 − f) + new × f with f = cos(left / total · π/2); during a blend
  the follow's look point snaps to Sora's target and its stick is off. The follow's entry after
  the lock takes the drawn view's yaw (KH1's entry from the public view). A new target while locked
  (Re:CoM's handle changes: KH1's L2 step, or Re:CoM's switch): KH1's re-lock blend of the look
  point over 60 frames (`0x142537F08` / `F10`).
- **The lock step** (`camera::kh1::Follow::lock_step`, KH1's mode 3 `0x1401E2650`; Checked (decomp,
  the eye close's scale `(len − 75) × 0.9` read in the disasm at `0x1401E2838..0x1401E2864`)):

| Piece | KH1 |
|---|---|
| Anchor | Sora + Ry(yaw) · (0, −100 + height assist, 350) (`0x140506DF0`, Checked (exe)) |
| Look-past point | the lock point + (radius + 100) horizontally away from the anchor, half the anchor's height gap back; the anchor raised by half the gap when it is steep |
| Eye | 90 % of the horizontal gap to the anchor past a 75 dead zone, 90 % vertically; a ray from Sora's body point (0.85 m up, code `0x8003`) clamps it |
| Yaw | half the error outside ±5° of the line to a point 100 ahead of Sora (`0x140506E00`), at most 10° a frame |
| Look | from the eye toward the yaw, the look-past point's distance and height |
| Keep Sora | his feet outside half of the 53° FOV: a counter to 40 (+1 a frame; −2 inside 80 %) pulls the look point up to 80 % to his body point |
| Height assist | the floor under the eye (`0x1401E5A20`): Sora > 30 below it → the anchor up to 170 higher; a target > 200 above the eye → up to 20 lower |

- **In Re:CoM**: the lock point is Re:CoM's own lock-on point (`0x1401FC400`: the look-target
  override's, else `player+0x870`, the target's root + its `CamTrgOfs`, `0x14017F770`) where KH1
  uses its lock table's bone point (`0x1402B02D0`): Assumption, the plan's flagged risk. The
  radius is the KH1 twin's lock radius (`kh1-stats.json`, `win::stats::lock_radius`), 0 for an
  enemy without a twin (Assumption). The rays are Re:CoM's camera sweep; the floor probe one sweep
  of 100 m.
- **Not carried**: Select's first person (scope § 11).
- **Per frame** (mode 7): 2 Re:CoM sweeps (the body ray, the floor probe) against 1 in Re:CoM's
  lock step; not measured in game.
- Tests: `camera::tests::the_lock_camera_frames_sora_and_the_target` (the rest pose: the eye at the
  75 dead zone of the anchor 3.5 m behind, the look 1 m past the target; the 90 % close; the 10°
  cap), `mode_blends_and_relock` (the 60 blend's weights, the re-lock blend, the unlock's 45 blend
  and entry), the dispatcher's 6 ↔ 7 rows.

---

## 16. Stage 4 built (0.138.0): KH2's follow camera (mode 8), its Auto behind the Auto camera row

Owner's answers: KH2's lengths × 0.89 for KH1's Sora (Q9 (b)); the Auto camera row for KH1 and KH2
(Q6 (b)); each game's own Normal on Re:CoM's inversion rows (Q5 (a)); KH2's re-place after an event
(Q4 (b)). KH2 Steam exe: the owner's `KINGDOM HEARTS II FINAL MIX.exe`; decomp `kh2decomp`.

- **Row**: Camera is a three-value row **Re:CoM / KH1 / KH2** (Re:CoM's single-value row, as Dodge
  roll); ini `camera = kh2` (no longer a warning). Auto camera row: KH1's or KH2's Auto.
- **Native shape**: as stages 1–3. Mode 8 has a record in the block (`camera::records` slot 2; slot
  3 for mode 9 the same until stage 5): Re:CoM's setters set its FOV and rest. `camera::dispatch`
  switches Re:CoM's 1 / 3 / 4 (and our 6 / 7) to 8 at Camera = KH2 with Re:CoM's `0x1401FBE60`; the
  step (`camera::kh2::Cam::step`) replaces Re:CoM's step, Re:CoM's tail runs. No new hook, patch or
  site. At stage 4 the KH2 follow stays on while locked (stage 5 brings mode 9).
- **Units**: KH2's step runs in KH2's own numbers on a world shrunk by 1 / 0.89 (positions in ÷ 0.89,
  out × 0.89, Re:CoM's sweeps in KH1 units): every length below is applied × 0.89, angles, rates
  and screen margins as they are.

### KH2's camera (KH2 Steam; Checked (decomp) unless tagged)

| What | KH2 | × 0.89 in Re:CoM |
|---|---|---|
| Frame (`0x140166100`) | last goals ← goals; mode 0 the follow `0x140165110`; the view `0x140165700` | — |
| Row 0 `0x140718C00` (Checked (exe)), loader `0x140165A70` | FOV 1.5 rad, default 420, lock 380, range 250..500, k 0.008 | 373.8 / 338.2 / 222.5..445 units; k ÷ 0.89 |
| Look goal | Sora's feet − 170 (y down) | 1.513 m above the feet |
| Eye goal `0x140164520` | yaw to its goal ≤ 10° a frame (`0x140718BE4`, Checked (exe)) unless a cut; eye = look + Ry(yaw)·(0, 0, d); eye.y = look.y + 150 − (d − 250)² · k | at 420: 3.738 m back, 0.723 m over the look point |
| Right stick (`0x14039C180`, table `0x1405C34F0`) | per axis: byte − 127.5 less 60 (`0x140BF3338`), over 67.5; a direction bit once past 60; tilt = |(x, y)| ≤ 1; the mouse 1 | Re:CoM's ints → bytes (`units::byte_from_recom_axis`) |
| Turn | right / left bit: yaw and goal ± tilt · 3° (`0x140718BF0`, Checked (exe)) · step | as KH2 |
| Zoom `0x1401654D0` | up / down bit: d ∓ tilt · 8 · step, clamped 250..500 (no pitch: the height curve) | 7.12 a frame |
| Options | bit 7 (`0x1402E5330`) swaps left / right, bit 8 (`0x1402E5350`) up / down: KH2's Normal turns + with the stick right and zooms in with it up, as KH1's Normal | `kh2::swaps` from Re:CoM's `+0x2F0` / `+0x2F4` |
| Reset (logical `0x20000` = raw `0x4`, R3 in the PS2 order) | goal = behind Sora (`0x140164E90`: facing + π), d = 420, the drawn eye to the eye goal; a cut (yaw at once, view snapped `0x140164D70`) when the left stick is held | Re:CoM's reset bit 0x400 (Assumption R3, § 2.4) |
| Swing-behind (auto `cam+0x42`, Sora moved in x / z `0x1403B8940`, left tilt ≠ 0, right tilt 0, left y < 0.9, no cut) | yaw and goal toward behind by ≤ 2 (|lx| + 1)(|ly| + 1) · tilt · ((run − 8) · 0.00017453 + 0.0043633) · step (`0x140718BEC` / `BE8`, Checked (exe)); d back toward 420 at 0.5 · step (only 1 or more away), clamped | 0.445 a frame; run = 8 (Assumption: KH2 Sora's `+0x12C` in his base form, KH2-Combat-in-KH1's notes; writer not found): about 1° a frame pushing straight on |
| Wall `0x140164940` (ray `0x140171210`) | look goal → eye goal: the eye at the hit + 5 along the polygon normal | Re:CoM's sweep (stage 2's normal, Assumption) + 4.45 units |
| Springs `0x140165BD0` | cur += n · ((v·w)/|v| · 0.5 + (1 − |v|) · 2.5) · rate, v = cur − goal, w = (last goal − goal) · rate; rate 0.07 + (run − 8) · 0.0006 (mouse mode `0x142B44A88`: 0.4 / 0.0018), halved at step < 2; the look point rate + 0.03: **eye 8.75 %, look 16.25 % of (gap − 1) a frame**, resting 1 away; mouse 50 % / 57.5 % | rest gap 0.89 units; Re:CoM's keyboard / mouse flag for KH2's mouse mode (Assumption) |
| View wall (mode 0) | drawn look → drawn eye: a hit moves the drawn eye and becomes the eye goal | as KH2 |
| Re-place `0x140164FA0` (room start `0x140164C60` → the loader's 420, `0x140164CB0`) | yaw = goal = behind, look goal, eye goal, snapped; no wall test | a new camera: room start; back from another camera: the re-place, distance kept |
| Frame step `0x140717480` | 1.0 (Checked (exe)) | Re:CoM's `+0x14` × 0.02 = 1.0 |

**FOV** (Checked (decomp + exe), both games): KH2's `0x14019DDE0` (param 0) gives focal
(512 / 0.8) / 2 / tan(fov / 2) = 320 / tan(0.75) on its 640 × 480 frame (its view `0x14019D050`:
0.8 / 0.8667 → 640 × 480; vertical 69.87°); Re:CoM's `0x14024FCB0` gives focal 256 · cot(fov / 2)
on its own 640 × 480 (`0x14023E720` with 512 / 0.8 and `0x140821ADC` = 384 / 0.8). The same focal
in Re:CoM's record: **2 atan(0.8 · tan 0.75) = 73.39°** (69.87° vertical in both). That both PC
ports widen 16:9 the same way: Assumption (as KH1's 53).

**Corrected from the stage 1–3 notes** (`/home/claude/kh2-cam-findings.md`): the pad bit with
`0x1403BA300` is **logical 0x400000 = raw Select (0x1)**: KH2's first person (mode 1, the ±70°
`0x140718BF8` / `BFC` state), not R3; R3 is the reset (logical 0x20000). KH2's stick bits come
past a **60 dead zone per axis** (47 % of a half tilt), the speed is the stick's tilt.

### Not carried (each an owner question or Re:CoM has no data)

- Select's first person (mode 1, `0x140165E30`): scope, as KH1's.
- KH2's R2 held (logical `0x100000000`, raw `0x200`) swapping Auto / Manual for as long as it is
  held (`0x140178AE0`), and letting the zoom work in areas that forbid it: R2 is Re:CoM's lock-on at
  Combat = Re:CoM; not carried (question).
- KH2's no-zoom areas (`0x1403BABF0`, an actor flag from the map), its camera volumes
  (`0x140167A50`), its attribute-5 polygons in the wall test, the mouse's speed multipliers
  (`cam+0` / `+4` = |d · 10|: no Re:CoM mouse delta; 1, as stage 1).

### In Re:CoM

- Entries (`win::camera::kh2_step`): a new camera → KH2's room start; the camera drawn again after
  another camera → KH2's re-place; a mode set by Re:CoM's setters (a sleight's restore, a switch
  from Re:CoM's or KH1's camera) → KH2 carries on from the view Re:CoM drew (KH2 has no entry: its
  mode changes, its springs carry the view).
- The left stick: Re:CoM's pad device `+0x74` / `+0x78` (`sites::data::PAD_DEVICES`, what Sora
  moves by), through KH2's dead zone.
- Riku's story, Camera = Re:CoM, boss cameras, sleights / summons / specials: as stages 1–3.

### Per frame (Camera = KH2)

KH2's math **62 ns a frame** (Linux release, 10⁶ frames, `camera::tests::cost_of_a_kh2_step`;
KH1's measured 152 ns in the same run); Re:CoM's camera sweep **2 times a frame** (the goal's wall
ray, the drawn eye's) against 1 in Re:CoM's follow; Re:CoM's tail once. Not measured in game.
Camera = Re:CoM: unchanged (one load, one compare).

### Tests

`camera::tests::kh2_records_and_rest_pose` (FOV 73.39°, the rest pose 1.513 / 3.738 / 0.723 m, the
dispatch rows, the swaps), `kh2_sticks_are_per_axis_past_60`,
`kh2_turns_3_degrees_and_zooms_8_on_its_height_curve` (3° and half tilt, 8 a frame to 250 where the
eye is 150 under the look point, 500, the swapped row), `kh2_reset_turns_at_most_10_degrees_or_cuts`,
`kh2_swings_behind_a_running_sora` (1° a frame, 0.5 back, not with the right stick / pulled back /
Auto off / standing), `kh2_springs_close_the_gap_at_its_rates` (8.75 % / 16.25 %, rest 1, the moving
goal's term), `kh2_wall_puts_the_eye_5_off_the_wall`; `menu` (the 3-value row) / `settings` tests.
