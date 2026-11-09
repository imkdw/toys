//! 디버그 빌드 전용 트레이스 (PLAN 6.2).
//!
//! `TYPING_PET_TRACE=<파일>` 을 주면 받은 입력, 앞발/머리 상태, 그린 레이어, 프레임 시각을 한 줄씩 JSON 으로 남긴다.
//! 키 코드를 기록하므로 릴리스 빌드에는 들어가지 않는다 (함수가 전부 빈 몸체가 된다).

use std::time::Instant;

#[cfg(debug_assertions)]
mod imp {
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;

    pub struct Tracer {
        pub t0: Instant,
        pub out: Mutex<std::io::BufWriter<std::fs::File>>,
    }

    pub static TRACER: OnceLock<Option<Tracer>> = OnceLock::new();

    pub fn init(t0: Instant) {
        TRACER.get_or_init(|| {
            let path = std::env::var("TYPING_PET_TRACE").ok()?;
            let f = std::fs::File::create(&path).ok()?;
            log::info!("트레이스 기록: {path}");
            Some(Tracer { t0, out: Mutex::new(std::io::BufWriter::new(f)) })
        });
    }

    pub fn write(now: Instant, kind: &str, mut v: serde_json::Value) {
        let Some(Some(t)) = TRACER.get() else { return };
        if let Some(o) = v.as_object_mut() {
            o.insert("t_us".into(), (now.saturating_duration_since(t.t0).as_micros() as u64).into());
            o.insert("kind".into(), kind.into());
        }
        if let Ok(mut w) = t.out.lock() {
            let _ = writeln!(w, "{v}");
            let _ = w.flush();
        }
    }

    pub fn flush() {
        if let Some(Some(t)) = TRACER.get() {
            if let Ok(mut w) = t.out.lock() {
                let _ = w.flush();
            }
        }
    }

    pub fn enabled() -> bool {
        matches!(TRACER.get(), Some(Some(_)))
    }
}

pub fn init(t0: Instant) {
    #[cfg(debug_assertions)]
    imp::init(t0);
    #[cfg(not(debug_assertions))]
    let _ = t0;
}

pub fn enabled() -> bool {
    #[cfg(debug_assertions)]
    return imp::enabled();
    #[cfg(not(debug_assertions))]
    false
}

/// `f` 는 트레이스가 켜져 있을 때만 호출된다.
#[inline]
pub fn event(now: Instant, kind: &str, f: impl FnOnce() -> serde_json::Value) {
    #[cfg(debug_assertions)]
    if imp::enabled() {
        imp::write(now, kind, f());
    }
    #[cfg(not(debug_assertions))]
    let _ = (now, kind, f);
}

pub fn flush() {
    #[cfg(debug_assertions)]
    imp::flush();
}
