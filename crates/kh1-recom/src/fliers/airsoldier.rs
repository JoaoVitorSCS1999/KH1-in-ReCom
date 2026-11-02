//! M11: the KH1 Air Soldier's class row (`docs/recom-m11-enemies.md` § The Air Soldier).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::airsoldier`).
pub const MANIFEST: &str = "kh1-airsoldier.json";

/// The attack names (`kh1_core::airsoldier::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["swoop kick", "slash", "swoop", "spin", "charge"];

/// The requests the behaviour plays (its manifest must map them).
pub const REQUESTS: &[u16] = {
    use kh1_core::airsoldier::req::*;
    &[IDLE, FLY, FLINCH, KNOCK, APPEAR, KICK, SLASH, SWOOP, SPIN, CHARGE_UP, CHARGE, CHARGE_HIT, CHARGE_MISS, BACK, STRAFE_L, STRAFE_R, POWER, POWER_FLY, POWER_END, CLIMB, CLIMB_LOOP, CLIMB_END, TUCK, TUCK_LOOP, TUCK_END]
};

/// `CEnemyAirsoldier` (Re:CoM id 15). Checked (exe + decomp): vtable `0x140531CC8` (139 slots, RTTI
/// at −8), ctor `0x14018A4E0` (`CHeartless`'s base ctor, then `+0x154 |= 0x80`, `+0x260 |= 0x100`
/// (gravity off), `+0xCC0 |= 1`), allocation `0xCD0` at `0x1401891F9`. Its own states: 1 (the flying
/// walk / think, `0x1401979F0`), 2 (its kick `e015_050`, `0x14018A6A0` / `0x14018A7D0`), 3 (its dash
/// punch `e015_051`, `0x14018A6F0` / `0x14018A8D0`); its think `0x14018A5C0`. KH1's foot attacks (the
/// swoop, the spin) run in state 2, the hand attacks and the charge in state 3 (the power each takes:
/// `kh1_recom_assets::airsoldier`).
pub static CLASS: Class = Class {
    name: "airsoldier",
    title: "Air Soldier",
    rtti: ".?AVCEnemyAirsoldier@@",
    recom_id: 15,
    vtable: 0x0053_1CC8,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x5341_314B, // "K1AS"
    ctor: &crate::sites::AIRSOLDIER_HOOKS[0],
    alloc_site: &crate::sites::AIRSOLDIER_ALLOC[0],
    ptrs: crate::sites::AIRSOLDIER_PTRS,
    attack_states: &[3, 3, 2, 2, 3],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: true,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    parse(text, "airsoldier", kh1_core::airsoldier::NAMES, REQUESTS)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::airsoldier::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_AIRSOLDIER_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_AIRSOLDIER_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!(m.consts.get("close_range"), 300.0);
        assert_eq!((m.recoil_of(0), m.recoil_of(1), m.recoil_of(4)), (Some(206), Some(205), Some(205)));
        assert_eq!(m.parts.len(), 5);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::airsoldier::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 3, 2, 11, 12, 30]);
    }
}
