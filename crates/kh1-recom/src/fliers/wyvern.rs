//! M11: the KH1 Wyvern's class row (`docs/recom-m11-enemies.md` § The Wyvern).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::wyvern`).
pub const MANIFEST: &str = "kh1-wyvern.json";

/// The attack names (`kh1_core::wyvern::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["bite", "dive", "landing", "charge", "charge end"];

/// The requests the behaviour plays (its manifest must map them).
pub const REQUESTS: &[u16] = {
    use kh1_core::wyvern::req::*;
    &[IDLE, FLINCH, KNOCK, APPEAR, BITE, DIVE_START, DIVE, LAND, WINDUP, CHARGE, CHARGE_END, CHASE_START, FLY, BANK_L, BANK_R, HALT, HOVER, ROAR, DIVE_HIT, NOTICE]
};

/// `CEnemyWyvern` (Re:CoM id 24). Checked (exe + decomp): vtable `0x140555DC8` (139 slots, RTTI at −8),
/// ctor `0x140195ED0` (`CHeartless`'s base ctor `0x140196DF0`, then `+0x154 |= 0x80`, `+0x260 |= 0x100`:
/// gravity off, `+0x2E0` = 0.1, `+0xCBC` = 10), allocation `0xCF0` at `0x140189373` (the factory's case 24;
/// its states use fields up to `+0xCE8`). Its own states: 1 (the flying walk, `0x1401960B0` → `0x1401979F0`
/// with `e024_001` / `e024_003`, step `0x1401964E0`: idle, `e024_005` / `e024_006` circling, `e024_004`),
/// 2 (`0x140195FB0` / `0x1401961B0`: `e024_002` → `e024_003` to within 1 m, then the claw `e024_050`), 3
/// (`0x140196060` / `0x140196350`: `e024_051` → `e024_052` at `RushAtkSpd` → `e024_053`); the think is
/// `CHeartless`'s `0x140196EC0`. Re:CoM's `e024_050` is an edit of KH1's bite 200, its rush of KH1's charge
/// (KH1's matcher, Checked (files)): KH1's bite runs in state 2, the body attacks (the dive, its landing, the
/// charge, its end) in state 3.
pub static CLASS: Class = Class {
    name: "wyvern",
    title: "Wyvern",
    rtti: ".?AVCEnemyWyvern@@",
    recom_id: 24,
    vtable: 0x0055_5DC8,
    slots: 139,
    alloc: 0xCF0,
    magic: 0x5957_314B, // "K1WY"
    ctor: &crate::sites::WYVERN_HOOKS[0],
    alloc_site: &crate::sites::WYVERN_ALLOC[0],
    ptrs: crate::sites::WYVERN_PTRS,
    attack_states: &[2, 3, 3, 3, 3],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    parse(text, "wyvern", kh1_core::wyvern::NAMES, REQUESTS)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::wyvern::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_WYVERN_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_WYVERN_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!((m.consts.get("chase_speed"), m.consts.get("dive_above")), (30.0, 400.0));
        // Every row's recoil is 110, which the model doesn't have: Re:CoM's flinch (owner 2026-11-02).
        assert!((0..5).all(|i| m.recoil_of(i).is_none()));
        assert_eq!(m.parts.len(), 11);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::wyvern::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 2, 3, 11, 12, 30]);
        assert_eq!(&CLASS.magic.to_le_bytes(), b"K1WY");
        // Every vtable slot the copy replaces is checked against Re:CoM's (begin and step).
        for s in crate::kh1_enemy::replaced_states(&CLASS) {
            let at = CLASS.vtable + crate::kh1_enemy::begin_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at), "state {s} begin");
            let at = CLASS.vtable + crate::kh1_enemy::step_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at) || s == 11 || s == 12, "state {s} step");
        }
    }
}
