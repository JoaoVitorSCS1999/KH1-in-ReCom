//! Riku in Reverse/Rebirth with KH1's movement (testing, fork branch `Re-Re-Reverse/Rebirth`;
//! ini `riku_kh1_movement`, default off). `docs/recom-riku-kh1.md`.
//!
//! What lets KH1's movement run on Riku natively:
//! - Re:CoM's per-character action tables (`0x1407C6300[player+0x398]`, 16-byte rows
//!   `{name, index}`): characters 3 / 4 (Riku) point at `0x1407C6370`, whose rows 0x00..0x38 mean
//!   what Sora's (`0x1407C5E80`) mean: idle, walk, run, jump, fall, land, roll, hit reactions,
//!   swings. Checked (exe: both tables read row by row).
//! - Riku's battle exe (`CPlayerRiku` vtable `0x1405A8868` `+0xB8` = `0x140216B10`) is
//!   `jmp 0x1401F9840`, Re:CoM's shared Battle exe; his Field exe is the shared `0x1401FA0F0`.
//!   So his actions run through the same switch and the same move / jump handlers as Sora's.
//!   Checked (exe: vtables and disasm).
//! - His own handlers: the roll exe (`+0x1C8` = `0x140212350`, Sora's is `0x140217310`); no glide
//!   (`+0x1A8` / `+0x1B0` are `CPlayer`'s defaults). Checked (exe).
//! - His clips (`PLAYER/0101/PL0101.ANN`, 51) sit on KH1's Riku skeleton: `PL0101.MDL` bone 1 is
//!   `ex_1010_sk` (KH1 `xa_ex_1010`), and 9 clips are exact bakes of KH1 Riku motions (all hit
//!   reactions, idle-turns). Checked (files).

/// Riku's action-table rows with no clip of his: their name is `p002_000`, which his pack
/// (`PL0101.ANN`, first slot `p002_001`) does not hold. Rows 0x1B..0x1E are Sora's glide rows,
/// 0x39..0x46 Sora's casts, item, and KH1-move bakes. Checked (exe: `0x1407C6370`; files).
pub const NO_CLIP: &[u32] = &[0x00, 0x1B, 0x1C, 0x1D, 0x1E];

/// The first row past Riku's own: 0x39..0x46 are `p002_000`, and our appended KH1 rows (0x47..)
/// are only in Sora's repointed table, not Riku's.
pub const ROW_END: u32 = 0x39;

/// Whether Riku's action table plays a clip of his on `row`. Anything else would start a motion
/// with no clip (a T-pose), so the KH1 frame keeps the current motion instead.
pub fn row_ok(row: u32) -> bool {
    row < ROW_END && !NO_CLIP.contains(&row)
}

/// Dark Riku's action-table rows with no clip (`0x1407C67F0`: `p003_000`; his pack `PL0201.ANN`
/// starts at `p003_001`): 0, 8..=0xD, 0x17..=0x1A, 0x1C..=0x1E, 0x39.. Row 0x1B (Sora's glide row)
/// has a clip of his, never played (no glide). Checked (exe).
pub const DARK_NO_CLIP: &[u32] = &[0x00, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E];

/// [`row_ok`] for Riku (`dark` false) or Dark Riku (0.152.0).
pub fn row_ok_for(dark: bool, row: u32) -> bool {
    if dark {
        row < ROW_END && !DARK_NO_CLIP.contains(&row)
    } else {
        row_ok(row)
    }
}

/// Riku's ability word bit for Re:CoM's higher jump (`+0x41C & 0x200000`: the jump init
/// `0x1401F29F0` reads it; `crate::abilities::recom`).
pub const HIGH_JUMP_BIT: u32 = 0x20_0000;

/// The KH1 movement abilities Riku has: Dodge Roll (always Re:CoM's own roll, his roll exe), and
/// High Jump when Re:CoM gave him its higher jump. No Glide (Riku has none in Re:CoM), no
/// Counterattack (no clip), no combo abilities (KH1's plain combo on his own swings).
///
/// Guard (0.152.0; fork owner, 2026-11-22: "falta a defesa"): KH1's Guard rule (Square below run
/// speed, the guard object live on KH1's frames 10..36, the kind-6 recoil) on his own block-recoil
/// clip, action row 0x24 ([`GUARD_ROW`]). KH1's Riku (`xa_ex_1010.mset`) has no Guard request (212):
/// his request 110 / 111 (entry 13) is the clash recoil Re:CoM baked as row 0x24 for him too.
/// Checked (files: both motion sets' request tables).
pub fn abilities(ability_word: u32) -> kh1_core::Abilities {
    kh1_core::Abilities { high_jump: ability_word & HIGH_JUMP_BIT != 0, dodge_roll: true, guard: true, counterattack: true, ..Default::default() }
}

