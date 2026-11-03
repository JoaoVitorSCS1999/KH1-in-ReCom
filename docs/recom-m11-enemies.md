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
    from its engaged state (`QUIET_LIMIT`, `Behaviour::reset`). KH1's longest silent wait was 180 frames until the
    Search Ghost: its idle search (`ex_2230.bd` `0x1044`) is silent for up to 600 frames, so the limit is 900 since
    its port (test `the_idle_search_is_the_longest_silence`: worst 450–600 frames).

### Steered attacks released on their first step (owner 2026-11-02, 0.73.0: "bandits dont spin when they do the move where they're supposed to spin towards you")

- **What happened** (Checked (log + code)). The owner's 0.73.0 log has 157 lines `bandit: eNNN: action 217 never
  ended (its Re:CoM attack state was left without its end; now Re:CoM state Some(3)), released`, one every
  other frame, each followed by `attack 217 (leap), steered`; the slash 200 shows the same in state 2. No spin
  (210 / 211) was played in that log, so the move the owner means is the leap (216 crouch, 217 flight at Sora).
  Assumption: the owner means the leap; the spin had the same fault.
- **Cause.** The safety net above ("an attack still playing when the AI is polled in state 0 / 1") was
  written before steered attacks (Stage 128). It tested only "polled and playing", but a steered attack is
  polled from its own attack state's step. So every steered attack of every class (Air Soldier, Darkball, Air
  Pirate, Wyvern, Bandit) was released on its first step since 0.69.0. The idle played, the AI asked for the
  attack again, Re:CoM's state machine (`0x140242F50`: the old state's end, the new one's begin, then its step,
  in the next update; Checked (decomp)) began it again, and the next step released it. The leap's 217
  restarted from frame 0 every other frame: no spin on screen.
- **Fix.** The host's rules are now pure functions in `kh1_core::enemy::host`, called by `win::kh1_enemy`:
  - `attack_state_left`: the net fires only when Re:CoM's state is not the attack's own state;
  - `after_poll`: what a steered attack's state does after the AI ran in it (unchanged rules).
- **Also: one start per `sys 12`.** The Bandit's leap asks for 217 before its flight loop and again in the
  loop's first frame. Both saw the host's 216, so 0.73.0 asked twice in one frame. `attack_keep` now takes the
  entry this frame already asked for first, as KH1 sets the entry at once (`0x14029F7C0`). Checked (decomp).
- **Tests** (`kh1_core::enemy::host::model::Model`: Re:CoM's state 1 and attack states as `win::kh1_enemy`
  runs them, through the same rules):
  - the Bandit's leap plays 217 once, unbroken through its flight (past one 20-frame cycle), in state 3;
  - the spin plays 211 once for its four 38-frame rounds, gliding at 4 a frame, then 212;
  - a 20 000-frame fight, 12 seeds: nothing released, no attack asked for again within two frames.
  - `kh1-recom`: every class through the model with its own attack-state map and the asset step's manifest
    (`KH1_<NAME>_JSON`): nothing released, no re-ask within two frames. Run on the session's files for all
    seven manifests.

  The old rule fails the first three (checked by reverting it), and the old `attack_keep` fails two.
- **Asset step:** no change.

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
  with the higher priority **repels** instead (`0x1402BF630(other attack, 0, its owner)`): its own owner gets **the
  other attack's hit record, flagged 0x20**, and its row is marked met (`+4 |= 0x10`); the other side still runs its
  own resolve and gets its kind 6 / 7. A 0x20 record does no damage (`0x1402BEEE0` returns at once) and plays no
  reaction (`0x1402B8E10` skips `& 0x22`), but the owner's script hit handler hears it (`0x1402B7B90` →
  `0x1402BCD60`) and KH1's hit counter counts it (`0x1402BFE60`). Checked (decomp). A type-2 side never repels. A
  projectile side (`+0x30 & 0x400`) is turned to the other side instead of getting a record (not built: see below).
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
  sets `+0x9C |= 0x20`, and the enemy hit handler plays Re:CoM's guard spark, the "Guard" popup (kind 3; skipped for
  ours, below) and calls
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
| Enemy's clash | `win/kh1_enemy.rs` `guard_event` | by the enemy's side (`guard_after_swing` keeps it): **a clash**: the row's recoil and kind kept as `clash_recoil`; the row ends; the attack ends into our state 11 (or state 11 is next), which plays KH1's clash reaction (`react::clash`) and tells the script (its hit handler); **a repel** (built 2026-11-02): no recoil, no reaction, its attack goes on, the hit counter + 1, the script hears `HitNews::Repelled` (and what its handler does in the handler runs now, `Behaviour::take_now`); the row that met is done (KH1's `+4 |= 0x10`): a plain row ends, a merged one stays on (Assumption below); **neither** (a side at 100, both 98+): nothing |
| KH1's front cone | `win/kh1_enemy.rs` `kh1_front` | KH1's 3D test (`0x1402B4640`: the enemy's facing turned by `+0x60`, the vector to Sora, cos ≥ `+0x64`) before each KH1 swing's apply: outside it the swing is a plain hit (as when the types don't meet). Re:CoM's `+0xB0` half angle stays for Re:CoM's own attacks. For a cos-0 cone (the Bandit's) the two tests agree (the facing has no height); for the Search Ghost's cos 1.0 only KH1's 3D test is right |
| Merged rows | `kh1-recom-assets` `enemy.rs` `GuardSpec::also` | rows made with the same request that differ only in their sphere and HIT slot (the Search Ghost's four) are one guard row: their clash numbers must match; their slots' windows are joined. The manifest lists them (`"also"`, `"slots"`); the Bandit's manifest is unchanged |

Per frame: nothing while no KH1 enemy is guarding (`GUARDED` = 0); one window test per guarding enemy; the swing
wrapper's check only when a KH1 swing meets a body.

**Differences from KH1 (known, kept):**

- Shape: KH1 tests the guard row's own sphere (the Bandit's blade, 40); Re:CoM tests the enemy's hurt bodies. A
  swing that reaches the body's front while the row is on is blocked even when it would have missed the blade
  (the owner chose Re:CoM's body flag). Owner question below.
- Angle: equal for cones without a turn (`turn` = 0; KH1's cos ≥ `c` = Re:CoM's ±acos(`c`)). A turned cone has no
  Re:CoM field: refused with a log line (`guard row … not built`). None of the Bandit's.
- Re:CoM's "Guard" popup: hidden over KH1 enemies (owner 2026-11-02, on 0.75.2: "hide Re:CoM's "Guard" popup over
  KH1 enemies"). The hit handler's guarded branch calls `0x14020C9A0(handle, 3)` at `0x14017FCE7` (`mov ecx,
  [rsi+0x20]; lea edx,[rbp+3]; call`), between the guard spark and the `+0x3D8` guard event. Checked (decomp + exe).
  That call is retargeted once at install, through a 14-byte stub near it, to `guard_popup`, which skips kind 3
  when the handle's object is one of ours. Everything else calls Re:CoM's popup as before (Re:CoM's enemies,
  Sora's popups, the other kinds). The spark, the clink and the guard event are unchanged. Sites
  `M11_GUARD_POPUP_CALLS` / `_CHECKED`; no branch lands inside the call (exe scan). Nothing runs per frame.
  Logged as `<class>: eNNN: Re:CoM's Guard popup skipped`.
- Elements: the guard keeps out the physical element only; KH1's type rule decides for Sora's swings. Our spells
  carry Re:CoM's spell attributes (KH1's types 5 / 6 meet no type-2 row, so they land as in KH1). Assumption:
  Re:CoM's other physical attacks (a turned-back shot, a summon) count as KH1's type 0.

**For the Large Body and the Defender:** (the Large Body is built, § The Large Body: its front row is type 1 at
priority 100, made with KH1's mode 3: a side at 100 is never moved, so Sora gets kind 7 and it gets nothing; no repel.) Their rows are types 1 / 4 with priorities above Sora's: KH1 repels. The repel
is built (2026-11-02, first used by the Search Ghost's burst rows): Sora gets his kind 6 / 7 from his own side, the enemy
the 0x20 record (`HitNews::Repelled`: its script's hit handler decides; no damage, no reaction, its attack goes on).
Their ports add their rows to `GUARDS` and send `Cmd::Guard`; their scripts' hit handlers must treat `Repelled` as KH1's
0x20 record (a hit record whose kind is the attacker's, which does no damage). Still to research: a **turned cone** (a
shield held to one side, row `+0x60` ≠ 0): `kh1_front` already tests it the KH1 way, but Re:CoM's body guard flag has
no facing offset (`+0xB0` is a half angle about the facing), so the host refuses such a row today (`guard row … has a
turned cone`); building it means setting Re:CoM's half angle to π (all round) and letting `kh1_front` decide for Sora's
swings, then deciding what Re:CoM's own attacks see. Assumption (merged rows and the repel): after a repel a merged row
stays on, since KH1's other row instances (the other hand, the other strike) could still meet a later swing, which
Re:CoM's whole-body guard can't tell apart.

### Hits that take no reaction (KH1's `sys 42` on Re:CoM's own super armour; M11, the Pirate)

KH1's Pirate keeps attacking when it is hit during its attacks: its hit handler (`ex_2100.bd` `0x2A3C`) calls
`sys 42` instead of asking for its hit state while one of its attack motions plays (`0x267E`) or while it lies
after a launch (`sys 70`).

**KH1. Checked (decomp + exe):**

- `sys 42` = `0x1402BBD80`: an effect on a hurt part when the record's `+0xC & 0x800` (`0x1402B8790`, a look: M13),
  then `0x1402B7E60`: the push slide `0x1402C7300` along the record's push direction `+0x10` at `G+0x2C` × the
  stats block's `+0xC4` (the flinch's push, `react`'s `flinch_push × weight`), and `*hit |= 2`: KH1's reaction
  (`0x1402B8E10`) then plays nothing, as after a scripted launch.
- `sys 70` = `0x1402BBF00` → `0x1402B7EE0`: `u = +0x164 − +0x3EA` (the request playing minus the last launch's base,
  a u16 written only by the launches `0x1402B79F0` / `0x1402B8530`); true when `u < 10` and `u % 5 == 3` (the lie
  requests base + 3 / + 8). Before any launch `+0x3EA` is 0 (Assumption: the actor's memory starts zeroed), so the
  test also holds for requests 3 and 8.

**Re:CoM. Checked (decomp):** its hit handler `0x14017F7E0` applies the damage, calls vtable `+0x3E0` and then its
reaction picker `0x14017D260`. With `+0x260` bit 20 the blow-away and knock kinds (2..6, 0x12, 0x1D, 0x1E) take the
flinch's path `0x14017D800`, as kind 1 does; with bit 21 that path plays only the damage voice (`0x140181B50` /
`0x140181A50`) and asks for no state (`+0x68` untouched). The status kinds keep their states (Stop 8 / 0x17, stun 7,
Gravity's flatten 0xD, freeze 0x11, 0xB / 0xC / 0xE..0x10 / 0x14 / 0x16 / 0x1F: decision M10's rule), and a killing
hit still goes to the defeat (`0x140180A80`). A KH1 hit on an attacking enemy carries the card-interrupt bit, so the
handler's `0x2000001` path ends the "card" first (`0x14017E1D0`, `+0x260 &= ~1`; nothing for our cardless attacks);
the attack state itself goes on.

**Built (no new hook, no new site; nothing per frame):**

| Part | Where | What |
|---|---|---|
| Behaviour | `kh1-core` `enemy/mod.rs` | `Behaviour::armoured(playing, launch_base, kind)` (KH1's `+0x164`, `+0x3EA`, the hit's KH1 kind; default false), `on_armoured_hit(dir)` (the `sys 42` path), `set_hit_dir(dir)` (the record's `+0x10`, for handlers that read it) |
| Rules | `kh1-core` `enemy/host.rs` | `RECOM_ARMOUR` (bits 20 / 21), `armour_flags`, `restore_armour` (the two bits back as they were), `armour_took` (the handler answered 1, the enemy lives, `+0x68` unchanged) |
| Host | `kh1-recom` `win/kh1_enemy.rs` | `armour_on` / `armour_off` around the existing enemy-hit wrapper's call (`win::attack::enemy_hit`, which every call of `0x14017F7E0` goes through): the bits on for the call when the behaviour says so, put back after; a hit the armour kept goes to `on_armoured_hit` with KH1's hit counter and a log line; the runtime keeps `launch_base` (each launch's base, `start_reaction`) |

Per hit: one atomic load while no KH1 class is installed, else one tail check and the behaviour's answer. The
other classes' behaviours answer false: nothing changes for them.

**The hit's kind before the call (owner 2026-11-02, Pirate question 3 (b): the fliers' "doesn't flinch" on this
armour).** The Darkball's and the dives' decisions need the KH1 kind of the hit about to be handled.

- Re:CoM (Checked (decomp)): the handler's first call, its gather `0x140154EB0`, walks the hurt bodies in order
  and picks the first with the repel bit (`+0x9C & 8`), the guarded bit (`& 0x20`; both: its guard branch, no
  picker, returns 0) or a hit count (`+0x90`); the hit apply `0x14016F1C0` writes the count, the attack's Re:CoM
  kind (`+0x80`, from the attack's `+0x44`) and the owner (`+0x98`) when it applies, before the handler. So the
  hit info `+0xF8` → `+0x50` is stale before the call (the gather writes it), but the bodies already describe
  this hit.
- Built: `kh1_core::enemy::host::gather` (the gather's order, a pure rule) and `win::attack::incoming`: read
  before the call, nothing taken. A KH1 swing's kind is the kind of Sora's row (the same value the wrapper gives
  `note_swing` after the call: `SWING` is noted only after the handler, so a peek of `SWING` itself would read
  the previous hit's); otherwise the waiting body's Re:CoM kind mapped (`kh1_kind_of_recom`, as `hit_of` reads it
  after the call): Re:CoM attackers and our spells too.
- `Behaviour::armoured(playing, launch_base, kind)` is asked only while a damaging hit waits; `kind` is None only
  when no kind could be read. The Pirate ignores it (its handler's clash test comes first, but no damaging hit
  through Re:CoM's handler carries kind 6 / 7: Sora's rows are kinds 0 / 3 / 5 / 8, Re:CoM's map to 3 / 5;
  Checked (code)); its tests are unchanged in meaning.
- **Clashes are never armoured.** A clash reaches our state 11 two ways: Sora's Guard block (`after_enemy_hit`
  → `guard_block`, which runs *inside* the wrapped call) and the enemy's guard row meeting a swing (the
  handler's guard branch → `guard_event`, no picker, so no armour is even asked for: no damaging hit waits). `guard_block` with a KH1 recoil ends the attack itself
  (no picker, unaffected); without one it hands the block to Re:CoM's picker (`recom_flinch`), which bit 21 would
  turn into the damage voice only. So the armour stays off for a call whose enemy has a Guard block pending
  (`win::guard::block_pending`, nothing taken); and the fliers answer false for kinds 6 / 7.
- `keeps_reaction` stays as the fallback for a hit the armour didn't take (no kind read, or a Guard block in the
  same frame): Re:CoM has then already asked for its flinch and cut the attack; `react_begin` hands back to
  state 1 as before.
- Per call: the tail check, then a scan of the hit bodies (≤ 32 reads, no lock) before the behaviour is asked
  (the old per-call lock now only while a hit waits). No new hook, no new site.

Found, not changed (owner question in § The Pirate): the Wyvern's attribute 2 / 3 / 4 hits call the same `sys 42`
(§ The Wyvern: "the engine's element reaction", left out); that is this no-reaction path, so KH1's Wyvern takes no
reaction from those hits. The Darkball's, the Air Pirate's and the Wyvern's "doesn't flinch" cases zeroed the reaction
kind (`keeps_reaction`) after Re:CoM's handler had already cut the attack; they now take this armour (owner
2026-11-02, below).

### Attack meets attack (clashes)

Owner (2026-11-02, Large Body choices): "build them into the shared enemy guard for every ported enemy, researching
KH1's order of tests first". Status: built on branch `clash-wip`; tests and the Windows build pass; **not seen in
game**; no KH1 trace of a clash yet. The asset step must be run again (the manifests gain each attack row's clash
numbers; the `.ANN` / `.ASD` / `.SES` files are byte-identical).

**KH1's order of tests. Checked (decomp), Steam addresses:**

- One collision pass a frame, `0x1402B4460`. First the **attack pass** `0x1402B47A0`: for each live attack object
  (64 slots of 0xC0 at `0x142DB4710`), `0x1402B3800` → `0x1402B3D50` lays its spheres along its swept bone line into one
  list, and **each new sphere is tested at once against every sphere already in the list** (`0x1402B3BE0`). Only after
  every attack is in the list comes the **body pass**: each actor's hurt parts (`0x1402B5E50`) against the whole list
  (`0x1402B3BE0` again; a hit → `0x1402B4060`, the damage record).
- `0x1402B3BE0`, for a pair: other side (`+0` bits 12..13), other owner, the attack not freed (`+0` bit 21) and not
  off (bit 10). **Against a hurt body**: the attack not clashed this frame (`+4 & 0x30` = 0), and not a guard type 1, 2
  or 4 (type 3 still hurts); the owner's hit-once mask (`actor+0x3E0`). **Against another attack**: they have not met
  before (their met masks `+0x54` / `+0x9C`, set both ways on a meeting, kept while both live) and their types meet
  (`0x140528CB8`). Then the shapes (`0x1402B5EF0`) and `0x1402B4A20`.
- `0x1402B4A20(a, b)`: each guard-type side's front cone (`0x1402B4640`, both orders; types 0, 5, 6, 7 always pass),
  then `0x1402B4BC0` both ways, returning 0, 0x10 or 0x20 per side:
  - 0 when that side already clashed this frame, when the other's priority is 0, or when both are 98 or more;
  - 0x10 with no record and **no mark** for a side at **100** (it is never moved), or at 98 / 99 against a side under 98;
  - a **repel** (0x10, marked) for a type 1 / 3 / 4 side above the other (the other side's record flagged 0x20 goes to
    its owner: no damage, no reaction, its hit handler hears it);
  - else a **clash record** on that side's owner (`0x1402BF060`): kind 6, or **7 when 40 or more under**, `+8` = its
    priority minus the other's, `+0x2C` its own row (whose `+0x40` is the recoil it plays); 0x10 / 0x20, marked.
  - A side that returned 0 is marked by the other side's non-zero return. A marked attack (`+4 |= 0x10 / 0x20`) meets
    **no hurt body and gets no other record for the rest of the frame**; `+4` is cleared each frame (`0x1402B47A0`), and
    the persistent mark goes to the script's hit word (`+0x60`). (Correction to § Enemy guard: a clashed row is not
    shut for good; it ends because its owner's reaction changes the request.)
  - Effects: each side's own hit effect when it returned non-zero (`0x1402B4400` → `0x1402B4320`), else a generic one
    at the midpoint of the two spheres (`0x1402AE650`, 10 / 11). A hit-stop and a pad rumble when the other attack is
    the player's (`0x14028C660(10)`, `0x140270990`).
- **Damage**: a clash record does none (`0x1402BEEE0` returns the HP for kinds 6 / 7 and for 0x20 records); and
  since the attack pass comes first, a swing and a row that clash in a frame both miss every body in that frame. The
  next frame the reactions (Sora `0x14029BBD0`, the enemy `0x1402B8E10` cases 6 / 7 → `0x1402B9140`) change both
  requests and their rows end. A side at 100 (or 98 / 99 against a lower one) is not marked: it still hits that frame.
- **Who recoils**: the lower side gets kind 6 (7 when 40+ under); equal priorities both get kind 6 (difference 0); a
  side at 100 never; a type 0 row never repels. **Sora** plays 0x6E (kind 6, pushed back −diff × 2.5 when under), 0x6F
  (kind 7) or 0x70 (in the air), as for a guard row (§ Enemy guard). **The enemy** plays its row's recoil (`+0x40`),
  its script's hit handler hearing the clash; KH1's reaction step plays only the strongest clash of a frame (`uVar6 <
  uVar5`).
- **Body spheres count**: a row is a row whatever its bone. The Large Body's charge belly `0x23C` (type 0, priority 90)
  meets Sora's swing like a fist.

**Each class's attack rows (Checked (files): the players' scripts, `+0x38 & 7`, `+0x3C`, `+0x40`, `+0x64`; the asset
step reads them):**

| Class | Rows: type / priority | Against the Kingdom Key's 30 |
|---|---|---|
| Shadow | claw, swipe, leap claw: 0 / 40 | both recoil (Sora kind 6, pushed 25; it 207 / 208) |
| Soldier | kicks, double kick, rush, air kick: 0 / 20; jumping kick, rush leap, spin-kick foot: 0 / 100; spin-kick chest `0x2AC`: **1 / 100, cone ±120°** | 20: both kind 6 (it is pushed); 100: Sora's strong recoil, it goes on |
| Air Soldier | slash 0 / 20; swoop kick, swoop 0 / 30; spin, charge 0 / 50 | both recoil |
| Powerwild | swipe 0 / 50; flurry hands, pounce 0 / 10; flurry body `0x1CC`: **2 / 10, front half** | both recoil |
| Darkball | dash 0 / 40; bite, drop 0 / 100 | dash: both; bite / drop: Sora's strong recoil |
| Bandit | slash, spin, spin end, leap (blade and body): 0 / 30 | both kind 6, no push |
| Pirate | every row 0 / 90 | Sora's strong recoil, it kind 6 (its recoil) |
| Air Pirate | swoop 0 / 50; slash 0 / 100 | swoop: both; slash: Sora's strong recoil |
| Wyvern | bite, dive, landing 0 / 100; charge `0x1CC`: 3 / 100, all round | Sora's strong recoil; it never moves |
| Search Ghost | burst 0 / 40 (hit A), 0 / 70 (hit B); drain 0 / 40 | both recoil |
| Large Body | punch (both fists), slam 0 / 30; charge 0 / 90; the stomp's ring: the child's row, type 6 | punch / slam: both kind 6; charge: Sora's strong recoil, it kind 6 (its handler ends the charge: 218, then 209); ring: **never** (type 6 meets no swing) |
| Neoshadow | claw 0 / 40; rise slash, dive 0 / 100 | claw: both; the others: Sora's strong recoil |

Every class has rows that clash; the only rows that can't are the Large Body's stomp ring (type 6) and the Bandit's
thrown blade (built before, § The Bandit's thrown scimitar). A Keyblade of priority 98+ makes Sora's side move only for
an enemy under 98.

**Re:CoM's side. Checked (decomp + exe):** Re:CoM has no attack-against-attack test: its collision step for an attack
(`0x14016F110`) tests it only against a body list (`*0x14087A838` player side / `*0x14087A840` enemies) through the
pair dispatch `0x14016EAD0`. An enemy's attacks are its attack set (`enemy+0x100`, `CAttackMotion`, built from its
`.ASD` by `0x14014DBE0`): each live entry (`+0x18` the `.ASD` record, `0x14014E010`; `+0x14` its window state, 1 = on,
`0x14014E830`; `+0x20` its collision, `+0x10` on / off by `0x14014DE90` / `0x14014DE80`, `+0x18` its shape: a sphere
`CSphereCollision` type 2, 0x80 bytes, or a line `CPoly3Collision` type 7, 0x90 bytes, `0x1401720E0`). The enemy update
`0x140185630` tests its attacks (`0x14014E6A0`) and then runs its hit handler `0x14017F7E0`.

**Built (no new hook; one new callee; nothing per frame outside a KH1 swing's hit window):**

| Part | Where | What |
|---|---|---|
| KH1's rule | `kh1-core` `enemy/guard.rs` | `met` (`0x1402B4BC0` with its return: `Met::{Nothing, Unmoved, Repels, Clash}`), `meeting` (both orders and the marks of `0x1402B4A20`), `front` (`0x1402B4640` by type) |
| The swing's frame | `kh1-recom` `kh1_enemy.rs` | `sweep`: one KH1 swing against the live rows in order: Sora keeps his first record, a marked swing gets no other, each row's record; `stronger` (KH1 plays a frame's strongest clash); `ClashRow`, `record_index`, `Manifest::clash_of` |
| Asset step | `kh1-recom-assets` `enemy.rs`, `shadow.rs` | each attack row's type, priority and cone (`attack_row`), and the `.ASD` records made from it (`extend_asd` returns their indices): the manifests' per-attack `"clash"` (the Shadow's top-level `"clash"` with each row's attack) |
| Live rows | `win/kh1_enemy.rs` `live_rows` | this battle's KH1 enemies' attack-set entries that are on, whose record the manifest names (Re:CoM's own records: none), with a sphere or line shape |
| The test | `win/attack.rs` `pair_dispatch` → `swing_meets_attacks` | in Sora's KH1 swing test (the existing wrapper), **before the hurt bodies** (KH1's attack pass first): the swing's spheres against each live row's own Re:CoM shape with Re:CoM's overlap tests (`0x1401777E0` for spheres, `0x140177480` for lines, the pair dispatch's cases 0x22 / 0x72, on a copy of the enemy's shape so Re:CoM's own stays untouched); KH1's types, the row's cone (guard types), the met masks (once per swing and row) |
| Sora's clash | `win/attack.rs` | his record into `SWING_CLASH` → `Kh1Sora::swing_clash` (0x6E / 0x6F / 0x70, push, combo counter, Counterattack from frame 12 of 0x6E), as for a guard row; a marked swing applies to no hurt body for the rest of that frame |
| The enemy's clash | `win/kh1_enemy.rs` `attack_clash_watch` (enemy-hit wrapper, before Re:CoM's handler) | a marked row's collision answers 0 in Re:CoM's test against Sora until then (`suppressed`); then its record plays: a clash = its row's recoil through our state 11 (Re:CoM's attack end with state 11 next, `clash_recoil`, the script hears `HitNews::Clash`), Re:CoM's flinch when the model lacks that recoil (the owner's rule for a guarded attack); a repel = `HitNews::Repelled` (no damage, no reaction); dropped in a Re:CoM state (Stop, stun) or when dead |
| Look | `win/attack.rs` | Re:CoM's guard spark and clink at the midpoint of the swing sphere and the row (KH1's midpoint) |
| Start log | `win/kh1_enemy.rs` | `<class>: … N attack rows meet Sora's swings (priorities …)`, or that the manifest is an older one |

