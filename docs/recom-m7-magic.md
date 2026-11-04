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
| **Cast motions** (Sora's mset, read from the owner's file): `0x36` entry 54, 28 frames, no flags; `0x3D` entry 57, 60 frames, free from 46, root back 15 cm over frames 0–10, then 5 cm forward to −10 cm at frame 60 (7.4c); `0x83` entry 79, 28; `0x8A` entry 82, 52 frames, free from 46, root back 42.4 cm over 0–10. | `xa_ex_0010.mset` | Checked (files) |
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
   **A won battle:** when Re:CoM's win is pending (`player+0x428 & 4`, the last enemy died during
   the cast, e.g. to Thunder's bolts), that end action enters Re:CoM's win action `0x1C` instead
   of the idle (`0x1401F6794` → `0x1401F1B20`); the KH1 frame then yields: no idle motion or
   action after it (`Kh1Sora::yield_to_recom`). Checked (decomp + tests);
   `recom-m6-battles.md` § Battle end.

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

## Stage 7.5: Blizzard / Blizzara / Blizzaga

Owner (2026-10-31): M7 "FULLY implemented"; decision 34 (Re:CoM's visuals and sounds), 36 (KH1's
resistances only). At Combat = KH1 in Sora's story, Blizzard from the command menu casts as KH1:
KH1's cast, KH1's seven shards on Re:CoM's own Blizzard shots.

### KH1 (what runs)

| what | KH1 (Steam) | tag |
|---|---|---|
| **Cast** `0x1402A3DB0` / `0x1402A3600`: ground `0x36 + 1` = `0x37` (blend 0), queued idle (release); air `0x83 + 1` = `0x84`, queued fall (release, blend 8), the 30 cm hop and air speed × 0.5, as Fire's air cast. No fast path (only Fire has one). | disasm `0x1402A3E92..0x1402A3ED7` | Checked (decomp + disasm) |
| **Motions**: `0x37` entry 55, 86 frames, free from 68, no root travel; `0x84` entry 80, 78 frames, free from 68. | `xa_ex_0010.mset` | Checked (files, read with `kh1-data`) |
| **Script** `xm_blizzard01..03.mag` (one script; the level passes the shard speed 20 / 30 / 40): charge effect; **wait 28** (`L0086e(28)`); then the request must still be `0x37` / `0x84` (55 / 132), else it ends: **no spell, no MP**; then the bone point (bone 48, scale 1), the spawner thread, **then the MP** (sys 47, 30 units). | `L006dc` | Checked (files: `magdis.py`) |
| **Shards** (spawner `0x4A`, shard thread `0x238`): six on a cone, the local vector (0.383, 0, 0.924) (22.5° off the facing) turned 60° about the facing axis each (sys 31), each starting (x, y) × 50 cm off the axis (19.15 cm, sys 93 / 10 / 7); then one straight ahead from the bone point. Each: **speed 20 / 30 / 40 cm a frame, life 26 frames**, its own attack (radius 70 / 90 / 120, data `+0x5C`), **one hit bit shared by all seven**; a shard ends on its attack's first hit. | `xm_blizzard01.mag` | Checked (files) |
| **Glide** (shard loop `L0029e`): x, z move by direction × speed; a ray (sys 77 → `0x140299530`, any surface) from the shard straight down by 50 + its own vertical step finds a surface → the shard sits 50 cm above it; none → it moves by its own vertical step. | `0x140299530` | Checked (files + decomp) |
| **One hit per enemy per cast**: the collision pass `0x1402B3BE0` skips an actor whose `+0x3E0` holds the attack's bit; a hit sets it (`0x1402BF5E0`); after each pass every actor keeps only bits some live attack holds (`0x1402B4460`), and a deleted attack frees its bit only when no other attack holds it (`0x1402B3560`). The seven shards share one bit, so an enemy one shard hit is skipped by the others for the whole cast, and a skipped shard flies on through it. Data `+0x50` (re-hit interval) is 0. | `0x1402B3BE0`, `0x1402B3560` | Checked (decomp + files) — the research note's Assumption is confirmed |
| **Damage** `0x1402BFA80`: power 22 / 27 / 34 + max MP (stat source 1), the curve on − Defence, floor / cap, × resist[3] (ice); kind 3 (flinch). | btltbl `0x6078` / `0x60E8` / `0x6158` | Checked (files + decomp) |

### In Re:CoM

- **Cast:** the same KH1 frame on Re:CoM's magic action as Fire (`Kh1Sora::command_magic`, the cast
  motions in KH1's record, `release_wait` = 28 for the script's wait); Re:CoM's clips: `0x37` → row
  `0x3A` (`p001_065`, 43 frames at 30 fps = 86), `0x84` → `0x3E` (`p001_069`, 39 = 78), both
  without root motion (KH1's have none). Re:CoM's action ends when the motion ends into its idle /
  fall (86 / 78) or anything replaces it. Checked (exe: the action table's names).
- **KH1 rows** (`magic::blizzard_row_bytes`): Re:CoM's rows 0x23 / 0x2B / 0x2C (checked against the
  exe at start) keep clip, blend, `bliz_all` sets 6 / 8 / 10 and `+5`; `+4` radius 70 / 90 / 120,
  `+9` speed 20 / 30 / 40, `+0xB` kind 1, `+0xC` 255 (no range), `+0xE` 13 (26 frames), `+0xF` 0x80.
- **Release** (`win::magic::release_blizzard`): Re:CoM's Blizzard case (`0x140222170` case `0x23`,
  Checked (decomp + disasm `0x1402223F4..0x1402224F3`)) made seven times with KH1's shards: class 2,
  kind / attribute (Re:CoM's ice 3) / AP from the action, the hand point + KH1's offset, the
  shard's pitch / yaw (`guard::aim_angles` of `magic::blizzard_shards`, from Sora's yaw), speed,
  life `+0x3C` and `+0x38` = 1300 ticks, radius, **the straight shard with the row's set, the cone
  shards with the next set (`0x1405A1790`, Re:CoM's side shards)**, flags `0x4000000` (Re:CoM's)
  `| 0x100 | 4` (a hit or wall ends it; the life ends it), and **the first shard's hit serial
  (`0x140156770`, `coll+0x70`) in every later shard's `desc+0x74`, as Re:CoM's own Blizzard does**.
  Each shard's mover gets Re:CoM's Fire set-up (`0x140157DA0(shot, 0.1, 0, 0, −1)`: a 10 cm mover,
  so gliding 50 cm up it never touches the floor; the attack keeps KH1's radius). Then MP 30.
- **Glide** (`kh1_shard_exe`, each shard's own flight-state exe as for 7.4b's re-target): Re:CoM's
  segment test `0x140170330` on the shard mover's own collision object (`mover+0x158`, parameter
  `+0x134`, as Re:CoM's movers and our Sora floor probe use it), from the shard's height at this
  frame's x, z, down by `0.5 − dy` m; the vertical velocity then puts it 0.5 m above the hit, or on
  its own slope (`magic::blizzard_glide_y`). Then Re:CoM's step (the thunk; no target, so the
  homing step returns at its first check).
- **One hit per enemy per cast: Re:CoM's own rule, unchanged.** The apply `0x14016F1C0` skips a
  body whose hit-once ring holds the attack's serial (`0x14016F960`) and records the serial on the
  body it hits (`0x14016FE20`); the enemy's hit handler (`0x140154EB0`, from `0x14017F7E0`) takes
  **one** hit body per run, clears every body's hit flag and **records that serial on all the
  enemy's bodies**. With the seven shards sharing one serial (as Re:CoM's own Blizzard), an enemy
  is hit once per cast, and a shard reaching an enemy already hit is not tested against it and
  flies on: KH1's per-actor shared bit. Checked (decomp). Nothing of ours.
- **Damage:** as Fire (`spell_hit`): KH1's power + max MP through the twin's / proxy's numbers and
  KH1's ice resistance only (Re:CoM's ice % not applied: the part's attribute reads 0xFF for the
  call).

Logs: `magic: release blizzard1: 7 shards from (…) yaw …, speed 0.20 m/f, life 1300 ticks, radius
0.70 m, serial N`, `release(blizzard1 f28 shot N)`, `mp: paid 30 (Blizzard release)`, hit lines
`hit blizzard1 on 0x…: KH1 …`; F9 trace: `cast blizzard1 f… w…`, the hits column `:blizzard1:kh1 …`.

Sites (7.5):

| address | bytes | use |
|---|---|---|
| `0x140156770` shot hit serial | `48 8B 81 A8 02 00 00 8B 40 70 C3` | called |
| `0x1402224A7` Blizzard case: serial | `E8 C4 42 F3 FF 89 45 C4` | checked only (its call lands on `0x140156770`) |
| `0x1402224E9` Blizzard case: side sets | `48 8D 0D A0 F2 37 00` | checked only (= `0x1405A1790`) |
| `0x1407C6EA0`, `0x1407C6F20` rows 0x23, 0x2B–0x2C | Re:CoM's Blizzard rows | the KH1 rows' templates |
| `0x140170330` segment test | (checked by M4) | called |

1:1 checks (7.5):

- **Checked (decomp + files)**, replayed by tests: the shards released at KH1's frame 28 (handler
  29), Sora free at 68 of the 86-frame motion and Re:CoM's action ended once; a cast cut before 28
  casts nothing and pays nothing; no fast path; a Fire refused during a Blizzard's lock
  (`movement::tests` "M7 stage 7.5"); the rows, attack data, cost, flags; the cone (six at 22.5°,
  60° apart about the axis, two level / two up / two down, each starting 19.15 cm off the axis,
  the straight one from the point); the glide's ray length and heights (`magic::tests`).
- **Checked (exe)**: the new sites, the rows, the clips' names and frame counts.
- **Assumptions:** (1) Re:CoM's hand point stands for KH1's bone 48 at scale 1 (Fire's
  Assumption 4); (2) the segment test's hit height is the surface point under the shard (the
  mover's 10 cm radius may find it a little earlier than KH1's thin ray); (3) a shard's walls end
  it as KH1's attack's wall flag ends Fire's ball (Re:CoM `0x100` on a wall contact); (4) Re:CoM's
  flight-state exe runs before the mover moves the shard (as 7.4b); (5) the enemy's hit handler runs
  between two shards' contacts with one enemy in different frames (it runs every enemy update;
  Checked (decomp) that it spreads the serial, not traced in game); (6) Re:CoM's `bliz_all` effects load in the field (as Fire's open
  question 3).
- **Not checked:** in game.

In-game checklist (7.5):

1. Log: the `magic:` start line names Blizzard (7.5) and 6 rows.
2. In battle: Magic → Blizzard: Re:CoM's Blizzard wind-up clip, about half a second later seven
   ice shards fan out (two level, two up, two down at a narrow angle, one straight), skimming
   about half a metre above the floor, for about 5 m (26 frames × 20 cm); each stops on the first
   enemy it hits; an enemy takes **one** shard however many cross it. MP bar −30 units.
3. Blizzara / Blizzaga: faster shards (30 / 40 cm a frame: 7.8 m / 10.4 m) with bigger hit radii.
4. On a slope or stairs: the level shards follow the floor; the rising ones keep rising.
5. Air Blizzard: the hop; landing before frame 28 cancels it (no MP).
6. Hit during the wind-up: no shards, no MP.
7. Sora can move again about 1.1 s after the command (frame 68), not at the motion's end.
8. F9 trace: `cast blizzard1 f… w…`, `release(blizzard1 f28 shot …)`, `pay30`, the hits column.
9. Combat = Re:CoM: Re:CoM's Blizzard cards unchanged; Riku's story unchanged.

## Stage 7.6: Thunder / Thundara / Thundaga

At Combat = KH1 in Sora's story: KH1's Thunder cast and strike thread, each bolt Re:CoM's own
Thunder bolt and hit path.

### KH1 (what runs)

| what | KH1 (Steam) | tag |
|---|---|---|
| **Cast**: ground `0x36 + 2` = `0x38`, idle queued (release); air `0x85`, the fall (blend 8), the 30 cm hop, air speed × 0.5. Motions: entry 56, **68 frames, no free key** (locked to the end), no root; air entry 81, 68. | `0x1402A3DB0`, `0x1402A3600` | Checked (decomp + files) |
| **Command** (`L004b6`): area = cylinder **radius 600, height 1000 (±500)**; centre = the command target's part if alive, **else Sora's position**; distances from the target (alive) else Sora. The area marker effect alone is drawn 2 m ahead along the camera's horizontal forward (sys 1.5 / 1.6, × 200) and on the ground (sys 77). (The research note's "no target: 2 m ahead" is the marker; the area itself is around Sora: Checked (files) — `0x59C` sets the centre to the caster's `+0x10`, `0x24C` adds the offset only to the effect point.) | `xm_thunder01.mag` | Checked (files) |
| **Tint**: `L00064` (sys 52) dims the screen colour 1 → 0.5 over 20 frames; back over 20 frames 40 frames after the release, or at once on an interrupt. | sys 52 `0x1402BC190` | Checked (files); not built (below) |
| **Release**: wait **30** (`L00c8e(30)`), the request must be `0x38` / `0x85` (56 / 133), else no spell, no MP; the strike thread starts, then **MP 1 point** (100). | `0xC22..0xC70` | Checked (files) |
| **Strikes** (thread `0x17E`): the centre moves to the target's part if still alive; every enemy (sys 65; up to 30 slots) with its distance into a list, **sorted nearest first** (selection sort, strict `<`); down the list: an enemy whose hurt bodies overlap the area (sys 90 → `0x1402B6AF0`: alive, each unmasked body's shape vs the cylinder) **is struck directly** (sys 160 → `0x1402BBB70`: the attack applied onto it, no collision; contact = its part), then **8 frames** (`L00c8e(8)`); one outside is passed over at once. Each enemy once; the first strike in the release frame. | `0x17E..0x384` | Checked (files + decomp) |
| **Damage**: power 16 / 20 / 26 + **max MP × 2** (stat source 3), the curve, floor / cap, × resist[4] (thunder); kind 3. | btltbl `0x61C8` / `0x6238` / `0x62A8` | Checked (files + decomp) |

### In Re:CoM

- **Cast**: as Blizzard, with `release_wait` 30; Re:CoM's clips `0x38` → row `0x39` (`p001_064`),
  `0x85` → `0x3D` (`p001_068`), 34 frames at 30 fps = 68, no root. KH1 rows from Re:CoM's 0x24 /
  0x2E / 0x2F (checked at start): Re:CoM's clip, `_thunder_all` sets 12 / 13 / 14 and its bolt's
  speed / radius / range kept; `+3` 0, `+0xB` 1 (KH1 kind 3; Re:CoM's Thunder kinds 0x1C–0x1E
  unused), `+0xF` 0x80.
- **Command** (`win::magic::thunder_command`, from `start_cast`): Re:CoM's target `+0x84C` (KH1's
  command target, Assumption 5) if it is an alive enemy (type 0x1004, HP > 0) → centre = its aim
  point (vtable `+0x50`); else Sora's position.
