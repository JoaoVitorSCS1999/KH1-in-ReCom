//! `kh1-recom-build`: the asset build step. Reads the player's own KH 1.5+2.5 folder (KH1's
//! and Re:CoM's archives) and writes `<mod>/recom/kh1-motions.json` (the KH1 entry → Re:CoM
//! slot map) and, when some KH1 motion is not in Re:CoM, `<mod>/recom/PLAYER/0001/PL0001.ANN`
//! and `<mod>/recom/PLAYER/0002/PL0002.ANN` (both extended) into Panacea's Re:CoM mod folder.
//! It also converts KH1's Keyblades, and the six KH2 ones Re:CoM has beyond KH1's (from the
//! player's KH2 FM archives), into Re:CoM weapon models (`PLAYER/00GG/PL00GG.MDL` and
//! `raw/PLAYER/00GG/PL00GG.RTM`, `docs/recom-keyblade-models.md`) and writes `kh1-stats.json`.
//! For the MP bar (M7 stage 7.2) it writes `raw/FORM/0002/FO0002.SPR` (Re:CoM's HUD sprite file
//! with KH1's MP gauge colours as new groups) and `kh1-mp-bar.json` (`docs/recom-m7-mp-bar.md`).
//! For the command menu (M7 stage 7.3) the same `FO0002.SPR` also gets Re:CoM's row plates, and
//! `kh1-command-menu.json` holds the menu's words from KH1's own `Word.bin` in every language
//! (`docs/recom-m7-command-menu.md`). For the KH1 HUD (decision 39, H1) it writes `kh1hud.lad` into
//! the same `FO0002.SPR`, `raw/FORM/0002/FO0002.RTM` with `kh1hud.imz`, and `kh1-hud.json`
//! (`docs/recom-kh1-hud.md`). For the command menu's sounds it writes `kh1-menu-sounds.bin` (KH1's
//! system sound bank `se000.win32.scd` from `amusic/se000.dat`, unchanged) and
//! `kh1-menu-sounds.json`, plus the menu entries as WAVs in `kh1-menu-sounds-preview/` for
//! listening (`docs/kh1-recom-assets.md`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use kh1_recom_assets::ann::Ann;
use kh1_recom_assets::build::{build, BuildOptions, Inputs, RECOM_SORA_SLOTS};
use kh1_recom_assets::install;
use kh1_recom_assets::keyblade::{convert, convert_kh2, group_for_card, kh2_template_distance, Converted, KeybladeModel, KH2_TEMPLATE_MATCH};
use kh1_recom_assets::kh2_keyblades;
use kh1_recom_assets::mdl::Mdl;
use kh1_recom_assets::stats::{Kh2Row, KH1_KEYBLADES};

const USAGE: &str = "usage: kh1-recom-build [--game <KH 1.5+2.5 folder>] [--mod-dir <Panacea mod_path>] [--out <file.ANN>]
                       [--loop E,E..] [--once E,E..] [--dry-run]

  --game     the game folder (holds Image/dt/Recom.hed and kh1_*.hed and the exes); default: .
  --mod-dir  Panacea's mod_path; default: mod_path= from <game>/panacea_settings.txt
  --out      write Sora's pack here (PL0002.ANN, the manifest, the stats and the Keyblade models'
             PLAYER/ and raw/PLAYER/ folders next to it) instead of
             <mod>/recom/PLAYER/000{1,2}/PL000{1,2}.ANN and <mod>/recom/kh1-motions.json
  --loop / --once   force new KH1 clips to loop / play once (default: KH1 pose at N = pose at 0)
  --dry-run  convert and report, write nothing";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("kh1-recom-build: {e}");
            ExitCode::FAILURE
        }
    }
}

fn entries(s: &str) -> Result<Vec<usize>, String> {
    s.split(',').filter(|x| !x.is_empty()).map(|x| x.trim().parse::<usize>().map_err(|_| format!("bad entry '{x}'"))).collect()
}

