//! M11: the KH1 Powerwild's class row (`docs/recom-m11-enemies.md` § The Powerwild).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::powerwild`).
pub const MANIFEST: &str = "kh1-powerwild.json";

/// The attack names (`kh1_core::powerwild::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["swipe", "flurry", "pounce"];

/// The requests the behaviour plays (its manifest must map them).
pub const REQUESTS: &[u16] = {
    use kh1_core::powerwild::req::*;
    &[IDLE, WALK, RUN, JUMP, FALL, AIR, LAND, APPEAR, STRAFE_A, STRAFE_B, HOP_LEFT, 203, 204, 205, HOP_RIGHT, 208, 209, 210, HOP_BACK, 216, 217, 218, SWIPE, FLURRY, POUNCE, STRETCH, SHAKE, NOTICE, ROAR, OWN_LAUNCH, OWN_AIR_LAUNCH]
};

/// `CEnemyPowerwild` (Re:CoM id 7). Checked (exe): vtable `0x140547798` (139 slots, RTTI at −8),
/// ctor `0x140191380` (only caller the factory `0x140188F40` at `0x1401890C2`), allocation `0xCD0` at
/// `0x1401890A9`. Its own states: 1 (`0x1401914F0` / `0x140191870`), 2 (its swipe `e007_051`, `.ASD`
/// attack id 0, `0x140191450` / `0x140191520`), 3 (its flurry `e007_050`, id 1, `0x1401914A0` /
/// `0x1401915F0`); states 0 / 11 / 12 / 30 are `CBattleEnemy`'s. KH1's swipe 212 runs in state 2,
/// the flurry 214 and the pounce 222 in state 3 (the power each takes:
/// `kh1_recom_assets::powerwild`).
pub static CLASS: Class = Class {
    name: "powerwild",
    title: "Powerwild",
    rtti: ".?AVCEnemyPowerwild@@",
    recom_id: 7,
    vtable: 0x0054_7798,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x5750_314B, // "K1PW"
    ctor: &crate::sites::POWERWILD_HOOKS[0],
    alloc_site: &crate::sites::POWERWILD_ALLOC[0],
    ptrs: crate::sites::POWERWILD_PTRS,
    attack_states: &[2, 3, 3],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: true,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    parse(text, "powerwild", kh1_core::powerwild::NAMES, REQUESTS)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::powerwild::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_POWERWILD_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_POWERWILD_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!(m.consts.get("close_range"), 300.0);
        assert_eq!((m.recoil_of(0), m.recoil_of(1), m.recoil_of(2)), (Some(226), Some(225), Some(229)));
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::powerwild::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 2, 3, 11, 12, 30]);
    }
}
