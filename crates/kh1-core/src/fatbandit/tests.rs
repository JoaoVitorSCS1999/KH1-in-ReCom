use super::*;
use crate::enemy::host::model::Model;

/// The player's `ex_2060.bd` / children / `btltbl.bin` / KH1 exe numbers as the asset step reads them (its manifest on
/// the session's files), and the host's points (any: the behaviour never reads them).
pub(super) fn consts() -> Consts {
    let v: &[(&str, f32)] = &[
        ("walk_speed", 2.0), ("run_speed", 6.0), ("turn_deg", 10.0), ("look_cone_in", 45.0), ("look_cone_out", 60.0), ("look_up", 20.0), ("look_down", 15.0),
        ("look_left", 90.0), ("look_right", 90.0), ("idle_search", 1500.0), ("lost_search", 10000.0), ("wander_inner_min", 200.0), ("wander_inner_max", 400.0),
        ("wander_outer_min", 400.0), ("wander_outer_max", 500.0), ("wander_speed", 2.0), ("wander_time", 300.0), ("wander_reaim", 4.0), ("wander_stop", 60.0),
        ("wander_done_chance", 0.7), ("wander_idle_chance", 0.4), ("idle_blend", 8.0), ("wander_idle_min", 120.0), ("wander_idle_max", 180.0), ("close_range", 300.0),
        ("mid_range", 600.0), ("close_belly_chance", 0.7), ("close_fire_chance", 0.4), ("mid_volley_chance", 0.3), ("mid_approach_time", 180.0),
        ("mid_approach_dist", 300.0), ("far_breath_chance", 0.8), ("far_approach_time", 180.0), ("far_approach_dist", 600.0), ("approach_speed", 6.0),
        ("approach_reaim", 6.0), ("route_turn_deg", 5.0), ("belly_blend", 14.0), ("belly_wait", 75.0), ("windup_blend", 8.0), ("windup_open", 68.0),
        ("windup_hold", 90.0), ("breath_blend", 8.0), ("breath_wait", 8.0), ("breath_aim_wait", 60.0), ("flame_time", 140.0), ("combo_gap", 18.0),
        ("combo_blend", 18.0), ("combo_from", 40.0), ("combo_wait", 38.0), ("quick_blend", 18.0), ("quick_from", 40.0), ("quick_wait", 18.0), ("fire_blend", 8.0),
        ("fire_from", 0.0), ("fire_wait", 64.0), ("volley_count", 2.0), ("volley_gap", 6.0), ("volley_blend", 4.0), ("volley_from", 12.0), ("volley_wait", 48.0),
        ("volley_rest", 60.0), ("appear_blend", 0.0), ("appear_fade", 30.0), ("appear_wait", 30.0), ("appear_then_blend", 0.0), ("stagger_front_deg", 80.0),
        ("count_reset", 3.0), ("count_flinch", 1.0), ("count_other", 2.0), ("count_second", 1.0), ("hit_sora_chance", 0.25), ("stagger_exp_div", 5.0),
        ("stagger_exp_min", 1.0), ("hit_count_max", 2.0), ("hit_flame_chance", 0.8), ("fire_t", 2.5), ("fire_turn", 0.0), ("fire_speed", 45.0), ("fire_life", 240.0),
        ("fire_fade", 20.0), ("fire_radius", 30.0), ("fire_recom_id", 1.0), ("fire_kind", 1.0), ("flame_rehit", 60.0), ("flame_t0", 3.0), ("flame_life", 140.0),
        ("flame_lift0", 25.0), ("flame_t1", 15.0), ("flame_lift1", 75.0), ("flame_r0", 23.0), ("flame_r1", 60.0), ("flame_grow", 60.0), ("flame_grow_div", 60.0),
        ("attack_blend", 8.0), ("air_accel", 0.1), ("air_drag", 0.95), ("fall_cap", 20.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("launch_min", 20.0),
        ("launch_max", 35.0), ("launch_hop", 1.3499999), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("weight", 1.0),
    ];
    let mut v: Vec<(String, f32)> = v.iter().map(|(k, x)| (k.to_string(), *x)).collect();
    for p in ["fire_at", "eye", "flame0", "flame1"] {
        for (s, x) in [("bone", 1.0), ("x", 0.0), ("y", 0.0), ("z", 0.0)] {
            v.push((format!("{p}_{s}"), x));
        }
    }
    v.push(("gravity".into(), crate::params::GRAVITY));
    v.push(("turn_rate".into(), std::f32::consts::PI / 16.0));
    Table::new(v, &all_names()).unwrap()
}

/// The player's `xa_ex_2060.mset` lengths by request (the asset step's clips).
pub(super) fn frames() -> Vec<(u16, f32)> {
    vec![
        (0, 356.0), (1, 72.0), (2, 48.0), (6, 60.0), (7, 102.0), (66, 110.0), (67, 210.0), (68, 182.0), (69, 102.0), (70, 356.0), (72, 52.0), (73, 52.0), (74, 52.0),
        (75, 52.0), (76, 62.0), (77, 62.0), (78, 52.0), (79, 52.0), (80, 52.0), (81, 52.0), (82, 18.0), (83, 30.0), (84, 72.0), (85, 94.0), (86, 74.0), (87, 18.0),
        (88, 30.0), (89, 72.0), (90, 94.0), (91, 74.0), (200, 158.0), (201, 108.0), (202, 52.0), (205, 180.0), (206, 120.0), (207, 264.0), (208, 268.0),
    ]
}

/// The asset step's looping clips (the pose rule), by request.
pub(super) fn loops() -> Vec<u16> {
    vec![0, 1, 6, 67, 70, 83, 85, 88, 90, 93, 95, 98, 100, 102, 104, 211, 213, 216, 218]
}

/// A flat floor at y = 0, Sora at `sora`; the current request is the last one played (a restart plays it again).
struct Sim {
    ai: Ai,
    pos: Vec3,
    current: Option<u16>,
    moved: f32,
    log: Vec<(usize, Cmd)>,
    queued: Option<u16>,
    age: f32,
    f: usize,
}

impl Sim {
    fn run(&mut self, sora: Vec3, n: usize) {
        for _ in 0..n {
            let airborne = self.ai.body().airborne();
            let w = World { step: 1.0, pos: self.pos, target: Some(sora), grounded: !airborne, airborne, moved: self.moved, motion: self.current };
            for c in self.ai.frame(w) {
                match c {
                    ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Attack { request, .. } => {
                        if self.current != Some(request) {
                            self.current = Some(request);
                            self.queued = None;
                            self.age = 0.0;
                        }
                    }
                    ECmd::Restart { request, start, .. } => {
                        self.current = Some(request);
                        self.queued = None;
                        self.age = start;
                    }
                    ECmd::Queue { request, .. } => self.queued = Some(request),
                    _ => {}
                }
                self.log.push((self.f, c));
            }
            self.age += 1.0;
            if let Some(cur) = self.current {
                let len = frames().iter().find(|x| x.0 == cur).map_or(0.0, |x| x.1);
                if self.age >= len && !loops().contains(&cur) {
                    if let Some(q) = self.queued.take() {
                        self.current = Some(q);
                        self.age = 0.0;
                    }
                }
            }
            let c = self.ai.consts();
            let steer = self.ai.take_steer();
            let d = self.ai.body().step(1.0, self.pos.y, steer, &c);
            self.pos += d;
            self.moved = d.length();
            if self.pos.y >= 0.0 && self.ai.body().falling() {
                self.pos.y = 0.0;
                self.ai.body().land();
            }
            self.f += 1;
        }
    }

    fn cmds(&self) -> Vec<Cmd> {
        self.log.iter().map(|x| x.1.clone()).collect()
    }

    /// The frames on which `c` was asked for.
    fn frames_of(&self, pred: impl Fn(&Cmd) -> bool) -> Vec<usize> {
        self.log.iter().filter(|x| pred(&x.1)).map(|x| x.0).collect()
    }
}

fn sim(seed: u32, spawned: bool) -> Sim {
    Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: Vec3::ZERO, current: None, moved: 0.0, log: Vec::new(), queued: None, age: 0.0, f: 0 }
}

