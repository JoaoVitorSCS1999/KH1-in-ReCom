//! Build script: `KH1_ASSET_STEP_HASH`, a hash of the asset step's own sources (this crate's and
//! the format crates' `src/`). The DLL's first-launch build stamps its files with it, so any
//! change to the step makes the DLL build again (`src/stamp.rs`, `docs/openkh-github-install.md`).
//! Line ends are dropped (`\r`), so a Windows checkout hashes as the CI's does.

use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn main() {
    let here = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let crates = here.parent().unwrap().to_path_buf();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |b: &[u8]| {
        for &x in b {
            if x != b'\r' {
                h ^= x as u64;
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
    };
    for name in ["kh1-recom-assets", "kh1-formats", "kh1-anim", "kh2-formats-lite"] {
        let src = crates.join(name).join("src");
        println!("cargo:rerun-if-changed={}", src.display());
        let mut list = Vec::new();
        files(&src, &mut list);
        list.sort();
        for p in list {
            let rel = p.strip_prefix(&crates).unwrap().to_string_lossy().replace('\\', "/");
            eat(rel.as_bytes());
            eat(&[0]);
            eat(&std::fs::read(&p).unwrap());
            eat(&[0]);
        }
    }
    println!("cargo:rustc-env=KH1_ASSET_STEP_HASH={h:016x}");
}
