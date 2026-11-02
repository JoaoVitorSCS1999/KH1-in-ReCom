//! Windows side: start-up (from Panacea's `OnInit`, or a fallback thread), hooks, and the glue
//! between our Config row and `kh1-recom.ini`.

mod abilities;
mod attack;
mod battle;
mod command_menu;
mod crash;
mod d2;
mod deck;
mod drops;
mod guard;
mod hook;
mod items;
mod keyblades;
mod kh1_hud;
mod magic;
mod magic_fx;
mod menu;
mod menu_sounds;
mod models;
mod movement;
mod mp;
mod mp_bar;
mod shop;
mod specials;
mod stats;
mod story_skip;
mod sys;
mod trace;
mod unlocks;

use crate::settings::{self, Settings};
use std::ffi::c_void;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, Once};
use sys::*;

static BASE: AtomicUsize = AtomicUsize::new(0);
static MODULE: AtomicUsize = AtomicUsize::new(0);
static START: Once = Once::new();
static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static INI_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);

/// The exe's image base.
pub(crate) fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}

/// A Steam virtual address (image base 0x140000000) in the running image.
pub(crate) fn va(base: usize, addr: u64) -> usize {
    base + (addr - 0x1_4000_0000) as usize
}

/// Appends a line to `kh1-recom.log`.
pub(crate) fn log(msg: &str) {
    let Ok(g) = LOG_PATH.lock() else { return };
    if let Some(p) = g.as_ref() {
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(p) {
            let _ = writeln!(f, "{msg}");
        }
    }
}

pub(crate) fn settings() -> Settings {
    SETTINGS.lock().ok().and_then(|g| g.clone()).unwrap_or_default()
}

/// Our Config row `k` was flipped: change the setting, rewrite the ini.
pub(crate) fn toggle(k: usize) {
    let mut s = settings();
    if k == crate::menu::ROW_GUARD_REACTIONS {
        s.guard_reactions = !s.guard_reactions;
    } else if k == crate::menu::ROW_ENEMY_PACE {
        s.enemy_pace = match s.enemy_pace {
            settings::Mode::Kh1 => settings::Mode::Recom,
            settings::Mode::Recom => settings::Mode::Kh1,
        };
    } else {
        s.combat = match s.combat {
            settings::Mode::Kh1 => settings::Mode::Recom,
            settings::Mode::Recom => settings::Mode::Kh1,
        };
    }
    if let Ok(mut g) = SETTINGS.lock() {
        *g = Some(s.clone());
    }
    if let Some(p) = INI_PATH.lock().ok().and_then(|g| g.clone()) {
        let text = std::fs::read_to_string(&p).unwrap_or_else(|_| settings::DEFAULT_FILE.to_string());
        if let Err(e) = std::fs::write(&p, settings::store(&text, &s)) {
            log(&format!("config menu: could not write {}: {e}", p.display()));
        }
    }
    if k == crate::menu::ROW_ENEMY_PACE {
        // Read at each battle start (`battle::manager_ctor`).
        log(&format!("config menu: enemy_pace = {} (from the next battle)", s.enemy_pace.as_str()));
        return;
    }
    if k == crate::menu::ROW_GUARD_REACTIONS {
        attack::set_guard_reactions(s.guard_reactions);
        log(&format!("config menu: guard_reactions = {}", settings::switch_str(s.guard_reactions)));
        return;
    }
    movement::set_mode(s.combat == settings::Mode::Kh1);
    log(&format!("config menu: combat = {}", s.combat.as_str()));
    // The MP bar follows the setting (made or removed on the live player gauges).
    mp_bar::refresh();
    // The weapon's group follows Combat: its files start loading now; the camp close shows it.
    models::prefetch();
    // KH1 casts' effects: Re:CoM's battle set starts loading now in the field (`magic_fx`).
    magic_fx::setting_changed();
}

/// The KH1-only clips the asset step added (Guard): the manifest
/// `kh1-motions.json` and the `PL0001.ANN` header Panacea will serve, looked for under the mod
/// folder Panacea passed, its `recom` folder, the DLL's folders and the game folder.
fn recom_bases(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = Vec::new();
    for d in dirs {
        bases.push(d.clone());
        if let Ok(rd) = std::fs::read_dir(d) {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().eq_ignore_ascii_case("recom") && e.path().is_dir() {
                    bases.push(e.path());
                }
            }
        }
    }
    bases
}

