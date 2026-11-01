//! Re:CoM's 2D instances, as our HUD controls (the MP bar, the command menu) use them: the
//! testable part (`win/d2.rs`; `docs/recom-m7-mp-bar.md` § 2D instances and draw lists).
//!
//! Re:CoM's 2D manager (`D2::C2DMan`, ctor `0x1402451E0`), Checked (decomp + exe):
//! - **One pool of 2048 instance entries** (`D2::C2DOSLCManager` at `[0x14087CE58]`, made with
//!   `0x1402425E0(+8, 0x20, 0x800)`; entry 0 is taken at once by the ctor's first allocation, so
//!   2047 are handed out). Its free count is `+8 +0x24` (`0x140242560`: `cmp [rcx+0x24],0`, then
//!   `dec`), its size `+8 +0x14`. A handle is the entry index plus a generation (`0x1402426B0`
//!   adds the pool size on release), so a stale handle finds nothing (`0x1402425B0`).
//! - **Each layer of a `C2DSeqCtrl` takes one entry**: a sprite group added with `+8`
//!   (`0x140247850` → `0x140246A40` → `0x140248F40` → `0x140249770`) and a text item added with
//!   `+0x60` (`0x1402476F0` → `0x1402475D0` → `0x140246B60` → `0x140249090` → `0x140249770`).
//!   The ctrl keeps the handles at `+0x144` (10 at most), the count at `+0x16D`.
//! - **Released only by the ctrl**: its clear (`0x140247C90`), its layer removals
//!   (`0x140247C40`, `0x140247CF0`, `0x140247D70`) and its dtor (`0x1402473D0`) call
//!   `0x140245D90(handle)`, which releases the entry and deletes the `C2DSeqObj`. Nothing frees a
//!   finished time-played layer by itself.
//! - **A text item's `C2DFnt` is owned by its layer**: the `C2DSeqObj`'s dtor (`0x1402489E0`)
//!   deletes its object list (`0x140248940`), which deletes the `C2DFntObj` (`0x140244170`),
//!   which deletes the font at its `+0x58` through the font's own deleting dtor (`0x140243FB0`).
//!   `CStockFolder`'s dtor (`0x140165490`) deletes its three ctrls and never its font
//!   (`+0x108`). So deleting the ctrl frees the font; deleting the font too would free it twice.
//! - **Three draw lists, one current** (`[0x14087CE80 + 8 i]`, the current one `[0x14087CE70]`,
//!   its index `[0x140820F94]`, set by `0x140249800`; `0x140245710` also sets the drawn index
//!   `[0x14087CE50]`). The manager steps and draws only the current list (`0x140246500`,
//!   `0x140245EB0`). A new instance goes into the current list (`0x140249770`), and a release
//!   looks for the handle **only in the current list** (`0x1402499C0` → `0x140249E40`): from
//!   another list it finds nothing, so the entry stays taken and the `C2DSeqObj` is not deleted,
//!   while the ctrl forgets the handle all the same. List 1 is the overlay screens' (the camp
//!   `0x1400D9C80` state 0, the pause `0x1401145F0`, the roulette, the save screen, the world
//!   warp, the Moogle shop; their dtors set 0 back); the HUD is on list 0.

/// Re:CoM's draw lists (`0x140249800`: `cmp ecx,3; jae`).
pub const LISTS: i32 = 3;

/// The pool's entries (`0x1402451E0`: `mov r8d,0x800`), and how many are handed out (entry 0 is
/// the ctor's).
pub const POOL_ENTRIES: i32 = 0x800;
pub const POOL_USABLE: i32 = POOL_ENTRIES - 1;

/// The switch our create or release needs: `Some((wanted, back))` when the current list is not
/// the one our object is (or goes) on, None when nothing needs switching (the same list, or either
/// index unknown).
pub fn switch(current: i32, wanted: i32) -> Option<(u32, u32)> {
    let ok = |i: i32| (0..LISTS).contains(&i);
    (ok(current) && ok(wanted) && current != wanted).then_some((wanted as u32, current as u32))
}

/// Our 2D objects as a log line: layers (= pool entries) and controls we hold now, and Re:CoM's
/// free pool entries.
pub fn census_line(layers: i32, ctrls: i32, pool: Option<(i32, i32)>, list: i32) -> String {
    let pool = match pool {
        Some((free, size)) => format!("Re:CoM's 2D pool {free} of {} free", size - 1),
        None => "Re:CoM's 2D pool not there".to_string(),
    };
    format!("ours {layers} layer(s) on {ctrls} control(s); {pool}; current list {list}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_only_between_known_different_lists() {
        assert_eq!(switch(0, 0), None);
        assert_eq!(switch(1, 1), None);
        // The camp (list 1) is up and our HUD objects are on list 0: switch there and back.
        assert_eq!(switch(1, 0), Some((0, 1)));
        assert_eq!(switch(0, 2), Some((2, 0)));
        // Before the manager made its lists (index -1) or an index out of range: no switch.
        assert_eq!(switch(-1, 0), None);
        assert_eq!(switch(0, -1), None);
        assert_eq!(switch(3, 0), None);
        assert_eq!(switch(0, 3), None);
    }

    #[test]
    fn census_reads_plainly() {
        assert_eq!(
            census_line(27, 9, Some((1990, POOL_ENTRIES)), 0),
            "ours 27 layer(s) on 9 control(s); Re:CoM's 2D pool 1990 of 2047 free; current list 0"
        );
        assert_eq!(census_line(0, 0, None, -1), "ours 0 layer(s) on 0 control(s); Re:CoM's 2D pool not there; current list -1");
        assert_eq!(POOL_USABLE, 2047);
    }
}
