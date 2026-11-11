//! 캐릭터 고유 행동 (docs/CHARACTER_BEHAVIOR_PLAN.md 6장): cue, behavior, 움직임 도구, 검증 규칙.
//! `_test` 의 가짜 behavior 와 테스트 안에서 만든 manifest 로 돈다 (OS, GPU 불필요).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use typing_pet::anim::{Animator, DrawCmd, DrawKind, Frame, motion_xf};
use typing_pet::config::{Behavior, Keys, LayerMode, Manifest, Motion, MotionKind, MotionTarget, Timing};
use typing_pet::input::Input;
use typing_pet::pet::*;

fn ms(v: u64) -> Duration {
    Duration::from_millis(v)
}

fn test_manifest() -> Manifest {
    Manifest::load_dir(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("characters/_test")).expect("_test manifest")
}

fn cues(p: &mut Pet) -> Vec<Cue> {
    p.drain_events().into_iter().filter_map(|e| if let PetEvent::Cue(c) = e { Some(c) } else { None }).collect()
}

fn layer_name(c: &DrawCmd) -> String {
    match &c.kind {
        DrawKind::Layer { name } => name.clone(),
        DrawKind::Mix { a, b, .. } => format!("mix:{a}>{b}"),
        DrawKind::Glow { .. } => "glow".into(),
    }
}

fn find<'a>(f: &'a Frame, name: &str) -> Option<&'a DrawCmd> {
    f.cmds.iter().find(|c| layer_name(c) == name)
}

/// behavior 를 바로 시작한 애니메이터 (head 상태에서, 머리 전환이 끝난 뒤).
fn forced(m: &Manifest, head: Head, name: &str) -> (Animator, Instant) {
    let t0 = Instant::now();
    let mut a = Animator::new(m, 240.0, t0);
    a.cue_behaviors = false;
    a.update(PetState { head, ..Default::default() }, &[], t0);
    let t1 = t0 + ms(2000);
    a.frame(t1);
    a.cue_behaviors = true;
    assert!(a.force(name, t1), "behavior {name}");
    (a, t1)
}

// ---------- cue (pet.rs) ----------

#[test]
fn cue_names_roundtrip_and_reject_typos() {
    for c in Cue::all() {
        assert_eq!(c.to_string().parse::<Cue>().unwrap(), c);
    }
    for bad in ["level:sunglasses", "level:", "key:a", "wak", "Idle"] {
        assert!(bad.parse::<Cue>().is_err(), "{bad}");
    }
    assert_eq!("level:overheat".parse::<Cue>().unwrap(), Cue::Level(Head::Overheat));
    assert_eq!(Cue::of_head(Head::Sunglasses), Cue::Egg);
}

#[test]
fn wake_after_long_rest_only() {
    let t0 = Instant::now();
    let mut p = Pet::new(Timing::default(), Keys::default(), t0);
    p.on_input(Input::Key(0), t0 + ms(1000));
    p.on_input(Input::Key(0), t0 + ms(15_999));
    assert!(!cues(&mut p).contains(&Cue::Wake), "15초 미만 쉼");
    p.on_input(Input::MouseMove, t0 + ms(31_000));
    assert_eq!(cues(&mut p), vec![Cue::Wake], "마우스로 깨워도 wake");
    p.on_input(Input::Key(0), t0 + ms(31_100));
    assert!(cues(&mut p).is_empty());
    let mut t = Timing::default();
    t.wake_after_s = 0;
    let mut p = Pet::new(t, Keys::default(), t0);
    p.on_input(Input::Key(0), t0 + ms(100_000));
    assert!(!cues(&mut p).contains(&Cue::Wake), "0 이면 끔");
}

