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
| `Behaviour::set_target_grounded` (default: nothing; M11, the Air Pirate) | Sora's KH1 movement state `*(target + 0x70)` (Checked (decomp): `0x1402B1960` keeps it there; 0 ground, 2 air) as his Re:CoM mover's floor contact `+0x120 & 5`, each frame before the AI frame (**Assumption**: his glide counts as the air) |
| `Behaviour::set_floor` / `on_landed` (defaults: nothing) | the floor height each frame; Re:CoM's `CCollAttack` apply `0x14016F1C0` answered 1 (a hit) on the player side (from the pair-dispatch wrapper, `win::guard::on_result`) → KH1's attack hit callback (`+0x38`) |
| host | a flying body's vertical velocity is written every frame (× the mover's speed scale, Re:CoM's acceleration cancelled), it is never `grounded`; the floor is Sora's height the last time he stood on the floor (**Assumption**: Re:CoM's battle floors are flat; Re:CoM's fliers keep no floor height: their gravity is off, `+0x260 & 0x100`, `0x1401D67B0(mover, 0)`) |

New for the Darkball (Checked (decomp) unless marked):

| Part | What |
|---|---|
| `Cmd::Scale(k)` | KH1's actor scale `+0x40..+0x48` (the Darkball's appearance and warps ramp it 0 → 1, 0.2 → 2 → 1): a factor on Re:CoM's own model scale (read before the first write), through Re:CoM's scale getter / setter `0x1401A4A20` / `0x1401A7050` (the Shadow's `Height` uses them for y); written as at least 0.001 (**Assumption**: a zero scale would leave the model matrix singular) |
| `Cmd::Visible(b)` | KH1's actor flag `0x1` (the model drawn) as Re:CoM's own hide bit `enemy+0x154` bit 0: the actor draw `0x1401A39B0` (every actor class's slot) draws nothing while it is set; the player's hide state sets it (`0x1401E5E80`), `CEnemyShadow`'s state 0 clears it (`0x140192850`), `CEnemyNeoshadow`'s ctor sets it (`0x140190150`), Re:CoM's defeat sets it once its death effect is half done (`0x1401832F0`). **Assumption**: the bit doesn't stop the motion clock (`0x1401A47C0`) |
| `Body::accel` | a script's write of the stats block's `+0x14` (the flight's easing `a` of `0x1402CC000`; the Darkball writes 0.05 / 0.1 / 0.2 / 1), else the manifest's `air_accel` |
| `script::warp`, `Body::warp` | `sys 86` (`0x1402BA560` → `0x140292BC0`): the actor moved to a point with a facing (the vector's fourth word, wrapped by `0x1402A8280`), its flight velocity `+0x3C0..+0x3D0` and speed `+0x2C` zeroed; the host moves the body there in one frame through Re:CoM's mover (its velocity) |
| `Behaviour::keeps_reaction` | KH1's script hit handler may zero the hit's reaction kind (`hp[0xC] &= ~0x1F`): no reaction; a reaction already running carries on in Re:CoM's state 11 / 12, otherwise back to state 1 (Re:CoM's handler has already cut the attack it hit: **Assumption**, the action ends where KH1's would go on) |
| quiet watchdog | a flier's AI that steers its body this frame (`+0x20` × `+0x2C`) counts as acting (the Darkball's invisible glide issues no command for its whole length) |
| asset step `enemy::OWN_CLIPS` | requests that get their own new clip although Re:CoM has their motion as an exact bake, because the bake carries Re:CoM's own attack records (bound by the clip's name, `0x14014DBE0`): they would hit with KH1's rows. The new clip gets the bake's SES entry (same frames) |
| asset step `enemy::BONE_POINTS` | a bone code's place in a motion's frame (KH1's pose through `kh1-anim`, actor frame, root motion off) as `<name>_x / _y / _z` constants |

Re:CoM's fliers (Checked (decomp)): every flier ctor sets `+0x154 |= 0x80` and `+0x260 |= 0x100` (gravity off in
Re:CoM's state code, `0x14017EB80`, `0x14017EED0`, `0x140198600`); their clips hold the body 0.61 m above the
actor (Air Soldier: `ex_2160_sk` at y 0.614 in `e015_001` and in KH1's idle, Checked (files)), so Re:CoM's
fliers float low; at Combat = KH1 they fly at KH1's heights (the script's own targets).

### Enemy guard (the shared enemy-guard step; owner 2026-11-02: Bandit question 2, option (b))

KH1 enemies that block (the Bandit's stance and spin now; the Large Body's front and the Defender's shield next) do
it with **guard-type attack rows**, not with a flag on their bodies. This step gives Re:CoM's engine KH1's rule
through Re:CoM's own body guard, the one Sora's Guard already uses.

**KH1's rule. Checked (decomp + exe), Steam addresses:**

- An attack row's clash type is its object bits 17..19 (row `+0x38` low bits). Types 1..4 are guard types: 1, 2
  and 4 never meet a hurt body; type 3 still does (`0x1402B3BE0`). Two attacks are tested against each other only
  when the type table `0x140528CB8` = `1F 09 09 3F 29 18 00 00` has `table[b] & (1 << a)` (symmetric). A type-2 row
  meets only types 0 and 3: Sora's swings (type 0) and nothing of magic (Fire / Blizzard / Thunder rows are types
  5 / 6).
- Front only: KH1 tests the cos of the angle between the enemy's facing (turned by row `+0x60`) and the attacker
  against row `+0x64` (`0x1402B4640`). Both of the Bandit's rows have turn 0, cos 0: the front half (±90°).
- A mode-2 row lives while its request plays and that motion's HIT slot is on (`0x1402B47A0`).
- When two attacks meet, `0x1402B4BC0` runs both ways (`0x1402B4A20`). Nothing when the other side's priority is 0
  or both are 98 or more; a side at 100 is never moved. Otherwise the lower side gets a clash record: kind 6, or
  kind 7 when it is 40 or more under (`diff < −0x28`), with `diff` = its priority − the other's. A type 1 / 3 / 4 side
  with the higher priority **repels** instead (`0x1402BF630`, flag 0x20): the Large Body and the Defender (not built:
  see below). A type-2 side never repels. A projectile side (`+0x30 & 0x400`) is turned to the other side instead
  of getting a record (not built: see below).
- Sora (`0x14029BBD0`): on the ground kind 6 plays the recoil 0x6E (`0x14029D720`; pushed back by −diff × 2.5,
  btltbl G+0x1C, when diff < 0), kind 7 the strong recoil 0x6F (`0x14029D5F0`, 64 frames, 118 cm root travel back);
  in the air either plays 0x70 (`0x14029D510`: hop 20, back velocity diff / 100 × 40, at most 8, G+0x30 / G+0x34).
  The swing ends, the combo counter `CB+0x41` drops by one, flag 0x80 for 8 frames. The Counterattack is offered
  from frame 12 of 0x6E (`0x140294220`), as after a Guard.
- The enemy: its script's hit handler hears the clash (kinds 6 / 7) and its row's recoil (`+0x40`) plays; a
  clashed row meets nothing more (`+4 & 0x30`).

**Re:CoM's side. Checked (decomp + exe):**

- Re:CoM's hit apply `0x14016F1C0` guards a body (result 2) when the body's `CCollBody +0x9C` has bit value 1, the
  attack's element is in `+0x8C`, and the attacker is within `+0xB0` (half angle) of the body's facing. It then
  sets `+0x9C |= 0x20`, and the enemy hit handler plays Re:CoM's guard spark, the "Guard" popup (kind 3) and calls
  vtable `+0x3D8` (`CBattleEnemy`'s is `ret 0`, `0x140181600`). Result 1 / 2 also mark `procAttackCollision +0x644`.
  The flip bit (value 2, result 3) is not used: it makes Re:CoM run its own Sora flip action 0xD instead of KH1's
  recoil.

**The step (built):**

| Part | Where | What |
|---|---|---|
| KH1's rule | `kh1-core` `enemy/guard.rs` | `MEETS` / `meets`, `resolve` (both sides' `Side`), `GuardRow` (the asset step's row: type, priority, recoil, turn / cos, windows per request), `live`, `half_angle`, `clash` |
| Command | `kh1-core` `enemy/script.rs` | `Cmd::Guard(Some((row, request)))` turns a guard row on with its request, `Cmd::Guard(None)` off |
| Asset step | `kh1-recom-assets` `enemy.rs` | `GUARDS` (per enemy: name, row offset, requests); `guard_rows` reads each row's type, priority, recoil and front cone from the player's script and its windows from the motions; the manifest's `"guards"` |
| Manifest | `kh1-recom` `kh1_enemy.rs` | `Manifest::guards` (`parse_guards`: optional; types, priorities, recoil, windows checked) |
| Runtime | `kh1-recom` `win/kh1_enemy.rs` | per enemy `GuardOn` (row, request, clock); each tick `guard_step` turns Re:CoM's guard on every hurt part (`+0x9C` bit 1, `+0x8C` physical, `+0xB0` = the row's half angle, Re:CoM's own values saved and put back) while the row's window is on, ends it when its request changes; `guard_watch` drops it when a Re:CoM state runs (Stop, stun); our vtable copy's `+0x3D8` = `guard_event` |
| Sora's swings | `kh1-recom` `win/attack.rs` | in the pair-dispatch wrapper, around each KH1 swing's apply: `guard_for_swing` clears the guard bit when KH1's types don't meet, else adds the swing's element; `guard_after_swing` puts the fields back and, on result 2, resolves both sides (`resolve`) |
| Sora's clash | `kh1-recom` `movement.rs` | `Host::take_swing_clash` → `Kh1Sora::swing_clash`: KH1's recoils above on Re:CoM's action rows (0x6E Re:CoM's `p001_081`; 0x6F / 0x70 appended rows, `rows::KH1_ONLY`), KH1's push, hop, flag 0x80, combo counter, `Event::SwingClashed` |
| Enemy's clash | `win/kh1_enemy.rs` `guard_event` | the row's recoil and kind kept as `clash_recoil`; the row ends; the attack ends into our state 11 (or state 11 is next), which plays KH1's clash reaction (`react::clash`) and tells the script (its hit handler) |

Per frame: nothing while no KH1 enemy is guarding (`GUARDED` = 0); one window test per guarding enemy; the swing
wrapper's check only when a KH1 swing meets a body.

**Differences from KH1 (known, kept):**

- Shape: KH1 tests the guard row's own sphere (the Bandit's blade, 40); Re:CoM tests the enemy's hurt bodies. A
  swing that reaches the body's front while the row is on is blocked even when it would have missed the blade
  (the owner chose Re:CoM's body flag). Owner question below.
- Angle: equal for cones without a turn (`turn` = 0; KH1's cos ≥ `c` = Re:CoM's ±acos(`c`)). A turned cone has no
  Re:CoM field: refused with a log line (`guard row … not built`). None of the Bandit's.
- Re:CoM's "Guard" popup shows over the enemy (its art: Assumption); KH1 shows none.
- Elements: the guard keeps out the physical element only; KH1's type rule decides for Sora's swings. Our spells
  carry Re:CoM's spell attributes (KH1's types 5 / 6 meet no type-2 row, so they land as in KH1). Assumption:
  Re:CoM's other physical attacks (a turned-back shot, a summon) count as KH1's type 0.

**For the Large Body and the Defender (not built):** their rows are types 1 / 4 with priorities above Sora's: KH1
repels (`Side::Repels`: Sora gets the clash, the enemy goes on, no recoil on its side). `guard_event` then has to
skip the enemy's recoil when its side is `Repels`; `resolve` already returns it. A turned cone (a shield held to one
side) needs Re:CoM's `+0xB0` plus a facing offset Re:CoM does not have: research before porting.

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

## The Darkball (M11 fliers)

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Darkball yet.

### KH1's side. Checked (files: `ex_2280.bd` disassembled; decomp: the syscalls named)

The script (0x3600 bytes; code from `0x338`, entry `0x1A28`) has the Air Soldier's newer shape (the actor at
`wp+0`, the state object at `wp+0x20`), on the flight of § Flying bodies, with its own routines (no shared
helper besides the random draws).

