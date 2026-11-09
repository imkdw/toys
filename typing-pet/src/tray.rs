//! 메뉴바 아이콘과 메뉴 (tray-icon + muda).

/// 메뉴 항목 선택 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ToggleVisible,
    ToggleEditPosition,
    Quit,
}

pub struct Tray {
    // TODO: TrayIcon, 메뉴 항목 id -> TrayAction 매핑
}

impl Tray {
    pub fn new() -> Self {
        // TODO: 아이콘/메뉴 생성 (보이기/숨기기, 위치 편집, 종료)
        Self {}
    }

    /// 입력 모니터링 권한이 없을 때 메뉴에 경고 표시.
    pub fn set_permission_warning(&mut self, on: bool) {
        // TODO
        let _ = on;
    }
}
