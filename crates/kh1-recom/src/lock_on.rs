//! Lock-on on R1 at Combat = KH1 on a controller (owner, 2026-11-04, `design-philosophy-instructions.md`
//! § Shop text, lock-on and camera: "Let's have the lock on button be mapped to R1 if combat is set
//! to KH1. If on mouse and keyboard it stays the same."). Notes: `docs/recom-lock-on.md`.
//!
//! **Re:CoM** (Checked (decomp + disasm + exe), `sites::LOCK_ON_HOOK` / `LOCK_ON_CHECKED`):
//!
//! - Re:CoM's lock-on is `0x1401E8DA0`, called once a frame from the player's Field and Battle exes
//!   (`0x1401F994B`, `0x1401FA1A2`). It reads its button from the player's pad object (`player +
//!   0x230`) field `+0x44` through its three readers only (`0x1401FF740` pressed, `0x1401FF600`
//!   released, `0x1401FF4F0` held): press locks, a second press then its release unlocks, held with
//!   a right-stick flick switches the target (`0x1401FF4A0`).
//! - The pad object's update `0x1401FF150` writes its action buttons every frame (`0x1401FF19D`):
//!   `+0x38` = 4 (L1: card scroll left, sleight with R1), `+0x3C` = 8 (R1: card scroll right),
//!   `+0x40` = 1 (L2), **`+0x44` = 2 (R2: lock-on)**, in Re:CoM's PS2 button order (L2 1, R2 2,
//!   L1 4, R1 8, Triangle 0x10, Circle 0x20, Cross 0x40, Square 0x80, …; the pad read
//!   `0x14029C690` swaps the PS2 pad bytes into it). So Re:CoM's lock-on is **R2** on a pad.
//! - The keyboard feeds the same PS2 pad word through Re:CoM's key bindings; which device is in
//!   use is `0x140C2FF64` (1 keyboard / mouse, 0 pad), set each frame by `0x140347EF0` (any key or
//!   mouse input → 1; any pad button or stick → 0) and read by Re:CoM's own glyph picker
//!   `0x140345B40` (getter `0x1403471C0`).
//!
//! **KH1** (Checked (KH1 Steam decomp + exe)): its pad step `0x14028B0B0` remaps the PS2 pad word
//! through the table `0x140528290` (19 pairs): R1 `0x800` → logical `0x400`, the lock-on button of
//! its handler `0x1402B0610` (press locks; pressed again unlocks; logical `0x800`, from L2, steps
//! through its lock list). L2 / R2 also give `0x400000` (a special camera mode's input and the end
//! of a held soft target), no lock-on.
//!
//! **Ours**: at Combat = KH1 (Sora's story, the main `CPlayerSora`), with the pad in use, Re:CoM's
//! lock-on reads **R1** (8) instead of R2: [`button`], written into `+0x44` right before Re:CoM's
//! lock-on runs (a wrapper on `0x1401E8DA0`). Its readers are only that function's; Re:CoM's update
//! writes 2 again the next frame. With the keyboard / mouse, Re:CoM's binding (2). R2 then does
//! nothing in a KH1 battle (Re:CoM's card input is off there: `win::attack::request_bits`).

/// Re:CoM's side (Steam RVAs and offsets). Checked (decomp + disasm + exe).
pub mod recom {
    /// The lock-on `0x1401E8DA0` (the hook).
    pub const LOCK_ON: u32 = 0x001E_8DA0;
    /// The player's pad object, and its lock-on button field.
    pub const PAD: usize = 0x230;
    pub const LOCK_BUTTON: usize = 0x44;
    /// "Keyboard / mouse in use" (1), else the pad (0).
    pub const KBM_IN_USE: u32 = 0x00C2_FF64;
}

/// Re:CoM's PS2-order button bits (the pad object's update `0x1401FF19D`).
pub mod bits {
    pub const L2: u32 = 1;
    pub const R2: u32 = 2;
    pub const L1: u32 = 4;
    pub const R1: u32 = 8;
}

/// The lock-on button Re:CoM's lock-on reads this frame: R1 at Combat = KH1 with the pad in use,
/// else Re:CoM's R2 (keyboard and mouse keep Re:CoM's binding; Combat = Re:CoM is Re:CoM's).
pub fn button(kh1_sora: bool, keyboard_mouse: bool) -> u32 {
    if kh1_sora && !keyboard_mouse {
        bits::R1
    } else {
        bits::R2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r1_only_on_the_pad_at_combat_kh1() {
        assert_eq!(button(true, false), bits::R1);
        assert_eq!(button(true, true), bits::R2, "keyboard and mouse: Re:CoM's binding");
        assert_eq!(button(false, false), bits::R2, "Combat = Re:CoM / Riku: Re:CoM's");
        assert_eq!(button(false, true), bits::R2);
        // Re:CoM's own lock-on button is the one its pad update writes.
        assert_eq!(bits::R2, 2);
        assert_eq!(bits::R1 & (bits::L1 | bits::L2 | bits::R2), 0, "R1 is its own bit");
    }
}