fn run() -> Result<(), String> {
    let mut game = PathBuf::from(".");
    let mut mod_dir: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut dry = false;
    let mut opt = BuildOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or_else(|| format!("{a} needs a value\n{USAGE}"));
        match a.as_str() {
            "--game" => game = PathBuf::from(val()?),
            "--mod-dir" => mod_dir = Some(PathBuf::from(val()?)),
            "--out" => out = Some(PathBuf::from(val()?)),
            "--loop" => opt.loop_overrides.extend(entries(&val()?)?.into_iter().map(|e| (e, true))),
            "--once" => opt.loop_overrides.extend(entries(&val()?)?.into_iter().map(|e| (e, false))),
            "--dry-run" => dry = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            _ => return Err(format!("unknown argument '{a}'\n{USAGE}")),
        }
    }
    let t0 = Instant::now();
    let keys = install::keys(&game).map_err(|e| format!("key table: {e}"))?;
    let arc = install::recom_archive(&game).map_err(|e| e.to_string())?;
    let ann = install::read_recom(&arc, &keys, install::SORA_ANN).map_err(|e| format!("{}: {e}", install::SORA_ANN))?;
    let mdl = install::read_recom(&arc, &keys, install::SORA_MDL).map_err(|e| format!("{}: {e}", install::SORA_MDL))?;
    let ses = install::read_recom(&arc, &keys, install::SORA_SES).map_err(|e| format!("{}: {e}", install::SORA_SES))?;
    // Sora's Halloween Town pack (group 11), extended with the same clips (`extend_costume_pack`).
    let halloween = match arc.find(install::HALLOWEEN_ANN) {
        Some(_) => Some(install::read_recom(&arc, &keys, install::HALLOWEEN_ANN).map_err(|e| format!("{}: {e}", install::HALLOWEEN_ANN))?),
        None => None,
    };
    let halloween_ses = match arc.find(install::HALLOWEEN_SES) {
        Some(_) => Some(install::read_recom(&arc, &keys, install::HALLOWEEN_SES).map_err(|e| format!("{}: {e}", install::HALLOWEEN_SES))?),
        None => None,
    };
    let (mset, mdls) = install::read_kh1(&game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    // The Keyblade (CPlayerWeapon, resource group 2): converted when both files are there.
    let weapon = match (arc.find(install::KEYBLADE_ANN), arc.find(install::KEYBLADE_MDL)) {
        (Some(_), Some(_)) => Some((
            install::read_recom(&arc, &keys, install::KEYBLADE_ANN).map_err(|e| format!("{}: {e}", install::KEYBLADE_ANN))?,
            install::read_recom(&arc, &keys, install::KEYBLADE_MDL).map_err(|e| format!("{}: {e}", install::KEYBLADE_MDL))?,
        )),
        _ => {
            println!("note: {} / .MDL not in Recom.hed; the Keyblade pack is not built", install::KEYBLADE_ANN);
            None
        }
    };
    let pl0002_mdl = weapon.as_ref().map(|w| w.1.clone());
    let t_read = t0.elapsed();
    let mut sources = vec![
        ("PL0001.ANN sha256", kh1_formats::egs::sha256_hex(&ann)),
        ("PL0001.MDL sha256", kh1_formats::egs::sha256_hex(&mdl)),
        ("xa_ex_0010.mset sha256", kh1_formats::egs::sha256_hex(&mset)),
        ("xa_ex_0010.mdls sha256", kh1_formats::egs::sha256_hex(&mdls)),
    ];
    if let Some((a, m)) = &weapon {
        sources.push(("PL0002.ANN sha256", kh1_formats::egs::sha256_hex(a)));
        sources.push(("PL0002.MDL sha256", kh1_formats::egs::sha256_hex(m)));
    }
    let t1 = Instant::now();
    let built = build(&Inputs { mset, mdls, ann, mdl, weapon }, &opt).map_err(|e| e.to_string())?;
    let t_conv = t1.elapsed();
    let n_recom = built.recom_entries();
    let n_new = built.new_slots();
    let n_edit = built.entries.iter().filter(|e| e.match_kind == Some(kh1_recom_assets::matching::MatchKind::Edited)).count();
    println!(
        "{} KH1 motions: {} play Re:CoM's own clips ({} bakes, {} with Re:CoM edits), {} get new slots {}; read {:.2}s, match + convert {:.2}s",
        built.entries.len(),
        n_recom,
        n_recom - n_edit,
        n_edit,
        n_new,
        if built.changed { format!("{}..{} (pack {} bytes, {} slots)", built.base_slots, built.ann.slots.len() - 1, built.bytes.len(), built.ann.slots.len()) } else { "(none: Re:CoM's packs are left as they are)".into() },
        t_read.as_secs_f64(),
        t_conv.as_secs_f64()
    );
    for e in built.entries.iter().filter(|e| !e.ambiguous.is_empty()) {
        println!("note: KH1 entry {} also matches Re:CoM slots {:?} (chose {})", e.kh1_entry, e.ambiguous, e.slot);
    }
    if let Some(w) = &built.weapon {
        println!("Keyblade: {} new clips; pack {} bytes ({} slots)", n_new, w.bytes.len(), w.ann.slots.len());
    }
    // KH1's Keyblades as Re:CoM weapon models (decision 25).
    let t2 = Instant::now();
    let mut keyblades = convert_keyblades(&game, keys.clone(), pl0002_mdl.as_deref())?;
    // The six from KH2 (decision 25): KH2's models, Strength and Magic, reach from the blade.
    let (kh2_rows, kh2_models) = kh2_keyblades_step(&game, keys.clone(), pl0002_mdl.as_deref())?;
    for r in &kh2_rows {
        let k = &r.keyblade;
        println!(
            "KH2 Keyblade card {:#x} {}: item {:#x}, stat row {} (Strength +{}, Magic +{}), obj/{}.mdlx, blade {:.4} m (Kingdom Key {:.4} m)",
            k.card, k.name, k.item, k.stat, k.strength, k.magic, k.model, r.blade_length, r.kk_blade_length
        );
    }
    keyblades.extend(kh2_models);
    for (m, c) in &keyblades {
        match c {
            Some(c) => println!("Keyblade card {} → group {}: {} (MDL {} bytes, RTM {} bytes)", m.card, m.group, c.report.line(), c.mdl.len(), c.rtm.len()),
            None => println!("Keyblade card {} → group {}: Re:CoM's own PL0002 ({:?})", m.card, m.group, m.textures),
        }
    }
    println!("Keyblades: {} converted in {:.2}s", keyblades.iter().filter(|k| k.1.is_some()).count(), t2.elapsed().as_secs_f64());
    // The MP bar (M7 stage 7.2): built here, written with the other files below.
    let mp_bar = match mp_bar_build(&game, keys.clone(), &arc) {
        Ok(b) => Some(b),
        Err(e) => {
            println!("note: MP bar not built: {e}");
            None
        }
    };
    // The command menu's words (M7 stage 7.3): KH1's own, from the player's btltbl.bin assets.
    let menu = match (&mp_bar, command_menu_build(&game, keys.clone(), &arc)) {
        (Some(_), Ok(m)) => Some(m),
        (None, _) => {
            println!("note: command menu not built: it needs the MP bar's FO0002.SPR (its plates are added there)");
            None
        }
        (_, Err(e)) => {
            println!("note: command menu not built: {e}");
            None
        }
    };
    // The KH1 HUD set (decision 39, stage H1): kh1hud.lad into the bar's FO0002.SPR, kh1hud.imz
    // into FO0002.RTM.
    let mut mp_bar = mp_bar;
    let hud = match mp_bar.as_mut() {
        Some(b) => match kh1_hud_build(&game, keys.clone(), &arc, b) {
            Ok(h) => Some(h),
            Err(e) => {
                println!("note: KH1 HUD not built: {e}");
                None
            }
        },
        None => {
            println!("note: KH1 HUD not built: it needs the MP bar's FO0002.SPR");
            None
        }
    };
    // KH1's command menu sounds (owner, 2026-11-01): KH1's system sound bank for Re:CoM's banks.
    let menu_sounds = match menu_sounds_build(&game, keys.clone()) {
        Ok(m) => Some(m),
        Err(e) => {
            println!("note: KH1 menu sounds not taken: {e} (the menu keeps Re:CoM's sounds)");
            None
        }
    };
    if dry {
        return Ok(());
    }
    // The runtime (crates/kh1-recom, win/motion.rs) looks for the manifest at the recom root.
    let (ann_path, wep_path, man, hw_dir) = match out {
        Some(p) => {
            let m = p.with_file_name(install::MANIFEST);
            let w = p.with_file_name("PL0002.ANN");
            let h = p.with_file_name("PL0011.ANN");
            (p, w, m, h)
        }
        None => {
            let m = match mod_dir {
                Some(m) => m,
                None => install::mod_path(&game).map_err(|e| e.to_string())?,
            };
            (install::mod_file(&m, install::SORA_ANN), install::mod_file(&m, install::KEYBLADE_ANN), install::recom_dir(&m).join(install::MANIFEST), install::mod_file(&m, install::HALLOWEEN_ANN))
        }
    };
    if wep_path == ann_path {
        return Err("--out must not be named PL0002.ANN".into());
    }
    if built.changed {
        if let Some(w) = &built.weapon {
            write(&wep_path, &w.bytes)?;
        }
        write(&ann_path, &built.bytes)?;
        match &halloween {
            Some(h) => write(&hw_dir, &kh1_recom_assets::build::extend_costume_pack(&built, h).map_err(|e| format!("{}: {e}", install::HALLOWEEN_ANN))?)?,
            None => println!("note: {} not in Recom.hed; Halloween Town keeps Re:CoM's pack", install::HALLOWEEN_ANN),
        }
    } else {
        // Nothing new: Panacea serves Re:CoM's own files. Remove packs an earlier run left.
        for p in [&ann_path, &wep_path] {
            remove_stale(p)?;
        }
    }
    if let Some(dir) = man.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let json = built.manifest_json(&sources.iter().map(|(k, v)| (*k, v.clone())).collect::<Vec<_>>());
    std::fs::write(&man, &json).map_err(|e| format!("{}: {e}", man.display()))?;
    println!("wrote {} ({} bytes)", man.display(), json.len());
    // Sounds for the KH1-only clips (Guard, Counterattack) in Re:CoM's own `PL0001.SES`.
    let clip = |entry: usize| built.entries.iter().find(|e| e.kh1_entry == entry && e.source.as_str() == "kh1").map(|e| e.name.clone());
    let (guard, counter) = (clip(110), clip(111));
    let mut sound = kh1_recom_assets::ses::parse(&ses).map_err(|e| format!("{}: {e}", install::SORA_SES))?;
    let n = kh1_recom_assets::ses::add_kh1(&mut sound, guard.as_deref(), counter.as_deref()).map_err(|e| e.to_string())?;
    let ses_path = ann_path.with_file_name("PL0001.SES");
    write(&ses_path, &kh1_recom_assets::ses::write(&sound))?;
    println!("wrote {} ({} KH1 clip entries: guard {:?}, counterattack {:?})", ses_path.display(), n, guard, counter);
    // Halloween Town's sound events: Re:CoM's `PL0011.SES` with the same KH1 entries.
    if let Some(hs) = &halloween_ses {
        let mut hsound = kh1_recom_assets::ses::parse(hs).map_err(|e| format!("{}: {e}", install::HALLOWEEN_SES))?;
        kh1_recom_assets::ses::add_kh1(&mut hsound, guard.as_deref(), counter.as_deref()).map_err(|e| e.to_string())?;
        write(&hw_dir.with_file_name("PL0011.SES"), &kh1_recom_assets::ses::write(&hsound))?;
    }
    // The Keyblade models, under the same root as the manifest (`<mod>/recom` or --out's folder).
    let root = man.parent().map(Path::to_path_buf).unwrap_or_default();
    for (m, c) in &keyblades {
        if let Some(c) = c {
            for (rel, bytes) in kh1_recom_assets::keyblade::mod_files(m.group, c).map_err(|e| e.to_string())? {
                write(&rel.split('/').fold(root.clone(), |p, s| p.join(s)), &bytes)?;
            }
        }
    }
    // The MP bar: Re:CoM's FO0002.SPR with the bar's groups (a raw PKG entry, every language's
    // copy inside) and the manifest the DLL reads. Without it, an older manifest is removed so the
    // DLL never adds groups the served file may not have.
    let bar_manifest = root.join(kh1_recom_assets::mp_bar::MANIFEST);
    match &mp_bar {
        Some(b) => {
            let entry = kh1_formats::egs::build_plain_entry(&b.spr, &b.assets, b.date).map_err(|e| e.to_string())?;
            write(&root.join("raw").join(kh1_recom_assets::mp_bar::SPR_PATH.replace('/', std::path::MAIN_SEPARATOR_STR)), &entry)?;
            write(&bar_manifest, b.manifest.as_bytes())?;
        }
        None => {
            if bar_manifest.is_file() {
                std::fs::remove_file(&bar_manifest).map_err(|e| format!("{}: {e}", bar_manifest.display()))?;
                println!("removed {} (no MP bar this run)", bar_manifest.display());
            }
        }
    }
    // The KH1 HUD: the raw FO0002.RTM entry and kh1-hud.json; without them an older manifest is
    // removed (the DLL then keeps Re:CoM's HUD).
    let hud_manifest = root.join(kh1_recom_assets::kh1_hud::MANIFEST);
    match &hud {
        Some(h) => {
            write(&root.join("raw").join(kh1_recom_assets::mp_bar::RTM_PATH.replace('/', std::path::MAIN_SEPARATOR_STR)), &h.rtm_entry)?;
            write(&hud_manifest, h.manifest.as_bytes())?;
        }
        None => {
            if hud_manifest.is_file() {
                std::fs::remove_file(&hud_manifest).map_err(|e| format!("{}: {e}", hud_manifest.display()))?;
                println!("removed {} (no KH1 HUD this run)", hud_manifest.display());
            }
        }
    }
    // The command menu's manifest; without it (or without the bar) an older one is removed.
    let menu_manifest = root.join(kh1_recom_assets::command_menu::MANIFEST);
    match &menu {
        Some(m) => write(&menu_manifest, m.as_bytes())?,
        None => {
            if menu_manifest.is_file() {
                std::fs::remove_file(&menu_manifest).map_err(|e| format!("{}: {e}", menu_manifest.display()))?;
                println!("removed {} (no command menu this run)", menu_manifest.display());
            }
        }
    }
    // KH1's menu sounds: the pack and its manifest; without them older ones are removed (the DLL
    // then plays Re:CoM's sounds).
    let snd_manifest = root.join(kh1_recom_assets::menu_sounds::MANIFEST);
    let snd_pack = root.join(kh1_recom_assets::menu_sounds::PACK);
    match &menu_sounds {
        Some((bin, json, previews)) => {
            write(&snd_pack, bin)?;
            write(&snd_manifest, json.as_bytes())?;
            // For listening only (not part of the mod): each menu entry as a WAV.
            for (name, wav) in previews {
                write(&root.join(kh1_recom_assets::menu_sounds::PREVIEW_DIR).join(name), wav)?;
            }
        }
        None => {
            for p in [&snd_manifest, &snd_pack] {
                if p.is_file() {
                    std::fs::remove_file(p).map_err(|e| format!("{}: {e}", p.display()))?;
                    println!("removed {} (no KH1 menu sounds this run)", p.display());
                }
            }
        }
    }
    // KH1 stats (decisions 12, 18, 21, 25): the enemy twins' records, Sora's Strength, the
    // Keyblades (with their model groups).
    let models: Vec<KeybladeModel> = keyblades.iter().map(|k| k.0.clone()).collect();
    let stats = kh1_stats(&game, keys, &arc, &models, &kh2_rows)?;
    for n in &stats.notes {
        println!("{n}");
    }
    let sp = man.with_file_name(install::STATS);
    std::fs::write(&sp, &stats.json).map_err(|e| format!("{}: {e}", sp.display()))?;
    println!("wrote {} ({} KH1 enemy twins, {} boss twins, {} bytes)", sp.display(), stats.twins, stats.bosses, stats.json.len());
    Ok(())
}

