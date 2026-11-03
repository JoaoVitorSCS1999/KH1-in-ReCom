# KH1's Attack inside Re:CoM's attack action (roadmap M5)

Steam addresses (Re:CoM image base `0x140000000`). Tags: Checked (decomp / exe / files / tests) or
Assumption. Code: `crates/kh1-recom` (`attack.rs`, `movement.rs`, `win/attack.rs`). Design:
`recom-attack-design.md`. Oracle: `crates/kh1-core` (`combo.rs`, `controller.rs`, `shape.rs`,
`damage.rs`).

## Status

- Built, `combat = kh1`, main `CPlayerSora` only (Riku and `combat = recom` pass through):
  - KH1's ground and air combo through Re:CoM's own attack action `0xA`, in the field and in
    battle;
  - KH1's rows, hit windows and combo windows; KH1's picker, counter and last-swing-missed rule;
    Combo Plus, Air Combo Plus and Combo Master from `kh1-recom.ini`;
  - KH1's Keyblade sphere chain, tested with Re:CoM's own collision primitives;
  - KH1's card-priority rule (hits land during enemy card plays);
  - KH1's reactions mapped to Re:CoM's kinds;
  - the M4 follow-ups: High Jump on Re:CoM's rows 0x14 / 0x15, the roll's root motion one update
    late, KH1's no-flinch roll protection;
  - Re:CoM's springs keep their own jump (`recom-kh1-movement.md` § Springs);
  - the F9 trace's attack columns.
- Stopped, with the reason below: KH1 damage numbers (KH1's per-enemy stat records aren't in the
  files read so far), Guard's repel and Counterattack, MP charge.
- Checked (tests): on a fake Re:CoM that does what Re:CoM does around the handlers (the motion
  clock, root motion one update late, the mover), the handlers match kh1-core frame for frame
  (position, facing, state, lock, rows started, refusals) for:
  - the ground combo 13 → 11 → 8 and a new combo;
  - the last-swing-missed refusal, and the chain when the swing hits;
  - the air combo 18 → 17 → 16;
  - a landing cutting an air swing;
  - Attack and jump on one frame (the jump wins);
  - the Dodge Roll (now exact, root motion included).
- Not checked in game. Tester thread `testers/recom-attack/thread.md`.

## How it sits in Re:CoM

### The path of one swing

