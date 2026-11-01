//! KH1's command menu sounds in Re:CoM's own sound banks (owner, 2026-11-01: "Build a sound bank
//! from your KH1 files at the asset step and play KH1's exact menu sounds").
//!
//! The asset step (`kh1-recom-assets` `menu_sounds`) copies KH1's own SCDs for menu sounds 1 / 3 /
//! 4 / 6 (move, back, error, open) from the player's `amusic/se000.dat` into
//! `kh1-menu-sounds.bin`, with `kh1-menu-sounds.json`. The DLL (`win/menu_sounds.rs`):
//!
//! - gives each SCD to Re:CoM's own bank registration `0x1402F5D70` (what Re:CoM's loader does
//!   with a sound pack's PC assets) as bank `GROUP_BASE + KH1 id`, the first time it is played;
//! - extends Re:CoM's id → bank map `0x1402A8560` with that one range: groups 9000..9999 are
//!   their own bank, as Re:CoM's own 200..599 are. Re:CoM gives groups ≥ 8000 no bank, so no
//!   Re:CoM id changes; Re:CoM puts its banks at 0–4, 0xC–0xF and 200–599 (a pack's `.seb`,
//!   `0x1402A8210`) and 0xFFFF (voices, `0x1402F1250`), so none is ever put at 9000..9999 or
//!   released from there (Checked (decomp));
//! - plays KH1 sound `n` with Re:CoM's own menu-sound call `0x1402AA360((9000 + n) × 1000, −1)`:
//!   Re:CoM makes sound 0 of the bank (`0x1402F3080`) and plays it (`0x1402F3B60`), as it plays
//!   its own.
//!
//! Re:CoM's ids stay the fallback (no asset files, the Re:CoM-styled menu, a bank not there).
//! Notes: `docs/recom-m7-command-menu.md` § Sounds.

use crate::json;

/// The manifest's `format` (`kh1-recom-assets` writes it).
pub const FORMAT: &str = "kh1-recom-assets menu-sounds 1";
/// The manifest, next to the other asset-step manifests.
pub const MANIFEST: &str = "kh1-menu-sounds.json";
/// The SCDs, next to the manifest.
pub const PACK: &str = "kh1-menu-sounds.bin";
/// Re:CoM groups for KH1's sounds: KH1 sound `n` is group `GROUP_BASE + n`, bank the same number.
pub const GROUP_BASE: u32 = 9000;
/// One past the last group of the range.
pub const GROUP_END: u32 = 10_000;
/// The KH1 menu sounds the DLL takes (`0x14017A040` ids: move, back, error, open).
pub const KH1_IDS: [u32; 4] = [1, 3, 4, 6];
/// Longest name Re:CoM's bank entry keeps (`+0x30..+0x50`).
pub const NAME_MAX: usize = 31;

/// One KH1 sound, ready for Re:CoM's bank registration.
#[derive(Clone, Debug, PartialEq)]
pub struct Bank {
    pub kh1_id: u32,
    /// KH1's asset name, NUL-terminated (Re:CoM keeps it as the bank entry's name).
    pub name: Vec<u8>,
    /// The bank number (= its id group).
    pub group: u32,
    /// The SCD, unchanged.
    pub data: Vec<u8>,
}

impl Bank {
    /// The name without its NUL.
    pub fn name_str(&self) -> String {
        String::from_utf8_lossy(&self.name[..self.name.len() - 1]).into_owned()
    }
}

/// Re:CoM's sound id for KH1 sound `kh1_id` (sound 0 of its bank).
pub fn recom_id(kh1_id: u32) -> u32 {
    (GROUP_BASE + kh1_id) * 1000
}

/// The bank our map gives an id (`0x1402A8560`'s own `id & 0x0FFFFFFF` / 1000 first), or None
/// for every id Re:CoM's own map decides.
pub fn slot_of(id: u32) -> Option<i32> {
    let g = (id & 0x0FFF_FFFF) / 1000;
    (GROUP_BASE..GROUP_END).contains(&g).then_some(g as i32)
}

