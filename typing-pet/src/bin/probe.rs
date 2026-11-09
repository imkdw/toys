//! 실제 macOS 키/마우스 이벤트를 보내고, 앱 트레이스로 반응을 확인하는 검증 도구 (PLAN 6.4).
//!
//! 사용법: probe <all-keys|modifiers|repeat|mouse|burst> --trace <앱 트레이스 파일> [--out <결과 json>] [--burst-secs N]
//!
//! 안전 장치:
//! - 먼저 자기 전용 빈 창을 띄워 포커스를 가져간 뒤 보낸다. 맨 앞 앱이 자기가 아니면 중단한다.
//! - 위험한 키(볼륨, Fn, Show Desktop)는 보내지 않는다. 조합키(Cmd+Q 등)는 절대 보내지 않는다.
//! - Caps Lock 은 두 번 보내 원래 상태로 되돌린다.

use std::ffi::c_void;
use std::io::BufRead;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use typing_pet::config::Manifest;
use typing_pet::input::{self, MODIFIERS, Zone, zone_of};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

type CGEventRef = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(src: *const c_void, key: u16, down: bool) -> CGEventRef;
    fn CGEventCreateMouseEvent(src: *const c_void, ty: u32, pos: CGPoint, button: u32) -> CGEventRef;
    fn CGEventCreateScrollWheelEvent(src: *const c_void, units: u32, count: u32, w1: i32, ...) -> CGEventRef;
    fn CGEventSetIntegerValueField(ev: CGEventRef, field: u32, value: i64);
    fn CGEventSetType(ev: CGEventRef, ty: u32);
    fn CGEventSetFlags(ev: CGEventRef, flags: u64);
    fn CGEventPost(tap: u32, ev: CGEventRef);
    fn CGPreflightPostEventAccess() -> bool;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(p: *const c_void);
}

const HID_TAP: u32 = 0;
const FLAGS_CHANGED: u32 = 12;
const FIELD_AUTOREPEAT: u32 = 8;

/// 보내지 않는 키: 볼륨(72~74), Fn(63, 이모지/받아쓰기 창), F11(103, 데스크탑 보기), 전원(127, ADB Power),
/// 수식키(54~62, modifiers 에서 따로 검사).
const SKIP: [u16; 15] = [72, 73, 74, 63, 103, 127, 54, 55, 56, 57, 58, 59, 60, 61, 62];

fn post(ev: CGEventRef) {
    if ev.is_null() {
        return;
    }
    unsafe {
        CGEventPost(HID_TAP, ev);
        CFRelease(ev);
    }
}

fn key(code: u16, down: bool, repeat: bool) {
    let ev = unsafe { CGEventCreateKeyboardEvent(std::ptr::null(), code, down) };
    if repeat {
        unsafe { CGEventSetIntegerValueField(ev, FIELD_AUTOREPEAT, 1) };
    }
    unsafe { CGEventSetFlags(ev, 0) };
    post(ev);
}

fn modifier(code: u16, down: bool) {
    let (_, _, dev, generic) = *MODIFIERS.iter().find(|m| m.0 == code).expect("수식키");
    let ev = unsafe { CGEventCreateKeyboardEvent(std::ptr::null(), code, down) };
    unsafe {
        CGEventSetType(ev, FLAGS_CHANGED);
        CGEventSetFlags(ev, if down { dev | generic } else { 0 });
    }
    post(ev);
}

fn mouse(ty: u32, x: f64, y: f64, button: u32) {
    post(unsafe { CGEventCreateMouseEvent(std::ptr::null(), ty, CGPoint { x, y }, button) });
}

fn scroll(dy: i32) {
    post(unsafe { CGEventCreateScrollWheelEvent(std::ptr::null(), 0, 1, dy) });
}

fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

fn frontmost_is_me() -> bool {
    use objc2_app_kit::NSWorkspace;
    let ws = NSWorkspace::sharedWorkspace();
    ws.frontmostApplication().is_some_and(|a| a.processIdentifier() == std::process::id() as i32)
}

