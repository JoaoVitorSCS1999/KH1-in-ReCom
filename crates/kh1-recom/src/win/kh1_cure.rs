//! M11: the KH1 Green Requiem's Cure on Re:CoM's own heal (`docs/recom-m11-enemies.md` § The Green Requiem; owner rule:
//! "each KH1 spell takes the effect / power of the Re:CoM spell of the same kind; heal amounts on Re:CoM's HP through
//! Re:CoM's own heal").
//!
//! - **The pick** (`kh1_core::enemy::Cmd::Support`, KH1's `sys 102` in the frame the cast starts): the Green Requiem's
//!   allies are the battle enemy manager's enemies of Re:CoM's enemy type (`0x1004`) but itself, in the manager's order
//!   (Re:CoM's own Green Requiem walks the same list in its state 2 begin `0x14018E640`; `super::battle::enemies`), with
//!   their Re:CoM HP (`+0x274`, max `0x14017F5A0`) and their distance; KH1's rule picks one with the behaviour's draw
//!   ([`kh1_core::greenrequiem::pick`]), else the caster itself (KH1's `0x350`). Its Re:CoM id (`+0x20`) is kept for the
//!   child's act.
//! - **The heal** (the child's [`kh1_core::enemy::Cmd::Throw`], 186 frames into 213): the kept id looked up as Re:CoM does
//!   (`0x1402AD350`; gone: nothing, KH1's `sys 120`); then Re:CoM's own Green Requiem heal exactly as its state 2 step
//!   `0x14018E9B0` makes it: its power `0x14017F0E0(caster, 0, 0x19, 0, 0, 0)` and `0x140181CE0(target, power,
//!   "_ex_2140", effect, &(0, 0, 0, 1), 0x1402ABB20(target, 1))`, the effect 8 / 9 / 10 for a target of Re:CoM's enemy id 3
//!   / 4 / 5, else 7 (`sites::GREENREQUIEM_CURE_CHECKED`). Re:CoM's heal keeps its own rules (nothing on an HP-0 enemy, a
//!   stopped one's heal held for after its Stop, its HP gauge and number popup).
//! - Not built: KH1's "can it be hit" test on the target (`0x140292070`: a sunk Shadow or a hidden Darkball takes no
//!   heal in KH1; here every living ally Re:CoM keeps does, as Re:CoM's own Green Requiem's; Assumption: rare); KH1's
//!   hit record reaching the target's script hit handler (it only marks it seen: no reaction, no state; nothing of ours
//!   reads that for a battle spawn).
//!
//! Per frame: nothing (one call per pick, one per heal).

use super::movement::{func, rd};
use super::{base, log};
use crate::kh1_enemy::Manifest;
use kh1_core::greenrequiem::{pick as kh1_pick, Ally};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

/// Re:CoM's functions (bytes checked: `sites::KH1_ENEMY_REWARD_CALLEES`, `sites::THROWN_CALLEES`, the shared lists).
mod f {
    pub const ATTACK_AP: u32 = 0x0017_F0E0;
    pub const HEAL: u32 = 0x0018_1CE0;
    pub const MAX_HP: u32 = 0x0017_F5A0;
    pub const BONE: u32 = 0x002A_BB20;
    pub const HANDLE_OBJ: u32 = 0x002A_D350;
}
/// The effect set's name Re:CoM's Green Requiem heals with (`sites::GREENREQUIEM_CURE_CHECKED`).
const EFFECT_NAME: usize = 0x0054_0958;
/// Enemy fields (Checked (decomp): `0x14018E640`, `0x14018E9B0`, `0x140181CE0`).
const ID: usize = 0x20;
const CLASS_ID: usize = 0x128;
const HP: usize = 0x274;

type ApFn = unsafe extern "win64" fn(usize, i32, i32, i16, *mut u32, u16) -> f32;
type HealFn = unsafe extern "win64" fn(usize, i32, usize, u32, *const [f32; 4], usize) -> u32;
type MaxHpFn = unsafe extern "win64" fn(usize) -> i32;
type BoneFn = unsafe extern "win64" fn(usize, i32) -> usize;
type HandleFn = unsafe extern "win64" fn(u32) -> usize;

