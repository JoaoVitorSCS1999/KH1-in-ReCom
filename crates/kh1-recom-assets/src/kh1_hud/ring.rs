//! KH1's MP ring built the way Re:CoM builds its own HP ring (owner, 2026-11-01: "there also just
//! seems to be a resolution difference between the MP bar and the health bar, if it's possible to
//! have these two bars looking similar quality, maybe built the same way that would be perfect").
//! `docs/recom-kh1-hud.md` § KH1's MP ring built as Re:CoM's HP ring.
//!
//! Re:CoM's HP ring (`gauge:01` groups 7..9, `crate::recom_hp`) draws whole quarter-ring pictures
//! straight from its texture, cut per frame by scissor parts (a sprite whose UVs are all 0, alone
//! in its sprite group: `CScissoring`, `0x14024B910`; the GS scissor stays set for the rest of the
//! layer, `0x140248A90` resets it after the layer's last sprite) and by quarter pictures turned so
//! their straight edge falls where the ring ends. Checked (decomp + files).
//!
//! Here, the same, from KH1's own quarter pictures (`uitex.bin` picture 5's MP and charge rings,
//! picture 7's MP back; KH1's fan `0x14026B790` / `0x14026E450` maps the one quarter picture onto
//! each quadrant, mirrored, the right and bottom halves stretched over one more unit). Each
//! quarter goes into the atlas whole, at the PC texture's own texels, so Re:CoM samples it once at
//! screen resolution as it does its own ring:
//!
//! - **MP back** (a band without cells): full quadrants as plain parts; the quadrant where max MP
//!   ends (30° or 60° into it) as Re:CoM does it: the quadrant's scissor and the quarter picture
//!   turned so its straight edge lies on KH1's end. Then the closing outline under the ring's
//!   start (9 o'clock): KH1 closes it with its "mp" / "HP" label plates, which this layout does not
//!   draw (owner, 2026-11-01: "the MP bar doesnt have the outline at the bottom of the bar").
//! - **MP cells, charge**: the quarter pictures stay where KH1 draws them; the cut is a set of
//!   scissor rects. KH1's cells are separated by clear gaps (Checked (files): about 2° wide at
//!   every 30°), so a cut between cells runs inside a gap and drops no texel of the art.
//! - **What is not KH1's** (scissors are axis-aligned whole 2D units): a charge cut inside a cell
//!   (1° a unit) is a staircase of 1-unit steps across the 4.5-unit orange band, where KH1's fan
//!   ends on a straight radial edge that fades over its last 0.7° (`0x14026B790` mode 4); the
//!   turned MP back quarter is drawn unstretched (KH1 stretches the right and bottom halves over
//!   one more unit: at most 0.9 unit of difference at the end of max MP).

use super::{kh1, obj, obj_piece, Atlas, Group, Pic, Piece, Step, FLAGS_COLOUR, FLAGS_STATIC, GROUP_WORD_FADE, GROUP_WORD_SCRUB, LONG};
use crate::mp_bar as mb;
use crate::uibin::Obj;
use crate::{bad, Result};
use std::collections::{BTreeMap, HashMap};

/// The atlas item a scissor part names (it has no texels: its sprite's UVs are all 0).
pub(crate) const SCISSOR: usize = usize::MAX;

/// Re:CoM's flags for a piece at a constant z rotation: `gauge:01` animations 80 / 85 (the HP
/// ring's cap, −π/2 at both ends) carry 0x77FD1, its static pieces 0x77FF1. Checked (files).
pub(crate) const FLAGS_TURNED: u32 = 0x77FD1;

/// The ring groups, from the first one (the manifest's `mp_ring.first`). Frames as the cluster's
/// own groups (`super::group`): MP back and MP min(max MP / MP, 20), charge min(units, 600); the
/// fades are played as the cluster's.
pub mod group {
    /// MP back (max MP).
    pub const BACK: usize = 0;
    /// MP.
    pub const MP: usize = 1;
    /// Charge.
    pub const CHARGE: usize = 2;
    /// MP gain fade of cell j: 3 + j.
    pub const MP_GAIN: usize = 3;
    /// MP loss fade of cell j: 12 + j.
    pub const MP_LOSS: usize = 12;
    /// Charge gain fade of [30 j, 30 j + t]: 21 + 30 j + t − 1.
    pub const CHARGE_GAIN: usize = 21;
    /// Charge loss fades.
    pub const CHARGE_LOSS: usize = 291;
    /// The dividers at 90° / 180° once max MP passes them, on this ring's seams (0.41.0; drawn
    /// as KH1's own 30° gaps since 0.102.4, [`super::dividers`]).
    pub const TICKS: usize = 561;
    /// Groups in all.
    pub const COUNT: usize = 562;
}

/// Texels whose PS2 alpha (0..128) is below this are ignored by the cuts (under 6 %).
const ALPHA_MIN: u8 = 8;

/// A KH1 quarter-ring picture taken whole: the quadrant square between the ring's texture centre
/// and its radius (`0x1405273B0`: u, v, radius in 1/16 texels), as an atlas item.
#[derive(Clone, Debug)]
pub(crate) struct Quarter {
    /// The atlas item (PS2 crop r × r, PC crops 2r × 2r).
    pub item: usize,
    /// The radius, units (= PS2 texels).
    pub r: i32,
    /// PC texels a side.
    n: usize,
    /// The PC crop's alpha (PS2 scale, 0..128), rows top to bottom; the ring's centre is the
    /// crop's bottom-right corner.
    alpha: Vec<u8>,
    /// The PC crop (first language), for the closing outline's measures.
    pc: Pic,
}

impl Quarter {
    /// Crops ring `ring` of a picture (`ps2`) and its PC copies (`hd`, one per language or one
    /// shared, at `scale`) into the atlas.
    pub fn new(atlas: &mut Atlas, ring: (i32, i32, i32), ps2: &Pic, hd: &[Pic], scale: usize) -> Result<Quarter> {
        let (u, v, r16) = ring;
        let s = scale as i32;
        if (u * s) % 16 != 0 || (v * s) % 16 != 0 || (r16 * s) % 16 != 0 || r16 % 16 != 0 {
            return bad(format!("kh1hud: ring ({u}, {v}, {r16}) is not on whole PC texels"));
        }
        let (uc, vc, n) = ((u * s / 16) as usize, (v * s / 16) as usize, (r16 * s / 16) as usize);
        let r = r16 / 16;
        if uc < n || vc < n || hd.iter().any(|p| p.w < uc || p.h < vc) {
            return bad(format!("kh1hud: ring ({u}, {v}, {r16}) outside its picture"));
        }
        let hdc: Vec<Pic> = hd.iter().map(|p| p.crop(uc - n, vc - n, n, n)).collect();
        // The PS2 copy (fallback, the base `kh1hud.lad`): the same square, on whole PS2 texels.
        let ps2c = ps2.crop(((u - r16) / 16).max(0) as usize, ((v - r16) / 16).max(0) as usize, r as usize, r as usize);
        let alpha = hdc[0].px.iter().map(|t| t[3]).collect();
        let pc = hdc[0].clone();
        let item = atlas.add(ps2c, hdc, scale);
        Ok(Quarter { item, r, n, alpha, pc })
    }

