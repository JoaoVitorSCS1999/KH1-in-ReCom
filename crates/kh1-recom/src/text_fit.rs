//! True squeeze to fit for Re:CoM's own text (owner, 2026-11-04, "M13 and camp text answers":
//! "Camp-page long names (Customize, Abilities, Re:CoM's text): (a) true squeeze, an x-scale
//! taught to Re:CoM's text drawing"). Notes: `docs/recom-m12-sleights.md` § 8.8.
//!
//! Re:CoM's text object `D2::C2DFnt` (0xB0 bytes, vtable `0x140606C98`) draws through its vtable
//! `+0x18` (`0x140289450` → `0x14025AA20`) and measures through `+0x28` (`0x140289440` →
//! `0x14025A380`). Its draw takes every glyph's width, height and advance from one size (`+0x3C`,
//! read once at `0x14025AC3B`) and has no width-only scale (Checked (decomp + disasm), 0.107.0).
//!
//! What this module teaches it, for the texts **our** code sets a scale on (everything else keeps
//! Re:CoM's vtable and runs exactly as before):
//!
//! - **The scale lives in the text's vtable.** A squeezed text's vtable pointer is one of our
//!   copies of Re:CoM's C2DFnt vtable ([`vtable::copy`]): Re:CoM's COL and its 7 slots, with `+0x18`
//!   and `+0x28` our wrappers, and the scale (1/1024 steps) two words before the slots. No field of
//!   Re:CoM's object is used: the scale travels with the vtable pointer, and Re:CoM's own dtor
//!   (slot `+0`, kept) replaces the pointer when the text is deleted, so nothing goes stale. Scale
//!   1.0 is Re:CoM's own vtable (never a copy).
//! - **Draw** (our `+0x18`): Re:CoM's draw runs unchanged; then the x of every vertex it wrote into
//!   its packet (Re:CoM's `XYZ2` pairs, register 5, x in bits 0..16 + 0x8000, `0x14025A0A0`) is
//!   scaled about the text's x (`+0x28`): [`packet::squeeze`]. Glyphs, spaces, letter spacing,
//!   button icons and the outline passes all narrow together (a true geometric squeeze); the
//!   left edge (left-aligned), right edge (right-aligned) or centre (centred) stays where Re:CoM
//!   puts it, since Re:CoM aligns from `+0x28` (`0x14025AC8E..0x14025ACCB`).
//! - **Measure** (our `+0x28`): Re:CoM's width routine, then the width × the scale (the drawn
//!   width). Its per-line table (`r8`) stays in pen space: Re:CoM's draw reads such a table from
//!   `+0x80` and our draw scales its result again.
//! - **Who sets it**: each of our boxes measures its text with Re:CoM's width routine at scale 1
//!   and sets [`level`] = `min(1, box / width)` (floored, so it always ends inside the box).

/// Re:CoM's side (Steam RVAs; `sites::TEXT_FIT_CHECKED` / `TEXT_FIT_PTRS`). Checked (decomp +
/// disasm + exe).
pub mod recom {
    /// `D2::C2DFnt`'s vtable (its COL at −8).
    pub const VTABLE: u32 = 0x0060_6C98;
    /// Its draw (`+0x18`: no text → nothing; else `0x14025AA20`) and measure (`+0x28` → `0x14025A380`).
    pub const DRAW: u32 = 0x0028_9450;
    pub const MEASURE: u32 = 0x0028_9440;
    /// The text-width routine `(int out[2], fnt, int* lines, int* count) -> lines`: out = width,
    /// height in 1/16 units; nothing written (0 returned) without text or a loaded font.
    pub const WIDTH: u32 = 0x0025_A380;
    /// The draw itself.
    pub const DRAW_BODY: u32 = 0x0025_AA20;
    /// The packet buffer's holder `0x140882DD8` (a pointer to a struct whose `+0` is the write
    /// pointer), read by the packet open `0x140281180` and close `0x140280F30`.
    pub const PACKET_HOLDER: u32 = 0x0088_2DD8;
    /// `C2DFnt` fields (`0x140258BC0`): x (×16; the draw's origin), the text pointer.
    pub const X: usize = 0x28;
    pub const TEXT: usize = 0x58;
    /// The draw's text copy `(src, dst)`: the button escapes expanded into the draw's 2048-byte
    /// buffer (`0x14025AC2B`: `mov rcx,[r15+0x58]; lea rdx,[rbp+0xD0]; call`).
    pub const EXPAND: u32 = 0x0034_80C0;
    pub const EXPAND_BUF: usize = 2048;
    /// `C2DFnt`'s size (`0x140112F10` allocates 0xB0; CFntM2 is 0xA8 and C2DFnt adds `+0xA8`).
    pub const FNT_SIZE: usize = 0xB0;
    /// `CButton +0x20`: the row's `C2DFnt` (`0x140112F10` makes it, 0xB0 bytes).
    pub const BUTTON_TEXT: usize = 0x20;

