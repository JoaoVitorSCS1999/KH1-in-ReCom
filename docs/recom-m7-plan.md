# M7: command menu, MP and magic — plan

Owner: "lets work on M7. Make sure all of that is FULLY implemented, also including a shortcut
menu, customize menu, MP bar" (2026-10-31). Decisions: 34 in `design-philosophy-instructions.md`.
Research notes (kept with the decomp tools, not in the repo): `m7-magic.md`, `m7-menus-mp.md`.

One system per version, in this order. Each stage is native: Re:CoM's own objects, rows, request
bits, HUD and camp code run the feature; ours only adds data, wrappers and commands.

| Stage | System | Native host (Re:CoM) | Notes |
|---|---|---|---|
| 7.1 | **MP**: max MP by level, MP and charge, charge from hits, saved per save | save spare range (next to the Keyblade byte), the enemy-hit wrapper | KH1 rules: max MP 3 → 8 (+1 at levels 12/24/36/48/60); bar k needs 30·k charge; hits add 3 / 6 / 24 × level factor (1 + 0.2·Δlevel, 1..2.5×). Log + trace only. |
| 7.2 | **MP bar** | a new layer object of `Gauge::CPlayerGauge` (made in its init, placed like Riku's Dark gauge), sequence data added to `Cockpit:00` (same `FO0002.SPR`; its neutral texel is readable, `gauge:01`'s isn't) at the asset step | straight bar under the HP bar, 26 units per MP (grows with max MP), KH1's MP colour read from the player's KH1 files. Built: `recom-m7-mp-bar.md`. |
| 7.3 | **Command menu**: Attack / Magic / Items / context, Magic list | `C2DSeqCtrl` rows on the player gauge's wrappers, Re:CoM's `Cockpit:00` text plate (two groups added at the asset step), `C2DFnt` text, request bits | field and battle; KH1's layout (16, 260), rows 22; highlighted row = another plate colour; KH1's words from the player's KH1 `Word.bin`; Items greyed until M8. Built: `recom-m7-command-menu.md`. |
| 7.4 | **Cardless cast + Fire** | Re:CoM's magic action `0x13` (initMagic `0x1402203D0`'s writes, exeMagic hooked, a Field exe case), new KH1 attack rows, Re:CoM's spawn's Fire case with KH1's numbers | MP paid at release, interrupt = no spell / no MP, Fire chain from frame 14, cast from combo windows, air-cast hop. Built: `recom-m7-magic.md`. |
| 7.5 | Blizzard | same | 7-shard cone. |
| 7.6 | Thunder | same | every enemy in the area, one bolt per 8 frames. |
| 7.7 | Cure | same | Re:CoM's heal: 50 / 150 / 300 HP; KH1's cast rules. |
| 7.8 | Gravity | Re:CoM kind `0xD` | KH1 twin's resist[8]; Re:CoM-only bosses: 2 %. |
| 7.9 | Stop | Re:CoM kind 8 | KH1 twin's resist[9]; Re:CoM-only bosses immune; held hits. |
| 7.10 | Aero | a new player damage status in the damage check | halves damage for (18 + max MP) s; Aerora/Aeroga wind. |
| 7.11 | **Spell unlocks and levels** | Re:CoM's card pickup and boss battle ends | base level at the first pickup of that magic card; -ra / -ga at Re:CoM bosses; saved per save. Built: `recom-m7-unlocks.md` (Re:CoM's first-obtained `0x14015C7F0`; a boss win = its enemy card's first pickup; two bosses a spell, +1 each). |
| 7.12 | **Shortcut menu** | the command menu object | hold L1: Triangle / Square / Cross cast slots 1 / 2 / 3. |
| 7.13 | **"KH1" camp entry + Customize** | our camp entry (renamed) and page, Re:CoM's camp buttons | submenu: Keyblades, Customize (3 shortcut slots from learned spells + None). |

Out of M7 (later milestones): Items and Summon rows' contents (M8), MP Rage / MP Haste and other
abilities (M9), KH1's context-row specials in battle (M9), lock-on on R1 (M12).

Until 7.11, a debug ini switch gives Sora every spell at level 3 so 7.4–7.10 can be tested. It stays
after 7.11 (`kh1_all_spells = on` overrides the learned levels in the menu).
