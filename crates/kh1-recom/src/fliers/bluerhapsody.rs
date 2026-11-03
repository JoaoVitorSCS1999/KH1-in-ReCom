//! M11: the KH1 Blue Rhapsody's class row (`docs/recom-m11-enemies.md` § The Blue Rhapsody).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::bluerhapsody`).
pub const MANIFEST: &str = "kh1-bluerhapsody.json";

/// The attack names (`kh1_core::bluerhapsody::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["melee"];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): its hops (3..6), its
/// reactions (flinch 72..75, knock-back 76 / 77, air flinch 78, air knock-back 80 / 81) and its melee's recoil 216.
pub const REQUESTS: &[u16] = {
    use kh1_core::bluerhapsody::req::*;
    &[IDLE, CROUCH, RISE, APEX, DROP, CAST, APPEAR, FLINCH, KNOCK, AIR_FLINCH, AIR_KNOCK, NOTICE, MELEE, MELEE_RECOIL, WANDER, REST]
};

/// The numbers the host reads for its Blizzard (`win::kh1_thrown`): its child index, its start point (bone code 0's
/// parent, on Re:CoM's bone), flight, Re:CoM power and radius.
pub const HOST_NAMES: &[&str] = &[
    "blizzard_child",
    "blizzard_at_bone",
    "blizzard_at_x",
    "blizzard_at_y",
    "blizzard_at_z",
    "blizzard_speed",
    "blizzard_life",
    "blizzard_radius",
    "blizzard_recom_id",
    "blizzard_kind",
];

