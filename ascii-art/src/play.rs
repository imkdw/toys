//! 크기 계산, 정지 이미지 출력, 영상 재생, 등장 효과.

use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::media::{self, Decoder, MediaInfo, MAX_FPS};
use crate::render::{render, to_ansi, Grid, Options, SX, SY};
use crate::term::{self, AltScreen, HiddenCursor, CLEAR, HOME, RESET, SYNC_OFF, SYNC_ON};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub cols: usize,
    pub rows: usize,
}

impl Layout {
    pub fn frame_size(&self) -> (usize, usize) {
        (self.cols * SX, self.rows * SY)
    }
}

/// 원본 비율을 지키며 칸 수를 정한다. rows = cols * 높이/너비 / 글자비.
pub fn fit(info: &MediaInfo, cols: Option<usize>, char_aspect: f64, reserve_rows: usize) -> Layout {
    let ratio = info.height as f64 / info.width as f64 / char_aspect;
    let c = match cols {
        Some(c) => c,
        None => {
            let (tc, tr) = term::size();
            let max_rows = tr.saturating_sub(reserve_rows).max(1);
            if (tc as f64 * ratio).round() as usize > max_rows {
                (max_rows as f64 / ratio) as usize
            } else {
                tc
            }
        }
    }
    .max(4);
    Layout { cols: c, rows: ((c as f64 * ratio).round() as usize).max(1) }
}

pub struct View {
    pub opts: Options,
    pub color: bool,
    pub cols: Option<usize>,
    pub char_aspect: f64,
    pub intro: bool,
    pub info: bool,
}

fn info_line(name: &str, elapsed: f64, cols: usize, rows: usize, chars: usize, extra: &str) -> String {
    let (tc, _) = term::size();
    let text = format!(" {name}  {elapsed:6.1}s  {cols}x{rows}  {chars} chars{extra}");
    let text: String = text.chars().take(tc).collect();
    let pad = tc.saturating_sub(text.chars().count());
    format!("\x1b[7m{text}{}{RESET}\n", " ".repeat(pad))
}

/// 작은 의사 난수 (등장 효과용, 매번 같은 모양이 나오게 시드 고정)
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (self.next() >> 11) as f64 / (1u64 << 53) as f64 * (hi - lo)
    }
}

/// 글자가 위에서 쏟아지며 그림이 완성되는 등장 효과. 각 프레임 격자를 콜백으로 넘긴다.
pub fn rain(grid: &Grid, duration: f64, fps: f64, mut draw: impl FnMut(&Grid) -> bool) {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let (rows, cols) = (grid.rows as f64, grid.cols);
    let delay: Vec<f64> = (0..cols).map(|_| rng.uniform(0.0, duration * 0.45)).collect();
    let speed: Vec<f64> = (0..cols).map(|_| rows / (duration * 0.5) * rng.uniform(0.8, 1.25)).collect();
    let trail = (rows / 8.0).max(2.0);
    let pool: Vec<char> = "01ABCDEFGHIJKLMNOPQRSTUVWXYZ#$%&*+=?@".chars().collect();
    let mut frame = grid.clone();
    let steps = (duration * fps) as usize;
    for step in 0..=steps * 2 {
        let t = step as f64 / fps;
        let mut done = true;
        for c in 0..cols {
            let head = (t - delay[c]) * speed[c];
            if head - trail < rows {
                done = false;
            }
            for r in 0..grid.rows {
                let i = r * cols + c;
                let rf = r as f64;
                if rf < head - trail {
                    frame.chars[i] = grid.chars[i];
                    frame.colors[i] = grid.colors[i];
                } else if rf <= head {
                    // 떨어지는 꼬리: 머리에 가까울수록 밝은 흰빛
                    frame.chars[i] = pool[(rng.next() % pool.len() as u64) as usize];
                    let k = ((rf - (head - trail)) / trail).clamp(0.0, 1.0) as f32;
                    let base = grid.colors[i];
                    let bright = [230u8, 255, 230];
                    frame.colors[i] = [0, 1, 2].map(|j| (base[j] as f32 * (1.0 - k) + bright[j] as f32 * k) as u8);
                } else {
                    frame.chars[i] = ' ';
                }
            }
        }
        if !draw(&frame) || done {
            break;
        }
    }
}

