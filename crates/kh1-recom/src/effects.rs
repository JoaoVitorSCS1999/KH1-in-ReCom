//! M13 stage 1: KH1's effect sets in Re:CoM's own effect container (`docs/recom-m13-plan.md`).
//!
//! The asset step (`kh1-recom-assets` `effects`) adds KH1's Sora set to Re:CoM's `EF1000.EFF`
//! as `kh1_ex_0010.dpx`; Re:CoM loads it with Sora's group as its own resource. Nothing plays it
//! yet (stage 3 writes the per-move events). The DLL only checks, once, that Re:CoM has it
//! (`win::effects`): the manifest's set names, looked up by Re:CoM's own effect-by-name.

use crate::json;

/// The manifest's `format` (`kh1-recom-assets` writes it).
pub const FORMAT: &str = "kh1-recom-assets effects 1";
/// The manifest, next to the other asset-step manifests.
pub const MANIFEST: &str = "kh1-effects.json";

/// One set the step added: its name in Re:CoM and its effect ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Set {
    pub name: String,
    pub ids: Vec<u32>,
}

/// The sets a manifest lists.
pub fn parse(text: &str) -> Result<Vec<Set>, String> {
    let v = json::parse(text)?;
    match v.get("format").and_then(|f| f.as_str()) {
        Some(FORMAT) => {}
        other => return Err(format!("{MANIFEST}: format {other:?}, expected {FORMAT:?} (run the asset step again)")),
    }
    let sets = v.get("sets").and_then(|s| s.as_array()).ok_or_else(|| format!("{MANIFEST}: no sets"))?;
    sets.iter()
        .map(|s| {
            let name = s.get("name").and_then(|n| n.as_str()).ok_or_else(|| format!("{MANIFEST}: a set without a name"))?;
            if name.is_empty() || name.len() > 21 || !name.is_ascii() {
                return Err(format!("{MANIFEST}: set name {name:?}"));
            }
            let ids = s.get("ids").and_then(|i| i.as_array()).map(|a| a.iter().filter_map(|x| x.as_u32()).collect()).unwrap_or_default();
            Ok(Set { name: name.to_string(), ids })
        })
        .collect()
}

/// The name Re:CoM's effect-by-name (`0x1402A4920`) is given: the set's name as a C string.
pub fn lookup_name(set: &str) -> Vec<u8> {
    let mut b = set.as_bytes().to_vec();
    b.push(0);
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_steps_manifest() {
        let j = "{\n  \"format\": \"kh1-recom-assets effects 1\",\n  \"container\": \"EFFECT/1000/EF1000.EFF\",\n  \"sets\": [\n    {\"name\": \"kh1_ex_0010.dpx\", \"source\": \"xa_ex_0010.mdls section 1\", \"bytes\": 126592, \"ids\": [0, 1, 2, 3, 4, 5, 6, 7], \"hd\": [\"kh1_ex_0010_dpx80.dds\"]}\n  ],\n  \"sources\": {\n    \"x\": \"y\"\n  }\n}\n";
        let s = parse(j).unwrap();
        assert_eq!(s, vec![Set { name: "kh1_ex_0010.dpx".into(), ids: (0..8).collect() }]);
        assert!(parse(&j.replace("effects 1", "effects 0")).is_err(), "an older step's manifest");
        assert_eq!(lookup_name("kh1_ex_0010.dpx").last(), Some(&0));
    }
}