1. **Input.** The request-bits builder `0x1401F8150` runs its field branch in battle too (our
   wrapper sets `+0x390 = 0` for the call; Re:CoM's battle-branch early returns are kept). So the
   confirm press sets `+0x424 & 4` (with Re:CoM's own ability bit `0x41C & 0x40` and lockout
   `+0x654` tests), and there's no card scroll, reload, stock or sleight input. The use-card button
   `0x1401E8FF0` is skipped. A jump on the same frame clears the Attack bit (KH1:
   `if !jump { attack } else { refused }`). Checked (decomp).
2. **Dispatch.** Re:CoM's own command dispatch starts it:
   - the move exe `0x1401EEDA0` calls the field-swing init `0x1401F0F70`, or in battle initAttack
     `0x1401F0D90`, which our wrapper turns into the field-swing init;
   - in the air, M4's jump-exe hook runs Re:CoM's air commands.
3. **KH1's frame decides.** Our field-swing-init wrapper runs KH1's frame (`Kh1Sora::frame`) with
   the press. KH1's command `0x1402A7510` applies the counter rules and the picker
   (`kh1_core::combo`). The row exec `0x1402A68F0` does the hop, the air speed and the facing.
4. **Re:CoM starts it.** `Host::start_swing` runs Re:CoM's own field-swing init, whose row pick
   (Sora's vtable `+0x128`, our slot) takes the chosen KH1 row:
   - `+0x3C8` → the KH1 row, `+0x638` / `+0x630` → the Re:CoM id of the clip;
   - the clip via `0x1401F7180` (root motion as KH1's request map has it);
   - AP, kind and attribute;
   - `+0x63C` / `+0x640` = KH1's counter / length;
   - `+0x654 = 0` (KH1 has no 20-frame lockout);
   - `+0x4E0 = 0` (a swing starts from standing: KH1 sets speed 0, root travel only);
   - `+0x648 = 0` (Re:CoM's own chain gate never fires).
5. **Each frame (action `0xA`).** Our wrapper on the field-swing exe `0x1401EAEB0`:
   - (a) runs KH1's frame first: the combo window, the next press (it chains through steps 3–4 on
     the same frame, as KH1 does), the facing turn π/16, the air arc;
   - (b) runs Re:CoM's own exe: the hit window and `procAttackCollision` → `onAttack`, its end at
     the clip's end;
   - (c) writes KH1's velocity and facing again.
6. **Battle.** The Battle exe had no case for action `0xA` (its table `0x1401FA050` sends ids
   without a case to `0x1401F9CC9`). Its entry for `0xA` now points at a 13-byte stub in the int3
   cave `0x1401FA6C1` (after a `ret`, before `0x1401FA6D0`): `mov rcx,rdi; call 0x1401EAEB0; jmp
   0x1401F9CC9`. That is what the table's own cases do. Checked (disasm `0x1401F9959..0x1401F998A`):
   `rdi` is the player, `esi` and `xmm6` are callee-saved.
7. **The end.** When KH1's swing motion ends into its queued idle / fall, a landing cuts it, or a
   walk-off replaces it, `Kh1Sora::leave_swing` calls Re:CoM's `0x1401F6660`, which ends its attack
   (the weapon's volume, effects, fields), and KH1's next motion plays. If Re:CoM ends it first, the
   next KH1 frame takes over from Re:CoM's state. An air swing Re:CoM ends keeps KH1's arc into the
   fall.
   **A won battle:** with Re:CoM's win pending (`player+0x428 & 4`), `0x1401F6660` enters the win
   action `0x1C` instead (`0x1401F6794` → `0x1401F1B20`); `leave_swing` sees it and the KH1 frame
   yields (no idle motion or action after it). When Re:CoM's own exe ends the swing (5b) into the
   win, the wrapper's re-write (5c) is skipped (the action is no longer `0xA`). Checked (decomp +
   tests); `recom-m6-battles.md` § Battle end.

### KH1 rows (`attack::kh1_row_bytes`). Checked (exe + files + tests)

Each KH1 row is a copy of the Re:CoM row (rows 1–7, `0x1407C6C80`, checked at start against the
Steam bytes) that plays the same KH1 motion, with:

| byte | KH1 row value |
|---|---|
| `+1` / `+3` | KH1 combo window start / end (0 / 0 = none) |
| `+4` / `+5` | KH1 hit window, one frame earlier (below) |
| `+6`..`+0xA` | 0: no homing, no stop distance, no speed, no card column |
| `+0xB` | Re:CoM kind: KH1 kind 3 (flinch) → 1, 5 (launch) → 2 |
| `+0xC` / `+0xD` | 0: no push, no Re:CoM hop (KH1's arc runs in the handler) |
| `+0xF` | `0x80` only ("KH1 row"; no Re:CoM chain, early cancel or push) |

| KH1 row | motion | Re:CoM id (clip) | KH1 hit | row `+4/+5` | combo window | kind |
|---|---|---|---|---|---|---|
| 13 | 200 | 1 (`0x30`) | 14–24 | 13–23 | 22–40 | 1 |
| 11 | 201 | 2 (`0x31`) | 18–32 | 17–31 | 26–42 | 1 |
| 14 | 202 | 3 (`0x32`) | 18–30 | 17–29 | 26–54 | 1 |
| 8 | 203 | 4 (`0x33`) | 26–38 | 25–37 | none | 2 |
| 17 | 205 | 5 (`0x34`) | 10–20 | 9–19 | 18–32 | 1 |
| 18 | 204 | 6 (`0x35`) | 6–20 | 5–19 | 18–34 | 1 |
| 16 | 206 | 7 (`0x36`) | 12–38 | 11–37 | none | 2 |
| 1 jump-up | 205 | 5 | 10–20 | 9–19 | 18–32 | 1 |
| 2 hop | 204 | 6 | 6–20 | 5–19 | 18–34 | 1 |

- **M9 stage 9a** adds the combo modifiers' rows (0, 3–7, 9, 10, 15), each on its own clip
  (Re:CoM's bake or an appended action row), Stun Impact's body sphere and the % of HP / Magic
  damage: `recom-m9-abilities.md`.
- **The hit window one frame earlier.** Re:CoM tests a row's window in its exe against the clock
  of the last motion update. The weapon's collision runs later in the frame on the new pose
  (`0x140225840` calls the test itself). KH1 tests its flags after this frame's update. So KH1's
  `start ≤ clock(N) < end` is Re:CoM's `start − 1 ≤ clock(N − 1) < end − 1`. Assumption: the
  weapon's task runs after Sora's motion update in the same frame. The trace's `hit_active` column
  (the weapon's `+0x23C`) shows it in game.
- The combo-window test is our own frame's (KH1's flags on Re:CoM's clock after the last update),
  as KH1's think reads them. The row's `+1` / `+3` record the same window for Re:CoM-side readers.
  Nothing of Re:CoM reads them.
- Not the design's gate patch at `0x1401EAF95`. Re:CoM's chain (`+0x650`) starts the next swing on
  the frame *after* the press, KH1's on the press frame. Our frame starts it through Re:CoM's own
  init on the press frame instead.
- Hop row 2 needs KH1's obstacle probe `0x1402C6E70`. Not built yet: the probe returns "no wall"
  (Assumption, as kh1-core's default `World`).

### Targets

- KH1's picker gets Re:CoM's attack target `player+0x84C` as a body part. Its point is Re:CoM's
  own aim point (vtable `+0x50`, as Re:CoM's row pick reads it). Assumption: that point stands
  for KH1's lock point.
- **Radius (0.7.5):** the KH1 twin's lock-point radius, from `kh1-stats.json`.
  - KH1's ground picker (`0x1402A7320`) measures gap = horizontal distance − the radius of
    each kind-0 lock point (`0x1402B0110` iterates them, `0x1402B04C0` returns point `+0xC` ×
    actor `+0x40`). Checked (decomp).
  - The points are the last group of the model's collision block (`.mdls` part 4, `CLS\x03`).
    Checked (files).
  - Shadow: 30 (0.30 m), so KH1 lunges at a Shadow 2.10–3.30 m away. With radius 0 (before
    0.7.5) the mod lunged at 1.80–3.00 m and swung in place at 3.00–3.30 m, where the swing can't
    reach (`hitbox-compare.md`).
  - Enemies without a twin keep radius 0.
- Assumptions:
  - The twin's actor scale (`+0x40`) is 1.0.
  - The Screwdiver has four kind-0 points (170, 40, 40, 40). The mod's single target uses the
    first.
- It counts as KH1's lock-on when Re:CoM's lock-on is on (M4's lock-on fields). Without a lock,
  KH1 scans every enemy part in its cones; here the scan sees only that target. M12 (lock-on)
  owns the rest.

## The Keyblade volume

### Re:CoM's. Checked (decomp)

- The weapon's `CCollAttack` (`weapon+0x240`, shape `+0x258`) is a type 7 thick triangle.
- `0x140225840` moves it each frame while `weapon+0x23C ≠ 0` and calls its test (vtable `+0x30`
  = `0x14016F110` → pair dispatch `0x14016EAD0(coll, body list)`). Bodies of type 2 / 4 / 5 are
  tested with `0x140177480` / `0x140176BE0`, then `coll` vtable `+0x38` = `0x14016F1C0` applies the
  hit (serial check: one hit per body per swing).
- Sphere primitive `0x1401777E0(ctx, body, attack)` and column primitive `0x140176FA0(ctx, body,
  attack, 0)` read the attack sphere's `+0x10` / `+0x20` (swept centres), `+0x70` radius, `+0x74`
  height. With `+0x10 = +0x20`, `0x1401782A0` is the plain test `|body − centre| < r₁ + r₂`
  (strict), the same as KH1's sphere test.

### KH1's on it (`attack::blade_points`, `blade_shapes`; our pair-dispatch wrapper)

- For the main Sora's weapon on a KH1 row, our wrapper tests KH1's spheres instead, with those two
  primitives, on the same body list and with the same skip rules. A hit runs Re:CoM's own apply on
  that body (its serial, back bonus, weak parts, effects).
- **Blade.** KH1's line runs from bone 282 (t = 0) to bone 283 (t = 1). Re:CoM's Keyblade bones
  `wep0` (2) and `k9` (3) are those bones × 0.01 (Checked (files) ≤ 0.012 mm,
  `kh1-recom-assets.md`); their world origins are read with `0x1402ABB20(weapon, i) + 0xD0`.
  Kingdom Key: A at t = −2.96 (0.625 m along the blade), B at t = 0. Radius 0 + 1.0 × 40 = 0.40 m.
  So the blade chain is 2 spheres, plus the swept chain from last frame's A.
- The first tested frame sweeps from Sora's centre. Assumption: `player+0x4B0`, the point
  `onAttack` copies as the attack origin.
- Not a new type id `0xD`: the shape stays type 7. Re:CoM's hilt and radius setters
  (`0x14016F760`, `0x140172690`) only write type 7, and the triangle's own tips feed Re:CoM's hit
  effects. The KH1 test is chosen by the row's `0x80` flag in the dispatch.
- Targets stay Re:CoM's own hurt shapes (M10/M11).

### Enemy hurt shapes, Re:CoM vs KH1: the Shadow (2026-10-31)

- **Re:CoM `ENEMY/0001/EN0001.ASD`** (`@COP`, version 3; body section: one 0x50-byte record;
  layout from `0x140155160`). Checked (files + decomp):
  - One **sphere** (kind 0), radius **0.55 m**, on bone `ex_2020_sk` (index 1), offset
    (0, 0, 0.1).
  - That bone is at the hips: 0.61 m up in the rest pose (`EN0001.MDL`).
  - The file's other section holds 3 attack records (`r_hand` / `l_hand`, clips `e001_050` /
    `e001_052`).
- **KH1 `xa_ex_2020.mdls` part 4.** Checked (files + decomp):
  - Two **ellipsoids**, radius 0.35 m.
  - Hips: bone 2, 0.61 m, half height 0.40 m.
  - Neck: bone 30, 1.06 m, half height 0.35 m.
- **Comparison** (rest pose):
  - Re:CoM's Shadow is wider (0.55 against 0.35 m across) and spans 0.06–1.16 m in height.
  - KH1's is narrower and taller: 0.21–1.41 m.
  - Against KH1's 0.40 m blade sphere, a hit needs the sphere centre within 0.95 m of Re:CoM's
    centre, against 0.75 m of KH1's part centres.
  - So KH1's blade hits Re:CoM's Shadow more easily than KH1's own Shadow, except for blows
    above about 1.2 m. The Shadow's hurt shape is not why swings whiff.
- The two games share the Shadow's skeleton: the hips and the bones below match to the
  millimetre. Checked (files).

## Damage and reactions

- **KH1 enemy stats (0.7.0, decisions 12 and 21).** Research notes: `kh1-enemy-stats.md` (kept
  with the decomp tools). Checked (decomp + files) unless tagged.
  - KH1 fills an enemy's stats block in `0x1402A58E0` from a 0x6C-byte record its AI script
    hands to AI syscall 45 (`0x1402BBDF0`, entry 45 of `0x140529180`): r0 HP, r1 / r2 Strength /
    Defence index into btltbl `0xC8A8` (21 rows), r3 / r4 damage floor / cap per hit
    (`+0xC0` / `+0xC2`, read by `0x1402BFA80`), r5 level, r6 knockback factor, r7 EXP, r9 the
    actor's own damage scale, r10..r25 resistances %.
  - An ordinary Heartless's AI is part 5 of its own `.mdls` (`0x140290D60`, binder
    `0x140287E40`); bosses and a few scripted fights use their room's script. The record is
    picked at init by the LV2 story bit (syscall 198: save `0xDC4` bit 0) and, for the Shadow,
    by world (syscall 105). The script offsets per world / LV were found by evaluating every
    script; room copies of the same scripts are byte-identical.
  - The asset step (`kh1-recom-assets` `stats`) reads each twin's `.mdls` from the player's
    KH1 packages and writes `kh1-stats.json`: Sora's KH1 Strength for levels 1..100 and, for 31
    Re:CoM classes, the twin's record for each Re:CoM world 1..13 (decision 21: the world's
    LV1 record; Twilight Town and Castle Oblivion, which KH1 lacks, get LV2). Pairings named
    only by internal names are tagged `assumed` (Barrel Spider, Sea Neon, Aquatank,
    Screwdiver, Wyvern, Creeper Plant, Neoshadow). Tornado Step isn't identified; it and every
    other enemy without a twin takes a proxy (§ KH1 numbers for enemies without a twin).
  - Runtime (`win/stats.rs`): the enemy's class is its vtable (31 Re:CoM `CEnemy*` vtables,
    RTTI-checked at start); the world is Re:CoM's `0x1402339C0(0)` (1 Traverse Town … 13 Castle
    Oblivion, the order of `0x140506AA0`; `0x14087B844` is the floor, not the world). Riku's
    story (`0x1401AAAE0`) keeps Re:CoM's numbers.
  - **HP:** a wrapper on Re:CoM's enemy stats init `0x14017FF50` (it sets `+0x274` from the
    enemy's per-floor row `+0x310`) sets the twin's KH1 HP, and the max-HP getter `0x14017F5A0`
    returns it for that enemy (both at KH1 only).
  - **Damage:** in the enemy hit handler, a KH1 hit's part value (`part+0x92`, read through
    `0x14016FDB0`) becomes KH1's damage (kh1-core's `0x1402BFA80`: curve[power − Defence],
    the row's bounds, the twin's floor / cap and resistance, a crit roll), with Sora's KH1
    Strength at his Re:CoM level (`*0x14087CBF8 + 0x1C`, the menu's level). The trace's hits
    column adds `:kh1 <damage>`.
- **Decision 13 (keep Re:CoM's extras).** Re:CoM's damage path is untouched. Once the KH1 numbers
  are on, they replace the AP, and the back bonus, weak parts and boss falloff still apply.
- **Reactions.** The KH1 row's kind: swings (KH1 kind 3) → Re:CoM 1 (flinch), finishers (kind 5)
  → Re:CoM 2 (blow-away when the enemy's `@ENP+0xC > 0`). Re:CoM's enemies use their own flinch and
  blow-away states. KH1's launch speed (20 + 15 × weight, hop × 1.35) can't be passed to them:
  Re:CoM's blow-away speeds are in each enemy's state code (not read). It comes with the KH1
  enemies (M10/M11).
- **Card priority (decision 14).** Re:CoM's enemy hit handler `0x14017F7E0` returns before damage
  while `enemy+0x260` bit 0 (card playing) is set. With bits `0x2000001` both set, it first ends the
  card (`0x14017E1D0`), then takes the hit.
  - For a KH1 swing's hit, our wrapper sets `0x2000000` for that call (and takes it off after),
    so Re:CoM's own path ends the card and the hit lands.
  - A KH1 hit is recognised by the body part our dispatch hit: the enemy's parts are
    `*(enemy+0xF8)+0x40`, `0x120` bytes each, count `+0x38`, the same objects the dispatch hands
    to the apply (Checked (decomp `0x140154EB0`, `0x14016F1C0`)). Other attackers keep Re:CoM's
    rule.
  - The trace's `hits` column logs each KH1 hit: the HP before and after, and `:card` when it
    broke into a card play.
  - **Correction (2026-10-31):** on its own this had almost no effect in game. Re:CoM switches
    the enemy's hurt bodies off for the whole attack (below), so Sora's blade found nothing to hit
    while the card bit was set.
- **Hits on attacking enemies (0.8.2, decision 26).** Owner: "sora should be able to hit enemies
  even if they are in the middle of attacking themselves". Research notes:
  `hits-on-attacking-enemies.md` (kept with the decomp tools).
  - **Re:CoM** (Checked, decomp + disasm):
    - When an enemy plays a card (message 0xB in `0x140181620`) it sets `+0x260 |= 5` and moves to
      an attack state 2..10.
    - Each attack state's start runs the enemy's vtable `+0x438`. Its default `0x14017DA80` is
      `0x1401A2D70(enemy, -1)`: every hurt body off (`node+0x10 = 0`).
    - The end runs `+0x440` = `0x14017DA90`, which clears card bits 0 and 2 and switches the
      bodies back on.
    - Vexen and Marluxia 2 call `0x14017DA80` directly. Nothing else calls it.
    - So no attacker can hit an enemy during its card attack. The Shadow's claw (state 2,
      `e001_050`) and leap claw (state 3) are such states.
  - **KH1** (Checked, decomp + files): hurt parts are skipped only when masked in `actor+0x3A8`
    (AI syscalls 116 / 117). The Shadow's script never masks its hurt parts (its three 116 / 117
    pairs change `+0x3AC`, lock points), so KH1's Shadow can be hit during its attack.
  - **The mod:** a wrapper on `0x14017DA80` (5 bytes moved, `BA FF FF FF FF`; its tail jump
    `E9 E6 52 02 00` checked) returns without switching the bodies off at the KH1 setting, in
    Sora's story. That covers every attacker and every enemy, bosses included (owner's choice).
    The attack's end still clears the card bits, and switching on bodies that are already on is
    harmless. A KH1 hit during the card then ends it through the card-priority path above. Runs
    once per enemy attack start.
  - Side effects, from the owner's choice:
    - Friend-card and other player attacks can now hit attacking enemies too.
    - Bosses with card attacks are hittable mid-attack at the KH1 setting. The card-checking
      boss risk stays with M6.
  - **Not checked:** whether in KH1 the blade meeting the claw's own attack object stops the hit,
    and whether a hit interrupts the claw in KH1. Not seen in game.
  - **Exception (2026-11-02, owner):** the attack a humanoid boss's revenge started keeps Re:CoM's
    rule (bodies off, no card-interrupt bit, no Guard flinch) until its state end `0x14017DA90`:
    `docs/recom-boss-revenge.md` § 4.
  - Also fixed:
    - A KH1 hit is now kept through the next swing's start: a chain could start before the
      enemy's update consumed the hit, and the hit then took Re:CoM's damage and rule.
    - The trace's `hits` column is cleared when F9 starts, so the first row no longer carries
      hits from before the recording.

## Re:CoM damage setting (owner, 2026-11-01)

> "Can we change it to where depending on which keyblade sora has equipped, it does recom's damage
> for the corresponding keyblade? lets also have Recom's enemy HP"

Owner's choices: every KH1 Keyblade swing uses Re:CoM's card formula for the equipped Keyblade;
KH1's normal swings take Re:CoM's normal column (0), KH1's combo finisher (KH1 kind 5) the
finisher column (2); Re:CoM's enemy HP; spells and specials do Re:CoM's card / sleight damage; a
setting `damage = recom | kh1`, **Re:CoM by default**. At Damage = KH1 everything below is off and
decisions 12 / 21 stand. At Combat = Re:CoM and in Riku's story nothing changes (every wrapper
passes through there). Code: `crate::damage` (testable), `win::damage`.

### Re:CoM's formula, as read. Checked (decomp + disasm + exe)

- **Row pick `0x1401F7520`, card branch** (`0x1401F7ACC..0x1401F7C24`): with a card (`+0x7F4` ≠ 0),
  `power = BattleCardList[card][row+0xA]` (u16) `+ player+0x664`; if `player+0x420 & 0x4000000`
  then `power += power / 2`; **`AP = attack stat +0x43C × power / 100`** (32-bit `imul`, C's
  truncating division by the `0x51EB851F` multiply); `+0x660` = AP, `+0x668` = row `+0xB` (kind),
  `+0x66C` = card `+0xB` (attribute); status bits = `1 << (n − 1)` for each non-zero nibble of card
  `+9`; then, when `player+0x3E8` ≠ 0, Re:CoM's enemy-card rewrite (`0x14015D5F0` on a 0x20-byte
  hit record), which only changes AP / kind while an **enemy card of Sora's side** is active
  (`*0x14087A818 + 0x1E8` ≥ 0x44). Without a card: AP = the attack stat, attribute 0.
