//! 트윈/이징, 크로스페이드, 연출 타이밍과 "이번 프레임에 그릴 레이어 목록" (PLAN 5.4, 6.2).
//!
//! 렌더러는 [`Animator::frame`] 이 만든 [`Frame`] 값을 그대로 그린다. 테스트는 이 값을 비교한다 (GPU 불필요).

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::config::{Anim, Manifest};
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
}

impl DrawCmd {
    fn layer(name: &str) -> Self {
        Self { kind: DrawKind::Layer { name: name.into() }, opacity: 1.0, offset: [0.0; 2], scale: 1.0, anchor: [0.0; 2], mask: None }
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

/// 시각 상태. Pet 의 상태 변화와 사건을 받아 시간에 따른 연출로 바꾼다.
#[derive(Debug, Clone)]
pub struct Animator {
    a: Anim,
    layers: Layers,
    tips: BTreeMap<String, [f32; 2]>,
    masks: BTreeMap<String, String>,
    head_anchor: [f32; 2],
    /// 캔버스 px / 표시 pt.
    px_per_pt: f32,
    pub breathing: bool,

    state: PetState,
    head_fade: Option<HeadFade>,
    dizzy_since: Instant,
    pop_start: Option<Instant>,
    press_l: Option<Instant>,
    press_r: Option<Instant>,
    glows: Vec<Glow>,
    /// 선글라스 낙하 시작 (머리가 Sunglasses 로 바뀐 시각).
    sun_drop: Option<Instant>,
    /// 선글라스 낙하 직전 머리 레이어 (낙하 중에 보이는 머리).
    sun_prev_head: String,
    pose_fade: Option<(Option<Pose>, Instant)>,
    prev_pose: Option<Pose>,
}

impl Animator {
    pub fn new(m: &Manifest, size_pt: f32, now: Instant) -> Self {
        Self {
            a: m.anim.clone(),
            layers: Layers::from_manifest(m),
            tips: m.tips.clone(),
            masks: m.masks.iter().filter(|(_, v)| m.layers.contains(v)).map(|(k, v)| (k.clone(), v.clone())).collect(),
            head_anchor: m.head_anchor,
            px_per_pt: m.canvas as f32 / size_pt,
            breathing: false,
            state: PetState::default(),
            head_fade: None,
            dizzy_since: now,
            pop_start: None,
            press_l: None,
            press_r: None,
            glows: Vec::new(),
            sun_drop: None,
            sun_prev_head: "head/normal".into(),
            pose_fade: None,
            prev_pose: None,
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
            self.pose_fade = Some((old.pose, now));
            self.prev_pose = old.pose;
        }
        for ev in events {
            match ev {
                PetEvent::PressL => {
                    self.press_l = Some(now);
                    self.add_glow(PawL::Down.layer(), now);
                }
                PetEvent::PressR(pose) => {
                    self.press_r = Some(now);
                    self.add_glow(pose.layer(), now);
                }
            }
        }
        self.state = state;
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
        let pose_now = st.pose.map(|p| p.layer()).and_then(|l| self.layers.resolve(l));
        let pose_prev = self.prev_pose.map(|p| p.layer()).and_then(|l| self.layers.resolve(l));
        let pose_p = self.pose_fade.as_ref().map(|(_, s)| ease_in_out(progress(now, *s, a.pose_fade_ms))).unwrap_or(1.0);
        let (full_layer, full_alpha) = match (&pose_now, &pose_prev) {
            (Some(l), _) => (Some(l.clone()), pose_p),
            (None, Some(l)) => (Some(l.clone()), 1.0 - pose_p),
            (None, None) => (None, 0.0),
        };

        let mut cmds = Vec::new();
        if full_alpha < 1.0 {
            cmds.push(DrawCmd::layer("base"));
            // 앞발
            let press = |start: Option<Instant>| -> f32 {
                start.map(|s| progress(now, s, a.press_ms)).filter(|p| *p < 1.0).map(bump).unwrap_or(0.0) * a.press_pt * pt
            };
            if let Some(l) = self.layers.resolve(st.paw_l.layer()) {
                let mut c = DrawCmd::layer(&l);
                c.mask = self.masks.get("paw_l").cloned();
                if st.paw_l == PawL::Down {
                    c.offset = [0.0, press(self.press_l)];
                }
                cmds.push(c);
            }
            if let Some(l) = self.layers.resolve(st.paw_r.layer()) {
                let mut c = DrawCmd::layer(&l);
                c.mask = self.masks.get("paw_r").cloned();
                if matches!(st.paw_r, PawR::Down | PawR::Center | PawR::MouseClick) {
                    c.offset = [0.0, press(self.press_r)];
                }
                cmds.push(c);
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
                cmds.push(DrawCmd { kind, opacity: 1.0, offset: [0.0, breathe], scale, anchor: self.head_anchor, mask });
            }
            if let Some(mut f) = fx {
                f.offset[1] += breathe;
                f.scale = scale;
                cmds.push(f);
            }
        }
        if let Some(l) = full_layer {
            if full_alpha > 0.0 {
                let mut c = DrawCmd::layer(&l);
                c.opacity = full_alpha;
                cmds.push(c);
            }
        }
        Frame { cmds }
    }

    /// 머리/앞발/키 빛 연출이 진행 중이라 매 프레임(vsync) 그려야 하는가.
    pub fn animating(&self, now: Instant) -> bool {
        let a = &self.a;
        let live = |s: Option<Instant>, d: u64| s.is_some_and(|s| progress(now, s, d) < 1.0);
        live(self.press_l, a.press_ms)
            || live(self.press_r, a.press_ms)
            || live(self.pop_start, a.pop_ms)
            || self.head_fade.as_ref().is_some_and(|f| progress(now, f.start, a.head_fade_ms) < 1.0)
            || self.pose_fade.as_ref().is_some_and(|f| progress(now, f.1, a.pose_fade_ms) < 1.0)
            || live(self.sun_drop, a.sunglasses_drop_ms + a.head_fade_ms)
            || !self.glows.iter().all(|g| progress(now, g.start, a.glow_ms) >= 1.0)
    }

    /// 아무 연출도 없는 상태 (숨쉬기 대상).
    fn is_still(&self, now: Instant) -> bool {
        !self.animating(now) && self.state.pose.is_none()
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
