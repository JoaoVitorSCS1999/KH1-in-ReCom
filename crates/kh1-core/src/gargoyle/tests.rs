use super::*;
use crate::enemy::host::model::Model;

/// The player's `ex_2220.bd` / `ex_2220_00.bd` / `btltbl.bin` / KH1 exe numbers as the asset step reads them (its
/// manifest on the session's files).
pub(super) fn consts() -> Consts {
    let v: &[(&str, f32)] = &[
        ("walk_speed", 2.0), ("run_speed", 8.0), ("close_range", 250.0), ("first_dive_chance", 0.5), ("first_approach_chance", 0.3), ("first_swoop_chance", 0.6),
        ("far_ball_chance", 0.7), ("far_swoop_chance", 0.3), ("close_dive_chance", 0.8), ("close_again_chance", 0.3), ("close_strafe2_chance", 0.5),
        ("hit_close_range", 400.0), ("hit_swipe_chance", 0.6), ("hit_dive_chance", 0.8), ("appear_fade", 30.0), ("appear_rise", -1.0), ("appear_rise_speed", 1.0),
        ("len_cut", 2.0), ("swipe2_cut", 8.0), ("motion_blend", 8.0), ("attack_blend", 8.0), ("settle_wait", 8.0), ("swoop_near", 300.0),
        ("swoop_hover_chance", 0.5), ("swoop_far", 700.0), ("swoop_turn_after", 40.0), ("swoop_extra", 6.0), ("swoop_vanish", 20.0), ("swoop_fade", 20.0),
        ("away_deg", 180.0), ("swoop_rise", -1.0), ("swoop_rise_speed", 2.0), ("swoop_reveal", 0.5), ("swoop_dive_chance", 0.1), ("back_fade", 30.0),
        ("strafe_turn", 4.0), ("strafe_side_chance", 0.5), ("strafe_left", -90.0), ("strafe_right", 90.0), ("strafe_low", 50.0), ("strafe_rise", -0.1),
        ("strafe_high", 80.0), ("strafe_sink", 0.1), ("strafe_speed", 16.0), ("hover_time", 300.0), ("hover_range", 400.0), ("hover_blend", 12.0),
        ("hover_wait", 12.0), ("hover_low", 180.0), ("hover_rise", -0.5), ("hover_high", 210.0), ("hover_sink", 0.5), ("hover_speed", 4.0), ("hover_far", 600.0),
        ("hover_stuck", 0.2), ("approach_time", 300.0), ("a1_speed", 4.0), ("a1_low", 50.0), ("a1_high", 80.0), ("a1_sink", 0.5), ("a1_rise", -0.5),
        ("a1_stop", 300.0), ("a2_speed", 10.0), ("a2_low", -20.0), ("a2_high", 20.0), ("a2_sink", 1.0), ("a2_rise", -1.0), ("a2_stop", 400.0), ("a3_speed", 10.0),
        ("a3_low", -20.0), ("a3_high", 20.0), ("a3_sink", 1.0), ("a3_rise", -1.0), ("a3_stop", 50.0), ("approach_stuck", 0.05), ("hold_low0", 50.0),
        ("hold_high0", 80.0), ("hold_low1", 180.0), ("hold_high1", 210.0), ("hold_rise", -0.5), ("hold_sink", 0.5), ("hold_speed", 1.0), ("air_accel", 0.1),
        ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0), ("gravity", crate::params::GRAVITY), ("fall_cap", 20.0), ("down_time", 30.0),
        ("air_knock_hop", 20.0), ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0),
        ("air_knock_speed", 16.0), ("weight", 1.0), ("ball_wait", 70.0), ("ball_charge", 160.0), ("ball_grow", 0.005), ("ball_lift", -15.0), ("ball_ease", 14.0),
        ("ball_ease_t", 1.0), ("ball_ease_k", 0.4), ("ball_toss", -0.8), ("ball_gravity", 0.5), ("ball_dist_max", 1200.0), ("ball_rise_max", 500.0),
        ("ball_speed_base", 10.0), ("ball_speed_add", 10.0), ("ball_tilt_base", -0.4), ("ball_tilt_add", -1.6), ("ball_bounces", 2.0), ("ball_turn", 1.0),
        ("ball_speed", 10.0), ("ball_life", 600.0), ("ball_fade", 20.0),
    ];
    Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), &all_names()).unwrap()
}