    /// Quadrant `q`'s picture, r × r about the centre (centre-relative; 0 top-left, 1 top-right
    /// mirrored in u, 2 bottom-right mirrored in u and v): a true circle about [`CENTRE`], as
    /// Re:CoM's HP ring is, so both bands keep one width all round (owner, 2026-11-01: "the MP bar
    /// should also be the same thickness as the HP bar"). KH1's fan stretches its right and bottom
    /// halves over r + 1 units about (443, 316) (`0x14026E450`'s `+ 0x10`); about Re:CoM's centre
    /// (444, 317) that is the same right and bottom edge, and the left and top halves one unit
    /// further out than KH1's.
    pub fn image(&self, q: usize) -> Piece {
        let r = self.r;
        let rect = match q {
            0 => [-r, -r, 0, 0],
            1 => [r, -r, 0, 0],
            _ => [r, r, 0, 0],
        };
        Piece { item: self.item, uv: [0, 0, r, r], rect, colour: 0x8080_8080, grad: 0 }
    }

    /// The texture angle (degrees; 0 along the quarter's 9 o'clock edge, its bottom row; 90
    /// along its 12 o'clock edge, its right column) of PC texel (i, j).
    fn texel_tau(&self, i: usize, j: usize) -> f64 {
        let n = self.n as f64;
        (n - j as f64 - 0.5).atan2(n - i as f64 - 0.5).to_degrees()
    }

    /// The centre-relative box of PC texel (i, j) in quadrant `q` (x0, x1, y0, y1).
    fn texel_box(&self, q: usize, i: usize, j: usize) -> (f64, f64, f64, f64) {
        let (r, n) = (self.r as f64, self.n as f64);
        let x = |t: f64| if q == 0 { -r + t * r / n } else { r * (1.0 - t / n) };
        let y = |t: f64| if q <= 1 { -r + t * r / n } else { r * (1.0 - t / n) };
        let (xa, xb) = (x(i as f64), x(i as f64 + 1.0));
        let (ya, yb) = (y(j as f64), y(j as f64 + 1.0));
        (xa.min(xb), xa.max(xb), ya.min(yb), ya.max(yb))
    }

    /// The texture angle at a centre-relative point of quadrant `q`.
    fn point_tau(&self, q: usize, x: f64, y: f64) -> f64 {
        let (dx, dy) = match q {
            0 => (-x, -y),
            1 => (x, -y),
            _ => (x, y),
        };
        dy.max(0.0).atan2(dx.max(0.0)).to_degrees()
    }
}

/// The texture angle span of screen angles [a0, a1] (KH1's: 0 = 9 o'clock, clockwise) in
/// quadrant `q`.
fn tau_span(q: usize, a0: f64, a1: f64) -> (f64, f64) {
    let t = |a: f64| match q {
        0 => a,
        1 => 180.0 - a,
        _ => a - 180.0,
    };
    let (x, y) = (t(a0), t(a1));
    (x.min(y), x.max(y))
}

/// What the cuts of a build drop or keep wrongly (scissor cells that hold texels on both sides of
/// a cut; resolved by the cell centre's side).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CutStats {
    /// Scissor cells with texels on both sides, over cuts between cells (in KH1's gaps).
    pub gap_conflicts: usize,
    /// The largest alpha (0..128) of a texel those resolutions dropped or kept wrongly.
    pub gap_alpha: u8,
    /// The same over cuts inside a cell (the charge): expected (a staircase).
    pub cell_conflicts: usize,
}

/// A scissor cell `X` covers centre-relative x in [X − 0.25, X + 0.75): Re:CoM floors each part
/// corner after its animation's translation + 0.25 (`0x14024A3F0` `>> 4`; `0x14024B4E0` adds the
/// 0.25), and our centre and layer offsets are whole units. The cells a span overlaps.
fn cells(a: f64, b: f64) -> std::ops::RangeInclusive<i32> {
    let lo = (a + 0.25).floor() as i32;
    let hi = ((b + 0.25) - 1e-9).ceil() as i32 - 1;
    lo..=hi.max(lo)
}

/// The scissor rects (centre-relative cells, inclusive: Re:CoM's scissor part corners) that keep
/// the art of quarter `qa` in quadrant `q` whose texture angle is in [t0, t1] and drop the rest.
/// `gap`: the cuts lie between cells (counted in `stats.gap_*`).
fn region(qa: &Quarter, q: usize, t0: f64, t1: f64, gap: bool, stats: &mut CutStats) -> Vec<[i32; 4]> {
    const E: f64 = 1e-6;
    let n = qa.n;
    // Bilinear filtering spreads a texel half a texel (a quarter unit) each way.
    let (px, py) = (0.25, 0.25);
    // (need alpha, wrong-side alpha) per cell.
    let mut grid: HashMap<(i32, i32), (u8, u8)> = HashMap::new();
    let (mut xmin, mut xmax, mut ymin, mut ymax) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for j in 0..n {
        for i in 0..n {
            let a = qa.alpha[j * n + i];
            if a < ALPHA_MIN {
                continue;
            }
            let t = qa.texel_tau(i, j);
            let inside = t >= t0 - E && t <= t1 + E;
            let (x0, x1, y0, y1) = qa.texel_box(q, i, j);
            for cy in cells(y0 - py, y1 + py) {
                for cx in cells(x0 - px, x1 + px) {
                    let e = grid.entry((cx, cy)).or_insert((0, 0));
                    if inside {
                        e.0 = e.0.max(a);
                    } else {
                        e.1 = e.1.max(a);
                    }
                    xmin = xmin.min(cx);
                    xmax = xmax.max(cx);
                    ymin = ymin.min(cy);
                    ymax = ymax.max(cy);
                }
            }
        }
    }
    // Cells with texels on both sides go by their centre's side.
    let mut need: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    let mut forbid: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    for (&(cx, cy), &(a_in, a_out)) in &grid {
        let keep = if a_in > 0 && a_out > 0 {
            let t = qa.point_tau(q, cx as f64 + 0.25, cy as f64 + 0.25);
            let k = t >= t0 - E && t <= t1 + E;
            if gap {
                stats.gap_conflicts += 1;
                stats.gap_alpha = stats.gap_alpha.max(if k { a_out } else { a_in });
            } else {
                stats.cell_conflicts += 1;
            }
            k
        } else {
            a_in > 0
        };
        if keep {
            need.entry(cy).or_default().push(cx);
        } else {
            forbid.entry(cy).or_default().push(cx);
        }
    }
    if need.is_empty() {
        return Vec::new();
    }
    // Per row, the runs of needed cells between forbidden ones: (first, last, lo range, hi range).
    let (lo_bound, hi_bound) = (xmin - 1, xmax + 1);
    let mut rows: BTreeMap<i32, Vec<(i32, i32, i32, i32)>> = BTreeMap::new();
    for (&cy, xs) in need.iter_mut() {
        xs.sort_unstable();
        let mut fs = forbid.get(&cy).cloned().unwrap_or_default();
        fs.sort_unstable();
        let mut runs: Vec<(i32, i32)> = Vec::new();
        for &x in xs.iter() {
            match runs.last_mut() {
                Some(l) if !fs.iter().any(|&f| f > l.1 && f < x) => l.1 = x,
                _ => runs.push((x, x)),
            }
        }
        let v = runs
            .into_iter()
            .map(|(a, b)| {
                let lmin = fs.iter().filter(|&&f| f < a).max().map_or(lo_bound, |f| f + 1);
                let hmax = fs.iter().filter(|&&f| f > b).min().map_or(hi_bound, |f| f - 1);
                (lmin, a, b, hmax)
            })
            .collect();
        rows.insert(cy, v);
    }
    // Rows merged downwards while one run each and their ranges meet.
    let mut out = Vec::new();
    let mut open: Option<(i32, i32, i32, i32, i32, i32)> = None; // y0, y1, lo_lo, lo_hi, hi_lo, hi_hi
    let close = |o: (i32, i32, i32, i32, i32, i32), out: &mut Vec<[i32; 4]>| {
        let (y0, y1, ll, lh, hl, hh) = o;
        let lo = if ll == lo_bound { lo_bound } else { (ll + lh).div_euclid(2) };
        let hi = if hh == hi_bound { hi_bound } else { (hl + hh + 1).div_euclid(2) };
        out.push([lo, y0, hi, y1]);
    };
    for cy in ymin..=ymax {
        let row = rows.get(&cy);
        match row.map(|v| v.as_slice()) {
            Some([(lmin, a, b, hmax)]) => {
                let merged = open.and_then(|o| {
                    let (ll, lh, hl, hh) = (o.2.max(*lmin), o.3.min(*a), o.4.max(*b), o.5.min(*hmax));
                    (o.1 + 1 == cy && ll <= lh && hl <= hh).then_some((o.0, cy, ll, lh, hl, hh))
                });
                match merged {
                    Some(m) => open = Some(m),
                    None => {
                        if let Some(o) = open.take() {
                            close(o, &mut out);
                        }
                        open = Some((cy, cy, *lmin, *a, *b, *hmax));
                    }
                }
            }
            Some(many) => {
                if let Some(o) = open.take() {
                    close(o, &mut out);
                }
                for &(lmin, a, b, hmax) in many {
                    close((cy, cy, lmin, a, b, hmax), &mut out);
                }
            }
            None => {
                if let Some(o) = open.take() {
                    close(o, &mut out);
                }
            }
        }
    }
    if let Some(o) = open.take() {
        close(o, &mut out);
    }
    out
}