| Routine | Offset | What |
|---|---|---|
| entry | `0x1A28` | init `0x1BD0` (records LV1 `0x14`, LV2 `0x80` / `0xEC` (world 16 from room 37); stats `+0x10` = 800 (the hop, unused); `sys 208`; handlers hit `0x1E20`); `sys 135` (flight, no velocity); a yield; turn rate 5° (`sys 125`); placed (`sys 132` = 0): a target within 1000 → engaged, else lost; spawned: the appearance `0x96E` → `0x97E`. Each frame: nothing while warping (`wp+0x4C`); a state request restarts the behaviour (1 idle `0x8C2` → `0x8DA`, 2 engaged `0x65E` → `0x682`, 3 lost `0x520` → `0x538`, 4 hit `0x338` → `0x35C`); while idle, a target within 1000 → engaged |
| appearance | `0x9A0` | flags 0x20 / 0x40 / 0x10 (unhittable), request 69, the idle queued; 80 up (`sys 86`); 0 → 1 scale over len(69) − 50 = 122 frames facing Sora, sinking 0.3 a frame from frame 30; flags cleared; 50 frames; then idle |
| idle | `0x8DA` | not noticed; forever: a point 5–10 m away (raised 0.5–1 m when under 0.7 m), turn to it (`0x34A0`: until the facing's cosine to it is 0.9, 200 frames at most), fly to it (`0x331C`: at 3, until within 10, 500 frames; rising when under 70, sinking when 1.5 m up and above Sora; stuck: a new point and the end), 30 frames |
| engaged | `0x682` | the notice once (`0xBCE`: request 205, 170 frames facing Sora); rounds: after 2 counted rounds lost; > 6 m: 50 % the warp (one round), else the side approach and dashes (one round); ≤ 6 m: the bite, the warp, the side approach and dashes (two rounds) |
| side approach | `0x3060` | to within 4.5 m (engaged) / 3 m (lost) for 300 frames: along the facing turned ±80° (chosen once) at 4 within 10 m, straight at 8 beyond; bobbing 0.5–1.5 m above the target (a new bob every 20–40 frames); stuck: turn, 20 frames, end |
| dash | `0xCC4` | turn to the target; 201 (8 + 68 frames facing it); 202 with row `0x158` (`sys 213`: mode 2, it lives while 202 plays) along the facing at 24 for 60 frames (sinking 0.1 when over 70 above the target, rising 0.4 when low); 203, the idle queued |
| bite | `0xC64` | turn to the target; the action 204 (`0x10000CC`) with row `0x238`, effect 10; the idle queued (bit 30) |
| warp | `0x2304` | the vanish `0x2C7A` (warping; 200; unhittable; 1 → 0 scale over 70 frames; back where it started (`sys 86` with the position's fourth word 1.0 as the facing); model hidden: flags 0x1 / 0x10 cleared; effect 8 attached); 30 frames; glides (`0x25C0`, at 24, easing 0.1, 600 frames at most) alternating a point 5–6 m from itself (a target farther than 10 m: one near it) and one 0.1–0.5 m from the target 0.7 m above it, a 60-frame pause between, until within 3 m of the target (the drop `0x27F8`) or 4 tries (the plain reappearance `0x2A78`); the easing 0.2 from then on |
| glide | `0x25C0` | within 3 m of the target after 60 frames, or within 3 m of the point: only vertical, done once level (easing 0.05 / 0.1); stuck after 30 frames: done |
| drop | `0x27F8` | the idle 10 frames; request 200 (row `0x1C8` made 8 frames later); shown; 0.2 → 2 scale over 10 frames, held 10, 2 → 1 over 64; hittable; not warping |
| plain reappearance | `0x2A78` | the idle 20 frames; turn to the target; 200; shown; to bone code 0's place (`sys 19(actor, 0, 0)` → bone 3, the core, about 96 above the actor, `sys 86`); 0 → 1 scale over 70 frames, sinking 0.5 from frame 10; hittable; not warping |
| hit | `0x35C` | the bite mark (100): the bite, engaged; the stagger mark (200, a Gravity hit): 205 for 170 frames; under 5 counted hits: the bite (< 3.5 m) or a dash (< 10 m), engaged; else the warp, lost |
| hit handler | `0x1E20` | noticed; kinds 4 / 5: a hit counted, the light run reset; kinds 6 / 7 (clash): nothing counted; others: both counted, the 5th light hit in a row marks the bite, the 6th and later: no reaction (the hit's reaction kind zeroed) and no state change; else state 4 |
| lost | `0x538` | the side approach to Sora (3 m, 300 frames); within 3 m: 8 m back along the line from him (0–1 m higher) at 6; Sora within 10 m → engaged |
| leash | `0x2112` | **not ported**: beyond 15 m (10.5 m out of sight, spawned) it sinks away (request 70) and is removed; in a Re:CoM battle room it would never be defeated |

Attacks (rows `0x70` bytes, `sys 23`; Checked (files)):

| Attack | Request | Row (bone code → bone, t, radius, KH1 power) | Hit | Recoil `+0x40` | Re:CoM id | Re:CoM state |
|---|---|---|---|---|---|---|
| bite | 204 (152 f, root, an action) | `0x238` (8 → 6, 0, 160, 100 %) | [22, 74) | 110 (no motion) | 1 | 3 |
| dash | 202 (60 f, looped; steered) | `0x158` (8 → 6, 0 → −1, 100, 40 %) | [0, 60) | 206 | 0 | 2 |
| drop | 200 (100 f; steered) | `0x1C8` (0 → 3, 0, 230, 100 %) | [12, 28) | 110 (no motion) | 1 | 3 |

KH1's attack objects default to mode 2 (`0x1402B2DC0`: `| 9`, bits 2..3 = 2; `sys 213` only sets it again):
they live while the request they first saw plays and hit while its HIT slot is on; the radius takes the actor's
scale only with flag `0x2000` (`0x1402B2DC0`), which the Darkball never sets. So the drop's row, made 8 frames
after its request, is the same as one made with it (its window starts at 12): the port makes it with the
request (`Cmd::Attack`).

Reactions: the flying forms (F `0x48..0x4B`, K `0x4C` / `0x4D`, a `0x4E` / `0x4F`, A `0x50` / `0x51`; no
launches: kind 5 → kind 4); weight 1.0 in all three records; hurt parts: 4 (collision block group 2 → Re:CoM
bones 2, 14, 10, 5). Checked (files).

Motions (`xa_ex_2280.mset`, 17 entries): KH1's matcher finds Re:CoM's `EN0023.ANN` (86 slots) bakes for the idle
(`e023_001`), a flinch (`e023_021`), 66 (`e023_070`), 70 (`e023_074`), the dash's wind-up and end (201 →
`e023_050`, 203 → `e023_052`), the dash 202 (`e023_051`) and the bite 204 (`e023_053`); edits (≤ 20 bones differ) for entry 1
(7, 72, 73, 78, 103 → `e023_020`), entries 3 / 4 (76, 77, 80, 81 → `e023_072`), 6 (67, 102, 104 → `e023_071`), 7
(68 → `e023_072`), 8 (the appearance 69 → `e023_073`), 15 (205 → `e023_075`); no Re:CoM match: 200, 206. The
dash and the bite get their own clips (112, 114: `enemy::OWN_CLIPS`), the rest of the bakes play Re:CoM's clips;
the edits are new clips with their Re:CoM clip's sounds (decision M10).

### Re:CoM's side. Checked (exe + decomp)

`CEnemyDarkball` (vtable `0x14053B7A8`, 139 slots, RTTI at −8), ctor `0x14018D5D0` (`40 53 48 83 EC 20`; its only
caller the factory `0x140188F40` case 23 at `0x140189362`; no branch into the moved bytes: exe scan), allocation
`0x140189349` `mov ecx, 0xCD0` → 0xCF0. It overrides states 1 (`0x14018D720` / `0x14018D8D0` → the flying walk
`0x140197450`), 2 (its dash: `e023_050`, `e023_051` at `BiteSpeed` for 12000 units or until it hit, `e023_052`;
`0x14018D6A0` / `0x14018D780`), 3 (its bite `e023_053`, `0x14018D6F0` / `0x14018D8A0`); its think is
`CHeartless`'s `0x140196EC0`. Our copy: states 0, 1, 2, 3, 11, 12, 30, the think; KH1's dash in state 2, the bite and
the drop in state 3. `EN0023.ASD`: 2 records (id 0 `e023_051` on `ex_2280_sk`, 0.6 m, the whole clip; id 1
`e023_053`, 1.3 m, [22, 110)). `EN0023.SES`: `e023_051`, `e023_053`.

Power mapping (decision M10): the dash on id 0 and the bite on id 1 (the same motions: Re:CoM's clips are
bakes of KH1's); the drop on id 1 (**Assumption**: nearest, Re:CoM's whole-body record; KH1 gives the drop and
the bite the same 100 %; owner question 1).

### What the asset step writes

`ENEMY/0023/EN0023.ANN` (Re:CoM's 86 slots byte-identical; KH1's clips at 100 + entry: 101, 103, 104, 106,
107, 108 (69), 110 (200), 112 (202), 114 (204), 115 (205), 116), `.ASD` (Re:CoM's 2 records kept + 3: the bite on
`e023_114`, the dash on `e023_112`, the drop on `e023_110`), `.SES` (Re:CoM's entries kept; 112 / 114 get
`e023_051` / `e023_053`'s events; the edits their Re:CoM clip's; the drop its template's sounds at its hit:
Assumption), `kh1-darkball.json` (`kh1-enemy 1`: 151 constants, 29 requests, 3 attacks, 4 parts). Test:
`darkball::tests::builds_from_the_players_files`.

### Owner install steps

1. Run the step (`kh1-recom-build`): it prints a `darkball (M11): …` line.
2. Copy into the OpenKH mod folder and add to `mod.yml`:

```yaml
- {name: ENEMY/0023/EN0023.ANN, method: copy, source: [{name: ENEMY/0023/EN0023.ANN}]}
- {name: ENEMY/0023/EN0023.ASD, method: copy, source: [{name: ENEMY/0023/EN0023.ASD}]}
- {name: ENEMY/0023/EN0023.SES, method: copy, source: [{name: ENEMY/0023/EN0023.SES}]}
- {name: kh1-darkball.json, method: copy, source: [{name: kh1-darkball.json}]}
```

3. The log shows `darkball: KH1's darkball (M11) …` at start and `battle N: Darkball … gets KH1's AI`.

### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`)

1. A Darkball battle: each grows out of nothing about 0.8 m up (69, ~2 s), sinking a little; it can't be hit
   until it has grown.
2. Within 10 m it notices Sora (205, ~3 s), then:
3. Close (≤ 6 m): a bite (204), then it shrinks away and vanishes; it reappears above Sora growing big (twice its
   size) and shrinking back (the drop, 200), or (when its glides miss him) reappears plainly further off; then it
   circles toward Sora at his side and dashes three times (201 → 202 → 203).
4. Far (> 6 m): the vanish / reappearance, or the side approach and the three dashes.
5. Hit it: KH1's flinch / knock-back in the air; it counters (a bite when close, a dash further off); after five
   hits it warps away instead; five quick light hits in a row: the sixth and later don't flinch it and it bites.
6. While it is shrinking, invisible or growing back it can't be hit.
7. Guard the dash: KH1's clash recoil (206). Guard the bite or the drop: Re:CoM's flinch.
8. Kill one: Re:CoM's prizes and EXP.
9. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Darkballs.

Please send `kh1-recom.log` (`darkball:` lines: `visible false` / `visible true` at each warp, the attacks, `hit
without a reaction`) and an F9 trace.

### KH1 trace (what to record in KH1)

A room with Darkballs: stand far 10 s; stand at 4 m 10 s; stand at 2 m 10 s; one swing / a full combo / a
finisher; six quick light hits; Guard a dash, a bite, a drop; kill one. The trace's enemy rows give the heights,
the glides' ends against Sora and the warps' timing to compare.

### Not built / open (the Darkball)

- The leash, the retargeting to another party member, the world rules (KH1 world 11 / 15 / 16 flags: the leash
  off, a hit rule for Sora's kinds 1 / 10, the LV2 record).
- KH1's hit attribute (`hp[0xC] >> 6 & 15`) isn't carried by the host: every hit is attribute 0, so the Gravity
  stagger (attribute 8 → 205) never plays, attributes 9 / 11 change state as any other hit (owner question 4).
  The armour's status exception (`actor+0x88 & 0x102`) is taken as never set (**Assumption**).
- The effects (`sys 74` 2 / 7 / 9 at the appearance and the warps, `sys 75` 10 at the bite, `sys 21` 8 attached
  while invisible: KH1's dark smoke and its trail) and the alpha fades: the warps show as shrinking / growing
  only (owner question 2).
- KH1's flags 0x10 / 0x20 (set with 0x40 during the appearance and the warps; 0x10 is Sora's body push, M10's
  note): not mapped; Re:CoM's body push stays on (KH1's is off only while it shrinks and grows).
- Re:CoM's lock-on flag stays while it is hidden and unhittable (M10's rule; KH1's automatic target skips it):
  owner question 3.
- A Re:CoM battle spawn is a room spawn of a type other than 1 (**Assumption**: `0x1132`'s type-1 rule, which
  engages Sora at any distance, not taken).
- `sys 48`'s search is "Sora within the range" (**Assumption**, as the Soldier's).
- The plain reappearance's warp to bone code 0 uses the idle's pose at frame 20 (**Assumption** ±10 units: KH1
  reads the live pose).
- The vanish's warp sets the facing to 1.0 rad (the position's fourth word): ported as read (Checked (decomp));
  it is invisible then and turns to its glide.
- A sixth light hit that lands while it attacks: Re:CoM's handler has already cut the attack (KH1's would go on).

## The Air Pirate (M11 fliers)

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Air Pirate yet.

### KH1's side. Checked (files: `ex_2210.bd` disassembled, folded and raw; decomp: the syscalls named)

The script (0x3880 bytes; code from `0x228`, entry `0x1CF6`) has the Air Soldier's newer shape (the actor at `wp+0`,
the state object at `wp+4`), on the flight of § Flying bodies, with its own routines (only the random draws and the
waits are the shared ones). It never sets a turn rate (`sys 125`) or a flight easing (stats `+0x14`): the stats
block's defaults (π/16 a frame, 0.1).