/// The player's `xa_ex_2220.mset` lengths by request (the asset step's clips).
pub(super) fn frames() -> Vec<(u16, f32)> {
    vec![
        (0, 220.0), (66, 40.0), (67, 240.0), (68, 80.0), (70, 60.0), (72, 50.0), (73, 50.0), (74, 50.0), (75, 50.0), (76, 80.0), (77, 80.0), (80, 80.0), (81, 80.0),
        (200, 80.0), (201, 80.0), (202, 70.0), (203, 100.0), (204, 124.0), (205, 50.0), (206, 84.0), (207, 58.0), (208, 132.0), (209, 116.0), (210, 200.0),
        (211, 60.0), (212, 84.0), (213, 80.0), (214, 60.0), (215, 52.0), (216, 52.0), (217, 70.0), (218, 114.0), (219, 328.0),
    ]
}

/// The asset step's looping clips, by request.
pub(super) fn loops() -> Vec<u16> {
    vec![0, 67, 102, 200, 201, 210]
}

/// A flat floor at y = 0, Sora on it; the host plays motions to their length and queues, keeps the visible / hittable
/// state the AI asked for; the AI is polled every frame (the Gargoyle plays no action of its own).
struct Sim {
    ai: Ai,
    pos: Vec3,
    current: Option<u16>,
    played: f32,
    queued: Option<u16>,
    moved: f32,
    log: Vec<(usize, Cmd)>,
    frame: usize,
    visible: bool,
    hittable: bool,
}

impl Sim {
    fn new(seed: u32, spawned: bool) -> Sim {
        Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: Vec3::new(0.0, -150.0, 0.0), current: None, played: 0.0, queued: None, moved: 5.0, log: Vec::new(), frame: 0, visible: true, hittable: true }
    }

    fn len(r: u16) -> f32 {
        frames().iter().find(|x| x.0 == r).map_or(30.0, |x| x.1)
    }

    fn run(&mut self, sora: impl Fn(usize) -> Vec3, n: usize) {
        for _ in 0..n {
            self.frame += 1;
            self.played += 1.0;
            if let Some(c) = self.current {
                if self.played >= Self::len(c) && !loops().contains(&c) {
                    if let Some(q) = self.queued.take() {
                        self.current = Some(q);
                        self.played = 0.0;
                    }
                }
            }
            self.ai.set_floor(0.0);
            let w = World { step: 1.0, pos: self.pos, target: Some(sora(self.frame)), grounded: false, airborne: false, moved: self.moved, motion: self.current };
            for c in self.ai.frame(w) {
                match c {
                    ECmd::Motion { request, .. } | ECmd::Attack { request, .. } if self.current != Some(request) => {
                        self.current = Some(request);
                        self.played = 0.0;
                        self.queued = None;
                    }
                    ECmd::Restart { request, .. } => {
                        self.current = Some(request);
                        self.played = 0.0;
                        self.queued = None;
                    }
                    ECmd::Queue { request, .. } => self.queued = Some(request),
                    ECmd::Visible(v) => self.visible = v,
                    ECmd::Hittable(h) => self.hittable = h,
                    _ => {}
                }
                self.log.push((self.frame, c));
            }
            let c = self.ai.consts();
            let steer = self.ai.take_steer();
            let d = self.ai.body().step(1.0, self.pos.y, steer, &c);
            self.pos += d;
            self.moved = d.length();
            if self.pos.y > 0.0 {
                self.pos.y = 0.0;
            }
        }
    }

    fn attacks(&self) -> Vec<(usize, Attack)> {
        self.log.iter().filter_map(|(f, c)| if let ECmd::Attack { attack, .. } = c { Some((*f, *attack)) } else { None }).collect()
    }

    fn motions(&self) -> Vec<u16> {
        self.log.iter().filter_map(|(_, c)| match c {
            ECmd::Motion { request, .. } | ECmd::Restart { request, .. } | ECmd::Attack { request, .. } => Some(*request),
            _ => None,
        }).collect()
    }
}

#[test]
fn the_names_cover_the_numbers() {
    assert!(Table::new(vec![], &all_names()).is_err());
    let c = consts();
    assert_eq!((c.num("close_range"), c.num("strafe_speed"), c.num("ball_life")), (250.0, 16.0, 600.0));
    assert_eq!(BallNumbers::from(&c).bounces, 2.0);
}

/// The battle spawn: 205 rising 1 a frame for its 50 frames, then the roar (213) and the first round.
#[test]
fn appears_rising_then_roars() {
    let mut s = Sim::new(7, true);
    s.run(|_| Vec3::new(0.0, 0.0, 600.0), 3);
    assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::Motion { request: req::APPEAR, .. })));
    assert!(s.ai.body().flying);
    let y0 = s.pos.y;
    s.run(|_| Vec3::new(0.0, 0.0, 600.0), 47);
    assert!((y0 - s.pos.y - 47.0).abs() < 2.0, "rises 1 a frame: {} → {}", y0, s.pos.y);
    s.run(|_| Vec3::new(0.0, 0.0, 600.0), 5);
    assert!(s.ai.risen());
    assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::State(st::ENGAGED))));
    assert!(s.log.iter().any(|(_, c)| matches!(c, ECmd::Motion { request: req::ROAR, .. })), "the roar");
}

