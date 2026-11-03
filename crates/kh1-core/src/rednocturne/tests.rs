use super::*;
use crate::enemy::host::model::Model;

/// The player's `ex_2110.bd` / `ex_2110_00.bd` / `ex_2110_01.bd` / `btltbl.bin` / KH1 exe numbers as the asset step reads
/// them (its manifest on the session's files, 2026-11-03).
pub(super) fn consts() -> Consts {
    let v: &[(&str, f32)] = &[
        ("walk_speed", 2.0), ("run_speed", 8.0), ("close_range", 500.0), ("mid_range", 700.0), ("near_fire_chance", 0.4), ("far_bigfire_chance", 0.5),
        ("hit_close_range", 500.0), ("idle_search", 1000.0), ("cast_blend", 8.0), ("cast_start", 0.0), ("bigfire_start", 8.0), ("cast_queue_blend", 0.0),
        ("idle_blend", 8.0), ("idle_start", 0.0), ("flip", -1.0), ("strafe_turn", 5.0), ("strafe_side_chance", 0.5), ("strafe_left", -90.0),
        ("strafe_right", 90.0), ("strafe_speed", 8.4), ("flee_min", 500.0), ("flee_max", 600.0), ("flee_speed", 14.0), ("flee_stop", 100.0), ("flee_up", 0.0),
        ("flee_high", 300.0), ("flee_time_min", 120.0), ("flee_time_max", 180.0), ("approach_speed", 8.4), ("approach_stop", 500.0), ("approach_up", 100.0),
        ("approach_high", 300.0), ("approach_time", 240.0), ("hover_turn", 2.0), ("hover_side_chance", 0.8), ("hover_left", -90.0), ("hover_right", 90.0),
        ("hover_mid", 90.0), ("hover_time_min", 90.0), ("hover_time_max", 120.0), ("hover_blend", 8.0), ("hover_speed", 3.64), ("hover_sink", 0.2),
        ("floor_probe", 20.0), ("hover_low_min", 15.0), ("hover_low_max", 30.0), ("hover_stuck", 0.84), ("wander_turn", 5.0), ("wander_sink", 0.3),
        ("wander_low", 150.0), ("wander_min", 150.0), ("wander_max", 180.0), ("wander_blend", 8.0), ("wander_speed", 8.0), ("fly_blend", 8.0),
        ("fly_no_motion", 999.0), ("fly_stuck", 0.3), ("stuck_half", 0.5), ("air_accel_set", 0.2), ("notice_world", 11.0), ("notice_blend", 8.0),
        ("wait_motion", 6.0), ("appear_fade", 30.0), ("appear_blend", 0.0), ("own_row_first", 10500.0), ("own_row_last", 10504.0), ("exp_div", 10.0),
        ("exp_add", 2.0), ("dizzy_time", 1800.0), ("fire_child", 0.0), ("bigfire_child", 1.0), ("air_accel", 0.1), ("air_drag", 0.95),
        ("turn_rate", std::f32::consts::PI / 16.0), ("gravity", crate::params::GRAVITY), ("fall_cap", 20.0), ("down_time", 30.0), ("air_knock_hop", 20.0),
        ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0),
        ("weight", 1.0), ("attack_blend", 8.0), ("fire_wait", 40.0), ("fire_t", -0.5), ("fire_up", -80.0), ("fire_turn", 1.0), ("fire_speed", 12.0),
        ("fire_life", 90.0), ("fire_fade", 20.0), ("bigfire_wait", 90.0), ("bigfire_t", -0.5), ("bigfire_up", -70.0), ("bigfire_turn", 2.0),
        ("bigfire_speed", 22.0), ("bigfire_life", 90.0), ("bigfire_fade", 20.0),
    ];
    Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), &all_names()).unwrap()
}

