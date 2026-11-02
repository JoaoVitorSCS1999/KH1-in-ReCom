//! KH1's item drops from enemies (M8 stage 8a.4, `docs/recom-m8-items.md` § Stage 8a.4): the pure
//! rules. Re:CoM's side is `win::drops`.
//!
//! - **KH1** (Checked (decomp + exe)): on defeat an enemy's AI script hands its prize table to
//!   syscall 122 / 215 (`0x1402BC640` / `0x1402BC6B0` → `0x1402ABC40`). After the seven prize
//!   counts, each of three (chance, item) pairs in order: the item is made (`0x1402BFF00`) when
//!   KH1's random float (`0x14031BA30`, [0, 1)) is below chance × the Lucky Strike factor
//!   (`0x142D60FA8` = 1 + 0.5 per equipped ability `0x1C`, `0x1402A6380`). The pickup
//!   (`0x1402C08D0`) adds it by `0x140284E60` (`crate::items::Inventory::pick_up`).
//! - **Here**: the twin's table from `kh1-stats.json` (format 6, read by the asset step from the
//!   player's KH1 files); Re:CoM's own prize factory makes each item as a new prize kind
//!   [`KH1_ITEM_KIND`] whose look is a new row of Re:CoM's prize look table ([`look_table`]); Re:CoM
//!   steps, draws, collects and times it out as its other prizes; its reward (Re:CoM's prize add
//!   `0x1401F4AB0`, a new case) is KH1's pickup.

/// Re:CoM's first prize kind above the cards (`0x1401F4AB0`: kinds ≤ 0xCA are cards; 0xCB..0xD7
/// EXP, HP, Moogle points, the roulette prizes). Checked (decomp + exe).
pub const FIRST_PRIZE_KIND: u32 = 0xCB;
/// Re:CoM's prize look rows (`0x1405B3150`: 13 rows of 16 bytes, kinds 0xCB..=0xD7: the effect
/// set name `rec_prise`, its id, the scale). Checked (exe).
pub const LOOK_ROWS: usize = 13;
/// A look row's size.
pub const LOOK_ROW: usize = 16;
/// The new prize kind: a KH1 item (the prize's `+0x248` holds the KH1 item id). The first kind
/// Re:CoM never uses: the prize add's switch sends kinds above 0xD7 to its default (no reward),
/// the bounce and pickup sound switches theirs (no bounce sound; pickup sound 6, the cards'),
/// and nothing else reads the kind above 0xCA. Checked (decomp + exe).
pub const KH1_ITEM_KIND: u32 = 0xD8;
/// Whose look a KH1 item prize borrows: 0xD6, the rare prize that starts Re:CoM's battle-end
/// roulette (`rec_prise` effect 5, scale 0.25). Re:CoM has no item prize; that this one reads as
/// "an item" is an **Assumption** (Re:CoM's `rec_prise.dpx` was not seen; owner to check).
pub const LOOK_FROM_KIND: u32 = 0xD6;

/// KH1's Lucky Strike factor (`0x142D60FA8`, `0x1402A6380`): 1 + 0.5 per equipped Lucky Strike
/// (ability `0x1C`; its name: Assumption). Our abilities have none yet (M9): 0.
pub fn lucky_factor(lucky_strikes: u32) -> f32 {
    1.0 + lucky_strikes as f32 * 0.5
}

/// Lucky Strikes Sora has equipped: none until KH1's abilities (M9).
pub const LUCKY_STRIKES: u32 = 0;

/// One (chance, item) pair of a KH1 prize table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drop {
    pub chance: f32,
    /// KH1's item id.
    pub item: u8,
}

/// KH1's roll (`0x1402ABC40`'s last loop): one random float per pair, in order; the item drops
/// when it is below chance × `lucky`. Returns the items that dropped (any id; see
/// [`spawnable`]).
pub fn roll(table: &[Drop], lucky: f32, mut rand: impl FnMut() -> f32) -> Vec<u8> {
    let mut out = Vec::new();
    for d in table {
        let r = rand();
        if r < d.chance * lucky {
            out.push(d.item);
        }
    }
    out
}

/// Whether a dropped KH1 item becomes a prize here: KH1's battle items only (the items the mod
/// has: `crate::items`). KH1's other drops (synthesis materials, accessories, Tents, …) have no
/// place yet: not spawned.
pub fn spawnable(item: u8) -> bool {
    crate::items::is_battle_item(item)
}