- **The bonus `+0x664`: it applies, and is 0 at Combat = KH1.** `initAttack` `0x1401F0D90` sets it
  before the row pick: `+0x420 & 0x40000000` ? `+0x664 + 20` : 0 (`0x1401F0EEA`). Only enemy card
  0x79's effect sets that bit (`0x140163C00` case 0x79), and Sora plays no card at Combat = KH1
  (Stage 29). We repeat `initAttack`'s rule at each KH1 swing start (it was not run at KH1: our
  `initAttack` hook starts the field swing), so the value is Re:CoM's own.
- **`+0x420 & 0x4000000`** (the ×1.5): set by Re:CoM's card play `0x1401F5580` from `+0x829`
  (with the Dark gauge `+0x44C`; Riku's Dark Mode: Assumption). Repeated as read; never set for KH1
  Sora (no card play).
- **`BattleCardList`** is `*0x14087A808`, set (with `StockSkillList` `*0x14087A810`) by the
  `CCardBattle` ctor `0x1401607A0` from the resident resource "BattleCardList" (other readers cache
  the same resource forever, e.g. `0x14015A260`). Its rows (Checked (files): `SY0002.BIN`'s
  directory entry 1, **offset 0x270**, size 0x7F0 = 127 rows; entry 2, offset 0xA60, is
  `StockSkillList`, 0x960 = 150 rows; each directory entry is u32 offset, 24-byte name, u32 size |
  0x80000000, so the offset word *before* a name is its own): u16 power for columns 0..3, `+8`
  recovery index, `+9` status nibbles, `+0xA` CP, `+0xB` attribute. This is the layout
  `recom-m8-items.md` describes (item cards: power 0, `+8` 3, `+0xA` CP, `+0xB` 5); the brief's
  "offset 0xA60" was `StockSkillList`'s.
- **Re:CoM's swing rows' columns** (`0x1407C6C80`, `+0xA`, read with pefile): row 1 → 0, row 2 → 0,
  row 3 → 1, row 4 → 2, row 5 → 1, row 6 → 0, row 7 → 2 (the owner's reading of 1–4 confirmed).
  Re:CoM's own ground chain alternates 1 → 3 → 1 … → 4 (`0x1401F7520`'s situation logic), the air
  chain 6 → 5 → 6 … → 7, so column 1 is Re:CoM's "second swing" column.

### What runs at Damage = Re:CoM (Combat = KH1, Sora's story)

- **Enemy HP**: the stats-init wrapper writes no KH1 HP (twins, proxies and boss twins), so
  `+0x274` keeps Re:CoM's row HP × difficulty and the max-HP getter `0x14017F5A0` answers Re:CoM's
  (no override). The KH1 records stay for what is not damage or HP: the KH1 level (MP charge,
  `0x1402BF330`), Stop's `resist[9]` and duration, Gravity's `resist[8]`, drops, lock radius.
- **Swings** (`win::attack::apply_row` → `win::damage::swing`, once per swing start, in battle
  only): after the row's kind is written, `+0x664` = `initAttack`'s bonus, then the card branch
  above with the **equipped Keyblade's card** (`save + 0x2674`, `win::stats::equipped_card`; 0 =
  Kingdom Key = card 1) at the row's column, then Re:CoM's own rewrite **`0x1401E66A0(player,
  &+0x660, &+0x668, attribute, status)`** (the function `initMagic` / `initStockSkill` /
  `exeStockSkill` call; the row pick inlines the same code). `onAttack` copies `+0x660 / +0x66C`
  into the weapon's attack; the enemy-hit wrapper **no longer replaces** the part's value. Re:CoM's
  path applies weak parts, the back bonus, its resistance for the card's attribute, boss falloff,
  the veto and `0x14017DAB0` unchanged.
- **In the field** (`player+0x390` = 0): Re:CoM's own field swing is cardless (`0x1401F0F70` writes
  card 0 → AP = attack stat), so KH1's field swings keep that.
- **Columns** (`damage::swing_column`; test `column_mapping`):

  | KH1 row | what | column | why |
  |---|---|---|---|
  | 13, 11, 14 | ground swing 1, swing 2, close swing 2 ("lunge") | 0 | owner: normal swings |
  | 8, 16 | ground / air finisher (kind 5) | 2 | owner: finishers |
  | 6, 7, 15 | Blitz, Ripple Drive, Hurricane Blast (kind 5) | 2 | owner: finishers |
  | 18, 17 | air swing 1, air swing 2 | 0, 1 | Re:CoM rows 6 / 5 they are built on |
  | 1, 2 | jump-up, hop | 1, 0 | Re:CoM rows 5 / 6 |
  | 0 | Aerial Sweep | 1 | Re:CoM row 5 |
  | 3 | Stun Impact | 2 | Re:CoM row 4 (finisher slot) |
| 4, 5 | Gravity Break, Zantetsuken | 2 at the swing's start; **at the hit KH1's % of Re:CoM's HP** | owner (follow-ups): below |
  | 9, 10 | Sliding Dash, Vortex | 0 | Re:CoM rows 1 / 2 |
  | 0x3E | Counterattack | 0 | built on Re:CoM row 1 |
  | 0x1B..0x21 | Sonic Blade | sleight | `recom-m9-abilities.md` § 9c.1 |
  | — | turned-back shots | the shot's own AP | unchanged: already Re:CoM's (decision 27) |

- **Spells / specials**: `recom-m7-magic.md` § Spell damage at Damage = Re:CoM;
  `recom-m9-abilities.md` § 9c.1 Sonic Blade at Damage = Re:CoM.
- **Per frame**: nothing new (one compare in the enemy-hit wrapper per hit).

### Gravity Break and Zantetsuken: KH1's % of Re:CoM's HP (owner, 2026-11-01, follow-ups)

> "Gravity Break and Zantetsuken take KH1's percentage of Re:CoM's HP" (instead of the Keyblade
> card's column 2)

- **KH1's rule** (`0x1402BFA80`, the % of HP element 8, `(+0x30 & 0x3C0) == 0x200`; Checked (KH1
  decomp)): power = the record's power `+0x14` + the stat term `+0x2C` (+ the attack's crit bonus,
  attack `+0x98`, on a crit); damage = `(int)((float)HP × ((float)power / 100) × resist8)` with **HP = the
  defender's current HP** (stats `+0x3C`; `+0x40` is the max, `recom-m8-items.md`), `resist8` =
  stats `+0x98`; **at least the record's floor** (`+0xC0`, 1 when it is under 2); **no Defence, no
  curve, no cap**, no Aero halving. Truncated toward zero.
- **The two records** (btltbl, Checked (files), `kh1_core::data`): Gravity Break `0x4F68` power
  **40 + Strength** (stat source 0), crit 200 with bonus 0 (a crit changes nothing); Zantetsuken
  `0x4FD8` power **250**, stat source 2 (no stat term), crit 200 bonus 0.
- **Bosses** (KH1's rule): the record's `resist[8]` scales it and the floor holds it up; no other
  boss rule in `0x1402BFA80`.
- **At Damage = Re:CoM** (`damage::percent_of_hp_row`, `attack::kh1_percent_of_hp`,
  `win::stats::recom_percent_hit`, in the enemy-hit wrapper): the swing starts as every swing
  (the card's column 2 AP and attribute, Re:CoM's rewrite); at the hit, the part's value
  becomes KH1's number with **HP = the enemy's Re:CoM current HP** `+0x274`, Sora's KH1 Strength
  (level + Keyblade, as at Damage = KH1) and the enemy's KH1 record (twin / proxy / boss twin)
  for `resist[8]` and the floor; no record: 100 %, floor 1. Re:CoM's path then applies its extras
  (weak part, back bonus, its resistance for the card's attribute, the frozen bonus
  `0x14015D940`) as for every KH1 hit (decision 13), **except the boss falloff `+0x290`** (owner,
  follow-ups 2: below). The part's `+0x92` is an i16: values over
  32767 are cut (Zantetsuken on more than 13106 HP left).
- Trace hits: `…:kh1pct <value>`; log `damage: swing row 4: KH1's % of Re:CoM's HP <hp> = <v>`.
- Tests: `attack::tests::percent_rows_on_recoms_hp`, `damage::tests::percent_rows_are_gravity_break_and_zantetsuken`.

#### Follow-ups 2 (owner, 2026-11-01): "skip Re:CoM's boss reduction for these two": the falloff (built)

0.59.0's notes called `0x14015D940` "the boss rule". **That label was wrong** (Checked (decomp +
exe + files), `recom-m7-magic.md` § Spell damage at Damage = Re:CoM):

- `0x14015D940` is the **frozen bonus**: `value += StockSkillList[0x23] column 1 × 10 / 100`
  (+100 with the session's `SY0002.BIN`), applied by the hit handler `0x14017F7E0` when
  `enemy+0x2B0` ≠ 0, which is the handle of the "ice" effect the **Freeze state** (`0x14017C190`)
  starts and its end clears. Any frozen enemy, boss or not.
- Re:CoM's **boss** reduction is the **falloff**: for ids 100..199 each hit the handler takes
  counts `+0x288`; from the 6th within 1500 ticks of the last, `+0x290` −= 0.25 (to 0.05), reset to
  1.0 by the enemy update `0x140185320`; `0x14017DAB0` multiplies every damage by `+0x290`.
  Bosses whose revenge acts (Stage 106) already have it off (their three fields are put back
  after each hit).

Owner's choice, as clarified (2026-11-01): "skip Re:CoM's boss reduction for these two" = the
**falloff** (option (a) of the question). Built (`damage::skips_falloff`, the enemy-hit wrapper,
`win::revenge::Falloff`, shared with Stage 106):

- at Damage = Re:CoM, a Gravity Break / Zantetsuken hit (with its KH1 % written) saves `+0x288` /
  `+0x28C` / `+0x290`, sets `+0x290` = 1.0 for Re:CoM's handler call, and puts the three back
  after: the hit neither takes nor feeds the falloff. Bosses are reduced by KH1's `resist[8]` and
  floor alone, plus Re:CoM's resistance for the card's attribute and its weak parts (kept);
- the frozen bonus (`0x14015D940`) stays;
- every other hit, and Damage = KH1: unchanged (the falloff as before; off on revenge bosses);
- log: `damage: swing row 4: Re:CoM's boss falloff skipped (its multiplier was 0.75)` when it
  would have reduced the hit. Test `damage::tests::only_percent_rows_skip_the_falloff_at_recom`.

### Sites (`sites::DAMAGE_CHECKED`, bytes re-read with pefile; checked at start)

| address | what | use |
|---|---|---|
| `0x1401E66A0` | enemy-card AP rewrite (16 bytes) | called |
| `0x1401F7ACC` | row pick card power (38 bytes, through the `0x4000000` test) | checked: the arithmetic repeated |
| `0x1401F0EEA` | `initAttack`'s bonus (20 bytes) | checked |
| `0x140160849` / `0x140160879` | `CCardBattle` stores of `0x14087A808` / `0x14087A810` | checked: the pointers read |
| `0x140219B27` | `exeStockSkill`'s Sonic Blade dash AP (38 bytes) | checked |
| `0x14017DD4C` | kind 0xD: the gate `param+0x40`, then current HP × AP (39 bytes) | checked: Re:CoM's Gravity relied on |
| `0x14017D626` | the reaction picker's case 0xD → flatten state `+0xAC0` (22 bytes) | checked: `win::magic::flattening` |
| `0x140183CFB` | the flatten exe's hold `+0x160 = 9000` (10 bytes) | checked |
| `0x14017DCB5` | kind 8: the gate `param+0x2C`, AP → `+0x2D8` | already `MAGIC_CHECKED` (7.9) |

### Logs and trace

- Start: `damage: damage = recom: …` (or `damage = kh1`); a mismatch of a site logs it and leaves
  the cardless AP.
- First 48 computations: `damage: swing row 13: Re:CoM card 0x1 column 0: power 100 x attack 10 /
  100 = AP 10 (attribute 0, status 0x4)` (`-> n (Re:CoM's enemy-card rewrite)` if it changed it).
- The trace's hits column: `e…:dmg <hp lost>:hp <left>:recom <ap> col <n> card <kind>` (spells:
  `… sleight 0xd`; Sonic Blade: `… col 0 sleight 0x9`). At Damage = KH1 it keeps `:kh1 <damage>`.
