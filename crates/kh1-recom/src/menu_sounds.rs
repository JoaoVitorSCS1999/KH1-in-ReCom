//! KH1's command menu sounds in Re:CoM's own sound banks (owner, 2026-11-01: "Build a sound bank
//! from your KH1 files at the asset step and play KH1's exact menu sounds").
//!
//! KH1 plays menu sound `id` as entry `id` of its system bank, the one SCD `se000.win32.scd` of
//! `amusic/se000.dat` (bank 2: `0x1400DE460` → `CreateSound(handle, id, 0, 1, 0)`; Checked
//! (decomp + exe)). The asset step (`kh1-recom-assets` `menu_sounds`) copies that SCD unchanged
//! into `kh1-menu-sounds.bin`, with `kh1-menu-sounds.json`. The DLL (`win/menu_sounds.rs`):
//!
//! - gives the SCD to Re:CoM's own bank registration `0x1402F5D70` (what Re:CoM's loader does
//!   with a sound pack's PC assets) as bank `GROUP` (9000), the first time a menu sound plays;
//! - extends Re:CoM's id → bank map `0x1402A8560` with groups 9000..9999, each its own bank, as
//!   Re:CoM's own 200..599 are. Re:CoM gives groups ≥ 8000 no bank, so no Re:CoM id changes;
//!   Re:CoM puts its banks at 0–4, 0xC–0xF and 200–599 (a pack's `.seb`, `0x1402A8210`) and
//!   0xFFFF (voices, `0x1402F1250`), so none is ever put at 9000..9999 or released from there
//!   (Checked (decomp));
//! - plays KH1 sound `n` with Re:CoM's own menu-sound call `0x1402AA360(9000000 + n, −1)`: Re:CoM
//!   makes sound `id % 1000 = n` of the bank (`0x1402F3080`, the same `CreateSound(handle, n, 0,
//!   1, 0)` as KH1's) and plays it (`0x1402F3B60`), as it plays its own.
//!
//! Re:CoM's ids stay the fallback (no asset files, the Re:CoM-styled menu, a bank not there).
//! Notes: `docs/recom-m7-command-menu.md` § Sounds.

use crate::json;

/// The manifest's `format` (`kh1-recom-assets` writes it).
pub const FORMAT: &str = "kh1-recom-assets menu-sounds 2";
/// The manifest, next to the other asset-step manifests.
pub const MANIFEST: &str = "kh1-menu-sounds.json";
/// The SCD, next to the manifest.
pub const PACK: &str = "kh1-menu-sounds.bin";
/// Re:CoM's bank for KH1's system bank: KH1 sound `n` is Re:CoM id `GROUP × 1000 + n`.
pub const GROUP: u32 = 9000;
/// The group range our map gives its own bank (`GROUP` in it).
pub const GROUP_FIRST: u32 = 9000;
/// One past the last group of the range.
pub const GROUP_END: u32 = 10_000;
/// The KH1 menu sounds the DLL plays (`0x14017A040` ids: move, back, error, open).
pub const KH1_IDS: [u32; 4] = [1, 3, 4, 6];
/// Longest name Re:CoM's bank entry keeps (`+0x30..+0x50`).
pub const NAME_MAX: usize = 31;

/// KH1's system bank, ready for Re:CoM's bank registration (the SCD itself stays in the pack
/// file: the DLL reads it into Re:CoM's own buffer).
#[derive(Clone, Debug, PartialEq)]
pub struct Bank {
    /// KH1's asset name, NUL-terminated (Re:CoM keeps it as the bank entry's name).
    pub name: Vec<u8>,
    /// The bank number (= its id group).
    pub group: u32,
    /// The SCD's size (the pack's).
    pub size: usize,
    /// The KH1 menu ids it has an entry for.
    pub ids: Vec<u32>,
}

impl Bank {
    /// The name without its NUL.
    pub fn name_str(&self) -> String {
        String::from_utf8_lossy(&self.name[..self.name.len() - 1]).into_owned()
    }
}

