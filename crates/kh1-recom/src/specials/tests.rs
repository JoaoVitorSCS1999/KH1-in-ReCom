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
    assert_eq!(offer(&[true, true, true, true, false], t(400.0, 0.0), &mut |_| true).map(|o| o.0), Some(cmd::RAGNAROK));
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
    assert!(cmd::is_ours(0x4B) && cmd::is_ours(0x4D) && cmd::is_ours(0x4E) && !cmd::is_ours(0x57));
    assert_eq!([0x4B, 0x4D, 0x4E].map(|c| cmd::name(c).unwrap()), [319, 321, 322]);
    // The special rows play the xl set's motions under our requests.
    let m = kh1_core::Moveset::kh1_defaults();
    assert_eq!(row_request(&m, 0x1B), Some(xl::request(xl::DASH)));
    assert_eq!(row_request(&m, 0x21), Some(xl::request(xl::FINISHER)));
    assert_eq!(row_request(&m, 13), None);
}

/// The Field exe's stub: `mov rcx,rbx; call 0x140218950; jmp 0x1401FA599` at the cave.
#[test]
fn field_case_stub() {
    let s = field_case::stub();
    assert_eq!(&s[..4], &[0x48, 0x8B, 0xCB, 0xE8]);
    let call = i32::from_le_bytes(s[4..8].try_into().unwrap()) as i64 + field_case::CAVE_RVA as i64 + 8;
    assert_eq!(call as u32, field_case::EXE_STOCK_RVA);
    assert_eq!(s[8], 0xE9);
    let jmp = i32::from_le_bytes(s[9..13].try_into().unwrap()) as i64 + field_case::CAVE_RVA as i64 + 13;
    assert_eq!(jmp as u32, field_case::DEFAULT_RVA);
    // The jump table's entry for action 0x16 (index action − 1 from 0x1401FA5F4).
    assert_eq!(field_case::TABLE_ENTRY_RVA, 0x001F_A5F4 + 4 * (RECOM_ACTION - 1));
}

/// Sonic Blade's rows in Re:CoM's layout: the xl motions' windows one frame earlier, Re:CoM's
/// stun kind for KH1's element 10 (as Stun Impact), flag 0x80; the clip row given.
#[test]
fn special_rows_take_the_xl_windows() {
    use crate::attack::{add_special_rows, kh1_table, special_rows_ready, RECOM_ROWS_1_7, RECOM_STUN_KIND};
    let m = kh1_core::Moveset::kh1_defaults();
    let mut t = kh1_table(&RECOM_ROWS_1_7, &m, &|_| None);
    assert!(!special_rows_ready(&t));
    add_special_rows(&mut t, &RECOM_ROWS_1_7, &m, &|e| Some(0x50 + e as u32));
    assert!(special_rows_ready(&t));
    let d = t[0x1B];
    assert_eq!((d[0], d[1], d[3], d[4], d[5], d[0xB], d[0xF]), (0x51, 10, 40, 3, 21, RECOM_STUN_KIND, 0x80));
    let f = t[0x21];
    assert_eq!((f[0], f[1], f[3], f[4], f[5], f[0xB]), (0x52, 0, 0, 15, 33, RECOM_STUN_KIND));
    assert_eq!(t[0x1B], t[0x20], "every dash the same");
    // Without the clip, nothing.
    let mut u = kh1_table(&RECOM_ROWS_1_7, &m, &|_| None);
    add_special_rows(&mut u, &RECOM_ROWS_1_7, &m, &|e| (e != xl::FINISHER).then_some(0x50));
    assert!(!special_rows_ready(&u));
}
