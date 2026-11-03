use super::*;
use crate::enemy::host::model::Model;

/// The player's `ex_2130.bd` / `ex_2130_00.bd` / `btltbl.bin` / KH1 exe numbers as the asset step reads them (its
/// manifest on the session's files, 2026-11-03).
pub(super) fn consts() -> Consts {
    let v: &[(&str, f32)] = &[
        ("walk_speed", 2.0), ("run_speed", 8.0), ("close_range", 250.0), ("mid_range", 500.0), ("thunder_chance", 0.6), ("charge_chance", 0.5),
        ("cast_blend", 8.0), ("cast_start", 8.0), ("hold_blend", 0.0), ("hold_start", 2.0), ("strike_blend", 0.0), ("strike_start", 3.0),
        ("rest_blend", 8.0), ("rest_start", 8.0), ("attack_blend", 8.0), ("thunder_child", 0.0), ("charge_blend", 8.0), ("charge_start", 8.0),
        ("rush_blend", 0.0), ("rush_time", 150.0), ("rush_stop", 30.0), ("rush_high", 500.0), ("rush_speed", 25.0), ("rush_stuck", 12.5),
        ("recover_blend", 0.0), ("recover_start", 2.0), ("flee_legs", 3.0), ("flee_min", 200.0), ("flee_vert", 250.0), ("flee_last_up", 50.0),
        ("flee_speed", 40.0), ("flee_stop", 10.0), ("flee_up", 0.0), ("flee_high", 500.0), ("flee_time", 20.0), ("drop_probe", 20.0),
        ("drop_floor_min", -400.0), ("drop_up", 100.0), ("drop_stop", 30.0), ("move_min", 200.0), ("move_vert", 150.0), ("move_speed", 15.0),
        ("move_stop", 10.0), ("move_up", 0.0), ("move_high", 500.0), ("move_time", 30.0), ("move_pause", 25.0), ("point_band", 100.0),
        ("point_up", 50.0), ("pause_blend", 8.0), ("wander_turn", 5.0), ("wander_sink", 0.3), ("wander_low", 150.0), ("flip", -1.0),
        ("wander_min", 150.0), ("wander_max", 180.0), ("wander_blend", 8.0), ("wander_speed", 8.0), ("idle_search", 1000.0), ("fly_no_motion", 999.0),
        ("fly_blend", 8.0), ("fly_stuck", 0.3), ("stuck_half", 0.5), ("air_accel_set", 0.2), ("notice_world", 11.0), ("notice_blend", 8.0),
        ("wait_motion", 6.0), ("appear_fade", 30.0), ("appear_blend", 0.0), ("appear_queue_blend", 8.0), ("exp_div", 10.0), ("exp_add", 2.0),
        ("air_accel", 0.1), ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0), ("gravity", crate::params::GRAVITY), ("fall_cap", 20.0),
        ("down_time", 30.0), ("air_knock_hop", 20.0), ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0),
        ("air_flinch_speed", 4.0), ("air_knock_speed", 16.0), ("weight", 1.0), ("thunder_wait", 80.0), ("thunder_cloud", 200.0),
        ("thunder_delay", 30.0), ("thunder_radius", 50.0), ("thunder_half", 128.0), ("thunder_mid", 128.0), ("thunder_recom_id", 0.0),
        ("thunder_kind", 1.0),
    ];
    Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), &all_names()).unwrap()
}

/// The player's `xa_ex_2130.mset` lengths by request (the asset step's clips).
pub(super) fn frames() -> Vec<(u16, f32)> {
    vec![
        (0, 200.0), (1, 40.0), (2, 246.0), (67, 340.0), (68, 60.0), (69, 90.0), (70, 66.0), (72, 50.0), (73, 50.0), (74, 50.0), (75, 50.0),
        (76, 104.0), (77, 104.0), (78, 50.0), (79, 50.0), (80, 104.0), (81, 104.0), (103, 50.0), (104, 32.0), (105, 8.0), (205, 104.0),
        (206, 70.0), (207, 164.0), (208, 126.0), (209, 40.0), (210, 54.0), (211, 80.0), (214, 158.0), (218, 320.0), (219, 200.0), (220, 200.0),
    ]
}

/// The asset step's looping clips, by request.
pub(super) fn loops() -> Vec<u16> {
    vec![0, 1, 2, 67, 104, 209, 218, 219, 220]
}