- **Charge (M7 f4):** at the command, `initMagic` case 0x1C's own charge, as Cure / Gravity / Stop /
  Aero get theirs: the row's set's effect id 0 (the set's `+0x18` name, `_thunder_all`) on the
  Keyblade's (`+0x3D0`) bone 2 at (−0.8, 0, 0) (`0x1407C7E50`), **kept on the Keyblade** (`+0x4C |=
  4`, as Re:CoM's Thunder does; Cure's and Aero's charge (`0x14022070A`) isn't kept, Stop's is).
  Checked (disasm `0x14022062B..0x140220697`, site `initMagic: Thunder charge (0x14022062B)`, 108
  bytes, checked only; its calls land on `0x1402ABB20`, `0x1401A2D50`, `0x140257EF0`). KH1's script
  also starts its charge effect at the command (`xm_thunder01.mag`); its look is Re:CoM's (decision
  34). Not taken from case 0x1C: its `0x1401E5F80(player, AP, row[4] × 0.1, row[0xC])` and
  `0x1401F7D80(player, 0x1407C5D40)` (Re:CoM's own Thunder targeting and its camera / slowdown:
  not read; KH1's Thunder picks its own targets) and `+0x504` (the bolt height, set by our release).
  In game: Re:CoM's Thunder charge on the Keyblade from the command to the end of the cast clip.
- **Release** (`release_thunder`): the centre moves to the target's aim point if it is still
  alive; Re:CoM's lock-on list's enemies (type 0x1004, HP left; without the lock-on flag: the
  bodies decide, as KH1's sys 90) with their distances from the target / Sora → KH1's order
  (`magic::thunder_order`); Re:CoM's Thunder hand effect (id 1); the first run of the strike
  thread; MP 1 point.
- **Strike thread** (`magic::Strikes`, KH1's script state): one run at the release, then one per
  frame at Sora's frame start (the request-bits wrapper calls `strikes_frame`: one atomic load when
  no Thunder is striking). An enemy is in the area when one of its live hurt bodies (Re:CoM's
  shapes as its primitives `0x1401777E0` / `0x140176FA0` read them: sphere `+0x20`, column `+0x10`
  → `+0x20`, each y + `+0x74`, radius `+0x70`) comes within its radius of the cylinder
  (`magic::body_in_area`). Several Thunders' threads run side by side, as KH1's.
- **A bolt** (`strike`, KH1's sys 160): **Re:CoM's Thunder case for that enemy** (`0x140222170` case
  `0x24`, Checked (disasm `0x140222748..0x14022293C`)): its bolt effect (id set − 10) at the
  enemy's aim x / z and 6 m under Sora's height (`+0x504`), its `CBullet` (class 1, flags `0x47`,
  life 800, pitched down π/2, the row's speed / range / radius × 0.1, the mover set-up 0.1) from
  Sora's height there, with the cast's kind / attribute (Re:CoM's thunder 4) / AP. Then **Re:CoM's
  own hit path, directly**: the bolt's `CCollAttack` is switched off (its vtable `+8`, the "off"
  Re:CoM's init uses for a held collision; also flag `0x200000` with a held count, so Re:CoM's own
  exe keeps it off) and its **apply** (vtable `+0x38` = `0x14016F1C0`) is called on the enemy's
  first live hurt body: the call Re:CoM's collision pass `0x14016EAD0` makes once its overlap test
  passes. Re:CoM records the hit (flag, value, the bolt's id, its ring, its hit effect), and the
  enemy's own update runs its hit handler (our KH1 damage on it, as for Fire, with KH1's thunder
  resistance only), its reaction and its hit-once spread.
- **Why the direct apply, not a bolt that Re:CoM collides:** KH1 strikes each listed enemy
  directly, once, in order, wherever its bodies are; a Re:CoM bolt (a 1 m sphere falling from
  Sora's height) would also touch neighbours within 1 m (hit early, or twice with separate
  serials) and could miss a flying or small enemy. The apply is Re:CoM's own function for "this
  attack touched this body"; only the overlap test is skipped, as KH1's sys 160 skips collision.
  No hit test of ours: the area test picks the targets, as KH1's script does.
- **Order quirk found:** the bullet init tests the held collision (`0x1401583E8`: flag bit 21 and
  `+0x268` > 0) **before** it copies `desc+0x8C` into `+0x268` (`0x140158544`), so the hold never
  applies at init; hence our explicit "off" right after the bolt is made. Checked (disasm).

Logs: `magic: release thunder1: area centre (…) r 6 m ±5 m, N enemies listed from (…), first bolt
…`; up to 60 `magic: bolt on 0x…: shot N, Re:CoM's apply -> Some(1)` lines; `mp: paid 100 (Thunder
release)`; hit lines `hit thunder1 on 0x…: KH1 …`. F9 trace events: `bolt <n> e<handle> shot <id>`,
`release(thunder1 f30 …)`, `pay100`; hits column `:thunder1:kh1 …`.

Sites (7.6; `magic_sites` checks the call / RIP targets):

| address | bytes | use |
|---|---|---|
| `0x14014DE80` `CCollAttack` off | `C7 41 10 00 00 00 00 C3` | called (slot `0x140523E30` compared) |
| `0x14016F1C0` `CCollAttack` apply | `40 55 53 57 48 8D 6C 24 B9 48 81 EC D0 00 00 00` | called (slot `0x140523E60` compared, and on each bolt's vtable) |
| `0x14016EC51` collision pass: apply call | `48 8B 06 48 8B D3 48 8B CE FF 50 38` | checked only |
| `0x1401583E8` bullet init: held collision | `8B 86 30 02 00 00 48 0F BA E0 15 73 16 83 BE 68 02 00 00 00 7E 0D 48 8B 8E A8 02 00 00 48 8B 01 FF 50 08` | checked only |
| `0x14022276C` Thunder case: hand effect | 59 bytes ending `E8 79 43 F8 FF` (→ `0x1401A6B20`) | checked only |
| `0x1402227F3`, `0x140222820` bolt effect drop / id | `F3 0F 58 05 31 6B 2E 00` (→ −6.0 at `0x14050932C`), `44 8D 41 F6 48 8B CF` | checked only |
| `0x140222867`, `0x140222898`, `0x1402228BC` bolt pitch / flags / life | `F3 0F 10 0D 81 E8 49 00` (→ π/2 at `0x1406C10F0`), `48 C7 45 B0 47 00 00 00`, `C7 45 8C 20 03 00 00` | checked only |
| `0x1407C6EB0`, `0x1407C6F50` rows 0x24, 0x2E–0x2F | Re:CoM's Thunder rows | the KH1 rows' templates |
| `0x1401A6B20` effect at a point | (checked by M5) | called |

1:1 checks (7.6):

- **Checked (files + decomp)**, replayed by tests: the release at KH1's frame 30 (handler 31), Sora
  locked to the motion's end (68) and Re:CoM's action ended once (`movement::tests` "7.6"); the
  rows, data (16 / 20 / 26 + 2 × max MP, thunder resistance only), cost 1 MP; the strike timing
  (the release run, then every 8 frames; an enemy outside passed over without a wait; each once),
  the nearest-first order and the 30 slots, the cylinder test for spheres and columns, its edge,
  top and rim (`magic::tests`).
- **Checked (exe)**: the new sites, the rows, the two vtable slots, the constants.
- **Assumptions:** (1) KH1's sys 90 shape test is a plain overlap of each body with the cylinder
  (`0x1402B5EF0` not read in full); Re:CoM's bodies are Re:CoM's sizes (as for every hit, M5);
  (2) KH1's struck part ≈ Re:CoM's first live hurt body; (3) the strike thread's first run is in
  the release frame (as Fire's ball, ±1 frame); (4) a target alive at the command but dead at
  the release: KH1 measures from its (dead) actor if it still exists, we from Sora; (5) a Re:CoM
  body that guards from the bolt's side (`+0x9C & 1` / `+0xAC`) blocks it as Re:CoM's own Thunder
  would (KH1's direct strike has no guard check); (6) the lock-on list holds every enemy KH1's
  sys 65 walk would (enemies register in their init `0x1401805A0`); field enemies' type: not
  checked; (7) the bolt's visual at Sora's height is Re:CoM's rule for its own target.
- **Not built: the world dim to 50 %.** Re:CoM's spells have no screen tint, and the screen-mode
  call Lethal Frame uses (`0x140232260`) is an update mask, not a colour; no world-only tint found.
  Open question (a)/(b)/(c) below.
- **Not checked:** in game.

In-game checklist (7.6):

1. Log: the start line names Thunder (7.6) and 9 rows.
2. In battle with several Heartless spread around: Magic → Thunder (target one): Re:CoM's Thunder
   cast clip, half a second later Re:CoM's Thunder bolt falls on the nearest enemy to the target
   (the target itself first), then one more bolt every 8 frames (about 7.5 a second) on each next
   nearest inside 6 m of the target, each enemy once; enemies beyond 6 m (or 5 m above / below)
   are skipped. MP −1 point.
3. Without a target: the area is around Sora (6 m).
4. Sora stays locked for the whole motion (about 1.1 s), the bolts go on after he is free.
5. A second Thunder while bolts still fall: both strike lists run.
6. Hit during the wind-up: nothing, no MP. Air Thunder: the hop; landing before 30 cancels it.
7. Damage: Thunder + 2 × max MP through KH1's numbers; the log's `hit thunder1 … KH1 …`.
8. F9 trace: `cast thunder1 f… w…`, `release(thunder1 f30 …)`, `bolt 1 e… shot …`, `pay100`.
9. Combat = Re:CoM: Re:CoM's Thunder cards unchanged; Riku's story unchanged.

Open question (7.6): **KH1's world dim during Thunder** (to 50 % over 20 frames, back 40 frames
after the release): (a) leave it out (built); (b) research Re:CoM's render / fade code for a
world-only colour scale (Re:CoM's room fades and its pause dim are candidates, not read) and build
it there if one exists; (c) a full-screen tint drawn by Re:CoM's HUD layer (a `C2DSeqCtrl` plate
at 50 % black over the world, under the HUD): native drawing, but it darkens through its own
plate, not the world's colour.

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
3. ~~`fire_all`'s effects are loaded in the field~~: they are not (owner's 0.45.0 video; Checked
   (decomp + exe)); fixed by loading Re:CoM's battle set in the field, § Spell effects in the field.
4. Re:CoM's clips `p001_062` / `p001_063` / `p001_066` / `p001_067` are KH1's motions baked at
   30 fps (the frame counts match: research § 3.5, Checked (files)); the release clips carry
   KH1's root travel.
5. Re:CoM's target `+0x84C` stands for KH1's command target, and its aim point (vtable `+0x50`)
   for KH1's target part.

## Open questions (owner)

1. Fire's re-acquire: answered (decision 36, built in 7.4b).
2. Element resistance on spell hits: answered (decision 36: KH1's only).
3. **Fire in the field**: answered by the owner's report ("the VFX dont appear"): built as (b),
   Re:CoM's own battle-set load at room load (§ Spell effects in the field).

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

## Stage 7.4c: the Fire release's root curve

Ask (coordinator, M7): "KH1's Fire release motion moves Sora back 15 cm by frame 10 then 5 cm
forward (to −10 cm at frame 60); 7.4's motion data holds −15 cm. Fix to KH1's curve".

- **KH1** (`xa_ex_0010.mset`, request `0x3D` → entry 57, root helper read with `kh1_mset.py`'s
  `root_local`): z = −1.5 cm a frame over frames 0–10 (−15.0 at 10), then +0.1 cm a frame to −10.0
  at frame 60; x, y 0. The air release `0x8A` (entry 82): −4.24096 cm a frame to −42.4096 at 10,
  then still to 52. Checked (files).
- **Fixed:** KH1's motion record (`magic::add_motions`) now holds `(0, 0) (10, −15) (60, −10)` for
  `0x3D` and −42.4096 (was −42.41) for `0x8A`. Tests: `magic::tests::cast_motions_are_kh1s_frame_
  counts_and_windows` (the curve at 0 / 5 / 10 / 20 / 46 / 60), `movement::tests::fire_release_
  steps_back_15_then_forward_to_10_like_kh1` (the per-frame travel the fake Re:CoM's root step hands
  out: −1.5 cm steps, then +0.1 cm steps).
- **What this changed in game (7.4c): nothing.** The record's root keys drive only the tests' fake
  Re:CoM; in game Sora's travel is Re:CoM's own clip `p001_063`'s root (the row is played with root
  motion, `rows::row_for`, decision 17's delay). Re:CoM's clip (bone 0 translation of
  `PL0001.ANN`, read with `recom_ann.py`): **back 40.5 cm by its frame 5 (KH1 frame 10), then
  forward to −35.7 cm at its end**, so 25.5 cm deeper than KH1's step for the whole motion. The air
  clip `p001_067` holds KH1's −42.4 cm exactly. Checked (files). This matches the asset note
  (`kh1-recom-assets.md`: 57 → `p001_063` is an *edited* bake whose root split differs). Open
  question (owner) below.
- **Found while testing (the replay, not KH1):** a release that starts from its queued wind-up end
  is started on Re:CoM's clock one step on (decision 17's rule for queued motions), so the fake's
  root step loses the travel of frame 0→1 (1.5 cm) and, at the end, the last update's delayed
  travel into the idle (0.1 cm): the fake ends at −8.6 cm, not −10. Whether Re:CoM's root step
  does the same in game (a clip started at t = 1 handing no travel for 0→1): not checked. Assumption.

Open question (7.4c), **answered by decision 38 (option (b))**: "Fire's release step is KH1's 15 cm
curve (replacing Re:CoM's clip travel in the root-step wrapper)". Built below (M7 f1).

### Built (M7 f1, decision 38): KH1's curve in the root-step wrapper

- **Where (no new hook):** our wrapper on Re:CoM's root step `0x14024DCA0` (decision 17's; bytes
  `0F 28 81 A0 00 00 00`, `sites::ATTACK_HOOKS`). For the main Sora in **a KH1 cast** (action `0x13`
  with `+0x3C8` in our table) whose playing row (`player+0x474`, written by `0x1401F7180`) is **0x38**
  (`p001_063`, the ground release, also the fast path's): Re:CoM's root position for the update
  (`out` = the anchor `ctrl+0xF0` + its clip's travel) becomes the anchor + **KH1's travel between
  the clip's time at the last update and its time now** (`ctrl+0x3C`, ticks, 50 a KH1 frame;
  `crate::magic::release_root`). Then decision 17's one-update delay applies to it as to every KH1
  root travel. Every other row and action: Re:CoM's own travel stands.
- **The line:** Re:CoM's clip moves only along the model's z (bone 0's x / y constant 0), so its
  own travel of the update gives the line (`0x14024C960` turns it by the model's yaw); it is pointed
  along Sora's facing (body `+0x54`, (sin, 0, cos)), and KH1's z difference (cm × 0.01) is put on it.
  When Re:CoM's clip holds still (its frame 29 → 30 = KH1 58 → 60) the facing is the line.
- **The clock** (`release_root::Clock`): the span is (the clip's last time, its time now); a new
  clip (first seen, another controller, or a time that went back: the Fire chain restarts it)
  starts from frame 0, so a release Re:CoM starts one step on (a queued motion) still hands out
  KH1's frame 0 → 1. Reset whenever the wrapper sees Sora in anything but that clip.
- **Every other spell checked:** the air release `0x8A` and `p001_067` are the same curve (−42.41 cm
  by frame 10, then still); the wind-ups and the Blizzard / Thunder / Cure / Gravity / Stop / Aero
  motions (entries 54, 55, 56, 79, 80, 81) have no root travel in KH1, and their clips (`p001_062`,
  `064`, `065`, `066`, `068`, `069`) none in Re:CoM, played without root motion. Checked (files:
  `kh1_mset.py` on `xa_ex_0010.mset`, `recom_ann.py` on `PL0001.ANN`). Nothing else to change.
- **Per frame:** one atomic swap on the wrapper's non-cast path (it already ran there); a few float
  ops per update of the release clip.
- **Log:** the first 20 updates: `magic: fire release root: KH1 frames t0 -> t1: Re:CoM's clip
  (x,y,z) -> KH1's (x,y,z)`.
