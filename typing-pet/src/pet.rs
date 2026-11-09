//! 상태 머신: 머리 상태, 앞발 상태, 활동 점수 (PLAN 5.3).
//!
//! 모든 함수는 현재 시각을 인자로 받는다 (`Instant::now()` 를 직접 부르지 않는다).
//! 테스트에서 시간을 마음대로 넘겨 120ms 복귀, 점수 감소, 60초 sleepy 를 즉시 검증한다.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::config::{Keys, Timing};
use crate::input::{Input, Zone, zone_of};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PawL {
    #[default]
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PawR {
    #[default]
    Up,
    Down,
    Center,
    Mouse,
    MouseClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Head {
    #[default]
    Normal,
    Excited,
    Dizzy,
    Overheat,
    Sleepy,
    Sunglasses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pose {
    Gesture,
}

/// 활동 점수 단계.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    #[default]
    Normal = 0,
    Excited = 1,
    Dizzy = 2,
    Overheat = 3,
}

impl Level {
    pub const ALL: [Level; 4] = [Level::Normal, Level::Excited, Level::Dizzy, Level::Overheat];
    fn idx(self) -> usize {
        self as usize
    }
    fn head(self) -> Head {
        match self {
            Level::Normal => Head::Normal,
            Level::Excited => Head::Excited,
            Level::Dizzy => Head::Dizzy,
            Level::Overheat => Head::Overheat,
        }
    }
}

impl PawL {
    pub fn layer(self) -> &'static str {
        match self {
            PawL::Up => "paw_l/up",
            PawL::Down => "paw_l/down",
        }
    }
}

impl PawR {
    pub fn layer(self) -> &'static str {
        match self {
            PawR::Up => "paw_r/up",
            PawR::Down => "paw_r/down",
            PawR::Center => "paw_r/center",
            PawR::Mouse => "paw_r/mouse",
            PawR::MouseClick => "paw_r/mouse_click",
        }
    }
}

impl Head {
    pub const ALL: [Head; 6] = [Head::Normal, Head::Excited, Head::Dizzy, Head::Overheat, Head::Sleepy, Head::Sunglasses];
    /// 대표 레이어 (dizzy 는 wiggle 과 교대).
    pub fn layer(self) -> &'static str {
        match self {
            Head::Normal => "head/normal",
            Head::Excited => "head/excited",
            Head::Dizzy => "head/dizzy",
            Head::Overheat => "head/overheat",
            Head::Sleepy => "head/sleepy",
            Head::Sunglasses => "head/sunglasses",
        }
    }
    /// 활동 단계 순위. 단계가 올라갈 때만 팝 연출을 한다.
    pub fn rank(self) -> Option<u8> {
        match self {
            Head::Normal => Some(0),
            Head::Excited => Some(1),
            Head::Dizzy => Some(2),
            Head::Overheat => Some(3),
            _ => None,
        }
    }
}

impl Pose {
    pub fn layer(self) -> &'static str {
        match self {
            Pose::Gesture => "full/gesture",
        }
    }
}

/// 화면에 보여줄 상태.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct PetState {
    pub paw_l: PawL,
    pub paw_r: PawR,
    pub head: Head,
    pub pose: Option<Pose>,
}

/// 연출용 1회성 사건.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PetEvent {
    /// 왼발이 키를 누름.
    PressL,
    /// 오른발이 키/패드를 누름 (자세 포함).
    PressR(PawR),
}

#[derive(Debug, Clone)]
pub struct Pet {
    t: Timing,
    keys: Keys,
    l_until: Option<Instant>,
    /// 오른발 키/클릭 동작과 끝나는 시각. 새 입력이 오면 덮어쓴다.
    r_act: Option<(PawR, Instant)>,
    mouse_until: Option<Instant>,
    score: f64,
    score_at: Instant,
    level: Level,
    level_since: Instant,
    last_input: Instant,
    gesture_until: Option<Instant>,
    gesture_ready_at: Option<Instant>,
    sun_until: Option<Instant>,
    egg_key: Option<u16>,
    egg_count: u32,
    events: Vec<PetEvent>,
}

fn ms(v: u64) -> Duration {
    Duration::from_millis(v)
}

impl Pet {
    pub fn new(timing: Timing, keys: Keys, now: Instant) -> Self {
        Self {
            t: timing,
            keys,
            l_until: None,
            r_act: None,
            mouse_until: None,
            score: 0.0,
            score_at: now,
            level: Level::Normal,
            level_since: now,
            last_input: now,
            gesture_until: None,
            gesture_ready_at: None,
            sun_until: None,
            egg_key: None,
            egg_count: 0,
            events: Vec::new(),
        }
    }

    pub fn timing(&self) -> &Timing {
        &self.t
    }

    pub fn level(&self) -> Level {
        self.level
    }

    /// 지금 시각의 활동 점수 (지수 감소 반영).
    pub fn score(&self, now: Instant) -> f64 {
        let dt = now.saturating_duration_since(self.score_at).as_secs_f64();
        self.score * 0.5f64.powf(dt / self.t.half_life_s)
    }

    pub fn on_input(&mut self, input: Input, now: Instant) {
        self.advance(now);
        self.last_input = now;
        match input {
            Input::Key(code) => {
                self.score = self.score(now) + 1.0;
                self.score_at = now;
                self.easter_egg(code, now);
                let until = now + ms(self.t.paw_down_ms);
                match zone_of(code, &self.keys) {
                    Zone::Left => {
                        // 120ms 안에 같은 발로 또 누르면 타이머만 연장
                        self.l_until = Some(until);
                        self.events.push(PetEvent::PressL);
                    }
                    Zone::Center => self.press_r(PawR::Center, until),
                    Zone::Right => self.press_r(PawR::Down, until),
                }
            }
            Input::MouseMove => {
                self.mouse_until = Some(now + ms(self.t.mouse_hold_ms));
            }
            Input::MouseClick => {
                self.mouse_until = Some(now + ms(self.t.click_ms + self.t.mouse_hold_ms));
                self.press_r(PawR::MouseClick, now + ms(self.t.click_ms));
            }
        }
        self.advance(now);
    }

