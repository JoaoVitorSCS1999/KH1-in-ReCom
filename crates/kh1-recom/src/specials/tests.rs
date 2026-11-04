use super::*;

fn t(gap: f32, above: f32) -> Option<OfferTarget> {
    Some(OfferTarget { gap, above })
}

const SONIC: [bool; 5] = [true, false, false, false, false];

/// `0x14028D0F0`: Sonic Blade in range (gap below 600) and not too high, scored by the MP.
#[test]
fn the_offer_is_kh1s() {
    let pays = &mut |c: i32| c <= 200;
    assert_eq!(offer(&SONIC, t(599.0, 0.0), pays), Some((cmd::SONIC_BLADE, false)));
    assert_eq!(offer(&SONIC, t(600.0, 0.0), pays), None, "range strict");
    assert_eq!(offer(&SONIC, t(100.0, 160.5), pays), None, "more than 160 above");
    assert_eq!(offer(&SONIC, t(100.0, 160.0), pays), Some((cmd::SONIC_BLADE, false)));
    assert_eq!(offer(&SONIC, t(100.0, -500.0), pays), Some((cmd::SONIC_BLADE, false)), "below is fine");
    assert_eq!(offer(&SONIC, t(100.0, 0.0), &mut |_| false), Some((cmd::SONIC_BLADE, true)), "MP short: greyed");
    assert_eq!(offer(&SONIC, None, pays), None, "no target");
    assert_eq!(offer(&[false; 5], t(1.0, 0.0), pays), None);
    // KH1's order: Trinity, Ragnarok, Ars Arcanum, Sonic Blade, Strike Raid; the best score wins,
    // the first among equals.
    let all = [true; 5];
    assert_eq!(offer(&all, t(100.0, 0.0), &mut |_| true).map(|o| o.0), Some(cmd::TRINITY));
    assert_eq!(offer(&all, t(700.0, 0.0), &mut |_| true).map(|o| o.0), Some(cmd::TRINITY), "Trinity 1200");
    assert_eq!(offer(&[true, true, true, true, false], t(400.0, 200.0), &mut |_| true).map(|o| o.0), Some(cmd::STRIKE_RAID), "Ragnarok last: Strike Raid first");
    // M12 step 7: KH1's 'Z' (only a target 160 or more above Sora) is off: the owner's choice
    // (2026-11-03, "Ragnarok offered against any target"). Range 600 and the MP stay KH1's.
    assert!(!RAGNAROK_HEIGHT_RULE);
    let rag = [false, false, false, true, false];
    assert_eq!(offer(&rag, t(400.0, 159.9), &mut |_| true), Some((cmd::RAGNAROK, false)), "less than 160 above: offered");
    assert_eq!(offer(&rag, t(400.0, 160.0), &mut |_| true), Some((cmd::RAGNAROK, false)));
    assert_eq!(offer(&rag, t(400.0, 0.0), &mut |_| true), Some((cmd::RAGNAROK, false)), "a target on the ground: offered");
    assert_eq!(offer(&rag, t(400.0, -300.0), &mut |_| true), Some((cmd::RAGNAROK, false)), "below Sora: offered");
    assert_eq!(offer(&rag, t(400.0, 0.0), &mut |c| c < 300), Some((cmd::RAGNAROK, true)), "MP short (3): greyed");
    assert_eq!(offer(&rag, t(600.0, 0.0), &mut |_| true), None, "range 600 strict");
    // The owner's order ("Ragnarok last"): Ars Arcanum, Sonic Blade and Strike Raid before Ragnarok.
    assert_eq!(ORDER, [4, 1, 0, 2, 3]);
    assert_eq!(KH1_ORDER, [4, 3, 1, 0, 2]);
    let no_tri = [true, true, true, true, false];
    assert_eq!(offer(&no_tri, t(100.0, 0.0), &mut |_| true).map(|o| o.0), Some(cmd::ARS_ARCANUM), "Ars Arcanum before Ragnarok");
    assert_eq!(offer(&no_tri, t(400.0, 0.0), &mut |_| true).map(|o| o.0), Some(cmd::SONIC_BLADE), "Ars Arcanum out of range (300): Sonic Blade");
    assert_eq!(offer(&[false, true, true, true, false], t(400.0, 0.0), &mut |_| true).map(|o| o.0), Some(cmd::STRIKE_RAID), "Strike Raid before Ragnarok");
    assert_eq!(offer(&[false, true, false, true, false], t(400.0, 0.0), &mut |_| true).map(|o| o.0), Some(cmd::RAGNAROK), "none of the others offered: Ragnarok");
    assert_eq!(offer(&[false, false, true, true, false], t(400.0, 0.0), &mut |c| c < 300).map(|o| o.0), Some(cmd::STRIKE_RAID), "both greyed: Strike Raid, Ragnarok last among equals");
    assert_eq!(offer(&no_tri, t(700.0, 0.0), &mut |_| true).map(|o| o.0), Some(cmd::STRIKE_RAID), "beyond 600: Strike Raid");
    // Sonic Blade's own height rule stays KH1's.
    assert_eq!(score(SONIC_BLADE, &OfferTarget { gap: 100.0, above: 200.0 }, &mut |_| true), 0);
    assert_eq!(offer(&[true, false, true, false, false], t(100.0, 0.0), &mut |c| c < 200).map(|o| o.0), Some(cmd::SONIC_BLADE), "both greyed: the first in KH1's order");
    assert_eq!(offer(&[true, false, true, false, false], t(800.0, 0.0), &mut |_| true), Some((cmd::STRIKE_RAID, false)), "Strike Raid 900");
}

