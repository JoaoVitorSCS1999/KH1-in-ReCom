# M7 stage 7.4: KH1's cardless cast and Fire (decision 34)

Owner (2026-10-31): "lets work on M7 … FULLY implemented"; decision 34: spell visuals and sounds
are Re:CoM's, everything else KH1's.

At Combat = KH1, in Sora's story, field and battle: choosing Fire (Fira, Firaga) in KH1's command
menu casts it as KH1 does: KH1's commit gate, its cast motions and windows, the Fire re-cast and
the cast from combo windows, the air hop, the release frame, the MP paid at the release (nothing
cast and nothing paid when the cast is cut before it), KH1's ball (speed, homing, life, radius, one
hit) and KH1's damage. Re:CoM runs it: its magic action `0x13`, its cast clips, its shot runtime
(`CHomingBullet`), its Fire effects and sounds, its hit path. At Combat = Re:CoM, or in Riku's
story, nothing of this runs: Re:CoM's magic cards are untouched.

Code: `crates/kh1-recom/src/magic.rs` (KH1's rules, rows, numbers; tested), `src/win/magic.rs`
(Re:CoM's side), the cast in `movement.rs` (`Kh1Sora::frame`) and `movement/tests.rs`. Research:
`m7-magic.md` § 2.1–2.6, § 3 (kept with the decomp tools, not in the repo).

Steam addresses (image base 0x140000000), both exes. Tags: **Checked (decomp)**, **Checked
(exe)** = bytes / disassembly of the Steam exe, **Checked (files)** = the owner's KH1 files
(`btltbl.bin`, `xa_ex_0010.mset`, `xm_fire01..03.mag`), **Assumption**.

## KH1's cast (what runs at the KH1 setting)

| what | KH1 (Steam) | tag |
|---|---|---|
| **Commit gate** `0x140293480` for a spell: refused in the take-off (state 1); taken when Sora is free or in a combo window (`0x1402A2EC0`); otherwise only **Fire while the release motion `0x3D` / `0x8A` is at frame ≥ 14**. A refusal makes no sound and keeps the list open; a taken command unwinds the menu to Attack. | `0x140293480` | Checked (decomp) |
| **Fire fast path** `0x1402A7510` type 2: a Fire command while the playing request is `0x3D` / `0x8A`, or swing 1 (200) / the lunge (202) **inside its combo window**: plays the release motion at once (blend 8: `movss xmm2,[0x14042F94C]` = 8.0), queues idle (air: fall), air speed × 0.5 in the air, **no hop, no wind-up, no usable / MP test**. | `0x1402A75B6..0x1402A767A` | Checked (decomp + disasm) |
| **Normal cast** `0x1402A3600` → `0x1402A3DB0` (ground) / its air branch: the usable test `0x14028EFD0` (MP `0x1402A4C10` for the cost); ground: wind-up `0x36` (action lock), release `0x3D`, idle (release); air: `0x83`, `0x8A`, fall (release, blend 8), then **a new jump arc of 30 cm** (`0x1402AF960(actor, G+0xE8 = 30)`) and the air velocity × G`+0xF0` = 0.5. Other states drop the command. | `0x1402A3600`, `0x1402A3DB0` | Checked (decomp + files) |
| **Facing** `0x1402A38E0` (command flag 2, every spell): toward the target's part, else the stick (or the facing); speed 0. | `0x1402A38E0` | Checked (decomp) |
| **Cast motions** (Sora's mset, read from the owner's file): `0x36` entry 54, 28 frames, no flags; `0x3D` entry 57, 60 frames, free from 46, root back 15 cm over frames 0–10; `0x83` entry 79, 28; `0x8A` entry 82, 52 frames, free from 46, root back 42.4 cm over 0–10. | `xa_ex_0010.mset` | Checked (files) |
| **The script** (`xm_fire01.mag`, the same at every level): on the wind-up path it yields while the request is `0x36` / `0x83`; on the fast path it waits 8 frames (`L006fa(8.0)`, − frame step per run). Then, if the request is not `0x3D` / `0x8A`, it ends: **no spell, no MP**. Else the ball thread starts (sys 20) and **the MP is paid** (sys 47 → `0x1402A5B60`, the data's `+0x30` = 30 units). | `xm_fire01.mag` 0x520.. | Checked (files: decoded with `magdis.py`; syscalls in the decomp) |
| **When the script runs**: spell threads are tasks of the list `0x142D22D08`, run by `0x14028F6B0`, registered at priority 137000 (`0x21728`); the actors' update runs earlier in the frame (the command menu at 55000). So a run sees that frame's motion update: the ball leaves **in the frame the wind-up ends (28 after the command)**, or 8 after the command on the fast path. | `0x14028F2D0`, `0x140284670` | Checked (decomp); that the actor list runs before 137000: Assumption (its runner's priority not pinned) |
| **The ball** (thread at `L0026e`): from the Keyblade's middle (bone 48, × 0.5) along Sora's facing, **40 cm a frame**, **life 30 frames**, turns **≤ 3° a frame** toward its target's part; with no or a dead target it picks the best enemy **within 500 cm of the ball** each frame (sys 78); starts fading 20 frames before its end; radius 50 (attack data `+0x5C`); ends on its first hit. | `xm_fire01.mag` | Checked (files) |
| **Re-target** (`xm_fire01.mag` loop `L000c2`; sys 78 `0x1402BA440` → `0x1402B0E90`, sys 120 `0x1402BAB10` → `0x140291FF0`): each frame the ball reads the cast's target from its thread arguments. **No target** (the command had none): it picks the best actor within 500 cm of the ball and homes on it this frame; the pick isn't kept, so it picks again next frame. **A target no longer alive**: no homing, **no new pick** (straight on). The pick: actors that are live, not flagged `+0 & 0x200`, on a side other than 0 and the caster's (`0x140291AB0`); `d` = 3D distance from the ball to the actor's position `+0x10`, taken when `d < 500`; score `d × (1 + 2 × (1 − cos))`, `cos` = the clamped cosine between the ball's direction and the way to the actor (0 for a zero vector); the first in range is taken, later ones only with a smaller score. | `0x1402B0E90` | Checked (files + decomp) |
| **Damage** `0x1402BFA80`: power (Fire 20 / Fira 28 / Firaga 36) + max MP (stat source 1), the curve on power − Defence, the target's floor / cap, × resist[2] (fire); no crit. Kind 3 (flinch). MP charge `+0x2C` = 0. | btltbl `0x5F28` / `0x5F98` / `0x6008` | Checked (files + decomp) |

Units: both games step 60 Hz frames (KH1 1.0 per frame; Re:CoM 50 ticks per frame); KH1 cm, Re:CoM
m (× 0.01). Checked (decomp + exe; `recom-timing.md`, research § 1).

## In Re:CoM

### The cast is KH1's frame on Re:CoM's magic action

As a KH1 swing is KH1's frame on Re:CoM's attack action `0xA` (M5):

1. **Command.** The command menu's Magic list (`command_menu.rs`) applies KH1's gate (World
   `gate_any` / `gate_fire`, read from the KH1 frame as last left) and issues the spell
   (`win::magic::issue`, the entry point the shortcuts of 7.12 will call). The request-bits
   wrapper runs before Sora's state exe, so Sora's KH1 frame of the same frame takes it
   (`FrameInput::magic`); a command no KH1 frame takes is dropped at the next frame's start.
2. **KH1's command** in `Kh1Sora::frame` (`Kh1Sora::command_magic`): the gate again (exact: after
   the frame's motion update), the facing, the fast path or the normal cast, the motion requests
   (KH1's motion record, as every KH1 move), the hop, `Host::start_cast`.
3. **Re:CoM's action** (`win::magic::start_cast`): `initMagic`'s (`0x1402203D0`) own writes for a
   cast, in its order, without the card: action `0x13` (`+0x408` / `+0x410` / `+0x40C` as
   `0x1401F8D70`), `+0x428 &= 0xFFF7FEF7`, `+0x424 &= ~0x20`, the attack fields, velocity 0,
   `+0x418 &= 0xFD7FFFFF`, `+0x5D8` = base gravity, `+0x604` = 3, the target cursor's busy bit
   (`0x140209800(+0x3F8, 8)`), the homing target (`0x1401F68A0`), `+0x65C` = 0, `+0x478` = 4 in
   the air, `+0x638` / `+0x414` = Re:CoM's attack id (Fire `0x22`), **`+0x3C8` = the KH1 row**,
   `+0x660` = the cardless AP (`+0x43C`, as M5's swings), `+0x668` = the row's kind, `+0x66C` =
   2 (Re:CoM's fire), `+0x418 |= 0x100000`. Checked (decomp). Not written: the air hover
   (`|= 0x2800000`, gravity × 0.16: KH1 hops instead) and the card modifier (no card).
   `initMagic` itself is not called: it reads the card (`+0x7F4` into `BattleCardList`
   `DAT_14087A808`, which is loaded only when a card battle is built, `0x140210100` →
   `0x1401607A0`; not in the field). Checked (decomp).
4. **Each frame.** Re:CoM's Battle exe calls `exeMagic` through Sora's vtable (`0x1401F9B4D`:
   `call [rax+0x150]`). Re:CoM's **Field exe has no case for action `0x13`** (its jump table
   `0x1401FA5F4` sends it to the default `0x1401FA599`: Re:CoM never casts in the field); its
   entry now points at a 13-byte stub in the int3 cave `0x1401FB0E1` (`mov rcx,rbx; call
   0x140217EE0; jmp 0x1401FA599`), what the table's own cases do. Our hook on `exeMagic`
   (`0x140217EE0`) runs KH1's frame for a KH1 cast (`+0x3C8` in our table), Re:CoM's exe for
   anything else. The motion record plays Re:CoM's clips: `0x36` → row `0x37` (`p001_062`),
   `0x3D` → `0x38` (`p001_063`, root motion, as `exeMagic` plays it), `0x83` → `0x3B`
   (`p001_066`), `0x8A` → `0x3C` (`p001_067`, root). The release clip's root travel is delayed
   one update like the swings' (decision 17).
5. **The release** (`Kh1Sora::cast_step`, KH1's script): right after the frame's replay of the
   last motion update (where KH1's script task sees it). Then `win::magic::release`: **Re:CoM's
   spawn's Fire case as a KH1 case**, the same calls with the KH1 row (`0x140222170` case `0x22`,
   Checked (disasm `0x14022220B..0x14022234D`)): the hand point `0x1401F0B30`, the descriptor
   `0x140156790(desc, 2 = CHomingBullet, 3)`, kind / attribute / AP from `+0x668` / `+0x66C` /
   `+0x660`, `+0xC = 0x80000000`, the target `+0x84C`, Sora's angles (`0x1401A4A00`), the effect
   set `0x1405A1730 + row[2] × 0x60` (Fire 3 / Fira 4 / Firaga 5: `fire_all`), the factory
   `0x1401A8A80`, the manager `0x140158DD0`, the mover set-up `0x140157DA0(shot, 0.1, 0.0, 0,
   -1)`. Then `super::mp::pay(30)`. Re:CoM's `CHomingBullet` steps, homes, draws, collides and
   ends the ball.
6. **The end.** When the release motion ends into its queued idle / fall, or a landing,
   walk-off or another command replaces the cast's motion, Re:CoM's `0x1401F6660` ends the
   action (as `exeMagic` does), unless a swing or another cast already took Re:CoM's action
   over. If Re:CoM ran something else (a hit), the next KH1 frame drops the cast: no ball, no MP
   if it hadn't released.

### The KH1 rows (`magic::fire_row_bytes`)

Re:CoM's own row of each level (`0x1407C6C70 + id × 16`: Fire `0x22`, Fira `0x29`, Firaga `0x2A`;
the bytes are checked against the Steam exe at start) with KH1's ball, in the fields Re:CoM's
spawn reads (Checked (disasm)):

| byte | Re:CoM reads | Re:CoM's Fire | KH1 row |
|---|---|---|---|
| `+0` / `+1` / `+2` | clip / blend / effect set | `0x37` / 1 / 3, 4, 5 | the same (decision 34: Re:CoM's visuals and sounds) |
| `+4` | radius × 0.01 m | 20 / 30 / 40 | **50** |
| `+8` | homing gate: homes while the target is nearer (m; `0x1401575D0`) | 10 / 11 / 12 | **255** (KH1 homes at any distance) |
| `+9` | speed × 0.01 m per frame | 15 | **40** |
| `+0xB` | kind | 1 / 2 / 3 | **1** (KH1 kind 3, flinch) |
| `+0xC` | range from the spawn point (m; `0x140156E20`) | 14 | **255** (KH1's life ends the ball) |
| `+0xD` | homing turn, degrees per frame (`0x1401D7F00` × dt × 0.02) | 20 | **3** |
| `+0xE` | life × 100 ticks (our KH1 case reads it; Re:CoM's Fire case uses 12000) | 20 / 25 / 30 | **15** = 1500 ticks = 30 frames |
| `+0xF` | flags | 0 | **`0x80`**: a KH1 row |

The descriptor's flags: Re:CoM's Fire `0x9100` without `0x1000` (Re:CoM's ball homes only once
0.5 m from its spawn point; KH1's from its first frame) and with `4` (the life ends the ball,
`0x140156E20`, as Re:CoM's Thunder and Stop shots): `0x8104`. The fade after the end
(`+0x94`): 20 frames (1000 ticks). Checked (decomp `0x1401575D0`, `0x140156E20`,
`0x140158660`).

### Damage

The enemy-hit wrapper (`0x14017F7E0`) recognises a hit part (`*(enemy+0xF8)+0x40`, 0x120 bytes,
hit flag `+0x90`) whose attacker (`+0x98`) is one of our balls (the last 16 shot ids,
`bullet+0x20`) and writes KH1's spell damage as the part's value (`+0x92`, as M5's swings):
power + max MP through KH1's curve with the twin's (or proxy's) Defence, floor, cap and fire
resistance (`win::stats::kh1_spell_hit`, `magic::spell_damage`). Re:CoM's own path then applies
its weak part and boss falloff (decision 13). **Re:CoM's own fire resistance is not stacked on
KH1's**: for that call the part's attribute (`+0x84`) reads `0xFF`, which Re:CoM's table
(`0x14017F440`: attributes 0..4 from the enemy's `@ENP`) answers with 100 %; it is put back
after. Checked (decomp `0x14017F7E0`, `0x14017F440`). Open question 2. The card-interrupt bit
as for swings (decision 14). Spells charge no MP. The trace's hits column: `…:fire1:kh1 <n>`.

### Re-target (stage 7.4b, decision 36)

Owner (2026-10-31, decision 36): "Fire re-targets as KH1". KH1's rule (above) is built as the
script has it. Note: the decision's summary said "a ball with no live target"; KH1's script
re-picks only for a ball **cast with no target**; a ball whose target dies flies straight and
never picks again (Checked (files): `L000fc` picks only when the thread's target argument is 0;
`L0013e` clears the target for the frame when sys 120 says it's dead, and `L0015c` then skips
the homing). Re:CoM's `CHomingBullet` already does the second half the same way (below), so only
balls cast with no target get the wrapper.

- **Where:** every Re:CoM bullet runs a flight state (`CStateTemplate<CBullet>` at `bullet+0x2B0`,
  set by the bullet init `0x140157E60`: `0x1401585D5` / `0x140158609`, current via `+0x68`), whose
  exe member pointer `+0x2D0` (this-adjust `+0x2D8` = 0) is the vcall thunk `0x1401560B8`
  (`mov rax,[rcx]; jmp [rax+0xA8]`); the state's exe `0x140156740` calls it with the bullet each
  frame (Re:CoM's FSM `0x140242F50`). For a `CHomingBullet` that is the homing step
  `0x1401575D0` (vtable `0x140525530` + 0xA8). Checked (decomp + disasm).
- **The wrapper:** for **our balls cast with no target** only (`+0x84C` ≤ 0 at the release), right
  after Re:CoM's shot manager made the ball, `win::magic::seek_on` points that ball's own exe
  member at `kh1_ball_exe` (after checking the ball's vtable is `CHomingBullet`'s, the member is
  the thunk and the adjust is 0). Each frame `kh1_ball_exe` writes KH1's pick
  (`magic::retarget::pick`) into Re:CoM's homing target `+0x2F4` (0 when nothing is in range),
  then calls Re:CoM's thunk: Re:CoM's homing step turns the ball ≤ 3° toward the pick's aim point
  (vtable `+0x50`) and rebuilds its velocity, as for any homing shot. Re:CoM's own shots never
  run our code.
- **KH1's inputs in Re:CoM:** the ball's position (its mover `+0x30`) and its direction before this
  frame's turn (the mover's aim angles `+0x50` / `+0x54` → `guard::forward`, the inverse of
  `0x1401A2F10`); the candidates are **Re:CoM's lock-on list** (`LockOnManager` `0x14087B1E0`,
  list `+0x88`), walked and filtered as Re:CoM's own nearest-target pick for homing shots
  (`0x1401CDB90`: the handle resolves, not the shot, `+0x154 & 0x10000`), kept to enemies (type
  `0x1004`) with HP left; the distance is to the enemy's position (mover `+0x30`), as KH1 measures
  to the actor's `+0x10`. Metres: KH1 cm → Re:CoM is (x, −y, −z) / 100, a rotation and a uniform
  scale, so 500 cm = 5 m and the cosine is unchanged.
- **A ball cast with a target** keeps Re:CoM's homing unchanged: it homes while the target's
  handle resolves (`0x1402AD350`), then clears `+0x2F4` and flies straight for good (Checked
  (decomp `0x1401575D0`)). That is KH1's dead-target rule, with "dead" = "removed".
- **Cost:** one lock-on list walk (≤ 64 entries) per frame per ball cast with no target, only while
  it flies (life `+0x160` > 0); nothing for any other shot.
- **Logs / trace:** `magic: re-target: shot N cast with no target: …` once per such ball (40
  lines in all), and each change of its pick: `rt shot N -> <handle> (d m)` / `rt shot N -> none`,
  also as F9 trace events in the cast column.

Sites (`sites::MAGIC_CALLEES` / `MAGIC_CHECKED` / `MAGIC_RETARGET_PTRS`, bytes from the Steam
exe; `magic_sites` checks the jumps between them):

| address | bytes | use |
|---|---|---|
| `0x1401560B8` homing thunk | `48 8B 01 FF A0 A8 00 00 00` | called (the ball's own exe before the wrapper) |
| `0x1405255D8` `CHomingBullet` vtable `+0xA8` | → `0x1401575D0` | compared at install (no re-target otherwise) |
| `0x1401575D0` homing step | `40 53 48 83 EC 50 48 8B 05 EB B4 70 00 48 33 C4 48 89 44 24 30 83 B9 F0 02 00 00 00` | checked only |
| `0x1401585D5` / `0x140158609` bullet init | `48 8D 05 DC DA FF FF` / `0F 11 8E D0 02 00 00` | checked only (the thunk lands in `+0x2D0`) |
| `0x140156740` `CStateTemplate<CBullet>` exe | `48 8B 51 20 48 8B C1 48 85 D2 74 0B 48 63 49 28 48 03 48 08 48 FF E2 C3` | checked only (calls the member with the bullet) |
| `0x1401CDBCF` / `0x1401CDC26` Re:CoM's nearest pick | `4C 8B 35 0A D6 6A 00` / `49 8B 9E 88 00 00 00 … F7 80 54 01 00 00 00 00 01 00` (47 bytes) | checked only (the manager global, the walk and its filter) |

1:1 checks (7.4b):

- **Checked (files + decomp)**, in `magic::tests::fire_retarget_is_kh1s_pick`: the 5 m strict 3D
  range, the score (ahead × 1, beside × 3, behind × 5), a farther enemy ahead beating a nearer one
  behind, no pick out of range, ties keeping the first, the pick following the ball's direction,
  the cosine's clamp and its invariance under the KH1 → Re:CoM axis flip.
- **Checked (exe)**: the 9 new sites' bytes and the vtable slot (script against the Steam exe).
- **Assumptions:** (1) Re:CoM's bullet flight-state exe runs before its mover moves it in the
  frame (KH1: pick, turn, move), as 7.4's homing already assumes; (2) KH1's `+0 & 0x200` excluded
  actor is a defeated one (set at `0x1402A8567` with the prize record), matched by Re:CoM's
  lock-on flag and HP > 0; (3) only enemies count (KH1's other-side actors; Re:CoM's lock-on list
  also holds chests, doors and NPCs in the field, which KH1 has no actors for); (4) KH1's actor
  array order vs Re:CoM's list order only matters for exact score ties; (5) a dying KH1 enemy
  stays "alive" for sys 120 until it is removed (`0x140292390` sets `+0x374 & 2`), as a dying
  Re:CoM enemy keeps resolving until deleted.
- **Not checked:** in game.

## Per frame

Nothing of ours per frame outside a KH1 cast (one atomic load in the request-bits wrapper for the
pending command; the `exeMagic` hook only runs in action `0x13`). During a cast: KH1's frame, as
for any KH1 move. The ball is Re:CoM's (no code of ours per frame); a hit costs a scan of the
enemy's parts against 16 ids.

## Logs and trace

- Start: `magic: KH1's cardless cast at combat = kh1 in Sora's story (M7 stage 7.4): Fire / Fira /
  Firaga on Re:CoM's magic action 0x13 with KH1 rows (3 rows), …; Field exe case 0x13 -> exeMagic`.
- Up to 80 lines: `magic: cast(fire1)` / `cast(fire1 fast)` / `cast(fire1 air)`,
  `cast-refused(fire1 not now | mp | state | no Re:CoM row | spell not in this stage)`,
  `magic: release fire1: shot id N at (x,y,z), speed 0.40 m/f, life 1500 ticks, turn 3°/f,
  radius 0.50 m, target T`, `release(fire1 f28 shot N)`, `cast-cut(fire1 f12 req 0x7)`,
  `mp: paid 30 (Fire release): …`.
- Up to 40 hit lines: `magic: hit fire1 on 0x…: KH1 48 (max MP 3), Re:CoM took 48 -> HP …`.
- Command menu: `cast Fire [0x7]: issued`, `not-now Fire [0x7]: KH1's gate refused it this frame`.
- F9 trace, events column: `…; cast -` or `cast fire1 f<frames since the command> [fast w<wait>]
  [air] [rel] [cut] [shot <id>]` and this frame's cast events; the MP token shows `pay30`.

## Sites (`sites::MAGIC_*`, bytes from the Steam exe)

| address | bytes | use |
|---|---|---|
| `0x140217EE0` exeMagic | `48 89 5C 24 08 48 89 74 24 10` | hooked (two whole `mov`s, no RIP-relative or branch) |
| `0x1405A9A88` Sora's vtable `+0x150` | → `0x140217EE0` | compared at install |
| `0x1401FA63C` Field exe jump table, action 0x13 | `99 A5 1F 00 D8 A2 1F 00` | patched: → `0x1401FB0E1` |
| `0x1401FB0E1` int3 cave | 15 × `CC` | patched: the 13-byte stub |
| `0x1401FA1B0` Field exe switch | `8B 83 0C 04 00 00 FF C8 83 F8 1E 0F 87 D8 03 00 00 4C 8D 05 38 5E E0 FF 48 98 41 8B 8C 80 F4 A5 1F 00` | checked only (player in `rbx`, table base) |
| `0x1401FA599` Field exe default | `8B 8B 60 04 00 00 29 8B BC 07 00 00` | checked only |
| `0x1401F9B4D` Battle exe case 0x13 | `48 8B 07 48 8B CF FF 90 50 01 00 00` | checked only |
| `0x140156790` descriptor init | `33 C0 89 11 89 41 08 44 89 41 04 C7 41 0C 02 00 00 80` | called |
| `0x1401F0B30` hand point | `40 53 48 81 EC 80 00 00 00` | called |
| `0x1401A4A00` actor angles | `48 8B 81 F0 00 00 00 48 83 C0 50 C3` | called |
| `0x1401A8A80` entity factory | `40 53 48 83 EC 30 48 C7 44 24 20 FE FF FF FF` | called |
| `0x140158DD0` shot manager | `41 56 48 83 EC 30 48 C7 44 24 20 FE FF FF FF 48 89 5C 24 40` | called |
| `0x140157DA0` shot mover set-up | `48 89 5C 24 08 57 48 83 EC 30 48 8B D9 0F 29 74 24 20` | called |
| `0x1401F68A0` homing target | `48 89 5C 24 08 57 48 83 EC 20 48 8B D9 33 FF 8B 89 4C 08 00 00` | called |
| `0x140209800` target cursor busy | `85 D2 74 14 83 89 F8 00 00 00 08 8B 81 D0 00 00 00 89 81 D8 00 00 00 C3` | called |
| `0x1407C6E90` row 0x22, `0x1407C6F00` rows 0x29–0x2A | Re:CoM's Fire rows | the KH1 rows' templates |

Already checked by earlier stages and called: `0x1401F6660` (end action), `0x1401F3C00` (ground
test), `0x1402AD350`. All 16 new sites compared with the Steam exe's bytes (script).

## 1:1 checks

- **Checked (decomp + files)**, replayed by tests (`movement::tests`, "M7 stage 7.4";
  `magic::tests`): the ball released in KH1's frame 28 of a ground cast (our handler sees it one
  handler later, as every KH1 motion update); Sora locked until frame 46 of the release motion,
  then free, and Re:CoM's action ended once at the motion's end; **Fire re-cast taken from frame
  14 of the release motion and its ball 8 frames after the command**; Fire from swing 1's combo
  window skips the wind-up (Re:CoM's attack action ended first); an interrupted cast (Re:CoM ran
  something else, or an air cast landed first) casts nothing and pays nothing; without MP the
  command does nothing; the air cast's 30 cm arc and halved air speed; the rows, the shot numbers
  (0.40 m a frame × 30 frames = 12 m = KH1's command range), the damage (Fire + max MP 3 vs
  Defence 5 = curve[18] = 48; fire resistance; the cap), the cast motions' frame counts and free
  keys, the Field stub's bytes.
- **Checked (exe)**: every new site's bytes and the vtable slot; Re:CoM's Fire rows; the effect
  sets 3/4/5 hold `fire_all`.
- **Not checked**: nothing seen in game; no KH1 recording of a cast (a KH1 video of Fire, a Fire
  chain and an air Fire, frame-stepped, would confirm the 28 / 8 / 14 numbers on screen).

## Not 1:1 by design or yet (and why)

1. **Homing re-acquire**: built in 7.4b (above), KH1's rule.
2. **The homing turn** is Re:CoM's: pitch and yaw each turn ≤ 3° a frame (`0x1401D7F00` →
   `0x1401D4730` per angle); KH1 turns the direction by ≤ 3° in all (sys 24). A target off both
   axes is turned up to √2 × 3° a frame. Assumption (`0x1401D4730` not read in full).
3. **The fade.** KH1's ball fades over its last 20 frames while it flies; Re:CoM's stops at its
   end and fades for 20 frames (`+0x26C`). Hit-wise the ball lives KH1's 30 frames. Assumption:
   Re:CoM's fade state no longer collides (`+0x154 |= 0x40000000`).
4. **The start point** is Re:CoM's hand point (the weapon), KH1's the Keyblade's middle.
   Assumption: close enough to see no difference.
5. **Chained Fires** come every 15 frames in our model (taken when the release motion's update
   has reached 14, the frame after); the research note estimated 14. The KH1 recording settles it.
6. **±1 frame**: KH1's actor list running before the spell task in the frame (priority 137000) is
   read from the task registrations, not traced.

## Assumptions

1. Re:CoM never sets action `0x13` in the field (its magic needs a card play, battle only;
   `0x1401EEDA0` dispatches request bit `0x20`, which only a card play sets), so the new Field
   case changes nothing at the Re:CoM setting.
2. The ball's life `+0x160` counts down by dt in Re:CoM's actor update (as the player's
   `+0x160` timer `exeMagic` sets).
3. `fire_all`'s effects are loaded in the field (Re:CoM never casts there): if not, the ball
   flies and hits but isn't drawn. Open question 3.
4. Re:CoM's clips `p001_062` / `p001_063` / `p001_066` / `p001_067` are KH1's motions baked at
   30 fps (the frame counts match: research § 3.5, Checked (files)); the release clips carry
   KH1's root travel.
5. Re:CoM's target `+0x84C` stands for KH1's command target, and its aim point (vtable `+0x50`)
   for KH1's target part.

## Open questions (owner)

1. Fire's re-acquire: answered (decision 36, built in 7.4b).
2. Element resistance on spell hits: answered (decision 36: KH1's only).
3. **Fire in the field**: if Re:CoM's Fire effects aren't loaded outside battle (to see in game),
   (a) cast without the visual there; (b) load Re:CoM's magic effect group at room load (a new
   resource request; needs research); (c) Fire only in battle at first.

## What the first in-game test must check

1. Log: the `magic:` start line (3 rows), `hooked exeMagic (0x140217EE0)`, the two patch lines
   (`magic: Field case stub`, `magic: Field jump table 0x13`).
2. `kh1_all_spells = on`, in battle with a Shadow: Magic → Fire. Sora turns to the target, plays
   Re:CoM's Fire wind-up and release, the ball leaves about half a second after the command,
   flies quickly (12 m in half a second), curves slightly toward the target, hits once, Re:CoM's
   Fire hit effect and sound. The MP bar loses 30 units (a third of the top point at 3 MP). Log:
   `cast(fire1)`, `release(fire1 f28 shot …)`, `mp: paid 30`, `hit fire1 … KH1 …`.
3. Fire again: the menu is back on Attack after each cast (KH1's unwind), so open Magic again
   and confirm Fire while the release motion plays: from about a quarter second after the ball
   left, it is taken at once with no wind-up and its ball leaves 8 frames later (the shortcut of
   7.12 makes the chain easy). MP runs out after 6 Fires at 3 MP (180 units: point k is 30·k).
4. Swing 1 then Fire inside its combo window: no wind-up, the ball 8 frames later.
5. Fire in the air (after a jump): Sora pops up a little and slows; landing before the ball
   leaves cancels it (no MP lost).
6. Get hit during the wind-up: no ball, no MP lost.
7. With MP too low: Fire greyed, confirm buzzes.
8. In the field (no enemy): the ball flies straight 12 m and fades; check that it is drawn
   (open question 3).
9. F9 trace: `cast fire1 f…` tokens, the release frame, `pay30`, the hits column `:fire1:kh1 …`.
10. Combat = Re:CoM: Re:CoM's Fire cards work as before; Riku's story: unchanged.
11. (7.4b) Re-target: with no target (turn away from the enemies, or in the field next to a
    field enemy without a target cursor), cast Fire so the ball passes within 5 m of an enemy: it
    bends toward it (≤ 3° a frame) and the log shows `re-target: shot N cast with no target` and
    `rt shot N -> <handle>`; F9 trace: `rt shot …` in the cast column. With a target that dies
    before the ball arrives: the ball flies straight on and never turns to another enemy.