| Routine | Offset | What |
|---|---|---|
| entry | `0x1CF6` | init `0x1E04` (records LV1 `0x14` / LV2 `0x80`; stats `+0x10` = 800, the hop, unused; work area; home; the search table `0x200`; handlers hit `0x1F0C`), `sys 135` with (0, 0, 0, 1) (flight, no velocity), a yield; outside KH1's world 11 (Olympus, `sys 105`) the appearance thread `0xA6E`; then each frame a state request restarts the behaviour: 1 engaged `0x4A2` (the target set), 2 idle `0x8D2` (the target cleared), 3 hit `0x274` (the hitter or Sora) |
| appearance | `0xA6E` → `0x195A` | face Sora (`sys 36`); spawn type 5: a rise from below (not ported, Assumption: Re:CoM's battle spawns are not type 5); else 69 (blend 0) with a 30-frame fade-in and flag 0x80, effect 2, 30 frames, then paused (flag 4, the idle queued with bit 30) until 69 ends; idle |
| idle | `0x8D2` | hits forgotten; Sora found (`sys 48`, 5000): the target, a yield, a rise of 170 when under 170 above him (`0x3706`), the circle, engaged. Not found: the circle, the idle 90–120 frames facing Sora, again |
| circle | `0x358A(time, turn)` | time [250, 500), turn [3, 5) (drawn in that order) × 0.5 degrees a frame; the facing turned −90° (208) or +90° (209, the turn negated) at random; the take-off 201 to its end (`sys 172`); `time` frames flying along the turning direction at 16 facing it; the halt 203 as an action, the idle queued |
| engaged | `0x4A2` | beyond 500 (or beyond 200 and more than 300 above the target): far rounds `0x5A4`; 200..500: mid `0x5DA`; within 200: close `0x69C`; a yield; Sora re-picked |
| far round | `0x5A4` → `0x2100` | until done (`wp+0x48`). Target below: nearly straight below (cos ≥ 0.9) 205 then the dive `0x27CA`; fairly level (cos ≥ 0.8) the wind-up 210 (beyond 1000, or under 170 above it; skipped when near `wp+0x44`) and the take-off 201, then charges at a point 30 under it until done; else the take-off and a charge at its x / z at its own height (16). Target above or level: nearly straight above (cos ≥ 0.76) the climb `0x2642`; fairly level (cos ≥ 0.86) the wind-up / take-off and charges at it; else the climb |
| charge | `0x2A22(target, p, speed, arrive, 60)` | near; flying 202 at `p` (its x / z the target's while he stands), banking 208 / 209 (blend 15) while he is off to a side (cos ≤ 0.98), back to 202 after 30 frames; within 200 of him: the swoop (more than 50 above him) or the slash, a round, done; within `arrive` (200) of `p`: done; stuck from frame 10 (moved < speed / 4 a frame): a rise of 300, done |
| dive | `0x27CA` | to 320 above where he was, diving 206 at 16 (stuck or within 100 of the floor: stop, near); from there 207 for its length sinking at 8 (only `+0x24` written: the direction's x / z stay); within 150 of him: the nudge `0x32B4`, the swoop, a round; near, done |
| climb | `0x2642` | within 150: the slash, a round; at or over 100 under him: the idle, near; else rising 204 at 16; stuck: near |
| mid | `0x5DA` | the idle, the approach `0x2E7E(10, 180)` (to 100 above him, or him while he is in the air over 100 up), the swoop, a round, the back-off |
| close | `0x69C` | 50 %: the back-off, then a rise of 200 when under 170 above the floor or under 100 above him; else the swoop (blend 0), a round, a flight to a point 5–7 m away (`0x30B6`, at 6) and the back-off; from 10 rounds the idle state |
| back-off | `0x3368(speed, far, motion)` | the flat line from him turned 180° (or −180°, at random), 10 frames, the take-off and flying (18) along it until `far` flat from him (the distance + 600 under 600, else + 300), the halt 203; stuck from frame 10: a rise of 300 |
| swoop | `0xC4E` / `0xEBC` | face him; low (under 50 above him): stop, two yields; high (over 80 above him) over a standing target: sink at 4 to 80; 200 with row `0xEC`, steering at him flat at 8 beyond 200 until 80 frames are left, those still; the idle |
| slash | `0xBC6` | 212 with row `0x15C`, facing him until 60 frames are left, those; the idle |
| hit | `0x274` | hit in the dive: 211 (blend 0) for its length; launched: 8 frames; within 150: 1–4 hits the swoop (over 50 above the floor) or the slash, 5–9 the back-off (600), else idle; within 500: 5–9 the back-off, 10+ idle; then engaged (rounds cleared) |
| hit handler | `0x1F0C` | attributes 9 / 11: nothing more (not carried); while 206 plays: a hit that is not a clash takes no reaction (`hp[0xC] &= ~0x1F`) and asks for 211, every hit counts; else a launch (kind 5) counts and asks for the pause, other hits but clashes count; the hit state 3 |

Attacks (rows `0x70` bytes, `sys 23`, mode 2: each lives while its request plays; Checked (files)):

| Attack | Request | Row (bone code → bone, t, radius, KH1 power) | Hit | Recoil `+0x40` | Re:CoM id | Re:CoM state |
|---|---|---|---|---|---|---|
| swoop | 200 (122 f, root; steered) | `0xEC` (29 → the sword hand `r_hand`, 0.1 → 1, 60, 50 %) | [41, 48) | 213 (in the model) | 1 | 3 |
| slash | 212 (100 f, root) | `0x15C` (29 → `r_hand`, −0.5 → 1, 60, 100 %) | [39.3, 51) | 110 (no motion) | 0 | 2 |

Reactions: the flying forms (F `0x48..0x4B`, K `0x4C` / `0x4D`, a `0x4E` / `0x4F`, A `0x50` / `0x51`; no launches: kind
5 → kind 4); weight 1.0 in both records; hurt parts: 7 (collision block group 2 → Re:CoM bones 47, 6, 8, 14, 20, 9, 15).
Checked (files).

Motions (`xa_ex_2210.mset`, 25 entries): KH1's matcher finds no exact Re:CoM bake on `EN0022.ANN` (74 slots); edits
(≤ 20 bones differ): the idle → `e022_001`, flinches 73 / 75 → `e022_022` / `e022_023`, 66 / 67 → `e022_070` / `e022_071`,
the slash 212 → `e022_050`. Every request is a new clip at 100 + entry; the edits get their Re:CoM clip's sounds.

### Re:CoM's side. Checked (exe + decomp)

`CEnemyAirpirate` (vtable `0x1405309E8`, 139 slots, RTTI at −8), ctor `0x14018A0B0` (`40 53 48 83 EC 20`: `CHeartless`'s
base ctor, then `+0x154 |= 0x80`, `+0x260 |= 0x100` (gravity off), `+0xCC0 |= 1`; its only caller the factory
`0x140188F40` case 22 at `0x140189338`; no branch into the moved bytes: exe scan), allocation `0x14018931F` `mov ecx,
0xCD0` → 0xCF0. It overrides states 1 (the flying walk `0x14018A310` → `0x1401979F0` with `e022_001` / `e022_002`, step
`0x14018A4D0` → `0x140197450`, end `0x14018A350`), 2 (`e022_050`, `0x14018A270` / `0x14018A370`), 3 (`e022_051`,
`0x14018A2C0` / `0x14018A420`) and the think (`0x14018A190`, its own). Our copy: states 0, 1, 2, 3, 11, 12, 30, the think;
KH1's slash in state 2, its swoop in state 3. `EN0022.ASD`: 2 records at `0x6C`, tail `0x1AC` (id 0 `e022_050` on `r_hand`,
0.5 m, [38, 60); id 1 `e022_051`, 0.5 m, [37, 52)). `EN0022.SES`: `e022_050`. `EN0022.EPD` `@ENP` attack entries: id 0 and
id 1. Re:CoM's skeleton names bone 1 `m_ex210_sk` (the asset step's converter now takes `m_ex…_sk` too; Checked (files)).

Power mapping (decision M10, "the Re:CoM attack of the same kind"): the slash on id 0 (Re:CoM's `e022_050` is an edit of
KH1's 212); the swoop on id 1 (**Assumption**: Re:CoM's other sword attack; its clip `e022_051` matches no KH1 motion
within the matcher's edit rule; owner question 1).

### What the asset step writes

`ENEMY/0022/EN0022.ANN` (Re:CoM's 74 slots byte-identical; KH1's clips at 100 + entry: 100..124), `.ASD` (Re:CoM's 2
records kept + 2: the swoop on `e022_116`, the slash on `e022_123`), `.SES` (Re:CoM's one entry kept; the slash's clip gets
`e022_050`'s sounds, its edit's; the swoop's template `e022_051` has none: no sounds), `kh1-airpirate.json` (`kh1-enemy 1`: 126 script constants read at checked offsets, 49 request
words checked, 2 attacks, 7 parts). Test: `airpirate::tests::builds_from_the_players_files`.

### Owner install steps

1. Run the step (`kh1-recom-build`): it prints an `airpirate (M11): …` line.
2. Copy into the OpenKH mod folder and add to `mod.yml`:

```yaml
- {name: ENEMY/0022/EN0022.ANN, method: copy, source: [{name: ENEMY/0022/EN0022.ANN}]}
- {name: ENEMY/0022/EN0022.ASD, method: copy, source: [{name: ENEMY/0022/EN0022.ASD}]}
- {name: ENEMY/0022/EN0022.SES, method: copy, source: [{name: ENEMY/0022/EN0022.SES}]}
- {name: kh1-airpirate.json, method: copy, source: [{name: kh1-airpirate.json}]}
```

3. The log shows `airpirate: KH1's airpirate (M11) …` at start and `battle N: … gets KH1's AI` in an Air Pirate battle.

### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`; an Air Pirate battle in Neverland)

1. Each Air Pirate plays KH1's appearance (69, ~1.3 s), rises to about 1.7 m above Sora and circles (201, then a long
   banked curve, 208 / 209, 4–8 s), stops (203) and engages.
2. Far (> 5 m): a wind-up (210, ~2 s) when very far or low, a take-off (201), then it flies at Sora (202, banking as he
   moves) and swoops (200, more than 0.5 m above him) or slashes (212).
3. Sora straight below it: 205, a vertical dive (206), a slower last stretch (207), a swoop.
4. Sora on a ledge above it: it climbs (204) and slashes.
5. 2–5 m: it flies to 1 m above Sora, swoops, then flies away (201 → 202 → 203).
6. Within 2 m: it backs off (and climbs) or swoops, flies off to a point 5–7 m away and backs off; after 10 rounds it
   circles again.
7. Hit it in the dive (206): no flinch; it plays 211, then counters. Other hits: KH1's flinch / knock-back in the air;
   1–4 hits close: a swoop or a slash; 5–9: it backs off; 10+: it circles again.
8. Guard the swoop: KH1's clash recoil (213). Guard the slash: Re:CoM's flinch.
9. Kill one: Re:CoM's prizes and EXP.
10. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Air Pirates.

Please send `kh1-recom.log` (`airpirate:` lines: `state …`, `attack 200 (swoop), steered`, `attack 212 (slash), steered`,
`action 203 then 0`, `hit without a reaction` in the dive, the `rand` lines) and an F9 trace.

### KH1 trace (what to record in KH1)

Neverland (the ship's deck or the hold): stand far 10 s; stand at 4 m 10 s; stand at 1.5 m 10 s; stand under one (let it
dive); jump onto something high; one swing / a full combo / a finisher; hit it while it dives; Guard a swoop and a slash;
kill one. The trace's enemy rows give the heights, the circle's radius and the charge's speed to compare.

### Not built / open (the Air Pirate)

- KH1's world-11 rules (Olympus cups: the search at 10000, no appearance), the spawn type 5 rise from below, the
  retargeting to another party member (`0x1364`), `sys 139`'s out-of-sight test (taken as in sight).
- KH1's hit attribute (`hp[0xC] >> 6 & 15`) is not carried by the host: attributes 9 / 11 (which end KH1's handler at
  once) and 8 / 9 (which keep the dive's reaction) never come.
- The effects (`sys 75` 2 at the appearance, 7 at each swoop; `sys 74`), the fade-in and flag 0x80 at the appearance.
- The script's two world-height tests (the close round's rise below y −170, the approach's target over y −100) are read
  as heights above the battle floor (**Assumption**: KH1's Air Pirate rooms have their floor at y = 0; Re:CoM's battle
  floors sit at any height; owner question 2). The floor is Sora's height the last time he stood (§ Flying bodies).
- The nudge `0x32B4` moves along world (0, 0.2, −1) while Sora is within 1 m flat (as the script writes it; Checked
  (files)): a KH1 quirk kept.
- The charge's bank clock (`sp+0x70`) is read before the script writes it: taken as 0 (**Assumption**).
- `sys 10`'s turning sign (the circle's and the banks' sides): the Air Soldier's convention (**Assumption**).
- Sora's KH1 movement state (`*(target + 0x70)`): 0 when his Re:CoM mover touches the floor, else 2 (**Assumption**: his
  glide counts as the air).

## The Wyvern (M11 fliers)

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Wyvern yet.

