//! 사용자 설정(위치/크기/캐릭터)과 캐릭터 manifest.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::pet::{Cue, Pose};

/// 사용자 설정. `~/Library/Application Support/typing-pet/config.toml` 에 저장.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// characters/ 아래 폴더 이름. None 이면 이름순 첫 번째 폴더 (`_` 로 시작하는 폴더 제외).
    pub character: Option<String>,
    /// 창 좌상단 위치 (논리 좌표). None 이면 주 화면 우하단.
    pub position: Option<(f64, f64)>,
    /// 표시 크기 (pt).
    pub size: f64,
    pub visible: bool,
    /// 가만히 있을 때 머리를 아주 천천히 위아래로.
    pub breathing: bool,
    /// 반응 줄이기: 장면(idle, milestone, gesture) 밖의 캐릭터 행동을 끈다.
    pub calm: bool,
    /// 직군 (characters/_props/props.toml 의 [[job]] id). 책상 위 장비가 바뀐다. None 이면 캐릭터 소품만.
    pub job: Option<String>,
    /// 입력 키 보기: 누른 키 이름이 머리 위에 구름 말풍선으로 떴다가 흩어진다. 켜면 창이 위로 조금 늘어난다.
    pub key_echo: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { character: None, position: None, size: 240.0, visible: true, breathing: true, calm: false, job: Some(DEFAULT_JOB.into()), key_echo: false }
    }
}

/// 처음 실행할 때의 직군.
pub const DEFAULT_JOB: &str = "basic";

pub const SIZES: [f64; 5] = [160.0, 200.0, 240.0, 320.0, 400.0];

/// 자유 크기 조절(스크롤/핀치) 범위 (pt).
pub const MIN_SIZE: f64 = 80.0;
pub const MAX_SIZE: f64 = 800.0;

/// 범위 안으로 넣고 1pt 단위로 맞춘다. 숫자가 아니면 기본 크기.
pub fn clamp_size(s: f64) -> f64 {
    if s.is_finite() { s.round().clamp(MIN_SIZE, MAX_SIZE) } else { Config::default().size }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("TYPING_PET_CONFIG") {
            return Some(PathBuf::from(p));
        }
        dirs::config_dir().map(|d| d.join("typing-pet").join("config.toml"))
    }

    /// 파일이 없거나 깨졌으면 기본값.
    pub fn load() -> Self {
        let Some(path) = Self::path() else { return Self::default() };
        let mut cfg: Self = match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                log::warn!("설정 파일이 깨져서 기본값 사용: {e}");
                Self::default()
            }),
            Err(_) => Self::default(),
        };
        cfg.size = clamp_size(cfg.size);
        cfg
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else { return Ok(()) };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)
    }
}

/// `characters/<name>/manifest.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub name: String,
    /// 메뉴에 보일 이름 (예: "우사기"). 없으면 폴더 이름.
    #[serde(default)]
    pub title: Option<String>,
    /// 캔버스 크기 (px). 모든 레이어가 이 크기의 정사각형이어야 한다.
    pub canvas: u32,
    /// layers/ 아래 PNG 이름 (확장자 제외). 예: "base", "head/normal", "paw_r/center".
    pub layers: Vec<String>,
    /// 머리 확대(팝)와 숨쉬기의 기준점 (캔버스 px). 보통 턱 아래 가운데.
    pub head_anchor: [f32; 2],
    /// 발 자세별 발끝 좌표 (캔버스 px). 키 빛을 그 자리에 그린다.
    #[serde(default)]
    pub tips: BTreeMap<String, [f32; 2]>,
    /// 레이어 묶음별 영역 마스크 (layers 안의 이름). 예: head = "mask/head".
    /// 마스크가 있는 레이어는 "마스크 안은 이 레이어로 교체" 로 그린다 (base 의 귀를 지울 수 있게).
    #[serde(default)]
    pub masks: BTreeMap<String, String>,
    /// 마우스 연출 축. 없으면 마우스 자세는 한 장으로 가만히 있다.
    #[serde(default)]
    pub swing: Option<Swing>,
    /// 애니 장면 연출 (`[[scene]]`). 발동 조건별로 여러 개면 돌아가며 쓴다.
    /// 읽을 때 `layer_mode = "full"` 인 behavior 로 바꿔 쓴다 ([`Manifest::all_behaviors`]).
    #[serde(default, rename = "scene")]
    pub scenes: Vec<Scene>,
    /// 캐릭터 고유 행동 (`[[behavior]]`). 엔진이 알린 cue 에 어떻게 반응할지.
    #[serde(default, rename = "behavior")]
    pub behaviors: Vec<Behavior>,
    /// 몸 전체 움직임(hop, squash 등)의 기준점 (캔버스 px). 보통 엉덩이 아래 가운데. 없으면 머리 기준점 아래.
    #[serde(default)]
    pub body_anchor: Option<[f32; 2]>,
    #[serde(default)]
    pub timing: Timing,
    #[serde(default)]
    pub anim: Anim,
    #[serde(default)]
    pub keys: Keys,
    /// 애니에 실제로 나온 표정만 그린다: `head/<이름>` -> 근거 (회차/프레임 id).
    /// 여기 없는 머리 레이어는 파일이 있어도 쓰지 않고 head/normal 로 그린다 (head/normal 은 항상 허용).
    #[serde(default)]
    pub expressions: BTreeMap<String, String>,
    /// 책상 소품 자리 (`[desk]`). 직군/캐릭터 소품을 이 자리에 놓는다.
    #[serde(default)]
    pub desk: Option<Desk>,

    #[serde(skip)]
    pub dir: PathBuf,
    /// 모든 캐릭터가 같이 쓰는 책상 소품 (`characters/_props`). 소품 레이어는 `prop/<id>` 로 layers 에 더한다.
    #[serde(skip)]
    pub catalog: crate::desk::Catalog,
    /// 처음 놓을 직군 소품 (앱은 설정의 직군, 렌더 CLI 는 `--job`). None 이면 캐릭터 소품만.
    #[serde(skip)]
    pub job: Option<String>,
}

