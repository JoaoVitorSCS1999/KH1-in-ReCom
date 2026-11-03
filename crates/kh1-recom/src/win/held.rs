//! M11, the Neoshadow's grab from below: Sora's held reaction (`crate::held`, `docs/recom-kh1-attack.md` § The
//! held reaction).
//!
//! - **The kind** ([`install`]): Re:CoM's kind table `0x14052A220` copied next to the exe with the held kind added
//!   at index 21, and the getter `0x140179410`'s `lea rcx, [table]` pointed at the copy (a data patch of its
//!   displacement, old bytes checked; the getter's 15 bytes are a checked site, `sites::THROWN_CALLEES`). Re:CoM's
//!   21 kinds are unchanged; only the Neoshadow's grab record (the asset step's) names index 21.
//! - **The case** ([`on_damage`], from the damage-check wrapper `attack::damage_check` on `0x1401E6DE0`): a pending
//!   hit of the held kind does no damage; Re:CoM's check runs for it with its no-reaction exit (`+0x420 & 2`, its
//!   gates first), and when it took the hit Sora enters the held reaction ([`enter`]): Re:CoM's status entry
//!   (`0x1401F6990(pl, 6)`, the action fields as the check writes them, then `0x1401F1200`'s no-status branch:
//!   status action `0x18`, sub-state `0xC`, velocity zero, `+0x428 &= ~8`) with the held clip's action-table row
//!   at KH1's blend. Re:CoM's status exe ends it at the clip's end; its damage check frees him with any damaging
//!   hit's reaction. No hook of its own and nothing of ours runs during the hold.
//! - **The enemy's view** ([`hold`]): the KH1 enemy host reads Sora's side for a behaviour that asks
//!   (`kh1_core::enemy::Hold`), each frame of that enemy's AI.

use super::movement::{self as mv, func, p, rd, wr};
use super::{base, log};
use crate::held as h;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::OnceLock;

/// KH1's blend of the held request, from the motion manifest's `held` clip (`win::kh1_clips`).
pub static PACK_BLEND: OnceLock<f32> = OnceLock::new();

/// The held clip's action-table row (valid once [`READY`]).
static ROW: AtomicU32 = AtomicU32::new(0);
/// KH1's blend of the held request (frames, f32 bits).
static BLEND: AtomicU32 = AtomicU32::new(0);
/// The kind table is extended and the row is in Sora's action table.
static READY: AtomicBool = AtomicBool::new(false);
static LINES: AtomicU32 = AtomicU32::new(0);

/// `0x1401F6990(pl, 6)`: what Re:CoM's damage check runs before every reaction it starts. Checked (decomp).
const REACTION_RESET: u32 = 0x001F_6990;
type ResetFn = unsafe extern "win64" fn(usize, u32);

/// Sora's fields the status entry writes (`0x1401E6DE0` before its switch, `0x1401F1200`). Checked (decomp).
const DEAD: usize = 0x7B0;
const IMMUNE: usize = 0x7BC;
const F_7B4: usize = 0x7B4;
const F_64C: usize = 0x64C;
const HIT_KIND: usize = 0x710;
const HIT_PENDING: usize = 0x720;
const HIT_VALUE: usize = 0x722;
/// A hit arrived (`+0x41C & 0x200`): the check looks at the pending hit only then.
const HIT_FLAGS: usize = 0x41C;
const HIT_ARRIVED: u32 = 0x200;
/// Re:CoM's hit reaction action (the check writes it before its switch; `0x1401F1200` then keeps it as the
/// previous action).
const HIT_REACTION: u32 = 0xC;

fn note(line: impl FnOnce() -> String) {
    if LINES.fetch_add(1, Ordering::Relaxed) < 40 {
        log(&line());
    }
}

