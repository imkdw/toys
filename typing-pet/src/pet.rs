//! 상태 머신: 머리 상태, 앞발 상태, 활동 점수 (PLAN 5.3).
//!
//! 모든 함수는 현재 시각을 인자로 받는다 (`Instant::now()` 를 직접 부르지 않는다).
//! 테스트에서 시간을 마음대로 넘겨 120ms 복귀, 점수 감소, 60초 sleepy 를 즉시 검증한다.

use std::collections::VecDeque;
use std::fmt;
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

/// 전체 프레임 연출 (애니 장면). 이름은 발동 조건이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pose {
    /// 과열이 식을 때.
    Gesture,
    /// 한동안 입력이 없을 때 (입력이 오면 바로 끝).
    Idle,
    /// 키를 `milestone_keys` 번 칠 때마다.
    Milestone,
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
            Pose::Idle => "full/idle",
            Pose::Milestone => "full/milestone",
        }
    }
}

/// cue 로 받는 이름 있는 키. 키 코드만 비교하고 저장하지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NamedKey {
    Enter,
    Backspace,
    Space,
    Tab,
    Escape,
    Delete,
}

impl NamedKey {
    pub const ALL: [NamedKey; 6] = [NamedKey::Enter, NamedKey::Backspace, NamedKey::Space, NamedKey::Tab, NamedKey::Escape, NamedKey::Delete];
    pub fn name(self) -> &'static str {
        match self {
            NamedKey::Enter => "enter",
            NamedKey::Backspace => "backspace",
            NamedKey::Space => "space",
            NamedKey::Tab => "tab",
            NamedKey::Escape => "escape",
            NamedKey::Delete => "delete",
        }
    }
    /// macOS 가상 키 코드 (Return 36, 키패드 Enter 76, Delete(백스페이스) 51, 앞으로 지우기 117).
    pub fn codes(self) -> &'static [u16] {
        match self {
            NamedKey::Enter => &[36, 76],
            NamedKey::Backspace => &[51],
            NamedKey::Space => &[49],
            NamedKey::Tab => &[48],
            NamedKey::Escape => &[53],
            NamedKey::Delete => &[117],
        }
    }
    pub fn of(code: u16) -> Option<NamedKey> {
        Self::ALL.into_iter().find(|k| k.codes().contains(&code))
    }
}

/// 엔진이 알리는 "무슨 일이 일어났는지". 어떻게 반응할지는 캐릭터 manifest 의 `[[behavior]]` 가 정한다.
///
/// manifest 에는 문자열로 쓴다: `idle`, `milestone`, `gesture`, `level:<normal|excited|dizzy|overheat|sleepy>`,
/// `egg`(선글라스), `wake`, `burst`, `key:<enter|backspace|space|tab|escape|delete>`, `ambient`, `prop`(책상 소품을 누름).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Cue {
    /// 한동안 입력이 없을 때 (pose).
    Idle,
    /// 키를 `milestone_keys` 번 칠 때마다 (pose).
    Milestone,
    /// 과열이 식을 때 (pose).
    Gesture,
    /// 머리 단계가 바뀜 (normal, excited, dizzy, overheat, sleepy).
    Level(Head),
    /// 선글라스 이스터에그.
    Egg,
    /// `wake_after_s` 이상 쉬다가 첫 입력.
    Wake,
    /// `burst_window_ms` 안에 `burst_keys` 번 이상.
    Burst,
    Key(NamedKey),
    /// 평소 상태에서 `ambient_every_s` 범위의 무작위 간격.
    Ambient,
    /// 책상 소품을 눌렀을 때 (직군/캐릭터 소품 상호작용).
    Prop,
}