    /// `CStrPrim` (0.132.0): the text of a `CTextWnd` (the Moogle's speech bubble), a `CFntM2`
    /// (0xA8, made by `0x140258CB0` in `0x1402B1D50`) with its own vtable `0x140508FD0` (COL at
    /// −8 `0x14070B5E8`): the same draw `+0x18` and measure `+0x28` as C2DFnt's, its own dtor
    /// `+0` `0x14029A080` and `+0x30` `0x140258E80`. Every code reference to the vtable is a store
    /// (13 `lea` + `mov [reg], rax`: its ctors and the dtors of it and its subclasses); no absolute
    /// pointer to it in the image (Checked (exe scan)).
    pub const STR_VTABLE: u32 = 0x0050_8FD0;
    pub const STR_COL: u32 = 0x0070_B5E8;
    /// C2DFnt's COL.
    pub const COL: u32 = 0x0071_8CF8;
    /// `CFntM2`'s size (both classes start with it; the width routine reads `+0x3C`, `+0x40`,
    /// `+0x44`, `+0x58` only).
    pub const FNTM2_SIZE: usize = 0xA8;
    /// `CFntM2`: the font size and the line gap (×16; `0x1402B1E8C` / `0x1402B1EAA`).
    pub const SIZE: usize = 0x3C;
    pub const GAP: usize = 0x48;

    /// `CTextWnd` (`0x1402AFF30`, 0x148 bytes) as its open `0x1402B1520` sets it from its message's
    /// record (`0x140240080`): `+0xB0` / `+0xB4` the text's margins (record `+0x28` / `+0x2A`,
    /// `0x1402B15B5`), `+0xB8` / `+0xBC` the bubble's size (record `+0x1A` / `+0x1C` rounded up to
    /// even, `0x1402B1FAF`; the bubble `CWnd` drawn at that size), `+0xD0` its `CText` (`0x1402B3AD0`:
    /// placed at the window + the margins, its size the window's). Re:CoM's text call `0x1402B0EF0`
    /// sets a new line on the same `CText` and never resizes the bubble.
    pub const WND_MARGINS: usize = 0xB0;
    pub const WND_SIZE: usize = 0xB8;
    pub const WND_TEXT: usize = 0xD0;
    /// `CText`: `+0x38` its `CStrPrim`, `+0x54` the line count of its text (the measure's return,
    /// `0x1402B1EC1`).
    pub const CTEXT_FNT: usize = 0x38;
    pub const CTEXT_LINES: usize = 0x54;
}

/// Scale 1.0 in the scale's 1/1024 steps.
pub const ONE: u32 = 1024;
/// The narrowest scale set (0.25): a text that would need less is drawn at 0.25 and overflows
/// (logged). Bounds the vtable copies to `ONE - MIN` levels.
pub const MIN: u32 = 256;

/// The scale level of a text `width16` (Re:CoM's 1/16 units) wide in a box `box_units` wide:
/// `floor(min(1, box / width) × 1024)`, at least [`MIN`]; [`ONE`] when it fits or either is
/// unknown.
pub fn level(width16: i32, box_units: f32) -> u32 {
    if width16 <= 0 || !(box_units > 0.0) {
        return ONE;
    }
    let b = (box_units * 16.0) as i64;
    if width16 as i64 <= b {
        return ONE;
    }
    let l = (b * ONE as i64 / width16 as i64) as u32;
    l.clamp(MIN, ONE - 1)
}

