//! 책상 소품: 직군(job)에 맞는 장비와 캐릭터 소품을 책상 자리(slot)에 놓고, 누르면 반응한다.
//!
//! 정의와 그림은 모든 캐릭터가 같이 쓰는 `characters/_props/` 에 있다.
//! - `props.toml`: `[[prop]]` (소품), `[[job]]` (직군별 소품 묶음)
//! - `layers/<id>.png`: 캔버스 크기 그대로, 소품이 제자리(slot)에 그려진 그림 (`art/scripts/make_props.py`)
//!
//! 캐릭터는 manifest `[desk] item` 으로 자기 소품 하나를 고른다 (캐릭터 자리).
//! 직군 소품은 왼쪽 자리들(left_back, left_front)에 놓는다. 장면 프레임이 자리를 덮으면 `[desk] hide` 로 그 자리를 숨긴다.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::Motion;

/// 소품을 누르고 반응하는 기본 시간 (ms).
pub const TAP_MS: u64 = 700;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Prop {
    pub id: String,
    /// 메뉴/문서용 이름.
    pub title: String,
    /// 놓이는 자리 (left_back, left_front, right_back).
    pub slot: String,
    /// 소품 바닥 가운데 (캔버스 px). 누를 때 움직임의 기준점.
    pub foot: [f32; 2],
    /// 누르면 하는 움직임 (target 은 "body" = 소품 자신). 없으면 작게 통통.
    #[serde(default)]
    pub tap: Vec<Motion>,
    #[serde(default)]
    pub tap_ms: Option<u64>,
}

impl Prop {
    pub fn layer(&self) -> String {
        format!("prop/{}", self.id)
    }
    pub fn tap_len(&self) -> u64 {
        self.tap_ms.unwrap_or(TAP_MS)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub id: String,
    pub title: String,
    /// 소품 id. 자리마다 하나.
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    #[serde(default, rename = "prop")]
    pub props: Vec<Prop>,
    #[serde(default, rename = "job")]
    pub jobs: Vec<Job>,
    #[serde(skip)]
    pub dir: PathBuf,
}

impl Catalog {
    /// `<characters>/_props/props.toml`. 없으면 빈 목록 (소품 없이 동작).
    pub fn load_in(characters: &Path) -> Result<Self, String> {
        let dir = characters.join("_props");
        let path = dir.join("props.toml");
        let Ok(text) = std::fs::read_to_string(&path) else { return Ok(Self::default()) };
        let mut c: Catalog = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        c.dir = dir;
        c.validate()?;
        Ok(c)
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut seen = BTreeMap::new();
        for p in &self.props {
            if seen.insert(p.id.as_str(), ()).is_some() {
                return Err(format!("소품 id {} 가 겹침", p.id));
            }
            if p.tap.iter().any(|m| m.target != "body") {
                return Err(format!("소품 {}: tap 움직임의 target 은 \"body\" (소품 자신) 만", p.id));
            }
        }
        for j in &self.jobs {
            let mut slots = BTreeMap::new();
            for i in &j.items {
                let p = self.prop(i).ok_or_else(|| format!("직군 {}: 없는 소품 {i}", j.id))?;
                if slots.insert(p.slot.as_str(), ()).is_some() {
                    return Err(format!("직군 {}: {} 자리에 소품이 둘", j.id, p.slot));
                }
            }
        }
        Ok(())
    }

    pub fn prop(&self, id: &str) -> Option<&Prop> {
        self.props.iter().find(|p| p.id == id)
    }

    pub fn job(&self, id: &str) -> Option<&Job> {
        self.jobs.iter().find(|j| j.id == id)
    }

    pub fn layer_path(&self, id: &str) -> PathBuf {
        self.dir.join("layers").join(format!("{id}.png"))
    }

    /// 직군 소품 + 캐릭터 소품. 캐릭터 소품과 같은 자리의 직군 소품은 뺀다 (캐릭터가 우선).
    pub fn items(&self, job: Option<&str>, character: Option<&str>) -> Vec<Prop> {
        let mine = character.and_then(|c| self.prop(c));
        let mut v: Vec<Prop> = job
            .and_then(|j| self.job(j))
            .map(|j| j.items.iter().filter_map(|i| self.prop(i)).filter(|p| mine.is_none_or(|m| m.slot != p.slot)).cloned().collect())
            .unwrap_or_default();
        v.extend(mine.cloned());
        v
    }
}