/// 책상 소품 (`[desk]`). 소품 그림은 characters/_props 에서 모든 캐릭터가 같이 쓴다 (src/desk.rs).
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Desk {
    /// 이 캐릭터의 소품 (props.toml 의 id). 캐릭터 자리에 놓인다.
    #[serde(default)]
    pub item: Option<String>,
    /// 장면 프레임이 덮는 자리: 그 프레임이 보이는 동안 그 자리 소품을 그리지 않는다 (slice.py 가 계산).
    #[serde(default)]
    pub hide: BTreeMap<String, Vec<String>>,
}

/// 마우스를 움직이는 동안 fx/mouse_arm 을 휘게 그리는 축 (캔버스 px).
/// root(어깨) 에서는 안 움직이고 tip(손끝) 쪽으로 갈수록 크게 움직인다.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Swing {
    pub root: [f32; 2],
    pub tip: [f32; 2],
    /// 축에서 이 거리까지는 다 움직이고, `radius[1]` 밖은 안 움직인다.
    #[serde(default = "Swing::default_radius")]
    pub radius: [f32; 2],
}

impl Swing {
    fn default_radius() -> [f32; 2] {
        [45.0, 95.0]
    }
}

/// 애니메이션 장면 하나: 전체 프레임 레이어 여러 장을 `frame_ms` 마다 돌려 그린다.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub name: String,
    /// 발동 조건: gesture(과열이 식을 때), idle(한동안 입력 없음), milestone(키 N번째).
    pub trigger: Pose,
    /// full/ 레이어 이름들 (frame_ms 마다 순서대로 반복). `step` 이 있으면 그쪽을 쓴다.
    #[serde(default)]
    pub frames: Vec<String>,
    #[serde(default = "Scene::default_frame_ms")]
    pub frame_ms: u64,
    /// 장면 길이. 없으면 timing 값(gesture_ms, idle_scene_ms, milestone_ms).
    /// idle 장면만 0 을 쓸 수 있고, 0 이면 입력이 올 때까지 계속한다.
    #[serde(default)]
    pub duration_ms: Option<u64>,
    /// 단계별 프레임/길이/다음 단계로 넘어가는 크로스페이드 (반복).
    #[serde(default, rename = "step")]
    pub steps: Vec<Step>,
    /// 숨쉬기: 머리 마스크 안을 천천히 오르내린다.
    #[serde(default)]
    pub breathe: Option<Breathe>,
    /// 장면 위에 얹어 움직이는 소품 (Zzz, 콧방울 등).
    #[serde(default)]
    pub fx: Vec<SceneFx>,
}

impl Scene {
    fn default_frame_ms() -> u64 {
        400
    }

    /// 재생 순서. step 이 없으면 frames 를 frame_ms 마다.
    pub fn timeline(&self) -> Vec<Step> {
        if !self.steps.is_empty() {
            return self.steps.clone();
        }
        self.frames.iter().map(|f| Step { frame: f.clone(), ms: self.frame_ms, blend_ms: 0 }).collect()
    }

