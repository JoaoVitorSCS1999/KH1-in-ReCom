# AGENTS.md: KH1 combat in Re:Chain of Memories

Read first, every session:

1. `docs/design-philosophy-instructions.md`
2. `docs/engineering-rules.md`
3. `docs/tester-requests.md` (its rules win if they differ)
4. The top ~15 stages below.

Work only on branch `testing`. Stages are newest first.

---

## Stage 135: M11: a shared enemy guard (KH1's rule on Re:CoM's body guard); the Bandit's block stance and spin repel, version 0.75.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Shared enemy guard first" for the Bandit's block stance and spin repel (then Large Body,
  Defender).
- KH1 (Checked (decomp + exe)): a guard row (type 2, table `0x140528CB8`) never hits a body and only meets Sora's
  swings (magic never blocked); front half only (`0x1402B4640`), while its request plays with its hit window on; two
  meeting attacks give the lower-priority side kind 6, or 7 when 40+ under (`0x1402B4BC0`); Sora: kind 6 recoil 0x6E
  + push, Counterattack from frame 12; kind 7 0x6F; in the air 0x70; the enemy plays its row's recoil and its script
  hears the clash.
- Re:CoM (Checked (decomp + exe)): its hit apply `0x14016F1C0` blocks a body with its guard bit (`+0x9C` bit 1, mask
  `+0x8C`, half angle `+0xB0`): set on the enemy's hurt bodies while the row is live (Re:CoM's values put back), KH1's
  type rule around each swing, the enemy's side from our vtable slot `+0x3D8`. Bandit: block stance → both kind 6
  (Sora recoils, the Bandit 232, back hop + throw); spin → Sora's strong recoil, the Bandit 213.
- No new hooks. Large Body / Defender shields (a turned cone) need more research.
- Asset step re-run: `kh1-bandit.json` gains `guards` (EN0011 files byte-identical).
- Checked: tests, Windows build, sites. Not checked: in game.

---

## Stage 134: M11: the KH1 Wyvern; motions from a start frame, KH1's loop rule, version 0.74.0 (2026-11-01)

- M11 (owner, 2026-11-02): a flier.
- Framework: `Cmd::MotionAt` (KH1's `sys 13` start frame through Re:CoM's motion play's start time, its third argument;
  Checked (decomp + disasm)); asset step `enemy::LOOPS` (clips KH1 loops though their last pose differs). Other
  enemies' outputs unchanged.
- Wyvern (`xa_ex_2320` ↔ `CEnemyWyvern` id 24; `ex_2320.bd`, 123 constants, 49 request words): appearance, notice
  (215) within 15 m; far: 70 % chase (207 / 208, banks 209 / 210), bite (200), fly past and up, come down (212, idle
  from frame 16), then charge or dive; 30 % charge (204, 40-frame rush 205 at 40/f, 206); close: roar (213) then dive
  (chase to 4 m above, 201, drop 202, land 203), every third close round the far attack; after 3 rounds rises and
  circles; a hit in the dive doesn't flinch (214); 5 counted hits within 3 m roar + dive, 3–10 m roar + charge + circle;
  Olympus rule (100 m search) in Re:CoM's Olympus. All guards: Re:CoM's flinch (recoil 110 missing). Vtable
  `0x140555DC8`, ctor `0x140195ED0`, alloc `0x140189373` 0xCF0 → 0xD10. Power: bite → id 0, charge → id 1 (Checked:
  Re:CoM's clips edit KH1's), dive → id 1 (Assumption).
- Asset step: `ENEMY/0024/EN0024.ANN` (77 kept, KH1 clips 100–127), `.ASD` (3 + 6), `.SES`, `kh1-wyvern.json`.
- Checked: tests, Windows build, 607 + 230 sites. Not checked: in game, KH1 trace.

---

## Stage 133: M11: the KH1 Air Pirate, version 0.73.0 (2026-11-01)

- M11 (owner, 2026-11-02): a flier.
- Framework: `Behaviour::set_target_grounded` (KH1's target state `+0x70`; fed from Sora's mover floor contact);
  the asset converter accepts a bone 1 named `m_ex…_sk` (Re:CoM's Air Pirate skeleton).
- Air Pirate (`xa_ex_2210` ↔ `CEnemyAirpirate` id 22; 126 constants, 49 request words; Checked (files + decomp)):
  appearance, rises to 170 above Sora and circles 4–8 s, stops (203), engages; far: Sora below → dive (205–207) then
  swoop; level → wind-up / take-off, banking flight, swoop or slash; above → climb and slash; 2–5 m: to 1 m above,
  swoop, back off; < 2 m: back off and climb, or swoop and fly off 5–7 m; after 10 rounds circles again; a hit in the
  dive doesn't flinch (211); 1–4 hits counter, 5–9 back off, 10+ circle. Swoop 200 (steered, frames 41–48), slash 212
  (frames 39.3–51); guarding the swoop: KH1's recoil 213, the slash: Re:CoM's flinch (no 110). Vtable `0x1405309E8`,
  ctor `0x14018A0B0`, alloc `0x14018931F` 0xCD0 → 0xCF0. Power: slash → id 0 (`e022_050` edits KH1's 212), swoop →
  id 1 (Assumption).
- Asset step: `ENEMY/0022/EN0022.ANN` (74 kept, KH1 clips 100–124), `.ASD` (2 + 2), `.SES`, `kh1-airpirate.json`.
- Assumptions: the two world-height tests as heights above the battle floor (owner question); others in the doc.
- Checked: tests, Windows build, 603 + 194 sites. Not checked: in game, KH1 trace.

---

## Stage 132: M11: the KH1 Bandit; movement state 7, KH1's hit counter, world-aware scripts; an attack-point fix, version 0.72.0 (2026-11-01)

- M11 (owner, 2026-11-02): a ground enemy.
- Framework (Checked (decomp)): movement state 7 (`sys 169` → `0x1402BB030`, the leap's flight: the flight step,
  airborne for reactions); KH1's per-enemy hit counter (record `+0x24` from `actor+0x3E8`, zeroed when no action
  plays, `0x1402BFE60` / `0x1402B7260`); a script's hit handler may rewrite the reaction kind (KH1 runs it before the
  reaction); KH1's world passed to scripts (decision 21's mapping). Asset step: attack points on bones the motion
  never poses now follow KH1's rule (the Bandit's blade 3.3 m → 1.30 m; the Air Soldier's spin row now ±7 cm around
  the toes: `EN0015.ASD` changes; Soldier / Powerwild byte-identical).
- Bandit (`xa_ex_2090` ↔ `CEnemyBandit` id 11; `ex_2090.bd`, 117 constants): appearance (69 ×2); wanders, engages
  within 15 m or when targeted; < 2 m back hop + spin or strafe + slash; 2–4 m strafe, pause, run-in slash or leap;
  far: run in, leap or throw; the leap dashes at 70 to 50 cm above Sora and carries on past; spin up to 4 rounds;
  counters after the 1st / 2nd hit (block stance 50 %, slash 20 %, back hop + leap 30 %); knock-backs become launches;
  Agrabah's rules in Re:CoM's Agrabah. Vtable `0x140534368`, ctor `0x14018AEB0`, alloc `0x140189151` 0xCD0 → 0xCF0.
  Power: slash → id 0, spin → id 1, leap → id 1 (Assumption).
- Not built yet (owner questions): the thrown blade (the throw motion plays, nothing flies); the block stance / spin
  repel. Asset step: `ENEMY/0011/EN0011.ANN`, `.ASD` (2 + 6), `.SES`, `kh1-bandit.json`.
- Checked: tests, Windows build, 609 + 176 sites. Not checked: in game, KH1 trace.

---

## Stage 131: M11: the KH1 Darkball (warp, model scale and hide through Re:CoM's own calls), version 0.71.0 (2026-11-01)

- M11 (owner, 2026-11-02): a flier.
- Framework (Checked (exe + decomp)): uniform model scale through Re:CoM's getter / setter (`0x1401A4A20` / `0x1401A7050`);
  hiding through Re:CoM's hide bit `enemy+0x154` bit 0 (its draw `0x1401A39B0`); KH1's warp (`sys 86`); the script's
  flight easing writes (stats `+0x14`); a hit handler may cancel a hit's reaction; a steering flier counts as active
  for the quiet watchdog. Asset step: `OWN_CLIPS` (KH1 requests on their own clips so Re:CoM's records stay on its
  clips), `BONE_POINTS` (a bone's position in a frame as a constant).
- Darkball (`xa_ex_2280` ↔ `CEnemyDarkball` id 23; `ex_2280.bd`, 151 constants): grows in 0.8 m up (unhittable until
  grown); wanders, engages within 10 m after notice (205); close: bite (204), warp, circle and 3 dashes (201–203);
  far: warp or circle + dashes; warp: shrinks, vanishes (unhittable), glides, reappears above Sora at twice the size
  (drop 200) or plainly after 4 tries; counters (bite close, dash far), warps away after 5 counted hits, 5th light hit
  in a row bites, from the 6th no flinch. Vtable `0x14053B7A8`, ctor `0x14018D5D0`, alloc `0x140189349` 0xCD0 → 0xCF0.
- Power: dash → Re:CoM id 0, bite → id 1 (exact copies), drop → id 1 (Assumption). Guarding the dash: KH1's recoil
  206; bite / drop: Re:CoM's flinch (no recoil 110 in the model).
- Asset step: `ENEMY/0023/EN0023.ANN` (Re:CoM's 86 kept), `.ASD` (2 + 3), `.SES`, `kh1-darkball.json`.
- Not ported: leash, party retargeting, world rules, KH1 hit attributes (no Gravity stagger), warp smoke / fades,
  no-push flag. Checked: tests, Windows build, 765 sites. Not checked: in game, KH1 trace.

---

## Stage 130: M11: the KH1 Powerwild; KH1's timed jump and scripted launch in the framework, version 0.70.0 (2026-11-01)

- M11 (owner, 2026-11-02): the ground family's first.
- Framework (Checked (KH1 decomp)): movement state 3, the timed jump (`sys 82` → `0x1402AF150`, step `0x1402AFAE0`; apex
  motion 6 frames before the top `0x1402AF630`, no landing motion); scripted launch (`sys 38` → `0x1402B79F0`): a hit
  handler can replace KH1's reaction (its random draw kept). Asset step: one `.ASD` record per hit window (a slot can
  open several times; Shadow / Soldier / Air Soldier outputs unchanged).
- Powerwild (`xa_ex_2030` ↔ `CEnemyPowerwild` id 7): first look; close: back hop / side hop + swipe; mid: sideways
  walk, pause / roar, shuffle into hop + swipe or swipe, maybe the pounce; far: run in; counters: flurry + back hop or
  side hop + pounce; above 40 % HP a launching / knock-back hit plays its own launch (base 270 / 280 in the air, hop
  80, speed 25) then the shake-off (224). All numbers from the player's `ex_2030.bd`. Vtable `0x140547798`, ctor
  `0x140191380`, alloc `0x1401890A9` (0xCD0). Power: swipe 212 → Re:CoM id 0, flurry 214 → id 1, pounce 222 → id 1
  (Assumption: nearest).
- Asset step: `ENEMY/0007/EN0007.ANN`, `.ASD` (Re:CoM's 3 + 9), `.SES`, `kh1-powerwild.json`.
- Checked: tests, Windows build, 757 sites. Not checked: in game, KH1 trace.

---

## Stage 129: Air Soldier: the quiet watchdog can restart it (Behaviour::reset), version 0.69.1 (2026-11-01)

- The Air Soldier's `Behaviour::reset` (Stage 127's quiet watchdog): back to engaged, still flying, no velocity, its
  appearance counted done. Test added.
- Research (no DLL change): KH1's second AI syscall table `0x1405752D0` (`sys1.0` `0x140394D80` a no-op, `sys1.3`
  `0x140394DE0` removes a child); `tools/kh1-ai-script.py` reads both tables. Child objects (`sys 94`, part 3 of the
  `.mdls`): Search Ghost's is a 35° / 3 m target cone with an effect (no projectile); Gargoyle's moves on its own and
  makes attack rows (owner question). Checked (exe + decomp + files).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 128: M11: the KH1 Air Soldier, with KH1's flight in the shared enemy framework, version 0.69.0 (2026-11-01)

- M11 (owner, 2026-11-02, "bring in the remaining KH1 adjacent heartless"): the flying family's first.
- Framework: KH1's flight (movement state 4, step `0x1402CC000`: horizontal eases at 0.1/frame, vertical at once, no
  gravity; fliers' finishers knock back); steered attacks (`Cmd::Attack`: the AI keeps steering while the attack
  plays in Re:CoM's attack state); behaviour inputs (floor height, Sora targeting it `player+0x84C`, the hit's KH1
  kind, an attack landing: result 1 of `0x14016F1C0`); per-class rows (`parse` / `make`, `ctor_ptr`).
- Air Soldier (`xa_ex_2160` ↔ `CEnemyAirsoldier` id 15; Checked (decomp + files)): appearance, air wander,
  engages within 15 m or when targeted; < 3 m strafe + slash or spinning dash; 3–6 m swoop kick / swoop; far: hover,
  90-frame wind-up, charge; hits fill an anger meter (+25 / +10 / +5), full and far: power-up slash. 127 constants
  read from the player's script. Vtable `0x140531CC8`, ctor `0x14018A4E0` (6 bytes), alloc `0x1401891F9` 0xCD0 → 0xCF0.
- Asset step: `ENEMY/0015/EN0015.ANN` (+ KH1 clips from 100), `.ASD` (5 records), `.SES`, `kh1-airsoldier.json`.
- Merge fix: a steered attack's end is our own leave to state 1 (Stage 127's take-over doesn't drop its follow-up).
- Assumptions: flat battle floors (Sora's last standing height); power by limb (hands / charge on Re:CoM's dash punch,
  feet on its kick); the chest body-block row left out. Not ported: leash, group-leader messages, head look, fade.
- Checked: tests, Windows build, 737 sites. Not checked: in game, KH1 trace.

---

## Stage 127: Fix: KH1 enemies freezing after a Re:CoM state (Stun Impact's stun, Stop, freeze…), version 0.68.3 (2026-11-01)

- Owner (2026-11-02): "i used stun impact on 2 soliders (check trace) and once they got out of the stun they are doing
  the same issue of just standing still".
- Cause (Checked (log + trace 20)): Re:CoM's stun cut the kick / spin kick; Re:CoM went back to state 1 with our runtime
  still holding the attack as playing; the AI isn't polled during an action and an attack only ends in its attack
  state: never. The watchdog left playing attacks to that state.
