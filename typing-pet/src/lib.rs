//! typing-pet 라이브러리: 앱(main.rs), 프로브(bin/probe.rs), 테스트(tests/)가 같이 쓴다.

pub mod anim;
pub mod config;
pub mod input;
pub mod pet;
pub mod render;
pub mod trace;

#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(target_os = "macos")]
pub mod tray;

use std::time::{Duration, Instant};

use crate::anim::{Animator, Frame};
use crate::config::Manifest;
use crate::pet::{Head, PawL, PawR, PetState, Pose};

/// 전환이 끝난 상태의 그릴 목록 (오프스크린 렌더/조합 시트용). 귀 흔들기는 `wiggle` 로 고른다.
pub fn settled_frame(m: &Manifest, state: PetState, wiggle: bool) -> Frame {
    let t0 = Instant::now();
    let mut a = Animator::new(m, 240.0, t0);
    a.update(state, &[], t0);
    // 전환이 다 끝난 뒤, 귀 흔들기 교대의 짝/홀 위치
    let w = m.anim.wiggle_ms.max(1);
    let n = 10_000 / w + if wiggle { 1 } else { 0 };
    let n = if (n % 2 == 1) == wiggle { n } else { n + 1 };
    a.frame(t0 + Duration::from_millis(n * w + w / 2))
}

/// 조합 시트에 넣을 모든 상태 (머리 6 x 왼발 2 x 오른발 5 + 장면 3 + 귀 흔들기).
pub fn all_states() -> Vec<(String, PetState, bool)> {
    let mut v = Vec::new();
    for head in Head::ALL {
        for paw_l in [PawL::Up, PawL::Down] {
            for paw_r in [PawR::Up, PawR::Down, PawR::Center, PawR::Mouse, PawR::MouseClick] {
                let name = format!("{}__{}__{}", head.layer().replace('/', "-"), paw_l.layer().replace('/', "-"), paw_r.layer().replace('/', "-"));
                v.push((name, PetState { paw_l, paw_r, head, pose: None }, false));
            }
        }
    }
    v.push(("head-dizzy_wiggle__paw_l-up__paw_r-up".into(), PetState { head: Head::Dizzy, ..Default::default() }, true));
    v.push(("full-gesture".into(), PetState { pose: Some(Pose::Gesture), ..Default::default() }, false));
    v.push(("full-idle".into(), PetState { pose: Some(Pose::Idle), ..Default::default() }, false));
    v.push(("full-milestone".into(), PetState { pose: Some(Pose::Milestone), ..Default::default() }, false));
    v
}

/// 참고 영상(PLAN 1.2)을 흉내 낸 입력 시나리오: (시각 ms, 입력).
/// 빠르게 쳐서 신남 -> 빙글 안경 -> 과열, 멈춰서 식으며 제스처, 마우스, 10연타 선글라스.
pub fn timeline_inputs() -> Vec<(u64, crate::input::Input)> {
    use crate::input::Input;
    let mut v = Vec::new();
    let keys = [0u16, 37, 1, 38, 2, 40, 3, 32, 13, 34, 14, 31, 49, 4];
    let mut t = 300u64;
    let mut i = 0;
    // 0 ~ 8s: 초당 약 12키
    while t < 8_000 {
        v.push((t, Input::Key(keys[i % keys.len()])));
        i += 1;
        t += if i % 5 == 0 { 110 } else { 75 };
    }
    // 8 ~ 11s: 마우스 이동과 클릭
    for k in 0..20 {
        v.push((8_200 + k * 100, Input::MouseMove));
    }
    v.push((9_000, Input::MouseClick));
    v.push((9_600, Input::MouseClick));
    // 13.5s (제스처가 끝난 뒤): 같은 키 10연타 -> 선글라스
    for k in 0..10 {
        v.push((13_500 + k * 90, Input::Key(17)));
    }
    // 이후 선글라스 쓴 채 천천히 타이핑
    let mut t = 15_000;
    while t < 17_500 {
        v.push((t, Input::Key(keys[i % keys.len()])));
        i += 1;
        t += 160;
    }
    v
}
