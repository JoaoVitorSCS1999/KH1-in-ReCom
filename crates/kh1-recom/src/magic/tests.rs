//! KH1's Fire numbers in Re:CoM's row and shot formats; the spell damage. The cast's frames are
//! tested on the movement tests' fake Re:CoM (`movement/tests.rs`, "M7 stage 7.4").

use super::*;

#[test]
fn fire_rows_keep_recoms_clip_and_effect_set_and_take_kh1s_ball() {
    let t = kh1_magic_table(&RECOM_FIRE_ROWS);
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
    // Blizzard … Aero come with their stages.
    assert!(t[3..].iter().all(|r| r[0] == 0));
    assert_eq!(table_index(spell::AERO, 3), Some(20));
    assert_eq!(table_index(spell::FIRE, 0), None);
    assert!(castable(spell::FIRE) && !castable(spell::BLIZZARD));
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
