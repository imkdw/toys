//! 입력 키 보기: 누른 키 이름이 머리 위 가운데에 구름 말풍선으로 톡 뜨고,
//! 이전 키들은 한 칸씩 오른쪽 위 뒤쪽으로 둥실둥실 밀려나며 작아지고 흐려진다 (연기처럼 휘는 꼬리).
//! 손을 멈추면 꼬리 끝(가장 오래된 것)부터 하나씩 구름처럼 흩어진다.
//!
//! - 키 값은 화면에 잠깐 그릴 뿐 저장하거나 기록하지 않는다. 떠 있는 동안만 메모리에 있다.
//! - 위치는 캐릭터 정사각형 기준 (0~1, 위로 벗어나면 음수). 창은 이 기능이 켜지면 위로 [`BAND`] 만큼 늘어난다.
//! - 키 이름은 US 배열 기준 키캡 표기 (한글 입력 중이어도 키캡에 적힌 글자).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// 기능이 켜졌을 때 창 위에 더하는 띠 높이 (캐릭터 크기 대비). 꼬리가 위로 올라갈 자리.
pub const BAND: f64 = 0.36;

/// 꼬리에 또렷하게 남는 최대 개수 (맨 앞 포함). 넘치면 꼬리 끝부터 흩어진다.
pub const MAX: usize = 6;
/// 톡 튀어나오는 시간.
const POP_MS: f32 = 180.0;
/// 한 칸 뒤로 밀리는 시간.
const STEP_MS: f32 = 300.0;
/// 마지막 입력 뒤 이만큼 지나면 꼬리 끝부터 흩어지기 시작한다.
const LIFE_MS: u64 = 1400;
/// 꼬리 끝부터 하나씩 흩어지는 간격.
const DRAIN_MS: u64 = 110;
/// 구름처럼 흩어지는 시간.
const FADE_MS: f32 = 420.0;
/// 맨 앞 말풍선 높이 (캐릭터 크기 대비).
const HEIGHT: f32 = 0.12;
/// 한 칸 뒤로 갈 때마다 줄어드는 비율, 흐려지는 정도.
const SHRINK: f32 = 0.84;
const DIM: f32 = 0.15;
/// 꼬리 곡선: u = 칸/MAX 일 때 오른쪽으로 `CURVE_X * u^2 + LEAN_X * u`, 위로 `RISE * sqrt(u) - SAG * u`.
/// 처음엔 위로 솟다가 점점 오른쪽으로 눕는다.
const CURVE_X: f32 = 0.44;
const LEAN_X: f32 = 0.05;
const RISE: f32 = 0.25;
const SAG: f32 = 0.045;
/// 뒤로 밀린 구름끼리 위아래로 엇갈리는 정도.
const JITTER: f32 = 0.035;
/// 둥실둥실 흔들림 (한 번 왕복 시간, 가로/세로 폭).
const WOBBLE_MS: f32 = 3200.0;
const WOBBLE_X: f32 = 0.006;
const WOBBLE_Y: f32 = 0.004;

