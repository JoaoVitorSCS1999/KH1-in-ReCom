//! M11: the KH1 Defender's class row (`docs/recom-m11-enemies.md` § The Defender).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::defender`).
pub const MANIFEST: &str = "kh1-defender.json";

/// The attack names (`kh1_core::defender::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["strike", "spin", "bash"];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): its reactions (flinch
/// 72..75, knock-back 76 / 77, air flinch 78 / 79; no launch: the shared reactions fall back) and its own 200..213.
pub const REQUESTS: &[u16] = {
    use kh1_core::defender::req::*;
    &[IDLE, WALK, AIR, LAND, 72, 73, 74, 75, 76, 77, 78, 79, STEP_A, TURN, STEP_B, GUARD_UP, GUARD, GUARD_DOWN, RECOIL, BASH, SPIN, STRIKE, FIRE_UP, FIRE, FLAME, APPEAR]
};

/// `CEnemyDefender` (Re:CoM id 26). Checked (exe): vtable `0x14053CA98` (139 slots, RTTI COL at −8), ctor `0x14018D8E0`
/// (`CHeartless`'s base ctor, then `+0x2E0` = 0.1; only caller the factory `0x140188F40` at `0x1401893E0`; no branch into
/// its first 6 bytes, no pointer to them: exe scan), allocation `0xCD0` at `0x1401893C7`. Its own states: 1
/// (`0x14018DA40` / the walk thunk `0x14018DD60`), 2 (its melee `e026_050`, `.ASD` attack id 0 on `lb_wep0`,
/// `0x14018D9A0` / `0x14018DA90`), 3 (its fireball `e026_051` → `e026_052`, a `CBullet` from `lb_wep0` with attack id 1's
/// power, `0x14018D9F0` / `0x14018DAF0`); states 0 / 11 / 12 / 30 `CBattleEnemy`'s; think `CHeartless`'s `0x140196EC0`.
/// KH1's strike 209, spin 208 and bash 207 (id 0) run in state 2; its fireballs and flame are Re:CoM bullets made from
/// state 1 (`win::kh1_thrown`, `win::kh1_flame`).
pub static CLASS: Class = Class {
    name: "defender",
    title: "Defender",
    rtti: ".?AVCEnemyDefender@@",
    recom_id: 26,
    vtable: 0x0053_CA98,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x4446_314B, // "K1FD"
    ctor: &crate::sites::DEFENDER_HOOKS[0],
    alloc_site: &crate::sites::DEFENDER_ALLOC[0],
    ptrs: crate::sites::DEFENDER_PTRS,
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
    let m = parse(text, "defender", &kh1_core::defender::all_names(), REQUESTS)?;
    let names: Vec<&str> = m.guards.iter().map(|g| g.name.as_str()).collect();
    if names != ["front", "spin"] {
        return Err(format!("guards {names:?}, the behaviour's are [\"front\", \"spin\"] (an older asset step? run it again)"));
    }
    let front = &m.guards[kh1_core::defender::FRONT as usize];
    if !front.always() || front.half_angle().is_none() {
        return Err("guard front: not KH1's mode 3 with a front cone".into());
    }
    let spin = &m.guards[kh1_core::defender::SPIN_GUARD as usize];
    if spin.always() || spin.half_angle().is_none() {
        return Err("guard spin: not a windowed all-round row".into());
    }
    if (m.consts.get("flame_child"), m.consts.get("fire_child")) != (kh1_core::defender::child::FLAME as f32, kh1_core::defender::child::FIREBALL as f32) {
        return Err("children: not the flame 0 and the fireball 1".into());
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::defender::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh1_core::enemy::host::model::Model;
    use kh1_core::Vec3;

    fn manifest() -> Option<Manifest> {
        let p = std::env::var_os("KH1_DEFENDER_JSON")?;
        Some((CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap())
    }

    /// The asset step's real manifest (`KH1_DEFENDER_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(m) = manifest() else { return };
        assert_eq!((m.consts.get("close_range"), m.consts.get("step_deg"), m.consts.get("fire_turn"), m.consts.get("flame_delay")), (470.0, 0.45, 1.0, 110.0));
        // The attack rows' clash recoil 110: not in the model (Re:CoM's flinch when Sora's Guard blocks them, owner).
        assert_eq!(m.recoil_of(0), None);
        assert_eq!(m.parts.len(), 14);
        let (f, s) = (&m.guards[0], &m.guards[1]);
        assert_eq!((f.priority, f.clash_type, s.priority, s.clash_type), (99, 1, 100, 1));
        assert!(f.always() && f.live(ke::guard::ALWAYS, 0.0) && !s.always());
        assert!((f.half_angle().unwrap().to_degrees() - 70.0).abs() < 0.01);
        assert!((s.half_angle().unwrap().to_degrees() - 180.0).abs() < 0.01);
        assert!(s.live(208, 50.0) && !s.live(208, 117.0));
        let p: Vec<(u8, u32, i32)> = m.clash.iter().map(|c| (c.attack, c.clash_type, c.priority)).collect();
        assert_eq!(p, vec![(0, 0, 100), (1, 0, 100), (2, 0, 100)]);
        assert!(!m.clip(208).unwrap().looped && m.clip(209).unwrap().root && m.clip(212).unwrap().looped);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    /// The behaviour through the host's rules with the real manifest (its lengths, its loops, its attack-state map): a
    /// 20 000-frame fight, 12 seeds: nothing released, the strike / spin / bash only in state 2 (or the frame its queued
    /// idle starts), no attack asked for again within two frames, fireballs and flames thrown.
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
                let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
                model.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
            }
            assert_eq!(model.released, 0, "seed {seed}");
            use kh1_core::defender::req::{BASH, SPIN, STRIKE};
            for r in [STRIKE, SPIN, BASH] {
                let a = model.attacks_of(r);
                assert!(a.windows(2).all(|w| w[1] - w[0] > 2), "seed {seed}");
                assert!(model.trace.iter().all(|t| t.1 != Some(r) || t.0 == 2 || t.0 == 1), "seed {seed}: {r}");
            }
            let throws = |n: u8| model.log.iter().filter(|(_, c)| *c == ke::Cmd::Throw(n)).count();
            assert!(throws(0) > 0 && throws(1) > 0, "seed {seed}: flames {} fireballs {}", throws(0), throws(1));
        }
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::defender::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 2, 11, 12, 30]);
    }
}