/// Converts the 17 KH1 Keyblades (`KH1_KEYBLADES`): their model names from KH1's weapon table,
/// each `.wpn` with its HD assets from KH1's archives, laid out on Re:CoM's own `PL0002.MDL`.
/// The Kingdom Key keeps Re:CoM's PL0002 (group 2). Without PL0002.MDL nothing is converted.
fn convert_keyblades(game: &Path, keys: kh1_formats::egs::KeyTable, pl0002: Option<&[u8]>) -> Result<Vec<(KeybladeModel, Option<Converted>)>, String> {
    let Some(pl0002) = pl0002 else {
        println!("note: no PL0002.MDL; the Keyblade models are not built");
        return Ok(Vec::new());
    };
    let template = Mdl::parse(pl0002).map_err(|e| format!("{}: {e}", install::KEYBLADE_MDL))?;
    let inst = install::kh1(game, keys).map_err(|e| format!("KH1 files: {e}"))?;
    let btl = inst.read("btltbl.bin").map_err(|e| format!("btltbl.bin: {e}"))?;
    let table = kh1_formats::btltbl::Btltbl::parse(btl).map_err(|e| format!("btltbl.bin: {e}"))?;
    let mut out = Vec::new();
    for (card, name, item) in KH1_KEYBLADES {
        let group = group_for_card(*card).ok_or_else(|| format!("{name}: no group for card {card}"))?;
        if *card == 1 {
            out.push((KeybladeModel { card: *card, group, textures: template.parts[0].texture_names() }, None));
            continue;
        }
        let model = table.weapon_for_item(*item).map_err(|e| format!("{name}: {e}"))?.model;
        let file = format!("{model}.wpn");
        let (wpn, assets, _) = inst.read_full(&file).map_err(|e| format!("{name}: {file}: {e}"))?;
        let c = convert(&template, &model, &wpn, &assets).map_err(|e| format!("{name} ({file}): {e}"))?;
        out.push((KeybladeModel { card: *card, group, textures: c.textures.clone() }, Some(c)));
    }
    Ok(out)
}

