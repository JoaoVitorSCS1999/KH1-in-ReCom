//! M11: the KH1 Pirate's class row (`docs/recom-m11-enemies.md` § The Pirate).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::pirate`).
pub const MANIFEST: &str = "kh1-pirate.json";

/// The attack names (`kh1_core::pirate::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["slash", "counter", "jump slash", "slam", "spin", "getup front", "getup back"];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): the clash
/// recoils 209 / 211 / 215 / 221 too.
pub const REQUESTS: &[u16] = {
    use kh1_core::pirate::req::*;
    &[IDLE, WALK, RUN, JUMP, FALL, APEX, AIR, LAND, APPEAR, LEAP, 201, 202, 203, STUCK, STUCK_HOLD, PULL, STANCE, STANCE_RECOIL, JUMP_SLASH, JUMP_SLASH_RECOIL, LAUGH, RECOVER, SLASH, RECOIL, COUNTER, SPIN, SLAM, DIZZY, TAUNT, SPIN_RECOIL, WINDUP, GETUP_FRONT, GETUP_BACK, OWN_LAUNCH, OWN_AIR_LAUNCH]
};

/// `CEnemyPirate` (Re:CoM id 21). Checked (exe): vtable `0x140546458` (139 slots, RTTI COL at −8), ctor
/// `0x140190F30` (only caller the factory `0x140188F40` at `0x14018930E`; no branch into its first 6 bytes:
/// exe scan), allocation `0xCD0` at `0x1401892F5`. Its own states: 1 (`0x140191090` / `0x140191370`), 2
/// (its spin `e021_050`, `.ASD` attack id 0, `0x140191000` / `0x1401910C0`), 3 (its slash `e021_051`, id 1,
/// `0x140191040` / `0x140191120`); states 0 / 11 / 12 / 30 and the attack states' ends are
/// `CBattleEnemy`'s. KH1's attacks run in the state whose Re:CoM attack id they take their power from
/// (`kh1_recom_assets::pirate`): id 0 (the spin 217, the jump slash 210, the get-up slashes 235 / 236) in
/// state 2, id 1 (the slash 214, the counter 216, the slam 218) in state 3.
pub static CLASS: Class = Class {
    name: "pirate",
    title: "Pirate",
    rtti: ".?AVCEnemyPirate@@",
    recom_id: 21,
    vtable: 0x0054_6458,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x5250_314B, // "K1PR"
    ctor: &crate::sites::PIRATE_HOOKS[0],
    alloc_site: &crate::sites::PIRATE_ALLOC[0],
    ptrs: crate::sites::PIRATE_PTRS,
    attack_states: &[3, 3, 2, 3, 2, 2, 2],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let m = parse(text, "pirate", kh1_core::pirate::NAMES, REQUESTS)?;
    let names: Vec<&str> = m.guards.iter().map(|g| g.name.as_str()).collect();
    let want: Vec<&str> = kh1_core::pirate::Guard::ALL.iter().map(|g| g.name()).collect();
    if names != want {
        return Err(format!("guards {names:?}, the behaviour's are {want:?} (an older asset step? run it again)"));
    }
    for (g, row) in kh1_core::pirate::Guard::ALL.iter().zip(&m.guards) {
        if let Some(r) = g.requests().iter().find(|&&r| !row.made_with(r)) {
            return Err(format!("guard {}: no window for its request {r}", row.name));
        }
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::pirate::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_PIRATE_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_PIRATE_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!((m.consts.get("close_range"), m.consts.get("spin_speed")), (250.0, 3.6));
        // The clash recoils its rows name (all in the model): 215 (the slash, the counter, the slam, the
        // get-ups), 211 (the jump slash), 221 (the spin).
        let r: Vec<Option<u16>> = (0..7).map(|i| m.recoil_of(i)).collect();
        assert_eq!(r, vec![Some(215), Some(215), Some(211), Some(215), Some(221), Some(215), Some(215)]);
        assert_eq!(m.parts.len(), 7);
        let g = &m.guards;
        assert_eq!((g[0].priority, g[0].recoil, g[0].clash_type, g[1].priority, g[1].recoil, g[1].clash_type), (90, 221, 2, 30, 209, 2));
        assert_eq!((g[0].half_angle(), g[1].half_angle()), (Some(std::f32::consts::FRAC_PI_2), Some(std::f32::consts::FRAC_PI_2)));
        assert!(g[0].live(217, 0.0) && g[0].live(217, 47.0) && g[1].live(208, 47.0));
        // The jump slash and the wind-up from frame 8 (`enemy::STARTS`); the spin loops.
        assert_eq!((m.clip(210).unwrap().frames, m.clip(222).unwrap().frames, m.clip(217).unwrap().looped), (58, 122, true));
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::pirate::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 3, 2, 11, 12, 30]);
    }
}