/// macOS 가상 키 코드 -> 키캡 표기. 모르는 키는 None (표시 안 함).
pub fn label_of(code: u16) -> Option<&'static str> {
    Some(match code {
        0 => "A",
        1 => "S",
        2 => "D",
        3 => "F",
        4 => "H",
        5 => "G",
        6 => "Z",
        7 => "X",
        8 => "C",
        9 => "V",
        10 => "§",
        11 => "B",
        12 => "Q",
        13 => "W",
        14 => "E",
        15 => "R",
        16 => "Y",
        17 => "T",
        18 => "1",
        19 => "2",
        20 => "3",
        21 => "4",
        22 => "6",
        23 => "5",
        24 => "=",
        25 => "9",
        26 => "7",
        27 => "-",
        28 => "8",
        29 => "0",
        30 => "]",
        31 => "O",
        32 => "U",
        33 => "[",
        34 => "I",
        35 => "P",
        36 => "↩",
        37 => "L",
        38 => "J",
        39 => "'",
        40 => "K",
        41 => ";",
        42 => "\\",
        43 => ",",
        44 => "/",
        45 => "N",
        46 => "M",
        47 => ".",
        48 => "⇥",
        49 => "space",
        50 => "`",
        51 => "⌫",
        53 => "esc",
        54 | 55 => "⌘",
        56 | 60 => "⇧",
        57 => "⇪",
        58 | 61 => "⌥",
        59 | 62 => "⌃",
        63 => "fn",
        65 => ".",
        67 => "*",
        69 => "+",
        71 => "⌧",
        75 => "/",
        76 => "⌤",
        78 => "-",
        81 => "=",
        82 => "0",
        83 => "1",
        84 => "2",
        85 => "3",
        86 => "4",
        87 => "5",
        88 => "6",
        89 => "7",
        91 => "8",
        92 => "9",
        93 => "¥",
        94 => "_",
        96 => "F5",
        97 => "F6",
        98 => "F7",
        99 => "F3",
        100 => "F8",
        101 => "F9",
        102 => "英数",
        103 => "F11",
        104 => "かな",
        105 => "F13",
        107 => "F14",
        109 => "F10",
        111 => "F12",
        113 => "F15",
        114 => "help",
        115 => "↖",
        116 => "⇞",
        117 => "⌦",
        118 => "F4",
        119 => "↘",
        120 => "F2",
        121 => "⇟",
        122 => "F1",
        123 => "←",
        124 => "→",
        125 => "↓",
        126 => "↑",
        _ => return None,
    })
}

/// 그릴 말풍선 하나.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub label: &'static str,
    /// 말풍선 가운데 (캐릭터 정사각형 기준).
    pub center: [f32; 2],
    /// 말풍선 높이 (캐릭터 크기 대비).
    pub height: f32,
    pub opacity: f32,
    /// 0 = 또렷, 1 = 다 흩어짐.
    pub dissolve: f32,
    pub seed: f32,
}

#[derive(Debug, Clone)]
struct Item {
    label: &'static str,
    start: Instant,
    seed: f32,
    /// 칸 (0 = 맨 앞). `from` 에서 `to` 로 `moved` 부터 STEP_MS 동안 밀린다.
    from: f32,
    to: f32,
    moved: Instant,
    /// 흩어지기 시작하는 시각 (입력이 올 때마다 다시 잡는다).
    fade_at: Instant,
}

impl Item {
    fn slot(&self, now: Instant) -> f32 {
        let t = (now.saturating_duration_since(self.moved).as_secs_f32() * 1000.0 / STEP_MS).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * ease_out(t)
    }

    fn fading(&self, now: Instant) -> bool {
        self.fade_at <= now
    }

    fn fade_end(&self) -> Instant {
        self.fade_at + Duration::from_secs_f32(FADE_MS / 1000.0)
    }
}

#[derive(Debug, Clone)]
pub struct KeyEcho {
    /// 앞(새것)부터 뒤(오래된 것) 순서.
    items: VecDeque<Item>,
    spot: [f32; 2],
    n: u32,
}

impl Default for KeyEcho {
    fn default() -> Self {
        Self { items: VecDeque::new(), spot: [0.5, 0.15], n: 0 }
    }
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_out_back(t: f32) -> f32 {
    let c = 1.9;
    1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2)
}

/// 말풍선 너비/높이 비율 (대략. text::cloud 와 같은 규칙).
pub fn aspect_of(label: &str) -> f32 {
    let n = label.chars().count();
    let body = if n <= 1 { 1.35 } else { (0.95 + 0.22 * n as f32).max(1.35) };
    body * 0.74 + 0.26
}

/// 칸 `s` 의 맨 앞 기준 위치 (오른쪽 +, 위 -).
fn trail(s: f32) -> [f32; 2] {
    let u = (s / MAX as f32).max(0.0);
    [CURVE_X * u * u + LEAN_X * u, -(RISE * u.sqrt() - SAG * u)]
}

impl KeyEcho {
    pub fn set_spot(&mut self, x: f32, top: f32) {
        self.spot = [x, top];
    }

    pub fn spot(&self) -> [f32; 2] {
        self.spot
    }