/// 0.159.0 (fork owner: "Falta o contra-ataque"): KH1's Counterattack (row 0x3E, after a guard
/// success) for Riku, who has no clip of it: the row it plays is his ground finisher's (KH1 row 8,
/// his own finisher clip); KH1's offer, input and timing are Sora's. Assumption: his finisher clip
/// reads as the counter.
pub const COUNTER_AS_ROW: usize = 8;

/// The KH1 row whose Re:CoM row Riku plays for `kh1_row`.
pub fn swing_row(kh1_row: usize) -> usize {
    if kh1_row == crate::attack::COUNTERATTACK_ROW {
        COUNTER_AS_ROW
    } else {
        kh1_row
    }
}

/// Re:CoM's body-guard angle rule (`0x14016F1C0`'s guard branch; Checked (exe: disasm)): the
/// attack point minus the guarder's bone 0 (`0x14023E6E0`), `atan2f(d.x, d.z)` minus the guarder's
/// yaw (its mover `+0x54`, `0x1401A4A00`), wrapped to ±π (`0x14016EAA0`), within `half` (KH1's
/// Guard: ±100°, `KH1_GUARD_ANGLE`). The facing test of the KH1 Guard's blade (0.182.0 Riku's,
/// 0.183.3 Sora's too).
pub fn guard_faces(guarder: [f32; 3], yaw: f32, point: [f32; 3], half: f32) -> bool {
    let (dx, dz) = (point[0] - guarder[0], point[2] - guarder[2]);
    if dx == 0.0 && dz == 0.0 {
        return true;
    }
    let pi = std::f32::consts::PI;
    let mut a = dx.atan2(dz) - yaw;
    while a > pi {
        a -= 2.0 * pi;
    }
    while a < -pi {
        a += 2.0 * pi;
    }
    a.abs() <= half
}

/// The point Re:CoM's weapon volume puts at the blade's end, in the weapon bone's frame
/// (`0x140225840`: `0x1407C7E50` = (−0.8, 0, 0, 1); Checked (exe + decomp)). The Wide modifier's
/// −1.2 is left out: KH1's Guard has no Wide.
pub const BLADE_END: [f32; 3] = [-0.8, 0.0, 0.0];

/// The player's blade for the KH1 Guard (0.182.0 Riku's, 0.183.3 Sora's too): (end, grip) in world
/// metres, from the rows of the bone Re:CoM puts its weapon volume on (`bone + 0xA0`, row 3 its origin).
/// The end is [`BLADE_END`] in that bone's frame, as Re:CoM places it (`0x1402B4A30`: x·row0 + y·row1 +
/// z·row2 + row3; Checked (decomp)). The grip is the bone's origin (Assumption: the bone sits at the hilt).
pub fn guard_blade(rows: [kh1_core::Vec3; 4]) -> (kh1_core::Vec3, kh1_core::Vec3) {
    let end = rows[0] * BLADE_END[0] + rows[1] * BLADE_END[1] + rows[2] * BLADE_END[2] + rows[3];
    (end, rows[3])
}

/// Riku's / Dark Riku's row for KH1's Guard request: their recoil row (`p002_043` / `p003_033`).
pub const GUARD_ROW: u32 = 0x24;

