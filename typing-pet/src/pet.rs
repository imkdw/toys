//! 상태 머신: 지금 어떤 프레임과 오버레이를 보여줄지 결정한다.

use std::time::Instant;

use crate::config::Manifest;
use crate::input::InputEvent;

/// 손 상태 (1차 이미지 4장에 대응).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hands {
    #[default]
    Idle,
    LeftDown,
    RightDown,
    BothDown,
}

/// 표정 (2차).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Expression {
    #[default]
    Normal,
    Happy,
}

/// 이스터에그 포즈 (2차). 켜져 있으면 손 상태보다 우선.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Special {
    Sunglasses,
    Gesture,
}

/// 위치가 자유로운 오버레이 효과 (2차).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Heart,
    Sweat,
}

/// 렌더러에 넘길 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameSelection {
    /// manifest 의 프레임 이름 (예: "left_down", "happy_base").
    pub frame: String,
    pub effects: Vec<Effect>,
}

pub struct Pet {
    hands: Hands,
    expression: Expression,
    special: Option<Special>,
    effects: Vec<Effect>,
    // TODO: 마지막 좌/우 입력 시각, 최근 3초 키 타임스탬프 링버퍼, 연타 카운터, 각 상태 만료 시각
}

impl Pet {
    pub fn new(manifest: &Manifest) -> Self {
        let _ = manifest; // TODO: 임계값/매핑 보관
        Self {
            hands: Hands::default(),
            expression: Expression::default(),
            special: None,
            effects: Vec::new(),
        }
    }

    /// 입력 반영. 좌우 판정 -> 120ms down 유지 -> 양쪽이면 both_down.
    pub fn on_input(&mut self, event: InputEvent, now: Instant) {
        // TODO
        let _ = (event, now);
    }

    /// 만료된 상태를 되돌린다 (down -> idle, 효과 소멸 등).
    pub fn tick(&mut self, now: Instant) {
        // TODO
        let _ = now;
    }

    /// 가장 가까운 상태 만료 시각. 없으면 None (ControlFlow::Wait).
    pub fn next_deadline(&self) -> Option<Instant> {
        // TODO
        None
    }

    pub fn frame(&self) -> FrameSelection {
        // TODO: expression, special 반영
        let _ = (self.expression, self.special);
        let frame = match self.hands {
            Hands::Idle => "base",
            Hands::LeftDown => "left_down",
            Hands::RightDown => "right_down",
            Hands::BothDown => "both_down",
        };
        FrameSelection {
            frame: frame.to_string(),
            effects: self.effects.clone(),
        }
    }
}
