//! 1층 단위 테스트 (PLAN 6.3). OS 와 GPU 없이 돈다.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use typing_pet::anim::{Animator, DrawKind, Layers};
use typing_pet::config::{Keys, Manifest, Timing};
use typing_pet::input::*;
use typing_pet::pet::*;

fn ms(v: u64) -> Duration {
    Duration::from_millis(v)
}

fn test_manifest() -> Manifest {
    Manifest::load_dir(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("characters/_test")).expect("_test manifest")
}

fn new_pet() -> (Pet, Instant) {
    let t0 = Instant::now();
    (Pet::new(Timing::default(), Keys::default(), t0), t0)
}

fn st(p: &mut Pet, t: Instant) -> PetState {
    p.advance(t);
    p.state(t)
}

// ---------- 키 판정 ----------

#[test]
fn every_keycode_has_a_zone_matching_manifest() {
    let m = test_manifest();
    let keys = &m.keys;
    // 왼쪽/가운데 목록은 서로 겹치지 않는다
    for k in &keys.left {
        assert!(!keys.center.contains(k), "key {k} 이 left/center 둘 다에 있음");
    }
    for code in 0u16..128 {
        let z = zone_of(code, keys);
        let want = if keys.left.contains(&code) {
            Zone::Left
        } else if keys.center.contains(&code) {
            Zone::Center
        } else {
            Zone::Right
        };
        assert_eq!(z, want, "keycode {code}");
    }
    // PLAN 5.3 의 대표 키
    let left = [50, 18, 19, 20, 21, 23, 48, 12, 13, 14, 15, 17, 57, 0, 1, 2, 3, 5, 56, 6, 7, 8, 9, 11, 59, 58, 55];
    for k in left {
        assert_eq!(zone_of(k, keys), Zone::Left, "left key {k}");
    }
    for k in [22, 26, 16, 32, 4, 38, 45, 46] {
        assert_eq!(zone_of(k, keys), Zone::Center, "center key {k}");
    }
    for k in [24, 27, 29, 25, 28, 34, 31, 35, 37, 41, 39, 43, 47, 44, 36, 51, 60, 62, 61, 54, 123, 124, 125, 126] {
        assert_eq!(zone_of(k, keys), Zone::Right, "right key {k}");
    }
}

#[test]
fn every_keycode_moves_exactly_the_expected_paw() {
    let keys = Keys::default();
    for code in 0u16..128 {
        let (mut p, t0) = new_pet();
        p.on_input(Input::Key(code), t0);
        let s = st(&mut p, t0);
        match zone_of(code, &keys) {
            Zone::Left => assert_eq!((s.paw_l, s.paw_r), (PawL::Down, PawR::Up), "key {code}"),
            Zone::Center => assert_eq!((s.paw_l, s.paw_r), (PawL::Up, PawR::Center), "key {code}"),
            Zone::Right => assert_eq!((s.paw_l, s.paw_r), (PawL::Up, PawR::Down), "key {code}"),
        }
        let s = st(&mut p, t0 + ms(120));
        assert_eq!((s.paw_l, s.paw_r), (PawL::Up, PawR::Up), "key {code} 120ms 후");
    }
}

// ---------- 변환: 수식키, 자동 반복, 마우스 쓰로틀 ----------

