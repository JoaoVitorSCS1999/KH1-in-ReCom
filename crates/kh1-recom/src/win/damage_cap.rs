//! Sora's damage cap at Combat = KH1, Sora's story (owner, 2026-11-05; rules and bytes in
//! `crate::damage_cap`, `docs/recom-kh1-attack.md` § Damage cap).
//!
//! - One patch: Re:CoM's damage apply's join `0x14017DF6A` jumps to a stub that looks the enemy
//!   (`rbx`) up in [`BLOCK`]'s slots and, when its value `xmm6` is over the slot's cap, takes the cap;
//!   then Re:CoM's own instructions and call. Runs once per damaging hit; nothing per frame.
//! - The enemy-hit wrapper (`win::attack::kh1_enemy_hit`) sets the slot per hit: [`arm`] for a hit of
//!   Sora's side (its cap in the join's units), [`forget`] when another hit arrives on that enemy. The
//!   slot stays until then, so Re:CoM's own later applies of the same hit (the flatten exe's `+0x2DC`,
//!   the landing of a kind 9 / 0x12 hit on an airborne enemy, `0x140182C50`) take the same cap.
//! - Cleared at every battle start (Re:CoM's battle manager ctor) and when Combat is Re:CoM.

use super::movement::{rd, wr};
use super::{log, va};
use crate::damage_cap::{self as dc, Rule, SLOTS};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

#[repr(C)]
struct Slot {
    enemy: AtomicU64,
    /// f32 bits.
    cap: AtomicU32,
    _pad: AtomicU32,
}

/// The block the stub reads (layout: `crate::damage_cap::SLOTS_AT`). The stub writes `count`, `last`,
/// `before`, `after` on a capped hit (the game's thread, as the wrapper).
#[repr(C)]
struct Block {
    count: AtomicU64,
    last: AtomicU64,
    before: AtomicU32,
    after: AtomicU32,
    _pad: [AtomicU32; 2],
    slots: [Slot; SLOTS],
}

#[allow(clippy::declare_interior_mutable_const)]
const FREE: Slot = Slot { enemy: AtomicU64::new(0), cap: AtomicU32::new(0), _pad: AtomicU32::new(0) };
static BLOCK: Block = Block { count: AtomicU64::new(0), last: AtomicU64::new(0), before: AtomicU32::new(0), after: AtomicU32::new(0), _pad: [AtomicU32::new(0), AtomicU32::new(0)], slots: [FREE; SLOTS] };

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// Slots in use (0: nothing to look up or clear).
static USED: AtomicU32 = AtomicU32::new(0);
/// The next slot to reuse when all are taken.
static NEXT: AtomicU32 = AtomicU32::new(0);
/// Enemies whose capped hit was logged this battle.
static LOGGED: Mutex<Vec<usize>> = Mutex::new(Vec::new());

/// The enemy's hurt-body block (`+0xF8`): `+8` its hit bits (4 = a damaging hit gathered), `+0x34`
/// the gathered value. Checked (decomp `0x140154EB0`).
const INFO: usize = 0xF8;

