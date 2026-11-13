//! 입력 키 보기 (머리 위에서 오른쪽 위로 이어지는 구름 꼬리) 테스트.

use std::time::{Duration, Instant};

use typing_pet::keyecho::{BAND, KeyEcho, MAX, label_of};

fn ms(t0: Instant, n: u64) -> Instant {
    t0 + Duration::from_millis(n)
}

#[test]
fn labels_for_keycaps() {
    assert_eq!(label_of(0), Some("A"));
    assert_eq!(label_of(36), Some("↩"));
    assert_eq!(label_of(49), Some("space"));
    assert_eq!(label_of(51), Some("⌫"));
    assert_eq!(label_of(55), Some("⌘"));
    assert_eq!(label_of(56), label_of(60));
    assert_eq!(label_of(123), Some("←"));
    assert_eq!(label_of(122), Some("F1"));
    assert_eq!(label_of(500), None);
    // 글자 키 26개가 전부 서로 다른 대문자
    let letters: std::collections::BTreeSet<&str> =
        (0..=50u16).filter_map(label_of).filter(|l| l.len() == 1 && l.chars().all(|c| c.is_ascii_uppercase())).collect();
    assert_eq!(letters.len(), 26);
}

#[test]
fn unknown_key_is_ignored() {
    let t0 = Instant::now();
    let mut k = KeyEcho::default();
    assert!(!k.push(500, t0));
    assert!(k.is_empty());
    assert!(k.sprites(t0).is_empty());
    assert!(!k.animating(t0));
    assert_eq!(k.next_redraw(t0), None);
}

#[test]
fn pops_at_center_above_head_then_dissolves_after_idle() {
    let t0 = Instant::now();
    let mut k = KeyEcho::default();
    k.set_spot(0.5, 0.2);
    assert!(k.push(0, t0));
    // 튀어나오는 중: 작고 아직 흩어지지 않음
    let s = k.sprites(ms(t0, 40));
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].label, "A");
    assert!(s[0].height < 0.12 && s[0].dissolve == 0.0);
    // 머리 위 가운데 (흔들림 정도만 벗어남), 꼭대기보다 위
    let s = k.sprites(ms(t0, 400));
    assert!((s[0].center[0] - 0.5).abs() < 0.01, "{:?}", s[0].center);
    assert!(s[0].center[1] < 0.2);
    assert!((s[0].height - 0.12).abs() < 1e-3);
    // 떠 있는 동안은 둥실둥실 흔들리므로 계속 그린다
    assert!(k.animating(ms(t0, 400)));
    // 손을 멈추고 한참 뒤 흩어진다
    assert!(k.sprites(ms(t0, 1300))[0].dissolve == 0.0);
    let s = k.sprites(ms(t0, 1600));
    assert!(s[0].dissolve > 0.3 && s[0].dissolve < 1.0);
    // 다 흩어지면 빠지고 더는 그리지 않는다
    assert!(k.sprites(ms(t0, 2000)).is_empty());
    assert!(!k.animating(ms(t0, 2000)));
    assert_eq!(k.next_redraw(ms(t0, 2000)), None);
}

#[test]
fn older_keys_trail_up_and_to_the_right() {
    let t0 = Instant::now();
    let mut k = KeyEcho::default();
    k.set_spot(0.5, 0.15);
    for i in 0..5u64 {
        k.push([0u16, 1, 2, 3, 5][i as usize], ms(t0, i * 400));
    }
    // 밀리기가 끝난 뒤 (꼬리 끝부터 순서대로 나온다: 마지막이 맨 앞)
    let s = k.sprites(ms(t0, 4 * 400 + 350));
    assert_eq!(s.iter().map(|x| x.label).collect::<Vec<_>>(), ["A", "S", "D", "F", "G"]);
    let front = s.last().unwrap();
    assert_eq!(front.label, "G");
    assert!((front.center[0] - 0.5).abs() < 0.01);
    // 뒤로 갈수록 위로, (첫 칸 뒤로는) 오른쪽으로, 작아지고 흐려진다
    for w in s.windows(2) {
        let (back, fwd) = (&w[0], &w[1]);
        assert!(back.center[1] < fwd.center[1], "위로 {:?} {:?}", back.center, fwd.center);
        assert!(back.center[0] > fwd.center[0] - 0.01, "오른쪽으로 {:?} {:?}", back.center, fwd.center);
        assert!(back.height < fwd.height && back.opacity < fwd.opacity);
    }
    assert!(s[0].center[0] > front.center[0] + 0.1, "꼬리 끝은 오른쪽 {:?}", s[0].center);
}

#[test]
fn long_trail_drops_the_far_end_and_stays_in_window() {
    let t0 = Instant::now();
    let mut k = KeyEcho::default();
    k.set_spot(0.5, 0.05);
    // 아주 빠르게 20번
    for i in 0..20u64 {
        k.push([0u16, 1, 2, 3][i as usize % 4], ms(t0, i * 60));
    }
    let s = k.sprites(ms(t0, 19 * 60 + 50));
    let solid = s.iter().filter(|x| x.dissolve == 0.0).count();
    assert_eq!(solid, MAX);
    // 흩어지는 것은 꼬리 쪽 (먼저 그려지는 쪽)
    let first_solid = s.iter().position(|x| x.dissolve == 0.0).unwrap();
    assert!(s[..first_solid].iter().all(|x| x.dissolve > 0.0));
    // 전부 창 안 (가로 0~1, 위로는 띠 안)
    for x in &s {
        let half_w = x.height * typing_pet::keyecho::aspect_of(x.label) / 2.0;
        assert!(x.center[0] - half_w >= -1e-4 && x.center[0] + half_w <= 1.0 + 1e-4, "{:?}", x.center);
        assert!(x.center[1] - x.height / 2.0 >= -(BAND as f32) - 1e-4, "{:?}", x.center);
    }
}

#[test]
fn idle_drains_from_the_far_end() {
    let t0 = Instant::now();
    let mut k = KeyEcho::default();
    k.set_spot(0.5, 0.15);
    for i in 0..4u64 {
        k.push(i as u16, ms(t0, i * 100));
    }
    let last = 300;
    // 손을 멈추고 1.4초 뒤 꼬리 끝(가장 오래된 A)부터 흩어지고, 맨 앞은 아직 또렷
    let s = k.sprites(ms(t0, last + 1400 + 50));
    assert_eq!(s[0].label, "A");
    assert!(s[0].dissolve > 0.0);
    assert!(s.last().unwrap().dissolve == 0.0);
    // 결국 다 사라진다
    assert!(k.sprites(ms(t0, last + 1400 + 4 * 110 + 500)).is_empty());
}

#[test]
fn new_trail_starts_at_head_after_everything_is_gone() {
    let t0 = Instant::now();
    let mut k = KeyEcho::default();
    k.set_spot(0.4, 0.1);
    for i in 0..5u64 {
        k.push(0, ms(t0, i * 50));
    }
    let later = ms(t0, 5_000);
    k.push(1, later);
    let s = k.sprites(ms(t0, 5_400));
    assert_eq!(s.len(), 1);
    assert!((s[0].center[0] - 0.4).abs() < 0.01);
}