impl Cue {
    /// 전체 프레임 장면(Pose)으로 이어지는 cue.
    pub fn pose(self) -> Option<Pose> {
        match self {
            Cue::Idle => Some(Pose::Idle),
            Cue::Milestone => Some(Pose::Milestone),
            Cue::Gesture => Some(Pose::Gesture),
            _ => None,
        }
    }
    pub fn of_pose(p: Pose) -> Cue {
        match p {
            Pose::Idle => Cue::Idle,
            Pose::Milestone => Cue::Milestone,
            Pose::Gesture => Cue::Gesture,
        }
    }
    /// 머리 상태가 바뀔 때 나는 cue.
    pub fn of_head(h: Head) -> Cue {
        if h == Head::Sunglasses { Cue::Egg } else { Cue::Level(h) }
    }
    /// `when = "hold"` (그 상태인 동안 반복) 를 쓸 수 있는 cue.
    pub fn can_hold(self) -> bool {
        matches!(self, Cue::Level(_) | Cue::Egg)
    }
    /// 테스트/문서용: 모든 cue 종류 (key 는 이름마다).
    pub fn all() -> Vec<Cue> {
        let mut v = vec![Cue::Idle, Cue::Milestone, Cue::Gesture];
        v.extend([Head::Normal, Head::Excited, Head::Dizzy, Head::Overheat, Head::Sleepy].map(Cue::Level));
        v.extend([Cue::Egg, Cue::Wake, Cue::Burst, Cue::Ambient, Cue::Prop]);
        v.extend(NamedKey::ALL.map(Cue::Key));
        v
    }
}

impl fmt::Display for Cue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cue::Idle => f.write_str("idle"),
            Cue::Milestone => f.write_str("milestone"),
            Cue::Gesture => f.write_str("gesture"),
            Cue::Level(h) => write!(f, "level:{}", h.layer().trim_start_matches("head/")),
            Cue::Egg => f.write_str("egg"),
            Cue::Wake => f.write_str("wake"),
            Cue::Burst => f.write_str("burst"),
            Cue::Key(k) => write!(f, "key:{}", k.name()),
            Cue::Ambient => f.write_str("ambient"),
            Cue::Prop => f.write_str("prop"),
        }
    }
}

impl std::str::FromStr for Cue {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let bad = || format!("알 수 없는 cue: {s:?} (idle, milestone, gesture, level:<단계>, egg, wake, burst, key:<이름>, ambient, prop)");
        Ok(match s {
            "idle" => Cue::Idle,
            "milestone" => Cue::Milestone,
            "gesture" => Cue::Gesture,
            "egg" => Cue::Egg,
            "wake" => Cue::Wake,
            "burst" => Cue::Burst,
            "ambient" => Cue::Ambient,
            "prop" => Cue::Prop,
            _ => {
                if let Some(l) = s.strip_prefix("level:") {
                    let h = [Head::Normal, Head::Excited, Head::Dizzy, Head::Overheat, Head::Sleepy]
                        .into_iter()
                        .find(|h| h.layer() == format!("head/{l}"))
                        .ok_or_else(bad)?;
                    Cue::Level(h)
                } else if let Some(k) = s.strip_prefix("key:") {
                    Cue::Key(NamedKey::ALL.into_iter().find(|n| n.name() == k).ok_or_else(bad)?)
                } else {
                    return Err(bad());
                }
            }
        })
    }
}

impl TryFrom<String> for Cue {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        s.parse()
    }
}

impl From<Cue> for String {
    fn from(c: Cue) -> String {
        c.to_string()
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
    /// 마우스를 움직임 (쓰로틀 후). 손을 슥슥 움직이는 연출에 쓴다.
    MouseMove,
    /// 캐릭터 행동을 부를 수 있는 일 (wake, burst, key, ambient). 단계/장면 cue 는 연출 쪽이 상태 변화로 안다.
    Cue(Cue),
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
    /// idle 장면 끝 시각과, 이번 쉬는 동안 이미 했는지.
    idle_until: Option<Instant>,
    idle_done: bool,
    /// idle_scene_ms = 0: 입력이 올 때까지 idle 장면.
    idle_hold: bool,
    milestone_until: Option<Instant>,
    keys_typed: u64,
    egg_key: Option<u16>,
    egg_count: u32,
    /// burst 판정용 최근 키 시각.
    recent_keys: VecDeque<Instant>,
    /// 다음 ambient cue 시각.
    ambient_at: Option<Instant>,
    rng: u64,
    events: Vec<PetEvent>,
}

fn ms(v: u64) -> Duration {
    Duration::from_millis(v)
}

impl Pet {
    pub fn new(timing: Timing, keys: Keys, now: Instant) -> Self {
        let mut p = Self {
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
            idle_until: None,
            idle_done: false,
            idle_hold: false,
            milestone_until: None,
            keys_typed: 0,
            egg_key: None,
            egg_count: 0,
            recent_keys: VecDeque::new(),
            ambient_at: None,
            rng: 0x9E37_79B9_7F4A_7C15,
            events: Vec::new(),
        };
        p.schedule_ambient(now);
        p
    }

