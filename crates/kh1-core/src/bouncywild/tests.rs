use super::*;
use crate::enemy::host::model::Model;

/// The player's `ex_2040.bd` / children / `btltbl.bin` / KH1 exe numbers as the asset step reads them (its manifest on
/// the session's files).
pub(super) fn consts() -> Consts {
    let v: &[(&str, f32)] = &[
        ("walk_speed", 5.0), ("run_speed", 13.0), ("hop_height", 500.0), ("idle_search", 1500.0), ("appear_blend", 0.0), ("appear_fade", 30.0),
        ("appear_wait", 30.0), ("appear_then_blend", 0.0), ("wander_inner", 350.0), ("wander_outer", 450.0), ("rest_blend", 12.0), ("rest_min", 120.0),
        ("rest_max", 180.0), ("stretch_blend", 8.0), ("wander_band", 100.0), ("wander_timeout", 300.0), ("wander_idle_chance", 0.4), ("wander_idle_blend", 8.0),
        ("wander_idle_min", 90.0), ("wander_idle_max", 180.0), ("wander_home_max", 700.0), ("rounds", 5.0), ("close_range", 400.0), ("mid_range", 800.0),
        ("far_banana_chance", 0.8), ("mid_peel_chance", 0.7), ("ready_blend", 8.0), ("ready_close", 300.0), ("call_blend", 8.0), ("call_end_blend", 12.0),
        ("taunt_blend", 8.0), ("taunt_min", 90.0), ("taunt_range", 800.0), ("taunt_end_blend", 8.0), ("recover_blend", 8.0), ("turn_blend", 8.0),
        ("notice_blend", 8.0), ("throw_blend", 8.0), ("peel_count", 1.0), ("toss_blend", 0.0), ("dance_blend", 8.0), ("dance_close", 300.0), ("dance_legs", 8.0),
        ("dance_min", 250.0), ("dance_max", 300.0), ("dance_leg_time", 180.0), ("dance_fx_every", 52.0), ("bounce_blend", 8.0), ("bounce_speed", 13.0),
        ("stuck_speed", 6.5), ("arrive_dist", 100.0), ("pose_wait", 8.0), ("flee_dist", -800.0), ("flee_first", 10.0), ("flee_time", 180.0), ("drop_count", 1.0),
        ("leap_speed", 33.0), ("leap_height", 130.0), ("leap_drag", 0.9), ("leap_time", 50.0), ("stuck_factor", 0.5), ("hit_range", 400.0),
        ("own_launch_hop", 350.0), ("own_launch_speed", 35.0), ("own_launch_down", 0.0), ("fire_child", 0.0), ("peel_child", 1.0), ("drop_child", 2.0),
        ("attack_blend", 8.0), ("air_accel", 0.1), ("air_drag", 0.95), ("fall_cap", 20.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("launch_min", 20.0),
        ("launch_max", 35.0), ("launch_hop", 1.3499999), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("weight", 1.0),
        ("fire_wait", 82.0), ("fire_t", 1.0), ("fire_turn", 1.25), ("fire_speed", 25.0), ("fire_life", 75.0), ("fire_fade", 10.0), ("peel_sweep", 30.0),
        ("peel_gravity", 0.8), ("peel_lie", 120.0), ("peel_reach", 50.0), ("peel_protect", 30.0), ("peel_wait", 80.0), ("peel_t", 1.0), ("peel_spread", 5.0),
        ("peel_speed_min", 8.0), ("peel_speed_max", 10.0), ("peel_up_min", 10.0), ("peel_up_max", 15.0), ("peel_spread_min", 1.5), ("peel_spread_max", 2.5),
        ("drop_t", 1.0), ("drop_speed_min", 3.0), ("drop_speed_max", 10.0), ("drop_up_min", 5.0), ("drop_up_max", 10.0), ("fire_at_bone", 28.0),
        ("fire_at_x", 0.054049924), ("fire_at_y", 0.0), ("fire_at_z", 0.0), ("fire_radius", 50.0), ("fire_recom_id", 1.0), ("fire_kind", 1.0),
    ];
    let mut v: Vec<(String, f32)> = v.iter().map(|(k, x)| (k.to_string(), *x)).collect();
    v.push(("gravity".into(), crate::params::GRAVITY));
    v.push(("turn_rate".into(), std::f32::consts::PI / 16.0));
    Table::new(v, &all_names()).unwrap()
}

/// The player's `xa_ex_2040.mset` lengths by request (the asset step's clips).
pub(super) fn frames() -> Vec<(u16, f32)> {
    vec![
        (0, 144.0), (1, 64.0), (2, 52.0), (3, 6.0), (4, 18.0), (5, 12.0), (6, 40.0), (7, 16.0), (69, 80.0), (70, 60.0), (72, 40.0), (73, 40.0), (74, 40.0),
        (75, 40.0), (78, 44.0), (79, 44.0), (82, 4.0), (83, 12.0), (84, 44.0), (85, 40.0), (86, 56.0), (92, 12.0), (93, 12.0), (94, 44.0), (95, 40.0), (96, 56.0),
        (200, 140.0), (202, 48.0), (203, 52.0), (204, 46.0), (205, 122.0), (206, 130.0), (207, 240.0), (208, 206.0), (209, 80.0), (211, 140.0), (212, 204.0),
        (213, 98.0), (215, 240.0), (216, 266.0), (217, 134.0),
    ]
}

/// The asset step's looping clips (the pose rule), by request.
pub(super) fn loops() -> Vec<u16> {
    vec![0, 1, 2, 6, 83, 85, 93, 95, 200, 203, 209, 212, 213, 215, 216]
}

fn len(r: u16) -> f32 {
    frames().iter().find(|x| x.0 == r).map_or(30.0, |x| x.1)
}

/// A flat floor at y = 0, Sora at `sora`; an action holds the AI for its motion's length (then its queued request
/// plays); a plain request plays until another, its queued one taking over at its end.
struct Sim {
    ai: Ai,
    pos: Vec3,
    busy: f32,
    current: Option<u16>,
    queued: Option<u16>,
    age: f32,
    moved: f32,
    log: Vec<(usize, Cmd)>,
    f: usize,
}

impl Sim {
    fn run(&mut self, sora: Vec3, n: usize) {
        for _ in 0..n {
            let airborne = self.ai.body().airborne();
            let w = World { step: 1.0, pos: self.pos, target: Some(sora), grounded: !airborne, airborne, moved: self.moved, motion: self.current };
            if self.busy > 0.0 {
                self.busy -= 1.0;
                if self.busy <= 0.0 {
                    self.current = self.queued.take();
                    self.age = 0.0;
                }
            } else {
                for c in self.ai.frame(w) {
                    match c {
                        ECmd::Action { request, then, .. } => {
                            self.busy = len(request);
                            self.current = Some(request);
                            self.queued = Some(then);
                            self.age = 0.0;
                        }
                        ECmd::Motion { request, .. } => {
                            if self.current != Some(request) {
                                self.current = Some(request);
                                self.queued = None;
                                self.age = 0.0;
                            }
                        }
                        ECmd::Restart { request, .. } => {
                            self.current = Some(request);
                            self.queued = None;
                            self.age = 0.0;
                        }
                        ECmd::Queue { request, .. } => self.queued = Some(request),
                        _ => {}
                    }
                    self.log.push((self.f, c));
                }
            }
            self.age += 1.0;
            if let Some(cur) = self.current {
                if self.busy <= 0.0 && self.age >= len(cur) && !loops().contains(&cur) {
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
            self.pos.y = self.pos.y.min(0.0);
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

    fn first(&self, p: impl Fn(&Cmd) -> bool) -> Option<usize> {
        self.log.iter().find(|x| p(&x.1)).map(|x| x.0)
    }
}

fn sim(seed: u32, spawned: bool) -> Sim {
    Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: Vec3::ZERO, busy: 0.0, current: None, queued: None, age: 0.0, moved: 0.0, log: Vec::new(), f: 0 }
}

/// A Bouncywild past its appearance, idle (Sora far away).
fn ready(seed: u32) -> Sim {
    let mut s = sim(seed, true);
    for _ in 0..200 {
        if s.ai.risen() {
            break;
        }
        s.run(Vec3::new(0.0, 0.0, 5000.0), 1);
    }
    assert!(s.ai.risen());
    s.log.clear();
    s.f = 0;
    s
}

/// A Bouncywild past its appearance running `f` alone (Sora its target, as a behaviour thread).
fn with<F: std::future::Future<Output = ()> + 'static>(seed: u32, f: impl FnOnce(Cx) -> F) -> Sim {
    let mut s = ready(seed);
    s.ai.io.borrow_mut().has_target = true;
    s.ai.io.borrow_mut().state = (st::HIT, false);
    let fut = f(s.ai.io.clone());
    s.ai.behaviour = Some(Box::pin(async move {
        fut.await;
        loop {
            frame().await;
        }
    }));
    s
}

#[test]
fn the_names_cover_the_numbers() {
    assert!(Table::new(vec![], &all_names()).is_err());
    assert_eq!(consts().num("leap_speed"), 33.0);
    assert!(Attack::ALL.is_empty());
    assert_eq!((child::BANANA, child::PEEL, child::DROP), (0, 1, 2));
}

/// The appearance: 69 (blend 0) and the 30-frame fade-in at once; the script held until 69 ends; then idle and
/// wandering.
#[test]
fn the_appearance_then_idle() {
    let mut s = sim(1, true);
    s.run(Vec3::new(0.0, 0.0, 5000.0), 2);
    assert_eq!(&s.cmds()[1..3], &[ECmd::Motion { request: req::APPEAR, blend: 0.0 }, ECmd::FadeIn(30.0)], "{:?}", s.cmds());
    s.run(Vec3::new(0.0, 0.0, 5000.0), 100);
    let idle = s.first(|c| *c == ECmd::State(st::IDLE)).expect("idle");
    assert!((79..=83).contains(&idle), "the script waits until 69 ends ({idle})");
    assert!(s.cmds().iter().any(|c| matches!(c, ECmd::Motion { request: req::WALK, .. })), "it wanders");
}

/// Sora comes within 15 m: engaged; the bounce-about (202, then 203 legs gliding at 13), the leap (3, then 4 with the
/// jump, 5 near the top, 6 queued) and the run-away dropping two peels, then its pose.
#[test]
fn the_first_round_bounces_leaps_and_runs_away() {
    let mut s = ready(3);
    s.run(Vec3::new(0.0, 0.0, 1200.0), 2500);
    let c = s.cmds();
    assert!(c.contains(&ECmd::State(st::ENGAGED)));
    let dance = s.first(|c| matches!(c, ECmd::Motion { request: req::DANCE, .. })).expect("202");
    let bounce = s.first(|c| matches!(c, ECmd::Motion { request: req::BOUNCE, .. })).expect("203");
    let leap = s.first(|c| matches!(c, ECmd::Motion { request: req::LEAP, .. })).expect("3");
    let drop = s.first(|c| *c == ECmd::Throw(child::DROP)).expect("a peel dropped");
    assert!(dance < bounce && bounce < leap && leap < drop, "{dance} {bounce} {leap} {drop}");
    assert_eq!(c.iter().filter(|x| matches!(x, ECmd::Action { request: req::POSE, .. })).count() >= 2, true);
    assert!(c.contains(&ECmd::Queue { request: 6, blend: 0.0 }), "the leap's fall queued at the top");
    assert!(c.iter().filter(|x| **x == ECmd::Throw(child::DROP)).count() >= 2);
}

/// The leap: facing Sora, 33 a frame with KH1's drag for 50 frames: about 6 m, 1.3 m high, landing on its own.
#[test]
fn the_leap_travels_its_kh1_distance() {
    let mut s = with(4, |cx| async move { leap(&cx).await });
    let z0 = s.pos.z;
    let mut top = 0.0f32;
    for _ in 0..80 {
        s.run(Vec3::new(0.0, 0.0, 300.0), 1);
        top = top.min(s.pos.y);
    }
    // 33 × Σ √0.9^k over the 50 frames ≈ 33 × (1 − 0.9487^50) / (1 − 0.9487) ≈ 600.
    assert!((s.pos.z - z0 - 600.0).abs() < 40.0, "z {} → {}", z0, s.pos.z);
    assert!((top + 130.0).abs() < 10.0, "its top {top}");
    assert!(!s.ai.body().airborne() && s.ai.body().timed.is_none());
}

/// The run-away: straight away from Sora at the run speed, a peel dropped after 10 frames and one at the end.
#[test]
fn the_run_away_flees_from_sora_dropping_peels() {
    let mut s = with(5, |cx| async move { run_away(&cx).await });
    s.run(Vec3::new(0.0, 0.0, 200.0), 300);
    let drops: Vec<usize> = s.log.iter().filter(|x| x.1 == ECmd::Throw(child::DROP)).map(|x| x.0).collect();
    assert_eq!(drops.len(), 2, "{:?}", s.cmds());
    assert!((9..=12).contains(&drops[0]), "{drops:?}");
    assert!(s.pos.z < -650.0, "it ran 7 m back: {}", s.pos.z);
    assert!(s.cmds().iter().any(|c| matches!(c, ECmd::Motion { request: req::RUN, .. })));
}

/// The banana leaves 82 frames into 211 (only while 211 still plays); the peel 80 frames into 200.
#[test]
fn the_banana_and_the_peel_leave_on_their_frames() {
    let mut s = with(6, |cx| async move { banana(&cx).await });
    s.run(Vec3::new(0.0, 0.0, 1200.0), 160);
    let t0 = s.first(|c| matches!(c, ECmd::Motion { request: req::THROW, .. })).unwrap();
    let t1 = s.first(|c| *c == ECmd::Throw(child::BANANA)).expect("the banana");
    assert!((82..=84).contains(&(t1 - t0)), "{}", t1 - t0);
    let mut s = with(7, |cx| async move { toss(&cx).await });
    s.run(Vec3::new(0.0, 0.0, 600.0), 160);
    let t0 = s.first(|c| matches!(c, ECmd::Motion { request: req::TOSS, .. })).unwrap();
    let t1 = s.first(|c| *c == ECmd::Throw(child::PEEL)).expect("the peel");
    assert!((80..=82).contains(&(t1 - t0)), "{}", t1 - t0);
    // A reaction's motion replacing 211 before frame 82: no banana (KH1's child ends).
    let mut s = with(8, |cx| async move { banana(&cx).await });
    s.run(Vec3::new(0.0, 0.0, 1200.0), 40);
    s.current = Some(req::FLINCH);
    s.run(Vec3::new(0.0, 0.0, 1200.0), 120);
    assert!(!s.cmds().contains(&ECmd::Throw(child::BANANA)));
}

/// By distance: close the leap, mid the peel toss or the taunt, far the banana or the call (206 then 207).
#[test]
fn the_rounds_by_distance() {
    let mut seen = std::collections::HashSet::new();
    for seed in 0..30u32 {
        let mut s = ready(seed);
        s.ai.io.borrow_mut().x.engaged_once = true;
        let d = [300.0, 600.0, 1200.0][seed as usize % 3];
        s.run(Vec3::new(0.0, 0.0, d), 900);
        for c in s.cmds() {
            match c {
                ECmd::Throw(n) => {
                    seen.insert(format!("throw {n}"));
                }
                ECmd::Motion { request: r @ (req::LEAP | req::TAUNT | req::TOSS | req::THROW), .. } | ECmd::Action { request: r @ (req::CALL | req::CALL_END), .. } => {
                    seen.insert(format!("{r}"));
                }
                _ => {}
            }
        }
    }
    for k in ["3", "213", "200", "211", "206", "207", "throw 0", "throw 1", "throw 2"] {
        assert!(seen.contains(k), "{k}: {seen:?}");
    }
}

/// After five rounds the look-about 215, then KH1's escape (left out): the rounds start again.
#[test]
fn after_five_rounds_the_look_about_and_no_escape() {
    let mut s = ready(9);
    s.ai.io.borrow_mut().x.engaged_once = true;
    s.run(Vec3::new(0.0, 0.0, 1200.0), 6000);
    let c = s.cmds();
    assert!(c.iter().any(|x| matches!(x, ECmd::Motion { request: req::READY, .. })), "215");
    assert!(s.ai.escapes() > 0);
    assert!(!c.iter().any(|x| matches!(x, ECmd::Motion { request: req::ESCAPE, .. } | ECmd::Action { request: req::ESCAPE, .. })), "70 never plays");
    let ready_at = s.first(|x| matches!(x, ECmd::Motion { request: req::READY, .. })).unwrap();
    assert!(s.log.iter().any(|x| x.0 > ready_at + 240 && matches!(x.1, ECmd::Throw(_) | ECmd::Motion { request: req::LEAP | req::TAUNT | req::TOSS | req::THROW, .. } | ECmd::Action { request: req::CALL, .. })), "it fights on");
}

/// A kind 4 / 5 hit: its own launch (KH1's 0x52 / 0x5C, hop 350, speed 35), then the recovery 212 and the look-about.
/// Another hit: near, the turn 205 and the run-away; far, the look-about. A repel record does nothing; a clash the hit
/// state.
#[test]
fn the_hit_handler() {
    let mut s = ready(10);
    s.ai.set_hit_kind(5);
    Behaviour::on_hit(&s.ai, HitNews::Damage);
    let l = s.ai.scripted_launch().expect("its own launch");
    assert_eq!((l.ground, l.air, l.hop, l.speed, l.down), (0x52, 0x5C, 350.0, 35.0, 0.0));
    assert!(s.ai.scripted_launch().is_none(), "taken once");
    s.run(Vec3::new(0.0, 0.0, 300.0), 3);
    assert!(s.cmds().iter().any(|c| matches!(c, ECmd::Action { request: req::RECOVER, .. })), "{:?}", s.cmds());
    let mut s = ready(11);
    s.ai.set_hit_kind(3);
    Behaviour::on_hit(&s.ai, HitNews::Damage);
    assert!(s.ai.scripted_launch().is_none());
    s.run(Vec3::new(0.0, 0.0, 200.0), 200);
    assert!(s.cmds().iter().any(|c| matches!(c, ECmd::Motion { request: req::TURN, .. })));
    let mut s = ready(12);
    s.ai.set_hit_kind(3);
    Behaviour::on_hit(&s.ai, HitNews::Damage);
    s.run(Vec3::new(0.0, 0.0, 900.0), 5);
    assert!(s.cmds().iter().any(|c| matches!(c, ECmd::Motion { request: req::READY, .. })));
    let mut s = ready(13);
    Behaviour::on_hit(&s.ai, HitNews::Repelled);
    s.run(Vec3::new(0.0, 0.0, 5000.0), 3);
    assert!(!s.cmds().contains(&ECmd::State(st::HIT)));
    Behaviour::on_hit(&s.ai, HitNews::Clash);
    s.run(Vec3::new(0.0, 0.0, 5000.0), 3);
    assert!(s.cmds().contains(&ECmd::State(st::HIT)));
}

/// A long fight: the AI acts at least every 600 frames (the host's quiet watchdog); it throws, tosses, drops and leaps.
#[test]
fn a_long_fight_never_stalls() {
    let mut seen = [false; 4];
    for seed in 0..12u32 {
        let mut s = ready(seed);
        let (mut last, mut worst, mut next_hit) = (0usize, 0usize, 300usize);
        let mut r = Kh1Rand::new(seed + 100);
        for f in 0..20_000usize {
            let a = f as f32 / 600.0;
            let rad = 200.0 + 450.0 * (1.0 + (f as f32 / 1700.0).sin());
            let sora = Vec3::new(a.sin() * rad, 0.0, a.cos() * rad);
            if f == next_hit {
                let x = r.next_f32();
                s.ai.set_hit_kind(if x < 0.3 { 5 } else { 3 });
                Behaviour::on_hit(&s.ai, if x < 0.1 { HitNews::Clash } else { HitNews::Damage });
                let _ = s.ai.scripted_launch();
                s.busy = 34.0;
                next_hit = f + 150 + (r.next_f32() * 250.0) as usize;
            }
            let n = s.log.len();
            s.run(sora, 1);
            if s.log[n..].iter().any(|c| !matches!(c.1, ECmd::Yaw(_))) || s.busy > 0.0 || s.ai.body().walk != Vec3::ZERO || s.ai.body().airborne() {
                worst = worst.max(f - last);
                last = f;
            }
        }
        assert!(worst < 600, "seed {seed}: {worst} frames without acting");
        let c = s.cmds();
        for (i, p) in [c.contains(&ECmd::Throw(child::BANANA)), c.contains(&ECmd::Throw(child::PEEL)), c.contains(&ECmd::Throw(child::DROP)), c.iter().any(|x| matches!(x, ECmd::Motion { request: req::LEAP, .. }))].into_iter().enumerate() {
            seen[i] |= p;
        }
    }
    assert!(seen.iter().all(|&x| x), "{seen:?}");
}

/// Through the host's rules (`enemy::host::model`): nothing released over a 20 000-frame fight, 12 seeds.
#[test]
fn hosted_a_long_fight_never_releases() {
    for seed in 0..12u32 {
        let Sim { ai, pos, .. } = ready(seed);
        let mut m = Model::new(Box::new(ai), consts(), frames(), loops(), Vec::new());
        m.pos = pos;
        for f in 0..20_000usize {
            let a = f as f32 / 600.0;
            let rad = 200.0 + 450.0 * (1.0 + (f as f32 / 1700.0).sin());
            m.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
        }
        assert_eq!(m.released, 0, "seed {seed}");
        assert!(m.log.iter().any(|(_, c)| matches!(c, ECmd::Throw(_))), "seed {seed}: it throws");
    }
}

/// A peel: tossed up and forward, it falls by KH1's 0.8 a frame until its 30-unit sphere touches the floor, then lies
/// 120 frames and ends.
#[test]
fn a_peel_falls_lies_and_ends() {
    let n = PeelNumbers { sweep: 30.0, gravity: 0.8, lie: 120.0 };
    let v = Peel::tossed(0.0, 0.0, 9.0, 12.5);
    assert_eq!((v.x, v.y), (0.0, -12.5));
    let mut p = Peel::new(Vec3::new(0.0, -100.0, 0.0), v);
    let mut k = 0;
    while p.lying.is_none() {
        p.step(&n, 0.0, 1.0);
        k += 1;
        assert!(k < 200);
    }
    assert_eq!(p.pos.y, -30.0, "its sphere on the floor");
    // y(t) = −100 − 12.5 t + 0.4 t (t − 1) reaches −30 after about 36 frames, 9 a frame forward.
    assert!((34..=38).contains(&k), "{k}");
    assert!((p.pos.z - 9.0 * k as f32).abs() < 12.0, "{}", p.pos.z);
    let mut l = 0;
    while !p.ended {
        p.step(&n, 0.0, 1.0);
        l += 1;
    }
    assert_eq!(l, 120);
    let d = Peel::dropped(Vec3::ZERO, Vec3::new(300.0, 0.0, 400.0), 5.0, 7.0);
    assert!((d.x - 3.0).abs() < 1e-4 && (d.z - 4.0).abs() < 1e-4 && d.y == -7.0);
}
