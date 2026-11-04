//! Enemy Skills, plan stage 4 (`docs/recom-enemy-skills.md` § "Stage 4 (built)"): the skills Re:CoM
//! tests with its own compare `slot == id` on Sora's side (research § 2.3), at Combat = KH1 in Sora's
//! story. Each compare also answers true while the skill is equipped, through a byte-checked patch
//! per site, the way Second Chance's stub was done (`crate::kh1_support::second_chance`): a jump to a
//! stub next to the exe that runs Re:CoM's moved instructions and Re:CoM's own outcome for the card.
//! Re:CoM then runs the effect itself. The glue (placing the stubs, arming them at each battle start,
//! the Auto-Life revive) is `win::enemy_skills_cmp`.
//!
//! - **Axel, Quick Recovery** (0x6F): Sora's damage check `0x1401E6DE0` sets `player+0x420` bit 0x4000
//!   when the slot is 0x6F (`0x1401E73A8 cmp edx,0x2C`); the hit-reaction exe `0x1401EBD80` then lets
//!   Sora act out of the reaction at once (its `0x1401E93A0(pl, 1, 0)`: move, jump, roll, commands).
//!   The slot read `0x1401E7381` answers 0x6F. Checked (decomp + exe).
//! - **Larxene, Dash** (0x70): Re:CoM's stick class `0x1401E9630` multiplies walk / run by 1.7
//!   (`0x1405A08BC`) when the slot is 0x70. Owner (2026-11-04, "Enemy Skills numbers"): **run × 1.25**.
//!   Re:CoM's factor gets × 1.25 at the class's join `0x1401E967F` (battle only: Re:CoM's card battle
//!   object `player+0x3E8` set, as the card's test). At Combat = KH1 Sora's run in the move exe is
//!   KH1's (`win::movement::stick_class`): it takes the same × 1.25 ([`DASH`]).
//! - **Vexen, Auto-Life** (0x71): three compares decide Re:CoM's revive: the KO init `0x1401F17F0`
//!   (`0x1401F1A85`: no death clear), the dead player's sequence `0x140210F10` (`0x140211032`: the
//!   revive flag 0x10 instead of the game over) and the game-over controller `0x1401A16E0`
//!   (`0x1401A1A13`). Their slot reads answer 0x71 while it is **available**: once per battle (owner),
//!   with Re:CoM's own revive at max HP ÷ 4 (`0x1401EC250` sub 3: `0x1401E59A0(pl, max / 4, 6000, 1)`,
//!   = the owner's ¼) and Re:CoM's 1 s of invulnerability. Its count call there (`0x1401EC369`,
//!   `0x14015E010(1, 0x28, 0x2E)`) is retargeted: the revive marks it used. Checked (decomp + exe).
//! - **Platinum Card, Invincible** (0x7E): two compares keep the HP (`0x1401E71F3` in the damage check,
//!   `0x1401F9129` in the HP loss `0x1401F90B0`). Owner: **only the first 5 hits each battle** do no
//!   damage ([`PLATINUM_HITS`]). Each site's join (after its compare) gets Re:CoM's outcome (`cmove
//!   ebx, r8d` / `mov edi, edx`: HP unchanged) while hits are left, one taken per damaging hit.
//! - **Ansem, Sleightblind** (0x74): its Re:CoM meaning hides sleights from a deck (no meaning at
//!   KH1). The khwiki's "fire, ice and thunder resistance" is Re:CoM's Ansem affinity record
//!   `0x1407C5C50` = 100 / 100 / **50 / 50 / 50** / 100 (% damage by attribute; 2 Fire, 3 Blizzard,
//!   4 Thunder), which the card's play writes into `player+0x7C4..` (`0x140163C00` case 0x74 →
//!   `0x1401F8810`) and the damage check reads (`0x1401E7150`). At KH1 Sora plays no card, so his
//!   record stays Re:CoM's default (all 100, `0x1407C5B90`); the stub at the read's join
//!   (`0x1401E7163`) gives Ansem's 50 for attributes 2..4 where Re:CoM's value is the default 100.
//!   Checked (decomp + exe). Note: the owner's "boss skills give the skill only, without the boss's
//!   elemental affinities" (2026-11-04) is lifted for Ansem alone (brief: build the resistance).
//! - **Gold Card, Premium Guard** (0x7D): no meaning at KH1 (no deck, no premium cards): left greyed.
//!
//! The stubs read [`Armed`] (one byte per skill, Platinum's hits left) through an absolute address,
//! and the main player (`*0x14087B380`) rip-relative, so only Sora's own checks take the outcome.
//! Nothing of ours runs per frame; each battle start sets the bytes once.

use crate::sites::Site;

/// The skills' bytes in [`Armed::flags`].
pub mod ix {
    pub const AXEL: usize = 0;
    pub const LARXENE: usize = 1;
    pub const VEXEN: usize = 2;
    pub const ANSEM: usize = 3;
    pub const COUNT: usize = 4;
}

/// The card ids (Re:CoM's enemy card kinds).
pub mod card {
    pub const AXEL: u8 = 0x6F;
    pub const LARXENE: u8 = 0x70;
    pub const VEXEN: u8 = 0x71;
    pub const ANSEM: u8 = 0x74;
    pub const GOLD: u8 = 0x7D;
    pub const PLATINUM: u8 = 0x7E;
}