/// A Keyblade's model entry and its conversion (None: Re:CoM's own PL0002).
type ModelOut = (KeybladeModel, Option<Converted>);

/// The six KH2 Keyblades (`kh2_keyblades::KH2_KEYBLADES`): their items, stat rows and models from
/// the player's KH2 files, each model's blade length, and (with Re:CoM's PL0002.MDL) the models
/// converted on it. KH2's Kingdom Key must match PL0002 first (`kh2_template_distance`), and
/// every KH2 Keyblade must have its skeleton. Without KH2's archives there are no KH2 rows.
fn kh2_keyblades_step(game: &Path, keys: kh1_formats::egs::KeyTable, pl0002: Option<&[u8]>) -> Result<(Vec<Kh2Row>, Vec<ModelOut>), String> {
    let inst = match install::kh2(game, keys) {
        Ok(i) => i,
        Err(e) => {
            println!("note: KH2's archives not read ({e}); the KH2 Keyblades are not built");
            return Ok((Vec::new(), Vec::new()));
        }
    };
    let read = |n: &str| inst.read(n).map_err(|e| format!("KH2 {n}: {e}"));
    let found = kh2_keyblades::find(&read("03system.bin")?, &read("00objentry.bin")?, &read(kh2_keyblades::SYS_BAR)?).map_err(|e| e.to_string())?;
    let kk = &found[0];
    let kk_mdlx = read(&kk.file())?;
    let reference = kh2_keyblades::reference_skeleton(&kk_mdlx).map_err(|e| e.to_string())?;
    let kk_len = kh2_keyblades::blade_length(&kk_mdlx).map_err(|e| e.to_string())?;
    let template = match pl0002 {
        Some(b) => {
            let t = Mdl::parse(b).map_err(|e| format!("{}: {e}", install::KEYBLADE_MDL))?;
            let d = kh2_template_distance(&t, &kk_mdlx).map_err(|e| e.to_string())?;
            println!("KH2's Kingdom Key ({}) on Re:CoM's PL0002: every vertex within {:.3} mm of PL0002's on the same bone", kk.model, d * 1000.0);
            if d > KH2_TEMPLATE_MATCH {
                return Err(format!("KH2's Kingdom Key is {:.3} mm from Re:CoM's PL0002 (more than {:.1} mm): the KH2 bone mapping does not hold for these files", d * 1000.0, KH2_TEMPLATE_MATCH * 1000.0));
            }
            Some(t)
        }
        None => None,
    };
    let mut rows = Vec::new();
    let mut models = Vec::new();
    for k in &found[1..] {
        let (mdlx, assets, _) = inst.read_full(&k.file()).map_err(|e| format!("{} ({}): {e}", k.name, k.file()))?;
        let len = kh2_keyblades::blade_length(&mdlx).map_err(|e| format!("{}: {e}", k.name))?;
        rows.push(Kh2Row { keyblade: k.clone(), blade_length: len, kk_blade_length: kk_len });
        if let Some(t) = &template {
            let group = group_for_card(k.card).ok_or_else(|| format!("{}: no group for card {:#x}", k.name, k.card))?;
            let c = convert_kh2(t, &k.model, &mdlx, &assets, &reference).map_err(|e| format!("{} ({}): {e}", k.name, k.file()))?;
            models.push((KeybladeModel { card: k.card, group, textures: c.textures.clone() }, Some(c)));
        }
    }
    Ok((rows, models))
}