Re:CoM's own enemies (`enemy_ai = recom`), Combat = Re:CoM and Riku's story: nothing runs (only our enemies' runtimes
have rows). Sora's Guard, card plays and Re:CoM's attacks are untouched.

Per frame: two atomic loads per enemy update and one per Re:CoM pair test while no clash is pending; during a KH1
swing's hit window, one walk of this battle's KH1 enemies' attack sets (at most 12 entries each) and Re:CoM's overlap
test for each live row. Not measured in game (Assumption: small next to Re:CoM's own collision step, which tests every
attack against every body each frame).

**Checked / Assumption:**

- Checked (decomp): the order and rules above; Re:CoM's attack set, entry, collision and shape layouts; the pair
  dispatch's cases; the enemy update's order. Checked (files): every class's row numbers (the asset test reads them from
  the players' scripts; each record a row names is one of the step's, on the attack's clip). Checked (exe): the new
  callee's first 16 bytes. Tests: `guard::attack_meets_attack`, `kh1_enemy::a_swing_frame_against_enemy_rows`,
  `clash_rows_parse_and_look_up`, `enemy::clash_tests` (players' files), the Large Body's real manifest.
- Assumption: Re:CoM's order of Sora's weapon test and the enemies' attack tests in a frame (an enemy updated before
  Sora's test this frame has already tested its row against Sora; KH1 tests attacks against attacks first); the enemy's
  shape positions as Re:CoM last set them (at most a frame old); Re:CoM's guard spark for KH1's clash effects; no
  hit-stop or rumble; the special-state test (`+0x88 & 1`) not modelled; the row's push bit (`+0x40` bit 22) not carried
  (as for a guard row).
- Not KH1's (found, owner question 2): the Soldier's spin-kick chest (type 1) and the Powerwild's flurry body (type 2)
  are guard types that never hurt in KH1, but the asset step made Re:CoM attack records of them, so they hurt Sora here
  (since 0.68.0 / 0.70.0). They now also clash as KH1's rows do.

**Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`; run the asset step first):**

1. Large Body: swing into its fist as it punches: both bounce (Sora's light recoil, it plays its punch recoil), no
   damage either way. Swing into its belly while it charges: Sora's strong recoil, the charge stops (its recoil, then
   its recovery). From behind while it charges: the swing lands (its back isn't a row).
2. Soldier: meet a kick with a swing: both bounce, it is pushed a little. Meet its spin kick or jumping kick: Sora
   bounces hard, the kick goes on and can still hit him.
3. Pirate: meet any slash with a swing: Sora bounces hard, the Pirate plays its recoil.
4. Wyvern / Neoshadow dive / Darkball bite: Sora bounces hard; they never stop.
5. Shadow, Bandit, Air Soldier, Powerwild, Search Ghost, Air Pirate's swoop: meeting their attacks makes both recoil.
6. The swing that bounced does no damage that frame, even when it also touched the body.
7. Magic never clashes (Fire passes through a fist and hits).
8. Re:CoM enemies, `enemy_ai = recom`, Combat = Re:CoM: as before.

Please send `kh1-recom.log` (`met Sora's swing`, `clashed with Sora's swing` lines) and an F9 trace. A KH1 recording
of the same meetings would let us check the frames.

**Owner questions:**

1. **Sora's Keyblade sparks.** KH1 plays each side's own hit effect, or a clash spark, and a 10-frame hit-stop with a
   pad rumble when Sora's swing clashes. Built: Re:CoM's guard spark and clink only. (a) Keep; (b) add Re:CoM's own
   hit-stop for Sora's clashes (research its hit-stop first); (c) wait for the effects milestone (M13).
2. **Guard-type rows inside attacks hurt Sora.** The Soldier's spin-kick chest and the Powerwild's flurry body never
   hurt in KH1 (they only bounce swings), but they do here. (a) Leave it; (b) keep their records for the clash test and
   make Re:CoM's test of them against Sora answer 0 (the clash step's path, one more lookup per enemy attack test);
   (c) have the asset step make them guard rows of the shared enemy guard instead (the Bandit's way: Re:CoM's body
   guard while they are on), no records.
3. **Re:CoM's test order.** If an enemy's row already hit Sora earlier in the same frame, KH1 would have clashed first.
   (a) Keep (at most one frame, rare); (b) research where Re:CoM runs Sora's weapon test against the enemies' updates
   and move the clash test to the earlier of the two.

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
| hit handler | `0x1E20` | noticed; kinds 4 / 5: a hit counted, the light run reset; kinds 6 / 7 (clash): nothing counted; others: both counted, the 5th light hit in a row marks the bite, the 6th and later: no reaction (the hit's reaction kind zeroed: `func_193a(hit, 0)` at `0x2078`, then the thread ends) and no state change; else state 4. No `sys 42` anywhere in the script (Checked (files): the whole `ex_2280.bd` listed); a kind-0 record runs no case of KH1's reaction `0x1402B8E10` (`uVar6 < uVar5` fails: no push, no motion; Checked (decomp)) |
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
   **The no-reaction armour (owner 2026-11-02):** land five light hits in a row (no finisher), then hit it while it
   dashes or bites: it takes the damage (Re:CoM's damage voice), doesn't flinch, and the dash or bite carries on to
   its end (before: the attack stopped there).
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
- ~~A sixth light hit that lands while it attacks: Re:CoM's handler has already cut the attack (KH1's would go on).~~
  Built on the no-reaction armour (owner 2026-11-02, Pirate question 3 (b); § Hits that take no reaction):
  `darkball::Ai::armoured` is true for a light hit (kind not 4 / 5 / 6 / 7) that would be the `armor_limit`th (6th)
  or later of the run; Re:CoM's super armour keeps its state, so the dash (state 2), the bite or the drop (state 3), a
  reaction or a motion goes on; `on_armoured_hit` counts it as the handler does (noticed, a hit, a light hit, the
  bite mark from the 5th) with no push and no state request. Checked (files + decomp): the handler, no `sys 42`,
  kind 0 plays nothing. Checked (tests): unit `the_sixth_light_hit_on_the_armour`, host model
  `hosted_the_sixth_light_hit_keeps_the_attack` (dash in state 2, bite in state 3, no state request, never
  released). **Assumption:** Re:CoM's damage voice on the armoured hit (KH1 plays nothing), as the Pirate's. The
  hit's attribute isn't carried (as before): KH1's attribute 10 (Stun Impact, Sonic Blade) and status exceptions to
  the zeroing (`0x2048` / `func_170e`) don't apply, so those 6th+ hits take no reaction here. A hit whose kind the
  host couldn't read, or one in the frame Sora's Guard blocked it, keeps the old path (`keeps_reaction`: Re:CoM cuts
  the attack, back to state 1).

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
| hit handler | `0x1F0C` | attributes 9 / 11: nothing more (not carried); while 206 plays (`sys 51`, the request playing): a hit that is not a clash (6 / 7) nor of attribute 8 / 9 takes no reaction (`func_1c08(hit, 0)`: `hp[0xC] &= ~0x1F`) and marks the dive (`wp+0x4C`, its hit behaviour plays 211), every hit counts; else a launch (kind 5) counts and asks for the pause, other hits but clashes count; the hit state 3. No `sys 42` in the script (Checked (files)) |

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
   **The no-reaction armour (owner 2026-11-02):** stand under one so it dives, hit it during the straight drop: it
   takes the damage (Re:CoM's damage voice) and goes straight into 211 with no flinch frame in between (the log's
   `hit kind Some(3) without a reaction while Some(206) plays`); Guard its swoop: still KH1's recoil 213.
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
- **The dive's no-reaction hit on the armour** (owner 2026-11-02, Pirate question 3 (b); § Hits that take no
  reaction). Before: Re:CoM's handler asked for its flinch (state 11) and our `react_begin` handed straight back to
  state 1 (`keeps_reaction` false), a frame in state 11. Now `airpirate::Ai::armoured` is true while 206 plays and
  the hit's kind is known and not 6 / 7; Re:CoM's super armour keeps state 1 (206 is a plain motion, no attack
  rows: nothing to cut), `on_armoured_hit` marks the dive, counts the hit and asks for the hit state; 211 plays from
  the next AI frame. Checked (files): the handler `0x1F0C` (`sys 51` = the request playing, the zeroing, no `sys 42`;
  a kind-0 record plays nothing, Checked (decomp) `0x1402B8E10`). Checked (tests): `the_dive_on_the_armour`,
  `hosted_a_hit_in_the_dive_plays_211_in_state_1` (the host model: state 1 throughout, 211 the next frame for its
  86 frames, nothing released). **Assumption:** Re:CoM's damage voice on that hit (KH1 plays none). Fallback as
  before for a hit whose kind the host couldn't read or one in the frame Sora's Guard blocked it.
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
| hit handler | `0x2484` | death (`sys 44` = 0): `0x26B8` (not ported: Re:CoM's defeat); attributes 2 / 3 / 4: `sys 42` (`0x1402B7E60`: the engine's element reaction) and nothing more; 9 / 11: nothing more; the turn rate 11.25; while 202 plays (`sys 51`, the request playing): a hit not a clash (kinds 6 / 7) nor of attribute 8 / 9 takes no reaction (`func_20de(hit, 0)`: `hp[0xC] &= ~0x1F`; no `sys 42` on this path) and marks the dive (`wp+0x50`, its hit behaviour plays 214); every hit counts; otherwise every hit but a clash counts; the hit state 4 (Checked (files): `0x2484..0x26B0`) |
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
   **The no-reaction armour (owner 2026-11-02):** hit it during the straight drop: it takes the damage (Re:CoM's
   damage voice), no flinch, and goes into 214 on the next frame, without first dropping to Re:CoM's flinch (the
   log's `hit kind Some(3) without a reaction while Some(202) plays`); Guard the dive: still Re:CoM's flinch.
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
- **The dive's no-reaction hit on the armour** (owner 2026-11-02, Pirate question 3 (b); § Hits that take no
  reaction). Before: Re:CoM's handler asked for its flinch and cut the dive's attack state; `react_begin` handed
  back to state 1 and the hit state played 214 there. Now `wyvern::Ai::armoured` is true while 202 plays and the
  hit's kind is known and not 6 / 7; Re:CoM's super armour keeps the attack state 3; `on_armoured_hit` puts the
  turn rate back to 11.25°, marks the dive, counts the hit and asks for the hit state. The host (Checked (code +
  tests)): the next `attack_step` polls the steered dive, the behaviour restarts in its hit state and plays 214,
  `after_poll` sees another motion (Replaced): the action ends, Re:CoM's attack end runs and 214 plays on in
  state 1. Checked (files): the handler (`sys 51`, the zeroing, no `sys 42` on the dive path); kind 0 plays nothing
  (Checked (decomp) `0x1402B8E10`). Checked (tests): `the_dive_on_the_armour`,
  `hosted_a_hit_in_the_dive_plays_214_in_state_1` (the host model: state 3 for the hit's frame, 214 at the next
  AI frame, state 1 the frame after for its 92 frames, 202 not asked again, nothing released). **Assumption:**
  Re:CoM's damage voice on that hit (KH1 plays none). Re:CoM's handler clears the attack bit (`+0x260` bit 0, its
  `0x2000001` card path: no state change, Checked (decomp) `0x14017E1D0`) while the dive plays on, as for the
  Pirate. Fallback as before for a hit whose kind the host couldn't read or one in the frame Sora's Guard blocked
  it.
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

## The Search Ghost (M11 fliers)

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Search Ghost yet. The owner's
choices of 2026-11-02 (§ Search Ghost owner questions) are built: KH1's drain rule on Re:CoM's HP, EXP and prizes
(§ The drain's heal and reward), and the burst's type-1 rows through the shared enemy guard (§ The burst's repel rows).

### KH1's side. Checked (files: `ex_2230.bd` and `ex_2230_00.bd` disassembled, folded and raw; decomp: the syscalls named)

The script (0x3580 bytes; code from `0x5DC`, entry `0x253A`) has the Air Soldier's newer shape (the actor at `wp+0`, the
state object at `wp+0x20`, the search table at `wp+0x30`), on the flight of § Flying bodies, with its own routines (only
the random draws and the waits are the shared ones). The child object (`.mdls` part 3, its one sub-object
`ex_2230_00.bd`, 0x400 bytes, entry `0x202`) is the cone.

| Routine | Offset | What |
|---|---|---|
| entry | `0x253A` | init `0x26BC` (records LV1 `0x14` / LV2 `0x80`; stats `+0x10` = 800, the hop, unused; KH1's world: `wp+0x78` = world 11, `wp+0x74` = world 9 room 15, never read; home; the state object; the search table; handlers hit `0x285A`, `0x2836` (answers 0), `0x284E` (ends)), `sys 135` with (0, 0, 0, 1), a yield, the start `0x10E6`. Each frame: nothing while it warps (`wp+0x54`); a state request restarts the behaviour: 1 idle `0xCDE` (the target cleared), 2 engaged `0x786` (the target or Sora), 3 hit `0x628` (the hitter or Sora); while engaged, a drain that reached it (`wp+0x68`) is held (`wp+0x6C`) for 568 frames |
| start | `0x10E6` | placed (`sys 132` = 0) outside world 11: idle, draining always (`wp+0x7C` = 2); spawn type 6: face Sora, the appearance, idle, draining always; other spawns: Sora found (5000) or Sora, face him, the appearance (outside world 11), engaged, draining only below half its HP (`wp+0x7C` = 1) |
| appearance | `0x2130` | 69 (blend 0), a 30-frame fade-in, flag 0x80 for 30 frames, effect 2, 30 frames, then flag 4 with the idle queued (bit 30): the whole script waits for 69's end |
| idle | `0xCDE` | not noticed; turn rate 6; **the child** (`sys 94` at `0xD12`); forever: Sora beyond 10 m: a point 2–5 m from him, else 2–10 m from itself; the flight to it `0xDCE`; the search `0x1044` for [60, 600) frames |
| idle flight | `0xDCE(p, 3, 10)` | the idle (0) at 3 a frame, 500 frames at most; a target set → engaged; within 10 flat and a frame's travel vertically: done; rising (−0.2) under 50 over the floor, more than 2 m under home or 2 m under Sora; sinking (0.2) more than 1.5 m above him; stuck after 30 frames (under a quarter of the speed): a new point 2–5 m off |
| search | `0x1044(t)` | 202 (blend 20) for `t` frames, nothing else (KH1's longest silence, § Re:CoM's own states hand back); a target set → engaged |
| **the cone** | child `0x202` | effect 7 on bone 45 (`sys 109`); while the parent plays 0 or 202, each frame: the eye `sys 19(parent, 4141, 0)` (bone 45 \| 0x1000 at t 0: **its parent 44's position**, `0x1401D6320`'s lerp), the axis `sys 115(parent, (1, 0, 0, 0), 45)` (**44's** world row 0: `0x14029EA00` takes the parent, `0x1401D62B0`); for party members 0..2 (alive, flag 0x8000): the member's lock point 0 (`sys 32`) within 300 of the eye and the cosine to the axis over cos 35° (`sys 100`, `sys 3`) → the parent's target (`sys 49`), the effect removed, the child ends. Any other motion: the child ends |
| engaged | `0x786` | the notice once (`0x1210`: 206 as an action, the idle queued, effect 8); forever: below half its HP (`func_2212`, current / max) or draining always: the drain's approach `0xA34`, the hover with the hold `0xA0E`, the roam `0x7EC` (it ends in the idle); else the attack round `0xB62` |
| attack round | `0xB62` | the approach `0x30C4(target, 8, 450, idle)`; facing him, the burst; a flight `0x335A` to a point 5–8 m away at his height (8, within 1 m, 300 frames); easing 0.6, turn rate 6, 120 frames facing him, warping away 5–6 m once he is within 2 m; turn rate 11.25, easing 0.2 |
| burst | `0x128E` | effect 6 after 8 frames (`sys 107`); two hit bits, rows `0x1CC` / `0x23C` / `0x38C` / `0x3FC` and `0x2AC` / `0x31C` / `0x46C` / `0x4DC`; facing him, 204 for its length (facing him until 85 frames are left); the idle (blend 6) |
| drain's approach | `0xA34` | the target checked (alive, `sys 120`, flag 0x8000; else a new search); the approach `0x30C4(target, 20, 250, 200)`; facing him, the drain `0x13BE`; until a drain lands (`wp+0x58`), 200 again |
| drain | `0x13BE` | effect 5 after 8 frames; rows `0xEC` / `0x15C` with the callback `0x1794`; facing him, 203 for its length: with 60 or more frames left facing him; with more than 96 left: within 30 of him backing off (−3 × the facing at 4: 12 a frame), else creeping at him at 2. The callback (once, a target not reacting, without flag 0x80, `sys 170` = 1): `wp+0x58` = 1 and an orb effect (`sys 15`, effect 12) 80 above him; in the last 60 frames the orb flies to 30 above the Ghost (`p ← (p − me) × k + me − 30`, `k` 0.95 down 0.02 a frame); within 3.2 m of it: **drained** (`wp+0x68`, effect 11); within 1 m or with 20 frames left: the orb removed (`sys1.3`), the drain ends early. **An orb that starts within 1 m never drains** (the Ghost usually coasts in close: the hold is rare; tests `a_drain_from_close_heals_at_once` / `the_drain_holds_then_would_heal`). The rest of 203, the idle |
| hover | `0x2A50(target, speed, dist, time, mode)` | turn rate 2; away 20 m behind it (±180°), after [100, 180) frames off to a side (±60..120°), and on that leg within `dist` of him and level (± 1.5 m) a burst (turn rate 6) and away again, 3 bursts at most; rising under 50 over the floor; stuck after 60 frames: away at once. **Mode 1** (`0xA0E`: 8, 4 m, 800 frames, after the drain): once the hold is off (568 frames unhit, or no drain reached it) **HP = max HP** (`0x2ADE`: stats `+0x3C` = `+0x40`, then a debug print `sys1.0`; no effect), 30 frames facing him, done |
| roam | `0x7EC` | warp to a point 7–10 m away; Sora within 10 m: within 5 m the burst, else 202 for [400, 600) frames facing him, bursting once within 3.5 m; not found: 202 for [200, 300) frames facing Sora, bursting within 5.25 m; after 3 rounds the idle [30, 60) frames and the idle state |
| hit | `0x628` | noticed. Below half its HP (or draining always): 50 % the vanish (blend 0), invisibly at him at 24 to 0.8 m playing 200, face him, reappear; then engaged. Else 50 % a warp to a point 5–6 m away, otherwise the burst and the hover `(6, 4 m, 300, mode 0)`; then engaged |
| warp | `0x2EBA` | the vanish `0x2FF2` (warping, 201, the idle queued, flags 0x20 / 0x40 / 0x10: unhittable, its lock point masked (`sys 116`), effect 9, a 100-frame fade-out, 100 frames, hidden: flags 0x10 / 0x2 / 0x1 cleared); the flight to the point (24, within 10, 240 frames); the reappearance `0x2EEC` (stopped, the idle 80 frames, facing him, 201 and its queue **cancelled in the same frame** by `sys 12(0)` (`0x14029F7C0` zeroes the pending count `+0x178` first), shown, effect 10, a 100-frame fade-in, 80 frames, hittable, unmasked, not warping) |
| approach | `0x30C4(target, speed, dist, motion)` | at 50 above him playing `motion`, facing him, until within `dist` of him (or of that point); stuck after 30 frames: done within 1.5 × `dist`, else a detour flight to a point 4–6 m from him (200 frames) |
| flight to a point | `0x335A(p, speed, arrive, time)` | the idle; until within `arrive` flat and a frame's travel vertically; stuck (30 frames on): face ahead, stop, a new point 6–8 m from the target |
| hit handler | `0x285A` | easing 0.2; attribute 9: damage only; **while holding** (`0x28A6`, before the damage): the hold ends, `sys 194(EXP / 10 + 1)` (`0x28D8` / `0x28E0`; the stats block's `+0x68` is the record's EXP word r7, `0x1402A58E0`: an EXP bonus for the party, with its number shown, `0x140270990` / `0x140284990`) and the drop table `0x580` (`func_1F64`: at its lock point, `sys 32` → `sys 122`; 3 prizes of KH1 kind 4); then `sys 44` (the damage); death `0x2A14` (Re:CoM's defeat here); attribute 11: nothing more; the orb dropped (`wp+0x58` = 0); attribute 8 (Gravity, `0x29BC`): the same reward **on every Gravity hit** (`0x29DC` / `0x29E4`); every hit, clashes and repelled hits too: the hit state |
| leash | `0x34EE` → `0x18F0` | **not ported**: spawned and 15 m (12 m out of sight) from Sora it is removed; in a Re:CoM battle room it would never be defeated |

Attacks (rows `0x70` bytes, `sys 23`, mode 2: each lives while its request plays; Checked (files)):

| Attack | Request | Damaging rows (bone → its parent, t, radius) | Hit | Recoil `+0x40` | Re:CoM id | Re:CoM state |
|---|---|---|---|---|---|---|
| burst | 204 (164 f; the AI faces him) | `0x1CC` / `0x23C` (r. / l. hand, 1.5 → 1, 60) hit bit A; `0x2AC` / `0x31C` (the same) hit bit B | [36, 48) and [108, 118) | 205 (in the model) | 0 | 2 |
| drain | 203 (120 f; steered) | `0xEC` / `0x15C` (r. / l. hand, 1.5 → 1, 50), slots 0 / 1 | [34, 54) | 205 (in the model) | 1 | 3 |

The burst's other four rows (`0x38C` / `0x3FC` / `0x46C` / `0x4DC`, 90 at the hands at both strikes, no power words) are
**attack type 1**: KH1's attack-vs-hurt test skips types 1 / 2 / 4 (`0x1402B3BE0`), and type 1 meets only attack types 0 /
3 (the table `0x140528CB8`, byte 1 = 0x09): a repel layer for Sora's swings (§ Enemy guard: a type-1 side with the
higher priority repels). Built as the guard row `burst` (§ The burst's repel rows).

Reactions: the flying forms (F `0x48..0x4B`, K `0x4C` / `0x4D`, a `0x4E` / `0x4F`, A `0x50` / `0x51`; no launches: kind 5
→ kind 4); weight 1.0 in both records; hurt parts: 5 (collision block group 2 → Re:CoM bones 19, 2, 17, 51, 35). Checked
(files).

Motions (`xa_ex_2230.mset`, 17 entries): KH1's matcher finds Re:CoM bakes for the idle (0, 1, 2 → `e014_001`) and 70
(`e014_074`); edits (≤ 20 bones differ): entry 1 (7, 72, 73, 78, 103 → `e014_020`), 2 (74, 75, 79 → `e014_021`), 66 →
`e014_070`, 67 / 102 / 104 → `e014_071`, 68 → `e014_072`, **the drain 203 → `e014_051`**. Every other request is a new
clip at 100 + entry (69 → 108, 200..206 → 110..116); the edits get their Re:CoM clip's sounds.

### The cone, ported natively (owner 2026-11-02: "its gameplay (targeting) is ported")

- In the behaviour: KH1's child is a second script the behaviour runs itself (`kh1_core::searchghost`'s `cone`, polled
  after the parent's script each frame: Assumption ±1 frame, KH1's object order not read), made where the idle makes it
  (`0xD12`) and ending as KH1's does.
- Its inputs come from **Re:CoM's own animated bones** (framework: `enemy::Sight`, `Behaviour::set_sight`):
  - the eye and its axis: Re:CoM's `l_eye` (bone 32, the asset step's `sight_bone`: KH1 bone 45's parent 44). The
    converter poses each Re:CoM bone at its KH1 twin's world (`convert.rs` `targets`), so the node's position `+0xD0` and
    scaled row 0 `+0xA0` (`0x14023EB00`) are KH1's 44 world position and row 0 in Re:CoM's axes. Checked (decomp + files:
    both motions the cone runs in pose 44 and skip 45);
  - Sora's lock point 0: KH1's last collision group, code 8 at t 0 → bone 78's parent 77, which is Re:CoM's `chest1`
    (Re:CoM bone 37). Re:CoM keeps its index at `player+0x3A8` (`0x1401A4450(player, "chest1")`, so Halloween Town's
    skeleton works too). Checked (files + decomp).
- It runs even while the parent's script waits (`Behaviour::side_frame`): a reaction ends it as in KH1.
- Its look (effect 7, the searchlight) waits for the effects milestone (M13).

### The drain's heal and reward (owner 2026-11-02: "KH1's drain rule on Re:CoM's HP, EXP and prizes")

**KH1's rule** (Checked (files: `ex_2230.bd` disassembled, raw and folded) + decomp, Steam addresses):

- The orb counts only when it reaches the Ghost from 1–3.2 m (`orb_done` 100 ≤ distance < `orb_near` 320 at the drain's
  end, `wp+0x68`); an orb that starts within 1 m is removed at once and never counts.
- A counted orb is **held** (`wp+0x6C`) for `hold_time` 568 frames by the supervisor while it is engaged (`0x2678`).
- The hover after the drain (mode 1) heals as soon as the hold is off: **HP = max HP** (`0x2ADE`). An orb that never
  counted: at the hover's first frame (right after the drain); a counted one: 568 frames after it reached the Ghost.
- A hit while it holds (`0x28A6`, the hit handler, before `sys 44` applies the damage, so **the killing hit too**): the
  hold ends, **`sys 194(EXP / 10 + 1)`** (`0x1402BC9A0`: unless ability `0x40`'s slot says otherwise, `0x140270990`
  shows the number at its point `0x8004` and `0x140284990` adds it to each party member's EXP, **not** scaled by the
  defeat's EXP rate `0x142D5CB00` that `sys 167` applies, `0x1402A4F40`), and **the drop table `0x580`**: 3 prizes of
  KH1 kind 4 at its lock point (`0x1402ABC40`: counts × the Jackpot factor `0x142D60FA4`; kind 4 is an HP ball of +5,
  `0x1402AC080` → `0x1402A4920(…, 5)`; the kinds: 0..2 munny 1 / 5 / 20, 3 / 4 HP 1 / 5, 5 / 6 MP 2 / 15). The hit
  restarts its behaviour into the hit state: the hover (and its heal) is gone.
- With KH1's records: EXP 10 (LV1 `0x14`) → bonus 2; EXP 120 (LV2 `0x80`) → 13.

**On Re:CoM's state (built):**

| KH1 | Re:CoM | Where |
|---|---|---|
| HP = max HP (`0x2ADE`) | Re:CoM's own enemy heal `0x140181CE0(enemy, max − HP, no effect, 0xB, &point, 0)`, the call its own Ghost's drain makes for its 1/8 (`0x140192630`); max from `0x14017F5A0`. HP `+0x274`, the clamp and the HP gauge (`+0x430` → `0x14020B2D0`, `0x14020C9A0`) are Re:CoM's. Checked (decomp + exe) | `Cmd::HealFull`, `win::kh1_enemy::heal_full` |
| `sys 194(EXP / 10 + 1)` | **KH1's formula on Re:CoM's EXP for that Ghost** (`+0x2E4`, its `@ENP` row's `+8`, `0x14017FF50`: 12 on the first row up to 106 on the last, so bonuses 2..11 against KH1's 2 / 13), made as **Re:CoM's EXP prizes by its own defeat EXP drop `0x1401810A0(enemy, &point)`** (the bonus sits in `+0x2E4` for that call, Re:CoM's EXP put back after). Decision M10 keeps Re:CoM's EXP; Re:CoM gives EXP only through its EXP prizes, so the bonus is that many EXP prizes' worth. KH1's popup number: none (Re:CoM's EXP prizes show theirs on pickup). Checked (decomp + exe); the mapping is the port's choice (owner: "sensibly") | `Cmd::Reward`, `win::kh1_enemy::reward` |
| 3 × KH1 kind 4 (HP + 5) | 3 × **Re:CoM's HP prize 0xD3** (`0x1401F4AB0`: 5 % of Sora's max HP, Re:CoM's own heal and its "Recover"), made by Re:CoM's prize factory `0x1401A89C0` with the defeat EXP drop's launch (pitch (−70 − r × 10)°, yaw r × 360°, speed 0.25; Re:CoM's random float and angle normaliser). Re:CoM never drops 0xD3 from a defeat (only Pluto's friend card makes them, `0x1401E2E60`). Jackpot: 1 (no such ability yet). Checked (decomp) | `crate::drops::reward_prize_kind`, `win::drops::spawn_reward_prize` |
| at its lock point (`sys 32`) | at Re:CoM's prize point (vtable `+0x50`, `0x1401A4700`: where its defeat makes its prizes) | `win::kh1_enemy::prize_point` |
| the killing hit while holding | the reward first, then Re:CoM's defeat (its EXP prizes, roulette prizes, card) | `defeat_begin` → `Behaviour::on_defeat` |
| no `@ENP` row | nothing (Re:CoM makes no prize without one) | |

Framework (shared): `Cmd::HealFull`, `Cmd::Reward(Reward)` (the numbers from the player's script: `reward_exp_div` /
`reward_exp_add` at `0x28D8` / `0x28E0`, also pushed at `0x29DC` / `0x29E4`; `reward_prize_0..6` from the table `0x580`,
`crate::enemy::REWARDS`; a table with item chances is refused), `Behaviour::take_now` (what a handler does in the
handler: the host applies it right after `on_hit`, not after the reaction), `Behaviour::on_defeat` (the killing hit's
handler, before Re:CoM's defeat prizes).

Not built: KH1's EXP number popup; the Gravity reward on every Gravity hit (`0x29BC`, needs KH1's hit attribute, not
carried); ability `0x40`'s gate on `sys 194` (its name not read; Re:CoM has no such ability); the Jackpot factor.

Assumptions: Re:CoM's HP prize is the right twin of KH1's HP ball (Re:CoM's heal amount, decision M10); the descriptor's
`+0x2C` = 1 (the defeat EXP drop's; Pluto's 0xD3 use 0: only `+0x2E` bit 0 is read by the prize init `0x1402315B0`).

### The burst's repel rows (owner 2026-11-02: "its burst repels Sora's swings" through the shared enemy guard)

- Rows (Checked (files)): `0x38C` / `0x3FC` (right / left hand, HIT slot 0, hit bit A) and `0x46C` / `0x4DC` (slot 1, hit
  bit B), all type 1, priority 40, recoil 205, 0.9 m, front cone turn 0 / **cos 1.0**, made with 204 at `0x12D4..0x132A`.
  The asset step merges them into one guard row `burst` (`GuardSpec::also`: same clash numbers checked; windows of slots
  0 and 1 joined: 204 [36, 48) and [108, 118)); the behaviour sends `Cmd::Guard` with 204 at each burst.
- KH1's rule (§ Enemy guard), with Sora's Kingdom Key swing at priority 30: the row is 10 above, type 1 → it **repels**:
  Sora gets kind 6 (diff −10: recoil 0x6E, pushed back 25), the Ghost the swing's record flagged 0x20 (no damage, no
  reaction; its hit handler hears it: during a hold that is the reward, then the hit state). A Keyblade at 40: both kind
  6 (the Ghost plays 205); above 40: the Ghost kind 6 (or 7 at 40+ under), Sora kind 6 with no push.
- **KH1's front cone makes them inert in practice.** `0x1402B4640` tests a guard-type row only when the 3D cosine between
  the owner's facing (`actor+0x30` = (sin yaw, 0, cos yaw), `0x140297E10`) and the vector to the attacker is at least
  row `+0x64`; these rows hold **1.0** (the damaging rows' default), so the test passes only for an attacker exactly
  along its facing, at exactly its height (a hovering Ghost never is). Sora's swings (`0x1402B2DC0` via `0x1402B3150`)
  carry no flag 0x40 that would skip the test. Checked (decomp + exe + files). The port keeps the number (the
  manifest's `cos` 1.0) and KH1's 3D test (`win::kh1_enemy::kh1_front`, before Re:CoM's apply): **in game, Sora's
  swings should land on a bursting Ghost as before**. Owner question below.

### Re:CoM's side. Checked (exe + decomp)

`CEnemySearchghost` (vtable `0x14054C408`, 139 slots, RTTI at −8), ctor `0x1401923F0` (`40 53 48 83 EC 20`: `CHeartless`'s
base ctor, then `+0x154 |= 0x80`, `+0xCD0` = 0, its drain effect's handle, `+0x260 |= 0x100`: gravity off; its only
caller the factory `0x140188F40` case 14 at `0x1401891E8`; no branch into the moved bytes, no pointer to them, no
rip-relative operand: exe scan), allocation `0x1401891CF` `mov ecx, 0xCE0` → 0xD00. It overrides states 1 (the flying walk
`0x1401925B0` → `0x1401979F0` with `e014_001`, step `0x140192760`), 2 (`e014_050`, `0x140192510` / `0x1401925F0`), 3 (its
drain `e014_051`, `0x140192560` / `0x140192630`: when it lands, `+0x260 & 0x40`, **it heals 1/8 of its max HP**,
`0x140181CE0(enemy, max / 8, 0, 0xB)`, with the effect `ex_2230` 0xB on bone 1, `0x1401A2D50`); its think is
`CHeartless`'s `0x140196EC0`. Our copy: states 0, 1, 2, 3, 11, 12, 30, the think; KH1's burst in state 2, its drain in
state 3 (Re:CoM's 1/8 heal doesn't run; KH1's rule does, § The drain's heal and reward). `EN0014.ASD`: 2 records at `0x6C`, tail `0x1AC` (id 0 `e014_050`
on `l_fing3b`, 0.5 m, [108, 118); id 1 `e014_051` on `l_fing3b`, 0.5 m, [30, 60)). `EN0014.SES`: Re:CoM's entries.

Power mapping (decision M10, "the Re:CoM attack of the same kind"): the drain on id 1 (Re:CoM's drain `e014_051` is an
edit of KH1's 203); the burst on id 0 (Re:CoM's other attack; `e014_050` is not within the matcher's edit rule of KH1's
204, but its window is exactly 204's second strike, [108, 118): Checked (files)).

### Framework additions (M11, the Search Ghost)

- `enemy::Sight` and `Behaviour::set_sight`: the host gives a behaviour whose manifest names a `sight_bone` that bone's
  position and row 0 and Sora's lock point each frame (read from Re:CoM's nodes; the bone is resolved once at the ctor, no
  per-frame read for other classes).
- `Behaviour::side_frame`: KH1's child objects run every frame, also while the parent's script waits for an action or a
  reaction; the host calls it on the frames it doesn't poll `frame` (not after the defeat). Default: nothing.
- `Cmd::Note`: a log line for something KH1 does that isn't built (`note: …`).
- Asset step: `enemy::child_script` (part 3's sub-objects), `enemy::CHILD_CONSTS` (numbers and request checks from a
  child script) and `enemy::SIGHTS` (a bone code → `sight_bone`, KH1's parent rule).
- `kh1_enemy::watchdog::QUIET_LIMIT` 600 → 900 (the idle search's silence, above). Every other enemy's outputs are
  unchanged (their asset tests pass; the new tables are keyed by enemy).
- The drain rule (owner 2026-11-02): `Cmd::HealFull`, `Cmd::Reward` / `enemy::Reward`, `Behaviour::take_now`,
  `Behaviour::on_defeat`; asset step `enemy::REWARDS`; sites `KH1_ENEMY_REWARD_CALLEES` (Re:CoM's enemy heal
  `0x140181CE0`, its defeat EXP drop `0x1401810A0`) and the Ghost's vtable `+0x50` (prize point) / `+0x3D8` (guard event).
- The repel (owner 2026-11-02): `HitNews::Repelled`, the guard event's repel and "neither" outcomes, KH1's 3D front cone
  before Re:CoM's apply, merged guard rows (`GuardSpec::also`) (§ Enemy guard).

### What the asset step writes

`ENEMY/0014/EN0014.ANN` (Re:CoM's 75 slots byte-identical; KH1's clips at 100 + entry: 101..108, 110..116), `.ASD` (Re:CoM's
2 records kept + 6: the burst's two hands at each strike on `e014_114`, the drain's two hands on `e014_113`), `.SES`
(Re:CoM's entries kept; the edits get their Re:CoM clip's sounds: the drain `e014_051`'s; the burst its template's at its
window: Assumption), `kh1-searchghost.json` (`kh1-enemy 1`: 121 script constants read at checked offsets (and the stats
block's two defaults), 2 from the child, `sight_bone`, 25 request words checked + 5 in the child, 2 attacks, 5 parts; since the
drain rule also `reward_exp_div` / `reward_exp_add` (4 pushes checked) and `reward_prize_0..6` (the table `0x580`), and
`guards`: the row `burst`). Test: `searchghost::tests::builds_from_the_players_files`. **Re-run the step** for these
(`EN0014.ANN` / `.ASD` / `.SES` are byte-identical to the 0.76.0 step's; only `kh1-searchghost.json` changes; a 0.76.0
manifest fails to load: no `reward_*`).

### Owner install steps

1. Run the step (`kh1-recom-build`): it prints a `searchghost (M11): …` line.
2. Copy into the OpenKH mod folder and add to `mod.yml`:

```yaml
- {name: ENEMY/0014/EN0014.ANN, method: copy, source: [{name: ENEMY/0014/EN0014.ANN}]}
- {name: ENEMY/0014/EN0014.ASD, method: copy, source: [{name: ENEMY/0014/EN0014.ASD}]}
- {name: ENEMY/0014/EN0014.SES, method: copy, source: [{name: ENEMY/0014/EN0014.SES}]}
- {name: kh1-searchghost.json, method: copy, source: [{name: kh1-searchghost.json}]}
```

3. The log shows `searchghost: KH1's searchghost (M11) …` at start and `battle N: Search Ghost … gets KH1's AI` in a Search
   Ghost battle.

### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`; any Search Ghost battle)

