//! 터미널 크기, 시그널, 화면 상태 복구.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

pub static STOP: AtomicBool = AtomicBool::new(false);
pub static RESIZE: AtomicBool = AtomicBool::new(false);

pub const ALT_ON: &str = "\x1b[?1049h";
pub const ALT_OFF: &str = "\x1b[?1049l";
pub const HIDE: &str = "\x1b[?25l";
pub const SHOW: &str = "\x1b[?25h";
pub const HOME: &str = "\x1b[H";
pub const CLEAR: &str = "\x1b[2J";
pub const RESET: &str = "\x1b[0m";
/// 동기화 출력: 지원하는 터미널은 프레임을 한 번에 그려 찢김이 없다. 나머지는 무시한다.
pub const SYNC_ON: &str = "\x1b[?2026h";
pub const SYNC_OFF: &str = "\x1b[?2026l";

pub fn stdout_is_tty() -> bool {
    unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 }
}

/// (가로 칸, 세로 줄). 터미널이 아니면 COLUMNS/LINES, 그것도 없으면 100x40.
pub fn size() -> (usize, usize) {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        for fd in [libc::STDOUT_FILENO, libc::STDERR_FILENO, libc::STDIN_FILENO] {
            if libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
                return (ws.ws_col as usize, ws.ws_row as usize);
            }
        }
    }
    let env = |k: &str, d: usize| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    (env("COLUMNS", 100), env("LINES", 40))
}

extern "C" fn on_stop(_: libc::c_int) {
    STOP.store(true, Ordering::SeqCst);
}

extern "C" fn on_resize(_: libc::c_int) {
    RESIZE.store(true, Ordering::SeqCst);
}

pub fn install_signals() {
    unsafe {
        libc::signal(libc::SIGINT, on_stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, on_stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGHUP, on_stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGWINCH, on_resize as *const () as libc::sighandler_t);
    }
}

pub fn stopped() -> bool {
    STOP.load(Ordering::SeqCst)
}

pub fn take_resize() -> bool {
    RESIZE.swap(false, Ordering::SeqCst)
}

pub fn put(s: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(s.as_bytes());
    let _ = out.flush();
}

/// 대체 화면 + 커서 숨김. 어떻게 끝나든 Drop 에서 원래대로 돌린다.
pub struct AltScreen;

impl AltScreen {
    pub fn enter() -> AltScreen {
        put(&format!("{ALT_ON}{HIDE}{CLEAR}{HOME}"));
        AltScreen
    }
}

impl Drop for AltScreen {
    fn drop(&mut self) {
        put(&format!("{SYNC_OFF}{RESET}{SHOW}{ALT_OFF}"));
    }
}

/// 일반 화면에서 커서만 숨긴다 (이미지 등장 효과용).
pub struct HiddenCursor;

impl HiddenCursor {
    pub fn new() -> HiddenCursor {
        put(HIDE);
        HiddenCursor
    }
}

impl Drop for HiddenCursor {
    fn drop(&mut self) {
        put(&format!("{RESET}{SHOW}"));
    }
}
