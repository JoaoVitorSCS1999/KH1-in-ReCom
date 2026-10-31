use super::*;

#[test]
fn kh1_rows_keep_recoms_clip_and_take_kh1s_windows() {
    let m = Moveset::kh1_defaults();
    let t = kh1_table(&RECOM_ROWS_1_7, &m, Some(0x48));
    // Row 13 (motion 200): Re:CoM row 1's action 0x30 and effect 1; KH1 hit 14–24 held one frame
    // earlier for Re:CoM's exe-time test; combo window 22–40; kind 1 (flinch); only flag 0x80.
    let r = t[13];
    assert_eq!(r[0], 0x30);
    assert_eq!(r[2], 1);
    assert_eq!((r[4], r[5]), (13, 23));
    assert_eq!((r[1], r[3]), (22, 40));
    assert_eq!(&r[6..10], &[0, 0, 0, 0], "no homing");
    assert_eq!(r[0xB], 1);
    assert_eq!((r[0xC], r[0xD]), (0, 0));
    assert_eq!(r[0xF], FLAG_KH1);
    // The finisher (row 8, motion 203): hit 26–38, no window, kind 2 (blow-away), clip 0x33.
    let f = t[8];
    assert_eq!((f[0], f[4], f[5], f[1], f[3], f[0xB]), (0x33, 25, 37, 0, 0, 2));
    assert_eq!(row_window(&f), None);
    assert_eq!(row_window(&t[11]), Some((26.0, 42.0)));
    // Air rows: 18 → Re:CoM's id 6 clip (0x35), 17 → id 5 (0x34), 16 → id 7 (0x36).
    assert_eq!((t[18][0], t[17][0], t[16][0]), (0x35, 0x34, 0x36));
    assert_eq!((t[18][4], t[18][5]), (5, 19));
    // Rows Re:CoM can't play stay empty.
    assert_eq!(t[3], [0u8; 16]);
    // Counterattack (row 0x3E, motion 213): the appended clip row, hit 8–24 a frame earlier,
    // window 18–56, flinch; none without the clip.
    let c = t[COUNTERATTACK_ROW];
    assert_eq!((c[0], c[4], c[5], c[1], c[3], c[0xB]), (0x48, 7, 23, 18, 56, 1));
    assert_eq!(kh1_table(&RECOM_ROWS_1_7, &m, None)[COUNTERATTACK_ROW], [0u8; 16]);
    // Re:CoM's own rows never carry flag 0x80 (the gate's KH1 test leaves them alone).
    assert!(RECOM_ROWS_1_7.iter().all(|r| r[0xF] & FLAG_KH1 == 0 && row_window(r).is_none()));
}

#[test]
fn row_map_follows_the_motions() {
    let m = Moveset::kh1_defaults();
    for &(r, id) in ROW_MAP.iter() {
        let req = m.row_request(r);
        let clip = 0x30 + id - 1;
        assert_eq!(RECOM_ROWS_1_7[id as usize - 1][0] as u32, clip);
        // Re:CoM ids 1..7 play KH1 motions 200, 201, 202, 203, 205, 204, 206.
        let expect = [200, 201, 202, 203, 205, 204, 206][id as usize - 1];
        assert_eq!(req, expect, "row {r}");
    }
    assert_eq!(recom_id(13), Some(1));
    assert_eq!(recom_id(3), None);
    assert!(swing_root(&m, 200) && swing_root(&m, 203));
    assert!(!swing_root(&m, 204) && !swing_root(&m, 206));
    assert!(is_swing_request(206) && !is_swing_request(212));
    assert_eq!((recom_kind(3), recom_kind(5)), (1, 2));
}