/// The classes whose spell children are support spells (their `Cmd::Throw` is a heal, not a thrown object).
pub(super) fn owns(class: &str) -> bool {
    class == "greenrequiem"
}

/// The picked targets: (caster, child, Re:CoM id), until the child acts.
static PICKED: Mutex<Vec<(usize, u8, u32)>> = Mutex::new(Vec::new());
static LINES: AtomicU32 = AtomicU32::new(0);

fn line(s: impl FnOnce() -> String) {
    if LINES.fetch_add(1, Ordering::Relaxed) < 4000 {
        log(&format!("greenrequiem: {}", s()));
    }
}

/// KH1's `sys 102` for the caster's child `child` with the behaviour's draw: its allies from Re:CoM's battle list,
/// KH1's pick, the id kept.
pub(super) unsafe fn pick(en: usize, m: &Manifest, child: u8, draw: f32) {
    let c = |n: &str| m.consts.get(n);
    let me = super::kh1_enemy::kh1_place(en);
    let max_hp = func::<MaxHpFn>(f::MAX_HP);
    let allies: Vec<Ally> = super::battle::enemies(64)
        .into_iter()
        .filter(|&a| a != en)
        .map(|a| {
            let dist = match (me, super::kh1_enemy::kh1_place(a)) {
                (Some(p), Some(q)) => (q - p).length(),
                _ => f32::INFINITY,
            };
            Ally { id: rd(a + ID), hp: rd(a + HP), max: max_hp(a), dist }
        })
        .collect();
    let own: u32 = rd(en + ID);
    let got = kh1_pick(&allies, draw, c("pick_range"), c("pick_ratio"), c("pick_weight_0") as i32, c("pick_weight_1") as i32);
    let id = got.unwrap_or(own);
    line(|| {
        let list: Vec<String> = allies.iter().map(|a| format!("e{} {}/{} at {:.0}", a.id, a.hp, a.max, a.dist)).collect();
        format!("e{own}: Cure {child}: KH1's pick (sys 102, draw {draw:.4}) among [{}] -> e{id}{}", list.join(", "), if got.is_none() { " (itself)" } else { "" })
    });
    if let Ok(mut v) = PICKED.lock() {
        v.retain(|p| !(p.0 == en && p.1 == child));
        if v.len() > 64 {
            v.clear();
        }
        v.push((en, child, id));
    }
}

/// The child acts: Re:CoM's own Green Requiem heal on the kept target, if Re:CoM still has it.
pub(super) unsafe fn cure(en: usize, child: u8) {
    let own: u32 = rd(en + ID);
    let id = PICKED.lock().ok().and_then(|mut v| v.iter().position(|p| p.0 == en && p.1 == child).map(|i| v.remove(i).2));
    let Some(id) = id else {
        line(|| format!("e{own}: Cure {child}: no pick kept; nothing"));
        return;
    };
    let t = func::<HandleFn>(f::HANDLE_OBJ)(id);
    if t == 0 {
        line(|| format!("e{own}: Cure {child}: e{id} is gone (KH1's sys 120): nothing"));
        return;
    }
    let ap = func::<ApFn>(f::ATTACK_AP)(en, 0, 0x19, 0, std::ptr::null_mut(), 0) as i32;
    let eff = crate::fliers::greenrequiem::cure_effect(rd(t + CLASS_ID));
    let bone = func::<BoneFn>(f::BONE)(t, 1);
    let at = [0.0f32, 0.0, 0.0, 1.0];
    let before: i32 = rd(t + HP);
    // The effect set's name where the exe had it (`sites::GREENREQUIEM_CURE_CHECKED`), else the heal without its look.
    let name = base() + EFFECT_NAME;
    let name = if std::slice::from_raw_parts(name as *const u8, 9) == b"_ex_2140\0" { name } else { 0 };
    func::<HealFn>(f::HEAL)(t, ap, name, eff, &at, bone);
    let after: i32 = rd(t + HP);
    line(|| format!("e{own}: Cure {child} on e{id}: Re:CoM's Green Requiem heal (0x140181CE0, power {ap} from 0x14017F0E0(id 0, 0x19), \"_ex_2140\" {eff}): HP {before} -> {after}"));
}
