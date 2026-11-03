use super::*;
use crate::enemy::host::model::Model;

/// The player's `ex_2340.bd` / children / `btltbl.bin` / KH1 exe numbers as the asset step reads them (its manifest on
/// the session's files), and the host's points (any: the behaviour never reads them).
pub(super) fn consts() -> Consts {
    let v: &[(&str, f32)] = &[
        ("walk_speed", 2.5), ("run_speed", 8.0), ("turn_deg", 12.0), ("turn_slow_deg", 2.0), ("close_range", 470.0), ("far_range", 600.0),
        ("close_flame_chance", 0.3), ("close_mace_above", 0.4), ("close_follow_range", 400.0), ("close_follow_flame_above", 0.3), ("close_follow_far", 600.0),
        ("far_approach_chance", 0.3), ("far_fire_above", 0.5), ("mid_approach_chance", 0.3), ("mid_fire_above", 0.5), ("approach_near", 500.0),
        ("approach_near_min", 100.0), ("approach_near_max", 300.0), ("approach_far_min", 300.0), ("approach_far_max", 500.0), ("walk_blend", 8.0),
        ("walk_wait", 8.0), ("approach_speed", 2.5), ("approach_stop", 250.0), ("stuck_speed", 1.25), ("idle_blend", 8.0), ("idle_wait", 6.0),
        ("guard_break_min", 30.0), ("guard_break_max", 60.0), ("guard_time_min", 60.0), ("guard_time_max", 120.0), ("guard_break_chance", 0.5),
        ("guard_cone", 40.0), ("guard_hold_wait", 10.0), ("guard_aim_cone", 30.0), ("guard_mace_range", 350.0), ("guard_far", 600.0), ("guard_close", 200.0),
        ("guard_close_mace", 0.5), ("guard_drop_blend", 8.0), ("recoil_blend", 2.0), ("flame_len_less", 2.0), ("flame_turn_cone", 100.0),
        ("flame_turn_after", 180.0), ("flame_turn_near", 500.0), ("flame_turn_chance", 0.5), ("fire_len_less", 2.0), ("shot_len_less", 4.0),
        ("mace_cone", 30.0), ("face_cone", 30.0), ("back_deg", 180.0), ("back_cone", 50.0), ("side_deg", -90.0), ("side_cone", 80.0), ("step_max", 3.0),
        ("done_cone", 35.0), ("step_len_less", 2.0), ("step_deg", 0.45), ("turnaround_deg", 2.36), ("spin_range", 400.0), ("spin_deg", -3.0),
        ("fall_accel_a", 0.5), ("fall_accel_b", 0.5), ("fall_max", 20.0), ("hit_far", 400.0), ("hit_far_fire_above", 0.3), ("hit_near_mace", 0.3),
        ("hit_near_spin_above", 0.7), ("repel_count", 5.0), ("exp_div", 10.0), ("exp_add", 1.0), ("own_row_first", 12900.0), ("own_row_last", 12908.0),
        ("flame_child", 0.0), ("fire_child", 1.0), ("attack_blend", 8.0), ("air_accel", 0.1), ("air_drag", 0.95), ("fall_cap", 20.0), ("down_time", 30.0),
        ("air_knock_hop", 20.0), ("launch_min", 20.0), ("launch_max", 35.0), ("launch_hop", 1.3499999), ("flinch_push", 80.0), ("air_flinch_speed", 4.0),
        ("air_knock_speed", 16.0), ("weight", 1.0), ("flame_request", 212.0), ("flame_delay", 110.0), ("flame_rehit", 10.0), ("flame_t0", 3.0),
        ("flame_lift0", 25.0), ("flame_t1", 15.0), ("flame_lift1", 25.0), ("flame_ring", 8.0), ("flame_life", 140.0), ("flame_r0", 20.0), ("flame_r1", 40.0),
        ("flame_grow_until", 110.0), ("flame_grow_div", 30.0), ("flame_tail", 10.0), ("fire_t", 3.0), ("fire_turn", 1.0), ("fire_speed_min", 8.0),
        ("fire_speed_max", 10.0), ("fire_life", 100.0), ("fire_fade", 10.0), ("fire_radius", 60.0), ("fire_recom_id", 1.0), ("fire_kind", 1.0),
    ];
    let mut v: Vec<(String, f32)> = v.iter().map(|(k, x)| (k.to_string(), *x)).collect();
    for p in ["fire_at", "flame0", "flame1"] {
        for (s, x) in [("bone", 1.0), ("x", 0.0), ("y", 0.0), ("z", 0.0)] {
            v.push((format!("{p}_{s}"), x));
        }
    }
    v.push(("gravity".into(), crate::params::GRAVITY));
    v.push(("turn_rate".into(), std::f32::consts::PI / 16.0));
    Table::new(v, &all_names()).unwrap()
}