/// Re:CoM's prize descriptor (`0x1401A89C0`'s second argument, read by the prize init
/// `0x1402315B0`), as the EXP drop `0x1401810A0` fills it. Checked (decomp + disasm
/// `0x14018112D..0x1401811BD`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrizeDesc {
    /// Where it appears (the enemy's prize point).
    pub pos: [f32; 4],
    /// Launch angles (pitch, yaw, 0, 1), normalised by `0x140293F20`.
    pub angles: [f32; 4],
    /// `+0x20` → prize `+0x244`.
    pub kind: u32,
    /// `+0x24` → prize `+0x248` (the reward's argument).
    pub arg: u32,
    /// `+0x28` → prize `+0x24C` (0.25 for the EXP drop).
    pub speed: f32,
    /// `+0x2C` (1; byte `+0x2E` bit 0 clear: launched, not placed).
    pub flags: u32,
}

/// The EXP drop's launch: yaw = r1 × 360°, pitch = (−70 − r2 × 10)°, in radians with Re:CoM's
/// constants (`0x140529D5C` 360, `0x140509390` −70, `0x1406C11F4` 10, `0x140510D94` π / 180).
/// Checked (exe).
pub fn launch_angles(r1: f32, r2: f32) -> [f32; 4] {
    const DEG: f32 = 0.017_453_292;
    [(-70.0 - r2 * 10.0) * DEG, r1 * 360.0 * DEG, 0.0, 1.0]
}

/// Re:CoM's HP prize (`0x1401F4AB0` case 0xD3: `0x1401E59A0(player, 5 % of max HP (10 with
/// `player+0x390`), 9000, 0, 0)`; Re:CoM makes it itself only for Pluto's friend card, `0x1401E2E60`,
/// 3 / 7 / 10 of them). Checked (decomp).
pub const HP_PRIZE_KIND: u32 = 0xD3;

/// The Re:CoM prize a KH1 prize kind of a script's reward becomes (M11, `kh1_core::enemy::Reward`;
/// KH1's kinds are `0x1402AC080`'s switch: 0..2 munny 1 / 5 / 20, 3 / 4 HP 1 / 5 (`0x1402A4920`), 5 / 6
/// MP 2 / 15 (`0x1402A4AE0`); Checked (decomp)). KH1's HP balls are Re:CoM's HP prize (Re:CoM's heal,
/// decision M10's Re:CoM stats; the HUD keeps its "Recover", `docs/recom-kh1-hud.md`). The others have no
/// mapping yet (no reward that gives them is built): None.
pub fn reward_prize_kind(kh1_kind: usize) -> Option<u32> {
    matches!(kh1_kind, 3 | 4).then_some(HP_PRIZE_KIND)
}

/// A reward prize's descriptor at `pos`: launched as Re:CoM's defeat EXP drop launches its prizes
/// (`0x1401810A0`: speed 0.25, `+0x2C` 1).
pub fn reward_prize(pos: [f32; 4], kind: u32, r1: f32, r2: f32) -> PrizeDesc {
    PrizeDesc { pos, angles: launch_angles(r1, r2), kind, arg: 0, speed: 0.25, flags: 1 }
}

/// A KH1 item prize descriptor at `pos`.
pub fn item_prize(pos: [f32; 4], item: u8, r1: f32, r2: f32) -> PrizeDesc {
    PrizeDesc { pos, angles: launch_angles(r1, r2), kind: KH1_ITEM_KIND, arg: item as u32, speed: 0.25, flags: 1 }
}

/// Re:CoM's look table with the KH1 item row after its own: Re:CoM's 13 rows unchanged, then a
/// copy of [`LOOK_FROM_KIND`]'s row (so its index is `KH1_ITEM_KIND − 0xCB` = 13).
pub fn look_table(recom: &[u8; LOOK_ROWS * LOOK_ROW]) -> Vec<u8> {
    let mut t = recom.to_vec();
    let from = (LOOK_FROM_KIND - FIRST_PRIZE_KIND) as usize * LOOK_ROW;
    t.extend_from_slice(&recom[from..from + LOOK_ROW]);
    debug_assert_eq!(t.len(), (KH1_ITEM_KIND - FIRST_PRIZE_KIND + 1) as usize * LOOK_ROW);
    t
}

/// `lea rax,[rip+disp32]` at `at` reaching `target` (7 bytes), or None out of rel32 reach.
pub fn lea_rax(at: u64, target: u64) -> Option<[u8; 7]> {
    let d = target as i128 - (at as i128 + 7);
    let d = i32::try_from(d).ok()?;
    let b = d.to_le_bytes();
    Some([0x48, 0x8D, 0x05, b[0], b[1], b[2], b[3]])
}