#[test]
fn modifiers_press_maps_to_key_and_release_is_ignored() {
    let keys = Keys::default();
    let t0 = Instant::now();
    let mut c = Converter::new(ms(100));
    for &(code, name, dev, generic) in MODIFIERS.iter() {
        let down_flags = dev | generic;
        assert_eq!(c.convert(RawEvent::FlagsChanged { code, flags: down_flags }, t0), Some(Input::Key(code)), "{name} 누름");
        if code != 57 {
            assert_eq!(c.convert(RawEvent::FlagsChanged { code, flags: 0 }, t0), None, "{name} 뗌");
        }
        // 장치별 비트가 없는 키보드 (일반 비트만)
        if code != 57 {
            assert_eq!(c.convert(RawEvent::FlagsChanged { code, flags: generic }, t0), Some(Input::Key(code)), "{name} 일반 비트");
        }
    }
    // 왼쪽 Shift 를 누른 채 오른쪽 Shift 를 떼면 무시
    assert_eq!(c.convert(RawEvent::FlagsChanged { code: 60, flags: FLAG_SHIFT | DEV_LSHIFT }, t0), None);
    // 발 판정: 왼쪽 수식키는 왼발, 오른쪽 수식키는 오른발
    for (code, z) in [(56, Zone::Left), (59, Zone::Left), (58, Zone::Left), (55, Zone::Left), (57, Zone::Left), (63, Zone::Left), (60, Zone::Right), (62, Zone::Right), (61, Zone::Right), (54, Zone::Right)] {
        assert_eq!(zone_of(code, &keys), z, "modifier {code}");
    }
    // Caps Lock 은 켤 때/끌 때 모두 누름
    assert_eq!(c.convert(RawEvent::FlagsChanged { code: 57, flags: FLAG_CAPS }, t0), Some(Input::Key(57)));
    assert_eq!(c.convert(RawEvent::FlagsChanged { code: 57, flags: 0 }, t0), Some(Input::Key(57)));
}

#[test]
fn autorepeat_is_dropped_everywhere() {
    let t0 = Instant::now();
    let mut c = Converter::new(ms(100));
    assert_eq!(c.convert(RawEvent::KeyDown { code: 51, autorepeat: true }, t0), None);
    assert_eq!(c.convert(RawEvent::KeyDown { code: 51, autorepeat: false }, t0), Some(Input::Key(51)));
    assert_eq!(c.convert(RawEvent::KeyUp { code: 51 }, t0), None);

    // 백스페이스를 꾹 눌러도 점수, 이스터에그, 앞발에 한 번만 반영
    let (mut p, t0) = new_pet();
    let mut c = Converter::new(ms(100));
    let mut t = t0;
    let mut first = true;
    for _ in 0..200 {
        if let Some(i) = c.convert(RawEvent::KeyDown { code: 51, autorepeat: !first }, t) {
            p.on_input(i, t);
        }
        first = false;
        t += ms(30);
    }
    assert!(p.score(t0) <= 1.0 + 1e-9);
    assert_ne!(st(&mut p, t).head, Head::Sunglasses);
    assert_eq!(st(&mut p, t).head, Head::Normal);
}

#[test]
fn mouse_throttle_100ms() {
    let t0 = Instant::now();
    let mut c = Converter::new(ms(100));
    let mut n = 0;
    for i in 0..100 {
        let ev = [RawEvent::MouseMove, RawEvent::MouseDrag, RawEvent::Scroll][i % 3];
        if c.convert(ev, t0 + ms(i as u64 * 10)).is_some() {
            n += 1;
        }
    }
    assert_eq!(n, 10, "1초 동안 10ms 간격 100번 -> 10번");
    assert_eq!(c.convert(RawEvent::MouseDown, t0 + ms(1000)), Some(Input::MouseClick), "클릭은 쓰로틀 없음");
}

// ---------- 앞발 타이밍 ----------

#[test]
fn paw_timer_extends_on_repeat_press() {
    let (mut p, t0) = new_pet();
    p.on_input(Input::Key(0), t0);
    p.on_input(Input::Key(1), t0 + ms(100));
    assert_eq!(st(&mut p, t0 + ms(150)).paw_l, PawL::Down, "두 번째 누름으로 연장");
    assert_eq!(st(&mut p, t0 + ms(219)).paw_l, PawL::Down);
    assert_eq!(st(&mut p, t0 + ms(220)).paw_l, PawL::Up);
    // 연장 중 PressL 사건은 누를 때마다 (누름 연출)
    assert_eq!(p.drain_events().iter().filter(|e| **e == PetEvent::PressL).count(), 2);
}

#[test]
fn both_paws_down_within_window() {
    let (mut p, t0) = new_pet();
    p.on_input(Input::Key(0), t0); // A
    p.on_input(Input::Key(37), t0 + ms(50)); // L
    let s = st(&mut p, t0 + ms(60));
    assert_eq!((s.paw_l, s.paw_r), (PawL::Down, PawR::Down));
    let s = st(&mut p, t0 + ms(125));
    assert_eq!((s.paw_l, s.paw_r), (PawL::Up, PawR::Down));
    let s = st(&mut p, t0 + ms(170));
    assert_eq!((s.paw_l, s.paw_r), (PawL::Up, PawR::Up));
}