- Proxy lines end `[damage = recom: Re:CoM's HP stands; level and resistances used]`.

### Checked / not checked

- Checked (decomp + disasm + exe): the formula, the columns, the bonus, the tables' layout; tests
  for the columns, the AP arithmetic (truncation, bonus, ×1.5, status bits) and the setting's parse
  and default; the Windows build; all sites with pefile.
- Checked (KH1 decomp + files; tests): Gravity Break's and Zantetsuken's % rule and records.
- Not checked: in game; that `0x1401E66A0` leaves the AP alone at Combat = KH1 (it should: no
  enemy card of Sora's side; the log prints any change).
- Assumption: `player+0x390` ≠ 0 exactly in Re:CoM's battle room (the swing's card AP is used only
  then).

### The setting stays ini-only (owner, 2026-11-01)

Re:CoM's Config screen has 21 item slots and all are used (Combat 3, Guard reactions 2, Enemy
pace 2; `recom-config-menu.md` § Room for more rows). Owner's choice: "Damage" stays an **ini key
only**, `damage = recom | kh1` in `kh1-recom.ini`, read at start (default `recom`); no Config row.

### What the owner should test

1. No ini line (or `damage = recom`): the log says `damage = recom`. In a battle, swing at a
   Shadow with the Kingdom Key: the F9 trace's hits show `:recom <ap> col 0 card 0x1` for swings
   1 / 2 and `col 2` for the finisher; the HP left matches Re:CoM's (a Shadow takes as many hits as
   with Re:CoM's Kingdom Key cards).
2. Equip another Keyblade (KH1 > Items > Keyblade): the `card` in the trace follows it, and the
   AP follows that card's Re:CoM power (the `damage: swing row …` log lines print power and AP);
   it should hit like that Keyblade's attack card at Combat = Re:CoM.
3. Enemy HP: the HP bar / lock-on gauge show Re:CoM's HP (no `KH1 HP` in the log; proxy lines end
   with `[damage = recom …]`).
4. Bosses: every scripted HP threshold (e.g. Guard Armor's parts, Marluxia) behaves as at Combat =
   Re:CoM.
5. `damage = kh1`: everything as before this version (`:kh1 <damage>` in the trace).
6. Air combo, Counterattack, Sliding Dash / Blitz / Ripple Drive etc.: their `col` in the trace as
   in the table above.
7. Gravity Break (Re:CoM level 30, equipped) and Zantetsuken: the trace's hits show `:kh1pct <v>`
   and the log `damage: swing row 4: KH1's % of Re:CoM's HP <hp> = <v>`: Gravity Break takes
   (40 + KH1 Strength) % of the HP the enemy has left (e.g. Strength 51: 91 %), Zantetsuken 250 %
   (an ordinary Heartless dies); a boss takes its KH1 `resist[8]` share (e.g. 2 %), at least its
   floor, and **no** Re:CoM falloff: on a boss, hit it 6+ times quickly with swings (the log's
   `falloff skipped (its multiplier was …)`), then Gravity Break / Zantetsuken: full KH1 %.
   With `damage = kh1`: as before.

## Sora's KH1 Strength (decision 18: keep the Rod, give up the Shield)

All Checked (decomp + files) unless tagged.
- **New game** (`0x1402852C0`): each party record (`save + 4 + 0x74 × i`) is filled from btltbl
  file `0x39D8`, 23 bytes per character: HP, MP, AP, Strength, Defence, 16 resistances, 2 more.
  Sora: HP 18, MP 3, AP 3, **Strength 4**, Defence 3, all resistances 100.
- **Dive to the Heart** (`dh01.ard` script; event syscall 481 = `0x1401B03E0` → `0x140284B70`):
  adds btltbl `0x128 + 4 × choice` = (Strength, Defence, MP, AP) deltas. The script computes the
  choice from the kept item (var 256) and the given-up one (var 257): kept 0 → 0 / 1, kept 1 →
  2 / 3, kept 2 → 4 / 5. Rows: (+1, −2, −1, 0), (+1, −1, −1, −2), (−1, +1, −1, 0), (0, +1, −1, −2),
  (−2, 0, 0, 0), (0, −2, 0, 0). Assumption: items 0 / 1 / 2 are Sword / Shield / Rod (the deltas
  fit: keeping the Rod is the only choice without MP −1, giving up the Rod costs AP 2). So the
  owner's path, keep Rod and give up Shield, is row 5: Strength +0, Defence −2.
- **Level-ups** (`0x1402B51B0`): the bonus for reaching level L is btltbl `0x3AC0[L − 2]`
  (Sora's table, from `0x140528CC0`; the same table for every Dive choice). The choice only picks
  the EXP curve (`0x140528028[save + 0x48D]`) and the ability list (`0x140528D10[save + 0x48C]`).
  Bonus 4 = Strength +2 (`0x1402B5660`; 1 HP +3, 2 MP +1, 3 AP +2, 5 Defence +2, 6 / 7 slots).
- **In battle** (`0x1402A5130`): stats `+0x4C` = save Strength + the weapon's `+0x30` (Kingdom Key
  3) + accessories (none here).
- So Sora's KH1 Strength at Re:CoM level L = 4 + 0 + 3 + 2 × (number of 4s in the first L − 1
  entries of `0x3AC0`): 7 at level 1, 9 at 2, 13 at 10, 17 at 20, 27 at 50, 51 at 99. The asset
  step reads these from the player's btltbl.

## KH1 numbers for enemies without a twin (proxies; owner, 2026-10-31)

Owner: "it feels like any enemy that doesnt have a KH1 counterpart barely takes any damage in
comparison to the KH1 enemies, why is this and fix it".

### Why (Checked (trace): the owner's F9 traces of 2026-10-31, Sora at max level)

- The 31 twin classes took KH1's HP (Shadow 12, Darkball 60 in world 12) and KH1's damage with
  Sora's KH1 Strength 51: 300 per hit (the records' cap), so one hit.
- Every other enemy kept Re:CoM's per-floor HP (e.g. 225) and Re:CoM's damage. Without a card,
  Re:CoM's AP is Sora's Re:CoM attack stat `player+0x43C` (10 here; `0x1401F7520`, research
  `recom-motion-hits-damage.md` §3c–4). So about 20+ hits against one.
- The twins dying in one hit is **KH1-correct, not a bug**: it is KH1's own formula with a
  max-level Sora against KH1's first-visit (LV1) records (decision 21), as KH1 would give.

### Re:CoM's HP. Checked (decomp + exe)

