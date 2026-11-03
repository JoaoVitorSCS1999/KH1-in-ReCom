//! M11: the KH1 Green Requiem's class row (`docs/recom-m11-enemies.md` § The Green Requiem).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::greenrequiem`).
pub const MANIFEST: &str = "kh1-greenrequiem.json";

/// No attack of its own (its one row is its Cure's, applied to an ally: `win::kh1_cure`).
pub const ATTACKS: &[&str] = &[];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): its reactions (flinch
/// 72..75, knock-back 76 / 77, air flinch 78, air knock-back 80 / 81).
pub const REQUESTS: &[u16] = {
    use kh1_core::greenrequiem::req::*;
    &[IDLE, HOVER, APPEAR, FLINCH, KNOCK, AIR_FLINCH, AIR_KNOCK, CHARGE, CAST, NOTICE, WANDER, REST]
};

/// The numbers the host reads for its Cure (`win::kh1_cure`): its child index, KH1's pick (range, HP ratio, the search
/// entry's first two weights).
pub const HOST_NAMES: &[&str] = &["cure_child", "pick_range", "pick_ratio", "pick_weight_0", "pick_weight_1"];

/// `CEnemyGreenrequiem` (Re:CoM id 6). Checked (exe + decomp): vtable `0x1405404C8` (139 slots, RTTI COL at −8), ctor
/// `0x14018E570` (`CHeartless`'s base ctor `0x140196DF0`, then `+0x154 |= 0x80`, `+0x260 |= 0x100`: gravity off; its only
/// caller the factory `0x140188F40` at `0x140189098`), allocation `0xCE0` at `0x14018907F`. Its own states: 1 (the flying
/// walk `0x14018E780` → `0x1401979F0` with `e003_001` / `e003_002`, step `0x14018EC60`, end `0x14018E7C0`), 2 (its Cure
/// `e006_051`: begin `0x14018E640` picks the battle enemy of its enemy type with the most HP missing, else itself (`+0xCD0`
/// its id), and flies about it; step `0x14018E9B0`: past clip time 6000 one `0x140181CE0(target, 0x14017F0E0(enemy, 0,
/// 0x19), "_ex_2140", 7..10, …)`, then Re:CoM's attack end); states 0 / 3 / 11 / 12 / 30 `CBattleEnemy`'s; think
/// `CHeartless`'s `0x140196EC0`. KH1's Green Requiem has no attack of its own: every motion plays in state 1, its Cure is
/// Re:CoM's own heal made from state 1 (`win::kh1_cure`); state 2 is never entered.
pub static CLASS: Class = Class {
    name: "greenrequiem",
    title: "Green Requiem",
    rtti: ".?AVCEnemyGreenrequiem@@",
    recom_id: 6,
    vtable: 0x0054_04C8,
    slots: 139,
    alloc: 0xCE0,
    magic: 0x5247_314B, // "K1GR"
    ctor: &crate::sites::GREENREQUIEM_HOOKS[0],
    alloc_site: &crate::sites::GREENREQUIEM_ALLOC[0],
    ptrs: crate::sites::GREENREQUIEM_PTRS,
    attack_states: &[],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

/// Re:CoM's effect (of `"_ex_2140"`) for its Green Requiem's heal on a target of Re:CoM enemy id `class_id` (`+0x128`): its
/// state 2 step's switch on `% 10000` (`0x14018EA34`, `sites::GREENREQUIEM_CURE_CHECKED`): 3 (Red Nocturne) 8, 4 (Blue
/// Rhapsody) 9, 5 (Yellow Opera) 10, any other 7. Checked (disasm).
pub fn cure_effect(class_id: i32) -> u32 {
    match class_id % 10000 {
        3 => 8,
        4 => 9,
        5 => 10,
        _ => 7,
    }
}

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let m = parse(text, "greenrequiem", &kh1_core::greenrequiem::all_names(), REQUESTS)?;
    for n in HOST_NAMES {
        if !m.consts.get(n).is_finite() {
            return Err(format!("constant {n} missing (an older asset step? run it again)"));
        }
    }
    if !m.recoil.is_empty() {
        return Err("the Green Requiem has no attack of its own".into());
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::greenrequiem::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh1_core::enemy::host::model::Model;
    use kh1_core::greenrequiem::{child, req};
    use kh1_core::Vec3;

    fn manifest() -> Option<Manifest> {
        let p = std::env::var_os("KH1_GREENREQUIEM_JSON")?;
        Some((CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap())
    }

    /// The asset step's real manifest (`KH1_GREENREQUIEM_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(m) = manifest() else { return };
        assert_eq!((m.consts.get("cure_chance"), m.consts.get("move_chance"), m.consts.get("cast_speed"), m.consts.get("rise_up"), m.consts.get("cure_wait")), (0.8, 0.3, 6.0, 180.0, 186.0));
        assert_eq!((m.consts.get("pick_range"), m.consts.get("pick_ratio"), m.consts.get("pick_weight_0"), m.consts.get("pick_weight_1"), m.consts.get("cure_child")), (5000.0, 1.0, 60.0, 30.0, 0.0));
        assert!(m.recoil.is_empty() && m.guards.is_empty() && !m.has_never_hurting());
        assert_eq!((m.clip(req::CHARGE).unwrap().frames, m.clip(req::CAST).unwrap().frames, m.clip(req::APPEAR).unwrap().frames), (128, 242, 90));
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    /// The behaviour through the host's rules with the real manifest: a 20 000-frame fight, 12 seeds (Sora circling
    /// 1.5..15.5 m away): nothing released, no attack state, Cures cast (each after its pick), never quiet for 600 frames.
    #[test]
    fn hosted_with_the_real_manifest() {
        let Some(m) = manifest() else { return };
        let frames = m.frames();
        let loops: Vec<u16> = frames.iter().map(|f| f.0).filter(|&r| m.clip(r).is_some_and(|c| c.looped)).collect();
        for seed in 0..12u32 {
            let ai = (CLASS.make)(&m, seed, 0.0);
            let mut model = Model::new(ai, m.consts.clone(), frames.clone(), loops.clone(), CLASS.attack_states.to_vec());
            model.pos = Vec3::new(0.0, -150.0, 0.0);
            let (mut last, mut gap) = (0usize, 0usize);
            for f in 0..20_000usize {
                let a = f as f32 / 600.0;
                let rad = 150.0 + 700.0 * (1.0 + (f as f32 / 1700.0).sin());
                let n = model.log.len();
                model.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
                if model.log.len() > n || model.moved > 0.0 {
                    gap = gap.max(f - last);
                    last = f;
                }
            }
            assert_eq!(model.released, 0, "seed {seed}");
            assert!(gap < 600, "seed {seed}: quiet {gap}");
            assert!(model.trace.iter().all(|t| t.0 == 1), "seed {seed}: an attack state");
            let cures: Vec<usize> = model.log.iter().filter(|(_, c)| *c == ke::Cmd::Throw(child::CURE)).map(|(f, _)| *f).collect();
            assert!(!cures.is_empty(), "seed {seed}: no Cure");
            for c in cures {
                assert!(model.log.iter().any(|(f, x)| *f + 128 + 186 == c && matches!(x, ke::Cmd::Support { child: child::CURE, .. })), "seed {seed}: the Cure at {c} without its pick");
            }
        }
    }

    #[test]
    fn cure_effects_by_class() {
        assert_eq!((cure_effect(3), cure_effect(4), cure_effect(5), cure_effect(6), cure_effect(10004), cure_effect(1)), (8, 9, 10, 7, 9, 7));
    }

    #[test]
    fn row() {
        assert!(CLASS.attack_names.is_empty() && CLASS.attack_states.is_empty());
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 11, 12, 30]);
        assert_eq!(&CLASS.magic.to_le_bytes(), b"K1GR");
        for s in crate::kh1_enemy::replaced_states(&CLASS) {
            let at = CLASS.vtable + crate::kh1_enemy::begin_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at), "state {s} begin");
            let at = CLASS.vtable + crate::kh1_enemy::step_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at) || s == 11 || s == 12, "state {s} step");
        }
        assert!(crate::kh1_enemy::CLASSES.iter().any(|c| c.name == "greenrequiem"));
    }
}