    fn press_r(&mut self, pose: PawR, until: Instant) {
        self.r_act = Some((pose, until));
        self.events.push(PetEvent::PressR(pose));
    }

    fn easter_egg(&mut self, code: u16, now: Instant) {
        if self.egg_key == Some(code) {
            self.egg_count += 1;
        } else {
            self.egg_key = Some(code);
            self.egg_count = 1;
        }
        if self.egg_count >= self.t.easter_egg_repeat {
            self.egg_count = 0;
            self.egg_key = None;
            self.sun_until = match self.sun_until {
                Some(_) => None,
                None => Some(now + ms(self.t.sunglasses_ms)),
            };
        }
    }

    /// 만료된 타이머 정리와 단계 변경.
    pub fn advance(&mut self, now: Instant) {
        if self.l_until.is_some_and(|t| t <= now) {
            self.l_until = None;
        }
        if self.r_act.is_some_and(|(_, t)| t <= now) {
            self.r_act = None;
        }
        if self.mouse_until.is_some_and(|t| t <= now) {
            self.mouse_until = None;
        }
        if self.gesture_until.is_some_and(|t| t <= now) {
            self.gesture_until = None;
        }
        if self.sun_until.is_some_and(|t| t <= now) {
            self.sun_until = None;
        }
        // 단계는 한 번에 한 칸, 최소 유지 시간이 지나야 바뀐다.
        let dwell = ms(self.t.min_dwell_ms);
        if now.saturating_duration_since(self.level_since) >= dwell {
            let s = self.score(now);
            let i = self.level.idx();
            if i < 3 && s >= self.t.level_up[i] {
                self.level = Level::ALL[i + 1];
                self.level_since = now;
            } else if i > 0 && s < self.t.level_down[i - 1] {
                let from = self.level;
                self.level = Level::ALL[i - 1];
                self.level_since = now;
                if from == Level::Overheat && self.t.gesture && self.gesture_ready_at.is_none_or(|t| now >= t) {
                    self.gesture_until = Some(now + ms(self.t.gesture_ms));
                    self.gesture_ready_at = Some(now + Duration::from_secs(self.t.gesture_cooldown_s));
                }
            }
        }
    }

    pub fn is_sleepy(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.last_input) >= Duration::from_secs(self.t.sleepy_s)
    }

    /// 지금 보여줄 상태. 먼저 `advance(now)` 를 부른 뒤 쓴다.
    pub fn state(&self, now: Instant) -> PetState {
        let paw_l = if self.l_until.is_some_and(|t| t > now) { PawL::Down } else { PawL::Up };
        let paw_r = match self.r_act {
            Some((pose, t)) if t > now => pose,
            _ if self.mouse_until.is_some_and(|t| t > now) => PawR::Mouse,
            _ => PawR::Up,
        };
        let head = if self.sun_until.is_some_and(|t| t > now) {
            Head::Sunglasses
        } else if self.is_sleepy(now) {
            Head::Sleepy
        } else {
            self.level.head()
        };
        let pose = self.gesture_until.filter(|t| *t > now).map(|_| Pose::Gesture);
        PetState { paw_l, paw_r, head, pose }
    }

    pub fn drain_events(&mut self) -> Vec<PetEvent> {
        std::mem::take(&mut self.events)
    }

    /// 다음에 상태가 바뀔 수 있는 시각. 없으면 None. 항상 now 보다 미래.
    pub fn next_deadline(&self, now: Instant) -> Option<Instant> {
        let mut cands: Vec<Instant> = [self.l_until, self.r_act.map(|r| r.1), self.mouse_until, self.gesture_until, self.sun_until]
            .into_iter()
            .flatten()
            .collect();
        if !self.is_sleepy(now) {
            cands.push(self.last_input + Duration::from_secs(self.t.sleepy_s));
        }
        let dwell_end = self.level_since + ms(self.t.min_dwell_ms);
        let i = self.level.idx();
        let s = self.score(now);
        if i < 3 && s >= self.t.level_up[i] {
            cands.push(dwell_end);
        }
        if i > 0 {
            let thr = self.t.level_down[i - 1];
            // score(t) = score * 0.5^(dt/hl) 가 thr 아래로 내려가는 시각 (+1ms 여유)
            let cross = if self.score <= thr {
                self.score_at
            } else {
                self.score_at + Duration::from_secs_f64(self.t.half_life_s * (self.score / thr).log2())
            } + ms(1);
            cands.push(cross.max(dwell_end));
        }
        let floor = now + Duration::from_micros(500);
        cands.into_iter().min().map(|t| t.max(floor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pet() -> (Pet, Instant) {
        let t0 = Instant::now();
        (Pet::new(Timing::default(), Keys::default(), t0), t0)
    }

    #[test]
    fn left_key_down_then_up() {
        let (mut p, t0) = pet();
        p.on_input(Input::Key(0), t0); // A
        assert_eq!(p.state(t0).paw_l, PawL::Down);
        p.advance(t0 + ms(119));
        assert_eq!(p.state(t0 + ms(119)).paw_l, PawL::Down);
        p.advance(t0 + ms(120));
        assert_eq!(p.state(t0 + ms(120)).paw_l, PawL::Up);
    }
}