- **1:1:** Checked (files + tests: `magic::tests::fire_release_root_is_kh1s_curve_on_recoms_line`):
  KH1's keys at 0 / 3 / 10 / 33.5 / 60, the back step 1.5 cm an update on Re:CoM's line, the drift
  0.1 cm, the 0.2 cm tail after Re:CoM's clip stops, the update spanning frame 10, the whole release
  −10 cm, the clock's restarts. **Assumptions:** (1) the model's +z is Sora's facing (the clip steps
  him back: Re:CoM's travel and KH1's agree in sign); (2) `ctrl+0x3C` at the wrapper is the clip's
  time after this update's advance (`0x14024CCC0` advances it before `0x14024C960` computes the
  travel); (3) the fast path's 8-frame blend: KH1 blends the root position of the old and new
  motions (`kh1-anim`'s `blend_frame`), ours hands the new clip's KH1 travel from its frame 0 (the
  chained release is already back 15 cm when the next starts: KH1's blended travel over those 8
  frames is not modelled). **Not checked:** in game.
- **In-game check:** ground Fire: Sora steps back about 15 cm (a short hop back, not the old 40 cm)
  during the first sixth of a second, then creeps forward 5 cm; the log's `fire release root` lines
  show Re:CoM's ~0.040 m steps turned into KH1's 0.015 m. Air Fire, the other spells: unchanged.

## Stage 7.7: Cure / Cura / Curaga

Owner (decision 34): "Cure heals exactly as Re:CoM's Cure", "Cure 50, Cura 150, Curaga 300 HP
… with KH1's cast rules". At Combat = KH1 in Sora's story, Cure from the command menu casts as
KH1 and heals with Re:CoM's own heal.

### KH1 (what runs)

| what | KH1 (Steam) | tag |
|---|---|---|
| **Cast**: ground `0x36 + 3` = `0x39`, idle queued (release); air `0x86`, the fall (blend 8), the 30 cm hop, air speed × 0.5. Motions: **entry 56 (Thunder's), 68 frames, no free key** (locked to the end), no root; air entry 81, 68. | `0x1402A3DB0`, `0x1402A3600`; `xa_ex_0010.mset` (requests 0x39 / 0x86 → entries 56 / 81) | Checked (decomp + files) |
| **Script** `xm_cure01..03.mag` (one script, the level only in the data): charge effect on the Keyblade (`L004a6`), **wait 36** (`L004e8(36)`), release check (request `0x39` / `0x86`, else end: no heal, no MP). | `0x420..0x4A4` | Checked (files: `magdis.py`) |
| **Heal thread** (`L00056`): the target = the command's member, else the caster; skipped when not alive (sys 120), flagged `0x200`, or lacking `0x8000` (Assumption: defeated / healable); else the data's attack struck onto it (sys 23 / 60: KH1 heals 15 / 27 / 36 + max MP), **then the MP** (sys 47, 100 = one point); then the heal effect follows the target. | `0x56..0x194` | Checked (files) |
| Data: stat source 1, element 11 (Cure), power 15 / 27 / 36, cost 100. | btltbl `0x6318` / `0x6388` / `0x63F8` | Checked (files) |

### In Re:CoM

- **Cast** as Thunder's (`release_wait` 36); Re:CoM's clips: `0x39` → row `0x39` (`p001_064`, the
  clip Re:CoM's own Cure plays: row 0x25's `+0` = 0x39), `0x86` → `0x3D`. KH1 rows from Re:CoM's
  0x25 / 0x36 / 0x37 (checked at start): clip, `cure_all` sets 15 / 16 / 17, `+3` 0, `+0xF` 0x80.
- **Command** (`start_cast`): `initMagic` case 0x1D's charge: the set's effect id 0 on the weapon's
  (`+0x3D0`) bone 2 at (−0.8, 0, 0) (`0x1402ABB20`, `0x1401A2D50`; Checked (disasm
  `0x14022070A..0x140220757`)); the action's AP = **Re:CoM's heal: 50 / 150 / 300** (its card /
  sleight rule power ÷ 10, `SY0002.BIN`; Checked (files + decomp)).
- **Release** (`release_cure`): KH1's target test on Sora (the only target: no party members yet):
  HP (`+0x42C`) > 0 and not in Re:CoM's death path (`+0x7B0`); else nothing and no MP. Then
  **Re:CoM's Cure case** (`0x140222170` case `0x25`, Checked (disasm `0x140222946..0x1402229CF`)):
  its effect id set − 14 on Sora's bone 0 at (0, 0.85, 0), kept on him (`+0x4C |= 4`), and its heal
  **`0x1401E59A0(player, AP, 0, 1, 0)`** (HP += AP, clamped to the max; the HUD; its sound). Then
  KH1's MP (one point).
- Re:CoM's own Cure cards are untouched (Combat = Re:CoM, Riku's story).

Logs: `magic: release cure1: Re:CoM's heal 50 -> HP 120 -> 170 (max 300)`, `mp: paid 100 (Cure
release)`, or `release cure1: Sora can't be healed …: no heal, no MP`. F9 trace: `cast cure1 f…
w…`, `release(cure1 f36 no-shot)`, `heal 50 hp 170`, `pay100`.

Sites (7.7; `magic_sites` checks the calls and RIP loads):

| address | bytes | use |
|---|---|---|
| `0x1401A2D50` effect on bone | `45 8B D0 4C 8B DA 44 8B 81 50 01 00 00 41 8B D2 49 8B CB` | called |
| `0x140257EF0` effect object | `48 83 EC 28 8B D1 48 8B 0D 13 50 62 00 48 85 C9` | called |
| `0x1401E59A0` heal | `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 48` | called |
| `0x140222946` Cure case | 137 bytes ending `E8 D1 2F FC FF` (→ the heal) | checked only |
| `0x14022070A` initMagic Cure charge | 77 bytes ending `E8 F9 25 F8 FF` (→ `0x1401A2D50`) | checked only |
| `0x1407C6EC0`, `0x1407C6FD0` rows 0x25, 0x36–0x37 | Re:CoM's Cure rows | the KH1 rows' templates |
| `0x1407C7E50`, `0x1405A0910` | (−0.8, 0, 0, 1), (0, 0.85, 0, 1) | the offsets |
| `0x1402ABB20` bone | (checked by M5) | called |

1:1 checks (7.7):

- **Checked (files + decomp)**, replayed by tests: the heal at KH1's frame 36 (handler 37), Sora
  locked to the motion's end (68), Re:CoM's action ended once; a cast cut before 36 heals nothing
  and pays nothing; the air cast's hop and clip (`movement::tests` "7.7"); the rows, the heal per
  level (50 / 150 / 300), the cost (one point), no damage record (`magic::tests`).
- **Checked (exe)**: the new sites, the rows, the offsets.
- **Assumptions:** (1) KH1's `0x8000` / `0x200` target flags mean healable / defeated, which for
  Sora in Re:CoM is "HP left and not dying"; (2) `initMagic`'s charge and the Cure case's effect
  are Re:CoM's Cure look (decision 34) also when the heal comes at KH1's 36 instead of Re:CoM's 30;
  (3) a party member as target comes with the party (out of M7).
- **Not checked:** in game. Re:CoM's heal sound is the heal function's own (`param_4` = 1).

In-game checklist (7.7):

1. Log: the start line names Cure (7.7) and 12 rows.
2. Hurt Sora, Magic → Cure: Re:CoM's charge on the Keyblade, Re:CoM's cast clip, about 0.6 s later
   (36 frames) Re:CoM's Cure effect on Sora and +50 HP on the HP bar; MP −1 point. Sora stays
   locked for the whole motion (about 1.1 s).
3. Cura / Curaga (debug `kh1_all_spells`): +150 / +300 HP. At full HP: still cast and paid
   (KH1 heals a full target too).
4. Get hit before the heal: nothing healed, no MP. In the air: the hop; landing before 36 cancels.
5. In the field: the same (Re:CoM's heal also works outside battle).
6. F9 trace: `cast cure1 …`, `release(cure1 f36 no-shot)`, `heal 50 hp …`, `pay100`.
7. Combat = Re:CoM: Re:CoM's Cure cards unchanged; Riku's story unchanged.

## Stage 7.8: Gravity / Gravira / Graviga

Owner (decision 34): "Stop and Gravity against bosses exactly as in KH1"; Re:CoM-only bosses take
KH1's final-boss Gravity (2 %, via their proxy records, `recom-kh1-attack.md`). At Combat = KH1 in
Sora's story, Gravity casts as KH1's: a sphere falls on the target and crushes every enemy under it
once, for KH1's % of their current HP.

### KH1 (what runs)

| what | KH1 (Steam) | tag |
|---|---|---|
| **Cast**: ground `0x36 + 4` = `0x3A` (entry 55, Blizzard's: 86 frames, free from 68), air `0x87` (entry 80, 78, free from 68); the hop as every air cast. | `xa_ex_0010.mset`; `0x1402A3DB0` | Checked (files + decomp) |
| **Command** (`L008cc`): charge effect on the Keyblade; the marker thread (28 frames): its point is the target's part if alive, else the caster's position + **200 cm along his facing** (the offset taken at the command); the sphere's start is the point **240 cm up**. **Wait 28**, release check (`0x3A` / `0x87`, else nothing, no MP); the falling thread; **MP 100**; the hand effect. | `xm_gravity01.mag` `0x25A`, `0x8CC..0x9E0` | Checked (files) |
| **Fall** (`0x44..0x16C`): scale 0.01 → S over N frames (S, N = 1.0, 60 / 1.5, 70 / 1.6, 60, the level's arguments), **230 cm down**; each frame enemies touching a sphere of 170 × the current scale get `+0x88 |= 4` (slowed: their time scale eased to 0.8, `0x140297FE0`). | `L0055c` | Checked (files + decomp) |
| **Impact** (`L0044c`), the run after the N-th (so frame **88 / 98 / 88** after the command): one attack, **struck directly on every enemy whose bodies touch a sphere of 170 × S cm** (170 / 255 / 272; sys 90 type 1) at the sphere's centre (10 cm above the point); then a 20-frame fade growing × 1.02 a frame. | `0x16C..0x258` | Checked (files) |
| **Damage** `0x1402BFA80` element 8: **current HP × (power + 5 × max MP)% × resist[8]**, raised to the record's floor (1 when the floor is under 2); no Defence, no cap. Power 40 / 55 / 70, stat source 5. Kind 8. | btltbl `0x6468` / `0x64D8` / `0x6548` | Checked (files + decomp) |

### In Re:CoM

- **Cast** as Blizzard's (`release_wait` 28); Re:CoM's clips `0x3A` → row `0x3A` (`p001_065`, the clip
  Re:CoM's own Gravity plays: row 0x26's `+0` = 0x3A), `0x87` → `0x3E`. KH1 rows from Re:CoM's 0x26 /
  0x38 / 0x39 (checked at start): clip, `grav_all` sets 18 / 19 / 20, its `+5` / `+0xD`; `+3` 0,
  `+0xB` 1, `+0xF` 0x80.
- **Command** (`start_cast` → `gravity_command`): `initMagic` case 0x1E's charge (the set's `+0x30`
  entry, id 0, on the weapon's bone 2; Checked (disasm `0x1402207A5..0x1402207F4`)); KH1's marker
  point (Re:CoM's target `+0x84C` if it is an alive enemy, its aim point; else Sora + 2 m along his
  facing, the offset kept) and Re:CoM's marker effect (id 2 of the `+0x30` entry, as `initMagic`'s)
  at the sphere's start.
- **Release** (`release_gravity`): the point again (the target if still alive, else Sora now + the
  command's offset); **Re:CoM's Gravity case** (`0x140222170` case `0x26`, Checked (disasm
  `0x1402229E0..0x140222B90`)): its hand effect (id 1) and its sphere (`CHomingBullet`, flags
  `0x41E1`, `+0xB8` = `0x1407C78D0`, `+0x1C` = row `+5` × 0.01, mover `row[0xD]` × 0.01) with KH1's
  numbers: **from 2.4 m above the point, pitched straight down, 2.3 m / N a frame, no homing target,
  life N + 2 frames**, and `| 4` (the life ends it). Its attack is **held off** (flag `0x200000` +
  Re:CoM's "off", as 7.6's bolts): KH1 strikes at the impact, not on contact. Then KH1's MP.
- **The fall** (`gravity_frame`, from the per-frame spell check that runs Thunder's strikes: one
  atomic load when nothing runs): KH1's clock (`magic::Fall`: the release run counts the first frame)
  and KH1's growth on Re:CoM's sphere effect (the bullet's effect `+0x244` → `0x140257EF0`, scale
  `+0x10`: KH1's 0.01 → S itself, Re:CoM's own growth off since 0.113.0, § The sphere's size below). At the impact: every alive enemy of Re:CoM's
  lock-on list whose live hurt bodies touch the sphere of 1.7 × S m (`magic::body_touches`, Re:CoM's
  shapes) is struck once with the sphere's attack through **Re:CoM's apply** (7.6's mechanism,
  `apply_to`). If Re:CoM already ended the sphere (a wall), a held bolt of the same set at the centre
  carries the strikes (`carrier_at`).
- **Damage** (`spell_hit`, as every spell): KH1's number from the twin's / proxy's record with the
  enemy's **current** HP (`+0x274`, KH1's for twins and proxies) and its resist[8], written as the
  hit's value. An enemy with no KH1 record (none at KH1 in Sora's story except 100 Acre Wood's
  minigame) takes KH1's % at 100 %. Re:CoM's boss falloff (`+0x290`) still applies (decision 13).
- **Re:CoM's Gravity gate (enemy param `+0x40`) is not consulted at KH1**: Re:CoM's kind 0xD
  (`0x14017DC50`) is its own damage (`HP × AP / 100` with an integer %, behind the gate) and its own
  3-second squash (state `+0xAC0`: `0x14017C350` / `0x140183BA0`, flattened for 9000 ticks). KH1's
  Gravity is neither: its damage is the % above with KH1's resist[8] (never immune: the floor), so
  the KH1 rows' kind is Re:CoM's flinch (1) and the KH1 number is the hit's value. The resist[8]
  decides instead of the gate. Checked (decomp).

Logs: `magic: release gravity1: sphere N from (…) falls 2.3 m over 60 f to (…), impact radius
1.70 m, target …`, `gravity1 impact on 0x…: Re:CoM's apply -> Some(1)` (bolt-line budget), hit lines
`hit gravity1 on 0x…: KH1 …`, `mp: paid 100 (Gravity release)`. F9 trace: `cast gravity1 …`,
`release(gravity1 f28 shot N)`, `impact gravity1 shot N n<enemies>`, hits `:gravity1:kh1 …`.

Sites (7.8):

| address | bytes | use |
|---|---|---|
| `0x1402207A5` initMagic Gravity charge | 79 bytes ending `E8 5C 25 F8 FF` (→ `0x1401A2D50`; the set's `+0x30`) | checked only |
| `0x1402229F8` Gravity case hand effect | 59 bytes ending `E8 ED 40 F8 FF` (→ `0x1401A6B20`) | checked only |
| `0x140222AB7` Gravity case flags | `48 C7 45 B0 E1 41 00 00` | checked only |
| `0x140222B09` Gravity case `+0xB8` | `48 8D 05 C0 4D 5A 00` (→ `0x1407C78D0`) | checked only |
| `0x1407C78D0` | (0, 0, 0, 0, 0, 0, 0, 1.0) | the descriptor's `+0xB8` |
| `0x1407C6ED0`, `0x1407C6FF0` rows 0x26, 0x38–0x39 | Re:CoM's Gravity rows | the KH1 rows' templates |

1:1 checks (7.8):

- **Checked (files + decomp)**, replayed by tests: the release at 28 (handler 29), Sora free at the
  key (68), Re:CoM's action ended once, a cut cast casts nothing (`movement::tests` "7.8"); the
  fall's clock (the impact N frames after the release: 88 / 98 / 88 after the command), the scale's
  growth, the impact radii 1.7 / 2.55 / 2.72 m, the sphere test (not a column), the damage (55 % at
  max MP 3, LV2 80 %, Re:CoM-only boss 2 %, the floor, immune records still taking the floor), the
  rows, cost and data (`magic::tests`).
- **Checked (exe)**: the new sites.
- **Assumptions:** (1) KH1's no-target offset `caster+0x30 × 200` is the facing direction;
  (2) KH1's struck part ≈ Re:CoM's first live hurt body (as 7.6); (3) Re:CoM's sphere effect at
  scale 1 is the size KH1's sphere has at scale 1 (the visual only: the hits use KH1's radius);
  (4) a bullet's entity handle (`+0x20`) resolves through `0x1402AD350` (the apply resolves bodies'
  owners the same way); if not, the held bolt carries the strikes and the growth is skipped;
  (5) Re:CoM's `0x41E1` flags don't end the sphere before its life with the attack held (if a wall
  ends it, the strikes still come, from the bolt).
- **The sphere's size (0.113.0)**, owner 2026-11-04: "gravity's visual is really small for some
  reason". Cause (Checked (decomp)): Re:CoM's Gravity flags `0x41E1` carry bit 7, Re:CoM's own
  growth: `CHomingBullet`'s init `0x140158660` sets the effect's scale to `desc+0x1C / desc+0x78`
  (not `desc+0x90`) and each update (`0x140156E20` → `0x140156350`) adds that again, so in Re:CoM
  the effect scale = the hit radius / the row's 1.5 m. 7.8's growth took that first value as the
  "spawn scale" (≈ 0.024 / 0.024 / 0.029 with KH1's radius in `+0x78`) and multiplied KH1's 0.01 → S
  by it, with one Re:CoM step on top: at the impact 0.047 / 0.059 / 0.076 instead of 1.0 / 1.5 /
  1.6, **about 21 / 25.5 / 21 times too small** (up to 42 / 42 / 34 if Re:CoM's step ran first;
  `magic::gravity::old_effect_scale`, test `gravity_sphere_draws_kh1s_scale_not_recoms_self_growth`).
  Fix: the KH1 sphere's flags drop bit 7 (`magic::gravity::sphere_flags`: `0x4161`), `desc+0x90` =
  KH1's start 0.01 (the init writes it to the effect), and the growth writes KH1's scale itself
  (`effect_scale`). Assumption (3) above still stands: `grav_all` at scale 1 = KH1's sphere at
  scale 1 (`EF2040.EFF` is not in the session; if the owner sends it with KH1's `xm_gravity01.mag`
  the two programs can be compared). Re:CoM's own rule (scale = hit radius / 1.5 m) would draw
  KH1's sphere 1.7 / 1.5 = 1.13 times bigger: the alternative if it looks small against KH1's.
  Not carried: KH1's 20-frame fade growing × 1.02 a frame after the impact (Re:CoM's life end fades
  it at S). Combat = Re:CoM: Re:CoM's own Gravity (with bit 7) unchanged.
- **The flatten keeps its hurt bodies (0.117.0)**, owner 2026-11-04: "when the enemy is flattened
  from gravity, sora is unable to attack them, lets fix that". Cause (Checked (decomp + exe)): at
  Damage = Re:CoM (the default) our Gravity's kind is Re:CoM's 0xD, whose flatten state `+0xAC0`
  (picker `0x14017D260` case 0xD) begins with `0x14017C350`: **every hurt body off**
  (`0x1401A2D70(e, −1)`) and every attack body off, for the whole squash and its 9000-tick hold;
  its end `0x14017EB80` turns them on. Both are that state's begin / end in 78 class tables, no
  other reference (exe scan). KH1 (Checked (decomp)): the hit pass `0x1402B7260` runs the flatten
  (movement 0x16 / 0x17, request 0x66) and then the can-be-hit test `0x140292070` (flags
  `& 0x8048 == 0x8000`, a body not masked; the mask `+0x3A8` only by AI-script syscalls
  `0x1402BAA00` / `0x1402BAA40`): a crushed actor stays hittable. A flinch on it
  (`0x1402B8E10` kind 3 → `0x1402B1B10` true for movement 0x16 → `0x1402B8B90`) knocks it out of
  the flatten (request 0x4E / 0x4F, movement 6), and its update springs `+0x480` back +0.05 a frame.
  Built (`magic::flatten`, `win::magic::flatten_begin` / `flatten_end`, sites `FLATTEN_HOOKS` (11
  bytes each, no branch in: exe scan) / `FLATTEN_CHECKED`): in a KH1 battle, after Re:CoM's begin,
  Re:CoM's own hurt-bodies-on `0x1401A2DB0(e, −1)` (attack bodies stay off); swings, spells, shots
  and sleights then hit it through Re:CoM's hit handler (its damage; its reaction ends the
  flatten, as KH1's knock does). When the flatten ends before Re:CoM stood it up (height < 1), the
  squash list springs it back +0.05 a frame from there (18 frames from 0.1; the per-frame spell
  check runs only while it does). Combat = Re:CoM / Riku's story: Re:CoM's rule (untouchable).
  Assumptions: Re:CoM's flinch for KH1's knock 0x4E / 0x4F (Re:CoM owns the reaction); Re:CoM's
  base height 1.0 (its own flatten writes 1.0). At Damage = KH1 no flatten happens (the KH1 row's
  kind is a flinch; KH1's own hit flatten is open question 2 below).
- **Not built in 7.8: the slow under the falling sphere** (M7 f2 below: it is a squash, not a
  slow; KH1: `+0x88 |= 4` each frame, the actor's time scale eased to 0.8 with a wobble and back at
  +0.05 a frame). Re:CoM's enemies have a per-actor
  frame step (`+0x14`, ticks, read by their motion update `0x14024E470` and effects); its writer was
  not found, and no Re:CoM slow status was found (its kind specials have Stop, Freeze, Magnet,
  Synchro, Gravity; no slow). Open question below.
- **Not checked:** in game.

Open question (7.8): **KH1's slow under the falling Gravity sphere.** (a) leave it out (built);
(b) research Re:CoM's per-actor frame step (`actor+0x14`) and its writer, and scale it to 0.8 for
enemies under the sphere (and ease it back) if Re:CoM's task runner lets a per-actor step through:
everything of the enemy slows, as KH1's time scale; (c) slow only their motion (Re:CoM's motion
speed `ctrl+0x4C`, which Re:CoM's own Stop sets to 0): animations and root motion slow, their AI
timers and shots don't. **Decision 38 chose (b) "slow everything … research first"; the research
(M7 f2, below) found that KH1 has no slow there: it squashes.**

### M7 f2: what KH1 does under the falling sphere, and what is built

**Research (Checked (files + decomp))**: the research note's "time scale" was an Assumption, and
it is wrong:

| what | KH1 (Steam) | tag |
|---|---|---|
| The falling thread (`L000ee`), each fall frame: the sphere down 230 / N cm, its scale up, then `L0055c(sphere centre, 170 × scale)`: every enemy (sys 65) whose bodies touch that sphere (sys 90 type 1) gets **`actor+0x88 |= 4`**. Not on the impact run. | `xm_gravity01.mag` `0xEE..0x168`, `0x55C..0x5FA` (`magdis.py`) | Checked (files) |
| The actor update `0x140297FE0`, normal branch (skipped while `+0x88 & 1`, stopped; and for the flattened requests 0x66..0x68, and when the param block `+0x6C`'s byte 0 bit 0 is set): with bit 4, `+0x480 = (+0x480 − 0.8) × 0.95 + 0.8 − sin(+0x484) × 0.05`, `+0x484 = wrap(+0x484 + π/2)` (`0x1402A8280`, [−π, π]); without it, `+0x480 += 0.05` up to 1.0 and `+0x484 = 0`; then bit 4 is cleared. `+0x480` starts at 1.0 (`0x140292660`). | `0x140297FE0` | Checked (decomp) |
| **`+0x480`'s only reader**: the actor's world-matrix build `0x1402A0A40`: its scale `+0x40` with **y × `+0x480`**, applied with `0x1402A9090` (flag `+0x374 |= 0x2000`). Bit 4 of `+0x88` has no other reader (the other `|= 4` writers, `0x14027D4B0` / `0x14027E460`, are another struct's). | `0x1402A0A40` | Checked (decomp) |
| (Related, not asked) KH1's Gravity **hit** flattens the enemy: the hit pass `0x1402B7260` holds `+0x480` at 0.1 in states 0x16 / 0x17 or request 0x66 while its timer `+0x404` is ≤ 60, then eases it back (`(t − 60) / 15 × 0.9 + 0.1`, or `× 0.125`), and request 0x67 pulls it toward 1 by 0.3 a frame. | `0x1402B7260` | Checked (decomp); not built (owner question 2) |

So **KH1's "slow" is a vertical squash of the model** to about 0.8 of its height, wobbling with a
4-frame period, springing back at 0.05 a frame once out from under the sphere. Nothing of the
enemy's motion, think, timers or speed changes.

**Built (the most KH1-faithful native option; owner question 1):** KH1's squash on Re:CoM's own
height scale, the lever Re:CoM's own Gravity squash uses (state `0x140183BA0`, stop exe
`0x1401843A0` case 5: `y = timer / 250`, then 1.0):

- **Where:** the per-frame spell check that runs Gravity's falls (`win::magic::gravity_frame`, from
  the request-bits wrapper: one atomic load when no spell thread runs) → `squash_frame`:
  1. each squashed enemy's KH1 update (`magic::squash::Squash::step`) with the bit the last run
     left: its height = Re:CoM's height before (read once) × KH1's `+0x480`, written through
     Re:CoM's `0x1401A7050(enemy, &scale)` (the mover's `+0x70`, read with `0x1401A4A20`); a stopped
     enemy (Re:CoM's stop state, `+0x260` bit 19) isn't updated, as KH1's; back at 1.0 its height is
     put back exactly and it leaves the list;
  2. each falling sphere's test (fall frames only, not the impact run): the sphere's KH1 centre
     (`squash::centre_at`: straight down 2.3 m over N frames to the impact centre) and radius
     (`radius_at`: 1.7 m × the scale); every alive enemy of Re:CoM's lock-on list whose live hurt
     bodies touch it (`magic::body_touches`, KH1's sys 90 type 1, as at the impact) is under it for
     its next update.
- **Per frame:** only while a Gravity sphere falls or an enemy springs back (≤ 6 frames after): a
  lock-on list walk per falling sphere and one scale write per squashed enemy.
- **Logs / trace:** `magic: gravity1 squash on 0x…: under the falling sphere (centre …, r … m),
  Re:CoM's height scale 1.000 x KH1's +0x480 from the next update` (20 lines); F9 events `squash
  n<enemies> u<under now>`.
- **Sites** (`sites::MAGIC_CALLEES`, called): `0x1401A4A20` `48 8B 81 F0 00 00 00 48 83 C0 70 C3`,
  `0x1401A7050` `48 8B 81 F0 00 00 00 0F 28 02 66 0F 7F 40 70 C3` (both whole functions). Checked
  (exe: a script compares every literal site of `sites.rs` with the Steam exe, 321 / 321).
- **1:1:** Checked (decomp + files; `magic::tests::gravity_squash_is_kh1s_height_ease_and_wobble`):
  the ease, the wobble's phase and wrap, the spring back, the test sphere's centre and radius per
  frame. **Assumptions:** (1) KH1's param-block bit (`+0x6C` byte 0 bit 0) that exempts some
  enemies has no Re:CoM counterpart: every enemy squashes; (2) Re:CoM's mover scale y scales the
  drawn model's height as KH1's matrix scale does (Re:CoM's own Gravity squash uses it so); whether
  Re:CoM's hurt bodies follow it is Re:CoM's; (3) an enemy Re:CoM re-scales itself every frame
  would fight our write (none known); (4) the release run's own test (the first fall frame) is
  skipped (±1 frame).
- **Not checked:** in game. **In-game check:** cast Gravity on a group: while the sphere falls,
  every enemy under it shrinks to about 80 % height with a quick wobble and keeps moving and
  attacking at its normal speed; after the impact they pop back to full height within about 5
  frames. Combat = Re:CoM: Re:CoM's own Gravity unchanged.

Owner questions (M7 f2):

1. **Decision 38 asked for a slow; KH1 has none: it squashes.** (a) KH1's squash (built: enemies
   under the falling sphere shrink to about 80 % height with a small wobble, nothing slows);
   (b) the slow as decided (not KH1: the enemy's whole step at 0.8 through its per-actor dt
   `actor+0x14`, which Re:CoM's actor update `0x1401A59A0` counts its timers by; its writer not yet
   found); (c) neither.
2. **KH1's Gravity hit flattens the enemy** (height 0.1 for about a second, then it springs back):
   add it to the impact's hit, on the same height lever? (a) yes; (b) no (built: KH1's damage only).

In-game checklist (7.8):

1. Log: the start line names Gravity (7.8) and 15 rows.
2. In battle, target a Heartless: Magic → Gravity: Re:CoM's charge and marker above it, Re:CoM's
   cast clip, about 0.5 s later (28 frames) Re:CoM's Gravity sphere appears 2.4 m above it, grows
   while it falls for 1 s (60 frames), and at its bottom every enemy within 1.7 m takes one hit:
   KH1's 55 % of its current HP at max MP 3 (log `hit gravity1 … KH1 …`). MP −1 point.
3. Gravira / Graviga: bigger spheres (2.55 / 2.72 m), Gravira falls 70 frames.
4. A boss (Guard Armor): 4 % × 55 % of its HP (floor 2); a Re:CoM-only boss (Larxene): 2 %.
5. Without a target: the sphere falls 2 m in front of Sora.
6. Hit before 28: nothing, no MP. Sora is free about 1.1 s after the command (frame 68).
7. F9 trace: `release(gravity1 f28 shot …)`, `impact gravity1 shot … n…`, `pay100`.
8. Combat = Re:CoM: Re:CoM's Gravity cards unchanged (kind 0xD and its gate still Re:CoM's).

## Stage 7.9: Stop / Stopra / Stopga

Owner (decision 34): "Stop and Gravity against bosses exactly as in KH1"; Re:CoM-only bosses are
immune (KH1's final boss's Stop 0, through their proxy records). At Combat = KH1 in Sora's story,
Stop casts as KH1's and stops each enemy in its area, nearest first, in **Re:CoM's own stop state**.

### KH1 (what runs)

| what | KH1 (Steam) | tag |
|---|---|---|
| **Cast**: ground `0x36 + 5` = `0x3B`, air `0x88`: Thunder's entries 56 / 81, 68 frames, locked to the end, no root; the hop in the air. | `xa_ex_0010.mset` | Checked (files) |
| **Command** (`L00890`): **wait 4**, the charge effect, then **each frame for 24 frames** the release check (`0x3B` / `0x88`; a miss fades the effect and ends it: no spell, no MP); the thread at **28**; **MP 200** (two points). | `xm_stop01.mag` `0x87C..0x95E` | Checked (files) |
| **Thread** (`0x44..0x206`): centre = the target's part if alive, else the caster's position; every enemy (30 slots) nearest first from the centre (`L0040e`, Thunder's selection sort); down the list, an enemy whose bodies touch the cylinder **250 / 350 / 500 cm, ±300 cm** (sys 90 type 2) gets the Stop effect and **one direct strike** (sys 60), then **4 frames**; one outside is passed over at once. | `0x44..0x206`, `L00280` | Checked (files) |
| **The strike** (`0x1402BEEE0`, element 9): only if `resist[9]` > 0: `actor+0x88 |= 1`, `+0x3E4` = **(2 + max MP) × 60 × resist[9] frames** (power 2 + max MP at every level). | btltbl `0x65B8` / `0x6628` / `0x6698` | Checked (files + decomp) |
| **While stopped** (`0x140297FE0`): no motion, think or hit pass; queued hit records stay queued and **land when Stop ends**; the actor's own attacks are skipped (`0x1402B47A0`). | | Checked (decomp; research § 2.5) |

### Re:CoM already has a stop state, and it holds hits (Checked, decomp)

- **Entry:** a hit of kind 8 (`0x14017DC50` case 8): if the enemy's param `+0x2C` ≠ 0, `+0x2D8 =` the
  hit's value and no damage; the reaction picker (`0x14017D260` case 8) sends it to the state at
  `+0x940` (state 18 of `CBattleEnemy`'s 31; its begin / exe / end are vtable `+0x298` / `+0x2A0` /
  `+0x2A8` through the state table's thunks `0x14017C840` / `0x140184C70` / `0x14017EDD0`, filled
  at start-up into `0x14087AA30` / `0x14087AC20` / `0x14087AE10`).
- **Begin** `0x14017C740` (default; the research note's `0x14017C400` is the Magnet state's begin):
  the "abnormal" effect 4 (Re:CoM's stop look), **timer `+0x160 = +0x2D8`**, motion speed 0,
  `+0x260 |= 0x84000` (bit 19 "stopped", bit 14 "no HP loss"), `+0x154 |= 0x8000` (no root motion).
- **Held hits:** while bit 14 is set, Re:CoM's damage (`0x14017DAB0`) subtracts nothing; the stop
  exe `0x1401843A0` **records up to 8 hits** of kinds 1–5, 0xD, 0x11, 0x15 (their value × the
  attribute's resistance, Gravity's % of HP), and when the timer runs out **lands them one by one**
  (HP, its hit effect, a short reaction between each: half the reaction motion or 750 ticks), then
  leaves. So KH1's "hits land when Stop ends" is Re:CoM's own behaviour; the differences: Re:CoM
  keeps at most 8 (KH1's queue: no cap found) and spaces the landings by its reaction (KH1's pace
  after Stop: not read).
- **A stopped enemy's attacks:** the begin doesn't switch its attack bodies off (`+0x94`), so an
  enemy frozen mid-swing could still hit Sora. KH1 skips them: **we switch them off with Re:CoM's
  own `0x1401A2D90(enemy, −1)`** (what its reaction states' begins call) right after the hit
  handler sent the enemy to the stop state.
- **Every Re:CoM boss class has it** (Checked (exe): vtable `+0x298` of every `CEnemy*` / `CBoss*`
  class): 69 classes use the default `0x14017C740`, among them Guard Armor, Trickmaster, Darkside,
  Parasite Cage, Oogie (and his dice), Ursula (and her tentacles), Captain Hook, Dragon Maleficent,
  Jafar (bird, lamp), Hades, Cloud, Larxene, Vexen, Lexaeus, Zexion (and his book), Axel, Riku,
  Marluxia 1 / 3 (and the Specter), Ansem; **Marluxia 2, his Scythe and Funnel** wrap it
  (`0x140400230` / `0x140400270` / `0x14040AA40` call `0x14017C740` first). Not stoppable: parts
  that are not `CBattleEnemy`s (Guard Armor's feet `CGuardarmorFoot`, Marluxia 2's attack objects,
  Darkside's balls, the dragon's fire): they have no hurt bodies of an enemy, as KH1's parts.

### In Re:CoM

- **Cast** as Thunder's (`release_wait` 28: KH1's 4 + 24; a cut motion before 28 ends the cast,
  as KH1's per-frame check); Re:CoM's clips `0x3B` → row `0x39`, `0x88` → `0x3D`. KH1 rows from
  Re:CoM's 0x27 / 0x3A / 0x3B (checked at start): clip, `stop_all` sets 21–23, its shot's numbers,
  **kind 8 kept** (Re:CoM's stop kind); `+3` 0, `+0xF` 0x80.
- **Command:** `initMagic` case 0x1F's charge (the set's id 0 on Sora's bone 0 at (0, 0.85, 0), kept
  on him; Checked (disasm `0x140220926..0x140220988`)); the command's target is kept.
- **Release** (`release_stop`): the centre (the target's aim point if alive, else Sora), Re:CoM's
  lock-on enemies nearest first (`thunder_order`), the strike thread (`Strikes::every(…, 4)`, the
  cylinder `stop::area`), its first run now, then KH1's MP (two points).
- **A strike** (`stop_strike`): **Re:CoM's Stop case for that enemy** (`0x140222170` case `0x27` with
  a target, Checked (disasm `0x140222BBA..0x140222CB0`)): its `CBullet` with flags `0x25` at the
  enemy's aim point (speed 0, the row's range / life / radius × 0.1, `stop_all`), held off, then
  **Re:CoM's apply** onto the enemy (7.6's mechanism).
- **The hit** (`win::attack::stop_hit`): the hit's value = **KH1's duration in Re:CoM ticks** ((2 +
  max MP) × 60 × resist[9] × 50; carried in the part's i16 value, the whole duration written into
  `+0x2D8` when longer: see § Stop on attacking enemies below); **Re:CoM's gate
  (param `+0x2C`) is set to KH1's answer for the call** (open when the twin's / proxy's
  `resist[9]` > 0, shut at 0 = immune) and put back after; Re:CoM's kind 8 then stores the value and
  its stop state runs it. An enemy with no KH1 record takes 100 %. After the handler: the attack
  bodies off. A stopped enemy's later spell hits keep the neutral attribute on the part so the held
  hit lands with KH1's resistance only (decision 36).
- Re:CoM's own Stop cards (kind 8 with its own gate) are untouched at Combat = Re:CoM.

Logs: `magic: release stop1: centre (…) r 2.5 m ±3 m, N enemies listed, first …`, `stop on 0x…:
shot N, Re:CoM's apply -> Some(1)`, `hit stop1 on 0x…: KH1 resist[9] 100 % -> 15000 ticks (300
frames), Re:CoM's stop state, its attacks off` / `… -> immune`, `mp: paid 200 (Stop release)`.
F9 trace: `cast stop1 …`, `release(stop1 f28 shot …)`, `stop 1 e… shot …`, `pay200`, hits
`e…:stop1:15000t`.

Sites (7.9):

| address | bytes | use |
|---|---|---|
| `0x1401A2D90` attack bodies off | `48 8B 89 F8 00 00 00 48 85 C9 0F 85 80 26 FB FF C3` | called |
| `0x14017DCB5` kind specials, Stop gate | `48 8B 83 08 03 00 00 83 78 2C 00 0F 84 46 02 00 00 F3 0F 2C C6 89 83 D8 02 00 00` | checked (its param field is written for the call) |
| `0x14017D28A` reaction picker case 8 | `… 48 8D 83 40 09 00 00 … 48 89 43 68` | checked only |
| `0x14017F937` hit handler's stop test | `48 8B 56 60 48 8D 8E 40 09 00 00` | checked only |
| `0x14017C740`, `0x1401843A0`, `0x14017ED30` stop begin / exe / end | `48 89 5C 24 08 57 48 83 EC 30 …`, `4C 8B DC 57 48 81 EC B0 00 00 00 …`, `40 53 48 83 EC 20 …` | checked only |
| `0x140222BC8` Stop case flags | 44 bytes (`… C7 45 B0 05 …`, `… C7 45 B0 25 …`) | checked only |
| `0x140220926` initMagic Stop charge | 102 bytes (→ `0x1402ABB20`, `0x1401A2D50`, `0x140257EF0`) | checked only |
| `0x1407C6EE0`, `0x1407C7010` rows 0x27, 0x3A–0x3B | Re:CoM's Stop rows | the KH1 rows' templates |

1:1 checks (7.9):

- **Checked (files + decomp)**, replayed by tests: the release at 28 (handler 29), Sora locked to the
  end (68), Re:CoM's action ended once, a cut cast nothing (`movement::tests` "7.9"); the rows, the
  areas (2.5 / 3.5 / 5 m, ±3 m), the cost, the duration (300 frames at max MP 3, × resist[9], 0 =
  immune, 600 at max MP 8 fitting Re:CoM's value), the order and one strike every 4 frames with
  outside enemies passed over (`magic::tests`).
- **Checked (exe)**: the new sites; every boss class's stop slot (offline script over the RTTI
  vtables).
- **Assumptions:** (1) Re:CoM's generic update counts the stop state's `+0x160` down by the frame
  step (as its other state timers; not traced); (2) a second Stop on a stopped enemy: Re:CoM keeps
  its first timer (its picker leaves a stopped enemy in place and the begin isn't rerun); KH1's apply
  rewrites the duration only for actors with a stop handler (`+0x408`): not compared; (3) an immune
  enemy shows Re:CoM's own "no effect" answer to a gated kind (`0x14020C9A0(…, 6)` and vtable
  `+0x3F0`); KH1 plays its Stop effect and nothing else; (4) KH1's struck part ≈ the first live
  body (7.6).
- **Not 1:1 (Re:CoM's held hits):** at most 8 hits are held (KH1: no cap found) and they land spaced
  by Re:CoM's short reaction instead of KH1's pace (not read). Open question below.
- **Not checked:** in game.

Open question (7.9): **the held hits' cap and pace.** (a) keep Re:CoM's (built: 8 hits, landed one
by one with its reaction); (b) read KH1's post-Stop landing pace (`0x1402B7260` / `0x1402B8E10`'s
queue handling) and, if it differs, land Re:CoM's held list at KH1's pace by setting the stop
exe's per-landing timer (`+0x160` in its phase 2) from our hit wrapper; (c) also lift the 8-hit cap
(needs a larger held list than Re:CoM's 8 × 0x30 slots at `+0x340`: a design change in Re:CoM's
enemy object, not just data). **Answered by decision 38: (b), Re:CoM's 8-hit cap kept.** Built
below (M7 f3).

### M7 f3: the held hits land at KH1's pace

**KH1 (Checked (decomp)):**

| what | KH1 (Steam) |
|---|---|
| A hit on a stopped actor (`+0x88 & 1`) takes the record builder's third branch (`0x1402BF630`, pool `0x142DE6E10`, 128 records for everyone: no per-actor cap); **if the actor's queue (`+0x3EC`) already holds a record**, the new record's `+0xC` bits 12–15 get a delay of **`6 − (int)(r × −6)` frames**, `r` = `0x14031BA30()` = three `rand()` low bytes / 2²⁴ ∈ [0, 1): **6..11 frames, equally likely**. The first record of an empty queue has none. (`+0xC |= 0x800`, and `0x10000` while stopped.) | `0x1402BF630` (`0x1402BF84C..`), `0x14031BA30` |
| While stopped the actor's update skips the hit pass (only Stop records are handled); when the timer `+0x3E4` runs out the bit clears. | `0x140297FE0` |
| From the next frame the hit pass `0x1402B8E10` loops `0x1402BF160` (pop): a head with a delay loses one and **ends the pop for this frame**; a head with none is applied (`0x1402B7B90`: the damage, `0x1402BEEE0`) and the next head is looked at in the same frame. | `0x1402B8E10`, `0x1402BF160` |

So the first held hit lands on the first frame after Stop, and **each next one 6–11 frames after
the one before** (each its own random delay).

**In Re:CoM (Checked (decomp + disasm)):** the stop exe `0x1401843A0` phase 1 lands the held hit
`+0x33E` (HP, effect, reaction) and sets the per-landing timer `+0x160` to half its reaction motion
or 750 ticks (`0x140184910`), phase 2 (`0x14018499D`) waits while the timer is above 0, then
`+0x33E += 1`, phase 1, which lands the next one the frame after. The actor update `0x1401A59A0`
counts `+0x160` down by the enemy's dt (`+0x14`) **before** its pre-update (`+0x78`: the hit
handler, our wrapper) and the state machine's exe (`0x1401A5A7F`).

**Built:** in the enemy-hit wrapper (`0x14017F7E0`, no new hook), at Combat = KH1:
`win::magic::held_hit_pace(enemy)`: when the enemy is in Re:CoM's stop state (`+0x60` = `+0x940`,
the hit handler's own test) in phase 2 with a held index not yet paced (the frame after a landing)
and another held hit to come, the timer becomes **`gap × 50 − 2 × dt` ticks** (`magic::stop::
landing_timer`) with **KH1's gap** (`stop::held_gap` of KH1's own `0x14031BA30` random, kh1-core's
`Kh1Rand::next_f32`, seeded once per session): Re:CoM then lands the next held hit exactly `gap`
frames after the last. The last held hit keeps Re:CoM's own wait before its recovery. Re:CoM's
8-hit cap, its landing, damage, effect and reaction are its own.