/// `v` × the scale, floored (Re:CoM's 1/16 units).
pub fn scaled(v: i64, level: u32) -> i64 {
    (v * level as i64) >> 10
}

/// A level as a factor (logs).
pub fn factor(level: u32) -> f32 {
    level as f32 / ONE as f32
}

/// One of Re:CoM's `XYZ2` words with its x scaled about `origin16` (the text's `+0x28`, 1/16
/// units): x is bits 0..16, stored + 0x8000 (`0x14025A286`: `lea eax,[rbp+0x8000]; and
/// eax,0x1FFFF`); y and the rest untouched.
pub fn squeeze_x(xyz: u64, origin16: i32, level: u32) -> u64 {
    let x = (xyz & 0x1_FFFF) as i64 - 0x8000;
    let o = origin16 as i64;
    let nx = o + scaled(x - o, level);
    (xyz & !0x1_FFFF) | ((nx + 0x8000) as u64 & 0x1_FFFF)
}

/// Re:CoM's C2DFnt draw packet (`0x14025AA20`): 16-byte (data, register) pairs from the buffer's
/// write pointer (`*[0x140882DD8]`) before the draw. Checked (disasm `0x14025AD01..0x14025AD5A`,
/// `0x14025A0A0`, `0x14025B719..0x14025B73B`).
pub mod packet {
    /// Qword 0..1: the GIF tag (`0x140281180` reserves it, `0x140281E40` writes it at the end).
    /// Then the header: (5, 9), (font register word, 0x17), (0x356, 0) (PRIM), (colour, 1).
    pub const HEADER: [(usize, u64); 6] = [(2, 5), (3, 9), (5, 0x17), (6, 0x356), (7, 0), (9, 1)];
    /// The first glyph pair.
    pub const FIRST: usize = 10;
    /// `XYZ2`: each glyph quad writes (TEX0, 7), (UV, 3), (XYZ, 5), (UV, 3), (XYZ, 5).
    pub const XYZ: u64 = 5;

    /// Scales the x of every `XYZ2` pair of a C2DFnt draw's packet `q` (its qwords, from the write
    /// pointer before the draw to after) about `origin16`. Refused, untouched, unless `q` starts
    /// with Re:CoM's C2DFnt header. The vertex count.
    pub fn squeeze(q: &mut [u64], origin16: i32, level: u32) -> Result<usize, &'static str> {
        if q.len() < FIRST {
            return Err("shorter than the C2DFnt header");
        }
        if HEADER.iter().any(|&(i, v)| q[i] != v) {
            return Err("not a C2DFnt draw's header");
        }
        let mut n = 0;
        let mut k = FIRST;
        while k + 1 < q.len() {
            if q[k + 1] == XYZ {
                q[k] = super::squeeze_x(q[k], origin16, level);
                n += 1;
            }
            k += 2;
        }
        Ok(n)
    }
}

/// Our copies of Re:CoM's C2DFnt vtable (`0x140606C98`: COL at −8, 7 slots; the next word is the
/// next vtable's COL. Checked (exe)).
pub mod vtable {
    /// Re:CoM's slots: `+0` deleting dtor `0x140243FB0`, `+8` register `0x140261D70`, `+0x10` packet
    /// size `0x140289430`, `+0x18` draw `0x140289450`, `+0x20` depth `0x1400D3D00`, `+0x28` measure
    /// `0x140289440`, `+0x30` `0x140243FF0`.
    pub const SLOTS: usize = 7;
    /// The draw slot (`+0x18`).
    pub const DRAW: usize = 3;
    /// The measure slot (`+0x28`).
    pub const MEASURE: usize = 5;
    /// Words of a copy: our marker, the level, Re:CoM's COL, the slots.
    pub const WORDS: usize = 3 + SLOTS;
    /// The word the text's vtable pointer points at (slot 0).
    pub const VPTR_AT: usize = 3;
    /// Our marker, two words before the slots.
    pub const MARKER: u64 = u64::from_le_bytes(*b"KH1TXSQZ");