/// The player's `xa_ex_2340.mset` lengths by request (the asset step's clips).
pub(super) fn frames() -> Vec<(u16, f32)> {
    vec![
        (0, 200.0), (1, 108.0), (6, 200.0), (7, 72.0), (70, 60.0), (72, 88.0), (73, 88.0), (74, 92.0), (75, 92.0), (76, 96.0), (77, 102.0), (78, 76.0), (79, 76.0),
        (102, 200.0), (103, 88.0), (104, 200.0), (200, 60.0), (201, 76.0), (202, 60.0), (203, 32.0), (204, 140.0), (205, 26.0), (206, 60.0), (207, 208.0),
        (208, 120.0), (209, 128.0), (210, 100.0), (211, 100.0), (212, 280.0), (213, 66.0),
    ]
}

/// The asset step's looping clips (the pose rule), by request.
pub(super) fn loops() -> Vec<u16> {
    vec![0, 1, 6, 78, 79, 102, 104, 200, 201, 202, 204, 206, 212]
}

/// The class map (`ground::defender::CLASS.attack_states`).
const STATES: [i32; 3] = [2, 2, 2];

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
                        ECmd::Motion { request, .. } | ECmd::Attack { request, .. } => {
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

    fn yaw(&self) -> f32 {
        self.ai.body().yaw
    }
}

fn sim(seed: u32, spawned: bool) -> Sim {
    Sim { ai: Ai::new(consts(), frames(), seed, 0.0, spawned), pos: Vec3::ZERO, busy: 0.0, current: None, queued: None, age: 0.0, moved: 0.0, log: Vec::new(), f: 0 }
}

/// A Defender past its appearance (the engaged state asked for, its first round not begun).
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

/// A Defender past its appearance running `f` alone (Sora its target, as the hit state's thread).
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

/// The appearance: the shield's front row (mode 3) at its first frame; unhittable, 213 to its end, then hittable and
/// engaged (the 0.25-a-frame fall ends on the floor at once).
#[test]
fn the_appearance_and_the_front_row() {
    let mut s = sim(1, true);
    s.run(Vec3::new(0.0, 0.0, 800.0), 1);
    assert_eq!(s.cmds()[0], ECmd::Guard(Some((FRONT, ALWAYS))));
    s.run(Vec3::new(0.0, 0.0, 800.0), 100);
    let c = s.cmds();
    let off = c.iter().position(|x| *x == ECmd::Hittable(false)).expect("unhittable");
    let on = c.iter().position(|x| *x == ECmd::Hittable(true)).expect("hittable again");
    let appear = c.iter().position(|x| *x == ECmd::Motion { request: req::APPEAR, blend: 0.0 }).expect("213");
    assert!(off < appear && appear < on);
    let fa = s.log.iter().find(|x| x.1 == ECmd::Motion { request: req::APPEAR, blend: 0.0 }).unwrap().0;
    let fo = s.log.iter().find(|x| x.1 == ECmd::Hittable(true)).unwrap().0;
    assert!((63..=68).contains(&(fo - fa)), "213 plays to its end ({} frames)", fo - fa);
    assert!(s.ai.risen());
}

