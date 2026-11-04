//! Sleights outside battle (owner, 2026-11-04, option (a): "Plain field strike"; `crate::field_sleights`,
//! `docs/recom-field-sleights.md` § Built). Re:CoM's own sleight code runs in a field room:
//!
//! - **The Field exe's case 0x16** ([`install`]): its jump table's entry for action 0x16 goes to a
//!   13-byte stub (`mov rcx,rbx; call exeStockSkill; jmp default`), as the magic and item cases. The
//!   field's Sora then runs `exeStockSkill` (our specials wrapper passes Re:CoM's sleights through)
//!   for any sleight started there; nothing sets 0x16 in the field at Combat = Re:CoM.
//! - **The list pointers** ([`prepare`], once per field start): Re:CoM's own lookups fill
//!   `BattleCardList` / `StockSkillList`.
//! - **Packs** ([`plm_init`], once per room build): the field PLM gets the battle branch's writes
//!   where the field allocated the ring; elsewhere the pack sleights stay greyed ([`gate`]).
//! - **The menu's room gate** ([`gate`], the menu's build: two loads in a field room).
//! - **The battle start** ([`battle_start`], the battle manager ctor's wrapper, once per battle): a
//!   field sleight its strike cut is logged and our "running" marks are dropped. Re:CoM's own player
//!   dtor fades the room back (`sites::FIELD_SLEIGHT_CHECKED`, "player dtor dim exit"); the battle's
//!   Sora is a new player (none of the field one's protection bits).
//!
//! Nothing of ours runs per frame outside the menu's build and the sleight's own frames.

use super::movement::{self as mv, func, orig, p, rd, wr};
use super::{base, log, va};
use crate::field_sleights::{self as fs, field_case as fc, lists, packs};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The PLM init wrapper is in (field packs possible).
static PACKS_HOOKED: AtomicBool = AtomicBool::new(false);
pub static ORIG_PLM_INIT: AtomicUsize = AtomicUsize::new(0);
/// The player whose field sleight (ours) is running, and its id + 1 (for the cut's log line).
static FIELD_RUN: AtomicUsize = AtomicUsize::new(0);
static FIELD_ID: AtomicU32 = AtomicU32::new(0);
static LOG_LEFT: AtomicU32 = AtomicU32::new(60);

type LookupFn = unsafe extern "win64" fn(u32, *const u8, i32) -> usize;
type DataFn = unsafe extern "win64" fn(usize) -> usize;
type WorldFn = unsafe extern "win64" fn(i32) -> u32;
type RoomFn = unsafe extern "win64" fn() -> u32;
type PlmFn = unsafe extern "win64" fn(usize);

/// The sleight resources object (`initStockSkill`'s `0x140211790(+0x3B8, …)`): the PLM.
const PLM: usize = 0x3B8;

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("field sleights: {}", line()));
    }
}