/// Puts the stub in: the join's and its context's bytes checked (`sites::DAMAGE_CAP_PATCH` /
/// `DAMAGE_CAP_CHECKED`), the stub placed near the join for the running image's addresses (0.139.1's
/// lesson: every exe address rebased), then the join replaced by a jump to it. Without it Sora's hits
/// are not capped (logged).
pub unsafe fn install(bs: usize) {
    use crate::sites::{DAMAGE_CAP_CHECKED, DAMAGE_CAP_PATCH};
    for c in DAMAGE_CAP_CHECKED.iter().chain(DAMAGE_CAP_PATCH) {
        if std::slice::from_raw_parts((bs + c.rva as usize) as *const u8, c.expect.len()) != c.expect {
            log(&format!("damage cap: {} is not as expected: Sora's hits are not capped", c.name));
            return;
        }
    }
    let s = &DAMAGE_CAP_PATCH[0];
    let at = bs + s.rva as usize;
    let back = (at + s.expect.len()) as u64;
    let apply = va(bs, dc::APPLY) as u64;
    let block = &BLOCK as *const Block as u64;
    let filler = vec![0xCCu8; dc::STUB_LEN];
    let Some(stub) = super::hook::place_near(at, &filler) else {
        log("damage cap: no memory near Re:CoM's damage apply: Sora's hits are not capped");
        return;
    };
    let Some(code) = dc::stub(block, stub as u64, apply, back) else {
        log("damage cap: Re:CoM's damage apply is out of the stub's reach: Sora's hits are not capped");
        return;
    };
    let Some(entry) = crate::hookplan::entry_patch(at as u64, s.expect.len(), stub as u64) else {
        log("damage cap: the stub is out of the join's reach: Sora's hits are not capped");
        return;
    };
    if !super::hook::patch(stub, &filler, &code, "damage cap stub") || !super::hook::patch(at, s.expect, &entry, s.name) {
        return;
    }
    INSTALLED.store(true, Ordering::Release);
    log(&format!("damage cap: {} through the stub at {stub:#x}: each hit of Sora's side on an enemy is capped at its KH1 per-hit cap (KH1's 0x1402BFA80 order: after every bonus, before the resistance), Combat = KH1 in Sora's story", s.name));
}

fn clear() {
    for s in &BLOCK.slots {
        s.enemy.store(0, Ordering::Relaxed);
    }
    USED.store(0, Ordering::Relaxed);
}

/// Re:CoM's battle manager ctor (every battle start): no slot carries over (the last battle's enemies
/// are freed), and each enemy's first capped hit is logged again.
pub fn battle_start() {
    clear();
    if let Ok(mut g) = LOGGED.lock() {
        g.clear();
    }
}

/// Combat = Re:CoM (the hit wrapper passes through): any slot left is cleared. One atomic load.
pub fn off() {
    if USED.load(Ordering::Relaxed) != 0 {
        clear();
    }
}

fn find(enemy: usize) -> Option<&'static Slot> {
    BLOCK.slots.iter().find(|s| s.enemy.load(Ordering::Relaxed) == enemy as u64)
}

fn set(enemy: usize, cap: f32) {
    let slot = find(enemy).or_else(|| BLOCK.slots.iter().find(|s| s.enemy.load(Ordering::Relaxed) == 0)).unwrap_or_else(|| &BLOCK.slots[NEXT.fetch_add(1, Ordering::Relaxed) as usize % SLOTS]);
    // The cap first: the stub runs on this thread, but a slot is never seen with another enemy's cap.
    slot.cap.store(cap.to_bits(), Ordering::Relaxed);
    slot.enemy.store(enemy as u64, Ordering::Relaxed);
    let n = BLOCK.slots.iter().filter(|s| s.enemy.load(Ordering::Relaxed) != 0).count();
    USED.store(n as u32, Ordering::Relaxed);
}

/// A hit not of Sora's side (or one never capped) arrives on `enemy`: its slot, if any, goes.
/// `waiting`: whether a hit is really waiting (asked only when the enemy has a slot).
pub fn forget(enemy: usize, waiting: impl FnOnce() -> bool) {
    if USED.load(Ordering::Relaxed) == 0 {
        return;
    }
    let Some(slot) = find(enemy) else { return };
    if waiting() {
        slot.enemy.store(0, Ordering::Relaxed);
        USED.fetch_sub(1, Ordering::Relaxed);
    }
}

/// A hit arrived on `enemy` that is never capped (Stop, Gravity, the % of HP rows, no KH1 number): its
/// slot, if any, goes.
pub fn uncapped(enemy: usize) {
    forget(enemy, || true);
}