    /// A copy for `level`: Re:CoM's COL and slots, draw and measure ours.
    pub fn copy(level: u32, col: u64, recom: [u64; SLOTS], draw: u64, measure: u64) -> [u64; WORDS] {
        let mut w = [0u64; WORDS];
        w[0] = MARKER;
        w[1] = level as u64;
        w[2] = col;
        w[VPTR_AT..].copy_from_slice(&recom);
        w[VPTR_AT + DRAW] = draw;
        w[VPTR_AT + MEASURE] = measure;
        w
    }

    /// The level of one of our copies, from the two words before its COL (`before` = the words at
    /// vptr − 0x18 and vptr − 0x10); None for any other vtable.
    pub fn level_of(before: [u64; 2]) -> Option<u32> {
        (before[0] == MARKER && (super::MIN as u64..super::ONE as u64).contains(&before[1])).then_some(before[1] as u32)
    }
}

/// The level of a text `width16` wide in a box `box16` wide (both Re:CoM's 1/16 units): as
/// [`level`].
pub fn level16(width16: i64, box16: i64) -> u32 {
    if width16 <= 0 || box16 <= 0 || width16 <= box16 {
        return ONE;
    }
    ((box16 * ONE as i64 / width16) as u32).clamp(MIN, ONE - 1)
}

/// Our lines in a box of a few lines (the Moogle's speech bubble, `win::shop`): wrapped onto the
/// box's lines the way Re:CoM's own messages are (a `\n` between lines: Re:CoM's width routine
/// `0x14025A380` and draw start a new line at byte 10; Checked (decomp)), squeezed (the true
/// squeeze, [`level`]) only when no wrap fits.
pub mod wrap {
    use super::level16;

    /// A text laid out for a box.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Laid {
        /// The lines, `\n` between them.
        pub text: String,
        pub lines: usize,
        /// The x scale (1/1024): [`super::ONE`] when every line fits as it is.
        pub level: u32,
        /// The widest line at scale 1 (1/16 units).
        pub widest: i64,
    }

    /// Greedy word wrap of `para` (single spaces between words) at `w16`: a word goes on the line
    /// while the line stays within `w16`; a word wider than `w16` gets a line of its own. The
    /// lines and the widest's width; None when a width is unknown.
    fn para(para: &str, w16: i64, m: &mut dyn FnMut(&str) -> Option<i64>) -> Option<(Vec<String>, i64)> {
        let mut lines: Vec<String> = Vec::new();
        let mut widest = 0;
        let mut cur = String::new();
        let mut cur_w = 0;
        for word in para.split(' ').filter(|w| !w.is_empty()) {
            if cur.is_empty() {
                cur = word.to_string();
                cur_w = m(&cur)?;
                continue;
            }
            let try_line = format!("{cur} {word}");
            let w = m(&try_line)?;
            if w <= w16 {
                cur = try_line;
                cur_w = w;
            } else {
                widest = widest.max(cur_w);
                lines.push(std::mem::replace(&mut cur, word.to_string()));
                cur_w = m(&cur)?;
            }
        }
        widest = widest.max(cur_w);
        lines.push(cur);
        Some((lines, widest))
    }

    /// `paras` wrapped at `w16`, one after the other.
    fn paras(paras: &[&str], w16: i64, m: &mut dyn FnMut(&str) -> Option<i64>) -> Option<(Vec<String>, i64)> {
        let mut all = Vec::new();
        let mut widest = 0;
        for p in paras {
            let (l, w) = para(p, w16, m)?;
            all.extend(l);
            widest = widest.max(w);
        }
        Some((all, widest))
    }

    /// The narrowest wrap width (≥ `box16`) at which `ps` takes at most `max` lines, and that wrap.
    /// Re:CoM's widths only grow as words are added, so the line count only falls as the width
    /// grows (a binary search on the width, 1/16 units).
    fn narrowest(ps: &[&str], box16: i64, max: usize, m: &mut dyn FnMut(&str) -> Option<i64>) -> Option<(Vec<String>, i64)> {
        let mut hi = ps.iter().map(|p| m(p)).collect::<Option<Vec<i64>>>()?.into_iter().max().unwrap_or(0).max(box16);
        let mut best = paras(ps, hi, m)?;
        if best.0.len() > max {
            return None;
        }
        let mut lo = box16;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let got = paras(ps, mid, m)?;
            if got.0.len() <= max {
                hi = mid;
                best = got;
            } else {
                lo = mid + 1;
            }
        }
        Some(best)
    }

    fn laid(lines: Vec<String>, widest: i64, box16: i64) -> Laid {
        Laid { lines: lines.len(), text: lines.join("\n"), level: level16(widest, box16), widest }
    }

    /// `text` (our line; a `\n` marks where we prefer a new line) for a box `box16` wide (1/16
    /// units) holding `max_lines` lines, `m` Re:CoM's width of a line:
    ///
    /// 1. our lines, each wrapped at the box (the way Re:CoM's own two-line lines read);
    /// 2. else the whole text wrapped at the box (our breaks dropped);
    /// 3. else squeezed: the narrowest wrap that takes `max_lines` lines, our breaks kept when
    ///    there are no more of our lines than the box holds (they read as Re:CoM's: what the item
    ///    does, then its price), drawn at box / its widest line (the true squeeze).
    ///
    /// None when a width is unknown (Re:CoM's font not loaded): the caller keeps the text as is.
    pub fn layout(text: &str, box16: i64, max_lines: usize, m: &mut dyn FnMut(&str) -> Option<i64>) -> Option<Laid> {
        let max = max_lines.max(1);
        let ours: Vec<&str> = text.split('\n').collect();
        let joined = ours.join(" ");
        let (l, w) = paras(&ours, box16, m)?;
        if l.len() <= max && w <= box16 {
            return Some(laid(l, w, box16));
        }
        let (l, w) = para(&joined, box16, m)?;
        if l.len() <= max && w <= box16 {
            return Some(laid(l, w, box16));
        }
        let kept = if ours.len() <= max { narrowest(&ours, box16, max, m) } else { None };
        let best = match kept {
            Some(k) => k,
            None => narrowest(&[joined.as_str()], box16, max, m)?,
        };
        Some(laid(best.0, best.1, box16))
    }

    /// The lines a box holds: `n` lines of a `size16` font with `gap16` between them take
    /// `n × size + (n − 1) × gap` (Re:CoM's width routine's height, `0x14025A380`); at least 1.
    pub fn lines_in(height16: i64, size16: i64, gap16: i64) -> usize {
        if size16 <= 0 || height16 <= 0 {
            return 1;
        }
        (((height16 + gap16.max(0)) / (size16 + gap16.max(0))).max(1)) as usize
    }

}

