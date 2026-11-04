//! M13 stage 2: KH1's own sounds as Re:CoM sound banks (`docs/recom-m13-plan.md` § Stage 2).
//!
//! The asset step (`kh1-recom-assets` `sounds`) copies KH1's one-sound SCDs (Sora's own and the
//! Kingdom Key's, `se0<id>.win32.scd`) unchanged into `kh1-sounds.bin` and writes the id map into
//! `kh1-sounds.json`: KH1 sound `id` = Re:CoM bank `9001 …`, played as Re:CoM id `bank × 1000`
//! (entry 0, as KH1 plays its one-sound SCDs, `0x1400DE460`). KH1 ids under 1000 are entries of
//! KH1's system bank, which the menu sounds give Re:CoM as bank 9000 (`crate::menu_sounds`). The
//! DLL (`win::sounds`) hands each SCD to Re:CoM's own bank registration `0x1402F5D70`, as the menu
//! sounds do; Re:CoM's own play path then plays them (`0x1402AA0F0` → `0x1402F3080`, the first
//! bank entry with that bank, `CreateSound(handle, id % 1000)`; Checked (decomp)).

use crate::json;

/// The manifest's `format` (`kh1-recom-assets` writes it).
pub const FORMAT: &str = "kh1-recom-assets sounds 1";
/// The manifest, next to the other asset-step manifests.
pub const MANIFEST: &str = "kh1-sounds.json";
/// The SCD pack, next to the manifest.
pub const PACK: &str = "kh1-sounds.bin";
/// KH1's system bank as Re:CoM's (the menu sounds' bank).
pub const SYSTEM_GROUP: u32 = crate::menu_sounds::GROUP;
/// The banks a KH1 SCD may get (inside the DLL's own map range 9000..9999, after the system bank).
pub const FIRST_BANK: u32 = 9001;
/// One past the last.
pub const BANK_END: u32 = crate::menu_sounds::GROUP_END;
/// Longest bank entry name Re:CoM keeps (`+0x30..+0x50`).
pub const NAME_MAX: usize = crate::menu_sounds::NAME_MAX;

/// One KH1 SCD as a Re:CoM bank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bank {
    /// KH1's sound id.
    pub kh1_id: u32,
    /// Re:CoM's bank (its id is `bank × 1000`).
    pub bank: u32,
    /// KH1's asset name, NUL-terminated (the bank entry's name).
    pub name: Vec<u8>,
    /// Where it is in the pack.
    pub offset: usize,
    /// Its size.
    pub size: usize,
    /// M13 stage 4: the Keyblade cards whose `.se` holds it (registered while one is equipped); empty
    /// for Sora's own (always registered; also an older manifest's rows).
    pub cards: Vec<u16>,
}

impl Bank {
    /// Whether it is wanted with Keyblade card `card` equipped.
    pub fn wanted_with(&self, card: u16) -> bool {
        self.cards.is_empty() || self.cards.contains(&card)
    }

    /// The name without its NUL.
    pub fn name_str(&self) -> String {
        String::from_utf8_lossy(&self.name[..self.name.len() - 1]).into_owned()
    }
}

/// What `kh1-sounds.json` lists.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    /// In KH1 id order.
    pub banks: Vec<Bank>,
    /// The pack's size the step wrote.
    pub pack_size: usize,
    /// The KH1 weapon sound base the step took (the Kingdom Key's).
    pub weapon_base: u32,
    /// Wanted ids the step found nowhere.
    pub missing: Vec<u32>,
}

impl Manifest {
    /// The Re:CoM id that plays KH1 sound `kh1_id`: a system-bank entry under 1000 as bank
    /// 9000's entry; an SCD the step took as its bank's entry 0; None otherwise.
    pub fn recom_id(&self, kh1_id: u32) -> Option<u32> {
        if kh1_id < 1000 {
            return Some(SYSTEM_GROUP * 1000 + kh1_id);
        }
        self.banks.iter().find(|b| b.kh1_id == kh1_id).map(|b| b.bank * 1000)
    }
}

