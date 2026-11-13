//! 트윈/이징, 크로스페이드, 연출 타이밍과 "이번 프레임에 그릴 레이어 목록" (PLAN 5.4, 6.2).
//!
//! 렌더러는 [`Animator::frame`] 이 만든 [`Frame`] 값을 그대로 그린다. 테스트는 이 값을 비교한다 (GPU 불필요).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::config::{Anim, Behavior, Breathe, Desk, FxKind, LayerMode, Manifest, Motion, MotionKind, MotionTarget, SceneFx, Step, Swing, When};
use crate::desk::{Catalog, Prop};
use crate::pet::{Cue, Head, PawL, PawR, PetEvent, PetState, Pose};

// ---------- 이징 ----------

pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// 살짝 넘쳤다가 돌아오는 곡선 (선글라스 착지).
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// 0 -> 1 -> 0 으로 튕기는 곡선 (팝, 누름). 빠르게 올라가고 천천히 내려온다.
pub fn bump(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.35 { ease_out(t / 0.35) } else { 1.0 - ease_in_out((t - 0.35) / 0.65) }
}

fn progress(now: Instant, start: Instant, dur_ms: u64) -> f32 {
    if dur_ms == 0 {
        return 1.0;
    }
    (now.saturating_duration_since(start).as_secs_f32() / (dur_ms as f32 / 1000.0)).min(1.0)
}

// ---------- 그릴 목록 ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DrawKind {
    Layer { name: String },
    /// 두 레이어를 (1-t, t) 로 섞어서 한 장처럼 그림 (크로스페이드).
    Mix { a: String, b: String, t: f32 },
    /// 하늘색 키 빛. 좌표/반경은 캔버스 px.
    Glow { center: [f32; 2], radius: f32, color: [f32; 4] },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawCmd {
    #[serde(flatten)]
    pub kind: DrawKind,
    pub opacity: f32,
    /// 캔버스 px 이동.
    pub offset: [f32; 2],
    pub scale: f32,
    /// 가로/세로 배율 (scale 에 곱함). squash 같은 납작해지는 움직임.
    #[serde(default = "unit2", skip_serializing_if = "is_unit2")]
    pub squash: [f32; 2],
    /// 확대 기준점 (캔버스 px).
    pub anchor: [f32; 2],
    /// 영역 마스크 레이어. 있으면 마스크 안을 이 레이어로 교체, 없으면 일반 over 합성.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<String>,
    /// 휘게 그리기 (마우스 연출). 마스크와 같이 쓰지 않는다.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warp: Option<Warp>,
}

/// root 에서 tip 으로 갈수록, 축에 가까울수록 `d` 만큼 밀어서 그린다 (캔버스 px).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Warp {
    pub root: [f32; 2],
    pub tip: [f32; 2],
    pub radius: [f32; 2],
    pub d: [f32; 2],
}

fn unit2() -> [f32; 2] {
    [1.0; 2]
}

fn is_unit2(v: &[f32; 2]) -> bool {
    *v == [1.0; 2]
}