1. Each Search Ghost faces Sora and plays KH1's appearance (69, 2 s), then notices him (206, ~1.8 s).
2. Above half its HP: it flies in to about 4.5 m, bursts (204: two swipes, ~0.6 s and ~1.8 s in), flies off 5–8 m, watches
   him for 2 s and vanishes when he comes within 2 m.
3. The vanish: it plays 201 and fades (it can't be hit from the start), disappears after ~1.7 s, glides invisibly, shows
   again ~1.3 s after it stops, and can be hit ~1.3 s after that.
4. Hit it (above half): it warps away 5–6 m, or bursts and hovers off (away, then off to a side, bursting when level with
   Sora within 4 m).
5. Below half its HP: it lunges in (200) and drains (203): a hit with both hands about 0.6–0.9 s in; it then hovers off
   and, after a while, warps around searching (202) and bursts when Sora is close; after three warps it idles.
6. The idle: it glides slowly and searches (202, up to 10 s); it engages only when Sora is in front of its eye within 3 m
   (35° either side), or when hit.
7. Guard the burst or the drain: KH1's clash recoil (205).
8. Kill one: Re:CoM's prizes and EXP.
9. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Search Ghosts (its drain heals it 1/8 of its HP).
10. The drain from close (the usual case: it drains while touching Sora): as its drain ends and it hovers off, its HP bar
    goes back to full (the log: `heals to full HP`, `KH1's full heal … HP a -> max`).
11. The drain from 1–3 m (step back as it drains): it hovers off with its HP unchanged; ~9.5 s later its HP bar goes back
    to full.
12. Hit it in those ~9.5 s: its HP stays down; EXP prizes and 3 HP prizes (Re:CoM's HP orbs) burst out of it; collecting
    an HP prize heals Sora (Re:CoM's "Recover"). The log: `reward: EXP bonus EXP / 10 + 1 …`, `KH1's reward at … EXP bonus
    n (Re:CoM's EXP e / 10 + 1)`, the prizes made. Hit it again: no second reward.
13. Kill it with the hit that ends the hold: the reward's prizes, then Re:CoM's defeat prizes.
14. The burst: Sora's swings land on a bursting Ghost (its repel rows never meet him, as in KH1: § The burst's repel rows).
    The log: `guard row 0 with 204` at each burst, and no `guard event` line.

Please send `kh1-recom.log` (`searchghost:` lines: `state …`, `attack 204 (burst), steered`, `attack 203 (drain), steered`,
`its drain hit Sora`, `visible false` / `visible true`, `hittable …`, `target true`, `heals to full HP`, `KH1's full heal`,
`reward`, `KH1's reward`, `guard row`, `guard event`, `note: …`, the `rand` lines) and an F9 trace.

### KH1 trace (what to record in KH1)

A KH1 room with Search Ghosts (Monstro or Neverland): stand far 10 s; stand at 4 m 10 s; stand
behind an idle one, then in front of it at 2 m; one swing / a full combo / a finisher; bring one below half its HP and let it
drain you, then (a) hit it within 9 s, (b) leave it alone for 10 s; Guard a burst and a drain; kill one. The trace's enemy
rows give the warp timings, the cone's reach and the drain's hold to compare.

### Not built / open (the Search Ghost)

- The leash, the party retargeting, `sys 139`'s out-of-sight test (taken as in sight), the placed start and spawn type 6
  (Re:CoM's battles spawn every Ghost: Assumption, as the Darkball's).
- KH1's world 11: its searches reach 100 m there (ported); **its appearance is skipped there in KH1, kept here** (owner
  2026-11-02, "battle spawns play KH1's appearance").
- KH1's hit attribute (`hp[0xC] >> 6 & 15`) isn't carried: attribute 9 (damage only), 11 (no hit state) and 8 (Gravity's
  reward on every Gravity hit, `0x29BC`) never come.
- The drain's reward: KH1's EXP popup number, ability `0x40`'s gate, the Jackpot factor (§ The drain's heal and reward).
- The effects (`sys 75` 2 / 8 / 9 / 10 / 11, `sys 107` 5 / 6, the cone's `sys 109` 7, the drain's orb `sys 15` / `sys 16`:
  its flight is counted, not drawn), the alpha fades (no Re:CoM model alpha), flag 0x80 at the appearance, the lock point
  mask while it warps (Re:CoM's lockable flag stays, the Darkball's rule): the effects milestone (M13).
- The drain's callback refuses a target that is reacting, has KH1's flag 0x80 or isn't `sys 170`'s 1 (**Assumption**: a
  hit Re:CoM applied to Sora passes).
- The cone's other party members and their flag 0x8000 (**Assumption**: Sora passes); the cone runs after the parent's
  script in a frame (**Assumption** ±1 frame); Re:CoM's nodes are last frame's pose (**Assumption** ±1 frame).
- Each hit bit's rows are Re:CoM records with their own hit serials (`0x14014E830` → `0x14016F7D0` at each record's window
  start): the two hands of a strike could both hit (**Assumption**: Sora's post-hit invulnerability, decision 31, keeps it
  to one, as the Wyvern's charge).
- `sys 10`'s turning sign (the hover's sides): the Air Soldier's convention (**Assumption**).
- The floor for `sys 89` is Sora's height when he last stood (**Assumption**: flat battle floors).

