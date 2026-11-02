# Design philosophy

A Rust mod (a DLL) that brings Kingdom Hearts 1's combat (Final Mix, PC, Steam build) into
Kingdom Hearts Re:Chain of Memories (PC, Steam build).

Every setting has a Config-menu toggle between **Re:CoM** and **KH1**.

- With every setting on KH1, the game must feel 1:1 with KH1.
- With every setting on Re:CoM, it must play exactly like Re:CoM.

These rules override any habit of the assistant.

## Native always

Re:CoM's own systems own Sora. Native means giving Re:CoM's engine more abilities so it can do
everything KH1 can, and so KH1 runs 1:1 inside Re:CoM. We add to Re:CoM's own systems:

- new states in its player state table;
- new commands in its command/card dispatcher, including commands that need no card;
- new rows in its attack, hit and damage tables;
- new motion slots;
- new effect programs;
- new sound banks;
- new camera modes;
- new HUD elements drawn by Re:CoM's own HUD code;
- new functions or wrappers patched into Re:CoM's own functions.

Re:CoM then runs the KH1 feature itself: it steps it, draws it, plays it, collides it and ends it.

### Not native

Any KH1 logic of ours that runs beside Re:CoM's and copies its results into Re:CoM's fields is
not native, even if it never takes Sora over. Examples:

- our own KH1 player controller driving Sora;
- fake cards slipped into the deck so KH1 attacks "look like" card plays;
- our own hit sweeps or damage maths;
- our own magic or summon runtime;
- our own HUD or renderer.

That code is "running along" or "deprecated", and every change should move it into Re:CoM's
systems. If something can only run along for now, say so and ask before shipping it that way.
Never offer hybrid versions, and never ask which version to build.

## Build what Re:CoM lacks

If KH1 has a feature Re:CoM doesn't, build it into Re:CoM's engine without asking each time.
Examples:

- MP and the magic menu;
- the command menu with free Attack;
- items and quick slots;
- persistent party members with AI;
- AP and abilities;
- Guard;
- combo finishers;
- KH1 summons;
- KH1's level-up stats.

Ask only about open design choices, and before choosing between native designs that change how
the feature plays.

## 1:1 target

With a setting on KH1, every value matches KH1's exe exactly: timings, frame data, momentum,
speeds, windows, damage, MP costs, camera. No approximating.

- KH1's numbers run inside Re:CoM's engine.
- Confirm both games' tick and animation rates in their decomps before converting frame counts.
  Don't assume they differ, or that they match.
- A feature is 1:1 when the tester's trace matches KH1 frame for frame, or the KH1 decomp
  confirms the numbers. Say which you checked and what you couldn't.
- If a real native attempt still can't match KH1, stop and ask. Say what differs, why, and what
  the native options are.

## Re:CoM stays whole

- Nothing Re:CoM does may break or change because of our work. That includes the features KH1
  never had: cards, card breaks, sleights, Reload, stocking, enemy cards, world/room cards,
  Moogle shops and Reverse/Rebirth.
- Replaced systems must still give Re:CoM's events, scripts, bosses and room logic everything
  they read. Bosses and scripted fights that check card values are the main risk here.
- KH1's and Re:CoM's systems share the same native code wherever they can.
- When a choice strongly affects how the game feels, it becomes a Config toggle, not a silent
  change for everyone. Card breaks against KH1-style attacks are an example.

## Little of ours per frame

- As little of our code as possible runs every frame.
- Prefer data that Re:CoM's own code reads (rows, commands, motion slots, attack data, effect
  programs, sound banks) over per-frame hooks that steer it.
- Measure before proposing a performance fix. Don't trade native integration for a small speed
  win without asking.

## Assets

A build step makes them once, from the player's own extracted KH1 and Re:CoM files, into the
mod folder for Re:CoM to load as its own. Nothing from the games goes in the repo. Plan each
step with the tester first.

## Scope is open-ended

Look in KH1 and Re:CoM for related mechanics nobody mentioned. Examples: KH1's MP regen rules,
combo-ender knockback, Trinity, Donald's and Goofy's AI settings, the lock-on camera, the Scan HP
bar. When you find one, say what it is, where it is, and whether it matters for 1:1 feel, then
ask before adding it.

## Decided by the owner (2026-10-29)

