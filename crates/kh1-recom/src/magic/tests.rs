//! KH1's Fire numbers in Re:CoM's row and shot formats; the spell damage. The cast's frames are
//! tested on the movement tests' fake Re:CoM (`movement/tests.rs`, "M7 stage 7.4").

use super::*;

#[test]
fn fire_rows_keep_recoms_clip_and_effect_set_and_take_kh1s_ball() {
    let t = kh1_magic_table(&RECOM_FIRE_ROWS, &RECOM_BLIZZARD_ROWS, &RECOM_THUNDER_ROWS);
    assert_eq!(t.len(), 21);
    for lv in 1..=3u8 {
        let i = table_index(spell::FIRE, lv).unwrap();
        let r = &t[i];
        let base = &RECOM_FIRE_ROWS[lv as usize - 1];
        // Re:CoM's clip (0x37), blend and Fire / Fira / Firaga effect set (3 / 4 / 5).
        assert_eq!(&r[..3], &base[..3]);
        assert_eq!(r[2], 2 + lv);
        // KH1's ball: 50 cm, 40 cm a frame, 3° a frame, 30 frames, a flinch, no range or gate.
        assert_eq!((r[4], r[8], r[9], r[0xB], r[0xC], r[0xD], r[0xE]), (50, 255, 40, 1, 255, 3, 15));
        assert_eq!(r[0xF] & FLAG_KH1, FLAG_KH1);
        assert_eq!(recom_base_row(spell::FIRE, lv), Some([0x22, 0x29, 0x2A][lv as usize - 1]));
    }
    // Cure … Aero come with their stages.
    assert!(t[9..].iter().all(|r| r[0] == 0));
    assert_eq!(table_index(spell::AERO, 3), Some(20));
    assert_eq!(table_index(spell::FIRE, 0), None);
    assert!(castable(spell::FIRE) && castable(spell::BLIZZARD) && castable(spell::THUNDER) && !castable(spell::CURE));
}

#[test]
fn shot_numbers_are_kh1s_in_recoms_units() {
    let row = fire_row_bytes(&RECOM_FIRE_ROWS[0]);
    let n = shot_numbers(&row);
    // 40 cm a frame = 0.40 m per 60 Hz frame (Re:CoM's mover takes `+0x80` per frame).
    assert!((n.speed - 0.40).abs() < 1e-6);
    // 30 frames × 50 ticks.
    assert_eq!(n.life, 1500);
    assert_eq!(n.fade, 1000);
    assert!((n.radius - 0.5).abs() < 1e-6);
    assert_eq!((n.turn, n.gate, n.range), (3.0, 255.0, 255.0));
    // Re:CoM's Fire flags without the 0.5 m homing delay, with the life ending the ball.
    assert_eq!(n.flags, 0x8104);
    assert_eq!(n.flags & 0x1000, 0);
    assert_eq!(n.flags & 0x100, 0x100, "a hit or wall still ends it, as Re:CoM's Fire");
    // The ball covers 12 m in its life (KH1: 40 × 30 cm, the command's range 1200).
    assert!((n.speed * n.life as f32 / 50.0 - 12.0).abs() < 1e-4);
}

#[test]
fn fire_attack_data_is_kh1s_per_level() {
    for (lv, p) in [(1, 20), (2, 28), (3, 36)] {
        let a = fire_attack(lv);
        assert_eq!((a.power, a.stat_source, a.element, a.kind, a.mp_charge, a.radius), (p, 1, 2, 3, 0, 50.0));
    }
    assert_eq!(fire::COST, 30);
    assert_eq!(fire::LIFE * fire::SPEED, 1200.0);
}