/// Sora to its side: side steps toward him (200 turning +0.45° a frame on its +90° side, 202 −0.45° on its −90°), the
/// first for 58 frames (26.1°), the next of the same request a frame (KH1 holds it at its end); then the strike after
/// three. Behind it: the turn-around (2.36° a frame for 74 frames).
#[test]
fn it_turns_its_shield_toward_sora() {
    // Sora at +90° (yaw +π/2: (1, 0, 0)).
    let mut s = with(3, |cx| async move { turn_to_target(&cx).await });
    let y0 = s.yaw();
    s.run(Vec3::new(500.0, 0.0, 0.0), 58);
    assert!(s.cmds().contains(&ECmd::Motion { request: req::STEP_A, blend: 0.0 }));
    assert!(((s.yaw() - y0).to_degrees() - 0.45 * 58.0).abs() < 0.05, "{}", (s.yaw() - y0).to_degrees());
    s.run(Vec3::new(500.0, 0.0, 0.0), 2);
    assert!(((s.yaw() - y0).to_degrees() - 0.45 * 59.0).abs() < 0.05, "the second step: one frame");
    s.run(Vec3::new(500.0, 0.0, 0.0), 300);
    let strikes = s.cmds().iter().filter(|c| matches!(c, ECmd::Action { request: req::STRIKE, attack: Some(Attack::Strike), .. })).count();
    assert_eq!(strikes, 1, "three steps, then the strike");
    // Sora at −90°: 202.
    let mut s = with(3, |cx| async move { turn_to_target(&cx).await });
    s.run(Vec3::new(-500.0, 0.0, 0.0), 58);
    assert!(s.cmds().contains(&ECmd::Motion { request: req::STEP_B, blend: 0.0 }));
    assert!((s.yaw().to_degrees() + 0.45 * 58.0).abs() < 0.05);
    // Behind: the turn-around.
    let mut s = with(3, |cx| async move { turn_to_target(&cx).await });
    s.run(Vec3::new(0.0, 0.0, -500.0), 74);
    assert!(s.cmds().contains(&ECmd::Motion { request: req::TURN, blend: 0.0 }));
    assert!(((s.yaw()).to_degrees() - 2.36 * 74.0).abs() < 0.1, "{}", s.yaw().to_degrees());
    // In front: nothing.
    let mut s = with(3, |cx| async move { turn_to_target(&cx).await });
    s.run(Vec3::new(10.0, 0.0, 500.0), 5);
    assert!(!s.cmds().iter().any(|c| matches!(c, ECmd::Motion { request: 200..=202, .. })));
}

/// `0x3362`: the cone tests.
#[test]
fn the_cone_rule() {
    let me = Vec3::ZERO;
    assert!(!outside(0.0, me, Some(Vec3::new(0.0, 0.0, 100.0)), 0.0, 30.0));
    assert!(outside(0.0, me, Some(Vec3::new(100.0, 0.0, 100.0)), 0.0, 30.0));
    assert!(!outside(0.0, me, Some(Vec3::new(0.0, -300.0, -100.0)), 180.0, 50.0), "height ignored, behind");
    assert!(!outside(0.0, me, Some(Vec3::new(-100.0, 0.0, 0.0)), -90.0, 80.0));
    assert!(outside(0.0, me, None, 0.0, 30.0));
}

/// The fireballs: 210 then 211 (queued), one shot (the child 1) at 211's start whatever the count; the flame: 212 with
/// the idle queued and the child 0 at once.
#[test]
fn one_fireball_per_round_and_the_flame() {
    let mut s = with(5, |cx| async move { fireballs(&cx, 3).await });
    s.run(Vec3::new(0.0, 0.0, 700.0), 260);
    let c = s.cmds();
    assert_eq!(c.iter().filter(|x| **x == ECmd::Throw(child::FIREBALL)).count(), 1);
    let up = s.log.iter().find(|x| x.1 == ECmd::Motion { request: req::FIRE_UP, blend: 0.0 }).unwrap().0;
    let shot = s.log.iter().find(|x| x.1 == ECmd::Throw(child::FIREBALL)).unwrap().0;
    assert_eq!(shot - up, 98, "the shot leaves 98 frames into 210");
    assert!(s.log.iter().any(|x| x.0 == shot && x.1 == ECmd::Motion { request: req::FIRE, blend: 0.0 }));
    let mut s = with(5, |cx| async move { flame(&cx).await });
    s.run(Vec3::new(0.0, 0.0, 300.0), 10);
    let c = s.cmds();
    let i = c.iter().position(|x| *x == ECmd::Motion { request: req::FLAME, blend: 0.0 }).unwrap();
    assert_eq!(&c[i..i + 3], &[ECmd::Motion { request: req::FLAME, blend: 0.0 }, ECmd::Queue { request: req::IDLE, blend: 0.0 }, ECmd::Throw(child::FLAME)]);
}