/// Owner (2026-11-04): Platinum Card's Invincible covers the first 5 hits of each battle.
pub const PLATINUM_HITS: i32 = 5;
/// Owner (2026-11-04): Larxene's Dash, run × 1.25 (Re:CoM's card: × 1.7, `0x1405A08BC`).
pub const DASH: f32 = 1.25;
/// Re:CoM's Ansem affinity at attributes 2..4 (`0x1407C5C50`), and its default (`0x1407C5B90`).
pub const ANSEM_PERCENT: i32 = 50;
pub const DEFAULT_PERCENT: i32 = 100;

/// What the stubs read: one byte per skill (non-zero: the compare answers true), Platinum's hits
/// left this battle. `repr(C)`: the stubs' offsets.
#[repr(C)]
pub struct Armed {
    pub flags: [std::sync::atomic::AtomicU8; 8],
    pub platinum_left: std::sync::atomic::AtomicI32,
}

impl Armed {
    pub const fn new() -> Armed {
        use std::sync::atomic::{AtomicI32, AtomicU8};
        Armed { flags: [const { AtomicU8::new(0) }; 8], platinum_left: AtomicI32::new(0) }
    }
}

impl Default for Armed {
    fn default() -> Self {
        Armed::new()
    }
}

/// [`Armed::platinum_left`]'s offset.
pub const PLATINUM_LEFT: usize = 8;

/// What a battle start arms ([`arm`]): a skill acts at Combat = KH1 in Sora's story, equipped, its
/// stub placed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Arming {
    pub flags: [u8; ix::COUNT],
    pub platinum_left: i32,
}

/// The skills a battle start arms: `kh1` (Combat = KH1, Sora's story), `has(card)` (equipped, or
/// Black Fungus's for this battle), `placed(card)` (its stubs are in).
pub fn arm(kh1: bool, has: &dyn Fn(u8) -> bool, placed: &dyn Fn(u8) -> bool) -> Arming {
    let on = |c: u8| kh1 && has(c) && placed(c);
    let mut a = Arming::default();
    for (i, c) in [(ix::AXEL, card::AXEL), (ix::LARXENE, card::LARXENE), (ix::VEXEN, card::VEXEN), (ix::ANSEM, card::ANSEM)] {
        a.flags[i] = on(c) as u8;
    }
    a.platinum_left = if on(card::PLATINUM) { PLATINUM_HITS } else { 0 };
    a
}

/// The stub's outcome for Platinum's two sites: the HP after a hit (`new`) from `old`, with `left`
/// hits to cover. Returns the HP and the hits left.
pub fn platinum(old: i32, new: i32, left: i32, sora: bool) -> (i32, i32) {
    if sora && new < old && left > 0 {
        (old, left - 1)
    } else {
        (new, left)
    }
}

/// The stub's outcome for Ansem: the % Re:CoM read for attribute `attr` (`player+0x714`, the
/// index of `+0x7C4`).
pub fn ansem(percent: i32, attr: i32, on: bool) -> i32 {
    if on && (2..=4).contains(&attr) && percent == DEFAULT_PERCENT {
        ANSEM_PERCENT
    } else {
        percent
    }
}

/// The run speed at KH1's move exe (`win::movement::stick_class`) with Dash on (battle only).
pub fn dash_run(run: f32, on: bool) -> f32 {
    if on {
        run * DASH
    } else {
        run
    }
}

// ---- the sites (Steam exe; bytes read from `recomsteam.exe`, each checked at install) ----

/// The bytes each stub replaces (moved into the stub), in [`Patch`] order.
pub const PATCH: &[Site] = &[
    // movzx ecx, byte [rax+0x1E8]: the damage check's slot read before Axel's compare.
    Site { name: "es cmp: damage check slot read for Quick Recovery (0x1401E7381)", rva: 0x001E_7381, expect: &[0x0F, 0xB6, 0x88, 0xE8, 0x01, 0x00, 0x00] },
    // movss xmm0, [rip+0x31FBB9] (0.9, the walk / run threshold): the stick class's join after Dash's compare.
    Site { name: "es cmp: stick class join after Dash (0x1401E967F)", rva: 0x001E_967F, expect: &[0xF3, 0x0F, 0x10, 0x05, 0xB9, 0xFB, 0x31, 0x00] },
    // movzx eax, byte [rax+0x1E8]: the KO init's slot read before Auto-Life's compare.
    Site { name: "es cmp: KO init slot read for Auto-Life (0x1401F1A7A)", rva: 0x001F_1A7A, expect: &[0x0F, 0xB6, 0x80, 0xE8, 0x01, 0x00, 0x00] },
    // movzx eax, byte [rax+0x1E8]: the dead player's sequence (case 7) before Auto-Life's compare.
    Site { name: "es cmp: death sequence slot read for Auto-Life (0x140211027)", rva: 0x0021_1027, expect: &[0x0F, 0xB6, 0x80, 0xE8, 0x01, 0x00, 0x00] },
    // movzx ecx, byte [rax+0x1E8]: the game-over controller's slot read before Auto-Life's compare.
    Site { name: "es cmp: game-over controller slot read for Auto-Life (0x1401A1A07)", rva: 0x001A_1A07, expect: &[0x0F, 0xB6, 0x88, 0xE8, 0x01, 0x00, 0x00] },
    // mov rax, [rdi+0x3E8]: the damage check's join after Invincible's compare (`cmove ebx, r8d`).
    Site { name: "es cmp: damage check join after Invincible (0x1401E71FA)", rva: 0x001E_71FA, expect: &[0x48, 0x8B, 0x87, 0xE8, 0x03, 0x00, 0x00] },
    // test edi,edi; cmovns esi,edi; mov [rbx+0x42C],esi: the HP loss's store after Invincible's compare.
    Site { name: "es cmp: HP loss store after Invincible (0x1401F9163)", rva: 0x001F_9163, expect: &[0x85, 0xFF, 0x0F, 0x49, 0xF7, 0x89, 0xB3, 0x2C, 0x04, 0x00, 0x00] },
    // cvttss2si r14d, xmm0: the damage check's join after the affinity read.
    Site { name: "es cmp: damage check affinity join for Sleightblind (0x1401E7163)", rva: 0x001E_7163, expect: &[0xF3, 0x44, 0x0F, 0x2C, 0xF0] },
];

