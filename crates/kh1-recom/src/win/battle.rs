//! M6 stage 1, battles at the KH1 setting (decisions 29 / 30; `crate::battle`,
//! `docs/recom-m6-battles.md`).
//!
//! - `manager_ctor` (`0x1401859B0`, once per battle): decides whether this battle is a KH1 one
//!   (Combat = KH1 and Sora's story) and switches the three serialiser patches P2–P4 to match
//!   with checked writes, all or none. At the Re:CoM setting the bytes are Re:CoM's own.
//! - `card_play` (`0x140198870`, enemy vtable `+0x98`, once per granted attack): in a KH1 battle
//!   the enemy enters its chosen attack state as message 2 would, with no card. Re:CoM's own
//!   state code runs the attack. CBTL's lock gates keep Re:CoM's refusal.
//! - `set_play_card` (`0x140163830`, CBTL::setPlayCard): a pass-through that logs, once per
//!   class, who still plays a side-2 card in a KH1 battle (human bosses, Marluxia 3's sleights),
//!   and (stage 3) each side-2 card, per battle, with Sora's action and what bosses read.
//! - Stage 3 (`crate::boss`): P5, Re:CoM's "Sora's action" getter `0x14017F720` rewritten in
//!   place by the same switch (ours in a KH1 battle: action 0xA reads as 0xB); boss attacks
//!   without a card logged per battle; `duel_init` (`0x140220C40`, log only): Sora's card duel.
//!
//! Per frame: nothing. The trace (F9 only) reads the enemy list through `trace_token`.

use super::movement::{self as mv, func, orig, rd, wr};
use super::{base, log, va};
use crate::battle::{self as b, cbtl, enemy, Bytes, Gate, Handler, Patch, Path, Play, Why};
use crate::boss;
use crate::sites::{data, M6_BOSS_PATCHES, M6_PATCHES};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_CARD_PLAY: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_MANAGER_CTOR: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_SET_PLAY_CARD: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_DUEL_INIT: AtomicUsize = AtomicUsize::new(0);

/// This battle is a KH1 one (decided when Re:CoM built its battle enemy manager).
static KH1_BATTLE: AtomicBool = AtomicBool::new(false);
static ATTACK_LINES: AtomicU32 = AtomicU32::new(0);
static BATTLE_LINES: AtomicU32 = AtomicU32::new(0);
static REFUSAL_LINES: AtomicU32 = AtomicU32::new(0);
/// Stage 3: battles so far (manager ctors), the boss lines of this battle and in all, the side-2
/// cards of this battle, the duel lines.
static BATTLE_NO: AtomicU32 = AtomicU32::new(0);
static BOSS_BATTLE_LINES: AtomicU32 = AtomicU32::new(0);
static BOSS_TOTAL_LINES: AtomicU32 = AtomicU32::new(0);
static SIDE2_CARDS: AtomicU32 = AtomicU32::new(0);
static DUEL_LINES: AtomicU32 = AtomicU32::new(0);

/// Re:CoM's "Sora's action" getter (P5's site; `sites::M6_BOSS_PATCHES`).
const ACTION_GETTER: u32 = 0x0017_F720;
/// Sora's action (`player+0x40C`).
const PL_ACTION: usize = 0x40C;
type GetterFn = unsafe extern "win64" fn() -> u32;
type DuelInitFn = unsafe extern "win64" fn(usize, i32);
/// (vtable, path) pairs already logged: 0 cardless, 1 Re:CoM's card play (fallback), 2 a side-2
/// card set in a KH1 battle.
static CLASSES: Mutex<Vec<(usize, u8)>> = Mutex::new(Vec::new());

/// `0x1402AD300` (Checked (exe), `sites::M6_CALLEES`).
const TASK_BY_ID: u32 = 0x002A_D300;

type TaskFn = unsafe extern "win64" fn(u32) -> usize;
type PlayFn = unsafe extern "win64" fn(usize) -> u64;
type CtorFn = unsafe extern "win64" fn(usize, usize, usize, usize) -> usize;
type SetPlayFn = unsafe extern "win64" fn(usize, u32, i32, usize, usize, i32) -> u64;
type TypeFn = unsafe extern "win64" fn(usize) -> u32;