#[test]
fn burst_needs_many_keys_in_window() {
    let t0 = Instant::now();
    let mut p = Pet::new(Timing::default(), Keys::default(), t0);
    // 초당 5키는 burst 가 아니다
    for k in 0..30 {
        p.on_input(Input::Key(0), t0 + ms(k * 200));
    }
    assert!(!cues(&mut p).contains(&Cue::Burst));
    // 1초 안에 9키: 한 번. 한 번 나면 다시 9키를 세야 한다
    let s = t0 + ms(10_000);
    for k in 0..17 {
        p.on_input(Input::Key(0), s + ms(k * 50));
    }
    assert_eq!(cues(&mut p).iter().filter(|c| **c == Cue::Burst).count(), 1);
    p.on_input(Input::Key(0), s + ms(17 * 50));
    assert_eq!(cues(&mut p), vec![Cue::Burst]);
}

#[test]
fn named_keys_only_by_code() {
    let t0 = Instant::now();
    let mut p = Pet::new(Timing::default(), Keys::default(), t0);
    for (code, want) in [(36u16, Some(NamedKey::Enter)), (76, Some(NamedKey::Enter)), (51, Some(NamedKey::Backspace)), (49, Some(NamedKey::Space)), (53, Some(NamedKey::Escape)), (0, None), (37, None)] {
        p.on_input(Input::Key(code), t0 + ms(10));
        assert_eq!(cues(&mut p), want.map(Cue::Key).into_iter().collect::<Vec<_>>(), "키 코드 {code}");
    }
}

#[test]
fn ambient_random_interval_skips_scenes_and_can_be_off() {
    let t0 = Instant::now();
    let fire_times = |seed: u64| {
        let mut p = Pet::new(Timing::default(), Keys::default(), t0);
        p.reseed(seed, t0);
        let mut t = t0;
        let mut v = Vec::new();
        // 앱처럼 deadline 마다 깨어난다 (장면이 없도록 키를 계속 쳐서 idle 을 막지는 않는다: 기본 타이밍엔 idle 장면이 있다)
        while t < t0 + Duration::from_secs(1000) {
            let d = p.next_deadline(t).unwrap();
            assert!(d > t);
            t = d;
            p.advance(t);
            if cues(&mut p).contains(&Cue::Ambient) {
                v.push(t.duration_since(t0).as_secs_f64());
            }
        }
        v
    };
    let a = fire_times(1);
    assert_eq!(a, fire_times(1), "같은 시드는 같은 순서");
    assert!(!a.is_empty());
    let mut prev = 0.0;
    for t in &a {
        let gap = t - prev;
        assert!((59.9..=180.1).contains(&gap), "간격 {gap}");
        prev = *t;
    }
    assert_ne!(a, fire_times(99), "시드가 다르면 다른 간격");
    // 입력이 올 때까지 계속하는 idle 장면 중에는 ambient 를 하지 않는다
    let mut t = Timing::default();
    t.idle_scene_s = 5;
    t.idle_scene_ms = 0;
    let mut p = Pet::new(t.clone(), Keys::default(), t0);
    p.on_input(Input::Key(0), t0);
    for s in 6..400 {
        p.advance(t0 + Duration::from_secs(s));
        assert!(!cues(&mut p).contains(&Cue::Ambient));
    }
    t.ambient_every_s = [0, 0];
    let p = Pet::new(t, Keys::default(), t0);
    assert!(p.next_deadline(t0 + ms(1)).is_some_and(|d| d < t0 + Duration::from_secs(61)), "끔: 다른 deadline 만");
}

// ---------- behavior 읽기와 검증 (config.rs) ----------

const SCENE_TOML: &str = r#"
[[scene]]
name = "sleep"
trigger = "idle"
duration_ms = 0

[scene.breathe]
period_ms = 4000
pt = 2.0

[[scene.step]]
frame = "full/gesture"
ms = 1000
blend_ms = 200

[[scene.step]]
frame = "full/startle"
ms = 500

[[scene.fx]]
layer = "fx/note"
kind = "float"
anchor = [10, 20]
period_ms = 1000
"#;

fn with_toml(mut m: Manifest, toml_text: &str) -> Manifest {
    let t: toml::Table = toml::from_str(toml_text).unwrap();
    if let Some(v) = t.get("scene") {
        m.scenes = v.clone().try_into().unwrap();
    }
    if let Some(v) = t.get("behavior") {
        m.behaviors = v.clone().try_into().unwrap();
    }
    m.timing = Timing::default();
    m.validate().unwrap();
    m.apply_scenes();
    m
}

