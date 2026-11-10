//! 전역 키/마우스 감지 (CGEventTap) -> 앱 이벤트.
//!
//! 원칙:
//! - 탭은 읽기 전용(listen only). 입력을 막거나 바꾸지 않는다. "입력 모니터링" 권한만 필요.
//! - 감지 스레드는 이벤트 종류와 키 코드만 채널로 넘기고, 키 값을 저장하거나 기록하지 않는다.
//! - CGEvent -> 앱 이벤트 변환(자동 반복 제외, 수식키, 마우스 쓰로틀)은 [`Converter`] 순수 로직으로
//!   분리해서 OS 없이 테스트한다.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::config::Keys;

/// 앱이 쓰는 입력 이벤트.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    /// 키 누름 (macOS 가상 키 코드). 자동 반복은 이미 걸러진 상태.
    Key(u16),
    /// 마우스 이동/드래그/스크롤 (쓰로틀 후).
    MouseMove,
    MouseClick,
}

/// 커서 위치 (전역 좌표, 주 화면 좌상단 원점, pt). 캐릭터 위에 커서가 있을 때만 창이 클릭을 받게 하는 데 쓴다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cursor {
    Move(f64, f64),
    /// 왼쪽 버튼을 뗌 (드래그 끝).
    Up,
}

/// 화면 기준 키 위치.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Zone {
    Left,
    Center,
    Right,
}

pub fn zone_of(code: u16, keys: &Keys) -> Zone {
    if keys.left.contains(&code) {
        Zone::Left
    } else if keys.center.contains(&code) {
        Zone::Center
    } else {
        Zone::Right
    }
}

/// CGEvent 에서 뽑은 날것의 값. 테스트에서 직접 만든다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawEvent {
    KeyDown { code: u16, autorepeat: bool },
    KeyUp { code: u16 },
    /// 수식키 변경. flags 는 CGEventFlags 원값.
    FlagsChanged { code: u16, flags: u64 },
    MouseMove,
    MouseDrag,
    Scroll,
    MouseDown,
}

// CGEventFlags
pub const FLAG_CAPS: u64 = 0x0001_0000;
pub const FLAG_SHIFT: u64 = 0x0002_0000;
pub const FLAG_CTRL: u64 = 0x0004_0000;
pub const FLAG_ALT: u64 = 0x0008_0000;
pub const FLAG_CMD: u64 = 0x0010_0000;
pub const FLAG_FN: u64 = 0x0080_0000;
// 장치별(좌/우) 비트 (IOKit NX_DEVICE*KEYMASK)
pub const DEV_LCTRL: u64 = 0x0000_0001;
pub const DEV_LSHIFT: u64 = 0x0000_0002;
pub const DEV_RSHIFT: u64 = 0x0000_0004;
pub const DEV_LCMD: u64 = 0x0000_0008;
pub const DEV_RCMD: u64 = 0x0000_0010;
pub const DEV_LALT: u64 = 0x0000_0020;
pub const DEV_RALT: u64 = 0x0000_0040;
pub const DEV_RCTRL: u64 = 0x0000_2000;

/// 수식키 키 코드와, 눌린 상태일 때 켜지는 플래그 비트.
/// 장치별 비트로 좌/우를 구분하고, 그 비트가 없는 키보드면 일반 비트로 판정한다.
pub const MODIFIERS: [(u16, &str, u64, u64); 10] = [
    (56, "LShift", DEV_LSHIFT, FLAG_SHIFT),
    (60, "RShift", DEV_RSHIFT, FLAG_SHIFT),
    (59, "LCtrl", DEV_LCTRL, FLAG_CTRL),
    (62, "RCtrl", DEV_RCTRL, FLAG_CTRL),
    (58, "LOption", DEV_LALT, FLAG_ALT),
    (61, "ROption", DEV_RALT, FLAG_ALT),
    (55, "LCmd", DEV_LCMD, FLAG_CMD),
    (54, "RCmd", DEV_RCMD, FLAG_CMD),
    (57, "CapsLock", 0, FLAG_CAPS),
    (63, "Fn", 0, FLAG_FN),
];

const DEV_MASK: u64 = DEV_LCTRL | DEV_LSHIFT | DEV_RSHIFT | DEV_LCMD | DEV_RCMD | DEV_LALT | DEV_RALT | DEV_RCTRL;