#[test]
fn spell_damage_is_power_plus_max_mp_through_the_curve_and_fire_resistance() {
    let mut rng = Kh1Rand::new(1);
    let mut def = DefenderStats { defence: 5, ..Default::default() };
    // Fire (20) + max MP 3 = 23 − Defence 5 = curve[18] = 48.
    assert_eq!(spell_damage(&fire_attack(1), 3, &def, &mut rng), 48);
    // Firaga (36) + 8 − 5 = 39: the curve's cap of 300.
    assert_eq!(spell_damage(&fire_attack(3), 8, &def, &mut rng), 300);
    // KH1's fire resistance (`resist[2]`), and the record's cap.
    def.resist[2] = 0.5;
    assert_eq!(spell_damage(&fire_attack(1), 3, &def, &mut rng), 24);
    def.resist[2] = 1.0;
    def.max_damage = 20;
    assert_eq!(spell_damage(&fire_attack(1), 3, &def, &mut rng), 20);
    // Physical resistance doesn't touch Fire.
    def.max_damage = 0;
    def.resist[1] = 0.0;
    assert_eq!(spell_damage(&fire_attack(1), 3, &def, &mut rng), 48);
}

#[test]
fn cast_motions_are_kh1s_frame_counts_and_windows() {
    let mut m = MotionSet::default();
    add_motions(&mut m);
    let len = |r| m.info(r).unwrap().length;
    assert_eq!((len(0x36), len(0x3D), len(0x83), len(0x8A)), (28.0, 60.0, 28.0, 52.0));
    let free = |r| m.info(r).unwrap().first(FlagKind::Free);
    assert_eq!((free(0x36), free(0x3D), free(0x8A)), (None, Some(46.0), Some(46.0)));
    assert!((m.info(0x3D).unwrap().root_at(10.0).z + 15.0).abs() < 1e-4);
    assert!(is_cast_request(0x36) && is_cast_request(0x8A) && !is_cast_request(0x3E) && !is_cast_request(200));
}

#[test]
fn trace_tokens() {
    assert_eq!(trace_token(None), "cast -");
    let c = Cast { start: CastStart { spell: 0, level: 2, air: true, fast: true }, wait: 3.0, age: 5.0, released: false, cancelled: false, shot: None, fresh: false };
    assert_eq!(trace_token(Some(&c)), "cast fire2 f5 fast w3 air");
    assert_eq!(CastEvent::Released { spell: 0, level: 1, frame: 28.0, shot: Some(7) }.token(), "release(fire1 f28 shot 7)");
}

#[test]
fn field_case_stub_calls_exe_magic_on_rbx_and_goes_on_at_the_default() {
    let b = field_case::stub();
    assert_eq!(&b[..4], &[0x48, 0x8B, 0xCB, 0xE8]);
    let call = i32::from_le_bytes(b[4..8].try_into().unwrap()) as i64;
    assert_eq!(field_case::CAVE_RVA as i64 + 8 + call, 0x0021_7EE0);
    assert_eq!(b[8], 0xE9);
    let jmp = i32::from_le_bytes(b[9..13].try_into().unwrap()) as i64;
    assert_eq!(field_case::CAVE_RVA as i64 + 13 + jmp, 0x001F_A599);
    assert_eq!(field_case::entry(), [0xE1, 0xB0, 0x1F, 0x00]);
}