/// The MP bar's files (M7 stage 7.2): the raw `FO0002.SPR` entry and `kh1-mp-bar.json`.
struct MpBar {
    /// The patched `FO0002.SPR`, its PC assets and the entry's date word (the entry is built at
    /// write time, after the KH1 HUD's files are added).
    spr: Vec<u8>,
    assets: Vec<kh1_formats::egs::HdAsset>,
    date: i32,
    manifest: String,
}

/// Reads KH1's gauge look (`command2/uitex.bin`) and Re:CoM's neutral texel (`FO0002.RTM`), and adds
/// the bar's groups to `Cockpit:00` in Re:CoM's `FO0002.SPR` and in each of its PC assets that is
/// a `Cockpit.lad` or a whole container (the language copies), as a plain PKG entry with the same
/// asset names, tags and date (`docs/recom-m7-mp-bar.md`).
fn mp_bar_build(game: &Path, keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive) -> Result<MpBar, String> {
    use kh1_recom_assets::mp_bar::{self as mb, Patched};
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let (uitex, hd, _) = inst.read_full(mb::KH1_UITEX).map_err(|e| format!("{}: {e}", mb::KH1_UITEX))?;
    if !hd.is_empty() {
        let names: Vec<&str> = hd.iter().map(|a| a.name.as_str()).collect();
        println!("note: {} has {} PC assets {:?}; the MP bar's colours are read from its PS2 pictures", mb::KH1_UITEX, hd.len(), names);
    }
    let look = mb::kh1_look(&uitex).map_err(|e| format!("{}: {e}", mb::KH1_UITEX))?;
    let rtm = install::read_recom(arc, &keys, mb::RTM_PATH).map_err(|e| format!("{}: {e}", mb::RTM_PATH))?;
    let neutral = mb::neutral_texel(&rtm).map_err(|e| format!("{}: {e}", mb::RTM_PATH))?;
    let e = arc.find(mb::SPR_PATH).ok_or_else(|| format!("{} not in Recom.hed", mb::SPR_PATH))?;
    let raw = arc.read_raw_entry(e).map_err(|e| format!("{}: {e}", mb::SPR_PATH))?;
    let date = i32::from_le_bytes(raw.get(12..16).ok_or("short entry")?.try_into().unwrap());
    let (spr, assets) = kh1_formats::egs::decode_entry_full(&keys, &raw).map_err(|e| format!("{}: {e}", mb::SPR_PATH))?;
    let (base, first) = mb::patch_spr(&spr, &look, neutral).map_err(|e| format!("{}: {e}", mb::SPR_PATH))?;
    let mut files = vec![(mb::SPR_PATH.to_string(), Patched::Bar(first))];
    let mut out_assets = Vec::with_capacity(assets.len());
    for a in &assets {
        let (data, how) = mb::patch_asset(&a.name, &a.data, &look, neutral).map_err(|e| format!("{} asset {}: {e}", mb::SPR_PATH, a.name))?;
        if let Patched::Bar(f) = how {
            if f != first {
                println!("note: {} asset {}: its own Cockpit:00 has {f} groups (the base file {first}); the bar's groups start at {f} there", mb::SPR_PATH, a.name);
            }
        }
        files.push((a.name.clone(), how));
        out_assets.push(kh1_formats::egs::HdAsset { name: a.name.clone(), orig_offset: a.orig_offset, data });
    }
    let sources = [
        ("uitex.bin sha256", kh1_formats::egs::sha256_hex(&uitex)),
        ("FO0002.RTM sha256", kh1_formats::egs::sha256_hex(&rtm)),
        ("FO0002.SPR sha256", kh1_formats::egs::sha256_hex(&spr)),
    ];
    let mut firsts: Vec<usize> = files.iter().filter_map(|f| if let Patched::Bar(g) = f.1 { Some(g) } else { None }).collect();
    firsts.sort_unstable();
    firsts.dedup();
    let manifest = mb::manifest_json(first, &firsts, &look, neutral, &files, &sources);
    let patched: Vec<&str> = files.iter().filter(|f| matches!(f.1, Patched::Bar(_))).map(|f| f.0.as_str()).collect();
    let kept: Vec<&str> = files.iter().filter(|f| f.1 == Patched::Kept).map(|f| f.0.as_str()).collect();
    println!(
        "MP bar: KH1 MP {:?}→{:?}, charge {:?}→{:?}, background {:?} / {:?}, segment {}×{} (charge {} high); Re:CoM neutral texel {:?}; groups {}..{} of {} added to {:?}{}",
        look.mp_top,
        look.mp_bottom,
        look.charge_top,
        look.charge_bottom,
        look.back,
        look.border,
        look.slot_w,
        look.mp_h,
        look.charge_h,
        neutral,
        first,
        first + mb::group::COUNT - 1,
        mb::SET,
        patched,
        if kept.is_empty() { String::new() } else { format!("; kept as they are: {kept:?}") }
    );
    Ok(MpBar { spr: base, assets: out_assets, date, manifest })
}

/// The KH1 HUD's files (stage H1): the raw `FO0002.RTM` entry and `kh1-hud.json`.
struct Hud {
    rtm_entry: Vec<u8>,
    manifest: String,
}

