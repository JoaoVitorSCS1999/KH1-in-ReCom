//! KH1's Attack in Re:CoM's own attack action (`crate::attack`, `docs/recom-kh1-attack.md`).
//!
//! Hooks (`sites::ATTACK_HOOKS`), each a pass-through unless `combat = kh1` and the player is the
//! main player's `CPlayerSora`:
//!
//! - request bits `0x1401F8150`: in battle, Re:CoM's field branch builds the request (confirm →
//!   Attack `+0x424 & 4`; no card scroll, reload, stock or sleight input); a jump pressed on the same
//!   frame wins (KH1's order). First, KH1's MP refill step while a point is waiting (`super::mp`).
//! - use-card button `0x1401E8FF0`: skipped (no card plays at KH1).
//! - initAttack `0x1401F0D90`: the card attack becomes the field swing.
//! - field swing init `0x1401F0F70`: Re:CoM's command dispatch reached it, so KH1's frame runs with
//!   the Attack press (its command picks the row), and Re:CoM's init then starts that row.
//! - field swing exe `0x1401EAEB0` (also reached from the Battle exe through its jump-table entry for
//!   action `0xA`): KH1's frame (combo window, chain, facing, air physics), then Re:CoM's exe (hit
//!   window, `onAttack`, end), then our velocity and facing again.
//! - Sora's vtable slot `+0x128` (the row pick): takes the KH1 row KH1's frame chose.
//! - attack pair dispatch `0x14016EAD0`: for Sora's Keyblade on a KH1 row, KH1's sphere chain
//!   tested with Re:CoM's own sphere / column primitives.
//! - enemy hit handler `0x14017F7E0`: a KH1 hit lands during the enemy's card play (decision 14),
//!   through Re:CoM's own interruptible-card path; a melee attacker Sora's Guard blocked plays
//!   Re:CoM's own flinch (decision 27, `super::guard`); a KH1 hit charges KH1's MP (`super::mp`).
//! - the pair dispatch also hands its non-zero results to `super::guard` (decision 27: a blocked
//!   shot is turned back there).
//! - player damage check `0x1401E6DE0`: KH1's no-flinch frames (damage, no reaction).
//! - root motion step `0x14024DCA0`: Sora's root travel one update late (decision 17).

use super::movement::{self as mv, func, orig, p, rd, rd_vec, wr};
use super::{base, log, va};
use crate::attack::{self as atk, field};
use crate::movement::SwingStart;
use crate::sites::data;
use crate::units::to_kh1;
use kh1_core::{Moveset, TargetPart, Vec3};
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

pub static ORIG_REQUEST_BITS: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_CARD_BUTTON: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_INIT_ATTACK: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_FIELD_INIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_SWING_EXE: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_PAIR: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_ENEMY_HIT: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_DAMAGE_CHECK: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_ROOT_STEP: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_HURT_OFF: AtomicUsize = AtomicUsize::new(0);

/// The KH1 rows (by KH1 row id), built once from Re:CoM's rows 1–7. Never freed: `+0x3C8` points
/// into it.
static TABLE: OnceLock<&'static [[u8; 16]]> = OnceLock::new();
static MOVESET: OnceLock<Moveset> = OnceLock::new();
/// The main player whose KH1 no-flinch is on.
static NO_FLINCH: AtomicUsize = AtomicUsize::new(0);
static DUEL_LINES: AtomicU32 = AtomicU32::new(0);
/// The pending hit's reaction kind and the pending-hit count the damage check reads. Checked
/// (decomp `0x1401E6DE0`).
const HIT_KIND: usize = 0x710;
const HIT_PENDING: usize = 0x720;
/// The player in a hit reaction that started at the KH1 setting (decision 31).
static HIT_ARMED: AtomicUsize = AtomicUsize::new(0);
/// Re:CoM's hit reaction action, set only by the damage check `0x1401E6DE0`. Checked (decomp).
const HIT_REACTION: u32 = 0xC;
/// Re:CoM's hit-immunity timer: the damage check skips hits while it is above 0; the Battle /
/// Field exes subtract dt each frame; the revive sets 3000. Checked (decomp).
const HIT_IMMUNE: usize = 0x7BC;
/// Set when Sora's HP reaches 0 (the damage check's death path). Checked (decomp).
const DEAD: usize = 0x7B0;
/// Sora's frames (counted by the state exes), for the root delay.
static TICK: AtomicU64 = AtomicU64::new(0);
static ROOT: Mutex<crate::movement::RootDelay> = Mutex::new(crate::movement::RootDelay::new());
static BLADE: Mutex<atk::BladeTrack> = Mutex::new(atk::BladeTrack { serial: 0, cur_a: None, prev_a: None });
/// Enemy body parts a KH1 swing hit, until their owner's hit handler takes the hit.
static KH1_HITS: Mutex<Vec<(usize, u32)>> = Mutex::new(Vec::new());
/// Counts KH1 swing starts; a KH1 hit is kept through the next swing's start (a chain can start
/// before the enemy's update consumes the last swing's hit), and dropped after that.
static SWING_GEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// `debug_attack = on`: one-line logs of the attack's decisions, at most [`DEBUG_LINES`].
static DEBUG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static DEBUG_LEFT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(DEBUG_LINES);
const DEBUG_LINES: u32 = 200;

pub fn set_debug(on: bool) {
    DEBUG.store(on, Ordering::Relaxed);
}

/// The Config row "Guard reactions" (decision 27, `super::guard`).
pub fn set_guard_reactions(on: bool) {
    super::guard::set_on(on);
}

fn debug(line: impl FnOnce() -> String) {
    if !DEBUG.load(Ordering::Relaxed) {
        return;
    }
    if DEBUG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("debug_attack: {}", line()));
    }
}

/// The trace's hit log for the frame.
pub static HIT_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The swing KH1's frame chose, for Re:CoM's row pick inside the init it calls (set and taken
/// separately: never held across the call).
static PENDING: Mutex<Option<SwingStart>> = Mutex::new(None);

fn set_pending(s: Option<SwingStart>) {
    if let Ok(mut g) = PENDING.lock() {
        *g = s;
    }
}

fn take_pending() -> Option<SwingStart> {
    PENDING.lock().ok().and_then(|mut g| g.take())
}

// Re:CoM functions.
const END_ATTACK: u32 = 0x001F_6660;
const WEAPON_END: u32 = 0x0022_4FB0;
const ACTOR_BONE: u32 = 0x002A_BB20;
const SPHERE_SPHERE: u32 = 0x0017_77E0;
const SPHERE_COLUMN: u32 = 0x0017_6FA0;
/// `0x1401A6B20(actor, effect group name, index, position, 0, 0.0f)`: an effect of a named group at
/// a point, and the sound its group's table gives it (`0x1402588B0`). Checked (exe + decomp).
const EFFECT_AT: u32 = 0x001A_6B20;
/// The pointer to the effect group name `"always"` (`0x1407C2C40`), which the guard spark
/// `0x140182260` passes. Checked (exe: points at `"always\0"` in the file).
const EFFECT_ALWAYS: u32 = 0x007C_2C40;
/// Re:CoM's guard spark (with its clink) in `"always"` (`0x140182260`). Checked (exe).
const GUARD_SPARK: u32 = 0xB;
const RECOM_PICKER: u64 = 0x1_401F_7520;
/// The collision context the pair dispatch passes to the primitives (`DAT_14087A870`).
const COLL_CONTEXT: u64 = 0x1_4087_A870;
/// `CPlayerSora` vtable slot `+0x128` (the row pick). Checked (exe: holds `0x1401F7520`).
const SORA_PICKER_SLOT: u64 = 0x1_405A_9938 + 0x128;

type PlFn = unsafe extern "win64" fn(usize);
type PlRetFn = unsafe extern "win64" fn(usize) -> u64;
type PlIntFn = unsafe extern "win64" fn(usize, i32) -> u64;
type PairFn = unsafe extern "win64" fn(usize, usize) -> i32;
type PrimFn = unsafe extern "win64" fn(usize, usize, usize, i32) -> u8;
type BoneFn = unsafe extern "win64" fn(usize, i32) -> usize;
type ApplyFn = unsafe extern "win64" fn(usize, usize) -> i32;
type RootFn = unsafe extern "win64" fn(usize, *mut [f32; 4], *mut f32);
type VecFn = unsafe extern "win64" fn(usize, *mut [f32; 4]) -> u64;
type EffectFn = unsafe extern "win64" fn(usize, usize, u32, *const [f32; 4], u64, f32) -> i32;

pub(super) fn moveset() -> &'static Moveset {
    MOVESET.get_or_init(Moveset::kh1_defaults)
}

/// The KH1 row id `+0x3C8` points at, if it points into our table.
unsafe fn kh1_row_of(pl: usize) -> Option<usize> {
    let t = TABLE.get()?;
    let ptr: usize = rd(pl + field::ROW_PTR);
    let start = t.as_ptr() as usize;
    (ptr >= start && ptr < start + t.len() * 16 && (ptr - start).is_multiple_of(16)).then(|| (ptr - start) / 16)
}