/// Reads `kh1-sounds.json`; `pack_size` is the pack file's size.
pub fn parse(text: &str, pack_size: u64) -> Result<Manifest, String> {
    let v = json::parse(text)?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(format!("{MANIFEST}: not a \"{FORMAT}\" manifest (an older asset step? run it again)"));
    }
    if v.get("system_group").and_then(|x| x.as_u32()) != Some(SYSTEM_GROUP) {
        return Err(format!("{MANIFEST}: bank numbering is not this DLL's"));
    }
    let size = v.get("pack_size").and_then(|x| x.as_u32()).ok_or("no \"pack_size\"")? as usize;
    if size as u64 != pack_size {
        return Err(format!("{PACK} is {pack_size} bytes, not the manifest's {size} (an older pack?)"));
    }
    let rows = v.get("sounds").and_then(|s| s.as_array()).ok_or("no \"sounds\"")?;
    let mut banks: Vec<Bank> = Vec::with_capacity(rows.len());
    for r in rows {
        let n = |k: &str| r.get(k).and_then(|x| x.as_u32()).ok_or_else(|| format!("{MANIFEST}: a sound has no \"{k}\""));
        let (kh1_id, bank, offset, sz) = (n("kh1_id")?, n("bank")?, n("offset")? as usize, n("size")? as usize);
        let name = r.get("name").and_then(|x| x.as_str()).ok_or_else(|| format!("{MANIFEST}: sound {kh1_id} has no name"))?;
        if !(FIRST_BANK..BANK_END).contains(&bank) || banks.iter().any(|b| b.bank == bank || b.kh1_id == kh1_id) {
            return Err(format!("{MANIFEST}: sound {kh1_id} on bank {bank} (outside 9001..9999 or twice)"));
        }
        if kh1_id < 1000 || sz < 0x30 || offset.checked_add(sz).is_none_or(|e| e > size) {
            return Err(format!("{MANIFEST}: sound {kh1_id} ({offset:#x}+{sz:#x}) is not a KH1 SCD inside the pack"));
        }
        if name.is_empty() || name.len() > NAME_MAX || name.bytes().any(|c| c == 0 || !c.is_ascii()) {
            return Err(format!("{MANIFEST}: bad bank name {name:?}"));
        }
        let mut cname = name.as_bytes().to_vec();
        cname.push(0);
        let cards = r.get("cards").and_then(|c| c.as_array()).map(|a| a.iter().filter_map(|x| x.as_u32()).filter(|&x| x <= u16::MAX as u32).map(|x| x as u16).collect()).unwrap_or_default();
        banks.push(Bank { kh1_id, bank, name: cname, offset, size: sz, cards });
    }
    let missing = v.get("missing").and_then(|m| m.as_array()).map(|a| a.iter().filter_map(|x| x.as_u32()).collect()).unwrap_or_default();
    let weapon_base = v.get("weapon_base").and_then(|x| x.as_u32()).unwrap_or(0);
    Ok(Manifest { banks, pack_size: size, weapon_base, missing })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(rows: &[(u32, u32, &str, usize, usize)], size: usize) -> String {
        let r: Vec<String> = rows.iter().map(|(id, b, n, o, s)| format!("{{\"kh1_id\": {id}, \"bank\": {b}, \"recom_id\": {}, \"name\": {n:?}, \"offset\": {o}, \"size\": {s}, \"source\": \"xa_ex_0010.mdls\"}}", b * 1000)).collect();
        format!("{{\n  \"format\": \"{FORMAT}\",\n  \"pack\": \"{PACK}\",\n  \"pack_size\": {size},\n  \"system_group\": 9000,\n  \"weapon_base\": 11024,\n  \"sounds\": [\n{}\n  ],\n  \"missing\": [11024, 11025],\n  \"sources\": {{}}\n}}\n", r.join(",\n"))
    }

    #[test]
    fn reads_the_steps_manifest_and_maps_ids() {
        let m = parse(&manifest(&[(11001, 9001, "se011001.win32.scd", 0, 0x7720), (11023, 9002, "se011023.win32.scd", 0x7720, 0xD0F0)], 0x14810), 0x14810).unwrap();
        assert_eq!(m.banks.len(), 2);
        assert_eq!(m.banks[1].name_str(), "se011023.win32.scd");
        assert_eq!(m.recom_id(11001), Some(9_001_000));
        assert_eq!(m.recom_id(11023), Some(9_002_000));
        assert_eq!(m.recom_id(81), Some(9_000_081), "the effect sounds: KH1's system bank entries");
        assert_eq!(m.recom_id(11024), None);
        assert_eq!(m.missing, vec![11024, 11025]);
        assert_eq!(m.weapon_base, 11024);
        // Every bank is one the DLL's id → bank map gives its own bank.
        for b in &m.banks {
            assert_eq!(crate::menu_sounds::slot_of(b.bank * 1000), Some(b.bank as i32));
        }
    }

    /// The asset step's real output, when `KH1_SOUNDS_JSON` names it (its pack next to it): it
    /// parses, every SCD it lists starts with an SCD header, and the effects' `.ese` ids map.
    #[test]
    fn the_steps_output_parses() {
        let Some(p) = std::env::var_os("KH1_SOUNDS_JSON") else { return };
        let p = std::path::PathBuf::from(p);
        let pack = std::fs::read(p.with_file_name(PACK)).unwrap();
        let m = parse(&std::fs::read_to_string(&p).unwrap(), pack.len() as u64).unwrap();
        assert!(!m.banks.is_empty());
        for b in &m.banks {
            assert!(crate::menu_sounds::scd_head_ok(&pack[b.offset..b.offset + b.size], b.size), "{}", b.name_str());
        }
        eprintln!("{} banks {}..{}, {} missing; cards {:?}", m.banks.len(), m.banks[0].bank, m.banks.last().unwrap().bank, m.missing.len(), m.banks.iter().map(|b| (b.kh1_id, b.cards.clone())).collect::<Vec<_>>());
    }

    #[test]
    fn keyblade_banks_are_wanted_only_with_their_keyblade() {
        let j = manifest(&[(11001, 9001, "se011001.win32.scd", 0, 0x100), (11018, 9002, "se011018.win32.scd", 0x100, 0x100), (11424, 9003, "se011424.win32.scd", 0x200, 0x100)], 0x300)
            .replacen("\"source\": \"xa_ex_0010.mdls\"}", "\"source\": \"xa_ex_0010.mdls\", \"cards\": []}", 1)
            .replacen("\"source\": \"xa_ex_0010.mdls\"}", "\"source\": \"xw_ex_5010.se\", \"cards\": [1, 2]}", 1)
            .replacen("\"source\": \"xa_ex_0010.mdls\"}", "\"source\": \"xw_ex_5030.se\", \"cards\": [2]}", 1);
        let m = parse(&j, 0x300).unwrap();
        let with = |c: u16| m.banks.iter().filter(|b| b.wanted_with(c)).map(|b| b.kh1_id).collect::<Vec<_>>();
        assert_eq!(with(1), vec![11001, 11018]);
        assert_eq!(with(2), vec![11001, 11018, 11424]);
        assert_eq!(with(0x13), vec![11001], "card 0x13 alone: Sora's (a KH2 Keyblade plays the Kingdom Key's sounds: the DLL asks for card 1's)");
    }

    #[test]
    fn bad_manifests_are_refused() {
        let ok = |rows: &[(u32, u32, &str, usize, usize)], size: usize, file: u64| parse(&manifest(rows, size), file);
        assert!(ok(&[(11001, 9001, "se011001.win32.scd", 0, 0x100)], 0x100, 0x100).is_ok());
        assert!(ok(&[(11001, 9001, "se011001.win32.scd", 0, 0x100)], 0x100, 0x101).is_err(), "pack size");
        assert!(ok(&[(11001, 9000, "se011001.win32.scd", 0, 0x100)], 0x100, 0x100).is_err(), "the system bank's number");
        assert!(ok(&[(11001, 10_000, "se011001.win32.scd", 0, 0x100)], 0x100, 0x100).is_err(), "outside the map");
        assert!(ok(&[(11001, 9001, "a", 0, 0x100), (11002, 9001, "b", 0, 0x100)], 0x100, 0x100).is_err(), "a bank twice");
        assert!(ok(&[(11001, 9001, "a", 0x80, 0x100)], 0x100, 0x100).is_err(), "past the pack");
        assert!(ok(&[(81, 9001, "a", 0, 0x100)], 0x100, 0x100).is_err(), "system ids are not SCDs of their own");
        assert!(ok(&[(11001, 9001, "this-name-is-longer-than-31-bytes.scd", 0, 0x100)], 0x100, 0x100).is_err());
        assert!(parse(&manifest(&[], 0).replace("sounds 1", "sounds 0"), 0).is_err());
    }
}
