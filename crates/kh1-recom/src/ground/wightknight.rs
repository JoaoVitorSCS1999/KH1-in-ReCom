//! M11: the KH1 Wight Knight's class row (`docs/recom-m11-enemies.md` § The Wight Knight).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::wightknight`).
pub const MANIFEST: &str = "kh1-wightknight.json";

/// The attack names (`kh1_core::wightknight::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["swipe", "back swipe", "leap slash"];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): its reactions (flinch
/// 72..75, air flinch 78 / 79, air knock-back 80 / 81, launch 82..86; the model has no ground knock-back) and every
/// row's recoil 217. The model has no run (request 2): the script never runs.
pub const REQUESTS: &[u16] = {
    use kh1_core::wightknight::req::*;
    &[IDLE, WALK, JUMP, FALL, APEX, AIR, LAND, APPEAR, 72, 73, 74, 75, 78, 79, 80, 81, 82, 83, 84, 85, 86, WINDUP, SWIPE, BACK_SWIPE, SWIPE_END, BACK_SWIPE_END, LEAP_FWD, LEAP_BACK, CROUCH, 208, 209, 210, DANCE, LEAP_SLASH, RECOIL, SWAY]
};

/// `CEnemyWightnight` (Re:CoM id 12). Checked (exe): vtable `0x140553748` (139 slots, RTTI COL at −8), ctor
/// `0x140194ED0` (only caller the factory `0x140188F40` at `0x140189194`; no branch into its first 6 bytes, no pointer
/// to them: exe scan), allocation `0xCD0` at `0x14018917B`. Its own states: 1 (`0x140195040` / `0x140195410`), 2 (its
/// two-strike claw `e012_052`, `.ASD` attack id 0, `0x140194FA0` / `0x140195090`), 3 (the wind-up `e012_053`, then
/// `e012_054`, id 1, `0x140194FF0` / `0x140195320`); states 0 / 11 / 12 / 30 `CBattleEnemy`'s; think `CHeartless`'s
/// `0x140196EC0`. KH1's attacks run in the state whose Re:CoM attack id they take their power from
/// (`kh1_recom_assets::wightknight`): the swipes 201 / 202 (id 1) in state 3, the leap slash 216 (id 0) in state 2.
pub static CLASS: Class = Class {
    name: "wightknight",
    title: "Wight Knight",
    rtti: ".?AVCEnemyWightnight@@",
    recom_id: 12,
    vtable: 0x0055_3748,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x4B57_314B, // "K1WK"
    ctor: &crate::sites::WIGHTKNIGHT_HOOKS[0],
    alloc_site: &crate::sites::WIGHTKNIGHT_ALLOC[0],
    ptrs: crate::sites::WIGHTKNIGHT_PTRS,
    attack_states: &[3, 3, 2],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let m = parse(text, "wightknight", kh1_core::wightknight::NAMES, REQUESTS)?;
    let names: Vec<&str> = m.guards.iter().map(|g| g.name.as_str()).collect();
    let want: Vec<&str> = kh1_core::wightknight::Guard::ALL.iter().map(|g| g.name()).collect();
    if names != want {
        return Err(format!("guards {names:?}, the behaviour's are {want:?} (an older asset step? run it again)"));
    }
    for (g, row) in kh1_core::wightknight::Guard::ALL.iter().zip(&m.guards) {
        if let Some(r) = g.requests().iter().find(|&&r| !row.made_with(r)) {
            return Err(format!("guard {}: no window for its request {r}", row.name));
        }
        if row.half_angle().is_none() {
            return Err(format!("guard {}: a turned cone ({}): not KH1's body row", row.name, row.turn));
        }
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::wightknight::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_WIGHTKNIGHT_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_WIGHTKNIGHT_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!((m.consts.get("close_range"), m.consts.get("leap_height"), m.consts.get("walk_speed")), (300.0, 500.0, 5.0));
        // Every row's clash recoil is 217 (in the model): Sora's Guard plays KH1's recoil for every attack.
        let r: Vec<Option<u16>> = (0..3).map(|i| m.recoil_of(i)).collect();
        assert_eq!(r, vec![Some(217); 3]);
        assert_eq!(m.parts.len(), 5);
        let g = &m.guards;
        assert_eq!((g[0].priority, g[0].clash_type, g[1].priority, g[1].clash_type, g[2].priority, g[2].recoil), (100, 1, 20, 2, 20, 217));
        for row in g {
            assert!((row.half_angle().unwrap() - std::f32::consts::PI).abs() < 1e-6, "all round");
        }
        assert!(g[0].live(200, 6.0) && !g[0].live(200, 5.0) && g[1].live(202, 69.0) && g[2].live(216, 2.0) && !g[2].live(216, 34.0));
        assert!(m.clip(201).unwrap().root && m.clip(205).unwrap().root && !m.clip(212).unwrap().root);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::wightknight::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 3, 2, 11, 12, 30]);
    }
}