/// Sora's body (`player+0x690`) guard fields (Checked (decomp `0x14016F1C0`, body ctor)):
/// `+0x9C` flags (1 guard, 0x20 guarded), `+0x8C` guard element mask (all by default), `+0xB0`
/// the guard's half-angle (π by default).
const BODY_GUARD: usize = 0x690 + 0x9C;
const BODY_GUARD_ANGLE: usize = 0x690 + 0xB0;
/// KH1's guard arc: cosine −0.1736 (record `0x4DA8` +0x64) = ±100°.
const KH1_GUARD_ANGLE: f32 = 1.745_329_3;
static GUARD_ON: AtomicUsize = AtomicUsize::new(0);
static GUARD_HIT: AtomicUsize = AtomicUsize::new(0);
/// Where Re:CoM's apply put the guarded hit (Sora's body `+0x30`, `player+0x6C0`, copied from the
/// attack's `+0x60` in `0x14016F1C0`'s guard branch; Checked (decomp)), kept with `GUARD_HIT`.
static GUARD_POINT: Mutex<[f32; 4]> = Mutex::new([0.0; 4]);
const BODY_HIT_POINT: usize = 0x690 + 0x30;

/// KH1's guard object live: Re:CoM's own body guard on Sora (all elements, ±100°), or off.
pub(super) unsafe fn set_guard(pl: usize, on: bool) {
    let f: u32 = rd(pl + BODY_GUARD);
    if on {
        wr(pl + BODY_GUARD, f | 1);
        wr(pl + BODY_GUARD_ANGLE, KH1_GUARD_ANGLE);
        GUARD_ON.store(pl, Ordering::Relaxed);
    } else {
        wr(pl + BODY_GUARD, f & !1);
        wr(pl + BODY_GUARD_ANGLE, std::f32::consts::PI);
        GUARD_ON.store(0, Ordering::Relaxed);
    }
    GUARD_HIT.store(0, Ordering::Relaxed);
}

/// KH1's guard object is live on this player (Re:CoM's body guard switched on by `set_guard`).
pub(super) fn guard_live(pl: usize) -> bool {
    pl != 0 && GUARD_ON.load(Ordering::Relaxed) == pl
}

/// A guarded hit since the last KH1 frame.
pub(super) unsafe fn take_guard_hit(pl: usize) -> bool {
    GUARD_HIT.swap(0, Ordering::Relaxed) == pl || rd::<u32>(pl + BODY_GUARD) & 0x20 != 0
}

/// Re:CoM's own guard spark and clink at the guarded point: what it plays when an enemy guards
/// Sora's swing (`0x140182260` → `0x1401A6B20(actor, "always", 0xB, point, 0, 0.0f)`), here with
/// Sora as the guarder. Once per guard success. If the `"always"` group isn't loaded,
/// `0x1402588B0` finds no group and does nothing (Checked (decomp)).
pub(super) unsafe fn guard_clink(pl: usize) {
    let name: usize = rd(base() + EFFECT_ALWAYS as usize);
    if name == 0 || std::slice::from_raw_parts(name as *const u8, 7) != b"always\0" {
        log("guard: the \"always\" effect name isn't where 0x1407C2C40 points; no clink");
        return;
    }
    let point = GUARD_POINT.lock().map(|g| *g).unwrap_or_else(|_| rd(pl + BODY_HIT_POINT));
    func::<EffectFn>(EFFECT_AT)(pl, name, GUARD_SPARK, &point, 0, 0.0);
}

/// M9 stage 9c.1: our attack table's row `r` (a special's), for `player+0x3C8`.
pub(super) fn row_ptr(r: usize) -> Option<usize> {
    TABLE.get().and_then(|t| t.get(r)).filter(|b| b[0] != 0).map(|b| b.as_ptr() as usize)
}

/// M9 stage 9c.1: every Sonic Blade row is in the table (their clips were converted).
pub(super) fn special_rows_ready() -> bool {
    TABLE.get().is_some_and(|t| atk::special_rows_ready(t))
}

/// M9 stage 9c.1: `player+0x3C8` points at one of our special rows.
pub(super) unsafe fn special_row_live(pl: usize) -> bool {
    kh1_row_of(pl).is_some_and(|r| (kh1_core::data::row::SONIC_BLADE_DASH..=kh1_core::data::row::SONIC_BLADE_FINISHER).contains(&r))
}

/// A new KH1 attack starts (a swing's row, M9 9c.1 a special's): the KH1 hits of attacks two
/// starts back are forgotten (as `apply_row`).
pub(super) fn new_attack() {
    let gen = SWING_GEN.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    if let Ok(mut g) = KH1_HITS.lock() {
        g.retain(|&(_, at)| gen.wrapping_sub(at) <= 1);
    }
}

/// Whether our attack table holds this KH1 row.
pub(super) fn row_ready(r: usize) -> bool {
    TABLE.get().is_some_and(|t| t.get(r).is_some_and(|b| b[0] != 0))
}

pub(super) fn set_no_flinch(pl: usize, on: bool) {
    NO_FLINCH.store(if on { pl } else { 0 }, Ordering::Relaxed);
}

/// A state exe ran for the main Sora (one per frame).
pub(super) fn tick() {
    TICK.fetch_add(1, Ordering::Relaxed);
    TICK_LIVE.store(true, Ordering::Relaxed);
}

/// The state exes count Sora's frames (the trace hooks are in): without that count the delay
/// can't tell consecutive updates, and Re:CoM's own same-frame root motion stays.
static TICK_LIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Re:CoM's attack target (`player+0x84C`) as a KH1 body part: its aim point (vtable `+0x50`, as
/// Re:CoM's own row pick reads it), with the KH1 twin's lock-point radius (`kh1-stats.json`),
/// which KH1's picker subtracts (`0x1402A7320`); 0 for enemies without a twin. Assumption: Re:CoM's
/// aim point stands for KH1's lock point.
pub(super) unsafe fn attack_target(pl: usize) -> Option<TargetPart> {
    let h: i32 = rd(pl + p::LOCK_TARGET);
    if h <= 0 {
        return None;
    }
    let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(h as u32);
    if obj == 0 {
        return None;
    }
    let vt: usize = rd(obj);
    let f: VecFn = std::mem::transmute_copy(&rd::<usize>(vt + 0x50));
    let mut out = [0f32; 4];
    f(obj, &mut out);
    let radius = super::stats::lock_radius(obj).unwrap_or(0.0);
    Some(TargetPart { owner: h as u32, centre: to_kh1(Vec3::new(out[0], out[1], out[2])), radius })
}

// ---- starting and ending swings (from `RecomHost`) ----

/// KH1's frame chose a swing: Re:CoM's field-swing init starts it, and its row pick (our vtable
/// slot) takes the row from `PENDING`.
pub(super) unsafe fn start_swing(pl: usize, s: &SwingStart) -> bool {
    let Some(t) = TABLE.get() else { return false };
    if t.get(s.kh1_row).is_none_or(|r| r[0] == 0) {
        return false;
    }
    // A chain: end the last swing's volume first, as Re:CoM's own chain does (`0x1401EAFC2`).
    let w: usize = rd(pl + field::WEAPON);
    if w != 0 && rd::<u32>(pl + p::ACTION) == crate::movement::action::ATTACK {
        func::<PlFn>(WEAPON_END)(w);
    }
    set_pending(Some(*s));
    debug(|| {
        let b: usize = rd(pl + p::BODY);
        let pos = if b != 0 { to_kh1(rd_vec(b + 0x30)) } else { Vec3::ZERO };
        let t = attack_target(pl);
        format!(
            "start row={} id={} req={} root={} n={}/{} fin={} action={:#x} pos=({:.1},{:.1},{:.1}) target={:?} lock={}",
            s.kh1_row, s.recom_id, s.request, s.root, s.counter, s.length, s.finisher, rd::<u32>(pl + p::ACTION), pos.x, pos.y, pos.z,
            t.map(|t| (t.centre.x.round(), t.centre.y.round(), t.centre.z.round())),
            rd::<i32>(pl + p::BATTLE) != 0 && rd::<usize>(pl + p::TARGET_CURSOR) != 0 && rd::<i32>(rd::<usize>(pl + p::TARGET_CURSOR) + 0xE0) != 0
        )
    });
    let o: PlIntFn = orig(&ORIG_FIELD_INIT);
    o(pl, s.counter.saturating_sub(1) as i32);
    // The row pick took it, unless something replaced Sora's vtable slot.
    take_pending().is_none()
}

/// Re:CoM ends its attack action (`0x1401F6660`). True when it entered its win action instead
/// (`0x1401F6794`: the battle's win is pending, `player+0x428 & 4`).
pub(super) unsafe fn end_swing(pl: usize, airborne: bool) -> bool {
    func::<PlIntFn>(END_ATTACK)(pl, airborne as i32);
    mv::in_recom_win(pl)
}

