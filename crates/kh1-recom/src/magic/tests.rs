//! KH1's Fire numbers in Re:CoM's row and shot formats; the spell damage. The cast's frames are
//! tested on the movement tests' fake Re:CoM (`movement/tests.rs`, "M7 stage 7.4").

use super::*;

#[test]
fn fire_rows_keep_recoms_clip_and_effect_set_and_take_kh1s_ball() {
    let t = kh1_magic_table(&recom_rows());
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
    // Aero comes with its stage.
    assert!(t[18..].iter().all(|r| r[0] == 0));
    assert_eq!(table_index(spell::AERO, 3), Some(20));
    assert_eq!(table_index(spell::FIRE, 0), None);
    assert!(castable(spell::FIRE) && castable(spell::BLIZZARD) && castable(spell::THUNDER) && castable(spell::CURE) && castable(spell::GRAVITY) && castable(spell::STOP) && !castable(spell::AERO));
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
    // KH1's root helper (entry 57): back 1.5 cm a frame to −15 cm at frame 10, then 0.1 cm a frame
    // forward to −10 cm at its end (entry 82: back to −42.41 cm at 10, then still). Checked (files).
    let z = |r, t| m.info(r).unwrap().root_at(t).z;
    for (t, want) in [(0.0, 0.0), (5.0, -7.5), (10.0, -15.0), (20.0, -14.0), (46.0, -11.4), (60.0, -10.0)] {
        assert!((z(0x3D, t) - want).abs() < 1e-4, "0x3D at {t}: {}", z(0x3D, t));
    }
    for (t, want) in [(5.0, -21.2048), (10.0, -42.4096), (52.0, -42.4096)] {
        assert!((z(0x8A, t) - want).abs() < 1e-3, "0x8A at {t}: {}", z(0x8A, t));
    }
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
    let t = kh1_magic_table(&recom_rows());
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
    assert!(is_cast_request(0x37) && is_cast_request(0x84) && !is_cast_request(0x3C));
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
    let t = kh1_magic_table(&recom_rows());
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

#[test]
fn cure_rows_and_numbers_are_kh1s_cast_with_recoms_heal() {
    let t = kh1_magic_table(&recom_rows());
    for lv in 1..=3u8 {
        let i = lv as usize - 1;
        let r = &t[table_index(spell::CURE, lv).unwrap()];
        let base = &RECOM_CURE_ROWS[i];
        // Re:CoM's clip 0x39 (Thunder's p001_064) and its Cure / Cura / Curaga sets 15 / 16 / 17.
        assert_eq!(&r[..3], &base[..3]);
        assert_eq!(r[2], 15 + lv - 1);
        assert_eq!((r[3], r[0xF] & FLAG_KH1), (0, FLAG_KH1));
        assert_eq!(recom_base_row(spell::CURE, lv), Some([0x25, 0x36, 0x37][i]));
        // Re:CoM's heal (decision 34): 500 / 1500 / 3000 ÷ 10.
        assert_eq!(cure::heal(lv), [50, 150, 300][i]);
    }
    // KH1: one point of MP, the release 36 frames after the command, no damage record.
    assert_eq!((cost(spell::CURE), release_wait(spell::CURE)), (100, 36.0));
    assert_eq!(spell_attack(spell::CURE, 1), None);
    assert_eq!((cast_req::first(spell::CURE, false), cast_req::first(spell::CURE, true)), (0x39, 0x86));
    assert_eq!((cast_req::release(spell::CURE, false), cast_req::release(spell::CURE, true)), (0x39, 0x86));
    assert!(is_cast_request(0x39) && is_cast_request(0x86));
    let mut m = MotionSet::default();
    add_motions(&mut m);
    // Entries 56 / 81: 68 frames, no free key (locked to the end).
    for r in [0x39, 0x86] {
        assert_eq!(m.info(r).unwrap().length, 68.0);
        assert_eq!(m.info(r).unwrap().first(FlagKind::Free), None);
    }
    // The thread heals only a live target (Sora with HP left, not dying).
    assert!(cure::target_ok(1, false) && !cure::target_ok(0, false) && !cure::target_ok(40, true));
}

#[test]
fn gravity_rows_numbers_and_fall_are_kh1s() {
    let t = kh1_magic_table(&recom_rows());
    for lv in 1..=3u8 {
        let i = lv as usize - 1;
        let r = &t[table_index(spell::GRAVITY, lv).unwrap()];
        let base = &RECOM_GRAVITY_ROWS[i];
        // Re:CoM's clip 0x3A (Blizzard's p001_065) and its Gravity sets 18 / 19 / 20.
        assert_eq!((&r[..3], r[5], r[0xD]), (&base[..3], base[5], base[0xD]));
        assert_eq!(r[2], 18 + lv - 1);
        // Re:CoM's flinch, not its kind 0xD.
        assert_eq!((r[3], r[0xB], r[0xF] & FLAG_KH1), (0, 1, FLAG_KH1));
        assert_eq!(recom_base_row(spell::GRAVITY, lv), Some([0x26, 0x38, 0x39][i]));
        let a = gravity_attack(lv);
        assert_eq!((a.power, a.stat_source, a.element, a.kind), ([40, 55, 70][i], 5, 8, 8));
    }
    assert_eq!((cost(spell::GRAVITY), release_wait(spell::GRAVITY)), (100, 28.0));
    assert_eq!((cast_req::first(spell::GRAVITY, false), cast_req::first(spell::GRAVITY, true)), (0x3A, 0x87));
    let mut m = MotionSet::default();
    add_motions(&mut m);
    assert_eq!((m.info(0x3A).unwrap().length, m.info(0x87).unwrap().length), (86.0, 78.0));
    assert_eq!((m.info(0x3A).unwrap().first(FlagKind::Free), m.info(0x87).unwrap().first(FlagKind::Free)), (Some(68.0), Some(68.0)));
    // The impact radius 170 × S cm and the scale's growth.
    assert_eq!([1u8, 2, 3].map(gravity::impact_radius_m), [1.7, 2.55, 2.72]);
    assert!((gravity::scale_at(2, 35.0) - (0.01 + 1.49 * 0.5)).abs() < 1e-6);
    assert_eq!(gravity::scale_at(3, 99.0), 1.6);
}

#[test]
fn gravity_damage_is_kh1s_percent_of_current_hp() {
    // Gravity + 5 × max MP 3 = 55 % of the current HP × resist[8].
    assert_eq!(gravity_damage(1, 3, 200, 1.0, 0), 110);
    // An LV2 Heartless (80 %): 44 % → 88.
    assert_eq!(gravity_damage(1, 3, 200, 0.8, 0), 88);
    // A Re:CoM-only boss (decision 34, 2 %) at 1000 HP: 1000 × 0.55 × 0.02 = 11.
    assert_eq!(gravity_damage(1, 3, 1000, 0.02, 2), 11);
    // Under the floor: the floor (2), or 1 when the floor is under 2.
    assert_eq!(gravity_damage(1, 3, 50, 0.02, 2), 2);
    assert_eq!(gravity_damage(1, 3, 50, 0.02, 0), 1);
    // Graviga (70 + 40 = 110 % at max MP 8) on a twin: never more than ... the current HP × 110 %.
    assert_eq!(gravity_damage(3, 8, 100, 0.04, 2), 4);
    // Immune record (Sephiroth 0 %): still the floor.
    assert_eq!(gravity_damage(2, 3, 900, 0.0, 4), 4);
}

#[test]
fn gravity_falls_its_frames_then_strikes_once() {
    for (lv, n) in [(1u8, 60), (2, 70), (3, 60)] {
        let mut f = Fall::new(lv);
        let mut runs = 0;
        // The release run counted the first frame; the impact is the run N frames after it.
        while !f.run(1.0) {
            runs += 1;
            assert!(runs < 200);
        }
        assert_eq!(runs, n - 1, "level {lv}: N − 1 more frames fall, the next run strikes");
        assert_eq!(f.fallen(), n as f32);
    }
    // The impact sphere: a body that touches 1.7 m around the centre counts.
    let c = [0.0, 1.0, 0.0];
    let a = Area::Sphere { radius: 1.7 };
    assert!(body_touches(c, a, &BodyShape::Sphere { c: [2.0, 1.0, 0.0], r: 0.31 }));
    assert!(!body_touches(c, a, &BodyShape::Sphere { c: [2.0, 1.0, 0.0], r: 0.29 }));
    assert!(!body_touches(c, a, &BodyShape::Sphere { c: [0.0, 3.0, 0.0], r: 0.2 }), "a sphere, not a column");
    assert!(body_touches(c, a, &BodyShape::Column { a: [1.0, -5.0, 1.0], b: [1.0, 5.0, 1.0], r: 0.1 }));
}

#[test]
fn stop_rows_area_and_duration_are_kh1s() {
    let t = kh1_magic_table(&recom_rows());
    for lv in 1..=3u8 {
        let i = lv as usize - 1;
        let r = &t[table_index(spell::STOP, lv).unwrap()];
        let base = &RECOM_STOP_ROWS[i];
        // Re:CoM's clip 0x39, sets 21 / 22 / 23, and its stop kind 8 (Re:CoM's stop state).
        assert_eq!((&r[..3], r[4], r[0xC], r[0xE]), (&base[..3], base[4], base[0xC], base[0xE]));
        assert_eq!((r[2], r[0xB] as u32), (21 + lv - 1, RECOM_STOP_KIND));
        assert_eq!((r[3], r[0xF] & FLAG_KH1), (0, FLAG_KH1));
        assert_eq!(recom_base_row(spell::STOP, lv), Some([0x27, 0x3A, 0x3B][i]));
        assert_eq!(stop::area(lv), Area::Cylinder { radius: [2.5, 3.5, 5.0][i], half: 3.0 });
    }
    assert_eq!((cost(spell::STOP), release_wait(spell::STOP)), (200, 28.0));
    assert_eq!((cast_req::first(spell::STOP, false), cast_req::first(spell::STOP, true)), (0x3B, 0x88));
    // (2 + max MP) × 60 × resist[9] frames, the same at every level; 0 = immune.
    assert_eq!(stop::frames(3, 1.0), Some(300.0));
    assert_eq!(stop::frames(3, 0.3), Some(90.0));
    assert_eq!(stop::frames(8, 1.0), Some(600.0));
    assert_eq!(stop::frames(5, 0.0), None);
    assert_eq!(stop::ticks(3, 1.0), Some(15000));
    assert_eq!(stop::ticks(8, 1.0), Some(30000), "KH1's longest fits Re:CoM's i16 hit value");
    assert_eq!(stop::ticks(3, 0.2), Some(3000));
    // The damage path returns the power for element 9 (the points of the duration).
    let mut rng = Kh1Rand::new(1);
    assert_eq!(spell_damage(&stop_attack(2), 3, &DefenderStats::default(), &mut rng), 5);
}

#[test]
fn stop_strikes_one_every_4_frames_nearest_first() {
    let order = thunder_order([0.0; 3], [(1, [3.0, 0.0, 0.0]), (2, [1.0, 0.0, 0.0]), (3, [2.0, 0.0, 0.0]), (4, [9.0, 0.0, 0.0])]);
    assert_eq!(order, vec![2, 3, 1, 4]);
    let mut s = Strikes::every(order, stop::INTERVAL);
    let a = stop::area(1);
    let inside = |id: u32| {
        let x = [0.0, 3.0, 1.0, 2.0, 9.0][id as usize];
        body_touches([0.0; 3], a, &BodyShape::Sphere { c: [x, 0.0, 0.0], r: 0.3 })
    };
    let mut at = Vec::new();
    for frame in 0..30 {
        if let Some(id) = s.run(1.0, inside) {
            at.push((frame, id));
        }
    }
    // 2.5 m and a 0.3 m body: 1 and 2 m inside; 3 m (0.2 m short) and 9 m passed over at once.
    assert_eq!(at, vec![(0, 2), (4, 3)]);
    assert!(s.done());
}