fn kh1_clips(dirs: &[PathBuf]) -> (crate::rows::Extension, Vec<(u32, String)>) {
    use crate::{motions, rows};
    let bases = recom_bases(dirs);
    let Some(manifest) = bases.iter().map(|b| b.join("kh1-motions.json")).find(|p| p.is_file()) else {
        log("movement: no kh1-motions.json found (asset step not run?): Guard has no clip");
        return (rows::Extension::default(), Vec::new());
    };
    let text = std::fs::read_to_string(&manifest).map_err(|e| e.to_string());
    let m = match text.clone().and_then(|t| motions::parse_manifest(&t)) {
        Ok(m) => m,
        Err(e) => {
            log(&format!("movement: {}: {e}", manifest.display()));
            return (rows::Extension::default(), Vec::new());
        }
    };
    // Stage 9c.1: KH1's specials' motions (`xl_limit.dat`), keyed by xl entry.
    let xl = match text.and_then(|t| motions::parse_xl(&t)) {
        Ok(x) => x,
        Err(e) => {
            log(&format!("specials: {}: {e}; no special clips", manifest.display()));
            Vec::new()
        }
    };
    let ann = bases.iter().map(|b| b.join("PLAYER").join("0001").join("PL0001.ANN")).find(|p| p.is_file()).and_then(|p| {
        use std::io::Read;
        let mut f = std::fs::File::open(&p).ok()?;
        let mut head = [0u8; 4];
        f.read_exact(&mut head).ok()?;
        let n = u32::from_le_bytes(head).min(4096);
        let mut rest = vec![0u8; motions::ann_header_len(n) - 4];
        f.read_exact(&mut rest).ok()?;
        let mut all = head.to_vec();
        all.extend_from_slice(&rest);
        motions::ann_header(&all).ok()
    });
    let Some(ann) = ann else {
        log("movement: no extended PL0001.ANN found next to the manifest: KH1-only clips not used");
        return (rows::Extension::default(), Vec::new());
    };
    // Entries Re:CoM already has (M9 9a: the modifiers it baked) play its own action row of the
    // clip; the row's name is read from Sora's action table in the running exe.
    let table = recom_row_names();
    let ext = rows::Extension::plan(|e| motions::clip_for_entry(&m, &ann, e).is_ok())
        .with_recom(|e| motions::recom_clip_for_entry(&m, e).and_then(|c| motions::row_of_clip(&table, &c.name)))
        .with_xl(|e| motions::clip_for_entry(&xl, &ann, e as u32).is_ok());
    let names = rows::appended_rows(&ext, |e| motions::clip_for_entry(&m, &ann, e).ok().map(|c| c.name.clone()), |e| motions::clip_for_entry(&xl, &ann, e as u32).ok().map(|c| c.name.clone()));
    let recom: Vec<(u16, u32)> = rows::KH1_ONLY.iter().zip(ext.recom.iter()).filter_map(|(&(req, _), r)| Some((req, (*r)?))).collect();
    log(&format!("movement: manifest {}; KH1-only clips {:?}; Re:CoM's rows for KH1 requests {:x?}; xl_limit clips (specials) {:?}", manifest.display(), names, recom, ext.xl));
    (ext, names)
}

/// The clip names of Re:CoM's own rows of Sora's action table (`0x1407C5E80`: 16-byte rows
/// `{const char* name; int index; int pad}`, 0x47 rows), read from the running exe.
fn recom_row_names() -> Vec<Option<String>> {
    let b = base();
    let table = va(b, crate::sites::data::SORA_TABLE);
    (0..crate::rows::RECOM_ROWS as usize)
        .map(|i| {
            // SAFETY: rows of the image's own table; each name a NUL-terminated string in the image,
            // read a byte at a time up to its NUL (at most 16: Re:CoM's clip names are shorter).
            unsafe {
                let p: usize = std::ptr::read_unaligned((table + i * 16) as *const usize);
                if p == 0 {
                    return None;
                }
                let mut name = Vec::new();
                for k in 0..16 {
                    match *((p + k) as *const u8) {
                        0 => return String::from_utf8(name).ok(),
                        c => name.push(c),
                    }
                }
                None
            }
        })
        .collect()
}