- Fix (all classes): our own returns to state 1 are marked; any other entry means a Re:CoM state ran (stun, Stop,
  freeze, Gravity's flatten, Re:CoM flinch, first-strike stun): the pending action / reaction / queue / clash are
  dropped, idle plays, the AI goes on (KH1: a status ends the current action; that the AI carries on: Assumption).
  The appearance's end calls Re:CoM's default state outside our lock. New nets (logged past the cap): an attack
  still playing after its state was left; a reaction over 600 frames; an AI silent 600 frames in state 1 restarts
  from engaged (`Behaviour::reset`; KH1's longest silent wait 180).
- Checked: tests, Windows build, 717 sites. Not checked: in game.

---

## Stage 126: KH1 enemies: Re:CoM's flinch for a guarded attack with no KH1 recoil; KH1's wander centre, version 0.68.2 (2026-11-01)

- Asks (owner, 2026-11-02, on 0.68.0's questions): guarded attacks whose KH1 recoil the model lacks play "Re:CoM's
  flinch"; the Shadow's wander "Fix it to KH1".
- Every KH1 enemy class: a Guard block of an attack with no recoil motion makes the next state 11 Re:CoM's own flinch
  (its begin / step) until the next state 1. KH1's wander picks its points around the enemy's current place
  (was its home for the Shadow).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 125: Fix: KH1 Soldiers and Shadows freezing; a watchdog for every KH1 enemy, version 0.68.1 (2026-11-01)

- Owner (2026-11-02, video): "this soldier just decides to not attack and stay in one position"; "it seems the shadows
  are doing the same thing".
- Cause 1 (Soldier; Checked (files + log + trace)): the asset step marks a clip looping when its last pose matches its
  first; kick 200 and air kick 204 were, so Re:CoM looped them and the motion clock (`0x1401A47C0`) wrapped before KH1's
  length: the action never ended. `ClipWatch`: the clock going back after half the motion counts as its end.
- Cause 2 (Shadow; Checked (log + code)): when the rise ends and the first attack is requested on the same frame, state
  0's step called Re:CoM's default state (`0x14017D6F0`), overwriting the attack state, which never began. State 0 no
  longer does while an attack is pending; a pending attack state is requested again if something replaced it.
- Watchdog (all classes): an attack state not begun 10 frames after the request, or an action playing 30 frames past
  its motion's length, is released as if its motion ended, with a log line past the cap. Tests: a 20000-frame fight
  per enemy (12 seeds) acts at least every 600 frames.
- Not a refactor regression (M10's paths compared). The Shadow popping up while sunk: cause not found; new logging
  (motions and Re:CoM state changes while unhittable).
- Checked: tests, Windows build, 717 sites. Not checked: in game.

---

## Stage 124: M11 step 1b: the KH1 Soldier on Re:CoM's enemy states, version 0.68.0 (2026-11-01)

- KH1 (Checked (decomp + files)): `xa_ex_2010` script: appearance, wander, engage within 1000; close (< 350) kick or
  double kick (> 280), sometimes back off + spin kick; mid (350–450) jumping kick / spin kick / run-in kick; far run in;
  arc strafe after each round; jump-kick at Sora above; counters when hit. 64 constants + 14 request words read
  from the player's script.
- Re:CoM: `CEnemySoldier` vtable `0x14054EA68`, ctor `0x140193510` hooked (6 bytes), alloc `0x140188FD7` 0xCD0 →
  0xCF0; copy runs states 0, 1, 2 (ground kicks), 3 (airborne attacks), 11, 12, 30 and the think. Re:CoM's HP,
  damage, drops, EXP.
- Asset step: `ENEMY/0002/EN0002.ANN` (Re:CoM's 75 slots kept, KH1 clips from 100), `.ASD` (10 records), `.SES`,
  `kh1-soldier.json`.
- Not ported: KH1's leash (would make a Re:CoM battle unwinnable); scripted-room rules.
- Assumptions (`docs/recom-m11-enemies.md`): power mapping (kick / spin kick); missing recoils stop the attack.
- Checked: tests, Windows build, 717 sites. Not checked: in game, KH1 trace.

---

## Stage 123: M11 step 1a: a shared KH1-enemy framework from the Shadow's code (Shadow unchanged), version 0.67.4 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11).
- Framework: `kh1-core` `enemy/` (KH1's enemy body physics, the shared reaction engine, the script runtime, the
  helper routines KH1's enemy scripts share byte for byte, request bit 23); `kh1-recom` `kh1_enemy` (one class table:
  vtable, alloc / tail, ctor and size sites, attack-state map, appearance, manifest; one ctor wrapper per class;
  the Shadow is class 0); `kh1-recom-assets` `enemy` (one Spec per enemy writes .ANN / .ASD / .SES / manifest);
  `kh1-trace` enemy rows gain `kind` / `spawn`; `tools/kh1-ai-script.py` (lists a KH1 AI script from the player's files).
- Shadow unchanged: its tests pass, its four asset outputs hash the same, same manifest, logs and trace token.
- Checked: tests, Windows build, sites. Not checked: in game.

---

## Stage 122: Revenge moves untouchable whenever a boss plays them; Hook's hole closed, version 0.67.3 (2026-11-01)

- Owner (2026-11-02): "you can still hit bosses during their revenge move (or if they use their revenge move naturally
  instead of it being triggered by revenge value)".
- Holes (Checked (decomp + code)): (1) the window opened only for a revenge-started state, so a boss's own use of the
  same attack was hittable: now it opens every time the revenge-move state begins (own choice logged); (2) Captain
  Hook's ctor `0x1403C6340` sets `+0x260 |= 0x2000000` permanently, so his handler always ended the attack and took the
  hit (`0x14017E1D0`): held off for the handler call during the window; (3) every frame counted as an ignored hit (the
  handler runs per frame, `0x140185630`): only real waiting hits counted.
- Checked not holes: all attack states 2..10 begin through Re:CoM's wrappers calling `+0x438` → `0x14017DA80`; our
  blade sweep and `strike_body` skip bodies off (`+0x10` = 0); all damage goes through `0x14017F7E0`.
- Checked: tests (442), Windows build, sites. Not checked: in game (no boss log yet).

---

## Stage 121: Shadow: unhittable from the sink's first frame to rise frame 50, as KH1, version 0.67.2 (2026-11-01)

- Owner (2026-11-02): "i can attack the shadow when its flat on the ground". Trace 18: three hits during sinks.
- KH1 (Checked (decomp + script)): actor flag 0x40 skips the hit test `0x1402B6AF0` (gate `0x140291920`) and the auto
  target `0x1402B0A00`; `ex_2020.bd` sets it on the sink's first frame (request 70, `0x1366..0x138C`), clears it at
  rise frame len(69) − 30 = 50 (`0x12D8..0x1314`), and sets it at init for a floor-spawned Shadow.
- Fix: `kh1_core::shadow` emits `Hittable` at those three points; the DLL uses Re:CoM's own hurt-body switch
  (`0x1401A2D70` off, `0x1401A2DB0` on). 0.67.1 switched them at sink frame 105 and rise frame 0. Lock-on unchanged
  (KH1's lock button can lock a sunk Shadow; Re:CoM's lockable flag stays).
- Trace: defeated Shadows drop out 31 frames after defeat; `h0` / `h1` token. Log cap 4000 → 20000 lines.
- The traces otherwise match KH1's script (sink / rise, leap, claw / swipe, flinches, launch, clash).
- Checked: tests, Windows build, 697 sites, the trace. Not checked: in game.

---

## Stage 120: Fix: crash at the start of every battle with a Shadow, version 0.67.1 (2026-11-01)

- Owner (2026-11-02): "the game crashes when getting into a battle with a shadow". Log: access violation at
  `kh1-recom.dll+0xa4e44` reading 0x54, called from the factory `0x140188fcb` right after the ctor.
- Cause: `win::shadow::ctor` read the facing from the mover `*(enemy+0xF0)+0x54`; Re:CoM's ctor hasn't made the
  mover yet (`+0xF0` = 0; the log's rax 0). Now facing 0 without a mover (state 0's begin takes the spawn's facing
  from the mover, as before, and skips it without one); the hit reader returns no hit without a mover.
- Checked: tests, Windows build, the crash address in the 0.67.0 DLL. Not checked: in game.

---

## Stage 119: M10: the KH1 Shadow (AI, attacks, reactions, motions) on Re:CoM's own enemy states, Re:CoM's stats, version 0.67.0 (2026-11-01)

- Ask (owner, 2026-11-02): "lets move on to doing phase 4, except, i only want the AI, attacks, reactions and motions
  from KH1 equivalent enemies, everything else (emphasis on stats) stay the same as they are in RECoM"; M10 choices in
  `design-philosophy-instructions.md` (native states, `enemy_ai` default kh1, battle only).
- KH1 (Checked (files + decomp)): the Shadow's AI script `ex_2020.bd` (`xa_ex_2020.mdls` part 5, VM `0x1402CC620`,
  pauses while an action motion plays `0x140297870`); attack rows in the script (claw / swipe / leap claw, 35 cm);
  reactions `0x1402B9230` / `0x1402B8B90` / `0x1402B77D0` / `0x1402B8530` / clash `0x1402B9140`.
- Built:
  - `kh1-core` `shadow.rs`: the Shadow's behaviour and reactions (numbers as data); the Dummy's kind-3 flinch fixed.
  - Asset step: `ENEMY/0001/EN0001.ANN` (KH1 clips at slot 100 + entry; Re:CoM's 76 kept), `.ASD` (4 new attack
    records with KH1's hit windows), `.SES` (Re:CoM's Shadow sounds on the new clips), `kh1-shadow.json` (the
    constants read from the player's `ex_2020.bd` / `btltbl.bin`).
  - DLL: ctor hook `0x140192770` and factory size `0x140188FAD` 0xCE0 → 0xD00; battle Shadows get a vtable copy whose
    states 0, 1, 2–4, 11, 12, 30 run the KH1 Shadow, think `+0x3D0` a no-op; Re:CoM's motion play, attack records,
    damage, defeat, prizes, EXP, stats; KH1's clash recoil on a Guard block. ini `enemy_ai = kh1 | recom` (kh1).
  - `kh1-trace`: an enemies CSV per recording (for the owner's KH1 Shadow trace).
- Not built: the first rise's alpha fade (no Re:CoM model alpha field found).
- Assumptions in `docs/recom-m10-shadow.md`. Checked: tests, Windows build, 693 sites. Not checked: in game, KH1 trace.

---

## Stage 118: Silent human-boss item-card page switch at Combat = KH1, version 0.66.1 (2026-11-01)

- Ask (owner, 2026-11-02, on 0.65.2's questions): "1 keep it silent" (first deal), "2 silence it" (page switch).
- The page switch's `call 0x1402AA360` (id 3) at `0x140167A35` in the revolver's page switch `0x140167910` (plays only
  when its third argument is 0) joins the side-checking stub (ninth call): enemy decks silent, Sora's own switch
  (`0x1401F8303`, side 1) passes. Its sound-playing callers: the human boss's `+0x468` `0x140199130` and Hades'
  `0x1403B10C0` (side 2). Menus play sound 3 from their own calls (untouched). Checked (decomp + disasm + exe).
- Checked: tests (434), Windows build, 670 sites. Not checked: in game.

---

## Stage 117: Re:CoM's whole dodge roll at Combat = KH1, version 0.66.0 (2026-11-01)

- Ask (owner, 2026-11-02): "make dodge roll work how it does in ReCoM, not KH1"; chose "The whole roll".
- Re:CoM's roll (Checked (decomp + exe + files)): action 6, start `0x1401F10C0` (untouchable bit `+0x418` 1, turn to the
  stick ≥ 0.3, row 0x16 `p001_070` with root motion), exe `0x140217310` (bit cleared at 600 units = 12 frames,
  moving cancel from 1400 = 28 frames, then stand / walk / run or fall); 3.5 m travel (Re:CoM's clip bakes KH1's).
- At `dodge_roll = recom` (new default): KH1's input rule decides when (Square on the ground; with Guard at run tilt;
  KH1's locks; a command replaces it), then Re:CoM's start runs through the existing hook's original and its exe
  runs untouched; the next KH1 frame takes Sora back fresh. Invincibility Re:CoM's own (decision 17's no-flinch frames
  unused). `kh1` keeps the KH1 roll; `on` = recom; `off` removes the roll (now also with a save loaded).
- No new site. Assumption: Re:CoM calls the root step on a clip's first update.
- Checked: tests (429), Windows build, 629 sites. Not checked: in game.

---

## Stage 116: Silent enemy card stock / reload / refill sounds at Combat = KH1, version 0.65.2 (2026-11-01)

- Ask (owner, 2026-11-02): "make it so that you cant hear enemies stocking cards or refilling their deck".
- Every one goes through Re:CoM's sound play `0x1402AA360(id, -1)` from deck code shared with Sora; the deck / stock
  folder's side is `+0x5C` (1 Sora, 2 enemy). Eight exact calls retargeted to a stub that skips enemy decks (returns
  0, as a play that plays nothing): reload charge 0x13 (`0x140167C01`), gauge step 0x14 (`0x140167BDA`), reload done
  0x15 (`0x140167C93`, `0x140167B01`), deck refill / first deal 0x1B (`0x14016B3E7`, `0x14016A091`), boss stock 0xF
  (`0x14016DCA4`), Marluxia 3's folder 0xF (`0x14016DE3F`). Switched at battle start with stage 2/3's switches;
  Combat = Re:CoM, Riku's story and minigames keep Re:CoM's bytes. Checked (decomp + disasm + exe).
- Not silenced (outside the ask): a human boss's item-card page switch (sound 3). Assumption: no reload voice.
- Checked: tests, Windows build, 652 sites. Not checked: in game.

---

## Stage 115: Revenge: KH2's frame-20 check, Hades untouchable only in his wind-up, Cloud's revenge = his charge, version 0.65.1 (2026-11-01)

- Asks (owner, 2026-11-02, on 0.65.0's questions): Hades "Only the wind-up"; check frame "KH2's frame 20"; Cloud's
  revenge "Homing charge".
- Check timing (bug fix; Checked (disasm)): `+0x68` is zeroed after each change by `0x140242F50`, so 0.65.0 decided on
  frame 1. Now: at frame 20 (Hades 30) in the flinch / blow-away step hooks; an earlier end decided in Re:CoM's
  state-change callback `0x1401A3B80` (the 12 boss classes' state-machine table slot `+0x28` → our wrapper, Re:CoM's
  callback first); once per reaction.
- Hades: his state-5 exe (vtable `+0x168`) wrapped; when `+0xC80` leaves 0 (wind-up `e107_054` → spin `e107_055`) the
  window closes and Re:CoM's `0x1401A2DB0(boss, -1)` turns his hurt bodies back on; the spin takes hits (no flinch).
- Cloud: revenge state 4 (begin `0x1403AE680`, exe `0x1403AF190`): 2 dashes ≥ 50 % HP, 4 below; each turns toward
  Sora ~25 frames then dashes straight at 0.1 m/f (6 m/s): sidestep, Guard (frames 10–36) or Re:CoM's roll avoid it.
- Checked: tests (422), Windows build, sites. Not checked: in game; Cloud's .EPD numbers.

---

## Stage 114: Revenge moves can't be damaged or interrupted, version 0.65.0 (2026-11-01)

- Ask (owner, 2026-11-02): "the humanoid bosses that have revenge value can't be damaged or interrupted during their
  retaliation move".
- Re:CoM's own rule: its hit handler `0x14017F7E0` drops hits while `+0x260` bit 0 is set without `0x2000000`, and
  `0x14017DA80` turns the attacker's hurt bodies off (Checked (decomp)). For the attack a revenge started only, our
  hurt-off wrapper lets Re:CoM's body-off run and every hit source (swings, spells, Stop, turned-back shots, Re:CoM's
  own) reaches the handler without the interrupt bit; the swing / spell record is used up; Guard's flinch on the boss
  skipped. Window ends at Re:CoM's attack-state end `0x14017DA90` (new hook, 8 bytes moved; no branch into them).
- Meter: nothing adds during the window (bodies off: hits never meet the boss, as KH2's add needs contact); keeps
  draining. Revenge tail 0x20 → 0x30 bytes.
- Windows: Vexen 5 s of needles; Hades' state 5 = wind-up + 4 / 8 / 12 s spinning (owner question); Cloud's chase has
  no time limit. Found, not changed: the revenge check fires on the flinch's first frame (`+0x68` zeroed after each
  change by `0x140242F50`), not frame 20 (owner question).
- Checked: tests, Windows build, sites. Not checked: in game; enemy motion lengths.

---

## Stage 113: Gimmick cards: the HP drop at 50 % only, version 0.64.1 (2026-11-01)

- Ask (owner, 2026-11-02): "only drop one at 50% and occasionally when guarding".
- `gimmick::HP_THRESHOLDS` = [50]: the five card-break bosses drop their card once per fight at 50 % of max HP,
  without a stagger (same conditions and spawn as Stage 112). The Guard trigger is unchanged: each block posts
  Re:CoM's card break and the boss's own drop roll decides (Guard Armor 30 %, others their .EPD chance: Assumption).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 112: Gimmick cards: Guard blocks break the five card-break bosses; cards at 75 / 50 / 25 % HP, version 0.64.0 (2026-11-01)

- Ask (owner, 2026-11-02): "interrupting an attack with a finisher is very rare"; chose (a) Guard + (d) HP thresholds,
  for Guard Armor, Trickmaster, Jafar, Parasite Cage, Dragon Maleficent at Combat = KH1, Sora's story.
- (a) Our Guard's block (`win::guard::on_result`, in the pair-dispatch wrapper `0x14016EAD0`) of the boss's own
  attack or a Re:CoM shot it fired (`CBullet` / `CHomingBullet`, shooter `+0x238`) posts the card-break message 3
  (as Stage 110); once per blocked attack (collision + serial `+0x70`); not in / entering state 13, dead or pending.
  Block detection no longer needs the guard-reactions toggle (it gates only the reactions).
- (d) After each hit (KH1 enemy-hit wrapper on `0x14017F7E0`; `+0x274` and the max-HP getter `0x14017F5A0`): crossing
  75 / 50 / 25 % drops the boss's card without a stagger, one card per hit, once per threshold per fight, not on the
  killing hit, no chance roll; a boss's own condition (Trickmaster's table, Jafar not in state 15, Parasite Cage's card
  unused, Dragon's footing not out) makes it wait. Spawned as each boss's own drop call: `0x140163740` then the prize
  factory `0x1401A89C0(0x1401A8A80(), &desc, 0)` with that boss's descriptor. Checked (decomp + disasm + exe).
- Finisher-interrupt break kept. Assumptions: the bosses' own projectile classes aren't linked for (a); HP lost only
  through the hit handler.
- Checked: tests, Windows build, 600 sites. Not checked: in game.

---

## Stage 111: Summons: usable whenever Sora has the MP; Sora invincible through them; Cloud first, version 0.63.0 (2026-11-01)

- Asks (owner, 2026-11-02): "summons should be able to be used as long as you have MP, also, sora should have iframes
  through all summon animations"; "cloud should also be at the top of the summons list".
- KH1's once-per-battle rule dropped: usable in a battle room, on the ground, with whole MP ≥ cost, no summon out (the
  row stays locked while one is out: a second sleight can't start during action 0x16).
- Invincible: the existing damage-check wrapper (`0x1401E6DE0`) sets Re:CoM's protection bit `player+0x418 & 1`
  for the call while a summon is out (action 0x16 from our summon, battle room, same player), so Re:CoM's own early
  exit drops the hit (no damage, no reaction), as Sonic Blade and the roll; checked every frame by the Battle exe
  (`0x1401F9840`); ends with Re:CoM's end action `0x1401F6660`, the battle or a room change. Checked (decomp).
- Cloud first in the list (`summons::list_order`), the others in learn order; KH1's placement kept.
- No new hook, call or site. Assumptions: every hit on Sora goes through `0x1401E6DE0`; a card break of the sleight
  (not a hit) can still end it.
- Checked: tests, Windows build, 577 sites. Not checked: in game.

---

## Stage 110: Gimmick cards: a finisher that interrupts a card-break boss gives Re:CoM's card break, version 0.62.1 (2026-11-01)

- Ask (owner, 2026-11-02, option c): when a KH1 combo finisher interrupts the attack of Guard Armor, Trickmaster,
  Jafar, Parasite Cage or Dragon Maleficent, the boss gets Re:CoM's own card-break stagger, which rolls its drop.
- Built (no new hook): after Re:CoM's hit handler, for those five classes (vtables RTTI-checked), a kind-5 hit that
  ended the attack on a surviving boss posts Re:CoM's card-break message as its resolver `0x14015EAC0` does:
  `0x1401D2770(CBTL+0x58, 0, CBTL+0x20, boss+0x20, 3, none)`; the base handler `0x140181620` case 3 (`0x140181739`)
  ends the attack and sets state 13; each boss's own state-13 begin plays the stagger and rolls the drop (Guard
  Armor 30 %, Parasite Cage 30 % once, the others their .EPD chance: Assumption). Checked (decomp + disasm + exe).
- Assumptions: Re:CoM's message manager runs in KH1 battles (as Stage 108's message 6); how often vs Re:CoM.
- Checked: tests, Windows build, 567 sites. Not checked: in game.

---

## Stage 109: Summons: KH1's Summon list issuing Re:CoM's LV3 summon sleights (Cloud: Omnislash), version 0.62.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Lets get summons working. I want the summon section on the command menu to work like KH1,
  but each summon should do their lv 3 recom summon equivalent, cloud included"; chose MP from Re:CoM's CP (2 / 3),
  KH1's party rule skipped until the party, Re:CoM's damage at both settings, KH1's once-per-battle kept.
- KH1 (Checked (decomp + files)): Summon = the Magic list's last entry (`0x1402944C0`), row 3 opens the list
  (`0x1402937F0`), place `0x14027B870` (x 27; one entry y 326, more 348 − 22n), learn order `save+0x7D0`, greying
  `0x140288F30` (MP, battle mode, ground, once per battle, party), names Word.bin 286–291.
- Re:CoM (Checked (decomp + exe + files)): LV3 = the same summon card ×3 (`0x1401669F0`): Simba Proud Roar LV3 0x35,
  Genie Showtime LV3 0x3E, Bambi Paradise LV3 0x39, Dumbo Splash LV3 0x37, Tinker Bell Twinkle LV3 0x40, Mushu Flare
  Breath LV3 0x3C, Cloud Omnislash 0x42. Started natively: `player+0x82C` = sleight, Sora's vtable `+0x130`
  (`0x1402247A0`); Re:CoM loads the pack (`0x140211790`), spawns the `CSummon*`, hits, ends (`0x1401F6660`).
- Built at Combat = KH1, Sora's story, battle rooms: KH1's Summon row / list / greying / sounds; MP 2 (Simba, Genie,
  Bambi, Dumbo, Tinker Bell) / 3 (Mushu, Cloud), paid once Re:CoM's action is 0x16; row locked while one is out, no
  Dismiss; each summon learns with its first Re:CoM summon card (`0x14015C7F0` wrapper); learn order in a new save
  block `save+0x26AE..=0x26B6` (older saves: KH1 index order, Cloud last); preload on cursor rest; party rule in
  `summons::party_ready()` (always true until the party). Cloud: command 0x35, word 0x8029 "Cloud" (English).
- No new hook or patch; call sites checked at start.
- Assumptions: the start from our wrapper equals Re:CoM's own (no card record needed); the once-per-battle reset at
  the next battle; Cloud's name in every language; `UK_uibin_sum.bin`'s layout (the asset step reports it).
- Asset step re-run: summon words and the Summon panels.
- Checked: tests, Windows build, 539 sites. Not checked: in game.

---

## Stage 108: Gimmick cards: Re:CoM's drops, a Triangle "Stagger" prompt that plays the card, version 0.61.0 (2026-11-01)

- Ask (owner, 2026-11-01): "certain bosses drop "gimmick cards" in their battles. I want these to drop, and when sora
  picks them up, i want there to be a triangle prompt that says "Stagger," and pressing triangle will activate the card".
- Re:CoM (Checked (decomp + exe)): kinds 0x3A–0x41 (type 6), one held at a time, never stocked; played as action 0x19,
  the boss gets message 6 at frame 50 and runs its own reaction. Drops: Oogie (dice), Ursula (tentacle), Darkside
  (summoned Heartless gone), Jafar (Blizzard on its guard; Assumption) drop at KH1; Guard Armor, Trickmaster, Jafar,
  Parasite Cage, Dragon Maleficent drop on a card break (none at KH1: owner question).
- Built at Combat = KH1, Sora's story: Re:CoM's drops (log-only pass-through on the prize factory `0x1401A89C0`);
  pick-up keeps Re:CoM's one-at-a-time count (CBTL `+0x1D0`) with the deck add skipped for the call; the context tab
  offers "Stagger" (command 0xF0, word 901, English everywhere) when Re:CoM's card play would accept it, priority
  object command > Stagger > Sonic Blade; Triangle makes Re:CoM's play writes (`0x140161D60`, `0x14015D5B0(-1)`,
  `0x1401F5580` message 1, `+0x428` bit 9 → `+0x424 |= 0x40`), Re:CoM's action does the rest; forgotten at
  battle end. `docs/recom-gimmick-cards.md`.
- Assumptions: the .EPD drop chances; each reaction's look; nothing needs the card in the deck at KH1.
- Asset step re-run: the word in KH1's font.
- Checked: tests, Windows build, 553 sites. Not checked: in game.

---

## Stage 107: Damage = Re:CoM: Aero level 1 wind; Gravity Break and Zantetsuken skip Re:CoM's boss falloff, version 0.60.1 (2026-11-01)

- Asks (owner, 2026-11-01, on 0.59.0's questions): 1 (b) Aero level 1 gets a wind with the card's value; 3 (b) "skip
  Re:CoM's reduction for these two"; 2 (a), 4 (a) unchanged.
- Aero level 1 (Damage = Re:CoM, battle rooms): card 0x20 (300 × attack / 100, attribute 1), a wind shaped as KH1's
  Aerora wind (btltbl `0x5CF8`: 80 cm × Sora's size, a hit every 30 frames through `0x14016F7D0`, KH1's flinch;
  using level 2's shape for level 1: Assumption), for the barrier's (18 + max MP) × 60 frames; Re:CoM's Aero card
  effect. Field and Damage = KH1: barrier only, as KH1.
- Correction: `0x14015D940` (named "boss rule" in 0.59.0's question) is Re:CoM's bonus against frozen enemies
  (`enemy+0x2B0`; Checked (decomp + exe + files)); Re:CoM's boss reduction is the falloff (`+0x288` / `+0x28C` /
  `+0x290`). Gravity Break and Zantetsuken now skip the falloff (`+0x290` = 1.0 for the handler call, the three
  fields put back; shared helper `win::revenge::Falloff`); frozen bonus, resistance, weak parts kept.
- No new sites or hooks.
- Checked: tests, Windows build, 529 sites. Not checked: in game.

---

## Stage 106: Boss revenge value (KH2's) for humanoid bosses at Combat = KH1, version 0.60.0 (2026-11-01)

- Ask (owner, 2026-11-01): "lets also have humanoid bosses have a revenge value system, similar to how they work
  in their fights in KH2 … borrow their revenge value"; chose story / AS limits, all humanoids, the boss's own Re:CoM
  attack nearest its KH2 revenge move, Re:CoM's boss falloff off for them.
- KH2 (Checked (decomp + exe + files), `docs/recom-boss-revenge.md`): hit adds ATKP `+0x2A` to enemy `+0xD48`;
  limit `+0xD4C` (default 100) set by AI syscall table 2 #76 `0x140431E30`; decays 1 per 60 Hz frame, paused in
  damage motions; checked by motion trigger 28 at frame 20 of the flinches (Hades 30); no reset.
- Limits: Axel 92, Larxene 100, Vexen 150, Marluxia (all forms; parts count on the body) 100, Hades 80 → 50 below
  30 % HP, Cloud 100, Hook / Riku / Riku Replica / hooded figure / Leon 100 (KH2's default).
- Re:CoM side: the 12 boss bodies' factory sizes (`mov ecx, imm32`) raised by 0x20 for the counter (all-or-nothing);
  hooks on the flinch step `0x140184100` and blow-away step `0x140182E50` (6 bytes each), returning at once for
  other enemies; lazy decay at each hit / check minus flinch frames; the check at KH2's frame (or at the flinch's
  end if shorter: Assumption that Re:CoM's flinches can be shorter). Revenge starts the boss's own attack state
  (Axel 4, Larxene 4, Vexen 6, Marluxia 2, Hades 5, Cloud 6 Omnislash, Hook 3, Riku 4) with P1's message-2 writes;
  hooded figure and Leon have no attack state (counter only, falloff kept). Falloff fields put back for the rest.
- Hit values: KH2 Sora's rows (swing 8, finisher 24, air 12 / 28, Fire 8, Blizzard 16, Thunder 32); 10 for hits
  without a KH2 twin (Assumption).
- ini `boss_revenge = on | off` (default on); Combat = KH1, Sora's story only.
- Assumptions: each 'nearest' attack state (chosen from its motions and code); Sora's dt as the clock.
- Checked: tests, Windows build, 526 sites. Not checked: in game.

---

## Stage 105: Damage = Re:CoM: Re:CoM's Gravity / Stop / Aero; KH1's % for Gravity Break and Zantetsuken, version 0.59.0 (2026-11-01)

- Asks (owner, 2026-11-01, on 0.58.1's questions): Damage ini only; Gravity and Stop "Re:CoM's numbers"; Aero
  "Re:CoM's Aero by level"; Gravity Break / Zantetsuken "KH1's % of Re:CoM HP".
- At Damage = Re:CoM (Checked (decomp + disasm + exe + files)):
  - Gravity: card 0x1E / Gravira 0x11 / Graviga 0x18, 20 / 40 / 70 % of current HP through Re:CoM's kind 0xD
    (`0x14017DC50`, gate `param+0x40`, `0x14017DD4C`), then its flatten state `+0xAC0` (`0x14017D626`, 9000-tick hold
    `0x140183CFB`); KH1's squash leaves a flattened enemy alone.
  - Stop: card 0x1F / Stopra 0x12 / Stopga 0x19, 9000 / 15000 / 21000 ticks through kind 8 (gate `param+0x2C`).
  - Aero's wind: card 0x20 / Aerora 0x13 / Aeroga 0x1A, 300 / 600 / 900 × attack / 100 per wind hit.
  - Gravity Break / Zantetsuken: KH1's rule (`0x1402BFA80`): int(current HP × power / 100 × resist[8]), at least the
    record's floor; power 40 + KH1 Strength / 250; on Re:CoM's current HP.
  - KH1's cast, area and order kept; the card interrupt not set when Re:CoM's gate refuses.
- Field: Gravity / Stop keep KH1's rules (no card table); Aero's wind takes Re:CoM's no-card AP as Fire.
- No new hooks; three check-only sites. Damage = KH1 unchanged.
- Assumption: the live tables match the session's SY0002.BIN (the DLL reads the live ones).
- Checked: tests, Windows build, 491 sites. Not checked: in game.

---

## Stage 104: Sonic Blade: Re:CoM's start tint, version 0.58.1 (2026-11-01)

- Ask (owner, 2026-11-01): "Add Re:CoM's tint" (as base Re:CoM's Sonic Blade).
- At our Sonic Blade's start, before the `limit1` effect: `0x1401F7D80(player, 0x1407C5D50, 20.0, 0)`, as Re:CoM's
  SONICRAVE start (`0x140218E05`; Checked (decomp + disasm)). It sets a flag Re:CoM's end action `0x1401F6660`
  (already our special's end) reads to fade back over 20.0.
- Checked: tests, Windows build, sites. Not checked: in game.

---

## Stage 103: Damage setting: Re:CoM damage per equipped Keyblade and Re:CoM enemy HP (default), version 0.58.0 (2026-11-01)

- Ask (owner, 2026-11-01): "depending on which keyblade sora has equipped, it does recom's damage for the
  corresponding keyblade? lets also have Recom's enemy HP"; chose: KH1's normal swings Re:CoM's normal column,
  KH1's finisher Re:CoM's finisher column; spells and specials Re:CoM's card / sleight damage.
- Re:CoM's formula (Checked (decomp + disasm), row pick `0x1401F7520`, card branch `0x1401F7ACC`): power = card
  `BattleCardList` (`0x14087A808`) power at the row's column + `+0x664` (0 at KH1: no enemy card 0x79), + half
  when `+0x420 & 0x4000000`; AP = attack stat `+0x43C` × power / 100; attribute card `+0xB`; then Re:CoM's
  enemy-card rewrite `0x1401E66A0`. Re:CoM's rows 1–7 columns 0, 0, 1, 2, 1, 0, 2 (Checked (exe)).
- New ini setting `damage = recom | kh1` (default recom; KH1 = decisions 12 / 21 as before). At recom:
  - swings: the equipped Keyblade card (`save+0x2674`) at column 0 for normal swings, 2 for KH1 kind-5 finishers
    (Blitz, Ripple Drive, Hurricane Blast too); other KH1 moves the column of the Re:CoM row they are built on
    (table in `docs/recom-kh1-attack.md` § Re:CoM damage setting); field swings keep Re:CoM's no-card AP.
  - enemy HP: Re:CoM's own (no KH1 HP write or max-HP override); KH1 records kept for level, Stop / Gravity
    resist, drops, lock radius.
  - Fire / Blizzard / Thunder: card 0x1A / 0x1B / 0x1C, then the -ra / -ga sleights 0xD..0xF / 0x14..0x16;
    Sonic Blade sleight 9: dashes row 0xF (col 0), finisher row 0x10 (col 2). Cure, Gravity, Stop, Aero unchanged
    (owner questions).
- No Config row yet (Re:CoM's Config slots are full): owner question.
- Checked: tests, Windows build, 488 sites. Assumptions: `player+0x390` exactly in battle rooms; the rewrite
  changes nothing at KH1 (logged). Not checked: in game.

---

## Stage 102: Sonic Blade: Re:CoM's voices and effects per dash; battle only, version 0.57.0 (2026-11-01)

- Asks (owner, 2026-11-01, on 0.56.1's questions): voices "however it works in base ReCoM"; per-dash effects
  "however it works in base ReCoM"; field use "Only in battle".
- Re:CoM's Sonic Blade (Checked (decomp + exe), `exeStockSkill` `0x140218950` SONICRAVE): start 0x8C, six dashes
  0x8D–0x92, finisher 0x93, 1:1 with KH1's wind-up, rows 0x1B–0x20, 0x21. Each part plays a motion of the
  sleight's pack 11009 (start 0, dashes 1, finisher 2); its voices / sounds / effects are that motion's events
  (the sub-pack play `0x1401A3580` points Sora's sleight event players `+0x110` / `+0x120` at it).
- Built: when a KH1 Sonic Blade motion starts, `win::specials::motion_events` makes the same event calls on
  the matching Re:CoM motion (`0x140179A30` / `0x14017A240` effects, `0x140238710` / `0x140239260` sounds);
  KH1's motion, timing, damage, travel unchanged. The asset step no longer writes our SES entries for them.
- Battle only: offer and start need Re:CoM's battle flag (`player+0x390`); the Field exe entry `0x1401FA648`
  and cave `0x1401FB7B2` are no longer patched (Re:CoM's bytes).
- Assumptions: Re:CoM's sleight motions as long as KH1's (events at Re:CoM's frames on KH1's clip); the
  sleight's resources loaded by the first press (logged if not); `+0x390` set exactly in the Battle exe.
- Checked: tests, Windows build, 476 sites (pefile). Not checked: in game; the sleight's event files.

---

## Stage 101: Stock items on the green item row, version 0.56.2 (2026-11-01)

- Ask (owner, 2026-11-01, screenshot of KH1 > Items > Stock, its rows red with the Keyblade symbol): "have all
  the items also be green and have the items symbol".
- Change: `items_page::row_look` gives every stock item row (`Label::Stocked`: the Stock list and the pick
  from it) the item slots' look: the asset step's green row with the item symbol (variant 7, group 0x161 or
  the language copy's), else Re:CoM's green row 0x131. Keyblade slot, Sora row, commands, Yes / No stay red.
- No new address or hook. `docs/recom-m8-items.md` § Stock items green too.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 100: Item-row symbol in every language's camp menu, version 0.56.1 (2026-11-01)

- Found on the owner's PC (asset step run, 0.56.0): `note: camp item row not built: FORM/0100/FO0100.SPR asset
  FR_camp.lad: camp:00 has 367 animation groups, Re:CoM Steam's has 353`. The entry's language copies have 14
  more groups than the base file (Checked: the run's log), as the MP bar's `<LANG>_Cockpit.lad` do.
- Change: the asset step gives each copy the new group after its own groups (base 353, the language copies
  367); the base file must still be Re:CoM Steam's. `kh1-camp-rows.json` format 2 lists them (`item_rows`).
  The DLL picks the loaded set's last group when it is one of them, before each list set-up (no new hook).
- Assumption: the extra groups come after Re:CoM's and 0x131 is the same green row in every copy (the step
  checks 0x131's icon per copy). `docs/recom-m8-items.md` § Language copies.
- Checked: tests, Windows build. Not checked: the owner's PC assets, in game.

---

## Stage 99: Skip the intro tutorial and the Leon battle at Combat = KH1, version 0.56.0 (2026-11-01)

- Ask (owner, 2026-11-01): "If a player has KH1 Combat enabled, have the player skip the leon training battle.
  Also the intro tutorial."; method (a) "Skip the battle room".
- Hook: Re:CoM's battle-entry request `0x140236690` (10 bytes `48 89 5C 24 08 57 48 83 EC 20`, no branch into
  them; Checked (exe)). Acts only at Combat = KH1, Sora's story, Traverse Town (`save+0x1878` = 1), for event
  room 2000 → battle 1100 (intro) or 2002 → 1101 (Leon). Everything else goes to Re:CoM unchanged.
- It does what each battle script's own exit does, in its order (Checked (files): the ABC scripts):
  - intro: clears its 13 session flags, HUD-hide 0, friends-met (0, 0x22) |= 1 then |= 2, room change
    `0x1402360F0(0, 0, 2001)`;
  - Leon: clears its 10 session flags, keep-music (1, 0x32) = 1, HUD-hide 0, `0x1402360F0(1, 0x140233AE0(), 2003)`.
  - Fallback: no floor for the room (`0x14087CBE2[kind]` < 0) or a refusal → Re:CoM's battle runs.
- Runs once per battle request, never per frame. Saves: only the friends-met bits, as after a played tutorial.
- Assumptions: a scene room may change straight to the next scene room (vanilla only does it from a battle
  room); tutorial Leon gives no EXP. `docs/recom-story-skips.md` § Built.
- Checked: tests, Windows build, site bytes (pefile). Not checked: in game.

---

## Stage 98: Green item symbol on the camp's item rows, version 0.55.0 (2026-11-01)

- Ask (owner, 2026-11-01): "for the item slots, the color is correct, but they have the green mickey symbol, but
  lets use the green item symbol"; the plan "Yes, add it".
- Found (Checked (decomp + files)): Re:CoM's item symbol is `camp.lad` sprite 166 (sprite group 83), drawn only by
  the Deck page's type tabs; no camp row draws it, and no call swaps one sprite inside a playing sequence.
- Built: the asset step (`camp_rows`) appends group 0x161 to `camp:00` in Re:CoM's `FORM/0100/FO0100.SPR` (and
  its PC `camp.lad` assets): a copy of green row 0x131 with its icon's sprite group 85 → 83; writes
  `raw/FORM/0100/FO0100.SPR` + `kh1-camp-rows.json`. The DLL gives our pages' rows an 8-entry copy of Re:CoM's
  table `0x1407C11F0` (+0x161) while the item row is loaded; the Stock row and item slots take it. No new hook.
- Fallback: without the manifest (or a `camp:00` without 354 groups) the item rows keep 0x131.
- Assumption: sprite 166 is green in the texture (`FO0100.RTM` not in the session).
- Checked: tests, Windows build, the session's base `FO0100.SPR` patch. Not checked: the PC assets, in game.

---

## Stage 97: Stop works on attacking enemies; its full duration, version 0.54.1 (2026-11-01)

- Ask (owner, 2026-11-01): "stop doesnt seem to work". Log `kh1-recom-stop-0530.log`: eight Stopga strikes
  reached Re:CoM's handler, none entered its stop state.
- Cause (Checked (decomp + disasm)): Re:CoM's enemy hit handler `0x14017F7E0` drops any hit on an enemy in an
  attack (`+0x260 & 1`, `0x14017F8CB`) unless the card-interrupt bit `0x2000000` is set; every other KH1 hit
  sets it (decision 14), Stop's never did (since 0.23.0). Not a regression from 0.50.0. That each of the
  eight strikes met an attacking enemy: Assumption (the old log didn't print it; the new one does).
- Also: KH1's duration was clipped to the i16 carrier (32767 ticks); Re:CoM's `+0x2D8` is i32
  (`0x14017DCC6`, begin copies it at `0x14017C7F4`). Checked (disasm).
- Fix (native, no new hook): Stop sets the interrupt bit for the handler call on attacking, non-immune
  enemies; when the handler entered the stop and the duration is longer than the carrier, the whole
  duration goes into `+0x2D8`. KH1's numbers unchanged. `docs/recom-m7-magic.md` § Stop on attacking enemies.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 96: M9 stage 9c.1: KH1's Sonic Blade; Guard and Dodge Roll in Sora's kit, version 0.54.0 (2026-11-01)

- Asks (owner, 2026-11-01): "just have guard and dodge roll be a part of sora's movement kit from the start,
  dont add them as abilities"; High Jump / Glide / Superglide equipped when Re:CoM gives them; Sonic Blade with
  Re:CoM's sleight effects, no scene tint for now.
- Kit: Guard and Dodge Roll always on at Combat = KH1 (no AP, not on the page); save block v4 (their AP back);
  High Jump / Glide / Superglide auto-equipped once on arrival, removable.
- KH1 (Checked (decomp + files)): offer `0x14028D0F0` (order, range 600, < 160 above, 2 MP), start
  `0x14028D800` (MP on the press frame), `0x14039EAD0` state 0x20 (invincible, root motion only), dashes rows
  0x1B–0x20 + finisher 0x21, follow-ups Rave / Blast on a fresh Triangle in the window; `xl_limit.dat`
  motions 0–2 (30 / 54 / 82 f; travel 5.62 m / 7.34 m).
- Built: asset step converts `xl_limit.dat` entries 0–2 into slots (230–232 with the owner's files) + SES
  entries + words 319 / 321 / 322. The special runs on Re:CoM's action 0x16: `initStockSkill`'s writes for
  case 9 without a card, `exeStockSkill` `0x140218950` hooked (11 bytes) to run the KH1 frame then Re:CoM's
  `procAttackCollision` `0x1401F5DC0`; Field exe action-0x16 entry `0x1401FA648` → 13-byte cave stub at
  `0x1401FB7B2`; Re:CoM's `limit1` start effect; invincible via the damage-check path. Offer in the
  context tab ("Sonic"), follow-ups in the Attack row. Root-motion safety net 0.5 → 1.0 m/f.
- Assumptions: dash 1 at frame 31; Re:CoM's lock target for KH1's; Re:CoM's sound 0xD1 for KH1's
  4003 / 4004; nobody reads the sleight id `+0x82C`. `docs/recom-m9-abilities.md` § 9c.1.
- Checked: tests, Windows build, site bytes. Not checked: in game, KH1 trace.

---

## Stage 95: M9 9b follow-ups: Guard and Dodge Roll from the start, Re:CoM's High Jump / Glide / Superglide, Combo Master 50, Rod max MP, blue rows, version 0.53.0 (2026-11-01)

- Asks (owner, 2026-11-01): Guard from the start; High Jump, Glide, Superglide where Re:CoM unlocks them;
  Dodge Roll equipped from the start; Combo Master at level 50; Rod max MP +1 at 30 / 66; Abilities rows
  blue; "in the customize menu, make these slots be blue with the magic symbol".
- Re:CoM (Checked (decomp + exe)): int variable (0, 0x24) bits 1 / 2 / 4 = High Jump / Glide / Superglide
  (copied by `0x1401F0C50` into `player+0x41C`; set by the event script `0x140145C50` cmds 0x2C / 0x41 and
  bit 4 by the reward pick `0x1401BD9D0` in Castle Oblivion). Which events: Assumption (KH wiki: Monstro,
  Neverland, Room of Rewards).
- Built: unlocks as asked (read at Attack presses and the Abilities page, not per frame); save layout v3
  (ini-given High Jump / Glide / Superglide / Combo Master return when their source is met); the five ini
  keys are now only a no-save fallback. Max MP +1 at Re:CoM levels 30 and 66 (Checked: btltbl Rod list,
  bonus kind 2). Abilities rows, Customize slot rows and picker spell rows: Re:CoM list entry 0x133 (blue,
  magic icon) via `0x140112F10`.
- Checked: 532 tests, Windows build, all site bytes. Not checked: in game.

---

## Stage 94: M9 stage 9b: KH1's ability equip screen with AP; green item rows, version 0.52.0 (2026-11-01)

- Asks (owner, 2026-11-01): equip screen with KH1's AP; new unlocks unequipped; Gravity Break at Re:CoM level
  30; Rod list. "lets make "stock" have the item symbol, and make the box green" / "the same for all the item
  slots". Owner's spell-effects decisions recorded (own toggle, Re:CoM's sounds, after the specials).
- KH1 (Checked (decomp + btltbl)): AP = record +9: 3 at start, +2 at 24 levels (`0x1402852C0`, `0x1402B5660`);
  costs btltbl 0x33D8 (row id − 1); equip refused over max AP, greyed rows (`0x140310650`, `0x140311C20`);
  shared abilities (High Jump, Glide, Superglide) free.
- Built: "Abilities" row in the KH1 camp submenu (Sora / Shared → learned abilities, equipped marked, greyed
  when unaffordable, confirm toggles, AP left / max + cost + used / max). Unlocks: Gravity Break at 30; Rod
  list: Guard 33, Counterattack 57, Air Combo Plus 24 / 45, Combo Plus 87 / 93 / 99; High Jump, Glide,
  Superglide, Dodge Roll, Combo Master by their ini keys (owner question). New unlocks unequipped; 9a saves
  migrated within AP. Each KH1 frame reads the equipped set. Items page: Stock and item slots use Re:CoM's
  list entry 0x131 (green box, icon sprite 278) through its own row setup `0x140112F10`.
- Assumption: English names; sprite 278 = the item icon. No new sites.
- Checked: 528 tests, Windows build, all site bytes. Not checked: in game.

---

## Stage 93: M9 stage 9a: KH1's combo-modifier abilities and the ability state, version 0.51.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Finish everything up to M9"; abilities unlock with Re:CoM (its level-up
  sleights) and, without a Re:CoM source, at KH1's learn levels; equip screen with KH1's AP next.
- KH1 (Checked (decomp + files)): picker `0x1402A6EE0` / `0x1402A6C70` conditions for Stun Impact, Gravity
  Break, Zantetsuken, Blitz, Ripple Drive, Sliding Dash, Vortex, Aerial Sweep, Hurricane Blast; their
  motions, windows, travel and attack records; AP costs (btltbl 0x33D8); learn tables.
- Built: ability block at save+0x269F..0x26AD (unlocked, equipped; AP reserved); Re:CoM-sourced unlocks
  via Re:CoM's own sleight-learned `0x14023AB00` (called, 16 bytes checked; storage save+0x2840); others at
  KH1's Rod-list levels on Re:CoM's level (Ripple Drive 12, Hurricane Blast 18, Aerial Sweep 36, Vortex 84);
  unlocked = equipped until 9b. Equipped modifiers passed to KH1's pickers (only rows that exist). Six
  KH1-only clips on extra action rows; Sliding Dash / Stun Impact / Zantetsuken play Re:CoM's own copies
  (rows 0x45 / 0x44 / 0x46). Rows 0, 3–7, 9, 10, 15 via `kh1_row_bytes`. Stun Impact a 2.5 m sphere at
  `belt1` (Re:CoM's collision); Gravity Break / Zantetsuken damage on current HP; Ripple Drive max MP × 3;
  stun → Re:CoM's kind 7. SES entries for the six clips. Debug `kh1_all_abilities`.
  `docs/recom-m9-abilities.md`.
- Not 1:1 yet: stun length / immunity Re:CoM's; no KH1 effects (M13); Re:CoM's blow-away; side scans see
  only Re:CoM's target (M12). Gravity Break has no KH1 learn level: never unlocks yet (owner question).
- Checked: tests, Windows build, 446 site byte sets. Not checked: in game.

---

## Stage 92: Battle-end softlock: KH1 strikes take Re:CoM's contact point; no new KH1 command after the win, version 0.50.0 (2026-11-01)

- Ask (owner, 2026-11-01): "ending a battle with magic still softlocks the game" (log: the battle manager
  waits on 5 EXP prizes that never arrive).
- Cause (Checked (decomp + disasm); the prizes' non-finite positions: Assumption, the old log read the
  wrong field): the manager `0x1401871D0` waits for its children; an EXP prize (update `0x140231010`) is
  collected only within 2 m of Sora's `+0x4B0` (`0x140231318`), no timeout. Re:CoM's defeat `0x14017BBC0`
  puts an enemy's prizes at the killing hit's body contact `+0x50`, written only by Re:CoM's overlap tests
  (`0x1401777E0` / `0x140176FA0`, run by its collision pass `0x14016EAD0`). Our Thunder / Gravity / Stop
  strikes called the hit apply directly: a kill by them dropped prizes at a stale or garbage point (NaN →
  never arrives → softlock).
- Fix: `win::magic::strike_body` runs Re:CoM's own overlap test (its pair rule) before the apply, as the
  collision pass does. After Re:CoM's win (`player+0x428 & 4`, as `0x1401E9180` refuses card plays) no new
  KH1 Attack / Counter / spell / item starts. Battle-end log shows each prize's real position, flags,
  Sora's prize target; a non-finite defeat point is logged.
- Checked: tests, Windows build, site bytes (two new check-only sites in the collision pass). Not checked: in
  game.

---

## Stage 91: KH1 camp: one Items menu with Sora's Keyblade slot and item slots, version 0.49.0 (2026-11-01)

- Ask (owner, 2026-11-01): "I would like one menu inside of the "KH1" menu for "Items" in "items," i want a
  slot for Keyblade, and all the item slots for items. eventually if we add accessories as well we will have
  slots for them there but dont worry about that for now".
- Built: the KH1 submenu is Items, Customize (the Keyblades row goes). Items → Sora: a Keyblade slot first
  (its name; stats panel Strength / Magic / Reach; confirm opens the owned-Keyblade list on the page, equip
  as before; cancel back to the slot), then the item slots (8a.2's KH1 rules unchanged); Stock as before.
  Sora's list is a list of slot kinds (accessories later go between). No new sites.
- Assumption: the Keyblade list keeps our old picker's behaviour (KH1's weapon screen not read); its scroll
  bar is made and deleted while the page stays open.
- Checked: tests, Windows build, all site bytes. Not checked: in game.

---

## Stage 90: KH1 casts in the field: Keyblade shown, spell effects loaded; Customize rows show their buttons, version 0.48.0 (2026-11-01)

- Asks (owner, 2026-11-01): "when casting spells, soras keyblade isnt visible, and the VFX dont appear";
  "for the customize menu, instead of saying 'shortcut 1, shortcut 2 etc' it should be the corresponding
  button".
- Keyblade (Checked (decomp)): Re:CoM draws it in the field only for its swings / Guard (`0x1401F5D30`);
  its magic never does (battle only). A KH1 cast now calls Re:CoM's same draw at the command.
- Effects (Checked (decomp + exe)): spell effects come only with Sora's battle resources (PLAYER group
  1001, effect groups 2000–2060), loaded by the battle scene (`0x1401AEA20` → `0x140211960(0)`) and dropped
  on camp open (`0x140211560`) unless Re:CoM's flag (0, 0x60). At Combat = KH1: the field state's player
  load (`0x1401AF933`, 5 bytes) also runs `0x140211960(0)`; the two camp-open unloads (`0x1401A95BD`,
  `0x1401A99F9`) keep it; the Config switch to KH1 loads it. Re:CoM's own calls only.
- Customize: rows and help use Re:CoM's font button escapes (`F5 67` Triangle, `F5 66` Square, `F5 64`
  confirm, `F5 68` L1; `0x1403480C0` → `0x140345B40`, following the controller / bound key), names if the
  font lacks them.
- Not measured: the field room load time with group 1001.
- Checked: tests, Windows build, all site bytes. Not checked: in game.

---

## Stage 89: M8 stage 8a.4: KH1 item drops from enemies, version 0.47.0 (2026-11-01)

- Ask (owner, 2026-11-01): items dropped by enemies at KH1's rates.
- KH1 (Checked (decomp + exe + files)): each enemy's defeat routine passes a table of three (chance, item)
  pairs to syscall 122 (the path that also grants EXP, syscall 167); `0x1402ABC40` rolls one draw per
  pair, below chance × Lucky Strike. Every KH1 twin's table read from the player's files (asset step,
  `kh1-stats.json` format 6).
- Built: wrapper on Re:CoM's defeat card prize `0x140180D20` (9 bytes): after Re:CoM's prizes, the twin's
  KH1 table rolled; each battle item made by Re:CoM's prize factory `0x1401A89C0` as a new kind 0xD8,
  launched like EXP orbs, ending like them (collected, or ~10 s then fade). Re:CoM's look table copied
  with a 14th row (0xD6's effect), its two loads (`0x140231676`, `0x140230700`) pointed at the copy.
  Wrapper on the prize add `0x1401F4AB0` (10 bytes): kind 0xD8 → KH1's pickup rule, Re:CoM's sound.
  `docs/recom-m8-items.md` § 8a.4.
- Not KH1's: Re:CoM's launch and 10 s life (KH1: stays, ≤ 8); refused pickup loses the item; only battle
  items made; Lucky Strike 1 (no abilities yet); Re:CoM's random float.
- Checked: tests, Windows build, all site bytes against the exe. Not checked: in game.

---

## Stage 88: M8 stage 8a.3: KH1 items in the Moogle shop (a fifth Items tab), version 0.46.0 (2026-11-01)

- Ask (owner, 2026-11-01): items in the Moogle shop; a fifth "Items" tab, one item per purchase, 1 Moogle
  Point per KH1 munny.
- KH1 (Checked (files)): every KH1 item shop sells Potion 60, Hi-Potion 150, Ether 200.
- Built: wrapper on the pack screen update `0x1403A6930` (15 bytes checked): Left from tab 0 / Right from tab
  3 opens tab "Items": Re:CoM's packs released as its close state does, three made with Re:CoM's pack
  constructor (never sold out), pictures = Re:CoM's Potion / Hi-Potion / Ether card faces (`C2DCard`);
  input, sounds, help lines, cursors, payment (tally, trophy, setter, window) Re:CoM's; buy = KH1's pickup
  rule; refused when no room or too few points. Leaving rebuilds Re:CoM's tab. `docs/recom-m8-items.md`
  § 8a.3.
- Not yet: the tab's own art (needs `FORM/0109` dumped); the screen is unreachable when the card
  collection is full (Re:CoM's rule). Owner questions in the doc.
- Checked: 483 tests, Windows build, all 408 site byte sets. Not checked: in game.

---

## Stage 87: M8 stage 8a.2: KH1 Items page in the KH1 camp entry, version 0.45.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Finish everything up to M9" (M8 items).
- KH1's Items screen (Checked (decomp): `0x14030D160`, `0x14030CB50`, `0x14030F330`, `0x14030FA10`, `0x14030D7B0`,
  `0x14030EEB0`, `0x14030F160`): owner (Sora / Stock) → item → Use / Move / Remove / Equip / Discard;
  repeats allowed; 99 cap; cursor to the first empty slot after Equip; stock sorted by the row's key.
- Built: "Items" row in the KH1 camp submenu on the Customize page type (Re:CoM's camp list, cursor,
  sounds), KH1's rules; Use heals via Re:CoM's `0x1401E59A0` then Re:CoM's stats copy `0x1401F0120`
  (callee, 16 bytes checked). `docs/recom-m8-items.md` § 8a.2.
- 8a.3 (Moogle shop) not built: Re:CoM's shop lists only card packs (2×2 pack grids, per-pack data);
  KH1 sells Potion 60 / Hi-Potion 150 / Ether 200 munny at every stage, with a quantity and a
  destination. Owner question (how to sell in the Moogle shop).
- Not KH1's: menu words ours (English), no target list, refusals on the help line.
- Checked: 474 tests, Windows build, all 376 site byte sets. Not checked: in game.

---

## Stage 86: M8 stage 8a.1: KH1 items: Items list, slots, stock, use, version 0.44.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Finish everything up to M9"; items from the Moogle shop and drops, healing
  Re:CoM's item-card amounts (owner's choice; see below).
- KH1 (Checked (decomp + files)): btltbl item rows (0x1A58, 0x14 bytes); use = motion 0x3E (82 f, free from
  74), hand moment f34, heal f65 (task `0x1402C2880`); slot cleared on use, never refilled from stock;
  pickup fills an empty slot, else stock (≤ 99); new game 3 empty slots. A hit does not return the item
  (only death does; the brief's "hit returns it" was wrong).
- Built: Items list in the command menu (KH1's green panels from `exchange/UK_uibin_item.bin`, item
  names in KH1's menu font, six languages); a use runs KH1's frame on Re:CoM's item action 0xE
  (`exeItem` `0x1401ED6E0` hooked, 10 bytes; field exe's action-0xE table entry `0x1401FA628` pointed at
  a 13-byte stub in padding `0x1401FAEA2`); Re:CoM's own item clip (row 0x2D = KH1's motion baked);
  Re:CoM's heal `0x1401E59A0` and effect; MP via `win::mp::add_points`. Save layout v2 at
  save+0x2685..0x269E (slots, stock). Debug `kh1_test_items`. `docs/recom-m8-items.md`.
- Owner's "Re:CoM's item-card amounts": Re:CoM's item cards heal no HP (they reload cards; Checked
  (decomp + files)). Provisional: KH1 × 560/120 (Potion 140, Hi-Potion 280, Elixirs full); Ether +3 MP.
  Owner question.
- Not KH1's / not yet: Sora only (no party list), no hand effect at f34, always 3 slots.
- Checked: 462 tests, Windows build, all 375 site byte sets against the exe, HUD dump test. Not
  checked: in game.

---

## Stage 85: M7 stage 7.13: the KH1 camp entry with Keyblades and Customize, version 0.43.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Finish everything up to M9" (M7 7.13: "KH1" camp entry + Customize, plan
  row 7.13).
- Built: our camp entry renamed "KH1", opening a submenu made the way Re:CoM's Deck submenu is (its
  own 7 entry buttons relabelled with the Deck's calls; cursor, glove, fades, breadcrumb, sounds
  Re:CoM's): Keyblades (unchanged page) and Customize (new): KH1's three shortcut slots, each with a
  picker of learned spells at their level + None, no duplicate check (KH1 `0x140302950`,
  `0x140302740`, `0x1402857C0`; Checked (decomp)). Writes our save slots (save+0x2681..0x2683); the L1
  list reads them at once. Only at Combat = KH1 in Sora's story. No new hook sites.
- Changed: the camp runs our code on `camp+0xBC` (the confirmed top entry) instead of `+0xB8`; the
  main panel's layer 2 is hidden only while a page from the submenu is open (Re:CoM's Deck rule).
- Not KH1's: "Shortcut 1–3" / "None" words are ours (KH1's sysmsg not in the asset step); no glyphs in
  the rows (the help line names the button). `docs/recom-keyblade-equip.md` § Stage 7.13.
- Checked: tests (446), Windows build, all site bytes against the Steam exe. Not checked: in game.

---

## Stage 84: M7 stage 7.12: KH1's shortcut menu on L1, version 0.42.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Finish everything up to M9"; shortcuts on L1 like KH1 (owner's choice). Owner's
  M7–M9 decisions recorded in `docs/design-philosophy-instructions.md`.
- KH1 (Checked (decomp + exe + files)): slots save +0x844..0x846, spells only; L1 held swaps the
  shortcut list in (`0x1402937F0`); Triangle / Square / confirm cast slot 1 / 2 / 3 through the menu's
  commit (`0x140293480`); greyed → error sound 4; busy → dropped; L1 suppresses Attack, roll / guard,
  context; panel `exchange/UK_uibin_cut.bin` at (margin + 16, 282), words 0xE6 / 0x73.
- Built: shortcut list in the command menu logic (`command_menu::step_shortcut`), slots in our save
  block (save+0x2681..0x2683, KH1's fill-on-learn, old saves filled once in KH1's order), casts via
  `magic::issue`, panel groups in `kh1hud:00` from KH1's `cut` layout and `Icon.tm2` glyphs; Square
  withheld from Re:CoM's request bits while the list is up; save / warp point Triangle calls
  (`0x1401C530E`, `0x1401CC31B`, 5 bytes each) retargeted so L1 + Triangle casts instead of saving.
- Not KH1's: Cure / Aero cast on Sora at once (no party list); a jump always takes the frame.
  Assumption: L1 = pad bit 4, the keyboard key is Re:CoM's L1 binding.
- Checked: tests, Windows build, all site bytes against the Steam exe, offline render of the panel.
  Not checked: in game.

---

## Stage 83: MP ring draws; one centre with the HP ring; ticks; room plate top left; save point MP; spell voices, version 0.41.0 (2026-11-01)

- Asks (owner, 2026-11-01): "the blue of the MP bar doesnt show"; "lets also make sure that all the MP
  segments are evenly spaced"; "the MP bar should also be the same thickness as the HP bar, and the
  bars should start from the same place"; "the room indicator should be in the top left of the
  screen"; "standing on a save point should refill MP"; "casting cure makes sora say \"thunder!\" etc".
- MP ring (Checked (decomp): `0x140247850` / `0x14024BB10`, a control's later layers are drawn relative
  to its first layer's first sprite): MP back, MP, charge, ticks now one layer a control (four
  controls). Ring rebuilt about Re:CoM's HP centre (444, 317), quarters r × r; HP control origin
  (444, 317) (was 0.25 up-left); new ring group TICKS (KH1's 90° / 180° ticks on the seams); 562
  groups. `docs/recom-kh1-hud.md` § 0.41.0.
- Room plate: top left (`hud_room_plate_top_left`, old key read too), moved by the gauge margin
  `+0xA0`. Assumption: right-anchored plate.
- Save point: wrapper on `0x1401C5180` (13 bytes checked): KH1's 1 MP every third frame and the charge
  emptied while Sora is on it (`+0x2C0`). Checked (decomp + exe): KH1 `0x1402ACE20` / `0x1402AD0B0`.
  `docs/recom-m7-mp.md`.
- Spell voices: KH1 cast motions pass Re:CoM's own SES variant (`0x14021FEF0(pl, 4)`, vtable `+0x110`
  checked), as `initMagic` does. Checked (decomp + files). Not KH1's: Re:CoM's voice frame (3 / 2,
  KH1 0) and one take a spell. `docs/recom-m7-magic.md` § Spell voices.
- Checked: tests, Windows build, all 349 site byte sets against the Steam exe, offline render of the
  ring. Not checked: in game.

---

## Stage 82: KH1 menu sounds: the SCD's sound table read; asset kept whole, version 0.40.1 (2026-11-01)

- Ask (owner, 2026-11-01): "notice the quality of the menu and the sounds playing … lets try to get
  this 1:1 in ReCoM". 0.40.0's asset run printed menu sounds 1 (move) and 3 (back) as "stream 1 / 3:
  empty".
- Finding (Checked (files): the owner's `se000.win32.scd`): the SCD maps each sound to its streams
  through a table at `tables + 0x20` (16-byte entry per sound, then (u16 track, u16 stream) pairs).
  Move 1 → stream 221, back 3 → 220, error 4 → 219, open 6 → 218 (stereo, 0.1–0.25 s audible); streams
  0–3 and 5 are empty slots. 174 of 193 tracks' waits equal their stream's length exactly. What the game
  plays never depended on this (both drivers resolve the table; same driver body, SCD unchanged); the
  step's printout, manifest `stream` and preview WAVs were wrong.
- Finding: the 1280 bytes past the SCD's size word are the end of stream 222, not padding (0.39.2's
  Assumption was wrong).
- Change: asset step reads the sound table (`streams_of`), prints / previews the right stream, writes
  `streams` per id in the manifest; keeps the asset whole (note when streams pass the size word).
  DLL: `scd_head_ok` takes a size word up to the pack's size (was: equal). Manifest format unchanged.
- Checked: files (table, durations), tests. Assumption: Re:CoM's driver reads past the size word as
  KH1's does (both get the whole asset). Not heard in game.

---

## Stage 81: MP ring drawn as Re:CoM draws its HP ring; closing outline; follows the gauge; room plate top right, version 0.40.0 (2026-10-31)

- Asks (owner, 2026-11-01): "the MP bar doesnt have the outline at the bottom of the bar", "the health
  bar moves, can we make the MP bar move at the same time?", "the bottom right of the screen shows the
  type of room. can we move this to the top right", "a resolution difference between the MP bar and the
  health bar … maybe built the same way".
- MP ring: 561 groups in kh1hud:00 (manifest "mp_ring"): KH1's three quarter-ring pictures whole in
  the atlas, cut per frame by Re:CoM's scissor sprites (`0x14024B910`, reset `0x14024A3F0`) and rotated
  as Re:CoM's HP ring (angles negated on load `0x14024B4E0`); no pre-rendered pictures. Differences:
  the charge's partial end in 1-unit steps without KH1's soft fade; max-MP ends in the right
  quadrants up to 0.9 unit off. Checked (decomp + offline render).
- Outline: a 2-unit black bar under the ring's 9 o'clock start (KH1 closes it with the label plates).
- Shake: the ring's controls hang from the HP control `+0x78` (`0x140248650`), own colour.
  That Re:CoM moves `+0x78` itself: Assumption (no movement keys in its groups; Checked (files)).
- Room plate: wrapper on `CNaviMap`'s plate set-up `0x1401E44A0` (15 bytes checked): its two
  controls (`+0x128`) 200 units up, under the minimap; `hud_room_plate_top_right` (default on).
  Measured on Destiny Islands only. Not seen in game.

---

## Stage 80: KH1 menu sounds: se000's padding left out, version 0.39.2 (2026-10-31)

- Owner's asset run of 0.39.1: "se000.win32.scd: SCD says 15277472 bytes, has 15278752" (refused).
- The asset holds 1280 bytes after the SCD's own size word; the step now keeps the SCD's said size
  (the bank Re:CoM registers) and notes the padding. Checked (owner's run); the padding being
  padding: Assumption (zeros after the last stream).

---

## Stage 79: KH1 menu sounds: entries of se000's one SCD, version 0.39.1 (2026-10-31)

- Owner's asset run of 0.39.0: "amusic/se000.dat has none of KH1's menu sounds (its sound assets:
  [se000.win32.scd])".
- KH1 (Checked (decomp + exe)): bank 2 (`0x1404D5EB8`) is `se000.win32.scd` (number atoi(name+2) = 0,
  registered at start `0x14017A080(2, …)`); `SslSeCreate(2, id)` (`0x1400DE460`) plays entry `id` of it
  (`CreateSound(h, id, 0, 1, 0)` at `0x1400DE50E`). Re:CoM's create does the same with `id % 1000`
  (`0x1402F314E`, the same engine function). So the whole SCD is bank 9000 and sound n plays as
  9000000 + n. Asset step: the SCD unchanged, per-id info + preview WAVs. Not heard in game.

---

## Stage 78: KH1's own command menu sounds, version 0.39.0 (2026-10-31)

- Asks (owner, 2026-11-01): "going through the command menu plays the wrong sounds" → KH1's own sounds
  (owner's choice); "notice the quality of the menu and the sounds playing … lets try to get this 1:1".
- KH1 (Checked (decomp + exe)): `0x14017A040(id)` plays bank 2 sound `id` = the PC asset
  `se<id as 6 digits>.win32.scd` (lookup `0x1400E0A50`, `0x1400DB1F0`); menu ids 1 move, 3 back,
  4 error, 6 open; no sound on a command / spell issue (the owner's video agrees: 16 moves one
  sound, Magic's open another, no confirm sound).
- Asset step: ids 1/3/4/6 from `amusic/se000.dat`'s assets, whole SCDs, into kh1-menu-sounds.bin /
  .json. DLL: hook on Re:CoM's bank map `0x1402A8560` (13 bytes checked): groups 9000–9999 are
  ours; on first play the SCD is registered with Re:CoM's `0x1402F5D70` (Re:CoM's bank list owns
  it) and played by Re:CoM's `0x1402AA360` as 9001000 / 9003000 / 9004000 / 9006000. Re:CoM's ids
  stay the fallback. Not heard in game; se000.dat holding these ids: Assumption (checked per id
  by the step).

---

## Stage 77: KH1 command menu words at KH1's brightness, version 0.38.1 (2026-10-31)

- Ask (owner, 2026-11-01): "for the command menu, lets try to get the resolution of the graphics to be
  greater than how it is … lets try to get this 1:1" (video of KH1's menu).
- Side by side at the same size (owner's 2560 × 1440 screenshot vs the KH1 video): the art is the
  same PC textures at the same density (KH1 PC's menu pictures are 2× too; Checked (files)); the
  visible difference was the words: ours pure white, KH1's off-white. The baked glyphs are KH1's grey
  doubled, so the level is bright / 255 (0xE6 → 230), not bright / 128 (clipped to white). Words in
  rows, the cursor row and the tab. Not seen in game.

---

## Stage 76: Re:CoM's HP gauge at KH1's place, turned 180°, with KH1's MP ring, version 0.38.0 (2026-10-31)

- Ask (owner, 2026-11-01): "Lets just keep the KH1 command menu and the triangle prompts. For the health
  bar, is it possible to just get the Re:CoM health bar, place it in the same place as the KH1 health
  bar, and rotate the ReCoM green health so its in the same orientation?" (mockup confirmed; MP
  "Around the ring").
- Re:CoM's ring is scissored from parts' corners with no min / max (`0x14024A3F0` → `0x1402643D0`):
  a turned control would empty the scissor. So the asset step adds `gauge:01` groups 7–9 again,
  turned 180° (47–49), in gauge.lad and its 5 copies; the DLL swaps layers 1–3 of the HP control
  for them once at init (`0x1402482F0`, layer replace) and moves the control to KH1's ring centre;
  face and disc stay upright. Re:CoM runs the gauge itself after that. Checked (decomp + exe +
  offline render of the owner's UK_gauge.lad).
- MP: the KH1 cluster's MP back / MP / charge / ticks only, 1:1 outside Re:CoM's ring (42–55 vs
  29–43). Keys `hud_recom_hp_kh1_place`, `hud_kh1_mp_ring` (default on); `hud_kh1_gauge` default off.
  Not seen in game; Re:CoM's gauge art not rendered offline (not in session).

---

## Stage 75: HUD pieces as settings, Re:CoM's LEVEL UP, smooth rings, tab seam, prize diagnostics, version 0.37.0 (2026-10-31)

- Asks (owner, 2026-11-01): "I want to be able to pick and choose which elements i want", "I want
  the ReCoM level up message instead of the KH1 level up hud", "the health bar has this grainy harsh
  looking texture ... for the MP, youll notice that line thickness varies", "the triangle prompt hud
  has a weird like line through the middle of it", "sora is frozen after battle again".
- `settings::HudParts` (7 ini keys, defaults = the owner's picks); `kh1_hud::gauge_on` / `menu_on`;
  Re:CoM's pieces hidden per bit. H8 off. Config rows: no free item slots (owner question).
- Rings: 4 × 4 bilinear samples a PC texel, cut edges + half a texel (offline render: even
  outlines). Seam: band 1 unit up under the stripe.
- Stuck battle end: the manager waits on 5 `CPrize` (log); the battle-end line now gives each
  prize's kind, words +0x240..+0x260 and position (diagnostics). Not seen in game.

---

## Stage 74: Context tab at save points and warp points, version 0.36.1 (2026-10-31)

- Ask (owner, 2026-11-01): "standing on a save point and using the purple teleporter thing dont
  prompt the triangle to interact".
- Re:CoM (Checked (decomp + exe)): `CGimmickSavePoint` (kind 0x2004, update `0x1401C5180`, in range
  `+0x2C0` = 1) and `CGimmickWarpPoint` (0x2005, update `0x1401CC280`) read Triangle from the pad
  themselves (`0x14029B550`) on the lock / nearest target (`player+0x850`, else `+0x84C`) with
  Sora's action 1..4; the field action `0x1401E80C0` doesn't handle them.
- KH1: the save point's context command 0x36 "Save" (message 293, `0x1402AD11F`); no warp word
  in KH1: "Examine" (5) built (owner choice). Check-only sites; no hook. Triangle still saves /
  warps as before. Assets: words 5 and 293 added (asset step re-run). Not seen in game.

---

## Stage 73: HUD fixes: HP in KH1 units, Re:CoM lock-on, menu text / battle colour, smooth rings, crash notes, version 0.36.0 (2026-10-31)

- Asks (owner, 2026-11-01): "the font is oddly positioned when hovering over an option. the health
  bar and MP bar are REALLY ugly looking", "lets just use Re:CoM's lock on graphics", "lets have
  Re:CoM's health bar" (+ remove LOCK ON!), "when in battle, the command menu still has a blue
  tint", "losing health doesnt affect the health bar, and my game just crashed".
- HP: Re:CoM HP × 120 / 560 (Re:CoM's cap `0x230` = KH1's `0x78`), rounded up. Checked (decomp).
- Lock-on: H5 / H7 off; Re:CoM's cursor and enemy gauges back (mask bit 0x10 no longer added).
- Menu: cursor-row word y 22 (owner's KH1 screenshot); battle selection gradient red
  (`0x14027A2B0`, Checked (decomp)).
- Rings: PC bake samples bilinearly (offline render). Crash notes: `win/crash.rs` (vectored
  handler, log only). Crash cause: not known yet (no dump). Not seen in game.

---

## Stage 72: KH1 FM context tab (green Triangle prompt), version 0.35.0 (2026-10-31)

- Ask (owner, 2026-11-01): "the menu shouldnt have the context option at the bottom, it should behave
  like the KH1 final mix version, where the green triangle prompt appears when interacting with
  something".
- KH1 FM (`0x140278B40`, `0x140277CF0`, `0x140279FC0`; Checked (decomp + exe + files)): the root
  list is Attack / Magic / Items / row 3 (Summon once learned, else KH1's greyed dash); the context
  is never a row: a green tab (`com` objects 4–7 widened, picture 2's PC-only rows, vertex
  colours) at (list x + 9 + slide, 250) over the COMMAND header, the word at + (18, 7), the
  Triangle (`0F 46` font escape, `Icon.tm2` 0xE411) at (list x + 6 + slide, list y − 3); slide
  x −10 → 0 and alpha 0 → 1 over 14 frames (7.3 read the alpha as an X scale). Triangle runs it.
- Logic: `Scene::tab` replaces the row-3 context; confirm / Right on row 3 is the error sound.
  KH1 look: plate, Triangle and word controls. Re:CoM look: the highlighted plate tinted green
  at (8, 232), no Triangle (none found in `Cockpit`). kh1-hud.json format 3 (DLL rejects 2).
- Checked: tests, Windows build, offline render of the bake. Assumption: the PC's icon remap
  (40 × 48 cell drawn 16 × 19 units). Not seen in game.

---

## Stage 71: KH1 HUD polish: KH1's menu font, fourth row, no seams, version 0.34.0 (2026-10-31)

- Ask (owner, 2026-11-01): "the bars are all choppy, the text is wrong, theres no bottom row to the
  command menu, lets fix this".
- Words: KH1's system font (`sysfont` PC texture + `exchange/US_font_data_tbl.bin`), not
  `Theater`'s `menufont`; "Attack" 44.3 units (44.5 on the owner's screenshot). Checked (files +
  decomp `0x1402D1570`, `0x1402D2050`). Vertical place by eye: Assumption.
- Fourth row: KH1's look always draws root row 3 (context command, else "Summon" greyed) with
  the last-row frame (DLL `win/command_menu.rs` `with_summon_row`).
- Seams: own atlas boxes for every rect drawn from a whole picture, clear texels take their
  neighbours' colour, the PC ring quarters cover their cut edges. Checked: offline bilinear
  render of the bake. Not seen in game.

---

## Stage 70: KH1 HUD: straight HP bar drawn without brackets, version 0.33.1 (2026-10-31)

- Ask (owner, 2026-11-01): "for some reason the hp is segmented".
- Cause: KH1 PC's pictures 5 / 7 are repacked where the straight HP / MP pieces sit; `g_s` keeps
  the PS2 UVs, so the PC copy showed bracket pieces every 26 units (reproduced offline). Asset
  step: rects whose PS2 coverage is lost on PC are redrawn from the PS2 texels at 2× (40 rects on
  the owner's files); everything else keeps the PC art. Checked (files + offline render of the
  bake). Assumption: KH1 PC remaps those UVs somewhere we have not read. No DLL change.

---

## Stage 69: KH1 HUD H8, LEVEL UP!! pop-up, version 0.33.0 (2026-10-31)

- Re:CoM's `0x14020C9A0` kind 4 is the level-up (Checked, caller + sound 0x36); hook
  (`48 89 5C 24 10 56 48 83 EC 20`) shows KH1's LEVEL UP!! box at (16, 16) (drop in, hold, 8-frame
  fade) instead of Re:CoM's popup; Re:CoM's level-up choice stays. Not mapped: EXP (KH1 floats it
  per enemy, Re:CoM counts at battle end), munny / Moogle points, item windows (M8). Not seen in
  game.

---

## Stage 68: KH1 HUD H7, KH1's lock-on cursor, version 0.32.0 (2026-10-31)

- Wrapper on Re:CoM's lock-on marker update `0x140209D30` (`40 57 48 81 EC 80 00 00 00`): Re:CoM's
  cursor layer hidden, KH1's `lockon_c` on our projected control (`0x1402471D0`) at the same
  lock point, not scaled by distance (KH1). Not seen in game.

---

## Stage 67: KH1 HUD H6, Donald's and Goofy's gauges, version 0.31.0 (2026-10-31)

- KH1's geometry and faces top-right, shown while Re:CoM has them out as friends: hooks on
  Donald / Goofy appear `0x1401DDC20` / `0x1401DF9C0`, CFriend exit `0x1401D94D0` and dtor
  `0x1401D90F0` (first bytes checked). Re:CoM's friends have no HP / MP: full rings. At Combat =
  KH1 there are no friend cards, so they show only at Combat = Re:CoM with `hud = kh1`. Not seen
  in game.

---

## Stage 66: KH1 HUD H5, LOCK ON! and the target gauge, version 0.30.0 (2026-10-31)

- KH1's rules: origin (16, 24), +28 under the plate, body min(max HP / 2, 150), one bar per 300 HP,
  marker on the top bar, 5-frame gauge / 6-frame plate fades; follows Re:CoM's lock each battle
  frame from the request-bits wrapper; max HP from `0x14017F5A0`. Not seen in game.

---

## Stage 65: KH1 HUD H4, KH1's command menu look, version 0.29.0 (2026-10-31)

- At `hud = kh1`: KH1's `com` / `com_battle` / `mag` pieces, field look in the field, battle
  look in battle rooms (title red over 20 frames), Magic list in KH1's purple, Summon row greyed;
  7.3's rules unchanged; `hud = recom` keeps 7.3's look. Words: KH1's menu font glyphs (Theater
  `uk/font/menufont`) baked as sprite parts per word and language (Re:CoM's font format isn't
  decoded: owner question). Not seen in game.

---

## Stage 64: KH1 HUD H2b, shake, slides, HUD fade, soft arc ends, version 0.28.1 (2026-10-31)

- Every distinct piece set of Sora's cluster (852 units) gets played groups baked at the asset step:
  KH1's three hit shakes (10/22/38 frames), two size-class slides, the 10-frame fade out / in; the
  DLL swaps the held layers for these units only while a motion runs (play-from-time `0x140248550`,
  layer time `0x140248000`, first bytes checked). KH1's 0.7° soft end baked into the HP, charge and
  danger arcs (Checked, decomp + files). KH1 restarts MP / charge fades from scratch (H2 already).
  Not built: back-arc end strips. Not seen in game.

---

## Stage 63: M7 stage 7.11, spell unlocks, version 0.28.0 (2026-10-31)

> Decision 34: base level with the Re:CoM magic card's first pickup; -ra / -ga at Re:CoM bosses

- Hook on Re:CoM's "first obtained" `0x14015C7F0` (`48 83 EC 28 B8 FF 00 00 00`), which every
  card pickup passes: magic cards (kinds 0x1A–0x20) give level 1; a boss's enemy card's first
  pickup (= its first defeat) raises its spells one level each (table in
  `docs/recom-m7-unlocks.md`: two bosses per spell). Levels at `save+0x267A`, raised only; old
  saves sync once (bit 0 of `save+0x2684`). `kh1_all_spells` still overrides. Owner to review
  the table. Not seen in game.

---

## Stage 62: Thunder's charge effect, version 0.27.4 (2026-10-31)

- Thunder's cast now plays Re:CoM's own Thunder charge (its cast setup at `0x14022062B`, 108
  bytes checked: on the Keyblade's bone 2, kept on the blade) as Cure / Gravity / Stop / Aero do.
  Not seen in game.

---

## Stage 61: Stop's held hits at KH1's pace, version 0.27.3 (2026-10-31)

- KH1 (Checked, decomp): each held hit after the first lands 6–11 frames after the previous
  (`6 − (int)(r·−6)`). In the existing enemy-hit wrapper, Re:CoM's per-landing timer is set to
  `gap·50 − 2·dt` after each landing; Re:CoM's 8-hit cap kept. Checked sites `0x140184910`,
  `0x14018499D`, `0x1401A5A7F`. Not seen in game.

---

## Stage 60: Gravity: KH1's squash under the falling sphere, version 0.27.2 (2026-10-31)

> Decision 38 asked for a slow; KH1 has none: its field `+0x480` only scales the model's height
> (Checked, KH1 decomp + .mag). Built KH1's squash; owner question.

- Enemies whose bodies touch the falling sphere (170 cm × scale) squash toward 0.8 height with
  KH1's wobble `(s−0.8)·0.95+0.8−sin φ·0.05` (φ += π/2 a frame) and spring back 0.05 a frame,
  through Re:CoM's own scale getter / setter (`0x1401A4A20`, `0x1401A7050`). Runs only while a
  sphere falls or an enemy springs back. Not seen in game.

---

## Stage 59: Fire's release step as KH1, version 0.27.1 (2026-10-31)

> Decision 38: Fire's release step is KH1's 15 cm curve

- In the existing root-step wrapper (`0x14024DCA0`), during a KH1 cast playing row 0x38
  (`p001_063`), Re:CoM's per-update travel is replaced by KH1's curve (−15 cm at frame 10, −10 cm
  at 60) along Re:CoM's line. Only Fire's ground release differs (air and other casts match;
  Checked, both games' motion files). Not seen in game.

---

## Stage 58: KH1 HUD H3, Re:CoM's HUD hidden, version 0.27.0 (2026-10-31)

- At `hud = kh1`: Re:CoM's HP gauge, FRIENDS row and prize gauge (EXP+ / NEXT LV / Moogle
  points) drawn at alpha 0 (`0x140248590`) so their logic keeps running (the battle end waits on
  the prize gauge; Re:CoM's low-HP sound plays); enemy gauges and the minimap (room plate assumed
  with it) hidden through their own hide paths: five mask reads (bits 0x10, 8) via our getter while
  the KH1 HUD is up. Kept: level-up choice, Guard / Recover / Resist / LEVEL UP! popups, the
  command menu. Not seen in game.

---

## Stage 57: KH1 HUD H2, Sora's HP/MP cluster, version 0.26.0 (2026-10-31)

- At `hud = kh1`: Sora's cluster bottom-right at KH1's places and margin (+86): back / front /
  face `C2DSeqCtrl`s on `kh1hud:00`; HP on raw Re:CoM HP at 5° per HP to 120 HP then segments,
  MP, charge; KH1's MP/charge fades, hurt face (40/50/60 frames), red danger flash, size-class
  offset. Updates on events only: wrappers on Re:CoM's HP setters `0x14020D6A0` / `0x14020D8E0`
  (first bytes checked) and the MP hooks. The 7.2 bar isn't made at `hud = kh1`.
- Not built: hit shake / size-class slide, arc end strips and 0.7° soft edge, KH1's 10-frame HUD
  fade, MP fade resume (owner questions / gaps in `recom-kh1-hud.md`). Not seen in game.

---

## Stage 56: KH1 HUD H1, the kh1hud sprite set, version 0.25.0 (2026-10-31)

> Decision 39: an option where Re:CoM's HUD is replaced by KH1's, KH1's exact graphics and places

- KH1's HUD is data (Checked, files + decomp): 24 `exchange/<L>_uibin_*.bin` layouts ("TTUI" v2,
  all 120 + com_battle + danger decoded from the owner's dump), `command2/uitex.bin` pictures,
  portraits in `.mdls` section 2. The asset step builds `kh1hud.lad` into `FO0002.SPR` and
  `kh1hud.imz` into `FO0002.RTM` (PS2 1024×512, HD 2048×1024, FR/GR/IT/SP/UK copies), KH1's arcs
  baked by running KH1's fan code (`0x14026B790` / `0x14026E450`) over its ring textures.
- Re:CoM registers `kh1hud:00` (632 groups) as its own; `kh1-hud.json`; interim switch
  `hud = recom | kh1` in the ini (default recom) until the KH1 camp entry. `docs/recom-kh1-hud.md`.

---

## Stage 55: 2D draw lists: no leak from Config flips; pool counter, version 0.24.4 (2026-10-31)

- Re:CoM's 2D pool: 2047 usable entries (`0x140245293`); each C2DSeqCtrl layer (sprite group or
  attached C2DFnt) takes one, only the ctrl frees it; a release looks only in the current draw list
  (`0x1402499C0` → `[0x14087CE70]`). Checked (decomp + exe).
- Leak found: flipping Combat in the camp's Config built / deleted our bar and menu rows on the
  camp's list 1, so their releases failed (~13 entries per flip). Now each gauge records its list
  at init and our adds / clears / deletes run on it (Re:CoM's setter `0x140249800`, switched back
  at once). Normal play already balanced to zero. Log: `2d: …` lines (ours / Re:CoM's free
  count); the battle-end log ends with the pool state. Not seen in game.

---

## Stage 54: Battle end: KH1 yields to Re:CoM's win; battle-end log, version 0.24.3 (2026-10-31)

> "after finishing a battle in the field sora is locked in place and doesnt return to the field"

- Re:CoM's battle end (Checked, decomp + disasm; `recom-m6-battles.md` § Battle end): last enemy
  → `player+0x428 |= 4` (`0x1401F4A60`) → win action 0x1C (`0x1401F1B20`) → manager
  `0x1401871D0` states 1 (children, enemies) → 2/3 (prize gauge `+0x94`) → 4 (win pose done
  `0x1401F3C90`) → 5/6 (roulette, task 0x22) → 7 (flag (1, 2), room script returns Sora).
- Owner's trace 13: win pose finished (0x1C sub 4), no enemies: the manager waits elsewhere; not
  identified. None of our hooks writes the manager, prize gauge, pause flag, roulette or flag.
- Fixed a real deviation: a KH1 cast or swing ending after the win overwrote Re:CoM's win action
  for a frame (0x1401F1B20 ran twice); `Kh1Sora::yield_to_recom` now stops our frame then.
- New `battle end:` log lines and an `end …` trace token name what Re:CoM waits for (manager
  state, children by class, prize gauge states and animation ids, pause flag, roulette, flag
  (1, 2)). 9 checked-only sites. Not seen in game.

---

## Stage 53: MP bar under the FRIENDS row, version 0.24.2 (2026-10-31)

> Owner's video (2026-10-31): the MP bar sits over Re:CoM's FRIENDS row (label and friend cards
> under the HP bar), and after Combat Re:CoM → KH1 the rebuilt bar appeared far off.

- The bar (and its fade pieces) now sits 40 units lower, under the friend cards (measured on the
  video: cards' bottom ≈ 84 units, the bar's top was 50). Assumption until a screenshot.
- The off-place rebuild is what 0.24.1's rest transform addresses (not yet seen: the game was
  running 0.24.0 in the video). Not seen in game.

---

## Stage 52: Command menu installs; MP bar placed at rest, version 0.24.1 (2026-10-31)

> "The command menu isnt there. the MP bar clips through the \"friends\" bar, and also, when switching
> from KH1 to Recom, and then back to KH1, the MP bar is randomly placed somewhere else each time"

- Cause 1 (Checked, owner's log): the command menu's install checked `menu_groups()`, which is
  None until a bar is built on the first player gauge (later), so it logged "no plate groups" and
  never installed. Now it checks the manifest's `extra_groups` at install; the groups are still
  resolved when the bar is built.
- Cause 2 (Assumption, likely): a rebuilt bar's new `C2DSeqCtrl` kept whatever transform the
  reused memory held. Every control we make now gets position (0, 0, 0, 1) and scale 1 at once.
- The overlap with Re:CoM's friend gauge is not fixed yet: screenshot requested. Not seen in game.

---

## Stage 51: M7 stage 7.10, Aero, version 0.24.0 (2026-10-31)

- Aero / Aerora / Aeroga (Checked, KH1 script + decomp): cast at 30 or at once when cut; 2 MP;
  barrier (18 + max MP) × 60 frames halving Sora's damage (÷ 2, ≥ 1) in the existing damage-check
  wrapper; re-cast restarts it. Aerora/Aeroga's wind: Re:CoM's Aero shot as a real attack 0.8 m
  around Sora, collided by Re:CoM, KH1 damage 8 + 2 × max MP, a new hit serial every 30 frames
  (`0x14016F7D0`). Plain Aero: the shot without collision as the aura. Not seen in game.

---

## Stage 50: M7 stage 7.9, Stop, version 0.23.0 (2026-10-31)

- Stop / Stopra / Stopga (Checked, KH1 script + decomp): at 28, every enemy in a 2.5/3.5/5 m,
  ±3 m cylinder, nearest first, one per 4 frames; (2 + max MP) × 60 × KH1 resist[9] frames, 0 =
  immune (Re:CoM-only bosses); 2 MP. Re:CoM's own stop state (kind 8, start `0x14017C740`)
  already holds up to 8 hits and lands them after (Checked, decomp); its gate `+0x2C` is set from
  KH1's resistance per strike; stopped enemies' attacks off with `0x1401A2D90`. Every boss class
  has the stop state; non-enemy parts can't be stopped. Not seen in game.

---

## Stage 49: M7 stage 7.8, Gravity, version 0.22.0 (2026-10-31)

- Gravity / Gravira / Graviga (Checked, KH1 script + decomp): sphere 2.4 m above the target (no
  target: 2 m ahead) falls 2.3 m over 60/70/60 frames growing to 1.0/1.5/1.6; impact at 88/98/88
  hits every enemy within 1.7 × scale m once: current HP × (power + 5 × max MP)% × KH1 resist[8],
  at least the floor (Re:CoM-only bosses 2 %); 1 MP.
- Re:CoM's Gravity shot (collision held) + Re:CoM's hit function at impact, as a flinch carrying
  KH1's number (Re:CoM's Gravity kind can't carry it and flattens for 3 s); Re:CoM's gate `+0x40`
  isn't consulted at KH1. The slow under the sphere isn't built (owner question). Not seen in game.

---

## Stage 48: M7 stage 7.7, Cure, version 0.21.0 (2026-10-31)

- Cure / Cura / Curaga: heal 36 frames after the command, locked for 68, 1 MP paid at the heal;
  cut or not healable = no heal, no MP (Checked, KH1 script + motion). Heal = Re:CoM's own Cure
  case `0x140222946` (effect + heal `0x1401E59A0`) for Re:CoM's 50 / 150 / 300 HP (decision 34);
  Re:CoM's charge effect from `0x14022070A`. Not seen in game.

---

## Stage 47: Fire release root curve as KH1, version 0.20.1 (2026-10-31)

- KH1's Fire release motion (Checked, files): back 1.5 cm a frame to −15 cm at frame 10, then
  0.1 cm a frame forward to −10 cm at 60 (air −42.41 cm at 10, held). The motion record now holds
  that curve (it held −15 cm). In game Sora's travel still comes from Re:CoM's clip `p001_063`
  (−40.5 cm at frame 10): owner question. Not seen in game.

---

## Stage 46: M7 stage 7.6, Thunder, version 0.20.0 (2026-10-31)

- Thunder / Thundara / Thundaga (Checked, decomp + files): release at 30, locked for 68 frames,
  1 MP; every enemy (up to 30) nearest first from the target (no target: Sora; only the marker
  sits 2 m ahead along the camera) whose hurt body overlaps a 6 m radius, ±5 m cylinder; first
  strike at release, then one per 8 frames, each enemy once.
- Each bolt: Re:CoM's own Thunder spawn made at that enemy (effects + bolt shot, its collision
  off), then Re:CoM's own hit function on the enemy's first hurt body (the call its collision pass
  makes after a contact; vtable slots `0x140523E30` / `0x140523E60` compared per bolt). Damage
  KH1 power + max MP × 2, thunder resistance. KH1's 50 % world dim not built (owner question).
  Not seen in game.

---

## Stage 45: M7 stage 7.5, Blizzard, version 0.19.0 (2026-10-31)

- Blizzard / Blizzara / Blizzaga from the command menu (Checked, decomp + files): release at 28
  only while the cast motion plays (else no spell, no MP); motion 86 free at 68 (air 78 / 68);
  cost 30 units; 7 shards (6 on a 22.5° cone + 1 straight, 19.15 cm off axis), 20/30/40 cm a frame,
  26 frames, radius 70/90/120, a hit or wall ends a shard; glide 50 cm over Re:CoM's floor test.
- One hit per enemy per cast: KH1's shared hit bit (Checked, decomp) = Re:CoM's shared hit serial
  (`0x140156770`), so Re:CoM's own hit handler does it. Built from Re:CoM's Blizzard spawn
  (`0x1402224A7`, `0x1402224E9`) and rows `0x1407C6EA0` / `0x1407C6F20`, Re:CoM's effects.
  Damage: KH1 power + max MP, KH1 ice resistance only. Not seen in game.

---

## Stage 44: Fire re-targets as KH1, version 0.18.1 (2026-10-31)

> Decision 36: Fire re-targets as KH1

- KH1's Fire script re-picks only for a ball cast with no target (a dead target: the ball flies
  straight; Checked, decomp + files). The pick `0x1402B0E90`: enemies within 5 m of the ball,
  score distance × (1 + 2 × (1 − cos angle to the ball's heading)), lowest wins, every frame.
- Our target-less balls get their own per-frame step: a wrapper writes KH1's pick into Re:CoM's
  homing target, then runs Re:CoM's homing step (`0x1401575D0`, via the thunk `0x1401560B8`;
  CHomingBullet vtable +0xA8 `0x1405255D8` compared at install). Candidates: Re:CoM's lock-on list
  filtered as its nearest pick (`0x1401CDBCF`) does. Re:CoM's shots never run it. Not seen in game.

---

## Stage 43: M7 stage 7.4, the cardless cast and Fire, version 0.18.0 (2026-10-31)

> Decision 34 (M7 "FULLY implemented"; Re:CoM's spell visuals and sounds)

- Choosing Fire / Fira / Firaga in the command menu casts it through Re:CoM's own magic action
  0x13, its cast clips and its homing-shot runtime, with Re:CoM's Fire effects. One entry point
  `win::magic::issue(spell, level)` for the menu and (7.12) shortcuts.
- KH1's timing (Checked, decomp + files): 28-frame wind-up, release motion 60 (air 52) free from
  46, ball in frame 28; fast path (Fire's release motion from frame 14, swing 1 / lunge windows):
  no wind-up, ball 8 frames after the command; cut before release = no ball, no MP; air hop 30 cm,
  air speed halved. Ball 40 cm/frame × 30 frames, 3°/frame homing, radius 50, one hit.
- MP: can_pay before (not on the fast path, as KH1), `pay(30)` at release. Damage: spell power +
  max MP through KH1's curve, twin/proxy Defence, floor, cap and KH1 fire resistance (Re:CoM's weak
  part and boss falloff on top).
- Re:CoM's init isn't called (it reads the played card's table, not loaded in the field); the cast
  does its writes cardlessly and points the row at KH1 rows built on Re:CoM's Fire rows
  (`0x1407C6E90`, `0x1407C6F00`). Hook: magic exe `0x140217EE0` (`48 89 5C 24 08 48 89 74 24 10`).
  The Field exe's jump table gets an action-0x13 case (`0x1401FA63C` → a 13-byte stub in the int3
  cave `0x1401FB0E1`). `docs/recom-m7-magic.md`. Not seen in game.

---

## Stage 42: HD atlas scale for the bar and the menu plates, version 0.17.1 (2026-10-31)

- The owner's `FO0002.SPR` language copies are HD layouts with every UV × 2 (Checked, files), so
  0.17.0's asset step found no text plate there and wrote the bar's sprites at PS2 UVs. The step
  now scales the bar's and the plates' UVs by each set's atlas scale and finds the plate in every
  copy. Rebuilt and installed on the owner's PC. Not seen in game.

---

## Stage 41: M7 stage 7.3, KH1's command menu, version 0.17.0 (2026-10-31)

> Decision 34: command menu from Re:CoM's pieces, laid out like KH1's, field and battle, plate
> colour cursor, KH1's own words

- Attack / Magic / Items / context row and the Magic list at Combat = KH1 in Sora's story, field
  and battle: each row a Re:CoM `C2DSeqCtrl` playing a Re:CoM text plate (2 groups the asset step
  adds to `Cockpit:00` after the MP bar's: plate, and plate with the magic plate's blue strip as
  the highlight) with a `C2DFnt` label holding KH1's word (`kh1-command-menu.json`, decoded from
  `btltbl.bin`'s `*_Word.bin`, 6 languages; ini `menu_language`, default `us`).
- KH1's rules (Checked, decomp): layout (16, 260), rows 22, Magic list at x + 11; fresh d-pad only;
  Right/confirm opens, Left/cancel back; brightness ±32 a frame, 10-frame fade, 14-frame context
  tab; Counterattack shown as "Counter"; greying by `Mp::can_pay` and the 1100 range check.
- Confirm on Attack is today's Attack path. Spells buzz until 7.4 (cast). Items greyed (M8).
  Field context row from the object Re:CoM's Triangle uses (`0x1401E80C0`, read only): Open /
  Talk / Lift / Jump On. `kh1_all_spells = on` gives all 7 spells at level 3 until 7.11.
- No new hook or patch; runs from the MP bar's gauge wrappers and the request-bits wrapper.
  Checked sites: `0x140248590`, `0x1402476F0` (vtable `0x1406073A0`), `0x1401E80E8`, `0x1401B8A10`.
  `docs/recom-m7-command-menu.md`. Not seen in game.

---

## Stage 40: MP bar in every language copy, version 0.16.1 (2026-10-31)

- The asset step on the owner's files: `FO0002.SPR`'s language copies of `Cockpit.lad` have 157
  groups (the base file 151). Checked (files). Each copy now gets the bar after its own groups;
  `kh1-mp-bar.json` lists the first groups (151, 157) and the DLL picks by the loaded set's count.
- Built on the owner's PC; `raw/FORM/0002/FO0002.SPR` and `kh1-mp-bar.json` added to the mod and
  `mod.yml`. Not seen in game.

---

## Stage 39: M7 stage 7.2, the MP bar, version 0.16.0 (2026-10-31)

> Decision 34: "Re:CoM-style bar" under the HP bar, "Grows with max MP", colour "From KH1's files"

- A straight bar under Re:CoM's HP bar (where Riku's Dark gauge sits), field and battle, at
  Combat = KH1 in Sora's story: one 24-unit slot per max-MP point, 26 apart (KH1's straight
  segments), blue up to MP, KH1's orange charge line on top; KH1's fades (10 frames, ring part
  only). Checked (KH1 decomp: pitch 26, 30 units per slot, caps, fade rule).
- Drawn by Re:CoM: a `C2DSeqCtrl` built as the Dark gauge is, playing 8 animation groups the
  asset step adds to `Cockpit:00` in `FO0002.SPR` (and its language copies), coloured by vertex
  colours from the player's KH1 `command2/uitex.bin` (picture 5 / 7) over Re:CoM's 128-grey texel
  (`FO0002.RTM`). Hooks: CPlayerGauge init `0x140208360` (`48 8B C4 56 57 41 56 48 83 EC 70`), the
  second-bar state `0x14020C190` (`40 55 57 48 83 EC 48`), the deleting dtor `0x140207770`
  (`40 57 48 83 EC 30`). Updates only on MP events. Re:CoM setting / Riku's story: nothing made.
- Asset step: `raw/FORM/0002/FO0002.SPR` + `kh1-mp-bar.json`. Assumptions: texel × vertex / 128
  on PC, vertex order, position (screenshot needed). `docs/recom-m7-mp-bar.md`. Not seen in game.

---

## Stage 38: MP refills where HP refills; KH2 Keyblades' MP, version 0.15.2 (2026-10-31)

> Owner's answers (decision 35): MP refill "Where HP refills"; KH2 Keyblades' MP "Use KH2 Magic"

- Re:CoM's three `HP = max HP` writers (Checked, decomp + exe): the state step `0x1401AC9D0`
  (wrapper), the checkpoint heal in `0x140237CE0` (its call `0x1402374C0(0)` at `0x140237F3A`
  retargeted through a jump placed near it; context bytes checked; no branch into it), the level-up
  HP choice in `0x14010EF10` (wrapper). After each, at KH1, MP goes to full (`restore_full`).
- The six KH2 Keyblades' `magic` (KH2 Magic) is now their max-MP bonus.
- Assumption: no other full heal; what each moment is in play isn't traced. Not seen in game.
- `docs/recom-m7-mp.md` § Refills and the KH2 Keyblades.

---

## Stage 37: KH1 numbers for enemies without a twin, version 0.15.1 (2026-10-31)

> "it feels like any enemy that doesnt have a KH1 counterpart barely takes any damage in
> comparison to the KH1 enemies, why is this and fix it"

- Cause (Checked, owner's F9 traces): twins took KH1 damage against KH1 HP (max-level Sora: 300
  = the cap, one hit, which is KH1's own result against first-visit records); every other enemy,
  every boss included, kept Re:CoM's floor HP and took Re:CoM's cardless AP = Sora's attack stat
  `player+0x43c` = 10 per hit.
- Re:CoM's enemy HP is the `@ENP` row of `ENEMY/<id>/EN<id>.EPD` picked by floor (the i16 at
  `0x14087B844`, getter `0x1402337D0` `0f b7 05 6d 80 64 00 c3`). Checked (decomp + exe + files).
- Asset step (`kh1-stats.json` format 5): Re:CoM HP rows of the twins and boss twins from
  `Recom.pkg`, 11 KH1 boss records (Guard Armor 700, Tutorial Leon 120, Genie Jafar 750, Cloud 300,
  Trickmaster 600, Parasite Cage 450, Oogie 450, giant Ursula 900, Hook 900, Dragon Maleficent 1200,
  Darkside 300) and per world × floor ratios (KH1 HP ÷ Re:CoM HP, lower-median twin).
- Runtime (no new hook; the stats-init wrapper `0x14017FF50` and max-HP getter `0x14017F5A0`):
  boss twins take their KH1 record; other enemies get a proxy: Re:CoM HP × the cell's ratio, the
  median twin's Defence / level / floor / cap, 100 % resistances; Re:CoM-only bosses the world's
  boss ratio and decision 34's Gravity 2 % / Stop immune. MP charge reads the proxy's level.
- The owner's `kh1-stats.json` was rebuilt on their PC with this build's asset step. Not seen in
  game.

---

## Stage 36: M7 stage 7.1, KH1 MP, version 0.15.0 (2026-10-31)

> "lets work on M7. Make sure all of that is FULLY implemented, also including a shortcut menu,
> customize menu, MP bar" (decision 34; plan `docs/recom-m7-plan.md`)

- KH1's MP at Combat = KH1 in Sora's story: max MP 3 + 1 at Sora's Re:CoM levels 12/24/36/48/60
  (btltbl `0x3AC0`) + the equipped KH1 Keyblade's MP bonus (weapon `+0x38`); MP points and the
  charge with KH1's exact rules: refill `0x1402A4CB0` (point k needs 30·k), add `0x1402A4A00` /
  `0x1402A4AE0`, can-pay `0x1402A4C10`, pay `0x1402A5B60`, charge per hit `0x1402BF330` (attack
  `+0x2C` × (1 + 0.2·Δlevel), 1..250 %), level-up `0x1402B51B0`. Checked (decomp + exe + files).
- No new hook: the refill step runs in the existing request-bits wrapper (one flag check per frame
  unless a point is waiting), the hit charge in the existing enemy-hit wrapper.
- Saved per save in the spare range `save+0x2675..=0x2684` (versioned; room for 7.11's spell levels
  and 7.12's shortcuts); all-zero = full MP. `docs/recom-m7-mp.md`.
- No HUD yet (7.2); log `mp:` lines and the F9 trace's `mp` token. Damage-taken charge waits for
  M9 (KH1 abilities). Not seen in game.

---

## Stage 35: no card duel at KH1, version 0.14.1 (2026-10-31)

> Owner: "c" (Marluxia 3's card duel: at KH1, the attack that starts it becomes a plain hit, so
> the duel never starts; decision 33)

- Re:CoM's damage check `0x1401E6DE0` reads the pending hit's reaction kind `player+0x710` once
  (after the hit-pending test `+0x720`); kind `0x21` calls the card-duel init `0x140220C40`
  (action 0x24). Kind 1 is the plain flinch. Damage depends only on kinds 8 and 0x22. Checked
  (decomp).
- Change: in the existing damage-check wrapper, at Combat = KH1 for the main Sora, a pending
  kind `0x21` becomes 1 before Re:CoM's check runs. One read per frame. No new hook.
- Marluxia 3 enters its duel mode (`+0xCE4 |= 4`, `0x1403F3590`) only while Sora's action is 0x24;
  otherwise its state 3 plays its own no-duel branch (motion 0x34). Checked (decomp).
- The hit also arms KH1's post-hit invulnerability (Stage 30). Not seen in game.

---

## Stage 34: M6 stage 3, bosses, version 0.14.0 (2026-10-31)

> "Lets move onto the next thing, go ahead" (M6 stage 3 per decisions 29, 30: bosses react to
> KH1 swings, human bosses keep a hidden move list, Marluxia 3's stock folder hidden)

- **P5:** Re:CoM's "Sora's action" getter `0x14017F720` is rewritten in place (32 bytes, Re:CoM
  `48 8B 05 59 BC 6F 00 48 85 C0 74 07 …`) so action 0xA (KH1 swings, air swings, Counterattack)
  answers 0xB, Re:CoM's card attack. All 15 callers checked (12 compare with 0xB, 3 with 0xC /
  0x22 in Riku's story; none with 0xA); nothing branches into the range. Switched per battle on
  its own (only a KH1 battle in Sora's story). Checked (exe + decomp).
- **Human bosses:** their revolver, CBTL and message 2 run as Re:CoM's; their card UI is hidden
  by Re:CoM's event-hide mask: side-2 cards (0x40 in stage 2's card mask), the enemy cue
  (`0x14015E3F3`, `0x140161A64`), the stock row (`0x140169CE3`, `0x140169D07`, `0x140169D48`,
  `0x14016E2B1`). 16 mask reads switch together. Hiding is visual only. Checked (decomp).
- **Marluxia 3's stock folder** hidden through its own mask reads (`0x1401690A6`,
  `0x1401690DD`, `0x140169103`). The hidden folder parks in state 5; state 6 only plays its
  cards in the card duel. Checked (decomp).
- No boss waits for a Sora card or a card break, except Marluxia 3's card duel, which Sora
  always loses at KH1 (only the Reload card): owner question. Readers of Sora's deck re-checked
  against the empty revolver. 203 site byte lists re-read, 0 mismatches.
- Assumption: bosses react to KH1 swings more often than to Re:CoM's attack cards.
- `crates/kh1-recom/src/boss.rs`, `docs/recom-m6-battles.md` § Stage 3 (per-boss checklist). Not
  seen in game.

---

## Stage 33: Enemy pace in the Config menu, version 0.13.4 (2026-10-31)

> "also do A" (Guard reactions and Enemy pace as Re:CoM's one-value "Game Mode" style rows, so
> both fit in the Config menu)

- Re:CoM's Config items: 21 slots at `camp+0xB38`, Re:CoM uses 0..13. Combat keeps two values
  (14..16); Guard reactions (17, 18) and the new Enemy pace (19, 20) are label + one wide value
  item built like Re:CoM's row 4: value sequence 8 of `0x1407C1240`, x element `0x16`, selected
  0, cursor animation `0x100`. Checked (decomp `0x1400DF1B0`).
- Left / right on a single row flip the setting and redraw (the value shows the other option).
  Enemy pace is written to `kh1-recom.ini` and read at the next battle start.
- Help: "Re:CoM: a crowd shares attacks. KH1: each attacks as if alone." (62 characters).
- Tests: slots 14..20 all used, styles, row 7 toggles, panel grows 3 rows. Assumption: the
  taller panel still clears the camp's bottom bars. Not seen in game.

---

## Stage 32: Config help lines fit the help box, version 0.13.3 (2026-10-31)

> Owner's Config screenshots: the help line is cut at the box's right edge, so the Combat
> row's "(mod v…)" never shows ("how can i check mod version in game?").

- Checked (screenshot): the help box shows about 69 characters of one line; Combat's help showed
  up to "combat." (68), Guard reactions' up to "KH1 com" (69).
- Change: shorter help lines (63 characters each): "Re:CoM: card battles. KH1: Kingdom Hearts
  combat. (mod v0.13.3)" and "On: blocked attackers flinch, blocked shots fly back. KH1 only."
  `menu::HELP_MAX` 90 → 66 (test). Text only; no address changes.

---

## Stage 31: Enemy pace setting, version 0.13.2 (2026-10-31)

> "is there a way to make enemies less aggressive? is there a system within KH1 that makes it so
> that enemies dont attack as frequently?" … "lets try b for now" (decision 32)

- Finding: KH1 has no shared attack limiter as far as checked (Shadow's script, the 26 common
  syscalls: Checked (decomp + files); the other syscalls: Assumption). The extra aggression is
  Stage 28's P4: Re:CoM's think rate `0x14017F380` is no longer multiplied by the enemy count,
  but each Re:CoM enemy's solo rate was tuned to be shared. Checked (decomp).
- Change: `enemy_pace = recom | kh1` in `kh1-recom.ini`, default `recom`. At each battle start
  (manager ctor wrapper `0x1401859B0`) P2 and P3 follow the battle as before and P4
  (`0x14017F3DE`, `83 F9 64 7D 26` <-> `83 F9 64 EB 26`) is ours only at `kh1`
  (`battle::wanted`, `plan_each`). Enemies can still attack while others do (no card slot, P2,
  P3).
- Not in the Config menu: Re:CoM's item array has one free slot left (`recom-config-menu.md`).
  Owner asked.
- Not 1:1 at `recom` (it is Re:CoM's pacing); `kh1` is decision 30. Not seen in game.
- The code went in with the previous commit; this one adds the version and notes.

---

## Stage 30: KH1's post-hit invulnerability, version 0.13.1 (2026-10-31)

> "Lets also carry over sora's KH1 mechanic where if he gets hit he has a period of time where
> he cant be hit again to prevent getting combo locked" (decision 31)

- KH1: every damage reaction sets flag `0x80` with 30 frames (`0x140297F30`), counted down by
  `0x140297FE0` from the reaction's release at 1 per 60 fps frame. Checked (decomp).
- Re:CoM's own hit-immunity timer `player+0x7BC`: the damage check `0x1401E6DE0` skips hits while
  it is above 0; the Battle / Field exes subtract dt. Checked (decomp).
- Change: in the existing damage-check wrapper, at Combat = KH1 for the main Sora, a hit
  reaction (action `0xC`) that starts holds `+0x7BC` at `1500 + dt` (30 frames) each of its
  frames; Re:CoM counts the 30 frames down itself after it. No new hook; one read per frame.
- Checked (test): 30 immune frames after the reaction at 60 and 30 fps. Assumption: leaving `0xC`
  = KH1's lock release; hit order within a frame (±1 frame). Not seen in game.
- `docs/recom-kh1-attack.md` § Post-hit invulnerability.

---

## Stage 29: M6 stage 2, no deck for Sora in battle, version 0.13.0 (2026-10-31)

> "The deck shouldnt be a mechanic when set to KH1. sora can still collect cards just in case
> they want to change from KH1 combat to ReCoM combat, but in battle there shouldnt be a deck to
> draw cards from when KH1 is selected" (decision 29)

- **Empty revolver** (hook `0x1401F43D0`, `40 57 41 56 41 57 48 83 EC 40`): at KH1 in Sora's
  story, outside minigame rooms, the battle revolver is built by Re:CoM's own constructor
  `0x140164D40` from Re:CoM's end-of-deck entry, which is what Re:CoM builds for a saved deck of
  0 cards (0 cards + Reload). It also replaces the tutorial deck. Checked (decomp + exe).
- No draw, Reload, stock or sleights follow from it with no more code (no card to play; Reload
  only from Re:CoM's battle branch, unused by KH1 Sora since Stage 9). Checked (decomp).
- **No Friend-card drops** (hook `0x14017E190`, `40 53 48 83 EC 20 48 8B D9`): the roll returns
  as Re:CoM's failed roll does. Its cards are all type 5, which only ever go into the battle
  revolver; the collection is untouched. Checked (decomp + exe tables).
- **Deck HUD hidden by Re:CoM's own event-hide mask** (variable (1, 0xE2)): seven reads
  (`0x140168A34/A93/B32/B90`, `0x14015037C`, `0x140151625`, `0x1401545BD`) call a stub that adds
  the cockpit bit 2 for this battle's empty revolver, or 0x20 for Sora's side-1 cards. Re:CoM's
  bytes go back at each battle start at the Re:CoM setting. No branch or pointer lands inside the
  patched bytes (exe scan); 163 site byte lists re-checked.
- Log-only readers `0x1401EA1A0`, `0x1401EA1E0`, `0x140167CF0`; the F9 trace gains `m6deck`.
- Assumption: the tutorial battles' scripts may wait for card flags (0x12, 0x14–0x16) that
  can't fire at KH1 (already true since Stage 9). No flags are faked; the log and trace show it.
- `crates/kh1-recom/src/{deck.rs, win/deck.rs}`, `docs/recom-m6-battles.md` § Stage 2. Not seen in
  game.

---

## Stage 28: M6 stage 1, Heartless attack without cards and several at once, version 0.12.0 (2026-10-31)

> "When KH1 is selected, enemies shouldnt play cards, and also, multiple enemies should be able
> to attack at the same time, exactly how it works in KH1" / crowd pace: "Each attacks as if
> alone (KH1)" (decisions 29, 30)

- Research `m6-cardless-battles.md`: Heartless attacks run through the card play `0x140198870`.
  Re:CoM serialises enemy attacks in four places: CBTL's single enemy slot, the think gate
  `0x14017D010`, one grant per frame in `0x140187600`, and think-rate scaling by enemy count
  `0x14017F380`. Checked (decomp + exe).
- **P1:** a wrapper on the card play, at KH1 in Sora's story. With HP above 0, CBTL not locked
  and an attack state 2..10, it does what message 2 does: set the flags, set the next state, no
  card and no CBTL slot. Re:CoM's state code runs the attack. When CBTL is locked (win pose,
  escape, death, duels), Re:CoM's card play refuses as usual. Marluxia 3 keeps Re:CoM's path
  while its own handler ignores message 2.
- **P2–P4:** byte patches written at each battle start (wrapper on the manager constructor
  `0x1401859B0`): the think gate always yes, every request granted, no crowd scaling. Re:CoM's
  bytes are written back at the Re:CoM setting.
- No branch or pointer lands inside the patched bytes (exe scan). 132 site byte lists re-checked.
- Not in this stage: human bosses (deck-picked attacks), the action-0xB check of 12 bosses
  (Tutorial Leon still never attacks at KH1), Sora's deck, Marluxia 3's sleight cards.
  `docs/recom-m6-battles.md`. Not seen in game.

---

## Stage 27: turned-back shots do damage, version 0.11.2 (2026-10-31)

> "it looks like when guarding a projectile, it sends it back but doesnt do any damage. make
> that do damage" (decision 28)

- Owner's 0.11.0 log (Checked): turned-back shots met their shooters for 0 damage. The shooter was
  in its card attack, and Re:CoM's hit handler skips hits then.
- Change: a turned-back shot's hit gets the card-interrupt bit `0x2000000`, as a KH1 swing does,
  so Re:CoM ends the card and applies the shot's damage. No new hook.
- Not seen in game. Next: M6 (decision 29).

---

## Stage 26: Two Become One from KH2, version 0.11.1 (2026-10-31)

> "star seeker and bond of flame are correct. two become one is the wrong keyblade, but the
> correct keyblade exists in KH2. the rest of the keyblades do not exist inside of KH2 or KH1,
> but for right now the placeholder models are fine"

- Finding: the exe's strings at `0x1405060F0` are internal names. The cards Re:CoM shows for
  kinds 0x14..0x17 are Total Eclipse, Midnight Roar, Maverick Flare and Two Become One.
- Card 0x17 now converts KH2's Two Become One (item 0x21F, stat 148, Strength +5, Magic +4,
  `W_EX010_P0`, reach x1.14) in group 42. Checked (files). 0x14..0x16 keep their KH2
  placeholders. Asset step only; the DLL is unchanged apart from its version.
- Not seen in game.

---

## Stage 25: enemy reactions to Guard, version 0.11.0 (2026-10-31)

> decision 27: "Fly back, hurt enemies" / "Flinch" / "Include them" / default "On"

- New Config row "Guard reactions" (row 6, Off / On, default On, `guard_reactions` in the ini).
  It only acts at Combat = KH1.
- Block detection in the pair-dispatch wrapper: result 2 on the player-side list
  (`*0x14087A838`), Sora's KH1 Guard live, his body `+0x9C & 0x20`, and `+0x98` = the attack's
  owner. Checked (decomp).
- **Melee:** after the attacker's hit handler, Re:CoM's reaction picker `0x14017D260(enemy, 1)`.
  If it picked the flinch (state 11), the card ends with `0x14017E1D0` and bit 0 is cleared.
  Super armour keeps Re:CoM's voice-only reaction and the card. Bosses included.
- **Shots:** Re:CoM's list setter `0x14014E5A0(coll, 0)` moves the shot to the enemies' list.
  Its velocity and aim angles are reversed, a homing shot is pointed at its shooter
  (`CBullet +0x258` with flag 0x800, `CHomingBullet +0x2F4`; the research note's `+0x32C` was
  wrong), and that test answers 0 so the bullet isn't ended. Re:CoM then moves it, collides it
  and applies its damage.
- New callees byte-checked: `0x14017D260`, `0x14017E1D0`, `0x14014E5A0`. Classes checked by RTTI
  at start.
- Assumptions: bullet models face their mover angles; the flinch knockback direction (it reads
  the last hitter); a returned shot does nothing to an enemy still in its card (Re:CoM's rule
  kept, to ask). Not seen in game.

---

## Stage 24: the six KH2 Keyblades, step 3 of the Keyblades, version 0.10.0 (2026-10-31)

> "Besides soul eater, every other keyblade should be from KH2 … pull the stats/models (not
> including keyblade abilities) from KH2" / "Measured reach, KK rest" / "perfect, everything is
> great, lets move on" (decision 25)

- New crate `kh2-formats-lite`, ported from KH2-Combat-in-KH1's `kh2-formats` (after OpenKH),
  credited in its `lib.rs`: KH2 archives (`kh1-formats` `egs` now opens `kh2_*`), `.mdlx`
  models and skeletons, `03system.bin`'s items and stats, `00objentry.bin`, `sys.bar` names.
- KH2 data (Checked, decomp + files):
  - Items: `03system.bin` `item` at `0x1087C`, 535 × 24 bytes; stat id at `+6`
    (`0x1403E3EF0`).
  - Stat rows after them, 16 bytes; `+4` / `+5` compared with a character's `+0x188` /
    `+0x18A` (`0x1402FCC80`, `0x1402FD050`). Calling them Strength / Magic follows OpenKH's
    names, and the Kingdom Key's +3 / +1.
  - Models via `went` → `00objentry.bin`: Star Seeker `W_EX010_30`, Monochrome `_60`,
    Follow the Wind `_70`, Hidden Dragon `_40`, Photon Debugger `_90`, Bond of Flame `_K0`.
- Converter: KH2's Kingdom Key mesh is Re:CoM's PL0002 within 0.077 mm on the same bones, and
  all six have PL0002's 12-bone skeleton. So the mapping is the identity (× 0.01, same axes), and
  the asset step checks it on every run. Groups 38..43 (card + 19).
- Numbers: KH2 Strength / Magic; reach = KH1 Kingdom Key's −3.7 × blade length ÷ KH2 Kingdom
  Key's 0.779 m (KH1's own Keyblades follow their models within 0.13 by the same rule, except
  Olympia); the rest the Kingdom Key's KH1 row.
- Runtime: the group rule `our_group_for_card` covers cards 2..0x11 and 0x13..0x18 (Soul Eater
  none); 110 table rows. No new hook.
- Assumptions: the facing flag mapping; list-A triangles on semi-transparent texels; Bond of
  Flame's charm blending; the HD raw route and trail length (carried over). Not seen in game.

---

## Stage 23: Guard and Counterattack in Halloween Town, version 0.9.1 (2026-10-31)

> "check the trace, trying to use guard in battle freezes sora"

- Trace 12 (Checked): Guard at frame 17, then the motion time stayed at 0 for 77 frames until
  a hit. The KH1 step ran normally (1.0). The log names world 6 (Halloween Town).
- Cause: in Halloween Town Sora uses resource group 11. `PL0011.ANN` lacked our appended clips
  `p001_225` / `p001_226`, so rows 0x47 / 0x48 resolved to no clip. Checked (files + decomp: the
  row's index from `0x1407C6300[player+0x398]`, `0x1401F7180`).
- Fix (asset step only): `PL0011.ANN` extended with the same clips at the same slots (same
  skeleton, Checked), and `PL0011.SES` with the same sounds. The DLL is unchanged apart from its
  version. `docs/kh1-recom-assets.md` § Halloween Town's pack.
- Also (owner's request, no code): `combo_master = on` in the owner's `kh1-recom.ini` for
  testing.
- Not seen in game.

---

## Stage 22: KH1 Keyblade models, step 2 of the Keyblades, version 0.9.0 (2026-10-31)

> "it all looks good continue" / swap timing: "When the menu closes" / memory: "Only the equipped
> one" / Halloween: "Equipped Keyblade everywhere" / first test: "Skip, build it all"
> (decision 25)

- Asset step (`docs/recom-keyblade-models.md`):
  - Re:CoM `.MDL` / `.RTM` readers and writers, byte-identical on Re:CoM's own files (PL0002,
    PL0011, PL0101 MDL; five RTMs). Checked (files).
  - KH1 `.wpn` reader (`kh1-formats/wpn.rs`), DDS decoding and a deterministic 256-colour
    quantiser for the TIM2 fallback.
  - The converter turns the HD `.cvbl` mesh of each of the 16 KH1 Keyblades (cards 2..0x11) into
    a Re:CoM MDL with PL0002's skeleton byte for byte (all vertices on joints 2..11) and an RTM
    whose HD `.dds` textures (`xw_ex_50x0_n_tm2.dds`, names unused by Re:CoM's 1,649 textures)
    go through Panacea's raw route.
  - New player resource groups 21..36 = card + 19, free in Re:CoM's table and archive. Checked
    (exe + files).
  - `kh1-stats.json` format 4 adds `group` and `textures`.
- Runtime (`win/models.rs`):
  - Re:CoM's PLAYER resource table is extended with the 16 groups (one checked pointer write
    at `0x1407EBF28`).
  - The weapon's model-init vtable slot `0x1405AADE0` picks the equipped group.
  - Wrappers on "queue the player's group" `0x140211AA0` and on the field camp open/close
    `0x1401A9590`. The weapon is rebuilt at camp close with Re:CoM's own constructor
    `0x140224BA0` and destructor, only when the equipped group changed.
  - Only the equipped group of ours is loaded. Re:CoM's group 2 stays loaded as Re:CoM loads
    it: 100 Acre Wood's Veggie Panic Keyblade builds from it, and the shared ANN / EET keep
    group 2's ids.
  - Halloween Town shows the equipped Keyblade at the KH1 setting.
- Assumptions: HD textures through Panacea's raw route; the group-2-first load order; the
  weapon rebuilt every room and battle; the trail length on longer Keyblades. Not seen in game.

---

## Stage 21: enemies stay hittable while they attack, version 0.8.2 (2026-10-31)

> "sora should be able to hit enemies even if they are in the middle of attacking themselves" /
> "Enemies stay hittable" / "Include bosses" (decision 26)

- Cause (Checked, decomp + disasm): every Re:CoM enemy runs its vtable `+0x438` (default
  `0x14017DA80` = `0x1401A2D70(enemy, -1)`, all hurt bodies off) when an attack state 2..10
  starts. `+0x440` (`0x14017DA90`) switches them back on at its end. Trace 11's missed swing had a
  still Shadow 0.65–0.86 m ahead for the whole hit window (Checked, trace). That the Shadow was
  mid-claw is an Assumption: the trace has no enemy state.
- KH1 (Checked, decomp + files): the Shadow's script never masks its hurt parts.
- Change: a wrapper on `0x14017DA80` (5 bytes `BA FF FF FF FF`; the tail jump checked as a
  callee) skips "bodies off" at the KH1 setting in Sora's story, for every enemy and attacker.
  It runs once per attack start.
- Also: KH1 hits survive the next swing's start; the trace's `hits` column is cleared at F9.
- Correction: Stage 9's "hits land during card plays" (decision 14) never fired in game, because
  the bodies were off. Not seen in game.

---

## Stage 20: the stance idle after Guard and Counterattack in the field, version 0.8.1 (2026-10-31)

> "when sora guards out of combat, the animation needs to smoothly go into the correct idle"
> (owner's video: the Guard ends in the battle crouch, then the field idle pops in)

- Re:CoM's idle choice `0x1401E6AC0`: the stance idle (row 0x2A, `p001_031`) in battle stance,
  or when the previous or current row is 0x24–0x25 or a swing row 0x30–0x33 (or the current row
  is 0x2A); else the field idle (row 1). Checked (decomp).
- Our KH1-only rows (Guard, Counterattack, appended after 0x47) end in the battle stance like
  Re:CoM's swings, but the rule didn't count them, so a field Guard went to the field idle.
  `rows::row_for` now counts them as swing rows. Test added.
- Not seen in game.

---

## Stage 19: Keyblades at the KH1 setting, step 1 (owning and equipping), version 0.8.0 (2026-10-31)

> "a. Besides soul eater, every other keyblade should be from KH2 … incorporated NATIVELY in this
> camp menu along with the KH1 keyblades" / unlock: "Ever earned" / Soul Eater: "Not for Sora" /
> Riku's story: "Hide" / menu: "New camp entry" (decision 25)

- Asset step: `kh1-stats.json` format 3 adds `keyblades` (the 17 KH1 Keyblades with Re:CoM
  cards, by Re:CoM card kind 1..0x11, from KH1's weapon table: radius, reach, Strength, crit,
  priority, magic) and `sora_base_strength` (without a Keyblade). Checked (files: Kingdom Key and
  Ultima Weapon rows; names matched by KH1's item order and stats, Assumption for the names).
- Runtime: the equipped Keyblade's KH1 numbers drive the blade test, the damage and Strength.
  It is one byte at `save + 0x2674` (`*(u64*)0x14087B110`), a range of Re:CoM's save block no
  code uses, which Re:CoM saves, checksums and zeroes itself (`recom-cards-save.md`). 0 = the
  Kingdom Key. Checked (decomp + exe scan); the log checks the range on real saves.
- Camp menu: an 8th entry "Keyblades" (KH1 setting, Sora's story, below Settings), through
  wrappers on the camp update `0x1400D9C80`, the entry build `0x1400E3DE0`, the cursor set
  `0x1400DEA10` and the teardown `0x1400D7C20` (all-or-nothing). Its page runs in Re:CoM's Config
  states 0x4A..0x4C, built from Re:CoM's classes after the Status page `0x1400EB7E0`. Unlocked =
  Re:CoM's "ever obtained" state `0x14015B3F0` (normal or premium); names from Re:CoM's card
  names (`0x140164CA0`). Details and addresses: `docs/recom-keyblade-equip.md`.
- Owner's screenshot: the 8th entry has room at Re:CoM's spacing (y ≈ 507 of 720).
- Assumptions: object ownership on delete; the equipped mark (Re:CoM's row marker 0x1D); the
  header title; the spare range on real saves. Not seen in game.

---

## Stage 18: KH1's lunge distance uses the enemy's body radius, version 0.7.5 (2026-10-31)

> "a then b then c" (owner: (a) the lunge distance, (b) KH1 hurtboxes on twins, (c) the
> equipped Keyblade's size)

- Finding (`hitbox-compare.md`): KH1's ground picker (`0x1402A7320`) subtracts each kind-0 lock
  point's radius (`0x1402B04C0`: point `+0xC` × actor `+0x40`) before its 180 / 300 thresholds.
  The mod used 0, so at 3.00–3.30 m from a Shadow it swung in place and missed where KH1 lunges.
  Checked (decomp).
- Asset step: `kh1-stats.json` format 2 adds each twin's lock radius from its `.mdls` part 4
  (`CLS\x03` lock points, kind = byte 0 & 0xF). Checked (files): all 31 twins have one;
  Shadow 30, Soldier 35, Large Body 100. The Screwdiver has four (170, 40, 40, 40); the first is
  used (Assumption).
- Runtime: Re:CoM's attack target becomes KH1's part with that radius (`win/attack.rs`
  `attack_target`). Data only; no new hook.
- Assumption: the twin's actor scale is 1.0. Not seen in game.

---

## Stage 17: the Keyblade appears on Guard, version 0.7.4 (2026-10-31)

> "when sora doesnt have his keyblade out and does guard, the keyblade should appear"

- The Guard start calls Re:CoM's own "draw the Keyblade", `0x1401F5D30(Sora, 1)` (Sora's
  vtable `+0xF8`, as the swing init `0x1401F0F70` calls it). It plays `kb_appear` only when the
  blade was put away. Re:CoM's own rules put it away again. New byte-checked callee. Checked
  (exe + decomp).
- Owner's trace 10 (0.7.2): 2 of 4 Guards blocked. The 2 that didn't were hit on Guard frame 4
  (startup) and frame 55 (after the motion), outside KH1's live frames 10–36. Checked (trace +
  files). The window stays KH1's.
- Owner: Sora is at max level, so the 300-damage hits (KH1's cap, Strength 51) are right.
- Assumption: KH1's Sora always holds the Keyblade. Not built; to ask.
- Not seen in game.

---

## Stage 16: KH1 model writer, the flip test's step 1, version 0.7.3 (2026-10-30)

> "would it not be easier to just bring all the maps and unique enemies/bosses into KH1 as a mod
> for KH1?" / option (b): "b" (owner, twice: scope it, then run the decisive test)

- Notes: `docs/flip-feasibility.md` (the scope) and `docs/kh1-model-writer.md` (this stage).
- `kh1-formats`:
  - `mobj.rs`: whole `.mdls` files read and written (container, `MOBJ`, both models, PS2
    packets, weighted vertices, textures). Every size, count and offset is rebuilt.
  - `cvbl.rs`: the PC's HD meshes (`.cvbl`) read and written.
  - `egs.rs`: `decode_entry_full` (an entry with its HD assets), `build_plain_entry` (a plain
    entry for Panacea's `<mod_path>/kh1/raw/` route), `Archive::read_raw_entry`,
    `Install::read_full`; `KeyTable::decrypt_head` is public.
- `kh1-recom-assets`: `flip.rs` and the CLI `kh1-flip-test` (`roundtrip`, `install rebuild |
  big | remove`).
- Results: byte-exact on the owner's install, 500 of 500 `.mdls` and 941 of 941 `.cvbl`, and
  every rebuilt entry decodes back (run on the owner's PC through the device link). Checked
  (files). The Shadow's `rebuild` and `big` (×1.5 positions) entries are on the owner's PC.
- Facts found: the PC draws the `.cvbl` HD mesh, not the PS2 packets; HD textures are tagged by
  `.mdls` texture offsets; the block after `MOBJ` holds `_KN5` / `BONE` / `URSL` data; offsets
  past the end of a `.mdls` point at the end rounded up to 0x80. Checked (files + source).
- Assumptions: Panacea's raw route works for KH1 models; the 12 extra inflated bytes of five
  Halloween Town `-1c2.dds` are never read.
- No change to the DLL or to Re:CoM. Not seen in game: `testers/kh1-flip-model/thread.md`.
- Windows type-check not run locally (the Windows target can't be downloaded in this session);
  the DLL doesn't use the changed crates, and the build bot builds and tests the workspace.

---

## Stage 15: sounds for Guard, its success and the Counterattack, version 0.7.2 (2026-10-30)

> "guard and counterattack work on soras end, lets try to get their sounds working go ahead" /
> sound source: "Re:CoM's sounds now"

- Asset step:
  - `PL0001.SES` is Re:CoM's sound file plus entries for `p001_225` (Guard: whoosh 1236 and
    swing grunts at 0) and `p001_226` (Counterattack: whoosh 1237 and finisher grunts at 0,
    steps at 46 and 80).
  - The KH1 frames are from `xa_ex_0010.mset`. SES frames are 60 Hz motion frames
    (`0x1401A47C0`, 50 ticks a frame).
  - Re:CoM's 50 entries are byte for byte. Checked (files + decomp).
- Runtime: at a guard success, Re:CoM's own guard spark and clink, `0x1401A6B20(Sora,
  "always", 0xB, hit point, 0, 0.0f)` (as `0x140182260` does for enemies). The hit point is
  `player+0x6C0`, kept by the damage-check wrapper. New byte-checked callee `0x1401A6B20`; the
  name pointer `0x1407C2C40` is checked against `"always"` at run time. Checked (exe + decomp).
- Assumptions:
  - The Guard whoosh and voice are the nearest Re:CoM sounds (KH1's weapon + 0xD and clip 24
    have no known Re:CoM match).
  - The id of `always` #0xB's clink is unknown.
  - Re:CoM fades motion sounds on the next motion; KH1 appears not to.
- Also: the replay tool's ability map runs without Counterattack (its bit isn't mapped). A
  stale doc line about the world address was fixed.
- Not seen in game.

---

## Stage 14: KH1 Guard blocks, its recoil, and the Counterattack, version 0.7.1 (2026-10-30)

> "Go ahead and" (owner, after "I'll start on Guard"); decision 20 (KH1 for Sora's side; new
> Re:CoM enemy reactions behind a toggle, next version)

- KH1's rules (`docs/recom-kh1-attack.md` § Guard): guard object live on frames 10–36; kind-6
  success `0x14029D720`; Counterattack offered from frame 12 of request 0x6E (`0x140294220`,
  `0x140293480`), row 0x3E. Checked (decomp + files).
- Re:CoM side: Sora's body guard `player+0x72C` bit 0 and `+0x740` = 100° while live; the
  guarded bit `+0x72C & 0x20` read in the damage-check wrapper; flag 0x80 as `+0x418 & 1`; the
  recoil plays Re:CoM's row 0x24 (`p001_081`, KH1 entry 72's bake); the Counterattack plays the
  KH1-only clip `p001_226` through a second appended action row (`rows::KH1_ONLY`) and a KH1 row
  0x3E in the attack table. Checked (decomp).
- kh1-core: motion 0x6E (entry 72: 60 frames, free 28–60, window 28–60), the Counterattack
  ability bit, a `GuardSucceeded` event. The oracle's controller has no clash yet; the handler
  tests check KH1's numbers directly.
- ini: `counterattack = on` (default).
- Not seen in game. Enemy reactions (decision 20's toggle) are the next version.

---

## Stage 13: KH1 damage and HP on Re:CoM's enemies, Sora's KH1 Strength, version 0.7.0 (2026-10-30)

> "Yes do that order, and have it follow the Rod" / Strength path: "Rod, give up Shield" /
> enemy stats: "Same world, LV1"

- **Strength** (decision 18): new game btltbl `0x39D8` (Sora Strength 4), Dive deltas
  `0x128 + 4 × choice` (keep Rod, give up Shield: +0), level bonuses `0x3AC0` (bonus 4 =
  Strength +2, `0x1402B51B0` → `0x1402B5660`), Kingdom Key +3 (`0x1402A5130`). 7 at level 1 … 51
  at 99. Checked (decomp + files).
- **Enemy records** (decisions 12, 21): KH1's AI syscall 45 records, read by the asset step
  from each twin's `.mdls` part 5 at the script offsets found by evaluating the scripts (per
  world for the Shadow, LV1 / LV2 for the rest). 31 Re:CoM classes. Checked (decomp + files).
  Pairings by internal name only: Assumption, tagged in the file.
- **Runtime** (`win/stats.rs`, new hooks `0x14017FF50` enemy stats init and `0x14017F5A0` max HP;
  callees `0x1402339C0` world, `0x1401AAAE0` Riku's story; all byte-checked): KH1 HP on twins,
  KH1 damage written as the hit part's value `+0x92` in the enemy hit handler, then Re:CoM's own
  weak-part / resistance / falloff. Sora's level from `*0x14087CBF8 + 0x1C`. Riku's story and
  classes without a twin keep Re:CoM's numbers. Checked (decomp); not seen in game.
- **Swing aim**: KH1 keeps the yaw taken at the press (`0x1402A7510` → `0x1402A38E0`). The mod
  already does this. Checked (decomp).
- Decisions 19–21 recorded. Guard (decision 20) and M6 are the next versions.
- Assumption: Re:CoM's damage path adds nothing else to the value (`0x14015D5F0` can rewrite
  AP; not read). The log names each class's KH1 HP / Defence per world the first time.

---

## Stage 12: Re:CoM's ledge grab from a KH1 jump, version 0.6.3 (2026-10-30)

> "sora is also unable to ledge grab with KH1 selected"

- Addresses: the jump exe `0x1401EDA60` (hooked since Stage 8). Its ledge tail is at
  `0x1401EE0E6..0x1401EE109`: the wall climb check `0x1401E8C00(pl, 1)`, the climb init
  `0x1401F2C20(pl, n)` when the result is above 1, the hang check `0x1401E8420(pl)`, and an
  inline hang block. That block makes the same writes and calls as `0x1401F26A0` (action 9,
  row 0xF). Checked (disasm + decomp).
- Cause: at KH1 setting our hook ran the KH1 air frame in place of Re:CoM's jump exe, so the
  ledge tail never ran (decision 16 keeps Re:CoM's ledges). Checked (decomp).
- Change: after the KH1 air frame, the hook calls those Re:CoM functions under Re:CoM's
  conditions: action 7, rising or falling, and not landing this frame. Re:CoM's hang, climb
  and drop then run as its own actions 9 / 0x10. Four new callee checks with the Steam bytes
  (`sites::MOVE_CALLEES`). The ground ledge checks already ran, because the original move exe
  runs on the ground. Checked (decomp).
- Assumption: a jump or drop from the hang goes through our jump init and continues as a KH1
  jump or fall. Not seen in game yet. The log says "movement: Re:CoM's ledge hang/climb started
  from a KH1 jump" the first time each happens.
- Also, from the owner's 0.6.1 traces 7–8 (no code change): the position jump is fixed. The
  "missing swings" are KH1's own press rules: presses before the combo window, and presses
  after a swing that hit nothing. Details in `docs/recom-kh1-attack.md` § In game, 0.6.1.
  Checked (trace + decomp).

---

## Stage 11: mod version on the Combat row's help line, version 0.6.2 (2026-10-30)

> "also make it so hovering over the recom/kh1 config option shows the mod version as well"

- Address: the help line `camp+0x9D0`, set with `0x140259000` by our Config draw hook
  (`0x1400DF1B0`), unchanged since Stage 6. Checked (decomp).
- Change: the Combat row's help text now ends with "(mod v<version>)". The version comes from
  `crates/kh1-recom/Cargo.toml` at build time, so it is the same as the log's first line. The
  text stays static, NUL-terminated ASCII with no `%`, so `0x140259000` keeps the pointer as
  before. A unit test checks the suffix. Checked (test).
- Assumption: the longer line (82 characters) fits the help box without being cut off. Not
  seen in game yet.
- Also in this build: the 0.6.1 swing / guard / roll fix (Stage 10). The owner's 0.6.0
  retest (traces 3-6) ran 0.6.0, because the DLL installed on the PC was copied before the
  build bot had built 0.6.1. Those traces show the Stage 10 fault in all 9 moves (attack:
  action 0xA; guard: action 6 sub 1; roll: action 6 sub 0), and the Stage 10 hook covers
  actions 6 and 0xA. Checked (trace).

---

## Stage 10: fix the swing / roll position jump, version 0.6.1 (2026-10-30)

> "sora cant do his ground combo, it either makes him jump or teleport or just jitter
> uncontrollably. this also happens in the field" / "something similar happens with dodge roll as
> well"

- Owner's traces (`testers/recom-attack/owner-0.6.0-attack-*.csv`): on the Attack press, Sora's
  position jumped from the floor at (−416, −750, 1364) to (10.6, −597, −34.7), 153 units below
  the floor. He then free-fell until Re:CoM's out-of-bounds handler respawned him; a press while
  falling gave air row 18. Checked (trace).
- Root cause: the root-motion hook at `0x14024DCA0`. That function returns an absolute position
  (`ctrl+0xF0` + this update's rotated travel, via `0x14024C960`), and the entity task sets
  `P + mask∘(out − P)`. Our one-update delay treated the return value as a travel, so the first
  update of a swing or roll returned (0, 0, 0). Checked (decomp + trace).
- Fix: the hook delays only `out − anchor` and returns `anchor + last travel`. If ours differs
  from Re:CoM's by more than 0.5 m, Re:CoM's stands. It passes through when the frame counter isn't
  running.
- Regression test: swing and roll away from the origin keep Sora's position; travel = KH1's 100.
- New ini key `debug_attack = on` logs the first 200 attack decisions.
- Not yet seen in game.

---

## Stage 9: KH1 Attack inside Re:CoM (M5), spring fix, version 0.6.0 (2026-10-30)

> M5 per the roadmap, plus the owner's report on 0.5.0: "sora needs to be able to jump as high as
> he is supposed to on these springs"

- **Springs:**
  - Re:CoM's jump init launches springs: gimmick `0x2002`, `0x1401E80C0(pl, 0x2002) == 7`,
    ability `+0x41C & 0x20000`, speed from `0x1405A0100` (39–75.8 against a normal 28),
    `+0x418 |= 0x1000000`, row 0x14.
  - Every KH1 ground jump now runs Re:CoM's spring test first. On a spring, Re:CoM's own jump runs
    until landing, then KH1's rules resume. Checked (decomp); not yet seen in game.
- **KH1 Attack** at `combat = kh1` (`docs/recom-kh1-attack.md`):
  - In battle, the confirm press takes Re:CoM's field (cardless) branch, and the use-card button
    is skipped.
  - initAttack becomes the field-swing init. kh1-core's picker chooses the row in Sora's vtable
    slot `+0x128` (`0x1405A9A60`).
  - Our wrapper on the field swing exe runs KH1's combo window, chain, facing and air arc, then
    Re:CoM's exe.
  - Battle jump-table entry `0x1401FA074` points action 0xA at a 13-byte stub in the int3 gap at
    `0x1401FA6C1` (15 free bytes, lead-checked).
  - KH1 rows in Re:CoM's layout reuse Re:CoM's clips 1–7, with KH1's hit and combo windows (flag
    0x80) and no 20-frame lockout.
  - KH1's two-sphere blade plus sweep (radius 0.40 m), tested with Re:CoM's own primitives.
  - Card priority: a KH1 hit interrupts the enemy's card through Re:CoM's `0x2000001` path.
  - Reactions: swings flinch (kind 1), finishers knock away (kind 2).
  - ini: `combo_plus`, `air_combo_plus`, `combo_master`.
  - Hooks also on `0x1401F8150`, `0x1401E8FF0`, `0x1401F0D90`, `0x1401F0F70`, `0x1401EAEB0`,
    `0x14016EAD0`, `0x14017F7E0`, `0x1401E6DE0` and `0x14024DCA0`. 54 sites byte-checked.
- **M4 follow-ups:**
  - High Jump plays Re:CoM's rows 0x14/0x15.
  - Root motion is one update late for Sora (the roll is now exact against kh1-core).
  - **Finding:** KH1's roll has no untouchable frames. The `0x2B`/`0x2A` ops are only queued by
    event scripts, so roll protection = 12 no-flinch frames, damage still taken. This corrects
    KH2-Combat-in-KH1's `kh1-intangible.md`. Checked (decomp).
- **Damage:**
  - Still Re:CoM's (logged FALLBACK). KH1 enemy stats come from 0x6C-byte records passed by an AI
    script call (syscall 45, `0x1402BBDF0` → `0x1402A58E0`). Strength/Defence index a 21-row table
    at btltbl `0xC8A8`. Checked (decomp + files).
  - The per-enemy records are in KH1's enemy script files, not read yet.
- **Not done:** Guard repel / Counterattack (Re:CoM's guard fields at `player+0x690` found, not
  wired); MP charge (needs the enemy's KH1 level).
- 1:1, Checked (tests vs kh1-core): ground 13→11→8, missed-swing refusal, air 18→17→16, landing
  cutting an air swing, jump beating Attack, the roll.
- Assumptions: hit window one frame earlier (Re:CoM's hit test runs after the animation update);
  the target aim point stands in for KH1's body part. Nothing seen in game yet; tester thread
  `testers/recom-attack/thread.md`.

---

## Stage 8: KH1 movement inside Re:CoM's player actions (M4), version 0.5.0 (2026-10-30)

> "looks good enough for now, lets work on phase 2"

- At `combat = kh1`, for the main `CPlayerSora` only (Riku untouched), Re:CoM's own movement
  actions run KH1's rules and numbers (`docs/recom-kh1-movement.md`):
  - actions 2/3/4 are ground movement, 6 Dodge Roll / Guard, 7 take-off / air / landing, 8 glide;
  - Re:CoM still plays the animations, applies root motion, moves and collides Sora.
- Hooks, all byte-checked and all-or-nothing (39 site byte lists re-checked against the exe):
  - move exe `0x1401EEDA0`, stick class `0x1401E9630`, move step `0x1401EF3C0`;
  - jump init `0x1401F29F0`, avoid init `0x1401F10C0`, jump exe `0x1401EDA60`;
  - Sora avoid exe `0x140217310`, Sora glide exe `0x140217B50`;
  - trace-only hooks on Field / Battle exes (one flag test while not recording).
- Units, Checked (exe + decomp):
  - KH1 (x, y, z) cm, y down = Re:CoM (x, −y, −z)/100 m, y up;
  - KH1 yaw ψ = Re:CoM π − ψ;
  - Re:CoM's `+0x464` = dt × 0.02 (60 Hz frames);
  - Re:CoM's jump peaks at 1.92 m at frame 28, KH1's at 190 units at frame 28.
- Animations follow decision 11: Re:CoM's rows, plus three appended rows for the KH1-only High Jump,
  its apex, and Guard (from the asset manifest).
- Abilities in `kh1-recom.ini` (`high_jump`, `glide`, `superglide`, `dodge_roll`, `guard`).
- F9 Re:CoM trace in the kh1-trace schema.
- 1:1, Checked (tests) against kh1-core frame for frame. Known: the Dodge Roll moves one frame
  early (same distance and end point), and follow-on animations start one frame late (the lock
  releases on KH1's frame).
- Owner decisions recorded this stage (design doc 12–15): KH1 damage formula with KH1-twin stats;
  keep Re:CoM's hit extras; KH1 card priority (hits land); enemy cards hidden.
- Not checked in game. Tester thread `testers/recom-movement/thread.md`.

---

## Stage 7: Config window border fits the Combat row, version 0.4.2 (2026-10-30)

> Owner's 0.4.1 screenshots: the fill reached below "Combat" but the border stopped at "Game
> Mode". The help line on Combat was right.

- Re:CoM's 9-slice (`0x14024C270` / `0x14024AD40`) only handles sprites with exactly 9 parts. The
  panel's border and highlight are 8-part sprites, so the earlier bottom offset never reached
  them. Checked (disasm).
- Fix: at the same open/close points, nothing per frame:
  - each border part below the sprite's middle gets its own copy of its shared sprite-set rect,
    moved down 26 units;
  - this moves the bottom corners and edge and stretches the sides;
  - Re:CoM re-reads the rect each update (`0x14024B380`), so it then draws the frame itself.
  - The shared data isn't changed.
- In game (owner's 0.4.1 screenshot): the Combat row works: cursor, highlight, and the help text
  fits on one line. The border fix hasn't been seen yet.

---

## Stage 6: owner's animation rule, Config panel fit, version 0.4.1 (2026-10-30)

> "i wouldn't bother having an option to switch between KH1 and RECOM animations because for all
> intents and purposes they're the same, just use RECOM's default animations, and KH1 animations
> for anything that isnt included in ReCOM"

- Asset step (`kh1-recom-assets`, new `matching.rs`): each KH1 entry is matched against Re:CoM's
  clips at their key frames on all 135 bones.
  - 39 are bakes: every bone ≤0.9° and ≤1 mm; worst 0.74°, the four known pairs <0.02°.
  - 10 are Re:CoM edits: ≤20 bones differ, no other owner.
  - The closest wrong pair is 17 bones; the closest clip for a KH1-only entry is 26 bones.
  - Matches all 46 of `clip_origin`'s pairs, plus 0→p001_001, 65→p001_043, 78→p001_157.
    Checked (files, tests).
  - Only the 69 KH1-only entries get new slots (161–229). `PL0001.ANN` is 10.0 MB and `PL0002.ANN`
    0.84 MB. `kh1-motions.json` maps all 118 KH1 entries to the Re:CoM slot that plays them.
- DLL:
  - The "KH1 motions test" row, its ini key and the action-table row swap are removed. An old ini
    line is ignored.
  - Config panel: in the existing draw hook, Re:CoM's own 9-slice (`C2DSprObj`, vtable
    `0x1406077D8`, corner offsets `+0xA0..+0xAC`, flag 8 at `+0x144`, applied in `0x14024AD40`) is
    given one extra row of bottom offset. Checked (decomp); that the panel art is a 9-part sprite is
    an Assumption, and the log reports it.
- In game (owner's screenshot of 0.4.0): our rows render with Re:CoM's own buttons, font and value
  highlights. The panel was one frame short, which is what this stage fixes. Not yet seen in game.

---

## Stage 5: Phase 1 in Re:CoM: KH1 motions asset step, motion switch, Config rows, version 0.4.0 (2026-10-30)

> "lets finish phase 1" (ROADMAP M2 + M3)

- **Asset step** (`crates/kh1-recom-assets`, CLI `kh1-recom-build`; `docs/kh1-recom-assets.md`):
  - Converts all 118 KH1 Sora motions (via `kh1-anim`) into new 60 fps slots 161–278 of Re:CoM's
    `PLAYER/0001/PL0001.ANN`, and the matching Keyblade clips into `PLAYER/0002/PL0002.ANN`.
    Re:CoM's own slots stay byte-identical.
  - Writes `<mod_path>\Recom\…` plus `kh1-motions.json`. Panacea serves files from
    mod_path + "/recom" by archive path. Checked (OpenKH Panacea source).
  - Validation: Re:CoM pack decode → encode is byte-identical; converted clips match KH1's pose
    ≤0.045 mm / 0.005° per frame. Against Square Enix's own bakes they are ≤0.019° (body) and
    ≤0.007° (Keyblade, entry 98). Checked (files, tests).
  - Known: the Keyblade tip `k9` is off by 1.58° in entry 4 (KH1 bone 283's copy-orientation
    constraint in `kh1-anim`), and by 0.18° in entry 74.
  - Loader: no 161-slot limit; sizes come from the file (`0x14029F3A0`), and the name lookup
    (`0x1401A47E0` / `0x1402A42B0`) handles slots ≥161. Checked (decomp).
- **DLL** (`crates/kh1-recom`; `docs/recom-config-menu.md`, `docs/recom-motion-switch.md`):
  - Hook infrastructure: byte-checked, all-or-nothing. Starts from Panacea's `OnInit`.
  - Config: two rows, "Combat: Re:CoM / KH1" and "KH1 motions test: Off / On", added to the camp
    menu's Config screen using Re:CoM's own item, layout, cursor and help code. Hooks: Config input
    `0x1400D3EB0` (`48 89 5C 24 08 57 48 83 EC 20`) and draw `0x1400DF1B0`
    (`48 8B C4 55 41 54 41 55 41 56`). Bytes re-checked by the lead against the exe.
  - The values are saved in `kh1-recom.ini`: the Config save has no byte that can be proven free.
  - `config_menu = off` is a safety switch.
  - The KH1 motions test repoints 12 name pointers in Sora's action table (`0x1407C5E80`): idle,
    walk, run, swings 1–2, lunge, finisher. Re:CoM resolves and plays them itself from the next
    room load. Data only, nothing per frame.
- Not checked in game: the rows' look and behaviour, the clips playing, the Keyblade following.
  Tester thread `testers/recom-phase1/thread.md`.

## Stage 4: KH1 trace logger, replay and diff (M1), version 0.3.0 (2026-10-30)

> "lets finish phase 1" (ROADMAP M1)

- `crates/kh1-trace` (`docs/kh1-trace.md`):
  - `kh1-trace.dll` for KH1 FM Steam. Build check: PE time stamp `0x669E37CE`, size
    `0x2F91000`.
  - One hook on the per-actor task `0x140297FE0` (`48 8B C4 57 48 83 EC 60`). F9 records
    `kh1-trace-<n>.csv` with about 70 columns: state, motion, position, velocities, combo, and the
    inputs.
  - `kh1-tracediff` compares two traces.
- Playground: `--replay <csv>` feeds the recorded inputs to the controller and diffs the result.
  Replay is deterministic byte for byte (tests).
- Finding: KH1's "ground contact" is +0x2D0 bit 0, not bit 1. Checked (decomp).
- Assumptions: the camera yaw formula (checked against KH1's own move vector on real traces); Sora
  is party slot 0; Panacea calls `OnInit` before frames run.
- Not run in KH1 yet. Tester thread `testers/kh1-traces/thread.md`.

---

## Stage 3: smoother Sora in the playground, PS5 controllers, version 0.2.1 (2026-10-30)

> "lets make the actual character model looking smoother, and make sure ps5 controllers work with
> it" (owner chose a smoother mannequin, not the real mesh)

- `kh1-playground/src/body.rs` replaces the old one-capsule-per-bone figure:
  - smooth tapered limbs (two spheres joined by a cone, with smooth normals) and rounded joints;
  - the pelvis, chest, head and shoes are ellipsoids oriented by the surrounding bones;
  - Sora's 14 real hair spikes, jacket tails, hood and fingers follow their own bones;
  - MSAA ×4, plus a fill light.
- The KH1 bone roles come from matching KH1's skeleton to Re:CoM's named one, e.g. 17/18/20/23 for
  the right leg and 93/94/97/100 for the right arm. Checked (files).
- **Finding:** KH1 doesn't pose the end bones of chains (flags `0x6000000`: hair tips, jacket
  tails, hood). The figure places them from the parent's world matrix and the end bone's rest
  translation. Checked (files: idle pose dump).
- Gamepads:
  - Bevy's gilrs uses Windows.Gaming.Input with SDL's controller database, which includes the
    DualSense `054C:0CE6` and the Edge `0DF2` over USB and Bluetooth (gilrs builds USB-style
    GUIDs). Checked (crate source; the mapping string is in the built exe).
  - The HUD shows each pad's name, ids, stick and held buttons, with PS labels.
  - Not tested with a real pad.
- No change to the rewrite crates or the Re:CoM DLL, apart from the version number.

---

## Stage 2: KH1 battle system Rust rewrite + playground demo, version 0.2.0 (2026-10-30)

> "Lets try to do a full 1:1 Rust Rewrite of the KH1 Battle System, similar to how we did the KH2
> battle system rewrite, with a demo i can test" (owner: reference + demo; first demo movement +
> Attack)

- New crates (`docs/kh1-rewrite.md`):
  - `kh1-formats`: reads PC HED/PKG and KH1's `.mset`, `.mdls` and `btltbl.bin`.
  - `kh1-anim`: KH1's posing pass (curves, start pose, constraints, two-bone IK, frozen-snapshot
    blends).
  - `kh1-core`: the 60 Hz controller and combat.
  - `kh1-data`: install discovery and `SoraKit`.
  - `kh1-playground`: Bevy demo.
- Data is read at runtime from the player's `kh1_first.pkg` (`xa_ex_0010.mset/.mdls`,
  `btltbl.bin`). Checked (files: decoding gives the extracted files' SHA-256).
- `kh1-anim` matches Square Enix's own Re:CoM bakes of the same KH1 motions to ≤0.019° local
  rotation and ≤0.051 mm world position on swing / roll / run / jump. Checked (files, tests).
- `kh1-core` corrections to the notes (cone 22°, air row 17 above, combo 13→14→8 with a close
  target, …) are in `docs/kh1-sora.md`. Checked (decomp).
- Assumptions are marked in the code and the HUD: root travel subtraction, MSVC `rand`, task order,
  dummy stats (Strength 5, Defence 2, 300 HP), and the camera (an orbit camera, not KH1's).
- Headless check: a scripted combo runs rows 13 → 14 → 8 and hits the close dummy for 3, 3 and 7.
- The `kh1-recom` DLL is unchanged apart from its version (0.2.0); it still only checks the game.
- 70 tests (all pass with `KH1_TEST_DATA`); clippy clean.
- Not checked against a KH1 trace yet. Gamepad untested.

---

## Stage 1: the mod's skeleton DLL, version 0.1.0 (2026-10-30)

> "B, and use whichever is least effort and is most native to RECom" (owner: Panacea loads the
> mod; at KH1 setting Sora plays KH1's motions re-baked at 60 fps)

- New crate `crates/kh1-recom`: a cdylib that OpenKH's Panacea loads (`openkh/mod.yml`, copied as
  `dll/kh1-recom.dll`). On load it starts one thread and then nothing runs per frame:
  - checks the exe is the Steam build: PE time stamp `0x669E38DF`, size of image `0xD4A000`.
    Checked (exe).
  - checks the first 16 bytes of each future hook site. Checked (exe bytes = decomp function
    starts):
    - initAttack `0x1401F0D90`
    - field swing init `0x1401F0F70`
    - player motion request `0x1401F7180`
    - attack row pick `0x1401F7520`
    - Battle exe `0x1401F9840`
    - Field exe `0x1401FA0F0`
    - use-card button `0x1401E8FF0`
    - `.ANN` loader `0x14029F3A0`
  - reads `kh1-recom.ini` (`combat = kh1 | recom`, default `kh1`), writing it if missing. Nothing
    reads the setting yet. The Config-menu toggle is still to do.
  - writes `kh1-recom.log` next to the game.
- **No hooks installed and nothing patched.** The game plays exactly as Re:CoM.
- Panacea knows Re:CoM: its `DBGHELP.dll` names the Re:CoM exe, uses a `/recom` mod subfolder, and
  hooks Re:CoM's `Axa::CFileMan`. Checked (files: strings in the owner's Panacea DLL). Whether it
  loads `dll/*.dll` for Re:CoM as it does for KH1: Assumption, to check with the first launch.
- Build: MinGW cross-compile (`tools/build-win.sh`); build bot `.github/workflows/build.yml`
  commits `openkh/kh1-recom.dll` on `testing`. 7 unit tests.
- Not checked in game yet: that Panacea loads the DLL, and the log on the owner's machine.

---

## Stage 0: groundwork (2026-10-29)

> "Where I'd start: a cardless Attack, built into Re:CoM's own battle code. Before that comes a
> short groundwork step (step 0)."

No code. `VERSION` not created yet; the first change under `crates/` starts it.

- Repo seeded: these rules files, `docs/sources.md`.
- Re:CoM Steam decomp made (see `docs/sources.md`). Checked (exe): SHA-256 `acf42e96…6bbf`.
- Finding: Re:CoM's exe is mixed-mode C++/CLI; the .NET part is only the shell. Checked (exe).
- Finding: the KH1 notes in KH2-Combat-in-KH1 use Epic addresses; convert to Steam before use.
  Checked (files).

Step 0 mapping (2026-10-29, notes only):

- `docs/recom-timing.md`: both games step by a variable delta in 60 Hz frames. Re:CoM uses 50
  units per frame, so KH1 frames × 50 = Re:CoM time. Checked (decomp + exe, both games).
- `docs/recom-player.md`: player state machine (table-driven, 4 states: Field / Battle / Wait /
  Hide), the action switch, and the card path from button to `initAttack`. Checked (decomp).
- `docs/recom-combat.md`: motion request and advance, Sora's attack-row table `0x1407C6C70`, the
  hit path and the damage formula. Checked (decomp; rows read from the exe).
- `docs/recom-battle-entry.md`: field-to-battle handover (separate battle room) and the native
  cardless field swing (action `0xA`, card id 0) that shares battle collision. Checked (decomp).
- Assumption, well supported: Re:CoM's motion system is not KH2's mset system.

Step 0 still to do:

1. Plan the trace tool with the tester: the same fields from both games (tick, state, motion,
   frame, position, velocity, hit events).
2. KH1's side mapped in `docs/kh1-sora.md` (states, movement, jump, glide, roll, guard, motion
   requests, the Attack combo). Still needed: the numbers in KH1's data files (`btltbl.bin`,
   `xa_ex_0010.mset`) and Re:CoM's (`stockskilllist.bin`, `PL0001.ANN`), from the player's
   extracted files.
3. Open, not read yet: the damage veto `0x140180920`, the AP rewrite `0x14015D5F0`, the
   double-hit spawn `0x140158DD0`, and which hit kind the field swing produces on map enemies.