### Search Ghost owner questions

Answered 2026-11-02: 1 (a) (built, § The drain's heal and reward), 2 (a) (built, § The burst's repel rows), 3 kept.

4. **The burst's cone (new, 2026-11-02).** KH1's four type-1 rows have the front cone cos 1.0, so KH1's own test lets them
   meet only a swing from exactly along its facing at exactly its height: in KH1 the burst practically never repels Sora
   (Checked (decomp + exe + files); a KH1 trace swinging into a burst would confirm). Built: KH1's number (inert).
   (a) Keep KH1's (1:1: the swings land); (b) give the rows a cone of their own as a deliberate change (for example the
   front half, as the Bandit's block), behind an ini key, so the burst repels Sora's swings from in front (not 1:1).

Earlier questions (kept for the record):

1. **The drain and its HP.** KH1: a landed drain whose orb reaches the Ghost (from 1–3.2 m) is *held* for 568 frames
   (9.5 s); a hit in that time ends it and gives the party an EXP bonus (its level / 10 + 1) and 3 prizes; if nobody hits it,
   or when the orb starts within 1 m (the usual case: it coasts in close), **its HP goes back to full** (`0x2ADE`). Re:CoM:
   its drain heals it 1/8 of its max HP when it lands. Built: neither (the log says `note: …` where KH1 would heal or
   reward); the timing (the hold, the hover that waits for it) is KH1's. (a) KH1's full heal on Re:CoM's HP and its reward as
   Re:CoM prizes / EXP, (b) Re:CoM's 1/8 heal when the drain lands (Re:CoM's own `0x140181CE0` and effect), (c) keep neither.