/// A spawned Fat Bandit past its appearance, idle, Sora far away.
fn ready(seed: u32) -> Sim {
    let mut s = sim(seed, true);
    s.run(Vec3::new(0.0, 0.0, 5000.0), 110);
    assert!(s.ai.risen());
    s.log.clear();
    s.f = 0;
    s
}

/// A Fat Bandit past its appearance running `f` alone (Sora its target, the hit state's thread).
fn with<F: std::future::Future<Output = ()> + 'static>(seed: u32, f: impl FnOnce(Cx) -> F) -> Sim {
    let mut s = ready(seed);
    s.ai.io.borrow_mut().has_target = true;
    s.ai.io.borrow_mut().state = (st::HIT, false);
    let fut = f(s.ai.io.clone());
    s.ai.behaviour = Some(Box::pin(async move {
        fut.await;
        halt().await;
    }));
    s
}

fn motions(log: &[Cmd]) -> Vec<u16> {
    log.iter()
        .filter_map(|c| match c {
            ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Restart { request, .. } | ECmd::Attack { request, .. } | ECmd::Action { request, .. } => Some(*request),
            _ => None,
        })
        .collect()
}

fn throws(log: &[Cmd]) -> Vec<u8> {
    log.iter().filter_map(|c| if let ECmd::Throw(n) = c { Some(*n) } else { None }).collect()
}