impl DrawCmd {
    fn layer(name: &str) -> Self {
        Self {
            kind: DrawKind::Layer { name: name.into() },
            opacity: 1.0,
            offset: [0.0; 2],
            scale: 1.0,
            squash: [1.0; 2],
            anchor: [0.0; 2],
            mask: None,
            warp: None,
        }
    }
    /// 이 명령이 쓰는 레이어 이름들.
    pub fn layers(&self) -> Vec<&str> {
        match &self.kind {
            DrawKind::Layer { name } => vec![name.as_str()],
            DrawKind::Mix { a, b, .. } => vec![a.as_str(), b.as_str()],
            DrawKind::Glow { .. } => vec![],
        }
        .into_iter()
        .chain(self.mask.as_deref())
        .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Frame {
    pub cmds: Vec<DrawCmd>,
}

impl Frame {
    pub fn layer_names(&self) -> Vec<&str> {
        self.cmds.iter().flat_map(|c| c.layers()).collect()
    }
}

// ---------- 레이어 대체 ----------

/// 없는 레이어를 가장 가까운 레이어로 대체한다. None 이면 그리지 않는다 (base 에 이미 포함된 모습).
#[derive(Debug, Clone)]
pub struct Layers {
    have: BTreeSet<String>,
    /// 그려도 되는 머리 (애니 근거가 있는 표정). None 이면 제한 없음 (테스트용 [`Layers::new`]).
    heads: Option<BTreeSet<String>>,
}

impl Layers {
    pub fn new(have: BTreeSet<String>) -> Self {
        Self { have, heads: None }
    }
    /// manifest 의 `[expressions]` 에 없는 머리는 파일이 있어도 없는 것으로 본다 (head/normal 로 대체).
    pub fn from_manifest(m: &Manifest) -> Self {
        let heads = m.layers.iter().filter(|l| l.starts_with("head/") && m.allows_head(l)).cloned().collect();
        Self { have: m.layer_set(), heads: Some(heads) }
    }
    pub fn has(&self, name: &str) -> bool {
        self.have.contains(name)
    }
    /// 머리로 그릴 수 있는 레이어인가 (파일이 있고 표정이 허용됨).
    pub fn has_head(&self, name: &str) -> bool {
        self.have.contains(name) && self.heads.as_ref().is_none_or(|h| h.contains(name))
    }
    /// 머리 자리에 그릴 레이어. 허용되지 않은 표정은 가장 가까운 허용된 머리로 (보통 head/normal).
    pub fn resolve(&self, name: &str) -> Option<String> {
        let mut cur = name;
        loop {
            let ok = if cur.starts_with("head/") { self.has_head(cur) } else { self.have.contains(cur) };
            if ok {
                return Some(cur.to_string());
            }
            cur = match cur {
                "head/dizzy_wiggle" => "head/dizzy",
                "head/sunglasses" | "head/excited" | "head/dizzy" | "head/overheat" | "head/sleepy" => "head/normal",
                "paw_r/center" => "paw_r/down",
                "paw_r/mouse_click" => "paw_r/mouse",
                "paw_r/mouse" | "paw_r/down" => "paw_r/up",
                "paw_l/down" => "paw_l/up",
                _ => return None,
            };
        }
    }
}

// ---------- 변환 (움직임 도구) ----------

/// `p -> anchor + (p - anchor) * s + offset` (캔버스 px). 그리기 명령의 위치/배율과 같은 꼴.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xf {
    pub anchor: [f32; 2],
    pub offset: [f32; 2],
    pub s: [f32; 2],
}

impl Xf {
    pub fn identity(anchor: [f32; 2]) -> Self {
        Self { anchor, offset: [0.0; 2], s: [1.0; 2] }
    }
    pub fn is_identity(&self) -> bool {
        self.offset == [0.0; 2] && self.s == [1.0; 2]
    }
    /// 먼저 `inner`, 그다음 `self` 를 적용한 변환 (기준점은 inner 의 것).
    pub fn then_of(&self, inner: &Xf) -> Xf {
        let (o, i) = (self, inner);
        let mut r = Xf { anchor: i.anchor, offset: [0.0; 2], s: [i.s[0] * o.s[0], i.s[1] * o.s[1]] };
        for k in 0..2 {
            r.offset[k] = o.anchor[k] + (i.anchor[k] - o.anchor[k]) * o.s[k] + i.offset[k] * o.s[k] + o.offset[k] - i.anchor[k];
        }
        r
    }
    /// 그리기 명령에 덧씌운다.
    pub fn apply(&self, c: &mut DrawCmd) {
        if self.is_identity() {
            return;
        }
        let inner = Xf { anchor: c.anchor, offset: c.offset, s: [c.scale * c.squash[0], c.scale * c.squash[1]] };
        let r = self.then_of(&inner);
        c.anchor = r.anchor;
        c.offset = r.offset;
        if (r.s[0] - r.s[1]).abs() < 1e-6 {
            c.scale = r.s[0];
            c.squash = [1.0; 2];
        } else {
            c.scale = 1.0;
            c.squash = r.s;
        }
    }
    fn clamp_offset(mut self, max: f32) -> Self {
        for v in &mut self.offset {
            *v = v.clamp(-max, max);
        }
        self
    }
}

/// 0 -> 1 로 `ms` 동안 올라가는 가장자리 (반복 움직임이 뚝 끊기지 않게).
fn edge(t: f32, ms: f32) -> f32 {
    ease_in_out(t / ms)
}

/// `target` 에 걸린 움직임들의 합. `lt` 는 behavior 안 경과 ms, `len` 은 behavior 길이 (모르면 None).
pub fn motion_xf(motions: &[Motion], fx: &[SceneFx], target: MotionTarget, lt: f32, len: Option<f32>, anchor: [f32; 2], px_per_pt: f32) -> Xf {
    use std::f32::consts::PI;
    let mut x = Xf::identity(anchor);
    for m in motions.iter().filter(|m| m.resolve_target(fx) == Some(target)) {
        let t = lt - m.delay_ms as f32;
        let p = m.period() as f32;
        if t < 0.0 || (m.count > 0 && t >= m.count as f32 * p) {
            continue;
        }
        let u = fract(t / p);
        // 끝없이 반복하는 움직임은 시작과 끝 100ms 동안 서서히
        let env = if m.count == 0 { edge(t, 100.0) * len.map_or(1.0, |l| edge(l - lt, 100.0)) } else { 1.0 };
        let a = m.amp() * px_per_pt * env;
        let (mut dx, mut dy, mut k) = (0.0, 0.0, 0.0);
        match m.kind {
            MotionKind::Hop => {
                if u < 0.75 {
                    dy = -a * (PI * u / 0.75).sin();
                } else {
                    k = (PI * (u - 0.75) / 0.25).sin() * env;
                }
            }
            MotionKind::Tremble => {
                // 두 주파수를 섞어 기계적이지 않게 (합은 진폭 안)
                dx = a * (0.7 * (2.0 * PI * u).sin() + 0.3 * (2.0 * PI * 2.3 * t / p).sin());
            }
            MotionKind::Sway => dx = a * (2.0 * PI * u).sin(),
            MotionKind::Nod => dy = a * (PI * u).sin(),
            MotionKind::Squash => k = (PI * u).sin() * env,
        }
        x.offset[0] += dx;
        x.offset[1] += dy;
        if k != 0.0 {
            x.s[0] *= 1.0 + m.squash * k;
            x.s[1] *= 1.0 - m.squash * k;
        }
    }
    x
}

/// 소품 하나의 그리기 명령. `el` 은 장면/behavior 안 경과 ms, `w` 는 보이는 정도, `bob` 은 숨쉬기 오르내림 (px).
fn fx_cmd(x: &SceneFx, el: f32, w: f32, bob: f32, pt: f32) -> Option<DrawCmd> {
    if w <= 0.0 {
        return None;
    }
    let t = el - x.delay_ms as f32;
    if t < 0.0 {
        return None;
    }
    let period = x.period_ms.max(1) as f32;
    let u = fract(t / period + x.phase);
    let mut c = DrawCmd::layer(&x.layer);
    c.anchor = x.anchor;
    match x.kind {
        FxKind::Float => {
            // 아래에서 나타나 떠오르며 살짝 흔들리다 사라진다
            let fade = ease_in_out(u / 0.25) * (1.0 - ease_in_out((u - 0.6) / 0.4));
            c.opacity = w * fade;
            c.offset = [(u * std::f32::consts::TAU).sin() * 2.0 * pt, -u * x.rise_pt * pt];
            c.scale = 0.85 + 0.3 * u;
        }
        FxKind::Pulse => {
            let k = 0.5 + 0.5 * (u * std::f32::consts::TAU).sin();
            c.opacity = w;
            c.offset = [0.0, bob];
            c.scale = x.scale[0] + (x.scale[1] - x.scale[0]) * k;
        }
        FxKind::Fall => {
            // 맺혔다가 점점 빨리 떨어지며 사라진다
            let fall = ((u - 0.2) / 0.8).clamp(0.0, 1.0);
            c.opacity = w * ease_in_out(u / 0.15) * (1.0 - ease_in_out((u - 0.6) / 0.4));
            c.offset = [0.0, fall * fall * x.rise_pt * pt];
        }
        FxKind::PopIn => {
            if t >= period {
                return None;
            }
            let fade_out = 1.0 - ease_in_out((t - (period - 200.0).max(0.0)) / 200.0_f32.min(period));
            c.opacity = w * ease_in_out(t / 80.0) * fade_out;
            c.scale = (x.scale[0] + (x.scale[1] - x.scale[0]) * ease_out_back(t / 220.0)).max(0.01);
        }
        FxKind::Flash => {
            if t >= period {
                return None;
            }
            c.opacity = w * bump(t / period);
        }
    }
    (c.opacity > 0.0).then_some(c)
}

// ---------- 애니메이터 ----------

#[derive(Debug, Clone)]
struct HeadFade {
    from: String,
    start: Instant,
}

#[derive(Debug, Clone)]
struct Glow {
    center: [f32; 2],
    start: Instant,
}

/// 전체 프레임 재생기 (장면 단계 반복, 또는 한 장). 시간은 장면 안 경과 ms.
#[derive(Debug, Clone)]
struct Full {
    steps: Vec<Step>,
    cycle: u64,
    breathe: Option<Breathe>,
    fx: Vec<SceneFx>,
}

fn fract(x: f32) -> f32 {
    x - x.floor()
}

impl Full {
    fn new(steps: Vec<Step>) -> Self {
        let cycle = steps.iter().map(|s| s.ms.max(1)).sum::<u64>().max(1);
        Self { steps, cycle, breathe: None, fx: Vec::new() }
    }
    /// (지금 단계, 다음 단계로 블렌드 진행 0~1, 지금 단계 안에서 지난 ms).
    fn pos(&self, el: u64) -> (usize, f32, u64) {
        let mut t = el % self.cycle;
        for (i, s) in self.steps.iter().enumerate() {
            let ms = s.ms.max(1);
            if t < ms {
                let b = if s.blend_ms > 0 && t + s.blend_ms > ms {
                    ease_in_out((t + s.blend_ms - ms) as f32 / s.blend_ms as f32)
                } else {
                    0.0
                };
                return (i, b, t);
            }
            t -= ms;
        }
        (0, 0.0, 0)
    }
    fn next_frame(&self, i: usize) -> &str {
        &self.steps[(i + 1) % self.steps.len()].frame
    }
    /// 대표 프레임 (블렌드 중이면 더 많이 보이는 쪽).
    fn at(&self, el: u64) -> &str {
        let (i, b, _) = self.pos(el);
        if b > 0.5 { self.next_frame(i) } else { &self.steps[i].frame }
    }
    fn kind(&self, el: u64) -> DrawKind {
        let (i, b, _) = self.pos(el);
        let (a, n) = (&self.steps[i].frame, self.next_frame(i));
        if b > 0.0 && a != n {
            DrawKind::Mix { a: a.clone(), b: n.to_string(), t: b }
        } else {
            DrawKind::Layer { name: a.clone() }
        }
    }
    /// 숨쉬기/블렌드/소품이 있어 계속 다시 그려야 하는가.
    fn moving(&self) -> bool {
        self.breathe.is_some() || !self.fx.is_empty() || self.steps.iter().any(|s| s.blend_ms > 0)
    }
    /// 소품이 보일 정도: `on` 프레임이 보이는 비율.
    fn on_weight(&self, on: &[String], el: u64) -> f32 {
        if on.is_empty() {
            return 1.0;
        }
        let (i, b, _) = self.pos(el);
        let w = |f: &str| if on.iter().any(|o| o == f) { 1.0 } else { 0.0 };
        w(&self.steps[i].frame) * (1.0 - b) + w(self.next_frame(i)) * b
    }
    /// 숨쉬기 위상 -1~1 (없으면 0).
    fn breath(&self, el: u64) -> f32 {
        self.breathe.as_ref().map_or(0.0, |b| (el as f32 / b.period_ms.max(1) as f32 * std::f32::consts::TAU).sin())
    }
    /// 블렌드가 없는 장면의 다음 프레임 전환까지 남은 ms.
    fn next_flip(&self, el: u64) -> Option<u64> {
        if self.steps.len() < 2 {
            return None;
        }
        let (i, _, t) = self.pos(el);
        Some(self.steps[i].ms.max(1) - t)
    }
}

/// 재생 중인 behavior 하나 (또는 장면이 없는 pose 의 대체 한 장).
#[derive(Debug, Clone)]
struct Run {
    serial: u64,
    /// behaviors 의 번호. None 이면 대체 한 장.
    idx: Option<usize>,
    name: String,
    mode: LayerMode,
    priority: i32,
    start: Instant,
    /// 끝 시각. None 이면 pose/hold 조건이 끝날 때까지.
    end: Option<Instant>,
    /// pose 로 시작한 full 이면 그 pose (pose 가 바뀌면 끝).
    pose: Option<Pose>,
    /// hold: 조건 cue, 재생 ms (0 이면 계속), 쉬는 ms.
    hold: Option<(Cue, u64, u64)>,
    full: Option<Full>,
}

impl Run {
    fn el_ms(&self, now: Instant) -> u64 {
        now.saturating_duration_since(self.start).as_millis() as u64
    }
    /// 보이는 동안의 behavior 안 경과 ms. hold 가 쉬는 중이면 None.
    fn local(&self, now: Instant) -> Option<u64> {
        let el = self.el_ms(now);
        match self.hold {
            Some((_, d, c)) if d > 0 => {
                let ph = el % (d + c).max(1);
                (ph < d).then_some(ph)
            }
            _ => Some(el),
        }
    }
    /// 한 번 재생하는 길이 (ms). 조건이 끝날 때까지면 None.
    fn len_ms(&self) -> Option<u64> {
        match (self.hold, self.end) {
            (Some((_, d, _)), _) if d > 0 => Some(d),
            (_, Some(e)) => Some(e.saturating_duration_since(self.start).as_millis() as u64),
            _ => None,
        }
    }
    /// 다음에 보이기 시작하거나 멈추는 시각 (hold 의 재생/쉬기 경계, 끝).
    fn next_edge(&self, now: Instant) -> Option<Instant> {
        if let Some((_, d, c)) = self.hold {
            if d > 0 {
                let cyc = (d + c).max(1);
                let ph = self.el_ms(now) % cyc;
                let left = if ph < d { d - ph } else { cyc - ph };
                return Some(now + Duration::from_millis(left.max(1)));
            }
        }
        self.end.filter(|e| *e > now)
    }
}

/// 시각 상태. Pet 의 상태 변화와 사건을 받아 시간에 따른 연출로 바꾼다.
#[derive(Debug, Clone)]
pub struct Animator {
    a: Anim,
    layers: Layers,
    tips: BTreeMap<String, [f32; 2]>,
    masks: BTreeMap<String, String>,
    head_anchor: [f32; 2],
    body_anchor: [f32; 2],
    /// 마우스 연출 축. 레이어(paw_r/mouse_bg, fx/mouse_arm)가 없으면 None.
    swing: Option<Swing>,
    /// 캔버스 px / 표시 pt.
    px_per_pt: f32,
    pub breathing: bool,
    /// false 면 장면 cue(idle, milestone, gesture) 밖의 behavior 를 하지 않는다 (조합 시트, "반응 줄이기").
    pub cue_behaviors: bool,