/// The KH1 movement handlers (`movement`) and the trace's sampling points (`trace`).
fn install_movement(base: usize, s: &Settings, dirs: &[PathBuf], game_dir: &Path) {
    let (ext, names) = kh1_clips(dirs);
    // SAFETY: data patch with checked old values, before the first room load.
    let ext = if unsafe { movement::extend_action_table(&ext, &names) } { ext } else { crate::rows::Extension::default() };
    let _ = movement::EXT.set(ext);
    let site = |name: &str| crate::sites::MOVE_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("move exe (actions 2/3/4)"), ours: movement::move_exe as *const () as usize, original: &movement::ORIG_MOVE_EXE },
        hook::Hook { site: site("stick class (0x1401E9630)"), ours: movement::stick_class as *const () as usize, original: &movement::ORIG_STICK_CLASS },
        hook::Hook { site: site("move step (0x1401EF3C0)"), ours: movement::move_step as *const () as usize, original: &movement::ORIG_MOVE_STEP },
        hook::Hook { site: site("jump init (0x1401F29F0)"), ours: movement::jump_init as *const () as usize, original: &movement::ORIG_JUMP_INIT },
        hook::Hook { site: site("avoid init (0x1401F10C0)"), ours: movement::avoid_init as *const () as usize, original: &movement::ORIG_AVOID_INIT },
        hook::Hook { site: site("jump exe (action 7)"), ours: movement::jump_exe as *const () as usize, original: &movement::ORIG_JUMP_EXE },
        hook::Hook { site: site("CPlayerSora avoid exe (action 6)"), ours: movement::avoid_exe as *const () as usize, original: &movement::ORIG_AVOID_EXE },
        hook::Hook { site: site("CPlayerSora glide exe (action 8)"), ours: movement::glide_exe as *const () as usize, original: &movement::ORIG_GLIDE_EXE },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        movement::mark_installed();
        movement::set_abilities(s.abilities);
        abilities::set_ini(&s.abilities);
        movement::set_mode(s.combat == settings::Mode::Kh1);
        log(&format!(
            "movement: KH1 handlers in Re:CoM's actions 2/3/4, 6, 7, 8 ({}); abilities {:?}",
            if s.combat == settings::Mode::Kh1 { "on: combat = kh1" } else { "pass-through: combat = recom" },
            s.abilities
        ));
    } else {
        log("movement: not hooked; Sora moves as in Re:CoM");
    }
    let stats_file = recom_bases(dirs).into_iter().map(|b| b.join("kh1-stats.json")).find(|p| p.is_file());
    let bar_file = recom_bases(dirs).into_iter().map(|b| b.join("kh1-mp-bar.json")).find(|p| p.is_file());
    let menu_file = recom_bases(dirs).into_iter().map(|b| b.join("kh1-command-menu.json")).find(|p| p.is_file());
    let hud_file = recom_bases(dirs).into_iter().map(|b| b.join("kh1-hud.json")).find(|p| p.is_file());
    let sound_file = recom_bases(dirs).into_iter().map(|b| b.join(crate::menu_sounds::MANIFEST)).find(|p| p.is_file());
    install_attack(base, s, stats_file.as_deref(), bar_file.as_deref(), menu_file.as_deref(), hud_file.as_deref(), sound_file.as_deref());
    install_battles(base);
    install_story_skip(base);
    let th = &crate::sites::TRACE_HOOKS;
    let hooks = [
        hook::Hook { site: &th[0], ours: trace::field_exe as *const () as usize, original: &trace::ORIG_FIELD },
        hook::Hook { site: &th[1], ours: trace::battle_exe as *const () as usize, original: &trace::ORIG_BATTLE },
    ];
    // SAFETY: as above.
    if unsafe { hook::install_all(base, &hooks) } {
        trace::start(game_dir.to_path_buf());
        log("trace: press F9 in game to start / stop recording kh1-recom-trace-<n>.csv next to the game");
    } else {
        log("trace: not hooked");
    }
}