2. **The burst's repel layer** (its four type-1 rows `0x38C` / `0x3FC` / `0x46C` / `0x4DC`: priority 40, row `+0x60`
   turn 0, `+0x64` 1.0, on the two strikes' windows): in KH1 a type-1 row meets only Sora's swings, and with the higher
   priority it **repels** them (`0x1402BF630`), else it gets the clash (§ Enemy guard: KH1's rule). The shared
   enemy-guard step is built (Stage 135, the Bandit's type-2 rows); its repel is the part still to build, for the Large
   Body and the Defender. (a) Add the Search Ghost's rows to that step when the repel is built, (b) leave them out.
3. **The burst's power**: Re:CoM's Search Ghost has `e014_050` (id 0) and its drain (id 1). Chosen: the burst on id 0 (its
   window is the burst's second strike), the drain on id 1. (a) Keep, (b) another rule.

## The other fliers: not ported yet (survey for the next pass)

Search Ghost, Air Pirate, Gargoyle, Darkball and Wyvern were surveyed, not ported (time); the Darkball, the
Air Pirate, the Wyvern and the Search Ghost are built since (§ The Darkball, § The Air Pirate, § The Wyvern, § The
Search Ghost). All five enter flight
once with `sys 135` and steer through `+0x20` / `+0x2C` like the Air Soldier (6–14 steering writes each), so § Flying
bodies covers their bodies; each is a new script (1170–1710 listing lines) to port routine by routine. Checked
(files: the scripts listed; exe: ctors, allocation sites, vtables) unless marked:

| Enemy | Script (code from) | Re:CoM class (ctor / alloc / vtable) | Re:CoM `.ASD` | Open before porting |
|---|---|---|---|---|
| Search Ghost (**built**) | `ex_2230.bd` (`0x5DC`) | `CEnemySearchghost` `0x1401923F0` / `0x1401891CF` **0xCE0** / `0x14054C408` | 2 (`l_fing3b`, ids 0 / 1) | its child is the cone (no bullet), the drain's orb is an effect, `sys1.3` removes it: § The Search Ghost |
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
- **Sora's swing knocks it away** (owner 2026-11-02, on 0.75.2: "Sora's swings can knock the thrown scimitar away").
  - KH1 (Checked (decomp + files)). Two attacks of types that meet (`0x140528CB8`; 0 meets 0) are resolved
    both ways (`0x1402B4A20` → `0x1402B4BC0`). Neither row is a guard type, so there is no front cone
    (`0x1402B4640` tests it only for types 1..4).
  - The blade's row `0x1CC` is type 0, priority 0. Sora's side gets nothing: no record, since the other side's
    priority is 0. The blade's side gets clash bit 0x10 (0x20 at 40+ under), and its owner becomes the swinger
    (a projectile, `+0x30 & 0x400`).
  - Its child script hears `+4 & 0x30` (`ex_2090_00.bd` `0x1A0`). It then:
    - turns back with the random tilt (`0x1B8..0x222`);
    - fades over 60 frames (`sys 26`) and gives its model a random spin;
    - slides with its speed × 0.9 a frame for 60 frames, without moving its attack again (`0x282..0x2CC`);
    - ends (`sys 97`).
  - Here (no new hook, no per-frame code of its own):
    - In the swing's pair test (`win::attack::pair_dispatch`, while Sora's KH1 swing is live), the same
      spheres meet each flying blade's own Re:CoM collision shape through Re:CoM's overlap tests
      (`0x1401777E0` / `0x140176FA0`).
    - A blade they meet gets `Thrown::repel` (KH1's three draws from a stream of our own: Assumption), its
      Re:CoM attack off (`0x14014DE80`, `coll+0x10 = 0`) and its life raised to outlast the slide.
    - Re:CoM's guard spark and clink play at the blade (`"always"` 0xB, actor 0, as `0x140182260`).
    - Our flight step slides it, and Re:CoM's life ends it after 60 frames.
    - One flag test per live swing frame while no blade flies.
  - Not KH1's:
    - Re:CoM's spark for KH1's clash effect (`0x1402AE650` 10 / 11);
    - Re:CoM's end fade (1000 ticks) after the slide, for KH1's 60-frame fade during it;
    - no model spin (the look is a Re:CoM effect);
    - Sora's swing keeps hitting after the clash. KH1 marks it clashed (`+4 |= 0x10`); that a clashed
      swing meets no body afterwards is an Assumption, not built.
  - A blade Sora's Guard turned back is left to Re:CoM (decision 28).
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
- **Not built (guard / blade):** KH1's repel slide after a Guard (decision 28's turn-back
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
   Bandit. Swing at a flying blade: it bounces back off the Keyblade with a spark and slides away, harmless, for
   about 1 s. Over a Bandit that blocks, no Re:CoM "Guard" popup shows. Let the blade hit you: Re:CoM's damage for the Bandit's whirl.
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
Answered 2026-11-02 (on 0.75.2): 5 (b) hide the popup (built); 6 (a) keep Re:CoM's body guard; 7 (b) swings knock the
blade away (built); 8 (a) keep the whirl's power.
The questions as asked (0.75.2):

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

### The Pirate (built)

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Pirate yet. Its attacks keep
going through damage as KH1's do (§ Hits that take no reaction), its block stance and its spin block Sora's swings
from the front through the shared enemy-guard step (§ Enemy guard).

#### KH1's side. Checked (files: `ex_2100.bd` disassembled, code from `0x5A8`; decomp: the syscalls named)

The script (0x2B80 bytes) has the Soldier's older shape (the actor at `wp+0`, the state object at `wp+0x20`, a search
object at `wp+0x30`). No child object (no `sys 94`).

| Routine | Offset | What |
|---|---|---|
| entry | `0x2824` | init `0x2960` (records LV1 `0x14` / LV2 `0x80`, walk 3, run 6, home, the handlers: hit `0x2A3C`); a yield; the appearance thread `0x992`; each frame: while idle the search `0x182E(&wp30, 0, 1000, 1)` (Sora within 1000; in KH1's world 11 within 10000 and always found) and his height within ±100 of its own → engaged with the target, else no target; then a state request restarts the behaviour (1 idle `0x946`, 2 engaged `0x6B4`, 3 hit `0x5CC`) |
| appearance | `0x992` → `0x1B58` | (spawn type ≠ 0) the 30-frame fade-in, actor flag 0x80 for 30 frames (`0x24FA`), facing Sora now, action 69 with bit 23 (`0x1800045`, blend 8) and 6 queued, `sys 50` with a zero vector, effect 2; while in the air; then idle |
| idle | `0x946` | the shared wander `0x1A48(100, 300, &home)` (points [0, 100) and [200, 300) about where it stands, walked to (300 each), 40 % an idle of [90, 180), home beyond 700), then 60 % the taunt 220 |
| engaged | `0x6B4` | (the leash `0x118C`) Sora above `0x870`; < 250 close `0x828`, < 450 mid `0x7DA`, else far `0x756`; Sora above again; (the leash); the engaged search; engaged again |
| above | `0x870` | Sora on the ground (`*(target + 0x70)` = 0) above it (KH1 y down): 50–300 the leap; 400 or more the back hop, the run in and the leap. Heights relative to its own: no world height (the owner's rule for world heights doesn't apply) |
| close | `0x828` | 30 % a side hop and the slash; under 50 % (the same draw) the jump slash; else the spin |
| mid | `0x7DA` | 30 % the forward hop; then 40 % a side hop and the slash, else the spin |
| far | `0x756` | 60 % the back hop, the run in (to 350) and the leap, then beyond 450 30 % the taunt; else 30 % the laugh 212, then the spin |
| hit | `0x5CC` | after its own launch (`wp+0x48` ≠ 2) the get-up slash (235 from the front, 236 from behind) and the spin; with a hit marked (`wp+0x54` = 5) within 300: while it has raised the stance at most once (`wp+0x40` ≤ 1) 30 % the stance, else the counter slash; past that the count back to 0 and the counter slash; farther the jump slash; no hit marked: the leap. Then engaged |
| hit handler | `0x2A3C` | `wp+0x48` = 2. A clash (kinds 6 / 7): turn to the attacker, mark, the hit state. A damaging hit (`0x1F40` → `sys 44`) while one of its attacks plays (216, 214, 235, 236, 218, 210) or while it lies (`sys 70`): **`sys 42`** only (§ Hits that take no reaction). Else kind 4 / 5 (`0x25CA`): its own launch (`sys 38`: 240 on the ground / 250 in the air, hop 100, speed 30, down 0), the side (`0x151E`: `sys 100` of its facing against the push < 0 is the front), the hit state (no mark). Else mark, the hit state. Death: `0x19CE` (not ported: Re:CoM's defeat) |

Attacks (KH1's bone rows of mode 2 live while their request plays: `0x1402B2DC0` gives a bone row flags `1001b`;
`sys 213(row, 2)` sets the same mode on the spin's and the stance's guard rows, `0x1402BBC10`; Checked (decomp)):

| Attack | Requests | Damaging rows (bone code → bone, t, radius; HIT slot) | Hit windows | Recoil | Re:CoM id | How it plays |
|---|---|---|---|---|---|---|
| slash | 214 (178 f, root) | `0x23C` (48 → `wep0`, 2.5 → −0.5, 50; 0) | [122, 134) | 215 | 1 | plain request (steered): turning to Sora to its end, then the idle |
| counter | 216 (110 f, root) | `0x4DC` (blade, 50; 0) | [32, 35) | 215 | 1 | action |
| jump slash | 210 from frame 8 (58 f) | `0x31C` (blade, 35; 0) | [28, 34) of the motion, [20, 26) of the clip | 211 | 0 | action, at once in a 60-frame timed jump (facing × 18, height 12), 6 queued; then the forward hop |
| slam | 218 (52 f) | `0x38C` (blade, 50; 0) | [0, 5) | 215 | 1 | plain request on landing from the leap (facing × 12, height 200, 45 frames); then 90 % 213, else 205 → 206 → 207 |
| spin | 222 from frame 8 (122 f), 217 (48 f, looped) | `0xEC` (slot 0), `0x15C` (slot 1) (blade, 40) | [12, 18), [36, 42) each cycle | 221 | 0 | plain request: gliding at Sora at 3.6 (a random point 400–600 away within 30) for 180 frames, ended off the ground or 15 frames after a row landed (`0x117C`), then 219 |
| get-up slash | 235 (88 f, root) / 236 (64 f, root) | `0x3FC` (blade, 50; 0) | [44, 62) / [20, 30) | 215 | 0 | action |

Guard-type rows (type 2, front half: cos 0, turn 0): the spin's body `0x1CC` (code 8, 55, priority 90, recoil 221, HIT
slot 2: on through 217) and the stance's blade `0x46C` (code 48, 25, priority 30, recoil 209, HIT slot 0: on through
208). Row `0x2AC` (type 1, priority 100) is never made. The hops `0x157C(angle, speed, height, time, 3)`: forward 0°
× 30, 150 high, 40 frames; sideways ±15° × 25, 30 high, 30 frames; back 180° × 20, 100 high, 30 frames (then the run
in, `0x23B0(1, 300, 20, 350)`); drag 0.9; the trio 3 / 4 (bit 16) / 5 / 6. Reactions: the shared engine with its
motions (flinch 0x48..0x4B, air flinch 0x4E / 0x4F, launch 0x52.., air launch 0x5C..; no knock-back 0x4C / 0x50, so a
kind 4 launches, but its handler takes kinds 4 / 5 as its own launch); its own launch 240 / 250 (its get-up 244 / 249
two frames long). Weight 1.0 in both records. Hurt parts: 7 (Re:CoM bones 16, 2, 15, 11, 7, 27, 43). Checked (files).

#### Re:CoM's side. Checked (exe + decomp)

`CEnemyPirate` (vtable `0x140546458`, 139 slots, RTTI COL at −8), ctor `0x140190F30` (`40 53 48 83 EC 20`, its only
reference the factory's call at `0x14018930E`; no branch into the moved bytes, no pointer to it: exe scan), allocation
`0x1401892F5` `mov ecx, 0xCD0` → 0xCF0. Its own states 1 (`0x140191090` / `0x140191370`), 2 (its spin `e021_050`,
`0x140191000` / `0x1401910C0`), 3 (its slash `e021_051`, `0x140191040` / `0x140191120`); 0 / 11 / 12 / 30 and the attack
states' ends `CBattleEnemy`'s; think `0x140196EC0` (`CHeartless`'s). Our copy: states 0, 1, 2 (the spin, the jump slash,
the get-up slashes), 3 (the slash, the counter, the slam), 11, 12, 30, the think. `EN0021.ASD`: 2 records at `0x6C`,
tail `0x1AC` (id 0 on `e021_050`, `wep0`, 0.2 m, record kind index 0 = Re:CoM kind 1; id 1 on `e021_051` [113, 140),
kind index 8 = Re:CoM kind 7). `EN0021.SES`: `e021_050`, `e021_051`. `EN0021.ANN`: 75 slots, 30 fps clips.

**Power mapping** (decision M10; the owner's "the Re:CoM attack of the same kind"): the asset step's matcher pairs
Re:CoM's `e021_050` with KH1's spin 217 and `e021_051` with KH1's slash 214 (Re:CoM's 30 fps edits: every bone but
the root, hips and spine agrees; Checked (files)): spin → id 0, slash → id 1. The others have no Re:CoM twin; by the
kind of hit (their rows' `+8`, KH1's reaction on Sora, and `+0x14`, KH1's power): the jump slash and the get-up
slashes (kind 1, power 2, as the spin's) → id 0; the counter and the slam (kind 4, power 4; the slash's kind 4, power
6) → id 1 (Assumption: the nearest kind; owner question 1). Each runs in the state of its id (2 / 3).

#### What the asset step writes

`ENEMY/0021/EN0021.ANN` (Re:CoM's 75 slots byte-identical, KH1's clips at 100 + entry: no exact bake, so every request
plays a KH1 clip; the jump slash 210 and the wind-up 222 from frame 8, `enemy::STARTS`; the spin 217 looped,
`enemy::LOOPS`), `EN0021.ASD` (Re:CoM's 2 records kept, 8 added: slash 1, counter 1, jump slash 1, slam 1, spin 2,
get-ups 2), `EN0021.SES` (Re:CoM's 2 entries kept; the slash and the spin take their Re:CoM twins' sounds, the others
their template's at the hit window: Assumption), `kh1-pirate.json` (`kh1-enemy 1`, 105 numbers, the two guard rows).
Test `pirate::tests::builds_from_the_players_files`. In `mod.yml`:

```yaml
- {name: ENEMY/0021/EN0021.ANN, method: copy, source: [{name: ENEMY/0021/EN0021.ANN}]}
- {name: ENEMY/0021/EN0021.ASD, method: copy, source: [{name: ENEMY/0021/EN0021.ASD}]}
- {name: ENEMY/0021/EN0021.SES, method: copy, source: [{name: ENEMY/0021/EN0021.SES}]}
- {name: kh1-pirate.json, method: copy, source: [{name: kh1-pirate.json}]}
```

The step prints `pirate (M11): …`; the log shows `pirate: KH1's pirate (M11) …` at start, `battle N: Pirate … gets KH1's
AI` in a Pirate battle, per Pirate `pirate: eNNN: KH1 world Some(13) (Re:CoM world 8)` (Neverland) at its appearance, and
`pirate: eNNN: hit kind … without a reaction while Some(214) plays (KH1's no-reaction hit on Re:CoM's super armour)` for a hit its
attacks took.

#### Checked / Assumption

- **Checked (files):** every script number at its offset (the asset test reads them from the player's script), 39 request
  words, the rows (types, priorities, recoils 215 / 211 / 221 / 209, all in the model), the hit windows, the hurt parts,
  the start frames (the script's 8s equal `STARTS`), Re:CoM's records and clips, the matcher's pairs.
- **Checked (decomp):** `sys 42` / `sys 70` / `sys 213` / `sys 38`, the attack rows' modes (`0x1402B2DC0`,
  `0x1402B47A0`), `+0x3EA`'s writers, Re:CoM's hit handler and picker with bits 20 / 21, the state begins' clips.
- **Checked (exe):** the class, its slots, the ctor and its only call, the factory size; every site (617 byte sites, 269
  pointer sites, pefile, 2026-11-02).
- **Assumptions:** the power of the five attacks without a Re:CoM twin; `+0x3EA` 0 before any launch (so a hit during the
  first crouch 3 of a hop or jump slash takes no reaction until its first launch); Re:CoM's damage voice on an armoured
  hit (KH1 plays no reaction); the search finds Sora within its range (`sys 48`'s cone and `sys 139` not ported); every hit
  of Sora's as KH1's type 0; the apex motion a frame late (as the Powerwild); seeds per enemy; Re:CoM's sounds on the
  new clips; KH1's queue blends taken as the host's (as every port); Pirates appear in Neverland battles (Re:CoM's).
- **Ported as written:** the stance's hit count only grows when a hit comes without a state request (KH1's handler always
  asks, so it restarts the behaviour first); the slam's landing callback is overwritten before it is read.
- **Not ported:** the leash (KH1 removes it 200 m away), the death routine, the effects (the slash's 0x810 / 0x811, the
  spin's 14, the appearance's 2: M13), the appearance's flag 0x80, the fade-in (no Re:CoM alpha).

#### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`; a Neverland Pirate battle)

1. Each Pirate plays KH1's appearance (69, about 0.5 s), then wanders a few metres about where it stands, often taunting
   (220).
2. Within 10 m at its height it engages. Close (< 2.5 m): a small side hop and the long slash (214, turning to follow Sora,
   the hit late in the swing), the jump slash (a low long jump with a slash, then a big hop forward), or the spin. Mid
   (2.5–4.5 m): sometimes a big hop forward, then a side hop and the slash, or the spin. Far: a back hop, a run in and the
   leap (a 2 m jump at Sora, a slam on landing; mostly it recovers (213), sometimes its sword sticks (205–207)), or a laugh
   (212) and the spin.
3. The spin: a wind-up (222), then about 3 s spinning after Sora; when it hits him it stops a quarter second later, dizzy
   (219).
4. Hit it during its slash, counter, jump slash, slam or get-up slash: it takes the damage (Re:CoM's damage voice), slides
   back a little and keeps attacking (no flinch). Stop, Stun Impact, Blizzard's freeze and Gravity still take it over.
5. Hit it at other times: KH1's flinch; a finisher (a knock or a launch): its own launch, then it gets up slashing (235
   when hit from the front, 236 from behind) and spins.
6. After a hit or a clash, within 3 m: the block stance (208; Sora's swings from the front bounce off: recoil 0x6E with the
   Counterattack, it plays 209 and counters) or the counter slash (216); farther: the jump slash.
7. Swing into its spin from the front: Sora's strong recoil (0x6F; in the air 0x70), the Pirate plays 221 and counters;
   from behind the swing lands and it flinches.
8. Guard its attacks: KH1's recoil 215 (slash, counter, slam, get-ups), 211 (jump slash), 221 (spin). No Re:CoM "Guard"
   popup over it.
9. Kill one: KH1's reaction motion, Re:CoM's prizes and EXP.
10. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Pirates.

Please send `kh1-recom.log` (`pirate:` lines) and an F9 trace.

#### KH1 trace (what to record in KH1)

A Neverland room with Pirates (the ship's hold): stand far 10 s; stand at 3 m 10 s; stand at 1.5 m 10 s; one swing; a full
combo; a finisher (its launch and get-up slash, from the front and from behind); swing during its long slash and during
its spin (front and back); swing at its block stance (front and back); Guard its slash, its jump slash and its spin; let
the leap land near you; kill one.

#### Pirate owner questions

1. **The power of the attacks Re:CoM's Pirate lacks**: Re:CoM has its spin (id 0, Re:CoM's kind 1) and its long slash (id 1,
   Re:CoM's kind 7). Built by the kind of hit: the jump slash and the get-up slashes (flinching hits, KH1 power 2) take the
   spin's; the counter and the slam (knock-back hits, power 4) the slash's. (a) Keep, (b) all five on the slash's (id 1),
   (c) all five on the spin's (id 0).
2. **The Wyvern's "element reactions"** (found here): the Wyvern's hit handler calls the same `sys 42` for hits of
   attribute 2 / 3 / 4 (§ The Wyvern, left out as "KH1's element-based reactions"); `sys 42` is KH1's no-reaction path (a
   push slide, no reaction motion), so KH1's Wyvern doesn't react to those hits. (a) Leave it out (today: they react as
   any hit), (b) build it on this armour (the host would pass our spells' KH1 attribute to the behaviours: a framework
   change).
3. **The fliers' "doesn't flinch"** (Darkball, Air Pirate's dive, Wyvern's dive): their handlers zero the reaction after
   Re:CoM's handler has already ended the attack, so the attack stops there (KH1's goes on). The armour built here keeps an
   attack going. (a) Leave them, (b) move them onto it (one version each). **Answered (owner 2026-11-02): (b); built**
   (§ Hits that take no reaction, and each flier's section).

### The Large Body (built)

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Large Body yet. Its belly's front
row repels Sora's swings from the front through the shared enemy guard (KH1's mode-3 row, new), its flinch-type hits take
no reaction (§ Hits that take no reaction), its stomp's ring is Re:CoM's own shockwave record grown by KH1's numbers and
tested by KH1's ring rule inside Re:CoM's circle test (§ The stomp's ring, 2026-11-02).

#### KH1's side. Checked (files: `ex_2050.bd` disassembled, code from `0x458`, folded and raw; its child `ex_2050_00.bd`; decomp: the syscalls named)

The script (0x2600 bytes) has the Pirate's shape (the actor at `wp+0`, the state object at `wp+0x20`, the search object
at `wp+0x30`). One child object (`.mdls` part 3: `ex_2050_00.bd`, 0x480 bytes, entry `0x2F8`): the stomp's ring.

| Routine | Offset | What |
|---|---|---|
| entry | `0x1FF8` | init `0x20DC` (records LV1 `0x14` / LV2 `0x80`; stats `+4` walk 3.5, `+8` run 5, `+0x10` hop 20 (unused: no jump motion); `sys 125(8)`: 8° a frame; home; `wp+0x48` = 0, `+0x54` = 4, `+0x58` = 7; **the front row** `0x9CA`; handlers: hit `0x21BE`); a yield; the appearance thread `0x672`; each frame: while idle the search `0x11B8(&wp30, 0, 1500, 1)` (world 11: 10000 and always found) → engaged with the target; a state request restarts the behaviour (1 idle `0x61E`, 2 engaged `0x516`, 3 hit `0x458`) |
| appearance | `0x682` → `0x1B30` | spawn type ≠ 0 and ≠ 30: face Sora now (`sys 11`); 69 (a plain request, blend 0); the 30-frame fade-in; flag 0x80 for 30 frames; effect 2; 30 frames; actor flag 4 with the idle queued (bit 30, blend 0): the script waits until 69 ends; idle |
| idle | `0x636` | spawn type 30 (asleep): 211, 212 (not ported); else forever the shared wander `0x1422(300, 400, &home)` (points [200, 300) and [300, 400) about where it stands, 300 frames each, 40 % an idle of [90, 180), home beyond 700) |
| engaged | `0x53A` | the first look once (`0xC30`: `wp+0x54` = 5; 214, blend 0, to its end turning to the target; the idle at 8); the leash `0xBEA` (not ported); not enraged `0x5C8`: < 250 the positioning and the punch, < 750 the positioning and the slam, farther the positioning, the stomp and the slam; enraged: the charge `0x6D8`; the leash; the search (Sora); engaged again |
| positioning | `0xB0C` | walk (run once enraged): within 400 of the target to a random point 400–600 from where it stands (120–150 frames, the shared walk); farther than 450 the shared approach (240–300 frames, the target re-read every 5, until within 350) |
| punch | `0xAB4` | one hit bit; turning to the target; rows `0xEC` / `0x15C` (the two hands); action 200 (blend 8, the idle queued at 8); then 75 frames |
| slam | `0xA74` | row `0x1CC` (the belly); turning to the target; action 231 (blend 8); then 60 frames |
| stomp | `0xA08` | on the ground; turning to the target; 230 (a plain request, blend 0); 88 frames; effect 13; **the child** (`sys 94(actor, 0, target, 0x31C, 0x31C)`); until 230 is done |
| the ring | child `0x2F8` | effect 7; an attack object from the row `0x31C` (flag 64: no front cone) at the parent's position; for 28 frames each frame: every party member standing `[r − 100, r)` from that point (horizontally) and within 50 of its height (`0x18`) takes the attack (`sys 60`); then `r` += 25 × step (r starts 0: 0 … 675) |
| rage | `0xCC2` | effect 9; 217 (blend 8) to its end turning to the target; the idle at 8 |
| charge | `0x6D8` | on the ground; the wind-up 206 (blend 8) to its end turning to the target; the belly row `0x23C` (`sys 213(row, 2)`: lives while 207 plays; its callback `0x968`: `wp+0x58` = 10); **the front row removed** (`0x978`); `wp+0x58` = 6; for at most 180 frames while it is 6: at the target until within 100 of the floor under him (`0x12F6`), then on to a random point 300–500 from where it is until within 100 of it or stuck (moved under 3.6 × step × 0.5 last frame, `sys 118`); each frame `sys 12(207, 8)`, the stats block's `+0xC` = 12 and the glide (`sys 40` mode 2); ended when 207 no longer plays. Then the front row back (`0x908`), on the ground: its row hit Sora → action 208 (blend 0) and 45–120 frames; else action 209 (blend 8) and 30–150 frames; `wp+0x58` = 7 |
| hit state | `0x47C` | (target: the hitter or Sora) `wp+0x4C` = 2: not enraged: fewer than 2 counted hits (`wp+0x50`) the punch, else the stomp and the count back to 0; enraged the charge. `wp+0x4C` = 3: enraged, the rage. Then the leash and engaged |
| hit handler | `0x21BE` | `wp+0x54` = 5 (no first look after a hit); a clash (kinds 6 / 7) or a repel record (`hp[0] & 0x20`): during 207 the charge ends (`wp+0x58` = 9) and the front row comes back (`0x908`); else the count + 1 and the hit state (2). The damage (`0x1854` → `sys 44`): killed → its death `0x24EA` (the front row removed; KH1 then plays the hit's own reaction). Alive: **not enraged and HP / max under 0.4** (`0x1C1E`: stats `+0x3C` / `+0x40`) → `sys 42` (no reaction, the push slide), actor flag 4 cleared (`0x1DF6`: its action ends now), `wp+0x48` = 1, the hit state (3); **during 207** → the charge ends (8), the front row back, `sys 79` (unless attribute 1) and the hit state (2); **kinds 4 / 5**: during 230, 231 or 217 `sys 42`, else `sys 79` and the hit state (2); the count + 1 either way; **any other kind (a flinch): `sys 42` only**. Hits of attribute 8 / 9 / 10 / 11 (`0x1346`) take other paths (not carried by the host, as every port) |

`sys 79` (`0x1402BBF40` → `0x1402B77D0`): KH1's knock-back whatever the record's kind (on the ground `0x1402B8CC0`: 0x4C /
0x4D by side when the model has them, else the launch 0x52, else the flinch; in the air 0x50 / 0x51), the record marked
done (`*hit |= 2`). `sys 213` (`0x1402BBC10`): an attack object's mode (bits 2..3); mode 2 lives while the request it
first saw plays and its HIT slot is on, **mode 3 is tested every frame** whatever plays (`0x1402B47A0`), until mode 0
removes it. Checked (decomp).

Attacks (rows `0x70` bytes; Checked (files)):

| Attack | Request | Rows (bone code → Re:CoM bone, t, radius; HIT slot) | Hit windows | Recoil `+0x40` | KH1 `+8` / power `+0x14` | Re:CoM id | Re:CoM state |
|---|---|---|---|---|---|---|---|
| punch | 200 (158 f, action) | `0xEC` (25 → `r_foarm`, 1.3 → 0, 60; 0), `0x15C` (24 → `l_foarm`; 1) | right [54, 68), [84, 106); left [50, 72), [86, 100) | 215 | 1 / 6, priority 30 | 0 | 2 |
| slam | 231 (272 f, action, root) | `0x1CC` (12 → `hip1`, −0.3 → 0.5, 150; 0) | [102, 120) | 216 | 4 / 8, priority 30 | 0 | 2 |
| charge | 207 (24 f looped, steered) | `0x23C` (12 → `hip1`, −0.3 → 0.5, 125; 0) | [0, 24) each cycle | 218 | 4 / 8, **priority 90** | 0 | 2 |
| stomp's ring | 230 (182 f, steered) | the child's `0x31C` (no bone; type 6, priority 0) | the child's 28 frames from 88 | 110 (not in the model) | 1 / 6 | 1 | 3 |

The front row `0x38C` (bone code 12, −0.2, 100; HIT slot 0): **type 1, priority 100, recoil 110, cone turn 0 / cos
0.34202 (70°)**, made with mode 3. Row `0x2AC` (type 2, priority 99, cos −1) is never made (no push of its address).

**What the front does in KH1** (`0x1402B4A20` / `0x1402B4BC0` / `0x1402B4640`, Checked (decomp)): a type-1 row meets only
attack types 0 and 3 (Sora's swings; no magic: Fire / Blizzard / Thunder are types 5 / 6), only from inside its cone (the
3D cosine between its facing and the vector to the attacker ≥ 0.342: 70° either side, and up to about 70° above). Its own
side: `0x1402B4BC0` returns before any record when its priority is 100 (**a side at 100 is never moved**: no clash, no
repel, the row isn't marked met: it stays). Sora's side: his swing (the Kingdom Key's 30) is 70 under → **kind 7, the
strong recoil 0x6F** (in the air 0x70), no damage. A swing of 98 or more meets no clash at all (both 98+). So the Large
Body is hit only from behind or the sides beyond 70°, by magic, and during the charge (its row removed). Its hit handler's
repel branch (`hp[0] & 0x20`) never runs for this row.

Reactions: F (0x48..0x4B), K (0x4C / 0x4D), L (0x52.., 0x57..); no air flinch or air knock-back, no jump motions (3, 4,
5): it never leaves the ground of its own. Weight 1.0 in both records. Hurt parts: 9 (Re:CoM bones 39, 3, 17, 48, 18,
49, 15, 23, 54). Checked (files).

#### Re:CoM's side. Checked (exe + decomp)

`CEnemyLargebody` (vtable `0x140542AD0`, 139 slots, RTTI COL at −8), ctor `0x14018F090` (`40 53 48 83 EC 20`: `CHeartless`'s
base ctor, then `+0x2E0` = 0.1; its only reference the factory's call at `0x140189116`; no branch into the moved bytes, no
pointer to them: exe scan), allocation `0x1401890FD` `mov ecx, 0xCE0` → 0xD00. Its own states 1 (`0x14018F1F0` / the walk
thunk `0x14018F3E0`), 2 (its belly bump `e009_050`, `0x14018F150` / `0x14018F220`: forward at 0.35 until frame 92), 3 (its
stomp `e009_053`, `0x14018F1A0` / `0x14018F280`: a camera quake at frame 90, then `+0xCD0` grows 0.3 a frame and
`0x14014E5C0(+0x100, +0xCD0)` sets every record's radius until frame 130); 0 / 11 / 12 / 30 and the attack states' ends
`CBattleEnemy`'s; think `0x140196EC0`. `EN0009.ASD`: 2 records at `0x6C`, tail `0x1AC`: id 0 on `e009_050` (`ex_2050_sk`,
1.0 m, shape index 3, frames 92–130, kind index 1), id 1 on `e009_053` (Root, 0.3 m up, **shape index 4 = type 11
`CCircleCollision`** (`0x1407C1C98`: indices 0..4 → types 1, 2, 7, 8, 11; ctor case 11 `0x1401720E0`), radius 0.76,
frames 90–120, kind index 0). `EN0009.SES`: `e009_031`, `e009_033`, `e009_050`, `e009_053`. `EN0009.ANN`: 75 slots.

**Re:CoM's Large Body has no body guard**: the `CCollBody` ctor `0x14016F820` clears `+0x98..+0x9F` (and sets `+0x8C` all
elements, `+0xB0` π); the enemy body builder `0x140155160` never sets bit 0 of `+0x9C`; no Re:CoM code sets it on an enemy
body (decomp scan of every `+0x9C` write: only the player's guard, `player+0x72C`). Its front / back difference in
Re:CoM is a damage matter (Assumption: weak parts in its `.EPD`, kept by decision 13). So `enemy_ai = recom` leaves it
exactly as Re:CoM has it (nothing of ours touches a Re:CoM Large Body).

Our copy: states 0, 1, 2 (the punch, the slam, the charge), 3 (the stomp), 11, 12, 30, the think.

**Power mapping** (decision M10, "the Re:CoM attack of the same kind"): the asset step's matcher pairs Re:CoM's
`e009_050` with KH1's slam 231 and `e009_053` with KH1's stomp 230 (Re:CoM's 30 fps edits: their SES entries carry over;
Checked (files)): the slam → id 0, the stomp's ring → id 1 (Re:CoM's own shockwave). The punch and the charge have no
Re:CoM twin; both are body attacks (KH1 power 6 / 8, Re:CoM has no other melee record): id 0 (Assumption; owner question 2).

#### The front guard (KH1's mode 3 on the shared enemy guard)

- Framework (`kh1_core::enemy::guard::ALWAYS`): a guard row KH1 makes with mode 3 has no windows in the manifest (the
  asset step's `GUARDS` entry with no requests); `GuardRow::live(ALWAYS, _)` / `made_with(ALWAYS)` / `always()`.
  `Cmd::Guard(Some((row, ALWAYS)))` turns it on; `guard_step` keeps it on whatever plays; Re:CoM's states (Stop, stun,
  freeze: `guard_watch`, `take_over`) don't end it (KH1: `0x1402B47A0` skips only non-guard rows of an owner with `+0x88 &
  1`); the defeat does. A row at priority 100 also stays on through Re:CoM's own physical attacks (a turned-back shot, a
  summon: `guard_event`'s fallback is "neither" for it, not the kind-6 clash).
- The behaviour sends it at its first frame (KH1's init), `Cmd::Guard(None)` when the charge starts (`0x978`), and again at
  the charge's end or when a hit ends the charge (`0x908`, the handler's through `take_now`).
- Host (unchanged rules): Re:CoM's body guard on every hurt part (`+0x9C` bit 1, physical, half angle acos(0.342) = 70°);
  each KH1 swing that reaches a hurt part: KH1's 3D cone (`kh1_front`) and type rule, then `guard_after_swing`: Sora kind 7
  (`Kh1Sora::swing_clash`: 0x6F; 0x70 in the air), the Large Body "neither" (`guard_event`: no reaction, no news, the row
  stays). Re:CoM's "Guard" popup is hidden over it (0.76.2). Magic lands (our spells carry Re:CoM's spell attributes).
- `GUARDED` is given back when a runtime whose bodies still carry the flag is dropped (a battle left with a Large Body
  alive), so the per-swing / per-frame early-outs stay cheap.

#### The stomp's ring (KH1's child on Re:CoM's own shockwave)

- Asset step (`enemy::RINGS`, new): a copy of Re:CoM's id-1 record (its `e009_053` circle at its root; the step refuses a
  template that isn't shape index 4) on KH1's 230 clip (`e009_138`), radius 0, **its offset y (`+0x18`) 0** (Re:CoM's
  template sits 0.3 m up the root; KH1 measures the height from the stomp's own spot), on for frames [88, 116)
  (`ring_at` from the parent, `ring_life` from the child); the manifest's attack "stomp" names the child's row `0x31C`
  (recoil 110).
- Host (`kh1_enemy::Ring`, `win::kh1_enemy::ring_step`): while 230 plays inside those frames, the record's radius is
  KH1's `ring_growth` × (frame − 88) (0 … 6.75 m) through Re:CoM's own `0x14014E5C0(enemy+0x100, r)`, the call Re:CoM's
  Large Body makes for its shockwave (`sites::KH1_ENEMY_RING_CALLEES`: `40 53 48 83 EC 30 8B 41 14 48 8B 59 18`; it sets
  every record's radius in the set: Re:CoM re-reads a record's own radius when it starts, `0x14014E830` is a caller of the
  radius setter `0x140172690`: Assumption that the other records aren't live then; none are during 230). Re:CoM collides
  it, hits Sora with id 1's power, ends it. One option test a frame for every other class.
- **KH1's ring rule in Re:CoM's circle test** (owner 2026-11-02: research first; Re:CoM's is a disk, so KH1's ring):
  - Re:CoM's test (Checked (decomp)): the pair dispatch `0x14016EAD0` sends a type-11 circle attack against a sphere or
    column body (cases 0xB2 / 0xB4 / 0xB5) to `0x140177190` (its only caller; arguments: context, the body's shape, the
    circle). Against Sora's column (`player+0x690`, type 4: `+0x10` the bottom at his toes' mean height, `+0x20` his head
    + 0.2, both at his hips' x / z, `0x1401F4000`; radius 0.25) it hits when the column's axis at the circle's height is
    within circle radius + 0.25 m (strict) and the circle's height lies between the column's ends: **a solid flat disk**
    at the circle's centre `+0x10` (Re:CoM's last-frame place, `0x14014D740`), radius `+0x70`. On a hit it writes the
    contact point (`+0x50`, the circle's height, w 1) on the circle and the body.
  - **The wrapper** `win::kh1_enemy::circle_test` on `0x140177190` (`40 55 56 57 48 83 EC 50`, 8 bytes moved: whole
    instructions, no rip operand; no branch or pointer lands inside them: exe scan, pefile + capstone, 2026-11-02;
    `sites::KH1_ENEMY_RING_HOOKS`): a circle whose `+0x74` is above 0 is our ring; every other circle runs Re:CoM's test
    (the trampoline). For our ring: KH1's routine `0x18` (`ex_2050_00.bd`: miss if |Δy| > 50, then hit if r − 100 < d <
    r, d flat) on Re:CoM's body: the body's position point (a column's lower end; a sphere's lowest point, centre `+0x20`
    + lift `+0x74` − radius) within 0.5 m of the circle's height (inclusive), its flat distance from the circle's centre
    strictly inside (r − 1 m, r), **no body radius added**; the contact point as Re:CoM writes it (the body's x / z at
    the ring's height). Pure rule `kh1_enemy::ring_hits` / `body_foot` (tests `ring_rule`, `ring_body_foot`).
  - **Only our ring**: Re:CoM's factory `0x1401720E0` makes every circle with `+0x70` = 0.5, `+0x74` = 0; nothing else
    writes a circle's `+0x74` (Checked (decomp scan): every function that loads a collision holder's shape `+0x18` and
    writes its `+0x74`: only `0x140172690` (types 2 only for `+0x74`) and `0x140172720` (types 1 / 2); the other
    `+0x74` writers in the collision code (`0x140170B60`, `0x140172830`, `0x140172B90`, `0x1401731E0`, `0x1401753F0`,
    `0x140178F00`) write points, spheres or ride matrices, not a circle). `ring_step` writes the band (`ring_band` 100 →
    1.0 m) on its enemy's live type-11 entry each ring frame (Re:CoM remakes the entry's shape when the record's window
    opens) and the height (`ring_height` 50 → 0.5 m) for the wrapper. Re:CoM's own Large Body (`enemy_ai = recom`),
    its shockwave, Riku's story and every other circle attack: Re:CoM's test, unchanged.
  - **Per frame**: nothing new outside the hooked test itself, which runs only when Re:CoM's collision pass tests a
    circle (one float read for Re:CoM's circles); `ring_step`'s band write runs only during the ring's 28 frames.
  - Installed after the classes when one with a ring (the Large Body) is in; log `kh1 enemies: Re:CoM's circle test
    wrapped …`, and the first 20 ring hits `kh1 enemies: the stomp ring hit (… distance … height …)`.
- Differences from KH1 still standing: the record follows the Large Body's root (KH1's ring stays where it was made: it
  doesn't move during 230); a ring cut by a reaction ends with 230 (KH1's child runs its 28 frames); Re:CoM hits once per
  attack serial (KH1's child calls `sys 60` each frame: whether KH1 lands it more than once is not checked).

#### The hit handler on Re:CoM's states

- **No reaction** (`sys 42`): every damaging hit but a knock-back / launch outside 230 / 231 / 217, and none during 207,
  takes Re:CoM's super armour for the handler call (§ Hits that take no reaction): Re:CoM's damage, no state, KH1's push
  slide (`on_armoured_hit`). The decision needs the hit's KH1 kind *before* Re:CoM's handler: `armour_on` now sets it
  (`Behaviour::set_hit_kind`) from the KH1 swing waiting on the body (`attack::peek_kh1_kind`, `take_kh1_hit`'s test
  without taking it) or Re:CoM's kind of the hit info (`hit_of`'s rule) — the other classes ignore it.
- **The rage** after the damage: `armour_off` now passes Re:CoM's HP / max (`reads_hp`) before `on_armoured_hit`; the
  behaviour asks for the hit state and `Behaviour::ends_action` (new: KH1's flag 4 cleared): the host ends the action
  (Re:CoM's attack end in an attack state, our own leave to state 1; a plain action in state 1 stops being waited on; a
  running reaction plays on: Assumption, KH1's script would run during it). A rage hit on Re:CoM's reaction path (a
  knock-back kind) is `keeps_reaction` false (Re:CoM's handler already cut the action: KH1's flag 4).
- **KH1's knock-back** (`sys 79`): `reaction_kind` 4 (the shared `react` then plays 0x4C / 0x4D by side); the killing hit
  keeps its own kind (`on_defeat`).
- **"Is it charging"** reads the request playing when the hit arrives (`Behaviour::set_playing`, new: KH1's `sys 51`), not
  the AI's last frame (stale while an action plays).

#### What the asset step writes

`ENEMY/0009/EN0009.ANN` (Re:CoM's 75 slots byte-identical; KH1's clips at 100 + entry, 100..139: no exact Re:CoM bake),
`EN0009.ASD` (Re:CoM's 2 records kept + 7: the punch's two hands × two windows, the slam, the charge, the ring), `EN0009.SES`
(Re:CoM's 4 entries kept; the slam and the stomp take their Re:CoM twins' sounds, the punch and the charge their template's
at the hit window: Assumption), `kh1-largebody.json` (`kh1-enemy 1`: 65 numbers from `ex_2050.bd` at checked offsets, 4
from the child, 21 request words checked + 2 in the child, 4 attacks, the guard row `front` (no windows), 9 parts). Test
`largebody::tests::builds_from_the_players_files`. Other enemies' outputs are unchanged (their entries in `GUARDS`,
`RINGS`, `CHILD_CONSTS` are their own; the recoil rule relaxed only for a priority-100 row). In `mod.yml`:

```yaml
- {name: ENEMY/0009/EN0009.ANN, method: copy, source: [{name: ENEMY/0009/EN0009.ANN}]}
- {name: ENEMY/0009/EN0009.ASD, method: copy, source: [{name: ENEMY/0009/EN0009.ASD}]}
- {name: ENEMY/0009/EN0009.SES, method: copy, source: [{name: ENEMY/0009/EN0009.SES}]}
- {name: kh1-largebody.json, method: copy, source: [{name: kh1-largebody.json}]}
```

The step prints `largebody (M11): …`; the log shows `largebody: KH1's largebody (M11) …` at start, `battle N: Large Body …
gets KH1's AI` in a Large Body battle, `guard row 0 with 65535` (its front row, at the start and after each charge),
`its guard row stopped Sora's swing (priority 30 vs 100): Sora Clash { kind: 7, diff: -70 }, it None`, `its ring starts`,
`hit kind … without a reaction while … plays`, and `its hit handler ends the action` at its rage.

#### Checked / Assumption

- **Checked (files):** every script number at its offset (the asset test reads them), 21 request words and the child's 2,
  the rows (types, priorities, cones, recoils 215 / 216 / 218 in the model, 110 not), the hit windows, the hurt parts, the
  clip lengths, Re:CoM's records (shape index 4 for id 1) and clips, the matcher's pairs (231 ↔ `e009_050`, 230 ↔
  `e009_053`).
- **Checked (decomp):** `sys 79` / `sys 42` / `sys 213` / `sys 23` / `sys 94`, mode 3 (`0x1402B47A0`), the priority-100 rule
  and the cone (`0x1402B4BC0`, `0x1402B4640`), Re:CoM's state 3 shockwave (`0x14018F280` → `0x14014E5C0` → `0x140172690`
  type 11 `+0x70`), the `CCollBody` guard bit never set on Re:CoM's enemies.
- **Checked (exe):** the class, its 139 slots, the ctor and its only call, the factory size, the radius setter's bytes; every
  site (582 byte sites, 290 pointer sites, pefile, 2026-11-02).
- **Checked (decomp + exe), the ring's test:** `0x140177190` (a disk; its prologue bytes, no branch into them), its only
  caller `0x14016EAD0`, Sora's column (`0x1401F3360`, `0x1401F4000`), the circle factory and the `+0x74` writers (scan),
  the circle's centre `0x14014D740`, the window open `0x14014E830`. **Checked (files):** KH1's band 100 / height 50
  (`0x3CC` / `0x3D2`), the ring record's offset y 0 and bone `Root`, the stomp 230 without root motion (asset test).
- **Assumptions:** the punch and the charge on id 0; the ring's centre: Re:CoM's `Root` at the Large Body's own place on
  its floor stands for KH1's parent position point (230 has no root motion, so `Root` is the model's origin; that the
  origin sits on the floor is not checked), and Sora's column bottom (toes' mean height, hips' x / z) and a sphere's
  lowest point for KH1's actor `+0x50` (the feet, as `notes/hitbox-compare.md`); the first frame of the ring: if
  Re:CoM opens the record's window after `ring_step` in frame 88, that one frame tests Re:CoM's way at radius 0 (a hit
  only within 0.25 m of its root, inside its own body; not checked which runs first); the ring on the Large Body's root; the battle floors flat (the charge's floor point `0x12F6` is its own height); the search finds Sora
  within its range (`sys 48`'s cone and `sys 139` not ported); every hit of Sora's as KH1's type 0 / attribute 0 (the
  attribute 1 exception of the charge's knock-back never comes); the rage during a reaction waits for its end; seeds per
  enemy; Re:CoM's sounds on the new clips; the queue blends the host's.
- **Not ported:** the leash (`0xBEA`: 18 m / 10.8 m out of sight), the sleeping spawn (30), the death routine, the effects
  (2, 7, 9, 10, 13, 14: M13), the appearance's flag 0x80 and fade-in. Sora's swings against its damaging rows: built
  (owner question 3 answered (b), § Attack meets attack (clashes)).

#### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`; any Large Body battle)

1. Each Large Body plays KH1's appearance (69, about 1.7 s), wanders a few metres about where it stands, and turns to
   look at Sora (214) the first time it notices him (within 15 m).
2. Swing at it from the front (within about 70° of where its belly faces): Sora bounces off hard (the strong recoil), it
   takes no damage and doesn't react; no "Guard" popup. From behind or well to the side the swings land.
3. Hit it from behind with ordinary swings: the damage lands, it slides back a little and carries on (no flinch). A combo
   finisher (a knock-back or launch) knocks it back (its own knock-back), then it counters: a punch (both fists), and after
   two such hits the stomp.
4. Close (< 2.5 m): it steps away 4–6 m first if Sora is within 4 m, then punches; mid (< 7.5 m): the belly slam (a big
   forward belly flop, about 1.7 s in); far: it walks in, stomps (230: after about 1.5 s a shockwave ring spreads 6.75 m
   along the floor over half a second) and then slams.
5. Bring it under 40 % HP: it stops whatever it is doing, roars (217, about 2 s), and from then on only charges: a wind-up
   (206, 1.5 s), then it runs at Sora fast (12 a frame) for up to 3 s, past him to a spot 3–5 m on; during the charge
   its front no longer blocks (swings from the front land) and any hit knocks it back; if the charge hit Sora it stops
   (208) and pauses, else it recovers (209, 5 s).
6. Guard its punch, slam or charge: KH1's recoil (215 / 216 / 218) and its counter; Guard the ring: Re:CoM's flinch.
6a. The stomp's ring (asset step re-run): jump as its edge reaches Sora: it passes under him (a jump of more than 0.5 m
   clears it). Stand still inside it after its edge has passed, or close to the Large Body while it stomps: no hit
   (only the 1 m wide edge hits). Standing on its path on the floor: hit as the edge passes. The log shows `the stomp
   ring hit (KH1's ring: …)` with the distance and height.
7. Stop, Stun Impact, Blizzard's freeze and Gravity still take it over; its front still blocks swings while it is stopped.
8. Kill one: KH1's reaction motion, Re:CoM's prizes and EXP.
9. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Large Bodies (no change).

Please send `kh1-recom.log` (`largebody:` lines) and an F9 trace.

#### KH1 trace (what to record in KH1)

A KH1 room with Large Bodies (Traverse Town after Guard Armor, for example): stand far 10 s;
stand at 5 m 10 s; stand at 2 m 10 s; swing at its front once and a full combo (front), then from behind (one swing, a
full combo, a finisher); stand on the ring's path and jump over it; bring one under 40 % HP and let it charge you, then
dodge one charge; Guard a punch, a slam, a charge and the ring; swing into its belly while it charges (front and back);
kill one. The trace's enemy rows give the rounds' timings, the ring's reach and the charge's path to compare.

#### Large Body owner questions

1. **The stomp's ring shape.** Answered (owner 2026-11-02: research first): Re:CoM's is a disk, so KH1's ring is built in
   Re:CoM's circle test (§ The stomp's ring). The question was: KH1's ring hits Sora only where its edge passes (`[r − 1 m, r)` from the stomp's spot) and
   only within 0.5 m of its height (jumping clears it). Built: Re:CoM's own shockwave record (its Large Body's circle) grown
   by KH1's numbers (25 a frame for 28 frames from frame 88): KH1's reach and timing, Re:CoM's shape (its test isn't read
   yet; a jump may not clear it). (a) Keep; (b) add KH1's band and height test as a filter on that record's hits on Sora
   (Re:CoM still collides it; we only refuse a hit KH1 wouldn't land); (c) research `CCircleCollision`'s test first and
   decide then.
2. **The power of the punch and the charge.** Re:CoM's Large Body has only its belly bump (id 0, the twin of KH1's slam)
   and its stomp's shockwave (id 1, the twin of KH1's ring). Built: the punch and the charge on the belly bump's power.
   (a) Keep; (b) the charge on id 0, the punch at a fraction; (c) another rule.