/// The boxes (units, from the text's left edge) of our texts drawn in Re:CoM's text.
pub mod boxes {
    /// Room kept before a box's edge.
    pub const MARGIN: f32 = 2.0;

    /// Camp list rows (`0x140112F10`'s row): the plate (sprite group 270 of `camp:00`, used by
    /// groups 0x12E / 0x131 / 0x133 and our item row) spans x 0..197; Re:CoM's cursor frame 0x12D
    /// −1..198 hugs it. Checked (files: the owner's `FO0100.SPR`).
    pub const CAMP_PLATE_RIGHT: f32 = 197.0;
    /// The row text's left edge: the group's first piece (a 1 × 1 sprite at (22, 14)) plus the
    /// text's offset (1, 2) (`0x1400E6DF0` mode 9; ours: `Page::redraw`). Assumption: that first
    /// piece is the text's anchor (the text then sits from y 1 to its bottom 16 inside the plate's
    /// −5..21, right of the row icon at 7..20); without it the text starts at 1 and this box
    /// squeezes 22 units more than needed.
    pub const CAMP_TEXT_LEFT: f32 = 23.0;
    /// 172.
    pub const CAMP_ROW: f32 = CAMP_PLATE_RIGHT - CAMP_TEXT_LEFT - MARGIN;

    /// The status panel's value: right-aligned (flags 0x41) at x 220 of its row, the label from
    /// x 0 of the same row sprite (`0x1400E6DF0` rows 0..4; ours `Page::stats`). Checked (decomp).
    pub const STATUS_RIGHT: f32 = 220.0;
    /// Room kept between the label's end and the value (ours).
    pub const STATUS_GAP: f32 = 8.0;
    /// A value's box beside a label `label` units wide.
    pub fn status(label: f32) -> f32 {
        STATUS_RIGHT - label.max(0.0) - STATUS_GAP
    }

