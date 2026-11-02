//! M11: the KH1 Darkball's class row (`docs/recom-m11-enemies.md` § The Darkball).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::darkball`).
pub const MANIFEST: &str = "kh1-darkball.json";

/// The attack names (`kh1_core::darkball::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["bite", "dash", "drop"];

/// The requests the behaviour plays (its manifest must map them).
pub const REQUESTS: &[u16] = {
    use kh1_core::darkball::req::*;
    &[IDLE, FLINCH, KNOCK, APPEAR, WARP, DASH_UP, DASH, DASH_END, BITE, NOTICE]
};

/// `CEnemyDarkball` (Re:CoM id 23). Checked (exe + decomp): vtable `0x14053B7A8` (139 slots, RTTI at
/// −8), ctor `0x14018D5D0` (`CHeartless`'s base ctor, then `+0x154 |= 0x80`, `+0x260 |= 0x100`:
/// gravity off), allocation `0xCD0` at `0x140189349` (the factory's case 23). Its own states: 1 (the
/// flying walk, `0x14018D720` / `0x14018D8D0` → `0x140197450`), 2 (its dash `e023_050` → `e023_051`
/// at `BiteSpeed` → `e023_052`, `0x14018D6A0` / `0x14018D780`), 3 (its bite `e023_053`, `0x14018D6F0`
/// / `0x14018D8A0`); the think is `CHeartless`'s `0x140196EC0`. Re:CoM's dash and bite clips are
/// bakes of KH1's 201..203 and 204 (KH1's matcher, Checked (files)): KH1's dash runs in state 2, its
/// bite and its drop (the reappearance's request 200 with its row) in state 3.
pub static CLASS: Class = Class {
    name: "darkball",
    title: "Darkball",
    rtti: ".?AVCEnemyDarkball@@",
    recom_id: 23,
    vtable: 0x0053_B7A8,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x4244_314B, // "K1DB"
    ctor: &crate::sites::DARKBALL_HOOKS[0],
    alloc_site: &crate::sites::DARKBALL_ALLOC[0],
    ptrs: crate::sites::DARKBALL_PTRS,
    attack_states: &[3, 2, 3],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    parse(text, "darkball", kh1_core::darkball::NAMES, REQUESTS)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::darkball::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_DARKBALL_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_DARKBALL_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!((m.consts.get("far_range"), m.consts.get("glide_speed")), (600.0, 24.0));
        // The dash's recoil 206 is a motion of the model; the bite's and the drop's 110 is not
        // (Re:CoM's flinch, owner 2026-11-02).
        assert_eq!((m.recoil_of(0), m.recoil_of(1), m.recoil_of(2)), (None, Some(206), None));
        assert!(!m.parts.is_empty());
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::darkball::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 3, 2, 11, 12, 30]);
        assert_eq!(&CLASS.magic.to_le_bytes(), b"K1DB");
    }
}