/// Re:CoM's sound id for KH1 sound `kh1_id` (entry `kh1_id` of bank `GROUP`).
pub fn recom_id(kh1_id: u32) -> u32 {
    GROUP * 1000 + kh1_id
}

/// The bank our map gives an id (`0x1402A8560`'s own `id & 0x0FFFFFFF` / 1000 first), or None
/// for every id Re:CoM's own map decides.
pub fn slot_of(id: u32) -> Option<i32> {
    let g = (id & 0x0FFF_FFFF) / 1000;
    (GROUP_FIRST..GROUP_END).contains(&g).then_some(g as i32)
}

/// Checks the pack's head: an SCD (`SEDBSSCF`, little-endian, version 1–4) whose own size word fits
/// in the pack's `size` bytes (KH1's `se000.win32.scd` holds 1280 bytes past its size word: the
/// end of its last stream; KH1 loads the asset whole, and so does the DLL). Only the first 0x14
/// bytes are needed.
pub fn scd_head_ok(head: &[u8], size: usize) -> bool {
    let Some(h) = head.get(..0x14) else { return false };
    let version = u32::from_le_bytes([h[8], h[9], h[10], h[11]]);
    h.starts_with(b"SEDBSSCF") && (1..=4).contains(&version) && h[0xC] == 0 && (0x30..=size).contains(&(u32::from_le_bytes([h[0x10], h[0x11], h[0x12], h[0x13]]) as usize))
}

