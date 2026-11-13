//! 책상 소품(직군 장비, 캐릭터 소품, 누르기)과 애니에 없는 표정 막기.
//! `_test` 레이어와 테스트 안에서 만든 소품 목록으로 돈다 (로컬 characters/_props 와 무관, GPU 불필요).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use typing_pet::anim::{Animator, DrawCmd, DrawKind, Frame};
use typing_pet::config::{Desk, Manifest};
use typing_pet::desk::Catalog;
use typing_pet::pet::*;

fn ms(v: u64) -> Duration {
    Duration::from_millis(v)
}

fn catalog() -> Catalog {
    toml::from_str(
        r#"
[[prop]]
id = "mug"
title = "머그"
slot = "left_back"
foot = [100.0, 400.0]

[[prop.tap]]
target = "body"
kind = "hop"
pt = 4
period_ms = 400
count = 1

[[prop]]
id = "duck"
title = "오리"
slot = "left_front"
foot = [110.0, 460.0]

[[prop]]
id = "lamp"
title = "스탠드"
slot = "left_back"
foot = [100.0, 400.0]

[[prop]]
id = "cake"
title = "케이크"
slot = "right_back"
foot = [420.0, 400.0]

[[job]]
id = "dev"
title = "개발자"
items = ["mug", "duck"]

[[job]]
id = "student"
title = "학생"
items = ["lamp"]
"#,
    )
    .unwrap()
}

/// `_test` + 소품 목록. 소품 레이어는 이름만 있으면 된다 (그리기 목록만 비교).
fn manifest(item: Option<&str>) -> Manifest {
    let mut m = Manifest::load_dir(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("characters/_test")).unwrap();
    m.catalog = catalog();
    m.layers.retain(|l| !l.starts_with("prop/"));
    for p in &m.catalog.props {
        m.layers.push(p.layer());
    }
    m.desk = Some(Desk { item: item.map(str::to_string), ..Default::default() });
    m
}

fn names(f: &Frame) -> Vec<String> {
    f.cmds
        .iter()
        .map(|c| match &c.kind {
            DrawKind::Layer { name } => name.clone(),
            DrawKind::Mix { a, b, .. } => format!("mix:{a}>{b}"),
            DrawKind::Glow { .. } => "glow".into(),
        })
        .collect()
}

fn find<'a>(f: &'a Frame, name: &str) -> Option<&'a DrawCmd> {
    f.cmds.iter().find(|c| matches!(&c.kind, DrawKind::Layer { name: n } if n == name))
}

#[test]
fn catalog_rejects_bad_jobs() {
    let mut c = catalog();
    assert!(c.validate().is_ok());
    c.jobs[0].items.push("nope".into());
    assert!(c.validate().unwrap_err().contains("없는 소품"));
    let mut c = catalog();
    c.jobs[0].items.push("lamp".into());
    assert!(c.validate().unwrap_err().contains("자리에 소품이 둘"), "mug 와 lamp 는 같은 자리");
}

#[test]
fn job_and_character_items_share_the_desk() {
    let c = catalog();
    let ids = |v: Vec<typing_pet::desk::Prop>| v.into_iter().map(|p| p.id).collect::<Vec<_>>();
    assert_eq!(ids(c.items(Some("dev"), Some("cake"))), ["mug", "duck", "cake"]);
    assert_eq!(ids(c.items(None, Some("cake"))), ["cake"]);
    assert_eq!(ids(c.items(Some("nope"), None)), Vec::<String>::new());
    // 캐릭터 소품과 같은 자리의 직군 소품은 뺀다
    assert_eq!(ids(c.items(Some("dev"), Some("lamp"))), ["duck", "lamp"]);
}

#[test]
fn props_are_drawn_on_top_and_follow_the_job() {
    let t0 = Instant::now();
    let mut m = manifest(Some("cake"));
    m.job = Some("dev".into());
    let mut a = Animator::new(&m, 240.0, t0);
    let f = a.frame(t0 + ms(2000));
    let n = names(&f);
    let head = n.iter().position(|x| x == "head/normal").unwrap();
    for p in ["prop/mug", "prop/duck", "prop/cake"] {
        assert!(n.iter().position(|x| x == p).unwrap() > head, "소품은 앞발/머리 뒤에 그린다: {n:?}");
    }
    a.set_job(Some("student"));
    let n = names(&a.frame(t0 + ms(2100)));
    assert!(n.contains(&"prop/lamp".into()) && n.contains(&"prop/cake".into()) && !n.contains(&"prop/mug".into()), "{n:?}");
    a.set_job(None);
    assert_eq!(a.items().iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["cake"]);
}