- The stats init `0x14017FF50(enemy, 0xC, id, …)`: `enemy+0x308` = the enemy's `@ENP` record
  (resource type 0x15, id `0xC × 10000 + id`; file `ENEMY/<id>/EN<id>.EPD`, loader type 7 in the
  exe's resource lists); `enemy+0x310` = its row for the floor; `+0x274` = row `+0` × the
  difficulty % (`0x14052B4C8`: 75 / 100 / 100 / 100) / 100.
- The row is the **floor**, not the world: `0x1402337D0` = `movzx eax, word [0x14087B844]`;
  row = floor − 1 (0xE → 0, 0xF → 2, 100 → 0, ≤ 0 → 0, Riku's negative floors + 12).
  Sora picks the world of floors 2–10, so Re:CoM's HP in a world depends on its floor.
- `@ENP`: `+4` 1 in the file, `+8` row table offset (16-byte rows: HP, attack, prize count),
  `+0x48` / `+0x50` / `+0x94` other tables (`0x14017B4B0` relocates the four).
- Re:CoM's enemy ids (factory `0x140188F40`, ctor → vtable): 1–31 the Heartless (29 Tornado
  Step and 30 Loudness are the two ordinary classes without a twin), 37 / 38 100 Acre Wood's
  Beehive / Bee, 100–138 bosses and their parts, 201–209 Oogie's dice. Re:CoM's own boss rule is
  id ≥ 100 (`0x1401805A0`).

### The rule (`kh1-stats.json` format 5; asset step `stats::proxy_tables`, runtime `Kh1Stats::proxy`)

At Combat = KH1, in Sora's story, in the same stats-init wrapper as the twins (no new hook):

- **Ordinary enemy without a twin** (id < 100): KH1 HP = its own Re:CoM row HP (before the
  difficulty %, as the twins' KH1 HP ignores it) × the cell's ratio, rounded, at least 1.
  The cell for (world, floor row) is the **lower median, by KH1 HP ÷ Re:CoM HP on that row, of
  all ordinary twins** (fixed-damage records left out: White Mushroom 10 / 10, Black Fungus
  2 / 2; ties by class name). The proxy takes that twin's Defence, Strength, level, damage floor
  and cap, and 100 % for every element (KH1's ordinary Heartless).
  Assumption: "the twins of the same world" is all the twins, because Re:CoM's per-world rosters
  are not in the files read. KH1's records already differ by world only for the Shadow and the
  LV2 worlds (Twilight Town, Castle Oblivion).
- **Boss twins** (Re:CoM's bosses with a KH1 equivalent; class **and** id must match, because
  a Jafar-family part inits with Jafar's id 108). Each takes its own KH1 record, read from the
  boss model's AI script (part 5, as for the ordinary twins; Checked (files)):

  | Re:CoM (id) | Home world | KH1 model, LV1 / LV2 record | LV1: HP, Def, floor / cap, level |
  |---|---|---|---|
  | Guard Armor (102) | 1 Traverse Town | `xa_tw_3000` 0x14 / 0x14 (torso) | 700, 8, 2 / 30, 12 |
  | Leon, tutorial (101) | none (not pooled) | `xa_ex_1030` 0x14 (Traverse Town Leon) | 120, 8, 2 / 10, 10 |
  | Jafar (108) | 2 Agrabah | `xa_al_3020` 0x14 (Genie Jafar; Assumption) | 750, 15, 2 / 20, 29 |
  | Cloud (106) | 3 Olympus Coliseum | `xa_ex_1150` 0x14 / 0x80 | 300, 10, 2 / 20, 15 |
  | Trickmaster (105) | 4 Wonderland | `xa_aw_3000` 0x14 | 600, 9, 2 / 20, 15 |
  | Parasite Cage (110) | 5 Monstro | `xa_pi_3000` 0x14 / 0xEC (pi05) | 450, 17, 2 / 15, 30 |
  | Oogie Boogie (109) | 6 Halloween Town | `xa_nm_3000` 0x14 / 0x80 | 450, 20, 2 / 15, 38 |
  | Ursula (111) | 7 Atlantica | `xa_lm_3000` 0x14 / 0x80 (giant; Assumption) | 900, 19, 2 / 20, 36 |
  | Captain Hook (112) | 8 Neverland | `xa_pp_3000` 0x14 | 900, 21, 2 / 20, 42 |
  | Dragon Maleficent (113) | 9 Hollow Bastion | `xa_pc_3000` 0x14 | 1200, 24, 2 / 20, 48 |
  | Darkside (115) | 12 Destiny Islands | `xa_di_3000` 0x80 (di09) | 300, 6, 3 / 20, 0 |

  Hades (107) is not paired: KH1 fights him only in the Hades Cup (1800 HP, level 64).
- **Other bosses** (id ≥ 100 without their own record: Larxene, Riku / Riku Replica, Axel,
  Vexen, Marluxia's forms and parts, Lexaeus / Zexion if met, the hooded tutorial figure, Hades,
  Jafar's lamp and Iago, Oogie's dice…): KH1 HP = Re:CoM row HP × the world's boss cell. The
  cell is the world's home boss twin (KH1 HP ÷ its Re:CoM HP on that row), else the lower
  median of the pooled boss twins. The proxy takes that boss's Defence, level, floor / cap and
  resistances, with **KH1's final boss's Gravity and Stop** (decision 34; read from
  `xa_ew_3020` 0x14: Gravity 2 %, Stop 0 = immune; Checked (files)).
- Worlds 11 / 13 (no KH1 world) use the LV2 records of twins and boss twins (decision 21).
  World 0 (Castle Oblivion's floor halls) scales as world 13. Assumption: the world getter
  answers 0 there; the log shows the world each proxy used. A twin class met in world 0 (where
  its twin record doesn't apply) takes the world-13 ordinary proxy.
- Not proxied: Riku's story (decision 6), ids 37 / 38 (100 Acre Wood's minigame), enemies
  without a row or with 0 Re:CoM HP, and the Re:CoM setting.
- Hits: `kh1_hit` uses the twin's record, else the proxy's, so a proxy takes KH1's damage
  (`0x1402BFA80` through kh1-core) on the same path. Re:CoM's weak part, back bonus and boss
  falloff still apply (decision 13). The trace keeps `:kh1 <damage>`. HP bars, deaths and boss
  HP checks read `+0x274` and the max-HP getter `0x14017F5A0`, as for the twins.
- Lock radius: unchanged for non-twins (Re:CoM's).

### Logs (`kh1-recom.log`, once per class, world and floor)

- `stats: proxy <class> (id n) world w floor f: Re:CoM HP x -> KH1 HP y, Defence d, floor a
  cap b, level l, gravity g stop s, ratio r (from <twin>)`; `proxy boss` and `boss twin` lines
  are the same for bosses.
- `stats: check <twin class> floor f: Re:CoM row HP x, kh1-stats.json y (ok | MISMATCH …)`: the
  twin's live `@ENP` row against the asset step's reading. A mismatch means the `.EPD` reading
  is wrong, and every ratio with it.

### Not checked

- No `.EPD` was in this session. The `@ENP` reader follows the decomp. That the first `@ENP` in
  the file is Re:CoM's default record is an Assumption (strong); the `check` log line confirms
  it in game.
- The ratio table itself: `kh1-recom-build` prints it from the owner's files.
- Whether Re:CoM's Guard Armor (and other multi-part bosses) keep a separate part HP
  (`+0x260` bit 24 path): not read. The KH1 record gives the torso's 700.
- Scripted fights that compare an absolute HP value (not a fraction of the max HP): not
  searched (the same risk as for the twins).

## Swings and a moving locked target (checked 2026-10-30)

- KH1 takes the aim once, at the Attack command: `0x1402A7510` → `0x1402A38E0` → `0x1402A3B00`
  (the row's face-target flag). The other callers of `0x1402A3B00` are the movement think
  `0x1402A7B30` (runs only while `0x140297870` says Sora is free: flags `& 4`, `+0x88 & 0x20`,
  `+0x444 & 1` all clear), the party AI and summons. A swing keeps the yaw taken at the press, as
  kh1-core does. Checked (decomp). Assumption: a swing sets flag 4 (kh1-core's model).

## Guard, guard success and Counterattack (0.7.1; decision 20)

Research notes `kh1-guard-counter.md` (kept with the decomp tools). Checked (decomp + files)
unless tagged.

- **KH1.** Guard (row 0x3D, record `0x4DA8`) is a pure-guard attack object on the Keyblade
  (clash type 2, ±100°: cosine −0.1736), live on motion 212's hit track (frames 10–36). When it
  stops an enemy attack, `0x1402B4BC0` always gives Sora a recoil: kind 6 (`0x14029D720`) unless
  the enemy out-prioritises the Keyblade by more than 40 (kind 7).
  - Kind 6: request 0x6E at once (the guard motion and its object end), the idle queued, flag
    0x80 (no damage, no reaction) whose 8-frame countdown starts at the next action request or
    the lock release, CB+0x41 − 1, CB+0x50 |= 0x10, a pushback only when the enemy's priority
    is higher. Motion 0x6E (entry 72): 60 frames, no travel, free 28–60, combo window 28–60.
  - Counterattack (ability 19, `0x142D5EC10 & 4`): the menu's first slot becomes command 0x63
    (`0x140294220`) while the request is 0x6E, CB+0x50 holds the clash bit and a target exists;
    the accept (`0x140293480`) takes it from motion time 12. Command step `0x1402A7510` type 99:
    CB+0x41 + 1, row 0x3E (record `0x4E18`, motion 213: 86 frames, hit 8–24, combo window 18–56,
    travel 267.05), aimed at the target, else the stick, else the facing.
- **In Re:CoM (native).** Re:CoM's own body guard does the blocking: the hit apply
  `0x14016F1C0` returns 2 (guarded: no damage, no hit counter) when `body+0x9C & 1`, the hit's
  element bit is in `body+0x8C` (all by default) and the attacker is within `body+0xB0` of the
  body's facing, and it sets `body+0x9C |= 0x20`. Sora's body is `player+0x690`.
  - The KH1 handler sets `+0x72C |= 1` and `+0x740 = 1.7453` (100°) while the guard motion's
    hit track is live, and clears them (angle back to π) after.
  - The damage-check wrapper (`0x1401E6DE0`) catches `+0x72C & 0x20` before Re:CoM clears it
    (`0x1401F4000` / `0x1401FABC0` each frame) and hands it to the next KH1 frame, which runs the
    kind-6 success: KH1's request 0x6E (Re:CoM's bake `p001_081`, row 0x24), the lock, the combo
    counter −1, the clash bit, and flag 0x80 as Re:CoM's `+0x418 & 1` (the damage check skips
    hits) until 8 frames after the next action or the lock release.
  - The Counterattack is KH1's row 0x3E in our attack table, playing the asset step's KH1-only
    clip (`p001_226`) through a second appended action row. ini `counterattack = on | off`.
- **Differences from KH1, by decision 20:** Re:CoM attacks have no KH1 priority, so every
  attack Re:CoM's guard stops gives kind 6 with no pushback. Re:CoM's guard covers Sora's body,
  not the Keyblade's capsule, and measures the angle to the hit point. Re:CoM enemies'
  reactions to a block (their flinch, turned-back shots) are § Enemy reactions to Guard
  (decision 27), behind the Config row "Guard reactions".
- Assumption: leaving the recoil early in its free frames counts as the lock release for the
  8-frame countdown. Not traced in KH1.
- Note to re-check: Stage 9 found the roll's `0x140297F30(Sora, 12)` leaves damage on (no-flinch
  only); this research reads flag 0x80 as no damage too (`0x1402BEEE0`). The roll is unchanged.

### The Keyblade on Guard (0.7.4; owner: "when sora doesnt have his keyblade out and does guard, the keyblade should appear")

- **Re:CoM** (Checked, decomp + exe):
  - Sora's vtable (`0x1405A9938`) `+0xF8` = `0x1401F5D30` draws the Keyblade. It calls the
    weapon's `0x140225240(weapon, 1)`, which, only if the blade is put away (`weapon+0x154 & 1`),
    shows it and plays the `kb_appear` effect. Then it clears `player+0x418 & 2`.
  - `+0x100` = `0x1401F5480` puts it away (`weapon+0x154 |= 1`, `player+0x418 |= 2`).
  - The swing init `0x1401F0F70` ends by calling `+0xF8` with 1.
  - Re:CoM puts the blade away at state and action changes: leaving the Battle state
    (`0x1401EA310`, always), leaving the Field state (`0x1401EA3A0`, when bit variable 1 is set),
    carry `0x1401F1670`, the ledge climb `0x1401F2C20`, and others.
- **The mod:** the Guard start calls `0x1401F5D30(Sora, 1)`, as a swing does. Re:CoM's own rules
  put it away. Byte-checked callee.
- **KH1** (Assumption, from playing KH1; not checked in the decomp): Sora always holds the
  Keyblade, in towns too. Matching that at the KH1 setting is a scope question for the owner,
  not done.

### Guard sounds (0.7.2; owner: "Re:CoM's sounds now")

Research notes `guard-counter-sounds.md` (kept with the decomp tools).

- **KH1** (Checked, decomp + files; KH1 frames are 60 Hz motion frames):
  - Guard (212): weapon + 0xD at frame 0; voice group 19 (clip 24) at the row start.
  - Recoil 0x6E: sound `0x2AFD` at frame 0, steps at 8.09 and 59.26. That `0x2AFD` is the
    clink is an Assumption (strong).
  - Counterattack (213): weapon + 0xB at frame 0; finisher voice group 5 at the row start;
    steps at 46 and 80.
  - A guard success plays no voice. Its spark comes from the Keyblade's effect table.
- **Re:CoM, how motion sounds play** (Checked, decomp):
  - Sora's motion sounds are `PLAYER/0001/PL0001.SES`. Its entries bind to the motion pack's
    clips by name at load.
  - They start natively on every motion start (`0x1401A3470` → `0x140239260`).
  - Their frames are 60 Hz motion frames, so KH1 frames carry over 1:1.
- **What the mod does:**
  - The asset step writes `PL0001.SES`: Re:CoM's 50 entries byte for byte, plus two entries
    for the KH1-only clips.
  - Counterattack (`p001_226`):
    - whoosh 1237 at 0. This is Re:CoM's copy of KH1's weapon + 0xB (from the swing that
      shares it).
    - Re:CoM's finisher grunts at 0 (2010 / 2011 / 2012, its weights).
    - steps at 46 and 80.
  - Guard (`p001_225`):
    - whoosh 1236 at 0.
    - Re:CoM's swing grunts at 0 (2005 / 2006 / 2008).
    - These are the nearest Re:CoM sounds, not KH1's: Re:CoM has no clip with weapon + 0xD, and
      which Re:CoM voice is KH1's clip 24 is unknown.
  - Guard success: Re:CoM's own guard spark and its clink, `0x1401A6B20(Sora, "always", 0xB,
    hit point, 0, 0.0f)`, once, at the guarded hit point (`player+0x6C0`, kept by the
    damage-check wrapper). This is what Re:CoM plays when an enemy guards Sora's swing
    (`0x140182260`). The clink rides on the effect (the group's sound table), so the next motion
    doesn't cut it.
  - The recoil clip `p001_081` keeps Re:CoM's own sounds (one step at 8), because Re:CoM's own
    repel uses it.
- **Differences from KH1:**
  - The sounds are Re:CoM's, not KH1's.
  - Re:CoM fades a motion's sounds when the next motion starts. KH1 doesn't appear to
    (Assumption).
- **Not checked:**
  - The sound id of `always` #0xB: its sound table's file wasn't found.
  - That `always` is loaded wherever Sora guards (Assumption). If it isn't, `0x1402588B0` finds
    no group and does nothing (Checked, decomp).

## Enemy reactions to Guard (decision 27)

> Gimmick cards (owner, 2026-11-01, trigger (a)): the block detection in `win::guard::on_result`
> no longer needs the Guard reactions toggle; the toggle gates only the reactions. A block also
> calls `win::gimmick::on_guard_block` (`recom-gimmick-cards.md` § 4).

Owner (2026-10-31; completes decision 20): a new Config row "Guard reactions" (Off / On, **on by
default**, ini `guard_reactions`), with an effect only at Combat = KH1. When Sora's KH1 Guard
blocks an enemy attack, a **melee** attacker plays Re:CoM's own flinch (its card ends first) and a
**shot flies back and hurts enemies** with Re:CoM's damage; bosses and duels included. Research
notes: `guard-enemy-reactions.md` (kept with the decomp tools). Code: `crates/kh1-recom`
(`guard.rs`, `win/guard.rs`, the two wrappers in `win/attack.rs`). Not KH1 1:1 by the owner's
choice: KH1 plays a per-attack recoil Re:CoM has no data for (the flinch stands in), and KH1's
generic code makes a blocked shot inert (research §3), not harmful.

### Where it runs (no new hook)

- **The block.** Re:CoM's hit apply `0x14016F1C0` returns 2 when Sora's body guard stops the
  attack, after writing the attack's owner id into the body's `+0x98` (`player+0x728`), setting
  `+0x9C |= 0x20` and recording the attack's serial on the body (so this attack instance never
  tests Sora again). Checked (decomp).
- **Detected** in the existing pair-dispatch wrapper (`0x14016EAD0`): after Re:CoM's own
  dispatch, only when its result is non-zero. A block event is: result 2, the list is the player
  side's (`*0x14087A838`, the list `0x14016F110` picks when the shape flags `& 0xE != 0`),
  Combat = KH1, the row on, Sora's KH1 Guard live (our `set_guard`), Sora's body `+0x9C & 0x20`
  and `+0x98` = the collision's owner (`coll+0x20`). Checked (decomp `0x14016F110`,
  `0x14016EAD0`, `0x14016F1C0`). Cost otherwise: the compare of the result with 0.
- **The attacker** is `0x1402AD350(coll+0x20)` (the handle lookup). Checked (decomp).
  - **Shot:** its vtable is `CBullet` (`0x140525430`, 0x2F0 bytes) or `CHomingBullet`
    (`0x140525530`, 0x340 bytes) and the collision is the bullet's own (`bullet+0x2A8`). The
    bullet manager `0x140158DD0` makes only these two (spawn kind 1 / 2). Checked (decomp + exe
    RTTI).
  - **Melee:** the collision is one of the actor's `CAttackMotion` entries (`actor+0x100`,
    vtable `0x140523E70`; count `+0x14`, entries `+0x18`, 0x40 bytes each, live when `+0x18` is
    set, the collision at `+0x20`), the ones `0x14014E6A0` tests in the enemy update
    `0x140185630`. Checked (decomp + disasm `0x14014E6A0`, `0x1401A4E70`).
  - **Anything else** (the 16 boss classes that build their own `CCollAttack`, gimmicks): left
    exactly as Re:CoM does it (blocked, no reaction), logged once per class.
- The four vtables are checked by their RTTI names at start, and `CCollAttack` vtable `+0x20`
  (`0x140523E48`) must hold `0x14014E5A0`; otherwise the reactions stay off (logged).

### Melee: Re:CoM's own flinch

- In the pair-dispatch wrapper the attacker's id is only remembered (we are inside its own
  update, in `0x14014E6A0`). Several entries of one attacker blocked in one frame react once.
- In the existing enemy-hit wrapper (`0x14017F7E0`, whose only caller is `0x140185630`, right
  after the attack check), when that enemy's own hit handler has run and returned 0 (it wasn't
  hit itself this frame; if it was, Re:CoM's own reaction to that hit stands):
  1. skipped when HP (`+0x274`) ≤ 0, or `+0x260` bit 13 is set (Re:CoM's hit handler gives
     such an enemy no reaction either, it only plays the guard spark). Checked (decomp);
  2. **`0x14017D260(enemy, 1)`**, Re:CoM's reaction picker (its only caller is the hit handler):
     kind 1 (Sora's basic swing kind) takes its default branch to `0x14017D800`, which sets
     `+0x68 = enemy + 0x780` (state 11, the flinch) and clears `+0x260` bit 4, unless bit 19 or
     bit 21 (super armour) is set: then only Re:CoM's damage voice plays (`0x140181B50`).
     Checked (decomp). Each class's own flinch init (`+0x1F0` slot, bosses' face variants
     included) then runs when Re:CoM switches state;
  3. **the card ends** only when the picker chose the flinch: `0x14017E1D0(enemy)` (CBTL
     `resetNowPlaying` if it is the side-2 card now playing; no break, no counters), then
     `+0x260 &= ~1`. These are exactly the two steps of Re:CoM's own "interruptible card hit"
     path (`0x14017F7E0`, `+0x260 & 0x2000001`). That path itself can't be used: it runs only
     for a hit body, and a block hits nothing on the attacker. With super armour the attack goes
     on, so its card is kept (ending it would leave CBTL's now-playing out of step with an
     enemy still in its card attack). The attack state's own end (`0x14017DA90`) then clears
     the card bits and switches the hurt bodies back on, as for any interrupted attack.
     Checked (decomp).
- Not called: `0x14017E190` and vtable `+0x3E0` (the "I was hit" hooks that run before the
  picker on a hit), damage, notices. The blocked enemy still gets Re:CoM's "my attack
  connected" bit (`+0x260 |= 0x40`), as before.
- A remembered attacker that never runs the hit handler (a gimmick using `0x14014E6A0`
  directly) is dropped after 64 hit-handler calls.

### Shots: turned back with Re:CoM's own bullet runtime

At the block, once (the turned-back shot never tests the player side again):
1. **Re:CoM's list setter** `0x14014E5A0(coll, 0)` (`CCollAttack` vtable `+0x20`): `shape+100 =
   1`, so `0x14016F110` tests the shot against the enemies' list (`*0x14087A840`) from now on.
   Checked (decomp + exe). Within this same call `0x14016F110` re-reads the flags, so a shot
   that tested both lists (`& 0xE == 4`) stops after the player side's.
2. **Velocity reversed**: the mover `bullet+0xF0`, velocity `+0x80` (`0x1401D69F0` takes its
   length as the speed). Checked (decomp).
3. **Aim angles reversed**: mover `+0x50` (pitch), `+0x54` (yaw) set to the reversed velocity's
   angles in Re:CoM's own convention (`0x1401A2F10`: pitch = −atan2(dy, |dxz|), yaw = atan2(dx,
   dz)). Re:CoM rebuilds a bullet's velocity from these angles (`0x1401D5740`: forward
   `0x1407C5860` = (0, 0, 1), rotated by +0x50 / +0x54 / +0x58, times the speed `+0x23C`) in its
   homing ticks and in the bullet state's `+0xA0` (`0x1401561B0`), so without this a homing shot
   would turn straight back toward Sora. Checked (decomp + exe). Roll `+0x58` is left (logged).
4. **Homing at the shooter** (`bullet+0x238`, the spawning actor's id, written by the manager
   `0x140158DD0` for actor types 0x1000.., 0x2000.., 0x4000; 0 otherwise, then no retarget).
   Checked (decomp):
   - `CBullet` with flag `0x800` homes on the actor `+0x258` while `+0x25C` > 0
     (`0x1401573C0`): `+0x258` = shooter;
   - `CHomingBullet` homes on the actor `+0x2F4` (`0x1401575D0`, and `0x1401563F0` with flag
     `0x2000`): `+0x2F4` = shooter when it was > 0 or flag `0x2000` is set; with flag `0x80000`
     it steers to the point `+0x300` instead: the shooter's position (mover `+0x30`) goes there.
   - **Correction to the research** (§1.2 / §4.3 named `+0x32C`): `+0x32C` (spawn `+0x98`) is
     only read by `CHomingBullet`'s floor-roll steer (flag `0x400000`, `0x1401561D0`), not the
     homing. Checked (decomp). And `+0x32C` does not exist on a `CBullet` (0x2F0 bytes).
5. **That one test answers 0** (the wrapper returns 0 instead of 2), so the bullet's step
   (`0x140156A30` / `0x140156E20`) takes its "no hit" branch: no hit effect, no `+0x298`, no end.
   Sora's body still has `+0x9C & 0x20`, so his KH1 guard recoil runs as before. Checked
   (decomp).

From then on Re:CoM moves, draws, collides and ends the shot (its range from its spawn point
`+0xA0` / `+0x240`, its lifetime, map contact). It can hit any enemy body, its shooter included
(the collision's owner is the bullet, not the shooter). A hit takes Re:CoM's own enemy hit path
with the shot's own kind, element and AP (`coll+0x44 / +0x50 / +0x48`), the enemy's
resistances and the reaction its kind gives (Re:CoM's damage, by the owner's choice; the KH1
numbers of decisions 12 / 21 apply only to KH1 swings).

### Logs (`kh1-recom.log`)

- `guard reactions: block melee <class> id=… coll=… kind=… elem=… ap=… serial=… flags=… state=…`
  and, when its hit handler runs, `<class> id=…: picker(1) -> flinch (state 11) | voice only …
  flags A->B state S next N->M card ended | bit cleared | kept | none` (or why it was skipped).
- `guard reactions: block shot CBullet|CHomingBullet id=… shooter=… (<class>) … flags=… list
  A->B vel (…)->(…) angles (…)->(…) roll … homing …`.
- `guard reactions: turned-back shot id=… met id=… (<class>) result r`, then `turned-back shot
  id=… hit <class> id=…: damage d hp a->b` (with a note when a card play stopped the damage).
- First 50 block lines and first 50 hit lines; "no reaction" classes once each.

### Addresses (all Checked against the Steam exe's bytes and the decomp)

| address | what | first bytes |
|---|---|---|
| `0x14017D260` | enemy reaction picker (called) | `40 53 48 83 EC 20 4C 8B 81 60 02 00 00 48 8B D9` |
| `0x14017E1D0` | enemy card end (called) | `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20 48 8B F9` |
| `0x14014E5A0` | `CCollAttack` list setter (called; the whole function) | `48 8B 41 18 48 85 C0 74 06 83 CA 01 89 50 64 C3` |
| `0x1402AD350` | handle → object (called; already checked) | `48 83 EC 28 8B D1 48 8B 0D 33 9D 84 00 48 85 C9` |
| `0x14087A838` / `0x14087A840` | player side's / enemies' collision list | data |
| `0x140523E28` / `+0x20` | `CCollAttack` vtable / its setter slot | RTTI + pointer, at start |
| `0x140523E70` | `CAttackMotion` vtable | RTTI, at start |
| `0x140525430` / `0x140525530` | `CBullet` / `CHomingBullet` vtables | RTTI, at start |

### Risks

- **Bosses and duels** (included, owner's choice): every blocked melee hit can now end a boss's
  card (as decision 14 does on a hit). Sleight chains, duel mode and story bosses that wait for
  their card are the main risk. Super armour (bits 19 / 21) keeps the card and plays only the
  voice.
- **A turned-back shot meeting a card-playing enemy deals no damage**: Re:CoM's hit handler
  returns before damage while `+0x260` bit 0 is set (unless bit 25), and the shooter is often
  still in its card attack when its shot comes back. The log notes it. Whether a turned-back
  shot should break into the card like a KH1 swing (decision 14's route) is an open question
  for the owner.
- **Damage scale**: the shot's Re:CoM AP against KH1 HP on twins (decision 12) may be large or
  small next to Sora's KH1 hits. Not measured.
- **Order in the frame** (Assumption): our `+0x68` write comes after the rush / charge states'
  own "connected" handling only if their state exe runs before `+0x78`; the log's `next` shows
  what was set at the time.
- **State 11's knockback** reads the enemy's last hit body's attacker (`info+0x50` → `+0x98`),
  which after a block is stale (an earlier hitter, or a dead bullet: then Sora's position). The
  direction may be off (Assumption).
- **Turned-back shots** may hit gimmicks or objects whose bodies are on the enemies' list.
- A homing `CBullet` with flag bit 29 re-picks the nearest flagged object (`0x1401CDB90`) once
  its shooter is gone; it can no longer hit Sora (enemies' list only).
- The other results of the same dispatch call (other player-side bodies hit by that shot in
  the same test) keep their own hits; only the answer to the bullet changes.

### Not verified

- In game: nothing yet.
- That a bullet's model faces its mover angles (Assumption): if not, a turned-back shot flies
  backwards-facing.
- Roll `+0x58` is 0 on Re:CoM's shots (Assumption; the log prints it). If not, a homing shot's
  rebuilt velocity is off by the roll.
- The spawn-time order of the bullet state's `+0xA0` (velocity from angles) vs a direct
  velocity write: reversing both covers either.
- KH1: the per-attack recoil data (`+0x40` of KH1's enemy attack data) is not read (the flinch is
  a stand-in by decision 27).

## MP charge (M7 stage 7.1)

- KH1 charges `record +0x2C` × (1 + (enemy level − Sora's level) × 0.2): 3 / 6 / 24 per hit, at
  most 250 %, at least 1 (`0x1402BF330`). Point k of MP needs 30·k units (`0x1402A4CB0`). An enemy
  of level 0 charges nothing.
- Built in M7 stage 7.1: the enemy-hit wrapper charges KH1's MP for each KH1 hit, with the twin's
  level; see `recom-m7-mp.md`.

## The M4 follow-ups (decision 17)

- **High Jump** plays Re:CoM's own higher jump: rows 0x14 / 0x15 (`p001_028` / `p001_029`). Only
  Guard is appended to Sora's action table now (row 0x47). Checked (tests).
- **Root motion one update late.** Our wrapper on `0x14024DCA0(ctrl, out, out yaw)` (the entity
  task's root step, `0x1401A59A0`) delays Sora's root travel by one update, only in our KH1 actions
  (avoid 6: roll / Guard; attack `0xA`). Re:CoM's own actions are left alone.
  - **`out` is an absolute position, not a travel** (fixed after 0.6.0's in-game bug):
    - `ctrl+0xA0` = the model's position `ctrl+0xF0` + this update's rotated root travel
      (`0x14024C960`, the anchor copied in `0x14024CCC0`);
    - the entity task sets the body to `P + mask∘(out − P)` (`0x14023E6E0` a − b, `0x14023E630`
      a ∘ b, `0x14023E0D0` a + b, mask at actor `+0x140`). Checked (decomp).
  - 0.6.0 stored and returned it as a travel, so on a swing's or roll's first update (nothing
    pending) it returned (0, 0, 0) and Sora was moved to near the room's origin. The owner's traces
    show it: idle at (−416.35, −750.35, 1363.55), then (10.6, −597.0, −34.7) on the Attack press,
    then walk-off and a fall.
  - Now: travel = `out − anchor` is delayed, and the hook returns `anchor + last travel`. That is
    Re:CoM's own position with only the travel term changed, never more than 0.5 m from Re:CoM's
    own result (a safety net: otherwise Re:CoM's stands).
  - The hook passes through when the state exes aren't counting frames (trace hooks missing).
  - Test `swing_and_roll_away_from_the_origin_keep_soras_position` uses a non-origin start, a
    model offset and Re:CoM's absolute root step: it fails with 0.6.0's logic (the same jump to
    near the origin) and passes now, for the swing and the roll.
  - `debug_attack = on` in the ini logs the anchor / root / output for the first 200 lines.
  - KH1 applies an update's travel in the next physics step, also on the frame a new motion
    starts, so the travel carries across a chain (`RootDelay`, keyed by Sora's frame count from
    the state exes).
  - The roll and the swings now match kh1-core exactly. Checked (tests).
  - Assumption: Re:CoM's clip-start update leaves `ctrl+0xA0` at zero.
- **Roll protection.** KH1's roll sets only the no-flinch timer: `0x1402C9E60` → `0x140297F30(12.0)`
  sets flag `0x80`. Its untouchable frames don't exist:
  - the hit pass needs `flags & 0x8048 == 0x8000` (`0x140292070`);
  - flag `0x40` is set and cleared only by actor-command ops `0x2B` / `0x2A` (`0x14028E6E0`'s queue);
  - those ops are queued only by event-script opcodes (`0x1401C8820`, `0x1401C88C0`,
    `0x1401C8B00`, `0x1401C8B60`, `0x1401C8C00`, `0x1401C8C60`), always next to `0x24` / `0x23`;
  - the roll motion's tracks hold only types 0 (free from 30), 4 and 6, and the mset's third
    section is a 48-byte stub.
  - Checked (decomp + files). This corrects KH2-Combat-in-KH1's `kh1-intangible.md`, which calls
    them "motion events".
  - So at KH1, Sora takes damage during the roll, with no hit reaction for 12 frames.
  - Since 2026-11-02 this is only `dodge_roll = kh1`. The default `recom` runs Re:CoM's own roll
    and its own untouchable bit (`recom-kh1-movement.md` § Re:CoM's roll).
  - Re:CoM's untouchable bit `+0x418 & 1` is no longer set. Our wrapper on the damage check
    `0x1401E6DE0` sets `+0x420 & 2` for that call only. The check reads `+0x420` once at its start,
    and after applying the damage takes its own no-reaction exit (`(+0x420 & 0x8010002) ≠ 0` →
    return 0). Death still goes through.
  - Checked (decomp): the bit's only other readers are the room-card regen ticks (`0x1401600D0`,
    `0x1401601C0`), which don't run inside the call.

## Post-hit invulnerability (0.13.1; decision 31)

Owner: "if he gets hit he has a period of time where he cant be hit again to prevent getting
combo locked".

### KH1. Checked (decomp)

- Every damage reaction (`0x14029CC40`, `CD80`, `CE10`, `CEC0`, `CFD0`, `D110`, `D210`, `D340`,
  `D450`) calls `0x140297F30(Sora, 30.0)`: Sora flag `0x80` (no damage, no reaction) and timer
  `+0xC` = 30, and it clears `0x100000`.
- Sora's update `0x140297FE0`: while `flags & 0x1000080 == 0x80`, the frame the lock (flag 4)
  is off sets `0x100000` without counting; from the next frame the timer loses the frame step
  `DAT_14233FBDC` (1 at 60 fps) and both bits clear at ≤ 0. So the 30 frames start when the
  reaction releases Sora.
- The guard recoils (`0x14029D510`, `D5F0`, `D650`) use 8.0; that is the guard's own timer
  (0.7.1), unchanged.

### Re:CoM. Checked (decomp)

- Without it, Re:CoM's damage check `0x1401E6DE0` takes hits during its hit reaction (action
  `0xC`) and right after it, so Sora can be juggled.
- `player+0x7BC` is Re:CoM's own hit-immunity timer: the damage check returns before any damage
  while it is above 0 (`+0x722` cleared, as for the roll bit `+0x418 & 1`), the Battle / Field
  exes (`0x1401F9840`, `0x1401FA0F0`, and the third exe at `0x140210000`) subtract dt after the
  action step, and the revive sets 3000. No other reader.
- The damage check is the only writer of action `0xC` (an 8-byte write that also zeroes the time
  in action `+0x410`) and runs every frame of it (the exes skip it only in `0x17`, `0x1C`, `0x1D`
  and `0x20`).
- dt: 50 units per 60 fps frame (100 at 30 fps); 30 KH1 frames = 1500 units.

### Ours (no new hook; the damage-check wrapper)

- After Re:CoM's check, at Combat = KH1 for the main Sora: a reaction that began in this call
  (action `0xC`, time 0, not dead) is armed. On each frame of an armed reaction, `+0x7BC` is
  held at `max(+0x7BC, 1500 + dt)` (`attack::post_hit_hold`). The exe subtracts dt that frame, so
  when the reaction ends Re:CoM counts exactly 1500 units = 30 frames itself. Leaving `0xC`
  disarms it.
- No reaction (the no-flinch frames, super armour, card breaks), no immunity, as in KH1.
- Re:CoM setting: nothing written.
- Test: `post_hit_hold_leaves_thirty_frames` (Re:CoM's check-then-subtract order, 60 and 30 fps).
- Assumption: Re:CoM leaving `0xC` stands for KH1's lock release. Re:CoM's reaction clips are its
  own, so the time spent inside the reaction is Re:CoM's; only the 30 frames after it are KH1's.
- Assumption: within one frame KH1 checks hits after Sora's update; if it checks before, KH1's
  window is one frame longer. Not traced.
- Not seen in game.

## The held reaction (M11, the Neoshadow's grab; owner 2026-11-02)

Owner (2026-11-02, `docs/design-philosophy-instructions.md` "Neoshadow grab / stuck turn"): the grab as a new Re:CoM
player reaction ("held") playing KH1's held clip as a new Sora motion slot, freed by a hit. Code: `crate::held` (the
rules), `win::held` (the kind table, the case, the entry), the damage-check wrapper (`attack::damage_check`). Not
seen in game; no KH1 trace.

### KH1. Checked (files + decomp)

- `ex_2420.bd` `0x1AF6` (the hidden Neoshadow, two left): when its grab test `0x3FAA` passes (`sys 141` =
  `0x140291920`: Sora can be hit, flags `& 0x8048 == 0x8000` and none of `0x1000080`, so not in his post-hit / no-flinch
  frames; `sys 218` = `0x1402BB4B0`: his movement state not an air / swim one; his height 0; his flag `0x400` clear,
  not identified), it puts Sora into request 200 of its own `.mset` player set: `sys 113` (`0x1402BA8C0`: the set
  `*(table + 4 + n × 4)` by the target's character number), `sys 13(Sora, 0x10000C8, 4, 0)` (the word at `0x1CD4`,
  the blend 4.0 at `0x1CDA`; bit 24 = KH1's action lock, `0x14029F980` sets actor flag 4), the lock's release
  `0x40000000` queued (`0x1CEC`, blend 8).
- The clip: `xa_ex_2420.mset` section 3 (`+0xA4B00`, 10 player sets; entry 0 at `+0xA4B80` is the only one with its own
  set, the nine others share `+0xB3B80`), request 200: **126 frames, no root motion**. Checked (files). That Sora's
  character number is 0: Assumption (strong).
- No damage: the grab makes no attack row. A damaging hit's reaction (its own motion request) replaces 200, and the
  Neoshadow lets go when Sora's motion is no longer 200 (`0x1E06`). No button escape. KH1's reactions set the
  post-hit frames (`0x140297F30(30)`); the grab doesn't: **a held Sora can be hit**, and the hit that frees him gives
  him KH1's 30 frames after its reaction, as any hit.

### Re:CoM. Checked (decomp + exe)

- An enemy reaches Sora only through a hit. An `.ASD` record's `+0x68` indexes the kind table `0x14052A220` (21
  entries: 1, 2, 3, 36, 37, 4, 5, 26, 7, 38, 6, 10, 39, 40, 8, 41, 28, 29, 30, 42, 32; Checked (exe)) through the getter
  `0x140179410` (`movsxd rax,[rcx+0x68]; lea rcx,[0x14052A220]; mov eax,[rcx+rax*4]; ret`; the table's only reader,
  Checked (decomp)); the attack set passes it through `0x14017F0E0` to the attack body's `+0x44`, the hit copies it to
  Sora's body `+0x80` = `player+0x710`, read only by the damage check `0x1401E6DE0`.
- The damage check's status kinds (8, 0xC, 0x11, 0x1B, 0x20) set a bit of `+0x420` and enter the status action `0x18`
  (`0x1401F1200`); with no bit set, `0x1401F1200` takes sub-state `0xC` and plays row 0x28 (the get-up). The status exe
  `0x1401EB0F0`'s case `0xC` waits for the clip's end (`0x1401A57D0`) and then stands Sora up (`0x1401F3180(pl, 0)`);
  it reads no button. The Battle / Field exes run the damage check every frame of action `0x18`; a hit there takes
  Re:CoM's reaction (action `0xC`) unless `+0x420 & 0x8010002`, and the reaction's end goes back to the status only
  for `+0x420 & 0x8810023`.
- Re:CoM's Stop kind (8) takes no HP in the check (`local_b0 != 8`): a kind that does no damage is the check's own
  rule.

### Ours

| Part | Where | What | Tag |
|---|---|---|---|
| The kind | `win::held::install` | Re:CoM's 21 kinds copied next to the exe (`hook::place_data_near`) with the held kind `0x40` at index 21; the getter's `lea` displacement at `0x140179417` (old `05 0E 3B 00`) points at the copy. Re:CoM's records read the same 21 kinds. Installed when the held clip's row exists, at both Combat settings (inert at Re:CoM: only the KH1 Neoshadow's grab record names index 21) | Checked (exe: the getter's 15 bytes, `sites::THROWN_CALLEES`) |
| The case | `win::held::on_damage` in the damage-check wrapper | a pending hit of kind `0x40`: its value set to 0 (no damage), Re:CoM's check run with its no-reaction bit `+0x420 & 2` (its gates first: dead, `+0x418 & 1`, `+0x7BC`, `+0x7B4`, Sora's guard `+0x72C & 8`: each returns with the hit pending); when it cleared the pending word `+0x720` and Sora lives, the held entry; else nothing. Not KH1 Sora, no held clip, already in a status, the no-flinch frames: no damage, no reaction | Checked (decomp) |
| The entry | `win::held::enter` | Re:CoM's status entry as the check makes it: `0x1401F6990(pl, 6)` (its reaction prelude), `+0x408` = action, action `0xC` (8 bytes), `+0x414` = 0, `+0x64C` = 0, `+0x7B4` = 0; then `0x1401F1200`'s no-bit branch: `+0x408` = `0xC`, action `0x18` (8 bytes), velocity 0 (`0x1401A38D0`'s writes), `+0x414` = `0xC`, the held clip's row (`0x1401F7180`, KH1's blend 4, no root motion), `+0x428 &= ~8`. Left out: the damage face `0x1401F8AC0` (no damage) and Re:CoM's rumble `0x140211E40(2)` (KH1's grab rumble is not ported) | Checked (decomp); the face: Assumption |
| The hold | Re:CoM | its status exe plays the clip to its end (127 samples = KH1's 126 frames at 60 fps) and stands him up; any damaging hit's reaction ends it for good; no button; nothing of ours runs | Checked (decomp) |
| The row | `rows::Extension::held`, `movement::extend_action_table` | the held clip's row appended after the KH1-only and xl rows (`motions::parse_held`: the manifest's `held` section; the pack must hold the slot) | Checked (tests) |
| The enemy's view | `win::held::hold` → `kh1_core::enemy::Hold` | grabbable: KH1 Sora, the row ready, alive, not `+0x418 & 1`, `+0x7BC <= 0`, `+0x7B4 == 0`, not the no-flinch frames, his KH1 Guard down, on the floor, not in action `0x18`; held: action `0x18`, sub-state `0xC`, row `+0x474` = the held row. Read only for a behaviour that asks (the Neoshadow), each frame of its AI | Checked (decomp) for the fields; Guard down: Assumption (KH1's flag `0x400` not identified) |

- **Post-hit invulnerability** (decision 31, `+0x7BC`): the held entry sets none (KH1's grab sets no flag `0x80`), so the
  dive's hit lands on a held Sora and frees him; its reaction (action `0xC`) arms KH1's 30 frames as any hit. A Sora in
  his post-hit frames is not grabbable (KH1's `sys 141`), and Re:CoM's check would refuse the grab's hit anyway.
- **At Combat = Re:CoM** nothing changes: the host says "not grabbable", the Neoshadow takes KH1's miss (209), and a
  held-kind hit on Re:CoM's Sora (none is made) would do nothing.
- Tests: `held::tests` (kind table, the case, the grab test, "taken"), `rows::tests::the_held_row_follows_the_others`,
  `motions::tests::held_section`, `kh1_core::neoshadow::tests` (the grab, a hit freeing him, a grab that never lands),
  the asset tests on the player's files (`validate::i_held_clip_from_the_neoshadows_player_set`,
  `neoshadow::tests::builds_from_the_players_files`).
- Assumptions: Re:CoM's message `0x14017F0E0` sends with the kind (to manager 0x16) leaves `0x40` as it is (no card
  rewrites it in a KH1 battle); a held-kind hit on a friend card's Donald / Goofy would take their switch's default
  (the grab's record is aimed at Sora's spot); the hold starts one or two frames after KH1's (Re:CoM's hit order: the
  record lands on 208's first frames, the check runs in Sora's update); Sora's face during the hold stays as it was.

## Hooks and patches (`sites::ATTACK_HOOKS`, `ATTACK_CALLEES`, `ATTACK_DATA`)

All bytes were compared with the Steam exe (54 sites, script check). Hooks are all or nothing,
after the data patches, which are also all or nothing.

| address | what | first bytes (moved) |
|---|---|---|
| `0x1401F8150` | request bits | `48 89 5C 24 10 57 48 83 EC 20` |
| `0x1401E8FF0` | use-card button | `40 53 48 83 EC 20` |
| `0x1401F0D90` | initAttack | `48 89 5C 24 08 48 89 6C 24 10` |
| `0x1401F0F70` | field swing init | `48 89 5C 24 08 48 89 74 24 10` |
| `0x1401EAEB0` | field swing exe | `48 89 5C 24 08 48 89 74 24 10` |
| `0x14016EAD0` | attack pair dispatch | `48 89 5C 24 18 48 89 6C 24 20` |
| `0x14017F7E0` | enemy hit handler | `48 89 5C 24 20 55 56 41 56` |
| `0x1401E6DE0` | player damage check | `40 55 57 48 8D 6C 24 B1` |
| `0x14024DCA0` | root motion step | `0F 28 81 A0 00 00 00` |

Data (checked old values):

- `0x1405A9A60` (`CPlayerSora` vtable `+0x128`): `0x1401F7520` → our row pick. Compared after
  relocation.
- `0x1401FA074` (Battle jump table, id `0xA`): RVA `0x1F9CC9` → `0x1FA6C1`.
- `0x1401FA6C1`: 13 × `CC` → the stub.
- `0x140179417` (M11, the held reaction): the kind getter's `lea` displacement `05 0E 3B 00` (→ `0x14052A220`) → our
  22-kind copy next to the exe (§ The held reaction). Its callee `0x1401F6990` is `sites::HELD_CALLEES`.
- Re:CoM's rows 1–7 must equal `attack::RECOM_ROWS_1_7`.

Called: `0x1401F6660` (end attack), `0x140224FB0` (weapon attack end), `0x1402ABB20` (bone),
`0x1401777E0`, `0x140176FA0`, `0x1401F7180`, `0x1402AD350`; `0x1401E80C0` (springs); and for the
Guard reactions `0x14017D260`, `0x14017E1D0`, `0x14014E5A0` (§ Enemy reactions to Guard).

Per frame, our code runs only as cheap tests outside a KH1 swing: the request bits, the damage
check and the root step (a pointer compare), and the pair dispatch only while a weapon volume is
live.

## Trace (F9)

New cells in the existing schema:

- `row` (KH1 row), `combo_counter`, `combo_max` / `air_combo_max`, `combo_master`;
- `hit_active` (Re:CoM's weapon volume live), `swing_hits` (Re:CoM's `+0x644`);
- `hits` (each KH1 hit: enemy, damage, HP after, `:card`);
- `events`: `AttackStarted { row, request, counter, finisher }`, `AttackRefused(…)`, and Re:CoM's
  `attack_id` while in action `0xA`.

## Debug log

`debug_attack = on` in `kh1-recom.ini` writes one line per decision to `kh1-recom.log` (the first
200 lines): `dispatch` (Re:CoM's dispatch reached the field-swing init), `start` (row, Re:CoM id,
counter, Sora's position, the target, lock-on), `exe` (action, motion frame, body velocity,
contact, hit) and `root` (anchor, Re:CoM's root position, our output).

## In game, 0.6.1 (owner's traces 7 and 8, 2026-10-30)

- The swing / guard / roll position jump is gone: no frame-to-frame jump over 20 units in
  either trace. Checked (trace).
- "Sora is missing a lot of his swings": 11 presses gave no swing. Each was refused by one of
  KH1's own rules (`kh1-sora-data.md`):
  - 8 × `NotAccepted`: the press came before the combo window opened (13–19 frames into a
    swing whose window is 22–40 / 26–42 / 26–54) or during the finisher, which has no window.
    KH1 keeps no press for later: `0x140293480` drops a command while Sora can't act. Checked
    (decomp). The owner pressed about every 15 frames, so about half the presses fell early.
  - 3 × `LastSwingMissed`: pressed inside the window after a swing that hit nothing. KH1 ends
    the combo there unless Combo Master is equipped. Checked (decomp).
  - The windows match KH1's files frame for frame (row 13: hit 14–23, window 22–39 in the
    trace, against 14–24 / 22–40). Checked (trace + files).
- Why the swings missed, from the trace: the locked enemy walked out of range during the swing
  (frame 112, about 190–230 units away). In frame 166 it circled Sora at about 80 units while
  Sora's facing stayed on the yaw taken at the press, 50–70° away from it. Whether KH1 keeps
  turning a ground swing toward a moving locked target after the press is not checked yet
  (the row exec's face-target flag, `0x1402A38E0`). Assumption: it doesn't.

## Assumptions to check in game

- The weapon's collision runs after Sora's motion update in the frame (the hit-window lead).
- `player+0x4B0` is Sora's centre (the first sweep).
- Re:CoM's target aim point stands for KH1's body-part centre, radius 0.
- The KH1 swing's Re:CoM clips have KH1's root travel (they are bakes of KH1's motions).
- An enemy's hit-handler parts are the dispatch's body nodes.
- `ctrl+0xA0` is zero on a clip's first update.

### Turned-back shots break into the card (0.11.2, decision 28)

- Owner's log on 0.11.0 (Checked): every turned-back shot met its shooter (Blue Rhapsody, Red
  Nocturne, Creeper Plant), and every time it did 0 damage, logged "card playing: Re:CoM's hit
  handler took no hit". The shooter was still in its card attack (`+0x260 & 1`), and Re:CoM's hit
  handler `0x14017F7E0` returns before damage then.
- Change: when the hit the handler is about to take comes from one of our turned-back shots, the
  enemy-hit wrapper sets `0x2000000` for that call, as it does for a KH1 swing (decision 14).
  Re:CoM's own path then ends the card and takes the hit with the shot's Re:CoM damage.
  - How a turned-back hit is recognised: the enemy's hit info `+0xF8` has its hit flag, and the
    hit body's attacker is a shot we turned back.
- Not seen in game.