// ---------- 트레이스 읽기 ----------

#[derive(Debug, Clone)]
struct Line {
    t_us: u64,
    kind: String,
    v: Value,
}

fn read_trace(path: &str, skip: usize) -> Vec<Line> {
    let Ok(f) = std::fs::File::open(path) else { return vec![] };
    std::io::BufReader::new(f)
        .lines()
        .skip(skip)
        .map_while(Result::ok)
        .filter_map(|l| serde_json::from_str::<Value>(&l).ok())
        .map(|v| Line { t_us: v["t_us"].as_u64().unwrap_or(0), kind: v["kind"].as_str().unwrap_or("").to_string(), v })
        .collect()
}

fn trace_len(path: &str) -> usize {
    std::fs::File::open(path).map(|f| std::io::BufReader::new(f).lines().count()).unwrap_or(0)
}

fn input_key(l: &Line) -> Option<u16> {
    (l.kind == "input").then(|| l.v["input"]["Key"].as_u64().map(|k| k as u16)).flatten()
}

fn input_name(l: &Line) -> Option<String> {
    if l.kind != "input" {
        return None;
    }
    match &l.v["input"] {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o.keys().next().cloned(),
        _ => None,
    }
}

// ---------- 시나리오 ----------

struct Ctx {
    trace: String,
    win: (f64, f64, f64, f64),
    keys: typing_pet::config::Keys,
    paw_ms: u64,
    burst_secs: u64,
}

#[derive(Default)]
struct Report {
    sent: usize,
    received: usize,
    failures: Vec<String>,
    notes: Vec<String>,
    skipped: Vec<u16>,
}

/// 키 하나에 대해 "입력 수신 -> 기대한 발 down -> 약 120ms 후 up" 확인.
fn check_key_reaction(lines: &[Line], idx: usize, code: u16, keys: &typing_pet::config::Keys, paw_ms: u64, r: &mut Report) {
    let t_in = lines[idx].t_us;
    let zone = zone_of(code, keys);
    let (field, down) = match zone {
        Zone::Left => ("paw_l", "down"),
        Zone::Center => ("paw_r", "center"),
        Zone::Right => ("paw_r", "down"),
    };
    let states: Vec<&Line> = lines[idx + 1..].iter().filter(|l| l.kind == "state").collect();
    let Some(first) = states.first() else {
        r.failures.push(format!("key {code}: 상태 변화 없음"));
        return;
    };
    if first.v[field] != down {
        r.failures.push(format!("key {code}: {field} 기대 {down}, 실제 {}", first.v[field]));
        return;
    }
    match states.iter().find(|l| l.v[field] != down) {
        Some(up) => {
            let dt = (up.t_us - t_in) / 1000;
            if dt + 2 < paw_ms || dt > paw_ms + 120 {
                r.failures.push(format!("key {code}: {field} 복귀 시간 {dt}ms (기대 {paw_ms}ms)"));
            }
        }
        None => r.failures.push(format!("key {code}: {field} 가 up 으로 돌아오지 않음")),
    }
}

fn run_all_keys(c: &Ctx, r: &mut Report) {
    let start = trace_len(&c.trace);
    let mut sent = Vec::new();
    for code in 0u16..128 {
        if SKIP.contains(&code) {
            r.skipped.push(code);
            continue;
        }
        key(code, true, false);
        sleep_ms(25);
        key(code, false, false);
        sent.push(code);
        sleep_ms(c.paw_ms + 140);
    }
    sleep_ms(800);
    let lines = read_trace(&c.trace, start);
    let got: Vec<(usize, u16)> = lines.iter().enumerate().filter_map(|(i, l)| input_key(l).map(|k| (i, k))).collect();
    r.sent = sent.len();
    r.received = got.len();
    let got_codes: Vec<u16> = got.iter().map(|g| g.1).collect();
    if got_codes != sent {
        let missing: Vec<u16> = sent.iter().copied().filter(|k| !got_codes.contains(k)).collect();
        r.failures.push(format!("받은 키 순서/개수 불일치: 보냄 {} 받음 {} 빠짐 {missing:?}", sent.len(), got.len()));
    }
    for (i, code) in got {
        check_key_reaction(&lines, i, code, &c.keys, c.paw_ms, r);
    }
}