/// Builds the `kh1hud` set from KH1's `uitex.bin` (+ PC assets), `UK_uibin_g_s.bin`,
/// `UK_uibin_g_red.bin` and the party's faces, adds `kh1hud.lad` (+ `<LANG>_kh1hud.lad`) to the MP
/// bar's `FO0002.SPR` and `kh1hud.imz` (+ `<LANG>_kh1hud.imz`) to Re:CoM's `FO0002.RTM`
/// (`docs/recom-kh1-hud.md`).
fn kh1_hud_build(game: &Path, keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive, bar: &mut MpBar) -> Result<Hud, String> {
    use kh1_recom_assets::kh1_hud as kh;
    use kh1_recom_assets::mp_bar as mb;
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let (uitex, uitex_hd, _) = inst.read_full(mb::KH1_UITEX).map_err(|e| format!("{}: {e}", mb::KH1_UITEX))?;
    let g_s = inst.read(kh::G_S).map_err(|e| format!("{}: {e}", kh::G_S))?;
    let g_red = inst.read(kh::G_RED).map_err(|e| format!("{}: {e}", kh::G_RED))?;
    let g_act = inst.read(kh::G_ACT).map_err(|e| format!("{}: {e}", kh::G_ACT))?;
    // H5: the LOCK ON! plate and the target gauge.
    let lock = inst.read(kh::LOCK).ok();
    let g_enm = inst.read(kh::G_ENM).ok();
    // H6: the party's gauges.
    let g_c = inst.read(kh::G_C).ok();
    // H8: the LEVEL UP!! box.
    let lev_s = inst.read(kh::LEV_S).ok();
    // H4: the command menu's layouts, KH1's menu font (Image/dt/Theater) and the words.
    let menu_files = kh1_menu_files(game, &keys, &inst);
    let mut faces = Vec::new();
    for n in kh::FACES {
        let (m, a, _) = inst.read_full(n).map_err(|e| format!("{n}: {e}"))?;
        faces.push(kh::face(&m, &a).map_err(|e| format!("{n}: {e}"))?);
    }
    let faces: [kh::Source; 3] = faces.try_into().map_err(|_| "faces")?;
    let langs = kh::languages(&bar.assets);
    if langs.is_empty() {
        return Err("FO0002.SPR has no <LANG>_Cockpit.lad PC layouts (no language to build for)".into());
    }
    let lrefs: Vec<&str> = langs.iter().map(String::as_str).collect();
    let menu = match &menu_files {
        Ok(m) => Some(kh::MenuSources {
            com: &m.com,
            com_battle: &m.com_battle,
            mag: &m.mag,
            font: [&m.font[0], &m.font[1]],
            words: m.words.clone(),
            icon: m.icon.as_ref().map(|[a, b]| [a.as_slice(), b.as_slice()]),
            cut: m.cut.as_deref(),
            item: m.item.as_deref(),
        }),
        Err(e) => {
            println!("note: KH1 HUD: no KH1 command menu look: {e}");
            None
        }
    };
    let src = kh::Sources { uitex: &uitex, uitex_hd: &uitex_hd, g_s: &g_s, g_red: &g_red, g_act: &g_act, menu, lock: lock.as_deref(), g_enm: g_enm.as_deref(), g_c: g_c.as_deref(), lev_s: lev_s.as_deref(), faces };
    let b = kh::build(&src, &lrefs).map_err(|e| e.to_string())?;
    bar.spr = kh::add_to_spr(&bar.spr, &mut bar.assets, &b, &langs).map_err(|e| format!("{}: {e}", mb::SPR_PATH))?;
    // Re:CoM's HP ring and bar turned 180° for KH1's place (`hud_recom_hp_kh1_place`): new groups
    // in every copy of `gauge:01`. Without them the DLL leaves Re:CoM's gauge where it is.
    let recom_hp = match kh1_recom_assets::recom_hp::patch_spr(&mut bar.spr, &mut bar.assets) {
        Ok((first, files)) => {
            println!("KH1 HUD: Re:CoM's HP ring and bar turned 180° as {} groups {first}..{} in {files:?}", kh1_recom_assets::recom_hp::SET, first + 2);
            Some(first)
        }
        Err(e) => {
            println!("note: KH1 HUD: Re:CoM's HP gauge not turned ({e}); hud_recom_hp_kh1_place will leave it top-left");
            None
        }
    };
    let e = arc.find(mb::RTM_PATH).ok_or_else(|| format!("{} not in Recom.hed", mb::RTM_PATH))?;
    let raw = arc.read_raw_entry(e).map_err(|e| format!("{}: {e}", mb::RTM_PATH))?;
    let date = i32::from_le_bytes(raw.get(12..16).ok_or("short entry")?.try_into().unwrap());
    let (rtm, mut rtm_assets) = kh1_formats::egs::decode_entry_full(&keys, &raw).map_err(|e| format!("{}: {e}", mb::RTM_PATH))?;
    let rtm_new = kh::add_to_rtm(&rtm, &mut rtm_assets, &b, &langs).map_err(|e| format!("{}: {e}", mb::RTM_PATH))?;
    let rtm_entry = kh1_formats::egs::build_plain_entry(&rtm_new, &rtm_assets, date).map_err(|e| e.to_string())?;
    let sources = [
        ("uitex.bin sha256", kh1_formats::egs::sha256_hex(&uitex)),
        ("UK_uibin_g_s.bin sha256", kh1_formats::egs::sha256_hex(&g_s)),
        ("UK_uibin_g_red.bin sha256", kh1_formats::egs::sha256_hex(&g_red)),
        ("UK_uibin_g_act.bin sha256", kh1_formats::egs::sha256_hex(&g_act)),
        ("FO0002.RTM sha256", kh1_formats::egs::sha256_hex(&rtm)),
    ];
    let manifest = kh::manifest_json(&b, &langs, &sources, recom_hp);
    if let Some(m) = &b.menu {
        println!("KH1 HUD: command menu look from group {}: {} glyphs of KH1's menu font, words for {:?}{}", m.first, m.glyphs, m.words.iter().map(|w| w.0.as_str()).collect::<Vec<_>>(), if m.missing.is_empty() { String::new() } else { format!("; no glyph for {:?} (drawn as a space)", m.missing) });
        println!("KH1 HUD: FM context tab from uitex.bin picture 2's PC rows; Triangle {}", if m.tab_icon { "from Theater's Icon.tm2 (the PlayStation glyph)" } else { "MISSING (no Icon.tm2 / FontIcon.inf): the tab shows without it" });
        match m.items_list {
            Some(g) => println!("KH1 HUD: Items list (M8 stage 8a.1) from group {g}: {}", kh::ITEM),
            None => println!("note: KH1 HUD: no Items list panels (no {}): the DLL draws the Items list with the Magic list's", kh::ITEM),
        }
    }
    match &b.shortcut {
        Some(sc) => println!(
            "KH1 HUD: shortcut panel (M7 stage 7.12) from group {}: {} (picture 0's frame, picture 1's title); button glyphs {}",
            sc.first,
            kh::CUT,
            if sc.icons { "Triangle / Square / Cross / Circle from Theater's Icon.tm2" } else { "MISSING (no Icon.tm2 / FontIcon.inf): the panel shows without them" }
        ),
        None => println!("note: KH1 HUD: no shortcut panel (no {} or no command menu look): the DLL's shortcuts cast without a panel", kh::CUT),
    }
    println!(
        "KH1 HUD: {} groups ({} motion units), {} sprites, {} atlas items; PS2 atlas {}×{}, PC atlas {}×{}; kh1hud.lad + {} language copies into FO0002.SPR, kh1hud.imz + {} into FO0002.RTM ({:?}); {} PC texture rects redrawn from the PS2 pictures (repacked on PC)",
        b.groups,
        b.motion.first.len(),
        b.sprites,
        b.pieces,
        b.atlas.0,
        b.atlas.1,
        b.atlas.2,
        b.atlas.3,
        langs.len(),
        langs.len(),
        langs,
        b.refit
    );
    let (rf, rs) = b.ring;
    println!(
        "KH1 HUD: MP ring built as Re:CoM's HP ring from group {rf} ({} groups: KH1's quarter pictures whole, scissor parts, turned max-MP end, closing outline); cuts between cells: {} scissor cells with texels on both sides (largest alpha {}), charge cuts inside a cell: {} (staircases)",
        kh::ring_group::COUNT,
        rs.gap_conflicts,
        rs.gap_alpha,
        rs.cell_conflicts
    );
    Ok(Hud { rtm_entry, manifest })
}