- **Per frame:** one compare per enemy update (its state), a small list only while held hits land.
- **Log / trace:** `magic: stop held hits on 0x…: hit i of n landed; the next in KH1's g frames
  (Re:CoM's timer t -> t' ticks)` (40 lines); F9 event `held i/n next gf`.
- **Sites** (`sites::MAGIC_CHECKED`, checked only): `0x140184910` `B8 EE 02 00 00 48 8B CF 89 87 60
  01 00 00`; `0x14018499D` `83 B9 60 01 00 00 00 0F 8F 2B 02 00 00 66 FF 81 3E 03 00 00 C7 81 70 02
  00 00 01 00 00 00`; `0x1401A5A7F` (39 bytes: `+0x160 −= +0x14`, `call [rax+0x78]`, `[rdi+0x58]`
  `+8`). Checked (exe).
- **1:1:** Checked (decomp + tests: `magic::tests::stop_held_hits_land_at_kh1s_pace_in_recoms_stop_exe`):
  KH1's 6..11 frames and their even spread; a frame-by-frame replay of Re:CoM's order (timer down,
  wrapper, exe phases 1 / 2) landing hits exactly at the gaps (6, 6, 6 and 11, 7, 9, 6, 10, 8, 11);
  the 30 fps ticks. **Assumptions:** (1) the first held hit lands one frame after Stop in both games
  (Re:CoM: phase 0 → 1, then the landing; KH1: the bit clears, then the hit pass) — not traced;
  (2) KH1's random draws are shared with everything else KH1 rolls, so the exact sequence isn't
  reproducible, only its law (6..11, even); (3) a held hit of value < 1 ("no effect") lands at once
  in Re:CoM (no timer), KH1 waits its delay (not changed).