/// 0.157.0: the row Riku's Guard plays when the asset step gave his pack Sora's KH1 Guard
/// (`p002_225`, `kh1_recom_assets::riku_skin::RIKU_GUARD`): his first row with no clip
/// (`p002_000`, 0x39; Re:CoM never plays it for him), renamed to the guard clip in his action
/// tables (`win::riku_skin::guard_rows`). The guard's recoil stays row 0x24 (his clash recoil).
pub const KH1_GUARD_ROW: u32 = 0x39;
/// The guard clip's name, as the asset step writes it.
pub const KH1_GUARD_CLIP: &[u8] = b"p002_225\0";
/// 0.158.2: KH1 Riku's own guard recoil (request 110, entry 13; fork owner: "prefiro que use a
/// versão do KH1 da animação de quando toma impacto"), his next row with no clip (0x3A), renamed
/// like 0x39; played for KH1's guard recoils 0x6E / 0x6F.
pub const KH1_RECOIL_ROW: u32 = 0x3A;
pub const KH1_RECOIL_CLIP: &[u8] = b"p002_226\0";
/// 0.183.2: Dark Riku's (the real Dark Mode's) guard and its recoil, the same clips on his skeleton in his
/// own pack (`kh1_recom_assets::riku_skin::{DARK_GUARD, DARK_RECOIL}`), on the same rows of his table.
pub const DARK_GUARD_CLIP: &[u8] = b"p003_225\0";
/// Dark Riku's guard recoil clip (see [`DARK_GUARD_CLIP`]).
pub const DARK_RECOIL_CLIP: &[u8] = b"p003_226\0";

/// 0.184.1: Riku's / Dark Riku's cast rows (the next clipless rows of both tables, `p002_000` / `p003_000` up to 0x46;
/// Checked (exe)), named after the asset step's cast clips (`kh1_recom_assets::riku_skin::CAST_CLIPS`). 0.185.0 (DDD's
/// casts, ground and air apart): on the ground Fire's wind-up, Fire's release, Blizzard, Thunder / Aero, Cure, Gravity /
/// Stop; then the same six in the air.
pub const CAST_ROWS: [u32; 12] = [0x3B, 0x3C, 0x3D, 0x3E, 0x3F, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46];
/// Their clip names for Riku and for Dark Riku.
pub const CAST_CLIPS: [&[u8]; 12] = [b"p002_227\0", b"p002_228\0", b"p002_229\0", b"p002_230\0", b"p002_231\0", b"p002_232\0", b"p002_233\0", b"p002_234\0", b"p002_235\0", b"p002_236\0", b"p002_237\0", b"p002_238\0"];
pub const DARK_CAST_CLIPS: [&[u8]; 12] = [b"p003_227\0", b"p003_228\0", b"p003_229\0", b"p003_230\0", b"p003_231\0", b"p003_232\0", b"p003_233\0", b"p003_234\0", b"p003_235\0", b"p003_236\0", b"p003_237\0", b"p003_238\0"];

/// 0.184.6 (fork owner, 2026-11-23: the cast motions "flicando"; "ajusta tbm a animação de guard pra ficar mais suave
/// quando terminar"): the blends (KH1 frames) Re:CoM's motion request gets for Riku's own KH1 rows, which start and
/// end in poses of his sleights or KH1 Riku's, not his idle: a cast's first motion blends in over [`CAST_IN`] frames,
/// and leaving any of them (a cast clip, the Guard, its recoil) for another row blends out over [`OUT`]. A visual
/// blend only: KH1's frame keeps its own timing. Assumption: the numbers (Re:CoM's own swing ends blend 8).
pub const CAST_IN: f32 = 6.0;
pub const OUT: f32 = 10.0;

/// One of Riku's KH1-only rows (the cast clips, the Guard, its recoil).
pub fn kh1_only_row(row: u32) -> bool {
    CAST_ROWS.contains(&row) || row == KH1_GUARD_ROW || row == KH1_RECOIL_ROW
}

/// The blend for Riku's motion request from Re:CoM's current row `cur` to `new` (`first_cast`: the request is a cast's
/// first motion, `crate::magic::cast_req::first`).
pub fn blend_for(cur: u32, new: u32, first_cast: bool, blend: f32) -> f32 {
    if first_cast && CAST_ROWS.contains(&new) && !kh1_only_row(cur) {
        blend.max(CAST_IN)
    } else if kh1_only_row(cur) && !kh1_only_row(new) {
        blend.max(OUT)
    } else {
        blend
    }
}

/// 0.198.13 (fork owner, on the cast clips: "tem mudanças muito bruscas entre a animação e voltar para o estado idle"; "magia é o
/// riku normal e o dark mode"): the idle row after one of his cast rows (`cur` or `prev`) is the battle stance 0x2A, field or
/// battle, as after a swing (`crate::rows::Stance::idle_battle`): the asset step eases each cast clip's end into the stance
/// (`kh1_recom_assets::riku_skin::ease_cast_ends`), and the field idle (row 1) is 86 cm from it. Other rows: `row` as given.
pub fn idle_after_cast(cur: i32, prev: i32, row: u32) -> u32 {
    let cast = |r: i32| r >= 0 && CAST_ROWS.contains(&(r as u32));
    if row == crate::rows::row::IDLE && (cast(cur) || cast(prev)) {
        crate::rows::row::STANCE_IDLE
    } else {
        row
    }
}

