//! 0.111.0: the command menu's Party list, Re:CoM's side (`crate::party`,
//! `docs/recom-command-pages-party.md`).
//!
//! - **Taught** ([`learned`]): Re:CoM's own learned test `0x14023AB00(id)` for each Party sleight
//!   (the abilities' checked callee): the Customize picker (camp) and the menu.
//! - **Available in this battle** ([`available`]): taught, and every Friend card it needs can drop
//!   here as Re:CoM's drop roll `0x1401602A0` decides it (`crate::party::can_drop`) from Re:CoM's own
//!   getters: the world `0x1402339C0(0)`, the room type `0x140233860`, Riku's story `0x1401AAAE0`,
//!   the minigame room `0x140235360`, the friends met (int var (0, 0x22), `0x1401B08C0`). Read once
//!   per battle (owner: "Whole battle, if possible") at the menu's first build in a battle room;
//!   none of these change inside one. Outside battle: none (friend sleights stay battle-only:
//!   `exeStockSkill`'s friend branch reads the card battle's active card without a null test, and
//!   Re:CoM's sleight packs only finish loading in battle rooms; `docs/recom-field-sleights.md`).
//! - **The start**: Re:CoM's own sleight start (`super::summons::start_party`, the Sleights list's
//!   path), which plays the friend: its pack (`initStockSkill`'s `0x140211790`), the actor
//!   (`0x14020E950`), its attack, its exit.
//!
//! Nothing per frame but one atomic load (and, while a battle's set isn't read yet, the reads).

use super::log;
use super::movement::{func, p, rd};
use crate::party::{self as pt, Place, Set};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// This battle's available set (bit 31: read), or 0 (not read / not in battle).
static AVAIL: AtomicU32 = AtomicU32::new(0);
const READ: u32 = 1 << 31;
/// The battle player [`AVAIL`] was read for (each battle room builds its own player).
static AVAIL_PLAYER: AtomicUsize = AtomicUsize::new(0);

const WORLD: u32 = 0x0023_39C0;
const ROOM_TYPE: u32 = 0x0023_3860;
const RIKU: u32 = 0x001A_AAE0;
const MINIGAME: u32 = 0x0023_5360;
const VAR_GET: u32 = 0x001B_08C0;
const LEARNED: u32 = 0x0023_AB00;
/// The friends-met bits (int var set 0, id 0x22; `0x1401602A0`). Checked (decomp).
const MET_VAR: (u32, u32) = (0, 0x22);

type IntFn = unsafe extern "win64" fn(i32) -> i32;
type VoidFn = unsafe extern "win64" fn() -> u32;
type VarGetFn = unsafe extern "win64" fn(u32, u32) -> u64;
type LearnedFn = unsafe extern "win64" fn(u16) -> u64;

/// Checks the Re:CoM code and data the list reads (`sites::PARTY_CHECKED` and the earlier stages'
/// callees). No hook, no patch.
pub unsafe fn install(b: usize) -> bool {
    let have = |at: usize, n: usize| std::slice::from_raw_parts(at as *const u8, n).to_vec();
    let wanted = [WORLD, ROOM_TYPE, RIKU, MINIGAME, VAR_GET, LEARNED, 0x007C_1F10];
    for rva in wanted {
        let Some(site) = crate::sites::all_sites().find(|s| s.rva == rva) else {
            log(&format!("party: no checked site for {:#x}; no Party list", 0x1_4000_0000u64 + rva as u64));
            return false;
        };
        if have(b + rva as usize, site.expect.len()) != site.expect {
            log(&format!("party: {} is not as expected; no Party list", site.name));
            return false;
        }
    }
    INSTALLED.store(true, Ordering::Release);
    log(&format!(
        "party: the command menu's Party list (page 2): {}; each in a battle where its Friend card can drop (Re:CoM's table 0x1407C1F10) once Re:CoM taught it, through Re:CoM's sleight start",
        pt::ALL.iter().map(|f| format!("{} {:#x} ({} MP)", f.name, f.sleight, f.mp)).collect::<Vec<_>>().join(", ")
    ));
    true
}

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// The Party sleights Re:CoM has taught (Re:CoM's own test, a pure read of its save).
pub fn learned() -> Set {
    if !installed() {
        return 0;
    }
    // SAFETY: Re:CoM's own test (bytes checked at install), a pure read.
    let f = unsafe { func::<LearnedFn>(LEARNED) };
    let mut s = 0;
    for (i, fr) in pt::ALL.iter().enumerate() {
        // SAFETY: as above.
        if unsafe { f(fr.sleight) } & 0xFF != 0 {
            s |= 1 << i;
        }
    }
    s
}

/// The Party sleights' ids Re:CoM has taught (the Customize picker's).
pub fn learned_ids() -> Vec<u16> {
    let s = learned();
    pt::ALL.iter().enumerate().filter(|(i, _)| s >> i & 1 != 0).map(|(_, f)| f.sleight).collect()
}

/// Where this battle is, from Re:CoM's getters.
unsafe fn place() -> Place {
    Place {
        world: func::<IntFn>(WORLD)(0) & 0xFF,
        room_type: func::<VoidFn>(ROOM_TYPE)(),
        riku: func::<VoidFn>(RIKU)() & 0xFF != 0,
        minigame: func::<VoidFn>(MINIGAME)() & 0xFF != 0,
        met: func::<VarGetFn>(VAR_GET)(MET_VAR.0, MET_VAR.1) as u32,
    }
}

/// (taught, available in this battle) for player `pl`'s menu build. Read at the first build in a
/// battle room and kept for that battle's player (bit 31 read, the taught set in bits 16.., the
/// available set in bits 0..); outside battle: (taught, none), Re:CoM's test each call (11 calls of
/// a save read, only while page 2 shows or a slot holds a Party sleight; not measured).
pub unsafe fn sets(pl: usize) -> (Set, Set) {
    if !installed() || pl == 0 {
        return (0, 0);
    }
    if rd::<i32>(pl + p::BATTLE) == 0 {
        AVAIL.store(0, Ordering::Relaxed);
        return (learned(), 0);
    }
    let a = AVAIL.load(Ordering::Relaxed);
    if a & READ != 0 && AVAIL_PLAYER.load(Ordering::Relaxed) == pl {
        return (((a >> 16) & 0x7FFF) as Set, (a & 0x7FFF) as Set);
    }
    let taught = learned();
    let at = place();
    let s = pt::available(taught, &at);
    AVAIL.store(READ | (taught as u32) << 16 | s as u32, Ordering::Relaxed);
    AVAIL_PLAYER.store(pl, Ordering::Relaxed);
    log(&format!(
        "party: this battle (world {}, room type {}, friends met {:#x}, drop row {}; taught {:#x}): {}",
        at.world,
        at.room_type as i32,
        at.met,
        pt::drop_row(&at),
        taught,
        if s == 0 { "no Party sleight available".to_string() } else { pt::list(s).iter().map(|&i| pt::ALL[i].name).collect::<Vec<_>>().join(", ") }
    ));
    (taught, s)
}
