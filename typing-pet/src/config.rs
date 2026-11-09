//! 사용자 설정(위치/크기/캐릭터)과 캐릭터 manifest.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 사용자 설정. `~/Library/Application Support/typing-pet/config.toml` 에 저장.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// characters/ 아래 폴더 이름. None 이면 첫 번째 폴더.
    pub character: Option<String>,
    /// 창 좌상단 위치 (논리 좌표). None 이면 화면 우하단.
    pub position: Option<(f64, f64)>,
    /// 표시 크기 (pt).
    pub size: f64,
    pub visible: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            character: None,
            position: None,
            size: 240.0,
            visible: true,
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("typing-pet").join("config.toml"))
    }

    /// 파일이 없거나 깨졌으면 기본값.
    pub fn load() -> Self {
        // TODO: 파일 읽기
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        // TODO
        Ok(())
    }
}

/// `characters/<name>/manifest.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub name: String,
    /// 캔버스 크기 (px). 모든 프레임이 같아야 한다.
    pub canvas: u32,
    pub frames: Frames,
    #[serde(default)]
    pub effects: Effects,
    pub timing: Timing,
    pub keys: Keys,

    #[serde(skip)]
    pub dir: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Frames {
    pub base: String,
    pub left_down: String,
    pub right_down: String,
    pub both_down: String,
    // 2차
    pub happy_base: Option<String>,
    pub happy_left_down: Option<String>,
    pub happy_right_down: Option<String>,
    pub happy_both_down: Option<String>,
    pub sunglasses: Option<String>,
    pub gesture: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Effects {
    pub heart: Option<String>,
    pub sweat: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Timing {
    pub down_ms: u64,
    pub speed_window_ms: u64,
    pub happy_kps: f32,
    pub sweat_kps: f32,
    pub heart_ms: u64,
    pub special_ms: u64,
    pub easter_egg_repeat: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Keys {
    /// 왼발로 칠 macOS virtual keycode 목록. 나머지는 오른발.
    pub left: Vec<u32>,
}

impl Manifest {
    pub fn characters_dir() -> PathBuf {
        // TODO: .app 번들 안 Resources 경로 우선
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("characters")
    }

    /// name 이 None 이면 characters/ 아래 첫 번째 폴더(이름순)를 쓴다.
    pub fn load(name: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let root = Self::characters_dir();
        let dir = match name {
            Some(name) => root.join(name),
            None => {
                let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)?
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.join("manifest.toml").is_file())
                    .collect();
                dirs.sort();
                dirs.into_iter()
                    .next()
                    .ok_or("characters/ 아래에 manifest.toml 이 있는 폴더가 없음")?
            }
        };
        let text = std::fs::read_to_string(dir.join("manifest.toml"))?;
        let mut manifest: Manifest = toml::from_str(&text)?;
        manifest.dir = dir;
        Ok(manifest)
    }

    pub fn frame_path(&self, file: &str) -> PathBuf {
        self.dir.join("frames").join(file)
    }
}
