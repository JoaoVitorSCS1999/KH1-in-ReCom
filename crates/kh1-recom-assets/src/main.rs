//! `kh1-recom-build`: the asset build step on the command line. The step itself is the library's
//! [`kh1_recom_assets::step`] (the DLL runs the same code on first launch:
//! `docs/openkh-github-install.md`). It reads the player's own KH 1.5+2.5 folder (KH1's,
//! KH2 FM's and Re:CoM's archives) and writes the game-made files into Panacea's Re:CoM mod
//! folder (or next to `--out`): `docs/kh1-recom-assets.md` lists them.

use std::path::PathBuf;
use std::process::ExitCode;

use kh1_recom_assets::step::{self, Dest, Options};

const USAGE: &str = "usage: kh1-recom-build [--game <KH 1.5+2.5 folder>] [--mod-dir <Panacea mod_path>] [--out <file.ANN>]
                       [--root <folder>] [--loop E,E..] [--once E,E..] [--dry-run]

  --game     the game folder (holds Image/dt/Recom.hed and kh1_*.hed and the exes); default: .
  --mod-dir  Panacea's mod_path; default: mod_path= from <game>/panacea_settings.txt
  --out      write Sora's pack here (PL0002.ANN, the manifest, the stats and the Keyblade models'
             PLAYER/ and raw/PLAYER/ folders next to it) instead of
             <mod>/recom/PLAYER/000{1,2}/PL000{1,2}.ANN and <mod>/recom/kh1-motions.json
  --root     write everything under this folder as Panacea's <mod>/recom would hold it
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
    let mut opt = Options::new(PathBuf::from("."), Dest::ModDir(None));
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or_else(|| format!("{a} needs a value\n{USAGE}"));
        match a.as_str() {
            "--game" => opt.game = PathBuf::from(val()?),
            // --out (and --root) win over --mod-dir, whatever the order.
            "--mod-dir" => {
                let m = PathBuf::from(val()?);
                if matches!(opt.dest, Dest::ModDir(_)) {
                    opt.dest = Dest::ModDir(Some(m));
                }
            }
            "--out" => opt.dest = Dest::Out(PathBuf::from(val()?)),
            "--root" => opt.dest = Dest::Root(PathBuf::from(val()?)),
            "--loop" => opt.build.loop_overrides.extend(entries(&val()?)?.into_iter().map(|e| (e, true))),
            "--once" => opt.build.loop_overrides.extend(entries(&val()?)?.into_iter().map(|e| (e, false))),
            "--dry-run" => opt.dry_run = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            _ => return Err(format!("unknown argument '{a}'\n{USAGE}")),
        }
    }
    step::run(&opt).map(|_| ())
}