    pub fn push(&mut self, code: u16, now: Instant) -> bool {
        let Some(label) = label_of(code) else { return false };
        self.items.retain(|it| it.fade_end() > now);
        // 또렷한 것들은 한 칸씩 뒤로
        for it in self.items.iter_mut().filter(|it| !it.fading(now)) {
            it.from = it.slot(now);
            it.to += 1.0;
            it.moved = now;
        }
        let seed = (self.n as f32 * 0.618_034).fract();
        self.n = self.n.wrapping_add(1);
        self.items.push_front(Item { label, start: now, seed, from: 0.0, to: 0.0, moved: now, fade_at: now });
        // 흩어지는 시각을 다시 잡는다: 넘친 것은 바로, 나머지는 손을 멈춘 뒤 꼬리 끝부터 하나씩
        let live: Vec<usize> = (0..self.items.len()).filter(|&i| i == 0 || !self.items[i].fading(now)).collect();
        let k = live.len().min(MAX);
        for (rank, &i) in live.iter().enumerate() {
            let it = &mut self.items[i];
            it.fade_at = if rank >= MAX { now } else { now + Duration::from_millis(LIFE_MS + (k - 1 - rank) as u64 * DRAIN_MS) };
        }
        true
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 다 흩어진 것을 빼고 지금 그릴 말풍선 (꼬리 끝부터, 맨 앞 것이 가장 위에 그려진다).
    pub fn sprites(&mut self, now: Instant) -> Vec<Sprite> {
        self.items.retain(|it| it.fade_end() > now);
        // 맨 앞은 머리 꼭대기 바로 위
        let base = [self.spot[0], self.spot[1] - HEIGHT * 0.5 - 0.015];
        let top_limit = -(BAND as f32) + 0.01;
        self.items
            .iter()
            .rev()
            .map(|it| {
                let ms = now.saturating_duration_since(it.start).as_secs_f32() * 1000.0;
                let pop = (ms / POP_MS).clamp(0.0, 1.0);
                let fade = (now.saturating_duration_since(it.fade_at).as_secs_f32() * 1000.0 / FADE_MS).clamp(0.0, 1.0);
                // 흩어지는 것은 꼬리 방향으로 조금 더 흘러가며 사라진다 (꼬리 끝에 뭉치지 않게)
                let s = it.slot(now) + 0.7 * ease_out(fade);
                let [tx, ty] = trail(s);
                // 뒤로 밀린 것끼리 위아래로 조금 엇갈리고, 다들 천천히 흔들린다
                let ph = std::f32::consts::TAU * (ms / WOBBLE_MS + it.seed);
                let mut x = base[0] + tx + WOBBLE_X * ph.sin();
                let mut y = base[1] + ty + JITTER * (it.seed - 0.5) * s.min(1.0) + WOBBLE_Y * ph.cos();
                // 튀어나올 때 아래에서, 흩어질 때 살짝 떠오른다
                y += 0.02 * (1.0 - ease_out(pop));
                let height = HEIGHT * SHRINK.powf(s) * (0.4 + 0.6 * ease_out_back(pop)) * (1.0 + 0.2 * ease_out(fade));
                // 창 밖으로 나가지 않게
                let half_w = height * aspect_of(it.label) / 2.0;
                x = x.clamp(half_w, 1.0 - half_w);
                y = y.max(top_limit + height / 2.0);
                Sprite {
                    label: it.label,
                    center: [x, y],
                    height,
                    opacity: (pop * 2.0).min(1.0) * (1.0 - DIM * s).max(0.0),
                    dissolve: fade,
                    seed: it.seed,
                }
            })
            .collect()
    }

    /// 지금 움직이는 중. 떠 있는 동안은 둥실둥실 흔들리므로 늘 true.
    pub fn animating(&self, now: Instant) -> bool {
        self.next_redraw(now).is_some_and(|t| t <= now)
    }

    /// 다음에 다시 그려야 하는 시각 (떠 있는 것이 있으면 바로).
    pub fn next_redraw(&self, now: Instant) -> Option<Instant> {
        self.items.iter().any(|it| it.fade_end() > now).then_some(now)
    }
}

impl Sprite {
    /// 텍스처를 그리는 기본 높이 (캐릭터 크기 대비). 튀어나오기/부풀기는 이걸 늘려 그린다.
    pub const BASE_HEIGHT: f32 = HEIGHT * 1.1;
}
