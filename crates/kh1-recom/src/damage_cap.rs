//! Sora's damage cap (owner, 2026-11-05; `docs/recom-kh1-attack.md` § Damage cap): each hit on an
//! enemy from Sora's side is capped at that enemy's KH1 per-hit cap (its KH1 record's `+0xC2`), at
//! Combat = KH1 in Sora's story. Testable rules and bytes; the runtime is `win::damage_cap`.
//!
//! - **KH1's rule** (Checked (KH1 decomp `0x1402BFA80`)): power → `curve[clamp(power − Defence)]` →
//!   the attack's bounds → the defender's floor `+0xC0` / cap `+0xC2` → × the element's resistance →
//!   Aero / 2 → at least 1. The % of HP element (8) and elements 9 / 11 / 12 are never capped.
//! - **Where in Re:CoM** (Checked (Re:CoM decomp + disasm)): the enemy damage apply `0x14017DC50`'s
//!   join `0x14017DF6A`, where every damaging kind arrives with its value in `xmm6` (the gathered
//!   part value: weak part `0x140154EB0`, the frozen bonus `0x14015D940`, Re:CoM's kind 0xD % of HP
//!   already in it) and the attribute's resistance % in `xmm9`; `0x14017DAB0` then takes
//!   `(int)(value × resistance × boss falloff / 100)` off the HP. The value is also stored in
//!   `+0x2DC`, which the flatten state's exe `0x140184D20` applies again (`0x140184DCB`). Capping
//!   `xmm6` there is KH1's order: after every bonus, before the resistance (and the boss falloff).
//! - **The cap in the join's units**, set per hit by the enemy-hit wrapper:
//!   - Damage = KH1, a KH1 number (swings, specials, spells, Ragnarok's shots): our number carries
//!     KH1's resistance already, so the cap is KH1's cap × that resistance
//!     (`kh1_core::damage::compute_damage_cap_later`), the number made without its cap;
//!   - Damage = KH1, Re:CoM's number (Sleights-list sleights, LV3 summons, Party sleights, scaled by
//!     KH1 / Re:CoM max HP, `crate::sleights::kh1_scaled`): the raw cap ([`raw`]);
//!   - Damage = Re:CoM: the same share of HP ([`recom_share`]).
//!   - No cap: the % of HP hits (Gravity Break, Zantetsuken, Gravity; Re:CoM's kind 0xD), Stop
//!     (kind 8 never reaches the join), an enemy without a KH1 record or with cap 0.

/// Re:CoM's join (`0x14017DF6A`): `movaps xmm2,xmm9; movss [rbx+0x2DC],xmm6; movaps xmm1,xmm6; mov
/// rcx,rbx; call 0x14017DAB0` (23 bytes). Eleven branches of `0x14017DC50` land on its first byte;
/// none inside (Checked (exe: a disasm of `0x14017DC50..0x14017DFC0` and a rel32 scan of `.text`)).
pub const JOIN: u64 = 0x1_4017_DF6A;
pub const JOIN_BYTES: [u8; 23] = [0x41, 0x0F, 0x28, 0xD1, 0xF3, 0x0F, 0x11, 0xB3, 0xDC, 0x02, 0x00, 0x00, 0x0F, 0x28, 0xCE, 0x48, 0x8B, 0xCB, 0xE8, 0x2F, 0xFB, 0xFF, 0xFF];
/// `0x14017DAB0`, the call's target (the HP store).
pub const APPLY: u64 = 0x1_4017_DAB0;
/// After the call: `test eax,eax` (`0x14017DF81`).
pub const BACK: u64 = JOIN + JOIN_BYTES.len() as u64;

/// Re:CoM's kinds whose value is not a plain HP damage the cap is for: 8 (Stop: the value is a
/// duration, `0x14017DCB5`, never at the join) and 0xD (Gravity's % of the current HP, `0x14017DD4C`).
pub fn never_capped_kind(kind: u32) -> bool {
    kind == 8 || kind == 0xD
}

/// Damage = KH1, a hit carrying Re:CoM's own number on KH1's HP (a sleight's or summon's): KH1's
/// raw cap. None: no cap.
pub fn raw(cap: u16) -> Option<f32> {
    (cap != 0).then_some(cap as f32)
}

/// Damage = Re:CoM (owner's answer (b)): the same share of HP, `cap × Re:CoM max HP ÷ KH1 max HP`,
/// rounded to the nearest whole point (halves up), at least 1. KH1 max HP: the record's HP (a twin's
/// KH1 HP, a proxy's KH1-scale HP, a boss twin's own). None: no cap, or no HP to share.
pub fn recom_share(cap: u16, recom_max_hp: i32, kh1_max_hp: i32) -> Option<f32> {
    if cap == 0 || recom_max_hp <= 0 || kh1_max_hp <= 0 {
        return None;
    }
    let (c, r, k) = (cap as i64, recom_max_hp as i64, kh1_max_hp as i64);
    let v = (2 * c * r + k) / (2 * k);
    Some(v.clamp(1, i32::MAX as i64) as f32)
}