    /// The command menu in Re:CoM's look: Re:CoM's text plate (`Cockpit:00` group 0x19's cap
    /// −13..13 and strip 0..152 at X translate 13: right edge 165), copied by the asset step
    /// without the group's first piece. Checked (files: the owner's `FO0002.SPR`; the asset step's
    /// plate groups).
    pub const COMMAND_PLATE_RIGHT: f32 = 165.0;
    /// The text's left edge: attached at (0, 0) with the screen offset −4 (`CStockFolder`'s,
    /// `0x140165BEC`). Checked (exe).
    pub const COMMAND_TEXT_LEFT: f32 = -4.0;
    /// 167.
    pub const COMMAND_ROW: f32 = COMMAND_PLATE_RIGHT - COMMAND_TEXT_LEFT - MARGIN;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xyz(x: i32, y: i32, rest: u64) -> u64 {
        (rest << 34) | ((((y + 0x8000) & 0x1_FFFF) as u64) << 17) | (((x + 0x8000) & 0x1_FFFF) as u64)
    }

    fn x_of(w: u64) -> i32 {
        (w & 0x1_FFFF) as i32 - 0x8000
    }

    #[test]
    fn the_level_fits_or_is_one() {
        // Fits (or unknown): Re:CoM's vtable, nothing changes.
        assert_eq!(level(172 * 16, boxes::CAMP_ROW), ONE);
        assert_eq!(level(100 * 16, boxes::CAMP_ROW), ONE);
        assert_eq!(level(0, 100.0), ONE);
        assert_eq!(level(-5, 100.0), ONE);
        assert_eq!(level(3000, 0.0), ONE);
        assert_eq!(level(3000, f32::NAN), ONE);
        // Too wide: floored, so the drawn width never passes the box.
        for w in (173 * 16..400 * 16).step_by(7) {
            let l = level(w, boxes::CAMP_ROW);
            assert!(l < ONE && l >= MIN);
            assert!(scaled(w as i64, l) <= (boxes::CAMP_ROW * 16.0) as i64, "{w}");
            // ... and is the widest that does (within one step).
            if l > MIN {
                assert!(scaled(w as i64, l + 1) > (boxes::CAMP_ROW * 16.0) as i64 - 16, "{w}");
            }
        }
        // Absurd widths stop at 0.25.
        assert_eq!(level(10_000 * 16, 10.0), MIN);
    }

    #[test]
    fn scale_one_is_the_identity_and_y_is_untouched() {
        for &(x, y) in &[(0, 0), (-300, 77), (8191, 6143), (-0x8000, 5), (0x17FFF, -3)] {
            let w = xyz(x, y, 0x1234);
            assert_eq!(squeeze_x(w, 4000, ONE), w, "{x}");
            assert_eq!(squeeze_x(w, x, 700), w, "the origin itself stays");
            let s = squeeze_x(w, 1000, 512);
            assert_eq!(s & !0x1_FFFF, w & !0x1_FFFF, "y and the rest untouched");
        }
    }

    #[test]
    fn squeezing_keeps_the_aligned_edge() {
        let o = 160 * 16;
        // Left-aligned: the pen starts at the origin; the first glyph stays, the far edge moves in.
        let left = [o, o + 9 * 16, o + 100 * 16];
        let got: Vec<i32> = left.iter().map(|&x| x_of(squeeze_x(xyz(x, 0, 0), o, 512))).collect();
        assert_eq!(got, [o, o + 72, o + 50 * 16]);
        // Right-aligned: Re:CoM starts the pen at origin − width; the right edge (origin) stays.
        let right = [o - 100 * 16, o - 10 * 16, o];
        let got: Vec<i32> = right.iter().map(|&x| x_of(squeeze_x(xyz(x, 0, 0), o, 512))).collect();
        assert_eq!(got, [o - 50 * 16, o - 5 * 16, o]);
        // Centred: the centre stays, both edges move in.
        let got: Vec<i32> = [o - 800, o + 800].iter().map(|&x| x_of(squeeze_x(xyz(x, 0, 0), o, 768))).collect();
        assert_eq!(got, [o - 600, o + 600]);
    }