    state: PetState,
    head_fade: Option<HeadFade>,
    dizzy_since: Instant,
    pop_start: Option<Instant>,
    /// 마우스 흔들기 시작 시각과 마지막 마우스 이동 시각.
    swing_start: Option<Instant>,
    last_move: Option<Instant>,
    glows: Vec<Glow>,
    /// 선글라스 낙하 시작 (머리가 Sunglasses 로 바뀐 시각).
    sun_drop: Option<Instant>,
    /// 선글라스 낙하 직전 머리 레이어 (낙하 중에 보이는 머리).
    sun_prev_head: String,

    behaviors: Vec<Behavior>,
    /// cue 별로 몇 번 했는지 (여러 behavior 를 돌아가며 고른다).
    cue_count: HashMap<Cue, usize>,
    /// behavior 별 마지막 시작 시각 (cooldown).
    last_start: HashMap<usize, Instant>,
    runs: Vec<Run>,
    serial: u64,
    rng: u64,
    /// 지금 보이는 full 과 그 직전 것 (바뀔 때 크로스페이드).
    shown_full: Option<Run>,
    prev_full: Option<Run>,
    pose_fade: Option<Instant>,
    /// 새로 시작한 behavior 이름 (트레이스용, drain_started 로 비운다).
    started: Vec<String>,