/// Re:CoM's states for its attack (the class row's map: the charge in Re:CoM's rush state 3).
const ATTACK_STATES: [i32; 1] = [3];

fn model(seed: u32, spawned: bool) -> Model<Consts> {
    let ai = Box::new(Ai::new(consts(), frames(), seed, 0.0, spawned));
    let mut m = Model::new(ai, consts(), frames(), loops(), ATTACK_STATES.to_vec());
    m.pos = Vec3::new(0.0, -150.0, 0.0);
    m
}

fn strikes(m: &Model<Consts>) -> Vec<(usize, Vec3)> {
    m.log.iter().filter_map(|(f, c)| if let ECmd::Strike { child: child::THUNDER, at } = c { Some((*f, *at)) } else { None }).collect()
}

fn asked(m: &Model<Consts>, r: u16) -> Vec<usize> {
    m.log.iter().filter(|(_, c)| matches!(c, ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Restart { request, .. } if *request == r)).map(|(f, _)| *f).collect()
}

fn marks(m: &Model<Consts>) -> Vec<usize> {
    m.log.iter().filter(|(_, c)| matches!(c, ECmd::Note(n) if n.starts_with("the Thunder's mark"))).map(|(f, _)| *f).collect()
}

/// Sora keeps `d` away (horizontally) from the Yellow Opera, on the floor, for `n` frames; his place each frame.
fn keep(m: &mut Model<Consts>, d: f32, n: usize) {
    for _ in 0..n {
        let s = Vec3::new(m.pos.x + d, 0.0, m.pos.z);
        m.run(s, 1);
    }
}

/// A spawned one plays KH1's appearance (69, an action, the idle hover queued) and fades in; far from Sora it idles
/// (219), facing yaw 0 (the family's idle leg); within 10 m it engages.
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
    assert!(strikes(&m).is_empty());
    m.run(Vec3::new(400.0, 0.0, 0.0), 600);
    assert!(m.log.iter().any(|(_, c)| *c == ECmd::State(st::ENGAGED)));
}

/// Far (beyond 5 m): 60 % of rounds the Thunder: 205 at blend 8 from frame 8, 206 queued from frame 2 and, once 206
/// plays, 207 from frame 3; the mark 80 frames into 207 at Sora's place then, on the floor; the strike 30 frames later
/// at the mark; then 220 from frame 8 and two repositions.
#[test]
fn far_it_casts_the_thunder() {
    let mut found = 0;
    for seed in 0..12 {
        let mut m = model(seed, false);
        // Sora walks a slow circle 8 m away (his place changes every frame).
        let mut sora_at = Vec::new();
        for f in 0..3000usize {
            let a = f as f32 / 300.0;
            let s = Vec3::new(a.sin() * 800.0, 0.0, a.cos() * 800.0);
            sora_at.push(s);
            m.run(s, 1);
        }
        let mk = marks(&m);
        for (f, at) in strikes(&m) {
            let mark = *mk.iter().filter(|&&x| x < f).max().unwrap();
            assert_eq!(f - mark, 30, "seed {seed}: the strike 30 frames after the mark");
            let s = sora_at[mark];
            assert!((at - Vec3::new(s.x, 0.0, s.z)).length() < 1e-3, "seed {seed}: the strike at Sora's place at the mark ({at:?} / {s:?})");
            // The mark on 207's 81st frame.
            let first207 = (0..mark).rev().take_while(|&i| m.trace[i].1 == Some(req::STRIKE)).last().unwrap();
            assert_eq!(mark - first207, 80, "seed {seed}");
            // 205 from 8, then the queues.
            let ask = asked(&m, req::CAST).into_iter().filter(|&a| a < mark).max().unwrap();
            assert!(m.log.iter().any(|(g, c)| *g == ask && matches!(c, ECmd::MotionAt { request: req::CAST, blend, start } if *blend == 8.0 && *start == 8.0)));
            assert!(m.log.iter().any(|(g, c)| *g == ask && matches!(c, ECmd::QueueAt { request: req::HOLD, start, .. } if *start == 2.0)));
            assert!(m.log.iter().any(|(g, c)| *g > ask && *g < mark && matches!(c, ECmd::QueueAt { request: req::STRIKE, start, .. } if *start == 3.0)));
            found += 1;
        }
        assert!(m.log.iter().any(|(_, c)| matches!(c, ECmd::MotionAt { request: req::REST, start, .. } if *start == 8.0)));
    }
    assert!(found > 0);
}

