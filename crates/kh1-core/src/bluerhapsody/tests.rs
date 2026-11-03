use super::*;
use crate::enemy::host::model::Model;

/// The player's `ex_2120.bd` / `ex_2120_00.bd` / `btltbl.bin` / KH1 exe numbers as the asset step reads them (its
/// manifest on the session's files, 2026-11-03).
pub(super) fn consts() -> Consts {
    let v: &[(&str, f32)] = &[
        ("walk_speed", 2.0), ("run_speed", 8.0), ("close_range", 250.0), ("mid_range", 500.0), ("hit_close_range", 500.0), ("hop_side_chance", 0.5),
        ("double_hop_chance", 0.6), ("hop_turn", 20.0), ("flip", -1.0), ("hop_speed", 30.0), ("hop_height", 60.0), ("hop_drag", 0.9), ("hop_time", 12.0),
        ("back_turn", 180.0), ("back_speed", 35.0), ("back_height", 250.0), ("back_drag", 0.9), ("back_time", 40.0), ("approach_count", 120.0),
        ("approach_speed", 20.0), ("approach_height", 120.0), ("approach_drag", 0.9), ("approach_time", 25.0), ("climb_speed", 5.0), ("climb_height", 500.0),
        ("climb_drag", 1.0), ("climb_time", 60.0), ("approach_stop", 750.0), ("rise_up", 35.0), ("floor_probe", 20.0), ("rise_speed", 6.0), ("rise_stop", 20.0),
        ("rise_aim", 0.0), ("rise_high", 500.0), ("rise_time", 300.0), ("wander_turn", 5.0), ("wander_sink", 0.3), ("wander_low", 150.0), ("wander_min", 150.0),
        ("wander_max", 180.0), ("wander_blend", 8.0), ("wander_speed", 8.0), ("idle_search", 1000.0), ("fly_no_motion", 999.0), ("fly_blend", 8.0),
        ("fly_stuck", 0.3), ("stuck_half", 0.5), ("air_accel_set", 0.2), ("notice_world", 11.0), ("notice_blend", 8.0), ("wait_motion", 6.0),
        ("appear_fade", 30.0), ("appear_blend", 0.0), ("appear_queue_blend", 8.0), ("cast_blend", 0.0), ("cast_start", 0.0), ("melee_blend", 8.0),
        ("melee_start", 8.0), ("attack_blend", 8.0), ("rest_blend", 0.0), ("rest_start", 2.0), ("exp_div", 10.0), ("exp_add", 2.0), ("dizzy_time", 1800.0),
        ("blizzard_child", 0.0), ("air_accel", 0.1), ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0), ("gravity", crate::params::GRAVITY),
        ("fall_cap", 20.0), ("down_time", 30.0), ("air_knock_hop", 20.0), ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.35),
        ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("weight", 1.0), ("blizzard_wait", 84.0), ("blizzard_t", 0.0),
        ("blizzard_life", 60.0), ("blizzard_speed", 25.0),
    ];
    Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), &all_names()).unwrap()
}

/// The player's `xa_ex_2120.mset` lengths by request (the asset step's clips).
pub(super) fn frames() -> Vec<(u16, f32)> {
    vec![
        (0, 200.0), (1, 200.0), (2, 246.0), (3, 16.0), (4, 8.0), (5, 26.0), (6, 32.0), (7, 8.0), (55, 148.0), (66, 60.0), (67, 340.0), (68, 60.0),
        (69, 90.0), (70, 66.0), (72, 50.0), (73, 50.0), (74, 50.0), (75, 50.0), (76, 104.0), (77, 104.0), (78, 50.0), (79, 50.0), (80, 104.0), (81, 104.0),
        (103, 50.0), (104, 32.0), (204, 148.0), (214, 158.0), (215, 148.0), (216, 52.0), (219, 200.0), (220, 200.0),
    ]
}

/// The asset step's looping clips, by request.
pub(super) fn loops() -> Vec<u16> {
    vec![0, 1, 2, 6, 55, 67, 104, 204, 215, 219, 220]
}

/// Re:CoM's states for its attack (the class row's map: the melee in Re:CoM's melee state 3).
const ATTACK_STATES: [i32; 1] = [3];

fn model(seed: u32, spawned: bool) -> Model<Consts> {
    let ai = Box::new(Ai::new(consts(), frames(), seed, 0.0, spawned));
    let mut m = Model::new(ai, consts(), frames(), loops(), ATTACK_STATES.to_vec());
    m.pos = Vec3::new(0.0, -150.0, 0.0);
    m
}

fn throws(m: &Model<Consts>) -> Vec<usize> {
    m.log.iter().filter(|(_, c)| *c == ECmd::Throw(child::BLIZZARD)).map(|(f, _)| *f).collect()
}

