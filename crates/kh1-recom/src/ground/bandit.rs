//! M11: the KH1 Bandit's class row (`docs/recom-m11-enemies.md` § The Bandit).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::bandit`).
pub const MANIFEST: &str = "kh1-bandit.json";

/// The attack names (`kh1_core::bandit::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["slash", "spin", "spin end", "leap"];

/// The requests the behaviour plays (its manifest must map them).
pub const REQUESTS: &[u16] = {
    use kh1_core::bandit::req::*;
    &[IDLE, WALK, RUN, AIR, LAND, APPEAR, SLASH, THROW, SPIN_UP, SPIN, SPIN_END, LEAP_CROUCH, LEAP, LEAP_SLASH, HOP, 222, 223, 224, STANCE, STANCE_HOLD, STANCE_END]
};

/// `CEnemyBandit` (Re:CoM id 11). Checked (exe): vtable `0x140534368` (139 slots, RTTI COL at −8),
/// ctor `0x14018AEB0` (only caller the factory `0x140188F40` at `0x14018916A`; no branch into its
/// first 6 bytes: exe scan), allocation `0xCD0` at `0x140189151`. Its own states: 1
/// (`0x14018AFF0` / `0x14018B3A0`), 2 (its slash `e011_050`, `.ASD` attack id 0, `0x14018AF70` /
/// `0x14018B120`), 3 (its whirl `e011_051` → `052` → `053`, id 1, `0x14018AFA0` / `0x14018B160`);
/// states 0 / 11 / 12 / 30 are `CBattleEnemy`'s. KH1's slash 200 runs in state 2, the spin (211,
/// 212) and the leap 217 in state 3 (the power each takes: `kh1_recom_assets::bandit`).
pub static CLASS: Class = Class {
    name: "bandit",
    title: "Bandit",
    rtti: ".?AVCEnemyBandit@@",
    recom_id: 11,
    vtable: 0x0053_4368,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x4442_314B, // "K1BD"
    ctor: &crate::sites::BANDIT_HOOKS[0],
    alloc_site: &crate::sites::BANDIT_ALLOC[0],
    ptrs: crate::sites::BANDIT_PTRS,
    attack_states: &[2, 3, 3, 3],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    parse(text, "bandit", kh1_core::bandit::NAMES, REQUESTS)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::bandit::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_BANDIT_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_BANDIT_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!((m.consts.get("close_range"), m.consts.get("leap_speed")), (200.0, 70.0));
        // The clash recoils its rows name: 205 (slash, leap), 213 (the spin's); all in the model.
        assert_eq!((m.recoil_of(0), m.recoil_of(1), m.recoil_of(2), m.recoil_of(3)), (Some(205), Some(213), Some(213), Some(205)));
        assert_eq!(m.parts.len(), 6);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::bandit::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 2, 3, 11, 12, 30]);
    }
}