/// KH1's Attack (`attack`, `docs/recom-kh1-attack.md`): the KH1 rows, the data patches, then the
/// hooks, all or nothing. Needs the movement handlers.
fn install_attack(base: usize, s: &Settings, stats_file: Option<&Path>, bar_file: Option<&Path>, menu_file: Option<&Path>, hud_file: Option<&Path>, sound_file: Option<&Path>) {
    if !movement::installed() {
        log("attack: movement not hooked; KH1 Attack not installed");
        return;
    }
    attack::set_debug(s.debug_attack);
    attack::set_guard_reactions(s.guard_reactions);
    abilities::set_all(s.kh1_all_abilities);
    if s.debug_attack {
        log("attack: debug_attack = on (first 200 lines)");
    }
    // SAFETY: reads of the mapped exe; data patches with checked old values.
    if !unsafe { attack::build_table(base) && attack::patch_data(base) } {
        return;
    }
    let site = |name: &str| crate::sites::ATTACK_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("request bits (0x1401F8150)"), ours: attack::request_bits as *const () as usize, original: &attack::ORIG_REQUEST_BITS },
        hook::Hook { site: site("use-card button (0x1401E8FF0)"), ours: attack::card_button as *const () as usize, original: &attack::ORIG_CARD_BUTTON },
        hook::Hook { site: site("initAttack (0x1401F0D90)"), ours: attack::init_attack as *const () as usize, original: &attack::ORIG_INIT_ATTACK },
        hook::Hook { site: site("field swing init (0x1401F0F70)"), ours: attack::field_init as *const () as usize, original: &attack::ORIG_FIELD_INIT },
        hook::Hook { site: site("field swing exe (0x1401EAEB0)"), ours: attack::swing_exe as *const () as usize, original: &attack::ORIG_SWING_EXE },
        hook::Hook { site: site("attack pair dispatch (0x14016EAD0)"), ours: attack::pair_dispatch as *const () as usize, original: &attack::ORIG_PAIR },
        hook::Hook { site: site("enemy hit handler (0x14017F7E0)"), ours: attack::enemy_hit as *const () as usize, original: &attack::ORIG_ENEMY_HIT },
        hook::Hook { site: site("player damage check (0x1401E6DE0)"), ours: attack::damage_check as *const () as usize, original: &attack::ORIG_DAMAGE_CHECK },
        hook::Hook { site: site("root motion step (0x14024DCA0)"), ours: attack::root_step as *const () as usize, original: &attack::ORIG_ROOT_STEP },
        hook::Hook { site: site("enemy attack hurt off (0x14017DA80)"), ours: attack::hurt_off as *const () as usize, original: &attack::ORIG_HURT_OFF },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        let a = s.abilities;
        log(&format!(
            "attack: KH1 Attack in Re:CoM's attack action 0xA, field and battle (combo_plus {}, air_combo_plus {}, combo_master {}); KH1 Keyblade chain in the pair dispatch; hits land during enemy cards; no-flinch roll; root motion one update late",
            a.combo_plus, a.air_combo_plus, a.combo_master
        ));
        if guard::install(base) {
            log(&format!(
                "guard reactions: in the pair dispatch and enemy hit wrappers (decision 27; {} at combat = kh1)",
                settings::switch_str(s.guard_reactions)
            ));
        }
        install_stats(base, stats_file);
        log(&format!(
            "mp: KH1 MP at combat = kh1 in Sora's story (M7 stage 7.1): save+{:#x}..={:#x} layout v{}; charge from KH1 hits in the enemy-hit wrapper, refill one point per frame in the request-bits wrapper; no hook of its own",
            crate::mp::BLOCK_FIRST,
            crate::mp::BLOCK_LAST,
            crate::mp::LAYOUT_V1
        ));
        install_mp_refills(base);
        // The MP bar (M7 stage 7.2): Re:CoM's player gauge draws it; needs the asset step's file.
        // The command menu (7.3) runs from its wrappers and needs its plate groups.
        if mp_bar::install(base, bar_file) {
            // KH1's own menu sounds (the KH1 look's), in Re:CoM's sound banks.
            if command_menu::install(base, menu_file, s) {
                menu_sounds::install(base, sound_file);
            }
            // The KH1 HUD (decision 39) hangs from the same gauge wrappers.
            kh1_hud::install(base, hud_file);
        }
        install_magic(base);
        install_items(base, s);
        install_specials(base);
        install_drops(base);
        install_unlocks(base);
    } else {
        log("attack: not hooked; Sora attacks as in Re:CoM (the Battle case and row-pick slot stay patched but are not reached)");
    }
}