#[test]
fn tapping_a_prop_moves_it_and_tells_the_character() {
    let t0 = Instant::now();
    let mut m = manifest(None);
    m.job = Some("dev".into());
    let mut a = Animator::new(&m, 240.0, t0);
    a.frame(t0 + ms(1000));
    assert!(!a.tap("cake", t0 + ms(1000)), "놓여 있지 않은 소품");
    assert!(a.tap("mug", t0 + ms(1000)));
    assert_eq!(a.drain_started(), ["t_poke"], "prop cue 로 캐릭터가 반응");
    assert!(a.animating(t0 + ms(1100)));
    // hop 정점 (0.375 주기) 에서 위로, 끝나면 제자리
    let up = find(&a.frame(t0 + ms(1150)), "prop/mug").unwrap().offset[1];
    assert!(up < -1.0, "{up}");
    let done = find(&a.frame(t0 + ms(2000)), "prop/mug").unwrap().clone();
    assert_eq!((done.offset, done.scale), ([0.0, 0.0], 1.0));
    // 기본 움직임 (tap 이 없는 소품)도 움직인다
    assert!(a.tap("duck", t0 + ms(3000)));
    let f = a.frame(t0 + ms(3140));
    assert!(find(&f, "prop/duck").unwrap().offset[1] < -1.0);
}

#[test]
fn scene_frames_hide_covered_slots() {
    let t0 = Instant::now();
    let mut m = manifest(Some("cake"));
    m.job = Some("dev".into());
    m.desk.as_mut().unwrap().hide.insert("full/gesture".into(), vec!["right_back".into()]);
    let mut a = Animator::new(&m, 240.0, t0);
    a.update(PetState { pose: Some(Pose::Gesture), ..Default::default() }, &[], t0);
    let n = names(&a.frame(t0 + ms(3000)));
    assert!(n.contains(&"full/gesture".into()), "{n:?}");
    assert!(n.contains(&"prop/mug".into()) && !n.contains(&"prop/cake".into()), "장면이 덮는 자리만 숨김: {n:?}");
}

#[test]
fn desk_rules_for_characters() {
    let mut m = manifest(None);
    m.desk = None;
    let errs = m.validate_character(&[]);
    assert!(errs.iter().any(|e| e.contains("[desk] item")), "{errs:?}");
    m.desk = Some(Desk { item: Some("nope".into()), ..Default::default() });
    assert!(m.validate_character(&[]).iter().any(|e| e.contains("props.toml 에 없음")));
    m.desk = Some(Desk { item: Some("cake".into()), ..Default::default() });
    let errs = m.validate_character(&[]);
    assert!(!errs.iter().any(|e| e.contains("desk") || e.contains("prop cue")), "{errs:?}");
    m.behaviors.retain(|b| b.name != "t_poke");
    assert!(m.validate_character(&[]).iter().any(|e| e.contains("prop cue")));
}

// ---------- 애니에 없는 표정 막기 ----------

#[test]
fn heads_without_anime_evidence_are_never_drawn() {
    let t0 = Instant::now();
    let mut m = manifest(None);
    m.expressions.clear();
    m.expressions.insert("head/excited".into(), "e000-000".into());
    let mut a = Animator::new(&m, 240.0, t0);
    let mut at = t0;
    for head in Head::ALL {
        at += ms(3000);
        a.update(PetState { head, ..Default::default() }, &[], at);
        for dt in [0, 40, 120, 400, 2500] {
            for n in names(&a.frame(at + ms(dt))) {
                assert!(!n.contains("head/") || n == "head/normal" || n == "head/excited" || n.starts_with("mix:"), "{head:?} {n}");
                if let Some(mix) = n.strip_prefix("mix:") {
                    assert!(mix.split('>').all(|h| h == "head/normal" || h == "head/excited"), "{head:?} {n}");
                }
            }
            assert!(find(&a.frame(at + ms(dt)), "fx/sunglasses").is_none(), "선글라스 소품도 안 나온다");
        }
    }
    // behavior 의 머리 바꾸기도 막힌다 (t_nod: head/alt_a)
    assert!(a.force("t_nod", at + ms(5000)));
    let n = names(&a.frame(at + ms(5300)));
    assert!(!n.iter().any(|x| x.contains("alt_a")), "{n:?}");
    // 검사에서도 걸린다
    assert!(m.validate_character(&[]).iter().any(|e| e.contains("근거가 없어")));
}
