//! typing-pet 라이브러리: 앱(main.rs), 프로브(bin/probe.rs), 테스트(tests/)가 같이 쓴다.

pub mod anim;
pub mod config;
pub mod desk;
pub mod input;
pub mod keyecho;
pub mod pet;
pub mod render;
pub mod trace;

#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(target_os = "macos")]
pub mod text;
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
    // 조합 시트는 상태 조합만 본다 (단계 cue 로 나오는 behavior 는 behavior_frames 에서 따로)
    a.cue_behaviors = false;
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

/// 장면 cue(idle, milestone, gesture) 밖의 behavior 마다 한 장: 그 behavior 를 바로 시작해 중간쯤을 그린다.
/// 이름은 `bhv-<behavior 이름>`. 단계 cue 면 그 머리 상태에서 시작한다.
pub fn behavior_frames(m: &Manifest) -> Vec<(String, Frame)> {
    use crate::pet::Cue;
    let mut v = Vec::new();
    for b in m.all_behaviors().into_iter().filter(|b| b.on.pose().is_none()) {
        let t0 = Instant::now();
        let mut a = Animator::new(m, 240.0, t0);
        a.cue_behaviors = false;
        let head = match b.on {
            Cue::Level(h) => h,
            Cue::Egg => Head::Sunglasses,
            _ => Head::Normal,
        };
        // 머리 전환이 끝난 뒤에 시작한다
        let t1 = t0 + Duration::from_millis(2000);
        a.update(PetState { head, ..Default::default() }, &[], t0);
        a.frame(t1);
        a.cue_behaviors = true;
        if !a.force(&b.name, t1) {
            continue;
        }
        let len = b.duration_ms.filter(|&d| d > 0).unwrap_or(crate::config::Behavior::DEFAULT_MS);
        v.push((format!("bhv-{}", b.name), a.frame(t1 + Duration::from_millis(len * 2 / 5))));
    }
    v
}

/// 움직임만 뺀 manifest: behavior/장면의 몸/머리/소품 움직임, 숨쉬기, 머리 팝, 마우스 흔들기를 0 으로.
/// 같은 입력으로 돌리면 같은 장면/레이어/페이드에 움직임만 없는 "정지 기준" 프레임이 나온다 (움직임 QA, 그리는 방법과 상관없이).
pub fn still_manifest(m: &Manifest) -> Manifest {
    let mut s = m.clone();
    for b in &mut s.behaviors {
        b.motion.clear();
        b.breathe = None;
    }
    for sc in &mut s.scenes {
        sc.breathe = None;
    }
    s.anim.pop_scale = 1.0;
    s.anim.swing_pt = [0.0; 2];
    s.anim.breathe_pt = 0.0;
    s
}

/// behavior 하나를 처음부터 끝까지 (길이가 없거나 길면 6초까지) `fps` 로 재생한 (프레임, 움직임을 뺀 같은 프레임) 들 (움직임 QA).
/// 장면(idle, milestone, gesture) behavior 도 그 pose 에서 강제로 튼다.
pub fn behavior_run(m: &Manifest, name: &str, fps: u64) -> Option<Vec<(Frame, Frame)>> {
    let b = m.all_behaviors().into_iter().find(|b| b.name == name)?;
    let still = still_manifest(m);
    let t0 = Instant::now();
    let (mut a, t1) = forced_at(m, &b, t0)?;
    let (mut s, _) = forced_at(&still, &b, t0)?;
    let len = b.duration_ms.filter(|&d| d > 0).unwrap_or(if b.on.pose().is_some() { 6_000 } else { crate::config::Behavior::DEFAULT_MS }).min(6_000);
    let n = len * fps / 1000;
    Some((0..n).map(|i| { let t = t1 + Duration::from_millis(i * 1000 / fps); (a.frame(t), s.frame(t)) }).collect())
}

fn forced_at(m: &Manifest, b: &crate::config::Behavior, t0: Instant) -> Option<(Animator, Instant)> {
    use crate::pet::Cue;
    let mut a = Animator::new(m, crate::anim::REF_SIZE_PT, t0);
    a.cue_behaviors = false;
    let head = match b.on {
        Cue::Level(h) => h,
        Cue::Egg => Head::Sunglasses,
        _ => Head::Normal,
    };
    let t1 = t0 + Duration::from_millis(2000);
    a.update(PetState { head, ..Default::default() }, &[], t0);
    a.frame(t1);
    if let Some(p) = b.on.pose() {
        a.update(PetState { head, pose: Some(p), ..Default::default() }, &[], t1);
    }
    a.cue_behaviors = true;
    a.force(&b.name, t1).then_some((a, t1))
}

/// 두 그림이 겹쳐 보이는 프레임인가 (크로스페이드, 장면 페이드). 움직임 QA 의 고스트 검사에서 뺀다.
pub fn is_blend(f: &Frame) -> bool {
    use crate::anim::DrawKind;
    f.cmds.iter().any(|c| match &c.kind {
        DrawKind::Mix { .. } => true,
        DrawKind::Layer { name } => c.opacity < 0.999 && ["base", "full/", "paw_", "head/"].iter().any(|p| name.starts_with(p)),
        DrawKind::Glow { .. } => false,
    })
}

/// `--scenario` 이름별 입력: demo(기본, [`timeline_inputs`]), idle(키 하나 뒤 입력 없음), milestone(키를 빠르게 계속),
/// wake(쉬다가 다시 치기), burst(가끔 몰아치기), levels(천천히 과열까지 올렸다 식히기), ambient(키 하나 뒤 오래 가만히, --secs 200 쯤),
/// keys(Enter/Backspace/Space 등 이름 있는 키).
pub fn scenario_inputs(name: &str) -> Option<Vec<(u64, crate::input::Input)>> {
    use crate::input::Input;
    let typing = |from: u64, to: u64, gap: u64| (0..).map(move |k| (from + k * gap, Input::Key([0u16, 37, 1, 38][k as usize % 4]))).take_while(move |(t, _)| *t < to);
    match name {
        "demo" => Some(timeline_inputs()),
        "idle" | "ambient" => Some(vec![(300, Input::Key(0))]),
        "milestone" => Some((0..400u64).map(|k| (300 + k * 40, Input::Key([0u16, 37, 1, 38][k as usize % 4]))).collect()),
        "wake" => Some(typing(300, 2_000, 200).chain(typing(18_000, 22_000, 200)).collect()),
        "burst" => Some((0..4u64).flat_map(|r| typing(500 + r * 3_000, 1_500 + r * 3_000, 70)).collect()),
        "levels" => Some(typing(300, 9_000, 75).chain(typing(9_000, 12_000, 300)).collect()),
        "mouse" => Some(typing(300, 1_500, 120).chain((0..60u64).map(|k| (2_000 + k * 50, Input::MouseMove))).chain([(3_200, Input::MouseClick), (4_200, Input::MouseClick)]).collect()),
        "keys" => Some([36u16, 51, 51, 51, 49, 48, 53, 117, 36].iter().enumerate().map(|(i, &c)| (300 + i as u64 * 1_500, Input::Key(c))).collect()),
        _ => None,
    }
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