fn capped(counter: &AtomicU32, cap: u32, line: impl FnOnce() -> String) {
    if counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < cap).then_some(n + 1)).is_ok() {
        log(&format!("m6: {}", line()));
    }
}

/// A stage-3 boss line: per battle and in all ([`boss::boss_line`]). `line` returns None for a
/// line that doesn't count (not a boss).
fn boss_capped(line: impl FnOnce() -> Option<String>) {
    if !boss::boss_line(BOSS_BATTLE_LINES.load(Ordering::Relaxed), BOSS_TOTAL_LINES.load(Ordering::Relaxed)) {
        return;
    }
    if let Some(l) = line() {
        BOSS_BATTLE_LINES.fetch_add(1, Ordering::Relaxed);
        BOSS_TOTAL_LINES.fetch_add(1, Ordering::Relaxed);
        log(&format!("m6: {l}"));
    }
}

unsafe fn main_player() -> usize {
    rd(va(base(), data::MAIN_PLAYER))
}

/// Sora's action and what the getter answers now (P5 or Re:CoM's bytes).
unsafe fn action_and_answer() -> (Option<u32>, u32) {
    let pl = main_player();
    let a = (pl != 0).then(|| rd::<u32>(pl + PL_ACTION));
    (a, func::<GetterFn>(ACTION_GETTER)())
}

unsafe fn task(id: u32) -> usize {
    func::<TaskFn>(TASK_BY_ID)(id)
}

unsafe fn class_name(obj: usize) -> String {
    if obj == 0 {
        return "none".into();
    }
    let vt: usize = rd(obj);
    super::stats::rtti_name(base(), vt).map(|n| n.trim_start_matches(".?AV").trim_end_matches("@@").to_string()).unwrap_or_else(|| format!("vt {vt:#x}"))
}

/// Logs a class's path once.
unsafe fn class_once(obj: usize, path: u8, what: impl FnOnce(&str) -> String) {
    let vt: usize = if obj != 0 { rd(obj) } else { 0 };
    let new = CLASSES
        .lock()
        .map(|mut v| {
            let n = !v.contains(&(vt, path)) && v.len() < b::CLASS_LOG_MAX;
            if n {
                v.push((vt, path));
            }
            n
        })
        .unwrap_or(false);
    if new {
        let name = class_name(obj);
        log(&format!("m6: class {name}: {}", what(&name)));
    }
}

/// The enemies in Re:CoM's battle enemy manager (task type 0x1004), at most `max`.
unsafe fn enemies(max: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let m = task(b::TASK_ENEMY_MANAGER);
    if m == 0 {
        return out;
    }
    let mut node: usize = rd(m + b::MANAGER_LIST);
    let mut guard = 0;
    while node != 0 && guard < 256 && out.len() < max {
        let obj: usize = rd(node + b::NODE_OBJECT);
        if obj != 0 {
            let vt: usize = rd(obj);
            let ty: TypeFn = std::mem::transmute_copy(&rd::<usize>(vt + enemy::VT_TYPE));
            if ty(obj) == b::ENEMY_TYPE {
                out.push(obj);
            }
        }
        node = rd(node + b::NODE_NEXT);
        guard += 1;
    }
    out
}

unsafe fn state_of(e: usize) -> Option<usize> {
    crate::guard::state_index(e, rd(e + enemy::STATE))
}

unsafe fn attacking_now() -> usize {
    enemies(256).into_iter().filter(|&e| b::attacking(rd(e + enemy::FLAGS), state_of(e))).count()
}

unsafe fn handler_of(e: usize) -> Handler {
    let vt: usize = rd(e);
    let h: usize = rd(vt + enemy::VT_MESSAGE);
    let bs = base();
    if h == bs + b::HANDLER_GENERIC as usize {
        Handler::Generic
    } else if h == bs + b::HANDLER_MARLUXIA3 as usize {
        Handler::Marluxia3
    } else {
        Handler::Other
    }
}

