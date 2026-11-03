//! M11: the KH1 Gargoyle's class row (`docs/recom-m11-enemies.md` § The Gargoyle).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::gargoyle`).
pub const MANIFEST: &str = "kh1-gargoyle.json";

/// The attack names (`kh1_core::gargoyle::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["dive", "swipe", "swipe end"];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): its reactions (flinch
/// 72..75, knock-back 76 / 77, air knock-back 80 / 81) and the swipes' recoils 215 / 216.
pub const REQUESTS: &[u16] = {
    use kh1_core::gargoyle::req::*;
    &[IDLE, FLINCH, KNOCK, AIR_KNOCK, FLY, HOVER, STRAFE_A, DIVE, VANISH_NEAR, APPEAR, VANISH_FAR, APPEAR_FAR, BACK, BACK_IN, SWIPE, ROAR, STRAFE_B, SWIPE_END, BALL, 215, 216]
};

/// `CEnemyGargoyle` (Re:CoM id 13). Checked (exe + decomp): vtable `0x14053F198` (139 slots, RTTI COL at −8), ctor
/// `0x14018E150` (`CHeartless`'s base ctor `0x140196DF0`, then `+0x154 |= 0x80`, `+0x260 |= 0x100`: gravity off; its
/// only caller the factory `0x140188F40` at `0x1401891BE`), allocation `0xCD0` at `0x1401891A5`. Its own states: 1
/// (the flying walk `0x14018E2C0` → `0x1401979F0` with `e013_001` / `e013_002`, step `0x14018E560` → `0x140197450`, end
/// `0x14018E300` → `0x1401979C0`), 2 (its wing strike `e013_050`, `.ASD` attack id 0, `0x14018E220` / `0x14018E310`), 3
/// (its shot `e013_051`, `0x14018E270` / `0x14018E350`: at clip time 3150 a `CBullet` from `chin` with
/// `0x14017F0E0(enemy, 1, 1, 0)`'s AP, 0.25 m a frame homing 2.5° a frame, effect set `0x14053F630`); states 0 / 11 / 12 /
/// 30 `CBattleEnemy`'s; think `CHeartless`'s `0x140196EC0`. KH1's three attacks (all plain requests with rows, steered)
/// run in state 2; its ball is a Re:CoM bullet made from state 1 (`win::kh1_thrown`); state 3 is never entered.
pub static CLASS: Class = Class {
    name: "gargoyle",
    title: "Gargoyle",
    rtti: ".?AVCEnemyGargoyle@@",
    recom_id: 13,
    vtable: 0x0053_F198,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x4147_314B, // "K1GA"
    ctor: &crate::sites::GARGOYLE_HOOKS[0],
    alloc_site: &crate::sites::GARGOYLE_ALLOC[0],
    ptrs: crate::sites::GARGOYLE_PTRS,
    attack_states: &[2, 2, 2],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let m = parse(text, "gargoyle", &kh1_core::gargoyle::all_names(), REQUESTS)?;
    // The ball's points and Re:CoM shot numbers (the host's, `win::kh1_thrown`).
    for n in ["ball_hold_bone", "ball_hold_x", "ball_hold_y", "ball_hold_z", "ball_at_bone", "ball_at_x", "ball_at_y", "ball_at_z", "ball_radius", "ball_recom_id", "ball_kind", "ball_bat_radius"] {
        if !m.consts.get(n).is_finite() {
            return Err(format!("constant {n} missing (an older asset step? run it again)"));
        }
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::gargoyle::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh1_core::enemy::host::model::Model;
    use kh1_core::Vec3;

    fn manifest() -> Option<Manifest> {
        let p = std::env::var_os("KH1_GARGOYLE_JSON")?;
        Some((CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap())
    }

    /// The asset step's real manifest (`KH1_GARGOYLE_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(m) = manifest() else { return };
        assert_eq!((m.consts.get("close_range"), m.consts.get("strafe_speed"), m.consts.get("ball_life"), m.consts.get("ball_radius")), (250.0, 16.0, 600.0, 30.0));
        // The dive's 110 is no motion of the model (Re:CoM's flinch when Sora guards it); the swipes' 215 / 216 are.
        assert_eq!((m.recoil_of(0), m.recoil_of(1), m.recoil_of(2)), (None, Some(215), Some(216)));
        // The body rows of the swipes never hurt (type 2): one record each.
        assert!(m.has_never_hurting());
        assert_eq!((0..20u16).filter(|&r| m.never_hurts(r)).count(), 2);
        assert!(m.guards.is_empty());
        assert!(!m.clip(219).unwrap().looped && m.clip(219).unwrap().frames == 328);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    /// The behaviour through the host's rules with the real manifest: a 20 000-frame fight, 12 seeds: nothing released,
    /// the attacks only in state 2, no attack asked for again within two frames, balls thrown.
    #[test]
    fn hosted_with_the_real_manifest() {
        let Some(m) = manifest() else { return };
        let frames = m.frames();
        let loops: Vec<u16> = frames.iter().map(|f| f.0).filter(|&r| m.clip(r).is_some_and(|c| c.looped)).collect();
        for seed in 0..12u32 {
            let ai = (CLASS.make)(&m, seed, 0.0);
            let mut model = Model::new(ai, m.consts.clone(), frames.clone(), loops.clone(), CLASS.attack_states.to_vec());
            model.pos = Vec3::new(0.0, -150.0, 0.0);
            for f in 0..20_000usize {
                let a = f as f32 / 600.0;
                let rad = 150.0 + 300.0 * (1.0 + (f as f32 / 1700.0).sin());
                model.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
            }
            assert_eq!(model.released, 0, "seed {seed}");
            for r in [kh1_core::gargoyle::req::DIVE, kh1_core::gargoyle::req::SWIPE, kh1_core::gargoyle::req::SWIPE_END] {
                assert!(!model.attacks_of(r).is_empty(), "seed {seed}: {r}");
                assert!(model.trace.iter().all(|t| t.1 != Some(r) || t.0 == 2), "seed {seed}: {r} outside state 2");
            }
            assert!(model.log.iter().any(|(_, c)| *c == ke::Cmd::Throw(0)), "seed {seed}: no ball");
        }
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::gargoyle::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 2, 11, 12, 30]);
        assert_eq!(&CLASS.magic.to_le_bytes(), b"K1GA");
        for s in crate::kh1_enemy::replaced_states(&CLASS) {
            let at = CLASS.vtable + crate::kh1_enemy::begin_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at), "state {s} begin");
            let at = CLASS.vtable + crate::kh1_enemy::step_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at) || s == 11 || s == 12, "state {s} step");
        }
    }
}