/// KH1's cardless cast and Fire (M7 stage 7.4, `magic`): the KH1 rows, the Field exe's case for
/// action 0x13, then the exeMagic hook. Needs the Attack's hooks (the request-bits wrapper and the
/// enemy-hit wrapper carry its command and hits).
fn install_magic(base: usize) {
    // SAFETY: the vtable slot is a read of the mapped image.
    let slot_ok = unsafe {
        let p = &crate::sites::MAGIC_PTRS[0];
        movement::rd::<usize>(base + p.rva as usize) == base + p.target as usize
    };
    if !slot_ok {
        log("magic: CPlayerSora's exeMagic slot is not 0x140217EE0; no KH1 cast");
        return;
    }
    // SAFETY: reads of the mapped exe; data patches with checked old values.
    if !unsafe { magic::install(base) } {
        return;
    }
    let site = |name: &str| crate::sites::MAGIC_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [hook::Hook { site: site("exeMagic (0x140217EE0)"), ours: magic::exe_magic as *const () as usize, original: &magic::ORIG_EXE_MAGIC }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        magic::mark_installed();
        log(&magic::start_line());
        // SAFETY: branch patches with checked old bytes (code of the mapped exe).
        if unsafe { magic_fx::install(base) } {
            log("magic fx: Re:CoM's battle set (spell effects) in the field at combat = kh1 in Sora's story: field state 1 0x1401AF933 -> 0x140211AA0 then 0x140211960; camp unloads 0x1401A95BD / 0x1401A99F9 keep it");
        }
    } else {
        log("magic: exeMagic not hooked; no KH1 cast (the Field case stays patched but is not reached: Re:CoM never casts in the field)");
    }
}

/// M8 stage 8a.1 (`items`): KH1's items on Re:CoM's item action `0xE`: the Field exe's case for
/// it, then the exeItem hook. Needs the Attack's hooks (the request-bits wrapper carries the command
/// and the use task) and the command menu (its Items list issues them).
fn install_items(base: usize, s: &Settings) {
    // SAFETY: reads of the mapped exe; data patches with checked old values.
    if !unsafe { items::install(base, s) } {
        return;
    }
    let site = |name: &str| crate::sites::ITEM_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [hook::Hook { site: site("exeItem (0x1401ED6E0)"), ours: items::exe_item as *const () as usize, original: &items::ORIG_EXE_ITEM }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        items::mark_installed();
        log(&items::start_line());
        install_shop(base);
    } else {
        log("items: exeItem not hooked; no KH1 items (the Field case stays patched but is not reached: Re:CoM never uses items in the field)");
    }
}

/// M9 stage 9c.1 (`specials`): KH1's Sonic Blade on Re:CoM's sleight action `0x16`, in battle
/// (the Battle exe's own case): the exeStockSkill hook. Needs the Attack's hooks (the request-bits
/// wrapper, the pair dispatch and the enemy-hit wrapper carry its command, spheres and damage),
/// the command menu (its context tab offers it) and the asset step's xl_limit clips.
fn install_specials(base: usize) {
    if !command_menu::installed() {
        log("specials: no KH1 command menu; no KH1 specials");
        return;
    }
    // SAFETY: the vtable slot is a read of the mapped image.
    let slot_ok = unsafe {
        let p = &crate::sites::SPECIAL_PTRS[0];
        movement::rd::<usize>(base + p.rva as usize) == base + p.target as usize
    };
    if !slot_ok {
        log("specials: CPlayerSora's exeStockSkill slot is not 0x140218950; no KH1 specials");
        return;
    }
    // SAFETY: reads of the mapped exe.
    if !unsafe { specials::install(base) } {
        return;
    }
    let site = |name: &str| crate::sites::SPECIAL_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [hook::Hook { site: site("exeStockSkill (0x140218950)"), ours: specials::exe_stock as *const () as usize, original: &specials::ORIG_EXE_STOCK }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        specials::mark_installed();
        log(&specials::start_line());
    } else {
        log("specials: exeStockSkill not hooked; no KH1 specials");
    }
}

/// M8 stage 8a.3 (`shop`): the Items tab in Re:CoM's Moogle shop, a wrapper on its pack screen's
/// update. Needs KH1's items (their store and pickup rule).
fn install_shop(base: usize) {
    let site = |name: &str| crate::sites::SHOP_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [hook::Hook { site: site("pack screen update (0x1403A6930)"), ours: shop::pack_screen as *const () as usize, original: &shop::ORIG_PACK_SCREEN }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        shop::mark_installed();
        log(&shop::start_line());
    } else {
        log("shop: pack screen not hooked; the Moogle shop is Re:CoM's (no Items tab)");
    }
}