/// One drawn thing of a frame: a piece at a translation (KH1 units) and a turn (degrees,
/// counter-clockwise), drawn without a scissor (stage 0), under its own scissor rect (stage 1),
/// or under its own after every stage-1 pair (stage 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Op {
    stage: u8,
    scissor: [i32; 4],
    piece: Piece,
    turn: i32,
    tr: (i32, i32),
}

/// A scissor part as a piece (centre-relative cells).
fn scissor_piece(rect: [i32; 4]) -> Piece {
    Piece { item: SCISSOR, uv: [0; 4], rect, colour: 0x8080_8080, grad: 0 }
}

/// The steps of one op over frames `fr` with colour `colour` (`flags` for the image), the
/// scissor first for stages 1 and 2. `centre`: where scissor rects are relative to.
fn steps_of(op: &Op, pieces: Vec<Piece>, fr: (i32, i32), colour: (u32, u32), flags: u32, centre: (i32, i32), scissor: bool) -> Vec<Step> {
    let one = 0x8080_8080;
    let mut v = Vec::new();
    if scissor && op.stage > 0 {
        v.push(Step { pieces: vec![scissor_piece(op.scissor)], frames: fr, colour: (one, one), flags: FLAGS_STATIC, tr: [centre.0, centre.1, centre.0, centre.1], rot: 0.0 });
    }
    let flags = if op.turn != 0 && flags == FLAGS_STATIC { FLAGS_TURNED } else { flags };
    v.push(Step { pieces, frames: fr, colour, flags, tr: [op.tr.0, op.tr.1, op.tr.0, op.tr.1], rot: (op.turn as f32).to_radians() });
    v
}

/// A held group from each frame's ops: stage-0 ops sharing their frame runs, translation and turn
/// share one animation a run; each scissored op is its scissor's animation then its image's, in
/// stage order (any frame's active animations are then: its stage-0 pieces, then whole
/// scissor / image pairs). The last frame's ops stay for any larger frame.
fn held(frames: &[Vec<Op>], centre: (i32, i32)) -> Group {
    let mut runs: BTreeMap<Op, Vec<(i32, i32)>> = BTreeMap::new();
    for (f, ops) in frames.iter().enumerate() {
        for op in ops {
            let r = runs.entry(*op).or_default();
            match r.last_mut() {
                Some(l) if l.1 + 1 == f as i32 => l.1 = f as i32,
                _ => r.push((f as i32, f as i32)),
            }
        }
    }
    let lastf = frames.len() as i32 - 1;
    for r in runs.values_mut() {
        if let Some(l) = r.last_mut() {
            if l.1 == lastf {
                l.1 = LONG;
            }
        }
    }
    // (stage, scissor, runs, turn, tr) → pieces; stage-0 keys ignore the scissor.
    let mut by: BTreeMap<(u8, [i32; 4], Vec<(i32, i32)>, i32, (i32, i32)), Vec<Piece>> = BTreeMap::new();
    for (op, r) in runs {
        let sc = if op.stage == 0 { [0; 4] } else { op.scissor };
        by.entry((op.stage, sc, r, op.turn, op.tr)).or_default().push(op.piece);
    }
    let one = 0x8080_8080;
    let mut steps = Vec::new();
    for ((stage, sc, r, turn, tr), pieces) in by {
        let op = Op { stage, scissor: sc, piece: pieces[0], turn, tr };
        for fr in r {
            steps.extend(steps_of(&op, pieces.clone(), fr, (one, one), FLAGS_STATIC, centre, true));
        }
    }
    Group { steps, once: true, word: GROUP_WORD_SCRUB }
}

/// A played fade of `ops`: KH1's 10-frame fade (`mb::FADE_*`), `from` → `to` over frames
/// 0 ..= last, then `then` (held to frame 10,000,000 when `hold`, else to the fade's end). A
/// scissor animation spans both phases.
fn fade(ops: &[Op], from: u32, to: u32, then: u32, hold: bool, centre: (i32, i32)) -> Group {
    let last = mb::FADE_FRAMES - 2;
    let end = if hold { LONG } else { mb::FADE_FRAMES };
    let mut sorted = ops.to_vec();
    sorted.sort();
    let mut steps = Vec::new();
    for op in &sorted {
        if op.stage > 0 {
            steps.push(Step { pieces: vec![scissor_piece(op.scissor)], frames: (0, end), colour: (0x8080_8080, 0x8080_8080), flags: FLAGS_STATIC, tr: [centre.0, centre.1, centre.0, centre.1], rot: 0.0 });
        }
        steps.extend(steps_of(op, vec![op.piece], (0, last), (from, to), FLAGS_COLOUR, centre, false));
        steps.extend(steps_of(op, vec![op.piece], (last + 1, end), (then, then), FLAGS_COLOUR, centre, false));
    }
    Group { steps, once: true, word: GROUP_WORD_FADE }
}

