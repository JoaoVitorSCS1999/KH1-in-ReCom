# Boss revenge at Combat = KH1 (KH2's revenge value)

Owner (2026-11-01): "lets also have humanoid bosses have a revenge value system, similar to how they
work in their fights in KH2 … so that sora cant endlessly combo him. All the organization 13 bosses
should have KH2 counterparts, just borrow their revenge value".

Owner's choices (2026-11-01): story / Absent Silhouette limits; every humanoid boss (not Genie
Jafar); on revenge the boss plays its own Re:CoM attack nearest its KH2 revenge move (Larxene: her
hop away); Re:CoM's boss damage falloff off for bosses with revenge, on for the rest. Engineering
(coordinator): KH2's timing (the check 20 frames into the reaction, Hades 30), lazy decay, ini key
`boss_revenge = on | off` (default on, no Config row: Re:CoM's 21 item slots are full).

Code: `crates/kh1-recom/src/revenge.rs` (table, values, counter rules, tests),
`crates/kh1-recom/src/win/revenge.rs` (hooks, tail, start, untouchable window); sites
`sites::REVENGE_HOOKS`, `REVENGE_ALLOC`, `REVENGE_PTRS`, `REVENGE_CHECKED`.

Owner (2026-11-02): "Can we make it so where the humanoid bosses that have revenge value can't be
damaged or interrupted during their retaliation move?" → § 4.

## 1. KH2 FM (Steam exe, timestamp `0x669E384A`)

| Piece | Address | What it does | Tag |
|---|---|---|---|
| Hit value | ATKP `+0x2A` (00battle `atkp`, 48-byte rows); rows found by `0x1403EAFC0` (id, sub) and `0x1403EB230` (hit count) | each attack row's revenge value | Checked (decomp + files) |
| Counter / limit | enemy `+0xD48` / `+0xD4C` (floats); the enemy ctors `0x1403DB840`, `0x1403DB930` set 0 / 100.0 | per enemy | Checked (decomp) |
| Limit set | AI syscall table 2 #76 = `0x140431E30` (`trap_enemy_set_karma_limit`; table list `0x140753490`, table 2 at `0x140756B60`, entry `0x140757020`) | the AI script's value | Checked (exe: all 540 of OpenKH's trap descriptors match the exe's) |
| Add | `0x1403DB730`, `0x1403DBB90` (the enemy's on-hit callback) | counter += the row's value, any attacker | Checked (decomp) |
| Decay | `0x1403DB440`, `0x1403DBBF0` with `0x1403CA250` / `0x1403CA7C0` | − the frame delta (1 per 60 Hz frame; `0x140154600`) while an AI action runs, not in a damage motion (slots 10–39, 140–150) during a priority < 300 action | Checked (decomp); action-flag meaning Assumption |
| Reset | none | only the decay lowers it | Checked (decomp, by field offset) |
| Check | motion frame trigger 28 → `0x14040DD70` case `0x1C` → `0x1403CAC50(enemy + 0x390, "revenge")` | limit ≤ counter starts the action named "revenge" from the AI's action table (`0x1403CA700`, `0x1403CAD00`) | Checked (decomp) |
| Where trigger 28 sits | the boss `.mset`s: frame 20 (Hades' flinches 30) of slots 10–15 (`20A`–`25A`), 16 / 18 and 20 / 21 (`30A` / `32A`), 131 / 132 | the damage motions | Checked (files) |
| Who picks the move | the boss AI's "revenge" action (priority 300, flag 1), by its rank (normal / hard / super_hard, by HP) | per boss | Checked (files: AI scripts read) |
| Difficulty | no difficulty term in add / decay / check | | Checked (decomp) |

### Per boss (the player's KH2 files; AI = the `.mdlx` BAR entry type 3)

| KH2 object | Limit (story / AS) | Data | Revenge move (action → slot) | Trigger 28 |
|---|---|---|---|---|
| `B_EX110` Axel II; `B_EX110_SKIRMISH` (same AI bytes) | 92 when spawn argument 1 = 0, else 100 (default) | `B_EX110_LV99`: 75 | burst (`atk_burst_revenge`, 203/204) at normal rank, assault in the air / at super hard | yes |
| `B_EX400` Larxene (`N_CM010_BTL`: no fight AI) | 100 | 75 | `revenge_utsusemi_start` (decoy teleport) / `revenge_thunder_air` | yes |
| `N_CM040_BTL` Vexen | 150 | 125 | `rst_<rank>` (204 / 205) | yes |
| `N_CM000_BTL` Marluxia | 100 | 75 | `revenge_kukankiri` (206, space-cut) and follow-ups | yes |
| `N_CM020_BTL` Lexaeus | 100 | 75 | stagger, `atk_revenge_kabuto` / `atk_daichi` | yes |
| `B_EX370` Zexion | 125 / 100 / 75 by phase | 75 | `revenge_capture(_all)_air`, `atk_capture` | yes |
| `B_HE030` Hades (3rd, Paradox) | 80; 50 below 30 % HP (checked on each hit, never back) | — | `warp` (air) / `atk_figawall` | yes (frame 30) |
| `B_HE030_PART` Hades 1–2 | none (100) | — | starts unregistered actions | only slots 131/132/241/242 |
| `N_HB550_BTL2` Cloud | 100 / 80 / 60 / 40 by spawn arg 1 | — | `bushin_check`, `atk_kyougiri` / `atk_sonicrave` | `.mset` not found |
| `N_HB530_BTL2` Leon | 100 / 80 / 60 / 40 | — | `atk_slash` / `atk_circle` | `.mset` not found |
| `B_AL020` Genie Jafar | none (100) | — | ranked actions, never started | none: no revenge |
| `B_CA010` Barbossa | none (100) | — | mode / Heartless call / charges | yes |

"Data" = spawn argument 2 = 1 (Assumption: the room spawn files weren't read).

## 2. In Re:CoM

### Re:CoM's own anti-combo today (Checked (decomp))

- Boss damage falloff in the hit handler `0x14017F7E0` (ids 100..199): `+0x288` hits, after 5 the
  multiplier `+0x290` −0.25 a hit (floor 0.05), `+0x28C` = 1500 ticks; the enemy update
  `0x140185320` resets both when the timer runs out. Damage only.
- Human bosses: reload escapes (`+0x260` bit 7, set by `0x140199E10`) and random reactions when
  Sora attacks (`0x14017F720`); bosses' card attacks are not hittable. At KH1 (decisions 14, 26,
  P5) only the falloff and the random reactions remain.

### What we built

| Part | How | Tag |
|---|---|---|
| Counter | a 0x30-byte tail (0x20 until 2026-11-02; +0x20 state, +0x24 ignored-hit counts, +0x28 window start for § 4) after each boss body: the factory `0x140188F40`'s `mov ecx, size` for the 12 classes raised by the tail at install (`sites::REVENGE_ALLOC`); set up by our stats-init wrapper (`0x14017FF50`) and again on first use when its magic / battle number differ | Checked (disasm: each size feeds `0x1404ACE54` right before the class ctor; the factory's only write past the ctor is `+0x294` at `0x140189B7F`) |
| Add | in the enemy-hit hook: `win::revenge::enemy_hit` runs the KH1 wrapper, then adds the hit's value when Re:CoM applied it (return 1; held hits of a stopped enemy when they land). Marluxia 2 / 3's parts (`CBossMarluxia2Funnel`, `CBossMarluxia2Scythe`, `CBossMarluxia3Specter`) add to the body | Checked (decomp) |
| Falloff off | for bosses whose revenge acts, `+0x288` / `+0x28C` / `+0x290` put back after Re:CoM's handler (Hood and Leon keep it: no attack state of their own) | Checked (decomp) |
| Decay | lazy, at each hit and check: 1 per 60 Hz frame of the game clock (Sora's dt `player+0x460` summed in the request-bits wrapper), minus the frames spent in the flinch / blow-away states (counted in their step hooks) | Checked (decomp); matches KH2's pause in damage motions |
| Check | hooks on Re:CoM's flinch step `0x140184100` (state 11, vtable `+0x1F8` of all 12 classes) and blow-away step `0x140182E50` (state 12, `+0x210`): at frame 20 (Hades 30) of the state, or at its end if Re:CoM's state is shorter (its exe then requests another state, `+0x68` ≠ `+0x60`); once per reaction; a new hit starts a new reaction | Checked (exe: `sites::REVENGE_PTRS`); the end rule is ours (Re:CoM's flinches can be shorter than KH2's damage motions) |
| Start | the transition Re:CoM's message 2 makes, as the cardless card play P1 does: `+0x260` = `cardless_flags`, `+0x278` = the state, `+0x68` = the state's object; only when the boss is alive and CBTL isn't locked (`0x1401634B0`'s test). Re:CoM's own state code then runs the attack and ends it | Checked (decomp) |

The human bosses don't start attacks through the attack request: their `+0x98` (`0x1401829D0`)
only clears bits; their think plays a card from their move list (`0x140199A00`) and message 2 starts
it. The sleight start `0x14019A1E0` needs stocked cards. So the start is message 2's transition.

### Hit values (KH2 Sora's ATKP `+0x2A`, Checked (files))

Ground swing 8 (126–128), ground finisher 24 (129 / 130), air swing 12 (131 / 132; which rows are
air: Assumption), air finisher 28 (133 / 134), Fire 8, Blizzard 16, Thunder 32. Everything else
(Gravity, Stop, Aero, the modifiers and their finishers, Sonic Blade, Counterattack, the jump-up
swing counts as a ground swing, a shot Guard turned back): 10, KH2's most common value
(Assumption: the coordinator's choice).

### Per boss (Re:CoM)

| Re:CoM (id) | Limit | Revenge: Re:CoM state (what it is) | Check frame |
|---|---|---|---|
| Axel (118) | 92 | 4: warp (`e118_003` / `e118_004`), then a growing flame effect around him (`e118_060` / `061`) — nearest KH2's burst | 20 |
| Larxene (117) | 100 | 4: hop back (`e117_008`) within 6 m, then lightning from range (`e117_055` / `056`) — her escape | 20 |
| Vexen (114) | 150 | 6: ice needles (`INeedleSpd`, `e114_062..064`) | 20 |
| Marluxia (119) | 100 | 2: scythe cut at Sora (`e119_050`) | 20 |
| Marluxia 2 (120), parts 135–138 count on the body | 100 | 2: the death scythe (`CBossMarluxia2AtkDScythe`) | 20 |
| Marluxia 3 (132) | 100 | 2: scythe strike through its specter object | 20 |
| Hades (107) | 80, 50 below 30 % HP | 5: fire-up (`e107_054..056`, no flinch for 3000 ticks, flames) — nearest the Firaga wall | 30 |
| Cloud (106) | 100 | 6: Omnislash ("bushin" dashes, `e106_057..062`) — nearest Sonic Rave | 20 |
| Captain Hook (112) | 100 | 3: sword slash (`e112_050`) | 20 |
| Riku / Riku Replica (116) | 100 | 4: slash (`p003_039`) | 20 |
| Hooded figure (100) | 100 | none: no attack state of its own (counter only) | 20 |
| Leon (101) | 100 | none: his only attack is his reaction card (fight skipped at KH1) | 20 |

Which state is "nearest" KH2's move is an Assumption from each state's motions and code; the
states are the classes' own (Checked (exe): vtables). Not in the table: Genie Jafar (no revenge in
KH2), Riku's story (Re:CoM's numbers).

### Logs and trace

- Start: `revenge: KH2's revenge value on 12 humanoid bosses …`, or why it isn't installed.
- Each decision past the limit (first 200): `revenge: Axel 0x… counter 96.0 >= 92 at frame 20 of its
  reaction -> state 4 (warp, then a flame burst around him)`; refused (dead, CBTL locked) and
  no-attack lines.
- F9 trace, `hits` column: `:rv <counter>` after each counted hit.

## 3. Assumptions and not checked

- **Finding (2026-11-02), not changed:** the check's "the state ends before frame 20" test
  (`+0x68 != +0x60` after Re:CoM's flinch step) is true on every frame without a queued change:
  Re:CoM's state machine `0x140242F50` (the enemy's at `+0x58`: `+0x60` current, `+0x68` next, run
  by the actor update `0x1401A59A0`) zeroes `+0x68` after each change. So the decision is made on
  the **first frame** of each flinch / blow-away, not at KH2's frame 20 (Hades 30). Checked
  (decomp). A plain fix (`+0x68 != 0`) would miss flinches the boss leaves through its own think:
  Re:CoM's flinch step `0x140184100` frees the boss (`+0x260 |= 0x1010`, bit 0 cleared) a quarter
  into the motion, and the think (`+0x3D0` from `0x140185320`) can queue its next state before the
  step runs. Owner question (how the check should time).
- Nothing seen in game.
- The attack states chosen (above); the human bosses' attacks starting without a card read no card
  record (checked: none of the ten chosen states reads `+0x318..+0x338`); Marluxia 2's death
  scythe uses the last think's variant fields (`+0xD30`, `+0xDD0`).
- Re:CoM's flinch / blow-away lengths against KH2's frame 20 (the state's end decides when shorter).
- The game clock follows Sora's dt (`player+0x460`, Re:CoM units); pauses stop both.
- KH1 rows as KH2 rows (air swings 131 / 132).
- The hooded figure and Leon get a counter but nothing starts.

## 4. Revenge moves untouchable (2026-11-02)

Owner: "the humanoid bosses that have revenge value can't be damaged or interrupted during their
retaliation move". Built natively: Re:CoM's own rule for an attacking enemy, without decisions 14 /
26's bypass, for that one attack.

### Re:CoM's rule (Checked (decomp + exe))

| Piece | Address | What it does |
|---|---|---|
| Hit handler drop | `0x14017F7E0` (`0x14017F8CB`) | `+0x260 & 1` without `0x2000000`: returns 0 before damage and reaction (with both bits it ends the card, `0x14017E1D0`, and takes the hit) |
| Hurt bodies off | vtable `+0x438`, default `0x14017DA80` = `0x1401A2D70(e, -1)`; Vexen `0x1403B3260` and Marluxia 2 `0x140400410` call it first | run by every attack state's begin wrapper (states 2..10: class begin `+0x118 + (k − 2) × 0x18`, then `+0x438`; `0x14017B720` …) |
| Attack end | `0x14017E540` | clears bit 0, queues the next state (or the idle `+0x2F8`) |
| State end | vtable `+0x440`, default `0x14017DA90` (Marluxia 2's `0x140400460` calls it) | clears bits 0 / 2, all hurt bodies back on (`0x1401A2DB0`) |
| State machine | `0x140242F50` (enemy `+0x58`) | sets `+0x60` = the new state, zeroes `+0x68`, then runs the begin |

The revenge start (message 2's writes, `crate::battle::cardless_flags`) sets bit 0. Checked
(decomp). Its begin runs `+0x438` like any attack state (all 12 classes: `REVENGE_PTRS`).

### What we built

| Part | How | Tag |
|---|---|---|
| Pending | the revenge start marks the tail (`flag::PENDING`, the state at `+0x20`) | Checked (decomp) |
| Window opens | our hurt-off wrapper (`0x14017DA80`, decision 26) runs Re:CoM's own call when the state beginning (`+0x60`) is the boss's revenge state with the mark (`win::revenge::begin_untouchable`): bodies off. Any other attack keeps decision 26 | Checked (decomp) |
| Hits | in the enemy-hit hook, a hit on a boss in its window (or pending; bit 0 set, HP > 0) goes to Re:CoM's handler without the card-interrupt bit for every source (KH1 swing, spell, Stop, turned-back shot, Re:CoM's own), so it drops it (`win::attack::untouchable_hit`); the KH1 swing / spell record is taken (it can't land later); no MP charge | Checked (decomp) |
| Guard | decision 27's flinch for a blocked melee attacker is skipped for a boss in its window (`win::guard::after_enemy_hit`) | — |
| Window closes | new hook on Re:CoM's state end `0x14017DA90` (8 bytes `48 83 A1 60 02 00 00 FA` moved; tail `BA FF FF FF FF E9 0E 53 02 00` checked; no branch into `0x14017DA91..97`, Checked (exe rel32 / pointer scan)): the mark cleared, the end line logged; Re:CoM turns the bodies on. A boss that dies mid-move ends here too (its death state replaces the attack) | Checked (decomp + exe) |
| Off | `boss_revenge = off`, Combat = Re:CoM, Riku's story: nothing marked, all as before | — |

Other bypass points checked (grep of `0x2000000`, `0x14017DA80`): the five interrupt-bit sets in
`win::attack` (KH1 swing, turned-back shot, spell via `crate::damage::interrupts`, Stop, Re:CoM Stop)
are all past the untouchable branch; the hurt-off wrapper is the only skip of `0x14017DA80`; Stop's
"attacks off" (`0x1401A2D90`) only runs after a landed Stop; gimmick-card break (message 3) and
message 6 go to Heartless bosses only (none in `BOSSES`); summons and specials use Re:CoM's
collision (bodies off) and set no interrupt bit. Checked (code).

### The meter (choice)

Hits during the window add nothing; the counter keeps draining (1 a frame; an attack state has no
flinch frames). KH2 (Checked (decomp + exe)): the add `0x1403DB730` sits in the enemy's on-hit slot
`+0xC0` (vtable `0x1405C71D8`), which `0x1403D1730` (and `0x1403D0780`) call after the damage step
`0x1403D1B40` for every hit that meets the enemy: the damage step is behind a flag test, the add is
not. The decay keeps running during the priority-300 revenge action (§ 1's decay row). Here the
boss's hurt bodies are off, so Sora's attacks don't meet it: KH2's "no contact, no add". Hits that
reach the handler anyway (none expected: Thunder / Gravity / Stop's `strike_body` skips bodies that
are off) and part hits (Marluxia 2 / 3) add nothing during the window either. Whether KH2's own
revenge moves are contact-free: Assumption (not read per boss).

### Per boss: the window (state begin → Re:CoM's attack end)

Motion lengths not checked (the enemies' `.ANN` files aren't in this session); the rest Checked
(decomp of each state's begin / exe). Timers are the enemy's `+0x160`, counted down by its dt in the
actor update `0x1401A59A0` (Checked (decomp)).

| Boss | State | Window |
|---|---|---|
| Axel | 4 | `e118_003` warp, `e118_004` ×2, flame `e118_060` / `e118_061`; ends with the last motion |
| Larxene | 4 | hop back `e117_008` (or `e117_003`), lightning `e117_055` / `e117_056`; ends with the last motion |
| Vexen | 6 | `e114_062`, then `e114_063` for **300 frames (15000 ticks, 5 s)** with needles every 12 frames, then `e114_064` |
| Marluxia | 2 | `e119_050`; one motion |
| Marluxia 2 | 2 | **58 frames** (2900 ticks), then the death scythe until it is gone (`0x1403FBA20`); variant 0 ends at once |
| Marluxia 3 | 2 | one motion |
| Hades | 5 | `e107_054`, then `e107_055` spinning for **240, 480 or 720 frames** (4 / 8 / 12 s, random: (rand % 3 + 1) × 12000 ticks), then the attack ends |
| Cloud | 6 | `e106_057`, then five rounds of chase (`e106_058`, no time limit until within 4 m of Sora) and dash (`e106_059` / `060` / `061` / `061` / `060`), then `e106_062` |
| Captain Hook | 3 | `e112_003` / `e112_011` / `e112_050` (and `e112_004`); ends with the last motion |
| Riku / Riku Replica | 4 | `p003_039`; one motion |
| Hooded figure, Leon | — | no revenge attack: no window |

**Hades:** 4–12 s untouchable plus the wind-up, the longest by far (KH2's warp / Firaga wall is a
short move). The state also starts his fire-up mode (`+0x260 |= 0x80000`, reaction picker: voice
only) for his next three attacks (`0x1403B0470`); those are not untouchable (hits land, he doesn't
flinch, so no revenge check either). It is Re:CoM's own length for that attack, but at Re:CoM Sora
can break it with a card. Owner question: keep the whole state, or untouchable only through the
wind-up `e107_054`. Cloud's chase has no time limit (window as long as Sora keeps away); Vexen's is
5 s.

### Logs and trace

- `revenge: Axel untouchable (state 4: warp, then a flame burst around him) 0x…: Re:CoM's rule …`
  when Re:CoM begins the state.
- `revenge: Axel untouchable ends (state 4) 0x… after 93 frames: ignored 5 hits (3 KH1 swings, 2 KH1
  spells, 0 other), 1 Guard reactions skipped; HP 1234` at Re:CoM's state end (`(dead)` if HP 0).
- `… another attack state began … not untouchable` if Re:CoM began another attack before the revenge
  state; `… took a hit in its untouchable move` should never appear.
- F9 trace, `hits` column: `e…:untouchable:Swing:dmg 0:hp …` per ignored hit.
- Shares the 200-line cap with the other `revenge:` lines.

### Assumptions

- No end line if the battle ends during the move (the boss is deleted without its state end).
- "Other" ignored hits may include Re:CoM's call for a swing Sora's Guard blocked.
- Marluxia 2 / 3's parts keep Re:CoM's rules: Marluxia 2's own `+0x438` turns two of them off too.
- Not seen in game.

## 5. Owner test list

1. Log at start: `revenge: KH2's revenge value on 12 humanoid bosses …`, no `not installed`.
2. Axel: three or so chained ground combos without a pause → after a finisher he warps and bursts
   (`revenge: Axel … >= 92 … -> state 4`). Pauses of a second or more between combos drain it.
3. Vexen takes about one combo more (150) before his ice needles.
4. Larxene: past 100 she hops back and casts lightning.
5. Hades: 80 at first; below 30 % HP his revenge (fire-up) comes sooner (50).
6. Marluxia 2: hitting the funnels / scythes counts toward the body's revenge.
7. Cloud, Hook, Riku / Riku Replica, Marluxia 1 and 3: each breaks out with the listed attack.
8. Damage on these bosses no longer drops after 5 hits (falloff off); Guard Armor and other
   Heartless bosses keep Re:CoM's falloff.
9. `boss_revenge = off`: no `revenge:` lines, Re:CoM's falloff back.
10. Combat = Re:CoM and Riku's story: unchanged.
11. Untouchable (§ 4): during Axel's revenge (warp + flame burst) swings, Fire / Blizzard / Thunder
    and Sonic Blade do 0 damage and don't stop it; log `revenge: Axel untouchable (state 4 …)`, then
    `… untouchable ends (state 4) after N frames: ignored M hits …`.
12. Right after the end line, hits land (damage and flinches) as before.
13. Guard during a revenge move: Sora blocks, the boss doesn't flinch (`… Guard reactions skipped`).
14. Vexen's needles (~5 s) and Hades' fire-up spin (4–12 s) stay untouchable the whole time: say if
    Hades' is too long.
15. A normal (non-revenge) attack of the same boss is still hittable mid-attack (decision 26).
16. `boss_revenge = off`: bosses hittable in every attack, as before.