pub fn show_image(info: &MediaInfo, v: &View) -> Result<(), String> {
    let reserve = if v.info { 2 } else { 1 };
    let layout = fit(info, v.cols, v.char_aspect, reserve);
    let (fw, fh) = layout.frame_size();
    let grid = render(&media::read_frame(info, fw, fh, 0.0)?, fw, fh, &v.opts);
    let mut out = String::new();
    if v.info {
        out.push_str(&info_line(&info.name(), 0.0, grid.cols, grid.rows, grid.nonblank(), ""));
    }
    if v.intro && term::stdout_is_tty() {
        let _cursor = HiddenCursor::new();
        term::put(&out);
        // 그림이 들어갈 자리를 확보하고 그 위치를 저장해 두고 덮어 그린다
        let up = grid.rows.saturating_sub(1);
        term::put(&format!("{}{}\r\x1b7", "\n".repeat(up), if up > 0 { format!("\x1b[{up}A") } else { String::new() }));
        let mut buf = String::new();
        rain(&grid, 1.6, 30.0, |g| {
            let t = Instant::now();
            buf.clear();
            buf.push_str(SYNC_ON);
            buf.push_str("\x1b8");
            to_ansi(g, v.color, &mut buf);
            buf.push_str(SYNC_OFF);
            term::put(&buf);
            sleep(Duration::from_secs_f64(1.0 / 30.0).saturating_sub(t.elapsed()));
            !term::stopped()
        });
        buf.clear();
        buf.push_str("\x1b8");
        to_ansi(&grid, v.color, &mut buf);
        buf.push('\n');
        term::put(&buf);
        return Ok(());
    }
    to_ansi(&grid, v.color, &mut out);
    out.push('\n');
    term::put(&out);
    Ok(())
}

#[derive(Default)]
struct Stats {
    decoded: usize,
    shown: usize,
    skipped: usize,
    render_secs: f64,
    bytes: usize,
    size: (usize, usize),
}

struct StatsGuard<'a> {
    path: Option<String>,
    stats: &'a std::cell::RefCell<Stats>,
    clock: Instant,
}

impl Drop for StatsGuard<'_> {
    fn drop(&mut self) {
        let Some(p) = &self.path else { return };
        let s = self.stats.borrow();
        let text = format!(
            "size={}x{} wall={:.2}s decoded={} shown={} skipped={} render_avg_ms={:.2} bytes_per_frame={}\n",
            s.size.0, s.size.1, self.clock.elapsed().as_secs_f64(), s.decoded, s.shown, s.skipped,
            if s.decoded > 0 { s.render_secs * 1000.0 / s.decoded as f64 } else { 0.0 },
            if s.decoded > 0 { s.bytes / s.decoded } else { 0 },
        );
        let _ = std::fs::write(p, text);
    }
}

struct Cached {
    body: String,
    nonblank: usize,
}