#[test]
fn the_names_cover_the_numbers() {
    assert!(Table::new(vec![], &all_names()).is_err());
    assert_eq!((consts().num("close_range"), consts().num("windup_open"), consts().num("fire_wait")), (300.0, 68.0, 64.0));
    assert_eq!((Attack::Belly.request(), Attack::Belly.rows()), (200, &[0x31C, 0x38C][..]));
    assert!(NAMES.iter().all(|n| !HOST_NAMES.contains(n)));
}

/// `0x254E`: the front row on from the first frame (mode 3), the turn rate 10° a frame; `0x20F8`: facing Sora, 69
/// (blend 0) with the fade-in, 30 frames, then paused until 69 ends (102 frames); the idle state, the wander.
#[test]
fn appearance_with_the_front_row_on() {
    let mut s = sim(1, true);
    s.run(Vec3::new(0.0, 0.0, 5000.0), 2);
    assert!(s.cmds().contains(&ECmd::Guard(Some((FRONT, ALWAYS)))), "{:?}", s.log);
    assert!(s.cmds().contains(&ECmd::Motion { request: req::APPEAR, blend: 0.0 }) && s.cmds().contains(&ECmd::FadeIn(30.0)));
    assert!((s.ai.body().turn.unwrap() - 10f32.to_radians()).abs() < 1e-6);
    s.run(Vec3::new(0.0, 0.0, 5000.0), 60);
    assert!(!s.ai.risen(), "paused while 69 plays");
    s.run(Vec3::new(0.0, 0.0, 5000.0), 50);
    assert!(s.ai.risen() && s.cmds().contains(&ECmd::State(st::IDLE)));
    s.run(Vec3::new(0.0, 0.0, 5000.0), 20);
    assert!(s.cmds().iter().any(|c| matches!(c, ECmd::Motion { request: req::WALK, .. })), "it wanders");
    assert_eq!(s.cmds().iter().filter(|c| matches!(c, ECmd::Guard(_))).count(), 1, "the row is made once");
}

/// The wander's route (`0x36BE`): 4 frames walking along the facing at 2, one frame still, the turn rate 5° while it
/// goes (10 after).
#[test]
fn the_wander_walks_in_bursts() {
    let mut s = ready(2);
    let mut moving = Vec::new();
    for _ in 0..40 {
        s.run(Vec3::new(0.0, 0.0, 5000.0), 1);
        moving.push(s.moved > 0.0);
    }
    let still: Vec<usize> = moving.iter().enumerate().filter(|(_, m)| !**m).map(|(i, _)| i).collect();
    assert!(still.windows(2).any(|w| w[1] - w[0] == 5), "a still frame after every 4: {moving:?}");
    assert!(s.moved <= 2.0 + 1e-3);
    assert!((s.ai.body().turn.unwrap() - 5f32.to_radians()).abs() < 1e-6);
}

/// The search: Sora within 1500 engages; farther, not; Sora targeting it engages between wanders.
#[test]
fn the_search() {
    let mut s = ready(3);
    s.run(Vec3::new(0.0, 0.0, 1400.0), 3);
    assert!(s.cmds().contains(&ECmd::State(st::ENGAGED)), "{:?}", s.log);
    let mut s = ready(3);
    s.run(Vec3::new(0.0, 0.0, 1600.0), 600);
    assert!(!s.cmds().contains(&ECmd::State(st::ENGAGED)));
    s.ai.set_targeted(true);
    s.run(Vec3::new(0.0, 0.0, 1600.0), 1500);
    assert!(s.cmds().contains(&ECmd::State(st::ENGAGED)), "targeted");
}

