//! M11: the KH1 Red Nocturne's class row (`docs/recom-m11-enemies.md` § The Red Nocturne).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::rednocturne`).
pub const MANIFEST: &str = "kh1-rednocturne.json";

/// No attack of its own (its rows are its two spell children's: `win::kh1_thrown`'s fires).
pub const ATTACKS: &[&str] = &[];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): its reactions (flinch
/// 72..75, knock-back 76 / 77, air flinch 78, air knock-back 80 / 81).
pub const REQUESTS: &[u16] = {
    use kh1_core::rednocturne::req::*;
    &[IDLE, HOVER, APPEAR, FLINCH, KNOCK, AIR_FLINCH, AIR_KNOCK, CHARGE, CAST, BIG_CHARGE, BIG_CAST, NOTICE, FLEE, WANDER, REST]
};

/// The numbers the host reads for its fires (`win::kh1_thrown`): each child's start, flight, Re:CoM power and radii, its
/// index; its own rows' serial range (the hit handler's `sys 188`).
pub const HOST_NAMES: &[&str] = &[
    "fire_child",
    "fire_t",
    "fire_up",
    "fire_turn",
    "fire_speed",
    "fire_life",
    "fire_radius",
    "fire_recom_id",
    "fire_kind",
    "fire_bat_radius",
    "bigfire_child",
    "bigfire_t",
    "bigfire_up",
    "bigfire_turn",
    "bigfire_speed",
    "bigfire_life",
    "bigfire_radius",
    "bigfire_recom_id",
    "bigfire_kind",
    "bigfire_bat_radius",
    "own_row_first",
    "own_row_last",
];

/// `CEnemyRednocturne` (Re:CoM id 3). Checked (exe + decomp): vtable `0x140548A58` (139 slots, RTTI COL at −8), ctor
/// `0x140191880` (`CHeartless`'s base ctor `0x140196DF0`, then `+0x154 |= 0x80`, `+0x260 |= 0x100`: gravity off; its only
/// caller the factory `0x140188F40` at `0x14018901A`), allocation `0xCD0` at `0x140189001`. Its own states: 1 (the flying
/// walk `0x1401919F0` → `0x1401979F0` with `e003_001` / `e003_002`, step `0x140191D80` → `0x140197450`, end
/// `0x140191BF0` → `0x1401979C0`), 2 (its small shot `e003_051`: at clip time 0x79D a `CBullet` from its own aim point
/// at Sora's, `0x14017F0E0(enemy, 0, 1)`'s AP, effect set `0x140548EF0`; `0x140191950` / `0x140191C00`), 3 (its big shot
/// `e003_053`: at 0x1035, id 1, effect set `0x140548F60`; `0x1401919A0` / `0x140191CC0`); states 0 / 11 / 12 / 30
/// `CBattleEnemy`'s; think `CHeartless`'s `0x140196EC0`. KH1's Red Nocturne has no attack of its own: every motion plays
/// in state 1, its fires are Re:CoM bullets made from state 1 (`win::kh1_thrown`); states 2 and 3 are never entered.
pub static CLASS: Class = Class {
    name: "rednocturne",
    title: "Red Nocturne",
    rtti: ".?AVCEnemyRednocturne@@",
    recom_id: 3,
    vtable: 0x0054_8A58,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x4E52_314B, // "K1RN"
    ctor: &crate::sites::REDNOCTURNE_HOOKS[0],
    alloc_site: &crate::sites::REDNOCTURNE_ALLOC[0],
    ptrs: crate::sites::REDNOCTURNE_PTRS,
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
    let m = parse(text, "rednocturne", &kh1_core::rednocturne::all_names(), REQUESTS)?;
    for n in HOST_NAMES {
        if !m.consts.get(n).is_finite() {
            return Err(format!("constant {n} missing (an older asset step? run it again)"));
        }
    }
    if !m.recoil.is_empty() {
        return Err("the Red Nocturne has no attack of its own".into());
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::rednocturne::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh1_core::enemy::host::model::Model;
    use kh1_core::rednocturne::{child, req};
    use kh1_core::Vec3;

    fn manifest() -> Option<Manifest> {
        let p = std::env::var_os("KH1_REDNOCTURNE_JSON")?;
        Some((CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap())
    }

    /// The asset step's real manifest (`KH1_REDNOCTURNE_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(m) = manifest() else { return };
        assert_eq!((m.consts.get("close_range"), m.consts.get("strafe_speed"), m.consts.get("fire_speed"), m.consts.get("bigfire_speed")), (500.0, 8.4, 12.0, 22.0));
        assert_eq!((m.consts.get("fire_radius"), m.consts.get("bigfire_radius"), m.consts.get("fire_bat_radius"), m.consts.get("fire_recom_id"), m.consts.get("bigfire_recom_id")), (30.0, 40.0, 60.0, 0.0, 1.0));
        assert!(m.recoil.is_empty() && m.guards.is_empty() && !m.has_never_hurting());
        assert_eq!((m.clip(req::CAST).unwrap().frames, m.clip(req::BIG_CAST).unwrap().frames, m.clip(req::APPEAR).unwrap().frames), (116, 200, 90));
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    /// The behaviour through the host's rules with the real manifest: a 20 000-frame fight, 12 seeds (Sora circling
    /// 1.5..15.5 m away): nothing released, no attack state, both fires cast, never quiet for 600 frames.
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
            assert!(model.trace.iter().all(|t| t.0 == 1), "seed {seed}: an attack state");
            for n in [child::FIRE, child::BIG_FIRE] {
                assert!(model.log.iter().any(|(_, c)| *c == ke::Cmd::Throw(n)), "seed {seed}: no fire {n}");
            }
        }
    }

    #[test]
    fn row() {
        assert!(CLASS.attack_names.is_empty() && CLASS.attack_states.is_empty());
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 11, 12, 30]);
        assert_eq!(&CLASS.magic.to_le_bytes(), b"K1RN");
        for s in crate::kh1_enemy::replaced_states(&CLASS) {
            let at = CLASS.vtable + crate::kh1_enemy::begin_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at), "state {s} begin");
            let at = CLASS.vtable + crate::kh1_enemy::step_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at) || s == 11 || s == 12, "state {s} step");
        }
    }
}
