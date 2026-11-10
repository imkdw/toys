//! 사용자 설정(위치/크기/캐릭터)과 캐릭터 manifest.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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
        Ok(())
    }

    pub fn layer_path(&self, name: &str) -> PathBuf {
        self.dir.join("layers").join(format!("{name}.png"))
    }

    pub fn layer_set(&self) -> BTreeSet<String> {
        self.layers.iter().cloned().collect()
    }
}
