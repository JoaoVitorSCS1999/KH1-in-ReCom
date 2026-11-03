//! M11: the KH1 Neoshadow's class row (`docs/recom-m11-enemies.md` § The Neoshadow).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::neoshadow`).
pub const MANIFEST: &str = "kh1-neoshadow.json";

/// The attack names (`kh1_core::neoshadow::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["claw", "rise slash", "dive", "grab"];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): its reactions
/// (flinch 72..75, knock-back 76 / 77, air flinch 78 / 79; the model has no launch) and the claw's recoil 215.
pub const REQUESTS: &[u16] = {
    use kh1_core::neoshadow::req::*;
    &[IDLE, WALK, RUN, AIR, LAND, DEATH, 73, 74, 75, 76, 77, 78, 79, STRAFE_L, STRAFE_R, CLAW, CROUCH, HOVER, DIVE, LAND_DIVE, RISE_SLASH, GRAB, MISS, CALL, CALLED_RISE, CALLED_HOLD, CALLED_END, 215, SINK, UNDER, COME_UP]
};

/// `CEnemyNeoshadow` (Re:CoM id 27). Checked (exe): vtable `0x140545128` (139 slots, RTTI COL at −8), ctor
/// `0x140190150` (only caller the factory `0x140188F40` at `0x14018940A`; no branch into its first 6 bytes, no
/// pointer to them: exe scan), allocation `0xCE0` at `0x1401893F1`. Its own states: 0 (`0x140190220` / `0x140190480`
/// / end `0x140190400`: shown, its rise `e027_011` with its bodies off, turning to Sora, the default state at the
/// motion's end), 1 (`0x1401903E0` / `0x140190860`), 2 (its claw `e027_050`, `.ASD` attack id 0, `0x140190330` /
/// `0x140190540`), 3 (sink `e027_051`, up 2 m from Sora `e027_052`, id 1, then `e027_053`: `0x140190380` /
/// `0x1401905A0`); 11 / 12 / 30 `CBattleEnemy`'s; think `CHeartless`'s `0x140196EC0`. KH1's attacks run in the
/// state whose Re:CoM attack id they take their power from (`kh1_recom_assets::neoshadow`): the claw 202 (id 0) in
/// state 2, the rising slash 207 and the dive 205 (id 1) in state 3, the grab 208 (its held record on id 1's, M11
/// the held reaction) in state 3. Appearance: KH1's room Neoshadow has none
/// (`Appear::Base`: Re:CoM's state 0 begin runs, KH1's frame from the next, then the default state).
pub static CLASS: Class = Class {
    name: "neoshadow",
    title: "Neoshadow",
    rtti: ".?AVCEnemyNeoshadow@@",
    recom_id: 27,
    vtable: 0x0054_5128,
    slots: 139,
    alloc: 0xCE0,
    magic: 0x534E_314B, // "K1NS"
    ctor: &crate::sites::NEOSHADOW_HOOKS[0],
    alloc_site: &crate::sites::NEOSHADOW_ALLOC[0],
    ptrs: crate::sites::NEOSHADOW_PTRS,
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
    let m = parse(text, "neoshadow", kh1_core::neoshadow::NAMES, REQUESTS)?;
    let names: Vec<&str> = m.guards.iter().map(|g| g.name.as_str()).collect();
    if names != ["shield"] {
        return Err(format!("guards {names:?}, the behaviour's are [\"shield\"] (an older asset step? run it again)"));
    }
    let g = &m.guards[kh1_core::neoshadow::SHIELD as usize];
    if !g.windows.is_empty() || g.half_angle().is_none() {
        return Err(format!("guard shield: windows {:?} turn {}: not KH1's held body row", g.windows, g.turn));
    }
    // The grab's record names the held kind by its index in our kind table (`crate::held`).
    let k = m.consts.get("grab_kind_index");
    if k != crate::held::HELD_INDEX as f32 {
        return Err(format!("the grab's record names kind index {k}, the held kind's is {} (an older asset step? run it again)", crate::held::HELD_INDEX));
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // The battle's group comes with `Behaviour::set_battle` (the ctor, right after this).
    Box::new(kh1_core::neoshadow::Ai::new(m.consts.clone(), m.frames(), seed, yaw))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The asset step's real manifest (`KH1_NEOSHADOW_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(p) = std::env::var_os("KH1_NEOSHADOW_JSON") else { return };
        let m = (CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap();
        assert_eq!((m.consts.get("crowd"), m.consts.get("dive_speed"), m.consts.get("slot_2_0_y")), (5.0, 80.0, 5480.0));
        // The claw's recoil 215 (in the model); the rising slash's and the dive's 110 (not: Re:CoM's flinch).
        let r: Vec<Option<u16>> = (0..4).map(|i| m.recoil_of(i)).collect();
        assert_eq!(r, vec![Some(215), None, None, None]);
        assert_eq!((m.consts.get("grab_trim"), m.consts.get("grab_let_go"), m.consts.get("grab_record_frames"), m.consts.get("grab_kind_index")), (2.0, 30.0, 2.0, 21.0));
        let g = &m.guards[0];
        assert_eq!((g.clash_type, g.priority, g.half_angle()), (4, 100, Some(std::f32::consts::PI)));
        assert!(m.clip(205).unwrap().looped && !m.clip(207).unwrap().looped);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::neoshadow::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 2, 3, 11, 12, 30]);
    }
}