pub fn play_video(info: &MediaInfo, v: &View, fps: Option<f64>, once: bool) -> Result<(), String> {
    if !term::stdout_is_tty() {
        return Err("영상 재생은 터미널에서만 됩니다. 한 프레임만 뽑으려면 --snapshot 을 쓰세요.".into());
    }
    let target = fps.unwrap_or_else(|| info.fps.min(MAX_FPS));
    let period = Duration::from_secs_f64(1.0 / target);
    let reserve = if v.info { 1 } else { 0 };
    let auto = v.cols.is_none();
    let budget = 512 * 1024 * 1024; // 반복 재생용 출력 캐시 한도
    let name = info.name();
    let extra = format!("  {target:.0}fps");

    let _screen = AltScreen::enter();
    let mut layout = fit(info, v.cols, v.char_aspect, reserve);
    let mut cache: Option<Vec<Cached>> = None; // 처음부터 끝까지 다 받은 한 바퀴
    let mut start_at = 0.0f64;
    let mut first = true;
    let clock = Instant::now();
    let mut out = String::new();
    // ASCII_ART_STATS=<파일> 이면 끝날 때 재생 통계를 남긴다 (성능 확인용)
    let stats_path = std::env::var("ASCII_ART_STATS").ok();
    let stats = std::cell::RefCell::new(Stats::default());
    let _stats_guard = StatsGuard { path: stats_path, stats: &stats, clock };

    let present = |out: &mut String, c: &Cached, layout: Layout| {
        let mut s = String::with_capacity(c.body.len() + 256);
        s.push_str(SYNC_ON);
        s.push_str(HOME);
        if v.info {
            s.push_str(&info_line(&name, clock.elapsed().as_secs_f64(), layout.cols, layout.rows, c.nonblank, &extra));
        }
        s.push_str(&c.body);
        s.push_str(SYNC_OFF);
        *out = s;
        term::put(out);
    };

    let check_resize = |layout: &mut Layout| -> bool {
        if term::take_resize() && auto {
            let new = fit(info, v.cols, v.char_aspect, reserve);
            if new != *layout {
                *layout = new;
                term::put(CLEAR);
                return true;
            }
        }
        false
    };

    'outer: loop {
        if term::stopped() {
            break;
        }
        if let Some(frames) = &cache {
            // 디코딩 없이 캐시에서 시간 기준으로 재생, 늦으면 건너뛴다
            let t0 = Instant::now();
            let mut i = 0usize;
            while i < frames.len() {
                if term::stopped() {
                    break 'outer;
                }
                if check_resize(&mut layout) {
                    start_at = i as f64 * period.as_secs_f64();
                    cache = None;
                    continue 'outer;
                }
                present(&mut out, &frames[i], layout);
                stats.borrow_mut().shown += 1;
                i += 1;
                let due = t0 + period * i as u32;
                let now = Instant::now();
                if now < due {
                    sleep(due - now);
                } else {
                    let j = ((now - t0).as_secs_f64() / period.as_secs_f64()) as usize;
                    if j > i {
                        stats.borrow_mut().skipped += j - i;
                        i = j;
                    }
                }
            }
            if once {
                break;
            }
            continue;
        }

        let (fw, fh) = layout.frame_size();
        let dec = Decoder::spawn(info, fw, fh, Some(target), start_at, None)?;
        let mut caching = start_at == 0.0;
        let mut pass: Vec<Cached> = Vec::new();
        let mut bytes = 0usize;
        let mut t0: Option<Instant> = None; // 첫 프레임이 도착한 순간부터 잰다 (ffmpeg 시작 지연 제외)
        let mut i = 0usize;
        let mut restarted = false;
        loop {
            if term::stopped() {
                break 'outer;
            }
            if check_resize(&mut layout) {
                start_at += i as f64 * period.as_secs_f64();
                restarted = true;
                break;
            }
            let frame = match dec.frames.recv_timeout(Duration::from_millis(200)) {
                Ok(f) => f,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            };
            let rt = Instant::now();
            let grid = render(&frame, fw, fh, &v.opts);
            if first && v.intro {
                let mut buf = String::new();
                rain(&grid, 1.6, 30.0, |g| {
                    let t = Instant::now();
                    buf.clear();
                    buf.push_str(SYNC_ON);
                    buf.push_str(HOME);
                    if v.info {
                        buf.push('\n');
                    }
                    to_ansi(g, v.color, &mut buf);
                    buf.push_str(SYNC_OFF);
                    term::put(&buf);
                    sleep(Duration::from_secs_f64(1.0 / 30.0).saturating_sub(t.elapsed()));
                    !term::stopped()
                });
                t0 = Some(Instant::now());
            }
            first = false;
            let t0 = *t0.get_or_insert_with(Instant::now);
            let mut body = String::with_capacity(grid.cols * grid.rows * 4);
            to_ansi(&grid, v.color, &mut body);
            let c = Cached { body, nonblank: grid.nonblank() };
            {
                let mut s = stats.borrow_mut();
                s.decoded += 1;
                s.render_secs += rt.elapsed().as_secs_f64();
                s.bytes += c.body.len();
                s.size = (layout.cols, layout.rows);
            }
            let due = t0 + period * i as u32;
            i += 1;
            let now = Instant::now();
            if now < due {
                sleep(due - now);
            }
            if now <= due + period {
                present(&mut out, &c, layout); // 많이 늦은 프레임은 그리지 않고 건너뛴다
                stats.borrow_mut().shown += 1;
            } else {
                stats.borrow_mut().skipped += 1;
            }
            if caching {
                bytes += c.body.len();
                pass.push(c);
                if bytes > budget {
                    caching = false;
                    pass = Vec::new();
                }
            }
        }
        if restarted {
            continue;
        }
        let whole_pass = start_at == 0.0;
        dec.finish()?;
        if i == 0 {
            return Err("프레임을 하나도 읽지 못했습니다.".into());
        }
        start_at = 0.0;
        if whole_pass && i == 1 {
            // 프레임이 하나뿐인 GIF: 정지 화면으로 두고 종료나 크기 변경을 기다린다
            loop {
                if term::stopped() {
                    break 'outer;
                }
                if check_resize(&mut layout) {
                    continue 'outer;
                }
                sleep(Duration::from_millis(50));
            }
        }
        if once {
            break;
        }
        if caching && whole_pass {
            cache = Some(pass);
        }
    }
    Ok(())
}