/// The breath (`0x1308`): 205 (blend 8); the front row off at 68; 207 at 158 (blend 8); the flame (child 1) 68 frames
/// later; to 207's end (the flame lasts at most 140 of its 264); the idle; the row back.
#[test]
fn the_breath() {
    let mut s = with(4, |cx| async move { breath(&cx).await });
    let sora = Vec3::new(0.0, 0.0, 250.0);
    s.run(sora, 500);
    let windup = s.frames_of(|c| matches!(c, ECmd::Motion { request: req::WINDUP, blend } if *blend == 8.0));
    let off = s.frames_of(|c| *c == ECmd::Guard(None));
    let breath = s.frames_of(|c| matches!(c, ECmd::Motion { request: req::BREATH, blend } if *blend == 8.0));
    let flame = s.frames_of(|c| *c == ECmd::Throw(child::FLAME));
    let back = s.frames_of(|c| *c == ECmd::Guard(Some((FRONT, ALWAYS))));
    let idle = s.frames_of(|c| matches!(c, ECmd::Motion { request: req::IDLE, .. }));
    assert_eq!(windup.len(), 1);
    let w = windup[0];
    assert_eq!((off[0] - w, breath[0] - w, flame[0] - w), (68, 158, 226), "the wind-up's 226 frames");
    let end = breath[0] + 264;
    assert!(idle.iter().any(|&f| f + 1 >= end && f <= end + 1), "the idle at 207's end: {idle:?} (207 from {})", breath[0]);
    assert!(back.iter().any(|&f| f + 1 >= end), "the row back after the flame");
}

/// The volley (`0x1610`): 201 (blend 8), the fireball 64 frames in; 6 frames; twice: 201 again from 12 (blend 4), the
/// fireball 48 frames later, 6 frames; to 201's end; the idle; 60 frames.
#[test]
fn the_volley() {
    let mut s = with(5, |cx| async move {
        volley(&cx).await;
        play(&cx, 99, 0.0);
    });
    s.run(Vec3::new(0.0, 0.0, 900.0), 400);
    let shots = s.frames_of(|c| *c == ECmd::Throw(child::FIREBALL));
    let again = s.frames_of(|c| matches!(c, ECmd::Restart { request: req::FIRE, start, blend } if *start == 12.0 && *blend == 4.0));
    assert_eq!(shots.len(), 3, "{:?}", s.log);
    assert_eq!(again.len(), 2);
    assert_eq!(shots[0], 64);
    assert_eq!((again[0] - shots[0], shots[1] - again[0], again[1] - shots[1], shots[2] - again[1]), (6, 48, 7, 48));
    // To 201's end (from 12, at 60 played + 6 + …), the idle, 60 frames.
    let idle = s.frames_of(|c| matches!(c, ECmd::Motion { request: req::IDLE, .. }));
    let done = s.frames_of(|c| matches!(c, ECmd::Motion { request: 99, .. }));
    assert_eq!(done[0] - idle.iter().copied().filter(|&f| f > shots[2]).next().unwrap(), 60);
}

/// Two fireballs and a flame (`0x129A`): the second 201 starts again from 0 while the first plays; 18 frames; 207
/// from 40 (blend 18); 38 frames; the flame.
#[test]
fn two_fireballs_and_a_flame() {
    let mut s = with(6, |cx| async move { fire_and_flame(&cx).await });
    s.run(Vec3::new(0.0, 0.0, 200.0), 400);
    let shots = s.frames_of(|c| *c == ECmd::Throw(child::FIREBALL));
    assert_eq!(shots, vec![64, 128]);
    assert!(s.cmds().contains(&ECmd::Restart { request: req::FIRE, blend: 8.0, start: 0.0 }));
    let b = s.frames_of(|c| matches!(c, ECmd::MotionAt { request: req::BREATH, blend, start } if *blend == 18.0 && *start == 40.0));
    let flame = s.frames_of(|c| *c == ECmd::Throw(child::FLAME));
    assert_eq!((b[0] - shots[1], flame[0] - b[0]), (18, 38));
}