/// Re:CoM's Heartless card play (`0x140198870`). In a KH1 battle: message 2's work, no card.
pub unsafe extern "win64" fn card_play(e: usize) -> u64 {
    let o: PlayFn = orig(&ORIG_CARD_PLAY);
    if !KH1_BATTLE.load(Ordering::Acquire) {
        return o(e);
    }
    let c = task(b::TASK_CBTL);
    let gate = (c != 0).then(|| Gate { lock: rd(c + cbtl::LOCK), gimmick: rd(c + cbtl::GIMMICK), duel: rd(c + cbtl::DUEL) });
    let handler = handler_of(e);
    let p = Play {
        kh1_battle: true,
        flags: rd(e + enemy::FLAGS),
        hp: rd(e + enemy::HP),
        chosen: rd(e + enemy::CHOSEN),
        handler,
        marluxia3_mode: if handler == Handler::Marluxia3 { rd(e + enemy::MARLUXIA3_MODE) } else { 0 },
        gate,
    };
    match b::decide(&p) {
        Path::Cardless { flags, state } => {
            wr(e + enemy::FLAGS, flags);
            wr(e + enemy::NEXT_STATE, b::state_address(e, state));
            class_once(e, 0, |_| "attacks without a card (P1)".into());
            boss_capped(|| {
                let name = class_name(e);
                boss::is_boss_class(&name).then(|| {
                    format!("boss attack without a card (battle {}): {name} id={} state={state} hp={} cooldown={}", BATTLE_NO.load(Ordering::Relaxed), rd::<u32>(e + enemy::ID), p.hp, rd::<i32>(e + enemy::COOLDOWN))
                })
            });
            capped(&ATTACK_LINES, b::ATTACK_LOG_LINES, || {
                format!(
                    "attack without a card: {} id={} state={state} attacking_now={} flags {:#x}->{flags:#x} cooldown={}",
                    class_name(e),
                    rd::<u32>(e + enemy::ID),
                    attacking_now(),
                    p.flags,
                    rd::<i32>(e + enemy::COOLDOWN)
                )
            });
            1
        }
        Path::Recom(why) => {
            match why {
                Why::Locked(g) => capped(&REFUSAL_LINES, b::REFUSAL_LOG_LINES, || {
                    format!("CBTL locked (+0x208={:#x}, gimmick={}, duel={}): Re:CoM refuses {} id={} as it refuses a card", g.lock, g.gimmick, g.duel, class_name(e), rd::<u32>(e + enemy::ID))
                }),
                why => {
                    class_once(e, 1, |_| format!("Re:CoM's card play ({why:?}); whether its card shows: the `boss card` / `side-2` lines"));
                    boss_capped(|| {
                        let name = class_name(e);
                        boss::is_boss_class(&name).then(|| format!("boss takes Re:CoM's card play (battle {}, {why:?}): {name} id={} state={}", BATTLE_NO.load(Ordering::Relaxed), rd::<u32>(e + enemy::ID), p.chosen))
                    });
                }
            }
            o(e)
        }
    }
}