/// Re:CoM's row pick for a KH1 row (`0x1401F7520`'s writes, cardless).
unsafe fn apply_row(pl: usize, s: &SwingStart) -> bool {
    let Some(t) = TABLE.get() else { return false };
    let Some(row) = t.get(s.kh1_row).filter(|r| r[0] != 0) else { return false };
    wr(pl + field::HOMING_TARGET, 0i32);
    wr(pl + field::VSPEED, 0f32);
    wr(pl + field::SITUATION, s.recom_id);
    wr(pl + field::ATTACK_ID, s.recom_id);
    wr(pl + field::ROW_PTR, row.as_ptr() as usize);
    func::<mv::MotionFn>(mv::MOTION_REQUEST)(pl, row[0] as i32, 0, s.root as u64, 0, 0, 0);
    wr(pl + field::CARD_7B4, 0i32);
    // Re:CoM's cardless AP (the attack stat): at Damage = KH1 the KH1 numbers replace it per hit
    // (`enemy_hit`); in the field it is Re:CoM's own field-swing AP.
    wr(pl + field::AP, rd::<i32>(pl + field::ATTACK_STAT));
    wr(pl + field::KIND, row[0xB] as u32);
    wr(pl + field::ATTR, 0u32);
    // Damage = Re:CoM (owner, 2026-11-01), in battle: the row pick's card AP for the equipped
    // Keyblade's card at this row's column, its attribute, Re:CoM's enemy-card rewrite.
    super::damage::swing(pl, s.kh1_row);
    wr(pl + field::CHAINABLE, 0u32);
    wr(pl + field::PREV_SITUATION, s.recom_id);
    wr(pl + field::COUNTER, s.counter as i32);
    wr(pl + field::LENGTH, s.length as i32);
    // KH1 has no post-swing lockout, and a swing starts from standing (root travel only).
    wr(pl + field::LOCKOUT, 0i32);
    wr(pl + field::SPEED_4E0, 0f32);
    new_attack();
    true
}

// ---- hooks ----

unsafe fn battle_guards_clear(pl: usize, req: u32) -> bool {
    // The Battle branch's own early returns (`0x1401F8150`): keep them Re:CoM's.
    rd::<u32>(pl + p::FLAGS_418) & 0x4_0000 == 0 && rd::<i32>(pl + 0x7B0) == 0 && rd::<u32>(pl + p::CARD) & 4 == 0 && rd::<u32>(pl + p::FLAGS_420) & 0x200_0000 == 0 && req & 0x10_0000 == 0
}

pub unsafe extern "win64" fn request_bits(pl: usize) {
    let o: PlFn = orig(&ORIG_REQUEST_BITS);
    if !mv::kh1_sora(pl) {
        return o(pl);
    }
    // KH1's MP refill runs in the actor pre-step, at the start of the frame (`0x1402A43A0` →
    // `0x1402A4CB0`): one point per frame while one is waiting (M7 stage 7.1).
    super::mp::frame();
    // Boss revenge: the game clock its decay reads (one add).
    super::revenge::frame(pl);
    // A magic command no KH1 frame took last frame is gone (M7 stage 7.4).
    super::magic::new_frame();
    // A Thunder's strikes (7.6): its script thread's run (nothing unless one is going).
    super::magic::strikes_frame(pl);
    // An item command no KH1 frame took last frame is gone; KH1's item use tasks run (M8 stage
    // 8a.1: one atomic load unless a use is going).
    super::items::new_frame();
    super::items::task_frame(pl);
    // A special command no KH1 frame took last frame is gone (M9 stage 9c.1).
    super::specials::new_frame();
    // Re:CoM's battle end, logged (two loads unless its win is pending; `crate::battle::end`).
    super::battle::end_watch(pl);
    let battle: i32 = rd(pl + p::BATTLE);
    let forced = battle != 0 && battle_guards_clear(pl, rd(pl + p::REQUEST));
    if forced {
        // Re:CoM's own field branch: the confirm press is the Attack, no card input.
        wr(pl + p::BATTLE, 0i32);
        o(pl);
        wr(pl + p::BATTLE, battle);
    } else {
        o(pl);
    }
    // KH1's command menu (M7 stage 7.3): confirm runs the highlighted row (Attack only from the
    // Attack row), the jump only from the root list (cancel closes a submenu), Triangle only
    // when the context row's tab is in.
    // KH1's LOCK ON! plate and target gauge (decision 39, H5) follow Re:CoM's lock.
    super::kh1_hud::lock_tick(pl);
    if let Some(k) = super::command_menu::frame(pl) {
        let mut r: u32 = rd(pl + p::REQUEST);
        if !k.attack {
            r &= !4;
        }
        if !k.jump {
            r &= !1;
        }
        if !k.triangle {
            r &= !0x200;
        }
        // Stage 7.12: while the shortcut list is up Square is slot 2, not the roll / guard.
        if !k.square {
            r &= !2;
        }
        wr(pl + p::REQUEST, r);
    }
    // A played gimmick card's request (Re:CoM's battle-branch conversion the field branch skips).
    super::gimmick::request(pl, forced);
    // KH1: a jump on the same frame wins over the Attack (`0x1402A6220`).
    let r: u32 = rd(pl + p::REQUEST);
    if r & 1 != 0 && r & 4 != 0 {
        wr(pl + p::REQUEST, r & !4);
    } else if r & 4 != 0 {
        // M9 9a: an Attack press: the ability state follows Re:CoM's sleights and level before KH1's
        // pickers run (only on presses, not per frame).
        super::abilities::sync();
    }
}

pub unsafe extern "win64" fn card_button(pl: usize) -> u64 {
    if mv::kh1_sora(pl) {
        return 0;
    }
    orig::<PlRetFn>(&ORIG_CARD_BUTTON)(pl)
}

pub unsafe extern "win64" fn init_attack(pl: usize, a: i32) -> u64 {
    if mv::kh1_sora(pl) {
        field_init(pl, 0);
        return 0;
    }
    orig::<PlIntFn>(&ORIG_INIT_ATTACK)(pl, a)
}

pub unsafe extern "win64" fn field_init(pl: usize, counter: i32) -> u64 {
    let o: PlIntFn = orig(&ORIG_FIELD_INIT);
    if !mv::kh1_sora(pl) || TABLE.get().is_none() {
        return o(pl, counter);
    }
    // Re:CoM's dispatch saw the Attack: KH1's frame decides (its command, its picker) and starts
    // the swing through `start_swing`.
    let fresh = mv::dispatch_fresh(pl);
    debug(|| format!("dispatch action={:#x} fresh={fresh} req={:#x}", rd::<u32>(pl + p::ACTION), rd::<u32>(pl + p::REQUEST)));
    mv::run_frame(pl, fresh);
    0
}

pub unsafe extern "win64" fn swing_exe(pl: usize) {
    let o: PlFn = orig(&ORIG_SWING_EXE);
    if !mv::kh1_sora(pl) || kh1_row_of(pl).is_none() {
        return o(pl);
    }
    if !mv::continues(pl) {
        // Re:CoM ran something else in between: its exe finishes this swing.
        return o(pl);
    }
    mv::run_frame(pl, false);
    debug(|| {
        let c: usize = rd(pl + p::CTRL);
        let b: usize = rd(pl + p::BODY);
        let v = if b != 0 { rd_vec(b + 0x80) } else { Vec3::ZERO };
        format!(
            "exe action={:#x} frame={:.1} vel=({:.3},{:.3},{:.3}) contact={:#x} hit={}",
            rd::<u32>(pl + p::ACTION),
            if c != 0 { rd::<i32>(c + 0x3C) as f32 / 50.0 } else { -1.0 },
            v.x,
            v.y,
            v.z,
            if b != 0 { rd::<u32>(b + 0x120) } else { 0 },
            rd::<i32>(pl + field::HIT)
        )
    });
    if rd::<u32>(pl + p::ACTION) == crate::movement::action::ATTACK {
        o(pl);
        if rd::<u32>(pl + p::ACTION) == crate::movement::action::ATTACK {
            mv::reapply(pl);
        }
    }
}

/// Sora's vtable slot `+0x128`.
pub unsafe extern "win64" fn picker(pl: usize) {
    if mv::kh1_sora(pl) {
        if let Some(s) = take_pending() {
            if apply_row(pl, &s) {
                return;
            }
        }
    }
    let f: PlFn = std::mem::transmute_copy(&va(base(), RECOM_PICKER));
    f(pl)
}

/// A fake Re:CoM sphere shape (`CSphereCollision` fields the primitives read: `+0x10` / `+0x20`
/// the swept centres, `+0x70` radius, `+0x74` height offset; they write `+0x30..+0x60`).
#[repr(C, align(16))]
struct FakeSphere([u8; 0x90]);

/// Re:CoM's own pair dispatch, and the Guard reactions (decision 27) on its non-zero results: a
/// block of Sora's KH1 Guard (result 2 on the player side) may turn a shot back, which answers 0
/// for that one test (`super::guard::on_result`). One compare per call otherwise.
unsafe fn recom_pair(coll: usize, list: usize) -> i32 {
    let r = orig::<PairFn>(&ORIG_PAIR)(coll, list);
    if r == 0 {
        return r;
    }
    super::guard::on_result(coll, list, r)
}