/// The spin: 208 with its row and its all-round guard row, turning −3° a frame (one turn in its 120 frames).
#[test]
fn the_spin_turns_once() {
    let mut s = with(6, |cx| async move { spin(&cx).await });
    s.run(Vec3::new(0.0, 0.0, 300.0), 125);
    let c = s.cmds();
    let i = c.iter().position(|x| matches!(x, ECmd::Attack { request: req::SPIN, attack: Attack::Spin, .. })).unwrap();
    assert_eq!(c[i + 1], ECmd::Guard(Some((SPIN_GUARD, req::SPIN))));
    let turned: f32 = s.log.iter().filter(|x| matches!(x.1, ECmd::Yaw(_))).count() as f32;
    assert!((118.0..=120.0).contains(&turned), "{turned}");
    let mut s = with(6, |cx| async move { spin(&cx).await });
    s.run(Vec3::new(0.0, 0.0, 450.0), 3);
    assert!(s.cmds().iter().any(|x| matches!(x, ECmd::Action { request: req::STRIKE, .. })), "beyond 400: the strike");
}

/// The guard: 203 (an action) then 204 held; Sora close (< 200) breaks out into the strike or the flame.
#[test]
fn the_guard() {
    let mut s = with(9, |cx| async move { guard(&cx).await });
    s.run(Vec3::new(0.0, 0.0, 550.0), 200);
    let c = s.cmds();
    assert_eq!(c[0..1], [ECmd::Action { request: req::GUARD_UP, attack: None, then: req::GUARD, blend: 0.0 }]);
    assert!(c.iter().any(|x| matches!(x, ECmd::Action { request: req::GUARD_DOWN, .. })) || c.iter().any(|x| matches!(x, ECmd::Motion { request: req::FLAME, .. } | ECmd::Action { request: req::STRIKE, .. } | ECmd::Motion { request: req::FIRE_UP, .. })));
    // Sora off its front: no guard at all.
    let mut s = with(9, |cx| async move { guard(&cx).await });
    s.run(Vec3::new(550.0, 0.0, 0.0), 5);
    assert!(!s.cmds().iter().any(|x| matches!(x, ECmd::Action { request: req::GUARD_UP, .. })));
}

/// The handler's repel count: four repels in a quiet motion each ask for the recoil 206; the fifth gives KH1's EXP
/// bonus and the bash 207; in the strike (busy) they only count.
#[test]
fn repels_count_to_the_bash() {
    let mut s = ready(2);
    s.run(Vec3::new(0.0, 0.0, 2000.0), 1);
    for i in 1..=4 {
        Behaviour::set_playing(&s.ai, Some(req::IDLE));
        Behaviour::on_hit(&s.ai, HitNews::Repelled);
        assert!(Behaviour::take_now(&s.ai).is_empty());
        assert!(s.ai.io.borrow().x.recoil && !s.ai.io.borrow().x.bash, "repel {i}");
        assert_eq!(s.ai.io.borrow().state, (st::HIT, true));
        s.ai.io.borrow_mut().state.1 = false;
    }
    Behaviour::on_hit(&s.ai, HitNews::Repelled);
    let now = Behaviour::take_now(&s.ai);
    assert_eq!(now, vec![ECmd::Reward(Reward { exp_div: 10, exp_add: 1, exp_min: 0, prizes: [0; 7] })]);
    assert!(s.ai.io.borrow().x.bash && s.ai.io.borrow().x.repels == 0);
    s.run(Vec3::new(0.0, 0.0, 2000.0), 3);
    assert!(s.cmds().iter().any(|c| matches!(c, ECmd::Action { request: req::BASH, attack: Some(Attack::Bash), .. })));
    // Busy (the strike): counted, nothing asked.
    let s = ready(2);
    s.ai.io.borrow_mut().state.1 = false;
    Behaviour::set_playing(&s.ai, Some(req::STRIKE));
    for _ in 0..6 {
        Behaviour::on_hit(&s.ai, HitNews::Repelled);
    }
    assert!(!s.ai.io.borrow().state.1 && s.ai.io.borrow().x.repels == 6);
    // The sixth, in its fireball: the bash.
    Behaviour::set_playing(&s.ai, Some(req::FIRE));
    Behaviour::on_hit(&s.ai, HitNews::Repelled);
    assert!(s.ai.io.borrow().x.bash);
}