fn asked(m: &Model<Consts>, r: u16) -> Vec<usize> {
    m.log.iter().filter(|(_, c)| matches!(c, ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Restart { request, .. } if *request == r)).map(|(f, _)| *f).collect()
}

/// Sora keeps `d` away (horizontally) from the Blue Rhapsody, on the floor, for `n` frames.
fn keep(m: &mut Model<Consts>, d: f32, n: usize) {
    for _ in 0..n {
        let s = Vec3::new(m.pos.x + d, 0.0, m.pos.z);
        m.run(s, 1);
    }
}

/// A spawned one plays KH1's appearance (69, an action, the idle hover queued) and fades in; far from Sora it idles
/// (219), facing yaw 0 (KH1's `sys 11` of a zero vector, the family's idle leg); within 10 m it engages.
#[test]
fn it_appears_then_idles_or_engages() {
    let mut m = model(1, true);
    m.run(Vec3::new(3000.0, 0.0, 0.0), 120);
    let first = m.log.iter().find(|(_, c)| matches!(c, ECmd::Action { .. })).unwrap();
    assert!(matches!(first.1, ECmd::Action { request: req::APPEAR, then: req::WANDER, attack: None, .. }));
    assert!(m.log.iter().any(|(_, c)| *c == ECmd::FadeIn(30.0)));
    m.run(Vec3::new(3000.0, 0.0, 0.0), 400);
    assert!(!asked(&m, req::WANDER).is_empty() || m.current == Some(req::WANDER));
    let late: Vec<_> = m.log.iter().filter(|(f, c)| *f > 200 && matches!(c, ECmd::Yaw(_))).collect();
    assert!(!late.is_empty() && late.iter().all(|(_, c)| *c == ECmd::Yaw(0.0)), "{late:?}");
    assert!(throws(&m).is_empty());
    m.run(Vec3::new(400.0, 0.0, 0.0), 600);
    assert!(m.log.iter().any(|(_, c)| *c == ECmd::State(st::ENGAGED)));
}

/// Engaged, it first flies straight to 35 above the floor under Sora (`0x78E`: 6 a frame, 220 playing) and holds there.
#[test]
fn it_flies_to_thirty_five_above_the_floor() {
    let mut m = model(2, false);
    m.pos = Vec3::new(0.0, -150.0, 0.0);
    // Sora within 10 m (but beyond 7.5 m): a placed one's notice (214), then `0x78E`, then the approach's first hop.
    let mut n = 0;
    while asked(&m, req::CROUCH).is_empty() && n < 1000 {
        keep(&mut m, 900.0, 1);
        n += 1;
    }
    assert!(!asked(&m, req::NOTICE).is_empty(), "a placed one notices first");
    assert!((m.pos.y + 35.0).abs() <= 20.0 + 1e-3, "within the 20 stop of 35 above: {} (frame {n})", m.pos.y);
    assert!(!asked(&m, req::REST).is_empty());
}

/// Close (under 2.5 m): the back-off (a hop 35 back, 250 high over 40 frames: crouch 3, rise 4, apex 5 / drop 6), back in
/// flight, then the Blizzard: 55 at blend 0, and its child acts on 55's 85th frame after the ask (84 counted from the
/// child's first frame).
#[test]
fn close_it_backs_off_and_casts_the_blizzard() {
    let mut found = 0;
    for seed in 0..12 {
        let mut m = model(seed, false);
        m.pos = Vec3::new(0.0, -35.0, 0.0);
        keep(&mut m, 150.0, 1500);
        for f in throws(&m) {
            let ask = asked(&m, req::CAST).into_iter().filter(|&a| a < f).max().unwrap();
            assert_eq!(f - ask, 85, "seed {seed}: 55 asked at {ask}, the Blizzard at {f}");
            // A back-off hop came first: its crouch, its rise.
            let crouch = asked(&m, req::CROUCH).into_iter().filter(|&a| a < ask).max();
            assert!(crouch.is_some(), "seed {seed}: no hop before the cast");
            found += 1;
        }
        assert!(m.log.iter().any(|(_, c)| matches!(c, ECmd::Motion { request: req::APEX, .. })), "seed {seed}: no apex");
        assert!(m.ai.body().flight_hop.is_none() || m.ai.body().timed.is_some());
    }
    assert!(found > 0);
}

