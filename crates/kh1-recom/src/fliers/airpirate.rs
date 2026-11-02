//! M11: the KH1 Air Pirate's class row (`docs/recom-m11-enemies.md` § The Air Pirate).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::airpirate`).
pub const MANIFEST: &str = "kh1-airpirate.json";

/// The attack names (`kh1_core::airpirate::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["swoop", "slash"];

/// The requests the behaviour plays (its manifest must map them).
pub const REQUESTS: &[u16] = {
    use kh1_core::airpirate::req::*;
    &[IDLE, FLINCH, KNOCK, APPEAR, SWOOP, TAKE_OFF, FLY, HALT, RISE, DIVE_START, DIVE, DIVE_END, BANK_L, BANK_R, WINDUP, DIVE_HIT, SLASH, RECOIL]
};

/// `CEnemyAirpirate` (Re:CoM id 22). Checked (exe + decomp): vtable `0x1405309E8` (139 slots, RTTI at
/// −8), ctor `0x14018A0B0` (`CHeartless`'s base ctor, then `+0x154 |= 0x80`, `+0x260 |= 0x100`: gravity
/// off, `+0xCC0 |= 1`), allocation `0xCD0` at `0x14018931F` (the factory's case 22). Its own states: 1
/// (the flying walk, `0x14018A310` → `0x1401979F0` with `e022_001` / `e022_002`, step `0x14018A4D0` →
/// `0x140197450`), 2 (`e022_050`, `0x14018A270` / `0x14018A370`), 3 (`e022_051`, `0x14018A2C0` /
/// `0x14018A420`); its own think `0x14018A190`. Re:CoM's `e022_050` is an edit of KH1's slash 212 (KH1's
/// matcher, Checked (files)): KH1's slash runs in state 2, its swoop 200 in state 3.
pub static CLASS: Class = Class {
    name: "airpirate",
    title: "Air Pirate",
    rtti: ".?AVCEnemyAirpirate@@",
    recom_id: 22,
    vtable: 0x0053_09E8,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x5041_314B, // "K1AP"
    ctor: &crate::sites::AIRPIRATE_HOOKS[0],
    alloc_site: &crate::sites::AIRPIRATE_ALLOC[0],
    ptrs: crate::sites::AIRPIRATE_PTRS,
    attack_states: &[3, 2],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    parse(text, "airpirate", kh1_core::airpirate::NAMES, REQUESTS)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::airpirate::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_AIRPIRATE_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_AIRPIRATE_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!((m.consts.get("charge_speed"), m.consts.get("dive_above")), (24.0, 320.0));
        // The swoop's recoil 213 is a motion of the model; the slash's 110 is not (Re:CoM's flinch,
        // owner 2026-11-02).
        assert_eq!((m.recoil_of(0), m.recoil_of(1)), (Some(213), None));
        assert_eq!(m.parts.len(), 7);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::airpirate::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 3, 2, 11, 12, 30]);
        assert_eq!(&CLASS.magic.to_le_bytes(), b"K1AP");
        // Every vtable slot the copy replaces is checked against Re:CoM's (begin and step).
        for s in crate::kh1_enemy::replaced_states(&CLASS) {
            let at = CLASS.vtable + crate::kh1_enemy::begin_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at), "state {s} begin");
        }
    }
}