/// Its own batted fireball on its front: no armour, KH1's EXP bonus, a knock-back (kind 4), the hit state; from behind
/// the plain path. KH1's no-reaction hits: the strike, the spin, the turn-around, and the bash from the front.
#[test]
fn its_own_fireball_and_the_no_reaction_hits() {
    let s = ready(4);
    // Facing +z; a push direction −z: the attacker in front.
    Behaviour::set_hit_dir(&s.ai, Vec3::new(0.0, 0.0, -1.0));
    Behaviour::set_hit_own_row(&s.ai, true);
    Behaviour::set_hit_kind(&s.ai, 3);
    assert!(!Behaviour::armoured(&s.ai, Some(req::STRIKE), 0, Some(3)));
    Behaviour::set_playing(&s.ai, Some(req::STRIKE));
    Behaviour::on_hit(&s.ai, HitNews::Damage);
    assert_eq!(Behaviour::reaction_kind(&s.ai, 3), 4);
    assert_eq!(Behaviour::take_now(&s.ai).len(), 1);
    assert_eq!(s.ai.io.borrow().state, (st::HIT, true));
    // From behind: the plain path (here in the strike: no reaction).
    Behaviour::set_hit_dir(&s.ai, Vec3::new(0.0, 0.0, 1.0));
    assert!(Behaviour::armoured(&s.ai, Some(req::STRIKE), 0, Some(3)));
    Behaviour::set_hit_own_row(&s.ai, false);
    for r in [req::STRIKE, req::SPIN, req::TURN] {
        assert!(Behaviour::armoured(&s.ai, Some(r), 0, Some(3)), "{r}");
    }
    assert!(!Behaviour::armoured(&s.ai, Some(req::BASH), 0, Some(3)), "the bash from behind reacts");
    Behaviour::set_hit_dir(&s.ai, Vec3::new(0.0, 0.0, -1.0));
    assert!(Behaviour::armoured(&s.ai, Some(req::BASH), 0, Some(3)));
    assert!(!Behaviour::armoured(&s.ai, Some(req::IDLE), 0, Some(3)));
    assert!(!Behaviour::armoured(&s.ai, Some(req::STRIKE), 0, Some(6)), "never a clash");
    // A plain damaging hit: the hit state; its reaction kept.
    s.ai.io.borrow_mut().state.1 = false;
    Behaviour::set_playing(&s.ai, Some(req::IDLE));
    Behaviour::on_hit(&s.ai, HitNews::Damage);
    assert!(Behaviour::keeps_reaction(&s.ai) && s.ai.io.borrow().state == (st::HIT, true));
    assert_eq!(Behaviour::reaction_kind(&s.ai, 3), 3);
    // The killing hit: a flinch becomes a knock-back.
    Behaviour::on_defeat(&s.ai);
    assert_eq!(Behaviour::reaction_kind(&s.ai, 3), 4);
}