- **Not 1:1 (decision 38 keeps it):** Re:CoM holds at most 8 hits; KH1 holds all.
- **Not checked:** in game. **In-game check:** Stop a Heartless and hit it 5 times: when it thaws
  the first hit lands at once, then the others one by one about a tenth to a fifth of a second
  apart (6–11 frames), faster than before (Re:CoM's half-reaction); log `stop held hits … the next
  in KH1's … frames`.

In-game checklist (7.9):

1. Log: the start line names Stop (7.9) and 18 rows.
2. Several Heartless near a target: Magic → Stop: Re:CoM's charge on Sora, the cast clip, about 0.5 s
   later Re:CoM's Stop shot appears on the nearest enemy, then one more every 4 frames on each next
   nearest within 2.5 m (Stopra 3.5, Stopga 5) of the target; each freezes in Re:CoM's stop state
   for 5 s at max MP 3 (300 frames; log `hit stop1 … 15000 ticks`). MP −2 points.
3. Hit a stopped enemy several times: no damage numbers until it thaws, then the hits land one by
   one (Re:CoM's held hits); spells keep KH1's numbers.
4. A stopped enemy frozen mid-attack doesn't hurt Sora.
5. Bosses: Guard Armor full time (Stop 100 %), Trickmaster 30 % (1.5 s), Larxene / Riku Replica /
   Marluxia: immune (`… -> immune`).
6. Without a target: the area is around Sora.
7. F9 trace: `release(stop1 f28 …)`, `stop 1 e… shot …`, `pay200`.
8. Combat = Re:CoM: Re:CoM's Stop cards unchanged.

## Stage 7.10: Aero / Aerora / Aeroga

Owner (decision 34): "Aero works as KH1's". At Combat = KH1 in Sora's story, Aero puts KH1's barrier
on Sora: his damage taken is halved for (18 + max MP) seconds, and Aerora / Aeroga add KH1's wind
around him that hits enemies again every half second.

### KH1 (what runs)

| what | KH1 (Steam) | tag |
|---|---|---|
| **Cast**: ground `0x36 + 6` = `0x3C`, air `0x89`: Thunder's entries 56 / 81, 68 frames, locked to the end; the hop in the air. | `xa_ex_0010.mset` | Checked (files) |
| **Command** (`0x4C4..0x674`): charge; **up to 30 frames while the request is 60 / 137**; when the motion changes **it casts at once** (no cancel); the cast aura; if the target (the member, else the caster) and the caster are alive and healable: the barrier strike, the barrier thread, **MP 200**, and after 24 frames the lasting barrier effect (if it had none). | `xm_air01.mag` | Checked (files) |
| **Barrier** (`0x1402BEEE0` element 12): `+0x88 |= 8`, timer **(18 + max MP) × 60 frames** (power 18 + max MP, every level), counted down by the frame step; a re-cast rewrites it. While on: **damage taken ÷ 2** after the resistance, then at least 1 (`0x1402BFA80`). | btltbl `0x6708` / `0x6778` / `0x67E8` | Checked (files + decomp) |
| **Wind** (Aerora / Aeroga, data 2 `0x5CF8` / `0x5D68`): an attack owned by the target, **radius 80 × its size**, always on, **re-hit every 30 frames** (`+0x50`), at its body centre (bone 32771) **while the barrier lasts**; power 8 + **max MP × 2**, element 0, kind 3. Each cast starts its own thread (a re-cast adds a wind). | `0x44..0x1DE` | Checked (files) |

### In Re:CoM

- **Cast** as Thunder's (`release_wait` 30); Re:CoM's clips `0x3C` → row `0x39`, `0x89` → `0x3D`.
  KH1 rows from Re:CoM's 0x28 / 0x3C / 0x3D (checked at start): clip, `airo_all` sets 24–26, `+3` 0,
  `+0xB` 1 (the wind's flinch), `+0xF` 0x80. `initMagic` case 0x20's charge (Cure's: the weapon's
  bone 2).
- **KH1's early cast**: a cut Aero (a landing, a walk-off, Re:CoM's hit reaction) is released in
  that frame instead of dropped (`Kh1Sora::leave_cast`, `resync`, `cast_step`); KH1's MP rule
  (paid only when the target is valid) stays.
- **Release** (`release_aero`): Sora must be alive (the only target; no party yet). **Re:CoM's Aero
  case's look** (`0x140222170` case `0x28`, Checked (disasm `0x140222CB0..0x140222E7A`)): its hand
  effect id 1, the ground wind set − 0x16 when on the ground, set − 0x11, at Sora's position.
  **KH1's barrier**: a new player status of ours (the countdown, `magic::aero::Barrier`), re-written
  by a re-cast. Then Re:CoM's **Aero shot** (its `CBullet`: flags 7, pitch −π/2, the row's set) at
  Sora's body centre with life = the barrier: **held off for Aero** (the lasting aura only: Re:CoM's
  `airo_all` look on its shot), **KH1's wind for Aerora / Aeroga** (radius 0.8 m, kind 1, Re:CoM's
  aero attribute 1, collided by Re:CoM). Then KH1's MP (two points).