    /// 이 장면이 쓰는 레이어 이름들.
    pub fn layer_names(&self) -> Vec<&str> {
        self.frames.iter().map(|s| s.as_str()).chain(self.steps.iter().map(|s| s.frame.as_str())).chain(self.fx.iter().map(|f| f.layer.as_str())).collect()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub frame: String,
    pub ms: u64,
    /// 이 단계의 끝 blend_ms 동안 다음 단계 프레임으로 크로스페이드.
    #[serde(default)]
    pub blend_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Breathe {
    pub period_ms: u64,
    /// 오르내림 폭 (pt).
    #[serde(default = "Breathe::default_pt")]
    pub pt: f32,
}

impl Breathe {
    fn default_pt() -> f32 {
        1.5
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FxKind {
    /// anchor 에서 위로 떠오르며 나타났다 사라진다 (Zzz).
    Float,
    /// anchor 를 기준으로 커졌다 작아진다 (콧방울). 숨쉬기 오르내림도 따라간다.
    Pulse,
    /// delay_ms 뒤 커지며 튀어나와 period_ms 동안 보이다 사라진다 (말풍선). 반복하지 않는다.
    PopIn,
    /// anchor 에서 아래로 rise_pt 만큼 떨어지며 사라진다 (눈물, 땀). period_ms 마다 반복.
    Fall,
    /// delay_ms 뒤 period_ms 동안 한 번 번쩍 (카메라 플래시). 레이어는 보통 fx/white.
    Flash,
}

impl FxKind {
    /// 한 번만 나오는 종류 (반복하지 않음).
    pub fn one_shot(self) -> bool {
        matches!(self, FxKind::PopIn | FxKind::Flash)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SceneFx {
    pub layer: String,
    pub kind: FxKind,
    /// 움직임 기준점 (캔버스 px). pulse 는 붙어 있는 자리.
    pub anchor: [f32; 2],
    #[serde(default = "SceneFx::default_period")]
    pub period_ms: u64,
    /// 위상 (0~1). 여러 개를 엇갈리게 띄울 때.
    #[serde(default)]
    pub phase: f32,
    /// pop_in/flash: 나타나기 전 기다리는 시간. 다른 종류는 위상처럼 늦게 시작한다.
    #[serde(default)]
    pub delay_ms: u64,
    /// float: 한 주기 동안 떠오르는 높이, fall: 떨어지는 거리 (pt).
    #[serde(default = "SceneFx::default_rise")]
    pub rise_pt: f32,
    /// pulse: 가장 작을 때/클 때 배율.
    #[serde(default = "SceneFx::default_scale")]
    pub scale: [f32; 2],
    /// 이 프레임들이 보일 때만 보인다 (비면 항상). 블렌드 중에는 같이 흐려진다.
    #[serde(default)]
    pub on: Vec<String>,
}

impl SceneFx {
    fn default_period() -> u64 {
        2400
    }
    fn default_rise() -> f32 {
        18.0
    }
    fn default_scale() -> [f32; 2] {
        [0.4, 1.0]
    }
}

/// behavior 를 어디에 그리나.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayerMode {
    /// 평소 합성(앞발/머리) 위에 움직임/소품/머리 바꾸기를 얹는다.
    Overlay,
    /// 전체 프레임 장면 (기존 `[[scene]]`).
    Full,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum When {
    /// cue 가 나는 순간 한 번.
    #[default]
    Enter,
    /// 그 상태(level:*, egg)인 동안 duration_ms 재생, cooldown_s 쉬기를 반복. duration_ms 가 없으면 계속.
    Hold,
}

/// 캐릭터 고유 행동 하나 (`[[behavior]]`).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Behavior {
    pub name: String,
    /// 반응할 cue (`src/pet.rs` [`Cue`]).
    pub on: Cue,
    #[serde(default)]
    pub when: When,
    /// 0~1. cue 가 날 때 이 확률로만 한다.
    #[serde(default = "Behavior::default_chance")]
    pub chance: f32,
    /// 한 번 한 뒤 이만큼은 다시 하지 않는다. hold 는 재생 사이 쉬는 시간.
    #[serde(default)]
    pub cooldown_s: f32,
    /// 같은 층(full / overlay)에서 겹치면 큰 쪽을 보여 준다. 같으면 나중 것.
    #[serde(default = "Behavior::default_priority")]
    pub priority: i32,
    /// 없으면 frames/step 이 있을 때 full, 아니면 overlay.
    #[serde(default)]
    pub layer_mode: Option<LayerMode>,
    /// 길이. idle/milestone/gesture 의 full 은 timing 값이 기본이고, 나머지는 1200ms.
    #[serde(default)]
    pub duration_ms: Option<u64>,
    /// full: 프레임 (frame_ms 마다 반복). step 이 있으면 그쪽.
    #[serde(default)]
    pub frames: Vec<String>,
    #[serde(default = "Scene::default_frame_ms")]
    pub frame_ms: u64,
    #[serde(default, rename = "step")]
    pub steps: Vec<Step>,
    /// full: 머리 마스크 안 숨쉬기.
    #[serde(default)]
    pub breathe: Option<Breathe>,
    #[serde(default)]
    pub fx: Vec<SceneFx>,
    #[serde(default)]
    pub motion: Vec<Motion>,
    /// overlay: 머리 레이어를 잠깐 바꾼다.
    #[serde(default)]
    pub head: Option<HeadSwap>,
}

impl Behavior {
    fn default_chance() -> f32 {
        1.0
    }
    fn default_priority() -> i32 {
        10
    }
    pub const DEFAULT_MS: u64 = 1200;

    pub fn mode(&self) -> LayerMode {
        self.layer_mode.unwrap_or(if self.frames.is_empty() && self.steps.is_empty() { LayerMode::Overlay } else { LayerMode::Full })
    }

    /// 재생 순서. step 이 없으면 frames 를 frame_ms 마다.
    pub fn timeline(&self) -> Vec<Step> {
        if !self.steps.is_empty() {
            return self.steps.clone();
        }
        self.frames.iter().map(|f| Step { frame: f.clone(), ms: self.frame_ms, blend_ms: 0 }).collect()
    }

    /// 이 behavior 가 쓰는 레이어 이름들.
    pub fn layer_names(&self) -> Vec<&str> {
        self.frames
            .iter()
            .map(|s| s.as_str())
            .chain(self.steps.iter().map(|s| s.frame.as_str()))
            .chain(self.fx.iter().map(|f| f.layer.as_str()))
            .chain(self.head.iter().flat_map(|h| h.frames()))
            .collect()
    }

    /// 2장 교대만 하는 게 아니라 실제로 움직이는가 (motion, 소품, 숨쉬기, 크로스페이드).
    pub fn moves(&self) -> bool {
        !self.motion.is_empty() || !self.fx.is_empty() || self.breathe.is_some() || self.steps.iter().any(|s| s.blend_ms > 0)
    }

    /// 기존 `[[scene]]` 을 full behavior 로.
    pub fn from_scene(sc: &Scene) -> Self {
        Self {
            name: sc.name.clone(),
            on: Cue::of_pose(sc.trigger),
            when: When::Enter,
            chance: 1.0,
            cooldown_s: 0.0,
            priority: Self::default_priority(),
            layer_mode: Some(LayerMode::Full),
            duration_ms: sc.duration_ms,
            frames: sc.frames.clone(),
            frame_ms: sc.frame_ms,
            steps: sc.steps.clone(),
            breathe: sc.breathe.clone(),
            fx: sc.fx.clone(),
            motion: Vec::new(),
            head: None,
        }
    }
}

/// 움직임 도구. 모두 시간 함수라 프레임 수와 무관하다.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MotionKind {
    /// 위로 튀었다 내려오고 착지 때 살짝 납작해진다. pt = 높이.
    Hop,
    /// 가로로 작고 빠르게 떤다. pt = 진폭.
    Tremble,
    /// 좌우로 천천히 오간다. pt = 진폭.
    Sway,
    /// 아래로 끄덕였다 돌아온다. pt = 깊이.
    Nod,
    /// 기준점에서 세로로 줄고 가로로 늘었다 돌아온다. squash = 줄어드는 비율.
    Squash,
}

impl MotionKind {
    fn defaults(self) -> (f32, u64) {
        match self {
            MotionKind::Hop => (6.0, 450),
            MotionKind::Tremble => (1.2, 70),
            MotionKind::Sway => (2.5, 1200),
            MotionKind::Nod => (3.0, 500),
            MotionKind::Squash => (0.0, 300),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionTarget {
    /// 캐릭터 전체 (mask/body 안). 책상/키보드는 제자리.
    Body,
    /// 머리 마스크 안.
    Head,
    /// 이 behavior 의 소품 하나 (fx 의 순서).
    Fx(usize),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Motion {
    /// body | head | fx:<레이어 이름> (예: fx:fx/ura 또는 fx:ura).
    pub target: String,
    pub kind: MotionKind,
    /// 폭 (pt). 없으면 종류별 기본값.
    #[serde(default)]
    pub pt: Option<f32>,
    /// 한 번 하는 데 걸리는 시간. 없으면 종류별 기본값.
    #[serde(default)]
    pub period_ms: Option<u64>,
    /// 몇 번 할지. 0 이면 behavior 가 끝날 때까지 반복.
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub delay_ms: u64,
    /// hop 착지/squash 때 세로로 줄어드는 비율.
    #[serde(default = "Motion::default_squash")]
    pub squash: f32,
}

impl Motion {
    fn default_squash() -> f32 {
        0.05
    }
    pub fn amp(&self) -> f32 {
        self.pt.unwrap_or(self.kind.defaults().0)
    }
    pub fn period(&self) -> u64 {
        self.period_ms.unwrap_or(self.kind.defaults().1).max(1)
    }
    /// 대상 해석. fx 는 behavior 의 fx 목록 안에서 찾는다.
    pub fn resolve_target(&self, fx: &[SceneFx]) -> Option<MotionTarget> {
        match self.target.as_str() {
            "body" => Some(MotionTarget::Body),
            "head" => Some(MotionTarget::Head),
            t => {
                let name = t.strip_prefix("fx:")?;
                let full = if name.starts_with("fx/") { name.to_string() } else { format!("fx/{name}") };
                fx.iter().position(|f| f.layer == full).map(MotionTarget::Fx)
            }
        }
    }
}

/// overlay 동안 머리 레이어를 바꾼다. `layer` 한 장 또는 `frames` 를 frame_ms 마다.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HeadSwap {
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub frames: Vec<String>,
    #[serde(default = "HeadSwap::default_frame_ms")]
    pub frame_ms: u64,
}

impl HeadSwap {
    fn default_frame_ms() -> u64 {
        150
    }
    pub fn frames(&self) -> Vec<&str> {
        self.layer.iter().map(|s| s.as_str()).chain(self.frames.iter().map(|s| s.as_str())).collect()
    }
}

/// `validate_character` 의 숫자 (계획 5.2).
pub const MIN_BASIC_PER_CUE: usize = 1;
pub const MIN_EXTRA_BEHAVIORS: usize = 3;
pub const MIN_AMBIENT_S: u64 = 60;

/// 상태 머신 타이밍 (PLAN 5.3).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Timing {
    pub paw_down_ms: u64,
    pub click_ms: u64,
    pub mouse_hold_ms: u64,
    pub mouse_throttle_ms: u64,
    pub half_life_s: f64,
    pub min_dwell_ms: u64,
    /// excited, dizzy, overheat 로 올라가는 점수.
    pub level_up: [f64; 3],
    /// excited, dizzy, overheat 에서 내려오는 점수.
    pub level_down: [f64; 3],
    pub sleepy_s: u64,
    pub gesture: bool,
    pub gesture_ms: u64,
    pub gesture_cooldown_s: u64,
    /// 입력이 이만큼 없으면 idle 장면 (0 이면 끔). 한 번 쉬는 동안 한 번만.
    pub idle_scene_s: u64,
    pub idle_scene_ms: u64,
    /// 키를 이만큼 칠 때마다 milestone 장면 (0 이면 끔).
    pub milestone_keys: u64,
    pub milestone_ms: u64,
    pub sunglasses_ms: u64,
    pub easter_egg_repeat: u32,
    /// 이만큼 쉬다가 입력하면 wake cue (0 이면 끔).
    pub wake_after_s: u64,
    /// burst_window_ms 안에 burst_keys 번 치면 burst cue (burst_keys = 0 이면 끔).
    pub burst_window_ms: u64,
    pub burst_keys: u32,
    /// ambient cue 간격 범위 (초). 두 값 사이 무작위. 뒤 값이 0 이면 끔.
    pub ambient_every_s: [u64; 2],
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            paw_down_ms: 120,
            click_ms: 120,
            mouse_hold_ms: 1000,
            mouse_throttle_ms: 100,
            half_life_s: 3.0,
            min_dwell_ms: 1500,
            level_up: [12.0, 22.0, 32.0],
            level_down: [6.0, 14.0, 22.0],
            sleepy_s: 60,
            gesture: true,
            gesture_ms: 2000,
            gesture_cooldown_s: 300,
            idle_scene_s: 20,
            idle_scene_ms: 6000,
            milestone_keys: 300,
            milestone_ms: 2500,
            sunglasses_ms: 30_000,
            easter_egg_repeat: 10,
            wake_after_s: 15,
            burst_window_ms: 1000,
            burst_keys: 9,
            ambient_every_s: [60, 180],
        }
    }
}

/// 연출 값 (PLAN 5.4). 길이 단위 pt 는 화면 표시 기준.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Anim {
    pub head_fade_ms: u64,
    pub pop_ms: u64,
    pub pop_scale: f32,
    pub glow_ms: u64,
    pub glow_r0_pt: f32,
    pub glow_r1_pt: f32,
    /// 하늘색 (straight RGBA, 0~1).
    pub glow_color: [f32; 4],
    pub wiggle_ms: u64,
    pub sunglasses_drop_ms: u64,
    pub sunglasses_drop_pt: f32,
    pub pose_fade_ms: u64,
    pub breathe_period_s: f32,
    pub breathe_pt: f32,
    pub breathe_fps: f32,
    /// 마우스 연출: 손끝 흔들림 폭 (pt, 가로/세로), 초당 왕복 수.
    pub swing_pt: [f32; 2],
    pub swing_hz: f32,
    /// 마지막 마우스 이동 뒤 이만큼은 계속 흔들고, 이후 `swing_fade_ms` 동안 멈춘다.
    pub swing_hold_ms: u64,
    pub swing_fade_ms: u64,
}

impl Default for Anim {
    fn default() -> Self {
        Self {
            head_fade_ms: 250,
            pop_ms: 200,
            pop_scale: 1.04,
            glow_ms: 220,
            glow_r0_pt: 10.0,
            glow_r1_pt: 16.0,
            glow_color: [0.45, 0.85, 1.0, 0.85],
            wiggle_ms: 80,
            sunglasses_drop_ms: 400,
            sunglasses_drop_pt: 30.0,
            pose_fade_ms: 250,
            breathe_period_s: 3.0,
            breathe_pt: 1.5,
            breathe_fps: 30.0,
            swing_pt: [4.0, 1.2],
            swing_hz: 3.5,
            swing_hold_ms: 200,
            swing_fade_ms: 150,
        }
    }
}

/// 키 위치 판정 (macOS 가상 키 코드, 물리 위치 기준). 목록에 없는 키는 오른쪽.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Keys {
    pub left: Vec<u16>,
    pub center: Vec<u16>,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            left: vec![
                50, 18, 19, 20, 21, 23, // ` 1 2 3 4 5
                48, 12, 13, 14, 15, 17, // Tab Q W E R T
                57, 0, 1, 2, 3, 5, // Caps A S D F G
                56, 6, 7, 8, 9, 11, // LShift Z X C V B
                59, 58, 55, // LCtrl LOption LCmd
                63, 53, // Fn, Esc
            ],
            center: vec![22, 26, 16, 32, 4, 38, 45, 46, 49], // 6 7 Y U H J N M Space
        }
    }
}