    /// 무작위 순서를 바꾼다 (앱은 시작 시각으로, 테스트는 고정값으로). ambient 일정도 다시 잡는다.
    pub fn reseed(&mut self, seed: u64, now: Instant) {
        self.rng = seed | 1;
        self.schedule_ambient(now);
    }

    /// 0~1 (xorshift64*).
    fn rand01(&mut self) -> f64 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }

    fn schedule_ambient(&mut self, now: Instant) {
        let [lo, hi] = self.t.ambient_every_s;
        self.ambient_at = (hi > 0).then(|| {
            let lo = lo.min(hi) as f64;
            now + Duration::from_secs_f64(lo + (hi as f64 - lo) * self.rand01())
        });
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
        let rest = now.saturating_duration_since(self.last_input);
        if self.t.wake_after_s > 0 && rest >= Duration::from_secs(self.t.wake_after_s) {
            self.events.push(PetEvent::Cue(Cue::Wake));
        }
        self.last_input = now;
        self.idle_until = None;
        self.idle_done = false;
        self.idle_hold = false;
        match input {
            Input::Key(code) => {
                self.keys_typed += 1;
                let mk = self.t.milestone_keys;
                if mk > 0 && self.keys_typed % mk == 0 && self.gesture_until.is_none() {
                    self.milestone_until = Some(now + ms(self.t.milestone_ms));
                }
                self.score = self.score(now) + 1.0;
                self.score_at = now;
                self.easter_egg(code, now);
                self.burst(now);
                if let Some(k) = NamedKey::of(code) {
                    self.events.push(PetEvent::Cue(Cue::Key(k)));
                }
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
                self.events.push(PetEvent::MouseMove);
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

    /// 짧은 시간에 몰아친 키. 한 번 나면 창을 비우고 다시 센다.
    fn burst(&mut self, now: Instant) {
        if self.t.burst_keys == 0 {
            return;
        }
        let win = ms(self.t.burst_window_ms);
        self.recent_keys.retain(|&t| now.saturating_duration_since(t) < win);
        self.recent_keys.push_back(now);
        if self.recent_keys.len() as u32 >= self.t.burst_keys {
            self.recent_keys.clear();
            self.events.push(PetEvent::Cue(Cue::Burst));
        }
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
        if self.milestone_until.is_some_and(|t| t <= now) {
            self.milestone_until = None;
        }
        if self.idle_until.is_some_and(|t| t <= now) {
            self.idle_until = None;
        }
        if let Some(start) = self.idle_start() {
            if now >= start {
                self.idle_done = true;
                if self.t.idle_scene_ms == 0 {
                    self.idle_hold = true;
                } else {
                    let end = start + ms(self.t.idle_scene_ms);
                    self.idle_until = (end > now).then_some(end);
                }
            }
        }
        if self.ambient_at.is_some_and(|t| t <= now) {
            // 장면 중이면 건너뛰고 다음 일정만 잡는다
            if self.state(now).pose.is_none() {
                self.events.push(PetEvent::Cue(Cue::Ambient));
            }
            self.schedule_ambient(now);
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

    /// 아직 안 한 idle 장면이 시작할 시각.
    fn idle_start(&self) -> Option<Instant> {
        (self.t.idle_scene_s > 0 && !self.idle_done).then(|| self.last_input + Duration::from_secs(self.t.idle_scene_s))
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
        let live = |t: Option<Instant>| t.is_some_and(|t| t > now);
        let pose = if live(self.gesture_until) {
            Some(Pose::Gesture)
        } else if live(self.milestone_until) {
            Some(Pose::Milestone)
        } else if live(self.idle_until) || self.idle_hold {
            Some(Pose::Idle)
        } else {
            None
        };
        PetState { paw_l, paw_r, head, pose }
    }

    pub fn drain_events(&mut self) -> Vec<PetEvent> {
        std::mem::take(&mut self.events)
    }

    /// 다음에 상태가 바뀔 수 있는 시각. 없으면 None. 항상 now 보다 미래.
    pub fn next_deadline(&self, now: Instant) -> Option<Instant> {
        let mut cands: Vec<Instant> = [self.l_until, self.r_act.map(|r| r.1), self.mouse_until, self.gesture_until, self.sun_until, self.milestone_until, self.idle_until, self.idle_start(), self.ambient_at]
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
