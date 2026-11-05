# Lock-on on R1 at Combat = KH1 (0.133.0)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Shop text, lock-on and camera), verbatim:
"Let's have the lock on button be mapped to R1 if combat is set to KH1. If on mouse and keyboard it
stays the same." Code: `crates/kh1-recom/src/lock_on.rs` (pure), `src/win/lock_on.rs`; sites
`sites::LOCK_ON_HOOK`, `LOCK_ON_CHECKED` (test `sites::tests::lock_on_sites`).

## Re:CoM's lock-on (Checked (decomp + disasm + exe))

| what | where |
|---|---|
| The lock-on, once a frame from the player's Field and Battle exes. Off unless the target cursor (`player+0x3F8`) allows it (`+0xF0`) and `player+0x418 & 0x10000` is clear. Not locked: the button **pressed** locks (sound 7, `0x140208EC0`). Locked: pressed again sets `+0x424 & 0x1000`, then its **release** unlocks (sound 8); **held** with a right-stick flick (`0x1401FF4A0`: the stick's direction changed) switches to the next target that way (`0x1401CD4B0`). The handle goes to `player+0x850` (the camera's mode 3 ↔ 4 reads it) and the target to `+0x84C`. | `0x1401E8DA0` (callers `0x1401F994B`, `0x1401FA1A2`) |
| Its button is the player's pad object (`player+0x230`) field **`+0x44`**, read only by the lock-on's three readers: pressed `0x1401FF740`, released `0x1401FF600`, held `0x1401FF4F0` (each `mov ecx,[rcx+0x44]` then the pad test; each called only from the lock-on). | `0x1401FF4F0`, `0x1401FF600`, `0x1401FF740` |
| The pad object's update writes its action buttons every frame: `+0x38` = 4, `+0x3C` = 8, `+0x40` = 1, **`+0x44` = 2**. | `0x1401FF150` (`0x1401FF19D`) |
| Button order (Re:CoM's pad word): PS2 order with the two pad bytes swapped by the pad read (`0x14029C690`): L2 1, **R2 2**, L1 4, **R1 8**, Triangle 0x10, Circle 0x20, Cross 0x40, Square 0x80, Select 0x100, L3 0x200, R3 0x400, Start 0x800, Up 0x1000, Right 0x2000, Down 0x4000, Left 0x8000. Consistent with every reader we use (L1 = 4 for the shortcut list, Triangle 0x10 for stock, L1 + R1 = 4 + 8 for a sleight `0x1401FF840`). So **Re:CoM's lock-on is R2** on a pad (Assumption: R2 / R1 here are the buttons Re:CoM's own controls call R2 / R1 with the default binding). | `0x14029C690`, `0x1401FF840` |
| What R1 does in Re:CoM: card scroll right (`+0x3C` with D-pad Down, `0x1401FF530` / `0x1401FF780` in the request bits `0x1401F8150`), sleight with L1. At Combat = KH1 in battle the request bits run Re:CoM's field branch (no card scroll, reload, stock or sleight input: `win::attack::request_bits`) and the use-card button is skipped, so **R1 does nothing at KH1** before this version; ours: none (the command menu counts it only as "another button", the L1 list is L1 held). | `0x1401F8150`, `win::attack` |
| Pad and keyboard: the PC port reads XInput (`XINPUT1_4` ordinals 2 / 3), DirectInput 8 and HID pads and the keyboard into the same PS2 pad word through its bindings (`0x14083F3F0..`). Which device is in use: **`0x140C2FF64`** (1 keyboard / mouse, 0 pad), set each frame by `0x140347EF0` (any key or mouse input → 1 and the key glyphs `0x140349F40`; any pad button or stick → 0 and the pad glyphs `0x140349120`); getter `0x1403471C0`, also read by Re:CoM's glyph picker `0x140345B40`. | `0x140347EF0`, `0x1403471C0` |

## KH1's (Checked (KH1 Steam decomp + exe))

| what | where |
|---|---|
| The pad step remaps the PS2 pad word through a 19-pair table: **R1 `0x800` → logical `0x400`** (the lock-on), L2 `0x100` → `0x800` and `0x400000`, R2 `0x200` → `0x400000`, L1 `0x400` → `0x40000` (the shortcut list), R3 `0x4` → `0x200` (camera recentre), Select `0x1` → `0x8000` (first person). | `0x14028B0B0`, table `0x140528290` |
| The lock-on: not locked, `0x400` pressed locks (state 3, the lock-on camera `0x1401E41A0`); locked, `0x400` pressed unlocks (sound 0x45), or its **release** when KH1's config `*(0x142E1F4D8)+4` is 1 (`0x1402FAFA0`; a hold-style option: Assumption); **`0x800` pressed (L2) steps to the next target** in its lock list. | `0x1402B0610` |
| L2 / R2's `0x400000`: a special camera mode's input (`0x14028C7E0`, mode 0xB) and the end of a held soft target (`0x1402B11B0`); no lock-on. | |

## Built

At Combat = KH1 in Sora's story (the main `CPlayerSora`; Riku's story and Combat = Re:CoM:
Re:CoM's), a wrapper on Re:CoM's lock-on writes its button field `+0x44` right before Re:CoM's
own runs: **R1 (8) when the pad is in use, Re:CoM's R2 (2) when the keyboard / mouse is**
(`crate::lock_on::button`). Re:CoM then locks, unlocks and switches targets as it does (press;
press and release; held with a right-stick flick), now on R1; its pad update writes 2 again the
next frame and nothing else reads the field.

- **The old button (R2)** does nothing at Combat = KH1 on a pad (KH1 gives R2 no lock-on or
  battle action; its camera use belongs to the camera work, `recom-m12-camera-plan.md`).
- **Keyboard and mouse**: Re:CoM's lock-on key, unchanged.
- **Button prompts**: Re:CoM draws button glyphs through its text escapes (`0x140345B40`: `F5 64`
  … `F5 6F` → the glyph table `0x14083F510..0x14083F52A`, filled per device by `0x140349120` /
  `0x1403488E0`). Escape `F5 6B` → `0x14083F522`, by its place in that table R2's glyph:
  Assumption. Whether any of Re:CoM's texts name the lock-on button (the tutorials, `CTechHelp`)
  is in its message files, not in the session: **not changed** (owner question, below).

Sites: hook `0x1401E8DA0` (6 bytes; its only code references are the two calls: rel32 scan; the decomp's other references are unwind data); checked: its
head, the pad update's stores, the three readers and their pad tests, the device detection and
its getter. `check_sites_rs.py`: 1390 sites, 0 mismatches.

Per frame: Re:CoM's lock-on runs once a frame; ours before it: one atomic load and the main-Sora
compare; at KH1 one flag read and one store.

## 1:1 checks

Checked (KH1 decomp + exe): KH1's lock-on button is R1 (raw `0x800` → `0x400`). Checked (Re:CoM
decomp + disasm + exe): Re:CoM's lock-on is R2 and reads only `+0x44`. Tests:
`lock_on::tests::r1_only_on_the_pad_at_combat_kh1`, `sites::tests::lock_on_sites`.
**Not KH1's** (Re:CoM's kept, owner question): how the lock ends and switches (KH1: R1 again
unlocks, L2 steps through the targets; Re:CoM: press and release unlocks, held + right stick
switches). **Not checked**: in game; the device flag's timing against the lock-on in the same frame
(Assumption: the input poll runs first); a controller remapped in Re:CoM's own settings (R1 is
whatever Re:CoM's R1 is).

## What to test (Combat = KH1, Sora's story, a battle, a controller)

1. R1: locks on (Re:CoM's sound and cursor). R1 again, then let go: unlocks. Hold R1 and flick the
   right stick: the next target that way.
2. R2: nothing.
3. Keyboard / mouse: Re:CoM's lock-on key as before; a controller press after it: R1 again (log
   `lock-on: the controller in use …` / `keyboard / mouse in use …`).
4. Combat = Re:CoM or Riku's story: R2 as before; R1 Re:CoM's (card scroll).
5. L1 held (the shortcut list) and R1: the lock (no sleight: Re:CoM's card input is off at KH1).

## Owner questions

1. **KH1's lock behaviour too?** (a) keep Re:CoM's (built: press and release unlocks, hold R1 + right
   stick switches); (b) KH1's: R1 again unlocks at once, L2 steps through the targets (KH1's list
   order); (c) KH1's, with KH1's hold option as a Config row.
2. **Prompts**: does any Re:CoM text (tutorials, help) show the lock-on button? A screenshot tells
   which escape it uses, so it can follow R1 on a controller at Combat = KH1.