fn run_modifiers(c: &Ctx, r: &mut Report) {
    let start = trace_len(&c.trace);
    let mut sent = Vec::new();
    for &(code, name, ..) in MODIFIERS.iter() {
        if code == 63 {
            r.notes.push("Fn 은 시스템 동작(이모지/받아쓰기) 위험으로 보내지 않음 (단위 테스트로 검증)".into());
            r.skipped.push(code);
            continue;
        }
        if code == 57 {
            // Caps Lock 은 두 번 (켜고 끄기) -> 누름 2회
            for on in [true, false] {
                let ev = unsafe { CGEventCreateKeyboardEvent(std::ptr::null(), 57, true) };
                unsafe {
                    CGEventSetType(ev, FLAGS_CHANGED);
                    CGEventSetFlags(ev, if on { input::FLAG_CAPS } else { 0 });
                }
                post(ev);
                sent.push(57);
                sleep_ms(c.paw_ms + 140);
            }
            continue;
        }
        let _ = name;
        modifier(code, true);
        sleep_ms(30);
        modifier(code, false);
        sent.push(code);
        sleep_ms(c.paw_ms + 140);
    }
    sleep_ms(800);
    let lines = read_trace(&c.trace, start);
    let got: Vec<(usize, u16)> = lines.iter().enumerate().filter_map(|(i, l)| input_key(l).map(|k| (i, k))).collect();
    r.sent = sent.len();
    r.received = got.len();
    let got_codes: Vec<u16> = got.iter().map(|g| g.1).collect();
    if got_codes != sent {
        r.failures.push(format!("수식키: 보냄 {sent:?} 받음 {got_codes:?} (뗄 때는 무시돼야 함)"));
    }
    for (i, code) in got {
        check_key_reaction(&lines, i, code, &c.keys, c.paw_ms, r);
    }
}

fn run_repeat(c: &Ctx, r: &mut Report) {
    let start = trace_len(&c.trace);
    // J(38) 를 꾹 누른 것처럼: 일반 down 1번 + 자동 반복 down 20번 + up
    key(38, true, false);
    for _ in 0..20 {
        sleep_ms(33);
        key(38, true, true);
    }
    key(38, false, false);
    sleep_ms(800);
    let lines = read_trace(&c.trace, start);
    let got = lines.iter().filter(|l| input_key(l).is_some()).count();
    r.sent = 1;
    r.received = got;
    if got != 1 {
        r.failures.push(format!("자동 반복: 반응 {got}번 (기대 1번)"));
    }
}