#[test]
fn scene_reads_as_full_behavior_with_same_frames() {
    let mut base = test_manifest();
    base.behaviors.clear();
    let old = with_toml(base.clone(), SCENE_TOML);
    let new_text = SCENE_TOML.replace("[[scene]]", "[[behavior]]").replace("trigger = \"idle\"", "on = \"idle\"\nlayer_mode = \"full\"").replace("[scene.", "[behavior.").replace("[[scene.", "[[behavior.");
    let new = with_toml(base, &new_text);
    let b = &old.all_behaviors()[0];
    assert_eq!((b.on, b.mode(), b.duration_ms), (Cue::Idle, LayerMode::Full, Some(0)));
    assert_eq!(old.timing, new.timing, "타이밍 반영도 같다");
    let t0 = Instant::now();
    let (mut a, mut c) = (Animator::new(&old, 240.0, t0), Animator::new(&new, 240.0, t0));
    let s = PetState { pose: Some(Pose::Idle), ..Default::default() };
    a.update(s, &[], t0);
    c.update(s, &[], t0);
    for t in [0, 100, 300, 900, 1200, 1600, 5000] {
        assert_eq!(a.frame(t0 + ms(t)), c.frame(t0 + ms(t)), "{t}ms");
    }
}

#[test]
fn behavior_validation_errors() {
    let mut base = test_manifest();
    base.behaviors.clear();
    let bad = |body: &str| {
        let mut m = base.clone();
        let t: toml::Table = toml::from_str(&format!("[[behavior]]\nname = \"x\"\n{body}")).unwrap();
        m.behaviors = t["behavior"].clone().try_into().map_err(|e: toml::de::Error| e.to_string())?;
        m.validate()
    };
    assert!(bad("on = \"wake\"\n[[behavior.motion]]\ntarget = \"body\"\nkind = \"hop\"").is_ok());
    assert!(bad("on = \"wakeup\"\n[[behavior.motion]]\ntarget = \"body\"\nkind = \"hop\"").is_err(), "cue 오타");
    assert!(bad("on = \"wake\"\nwhen = \"hold\"\n[[behavior.motion]]\ntarget = \"body\"\nkind = \"hop\"").is_err(), "hold 는 단계 cue 만");
    assert!(bad("on = \"wake\"").is_err(), "아무것도 안 하는 overlay");
    assert!(bad("on = \"wake\"\nlayer_mode = \"overlay\"\nframes = [\"full/startle\"]").is_err(), "overlay 에 frames");
    assert!(bad("on = \"wake\"\nframes = [\"full/nope\"]").is_err(), "없는 레이어");
    assert!(bad("on = \"wake\"\nduration_ms = 0\n[[behavior.motion]]\ntarget = \"body\"\nkind = \"hop\"").is_err(), "끝없는 길이");
    assert!(bad("on = \"wake\"\n[[behavior.motion]]\ntarget = \"fx:nope\"\nkind = \"sway\"").is_err(), "없는 소품을 움직임");
    assert!(bad("on = \"wake\"\nchance = 1.5\n[[behavior.motion]]\ntarget = \"body\"\nkind = \"hop\"").is_err());
    assert!(bad("on = \"wake\"\n[behavior.head]\nlayer = \"head/nope\"").is_err());
    assert!(bad("on = \"wake\"\n[[behavior.motion]]\ntarget = \"body\"\nkind = \"spin\"").is_err(), "모르는 움직임");
}

