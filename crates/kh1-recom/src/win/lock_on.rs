//! Lock-on on R1 at Combat = KH1 on a controller (`crate::lock_on`, `docs/recom-lock-on.md`).
//!
//! A wrapper on Re:CoM's lock-on `0x1401E8DA0` (`sites::LOCK_ON_HOOK`): before Re:CoM's own runs,
//! its button field (the player's pad object `+0x44`, read only by that function's three readers)
//! gets [`crate::lock_on::button`]: R1 for KH1 Sora with the pad in use, else Re:CoM's R2. Re:CoM
//! then locks, unlocks and switches targets exactly as it does on R2 (press, press and release,
//! held with a right-stick flick), and its pad update writes R2 back the next frame.
//!
//! Per frame: one call a frame of Re:CoM's lock-on: an atomic load and two compares (KH1 Sora),
//! one flag read and one store.

use super::movement::{self as mv, orig, rd, wr};
use super::{base, log};
use crate::lock_on::{self as lo, recom};
use std::sync::atomic::{AtomicUsize, Ordering};

pub static ORIG_LOCK_ON: AtomicUsize = AtomicUsize::new(0);
/// The last device logged (0 none yet, 1 pad, 2 keyboard / mouse).
static LOGGED: AtomicUsize = AtomicUsize::new(0);

type PlFn = unsafe extern "win64" fn(usize) -> u64;

pub fn mark_installed() {
    log("lock-on: at combat = kh1 in Sora's story the controller's R1 is Re:CoM's lock-on (its lock-on 0x1401E8DA0 reads the pad object's +0x44 = 8, R1, instead of Re:CoM's 2, R2); keyboard / mouse keep Re:CoM's binding");
}

/// Re:CoM's lock-on, wrapped.
pub unsafe extern "win64" fn lock_on(pl: usize) -> u64 {
    let o: PlFn = orig(&ORIG_LOCK_ON);
    if pl != 0 && mv::kh1_sora(pl) {
        let kbm = rd::<u32>(base() + recom::KBM_IN_USE as usize) != 0;
        let b = lo::button(true, kbm);
        wr::<u32>(pl + recom::PAD + recom::LOCK_BUTTON, b);
        let dev = if kbm { 2 } else { 1 };
        if LOGGED.swap(dev, Ordering::Relaxed) != dev {
            log(&format!("lock-on: {} in use: the lock-on button is {}", if kbm { "keyboard / mouse" } else { "the controller" }, if b == lo::bits::R1 { "R1" } else { "Re:CoM's (R2 / its key)" }));
        }
    }
    o(pl)
}
