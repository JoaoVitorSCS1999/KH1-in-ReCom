# M11: KH1's other Heartless in Re:CoM's battles

Owner's ask (2026-11-02): "it works perfect. Now let's bring in the remaining KH1 adjacent heartless".
M11 is every ordinary Heartless with a KH1 twin (not the bosses), under M10's rules
(`docs/design-philosophy-instructions.md` § Phase 4 / M10 choices / M11): **KH1's AI, attacks, reactions and
motions; Re:CoM's stats and everything else; native states; `enemy_ai = kh1 | recom`; battle enemies only;
Re:CoM's attack power per attack; Re:CoM's Stop / freeze / Gravity states; KH1's clash recoil on Guard.**

Step 1 (this note): a shared framework made from the Shadow's code (M10, `docs/recom-m10-shadow.md`), proven
on the Soldier; a checklist and the families for the other 29.

Tags: Checked (exe / decomp / files / trace) or Assumption. Addresses are Steam (KH1 FM and Re:CoM), image
base `0x140000000`. Script offsets are offsets in the player's own `.bd` (part 5 of the twin's `.mdls`).

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Soldier yet.

## The framework

| Part | Where | What |
|---|---|---|
| Engine | `kh1-core` `enemy/` | `body.rs` (KH1's actor movement: turn, walk, arc, air steering, push slide, the new `held` for request bit 23), `react.rs` (KH1's reactions, `0x1402B8E10` and its cases; the clash; landing / lying / get-up; the hurt part nearest a contact point), `script.rs` (the script runtime and the shared routines), `mod.rs` (`Numbers` / `Engine` / `Table`, `Behaviour`, `HitNews`, KH1's exe numbers `EXE_NUMBERS`, `MOVE_BLEND`) |
| Behaviours | `kh1-core` `shadow.rs`, `soldier.rs` | each enemy's own script on the shared pieces; both implement `enemy::Behaviour` |
| Runtime table | `kh1-recom` `kh1_enemy.rs` | `CLASSES` (per Re:CoM class: RTTI, id, vtable / slots, allocation size and tail magic, ctor and factory-size sites, vtable slot checks, the Re:CoM attack state of each KH1 attack, attack names, appearance kind, `reads_hp`, manifest), the generic manifest `kh1-enemy 1`, `make` (a class's behaviour), log tokens |
| Runtime | `kh1-recom` `win/kh1_enemy.rs` | M10's runtime made generic: a ctor wrapper per class (`ctor::<K>`), the factory size patch, the vtable copy, the per-enemy `Runtime` and tail, the states, the reactions, the hittable switch, Guard's clash, the trace token; per-class log prefix (`shadow:`, `soldier:`) |
| Sites | `kh1-recom` `sites.rs` | per class: `<CLASS>_HOOKS` (ctor, in the hook-shape test), `<CLASS>_ALLOC`, `<CLASS>_PTRS`; the shared callees stay in `SHADOW_CALLEES` |
| Asset step | `kh1-recom-assets` `enemy.rs` | `Spec` (one per enemy) and `build`: the `.ANN` (KH1 clips at slot 100 + entry), `.ASD` (one record per attack row, the row's hit slot, the Re:CoM template by attack id), `.SES` (Re:CoM's sounds), the manifest; script constants (float and integer pushes), request-word checks, the weight, btltbl's globals, the hurt parts from the collision block |
| Specs | `kh1-recom-assets` `soldier.rs` | the Soldier's offsets, requests, rows, Re:CoM ids; `enemy::SPECS` lists the built enemies |
| KH1 trace | `kh1-trace` | the enemy rows gain `kind` / `spawn` (syscalls 114 / 132's numbers) to tell enemies apart |

The Shadow runs on it unchanged: its tests are unchanged and pass, its asset outputs are byte-identical
(Checked: `EN0001.ANN` / `.ASD` / `.SES` / `kh1-shadow.json` hashed before and after on the session's files),
its manifest stays `kh1-shadow 1`, its log lines and trace token are the same. Its ANN and ASD now go through
`enemy::extend_ann` / `enemy::extend_asd` (the same code); its SES and manifest writers stay its own.

### Shared script routines. Checked (files)

KH1's enemy scripts share routines byte for byte (their branches and calls are relative). The Shadow's and
the Soldier's share 26; the other 28 scripts contain 13–34 of the Soldier's routines verbatim (`.bd` scan).
Ported once in `kh1_core::enemy::script`:

| Routine | Shadow | Soldier | What |
|---|---|---|---|
| `rand_range` | `0x1FB6` | `0x1A70` | rand × (b − a) + a |
| `random_offset` | `0x1F70` | `0x1A2A` | a vector of length [a, b) at a random angle |
| `walk_to` | `0x2440` | `0x1F16` | move each frame until arrived on the ground or the time is out |
| `approach` | `0x2374` | `0x1E4A` | chase the target, re-reading it every n frames, until within d |
| `wander` | `0x1BC4` | `0x15BA` | two points about the **current** position, sometimes stand, home when too far |
| `wait_frames` | `0x2D42` | `0x23EA` | count down by the step |
| `wait_grounded` | `0x1D74` | `0x182E` | movement state 0 |
| `face_target` / now | `0x2302` | `0x1DD8` / `0x1E02` | `sys 36` / `sys 11` |
| `jump_at` | `0x2142` | `0x1BFC` | request 3 (action), 4 queued, yield, `sys 50` |
| state requests | (`wp+0x20`) | `0x1996` / `0x19B4` / `0x19EA` / `0x1A06` | the supervisor's state object |
| move toward | `sys 40` | `sys 40` | `0x140299D60` (walk / run / glide; blend 8 Checked (decomp)) |

**Shadow's wander (fixed 2026-11-02):** KH1's wander (`0x1BC4` / `0x15BA`) adds both points to the actor's
**current** position (`sys 7` with `actor+0x10`), not to home; M10's port added them to home. Owner: "fix it to
KH1's rule": the Shadow now calls the shared `wander` (same draws, same order; only the centre moved).

### What the runtime does per class (unchanged from M10 for the Shadow)

At Combat = KH1, Sora's story, `enemy_ai = kh1`, a battle enemy of a listed class gets our vtable copy: states
0 (appearance), 1 (KH1's frame with the AI), its attack states, 11 / 12 (KH1's reactions, the clash), 30
(Re:CoM's defeat with KH1's motion), think `+0x3D0` nothing. The appearance is per class:

- `Appear::Shadow`: `CEnemyShadow`'s own state 0 writes without its motion, KH1's rise, its battle-start
  handshake (M10).
- `Appear::Base` (a class that keeps `CBattleEnemy`'s state 0, e.g. the Soldier): Re:CoM's begin
  `0x14017B590` as it is (its writes, the battle message `0x96`, its appearance motion if any), KH1's
  appearance from the next frame, then Re:CoM's default state `0x14017D6F0` (what its step `0x1401829E0`
  does when its motion ends). Re:CoM's alternate appearance (`+0x260 & 0x20000000008`, the first-strike
  stun) stays Re:CoM's for every class.

New for M11:

- `Cmd::Held` (KH1's request bit 23 → actor flag `0x1000`, `0x14029F980`; the air step `0x1402AF6A0` then
  moves nothing and keeps the arc at the body's height): `Body::held`; the root motion moves the body; cleared
  when the action ends (Assumption: KH1 clears it with the next motion request without bit 23).
- The clash recoil per attack from its rows' `+0x40`; when the model has no motion for it (KH1's request names
  no entry: `0x1402A01E0` flags 0x200000), **Re:CoM's own flinch** (owner 2026-11-02, every enemy): `guard_block`
  hands the block back to `win::guard`'s pre-M10 path (Re:CoM's picker `0x14017D260(enemy, 1)`, its card end
  `0x14017E1D0`), and our state 11 runs Re:CoM's own begin / step (`0x14017C530` / `0x140184100`) until the next
  state 1; the behaviour still hears the clash (the Soldier's counter). Decision 27's toggle still gates it.
- `reads_hp`: the runtime passes Re:CoM's HP / max HP (`+0x274`, `0x14017F5A0`) at each hit (the Soldier's
  counter reads its HP ratio; Re:CoM's HP, decision M10).
- Hurt parts from the manifest (any count): the contact point's nearest part bone (`react::nearest_part`;
  for two parts it is M10's rule).

### Actions that never end (owner 2026-11-02, 0.68.0: "this soldier just decides to not attack")

The AI is not polled while an action plays (KH1's script pauses while an action motion plays). Two host paths
could leave that wait open forever; both are fixed, and a safety net now ends any wait the same way:

- **A clip Re:CoM loops.** The asset step marks a clip as looping when KH1's last pose equals its first
  (M10's rule). The Soldier's kick 200 (entry 37, 110 frames) and air kick 204 (entry 41) are marked
  that way (Checked (files): the player's manifest). Re:CoM loops them, so the motion clock (`0x1401A47C0`)
  wraps to 0 at the clip's end and never reaches KH1's 5500 units. Every kick 200 in the 0.68.0 log
  waited forever (e336 at line 3966 until it was hit at 4727; e335 and e483 the same). The trace shows
  e336 in Re:CoM state 2 with request 200 for 700 frames. Fix: `ClipWatch` treats a clock that goes back,
  once half the motion has played, as the end of the motion. Assumption: KH1 ends an action at its
  motion's length. The kick is a one-shot motion in KH1; our loop rule marked it, not KH1. The Shadow has
  no looping action clip (its loops, 202 underground and 203 taunt, are plain motions).
- **An attack state request overwritten in the appearance's last frame.** The behaviour runs in state 0's
  step. When the rise ends and the first attack is asked for in the same frame (enter state 2: Re:CoM's next
  state), state 0's step then called the default state `0x14017D6F0`. That overwrote the next state, so the
  attack state never began. This is Shadow e213 in the second log: rise, `state engaged`, claw, then
  nothing for 1400 lines until a hit. The same code existed in M10 (`win::shadow::appear_step`, Checked
  (diff)): this one is not a refactor regression, just a rarer path. Fix: state 0 does not call the default
  state while an attack is pending, and `tick` asks for a pending attack state again while Re:CoM's next
  state is another.
- **Safety net** (`kh1_enemy::watchdog`, every class): an attack whose state has not begun 10 KH1 frames
  after the request, or an action still playing 30 frames past its motion's length, is released as if its
  motion had finished. Its queued motion plays and the AI goes on. An attack state with no KH1 action
  leaves through Re:CoM's attack end. Each release is logged past the line cap:
  `<class>: eNNN: action NNN never started/ended (Re:CoM state, motion, clock …), released`.
- Tests: `a_looping_clip_ends_at_its_wrap`, `the_watchdog_releases_stuck_actions`, and per enemy
  `a_long_fight_never_stalls` (20 000 frames, 12 seeds, Sora circling 2–7 m away, a hit every 150–400
  frames: the AI acts at least every 600 frames).
- Diagnostic for the sunk Shadow that sometimes shows standing mid-travel (owner 2026-11-02). While unhittable,
  every motion change (`plays N (slot S) after M while unhittable`) and every Re:CoM state change
  (`Re:CoM state A -> B while unhittable`) is logged. The 0.68.0 log has no command of ours between
  `underground true` and `underground false`, so the cause is not found yet; an F9 trace of it is needed.

### Re:CoM's own states hand back (owner 2026-11-02, 0.68.2: "once they got out of the stun … just standing still")

- **What happened.** Stun Impact put two Soldiers into Re:CoM's stun state. Under the M10 choices, Stop,
  freeze, Gravity and the stun keep Re:CoM's own states, outside our copies. They were stunned mid-kick (e321)
  and mid-spin-kick. On leaving the stun Re:CoM entered state 1, but our runtime still held the attack
  as the running action. Trace 20 shows both in state 1 with their action pending (200, 203) and the idle
  playing. Checked (log, trace).
  - The AI is not polled while an action runs.
  - An attack ends only in its attack state's step, so it never ended.
  - The watchdog didn't fire because a playing attack was left to `attack_step`.
- **Fix: hand-back.** Every state 1 we ask for (an attack's end, a reaction's end, the appearance's end) is
  marked. Any other state 1 begin follows a state Re:CoM owns: stun, Stop, freeze, Gravity's flatten, a
  Re:CoM flinch, the first-strike stun. At that begin (`take_over`):
  - the action, reaction, queued motion and clash are dropped;
  - the motion Re:CoM played is forgotten, so the idle plays;
  - the AI goes on from its wait, logged as `<class>: eNNN: back in state 1 after a Re:CoM state (…): action …
    dropped; the AI goes on`.

  This is KH1's rule: a status ends the current action. Assumption: the KH1 AI's next step after a status
  is the rest of its routine. Its hit handler isn't told, since Re:CoM's stun hit didn't run our state 11.
- **Fix: the appearance's default state.** It is now called outside our lock. It used to run inside, so
  `try_lock` failed and Re:CoM's own state 1 begin ran at every appearance end.
- **Safety nets added** (all logged past the cap):
  - an attack still "playing" when the AI is polled in state 0 / 1 is released;
  - a reaction that takes 600 frames is released (`REACTION_LIMIT`);
  - an AI polled in state 1 that issues nothing for 600 frames, with no action or reaction pending, restarts
    from its engaged state (`QUIET_LIMIT`, `Behaviour::reset`). KH1's longest silent wait is 180 frames.
### Flying bodies (M11 fliers). Checked (decomp + disasm) unless marked

KH1's fliers (Air Soldier, Search Ghost, Air Pirate, Gargoyle, Darkball, Wyvern) all enter **movement state 4**
once (`sys 135` → `0x1402BACB0`: `0x1402B1960(actor, 4)`, flight velocity `+0x3C0` = the script's vector ×
its fourth word) and then steer by writing the actor's `+0x20` (a unit direction, its `y` included) and
`+0x2C` (a speed) every frame. State 4 is registered by `0x1402CBF80` (`0x1402B1B80(4, flags 2, …,
0x1402CC000, keep-state 0x1402B1840)`); its step `0x1402CC000`:

- `want = +0x20 × +0x2C`; `+0x3C0 = 0x1400E7CB0(want, +0x3C0, stats+0x14)` = `v·(1 − a) + want·a` (the
  wrapper `0x140386B70` passes `(out, want, v, a)` in rcx / rdx / r8 / xmm3: Checked (disasm)), `a` = the
  stats block's `+0x14` (0.1, `0x1402A5D70`; no flier script writes it: Checked (files));
- `+0x3C4 = want.y` (vertical at once), displacement `+0x3C0 × step`; no gravity, no ground walk, no floor
  probe (the move-toward routine's probe `0x140193650` runs only outside states 2 / with flag 0x100);
- the actor task zeroes `+0x2C` each frame before the script (`0x140297FE0`, `puVar1[0xb] = 0`), so a frame
  the script doesn't steer eases the flight off;
- a push slide (`0x1402C7300`) zeroes `+0x3C0`; a hop (`0x1402AF960`) sets state 2 (flight ends);
- the reactions: state 4's flags `& 3 == 2`, so `0x1402B1AE0` is true and `0x1402B1B10` false:
  `0x1402B8E10` takes the ground forms (kind 3 the ground flinch with its push, kind 5 skips the launch and
  is kind 4: the knock-back 0x4C / 0x4D through `0x1402B77D0` → `0x1402B8CC0`, the clash the ground recoil).

Other per-script engine values the fliers use: `sys 125` (`0x1402BABA0`) sets the turn rate (stats `+0xD0`,
degrees × π / 180), `sys 89` (`0x1402BA5D0`) reads `+0x3D4` = floor y − y (the height above the floor,
written by the move `0x1402999A0`), `sys 172` (`0x14029EFC0`) is "the motion has played to its end, no blend,
nothing queued", `sys 77` (`0x140299530`) a stepped line test (0 = nothing between), `sys 121` the look-at
target (`+0x470`, KH1's head turn, `0x1402A28C0`: not ported, the look is the model's).

KH1's attack objects of mode 2 (`0x1402B47A0`) take the owner's request (`+0x164`) at their first tick
and end when it changes; they hit while that motion's HIT slot is on (`+0x1CC & 0x10000 << sub`). The fliers
make them and then play a plain request (no action bit), so the script keeps steering while the attack
plays.

Built (`kh1-core` `enemy`, `kh1-recom` `win::kh1_enemy`):

| Part | What |
|---|---|
| `Body::flying`, `Body::fly(v)` | state 4; `Body::step` eases / follows as above (`Body::walk` is `+0x20 × +0x2C`) |
| `Body::turn` | `sys 125`'s rate, else the manifest's |
| `react::react_flying` | the ground forms, kind 5 as kind 4 (the host picks it while the body flies) |
| `script::{steer, fly, height, set_turn_rate, motion_done, tick_clock}` | the scripts' writes / reads; `Core::floor`, `Core::clock` |
| `Cmd::Attack` | a plain request made with attack rows: Re:CoM's attack state, the AI keeps running; it ends when another motion replaces it, an action starts or a one-shot clip has played (not at a looping clip's wrap: the script decides; the watchdog's end margin doesn't apply, the AI isn't waiting); a new attack begins its state again (`tick` asks for it) |
| `Behaviour::set_floor` / `on_landed` (defaults: nothing) | the floor height each frame; Re:CoM's `CCollAttack` apply `0x14016F1C0` answered 1 (a hit) on the player side (from the pair-dispatch wrapper, `win::guard::on_result`) → KH1's attack hit callback (`+0x38`) |
| host | a flying body's vertical velocity is written every frame (× the mover's speed scale, Re:CoM's acceleration cancelled), it is never `grounded`; the floor is Sora's height the last time he stood on the floor (**Assumption**: Re:CoM's battle floors are flat; Re:CoM's fliers keep no floor height: their gravity is off, `+0x260 & 0x100`, `0x1401D67B0(mover, 0)`) |

Re:CoM's fliers (Checked (decomp)): every flier ctor sets `+0x154 |= 0x80` and `+0x260 |= 0x100` (gravity off in
Re:CoM's state code, `0x14017EB80`, `0x14017EED0`, `0x140198600`); their clips hold the body 0.61 m above the
actor (Air Soldier: `ex_2160_sk` at y 0.614 in `e015_001` and in KH1's idle, Checked (files)), so Re:CoM's
fliers float low; at Combat = KH1 they fly at KH1's heights (the script's own targets).

## The Soldier

### KH1's side. Checked (files: `ex_2010.bd` disassembled; decomp: the VM `0x1402CC620` and syscalls)

The script (9344 bytes) has the Shadow's shape: a supervisor thread and one behaviour thread per state.

| Routine | Offset | What |
|---|---|---|
| supervisor | `0x2152` | init `0x2298`, yield, appearance thread `0x716`; each frame: while idle (state 1) Sora within 1000 (`0x13A0`) → engaged; a state request restarts the behaviour: 1 idle `0x6D2`, 2 engaged `0x4D2`, 3 hit `0x3E8` |
| init | `0x2298` | stats record (LV1 `0x14` / LV2 `0x80`, `sys 198`), walk 3 / run 7 / hop 300, home, handlers (hit `0x234C`) |
| appearance | `0x716` → `0x16CA` | spawned by the room (`sys 132` ≠ 0): fade-in 30, face Sora now, request 69 (`0x1800045`: action + bit 23), 6 queued, a zero jump, wait while in the air; then idle |
| idle | `0x6EA` | forever wander (300, 400 about the current position, band 100, timeout 300, 40 %: idle motion [90, 180), home beyond 700) |
| engaged | `0x4F6` | first time: the look `0x758` (208, only when placed, not spawned); rounds += 1; < 350 close, < 450 mid, else far; the jump at a target above `0x7E0`; engaged again |
| close | `0x684` | kick (< 280) or double kick; after 3 rounds 70 %: back off 150–200 (run, 180) and the spin kick; the strafe |
| mid | `0x5D8` | 40 % jumping kick, 40 % spin kick, else run in (120, 350) and kick; the strafe; then 80 %: 80 % jumping kick, else run in and kick |
| far | `0x5B6` | run in (300, 450, retarget 20), 30 % an idle pause [30, 60) when < 450; face the target |
| strafe | `0xCDE` → `0x1214` | 30 %: [30, 45) frames, else [60, 75); 90° to the side (50 / 50) running at 7, turning back toward the facing by 1°/f (< 350) or 0.5°/f |
| jump at a target above | `0x7E0` / `0xFB2` | target higher (y down: target.y − y < 0): hop 200 at min((d − 50) / 60, 20), steer at 20; from 37 frames, within ±30 of its height: the air kick |
| hit | `0x40C` | < 450: after 3 counted hits the spin kick (HP < 50 %) or a kick; else 60 % kick, 40 % spin kick; ≥ 450: 60 % crouch (216) and rush (207 → 209); then engaged |
| hit handler | `0x234C` | a damaging hit counts and asks for state 3; a clash (kinds 6 / 7) asks for state 3; death `0x1540` |
| leash | `0xE62` | **not ported**: KH1 removes a Soldier 600 (out of sight, `sys 139`) / 1500 away from Sora; in a Re:CoM battle room it would never be defeated and the battle could not end |

Attacks (rows `0x70` bytes, `sys 23`; the row's `+0x34` is its hit slot: `0x1402B2DC0` writes it to the
object's bits 4..5 and the hit test reads `0x10000 << sub`, Checked (decomp)):

| Attack | Request | Rows (bone code → KH1 bone, t, radius) | Hit windows | Recoil `+0x40` | Re:CoM id |
|---|---|---|---|---|---|
| kick | 200 (110 f) | `0xEC` (29 → 74 r. hand, 10 → 0.1, 40) | [60, 70) | 218 | 0 |
| double kick | 201 (134 f) | `0xEC`, `0x15C` (28 → 106 l. hand) | [60, 70), [92, 120) | 218 | 0 |
| jumping kick | 202 (140 f, bit 23) | `0x1CC` (12 → 3 hip, 1, 65) | [76, 116) | 110 (no motion) | 1 |
| spin kick | 203 (100 f) | `0x23C` (41 → 23 r. foot, 50), `0x2AC` (8 → 33 chest, 60) | [46, 64), [0, 46) | 110 (no motion) | 1 |
| air kick | 204 (58 f, bit 23) | `0x31C` (40 → 10 l. foot, 4 → 0, 50) | [20, 30) | 210 (no motion) | 0 |
| rush | 207 (120 f) | `0xEC`, `0x15C` | [60, 70), [92, 110) | 218 | 0 |
| rush leap | 209 (92 f, bit 23) | `0x1CC` | [16, 56) | 110 (no motion) | 1 |

Reactions: the shared engine with the Soldier's motions (flinch 0x48..0x4B, air flinch 0x4E / 0x4F, launch
0x52..0x5B, air launch 0x5C..0x65; no 0x4C / 0x4D / 0x50 / 0x51: kind 4 launches); weight 1.0 in both
records; hurt parts (collision block group 2: codes 0, 12, 24, 25 → the code bones' parents 35 head, 2 hips,
99 l. forearm, 67 r. forearm → Re:CoM bones 15, 2, 29, 43). Checked (files). Same rule gives M10's Shadow
parts (test `the_shadow_hurt_parts_match_m10s`).

Motions (`xa_ex_2010.mset`, 50 entries, 60 fps): KH1's matcher on Re:CoM's `EN0002.ANN` (75 slots, 30 fps):
one exact bake (entry 32 = request 66 → `e002_070`, played as Re:CoM's); edits (≤ 20 bones differ) for entries
1, 2, 6, 20, 21, 33–37, 39 (Re:CoM's kick `e002_050` is an edit of KH1's 200, its spin kick `e002_051` of
KH1's 202, `e002_073` of the appearance 69): new clips (decision M10: Re:CoM's clip only for an exact bake).

### Re:CoM's side. Checked (exe + decomp)

`CEnemySoldier` (vtable `0x14054EA68`, 139 slots, RTTI COL at −8), ctor `0x140193510` (`40 53 48 83 EC
20`, its only caller the factory `0x140188F40` id 2; no branch into the moved bytes: exe scan), allocation
`0x140188FD7` `mov ecx, 0xCD0` → 0xCF0 (fields used up to `+0xCBC`). It overrides states 1 (walk / card
think `0x140193680` / `0x140193A10`), 2 (kick `e002_050`, `0x1401935E0` / `0x1401936B0`), 3 (spin kick
`e002_051` with `KickSpeed`, `0x140193630` / `0x140193940`); states 0, 11, 12, 30 are `CBattleEnemy`'s
(`0x14017B590` / `0x1401829E0`, `0x14017C530`, `0x14017B9D0`, `0x14017BBC0` / `0x140183260`); think
`0x140196EC0`. Our copy replaces states 0, 1, 2, 3, 11, 12, 30 and the think: the ground kicks (kick, double
kick, spin kick, rush) in state 2, the attacks that leave the ground (jumping kick, air kick, rush leap) in
state 3. `EN0002.ASD`: 2 records at `0x6C`, tail `0x1AC` (kick `e002_050` on `r_hand` id 0, spin kick
`e002_051` on `l_toes` id 1). `EN0002.SES`: `e002_031`, `e002_033`, `e002_050`, `e002_051`, `e002_073`.

### What the asset step writes

`ENEMY/0002/EN0002.ANN` (Re:CoM's 75 slots byte-identical, KH1's clips at 100 + entry), `.ASD` (10 records:
one per row of each attack; windows × 50 units), `.SES` (Re:CoM's 5 entries kept; the clips paired with
Re:CoM's `e002_050` / `e002_051` / `e002_073` / … get their sounds at the same frames; the other attack clips
their template's sounds moved with the hit window: Assumption), `kh1-soldier.json` (`kh1-enemy 1`). On the
session's files: every constant read and checked, 10 records, 4 hurt parts (test
`soldier::tests::builds_from_the_players_files`).

### Owner install steps

1. Run the step as before (`kh1-recom-build`). It prints a `soldier (M11): …` line.
2. Copy into the OpenKH mod folder (same relative paths): `ENEMY/0002/EN0002.ANN`, `ENEMY/0002/EN0002.ASD`,
   `ENEMY/0002/EN0002.SES`, `kh1-soldier.json`, and add to `mod.yml`:

```yaml
- {name: ENEMY/0002/EN0002.ANN, method: copy, source: [{name: ENEMY/0002/EN0002.ANN}]}
- {name: ENEMY/0002/EN0002.ASD, method: copy, source: [{name: ENEMY/0002/EN0002.ASD}]}
- {name: ENEMY/0002/EN0002.SES, method: copy, source: [{name: ENEMY/0002/EN0002.SES}]}
- {name: kh1-soldier.json, method: copy, source: [{name: kh1-soldier.json}]}
```

3. Build in the Mods Manager, start the game. `kh1-recom.log` should show `soldier: KH1's soldier (M11) …` at
   start and `battle N: Soldier … gets KH1's AI` in a Soldier battle.

### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`)

1. A Soldier battle: each Soldier plays KH1's appearance (69), then wanders.
2. Within 10 m it engages: close (< 3.5 m) kicks / double kicks, then runs sideways in an arc; every third
   close round sometimes backs off and spin-kicks.
3. At 3.5–4.5 m: jumping kicks, spin kicks, or a run in and a kick.
4. Far: it runs in, sometimes pauses.
5. Jump above it: it jumps at Sora and kicks in the air.
6. Hit it: KH1's flinch / launch; then its counter (a kick or the spin kick; far: the crouch and the rush).
7. Guard a kick: KH1's clash recoil (218). Guard the jumping / spin / air kick: Re:CoM's flinch, then its counter.
8. Kill one: KH1's reaction motion, Re:CoM's prizes and EXP.
9. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Soldiers.

Please send `kh1-recom.log` (`soldier:` lines) and an F9 trace (its `m6` column carries a `soldier …`
token per Soldier).

### KH1 trace (what to record in KH1)

`kh1-trace-<n>-enemies.csv` (now with `kind` / `spawn`): a room with Soldiers only (Traverse Town 1st
District early), one scenario per file: stand far 10 s; stand at 4 m 10 s; stand at 2 m 10 s; jump on a ledge
above one; one plain swing from the front / behind; a full combo with the finisher; Guard a kick; Guard a
jumping kick; kill one.

### Not built / open (the Soldier)

- The leash `0xE62` (see above) and the scripted-room target rules (KH1 world 11; world / room / variant
  3 / 3 / 2): not ported (battle rooms only).
- The fade-in (no Re:CoM model alpha field found yet, as M10), actor flag 0x80 for 30 frames at the
  appearance (`0x1F94`, not mapped), the appearance's effect (`sys 75`).
- Re:CoM's battle spawns count as KH1's room spawn (`sys 132` ≠ 0): the appearance plays, the first look
  (208) never does (Assumption, as M10's rise). After Re:CoM's alternate appearance the look plays.
- `sys 172` read as "the motion has played to its length" (the look's end): Assumption.
- KH1's `sys 48` searches (the engaged search 5000) give Sora (Re:CoM has no party in battle).
- The jump at a target above runs on `target.y − y < 0` exactly (Re:CoM's floors are flat: equal heights
  when both stand; Assumption, check in a trace).
- The rush leap re-requests 209 one frame after it started from the queue (KH1 may do the same; Assumption).
- Queued-motion blends: the host gives a queued motion the attack blend (8) or 0 (M10's rule); KH1's
  queue blends per call (0 / 8) are not all carried.
- Hits that don't count in KH1 (attacker-less, kinds 8 / 9 / 10 / 11) count here (no Re:CoM equivalent read;
  Assumption).

## The Air Soldier (M11 fliers)

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Air Soldier yet.

### KH1's side. Checked (files: `ex_2160.bd` disassembled; decomp: the syscalls named)

The script (0x5100 bytes; code from `0x5C0`) is a newer shape than the Soldier's (the actor at `wp+0`, the
state object at `wp+0x30`, party / group handles), on the flight of § Flying bodies.

| Routine | Offset | What |
|---|---|---|
| entry | `0x3670` | init `0x38C0` (records LV1 `0x14` / LV2 `0x80`, turn rate 7.5°, walk 8, head-look limits, handlers `0x3A90` / `0x3A28` / `0x3A64`), `sys 135` (flight, no velocity), the appearance thread; each frame: a state request restarts the behaviour (1 idle `0x1442`, 2 engaged `0x1022`, 3 lost `0xD84`, 4 hit `0x5F2`); while idle the search (spawned: 1500) → engaged |
| appearance | `0x1966` → `0x3282` | home = here; a room spawn: request 69 with the 30-frame fade-in and flag 0x80 30 frames, 30 frames, then paused (flag 4, the idle queued with bit 30) until 69 ends; face Sora now; idle |
| idle | `0x1442` | Sora's target is it → engaged; else the wander `0x15FA(200, 800)`: a point [200, 800) from home, its height the floor's under it ± 200 (or the current height ± 150 when the floor is below and it is under 1200 high), flown to (request 1) at 8 for [120, 180) + 8 frames (arrived within 10: vertical only; low under 100: up; a point below: level), then the idle [55, 110) + 8 |
| engaged | `0x1022` | the target 7 m above / below: the climb `0x4CF2` / dive `0x4960`; flat distance < 300 close `0x1364`, < 600 mid `0x1264`, else far `0x123A`; the leash `0x3E52(0)` / `0x2A7C`; engaged again |
| close | `0x1364` | strafe [60, 120); 70 %: hover [90, 150), approach `0x4664(1, 8, 300, 500)`, slash; else spin, hover |
| mid | `0x1264` | 40 %: swoop kick, back off `0x41F8(780, 1200)`, hover; 30 %: hover, swoop, hover; else strafe [90, 150), swoop, hover [60, 120) |
| far | `0x123A` | hover [90, 150) (ends within 480), charge |
| hit | `0x5F2` | flight stopped; close: level (±100) spin, a little above (< 217.5) and over 100 high swoop, then back off (600, 900); far with the anger meter ≥ 100: 50 % (or HP < ¼) the power-up 225 → approach 226 `(12, 750, 600)` → within 900: 227, near (< 300) and level the slash, the meter emptied; strafe [18, 54); engaged |
| hit handler | `0x3A90` | the clash (kinds 6 / 7) or flag 32: `sys 135` with no velocity, state 4; damage: kind 5 +25 (and taken as kind 4), kind 4 +10, kind 3 +5 to `wp+0x54`, state 4; other kinds nothing |
| leash, group | `0x3E52`, `0x2A7C`, `0x4184`, `sys 81` | **not ported** (KH1 removes it far from / out of sight of Sora; messages to a group leader `party(0xB0000 | 160)`: none in Re:CoM) |

Attacks (each a plain request with rows, steered: `Cmd::Attack`). Rows (`sys 23`; `+0x40` the clash recoil;
the limb is the row's bone code's bone):

| Attack | Request | Row (limb, t, radius) | Hit | Recoil | Re:CoM id | Steering while it plays |
|---|---|---|---|---|---|---|
| swoop kick | 200 (64 f, root) | `0x2AC` (l. hand, 4 → 0, 50) | [28, 50) | 206 | 1 | at Sora 40 above at 20, until within 100 |
| slash | 201 **from frame 20** (58 f, root) | `0x38C` (r. hand, 15 → 0, 40) | [22, 52) → [2, 32) | 205 | 1 | vertical only (20 above, at 5), stops within 50 or once it hit |
| swoop | 202 (88 f) | `0x3FC` (r. toes, 50) | [32, 42) | 206 | 0 | rise to 120 above (60 f at 12), coast 32 f, down at 4 to 20 above |
| spin | 203 (80 f) | `0x4DC` (l. toes, 1 → −1, 70) | [24, 32) | 206 | 0 | the tuck 235 to a point 4–5 m off, then 150 f at Sora 100 above at 60 / at him at 30, until within 100; the meter emptied |
| charge | 211 loop (20 f) | `0x46C` (l. collar, 90) | all along | 205 | 1 | after 210's 90 f: at Sora's first position at 50 for [120, 180) f, until within 100 or it hit; then 212 (hit) / 213 (miss: misses + 2) |

Each attack also makes a chest row (`0xEC`, `0x15C`, `0x1CC`, `0x23C`: radius 50, reaction kind 0, mode bits 2):
**not made** (Assumption: KH1's body block during the attack, no reaction on Sora; owner question 2).

Reactions: the flying forms (F `0x48..0x4B`, K `0x4C` / `0x4D`; kind 5 → kind 4). Weight from the records;
hurt parts: 5 (collision block group 2 → Re:CoM bones 15, 14, 2, 3, 8). Checked (files).

Motions (`xa_ex_2160.mset`, 35 entries): KH1's matcher finds no Re:CoM bake or edit on `EN0015.ANN` (75
slots): every request is a new clip at 100 + entry. Re:CoM's clips hold the body 0.61 m above the actor as
KH1's do.

### Re:CoM's side. Checked (exe + decomp)

`CEnemyAirsoldier` (vtable `0x140531CC8`, 139 slots, RTTI at −8), ctor `0x14018A4E0` (`40 53 48 83 EC 20`, its
only caller the factory `0x140188F40` at `0x140189212`; no branch into the moved bytes: exe scan), allocation
`0x1401891F9` `mov ecx, 0xCD0` → 0xCF0. It overrides states 1 (`0x14018A740` / `0x14018A950`, the flying walk),
2 (kick `e015_050`), 3 (dash punch `e015_051`) and the think `0x14018A5C0`. Our copy: states 0, 1, 2, 3, 11, 12,
30, the think; the foot attacks (swoop, spin) in state 2, the hand attacks and the charge in state 3.
`EN0015.ASD`: 2 records (`r_toes` id 0, `r_hand` id 1).

### What the asset step writes

`ENEMY/0015/EN0015.ANN` (Re:CoM's 75 slots byte-identical; KH1's clips at 100 + entry; the slash's clip is
KH1's 201 from frame 20, `enemy::STARTS`), `.ASD` (5 records), `.SES` (Re:CoM's entries kept; the attack clips
get their template's sounds at the hit window: Assumption), `kh1-airsoldier.json` (`kh1-enemy 1`; `run_speed`
the stats block's default 8). Test: `airsoldier::tests::builds_from_the_players_files`.

### Owner install steps

1. Run the step (`kh1-recom-build`): it prints an `airsoldier (M11): …` line.
2. Copy into the OpenKH mod folder and add to `mod.yml`:

```yaml
- {name: ENEMY/0015/EN0015.ANN, method: copy, source: [{name: ENEMY/0015/EN0015.ANN}]}
- {name: ENEMY/0015/EN0015.ASD, method: copy, source: [{name: ENEMY/0015/EN0015.ASD}]}
- {name: ENEMY/0015/EN0015.SES, method: copy, source: [{name: ENEMY/0015/EN0015.SES}]}
- {name: kh1-airsoldier.json, method: copy, source: [{name: kh1-airsoldier.json}]}
```

3. The log shows `airsoldier: KH1's airsoldier (M11) …` at start and `battle N: Air Soldier … gets KH1's AI`.

### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`)

1. An Air Soldier battle: each plays KH1's appearance (69), then flies at KH1's heights (about Sora's head).
2. Idle: it wanders in the air about where it appeared; within 15 m (or when Sora targets it) it engages.
3. Far (≥ 6 m): it hovers, then winds up (210, ~1.5 s) and charges in a loop (211); a hit ends in 212, a miss in 213.
4. 3–6 m: the swoop kick then a back-off, or a rise and the swoop, or a sideways strafe and the swoop.
5. Close (< 3 m): a strafe, then the approach and the slash, or the tuck and the spinning dash.
6. Hit it: KH1's flinch / knock-back while it stays in the air (finishers knock it back, never launch it); after
   enough hits (the anger meter) a far one powers up (225 / 226 / 227) and slashes.
7. Guard each attack: KH1's clash recoil (205 / 206).
8. Kill one: Re:CoM's prizes and EXP.
9. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Air Soldiers.

Please send `kh1-recom.log` (`airsoldier:` lines) and an F9 trace.

### KH1 trace (what to record in KH1)

A room with Air Soldiers (Traverse Town 2nd / 3rd District): stand far 10 s; stand at 4 m 10 s; stand at 2 m
10 s; one swing / a full combo / a finisher; Guard each attack; kill one. The trace's enemy rows give the heights
and the flight speeds to compare.

### Not built / open (the Air Soldier)

- The leash, the group leader's messages, the world-8 room rule, the spawn type 23 entry (not Re:CoM battles).
- KH1's line test (`sys 77`) has no Re:CoM counterpart read: it finds nothing (**Assumption**: Re:CoM's battle
  rooms have no walls between Sora and an enemy), so the branches that need a wall between (the charge after
  two misses, the climb to the target's height) never run.
- The floor for `sys 89` is Sora's height when he last stood (**Assumption**: flat battle floors).
- The head look (`sys 121`), the fade-in, flag 0x80 at the appearance, the effects (`sys 75`).
- `sys 172` from the behaviour's own clock (**Assumption**: ±1 frame against KH1's motion time).
- The chest rows (above).

## The other fliers: not ported yet (survey for the next pass)

Search Ghost, Air Pirate, Gargoyle, Darkball and Wyvern were surveyed, not ported (time). All five enter flight
once with `sys 135` and steer through `+0x20` / `+0x2C` like the Air Soldier (6–14 steering writes each), so § Flying
bodies covers their bodies; each is a new script (1170–1710 listing lines) to port routine by routine. Checked
(files: the scripts listed; exe: ctors, allocation sites, vtables) unless marked:

| Enemy | Script (code from) | Re:CoM class (ctor / alloc / vtable) | Re:CoM `.ASD` | Open before porting |
|---|---|---|---|---|
| Search Ghost | `ex_2230.bd` (`0x5DC`) | `CEnemySearchghost` `0x1401923F0` / `0x1401891CF` **0xCE0** / `0x14054C408` | 2 (`l_fing3b`, ids 0 / 1) | a child object (`sys 94` at `0xD12`, its drain: on Re:CoM's own bullets, checklist step 9); `sys 116 / 117` (part masks); table-1 syscalls `sys1.0` / `sys1.3` (the lister reads only table 0) |
| Air Pirate | `ex_2210.bd` (`0x228`) | `CEnemyAirpirate` `0x14018A0B0` / `0x14018931F` 0xCD0 / `0x1405309E8` | 2 (`r_hand`, ids 0 / 1) | `sys 118` (×7), `sys1.0` (×7) |
| Gargoyle | `ex_2220.bd` (`0x538`) | `CEnemyGargoyle` `0x14018E150` / `0x1401891A5` 0xCD0 / `0x14053F198` | 2 (both wings, id 0 only) | a child object (`sys 94` at `0x1B24` with rows `0x3FC` / `0x46C`, request 219): on Re:CoM's bullets; one Re:CoM power for all its attacks |
| Darkball | `ex_2280.bd` (`0x338`) | `CEnemyDarkball` `0x14018D5D0` / `0x140189349` 0xCD0 / `0x14053B7A8` | 2 (`ex_2280_sk`, a whole-clip window and a 1.3 m one) | `sys 86`, `sys 74`, `sys1.0` / `sys1.3` |
| Wyvern | `ex_2320.bd` (`0x378`) | `CEnemyWyvern` `0x140195ED0` / `0x140189373` **0xCF0** / `0x140555DC8` | 3 (header to `0x10C`) | turn rate changed 19 times (`Body::turn` covers it), `sys 42`, an idle requested from frame 16 (`enemy::STARTS` covers it) |

Every ctor starts `40 53 48 83 EC 20 45 33` (Checked (exe)). The Air Soldier's shared routines (`0x2F48`, `0x2FBE`,
`0x2FA0`, `0x2FF4`, `0x307A`, `0x30B6`, `0x34C2`, `0x35E6`, …) recur byte for byte in all five (`.bd` scan).

## Per-enemy porting checklist

For enemy X (Re:CoM id N, KH1 model `xa_..._MMMM`), one agent per enemy:

1. **Files.** KH1 `xa_..._MMMM.mdls` / `.mset`; Re:CoM `ENEMY/NNNN/ENNNNN.{ANN,MDL,ASD,SES,EPD}` (table below:
   gaps). A missing `.SES` needs the step to make one (not built yet).
2. **KH1 script.** Part 5 of the `.mdls`. Disassemble with `tools/kh1-ai-script.py` (linear listing with
   folded expressions; the VM's ops are in its header). Find the supervisor, the init (`sys 45` record,
   stats block writes), the state routines, the hit handler (`sys 33`: values × 2 are code offsets), the
   attacks (`sys 23` rows), the requests (bit 24 action, bit 23 held, bit 30 queue). Mark which routines are
   the shared ones (`kh1_core::enemy::script`; byte-identical to the Soldier's: `.bd` search).
3. **Numbers.** Every constant the port reads gets a name and its push offset(s) (`Spec::consts`), every
   request word its offset (`Spec::requests`). Nothing of the game in the code.
4. **Behaviour.** `kh1-core/src/<enemy>.rs`: the script on `enemy::script` (one `async fn` per routine, the
   same order of yields), an `Attack` enum (its rows), `NAMES`, `impl Behaviour`; tests on a flat floor.
5. **Motions.** Run the asset step: the matcher's bakes play Re:CoM's clips; check the reaction requests the
   model has (`F` 0x48, `K` 0x4C, `a` 0x4E, `A` 0x50, `L` 0x52, `l` 0x5C in the table); root-motion and
   bit-23 attacks.
6. **Re:CoM class.** Vtable, slot count, ctor bytes (`40 53 48 83 EC 20` for all 30: same hook shape), the
   factory allocation site and size, the states the class overrides (its attack states), its think; check the
   ctor's only caller and no branch into the moved bytes (exe scan); `<CLASS>_HOOKS` / `_ALLOC` / `_PTRS` in
   `sites.rs` (in the hook-shape test list), pefile check.
7. **Attack power.** Each KH1 row → the Re:CoM attack id of the nearest Re:CoM attack (`.ASD` `+0x64`, `@ENP`);
   write down the rule (owner question when not obvious). Classes with no Re:CoM records (table) need a
   template.
8. **Table entries.** `kh1_enemy::CLASSES` (states per attack, names, appearance, `reads_hp`, manifest),
   `kh1_enemy::{make, parse_for}`, `enemy::SPECS`, the asset spec module.
9. **Projectiles** (casters): KH1 spawns a child object (`sys 94` → `0x14028F0A0` / `0x14028F5E0`, from the
   model's sub-object table `+0x13C`) that runs its own script: port it on Re:CoM's own bullets (`CBullet` /
   `CHomingBullet`, its shot table and collision), never our own projectile runtime. Research per caster.
10. **Checks.** Tests (`cargo test --workspace` with `KH1_M11_DIR`, `KH1_TEST_DATA`), the Windows build,
    pefile over every site, the asset outputs against Re:CoM's files (Re:CoM's bytes kept), a doc section
    with Checked / Assumption tags, the owner's install lines, the test list, the KH1 trace scenarios.

## The other 29, by family

States: those the Re:CoM class overrides (besides `CBattleEnemy`'s). Hits: KH1 requests with a HIT track.
Reactions: `F` flinch, `K` knock-back, `a` air flinch, `A` air knock-back, `L` launch, `l` air launch.
Missing: Re:CoM files not in the session's extraction (`.BIN` only exists for the Shadow and Spade Soldier
here; the step doesn't use it). Every ctor starts `40 53 48 83 EC 20 45 33` (Checked (exe)). Checked (exe +
files); the family is from the scripts' syscalls and the models' motions (Assumption where noted).

| Enemy | KH1 model (script) | Re:CoM class, id | vtable | ctor / alloc site, size | States | Hits | Reactions | Missing |
|---|---|---|---|---|---|---|---|---|
| Soldier (built) | `xa_ex_2010` (ex_2010.bd) | `CEnemySoldier` 2 | `0x14054ea68` | `0x140193510` / `0x140188fd7` 0xcd0 | 1,2,3 | 200,201,202,203,204,207,209 | FaLl | — |
| Large Body | `xa_ex_2050` (ex_2050.bd) | `CEnemyLargebody` 9 | `0x140542ad0` | `0x14018f090` / `0x1401890fd` 0xce0 | 1,2,3 | 200,207,231 | FKL | — |
| Red Nocturne | `xa_ex_2110` (ex_2110.bd) | `CEnemyRednocturne` 3 | `0x140548a58` | `0x140191880` / `0x140189001` 0xcd0 | 1,2,3 | — | FKaA | SES |
| Blue Rhapsody | `xa_ex_2120` (ex_2120.bd) | `CEnemyBluerhapsody` 4 | `0x140537ca8` | `0x14018c120` / `0x14018902b` 0xcd0 | 1,2,3 | 215 | FKaA | — |
| Yellow Opera | `xa_ex_2130` (ex_2130.bd) | `CEnemyYellowopera` 5 | `0x1405570c8` | `0x1401968b0` / `0x140189055` 0xcd0 | 1,2,3 | 209 | FKaA | SES |
| Green Requiem | `xa_ex_2140` (ex_2140.bd) | `CEnemyGreenrequiem` 6 | `0x1405404c8` | `0x14018e570` / `0x14018907f` 0xce0 | 1,2 | — | FKaA | SES |
| Powerwild | `xa_ex_2030` (ex_2030.bd) | `CEnemyPowerwild` 7 | `0x140547798` | `0x140191380` / `0x1401890a9` 0xcd0 | 1,2,3 | 212,214,222 | FaLl | — |
| Bouncywild | `xa_ex_2040` (ex_2040.bd) | `CEnemyBouncywild` 8 | `0x140538fb8` | `0x14018c4d0` / `0x1401890d3` 0xcd0 | 1,2,3 | — | FaLl | — |
| Air Soldier (built) | `xa_ex_2160` (ex_2160.bd) | `CEnemyAirsoldier` 15 | `0x140531cc8` | `0x14018a4e0` / `0x1401891f9` 0xcd0 | 1,2,3, think `0x14018a5c0` | 200,201,202,203,210,211,212 | FKaA | — |
| Bandit | `xa_ex_2090` (ex_2090.bd) | `CEnemyBandit` 11 | `0x140534368` | `0x14018aeb0` / `0x140189151` 0xcd0 | 1,2,3 | 200,202,210,211,212,216,217,218,230..233 | FaALl | — |
| Fat Bandit | `xa_ex_2060` (ex_2060.bd) | `CEnemyFatbandit` 10 | `0x14053ddf8` | `0x14018dd70` / `0x140189127` 0xcd0 | 1,2,3 | 200,205,206,207 | FKaALl | — |
| Barrel Spider | `xa_ex_2170` (ex_2170.bd) | `CEnemyBarrelspider` 28 | `0x140535688` | `0x14018b3b0` / `0x14018941b` 0xce0 | 1,2,3,19 | 205,207,211,213,214 | FKaA | — |
| Search Ghost | `xa_ex_2230` (ex_2230.bd) | `CEnemySearchghost` 14 | `0x14054c408` | `0x1401923f0` / `0x1401891cf` 0xce0 | 1,2,3 | 203,204 | FKaA | — |
| Pirate | `xa_ex_2100` (ex_2100.bd) | `CEnemyPirate` 21 | `0x140546458` | `0x140190f30` / `0x1401892f5` 0xcd0 | 1,2,3 | 208,210,214,216,217,218,235,236 | FaLl | — |
| Air Pirate | `xa_ex_2210` (ex_2210.bd) | `CEnemyAirpirate` 22 | `0x1405309e8` | `0x14018a0b0` / `0x14018931f` 0xcd0 | 1,2,3, think `0x14018a190` | 200,212 | FKaA | — |
| Wight Knight | `xa_ex_2200` (ex_2200.bd) | `CEnemyWightnight` 12 | `0x140553748` | `0x140194ed0` / `0x14018917b` 0xcd0 | 1,2,3 | 200,201,202,211,216 | FaALl | — |
| Gargoyle | `xa_ex_2220` (ex_2220.bd) | `CEnemyGargoyle` 13 | `0x14053f198` | `0x14018e150` / `0x1401891a5` 0xcd0 | 1,2,3 | 203,212,218 | FKA | — |
| Darkball | `xa_ex_2280` (ex_2280.bd) | `CEnemyDarkball` 23 | `0x14053b7a8` | `0x14018d5d0` / `0x140189349` 0xcd0 | 1,2,3 | 200,202,204 | FKaA | — |
| Defender | `xa_ex_2340` (ex_2340.bd) | `CEnemyDefender` 26 | `0x14053ca98` | `0x14018d8e0` / `0x1401893c7` 0xcd0 | 1,2,3 | 207,208,209 | FKa | — |
| Wizard | `xa_ex_2150` (ex_2150.bd) | `CEnemyWizard` 25 | `0x140554a58` | `0x140195420` / `0x14018939d` 0xcd0 | 1,2,3,4 | — | FKaA | SES |
| Sea Neon | `xa_ex_2070` (ex_2070.bd) | `CEnemySeaneon` 18 | `0x14054b0e8` | `0x1401920c0` / `0x140189277` 0xce0 | 1,2,3 | 201,204 | FKaA | — |
| Aquatank | `xa_ex_2080` (ex_2080.bd) | `CEnemyAquatank` 20 | `0x140532fe8` | `0x14018a960` / `0x1401892cb` 0xcd0 | 1,2,3 | 202,203 | FKaA | — |
| Screwdiver | `xa_ex_2240` (ex_2240.bd) | `CEnemyScrewdiver` 19 | `0x140549e18` | `0x140191d90` / `0x1401892a1` 0xcd0 | 1,2,3 | 203,205 | FK | — |
| Wyvern | `xa_ex_2320` (ex_2320.bd) | `CEnemyWyvern` 24 | `0x140555dc8` | `0x140195ed0` / `0x140189373` 0xcf0 | 1,2,3 | 200,202,203,205,206 | FKaA | — |
| White Mushroom | `xa_ex_2350` (ex_2350.bd) | `CEnemyWhitemushroom` 16 | `0x1405523f8` | `0x1401944b0` / `0x140189223` 0xce0 | 1,11,12,30 | — | FKaA | — |
| Black Fungus | `xa_ex_2380` (ex_2380.bd) | `CEnemyBlackfungus` 17 | `0x1405369c8` | `0x14018bac0` / `0x14018924d` 0xce0 | 1,2,3 | 200 | FaLl | SES |
| Creeper Plant | `xa_ex_2290` (ex_2290.bd) | `CEnemyCreapplant` 31 | `0x14053a348` | `0x14018ca40` / `0x140189499` 0xcd0 | 1,2,3,19 | 202,203,204,206,207 | FKaA | — |
| Neoshadow | `xa_ex_2420` (ex_2420.bd) | `CEnemyNeoshadow` 27 | `0x140545128` | `0x140190150` / `0x1401893f1` 0xce0 | 0,1,2,3 | 202,205,207 | FKaA | — |
| Spade Soldier | `xa_aw_1030` (aw_1030.bd) | `CEnemySpadesoldier` 103 | `0x14054fd90` | `0x140193a20` / `0x140189afc` 0xcd0 | 0,1,2,3,30 | 209 | FKaA | — |
| Heart Soldier | `xa_aw_1060` (aw_1060.bd) | `CEnemyHeartsoldier` 104 | `0x1405417d0` | `0x14018ec70` / `0x140189b23` 0xcd0 | 0,1,2,30 | 206,207,208,210 | FKaA | — |

Re:CoM `.ASD` records (template for the power rule): none for Red Nocturne, Green Requiem, Bouncywild, Wizard,
White Mushroom, Creeper Plant (their attacks are Re:CoM shots); one for Blue Rhapsody, Yellow Opera, Barrel
Spider, Defender, Aquatank, Heart Soldier; 3 Powerwild, Sea Neon, Wyvern, Neoshadow; 6 Wight Knight; 2 the
others. Re:CoM `.ANN` slots: 73–95 (all under the step's slot 100). Checked (files).

### Families

1. **Ground melee** (the Soldier's shape: walk / run, hit tracks, kicks and lunges; the framework as built):
   Large Body (no jump motion; KH1's hits from the front bounce off it: research its guard, Assumption), Powerwild, Bandit,
   Pirate, Wight Knight, Neoshadow (also sinks and rises like the Shadow; Re:CoM overrides its state 0 →
   probably `Appear::Shadow`-like), Spade / Heart Soldier (Re:CoM overrides their states 0 and 30: check their
   own appearance and defeat before choosing `Appear::Base`).
2. **Flying / hovering** (no walk on the floor; glide mode 2 of `sys 40`, height holding; `sys 125` sets the
   turn rate): Air Soldier (own Re:CoM think), Air Pirate (own Re:CoM think), Search Ghost, Gargoyle (no walk /
   run / jump motions), Darkball, Wyvern. Needs: the body's height kept by the script (KH1's air movement
   state 2 without gravity: Assumption, research in `0x1402AF6A0` / the movement states), Re:CoM's mover
   without gravity for them.
3. **Casters with projectiles** (KH1 child objects `sys 94`, effects `sys 225`; little or no HIT track):
   Red Nocturne, Blue Rhapsody, Yellow Opera, Green Requiem (the four also fly), Wizard (Re:CoM state 4 too),
   Bouncywild (bananas), Fat Bandit (fire breath + belly), Defender (fireballs + shield), Search Ghost (its
   drain). Needs: the child object's script → Re:CoM's own bullets (checklist step 9), Green Requiem's
   support spell (Re:CoM overrides no state 3 for it; KH1's: a barrier / heal on its allies, Assumption).
   Which of these shoot what is from KH1's play (Assumption) and the scripts' syscalls (Checked (files)).
4. **Water** (Atlantica, underwater movement): Sea Neon (no walk / run motions), Aquatank, Screwdiver (only `F`
   / `K` reactions; four lock points). Needs: Re:CoM's Atlantica swimming movement for enemies (how Re:CoM's
   own water enemies move: research), KH1's water movement mode.
5. **Special cases**: Barrel Spider (explodes; Re:CoM overrides state 19: its explosion state), Creeper Plant
   (rooted, Re:CoM state 19 too), White Mushroom (no attacks; its spell game; Re:CoM overrides states 11 / 12
   / 30: its reactions are its game, keep Re:CoM's for those?), Black Fungus (petrify / its guard; no `.SES`).

## Owner answers (2026-11-02, `docs/design-philosophy-instructions.md` § Soldier choices)

1. Shadow's wander: fixed to KH1's rule (above).
2. Soldier's power mapping: kept.
3. Guarded attacks without a KH1 recoil motion: Re:CoM's flinch, for every enemy (above).
4. Battle spawns: KH1's appearance kept.
5. The leash: stays out.

The questions as asked (0.68.0):

1. **The Shadow's wander** (finding above): KH1 picks its two wander points about where the Shadow stands
   now; M10's port picks them about its home. (a) Fix the Shadow to KH1's rule (a version on its own), (b)
   leave it.
2. **The Soldier's attack power** (decision M10, "Re:CoM's attack power per attack"): Re:CoM's Soldier has
   only its kick (id 0) and spin kick (id 1). Chosen: the kick, double kick and rush use the kick's power; the
   jumping kick and the rush leap (same row) the spin kick's; the spin kick's two rows the spin kick's; the air
   kick the kick's (by KH1's row `+0x3C`, 20 / 100). (a) Keep, (b) all KH1 kicks on Re:CoM's kick, (c) another
   mapping.
3. **Guarding the Soldier's jumping / spin / air kicks**: KH1's rows name clash recoils its model doesn't have
   (110, 210). Built: the attack stops and it idles. (a) Keep, (b) Re:CoM's flinch (state 11) as decision
   27's stand-in, (c) the attack carries on (no recoil).
4. **The KH1 appearance in Re:CoM's battles** (as M10's Shadow rise): every battle Soldier plays KH1's
   appearance (request 69), so it never plays the first look (208). (a) Keep, (b) treat Re:CoM's battle spawns
   as KH1's placed enemies (no appearance, the look on first engaging).
5. **The leash**: KH1 removes a Soldier that is far from Sora; not ported (battles could not end). Confirm.
6. **The Air Soldier's attack power** (fliers): Re:CoM's Air Soldier has a kick (id 0, `r_toes`) and a dash punch
   (id 1, `r_hand`). Chosen by the limb of KH1's row: the swoop kick and the slash (hands) and the charge (a body
   dash) the punch's power, the swoop and the spin (feet) the kick's. (a) Keep, (b) all on one, (c) another rule.
7. **The Air Soldier's chest rows**: each KH1 attack also makes a 50-unit row on its chest with reaction kind 0
   (no reaction on Sora; it looks like a body block). Not made. (a) Keep it out, (b) make it a Re:CoM record
   (it would hit with Re:CoM's reaction), (c) research its KH1 effect first.