/// 장면 3개만 있는 캐릭터 (지금의 실제 캐릭터와 같은 꼴).
fn basic_only(name: &str) -> Manifest {
    let mut m = test_manifest();
    m.name = name.into();
    m.behaviors.clear();
    let t: toml::Table = toml::from_str(&format!(
        r#"
[[scene]]
name = "{name}_rest"
trigger = "idle"
frames = ["full/gesture", "full/startle"]
[[scene]]
name = "{name}_win"
trigger = "milestone"
frames = ["full/gesture", "full/startle"]
[[scene]]
name = "{name}_cool"
trigger = "gesture"
frames = ["full/gesture", "full/startle"]
"#
    ))
    .unwrap();
    m.scenes = t["scene"].clone().try_into().unwrap();
    m
}

fn extra(name: &str, on: &str) -> Behavior {
    toml::from_str::<toml::Table>(&format!("name = \"{name}\"\non = \"{on}\"\n[[motion]]\ntarget = \"body\"\nkind = \"hop\""))
        .unwrap()
        .try_into()
        .unwrap()
}

#[test]
fn character_rules_need_unique_moving_behaviors() {
    let m = basic_only("a");
    let errs = m.validate_character(&[]);
    assert!(errs.iter().any(|e| e.contains("최소 3개")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("절반 이상")), "{errs:?}");
    // 장면이 하나 빠지면 그것도 걸린다
    let mut no_idle = m.clone();
    no_idle.scenes.remove(0);
    assert!(no_idle.validate_character(&[]).iter().any(|e| e.starts_with("idle cue")));
    // cue 밖 behavior 3개가 움직이면 통과
    let mut ok = m.clone();
    ok.behaviors = vec![extra("a_wake", "wake"), extra("a_burst", "burst"), extra("a_hot", "level:overheat")];
    assert_eq!(ok.validate_character(&[]), Vec::<String>::new());
    // 다른 캐릭터와 이름이 같으면 안 된다
    let mut other = basic_only("b");
    other.behaviors = vec![extra("a_wake", "wake")];
    assert!(ok.validate_character(&[other]).iter().any(|e| e.contains("이름이 겹침")));
    // 같은 그림 파일을 쓰면 안 된다 (같은 _test 폴더를 가리키는 두 캐릭터)
    let copy = basic_only("c");
    assert!(ok.validate_character(&[copy]).iter().any(|e| e.contains("같은 파일")));
    // ambient 가 너무 잦으면 안 된다
    let mut noisy = ok.clone();
    noisy.timing.ambient_every_s = [10, 20];
    assert!(noisy.validate_character(&[]).iter().any(|e| e.contains("ambient")));
}

// ---------- 움직임 도구 (anim.rs) ----------

fn one(kind: MotionKind, pt: f32, period: u64, count: u32) -> Vec<Motion> {
    vec![Motion { target: "body".into(), kind, pt: Some(pt), period_ms: Some(period), count, delay_ms: 0, squash: 0.05 }]
}

#[test]
fn motion_values_over_time() {
    let at = |m: &[Motion], t: f32, len: Option<f32>| motion_xf(m, &[], MotionTarget::Body, t, len, [0.0, 100.0], 2.0);
    // hop: 정점 (0.375 주기) 에서 pt * px_per_pt 만큼 위, 착지 때 납작, 끝나면 제자리
    let hop = one(MotionKind::Hop, 6.0, 400, 1);
    assert!((at(&hop, 150.0, None).offset[1] + 12.0).abs() < 1e-3, "{:?}", at(&hop, 150.0, None));
    let land = at(&hop, 350.0, None);
    assert!(land.s[1] < 1.0 && land.s[0] > 1.0 && land.offset[1] == 0.0, "착지 {land:?}");
    assert!(at(&hop, 400.0, None).is_identity() && at(&hop, 5000.0, None).is_identity());
    // tremble: 진폭 안에서 떨고, behavior 가 끝나면 0
    let tr = one(MotionKind::Tremble, 1.5, 60, 0);
    let xs: Vec<f32> = (0..200).map(|i| at(&tr, i as f32 * 7.0, Some(1400.0)).offset[0]).collect();
    assert!(xs.iter().all(|x| x.abs() <= 3.0 + 1e-3), "진폭 상한");
    assert!(xs.iter().any(|x| x.abs() > 1.5), "실제로 떤다");
    assert!(at(&tr, 1400.0, Some(1400.0)).offset[0].abs() < 1e-3);
    assert_eq!(at(&tr, 0.0, Some(1400.0)).offset[0], 0.0, "서서히 시작");
    // sway, nod
    let sw = one(MotionKind::Sway, 2.0, 1000, 1);
    assert!((at(&sw, 250.0, None).offset[0] - 4.0).abs() < 1e-3 && (at(&sw, 750.0, None).offset[0] + 4.0).abs() < 1e-3);
    let nod = one(MotionKind::Nod, 3.0, 500, 1);
    assert!((at(&nod, 250.0, None).offset[1] - 6.0).abs() < 1e-3, "아래로");
    // squash: 기준점(아래) 기준으로 세로 줄고 가로 늘었다 돌아온다
    let sq = vec![Motion { squash: 0.1, ..one(MotionKind::Squash, 0.0, 400, 1)[0].clone() }];
    let mid = at(&sq, 200.0, None);
    assert!((mid.s[1] - 0.9).abs() < 1e-4 && (mid.s[0] - 1.1).abs() < 1e-4);
    assert_eq!(mid.anchor, [0.0, 100.0]);
    // delay
    let late = vec![Motion { delay_ms: 300, ..hop[0].clone() }];
    assert!(at(&late, 200.0, None).is_identity() && !at(&late, 450.0, None).is_identity());
}

