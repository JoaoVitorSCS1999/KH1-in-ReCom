# M13: KH1 effects, sounds and voices (plan)

Owner's ask (2026-11-03): "Let's work on m13". Roadmap: "KH1's swing trails, hit effects, spell effects,
sounds and voices, built into Re:CoM's effect programs and sound banks."

The research is in `recom-m13-kh1-fx.md` (KH1), `recom-m13-recom-fx.md` (Re:CoM) and the inventory below.
The owner's answers are in `design-philosophy-instructions.md` § "M13 effects, sounds and voices".

## Key finding

KH1 and Re:CoM use the **same effect format (DPX `0x82`, PPP `0x96`), at the same version**:
- Re:CoM's `ex_5010.dpx` (the Kingdom Key trail and sparks, in `EFFECT/1000/EF1000.EFF`) is **byte-identical**
  to KH1's `xw_ex_5010.wpn` section 0. Checked (files).
- Re:CoM's `ex_0010.dpx` is an edited copy of KH1's Sora set with KH1's ids 6 and 7 (Ripple Drive) removed.
  Checked (files).
- Re:CoM's op table (`0x140859DD0`, 242 ops) starts with KH1's 235 ops (`0x14054FF50`), in the same order.
  Checked (exe).
- The one data difference: KH1 keeps an effect's sound in its program header (`+0xFC` / `+0xFE`); Re:CoM keeps
  it in a `.ese` table of the same name. Checked (files).

So KH1's effect sets go into Re:CoM as Re:CoM's own `.EFF` files (renamed `kh1_*`, since effect names are global),
with `.ese` tables written by the asset step. Re:CoM loads, steps, draws and ends them.

## How it is built (native)

1. **Asset step.** From the player's files, the asset step writes:
   - a new Re:CoM `.EFF` holding KH1's sets: Sora's 8-program set, each Keyblade's `.wpn` set, `always.dpx`,
     the spells' `.mag` sets and the ported enemies' sets;
   - the sets' `.ese` tables;
   - the sets' HD textures;
   - and loads it with Sora's group (a LINK row, or an EFF Sora already loads).
2. **Sound banks.** KH1's sounds registered as Re:CoM banks: Sora's `.mdls` SCDs, the Keyblade and effect
   sounds (`se000`, the world `se<ww>` packs) and the voices (`voice/<ww>_sora`). This uses the route the KH1
   menu sounds already use (bank 9000: `0x1402F5D70` registration, the `0x1402A8560` extension). A KH1 → Re:CoM
   sound-id map goes with them.
3. **Per-move events.** For every KH1 move, the asset step writes EET and SES records into Sora's pack;
   Re:CoM's own `0x14017A240` / `0x140239260` fire them. They follow KH1's frames:
   - trails on the mset type-7 windows;
   - sounds on the type-6 triggers;
   - voices by KH1's weighted groups.
   Each set of records is chosen by the toggle's variant.
4. **Toggles** (KH1 > Combat): Effects, Sounds and Voices, each KH1 / Re:CoM, plus decision 34's spell-effects
   toggle. At Re:CoM nothing changes.

## Stages (one system per version)

1. **Groundwork.**
   - Confirm Re:CoM's PPP tick rate against KH1's 30 Hz (`0x1401ECB50`).
   - Confirm whether Re:CoM's player reads program `+0xFC`.
   - Build the KH1 `.EFF` and load it with Sora. Nothing visible changes; the log shows it loaded.
2. **KH1 sound banks** and the id map.
3. **Sora's combos and finishers.** KH1 trails and swing sounds; KH1 voices (groups 4 / 5 by KH1's weights);
   the KH1-only moves' effects (Blitz, Ripple Drive, Hurricane Blast, Gravity Break, …).
4. **Per-Keyblade trails.** One EET per Keyblade group, each with its own `.wpn` set.
5. **Hit sparks.** KH1's by hit kind (normal, hard, critical, wall); Guard and clash sparks from `always.dpx`
   10 / 11.
6. **Spells.** Decision 34's toggle; Re:CoM's spell sounds kept; Fire first.
7. **Specials.** KH1's effects and voices for Sonic Blade, Strike Raid, Ars Arcanum and Ragnarok (owner,
   2026-11-03).
8. **Movement and damage.** Footsteps by surface (`footstep.bin`); jump, land and roll; hurt and death voices
   (groups 0 / 1 / 2); the silent recoils.
9. **Enemies.**
   - The ported Heartless's own sets: warps, the Search Ghost's light, the Fat Bandit's flame, the Gargoyle's
     ball, fades.
   - Sounds for the Red Nocturne, Green Requiem, Yellow Opera and the Air Pirate's swoop.

## Inventory before M13 (v0.101.6)

- No KH1 combat sound or voice plays: every swing, spell, special and enemy uses Re:CoM's. Only the KH1 menu
  sounds are KH1's (bank 9000).
- No KH1 effect is built. The asset step writes no EET, so the KH1-only clips have no motion effects.
- Silent:
  - Sora's strong recoil 0x6F, air recoil 0x70 and the Neoshadow-grab held reaction;
  - the Air Pirate's swoop;
  - every Red Nocturne / Green Requiem / Yellow Opera motion (Re:CoM has no `EN0003/5/6.SES`).
- Stand-ins: Guard and Counterattack sounds; guard and clash sparks; the Bandit's scimitar; the Fat Bandit's
  flame; the Gargoyle's ball; the Yellow Opera warning; the Aero barrier.

## Open (ask at their stage)

- No hit-stop at KH1 (none found in KH1: `recom-m13-kh1-fx.md` § 2; this corrects the clash "hit-stop" in
  `recom-m11-enemies.md`).
- KH1's room dust and splashes.
- Clash EXP.
- Effects without flag 0x800 end on a motion change (KH1's rule): part of stage 3's 1:1 check.