    /// A packet as Re:CoM's draw writes it: GIF tag, header, two glyph quads, the closing pair.
    fn packet(xs: &[i32]) -> Vec<u64> {
        let mut q = vec![0x8000_0000_0000_0004, 0xE, 5, 9, 0x1234_5678, 0x17, 0x356, 0, 0x8080_8080, 1];
        for &x in xs {
            q.extend_from_slice(&[0xAAAA, 7, 0x0010_0010, 3, xyz(x, 100, 3), 5, 0x0020_0020, 3, xyz(x + 15 * 16, 116, 3), 5]);
        }
        q.extend_from_slice(&[0x9999, 9]);
        q
    }

    #[test]
    fn the_packet_squeeze_touches_only_vertex_x() {
        let o = 50 * 16;
        let mut q = packet(&[o, o + 15 * 16]);
        let before = q.clone();
        assert_eq!(packet::squeeze(&mut q, o, 512), Ok(4));
        for (i, (a, b)) in before.iter().zip(&q).enumerate() {
            if i >= packet::FIRST && i % 2 == 0 && before[i + 1] == packet::XYZ {
                assert_eq!(a & !0x1_FFFF, b & !0x1_FFFF);
            } else {
                assert_eq!(a, b, "word {i}");
            }
        }
        let xs: Vec<i32> = (packet::FIRST..q.len()).step_by(2).filter(|&i| q[i + 1] == 5).map(|i| x_of(q[i])).collect();
        assert_eq!(xs, [o, o + 120, o + 120, o + 240], "both quads halved about the origin");
        // Scale 1: byte for byte.
        let mut one = before.clone();
        assert_eq!(packet::squeeze(&mut one, o, ONE), Ok(4));
        assert_eq!(one, before);
        // An odd trailing word (Re:CoM's 8-byte pad) is not read as a pair.
        let mut padded = before.clone();
        padded.push(5);
        assert_eq!(packet::squeeze(&mut padded, o, 512), Ok(4));
    }

    #[test]
    fn a_packet_that_is_not_a_c2dfnt_draw_is_left_alone() {
        let mut q = packet(&[0]);
        q[3] = 8;
        let b = q.clone();
        assert!(packet::squeeze(&mut q, 0, 512).is_err());
        assert_eq!(q, b);
        assert!(packet::squeeze(&mut [0u64; 4], 0, 512).is_err());
        assert!(packet::squeeze(&mut [], 0, 512).is_err());
    }

    #[test]
    fn the_vtable_copy_is_recoms_but_draw_and_measure() {
        let recom = [0x1_4024_3FB0, 0x1_4026_1D70, 0x1_4028_9430, 0x1_4028_9450, 0x1_400D_3D00, 0x1_4028_9440, 0x1_4024_3FF0];
        let w = vtable::copy(800, 0x1_4071_8CF8, recom, 0xD1, 0xD2);
        assert_eq!(w[2], 0x1_4071_8CF8, "Re:CoM's COL right before the slots (RTTI)");
        for (i, &s) in recom.iter().enumerate() {
            let want = match i {
                vtable::DRAW => 0xD1,
                vtable::MEASURE => 0xD2,
                _ => s,
            };
            assert_eq!(w[vtable::VPTR_AT + i], want, "slot {i}");
        }
        assert_eq!(vtable::level_of([w[0], w[1]]), Some(800));
        assert_eq!(vtable::level_of([0, 800]), None);
        assert_eq!(vtable::level_of([vtable::MARKER, ONE as u64]), None, "scale 1 is never a copy");
        assert_eq!((vtable::DRAW * 8, vtable::MEASURE * 8), (0x18, 0x28));
    }

    /// A model of Re:CoM's width routine: 10 units a glyph, 5 a space (1/16 units).
    fn mono(s: &str) -> Option<i64> {
        Some(s.chars().map(|c| if c == ' ' { 80 } else { 160 }).sum())
    }