#[test]
fn fire_retarget_is_kh1s_pick() {
    use retarget::*;
    // 500 cm = 5 m; the weight is the script's 2.0.
    assert_eq!((RANGE_M, WEIGHT), (5.0, 2.0));
    let ball = [0.0, 1.0, 0.0];
    let fwd = [0.0, 0.0, 1.0];
    // Straight ahead: score = distance. Straight behind: × (1 + 2 × 2) = 5. Beside: × 3.
    assert_eq!(score(ball, fwd, [0.0, 1.0, 2.0]), Some((2.0, 2.0)));
    assert_eq!(score(ball, fwd, [0.0, 1.0, -2.0]), Some((2.0, 10.0)));
    assert_eq!(score(ball, fwd, [2.0, 1.0, 0.0]), Some((2.0, 6.0)));
    // The range is strict and 3D (KH1's `d < 500`, `0x1402A9240`).
    assert_eq!(score(ball, fwd, [0.0, 1.0, 5.0]), None);
    assert!(score(ball, fwd, [0.0, 4.0, 4.0]).is_none());
    assert!(score(ball, fwd, [0.0, 1.0, 4.99]).is_some());
    // On the ball's own point: distance 0, cosine 0 (`0x1402A8C70`), score 0.
    assert_eq!(score(ball, fwd, ball), Some((0.0, 0.0)));
    // A nearer enemy behind loses to a farther one ahead: 1.5 m behind scores 7.5, 4 m ahead 4.
    let c = [(1, [0.0, 1.0, -1.5]), (2, [0.0, 1.0, 5.0]), (3, [0.0, 1.0, 4.0])];
    assert_eq!(pick(ball, fwd, c), Some((3, 4.0)));
    // Nothing in range: no pick (the ball flies straight this frame).
    assert_eq!(pick(ball, fwd, [(1, [0.0, 1.0, 6.0])]), None);
    // Ties keep the first (only a smaller score replaces it).
    assert_eq!(pick(ball, fwd, [(7, [1.0, 1.0, 0.0]), (8, [-1.0, 1.0, 0.0])]), Some((7, 1.0)));
    // The pick follows the ball's direction (Re:CoM's aim angles, `guard::forward`).
    let left = crate::guard::forward(0.0, -std::f32::consts::FRAC_PI_2);
    assert_eq!(pick(ball, left, [(1, [-3.0, 1.0, 0.0]), (2, [0.0, 1.0, 2.0])]).map(|p| p.0), Some(1));
    // The cosine is clamped and the KH1 → Re:CoM axis flip leaves it unchanged.
    assert_eq!(cos([1.0, 0.0, 0.0], [2.0, 0.0, 0.0]), 1.0);
    let (a, b) = ([1.0, 2.0, 3.0], [-2.0, 0.5, 1.0]);
    assert!((cos(a, b) - cos([a[0], -a[1], -a[2]], [b[0], -b[1], -b[2]])).abs() < 1e-6);
}

#[test]
fn blizzard_rows_and_numbers_are_kh1s() {
    let t = kh1_magic_table(&RECOM_FIRE_ROWS, &RECOM_BLIZZARD_ROWS, &RECOM_THUNDER_ROWS);
    for lv in 1..=3u8 {
        let r = &t[table_index(spell::BLIZZARD, lv).unwrap()];
        let base = &RECOM_BLIZZARD_ROWS[lv as usize - 1];
        // Re:CoM's clip 0x3A and its Blizzard / Blizzara / Blizzaga sets 6 / 8 / 10.
        assert_eq!(&r[..3], &base[..3]);
        assert_eq!(r[2], [6, 8, 10][lv as usize - 1]);
        assert_eq!(r[5], base[5]);
        // KH1: radius 70 / 90 / 120 cm, 20 / 30 / 40 cm a frame, 26 frames (13 × 100 ticks).
        let i = lv as usize - 1;
        assert_eq!((r[4], r[9], r[0xB], r[0xC], r[0xD], r[0xE]), ([70, 90, 120][i], [20, 30, 40][i], 1, 255, 0, 13));
        assert_eq!(r[0xF] & FLAG_KH1, FLAG_KH1);
        assert_eq!(recom_base_row(spell::BLIZZARD, lv), Some([0x23, 0x2B, 0x2C][i]));
        let a = blizzard_attack(lv);
        assert_eq!((a.power, a.stat_source, a.element, a.kind, a.mp_charge, a.radius), ([22, 27, 34][i], 1, 3, 3, 0, [70.0, 90.0, 120.0][i]));
    }
    assert_eq!((cost(spell::FIRE), cost(spell::BLIZZARD), cost(spell::THUNDER)), (30, 30, 100));
    assert_eq!(release_wait(spell::BLIZZARD), 28.0);
    assert_eq!(KH1_BLIZZARD_FLAGS & 0x104, 0x104, "a hit, a wall or the life ends a shard");
    assert_eq!(KH1_BLIZZARD_FLAGS & 0x8000_1000, 0, "no homing");
    assert_eq!((cast_req::first(spell::BLIZZARD, false), cast_req::first(spell::BLIZZARD, true)), (0x37, 0x84));
    assert_eq!((cast_req::release(spell::BLIZZARD, false), cast_req::release(spell::FIRE, true)), (0x37, 0x8A));
    assert!(is_cast_request(0x37) && is_cast_request(0x84) && !is_cast_request(0x39));
    assert_eq!(spell_attack(spell::BLIZZARD, 2).map(|a| a.power), Some(27));
}