/// Close (< 2.5 m): strafe (202 / 217), roar, dive or swipe (212 then 218), …, the back-off (208 unhittable and hidden,
/// 209 shown and hittable again).
#[test]
fn close_rounds_strafe_attack_and_back_off() {
    let mut s = Sim::new(3, false);
    let sora = |_| Vec3::new(0.0, 0.0, 150.0);
    s.run(sora, 1500);
    let m = s.motions();
    assert!(m.contains(&req::STRAFE_A) || m.contains(&req::STRAFE_B), "{m:?}");
    assert!(m.contains(&req::BACK) && m.contains(&req::BACK_IN), "{m:?}");
    let at = s.attacks();
    assert!(at.iter().any(|a| a.1 == Attack::Swipe) && at.iter().any(|a| a.1 == Attack::SwipeEnd), "{at:?}");
    // The swipe's end follows its start 82 frames later (len(212) − 2).
    let sw = at.iter().find(|a| a.1 == Attack::Swipe).unwrap().0;
    let end = at.iter().find(|a| a.1 == Attack::SwipeEnd && a.0 > sw).unwrap().0;
    assert_eq!(end - sw, 82);
    let off = s.log.iter().position(|(_, c)| *c == ECmd::Hittable(false)).expect("unhittable");
    let hid = s.log.iter().position(|(_, c)| *c == ECmd::Visible(false)).expect("hidden");
    let on = s.log.iter().skip(off).position(|(_, c)| *c == ECmd::Hittable(true)).unwrap() + off;
    assert!(off < hid && hid < on, "{off} {hid} {on}");
    assert!(s.visible && s.hittable);
}

/// Far (> 2.5 m): the ball (its child made with 219), then the swoop (vanish, the unseen approach, the reappearance and
/// an attack) or the approach; across seeds all of them.
#[test]
fn far_rounds_throw_swoop_and_approach() {
    let (mut threw, mut swooped, mut flew) = (false, false, false);
    for i in 1..16u32 {
        let mut s = Sim::new(i.wrapping_mul(0x9E37_79B9), false);
        s.run(|_| Vec3::new(0.0, 0.0, 900.0), 2000);
        let m = s.motions();
        if s.log.iter().any(|(_, c)| *c == ECmd::Throw(0)) {
            threw = true;
            let t = s.log.iter().position(|(_, c)| *c == ECmd::Throw(0)).unwrap();
            assert!(matches!(s.log[t + 1].1, ECmd::Motion { request: req::BALL, .. } | ECmd::Restart { request: req::BALL, .. }), "{:?}", &s.log[t..t + 2]);
        }
        if m.contains(&req::VANISH_FAR) || m.contains(&req::VANISH_NEAR) {
            swooped = true;
            assert!(s.log.iter().any(|(_, c)| *c == ECmd::Visible(false)));
        }
        if m.contains(&req::FLY) {
            flew = true;
        }
    }
    assert!(threw && swooped && flew);
}

/// The hit handler: any record asks for the hit state; the killing hit's flinch becomes a knock-back; a hidden
/// Gargoyle is shown at once.
#[test]
fn the_hit_handler() {
    let mut s = Sim::new(1, false);
    s.run(|_| Vec3::new(0.0, 0.0, 150.0), 30);
    s.ai.io.borrow_mut().state.1 = false;
    s.ai.on_hit(HitNews::Damage);
    assert!(s.ai.io.borrow().state.1);
    assert_eq!(s.ai.reaction_kind(3), 3);
    s.ai.io.borrow_mut().x.shown = false;
    s.ai.on_hit(HitNews::Clash);
    assert_eq!(s.ai.take_now(), vec![ECmd::Visible(true)]);
    assert!(s.ai.take_now().is_empty());
    s.ai.on_defeat();
    assert_eq!((s.ai.reaction_kind(3), s.ai.reaction_kind(5)), (4, 5));
    // The hit state within 4 m: swipe or dive (or neither), then the back-off.
    s.log.clear();
    s.run(|_| Vec3::new(0.0, 0.0, 150.0), 600);
    assert!(s.motions().contains(&req::BACK), "{:?}", s.motions());
}