pub unsafe extern "win64" fn pair_dispatch(coll: usize, list: usize) -> i32 {
    if !mv::kh1_on() {
        return recom_pair(coll, list);
    }
    let b = base();
    let pl: usize = rd(va(b, data::MAIN_PLAYER));
    if !mv::is_main_sora(pl) {
        return recom_pair(coll, list);
    }
    let weapon: usize = rd(pl + field::WEAPON);
    let shape: usize = rd(coll + 0x18);
    let Some(kh1_row) = kh1_row_of(pl) else { return recom_pair(coll, list) };
    if weapon == 0 || coll != weapon + 0x240 || shape == 0 {
        return recom_pair(coll, list);
    }
    let kh1_weapon = super::stats::equipped_weapon();
    let Some(live) = atk::live_attack(moveset(), kh1_row, &kh1_weapon) else { return recom_pair(coll, list) };
    let bone = func::<BoneFn>(ACTOR_BONE);
    let (a, bp) = if live.data.bone == atk::KH1_WEAPON_BONE {
        let (b2, b3) = (bone(weapon, 2), bone(weapon, 3));
        if b2 == 0 || b3 == 0 {
            return recom_pair(coll, list);
        }
        let (w2, w3) = (rd_vec(b2 + 0xD0), rd_vec(b3 + 0xD0));
        atk::blade_points(w2, w3, &live, &kh1_weapon)
    } else {
        // M9 9a: a record on one of Sora's own bones (Stun Impact: bone 8, a 2.5 m sphere), tested
        // while Re:CoM's weapon volume is live (the row's hit window), at Re:CoM's bone of Sora that
        // stands for it, with the same primitives.
        let Some(rb) = atk::recom_body_bone(live.data.bone) else { return recom_pair(coll, list) };
        let at = bone(pl, rb);
        if at == 0 {
            return recom_pair(coll, list);
        }
        let c = rd_vec(at + 0xD0);
        (c, c)
    };
    let serial: i32 = rd(coll + 0x70);
    let prev = BLADE.lock().map(|mut t| t.frame(serial, a)).unwrap_or(None);
    // Assumption: Sora's centre for the first frame's sweep is `player+0x4B0` (the attack origin
    // `onAttack` copies).
    let owner = rd_vec(pl + 0x4B0);
    let spheres = atk::blade_shapes(a, bp, live.radius * crate::units::METRES_PER_UNIT, live.data.sweeps(), prev, owner);
    let ctx: usize = rd(va(b, COLL_CONTEXT));
    let owner_id: i32 = rd(coll + 0x20);
    let vt: usize = rd(coll);
    let apply: ApplyFn = std::mem::transmute_copy(&rd::<usize>(vt + 0x38));
    let mut fake = FakeSphere([0; 0x90]);
    let fp = fake.0.as_mut_ptr() as usize;
    wr(fp + 0x60, 2u32);
    let mut result = 0;
    let mut node: usize = rd(list + 8);
    while node != 0 {
        let next: usize = rd(node + 8);
        if rd::<i32>(node + 0x10) != 0 && rd::<i32>(node + 0x20) != owner_id {
            let bshape: usize = rd(node + 0x18);
            let flags = if bshape == 0 { 0xFFu8 } else { rd::<u32>(bshape + 100) as u8 };
            if flags & 0xE != 6 && bshape != 0 {
                if spheres_meet(ctx, fp, bshape, &spheres) {
                    // The contact the apply reads from the attack's shape (`+0x30..+0x60`).
                    std::ptr::copy_nonoverlapping((fp + 0x30) as *const u8, (shape + 0x30) as *mut u8, 0x30);
                    // M11, the shared enemy-guard step: a body under a KH1 enemy's guard row guards
                    // this swing in Re:CoM's apply only when KH1's types meet (`0x140528CB8`), for
                    // its element whatever it is (`super::kh1_enemy::guard_for_swing`).
                    let mut guard = super::kh1_enemy::guard_for_swing(node, coll, live.data.clash_type());
                    let r = apply(coll, node);
                    if let Some(g) = guard.as_mut() {
                        super::kh1_enemy::guard_after_swing(g, r, live.data.clash_type(), live.priority);
                        if let Some((kind, diff)) = g.sora {
                            SWING_CLASH_KIND.store((kind as u32) << 16 | (diff as i16 as u16 as u32), Ordering::Relaxed);
                            SWING_CLASH.store(pl, Ordering::Relaxed);
                        }
                    }
                    if r != 0 {
                        if let Ok(mut g) = KH1_HITS.lock() {
                            if g.len() < 64 {
                                g.push((node, SWING_GEN.load(Ordering::Relaxed)));
                            }
                        }
                    }
                    if result == 0 {
                        result = r;
                    }
                }
            }
        }
        node = next;
    }
    // M11: the swing against the KH1 enemies' flying thrown objects (KH1's attack-vs-attack test,
    // `0x1402B4A20`), with the same spheres and Re:CoM's primitives (`super::kh1_thrown`).
    super::kh1_thrown::swing_at_blades(|bshape| spheres_meet(ctx, fp, bshape, &spheres));
    result
}

/// Whether one of the swing's spheres meets Re:CoM shape `bshape` (a sphere: kind 2; a column: 4 /
/// 5), through Re:CoM's own overlap tests on the fake sphere `fp` (its contact left in `fp + 0x30`).
unsafe fn spheres_meet(ctx: usize, fp: usize, bshape: usize, spheres: &[(Vec3, f32)]) -> bool {
    let kind: u32 = rd(bshape + 0x60);
    for &(c, r) in spheres {
        for o4 in [0x10, 0x20] {
            wr(fp + o4, [c.x, c.y, c.z, 1.0f32]);
        }
        wr(fp + 0x70, r);
        wr(fp + 0x74, 0f32);
        let hit = match kind {
            2 => func::<PrimFn>(SPHERE_SPHERE)(ctx, bshape, fp, 0) != 0,
            4 | 5 => func::<PrimFn>(SPHERE_COLUMN)(ctx, bshape, fp, 0) != 0,
            _ => false,
        };
        if hit {
            return true;
        }
    }
    false
}

/// Re:CoM's guard spark and clink at `point` with no actor (as `0x140182260` plays it for an
/// enemy's guard: actor 0); nothing when the `"always"` name isn't where it was.
pub(super) unsafe fn spark_at(point: [f32; 4]) {
    let name: usize = rd(base() + EFFECT_ALWAYS as usize);
    if name == 0 || std::slice::from_raw_parts(name as *const u8, 7) != b"always\0" {
        return;
    }
    func::<EffectFn>(EFFECT_AT)(0, name, GUARD_SPARK, &point, 0, 0.0);
}

/// M11, the shared enemy-guard step: Sora's swing met a KH1 enemy's guard row (the player, and
/// KH1's clash kind << 16 | the priority difference as an i16).
static SWING_CLASH: AtomicUsize = AtomicUsize::new(0);
static SWING_CLASH_KIND: AtomicU32 = AtomicU32::new(0);

/// The clash Sora's KH1 frame plays (`Kh1Sora::swing_clash`), once.
pub(super) fn take_swing_clash(pl: usize) -> Option<(u8, i32)> {
    if pl == 0 || SWING_CLASH.load(Ordering::Relaxed) != pl {
        return None;
    }
    SWING_CLASH.store(0, Ordering::Relaxed);
    let k = SWING_CLASH_KIND.load(Ordering::Relaxed);
    Some(((k >> 16) as u8, k as u16 as i16 as i32))
}

/// Whether one of the enemy's body parts took a KH1 swing's hit (consumed).
unsafe fn take_kh1_hit(enemy: usize) -> Option<usize> {
    let info: usize = rd(enemy + 0xF8);
    if info == 0 {
        return None;
    }
    let parts: usize = rd(info + 0x40);
    let n: i32 = rd(info + 0x38);
    let Ok(mut g) = KH1_HITS.lock() else { return None };
    for i in 0..n.clamp(0, 32) as usize {
        let part = parts + i * 0x120;
        if let Some(k) = g.iter().position(|&(x, _)| x == part) {
            if rd::<i16>(part + 0x90) != 0 {
                g.swap_remove(k);
                return Some(part);
            }
        }
    }
    None
}

/// The hit Re:CoM's handler is about to take on `enemy` (M11, the no-reaction armour), read before the
/// call without taking anything: its gather's pick (`kh1_core::enemy::host::gather`, `0x140154EB0`) over
/// the hurt bodies (`+0xF8` → `+0x40`, count `+0x38`, 0x120 each). None: no damaging hit waits (nothing,
/// or a guarded / repelled one: the handler's guard branch). Some(kind): KH1's reaction kind as
/// `kh1_enemy::hit_of` reads it after the call: a KH1 swing's (one of its bodies in `KH1_HITS`, as
/// `take_kh1_hit`; the kind of Sora's row, which the wrapper passes to `note_swing` only after the call),
/// else the waiting body's Re:CoM kind `+0x80` mapped (written by the hit apply `0x14016F1C0` with the
/// count; Checked (decomp)); Some(None) when neither gives one.
unsafe fn incoming(enemy: usize) -> Option<Option<u8>> {
    let info: usize = rd(enemy + 0xF8);
    if info == 0 {
        return None;
    }
    let parts: usize = rd(info + 0x40);
    let n = rd::<i32>(info + 0x38).clamp(0, 32) as usize;
    if parts == 0 {
        return None;
    }
    let body = |i: usize| parts + i * 0x120;
    let at = match kh1_core::enemy::host::gather((0..n).map(|i| (rd::<u32>(body(i) + 0x9C), rd::<i16>(body(i) + 0x90)))) {
        kh1_core::enemy::host::Gathered::Hit(i) => body(i),
        _ => return None,
    };
    let swing = KH1_HITS.lock().map(|g| (0..n).any(|i| rd::<i16>(body(i) + 0x90) != 0 && g.iter().any(|&(x, _)| x == body(i)))).unwrap_or(false);
    let pl: usize = rd(va(base(), data::MAIN_PLAYER));
    let kind = swing.then(|| if pl != 0 { kh1_row_of(pl) } else { None }).flatten().and_then(|r| moveset().row(r)).and_then(|x| moveset().attack(x.attack)).map(|a| a.kind);
    Some(kind.or_else(|| Some(crate::shadow::kh1_kind_of_recom(rd(at + 0x80)))))
}

