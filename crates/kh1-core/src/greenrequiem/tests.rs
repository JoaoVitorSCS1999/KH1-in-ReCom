use super::*;
use crate::enemy::host::model::Model;

/// The player's `ex_2140.bd` / `ex_2140_00.bd` / `btltbl.bin` / KH1 exe numbers as the asset step reads them (its
/// manifest on the session's files, 2026-11-03).
pub(super) fn consts() -> Consts {
    let v: &[(&str, f32)] = &[
        ("walk_speed", 2.0), ("run_speed", 8.0), ("cure_chance", 0.8), ("move_chance", 0.3), ("cure_point_min", 500.0), ("cure_point_max", 600.0),
        ("pick_entry", 1.0), ("pick_range", 5000.0), ("pick_ratio", 1.0), ("pick_weight_0", 60.0), ("pick_weight_1", 30.0), ("pick_weight_2", 10.0),
        ("pick_weight_3", 0.0), ("cure_child", 0.0), ("cast_blend", 8.0), ("cast_start", 8.0), ("cast_speed", 6.0), ("cast_stop", 10.0), ("cast_up", 180.0),
        ("cast_high", 180.0), ("cast_point_min", 600.0), ("cast_point_max", 800.0), ("rise_up", 180.0), ("floor_probe", 20.0), ("rise_speed", 6.0),
        ("rise_stop", 100.0), ("rise_aim", 0.0), ("rise_high", 180.0), ("rise_time", 300.0), ("move_min", 500.0), ("move_max", 600.0),
        ("move_speed_min", 6.0), ("move_speed_max", 7.2), ("move_stop", 10.0), ("move_up", 180.0), ("move_high", 180.0), ("move_time", 240.0),
        ("wander_turn", 5.0), ("wander_sink", 0.3), ("wander_low", 150.0), ("flip", -1.0), ("wander_min", 150.0), ("wander_max", 180.0),
        ("wander_blend", 8.0), ("wander_speed", 8.0), ("idle_search", 1000.0), ("fly_no_motion", 999.0), ("fly_blend", 8.0), ("fly_stuck", 0.3),
        ("stuck_half", 0.5), ("air_accel_set", 0.2), ("notice_world", 11.0), ("notice_blend", 8.0), ("wait_motion", 6.0), ("appear_fade", 30.0),
        ("appear_blend", 0.0), ("appear_queue_blend", 8.0), ("exp_div", 10.0), ("exp_add", 2.0), ("cure_wait", 186.0), ("air_accel", 0.1),
        ("air_drag", 0.95), ("turn_rate", std::f32::consts::PI / 16.0), ("gravity", crate::params::GRAVITY), ("fall_cap", 20.0), ("down_time", 30.0),
        ("air_knock_hop", 20.0), ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.35), ("flinch_push", 80.0), ("air_flinch_speed", 4.0),
        ("air_knock_speed", 16.0), ("weight", 1.0), ("attack_blend", 8.0),
    ];
    Table::new(v.iter().map(|(k, x)| (k.to_string(), *x)).collect(), &all_names()).unwrap()
}

/// The player's `xa_ex_2140.mset` lengths by request (the asset step's clips; 212 / 213 from frame 8).
pub(super) fn frames() -> Vec<(u16, f32)> {
    vec![
        (0, 200.0), (1, 200.0), (2, 246.0), (66, 60.0), (67, 340.0), (68, 60.0), (69, 90.0), (70, 66.0), (72, 50.0), (73, 50.0), (74, 50.0), (75, 50.0),
        (76, 104.0), (77, 104.0), (78, 50.0), (79, 50.0), (80, 104.0), (81, 104.0), (103, 50.0), (104, 32.0), (105, 8.0), (212, 128.0), (213, 242.0),
        (214, 158.0), (219, 200.0), (220, 200.0),
    ]
}

/// The asset step's looping clips, by request.
pub(super) fn loops() -> Vec<u16> {
    vec![0, 1, 2, 67, 104, 212, 213, 219, 220]
}

fn model(seed: u32, spawned: bool) -> Model<Consts> {
    let ai = Box::new(Ai::new(consts(), frames(), seed, 0.0, spawned));
    let mut m = Model::new(ai, consts(), frames(), loops(), Vec::new());
    m.pos = Vec3::new(0.0, -150.0, 0.0);
    m
}

fn cures(m: &Model<Consts>) -> Vec<usize> {
    m.log.iter().filter(|(_, c)| *c == ECmd::Throw(child::CURE)).map(|(f, _)| *f).collect()
}

fn picks(m: &Model<Consts>) -> Vec<(usize, f32)> {
    m.log.iter().filter_map(|(f, c)| match c {
        ECmd::Support { child: child::CURE, draw } => Some((*f, *draw)),
        _ => None,
    }).collect()
}