/// The retargeted call: Auto-Life's count at the revive (`0x1401EC250` sub 3).
pub const CALLS: &[Site] = &[Site { name: "es cmp: Auto-Life revive count call (0x1401EC369)", rva: 0x001E_C369, expect: &[0xE8, 0xA2, 0x1C, 0xF7, 0xFF] }];

/// Context: each Re:CoM compare the stubs stand for (not patched), the callee, Ansem's record.
pub const CHECKED: &[Site] = &[
    // cmp cl,0x43; jbe; movzx edx,cl; sub edx,0x43; jmp; mov edx,r15d; mov ecx,[rdi+0x420]; mov eax,ecx;
    // bts ecx,0xE; btr eax,0xE; cmp edx,0x2C; cmovne ecx,eax (Axel: bit 0x4000 when the slot is 0x6F).
    Site {
        name: "es cmp: Quick Recovery's compare (0x1401E7388)",
        rva: 0x001E_7388,
        expect: &[0x80, 0xF9, 0x43, 0x76, 0x08, 0x0F, 0xB6, 0xD1, 0x83, 0xEA, 0x43, 0xEB, 0x03, 0x41, 0x8B, 0xD7, 0x8B, 0x8F, 0x20, 0x04, 0x00, 0x00, 0x8B, 0xC1, 0x0F, 0xBA, 0xE9, 0x0E, 0x0F, 0xBA, 0xF0, 0x0E, 0x83, 0xFA, 0x2C, 0x0F, 0x45, 0xC8],
    },
    // mov rax,[rcx+0x3E8]; movss xmm2,[1.0]; test rax,rax; je join; movzx edx,[rax+0x1E8]; cmp dl,0x43; jbe join;
    // cmp dl,0x70; jne join; movss xmm2,[1.7] (Dash).
    Site {
        name: "es cmp: Dash's compare (0x1401E9652)",
        rva: 0x001E_9652,
        expect: &[
            0x48, 0x8B, 0x81, 0xE8, 0x03, 0x00, 0x00, 0xF3, 0x0F, 0x10, 0x15, 0x5F, 0x7A, 0x4D, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x19, 0x0F, 0xB6, 0x90, 0xE8, 0x01, 0x00, 0x00, 0x80, 0xFA, 0x43, 0x76, 0x0D, 0x80, 0xFA, 0x70, 0x75, 0x08, 0xF3, 0x0F, 0x10, 0x15, 0x3D, 0x72, 0x3B, 0x00,
        ],
    },
    // comiss xmm0,xmm1; mov rax,[rcx+0x3C0] after the join.
    Site { name: "es cmp: stick class after the join (0x1401E9687)", rva: 0x001E_9687, expect: &[0x0F, 0x2F, 0xC1, 0x48, 0x8B, 0x81, 0xC0, 0x03, 0x00, 0x00] },
    // cmp al,0x43; jbe; cmp al,0x71; je (no death clear).
    Site { name: "es cmp: KO init's Auto-Life compare (0x1401F1A81)", rva: 0x001F_1A81, expect: &[0x3C, 0x43, 0x76, 0x04, 0x3C, 0x71, 0x74, 0x74] },
    // cmp al,0x43; jbe; cmp al,0x71; jne (else the revive flag 0x10).
    Site { name: "es cmp: death sequence's Auto-Life compare (0x14021102E)", rva: 0x0021_102E, expect: &[0x3C, 0x43, 0x76, 0x21, 0x3C, 0x71, 0x75, 0x1D] },
    // cmp cl,0x43; jbe; cmp cl,0x71; jne (else +0x88 = 1).
    Site { name: "es cmp: game-over controller's Auto-Life compare (0x1401A1A0E)", rva: 0x001A_1A0E, expect: &[0x80, 0xF9, 0x43, 0x76, 0x0F, 0x80, 0xF9, 0x71, 0x75, 0x0A] },
    // lea edx,[rdi+0x28]; lea ecx,[rdi+1]; lea r8d,[rdi+0x2E] before the count call (1, 0x28, 0x2E).
    Site { name: "es cmp: Auto-Life count's arguments (0x1401EC35F)", rva: 0x001E_C35F, expect: &[0x8D, 0x57, 0x28, 0x8D, 0x4F, 0x01, 0x44, 0x8D, 0x47, 0x2E] },
    // The count `0x14015E010` (called by our revive callback): push rdi; sub rsp,0x20.
    Site { name: "es cmp: enemy card count (0x14015E010)", rva: 0x0015_E010, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x20] },
    // movzx ecx,[rax+0x1E8]; cmp cl,0x43; jbe; cmp cl,0x7E; cmove ebx,r8d (Invincible, damage check).
    Site { name: "es cmp: Invincible's compare in the damage check (0x1401E71E7)", rva: 0x001E_71E7, expect: &[0x0F, 0xB6, 0x88, 0xE8, 0x01, 0x00, 0x00, 0x80, 0xF9, 0x43, 0x76, 0x07, 0x80, 0xF9, 0x7E, 0x41, 0x0F, 0x44, 0xD8] },
    // movzx eax,[rax+0x1E8]; mov ecx,eax; cmp al,0x43; jbe; cmp al,0x7E; jne; mov edi,edx (Invincible, HP loss).
    Site { name: "es cmp: Invincible's compare in the HP loss (0x1401F911C)", rva: 0x001F_911C, expect: &[0x0F, 0xB6, 0x80, 0xE8, 0x01, 0x00, 0x00, 0x8B, 0xC8, 0x3C, 0x43, 0x76, 0x06, 0x3C, 0x7E, 0x75, 0x02, 0x8B, 0xFA] },
    // cmp ebx,5; ja; movss xmm0,[rdi+rbx*4+0x7C4]; jmp; movss xmm0,[rdi+0x7D8] (the affinity read).
    Site {
        name: "es cmp: damage check affinity read (0x1401E714B)",
        rva: 0x001E_714B,
        expect: &[0x83, 0xFB, 0x05, 0x77, 0x0B, 0xF3, 0x0F, 0x10, 0x84, 0x9F, 0xC4, 0x07, 0x00, 0x00, 0xEB, 0x08, 0xF3, 0x0F, 0x10, 0x87, 0xD8, 0x07, 0x00, 0x00],
    },
    // cmp r14d,0x3E8 after the join.
    Site { name: "es cmp: damage check after the affinity join (0x1401E7168)", rva: 0x001E_7168, expect: &[0x41, 0x81, 0xFE, 0xE8, 0x03, 0x00, 0x00] },
    // Re:CoM's Ansem affinity record (data): 100, 100, 50, 50, 50, 100 (f32).
    Site {
        name: "es cmp: Ansem's affinity record (0x1407C5C50)",
        rva: 0x007C_5C50,
        expect: &[0x00, 0x00, 0xC8, 0x42, 0x00, 0x00, 0xC8, 0x42, 0x00, 0x00, 0x48, 0x42, 0x00, 0x00, 0x48, 0x42, 0x00, 0x00, 0x48, 0x42, 0x00, 0x00, 0xC8, 0x42],
    },
];