#[test]
fn body_motion_moves_character_inside_body_mask_only() {
    let m = test_manifest();
    let (mut a, t) = forced(&m, Head::Normal, "t_bounce");
    let f = a.frame(t + ms(130)); // 첫 hop 정점 근처
    let names: Vec<String> = f.cmds.iter().map(layer_name).collect();
    assert_eq!(&names[..3], &["base", "base_clean", "base"], "{names:?}");
    let moved = &f.cmds[2];
    assert_eq!(moved.mask.as_deref(), Some("mask/body"));
    assert!(moved.offset[1] < -5.0, "몸이 위로 {:?}", moved.offset);
    assert_eq!(f.cmds[0].offset, [0.0, 0.0], "책상은 제자리");
    // 앞발은 키보드 위에 그대로, 머리는 몸을 따라 (squash 도 같이)
    assert_eq!(find(&f, "paw_l/up").unwrap().offset, [0.0, 0.0]);
    // 앞발은 키보드 윗줄 아래(paw_l_low)는 제자리, 위(paw_l_top, 팔)는 몸을 따라간다
    let paws: Vec<&DrawCmd> = f.cmds.iter().filter(|c| layer_name(c) == "paw_l/up").collect();
    assert_eq!(paws.len(), 2);
    assert_eq!((paws[0].mask.as_deref(), paws[0].offset), (Some("mask/paw_l_low"), [0.0, 0.0]));
    assert_eq!(paws[1].mask.as_deref(), Some("mask/paw_l_top"));
    assert!(paws[1].offset[1] < -5.0);
    let head = find(&f, "head/normal").unwrap();
    assert!(head.offset[1] < -5.0);
    assert!(a.animating(t + ms(130)));
    // 끝나면 원래 합성
    let f = a.frame(t + ms(800));
    assert!(find(&f, "base_clean").is_none() && f.cmds.iter().all(|c| c.offset == [0.0, 0.0] && c.squash == [1.0, 1.0]));
    assert_eq!(find(&f, "paw_l/up").unwrap().mask.as_deref(), Some("mask/paw_l"), "가만히 있으면 앞발 마스크 한 장");
    assert!(!a.animating(t + ms(1100)));
}

#[test]
fn body_motion_without_clean_desk_is_limited_to_2pt() {
    let mut m = test_manifest();
    m.layers.retain(|l| l != "base_clean");
    let (mut a, t) = forced(&m, Head::Normal, "t_bounce");
    let f = a.frame(t + ms(130));
    let moved = f.cmds.iter().find(|c| c.mask.as_deref() == Some("mask/body")).unwrap();
    let px_per_pt = m.canvas as f32 / 240.0;
    assert!((moved.offset[1] + 2.0 * px_per_pt).abs() < 1e-3, "2pt 로 제한 {:?}", moved.offset);
    assert!(find(&f, "base_clean").is_none());
}