/// The command menu look's KH1 inputs (H4).
struct MenuFiles {
    com: Vec<u8>,
    com_battle: Vec<u8>,
    mag: Vec<u8>,
    font: [Vec<u8>; 2],
    words: Vec<(String, Vec<(u16, String)>)>,
    /// `Icon.tm2`, `FontIcon.inf` (`Image/dt/Theater`), for the context tab's Triangle.
    icon: Option<[Vec<u8>; 2]>,
    /// `UK_uibin_cut.bin`, the shortcut panel (stage 7.12).
    cut: Option<Vec<u8>>,
    /// `UK_uibin_item.bin`, the Items list (M8 stage 8a.1).
    item: Option<Vec<u8>>,
}

fn kh1_menu_files(game: &Path, keys: &kh1_formats::egs::KeyTable, inst: &kh1_formats::egs::Install) -> Result<MenuFiles, String> {
    use kh1_recom_assets::command_menu as cm;
    use kh1_recom_assets::kh1_hud as kh;
    let rd = |n: &str| inst.read(n).map_err(|e| format!("{n}: {e}"));
    let (com, com_battle, mag) = (rd(kh::COM)?, rd(kh::COM_BATTLE)?, rd(kh::MAG)?);
    // The PC's button icons (the context tab's Triangle): `Image/dt/Theater`.
    let icon = (|| -> Result<[Vec<u8>; 2], String> {
        let theater = kh1_formats::egs::Archive::open(game.join("Image").join("dt").join("Theater.hed")).map_err(|e| format!("Image/dt/Theater.hed: {e}"))?;
        let [a, b] = kh::ICON_FILES.map(|n| theater.read(keys, n).map_err(|e| format!("Theater {n}: {e}")));
        Ok([a?, b?])
    })();
    let icon = match icon {
        Ok(i) => Some(i),
        Err(e) => {
            println!("note: KH1 HUD: no Triangle for the context tab: {e}");
            None
        }
    };
    let (_, sys_assets, _) = inst.read_full(kh::SYSFONT).map_err(|e| format!("{}: {e}", kh::SYSFONT))?;
    let dds = sys_assets.iter().find(|a| a.name.eq_ignore_ascii_case(kh::SYSFONT_HD)).ok_or_else(|| format!("{}: no PC asset {}", kh::SYSFONT, kh::SYSFONT_HD))?.data.clone();
    let font = [dds, rd(kh::FONT_TABLE)?];
    let (_, assets, _) = inst.read_full("btltbl.bin").map_err(|e| format!("btltbl.bin: {e}"))?;
    let mut words = Vec::new();
    for (lang, prefix) in cm::LANGUAGES {
        if let Some(a) = assets.iter().find(|a| a.name.eq_ignore_ascii_case(&format!("{prefix}Word.bin"))) {
            words.push((lang.to_string(), cm::words(&a.data).0));
        }
    }
    let cut = match rd(kh::CUT) {
        Ok(c) => Some(c),
        Err(e) => {
            println!("note: KH1 HUD: no shortcut panel: {e}");
            None
        }
    };
    let item = match rd(kh::ITEM) {
        Ok(c) => Some(c),
        Err(e) => {
            println!("note: KH1 HUD: no KH1 Items list panels (the DLL draws it with the Magic list's): {e}");
            None
        }
    };
    Ok(MenuFiles { com, com_battle, mag, font, words, icon, cut, item })
}