/// The belly (`0x1750`): 200 as a plain request with its rows (Re:CoM's attack state, the AI runs on), to its end (158
/// frames); the idle; 75 frames.
#[test]
fn the_belly() {
    let mut s = with(7, |cx| async move {
        belly(&cx).await;
        play(&cx, 99, 0.0);
    });
    s.run(Vec3::new(0.0, 0.0, 200.0), 300);
    let a = s.frames_of(|c| *c == ECmd::Attack { request: req::BELLY, attack: Attack::Belly, blend: 14.0 });
    let idle = s.frames_of(|c| matches!(c, ECmd::Motion { request: req::IDLE, .. }));
    let done = s.frames_of(|c| matches!(c, ECmd::Motion { request: 99, .. }));
    assert_eq!(a.len(), 1);
    assert_eq!((idle[0] - a[0], done[0] - idle[0]), (157, 75));
}

/// The rounds by distance: close (< 300) the belly or the breath; mid (< 600) the volley or the approach, breath and
/// belly; far (≥ 600) the approach and the breath, or the volley.
#[test]
fn rounds_by_distance() {
    let mut seen = [[false; 2]; 3];
    for seed in 0..24u32 {
        for (i, d) in [200.0, 450.0, 1200.0].into_iter().enumerate() {
            let mut s = with(seed, move |cx| async move {
                let d = crate::enemy::script::dist_to_target(&cx);
                if d < 300.0 {
                    close(&cx).await
                } else if d < 600.0 {
                    mid(&cx).await
                } else {
                    far(&cx).await
                }
            });
            s.run(Vec3::new(0.0, 0.0, d), 30);
            let m = motions(&s.cmds());
            match i {
                0 => {
                    assert!(m.contains(&req::BELLY) || m.contains(&req::WINDUP), "{m:?}");
                    seen[0][m.contains(&req::BELLY) as usize] = true;
                }
                1 => {
                    assert!(m.contains(&req::FIRE) || m.contains(&req::RUN), "{m:?}");
                    seen[1][m.contains(&req::FIRE) as usize] = true;
                }
                _ => {
                    assert!(m.contains(&req::FIRE) || m.contains(&req::RUN), "{m:?}");
                    seen[2][m.contains(&req::FIRE) as usize] = true;
                }
            }
        }
    }
    assert!(seen.iter().all(|r| r[0] && r[1]), "both branches of each: {seen:?}");
}