/// Near (under 5 m): a reposition, then half the time the charge: 208 at blend 8 from frame 8 to its end, the rush 209 as a
/// steered attack in Re:CoM's state 3 (toward where Sora stood) up to 150 frames; then 210 from frame 2 and 220.
#[test]
fn near_it_charges() {
    let mut found = 0;
    for seed in 0..12 {
        let mut m = model(seed, false);
        m.pos = Vec3::new(0.0, -100.0, 0.0);
        keep(&mut m, 300.0, 2500);
        for a in m.attacks_of(req::RUSH) {
            let wind = asked(&m, req::WIND_UP).into_iter().filter(|&w| w < a).max().unwrap();
            assert!(m.log.iter().any(|(g, c)| *g == wind && matches!(c, ECmd::MotionAt { request: req::WIND_UP, blend, start } if *blend == 8.0 && *start == 8.0)));
            // The wind-up played to its end first (126 frames; the model plays it from 0).
            assert!(a - wind >= 117, "seed {seed}: the rush {} frames after the wind-up", a - wind);
            let rush_len = m.trace[a..].iter().take_while(|t| t.1 == Some(req::RUSH) || t.1 == Some(req::WIND_UP)).count();
            assert!(rush_len <= 152, "seed {seed}: rush {rush_len} frames");
            found += 1;
        }
        assert!(m.trace.iter().all(|t| t.1 != Some(req::RUSH) || t.0 == 3), "seed {seed}: 209 outside state 3");
        if !m.attacks_of(req::RUSH).is_empty() {
            assert!(m.log.iter().any(|(_, c)| matches!(c, ECmd::MotionAt { request: req::RECOVER, start, .. } if *start == 2.0)), "seed {seed}: no recovery");
        }
    }
    assert!(found > 0);
}

/// The rush stops as soon as its rows land (`0x79A`: `wp+0` = 2) and the recovery follows.
#[test]
fn the_rush_stops_when_it_lands() {
    for seed in 0..12 {
        let mut m = model(seed, false);
        m.pos = Vec3::new(0.0, -100.0, 0.0);
        let mut landed_at = None;
        for _ in 0..3000 {
            let s = Vec3::new(m.pos.x + 300.0, 0.0, m.pos.z);
            m.run(s, 1);
            if landed_at.is_none() && m.current == Some(req::RUSH) && m.state == 3 {
                m.ai.on_landed(0);
                landed_at = Some(m.frame);
            }
            if let Some(l) = landed_at {
                if m.frame > l + 3 {
                    assert_ne!(m.current, Some(req::RUSH), "seed {seed}: still rushing after it landed");
                    assert!(asked(&m, req::RECOVER).iter().any(|&f| f >= l && f <= l + 2), "seed {seed}: no recovery after landing");
                    break;
                }
            }
        }
    }
}

/// The hit state: three flee legs at 40 a frame (220 playing), a drop to 100 above the floor, then the Thunder whatever the
/// distance.
#[test]
fn hit_it_flees_and_casts() {
    for seed in 0..8 {
        let mut m = model(seed, false);
        m.pos = Vec3::new(0.0, -100.0, 0.0);
        keep(&mut m, 300.0, 200);
        m.ai.on_hit(HitNews::Damage);
        let start = m.frame;
        let mut at_cast = None;
        for _ in 0..400 {
            let n = m.log.len();
            keep(&mut m, 300.0, 1);
            if at_cast.is_none() && m.log[n..].iter().any(|(_, c)| matches!(c, ECmd::MotionAt { request: req::CAST, .. })) {
                at_cast = Some((m.frame - start, m.pos));
            }
        }
        let (f, p) = at_cast.expect("a Thunder after the hit");
        assert!((4..=4 * 21 + 2).contains(&f), "seed {seed}: the flee's legs and the drop first ({f} frames)");
        assert!((p.y + 100.0).abs() <= 30.0 + 40.0, "seed {seed}: near 100 above the floor at the cast: {}", p.y);
    }
}