/// The cast row for a KH1 cast request (ground and air share a row; `crate::magic::cast_req`), None for anything
/// else.
pub fn cast_row(request: u16) -> Option<u32> {
    let i = match request {
        0x36 => 0,
        0x3D => 1,
        0x37 => 2,
        0x38 | 0x3C => 3,
        0x39 => 4,
        0x3A | 0x3B => 5,
        0x83 => 6,
        0x8A => 7,
        0x84 => 8,
        0x85 | 0x89 => 9,
        0x86 => 10,
        0x87 | 0x88 => 11,
        _ => return None,
    };
    Some(CAST_ROWS[i])
}

/// Re:CoM's attack row id for Riku's own swing that plays the same clip as Sora's row `sora_id`.
/// Sora's rows 1..7 (`0x1407C6C70`, action rows 0x30..0x36) have Riku's twins at 0x97..0x9D: the same
/// action rows and card columns, but their hit effect (row `+2`, by id: `CPlayerWeapon::onAttack`
/// `0x1402250B0` reads `0x1407C6C72[weapon+0x23C]` into the effect table `0x1405A1480`) is Riku's
/// `weapon` set (12; 0 on the finishers 4 / 7, in his `EFFECT/1500/EF1500.EFF`) instead of Sora's
/// `ex_5010` (in `EFFECT/1000`, not loaded in Riku's story: no spark on his hits). Other ids as they
/// are. Checked (exe: both row sets and the effect table; files: the two containers).
pub fn attack_id(sora_id: u32) -> u32 {
    if (1..=7).contains(&sora_id) {
        RIKU_ROW_1 - 1 + sora_id
    } else {
        sora_id
    }
}

/// Riku's twin of Sora's attack row 1 (`0x1407C6C70` row 0x97). Checked (exe).
pub const RIKU_ROW_1: u32 = 0x97;
/// Dark Riku's (rows 0x9E..=0xA4: action rows 0x30..=0x36, his effects 0x15 / 0x16). Checked (exe).
pub const DARK_ROW_1: u32 = 0x9E;

/// [`attack_id`] for Riku or Dark Riku.
pub fn attack_id_for(dark: bool, sora_id: u32) -> u32 {
    if dark && (1..=7).contains(&sora_id) {
        DARK_ROW_1 - 1 + sora_id
    } else {
        attack_id(sora_id)
    }
}

/// One of Riku's sleights in his command menu's Sleights list (fork owner, 2026-11-22: option (b),
/// "a list of his sleights paid with KH1 MP, and shortcuts").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sleight {
    /// Re:CoM's sleight id (`player+0x82C`).
    pub id: u16,
    /// `SY0002.CTD`'s name (its row id + 1; Checked (files)).
    pub name: &'static str,
    /// Whole MP points (ours: KH1 has no Riku; the Sleights list's tiers, the Dark ones a point more).
    /// 0 for the DP sleights (`crate::dark_points::DP_SLEIGHTS`, 0.179.0).
    pub mp: i32,
}

/// The sleights Riku's own sleight start takes alone (`CPlayerRiku` vtable `+0x130` = `0x140216AB0`:
/// ids 0x66..=0x6B start; 0x62..=0x64 are King Mickey's, his friend card; anything else refused).
/// Checked (exe: disasm). Dark Break / Dark Firaga / Dark Aura (0x5F..=0x61) are not among them.
pub const SLEIGHTS: [Sleight; 6] = [
    Sleight { id: 0x66, name: "Impulse", mp: 2 },
    Sleight { id: 0x67, name: "Maelstrom", mp: 2 },
    Sleight { id: 0x68, name: "Barrage", mp: 3 },
    Sleight { id: 0x69, name: "Dark Impulse", mp: 3 },
    Sleight { id: 0x6A, name: "Dark Maelstrom", mp: 3 },
    Sleight { id: 0x6B, name: "Dark Barrage", mp: 4 },
];

