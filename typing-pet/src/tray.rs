//! 메뉴바 아이콘과 메뉴 (tray-icon + muda).

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::config::{Config, SIZES};

/// 메뉴 항목 선택 결과.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayAction {
    ToggleVisible,
    ToggleBreathing,
    Size(f64),
    /// `Tray::characters` 의 순번.
    Character(usize),
    ToggleLoginItem,
    OpenPermissionSettings,
    Quit,
}

pub struct Tray {
    _icon: TrayIcon,
    menu: Menu,
    visible: CheckMenuItem,
    breathing: CheckMenuItem,
    login: CheckMenuItem,
    sizes: Vec<(f64, CheckMenuItem)>,
    /// (폴더 이름, 메뉴 항목)
    characters: Vec<(String, CheckMenuItem)>,
    warning: MenuItem,
    warning_shown: bool,
    ids: Vec<(tray_icon::menu::MenuId, TrayAction)>,
}

/// 메뉴바용 템플릿 아이콘 (토끼 머리 실루엣). 검정 + 알파, macOS 가 다크/라이트에 맞춰 칠한다.
fn icon() -> Icon {
    let n = 44u32;
    let mut rgba = vec![0u8; (n * n * 4) as usize];
    let inside = |x: f32, y: f32| -> f32 {
        // 머리 원
        let head = ((x - 22.0).powi(2) + (y - 29.0).powi(2)).sqrt() - 13.0;
        // 귀 두 개 (세로 타원)
        let ear = |cx: f32| (((x - cx) / 4.2).powi(2) + ((y - 11.0) / 10.0).powi(2)).sqrt() - 1.0;
        let e = ear(16.0).min(ear(28.0)) * 4.2;
        head.min(e)
    };
    for y in 0..n {
        for x in 0..n {
            // 2x2 슈퍼샘플링으로 가장자리 부드럽게
            let mut a = 0.0;
            for (dx, dy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                if inside(x as f32 + dx, y as f32 + dy) <= 0.0 {
                    a += 0.25;
                }
            }
            let i = ((y * n + x) * 4) as usize;
            rgba[i + 3] = (a * 255.0) as u8;
        }
    }
    Icon::from_rgba(rgba, n, n).expect("아이콘")
}

impl Tray {
    /// `characters`: (폴더 이름, 메뉴 이름) 목록, `current`: 지금 캐릭터 폴더 이름.
    pub fn new(
        cfg: &Config,
        characters: &[(String, String)],
        current: &str,
        send: impl Fn(TrayAction) + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let menu = Menu::new();
        let char_menu = Submenu::new("캐릭터", characters.len() > 1);
        let mut chars = Vec::new();
        for (name, title) in characters {
            let item = CheckMenuItem::new(title, true, name == current, None);
            char_menu.append(&item).map_err(|e| e.to_string())?;
            chars.push((name.clone(), item));
        }
        let visible = CheckMenuItem::new("보이기", true, cfg.visible, None);
        let breathing = CheckMenuItem::new("숨쉬기", true, cfg.breathing, None);
        let size_menu = Submenu::new("크기", true);
        let mut sizes = Vec::new();
        for s in SIZES {
            let item = CheckMenuItem::new(format!("{s:.0} pt"), true, (cfg.size - s).abs() < 0.5, None);
            size_menu.append(&item).map_err(|e| e.to_string())?;
            sizes.push((s, item));
        }
        let login = CheckMenuItem::new("로그인 시 자동 실행", crate::mac::bundle_path().is_some(), crate::mac::login_item_enabled(), None);
        let warning = MenuItem::new("⚠ 입력 모니터링 권한 필요 (설정 열기)", true, None);
        let quit = MenuItem::new("종료", true, None);
        menu.append_items(&[&char_menu, &visible, &breathing, &size_menu, &login, &PredefinedMenuItem::separator(), &quit])
            .map_err(|e| e.to_string())?;

        let mut ids = vec![
            (visible.id().clone(), TrayAction::ToggleVisible),
            (breathing.id().clone(), TrayAction::ToggleBreathing),
            (login.id().clone(), TrayAction::ToggleLoginItem),
            (warning.id().clone(), TrayAction::OpenPermissionSettings),
            (quit.id().clone(), TrayAction::Quit),
        ];
        for (s, item) in &sizes {
            ids.push((item.id().clone(), TrayAction::Size(*s)));
        }
        for (i, (_, item)) in chars.iter().enumerate() {
            ids.push((item.id().clone(), TrayAction::Character(i)));
        }
        let map = ids.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if let Some((_, a)) = map.iter().find(|(id, _)| *id == e.id) {
                send(*a);
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_icon_templated(icon())
            .with_tooltip("typing-pet")
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { _icon: icon, menu, visible, breathing, login, sizes, characters: chars, warning, warning_shown: false, ids })
    }

    /// 순번 -> 캐릭터 폴더 이름.
    pub fn character(&self, i: usize) -> Option<&str> {
        self.characters.get(i).map(|(n, _)| n.as_str())
    }

    /// 체크 표시를 지금 캐릭터에 맞춘다 (바꾸기 실패하면 이전 캐릭터로 되돌리는 데도 쓴다).
    pub fn set_character(&self, current: &str) {
        for (n, item) in &self.characters {
            item.set_checked(n == current);
        }
    }

    pub fn sync(&self, cfg: &Config) {
        self.visible.set_checked(cfg.visible);
        self.breathing.set_checked(cfg.breathing);
        self.login.set_checked(crate::mac::login_item_enabled());
        for (s, item) in &self.sizes {
            item.set_checked((cfg.size - s).abs() < 0.5);
        }
    }

    /// 입력 모니터링 권한이 없을 때 메뉴 맨 위에 경고 표시.
    pub fn set_permission_warning(&mut self, on: bool) {
        if on && !self.warning_shown {
            let _ = self.menu.prepend(&self.warning);
            self.warning_shown = true;
        } else if !on && self.warning_shown {
            let _ = self.menu.remove(&self.warning);
            self.warning_shown = false;
        }
    }

    pub fn action_count(&self) -> usize {
        self.ids.len()
    }
}
