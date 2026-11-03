//! M11: the KH1 Large Body's class row (`docs/recom-m11-enemies.md` § The Large Body).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::largebody`).
pub const MANIFEST: &str = "kh1-largebody.json";

/// The attack names (`kh1_core::largebody::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["punch", "slam", "charge", "stomp"];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): the clash recoils 215 /
/// 216 / 218 and KH1's flinch / knock-back too.
pub const REQUESTS: &[u16] = {
    use kh1_core::largebody::req::*;
    &[IDLE, WALK, RUN, AIR, LAND, FLINCH, KNOCK, KNOCK + 1, LAUNCH, APPEAR, PUNCH, WINDUP, CHARGE, CHARGE_HIT, CHARGE_MISS, LOOK, PUNCH_RECOIL, SLAM_RECOIL, CHARGE_RECOIL, RAGE, STOMP, SLAM]
};

/// `CEnemyLargebody` (Re:CoM id 9). Checked (exe): vtable `0x140542AD0` (139 slots, RTTI COL at −8), ctor `0x14018F090`
/// (only caller the factory `0x140188F40` at `0x140189116`; no branch into its first 6 bytes: exe scan), allocation
/// `0xCE0` at `0x1401890FD`. Its own states: 1 (`0x14018F1F0` / `0x14018F3E0`), 2 (its belly bump `e009_050`, `.ASD`
/// attack id 0, `0x14018F150` / `0x14018F220`), 3 (its stomp `e009_053`, id 1, a growing circle record,
/// `0x14018F1A0` / `0x14018F280`); states 0 / 11 / 12 / 30 and the attack states' ends are `CBattleEnemy`'s. KH1's
/// attacks run in the state whose Re:CoM attack id they take their power from (`kh1_recom_assets::largebody`): the
/// punch, the slam and the charge (id 0) in state 2, the stomp's ring (id 1) in state 3.
pub static CLASS: Class = Class {
    name: "largebody",
    title: "Large Body",
    rtti: ".?AVCEnemyLargebody@@",
    recom_id: 9,
    vtable: 0x0054_2AD0,
    slots: 139,
    alloc: 0xCE0,
    magic: 0x424C_314B, // "K1LB"
    ctor: &crate::sites::LARGEBODY_HOOKS[0],
    alloc_site: &crate::sites::LARGEBODY_ALLOC[0],
    ptrs: crate::sites::LARGEBODY_PTRS,
    attack_states: &[2, 2, 2, 3],
    attack_names: ATTACKS,
    appear: Appear::Base,
    // Its rage reads its HP (`0x1C1E`, current / max: Re:CoM's, decision M10).
    reads_hp: true,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let m = parse(text, "largebody", kh1_core::largebody::NAMES, REQUESTS)?;
    let names: Vec<&str> = m.guards.iter().map(|g| g.name.as_str()).collect();
    if names != ["front"] {
        return Err(format!("guards {names:?}, the behaviour's is [\"front\"] (an older asset step? run it again)"));
    }
    if !m.guards[kh1_core::largebody::FRONT as usize].always() {
        return Err("guard front: not KH1's mode 3 (it has windows)".into());
    }
    if m.ring.is_none() {
        return Err("no stomp ring (ring_* numbers)".into());
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::largebody::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_LARGEBODY_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_LARGEBODY_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!((m.consts.get("close_range"), m.consts.get("charge_speed")), (250.0, 12.0));
        // The clash recoils its rows name: 215 (the punch), 216 (the slam), 218 (the charge), all in the model; the
        // ring's 110 is not (Re:CoM's flinch for a guarded stomp ring).
        let r: Vec<Option<u16>> = (0..4).map(|i| m.recoil_of(i)).collect();
        assert_eq!(r, vec![Some(215), Some(216), Some(218), None]);
        assert_eq!(m.parts.len(), 9);
        let g = &m.guards[0];
        assert_eq!((g.priority, g.recoil, g.clash_type), (100, 110, 1));
        assert!(g.always() && g.live(ke::guard::ALWAYS, 0.0));
        assert!((g.half_angle().unwrap().to_degrees() - 70.0).abs() < 0.01);
        let ring = m.ring.unwrap();
        assert_eq!((ring.request, ring.start, ring.life, ring.growth), (230, 88.0, 28.0, 25.0));
        assert_eq!((ring.radius(87.0), ring.radius(88.0), ring.radius(100.0), ring.radius(116.0)), (None, Some(0.0), Some(3.0), None));
        assert!(m.clip(207).unwrap().looped && !m.clip(230).unwrap().looped);
        // M11, attack meets attack: its rows' clash numbers (punch 30 ×2, slam 30, charge 90; the ring a child row).
        let p: Vec<(u8, u32, i32)> = m.clash.iter().map(|c| (c.attack, c.clash_type, c.priority)).collect();
        assert_eq!(p, vec![(0, 0, 30), (0, 0, 30), (1, 0, 30), (2, 0, 90)]);
        assert!(m.clash.iter().all(|c| !c.records.is_empty()));
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::largebody::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 2, 3, 11, 12, 30]);
    }
}