/// What the hit is, for the log.
#[derive(Clone, Copy, Debug)]
pub enum What {
    Swing(usize),
    Spell(u8, u8),
    RagnarokShot,
    Sleight(&'static str),
}

impl What {
    fn text(self) -> String {
        match self {
            What::Swing(r) => format!("swing row {r:#x}"),
            What::Spell(s, lv) => format!("{}{lv}", crate::magic::spell::NAMES.get(s as usize).copied().unwrap_or("spell")),
            What::RagnarokShot => "Ragnarok's shot".to_string(),
            What::Sleight(n) => format!("sleight {n}"),
        }
    }
}

/// How the hit's number was made.
#[derive(Clone, Copy, Debug)]
pub enum Source {
    /// Damage = KH1, KH1's number made without its cap: the cap and the resistance it carries.
    Kh1(super::stats::CapLater),
    /// Damage = KH1, Re:CoM's number on KH1's HP: the raw cap.
    Raw,
    /// Damage = Re:CoM: the share of Re:CoM's max HP.
    Recom,
}

/// A hit armed for the join, for [`after`].
pub struct Armed {
    enemy: usize,
    count: u64,
    cap: f32,
    record_cap: u16,
    kh1_hp: i32,
    from: &'static str,
    what: What,
    rule: Rule,
}

/// Before Re:CoM's handler takes a hit of Sora's side on `enemy`: its cap in the join's units, from
/// its KH1 record (twin / proxy / boss twin), into its slot; no record, cap 0 or a kind never capped
/// (Re:CoM's kind `kind`: 8, 0xD): its slot goes. Per hit only.
pub fn arm(enemy: usize, kind: u32, what: What, source: Source) -> Option<Armed> {
    if !INSTALLED.load(Ordering::Acquire) {
        return None;
    }
    let planned = (!dc::never_capped_kind(kind)).then(|| plan(enemy, source)).flatten();
    let Some((cap, record_cap, kh1_hp, from, rule)) = planned else {
        uncapped(enemy);
        return None;
    };
    set(enemy, cap);
    Some(Armed { enemy, count: BLOCK.count.load(Ordering::Relaxed), cap, record_cap, kh1_hp, from, what, rule })
}

fn plan(enemy: usize, source: Source) -> Option<(f32, u16, i32, &'static str, Rule)> {
    let (t, from) = super::stats::record_from(enemy)?;
    let (cap, rule) = match source {
        Source::Kh1(later) => {
            let (c, resist) = later?;
            (c, Rule::Kh1 { resist })
        }
        Source::Raw => (dc::raw(t.max_damage)?, Rule::Raw),
        Source::Recom => {
            let recom_hp = super::stats::recom_max_hp(enemy)?;
            (dc::recom_share(t.max_damage, recom_hp, t.hp)?, Rule::Share { recom_hp })
        }
    };
    Some((cap, t.max_damage, t.hp, from, rule))
}

/// After Re:CoM's handler: when the join capped this hit, its numbers (logged once per enemy per
/// battle), for the trace. A stopped enemy's held hit is recorded by Re:CoM's stop exe from the
/// gathered value, not from the join: that value is capped here too.
pub unsafe fn after(armed: Option<Armed>) -> Option<(f32, f32)> {
    let a = armed?;
    if super::magic::stopped(a.enemy) {
        let info: usize = rd(a.enemy + INFO);
        if info != 0 && rd::<u32>(info + 8) & 4 != 0 {
            let v: i32 = rd(info + 0x34);
            let c = dc::held(v, a.cap);
            if c != v {
                wr(info + 0x34, c);
                note(&a, v as f32, c as f32, " (held by Stop)");
                return Some((v as f32, c as f32));
            }
        }
    }
    if BLOCK.count.load(Ordering::Relaxed) == a.count || BLOCK.last.load(Ordering::Relaxed) != a.enemy as u64 {
        return None;
    }
    let (before, after) = (f32::from_bits(BLOCK.before.load(Ordering::Relaxed)), f32::from_bits(BLOCK.after.load(Ordering::Relaxed)));
    note(&a, before, after, "");
    Some((before, after))
}

fn note(a: &Armed, before: f32, after: f32, tail: &str) {
    let Ok(mut g) = LOGGED.lock() else { return };
    if g.len() >= 64 || g.contains(&a.enemy) {
        return;
    }
    g.push(a.enemy);
    drop(g);
    let name = super::stats::class_label(a.enemy);
    log(&format!("{}{tail}", dc::line(&name, a.from, &a.what.text(), before, after, a.record_cap, a.kh1_hp, a.rule)));
}

/// The trace's hits-column tag for a capped hit.
pub fn tag(c: Option<(f32, f32)>) -> String {
    c.map_or(String::new(), |(b, a)| format!(":cap {b:.0}>{a:.0}"))
}