    /// 책상 소품 정의와 이 캐릭터의 자리 설정.
    catalog: Catalog,
    desk: Desk,
    job: Option<String>,
    /// 지금 책상에 놓인 소품 (직군 + 캐릭터).
    items: Vec<Prop>,
    /// 소품 id -> 누른 시각.
    taps: HashMap<String, Instant>,
}

impl Animator {
    pub fn new(m: &Manifest, size_pt: f32, now: Instant) -> Self {
        let ha = m.head_anchor;
        Self {
            a: m.anim.clone(),
            layers: Layers::from_manifest(m),
            tips: m.tips.clone(),
            masks: m.masks.iter().filter(|(_, v)| m.layers.contains(v)).map(|(k, v)| (k.clone(), v.clone())).collect(),
            head_anchor: ha,
            body_anchor: m.body_anchor.unwrap_or([ha[0], ha[1] + m.canvas as f32 * 0.25]),
            swing: m.swing.clone().filter(|_| ["paw_r/mouse_bg", "fx/mouse_arm"].iter().all(|l| m.layers.iter().any(|x| x == l))),
            px_per_pt: m.canvas as f32 / size_pt,
            breathing: false,
            cue_behaviors: true,
            state: PetState::default(),
            head_fade: None,
            dizzy_since: now,
            pop_start: None,
            swing_start: None,
            last_move: None,
            glows: Vec::new(),
            sun_drop: None,
            sun_prev_head: "head/normal".into(),
            behaviors: m.all_behaviors(),
            cue_count: HashMap::new(),
            last_start: HashMap::new(),
            runs: Vec::new(),
            serial: 0,
            rng: 0x2545_F491_4F6C_DD1D,
            shown_full: None,
            prev_full: None,
            pose_fade: None,
            started: Vec::new(),
            catalog: m.catalog.clone(),
            desk: m.desk.clone().unwrap_or_default(),
            job: m.job.clone(),
            items: Vec::new(),
            taps: HashMap::new(),
        }
        .with_items()
    }

    fn with_items(mut self) -> Self {
        self.refresh_items();
        self
    }

    fn refresh_items(&mut self) {
        let items = self.catalog.items(self.job.as_deref(), self.desk.item.as_deref());
        self.items = items.into_iter().filter(|p| self.layers.has(&p.layer())).collect();
        self.taps.retain(|id, _| self.items.iter().any(|p| &p.id == id));
    }

    /// 직군을 바꾼다 (없는 직군이면 캐릭터 소품만).
    pub fn set_job(&mut self, job: Option<&str>) {
        self.job = job.map(str::to_string);
        self.refresh_items();
    }

    /// 지금 책상에 놓인 소품.
    pub fn items(&self) -> &[Prop] {
        &self.items
    }

    /// 소품을 눌렀다: 소품이 반응하고 캐릭터에게 `prop` cue 를 알린다. 놓여 있지 않은 소품이면 false.
    pub fn tap(&mut self, id: &str, now: Instant) -> bool {
        if !self.items.iter().any(|p| p.id == id) {
            return false;
        }
        self.tick(now);
        self.taps.insert(id.to_string(), now);
        self.fire(Cue::Prop, None, now);
        true
    }

    fn tap_active(&self, now: Instant) -> bool {
        self.items.iter().any(|p| self.taps.get(&p.id).is_some_and(|&t| now.saturating_duration_since(t).as_millis() < p.tap_len() as u128))
    }

    /// 소품 그리기. `full` 이 보이는 장면 프레임이면 그 프레임이 덮는 자리는 건너뛴다.
    fn prop_cmds(&self, full: Option<&DrawKind>, alpha: f32, now: Instant) -> Vec<DrawCmd> {
        let hidden: Vec<&str> = match full {
            Some(DrawKind::Layer { name }) => vec![name.as_str()],
            Some(DrawKind::Mix { a, b, .. }) => vec![a.as_str(), b.as_str()],
            _ => vec![],
        }
        .into_iter()
        .filter_map(|f| self.desk.hide.get(f))
        .flatten()
        .map(String::as_str)
        .collect();
        let pt = self.px_per_pt;
        let mut v = Vec::new();
        for p in self.items.iter().filter(|p| !hidden.contains(&p.slot.as_str())) {
            let mut c = DrawCmd { opacity: alpha, anchor: p.foot, ..DrawCmd::layer(&p.layer()) };
            if let Some(&t) = self.taps.get(&p.id) {
                let lt = now.saturating_duration_since(t).as_millis() as f32;
                let len = p.tap_len() as f32;
                if lt < len {
                    let default_tap;
                    let tap: &[Motion] = if p.tap.is_empty() {
                        default_tap = [Motion { target: "body".into(), kind: MotionKind::Hop, pt: Some(3.5), period_ms: Some(380), count: 1, delay_ms: 0, squash: 0.1 }];
                        &default_tap
                    } else {
                        &p.tap
                    };
                    motion_xf(tap, &[], MotionTarget::Body, lt, Some(len), p.foot, pt).apply(&mut c);
                }
            }
            v.push(c);
        }
        v
    }

    pub fn set_size_pt(&mut self, canvas: u32, size_pt: f32) {
        self.px_per_pt = canvas as f32 / size_pt;
    }

    /// chance 판정 순서를 바꾼다.
    pub fn reseed(&mut self, seed: u64) {
        self.rng = seed | 1;
    }

    fn rand01(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn layers(&self) -> &Layers {
        &self.layers
    }

    pub fn state(&self) -> PetState {
        self.state
    }

    /// 지금 보이는 머리 레이어 이름 (dizzy 교대 반영, 대체 전 이름).
    fn head_layer(&self, head: Head, now: Instant) -> &'static str {
        if head == Head::Dizzy && self.layers.has_head("head/dizzy_wiggle") {
            let n = now.saturating_duration_since(self.dizzy_since).as_millis() / self.a.wiggle_ms.max(1) as u128;
            if n % 2 == 1 {
                return "head/dizzy_wiggle";
            }
        }
        head.layer()
    }