/// M8 stage 8a.4 (`drops`): KH1's item drops from enemies as a new Re:CoM prize kind. Needs the
/// items (the pickup puts the item in Sora's slots / stock) and the stats file's drop tables
/// (format 6). The look row first (harmless alone: Re:CoM's kinds read the same rows), then the
/// two wrappers, all or nothing.
fn install_drops(base: usize) {
    if !items::installed() {
        log("drops: KH1 items not installed; no KH1 drops");
        return;
    }
    if stats::drop_classes() == 0 {
        log("drops: kh1-stats.json has no drop tables (format 6: re-run kh1-recom-build); no KH1 drops");
        return;
    }
    // SAFETY: reads of the mapped exe; data patches with checked old values.
    if !unsafe { drops::install(base) } {
        return;
    }
    let site = |name: &str| crate::sites::DROP_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("defeat card prize (0x140180D20)"), ours: drops::defeat_prize as *const () as usize, original: &drops::ORIG_DEFEAT_PRIZE },
        hook::Hook { site: site("prize add (0x1401F4AB0)"), ours: drops::prize_add as *const () as usize, original: &drops::ORIG_PRIZE_ADD },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        drops::mark_installed();
        log(&drops::start_line());
    } else {
        log("drops: not hooked; no KH1 drops (the look table copy stays: Re:CoM's kinds read the same rows)");
    }
}

/// M7 stage 7.11 (`unlocks`, decision 34): KH1's spells learned from Re:CoM's card collection, through
/// a wrapper on Re:CoM's "first obtained" `0x14015C7F0`. Without it, spells come only from the debug
/// switch `kh1_all_spells`.
fn install_unlocks(base: usize) {
    let site = |name: &str| crate::sites::UNLOCK_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [hook::Hook { site: site("card first obtained (0x14015C7F0)"), ours: unlocks::first_obtained as *const () as usize, original: &unlocks::ORIG_FIRST_OBTAINED }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        unlocks::mark_installed();
        log(&unlocks::start_line());
    } else {
        log("unlocks: 0x14015C7F0 not hooked; no spell is learned (kh1_all_spells still gives all)");
    }
}

/// M6 stages 1 and 2 (`battle`, `deck`, decisions 29 / 30): at Combat = KH1 in Sora's story,
/// Heartless attack without cards, several at once, and Sora has no deck. Needs the movement
/// handlers (they carry the setting).
/// Decision 35: MP to full where Re:CoM fills Sora's HP (two wrappers and one call retarget,
/// `win::mp`). Each part on its own; a missing one only loses that refill.
fn install_mp_refills(base: usize) {
    let site = |name: &str| crate::sites::MP_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("HP refill state step (0x1401AC9D0)"), ours: mp::heal_state as *const () as usize, original: &mp::ORIG_HEAL_STATE },
        hook::Hook { site: site("level-up screen (0x14010EF10)"), ours: mp::level_up as *const () as usize, original: &mp::ORIG_LEVEL_UP },
        hook::Hook { site: site("save point update (0x1401C5180)"), ours: mp::save_point as *const () as usize, original: &mp::ORIG_SAVE_POINT },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    let w = unsafe { hook::install_all(base, &hooks) };
    // SAFETY: the call's bytes and context are checked inside `patch`.
    let c = unsafe { mp::install_checkpoint(base) };
    log(&format!(
        "mp: refill to full where Re:CoM fills HP (decision 35): state step and level-up wrappers {}, checkpoint call 0x140237F3A {}; KH1's save point refill (1 MP every 3rd frame on a save point) on the save point update 0x1401C5180",
        if w { "in" } else { "NOT in" },
        if c { "retargeted" } else { "NOT retargeted" }
    ));
}

fn install_battles(base: usize) {
    if !movement::installed() {
        log("m6: movement not hooked; battles stay Re:CoM's");
        return;
    }
    if battle::install(base) {
        log("m6: wrappers in (card play 0x140198870, battle enemy manager ctor 0x1401859B0, setPlayCard log 0x140163830); P2-P4 are switched at each battle start by the setting");
    } else {
        log("m6: not hooked; battles stay Re:CoM's (cards and turn-taking)");
    }
    // Stage 2: no deck for Sora (decision 29), on its own: it doesn't need stage 1's hooks.
    if deck::install(base) {
        log("m6: no-deck wrappers in (revolver build 0x1401F43D0, hit drop 0x14017E190); the 16 deck and enemy-card HUD mask reads are switched at each battle start by the setting");
    } else {
        log("m6: no-deck not hooked; Sora keeps Re:CoM's deck in battle");
    }
}

