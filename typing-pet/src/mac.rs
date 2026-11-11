//! macOS 창 설정 (PLAN 5.2).
//!
//! winit 의 AlwaysOnTop 만으로는 전체화면 앱 위와 다른 Space 에서 안 보인다.
//! NSWindow 레벨을 올리고 `canJoinAllSpaces | fullScreenAuxiliary | stationary` 를 준다.

use objc2::rc::Retained;
use objc2_app_kit::{NSAccessibility as _, NSApplicationActivationOptions, NSRunningApplication, NSView, NSWindow, NSWindowCollectionBehavior, NSWorkspace};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

fn ns_window(window: &Window) -> Option<objc2::rc::Retained<NSWindow>> {
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(h) = handle.as_raw() else { return None };
    // SAFETY: winit 이 살아 있는 NSView 포인터를 준다. 메인 스레드에서만 부른다.
    let view: &NSView = unsafe { h.ns_view.cast().as_ref() };
    view.window()
}

pub fn setup_overlay(window: &Window) {
    let Some(w) = ns_window(window) else {
        log::warn!("NSWindow 를 얻지 못함");
        return;
    };
    w.setLevel(objc2_app_kit::NSStatusWindowLevel);
    w.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    w.setHasShadow(false);
    w.setOpaque(false);
    // 접근성 트리에서 창을 뺀다. Rectangle 같은 창 관리 앱은 맨 앞 앱에 창이 없으면
    // 화면 맨 위 창(레벨이 높은 이 창)을 대상으로 잡아 옮기고 크기를 바꿔 버린다.
    // 접근성 요소가 아니면 앱의 AXWindows/AXFocusedWindow 에 안 나오므로 건드릴 수 없다.
    w.setAccessibilityElement(false);
}

/// CGWindowID (screencapture -l 에 쓴다).
pub fn window_number(window: &Window) -> Option<isize> {
    ns_window(window).map(|w| w.windowNumber())
}

/// 지금 맨 앞 앱 (자기 자신이면 None). 캐릭터를 끌어 옮긴 뒤 포커스를 돌려주는 데 쓴다.
pub fn frontmost_app() -> Option<Retained<NSRunningApplication>> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    let me = NSRunningApplication::currentApplication();
    (app.processIdentifier() != me.processIdentifier()).then_some(app)
}

pub fn activate(app: &NSRunningApplication) {
    app.activateWithOptions(NSApplicationActivationOptions::empty());
}

/// 시스템 설정의 "입력 모니터링" 화면을 연다.
pub fn open_input_monitoring_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        .spawn();
}

const LAUNCH_AGENT: &str = "dev.imkdw.typing-pet";

fn launch_agent_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join("Library/LaunchAgents").join(format!("{LAUNCH_AGENT}.plist")))
}

/// .app 번들 안에서 실행 중이면 번들 경로 (로그인 항목은 번들로만 등록한다).
pub fn bundle_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let app = exe.parent()?.parent()?.parent()?;
    (app.extension().is_some_and(|e| e == "app")).then(|| app.to_path_buf())
}

pub fn login_item_enabled() -> bool {
    launch_agent_path().is_some_and(|p| p.exists())
}

/// 로그인 시 자동 실행 켜기/끄기. ~/Library/LaunchAgents 에 plist 를 두면 다음 로그인부터 실행된다.
pub fn set_login_item(on: bool) -> std::io::Result<()> {
    let Some(path) = launch_agent_path() else { return Ok(()) };
    if !on {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        return Ok(());
    }
    let app = bundle_path().ok_or_else(|| std::io::Error::other(".app 번들로 실행할 때만 켤 수 있음"))?;
    std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")))?;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LAUNCH_AGENT}</string>
  <key>ProgramArguments</key>
  <array><string>/usr/bin/open</string><string>-a</string><string>{}</string></array>
  <key>RunAtLoad</key><true/>
</dict>
</plist>
"#,
        app.display()
    );
    std::fs::write(path, plist)
}