### KH1's side. Checked (files: `ex_2320.bd` disassembled, folded and raw; decomp: the syscalls named)

The script (0x3380 bytes; code from `0x378`, entry `0x21CC`) has the Air Soldier's newer shape (the actor at `wp+0`,
the state object at `wp+0x20`, the search table handle at `wp+0x30`), on the flight of § Flying bodies, with its own
routines (only the random draws and the frame wait are the shared ones). It never sets a turn rate at init: the stats
block's π/16 (11.25°), which every routine that changes it puts back; it sets the flight easing (stats `+0x14`) in its
circles, flights and chases (0.1 / 0.08 / 1, back to 0.2).

| Routine | Offset | What |
|---|---|---|
| entry | `0x21CC` | init `0x2362` (KH1's world: `wp+0x54` = world 11; records LV1 `0x14` / LV2 `0x80`; stats `+0x10` = 800, the hop, unused; stats `+0` `|= 2`, not identified (no reader found): not ported; home; the state object; the search table `0x350`; handlers hit `0x2484`, `0x2460` (answers 0), `0x2478` (ends)), `sys 135` with (0, 0, 0, 1) (flight, no velocity), a yield; placed (`sys 132` = 0): a target within 5000 or Sora, engaged; spawned: the appearance `0xC3C` → `0xC4C`. Then each frame: while idle (state 1) a target within 1500 (`0x1930`: in world 11 within 10000 and always Sora; out of sight (`sys 139`): none) → the target and engaged; a state request restarts the behaviour: 1 idle `0xB4E` and 3 lost `0x5D2` with the target cleared, 2 engaged `0x7BE` with the target (or Sora), 4 hit `0x3C4` with the hitter (or Sora) |
| appearance | `0xC4C` → `0xC6E` | face Sora now (`sys 11`); the 30-frame fade-in and flag 0x80 for 30 frames; a yield; 69 as an action (`0x1000045`, blend 8), the idle queued (bit 30, blend 12); effect 2; the idle state |
| idle | `0xB4E` | not noticed; forever: less than 170 above Sora → rise 300 (`0x30C2`); the circle `0x26EA` ([250, 500) frames, turn [3, 5)); the idle (blend 30) at turn rate 3 facing Sora [90, 120) frames |
| circle | `0x26EA(time, turn)` | turn rate 4; 60 %: the facing turned −90° and by the turn, 209; else +90°, the turn negated, 210 (blend 30); 10 frames; while the time lasts: the direction turned by turn × 0.5 a frame, facing it, flying at 16; the halt 211 easing off at 0.1 until 60 of its frames are left |
| engaged | `0x7BE` | the notice once (`0xCCC`: 215 (the idle queued) at turn rate 8 for its length + 6 frames facing the target); rounds: from 3 counted attack rounds (`wp+0x5C`) the lost state; farther than 600: 70 % the far attack `0x904`, else the charge `0xF52`; closer: the roar (`0xAF2`: 213 as an action, the idle queued), then the dive attack `0xA8C`, every third time the far attack |
| far attack | `0x904` | under 250 above the floor or less than 170 above the target: rise 300; turn to it (`0x322A` at 6°); the chase `0x2ADA` at it (30, to 2.5 m, 600 frames); the bite `0x13D8`; a round; a fly-by `0x28A4` to 6 m ahead and 4.5 m up (30, within 4 m, 500 frames); down to the target's height `0x2F5C`; then 70 % the charge (a round), else the dive attack |
| dive attack | `0xA8C` | the chase to 4 m above the target (within 1 m), the dive `0x113E`, a round |
| chase | `0x2ADA(target, p, speed, arrive, time)` | 207, 208 queued; toward `p` (its x / z the target's while he stands), facing him; after 10 frames banking (209 / 210, blend 16) while he is off to a side (cos ≤ 0.98), 208 after 16 frames (facing him until 60); within 2.5 m of him: done; within `arrive` of `p`: done once level, else stopped at once (easing 1) until the stuck test; stuck after 10 frames (moved < speed / 4): done |
| fly to | `0x28A4(p, speed, arrive, time)` | 208; along the line to `p`, its vertical part × 0.7; within `arrive` or stuck: the halt 211 (easing 0.08 to its last 60 frames), the idle (blend 10) |
| descend | `0x2F5C(target)` | turn to him (6°); 212 (blend 16) sinking at 4 until at his height (as it was) or the floor: the idle **from frame 16** (blend 24), 24 frames; stuck (< 0.4 a frame): stop |
| rise | `0x30C2(target, from, speed, dy)` | 212 (blend 16) facing him, rising at `speed` until `dy` above `from`, 300 frames at most (stuck after 30) |
| turn | `0x322A(p, rate)` | the idle; turning at `rate` until the facing's cosine to `p` is 0.99 (200 frames at most) |
| bite | `0x13D8` | facing the target at turn rate 6; 200 with rows `0xEC` / `0x15C` (blend 0 out of 208, else 8 and 8 frames); its 120 frames along the facing at 4 (sinking 0.2 while more than 30 above him), the last 20 at 24; 208 |
| dive | `0x113E(target)` | 201 (202 queued) for its length facing him, creeping at 4 while farther than 2.5 m (flat); 202 with row `0x23C` straight down at 24 until 2.5 m above where he was, 50 above the floor, or stuck; 203 with row `0x2AC` sinking at 14 until 10 above the floor, for its length; the idle |
| charge | `0xD70` (hit) / `0xF52` (engaged) | turn to him (1.4° / 3°); 204 (blend 0 / 16; `0xF52` waits 16 frames first) for its length − 2 turning at 2°; 205 with row `0x1CC` for 40 frames along the facing at 40 (sinking 0.5 while above him); 206 with row `0x1CC` again (the same hit bit), the idle queued, its length + 6 frames |
| hit | `0x3C4` | hit in the dive: 214 (blend 0) for its length; within 3 m: from 5 counted hits (`wp+0x48`, then 0) the idle 6 frames, the roar, the dive attack; within 10 m: from 5 hits the idle 6 frames, the roar, the charge `0xD70`, a rise when low, the lost state; else (or 1–4 hits) the engaged state |
| lost | `0x5D2` | hits forgotten; Sora within 5000: the target, a yield, a rise when low, the circle, the idle 6 frames, engaged; else a rise when low, the circle, the idle facing Sora [90, 120) frames, again |
| hit handler | `0x2484` | death (`sys 44` = 0): `0x26B8` (not ported: Re:CoM's defeat); attributes 2 / 3 / 4: `sys 42` (`0x1402B7E60`: the engine's element reaction) and nothing more; 9 / 11: nothing more; the turn rate 11.25; while 202 plays: a hit not a clash (kinds 6 / 7) nor of attribute 8 / 9 takes no reaction (`hp[0xC] &= ~0x1F`) and asks for 214; every hit counts; otherwise every hit but a clash counts; the hit state 4 |
| leash | `0x26D6` → `0x163A` | **not ported**: spawned and 18 m (12.6 m out of sight) from Sora, outside world 11, it is removed; in a Re:CoM battle room it would never be defeated |

Attacks (rows `0x70` bytes, `sys 23`, mode 2: each lives while its request plays; Checked (files)):

| Attack | Request | Row (bone code → bone, t, radius, KH1 power) | Hit | Recoil `+0x40` | Re:CoM id | Re:CoM state |
|---|---|---|---|---|---|---|
| bite | 200 (120 f; steered) | `0xEC` (45 → `r_toes1a`, 0, 60, 100 %), `0x15C` (44 → `l_toes1a`, 0, 60, 100 %) | [20, 84) | 110 (no motion) | 0 | 2 |
| dive | 202 (120 f loop; steered) | `0x23C` (0 → `head`, 0.5 → −2, 60, 100 %) | all along | 110 (no motion) | 1 | 3 |
| landing | 203 (100 f; steered) | `0x2AC` (the same) | [0, 36) | 110 (no motion) | 1 | 3 |
| charge | 205 (120 f loop; steered) | `0x1CC` (the same; attack type 3) | all along | 110 (no motion) | 1 | 3 |
| charge end | 206 (106 f; steered) | `0x1CC` again | [0, 24) | 110 (no motion) | 1 | 3 |

No row names a recoil the model has: guarding any Wyvern attack plays **Re:CoM's flinch** (owner 2026-11-02).

Reactions: the flying forms (F `0x48..0x4B`, K `0x4C` / `0x4D`, a `0x4E` / `0x4F`, A `0x50` / `0x51`; no launches: kind
5 → kind 4); weight 1.0 in both records; hurt parts: 11 (collision block group 2 → Re:CoM bones 35, 2, 4, 19, 27, 29, 37,
3, 18, 4, 19). Checked (files).

Motions (`xa_ex_2320.mset`, 28 entries): KH1's matcher finds Re:CoM bakes on `EN0024.ANN` (77 slots) for the two banks
(209 → `e024_005`, 210 → `e024_006`); edits (≤ 20 bones differ): the idle → `e024_001`, flinches 72–75 → `e024_020..023`,
66–70 → `e024_070..074`, the bite 200 → `e024_050`, the wind-up 204 → `e024_051`, the charge 205 → `e024_052`, its end 206
→ `e024_053`, 207 → `e024_002`, 208 → `e024_003`, the halt 211 → `e024_004`, the notice 215 → `e024_075`. Every request
but the banks is a new clip at 100 + entry; the edits get their Re:CoM clip's sounds.

### Re:CoM's side. Checked (exe + decomp)

`CEnemyWyvern` (vtable `0x140555DC8`, 139 slots, RTTI at −8), ctor `0x140195ED0` (`40 53 48 83 EC 20`: `CHeartless`'s base
ctor, then `+0x154 |= 0x80`, `+0x260 |= 0x100` (gravity off), `+0x2E0` = 0.1, `+0xCBC` = 10; its only caller the factory
`0x140188F40` case 24 at `0x14018938C`; no branch into the moved bytes, no pointer to them: exe scan), allocation
`0x140189373` `mov ecx, 0xCF0` → 0xD10 (its states use fields up to `+0xCE8`). It overrides states 1 (the flying walk
`0x1401960B0` → `0x1401979F0` with `e024_001` / `e024_003`, step `0x1401964E0`: circling with `e024_005` / `e024_006`,
`e024_004`), 2 (`0x140195FB0` / `0x1401961B0`: beyond 4 m `e024_002` → `e024_003` to within 1 m, then the claw
`e024_050`), 3 (`0x140196060` / `0x140196350`: `e024_051` → `e024_052` at `RushAtkSpd` → `e024_053`); its think is
`CHeartless`'s `0x140196EC0`. Our copy: states 0, 1, 2, 3, 11, 12, 30, the think; KH1's bite in state 2, the dive, the
landing, the charge and its end in state 3. `EN0024.ASD`: 3 records at `0x10C`, tail `0x2EC` (id 0 `e024_050` on
`r_foot2` and `l_foot2`, 0.5 m, [20, 80); id 1 `e024_052`, a 0.55 m line `head` → `tail2`, the whole clip). `EN0024.SES`:
`e024_050`, `e024_052`, `e024_053`, `e024_073`, `e024_075`.

Power mapping (decision M10, "the Re:CoM attack of the same kind"): the bite on id 0 (Re:CoM's claw `e024_050` is an
edit of KH1's 200, both on the feet); the charge and its end on id 1 (Re:CoM's rush `e024_051..053` are edits of KH1's
204..206); the dive and its landing on id 1 (**Assumption**: no Re:CoM twin; their rows are the charge row's shape, a body
ram; owner question 1).

### Framework additions (M11, the Wyvern)

- `Cmd::MotionAt { request, blend, start }`: KH1's `sys 13` with a start frame (its fourth word, the request entry's
  `+4`, `0x1402B9F80` / `0x14029F980`). The host plays it with Re:CoM's own start argument: the motion play's third
  argument (`0x1401A3470` → `0x14024D840` → `0x14024C760` → `0x14024E9F0`: the controller's time `+0x3C` = it × its
  speed `+0x4C`, the clock `0x1401A47C0` reads `+0x3C`; Checked (decomp + disasm)), in clock units (50 a frame).
  **Assumption**: the controller's speed is 1 for our clips. The Wyvern's descent ends in the idle from frame 16
  (`0x2FD4` / `0x3022`): the survey's "`enemy::STARTS` covers it" was wrong (STARTS cuts the request's clip for every
  play; the idle plays from frame 0 everywhere else).
- Asset step `enemy::LOOPS`: requests whose new clip loops although its last pose isn't its first. KH1 loops an enemy
  motion unless its request has bit 16 (§ The Bandit, Checked (decomp)); Re:CoM holds a clip not marked looping. Only
  where a script plays one past its length: the Wyvern's wind-up 204 (108 frames) in its engaged charge (16 + 106).

### What the asset step writes