#[test]
fn next_deadline_matches_paw_return() {
    let (mut p, t0) = new_pet();
    p.on_input(Input::Key(0), t0);
    assert_eq!(p.next_deadline(t0), Some(t0 + ms(120)));
}

// ---------- 마우스 ----------

#[test]
fn mouse_move_hold_and_click() {
    let (mut p, t0) = new_pet();
    p.on_input(Input::MouseMove, t0);
    assert_eq!(st(&mut p, t0).paw_r, PawR::Mouse);
    assert_eq!(st(&mut p, t0 + ms(999)).paw_r, PawR::Mouse);
    assert_eq!(st(&mut p, t0 + ms(1000)).paw_r, PawR::Up);

    let t1 = t0 + ms(2000);
    p.on_input(Input::MouseClick, t1);
    assert_eq!(st(&mut p, t1).paw_r, PawR::MouseClick);
    assert_eq!(st(&mut p, t1 + ms(119)).paw_r, PawR::MouseClick);
    assert_eq!(st(&mut p, t1 + ms(120)).paw_r, PawR::Mouse);
    assert_eq!(st(&mut p, t1 + ms(1119)).paw_r, PawR::Mouse);
    assert_eq!(st(&mut p, t1 + ms(1120)).paw_r, PawR::Up);
}

#[test]
fn right_key_during_mouse_goes_to_keyboard_then_back() {
    let (mut p, t0) = new_pet();
    p.on_input(Input::MouseMove, t0);
    p.on_input(Input::Key(37), t0 + ms(200)); // L, 오른쪽
    assert_eq!(st(&mut p, t0 + ms(210)).paw_r, PawR::Down);
    assert_eq!(st(&mut p, t0 + ms(320)).paw_r, PawR::Mouse);
    p.on_input(Input::Key(38), t0 + ms(400)); // J, 가운데
    assert_eq!(st(&mut p, t0 + ms(410)).paw_r, PawR::Center);
    assert_eq!(st(&mut p, t0 + ms(520)).paw_r, PawR::Mouse);
    // 왼쪽 키는 오른발에 영향 없음
    p.on_input(Input::Key(0), t0 + ms(600));
    assert_eq!(st(&mut p, t0 + ms(610)).paw_r, PawR::Mouse);
    assert_eq!(st(&mut p, t0 + ms(1000)).paw_r, PawR::Up);
}

// ---------- 활동 점수 ----------

/// 일정 속도로 누르면서 머리 상태 변화 기록.
fn type_at(p: &mut Pet, from: Instant, secs: f64, kps: f64, log: &mut Vec<(Instant, Head)>) -> Instant {
    let step = Duration::from_secs_f64(1.0 / kps);
    let end = from + Duration::from_secs_f64(secs);
    let mut t = from;
    let keys = [0u16, 37, 1, 40, 2, 38];
    let mut i = 0;
    while t < end {
        p.on_input(Input::Key(keys[i % keys.len()]), t);
        i += 1;
        let h = st(p, t).head;
        if log.last().is_none_or(|x| x.1 != h) {
            log.push((t, h));
        }
        t += step;
    }
    t
}

fn idle(p: &mut Pet, from: Instant, secs: f64, log: &mut Vec<(Instant, Head)>) -> Instant {
    let end = from + Duration::from_secs_f64(secs);
    let mut t = from;
    while t < end {
        let h = st(p, t).head;
        if log.last().is_none_or(|x| x.1 != h) {
            log.push((t, h));
        }
        t += ms(50);
    }
    t
}

#[test]
fn activity_climbs_in_order_and_cools_in_reverse() {
    let (mut p, t0) = new_pet();
    let mut log = Vec::new();
    let t = type_at(&mut p, t0, 20.0, 10.0, &mut log);
    let seq: Vec<Head> = log.iter().map(|x| x.1).collect();
    assert_eq!(seq, vec![Head::Normal, Head::Excited, Head::Dizzy, Head::Overheat], "오르는 순서");
    let mut down = Vec::new();
    idle(&mut p, t, 50.0, &mut down);
    let seq: Vec<Head> = down.iter().map(|x| x.1).collect();
    assert_eq!(seq, vec![Head::Overheat, Head::Dizzy, Head::Excited, Head::Normal], "식는 순서");
}