/// Re:CoM's enemy hit handler. KH1's rule (decision 14): Sora's hits land during the enemy's card
/// play. Re:CoM's own path for an interruptible card (`+0x260 & 0x2000001`: end the card, then
/// take the hit) runs for a KH1 hit.
pub unsafe extern "win64" fn enemy_hit(enemy: usize) -> u64 {
    if !mv::kh1_on() {
        return orig::<PlRetFn>(&ORIG_ENEMY_HIT)(enemy);
    }
    let hp: i32 = rd(enemy + 0x274);
    // M11 (the Pirate; the fliers' "doesn't flinch"): Re:CoM's super armour for the call while a KH1
    // enemy's script takes no reaction for the hit about to be handled, then the hit to its script.
    let armour = super::kh1_enemy::armour_on(enemy, || incoming(enemy));
    let r = kh1_enemy_hit(enemy);
    super::kh1_enemy::armour_off(armour, r);
    // Gimmick cards, trigger (d): a card-break boss crossing 50 % of its max HP.
    super::gimmick::after_hp(enemy, hp);
    r
}

unsafe fn kh1_enemy_hit(enemy: usize) -> u64 {
    let o: PlRetFn = orig(&ORIG_ENEMY_HIT);
    // M11, the shared enemy-guard step: a guard row left on while a Re:CoM state runs ends (one
    // atomic load while no KH1 enemy guards).
    super::kh1_enemy::guard_watch(enemy);
    // M7 f3 (decision 38): KH1's pace for the hits Stop held (one compare unless they are landing).
    super::magic::held_hit_pace(enemy);
    let Some(part) = take_kh1_hit(enemy) else {
        // M7 stage 7.4: one of our KH1 spell shots hit this enemy.
        if let Some((part, s, lv)) = super::magic::take_hit(enemy) {
            return spell_hit(enemy, part, s, lv, o);
        }
        let hp: i32 = rd(enemy + 0x274);
        // Decision 28: a shot Sora's Guard turned back breaks into the card like a KH1 swing.
        let flags: u64 = rd(enemy + 0x260);
        let force = flags & 1 != 0 && flags & 0x200_0000 == 0 && super::guard::turned_hit_pending(enemy);
        if force {
            wr(enemy + 0x260, flags | 0x200_0000);
        }
        let r = o(enemy);
        if force {
            let now: u64 = rd(enemy + 0x260);
            wr(enemy + 0x260, now & !0x200_0000);
        }
        // Decision 27: a melee attacker Sora's Guard blocked this frame reacts here, where Re:CoM
        // runs its own reactions.
        super::guard::after_enemy_hit(enemy, r, hp);
        return r;
    };
    // KH1's damage (decision 12) as the hit part's value (`+0x92`, read by `0x14016FDB0`); Re:CoM's
    // own path then applies its weak part, resistance % and boss falloff (decision 13).
    let pl: usize = rd(va(base(), data::MAIN_PLAYER));
    let row = if pl != 0 { kh1_row_of(pl) } else { None };
    super::revenge::note_swing(row);
    let hp: i32 = rd(enemy + 0x274);
    // Damage = Re:CoM (owner, 2026-11-01): the part carries Re:CoM's card / sleight AP from the
    // swing's start (`super::damage`); Re:CoM's path applies it as is.
    let recom = !super::damage::kh1();
    let carried: i16 = rd(part + 0x92);
    // M9 9a: the % of HP rows (Gravity Break, Zantetsuken) read the current HP; Ripple Drive Sora's
    // max MP (KH1's default 2 when the MP block isn't live).
    let max_mp = super::mp::current().map_or(kh1_core::AttackerStats::default().max_mp, |v| v.1);
    let kh1 = if recom { None } else { row.and_then(|r| super::stats::kh1_hit(enemy, r, super::stats::sora_level(), hp, max_mp)) };
    if let Some(d) = kh1 {
        wr(part + 0x92, d.clamp(0, i16::MAX as i32) as i16);
    }
    // Damage = Re:CoM (owner, 2026-11-01, follow-ups): Gravity Break and Zantetsuken take KH1's
    // percentage of the enemy's Re:CoM current HP instead of the card's column 2.
    let pct = if recom { row.and_then(|r| super::stats::recom_percent_hit(enemy, r, super::stats::sora_level(), hp, max_mp)) } else { None };
    if let Some(d) = pct {
        wr(part + 0x92, d.clamp(0, i16::MAX as i32) as i16);
        super::damage::note(|| format!("swing row {}: KH1's % of Re:CoM's HP {hp} = {d} (was the card's AP {carried})", row.unwrap_or(0)));
    }
    // Damage = Re:CoM (owner, 2026-11-01, follow-ups 2): Gravity Break and Zantetsuken skip
    // Re:CoM's boss falloff: its multiplier at 1.0 for the handler, its three fields put back after
    // (as for revenge bosses). Its frozen bonus, resistance and weak parts stay.
    let falloff = row.filter(|&r| crate::damage::skips_falloff(super::damage::kh1(), r, super::attack::moveset()) && pct.is_some()).map(|_| super::revenge::Falloff::save(enemy));
    if falloff.is_some() {
        super::revenge::Falloff::neutral(enemy);
    }
    let flags: u64 = rd(enemy + 0x260);
    let force = flags & 1 != 0 && flags & 0x200_0000 == 0;
    if force {
        wr(enemy + 0x260, flags | 0x200_0000);
    }
    let r = o(enemy);
    if force {
        let now: u64 = rd(enemy + 0x260);
        wr(enemy + 0x260, now & !0x200_0000);
    }
    if let Some(f) = falloff {
        f.restore(enemy);
        if f.mult() < 1.0 {
            super::damage::note(|| format!("swing row {}: Re:CoM's boss falloff skipped (its multiplier was {:.2})", row.unwrap_or(0), f.mult()));
        }
    }
    // M10 / M11: a KH1 enemy reacts with the swing's KH1 reaction kind.
    if let Some(k) = row.and_then(|r| moveset().row(r)).and_then(|x| moveset().attack(x.attack)).map(|a| a.kind) {
        super::kh1_enemy::note_swing(enemy, k);
    }
    super::guard::after_enemy_hit(enemy, r, hp);
    // Gimmick cards, option (c): a KH1 finisher that interrupted a card-break boss's attack.
    super::gimmick::after_kh1_hit(enemy, row, force);
    // KH1's MP charge for this hit (`0x1402BF330`: one per KH1 damage record; M7 stage 7.1).
    super::mp::on_kh1_hit(enemy, row);
    let after: i32 = rd(enemy + 0x274);
    if let Ok(mut g) = HIT_LOG.lock() {
        if g.len() < 16 {
            let tag = match (recom, row) {
                (true, Some(_)) if pct.is_some() => format!(":kh1pct {}", pct.unwrap_or(0)),
                (true, Some(r)) => super::damage::swing_tag(r, carried as i32),
                _ => kh1.map_or(String::new(), |d| format!(":kh1 {d}")),
            };
            g.push(format!("e{:x}:dmg {}:hp {}{}{}", enemy & 0x00FF_FFFF, hp - after, after, if force { ":card" } else { "" }, tag));
        }
    }
    r
}

/// Whether a hit is waiting on one of the enemy's hurt bodies for Re:CoM's handler: a body's hit
/// count `+0x90` (or its guarded / repelled bits `+0x9C & 0x28`), which the handler's gather
/// `0x140154EB0` reads and clears. The handler runs every frame for every enemy (its only caller
/// `0x140185630`), so this tells a real hit from the frame's call. Checked (decomp).
unsafe fn hit_arrived(enemy: usize) -> bool {
    let info: usize = rd(enemy + 0xF8);
    if info == 0 {
        return false;
    }
    let parts: usize = rd(info + 0x40);
    let n: i32 = rd(info + 0x38);
    (0..n.clamp(0, 32) as usize).any(|i| {
        let part = parts + i * 0x120;
        rd::<i16>(part + 0x90) != 0 || rd::<u32>(part + 0x9C) & 0x28 != 0
    })
}

