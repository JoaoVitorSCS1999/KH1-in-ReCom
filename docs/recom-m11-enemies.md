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

**Finding (Shadow, not changed):** KH1's wander (`0x1BC4` / `0x15BA`) adds both points to the actor's
**current** position (`sys 7` with `actor+0x10`), not to home; M10's Shadow port adds them to home. The shared
`wander` is KH1's; the Shadow keeps its own (its runtime must stay identical). Owner question below.

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
  no entry: `0x1402A01E0` flags 0x200000), the attack is cut and the idle plays (Assumption; logged).
- `reads_hp`: the runtime passes Re:CoM's HP / max HP (`+0x274`, `0x14017F5A0`) at each hit (the Soldier's
  counter reads its HP ratio; Re:CoM's HP, decision M10).
- Hurt parts from the manifest (any count): the contact point's nearest part bone (`react::nearest_part`;
  for two parts it is M10's rule).

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
7. Guard a kick: KH1's clash recoil (218). Guard the jumping / spin / air kick: the attack stops and it idles.
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
| Air Soldier | `xa_ex_2160` (ex_2160.bd) | `CEnemyAirsoldier` 15 | `0x140531cc8` | `0x14018a4e0` / `0x1401891f9` 0xcd0 | 1,2,3, think `0x14018a5c0` | 200,201,202,203,210,211,212 | FKaA | — |
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

## Owner questions

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