/// The hit handler: a flinch or a knock-back takes no reaction and asks for no state (`sys 42`), counting the run's
/// first two hits; on its front while its row is down (the breath), a non-flinch hit staggers it (the row back, the hit
/// state: 208 and the EXP bonus max(EXP / 5, 1)); from behind it puts the row back and nothing else; a clash asks for
/// the hit state.
#[test]
fn the_hit_handler() {
    let s = ready(8);
    assert!(s.ai.armoured(Some(req::BELLY), 0, Some(5)) && !s.ai.keeps_reaction());
    s.ai.set_hit_kind(3);
    s.ai.set_hit_count(1);
    s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
    assert_eq!((s.ai.io.borrow().state.1, s.ai.io.borrow().x.hits), (false, 1));
    assert!(s.ai.io.borrow().body.push.is_some());
    s.ai.set_hit_kind(5);
    s.ai.set_hit_count(2);
    s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
    assert_eq!((s.ai.io.borrow().state.1, s.ai.io.borrow().x.hits), (false, 2));
    s.ai.set_hit_count(3);
    s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
    assert_eq!(s.ai.io.borrow().x.hits, 2, "a run's third hit isn't counted");
    s.ai.set_hit_count(1);
    s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
    assert_eq!(s.ai.io.borrow().x.hits, 4, "a run's first knock-back +2");
    s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
    assert_eq!(s.ai.io.borrow().x.hits, 2, "past 3: back to 0, then +2");
    assert_eq!(s.ai.reaction_kind(5), 4);
    // The breath's open front (facing +z, Sora at +z: the hit pushes it along −z).
    let mut s = with(9, |cx| async move { breath(&cx).await });
    let sora = Vec3::new(0.0, 0.0, 250.0);
    s.run(sora, 80);
    assert!(s.ai.io.borrow().x.open);
    s.ai.set_hit_kind(3);
    s.ai.set_hit_count(1);
    s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
    assert!(!s.ai.io.borrow().x.stagger && !s.ai.io.borrow().x.open, "a flinch on its front: no stagger, the row back (`0x2EC6`)");
    assert_eq!(s.ai.take_now(), vec![ECmd::Guard(Some((FRONT, ALWAYS)))]);
    assert!(!s.ai.io.borrow().state.1);
    let mut s = with(9, |cx| async move { breath(&cx).await });
    s.run(sora, 80);
    s.ai.set_hit_kind(4);
    s.ai.set_hit_count(1);
    s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, 1.0));
    assert!(!s.ai.io.borrow().x.stagger && !s.ai.io.borrow().x.open, "a knock-back from behind: the row back");
    assert_eq!(s.ai.take_now(), vec![ECmd::Guard(Some((FRONT, ALWAYS)))]);
    assert!(!s.ai.io.borrow().state.1);
    // The breath goes on (no state): its flame still comes.
    s.run(sora, 200);
    assert!(s.cmds().contains(&ECmd::Throw(child::FLAME)));
    let mut s = with(9, |cx| async move { breath(&cx).await });
    s.run(sora, 80);
    s.ai.set_hit_kind(5);
    s.ai.set_hit_count(1);
    s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, -1.0));
    assert!(s.ai.io.borrow().x.stagger && s.ai.io.borrow().state == (st::HIT, true), "a knock-back on its front: the stagger");
    assert_eq!(s.ai.take_now(), vec![ECmd::Guard(Some((FRONT, ALWAYS)))]);
    s.log.clear();
    s.run(sora, 5);
    assert!(s.cmds().contains(&ECmd::Restart { request: req::STAGGER, blend: 0.0, start: 0.0 }) || s.cmds().contains(&ECmd::Motion { request: req::STAGGER, blend: 0.0 }), "{:?}", s.log);
    assert!(s.cmds().contains(&ECmd::Reward(Reward { exp_div: 5, exp_add: 0, exp_min: 1, prizes: [0; 7] })));
    assert_eq!(Reward { exp_div: 5, exp_add: 0, exp_min: 1, prizes: [0; 7] }.exp_bonus(3), 1);
    assert_eq!(Reward { exp_div: 5, exp_add: 0, exp_min: 1, prizes: [0; 7] }.exp_bonus(42), 8);
    s.run(sora, 270);
    assert!(s.cmds().contains(&ECmd::State(st::ENGAGED)), "then engaged");
    // A clash (Sora's Guard on the belly, a swing into its hands): the hit state.
    let s = ready(10);
    s.ai.set_hit_kind(6);
    Behaviour::on_hit(&s.ai, HitNews::Clash);
    assert_eq!(s.ai.io.borrow().state, (st::HIT, true));
}

/// The hit state's counter (`0x7A6`): at most 2 counted hits, 80 % the quick flame else two fireballs and a flame; more:
/// the count back to 0 and the volley.
#[test]
fn the_counter() {
    let mut flame = false;
    let mut fires = false;
    for seed in 0..12u32 {
        let mut s = ready(seed);
        s.ai.io.borrow_mut().has_target = true;
        s.ai.io.borrow_mut().x.hits = 2;
        s.ai.io.borrow_mut().state = (st::HIT, true);
        s.run(Vec3::new(0.0, 0.0, 400.0), 80);
        let m = motions(&s.cmds());
        flame |= m.first() == Some(&req::BREATH);
        fires |= m.first() == Some(&req::FIRE);
        assert!(m.first() == Some(&req::BREATH) || m.first() == Some(&req::FIRE), "{m:?}");
    }
    assert!(flame && fires);
    let mut s = ready(1);
    s.ai.io.borrow_mut().has_target = true;
    s.ai.io.borrow_mut().x.hits = 3;
    s.ai.io.borrow_mut().state = (st::HIT, true);
    s.run(Vec3::new(0.0, 0.0, 400.0), 400);
    assert_eq!(throws(&s.cmds()), vec![0, 0, 0], "the volley");
    assert_eq!(s.ai.io.borrow().x.hits, 0);
}