/// The kind table and the row. Needs the damage-check wrapper (the attack hooks) and the held clip's row in
/// Sora's action table (`movement::EXT`, the asset step's `held` clip).
pub unsafe fn install(base: usize) -> bool {
    let Some((row, blend)) = mv::EXT.get().and_then(|e| e.held).zip(PACK_BLEND.get().copied()) else {
        log("held: no held clip in the motion pack (re-run kh1-recom-build): the Neoshadow misses its grab from below (KH1's other branch)");
        return false;
    };
    let prelude = &crate::sites::HELD_CALLEES[0];
    if std::slice::from_raw_parts((base + prelude.rva as usize) as *const u8, prelude.expect.len()) != prelude.expect {
        log("held: Re:CoM's reaction prelude 0x1401F6990 is not as expected; no held reaction");
        return false;
    }
    let site = crate::sites::THROWN_CALLEES.iter().find(|s| s.rva == 0x0017_9410).expect("the kind getter's site");
    let at = base + site.rva as usize;
    if std::slice::from_raw_parts(at as *const u8, site.expect.len()) != site.expect {
        log("held: the kind getter 0x140179410 is not as expected; no held reaction");
        return false;
    }
    // movsxd rax,[rcx+0x68]; lea rcx,[rip+disp]; mov eax,[rcx+rax*4]; ret: the disp at +7, rip = +11.
    let disp_at = at + 7;
    let old: [u8; 4] = rd(disp_at);
    let table = (at as i64 + 11 + i32::from_le_bytes(old) as i64) as usize;
    let recom: Vec<u32> = (0..h::RECOM_KINDS.len()).map(|i| rd::<u32>(table + 4 * i)).collect();
    let Some(ext) = h::kind_table(&recom) else {
        log(&format!("held: Re:CoM's kind table at {table:#x} reads {recom:?}, not the Steam exe's; no held reaction"));
        return false;
    };
    let bytes: Vec<u8> = ext.iter().flat_map(|k| k.to_le_bytes()).collect();
    let Some(copy) = super::hook::place_data_near(at, &bytes) else {
        log("held: no memory near the exe for the kind table; no held reaction");
        return false;
    };
    let Ok(disp) = i32::try_from(copy as i64 - (at as i64 + 11)) else {
        log("held: the kind table's copy is out of reach; no held reaction");
        return false;
    };
    if !super::hook::patch(disp_at, &old, &disp.to_le_bytes(), "attack record kind table (0x140179417)") {
        return false;
    }
    ROW.store(row, Ordering::Relaxed);
    BLEND.store(blend.to_bits(), Ordering::Relaxed);
    READY.store(true, Ordering::Release);
    log(&format!(
        "held (M11, the Neoshadow's grab): Re:CoM's kind table at {copy:#x} with the held kind {:#x} at index {}; the held reaction is status 0x18 sub 0xC on action row {row:#x} (blend {blend}), entered from the damage-check wrapper at combat = kh1",
        h::HELD_KIND,
        h::HELD_INDEX
    ));
    true
}

/// Sora as the rules see him.
unsafe fn sora_now(pl: usize) -> h::SoraNow {
    let body: usize = rd(pl + p::BODY);
    h::SoraNow {
        kh1: mv::kh1_sora(pl),
        row: READY.load(Ordering::Acquire).then(|| ROW.load(Ordering::Relaxed)),
        dead: rd::<i32>(pl + DEAD) != 0,
        protected: rd::<u32>(pl + p::FLAGS_418) & 1 != 0,
        immune: rd::<i32>(pl + IMMUNE) > 0,
        busy_7b4: rd::<i32>(pl + F_7B4) != 0,
        no_flinch: super::attack::no_flinch(pl),
        guarding: super::attack::guard_live(pl),
        grounded: body != 0 && rd::<u32>(body + mv::body::CONTACT) & 5 != 0,
        action: rd(pl + p::ACTION),
        sub: rd(pl + p::ACTION_SUB),
        row_now: rd(pl + p::ROW),
    }
}

/// Sora's side of KH1's grab for the enemy host (`kh1_core::enemy::Hold`): the main player, when he is Sora.
pub unsafe fn hold() -> kh1_core::enemy::Hold {
    let pl: usize = rd(super::va(base(), crate::sites::data::MAIN_PLAYER));
    if !READY.load(Ordering::Acquire) || !mv::is_main_sora(pl) {
        return kh1_core::enemy::Hold::default();
    }
    let s = sora_now(pl);
    kh1_core::enemy::Hold { grabbable: h::grabbable(&s), held: h::is_held(&s) }
}