/// Dark Riku's Sleights list (0.152.0): his own sleight start (`CPlayerDRiku +0x130` =
/// `0x140206830`) takes 0x5F..=0x63 and 0x65..=0x6B (its jump table at `0x140206894`; 0x64 refused).
/// Checked (exe). The Dark ones, with Dark Break / Dark Firaga / Dark Aura (`SY0002.CTD`'s names);
/// MP ours, as Riku's tiers.
/// 0.179.0: Dark Break, Dark Firaga, Dark Aura and Inverse Burst (0x65: the same start path as the
/// first three, `0x14020E590` then `+0x138`; Checked (decomp)) pay DP (`crate::dark_points`).
pub const DARK_SLEIGHTS: [Sleight; 7] = [
    Sleight { id: 0x5F, name: "Dark Break", mp: 0 },
    Sleight { id: 0x60, name: "Dark Firaga", mp: 0 },
    Sleight { id: 0x61, name: "Dark Aura", mp: 0 },
    Sleight { id: 0x65, name: "Inverse Burst", mp: 0 },
    Sleight { id: 0x69, name: "Dark Impulse", mp: 3 },
    Sleight { id: 0x6A, name: "Dark Maelstrom", mp: 3 },
    Sleight { id: 0x6B, name: "Dark Barrage", mp: 4 },
];

/// Riku's or Dark Riku's sleight `id`.
pub fn sleight(id: u16) -> Option<&'static Sleight> {
    SLEIGHTS.iter().chain(DARK_SLEIGHTS.iter()).find(|s| s.id == id)
}

/// The list of one of them.
pub fn list(dark: bool) -> &'static [Sleight] {
    if dark {
        &DARK_SLEIGHTS
    } else {
        &SLEIGHTS
    }
}

/// 0.198.15: the ids of one list that Re:CoM has taught (`learned`: Re:CoM's own test `0x14023AB00`, the card state
/// in the save). Re:CoM teaches Riku 0x5F..0x61 and 0x66..0x6B at the end of the Ansem tutorial (battle 1100, also
/// at Combat = KH1 through the story skip) and 0x65 with the King's three in B11's hall (`WORLD00/2103`); none at a
/// new game. Checked (files + decomp: the workspace's `knowledge/ReCoM/recom-riku-sleights.md`).
pub fn learned_ids(dark: bool, learned: impl Fn(u16) -> bool) -> Vec<u16> {
    list(dark).iter().map(|s| s.id).filter(|&id| learned(id)).collect()
}

/// Their words in the command menu (`crate::sleights::WORD_BASE + id`), each id once.
pub fn words() -> impl Iterator<Item = (u16, &'static str)> {
    SLEIGHTS.iter().chain(DARK_SLEIGHTS.iter().filter(|d| !SLEIGHTS.iter().any(|s| s.id == d.id))).map(|s| (crate::sleights::WORD_BASE + s.id, s.name))
}

/// The shortcut slots Riku gets when all three are empty (and were never set up: the caller's
/// test): his first learned sleights. Slots already set are kept; his Customize page (0.149.0) sets
/// them after.
pub fn fill_shortcuts(slots: [u8; 3], learned: &[u16]) -> Option<[u8; 3]> {
    if slots != [0; 3] || learned.is_empty() {
        return None;
    }
    let mut out = [0u8; 3];
    for (k, &id) in learned.iter().take(3).enumerate() {
        out[k] = id as u8;
    }
    Some(out)
}

/// 0.198.18: Dark Mode's own three shortcut slots (tester, relayed by the fork owner 2026-11-27: "a separate shortcut
/// menu for Riku's dark mode"; the owner's option 3: a separate set, filled with the twins the first time). In the
/// save's spare tail right after the mod's last block (`crate::abilities::SKILL_EXT_LAST`), which no Re:CoM code reads
/// or writes (`crate::keyblades::SPARE_FIRST ..= SPARE_LAST`, Checked (exe), `docs/recom-keyblade-equip.md`). Each
/// story keeps its own save, so only Riku's story uses it.
pub mod dark_block {
    pub const FIRST: usize = crate::abilities::SKILL_EXT_LAST + 1;
    pub const LEN: usize = 4;
    pub const LAST: usize = FIRST + LEN - 1;
    /// The three slot bytes, as Riku's own (`crate::mp::off::SHORTCUTS`: 0 none, 1..=7 spell + 1, else a sleight id).
    pub const SLOTS: usize = 0;
    /// Bit [`SET`]: the set was filled (from his own set's twins) or set on his Customize page; the rest reserved (zero).
    pub const FLAGS: usize = 3;
    pub const SET: u8 = 1;
    const _: () = assert!(LAST <= crate::keyblades::SPARE_LAST && FLAGS + 1 == LEN);
}

