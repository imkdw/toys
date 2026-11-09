//! 무작위 입력 테스트 (PLAN 6.3). 키, 수식키, 자동 반복, 마우스, 시간 경과를 섞어 넣고 항상 지켜야 하는 규칙을 검사한다.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use proptest::prelude::*;
use typing_pet::anim::{Animator, Layers};
use typing_pet::config::Manifest;
use typing_pet::input::{Converter, MODIFIERS, RawEvent};
use typing_pet::pet::{Head, PawL, PawR, Pet};

#[derive(Debug, Clone)]
enum Step {
    Raw(RawEvent),
    Wait(u64),
}

fn raw() -> impl Strategy<Value = RawEvent> {
    prop_oneof![
        6 => (0u16..128, any::<bool>()).prop_map(|(code, r)| RawEvent::KeyDown { code, autorepeat: r }),
        2 => (0u16..128).prop_map(|code| RawEvent::KeyUp { code }),
        2 => (0usize..MODIFIERS.len(), any::<bool>()).prop_map(|(i, d)| {
            let (code, _, dev, gen_) = MODIFIERS[i];
            RawEvent::FlagsChanged { code, flags: if d { dev | gen_ } else { 0 } }
        }),
        2 => Just(RawEvent::MouseMove),
        1 => Just(RawEvent::MouseDrag),
        1 => Just(RawEvent::Scroll),
        1 => Just(RawEvent::MouseDown),
    ]
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        5 => raw().prop_map(Step::Raw),
        3 => (0u64..300).prop_map(Step::Wait),
        1 => (300u64..70_000).prop_map(Step::Wait),
    ]
}

fn manifest() -> Manifest {
    Manifest::load_dir(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("characters/_test")).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 400, ..ProptestConfig::default() })]

    #[test]
    fn invariants_hold(steps in prop::collection::vec(step(), 1..250)) {
        let m = manifest();
        let layers = Layers::from_manifest(&m);
        let t0 = Instant::now();
        let mut pet = Pet::new(m.timing.clone(), m.keys.clone(), t0);
        let mut conv = Converter::new(Duration::from_millis(m.timing.mouse_throttle_ms));
        let mut anim = Animator::new(&m, 240.0, t0);
        anim.breathing = true;
        let mut t = t0;
        for s in &steps {
            match s {
                Step::Raw(r) => {
                    if let Some(i) = conv.convert(*r, t) {
                        pet.on_input(i, t);
                    }
                }
                Step::Wait(ms) => {
                    // 기다리는 동안 앱처럼 deadline 마다 깨어남
                    let end = t + Duration::from_millis(*ms);
                    let mut guard = 0;
                    while let Some(d) = pet.next_deadline(t) {
                        // next_deadline 은 과거가 아니다 (CPU 를 태우는 무한 루프 방지)
                        prop_assert!(d > t, "deadline 이 과거/현재: {:?}", d.saturating_duration_since(t));
                        if d > end { break; }
                        t = d;
                        pet.advance(t);
                        guard += 1;
                        prop_assert!(guard < 10_000, "deadline 이 너무 잘게 쪼개짐");
                    }
                    t = end;
                }
            }
            pet.advance(t);
            let st = pet.state(t);
            let ev = pet.drain_events();
            anim.update(st, &ev, t);
            let f = anim.frame(t);
            for n in f.layer_names() {
                prop_assert!(layers.has(n), "없는 레이어 {n}");
            }
            for c in &f.cmds {
                prop_assert!(c.opacity >= 0.0 && c.opacity <= 1.0 && c.opacity.is_finite());
                prop_assert!(c.scale.is_finite() && c.offset.iter().all(|v| v.is_finite()));
            }
            if let Some(r) = anim.next_redraw(t) {
                prop_assert!(r > t);
            }
        }
        // 입력이 없으면 정해진 시간 안에 기본 자세로 돌아온다 (선글라스 30초, 점수 감소 포함 넉넉히 3분)
        let mut tt = t;
        let limit = t + Duration::from_secs(180);
        while let Some(d) = pet.next_deadline(tt) {
            if d > limit { break; }
            tt = d;
            pet.advance(tt);
        }
        let st = pet.state(limit);
        prop_assert_eq!(st.paw_l, PawL::Up);
        prop_assert_eq!(st.paw_r, PawR::Up);
        prop_assert!(st.pose.is_none());
        prop_assert!(matches!(st.head, Head::Sleepy | Head::Normal), "head {:?}", st.head);
        // sleepy 를 깨우면 normal
        pet.on_input(typing_pet::input::Input::MouseMove, limit);
        prop_assert_eq!(pet.state(limit).head, Head::Normal);
        // 연출도 끝난다
        anim.update(pet.state(limit), &[], limit);
        prop_assert!(!anim.animating(limit + Duration::from_secs(2)));
    }
}