/// The main player global (`*0x14087B380`, `sites::data::MAIN_PLAYER`), RVA.
pub const MAIN_PLAYER_RVA: u32 = 0x0087_B380;
/// The walk / run threshold the stick class's join reads (0.9), RVA.
pub const WALK_THRESHOLD_RVA: u32 = 0x0050_9240;

/// The stubs, in [`PATCH`] order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Patch {
    /// A slot read: Re:CoM's `movzx R, byte [rax+0x1E8]`, then (no enemy card and the skill armed,
    /// and the player check) the skill's id in R.
    Slot { reg: Reg, id: u8, flag: usize, player: Option<Reg64> },
    /// Dash at the stick class's join.
    Dash,
    /// Invincible at the damage check's join.
    PlatinumCheck,
    /// Invincible at the HP loss's store.
    PlatinumLoss,
    /// Sleightblind at the affinity join.
    Ansem,
}

/// The 32-bit register a slot read fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reg {
    Eax,
    Ecx,
}

/// The register holding the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reg64 {
    Rbx,
    Rdi,
}

/// Which stub each [`PATCH`] site gets, and the card it stands for.
pub const STUBS: [(Patch, u8); 8] = [
    (Patch::Slot { reg: Reg::Ecx, id: card::AXEL, flag: ix::AXEL, player: Some(Reg64::Rdi) }, card::AXEL),
    (Patch::Dash, card::LARXENE),
    (Patch::Slot { reg: Reg::Eax, id: card::VEXEN, flag: ix::VEXEN, player: Some(Reg64::Rbx) }, card::VEXEN),
    (Patch::Slot { reg: Reg::Eax, id: card::VEXEN, flag: ix::VEXEN, player: None }, card::VEXEN),
    (Patch::Slot { reg: Reg::Ecx, id: card::VEXEN, flag: ix::VEXEN, player: None }, card::VEXEN),
    (Patch::PlatinumCheck, card::PLATINUM),
    (Patch::PlatinumLoss, card::PLATINUM),
    (Patch::Ansem, card::ANSEM),
];