- **The damage halving** (`win::attack::aero_halve`, in our wrapper on Re:CoM's player damage check
  `0x1401E6DE0`): while the barrier is on and a hit is pending (`+0x720`), the hit's value on Sora's
  body (`+0x690`'s `+0x92` = `+0x722`, read by the check through `0x14016FDB0` before its
  resistance) becomes `max(v / 2, 1)`. Re:CoM's check then runs as ever. Checked (disasm
  `0x1401E6FBE`, `0x1401E70B0`).
- **The wind's native mechanism:** the wind is Re:CoM's own attack object (the Aero shot's
  `CCollAttack`), so **Re:CoM's collision pass collides it with enemies' hurt bodies, Re:CoM's apply
  records the hit, the enemy's handler takes it** (with KH1's wind damage on twins / proxies,
  `spell_hit`: 8 + 2 × max MP through the curve, element 0). KH1's re-hit every 30 frames is
  **Re:CoM's own "new serial" `0x14016F7D0(attack)`** (what its multi-hit attacks call): with a new
  serial, the apply's hit-once ring (`0x14016F960`) lets the attack through each enemy once more.
  Ours per frame, only while a barrier is on (`aero_frame`, from the per-frame spell check): the
  countdown, the shot moved to Sora's body centre (KH1's bone 32771 ≈ Re:CoM's position + 0.85 m,
  the offset Re:CoM's Cure / Stop cases use on him), its life set to what is left, and every 30
  frames the new serial. At the barrier's end the shots' lives reach 0 and Re:CoM ends and fades them.

Logs: `magic: release aero2: KH1's barrier 1260 f (max MP 3), damage taken halved; Re:CoM's Aero shot
Some(N) follows Sora, KH1's wind 0.8 m re-hitting every 30 f`, `magic: aero: Sora's hit 30 -> 15
(KH1's barrier)` (hit-line budget), `aero: the barrier is over`, hit lines `hit aero2 on 0x…: KH1 …`,
`mp: paid 200 (Aero release)`. F9 trace: `cast aero2 …`, `release(aero2 f30 shot …)`, `barrier 1260
wind`, then `barrier <frames left>` on every frame, `barrier off`.

Sites (7.10):

| address | bytes | use |
|---|---|---|
| `0x14016F7D0` attack new serial | `8B 05 7A 31 65 00 BA 01 00 00 00 83 C0 01 0F 44 C2 89 05 69 31 65 00 89 41 70 C3` | called |
| `0x1401A49B0` actor position | `48 8B 81 F0 00 00 00 48 83 C0 30 C3` | called |
| `0x140222D2F` Aero case effects | 138 bytes (→ `0x1401A49B0`, `0x1401F3C00`, `0x1401A6B20` × 2; ids set − 0x16, − 0x11) | checked only |
| `0x140222DFE` Aero case flags | `C7 44 24 5C 00 00 00 80 48 C7 45 B0 07 00 00 00` | checked only |
| `0x140520910` | −π/2 | the shot's pitch |
| `0x1401E6FBE`, `0x1401E70B0` damage check | the pending count `+0x720`; `lea rcx,[rdi+0x690]` … `call 0x14016FDB0` | checked only (the value `+0x722` is halved before the check) |
| `0x1407C6EF0`, `0x1407C7030` rows 0x28, 0x3C–0x3D | Re:CoM's Aero rows | the KH1 rows' templates |

1:1 checks (7.10):

- **Checked (files + decomp)**, replayed by tests: the cast at 30 while its motion plays (handler
  31), at once when Re:CoM cuts it (its hit reaction) or a landing replaces it (never dropped), Sora
  locked to 68 (`movement::tests` "7.10"); the barrier's 1260 / 1560 frames at max MP 3 / 8, the
  wind from Aerora on, 42 re-hits over a max-MP-3 barrier, the halving (÷ 2, at least 1, 0 stays 0),
  the wind's damage (8 + 2 × max MP through the curve), the rows and cost (`magic::tests`).
- **Checked (exe)**: the new sites.
- **Assumptions:** (1) Sora's size for the wind is 1.0 (KH1's sys 189; Re:CoM's Sora isn't scaled);
  (2) KH1's halving comes after its resistance; ours before Re:CoM's: the same while Sora's Re:CoM
  resistance is 100 % (no cards at KH1); (3) a bullet's handle resolves through `0x1402AD350` (7.8's
  Assumption 4): if not, the shot stays where it was made and lives the barrier's first length;
  (4) Re:CoM's Aero shot's `airo_all` look stands for KH1's lasting barrier effect (Re:CoM has no
  barrier); (5) the barrier lasts across Re:CoM's field / battle switch (KH1's lasts until its timer;
  ours counts in Sora's frames); (6) the 24-frame delay of KH1's lasting effect is not built (the
  shot's look starts at the cast).
- **Not checked:** in game.

In-game checklist (7.10):

1. Log: the start line names Aero (7.10) and 21 rows.
2. Magic → Aero: Re:CoM's charge, cast clip and Aero wind effects, Re:CoM's Aero look staying on
   Sora; MP −2 points; for 21 s at max MP 3 (1260 frames) every hit on Sora does half (log `aero:
   Sora's hit 30 -> 15`). F9: `barrier …` counting down, `barrier off`.
3. Aerora / Aeroga: enemies touching Sora (0.8 m around his body) take the wind's hit, again every
   half second while they stay; the wind follows Sora as he moves.
4. Get hit during the cast: the barrier still comes (at once), MP paid. Air Aero: landing early
   casts it at the landing.
5. Re-cast during a barrier: the timer starts again (and Aerora adds a second wind, as KH1).
6. Combat = Re:CoM: Re:CoM's Aero cards (its damaging wind) unchanged; Riku's story unchanged.

## Spell voices (0.41.0; owner, 2026-11-01: "casting cure makes sora say "thunder!" etc")

- **Cause** (Checked (decomp + files)): Re:CoM's spells share cast clips (`p001_064` / `068`:
  Thunder, Cure, Stop, Aero; `p001_065` / `069`: Blizzard, Gravity) and `PL0001.SES` holds one
  voice event per spell on them, picked by the motion request's sixth argument (the SES
  variant). `initMagic` passes `0x14021FEF0(pl, 4)` (Sora's vtable `+0x110`, on `+0x638`):
  Fire / Blizzard / Thunder 0, Cure / Gravity 1, Stop 3, Aero 4 (-ra / -ga the same). Our KH1
  cast passed 0: Thunder's voice (2023) on Cure / Stop / Aero, Blizzard's (2022) on Gravity.
- **Built:** `win::movement` passes Re:CoM's own variant for KH1 cast motions (vtable slot
  checked equal to `0x14021FEF0`; else 0, logged). SES voice events: 2021 Fire, 2022 Blizzard,
  2023 Thunder, 2024 Cure, 2026 Gravity (air: 3026, Re:CoM's own data), 2027 Stop, 2028 Aero.
- **KH1** (Checked (decomp + files)): the voice is queued by the cast code `0x1402A3600`
  (`0x1402901F0(Sora, spell + 7, target)`) on the cast's first tick (frame 0), from btltbl voice
  groups 7..13: 75 / 25 between two takes (Aero 50 / 50); Cure and Aero call an ally's name when
  aimed at one. Chained Fire has none (and none plays here: `p001_063` has no SES entry).
- **Not KH1's:** the voice starts at Re:CoM's frame (3 on the ground, 2 in the air), not frame
  0; Re:CoM's one take per spell, not KH1's two; no ally names (no party yet).

## The Keyblade and the spell effects in the field (castfix; owner's 0.45.0 video)

> Owner (video, field room "13F Entrance Hall", no enemies, Combat = KH1): "when casting spells,
> soras keyblade isnt visible, and the VFX dont appear." His log: the casts ran (thunder3 /
> fire3 released, shot ids made).

### The Keyblade

- **Cause** (Checked (decomp)): Re:CoM hides Sora's Keyblade in the field and draws it for its own
  attacks: the field swing init `0x1401F0F70` ends with `vtable +0xF8(Sora, 1)` = `0x1401F5D30`
  (→ `0x140225240(weapon, 1)`: shown, `kb_appear` effect; nothing when already out); Guard does the
  same (Stage 17). Re:CoM's `initMagic` (`0x1402203D0`) and `exeMagic` (`0x140217EE0`) never draw
  it: Re:CoM casts only in battle, where the Keyblade is already out. Our cast copies `initMagic`,
  so in the field the Keyblade stayed away for the whole cast.