/// The hit handler: a living hit or a clash asks for the hit state; a repel record asks nothing; every hit marks the
/// rush (`wp+0` = 1).
#[test]
fn the_hit_handler() {
    let ai = Ai::new(consts(), frames(), 3, 0.0, false);
    ai.on_hit(HitNews::Repelled);
    assert_eq!(ai.io.borrow().state, (st::NONE, false));
    assert_eq!(ai.io.borrow().x.mark, Mark::Hit);
    ai.io.borrow_mut().x.mark = Mark::None;
    ai.on_landed(0);
    assert_eq!(ai.io.borrow().x.mark, Mark::Landed);
    ai.on_hit(HitNews::Damage);
    assert_eq!(ai.io.borrow().state, (st::HIT, true));
    assert!(ai.take_now().is_empty());
    ai.io.borrow_mut().state = (st::ENGAGED, false);
    ai.on_hit(HitNews::Clash);
    assert_eq!(ai.io.borrow().state, (st::HIT, true));
    assert_eq!(ai.reaction_kind(3), 3);
}

/// A reaction before the mark ends the Thunder; after the mark it strikes on time (side frames, dead frames); Re:CoM's
/// own states (Stop…) end it either way.
#[test]
fn the_thunder_and_reactions() {
    let w = |motion: Option<u16>| World { step: 1.0, pos: Vec3::new(0.0, -100.0, 0.0), target: Some(Vec3::new(200.0, 0.0, 0.0)), grounded: false, airborne: false, moved: 1.0, motion };
    let mut ai = Ai::new(consts(), frames(), 3, 0.0, false);
    ai.set_floor(0.0);
    ai.io.borrow_mut().x.strikes.push(Strike::new(0, req::CAST, req::HOLD, req::STRIKE, 80.0, 30.0));
    assert!(Behaviour::side_frame(&mut ai, w(Some(req::CAST))).is_empty());
    assert!(Behaviour::side_frame(&mut ai, w(Some(0x48))).is_empty());
    assert!(ai.io.borrow().x.strikes.is_empty());
    // Marked, then a reaction and its death: it strikes 30 frames after the mark.
    let mut ai = Ai::new(consts(), frames(), 3, 0.0, false);
    ai.set_floor(0.0);
    ai.io.borrow_mut().x.strikes.push(Strike::new(0, req::CAST, req::HOLD, req::STRIKE, 80.0, 30.0));
    let mut out = Behaviour::side_frame(&mut ai, w(Some(req::HOLD)));
    for _ in 0..81 {
        out.extend(Behaviour::side_frame(&mut ai, w(Some(req::STRIKE))));
    }
    assert!(out.iter().any(|c| matches!(c, ECmd::Note(_))));
    let mut hit = None;
    for f in 0..40 {
        let cmds = if f < 10 { Behaviour::side_frame(&mut ai, w(Some(0x48))) } else { Behaviour::dead_frame(&mut ai, w(Some(0x48))) };
        if cmds.iter().any(|c| matches!(c, ECmd::Strike { .. })) {
            hit = Some(f + 1);
        }
    }
    assert_eq!(hit, Some(30));
    // Re:CoM's state took over after the mark: no strike.
    let mut ai = Ai::new(consts(), frames(), 3, 0.0, false);
    ai.io.borrow_mut().x.strikes.push(Strike::new(0, req::CAST, req::HOLD, req::STRIKE, 1.0, 30.0));
    let _ = Behaviour::side_frame(&mut ai, w(Some(req::HOLD)));
    for _ in 0..3 {
        let _ = Behaviour::side_frame(&mut ai, w(Some(req::STRIKE)));
    }
    assert!(ai.io.borrow().x.strikes[0].marked());
    ai.on_recom_state();
    assert!(ai.io.borrow().x.strikes.is_empty());
}

/// 20 000-frame fights (12 seeds, Sora circling 1.5..15.5 m from the start): the AI never goes quiet for 600 frames,
/// nothing is released, it strikes with Thunders and charges only in state 3, and it stays in the air.
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
        assert!(!strikes(&m).is_empty(), "seed {seed}: no Thunder");
        assert!(!m.attacks_of(req::RUSH).is_empty(), "seed {seed}: no charge");
        assert!(m.trace.iter().all(|t| t.1 != Some(req::RUSH) || t.0 == 3), "seed {seed}: 209 outside state 3");
        assert!(m.ai.body().flying, "seed {seed}: it flies");
    }
}
