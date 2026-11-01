//! Re:CoM's battle end, read only (`docs/recom-m6-battles.md` § Battle end): what the battle
//! enemy manager (`CBattleEnemyManager`, task 0x19) waits for after the last enemy, for the log
//! and the F9 trace. Nothing here changes Re:CoM. The runtime is `win/battle.rs` (`end_watch`).
//!
//! The manager's step `0x1401871D0` (Checked (decomp + disasm `0x1401871D0..0x1401873E6`)):
//!
//! | state `+0x58` | waits for | then |
//! |---|---|---|
//! | 1 | its child tasks `+0x50` ≤ 0 and its enemy count `+0x98` ≤ 0, and the room (`0x1402339B0`) not in `0x41A..=0x4AE` | 2 with the timer `+0x5C` = 6000 ticks (15000 with `+0x74`), or 4 at once with `+0x70` |
//! | 2 | Re:CoM's prize display busy (`0x140209830`: `CPrizeGauge` `+0x94` ≠ 0) | 3 |
//! |   | else the timer runs down by the task's step `+0x14` | 4 |
//! | 3 | the prize display closed (`+0x94` = 0) | 4 |
//! | 4 | Sora's win pose done (`0x1401F3C90`: alive `+0x7B0` = 0, action `0x1C`, `+0x414` > 2) | 5 / 6 (the roulette, `+0x6C` / `+0x70`), else 7 |
//! | 5, 6 | the roulette task 0x22 (`CRoulette`, `0x14011A950`) gone | 6 / 7 |
//! | 7 | — (sets event flag (1, 2) each step, `0x140182240`): the room script takes Sora back | — |
//!
//! Sora's win pose: the enemy count reaching 0 calls `0x1401F4A60`, which sets `player+0x428 |= 4`
//! (the win is pending; refused while Sora is dead or in action 0x17); Re:CoM enters its win
//! action `0x1C` (`0x1401F1B20`) from its move exe (`0x1401EEDA0`, actions 2–4 in battle), from
//! its end action (`0x1401F6660`, which every attack / magic exe calls at its end) or from a hit
//! (`0x1401E6DE0`). The Battle exe's case 0x1C plays the pose and moves `+0x414` 2 → 4.
//! Checked (decomp).

/// Re:CoM's win action and Sora's "win pending" bit (`player+0x428`).
pub const WIN_ACTION: u32 = 0x1C;
pub const WIN_PENDING: u32 = 4;
/// `player+0x7B0`: Sora is dead (Re:CoM's death reaction).
pub const DEAD: usize = 0x7B0;

/// `CBattleEnemyManager` fields (the step above).
pub mod manager {
    /// `CTask`'s child count (`0x1402AB850` clears it; `0x1402ABBA0` counts).
    pub const CHILDREN: usize = 0x50;
    pub const STATE: usize = 0x58;
    pub const TIMER: usize = 0x5C;
    /// Prize ids 0xD6 / 0xD7 picked up (`0x1401F4AB0`): the roulette after the win pose.
    pub const ROULETTE_A: usize = 0x6C;
    pub const ROULETTE_B: usize = 0x70;
    pub const COUNT: usize = 0x98;
}

/// `Gauge::CPrizeGauge` (`0x14087B690`, made by `CPlayerGauge`'s init or by `0x14020D940`):
/// the get-card display's state and the EXP display's state. Checked (decomp `0x1402094F0`,
/// `0x14020A710`).
pub mod prize {
    pub const CARD: usize = 0x94;
    pub const EXP: usize = 0x5C;
}

/// The roulette's task id (`0x14011A950`: `0x1402AC0A0(this, 0x22)`).
pub const TASK_ROULETTE: u32 = 0x22;

/// The manager as read this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Manager {
    pub state: i32,
    pub timer: i32,
    pub children: i32,
    pub count: i32,
    pub roulette: (i32, i32),
}

/// What Re:CoM's battle end looks like this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// Sora: action `+0x40C`, sub-state `+0x414`, phase `+0x604`, dead `+0x7B0`, win pending.
    pub action: u32,
    pub sub: i32,
    pub phase: u32,
    pub dead: bool,
    pub win_pending: bool,
    pub manager: Option<Manager>,
    /// `CPrizeGauge` (`+0x94`, `+0x5C`), if one exists.
    pub prize: Option<(i32, i32)>,
    /// The gauge pause `0x14087B808` (`0x140232050`; the roulette, save, world warp and shop
    /// screens set it): the prize display doesn't step while it is set.
    pub gauge_pause: i32,
    /// The roulette task 0x22 exists.
    pub roulette_task: bool,
    /// Event flag (1, 2): the manager reached state 7.
    pub over: bool,
    /// Re:CoM's HUD-hide mask, variable (1, 0xE2).
    pub hud_mask: u32,
}

/// Sora's win pose is done as `0x1401F3C90` tests it.
pub fn win_pose_done(v: &View) -> bool {
    !v.dead && v.action == WIN_ACTION && v.sub > 2
}

/// What Re:CoM is waiting for, from the manager's state (the table above).
pub fn waiting_for(v: &View) -> String {
    let Some(m) = v.manager else {
        return "no battle enemy manager (task 0x19)".into();
    };
    let prize = |p: Option<(i32, i32)>| p.map_or("no prize gauge".to_string(), |(c, e)| format!("prize gauge card {c} exp {e}"));
    match m.state {
        1 => format!("the battle's tasks and enemies to end (children {}, enemy count {})", m.children, m.count),
        2 if v.prize.is_some_and(|p| p.0 != 0) => format!("Re:CoM's prize display ({}): next step goes to 3", prize(v.prize)),
        2 => format!("the after-battle wait ({} ticks left)", m.timer),
        3 => {
            let paused = if v.gauge_pause != 0 { ", gauges paused (0x14087B808)" } else { "" };
            format!("Re:CoM's prize display to close ({}{paused})", prize(v.prize))
        }
        4 if win_pose_done(v) => "nothing: Sora's win pose is done, the next step goes on".into(),
        4 => format!("Sora's win pose (action {:#x}, sub {}, dead {})", v.action, v.sub, v.dead),
        5 | 6 => format!("the prize roulette (task 0x22 {})", if v.roulette_task { "running" } else { "gone: the next step goes on" }),
        7 => "the room script (flag (1, 2) set: Re:CoM's battle is over)".into(),
        s => format!("manager state {s}"),
    }
}