impl Manifest {
    /// 캐릭터 폴더 루트. `TYPING_PET_CHARACTERS` > .app 번들 Resources > 소스 트리 순.
    pub fn characters_dir() -> PathBuf {
        if let Ok(p) = std::env::var("TYPING_PET_CHARACTERS") {
            return PathBuf::from(p);
        }
        if let Ok(exe) = std::env::current_exe() {
            // TypingPet.app/Contents/MacOS/typing-pet -> Contents/Resources/characters
            if let Some(res) = exe.parent().and_then(|p| p.parent()).map(|p| p.join("Resources/characters")) {
                if res.is_dir() {
                    return res;
                }
            }
        }
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("characters")
    }

    /// 고를 수 있는 캐릭터 (폴더 이름, 메뉴 이름) 목록. manifest.toml 이 있고 `_` 로 시작하지 않는 폴더, 메뉴 이름순.
    pub fn available() -> Vec<(String, String)> {
        Self::available_in(&Self::characters_dir())
    }

    pub fn available_in(root: &Path) -> Vec<(String, String)> {
        let Ok(rd) = std::fs::read_dir(root) else { return Vec::new() };
        let mut v: Vec<(String, String)> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.join("manifest.toml").is_file())
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .filter(|n| !n.starts_with('_'))
            .map(|n| {
                // 메뉴 이름만 필요하므로 title 줄만 가볍게 읽는다 (깨진 manifest 도 목록에는 보이게)
                #[derive(Deserialize)]
                struct Head {
                    title: Option<String>,
                }
                let title = std::fs::read_to_string(root.join(&n).join("manifest.toml"))
                    .ok()
                    .and_then(|t| toml::from_str::<toml::Table>(&t).ok())
                    .and_then(|t| t.try_into::<Head>().ok())
                    .and_then(|h| h.title)
                    .unwrap_or_else(|| n.clone());
                (n, title)
            })
            .collect();
        v.sort_by(|a, b| (&a.1, &a.0).cmp(&(&b.1, &b.0)));
        v
    }

    /// name 이 None 이면 `available()` 의 첫 번째 (메뉴 이름순, `_` 로 시작하는 테스트 폴더 제외).
    pub fn load(name: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let root = Self::characters_dir();
        let dir = match name {
            Some(name) => root.join(name),
            None => {
                let (first, _) = Self::available_in(&root)
                    .into_iter()
                    .next()
                    .ok_or("characters/ 아래에 manifest.toml 이 있는 폴더가 없음")?;
                root.join(first)
            }
        };
        Self::load_dir(&dir)
    }

    pub fn load_dir(dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(dir.join("manifest.toml"))
            .map_err(|e| format!("{}: {e}", dir.join("manifest.toml").display()))?;
        let mut manifest: Manifest = toml::from_str(&text)?;
        manifest.dir = dir.to_path_buf();
        if let Some(root) = dir.parent() {
            manifest.catalog = crate::desk::Catalog::load_in(root)?;
            for p in &manifest.catalog.props {
                if manifest.catalog.layer_path(&p.id).is_file() {
                    manifest.layers.push(p.layer());
                }
            }
        }
        manifest.validate()?;
        manifest.apply_scenes();
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.layers.iter().any(|l| l == "base") {
            return Err("layers 에 base 가 없음".into());
        }
        let t = &self.timing;
        for i in 0..3 {
            if t.level_down[i] >= t.level_up[i] {
                return Err(format!("level_down[{i}] 은 level_up[{i}] 보다 작아야 함 (히스테리시스)"));
            }
        }
        if t.level_up.windows(2).any(|w| w[0] >= w[1]) {
            return Err("level_up 은 증가해야 함".into());
        }
        if t.half_life_s <= 0.0 {
            return Err("half_life_s 는 0 보다 커야 함".into());
        }
        for sc in &self.scenes {
            let tl = sc.timeline();
            if tl.is_empty() {
                return Err(format!("장면 {} 에 frames 나 step 이 없음", sc.name));
            }
            if let Some(f) = sc.layer_names().into_iter().find(|f| !self.layers.iter().any(|l| l == f)) {
                return Err(format!("장면 {} 의 레이어 {f} 이 layers 에 없음", sc.name));
            }
            if tl.iter().any(|s| s.ms == 0 || s.blend_ms > s.ms) {
                return Err(format!("장면 {} 의 단계 길이는 0 보다 크고 blend_ms 이상이어야 함", sc.name));
            }
            if sc.duration_ms == Some(0) && sc.trigger != Pose::Idle {
                return Err(format!("장면 {}: duration_ms = 0 (입력까지 계속)은 idle 장면만 쓸 수 있음", sc.name));
            }
            if sc.breathe.as_ref().is_some_and(|b| b.period_ms == 0) || sc.fx.iter().any(|f| f.period_ms == 0) {
                return Err(format!("장면 {} 의 period_ms 는 0 보다 커야 함", sc.name));
            }
        }
        for b in &self.behaviors {
            self.validate_behavior(b).map_err(|e| format!("behavior {}: {e}", b.name))?;
        }
        if let Some(d) = &self.desk {
            if let Some(f) = d.hide.keys().find(|f| !self.layers.iter().any(|l| l == *f)) {
                return Err(format!("[desk.hide] 의 프레임 {f} 이 layers 에 없음"));
            }
        }
        let [lo, hi] = t.ambient_every_s;
        if hi > 0 && lo > hi {
            return Err("ambient_every_s 는 [작은 값, 큰 값] 이어야 함".into());
        }
        Ok(())
    }

    fn validate_behavior(&self, b: &Behavior) -> Result<(), String> {
        if let Some(f) = b.layer_names().into_iter().find(|f| !self.layers.iter().any(|l| l == f)) {
            return Err(format!("레이어 {f:?} 이 layers 에 없음"));
        }
        if !(0.0..=1.0).contains(&b.chance) || !b.cooldown_s.is_finite() || b.cooldown_s < 0.0 {
            return Err("chance 는 0~1, cooldown_s 는 0 이상".into());
        }
        if b.when == When::Hold && !b.on.can_hold() {
            return Err(format!("when = \"hold\" 는 level:*, egg cue 만 쓸 수 있음 ({})", b.on));
        }
        if b.duration_ms == Some(0) && !(b.on == Cue::Idle || b.when == When::Hold) {
            return Err("duration_ms = 0 (끝없이)은 idle 과 hold 만 쓸 수 있음".into());
        }
        let tl = b.timeline();
        match b.mode() {
            LayerMode::Full => {
                if tl.is_empty() {
                    return Err("full 인데 frames 나 step 이 없음".into());
                }
                if b.head.is_some() {
                    return Err("head 바꾸기는 overlay 만 쓸 수 있음".into());
                }
            }
            LayerMode::Overlay => {
                if !tl.is_empty() || b.breathe.is_some() {
                    return Err("overlay 에는 frames/step/breathe 를 쓸 수 없음 (layer_mode = \"full\" 로)".into());
                }
                if b.motion.is_empty() && b.fx.is_empty() && b.head.is_none() {
                    return Err("overlay 인데 motion/fx/head 가 하나도 없음".into());
                }
            }
        }
        if tl.iter().any(|s| s.ms == 0 || s.blend_ms > s.ms) {
            return Err("단계 길이는 0 보다 크고 blend_ms 이상이어야 함".into());
        }
        if b.breathe.as_ref().is_some_and(|x| x.period_ms == 0) || b.fx.iter().any(|f| f.period_ms == 0) {
            return Err("period_ms 는 0 보다 커야 함".into());
        }
        if b.fx.iter().any(|f| f.layer.is_empty()) {
            return Err("fx 에 layer 가 없음".into());
        }
        for m in &b.motion {
            if m.resolve_target(&b.fx).is_none() {
                return Err(format!("motion target {:?} 을 모름 (body, head, fx:<이 behavior 의 fx 레이어>)", m.target));
            }
            if m.amp() < 0.0 || !m.amp().is_finite() || !(0.0..0.5).contains(&m.squash) {
                return Err(format!("motion {:?}: pt 는 0 이상, squash 는 0~0.5", m.kind));
            }
        }
        if let Some(h) = &b.head {
            if h.frames().is_empty() || h.frame_ms == 0 {
                return Err("head 에 layer 나 frames 가 없음".into());
            }
        }
        Ok(())
    }

    /// 명시한 `[[behavior]]` 와 `[[scene]]` 을 바꾼 것 전부. scene 이 먼저 (기존 순서 유지).
    pub fn all_behaviors(&self) -> Vec<Behavior> {
        self.scenes.iter().map(Behavior::from_scene).chain(self.behaviors.iter().cloned()).collect()
    }

    /// behavior 가 없는 장면 cue(idle, milestone)는 끄고, 첫 full behavior 길이를 타이밍에 반영한다.
    pub fn apply_scenes(&mut self) {
        let all = self.all_behaviors();
        let any = |c: Cue| all.iter().any(|b| b.on == c);
        let first = |c: Cue| all.iter().find(|b| b.on == c && b.mode() == LayerMode::Full).and_then(|b| b.duration_ms);
        let t = &mut self.timing;
        if !any(Cue::Idle) {
            t.idle_scene_s = 0;
        } else if let Some(d) = first(Cue::Idle) {
            t.idle_scene_ms = d;
        }
        if !any(Cue::Milestone) {
            t.milestone_keys = 0;
        } else if let Some(d) = first(Cue::Milestone) {
            t.milestone_ms = d;
        }
        if let Some(d) = first(Cue::Gesture) {
            t.gesture_ms = d;
        }
    }

    /// 새 캐릭터 규칙 (계획 5.2). 고유 행동이 모자라면 이유를 돌려준다 (비면 통과).
    /// `others` 는 비교할 다른 캐릭터들 (behavior 이름, 레이어 파일이 겹치면 안 됨).
    pub fn validate_character(&self, others: &[Manifest]) -> Vec<String> {
        let mut errs = Vec::new();
        if let Err(e) = self.validate() {
            errs.push(e);
        }
        let all = self.all_behaviors();
        for c in [Cue::Idle, Cue::Milestone, Cue::Gesture] {
            let n = all.iter().filter(|b| b.on == c).count();
            if n < MIN_BASIC_PER_CUE {
                errs.push(format!("{c} cue 에 behavior 가 {n}개 (최소 {MIN_BASIC_PER_CUE}개)"));
            }
        }
        let extra = all.iter().filter(|b| b.on.pose().is_none()).count();
        if extra < MIN_EXTRA_BEHAVIORS {
            errs.push(format!("idle/milestone/gesture 밖의 cue (wake, burst, level:*, egg, ambient, key:*) 에 behavior 가 {extra}개 (최소 {MIN_EXTRA_BEHAVIORS}개)"));
        }
        let moving = all.iter().filter(|b| b.moves()).count();
        if moving * 2 < all.len() {
            errs.push(format!("움직임(motion/fx/숨쉬기/크로스페이드)이 있는 behavior 가 {moving}/{}개 (절반 이상이어야 함)", all.len()));
        }
        let [_, hi] = self.timing.ambient_every_s;
        if hi > 0 && self.timing.ambient_every_s[0] < MIN_AMBIENT_S {
            errs.push(format!("ambient_every_s 최소 간격은 {MIN_AMBIENT_S}초 이상 (너무 자주 나오면 시끄러움)"));
        }
        for (h, why) in &self.expressions {
            if !h.starts_with("head/") || why.trim().is_empty() {
                errs.push(format!("[expressions] {h:?}: head/<이름> = \"<애니 회차/프레임 근거>\" 꼴이어야 함"));
            }
        }
        for b in &all {
            for h in b.head.iter().flat_map(|h| h.frames()) {
                if !self.allows_head(h) {
                    errs.push(format!("behavior {} 의 머리 {h} 는 [expressions] 에 근거가 없어 그려지지 않음", b.name));
                }
            }
        }
        if !self.catalog.props.is_empty() {
            match self.desk.as_ref().and_then(|d| d.item.as_deref()) {
                None => errs.push("[desk] item (캐릭터 책상 소품) 이 없음".into()),
                Some(i) if self.catalog.prop(i).is_none() => errs.push(format!("[desk] item {i} 이 characters/_props/props.toml 에 없음")),
                Some(i) if !self.layers.iter().any(|l| *l == format!("prop/{i}")) => errs.push(format!("소품 그림 prop/{i} 이 없음")),
                _ => {}
            }
            if !all.iter().any(|b| b.on == Cue::Prop) {
                errs.push("prop cue (책상 소품을 누름) 에 behavior 가 없음".into());
            }
        }
        let mine: BTreeSet<&str> = all.iter().map(|b| b.name.as_str()).collect();
        let my_hashes = self.behavior_layer_hashes();
        for o in others {
            let theirs: BTreeSet<String> = o.all_behaviors().into_iter().map(|b| b.name).collect();
            let same: Vec<&str> = mine.iter().copied().filter(|n| theirs.contains(*n)).collect();
            if !same.is_empty() {
                errs.push(format!("{} 와 behavior 이름이 겹침: {}", o.name, same.join(", ")));
            }
            let oh = o.behavior_layer_hashes();
            for (layer, h) in &my_hashes {
                if let Some((ol, _)) = oh.iter().find(|(_, x)| x == h) {
                    errs.push(format!("레이어 {layer} 이 {} 의 {ol} 과 같은 파일", o.name));
                }
            }
        }
        errs
    }

    /// behavior 가 쓰는 레이어 파일의 내용 해시 (읽을 수 없는 파일은 뺀다).
    fn behavior_layer_hashes(&self) -> Vec<(String, u64)> {
        use std::hash::{Hash, Hasher};
        let names: BTreeSet<String> = self.all_behaviors().iter().flat_map(|b| b.layer_names().into_iter().map(String::from).collect::<Vec<_>>()).collect();
        names
            .into_iter()
            .filter_map(|n| {
                let bytes = std::fs::read(self.layer_path(&n)).ok()?;
                let mut h = std::collections::hash_map::DefaultHasher::new();
                bytes.hash(&mut h);
                Some((n, h.finish()))
            })
            .collect()
    }

    /// 같은 폴더(characters/)의 다른 실제 캐릭터 (`_` 로 시작하는 폴더와 자기 자신 제외, 못 읽는 것은 건너뜀).
    pub fn siblings(&self) -> Vec<Manifest> {
        let Some(root) = self.dir.parent() else { return Vec::new() };
        let me = self.dir.file_name().map(|n| n.to_owned());
        Self::available_in(root)
            .into_iter()
            .filter(|(n, _)| Some(std::ffi::OsString::from(n)) != me)
            .filter_map(|(n, _)| Self::load_dir(&root.join(n)).ok())
            .collect()
    }

    pub fn scenes_for(&self, p: Pose) -> Vec<&Scene> {
        self.scenes.iter().filter(|s| s.trigger == p).collect()
    }

    pub fn layer_path(&self, name: &str) -> PathBuf {
        if let Some(id) = name.strip_prefix("prop/") {
            return self.catalog.layer_path(id);
        }
        self.dir.join("layers").join(format!("{name}.png"))
    }

    pub fn layer_set(&self) -> BTreeSet<String> {
        self.layers.iter().cloned().collect()
    }

    /// 이 머리 레이어를 그려도 되는가 (애니 근거가 있는 표정만). head/ 가 아닌 레이어는 항상 true.
    pub fn allows_head(&self, layer: &str) -> bool {
        !layer.starts_with("head/") || layer == "head/normal" || self.expressions.contains_key(layer)
    }
}