/// What the join does with a value and a cap: the smaller (`comiss` / `jbe`: a value at or under
/// the cap, or NaN, is left).
pub fn join(value: f32, cap: Option<f32>) -> f32 {
    match cap {
        Some(c) if value > c => c,
        _ => value,
    }
}

/// Re:CoM's stop exe `0x1401843A0` records a held hit from the gathered value (`+0xF8 → +0x34`),
/// not from the join: the wrapper caps that value too while the enemy is stopped.
pub fn held(value: i32, cap: f32) -> i32 {
    if value as f32 > cap {
        cap as i32
    } else {
        value
    }
}

/// The table the stub reads: this many enemies with a cap (each `{u64 enemy; f32 cap; u32 pad}`).
pub const SLOTS: usize = 16;
/// The block: `+0` count of capped hits (u64), `+8` the last capped enemy, `+0x10` / `+0x14` its
/// value before / after (f32), `+0x20` the slots.
pub const SLOTS_AT: u64 = 0x20;
pub const SLOT_SIZE: u64 = 16;

/// The stub at `at` (99 bytes): finds `rbx` (the enemy) in the slots; when its value `xmm6` is over
/// the slot's cap it counts, records and takes the cap; then Re:CoM's own four instructions and its
/// call to `apply`, and a jump back to `back`. It uses `rax`, `rcx` and `xmm0` before Re:CoM's
/// call, which overwrites them anyway (volatile), and no stack. None: out of rel32 reach.
pub fn stub(block: u64, at: u64, apply: u64, back: u64) -> Option<Vec<u8>> {
    let mut v = Vec::with_capacity(100);
    v.extend_from_slice(&[0x48, 0xB8]); // mov rax, slots
    v.extend_from_slice(&(block + SLOTS_AT).to_le_bytes());
    v.extend_from_slice(&[0xB9, SLOTS as u8, 0, 0, 0]); // mov ecx, SLOTS
    debug_assert_eq!(v.len(), 15);
    v.extend_from_slice(&[0x48, 0x3B, 0x18]); // loop: cmp rbx, [rax]
    v.extend_from_slice(&[0x74, 0x0A]); // je hit
    v.extend_from_slice(&[0x48, 0x83, 0xC0, SLOT_SIZE as u8]); // add rax, 16
    v.extend_from_slice(&[0xFF, 0xC9]); // dec ecx
    v.extend_from_slice(&[0x75, 0xF3]); // jnz loop
    v.extend_from_slice(&[0xEB, 0x29]); // jmp recom
    debug_assert_eq!(v.len(), 30);
    v.extend_from_slice(&[0xF3, 0x0F, 0x10, 0x40, 0x08]); // hit: movss xmm0, [rax+8]
    v.extend_from_slice(&[0x0F, 0x2F, 0xF0]); // comiss xmm6, xmm0
    v.extend_from_slice(&[0x76, 0x1F]); // jbe recom
    v.extend_from_slice(&[0x48, 0xB8]); // mov rax, block
    v.extend_from_slice(&block.to_le_bytes());
    v.extend_from_slice(&[0xF0, 0x48, 0xFF, 0x00]); // lock inc qword [rax]
    v.extend_from_slice(&[0x48, 0x89, 0x58, 0x08]); // mov [rax+8], rbx
    v.extend_from_slice(&[0xF3, 0x0F, 0x11, 0x70, 0x10]); // movss [rax+0x10], xmm6
    v.extend_from_slice(&[0xF3, 0x0F, 0x11, 0x40, 0x14]); // movss [rax+0x14], xmm0
    v.extend_from_slice(&[0x0F, 0x28, 0xF0]); // movaps xmm6, xmm0
    debug_assert_eq!(v.len(), 71);
    v.extend_from_slice(&JOIN_BYTES[..18]); // recom: Re:CoM's four instructions
    let call = at + v.len() as u64;
    v.extend_from_slice(&crate::hookplan::call_rel32(call, apply)?);
    let jmp = at + v.len() as u64;
    v.extend_from_slice(&crate::hookplan::rel_jmp(jmp, back)?);
    Some(v)
}

/// The stub's length.
pub const STUB_LEN: usize = 99;

/// Which rule set a hit's cap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rule {
    /// Damage = KH1, KH1's number: the cap × the element's resistance (that factor).
    Kh1 { resist: f32 },
    /// Damage = KH1, Re:CoM's number on KH1's HP: the raw cap.
    Raw,
    /// Damage = Re:CoM: the share of Re:CoM's max HP.
    Share { recom_hp: i32 },
}