fn run_mouse(c: &Ctx, r: &mut Report) {
    let (x, y, w, h) = c.win;
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let start = trace_len(&c.trace);
    // 1) 이동 50번을 10ms 간격 -> 쓰로틀(100ms) 후 약 5~6번
    for i in 0..50 {
        let a = i as f64 / 50.0 * std::f64::consts::TAU;
        mouse(5, cx + a.cos() * w * 0.3, cy + a.sin() * h * 0.3, 0);
        sleep_ms(10);
    }
    sleep_ms(1500);
    let l1 = read_trace(&c.trace, start);
    let moves = l1.iter().filter(|l| input_name(l).as_deref() == Some("MouseMove")).count();
    if !(4..=7).contains(&moves) {
        r.failures.push(format!("이동 쓰로틀: 50번 보냄 -> {moves}번 (기대 4~7)"));
    }
    let states: Vec<&Line> = l1.iter().filter(|l| l.kind == "state").collect();
    if !states.iter().any(|s| s.v["paw_r"] == "mouse") {
        r.failures.push("마우스 이동 후 paw_r 이 mouse 가 아님".into());
    }
    if let (Some(first_mouse), Some(last)) = (states.iter().position(|s| s.v["paw_r"] == "mouse"), states.last()) {
        if last.v["paw_r"] != "up" {
            r.failures.push(format!("마우스 멈춘 뒤 1초 후 paw_r 이 up 이 아님: {}", last.v["paw_r"]));
        }
        let _ = first_mouse;
    }
    // 2) 클릭 (자기 창 안, 왼쪽/오른쪽)
    for (down, up, btn) in [(1u32, 2u32, 0u32), (3, 4, 1)] {
        let s2 = trace_len(&c.trace);
        mouse(down, cx, cy, btn);
        sleep_ms(20);
        mouse(up, cx, cy, btn);
        sleep_ms(1500);
        let l = read_trace(&c.trace, s2);
        let clicks = l.iter().filter(|l| input_name(l).as_deref() == Some("MouseClick")).count();
        if clicks != 1 {
            r.failures.push(format!("클릭(button {btn}): 반응 {clicks}번 (기대 1)"));
        }
        let seq: Vec<String> = l.iter().filter(|l| l.kind == "state").map(|s| s.v["paw_r"].as_str().unwrap_or("").to_string()).collect();
        let mut dedup = seq.clone();
        dedup.dedup();
        let want = ["mouse_click", "mouse", "up"];
        if !dedup.ends_with(&want.map(String::from)) {
            r.failures.push(format!("클릭(button {btn}): paw_r 순서 {dedup:?} (기대 ...{want:?})"));
        }
        r.sent += 1;
        r.received += clicks;
    }
    // 3) 드래그와 스크롤
    let s3 = trace_len(&c.trace);
    mouse(1, cx, cy, 0);
    for i in 0..10 {
        mouse(6, cx + i as f64 * 3.0, cy, 0);
        sleep_ms(30);
    }
    mouse(2, cx + 30.0, cy, 0);
    sleep_ms(300);
    for _ in 0..5 {
        scroll(-1);
        sleep_ms(150);
    }
    sleep_ms(1500);
    let l = read_trace(&c.trace, s3);
    let moves = l.iter().filter(|l| input_name(l).as_deref() == Some("MouseMove")).count();
    if moves < 4 {
        r.failures.push(format!("드래그/스크롤 반응 {moves}번 (기대 4번 이상)"));
    }
    // 4) 마우스 중 오른쪽 키 -> 키보드로 갔다가 다시 mouse
    let s4 = trace_len(&c.trace);
    mouse(5, cx, cy, 0);
    sleep_ms(150);
    key(37, true, false); // L (오른쪽)
    key(37, false, false);
    sleep_ms(400);
    let l = read_trace(&c.trace, s4);
    let mut seq: Vec<String> = l.iter().filter(|l| l.kind == "state").map(|s| s.v["paw_r"].as_str().unwrap_or("").to_string()).collect();
    seq.dedup();
    if !seq.windows(3).any(|w| w == ["mouse", "down", "mouse"]) {
        r.failures.push(format!("마우스 중 오른쪽 키: paw_r 순서 {seq:?} (기대 mouse -> down -> mouse)"));
    }
    sleep_ms(1200);
}

