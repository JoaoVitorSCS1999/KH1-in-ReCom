//! M11: the KH1 Search Ghost's class row (`docs/recom-m11-enemies.md` § The Search Ghost).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::searchghost`).
pub const MANIFEST: &str = "kh1-searchghost.json";

/// The attack names (`kh1_core::searchghost::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["burst", "drain"];

/// The requests the behaviour plays (its manifest must map them).
pub const REQUESTS: &[u16] = {
    use kh1_core::searchghost::req::*;
    &[IDLE, FLINCH, KNOCK, APPEAR, LUNGE, VANISH, SEARCH, DRAIN, BURST, RECOIL, NOTICE]
};

/// `CEnemySearchghost` (Re:CoM id 14). Checked (exe + decomp): vtable `0x14054C408` (139 slots, RTTI at −8), ctor
/// `0x1401923F0` (`CHeartless`'s base ctor `0x140196DF0`, then `+0x154 |= 0x80`, `+0xCD0` = 0 (its drain effect's
/// handle), `+0x260 |= 0x100`: gravity off), allocation `0xCE0` at `0x1401891CF` (the factory's case 14). Its own
/// states: 1 (the flying walk `0x1401925B0` → `0x1401979F0` with `e014_001`, step `0x140192760`), 2
/// (`0x140192510` / `0x1401925F0`: `e014_050`), 3 (`0x140192560` / `0x140192630`: its drain `e014_051`, which
/// heals it 1/8 of its max HP when it lands, `0x140181CE0`, with its effect `ex_2230` 0xB); the think is
/// `CHeartless`'s `0x140196EC0`. Re:CoM's `e014_051` is an edit of KH1's drain 203 (KH1's matcher, Checked
/// (files)): KH1's burst runs in state 2, its drain in state 3 (our copy's state 3: Re:CoM's heal doesn't run).
pub static CLASS: Class = Class {
    name: "searchghost",
    title: "Search Ghost",
    rtti: ".?AVCEnemySearchghost@@",
    recom_id: 14,
    vtable: 0x0054_C408,
    slots: 139,
    alloc: 0xCE0,
    magic: 0x4753_314B, // "K1SG"
    ctor: &crate::sites::SEARCHGHOST_HOOKS[0],
    alloc_site: &crate::sites::SEARCHGHOST_ALLOC[0],
    ptrs: crate::sites::SEARCHGHOST_PTRS,
    attack_states: &[2, 3],
    attack_names: ATTACKS,
    appear: Appear::Base,
    // Its script drains below half its HP (`func_2212`, `0x646` / `0x7B4`): Re:CoM's HP ratio at each hit.
    reads_hp: true,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    parse(text, "searchghost", kh1_core::searchghost::NAMES, REQUESTS)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::searchghost::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_SEARCHGHOST_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_SEARCHGHOST_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!((m.consts.get("cone_range"), m.consts.get("sight_bone"), m.consts.get("hold_time")), (300.0, 32.0, 568.0));
        // Both rows name the recoil 205, which the model has: KH1's clash recoil when Sora guards.
        assert_eq!((m.recoil_of(0), m.recoil_of(1)), (Some(205), Some(205)));
        assert_eq!(m.parts.len(), 5);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::searchghost::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 2, 3, 11, 12, 30]);
        assert_eq!(&CLASS.magic.to_le_bytes(), b"K1SG");
        // Every vtable slot the copy replaces is checked against Re:CoM's (begin and step).
        for s in crate::kh1_enemy::replaced_states(&CLASS) {
            let at = CLASS.vtable + crate::kh1_enemy::begin_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at), "state {s} begin");
            let at = CLASS.vtable + crate::kh1_enemy::step_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at) || s == 11 || s == 12, "state {s} step");
        }
    }
}