/// 수식키 변경 이벤트가 "누름" 인지 판정. 뗄 때는 false.
pub fn modifier_pressed(code: u16, flags: u64) -> bool {
    let Some(&(_, _, dev, generic)) = MODIFIERS.iter().find(|m| m.0 == code) else {
        return false;
    };
    // Caps Lock 은 상태가 토글될 때마다 한 번만 온다. 켤 때/끌 때 모두 누름으로 본다.
    if code == 57 {
        return true;
    }
    if dev != 0 && flags & DEV_MASK != 0 {
        flags & dev != 0
    } else {
        flags & generic != 0
    }
}

/// CGEvent -> 앱 이벤트 변환기 (감지 스레드에서 사용).
#[derive(Debug, Clone)]
pub struct Converter {
    throttle: Duration,
    last_mouse: Option<Instant>,
}

impl Converter {
    pub fn new(mouse_throttle: Duration) -> Self {
        Self { throttle: mouse_throttle, last_mouse: None }
    }

    pub fn convert(&mut self, raw: RawEvent, now: Instant) -> Option<Input> {
        match raw {
            RawEvent::KeyDown { autorepeat: true, .. } => None,
            RawEvent::KeyDown { code, .. } => Some(Input::Key(code)),
            RawEvent::KeyUp { .. } => None,
            RawEvent::FlagsChanged { code, flags } => modifier_pressed(code, flags).then_some(Input::Key(code)),
            RawEvent::MouseMove | RawEvent::MouseDrag | RawEvent::Scroll => {
                if self.last_mouse.is_some_and(|t| now.duration_since(t) < self.throttle) {
                    None
                } else {
                    self.last_mouse = Some(now);
                    Some(Input::MouseMove)
                }
            }
            RawEvent::MouseDown => Some(Input::MouseClick),
        }
    }
}

#[cfg(target_os = "macos")]
pub use tap::{has_listen_permission, request_listen_permission, spawn};

/// 커서 위치를 이 간격보다 자주 보내지 않는다 (호버 판정용이라 60Hz 면 충분).
pub const CURSOR_INTERVAL: Duration = Duration::from_millis(16);

#[cfg(target_os = "macos")]
mod tap {
    //! CGEventTap 직접 FFI. rdev 는 키마다 TIS 함수를 불러 별도 스레드에서 크래시할 수 있어 쓰지 않는다.

    use std::ffi::c_void;
    use std::sync::atomic::{AtomicPtr, Ordering};
    use std::time::{Duration, Instant};

    use super::{CURSOR_INTERVAL, Converter, Cursor, Input, RawEvent};