3. **Sora's swings against its attacks.** Answered (owner 2026-11-02: (b), every ported enemy): built, § Attack meets
   attack (clashes). The question was: KH1 tests attack against attack: Sora's swing meeting the charge's belly sphere
   (priority 90, all round) gets the strong recoil and ends the charge (the Large Body gets a clash: 209); meeting a punching
   fist (30 against 30) both recoil. The framework builds this only for guard-type rows (and the Bandit's blade), so here a
   swing that reaches a charging Large Body hits its body. This is every enemy's damaging rows, not only the Large Body's.
   (a) Leave it (as every port so far); (b) build it in the shared enemy guard (each damaging row also as an all-round
   clash while its window is on), research first which test KH1 runs first when a swing meets both the row and the body.

### The Neoshadow (built)

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Neoshadow yet. Its attacks run on
Re:CoM's states 2 / 3; its body row bounces Sora's swings while it leaps, dives and rises slashing (the shared
enemy-guard step, a row held in KH1's mode 3); the hits it takes no reaction for keep Re:CoM's state (§ Hits that take no
reaction).

#### KH1's side. Checked (files: `ex_2420.bd` disassembled, code from `0x36A`; decomp: the syscalls named)

**Where KH1 has it.** Of the player's 214 KH1 rooms only `ew29.ard` (End of the World) names `ex_2420`
(Checked (files): `grep` of every `.ard`; KH1's world 16 is `ew`, the world code table `0x1404DCCA0`, Checked (exe)). The
other End of the World / Hollow Bastion / Olympus rooms' black Heartless are `ex_2310` / `ex_2312` (another model and
script). Re:CoM's `CEnemyNeoshadow` is built from `ex_2420` (Checked (files): `EN0027.MDL`'s root bone `m_ex420_sk`,
`EN0027.ASD` names `ex_2420`). The script names points and a height of room 29 (below).

**The shared block.** All the room's Neoshadows share one 0x80-byte block (`sys 138` → `0x1402BC620`: `DAT_142DB7B5C`,
zeroed by the room set-up `0x140287C60`; Checked (decomp)). Its first word counts them (each init adds one, each death
takes one); most choices go by that count. The port: one `Group` per Re:CoM battle (`Behaviour::set_battle`).