#[test]
fn offer_target_is_the_horizontal_gap_minus_the_radius() {
    let tp = TargetPart { owner: 1, centre: Vec3::new(300.0, -200.0, 400.0), radius: 30.0 };
    let o = OfferTarget::new(Vec3::ZERO, &tp);
    assert!((o.gap - 470.0).abs() < 1e-3);
    assert_eq!(o.above, 200.0, "KH1 y is down: the point is 200 above");
    let close = TargetPart { radius: 600.0, ..tp };
    assert_eq!(OfferTarget::new(Vec3::ZERO, &close).gap, 0.0);
}

#[test]
fn follow_ups_and_numbers() {
    assert_eq!([0, 1, 5, 6, 7].map(follow_cmd), [cmd::RAVE, cmd::RAVE, cmd::RAVE, cmd::BLAST, cmd::BLAST]);
    assert_eq!(ROWS, 7);
    assert_eq!(COST[SONIC_BLADE], 200);
    assert_eq!(TABLE[SONIC_BLADE], (11, 0x4B));
    assert!(cmd::is_ours(0x4B) && cmd::is_ours(0x4D) && cmd::is_ours(0x4E) && cmd::is_ours(0x57) && cmd::is_ours(0x5E));
    assert!(cmd::is_ours(0x5A) && cmd::is_ours(0x5B));
    assert!(!cmd::is_ours(0x52), "Trinity not built");
    assert_eq!([0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5E, 0x5F, 0x60].map(|c| cmd::name(c).unwrap()), [331, 332, 333, 334, 335, 338, 339, 340]);
    assert_eq!([0x4B, 0x4D, 0x4E].map(|c| cmd::name(c).unwrap()), [319, 321, 322]);
    // The special rows play the xl set's motions under our requests.
    let m = kh1_core::Moveset::kh1_defaults();
    assert_eq!(row_request(&m, 0x1B), Some(xl::request(xl::DASH)));
    assert_eq!(row_request(&m, 0x21), Some(xl::request(xl::FINISHER)));
    assert_eq!(row_request(&m, 13), None);
}

/// Base Re:CoM's events: the start, the dashes and the finisher are motions 0, 1, 2 of the pack
/// 11009 (sleight 9's group 0x3F1 + 10000).
#[test]
fn recom_events_follow_the_sleight() {
    assert_eq!(recom_events::PACK, 11_009);
    assert_eq!([xl::WINDUP, xl::DASH, xl::FINISHER].map(recom_events::motion_of), [Some(0), Some(1), Some(2)]);
    assert_eq!(recom_events::motion_of(3), None);
}