#[test]
fn no_flicker_near_threshold() {
    // 점수가 경계 근처에서 머물도록 천천히/빠르게 섞어 누른다
    let (mut p, t0) = new_pet();
    let mut log = Vec::new();
    let mut t = t0;
    for i in 0..60 {
        let kps = if i % 2 == 0 { 3.2 } else { 4.6 };
        t = type_at(&mut p, t, 0.7, kps, &mut log);
    }
    for w in log.windows(2) {
        let dt = w[1].0 - w[0].0;
        assert!(dt >= ms(1500), "단계가 {dt:?} 만에 다시 바뀜: {:?} -> {:?}", w[0].1, w[1].1);
    }
}

#[test]
fn coding_style_bursts_reach_excited() {
    // 코딩처럼 2초 치고 1초 쉬는 패턴도 excited 이상 도달
    let (mut p, t0) = new_pet();
    let mut log = Vec::new();
    let mut t = t0;
    for _ in 0..10 {
        t = type_at(&mut p, t, 2.0, 7.0, &mut log);
        t = idle(&mut p, t, 1.0, &mut log);
    }
    assert!(log.iter().any(|x| x.1 == Head::Excited), "{log:?}");
}

// ---------- sleepy, gesture, sunglasses ----------

#[test]
fn sleepy_after_60s_and_wakes_immediately() {
    let (mut p, t0) = new_pet();
    assert_eq!(st(&mut p, t0 + ms(59_999)).head, Head::Normal);
    assert_eq!(p.next_deadline(t0 + ms(59_000)), Some(t0 + ms(60_000)));
    assert_eq!(st(&mut p, t0 + ms(60_000)).head, Head::Sleepy);
    p.on_input(Input::MouseMove, t0 + ms(61_000));
    assert_eq!(st(&mut p, t0 + ms(61_000)).head, Head::Normal);
    assert_eq!(st(&mut p, t0 + ms(121_000)).head, Head::Sleepy);
    p.on_input(Input::Key(0), t0 + ms(122_000));
    assert_eq!(st(&mut p, t0 + ms(122_000)).head, Head::Normal);
}

#[test]
fn gesture_on_overheat_cooldown_with_reuse_delay() {
    let (mut p, t0) = new_pet();
    let mut log = Vec::new();
    let t = type_at(&mut p, t0, 20.0, 10.0, &mut log);
    assert_eq!(p.level(), Level::Overheat);
    // 식히면서 제스처 확인
    let mut saw = None;
    let mut tt = t;
    for _ in 0..400 {
        tt += ms(50);
        if st(&mut p, tt).pose == Some(Pose::Gesture) {
            saw = Some(tt);
            break;
        }
    }
    let start = saw.expect("과열이 식을 때 제스처");
    assert_eq!(p.level(), Level::Dizzy, "과열에서 한 칸 내려올 때");
    assert_eq!(st(&mut p, start + ms(1990)).pose, Some(Pose::Gesture));
    assert_eq!(st(&mut p, start + ms(2000)).pose, None, "2초 뒤 끝");
    tt = start + ms(20_000);
    // 5분 안에 다시 과열 -> 식어도 제스처 없음
    let t2 = type_at(&mut p, tt + ms(1000), 20.0, 10.0, &mut log);
    let mut again = false;
    let mut tt2 = t2;
    for _ in 0..400 {
        tt2 += ms(50);
        again |= st(&mut p, tt2).pose == Some(Pose::Gesture);
    }
    assert!(!again, "재사용 대기 중에는 제스처 없음");
    // 끌 수 있다
    let mut t = Timing::default();
    t.gesture = false;
    let mut p = Pet::new(t, Keys::default(), t0);
    let tt = type_at(&mut p, t0, 20.0, 10.0, &mut log);
    let mut any = false;
    for i in 0..400 {
        any |= st(&mut p, tt + ms(i * 50)).pose.is_some();
    }
    assert!(!any);
}

