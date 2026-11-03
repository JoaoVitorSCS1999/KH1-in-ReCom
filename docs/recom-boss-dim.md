# Bosses untouchable during the moves that dim the room (Combat = KH1)

Owner (2026-11-03): "anytime bosses use a move that dims the room, just make them invulnerable for
that move". Recorded in `docs/design-philosophy-instructions.md`. Combat = KH1, Sora's story;
Re:CoM's card battles and Riku's story unchanged.

Code: `crates/kh1-recom/src/dim.rs` (the rule, the slot layout, the tables, tests),
`crates/kh1-recom/src/win/revenge.rs` (`dim_begin`, `dim_exe_*`, `open_dim`; the window is the
revenge move's, `docs/recom-boss-revenge.md` § 4), `win/attack.rs` (`untouchable_hit`, `hurt_off`);
sites `sites::DIM_CHECKED`, `sites::DIM_EXE_SLOTS`.

## 1. What dims the room (Re:CoM, Checked (decomp + exe))

| Piece | Address | What it does |
|---|---|---|
| Colour setter | `0x140232170(mask, frames, &rgba, start, slot)` | sets one of `CSceneEnvTask`'s four slots: start = current, target = rgba, `+0x34` progress = 0, `+0x38` = 1 / frames, `+0x3C \|= 1`, `+0x30` = layer mask |
| The table | `[0x14087B7F0]`, 4 × 0x40 bytes (`0x140231EE0` makes it) | `+0` start, `+0x10` target, `+0x20` current, `+0x30` mask, `+0x34` progress, `+0x38` rate, `+0x3C` bit 0 |
| The fade | `0x140232060` (task step), `0x140231BE0` | progress += dt × 0.02 × rate; the colour eases to the target; a finished fade to 1.0 clears the mask |
| Who reads it | `0x140231E30(layer)` multiplies every slot whose mask holds the layer | `CMap` (`0x1401CFDC0`: layers 11 and 12; `0x14028E4D0`: 12), `CSky` (`0x1401D0710`: 12) |

So "the room dims" = a request for a colour under 1.0 on mask `0x1800` (layers 11 / 12): the map
and the sky go grey; Sora and the enemies keep their colour (their layer is the actor's `+0x150`).
The boss's state end asks for 1.0 again (30 frames back).

Not this: Re:CoM's global screen fader `CColorFader` (`[0x140AC4438]`, scene changes only); Guard
Armor's vtable `+0xC8` (`0x1403AB670`) fades its own body (mask 2, alpha 0). Effects that draw a
darkening of their own (effect files): Assumption, none read (`.EET` / `.PAX` not in the session).

## 2. Which bosses dim (every caller of `0x140232170`, Checked (decomp + exe))

Slot 0 for all but Marluxia 3 (the `[rsp+0x20]` argument; Checked (disasm: Riku, Larxene's helper;
decomp: the rest)). Colour 0.5 grey over 30 frames unless noted. Motion names are each state's;
what the move is: Assumption.

| Boss (id) | State | Where the dim is asked | Motions / notes |
|---|---|---|---|
| Riku / Riku Replica (116) | 2 | begin `0x1403B5AB0`, **only when its spawn value `+0x294` > 0** (factory `0x140188F40`'s last argument) | `e116_050` |
| | 5 | begin `0x1403B5BE0` | `p003_041` / `042` |
| | 6 | begin `0x1403B5C90` | `p003_043` |
| | 7 | begin `0x1403B5D20` | `p003_044..048`, `DACount` / `DASpeed` (Dark Aura) |
| Larxene (117) | 3 | step `0x1403BBAF0`, sub-step 4 (68 frames into the combo's last swing), through her helper `0x1403B8AB0` | combo `e117_050..053`, then lightning |
| | 4 | begin `0x1403B8C40` (only when Sora is 6–8 m away) or step `0x1403BC1D0` (after the hop back / approach), helper | lightning `e117_055` / `056` (her revenge move) |
| | 5 | begin `0x1403B8D50` | `e117_057` |
| | 8 | begin `0x1403B8EE0` | `e117_008..010`, `e117_065` |
| Axel (118) | 4 | begin `0x1403BEAA0` | warp + flame burst (his revenge move) |
| | 5 | begin `0x1403BEB40` | `e118_062..066`, `Firecount` / `Boundcount` |
| Marluxia (119) | 5, 6, 7 | begins `0x14040E3D0` (0.6), `0x14040E4A0` (0.5), `0x14040E540` (0.6), mask `0x1808` | `e119_057`; `e119_058..061`; `e119_063..066` |
| Marluxia 2 (120) | 6 | begin `0x1403FFD00` (16 frames) | |
| Marluxia 3 (132) | 7 | step `0x1403F5F90` (after its first motion), **slot 1, mask `0x1000`** | |
| | 9 | step `0x1403F6960` (when the wind starts; it asks for 1.0 again when its timer ends, still in the state) | `WindPow` / `WinsTime` |
| | idle think | state 1 step `0x1403F7110` (vtable `+0x108`), patterns 4 / 5 / 7 of `+0xDF4`, before their card plays | the attacks that follow begin with slot 1 dark; `0x1403EE0B0` asks for 1.0 when the pattern changes (at a state end) |
| | 4 | step `0x1403F4F30`, sub-step 4: **duel mode only** (`+0xCE4 & 4`), never at KH1 (decision 33) | |

**Never dim** (no call in their class code): Vexen, Cloud, Hades, Captain Hook, Leon, the hooded
figure, and every Heartless boss (Guard Armor, Trickmaster, Jafar, Parasite Cage, Oogie Boogie,
Ursula, Dragon Maleficent, Darkside). Their moves keep decision 26 (hittable).

Also dim, not bosses or not in scope: `CEnemyWizard` state 4 (`0x140195590`), `CEnemyAquatank`
state 3 (`0x14018AA70`), Ansem (Riku's story: states 4, 6, 7). Sora's own code dims slots 0 / 2 too
(player code `0x1401E4EB0`, `0x1401F6660`, `0x1401F9840` …; that these are his sleights: Assumption).

Slot 1 in battle: only Marluxia 3 and its part `CBossMarluxia3AllRangeBit` write it; the other
writers are field menus (`CRoomCreation`, `CWorldSelect` and their neighbours `0x1401153F0`,
`0x14012F4D0`, …) and `0x1404554F0` (Checked (exe): every call's slot argument). That a menu's slot 1
is never still dark in a battle: Assumption.

## 3. What we built

| Part | How | Tag |
|---|---|---|
| Begin dims | the bosses' state-change callback (`win::revenge::state_change`, already in the 12 classes' FSM `+0x28`) copies slot 0; when Re:CoM's attack begin wrapper (`0x14017B720` …: class begin, then `+0x438`) reaches the hurt-off hook, slot 0 is compared with the copy: changed, progress 0, bit 0, a colour under 1.0 on `0x1800` = this begin asked for the dark room (`crate::dim::at_begin`) | Checked (decomp: begin before `+0x438`; the FSM calls `+0x28` right before the begin) |
| Step dims | the four states' exes (`crate::dim::EXES`) are written with ours in the class vtables (`sites::DIM_EXE_SLOTS`, `PtrSite`s, Re:CoM's function checked as the old value): ours runs Re:CoM's exe and compares the slot before / after; on a dim it opens the window and calls the boss's own `+0x438` (Re:CoM's hurt bodies off; the hook lets it through for this boss) | Checked (exe: state handlers `jmp [rax+0x138]` / `0x150` / `0x198` / `0x1C8`) |
| Marluxia 3's own dim | at an attack begin, slot 1 dark (mask `0x1000`, colour under 1.0) opens the window | Checked (decomp) |
| The window | the revenge move's (`docs/recom-boss-revenge.md` § 4): Re:CoM's hurt bodies off for the state, hits go to Re:CoM's hit handler without the card-interrupt bit (its drop branch, `0x14017F8CB`), no Guard flinch, nothing on the revenge meter; tail flag `DIM` (0x20) | Checked (decomp) |
| Attack bit | a dim window holds whether or not `+0x260` bit 0 is still set (a KH1 hit before the dim may have ended the card, decision 14); for the handler's call the bit is lent and taken back, so Re:CoM's own rule drops the hit | Checked (decomp: the drop needs bit 0) |
| End | Re:CoM's state end `0x14017DA90` (hooked by revenge): flags cleared, end line; Re:CoM turns the bodies on and asks for 1.0. Marluxia 3 state 9's mid-state 1.0 doesn't end it (the move goes on) | Checked (decomp) |
| `boss_revenge = off` | the dim windows still work (the tails are always made); only the counter and falloff go | — |
| Per frame | nothing new, except the four steps while the boss is in that state and its window isn't open: two 64-byte reads and a compare (as Hades' wind-up step) | measured: not in game |

A revenge move that also dims (Axel 4, Larxene 4) opens the revenge window first (its own rule, bit
0); the dim rule adds nothing then.

## 4. Logs

- Start: `dim: bosses untouchable during the moves that dim the room …`.
- `dim: Axel untouchable (state 5: e118_062..066 …) 0x…: its state begin dims the room (Re:CoM's scene colour 0.50); …`
  (or `the room dims partway through its state`, or `the room is dim (its own colour slot 1) as the state begins`).
- `dim: Axel untouchable ends (state 5) 0x… after N frames: ignored M hits (… KH1 swings, … KH1 spells, … other), K Guard reactions skipped; HP …`.
- `dim: <boss> … the room dims, but its state is ending or it is down: not untouchable` (rare).
- Own cap of 200 lines. F9 trace `hits`: `e…:untouchable:Swing:dmg 0:hp …` as for revenge.

## 5. Assumptions and not checked

- Nothing seen in game. No trace.
- The owner's "dims the room" is this scene colour fade (the only boss-made room darkening in the
  code); an effect file darkening the screen on its own was not looked for.
- What each move is (the names above).
- Slot 1 is never left dark by a field menu when Marluxia 3's battle starts.
- Riku's `+0x294` (which fights have it > 0) not read.
- A boss that dims while Sora's own sleight dims slot 0 on the same frame before it: still seen (the
  comparison is around the boss's own code).

## 6. Owner test list

1. Log at start: `dim: bosses untouchable during the moves that dim the room …`.
2. Axel's bouncing fire (the room goes grey): swings, Fire / Blizzard / Thunder, Sonic Blade do 0
   damage and don't stop it; Guard blocks without a boss flinch; `dim: Axel untouchable (state 5 …)`,
   then `… untouchable ends (state 5) after N frames: ignored M hits …`. Right after, hits land.
3. Riku: each move that greys the room (Dark Aura etc.) is untouchable; his slash (revenge, not dim)
   as before.
4. Larxene: her combo is hittable until the room greys for its lightning end, then untouchable to
   its end (`the room dims partway through its state`). Her other grey moves from their start.
5. Marluxia 1 and 2: the grey moves untouchable; the others hittable.
6. Marluxia 3: while its background is dark, each attack it starts is untouchable; between attacks
   and once the room is bright again, hittable.
7. `boss_revenge = off`: 2–6 still hold.
8. Vexen, Cloud, Hades, Hook and the Heartless bosses: unchanged (their moves don't dim).
9. Sora's own sleight greying the room: bosses stay hittable.
10. Combat = Re:CoM, Riku's story: no `dim:` lines; everything as Re:CoM.

## 7. Owner questions

1. Larxene's combo greys the room only for its lightning end (~68 frames into the last swing):
   (a) untouchable from the grey (as built); (b) the whole combo from its start.
2. Marluxia 3's dark background also comes from its idle think, before an attack and between the
   attacks of one pattern: (a) each attack that starts or turns dark is untouchable (as built), the
   moments in between hittable; (b) also untouchable in between while the room is dark (a new
   untouchable state outside Re:CoM's attack rule).
3. Normal enemies that grey the room (Wizard, Aquatank): (a) leave them (as built: "bosses");
   (b) the same rule.
4. Bosses with big moves that don't grey the room (Cloud's Omnislash, Hades, Vexen …): (a) leave
   them (as built); (b) name the moves to add.