/// `CEnemyBluerhapsody` (Re:CoM id 4). Checked (exe + decomp): vtable `0x140537CA8` (139 slots, RTTI COL at −8), ctor
/// `0x14018C120` (`CHeartless`'s base ctor `0x140196DF0`, then `+0x154 |= 0x80`, `+0x260 |= 0x100`, `+0xCC0 |= 4`; its only
/// caller the factory `0x140188F40` at `0x140189044`), allocation `0xCD0` at `0x14018902B`. Its own states: 1 (the flying
/// walk `0x14018C280` → `0x140198180` with `e003_001` / `e003_002`, step `0x14018C460` (a 0.15 m/frame bounce off the floor
/// in its walk phases 2 / 5 / 8) → `0x140197AD0`, end `0x14018C2C0` → `0x140198150`), 2 (its Blizzard shot `e004_050`:
/// begin `0x14018C200`, step `0x14018C2D0`: past clip time 0x1099 a `CBullet` from its aim point, `0x14017F0E0(enemy, 0,
/// 1)`'s AP, 0.4 m a frame, radius 0.2, effect set `0x1407C2F10`), 3 (its body attack `e004_051`: begin `0x14018C250`,
/// step `0x14018C430`; its `.ASD` record, attack id 1, on `ante2` from frame 75 to 120); states 0 / 11 / 12 / 30
/// `CBattleEnemy`'s; think `CHeartless`'s `0x140196EC0`. KH1's melee (a plain request with rows, steered) runs in state 3;
/// its Blizzard is a Re:CoM bullet made from state 1 (`win::kh1_thrown`); state 2 is never entered.
pub static CLASS: Class = Class {
    name: "bluerhapsody",
    title: "Blue Rhapsody",
    rtti: ".?AVCEnemyBluerhapsody@@",
    recom_id: 4,
    vtable: 0x0053_7CA8,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x5242_314B, // "K1BR"
    ctor: &crate::sites::BLUERHAPSODY_HOOKS[0],
    alloc_site: &crate::sites::BLUERHAPSODY_ALLOC[0],
    ptrs: crate::sites::BLUERHAPSODY_PTRS,
    attack_states: &[3],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let m = parse(text, "bluerhapsody", &kh1_core::bluerhapsody::all_names(), REQUESTS)?;
    for n in HOST_NAMES {
        if !m.consts.get(n).is_finite() {
            return Err(format!("constant {n} missing (an older asset step? run it again)"));
        }
    }
    if m.recoil.len() != ATTACKS.len() {
        return Err(format!("{} attacks in the manifest, the Blue Rhapsody has {}", m.recoil.len(), ATTACKS.len()));
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::bluerhapsody::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh1_core::bluerhapsody::{child, req};
    use kh1_core::enemy::host::model::Model;
    use kh1_core::Vec3;

    fn manifest() -> Option<Manifest> {
        let p = std::env::var_os("KH1_BLUERHAPSODY_JSON")?;
        Some((CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap())
    }

    /// The asset step's real manifest (`KH1_BLUERHAPSODY_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(m) = manifest() else { return };
        assert_eq!((m.consts.get("close_range"), m.consts.get("mid_range"), m.consts.get("hop_speed"), m.consts.get("rise_up")), (250.0, 500.0, 30.0, 35.0));
        assert_eq!((m.consts.get("blizzard_wait"), m.consts.get("blizzard_speed"), m.consts.get("blizzard_life"), m.consts.get("blizzard_radius"), m.consts.get("blizzard_recom_id"), m.consts.get("blizzard_kind")), (84.0, 25.0, 60.0, 40.0, 0.0, 1.0));
        // The melee's KH1 recoil 216 is a motion of the model; its body row (type 2) never hurts: one record.
        assert_eq!(m.recoil_of(0), Some(req::MELEE_RECOIL));
        assert!(m.has_never_hurting() && m.guards.is_empty());
        assert_eq!((m.clip(req::CAST).unwrap().frames, m.clip(req::MELEE).unwrap().frames, m.clip(req::APPEAR).unwrap().frames, m.clip(req::CROUCH).unwrap().frames), (148, 148, 90, 16));
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    /// The behaviour through the host's rules with the real manifest: a 20 000-frame fight, 12 seeds (Sora circling
    /// 1.5..15.5 m away): nothing released, the melee only in state 3 and never asked for again within two frames,
    /// Blizzards cast, never quiet for 600 frames.
    #[test]
    fn hosted_with_the_real_manifest() {
        let Some(m) = manifest() else { return };
        let frames = m.frames();
        let loops: Vec<u16> = frames.iter().map(|f| f.0).filter(|&r| m.clip(r).is_some_and(|c| c.looped)).collect();
        for seed in 0..12u32 {
            let ai = (CLASS.make)(&m, seed, 0.0);
            let mut model = Model::new(ai, m.consts.clone(), frames.clone(), loops.clone(), CLASS.attack_states.to_vec());
            model.pos = Vec3::new(0.0, -150.0, 0.0);
            let (mut last, mut gap) = (0usize, 0usize);
            for f in 0..20_000usize {
                let a = f as f32 / 600.0;
                let rad = 150.0 + 700.0 * (1.0 + (f as f32 / 1700.0).sin());
                let n = model.log.len();
                model.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
                if model.log.len() > n || model.moved > 0.0 {
                    gap = gap.max(f - last);
                    last = f;
                }
            }
            assert_eq!(model.released, 0, "seed {seed}");
            assert!(gap < 600, "seed {seed}: quiet {gap}");
            let melee = model.attacks_of(req::MELEE);
            assert!(!melee.is_empty(), "seed {seed}: no melee");
            assert!(melee.windows(2).all(|w| w[1] > w[0] + 2), "seed {seed}: the melee asked again");
            assert!(model.trace.iter().all(|t| t.1 != Some(req::MELEE) || t.0 == 3), "seed {seed}: 215 outside state 3");
            assert!(model.trace.iter().all(|t| t.0 != 2), "seed {seed}: Re:CoM's state 2 entered");
            assert!(model.log.iter().any(|(_, c)| *c == ke::Cmd::Throw(child::BLIZZARD)), "seed {seed}: no Blizzard");
        }
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::bluerhapsody::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 3, 11, 12, 30]);
        assert_eq!(&CLASS.magic.to_le_bytes(), b"K1BR");
        for s in crate::kh1_enemy::replaced_states(&CLASS) {
            let at = CLASS.vtable + crate::kh1_enemy::begin_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at), "state {s} begin");
            let at = CLASS.vtable + crate::kh1_enemy::step_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at) || s == 11 || s == 12, "state {s} step");
        }
    }
}