    type CFMachPortRef = *mut c_void;
    type CFRunLoopSourceRef = *mut c_void;
    type CFRunLoopRef = *mut c_void;
    type CGEventRef = *mut c_void;
    type CGEventTapProxy = *mut c_void;
    type TapCallback = extern "C" fn(CGEventTapProxy, u32, CGEventRef, *mut c_void) -> CGEventRef;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: TapCallback,
            user_info: *mut c_void,
        ) -> CFMachPortRef;
        fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
        fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
        fn CGEventGetFlags(event: CGEventRef) -> u64;
        fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
        fn CGPreflightListenEventAccess() -> bool;
        fn CGRequestListenEventAccess() -> bool;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFMachPortCreateRunLoopSource(alloc: *const c_void, port: CFMachPortRef, order: isize) -> CFRunLoopSourceRef;
        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: *const c_void);
        fn CFRunLoopRun();
        static kCFRunLoopCommonModes: *const c_void;
    }

    const K_CG_SESSION_EVENT_TAP: u32 = 1;
    const K_CG_HEAD_INSERT: u32 = 0;
    const K_CG_LISTEN_ONLY: u32 = 1;

    const LEFT_DOWN: u32 = 1;
    const LEFT_UP: u32 = 2;
    const RIGHT_DOWN: u32 = 3;
    const MOUSE_MOVED: u32 = 5;
    const LEFT_DRAGGED: u32 = 6;
    const RIGHT_DRAGGED: u32 = 7;
    const KEY_DOWN: u32 = 10;
    const KEY_UP: u32 = 11;
    const FLAGS_CHANGED: u32 = 12;
    const SCROLL: u32 = 22;
    const OTHER_DOWN: u32 = 25;
    const OTHER_DRAGGED: u32 = 27;
    const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
    const TAP_DISABLED_BY_USER: u32 = 0xFFFF_FFFF;

    const FIELD_AUTOREPEAT: u32 = 8;
    const FIELD_KEYCODE: u32 = 9;

    static TAP: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

    struct Ctx {
        conv: Converter,
        send: Box<dyn Fn(Input, Instant) + Send>,
        cursor: Box<dyn Fn(Cursor) + Send>,
        last_cursor: Option<Instant>,
    }

    pub fn has_listen_permission() -> bool {
        unsafe { CGPreflightListenEventAccess() }
    }

    /// 시스템 권한 요청 창을 띄운다 (이미 거부됐으면 아무 일도 없다).
    pub fn request_listen_permission() -> bool {
        unsafe { CGRequestListenEventAccess() }
    }

    extern "C" fn callback(_proxy: CGEventTapProxy, ty: u32, event: CGEventRef, user: *mut c_void) -> CGEventRef {
        // 콜백은 채널에 넣고 바로 반환한다. 느리면 macOS 가 탭을 끈다.
        let ctx = unsafe { &mut *(user as *mut Ctx) };
        let now = Instant::now();
        let raw = match ty {
            TAP_DISABLED_BY_TIMEOUT | TAP_DISABLED_BY_USER => {
                let tap = TAP.load(Ordering::Acquire);
                if !tap.is_null() {
                    unsafe { CGEventTapEnable(tap, true) };
                }
                log::warn!("이벤트 탭이 꺼져서 다시 켬 (type={ty:#x})");
                return event;
            }
            LEFT_UP => {
                (ctx.cursor)(Cursor::Up);
                return event;
            }
            KEY_DOWN => RawEvent::KeyDown {
                code: unsafe { CGEventGetIntegerValueField(event, FIELD_KEYCODE) } as u16,
                autorepeat: unsafe { CGEventGetIntegerValueField(event, FIELD_AUTOREPEAT) } != 0,
            },
            KEY_UP => RawEvent::KeyUp { code: unsafe { CGEventGetIntegerValueField(event, FIELD_KEYCODE) } as u16 },
            FLAGS_CHANGED => RawEvent::FlagsChanged {
                code: unsafe { CGEventGetIntegerValueField(event, FIELD_KEYCODE) } as u16,
                flags: unsafe { CGEventGetFlags(event) },
            },
            MOUSE_MOVED => RawEvent::MouseMove,
            LEFT_DRAGGED | RIGHT_DRAGGED | OTHER_DRAGGED => RawEvent::MouseDrag,
            SCROLL => RawEvent::Scroll,
            LEFT_DOWN | RIGHT_DOWN | OTHER_DOWN => RawEvent::MouseDown,
            _ => return event,
        };
        if matches!(raw, RawEvent::MouseMove | RawEvent::MouseDrag | RawEvent::MouseDown)
            && (raw == RawEvent::MouseDown || ctx.last_cursor.is_none_or(|t| now.duration_since(t) >= CURSOR_INTERVAL))
        {
            ctx.last_cursor = Some(now);
            let p = unsafe { CGEventGetLocation(event) };
            (ctx.cursor)(Cursor::Move(p.x, p.y));
        }
        if let Some(input) = ctx.conv.convert(raw, now) {
            (ctx.send)(input, now);
        }
        event
    }

    /// 감지 스레드를 띄운다. 탭을 못 만들면(권한 없음) Err.
    pub fn spawn(
        mouse_throttle: Duration,
        send: impl Fn(Input, Instant) + Send + 'static,
        cursor: impl Fn(Cursor) + Send + 'static,
    ) -> Result<(), String> {
        let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
        std::thread::Builder::new()
            .name("event-tap".into())
            .spawn(move || {
                let mask: u64 = [
                    LEFT_DOWN, LEFT_UP, RIGHT_DOWN, OTHER_DOWN, MOUSE_MOVED, LEFT_DRAGGED, RIGHT_DRAGGED, OTHER_DRAGGED,
                    KEY_DOWN, KEY_UP, FLAGS_CHANGED, SCROLL,
                ]
                .iter()
                .fold(0, |m, t| m | (1u64 << t));
                let ctx = Box::into_raw(Box::new(Ctx {
                    conv: Converter::new(mouse_throttle),
                    send: Box::new(send),
                    cursor: Box::new(cursor),
                    last_cursor: None,
                }));
                let tap = unsafe {
                    CGEventTapCreate(K_CG_SESSION_EVENT_TAP, K_CG_HEAD_INSERT, K_CG_LISTEN_ONLY, mask, callback, ctx as *mut c_void)
                };
                if tap.is_null() {
                    drop(unsafe { Box::from_raw(ctx) });
                    let _ = tx.send(Err("CGEventTapCreate 실패 (입력 모니터링 권한 필요)".into()));
                    return;
                }
                TAP.store(tap, Ordering::Release);
                unsafe {
                    let src = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
                    CFRunLoopAddSource(CFRunLoopGetCurrent(), src, kCFRunLoopCommonModes);
                    CGEventTapEnable(tap, true);
                }
                let _ = tx.send(Ok(()));
                unsafe { CFRunLoopRun() };
            })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }
}