fn run_burst(c: &Ctx, r: &mut Report) {
    // 실제 타이핑 속도로 무작위 글자 입력. 빠르게 올렸다가 멈춰서 식힌다.
    let letters: Vec<u16> = vec![0, 1, 2, 3, 5, 4, 38, 40, 37, 41, 12, 13, 14, 15, 17, 16, 32, 34, 31, 35, 6, 7, 8, 9, 11, 45, 46, 49];
    let total = c.burst_secs.max(30) as f64;
    // 구간: (비율, 초당 키 수)
    let phases = [(0.15, 4.0), (0.25, 8.0), (0.35, 12.0), (0.25, 0.0)];
    let start = trace_len(&c.trace);
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut sent = 0usize;
    let mut last: Option<u16> = None;
    for (frac, kps) in phases {
        let end = Instant::now() + Duration::from_secs_f64(total * frac);
        while Instant::now() < end {
            if kps == 0.0 {
                sleep_ms(100);
                continue;
            }
            let mut k = letters[(rnd() % letters.len() as u64) as usize];
            if Some(k) == last {
                k = letters[(rnd() % letters.len() as u64) as usize];
            }
            last = Some(k);
            key(k, true, false);
            sleep_ms(15);
            key(k, false, false);
            sent += 1;
            // 간격에 ±40% 흔들림
            let base = 1000.0 / kps;
            let jitter = (rnd() % 800) as f64 / 1000.0 - 0.4;
            sleep_ms((base * (1.0 + jitter)).max(20.0) as u64 - 15);
        }
    }
    sleep_ms(3000);
    let lines = read_trace(&c.trace, start);
    let got = lines.iter().filter(|l| input_key(l).is_some()).count();
    r.sent = sent;
    r.received = got;
    if got != sent {
        r.failures.push(format!("놓친 입력: 보냄 {sent} 받음 {got}"));
    }
    // 머리 단계 변화
    let mut heads: Vec<(u64, String)> = Vec::new();
    for l in lines.iter().filter(|l| l.kind == "state") {
        let h = l.v["head"].as_str().unwrap_or("").to_string();
        if heads.last().is_none_or(|x| x.1 != h) {
            heads.push((l.t_us, h));
        }
    }
    let seq: Vec<&str> = heads.iter().map(|h| h.1.as_str()).collect();
    r.notes.push(format!("머리 단계 순서: {seq:?}"));
    let ups = ["normal", "excited", "dizzy", "overheat"];
    let mut pos = 0;
    for h in &seq {
        if pos < ups.len() && *h == ups[pos] {
            pos += 1;
        }
    }
    if pos < ups.len() {
        r.failures.push(format!("normal -> excited -> dizzy -> overheat 순서로 오르지 않음: {seq:?}"));
    }
    for w in heads.windows(2) {
        let dt = (w[1].0 - w[0].0) / 1000;
        let both_levels = ups.contains(&w[0].1.as_str()) && ups.contains(&w[1].1.as_str());
        if both_levels && dt < 1490 {
            r.failures.push(format!("단계가 {dt}ms 만에 다시 바뀜 ({} -> {}) : 깜빡임", w[0].1, w[1].1));
        }
    }
    let gest = lines.iter().any(|l| l.kind == "state" && l.v["pose"] == "gesture");
    r.notes.push(format!("과열 해제 제스처 {}", if gest { "나옴" } else { "안 나옴 (재사용 대기 중일 수 있음)" }));
    // 프레임 간격 (부드러움)
    let gaps: Vec<u64> = lines.iter().filter(|l| l.kind == "frame").filter_map(|l| l.v["gap_us"].as_u64()).collect();
    if !gaps.is_empty() {
        let mut g = gaps.clone();
        g.sort();
        let p50 = g[g.len() / 2];
        let slow = g.iter().filter(|x| **x as f64 > p50 as f64 * 1.5).count();
        r.notes.push(format!(
            "연출 프레임 {}개, 간격 중앙값 {:.2}ms, 1.5배 넘는 프레임 {} ({:.2}%)",
            g.len(),
            p50 as f64 / 1000.0,
            slow,
            slow as f64 * 100.0 / g.len() as f64
        ));
        // 판정은 verify.sh 4층에서 시스템 기준 측정(지우기만 하는 창)과 비교해서 한다

    }
}

// ---------- 창 + 실행 ----------

#[derive(Debug)]
enum Ev {
    Done(i32),
}

struct ProbeApp {
    cmd: String,
    args: Vec<String>,
    window: Option<Window>,
    proxy: EventLoopProxy<Ev>,
    started: bool,
}

