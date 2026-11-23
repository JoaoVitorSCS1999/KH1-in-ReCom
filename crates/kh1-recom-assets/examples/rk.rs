use kh1_recom_assets::install;
fn main() {
    let g = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let out = std::path::PathBuf::from(std::env::args().nth(2).unwrap());
    let keys = install::keys(&g).unwrap();
    let inst = install::kh1(&g, keys).unwrap();
    for base in ["xa_ex_1010", "xa_ex_1011", "xa_ex_1560", "xa_ex_1568"] {
        for ext in ["mset", "mdls"] {
            let n = format!("{base}.{ext}");
            match inst.read(&n) { Ok(d) => { std::fs::write(out.join(&n), &d).unwrap(); println!("{n} {}", d.len()); } Err(e) => println!("{n}: {e}") }
        }
        if let Ok(d) = inst.read(&format!("{base}.mset")) {
            let ms = kh1_formats::mset::MotionSet::parse(&d).unwrap();
            let mut reqs = Vec::new();
            for id in 0..400u16 { if let Some((m, mo)) = ms.motion_for(id) { reqs.push(format!("{id}->e{:?}:{}f", m.entry(), mo.frames)); } }
            println!("  {base} requests: {}", reqs.join(" "));
        }
    }
}