/// The target of a `lea rax,[rip+disp32]` at `at`.
pub fn lea_target(at: u64, code: &[u8]) -> Option<u64> {
    if code.len() != 7 || code[..3] != [0x48, 0x8D, 0x05] {
        return None;
    }
    let d = i32::from_le_bytes(code[3..7].try_into().ok()?);
    Some((at as i64 + 7 + d as i64) as u64)
}

/// Re:CoM's look table (`0x1405B3150`) and the two places that index it by the prize's kind:
/// the prize init (`0x1402315B0`) and its look set-up (`0x1402306D0`; nothing calls it).
pub mod site {
    pub const LOOK_TABLE: u64 = 0x1_405B_3150;
    pub const LEA_INIT: u64 = 0x1_4023_1676;
    pub const LEA_LOOK: u64 = 0x1_4023_0700;
}

/// What a drop did, for the log.
#[derive(Clone, Debug, PartialEq)]
pub enum Dropped {
    /// A prize was made for this battle item.
    Spawned(u8),
    /// It dropped in KH1 but the mod has no such item: not made.
    NotAnItemHere(u8),
}

/// The roll's outcome for an enemy: each dropped item, spawnable or not.
pub fn outcome(table: &[Drop], lucky: f32, rand: impl FnMut() -> f32) -> Vec<Dropped> {
    roll(table, lucky, rand).into_iter().map(|i| if spawnable(i) { Dropped::Spawned(i) } else { Dropped::NotAnItemHere(i) }).collect()
}