#[test]
fn head_swap_and_nod() {
    let m = test_manifest();
    let (mut a, t) = forced(&m, Head::Normal, "t_nod");
    match &find_head(&a.frame(t + ms(100))).kind {
        DrawKind::Mix { a, b, t } => assert!(a == "head/normal" && b == "head/alt_a" && *t > 0.0 && *t < 1.0),
        k => panic!("섞는 중이어야 함 {k:?}"),
    }
    let f = a.frame(t + ms(300));
    let h = find(&f, "head/alt_a").expect("바뀐 머리");
    assert!(h.offset[1] > 0.0, "끄덕 {:?}", h.offset);
    let f = a.frame(t + ms(700));
    assert!(find(&f, "head/normal").is_some(), "끝나면 원래 머리");
    // 두 장 교대 (ambient)
    let (mut a, t) = forced(&m, Head::Normal, "t_twitch");
    let seen: std::collections::BTreeSet<String> = (2..6).map(|i| layer_name(find_head(&a.frame(t + ms(i * 150 + 20))))).collect();
    assert!(seen.contains("head/alt_a") && seen.contains("head/alt_b"), "{seen:?}");
}

fn find_head(f: &Frame) -> &DrawCmd {
    f.cmds.iter().find(|c| c.mask.as_deref() == Some("mask/head")).expect("머리")
}

#[test]
fn fx_kinds_pop_fall_flash_float() {
    let m = test_manifest();
    // pop_in: 작게 시작해 커지고 period 뒤 사라진다
    let (mut a, t) = forced(&m, Head::Overheat, "t_shout");
    let s1 = find(&a.frame(t + ms(10)), "fx/bubble").unwrap().scale;
    let s2 = find(&a.frame(t + ms(300)), "fx/bubble").unwrap().scale;
    assert!(s1 < 0.6 && (s2 - 1.0).abs() < 0.05, "{s1} -> {s2}");
    assert!(find(&a.frame(t + ms(850)), "fx/bubble").is_none());
    // fall: 아래로 내려간다
    let (mut a, t) = forced(&m, Head::Overheat, "t_shiver");
    let y1 = find(&a.frame(t + ms(200)), "fx/drop").unwrap().offset[1];
    let y2 = find(&a.frame(t + ms(400)), "fx/drop").unwrap().offset[1];
    assert!(y2 > y1 && y1 >= 0.0, "{y1} -> {y2}");
    // flash: 한 번 번쩍, float 소품은 fx:note 움직임으로 좌우로도 흔들린다
    let (mut a, t) = forced(&m, Head::Normal, "t_flash");
    let op: Vec<f32> = [10, 100, 290, 400].iter().map(|&d| find(&a.frame(t + ms(d)), "fx/white").map_or(0.0, |c| c.opacity)).collect();
    assert!(op[1] > 0.5 && op[0] < op[1] && op[2] < op[1] && op[3] == 0.0, "{op:?}");
    assert!(find(&a.frame(t + ms(50)), "fx/note").is_none(), "delay_ms 전");
    let xs: Vec<f32> = (0..8).filter_map(|i| find(&a.frame(t + ms(200 + i * 60)), "fx/note").map(|c| c.offset[0])).collect();
    let spread = xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min);
    assert!(spread > 5.0, "흔들림 {xs:?}");
}

// ---------- 발동, 우선순위, 쿨다운, hold ----------

/// Pet 과 Animator 를 같이 돌리는 작은 앱.
struct Sim {
    pet: Pet,
    anim: Animator,
    t0: Instant,
    started: Vec<(u64, String)>,
}