/// The player's `xa_ex_2110.mset` lengths by request (the asset step's clips).
pub(super) fn frames() -> Vec<(u16, f32)> {
    vec![
        (0, 200.0), (1, 200.0), (2, 246.0), (66, 60.0), (67, 340.0), (68, 60.0), (69, 90.0), (70, 66.0), (72, 50.0), (73, 50.0), (74, 50.0), (75, 50.0),
        (76, 104.0), (77, 104.0), (78, 50.0), (79, 50.0), (80, 104.0), (81, 104.0), (200, 60.0), (201, 116.0), (202, 120.0), (203, 200.0), (214, 158.0),
        (217, 120.0), (219, 200.0), (220, 200.0),
    ]
}

/// The asset step's looping clips, by request.
pub(super) fn loops() -> Vec<u16> {
    vec![0, 1, 2, 67, 201, 203, 217, 219, 220]
}

fn model(seed: u32, spawned: bool) -> Model<Consts> {
    let ai = Box::new(Ai::new(consts(), frames(), seed, 0.0, spawned));
    let mut m = Model::new(ai, consts(), frames(), loops(), Vec::new());
    m.pos = Vec3::new(0.0, -150.0, 0.0);
    m
}

fn throws(m: &Model<Consts>, n: u8) -> Vec<usize> {
    m.log.iter().filter(|(_, c)| *c == ECmd::Throw(n)).map(|(f, _)| *f).collect()
}

fn asked(m: &Model<Consts>, r: u16) -> Vec<usize> {
    m.log.iter().filter(|(_, c)| matches!(c, ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Restart { request, .. } if *request == r)).map(|(f, _)| *f).collect()
}

/// A spawned one plays KH1's appearance (69, an action, the idle hover queued) and fades in; nothing else meanwhile.
#[test]
fn it_appears_then_idles_or_engages() {
    let mut m = model(1, true);
    m.run(Vec3::new(3000.0, 0.0, 0.0), 120);
    let first = m.log.iter().find(|(_, c)| matches!(c, ECmd::Action { .. })).unwrap();
    assert!(matches!(first.1, ECmd::Action { request: req::APPEAR, then: req::WANDER, attack: None, .. }));
    assert!(m.log.iter().any(|(_, c)| *c == ECmd::FadeIn(30.0)));
    // Sora far (30 m): idle legs (219), facing yaw 0 each frame (KH1's `sys 11` of a zero vector).
    m.run(Vec3::new(3000.0, 0.0, 0.0), 400);
    assert!(!asked(&m, req::WANDER).is_empty() || m.current == Some(req::WANDER));
    let late: Vec<_> = m.log.iter().filter(|(f, c)| *f > 200 && matches!(c, ECmd::Yaw(_))).collect();
    assert!(!late.is_empty() && late.iter().all(|(_, c)| *c == ECmd::Yaw(0.0)), "{late:?}");
    assert!(throws(&m, 0).is_empty() && throws(&m, 1).is_empty());
    // Sora comes within 10 m: engaged.
    m.run(Vec3::new(400.0, 0.0, 0.0), 600);
    assert!(m.log.iter().any(|(_, c)| *c == ECmd::State(st::ENGAGED)));
}

/// Close: the strafe's Fire leaves on the cast's 41st frame (40 frames counted in 201), 60 + 40 frames after 200 is asked.
#[test]
fn the_fire_leaves_forty_frames_into_its_cast() {
    let mut found = 0;
    for seed in 0..12 {
        let mut m = model(seed, false);
        m.run(Vec3::new(250.0, 0.0, 0.0), 3000);
        for f in throws(&m, child::FIRE) {
            let ask = asked(&m, req::CHARGE).into_iter().filter(|&a| a < f).max().unwrap();
            assert_eq!(f - ask, 60 + 40, "seed {seed}: 200 asked at {ask}, the Fire at {f}");
            found += 1;
        }
    }
    assert!(found > 0);
}