    /// 상태와 사건 반영.
    pub fn update(&mut self, state: PetState, events: &[PetEvent], now: Instant) {
        self.tick(now);
        let old = self.state;
        if state.head != old.head {
            let from = self.head_layer(old.head, now).to_string();
            if state.head == Head::Dizzy {
                self.dizzy_since = now;
            }
            if state.head == Head::Sunglasses && self.layers.has("fx/sunglasses") && self.layers.has_head("head/sunglasses") {
                // 선글라스가 위에서 내려온 뒤 머리를 크로스페이드
                self.sun_drop = Some(now);
                self.sun_prev_head = from;
                self.head_fade = None;
            } else {
                self.sun_drop = None;
                self.head_fade = Some(HeadFade { from, start: now });
            }
            if let (Some(a), Some(b)) = (old.head.rank(), state.head.rank()) {
                if b > a {
                    self.pop_start = Some(now);
                }
            }
            // 단계 cue: 이전 상태의 hold 는 끝내고, 새 상태의 enter/hold 를 시작
            let cue = Cue::of_head(state.head);
            self.runs.retain(|r| r.hold.is_none_or(|(c, ..)| c == cue));
            self.fire(cue, None, now);
            self.start_holds(cue, now);
        }
        if state.pose != old.pose {
            self.runs.retain(|r| r.pose.is_none() || r.pose != old.pose);
            if let Some(p) = state.pose {
                self.fire(Cue::of_pose(p), Some(p), now);
            }
        }
        for ev in events {
            match ev {
                PetEvent::PressL => self.add_glow(PawL::Down.layer(), now),
                PetEvent::PressR(pose) => self.add_glow(pose.layer(), now),
                PetEvent::MouseMove => {
                    if self.swing_env(now) <= 0.0 {
                        self.swing_start = Some(now);
                    }
                    self.last_move = Some(now);
                }
                PetEvent::Cue(c) => self.fire(*c, None, now),
            }
        }
        self.state = state;
        self.tick(now);
    }

    /// cue 에 반응: 이 cue 의 enter behavior 중 cooldown/chance 를 통과한 것을 돌아가며 하나.
    /// 장면 cue 인데 고를 게 없으면 레이어 한 장(full/gesture 등)으로 대신한다.
    fn fire(&mut self, cue: Cue, pose: Option<Pose>, now: Instant) {
        let mut cands = Vec::new();
        if self.cue_behaviors || pose.is_some() {
            for i in 0..self.behaviors.len() {
                let b = &self.behaviors[i];
                if b.on != cue || b.when != When::Enter {
                    continue;
                }
                let cd = Duration::from_secs_f32(b.cooldown_s);
                if self.last_start.get(&i).is_some_and(|&t| now.saturating_duration_since(t) < cd) {
                    continue;
                }
                let chance = b.chance;
                if chance < 1.0 && self.rand01() >= chance {
                    continue;
                }
                cands.push(i);
            }
        }
        if !cands.is_empty() {
            let c = self.cue_count.entry(cue).or_default();
            let i = cands[*c % cands.len()];
            *c += 1;
            if self.start_run(i, pose, None, now) {
                return;
            }
        }
        if let Some(p) = pose {
            if let Some(l) = self.layers.resolve(p.layer()) {
                self.push_run(Run {
                    serial: 0,
                    idx: None,
                    name: l.clone(),
                    mode: LayerMode::Full,
                    priority: 10,
                    start: now,
                    end: None,
                    pose: Some(p),
                    hold: None,
                    full: Some(Full::new(vec![Step { frame: l, ms: 1000, blend_ms: 0 }])),
                });
            }
        }
    }

    /// 이 상태인 동안 반복하는 behavior 들 (chance 는 들어갈 때 한 번).
    fn start_holds(&mut self, cue: Cue, now: Instant) {
        if !self.cue_behaviors {
            return;
        }
        for i in 0..self.behaviors.len() {
            let b = &self.behaviors[i];
            if b.on != cue || b.when != When::Hold || self.runs.iter().any(|r| r.idx == Some(i)) {
                continue;
            }
            let hold = (cue, b.duration_ms.unwrap_or(0), (b.cooldown_s * 1000.0) as u64);
            let chance = b.chance;
            if chance < 1.0 && self.rand01() >= chance {
                continue;
            }
            self.start_run(i, None, Some(hold), now);
        }
    }

    /// behavior 하나를 시작한다. full 인데 그릴 프레임이 없으면 false.
    fn start_run(&mut self, i: usize, pose: Option<Pose>, hold: Option<(Cue, u64, u64)>, now: Instant) -> bool {
        let b = &self.behaviors[i];
        let mode = b.mode();
        let full = match mode {
            LayerMode::Full => {
                let steps: Vec<Step> = b.timeline().into_iter().filter(|s| self.layers.has(&s.frame)).collect();
                if steps.is_empty() {
                    return false;
                }
                let mut f = Full::new(steps);
                f.breathe = b.breathe.clone();
                f.fx = b.fx.clone();
                Some(f)
            }
            LayerMode::Overlay => None,
        };
        // pose 로 시작한 full 은 pose 가 끝날 때, hold 는 조건이 끝날 때 끝난다
        let end = if hold.is_some() || (pose.is_some() && mode == LayerMode::Full) {
            None
        } else {
            Some(now + Duration::from_millis(b.duration_ms.unwrap_or(Behavior::DEFAULT_MS)))
        };
        let run = Run {
            serial: 0,
            idx: Some(i),
            name: b.name.clone(),
            mode,
            priority: b.priority,
            start: now,
            end,
            pose: if mode == LayerMode::Full { pose } else { None },
            hold,
            full,
        };
        self.last_start.insert(i, now);
        self.started.push(run.name.clone());
        self.push_run(run);
        true
    }

    fn push_run(&mut self, mut r: Run) {
        self.serial += 1;
        r.serial = self.serial;
        self.runs.push(r);
    }

    /// 조합 시트/검사용: 조건과 상관없이 behavior 를 바로 시작한다. 이름이 없으면 false.
    pub fn force(&mut self, name: &str, now: Instant) -> bool {
        let Some(i) = self.behaviors.iter().position(|b| b.name == name) else { return false };
        let b = &self.behaviors[i];
        let hold = (b.when == When::Hold).then(|| (b.on, b.duration_ms.unwrap_or(0), (b.cooldown_s * 1000.0) as u64));
        let pose = b.on.pose().filter(|_| hold.is_none());
        let ok = self.start_run(i, pose, hold, now);
        self.tick(now);
        ok
    }