impl Sim {
    fn new(m: &Manifest) -> Self {
        let t0 = Instant::now();
        let mut pet = Pet::new(m.timing.clone(), m.keys.clone(), t0);
        pet.reseed(3, t0);
        let mut anim = Animator::new(m, 240.0, t0);
        anim.reseed(3);
        Self { pet, anim, t0, started: Vec::new() }
    }
    fn at(&self, t: u64) -> Instant {
        self.t0 + ms(t)
    }
    fn sync(&mut self, t: u64) {
        let now = self.at(t);
        self.pet.advance(now);
        let ev = self.pet.drain_events();
        self.anim.update(self.pet.state(now), &ev, now);
        for n in self.anim.drain_started() {
            self.started.push((t, n));
        }
    }
    fn key(&mut self, t: u64, code: u16) {
        self.pet.on_input(Input::Key(code), self.at(t));
        self.sync(t);
    }
    fn names(&self) -> Vec<&str> {
        self.started.iter().map(|(_, n)| n.as_str()).collect()
    }
}

#[test]
fn wake_plays_full_behavior_then_returns() {
    let m = test_manifest();
    let mut s = Sim::new(&m);
    s.key(100, 0);
    s.key(20_000, 0);
    assert_eq!(s.names(), vec!["t_startle"]);
    assert_eq!(s.pet.state(s.at(20_000)).pose, None, "pet 장면과 따로 논다");
    let f = s.anim.frame(s.at(20_150));
    assert_eq!(s.anim.full_layer(s.at(20_400)), Some("full/startle"));
    assert!(f.cmds.iter().any(|c| c.mask.as_deref() == Some("mask/body") && c.offset[1] < 0.0), "full 도 몸 마스크 안을 띄운다");
    assert!(s.anim.next_redraw(s.at(20_400)).is_some());
    // 800ms 뒤 끝나고 원래 합성으로 페이드
    s.sync(20_801);
    assert_eq!(s.anim.full_layer(s.at(20_801)), None);
    let f = s.anim.frame(s.at(21_200));
    assert!(!f.layer_names().iter().any(|n| n.starts_with("full/")));
    // 다시 15초 넘게 쉬면 또 한다
    s.key(36_000, 0);
    assert_eq!(s.names(), vec!["t_startle", "t_startle"]);
    // cooldown 5초: 그 안에 같은 cue 가 다시 와도 안 한다
    s.anim.update(s.pet.state(s.at(37_000)), &[PetEvent::Cue(Cue::Wake)], s.at(37_000));
    assert!(s.anim.drain_started().is_empty());
    s.anim.update(s.pet.state(s.at(41_100)), &[PetEvent::Cue(Cue::Wake)], s.at(41_100));
    assert_eq!(s.anim.drain_started(), vec!["t_startle".to_string()]);
}

#[test]
fn burst_cooldown_and_full_hides_overlay() {
    let m = test_manifest();
    let mut s = Sim::new(&m);
    for k in 0..40 {
        s.key(100 + k * 50, 0);
    }
    // 2초 동안 burst 4번이지만 cooldown 2초라 1번
    assert_eq!(s.names().iter().filter(|n| **n == "t_bounce").count(), 1, "{:?}", s.started);
    // full 이 보이는 동안 overlay 는 그리지 않는다
    let mut s = Sim::new(&m);
    s.key(100, 0);
    for k in 0..9 {
        s.key(20_000 + k * 40, 0); // 첫 키가 wake, 9번째가 burst
    }
    assert!(s.names().contains(&"t_startle") && s.names().contains(&"t_bounce"), "{:?}", s.started);
    let now = s.at(20_600);
    assert_eq!(s.anim.playing(now), vec!["t_startle".to_string(), "t_bounce".to_string()]);
    let f = s.anim.frame(now);
    assert!(f.cmds.iter().all(|c| c.mask.as_deref() != Some("mask/body") || c.layers()[0].starts_with("full/") || c.layers()[0] == "base_clean"));
}