/// Far: the big Fire leaves 90 frames into 203 (the model plays 202 from its start: its 120 frames, then 90).
#[test]
fn the_big_fire_leaves_ninety_frames_into_its_cast() {
    let mut found = 0;
    for seed in 0..12 {
        let mut m = model(seed, false);
        // Sora keeps 9 m away (beyond 700: the far rounds; within the 10 m search).
        for _ in 0..3000 {
            let s = Vec3::new(m.pos.x + 900.0, 0.0, m.pos.z);
            m.run(s, 1);
        }
        for f in throws(&m, child::BIG_FIRE) {
            let ask = asked(&m, req::BIG_CHARGE).into_iter().filter(|&a| a < f).max().unwrap();
            assert_eq!(f - ask, 120 + 90, "seed {seed}");
            found += 1;
        }
        assert!(m.log.iter().any(|(_, c)| matches!(c, ECmd::MotionAt { request: req::BIG_CHARGE, start, .. } if *start == 8.0)), "202 from frame 8");
    }
    assert!(found > 0);
}

/// A reaction (another motion) in the wait: the child ends unseen, no fire.
#[test]
fn a_reaction_in_the_wait_ends_the_cast() {
    let mut ai = Ai::new(consts(), frames(), 3, 0.0, false);
    let w = |motion: Option<u16>| World { step: 1.0, pos: Vec3::new(0.0, -150.0, 0.0), target: Some(Vec3::new(200.0, 0.0, 0.0)), grounded: false, airborne: false, moved: 1.0, motion };
    ai.io.borrow_mut().x.casts.push(Cast::new(0, req::CHARGE, req::CAST, 40.0));
    assert!(Behaviour::side_frame(&mut ai, w(Some(req::CAST))).is_empty());
    assert!(Behaviour::side_frame(&mut ai, w(Some(0x48))).is_empty());
    assert!(ai.io.borrow().x.casts.is_empty());
}

/// The hit handler: a living hit asks for the hit state; one from its own kind's fire sent back also gives KH1's EXP bonus
/// (EXP / 10 + 2) at once; a repel record asks nothing.
#[test]
fn the_hit_handler() {
    let ai = Ai::new(consts(), frames(), 3, 0.0, false);
    ai.on_hit(HitNews::Repelled);
    assert_eq!(ai.io.borrow().state, (st::NONE, false));
    ai.set_hit_own_row(false);
    ai.on_hit(HitNews::Damage);
    assert_eq!(ai.io.borrow().state, (st::HIT, true));
    assert!(ai.take_now().is_empty());
    ai.set_hit_own_row(true);
    ai.on_hit(HitNews::Damage);
    let now = ai.take_now();
    assert!(now.iter().any(|c| matches!(c, ECmd::Reward(r) if r.exp_bonus(100) == 12)), "{now:?}");
    assert_eq!(ai.reaction_kind(3), 3, "the handler zeroes no kind (Fire's aside)");
    assert!(ai.own_row_stun(), "its own fire's hit takes Re:CoM's stun (owner 2026-11-03)");
}

/// The start of a fire: 50 ahead along its facing (`sys 19(parent, 0x8001, −0.5)`: the matrix's z row is −facing), then
/// up.
#[test]
fn a_fire_starts_ahead_and_up() {
    let p = fire_start(Vec3::new(10.0, -100.0, 20.0), 0.0, -0.5, -80.0);
    assert!((p - Vec3::new(10.0, -180.0, 70.0)).length() < 1e-4, "{p:?}");
    let p = fire_start(Vec3::ZERO, std::f32::consts::FRAC_PI_2, -0.5, -70.0);
    assert!((p - Vec3::new(50.0, -70.0, 0.0)).length() < 1e-3, "{p:?}");
}

/// 20 000-frame fights (12 seeds, Sora circling 1.5..15.5 m from the start): the AI never goes quiet for 600 frames, both fires are
/// cast, and it never asks for an attack (it has none).
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
        }
        assert!(gap < 600, "seed {seed}: quiet for {gap} frames");
        assert_eq!(m.released, 0);
        assert!(!throws(&m, child::FIRE).is_empty(), "seed {seed}: no Fire");
        assert!(!throws(&m, child::BIG_FIRE).is_empty(), "seed {seed}: no big Fire");
        assert!(m.log.iter().all(|(_, c)| !matches!(c, ECmd::Attack { .. })));
        assert!(m.pos.y < 0.0 + 1e-3 || m.pos.y.is_finite());
    }
}