/// Reads `kh1-menu-sounds.json`; `pack_size` is the pack file's size, `head` its first bytes.
pub fn parse(manifest: &str, pack_size: u64, head: &[u8]) -> Result<Bank, String> {
    let v = json::parse(manifest)?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(format!("not a \"{FORMAT}\" manifest (an older asset step? run it again)"));
    }
    if v.get("group").and_then(|x| x.as_u32()) != Some(GROUP) {
        return Err("bank numbering is not this DLL's".into());
    }
    let size = v.get("pack_size").and_then(|x| x.as_u32()).ok_or("no \"pack_size\"")? as usize;
    if size as u64 != pack_size {
        return Err(format!("{PACK} is {pack_size} bytes, not the manifest's {size} (an older pack?)"));
    }
    if !scd_head_ok(head, size) {
        return Err(format!("{PACK} is not a whole SCD"));
    }
    let name = v.get("name").and_then(|x| x.as_str()).ok_or("no \"name\"")?;
    if name.is_empty() || name.len() > NAME_MAX || name.bytes().any(|c| c == 0 || !c.is_ascii()) {
        return Err(format!("bad bank name {name:?}"));
    }
    let rows = v.get("sounds").and_then(|s| s.as_array()).ok_or("no \"sounds\"")?;
    let mut ids = Vec::new();
    for r in rows {
        let n = |k: &str| r.get(k).and_then(|x| x.as_u32()).ok_or_else(|| format!("a sound has no \"{k}\""));
        let (id, entry, rid) = (n("kh1_id")?, n("entry")?, n("recom_id")?);
        // KH1 plays sound `id` as entry `id` (0x1400DE460).
        if !KH1_IDS.contains(&id) || entry != id || rid != recom_id(id) || ids.contains(&id) {
            return Err(format!("unexpected sound {id} (entry {entry}, id {rid})"));
        }
        ids.push(id);
    }
    let mut cname = name.as_bytes().to_vec();
    cname.push(0);
    Ok(Bank { name: cname, group: GROUP, size, ids })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(n: usize) -> Vec<u8> {
        let mut d = vec![0u8; 0x30];
        d[..8].copy_from_slice(b"SEDBSSCF");
        d[8] = 3;
        d[0x10..0x14].copy_from_slice(&(n as u32).to_le_bytes());
        d
    }

    fn manifest(rows: &[(u32, u32, u32)], size: usize, name: &str) -> String {
        let r: Vec<String> = rows.iter().map(|(id, e, rid)| format!("{{\"kh1_id\": {id}, \"use\": \"x\", \"entry\": {e}, \"recom_id\": {rid}, \"stream\": null}}")).collect();
        format!("{{\"format\": \"{FORMAT}\", \"pack\": \"{PACK}\", \"pack_size\": {size}, \"group\": 9000, \"name\": {name:?}, \"sounds\": [{}], \"sources\": {{}}}}", r.join(", "))
    }

    #[test]
    fn ids_map_to_entries_of_bank_9000_and_nothing_of_recoms_moves() {
        assert_eq!(recom_id(1), 9_000_001);
        assert_eq!(recom_id(6), 9_000_006);
        assert_eq!(slot_of(recom_id(1)), Some(9000));
        assert_eq!(recom_id(6) % 1000, 6, "Re:CoM makes sound id % 1000 of the bank");
        // The stream bit is masked first, as Re:CoM's map does.
        assert_eq!(slot_of(recom_id(3) | 0x1000_0000), Some(9000));
        // Every group Re:CoM's map decides (0..7999 and the -1 above) stays Re:CoM's.
        for id in [0u32, 1, 2, 4, 5, 0xE, 110_014, 260_040, 271_043, 601_001, 2_005_000, 7_999_999, 8_000_000, 8_999_999, 10_000_000] {
            assert_eq!(slot_of(id), None, "{id}");
        }
    }

    #[test]
    fn manifest_gives_the_bank() {
        let m = manifest(&[(1, 1, 9_000_001), (3, 3, 9_000_003), (4, 4, 9_000_004), (6, 6, 9_000_006)], 0x1000, "se000.win32.scd");
        let b = parse(&m, 0x1000, &head(0x1000)).unwrap();
        assert_eq!((b.name.as_slice(), b.group, b.size, b.ids.clone()), (b"se000.win32.scd\0".as_slice(), 9000, 0x1000, vec![1, 3, 4, 6]));
        assert_eq!(b.name_str(), "se000.win32.scd");
    }

    #[test]
    fn bad_manifests_are_refused() {
        let ok = |rows: &[(u32, u32, u32)], size: usize, file: u64, h: &[u8], name: &str| parse(&manifest(rows, size, name), file, h);
        let h = head(0x1000);
        assert!(ok(&[(1, 1, 9_000_001)], 0x1000, 0x1000, &h, "se000.win32.scd").is_ok());
        // Pack size, SCD head, entry ≠ id, wrong Re:CoM id, unknown id, duplicate, name.
        assert!(ok(&[(1, 1, 9_000_001)], 0x1000, 0x1001, &h, "se000.win32.scd").is_err());
        assert!(ok(&[(1, 1, 9_000_001)], 0x1000, 0x1000, &head(0x2000), "se000.win32.scd").is_err());
        assert!(ok(&[(1, 2, 9_000_001)], 0x1000, 0x1000, &h, "se000.win32.scd").is_err());
        assert!(ok(&[(1, 1, 9_001_000)], 0x1000, 0x1000, &h, "se000.win32.scd").is_err());
        assert!(ok(&[(2, 2, 9_000_002)], 0x1000, 0x1000, &h, "se000.win32.scd").is_err());
        assert!(ok(&[(1, 1, 9_000_001), (1, 1, 9_000_001)], 0x1000, 0x1000, &h, "se000.win32.scd").is_err());
        assert!(ok(&[(1, 1, 9_000_001)], 0x1000, 0x1000, &h, "this-name-is-longer-than-31-bytes.win32.scd").is_err());
        // The 0.39.0 format (one SCD per sound) is refused.
        assert!(parse(&manifest(&[], 0x1000, "x").replace("menu-sounds 2", "menu-sounds 1"), 0x1000, &h).is_err());
        let mut v9 = h.clone();
        v9[8] = 9;
        assert!(!scd_head_ok(&v9, 0x1000));
        // A pack longer than its size word (KH1's se000) passes; a shorter one does not.
        assert!(scd_head_ok(&head(0x800), 0x1000) && !scd_head_ok(&head(0x2000), 0x1000));
    }
}