    /// 끝난 run 정리, 보이는 full 이 바뀌었으면 크로스페이드 시작.
    fn tick(&mut self, now: Instant) {
        self.runs.retain(|r| r.end.is_none_or(|e| e > now));
        let cur = self.top(LayerMode::Full, now).map(|r| r.serial);
        if cur != self.shown_full.as_ref().map(|r| r.serial) {
            self.prev_full = self.shown_full.take();
            self.shown_full = self.top(LayerMode::Full, now).cloned();
            self.pose_fade = Some(now);
        }
    }

    /// 이 층에서 지금 보이는 run: 우선순위가 큰 쪽, 같으면 나중 것.
    fn top(&self, mode: LayerMode, now: Instant) -> Option<&Run> {
        self.runs.iter().filter(|r| r.mode == mode && r.end.is_none_or(|e| e > now) && r.local(now).is_some()).max_by_key(|r| (r.priority, r.serial))
    }

    fn behavior(&self, r: &Run) -> Option<&Behavior> {
        r.idx.map(|i| &self.behaviors[i])
    }

    /// 새로 시작한 behavior 이름들 (트레이스용).
    pub fn drain_started(&mut self) -> Vec<String> {
        std::mem::take(&mut self.started)
    }

    /// 지금 보이는 behavior 이름 (full, overlay 순).
    pub fn playing(&self, now: Instant) -> Vec<String> {
        [LayerMode::Full, LayerMode::Overlay].into_iter().filter_map(|m| self.top(m, now)).filter(|r| r.idx.is_some()).map(|r| r.name.clone()).collect()
    }

    /// 지금 재생 중인 장면 프레임 (테스트/추적용).
    pub fn full_layer(&self, now: Instant) -> Option<&str> {
        let r = self.top(LayerMode::Full, now)?;
        r.full.as_ref().map(|f| f.at(r.local(now).unwrap_or(0)))
    }

    /// 마우스 흔들기 세기 0~1: 시작할 때 swing_fade_ms 동안 커지고, 마지막 이동 뒤 swing_hold_ms 가 지나면 줄어든다.
    fn swing_env(&self, now: Instant) -> f32 {
        let (Some(start), Some(last)) = (self.swing_start, self.last_move) else { return 0.0 };
        let a = &self.a;
        let fade_in = ease_in_out(progress(now, start, a.swing_fade_ms));
        let stop = last + Duration::from_millis(a.swing_hold_ms);
        let fade_out = if now <= stop { 1.0 } else { 1.0 - ease_in_out(progress(now, stop, a.swing_fade_ms)) };
        fade_in * fade_out
    }

    fn swing_active(&self, now: Instant) -> bool {
        self.swing.is_some() && self.state.paw_r == PawR::Mouse && self.swing_env(now) > 0.0
    }

    fn add_glow(&mut self, pose_layer: &str, now: Instant) {
        if let Some(&c) = self.tips.get(pose_layer) {
            self.glows.push(Glow { center: c, start: now });
        }
    }

    /// 몸 전체 움직임. base_clean(캐릭터를 지운 책상) 이 없으면 드러나는 자리가 어색하지 않게 2pt 안으로.
    fn body_xf(&self, motion: &[Motion], fx: &[SceneFx], lt: f32, len: Option<f32>) -> Xf {
        let x = motion_xf(motion, fx, MotionTarget::Body, lt, len, self.body_anchor, self.px_per_pt);
        if self.layers.has("base_clean") { x } else { x.clamp_offset(2.0 * self.px_per_pt) }
    }

    /// 몸을 움직여 그린다: 제자리에 `rest`, 캐릭터를 지운 책상(있으면), 몸 마스크 안을 옮겨서 `kind`.
    /// 몸 마스크가 없으면 `rest` 자체를 옮긴다.
    fn body_cmds(&self, rest: DrawCmd, kind: DrawKind, body: &Xf) -> Vec<DrawCmd> {
        if body.is_identity() {
            return vec![rest];
        }
        let alpha = rest.opacity;
        let Some(bm) = self.masks.get("body") else {
            let mut c = rest;
            body.apply(&mut c);
            return vec![c];
        };
        let mut v = vec![rest];
        if self.layers.has("base_clean") {
            v.push(DrawCmd { opacity: alpha, mask: Some(bm.clone()), ..DrawCmd::layer("base_clean") });
        }
        let mut moved = DrawCmd { kind, opacity: alpha, mask: Some(bm.clone()), ..DrawCmd::layer("") };
        body.apply(&mut moved);
        v.push(moved);
        v
    }

    /// 앞발 그리기. 몸이 움직이고 `<g>_top`/`<g>_low` 마스크가 있으면 키보드 윗줄 아래(책상, 키보드 조각)는 제자리,
    /// 위(팔)는 몸과 같이 옮긴다. 없으면 앞발 전체를 제자리에.
    fn paw_cmds(&self, layer: &str, group: &str, body: &Xf) -> Vec<DrawCmd> {
        let split = (self.masks.get(&format!("{group}_low")), self.masks.get(&format!("{group}_top")));
        match split {
            (Some(low), Some(top)) if !body.is_identity() => {
                let rest = DrawCmd { mask: Some(low.clone()), ..DrawCmd::layer(layer) };
                let mut moved = DrawCmd { mask: Some(top.clone()), ..DrawCmd::layer(layer) };
                body.apply(&mut moved);
                vec![rest, moved]
            }
            _ => vec![DrawCmd { mask: self.masks.get(group).cloned(), ..DrawCmd::layer(layer) }],
        }
    }