/// The ball's child (`ex_2220_00.bd`): nothing for 70 frames, held at the chin point 15 up for 160, eased toward the
/// launch point for 14, launched on the next (frame 244: within 219's 328), tossed up along the facing at 10 a frame,
/// falling; each floor crossing re-aims it at the target the next frame (speed 10..20, tilt −0.4..−2.0); the third
/// ends it.
#[test]
fn the_ball_charges_launches_and_bounces_twice() {
    let n = BallNumbers::from(&consts());
    let mut b = Ball::new(n);
    let tgt = Vec3::new(0.0, -100.0, 800.0);
    let mut w = BallWorld { step: 1.0, parent_plays: true, hold: Vec3::new(0.0, -200.0, 50.0), at: Vec3::new(30.0, -180.0, 60.0), facing: Vec3::new(0.0, 0.0, 1.0), target: Some(tgt), floor: Some(0.0) };
    let mut events = Vec::new();
    for f in 0..2000 {
        let e = b.step(&w);
        if e != BallEvent::Nothing {
            events.push((f, e, b.pos));
        }
        if f == 100 {
            assert!((b.pos - Vec3::new(0.0, -215.0, 50.0)).length() < 1e-3, "held 15 up: {:?}", b.pos);
        }
        if b.ended() {
            break;
        }
        w.hold.z += 0.0;
    }
    let kinds: Vec<BallEvent> = events.iter().map(|e| e.1).collect();
    assert_eq!(kinds, vec![BallEvent::Appears, BallEvent::Launched, BallEvent::Bounced, BallEvent::Bounced, BallEvent::Ended], "{events:?}");
    assert_eq!((events[0].0, events[1].0), (70, 244));
    // Eased 14 times by 40 % toward the launch point: within 0.6^14 of the way.
    let launch_from = events[1].2;
    assert!(launch_from.z > 50.0);
    // The bounces move toward the target.
    assert!(events[3].2.z > events[2].2.z && events[2].2.z > launch_from.z);
    // A parent that stops playing 219 before the launch ends it.
    let mut b = Ball::new(n);
    for _ in 0..100 {
        b.step(&w);
    }
    w.parent_plays = false;
    assert_eq!(b.step(&w), BallEvent::Ended);
    // Once flying, the parent no longer matters; the life ends it.
    let mut b = Ball::new(n);
    w.parent_plays = true;
    w.floor = None;
    while !b.flying() {
        b.step(&w);
    }
    w.parent_plays = false;
    let mut k = 0;
    while !b.ended() {
        b.step(&w);
        k += 1;
    }
    assert_eq!(k, 600, "its 600-frame life (no floor: no contact)");
}

/// The first flight: tossed up at (0, −0.8, 1) normalised, 10 a frame, 0.5 more fall each frame (KH1 y down).
#[test]
fn the_launch_is_tossed_up_and_falls() {
    let n = BallNumbers::from(&consts());
    let mut b = Ball::new(n);
    let w = BallWorld { step: 1.0, parent_plays: true, hold: Vec3::ZERO, at: Vec3::new(0.0, -200.0, 0.0), facing: Vec3::new(0.0, 0.0, 1.0), target: None, floor: Some(0.0) };
    while !b.flying() {
        b.step(&w);
    }
    let l = (1.0f32 + 0.64).sqrt();
    let p0 = b.pos;
    b.step(&w);
    let d = b.pos - p0;
    assert!((d.z - 10.0 / l).abs() < 1e-3 && (d.y - (-8.0 / l + 1.0)).abs() < 1e-3, "{d:?}");
}

/// The swing's send-back (`0x748`): toward the parent and the swinger's target together, tossed up again, its life and
/// count anew, its speed kept; its second flight ends as the first.
#[test]
fn a_bat_sends_it_back_once() {
    let n = BallNumbers::from(&consts());
    let mut b = Ball::new(n);
    let w = BallWorld { step: 1.0, parent_plays: true, hold: Vec3::ZERO, at: Vec3::new(0.0, -200.0, 0.0), facing: Vec3::new(0.0, 0.0, 1.0), target: None, floor: Some(0.0) };
    while !b.flying() {
        b.step(&w);
    }
    for _ in 0..5 {
        b.step(&w);
    }
    b.bat(Some(Vec3::new(0.0, -200.0, -500.0)), Some(Vec3::new(500.0, -200.0, b.pos.z)), Vec3::new(0.0, 0.0, -1.0));
    assert!(b.batted && b.time == 0.0 && b.flying());
    assert!(b.dir.z < 0.0 && b.dir.x > 0.0 && b.dir.y < 0.0, "{:?}", b.dir);
    // No target of Sora's: straight back at the parent (its direction's flat part).
    let mut c = Ball::new(n);
    c.pos = Vec3::new(0.0, -100.0, 300.0);
    c.bat(Some(Vec3::new(0.0, -100.0, 0.0)), None, Vec3::ZERO);
    assert!(c.dir.z < 0.0 && c.dir.x.abs() < 1e-6);
}