/// A table for the log: `potion 1%, #233 3%`.
pub fn describe(table: &[Drop]) -> String {
    let v: Vec<String> = table
        .iter()
        .filter(|d| d.chance > 0.0)
        .map(|d| {
            let n = crate::items::name(d.item);
            let name = if n == "?" { format!("#{}", d.item) } else { n.to_string() };
            format!("{name} {}%", d.chance * 100.0)
        })
        .collect();
    if v.is_empty() {
        "none".into()
    } else {
        v.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq(v: &[f32]) -> impl FnMut() -> f32 + '_ {
        let mut i = 0;
        move || {
            let r = v[i % v.len()];
            i += 1;
            r
        }
    }

    // The Air Soldier's table (Checked (files): Potion 2 %, Hi-Potion 1 %, item 237 4 %).
    const AIR_SOLDIER: [Drop; 3] = [Drop { chance: 0.02, item: 1 }, Drop { chance: 0.01, item: 2 }, Drop { chance: 0.04, item: 237 }];

    #[test]
    fn each_pair_draws_once_in_order_and_drops_below_its_chance() {
        let mut n = 0;
        let r = roll(&AIR_SOLDIER, 1.0, || {
            n += 1;
            0.5
        });
        assert!(r.is_empty());
        assert_eq!(n, 3, "three draws even when nothing drops");
        assert_eq!(roll(&AIR_SOLDIER, 1.0, seq(&[0.019, 0.5, 0.039])), vec![1, 237]);
        assert_eq!(roll(&AIR_SOLDIER, 1.0, seq(&[0.02, 0.0099, 0.04])), vec![2], "strictly below");
        // A zero chance never drops, not even on a draw of 0.
        assert!(roll(&[Drop { chance: 0.0, item: 1 }], 1.0, seq(&[0.0])).is_empty());
        // 100 %: always (KH1's floats are below 1).
        assert_eq!(roll(&[Drop { chance: 1.0, item: 1 }], 1.0, seq(&[0.999_999])), vec![1]);
    }

    #[test]
    fn lucky_strike_scales_the_chances() {
        assert_eq!(lucky_factor(LUCKY_STRIKES), 1.0, "no Lucky Strike in our abilities yet");
        assert_eq!(lucky_factor(2), 2.0);
        assert_eq!(roll(&AIR_SOLDIER, lucky_factor(1), seq(&[0.029, 0.5, 0.5])), vec![1]);
        assert!(roll(&AIR_SOLDIER, lucky_factor(0), seq(&[0.029, 0.5, 0.5])).is_empty());
    }

    #[test]
    fn only_battle_items_become_prizes() {
        let o = outcome(&AIR_SOLDIER, 1.0, seq(&[0.0, 0.0, 0.0]));
        assert_eq!(o, vec![Dropped::Spawned(1), Dropped::Spawned(2), Dropped::NotAnItemHere(237)]);
        for i in [1, 2, 3, 4, 6, 7, 8] {
            assert!(spawnable(i));
        }
        for i in [0, 5, 16, 17, 142, 233] {
            assert!(!spawnable(i), "{i}");
        }
        assert_eq!(describe(&AIR_SOLDIER), "potion 2%, hi-potion 1%, #237 4%");
        assert_eq!(describe(&[Drop { chance: 0.0, item: 0 }]), "none");
    }

    #[test]
    fn the_rate_over_many_rolls_is_the_chance() {
        // A uniform sequence of draws: Potion at 2 % drops on 2 % of them.
        let n = 10_000;
        let mut k = 0u32;
        let hits = (0..n).filter(|_| {
            let r = (k as f32 + 0.5) / n as f32;
            k += 1;
            !roll(&AIR_SOLDIER[..1], 1.0, || r).is_empty()
        });
        assert_eq!(hits.count(), 200);
    }

    #[test]
    fn the_descriptor_is_re_coms_layout() {
        assert_eq!(std::mem::size_of::<PrizeDesc>(), 0x30);
        let d = item_prize([1.0, 2.0, 3.0, 1.0], 3, 0.5, 1.0);
        assert_eq!((d.kind, d.arg, d.speed, d.flags), (0xD8, 3, 0.25, 1));
        let a = d.angles;
        assert!((a[0] - (-80f32).to_radians()).abs() < 1e-5 && (a[1] - 180f32.to_radians()).abs() < 1e-5 && a[2] == 0.0 && a[3] == 1.0);
        // Field offsets as the EXP drop writes them (rsp+0x20 pos, +0x30 angles, +0x40 kind, +0x44
        // arg, +0x48 speed, +0x4C flags).
        let base = &d as *const PrizeDesc as usize;
        assert_eq!(&d.kind as *const u32 as usize - base, 0x20);
        assert_eq!(&d.speed as *const f32 as usize - base, 0x28);
        assert_eq!(&d.flags as *const u32 as usize - base, 0x2C);
    }

    #[test]
    fn reward_prizes_are_re_coms_hp_prize() {
        assert_eq!((0..7).map(reward_prize_kind).collect::<Vec<_>>(), vec![None, None, None, Some(0xD3), Some(0xD3), None, None]);
        let d = reward_prize([1.0, 2.0, 3.0, 1.0], HP_PRIZE_KIND, 0.5, 1.0);
        assert_eq!((d.kind, d.arg, d.speed, d.flags, d.angles), (0xD3, 0, 0.25, 1, launch_angles(0.5, 1.0)));
    }

    #[test]
    fn the_look_table_keeps_re_coms_rows_and_adds_the_item_row() {
        let mut recom = [0u8; LOOK_ROWS * LOOK_ROW];
        for (i, b) in recom.iter_mut().enumerate() {
            *b = i as u8;
        }
        let t = look_table(&recom);
        assert_eq!(t.len(), 14 * 16);
        assert_eq!(&t[..13 * 16], &recom[..]);
        // Row 13 (kind 0xD8) is 0xD6's (row 11).
        assert_eq!(&t[13 * 16..], &recom[11 * 16..12 * 16]);
        assert_eq!((KH1_ITEM_KIND - FIRST_PRIZE_KIND) as usize, 13);
    }

    #[test]
    fn lea_retargets_within_reach() {
        // Re:CoM's own: 0x140231676 `48 8D 05 D3 1A 38 00` → 0x1405B3150 (Checked (exe)).
        assert_eq!(lea_target(site::LEA_INIT, &[0x48, 0x8D, 0x05, 0xD3, 0x1A, 0x38, 0x00]), Some(site::LOOK_TABLE));
        assert_eq!(lea_target(site::LEA_LOOK, &[0x48, 0x8D, 0x05, 0x49, 0x2A, 0x38, 0x00]), Some(site::LOOK_TABLE));
        assert_eq!(lea_rax(site::LEA_INIT, site::LOOK_TABLE), Some([0x48, 0x8D, 0x05, 0xD3, 0x1A, 0x38, 0x00]));
        let below = site::LEA_INIT - 0x1000_0000;
        assert_eq!(lea_target(site::LEA_INIT, &lea_rax(site::LEA_INIT, below).unwrap()), Some(below));
        assert_eq!(lea_rax(site::LEA_INIT, site::LEA_INIT + 0x1_0000_0000), None);
        assert_eq!(lea_target(0, &[0x48, 0x8B, 0x05, 0, 0, 0, 0]), None);
    }
}