/// Riku's own sleights and their Dark twins (`DARK_SLEIGHTS`), Re:CoM's own rule (Checked (exe + decomp), the workspace's
/// `recom-riku-sleights.md` § 4): in Dark Mode the duel end `0x140162DD0` gives 0x69 / 0x6A / 0x6B where Riku gets
/// 0x66 / 0x67 / 0x68 (main player `+0x420 & 0x400000`), the stack check `0x1401669F0` forms 0x65 Inverse Burst where Riku
/// forms 0x64 Holy Burst (stock flag `+0x98 & 0x100`), and Dark Riku's sleight init `0x1402042F0` has no case for 0x66..0x68.
pub const TWINS: [(u16, u16); 4] = [(0x66, 0x69), (0x67, 0x6A), (0x68, 0x6B), (0x64, 0x65)];

/// The King's sleights Dark Riku's own start and init take (`0x140206830`, `0x1402042F0`: 0x62, 0x63 (it plays LV3, the
/// same use counter as 0x62); 0x64 Holy Burst refused: his is 0x65, [`TWINS`]). Checked (exe + decomp).
pub const DARK_KING: [u16; 2] = [0x62, 0x63];

/// One slot byte of Riku's set as it goes into Dark Mode's set the first time: a spell stays; one of Dark Riku's
/// learned sleights (`dark_learned`) stays; Impulse / Maelstrom / Barrage / Holy Burst become their Dark twin ([`TWINS`],
/// Re:CoM's rule) when Re:CoM has taught it; the King's two Dark Riku can start stay; anything else is emptied.
pub fn dark_twin(b: u8, dark_learned: &[u16]) -> u8 {
    let id = b as u16;
    if (1..=7).contains(&b) || DARK_KING.contains(&id) || dark_learned.contains(&id) {
        return b;
    }
    TWINS.iter().find(|t| t.0 == id).map(|t| t.1).filter(|d| dark_learned.contains(d)).map_or(0, |d| d as u8)
}