    #[test]
    fn wrap_keeps_our_lines_when_they_fit() {
        let m = &mut mono;
        // Fits on one line: as it is.
        let l = wrap::layout("Not enough Moogle Points.", 400 * 16, 2, m).unwrap();
        assert_eq!((l.text.as_str(), l.lines, l.level), ("Not enough Moogle Points.", 1, ONE));
        // Our two lines, each fitting.
        let l = wrap::layout("Restores HP.\n10 points. Sora has 11.", 220 * 16, 2, m).unwrap();
        assert_eq!((l.text.as_str(), l.lines, l.level), ("Restores HP.\n10 points. Sora has 11.", 2, ONE));
        // A line wider than the box wraps at a space.
        let l = wrap::layout("Sora can't carry any more of this.", 200 * 16, 2, m).unwrap();
        assert_eq!(l.text, "Sora can't carry any\nmore of this.");
        assert_eq!(l.level, ONE);
        for line in l.text.split('\n') {
            assert!(mono(line).unwrap() <= 200 * 16);
        }
    }

    #[test]
    fn wrap_drops_our_breaks_before_squeezing() {
        let m = &mut mono;
        // Three of our lines in a one-line box that holds them all on one line: joined.
        let l = wrap::layout("a\nb\nc", 100 * 16, 1, m).unwrap();
        assert_eq!((l.text.as_str(), l.level), ("a b c", ONE));
    }

    #[test]
    fn wrap_squeezes_only_when_no_wrap_fits() {
        let m = &mut mono;
        // Two lines allowed, line 2 too wide, and wrapping it would make three: squeezed, our
        // lines kept, the widest line drawn at the box.
        let l = wrap::layout("Restores HP.\n200 points. Sora has 107.", 160 * 16, 2, m).unwrap();
        assert_eq!(l.lines, 2);
        assert!(l.level < ONE);
        assert!(scaled(l.widest, l.level) <= 160 * 16);
        assert_eq!(l.text, "Restores HP.\n200 points. Sora has 107.");
        // A single word wider than the box: squeezed to it.
        let l = wrap::layout("Supercalifragilistic", 100 * 16, 2, m).unwrap();
        assert_eq!(l.lines, 1);
        assert!(scaled(l.widest, l.level) <= 100 * 16 && l.level < ONE);
        // Unknown widths: no layout.
        assert_eq!(wrap::layout("x y", 100, 2, &mut |_| None), None);
    }

    #[test]
    fn wrap_squeeze_takes_the_narrowest_wrap() {
        let m = &mut mono;
        // One line allowed: the whole text on it, squeezed to the box.
        let l = wrap::layout("Sell Hi-Potion x107\nfor 642 Moogle Points?", 150 * 16, 1, m).unwrap();
        assert_eq!(l.text, "Sell Hi-Potion x107 for 642 Moogle Points?");
        assert_eq!(l.widest, mono(&l.text).unwrap());
        // Two allowed, both of ours too wide: the narrowest two-line wrap (here ours).
        let l = wrap::layout("Sell Hi-Potion x107\nfor 642 Moogle Points?", 150 * 16, 2, m).unwrap();
        assert_eq!(l.lines, 2);
        assert!(l.widest <= mono("for 642 Moogle Points?").unwrap());
        assert!(scaled(l.widest, l.level) <= 150 * 16);
    }

    #[test]
    fn lines_in_a_box() {
        // n × size + (n − 1) × gap.
        assert_eq!(wrap::lines_in(16 * 16, 16 * 16, 4 * 16), 1);
        assert_eq!(wrap::lines_in(36 * 16, 16 * 16, 4 * 16), 2);
        assert_eq!(wrap::lines_in(55 * 16, 16 * 16, 4 * 16), 2);
        assert_eq!(wrap::lines_in(56 * 16, 16 * 16, 4 * 16), 3);
        assert_eq!(wrap::lines_in(0, 16, 4), 1);
        assert_eq!(wrap::lines_in(100, 0, 4), 1);
        assert_eq!(level16(100, 200), ONE);
        assert_eq!(level16(200, 100), 512);
    }

    #[test]
    fn boxes() {
        assert_eq!(boxes::CAMP_ROW, 172.0);
        assert_eq!(boxes::COMMAND_ROW, 167.0);
        assert_eq!(boxes::status(40.0), 172.0);
        assert_eq!(boxes::status(-1.0), 212.0);
    }
}