#[test]
fn hold_repeats_while_level_lasts() {
    let m = test_manifest();
    let (mut a, t) = forced(&m, Head::Overheat, "t_shiver");
    // 600ms 재생, 400ms 쉼, 반복
    let shaking = |a: &mut Animator, d: u64| a.frame(t + ms(d)).cmds.iter().any(|c| c.offset[0] != 0.0);
    assert!(shaking(&mut a, 300) && !shaking(&mut a, 800) && shaking(&mut a, 1300));
    assert_eq!(a.next_redraw(t + ms(700)), Some(t + ms(1000)), "쉬는 동안은 다음 재생 때 깨어남");
    assert!(!a.animating(t + ms(800)));
    // 단계를 벗어나면 끝
    a.update(PetState { head: Head::Dizzy, ..Default::default() }, &[], t + ms(1100));
    assert!(!shaking(&mut a, 1300) && !shaking(&mut a, 2300));
    // 실제 흐름: 과열에 들어가면 외침(enter) 과 떨기(hold) 가 같이 시작한다
    let mut s = Sim::new(&m);
    let mut k = 0;
    while s.pet.level() != Level::Overheat {
        s.key(100 + k * 60, [0u16, 37, 1, 38][k as usize % 4]);
        k += 1;
    }
    assert!(s.names().contains(&"t_shout") && s.names().contains(&"t_shiver"), "{:?}", s.started);
}

#[test]
fn priority_picks_higher_overlay() {
    let m = test_manifest();
    let (mut a, t) = forced(&m, Head::Overheat, "t_shiver"); // priority 10
    assert!(a.force("t_shout", t + ms(100))); // priority 20
    assert_eq!(a.playing(t + ms(200)), vec!["t_shout".to_string()]);
    // 외침이 끝나면 (900ms) 떨기가 다시 보인다
    assert_eq!(a.playing(t + ms(1100)), vec!["t_shiver".to_string()]);
}

#[test]
fn chance_and_calm_switch() {
    let m = test_manifest();
    let mut s = Sim::new(&m);
    for k in 0..200 {
        s.key(100 + k * 1100, 53); // Escape, t_flash chance 0.5
    }
    let n = s.names().iter().filter(|n| **n == "t_flash").count();
    assert!((60..=140).contains(&n), "반쯤 {n}");
    // 반응 줄이기: 장면 밖 behavior 는 안 하고, 장면은 그대로
    let mut s = Sim::new(&m);
    s.anim.cue_behaviors = false;
    s.key(100, 36);
    s.key(30_000, 53);
    assert!(s.names().is_empty());
    s.anim.update(PetState { pose: Some(Pose::Gesture), ..Default::default() }, &[], s.at(31_000));
    assert_eq!(s.anim.full_layer(s.at(31_500)), Some("full/gesture"));
}

#[test]
fn behavior_frames_and_scenarios_cover_every_cue() {
    let m = test_manifest();
    let frames = typing_pet::behavior_frames(&m);
    let names: Vec<&str> = frames.iter().map(|(n, _)| n.as_str()).collect();
    for b in m.all_behaviors() {
        assert!(names.contains(&format!("bhv-{}", b.name).as_str()), "{} 없음", b.name);
    }
    let layers = typing_pet::anim::Layers::from_manifest(&m);
    for (n, f) in &frames {
        assert!(f.layer_names().iter().all(|l| layers.has(l)), "{n}");
    }
    // 시나리오가 그 cue 를 실제로 낸다
    let fired = |name: &str| {
        let t0 = Instant::now();
        let mut p = Pet::new(Timing::default(), Keys::default(), t0);
        let mut v = Vec::new();
        for (t, i) in typing_pet::scenario_inputs(name).unwrap() {
            p.on_input(i, t0 + ms(t));
            v.extend(cues(&mut p));
            v.extend((p.level() == Level::Overheat).then_some(Cue::Level(Head::Overheat)));
        }
        v
    };
    assert!(fired("wake").contains(&Cue::Wake));
    assert!(fired("burst").contains(&Cue::Burst));
    assert!(fired("levels").contains(&Cue::Level(Head::Overheat)));
    let k = fired("keys");
    assert!(k.contains(&Cue::Key(NamedKey::Enter)) && k.contains(&Cue::Key(NamedKey::Backspace)));
}