/// What the ring groups are made from.
pub(crate) struct Inputs<'a> {
    /// `g_s`'s objects (the straight segments, the back cap).
    pub objs: &'a [Obj],
    /// Pictures 5 and 7 taken whole (the straight segments' items).
    pub it5: usize,
    /// Picture 7.
    pub it7: usize,
    /// The quarter pictures.
    pub back: Quarter,
    /// MP.
    pub mp: Quarter,
    /// Charge.
    pub charge: Quarter,
    /// One texel of the MP back's black outline (PS2 1 × 1, PC 2 × 2), for the closing outline.
    pub black: usize,
    /// The 90° and 180° dividers ([`dividers`]).
    pub div: Dividers,
}

/// The ring's centre (KH1 units): Re:CoM's HP ring's centre at KH1's place (the DLL's
/// `recom_hp::CENTRE`, (444, 317)), so the MP ring is concentric with it and both start on the
/// same line at 9 o'clock (owner, 2026-11-01: "the bars should start from the same place, (the MP
/// bar is currently starting a few pixels above where the HP bar starts)"). 0.40.0 used KH1's
/// gauge centre (443, 316) (`0x1405272B0`), one unit up-left of Re:CoM's ring.
pub const CENTRE: (i32, i32) = (444, 317);

/// How far the ring moved from KH1's centre in x: the straight segments past 270° (KH1's
/// absolute objects, joined at 6 o'clock) and the 12 o'clock tick move with it. In y the bottom
/// half is where KH1 draws it (KH1's stretched lower half ends at 316 + 58 = 317 + 57).
const SHIFT_X: i32 = CENTRE.0 - kh1::CENTRE.0;

/// The closing outline under the ring's start: from the MP back's outer edge to its inner edge
/// along its 9 o'clock row, as thick as the back's outer black outline there (Checked (files):
/// 4 PC texels = 2 units, r 41..57 on the owner's picture 7). Centre-relative rect.
pub fn closing_rect(back: &Quarter) -> Result<[i32; 4]> {
    let (n, pc) = (back.n, &back.pc);
    let row = |i: usize| pc.px[(n - 1) * n + i];
    // From the outer edge (column 0) inwards: the black outline, then the band, then clear.
    let first = (0..n).find(|&i| row(i)[3] >= 64).ok_or_else(|| crate::Error::Format("kh1hud: the MP back's 9 o'clock row is clear".into()))?;
    let black = (first..n).take_while(|&i| row(i)[3] >= 64 && row(i)[..3].iter().all(|&c| c <= 16)).count();
    let last = (first..n).take_while(|&i| row(i)[3] >= 8).last().unwrap_or(first);
    if black == 0 {
        return bad("kh1hud: no black outline at the MP back's 9 o'clock end");
    }
    // PC texels → units (2 a unit), rounded out to whole units.
    let s = (n as i32) / back.r;
    let x0 = -back.r + first as i32 / s;
    let x1 = -back.r + (last as i32 + s) / s;
    let t = (black as i32 + s - 1) / s;
    Ok([x0, 0, x1, t])
}

/// One black texel of the MP back's outer outline near its 9 o'clock end: (PS2 x, y) in picture 7
/// whose PS2 texel and PC texels are all opaque black.
pub fn black_texel(ps2: &Pic, hd: &[Pic], ring: (i32, i32, i32), scale: usize) -> Option<(usize, usize)> {
    let (u, v, r) = ring;
    let (uc, vc, r) = ((u / 16) as usize, (v / 16) as usize, (r / 16) as usize);
    let dark = |t: [u8; 4]| t[3] >= 0x7F && t[..3].iter().all(|&c| c <= 8);
    for y in (vc.saturating_sub(4)..vc).rev() {
        for x in uc.saturating_sub(r)..uc {
            if dark(ps2.px[y * ps2.w + x]) && hd.iter().all(|p| (0..scale).all(|dy| (0..scale).all(|dx| dark(p.px[(y * scale + dy) * p.w + x * scale + dx])))) {
                return Some((x, y));
            }
        }
    }
    None
}

/// Half the dividers' width, units. KH1's 30° gaps fade the MP cells over about ±2.25 units of
/// their centre line (Checked (files): the cells' alpha across the 30° and 60° gaps of the
/// owner's `-a2c0.dds`: clear within ±1, half at ±1.5, whole by ±2.25, the same at every radius of
/// the band), so ±3 holds the whole fade. Even widths only: the seams are on the ring's centre,
/// and parts and translations are whole units.
pub const DIV_HALF: i32 = 3;

/// The 90° / 180° dividers: their atlas items (the vertical strip at 12 o'clock, the horizontal
/// one at 3 o'clock) and where their strips start inside the ring (units from the centre).
#[derive(Clone, Debug)]
pub struct Dividers {
    /// Vertical, horizontal.
    pub items: [usize; 2],
    /// The strips run from here out to the MP back's radius.
    pub r_in: i32,
}