/// Revenge moves untouchable (owner, 2026-11-02): the frame's handler call for a boss in its revenge
/// move (`super::revenge::untouchable`; revenge-started or its own choice). Re:CoM's handler decides
/// with its own rule for an attacking enemy: no card-interrupt bit (decision 14's `0x2000000` not set
/// for any source: swing, spell, Stop, turned-back shot), so it drops the hit while `+0x260 & 1`
/// (`0x14017F8CB`): no damage, no reaction, the attack goes on.
///
/// Captain Hook's ctor `0x1403C6340` sets `0x2000000` for good (Re:CoM's "always interruptible"), which
/// would make that rule end his attack and take the hit: the bit is held off for this call and put
/// back after. The KH1 swing's / spell's hit record is taken so it can't land on a later call; no MP
/// charge (no hit landed). The Guard rule still sees the call (it skips a boss in its revenge move).
/// Returns Re:CoM's result and, when a hit had arrived, what it was.
pub(super) unsafe fn untouchable_hit(enemy: usize) -> (u64, Option<crate::revenge::IgnoredKind>) {
    use crate::revenge::IgnoredKind;
    let o: PlRetFn = orig(&ORIG_ENEMY_HIT);
    let arrived = hit_arrived(enemy);
    let kind = if take_kh1_hit(enemy).is_some() {
        Some(IgnoredKind::Swing)
    } else if super::magic::take_hit(enemy).is_some() {
        Some(IgnoredKind::Spell)
    } else {
        arrived.then_some(IgnoredKind::Other)
    };
    let hp: i32 = rd(enemy + 0x274);
    let flags: u64 = rd(enemy + 0x260);
    let held = flags & 0x200_0000 != 0;
    if held {
        wr(enemy + 0x260, flags & !0x200_0000);
    }
    let r = o(enemy);
    if held {
        let now: u64 = rd(enemy + 0x260);
        wr(enemy + 0x260, now | 0x200_0000);
    }
    super::guard::after_enemy_hit(enemy, r, hp);
    let after: i32 = rd(enemy + 0x274);
    if let Some(k) = kind {
        if let Ok(mut g) = HIT_LOG.lock() {
            if g.len() < 16 {
                g.push(format!("e{:x}:untouchable:{k:?}:dmg {}:hp {after}{}", enemy & 0x00FF_FFFF, hp - after, if held { ":hook bit held" } else { "" }));
            }
        }
    }
    (r, kind)
}

/// A KH1 spell shot's hit (M7 stage 7.4): KH1's spell damage (power + max MP through KH1's curve,
/// the twin's or proxy's Defence, floor, cap and the element's resistance) as the hit part's
/// value, as for a swing; Re:CoM's own path then applies its weak part and boss falloff
/// (decision 13). Re:CoM's own resistance for the shot's attribute is not applied on top (KH1's
/// is in the number): the part's attribute reads as one Re:CoM's table doesn't cover for the
/// call. The card-interrupt bit as for a swing (decision 14). Spells charge no MP (KH1 `+0x2C`
/// = 0).
unsafe fn spell_hit(enemy: usize, part: usize, s: u8, lv: u8, o: PlRetFn) -> u64 {
    super::revenge::note_spell(s);
    if s == crate::magic::spell::STOP {
        return stop_hit(enemy, part, lv, o);
    }
    let max_mp = super::mp::current().map(|v| v.1);
    let hp_now: i32 = rd(enemy + 0x274);
    // Damage = Re:CoM (owner, 2026-11-01): Fire / Blizzard / Thunder / Gravity / Aero's wind carry
    // their Re:CoM card's / sleight's AP and attribute from the cast (`super::damage::spell`),
    // Gravity also Re:CoM's kind 0xD (its % of the current HP, its gate `+0x40`, its frozen bonus and
    // its 3-second flatten); Re:CoM's path applies them (its resistance included).
    let recom = super::damage::recom_spell_hits(s);
    let carried: i16 = rd(part + 0x92);
    let mut kh1 = if recom { None } else { crate::magic::spell_attack(s, lv).and_then(|data| max_mp.and_then(|m| super::stats::kh1_spell_hit(enemy, &data, m, hp_now))) };
    // Gravity on an enemy without KH1 numbers (7.8): KH1's % of its current HP at 100 % (KH1's
    // ordinary Heartless), at least 1. Re:CoM's own AP is no Gravity number at KH1's rules.
    if !recom && kh1.is_none() && s == crate::magic::spell::GRAVITY {
        kh1 = max_mp.map(|m| crate::magic::gravity_damage(lv, m, hp_now, 1.0, 0));
    }
    let attr: u32 = rd(part + super::magic::PART_ATTR);
    if let Some(d) = kh1 {
        wr(part + 0x92, d.clamp(0, i16::MAX as i32) as i16);
        wr(part + super::magic::PART_ATTR, super::magic::NEUTRAL_ATTR);
    }
    // Re:CoM's own gate for its kind (Gravity `+0x40`) at Damage = Re:CoM: an immune enemy's attack
    // goes on (no card interrupt for a hit Re:CoM refuses), as Stop's.
    let gate = if recom { super::damage::recom_gate_open(enemy, s) } else { None };
    let flags: u64 = rd(enemy + 0x260);
    let force = crate::damage::interrupts(flags, gate);
    if force {
        wr(enemy + 0x260, flags | 0x200_0000);
    }
    let hp: i32 = rd(enemy + 0x274);
    let r = o(enemy);
    if force {
        let now: u64 = rd(enemy + 0x260);
        wr(enemy + 0x260, now & !0x200_0000);
    }
    // A stopped enemy holds the hit (Re:CoM's stop exe, `0x1401843A0`, reads the part's attribute
    // for its own resistance later in this update): the neutral attribute stays until the next
    // apply rewrites it, so the held hit lands with KH1's resistance only (decision 36).
    if kh1.is_some() && !super::magic::stopped(enemy) {
        wr(part + super::magic::PART_ATTR, attr);
    }
    super::guard::after_enemy_hit(enemy, r, hp);
    let after: i32 = rd(enemy + 0x274);
    let name = crate::magic::spell::NAMES.get(s as usize).copied().unwrap_or("?");
    let flat = recom && s == crate::magic::spell::GRAVITY && super::magic::flattening(enemy);
    super::magic::hit_line(|| {
        let what = if recom { format!("Re:CoM AP {carried} ({})", super::damage::spell_tag(s, lv, carried as i32).trim_start_matches(':')) } else { format!("KH1 {}", kh1.map_or("- (no KH1 numbers: Re:CoM's)".to_string(), |d| d.to_string())) };
        let g = match gate {
            Some(true) => ", Re:CoM's gate open",
            Some(false) => ", Re:CoM's gate shut: immune",
            None => "",
        };
        format!("hit {name}{lv} on {:#x}: {what} (max MP {}){g}, Re:CoM took {} -> HP {after}{}{}", enemy & 0x00FF_FFFF, max_mp.unwrap_or(0), hp - after, if force { " (card broken)" } else { "" }, if flat { ", Re:CoM's flatten state" } else { "" })
    });
    if let Ok(mut g) = HIT_LOG.lock() {
        if g.len() < 16 {
            let tag = if recom { super::damage::spell_tag(s, lv, carried as i32) } else { kh1.map_or(String::new(), |d| format!(":kh1 {d}")) };
            g.push(format!("e{:x}:dmg {}:hp {after}{}:{name}{lv}{}", enemy & 0x00FF_FFFF, hp - after, if force { ":card" } else { "" }, tag));
        }
    }
    r
}

/// A KH1 Stop strike (M7 stage 7.9): KH1's rule on Re:CoM's stop. The twin's / proxy's `resist[9]`
/// decides instead of Re:CoM's gate (param `+0x2C`, opened or shut for the call: 0 = immune, as
/// KH1); the hit's value is KH1's duration in Re:CoM's ticks ((2 + max MP) × 60 × resist[9] frames
/// × 50), which Re:CoM's kind 8 (`0x14017DC50`) puts in `+0x2D8` and its stop state's begin
/// (`0x14017C740`) in its timer. Re:CoM then holds the hits the enemy takes and lands them when the
/// stop ends (its stop exe `0x1401843A0`). Once stopped, the enemy's own attack bodies go off (KH1
/// skips a stopped actor's attacks). An enemy with no KH1 record takes 100 %.
///
/// An enemy in an attack (`+0x260 & 1`) gets the card-interrupt bit for the call, as every other
/// KH1 hit (decision 14; `crate::magic::stop::interrupts`): Re:CoM's handler drops any hit on an
/// attacking enemy without it (`0x14017F8CB`), before its kind 8 (owner's 0.53.0 log: no strike
/// entered the stop state). A duration longer than the i16 carrier goes whole into `+0x2D8`
/// after the handler (`stop::timer_after_entry`), before the stop state's begin copies it.
unsafe fn stop_hit(enemy: usize, part: usize, lv: u8, o: PlRetFn) -> u64 {
    use crate::magic::stop;
    if super::damage::recom_spell_hits(crate::magic::spell::STOP) {
        return recom_stop_hit(enemy, part, lv, o);
    }
    let max_mp = super::mp::current().map_or(0, |v| v.1);
    let rec = super::stats::kh1_record(enemy);
    let r9 = rec.as_ref().map_or(1.0, |t| t.resist.get(9).copied().unwrap_or(100) as f32 / 100.0);
    let ticks = stop::ticks(max_mp, r9);
    let attr: u32 = rd(part + super::magic::PART_ATTR);
    wr(part + 0x92, ticks.map_or(0, stop::carrier));
    wr(part + super::magic::PART_ATTR, super::magic::NEUTRAL_ATTR);
    let gate = super::magic::stop_gate(enemy, ticks.is_some());
    let flags: u64 = rd(enemy + 0x260);
    let force = stop::interrupts(flags, ticks);
    if force {
        wr(enemy + 0x260, flags | 0x200_0000);
    }
    let r = o(enemy);
    if force {
        let now: u64 = rd(enemy + 0x260);
        wr(enemy + 0x260, now & !0x200_0000);
    }
    super::magic::stop_gate_back(gate);
    wr(part + super::magic::PART_ATTR, attr);
    let entered = super::magic::stop_entering(enemy);
    let whole = ticks.and_then(|t| stop::timer_after_entry(entered, t));
    if let Some(t) = whole {
        super::magic::stop_timer(enemy, t);
    }
    let off = entered && super::magic::stop_attacks_off(enemy);
    let state: usize = rd(enemy + super::magic::enemy_stop::NEXT_STATE);
    super::magic::hit_line(|| {
        format!(
            "hit stop{lv} on {:#x}: KH1 resist[9] {:.0} % -> {}{}{}",
            enemy & 0x00FF_FFFF,
            r9 * 100.0,
            ticks.map_or("immune".to_string(), |t| format!("{t} ticks ({:.0} frames)", t as f32 / 50.0)),
            if force { " (its attack broken)" } else { "" },
            if off {
                format!(", Re:CoM's stop state, its attacks off{}", if whole.is_some() { ", Re:CoM's timer given the whole duration" } else { "" })
            } else if ticks.is_some() {
                format!(", NOT stopped: Re:CoM's handler -> {r}, flags {flags:#x}, next state +{:#x}", state.wrapping_sub(enemy))
            } else {
                String::new()
            }
        )
    });
    if let Ok(mut g) = HIT_LOG.lock() {
        if g.len() < 16 {
            g.push(format!("e{:x}:stop{lv}:{}", enemy & 0x00FF_FFFF, ticks.map_or("immune".to_string(), |t| format!("{t}t"))));
        }
    }
    r
}