/// A small x86-64 writer for the stubs: short jumps to labels, rel32 to absolute targets.
struct Asm {
    at: u64,
    v: Vec<u8>,
    /// (offset of the rel8 byte, label).
    fix8: Vec<(usize, usize)>,
    labels: Vec<Option<usize>>,
}

impl Asm {
    fn new(at: u64) -> Asm {
        Asm { at, v: Vec::with_capacity(64), fix8: Vec::new(), labels: Vec::new() }
    }
    fn b(&mut self, bytes: &[u8]) {
        self.v.extend_from_slice(bytes);
    }
    fn label(&mut self) -> usize {
        self.labels.push(None);
        self.labels.len() - 1
    }
    fn bind(&mut self, l: usize) {
        self.labels[l] = Some(self.v.len());
    }
    /// A short jump (`op` = 0xEB or a Jcc rel8 opcode) to label `l`.
    fn j8(&mut self, op: u8, l: usize) {
        self.v.push(op);
        self.fix8.push((self.v.len(), l));
        self.v.push(0);
    }
    /// The rip-relative disp32 to `target` for an instruction whose disp32 starts now and which
    /// ends `tail` bytes after it.
    fn rip(&mut self, target: u64, tail: usize) -> Option<()> {
        let next = self.at + self.v.len() as u64 + 4 + tail as u64;
        let d = i32::try_from(target as i64 - next as i64).ok()?;
        self.b(&d.to_le_bytes());
        Some(())
    }
    fn imm64(&mut self, x: u64) {
        self.b(&x.to_le_bytes());
    }
    fn jmp_abs(&mut self, to: u64) -> Option<()> {
        let from = self.at + self.v.len() as u64;
        let j = crate::hookplan::rel_jmp(from, to)?;
        self.b(&j);
        Some(())
    }
    fn finish(mut self) -> Option<Vec<u8>> {
        for (at, l) in std::mem::take(&mut self.fix8) {
            let to = self.labels[l]?;
            let rel = to as i64 - (at as i64 + 1);
            self.v[at] = i8::try_from(rel).ok()? as u8;
        }
        (self.v.len() <= crate::hookplan::BLOCK).then_some(self.v)
    }
}

/// Where a stub's code and data go: the block address `at`, [`Armed`]'s address, the image base.
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub at: u64,
    pub armed: u64,
    pub base: u64,
}