/// KH1's command words (`btltbl.bin`'s PC assets `<LANG>_Word.bin`, decoded for the menu's
/// commands) and the plates found in Re:CoM's `Cockpit:00` (base file and language copies):
/// `kh1-command-menu.json` (M7 stage 7.3, `docs/recom-m7-command-menu.md`).
fn command_menu_build(game: &Path, keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive) -> Result<String, String> {
    use kh1_recom_assets::command_menu as cm;
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let (btl, assets, _) = inst.read_full("btltbl.bin").map_err(|e| format!("btltbl.bin: {e}"))?;
    let mut langs = Vec::new();
    for (lang, prefix) in cm::LANGUAGES {
        let name = format!("{prefix}Word.bin");
        let Some(a) = assets.iter().find(|a| a.name.eq_ignore_ascii_case(&name)) else {
            println!("note: btltbl.bin has no PC asset {name}: no {lang} words");
            continue;
        };
        let (w, missing) = cm::words(&a.data);
        if !missing.is_empty() {
            println!("note: {name}: messages {missing:?} not decoded (the DLL falls back to US English for them)");
        }
        langs.push((*lang, w));
    }
    if !langs.iter().any(|(l, _)| *l == "us") {
        return Err("btltbl.bin has no US_Word.bin (the fallback language)".into());
    }
    // The plates: what the bar's patch added to the base file and to each language copy.
    let e = arc.find(kh1_recom_assets::mp_bar::SPR_PATH).ok_or("FO0002.SPR not in Recom.hed")?;
    let raw = arc.read_raw_entry(e).map_err(|e| e.to_string())?;
    let (spr, spr_assets) = kh1_formats::egs::decode_entry_full(&keys, &raw).map_err(|e| e.to_string())?;
    let base = cm::probe_plates(&spr).ok_or("FO0002.SPR: no Cockpit.lad")?;
    for a in spr_assets.iter().filter(|a| !a.data.starts_with(b"LAYD") || a.name.to_ascii_lowercase().ends_with("cockpit.lad")) {
        if let Some(p) = cm::probe_plates(&a.data) {
            if !p.found {
                println!("note: FO0002.SPR asset {}: Re:CoM's text plate is not where the base file has it; its menu rows have no plates", a.name);
            }
        }
    }
    for (lang, w) in &langs {
        let show: Vec<String> = w.iter().filter(|(id, _)| [1u16, 2, 3, 7, 343].contains(id)).map(|(id, t)| format!("{id} {t:?}")).collect();
        println!("command menu: {lang} words {} ({})", w.len(), show.join(", "));
    }
    println!("command menu: plates {} (highlight vertex colour {:#010x}), groups after the MP bar's", if base.found { "from Re:CoM's text plate" } else { "NOT found (empty groups)" }, base.colour);
    let sources = [("btltbl.bin sha256", kh1_formats::egs::sha256_hex(&btl)), ("FO0002.SPR sha256", kh1_formats::egs::sha256_hex(&spr))];
    Ok(cm::manifest_json(&langs, kh1_recom_assets::mp_bar::group::COUNT, base, &sources))
}

/// KH1's command menu sounds (`menu_sounds`): KH1's system sound bank, the PC asset
/// `se000.win32.scd` of `amusic/se000.dat` (bank 2, sound `id` = entry `id`), unchanged, as
/// `kh1-menu-sounds.bin` + `.json`, and the menu entries as WAVs for listening.
#[allow(clippy::type_complexity)]
fn menu_sounds_build(game: &Path, keys: kh1_formats::egs::KeyTable) -> Result<(Vec<u8>, String, Vec<(String, Vec<u8>)>), String> {
    use kh1_recom_assets::menu_sounds as ms;
    let inst = install::kh1(game, keys).map_err(|e| format!("KH1 files: {e}"))?;
    let (data, assets, _) = inst.read_full(ms::KH1_FILE).map_err(|e| format!("{}: {e}", ms::KH1_FILE))?;
    let mut notes = Vec::new();
    let taken = ms::take(&assets, &mut notes).map_err(|e| e.to_string())?;
    for l in ms::lines(&taken) {
        println!("{l}");
    }
    for n in &notes {
        println!("note: {n}");
    }
    if ms::present(&taken).is_empty() {
        return Err(format!("{} has no entry for any menu sound", taken.name));
    }
    let sources = [("se000.dat sha256", kh1_formats::egs::sha256_hex(&data))];
    let (bin, json) = ms::build(&taken, &sources);
    Ok((bin, json, ms::previews(&taken)))
}

fn kh1_stats(game: &std::path::Path, keys: kh1_formats::egs::KeyTable, arc: &kh1_formats::egs::Archive, models: &[KeybladeModel], kh2: &[Kh2Row]) -> Result<kh1_recom_assets::stats::StatsJson, String> {
    // Re:CoM's enemy stats files (`ENEMY/<id>/EN<id>.EPD`), for the proxies' ratios.
    let mut recom = |name: &str| match arc.find(name) {
        None => None,
        Some(_) => match install::read_recom(arc, &keys, name) {
            Ok(b) => Some(b),
            Err(e) => {
                println!("note: {name}: {e}");
                None
            }
        },
    };
    let inst = install::kh1(game, keys.clone()).map_err(|e| format!("KH1 files: {e}"))?;
    let btl = inst.read("btltbl.bin").map_err(|e| format!("btltbl.bin: {e}"))?;
    let parsed = kh1_formats::btltbl::Btltbl::parse(btl.clone()).map_err(|e| format!("btltbl.bin: {e}"))?;
    // The Kingdom Key (item 0x51): its weapon row's Strength (+0x30).
    let kk = parsed.weapon_for_item(0x51).map_err(|e| format!("Kingdom Key: {e}"))?.strength as i32;
    let mut read = |name: &str| inst.read(name).map_err(|e| kh1_recom_assets::Error::Format(format!("{name}: {e}")));
    kh1_recom_assets::stats::build_json(&mut read, &mut recom, &btl, kk, models, kh2).map_err(|e| e.to_string())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("wrote {} ({} bytes)", path.display(), bytes.len());
    Ok(())
}

/// Deletes `path` only if it is a pack this step wrote earlier (an `.ANN` with more than
/// Re:CoM's 161 slots; Re:CoM's own have 161 and 158); anything else is left alone.
fn remove_stale(path: &Path) -> Result<(), String> {
    let Ok(b) = std::fs::read(path) else { return Ok(()) };
    let ours = Ann::parse(&b).is_ok_and(|a| a.slots.len() > RECOM_SORA_SLOTS);
    if ours {
        std::fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("removed {} (an earlier extended pack; Re:CoM's own is used now)", path.display());
    } else {
        println!("note: {} exists and was not written by this step; left as it is", path.display());
    }
    Ok(())
}