#[test]
fn sunglasses_easter_egg_on_off_and_timeout() {
    let (mut p, t0) = new_pet();
    let mut t = t0;
    for _ in 0..9 {
        p.on_input(Input::Key(17), t); // T
        t += ms(150);
    }
    assert_ne!(st(&mut p, t).head, Head::Sunglasses, "9번으로는 안 켜짐");
    p.on_input(Input::Key(17), t);
    assert_eq!(st(&mut p, t).head, Head::Sunglasses, "10연타로 켜짐");
    // 다른 머리 단계보다 우선
    let mut log = Vec::new();
    let t1 = type_at(&mut p, t, 5.0, 8.0, &mut log);
    assert_eq!(st(&mut p, t1).head, Head::Sunglasses);
    // 다시 10연타로 끔
    let mut t2 = t1;
    for _ in 0..10 {
        p.on_input(Input::Key(40), t2);
        t2 += ms(100);
    }
    assert_ne!(st(&mut p, t2).head, Head::Sunglasses);
    // 켜고 30초 지나면 꺼짐
    let mut t3 = t2 + ms(60_000);
    for _ in 0..10 {
        p.on_input(Input::Key(17), t3);
        t3 += ms(100);
    }
    assert_eq!(st(&mut p, t3).head, Head::Sunglasses);
    let on_at = t3 - ms(100);
    assert_eq!(st(&mut p, on_at + ms(29_999)).head, Head::Sunglasses);
    assert_ne!(st(&mut p, on_at + ms(30_000)).head, Head::Sunglasses);
    // 중간에 다른 키가 끼면 다시 셈
    let (mut p, t0) = new_pet();
    let mut t = t0;
    for i in 0..19 {
        p.on_input(Input::Key(if i == 9 { 0 } else { 17 }), t);
        t += ms(100);
    }
    assert_ne!(st(&mut p, t).head, Head::Sunglasses);
}

// ---------- 렌더 목록 ----------

fn layers_all() -> Layers {
    Layers::from_manifest(&test_manifest())
}

#[test]
fn render_list_order_matches_plan() {
    let m = test_manifest();
    let t0 = Instant::now();
    let mut a = Animator::new(&m, 240.0, t0);
    let s = PetState { paw_l: PawL::Down, paw_r: PawR::Center, head: Head::Normal, pose: None };
    a.update(s, &[PetEvent::PressL, PetEvent::PressR(PawR::Center)], t0);
    let f = a.frame(t0 + ms(10));
    let kinds: Vec<String> = f
        .cmds
        .iter()
        .map(|c| match &c.kind {
            DrawKind::Layer { name } => name.clone(),
            DrawKind::Mix { a, b, .. } => format!("mix:{a}>{b}"),
            DrawKind::Glow { .. } => "glow".into(),
        })
        .collect();
    assert_eq!(kinds, vec!["base", "paw_l/down", "paw_r/center", "glow", "glow", "head/normal"]);
    // 앞발 레이어에는 책상/키보드 조각이 들어 있으므로 누를 때도 제자리에 그린다
    assert_eq!(f.cmds[1].offset, [0.0, 0.0]);
    assert_eq!(f.cmds[2].offset, [0.0, 0.0]);
    // 220ms 뒤에는 빛 없음
    let f = a.frame(t0 + ms(230));
    assert!(!f.cmds.iter().any(|c| matches!(c.kind, DrawKind::Glow { .. })));
    assert!(!a.animating(t0 + ms(400)));
}

#[test]
fn crossfade_weights_sum_to_one() {
    let m = test_manifest();
    let t0 = Instant::now();
    let mut a = Animator::new(&m, 240.0, t0);
    a.update(PetState::default(), &[], t0);
    a.update(PetState { head: Head::Excited, ..Default::default() }, &[], t0 + ms(10));
    let mut saw_mix = false;
    for i in 0..30 {
        let f = a.frame(t0 + ms(10 + i * 10));
        for c in &f.cmds {
            if let DrawKind::Mix { a, b, t } = &c.kind {
                saw_mix = true;
                let (wa, wb) = (1.0 - t, *t);
                assert!(((wa + wb) - 1.0).abs() < 1e-6);
                assert!((0.0..=1.0).contains(t));
                assert_eq!((a.as_str(), b.as_str()), ("head/normal", "head/excited"));
            }
        }
    }
    assert!(saw_mix);
    // 끝나면 한 장
    let f = a.frame(t0 + ms(400));
    assert!(f.cmds.iter().any(|c| c.kind == DrawKind::Layer { name: "head/excited".into() }));
}