/// Mid range (2.5..5 m): one or two hops (± 20° off its facing, 30 a frame, 60 high, 12 frames), then the melee: 215 as a
/// steered attack from frame 8 at blend 8, in Re:CoM's state 3; then 220 from frame 2.
#[test]
fn mid_range_hops_then_melee() {
    let mut found = 0;
    for seed in 0..12 {
        let mut m = model(seed, false);
        m.pos = Vec3::new(0.0, -35.0, 0.0);
        keep(&mut m, 400.0, 2000);
        let at: Vec<usize> = m.log.iter().filter(|(_, c)| matches!(c, ECmd::AttackAt { request: req::MELEE, attack: 0, blend, start } if *blend == 8.0 && *start == 8.0)).map(|(f, _)| *f).collect();
        for &a in &at {
            let hops = asked(&m, req::CROUCH).into_iter().filter(|&c| c < a && a - c < 200).count();
            assert!((1..=2).contains(&hops), "seed {seed}: {hops} hops before the melee at {a}");
            found += 1;
        }
        assert!(m.trace.iter().all(|t| t.1 != Some(req::MELEE) || t.0 == 3), "seed {seed}: 215 outside state 3");
        assert!(m.log.iter().any(|(_, c)| matches!(c, ECmd::MotionAt { request: req::REST, start, .. } if *start == 2.0)));
    }
    assert!(found > 0);
}

/// Far (beyond 5 m, within 10 m to engage): the approach hops toward Sora (20 ahead, 120 high, 25 frames) until within
/// 7.5 m.
#[test]
fn far_it_hops_closer() {
    let mut m = model(5, false);
    m.pos = Vec3::new(0.0, -35.0, 0.0);
    let sora = Vec3::new(980.0, 0.0, 0.0);
    m.run(sora, 1500);
    assert!(asked(&m, req::CROUCH).len() >= 2, "hops");
    let d = (sora - m.pos).length();
    assert!(d < 980.0, "closer: {d}");
}

/// The hit handler: a living hit or a clash asks for the hit state; a repel record asks nothing; no reward (its element
/// rules are not carried).
#[test]
fn the_hit_handler() {
    let ai = Ai::new(consts(), frames(), 3, 0.0, false);
    ai.on_hit(HitNews::Repelled);
    assert_eq!(ai.io.borrow().state, (st::NONE, false));
    ai.on_hit(HitNews::Damage);
    assert_eq!(ai.io.borrow().state, (st::HIT, true));
    assert!(ai.take_now().is_empty());
    ai.io.borrow_mut().state = (st::ENGAGED, false);
    ai.on_hit(HitNews::Clash);
    assert_eq!(ai.io.borrow().state, (st::HIT, true));
    assert_eq!(ai.reaction_kind(3), 3);
}

/// A reaction in the cast: the child ends unseen, no Blizzard.
#[test]
fn a_reaction_in_the_cast_ends_the_blizzard() {
    let mut ai = Ai::new(consts(), frames(), 3, 0.0, false);
    let w = |motion: Option<u16>| World { step: 1.0, pos: Vec3::new(0.0, -35.0, 0.0), target: Some(Vec3::new(200.0, 0.0, 0.0)), grounded: false, airborne: false, moved: 1.0, motion };
    ai.io.borrow_mut().x.casts.push(Cast::direct(0, req::CAST, 84.0));
    assert!(Behaviour::side_frame(&mut ai, w(Some(req::CAST))).is_empty());
    assert!(Behaviour::side_frame(&mut ai, w(Some(0x48))).is_empty());
    assert!(ai.io.borrow().x.casts.is_empty());
}

/// 20 000-frame fights (12 seeds, Sora circling 1.5..15.5 m from the start): the AI never goes quiet for 600 frames,
/// nothing is released, it casts Blizzards and plays its melee only in state 3, and it stays in the air.
#[test]
fn long_fights_never_stall() {
    for seed in 0..12u32 {
        let mut m = model(seed, true);
        let mut last = 0usize;
        let mut gap = 0usize;
        for f in 0..20_000usize {
            let a = f as f32 / 600.0;
            let rad = 150.0 + 700.0 * (1.0 + (f as f32 / 1700.0).sin());
            let before = m.log.len();
            m.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
            let acted = m.log.len() > before || m.ai.body().walk != Vec3::ZERO || m.moved > 0.0;
            if acted {
                gap = gap.max(f - last);
                last = f;
            }
            assert!(m.pos.y <= 1e-3 && m.pos.y > -2000.0, "seed {seed} frame {f}: height {}", m.pos.y);
        }
        assert!(gap < 600, "seed {seed}: quiet for {gap} frames");
        assert_eq!(m.released, 0, "seed {seed}");
        assert!(!throws(&m).is_empty(), "seed {seed}: no Blizzard");
        assert!(!m.attacks_of(req::MELEE).is_empty(), "seed {seed}: no melee");
        assert!(m.trace.iter().all(|t| t.1 != Some(req::MELEE) || t.0 == 3), "seed {seed}: 215 outside state 3");
        assert!(m.ai.body().flying, "seed {seed}: it flies");
    }
}