> "the goal is to remove attack cards entirely, and have the card battles be replaced entirely by
> the KH1 combat."

At KH1 setting:

- **Sora in the field** moves and attacks like KH1 Sora, natively, with all his abilities. He is
  not a passthrough.
- **Battle transitions stay Re:CoM's.** Attacking a Heartless in the field, or walking into one,
  puts Sora into the battle arena with it, as in Re:CoM.
- **In the arena, no cards are used.** Sora fights as he does in KH1.
- **Enemies with a KH1 equivalent** (Shadow, for example) act, attack and animate exactly 1:1 as
  in KH1, natively inside Re:CoM's enemy systems.
- The card-value question (2026-10-29) is settled: with no cards in battle, there are no card
  breaks against KH1 attacks.

### Answers (owner, 2026-10-29)

1. Magic, summons and items are replaced by their KH1 equivalents. Those with no equivalent are
   removed for now and listed in `docs/card-replacement.md` for later.
2. Re:CoM-only enemies and bosses: keep their Re:CoM behaviour but without card plays, attacking
   on their own timing (option a), for now.
3. First strike: yes, a KH1 field hit keeps Re:CoM's stun / first-strike bonus into the arena.
4. Party: Donald and Goofy don't appear, for now.
5. Progression: Re:CoM's level-up choices, for now. To be revisited.
6. Riku's story (Reverse/Rebirth) stays as it is, for now.
7. Sleights: those with a KH1 equivalent become KH1's version (magic sleights like Fira and
   Firaga become KH1's magic levels; physical ones like Sonic Blade, Strike Raid, Ars Arcanum
   and Ragnarok become KH1 abilities). The rest are removed for now and noted.
8. Enemy cards: removed for now and noted.
9. Keyblade: Sora starts with Kingdom Key, using KH1's stats.
10. Animations (2026-10-30): "i wouldn't bother having an option to switch between KH1 and RECOM
    animations because for all intents and purposes they're the same, just use RECOM's default
    animations, and KH1 animations for anything that isnt included in ReCOM". Re:CoM's own clips
    (KH1 motions baked at 30 fps) play wherever Re:CoM has the motion. Only KH1 motions Re:CoM
    lacks are converted. There's no animation switch. Timing (hit windows, combo windows, travel)
    stays KH1's own data either way.
11. "Included in Re:CoM" means the same kind of move (owner, 2026-10-30, option (a)): where Re:CoM
    has the move (walk, run, jump, fall, land…), Sora uses Re:CoM's animation for it even when it
    isn't a bake of KH1's exact motion. KH1's converted clips are only for moves Re:CoM lacks
    (High Jump, Hurricane Blast, Counterattack, Ripple Drive…). Speeds and timing stay KH1's.
    The function-by-function mapping (KH1 request → Re:CoM action row) is built in M4.
12. Damage to Re:CoM's own enemies at KH1 setting (owner, 2026-10-30): KH1's formula with each
    enemy's KH1 twin's stats (HP, Defence…). This needs KH1's enemy stat tables (not found yet).
13. Re:CoM's damage extras (back-attack bonus, weak points, boss falloff after 5 hits) stay on for
    KH1 hits (owner: "Keep them").
14. Card priority at KH1 setting: KH1's rule. Sora's hits land even while an enemy plays a card.
15. The card shown over an attacking enemy is hidden at KH1 setting.
16. Ledges at KH1 setting (owner, 2026-10-30, option b): Re:CoM's own ledge climb and hang stay as
    they are.