/// Damage = Re:CoM (owner, 2026-11-01, follow-ups), in a battle room: **Re:CoM's own Stop**. The
/// strike carries Re:CoM's Stop card's / Stopra's / Stopga's AP (9000 / 15000 / 21000 ticks in the
/// session's `SY0002.BIN`) and kind 8 from the cast (`super::damage::spell`); Re:CoM's handler
/// applies its own gate (param `+0x2C`: 0 = immune) and puts the AP in `+0x2D8`, and its stop
/// state's begin in its timer (`0x14017DC50` case 8, `0x14017C740`). Nothing of KH1's duration or
/// `resist[9]`. Kept from KH1 (its cast, not its numbers): the card interrupt on an attacking,
/// non-immune enemy (decision 14), its attack bodies off once stopped, the held hits' pace.
unsafe fn recom_stop_hit(enemy: usize, part: usize, lv: u8, o: PlRetFn) -> u64 {
    let carried: i16 = rd(part + 0x92);
    let gate = super::damage::recom_gate_open(enemy, crate::magic::spell::STOP);
    let flags: u64 = rd(enemy + 0x260);
    let force = crate::damage::interrupts(flags, gate);
    if force {
        wr(enemy + 0x260, flags | 0x200_0000);
    }
    let r = o(enemy);
    if force {
        let now: u64 = rd(enemy + 0x260);
        wr(enemy + 0x260, now & !0x200_0000);
    }
    let entered = super::magic::stop_entering(enemy);
    let off = entered && super::magic::stop_attacks_off(enemy);
    let duration: i32 = rd(enemy + super::magic::enemy_stop::DURATION);
    super::magic::hit_line(|| {
        format!(
            "hit stop{lv} on {:#x}: Re:CoM AP {carried} ({}){}{}{}",
            enemy & 0x00FF_FFFF,
            super::damage::spell_tag(crate::magic::spell::STOP, lv, carried as i32).trim_start_matches(':'),
            match gate {
                Some(true) => ", Re:CoM's gate open",
                Some(false) => ", Re:CoM's gate shut: immune",
                None => ", no param block",
            },
            if force { " (its attack broken)" } else { "" },
            if off { format!(", Re:CoM's stop state for {duration} ticks ({:.0} frames), its attacks off", duration as f32 / 50.0) } else { format!(", not stopped: Re:CoM's handler -> {r}") }
        )
    });
    if let Ok(mut g) = HIT_LOG.lock() {
        if g.len() < 16 {
            let what = if gate == Some(false) { "immune".to_string() } else { format!("{carried}t") };
            g.push(format!("e{:x}:stop{lv}:{what}{}", enemy & 0x00FF_FFFF, super::damage::spell_tag(crate::magic::spell::STOP, lv, carried as i32)));
        }
    }
    r
}

/// Re:CoM's enemies switch every hurt body off for the whole of an attack state (vtable `+0x438`,
/// default `0x14017DA80` = `0x1401A2D70(enemy, -1)`, run when attack states 2..10 start; the end,
/// `0x14017DA90`, clears the card bits and switches them back on). Checked (decomp + disasm;
/// `docs/recom-kh1-attack.md` § Hits on attacking enemies). KH1's enemies stay hittable while they
/// attack (the Shadow's script never masks its hurt parts, `actor+0x3A8`). Owner (2026-10-31,
/// decision 26): at the KH1 setting, in Sora's story, enemies stay hittable while attacking, for
/// every attacker, bosses included. Runs once per enemy attack start.
///
/// Except the attack a boss's revenge started (owner, 2026-11-02, "can't be damaged or interrupted
/// during their retaliation move"): Re:CoM's own call runs for it (`super::revenge::begin_untouchable`).
pub unsafe extern "win64" fn hurt_off(enemy: usize) {
    if mv::kh1_on() && !super::stats::riku_story() && !super::revenge::begin_untouchable(enemy) {
        return;
    }
    orig::<PlFn>(&ORIG_HURT_OFF)(enemy)
}

/// Re:CoM's player damage check: KH1's no-flinch (`0x140297F30`: damage, no hit reaction) uses
/// the check's own no-reaction exit (`+0x420 & 2`, read once into a local at its entry; its only
/// other readers are the room-card regen ticks, which don't run inside this call).
pub unsafe extern "win64" fn damage_check(pl: usize) -> u64 {
    let o: PlRetFn = orig(&ORIG_DAMAGE_CHECK);
    // A hit guarded by Sora's body guard (`body+0x9C & 0x20`, `player+0x72C`): kept for the next
    // KH1 frame (Re:CoM clears the bit each frame, `0x1401F4000` / `0x1401FABC0`).
    if GUARD_ON.load(Ordering::Relaxed) == pl && rd::<u32>(pl + BODY_GUARD) & 0x20 != 0 {
        GUARD_HIT.store(pl, Ordering::Relaxed);
        if let Ok(mut g) = GUARD_POINT.lock() {
            *g = rd(pl + BODY_HIT_POINT);
        }
    }
    let kh1 = mv::kh1_sora(pl);
    if kh1 {
        no_card_duel(pl);
        aero_halve(pl);
    }
    // M8b (owner, 2026-11-02: "sora should have iframes through all summon animations"): while his
    // Re:CoM summon sleight is out, Re:CoM's own protection bit `+0x418 & 1` for this check (its
    // exit zeroes the hit's value: no damage, no reaction; the roll's and Sonic Blade's path).
    // Restored after unless it was already set (Re:CoM's attached summons set it themselves).
    let shield = kh1 && super::summons::out(pl);
    let f418: u32 = if shield { rd(pl + p::FLAGS_418) } else { 0 };
    if shield {
        if rd::<u32>(pl + HIT_FLAGS) & HIT_ARRIVED != 0 && rd::<i16>(pl + HIT_VALUE) != 0 {
            super::summons::hit_skipped(rd(pl + HIT_VALUE));
        }
        wr(pl + p::FLAGS_418, f418 | 1);
    }
    let r = if NO_FLINCH.load(Ordering::Relaxed) != pl || !kh1 {
        o(pl)
    } else {
        let f: u32 = rd(pl + p::FLAGS_420);
        wr(pl + p::FLAGS_420, f | 2);
        let r = o(pl);
        if f & 2 == 0 {
            wr(pl + p::FLAGS_420, rd::<u32>(pl + p::FLAGS_420) & !2);
        }
        r
    };
    if shield && f418 & 1 == 0 {
        wr(pl + p::FLAGS_418, rd::<u32>(pl + p::FLAGS_418) & !1);
    }
    post_hit(pl, kh1);
    r
}

/// KH1's Aero barrier (M7 stage 7.10): while it is on, the pending hit's value on Sora's body
/// (`player+0x690`'s `+0x92`, which the check reads through `0x14016FDB0` before its resistance) is
/// halved (KH1: ÷ 2, at least 1). Re:CoM's check then runs as ever (resistance, reaction, death).
/// KH1 halves after its resistance: the same number while Sora's Re:CoM resistance is 100 %.
unsafe fn aero_halve(pl: usize) {
    if !super::magic::barrier_on() || rd::<i16>(pl + HIT_PENDING) == 0 {
        return;
    }
    let v: i16 = rd(pl + HIT_VALUE);
    let h = crate::magic::aero::halve(v);
    if h != v {
        wr(pl + HIT_VALUE, h);
        super::magic::hit_line(|| format!("aero: Sora's hit {v} -> {h} (KH1's barrier)"));
    }
}