`ENEMY/0024/EN0024.ANN` (Re:CoM's 77 slots byte-identical; KH1's clips at 100 + entry: 100..127 but the banks), `.ASD`
(Re:CoM's 3 records kept + 6: the bite's two feet on `e024_112`, the dive on `e024_114`, the landing on `e024_115`, the
charge on `e024_117`, its end on `e024_118`), `.SES` (Re:CoM's 5 entries kept; the edits get their Re:CoM clip's sounds,
the dive and the landing the rush's at their windows: Assumption), `kh1-wyvern.json` (`kh1-enemy 1`: 123 script
constants read at checked offsets (and the stats block's two defaults), 49 request words checked, 5 attacks, 11 parts). Test:
`wyvern::tests::builds_from_the_players_files`.

### Owner install steps

1. Run the step (`kh1-recom-build`): it prints a `wyvern (M11): …` line.
2. Copy into the OpenKH mod folder and add to `mod.yml`:

```yaml
- {name: ENEMY/0024/EN0024.ANN, method: copy, source: [{name: ENEMY/0024/EN0024.ANN}]}
- {name: ENEMY/0024/EN0024.ASD, method: copy, source: [{name: ENEMY/0024/EN0024.ASD}]}
- {name: ENEMY/0024/EN0024.SES, method: copy, source: [{name: ENEMY/0024/EN0024.SES}]}
- {name: kh1-wyvern.json, method: copy, source: [{name: kh1-wyvern.json}]}
```

3. The log shows `wyvern: KH1's wyvern (M11) …` at start and `battle N: Wyvern … gets KH1's AI` in a Wyvern battle.

### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`; a Wyvern battle in Hollow Bastion)

1. Each Wyvern faces Sora and plays KH1's appearance (69, 1 s); within 15 m it notices him (215, ~3.5 s).
2. Close (≤ 6 m): a roar (213), then it flies to about 4 m above Sora (207, then 208 / banking 209 / 210), hovers into
   a dive (201), drops straight down (202) and lands (203); every third time the far attack instead.
3. Far (> 6 m), most often: it flies at Sora (207 / 208), bites (200), flies on past and up (208, halting with 211),
   comes down to his height (212, then the idle), then charges (wind-up 204, a rush 205 along the ground, 206) or dives.
   Otherwise the charge alone.
4. After three attack rounds it rises and circles (209 / 210, 4–8 s), halts (211), then attacks again.
5. Hit it during the dive (202): no flinch; it plays 214. Other hits: KH1's flinch / knock-back in the air; from the
   fifth counted hit within 3 m: the roar and a dive; at 3–10 m: the roar, the charge, then it rises and circles.
6. Guard any attack: Re:CoM's flinch (no KH1 recoil in the model), then its hit behaviour.
7. Kill one: KH1's reaction motion, Re:CoM's prizes and EXP.
8. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Wyverns.

Please send `kh1-recom.log` (`wyvern:` lines: `state …`, `attack 200 (bite), steered`, `attack 202 (dive), steered`,
`attack 205 (charge), steered`, `action 213 then 0`, `motion 0 b24 from 16`, `hit without a reaction` in the dive, the
`rand` lines) and an F9 trace.

### KH1 trace (what to record in KH1)

Hollow Bastion (the Rising Falls or the Castle Gates): stand far 10 s; stand at 4 m 10 s; stand at 2 m 10 s; one swing /
a full combo / a finisher; hit it while it dives; five quick hits close, five at 5 m; Guard a bite, a dive, a charge;
kill one. The trace's enemy rows give the heights, the chase and charge speeds and the circle's radius to compare.

### Not built / open (the Wyvern)

- The leash, the party retargeting (`0x18D6`), `sys 139`'s out-of-sight test (taken as in sight), the placed start
  (Re:CoM's battles spawn every Wyvern), the death routine.
- KH1's hit attribute (`hp[0xC] >> 6 & 15`) isn't carried by the host: the element reactions of attributes 2 / 3 / 4
  (`sys 42`; owner 2026-11-02: left out for now), attribute 8 / 9's dive reaction and 9 / 11's early end never come.
- The effects (`sys 75` 2 at the appearance), the fade-in and flag 0x80 at the appearance, the stats block's flag 2.
- KH1's world 11 rules (the 100 m search, always Sora) apply in Re:CoM's Olympus by decision 21's mapping, as the
  Bandit's Agrabah rules (**Assumption**: Re:CoM's Olympus battles hold Wyverns at all).
- The charge's two rows share one KH1 hit bit (one hit per charge); here 205's and 206's records are two Re:CoM attacks.
  **Assumption**: no second hit in practice (206 starts 40 frames after 205; Sora's KH1 post-hit invulnerability, 30
  frames from his reaction's end, decision 31, covers it). The bite's two rows are one Re:CoM attack (one clip).
- The queued idle after 206 is played by the behaviour when 206 has played (the host gives a steered attack no queue;
  the Darkball's rule).
- The floor for `sys 89` is Sora's height when he last stood (**Assumption**: flat battle floors); the descent and the
  dive end at it.
- `sys 10`'s turning sign (the circle's and the banks' sides): the Air Soldier's convention (**Assumption**).
- The 206 row is made the frame before 206 is asked for (`0xEFC`, then `sys 13` at `0xF14`): taken as living on 206
  (**Assumption**: KH1's mode-2 object takes its owner's request at its first tick, after the script; 206's HIT track is
  on [0, 24)).

### Wyvern owner questions

1. **The dive's power**: Re:CoM's Wyvern has its claw (id 0) and its rush (id 1). KH1's bite and charge have Re:CoM twins
   (the claw, the rush); KH1's dive (202, its landing 203) has none. Chosen: the rush's (id 1), the same body-ram row as
   the charge. (a) Keep, (b) the claw's, (c) another rule.

## The other fliers: not ported yet (survey for the next pass)

Search Ghost, Air Pirate, Gargoyle, Darkball and Wyvern were surveyed, not ported (time); the Darkball, the
Air Pirate and the Wyvern are built since (§ The Darkball, § The Air Pirate, § The Wyvern). All five enter flight
once with `sys 135` and steer through `+0x20` / `+0x2C` like the Air Soldier (6–14 steering writes each), so § Flying
bodies covers their bodies; each is a new script (1170–1710 listing lines) to port routine by routine. Checked
(files: the scripts listed; exe: ctors, allocation sites, vtables) unless marked:

| Enemy | Script (code from) | Re:CoM class (ctor / alloc / vtable) | Re:CoM `.ASD` | Open before porting |
|---|---|---|---|---|
| Search Ghost | `ex_2230.bd` (`0x5DC`) | `CEnemySearchghost` `0x1401923F0` / `0x1401891CF` **0xCE0** / `0x14054C408` | 2 (`l_fing3b`, ids 0 / 1) | a child object (`sys 94` at `0xD12`, its drain: on Re:CoM's own bullets, checklist step 9); `sys 116 / 117` (part masks); table-1 syscalls `sys1.0` / `sys1.3` (the lister reads only table 0) |
| Air Pirate (**built**) | `ex_2210.bd` (`0x228`) | `CEnemyAirpirate` `0x14018A0B0` / `0x14018931F` 0xCD0 / `0x1405309E8` | 2 (`r_hand`, ids 0 / 1) | `sys 118` (the stuck tests: `+0x10 − +0x490`), `sys1.0` (a no-op): § The Air Pirate |
| Gargoyle | `ex_2220.bd` (`0x538`) | `CEnemyGargoyle` `0x14018E150` / `0x1401891A5` 0xCD0 / `0x14053F198` | 2 (both wings, id 0 only) | a child object (`sys 94` at `0x1B24` with rows `0x3FC` / `0x46C`, request 219): on Re:CoM's bullets; one Re:CoM power for all its attacks |
| Darkball (**built**) | `ex_2280.bd` (`0x338`) | `CEnemyDarkball` `0x14018D5D0` / `0x140189349` 0xCD0 / `0x14053B7A8` | 2 (`ex_2280_sk`, a whole-clip window and a 1.3 m one) | `sys 86` (the warp), `sys 74` (effects), `sys1.0` / `sys1.3`: § The Darkball |
| Wyvern (**built**) | `ex_2320.bd` (`0x378`) | `CEnemyWyvern` `0x140195ED0` / `0x140189373` **0xCF0** / `0x140555DC8` | 3 (header to `0x10C`) | the turn rate (`Body::turn`), `sys 42` (KH1's element reactions: left out), the idle from frame 16 (`Cmd::MotionAt`, not `enemy::STARTS`): § The Wyvern |

**Found in the second pass (2026-11-02).** Checked (exe + decomp + files) unless marked:

- **Syscall table 1** (the scripts' `sys1.N`): the VM (`0x1402CC620`, op 11) takes `param[4] + mode × 8`; the
  script context's array at `0x140529FA0` holds table 0 `0x140529180` and table 1 `0x1405752D0`.
  `sys1.0(n)` = `0x140394D80`, a bare `ret` (a debug print of the state number: nothing to port);
  `sys1.3(h)` = `0x140394DE0`: marks the child object of handle `h` for removal (`*(obj+0x10) = 1`) and
  returns 0. `tools/kh1-ai-script.py` now reads both tables (argument counts were wrong for `sys1.*` before).
- **Child objects** (`sys 94` → `0x1402BC030` → `0x14028F0A0(actor, idx, target, a, b)`): entry `idx` of the
  model's sub-object table (`actor+0x13C`), the `.mdls` part 3 (absent in the Air Soldier's), which holds its
  own script:
  - **Search Ghost** `ex_2230_00.bd` (0x400 bytes, entry `0x202`): **not a projectile.** While the parent
    plays its idle (0) or its search motion (202), it makes an effect on the parent (`sys 109(parent, 7, 45)`,
    removed by `sys 97`) and each frame looks along bone code 4141 (`sys 19`, `sys 115(…, 45, …)`): a party
    member within 300 inside a cone (`sys 100` against `sys 3(35)`, 35°) becomes the parent's target (`sys 49`).
    So it ports as part of the Search Ghost's behaviour (a cone test against Sora) plus, for the look, a Re:CoM
    effect on the body (which effect: open). No bullet needed.
  - **Gargoyle** `ex_2220_00.bd` (0xC00 bytes, entry `0xB22`, made with the parent's rows `0x3FC` / `0x46C` and
    request 219 at `0x1B24`): a separate object that moves on its own (`sys 15` / `16` / `18` / `24` / `26` /
    `39` / `119` / `120`, not read yet) and makes attack rows on the parent (`0x234`, `0x868`): a moving hit,
    so it needs Re:CoM's own bullets (`docs/recom-m7-magic.md`'s shots) or the parent's own attack records,
    after its movement syscalls are read. Not decided: options in the report / owner question 8.

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

## Ground-melee family, part 1

Offsets below are in the player's `ex_2030.bd` (`xa_ex_2030.mdls` part 5). These are notes, not code.

### Re:CoM side (all four)

Checked (exe):
- Each ctor starts `40 53 48 83 EC 20 45 33 C9 45 33 C0`. Its only caller is the factory `0x140188F40`.
  No branch lands in its first 6 bytes.
- Each vtable has 139 slots. States 0 / 11 / 12 / 30 are `CBattleEnemy`'s (as the Soldier's), so the
  appearance is `Appear::Base`.
- The factory sizes are `B9 <size>`:

| Class (id) | vtable | ctor | factory size site | size | caller |
|---|---|---|---|---|---|
| Powerwild (7) | `0x140547798` | `0x140191380` | `0x1401890A9` | `0xCD0` | `0x1401890C2` |
| Bandit (11) | `0x140534368` | `0x14018AEB0` | `0x140189151` | `0xCD0` | `0x14018916A` |
| Pirate (21) | `0x140546458` | `0x140190F30` | `0x1401892F5` | `0xCD0` | `0x14018930E` |
| Large Body (9) | `0x140542AD0` | `0x14018F090` | `0x1401890FD` | `0xCE0` | `0x140189116` |

Attack states and clips (state begin → the clip name it plays; Checked (exe)):
- Powerwild: 2 → `e007_051` (`.ASD` attack id 0, one hit), 3 → `e007_050` (id 1, two hits).
- Bandit: 2 → `e011_050` or `e011_051`, 3 → `e011_051`.
- Pirate: 2 → `e021_050`, 3 → `e021_051`.
- Large Body: 2 → `e009_050`, 3 → `e009_053`.

### Powerwild's script

- **Init `0x2346`.** Walk 3, run 7, hop 500. Stat records `0x14` / `0x80`. Handlers: hit `0x2436`, event
  `0x23D6` (event 1001: `wp+0x38` = 3 → motion 260; scripted, Assumption: no Re:CoM battle sends it).
- **Supervisor `0x2262`.** While idle, it engages Sora within 1500. Its states are 1 idle `0x78C`, 2 engaged
  `0x5A2`, 3 hit `0x4B0`.
- **Appearance `0x7BC`** (spawn type ≠ 0 / 16). It faces Sora at once and fades in over 30 frames, then
  yields. Then action 69 (`0x1000045`, blend 8), idle queued (blend 12).
- **Idle loop.** The shared wander (350 / 450, band 100, timeout 300, idle 40 % for 90–180 frames, home
  700), then action 223 (blend 8) and a wait for the ground.
- **Engaged.**
  - First, the notice: motion 227, wait until it ends or request 6 plays, then the idle.
  - Close (< 300) alternates: the first time a backward hop (215); the next time a side hop (207 or 202,
    50 / 50), then attack 212.
  - Mid (< 700): a strafe (`0x13AA`: walk motion at speed 7, 60–120 frames, turning 1° a frame, stopping
    within 300). Then 60 % a pause (`0x9E6`) or else roar 228 (`0x8D8`). Then 70 % a hop-strafe (`0xA8E`)
    followed by the pounce (`0xD34`), or else attack 212.
  - Far: approach (run, timeout 150, retarget 20, stop at 560). Then 30 % an idle of 60–120 frames. Then
    60 % attack 212, 20 % roar 228, 20 % the pause.
- **Hit behaviour `0x4B0`.**
  - `wp+0x38` = 4 → motion 224, then idle.
  - Otherwise, within 700: 50 % flurry 214 then the backward hop; else a side hop then the pounce.
- **Hit handler `0x2436`.**
  - Damage of kind 4 / 5 above 40 % HP: its own launch (`sys 38` → `0x1402B79F0`), base 270 on the
    ground or 280 in the air, hop 80, speed 25, no down time; then `wp+0x38` = 4.
  - Otherwise `wp+0x38` = 2.
  - A clash, or kinds 8 / 10: `wp+0x38` = 2. Every case asks for state 3.
- **Hops `0xBEC` → `0x1152`.**
  - Plain motion r (215 / 207 / 202), for its length.
  - Then (r + 1) | `0x10000`, and `sys 82` → `0x1402AF150`: movement state 3. Its step is `0x1402AFAE0`.
    The velocity is the facing turned 180 / −70 / 70° × 20, height 50, 30 frames. g = 2h / (T/2)²,
    vy₀ = −g·T/2. Horizontal drag 0.9 (√0.9 at step < 2); vy capped at 20.
  - `sys 83` → `actor+0x3D0` = r − 3, so the apex motion (`0x1402AF630`: 6 frames before the top) is
    r + 2, with r + 3 queued.
  - Landing goes straight to movement state 0, with no landing motion (`0x1402AFAD0`). State 3 counts as
    airborne for reactions (`0x1402B1B10`: the state table's flag & 3 == 1).
- **Pounce `0xD34`.**
  - Jump start, hop 200, speed min((d − 200) / 30, 10).
  - In the air it steers at 7. From frame 19 it is action 222 (`0x10000DE` is bit 24) with rows `0x2AC` /
    `0x31C`.
- **Rows** (`+0x3C` / clash recoil):
  - 212: `0x23C` (50 / 226);
  - 214: `0xEC`, `0x15C`, `0x1CC` (10 / 225);
  - 222: `0x2AC`, `0x31C` (10 / 229).
- **Power mapping** (decision M10 pattern).
  - Checked (files): Re:CoM's `e007_051` is KH1's 212 (62 frames, one hit), and its `e007_050` is KH1's
    214 (98 frames).
  - So 212 → id 0 / state 2, and 214 → id 1 / state 3.
  - 222 → id 1 / state 3 (`+0x3C` 10, as 214's). Assumption: the nearest Re:CoM attack.

**Framework additions** (built): `Body::timed_jump` / `timed_apex` (movement state 3, the step above;
airborne for reactions; it lands with no landing motion: the host skips request 7), `react::scripted_launch`
(`0x1402B79F0`) with `Behaviour::scripted_launch` (read after `on_hit`; it replaces the reaction KH1 chose
first, whose random draw stays drawn). Assumption: the engine's hit processing runs before the script's hit
handler. The asset step now makes one `.ASD` record per HIT window (`hit_windows`), as Re:CoM's own models do.
The Soldier's and the Air Soldier's tracks have one window each, so their outputs are unchanged (their record
counts are tested).

### The Powerwild (built)

- `kh1_core::powerwild` (the AI above, every number a manifest constant), `ground::powerwild` (class row:
  attack states swipe 2, flurry 3, pounce 3; `reads_hp`), sites `POWERWILD_HOOKS` / `_ALLOC` / `_PTRS`,
  `kh1_recom_assets::powerwild` (constant offsets, request checks, rows, power mapping).
- **Asset outputs** (`kh1-recom-build`, from the player's `xa_ex_2030.mset` / `.mdls`, `btltbl.bin` and Re:CoM's
  `ENEMY/0007`): `ENEMY/0007/EN0007.ANN` (Re:CoM's 75 slots kept, KH1's clips after), `EN0007.ASD` (Re:CoM's 3
  records kept, 9 added: swipe 1, flurry 3 + 2 + 1, pounce 2), `EN0007.SES`, `kh1-powerwild.json`. In `mod.yml`:

```yaml
- {name: ENEMY/0007/EN0007.ANN, method: copy, source: [{name: ENEMY/0007/EN0007.ANN}]}
- {name: ENEMY/0007/EN0007.ASD, method: copy, source: [{name: ENEMY/0007/EN0007.ASD}]}
- {name: ENEMY/0007/EN0007.SES, method: copy, source: [{name: ENEMY/0007/EN0007.SES}]}
- {name: kh1-powerwild.json, method: copy, source: [{name: kh1-powerwild.json}]}
```

- **Checked (files):** the script's numbers at their offsets (the asset test reads them from the player's
  file), the request words, the rows and their recoils (226 / 225 / 229, all in the model), the hit windows
  (`e007_144`: right hand 20–26, 36–40, 48–54; left hand 28–36, 64–70; body 22–56).
- **Checked (exe / decomp):** movement state 3 and the scripted launch (above), the sites.
- **Assumptions:**
  - The apex motion plays one frame later than KH1's (the script runs before the step).
  - Request bit 16 (`0x10000`) is a plain request.
  - Event 1001, the leash and the death routine are not built.
  - The pounce takes the flurry's power.
  - Seeds per enemy.
- **Not checked yet:** a tester's trace against KH1 frame for frame.
- **Owner test list** (Combat = KH1, Sora's story, `enemy_ai = kh1`; a Powerwild battle in Deep Jungle):
  1. The first look (227), then by distance:
     - close: a back hop, next time a side hop and a swipe;
     - mid: a walk sideways, a pause or a roar, then a sideways shuffle into a side hop and swipe, then the
       pounce;
     - far: it runs in.
  2. Hit it with a launching finisher while above 40 % HP: it flips back with its own launch (270), then shakes
     it off (224).
  3. The hops land with no landing pose and travel about 3 m.

### The Bandit (built)

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Bandit yet. The block
stance's block and the spin's repel run on the shared enemy-guard step (§ Enemy guard), the thrown blade on a
Re:CoM homing bullet (§ The Bandit's thrown scimitar) (owner 2026-11-02: questions 1 (a), 2 (b)).

#### KH1's side. Checked (files: `ex_2090.bd` disassembled; decomp: the syscalls and engine functions named)

The script (0x4700 bytes, code from `0x4A0`) has the Air Soldier's newer shape (the actor at `wp+0`, the state
object at `wp+0x30`). Its child object `ex_2090_00.bd` sits in the `.mdls` part 3.

| Routine | Offset | What |
|---|---|---|
| entry | `0x317C` | init `0x34B0` (records LV1 `0x14` / LV2 `0x80`, walk 2, run 8, hop 400, handlers hit `0x367A`); party / group handles; KH1's world `wp+0x48` and room `wp+0x4C` (`wp+0x64` = 0 only in Agrabah rooms 2 / 17); the appearance `0x174A`; each frame a state request restarts the behaviour (1 idle `0x1190`, 2 engaged `0xB38`, 3 lost `0x8D0`, 4 hit `0x4FA`); while idle the search: a spawned Bandit engages within 1500 |
| appearance | `0x174A` → `0x2C8A` | home = here; Sora faced now; 69 (plain, blend 0), the 30-frame fade-in, 30 frames, the idle queued (`sys 14`) with the action flag set: KH1 loops 69 and plays the idle at its next cycle end, frame 48 |
| idle | `0x1190` / `0x1346` | Sora's target is it → engaged; else the wander: points [200, 400) and [400, 500) about where it stands, walked to (300 each); 70 %: 40 % of all an idle wait [238, 308) (Sora targeting it → engaged), then again; else (Agrabah) farther than 500 from home: home (600) and an idle wait |
| engaged | `0xB38` | < 200 close `0x107A`, < 400 mid `0xFA8`, else far `0xD8E`; then engaged again |
| close | `0x107A` | 40 %: back hop, spin, strafe; else strafe, pause, slash, back hop, strafe; the closing pause [60, 75) + 8 |
| mid | `0xFA8` | strafe (turn 3), pause [60, 90) + 8; 70 %: run in (90 + 8 frames, to 200), slash, strafe; else the leap; the closing pause |
| far | `0xD8E` | Agrabah: 30 % throw then run in to 600, 50 % leap, 20 % run in to 600; the closing pause. Elsewhere: past 2000 from Sora the leash `0x3B12` (not ported: nothing that round), else run in to 400 and a pause [30, 60) + 8 |
| hit | `0x4FA` | out of an arc; a swing blocked during the stance → back hop and throw; past 400: 50 % throw, else leap; after a 1st / 2nd hit seen (`wp+0x54`): 50 % the block stance, 20 % slash, 30 % back hop + leap; else 50 % leap, else spin. Outside Agrabah one more round by distance: two slashes (< 200), the leap (< 400), the spin. Then engaged |
| lost | `0x8D0` | Sora's target is it → the throw; Sora alive → engaged (reachable only after a clash with no attacker) |
| hit handler | `0x367A` | hits of attack type 9 / 11 (record `+0xC` bits 6..9): damage only (not told apart here); a clash (kinds 6 / 7): the hit state, the stance's flag kept; a damaging hit: the stance's flag cleared, `wp+0x54` = 1 when record `+0x24` is 1 or 2, **kind 4 → 5** (its launch), the hit state |
| strafe | `0x4334` | 60 % to −90° of the facing, else +90° (turn negated); run (request 2) at 8 for time + 10 frames, turning by turn × 0.5° a frame; stops off the ground or within 200; time from `0x4606`: 50 % [30, 45), 30 % [60, 90), 20 % [120, 150) |
| pause / approach | `0x44AE` / `0x42DC` | the idle for n + 8 frames until within 400 / run at the target (t + 8 frames, re-read every 20) to d |

Attacks (each a plain request with attack rows: the script keeps running; KH1's bone rows end when the request
changes, `0x1402B2DC0` sets flags `1001b` = mode 2, `0x1402B47A0`):

| Attack | Requests | Damaging rows (bone code → bone, t, radius; hit slot) | Hit windows | Recoil | Re:CoM id |
|---|---|---|---|---|---|
| slash | 200 (60 f, root) | `0x23C` (48 → 120 blade, 2.5 → 0, 40; 1) | [26, 38) | 205 | 0 |
| spin | 210 (46 f) wind-up, 211 (40 f loop) ×4, 212 (94 f) | `0x2AC` on 211 (blade; 0), `0x31C` on 212 (blade; 0) | 211 [0, 16); 212 [0, 12), [34, 50) | 213 | 1 |
| leap | 216 (60 f) crouch, 217 (20 f loop) flight, 218 (40 f, bit 16) | `0x3FC` (blade, 2.5 → 0, 40; 0), `0x38C` (blade, 0.3, 80; 1) on 217 | [0, 20) each | 205 | 1 |
| throw | 201 (438 f, root): the blade leaves at 90, back at 380 | the child's row `0x1CC` (no bone, 70) | the blade's flight | — | 1 (the blade, Assumption) |
| block stance | 230 (12 f), 231 (60 f loop) [60, 90) frames, 233 (16 f) | none: its blade row `0x15C` is guard-type | guard: 230–233's HIT slot 0 | (232) | — (guard) |

- **The spin** glides at the target (`sys 40` mode 2 at the stats block's `+0xC` = 4) through four rounds of 12 + 26
  frames (len − 2 − 12), each part ended off the ground; a round's landed hit ends the rounds (`wp+0x80`, the row's
  callback `0x228A` = Re:CoM's `CCollAttack` apply answering 1: `Behaviour::on_landed`).
- **The leap** is movement state 7 (`sys 169` → `0x1402BB030`, the flight step `0x1402CC000` with flags 1):
  `0x1D42(217, Sora + 50 up, 70, 100, 500)` steers at 70 each frame (easing 0.1), the goal below and the body more
  than 20 above the floor → straight down; it stops steering at the first frame within 100 of the point, so the
  flight velocity then eases off at 0.9 a frame and carries it on past Sora through 218 (KH1's physics: its stuck
  test `0x24FE(70 × 0.1)` is that ramp's first frame); then `sys 88(0)` and, above the floor, a drop.
- **The back hop** `0x41B8` → `0x41DC(221, 25, 60, 30, 180)`: movement state 3, 30 a frame backward, 60 high, 25
  frames, drag 0.9 (≈ 4.3 m), the trio 222 / 223 / 224 (`sys 83`).
- **Motions loop** unless their request has bit 16 (`0x14029FC40` copies it to `+0x160` bit 3; `0x14029F0A0` loops
  back to `+0x288`, 0 for enemies, or holds): `sys 172` is the end of each cycle (the port's `done_now`).
- **Rows that never hurt Sora** (attack type 2: `+0x38` low bits → object bits 17..19; type 2 meets no hurt body and
  only types 0 / 3 attacks, `0x140528CB8`): the spin's body row `0xEC` (code 8, 60, priority 90, recoil 213) and the
  stance's blade row `0x15C` (code 48, 40, priority 30, recoil 232). In KH1's attack-vs-attack resolution
  `0x1402B4BC0` the lower-priority side gets a clash record (kind 6, kind 7 when 40 or more under) and a type-2 object
  never repels: Sora's swing that meets them is deflected, the Bandit plays the row's recoil and counters (after the
  stance: the back hop and the throw).
- Reactions: the shared engine with the Bandit's motions (flinch 0x48..0x4B, air flinch 0x4E / 0x4F, air knock-back
  0x50 / 0x51, launch 0x52.., air launch 0x5C..; no ground knock-back: kind 4 launches, and the handler makes every
  damaging kind 4 a kind 5). Weight 1.0 in both records. Hurt parts: 6 (collision block group 2 → Re:CoM bones 16, 2,
  26, 11, 7, 42). Checked (files).
- Motions (`xa_ex_2090.mset`, 48 entries): KH1's matcher finds Re:CoM bakes for the idle 0 (`e011_001`), walk 1
  (`e011_002`) and the landing / appearance entry 7 (`e011_073`, requests 7 and 69); every other request is a new clip
  at 100 + entry. The pose rule marks 201, 211, 217, 231 looping (the loops the script relies on).

#### Re:CoM's side. Checked (exe + decomp)

`CEnemyBandit` (vtable `0x140534368`, 139 slots, RTTI COL at −8), ctor `0x14018AEB0` (`40 53 48 83 EC 20`, its only
reference the factory's call at `0x14018916A`; no branch into the moved bytes, no pointer to it: exe scan),
allocation `0x140189151` `mov ecx, 0xCD0` → 0xCF0. Its own states 1 (`0x14018AFF0` / `0x14018B3A0`), 2 (slash
`e011_050`, `0x14018AF70` / `0x14018B120`), 3 (its whirl `e011_051` → `052` → `053`, `0x14018AFA0` / `0x14018B160`);
0 / 11 / 12 / 30 `CBattleEnemy`'s; think `0x140196EC0`. Our copy: states 0, 1, 2 (the slash), 3 (spin, spin end,
leap), 11, 12, 30, the think. `EN0011.ASD`: 2 records at `0x6C`, tail `0x1AC` (id 0 `e011_050` on `wep0` 28–40, id 1
`e011_052` a 0.5 m `wep0` sphere through its loop). `EN0011.SES`: `e011_050`, `e011_052`. Re:CoM's Bandit has no
throw and no projectile (Checked (decomp): its state code).

**Power mapping** (decision M10): slash 200 → id 0 (Re:CoM's slash: the same swing, its window two frames later;
Checked (files)); spin 211 / 212 → id 1 (Re:CoM's whirl, the same kind of attack); leap 217 → id 1 (Assumption: a
moving lunge, nearest Re:CoM's charge; all its rows' `+0x3C` are 30, the slash's too).

#### Framework additions (commit `625e428`)

- `Body::air_flight` / `air_fly` / `ground`: movement state 7 (flight physics, airborne for reactions; a reaction's
  hop, a timed jump, flight or `sys 88` ends it).
- `Behaviour::set_world` (KH1's world id: `kh1_enemy::kh1_world` maps Re:CoM's world as decision 21 maps the twins'
  records; Checked (exe): KH1's code table `0x1404DCCA0`, 8 = `al`); `set_hit_count` (record `+0x24` = the actor's
  counter `+0x3E8`, one more per record clashes included, `0x1402BFE60`, zeroed when no action plays, `0x1402B7260`;
  the host keeps it per enemy); `reaction_kind` (KH1 runs the script's handler before the reaction, `0x1402B7B90` →
  `0x1402BCD60`, then `0x1402B8E10`'s switch; the host applies it to damaging hits and the killing hit).
- **Asset step fix** (`Converter::kh1_point_in`): KH1 takes a row's point from the parent's world matrix times the
  bone's local translation (`0x1401D6320`, then the lerp `0x1400E64D0`). For a chain-end bone the motion doesn't pose
  (flags 0x6000000) the old code read its never-built world matrix (the origin): the Bandit's blade record came out
  3.3 m long; now 1.30 m along `wep0` (Re:CoM's own slash record is 1.16 m out on the same bone). **The Air Soldier's
  spin row `0x4DC`** (left toes' end bone, 1 → −1, 70) had the same fault (a 1.7 m line through the origin); it is now
  ±7 cm about `l_toes`, so its `EN0015.ASD` changes: re-run the asset step. Soldier and Powerwild outputs are
  byte-identical (hashed before / after); the Shadow's `.mset` isn't in this session (its rows' bones are posed:
  unchanged by construction).
- Found, not changed: the scripted launch (`sys 38`, `0x1402B79F0`) sets `*hit |= 2`, so KH1 skips its own reaction
  and that reaction's random draw; the host still chooses KH1's reaction first and replaces it, so the Powerwild draws
  one extra random number after its own launch (its later choices shift by one draw).

#### The block stance and the spin's repel (the shared enemy-guard step)

- Rows (Checked (files), the asset step reads them from the player's script and motions): the spin's body row
  `0xEC` (type 2, priority 90, recoil 213, HIT slot 1: on in 210 frames 10–46, 211 0–40, 212 0–50) and the stance's
  blade row `0x15C` (type 2, priority 30, recoil 232, HIT slot 0: on in 230 0–12, 231 0–60, 233 0–10); both turn 0,
  cos 0 (the front half).
- The behaviour sends `Cmd::Guard` with the row and its request after each of those plays (`bandit::Guard`); the
  host ends the row when its request changes.
- Against Sora's KH1 swing (type 0; priority = the row's + the Keyblade's, clamped 1..99, the Kingdom Key's 30):
  - the stance (30): with the Kingdom Key both sides get kind 6, difference 0: Sora plays the recoil 0x6E (no push)
    with the Counterattack offered, the Bandit plays 232; its hit handler then runs its counter: the back hop and
    the throw (Checked (files): `0x367A` → `0x4FA`, the stance's flag);
  - the spin (90): Sora gets kind 7 (60 under): the strong recoil 0x6F (in the air 0x70); the Bandit kind 6: 213
    and its counter round.
  - A Keyblade with a higher priority changes the kinds by KH1's rule (`enemy::guard::resolve`).
- Magic is never blocked (KH1: types 5 / 6 meet no type-2 row). Hits from behind land (the front half only).

#### The Bandit's thrown scimitar (a Re:CoM homing bullet moved by KH1's numbers)

- KH1 (Checked (files: `ex_2090_00.bd` from the `.mdls` part 3, disassembled; decomp: its syscalls)): the child
  starts at the Bandit's bone code 28 along its facing at 10 a frame; past 120 frames its speed changes by −0.5 a
  frame (it stops at 140 and comes back); each frame it turns toward its target's lock point 0 by at most 3°
  (`sys 24` → `0x1402A85C0`); 300 frames at most; a hit ends it; a repel turns it back with a random tilt, fades
  it and slides it on (× 0.9 a frame) for 60 frames. Its attack row `0x1CC`: radius 70, no bone. The Bandit lets
  it go at frame 90 of 201 (8 + 82) and waits 290 frames.
- Numbers read by the asset step from the player's files into `kh1-bandit.json` (`blade_*`): life 300, turn-back
  120, speed 10, change −0.5, turn 3°, slide 60, drag 0.9, radius 70; the bone (KH1's code 28 → Re:CoM bone 32,
  `l_hand`: Assumption on the code's bone); Re:CoM's attack id 1.
- Re:CoM (Checked (decomp + disasm + exe)), made as Re:CoM's own enemies make theirs (the Aquatank's
  `0x14018AC50`): descriptor `0x140156790(desc, 2 = CHomingBullet, 3)` (side word `+0xC` left at `0x80000002`: it
  tests the player's side), the factory, the shot manager `0x140158DD0` with the Bandit as its shooter, the mover
  set-up `0x140157DA0`. Its flight state's exe (`STATE_EXE`, the vcall thunk `0x1401560B8`) is ours: KH1's step
  ([`kh1_core::enemy::thrown`]) writes the velocity toward Sora's aim point, then Re:CoM's own step runs. Re:CoM
  draws, moves, collides it, hits Sora with it and ends it (flags `0x100` a hit, 4 the life).
- **Look:** Re:CoM's Aero level-1 shot effect set `0x1405A2030` (entry 24 of `0x1405A1730`; slot 1 `airo_all` id 5,
  name pointer checked at `0x1405A2048` → `0x1405A3150`). Chosen as the nearest Re:CoM look to a spinning blade
  (a small whirl); that the set is loaded in every battle (Sora's spell resources): Assumption, the log says when
  the bullet cannot be made.
- **Power:** Re:CoM's power for the Bandit's attack id 1 (its whirl), through Re:CoM's own enemy-attack getters
  (`0x14017F0E0` AP: `@ENP +4` × difficulty × the record's `+0x10`, `0x14017F050` attribute, `0x140179410` the
  record's kind from ASD record `+0x68`). Mapping: the throw is a spinning blade, nearest the whirl (Assumption;
  owner question).
- **Sora's Guard:** decision 28's turn-back of Re:CoM bullets: the shot moves to the enemies' side and flies back;
  our step then only marks Re:CoM's homing target as the Bandit (`+0x2F4`) and Re:CoM's homing takes it. The Bandit
  takes its damage as Stage 27's turned-back shots do (Assumption: not tested on a KH1 enemy yet). KH1's own repel
  slide after a Guard is not used.
- Per frame: one KH1 flight step per flying blade (at most 8); nothing otherwise.

#### What the asset step writes

`ENEMY/0011/EN0011.ANN` (Re:CoM's 74 slots byte-identical, KH1's clips at 100 + entry), `EN0011.ASD` (Re:CoM's 2
records kept, 6 added: slash 1, spin 1, spin end 2, leap 2), `EN0011.SES` (Re:CoM's 2 entries kept; the attack clips
take their template's sounds at the hit window: Assumption), `kh1-bandit.json` (`kh1-enemy 1`, 117 numbers, the
blade's 12 and the two guard rows). Test `bandit::tests::builds_from_the_players_files`. **Re-run the asset step for
the guard and the blade:** only `kh1-bandit.json` changes (`EN0011.ANN` / `.ASD` / `.SES` are byte-identical); with an
older `kh1-bandit.json` the DLL logs `… (an older asset step? run it again); Re:CoM's behaviour kept` and the
Bandits play Re:CoM's AI. In `mod.yml`:

```yaml
- {name: ENEMY/0011/EN0011.ANN, method: copy, source: [{name: ENEMY/0011/EN0011.ANN}]}
- {name: ENEMY/0011/EN0011.ASD, method: copy, source: [{name: ENEMY/0011/EN0011.ASD}]}
- {name: ENEMY/0011/EN0011.SES, method: copy, source: [{name: ENEMY/0011/EN0011.SES}]}
- {name: kh1-bandit.json, method: copy, source: [{name: kh1-bandit.json}]}
```

The step prints `bandit (M11): …`; the log shows `bandit: KH1's bandit (M11) …` at start, `battle N: Bandit … gets
KH1's AI` in a Bandit battle, and per Bandit `bandit: eNNN: KH1 world Some(8) (Re:CoM world 2)` at its appearance.

#### Checked / Assumption

- **Checked (files):** every number at its offset (117, the test reads them from the player's script), the request
  words, the rows and their recoils (205 / 213, in the model), the hit windows, the hurt parts, the blade's length.
- **Checked (decomp):** the hit handler's order and kind rewrite, the hit counter, movement state 7, the motion loop /
  hold rule, `sys 100` (the cosine), `sys 172`, the attack types and the clash resolution, the bone point, the child
  object's script (below).
- **Checked (exe):** the class, its slots, the ctor, the factory size and call, KH1's world codes; every site (609
  byte sites, 176 pointer sites) against the Steam exe.
- **Checked (exe), the guard and the blade:** Re:CoM's body guard in `0x14016F1C0` (`+0x9C` bit 1, `+0x8C`, `+0xB0`,
  result 2, `+0x9C |= 0x20`), the hit handler's `+0x3D8` call, the bullet set-up as the Aquatank's, the Aero set's
  name, the type table `0x140528CB8`, KH1's recoil numbers (btltbl G+0x1C / +0x30 / +0x34; motions 0x6F / 0x70 from
  `xa_ex_0010.mset`); every site (782, pefile, 2026-11-02).
- **Assumptions:** the leap's power (id 1); the blade's power (id 1), its look, its bone (`l_hand`), the sign of
  its turn (Rodrigues about `dir × to_target`), Re:CoM's aim point for KH1's lock point 0, the Aero set loaded in
  every battle; Re:CoM's other physical attacks as KH1's type 0 for the guard; Re:CoM's battle floors are flat (`sys 89` from where it last stood); the
  apex motion one frame late (as the Powerwild); seeds per enemy; record types 9 / 11 not told apart; Agrabah's
  rules in Re:CoM's Agrabah (decision 21's pairing; owner question 4); Re:CoM's sounds on the new clips.
- **Not ported:** the leash and the fall-out death (KH1 removes it), the group leader's messages, KH1's room rules
  (Agrabah rooms 2 / 3–6 / 9–11 / 17, the sitting Bandits, the rooftop heights), the head look, the effects, the
  blade's part mask (`sys 98` / `99`: the Bandit's held blade stays drawn while the thrown one flies), the fade-in
  (no Re:CoM alpha), flag 0x80 at the appearance.
- **Not built (guard / blade):** Sora's swing knocking the flying blade away (KH1 turns a 0x400 projectile on a swing
  clash; Re:CoM has no attack-vs-attack test for bullets); KH1's repel slide after a Guard (decision 28's turn-back
  instead); walls end the blade (Re:CoM's flag `0x100`; KH1's blade ignores walls); turned guard cones (none here).

#### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`; an Agrabah Bandit battle)

1. Each Bandit plays KH1's appearance (69 twice, about 0.8 s), then wanders about where it stands.
2. Within 15 m (or when Sora targets it, between its walks) it engages: close (< 2 m) back hops and spins or strafes
   and slashes; at 2–4 m it strafes, pauses, then runs in and slashes or leaps; far it runs in, sometimes leaps, and
   sometimes throws its blade: a small Aero-like whirl leaves its hand at about 1.5 s, flies at Sora, slows past 2 s,
   comes back after him, and is gone at 5 s or when it hits.
3. The leap: a crouch, then a dash through the air at Sora about head height that carries on past him, then a drop.
4. Hit it: KH1's flinch / launch (a knock-back hit launches it); its counter: after the first hits of a combo the
   block stance (it raises its blade), a slash or a back hop and the leap.
5. Swing at its block stance from the front (Kingdom Key): Sora bounces off (recoil 0x6E, Counterattack offered),
   the Bandit plays 232, then back hops and throws. From behind the swing lands. A spell lands.
6. Swing into its spin from the front: Sora's strong recoil (0x6F, about a metre back; in the air 0x70, a small hop);
   the Bandit plays 213. Re:CoM's "Guard" popup shows over it (not KH1: owner question).
7. Guard its slash or leap: KH1's recoil 205; its spin: 213. Guard the thrown blade: it flies back and hits the
   Bandit. Let the blade hit you: Re:CoM's damage for the Bandit's whirl.
8. Kill one: KH1's reaction motion, Re:CoM's prizes and EXP.
9. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Bandits (no guard, no blade).

Please send `kh1-recom.log` (`bandit:` lines) and an F9 trace.

#### KH1 trace (what to record in KH1)

An Agrabah room with Bandits (the Main Street's): stand far 10 s; stand at 3 m 10 s; stand at 1.5 m 10 s; one swing;
a full combo; a finisher (its launch); Guard a slash; Guard a spin; let it leap at you and hold still; kill one; and
one fight in Olympus' cups (outside Agrabah: its follow-up rounds). For the guard and the blade: swing at its block
stance and its spin from the front and from behind (once on the ground, once in the air); let a thrown blade fly
past you and come back; Guard a thrown blade; swing at a flying blade.

#### Bandit owner questions

Answered 2026-11-02: 1 (a) the bullet with a Re:CoM effect; 2 (b) the shared enemy-guard step first (both built).
New (the guard and the blade, not answered yet):

5. **Re:CoM's "Guard" popup over a blocking enemy**: Re:CoM's hit handler shows it for every guarded hit (its spark
   and popup kind 3); KH1 shows only the clash. (a) Keep it, (b) hide it for KH1 enemies (skip the popup in the
   guarded path; Re:CoM's own guarding enemies keep theirs).
6. **The block's shape**: Re:CoM guards the hurt bodies' front half; KH1 blocks only swings that touch the guard
   row's blade sphere (40). So a swing at the Bandit's legs or side-front bounces off here but lands in KH1.
   (a) Keep Re:CoM's body guard (as chosen), (b) also test KH1's row sphere against the swing before the guard bit
   is left on (one more test per swing on a guarding enemy, Re:CoM's own sphere test).
7. **Swinging at the flying blade**: in KH1 a swing that meets it knocks it away (a projectile on a swing clash).
   Re:CoM has no attack-vs-bullet test. (a) Leave it out (the blade flies through swings), (b) test Sora's KH1
   blade against the bullet's sphere each frame it flies and turn it back as a Guard does (a per-frame test while
   a blade flies).
8. **The blade's power**: Re:CoM's Bandit has a slash (id 0) and a whirl (id 1). Chosen: the whirl's. (a) Keep, (b)
   the slash's.

The questions as asked (0.72.0):

1. **The thrown blade** (`sys 94` → `ex_2090_00.bd`, row `0x1CC`). Checked (files + decomp): a separate object (`sys 15`;
   that it is the blade's model: Assumption) made at the Bandit's bone code 28, flying along the Bandit's facing at
   10 a frame, turned toward its target's point each frame by `sys 24(…, 3)` (a 3° step: Assumption), from frame 120
   slowing by 0.5 a frame (so it comes back), 300 frames at most; it ends on a hit (its object flags `& 0x45`), and
   when repelled (`& 0x102` / `& 0x30`) it turns, drops and stops after 60 frames. The Bandit waits 290 frames for it. Native options: (a) one of Re:CoM's own bullets
   (`CHomingBullet`), moved and timed by those numbers each frame, with a Re:CoM effect as its look (the Gargoyle
   rule); (b) the same bullet with KH1's scimitar model converted at the asset step as its look; (c) leave it out for
   now (today: the throw motion plays, nothing flies).
2. **The block** (the stance's row `0x15C`) and **the spin's repel** (row `0xEC`): in KH1 Sora's swing that meets them
   is deflected (Sora's clash recoil), the Bandit plays 232 / 213 and counters (after a block: back hop and throw).
   Native route found: Re:CoM's `CCollBody` guard flag (`+0x9C` bit 1, which Re:CoM's own hit apply honours; Sora's
   Guard uses it) on its hurt bodies while those rows live, then KH1's clash on both sides. This changes how fights
   play (Sora's swings bounce off) and the same mechanic serves the Large Body and the Defender. (a) Build it for the
   Bandit now, (b) build it as a shared enemy-guard step first, (c) leave it out (today: hits land during the stance
   and the spin).
3. **The leap's power**: Re:CoM's Bandit has a slash (id 0) and a whirl (id 1). Chosen: the leap takes the whirl's.
   (a) Keep, (b) the slash's.
4. **Agrabah's rules**: KH1's Bandit plays differently by world (Agrabah: far rounds throw or leap, no follow-up after
   its counter; elsewhere far rounds only close in and every counter gets a follow-up). Built: Re:CoM's world mapped
   as decision 21 maps the records (Re:CoM's Agrabah = KH1's Agrabah). (a) Keep, (b) Agrabah's rules everywhere,
   (c) the other rules everywhere.

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
| Bandit (built) | `xa_ex_2090` (ex_2090.bd) | `CEnemyBandit` 11 | `0x140534368` | `0x14018aeb0` / `0x140189151` 0xcd0 | 1,2,3 | 200,202,210,211,212,216,217,218,230..233 | FaALl | — |
| Fat Bandit | `xa_ex_2060` (ex_2060.bd) | `CEnemyFatbandit` 10 | `0x14053ddf8` | `0x14018dd70` / `0x140189127` 0xcd0 | 1,2,3 | 200,205,206,207 | FKaALl | — |
| Barrel Spider | `xa_ex_2170` (ex_2170.bd) | `CEnemyBarrelspider` 28 | `0x140535688` | `0x14018b3b0` / `0x14018941b` 0xce0 | 1,2,3,19 | 205,207,211,213,214 | FKaA | — |
| Search Ghost | `xa_ex_2230` (ex_2230.bd) | `CEnemySearchghost` 14 | `0x14054c408` | `0x1401923f0` / `0x1401891cf` 0xce0 | 1,2,3 | 203,204 | FKaA | — |
| Pirate | `xa_ex_2100` (ex_2100.bd) | `CEnemyPirate` 21 | `0x140546458` | `0x140190f30` / `0x1401892f5` 0xcd0 | 1,2,3 | 208,210,214,216,217,218,235,236 | FaLl | — |
| Air Pirate (built) | `xa_ex_2210` (ex_2210.bd) | `CEnemyAirpirate` 22 | `0x1405309e8` | `0x14018a0b0` / `0x14018931f` 0xcd0 | 1,2,3, think `0x14018a190` | 200,212 | FKaA | — |
| Wight Knight | `xa_ex_2200` (ex_2200.bd) | `CEnemyWightnight` 12 | `0x140553748` | `0x140194ed0` / `0x14018917b` 0xcd0 | 1,2,3 | 200,201,202,211,216 | FaALl | — |
| Gargoyle | `xa_ex_2220` (ex_2220.bd) | `CEnemyGargoyle` 13 | `0x14053f198` | `0x14018e150` / `0x1401891a5` 0xcd0 | 1,2,3 | 203,212,218 | FKA | — |
| Darkball (built) | `xa_ex_2280` (ex_2280.bd) | `CEnemyDarkball` 23 | `0x14053b7a8` | `0x14018d5d0` / `0x140189349` 0xcd0 | 1,2,3 | 200,202,204 | FKaA | — |
| Defender | `xa_ex_2340` (ex_2340.bd) | `CEnemyDefender` 26 | `0x14053ca98` | `0x14018d8e0` / `0x1401893c7` 0xcd0 | 1,2,3 | 207,208,209 | FKa | — |
| Wizard | `xa_ex_2150` (ex_2150.bd) | `CEnemyWizard` 25 | `0x140554a58` | `0x140195420` / `0x14018939d` 0xcd0 | 1,2,3,4 | — | FKaA | SES |
| Sea Neon | `xa_ex_2070` (ex_2070.bd) | `CEnemySeaneon` 18 | `0x14054b0e8` | `0x1401920c0` / `0x140189277` 0xce0 | 1,2,3 | 201,204 | FKaA | — |
| Aquatank | `xa_ex_2080` (ex_2080.bd) | `CEnemyAquatank` 20 | `0x140532fe8` | `0x14018a960` / `0x1401892cb` 0xcd0 | 1,2,3 | 202,203 | FKaA | — |
| Screwdiver | `xa_ex_2240` (ex_2240.bd) | `CEnemyScrewdiver` 19 | `0x140549e18` | `0x140191d90` / `0x1401892a1` 0xcd0 | 1,2,3 | 203,205 | FK | — |
| Wyvern (built) | `xa_ex_2320` (ex_2320.bd) | `CEnemyWyvern` 24 | `0x140555dc8` | `0x140195ed0` / `0x140189373` 0xcf0 | 1,2,3 | 200,202,203,205,206 | FKaA | — |
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
8. **The Gargoyle's child object** (`ex_2220_00.bd`, a moving hit made with its attack 219): (a) a Re:CoM bullet
   (`CBullet` / `CHomingBullet`, as KH1 magic's shots) moved by KH1's child script's numbers, (b) the hit
   kept on the Gargoyle's own body as an attack record (only if the child turns out to stay on the parent),
   (c) leave the Gargoyle on Re:CoM's behaviour for now. To be chosen once its movement syscalls are read.

The Darkball's questions (M11, not answered yet):

1. **The Darkball's attack power**: Re:CoM's Darkball has its dash (id 0) and its bite (id 1), whose clips are
   bakes of KH1's dash 202 and bite 204: those take them. KH1's drop (the reappearance's 2.3 m slam, 100 % like the
   bite) has no Re:CoM twin. Chosen: the bite's (id 1). (a) Keep, (b) the dash's, (c) another rule.
2. **The warp's effects**: KH1 shows dark smoke as it vanishes and reappears and a trail while it is invisible
   (`sys 74` / `sys 21`); the port shows the shrink and the growth only (no effects are ported for any enemy
   yet). (a) Leave it for the effects milestone, (b) use one of Re:CoM's own effects (which: to research), (c)
   research KH1's effect ids first.
3. **Lock-on while it is invisible**: Re:CoM's lock-on can stay on (or pick) a Darkball that has vanished; KH1's
   automatic target skips it (flag 0x40), its lock button may not (M10's Shadow kept Re:CoM's lockable flag). (a)
   Keep (as the Shadow), (b) clear Re:CoM's lockable flag (`+0x154` bit 0x10000) while it is hidden.
4. **Gravity on a Darkball**: KH1's Darkball answers a Gravity hit with a 3-second stagger (205); the host doesn't
   carry KH1's hit attribute, so it never plays. (a) Leave it, (b) pass our spells' KH1 attribute to the enemies'
   behaviours (a framework change; Re:CoM's own Gravity at Damage = Re:CoM stays Re:CoM's flatten).

The Air Pirate's questions (M11, not answered yet):

1. **The Air Pirate's attack power**: Re:CoM's Air Pirate has two sword attacks, id 0 on `e022_050` (an edit of KH1's
   slash 212) and id 1 on `e022_051` (no KH1 twin). Chosen: the slash on id 0, the swoop 200 on id 1. (a) Keep, (b) both
   on id 0, (c) another rule.
2. **Two world-height tests**: KH1's script compares two world heights (the close round climbs when it is below
   y −170; the approach aims at Sora himself when he is in the air above y −100), written for KH1 rooms whose floor is
   at y = 0. Built: heights above the battle floor (Sora's last standing height). (a) Keep, (b) world heights as KH1's
   script has them (Re:CoM's battle floors are not at 0, so the tests would always or never pass).