17. (Lead, under decision 11) High Jump uses Re:CoM's own higher-jump animation rows (0x14/0x15).
    The Dodge Roll's one-frame root-motion lead is fixed natively by delaying Re:CoM's root-motion
    step one frame for Sora. Roll protection uses KH1's exact frames: 12 no-flinch frames plus the
    untouchable window from the roll motion's events, not Re:CoM's full invulnerability.
    (Stage 9 finding: KH1's roll has no untouchable window; protection is the 12 no-flinch frames.)
18. Sora's KH1 Strength (owner, 2026-10-30, option a): KH1's level-1 Sora plus the Kingdom Key,
    growing with Re:CoM's level-ups. Path: keep the Rod, give up the Shield (owner, 2026-10-30).
19. Work order after 0.6.3 (owner, 2026-10-30): finish M5 (KH1 damage numbers, Guard's repel and
    Counterattack, the swing-turning check), then M6 (battles without cards); M1's KH1
    recordings when the owner has time.
20. Guard against Re:CoM enemies (owner, 2026-10-30): Sora's side is KH1's (no damage, KH1's
    recoil 0x6E, Counterattack from frame 12). Re:CoM enemies also get new reactions when Sora
    blocks them (Re:CoM's own flinch for the attacker, blocked shots turned back), behind a
    Config toggle. Re:CoM attacks have no KH1 priority: all are blockable with KH1's normal
    recoil for Sora (kind 6, no pushback).
21. Which KH1 record a Re:CoM enemy uses (owner, 2026-10-30): the record of the Re:CoM floor's
    world as on a first KH1 visit (LV1), e.g. an Agrabah Shadow uses KH1's Agrabah Shadow.
    Worlds KH1 doesn't have (Twilight Town, Castle Oblivion) use KH1's LV2 record. HP, Defence,
    damage floor / cap, level and resistances come from that record (decision 12).

22. Sounds for KH1-only moves (owner, 2026-10-30): Re:CoM's own sounds for now ("Re:CoM's
    sounds now"). KH1's own sounds, through a sound bank built from the player's files, can come
    later.
23. Guard's live window stays KH1's frames 10–36 (owner, 2026-10-31, moved on after the trace
    showed the unblocked hits fell outside it).
24. Hurt shapes (owner, 2026-10-31): KH1 hurt shapes on twins (option b of the hitbox plan) are
    skipped for now. Re:CoM's Shadow is a 0.55 m sphere, wider than KH1's 0.35 m ellipsoids.
25. Keyblades at the KH1 setting (owner, 2026-10-31, option a):
    - Each Keyblade card Sora has earned unlocks that Keyblade as a KH1-style item.
    - It is equipped from a **new top-level camp menu entry**, shown only at the KH1 setting and
      drawn by Re:CoM's own menu code. Keyblade abilities are not included.
    - The 16 Keyblades KH1 has use KH1's weapon rows and KH1's models.
    - (Card names corrected 2026-10-31: the six are Star Seeker, Total Eclipse, Midnight Roar,
      Maverick Flare, Two Become One and Bond of Flame. KH2 has Star Seeker, Two Become One and
      Bond of Flame; the other three exist in neither game and keep KH2 placeholders for now:
      owner, "for right now the placeholder models are fine".)
    - Star Seeker, Monochrome, Follow the Wind, Hidden Dragon, Photon Debugger and Bond of Flame
      come from **KH2** (the player's KH2 files): KH2's model, Strength and Magic. Reach is
      measured from the KH2 model's blade length; radius, critical rate / bonus and priority are
      the Kingdom Key's.
    - Unlock rule: **ever earned** (Re:CoM's discovery state `0x14015B3F0`, normal or premium).
      The Kingdom Key is always unlocked.
    - **Soul Eater is not for Sora** (owner, 2026-10-31): Re:CoM never gives Sora its card
      (Riku-only, `0x1401592D0`), so it stays out of his list.
    - The entry is **hidden in Riku's story**.
    - The equipped Keyblade is one byte in Re:CoM's save block (`save + 0x2674`, a range no code
      uses; `docs/recom-keyblade-equip.md`).
    - Order: owning and equipping, then KH1 models, then KH2 models and stats.
    - Models (owner, 2026-10-31): the equipped Keyblade shows **as soon as the camp menu
      closes** (Sora's weapon rebuilt with Re:CoM's own constructor); **only the equipped one is
      loaded**; Halloween Town shows the equipped Keyblade like every other world; no look-only
      test first, build the full swap. Native route: new resource groups for the converted models
      in Re:CoM's player resource table, picked when the weapon is built
      (`docs/recom-keyblade-models.md`).

26. Hits on attacking enemies (owner, 2026-10-31): at the KH1 setting, enemies stay hittable
    while they attack, as in KH1: Re:CoM's "hurt bodies off" at an attack state's start
    (`0x14017DA80`) is skipped, for every attacker (option "Enemies stay hittable", not KH1
    swings only), **bosses included**.

27. Enemy reactions to Guard (owner, 2026-10-31; completes decision 20), behind a new Config row
    "Guard reactions", **on by default**, only at the KH1 setting:
    - a blocked **melee** attacker plays **Re:CoM's own flinch** (state 11 via `0x14017D260(enemy,
      1)`; its card ends first). KH1 plays a per-attack recoil that Re:CoM has no data for, so
      the flinch is a stand-in;
    - a blocked **shot flies back and hurts enemies** with Re:CoM's damage (not KH1: KH1's
      blocked shots change side and never hit again);
    - **bosses and duels included**.
    Research: `guard-enemy-reactions.md` (kept with the decomp tools).

28. Turned-back shots (owner, 2026-10-31): a shot Sora's Guard sends back must do damage
    ("it sends it back but doesnt do any damage. make that do damage").
29. M6, battles at the KH1 setting (owner, 2026-10-31, "go ahead"):
    - **Re:CoM's separate battle rooms stay** (field contact → battle instance, as Re:CoM).
    - **No deck in battle at KH1**: no card drawing, no card UI. Sora still collects cards (drops,
      rewards, shops), so switching back to Re:CoM combat works.
    - **Enemies don't play cards at KH1.**
    - **Several enemies can attack at the same time, exactly as in KH1** (no card-battle turn
      taking).
    - Re:CoM's events, scripts and bosses must still get every value they read (the main risk).

30. M6 details (owner, 2026-10-31), research `m6-cardless-battles.md` (kept with the decomp tools):
    - **Crowd pace: each enemy attacks as if alone (KH1)**. Re:CoM's think-rate scaling by enemy
      count (`0x14017F380`) is off at KH1, as are its other three serialisers (CBTL's single
      enemy slot, the think gate `0x14017D010`, one grant per frame in `0x140187600`).
    - **Human bosses keep their decks as a hidden move list**: same attacks, reload pauses and
      sleights as Re:CoM, no cards shown.
    - **Marluxia 3's stock folder is hidden** at KH1.
    - Staging: (1) Heartless attack without cards, several at once; (2) no deck / deck HUD for
      Sora; (3) bosses one at a time, starting with the "Sora is attacking" check (action 0xB)
      that 12 bosses read.

31. Post-hit invulnerability (owner, 2026-10-31): "Lets also carry over sora's KH1 mechanic
    where if he gets hit he has a period of time where he cant be hit again to prevent getting
    combo locked". At Combat = KH1: KH1's 30 frames from the end of each hit reaction, on Re:CoM's
    own hit-immunity timer `player+0x7BC` (`recom-kh1-attack.md` § Post-hit invulnerability).

32. Enemy pace (owner, 2026-10-31, "lets try b for now"): enemies felt too aggressive at KH1. KH1
    has no shared attack limiter as far as checked (Shadow's script and the 26 common syscalls;
    the rest Assumption), but Re:CoM's enemies are tuned for its crowd sharing. Setting
    `enemy_pace = recom | kh1` (default `recom`): `recom` keeps Re:CoM's crowd sharing (P4 off),
    `kh1` is decision 30's "each attacks as if alone". Interim, until enemies get their KH1
    twins' timing (M10). Config row "Enemy pace" (0.13.4): the owner chose (a), Re:CoM's row 4
    style (one wide value) for Guard reactions and Enemy pace, so all three rows fit in Re:CoM's
    21 item slots (`recom-config-menu.md`).

33. Marluxia 3's card duel (owner, 2026-10-31, "c"): at KH1 the attack that starts Re:CoM's card
    duel (Sora's reaction `0x21`) is a plain hit, so the duel never starts (Sora has no cards to
    duel with). `recom-m6-battles.md` § Marluxia 3's card duel.

34. M7 magic (owner, 2026-10-31, "lets work on M7 ... FULLY implemented, also including a shortcut
    menu, customize menu, MP bar"; answers):
    - **Spell visuals and sounds: Re:CoM's** own effects and sounds.
    - **Max MP grows on level up.**
    - **Spells and spell levels are earned from certain bosses / events.**
    - **Cure heals exactly as Re:CoM's Cure; Aero works as KH1's.**
    - **Stop and Gravity against bosses exactly as in KH1.**
    - **MP bar: Re:CoM-style straight bar** under Re:CoM's HP bar, in KH1 blue (built like Riku's
      Dark gauge).
    - **Command menu: Re:CoM's own menu pieces** (buttons, cursor, text), laid out like KH1's.
    - **Command menu shown in the field and in battle**, as in KH1.
    - **Customize: one "KH1" camp entry** (our entry renamed) with a submenu: Keyblades,
      Customize (later Abilities, Items).
    - **Spell unlocks: each spell's base level when Sora first gets that Re:CoM magic card; the
      -ra / -ga levels at Re:CoM bosses.**
    - **Gravity and Stop on Re:CoM-only bosses** (Larxene, Riku Replica, Axel, Vexen, Marluxia,
      the Riku fights): **KH1's final-boss values** (Gravity 2 %, immune to Stop).
    - Cure heals Re:CoM's amounts: Cure 50, Cura 150, Curaga 300 HP (`SY0002.BIN`
      BattleCardList / StockSkillList), with KH1's cast rules.
    - **MP bar grows with max MP** (KH1's 26 units per MP); colour **read from the player's KH1
      files** at the asset step.
    - **Command menu cursor: plate colour** (the highlighted row uses another colour of Re:CoM's
      command plate); row spacing KH1's 22 units.
    - **Menu words: KH1's own text** from the player's KH1 message files.
    - MP rule: the KH1 exe's (the k-th MP needs 30·k refill units; Fire / Blizzard cost 30 units).
    - Shortcuts saved per save (KH1).
    - Research: `m7-magic.md`, `m7-menus-mp.md` (kept with the decomp tools).

35. M7 MP details (owner, 2026-10-31): **MP refills to full wherever Re:CoM refills Sora's HP**
    (its HP-refill state step, its checkpoint heal, the level-up HP choice); **the six KH2
    Keyblades use their KH2 Magic stat as their max-MP bonus**. `recom-m7-mp.md`.

36. M7 spells (owner, 2026-10-31): **Fire re-targets as KH1** (each frame, a ball with no live
    target turns to the best enemy within 5 m: a wrapper on Re:CoM's homing step for our balls);
    **spell hits use KH1's element resistances only** (no Re:CoM % on top).

37. M7 (owner, 2026-10-31): **Fire re-picks exactly as KH1** (only balls cast with no target; a
    dead target means the ball flies straight); **Thunder's 50 % world dim is left out**.

38. M7 spell details (owner, 2026-10-31): **Fire's release step is KH1's 15 cm curve** (replacing
    Re:CoM's clip travel in the root-step wrapper); **Gravity slows everything about an enemy under
    the falling sphere** (its whole frame step to KH1's factor; research first); **Stop's held hits
    land at KH1's pace** (Re:CoM's 8-hit cap kept).

39. KH1 HUD (owner, 2026-10-31): "Would we be able to instead have an option where the Re:CoM
    [HUD] is replaced entirely by the KH1 hud? ... use the KH1 decomp to get exact positioning
    and graphics". Supersedes decision 34's HUD look for that option:
    - **A switch "HUD: Re:CoM / KH1", in the "KH1" camp entry** (stage 7.13).
    - At KH1 HUD: **KH1's own elements with KH1's graphics and exact positions** from the KH1
      decomp: the HP/MP portrait ring (bottom-right), the command menu with its COMMAND header
      (Attack / Magic / Items / Summon), the lock-on target HP bar, and **Donald's and Goofy's
      gauges top-right, always shown**.
    - **Re:CoM's HUD components are hidden** for now (minimap, room plate, EXP popup, Moogle
      points, FRIENDS row, Re:CoM's HP gauge). **Remind the owner** once the KH1 elements are right
      to place the Re:CoM extras (minimap, room plate, …): they will give exact places.
    - Details (owner, 2026-10-31): **HP ring: KH1's rule on Re:CoM's raw HP** (5° per HP to
      270°, then 26-unit segments per 6 HP); **Donald / Goofy gauges hidden until they join**
      (shown while Re:CoM has them out); **menu words in KH1's font converted into a Re:CoM
      font**; **command menu colour: KH1's field look in the field, the battle look in battle
      rooms**; **Re:CoM's enemy gauges hidden and KH1's lock-on cursor used** (Re:CoM's level-up
      choice kept); **low-HP sound: Re:CoM's**. Research `kh1-hud.md` (kept with the decomp tools);
      plan H0 (read-only dump of KH1's 24 `exchange/*_uibin_*.bin` layouts and HD textures on the
      owner's PC) then H1–H8, one element per version.
    - Hit shake / size-class slide (2026-11-01, owner away: "keep going"): built the native way,
      **baked into every piece's animation data at the asset step** (Re:CoM plays it; nothing of
      ours per frame). Owner to confirm when testing.

- **M7–M9 choices (owner, 2026-11-01: "Finish everything up to M9"):**
    - **Shortcuts (7.12):** L1 as in KH1. Small choices taken as KH1's (Cure / Aero from a
      shortcut cast on Sora, slots filled as spells are learned, L1 + Triangle at a save point
      casts, KH1's panel art).
    - **Items (M8a):** sold in Re:CoM's Moogle shop **and** dropped by enemies (KH1's rates).
      **Healing amounts: Re:CoM's own item-card amounts** (not KH1's 30 / 60 HP).
    - **Summons (M8b):** each unlocks with its first Re:CoM summon card. **Wait for the party:**
      summons are built after persistent Donald and Goofy (KH1's summon needs both and sends them
      away), so M8b follows the party milestone.
    - **Abilities (M9):** unlock with Re:CoM (its level-up sleights: Sliding Dash, Blitz, Stun
      Impact, Zantetsuken, Strike Raid, Sonic Blade, Ars Arcanum, Ragnarok); the five with no
      Re:CoM source (Vortex, Aerial Sweep, Hurricane Blast, Ripple Drive, Gravity Break) at
      **KH1's learn levels** on Re:CoM's level; an **equip screen with KH1's AP** (KH1's costs,
      AP +2 at KH1's levels, on Re:CoM's level).
    - **M9 follow-ups (owner, 2026-11-01):** Gravity Break (no KH1 learn level) unlocks at **Re:CoM
      level 30**; the learn list is **KH1's Rod list**; with the equip screen, a newly unlocked ability
      arrives **unequipped**, as in KH1.
    - **9b follow-ups (owner, 2026-11-01):** **Guard from the start** (still costing AP); High Jump,
      Glide and Superglide unlock **where Re:CoM unlocks them**; **Dodge Roll equipped from the start**;
      **Combo Master at Re:CoM level 50**; KH1's Rod-path **max MP +1 at levels 30 and 66** added; the
      Abilities page rows use Re:CoM's **blue** box.
    - **Later the same day:** **Guard and Dodge Roll are part of Sora's base movement kit from the
      start, not abilities** (no AP, not on the Abilities page); High Jump / Glide / Superglide are
      **equipped automatically when Re:CoM gives them** (still removable). Specials (9c): **Re:CoM's
      sleight effects** until the KH1-effects work; KH1's blue scene tint **skipped for now**.
    - **Spell effects (owner, 2026-11-01):** "lets look at how KH1 handles these FX and bring them in
      here 1:1": KH1's own spell effects (same effect engine as Re:CoM's) are ported natively, behind
      **their own toggle** (KH1 or Re:CoM spell effects); **Re:CoM's spell sounds are kept**; built
      **after the M9 specials**, Fire first as the pilot.
    - **Sonic Blade (owner, 2026-11-01, on 0.56.1's questions):** the dashes' voices "however it works
      in base ReCoM"; the per-dash effects "however it works in base ReCoM"; usable **only in battle**
      (not in the field rooms). Whether the other specials follow the same rule: ask at 9c.2.
    - **Damage and enemy HP (owner, 2026-11-01):** "depending on which keyblade sora has equipped, it
      does recom's damage for the corresponding keyblade … lets also have Recom's enemy HP". Re:CoM's
      formula (attack stat × the equipped Keyblade card's power ÷ 100) for every KH1 swing: **KH1's
      normal swings take Re:CoM's normal column (0), KH1's combo finisher Re:CoM's finisher column
      (2)**; Re:CoM's enemy HP; **spells and specials do Re:CoM's card / sleight damage** (each spell its
      Re:CoM magic card's, Sonic Blade Re:CoM's Sonic Blade sleight's). A Config setting (Damage: Re:CoM
      / KH1), Re:CoM by default; KH1 keeps decisions 12 / 21. Supersedes decision 12 at the default.
      Sonic Blade also gets **Re:CoM's start tint** (as base Re:CoM's).
    - **Damage follow-ups (owner, 2026-11-01, on 0.58.1's questions):** the Damage setting stays
      **ini only** (`damage = recom | kh1`); at Damage = Re:CoM, **Gravity and Stop use Re:CoM's own
      numbers** (Gravity / Gravira / Graviga 20 / 40 / 70 % of current HP with Re:CoM's immunities and
      flatten; Stop 9000 / 15000 / 21000 ticks); **Aero's wind uses Re:CoM's Aero by level** (card,
      then Aerora / Aeroga sleights, as Fire / Blizzard / Thunder); **Gravity Break and Zantetsuken
      take KH1's percentage of Re:CoM's HP**.
    - **Damage follow-ups 2 (owner, 2026-11-01, on 0.59.0's questions):** at Damage = Re:CoM, **Aero
      level 1 gets a wind carrying the Aero card's value** (not KH1, owner's choice); stopped enemies keep
      KH1's held-hit pace and attacks-off; **Gravity Break and Zantetsuken skip Re:CoM's boss falloff
      (`+0x290`), the "boss rule" of the question** ("skip Re:CoM's boss reduction for these two": KH1's
      `resist[8]` and floor alone reduce bosses; the frozen bonus `0x14015D940`, which the question
      mislabelled, stays); Aero's wind in the field keeps Re:CoM's no-card AP.
    - **Boss revenge value (owner, 2026-11-01):** "lets also have humanoid bosses have a revenge value
      system, similar to how they work in their fights in KH2 … borrow their revenge value". Chosen:
      KH2's **story / Absent Silhouette limits** (Axel 92, Larxene 100, Vexen 150, Marluxia 100 for all
      three forms, Hades 80 then 50 below 30 % HP, Cloud 100); **all humanoid bosses** (also Hook,
      Riku, Riku Replica, the hooded figure: KH2's default 100); on revenge the boss plays **its own
      Re:CoM attack nearest its KH2 revenge move** (Larxene: her leap, as KH2's decoy teleport); **Re:CoM's
      boss damage falloff is off for bosses that have revenge**. KH1 hits add KH2 Sora's ATKP revenge
      values (swing 8, finisher 24, air swing 12, air finisher 28, Fire 8, Blizzard 16, Thunder 32).
      Notes: `docs/recom-boss-revenge.md`.
    - **Revenge moves untouchable (owner, 2026-11-02):** "the humanoid bosses that have revenge value
      can't be damaged or interrupted during their retaliation move": while a boss plays the attack its
      revenge started, Sora's hits do no damage and don't interrupt it (Re:CoM's own rule for an attacking
      enemy, without decisions 14 / 26's bypass for that attack).
      Then (2026-11-02, on 0.65.0's questions): **Hades untouchable only during his wind-up** (his spin
      can be hit; his fire-up still doesn't flinch); the revenge check at **KH2's frame 20 of the flinch**
      (Hades 30), or at the flinch's end if shorter (0.65.0 checked on its first frame: a bug).
      **Cloud's revenge move: his homing charge** (state 4, `e106_053`, 2 charges above half HP, 4 below;
      nearest KH2's Sonic Rave), instead of Omnislash.
      Then (2026-11-02): "you can still hit bosses during their revenge move (or if they use their revenge
      move naturally instead of it being triggered by revenge value)": a bug to fix, and **the boss's
      revenge-move attack is untouchable whenever it plays it**, by revenge or by its own choice.
    - **Dodge roll (owner, 2026-11-02):** "make dodge roll work how it does in ReCoM, not KH1" — **the
      whole roll** (Re:CoM's motion, speed, distance, timing and invincibility) at Combat = KH1. Supersedes
      decision 17's KH1 roll frames; an ini key keeps KH1's roll selectable.
    - **Enemy card sounds (owner, 2026-11-02):** "you cant hear enemies stocking cards or refilling their
      deck": at Combat = KH1, the sounds of enemies stocking cards and reloading their deck are silent.
      Then (on 0.65.2's questions): the enemies' first deal at battle start **stays silent**; a human
      boss's item-card page switch (sound 3) is **silenced too**.
    - **Phase 4, KH1 enemies (owner, 2026-11-02):** "lets move on to doing phase 4, except, i only want
      the AI, attacks, reactions and motions from KH1 equivalent enemies, everything else (emphasis on
      stats) stay the same as they are in RECoM". KH1-equivalent enemies take **KH1's AI, attacks
      (timing, hit shapes, behaviour), reactions and motions**; **stats (HP, damage, defence, drops,
      EXP…) and everything else stay Re:CoM's** (the Damage setting still decides Sora's damage to
      them). M10 Shadow pilot first, then M11.
      M10 choices (2026-11-02): native states (KH1's behaviour ported as new Re:CoM states; the
      interpreter would run along: not offered); `enemy_ai = kh1 | recom` ini (default kh1); KH1's
      motions incl. rise and death (Re:CoM's prizes / EXP); KH1's reactions (launch weight as KH1's;
      flinch part from the hit's height, Re:CoM's hurt shapes); KH1's recoil when Sora guards; Re:CoM's
      attack power per attack; Re:CoM's Stop / freeze / Gravity states; **battle enemies only** (field
      Shadows keep Re:CoM's roaming, owner).
      Then (2026-11-02, on 0.67.2's questions): a sunk Shadow **stays lockable** (Re:CoM's one lockable
      flag; KH1's lock button can lock it, its auto-target can't) and **keeps pushing Sora** while it sinks
      and rises (KH1 lets him walk through): both left as they are.
    - **M11 (owner, 2026-11-02):** "it works perfect. Now let's bring in the remaining KH1 adjacent
      heartless": the 30 other ordinary Heartless with a KH1 twin, under M10's rules (KH1 AI / attacks /
      reactions / motions, Re:CoM stats, battle only). Step 1: a shared framework from the Shadow's
      code, proven on the Soldier; then the families.
      Soldier choices (2026-11-02, on 0.68.0's questions): power mapping kept (kicks / double kick / rush /
      air kick on Re:CoM's kick, jumping kick / rush leap / spin kick on its spin kick); **a guarded attack
      whose KH1 recoil motion the model lacks plays Re:CoM's flinch**; battle spawns keep KH1's appearance;
      KH1's wander picks points **around where the enemy stands** (fix the Shadow to it); KH1's leash
      left out (would make a battle unwinnable).
    - **Summons now (owner, 2026-11-01):** "Lets get summons working. I want the summon section on the
      command menu to work like KH1, but each summon should do their lv 3 recom summon equivalent, cloud
      included" (supersedes "wait for the party"). KH1's Summon list (Magic list's last entry, learn
      order, KH1's greying) issues **Re:CoM's LV3 sleight** of each summon (same card ×3: Proud Roar /
      Showtime / Paradise / Splash / Twinkle / Flare Breath LV3; **Cloud: Omnislash**), started natively
      without a card. Chosen: **MP cost from Re:CoM's CP** (2 MP for the 20-CP cards Simba..Tinker Bell, 3
      for the 25-CP Mushu and Cloud); **KH1's party rule skipped until the party exists**; **Re:CoM's
      summon damage at both Damage settings**; **KH1's once-per-battle rule kept** (row locked while one
      is out, no Dismiss); battle rooms only; each unlocks with its first Re:CoM summon card.
    - **Summons follow-up (owner, 2026-11-02):** "summons should be able to be used as long as you have
      MP, also, sora should have iframes through all summon animations". KH1's once-per-battle rule is
      **dropped** (usable whenever Sora has the MP); **Sora is invincible through every summon's whole
      animation** (start to Re:CoM's end action); "cloud should also be at the top of the summons list"
      (**Cloud first**, then the others in learn order).
    - **Gimmick card triggers (owner, 2026-11-02):** "interrupting an attack with a finisher is very rare";
      chose **(a) + (d)**: (a) when Sora's KH1 Guard blocks an attack of one of the five card-break bosses
      (Guard Armor, Trickmaster, Jafar, Parasite Cage, Dragon Maleficent), the boss gets Re:CoM's card-break
      stagger and its own drop roll; (d) each of those bosses also drops its gimmick card, without a
      stagger, at **75 %, 50 % and 25 % of its HP**. Then (2026-11-02): "only drop one at 50% and
      occasionally when guarding": the HP drop at **50 % only**.

## Known open design questions (ask the owner, don't guess)

- Revisit later: party, progression.