#[test]
fn blade_is_two_spheres_on_the_kingdom_key_plus_the_sweep() {
    let m = Moveset::kh1_defaults();
    let w = WeaponData::kingdom_key();
    let live = live_attack(&m, 13, &w).unwrap();
    // Radius 0 + 1.0 × 40 = 40 units (0.40 m), fractions 1.0 / 0.0 × 0.8.
    assert_eq!(live.radius, 40.0);
    // Re:CoM metres: wep0 at the origin, k9 0.2111 m along −x (any direction works).
    let w2 = Vec3::ZERO;
    let w3 = Vec3::new(-0.2111, 0.0, 0.0);
    let (a, b) = blade_points(w2, w3, &live, &w);
    assert!((a.x - 0.2111 * 2.96).abs() < 1e-5, "A at t = −2.96: {a:?}");
    assert_eq!(b, w2);
    let r = live.radius * crate::units::METRES_PER_UNIT;
    // Without a sweep: spheres at A and A + 0.4 toward B (0.625 m of blade).
    let s = blade_shapes(a, b, r, false, None, Vec3::ZERO);
    assert_eq!(s.len(), 2);
    assert!((s[1].0.x - (a.x - 0.4)).abs() < 1e-5);
    // The first tested frame sweeps from the owner's centre; the next from the last A.
    let owner = Vec3::new(0.0, 0.0, -1.0);
    let first = blade_shapes(a, b, r, true, None, owner);
    assert!(first.len() > 2);
    let next = blade_shapes(a, b, r, true, Some(a), owner);
    assert_eq!(next.len(), 3, "a still blade sweeps one sphere at A");
}

#[test]
fn blade_track_moves_on_once_per_pose() {
    let mut t = BladeTrack::default();
    let a1 = Vec3::new(1.0, 0.0, 0.0);
    let a2 = Vec3::new(1.0, 0.1, 0.0);
    assert_eq!(t.frame(5, a1), None, "a new swing seeds from the owner");
    assert_eq!(t.frame(5, a1), None, "the same frame tested twice");
    assert_eq!(t.frame(5, a2), Some(a1));
    assert_eq!(t.frame(5, a2), Some(a1));
    assert_eq!(t.frame(6, a2), None, "the next swing seeds again");
}

fn twin(defence: i32) -> EnemyStats {
    EnemyStats { hp: 12, strength: 8, defence, min_damage: 0, max_damage: 0, level: 1, exp: 1, resist: [100; 16] }
}

#[test]
fn kh1_damage_uses_the_twins_defence_curve() {
    let m = Moveset::kh1_defaults();
    let mut rng = kh1_core::Kh1Rand::new(1);
    // Defence 8, Strength 10 swing → curve[2] = 2.
    assert_eq!(kh1_damage(&m, 13, 10, &WeaponData::kingdom_key(), &twin(8), &mut rng).unwrap().damage, 2);
    // The finisher adds 4 power → curve[6] = 6 (or a critical 10 + 4 → curve[10] = 12).
    for _ in 0..20 {
        let d = kh1_damage(&m, 8, 10, &WeaponData::kingdom_key(), &twin(8), &mut rng).unwrap();
        assert_eq!(d.damage, if d.critical { 12 } else { 6 });
    }
    // A cap of 1 per hit (KH1's record +0x10) holds.
    let mut capped = twin(0);
    capped.max_damage = 1;
    assert_eq!(kh1_damage(&m, 8, 40, &WeaponData::kingdom_key(), &capped, &mut rng).unwrap().damage, 1);
}

