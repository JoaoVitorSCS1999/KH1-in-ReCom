//! M11: the KH1 Yellow Opera's class row (`docs/recom-m11-enemies.md` § The Yellow Opera).

use crate::kh1_enemy::{parse, Appear, Class, Manifest, FORMAT};
use kh1_core::enemy as ke;

/// The manifest the asset step writes (`kh1_recom_assets::yellowopera`).
pub const MANIFEST: &str = "kh1-yellowopera.json";

/// The attack names (`kh1_core::yellowopera::Attack::ALL`'s order).
pub const ATTACKS: &[&str] = &["charge"];

/// The requests the behaviour plays and the host plays for it (its manifest must map them): its Thunder's three (205..207),
/// its charge's three (208..210) and the rush rows' recoil 211, its reactions (flinch 72..75, knock-back 76 / 77, air
/// flinch 78, air knock-back 80 / 81).
pub const REQUESTS: &[u16] = {
    use kh1_core::yellowopera::req::*;
    &[IDLE, CAST, HOLD, STRIKE, WIND_UP, RUSH, RECOVER, RUSH_RECOIL, APPEAR, FLINCH, KNOCK, AIR_FLINCH, AIR_KNOCK, NOTICE, WANDER, REST]
};

/// The numbers the host reads for its Thunder (`win::kh1_thrown::strike`): its child index, its cylinder (radius,
/// half-height, centre above the mark), Re:CoM power.
pub const HOST_NAMES: &[&str] = &["thunder_child", "thunder_radius", "thunder_half", "thunder_mid", "thunder_recom_id", "thunder_kind"];

/// `CEnemyYellowopera` (Re:CoM id 5). Checked (exe + decomp): vtable `0x1405570C8` (139 slots, RTTI COL at −8), ctor
/// `0x1401968B0` (`CHeartless`'s base ctor `0x140196DF0`, then `+0x154 |= 0x80`, `+0x260 |= 0x100`: gravity off; its only
/// caller the factory `0x140188F40` at `0x14018906E`), allocation `0xCD0` at `0x140189055`. Its own states: 1 (the flying
/// walk `0x140196A20` → `0x1401979F0` with `e003_001` / `e003_002`, step `0x140196DE0` → `0x140197450`, end `0x140196A70`
/// → `0x1401979C0`), 2 (its Thunder `e005_052`: begin `0x140196980`, step `0x140196A80`: past clip time 0xD15 a `CBullet`
/// 4 m above Sora's place pitched straight down, 5 m a frame, radius 0.25, kind 0x1C, `0x14017F0E0(enemy, 0, 1)`'s AP,
/// effect set `0x140557560`), 3 (its rush `e005_053` → `e005_054` (`RushAtkTime`, `RushSpeed`; its `.ASD`'s one record,
/// attack id 1) → `e005_055`: begin `0x1401969D0`, step `0x140196C10`); states 0 / 11 / 12 / 30 `CBattleEnemy`'s; think
/// `CHeartless`'s `0x140196EC0`. KH1's charge (a plain request with rows, steered) runs in state 3; its Thunder is a
/// Re:CoM bullet made from state 1 (`win::kh1_thrown::strike`); state 2 is never entered.
pub static CLASS: Class = Class {
    name: "yellowopera",
    title: "Yellow Opera",
    rtti: ".?AVCEnemyYellowopera@@",
    recom_id: 5,
    vtable: 0x0055_70C8,
    slots: 139,
    alloc: 0xCD0,
    magic: 0x4F59_314B, // "K1YO"
    ctor: &crate::sites::YELLOWOPERA_HOOKS[0],
    alloc_site: &crate::sites::YELLOWOPERA_ALLOC[0],
    ptrs: crate::sites::YELLOWOPERA_PTRS,
    attack_states: &[3],
    attack_names: ATTACKS,
    appear: Appear::Base,
    reads_hp: false,
    manifest: MANIFEST,
    format: FORMAT,
    parse: parse_manifest,
    make,
};

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let m = parse(text, "yellowopera", &kh1_core::yellowopera::all_names(), REQUESTS)?;
    for n in HOST_NAMES {
        if !m.consts.get(n).is_finite() {
            return Err(format!("constant {n} missing (an older asset step? run it again)"));
        }
    }
    if m.recoil.len() != ATTACKS.len() {
        return Err(format!("{} attacks in the manifest, the Yellow Opera has {}", m.recoil.len(), ATTACKS.len()));
    }
    Ok(m)
}