| Routine | Offset | What |
|---|---|---|
| entry | `0x443E` | init `0x46F8` (stat record `0x14`, run 12, the block's words `+4..+0x4C` cleared, one more counted unless one is calling `+0x50`: then a called one, hidden and unhittable); a yield; a called one comes up (`0xFC8`), the others idle (`0xF34`); each frame a state request restarts the behaviour (1 idle, 2 engaged `0x724`, 3 `0x680` (never asked), 4 dive `0x5C6`, 5 hit `0x3DE`); nothing else while it travels underground (`wp+0x50`); KH1's grab holding Sora sends a free one to state 4; the last counted one's death makes a called one vanish |
| idle | `0xF34` | the search (Sora within 5000; at most four of them: Sora anyway), facing him now, engaged |
| engaged | `0x724` | **five or more** (`0xBCC`): unless a gathered dive is on, the gathered dive; then 3 rounds. **Three or four**: join a gathered dive that waits, else the formation (`0x1FB0`). **Two**: the first to come goes underground for good (`+0x48`, `wp+0x48`: sink, hidden, unhittable) and ambushes Sora (`0xBA2`: [120, 180) frames, then `0x1AF6`); the other fights 3 rounds. **One**: `0x8FA` |
| rounds | `0xC3A(n)` | < 500: 50 % the claw; sink, circle underground 600–1000 away and come up; with others, run to a point 800–900 away (180 f), alone 50 % the rising slash. < 900 (counts two): two or fewer 50 %, or alone: 50 % the rising slash, else a dive; else strafe (3 a frame, to 300, 150 f) and the approach (run, 180 f, re-aim 20, to 300). Farther: over 40 % or more than two: run at Sora (240 f, to 150, at 12) and the claw; else alone a dive, else sink, come up near Sora (200–300) and the rising slash. A gathered dive waiting ends them |
| claw | `0x1878` | rows `0x1D0` / `0x240` (both hands), 202 (blend 8, root motion), idle queued; turning to Sora while more than 40 of its len + 6 frames are left |
| rising slash | `0x1568` | sink; hidden; 20 f; Sora on the floor: straight under him (`sys 86`), else underground to within 50 of him (speed 30); the body row (`0x80`, mode 3) on, shown, facing Sora now, row `0x160` and 207 (blend 0), hittable; 70 f before 207's end the body row ends |
| sink | `0x5A98` | 216 (blend 8), 217 queued; no reaction (`wp+0x40`) until frame 40 + 6, then KH1's flags 0x20 / 0x40 / 0x10 (unhittable) and the height 1 → 0 over 20 f |
| come up | `0x596A` | 218 (blend 0), idle queued; height 0 → 1 over len − 2; then hittable |
| underground | `0x552C` / `0x55D2` / `0x5652` | 180 f at 12 (30 to Sora); straight at a point, or (the rounds' sink) along its own facing turned 4° a frame (it circles; the side flips every 60 f); stuck (from 30 f, moved under speed × 0.25): turn toward room 29's spot `(8700, 5300, 100)` ±60°, 700–1000 away |
| leap | `0x1950` / `0x3036` | the crouch 203 (blend 8), 204 queued; in its last 14 of len + 6 frames: movement state 7 (`sys 169`, no velocity), the body row on, rising at 20 a frame to 120 above the floor (`sys 89`) |
| gathered dive | `0x3036` | the leap; one more up (`+4`); the dive starts when all are up or there are at most four (`+0xC`); 30 f; one diver at a time (the token `+8`) |
| dive | `0x3326` | aim: Sora (80 above him when he is on the floor or within 80 of it), 1.4 × as far, 800 when under 500 (`0x3664`); 205 (blend 8), 8 f, row `0xF0` (mode 2); for len × 9 frames flying at the aim at 80 a frame (re-aimed farther than 600 from Sora; level under 80 above the floor), stopping within 250 of the aim or (from 30 f) when it moved under 40; then the easing 0.4 for good (stats `+0x14`), the body row off, 206 with 6 queued for len − 4, movement state 0 (`sys 88`), on the floor; higher than room 29's height 5380 it runs home 60 f (`0x372C`) |
| formation | `0x1FB0` / `0x2430` | sink, hidden, 20 f; each sunk one counts (`+0x10`); all sunk (or two or fewer): go (`+0x14`); 6 f; a free place of a 9-place layout of room 29 (by Sora's x: < 8250, > 9200, else the middle one; each frame a draw in [1, 10) takes the first free place at or after it); 20 f; the rising slash out of it, all together |
| two left | `0xBA2` → `0x1AF6` | while Sora is off the floor (≤ 180 f, unless alone or 13 m away), wait; 20 f under Sora, facing his way; then **KH1 grabs him** when he stands on the floor and can be hit: he plays motion 200 from the Neoshadow's own `.mset` (`sys 113`, `0x1402BA8C0`; request `0x10000C8`) while it plays 208 and another Neoshadow dives at him (`+0x4C` → state 4); else it misses: at his feet, shown, 209, hidden again |
| alone | `0x8FA` | the hidden one: the block cleared, to room 29's spot `(9000, 5300, 100)`, 30 f, shown, come up, the call; then while alone: underground (straight) to `(8900, 5300, 100)` (other worlds: `(−100, 0, −1200)`, never in KH1), the call `0x11FA` (210, 240 + 6 f; in its last 80 KH1 wakes the room's sleeping objects 600..606, `sys 37` with `0xB0000 | id` → `0x140290D60` → `0x140287850`), 5 rounds |
| a called one | `0xFC8` | (made while one calls) shown, facing Sora, 211, 212 four times its length, counted, action 213 with idle queued, idle |
| hit state | `0x3DE` | after a counter mark (`wp+0x3C` 100: a knock-back / launch, or the 4th flinch in a row): with other than three or four and no gathering, two or fewer 50 % the rising slash, else sink away 600–1000; else (no gathering) five or more the claw, three or four turn to Sora, two or fewer the rising slash; then engaged |
| hit handler | `0x498C` | a called one rising (`wp+0x54`): nothing, before the damage; the hidden one (`wp+0x48`) or the body row on (`wp+0x58`): no reaction; a killing hit by Sora outside the sink: death (`0x4E1C`: one fewer counted; the last: the called ones vanish; the drops) and 72 facing the attacker (`0x5304`); a killing hit during the sink (216) or by another: HP 1; the call (210) or `wp+0x40`: no reaction; 213: KH1's push only (`sys 42`); kinds 4 / 5 mark the counter; kind 3 counts the run, the 4th marks it and `wp+0x40`; the hit state |

Attacks (KH1's rows; `0x1402B2DC0`, modes `0x1402B47A0` / `sys 213` `0x1402BBC10`: mode 2 lives while its request plays and
its HIT slot is on, **mode 3 every frame** until `sys 213(row, 0)`; Checked (decomp)):

| Attack | Request | Rows (bone code, t, radius; HIT slot) | Hit windows | Recoil | KH1 kind / power | Re:CoM id | How it plays |
|---|---|---|---|---|---|---|---|
| claw | 202 (74 f, root) | `0x1D0` (28, 50; 0), `0x240` (29, 50; 1) | [40, 54), [52, 68) | 215 | 1 / 4 | 0 | plain request (steered), state 2 |
| rising slash | 207 (120 f) | `0x160` (8, a line 1 → 0, 150; 0) | [26, 50) | 110 (none) | 1 / 8 | 1 | plain request, state 3 |
| dive | 205 (40 f, looped) | `0xF0` (8, 60; 0) | [0, 40) each cycle | 110 (none) | 4 / 6 | 1 | plain request, state 3, its body in movement state 7 |

The body row `0x80` (code 8, radius 80, **type 4, priority 100, cone cos −1: all round**, mode 3): Sora's swings meet it
(type 0 meets 4) and get KH1's strong recoil (kind 7: 70 under); a side at 100 is never moved, so the Neoshadow hears
nothing (`0x1402B4BC0`). On while `wp+0x58` (the leap, the hover, the dive, and the rising slash's first 50 frames).

Reactions: the shared engine with its motions (flinch 72..75 = 0x48..0x4B, knock-back 76 / 77, air flinch 78 / 79, air
knock-back 80 / 81 = its 78 / 79; no launch: a launch is a flinch or knock-back). Weight 1.0. Hurt parts: 7 (Re:CoM bones
30, 2, 3, 7, 12, 14, 54). Checked (files).

#### Re:CoM's side. Checked (exe + decomp)

`CEnemyNeoshadow` (vtable `0x140545128`, 139 slots, RTTI `.?AVCEnemyNeoshadow@@` at −8), ctor `0x140190150` (`40 53 48 83 EC
20`: hides it, `+0x154 |= 1`; its only reference the factory's call at `0x14018940A`; no branch into the moved bytes, no
pointer to them: exe scan), allocation `0x1401893F1` `mov ecx, 0xCE0` → 0xD00. Its own states: 0 (`0x140190220`: shown, its
rise `e027_011`, hurt and attack bodies off; step `0x140190480`: turning to Sora, the default state at the motion's end;
end `0x140190400`: bodies on), 1 (`0x1401903E0` / `0x140190860`), 2 (its claw `e027_050`, `0x140190330` / `0x140190540`), 3
(`0x140190380` / `0x1401905A0`: sink `e027_051`, come up 2 m from Sora at a random angle with `e027_052`, then `e027_053`);
11 / 12 / 30 and the attack states' ends `CBattleEnemy`'s; think `0x140196EC0`. Our copy: states 0, 1, 2 (the claw), 3 (the
rising slash, the dive), 11, 12, 30, the think. `EN0027.ASD`: 3 records at `0x6C`, tail `0x24C` (id 0 on `e027_050`: both
hands, 0.35 m; id 1 on `e027_052`: `m_ex420_sk`, 1.0 m from frame 42). `EN0027.ANN`: 73 slots. KH1's appearance: none (a
room Neoshadow just stands there): `Appear::Base` (Re:CoM's state 0 begin, our frame from the next, the default state).

**Power mapping** (decision M10; "the Re:CoM attack of the same kind"): the matcher pairs none of KH1's attacks with Re:CoM's
clips (no edits of 202 / 207 / 205). By kind: the claw 202 (both hands) → Re:CoM's claw (id 0); the rising slash 207 (out of
the floor under Sora) → Re:CoM's come-up strike (id 1); the dive 205 (no Re:CoM twin) → id 1 (Assumption; owner question 4).

#### Framework additions (M11, the Neoshadow)

| Part | Where | What |
|---|---|---|
| `Behaviour::set_battle` | `kh1-core` `enemy/mod.rs`, `win::kh1_enemy::ctor` | the battle number right after `make`: scripts sharing KH1's room block share it per battle |
| `Behaviour::death_motion` | `defeat_begin` | a script's own death motion in place of the killing hit's reaction (the Neoshadow's 72: its handler zeroes the reaction) |
| `guard::HELD` | `enemy/guard.rs`, `guard_step` | a guard row in KH1's mode 3: on every frame until `Cmd::Guard(None)`; the asset step's `GuardSpec` with no requests (no windows) |
| priority 100 | `guard_event` | a row at 100 is never moved by any attacker (Re:CoM's own attacks too); the asset step accepts its missing recoil |
| host model | `enemy/host.rs` `model` | a body on the ground state above the floor falls (Re:CoM's mover gravity) |

The Shadow's sunk-state machinery is reused as it is: `Cmd::Hittable` (Re:CoM's hurt-body switch), `Cmd::Height` (Re:CoM's
model scale y), `Cmd::Underground` (log), `Cmd::Visible` (Re:CoM's hide bit). Owner decisions for the Shadow apply: sunk =
unhittable (from the sink's frame 46 to the come-up's end), lockable stays, Sora's body push not mapped.

**No fixed points of KH1's room 29** (owner, 2026-11-02, question 1: "drop room 29's fixed spots: no fixed points, they
wander and attack instead"). The script names room 29's points: the formations' 27 places, the spot the hidden one comes up
at `(9000, 5300, 100)`, the spot a lone one goes to `(8900, 5300, 100)`, the stuck traveller's spot `(8700, 5300, 100)`, the
height 5380. None is used as a point now; every other part of the script (counts, timings, the call, sinks, rises, dives) is
kept:

| KH1 (room 29) | The port | Tag |
|---|---|---|
| alone `0x8FA`: underground straight to `(8900, 5300, 100)`, come up, the call | underground straight to a point 600–1000 about its own place (the script's own sink-away rule `0x552C(actor+0x10, 600, 1000)`, `near_away_min` / `near_away_max`), come up, the call | Assumption (nearest KH1-native rule) |
| the hidden one alone: `sys 86` to `(9000, 5300, 100)`, 30 f, come up, the call | `sys 86` to a point 600–1000 about its own place (same rule), 30 f, come up, the call | Assumption |
| the formation `0x2430`: a layout by Sora's x in room 29, `sys 86` to a free place of it | the place draw kept (KH1's frames and random numbers, the group's words), no layout, no move: each rises where it sank, all together, facing Sora at once (the rise's own `0x403C`) | Assumption |
| the stuck traveller `0x58AE`: turn toward `(8700, 5300, 100)` ±60°, 700–1000 away | turn toward Sora ±60°, 700–1000 away (no target: back the way it faces) | Assumption (owner question 6) |
| the walk home after a dive `0x372C`: higher than 5380 (100 above room 29's floor 5480), walk to its home (`wp+0x10`, its own spawn) 60 f | kept: more than 100 above the battle floor, walk home (its own spawn point, not a room point; never on Re:CoM's flat floors after the landing) | Assumption (flat floors) |

The point numbers are still read from the player's script (the manifest is unchanged: the asset step need not be re-run).
Checked (files): every use of the point numbers in `ex_2420.bd` (`0x58AE`, `0xA32`, `0xAE6`, `0x2430`..`0x28DC`, `0x3736`)
listed with `tools/kh1-ai-script.py`.

#### What the asset step writes

`ENEMY/0027/EN0027.ANN` (Re:CoM's 73 slots byte-identical, KH1's clips at 100 + entry; 67 plays Re:CoM's exact bake
`e027_071`), `EN0027.ASD` (Re:CoM's 3 records kept, 4 added: the claw's two hands (id 0), the rising slash (id 1, a 1.5 m
line), the dive (id 1, 0.6 m)), `EN0027.SES` (Re:CoM's entries kept; KH1's attack clips take their template record's sounds
at the hit window: Assumption), `kh1-neoshadow.json` (`kh1-enemy 1`, 207 numbers read at their pushes, the shield guard row).
Test `neoshadow::tests::builds_from_the_players_files`. In `mod.yml`:

```yaml
- {name: ENEMY/0027/EN0027.ANN, method: copy, source: [{name: ENEMY/0027/EN0027.ANN}]}
- {name: ENEMY/0027/EN0027.ASD, method: copy, source: [{name: ENEMY/0027/EN0027.ASD}]}
- {name: ENEMY/0027/EN0027.SES, method: copy, source: [{name: ENEMY/0027/EN0027.SES}]}
- {name: kh1-neoshadow.json, method: copy, source: [{name: kh1-neoshadow.json}]}
```

The step prints `neoshadow (M11): …`; the log shows `neoshadow: KH1's Neoshadow (M11) …` at start and `battle N: Neoshadow …
gets KH1's AI` in a Neoshadow battle.

#### Checked / Assumption

- **Checked (files):** every script number at its offset (the asset test reads the 207 from the player's script; names with
  several pushes agree), 27 request words, the rows (types, priorities, recoils, radii, slots), the hit windows, the hurt
  parts, Re:CoM's records and clips, the only KH1 room with the model.
- **Checked (decomp):** `sys 138` and the room set-up's clearing; `sys 213`'s modes; `sys 169` / `88` / `86` / `89` / `118`;
  `sys 37` / `sys 4` with `0xB0000 | id` (no such object in Re:CoM's battles: nothing); `sys 113` (Sora takes a motion of
  the enemy's `.mset`); `0x140299D60`'s mode (`& 0xFFFF`); the reactions read stats `+0xC4`, not `sys 124`'s block; KH1's
  `+0x374` bit 3 (`sys 163`) is "drawn this frame" (`0x1402A14F0`).
- **Checked (exe):** the class, its slots, RTTI, the ctor and its only call, the factory size, the world code table; every site
  (911, pefile, 0 mismatches, 2026-11-02).
- **Assumptions:** the power of the dive (owner: id 1, as built); no fixed points (owner): a lone one and the hidden one go to
  a point 600–1000 about their own place, the formation rises where each sank, a stuck traveller turns toward Sora, heights
  from the battle floor (room 29's floor 5480); the grab not built (the miss instead); the call wakes nothing; a called one unhittable while it comes up (KH1 takes
  the hits without damage: Re:CoM can't); KH1's `+0x90` (a dive's other end) never set (not identified); `sys 124` (written 0
  / 100 around its attacks) not ported (not identified); a killing hit during the sink's first frames kills (KH1 keeps it at
  1 HP: Re:CoM's defeat); the dive's row on from 205's first frame (KH1: 8 frames in); the miss faces its own way (KH1:
  Sora's); the script's world taken as 16 (room 29); KH1's `still` facing commands only for the host's quiet watchdog during
  the group's long waits; per-enemy random streams; Re:CoM's sounds on KH1's clips; effects and fades (M13).
- **Not ported:** the grab (researched, § The grab from below; owner question 2), the room's sleeping objects, the effects, the masks, the rumble, the fade-outs.

#### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`; any Neoshadow battle)

1. Neoshadows stand where they appear (no rise) and come at Sora at once.
2. **One Neoshadow:** it sinks, slides underground 6–10 m from where it was (no fixed spot), comes up and raises its arms for
   about 4 s (KH1's call; nothing comes in a Re:CoM battle), then fights: claws, sinks and circles underground, comes up
   slashing right under Sora, leaps up and dives at him. Then again, each time from where it is.
3. **Two:** one sinks and stays hidden; every 2–3 s it pops up at Sora's feet (209) and vanishes; it can't be hit. The other
   fights. Kill the other: the hidden one comes up 6–10 m from where it last was and does 2.
4. **Three or four:** they all sink together and rise slashing out of the floor at once, each where it sank (no formation
   places).
5. **Five or more:** they all leap up and hover; once all are up they dive at Sora one after another.
6. Swing at one while it leaps, hovers, dives or rises slashing: Sora bounces off (his strong recoil); no damage.
7. Hit it with plain swings: KH1's flinch; after a flinch, two or fewer of them: it sinks and comes up slashing under Sora. The
   fourth flinch in a row or a finisher: no reaction until it counters.
8. Guard its claw: KH1's recoil 215; its rising slash or dive: Re:CoM's flinch. No Re:CoM "Guard" popup over it.
9. Kill one: it turns to Sora and plays its death (72); Re:CoM's prizes and EXP.
10. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Neoshadows.

Please send `kh1-recom.log` (`neoshadow:` lines) and an F9 trace.

#### KH1 trace (what to record in KH1)

End of the World, the room with the Neoshadow ambush (KH1's `ew29`): stand still 10 s at the start (the count's behaviour);
stand at 3 m and at 8 m 10 s each; one swing; a full combo and a finisher; swing at one while it dives and while it rises
slashing; Guard its claw and its dive; kill them one by one down to two (the hidden one) and to one (the spot and the call);
record each count once.

#### The grab from below: research (owner choice 2, "build the grab from below natively"; not built: owner question 2)

**KH1's grab** (`0x1AF6`, two left, the hidden one; Checked (files: `ex_2420.bd` listed from `0x36A`, `xa_ex_2420.mset`
parsed with `kh1-formats`) + decomp as named):

| What | KH1 | Tag |
|---|---|---|
| Wait | while Sora is off the floor (`Sora+0x70` = 21 or ≠ 0), unless one is left, he is 1300+ away or 180 frames passed | Checked (files) |
| Follow | 20 frames at Sora's place (`sys 86`), facing his way (`sys 11` with his `+0x30`), its hand effect on | Checked (files) |
| Grab test | `0x3FAA(Sora)` (`sys 141` and `sys 218` and his flag `0x400` clear), his height 0 (`sys 89`), `Sora+0x70` = 0, flag `0x400` clear; else the miss (209 at his feet + 8, shown, hidden again: built) | Checked (files); the meaning of `sys 141` / `218`: Assumption (alive / hittable) |
| Sora | `sys 113` (`0x1402BA8C0`): his second motion set = the Neoshadow's player-set table (`.mset` `+0xA4B00`, ten entries, his at `+0xA4B80`); request `0x10000C8` (bit 24 = KH1's action lock, `0x14029F980` sets actor flag 4; request 200 of that set: **126 frames, no root motion, 293 bones**), blend 4; queued `0x40000000` (the lock's release) blend 8 | Checked (files + decomp) |
| The Neoshadow | the group's grab word `+0x4C` = 1; shown; `sys 124(0)`; at Sora facing his way; effect 6; **208 (126 frames)**; if its `+0x90` = 1 Sora is moved to it once (not identified, as the dive's); the pad rumble (`sys 171`, pattern at `0x318` → `0x1401E1400`) | Checked (files + decomp) |
| Hold | for len(208) − 2 frames it stays at Sora's place facing his way; it lets go when 30 or fewer are left (**94 frames held**) or when Sora's motion is no longer 200 (a hit's reaction replaced it); rumble off (`sys 196` → `0x1401E1490(0)`); `+0x4C` = 0; effect off; `sys 124(100)`; hidden | Checked (files + exe bytes of `sys 196`) |
| Damage | **none from the grab**: `0x1AF6` makes no attack row. The damage is the others': while `+0x4C` is set the supervisor (`0x453C`) sends each free Neoshadow (not hidden, not leaping / diving / sinking / rising, on the floor) to state 4 (`0x5C6`): a dive at Sora | Checked (files) |
| Escape | none in the script; Sora is action-locked (bit 24) until 200 ends (the queued release) or a reaction replaces it | Checked (decomp `0x14029F980`); a damaging hit on a locked Sora still reacting: Assumption (KH1's reactions request their motion through the same call, its callers `0x14029CC40`.., Checked) |

So the grab holds Sora for up to 126 frames with KH1's own held clip and no damage, and the second Neoshadow's dive (its leap
alone is 66 + 6 frames) is what hurts him and frees him.

**Re:CoM's player reactions** (Checked (decomp + exe), `CPlayerSora` vtable `0x1405A9938`): Re:CoM's damage check
`0x1401E6DE0` dispatches the hit's kind (`player+0x710`): flinches 1 / 2 / 3 / 0x1C..0x1E / 0x22 / 0x23 (action 0xC, rows
0x1F..0x23), the launch 6, the card duel 0x21 (action 0x24), and the statuses, all into the status action 0x18 (init
`0x1401F1200`, exe `0x1401EB0F0`, by `player+0x420`): stun (kind 7 / 0x1F, bit 0: row 0x27 `p001_036`, 9000 ticks = 180
frames, every button press −100), Vexen's freeze (0xC, bit 0x20: 15000 ticks, the `vexen` effect, presses), Stop (8, bit 2:
the hit's own length, the `abnormal` effect, presses), bit 16 (0x1B: row 0x20 held), bit 23 (0x11: row 0x21, fainting eyes),
bit 27 (0x20: motion 0xFCB). Sora's actions 0x20 / 0x21 / 0x22 are empty (`ret`). No Re:CoM enemy or boss holds Sora: the
enemy code that moves him (calls of Re:CoM's place / angle setters `0x1401A6E00` / `0x1401A6D10` / `0x1401A8520` on Sora)
places him for a battle's scripted start (event variable `0xE1`: `CBossBoogie` `0x1403D9250`, `CBossParasiteCage`
`0x1403C3090` / `0x1403C46A0`, `CBossTrickMaster` `0x1403CC800`, `CBossDarkSide` `0x1403E6EF0`) or keeps him on its arena's
floor heights (`CBossBoogie` `0x1403DB7F0`); `CBossZexion` `0x14041FBB0` (Riku's story only) not read. A hit during Stop, bit 16 or bit 27 takes no reaction (`+0x420 & 0x8010002`,
the hold goes on); during stun, the freeze or bit 23 the hit's reaction plays and then the status comes back (the reaction's
end checks `+0x420 & 0x8810023` → `0x1401F1200`, the timer set again).

**What is missing in Re:CoM** for KH1's grab: a player reaction that (1) plays a clip the enemy supplies (KH1's held clip, a new
Sora motion slot), (2) locks Sora's actions for that clip with no button escape, (3) ends at the clip's end, (4) gives way for
good to any damaging hit's own reaction, (5) does no damage, (6) is started by the enemy through a hit (Re:CoM: a 0-damage
attack record on the Neoshadow carrying a reaction kind). Re:CoM's stun / freeze / Stop give (2) only partly (presses
shorten them), never (1), and (4) differently (they come back after a hit, or take no reaction). KH1-combat Sora takes Re:CoM's
reactions through the damage check (action 0xC; `docs/recom-kh1-attack.md` § Post-hit invulnerability); that our KH1 frame
leaves Re:CoM's status action 0x18 alone as it does 0xC: Assumption (not traced).

Nothing built: the miss stays (owner question 2).

#### Neoshadow owner questions

1. **Room 29's points.** Answered (owner, 2026-11-02): no fixed points, they wander and attack instead. Built (§ No fixed points
   of KH1's room 29).
2. **The grab from below (two Neoshadows).** Owner (2026-11-02): "build the grab from below natively". Research (§ The grab
   from below): Re:CoM has no reaction that holds Sora with an enemy's clip and lets a hit free him (its stun, Vexen's freeze and
   Stop are the nearest), so nothing is built yet; KH1's miss stays. Which native build?
   (a) **A new Re:CoM player reaction** (KH1's grab): a new reaction kind in Re:CoM's damage check `0x1401E6DE0` and a new
       case of its status action 0x18 (`0x1401EB0F0`): Sora plays KH1's held clip (the Neoshadow's `.mset` Sora set, request
       200, 126 frames) as a new motion slot of `PL0001.ANN` (asset step re-run), locked for it, no escape, any damaging hit's
       reaction ends it; no damage; started by a 0-damage record on the Neoshadow (a new `EN0027.ASD` row) at his feet; the
       Neoshadow holds 94 frames and the other dives on cue (KH1's `+0x4C`). Two new hooks (exact first bytes), our per-frame
       code only during the hold. 1:1 except Re:CoM's hit order.
   (b) **Re:CoM's stun** (kind 7) through a 0-damage record: Sora dizzy (Re:CoM's `p001_036`, fainting eyes) for 180 frames,
       presses shorten it; the dive's hit flinches him and the stun comes back. No new hook. Not 1:1 (look, length, escape).
   (c) **Re:CoM's Stop** (kind 8) for KH1's 94 / 126 frames: frozen with Re:CoM's `abnormal` effect; the dive hurts him with no
       reaction and does not free him. No new hook. Not 1:1.
   (d) Keep KH1's miss (built).
3. **The call (one left).** Answered (owner, 2026-11-02): the call stays, nothing comes (as built).
4. **The dive's power** (no Re:CoM twin). Answered (owner, 2026-11-02): Re:CoM's come-up strike, id 1 (as built).
5. **Fire on a diving or rising Neoshadow.** Answered (owner, 2026-11-02): Fire hits it, no turn-back (as built).
6. **Where a stuck traveller turns** (new, from question 1). KH1 turns a Neoshadow that is stuck underground toward room 29's
   middle (`(8700, 5300, 100)`), away from walls. Built: (a) toward Sora (he stands in the open room). Other choices: (b) back
   the way it faces; (c) toward where it came from.

### The Wight Knight (built)

Status: built, tests and Windows build pass; **not seen in game**; no KH1 trace of the Wight Knight yet. Its attacks run on
Re:CoM's states 2 / 3; its three body rows bounce Sora's swings during its wind-up, its swipes and its leap slash (the shared
enemy guard, KH1's mode 2 rows); every hit it takes plays KH1's reaction (no `sys 42` in its script).

#### KH1's side. Checked (files: `ex_2200.bd` disassembled, code from `0x618`; decomp: the syscalls named)

The script (0x2900 bytes) keeps the actor at `wp+0`, the state object at `wp+0x20`, the search object at `wp+0x30`; no child
object (no `sys 94`). The Wight Knight has **no idle state**: it engages at once and never wanders.

| Routine | Offset | What |
|---|---|---|
| entry | `0x22F4` | init `0x23FE` (`wp+0x5C` = KH1's world, `wp+0x64` = `sys 105`'s second word is 8 (a room: Assumption), stats `+4` walk 5 (run never written: 8, the default), the hit handler `0x24D2`, stat record `0x14` / `0x80`); a yield; the target party 0; the appearance thread `0xA90`; each frame: an effect ends unless 212 / 218 / 207 / 213 / 200 / 69 plays (M13); a state request restarts the behaviour (1 engaged `0x7AA` → `0x7E2`, 2 hit `0x618` → `0x664`) |
| appearance | `0xA90` → `0xC46` | `wp+0x64` or spawn type ≠ 0 / 5: the 30-frame fade-in, flag 0x80 30 frames, effect 10, facing Sora now, 69 (a plain request, blend 0, the idle queued at 0) for len − 4 = 108 frames turning to Sora (off the ground: the end), the idle at 8 and 6 frames (`0x2870`); spawn type 5: hangs (215) until Sora is within 6 m and not standing / walking (or it falls), then drops (213, an action): not ported (Re:CoM's battle spawns play 69); then engaged |
| engaged | `0x7E2` | the first time: above 80 % the combo, under 50 % the leap slash, else the forward leap and the leap slash; then forever: < 300 close, else far; a yield; (world 11: the leash `0x2764(1000, 1000)`, not ported); the search `0x1D2E(&wp30, 1, 5000, 3)` (Sora) |
| close | `0x95A` | above 50 %: the back leap (206), then beyond 400: under 30 % the leap slash, above 50 % the sway (218), else the circling walk (0.5° a frame) and the forward leap; within 400: under 40 % the leap slash, above 80 % the sway, else the combo. Else: the combo and the back leap |
| far | `0x910` | above 80 %: the forward leap and `0xD34` (beyond 250: above 50 % the leap slash, else the dance 212; within: the combo); else the circling walk (0.6° a frame) and the dance |
| hit state | `0x664` | (`wp+0x58` = 1, attribute 8: the leap slash; not carried) beyond 400: under 80 % the leap slash, else the forward leap; within: above 70 % the leap slash, else a leap (above 50 % forward, else back) and beyond 300 the leap slash, else the combo. Then engaged |
| hit handler | `0x24D2` | a clash (kinds 6 / 7): the hit state. The damage (`0x1E38` → `sys 44`, 0 = killed): killed → attribute 8 an EXP bonus (`sys 194`), **kind 3 → 4** (`0x222A`), the death `0x2644` (drops `0x5BC`, fade, EXP: Re:CoM's defeat here); alive (attribute 0) → the hit state; attribute 8: EXP bonus, `wp+0x50` = `wp+0x58` = 1, the hit state; attribute 10: the hit state; a repel record (`hp[0] & 0x20`, never: its rows don't repel): nothing |
| combo | `0x146E` | on the ground; the start `0x1834`: `wp+0x40` / `+0x54` / `+0x48` = 0, body row `0xEC`, action 200 (blend 4, 201 queued); then by turns `0x16FC` (201, `wp+0x40` = 1) / `0x15C4` (202, `wp+0x40` = 0): `wp+0x4C` = 0, `wp+0x54` + 1, `wp+0x48` = 0, turning to Sora, body row `0x15C` and the two hands, the action (blend 0, the other swipe queued); a landed hand (callback `0x1D0C`: `wp+0x48` = 1) → `wp+0x54` = 4; Sora beyond 350 → `wp+0x4C` = 1; until `wp+0x4C` or 4 swipes (a yield between); then 203 after 201 / 204 after 202 (actions, the idle queued); `wp+0x4C`: the leap slash |
| leap slash | `0x18CA` | on the ground; beyond 600 the far leap `0x1BAA` (205, lift at 22); Sora's way (3D, unit) and distance d; turning to him, action 207 (blend 0, 208 queued); 208 (bit 16) and movement state 3 (`sys 82`: way × d × 0.035 with height 500, drag 0.9, 30 frames; `sys 83(207)`: 209 at the top, 210 queued), facing him now each frame in the air; on landing facing him now, the hands `0x38C` / `0x3FC` / `0x46C` / `0x4DC` and body row `0x54C`, 216 (blend 0, a plain request) with the idle queued, for len − 2 − 8 = 86 frames (off the ground: the end) |
| leaps | `0x1284(back, facing)` | on the ground; turning to Sora; 205 forward / 206 back (blend 4, 6 queued, root motion: 4.1 m / 3.8 m); from frame 20 movement state 7 with no velocity (`sys 169`); at the motion's end movement state 0 (`sys 88`), a yield, until on the ground (turning to Sora when `facing`) |
| circling walk | `0xD96(rate)` | on the ground; [100, 200) frames; Sora's way turned −90° (60 %, the rate as given) or +90° (the rate negated); each frame the walk (1, blend 16), the way turned rate × step, the target yaw along it, the walk at 5 along it; after the yield: moved under 2.5 → the idle and the stuck hop `0x266C` (3 action, 4 bit 16, `sys 82`: facing × 20, height 400, 0.9, 50 frames); off the ground: the end; within 100 of Sora after 50 frames: the idle and the back leap; else at the end the idle |
| dance / sway | `0x103C(dance)` | on the ground; 212 for 200 frames (`dance`) or 218 for 150 turning to Sora (blend 16); off the ground: the end; another motion playing: the idle, the end; within 200 after 80 frames: the idle and the back leap facing Sora; unless stopped the idle; **then the leap slash** |

Rows (`0x70` bytes; Checked (files)):

| Row | Bone code, t, radius; HIT slot | Type / priority / recoil | KH1 `+8` / power | Made with | Windows |
|---|---|---|---|---|---|
| `0xEC` | 8 (body), 50; 0 | **1 / 100 / 110**, cone cos −1 (all round) | 0 / 0 | 200 | [6, 40) |
| `0x15C` | 8, 50; 2 | **2 / 20 / 217**, cos −1 | 0 / 0 | 201, 202 | [0, 70) |
| `0x1CC` / `0x23C` | 29 / 28 (hands), line 0 → −5, 40; 0 / 1 | 0 / 20 / 217 | 3 / 4 | 201 | [20, 60) |
| `0x2AC` / `0x31C` | the same | 0 / 20 / 217 | 3 / 4 | 202 | [20, 60) |
| `0x38C` / `0x3FC` | 29 / 28, line 1 → −4, 30; 0 | 0 / 20 / 217 | 3 / 2 | 216 | [2, 10) |
| `0x46C` / `0x4DC` | the same; 2 | 0 / 20 / 217 | 3 / 2 | 216 | [24, 34) |
| `0x54C` | 8, 50; 1 | **2 / 20 / 217**, cos −1 | 0 / 0 | 216 | [2, 34) |

What the body rows do (`0x1402B4A20` / `0x1402B4BC0` / `0x1402B4640`, the shared enemy guard's rules): Sora's swing (type 0,
the Kingdom Key's 30) meets them from any side; against `0xEC` (100) Sora gets **kind 7, the strong recoil 0x6F**, the Wight
Knight nothing; against `0x15C` / `0x54C` (20) **both get kind 6**: Sora's recoil 0x6E (no push: he is 10 over), the Wight
Knight its recoil 217 and its hit state. A clashed swing hits nothing more: during the wind-up's [6, 40), the swipes and the
leap slash's [2, 34) Sora's swings never damage it; magic (types 5 / 6) does.

Reactions: F (72..75), a (78 / 79), A (80 / 81 = its 78 / 79), L (82..86, 87..101); no ground knock-back (a kind 4 launches);
no run motion (request 2). Weight 1.0 in both records. Hurt parts: 5 (Re:CoM bones 39, 2, 3, 8, 17). Checked (files).

#### Re:CoM's side. Checked (exe + decomp)

`CEnemyWightnight` (vtable `0x140553748`, 139 slots, RTTI `.?AVCEnemyWightnight@@` at −8), ctor `0x140194ED0` (`40 53 48 83 EC
20`: `CHeartless`'s base ctor, then `+0xCC0 |= 1`, `+0x298` = 6, `+0x29C` = 7; its only reference the factory's call at
`0x140189194`; no branch into the moved bytes, no pointer to them: exe scan), allocation `0x14018917B` `mov ecx, 0xCD0` →
0xCF0. Its own states 1 (`0x140195040` / `0x140195410`, end the shared thunk `0x140195080`), 2 (`0x140194FA0` / `0x140195090`:
its two-strike claw `e012_052`, then 10 % a jump `e012_003..007`), 3 (`0x140194FF0` / `0x140195320`: the wind-up `e012_053`
turning to Sora, then `e012_054`); the attack states' ends are its own `ret`s (as `CBattleEnemy`'s `0x14017E6C0` /
`0x14017E700`); 0 / 11 / 12 / 30 `CBattleEnemy`'s; think `0x140196EC0`. Our copy: states 0, 1, 2 (the leap slash), 3 (the
swipes), 11, 12, 30, the think. `EN0012.ASD`: 6 records at `0xBC`, tail `0x47C` (id 0 on `e012_052`: each hand at [0, 16)
and [20, 45), 0.4 m; id 1 on `e012_054`: each hand, the whole motion). `EN0012.SES`: `e012_004`, `e012_052`, `e012_054`.
`EN0012.ANN`: 75 slots.

**Power mapping** (decision M10, "the Re:CoM attack of the same kind"): the matcher pairs none of KH1's attacks. `e012_054` is
KH1's swipe in Re:CoM's hands: its root goes 0.94 m forward by KH1 frame 42 and back to 0.90 m by 66, as KH1's 201 (0.94 m at
42, 0.89 m at 68), after a wind-up as KH1's 200 (Checked (files): both roots read). So the swipes 201 / 202 → **id 1** (state
3). The leap slash 216 (two strikes) → **id 0**, Re:CoM's two-strike claw (Assumption: by elimination; owner question 1).

#### What the asset step writes

`ENEMY/0012/EN0012.ANN` (Re:CoM's 75 slots byte-identical; KH1's clips at 100 + entry, 100..143; 5 and 6 play Re:CoM's exact
bakes `e012_005` / `e012_006`), `EN0012.ASD` (Re:CoM's 6 records kept + 8: each swipe's two hands (id 1, a 1.48 m line, 0.40
m, [20, 60)), the leap slash's two hands at its two strikes (id 0, 1.48 m line from 0.30 m, 0.30 m, [2, 10) and [24, 34))),
`EN0012.SES` (Re:CoM's 3 entries kept; the swipes and the leap slash take their template's sounds at the hit window:
Assumption), `kh1-wightknight.json` (`kh1-enemy 1`: 66 numbers read at their pushes and the run speed's stats default, 39 request words checked, 3 attacks
(recoil 217), the 3 body rows, 5 parts). Test `wightknight::tests::builds_from_the_players_files`. In `mod.yml`:

```yaml
- {name: ENEMY/0012/EN0012.ANN, method: copy, source: [{name: ENEMY/0012/EN0012.ANN}]}
- {name: ENEMY/0012/EN0012.ASD, method: copy, source: [{name: ENEMY/0012/EN0012.ASD}]}
- {name: ENEMY/0012/EN0012.SES, method: copy, source: [{name: ENEMY/0012/EN0012.SES}]}
- {name: kh1-wightknight.json, method: copy, source: [{name: kh1-wightknight.json}]}
```

The step prints `wightknight (M11): …`; the log shows `wightknight: KH1's wightknight (M11) …` at start, `battle N: Wight Knight …
gets KH1's AI` in a Wight Knight battle, and `its guard row stopped Sora's swing` when a swing bounces.

#### Checked / Assumption

- **Checked (files):** every script number at its offset (the asset test reads the 66 from the player's script; names with
  several pushes agree), 39 request words, the rows (types, priorities, cones, recoils, radii, slots), the hit windows, the
  hurt parts, the clip lengths and root travel, Re:CoM's records, clips and sounds, `e012_054`'s root against KH1's 201.
- **Checked (decomp):** the VM's ops; `sys 13` writes the request entry whatever plays (`0x14029F980`: an action asked while
  its queued twin plays restarts it); `sys 169` / `sys 88` / `sys 82` / `sys 83` (the framework's); the guard rules
  (`0x1402B4BC0`, `0x1402B4640`); Re:CoM's class's states and ctor.
- **Checked (exe):** the class, its 139 slots, RTTI, the ctor and its only call, the factory size; every site (954, pefile, 0
  mismatches, 2026-11-02).
- **Assumptions:** the leap slash on id 0; `sys 163` ("drawn this frame") always true; attribute-8 / 10 hits never come (the
  host passes no attribute; `wp+0x50` / `+0x58` paths not built); the leaps' height comes from Re:CoM playing the clip's root
  (KH1's state 7 holds the body while the root lifts it); the apex motion a frame late (as the Powerwild); `sys 10`'s sign
  (one rule for both turns of the circling walk); seeds per enemy; Re:CoM's sounds on KH1's clips; KH1's queue blends taken as
  the host's; Wight Knights appear in Re:CoM's Halloween Town battles (Re:CoM's).
- **Not ported:** the leash (world 11 only), the hanging appearance (spawn type 5), the death routine, the effects (9, 10: M13),
  the appearance's flag 0x80; Sora's swings against its damaging hand rows (KH1 tests attack against attack: the owner's Large
  Body question 3 (b), framework work to come; its body rows already bounce every swing during those attacks).

#### Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`; a Halloween Town Wight Knight battle)

1. Each Wight Knight plays KH1's appearance (69, about 2 s, turning to Sora), then comes at once (no wandering).
2. Its first move: the combo, the leap slash, or a forward leap and the leap slash.
3. Close (< 3 m): half the time a big back flip (206), then the leap slash, the sway (218) or the combo; otherwise the combo then
   the back flip. Far: sometimes a forward flip (205), else it walks sideways around Sora for 2–3 s and dances (212) for 3 s;
   after a dance or sway it always does the leap slash. The flips should carry it 4 m and leave the floor (KH1 lifts it
   by the motion's root from frame 20).
4. The combo: a wind-up (200, swings bounce hard off it), then up to four alternating long-armed swipes (201 / 202) stepping
   forward, then a recovery (203 / 204). A swipe that hits Sora ends the combo; if Sora backs off past 3.5 m it ends and leap-slashes.
5. The leap slash: a crouch, a 5 m high jump at Sora (covering about half the distance), facing him in the air, a two-strike
   slash on landing (216). Beyond 6 m a forward flip comes first.
6. Swing into it during a swipe or the leap slash's first half second: both recoil (Sora 0x6E, it 217), no damage, then it
   counters. During the wind-up: Sora's strong recoil (0x6F). Magic lands. No "Guard" popup.
7. Hit it at other times: KH1's flinch, then the hit state (a leap slash, a flip, or a flip and the combo).
8. Guard its swipes or leap slash: KH1's recoil 217, then it counters.
9. Circling it near a wall: when it is stuck it hops 4 m high (the stuck hop).
10. Kill one: KH1's launch reaction (a killing flinch becomes a knock-back), Re:CoM's prizes and EXP.
11. `enemy_ai = recom`, Combat = Re:CoM, Riku's story: Re:CoM's Wight Knights.

Please send `kh1-recom.log` (`wightknight:` lines) and an F9 trace.

#### KH1 trace (what to record in KH1)

Halloween Town (the square or the graveyard): stand far (10 m) 10 s; stand at 5 m 10 s; stand at 2 m 10 s; walk around one; one
swing and a full combo between its attacks; swing into it during its wind-up, a swipe and the leap slash; Guard a swipe and the
leap slash; kill one with a flinching hit. The trace's enemy rows give the flips' travel and height, the leap slash's arc and
the combo's timing to compare.

#### Wight Knight owner questions

1. **The leap slash's power.** Re:CoM's Wight Knight has its two-strike claw (id 0, `e012_052`) and its lunge (id 1, `e012_054`,
   the twin of KH1's swipe by its root). Built: the swipes on the lunge's power, the leap slash (two strikes on landing) on the
   claw's. (a) Keep; (b) the leap slash on id 1 too; (c) another rule.
2. **Hits of KH1 attribute 8 / 10.** Its hit handler gives an attribute-8 hit an EXP bonus and switches it to lower leaps (100
   high, 50 frames) with no flips, and an attribute-10 hit only asks for its hit state; the host passes no hit attribute (every
   port so far), so these never happen. (a) Leave them out (built); (b) research which KH1 hits carry attributes 8 / 10 first
   (a framework change for every enemy, as the Darkball's Gravity question).

## The other 29, by family

States: those the Re:CoM class overrides (besides `CBattleEnemy`'s). Hits: KH1 requests with a HIT track.
Reactions: `F` flinch, `K` knock-back, `a` air flinch, `A` air knock-back, `L` launch, `l` air launch.
Missing: Re:CoM files not in the session's extraction (`.BIN` only exists for the Shadow and Spade Soldier
here; the step doesn't use it). Every ctor starts `40 53 48 83 EC 20 45 33` (Checked (exe)). Checked (exe +
files); the family is from the scripts' syscalls and the models' motions (Assumption where noted).

| Enemy | KH1 model (script) | Re:CoM class, id | vtable | ctor / alloc site, size | States | Hits | Reactions | Missing |
|---|---|---|---|---|---|---|---|---|
| Soldier (built) | `xa_ex_2010` (ex_2010.bd) | `CEnemySoldier` 2 | `0x14054ea68` | `0x140193510` / `0x140188fd7` 0xcd0 | 1,2,3 | 200,201,202,203,204,207,209 | FaLl | — |
| Large Body (built) | `xa_ex_2050` (ex_2050.bd) | `CEnemyLargebody` 9 | `0x140542ad0` | `0x14018f090` / `0x1401890fd` 0xce0 | 1,2,3 | 200,207,231 | FKL | — |
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
| Pirate (built) | `xa_ex_2100` (ex_2100.bd) | `CEnemyPirate` 21 | `0x140546458` | `0x140190f30` / `0x1401892f5` 0xcd0 | 1,2,3 | 208,210,214,216,217,218,235,236 | FaLl | — |
| Air Pirate (built) | `xa_ex_2210` (ex_2210.bd) | `CEnemyAirpirate` 22 | `0x1405309e8` | `0x14018a0b0` / `0x14018931f` 0xcd0 | 1,2,3, think `0x14018a190` | 200,212 | FKaA | — |
| Wight Knight (built) | `xa_ex_2200` (ex_2200.bd) | `CEnemyWightnight` 12 | `0x140553748` | `0x140194ed0` / `0x14018917b` 0xcd0 | 1,2,3 | 200,201,202,211,216 | FaALl | — |
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
| Neoshadow (built) | `xa_ex_2420` (ex_2420.bd) | `CEnemyNeoshadow` 27 | `0x140545128` | `0x140190150` / `0x1401893f1` 0xce0 | 0,1,2,3 | 202,205,207 | FKaA | — |
| Spade Soldier | `xa_aw_1030` (aw_1030.bd) | `CEnemySpadesoldier` 103 | `0x14054fd90` | `0x140193a20` / `0x140189afc` 0xcd0 | 0,1,2,3,30 | 209 | FKaA | — |
| Heart Soldier | `xa_aw_1060` (aw_1060.bd) | `CEnemyHeartsoldier` 104 | `0x1405417d0` | `0x14018ec70` / `0x140189b23` 0xcd0 | 0,1,2,30 | 206,207,208,210 | FKaA | — |

Re:CoM `.ASD` records (template for the power rule): none for Red Nocturne, Green Requiem, Bouncywild, Wizard,
White Mushroom, Creeper Plant (their attacks are Re:CoM shots); one for Blue Rhapsody, Yellow Opera, Barrel
Spider, Defender, Aquatank, Heart Soldier; 3 Powerwild, Sea Neon, Wyvern, Neoshadow; 6 Wight Knight; 2 the
others. Re:CoM `.ANN` slots: 73–95 (all under the step's slot 100). Checked (files).

### Families

1. **Ground melee** (the Soldier's shape: walk / run, hit tracks, kicks and lunges; the framework as built):
   Large Body (built: § The Large Body), Powerwild, Bandit,
   Pirate, Wight Knight (built: § The Wight Knight), Neoshadow (built: `Appear::Base`, KH1 has no appearance for it; its sinks reuse the Shadow's machinery), Spade / Heart Soldier (Re:CoM overrides their states 0 and 30: check their
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

## Fix 0.75.2: crash when Air Soldiers entered a battle (runtime lookup)

Owner (2026-11-02, on 0.73.0): "game crashed". Log: `CRASH: exception 0xc0000094` (integer divide by zero)
at exe `0x140197B60`, called from `kh1-recom.dll+0xb8d11` = `win::kh1_enemy::think_step`, right after four Air
Soldiers got KH1's AI. `0x140197AD0` is `CHeartless`'s state-1 walk step; it divides by `+0xCA4`, which
Re:CoM's think (`+0x3D0`) sets, and our copy replaces the think with nothing. Checked (decomp + the 0.73.0 DLL's
symbols). So `think_step`'s lookup had failed and it ran Re:CoM's own state 1.

Cause (Checked (code)): the runtime table was indexed by the tail's stored index, and the ctor
`retain`ed out an older runtime at the same address (an enemy rebuilt in the same battle, as waves reuse
freed memory), which shifted every later runtime: their indices went stale, the lookup failed.

Fix: an enemy rebuilt at the same address replaces its runtime in place (other battles' runtimes, which
come first, are still dropped); the lookup takes the stored index when it still matches, else the address;
a failed lookup for one of ours is logged once (`KH1 runtime lookup failed (…)`) and **never runs Re:CoM's
own state 1** (begin or step): the frame is skipped. Checked: tests, Windows build. Not checked: in game.