/// Story skips at Combat = KH1 (`story_skip`, owner 2026-11-01, option (a)): the intro tutorial and
/// Leon's training battle skipped through their own exit. One wrapper; the setting comes from the
/// movement handlers, so without them it always passes through.
fn install_story_skip(base: usize) {
    let hooks = [hook::Hook { site: &crate::sites::STORY_SKIP_HOOKS[0], ours: story_skip::battle_entry as *const () as usize, original: &story_skip::ORIG_BATTLE_ENTRY }];
    // SAFETY: the target's bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        log(&story_skip::start_line());
    } else {
        log("story skip: 0x140236690 not hooked; the tutorial battles play as in Re:CoM");
    }
}

/// The camp menu's "KH1" entry (`keyblades`, decision 25; stage 7.13: its submenu of Keyblades and
/// Customize): four camp wrappers, all or
/// nothing. Needs the Config input hook (the page runs through it), so it is installed only
/// after that one. Whether the entry shows is decided each time Re:CoM builds the list.
fn install_keyblades(base: usize, bases: &[PathBuf]) {
    let site = |name: &str| crate::sites::KEYBLADE_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("camp update (0x1400D9C80)"), ours: keyblades::camp_update as *const () as usize, original: &keyblades::ORIG_UPDATE },
        hook::Hook { site: site("camp entry build (0x1400E3DE0)"), ours: keyblades::entry_build as *const () as usize, original: &keyblades::ORIG_BUILD },
        hook::Hook { site: site("camp cursor set (0x1400DEA10)"), ours: keyblades::cursor_set as *const () as usize, original: &keyblades::ORIG_CURSOR },
        hook::Hook { site: site("camp teardown (0x1400D7C20)"), ours: keyblades::teardown as *const () as usize, original: &keyblades::ORIG_TEARDOWN },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        log("keyblades: camp wrappers in; the 8th entry \"KH1\" (Keyblades, Customize) shows at Combat = KH1 in Sora's story");
        keyblades::load_item_row(bases.iter().map(|b| b.join(crate::camp_rows::MANIFEST)).find(|p| p.is_file()).as_deref());
    } else {
        log("keyblades: not hooked; no KH1 entry (the Kingdom Key's numbers stand; shortcuts keep KH1's learn order)");
    }
}

/// KH1 damage and HP on Re:CoM's KH1-equivalent enemies (`stats`): needs `kh1-stats.json`.
fn install_stats(base: usize, stats_file: Option<&Path>) {
    if !stats::load(stats_file) {
        log("attack: damage: Re:CoM's own numbers (cardless AP = attack stat): no KH1 stats");
        return;
    }
    // The equipped Keyblade's model (decision 25): its groups are in the stats file (format 4).
    models::install(base);
    let site = |name: &str| crate::sites::STATS_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        hook::Hook { site: site("enemy stats init (0x14017FF50)"), ours: stats::stat_init as *const () as usize, original: &stats::ORIG_STAT_INIT },
        hook::Hook { site: site("enemy max HP (0x14017F5A0)"), ours: stats::max_hp as *const () as usize, original: &stats::ORIG_MAX_HP },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    if unsafe { hook::install_all(base, &hooks) } {
        log("attack: damage: KH1's (Sora's KH1 Strength, the twin's Defence, floor, cap and resistance; Re:CoM's weak part and falloff on top); KH1 HP on twins");
    } else {
        log("attack: damage: Re:CoM's own numbers: the stats hooks are not in");
    }
}