/// The owner's question (KH1's pace): from the end of each attack to the next one, KH1's waits (the belly's 75
/// frames, the volley's 60, the breath's 226-frame wind-up) and its walks: over a long fight at 2–7 m the Fat Bandit
/// starts an attack (the belly, a fireball or the wind-up) on average no more often than every 4 seconds.
#[test]
fn kh1_pace_leaves_openings() {
    for seed in 0..6u32 {
        let mut s = ready(seed);
        for f in 0..12_000usize {
            let a = f as f32 / 900.0;
            let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 2100.0).sin());
            s.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
        }
        // A round's first attack: the belly, the breath's wind-up, a first fireball (not the volley's or the second
        // one's restart), a counter's flame from 40.
        let starts = s
            .log
            .iter()
            .filter(|(_, c)| matches!(c, ECmd::Attack { .. } | ECmd::Motion { request: req::WINDUP | req::FIRE, .. } | ECmd::MotionAt { request: req::BREATH, .. }))
            .count();
        let per = 12_000.0 / starts as f32;
        println!("seed {seed}: a round every {per:.0} frames ({starts} in 12 000)");
        assert!(per > 200.0, "seed {seed}: an attack every {per:.0} frames ({starts})");
    }
}

/// A long fight: the AI acts at least every 600 frames (the host's quiet watchdog), with Sora circling 2–7 m away and
/// a hit every 150–400 frames (12 seeds, 20 000 frames each).
#[test]
fn a_long_fight_never_stalls() {
    for seed in 0..12u32 {
        let mut s = ready(seed);
        let (mut last, mut worst, mut next_hit) = (0usize, 0usize, 300usize);
        let mut r = Kh1Rand::new(seed + 100);
        for f in 0..20_000usize {
            let a = f as f32 / 600.0;
            let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
            let sora = Vec3::new(a.sin() * rad, 0.0, a.cos() * rad);
            if f == next_hit {
                let kind = [3u8, 3, 4, 5][(r.next_f32() * 4.0) as usize % 4];
                s.ai.set_hit_kind(kind);
                s.ai.set_hit_count(1 + (r.next_f32() * 3.0) as u16);
                let dir = if r.next_f32() < 0.5 { 1.0 } else { -1.0 };
                if r.next_f32() < 0.15 {
                    Behaviour::on_hit(&s.ai, HitNews::Clash);
                } else {
                    assert!(s.ai.armoured(s.current, 0, Some(kind)));
                    s.ai.on_armoured_hit(Vec3::new(0.0, 0.0, dir));
                }
                let _ = s.ai.take_now();
                next_hit = f + 150 + (r.next_f32() * 250.0) as usize;
            }
            let n = s.log.len();
            s.run(sora, 1);
            if s.log[n..].iter().any(|(_, c)| !matches!(c, ECmd::Yaw(_) | ECmd::Motion { request: 1 | 2, .. })) || s.ai.body().walk != Vec3::ZERO || s.moved > 0.0 {
                worst = worst.max(f - last);
                last = f;
            }
        }
        assert!(worst < 600, "seed {seed}: {worst} frames without acting");
    }
}

/// The Fat Bandit through the host's rules (`crate::enemy::host::model`, what `kh1-recom`'s `win::kh1_enemy` decides)
/// with its class map (the belly in state 2) and the asset step's looping clips.
fn hosted(s: Sim) -> Model<Consts> {
    let Sim { ai, pos, .. } = s;
    let mut m = Model::new(Box::new(ai), consts(), frames(), loops(), vec![2]);
    m.pos = pos;
    m
}

/// The belly in the host: 200 asked for once and played in state 2 for its 158 frames, never released; the volley's
/// 201 restarted twice in state 1.
#[test]
fn hosted_the_belly_and_the_volley() {
    let s = with(12, |cx| async move {
        belly(&cx).await;
    });
    let mut m = hosted(s);
    m.run(Vec3::new(0.0, 0.0, 200.0), 300);
    assert_eq!(m.released, 0);
    assert_eq!(m.attacks_of(req::BELLY).len(), 1);
    let run = m.longest_run(req::BELLY);
    assert!((156..=158).contains(&run), "200's 158 frames: {run}");
    assert!(m.trace.iter().all(|t| t.1 != Some(req::BELLY) || t.0 == 2));
    let s = with(13, |cx| async move {
        volley(&cx).await;
    });
    let mut m = hosted(s);
    m.run(Vec3::new(0.0, 0.0, 900.0), 400);
    let shots: Vec<usize> = m.log.iter().filter(|(_, c)| *c == ECmd::Throw(child::FIREBALL)).map(|x| x.0).collect();
    assert_eq!(shots.len(), 3, "{:?}", m.log);
    assert!(m.trace.iter().all(|t| t.0 == 1));
}