#[test]
fn blizzard_shards_are_kh1s_cone() {
    for yaw in [0.0f32, 1.0, -2.5] {
        let s = blizzard_shards(yaw);
        let f = [yaw.sin(), 0.0, yaw.cos()];
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        // The straight one last, from the Keyblade point.
        assert_eq!(s[6].0, [0.0; 3]);
        assert!((dot(s[6].1, f) - 1.0).abs() < 1e-6);
        let mut ups = 0;
        for (off, dir) in &s[..6] {
            // 22.5° off the facing (the script's 0.924), a unit vector (0.383² + 0.924² ≈ 1).
            assert!((dot(*dir, f) - 0.924).abs() < 1e-4, "{dir:?}");
            assert!((dot(*dir, *dir) - 1.0).abs() < 1e-3);
            // Starting 19.15 cm off the axis, square to it, on the side the shard flies to.
            assert!(dot(*off, f).abs() < 1e-6);
            assert!((dot(*off, *off).sqrt() - 0.1915).abs() < 1e-4);
            assert!(dot(*off, *dir) > 0.0);
            if dir[1] > 0.01 {
                ups += 1;
            }
        }
        // Every 60° about the axis: two level, two up, two down.
        assert_eq!(ups, 2);
        assert_eq!(s[..6].iter().filter(|x| x.1[1].abs() < 1e-6).count(), 2);
        for i in 0..6 {
            let (a, b) = (s[i].1, s[(i + 1) % 6].1);
            let (ra, rb) = ([a[0] - 0.924 * f[0], a[1], a[2] - 0.924 * f[2]], [b[0] - 0.924 * f[0], b[1], b[2] - 0.924 * f[2]]);
            assert!((cos_between(ra, rb) - 0.5).abs() < 1e-3, "60° apart");
        }
    }
    fn cos_between(a: [f32; 3], b: [f32; 3]) -> f32 {
        retarget::cos(a, b)
    }
}

#[test]
fn blizzard_glide_is_kh1s() {
    // A level shard (dy 0) with the floor within 50 cm below sits 50 cm above it; the ray is 50 cm.
    assert!((blizzard_ray(0.0) - 0.5).abs() < 1e-6);
    assert!((blizzard_glide_y(1.0, 0.0, Some(0.62)) - 1.12).abs() < 1e-6);
    // No floor within reach: it keeps its own slope (a sinking shard 15 cm a frame).
    assert!((blizzard_glide_y(1.0, -0.15, None) - 0.85).abs() < 1e-6);
    // A sinking shard reaches further (KH1: 50 + its fall this frame), a rising one less.
    assert!((blizzard_ray(-0.15) - 0.65).abs() < 1e-6);
    assert!((blizzard_ray(0.15) - 0.35).abs() < 1e-6);
}

#[test]
fn thunder_rows_and_numbers_are_kh1s() {
    let t = kh1_magic_table(&RECOM_FIRE_ROWS, &RECOM_BLIZZARD_ROWS, &RECOM_THUNDER_ROWS);
    for lv in 1..=3u8 {
        let i = lv as usize - 1;
        let r = &t[table_index(spell::THUNDER, lv).unwrap()];
        let base = &RECOM_THUNDER_ROWS[i];
        // Re:CoM's clip 0x39, sets 12 / 13 / 14, its bolt's speed, radius and range.
        assert_eq!((&r[..3], r[4], r[9], r[0xC]), (&base[..3], base[4], base[9], base[0xC]));
        assert_eq!(r[2], 12 + lv - 1);
        assert_eq!((r[3], r[0xB], r[0xF] & FLAG_KH1), (0, 1, FLAG_KH1));
        assert_eq!(recom_base_row(spell::THUNDER, lv), Some([0x24, 0x2E, 0x2F][i]));
        let a = thunder_attack(lv);
        assert_eq!((a.power, a.stat_source, a.element, a.kind, a.mp_charge), ([16, 20, 26][i], 3, 4, 3, 0));
    }
    assert_eq!((cost(spell::THUNDER), release_wait(spell::THUNDER)), (100, 30.0));
    assert_eq!((cast_req::first(spell::THUNDER, false), cast_req::first(spell::THUNDER, true)), (0x38, 0x85));
    assert!(is_cast_request(0x38) && is_cast_request(0x85));
    // Thunder + max MP × 2: 16 + 6 = 22 − Defence 5 = curve[17]; × KH1's thunder resistance.
    let mut rng = Kh1Rand::new(1);
    let mut def = DefenderStats { defence: 5, ..Default::default() };
    let full = spell_damage(&thunder_attack(1), 3, &def, &mut rng);
    let fire = spell_damage(&fire_attack(1), 3, &def, &mut rng);
    assert!(full < fire, "Thunder 16 + 2 × 3 = 22 < Fire 20 + 3 = 23");
    def.resist[4] = 0.5;
    assert_eq!(spell_damage(&thunder_attack(1), 3, &def, &mut rng), full / 2);
    def.resist[2] = 0.0;
    assert_eq!(spell_damage(&thunder_attack(1), 3, &def, &mut rng), full / 2, "fire resistance doesn't touch Thunder");
}