/// The MP cells' coverage (0..1) across KH1's 30° gaps (both gaps of the quarter, both sides,
/// every half unit of the band's radius), averaged over each PC texel column of a divider, from
/// −[`DIV_HALF`] to +[`DIV_HALF`] (the strips' alpha is 1 − this).
fn gap_cover(mp: &Quarter) -> Result<Vec<f64>> {
    let (n, s) = (mp.n, mp.n as f64 / mp.r as f64);
    // Bilinear alpha (0..1) at a point of the quarter (PC texels, from its top-left corner).
    let alpha = |x: f64, y: f64| -> f64 {
        let (x, y) = (x - 0.5, y - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let at = |i: f64, j: f64| -> f64 {
            let (i, j) = (i.clamp(0.0, n as f64 - 1.0) as usize, j.clamp(0.0, n as f64 - 1.0) as usize);
            mp.alpha[j * n + i] as f64 / 128.0
        };
        at(x0, y0) * (1.0 - fx) * (1.0 - fy) + at(x0 + 1.0, y0) * fx * (1.0 - fy) + at(x0, y0 + 1.0) * (1.0 - fx) * fy + at(x0 + 1.0, y0 + 1.0) * fx * fy
    };
    // At (radius, texture angle), units and degrees.
    let polar = |r: f64, t: f64| alpha(n as f64 - r * s * t.to_radians().cos(), n as f64 - r * s * t.to_radians().sin());
    // The cells' band: where the middle of a cell (45°) is mostly covered.
    let band: Vec<f64> = (0..mp.r * 4).map(|k| k as f64 / 4.0).filter(|&r| polar(r, 45.0) > 0.75).collect();
    let (lo, hi) = match (band.first(), band.last()) {
        (Some(&a), Some(&b)) if b - a > 4.0 => (a + 1.0, b - 1.0),
        _ => return bad("kh1hud ring: no MP cells at 45° of picture 5's MP ring"),
    };
    let cols = (2 * DIV_HALF) as usize * s.round() as usize;
    let mut cover = vec![0.0; cols];
    let mut count = 0usize;
    for gap in [30.0, 60.0] {
        let mut r = lo;
        while r <= hi + 1e-9 {
            for (c, v) in cover.iter_mut().enumerate() {
                for k in 0..4 {
                    let d = -DIV_HALF as f64 + (c as f64 + (k as f64 + 0.5) / 4.0) / s;
                    *v += polar(r, gap + (d / r).to_degrees()) + polar(r, gap - (d / r).to_degrees());
                }
            }
            count += 8;
            r += 0.5;
        }
    }
    cover.iter_mut().for_each(|v| *v /= count as f64);
    let (mid, edge) = (cover[cols / 2 - 1].max(cover[cols / 2]), cover[0].min(cover[cols - 1]));
    if mid > 0.1 || edge < 0.9 {
        return bad(format!("kh1hud ring: picture 5's MP cells have no clear 30° / 60° gaps (coverage {mid:.2} at the gap, {edge:.2} at ±{DIV_HALF})"));
    }
    Ok(cover)
}

/// KH1 marks 90° and 180° of max MP with two ticks over the ring (`g_s` objects 37 / 38: 2 × 14
/// units of navy (24, 28, 42) with one black texel at each end, picture 7 (70, 97) / (74, 97);
/// clear on PC, so redrawn from the PS2 texels, [`super::fit_layouts`]); its quarter pictures have
/// gaps at 30° and 60° only. Drawn as they were on this ring (0.41.0–0.102.3) the 12 o'clock tick
/// read thinner than the gaps (2 units against the gaps' 3 at half coverage), stuck half a unit
/// out past the ring's outer edge (its top at r 57, the ring's art ends at r 56.5) and broke the
/// ring's 2-unit outlines with its 1-unit ends (owner, 2026-11-04). Here each divider is a strip
/// of the MP back's own art on the seam (the back's radial profile at that edge: its outlines and
/// navy), its alpha 1 − the cells' coverage across KH1's own 30° gaps, on the back's opaque
/// texels only: over the cells it composes exactly what a gap shows (the back through the cells'
/// fading edges), and it never reaches past the back's art (the back draws its own soft edges). One PC texture for every language (the ring's art is the same in all
/// five; Checked (files)).
pub(crate) fn dividers(atlas: &mut Atlas, back: &Quarter, mp: &Quarter) -> Result<Dividers> {
    let cover = gap_cover(mp)?;
    let (n, s) = (back.n, back.n / back.r as usize);
    if s % 2 != 0 || s == 0 {
        return bad(format!("kh1hud ring: picture 7's PC scale {s} is not even"));
    }
    // The back's radial profile, from its outer edge in: at 12 o'clock (the quarter's right
    // column) and at 3 o'clock (its bottom row, mirrored there).
    let col: Vec<[u8; 4]> = (0..n).map(|j| back.pc.px[j * n + n - 1]).collect();
    let row: Vec<[u8; 4]> = (0..n).map(|i| back.pc.px[(n - 1) * n + i]).collect();
    let last = |v: &[[u8; 4]]| v.iter().rposition(|t| t[3] >= ALPHA_MIN);
    let deepest = match (last(&col), last(&row)) {
        (Some(a), Some(b)) => a.max(b),
        _ => return bad("kh1hud ring: the MP back's 12 / 3 o'clock edge is clear"),
    };
    let depth = (deepest + s) / s; // whole units from the outer edge
    let r_in = back.r - depth as i32;
    let rows = depth * s;
    let cols = cover.len();
    // Only where the back is opaque and its neighbours too: the strip then lies on the back's own
    // opaque art, and its filtered edge falls on the outlines (the back's soft outer and inner
    // edge texels are drawn once, by the back).
    let solid = |p: &[[u8; 4]], k: usize| (k.saturating_sub(1)..=(k + 1).min(n - 1)).all(|j| p[j][3] >= 0x80);
    let fade = |p: &[[u8; 4]], k: usize, c: usize| {
        let t = p[k];
        let a = if solid(p, k) { (t[3] as f64 * (1.0 - cover[c]).clamp(0.0, 1.0)).round() as u8 } else { 0 };
        [t[0], t[1], t[2], a]
    };
    let mut v = Pic::blank(cols, rows);
    let mut h = Pic::blank(rows, cols);
    for c in 0..cols {
        for k in 0..rows {
            v.px[k * cols + c] = fade(&col, k, c);
            h.px[c * rows + k] = fade(&row, rows - 1 - k, c);
        }
    }
    let half = |p: &Pic| (0..s / 2).fold(p.clone(), |p, _| super::menu::half(&p));
    let items = [atlas.add(half(&v), vec![v], s), atlas.add(half(&h), vec![h], s)];
    Ok(Dividers { items, r_in })
}

/// The pieces of `groups` drawn from the whole pictures `items` (the straight segments past 270°
/// and their cap; the ring draws nothing else from them): (group, step, piece). Taken before
/// [`super::split_raw`] gives each its own atlas box, for [`lift_straights`].
pub(crate) fn straight_marks(groups: &[Group], items: &[usize]) -> Vec<(usize, usize, usize)> {
    let mut v = Vec::new();
    for (g, gr) in groups.iter().enumerate() {
        for (s, st) in gr.steps.iter().enumerate() {
            for (p, pc) in st.pieces.iter().enumerate() {
                if items.contains(&pc.item) {
                    v.push((g, s, p));
                }
            }
        }
    }
    v
}

/// KH1's straight MP pieces (back segments `g_s` 17, cap 21, MP 32, charge 33 at KH1's (−1, +1)
/// / (0, +1)) have their art flush with their rects (the back 358..374, MP 360..372, charge
/// 368..372; Checked (files)), while the ring's bottom half ends half a unit inside its quarter
/// (the PC art's outermost texel row is clear: the back's art spans r 40.5..56.5, the MP cells'
/// r 42.5..54.5, about the centre; KH1's fan draws the bottom half unstretched from its apex
/// (443, 317), `0x14026E450`: its arc ends at 373.5 / 371.5 too, Checked (decomp + files)). So
/// the straight part sat half a unit lower than the arc it continues, and under Re:CoM's HP bar
/// (which hides both tops) it read half a unit thicker (owner, 2026-11-04). Parts and
/// translations are whole units, so each marked piece's art moves up half a unit in its texture
/// instead: a copy of its box with half a unit of clear texels above and below (the PC copy; the
/// PS2 one, a whole unit a texel, keeps its place), its rect one unit taller at the top. Returns
/// the copies made.
pub(crate) fn lift_straights(atlas: &mut Atlas, groups: &mut [Group], marks: &[(usize, usize, usize)]) -> Result<usize> {
    let mut made: BTreeMap<usize, usize> = BTreeMap::new();
    for &(g, s, p) in marks {
        let pc = groups[g].steps[s].pieces[p];
        if pc.uv[0] != 0 || pc.uv[1] != 0 || pc.rect[1] >= pc.rect[3] {
            return bad(format!("kh1hud ring: straight piece {pc:?} is not a whole box drawn upright"));
        }
        let item = match made.get(&pc.item) {
            Some(&i) => i,
            None => {
                let (ps2, hds, sc) = &atlas.items[pc.item];
                let sc = *sc;
                if sc % 2 != 0 {
                    return bad(format!("kh1hud ring: PC scale {sc} is not even"));
                }
                let pad = |p: &Pic, top: usize, bottom: usize| {
                    let mut q = Pic::blank(p.w, p.h + top + bottom);
                    q.px[top * p.w..(top + p.h) * p.w].copy_from_slice(&p.px);
                    q
                };
                let ps2 = pad(ps2, 1, 0);
                let hds: Vec<Pic> = hds.iter().map(|h| pad(h, sc / 2, sc / 2)).collect();
                let i = atlas.add(ps2, hds, sc);
                made.insert(pc.item, i);
                i
            }
        };
        let q = &mut groups[g].steps[s].pieces[p];
        q.item = item;
        q.uv[3] += 1;
        q.rect[1] -= 1;
    }
    Ok(made.len())
}

/// Quadrant `q`'s scissor (centre-relative cells; one cell past the centre lines toward the
/// quadrant, so pieces that meet there overlap by a quarter unit instead of leaving a gap).
fn quadrant_scissor(q: usize, r: i32) -> [i32; 4] {
    let m = r + 3;
    match q {
        0 => [-m, -m, 0, 0],
        1 => [0, -m, m, 0],
        _ => [0, 0, m, m],
    }
}

/// A scissor that covers the whole screen (Re:CoM clamps it, `0x14024A3F0`): the reset.
const FULL: [i32; 4] = [-2000, -2000, 2000, 2000];

/// The scissored ops that show quarter `qa`'s art in screen angles [a0, a1] of quadrant `q`
/// (whole when the span is the quadrant).
fn cut_ops(qa: &Quarter, q: usize, a0: f64, a1: f64, gap: bool, stats: &mut CutStats, colour: u32) -> Vec<Op> {
    let piece = Piece { colour, ..qa.image(q) };
    if a0 <= 90.0 * q as f64 && a1 >= 90.0 * q as f64 + 90.0 {
        return vec![Op { stage: 0, scissor: [0; 4], piece, turn: 0, tr: CENTRE }];
    }
    let (t0, t1) = tau_span(q, a0, a1);
    region(qa, q, t0, t1, gap, stats).into_iter().map(|sc| Op { stage: 1, scissor: sc, piece, turn: 0, tr: CENTRE }).collect()
}

/// The ops showing quarter `qa` over screen angles [0, a] (cut between cells when `gap`).
fn arc_ops(qa: &Quarter, a: f64, gap: bool, stats: &mut CutStats) -> Vec<Op> {
    let mut v = Vec::new();
    for q in 0..3 {
        let q0 = 90.0 * q as f64;
        if a > q0 {
            v.extend(cut_ops(qa, q, q0, a.min(q0 + 90.0), gap, stats, 0x8080_8080));
        }
    }
    v
}

/// The ring groups ([`group`]'s order). `stats` gathers the cuts' resolutions.
pub(crate) fn groups(i: &Inputs, stats: &mut CutStats) -> Result<Vec<Group>> {
    let objs = i.objs;
    let plain = |p: Piece| Op { stage: 0, scissor: [0; 4], piece: p, turn: 0, tr: (0, 0) };
    let close = closing_rect(&i.back)?;
    let cap = Op { stage: 2, scissor: FULL, piece: Piece { item: i.black, uv: [0, 0, 1, 1], rect: close, colour: 0x8080_8080, grad: 0 }, turn: 0, tr: CENTRE };
    let mut out = Vec::with_capacity(group::COUNT);
    // MP back: 30° a max MP point to 270°; the quadrant it ends in turned as Re:CoM's ring; past
    // 9 KH1's straight back segments and their cap (`0x140269F50`, as the cluster's).
    let mut f = Vec::new();
    for m in 0..=kh1::MP_CAP {
        let a = (m * kh1::MP_DEG).min(270);
        let mut v = Vec::new();
        for q in 0..3usize {
            let q0 = 90 * q as i32;
            if a >= q0 + 90 {
                v.push(Op { stage: 0, scissor: [0; 4], piece: i.back.image(q), turn: 0, tr: CENTRE });
            } else if a > q0 {
                let b = a - q0;
                v.push(Op { stage: 1, scissor: quadrant_scissor(q, i.back.r), piece: i.back.image(q), turn: 90 - b, tr: CENTRE });
            }
        }
        if m > 0 {
            v.push(cap);
        }
        if m >= kh1::MP_RING {
            let mut dx = -1 + SHIFT_X;
            for _ in 0..m - kh1::MP_RING {
                v.push(plain(obj_piece(&objs[obj::BACK_MP], i.it7, dx, 1, 0, None)));
                dx -= kh1::PITCH;
            }
            v.push(plain(obj_piece(&objs[obj::CAP_MP], i.it7, dx, 1, 0, Some(kh1::MP_CAP_X))));
        }
        f.push(v);
    }
    out.push(held(&f, CENTRE));
    // MP: whole cells to min(MP, 9), cut in KH1's gaps; past 9 a straight segment a point.
    let mut f = Vec::new();
    for m in 0..=kh1::MP_CAP {
        let mut v = arc_ops(&i.mp, (m.min(kh1::MP_RING) * kh1::MP_DEG) as f64, true, stats);
        let mut dx = -1 + SHIFT_X;
        for _ in kh1::MP_RING..m {
            v.push(plain(obj_piece(&objs[obj::SEG_MP], i.it5, dx, 1, 0, None)));
            dx -= kh1::PITCH;
        }
        f.push(v);
    }
    out.push(held(&f, CENTRE));
    // Charge: 1° a unit to 270, then 30 units a straight segment (partial cut), as the cluster's.
    let seg_w = objs[obj::SEG_CHARGE].rect[2];
    let mut f = Vec::new();
    for c in 0..=kh1::CHARGE_CAP {
        let a = c.min(kh1::CHARGE_RING);
        let mut v = arc_ops(&i.charge, a as f64, a % 30 == 0, stats);
        if c > kh1::CHARGE_RING {
            let (n, rem) = ((c - kh1::CHARGE_RING) / kh1::CHARGE_PER_SEG, (c - kh1::CHARGE_RING) % kh1::CHARGE_PER_SEG);
            let mut dx = SHIFT_X;
            for _ in 0..n {
                v.push(plain(obj_piece(&objs[obj::SEG_CHARGE], i.it5, dx, 1, 0, None)));
                dx -= kh1::PITCH;
            }
            if rem != 0 {
                v.push(plain(obj_piece(&objs[obj::SEG_CHARGE], i.it5, dx, 1, super::cut(rem, kh1::CHARGE_PER_SEG, seg_w), None)));
            }
        }
        f.push(v);
    }
    out.push(held(&f, CENTRE));
    // Fades, as the cluster's (`super::build`): MP gain black over the cell, alpha 115 → 13; MP
    // loss the cell's RGB 115 → 13; charge [30 j, 30 j + t] fading in (held) or out.
    let alpha = |a: u8| (a as u32) << 24 | 0x80_8080;
    let grey = |c: u8| 0x8000_0000 | (c as u32) * 0x01_0101;
    let cell = |qa: &Quarter, j: i32, t: i32, colour: u32, stats: &mut CutStats| -> Vec<Op> {
        let q = (j / 3) as usize;
        cut_ops(qa, q, (30 * j) as f64, (30 * j + t) as f64, t == 30, stats, colour)
    };
    for j in 0..kh1::MP_RING {
        let ops = cell(&i.mp, j, 30, 0x8000_0000, stats);
        out.push(fade(&ops, alpha(mb::FADE_FIRST), alpha(mb::FADE_LAST), 0x0080_8080, false, CENTRE));
    }
    for j in 0..kh1::MP_RING {
        let ops = cell(&i.mp, j, 30, 0x8080_8080, stats);
        out.push(fade(&ops, grey(mb::FADE_FIRST), grey(mb::FADE_LAST), 0, false, CENTRE));
    }
    let mut charge_cells = Vec::new();
    for j in 0..9 {
        for t in 1..=30 {
            charge_cells.push(cell(&i.charge, j, t, 0x8080_8080, stats));
        }
    }
    for ops in &charge_cells {
        out.push(fade(ops, alpha(mb::FADE_LAST), alpha(mb::FADE_FIRST), 0x8080_8080, true, CENTRE));
    }
    for ops in &charge_cells {
        out.push(fade(ops, alpha(mb::FADE_FIRST), alpha(mb::FADE_LAST), 0x0080_8080, false, CENTRE));
    }
    // Dividers: once max MP passes 90° / 180° (KH1's rule for its ticks, `g_s` objects 37 / 38,
    // `0x14026E630` `+0x78` / `+0x80`), a strip of the MP back's own art on the seam, faded at
    // its sides as KH1's 30° gaps fade the cells (owner, 2026-11-04: the 12 o'clock one was "too
    // narrow, differently drawn, and extends past the bar"; [`dividers`]).
    let [dv, dh] = i.div.items;
    let (w, rin, rout) = (DIV_HALF, i.div.r_in, i.back.r);
    let strip = |item: usize, rect: [i32; 4], uv: [i32; 4]| Op { stage: 0, scissor: [0; 4], piece: Piece { item, uv, rect, colour: 0x8080_8080, grad: 0 }, turn: 0, tr: CENTRE };
    let s90 = strip(dv, [-w, -rout, w, -rin], [0, 0, 2 * w, rout - rin]);
    let s180 = strip(dh, [rin, -w, rout, w], [0, 0, rout - rin, 2 * w]);
    let mut f = Vec::new();
    for m in 0..=kh1::MP_CAP {
        let mut v = Vec::new();
        if m * kh1::MP_DEG > 90 {
            v.push(s90);
        }
        if m * kh1::MP_DEG > 180 {
            v.push(s180);
        }
        f.push(v);
    }
    out.push(held(&f, CENTRE));
    if out.len() != group::COUNT {
        return bad(format!("kh1hud ring: {} groups, expected {}", out.len(), group::COUNT));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic quarter: a band r 42..54 with clear 2° gaps at 30° and 60° (KH1's MP ring's
    /// shape), PC 2 texels a unit.
    fn quarter() -> (Atlas, Quarter) {
        let (r, n) = (55usize, 110usize);
        let mut pc = Pic { w: 128 * 2, h: 128 * 2, px: vec![[0; 4]; 256 * 256] };
        let (uc, vc) = (113usize, 113usize);
        for j in 0..n {
            for i in 0..n {
                let (dx, dy) = ((n - i) as f64 - 0.5, (n - j) as f64 - 0.5);
                let rr = (dx * dx + dy * dy).sqrt() / 2.0;
                let t = dy.atan2(dx).to_degrees();
                let gap = (t - 30.0).abs() < 1.0 || (t - 60.0).abs() < 1.0;
                if (42.0..=54.0).contains(&rr) && !gap {
                    pc.px[(vc - n + j) * pc.w + uc - n + i] = [40, 60, 200, 0x80];
                }
            }
        }
        let ps2 = Pic { w: 128, h: 128, px: vec![[0; 4]; 128 * 128] };
        let mut atlas = Atlas { items: Vec::new() };
        let q = Quarter::new(&mut atlas, (904, 904, 880), &ps2, &[pc], 2).unwrap();
        assert_eq!((q.r, q.n), (r as i32, n));
        (atlas, q)
    }

    /// Every texel the rects keep is inside, every one they drop is outside (cells as Re:CoM's).
    fn check(q: &Quarter, quad: usize, t0: f64, t1: f64, rects: &[[i32; 4]]) -> (usize, usize) {
        let mut bad = (0, 0);
        let covered = |x: f64, y: f64| rects.iter().any(|r| x >= r[0] as f64 - 0.25 && x < r[2] as f64 + 0.75 && y >= r[1] as f64 - 0.25 && y < r[3] as f64 + 0.75);
        for j in 0..q.n {
            for i in 0..q.n {
                if q.alpha[j * q.n + i] < ALPHA_MIN {
                    continue;
                }
                let (x0, x1, y0, y1) = q.texel_box(quad, i, j);
                let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                let t = q.texel_tau(i, j);
                let inside = t >= t0 && t <= t1;
                if inside != covered(cx, cy) {
                    if inside {
                        bad.0 += 1;
                    } else {
                        bad.1 += 1;
                    }
                }
            }
        }
        // No two rects overlap (an image drawn twice would double its edges).
        for (a, ra) in rects.iter().enumerate() {
            for rb in &rects[a + 1..] {
                assert!(ra[2] < rb[0] || rb[2] < ra[0] || ra[3] < rb[1] || rb[3] < ra[1], "{ra:?} {rb:?}");
            }
        }
        bad
    }

    #[test]
    fn gap_cuts_keep_whole_cells() {
        let (_a, q) = quarter();
        for quad in 0..3 {
            for (t0, t1) in [(0.0, 30.0), (0.0, 60.0), (30.0, 60.0), (60.0, 90.0), (30.0, 90.0)] {
                let mut st = CutStats::default();
                let rects = region(&q, quad, t0, t1, true, &mut st);
                assert!(!rects.is_empty());
                assert_eq!(check(&q, quad, t0, t1, &rects), (0, 0), "quadrant {quad} [{t0}, {t1}] {rects:?} {st:?}");
                assert!(rects.len() <= 16, "{} rects", rects.len());
            }
        }
    }

    #[test]
    fn cuts_in_a_cell_are_staircases() {
        let (_a, q) = quarter();
        let mut st = CutStats::default();
        let rects = region(&q, 1, 0.0, 45.0, false, &mut st);
        let (lost, kept) = check(&q, 1, 0.0, 45.0, &rects);
        // Only texels next to the cut go the wrong way.
        assert!(lost + kept < 60, "{lost} {kept}");
        assert!(st.cell_conflicts > 0 && st.gap_conflicts == 0);
    }

    #[test]
    fn angles_per_quadrant() {
        assert_eq!(tau_span(0, 0.0, 30.0), (0.0, 30.0));
        assert_eq!(tau_span(1, 90.0, 120.0), (60.0, 90.0));
        assert_eq!(tau_span(2, 210.0, 240.0), (30.0, 60.0));
        let (_a, q) = quarter();
        // The far corner of each quadrant's picture is texel (0, 0).
        assert_eq!(q.texel_box(0, 0, 0), (-55.0, -54.5, -55.0, -54.5));
        let (x0, x1, ..) = q.texel_box(1, 0, 0);
        // Unstretched: the top-right quadrant mirrors the top-left one about the centre.
        assert!((x1 - 55.0).abs() < 1e-9 && (x0 - 55.0 * (1.0 - 1.0 / 110.0)).abs() < 1e-9);
        assert!((q.point_tau(0, -10.0, 0.0) - 0.0).abs() < 1e-9 && (q.point_tau(1, 0.0, -10.0) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn held_groups_put_scissor_pairs_after_plain_pieces() {
        let (_a, q) = quarter();
        let mut st = CutStats::default();
        let frames: Vec<Vec<Op>> = (0..=9).map(|m| arc_ops(&q, (30 * m) as f64, true, &mut st)).collect();
        let g = held(&frames, CENTRE);
        // At every frame: the active steps are plain pieces, then (scissor, image) pairs.
        for f in 0..=9 {
            let act: Vec<&Step> = g.steps.iter().filter(|s| s.frames.0 <= f && f <= s.frames.1).collect();
            let first_sc = act.iter().position(|s| s.pieces[0].item == SCISSOR).unwrap_or(act.len());
            assert!(act[..first_sc].iter().all(|s| s.pieces.iter().all(|p| p.item != SCISSOR)));
            for pair in act[first_sc..].chunks(2) {
                assert_eq!(pair.len(), 2);
                assert!(pair[0].pieces.len() == 1 && pair[0].pieces[0].item == SCISSOR);
                assert!(pair[1].pieces.iter().all(|p| p.item != SCISSOR));
            }
            // Whole quadrants are plain: m = 3 shows TL whole, no pair.
            if f == 3 || f == 6 || f == 9 {
                assert_eq!(first_sc, act.len(), "frame {f}");
                assert_eq!(act.iter().map(|s| s.pieces.len()).sum::<usize>(), f as usize / 3);
            }
        }
    }

    /// A synthetic MP back quarter (picture 7's shape): black r 40.5..42.5 and 54.5..56.5, navy
    /// between, clear elsewhere, PC 2 texels a unit.
    fn back_quarter(atlas: &mut Atlas) -> Quarter {
        let n = 114usize;
        let mut pc = Pic { w: 256, h: 256, px: vec![[0; 4]; 256 * 256] };
        let (uc, vc) = (215usize, 189usize);
        for j in 0..n {
            for i in 0..n {
                let (dx, dy) = ((n - i) as f64 - 0.5, (n - j) as f64 - 0.5);
                let rr = (dx * dx + dy * dy).sqrt() / 2.0;
                let t = if (40.5..42.5).contains(&rr) || (54.5..56.5).contains(&rr) {
                    [0, 0, 0, 0x80]
                } else if (42.5..54.5).contains(&rr) {
                    [24, 28, 42, 0x80]
                } else {
                    [0; 4]
                };
                pc.px[(vc - n + j) * pc.w + uc - n + i] = t;
            }
        }
        let ps2 = Pic { w: 128, h: 128, px: vec![[0; 4]; 128 * 128] };
        Quarter::new(atlas, (1720, 1512, 912), &ps2, &[pc], 2).unwrap()
    }

    #[test]
    fn dividers_are_the_back_faded_as_kh1s_gaps() {
        let (mut atlas, mp) = quarter();
        let back = back_quarter(&mut atlas);
        let d = dividers(&mut atlas, &back, &mp).unwrap();
        // The cells' coverage across a gap: symmetric, clear in the middle, whole at the sides.
        let c = &gap_cover(&mp).unwrap();
        assert_eq!(c.len(), 12);
        for k in 0..6 {
            assert!((c[k] - c[11 - k]).abs() < 1e-9);
        }
        assert!(c[5] < 0.05 && c[0] > 0.95, "{c:?}");
        // The strips start where the back's art ends inside (r 40) and reach its radius.
        assert_eq!(d.r_in, 40);
        let (_, hv, _) = &atlas.items[d.items[0]];
        let (_, hh, _) = &atlas.items[d.items[1]];
        let (v, h) = (&hv[0], &hh[0]);
        assert_eq!((v.w, v.h, h.w, h.h), (12, 34, 34, 12));
        // Never past the back's opaque art (no texel the back draws soft or not at all), and the
        // middle columns are the back itself.
        let n = back.n;
        for k in 0..34 {
            let b = back.pc.px[k * n + n - 1];
            for col in 0..12 {
                let t = v.px[k * 12 + col];
                assert!(t[3] <= b[3]);
                if b[3] < 0x80 {
                    assert_eq!(t[3], 0);
                }
            }
            let mid = v.px[k * 12 + 5];
            if k >= 2 && k <= 31 {
                assert_eq!(mid[..3], b[..3]);
                assert!(mid[3] >= 0x7A, "row {k}: {mid:?}");
            }
            // The horizontal strip is the same profile, the ring's outside on the right.
            assert_eq!(h.px[5 * 34 + 33 - k], mid);
        }
    }

    #[test]
    fn straights_lift_half_a_unit() {
        let mut atlas = Atlas { items: Vec::new() };
        let mut hd = Pic::blank(4, 4);
        hd.px.iter_mut().for_each(|t| *t = [1, 2, 3, 0x80]);
        let item = atlas.add(Pic::blank(2, 2), vec![hd], 2);
        let p = Piece { item, uv: [0, 0, 2, 2], rect: [10, 20, 12, 22], colour: 0x8080_8080, grad: 0 };
        let one = 0x8080_8080;
        let mut gs = vec![Group { steps: vec![Step { pieces: vec![p, p], frames: (0, 1), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 }], once: true, word: GROUP_WORD_SCRUB }];
        let marks = straight_marks(&gs, &[item]);
        assert_eq!(marks.len(), 2);
        assert_eq!(lift_straights(&mut atlas, &mut gs, &marks).unwrap(), 1);
        let q = gs[0].steps[0].pieces[0];
        assert_eq!((q.uv, q.rect), ([0, 0, 2, 3], [10, 19, 12, 22]));
        let (ps2, hds, _) = &atlas.items[q.item];
        assert_eq!((ps2.w, ps2.h, hds[0].w, hds[0].h), (2, 3, 4, 6));
        // Half a unit (one PC texel) clear above and below: the art 19.5..21.5 in the rect 19..22.
        let a: Vec<u8> = (0..6).map(|j| hds[0].px[j * 4][3]).collect();
        assert_eq!(a, vec![0, 0x80, 0x80, 0x80, 0x80, 0]);
    }

    #[test]
    fn turned_quadrant_ends_where_max_mp_ends() {
        // The turned top-left picture's 12 o'clock edge (its right edge, pointing up) at 30° CCW
        // of 60°: 30° above 9 o'clock... checked as Re:CoM turns: (x, y) → (x c − y s, x s + y c)
        // with the in-memory angle −rot (`0x14024B4E0` negates, `0x1402EDB10`).
        for b in [30i32, 60] {
            let rot = ((90 - b) as f32).to_radians();
            let th = -rot as f64;
            let (c, s) = (th.cos(), th.sin());
            // The edge direction (0, −1) (12 o'clock).
            let (x, y) = (0.0 * c - (-1.0) * s, 0.0 * s + (-1.0) * c);
            let screen = (-y).atan2(-x).to_degrees(); // KH1's angle from 9 o'clock, clockwise
            assert!((screen - b as f64).abs() < 1e-4, "{b}: {screen}");
        }
    }
}