fn module_path(m: *mut c_void) -> PathBuf {
    let mut buf = vec![0u16; 1024];
    // SAFETY: the buffer length is passed; a null module names the exe.
    let n = unsafe { GetModuleFileNameW(m, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    PathBuf::from(String::from_utf16_lossy(&buf[..n.min(buf.len())]))
}

fn wide_to_path(p: *const u16) -> Option<PathBuf> {
    if p.is_null() {
        return None;
    }
    // SAFETY: Panacea passes a NUL-terminated wide string (its mod folder).
    let s = unsafe {
        let mut n = 0;
        while *p.add(n) != 0 && n < 4096 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    };
    (!s.is_empty()).then(|| PathBuf::from(s))
}

fn run(mod_dir: Option<PathBuf>) {
    // SAFETY: a null name returns the exe's own (mapped) image base.
    let base_ptr = unsafe { GetModuleHandleW(std::ptr::null()) } as *const u8;
    if base_ptr.is_null() {
        return;
    }
    // SAFETY: the first 0x1000 bytes of a mapped PE image are its headers.
    let headers = unsafe { std::slice::from_raw_parts(base_ptr, 0x1000) };
    let pe = crate::sites::pe_info(headers);
    let size = pe.as_ref().map(|p| p.size_of_image).unwrap_or(0);
    let read_site = |rva: u32, n: usize| -> Vec<u8> {
        if (rva as u64 + n as u64) > size as u64 {
            return Vec::new();
        }
        // SAFETY: inside the mapped image (checked against its size); code is readable.
        unsafe { std::slice::from_raw_parts(base_ptr.add(rva as usize), n).to_vec() }
    };
    let exe = module_path(std::ptr::null_mut());
    let game_dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let dll = module_path(MODULE.load(Ordering::Relaxed) as *mut c_void);
    // The DLL sits in mod_path\Recom\dll: its parent's parent is the mod folder.
    let mod_dir = mod_dir.or_else(|| dll.parent().and_then(Path::parent).map(Path::to_path_buf)).unwrap_or_else(|| game_dir.clone());
    let ini_path = game_dir.join("kh1-recom.ini");
    let ini = match std::fs::read_to_string(&ini_path) {
        Ok(t) => Ok(t),
        Err(_) => {
            let _ = std::fs::write(&ini_path, settings::DEFAULT_FILE);
            Err(format!("{} was missing and has been written with the defaults", ini_path.display()))
        }
    };
    let s = match &ini {
        Ok(t) => settings::parse(t).0,
        Err(_) => Settings::default(),
    };
    let (report, ok) = crate::startup_report(&exe.to_string_lossy(), pe, &read_site, ini);
    let log_path = game_dir.join("kh1-recom.log");
    let _ = std::fs::write(&log_path, &report);
    if let Ok(mut g) = LOG_PATH.lock() {
        *g = Some(log_path);
    }
    if let Ok(mut g) = INI_PATH.lock() {
        *g = Some(ini_path);
    }
    if let Ok(mut g) = SETTINGS.lock() {
        *g = Some(s.clone());
    }
    log(&format!("dll: {}", dll.display()));
    log(&format!("mod folder: {}", mod_dir.display()));
    if !ok {
        return;
    }
    let base = base_ptr as usize;
    BASE.store(base, Ordering::Relaxed);
    crash::install(MODULE.load(Ordering::Relaxed));

    // Config menu rows.
    if s.config_menu {
        let hooks = [
            hook::Hook { site: &crate::sites::HOOKS[0], ours: menu::cfg_input as *const () as usize, original: &menu::ORIG_INPUT },
            hook::Hook { site: &crate::sites::HOOKS[1], ours: menu::cfg_draw as *const () as usize, original: &menu::ORIG_DRAW },
        ];
        // SAFETY: the targets' bytes are checked again inside, before anything is written.
        if unsafe { hook::install_all(base, &hooks) } {
            log("config menu: rows added (camp menu > Config, below Re:CoM's rows: Combat, Guard reactions)");
            install_keyblades(base, &recom_bases(&manifest_dirs(&mod_dir, &dll, &game_dir)));
        } else {
            log("config menu: not hooked; use kh1-recom.ini");
        }
    } else {
        log("config menu: off in the ini; Re:CoM's menu untouched (no Keyblades entry either)");
    }

    let dirs = manifest_dirs(&mod_dir, &dll, &game_dir);
    install_movement(base, &s, &dirs, &game_dir);
}

/// Where the asset step's files are looked for: the mod folder, the DLL's folder and its parent,
/// the game folder (each also with its `recom` subfolder, `recom_bases`).
fn manifest_dirs(mod_dir: &Path, dll: &Path, game_dir: &Path) -> [PathBuf; 4] {
    let dll_dir = dll.parent().map(Path::to_path_buf).unwrap_or_default();
    [mod_dir.to_path_buf(), dll_dir.parent().map(Path::to_path_buf).unwrap_or_default(), dll_dir, game_dir.to_path_buf()]
}

fn start(mod_dir: Option<PathBuf>) {
    START.call_once(|| run(mod_dir));
}

/// Panacea calls this once after loading every DLL in `mod_path\Recom\dll`, with the mod folder.
///
/// # Safety
/// `folder` is null or a NUL-terminated wide string.
#[no_mangle]
pub unsafe extern "C" fn OnInit(folder: *const u16) {
    start(wide_to_path(folder));
}

/// # Safety
/// Called by the Windows loader.
#[no_mangle]
pub unsafe extern "system" fn DllMain(module: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        MODULE.store(module as usize, Ordering::Relaxed);
        // If nothing calls OnInit (loaded some other way), start anyway after a moment.
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(3));
            start(None);
        });
    }
    1
}