- **Fix:** KH1's command (`Kh1Sora::command_magic`), once Re:CoM's magic action is set
  (`Host::start_cast`), draws it through `Host::draw_weapon` = `0x1401F5D30(Sora, 1)`, Re:CoM's own
  call as the swing init and Guard make it. Every spell, ground and air, the Fire fast path too
  (already out: Re:CoM's function does nothing). Re:CoM's own rules put it away again later, as
  after a field swing or Guard. No new site (the callee is M5's / Stage 17's).
- KH1: the owner's report ("KH1 shows the Keyblade while casting"); KH1's Sora always holds it
  (Stage 17's Assumption). Test: `movement::tests` "ground_fire_releases_at_28…" (one draw at the
  command).

### Spell effects in the field

- **Cause** (Checked (decomp + exe)):
  - Re:CoM's spell effects play by name (the effect sets `0x1405A1730`: `fire_all`, `bliz_all`,
    `_thunder_all`, `cure_all`, `grav_all`, `stop_all`, `airo_all`). Both effect calls
    (`0x1401A2D50` → `0x1402577D0`, `0x1401A6B20` → `0x1402588B0`) and the bullet init look the
    name up among the **loaded** resources (`0x1402A4920` → `0x1402A43E0(5, name)`, also with a
    leading `_`); none loaded: no effect (handle 0), nothing else changes. The effect system itself
    runs in the field (save points, `kb_appear`).
  - The packs come with Sora's **battle set**: PLAYER group 1001 (Halloween Town Sora's too; Riku
    1101, the other Riku 1201), which has no files of its own, only LINK rows to EFFECT groups
    1100, 1102, 2000, 2003, 2010, 2013, 2017, 2020, 2023, 2030, 2040, 2050, 2060, 2063, 2073, 2078,
    3090, 3220, 501, 502, 504, 506, 556 (PLAYER table `0x1408055D0`, EFFECT table `0x140810500`).
    EFFECT 2000..2060 (and 2003 / 2013 / 2023 / 2063, which link them) are linked by **no other
    row** of any resource table (Checked (exe): all 10 tables' LINK rows); the groups loaded with
    the player in the field (SYS 1–3, PLAYER 1 / 2 / 11) link EFFECT 1, 100, 102, 500, 505,
    507..510, 1000 only. Which pack holds which name: not read (no `Recom.pkg` in the session);
    2000..2060 in steps of 10 = the seven spells is an **Assumption** (strong).
  - Re:CoM queues the set with `0x140211960(0)` (char from the world → group 1001, `0x1407C7AE4`;
    `0x14023CCE0` when `0x1402A53E0(1, group)` says it isn't loaded) in **`CGameSubBattle`'s state 1**
    (`0x1401AEA20`) and, under its system byte (0, 0x60), in `CGameMain`'s load state
    (`0x1401AD040`). **`CGameSubField`'s state 1** (`0x1401AF920`) queues only the player's group
    (`0x140211AA0(0)`). The set is unloaded by `0x140211560(0)` when a scene changes
    (`0x1401AA000`, `0x1401A9D20`) and when the camp opens (`0x1401A9590` open path,
    `0x1401A9990`), each unless that system byte is set (it keeps the set resident).
  - So: **in battle rooms the effects appear** (Re:CoM's battle state loads the set before the
    room starts; Re:CoM's own magic cards use them there); **in the field they were never loaded.**
- **Fix (Re:CoM's own calls, Re:CoM's own rule under its byte 0x60):** at Combat = KH1 in Sora's
  story (`magic::field_fx::wanted`),
  1. **room load:** `CGameSubField` state 1's tail `jmp 0x140211AA0` (`0x1401AF933`, `E9 68 21 06
     00`) goes through a jump placed near it to `win::magic_fx::field_load`, which makes that call
     and then `0x140211960(0)`, the call `CGameSubBattle`'s state 1 makes (its bytes are the field
     state's with this tail: `sites::MAGIC_FX_CHECKED`). The room's next state waits on the loader
     queue as for the player's group;
  2. **camp:** the camp openings' `call 0x140211560` (`0x1401A95BD`, `0x1401A99F9`, each `E8 rel32`)
     go to `camp_unload`, which keeps the set (as Re:CoM does under the byte) and otherwise calls
     Re:CoM's;
  3. **Combat switched to KH1 in the camp's Config:** `0x140211960(0)` at once (only with Sora in a
     room), so the room the camp closes into has the effects (after the loader's few frames).
  At Combat = Re:CoM or in Riku's story every path is Re:CoM's (the jump target and the calls are
  Re:CoM's own functions). A set left loaded after a switch to Re:CoM goes at Re:CoM's next unload
  (room change or camp opening). No effect of ours; nothing per frame.
- **Not changed:** Re:CoM's unloads on a scene change (`0x1401AA000`, `0x1401A9D20`): each field
  room load queues the set again (the vanilla battle loads it on every battle entry the same way).
  **Cost to watch:** a room change in the field now also loads the set's 23 packs (time not
  measured). Option if it shows: keep the set over room changes too, as Re:CoM's byte 0x60 does
  (two more call sites; those paths also clear a battle spawn flag `0x14087B6B0`, so they need
  care).

Sites (castfix; all bytes from the Steam exe, re-read with pefile: 394 site byte lists, 0
mismatches):

| address | bytes | use |
|---|---|---|
| `0x1401AF933` field state 1: player group jump | `E9 68 21 06 00` | patched: → near jump → `field_load` |
| `0x1401A95BD` camp open: battle set unload | `E8 9E 7F 06 00` | patched: → near jump → `camp_unload` |
| `0x1401A99F9` camp open (menu button): battle set unload | `E8 62 7B 06 00` | patched: → same |
| `0x140211960` battle set load | `40 53 48 83 EC 20 48 89 7C 24 30 85 C9 75 2D` | called |
| `0x140211560` battle set unload | `48 83 EC 28 8B 05 76 65 5B 00 85 C9 0F 45 C1 89` | called (Re:CoM's path) |
| `0x1401AF920` field state 1 | `48 83 EC 28 E8 17 59 08 00 85 C0 75 0B 33 C9 48 83 C4 28` | checked only (ends at the jump) |
| `0x1401AEA20` battle state 1 | 24 bytes ending `E9 28 2F 06 00` (→ `0x140211960`) | checked only |
| `0x1401A95AB`, `0x1401A99E9` the byte-0x60 tests | `BA 60 00 00 00 33 C9 E8 49 72 00 00 84 C0 75 07 33 C9`, `8D 57 60 33 C9 E8 0D 6E 00 00 84 C0 75 07 33 C9` | checked only (end at the calls) |
| `0x140211AA0` player group load | (models' hook site) | called (through the models' wrapper when it is in) |
| `0x1401F5D30` draw the Keyblade | (M5) | called |

Logs: `magic fx: Re:CoM's battle set (spell effects) in the field at combat = kh1 in Sora's story:
…` at start; up to 40 lines `magic fx: field room: Re:CoM's battle set (PLAYER group 1001) loaded or
queued …`, `magic fx: camp opened: the battle set stays loaded …`, `magic fx: Combat = KH1: …`.

1:1 / checks: Checked (decomp + exe) the cause and every address above; tests
(`magic::tests::field_fx_wants_the_battle_set_only_for_kh1_casts_in_soras_story`,
`sites::tests::magic_field_fx_sites`, the movement test's Keyblade draw). **Not checked:** in game;
which EFFECT pack holds which name; the loader's time for the set at a room change.

What the owner should test:

1. Log: the `magic fx:` start line; entering a field room: `magic fx: field room: … group 1001 …`.
2. 13F Entrance Hall (no enemies), Combat = KH1: each spell. Sora draws the Keyblade (Re:CoM's
   `kb_appear` flash) at the command and holds it for the cast; Fire's ball, Blizzard's shards,
   Thunder's charge on the blade and hand effect, Cure's sparkle, Gravity's sphere, Stop's and
   Aero's effects all show (and their sounds, if they ride the effects).
3. Open and close the camp, cast again: effects still there (log `camp opened: the battle set
   stays loaded`).
4. Combat = Re:CoM in the camp, close, walk to another room, back to KH1 in the camp, close, cast:
   effects there (log `Combat = KH1: …`).
5. Room changes in the field: note whether the load pause got longer than before.
6. A battle: casts and Re:CoM's own magic cards (Combat = Re:CoM) look as before.

## Strikes and Re:CoM's contact (the battle-end soft-lock; owner, 0.47.0: "ending a battle with magic still softlocks the game")

Owner's log `kh1-recom-softlock-0470.log`, battle 2: the manager stayed in state 1 ("children 5,
enemy count 0") with Sora's win pose done; the five children were EXP prizes (kinds 0xD0, 0xD0,
0xCC, 0xCC, 0xCB) whose words `+0x240..+0x25C` read `[1 … 0x3E99999A … 1 0]`. They were already in
that state on the first battle-end line (the frame of the last kill), i.e. for the whole rest of
the battle (two Blizzagas and a dozen Firagas), while the prizes of the other kills sat in state 0
and were picked up normally after the win. The same battle had two Thundaga bolt kills; the
0.24.2 soft-lock (`recom-m6-battles.md` § Battle end) also ended on Thundaga bolt kills, while
battle 1 of that log, also ended by bolts, returned.

### Re:CoM's EXP prize (kinds ≥ 0xCB). Checked (decomp + disasm)

- **Made** by the factory `0x1401A89C0` → init `0x1402315B0` (state machine at `+0x278`: enter
  `0x14022EDE0`, update `0x140231010`); the enter sets the timer `+0x160` = 4500 ticks, the launch
  `0x1401D5610(mover, +0x24C)` and state 0 (3 while flag (1, 0x21), the battle-entry flag, is set).
- **Words:** `+0x240` state; `+0x244` kind; `+0x248` amount; `+0x24C` launch speed (0.25, then 0.3
  once pulled); `+0x250` its effect; `+0x254` not written by it; `+0x258` "may be pulled" (set
  when the first timer runs out, at once for a new prize); `+0x25C` **"arrived"**; `+0x260` the
  reward mode (`0x140230F90`).
- **State 0** (lying): timer out with `+0x258` → state 2 (fade 1500 ticks, then gone): an EXP
  prize nobody pulls ends by itself. Else bounce (sound `0x140230E70`), then `0x140230C70`: flag
  (1, 0x21) clear and (its y < −75, or `+0x258` and its distance² to the prize player
  `[0x14087B380]`'s mover ≤ (player `+0x550`)²) → state 1 (gravity off, map collision off:
  mover `+0x118 |= 1`). The player's `+0x550` is 5 m in battle (`CBTL::resetNowEnchant`
  `0x140161E10`) and 50 m once Sora's win pose is up (the win exe `0x1401EC500`, sub 2 → 4).
- **State 1** (pulled): toward the player's vtable `+0x50` point (`0x1401F0A10`: `+0x4B0`) at 0.3 m
  a frame, turning 25 % of the angle a frame (`0x1401D6B40` → `0x1401D6A10`, `0x1401D4670`), no
  map collision; **only** when `comiss 4.0, dist²` passes (`0x140231318`: `jb` out on "4 <
  dist²" **and on an unordered compare**) is `+0x25C` set, the pickup sound played and the
  3000-tick shrink started, after which the reward `0x140230F90` and the delete run. State 1 has
  **no timeout**. A finite prize arrives (worked out from those numbers: a circle around a still
  point needs a turn of 0.3 / r a frame, which 25 % of at most π reaches only below r ≈ 0.4–0.8 m,
  inside the 2 m arrival; Checked (decomp) for the numbers, the bound is arithmetic). A prize whose
  position is non-finite never does, and `0x140230C70` sends it to state 1 at its first update
  (both of its compares are unordered → "pull"). So a non-finite prize holds the battle manager in
  state 1 for ever.

### Where the prize's position comes from. Checked (decomp + disasm)

- Re:CoM's defeat `0x14017BBC0` takes **the killing hit's body** (`enemy+0xF8` → `+0x50`) and
  makes the death effect (`0x1401A68B0`) and **all** the enemy's prizes (`0x140181300` →
  `0x1401810A0` EXP, `0x140186480`, `0x1401867F0`, `0x140180D20` cards) at that body's shape
  `+0x50` (`0x14017BC97..0x14017BCBD`); only a death without a hit body uses the enemy's position.
- A shape's `+0x50` is **the contact point**, written (with the normal `+0x40`) on both shapes
  only by Re:CoM's overlap tests when they find a contact (`0x1401777E0`, `0x140176FA0`, …), which
  its collision pass `0x14016EAD0` runs before every apply. A new shape (`0x1401720E0`) gets
  `+0x10..+0x3C` and `+0x60..` only: **`+0x40..+0x5C` hold whatever the allocation held** until a
  first contact.
- 7.6's bolts (and 7.8's impact, 7.9's Stop) applied the attack (`0x14016F1C0`) **directly**,
  without the test: so a bolt kill made the death effect and the prizes at the body's last
  contact from Re:CoM's own pass (a stale point), or, on a body never touched by it, at garbage.
  Garbage that is non-finite gives the soft-lock above; finite garbage gives prizes somewhere odd
  (which time out). This is why it is intermittent and why it follows Thunder kills. KH1 swings
  run Re:CoM's tests (`win::attack`, Stage 9) and are not affected; Fire and Blizzard are collided
  by Re:CoM's own pass.
- **Not proven from this log:** the five prizes' positions (the 0.37.0 diagnostics read `+0x30` of
  the prize, not its mover's: every prize printed (−0, 0, −0)). That they came from the Thundaga
  bolt kill of Tornadostep 577 is an **Assumption** (their creation order and their EXP split,
  which differs from the other Tornadostep's set). The mechanism above is Checked (decomp); that
  their positions were non-finite is the only reading of Re:CoM's code that keeps a prize in state
  1 for a minute, and the new log lines say it directly.

### Fixed (native): Re:CoM's contact for every strike of ours

- `win::magic::strike_body` strikes as Re:CoM's pass strikes one body: the enemy's first live hurt
  body the pass would test against the shot's attack (its key `body kind | attack kind << 4`,
  0x22 → `0x1401777E0`, 0x24 / 0x25 → `0x140176FA0(…, 0)`; flags `& 0xE == 6` skipped:
  `crate::magic::strike_contact`), **Re:CoM's own overlap test** with the shot's sweep set
  straight down from above the body onto its centre (the bolt falling onto it; the end inside the
  body, so the test always contacts), then the apply. Re:CoM's test writes the contact; Re:CoM's
  defeat makes the death effect and the prizes there; the hit effect's angles come from a real
  sweep (straight down) instead of a zero-length one. Used by Thunder's bolts, Gravity's impact
  and Stop's strikes. No new hook; the two tests are the KH1 blade's (`sites::ATTACK_CALLEES`), the
  context `[0x14087A870]` and the call shapes checked at the pass's call sites
  (`sites::MAGIC_CHECKED`: `0x14016EBAF`, `0x14016EBE5`).
- KH1 numbers unchanged (KH1's strike order, area, timing, damage); Re:CoM's contact only decides
  where its own hit effect, death effect and prizes are.
- **The win:** Re:CoM starts no card once its battle is won (`0x1401E9180` answers no while
  `player+0x428 & 4`); the KH1 frame now takes no new command then either (`Host::battle_won`):
  the log's Firaga after "battle end: started" (shot 670) is refused, the running cast ends and
  Re:CoM's end action enters the win, as with a card. Not the soft-lock's cause (its prizes were
  stuck before the win), a deviation from Re:CoM's rule.
- **Diagnostics:** the battle-end line's prizes now give the mover's position, velocity and flags
  (NON-FINITE marked), the timer, the step and `+0x260`, and the prize target (`+0x4B0`), body and
  radius; the defeat-prize wrapper logs a non-finite defeat point ("battle end: enemy … defeated
  at a non-finite point").
- Tests: `magic::tests::strike_contact_*`, `movement::tests::no_new_kh1_command_once_re_coms_battle_is_won`.
- **Items (8a.1) and other KH1 actions:** an item use hits nothing (no prize path); the KH1 swings
  use Re:CoM's tests; Aero's wind is collided by Re:CoM's pass. None has this failure mode. Item
  use after the win is refused as above.
- Not checked in game.

## Stop on attacking enemies (owner, 0.53.0: "stop doesnt seem to work")

Owner's log `kh1-recom-stop-0530.log`: eight Stopga strikes (Creapplant 333 four times, Tornadostep
906 / 907 twice each), every one `Re:CoM's contact and apply -> Ok(1)` and `hit stop3 on …: KH1
resist[9] … ticks`, and **none** with the suffix `, Re:CoM's stop state, its attacks off`, which
`stop_attacks_off` prints when Re:CoM's handler has set the enemy's next state to its stop state
(`+0x68` = `+0x940`). So the strike reached Re:CoM's handler and the handler never chose the stop.

### Cause. Checked (decomp + disasm; the per-strike reading: Assumption, see below)

- Re:CoM's enemy hit handler `0x14017F7E0` returns at once, before its kind specials
  (`0x14017DC50`, kind 8 = stop) and its reaction picker (`0x14017D260`, case 8 → `+0x940`), for an
  enemy whose `+0x260` bit 0 (in a card attack) is set (`0x14017F8CB`: `test cl,1; jne` → return 0),
  unless bit `0x2000000` (card interrupt) is set too, in which case it ends the card
  (`0x14017E1D0`) and clears bit 0 first (`0x14017F8A4..0x14017F8C4`).
- M6's cardless attacks (Stage 28) set bit 0 for every attack (the log's `flags …810 -> …005`), and
  at `enemy_pace = kh1` enemies attack almost all the time (the log: `attacking_now` 1–3 of 2–3
  enemies; 4 of the 5 Firaga hits on Creapplant 333 broke an attack).
- Every other KH1 hit sets `0x2000000` for the call (decision 14: swings since Stage 9, spells in
  `spell_hit` since 7.4). **`stop_hit` never did** (since 7.9, 0.23.0): a Stop on an attacking
  enemy was dropped by Re:CoM's handler without a trace.
- **Not a regression from 0.50.0.** 0.50.0 added Re:CoM's overlap test before the same apply
  (`strike_body`); the apply answered 1 and our hit wrapper ran for every strike. The overlap test
  doesn't touch the enemy's flags, the part's kind (`+0x80`, the attack's `+0x44`, written by the
  apply `0x14016F1C0`) or value. Stop was never seen working in game before 0.53.0 (no tester
  thread mentions it).
- That each of the eight strikes met an attacking enemy is an **Assumption** (the old log line
  didn't print the handler's answer or the flags); the new line does.

### Also: the duration was clipped

The log's max MP 12 gives (2 + 12) × 60 = 840 frames = 42000 ticks; the hit's value (the part's
`+0x92`, i16, read by `0x14016FDB0` into the info's `+0x34`) held 32767 (655 frames). Re:CoM's own
fields are i32: kind 8 stores `+0x2D8` (`0x14017DCC6`: `cvttss2si eax,xmm6; mov [rbx+0x2D8],eax`)
and the stop state's begin copies it into its timer (`0x14017C7F4`: `mov eax,[rbx+0x2D8]; mov
[rbx+0x160],eax`). Checked (disasm). (7.9's "the longest, 600 frames at max MP 8" no longer holds:
the Rod list and the KH2 Keyblades raise max MP past 8.)

### Fixed (native; no new hook or site)

- `stop_hit`: an enemy in an attack (`+0x260 & 1`) gets Re:CoM's card-interrupt bit `0x2000000` for
  the handler call, exactly as `spell_hit` does (`magic::stop::interrupts`). Re:CoM then ends the
  attack and runs its own kind 8 and stop state. Not for an immune enemy (`resist[9]` 0: KH1 doesn't
  interrupt it; Re:CoM keeps its own answer).
- When the handler has entered the stop (`+0x68` = `+0x940`) and KH1's duration is longer than the
  i16 carrier, the whole duration goes into Re:CoM's `+0x2D8` before its begin copies it
  (`magic::stop::timer_after_entry`, `win::magic::stop_timer`).
- KH1's numbers unchanged ((2 + max MP) × 60 × resist[9] frames, area, order, every 4 frames). The
  0.50.0 contact (Re:CoM's overlap test before the apply) is kept.
- Logs: `hit stop3 on …: … (its attack broken), Re:CoM's stop state, its attacks off[, Re:CoM's
  timer given the whole duration]`, or `…, NOT stopped: Re:CoM's handler -> r, flags 0x…, next
  state +0x…`; a listed enemy outside the area: `stop3: 0x… passed over (no hurt body in the area,
  as KH1)` (the log's "2 enemies listed" with one strike: the other was outside, Assumption until
  this line shows it).
- Tests: `magic::tests` (Stop's ticks at max MP 12, the carrier, the timer write, the interrupt on
  the log's flags).

### Thunder and Gravity

Both strike through `spell_hit`, which already sets the interrupt bit, and their kinds (flinch /
knock-away) go through Re:CoM's normal damage path: not affected. 0.50.0's `strike_body` hits only a
body Re:CoM's pass would hit (`strike_contact::test_for`); bodies flagged `& 0xE == 6` are skipped
(as Re:CoM's pass). Checked (code); the log has no Thunder or Gravity casts to confirm in game.

### In-game check

1. Several Heartless that attack a lot (Creapplants, Tornadosteps): Stopga. Each in the area
   freezes in Re:CoM's stop look, also mid-attack; log `hit stop3 … (its attack broken), Re:CoM's
   stop state, its attacks off`.
2. With max MP 12: about 14 s (840 frames) at 100 %, 8.4 s at 60 %; log `…, Re:CoM's timer given the
   whole duration`.
3. Hit a stopped enemy: no damage until it thaws, then the held hits land (M7 f3).
4. Any `NOT stopped` line: send the log (it names Re:CoM's answer and the enemy's flags).
5. Thunder / Gravity still hit attacking enemies (`(card broken)` on their hit lines).

## Spell damage at Damage = Re:CoM (owner, 2026-11-01)

> "spells and specials do Re:CoM's card / sleight damage" (each KH1 spell its Re:CoM magic card's)

Setting `damage = recom | kh1` (`recom-kh1-attack.md` § Re:CoM damage setting; Re:CoM by
default). At Damage = KH1 this section is off (KH1's spell damage as above).

### How Re:CoM computes a magic card's and a magic sleight's AP. Checked (decomp + exe)

- **Cards** (`initMagic` `0x1402203D0`, card `+0x7F4` = 0x1A..0x20): `+0x638` = row 0x22..0x28,
  `+0x3C8` = that row; AP from `BattleCardList[card][row+0xA]`: Fire / Blizzard / Thunder / Aero
  **power × attack stat `+0x43C` ÷ 100**; Cure power × 10 ÷ 100 (= power ÷ 10, the heal); Gravity
  and Stop **the power as is**. Then `+0x668` = row `+0xB`, `+0x66C` = card `+0xB`, status = card
  `+9`, and `0x1401E66A0` (the enemy-card rewrite).
- **Sleights** (`initStockSkill` `0x140220DF0`, sleight `+0x82C`): Fira 0xD → row 0x29, Blizzara 0xE
  → 0x2B, Thundara 0xF → 0x2E, Cura 0x10 → 0x36, Gravira 0x11 → 0x38, Stopra 0x12 → 0x3A, Aerora
  0x13 → 0x3C; Firaga 0x14 → 0x2A, Blizzaga 0x15 → 0x2C, Thundaga 0x16 → 0x2F, Curaga 0x17 → 0x37,
  Graviga 0x18 → 0x39, Stopga 0x19 → 0x3B, Aeroga 0x1A → 0x3D. AP from
  `StockSkillList[sleight][row+0xA]` by the case's mode: 1 = power × attack stat ÷ 100 (Fira,
  Blizzara, Thundara, Aerora and their -ga), 2 = power ÷ 10 (Cura, Curaga), 3 = power as is
  (Gravira / Graviga, Stopra / Stopga). Attribute `StockSkillList +0xA`, status `+0xB`, then the same
  rewrite.
- Every one of these rows has column 0 (exe; `magic::RECOM_*_ROWS`).
- Powers in the session's `SY0002.BIN` (Checked (files)): Fire 500 / Fira 1000 / Firaga 1500;
  Blizzard 400 / 700 / 1100; Thunder 350 / 600 / 900; Cure 500 / 1500 / 3000 (→ 50 / 150 / 300);
  Gravity 20 / 40 / 70; Stop 9000 / 15000 / 21000; Aero 300 / 600 / 900. The DLL reads the live
  tables, not these numbers.
- Re:CoM's kinds (`0x14017DC50`, called by the hit handler `0x14017F7E0` with the hit's value
  `*(+0xF8)+0x34` as a float, the attribute's resistance % and the attack's kind `+0x80`;
  Checked (decomp + disasm)):
  - **0xD (Gravity)**: refused (returns 0) when the param block's gate `+0x308 → +0x40` is 0;
    else `value = (float)current HP +0x274 × AP / 100.0`, then, when `+0x2B0` ≠ 0, the frozen
    bonus `0x14015D940` on the int (see below); then `0x14017DAB0`: `(int)(value × resistance % ×
    +0x290 / 100)`
    (doubles), HP −= it (the veto `0x140180920`), at least 1. The hit handler skips its own boss
    rule for kinds 8 / 0xD / 0x10 / 0x14 / 0x17 (mask `0x912100`). The reaction picker's case
    0xD (`0x14017D626`) sends the enemy to its **flatten state `+0xAC0`** (exe `0x140183BA0`:
    height = timer ÷ 250 down to 0.1, then **held 9000 ticks = 3 s** (`0x140183CFB`), then 1.0).
    An immune enemy: the handler's "no damage" branch (HUD popup 6, vtable `+0x3F0`).
  - **8 (Stop)**: refused when the gate `+0x308 → +0x2C` is 0; else `+0x2D8 = (int)value` (no
    resistance, no frozen bonus); the picker's case 8 → stop state `+0x940`, whose begin copies
    `+0x2D8` into its timer (`0x14017C7F4`). So the AP **is** the duration in ticks.
- **The frozen bonus `0x14015D940`** (Checked (decomp + exe + files); it is **not** a boss rule,
  which 0.59.0's notes called it): `value += StockSkillList[0x23] column 1 × 10 / 100` (1000 in
  the session's `SY0002.BIN` → **+100**). Its test `+0x2B0` is the handle of the **"ice"** effect
  (`0x1407C2C10` → "ice") that the Freeze state's begin `0x14017C190` starts (id 3; timer `24000 ÷
  Freeze`) and its exe `0x140183AD0` / the cleanup `0x14017EAA0` clear: **a frozen enemy takes
  100 more**. The hit handler adds it to the hit value before `0x14017DC50` (not for kinds 8 /
  0xD / 0x10 / 0x14 / 0x17, mask `0x912100`, and not at resistance 0); kind 0xD adds it to its %
  result instead.
- **Re:CoM's boss reduction is the falloff** (Checked (decomp)): for ids 100..199 (bosses) each
  hit the handler takes (`0x14017FBF4..`) counts `+0x288`; from the 6th, `+0x290` −= 0.25 (down to
  0.05); `+0x28C` = 1500 ticks; the enemy update `0x140185320` counts it down by the enemy's dt and zeroes `+0x288`, puts
  `+0x290` back to 1.0 when 1500 ticks (30 frames) pass without a hit. `0x14017DAB0`
  multiplies every damage by `+0x290`. Stage 106 restores these three fields after each hit on a
  boss whose revenge acts (falloff off for them); Damage = Re:CoM does the same for Gravity
  Break / Zantetsuken hits (owner, follow-ups 2; `win::revenge::Falloff`).

### KH1 level → Re:CoM card / sleight (`damage::spell_source`)

| KH1 | level 1 | level 2 (-ra) | level 3 (-ga) | at Damage = Re:CoM (battle room) |
|---|---|---|---|---|
| Fire | card 0x1A, row 0x22 | sleight 0xD (Fira), row 0x29 | 0x14 (Firaga), 0x2A | **Re:CoM's AP** (power × stat ÷ 100), attribute 2 |
| Blizzard | card 0x1B, 0x23 | 0xE, 0x2B | 0x15, 0x2C | **Re:CoM's AP**, attribute 3 |
| Thunder | card 0x1C, 0x24 | 0xF, 0x2E | 0x16, 0x2F | **Re:CoM's AP** for each strike, attribute 4 |
| Cure | card 0x1D, 0x25 | 0x10, 0x36 | 0x17, 0x37 | Re:CoM's heal 50 / 150 / 300 (already, decision 34) |
| Gravity | card 0x1E, 0x26 | 0x11, 0x38 | 0x18, 0x39 | **Re:CoM's Gravity**: AP 20 / 40 / 70 (as is), **kind 0xD**, attribute 5, status 0x93 |
| Stop | card 0x1F, 0x27 | 0x12, 0x3A | 0x19, 0x3B | **Re:CoM's Stop**: AP 9000 / 15000 / 21000 ticks (as is), **kind 8**, attribute 5 |
| Aero | card 0x20, 0x28 | 0x13, 0x3C | 0x1A, 0x3D | the **wind**'s hits: **Re:CoM's AP** (300 / 600 / 900 × stat ÷ 100), attribute 1; KH1's barrier |

Powers, attributes and status from the session's `SY0002.BIN` (Checked (files)); the rules and
the kinds from `initMagic` `0x1402203D0` cases 0x1E / 0x1F / 0x20 and `initStockSkill`
`0x140220DF0` cases 0x11 / 0x12 / 0x13 / 0x18 / 0x19 / 0x1A (modes 3, 3, 1, 3, 3, 1), rows' `+0xB`
read from the exe (0x26 / 0x38 / 0x39: 0xD; 0x27 / 0x3A / 0x3B: 8; 0x28 / 0x3C / 0x3D: 1 / 2 /
3). Checked (decomp + exe). The DLL reads the live tables, not these numbers. Tests
(`damage::tests`): `gravity_maps_to_recoms_card_and_sleights`,
`stop_maps_to_recoms_card_and_sleights`, `aero_wind_maps_to_recoms_card_and_sleights`,
`spell_interrupts_follow_recoms_gate`.

### What runs (Combat = KH1, Sora's story, Damage = Re:CoM)

- **Cast** (`win::magic::start_cast` → `win::damage::spell`, once per cast, in a battle room):
  after the cast's kind and attribute are written, every spell above but Cure writes **the Re:CoM
  AP and attribute of its card / sleight** into `+0x660 / +0x66C`, Gravity and Stop also **Re:CoM's
  kind** into `+0x668` (0xD / 8, `damage::recom_kind`), and runs Re:CoM's rewrite `0x1401E66A0`,
  as `initMagic` / `initStockSkill` do. Each release reads those fields into its shots: Fire's
  homing ball, Blizzard's shards, Thunder's strikes, Gravity's sphere (and its held carrier),
  Stop's strikes, **Aero's barrier shot** (its wind's attack: descriptor `+0x68 / +0x6C / +0x70`
  from the player's fields, no longer the attack stat and attribute 1 directly; identical at
  Damage = KH1, where the cast writes those).
- **KH1's cast, area and target order are kept** for all of them (owner): wind-ups, releases,
  Thunder's / Stop's nearest-first lists and intervals, Gravity's fall, growth and 1.7 × S m impact
  sphere, KH1's squash under the falling sphere, Aero's barrier, its 0.8 m wind and 30-frame
  re-hit, the MP.
- **Hits** (enemy-hit wrapper, `win::damage::recom_spell_hits`): no KH1 number is written and the
  attribute is not neutralised: Re:CoM's path applies the AP with **Re:CoM's own resistance**,
  weak parts, its frozen bonus and its boss falloff:
  - **Gravity**: Re:CoM's kind 0xD with its gate `+0x40` (immune: no damage), % of the current
    HP, frozen bonus, resistance, falloff, and its **3-second flatten**. KH1's squash leaves an enemy in the
    flatten state (`win::magic::flattening`: no spring-back write, not squashed again under
    another sphere), as KH1's squash skips flattened actors. No KH1 `resist[8]`, no floor.
  - **Stop** (`win::attack::recom_stop_hit`): Re:CoM's kind 8 with its gate `+0x2C`; the
    duration is the AP. No KH1 `resist[9]`, duration or whole-duration write. Kept from KH1's
    Stop: the stopped enemy's attack bodies off, the held hits' 6–11-frame pace (M7 f3).
  - **Aero's wind**: each re-hit carries Re:CoM's AP and attribute 1; the hit keeps KH1's
    reaction (the KH1 row's kind 1), as Fire / Blizzard / Thunder.
  - **Aero (level 1) gets a wind** (owner, 2026-11-01, follow-ups 2, "1 (b)"; not KH1: KH1's
    level 1 has no wind record, `aero::has_wind`) when the cast took the Aero card's AP
    (`damage::aero_wind`, `win::damage::recom_cast`): card 0x20, **300 × attack ÷ 100**, attribute
    1. Its shape and pace are **Aerora's KH1 wind** (btltbl `0x5CF8`, the nearest KH1 data, the
    same numbers as Aeroga's `0x5D68`): a sphere of **80 cm** × Sora's size (size 1.0:
    Assumption, as for Aerora) at his body centre, **re-hit every 30 frames** (Re:CoM's new hit
    serial `0x14016F7D0`), **alive for the whole barrier** ((18 + max MP) × 60 frames, KH1's
    level 1's own barrier), KH1's flinch (the KH1 row's kind 1). Its look is Re:CoM's Aero card's
    (row 0x28's effect set 24). All of these are Aerora's or the barrier's numbers used for level 1:
    **Assumption** (no KH1 level-1 wind to read). At Damage = KH1, and in the field (no card AP),
    level 1 stays KH1's: the barrier only.
  - **Card interrupt** (decision 14): an attacking enemy's card is broken for the hit, **unless
    Re:CoM's gate for the kind refuses it** (`damage::interrupts`): an immune enemy's attack goes
    on, as KH1's Stop on a `resist[9]` 0 enemy.
- **In the field** (`player+0x390` = 0): Re:CoM has no magic there. Fire / Blizzard / Thunder /
  Aero's wind keep the cardless AP (the attack stat) through Re:CoM's path, as the swings; Gravity
  and Stop keep **KH1's rules** (their Re:CoM numbers only exist with a card's AP and kind: their
  hits are Re:CoM's only when the cast wrote them, a per-spell flag set by `win::damage::spell`).
- At Damage = KH1 nothing of this runs (KH1's spell damage, Gravity's KH1 % and Stop's KH1 duration
  as before).

### Logs and trace

- `damage: fire2: Re:CoM sleight 0xd (row 0x29) column 0: power 1000 x attack 10 / 100 = AP 100
  (attribute 2)`; `damage: gravity1: Re:CoM card 0x1e (row 0x26) column 0: power 20 as is = AP 20
  (attribute 5, Re:CoM's kind 0xd)`; `damage: stop3: … power 21000 as is = AP 21000 (attribute 5,
  Re:CoM's kind 0x8)` (first 48 computations).
- `magic: hit gravity1 on …: Re:CoM AP 20 (recom 20 col 0 card 0x1e) (max MP 3), Re:CoM's gate
  open, Re:CoM took 160 -> HP 640, Re:CoM's flatten state`;
  `magic: hit stop1 on …: Re:CoM AP 9000 (…), Re:CoM's gate open, Re:CoM's stop state for 9000
  ticks (180 frames), its attacks off` (or `gate shut: immune`).
- Trace hits `…:gravity1:recom 20 col 0 card 0x1e`, `e…:stop1:9000t:recom 9000 col 0 card 0x1f`,
  `…:aero2:recom 78 col 0 sleight 0x13`.

### Sites

`recom-kh1-attack.md` § Re:CoM damage setting § Sites: `0x14017DD4C` (kind 0xD gate and %),
`0x14017D626` (case 0xD → flatten), `0x140183CFB` (the 9000-tick hold), checked at start with
the setting's other sites; kind 8's `0x14017DCB5` was already checked (7.9). No new hook.

### Owner questions (Damage = Re:CoM): answered 2026-11-01

1. Gravity: (b) Re:CoM's own Gravity card / Gravira / Graviga (built above).
2. Stop: (b) Re:CoM's durations with Re:CoM's gate (built above).
3. Aero: the wind's hits take Re:CoM's Aero by level (card, Aerora, Aeroga; built above).

Follow-ups 2 (owner, 2026-11-01, on 0.59.0's questions):

1. **(b)** Aero level 1 gets a wind with the Aero card's value at Damage = Re:CoM (built above).
2. **(a)** Stopped enemies keep KH1's held-hit pace and attacks-off (no change).
3. **(b)** "Gravity Break and Zantetsuken skip Re:CoM's boss reduction": the **falloff** `+0x290`
   (the question's "boss rule" `0x14015D940` is the frozen bonus, above, and stays). Built:
   `recom-kh1-attack.md` § Gravity Break and Zantetsuken.
4. **(a)** Aero's wind in the field keeps Re:CoM's no-card AP (the attack stat), as Fire (no
   change). Aero (level 1) in the field: no card AP, so no wind (KH1's barrier only).

### What the owner should test

1. In battle, cast Fire, Fira (needs the -ra level), Firaga: the log's `damage: fire…` lines show
   cards / sleights 0x1A / 0xD / 0x14 and the AP; the trace's hits `:recom <ap> col 0 card 0x1a`
   (etc.); damage as Re:CoM's Fire card / Fira / Firaga sleights at Combat = Re:CoM against the
   same enemy (resistances included: a fire-weak enemy takes more).
2. Blizzard (each shard that hits) and Thunder (each strike): the same.
3. Cure: unchanged (50 / 150 / 300).
4. **Gravity / Gravira / Graviga**: KH1's cast and falling sphere; at the impact each enemy under
   it loses 20 / 40 / 70 % of the HP it has left (Re:CoM's resistance and boss falloff on top) and is
   **flattened for 3 seconds** as by Re:CoM's Gravity card; an enemy Re:CoM's Gravity can't touch
   takes nothing (log `gate shut: immune`) and its attack goes on. No pop back to full height
   during the flatten.
5. **Stop / Stopra / Stopga**: KH1's cast, area and nearest-first order; each enemy reached stops
   for **3 / 5 / 7 s** (9000 / 15000 / 21000 ticks), whatever Sora's max MP; enemies Re:CoM's
   Stop can't stop aren't stopped (log `gate shut: immune`). Held hits land after it, 6–11 frames
   apart.
6. **Aerora / Aeroga**: the wind's hits (every 30 frames while the barrier lasts) do Re:CoM's
   Aerora / Aeroga AP (600 / 900 × attack ÷ 100); trace `:aero2:recom …`. The barrier still halves
   Sora's damage (KH1's).
7. **Aero (level 1), in battle**: the log's `release aero1: … Aerora's KH1 wind 0.8 m re-hitting
   every 30 f with the Aero card's AP (Damage = Re:CoM)`; enemies within about 0.8 m of Sora take
   a hit every half second while the barrier lasts, each the Aero card's 300 × attack ÷ 100
   (attack 13: 39); trace `:aero1:recom 39 col 0 card 0x20`. In the field: barrier only.
   `damage = kh1`: barrier only (KH1).
8. In the field: Gravity and Stop as before (KH1's); `damage = kh1`: everything as before.