fn make(m: &Manifest, seed: u32, yaw: f32) -> Box<dyn ke::Behaviour> {
    // Re:CoM's battle spawns play KH1's appearance (owner 2026-11-02, Soldier choices).
    Box::new(kh1_core::yellowopera::Ai::new(m.consts.clone(), m.frames(), seed, yaw, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh1_core::enemy::host::model::Model;
    use kh1_core::yellowopera::{child, req};
    use kh1_core::Vec3;

    fn manifest() -> Option<Manifest> {
        let p = std::env::var_os("KH1_YELLOWOPERA_JSON")?;
        Some((CLASS.parse)(&std::fs::read_to_string(p).unwrap()).unwrap())
    }

    /// The asset step's real manifest (`KH1_YELLOWOPERA_JSON`) parses here and makes a behaviour.
    #[test]
    fn asset_step_manifest_parses() {
        let Some(m) = manifest() else { return };
        assert_eq!((m.consts.get("close_range"), m.consts.get("mid_range"), m.consts.get("thunder_chance"), m.consts.get("rush_speed")), (250.0, 500.0, 0.6, 25.0));
        assert_eq!((m.consts.get("thunder_wait"), m.consts.get("thunder_delay"), m.consts.get("thunder_radius"), m.consts.get("thunder_half"), m.consts.get("thunder_mid"), m.consts.get("thunder_recom_id"), m.consts.get("thunder_kind")), (80.0, 30.0, 50.0, 128.0, 128.0, 0.0, 1.0));
        // The rush's KH1 recoil 211 is a motion of the model; its body row (type 2) never hurts: one record.
        assert_eq!(m.recoil_of(0), Some(req::RUSH_RECOIL));
        assert!(m.has_never_hurting() && m.guards.is_empty());
        assert_eq!((m.clip(req::CAST).unwrap().frames, m.clip(req::HOLD).unwrap().frames, m.clip(req::STRIKE).unwrap().frames, m.clip(req::RUSH).unwrap().frames), (104, 70, 164, 40));
        assert!(m.clip(req::RUSH).unwrap().looped);
        let _ = (CLASS.make)(&m, 1, 0.0);
    }

    /// The behaviour through the host's rules with the real manifest: a 20 000-frame fight, 12 seeds (Sora circling
    /// 1.5..15.5 m away): nothing released, the charge only in state 3 and never asked for again within two frames,
    /// Thunders struck, never quiet for 600 frames, Re:CoM's state 2 never entered.
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
            let rush = model.attacks_of(req::RUSH);
            assert!(!rush.is_empty(), "seed {seed}: no charge");
            assert!(rush.windows(2).all(|w| w[1] > w[0] + 2), "seed {seed}: the charge asked again");
            assert!(model.trace.iter().all(|t| t.1 != Some(req::RUSH) || t.0 == 3), "seed {seed}: 209 outside state 3");
            assert!(model.trace.iter().all(|t| t.0 != 2), "seed {seed}: Re:CoM's state 2 entered");
            assert!(model.log.iter().any(|(_, c)| matches!(c, ke::Cmd::Strike { child: child::THUNDER, .. })), "seed {seed}: no Thunder");
        }
    }

    #[test]
    fn row() {
        assert_eq!(CLASS.attack_names, kh1_core::yellowopera::Attack::ALL.map(|a| a.name()));
        assert_eq!(CLASS.alloc_site.expect, &crate::kh1_enemy::alloc_bytes(CLASS.alloc));
        assert_eq!(crate::kh1_enemy::replaced_states(&CLASS), vec![0, 1, 3, 11, 12, 30]);
        assert_eq!(&CLASS.magic.to_le_bytes(), b"K1YO");
        for s in crate::kh1_enemy::replaced_states(&CLASS) {
            let at = CLASS.vtable + crate::kh1_enemy::begin_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at), "state {s} begin");
            let at = CLASS.vtable + crate::kh1_enemy::step_slot(s) as u32;
            assert!(CLASS.ptrs.iter().any(|p| p.rva == at) || s == 11 || s == 12, "state {s} step");
        }
        // Every magic is unique among the classes.
        let mut seen: Vec<u32> = crate::kh1_enemy::CLASSES.iter().map(|c| c.magic).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), crate::kh1_enemy::CLASSES.len());
    }
}