/// Re:CoM's battle enemy manager ctor (once per battle): the battle's setting and P2–P4.
pub unsafe extern "win64" fn manager_ctor(this: usize, a: usize, c: usize, d: usize) -> usize {
    let kh1 = mv::kh1_on() && !super::stats::riku_story();
    let pace_kh1 = super::settings().enemy_pace == crate::settings::Mode::Kh1;
    let want = b::wanted(kh1, pace_kh1);
    let switched = switch_patches(M6_PATCHES, &want);
    // Stage 3, P5: "Sora is attacking" for the bosses, on its own (a foreign getter leaves P2-P4).
    let p5 = switch_patches(M6_BOSS_PATCHES, &[kh1]);
    KH1_BATTLE.store(kh1, Ordering::Release);
    let battle = BATTLE_NO.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    BOSS_BATTLE_LINES.store(0, Ordering::Relaxed);
    SIDE2_CARDS.store(0, Ordering::Relaxed);
    capped(&BATTLE_LINES, b::BATTLE_LOG_LINES, || {
        format!(
            "battle start (battle {battle}): {}; no-card attacks (P1) {}; think gate (P2) and grant all (P3): {}; crowd pace (P4): {}; Sora attacking for bosses (P5): {}",
            if kh1 { "KH1 (Combat = KH1, Sora's story)" } else if mv::kh1_on() { "Re:CoM (Riku's story)" } else { "Re:CoM (Combat = Re:CoM)" },
            if kh1 { "on" } else { "off" },
            match switched {
                Some(_) if want[0] => "ours",
                Some(_) => "Re:CoM's",
                None => "UNCHANGED: a site held unexpected bytes (see above); Re:CoM's turn-taking may stay",
            },
            match switched {
                Some(n) => format!(
                    "{} ({n} site(s) written this battle)",
                    if want[2] { "ours, each enemy as if alone (enemy_pace = kh1)" } else if kh1 { "Re:CoM's crowd sharing (enemy_pace = recom)" } else { "Re:CoM's" }
                ),
                None => "UNCHANGED".into(),
            },
            boss::p5_part(kh1, p5)
        )
    });
    orig::<CtorFn>(&ORIG_MANAGER_CTOR)(this, a, c, d)
}

/// Makes each patch of `patches` ours or Re:CoM's as `want` says, all or none (P2–P4 together,
/// P5 on its own). The number of sites written, or None.
unsafe fn switch_patches(patches: &[Patch], want: &[bool]) -> Option<usize> {
    let bs = base();
    let now: Vec<Bytes> = patches
        .iter()
        .map(|p| b::bytes_state(p, std::slice::from_raw_parts((bs + p.site.rva as usize) as *const u8, p.site.expect.len())))
        .collect();
    let pairs: Vec<(&[u8], &[u8])> = patches.iter().map(|p| (p.site.expect, p.new)).collect();
    let Some(writes) = b::plan_each(&pairs, &now, want) else {
        for (p, s) in patches.iter().zip(&now) {
            if *s == Bytes::Other {
                log(&format!("m6: {} holds bytes that are neither Re:CoM's nor ours; no patch switched", p.site.name));
            }
        }
        return None;
    };
    let mut done: Vec<b::Write> = Vec::new();
    for (i, from, to) in writes {
        let p = &patches[i];
        if !super::hook::patch(bs + p.site.rva as usize, from, to, p.site.name) {
            // Undo what this call wrote, so the set stays together.
            for &(j, from2, to2) in done.iter().rev() {
                let q = &patches[j];
                super::hook::patch(bs + q.site.rva as usize, to2, from2, q.site.name);
            }
            log(&format!("m6: a patch write failed; {} left as they were", patches.iter().map(|p| p.site.name).collect::<Vec<_>>().join(", ")));
            return None;
        }
        done.push((i, from, to));
    }
    let n = done.len();
    Some(n)
}

/// CBTL::setPlayCard (`0x140163830(cbtl, owner id, side, card, item, …)`): logs, once per class,
/// who still sets a side-2 card in a KH1 battle. Pass-through.
pub unsafe extern "win64" fn set_play_card(cb: usize, id: u32, side: i32, card: usize, item: usize, p6: i32) -> u64 {
    let r = orig::<SetPlayFn>(&ORIG_SET_PLAY_CARD)(cb, id, side, card, item, p6);
    if side == 2 && r as i32 > 0 && KH1_BATTLE.load(Ordering::Relaxed) {
        let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(id);
        class_once(obj, 2, |_| format!("plays a side-2 card at KH1 (Re:CoM's card path: id {id}, {})", if card != 0 { "a revolver card" } else { "a new card" }));
        // Stage 3: each side-2 card of this battle (a human boss's move list at work).
        let n = SIDE2_CARDS.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        boss_capped(|| {
            let (a, ans) = action_and_answer();
            Some(boss::card_line(&boss::CardLine {
                battle: BATTLE_NO.load(Ordering::Relaxed),
                n,
                class: &class_name(obj),
                id,
                revolver_card: card != 0,
                action: a.unwrap_or(0),
                answer: ans,
                duel_mode: rd(cb + cbtl::DUEL),
                hidden: super::deck::enemy_ui_hidden(),
            }))
        });
    }
    r
}