    /// 이 시각에 그릴 목록.
    pub fn frame(&mut self, now: Instant) -> Frame {
        self.tick(now);
        self.glows.retain(|g| progress(now, g.start, self.a.glow_ms) < 1.0);
        let a = &self.a;
        let pt = self.px_per_pt;
        let st = self.state;

        // 특수 포즈 투명도
        let pose_p = self.pose_fade.map(|s| ease_in_out(progress(now, s, a.pose_fade_ms))).unwrap_or(1.0);
        let shown = self.top(LayerMode::Full, now);
        let (full_run, full_alpha) = match (shown, &self.prev_full) {
            (Some(r), _) => (Some(r), pose_p),
            (None, Some(r)) => (Some(r), 1.0 - pose_p),
            (None, None) => (None, 0.0),
        };

        let mut cmds = Vec::new();
        if full_alpha < 1.0 {
            // 얹는 behavior: 몸/머리 움직임, 머리 바꾸기, 소품
            let ov = self.top(LayerMode::Overlay, now).and_then(|r| Some((r, self.behavior(r)?, r.local(now)? as f32, r.len_ms().map(|l| l as f32))));
            let body = ov.map_or(Xf::identity(self.body_anchor), |(_, b, lt, len)| self.body_xf(&b.motion, &b.fx, lt, len));
            let head_m = ov.map_or(Xf::identity(self.head_anchor), |(_, b, lt, len)| {
                motion_xf(&b.motion, &b.fx, MotionTarget::Head, lt, len, self.head_anchor, pt)
            });
            cmds.extend(self.body_cmds(DrawCmd::layer("base"), DrawKind::Layer { name: "base".into() }, &body));
            // 앞발. 책상이 같이 움직이지 않도록 앞발 레이어는 제자리에 그린다 (몸이 움직이면 팔 부분만 따라감)
            if let Some(l) = self.layers.resolve(st.paw_l.layer()) {
                cmds.extend(self.paw_cmds(&l, "paw_l", &body));
            }
            match (&self.swing, st.paw_r) {
                (Some(sw), PawR::Mouse) => {
                    // 팔을 치운 바탕 위에 팔을 휘게 그린다 (멈춰 있으면 d = 0 이라 원래 그림과 같다)
                    let mut bg = DrawCmd::layer("paw_r/mouse_bg");
                    bg.mask = self.masks.get("paw_r").cloned();
                    cmds.push(bg);
                    let e = self.swing_env(now);
                    let t = self.swing_start.map_or(0.0, |s| now.saturating_duration_since(s).as_secs_f32());
                    let ph = t * a.swing_hz * std::f32::consts::TAU;
                    let d = [ph.sin() * a.swing_pt[0] * pt * e, (2.0 * ph).sin() * a.swing_pt[1] * pt * e];
                    let mut arm = DrawCmd::layer("fx/mouse_arm");
                    arm.warp = Some(Warp { root: sw.root, tip: sw.tip, radius: sw.radius, d });
                    cmds.push(arm);
                }
                _ => {
                    if let Some(l) = self.layers.resolve(st.paw_r.layer()) {
                        cmds.extend(self.paw_cmds(&l, "paw_r", &body));
                    }
                }
            }
            // 키 빛
            for g in &self.glows {
                let p = ease_out(progress(now, g.start, a.glow_ms));
                let r = (a.glow_r0_pt + (a.glow_r1_pt - a.glow_r0_pt) * p) * pt;
                cmds.push(DrawCmd {
                    kind: DrawKind::Glow { center: g.center, radius: r, color: a.glow_color },
                    opacity: 1.0 - p,
                    anchor: g.center,
                    ..DrawCmd::layer("")
                });
            }
            // 머리
            let mut scale = 1.0;
            if let Some(s) = self.pop_start {
                let p = progress(now, s, a.pop_ms);
                if p < 1.0 {
                    scale = 1.0 + (a.pop_scale - 1.0) * bump(p);
                }
            }
            let breathe = if self.breathing && self.is_still(now) {
                let t = now.elapsed_since_epoch_secs();
                (t * std::f32::consts::TAU / a.breathe_period_s).sin() * a.breathe_pt * pt
            } else {
                0.0
            };
            let mut head_cur = self.layers.resolve(self.head_layer(st.head, now));
            // 머리 바꾸기: 시작/끝 head_fade_ms 동안 원래 머리와 섞는다
            let mut swap: Option<(String, f32)> = None;
            if let Some((_, b, lt, len)) = ov {
                if let Some(h) = &b.head {
                    let fr = h.frames();
                    let name = fr[(lt as u64 / h.frame_ms.max(1)) as usize % fr.len()];
                    let fade = a.head_fade_ms.max(1) as f32;
                    let w = ease_in_out(lt / fade) * len.map_or(1.0, |l| ease_in_out((l - lt) / fade));
                    swap = self.layers.resolve(name).map(|n| (n, w));
                }
            }
            let mut fx: Option<DrawCmd> = None;
            let head_kind = if let Some(start) = self.sun_drop {
                // 1) 이전 머리 위로 선글라스가 내려옴 2) 머리를 head/sunglasses 로 크로스페이드, 선글라스는 사라짐
                let drop_p = progress(now, start, a.sunglasses_drop_ms);
                let prev = self.layers.resolve(&self.sun_prev_head);
                let mut fx_cmd = DrawCmd::layer("fx/sunglasses");
                fx_cmd.anchor = self.head_anchor;
                if drop_p < 1.0 {
                    fx_cmd.opacity = ease_out(drop_p);
                    fx_cmd.offset = [0.0, -(1.0 - ease_out_back(drop_p)) * a.sunglasses_drop_pt * pt];
                    fx = Some(fx_cmd);
                    prev.map(|name| DrawKind::Layer { name })
                } else {
                    let fade_start = start + Duration::from_millis(a.sunglasses_drop_ms);
                    let t = ease_in_out(progress(now, fade_start, a.head_fade_ms));
                    if t < 1.0 {
                        fx_cmd.opacity = 1.0 - t;
                        fx = Some(fx_cmd);
                    }
                    mix_or_layer(prev, head_cur, t)
                }
            } else if let Some((name, w)) = swap {
                mix_or_layer(head_cur.take(), Some(name), w)
            } else if let Some(f) = &self.head_fade {
                let t = ease_in_out(progress(now, f.start, a.head_fade_ms));
                mix_or_layer(self.layers.resolve(&f.from), head_cur, t)
            } else {
                head_cur.map(|name| DrawKind::Layer { name })
            };
            let head_xf = body.then_of(&head_m);
            if let Some(kind) = head_kind {
                let mask = self.masks.get("head").cloned();
                let mut c = DrawCmd { kind, offset: [0.0, breathe], scale, anchor: self.head_anchor, mask, ..DrawCmd::layer("") };
                head_xf.apply(&mut c);
                cmds.push(c);
            }
            if let Some(mut f) = fx {
                f.offset[1] += breathe;
                f.scale = scale;
                head_xf.apply(&mut f);
                cmds.push(f);
            }
            cmds.extend(self.prop_cmds(None, 1.0, now));
            if let Some((_, b, lt, len)) = ov {
                cmds.extend(self.behavior_fx(&b.motion, &b.fx, lt, len, |_| 1.0, 0.0, 1.0));
            }
        }
        if let Some(r) = full_run {
            if full_alpha > 0.0 {
                cmds.extend(self.full_cmds(r, full_alpha, now));
                let shown = r.full.as_ref().map(|f| f.kind(r.local(now).unwrap_or(0)));
                cmds.extend(self.prop_cmds(shown.as_ref(), full_alpha, now));
            }
        }
        Frame { cmds }
    }