/// Patches the Field exe's case for action 0x16 and hooks the PLM init (all its sites checked
/// first). False: nothing patched, no field sleights (logged).
pub unsafe fn install(b: usize) -> bool {
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    for c in crate::sites::FIELD_SLEIGHT_CHECKED.iter().chain(crate::sites::FIELD_SLEIGHT_DATA) {
        if have(b + c.rva as usize, c.expect.len()) != c.expect {
            log(&format!("field sleights: {} is not as expected; sleights stay battle-only", c.name));
            return false;
        }
    }
    for c in crate::sites::all_sites().filter(|c| crate::sites::FIELD_SLEIGHT_ALSO.contains(&c.rva)) {
        if have(b + c.rva as usize, c.expect.len()) != c.expect {
            log(&format!("field sleights: {} is not as expected; sleights stay battle-only", c.name));
            return false;
        }
    }
    let cave = b + fc::CAVE_RVA as usize;
    let entry = b + fc::TABLE_ENTRY_RVA as usize;
    let old_entry = fc::DEFAULT_RVA.to_le_bytes();
    if !super::hook::patch(cave, &[0xCC; fc::STUB_LEN], &fc::stub(), "field sleights: Field case stub") {
        return false;
    }
    if !super::hook::patch(entry, &old_entry, &fc::entry(), "field sleights: Field jump table 0x16") {
        super::hook::patch(cave, &fc::stub(), &[0xCC; fc::STUB_LEN], "field sleights: Field case stub (undo)");
        return false;
    }
    INSTALLED.store(true, Ordering::Release);
    let hooks = [super::hook::Hook { site: &crate::sites::FIELD_SLEIGHT_HOOKS[0], ours: plm_init as *const () as usize, original: &ORIG_PLM_INIT }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    let packs = super::hook::install_all(b, &hooks);
    PACKS_HOOKED.store(packs, Ordering::Release);
    if !packs {
        log("field sleights: the PLM init is not hooked; in the field only the sleights without a pack (Fire / Blizzard / Thunder / Reflect Raid, Judgment, Homing Blizzara, Warpinator, Warp, Teleport, Strike Raid)");
    }
    true
}

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

pub fn start_line() -> String {
    format!(
        "field sleights: at combat = kh1 in Sora's story Re:CoM's sleights run in field rooms (owner, 2026-11-04, option (a)): Field exe case 0x16 {:#x} -> stub {:#x} (exeStockSkill, then {:#x}); BattleCardList / StockSkillList by Re:CoM's lookup 0x1402A43E0 at each field start; sleight packs in field rooms {} (PLM init 0x140210100: the battle branch's +0x88 |= 1 and ring fields where the field allocated the ring: not world 10, not rooms 2000-2999); free in the field (no MP); a hit on a field Heartless is Re:CoM's (its kind: strike and battle, paralyse, freeze, stun)",
        0x1_4000_0000u64 + fc::TABLE_ENTRY_RVA as u64,
        0x1_4000_0000u64 + fc::CAVE_RVA as u64,
        0x1_4000_0000u64 + fc::DEFAULT_RVA as u64,
        if PACKS_HOOKED.load(Ordering::Acquire) { "on" } else { "OFF (not hooked)" }
    )
}

/// `pl` is the KH1 Sora in a field room where Re:CoM's sleights run (the case is in, Combat = KH1,
/// the main player a `CPlayerSora`: Riku's story's is `CPlayerRiku`, `mv::kh1_sora`). Three loads.
pub unsafe fn field_on(pl: usize) -> bool {
    installed() && pl != 0 && rd::<i32>(pl + p::BATTLE) == 0 && mv::kh1_sora(pl)
}

/// This room's PLM loads sleight packs (`+0x88 & 1`: battle and minigame rooms by Re:CoM, field
/// rooms by [`plm_init`]).
pub unsafe fn packs_on(pl: usize) -> bool {
    let plm: usize = rd(pl + PLM);
    plm != 0 && rd::<u32>(plm + packs::FLAGS) & packs::LOADING != 0
}

/// The menu's room gate (`crate::summons::Gate::field` / `packs`), each build: (a field room where
/// sleights run, its packs load). Battle rooms: (false, true).
pub unsafe fn gate(pl: usize) -> (bool, bool) {
    if pl == 0 {
        return (false, false);
    }
    // A field sleight of ours that ended in the field (Re:CoM's end action: Sora's action no longer
    // 0x16) is no cut. The battle start's Wait (player state 2) keeps the action.
    let run = FIELD_RUN.load(Ordering::Relaxed);
    if run != 0 && (run != pl || rd::<u32>(pl + p::ACTION) != fc::ACTION) {
        let _ = FIELD_RUN.compare_exchange(run, 0, Ordering::Relaxed, Ordering::Relaxed);
    }
    if rd::<i32>(pl + p::BATTLE) != 0 {
        return (false, true);
    }
    let field = field_on(pl);
    (field, field && packs_on(pl))
}

/// A sleight's start in a field room ([`field_on`]): Re:CoM's own lookups fill the two list pointers
/// its sleight code reads (`crate::field_sleights::lists`), and the start is noted for the battle's
/// cut line. False (logged): a list isn't loaded; nothing changed but the pointers found.
pub unsafe fn prepare(pl: usize, id: u16, name: &str) -> bool {
    let b = base();
    let lookup = func::<LookupFn>(lists::LOOKUP_RVA);
    let data = func::<DataFn>(lists::DATA_RVA);
    let find = |n: &[u8]| {
        let node = lookup(lists::GROUP, n.as_ptr(), 0);
        if node == 0 {
            0
        } else {
            data(node)
        }
    };
    let cards = find(lists::BATTLE_CARD_NAME);
    let skills = find(lists::STOCK_SKILL_NAME);
    if cards == 0 || skills == 0 {
        note(|| format!("{name} ({id:#x}): Re:CoM's {} not loaded in this room; not started", if skills == 0 { "StockSkillList" } else { "BattleCardList" }));
        return false;
    }
    let at_cards = va(b, crate::damage::BATTLE_CARD_LIST);
    let at_skills = va(b, crate::damage::STOCK_SKILL_LIST);
    let (old_cards, old_skills): (usize, usize) = (rd(at_cards), rd(at_skills));
    wr(at_cards, cards);
    wr(at_skills, skills);
    if old_cards != cards || old_skills != skills {
        note(|| format!("{name} ({id:#x}): BattleCardList {old_cards:#x} -> {cards:#x}, StockSkillList {old_skills:#x} -> {skills:#x} (Re:CoM's lookup)"));
    }
    FIELD_ID.store(id as u32 + 1, Ordering::Relaxed);
    FIELD_RUN.store(pl, Ordering::Relaxed);
    true
}

/// The battle manager ctor (once per battle, before its Sora's first frame): a field sleight still
/// marked running was cut by this battle's first strike or contact (Re:CoM's encounter sends the
/// field Sora to its Wait state; the field is torn down). Its player is gone: Re:CoM's player dtor
/// faded the room back if its dim was on; the battle's Sora has none of its bits. Our marks of it
/// (the specials' running player, the summons' / Sleights list's) are dropped.
pub unsafe fn battle_start() {
    let o = FIELD_RUN.swap(0, Ordering::Relaxed);
    let id = FIELD_ID.swap(0, Ordering::Relaxed);
    if o != 0 && id != 0 {
        let id = (id - 1) as u16;
        let colour: usize = rd(va(base(), DIM_SLOTS));
        let mask: u32 = if colour != 0 { rd(colour + 0x30) } else { 0 };
        note(|| format!("{} ({id:#x}): cut by the battle start (Re:CoM's field strike or contact); the field player is gone (Re:CoM's dtor fades a dim back: room colour slot 0 mask now {mask:#x})", crate::sleights::name(id)));
    }
    super::specials::forget_running();
    super::summons::forget_running();
}

/// Re:CoM's colour slots (`0x140232170` writes slot n at `[0x14087B7F0] + n × 0x40`; `+0x30` its
/// mask, cleared once it is back to white). For the cut's log line only.
const DIM_SLOTS: u64 = 0x1_4087_B7F0;

/// Re:CoM's PLM init, then, for a field room at Combat = KH1 where the field allocated the ring
/// (`crate::field_sleights::packs::wanted`), the battle branch's own writes. Once per PLM (one per
/// room build).
pub unsafe extern "win64" fn plm_init(plm: usize) {
    orig::<PlmFn>(&ORIG_PLM_INIT)(plm);
    if plm == 0 {
        return;
    }
    let b = base();
    let ring: usize = rd(va(b, packs::RING));
    let room = packs::Room {
        kh1: mv::kh1_on() && !super::stats::riku_story(),
        loading_off: rd::<u32>(plm + packs::FLAGS) & packs::LOADING == 0,
        world: func::<WorldFn>(packs::WORLD_RVA)(1),
        room_2000: func::<RoomFn>(packs::ROOM_2000_RVA)() & 0xFF != 0,
        ring: (ring as i64) > 0,
    };
    if !packs::wanted(&room) {
        if room.kh1 && room.loading_off {
            note(|| format!("room build: no sleight packs here (world {}, room 2000-2999 {}, ring {ring:#x}): the pack sleights stay greyed", room.world, room.room_2000));
        }
        return;
    }
    let size: u32 = rd(va(b, packs::RING_SIZE));
    let (cursor, end, space) = packs::fields(ring as u64, size);
    wr(plm + packs::FLAGS, rd::<u32>(plm + packs::FLAGS) | packs::LOADING);
    wr(plm + packs::CURSOR, cursor);
    wr(plm + packs::END, end);
    wr(plm + packs::SPACE, space);
    for i in 0..packs::SLOT_COUNT {
        wr(plm + packs::SLOTS + i * packs::SLOT_LEN + packs::SLOT_PTR, end);
    }
    note(|| format!("room build (world {}): sleight packs on in this field room (ring {ring:#x}, {size:#x} bytes)", room.world));
}

/// A field sleight's start for the shared paths (`win::summons`, `win::specials`): field room or
/// not, and the lists filled when it is one. Some(in the field) to go on, None to refuse.
pub unsafe fn start_room(pl: usize, id: u16, name: &str) -> Option<bool> {
    if rd::<i32>(pl + p::BATTLE) != 0 {
        return Some(false);
    }
    if !field_on(pl) {
        note(|| format!("{name} ({id:#x}): not in battle and no field sleights here"));
        return None;
    }
    if !fs::in_field(id) && crate::summons::ALL.iter().all(|s| s.sleight != id) {
        note(|| format!("{name} ({id:#x}): not a field sleight"));
        return None;
    }
    if fs::needs_pack(id) && !packs_on(pl) {
        note(|| format!("{name} ({id:#x}): its pack doesn't load in this room"));
        return None;
    }
    prepare(pl, id, name).then_some(true)
}