fn asked(m: &Model<Consts>, r: u16) -> Vec<usize> {
    m.log.iter().filter(|(_, c)| matches!(c, ECmd::Motion { request, .. } | ECmd::MotionAt { request, .. } | ECmd::Restart { request, .. } if *request == r)).map(|(f, _)| *f).collect()
}

/// A spawned one plays KH1's appearance (69, an action, the idle hover queued) and fades in, then idles facing yaw 0;
/// Sora within 10 m engages it.
#[test]
fn it_appears_then_idles_or_engages() {
    let mut m = model(1, true);
    m.run(Vec3::new(3000.0, 0.0, 0.0), 120);
    let first = m.log.iter().find(|(_, c)| matches!(c, ECmd::Action { .. })).unwrap();
    assert!(matches!(first.1, ECmd::Action { request: req::APPEAR, then: req::WANDER, attack: None, .. }));
    assert!(m.log.iter().any(|(_, c)| *c == ECmd::FadeIn(30.0)));
    m.run(Vec3::new(3000.0, 0.0, 0.0), 400);
    let late: Vec<_> = m.log.iter().filter(|(f, c)| *f > 200 && matches!(c, ECmd::Yaw(_))).collect();
    assert!(!late.is_empty() && late.iter().all(|(_, c)| *c == ECmd::Yaw(0.0)), "{late:?}");
    assert!(cures(&m).is_empty() && picks(&m).is_empty());
    m.run(Vec3::new(400.0, 0.0, 0.0), 600);
    assert!(m.log.iter().any(|(_, c)| *c == ECmd::State(st::ENGAGED)));
}

/// The Cure: its pick (`sys 102`'s draw) in the frame 212 is asked, 213 queued at blend 8, and the child acts 186 frames
/// into 213 (the model plays 212's 128 frames first); every Cure follows its own pick.
#[test]
fn the_cure_acts_186_frames_into_its_cast() {
    let mut found = 0;
    for seed in 0..12 {
        let mut m = model(seed, false);
        m.run(Vec3::new(300.0, 0.0, 0.0), 4000);
        let ps = picks(&m);
        assert!(ps.iter().all(|&(_, d)| (0.0..1.0).contains(&d)));
        for &(f, _) in &ps {
            assert!(asked(&m, req::CHARGE).contains(&f), "seed {seed}: the pick at {f} with 212");
            assert!(m.log.iter().any(|(g, c)| *g == f && *c == ECmd::Queue { request: req::CAST, blend: 8.0 }));
        }
        for c in cures(&m) {
            let ask = asked(&m, req::CHARGE).into_iter().filter(|&a| a < c).max().unwrap();
            assert_eq!(c - ask, 128 + 186, "seed {seed}: 212 asked at {ask}, the Cure at {c}");
            assert!(ps.iter().any(|&(f, _)| f == ask));
            found += 1;
        }
    }
    assert!(found > 0);
}

/// While it casts it flies toward 180 above the floor under Sora (y down: −180), facing him each frame.
#[test]
fn it_casts_hovering_over_sora() {
    let mut m = model(4, false);
    let sora = Vec3::new(300.0, 0.0, 200.0);
    m.run(sora, 1500);
    let casting: Vec<usize> = m.trace.iter().enumerate().filter(|(_, t)| t.1 == Some(req::CAST)).map(|(f, _)| f).collect();
    assert!(!casting.is_empty());
    let mut m2 = model(4, false);
    m2.run(sora, *casting.last().unwrap());
    assert!((m2.pos.y + 180.0).abs() < 40.0, "{:?}", m2.pos);
    assert!(Vec3::new(m2.pos.x - sora.x, 0.0, m2.pos.z - sora.z).length() < 900.0, "{:?}", m2.pos);
}

/// A reaction (another motion) in the wait: the child ends unseen, no Cure.
#[test]
fn a_reaction_in_the_wait_ends_the_cure() {
    let mut ai = Ai::new(consts(), frames(), 3, 0.0, false);
    let w = |motion: Option<u16>| World { step: 1.0, pos: Vec3::new(0.0, -150.0, 0.0), target: Some(Vec3::new(200.0, 0.0, 0.0)), grounded: false, airborne: false, moved: 1.0, motion };
    ai.io.borrow_mut().x.casts.push(Cast::new(child::CURE, req::CHARGE, req::CAST, 186.0));
    assert!(Behaviour::side_frame(&mut ai, w(Some(req::CHARGE))).is_empty());
    assert!(Behaviour::side_frame(&mut ai, w(Some(req::CAST))).is_empty());
    assert!(Behaviour::side_frame(&mut ai, w(Some(0x48))).is_empty());
    assert!(ai.io.borrow().x.casts.is_empty());
    // A dying caster's: its cast motion still playing counts on (KH1's child tests its motion), a reaction ends it.
    ai.on_defeat();
    ai.io.borrow_mut().x.casts.push(Cast::new(child::CURE, req::CHARGE, req::CAST, 2.0));
    let mut out = Vec::new();
    for _ in 0..3 {
        out.extend(Behaviour::dead_frame(&mut ai, w(Some(req::CAST))));
    }
    assert_eq!(out, vec![ECmd::Throw(child::CURE)]);
}