/// Sora's card-duel init (`0x140220C40(player, start)`, action 0x24; Marluxia 3's countdown).
/// Log only, pass-through.
pub unsafe extern "win64" fn duel_init(pl: usize, start: i32) {
    if start != 0 && KH1_BATTLE.load(Ordering::Relaxed) {
        let rev: usize = rd(pl + crate::deck::player::REVOLVER);
        let cards: i16 = if rev != 0 { rd(rev + crate::deck::revolver::CARDS) } else { 0 };
        capped(&DUEL_LINES, boss::DUEL_LOG_LINES, || boss::duel_line(BATTLE_NO.load(Ordering::Relaxed), cards));
    }
    orig::<DuelInitFn>(&ORIG_DUEL_INIT)(pl, start)
}

/// The trace's M6 token (F9 recording only).
pub unsafe fn trace_token() -> String {
    let es = enemies(256);
    let attacking = es.iter().filter(|&&e| b::attacking(rd(e + enemy::FLAGS), state_of(e))).count();
    let cells: Vec<String> = es
        .iter()
        .take(b::TRACE_ENEMIES)
        .map(|&e| b::trace_enemy(rd(e + enemy::ID), state_of(e), rd(e + enemy::FLAGS), rd(e + enemy::COOLDOWN)))
        .collect();
    let c = task(b::TASK_CBTL);
    let slot = (c != 0).then(|| rd::<i32>(c + cbtl::SLOT_ENEMY));
    let m6 = b::trace_token(KH1_BATTLE.load(Ordering::Relaxed), attacking, slot, &cells);
    // Stage 3: P5's bytes now, Sora's action, what the bosses read, CBTL's duel mode, the enemy
    // card UI.
    let p = &M6_BOSS_PATCHES[0];
    let have = std::slice::from_raw_parts((base() + p.site.rva as usize) as *const u8, p.site.expect.len());
    let p5 = match b::bytes_state(p, have) {
        Bytes::Ours => "ours",
        Bytes::Recom => "recom",
        Bytes::Other => "other",
    };
    let (a, ans) = action_and_answer();
    let duel = (c != 0).then(|| rd::<u32>(c + cbtl::DUEL));
    format!("{m6}; {}", boss::trace_token(p5, a, ans, duel, super::deck::enemy_ui_hidden()))
}

/// Installs the three wrappers, all or none (needs the movement handlers: the setting), then
/// stage 3's log-only duel hook apart.
pub fn install(bs: usize) -> bool {
    let site = |name: &str| crate::sites::M6_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        super::hook::Hook { site: site("enemy card play (0x140198870)"), ours: card_play as *const () as usize, original: &ORIG_CARD_PLAY },
        super::hook::Hook { site: site("battle enemy manager ctor (0x1401859B0)"), ours: manager_ctor as *const () as usize, original: &ORIG_MANAGER_CTOR },
        super::hook::Hook { site: site("CBTL setPlayCard (0x140163830)"), ours: set_play_card as *const () as usize, original: &ORIG_SET_PLAY_CARD },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if !unsafe { super::hook::install_all(bs, &hooks) } {
        return false;
    }
    log("m6: bosses (stage 3): P5 (Sora's action getter 0x14017F720) is switched with P2-P4 at each battle start; side-2 cards and boss attacks logged per battle");
    let duel = [super::hook::Hook { site: &crate::sites::M6_BOSS_HOOKS[0], ours: duel_init as *const () as usize, original: &ORIG_DUEL_INIT }];
    // SAFETY: as above.
    if unsafe { super::hook::install_all(bs, &duel) } {
        log("m6: card duel logged (0x140220C40, log only)");
    } else {
        log("m6: card duel not hooked (log only; the battle work stands)");
    }
    true
}