/// Dark Mode's set the first time it is needed: Riku's set, slot by slot through [`dark_twin`].
pub fn dark_fill(riku: [u8; 3], dark_learned: &[u16]) -> [u8; 3] {
    riku.map(|b| dark_twin(b, dark_learned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::row;

    /// 0.198.18: Dark Mode's set starts as Riku's with the Dark twins: no shortcut goes dead on the change.
    #[test]
    fn dark_modes_set_starts_with_the_twins() {
        let dark: Vec<u16> = DARK_SLEIGHTS.iter().map(|s| s.id).collect();
        // Impulse, Fire (spell byte 1), Barrage → Dark Impulse, Fire, Dark Barrage.
        assert_eq!(dark_fill([0x66, 1, 0x68], &dark), [0x69, 1, 0x6B]);
        // Dark ones and the King's two Dark Riku can start stay; Holy Burst (0x64) becomes Inverse Burst (0x65, Re:CoM's
        // own swap in Dark Mode); an empty slot stays empty.
        assert_eq!(dark_fill([0x6A, 0x63, 0x64], &dark), [0x6A, 0x63, 0x65]);
        assert_eq!(dark_fill([0; 3], &dark), [0; 3]);
        // A twin Re:CoM hasn't taught yet: empty, not a sleight he can't use.
        assert_eq!(dark_fill([0x67, 0x66, 0], &[0x69]), [0, 0x69, 0]);
        // Every twin is one of Dark Riku's sleights, every source one of Riku's (or the King's Holy Burst, his friend card).
        for (r, d) in TWINS {
            assert!(SLEIGHTS.iter().any(|s| s.id == r) || r == 0x64);
            assert!(DARK_SLEIGHTS.iter().any(|s| s.id == d));
        }
        assert_eq!(dark_block::FIRST, 0x26D0);
    }

    #[test]
    fn rikus_rows_cover_kh1_movement() {
        for r in [row::IDLE, row::WALK, row::RUN, row::JUMP, row::JUMP_TOP, row::FALL, row::LAND, row::WALK_B, row::RUN_B, row::ROLL, row::STANCE_IDLE, row::STANCE_WALK, row::STANCE_RUN, row::HIGH_JUMP, row::HIGH_JUMP_TOP] {
            assert!(row_ok(r), "row {r:#x}");
        }
        for r in [row::GLIDE, row::GLIDE_FAST, row::GLIDE_TURN_A, row::GLIDE_TURN_B, row::THUNDER, row::BLIZZARD, row::FIRE_AIR_WINDUP, 0x46, 0x47, 0x60] {
            assert!(!row_ok(r), "row {r:#x}");
        }
    }

    #[test]
    fn rikus_attack_rows() {
        assert_eq!((1..=7).map(attack_id).collect::<Vec<_>>(), (0x97..=0x9D).collect::<Vec<_>>());
        assert_eq!(attack_id(0), 0);
        assert_eq!(attack_id(0x44), 0x44);
    }

    #[test]
    fn only_the_sleights_recom_taught_are_listed() {
        // A new game: none (the owner's screenshot of Re:CoM's Sleights List, all `?????`).
        assert!(learned_ids(false, |_| false).is_empty());
        assert!(learned_ids(true, |_| false).is_empty());
        // After the Ansem tutorial (cards 0x5F..0x61, 0x66..0x6B): all of Riku's, Dark Riku's but Inverse Burst.
        let tutorial = |id: u16| (0x5F..=0x61).contains(&id) || (0x66..=0x6B).contains(&id);
        assert_eq!(learned_ids(false, tutorial), vec![0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B]);
        assert_eq!(learned_ids(true, tutorial), vec![0x5F, 0x60, 0x61, 0x69, 0x6A, 0x6B]);
        // B11's hall adds 0x65.
        assert_eq!(learned_ids(true, |id| tutorial(id) || id == 0x65), vec![0x5F, 0x60, 0x61, 0x65, 0x69, 0x6A, 0x6B]);
        // The shortcuts fill from what is learned.
        assert_eq!(fill_shortcuts([0; 3], &learned_ids(false, tutorial)), Some([0x66, 0x67, 0x68]));
        assert_eq!(fill_shortcuts([0; 3], &learned_ids(false, |_| false)), None);
    }

    #[test]
    fn rikus_sleights() {
        assert!(SLEIGHTS.iter().all(|s| (0x66..=0x6B).contains(&s.id)));
        assert!(DARK_SLEIGHTS.iter().all(|s| (0x5F..=0x63).contains(&s.id) || (0x65..=0x6B).contains(&s.id)));
        assert_eq!(words().count(), 10, "0.179.0: Inverse Burst");
        // 0.179.0: Dark Riku's own four pay DP, never MP.
        assert!([0x5F, 0x60, 0x61, 0x65].iter().all(|&id| crate::sleights::pay_points(id, 99).is_none() && crate::sleights::cost(id).is_some_and(|c| matches!(c, crate::sleights::Cost::Dp(_)))));
        assert_eq!(sleight(0x65).map(|s| s.name), Some("Inverse Burst"));
        assert_eq!(attack_id_for(true, 4), 0xA1);
        assert_eq!(attack_id_for(false, 4), 0x9A);
        assert!(row_ok_for(true, 0x30) && !row_ok_for(true, 0x08) && row_ok_for(false, 0x08));
        assert_eq!(sleight(0x67).map(|s| s.name), Some("Maelstrom"));
        assert_eq!(sleight(0x5F).map(|s| s.name), Some("Dark Break"));
        assert!(sleight(0x64).is_none(), "Holy Burst is the King's");
        assert_eq!(fill_shortcuts([0; 3], &[0x66, 0x67]), Some([0x66, 0x67, 0]));
        assert_eq!(fill_shortcuts([0x66, 0, 0], &[0x67]), None);
        assert_eq!(fill_shortcuts([0; 3], &[]), None);
        // Their commands don't reach Stagger's 0xF0.
        assert!(SLEIGHTS.iter().all(|s| crate::sleights::command(s.id) < 0xF0));
        assert!(SLEIGHTS.iter().all(|s| crate::sleights::of_command(crate::sleights::command(s.id)) == Some(s.id)));
        assert_eq!(crate::sleights::pay_points(0x68, 3), Some(3));
        assert_eq!(crate::sleights::pay_points(0x68, 2), None);
        assert_eq!(crate::sleights::name(0x69), "Dark Impulse");
        assert_eq!(crate::sleights::list(&[0x67, 0x66]), vec![0x66, 0x67]);
        assert_eq!(crate::command_menu::shortcut::slot_sleight(0x66), Some(0x66));
    }

    #[test]
    fn rikus_kh1_rows_blend_in_and_out() {
        // From the stance into a cast: blended in; Fire's wind-up into its release: as KH1 asks (0).
        assert_eq!(blend_for(0x2A, 0x3B, true, 0.0), CAST_IN);
        assert_eq!(blend_for(0x3B, 0x3C, false, 0.0), 0.0);
        // Out of a cast clip, the Guard or its recoil into the idle: blended out.
        assert_eq!(blend_for(0x3D, 0x2A, false, 0.0), OUT);
        assert_eq!(blend_for(KH1_GUARD_ROW, 0x2A, false, 0.0), OUT);
        assert_eq!(blend_for(KH1_RECOIL_ROW, 0x01, false, 4.0), OUT);
        // The Guard into its recoil (a block): at once, as KH1.
        assert_eq!(blend_for(KH1_GUARD_ROW, KH1_RECOIL_ROW, false, 0.0), 0.0);
        // Anything else as asked.
        assert_eq!(blend_for(0x01, 0x30, false, 8.0), 8.0);
    }

    #[test]
    fn the_idle_after_a_cast_is_the_stance() {
        use crate::rows::row::{IDLE, STANCE_IDLE, WALK};
        for &c in &CAST_ROWS {
            assert_eq!(idle_after_cast(c as i32, 1, IDLE), STANCE_IDLE);
            assert_eq!(idle_after_cast(1, c as i32, IDLE), STANCE_IDLE);
        }
        assert_eq!(idle_after_cast(1, 2, IDLE), IDLE, "a plain field idle stays Re:CoM's");
        assert_eq!(idle_after_cast(CAST_ROWS[0] as i32, 1, WALK), WALK, "only the idle request changes");
    }

    #[test]
    fn every_kh1_cast_request_has_a_cast_row() {
        use crate::magic::cast_req as c;
        for spell in 0..7u8 {
            for air in [false, true] {
                assert!(cast_row(c::first(spell, air)).is_some() && cast_row(c::release(spell, air)).is_some(), "{spell} {air}");
            }
        }
        assert_eq!(cast_row(c::FIRE_WINDUP), Some(0x3B));
        assert_eq!(cast_row(c::CURE), Some(0x3F));
        assert_eq!(cast_row(c::AERO_AIR), Some(0x44));
        assert_eq!(cast_row(c::FIRE_AIR_RELEASE), Some(0x42));
        // Ground and air apart: no air request plays a ground row.
        for s in 0..7u8 {
            assert_ne!(cast_row(c::first(s, false)), cast_row(c::first(s, true)), "{s}");
        }
        assert_eq!(cast_row(0x2A), None);
    }

    #[test]
    fn rikus_guard_faces_the_front() {
        let half = 1.745_329_3;
        // Yaw 0 faces +z (atan2(x, z) = 0).
        assert!(guard_faces([0.0; 3], 0.0, [0.0, 0.0, 2.0], half));
        assert!(guard_faces([0.0; 3], 0.0, [2.0, 0.0, 0.1], half));
        assert!(!guard_faces([0.0; 3], 0.0, [0.0, 0.0, -2.0], half));
        assert!(!guard_faces([0.0; 3], 0.0, [0.5, 0.0, -2.0], half));
        // Facing −z (yaw π): behind is +z.
        assert!(guard_faces([1.0, 0.0, 1.0], std::f32::consts::PI, [1.0, 0.0, -3.0], half));
        assert!(!guard_faces([1.0, 0.0, 1.0], std::f32::consts::PI, [1.0, 0.0, 4.0], half));
    }

    #[test]
    fn rikus_guard_blade_is_recoms_weapon_line() {
        use kh1_core::Vec3;
        // A bone at (1, 2, 3) whose x axis points along world −z: the end is 0.8 m along +z.
        let rows = [Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 3.0)];
        let (end, grip) = guard_blade(rows);
        assert_eq!(grip, Vec3::new(1.0, 2.0, 3.0));
        assert!((end - Vec3::new(1.0, 2.0, 3.8)).length() < 1e-6, "{end:?}");
    }

    #[test]
    fn rikus_abilities() {
        let a = abilities(0);
        assert!(a.dodge_roll && !a.high_jump && !a.glide && !a.superglide && a.guard && a.counterattack && a.combo_plus == 0);
        assert!(abilities(HIGH_JUMP_BIT).high_jump);
    }
}
