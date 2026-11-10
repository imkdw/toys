//! 트윈/이징, 크로스페이드, 연출 타이밍과 "이번 프레임에 그릴 레이어 목록" (PLAN 5.4, 6.2).
//!
//! 렌더러는 [`Animator::frame`] 이 만든 [`Frame`] 값을 그대로 그린다. 테스트는 이 값을 비교한다 (GPU 불필요).

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::config::{Anim, Breathe, FxKind, Manifest, Scene, SceneFx, Step, Swing};
use crate::pet::{Head, PawL, PawR, PetEvent, PetState, Pose};

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

impl DrawCmd {
    fn layer(name: &str) -> Self {
        Self {
            kind: DrawKind::Layer { name: name.into() },
            opacity: 1.0,
            offset: [0.0; 2],
            scale: 1.0,
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
}

impl Layers {
    pub fn new(have: BTreeSet<String>) -> Self {
        Self { have }
    }
    pub fn from_manifest(m: &Manifest) -> Self {
        Self::new(m.layer_set())
    }
    pub fn has(&self, name: &str) -> bool {
        self.have.contains(name)
    }
    pub fn resolve(&self, name: &str) -> Option<String> {
        let mut cur = name;
        loop {
            if self.have.contains(cur) {
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

/// 지금 재생 중인 전체 프레임 연출 (장면 단계 반복, 또는 한 장).
#[derive(Debug, Clone)]
struct Full {
    steps: Vec<Step>,
    cycle: u64,
    start: Instant,
    breathe: Option<Breathe>,
    fx: Vec<SceneFx>,
}

fn fract(x: f32) -> f32 {
    x - x.floor()
}

impl Full {
    fn new(steps: Vec<Step>, start: Instant) -> Self {
        let cycle = steps.iter().map(|s| s.ms.max(1)).sum::<u64>().max(1);
        Self { steps, cycle, start, breathe: None, fx: Vec::new() }
    }
    fn el_ms(&self, now: Instant) -> u64 {
        now.saturating_duration_since(self.start).as_millis() as u64
    }
    /// (지금 단계, 다음 단계로 블렌드 진행 0~1, 지금 단계 안에서 지난 ms).
    fn pos(&self, now: Instant) -> (usize, f32, u64) {
        let mut t = self.el_ms(now) % self.cycle;
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
    fn at(&self, now: Instant) -> &str {
        let (i, b, _) = self.pos(now);
        if b > 0.5 { self.next_frame(i) } else { &self.steps[i].frame }
    }
    fn kind(&self, now: Instant) -> DrawKind {
        let (i, b, _) = self.pos(now);
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
    fn on_weight(&self, on: &[String], now: Instant) -> f32 {
        if on.is_empty() {
            return 1.0;
        }
        let (i, b, _) = self.pos(now);
        let w = |f: &str| if on.iter().any(|o| o == f) { 1.0 } else { 0.0 };
        w(&self.steps[i].frame) * (1.0 - b) + w(self.next_frame(i)) * b
    }
    /// 숨쉬기 위상 -1~1 (없으면 0).
    fn breath(&self, now: Instant) -> f32 {
        self.breathe.as_ref().map_or(0.0, |b| (self.el_ms(now) as f32 / b.period_ms.max(1) as f32 * std::f32::consts::TAU).sin())
    }
    /// 블렌드가 없는 장면의 다음 프레임 전환 시각.
    fn next_flip(&self, now: Instant) -> Option<Instant> {
        if self.steps.len() < 2 {
            return None;
        }
        let (_, _, t) = self.pos(now);
        let (i, _, _) = self.pos(now);
        let left = self.steps[i].ms.max(1) - t;
        Some(now + Duration::from_millis(left))
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
    /// 마우스 연출 축. 레이어(paw_r/mouse_bg, fx/mouse_arm)가 없으면 None.
    swing: Option<Swing>,
    /// 캔버스 px / 표시 pt.
    px_per_pt: f32,
    pub breathing: bool,

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
    scenes: BTreeMap<Pose, Vec<Scene>>,
    /// 발동 조건별로 몇 번 틀었는지 (여러 장면을 돌아가며 고른다).
    scene_count: BTreeMap<Pose, usize>,
    full: Option<Full>,
    prev_full: Option<Full>,
    pose_fade: Option<Instant>,
}

impl Animator {
    pub fn new(m: &Manifest, size_pt: f32, now: Instant) -> Self {
        Self {
            a: m.anim.clone(),
            layers: Layers::from_manifest(m),
            tips: m.tips.clone(),
            masks: m.masks.iter().filter(|(_, v)| m.layers.contains(v)).map(|(k, v)| (k.clone(), v.clone())).collect(),
            head_anchor: m.head_anchor,
            swing: m.swing.clone().filter(|_| ["paw_r/mouse_bg", "fx/mouse_arm"].iter().all(|l| m.layers.iter().any(|x| x == l))),
            px_per_pt: m.canvas as f32 / size_pt,
            breathing: false,
            state: PetState::default(),
            head_fade: None,
            dizzy_since: now,
            pop_start: None,
            swing_start: None,
            last_move: None,
            glows: Vec::new(),
            sun_drop: None,
            sun_prev_head: "head/normal".into(),
            scenes: {
                let mut v: BTreeMap<Pose, Vec<Scene>> = BTreeMap::new();
                for sc in &m.scenes {
                    v.entry(sc.trigger).or_default().push(sc.clone());
                }
                v
            },
            scene_count: BTreeMap::new(),
            full: None,
            prev_full: None,
            pose_fade: None,
        }
    }

    pub fn set_size_pt(&mut self, canvas: u32, size_pt: f32) {
        self.px_per_pt = canvas as f32 / size_pt;
    }

    pub fn layers(&self) -> &Layers {
        &self.layers
    }

    pub fn state(&self) -> PetState {
        self.state
    }

    /// 지금 보이는 머리 레이어 이름 (dizzy 교대 반영, 대체 전 이름).
    fn head_layer(&self, head: Head, now: Instant) -> &'static str {
        if head == Head::Dizzy && self.layers.has("head/dizzy_wiggle") {
            let n = now.saturating_duration_since(self.dizzy_since).as_millis() / self.a.wiggle_ms.max(1) as u128;
            if n % 2 == 1 {
                return "head/dizzy_wiggle";
            }
        }
        head.layer()
    }

    /// 상태와 사건 반영.
    pub fn update(&mut self, state: PetState, events: &[PetEvent], now: Instant) {
        let old = self.state;
        if state.head != old.head {
            let from = self.head_layer(old.head, now).to_string();
            if state.head == Head::Dizzy {
                self.dizzy_since = now;
            }
            if state.head == Head::Sunglasses && self.layers.has("fx/sunglasses") {
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
        }
        if state.pose != old.pose {
            self.pose_fade = Some(now);
            self.prev_full = self.full.take();
            self.full = state.pose.and_then(|p| self.start_full(p, now));
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
            }
        }
        self.state = state;
    }

    /// 연출 시작: 이 조건의 장면이 있으면 돌아가며 하나, 없으면 레이어 한 장(full/gesture).
    fn start_full(&mut self, p: Pose, now: Instant) -> Option<Full> {
        if let Some(list) = self.scenes.get(&p).filter(|l| !l.is_empty()) {
            let c = self.scene_count.entry(p).or_default();
            let sc = &list[*c % list.len()];
            *c += 1;
            let steps: Vec<Step> = sc.timeline().into_iter().filter(|s| self.layers.has(&s.frame)).collect();
            if !steps.is_empty() {
                let mut f = Full::new(steps, now);
                f.breathe = sc.breathe.clone();
                f.fx = sc.fx.iter().filter(|x| self.layers.has(&x.layer)).cloned().collect();
                return Some(f);
            }
        }
        self.layers.resolve(p.layer()).map(|l| Full::new(vec![Step { frame: l, ms: 1000, blend_ms: 0 }], now))
    }

    /// 지금 재생 중인 장면 프레임 (테스트/추적용).
    pub fn full_layer(&self, now: Instant) -> Option<&str> {
        self.full.as_ref().map(|f| f.at(now))
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

    /// 이 시각에 그릴 목록.
    pub fn frame(&mut self, now: Instant) -> Frame {
        self.glows.retain(|g| progress(now, g.start, self.a.glow_ms) < 1.0);
        let a = &self.a;
        let pt = self.px_per_pt;
        let st = self.state;

        // 특수 포즈 투명도
        let pose_p = self.pose_fade.map(|s| ease_in_out(progress(now, s, a.pose_fade_ms))).unwrap_or(1.0);
        let (full_layer, full_alpha) = match (&self.full, &self.prev_full) {
            (Some(f), _) => (Some(f), pose_p),
            (None, Some(f)) => (Some(f), 1.0 - pose_p),
            (None, None) => (None, 0.0),
        };

        let mut cmds = Vec::new();
        if full_alpha < 1.0 {
            cmds.push(DrawCmd::layer("base"));
            // 앞발. 책상이 같이 움직이지 않도록 앞발 레이어는 제자리에 그린다
            if let Some(l) = self.layers.resolve(st.paw_l.layer()) {
                let mut c = DrawCmd::layer(&l);
                c.mask = self.masks.get("paw_l").cloned();
                cmds.push(c);
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
                        let mut c = DrawCmd::layer(&l);
                        c.mask = self.masks.get("paw_r").cloned();
                        cmds.push(c);
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
                    offset: [0.0; 2],
                    scale: 1.0,
                    anchor: g.center,
                    mask: None,
                    warp: None,
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
            let head_cur = self.layers.resolve(self.head_layer(st.head, now));
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
            } else if let Some(f) = &self.head_fade {
                let t = ease_in_out(progress(now, f.start, a.head_fade_ms));
                mix_or_layer(self.layers.resolve(&f.from), head_cur, t)
            } else {
                head_cur.map(|name| DrawKind::Layer { name })
            };
            if let Some(kind) = head_kind {
                let mask = self.masks.get("head").cloned();
                cmds.push(DrawCmd { kind, opacity: 1.0, offset: [0.0, breathe], scale, anchor: self.head_anchor, mask, warp: None });
            }
            if let Some(mut f) = fx {
                f.offset[1] += breathe;
                f.scale = scale;
                cmds.push(f);
            }
        }
        if let Some(f) = full_layer {
            if full_alpha > 0.0 {
                cmds.extend(self.full_cmds(f, full_alpha, now));
            }
        }
        Frame { cmds }
    }

    /// 장면 그리기: 프레임(블렌드), 숨쉬기(머리 마스크 안을 오르내림), 소품.
    fn full_cmds(&self, f: &Full, alpha: f32, now: Instant) -> Vec<DrawCmd> {
        let pt = self.px_per_pt;
        let kind = f.kind(now);
        let mut v = vec![DrawCmd { kind: kind.clone(), opacity: alpha, ..DrawCmd::layer("") }];
        let s = f.breath(now);
        let bob = f.breathe.as_ref().map_or(0.0, |b| -s * b.pt * pt);
        if let (Some(mask), true) = (self.masks.get("head"), f.breathe.is_some()) {
            v.push(DrawCmd { kind, opacity: alpha, offset: [0.0, bob], anchor: self.head_anchor, mask: Some(mask.clone()), ..DrawCmd::layer("") });
        }
        for x in &f.fx {
            let w = f.on_weight(&x.on, now) * alpha;
            if w <= 0.0 {
                continue;
            }
            let u = fract(f.el_ms(now) as f32 / x.period_ms.max(1) as f32 + x.phase);
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
            }
            if c.opacity > 0.0 {
                v.push(c);
            }
        }
        v
    }

    /// 머리/마우스/키 빛 연출이 진행 중이라 매 프레임(vsync) 그려야 하는가.
    pub fn animating(&self, now: Instant) -> bool {
        let a = &self.a;
        let live = |s: Option<Instant>, d: u64| s.is_some_and(|s| progress(now, s, d) < 1.0);
        self.swing_active(now)
            || live(self.pop_start, a.pop_ms)
            || self.head_fade.as_ref().is_some_and(|f| progress(now, f.start, a.head_fade_ms) < 1.0)
            || self.pose_fade.is_some_and(|s| progress(now, s, a.pose_fade_ms) < 1.0)
            || live(self.sun_drop, a.sunglasses_drop_ms + a.head_fade_ms)
            || !self.glows.iter().all(|g| progress(now, g.start, a.glow_ms) >= 1.0)
    }

    /// 아무 연출도 없는 상태 (숨쉬기 대상).
    fn is_still(&self, now: Instant) -> bool {
        !self.animating(now) && self.full.is_none()
    }

    /// 연속 애니메이션이 아닐 때 다음에 다시 그려야 하는 시각 (귀 흔들기 80ms, 숨쉬기 30fps).
    pub fn next_redraw(&self, now: Instant) -> Option<Instant> {
        let mut t: Option<Instant> = None;
        if self.state.head == Head::Dizzy && self.state.pose.is_none() && self.layers.has("head/dizzy_wiggle") {
            let w = self.a.wiggle_ms.max(1) as u128;
            let el = now.saturating_duration_since(self.dizzy_since).as_millis();
            let next = self.dizzy_since + Duration::from_millis(((el / w + 1) * w) as u64);
            t = Some(next);
        }
        if let Some(full) = &self.full {
            let f = if full.moving() { Some(now + Duration::from_secs_f32(1.0 / self.a.breathe_fps.max(1.0))) } else { full.next_flip(now) };
            if let Some(f) = f {
                t = Some(t.map_or(f, |x| x.min(f)));
            }
        }
        if self.breathing && self.is_still(now) {
            let b = now + Duration::from_secs_f32(1.0 / self.a.breathe_fps.max(1.0));
            t = Some(t.map_or(b, |x| x.min(b)));
        }
        t
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