/// One capped hit's log line ("damage cap: Guard Armor: 48 → 30 (KH1 cap 30 of 700 HP; …)").
pub fn line(enemy: &str, from: &str, what: &str, before: f32, after: f32, cap: u16, kh1_hp: i32, rule: Rule) -> String {
    let how = match rule {
        Rule::Kh1 { resist } if (resist - 1.0).abs() < 1e-6 => "; Damage = KH1".to_string(),
        Rule::Kh1 { resist } => format!(" × KH1's resistance {:.0} %, which comes after the cap; Damage = KH1", resist * 100.0),
        Rule::Raw => "; Damage = KH1, Re:CoM's number on KH1's HP".to_string(),
        Rule::Share { recom_hp } => format!(" = {} of Re:CoM's {recom_hp} HP (the same share); Damage = Re:CoM", after.round() as i32),
    };
    format!("damage cap: {enemy} ({from}), {what}: {before:.0} -> {after:.0} (KH1 cap {cap} of {kh1_hp} HP{how}); Re:CoM's resistance and boss falloff come after")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_share_of_hp() {
        // Guard Armor: KH1 cap 30 of 700 HP.
        assert_eq!(recom_share(30, 700, 700), Some(30.0));
        assert_eq!(recom_share(30, 1200, 700), Some(51.0), "51.43 → 51");
        assert_eq!(recom_share(30, 1190, 700), Some(51.0), "51.0");
        assert_eq!(recom_share(20, 35, 700), Some(1.0), "1.0");
        assert_eq!(recom_share(20, 17, 700), Some(1.0), "0.49 → at least 1");
        assert_eq!(recom_share(15, 50, 60), Some(13.0), "12.5 → 13 (halves up)");
        assert_eq!(recom_share(0, 100, 100), None, "cap 0: none");
        assert_eq!(recom_share(30, 100, 0), None);
        assert_eq!(recom_share(30, 0, 100), None);
        assert_eq!(recom_share(300, 30000, 12), Some(750000.0), "no overflow");
        assert_eq!(raw(30), Some(30.0));
        assert_eq!(raw(0), None);
    }

    #[test]
    fn the_join_takes_the_smaller() {
        assert_eq!(join(48.0, Some(30.0)), 30.0);
        assert_eq!(join(30.0, Some(30.0)), 30.0);
        assert_eq!(join(12.0, Some(30.0)), 12.0);
        assert_eq!(join(48.0, None), 48.0);
        assert!(join(f32::NAN, Some(30.0)).is_nan(), "comiss / jbe leaves NaN");
        assert_eq!(held(48, 30.0), 30);
        assert_eq!(held(24, 24.000_000_4), 24);
        assert_eq!(held(12, 30.0), 12);
        assert!(never_capped_kind(0xD) && never_capped_kind(8) && !never_capped_kind(1) && !never_capped_kind(2) && !never_capped_kind(7));
    }

    #[test]
    fn the_stub_replays_recoms_join() {
        let block = 0x0000_7FF6_1234_0000u64;
        let at = JOIN - 0x1000_0000;
        let s = stub(block, at, APPLY, BACK).unwrap();
        assert_eq!(s.len(), STUB_LEN);
        assert_eq!(&s[2..10], &(block + SLOTS_AT).to_le_bytes());
        assert_eq!(&s[42..50], &block.to_le_bytes());
        // Re:CoM's four instructions as they are, then its call to the same target, then back.
        assert_eq!(&s[71..89], &JOIN_BYTES[..18]);
        assert_eq!(crate::hookplan::call_target(at + 89, &s[89..94]), Some(APPLY));
        assert_eq!(crate::hookplan::call_target(JOIN + 18, &JOIN_BYTES[18..]), Some(APPLY), "the site's own call goes there");
        assert_eq!(s[94], 0xE9);
        let rel = i32::from_le_bytes(s[95..99].try_into().unwrap()) as i64;
        assert_eq!((at as i64 + 99 + rel) as u64, BACK);
        assert_eq!(BACK, 0x1_4017_DF81);
        // The short branches: loop back to 15, both exits to Re:CoM's bytes at 71, the hit at 30.
        assert_eq!(s[18], 0x74);
        assert_eq!(20 + s[19] as usize, 30, "je hit");
        assert_eq!(28 + (s[27] as i8 as isize), 15, "jnz loop");
        assert_eq!(30 + s[29] as usize, 71, "jmp recom");
        assert_eq!(40 + s[39] as usize, 71, "jbe recom");
        assert!(stub(block, 0x10, APPLY, BACK).is_none(), "out of reach");
    }

    #[test]
    fn log_lines() {
        let l = line("CGuardarmorFoot", "boss twin", "swing row 13", 48.0, 30.0, 30, 700, Rule::Kh1 { resist: 1.0 });
        assert!(l.starts_with("damage cap: CGuardarmorFoot (boss twin), swing row 13: 48 -> 30 (KH1 cap 30 of 700 HP; Damage = KH1)"), "{l}");
        let l = line("CBossGuardarmor", "boss twin", "sleight Sonic Blade", 120.0, 51.0, 30, 700, Rule::Share { recom_hp: 1200 });
        assert!(l.contains("120 -> 51 (KH1 cap 30 of 700 HP = 51 of Re:CoM's 1200 HP"), "{l}");
        let l = line("CEnemyShadow", "twin", "Fire1", 45.0, 45.0, 30, 21, Rule::Kh1 { resist: 1.5 });
        assert!(l.contains("× KH1's resistance 150 %"), "{l}");
    }
}