/// A long fight in the host (12 seeds, 20 000 frames, Sora circling 2–7 m away): no attack is released, none is asked
/// for again within two frames, the belly plays only in state 2.
#[test]
fn hosted_a_long_fight_never_loops_an_attack() {
    for seed in 0..12u32 {
        let mut m = hosted(ready(seed));
        for f in 0..20_000usize {
            let a = f as f32 / 600.0;
            let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
            m.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
        }
        assert_eq!(m.released, 0, "seed {seed}");
        let a = m.attacks_of(req::BELLY);
        assert!(a.windows(2).all(|w| w[1] - w[0] > 2), "seed {seed}: {a:?}");
        assert!(!a.is_empty(), "seed {seed}: it bellies");
        assert!(m.trace.iter().all(|t| t.1 != Some(req::BELLY) || t.0 == 2), "seed {seed}");
    }
}

/// [`fire_dir`]: straight at a target in front (pitch and turn inside the limits), clamped to 15° down / 20° up and
/// 90° to a side, straight ahead outside the cone or with none.
#[test]
fn the_fireballs_aim() {
    let c = |yaw: f32, eye: Vec3, t: Option<Vec3>| fire_dir(yaw, eye, t, 60.0, 20.0, 15.0, 90.0, 90.0);
    let d = c(0.0, Vec3::new(0.0, -150.0, 0.0), Some(Vec3::new(0.0, -100.0, 500.0)));
    let want = Vec3::new(0.0, 50.0, 500.0).normalized();
    assert!((d - want).length() < 1e-5, "{d:?}");
    // Far below: 15° down at most (y down).
    let d = c(0.0, Vec3::new(0.0, -150.0, 0.0), Some(Vec3::new(0.0, 0.0, 200.0)));
    assert!((d.y.asin().to_degrees() - 15.0).abs() < 1e-3, "{d:?}");
    // Above: 20° up at most.
    let d = c(0.0, Vec3::ZERO, Some(Vec3::new(0.0, -300.0, 400.0)));
    assert!((d.y.asin().to_degrees() + 20.0).abs() < 1e-3);
    // To the side inside the cone: the turn kept.
    let d = c(0.0, Vec3::ZERO, Some(Vec3::new(300.0, 0.0, 400.0)));
    assert!((d.x.atan2(d.z) - 300f32.atan2(400.0)).abs() < 1e-5);
    // Outside the cone (70° to the side) and with no target: ahead.
    let d = c(0.5, Vec3::ZERO, Some(Vec3::from_yaw(0.5 + 70f32.to_radians()) * 500.0));
    assert!((d - Vec3::from_yaw(0.5)).length() < 1e-6);
    assert_eq!(c(0.3, Vec3::ZERO, None), Vec3::from_yaw(0.3));
    assert!((d.length() - 1.0).abs() < 1e-5);
}

/// [`flame_spheres`]: none at t 0; at t 30 the line and the radii halved; grown, spheres every 41.5 from the start, the
/// radius stepping from 23 toward 60, one at least.
#[test]
fn the_flame_spheres() {
    let (a, b) = (Vec3::ZERO, Vec3::new(0.0, 0.0, 300.0));
    assert!(flame_spheres(a, b, 23.0, 60.0, 0.0, 60.0, 60.0).is_empty());
    let full = flame_spheres(a, b, 23.0, 60.0, 60.0, 60.0, 60.0);
    assert_eq!(full.len(), 8, "300 / 41.5 → 8 spheres: {full:?}");
    assert!((full[1].0.z - 41.5).abs() < 1e-4 && (full[0].1 - 23.0).abs() < 1e-6);
    let dr = 37.0 * 41.5 / 300.0;
    assert!((full[7].1 - (23.0 + 7.0 * dr)).abs() < 1e-3);
    let half = flame_spheres(a, b, 23.0, 60.0, 30.0, 60.0, 60.0);
    assert_eq!(half.len(), 8, "the count stays as both scale");
    assert!((half[1].0.z - 20.75).abs() < 1e-4 && (half[0].1 - 11.5).abs() < 1e-6);
    let one = flame_spheres(a, a, 23.0, 60.0, 90.0, 60.0, 60.0);
    assert_eq!(one, vec![(a, 23.0)]);
}