/// The hit handler: a living hit or a clash asks for the hit state (a Cure, then to its height); a repel asks nothing;
/// no reaction kind is rewritten.
#[test]
fn the_hit_handler() {
    let ai = Ai::new(consts(), frames(), 3, 0.0, false);
    ai.on_hit(HitNews::Repelled);
    assert_eq!(ai.io.borrow().state, (st::NONE, false));
    ai.on_hit(HitNews::Damage);
    assert_eq!(ai.io.borrow().state, (st::HIT, true));
    assert!(ai.take_now().is_empty());
    assert_eq!(ai.reaction_kind(3), 3);
    let mut m = model(5, false);
    m.run(Vec3::new(300.0, 0.0, 0.0), 300);
    m.ai.on_hit(HitNews::Damage);
    let n = m.log.len();
    m.run(Vec3::new(300.0, 0.0, 0.0), 3);
    assert!(m.log[n..].iter().any(|(_, c)| matches!(c, ECmd::Support { .. })), "the hit state casts at once");
}

fn ally(id: u32, hp: i32, max: i32) -> Ally {
    Ally { id, hp, max, dist: 300.0 }
}

/// KH1's pick (`0x1402B80A0`): the lowest HP ratio 60 %, the second 30 %, the third 10 %; with fewer allies the draws
/// fall back as KH1's branches do; nobody listed: none (the caster itself).
#[test]
fn the_pick_follows_kh1s_rule() {
    let a = [ally(1, 50, 100), ally(2, 10, 100), ally(3, 100, 100), ally(4, 70, 100)];
    let p = |d: f32, v: &[Ally]| pick(v, d, 5000.0, 1.0, 60, 30);
    assert_eq!(p(0.0, &a), Some(2));
    assert_eq!(p(0.59, &a), Some(2));
    assert_eq!(p(0.6, &a), Some(1));
    assert_eq!(p(0.89, &a), Some(1));
    assert_eq!(p(0.9, &a), Some(4));
    assert_eq!(p(0.999, &a), Some(4));
    // Two: the 10 % branch takes the second; one: always it.
    assert_eq!(p(0.95, &a[..2]), Some(1));
    assert_eq!(p(0.95, &a[..1]), Some(1));
    assert_eq!(p(0.1, &[]), None);
    // Max HP 0 or out of range: not listed.
    assert_eq!(p(0.1, &[Ally { id: 7, hp: 0, max: 0, dist: 1.0 }, Ally { id: 8, hp: 1, max: 10, dist: 6000.0 }]), None);
    // A dying ally (HP 0) is listed first (KH1: its fade keeps it in the actor list; the heal then finds nothing).
    assert_eq!(p(0.1, &[ally(1, 50, 100), ally(9, 0, 100)]), Some(9));
    // Equal keys keep the CRT's order: three at full HP sort to [2, 3, 1].
    let full = [ally(1, 100, 100), ally(2, 100, 100), ally(3, 100, 100)];
    assert_eq!((p(0.1, &full), p(0.7, &full), p(0.95, &full)), (Some(2), Some(3), Some(1)));
}

/// The CRT sort sorts (any length; its ties are its own).
#[test]
fn crt_qsort_sorts() {
    let mut r = Kh1Rand::new(7);
    for n in 0..40usize {
        let mut v: Vec<(i32, usize)> = (0..n).map(|i| ((r.next_f32() * 10.0) as i32, i)).collect();
        let mut keys: Vec<i32> = v.iter().map(|x| x.0).collect();
        crt_qsort(&mut v, |a, b| a.0 - b.0);
        keys.sort();
        assert_eq!(v.iter().map(|x| x.0).collect::<Vec<_>>(), keys, "n {n}");
        let mut ids: Vec<usize> = v.iter().map(|x| x.1).collect();
        ids.sort();
        assert_eq!(ids, (0..n).collect::<Vec<_>>());
    }
}

/// 20 000-frame fights (12 seeds, Sora circling 1.5..15.5 m from the start): the AI never goes quiet for 600 frames, it
/// casts its Cure, and it never asks for an attack (it has none).
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
        assert!(!cures(&m).is_empty(), "seed {seed}: no Cure");
        assert!(m.log.iter().all(|(_, c)| !matches!(c, ECmd::Attack { .. } | ECmd::AttackAt { .. })));
        assert!(m.pos.y.is_finite());
    }
}