#[test]
fn stats_json_reads_the_asset_steps_format() {
    let rec = r#"{"hp": 12, "strength": 8, "defence": 8, "floor": 2, "cap": 300, "level": 6, "exp": 1, "resist": [100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100]}"#;
    let worlds = vec![rec; 13].join(", ");
    let strength = (1..=100).map(|l| (6 + l).to_string()).collect::<Vec<_>>().join(", ");
    let t = format!(r#"{{"format": "kh1-recom-assets stats 2", "sora_strength": [{strength}], "enemies": [{{"class": "CEnemyShadow", "lock_radius": 30, "lock_radii": [30, 30], "worlds": [{worlds}]}}]}}"#);
    let s = parse_stats(&t).unwrap();
    assert_eq!(s.lock_radius("CEnemyShadow"), Some(30.0));
    assert_eq!(s.lock_radius("CEnemySoldier"), None);
    // Format 1 (no lock radius) still reads; the radius is then unknown.
    let old = parse_stats(&t.replace(r#""lock_radius": 30, "#, "")).unwrap();
    assert_eq!(old.lock_radius("CEnemyShadow"), None);
    // Format 3: base Strength plus the equipped Keyblade's, and the Keyblade rows.
    assert_eq!(s.strength_with(1, &WeaponData::kingdom_key()), Some(7), "format 2: the Kingdom Key's table");
    let base = (1..=100).map(|l| (3 + l).to_string()).collect::<Vec<_>>().join(", ");
    let kb = r#"{"card": 15, "name": "Ultima Weapon", "source": "kh1", "kh1_item": 100, "model": "xw_ex_5160", "radius": 50, "blade_far": -5.6, "blade_near": 0, "frac_scale": 0.8, "strength": 14, "crit": 40, "crit_bonus": 2, "priority": 90, "magic": 2}"#;
    let v3 = parse_stats(&t.replace(r#""sora_strength""#, &format!(r#""sora_base_strength": [{base}], "keyblades": [{kb}], "sora_strength""#))).unwrap();
    let u = v3.keyblade(15).unwrap();
    assert_eq!((u.name.as_str(), u.weapon.radius, u.weapon.strength, u.weapon.priority, u.magic), ("Ultima Weapon", 50.0, 14, 90, 2));
    assert_eq!(v3.strength_with(1, &u.weapon), Some(4 + 14));
    assert_eq!(v3.strength_with(1, &WeaponData::kingdom_key()), Some(4 + 3));
    assert!(v3.keyblade(4).is_none());
    // Format 3 has no model groups: no model swap.
    assert_eq!((u.group, u.textures.len()), (None, 0));
    assert_eq!(crate::keyblade_models::Models::from_stats(&v3).unwrap(), None);
    // Format 4: each converted Keyblade names its PLAYER group and textures.
    let kb4 = kb.replace(r#""magic": 2}"#, r#""magic": 2, "group": 34, "textures": ["xw_ex_5160_0", "xw_ex_5160_1"]}"#);
    let kk = kb.replace(r#""card": 15, "name": "Ultima Weapon""#, r#""card": 1, "name": "Kingdom Key""#);
    let v4 = parse_stats(&t.replace(r#""sora_strength""#, &format!(r#""sora_base_strength": [{base}], "keyblades": [{kk}, {kb4}], "sora_strength""#))).unwrap();
    let u4 = v4.keyblade(15).unwrap();
    assert_eq!((u4.group, u4.textures.clone()), (Some(34), vec!["xw_ex_5160_0".to_string(), "xw_ex_5160_1".to_string()]));
    let m = crate::keyblade_models::Models::from_stats(&v4).unwrap().unwrap();
    assert_eq!((m.group_of(15), m.group_of(1)), (Some(34), Some(2)));
    assert!(parse_stats(&t.replace(r#""sora_strength""#, &format!(r#""keyblades": [{}], "sora_strength""#, kb.replace(r#""magic": 2}"#, r#""magic": 2, "group": -3}"#)))).is_err());
    assert_eq!(s.strength(1), Some(7));
    assert_eq!(s.strength(150), Some(106));
    assert_eq!(s.twin("CEnemyShadow", 1).unwrap().defence, 8);
    assert!(s.twin("CEnemyShadow", 0).is_none() && s.twin("CEnemyShadow", 14).is_none());
    assert!(s.twin("CEnemySoldier", 1).is_none());
    assert!(parse_stats("{}").is_err());
}

#[test]
fn enp_rows_and_proxy_worlds_follow_recoms_stats_init() {
    // `0x14017FF50`: floor − 1; 0xE → 0, 0xF → 2, 100 → 0, 0 → 0; Riku's floors refused.
    assert_eq!((enp_row(1), enp_row(13), enp_row(0), enp_row(0xE), enp_row(0xF), enp_row(100)), (Some(0), Some(12), Some(0), Some(0), Some(2), Some(0)));
    assert_eq!((enp_row(-1), enp_row(-12), enp_row(16)), (None, None, None));
    assert_eq!((proxy_world(0), proxy_world(5), proxy_world(13), proxy_world(14), proxy_world(-1)), (Some(13), Some(5), Some(13), None, None));
    assert!(is_boss_id(102) && is_boss_id(201) && !is_boss_id(29) && !is_boss_id(99));
    assert_eq!((proxy_hp(225, 0.5), proxy_hp(1, 0.1), proxy_hp(33, 12.0 / 33.0)), (113, 1, 12));
}

/// A format-5 file in the asset step's shape (`kh1-recom-assets` `stats::build_json`).
fn stats_v5() -> String {
    let rec = |hp: i32, def: i32, cap: i32, grav: i32, stop: i32| {
        format!(r#"{{"hp": {hp}, "strength": {def}, "defence": {def}, "floor": 2, "cap": {cap}, "level": 30, "exp": 1, "resist": [50, 100, 49, 50, 100, 100, 100, 100, {grav}, {stop}, 50, 100, 100, 100, 100, 100]}}"#)
    };
    let worlds = |r: String| vec![r; 13].join(", ");
    let strength = (1..=100).map(|l| (6 + l).to_string()).collect::<Vec<_>>().join(", ");
    let hp = (0..13).map(|i| (33 + 3 * i).to_string()).collect::<Vec<_>>().join(", ");
    let cell = |ratio: f64, from: &str, r: String| format!(r#"{{"ratio": {ratio:.6}, "from": "{from}", "recom_hp": 50, "kh1_hp": 20, "record": {r}}}"#);
    // Ordinary: ratio 0.4 from the Soldier (Defence 8, cap 300); boss: ratio 2 from Guard Armor
    // with the final boss's Gravity 2 / Stop 0; world 4 row 0 has no cell.
    let orow = |w: usize| (0..13).map(|f| if w == 3 && f == 0 { "null".to_string() } else { cell(0.4, "CEnemySoldier", rec(20, 8, 300, 100, 100)) }).collect::<Vec<_>>().join(", ");
    let brow = (0..13).map(|_| cell(2.0, "CBossGuardarmor", rec(700, 8, 30, 2, 0))).collect::<Vec<_>>().join(", ");
    let ordinary = (0..13).map(|w| format!("[{}]", orow(w))).collect::<Vec<_>>().join(", ");
    let boss = vec![format!("[{brow}]"); 13].join(", ");
    format!(
        r#"{{"format": "kh1-recom-assets stats 5", "sora_strength": [{strength}], "recom_rows": 13, "enemies": [{{"class": "CEnemyShadow", "lock_radius": 30, "recom_id": 1, "recom_hp": [{hp}], "worlds": [{}]}}], "final_boss": {{"kh1_model": "xa_ew_3020.mdls", "gravity": 2, "stop": 0}}, "bosses": [{{"recom_id": 102, "class": "CBossGuardarmor", "name": "Guard Armor", "kh1_model": "xa_tw_3000.mdls", "home_world": 1, "pool": true, "assumed": false, "recom_hp": [{hp}], "worlds": [{}]}}], "proxies": {{"ordinary": [{ordinary}], "boss": [{boss}]}}}}"#,
        worlds(rec(12, 8, 300, 100, 100)),
        worlds(rec(700, 8, 30, 4, 100))
    )
}

#[test]
fn stats_v5_reads_the_boss_twins_and_proxy_tables() {
    let s = parse_stats(&stats_v5()).unwrap();
    assert_eq!(s.twin_recom_hp("CEnemyShadow", 0), Some(33));
    assert_eq!(s.twin_recom_hp("CEnemyShadow", 12), Some(69));
    assert_eq!(s.twin_recom_hp("CEnemySoldier", 0), None);
    assert_eq!((s.bosses.len(), s.bosses[0].id, s.bosses[0].worlds[0].hp), (1, 102, 700));
    let p = s.proxies.as_ref().unwrap();
    assert_eq!((p.ordinary.len(), p.ordinary[0].len(), p.boss.len()), (13, 13, 13));
    assert!(p.ordinary[3][0].is_none());
    // Older files (no proxies) still read: proxies off.
    let old = parse_stats(&stats_v5().replace(r#""proxies":"#, r#""unused":"#)).unwrap();
    assert!(old.proxies.is_none());
    // A table with a world missing is refused.
    let t = stats_v5();
    let at = t.find(r#""ordinary": ["#).unwrap() + r#""ordinary": ["#.len();
    let next = at + t[at..].find("], [").unwrap() + 3;
    let short = format!("{}{}", &t[..at], &t[next..]);
    assert!(parse_stats(&short).unwrap_err().contains("12 worlds"));
    let bad = stats_v5().replace(r#""ratio": 0.400000"#, r#""ratio": -1"#);
    assert!(parse_stats(&bad).is_err(), "a ratio must be positive");
}

#[test]
fn proxies_scale_recoms_hp_and_boss_twins_take_their_own_record() {
    let s = parse_stats(&stats_v5()).unwrap();
    // An ordinary Re:CoM-only enemy (Tornado Step, id 29) with 225 Re:CoM HP on 12F (floor 12).
    let p = s.proxy("CEnemyTornadostep", 29, 12, 12, 225).unwrap();
    assert_eq!((p.stats.hp, p.stats.defence, p.stats.max_damage, p.boss, p.own), (90, 8, 300, false, false));
    assert_eq!(p.stats.resist[8], 100);
    assert_eq!(p.from, "CEnemySoldier");
    // A Re:CoM-only boss (Larxene, id 117): × the boss ratio, Guard Armor's cap, KH1's
    // final-boss Gravity 2 % and Stop immunity (decision 34).
    let l = s.proxy("CBossLarxene", 117, 13, 7, 1000).unwrap();
    assert_eq!((l.stats.hp, l.stats.max_damage, l.stats.resist[8], l.stats.resist[9], l.boss, l.own), (2000, 30, 2, 0, true, false));
    // The boss twin itself: its own KH1 record (Gravity 4, Stop 100), whatever Re:CoM's HP.
    let g = s.proxy("CBossGuardarmor", 102, 1, 1, 350).unwrap();
    assert_eq!((g.stats.hp, g.stats.resist[8], g.stats.resist[9], g.own), (700, 4, 100, true));
    assert!((g.ratio - 2.0).abs() < 1e-9);
    // Another class with the boss's id (a boss part): the boss cell, not the twin's record.
    let part = s.proxy("CBossJafarLamp", 102, 1, 1, 350).unwrap();
    assert_eq!((part.stats.hp, part.own), (700, false));
    // Floor halls (world 0) scale as Castle Oblivion.
    assert_eq!(s.proxy("CBossRiku", 116, 0, 4, 400).unwrap().stats.hp, 800);
    // No proxy: 100 Acre Wood's minigame, no Re:CoM HP, Riku's floors, an empty cell, no tables.
    assert!(s.proxy("CEnemy100AcreBB_Bee", 38, 10, 100, 50).is_none());
    assert!(s.proxy("CEnemyTornadostep", 29, 1, 1, 0).is_none());
    assert!(s.proxy("CEnemyTornadostep", 29, 1, -3, 50).is_none());
    assert!(s.proxy("CEnemyTornadostep", 29, 4, 1, 50).is_none(), "world 4 floor 1 has no cell");
    assert!(s.proxy("CEnemyTornadostep", 29, 14, 1, 50).is_none());
    let old = parse_stats(&stats_v5().replace(r#""proxies":"#, r#""unused":"#)).unwrap();
    assert!(old.proxy("CEnemyTornadostep", 29, 1, 1, 50).is_none());
    assert_eq!(old.proxy("CBossGuardarmor", 102, 1, 1, 350).unwrap().stats.hp, 700, "boss twins need no table");
    // A proxy's hits take KH1's damage on its record: the Soldier's Defence 8, cap 300.
    let m = Moveset::kh1_defaults();
    let mut rng = kh1_core::Kh1Rand::new(1);
    assert_eq!(kh1_damage(&m, 13, 10, &WeaponData::kingdom_key(), &p.stats, &mut rng).unwrap().damage, 2);
    // Guard Armor's cap of 30 holds against a max-level Strength.
    for _ in 0..20 {
        assert!(kh1_damage(&m, 8, 51, &WeaponData::kingdom_key(), &l.stats, &mut rng).unwrap().damage <= 30);
    }
}

/// Re:CoM's per-frame order (Battle exe `0x1401F9840`): the damage check (immune while
/// `+0x7BC > 0`), the action step, then `+0x7BC -= dt`. The hold leaves KH1's 30 immune frames
/// after the last reaction frame, then the 31st frame takes hits.
#[test]
fn post_hit_hold_leaves_thirty_frames() {
    for dt in [50, 100] {
        let frames_per_tick = dt / RECOM_UNITS_PER_FRAME;
        let mut timer = 0;
        // Three reaction frames.
        for _ in 0..3 {
            timer = post_hit_hold(timer, dt);
            timer = (timer - dt).max(0);
        }
        let mut immune = 0;
        while timer > 0 {
            immune += frames_per_tick;
            timer = (timer - dt).max(0);
        }
        assert_eq!(immune, KH1_HIT_UNTOUCHABLE_FRAMES);
    }
    // Re:CoM's revive timer is never shortened.
    assert_eq!(post_hit_hold(3000, 50), 3000);
}
