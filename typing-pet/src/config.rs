//! 사용자 설정(위치/크기/캐릭터)과 캐릭터 manifest.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::pet::Pose;

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
}

impl Default for Config {
    fn default() -> Self {
        Self { character: None, position: None, size: 240.0, visible: true, breathing: true }
    }
}

pub const SIZES: [f64; 5] = [160.0, 200.0, 240.0, 320.0, 400.0];

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
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                log::warn!("설정 파일이 깨져서 기본값 사용: {e}");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
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
    #[serde(default, rename = "scene")]
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub timing: Timing,
    #[serde(default)]
    pub anim: Anim,
    #[serde(default)]
    pub keys: Keys,

    #[serde(skip)]
    pub dir: PathBuf,
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
    /// float: 한 주기 동안 떠오르는 높이 (pt).
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

    /// name 이 None 이면 characters/ 아래 이름순 첫 번째 폴더 (`_` 로 시작하는 테스트 폴더 제외).
    pub fn load(name: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let root = Self::characters_dir();
        let dir = match name {
            Some(name) => root.join(name),
            None => {
                let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)?
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.join("manifest.toml").is_file())
                    .filter(|p| !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('_')))
                    .collect();
                dirs.sort();
                dirs.into_iter().next().ok_or("characters/ 아래에 manifest.toml 이 있는 폴더가 없음")?
            }
        };
        Self::load_dir(&dir)
    }

    pub fn load_dir(dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(dir.join("manifest.toml"))
            .map_err(|e| format!("{}: {e}", dir.join("manifest.toml").display()))?;
        let mut manifest: Manifest = toml::from_str(&text)?;
        manifest.dir = dir.to_path_buf();
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
        Ok(())
    }

    /// 장면이 없는 발동 조건(idle, milestone)은 끄고, 장면 길이를 타이밍에 반영한다.
    pub fn apply_scenes(&mut self) {
        let first = |p: Pose| self.scenes.iter().find(|s| s.trigger == p).map(|s| s.duration_ms);
        let t = &mut self.timing;
        match first(Pose::Idle) {
            None => t.idle_scene_s = 0,
            Some(d) => t.idle_scene_ms = d.unwrap_or(t.idle_scene_ms),
        }
        match first(Pose::Milestone) {
            None => t.milestone_keys = 0,
            Some(d) => t.milestone_ms = d.unwrap_or(t.milestone_ms),
        }
        if let Some(Some(d)) = first(Pose::Gesture) {
            t.gesture_ms = d;
        }
    }

    pub fn scenes_for(&self, p: Pose) -> Vec<&Scene> {
        self.scenes.iter().filter(|s| s.trigger == p).collect()
    }

    pub fn layer_path(&self, name: &str) -> PathBuf {
        self.dir.join("layers").join(format!("{name}.png"))
    }

    pub fn layer_set(&self) -> BTreeSet<String> {
        self.layers.iter().cloned().collect()
    }
}