/// The damage check's held case, around Re:CoM's check `o` (the wrapper's other steps run inside `o`). `None`:
/// not the held kind (the wrapper runs as ever).
pub unsafe fn on_damage(pl: usize, o: &dyn Fn(usize) -> u64) -> Option<u64> {
    if !READY.load(Ordering::Acquire) {
        return None;
    }
    let pending: i16 = rd(pl + HIT_PENDING);
    if pending == 0 || rd::<u32>(pl + HIT_FLAGS) & HIT_ARRIVED == 0 {
        return None;
    }
    let s = sora_now(pl);
    let case = h::on_hit(rd(pl + HIT_KIND), &s);
    if case == h::OnHit::Recom {
        return None;
    }
    // The held kind does no damage (KH1's grab makes no attack row), and Re:CoM's check takes no reaction for it.
    wr(pl + HIT_VALUE, 0i16);
    let f: u32 = rd(pl + p::FLAGS_420);
    wr(pl + p::FLAGS_420, f | 2);
    let r = o(pl);
    if f & 2 == 0 {
        wr(pl + p::FLAGS_420, rd::<u32>(pl + p::FLAGS_420) & !2);
    }
    let h::OnHit::Hold { row } = case else {
        note(|| format!("held: a held-kind hit on Sora with no hold ({s:?}): no damage, no reaction"));
        return Some(r);
    };
    if !h::taken(pending, rd(pl + HIT_PENDING), rd::<i32>(pl + DEAD) != 0) {
        note(|| "held: Re:CoM's damage check refused the grab's hit (protected / immune / guarded): no hold".to_string());
        return Some(r);
    }
    enter(pl, row);
    Some(1)
}

/// Re:CoM's status entry, as its damage check makes it for a status kind (`0x1401E6DE0` from its reaction prelude
/// `0x1401F6990(pl, 6)` to `0x1401F1200`'s branch with no status bit), with the held clip's row at KH1's blend
/// in place of row 0x28. Checked (decomp). Left out: the damage face (`0x1401F8AC0`: no damage) and the pad
/// rumble (`0x140211E40(2)`: Re:CoM's, not KH1's pattern; KH1's grab rumble is not ported).
unsafe fn enter(pl: usize, row: u32) {
    func::<ResetFn>(REACTION_RESET)(pl, 6);
    // The check: the previous action, the hit reaction (an 8-byte write: the time to 0), the sub-state, `+0x64C`,
    // `+0x7B4`.
    let cur: u32 = rd(pl + p::ACTION);
    wr(pl + p::PREV_ACTION, cur);
    wr(pl + p::ACTION, HIT_REACTION as u64);
    wr(pl + p::ACTION_SUB, 0u32);
    wr(pl + F_64C, 0u64);
    wr(pl + F_7B4, 0u32);
    // `0x1401F1200`: the status action, its velocity to zero (`0x1401A38D0`), the clip sub-state and its row.
    wr(pl + p::PREV_ACTION, HIT_REACTION);
    wr(pl + p::ACTION, h::STATUS_ACTION as u64);
    let body: usize = rd(pl + p::BODY);
    if body != 0 {
        wr(body + mv::body::VEL, [0f32, 0.0, 0.0, 1.0]);
    }
    wr(pl + p::ACTION_SUB, h::HELD_SUB);
    let blend = f32::from_bits(BLEND.load(Ordering::Relaxed));
    func::<mv::MotionFn>(mv::MOTION_REQUEST)(pl, row as i32, crate::units::frames_to_units(blend), 0, 0, 0, 0);
    wr(pl + p::CARD, rd::<u32>(pl + p::CARD) & !8);
    note(|| format!("held: Sora held (the Neoshadow's grab): status 0x18 sub 0xC, row {row:#x}, blend {blend}; Re:CoM ends it at the clip's end, a damaging hit frees him"));
}