/// The log line (`battle end: …`).
pub fn line(v: &View) -> String {
    let m = v.manager.map_or("none".to_string(), |m| {
        format!("state {} timer {} children {} count {} roulette {}/{}", m.state, m.timer, m.children, m.count, m.roulette.0, m.roulette.1)
    });
    format!(
        "Sora action {:#x} sub {} phase {} dead {} win-pending {}; manager {m}; prize gauge {}; gauge pause {}; roulette task {}; flag (1, 2) {}; HUD mask {:#x}; waiting for {}",
        v.action,
        v.sub,
        v.phase,
        v.dead as u8,
        v.win_pending as u8,
        v.prize.map_or("none".to_string(), |(c, e)| format!("card {c} exp {e}")),
        v.gauge_pause,
        v.roulette_task as u8,
        v.over as u8,
        v.hud_mask,
        waiting_for(v)
    )
}

/// The F9 trace's token.
pub fn token(v: &View) -> String {
    let m = v.manager.map_or("-".to_string(), |m| format!("s{} t{} c{} n{}", m.state, m.timer, m.children, m.count));
    let p = v.prize.map_or("-".to_string(), |(c, e)| format!("{c}/{e}"));
    format!("end mgr={m} prize={p} pause={} roul={} over={} hud={:#x}", v.gauge_pause, v.roulette_task as u8, v.over as u8, v.hud_mask)
}

/// What [`key`] compares.
pub type Key = (u32, i32, bool, Option<(i32, i32, i32, (i32, i32))>, Option<(i32, i32)>, i32, bool, bool);

/// The parts whose change is worth a log line (not the timer).
pub fn key(v: &View) -> Key {
    (v.action, v.sub, v.win_pending, v.manager.map(|m| (m.state, m.children, m.count, m.roulette)), v.prize, v.gauge_pause, v.roulette_task, v.over)
}

/// Log lines at most (changes and the periodic repeat).
pub const LOG_LINES: u32 = 60;
/// While nothing changes, the line again every this many frames (10 s at 60 Hz).
pub const REPEAT_FRAMES: u32 = 600;

#[cfg(test)]
mod tests {
    use super::*;

    fn win(sub: i32) -> View {
        View { action: WIN_ACTION, sub, phase: 3, win_pending: true, ..Default::default() }
    }

    #[test]
    fn win_pose_done_is_re_coms_test() {
        // `0x1401F3C90`: `cmp [+0x414],2; jg` → done from sub 3; dead or another action: not.
        assert!(!win_pose_done(&win(2)));
        assert!(win_pose_done(&win(3)));
        assert!(win_pose_done(&win(4)));
        assert!(!win_pose_done(&View { dead: true, ..win(4) }));
        assert!(!win_pose_done(&View { action: 2, ..win(4) }));
    }

    #[test]
    fn names_what_the_manager_waits_for() {
        let m = |state| Some(Manager { state, timer: 1200, children: 0, count: 0, roulette: (0, 0) });
        assert!(waiting_for(&View::default()).starts_with("no battle enemy manager"));
        assert!(waiting_for(&View { manager: Some(Manager { state: 1, children: 2, ..Default::default() }), ..win(4) }).contains("children 2"));
        assert!(waiting_for(&View { manager: m(2), ..win(4) }).contains("1200 ticks"));
        assert!(waiting_for(&View { manager: m(2), prize: Some((2, 0)), ..win(4) }).contains("prize display"));
        let w = waiting_for(&View { manager: m(3), prize: Some((2, 3)), gauge_pause: 1, ..win(4) });
        assert!(w.contains("card 2 exp 3") && w.contains("paused"), "{w}");
        assert!(waiting_for(&View { manager: m(4), ..win(2) }).contains("win pose (action 0x1c, sub 2"));
        assert!(waiting_for(&View { manager: m(4), ..win(4) }).starts_with("nothing"));
        assert!(waiting_for(&View { manager: m(5), roulette_task: true, ..win(4) }).contains("running"));
        assert!(waiting_for(&View { manager: m(7), over: true, ..win(4) }).contains("room script"));
    }

    #[test]
    fn line_and_token() {
        let v = View { manager: Some(Manager { state: 3, timer: 0, children: 0, count: 0, roulette: (0, 0) }), prize: Some((2, 3)), hud_mask: 0x10, ..win(4) };
        let l = line(&v);
        assert!(l.starts_with("Sora action 0x1c sub 4 phase 3 dead 0 win-pending 1; manager state 3"), "{l}");
        assert!(l.ends_with("waiting for Re:CoM's prize display to close (prize gauge card 2 exp 3)"), "{l}");
        assert_eq!(token(&v), "end mgr=s3 t0 c0 n0 prize=2/3 pause=0 roul=0 over=0 hud=0x10");
        // The timer alone doesn't make a new line.
        let mut w = v;
        w.manager.as_mut().unwrap().timer = 50;
        assert_eq!(key(&v), key(&w));
        w.manager.as_mut().unwrap().state = 4;
        assert_ne!(key(&v), key(&w));
    }
}