#[test]
fn thunder_area_and_order_are_kh1s() {
    let c = [0.0, 0.0, 0.0];
    // The cylinder: 6 m around, ±5 m; a body counts when any of it is inside.
    let s = |x: f32, y: f32, r: f32| BodyShape::Sphere { c: [x, y, 0.0], r };
    assert!(body_in_area(c, &s(5.9, 0.0, 0.0)));
    assert!(!body_in_area(c, &s(6.1, 0.0, 0.0)));
    assert!(body_in_area(c, &s(6.4, 0.0, 0.5)), "its edge inside");
    assert!(body_in_area(c, &s(0.0, 4.9, 0.0)) && !body_in_area(c, &s(0.0, 5.2, 0.1)) && body_in_area(c, &s(0.0, -5.3, 0.4)));
    // The rim: 0.3 m out and 0.3 m up from the edge is 0.42 m away.
    assert!(!body_in_area(c, &s(6.3, 5.3, 0.4)) && body_in_area(c, &s(6.3, 5.3, 0.45)));
    // A column crossing the area's top from far above: its lowest point counts.
    let col = BodyShape::Column { a: [3.0, 9.0, 0.0], b: [3.0, 4.5, 0.0], r: 0.1 };
    assert!(body_in_area(c, &col));
    let col = BodyShape::Column { a: [7.0, 0.0, -3.0], b: [7.0, 0.0, 3.0], r: 0.95 };
    assert!(!body_in_area(c, &col));
    let col = BodyShape::Column { a: [7.0, 0.0, -3.0], b: [7.0, 0.0, 3.0], r: 1.05 };
    assert!(body_in_area(c, &col));
    // Nearest first from the target, the first 30 only; ties keep the earlier.
    let e = |id: u32, x: f32| (id, [x, 0.0, 0.0]);
    assert_eq!(thunder_order(c, [e(1, 5.0), e(2, -1.0), e(3, 3.0), e(4, 1.0)]), vec![2, 4, 3, 1]);
    let many: Vec<(u32, [f32; 3])> = (0..40).map(|i| (i, [40.0 - i as f32, 0.0, 0.0])).collect();
    let o = thunder_order(c, many);
    assert_eq!(o.len(), 30);
    assert_eq!((o[0], o[29]), (29, 0));
}

#[test]
fn thunder_strikes_one_every_8_frames_nearest_first_once_each() {
    let mut s = Strikes::new(vec![10, 11, 12, 13]);
    let outside = 11;
    let mut at = Vec::new();
    for frame in 0..60 {
        if let Some(id) = s.run(1.0, |id| id != outside) {
            at.push((frame, id));
        }
    }
    // The release's run strikes the nearest; one outside the area is passed over at once.
    assert_eq!(at, vec![(0, 10), (8, 12), (16, 13)]);
    assert!(s.done());
    assert_eq!(s.struck, 3);
    // Nothing inside: done in the first run, no bolt.
    let mut s = Strikes::new(vec![1, 2]);
    assert_eq!(s.run(1.0, |_| false), None);
    assert!(s.done());
}
