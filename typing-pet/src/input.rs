//! 전역 키/마우스 훅 -> 앱 이벤트.
//!
//! 원칙:
//! - 훅 스레드는 키 코드만 채널로 넘기고, 키 값을 저장하거나 기록하지 않는다.
//! - rdev 의 키 이름 변환은 쓰지 않는다 (macOS 별도 스레드 크래시 사례).

use winit::event_loop::EventLoopProxy;

use crate::AppEvent;

/// 화면 기준 어느 쪽 발로 칠지.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    /// 플랫폼 키 코드 (macOS virtual keycode).
    KeyDown(u32),
    MouseClick,
}

/// 키 훅 스레드를 띄운다. 권한이 없으면 경고만 남기고 앱은 계속 뜬다.
pub fn spawn(proxy: EventLoopProxy<AppEvent>) {
    // TODO: rdev::listen 또는 CGEventTap 으로 KeyPress/ButtonPress 수신
    // TODO: 입력 모니터링 권한 없을 때 메뉴바 경고
    let _ = proxy;
}

/// 키 코드로 좌우 판정. 실제 매핑은 manifest 에서 덮어쓴다.
pub fn side_of(keycode: u32, left_keys: &[u32]) -> Side {
    if left_keys.contains(&keycode) {
        Side::Left
    } else {
        Side::Right
    }
}
