//! M11: the KH1 Bouncywild's class row (`docs/recom-m11-enemies.md` § The Bouncywild).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::bouncywild`).
pub const MANIFEST: &str = "kh1-bouncywild.json";

/// No attack of its own makes rows on its body (`kh1_core::bouncywild::Attack::ALL` is empty): its banana and peels are
/// its children's (Re:CoM bullets, `win::kh1_thrown` / `win::kh1_peel`).
pub const ATTACKS: &[&str] = &[];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): its reactions (flinch
/// 72..75, air flinch 78 / 79, launch 0x52.. and air launch 0x5C.., its own launch's bases; no knock-back: the shared
/// reactions fall back), the leap's 3..6 and its own 69, 200..217.
pub const REQUESTS: &[u16] = {
    use kh1_core::bouncywild::req::*;
    &[IDLE, WALK, RUN, LEAP, LEAP + 1, LEAP + 2, AIR, LAND, 72, 73, 74, 75, 78, 79, LAUNCH, AIR_LAUNCH, APPEAR, TOSS, DANCE, BOUNCE, POSE, TURN, CALL, CALL_END, NOTICE, THROW, RECOVER, TAUNT, TAUNT_END, READY, STRETCH]
};

/// `CEnemyBouncywild` (Re:CoM id 8). Checked (exe): vtable `0x140538FB8` (139 slots, RTTI COL at −8), ctor `0x14018C4D0`
/// (`CHeartless`'s base ctor, then `+0xCC0 |= 2`; only caller the factory `0x140188F40` at `0x1401890EC`; no branch into
/// its first 6 bytes, no pointer to them: exe scan), allocation `0xCD0` at `0x1401890D3`. Its own states: 1
/// (`0x14018C630` / the walk thunk `0x14018CA30`), 2 (`e008_050` → `_051` → `_052`, a straight `CBullet` from `wep0` with
/// attack id 0's power, `0x14018C590` / `0x14018C660`), 3 (`e008_053`, a homing banana from `l_fing1a` with attack id 1's
/// power and Sora's slip kind 0x18, `0x14018C5E0` / `0x14018C870`); states 0 / 11 / 12 / 30 `CBattleEnemy`'s; think
/// `CHeartless`'s `0x140196EC0`. KH1's Bouncywild has no attack state of its own: everything runs in state 1, its banana
/// and peels are Re:CoM bullets made from there.
pub static CLASS: Class = Class {
    name: "bouncywild",
    title: "Bouncywild",
    rtti: ".?AVCEnemyBouncywild@@",
    recom_id: 8,
    vtable: 0x0053_8FB8,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x5742_314B, // "K1BW"
    ctor: &crate::sites::BOUNCYWILD_HOOKS[0],
    alloc_site: &crate::sites::BOUNCYWILD_ALLOC[0],
    ptrs: crate::sites::BOUNCYWILD_PTRS,
    attack_states: &[],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let m = parse(text, "bouncywild", &kh1_core::bouncywild::all_names(), REQUESTS)?;
    use kh1_core::bouncywild::child;
    let c = |k: &str| m.consts.get(k);
    if (c("fire_child"), c("peel_child"), c("drop_child")) != (child::BANANA as f32, child::PEEL as f32, child::DROP as f32) {
        return Err("children: not the banana 0, the tossed peel 1 and the dropped peel 2".into());
    }
    // The host starts all three at the banana's point (bone code 28 at t 1).
    if c("peel_t") != c("fire_t") || c("drop_t") != c("fire_t") {
        return Err(format!("the children's start points differ (t {} / {} / {})", c("fire_t"), c("peel_t"), c("drop_t")));
    }
    if !(c("peel_count") >= 1.0 && c("peel_count") <= 8.0 && c("drop_count") >= 1.0 && c("drop_count") <= 8.0) {
        return Err(format!("peel counts {} / {}", c("peel_count"), c("drop_count")));
    }
    if !m.clash.is_empty() || !m.guards.is_empty() {
        return Err("the Bouncywild has no attack or guard rows".into());
    }
    Ok(m)
}

/// Re:CoM's slip: the reaction kind its own Bouncywild's homing banana carries (`0x14018C870`, descriptor `+0x68`, the write
/// at `0x14018C924`), which the player damage check `0x1401E6DE0` plays as Sora's slip (case 0x18: `+0x414` = 6, row 0x29
/// `p001_049`, then `0x1401F6190`, a tenth of his Moogle points scattered as prizes). Checked (decomp + exe).
pub const SLIP_KIND: u32 = 0x18;

/// The owner's rule (2026-11-03, "a slip loses nothing"): a slip from one of our KH1 Bouncywild's peels or bananas (`ours`:
/// the pending hit's attacker, Sora's body `+0x98`, is one of their bullets) plays Re:CoM's slip without its Moogle point
/// scatter. Only Re:CoM's slip kind with a hit pending (the damage check reads `+0x710` only past its hit-pending test
/// `+0x720`); Re:CoM's own Bouncywild's banana (not ours) and every other kind keep Re:CoM's scatter.
pub fn slip_keeps_points(pending: bool, kind: u32, ours: bool) -> bool {
    pending && kind == SLIP_KIND && ours
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::bouncywild::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh1_core::enemy::host::model::Model;
    use kh1_core::Vec3;

    fn manifest() -> Option<Manifest> {
        let p = std::env::var_os("KH1_BOUNCYWILD_JSON")?;
        Some((CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap())
    }

    /// The asset step's real manifest (`KH1_BOUNCYWILD_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(m) = manifest() else { return };
        assert_eq!((m.consts.get("rounds"), m.consts.get("leap_speed"), m.consts.get("fire_turn"), m.consts.get("peel_lie")), (5.0, 33.0, 1.25, 120.0));
        assert_eq!((m.recoil.len(), m.parts.len()), (0, 6));
        assert!(m.clip(211).is_some_and(|c| c.frames > 82) && m.clip(200).is_some_and(|c| c.frames > 80));
        assert!(m.clip(203).unwrap().looped && !m.clip(4).unwrap().looped);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    /// The behaviour through the host's rules with the real manifest (its lengths, its loops, no attack state): a
    /// 20 000-frame fight, 12 seeds, Sora keeping 2–11 m from it (it never comes to him: he chases it): nothing
    /// released, banana, peels and drops thrown, every request it plays mapped.
    #[test]
    fn hosted_with_the_real_manifest() {
        let Some(m) = manifest() else { return };
        let frames = m.frames();
        let loops: Vec<u16> = frames.iter().map(|f| f.0).filter(|&r| m.clip(r).is_some_and(|c| c.looped)).collect();
        for seed in 0..12u32 {
            let ai = (CLASS.make)(&m, seed, 0.0);
            let mut model = Model::new(ai, m.consts.clone(), frames.clone(), loops.clone(), CLASS.attack_states.to_vec());
            for f in 0..20_000usize {
                let a = f as f32 / 600.0;
                let rad = 200.0 + 450.0 * (1.0 + (f as f32 / 1700.0).sin());
                let p = model.pos;
                model.run(Vec3::new(p.x + a.sin() * rad, 0.0, p.z + a.cos() * rad), 1);
            }
            assert_eq!(model.released, 0, "seed {seed}");
            let throws = |n: u8| model.log.iter().filter(|(_, c)| *c == ke::Cmd::Throw(n)).count();
            assert!(throws(0) > 0 && throws(1) > 0 && throws(2) > 0, "seed {seed}: bananas {} peels {} drops {}", throws(0), throws(1), throws(2));
            for (_, c) in &model.log {
                if let ke::Cmd::Motion { request, .. } | ke::Cmd::Action { request, .. } | ke::Cmd::Restart { request, .. } | ke::Cmd::Queue { request, .. } = c {
                    assert!(m.clip(*request).is_some(), "seed {seed}: request {request} has no clip");
                }
            }
        }
    }

    /// Only a pending slip from our KH1 Bouncywild's bullets keeps Sora's Moogle points.
    #[test]
    fn a_slip_loses_nothing_only_on_our_bullets() {
        assert!(slip_keeps_points(true, SLIP_KIND, true));
        assert!(!slip_keeps_points(true, SLIP_KIND, false), "Re:CoM's own Bouncywild's banana: Re:CoM's scatter");
        assert!(!slip_keeps_points(false, SLIP_KIND, true), "no hit pending: the check never reaches its switch");
        assert!(!slip_keeps_points(true, 1, true), "a flinch: no scatter call to skip");
        assert!(!slip_keeps_points(true, 0x27, true) && !slip_keeps_points(true, 0x28, true), "Re:CoM's other scatter kinds");
    }

    #[test]
    fn row() {
        assert!(CLASS.attack_names.is_empty() && kh1_core::bouncywild::Attack::ALL.is_empty());
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 11, 12, 30]);
    }
}