/// The Gargoyle through the host's rules (`crate::enemy::host::model`) with its class map (its three attacks in
/// Re:CoM's state 2) and the asset step's loops: the swipe → swipe end chain and every attack unbroken, nothing
/// released, never asked again within two frames.
#[test]
fn hosted_never_releases() {
    for seed in 1..8u32 {
        let ai = Ai::new(consts(), frames(), seed.wrapping_mul(0x9E37_79B9), 0.0, true);
        let mut m = Model::new(Box::new(ai), consts(), frames(), loops(), vec![2, 2, 2]);
        m.pos = Vec3::new(0.0, -150.0, 0.0);
        for f in 0..20_000usize {
            let a = f as f32 / 600.0;
            let rad = 150.0 + 300.0 * (1.0 + (f as f32 / 1700.0).sin());
            m.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
        }
        assert_eq!(m.released, 0, "seed {seed}");
        let issued: Vec<(usize, u16)> = m.log.iter().filter_map(|(f, c)| if let ECmd::Attack { request, .. } = c { Some((*f, *request)) } else { None }).collect();
        assert!(issued.len() > 20, "seed {seed}: {}", issued.len());
        for w in issued.windows(2) {
            assert!(w[1].1 != w[0].1 || w[1].0 > w[0].0 + 2, "seed {seed}: {:?}", w);
        }
        assert!(m.trace.iter().all(|t| t.0 == 1 || t.0 == 2), "seed {seed}: only states 1 and 2");
    }
}

/// No stall: Sora circling 1.5–7.5 m away for 20 000 frames (12 seeds), hit now and then: it acts (an attack, a ball, a
/// swoop's vanish) at least every 1500 frames, never stays hidden or unhittable long, never goes under the floor, and
/// uses all three attacks and the ball.
#[test]
fn a_long_fight_never_stalls() {
    for i in 1u32..=12 {
        let seed = i.wrapping_mul(0x9E37_79B9) ^ 0x1234_5678;
        let mut s = Sim::new(seed, true);
        let sora = |f: usize| {
            let a = f as f32 * 0.002;
            let r = 450.0 + 300.0 * (f as f32 * 0.0007).sin();
            Vec3::new(r * a.cos(), 0.0, r * a.sin())
        };
        let mut low = f32::INFINITY;
        let (mut hidden_for, mut worst_hidden) = (0, 0);
        let mut next_hit = 300;
        for f in 0..20_000usize {
            s.run(sora, 1);
            low = low.min(-s.pos.y);
            if !s.visible || !s.hittable {
                hidden_for += 1;
                worst_hidden = worst_hidden.max(hidden_for);
            } else {
                hidden_for = 0;
            }
            if f == next_hit {
                next_hit += 150 + (f * 7919 + seed as usize * 104_729) % 250;
                if s.hittable {
                    s.ai.set_hit_kind(3);
                    s.ai.on_hit(HitNews::Damage);
                    for c in s.ai.take_now() {
                        if c == ECmd::Visible(true) {
                            s.visible = true;
                        }
                    }
                }
            }
        }
        assert!(low >= 0.0, "seed {seed}: never under the floor");
        let at = s.attacks();
        let mut acts: Vec<usize> = at.iter().map(|a| a.0).chain(s.log.iter().filter(|(_, c)| matches!(c, ECmd::Throw(_) | ECmd::Visible(false))).map(|(f, _)| *f)).collect();
        acts.sort();
        let mut last = 0;
        for &f in &acts {
            assert!(f - last < 1500, "seed {seed}: nothing from {last} to {f}");
            last = f;
        }
        assert!(20_000 - last < 1500, "seed {seed}: nothing after {last}");
        assert!(worst_hidden < 600, "seed {seed}: hidden or unhittable {worst_hidden} frames in a row");
        let kinds: std::collections::HashSet<_> = at.iter().map(|a| a.1).collect();
        assert_eq!(kinds.len(), 3, "seed {seed}: {kinds:?}");
        assert!(s.log.iter().any(|(_, c)| *c == ECmd::Throw(0)), "seed {seed}: no ball");
        assert!(s.ai.body().flying, "seed {seed}");
    }
}