#[test]
fn level_up_pops_head_and_dizzy_wiggles() {
    let m = test_manifest();
    let t0 = Instant::now();
    let mut a = Animator::new(&m, 240.0, t0);
    a.update(PetState { head: Head::Dizzy, ..Default::default() }, &[], t0);
    let f = a.frame(t0 + ms(70));
    let head = f.cmds.iter().find(|c| c.layers().iter().any(|l| l.starts_with("head/"))).unwrap();
    assert!(head.scale > 1.0, "팝");
    let names: BTreeSet<String> = (0..20)
        .flat_map(|i| a.frame(t0 + ms(400 + i * 40)).layer_names().into_iter().map(String::from).collect::<Vec<_>>())
        .collect();
    assert!(names.contains("head/dizzy") && names.contains("head/dizzy_wiggle"), "80ms 교대: {names:?}");
    assert!(a.next_redraw(t0 + ms(500)).is_some());
}

#[test]
fn sunglasses_drop_then_crossfade() {
    let m = test_manifest();
    let t0 = Instant::now();
    let mut a = Animator::new(&m, 240.0, t0);
    a.update(PetState { head: Head::Sunglasses, ..Default::default() }, &[], t0);
    let f = a.frame(t0 + ms(100));
    let fx = f.cmds.iter().find(|c| c.layers() == vec!["fx/sunglasses"]).expect("선글라스 fx");
    assert!(fx.offset[1] < 0.0, "위에서 내려오는 중");
    assert!(fx.opacity > 0.0 && fx.opacity < 1.0);
    assert!(f.layer_names().contains(&"head/normal"));
    let f = a.frame(t0 + ms(500));
    assert!(f.cmds.iter().any(|c| matches!(&c.kind, DrawKind::Mix { b, .. } if b == "head/sunglasses")));
    let f = a.frame(t0 + ms(800));
    assert!(f.layer_names().contains(&"head/sunglasses"));
    assert!(!f.layer_names().contains(&"fx/sunglasses"));
}

#[test]
fn gesture_full_frame_replaces_layers() {
    let m = test_manifest();
    let t0 = Instant::now();
    let mut a = Animator::new(&m, 240.0, t0);
    a.update(PetState { pose: Some(Pose::Gesture), ..Default::default() }, &[], t0);
    let mid = a.frame(t0 + ms(120));
    let full = mid.cmds.last().unwrap();
    assert_eq!(full.layers(), vec!["full/gesture"]);
    assert!(full.opacity > 0.0 && full.opacity < 1.0);
    let done = a.frame(t0 + ms(300));
    assert_eq!(done.layer_names(), vec!["full/gesture"]);
}

#[test]
fn missing_layers_fall_back_to_existing_ones() {
    let have: BTreeSet<String> = ["base", "head/normal", "paw_l/up", "paw_r/up", "paw_r/down"].iter().map(|s| s.to_string()).collect();
    let l = Layers::new(have.clone());
    assert_eq!(l.resolve("head/dizzy_wiggle").as_deref(), Some("head/normal"));
    assert_eq!(l.resolve("paw_r/center").as_deref(), Some("paw_r/down"));
    assert_eq!(l.resolve("paw_r/mouse_click").as_deref(), Some("paw_r/up"));
    assert_eq!(l.resolve("paw_l/down").as_deref(), Some("paw_l/up"));
    assert_eq!(l.resolve("full/gesture"), None);
    let all = layers_all();
    for s in typing_pet::all_states() {
        let f = typing_pet::settled_frame(&test_manifest(), s.1, s.2);
        for n in f.layer_names() {
            assert!(all.has(n), "{} 에서 없는 레이어 {n}", s.0);
        }
    }
}

