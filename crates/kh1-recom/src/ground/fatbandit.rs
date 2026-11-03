//! M11: the KH1 Fat Bandit's class row (`docs/recom-m11-enemies.md` § The Fat Bandit).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::fatbandit`).
pub const MANIFEST: &str = "kh1-fatbandit.json";

/// The attack names (`kh1_core::fatbandit::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["belly"];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): its reactions (flinch
/// 72..75, knock-back 76 / 77, air flinch 78 / 79, air knock-back 80 / 81, launch 82..86; KH1's handler plays them
/// only for the killing hit) and the belly rows' recoil 202.
pub const REQUESTS: &[u16] = {
    use kh1_core::fatbandit::req::*;
    &[IDLE, WALK, RUN, AIR, LAND, APPEAR, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, BELLY, FIRE, BELLY_RECOIL, WINDUP, BREATH, STAGGER]
};

/// `CEnemyFatbandit` (Re:CoM id 10). Checked (exe): vtable `0x14053DDF8` (139 slots, RTTI COL at −8), ctor `0x14018DD70`
/// (`CHeartless`'s base ctor, then `+0x2E0` = 0.1; only caller the factory `0x140188F40` at `0x140189140`; no branch into
/// its first 6 bytes, no pointer to them: exe scan), allocation `0xCD0` at `0x140189127`. Its own states: 1
/// (`0x14018DEB0` / `0x14018E140`), 2 (its belly `e010_050`, `.ASD` attack id 0, `0x14018DE30` / `0x14018DEE0`), 3 (its
/// fireball `e010_051`, a `CBullet` with attack id 1's power, `0x14018DE60` / `0x14018DF20`); states 0 / 11 / 12 / 30
/// `CBattleEnemy`'s; think `CHeartless`'s `0x140196EC0`. KH1's belly 200 (id 0) runs in state 2; its fireballs and
/// flame are Re:CoM bullets made from state 1 (`win::kh1_thrown`, `win::kh1_flame`).
pub static CLASS: Class = Class {
    name: "fatbandit",
    title: "Fat Bandit",
    rtti: ".?AVCEnemyFatbandit@@",
    recom_id: 10,
    vtable: 0x0053_DDF8,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x4246_314B, // "K1FB"
    ctor: &crate::sites::FATBANDIT_HOOKS[0],
    alloc_site: &crate::sites::FATBANDIT_ALLOC[0],
    ptrs: crate::sites::FATBANDIT_PTRS,
    attack_states: &[2],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let m = parse(text, "fatbandit", &kh1_core::fatbandit::all_names(), REQUESTS)?;
    let names: Vec<&str> = m.guards.iter().map(|g| g.name.as_str()).collect();
    if names != ["front"] {
        return Err(format!("guards {names:?}, the behaviour's is [\"front\"] (an older asset step? run it again)"));
    }
    if !m.guards[kh1_core::fatbandit::FRONT as usize].always() {
        return Err("guard front: not KH1's mode 3 (it has windows)".into());
    }
    if m.guards[kh1_core::fatbandit::FRONT as usize].half_angle().is_none() {
        return Err("guard front: a turned cone".into());
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::fatbandit::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh1_core::enemy::host::model::Model;
    use kh1_core::Vec3;

    fn manifest() -> Option<Manifest> {
        let p = std::env::var_os("KH1_FATBANDIT_JSON")?;
        Some((CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap())
    }

    /// The asset step's real manifest (`KH1_FATBANDIT_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(m) = manifest() else { return };
        assert_eq!((m.consts.get("close_range"), m.consts.get("windup_open"), m.consts.get("fire_speed"), m.consts.get("flame_r1")), (300.0, 68.0, 45.0, 60.0));
        // The belly rows' clash recoil 202 (in the model); the front row's 110 is never played (priority 100).
        assert_eq!(m.recoil_of(0), Some(202));
        assert_eq!(m.parts.len(), 10);
        let g = &m.guards[0];
        assert_eq!((g.priority, g.recoil, g.clash_type), (100, 110, 1));
        assert!(g.always() && g.live(ke::guard::ALWAYS, 0.0));
        assert!((g.half_angle().unwrap().to_degrees() - 70.0).abs() < 0.01);
        assert!(!m.clip(207).unwrap().looped && !m.clip(201).unwrap().looped && m.clip(201).unwrap().root);
        let p: Vec<(u8, u32, i32)> = m.clash.iter().map(|c| (c.attack, c.clash_type, c.priority)).collect();
        assert_eq!(p, vec![(0, 0, 30), (0, 0, 30)]);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    /// The behaviour through the host's rules with the real manifest (its lengths, its loops, its attack-state map): a
    /// 20 000-frame fight, 12 seeds: nothing released, the belly only in state 2, no attack asked for again within two
    /// frames, fireballs and flames thrown.
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
            let a = model.attacks_of(kh1_core::fatbandit::req::BELLY);
            assert!(a.windows(2).all(|w| w[1] - w[0] > 2), "seed {seed}");
            assert!(model.trace.iter().all(|t| t.1 != Some(kh1_core::fatbandit::req::BELLY) || t.0 == 2), "seed {seed}");
            let throws = |n: u8| model.log.iter().filter(|(_, c)| *c == ke::Cmd::Throw(n)).count();
            assert!(throws(0) > 0 && throws(1) > 0, "seed {seed}: fireballs {} flames {}", throws(0), throws(1));
        }
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::fatbandit::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 2, 11, 12, 30]);
    }
}