impl ApplicationHandler<Ev> for ProbeApp {
    fn new_events(&mut self, el: &ActiveEventLoop, cause: StartCause) {
        if !matches!(cause, StartCause::Init) {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("typing-pet probe (입력 테스트 중)")
            .with_inner_size(LogicalSize::new(420.0, 320.0))
            .with_position(LogicalPosition::new(120.0, 160.0));
        let w = el.create_window(attrs).expect("probe 창");
        w.focus_window();
        self.window = Some(w);
    }

    fn resumed(&mut self, _el: &ActiveEventLoop) {}

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        if let WindowEvent::Focused(true) = ev {
            if self.started {
                return;
            }
            self.started = true;
            let w = self.window.as_ref().expect("창");
            let sf = w.scale_factor();
            let p = w.inner_position().map(|p| p.to_logical::<f64>(sf)).unwrap_or(LogicalPosition::new(120.0, 160.0));
            let s = w.inner_size().to_logical::<f64>(sf);
            let win = (p.x, p.y, s.width, s.height);
            let cmd = self.cmd.clone();
            let args = self.args.clone();
            let proxy = self.proxy.clone();
            std::thread::spawn(move || {
                let code = run(&cmd, &args, win);
                let _ = proxy.send_event(Ev::Done(code));
            });
        }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, ev: Ev) {
        let Ev::Done(code) = ev;
        EXIT.store(code, std::sync::atomic::Ordering::SeqCst);
        el.exit();
    }
}

static EXIT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

fn run(cmd: &str, args: &[String], win: (f64, f64, f64, f64)) -> i32 {
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let Some(trace) = get("--trace") else {
        eprintln!("--trace <앱 트레이스 파일> 필요");
        return 2;
    };
    sleep_ms(600);
    if !frontmost_is_me() {
        eprintln!("안전 중단: probe 창이 맨 앞이 아님 (다른 앱에 입력될 수 있음)");
        return 3;
    }
    let manifest = Manifest::load(None).ok();
    let c = Ctx {
        trace,
        win,
        keys: manifest.as_ref().map(|m| m.keys.clone()).unwrap_or_default(),
        paw_ms: manifest.as_ref().map(|m| m.timing.paw_down_ms).unwrap_or(120),
        burst_secs: get("--burst-secs").and_then(|s| s.parse().ok()).unwrap_or(180),
    };
    let mut r = Report::default();
    let t0 = Instant::now();
    match cmd {
        "all-keys" => run_all_keys(&c, &mut r),
        "modifiers" => run_modifiers(&c, &mut r),
        "repeat" => run_repeat(&c, &mut r),
        "mouse" => run_mouse(&c, &mut r),
        "burst" => run_burst(&c, &mut r),
        _ => {
            eprintln!("알 수 없는 명령: {cmd}");
            return 2;
        }
    }
    let pass = r.failures.is_empty();
    let out = json!({
        "cmd": cmd,
        "pass": pass,
        "sent": r.sent,
        "received": r.received,
        "skipped": r.skipped,
        "failures": r.failures,
        "notes": r.notes,
        "secs": t0.elapsed().as_secs_f64(),
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    if let Some(p) = get("--out") {
        let _ = std::fs::write(p, serde_json::to_string_pretty(&out).unwrap_or_default());
    }
    if pass { 0 } else { 1 }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(cmd) = args.get(1).cloned() else {
        eprintln!("사용법: probe <all-keys|modifiers|repeat|mouse|burst> --trace <파일> [--out <json>]");
        std::process::exit(2);
    };
    if !unsafe { CGPreflightPostEventAccess() } {
        let out = json!({"cmd": cmd, "pass": false, "skipped_layer": true, "failures": ["손쉬운 사용 권한 없음: 실제 입력 층 건너뜀"]});
        println!("{out}");
        std::process::exit(4);
    }
    let el = EventLoop::<Ev>::with_user_event().build().expect("이벤트 루프");
    let mut app = ProbeApp { cmd, args, window: None, proxy: el.create_proxy(), started: false };
    el.run_app(&mut app).expect("실행");
    std::process::exit(EXIT.load(std::sync::atomic::Ordering::SeqCst));
}