/// A long fight: the AI acts at least every 600 frames (the host's quiet watchdog), with Sora circling 2–7 m away and
/// a hit every 150–400 frames (12 seeds, 20 000 frames each); it shoots, burns, guards, strikes, spins and turns.
#[test]
fn a_long_fight_never_stalls() {
    let mut seen = [false; 6];
    for seed in 0..12u32 {
        let mut s = ready(seed);
        let (mut last, mut worst, mut next_hit) = (0usize, 0usize, 300usize);
        let mut r = Kh1Rand::new(seed + 100);
        for f in 0..20_000usize {
            let a = f as f32 / 600.0;
            let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
            let sora = Vec3::new(a.sin() * rad, 0.0, a.cos() * rad);
            if f == next_hit {
                let x = r.next_f32();
                Behaviour::set_playing(&s.ai, s.current);
                Behaviour::on_hit(&s.ai, if x < 0.15 { HitNews::Clash } else if x < 0.4 { HitNews::Repelled } else { HitNews::Damage });
                if x >= 0.4 && Behaviour::keeps_reaction(&s.ai) {
                    s.busy = 34.0;
                }
                let _ = Behaviour::take_now(&s.ai);
                next_hit = f + 150 + (r.next_f32() * 250.0) as usize;
            }
            let n = s.log.len();
            s.run(sora, 1);
            if s.log[n..].iter().any(|c| !matches!(c.1, ECmd::Yaw(_))) || s.busy > 0.0 || s.ai.body().walk != Vec3::ZERO {
                worst = worst.max(f - last);
                last = f;
            }
        }
        assert!(worst < 600, "seed {seed}: {worst} frames without acting");
        let c = s.cmds();
        for (i, p) in [
            c.contains(&ECmd::Throw(child::FIREBALL)),
            c.contains(&ECmd::Throw(child::FLAME)),
            c.iter().any(|x| matches!(x, ECmd::Action { request: req::GUARD_UP, .. })),
            c.iter().any(|x| matches!(x, ECmd::Action { request: req::STRIKE, .. })),
            c.iter().any(|x| matches!(x, ECmd::Attack { request: req::SPIN, .. })),
            c.iter().any(|x| matches!(x, ECmd::Motion { request: req::STEP_A | req::STEP_B | req::TURN, .. })),
        ]
        .into_iter()
        .enumerate()
        {
            seen[i] |= p;
        }
    }
    assert_eq!(seen, [true; 6], "fireball, flame, guard, strike, spin, turn");
}

/// The Defender through the host's rules (`crate::enemy::host::model`, what `kh1-recom`'s `win::kh1_enemy` decides) with
/// its class map and the asset step's looping clips: a 20 000-frame fight, 12 seeds: nothing released, the strike and
/// the spin only in state 2, none asked for again within two frames.
#[test]
fn hosted_a_long_fight_never_loops_an_attack() {
    for seed in 0..12u32 {
        let Sim { ai, pos, .. } = ready(seed);
        let mut m = Model::new(Box::new(ai), consts(), frames(), loops(), STATES.to_vec());
        m.pos = pos;
        for f in 0..20_000usize {
            let a = f as f32 / 600.0;
            let rad = 200.0 + 250.0 * (1.0 + (f as f32 / 1700.0).sin());
            m.run(Vec3::new(a.sin() * rad, 0.0, a.cos() * rad), 1);
        }
        assert_eq!(m.released, 0, "seed {seed}");
        for r in [req::STRIKE, req::SPIN] {
            assert!(m.trace.iter().all(|t| t.1 != Some(r) || t.0 == 2 || t.0 == 1), "seed {seed}: {r} outside its state");
        }
        let issued: Vec<(usize, u16)> = m.log.iter().filter_map(|(f, c)| match c {
            ECmd::Attack { request, .. } => Some((*f, *request)),
            ECmd::Action { request, attack: Some(_), .. } => Some((*f, *request)),
            _ => None,
        }).collect();
        assert!(issued.len() > 10, "seed {seed}: it attacks ({})", issued.len());
        for w in issued.windows(2) {
            assert!(w[1].1 != w[0].1 || w[1].0 > w[0].0 + 2, "seed {seed}: {} asked again at {} after {}", w[0].1, w[1].0, w[0].0);
        }
    }
}

/// The rows, guard rows and children.
#[test]
fn rows_and_children() {
    assert_eq!(Attack::ALL.map(|a| (a.request(), a.rows()[0])), [(209, 0x15C), (208, 0x1CC), (207, 0x3FC)]);
    assert_eq!(Attack::ALL.map(|a| a.index()), [0, 1, 2]);
    assert_eq!((FRONT, SPIN_GUARD, child::FLAME, child::FIREBALL), (0, 1, 0, 1));
    assert_eq!([busy_class(Some(209)), busy_class(Some(211)), busy_class(Some(212)), busy_class(Some(204)), busy_class(None)], [0, 1, 1, 2, 2]);
}