    /// behavior 소품들 (fx:<이름> 움직임 포함).
    #[allow(clippy::too_many_arguments)]
    fn behavior_fx(&self, motion: &[Motion], fx: &[SceneFx], lt: f32, len: Option<f32>, on: impl Fn(&SceneFx) -> f32, bob: f32, alpha: f32) -> Vec<DrawCmd> {
        let pt = self.px_per_pt;
        let mut v = Vec::new();
        for (i, x) in fx.iter().enumerate() {
            if let Some(mut c) = fx_cmd(x, lt, on(x) * alpha, bob, pt) {
                motion_xf(motion, fx, MotionTarget::Fx(i), lt, len, x.anchor, pt).apply(&mut c);
                v.push(c);
            }
        }
        v
    }

    /// 장면 그리기: 프레임(블렌드), 몸/머리 움직임과 숨쉬기(마스크 안을 옮김), 소품.
    fn full_cmds(&self, r: &Run, alpha: f32, now: Instant) -> Vec<DrawCmd> {
        let Some(f) = &r.full else { return Vec::new() };
        let pt = self.px_per_pt;
        let el = r.local(now).unwrap_or(0);
        let lt = el as f32;
        let len = r.len_ms().map(|l| l as f32);
        let motion: &[Motion] = self.behavior(r).map_or(&[], |b| &b.motion);
        let kind = f.kind(el);
        let body = self.body_xf(motion, &f.fx, lt, len);
        let mut v = self.body_cmds(DrawCmd { kind: kind.clone(), opacity: alpha, ..DrawCmd::layer("") }, kind.clone(), &body);
        let s = f.breath(el);
        let bob = f.breathe.as_ref().map_or(0.0, |b| -s * b.pt * pt);
        let mut head = motion_xf(motion, &f.fx, MotionTarget::Head, lt, len, self.head_anchor, pt);
        head.offset[1] += bob;
        if let (Some(mask), true) = (self.masks.get("head"), f.breathe.is_some() || !head.is_identity()) {
            let mut c = DrawCmd { kind, opacity: alpha, anchor: self.head_anchor, mask: Some(mask.clone()), ..DrawCmd::layer("") };
            body.then_of(&head).apply(&mut c);
            v.push(c);
        }
        v.extend(self.behavior_fx(motion, &f.fx, lt, len, |x| f.on_weight(&x.on, el), bob, alpha));
        v
    }

    /// 머리/마우스/키 빛/behavior 연출이 진행 중이라 매 프레임(vsync) 그려야 하는가.
    pub fn animating(&self, now: Instant) -> bool {
        let a = &self.a;
        let live = |s: Option<Instant>, d: u64| s.is_some_and(|s| progress(now, s, d) < 1.0);
        self.swing_active(now)
            || live(self.pop_start, a.pop_ms)
            || self.head_fade.as_ref().is_some_and(|f| progress(now, f.start, a.head_fade_ms) < 1.0)
            || self.pose_fade.is_some_and(|s| progress(now, s, a.pose_fade_ms) < 1.0)
            || live(self.sun_drop, a.sunglasses_drop_ms + a.head_fade_ms)
            || !self.glows.iter().all(|g| progress(now, g.start, a.glow_ms) >= 1.0)
            || self.overlay_moving(now)
            || self.tap_active(now)
    }

    /// 보이는 overlay 가 움직이는 중인가 (움직임/소품, 또는 머리 바꾸기가 섞이는 중).
    fn overlay_moving(&self, now: Instant) -> bool {
        if self.top(LayerMode::Full, now).is_some() {
            return false;
        }
        let Some(r) = self.top(LayerMode::Overlay, now) else { return false };
        let Some(b) = self.behavior(r) else { return false };
        if b.moves() {
            return true;
        }
        let lt = r.local(now).unwrap_or(0);
        let fade = self.a.head_fade_ms;
        lt < fade || r.len_ms().is_some_and(|l| l.saturating_sub(lt) < fade)
    }

    /// 아무 연출도 없는 상태 (숨쉬기 대상).
    fn is_still(&self, now: Instant) -> bool {
        !self.animating(now) && self.top(LayerMode::Full, now).is_none() && self.top(LayerMode::Overlay, now).is_none()
    }

    /// 연속 애니메이션이 아닐 때 다음에 다시 그려야 하는 시각 (귀 흔들기 80ms, 숨쉬기 30fps, 장면 프레임, behavior 끝).
    pub fn next_redraw(&self, now: Instant) -> Option<Instant> {
        let mut t: Option<Instant> = None;
        let mut at = |x: Instant| t = Some(t.map_or(x, |y: Instant| y.min(x)));
        let tick = now + Duration::from_secs_f32(1.0 / self.a.breathe_fps.max(1.0));
        let full = self.top(LayerMode::Full, now);
        if self.state.head == Head::Dizzy && full.is_none() && self.layers.has_head("head/dizzy_wiggle") {
            let w = self.a.wiggle_ms.max(1) as u128;
            let el = now.saturating_duration_since(self.dizzy_since).as_millis();
            at(self.dizzy_since + Duration::from_millis(((el / w + 1) * w) as u64));
        }
        if let Some(r) = full {
            if let Some(f) = &r.full {
                let moving = f.moving() || self.behavior(r).is_some_and(|b| !b.motion.is_empty());
                let el = r.local(now).unwrap_or(0);
                if moving {
                    at(tick);
                } else if let Some(left) = f.next_flip(el) {
                    at(now + Duration::from_millis(left));
                }
            }
        }
        if full.is_none() {
            if let Some(r) = self.top(LayerMode::Overlay, now) {
                if let Some(h) = self.behavior(r).and_then(|b| b.head.as_ref()).filter(|h| h.frames().len() > 1) {
                    let el = r.local(now).unwrap_or(0);
                    let fm = h.frame_ms.max(1);
                    at(now + Duration::from_millis(fm - el % fm));
                }
            }
        }
        // behavior 끝, hold 의 재생/쉬기 경계
        for r in &self.runs {
            if let Some(e) = r.next_edge(now) {
                at(e);
            }
        }
        if self.breathing && self.is_still(now) {
            at(tick);
        }
        t.filter(|x| *x > now)
    }
}

fn mix_or_layer(a: Option<String>, b: Option<String>, t: f32) -> Option<DrawKind> {
    match (a, b) {
        (Some(a), Some(b)) if a != b && t < 1.0 => Some(DrawKind::Mix { a, b, t }),
        (_, Some(b)) => Some(DrawKind::Layer { name: b }),
        _ => None,
    }
}

/// 숨쉬기 위상은 앱 시작 이후 경과로 계산 (재현성을 위해 고정 원점).
trait EpochSecs {
    fn elapsed_since_epoch_secs(&self) -> f32;
}

impl EpochSecs for Instant {
    fn elapsed_since_epoch_secs(&self) -> f32 {
        use std::sync::OnceLock;
        static EPOCH: OnceLock<Instant> = OnceLock::new();
        let e = *EPOCH.get_or_init(|| *self);
        self.saturating_duration_since(e).as_secs_f32()
    }
}