/// Reads `kh1-menu-sounds.json` and its pack.
pub fn parse(manifest: &str, pack: &[u8]) -> Result<Vec<Bank>, String> {
    let v = json::parse(manifest)?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(format!("not a \"{FORMAT}\" manifest"));
    }
    if v.get("group_base").and_then(|x| x.as_u32()) != Some(GROUP_BASE) {
        return Err("group numbering is not this DLL's".into());
    }
    if v.get("pack_size").and_then(|x| x.as_u32()) != Some(pack.len() as u32) {
        return Err(format!("{PACK} is {} bytes, not the manifest's (an older pack?)", pack.len()));
    }
    let rows = v.get("sounds").and_then(|s| s.as_array()).ok_or("no \"sounds\"")?;
    let mut out: Vec<Bank> = Vec::new();
    for r in rows {
        let n = |k: &str| r.get(k).and_then(|x| x.as_u32()).ok_or_else(|| format!("a sound has no \"{k}\""));
        let (id, group, at, size) = (n("kh1_id")?, n("group")?, n("offset")? as usize, n("size")? as usize);
        let name = r.get("name").and_then(|x| x.as_str()).ok_or("a sound has no \"name\"")?;
        if !KH1_IDS.contains(&id) || group != GROUP_BASE + id || out.iter().any(|b| b.kh1_id == id) {
            return Err(format!("unexpected sound {id} (group {group})"));
        }
        if name.is_empty() || name.len() > NAME_MAX || name.bytes().any(|c| c == 0 || !c.is_ascii()) {
            return Err(format!("sound {id}: bad name {name:?}"));
        }
        let data = pack.get(at..at.checked_add(size).ok_or("bad size")?).ok_or_else(|| format!("sound {id}: {size} bytes at {at} run past {PACK}"))?;
        // An SCD whose own size word is its size (both drivers read it so).
        let own = data.get(0x10..0x14).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
        if !data.starts_with(b"SEDBSSCF") || own != Some(size) {
            return Err(format!("sound {id}: not a whole SCD"));
        }
        let mut cname = name.as_bytes().to_vec();
        cname.push(0);
        out.push(Bank { kh1_id: id, name: cname, group, data: data.to_vec() });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scd(n: usize) -> Vec<u8> {
        let mut d = vec![0u8; n];
        d[..8].copy_from_slice(b"SEDBSSCF");
        d[0x10..0x14].copy_from_slice(&(n as u32).to_le_bytes());
        d
    }

    fn manifest(rows: &[(u32, &str, u32, usize, usize)], pack: usize) -> String {
        let r: Vec<String> = rows.iter().map(|(id, name, g, at, size)| format!("{{\"kh1_id\": {id}, \"use\": \"x\", \"name\": {name:?}, \"group\": {g}, \"offset\": {at}, \"size\": {size}}}")).collect();
        format!("{{\"format\": \"{FORMAT}\", \"pack\": \"{PACK}\", \"pack_size\": {pack}, \"group_base\": 9000, \"sounds\": [{}], \"sources\": {{}}}}", r.join(", "))
    }

    #[test]
    fn ids_map_to_their_own_banks_and_nothing_of_recoms_moves() {
        assert_eq!(recom_id(1), 9_001_000);
        assert_eq!(recom_id(6), 9_006_000);
        assert_eq!(slot_of(recom_id(1)), Some(9001));
        assert_eq!(slot_of(recom_id(6) + 999), Some(9006));
        // The stream bit is masked first, as Re:CoM's map does.
        assert_eq!(slot_of(recom_id(3) | 0x1000_0000), Some(9003));
        // Every group Re:CoM's map decides (0..7999 and the -1 above) stays Re:CoM's.
        for id in [0u32, 1, 2, 4, 5, 0xE, 110_014, 260_040, 271_043, 601_001, 2_005_000, 7_999_999, 8_000_000, 8_999_999, 10_000_000] {
            assert_eq!(slot_of(id), None, "{id}");
        }
    }

    #[test]
    fn manifest_and_pack_give_the_banks() {
        let mut pack = scd(0x40);
        pack.extend(scd(0x30));
        let m = manifest(&[(1, "se000001.win32.scd", 9001, 0, 0x40), (6, "se000006.win32.scd", 9006, 0x40, 0x30)], pack.len());
        let b = parse(&m, &pack).unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!((b[1].kh1_id, b[1].group, b[1].name.as_slice(), b[1].data.len()), (6, 9006, b"se000006.win32.scd\0".as_slice(), 0x30));
        assert_eq!(b[0].name_str(), "se000001.win32.scd");
    }

    #[test]
    fn bad_manifests_are_refused() {
        let pack = scd(0x40);
        let ok = |rows: &[(u32, &str, u32, usize, usize)], n: usize| parse(&manifest(rows, n), &pack);
        assert!(ok(&[(1, "se000001.win32.scd", 9001, 0, 0x40)], 0x40).is_ok());
        // Pack size, range, group, id, duplicate, name, SCD.
        assert!(ok(&[(1, "se000001.win32.scd", 9001, 0, 0x40)], 0x41).is_err());
        assert!(ok(&[(1, "se000001.win32.scd", 9001, 8, 0x40)], 0x40).is_err());
        assert!(ok(&[(1, "se000001.win32.scd", 9002, 0, 0x40)], 0x40).is_err());
        assert!(ok(&[(2, "se000002.win32.scd", 9002, 0, 0x40)], 0x40).is_err());
        assert!(ok(&[(1, "a", 9001, 0, 0x40), (1, "b", 9001, 0, 0x40)], 0x40).is_err());
        assert!(ok(&[(1, "this-name-is-longer-than-31-bytes.win32.scd", 9001, 0, 0x40)], 0x40).is_err());
        assert!(ok(&[(1, "se000001.win32.scd", 9001, 0, 0x20)], 0x40).is_err(), "the SCD's own size word");
        assert!(parse(&manifest(&[], 0).replace("menu-sounds 1", "menu-sounds 0"), &[]).is_err());
    }
}