/// Sonic Blade's rows in Re:CoM's layout: the xl motions' windows one frame earlier, Re:CoM's
/// stun kind for KH1's element 10 (as Stun Impact), flag 0x80; the clip row given.
#[test]
fn special_rows_take_the_xl_windows() {
    use crate::attack::{add_special_rows, kh1_table, special_rows_ready, RECOM_ROWS_1_7, RECOM_STUN_KIND};
    let m = kh1_core::Moveset::kh1_defaults();
    let mut t = kh1_table(&RECOM_ROWS_1_7, &m, &|_| None);
    assert!(!special_rows_ready(&t, SONIC_BLADE) && !special_rows_ready(&t, STRIKE_RAID));
    add_special_rows(&mut t, &RECOM_ROWS_1_7, &m, &|e| Some(0x50 + e as u32));
    assert!(special_rows_ready(&t, SONIC_BLADE) && special_rows_ready(&t, STRIKE_RAID));
    assert!(special_rows_ready(&t, ARS_ARCANUM));
    assert!(special_rows_ready(&t, RAGNAROK));
    assert!(!special_rows_ready(&t, 4), "Trinity not built");
    // M12 step 7: Ragnarok's five rising hits on rows 0x36..=0x3A (xl 27..=31).
    assert_eq!((t[0x36][0], t[0x3A][0]), (0x50 + 27, 0x50 + 31));
    // M12 step 7: Ars Arcanum's rows: KH1's windows one frame early, the seventh's combo window
    // 22..52 (the follow-ups), the finisher on Re:CoM's finisher layout.
    let a7 = t[0x28];
    assert_eq!((a7[0], a7[1], a7[3], a7[4], a7[5]), (0x50 + 10, 22, 52, 13, 21));
    assert_eq!((t[0x22][0], t[0x22][4], t[0x22][5], t[0x22][1]), (0x54, 7, 15, 0));
    assert_eq!(t[0x2E][0], 0x50 + 16);
    let d = t[0x1B];
    assert_eq!((d[0], d[1], d[3], d[4], d[5], d[0xB], d[0xF]), (0x51, 10, 40, 3, 21, RECOM_STUN_KIND, 0x80));
    let f = t[0x21];
    assert_eq!((f[0], f[1], f[3], f[4], f[5], f[0xB]), (0x52, 0, 0, 15, 33, RECOM_STUN_KIND));
    assert_eq!(t[0x1B], t[0x20], "every dash the same");
    // Without the clip, nothing.
    let mut u = kh1_table(&RECOM_ROWS_1_7, &m, &|_| None);
    add_special_rows(&mut u, &RECOM_ROWS_1_7, &m, &|e| (e != xl::FINISHER).then_some(0x50));
    assert!(!special_rows_ready(&u, SONIC_BLADE));
}

/// M12 step 7: Ragnarok's numbers (`0x14039F330`, `0x14039FF70`, `0x1406A0C00`).
#[test]
fn ragnaroks_numbers_are_kh1s() {
    use ragnarok::*;
    assert_eq!(HOP_SPEED, [8.0, 4.0, 4.0, 4.0, 8.0]);
    assert!(!offered(9.9) && offered(10.0) && offered(69.9) && !offered(70.0));
    assert_eq!(fire_of(10.0), Fire { shots: 10, hop: 10.0, back: 0.0, big: false });
    assert_eq!(fire_of(40.0), Fire { shots: 17, hop: 20.0, back: -4.0, big: false });
    assert_eq!(fire_of(55.0), Fire { shots: 25, hop: 30.0, back: -8.0, big: true });
    assert_eq!(fire_of(60.0), Fire { shots: 10, hop: 10.0, back: 0.0, big: false });
    assert_eq!((shot_life(0.0), shot_life(1.0), shot_bend(0.0), shot_bend(1.0)), (180.0, 240.0, 0.4, 0.8));
    // The ring: from up (KH1's −y), 2π / n apart, all unit, about the facing axis (no z part when
    // facing +z).
    let r = ring(Vec3::new(0.0, -1.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 4);
    assert_eq!(r.len(), 4);
    assert!((r[0] - Vec3::new(0.0, -1.0, 0.0)).length() < 1e-6 && (r[1] - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-6);
    assert!(r.iter().all(|d| (d.length() - 1.0).abs() < 1e-6 && d.z.abs() < 1e-6));
    let rec = shot_record();
    assert_eq!((rec.stat_source, rec.power, rec.kind, rec.clash, rec.radius), (3, 20, 3, 6, 50.0));
}

/// A shot turns at most 5° a frame toward its bent aim and flies 12 a frame; it closes on a target.
#[test]
fn ragnarok_shots_home_as_kh1s() {
    use ragnarok::*;
    let (p, d) = shot_step(Vec3::ZERO, Vec3::new(0.0, -1.0, 0.0), Some(Vec3::new(0.0, 0.0, 500.0)), 0.4, 1.0);
    assert!((d.length() - 1.0).abs() < 1e-5);
    assert!((d.dot(Vec3::new(0.0, -1.0, 0.0)).acos() - SHOT_TURN).abs() < 1e-4, "5° this frame");
    assert!(((p - Vec3::ZERO).length() - SHOT_SPEED).abs() < 1e-4);
    // Many frames on: it reaches the target's neighbourhood.
    let (mut p, mut d) = (Vec3::ZERO, Vec3::new(0.0, -1.0, 0.0));
    let goal = Vec3::new(0.0, 0.0, 500.0);
    let mut best = f32::MAX;
    for _ in 0..200 {
        (p, d) = shot_step(p, d, Some(goal), 0.4, 1.0);
        best = best.min((goal - p).length());
    }
    assert!(best < 60.0, "closest {best}");
}