/// Sora's pending hit value (his body `+0x690`, value `+0x92`). Checked (disasm `0x1401E70B0`:
/// `lea rcx,[rdi+0x690]; … call 0x14016FDB0`).
const HIT_VALUE: usize = 0x722;
/// A hit arrived (`+0x41C & 0x200`): the damage check goes past its early return only with it.
/// Checked (decomp `0x1401E6DE0`).
const HIT_FLAGS: usize = 0x41C;
const HIT_ARRIVED: u32 = 0x200;

/// Decision 33: at KH1 a hit with Re:CoM's card-duel reaction (`0x21`, Marluxia 3) is a plain
/// flinch (kind 1), so Sora never enters the duel (action `0x24`), which he can't win without
/// cards. The damage check reads the kind from `+0x710` once, after its hit-pending test
/// (`+0x720`), and the damage doesn't depend on it (only kinds 8 and 0x22 do). Marluxia 3
/// starts its duel mode only while Sora's action is `0x24` (`0x1403F3590`, `+0xCE4 |= 4`; else its state 3 plays motion 0x34), so it
/// takes its own no-duel branch. Checked (decomp).
unsafe fn no_card_duel(pl: usize) {
    if rd::<i16>(pl + HIT_PENDING) == 0 || rd::<u32>(pl + HIT_KIND) != atk::RECOM_DUEL_REACTION {
        return;
    }
    wr(pl + HIT_KIND, atk::RECOM_FLINCH_REACTION);
    if DUEL_LINES.fetch_add(1, Ordering::Relaxed) < 20 {
        log("attack: a card-duel hit (reaction 0x21) taken as a plain flinch at KH1 (decision 33)");
    }
}

/// KH1's post-hit invulnerability (decision 31) on Re:CoM's own hit-immunity timer `+0x7BC`.
/// The damage check is the only writer of the hit reaction `0xC` and runs every frame of it
/// (Battle / Field exes); while the reaction plays we hold the timer at 30 frames + dt, so Re:CoM
/// counts KH1's 30 frames down itself from the reaction's end. Only for reactions that start at
/// the KH1 setting; the death reaction (`player+0x7B0`) is left alone.
unsafe fn post_hit(pl: usize, kh1: bool) {
    let reacting = rd::<u32>(pl + p::ACTION) == HIT_REACTION;
    let armed = HIT_ARMED.load(Ordering::Relaxed) == pl;
    if !reacting {
        if armed {
            HIT_ARMED.store(0, Ordering::Relaxed);
        }
        return;
    }
    if !armed {
        // A reaction that began in this call: Re:CoM zeroes the time in action with it.
        if !kh1 || rd::<u32>(pl + p::ACTION_TIME) != 0 || rd::<i32>(pl + DEAD) != 0 {
            return;
        }
        HIT_ARMED.store(pl, Ordering::Relaxed);
    }
    let t: i32 = rd(pl + HIT_IMMUNE);
    wr(pl + HIT_IMMUNE, atk::post_hit_hold(t, rd(pl + p::DT)));
}

/// Re:CoM's root step (`0x14024DCA0(ctrl, out travel, out yaw)`): for Sora's KH1 actions (the roll
/// and the swings), the last update's travel (decision 17).
pub unsafe extern "win64" fn root_step(ctrl: usize, out: *mut [f32; 4], yaw: *mut f32) {
    let o: RootFn = orig(&ORIG_ROOT_STEP);
    o(ctrl, out, yaw);
    if !mv::kh1_on() || out.is_null() || !TICK_LIVE.load(Ordering::Relaxed) {
        return;
    }
    let pl: usize = rd(va(base(), data::MAIN_PLAYER));
    if !mv::is_main_sora(pl) || rd::<usize>(pl + p::CTRL) != ctrl {
        return;
    }
    let a: u32 = rd(pl + p::ACTION);
    if mv::recom_roll_owns(pl) {
        // `dodge_roll = recom`: Re:CoM's own roll travels in its own update (no delay). A KH1
        // swing's last travel still pending (KH1 applies it in the next frame's physics) goes on
        // top once, on the roll's first update; then nothing of ours.
        super::magic::release_root_off();
        if let Ok(mut d) = ROOT.lock() {
            let late = d.swap(ctrl, TICK.load(Ordering::Relaxed), Vec3::ZERO);
            let late_m = late.length();
            if late_m > 0.0 && late_m <= crate::movement::ROOT_DELAY_MAX_GAP {
                let v = &mut *out;
                v[0] += late.x;
                v[1] += late.y;
                v[2] += late.z;
            }
        }
        return;
    }
    let cast = a == crate::magic::RECOM_MAGIC_ACTION && super::magic::kh1_cast(pl);
    // M9 stage 9c.1: a KH1 special's rows travel by their root motion, one update late as the
    // swings (KH1 applies it in the next frame's physics).
    let special = a == crate::specials::RECOM_ACTION && super::specials::kh1_special(pl);
    if a != crate::movement::action::AVOID && a != crate::movement::action::ATTACK && !cast && !special {
        super::magic::release_root_off();
        return;
    }
    // `out` is an absolute position (the model's position `ctrl+0xF0` + this update's travel,
    // `0x14024C960`), not a travel: delay the travel, keep the anchor.
    let v = &mut *out;
    let mut root = Vec3::new(v[0], v[1], v[2]);
    let anchor = rd_vec(ctrl + 0xF0);
    if cast {
        // Decision 38: the ground Fire release travels KH1's curve, not Re:CoM's clip's.
        root = super::magic::release_root(pl, ctrl, anchor, root);
    } else {
        super::magic::release_root_off();
    }
    if let Ok(mut d) = ROOT.lock() {
        let t = d.position(ctrl, TICK.load(Ordering::Relaxed), anchor, root, crate::movement::ROOT_DELAY_MAX_GAP);
        debug(|| format!("root a={a} id={id} anchor=({:.3},{:.3},{:.3}) root=({:.3},{:.3},{:.3}) out=({:.3},{:.3},{:.3})", anchor.x, anchor.y, anchor.z, root.x, root.y, root.z, t.x, t.y, t.z, id = rd::<u32>(pl + field::ATTACK_ID)));
        v[0] = t.x;
        v[1] = t.y;
        v[2] = t.z;
    }
}

// ---- install ----

/// Builds the KH1 rows from the live Re:CoM rows 1–7 (which must be exactly the Steam exe's).
pub unsafe fn build_table(b: usize) -> bool {
    let rows = va(b, atk::RECOM_ATTACK_ROWS) + 16;
    let mut live = [[0u8; 16]; 7];
    for (i, r) in live.iter_mut().enumerate() {
        *r = rd(rows + 16 * i);
    }
    if live != atk::RECOM_ROWS_1_7 {
        log("attack: Re:CoM's attack rows 1-7 are not the Steam exe's; KH1 Attack not installed");
        return false;
    }
    let ext = mv::EXT.get().copied().unwrap_or_default();
    let mut rows = atk::kh1_table(&live, moveset(), &|req| ext.get(req));
    // M9 stage 9c.1: Sonic Blade's rows on the xl clips' action rows.
    atk::add_special_rows(&mut rows, &live, moveset(), &|e| ext.xl_row(e));
    log(&format!("attack: Sonic Blade rows 0x1B..=0x21 {}", if atk::special_rows_ready(&rows) { "ready" } else { "missing (no xl_limit clips in the motion pack: re-run kh1-recom-build)" }));
    let t: &'static [[u8; 16]] = Box::leak(rows.into_boxed_slice());
    // M9 9a: the pickers only get the modifiers whose rows Re:CoM can play.
    let ready = atk::modifier_rows_ready(t);
    super::abilities::set_rows_ready(ready);
    log(&format!(
        "attack: modifier rows ready: {:?}; missing (no clip in the motion pack): {:?}",
        crate::abilities::names(ready),
        crate::abilities::names(crate::abilities::MODIFIER_BITS & !ready)
    ));
    TABLE.set(t).is_ok()
}

/// Sora's vtable slot `+0x128` → our row pick (checked old value), and the Battle exe's case for
/// action `0xA` (jump-table entry → the cave stub). Data patches, nothing per frame.
pub unsafe fn patch_data(b: usize) -> bool {
    use super::hook::patch;
    let slot = va(b, SORA_PICKER_SLOT);
    let old = (va(b, RECOM_PICKER) as u64).to_le_bytes();
    let new = (picker as *const () as usize as u64).to_le_bytes();
    let cave = va(b, 0x1_4000_0000 + atk::battle_case::CAVE_RVA as u64);
    let entry = va(b, 0x1_4000_0000 + atk::battle_case::TABLE_ENTRY_RVA as u64);
    let old_entry = atk::battle_case::DEFAULT_RVA.to_le_bytes();
    let stub = atk::battle_case::stub();
    // All three or none: check every old value first.
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    if have(cave, 13) != [0xCC; 13] || have(entry, 4) != old_entry || have(slot, 8) != old {
        log("attack: the Battle case, its cave or Sora's row-pick slot is not as expected; KH1 Attack not installed");
        return false;
    }
    if !patch(cave, &[0xCC; 13], &stub, "attack: Battle case stub") {
        return false;
    }
    if !patch(entry, &old_entry, &atk::battle_case::entry(), "attack: Battle jump table 0xA") {
        return false;
    }
    patch(slot, &old, &new, "attack: Sora's row pick slot")
}