#[test]
fn config_roundtrip() {
    let dir = std::env::temp_dir().join(format!("typing-pet-test-{}", std::process::id()));
    let path = dir.join("config.toml");
    // SAFETY: 이 테스트만 이 환경 변수를 쓴다
    unsafe { std::env::set_var("TYPING_PET_CONFIG", &path) };
    let mut c = typing_pet::config::Config::default();
    c.position = Some((12.5, 34.0));
    c.size = 320.0;
    c.breathing = false;
    c.save().unwrap();
    assert_eq!(typing_pet::config::Config::load(), c);
    std::fs::write(&path, "not = [valid").unwrap();
    assert_eq!(typing_pet::config::Config::load(), typing_pet::config::Config::default());
    let _ = std::fs::remove_dir_all(dir);
}

// ---------- 마우스 연출 ----------

fn swing_manifest() -> Manifest {
    let mut m = test_manifest();
    m.layers.push("paw_r/mouse_bg".into());
    m.layers.push("fx/mouse_arm".into());
    m.swing = Some(typing_pet::config::Swing { root: [300.0, 300.0], tip: [340.0, 400.0], radius: [20.0, 40.0] });
    m
}

#[test]
fn mouse_move_swings_only_the_arm() {
    let m = swing_manifest();
    let t0 = Instant::now();
    let mut pet = Pet::new(m.timing.clone(), m.keys.clone(), t0);
    let mut a = Animator::new(&m, 240.0, t0);
    // 100ms 마다 마우스 이동 1초
    for k in 0..10 {
        let t = t0 + ms(k * 100);
        pet.on_input(Input::MouseMove, t);
        let ev = pet.drain_events();
        assert!(ev.contains(&PetEvent::MouseMove));
        a.update(pet.state(t), &ev, t);
    }
    let names = |f: &typing_pet::anim::Frame| f.layer_names().into_iter().map(String::from).collect::<Vec<_>>();
    let f = a.frame(t0 + ms(950));
    assert!(names(&f).contains(&"paw_r/mouse_bg".to_string()) && names(&f).contains(&"fx/mouse_arm".to_string()));
    assert!(!names(&f).contains(&"paw_r/mouse".to_string()), "한 장짜리 마우스 자세 대신 바탕 + 팔");
    // 팔만 휘고(warp), 다른 레이어는 움직이지 않는다
    let mut moved = 0;
    for k in 0..12 {
        let f = a.frame(t0 + ms(300 + k * 25));
        for c in &f.cmds {
            if let Some(w) = &c.warp {
                assert!(matches!(&c.kind, DrawKind::Layer { name } if name == "fx/mouse_arm"));
                if w.d[0].abs() > 1.0 {
                    moved += 1;
                }
            } else {
                assert_eq!(c.offset[0], 0.0);
            }
        }
    }
    assert!(moved >= 6, "흔들리는 프레임 {moved}");
    assert!(a.animating(t0 + ms(950)));
    // 마지막 이동(900ms) 뒤 hold + fade 가 지나면 멈춘다 (손은 아직 마우스 위)
    let t = t0 + ms(900 + m.anim.swing_hold_ms + m.anim.swing_fade_ms + 10);
    pet.advance(t);
    a.update(pet.state(t), &pet.drain_events(), t);
    assert_eq!(pet.state(t).paw_r, PawR::Mouse);
    let f = a.frame(t);
    let w = f.cmds.iter().find_map(|c| c.warp).expect("팔");
    assert_eq!(w.d, [0.0, 0.0]);
    assert!(!a.animating(t));
}

#[test]
fn mouse_pose_without_swing_layers_is_one_layer() {
    let m = test_manifest();
    let t0 = Instant::now();
    let mut a = Animator::new(&m, 240.0, t0);
    let s = PetState { paw_r: PawR::Mouse, ..Default::default() };
    a.update(s, &[PetEvent::MouseMove], t0);
    let f = a.frame(t0 + ms(50));
    assert!(f.layer_names().contains(&"paw_r/mouse"));
    assert!(f.cmds.iter().all(|c| c.warp.is_none()));
    assert!(!a.animating(t0 + ms(50)));
}