/// The stub for `PATCH[i]` (`STUBS[i]`) placed at `p.at`: Re:CoM's moved instructions and the
/// skill's outcome, then a jump back after the site. None when something is out of reach or the
/// stub would pass the block's 64 bytes.
pub fn stub(i: usize, p: Place) -> Option<Vec<u8>> {
    let site = PATCH.get(i)?;
    let (kind, _) = STUBS.get(i)?;
    let back = p.base + site.rva as u64 + site.expect.len() as u64;
    let main = p.base + MAIN_PLAYER_RVA as u64;
    let mut a = Asm::new(p.at);
    match *kind {
        Patch::Slot { reg, id, flag, player } => {
            let done = a.label();
            a.b(site.expect); // movzx R, byte [rax+0x1E8] (no rip operand)
            match reg {
                Reg::Eax => a.b(&[0x3C, 0x43]),       // cmp al, 0x43
                Reg::Ecx => a.b(&[0x80, 0xF9, 0x43]), // cmp cl, 0x43
            }
            a.j8(0x77, done); // ja done (an enemy card: Re:CoM's)
            a.b(&[0x50, 0x48, 0xB8]); // push rax; mov rax, imm64
            a.imm64(p.armed);
            a.b(&[0x80, 0x78, flag as u8, 0x00]); // cmp byte [rax+flag], 0
            a.b(&[0x58]); // pop rax
            a.j8(0x74, done); // je done
            if let Some(r) = player {
                a.b(&[0x48, 0x3B, if r == Reg64::Rbx { 0x1D } else { 0x3D }]); // cmp rbx/rdi, [rip+main]
                a.rip(main, 0)?;
                a.j8(0x75, done); // jne done
            }
            a.b(&[if reg == Reg::Eax { 0xB8 } else { 0xB9 }]); // mov eax/ecx, id
            a.b(&(id as u32).to_le_bytes());
            a.bind(done);
            a.jmp_abs(back)?;
        }
        Patch::Dash => {
            let moved = a.label();
            let k = a.label();
            a.b(&[0x48, 0x85, 0xC0]); // test rax, rax (the card battle: battle only)
            a.j8(0x74, moved);
            a.b(&[0x48, 0x3B, 0x0D]); // cmp rcx, [rip+main]
            a.rip(main, 0)?;
            a.j8(0x75, moved);
            a.b(&[0x48, 0xB8]); // mov rax, imm64 (free: the join's next read overwrites it)
            a.imm64(p.armed);
            a.b(&[0x80, 0x78, ix::LARXENE as u8, 0x00]); // cmp byte [rax+LARXENE], 0
            a.j8(0x74, moved);
            // mulss xmm2, [rip+k]: k is a label inside the block (fixed below).
            a.b(&[0xF3, 0x0F, 0x59, 0x15]);
            let k_disp = a.v.len();
            a.b(&[0, 0, 0, 0]);
            a.bind(moved);
            a.b(&[0xF3, 0x0F, 0x10, 0x05]); // movss xmm0, [rip+threshold] (Re:CoM's, re-aimed)
            a.rip(p.base + WALK_THRESHOLD_RVA as u64, 0)?;
            a.jmp_abs(back)?;
            a.bind(k);
            a.b(&DASH.to_le_bytes());
            let to = a.labels[k]?;
            let d = (to as i64 - (k_disp as i64 + 4)) as i32;
            a.v[k_disp..k_disp + 4].copy_from_slice(&d.to_le_bytes());
        }
        Patch::PlatinumCheck => {
            let moved = a.label();
            a.b(&[0x44, 0x39, 0xC3]); // cmp ebx, r8d (the HP after against before)
            a.j8(0x7D, moved); // jge moved (no HP lost)
            a.b(&[0x48, 0x3B, 0x3D]); // cmp rdi, [rip+main]
            a.rip(main, 0)?;
            a.j8(0x75, moved);
            a.b(&[0x48, 0xB8]); // mov rax, imm64 (free: the moved instruction loads it)
            a.imm64(p.armed + PLATINUM_LEFT as u64);
            a.b(&[0x83, 0x38, 0x00]); // cmp dword [rax], 0
            a.j8(0x7E, moved); // jle moved
            a.b(&[0xF0, 0xFF, 0x08]); // lock dec dword [rax]
            a.b(&[0x44, 0x89, 0xC3]); // mov ebx, r8d (Re:CoM's `cmove ebx, r8d`: HP kept)
            a.bind(moved);
            a.b(site.expect); // mov rax, [rdi+0x3E8]
            a.jmp_abs(back)?;
        }
        Patch::PlatinumLoss => {
            let done = a.label();
            a.b(&[0x50]); // push rax
            a.b(&[0x3B, 0xBB, 0x2C, 0x04, 0x00, 0x00]); // cmp edi, [rbx+0x42C] (HP after against before)
            a.j8(0x7D, done);
            a.b(&[0x48, 0x3B, 0x1D]); // cmp rbx, [rip+main]
            a.rip(main, 0)?;
            a.j8(0x75, done);
            a.b(&[0x48, 0xB8]);
            a.imm64(p.armed + PLATINUM_LEFT as u64);
            a.b(&[0x83, 0x38, 0x00]); // cmp dword [rax], 0
            a.j8(0x7E, done);
            a.b(&[0xF0, 0xFF, 0x08]); // lock dec dword [rax]
            a.b(&[0x8B, 0xBB, 0x2C, 0x04, 0x00, 0x00]); // mov edi, [rbx+0x42C] (Re:CoM's `mov edi, edx`: HP kept)
            a.bind(done);
            a.b(&[0x58]); // pop rax
            a.b(site.expect); // test edi,edi; cmovns esi,edi; mov [rbx+0x42C],esi
            a.jmp_abs(back)?;
        }
        Patch::Ansem => {
            let done = a.label();
            a.b(site.expect); // cvttss2si r14d, xmm0
            a.b(&[0x48, 0x3B, 0x3D]); // cmp rdi, [rip+main]
            a.rip(main, 0)?;
            a.j8(0x75, done);
            a.b(&[0x50, 0x48, 0xB8]); // push rax; mov rax, imm64
            a.imm64(p.armed);
            a.b(&[0x80, 0x78, ix::ANSEM as u8, 0x00]); // cmp byte [rax+ANSEM], 0
            a.b(&[0x58]); // pop rax
            a.j8(0x74, done);
            a.b(&[0x83, 0xFB, 0x02]); // cmp ebx, 2 (the attribute)
            a.j8(0x7C, done); // jl
            a.b(&[0x83, 0xFB, 0x04]); // cmp ebx, 4
            a.j8(0x7F, done); // jg
            a.b(&[0x41, 0x83, 0xFE, DEFAULT_PERCENT as u8]); // cmp r14d, 100
            a.j8(0x75, done);
            a.b(&[0x41, 0xBE]); // mov r14d, 50
            a.b(&(ANSEM_PERCENT as u32).to_le_bytes());
            a.bind(done);
            a.jmp_abs(back)?;
        }
    }
    a.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: u64 = 0x1_4000_0000;
    const AT: u64 = 0x1_3000_0000;
    const ARMED: u64 = 0x7FF8_1234_5670;

    fn place() -> Place {
        Place { at: AT, armed: ARMED, base: BASE }
    }

    /// The rel32 jump at the end of `code` (from `at + off`) lands on `to`.
    fn jumps_to(code: &[u8], off: usize, to: u64) -> bool {
        code[off] == 0xE9 && (AT as i64 + off as i64 + 5 + i32::from_le_bytes(code[off + 1..off + 5].try_into().unwrap()) as i64) as u64 == to
    }

    fn find(code: &[u8], pat: &[u8]) -> Option<usize> {
        code.windows(pat.len()).position(|w| w == pat)
    }

    /// Every short jump lands inside the stub (on an instruction we placed: checked by offsets below).
    fn rel8_targets(code: &[u8], ops: &[(usize, u8)]) -> Vec<usize> {
        ops.iter()
            .map(|&(o, op)| {
                assert_eq!(code[o], op, "opcode at {o}");
                (o as i64 + 2 + code[o + 1] as i8 as i64) as usize
            })
            .collect()
    }

    #[test]
    fn every_stub_fits_and_jumps_back() {
        for (i, s) in PATCH.iter().enumerate() {
            let code = stub(i, place()).unwrap_or_else(|| panic!("{}", s.name));
            assert!(code.len() <= crate::hookplan::BLOCK, "{}: {}", s.name, code.len());
            assert!(s.expect.len() >= 5, "{}", s.name);
            let back = BASE + s.rva as u64 + s.expect.len() as u64;
            let j = if matches!(STUBS[i].0, Patch::Dash) { code.len() - 4 - 5 } else { code.len() - 5 };
            assert!(jumps_to(&code, j, back), "{}: back", s.name);
            // The armed table's address is in it (all but none).
            assert!(find(&code, &ARMED.to_le_bytes()).is_some() || find(&code, &(ARMED + PLATINUM_LEFT as u64).to_le_bytes()).is_some(), "{}", s.name);
        }
        assert_eq!(PATCH.len(), STUBS.len());
        // Out of rel32 reach: None.
        assert!(stub(0, Place { at: 0x7FF0_0000_0000, armed: ARMED, base: BASE }).is_none());
    }

    /// Axel's slot read: Re:CoM's movzx first, its id in ecx when armed and Sora's (rdi).
    #[test]
    fn slot_read_stub_layout() {
        let c = stub(0, place()).unwrap();
        assert_eq!(&c[..7], PATCH[0].expect);
        assert_eq!(&c[7..10], &[0x80, 0xF9, 0x43]);
        assert_eq!(&c[12..15], &[0x50, 0x48, 0xB8]);
        assert_eq!(&c[15..23], &ARMED.to_le_bytes());
        assert_eq!(&c[23..27], &[0x80, 0x78, ix::AXEL as u8, 0x00]);
        assert_eq!(c[27], 0x58);
        // cmp rdi, [rip+main]: lands on the main player.
        assert_eq!(&c[30..33], &[0x48, 0x3B, 0x3D]);
        let d = i32::from_le_bytes(c[33..37].try_into().unwrap()) as i64;
        assert_eq!((AT as i64 + 37 + d) as u64, BASE + MAIN_PLAYER_RVA as u64);
        assert_eq!(&c[39..44], &[0xB9, card::AXEL, 0, 0, 0]);
        let done = 44;
        assert_eq!(rel8_targets(&c, &[(10, 0x77), (28, 0x74), (37, 0x75)]), vec![done, done, done]);
        // The Vexen reads: eax / ecx, with and without the player check.
        let ko = stub(2, place()).unwrap();
        assert_eq!(&ko[7..9], &[0x3C, 0x43]);
        assert_eq!(&ko[ko.len() - 10..ko.len() - 5], &[0xB8, card::VEXEN, 0, 0, 0]);
        assert!(find(&ko, &[0x48, 0x3B, 0x1D]).is_some(), "the KO init checks rbx");
        for i in [3, 4] {
            let v = stub(i, place()).unwrap();
            assert!(find(&v, &[0x48, 0x3B]).is_none(), "{}: no player check", PATCH[i].name);
            assert_eq!(v[v.len() - 10], if i == 3 { 0xB8 } else { 0xB9 });
            assert_eq!(v[v.len() - 9], card::VEXEN);
        }
    }

    #[test]
    fn dash_stub_layout() {
        let c = stub(1, place()).unwrap();
        assert_eq!(&c[..3], &[0x48, 0x85, 0xC0]);
        // The factor in the block's tail, and mulss reading it.
        assert_eq!(&c[c.len() - 4..], &DASH.to_le_bytes());
        let m = find(&c, &[0xF3, 0x0F, 0x59, 0x15]).unwrap();
        let d = i32::from_le_bytes(c[m + 4..m + 8].try_into().unwrap()) as usize;
        assert_eq!(m + 8 + d, c.len() - 4);
        // The moved movss reads Re:CoM's 0.9 at its own address.
        let mv = m + 8;
        assert_eq!(&c[mv..mv + 4], &[0xF3, 0x0F, 0x10, 0x05]);
        let d = i32::from_le_bytes(c[mv + 4..mv + 8].try_into().unwrap()) as i64;
        assert_eq!((AT as i64 + mv as i64 + 8 + d) as u64, BASE + WALK_THRESHOLD_RVA as u64);
        // The exe's own rip operand points at the same threshold.
        let s = &PATCH[1];
        let e = i32::from_le_bytes(s.expect[4..8].try_into().unwrap()) as i64;
        assert_eq!((BASE as i64 + s.rva as i64 + 8 + e) as u64, BASE + WALK_THRESHOLD_RVA as u64);
        // Every short jump lands on the moved movss.
        let jumps: Vec<(usize, u8)> = [(3usize, 0x74u8), (12, 0x75), (28, 0x74)].to_vec();
        assert!(rel8_targets(&c, &jumps).iter().all(|&t| t == mv));
    }

    #[test]
    fn platinum_stubs_layout() {
        let c = stub(5, place()).unwrap();
        assert_eq!(&c[..3], &[0x44, 0x39, 0xC3]);
        let mv = c.len() - 5 - 7;
        assert_eq!(&c[mv..mv + 7], PATCH[5].expect);
        assert!(find(&c, &(ARMED + PLATINUM_LEFT as u64).to_le_bytes()).is_some());
        assert_eq!(&c[mv - 6..mv], &[0xF0, 0xFF, 0x08, 0x44, 0x89, 0xC3]);
        assert!(rel8_targets(&c, &[(3, 0x7D), (12, 0x75), (27, 0x7E)]).iter().all(|&t| t == mv));
        let l = stub(6, place()).unwrap();
        assert_eq!(l[0], 0x50);
        let pop = l.len() - 5 - 11 - 1;
        assert_eq!(l[pop], 0x58);
        assert_eq!(&l[pop + 1..pop + 12], PATCH[6].expect);
        assert_eq!(&l[pop - 9..pop], &[0xF0, 0xFF, 0x08, 0x8B, 0xBB, 0x2C, 0x04, 0x00, 0x00]);
        assert!(rel8_targets(&l, &[(7, 0x7D), (16, 0x75), (31, 0x7E)]).iter().all(|&t| t == pop));
    }

    #[test]
    fn ansem_stub_layout() {
        let c = stub(7, place()).unwrap();
        assert_eq!(&c[..5], PATCH[7].expect);
        let tail = c.len() - 5;
        assert_eq!(&c[tail - 6..tail], &[0x41, 0xBE, ANSEM_PERCENT as u8, 0, 0, 0]);
        assert_eq!(&c[tail - 12..tail - 8], &[0x41, 0x83, 0xFE, DEFAULT_PERCENT as u8]);
        let js: Vec<(usize, u8)> = vec![(12, 0x75), (30, 0x74), (35, 0x7C), (40, 0x7F), (46, 0x75)];
        assert!(rel8_targets(&c, &js).iter().all(|&t| t == tail));
    }

    #[test]
    fn rules() {
        // Platinum: the first 5 damaging hits keep the HP; a 0-damage hit takes none.
        let mut left = PLATINUM_HITS;
        let mut kept = 0;
        for _ in 0..8 {
            let (hp, l) = platinum(100, 90, left, true);
            kept += (hp == 100) as i32;
            left = l;
        }
        assert_eq!((kept, left), (5, 0));
        assert_eq!(platinum(100, 100, 3, true), (100, 3), "no HP lost: no hit taken");
        assert_eq!(platinum(100, 50, 3, false), (50, 3), "not Sora");
        // Ansem: Re:CoM's default 100 → 50 for Fire / Blizzard / Thunder only.
        for attr in 0..8 {
            assert_eq!(ansem(100, attr, true), if (2..=4).contains(&attr) { 50 } else { 100 });
        }
        assert_eq!(ansem(1000, 2, true), 1000, "another record stands");
        assert_eq!(ansem(100, 3, false), 100);
        // Dash: run × 1.25, not Re:CoM's 1.7.
        assert_eq!(dash_run(8.0, true), 10.0);
        assert_eq!(dash_run(8.0, false), 8.0);
        // Arming: KH1, equipped, placed.
        let a = arm(true, &|c| c != card::ANSEM, &|_| true);
        assert_eq!(a.flags, [1, 1, 1, 0]);
        assert_eq!(a.platinum_left, PLATINUM_HITS);
        assert_eq!(arm(false, &|_| true, &|_| true), Arming::default(), "Combat = Re:CoM or Riku's story");
        assert_eq!(arm(true, &|_| true, &|c| c != card::PLATINUM).platinum_left, 0, "stub not placed");
    }

    /// The affinity record's floats (`CHECKED`'s data site) are Ansem's 100 / 100 / 50 / 50 / 50 / 100.
    #[test]
    fn ansem_record_is_the_rule() {
        let s = CHECKED.iter().find(|s| s.rva == 0x007C_5C50).unwrap();
        let f: Vec<i32> = s.expect.chunks(4).map(|c| f32::from_le_bytes(c.try_into().unwrap()) as i32).collect();
        assert_eq!(f, vec![100, 100, ANSEM_PERCENT, ANSEM_PERCENT, ANSEM_PERCENT, 100]);
    }

    /// No two patched sites overlap, and none overlaps another module's patched or hooked bytes
    /// except the compares' context (checked only).
    #[test]
    fn sites_apart() {
        let r = |s: &Site| s.rva..s.rva + s.expect.len() as u32;
        let ours: Vec<&Site> = PATCH.iter().chain(CALLS).collect();
        for (i, a) in ours.iter().enumerate() {
            for b in &ours[i + 1..] {
                assert!(r(a).end <= r(b).start || r(b).end <= r(a).start, "{} / {}", a.name, b.name);
            }
        }
        let same = |a: &Site, b: &Site| a.rva == b.rva && a.name == b.name;
        for s in crate::sites::all_sites() {
            if ours.iter().any(|o| same(o, s)) || CHECKED.iter().any(|o| same(o, s)) {
                continue;
            }
            for o in &ours {
                let (a, b) = (r(o), r(s));
                if a.start < b.end && b.start < a.end {
                    // Only Second Chance's context (checked before it patches; ours install after it).
                    assert_eq!(s.name, "damage check: HP before to the card rules (0x1401E71A2)", "{} overlaps {}", o.name, s.name);
                }
            }
        }
    }
}
